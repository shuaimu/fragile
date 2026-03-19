#!/usr/bin/env python3
"""Shadow-mode strict parser backend replay for TODO leaf M7.1.

This harness runs a representative non-RPC source corpus through strict `fragilec`
compile twice under one run root:
- baseline parser backend (default: libtooling)
- candidate parser backend (default: fragile-parser-clang)

It emits deterministic per-fixture logs, a summary manifest, and an explicit RPC
corpus queue artifact for deferred M9 closure.
"""

from __future__ import annotations

import argparse
import os
import shlex
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Mapping, Sequence

COMMAND_TIMEOUT_STATUS = 124
DEFAULT_BASELINE_BACKEND = "libtooling"
DEFAULT_CANDIDATE_BACKEND = "fragile-parser-clang"
TASK_LEAF = "M7.1"
DEFAULT_NON_RPC_CORPUS: tuple[str, ...] = (
    "tests/cpp/add_simple.cpp",
    "tests/cpp/factorial.cpp",
    "tests/cpp/namespace.cpp",
    "tests/cpp/class.cpp",
    "tests/cpp/constructor.cpp",
    "tests/cpp/grammar/14_struct_constructor.cpp",
    "tests/clang_integration/namespace_resolution.cpp",
    "tests/clang_integration/virtual_class.cpp",
)
RPC_QUEUE_ITEMS: tuple[tuple[str, str], ...] = (
    (
        "M9.1",
        "Rebuild strict test_rpc and rpcbench with new parser backend and no force-native paths",
    ),
    (
        "M9.2",
        "Run strict runtime replay for test_rpc and rpcbench and capture deterministic manifests",
    ),
    (
        "M9.3",
        "Run deterministic clang-vs-fragile benchmark gate (fragile_avg_qps >= clang_avg_qps)",
    ),
)


@dataclass(frozen=True)
class CommandResult:
    status: int
    stdout: str
    stderr: str
    timed_out: bool


@dataclass(frozen=True)
class FixtureReplayResult:
    fixture_relpath: str
    backend: str
    status: int
    timed_out: bool
    log_dir: Path
    output_object: Path


def shell_join(argv: Sequence[str]) -> str:
    return " ".join(shlex.quote(token) for token in argv)


def write_text(path: Path, value: str) -> None:
    path.write_text(value + "\n", encoding="utf-8")


def write_lines(path: Path, lines: Sequence[str]) -> None:
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def utc_timestamp_token(now: datetime | None = None) -> str:
    point = now if now is not None else datetime.now(timezone.utc)
    return point.astimezone(timezone.utc).strftime("%Y%m%dT%H%M%SZ")


def default_run_root_path(base_dir: Path) -> Path:
    run_name = f"fragile_m7_1_shadow_non_rpc_{utc_timestamp_token()}_p{os.getpid()}"
    return base_dir.resolve() / run_name


def sanitize_backend_token(value: str) -> str:
    cleaned = "".join(
        ch if (ch.isalnum() or ch in ("-", "_", ".")) else "_" for ch in value.strip()
    )
    if not cleaned:
        raise ValueError("backend label must not be empty")
    return cleaned


def sanitize_relpath_token(value: str) -> str:
    return "".join(ch if ch.isalnum() else "_" for ch in value)


def ensure_positive(name: str, value: int) -> None:
    if value <= 0:
        raise ValueError(f"{name} must be > 0, got {value}")


