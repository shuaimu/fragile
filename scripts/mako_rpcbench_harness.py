#!/usr/bin/env python3
"""Deterministic harness scaffolding for mako rpcbench benchmarking.

Leaf 1.1 added plan-only command/manifest scaffolding.
Leaf 1.2 added deterministic configure/clean/build execution capture.
Leaf 1.3 adds deterministic runtime replay for `test_rpc` and rpcbench trials.
Leaf 1.4 adds deterministic rpcbench QPS aggregation/comparison metadata.
"""

from __future__ import annotations

import argparse
import os
import re
import shlex
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Sequence, TextIO

SUPPORTED_LANES: tuple[str, str] = ("clang", "fragilec")
COMMAND_TIMEOUT_STATUS = 124
COMMAND_NOT_FOUND_STATUS = 127
SKIPPED_STATUS = -1
QPS_PATTERNS: tuple[re.Pattern[str], ...] = (
    re.compile(
        r"(?i)\bqps\b[^0-9+-]*([+-]?[0-9]+(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?)"
    ),
    re.compile(
        r"(?i)\b([+-]?[0-9]+(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?)\s*qps\b"
    ),
)


@dataclass(frozen=True)
class StepResult:
    status: int
    stdout: str
    stderr: str
    timed_out: bool


@dataclass(frozen=True)
class LaneExecutionSummary:
    configure_status: int
    clean_status: int
    build_status: int
    test_rpc_status: int
    completed_trials: int
    trial_qps_values: tuple[float | None, ...]
    avg_qps: float | None
    failure_class: str


@dataclass(frozen=True)
class RpcBenchConfig:
    duration_seconds: int
    client_threads: int
    outstanding_requests: int
    worker_threads: int
    epoll_instances: int
    payload_bytes: int


@dataclass(frozen=True)
class HarnessConfig:
    workspace_root: Path
    mako_root: Path
    run_root: Path
    clang_cxx: str
    fragile_cxx: str
    c_compiler: str
    cmake_bin: str
    build_type: str
    jobs: int
    configure_timeout_seconds: int
    clean_timeout_seconds: int
    build_timeout_seconds: int
    test_rpc_timeout_seconds: int
    rpc_client_timeout_seconds: int
    rpc_server_startup_wait_seconds: float
    rpc_server_shutdown_timeout_seconds: int
    trials: int
    base_port: int
    lanes: tuple[str, ...]
    build_only: bool
    rpcbench: RpcBenchConfig


@dataclass(frozen=True)
class ComparisonSummary:
    clang_avg_qps: float | None
    fragile_avg_qps: float | None
    fragile_minus_clang_qps: float | None
    fragile_over_clang_ratio: float | None
    no_regression_verdict: str


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    script_dir = Path(__file__).resolve().parent
    workspace_root = script_dir.parent
    default_mako_root = workspace_root / "vendor" / "mako"
    default_run_root = Path(
        f"/tmp/fragile_mako_rpcbench_harness_{os.getpid()}_{time.time_ns()}"
    )

    parser = argparse.ArgumentParser(
        description=(
            "Generate deterministic dual-lane rpcbench command plans and artifact manifests"
        )
    )
    parser.add_argument("--workspace-root", type=Path, default=workspace_root)
    parser.add_argument("--mako-root", type=Path, default=default_mako_root)
    parser.add_argument("--run-root", type=Path, default=default_run_root)
    parser.add_argument("--clang-cxx", default="clang++")
    parser.add_argument("--fragile-cxx", type=Path, default=workspace_root / "target" / "release" / "fragilec")
    parser.add_argument("--c-compiler", default="clang")
    parser.add_argument("--cmake-bin", default="cmake")
    parser.add_argument("--build-type", default="release")
    parser.add_argument("--jobs", type=int, default=max(os.cpu_count() or 1, 1))
    parser.add_argument("--configure-timeout-seconds", type=int, default=900)
    parser.add_argument("--clean-timeout-seconds", type=int, default=300)
    parser.add_argument("--build-timeout-seconds", type=int, default=3600)
    parser.add_argument("--test-rpc-timeout-seconds", type=int, default=120)
    parser.add_argument("--rpc-client-timeout-seconds", type=int, default=120)
    parser.add_argument("--rpc-server-startup-wait-seconds", type=float, default=1.0)
    parser.add_argument("--rpc-server-shutdown-timeout-seconds", type=int, default=10)
    parser.add_argument("--trials", type=int, default=3)
    parser.add_argument("--base-port", type=int, default=18900)
    parser.add_argument("--rpc-duration-seconds", type=int, default=10)
    parser.add_argument("--rpc-client-threads", type=int, default=8)
    parser.add_argument("--rpc-outstanding", type=int, default=1000)
    parser.add_argument("--rpc-worker-threads", type=int, default=16)
    parser.add_argument("--rpc-epoll-instances", type=int, default=2)
    parser.add_argument("--rpc-payload-bytes", type=int, default=10)
    parser.add_argument(
        "--lanes",
        default=",".join(SUPPORTED_LANES),
        help="comma-separated lane list; default: clang,fragilec",
    )
    parser.add_argument(
        "--build-only",
        action="store_true",
        help="run configure/clean/build only and skip test_rpc/rpcbench runtime steps",
    )
    parser.add_argument(
        "--plan-only",
        action="store_true",
        help="Only emit deterministic command-plan/manifest artifacts (leaf 1.1 behavior)",
    )
    return parser.parse_args(list(argv))


