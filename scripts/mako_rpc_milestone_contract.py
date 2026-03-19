#!/usr/bin/env python3
"""Shared milestone run-root naming and artifact contract helpers.

This module defines the naming contract and required artifact sets for:
- M0.1 strict baseline capture
- M0.2 parser backend A/B harness
- M9.2 strict runtime replay
- M9.3 benchmark comparison (clang vs fragile)
"""

from __future__ import annotations

import os
import re
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Iterable

RUN_ROOT_CONTRACT_VERSION = "1"
RUN_ROOT_NAME_PATTERN = (
    r"^fragile_(m0_1_strict_baseline|m0_2_parser_backend_ab|m9_2_strict_runtime_replay|m9_3_benchmark_comparison)_\d{8}T\d{6}Z_p\d+$"
)
_RUN_ROOT_RE = re.compile(RUN_ROOT_NAME_PATTERN)


@dataclass(frozen=True)
class ArtifactContractSummary:
    expected_count: int
    missing_count: int
    manifest_path: Path


def utc_timestamp_token(now: datetime | None = None) -> str:
    point = now if now is not None else datetime.now(timezone.utc)
    return point.astimezone(timezone.utc).strftime("%Y%m%dT%H%M%SZ")


def default_run_root_name(
    run_kind: str,
    *,
    now: datetime | None = None,
    pid: int | None = None,
) -> str:
    return f"fragile_{run_kind}_{utc_timestamp_token(now)}_p{pid if pid is not None else os.getpid()}"


def default_run_root_path(
    run_kind: str,
    *,
    base_dir: Path | None = None,
    now: datetime | None = None,
    pid: int | None = None,
) -> Path:
    root = (base_dir if base_dir is not None else Path("/tmp")).resolve()
    return root / default_run_root_name(run_kind, now=now, pid=pid)


def run_root_name_is_contract_valid(run_root_name: str) -> bool:
    return _RUN_ROOT_RE.fullmatch(run_root_name) is not None


def required_artifacts_m0_1() -> tuple[str, ...]:
    return (
        "strict_baseline_commands.txt",
        "strict_baseline_harness.status",
        "strict_baseline_harness.stdout.log",
        "strict_baseline_harness.stderr.log",
        "strict_baseline_inventory.status",
        "strict_baseline_inventory.stdout.log",
        "strict_baseline_inventory.stderr.log",
        "strict_baseline_replay.status",
        "strict_baseline_replay.stdout.log",
        "strict_baseline_replay.stderr.log",
        "benchmark_harness_manifest.txt",
        "rpc_compile_blocker_inventory_manifest.txt",
        "rpc_compile_blocker_replay_manifest.txt",
        "strict_baseline_manifest.txt",
    )


def required_artifacts_m0_2(
    *,
    baseline_backend: str,
    candidate_backend: str,
) -> tuple[str, ...]:
    return (
        "parser_backend_ab_commands.txt",
        "parser_backend_ab_baseline.status",
        "parser_backend_ab_baseline.stdout.log",
        "parser_backend_ab_baseline.stderr.log",
        "parser_backend_ab_candidate.status",
        "parser_backend_ab_candidate.stdout.log",
        "parser_backend_ab_candidate.stderr.log",
        f"baseline_{baseline_backend}/strict_baseline_manifest.txt",
        f"candidate_{candidate_backend}/strict_baseline_manifest.txt",
        "parser_backend_ab_baseline_comparable_manifest.txt",
        "parser_backend_ab_candidate_comparable_manifest.txt",
        "parser_backend_ab_manifest.txt",
    )


