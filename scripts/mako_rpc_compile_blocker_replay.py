#!/usr/bin/env python3
"""Focused replay hook for top-ranked RPC compile blockers (leaf 2.2).

The script consumes leaf-2.1 inventory artifacts, ranks blocker translation units
deterministically, replays compile commands for the top candidates, and captures
first-failure artifacts for follow-up fix work.
"""

from __future__ import annotations

import argparse
import json
import shlex
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Sequence

COMMAND_TIMEOUT_STATUS = 124
COMMAND_NOT_FOUND_STATUS = 127

DEFAULT_MAX_REPLAYS = 1
DEFAULT_TIMEOUT_SECONDS = 300
DEFAULT_LANES = ("clang", "fragilec")

BLOCKER_PRIORITY: dict[str, int] = {
    "unresolved_name_or_type_e0425": 0,
    "missing_method_e0599": 1,
    "arity_mismatch_e0061": 2,
    "type_mismatch_e0308": 3,
    "other_rustc_error": 4,
    "transpile_failure": 5,
    "other_build_failure": 6,
    "build_not_executed": 7,
    "none": 8,
}


@dataclass(frozen=True)
class InventoryEntry:
    lane: str
    build_status: int
    blocker_class: str
    blocker_file: str
    e0425_count: int


@dataclass(frozen=True)
class CommandPlan:
    argv: list[str]
    command_dir: Path
    command_source: str


@dataclass(frozen=True)
class CommandResult:
    status: int
    stdout: str
    stderr: str
    timed_out: bool


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Replay top-ranked blocker translation units from leaf-2.1 inventory artifacts"
    )
    parser.add_argument("--run-root", type=Path, required=True)
    parser.add_argument(
        "--lanes",
        default="",
        help="comma-separated lane list; default uses inventory manifest lanes",
    )
    parser.add_argument("--max-replays", type=int, default=DEFAULT_MAX_REPLAYS)
    parser.add_argument("--timeout-seconds", type=int, default=DEFAULT_TIMEOUT_SECONDS)
    return parser.parse_args(list(argv))


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def write_text(path: Path, value: str) -> None:
    path.write_text(value + "\n", encoding="utf-8")