def ensure_positive(name: str, value: int) -> None:
    if value <= 0:
        raise ValueError(f"{name} must be > 0, got {value}")


def ensure_non_negative(name: str, value: float) -> None:
    if value < 0:
        raise ValueError(f"{name} must be >= 0, got {value}")


def parse_lanes(raw: str) -> tuple[str, ...]:
    lanes = tuple(lane.strip() for lane in raw.split(",") if lane.strip())
    if not lanes:
        raise ValueError("lanes must include at least one lane name")
    unknown = [lane for lane in lanes if lane not in SUPPORTED_LANES]
    if unknown:
        raise ValueError(
            f"unsupported lane(s): {','.join(unknown)}; supported: {','.join(SUPPORTED_LANES)}"
        )
    # Preserve the first occurrence ordering while deduplicating.
    ordered_unique: list[str] = []
    seen: set[str] = set()
    for lane in lanes:
        if lane in seen:
            continue
        seen.add(lane)
        ordered_unique.append(lane)
    return tuple(ordered_unique)


def format_qps(value: float | None) -> str:
    if value is None:
        return "none"
    return f"{value:.6f}"


def parse_qps_from_text(text: str) -> float | None:
    matches: list[float] = []
    for pattern in QPS_PATTERNS:
        for match in pattern.finditer(text):
            try:
                matches.append(float(match.group(1)))
            except ValueError:
                continue
    if not matches:
        return None
    return matches[-1]


def compute_average_qps(values: Sequence[float | None]) -> float | None:
    present = [value for value in values if value is not None]
    if not present:
        return None
    return sum(present) / len(present)


def shell_join(argv: Iterable[str]) -> str:
    return " ".join(shlex.quote(token) for token in argv)


def lane_build_dir(run_root: Path, lane: str) -> Path:
    return run_root / f"build_{lane}"


def lane_trial_dir(run_root: Path, lane: str, trial_index: int) -> Path:
    return run_root / f"lane_{lane}" / f"trial_{trial_index:02d}"


def lane_trial_port(base_port: int, lane: str, trial_index: int) -> int:
    # Stable deterministic mapping: clang starts at base, fragilec at base+100.
    lane_offset = 0 if lane == "clang" else 100
    return base_port + lane_offset + (trial_index - 1)


def to_harness_config(ns: argparse.Namespace) -> HarnessConfig:
    ensure_positive("jobs", ns.jobs)
    ensure_positive("configure-timeout-seconds", ns.configure_timeout_seconds)
    ensure_positive("clean-timeout-seconds", ns.clean_timeout_seconds)
    ensure_positive("build-timeout-seconds", ns.build_timeout_seconds)
    ensure_positive("test-rpc-timeout-seconds", ns.test_rpc_timeout_seconds)
    ensure_positive("rpc-client-timeout-seconds", ns.rpc_client_timeout_seconds)
    ensure_positive(
        "rpc-server-shutdown-timeout-seconds", ns.rpc_server_shutdown_timeout_seconds
    )
    ensure_non_negative(
        "rpc-server-startup-wait-seconds", ns.rpc_server_startup_wait_seconds
    )
    ensure_positive("trials", ns.trials)
    ensure_positive("rpc-duration-seconds", ns.rpc_duration_seconds)
    ensure_positive("rpc-client-threads", ns.rpc_client_threads)
    ensure_positive("rpc-outstanding", ns.rpc_outstanding)
    ensure_positive("rpc-worker-threads", ns.rpc_worker_threads)
    ensure_positive("rpc-epoll-instances", ns.rpc_epoll_instances)
    ensure_positive("rpc-payload-bytes", ns.rpc_payload_bytes)

    if ns.base_port < 1024 or ns.base_port > 65535:
        raise ValueError(f"base-port must be within [1024, 65535], got {ns.base_port}")
    lanes = parse_lanes(ns.lanes)

    return HarnessConfig(
        workspace_root=ns.workspace_root.resolve(),
        mako_root=ns.mako_root.resolve(),
        run_root=ns.run_root.resolve(),
        clang_cxx=ns.clang_cxx,
        fragile_cxx=str(ns.fragile_cxx),
        c_compiler=ns.c_compiler,
        cmake_bin=ns.cmake_bin,
        build_type=ns.build_type,
        jobs=ns.jobs,
        configure_timeout_seconds=ns.configure_timeout_seconds,
        clean_timeout_seconds=ns.clean_timeout_seconds,
        build_timeout_seconds=ns.build_timeout_seconds,
        test_rpc_timeout_seconds=ns.test_rpc_timeout_seconds,
        rpc_client_timeout_seconds=ns.rpc_client_timeout_seconds,
        rpc_server_startup_wait_seconds=ns.rpc_server_startup_wait_seconds,
        rpc_server_shutdown_timeout_seconds=ns.rpc_server_shutdown_timeout_seconds,
        trials=ns.trials,
        base_port=ns.base_port,
        lanes=lanes,
        build_only=bool(ns.build_only),
        rpcbench=RpcBenchConfig(
            duration_seconds=ns.rpc_duration_seconds,
            client_threads=ns.rpc_client_threads,
            outstanding_requests=ns.rpc_outstanding,
            worker_threads=ns.rpc_worker_threads,
            epoll_instances=ns.rpc_epoll_instances,
            payload_bytes=ns.rpc_payload_bytes,
        ),
    )