def run_capture(
    argv: Sequence[str],
    *,
    env: Mapping[str, str],
    cwd: Path,
    timeout_seconds: int | None = None,
) -> CommandResult:
    try:
        output = subprocess.run(
            list(argv),
            check=False,
            text=True,
            capture_output=True,
            env=dict(env),
            cwd=str(cwd),
            timeout=timeout_seconds,
        )
        return CommandResult(
            status=output.returncode,
            stdout=output.stdout,
            stderr=output.stderr,
            timed_out=False,
        )
    except subprocess.TimeoutExpired as exc:
        stdout = exc.stdout if isinstance(exc.stdout, str) else ""
        stderr = exc.stderr if isinstance(exc.stderr, str) else ""
        if stderr and not stderr.endswith("\n"):
            stderr += "\n"
        stderr += (
            f"error: command timed out after {timeout_seconds}s: "
            f"{shell_join(list(argv))}\n"
        )
        return CommandResult(
            status=COMMAND_TIMEOUT_STATUS,
            stdout=stdout,
            stderr=stderr,
            timed_out=True,
        )
    except OSError as exc:
        return CommandResult(
            status=127,
            stdout="",
            stderr=(
                f"error: failed to run command: {shell_join(list(argv))} ({exc})\n"
            ),
            timed_out=False,
        )


def write_command_result(log_dir: Path, step: str, result: CommandResult) -> None:
    write_text(log_dir / f"{step}.status", str(result.status))
    write_lines(log_dir / f"{step}.stdout.log", result.stdout.splitlines())
    write_lines(log_dir / f"{step}.stderr.log", result.stderr.splitlines())


def parse_key_value_file(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key.strip()] = value.strip()
    return values


def ensure_fragilec_binary(
    *,
    fragilec_bin: Path,
    workspace_root: Path,
    run_root: Path,
    skip_build: bool,
) -> Path:
    fragilec_bin = fragilec_bin.resolve()
    if fragilec_bin.exists():
        return fragilec_bin
    if skip_build:
        raise FileNotFoundError(f"missing fragilec binary: {fragilec_bin}")

    cmd = ["cargo", "build", "-p", "fragile-cli", "--bin", "fragilec"]
    build_result = run_capture(cmd, env=os.environ, cwd=workspace_root)
    write_command_result(run_root, "build_fragilec", build_result)
    write_text(run_root / "build_fragilec.command", shell_join(cmd))
    if build_result.status != 0:
        raise RuntimeError(
            "failed to build fragilec binary "
            "(see build_fragilec.stderr.log for details)"
        )
    if not fragilec_bin.exists():
        raise FileNotFoundError(
            "fragilec build completed but binary was not found at "
            f"{fragilec_bin}"
        )
    return fragilec_bin


def resolve_fixtures(workspace_root: Path, fixtures: Sequence[str]) -> tuple[str, ...]:
    resolved: list[str] = []
    for raw in fixtures:
        rel = raw.strip()
        if not rel:
            raise ValueError("fixture entries must not be empty")
        if os.path.isabs(rel):
            raise ValueError(f"fixture must be a workspace-relative path: {rel}")
        source = workspace_root / rel
        if not source.exists():
            raise FileNotFoundError(f"fixture source does not exist: {source}")
        if not source.is_file():
            raise FileNotFoundError(f"fixture source is not a file: {source}")
        resolved.append(rel)

    if not resolved:
        raise ValueError("at least one fixture must be provided")
    return tuple(resolved)


def fixture_non_worsening(baseline_status: int, candidate_status: int) -> bool:
    if baseline_status == 0:
        return candidate_status == 0
    return True


def run_fixture_replay(
    *,
    fragilec_bin: Path,
    workspace_root: Path,
    run_root: Path,
    fixture_relpath: str,
    fixture_index: int,
    backend: str,
    compile_timeout_seconds: int,
) -> FixtureReplayResult:
    backend_token = sanitize_backend_token(backend)
    fixture_token = sanitize_relpath_token(fixture_relpath)
    log_dir = run_root / f"backend_{backend_token}" / f"fixture_{fixture_index:03d}_{fixture_token}"
    log_dir.mkdir(parents=True, exist_ok=True)

    source_path = workspace_root / fixture_relpath
    output_object = log_dir / "output.o"
    cmd = [
        str(fragilec_bin),
        str(source_path),
        "-c",
        "-o",
        str(output_object),
    ]
    env = dict(os.environ)
    env["FRAGILEC_MODE"] = "strict"
    env["FRAGILEC_PARSER_BACKEND"] = backend
    result = run_capture(
        cmd,
        env=env,
        cwd=workspace_root,
        timeout_seconds=compile_timeout_seconds,
    )

    write_text(log_dir / "compile.command", shell_join(cmd))
    write_text(log_dir / "compile.fixture_relpath", fixture_relpath)
    write_text(log_dir / "compile.backend", backend)
    write_text(log_dir / "compile.timed_out", "true" if result.timed_out else "false")
    write_command_result(log_dir, "compile", result)

    return FixtureReplayResult(
        fixture_relpath=fixture_relpath,
        backend=backend,
        status=result.status,
        timed_out=result.timed_out,
        log_dir=log_dir,
        output_object=output_object,
    )


