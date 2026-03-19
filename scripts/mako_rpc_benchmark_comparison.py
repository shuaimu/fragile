#!/usr/bin/env python3
"""Deterministic clang vs fragile benchmark comparison for TODO leaf M9.3.

This script executes the existing RPC harness with both `clang` and `fragilec`
lanes, captures deterministic benchmark manifests, and enforces the performance
no-regression gate:

    fragile_avg_qps >= clang_avg_qps

Success requires:
- Both lanes configure/clean/build successfully.
- `test_rpc` passes in both lanes (M9.A1 gate).
- All rpcbench server/client trials complete in both lanes (M9.A2 gate).
- `fragile_avg_qps >= clang_avg_qps` (M9.A3 performance gate).
"""

from __future__ import annotations

import argparse
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
    required_artifacts_m9_3,
    run_root_name_is_contract_valid,
    write_artifact_contract_manifest,
)

COMMAND_NOT_FOUND_STATUS = 127
STRICT_MODE_ENV = "FRAGILEC_MODE"
STRICT_MODE_VALUE = "strict"
PARSER_BACKEND_ENV = "FRAGILEC_PARSER_BACKEND"
STRICT_PARSER_BACKEND = "fragile-parser-clang"
FORCE_NATIVE_SOURCES_ENV = "FRAGILEC_FORCE_NATIVE_SOURCES"
PARSER_CORE_CODEGEN_ESCAPE_HATCH_ENV = "FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH"
DEFAULT_LANES = ("clang", "fragilec")


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


def normalized_nonempty(value: str | None) -> str | None:
    if value is None:
        return None
    normalized = value.strip()
    if not normalized:
        return None
    return normalized


def env_value_is_truthy(value: str | None) -> bool:
    normalized = normalized_nonempty(value)
    if normalized is None:
        return False
    return normalized.lower() in {"1", "true", "yes", "on"}


def assert_parent_env_is_strict_contract_compatible(
    base_env: Mapping[str, str],
) -> None:
    parser_backend = normalized_nonempty(base_env.get(PARSER_BACKEND_ENV))
    if parser_backend is not None and parser_backend != STRICT_PARSER_BACKEND:
        raise ValueError(
            f"{PARSER_BACKEND_ENV}={parser_backend} is incompatible with strict "
            f"benchmark comparison contract; expected `{STRICT_PARSER_BACKEND}` when set"
        )

    if env_value_is_truthy(base_env.get(FORCE_NATIVE_SOURCES_ENV)):
        raise ValueError(
            f"{FORCE_NATIVE_SOURCES_ENV} enables forbidden native bypass; unset it "
            "or set it to a falsey value before running benchmark comparison"
        )

    parser_core_escape_hatch = normalized_nonempty(
        base_env.get(PARSER_CORE_CODEGEN_ESCAPE_HATCH_ENV)
    )
    if parser_core_escape_hatch is not None:
        raise ValueError(
            f"{PARSER_CORE_CODEGEN_ESCAPE_HATCH_ENV}={parser_core_escape_hatch} is "
            "incompatible with strict benchmark comparison contract; escape hatch must be unset"
        )


def strict_benchmark_env(base_env: Mapping[str, str]) -> dict[str, str]:
    assert_parent_env_is_strict_contract_compatible(base_env)
    strict_env = dict(base_env)
    strict_env[STRICT_MODE_ENV] = STRICT_MODE_VALUE
    strict_env[PARSER_BACKEND_ENV] = STRICT_PARSER_BACKEND
    strict_env.pop(FORCE_NATIVE_SOURCES_ENV, None)
    strict_env.pop(PARSER_CORE_CODEGEN_ESCAPE_HATCH_ENV, None)
    return strict_env


def parse_positive_int(raw: str, *, field: str) -> int:
    try:
        value = int(raw)
    except ValueError as exc:
        raise ValueError(f"{field} must be an integer, got {raw!r}") from exc
    if value <= 0:
        raise ValueError(f"{field} must be > 0, got {value}")
    return value