def validate_layout(cfg: HarnessConfig) -> None:
    if not cfg.workspace_root.exists():
        raise FileNotFoundError(f"workspace root does not exist: {cfg.workspace_root}")
    if not cfg.mako_root.exists():
        raise FileNotFoundError(f"mako root does not exist: {cfg.mako_root}")
    if not cfg.mako_root.joinpath("CMakeLists.txt").exists():
        raise FileNotFoundError(
            f"mako root is missing CMakeLists.txt: {cfg.mako_root / 'CMakeLists.txt'}"
        )


def lane_cxx_compiler(cfg: HarnessConfig, lane: str) -> str:
    return cfg.clang_cxx if lane == "clang" else cfg.fragile_cxx


def configure_command(cfg: HarnessConfig, lane: str) -> list[str]:
    return [
        cfg.cmake_bin,
        "-S",
        str(cfg.mako_root),
        "-B",
        str(lane_build_dir(cfg.run_root, lane)),
        "-DCMAKE_BUILD_TYPE=" + cfg.build_type,
        "-DENABLE_TESTS=OFF",
        "-DCMAKE_C_COMPILER=" + cfg.c_compiler,
        "-DCMAKE_CXX_COMPILER=" + lane_cxx_compiler(cfg, lane),
    ]


def clean_command(cfg: HarnessConfig, lane: str) -> list[str]:
    return [
        cfg.cmake_bin,
        "--build",
        str(lane_build_dir(cfg.run_root, lane)),
        "--target",
        "clean",
    ]


def build_command(cfg: HarnessConfig, lane: str) -> list[str]:
    return [
        cfg.cmake_bin,
        "--build",
        str(lane_build_dir(cfg.run_root, lane)),
        "-j",
        str(cfg.jobs),
        "--target",
        "test_rpc",
        "rpcbench",
        "masstree_perf",
    ]


def test_rpc_command(cfg: HarnessConfig, lane: str) -> list[str]:
    return [str(lane_build_dir(cfg.run_root, lane) / "test_rpc")]


def rpc_server_command(cfg: HarnessConfig, lane: str, trial_index: int) -> list[str]:
    port = lane_trial_port(cfg.base_port, lane, trial_index)
    rpc = cfg.rpcbench
    return [
        str(lane_build_dir(cfg.run_root, lane) / "rpcbench"),
        "-s",
        f"127.0.0.1:{port}",
        "-w",
        str(rpc.worker_threads),
        "-e",
        str(rpc.epoll_instances),
        "-b",
        str(rpc.payload_bytes),
    ]


def rpc_client_command(cfg: HarnessConfig, lane: str, trial_index: int) -> list[str]:
    port = lane_trial_port(cfg.base_port, lane, trial_index)
    rpc = cfg.rpcbench
    return [
        str(lane_build_dir(cfg.run_root, lane) / "rpcbench"),
        "-c",
        f"127.0.0.1:{port}",
        "-n",
        str(rpc.duration_seconds),
        "-t",
        str(rpc.client_threads),
        "-o",
        str(rpc.outstanding_requests),
        "-w",
        str(rpc.worker_threads),
        "-e",
        str(rpc.epoll_instances),
        "-b",
        str(rpc.payload_bytes),
    ]


def expected_artifacts(cfg: HarnessConfig) -> list[str]:
    entries: list[str] = [
        "benchmark_harness_manifest.txt",
        "benchmark_harness_command_plan.txt",
        "benchmark_expected_artifacts.txt",
        "benchmark_qps_comparison_manifest.txt",
    ]

    for lane in cfg.lanes:
        lane_prefix = f"lane_{lane}"
        entries.extend(
            [
                f"{lane_prefix}/configure.status",
                f"{lane_prefix}/configure.stdout",
                f"{lane_prefix}/configure.stderr",
                f"{lane_prefix}/clean.status",
                f"{lane_prefix}/clean.stdout",
                f"{lane_prefix}/clean.stderr",
                f"{lane_prefix}/build.status",
                f"{lane_prefix}/build.stdout",
                f"{lane_prefix}/build.stderr",
                f"{lane_prefix}/failure_class.txt",
                f"{lane_prefix}/test_rpc.status",
                f"{lane_prefix}/test_rpc.stdout",
                f"{lane_prefix}/test_rpc.stderr",
            ]
        )
        for trial in range(1, cfg.trials + 1):
            trial_prefix = f"{lane_prefix}/trial_{trial:02d}"
            entries.extend(
                [
                    f"{trial_prefix}/rpc_server.status",
                    f"{trial_prefix}/rpc_server.stdout",
                    f"{trial_prefix}/rpc_server.stderr",
                    f"{trial_prefix}/rpc_client.status",
                    f"{trial_prefix}/rpc_client.stdout",
                    f"{trial_prefix}/rpc_client.stderr",
                ]
            )
    return sorted(entries)