def write_rpc_queue_manifest(
    *,
    path: Path,
    run_root: Path,
    baseline_backend: str,
    candidate_backend: str,
    fixture_count: int,
) -> None:
    lines = [
        "version=1",
        f"task_leaf={TASK_LEAF}",
        "queue_kind=rpc_corpus_deferred_for_m9",
        f"run_root={run_root}",
        f"baseline_backend={baseline_backend}",
        f"candidate_backend={candidate_backend}",
        f"non_rpc_fixture_count={fixture_count}",
        "defer_reason=Program priority keeps RPC closure after parser migration hardening",
        "todo_priority_reference=P0_before_P1",
        "acceptance_gate_reference=G4,G5,G6",
        "rpc_targets=test_rpc,rpcbench",
        f"queued_item_count={len(RPC_QUEUE_ITEMS)}",
    ]
    for index, (todo_ref, summary) in enumerate(RPC_QUEUE_ITEMS, start=1):
        lines.append(f"queued_item_{index:03d}_todo={todo_ref}")
        lines.append(f"queued_item_{index:03d}_summary={summary}")
    write_lines(path, lines)


def required_relpaths_for_run(
    *,
    fixtures: Sequence[str],
    backends: Sequence[str],
) -> tuple[str, ...]:
    relpaths = [
        "shadow_non_rpc_commands.txt",
        "shadow_non_rpc_manifest.txt",
        "rpc_corpus_queue_for_m9.txt",
    ]
    for backend in backends:
        backend_token = sanitize_backend_token(backend)
        for index, fixture_relpath in enumerate(fixtures, start=1):
            fixture_token = sanitize_relpath_token(fixture_relpath)
            base = f"backend_{backend_token}/fixture_{index:03d}_{fixture_token}"
            relpaths.extend(
                [
                    f"{base}/compile.command",
                    f"{base}/compile.fixture_relpath",
                    f"{base}/compile.backend",
                    f"{base}/compile.timed_out",
                    f"{base}/compile.status",
                    f"{base}/compile.stdout.log",
                    f"{base}/compile.stderr.log",
                ]
            )
    return tuple(relpaths)


def write_required_artifact_manifest(
    *,
    manifest_path: Path,
    run_root: Path,
    required_relpaths: Sequence[str],
) -> tuple[int, int]:
    missing_count = 0
    lines = [
        "version=1",
        f"task_leaf={TASK_LEAF}",
        f"run_root={run_root}",
        f"required_artifact_count={len(required_relpaths)}",
    ]
    for index, relpath in enumerate(required_relpaths, start=1):
        abspath = run_root / relpath
        exists = abspath.exists()
        if not exists:
            missing_count += 1
        lines.extend(
            [
                f"required_artifact_{index:03d}_relpath={relpath}",
                f"required_artifact_{index:03d}_exists={'true' if exists else 'false'}",
                f"required_artifact_{index:03d}_abspath={abspath}",
            ]
        )
    lines.append(f"missing_required_artifact_count={missing_count}")
    write_lines(manifest_path, lines)
    return len(required_relpaths), missing_count


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    script_dir = Path(__file__).resolve().parent
    workspace_root = script_dir.parent
    parser = argparse.ArgumentParser(
        description=(
            "Run strict fragilec parser backend shadow mode over representative "
            "non-RPC corpus for TODO leaf M7.1"
        )
    )
    parser.add_argument("--workspace-root", type=Path, default=workspace_root)
    parser.add_argument(
        "--run-root",
        type=Path,
        default=default_run_root_path(Path("/tmp")),
    )
    parser.add_argument(
        "--fragilec-bin",
        type=Path,
        default=workspace_root / "target" / "debug" / "fragilec",
    )
    parser.add_argument("--baseline-backend", default=DEFAULT_BASELINE_BACKEND)
    parser.add_argument("--candidate-backend", default=DEFAULT_CANDIDATE_BACKEND)
    parser.add_argument(
        "--compile-timeout-seconds",
        type=int,
        default=120,
    )
    parser.add_argument(
        "--fixture",
        action="append",
        default=[],
        help="workspace-relative fixture path; repeat to override default corpus",
    )
    parser.add_argument(
        "--skip-fragilec-build",
        action="store_true",
        help="fail instead of building fragilec when --fragilec-bin is missing",
    )
    ns = parser.parse_args(list(argv))

    ensure_positive("compile-timeout-seconds", ns.compile_timeout_seconds)
    sanitize_backend_token(ns.baseline_backend)
    sanitize_backend_token(ns.candidate_backend)
    return ns


