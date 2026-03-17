#!/usr/bin/env python3
"""Deterministic strict baseline capture for TODO leaf M0.1.

This script orchestrates three existing RPC tooling stages under one run root:
- strict harness capture (`mako_rpcbench_harness.py`)
- compile-blocker inventory capture (`mako_rpc_compile_blocker_inventory.py`)
- focused blocker replay with stage-timing trace (`mako_rpc_compile_blocker_replay.py`)

Artifacts are summarized in `strict_baseline_manifest.txt`.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import shlex
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping, Sequence

from mako_rpc_milestone_contract import (
    RUN_ROOT_CONTRACT_VERSION,
    RUN_ROOT_NAME_PATTERN,
    default_run_root_path,
    required_artifacts_m0_1,
    run_root_name_is_contract_valid,
    write_artifact_contract_manifest,
)

SUPPORTED_LANES: tuple[str, str] = ("clang", "fragilec")
COMMAND_NOT_FOUND_STATUS = 127
NON_COMPARABLE_KEYS: tuple[str, ...] = (
    "run_root",
    "harness_manifest",
    "inventory_manifest",
    "replay_manifest",
    "stage_timing_path",
    "stage_timing_parse_ms",
    "stage_timing_export_ms",
    "stage_timing_enrichment_ms",
    "stage_timing_codegen_ms",
    "stage_timing_total_ms",
    "stage_timing_error",
    "required_artifact_contract_manifest",
    "comparable_manifest",
    "comparable_manifest_sha256",
)


@dataclass(frozen=True)
class CommandResult:
    status: int
    stdout: str
    stderr: str


@dataclass(frozen=True)
class StageTimingSummary:
    exists: bool
    source_count: int
    status_count: int
    status: str
    last_stage_started: str
    last_stage_completed: str
    parse_ms: str
    export_ms: str
    enrichment_ms: str
    codegen_ms: str
    total_ms: str
    error: str


def shell_join(argv: Sequence[str]) -> str:
    return " ".join(shlex.quote(token) for token in argv)


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


def write_text(path: Path, value: str) -> None:
    path.write_text(value + "\n", encoding="utf-8")


def write_lines(path: Path, lines: list[str]) -> None:
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def parse_key_value_file(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        result[key.strip()] = value.strip()
    return result


def parse_manifest_lines(lines: Sequence[str]) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in lines:
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key.strip()] = value.strip()
    return values


def canonical_manifest_lines(values: Mapping[str, str]) -> list[str]:
    return [f"{key}={values[key]}" for key in sorted(values)]


def comparable_manifest(values: Mapping[str, str]) -> dict[str, str]:
    return {key: values[key] for key in sorted(values) if key not in NON_COMPARABLE_KEYS}


def manifest_sha256(values: Mapping[str, str]) -> str:
    digest = hashlib.sha256()
    for line in canonical_manifest_lines(values):
        digest.update(line.encode("utf-8"))
        digest.update(b"\n")
    return digest.hexdigest()


def run_capture(
    argv: list[str],
    *,
    env: Mapping[str, str],
    cwd: Path | None = None,
) -> CommandResult:
    try:
        output = subprocess.run(
            argv,
            check=False,
            capture_output=True,
            text=True,
            cwd=str(cwd) if cwd is not None else None,
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


def stage_timing_summary(path: Path) -> StageTimingSummary:
    if not path.exists():
        return StageTimingSummary(
            exists=False,
            source_count=0,
            status_count=0,
            status="none",
            last_stage_started="none",
            last_stage_completed="none",
            parse_ms="none",
            export_ms="none",
            enrichment_ms="none",
            codegen_ms="none",
            total_ms="none",
            error="none",
        )

    values = {
        "status": "none",
        "last_stage_started": "none",
        "last_stage_completed": "none",
        "parse_ms": "none",
        "export_ms": "none",
        "enrichment_ms": "none",
        "codegen_ms": "none",
        "total_ms": "none",
        "error": "none",
    }
    source_count = 0
    status_count = 0
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        key = key.strip()
        value = value.strip()
        if key == "source":
            source_count += 1
        elif key == "status":
            status_count += 1
        if key in values:
            values[key] = value

    return StageTimingSummary(
        exists=True,
        source_count=source_count,
        status_count=status_count,
        status=values["status"],
        last_stage_started=values["last_stage_started"],
        last_stage_completed=values["last_stage_completed"],
        parse_ms=values["parse_ms"],
        export_ms=values["export_ms"],
        enrichment_ms=values["enrichment_ms"],
        codegen_ms=values["codegen_ms"],
        total_ms=values["total_ms"],
        error=values["error"],
    )


def required_key(manifest: dict[str, str], key: str, *, source: str) -> str:
    if key not in manifest:
        raise KeyError(f"missing {source} manifest key: {key}")
    return manifest[key]


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    script_dir = Path(__file__).resolve().parent
    workspace_root = script_dir.parent
    default_mako_root = workspace_root / "vendor" / "mako"
    default_run_root = default_run_root_path(
        "m0_1_strict_baseline",
        base_dir=Path("/tmp"),
    )
    parser = argparse.ArgumentParser(
        description="Capture strict RPC baseline artifacts for TODO leaf M0.1."
    )
    parser.add_argument("--workspace-root", type=Path, default=workspace_root)
    parser.add_argument("--mako-root", type=Path, default=default_mako_root)
    parser.add_argument("--run-root", type=Path, default=default_run_root)
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
        "--stage-timing-path",
        type=Path,
        default=None,
        help="defaults to <run-root>/fragilec_transpile_stage_timing.log",
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
    if ns.jobs <= 0:
        raise ValueError(f"jobs must be > 0, got {ns.jobs}")
    if ns.trials <= 0:
        raise ValueError(f"trials must be > 0, got {ns.trials}")
    if ns.base_port < 1024 or ns.base_port > 65535:
        raise ValueError(f"base-port must be within [1024, 65535], got {ns.base_port}")
    if ns.build_timeout_seconds <= 0:
        raise ValueError(
            f"build-timeout-seconds must be > 0, got {ns.build_timeout_seconds}"
        )
    if ns.test_rpc_timeout_seconds <= 0:
        raise ValueError(
            "test-rpc-timeout-seconds must be > 0, got "
            f"{ns.test_rpc_timeout_seconds}"
        )
    if ns.rpc_client_timeout_seconds <= 0:
        raise ValueError(
            "rpc-client-timeout-seconds must be > 0, got "
            f"{ns.rpc_client_timeout_seconds}"
        )
    if ns.rpc_server_shutdown_timeout_seconds <= 0:
        raise ValueError(
            "rpc-server-shutdown-timeout-seconds must be > 0, got "
            f"{ns.rpc_server_shutdown_timeout_seconds}"
        )
    if ns.rpc_server_startup_wait_seconds < 0:
        raise ValueError(
            "rpc-server-startup-wait-seconds must be >= 0, got "
            f"{ns.rpc_server_startup_wait_seconds}"
        )
    if ns.replay_timeout_seconds <= 0:
        raise ValueError(
            f"replay-timeout-seconds must be > 0, got {ns.replay_timeout_seconds}"
        )
    if ns.replay_max_replays <= 0:
        raise ValueError(
            f"replay-max-replays must be > 0, got {ns.replay_max_replays}"
        )
    return ns


def main(argv: Sequence[str]) -> int:
    try:
        ns = parse_args(argv)
        lanes = parse_lanes(ns.lanes)
        workspace_root = ns.workspace_root.resolve()
        mako_root = ns.mako_root.resolve()
        run_root = ns.run_root.resolve()
        run_root.mkdir(parents=True, exist_ok=True)
        stage_timing_path = (
            ns.stage_timing_path.resolve()
            if ns.stage_timing_path is not None
            else (run_root / "fragilec_transpile_stage_timing.log")
        )
        if stage_timing_path.exists():
            stage_timing_path.unlink()

        base_env = dict(os.environ)
        strict_env = dict(base_env)
        strict_env["FRAGILEC_MODE"] = "strict"

        harness_cmd = [
            "python3",
            str(ns.harness_script.resolve()),
            "--workspace-root",
            str(workspace_root),
            "--mako-root",
            str(mako_root),
            "--run-root",
            str(run_root),
            "--lanes",
            ",".join(lanes),
            "--jobs",
            str(ns.jobs),
            "--trials",
            str(ns.trials),
            "--base-port",
            str(ns.base_port),
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
        ]
        if ns.build_only:
            harness_cmd.append("--build-only")

        inventory_cmd = [
            "python3",
            str(ns.inventory_script.resolve()),
            "--run-root",
            str(run_root),
            "--lanes",
            ",".join(lanes),
        ]

        replay_cmd = [
            "python3",
            str(ns.replay_script.resolve()),
            "--run-root",
            str(run_root),
            "--lanes",
            ",".join(lanes),
            "--max-replays",
            str(ns.replay_max_replays),
            "--timeout-seconds",
            str(ns.replay_timeout_seconds),
        ]
        replay_env = dict(strict_env)
        replay_env["FRAGILEC_TRANSPILE_STAGE_TIMING_PATH"] = str(stage_timing_path)

        write_lines(
            run_root / "strict_baseline_commands.txt",
            [
                "version=1",
                "task_leaf=M0.1",
                f"run_root={run_root}",
                "strict_env=FRAGILEC_MODE=strict",
                f"replay_stage_timing_path={stage_timing_path}",
                f"harness_command={shell_join(harness_cmd)}",
                f"inventory_command={shell_join(inventory_cmd)}",
                f"replay_command={shell_join(replay_cmd)}",
            ],
        )

        harness_result = run_capture(harness_cmd, env=strict_env)
        write_command_result(run_root, "strict_baseline_harness", harness_result)

        inventory_result = run_capture(inventory_cmd, env=base_env)
        write_command_result(run_root, "strict_baseline_inventory", inventory_result)
        if inventory_result.status != 0:
            raise RuntimeError(
                "inventory command failed; baseline capture incomplete (see strict_baseline_inventory.stderr.log)"
            )

        replay_result = run_capture(replay_cmd, env=replay_env)
        write_command_result(run_root, "strict_baseline_replay", replay_result)
        if replay_result.status != 0:
            raise RuntimeError(
                "replay command failed; baseline capture incomplete (see strict_baseline_replay.stderr.log)"
            )

        harness_manifest_path = run_root / "benchmark_harness_manifest.txt"
        inventory_manifest_path = run_root / "rpc_compile_blocker_inventory_manifest.txt"
        replay_manifest_path = run_root / "rpc_compile_blocker_replay_manifest.txt"
        if not harness_manifest_path.exists():
            raise FileNotFoundError(
                f"missing harness manifest artifact: {harness_manifest_path}"
            )
        if not inventory_manifest_path.exists():
            raise FileNotFoundError(
                f"missing inventory manifest artifact: {inventory_manifest_path}"
            )
        if not replay_manifest_path.exists():
            raise FileNotFoundError(
                f"missing replay manifest artifact: {replay_manifest_path}"
            )

        harness_manifest = parse_key_value_file(harness_manifest_path)
        inventory_manifest = parse_key_value_file(inventory_manifest_path)
        replay_manifest = parse_key_value_file(replay_manifest_path)
        timing_summary = stage_timing_summary(stage_timing_path)

        lines = [
            "version=1",
            "task_leaf=M0.1",
            f"run_root={run_root}",
            "strict_mode=true",
            f"lanes={','.join(lanes)}",
            f"harness_status={harness_result.status}",
            f"inventory_status={inventory_result.status}",
            f"replay_status={replay_result.status}",
            f"harness_manifest={harness_manifest_path}",
            f"inventory_manifest={inventory_manifest_path}",
            f"replay_manifest={replay_manifest_path}",
            f"stage_timing_path={stage_timing_path}",
            f"stage_timing_exists={'true' if timing_summary.exists else 'false'}",
            f"stage_timing_source_count={timing_summary.source_count}",
            f"stage_timing_status_count={timing_summary.status_count}",
            f"stage_timing_status={timing_summary.status}",
            f"stage_timing_last_stage_started={timing_summary.last_stage_started}",
            f"stage_timing_last_stage_completed={timing_summary.last_stage_completed}",
            f"stage_timing_parse_ms={timing_summary.parse_ms}",
            f"stage_timing_export_ms={timing_summary.export_ms}",
            f"stage_timing_enrichment_ms={timing_summary.enrichment_ms}",
            f"stage_timing_codegen_ms={timing_summary.codegen_ms}",
            f"stage_timing_total_ms={timing_summary.total_ms}",
            f"stage_timing_error={timing_summary.error}",
            f"replay_selected_count={replay_manifest.get('selected_count', '0')}",
            f"replay_01_blocker_class={replay_manifest.get('replay_01_blocker_class', 'none')}",
            f"replay_01_blocker_file={replay_manifest.get('replay_01_blocker_file', 'none')}",
            f"replay_01_status={replay_manifest.get('replay_01_status', 'none')}",
            f"replay_01_timed_out={replay_manifest.get('replay_01_timed_out', 'none')}",
            (
                "replay_01_first_failure_class="
                f"{replay_manifest.get('replay_01_first_failure_class', 'none')}"
            ),
        ]

        for lane in lanes:
            lines.extend(
                [
                    (
                        f"lane_{lane}_configure_status="
                        f"{required_key(harness_manifest, f'lane_{lane}_configure_status', source='harness')}"
                    ),
                    (
                        f"lane_{lane}_clean_status="
                        f"{required_key(harness_manifest, f'lane_{lane}_clean_status', source='harness')}"
                    ),
                    (
                        f"lane_{lane}_build_status="
                        f"{required_key(harness_manifest, f'lane_{lane}_build_status', source='harness')}"
                    ),
                    (
                        f"lane_{lane}_test_rpc_status="
                        f"{required_key(harness_manifest, f'lane_{lane}_test_rpc_status', source='harness')}"
                    ),
                    (
                        f"lane_{lane}_failure_class="
                        f"{required_key(harness_manifest, f'lane_{lane}_failure_class', source='harness')}"
                    ),
                    (
                        f"lane_{lane}_first_failing_compile_class="
                        f"{required_key(inventory_manifest, f'lane_{lane}_first_failing_compile_class', source='inventory')}"
                    ),
                    (
                        f"lane_{lane}_first_failing_compile_file="
                        f"{required_key(inventory_manifest, f'lane_{lane}_first_failing_compile_file', source='inventory')}"
                    ),
                    (
                        f"lane_{lane}_first_failing_compile_e0425_count="
                        f"{required_key(inventory_manifest, f'lane_{lane}_first_failing_compile_e0425_count', source='inventory')}"
                    ),
                ]
            )

        lines.extend(
            [
                f"run_root_contract_version={RUN_ROOT_CONTRACT_VERSION}",
                f"run_root_name_pattern={RUN_ROOT_NAME_PATTERN}",
                (
                    "run_root_name_is_contract_valid="
                    f"{'true' if run_root_name_is_contract_valid(run_root.name) else 'false'}"
                ),
            ]
        )
        manifest_path = run_root / "strict_baseline_manifest.txt"
        write_lines(manifest_path, lines)
        artifact_contract_summary = write_artifact_contract_manifest(
            manifest_path=run_root / "strict_baseline_required_artifacts_manifest.txt",
            task_leaf="M0.1",
            run_root=run_root,
            required_relpaths=required_artifacts_m0_1(),
        )
        lines.extend(
            [
                "required_artifact_contract_version=1",
                (
                    "required_artifact_contract_manifest="
                    f"{artifact_contract_summary.manifest_path}"
                ),
                (
                    "required_artifact_count="
                    f"{artifact_contract_summary.expected_count}"
                ),
                (
                    "missing_required_artifact_count="
                    f"{artifact_contract_summary.missing_count}"
                ),
            ]
        )
        manifest_values = parse_manifest_lines(lines)
        comparable_values = comparable_manifest(manifest_values)
        comparable_manifest_path = run_root / "strict_baseline_comparable_manifest.txt"
        write_lines(
            comparable_manifest_path,
            canonical_manifest_lines(comparable_values),
        )
        lines.extend(
            [
                f"comparable_manifest={comparable_manifest_path}",
                f"comparable_manifest_sha256={manifest_sha256(comparable_values)}",
                f"comparable_manifest_key_count={len(comparable_values)}",
                f"non_comparable_keys={','.join(NON_COMPARABLE_KEYS)}",
            ]
        )
        write_lines(manifest_path, lines)
        print(run_root)
        return 0
    except Exception as exc:  # pylint: disable=broad-except
        print(f"error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