def command_plan_lines(cfg: HarnessConfig) -> list[str]:
    lines: list[str] = []
    lines.append("# benchmark harness command plan (leaf 1.4)")
    lines.append(f"workspace_root={cfg.workspace_root}")
    lines.append(f"mako_root={cfg.mako_root}")
    lines.append(f"run_root={cfg.run_root}")
    lines.append(f"trials={cfg.trials}")
    lines.append(f"jobs={cfg.jobs}")

    for lane in cfg.lanes:
        lines.append("")
        lines.append(f"[lane:{lane}]")
        lines.append(f"configure={shell_join(configure_command(cfg, lane))}")
        lines.append(f"clean={shell_join(clean_command(cfg, lane))}")
        lines.append(f"build={shell_join(build_command(cfg, lane))}")
        lines.append(f"test_rpc={shell_join(test_rpc_command(cfg, lane))}")
        for trial in range(1, cfg.trials + 1):
            port = lane_trial_port(cfg.base_port, lane, trial)
            lines.append(f"trial_{trial:02d}_port={port}")
            lines.append(
                f"trial_{trial:02d}_server={shell_join(rpc_server_command(cfg, lane, trial))}"
            )
            lines.append(
                f"trial_{trial:02d}_client={shell_join(rpc_client_command(cfg, lane, trial))}"
            )

    return lines


def lane_trial_qps_values(
    cfg: HarnessConfig,
    lane: str,
    lane_summaries: dict[str, LaneExecutionSummary] | None,
) -> list[float | None]:
    if lane_summaries is None or lane not in lane_summaries:
        return [None] * cfg.trials
    values = list(lane_summaries[lane].trial_qps_values)
    if len(values) < cfg.trials:
        values.extend([None] * (cfg.trials - len(values)))
    return values[: cfg.trials]


def compute_comparison_summary(
    lane_summaries: dict[str, LaneExecutionSummary] | None,
    *,
    build_only: bool = False,
) -> ComparisonSummary:
    if build_only:
        return ComparisonSummary(
            clang_avg_qps=None,
            fragile_avg_qps=None,
            fragile_minus_clang_qps=None,
            fragile_over_clang_ratio=None,
            no_regression_verdict="not_executed",
        )
    if lane_summaries is None:
        return ComparisonSummary(
            clang_avg_qps=None,
            fragile_avg_qps=None,
            fragile_minus_clang_qps=None,
            fragile_over_clang_ratio=None,
            no_regression_verdict="not_executed",
        )

    clang_summary = lane_summaries.get("clang")
    fragile_summary = lane_summaries.get("fragilec")
    clang_avg_qps = None if clang_summary is None else clang_summary.avg_qps
    fragile_avg_qps = None if fragile_summary is None else fragile_summary.avg_qps

    if clang_avg_qps is None or fragile_avg_qps is None:
        return ComparisonSummary(
            clang_avg_qps=clang_avg_qps,
            fragile_avg_qps=fragile_avg_qps,
            fragile_minus_clang_qps=None,
            fragile_over_clang_ratio=None,
            no_regression_verdict="insufficient_data",
        )

    delta = fragile_avg_qps - clang_avg_qps
    ratio = None if clang_avg_qps == 0 else fragile_avg_qps / clang_avg_qps
    verdict = "pass" if fragile_avg_qps >= clang_avg_qps else "fail"
    return ComparisonSummary(
        clang_avg_qps=clang_avg_qps,
        fragile_avg_qps=fragile_avg_qps,
        fragile_minus_clang_qps=delta,
        fragile_over_clang_ratio=ratio,
        no_regression_verdict=verdict,
    )


def comparison_manifest_lines(
    cfg: HarnessConfig,
    plan_only: bool,
    lane_summaries: dict[str, LaneExecutionSummary] | None,
    comparison_summary: ComparisonSummary,
) -> list[str]:
    task_leaf = "1.1" if plan_only else "1.4"
    lines = [
        "version=1",
        f"task_leaf={task_leaf}",
        f"run_root={cfg.run_root}",
        f"plan_only={str(plan_only).lower()}",
        f"trials={cfg.trials}",
        f"clang_avg_qps={format_qps(comparison_summary.clang_avg_qps)}",
        f"fragile_avg_qps={format_qps(comparison_summary.fragile_avg_qps)}",
        f"fragile_minus_clang_qps={format_qps(comparison_summary.fragile_minus_clang_qps)}",
        f"fragile_over_clang_ratio={format_qps(comparison_summary.fragile_over_clang_ratio)}",
        f"no_regression_verdict={comparison_summary.no_regression_verdict}",
    ]
    for lane in cfg.lanes:
        trial_values = lane_trial_qps_values(cfg, lane, lane_summaries)
        for trial in range(1, cfg.trials + 1):
            lines.append(
                f"lane_{lane}_trial_{trial:02d}_qps={format_qps(trial_values[trial - 1])}"
            )
    return lines


