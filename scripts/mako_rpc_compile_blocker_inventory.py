#!/usr/bin/env python3
"""Deterministic compile-blocker inventory extraction for mako rpc harness runs.

Leaf 2.1 scope:
- read per-lane build artifacts emitted by `mako_rpcbench_harness.py`
- classify first failing compile blocker families deterministically
- persist lane inventory artifacts + root manifest for follow-up blocker-fix leaves
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import Sequence

DEFAULT_LANES: tuple[str, str] = ("clang", "fragilec")
SKIPPED_STATUS = -1

RUSTC_COMPILE_FAILURE_FILE_PATTERN = re.compile(
    r"\[fragilec\] fragile rustc object compile failed for (.+)"
)
TRANSPILE_FAILURE_FILE_PATTERN = re.compile(
    r"\[fragilec\] failed to transpile (.+?) with parser backend"
)
E0425_PATTERN = re.compile(r"error\[E0425\]")


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Extract deterministic compile blocker inventory from harness artifacts"
    )
    parser.add_argument("--run-root", type=Path, required=True)
    parser.add_argument(
        "--lanes",
        default=",".join(DEFAULT_LANES),
        help="comma-separated lane list; default: clang,fragilec",
    )
    return parser.parse_args(list(argv))


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def read_int(path: Path) -> int:
    value = read_text(path).strip()
    return int(value)


def write_text(path: Path, value: str) -> None:
    path.write_text(value + "\n", encoding="utf-8")


def first_failing_compile_file(build_stderr: str) -> str:
    match = RUSTC_COMPILE_FAILURE_FILE_PATTERN.search(build_stderr)
    if match is not None:
        return match.group(1).strip()
    match = TRANSPILE_FAILURE_FILE_PATTERN.search(build_stderr)
    if match is not None:
        return match.group(1).strip()
    return "none"


def classify_blocker(build_status: int, build_stderr: str) -> str:
    if build_status == 0:
        return "none"
    if build_status == SKIPPED_STATUS:
        return "build_not_executed"
    if "[fragilec] failed to transpile " in build_stderr:
        return "transpile_failure"
    if "error[E0425]" in build_stderr:
        return "unresolved_name_or_type_e0425"
    if "error[E0599]" in build_stderr:
        return "missing_method_e0599"
    if "error[E0061]" in build_stderr:
        return "arity_mismatch_e0061"
    if "error[E0308]" in build_stderr:
        return "type_mismatch_e0308"
    if "error[E" in build_stderr:
        return "other_rustc_error"
    return "other_build_failure"


def unresolved_name_count(build_stderr: str) -> int:
    return len(E0425_PATTERN.findall(build_stderr))


def lane_inventory_lines(
    lane: str,
    build_status: int,
    blocker_class: str,
    blocker_file: str,
    e0425_count: int,
) -> list[str]:
    return [
        f"lane_{lane}_build_status={build_status}",
        f"lane_{lane}_first_failing_compile_class={blocker_class}",
        f"lane_{lane}_first_failing_compile_file={blocker_file}",
        f"lane_{lane}_first_failing_compile_e0425_count={e0425_count}",
    ]


def run_inventory(run_root: Path, lanes: Sequence[str]) -> None:
    lines = [
        "version=1",
        "task_leaf=2.1",
        f"run_root={run_root}",
        f"lanes={','.join(lanes)}",
    ]

    for lane in lanes:
        lane_dir = run_root / f"lane_{lane}"
        build_status_path = lane_dir / "build.status"
        build_stderr_path = lane_dir / "build.stderr"
        if not build_status_path.exists():
            raise FileNotFoundError(f"missing build status artifact: {build_status_path}")
        if not build_stderr_path.exists():
            raise FileNotFoundError(f"missing build stderr artifact: {build_stderr_path}")

        build_status = read_int(build_status_path)
        build_stderr = read_text(build_stderr_path)
        blocker_class = classify_blocker(build_status, build_stderr)
        blocker_file = first_failing_compile_file(build_stderr)
        e0425_count = unresolved_name_count(build_stderr)

        if build_status in (0, SKIPPED_STATUS):
            blocker_file = "none"
            e0425_count = 0

        write_text(lane_dir / "first_failing_compile_class.txt", blocker_class)
        write_text(lane_dir / "first_failing_compile_file.txt", blocker_file)
        write_text(lane_dir / "first_failing_compile_e0425_count.txt", str(e0425_count))

        lines.extend(
            lane_inventory_lines(
                lane=lane,
                build_status=build_status,
                blocker_class=blocker_class,
                blocker_file=blocker_file,
                e0425_count=e0425_count,
            )
        )

    write_text(
        run_root / "rpc_compile_blocker_inventory_manifest.txt",
        "\n".join(lines),
    )


def parse_lanes(raw_lanes: str) -> list[str]:
    lanes = [lane.strip() for lane in raw_lanes.split(",") if lane.strip()]
    if not lanes:
        raise ValueError("lanes must include at least one lane name")
    return lanes


def main(argv: Sequence[str]) -> int:
    try:
        ns = parse_args(argv)
        run_root = ns.run_root.resolve()
        if not run_root.exists():
            raise FileNotFoundError(f"run root does not exist: {run_root}")
        run_inventory(run_root, parse_lanes(ns.lanes))
        print(run_root)
        return 0
    except Exception as exc:  # pylint: disable=broad-except
        print(f"error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