def write_lines(path: Path, lines: list[str]) -> None:
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def parse_key_value_file(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for line in read_text(path).splitlines():
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        result[key.strip()] = value.strip()
    return result


def parse_lanes(raw: str) -> list[str]:
    lanes = [lane.strip() for lane in raw.split(",") if lane.strip()]
    if not lanes:
        raise ValueError("lanes must include at least one lane name")
    return lanes


def parse_inventory_entries(
    manifest: dict[str, str],
    lanes: list[str],
) -> list[InventoryEntry]:
    entries: list[InventoryEntry] = []
    for lane in lanes:
        build_status_key = f"lane_{lane}_build_status"
        blocker_class_key = f"lane_{lane}_first_failing_compile_class"
        blocker_file_key = f"lane_{lane}_first_failing_compile_file"
        e0425_count_key = f"lane_{lane}_first_failing_compile_e0425_count"
        for key in (
            build_status_key,
            blocker_class_key,
            blocker_file_key,
            e0425_count_key,
        ):
            if key not in manifest:
                raise KeyError(f"missing inventory manifest key: {key}")
        entries.append(
            InventoryEntry(
                lane=lane,
                build_status=int(manifest[build_status_key]),
                blocker_class=manifest[blocker_class_key],
                blocker_file=manifest[blocker_file_key],
                e0425_count=int(manifest[e0425_count_key]),
            )
        )
    return entries


def replay_candidate(entry: InventoryEntry) -> bool:
    if entry.blocker_file == "none":
        return False
    if entry.blocker_class in ("none", "build_not_executed"):
        return False
    return True


def rank_key(entry: InventoryEntry) -> tuple[int, int, str, str]:
    priority = BLOCKER_PRIORITY.get(entry.blocker_class, 999)
    return (priority, -entry.e0425_count, entry.lane, entry.blocker_file)


def first_failure_class(status: int, stderr: str) -> str:
    if status == 0:
        return "none"
    if "[fragilec] failed to transpile " in stderr:
        return "transpile_failure"
    if "error[E0425]" in stderr:
        return "unresolved_name_or_type_e0425"
    if "error[E0599]" in stderr:
        return "missing_method_e0599"
    if "error[E0061]" in stderr:
        return "arity_mismatch_e0061"
    if "error[E0308]" in stderr:
        return "type_mismatch_e0308"
    if "error[E" in stderr:
        return "other_rustc_error"
    return "other_build_failure"


def first_failure_excerpt(status: int, stderr: str) -> str:
    if status == 0:
        return "none"
    for line in stderr.splitlines():
        stripped = line.strip()
        if not stripped:
            continue
        if "error[E" in stripped or stripped.startswith("error:"):
            return stripped
    for line in stderr.splitlines():
        stripped = line.strip()
        if stripped:
            return stripped
    return "none"


def shell_join(argv: list[str]) -> str:
    return " ".join(shlex.quote(token) for token in argv)


def rewrite_output_path(argv: list[str], output_path: Path) -> list[str]:
    rewritten = list(argv)
    replaced = False
    for i, token in enumerate(rewritten):
        if token == "-o" and i + 1 < len(rewritten):
            rewritten[i + 1] = str(output_path)
            replaced = True
            break
        if token.startswith("-o") and len(token) > 2:
            rewritten[i] = "-o" + str(output_path)
            replaced = True
            break
    if not replaced:
        rewritten.extend(["-o", str(output_path)])
    return rewritten


def resolve_compile_commands_plan(
    run_root: Path,
    entry: InventoryEntry,
    replay_object: Path,
) -> CommandPlan | None:
    compile_commands_path = run_root / f"build_{entry.lane}" / "compile_commands.json"
    if not compile_commands_path.exists():
        return None

    try:
        raw_entries = json.loads(read_text(compile_commands_path))
    except json.JSONDecodeError:
        return None
    if not isinstance(raw_entries, list):
        return None

    expected_file = Path(entry.blocker_file).resolve()
    for raw in raw_entries:
        if not isinstance(raw, dict):
            continue
        directory_raw = raw.get("directory")
        file_raw = raw.get("file")
        if not isinstance(file_raw, str):
            continue
        directory = (
            Path(directory_raw).resolve()
            if isinstance(directory_raw, str)
            else compile_commands_path.parent.resolve()
        )
        file_path = Path(file_raw)
        resolved_file = (
            file_path.resolve() if file_path.is_absolute() else (directory / file_path).resolve()
        )
        if resolved_file != expected_file:
            continue

        arguments = raw.get("arguments")
        command = raw.get("command")
        argv: list[str]
        if isinstance(arguments, list) and all(isinstance(tok, str) for tok in arguments):
            argv = [str(tok) for tok in arguments]
        elif isinstance(command, str):
            argv = shlex.split(command)
        else:
            continue

        return CommandPlan(
            argv=rewrite_output_path(argv, replay_object),
            command_dir=directory,
            command_source="compile_commands",
        )
    return None


def lane_compiler(harness_manifest: dict[str, str], lane: str) -> str:
    if lane == "clang":
        return harness_manifest.get("clang_cxx", "clang++")
    return harness_manifest.get("fragile_cxx", "fragilec")


def resolve_fallback_plan(
    run_root: Path,
    harness_manifest: dict[str, str],
    entry: InventoryEntry,
    replay_object: Path,
) -> CommandPlan:
    workspace_root = harness_manifest.get("workspace_root", str(run_root))
    return CommandPlan(
        argv=[
            lane_compiler(harness_manifest, entry.lane),
            "-std=gnu++17",
            "-c",
            entry.blocker_file,
            "-o",
            str(replay_object),
        ],
        command_dir=Path(workspace_root).resolve(),
        command_source="fallback_compiler",
    )


def run_capture(argv: list[str], cwd: Path, timeout_seconds: int) -> CommandResult:
    try:
        output = subprocess.run(
            argv,
            cwd=str(cwd),
            check=False,
            capture_output=True,
            text=True,
            timeout=timeout_seconds,
        )
        return CommandResult(
            status=output.returncode,
            stdout=output.stdout,
            stderr=output.stderr,
            timed_out=False,
        )
    except subprocess.TimeoutExpired as exc:
        timed_out_msg = (
            f"error: command timed out after {timeout_seconds} seconds: {shell_join(argv)}\n"
        )
        stdout = "" if exc.stdout is None else str(exc.stdout)
        stderr = ("" if exc.stderr is None else str(exc.stderr)) + timed_out_msg
        return CommandResult(
            status=COMMAND_TIMEOUT_STATUS,
            stdout=stdout,
            stderr=stderr,
            timed_out=True,
        )
    except OSError as exc:
        return CommandResult(
            status=COMMAND_NOT_FOUND_STATUS,
            stdout="",
            stderr=f"error: failed to run command: {shell_join(argv)} ({exc})\n",
            timed_out=False,
        )


def run_replay(run_root: Path, entries: list[InventoryEntry], max_replays: int, timeout_seconds: int) -> None:
    inventory_manifest_path = run_root / "rpc_compile_blocker_inventory_manifest.txt"
    if not inventory_manifest_path.exists():
        raise FileNotFoundError(
            f"missing inventory manifest artifact: {inventory_manifest_path}"
        )
    inventory_manifest = parse_key_value_file(inventory_manifest_path)

    benchmark_manifest_path = run_root / "benchmark_harness_manifest.txt"
    benchmark_manifest = (
        parse_key_value_file(benchmark_manifest_path)
        if benchmark_manifest_path.exists()
        else {}
    )

    candidates = [entry for entry in entries if replay_candidate(entry)]
    selected = sorted(candidates, key=rank_key)[:max_replays]

    plan_lines = [
        "version=1",
        "task_leaf=2.2",
        f"run_root={run_root}",
        f"max_replays={max_replays}",
        f"timeout_seconds={timeout_seconds}",
        f"selected_count={len(selected)}",
    ]
    manifest_lines = list(plan_lines)

    for idx, entry in enumerate(selected, start=1):
        replay_tag = f"replay_{idx:02d}"
        replay_dir = run_root / replay_tag
        replay_dir.mkdir(parents=True, exist_ok=True)
        replay_object = replay_dir / "focused_replay.o"

        plan = resolve_compile_commands_plan(run_root, entry, replay_object)
        if plan is None:
            plan = resolve_fallback_plan(run_root, benchmark_manifest, entry, replay_object)

        result = run_capture(plan.argv, plan.command_dir, timeout_seconds)
        failure_class = first_failure_class(result.status, result.stderr)
        failure_excerpt = first_failure_excerpt(result.status, result.stderr)

        write_text(replay_dir / "lane.txt", entry.lane)
        write_text(replay_dir / "blocker_class.txt", entry.blocker_class)
        write_text(replay_dir / "blocker_file.txt", entry.blocker_file)
        write_text(replay_dir / "command_source.txt", plan.command_source)
        write_text(replay_dir / "command_directory.txt", str(plan.command_dir))
        write_text(replay_dir / "command.txt", shell_join(plan.argv))
        write_text(replay_dir / "replay.status", str(result.status))
        write_text(replay_dir / "first_failure_class.txt", failure_class)
        write_text(replay_dir / "first_failure_excerpt.txt", failure_excerpt)
        write_lines(replay_dir / "replay.stdout", result.stdout.splitlines())
        write_lines(replay_dir / "replay.stderr", result.stderr.splitlines())

        plan_lines.append("")
        plan_lines.append(f"[{replay_tag}]")
        plan_lines.append(f"lane={entry.lane}")
        plan_lines.append(f"blocker_class={entry.blocker_class}")
        plan_lines.append(f"blocker_file={entry.blocker_file}")
        plan_lines.append(f"command_source={plan.command_source}")
        plan_lines.append(f"command_directory={plan.command_dir}")
        plan_lines.append(f"command={shell_join(plan.argv)}")

        manifest_lines.extend(
            [
                f"{replay_tag}_lane={entry.lane}",
                f"{replay_tag}_blocker_class={entry.blocker_class}",
                f"{replay_tag}_blocker_file={entry.blocker_file}",
                f"{replay_tag}_e0425_count={entry.e0425_count}",
                f"{replay_tag}_command_source={plan.command_source}",
                f"{replay_tag}_command_directory={plan.command_dir}",
                f"{replay_tag}_status={result.status}",
                f"{replay_tag}_timed_out={str(result.timed_out).lower()}",
                f"{replay_tag}_first_failure_class={failure_class}",
                f"{replay_tag}_first_failure_excerpt={failure_excerpt}",
                f"{replay_tag}_artifact_dir={replay_dir}",
            ]
        )

    write_lines(run_root / "rpc_compile_blocker_replay_plan.txt", plan_lines)
    write_lines(run_root / "rpc_compile_blocker_replay_manifest.txt", manifest_lines)


def main(argv: Sequence[str]) -> int:
    try:
        ns = parse_args(argv)
        if ns.max_replays <= 0:
            raise ValueError(f"max-replays must be > 0, got {ns.max_replays}")
        if ns.timeout_seconds <= 0:
            raise ValueError(f"timeout-seconds must be > 0, got {ns.timeout_seconds}")

        run_root = ns.run_root.resolve()
        if not run_root.exists():
            raise FileNotFoundError(f"run root does not exist: {run_root}")

        inventory_manifest_path = run_root / "rpc_compile_blocker_inventory_manifest.txt"
        if not inventory_manifest_path.exists():
            raise FileNotFoundError(
                f"missing inventory manifest artifact: {inventory_manifest_path}"
            )
        inventory_manifest = parse_key_value_file(inventory_manifest_path)
        lanes = (
            parse_lanes(ns.lanes)
            if ns.lanes.strip()
            else parse_lanes(inventory_manifest.get("lanes", ",".join(DEFAULT_LANES)))
        )
        entries = parse_inventory_entries(inventory_manifest, lanes)
        run_replay(
            run_root=run_root,
            entries=entries,
            max_replays=ns.max_replays,
            timeout_seconds=ns.timeout_seconds,
        )
        print(run_root)
        return 0
    except Exception as exc:  # pylint: disable=broad-except
        print(f"error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