def manifest_lines(
    cfg: HarnessConfig,
    plan_only: bool,
    lane_summaries: dict[str, LaneExecutionSummary] | None = None,
) -> list[str]:
    rpc = cfg.rpcbench
    task_leaf = "1.1" if plan_only else "1.4"
    comparison_summary = compute_comparison_summary(
        lane_summaries, build_only=cfg.build_only
    )
    lines = [
        "version=1",
        f"task_leaf={task_leaf}",
        f"workspace_root={cfg.workspace_root}",
        f"mako_root={cfg.mako_root}",
        f"run_root={cfg.run_root}",
        f"plan_only={str(plan_only).lower()}",
        f"lanes={','.join(cfg.lanes)}",
        f"build_only={str(cfg.build_only).lower()}",
        f"trials={cfg.trials}",
        f"jobs={cfg.jobs}",
        f"build_type={cfg.build_type}",
        f"c_compiler={cfg.c_compiler}",
        f"cmake_bin={cfg.cmake_bin}",
        f"clang_cxx={cfg.clang_cxx}",
        f"fragile_cxx={cfg.fragile_cxx}",
        f"configure_timeout_seconds={cfg.configure_timeout_seconds}",
        f"clean_timeout_seconds={cfg.clean_timeout_seconds}",
        f"build_timeout_seconds={cfg.build_timeout_seconds}",
        f"test_rpc_timeout_seconds={cfg.test_rpc_timeout_seconds}",
        f"rpc_client_timeout_seconds={cfg.rpc_client_timeout_seconds}",
        f"rpc_server_startup_wait_seconds={cfg.rpc_server_startup_wait_seconds}",
        f"rpc_server_shutdown_timeout_seconds={cfg.rpc_server_shutdown_timeout_seconds}",
        f"rpc_duration_seconds={rpc.duration_seconds}",
        f"rpc_client_threads={rpc.client_threads}",
        f"rpc_outstanding_requests={rpc.outstanding_requests}",
        f"rpc_worker_threads={rpc.worker_threads}",
        f"rpc_epoll_instances={rpc.epoll_instances}",
        f"rpc_payload_bytes={rpc.payload_bytes}",
        "artifact_contract_file=benchmark_expected_artifacts.txt",
        "command_plan_file=benchmark_harness_command_plan.txt",
        "comparison_manifest_file=benchmark_qps_comparison_manifest.txt",
        f"clang_avg_qps={format_qps(comparison_summary.clang_avg_qps)}",
        f"fragile_avg_qps={format_qps(comparison_summary.fragile_avg_qps)}",
        f"fragile_minus_clang_qps={format_qps(comparison_summary.fragile_minus_clang_qps)}",
        f"fragile_over_clang_ratio={format_qps(comparison_summary.fragile_over_clang_ratio)}",
        f"no_regression_verdict={comparison_summary.no_regression_verdict}",
    ]

    for lane in cfg.lanes:
        lines.append(f"lane_{lane}_build_dir={lane_build_dir(cfg.run_root, lane)}")
        trial_qps_values = lane_trial_qps_values(cfg, lane, lane_summaries)
        for trial in range(1, cfg.trials + 1):
            lines.append(
                f"lane_{lane}_trial_{trial:02d}_port={lane_trial_port(cfg.base_port, lane, trial)}"
            )
            lines.append(
                f"lane_{lane}_trial_{trial:02d}_qps={format_qps(trial_qps_values[trial - 1])}"
            )
        if lane_summaries is not None and lane in lane_summaries:
            summary = lane_summaries[lane]
            lines.append(f"lane_{lane}_configure_status={summary.configure_status}")
            lines.append(f"lane_{lane}_clean_status={summary.clean_status}")
            lines.append(f"lane_{lane}_build_status={summary.build_status}")
            lines.append(f"lane_{lane}_test_rpc_status={summary.test_rpc_status}")
            lines.append(f"lane_{lane}_completed_trials={summary.completed_trials}")
            lines.append(f"lane_{lane}_avg_qps={format_qps(summary.avg_qps)}")
            lines.append(f"lane_{lane}_failure_class={summary.failure_class}")
    return lines


def write_text_file(path: Path, lines: list[str]) -> None:
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def ensure_run_root_layout(cfg: HarnessConfig) -> None:
    cfg.run_root.mkdir(parents=True, exist_ok=True)
    for lane in cfg.lanes:
        lane_dir = cfg.run_root / f"lane_{lane}"
        lane_dir.mkdir(parents=True, exist_ok=True)
        for trial in range(1, cfg.trials + 1):
            lane_trial_dir(cfg.run_root, lane, trial).mkdir(parents=True, exist_ok=True)


