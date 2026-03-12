#!/usr/bin/env python3
"""Deterministic command-plan scaffolding for mako rpcbench benchmarking.

This leaf intentionally focuses on planning artifacts only (`--plan-only`), so
later leaves can plug in configure/build/run execution while keeping command
shape, trial naming, and artifact contracts stable.
"""

from __future__ import annotations

import argparse
import os
import shlex
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Sequence

LANES: tuple[str, str] = ("clang", "fragilec")


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
    build_type: str
    jobs: int
    trials: int
    base_port: int
    rpcbench: RpcBenchConfig


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
    parser.add_argument("--build-type", default="release")
    parser.add_argument("--jobs", type=int, default=max(os.cpu_count() or 1, 1))
    parser.add_argument("--trials", type=int, default=3)
    parser.add_argument("--base-port", type=int, default=18900)
    parser.add_argument("--rpc-duration-seconds", type=int, default=10)
    parser.add_argument("--rpc-client-threads", type=int, default=8)
    parser.add_argument("--rpc-outstanding", type=int, default=1000)
    parser.add_argument("--rpc-worker-threads", type=int, default=16)
    parser.add_argument("--rpc-epoll-instances", type=int, default=2)
    parser.add_argument("--rpc-payload-bytes", type=int, default=10)
    parser.add_argument(
        "--plan-only",
        action="store_true",
        help="Only emit deterministic command-plan/manifest artifacts (leaf 1.1 behavior)",
    )
    return parser.parse_args(list(argv))


def ensure_positive(name: str, value: int) -> None:
    if value <= 0:
        raise ValueError(f"{name} must be > 0, got {value}")


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
    ensure_positive("trials", ns.trials)
    ensure_positive("rpc-duration-seconds", ns.rpc_duration_seconds)
    ensure_positive("rpc-client-threads", ns.rpc_client_threads)
    ensure_positive("rpc-outstanding", ns.rpc_outstanding)
    ensure_positive("rpc-worker-threads", ns.rpc_worker_threads)
    ensure_positive("rpc-epoll-instances", ns.rpc_epoll_instances)
    ensure_positive("rpc-payload-bytes", ns.rpc_payload_bytes)

    if ns.base_port < 1024 or ns.base_port > 65535:
        raise ValueError(f"base-port must be within [1024, 65535], got {ns.base_port}")

    return HarnessConfig(
        workspace_root=ns.workspace_root.resolve(),
        mako_root=ns.mako_root.resolve(),
        run_root=ns.run_root.resolve(),
        clang_cxx=ns.clang_cxx,
        fragile_cxx=str(ns.fragile_cxx),
        c_compiler=ns.c_compiler,
        build_type=ns.build_type,
        jobs=ns.jobs,
        trials=ns.trials,
        base_port=ns.base_port,
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
        "cmake",
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
        "cmake",
        "--build",
        str(lane_build_dir(cfg.run_root, lane)),
        "--target",
        "clean",
    ]


def build_command(cfg: HarnessConfig, lane: str) -> list[str]:
    return [
        "cmake",
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
    ]

    for lane in LANES:
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
    lines.append("# benchmark harness command plan (leaf 1.1)")
    lines.append(f"workspace_root={cfg.workspace_root}")
    lines.append(f"mako_root={cfg.mako_root}")
    lines.append(f"run_root={cfg.run_root}")
    lines.append(f"trials={cfg.trials}")
    lines.append(f"jobs={cfg.jobs}")

    for lane in LANES:
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


def manifest_lines(cfg: HarnessConfig, plan_only: bool) -> list[str]:
    rpc = cfg.rpcbench
    lines = [
        "version=1",
        "task_leaf=1.1",
        f"workspace_root={cfg.workspace_root}",
        f"mako_root={cfg.mako_root}",
        f"run_root={cfg.run_root}",
        f"plan_only={str(plan_only).lower()}",
        f"lanes={','.join(LANES)}",
        f"trials={cfg.trials}",
        f"jobs={cfg.jobs}",
        f"build_type={cfg.build_type}",
        f"c_compiler={cfg.c_compiler}",
        f"clang_cxx={cfg.clang_cxx}",
        f"fragile_cxx={cfg.fragile_cxx}",
        f"rpc_duration_seconds={rpc.duration_seconds}",
        f"rpc_client_threads={rpc.client_threads}",
        f"rpc_outstanding_requests={rpc.outstanding_requests}",
        f"rpc_worker_threads={rpc.worker_threads}",
        f"rpc_epoll_instances={rpc.epoll_instances}",
        f"rpc_payload_bytes={rpc.payload_bytes}",
        "artifact_contract_file=benchmark_expected_artifacts.txt",
        "command_plan_file=benchmark_harness_command_plan.txt",
    ]

    for lane in LANES:
        lines.append(f"lane_{lane}_build_dir={lane_build_dir(cfg.run_root, lane)}")
        for trial in range(1, cfg.trials + 1):
            lines.append(
                f"lane_{lane}_trial_{trial:02d}_port={lane_trial_port(cfg.base_port, lane, trial)}"
            )
    return lines


def write_text_file(path: Path, lines: list[str]) -> None:
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def ensure_run_root_layout(cfg: HarnessConfig) -> None:
    cfg.run_root.mkdir(parents=True, exist_ok=True)
    for lane in LANES:
        lane_dir = cfg.run_root / f"lane_{lane}"
        lane_dir.mkdir(parents=True, exist_ok=True)
        for trial in range(1, cfg.trials + 1):
            lane_trial_dir(cfg.run_root, lane, trial).mkdir(parents=True, exist_ok=True)


def emit_plan_artifacts(cfg: HarnessConfig, plan_only: bool) -> None:
    ensure_run_root_layout(cfg)
    write_text_file(
        cfg.run_root / "benchmark_harness_manifest.txt", manifest_lines(cfg, plan_only)
    )
    write_text_file(
        cfg.run_root / "benchmark_harness_command_plan.txt", command_plan_lines(cfg)
    )
    write_text_file(
        cfg.run_root / "benchmark_expected_artifacts.txt", expected_artifacts(cfg)
    )


def main(argv: Sequence[str]) -> int:
    try:
        ns = parse_args(argv)
        cfg = to_harness_config(ns)
        validate_layout(cfg)
        emit_plan_artifacts(cfg, plan_only=bool(ns.plan_only))
        print(cfg.run_root)
        return 0
    except Exception as exc:  # pylint: disable=broad-except
        print(f"error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
