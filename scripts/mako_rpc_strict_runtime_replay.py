#!/usr/bin/env python3
"""Deterministic strict runtime replay capture for TODO leaf M9.2.

This script executes the existing RPC harness in strict fragilec mode for the
`fragilec` lane only and captures deterministic runtime manifests proving:
- `test_rpc` runtime step passed,
- rpcbench server/client runtime trials completed,
- strict no-bypass environment contract was enforced.

The harness' QPS comparison gate is performance-oriented and belongs to M9.3.
For M9.2 runtime-only evidence, a harness exit of `1` is accepted when the
manifest shows `no_regression_verdict=insufficient_data` while all runtime
steps passed in the strict lane.
"""

from __future__ import annotations

import argparse
import os
import re
import shlex
import subprocess
import sys
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping, Sequence

from mako_rpc_milestone_contract import (
    RUN_ROOT_CONTRACT_VERSION,
    RUN_ROOT_NAME_PATTERN,
    default_run_root_path,
    required_artifacts_m9_2,
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
RUSTC_ERROR_RE = re.compile(r"^error(?:\[(E\d{4})\])?:\s*(.+)$")
GMAKE_ERROR_RE = re.compile(r"^gmake(?:\[\d+\])?:\s+\*\*\*\s+(.+)$")


@dataclass(frozen=True)
class CommandResult:
    status: int
    stdout: str
    stderr: str


@dataclass(frozen=True)
class RuntimeStatusSummary:
    requested_trials: int
    passed_trials: int
    failed_trials: int
    all_trials_passed: bool


@dataclass(frozen=True)
class BlockerInventorySummary:
    manifest_path: Path
    total_count: int
    unique_count: int
    first_error_key: str
    baseline_total_count: int | None
    baseline_unique_count: int | None
    non_increase_total_vs_baseline: str
    non_increase_unique_vs_baseline: str
    non_increase_verdict: str


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


def normalize_error_key(error_code: str | None, message: str) -> str:
    normalized_message = " ".join(message.split())
    if error_code:
        return f"{error_code}:{normalized_message}"
    return f"error:{normalized_message}"


def collect_rustc_error_keys(stderr_text: str) -> list[str]:
    keys: list[str] = []
    for line in stderr_text.splitlines():
        normalized = line.strip()
        match = RUSTC_ERROR_RE.match(normalized)
        if match is None:
            if normalized.startswith("[fragilec]"):
                keys.append(normalize_error_key(None, f"fragilec:{normalized}"))
                continue
            gmake_match = GMAKE_ERROR_RE.match(normalized)
            if gmake_match is not None:
                keys.append(
                    normalize_error_key(
                        None, f"gmake:{gmake_match.group(1)}"
                    )
                )
            continue
        keys.append(normalize_error_key(match.group(1), match.group(2)))
    return keys


def parse_inventory_counts(path: Path) -> tuple[int, int]:
    values = parse_key_value_file(path)
    return (
        parse_any_int(
            required_key(values, "rustc_error_total_count", source=str(path)),
            field=f"{path} rustc_error_total_count",
        ),
        parse_any_int(
            required_key(values, "rustc_error_unique_count", source=str(path)),
            field=f"{path} rustc_error_unique_count",
        ),
    )


def baseline_inventory_counts(
    *,
    baseline_run_root: Path,
    lane: str,
) -> tuple[int, int]:
    manifest = baseline_run_root / "strict_runtime_replay_blocker_inventory_manifest.txt"
    if manifest.exists():
        return parse_inventory_counts(manifest)

    fallback_build_stderr = baseline_run_root / f"lane_{lane}" / "build.stderr"
    if fallback_build_stderr.exists():
        keys = collect_rustc_error_keys(
            fallback_build_stderr.read_text(encoding="utf-8", errors="replace")
        )
        return (len(keys), len(set(keys)))

    raise FileNotFoundError(
        "baseline run root is missing blocker inventory and build stderr: "
        f"{baseline_run_root}"
    )


def write_blocker_inventory_manifest(
    *,
    run_root: Path,
    lane: str,
    baseline_run_root: Path | None,
) -> BlockerInventorySummary:
    build_stderr_path = run_root / f"lane_{lane}" / "build.stderr"
    if build_stderr_path.exists():
        build_stderr = build_stderr_path.read_text(encoding="utf-8", errors="replace")
    else:
        build_stderr = ""

    error_keys = collect_rustc_error_keys(build_stderr)
    counts = Counter(error_keys)
    unique_keys = sorted(counts.keys())
    first_error_key = error_keys[0] if error_keys else "none"

    baseline_total: int | None = None
    baseline_unique: int | None = None
    non_increase_total = "unknown"
    non_increase_unique = "unknown"
    non_increase_verdict = "unknown"
    if baseline_run_root is not None:
        baseline_total, baseline_unique = baseline_inventory_counts(
            baseline_run_root=baseline_run_root,
            lane=lane,
        )
        total_ok = len(error_keys) <= baseline_total
        unique_ok = len(unique_keys) <= baseline_unique
        non_increase_total = "true" if total_ok else "false"
        non_increase_unique = "true" if unique_ok else "false"
        non_increase_verdict = "true" if total_ok and unique_ok else "false"

    lines = [
        "version=1",
        "task_leaf=M9.2.c.iv.a",
        f"run_root={run_root}",
        f"lane={lane}",
        f"source_build_stderr={build_stderr_path}",
        f"source_build_stderr_exists={'true' if build_stderr_path.exists() else 'false'}",
        f"rustc_error_total_count={len(error_keys)}",
        f"rustc_error_unique_count={len(unique_keys)}",
        f"first_error_key={first_error_key}",
        (
            "baseline_run_root="
            f"{baseline_run_root if baseline_run_root is not None else 'none'}"
        ),
        f"baseline_error_total_count={baseline_total if baseline_total is not None else -1}",
        (
            "baseline_error_unique_count="
            f"{baseline_unique if baseline_unique is not None else -1}"
        ),
        f"non_increase_total_vs_baseline={non_increase_total}",
        f"non_increase_unique_vs_baseline={non_increase_unique}",
        f"non_increase_verdict={non_increase_verdict}",
    ]
    for index, key in enumerate(unique_keys, start=1):
        lines.extend(
            [
                f"error_key_{index:03d}={key}",
                f"error_key_{index:03d}_count={counts[key]}",
            ]
        )

    manifest_path = run_root / "strict_runtime_replay_blocker_inventory_manifest.txt"
    write_lines(manifest_path, lines)
    return BlockerInventorySummary(
        manifest_path=manifest_path,
        total_count=len(error_keys),
        unique_count=len(unique_keys),
        first_error_key=first_error_key,
        baseline_total_count=baseline_total,
        baseline_unique_count=baseline_unique,
        non_increase_total_vs_baseline=non_increase_total,
        non_increase_unique_vs_baseline=non_increase_unique,
        non_increase_verdict=non_increase_verdict,
    )


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


def assert_parent_env_is_strict_contract_compatible(base_env: Mapping[str, str]) -> None:
    parser_backend = normalized_nonempty(base_env.get(PARSER_BACKEND_ENV))
    if parser_backend is not None and parser_backend != STRICT_PARSER_BACKEND:
        raise ValueError(
            f"{PARSER_BACKEND_ENV}={parser_backend} is incompatible with strict runtime "
            f"replay contract; expected `{STRICT_PARSER_BACKEND}` when set"
        )

    if env_value_is_truthy(base_env.get(FORCE_NATIVE_SOURCES_ENV)):
        raise ValueError(
            f"{FORCE_NATIVE_SOURCES_ENV} enables forbidden native bypass; unset it "
            "or set it to a falsey value before running strict runtime replay"
        )

    parser_core_escape_hatch = normalized_nonempty(
        base_env.get(PARSER_CORE_CODEGEN_ESCAPE_HATCH_ENV)
    )
    if parser_core_escape_hatch is not None:
        raise ValueError(
            f"{PARSER_CORE_CODEGEN_ESCAPE_HATCH_ENV}={parser_core_escape_hatch} is "
            "incompatible with strict runtime replay contract; escape hatch must be unset"
        )


def strict_runtime_replay_env(base_env: Mapping[str, str]) -> dict[str, str]:
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
        "m9_2_strict_runtime_replay",
        base_dir=Path("/tmp"),
    )
    parser = argparse.ArgumentParser(
        description="Capture strict runtime replay artifacts for TODO leaf M9.2."
    )
    parser.add_argument("--workspace-root", type=Path, default=workspace_root)
    parser.add_argument("--mako-root", type=Path, default=default_mako_root)
    parser.add_argument("--run-root", type=Path, default=default_run_root)
    parser.add_argument(
        "--fragile-cxx",
        type=Path,
        default=workspace_root / "target" / "release" / "fragilec",
    )
    parser.add_argument(
        "--skip-fragilec-build",
        action="store_true",
        help="skip cargo build for fragilec (useful for deterministic fake-harness tests)",
    )
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--trials", type=int, default=1)
    parser.add_argument(
        "--baseline-run-root",
        type=Path,
        default=None,
        help=(
            "optional prior strict runtime replay run root used for deterministic "
            "blocker non-increase comparison"
        ),
    )
    parser.add_argument("--base-port", type=int, default=23900)
    parser.add_argument("--build-timeout-seconds", type=int, default=3600)
    parser.add_argument("--test-rpc-timeout-seconds", type=int, default=120)
    parser.add_argument("--rpc-client-timeout-seconds", type=int, default=120)
    parser.add_argument("--rpc-server-startup-wait-seconds", type=float, default=1.0)
    parser.add_argument("--rpc-server-shutdown-timeout-seconds", type=int, default=15)
    parser.add_argument("--rpc-duration-seconds", type=int, default=5)
    parser.add_argument("--rpc-client-threads", type=int, default=8)
    parser.add_argument("--rpc-outstanding", type=int, default=1000)
    parser.add_argument("--rpc-worker-threads", type=int, default=16)
    parser.add_argument("--rpc-epoll-instances", type=int, default=2)
    parser.add_argument("--rpc-payload-bytes", type=int, default=10)
    parser.add_argument(
        "--skip-masstree-perf-target",
        dest="skip_masstree_perf_target",
        action="store_true",
        default=True,
        help="skip masstree_perf build target for runtime-focused replay (default)",
    )
    parser.add_argument(
        "--include-masstree-perf-target",
        dest="skip_masstree_perf_target",
        action="store_false",
        help="include masstree_perf build target in strict runtime replay",
    )
    parser.add_argument(
        "--skip-clean-step",
        dest="skip_clean_step",
        action="store_true",
        default=True,
        help="skip clean target before build for resumable replay (default)",
    )
    parser.add_argument(
        "--run-clean-step",
        dest="skip_clean_step",
        action="store_false",
        help="run clean target before build in strict runtime replay",
    )
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
        raise ValueError(f"base-port must be within [1024, 65535], got {ns.base_port}")
    if ns.build_timeout_seconds <= 0:
        raise ValueError(
            f"build-timeout-seconds must be > 0, got {ns.build_timeout_seconds}"
        )
    if ns.test_rpc_timeout_seconds <= 0:
        raise ValueError(
            f"test-rpc-timeout-seconds must be > 0, got {ns.test_rpc_timeout_seconds}"
        )
    if ns.rpc_client_timeout_seconds <= 0:
        raise ValueError(
            "rpc-client-timeout-seconds must be > 0, got "
            f"{ns.rpc_client_timeout_seconds}"
        )
    if ns.rpc_server_startup_wait_seconds < 0:
        raise ValueError(
            "rpc-server-startup-wait-seconds must be >= 0, got "
            f"{ns.rpc_server_startup_wait_seconds}"
        )
    if ns.rpc_server_shutdown_timeout_seconds <= 0:
        raise ValueError(
            "rpc-server-shutdown-timeout-seconds must be > 0, got "
            f"{ns.rpc_server_shutdown_timeout_seconds}"
        )
    if ns.rpc_duration_seconds <= 0:
        raise ValueError(
            f"rpc-duration-seconds must be > 0, got {ns.rpc_duration_seconds}"
        )
    if ns.rpc_client_threads <= 0:
        raise ValueError(
            f"rpc-client-threads must be > 0, got {ns.rpc_client_threads}"
        )
    if ns.rpc_outstanding <= 0:
        raise ValueError(f"rpc-outstanding must be > 0, got {ns.rpc_outstanding}")
    if ns.rpc_worker_threads <= 0:
        raise ValueError(
            f"rpc-worker-threads must be > 0, got {ns.rpc_worker_threads}"
        )
    if ns.rpc_epoll_instances <= 0:
        raise ValueError(
            f"rpc-epoll-instances must be > 0, got {ns.rpc_epoll_instances}"
        )
    if ns.rpc_payload_bytes <= 0:
        raise ValueError(
            f"rpc-payload-bytes must be > 0, got {ns.rpc_payload_bytes}"
        )
    return ns