def emit_plan_artifacts(
    cfg: HarnessConfig,
    plan_only: bool,
    lane_summaries: dict[str, LaneExecutionSummary] | None = None,
    comparison_summary: ComparisonSummary | None = None,
) -> None:
    if comparison_summary is None:
        comparison_summary = compute_comparison_summary(lane_summaries)
    ensure_run_root_layout(cfg)
    write_text_file(
        cfg.run_root / "benchmark_harness_manifest.txt",
        manifest_lines(cfg, plan_only, lane_summaries),
    )
    write_text_file(
        cfg.run_root / "benchmark_harness_command_plan.txt", command_plan_lines(cfg)
    )
    write_text_file(
        cfg.run_root / "benchmark_expected_artifacts.txt", expected_artifacts(cfg)
    )
    write_text_file(
        cfg.run_root / "benchmark_qps_comparison_manifest.txt",
        comparison_manifest_lines(cfg, plan_only, lane_summaries, comparison_summary),
    )


def _ensure_text(value: str | bytes | None) -> str:
    if value is None:
        return ""
    if isinstance(value, bytes):
        return value.decode("utf-8", errors="replace")
    return value


def run_command_capture(argv: list[str], timeout_seconds: int) -> StepResult:
    try:
        output = subprocess.run(
            argv,
            check=False,
            capture_output=True,
            text=True,
            timeout=timeout_seconds,
        )
        return StepResult(
            status=output.returncode,
            stdout=output.stdout,
            stderr=output.stderr,
            timed_out=False,
        )
    except subprocess.TimeoutExpired as exc:
        timeout_msg = (
            f"error: command timed out after {timeout_seconds} seconds: {shell_join(argv)}\n"
        )
        return StepResult(
            status=COMMAND_TIMEOUT_STATUS,
            stdout=_ensure_text(exc.stdout),
            stderr=_ensure_text(exc.stderr) + timeout_msg,
            timed_out=True,
        )
    except OSError as exc:
        return StepResult(
            status=COMMAND_NOT_FOUND_STATUS,
            stdout="",
            stderr=f"error: failed to run command: {shell_join(argv)} ({exc})\n",
            timed_out=False,
        )


def skipped_step_result(reason: str) -> StepResult:
    return StepResult(
        status=SKIPPED_STATUS,
        stdout="",
        stderr=f"skipped: {reason}\n",
        timed_out=False,
    )


def write_step_result(lane_dir: Path, step_name: str, result: StepResult) -> None:
    write_text_file(lane_dir / f"{step_name}.status", [str(result.status)])
    write_text_file(lane_dir / f"{step_name}.stdout", result.stdout.splitlines())
    write_text_file(lane_dir / f"{step_name}.stderr", result.stderr.splitlines())


def write_runtime_skipped_results(
    cfg: HarnessConfig,
    lane: str,
    reason: str,
    *,
    include_test_rpc: bool,
) -> None:
    lane_dir = cfg.run_root / f"lane_{lane}"
    if include_test_rpc:
        test_rpc_result = skipped_step_result(reason)
        write_step_result(lane_dir, "test_rpc", test_rpc_result)
    for trial in range(1, cfg.trials + 1):
        trial_dir = lane_trial_dir(cfg.run_root, lane, trial)
        write_step_result(trial_dir, "rpc_server", skipped_step_result(reason))
        write_step_result(trial_dir, "rpc_client", skipped_step_result(reason))


def run_background_process_to_files(
    argv: list[str],
    stdout_path: Path,
    stderr_path: Path,
) -> tuple[subprocess.Popen[str], TextIO, TextIO]:
    stdout_handle = stdout_path.open("w", encoding="utf-8")
    stderr_handle = stderr_path.open("w", encoding="utf-8")
    try:
        process = subprocess.Popen(
            argv,
            stdout=stdout_handle,
            stderr=stderr_handle,
            text=True,
        )
    except OSError:
        stdout_handle.close()
        stderr_handle.close()
        raise
    return process, stdout_handle, stderr_handle


def read_text_file(path: Path) -> str:
    if not path.exists():
        return ""
    return path.read_text(encoding="utf-8", errors="replace")


