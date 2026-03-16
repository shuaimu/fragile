#!/usr/bin/env python3
"""Parser-backend A/B strict baseline harness for TODO leaf M0.2.

This script runs the existing strict baseline capture twice under one parent run root:
- baseline parser backend run root
- candidate parser backend run root

It emits deterministic comparable-manifest snapshots and a deterministic diff manifest.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import shlex
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping, Sequence

SUPPORTED_LANES: tuple[str, str] = ("clang", "fragilec")
COMMAND_NOT_FOUND_STATUS = 127
NON_COMPARABLE_KEYS: tuple[str, ...] = (
    "run_root",
    "harness_manifest",
    "inventory_manifest",
    "replay_manifest",
    "stage_timing_path",
)


@dataclass(frozen=True)
class CommandResult:
    status: int
    stdout: str
    stderr: str


def shell_join(argv: Sequence[str]) -> str:
    return " ".join(shlex.quote(token) for token in argv)


def write_text(path: Path, value: str) -> None:
    path.write_text(value + "\n", encoding="utf-8")


def write_lines(path: Path, lines: list[str]) -> None:
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def parse_key_value_file(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key.strip()] = value.strip()
    return values


def run_capture(argv: list[str], *, env: Mapping[str, str]) -> CommandResult:
    try:
        output = subprocess.run(
            argv,
            check=False,
            text=True,
            capture_output=True,
            env=dict(env),
        )
        return CommandResult(
            status=output.returncode,
            stdout=output.stdout,
            stderr=output.stderr,
        )
    except OSError as exc:
        return CommandResult(
            status=COMMAND_NOT_FOUND_STATUS,
            stdout="",
            stderr=f"error: failed to run command: {shell_join(argv)} ({exc})\n",
        )


def write_command_result(run_root: Path, name: str, result: CommandResult) -> None:
    write_text(run_root / f"{name}.status", str(result.status))
    write_lines(run_root / f"{name}.stdout.log", result.stdout.splitlines())
    write_lines(run_root / f"{name}.stderr.log", result.stderr.splitlines())


def parse_lanes(raw: str) -> tuple[str, ...]:
    lanes = tuple(lane.strip() for lane in raw.split(",") if lane.strip())
    if not lanes:
        raise ValueError("lanes must include at least one lane name")
    unknown = [lane for lane in lanes if lane not in SUPPORTED_LANES]
    if unknown:
        raise ValueError(
            f"unsupported lane(s): {','.join(unknown)}; supported: {','.join(SUPPORTED_LANES)}"
        )
    ordered_unique: list[str] = []
    seen: set[str] = set()
    for lane in lanes:
        if lane in seen:
            continue
        seen.add(lane)
        ordered_unique.append(lane)
    return tuple(ordered_unique)


def ensure_positive(name: str, value: int) -> None:
    if value <= 0:
        raise ValueError(f"{name} must be > 0, got {value}")


def ensure_non_negative(name: str, value: float) -> None:
    if value < 0:
        raise ValueError(f"{name} must be >= 0, got {value}")


def sanitize_backend_token(value: str) -> str:
    cleaned = "".join(
        ch if (ch.isalnum() or ch in ("-", "_", ".")) else "_" for ch in value.strip()
    )
    if not cleaned:
        raise ValueError("backend label must not be empty")
    return cleaned


def canonical_manifest_lines(values: Mapping[str, str]) -> list[str]:
    return [f"{key}={values[key]}" for key in sorted(values)]


def comparable_manifest(values: Mapping[str, str]) -> dict[str, str]:
    return {
        key: values[key]
        for key in sorted(values)
        if key not in NON_COMPARABLE_KEYS
    }


def manifest_sha256(values: Mapping[str, str]) -> str:
    digest = hashlib.sha256()
    for line in canonical_manifest_lines(values):
        digest.update(line.encode("utf-8"))
        digest.update(b"\n")
    return digest.hexdigest()


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    script_dir = Path(__file__).resolve().parent
    workspace_root = script_dir.parent
    default_mako_root = workspace_root / "vendor" / "mako"
    default_run_root = Path(
        f"/tmp/fragile_m0_2_parser_backend_ab_{os.getpid()}_{time.time_ns()}"
    )

    parser = argparse.ArgumentParser(
        description=(
            "Run strict baseline capture side-by-side for baseline/candidate parser backends"
        )
    )
    parser.add_argument("--workspace-root", type=Path, default=workspace_root)
    parser.add_argument("--mako-root", type=Path, default=default_mako_root)
    parser.add_argument("--run-root", type=Path, default=default_run_root)
    parser.add_argument("--baseline-backend", default="libtooling")
    parser.add_argument("--candidate-backend", default="libclang")
    parser.add_argument("--lanes", default="fragilec")
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--trials", type=int, default=1)
    parser.add_argument("--base-port", type=int, default=18900)
    parser.add_argument("--build-timeout-seconds", type=int, default=600)
    parser.add_argument("--test-rpc-timeout-seconds", type=int, default=120)
    parser.add_argument("--rpc-client-timeout-seconds", type=int, default=120)
    parser.add_argument("--rpc-server-startup-wait-seconds", type=float, default=1.0)
    parser.add_argument("--rpc-server-shutdown-timeout-seconds", type=int, default=10)
    parser.add_argument("--replay-timeout-seconds", type=int, default=120)
    parser.add_argument("--replay-max-replays", type=int, default=1)
    parser.add_argument("--build-only", action="store_true")
    parser.add_argument(
        "--strict-baseline-script",
        type=Path,
        default=script_dir / "mako_rpc_strict_baseline.py",
    )
    parser.add_argument(
        "--harness-script",
        type=Path,
        default=script_dir / "mako_rpcbench_harness.py",
    )
    parser.add_argument(
        "--inventory-script",
        type=Path,
        default=script_dir / "mako_rpc_compile_blocker_inventory.py",
    )
    parser.add_argument(
        "--replay-script",
        type=Path,
        default=script_dir / "mako_rpc_compile_blocker_replay.py",
    )
    ns = parser.parse_args(list(argv))

    ensure_positive("jobs", ns.jobs)
    ensure_positive("trials", ns.trials)
    ensure_positive("build-timeout-seconds", ns.build_timeout_seconds)
    ensure_positive("test-rpc-timeout-seconds", ns.test_rpc_timeout_seconds)
    ensure_positive("rpc-client-timeout-seconds", ns.rpc_client_timeout_seconds)
    ensure_non_negative(
        "rpc-server-startup-wait-seconds", ns.rpc_server_startup_wait_seconds
    )
    ensure_positive(
        "rpc-server-shutdown-timeout-seconds", ns.rpc_server_shutdown_timeout_seconds
    )
    ensure_positive("replay-timeout-seconds", ns.replay_timeout_seconds)
    ensure_positive("replay-max-replays", ns.replay_max_replays)
    if ns.base_port < 1024 or ns.base_port > 65535:
        raise ValueError(f"base-port must be within [1024, 65535], got {ns.base_port}")
    sanitize_backend_token(ns.baseline_backend)
    sanitize_backend_token(ns.candidate_backend)
    return ns


def strict_baseline_command(
    ns: argparse.Namespace,
    *,
    run_root: Path,
    lanes: tuple[str, ...],
    base_port: int,
) -> list[str]:
    cmd = [
        "python3",
        str(ns.strict_baseline_script.resolve()),
        "--workspace-root",
        str(ns.workspace_root.resolve()),
        "--mako-root",
        str(ns.mako_root.resolve()),
        "--run-root",
        str(run_root),
        "--lanes",
        ",".join(lanes),
        "--jobs",
        str(ns.jobs),
        "--trials",
        str(ns.trials),
        "--base-port",
        str(base_port),
        "--build-timeout-seconds",
        str(ns.build_timeout_seconds),
        "--test-rpc-timeout-seconds",
        str(ns.test_rpc_timeout_seconds),
        "--rpc-client-timeout-seconds",
        str(ns.rpc_client_timeout_seconds),
        "--rpc-server-startup-wait-seconds",
        str(ns.rpc_server_startup_wait_seconds),
        "--rpc-server-shutdown-timeout-seconds",
        str(ns.rpc_server_shutdown_timeout_seconds),
        "--replay-timeout-seconds",
        str(ns.replay_timeout_seconds),
        "--replay-max-replays",
        str(ns.replay_max_replays),
        "--harness-script",
        str(ns.harness_script.resolve()),
        "--inventory-script",
        str(ns.inventory_script.resolve()),
        "--replay-script",
        str(ns.replay_script.resolve()),
    ]
    if ns.build_only:
        cmd.append("--build-only")
    return cmd


def main(argv: Sequence[str]) -> int:
    try:
        ns = parse_args(argv)
        lanes = parse_lanes(ns.lanes)
        run_root = ns.run_root.resolve()
        run_root.mkdir(parents=True, exist_ok=True)

        baseline_backend = sanitize_backend_token(ns.baseline_backend)
        candidate_backend = sanitize_backend_token(ns.candidate_backend)
        baseline_run_root = run_root / f"baseline_{baseline_backend}"
        candidate_run_root = run_root / f"candidate_{candidate_backend}"

        baseline_cmd = strict_baseline_command(
            ns,
            run_root=baseline_run_root,
            lanes=lanes,
            base_port=ns.base_port,
        )
        candidate_base_port = ns.base_port + 1000
        if candidate_base_port > 65535:
            raise ValueError(
                "candidate base port would exceed 65535; choose a lower --base-port"
            )
        candidate_cmd = strict_baseline_command(
            ns,
            run_root=candidate_run_root,
            lanes=lanes,
            base_port=candidate_base_port,
        )

        write_lines(
            run_root / "parser_backend_ab_commands.txt",
            [
                "version=1",
                "task_leaf=M0.2",
                f"run_root={run_root}",
                f"lanes={','.join(lanes)}",
                f"baseline_backend={baseline_backend}",
                f"candidate_backend={candidate_backend}",
                f"baseline_run_root={baseline_run_root}",
                f"candidate_run_root={candidate_run_root}",
                f"baseline_command={shell_join(baseline_cmd)}",
                f"candidate_command={shell_join(candidate_cmd)}",
            ],
        )

        baseline_env = dict(os.environ)
        baseline_env["FRAGILEC_PARSER_BACKEND"] = baseline_backend
        baseline_result = run_capture(baseline_cmd, env=baseline_env)
        write_command_result(run_root, "parser_backend_ab_baseline", baseline_result)

        candidate_env = dict(os.environ)
        candidate_env["FRAGILEC_PARSER_BACKEND"] = candidate_backend
        candidate_result = run_capture(candidate_cmd, env=candidate_env)
        write_command_result(run_root, "parser_backend_ab_candidate", candidate_result)

        if baseline_result.status != 0 or candidate_result.status != 0:
            raise RuntimeError(
                "strict baseline command failed for one or both backends "
                "(see parser_backend_ab_baseline.stderr.log / parser_backend_ab_candidate.stderr.log)"
            )

        baseline_manifest_path = baseline_run_root / "strict_baseline_manifest.txt"
        candidate_manifest_path = candidate_run_root / "strict_baseline_manifest.txt"
        if not baseline_manifest_path.exists():
            raise FileNotFoundError(
                f"missing baseline strict manifest artifact: {baseline_manifest_path}"
            )
        if not candidate_manifest_path.exists():
            raise FileNotFoundError(
                f"missing candidate strict manifest artifact: {candidate_manifest_path}"
            )

        baseline_manifest = parse_key_value_file(baseline_manifest_path)
        candidate_manifest = parse_key_value_file(candidate_manifest_path)
        baseline_comp = comparable_manifest(baseline_manifest)
        candidate_comp = comparable_manifest(candidate_manifest)

        baseline_comp_path = (
            run_root / "parser_backend_ab_baseline_comparable_manifest.txt"
        )
        candidate_comp_path = (
            run_root / "parser_backend_ab_candidate_comparable_manifest.txt"
        )
        write_lines(baseline_comp_path, canonical_manifest_lines(baseline_comp))
        write_lines(candidate_comp_path, canonical_manifest_lines(candidate_comp))

        baseline_keys = set(baseline_comp)
        candidate_keys = set(candidate_comp)
        common_keys = sorted(baseline_keys & candidate_keys)
        differing_keys = [
            key for key in common_keys if baseline_comp[key] != candidate_comp[key]
        ]
        missing_in_baseline = sorted(candidate_keys - baseline_keys)
        missing_in_candidate = sorted(baseline_keys - candidate_keys)
        comparable_equal = (
            len(differing_keys) == 0
            and len(missing_in_baseline) == 0
            and len(missing_in_candidate) == 0
        )

        lines = [
            "version=1",
            "task_leaf=M0.2",
            f"run_root={run_root}",
            f"lanes={','.join(lanes)}",
            f"baseline_backend={baseline_backend}",
            f"candidate_backend={candidate_backend}",
            f"baseline_command_status={baseline_result.status}",
            f"candidate_command_status={candidate_result.status}",
            f"baseline_run_root={baseline_run_root}",
            f"candidate_run_root={candidate_run_root}",
            f"baseline_manifest={baseline_manifest_path}",
            f"candidate_manifest={candidate_manifest_path}",
            f"baseline_comparable_manifest={baseline_comp_path}",
            f"candidate_comparable_manifest={candidate_comp_path}",
            f"baseline_comparable_sha256={manifest_sha256(baseline_comp)}",
            f"candidate_comparable_sha256={manifest_sha256(candidate_comp)}",
            f"baseline_comparable_key_count={len(baseline_comp)}",
            f"candidate_comparable_key_count={len(candidate_comp)}",
            f"common_key_count={len(common_keys)}",
            f"different_key_count={len(differing_keys)}",
            f"missing_in_baseline_count={len(missing_in_baseline)}",
            f"missing_in_candidate_count={len(missing_in_candidate)}",
            f"comparable_equal={'true' if comparable_equal else 'false'}",
            f"non_comparable_keys={','.join(NON_COMPARABLE_KEYS)}",
        ]

        for index, key in enumerate(differing_keys, start=1):
            lines.extend(
                [
                    f"different_{index:03d}_key={key}",
                    f"different_{index:03d}_baseline={baseline_comp[key]}",
                    f"different_{index:03d}_candidate={candidate_comp[key]}",
                ]
            )
        for index, key in enumerate(missing_in_baseline, start=1):
            lines.extend(
                [
                    f"missing_in_baseline_{index:03d}_key={key}",
                    f"missing_in_baseline_{index:03d}_candidate={candidate_comp[key]}",
                ]
            )
        for index, key in enumerate(missing_in_candidate, start=1):
            lines.extend(
                [
                    f"missing_in_candidate_{index:03d}_key={key}",
                    f"missing_in_candidate_{index:03d}_baseline={baseline_comp[key]}",
                ]
            )

        write_lines(run_root / "parser_backend_ab_manifest.txt", lines)
        print(run_root)
        return 0
    except Exception as exc:  # pylint: disable=broad-except
        print(f"error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