def read_trial_status_summary(
    *,
    run_root: Path,
    lane: str,
    trials: int,
) -> RuntimeStatusSummary:
    passed_trials = 0
    for trial_index in range(1, trials + 1):
        trial_dir = run_root / f"lane_{lane}" / f"trial_{trial_index:02d}"
        server_status = parse_any_int(
            (trial_dir / "rpc_server.status").read_text(encoding="utf-8").strip(),
            field=f"trial_{trial_index:02d} rpc_server.status",
        )
        client_status = parse_any_int(
            (trial_dir / "rpc_client.status").read_text(encoding="utf-8").strip(),
            field=f"trial_{trial_index:02d} rpc_client.status",
        )
        if server_status == 0 and client_status == 0:
            passed_trials += 1
    failed_trials = trials - passed_trials
    return RuntimeStatusSummary(
        requested_trials=trials,
        passed_trials=passed_trials,
        failed_trials=failed_trials,
        all_trials_passed=(failed_trials == 0),
    )


def main(argv: Sequence[str]) -> int:
    try:
        ns = parse_args(argv)
        workspace_root = ns.workspace_root.resolve()
        mako_root = ns.mako_root.resolve()
        run_root = ns.run_root.resolve()
        baseline_run_root = (
            ns.baseline_run_root.resolve() if ns.baseline_run_root is not None else None
        )
        run_root.mkdir(parents=True, exist_ok=True)

        if not workspace_root.exists():
            raise FileNotFoundError(f"workspace root does not exist: {workspace_root}")
        if not mako_root.exists():
            raise FileNotFoundError(f"mako root does not exist: {mako_root}")
        if not mako_root.joinpath("CMakeLists.txt").exists():
            raise FileNotFoundError(
                f"mako root is missing CMakeLists.txt: {mako_root / 'CMakeLists.txt'}"
            )

        base_env = dict(os.environ)
        strict_env = strict_runtime_replay_env(base_env)
        fragilec_path = ns.fragile_cxx.resolve()

        # Keep build profile aligned with default --fragile-cxx path
        # (target/release/fragilec) so replay never runs a stale binary.
        fragilec_build_cmd = [
            "cargo",
            "build",
            "--release",
            "-p",
            "fragile-cli",
            "--bin",
            "fragilec",
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
            "strict_runtime_replay_fragilec_build",
            fragilec_build_result,
        )
        if fragilec_build_result.status != 0:
            raise RuntimeError(
                "failed to build fragilec before runtime replay; "
                "see strict_runtime_replay_fragilec_build.stderr.log"
            )
        if not fragilec_path.exists():
            raise FileNotFoundError(
                f"fragilec binary not found after build: {fragilec_path}"
            )

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
            "--lanes",
            "fragilec",
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
        if ns.skip_masstree_perf_target:
            harness_cmd.append("--skip-masstree-perf-target")
        if ns.skip_clean_step:
            harness_cmd.append("--skip-clean-step")

        write_lines(
            run_root / "strict_runtime_replay_commands.txt",
            [
                "version=1",
                "task_leaf=M9.2",
                f"run_root={run_root}",
                "strict_env=FRAGILEC_MODE=strict FRAGILEC_PARSER_BACKEND=fragile-parser-clang",
                "strict_env_force_native_sources=unset",
                "strict_env_parser_core_codegen_escape_hatch=unset",
                f"fragilec_build_command={shell_join(fragilec_build_cmd)}",
                f"fragile_cxx={fragilec_path}",
                (
                    "skip_masstree_perf_target="
                    f"{'true' if ns.skip_masstree_perf_target else 'false'}"
                ),
                f"skip_clean_step={'true' if ns.skip_clean_step else 'false'}",
                (
                    "baseline_run_root="
                    f"{baseline_run_root if baseline_run_root is not None else 'none'}"
                ),
                f"harness_command={shell_join(harness_cmd)}",
            ],
        )

        harness_result = run_capture(harness_cmd, env=strict_env)
        write_command_result(run_root, "strict_runtime_replay_harness", harness_result)

        harness_manifest_path = run_root / "benchmark_harness_manifest.txt"
        comparison_manifest_path = run_root / "benchmark_qps_comparison_manifest.txt"
        if not harness_manifest_path.exists():
            raise FileNotFoundError(
                f"missing harness manifest artifact: {harness_manifest_path}"
            )
        if not comparison_manifest_path.exists():
            raise FileNotFoundError(
                "missing comparison manifest artifact: "
                f"{comparison_manifest_path}"
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

        lane_build_status = parse_any_int(
            required_key(harness_manifest, "lane_fragilec_build_status", source="harness"),
            field="lane_fragilec_build_status",
        )
        lane_test_rpc_status = parse_any_int(
            required_key(harness_manifest, "lane_fragilec_test_rpc_status", source="harness"),
            field="lane_fragilec_test_rpc_status",
        )
        lane_completed_trials = parse_any_int(
            required_key(
                harness_manifest,
                "lane_fragilec_completed_trials",
                source="harness",
            ),
            field="lane_fragilec_completed_trials",
        )
        lane_failure_class = required_key(
            harness_manifest,
            "lane_fragilec_failure_class",
            source="harness",
        )
        harness_skip_masstree_perf_target = required_key(
            harness_manifest,
            "skip_masstree_perf_target",
            source="harness",
        )
        harness_skip_clean_step = required_key(
            harness_manifest,
            "skip_clean_step",
            source="harness",
        )
        expected_skip_masstree = "true" if ns.skip_masstree_perf_target else "false"
        expected_skip_clean = "true" if ns.skip_clean_step else "false"
        if harness_skip_masstree_perf_target != expected_skip_masstree:
            raise ValueError(
                "harness skip_masstree_perf_target mismatch: "
                f"expected {expected_skip_masstree}, got {harness_skip_masstree_perf_target}"
            )
        if harness_skip_clean_step != expected_skip_clean:
            raise ValueError(
                "harness skip_clean_step mismatch: "
                f"expected {expected_skip_clean}, got {harness_skip_clean_step}"
            )
        harness_no_regression_verdict = required_key(
            harness_manifest,
            "no_regression_verdict",
            source="harness",
        )
        comparison_no_regression_verdict = required_key(
            comparison_manifest,
            "no_regression_verdict",
            source="comparison",
        )

        trial_summary = read_trial_status_summary(
            run_root=run_root,
            lane="fragilec",
            trials=ns.trials,
        )
        blocker_inventory = write_blocker_inventory_manifest(
            run_root=run_root,
            lane="fragilec",
            baseline_run_root=baseline_run_root,
        )

        lines = [
            "version=1",
            "task_leaf=M9.2",
            f"run_root={run_root}",
            "strict_mode=true",
            f"strict_env_mode={strict_env.get(STRICT_MODE_ENV, 'none')}",
            f"strict_env_parser_backend={strict_env.get(PARSER_BACKEND_ENV, 'none')}",
            "strict_env_force_native_sources=unset",
            "strict_env_parser_core_codegen_escape_hatch=unset",
            "lanes=fragilec",
            f"requested_trials={ns.trials}",
            f"harness_status={harness_result.status}",
            f"harness_manifest={harness_manifest_path}",
            f"comparison_manifest={comparison_manifest_path}",
            f"harness_no_regression_verdict={harness_no_regression_verdict}",
            f"comparison_no_regression_verdict={comparison_no_regression_verdict}",
            f"lane_fragilec_build_status={lane_build_status}",
            f"lane_fragilec_test_rpc_status={lane_test_rpc_status}",
            f"lane_fragilec_completed_trials={lane_completed_trials}",
            f"lane_fragilec_failure_class={lane_failure_class}",
            f"skip_masstree_perf_target={harness_skip_masstree_perf_target}",
            f"skip_clean_step={harness_skip_clean_step}",
            f"blocker_inventory_manifest={blocker_inventory.manifest_path}",
            f"blocker_error_total_count={blocker_inventory.total_count}",
            f"blocker_error_unique_count={blocker_inventory.unique_count}",
            f"blocker_first_error_key={blocker_inventory.first_error_key}",
            (
                "blocker_non_increase_total_vs_baseline="
                f"{blocker_inventory.non_increase_total_vs_baseline}"
            ),
            (
                "blocker_non_increase_unique_vs_baseline="
                f"{blocker_inventory.non_increase_unique_vs_baseline}"
            ),
            f"blocker_non_increase_verdict={blocker_inventory.non_increase_verdict}",
            (
                "runtime_all_trials_passed="
                f"{'true' if trial_summary.all_trials_passed else 'false'}"
            ),
            f"runtime_trial_passed_count={trial_summary.passed_trials}",
            f"runtime_trial_failed_count={trial_summary.failed_trials}",
            f"run_root_contract_version={RUN_ROOT_CONTRACT_VERSION}",
            f"run_root_name_pattern={RUN_ROOT_NAME_PATTERN}",
            (
                "run_root_name_is_contract_valid="
                f"{'true' if run_root_name_is_contract_valid(run_root.name) else 'false'}"
            ),
        ]
        manifest_path = run_root / "strict_runtime_replay_manifest.txt"
        write_lines(manifest_path, lines)

        artifact_contract_summary = write_artifact_contract_manifest(
            manifest_path=run_root / "strict_runtime_replay_required_artifacts_manifest.txt",
            task_leaf="M9.2",
            run_root=run_root,
            required_relpaths=required_artifacts_m9_2(trials=ns.trials),
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
                "strict runtime replay artifact contract has missing artifacts; "
                "see strict_runtime_replay_required_artifacts_manifest.txt"
            )

        lane_contract_pass = (
            lane_build_status == 0
            and lane_test_rpc_status == 0
            and lane_completed_trials == ns.trials
            and lane_failure_class == "none"
            and trial_summary.all_trials_passed
        )
        if not lane_contract_pass:
            raise RuntimeError(
                "strict runtime replay lane contract failed "
                f"(build={lane_build_status}, test_rpc={lane_test_rpc_status}, "
                f"completed_trials={lane_completed_trials}/{ns.trials}, "
                f"failure_class={lane_failure_class}, "
                f"runtime_all_trials_passed={trial_summary.all_trials_passed})"
            )

        # Runtime-only (single lane) replays commonly emit insufficient_data for
        # QPS comparison. Accept this case for M9.2 runtime evidence.
        if harness_result.status not in {0, 1}:
            raise RuntimeError(
                f"unexpected harness status {harness_result.status}; see strict_runtime_replay_harness.stderr.log"
            )
        if harness_result.status == 1 and harness_no_regression_verdict != "insufficient_data":
            raise RuntimeError(
                "harness exited non-zero without insufficient_data verdict; "
                f"observed no_regression_verdict={harness_no_regression_verdict}"
            )

        print(run_root)
        return 0
    except Exception as exc:  # pylint: disable=broad-except
        print(f"error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