def finalize_server_process(
    process: subprocess.Popen[str],
    stdout_handle: TextIO,
    stderr_handle: TextIO,
    stdout_path: Path,
    stderr_path: Path,
    shutdown_timeout_seconds: int,
) -> StepResult:
    terminated_by_harness = False
    timed_out = False
    timeout_error = ""

    if process.poll() is None:
        terminated_by_harness = True
        process.terminate()
        try:
            process.wait(timeout=shutdown_timeout_seconds)
        except subprocess.TimeoutExpired:
            timed_out = True
            timeout_error = (
                "error: rpc server did not exit after terminate within "
                f"{shutdown_timeout_seconds} seconds\n"
            )
            process.kill()
            process.wait()

    stdout_handle.close()
    stderr_handle.close()
    stdout = read_text_file(stdout_path)
    stderr = read_text_file(stderr_path) + timeout_error

    if timed_out:
        return StepResult(
            status=COMMAND_TIMEOUT_STATUS,
            stdout=stdout,
            stderr=stderr,
            timed_out=True,
        )

    returncode = process.returncode if process.returncode is not None else 0
    if terminated_by_harness and returncode < 0:
        returncode = 0
    return StepResult(
        status=returncode,
        stdout=stdout,
        stderr=stderr,
        timed_out=False,
    )


def run_rpc_trial(
    cfg: HarnessConfig,
    lane: str,
    trial_index: int,
) -> tuple[StepResult, StepResult, str]:
    trial_dir = lane_trial_dir(cfg.run_root, lane, trial_index)
    server_stdout_path = trial_dir / "rpc_server.live.stdout"
    server_stderr_path = trial_dir / "rpc_server.live.stderr"

    try:
        server_proc, server_stdout_handle, server_stderr_handle = run_background_process_to_files(
            rpc_server_command(cfg, lane, trial_index),
            server_stdout_path,
            server_stderr_path,
        )
    except OSError as exc:
        server_result = StepResult(
            status=COMMAND_NOT_FOUND_STATUS,
            stdout="",
            stderr=(
                "error: failed to start rpc server command: "
                f"{shell_join(rpc_server_command(cfg, lane, trial_index))} ({exc})\n"
            ),
            timed_out=False,
        )
        client_result = skipped_step_result("rpc server failed to start")
        server_stdout_path.unlink(missing_ok=True)
        server_stderr_path.unlink(missing_ok=True)
        return server_result, client_result, "rpc_server_failed"

    if cfg.rpc_server_startup_wait_seconds > 0:
        time.sleep(cfg.rpc_server_startup_wait_seconds)

    if server_proc.poll() is not None:
        server_stdout_handle.close()
        server_stderr_handle.close()
        server_result = StepResult(
            status=server_proc.returncode if server_proc.returncode is not None else 1,
            stdout=read_text_file(server_stdout_path),
            stderr=read_text_file(server_stderr_path),
            timed_out=False,
        )
        client_result = skipped_step_result("rpc server exited before client start")
        server_stdout_path.unlink(missing_ok=True)
        server_stderr_path.unlink(missing_ok=True)
        return server_result, client_result, "rpc_server_failed"

    client_result = run_command_capture(
        rpc_client_command(cfg, lane, trial_index),
        cfg.rpc_client_timeout_seconds,
    )
    server_result = finalize_server_process(
        server_proc,
        server_stdout_handle,
        server_stderr_handle,
        server_stdout_path,
        server_stderr_path,
        cfg.rpc_server_shutdown_timeout_seconds,
    )
    server_stdout_path.unlink(missing_ok=True)
    server_stderr_path.unlink(missing_ok=True)

    if server_result.timed_out:
        return server_result, client_result, "rpc_server_timeout"
    if server_result.status != 0:
        return server_result, client_result, "rpc_server_failed"
    if client_result.timed_out:
        return server_result, client_result, "rpc_client_timeout"
    if client_result.status != 0:
        return server_result, client_result, "rpc_client_failed"
    return server_result, client_result, "none"


def extract_trial_qps(client_result: StepResult, trial_failure: str) -> float | None:
    if trial_failure != "none":
        return None
    return parse_qps_from_text(client_result.stdout + "\n" + client_result.stderr)


def classify_lane_failure(
    configure_result: StepResult,
    clean_result: StepResult,
    build_result: StepResult,
    test_rpc_result: StepResult,
    runtime_failure_class: str,
    *,
    build_only: bool,
) -> str:
    if configure_result.timed_out:
        return "configure_timeout"
    if configure_result.status != 0:
        return "configure_failed"
    if clean_result.timed_out:
        return "clean_timeout"
    if clean_result.status != 0:
        return "clean_failed"
    if build_result.timed_out:
        return "build_timeout"
    if build_result.status != 0:
        return "build_failed"
    if build_only:
        return "none"
    if test_rpc_result.timed_out:
        return "test_rpc_timeout"
    if test_rpc_result.status != 0:
        return "test_rpc_failed"
    if runtime_failure_class != "none":
        return runtime_failure_class
    return "none"