def main(argv: Sequence[str]) -> int:
    try:
        ns = parse_args(argv)
        workspace_root = ns.workspace_root.resolve()
        if not workspace_root.exists():
            raise FileNotFoundError(f"workspace root does not exist: {workspace_root}")

        run_root = ns.run_root.resolve()
        run_root.mkdir(parents=True, exist_ok=True)

        fixtures = resolve_fixtures(
            workspace_root,
            ns.fixture if ns.fixture else DEFAULT_NON_RPC_CORPUS,
        )
        baseline_backend = sanitize_backend_token(ns.baseline_backend)
        candidate_backend = sanitize_backend_token(ns.candidate_backend)

        fragilec_bin = ensure_fragilec_binary(
            fragilec_bin=ns.fragilec_bin,
            workspace_root=workspace_root,
            run_root=run_root,
            skip_build=ns.skip_fragilec_build,
        )

        command_lines = [
            "version=1",
            f"task_leaf={TASK_LEAF}",
            f"run_root={run_root}",
            f"workspace_root={workspace_root}",
            f"fragilec_bin={fragilec_bin}",
            "strict_mode=true",
            f"baseline_backend={baseline_backend}",
            f"candidate_backend={candidate_backend}",
            f"fixture_count={len(fixtures)}",
        ]
        for index, fixture in enumerate(fixtures, start=1):
            command_lines.append(f"fixture_{index:03d}_relpath={fixture}")

        results_by_backend: dict[str, list[FixtureReplayResult]] = {
            baseline_backend: [],
            candidate_backend: [],
        }

        for backend in (baseline_backend, candidate_backend):
            for index, fixture in enumerate(fixtures, start=1):
                replay = run_fixture_replay(
                    fragilec_bin=fragilec_bin,
                    workspace_root=workspace_root,
                    run_root=run_root,
                    fixture_relpath=fixture,
                    fixture_index=index,
                    backend=backend,
                    compile_timeout_seconds=ns.compile_timeout_seconds,
                )
                command_lines.extend(
                    [
                        f"command_backend_{backend}_fixture_{index:03d}={shell_join([str(fragilec_bin), str((workspace_root / fixture)), '-c', '-o', str(replay.output_object)])}",
                        f"command_backend_{backend}_fixture_{index:03d}_log_dir={replay.log_dir}",
                    ]
                )
                results_by_backend[backend].append(replay)

        commands_path = run_root / "shadow_non_rpc_commands.txt"
        write_lines(commands_path, command_lines)

        baseline_results = results_by_backend[baseline_backend]
        candidate_results = results_by_backend[candidate_backend]
        baseline_success_count = sum(1 for item in baseline_results if item.status == 0)
        candidate_success_count = sum(1 for item in candidate_results if item.status == 0)
        baseline_failure_count = len(baseline_results) - baseline_success_count
        candidate_failure_count = len(candidate_results) - candidate_success_count

        queue_manifest_path = run_root / "rpc_corpus_queue_for_m9.txt"
        write_rpc_queue_manifest(
            path=queue_manifest_path,
            run_root=run_root,
            baseline_backend=baseline_backend,
            candidate_backend=candidate_backend,
            fixture_count=len(fixtures),
        )

        required_relpaths = required_relpaths_for_run(
            fixtures=fixtures,
            backends=(baseline_backend, candidate_backend),
        )
        required_manifest_path = run_root / "shadow_non_rpc_required_artifacts_manifest.txt"
        required_artifact_count, missing_required_artifact_count = (
            write_required_artifact_manifest(
                manifest_path=required_manifest_path,
                run_root=run_root,
                required_relpaths=required_relpaths,
            )
        )

        summary_lines = [
            "version=1",
            f"task_leaf={TASK_LEAF}",
            "mode=strict",
            "corpus_kind=representative_non_rpc",
            f"run_root={run_root}",
            f"workspace_root={workspace_root}",
            f"fragilec_bin={fragilec_bin}",
            f"baseline_backend={baseline_backend}",
            f"candidate_backend={candidate_backend}",
            f"fixture_count={len(fixtures)}",
            f"baseline_success_count={baseline_success_count}",
            f"baseline_failure_count={baseline_failure_count}",
            f"candidate_success_count={candidate_success_count}",
            f"candidate_failure_count={candidate_failure_count}",
            f"candidate_non_worsening_vs_baseline={'true' if candidate_failure_count <= baseline_failure_count else 'false'}",
            f"commands_manifest={commands_path}",
            f"rpc_queue_manifest={queue_manifest_path}",
            f"required_artifacts_manifest={required_manifest_path}",
            f"required_artifact_count={required_artifact_count}",
            f"missing_required_artifact_count={missing_required_artifact_count}",
        ]

        non_worsening_count = 0
        worsening_count = 0
        for index, fixture in enumerate(fixtures, start=1):
            baseline = baseline_results[index - 1]
            candidate = candidate_results[index - 1]
            non_worsening = fixture_non_worsening(baseline.status, candidate.status)
            if non_worsening:
                non_worsening_count += 1
            else:
                worsening_count += 1

            summary_lines.extend(
                [
                    f"fixture_{index:03d}_relpath={fixture}",
                    f"fixture_{index:03d}_baseline_status={baseline.status}",
                    f"fixture_{index:03d}_candidate_status={candidate.status}",
                    f"fixture_{index:03d}_baseline_timed_out={'true' if baseline.timed_out else 'false'}",
                    f"fixture_{index:03d}_candidate_timed_out={'true' if candidate.timed_out else 'false'}",
                    f"fixture_{index:03d}_non_worsening={'true' if non_worsening else 'false'}",
                    f"fixture_{index:03d}_baseline_log_dir={baseline.log_dir}",
                    f"fixture_{index:03d}_candidate_log_dir={candidate.log_dir}",
                ]
            )

        summary_lines.extend(
            [
                f"fixture_non_worsening_count={non_worsening_count}",
                f"fixture_worsening_count={worsening_count}",
            ]
        )

        summary_manifest_path = run_root / "shadow_non_rpc_manifest.txt"
        write_lines(summary_manifest_path, summary_lines)

        # Re-check required artifact contract after all summaries are written.
        required_artifact_count, missing_required_artifact_count = (
            write_required_artifact_manifest(
                manifest_path=required_manifest_path,
                run_root=run_root,
                required_relpaths=required_relpaths,
            )
        )

        summary = parse_key_value_file(summary_manifest_path)
        summary["required_artifact_count"] = str(required_artifact_count)
        summary["missing_required_artifact_count"] = str(missing_required_artifact_count)
        write_lines(
            summary_manifest_path,
            [f"{key}={summary[key]}" for key in summary],
        )

        print(run_root)
        return 0
    except Exception as exc:  # pylint: disable=broad-except
        print(f"error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