def parse_any_int(raw: str, *, field: str) -> int:
    try:
        return int(raw)
    except ValueError as exc:
        raise ValueError(f"{field} must be an integer, got {raw!r}") from exc


def required_key(values: Mapping[str, str], key: str, *, source: str) -> str:
    if key not in values:
        raise KeyError(f"missing {source} manifest key: {key}")
    return values[key]


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    script_dir = Path(__file__).resolve().parent
    workspace_root = script_dir.parent
    default_mako_root = workspace_root / "vendor" / "mako"
    default_run_root = default_run_root_path(
        "m9_3_benchmark_comparison",
        base_dir=Path("/tmp"),
    )
    parser = argparse.ArgumentParser(
        description="Run deterministic clang vs fragile benchmark comparison for TODO leaf M9.3."
    )
    parser.add_argument("--workspace-root", type=Path, default=workspace_root)
    parser.add_argument("--mako-root", type=Path, default=default_mako_root)
    parser.add_argument("--run-root", type=Path, default=default_run_root)
    parser.add_argument(
        "--fragile-cxx",
        type=Path,
        default=workspace_root / "target" / "release" / "fragilec",
    )
    parser.add_argument("--clang-cxx", default="clang++")
    parser.add_argument(
        "--skip-fragilec-build",
        action="store_true",
        help="skip cargo build for fragilec (useful for deterministic fake-harness tests)",
    )
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--trials", type=int, default=3)
    parser.add_argument("--base-port", type=int, default=24900)
    parser.add_argument("--build-timeout-seconds", type=int, default=3600)
    parser.add_argument("--configure-timeout-seconds", type=int, default=900)
    parser.add_argument("--clean-timeout-seconds", type=int, default=300)
    parser.add_argument("--test-rpc-timeout-seconds", type=int, default=120)
    parser.add_argument("--rpc-client-timeout-seconds", type=int, default=120)
    parser.add_argument("--rpc-server-startup-wait-seconds", type=float, default=1.0)
    parser.add_argument("--rpc-server-shutdown-timeout-seconds", type=int, default=15)
    parser.add_argument("--rpc-duration-seconds", type=int, default=10)
    parser.add_argument("--rpc-client-threads", type=int, default=8)
    parser.add_argument("--rpc-outstanding", type=int, default=1000)
    parser.add_argument("--rpc-worker-threads", type=int, default=16)
    parser.add_argument("--rpc-epoll-instances", type=int, default=2)
    parser.add_argument("--rpc-payload-bytes", type=int, default=10)
    parser.add_argument(
        "--harness-script",
        type=Path,
        default=script_dir / "mako_rpcbench_harness.py",
    )
    ns = parser.parse_args(list(argv))
    if ns.jobs <= 0:
        raise ValueError(f"jobs must be > 0, got {ns.jobs}")
    if ns.trials <= 0:
        raise ValueError(f"trials must be > 0, got {ns.trials}")
    if ns.base_port < 1024 or ns.base_port > 65535:
        raise ValueError(
            f"base-port must be within [1024, 65535], got {ns.base_port}"
        )
    for field_name in (
        "build_timeout_seconds",
        "configure_timeout_seconds",
        "clean_timeout_seconds",
        "test_rpc_timeout_seconds",
        "rpc_client_timeout_seconds",
        "rpc_server_shutdown_timeout_seconds",
        "rpc_duration_seconds",
        "rpc_client_threads",
        "rpc_outstanding",
        "rpc_worker_threads",
        "rpc_epoll_instances",
        "rpc_payload_bytes",
    ):
        if getattr(ns, field_name) <= 0:
            raise ValueError(
                f"{field_name.replace('_', '-')} must be > 0, "
                f"got {getattr(ns, field_name)}"
            )
    if ns.rpc_server_startup_wait_seconds < 0:
        raise ValueError(
            "rpc-server-startup-wait-seconds must be >= 0, "
            f"got {ns.rpc_server_startup_wait_seconds}"
        )
    return ns