def execute_harness_capture(
    cfg: HarnessConfig,
) -> dict[str, LaneExecutionSummary]:
    summaries: dict[str, LaneExecutionSummary] = {}

    for lane in cfg.lanes:
        lane_dir = cfg.run_root / f"lane_{lane}"
        lane_dir.mkdir(parents=True, exist_ok=True)

        configure_result = run_command_capture(
            configure_command(cfg, lane), cfg.configure_timeout_seconds
        )
        write_step_result(lane_dir, "configure", configure_result)

        if configure_result.status != 0:
            clean_result = skipped_step_result("configure step failed")
            build_result = skipped_step_result("configure step failed")
        else:
            clean_result = run_command_capture(clean_command(cfg, lane), cfg.clean_timeout_seconds)
            if clean_result.status != 0:
                build_result = skipped_step_result("clean step failed")
            else:
                build_result = run_command_capture(
                    build_command(cfg, lane), cfg.build_timeout_seconds
                )

        write_step_result(lane_dir, "clean", clean_result)
        write_step_result(lane_dir, "build", build_result)

        test_rpc_result = skipped_step_result("build step failed")
        completed_trials = 0
        trial_qps_values: list[float | None] = [None] * cfg.trials
        runtime_failure_class = "none"

        if cfg.build_only:
            skip_reason = (
                "build-only mode" if build_result.status == 0 else "build step failed"
            )
            test_rpc_result = skipped_step_result(skip_reason)
            write_runtime_skipped_results(
                cfg, lane, skip_reason, include_test_rpc=True
            )
        elif build_result.status == 0:
            test_rpc_result = run_command_capture(
                test_rpc_command(cfg, lane), cfg.test_rpc_timeout_seconds
            )
            write_step_result(lane_dir, "test_rpc", test_rpc_result)

            if test_rpc_result.status == 0:
                for trial in range(1, cfg.trials + 1):
                    trial_dir = lane_trial_dir(cfg.run_root, lane, trial)
                    server_result, client_result, trial_failure = run_rpc_trial(
                        cfg, lane, trial
                    )
                    write_step_result(trial_dir, "rpc_server", server_result)
                    write_step_result(trial_dir, "rpc_client", client_result)
                    if trial_failure == "none":
                        completed_trials += 1
                        trial_qps_values[trial - 1] = extract_trial_qps(
                            client_result, trial_failure
                        )
                    elif runtime_failure_class == "none":
                        runtime_failure_class = f"rpc_trial_{trial:02d}_{trial_failure}"
            else:
                write_runtime_skipped_results(
                    cfg, lane, "test_rpc step failed", include_test_rpc=False
                )
        else:
            write_runtime_skipped_results(
                cfg, lane, "build step failed", include_test_rpc=True
            )

        if cfg.build_only and build_result.status == 0:
            test_rpc_result = skipped_step_result("build-only mode")
        elif build_result.status != 0:
            # `write_runtime_skipped_results` already wrote this artifact.
            test_rpc_result = skipped_step_result("build step failed")
        elif test_rpc_result.status != 0:
            # Ensure all trial artifacts exist with skipped markers when test_rpc failed.
            for trial in range(1, cfg.trials + 1):
                trial_dir = lane_trial_dir(cfg.run_root, lane, trial)
                if not (trial_dir / "rpc_server.status").exists():
                    write_step_result(
                        trial_dir, "rpc_server", skipped_step_result("test_rpc step failed")
                    )
                    write_step_result(
                        trial_dir, "rpc_client", skipped_step_result("test_rpc step failed")
                    )

        failure_class = classify_lane_failure(
            configure_result,
            clean_result,
            build_result,
            test_rpc_result,
            runtime_failure_class,
            build_only=cfg.build_only,
        )
        write_text_file(lane_dir / "failure_class.txt", [failure_class])
        avg_qps = compute_average_qps(trial_qps_values)
        summaries[lane] = LaneExecutionSummary(
            configure_status=configure_result.status,
            clean_status=clean_result.status,
            build_status=build_result.status,
            test_rpc_status=test_rpc_result.status,
            completed_trials=completed_trials,
            trial_qps_values=tuple(trial_qps_values),
            avg_qps=avg_qps,
            failure_class=failure_class,
        )

    return summaries


def has_lane_failures(lane_summaries: dict[str, LaneExecutionSummary]) -> bool:
    return any(summary.failure_class != "none" for summary in lane_summaries.values())


def comparison_requires_failure(comparison_summary: ComparisonSummary) -> bool:
    return comparison_summary.no_regression_verdict in {"fail", "insufficient_data"}


def main(argv: Sequence[str]) -> int:
    try:
        ns = parse_args(argv)
        cfg = to_harness_config(ns)
        validate_layout(cfg)
        lane_summaries: dict[str, LaneExecutionSummary] | None = None
        comparison_summary = compute_comparison_summary(
            None, build_only=cfg.build_only
        )
        ensure_run_root_layout(cfg)
        if not ns.plan_only:
            lane_summaries = execute_harness_capture(cfg)
            comparison_summary = compute_comparison_summary(
                lane_summaries, build_only=cfg.build_only
            )
        emit_plan_artifacts(
            cfg,
            plan_only=bool(ns.plan_only),
            lane_summaries=lane_summaries,
            comparison_summary=comparison_summary,
        )
        print(cfg.run_root)
        if lane_summaries is not None:
            if has_lane_failures(lane_summaries):
                return 1
            if comparison_requires_failure(comparison_summary):
                return 1
        return 0
    except Exception as exc:  # pylint: disable=broad-except
        print(f"error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