def required_artifacts_m9_2(
    *,
    trials: int,
    lane: str = "fragilec",
) -> tuple[str, ...]:
    if trials <= 0:
        raise ValueError(f"trials must be > 0, got {trials}")
    if not lane:
        raise ValueError("lane must be non-empty")

    entries: list[str] = [
        "strict_runtime_replay_commands.txt",
        "strict_runtime_replay_fragilec_build.status",
        "strict_runtime_replay_fragilec_build.stdout.log",
        "strict_runtime_replay_fragilec_build.stderr.log",
        "strict_runtime_replay_harness.status",
        "strict_runtime_replay_harness.stdout.log",
        "strict_runtime_replay_harness.stderr.log",
        "benchmark_harness_manifest.txt",
        "benchmark_harness_command_plan.txt",
        "benchmark_expected_artifacts.txt",
        "benchmark_qps_comparison_manifest.txt",
        f"lane_{lane}/configure.status",
        f"lane_{lane}/configure.stdout",
        f"lane_{lane}/configure.stderr",
        f"lane_{lane}/clean.status",
        f"lane_{lane}/clean.stdout",
        f"lane_{lane}/clean.stderr",
        f"lane_{lane}/build.status",
        f"lane_{lane}/build.stdout",
        f"lane_{lane}/build.stderr",
        f"lane_{lane}/test_rpc.status",
        f"lane_{lane}/test_rpc.stdout",
        f"lane_{lane}/test_rpc.stderr",
        "strict_runtime_replay_manifest.txt",
    ]

    for trial_index in range(1, trials + 1):
        trial_prefix = f"lane_{lane}/trial_{trial_index:02d}"
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

    return tuple(entries)


def required_artifacts_m9_3(
    *,
    trials: int,
    lanes: tuple[str, ...] = ("clang", "fragilec"),
) -> tuple[str, ...]:
    if trials <= 0:
        raise ValueError(f"trials must be > 0, got {trials}")
    if not lanes:
        raise ValueError("lanes must be non-empty")

    entries: list[str] = [
        "benchmark_comparison_commands.txt",
        "benchmark_comparison_fragilec_build.status",
        "benchmark_comparison_fragilec_build.stdout.log",
        "benchmark_comparison_fragilec_build.stderr.log",
        "benchmark_comparison_harness.status",
        "benchmark_comparison_harness.stdout.log",
        "benchmark_comparison_harness.stderr.log",
        "benchmark_harness_manifest.txt",
        "benchmark_harness_command_plan.txt",
        "benchmark_expected_artifacts.txt",
        "benchmark_qps_comparison_manifest.txt",
        "benchmark_comparison_manifest.txt",
    ]

    for lane in lanes:
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
        for trial_index in range(1, trials + 1):
            trial_prefix = f"{lane_prefix}/trial_{trial_index:02d}"
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

    return tuple(entries)


def write_artifact_contract_manifest(
    *,
    manifest_path: Path,
    task_leaf: str,
    run_root: Path,
    required_relpaths: Iterable[str],
) -> ArtifactContractSummary:
    run_root = run_root.resolve()
    relpaths = tuple(required_relpaths)
    lines = [
        "version=1",
        f"task_leaf={task_leaf}",
        f"run_root={run_root}",
        f"run_root_contract_version={RUN_ROOT_CONTRACT_VERSION}",
        f"run_root_name_pattern={RUN_ROOT_NAME_PATTERN}",
        (
            "run_root_name_is_contract_valid="
            f"{'true' if run_root_name_is_contract_valid(run_root.name) else 'false'}"
        ),
        f"required_artifact_count={len(relpaths)}",
    ]

    missing_count = 0
    for index, rel in enumerate(relpaths, start=1):
        path = run_root / rel
        exists = path.exists()
        if not exists:
            missing_count += 1
        lines.extend(
            [
                f"required_artifact_{index:03d}_relpath={rel}",
                f"required_artifact_{index:03d}_exists={'true' if exists else 'false'}",
                f"required_artifact_{index:03d}_abspath={path}",
            ]
        )
    lines.append(f"missing_required_artifact_count={missing_count}")
    manifest_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return ArtifactContractSummary(
        expected_count=len(relpaths),
        missing_count=missing_count,
        manifest_path=manifest_path,
    )