def main(argv: Sequence[str]) -> int:
    try:
        ns = parse_args(argv)
        workspace_root = ns.workspace_root.resolve()
        mako_root = ns.mako_root.resolve()
        run_root = ns.run_root.resolve()
        run_root.mkdir(parents=True, exist_ok=True)

        if not workspace_root.exists():
            raise FileNotFoundError(
                f"workspace root does not exist: {workspace_root}"
            )
        if not mako_root.exists():
            raise FileNotFoundError(f"mako root does not exist: {mako_root}")
        if not mako_root.joinpath("CMakeLists.txt").exists():
            raise FileNotFoundError(
                f"mako root is missing CMakeLists.txt: {mako_root / 'CMakeLists.txt'}"
            )

        base_env = dict(os.environ)
        strict_env = strict_benchmark_env(base_env)
        fragilec_path = ns.fragile_cxx.resolve()

        # Step 1: build fragilec
        fragilec_build_cmd = [
            "cargo", "build", "-p", "fragile-cli", "--bin", "fragilec",
        ]
        if ns.skip_fragilec_build:
            fragilec_build_result = CommandResult(
                status=0,
                stdout="",
                stderr="skipped: --skip-fragilec-build enabled\n",
            )
        else:
            fragilec_build_result = run_capture(
                fragilec_build_cmd,
                env=base_env,
                cwd=workspace_root,
            )
        write_command_result(
            run_root,
            "benchmark_comparison_fragilec_build",
            fragilec_build_result,
        )
        if fragilec_build_result.status != 0:
            raise RuntimeError(
                "failed to build fragilec before benchmark comparison; "
                "see benchmark_comparison_fragilec_build.stderr.log"
            )
        if not ns.skip_fragilec_build and not fragilec_path.exists():
            raise FileNotFoundError(
                f"fragilec binary not found after build: {fragilec_path}"
            )

        # Step 2: invoke harness with both lanes
        lanes_str = ",".join(DEFAULT_LANES)
        harness_cmd = [
            "python3",
            str(ns.harness_script.resolve()),
            "--workspace-root",
            str(workspace_root),
            "--mako-root",
            str(mako_root),
            "--run-root",
            str(run_root),
            "--fragile-cxx",
            str(fragilec_path),
            "--clang-cxx",
            ns.clang_cxx,
            "--lanes",
            lanes_str,
            "--jobs",
            str(ns.jobs),
            "--trials",
            str(ns.trials),
            "--base-port",
            str(ns.base_port),
            "--configure-timeout-seconds",
            str(ns.configure_timeout_seconds),
            "--clean-timeout-seconds",
            str(ns.clean_timeout_seconds),
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
            "--rpc-duration-seconds",
            str(ns.rpc_duration_seconds),
            "--rpc-client-threads",
            str(ns.rpc_client_threads),
            "--rpc-outstanding",
            str(ns.rpc_outstanding),
            "--rpc-worker-threads",
            str(ns.rpc_worker_threads),
            "--rpc-epoll-instances",
            str(ns.rpc_epoll_instances),
            "--rpc-payload-bytes",
            str(ns.rpc_payload_bytes),
        ]

        write_lines(
            run_root / "benchmark_comparison_commands.txt",
            [
                "version=1",
                "task_leaf=M9.3",
                f"run_root={run_root}",
                "strict_env=FRAGILEC_MODE=strict FRAGILEC_PARSER_BACKEND=fragile-parser-clang",
                "strict_env_force_native_sources=unset",
                "strict_env_parser_core_codegen_escape_hatch=unset",
                f"fragilec_build_command={shell_join(fragilec_build_cmd)}",
                f"fragile_cxx={fragilec_path}",
                f"clang_cxx={ns.clang_cxx}",
                f"lanes={lanes_str}",
                f"harness_command={shell_join(harness_cmd)}",
            ],
        )

        harness_result = run_capture(harness_cmd, env=strict_env)
        write_command_result(
            run_root, "benchmark_comparison_harness", harness_result
        )

        # Step 3: read harness + comparison manifests
        harness_manifest_path = run_root / "benchmark_harness_manifest.txt"
        comparison_manifest_path = (
            run_root / "benchmark_qps_comparison_manifest.txt"
        )
        if not harness_manifest_path.exists():
            raise FileNotFoundError(
                f"missing harness manifest artifact: {harness_manifest_path}"
            )
        if not comparison_manifest_path.exists():
            raise FileNotFoundError(
                f"missing comparison manifest artifact: {comparison_manifest_path}"
            )

        harness_manifest = parse_key_value_file(harness_manifest_path)
        comparison_manifest = parse_key_value_file(comparison_manifest_path)

        requested_trials = parse_positive_int(
            required_key(harness_manifest, "trials", source="harness"),
            field="harness trials",
        )
        if requested_trials != ns.trials:
            raise ValueError(
                f"harness trials mismatch: expected {ns.trials}, got {requested_trials}"
            )

        # Step 4: extract per-lane metrics
        lane_metrics: dict[str, dict[str, str]] = {}
        for lane in DEFAULT_LANES:
            lane_build_status = parse_any_int(
                required_key(
                    harness_manifest,
                    f"lane_{lane}_build_status",
                    source="harness",
                ),
                field=f"lane_{lane}_build_status",
            )
            lane_test_rpc_status = parse_any_int(
                required_key(
                    harness_manifest,
                    f"lane_{lane}_test_rpc_status",
                    source="harness",
                ),
                field=f"lane_{lane}_test_rpc_status",
            )
            lane_completed_trials = parse_any_int(
                required_key(
                    harness_manifest,
                    f"lane_{lane}_completed_trials",
                    source="harness",
                ),
                field=f"lane_{lane}_completed_trials",
            )
            lane_failure_class = required_key(
                harness_manifest,
                f"lane_{lane}_failure_class",
                source="harness",
            )
            lane_avg_qps = required_key(
                harness_manifest,
                f"lane_{lane}_avg_qps",
                source="harness",
            )
            lane_metrics[lane] = {
                "build_status": str(lane_build_status),
                "test_rpc_status": str(lane_test_rpc_status),
                "completed_trials": str(lane_completed_trials),
                "failure_class": lane_failure_class,
                "avg_qps": lane_avg_qps,
            }

        # Step 5: extract comparison-level metrics
        no_regression_verdict = required_key(
            comparison_manifest, "no_regression_verdict", source="comparison"
        )
        clang_avg_qps = required_key(
            comparison_manifest, "clang_avg_qps", source="comparison"
        )
        fragile_avg_qps = required_key(
            comparison_manifest, "fragile_avg_qps", source="comparison"
        )
        fragile_minus_clang_qps = required_key(
            comparison_manifest,
            "fragile_minus_clang_qps",
            source="comparison",
        )
        fragile_over_clang_ratio = required_key(
            comparison_manifest,
            "fragile_over_clang_ratio",
            source="comparison",
        )

        # Step 6: emit M9.3 benchmark comparison manifest
        lines = [
            "version=1",
            "task_leaf=M9.3",
            f"run_root={run_root}",
            "strict_mode=true",
            f"strict_env_mode={strict_env.get(STRICT_MODE_ENV, 'none')}",
            f"strict_env_parser_backend={strict_env.get(PARSER_BACKEND_ENV, 'none')}",
            "strict_env_force_native_sources=unset",
            "strict_env_parser_core_codegen_escape_hatch=unset",
            f"lanes={lanes_str}",
            f"requested_trials={ns.trials}",
            f"harness_status={harness_result.status}",
            f"harness_manifest={harness_manifest_path}",
            f"comparison_manifest={comparison_manifest_path}",
            f"no_regression_verdict={no_regression_verdict}",
            f"clang_avg_qps={clang_avg_qps}",
            f"fragile_avg_qps={fragile_avg_qps}",
            f"fragile_minus_clang_qps={fragile_minus_clang_qps}",
            f"fragile_over_clang_ratio={fragile_over_clang_ratio}",
        ]
        for lane in DEFAULT_LANES:
            metrics = lane_metrics[lane]
            lines.extend(
                [
                    f"lane_{lane}_build_status={metrics['build_status']}",
                    f"lane_{lane}_test_rpc_status={metrics['test_rpc_status']}",
                    f"lane_{lane}_completed_trials={metrics['completed_trials']}",
                    f"lane_{lane}_failure_class={metrics['failure_class']}",
                    f"lane_{lane}_avg_qps={metrics['avg_qps']}",
                ]
            )

        # M9.A1 gate: test_rpc build/run pass in both lanes
        m9_a1_pass = all(
            lane_metrics[lane]["build_status"] == "0"
            and lane_metrics[lane]["test_rpc_status"] == "0"
            for lane in DEFAULT_LANES
        )
        lines.append(f"m9_a1_test_rpc_gate={'pass' if m9_a1_pass else 'fail'}")

        # M9.A2 gate: rpcbench server/client runtime in both lanes
        m9_a2_pass = all(
            lane_metrics[lane]["failure_class"] == "none"
            and lane_metrics[lane]["completed_trials"] == str(ns.trials)
            for lane in DEFAULT_LANES
        )
        lines.append(
            f"m9_a2_rpcbench_runtime_gate={'pass' if m9_a2_pass else 'fail'}"
        )

        # M9.A3 gate: performance no-regression
        m9_a3_pass = no_regression_verdict == "pass"
        lines.append(
            f"m9_a3_performance_gate={'pass' if m9_a3_pass else 'fail'}"
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
        manifest_path = run_root / "benchmark_comparison_manifest.txt"
        write_lines(manifest_path, lines)

        # Step 7: write artifact contract
        artifact_contract_summary = write_artifact_contract_manifest(
            manifest_path=(
                run_root / "benchmark_comparison_required_artifacts_manifest.txt"
            ),
            task_leaf="M9.3",
            run_root=run_root,
            required_relpaths=required_artifacts_m9_3(trials=ns.trials),
        )
        lines.extend(
            [
                "required_artifact_contract_version=1",
                (
                    "required_artifact_contract_manifest="
                    f"{artifact_contract_summary.manifest_path}"
                ),
                f"required_artifact_count={artifact_contract_summary.expected_count}",
                f"missing_required_artifact_count={artifact_contract_summary.missing_count}",
            ]
        )
        write_lines(manifest_path, lines)

        if artifact_contract_summary.missing_count != 0:
            raise RuntimeError(
                "benchmark comparison artifact contract has missing artifacts; "
                "see benchmark_comparison_required_artifacts_manifest.txt"
            )

        # Step 8: enforce gates
        # Harness exit code 1 is expected when verdict is fail or
        # insufficient_data. Only reject unexpected exit codes (>1).
        if harness_result.status not in {0, 1}:
            raise RuntimeError(
                f"harness exited with unexpected status {harness_result.status}; "
                "see benchmark_comparison_harness.stderr.log"
            )
        if not m9_a1_pass:
            raise RuntimeError(
                "M9.A1 gate failed: test_rpc build/run did not pass in both lanes"
            )
        if not m9_a2_pass:
            raise RuntimeError(
                "M9.A2 gate failed: rpcbench server/client runtime did not pass in both lanes"
            )
        if not m9_a3_pass:
            raise RuntimeError(
                "M9.A3 gate failed: performance no-regression verdict is "
                f"{no_regression_verdict!r} (expected 'pass'); "
                f"clang_avg_qps={clang_avg_qps}, fragile_avg_qps={fragile_avg_qps}"
            )

        print(run_root)
        return 0
    except Exception as exc:  # pylint: disable=broad-except
        print(f"error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
