#!/usr/bin/env python3
"""Run a command with deterministic timeout capture and persisted artifacts.

This helper is intentionally generic so CI-aligned local replays can:
- complete deterministically even when commands stall with no output,
- persist stdout/stderr/status artifacts under a run root, and
- classify timeout reason (`none`, `inactivity_timeout`, `wall_timeout`).
"""

from __future__ import annotations

import argparse
import fcntl
import os
import selectors
import shlex
import signal
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Sequence

TIMEOUT_STATUS = 124
COMMAND_NOT_FOUND_STATUS = 127


@dataclass(frozen=True)
class CommandResult:
    status: int
    timed_out: bool
    timeout_reason: str


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run a command with inactivity/wall timeout capture artifacts."
    )
    parser.add_argument("--run-root", type=Path, required=True)
    parser.add_argument("--name", required=True, help="artifact file prefix")
    parser.add_argument("--cwd", type=Path, default=None)
    parser.add_argument("--inactivity-timeout-seconds", type=int, default=120)
    parser.add_argument("--wall-timeout-seconds", type=int, default=3600)
    parser.add_argument(
        "--command",
        nargs=argparse.REMAINDER,
        required=True,
        help="command to run, e.g. --command cargo test --verbose",
    )
    ns = parser.parse_args(list(argv))
    if ns.inactivity_timeout_seconds <= 0:
        raise ValueError(
            "inactivity-timeout-seconds must be > 0, got "
            f"{ns.inactivity_timeout_seconds}"
        )
    if ns.wall_timeout_seconds <= 0:
        raise ValueError(
            f"wall-timeout-seconds must be > 0, got {ns.wall_timeout_seconds}"
        )
    if not ns.command:
        raise ValueError("command must not be empty")
    return ns


def shell_join(argv: Sequence[str]) -> str:
    return " ".join(shlex.quote(token) for token in argv)


def write_text(path: Path, value: str) -> None:
    path.write_text(value + "\n", encoding="utf-8")


def write_lines(path: Path, lines: list[str]) -> None:
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def drain_pipe(pipe: object, sink: object) -> None:
    # Pipe is expected to be a binary file-like object from subprocess pipes.
    # Use non-blocking reads so detached descendants holding writer FDs cannot
    # stall artifact finalization.
    fd = pipe.fileno()
    original_flags = fcntl.fcntl(fd, fcntl.F_GETFL)
    fcntl.fcntl(fd, fcntl.F_SETFL, original_flags | os.O_NONBLOCK)
    try:
        while True:
            try:
                chunk = os.read(fd, 4096)
            except BlockingIOError:
                return
            if not chunk:
                return
            sink.write(chunk)
    finally:
        fcntl.fcntl(fd, fcntl.F_SETFL, original_flags)


def run_command_capture(
    command: list[str],
    *,
    cwd: Path | None,
    stdout_path: Path,
    stderr_path: Path,
    inactivity_timeout_seconds: int,
    wall_timeout_seconds: int,
) -> CommandResult:
    with stdout_path.open("wb") as stdout_file, stderr_path.open("wb") as stderr_file:
        try:
            process = subprocess.Popen(
                command,
                cwd=str(cwd) if cwd is not None else None,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                start_new_session=True,
            )
        except FileNotFoundError as exc:
            stderr_file.write(
                f"error: command not found: {command[0]} ({exc})\n".encode("utf-8")
            )
            return CommandResult(
                status=COMMAND_NOT_FOUND_STATUS,
                timed_out=False,
                timeout_reason="none",
            )

        assert process.stdout is not None
        assert process.stderr is not None

        selector = selectors.DefaultSelector()
        selector.register(process.stdout, selectors.EVENT_READ, data=stdout_file)
        selector.register(process.stderr, selectors.EVENT_READ, data=stderr_file)

        start = time.monotonic()
        last_output = start
        timeout_reason = "none"

        while True:
            now = time.monotonic()
            wall_elapsed = now - start
            idle_elapsed = now - last_output
            if wall_elapsed >= wall_timeout_seconds:
                timeout_reason = "wall_timeout"
                break
            if idle_elapsed >= inactivity_timeout_seconds:
                timeout_reason = "inactivity_timeout"
                break

            if process.poll() is not None and not selector.get_map():
                break

            wall_remaining = wall_timeout_seconds - wall_elapsed
            idle_remaining = inactivity_timeout_seconds - idle_elapsed
            wait_timeout = max(0.0, min(0.5, wall_remaining, idle_remaining))
            events = selector.select(timeout=wait_timeout)
            for key, _mask in events:
                chunk = os.read(key.fileobj.fileno(), 4096)
                if not chunk:
                    selector.unregister(key.fileobj)
                    continue
                key.data.write(chunk)
                key.data.flush()
                last_output = time.monotonic()

        if timeout_reason != "none" and process.poll() is None:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass

        process.wait()
        # Drain any trailing output after process exit/kill.
        drain_pipe(process.stdout, stdout_file)
        drain_pipe(process.stderr, stderr_file)
        stdout_file.flush()
        stderr_file.flush()

        if timeout_reason != "none":
            with stderr_path.open("ab") as stderr_append:
                stderr_append.write(
                    (
                        "error: command timed out due to "
                        f"{timeout_reason} (inactivity_timeout_seconds="
                        f"{inactivity_timeout_seconds}, wall_timeout_seconds="
                        f"{wall_timeout_seconds})\n"
                    ).encode("utf-8")
                )
            return CommandResult(
                status=TIMEOUT_STATUS,
                timed_out=True,
                timeout_reason=timeout_reason,
            )
        return CommandResult(
            status=process.returncode,
            timed_out=False,
            timeout_reason="none",
        )


def main(argv: Sequence[str]) -> int:
    ns = parse_args(argv)
    run_root: Path = ns.run_root
    run_root.mkdir(parents=True, exist_ok=True)

    stdout_path = run_root / f"{ns.name}.stdout.log"
    stderr_path = run_root / f"{ns.name}.stderr.log"
    status_path = run_root / f"{ns.name}.status"
    manifest_path = run_root / f"{ns.name}.manifest.txt"

    command = list(ns.command)
    result = run_command_capture(
        command,
        cwd=ns.cwd,
        stdout_path=stdout_path,
        stderr_path=stderr_path,
        inactivity_timeout_seconds=ns.inactivity_timeout_seconds,
        wall_timeout_seconds=ns.wall_timeout_seconds,
    )
    write_text(status_path, str(result.status))
    write_lines(
        manifest_path,
        [
            "version=1",
            f"name={ns.name}",
            f"run_root={run_root}",
            f"cwd={ns.cwd if ns.cwd is not None else ''}",
            f"command={shell_join(command)}",
            f"status={result.status}",
            f"timed_out={'true' if result.timed_out else 'false'}",
            f"timeout_reason={result.timeout_reason}",
            f"inactivity_timeout_seconds={ns.inactivity_timeout_seconds}",
            f"wall_timeout_seconds={ns.wall_timeout_seconds}",
            f"stdout_path={stdout_path}",
            f"stderr_path={stderr_path}",
            f"status_path={status_path}",
        ],
    )
    return result.status


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
