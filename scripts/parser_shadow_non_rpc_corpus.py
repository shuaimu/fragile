#!/usr/bin/env python3
"""Shadow-mode strict parser backend replay for TODO leaf M7.2.

This harness runs a representative non-RPC source corpus through strict `fragilec`
compile twice under one run root:
- baseline parser backend (default: fragile-parser-clang)
- candidate parser backend (default: fragile-parser-clang)

It emits deterministic per-fixture logs, summary parity metrics, and an explicit
RPC corpus queue artifact for deferred M9 closure.
"""

from __future__ import annotations

import argparse
import os
import shlex
import subprocess
import sys
import time
from collections import Counter
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Mapping, Sequence

COMMAND_TIMEOUT_STATUS = 124
DEFAULT_BASELINE_BACKEND = "fragile-parser-clang"
DEFAULT_CANDIDATE_BACKEND = "fragile-parser-clang"
TASK_LEAF = "M7.2"
FRAGILEC_TRANSPILE_STAGE_TIMING_PATH_ENV = "FRAGILEC_TRANSPILE_STAGE_TIMING_PATH"
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
    elapsed_ms: int


@dataclass(frozen=True)
class TranspileStageTimingSnapshot:
    parse_ms: int | None = None
    export_ms: int | None = None
    enrichment_ms: int | None = None
    codegen_ms: int | None = None
    total_ms: int | None = None
    status: str | None = None
    last_stage_started: str | None = None
    last_stage_completed: str | None = None


@dataclass(frozen=True)
class FixtureReplayResult:
    fixture_relpath: str
    backend: str
    status: int
    timed_out: bool
    log_dir: Path
    output_object: Path
    compile_elapsed_ms: int
    first_failure_class: str
    unresolved_name_e0425_count: int
    runtime_status: str
    transpile_timing_exists: bool
    transpile_timing_path: Path
    transpile_timing: TranspileStageTimingSnapshot


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
    run_name = f"fragile_m7_2_shadow_non_rpc_{utc_timestamp_token()}_p{os.getpid()}"
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


def format_optional_int(value: int | None) -> str:
    return str(value) if value is not None else "na"


def format_optional_str(value: str | None) -> str:
    return value if value is not None else "na"


def format_counter(counter: Counter[str]) -> str:
    if not counter:
        return "none:0"
    return ",".join(f"{key}:{counter[key]}" for key in sorted(counter))


def parse_key_value_token(line: str, key: str) -> str | None:
    for token in line.split():
        prefix = f"{key}="
        if token.startswith(prefix):
            return token[len(prefix) :]
    return None


def assign_transpile_stage_elapsed(
    snapshot: TranspileStageTimingSnapshot,
    stage: str,
    elapsed_ms: int | None,
) -> TranspileStageTimingSnapshot:
    if stage == "parse":
        return TranspileStageTimingSnapshot(
            parse_ms=elapsed_ms,
            export_ms=snapshot.export_ms,
            enrichment_ms=snapshot.enrichment_ms,
            codegen_ms=snapshot.codegen_ms,
            total_ms=snapshot.total_ms,
            status=snapshot.status,
            last_stage_started=snapshot.last_stage_started,
            last_stage_completed=snapshot.last_stage_completed,
        )
    if stage == "export":
        return TranspileStageTimingSnapshot(
            parse_ms=snapshot.parse_ms,
            export_ms=elapsed_ms,
            enrichment_ms=snapshot.enrichment_ms,
            codegen_ms=snapshot.codegen_ms,
            total_ms=snapshot.total_ms,
            status=snapshot.status,
            last_stage_started=snapshot.last_stage_started,
            last_stage_completed=snapshot.last_stage_completed,
        )
    if stage == "enrichment":
        return TranspileStageTimingSnapshot(
            parse_ms=snapshot.parse_ms,
            export_ms=snapshot.export_ms,
            enrichment_ms=elapsed_ms,
            codegen_ms=snapshot.codegen_ms,
            total_ms=snapshot.total_ms,
            status=snapshot.status,
            last_stage_started=snapshot.last_stage_started,
            last_stage_completed=snapshot.last_stage_completed,
        )
    if stage == "codegen":
        return TranspileStageTimingSnapshot(
            parse_ms=snapshot.parse_ms,
            export_ms=snapshot.export_ms,
            enrichment_ms=snapshot.enrichment_ms,
            codegen_ms=elapsed_ms,
            total_ms=snapshot.total_ms,
            status=snapshot.status,
            last_stage_started=snapshot.last_stage_started,
            last_stage_completed=snapshot.last_stage_completed,
        )
    return snapshot


def parse_transpile_stage_timing_trace(
    path: Path,
) -> tuple[bool, TranspileStageTimingSnapshot]:
    if not path.exists():
        return (False, TranspileStageTimingSnapshot())

    content = path.read_text(encoding="utf-8", errors="replace")
    snapshot = TranspileStageTimingSnapshot()

    for raw in content.splitlines():
        line = raw.strip()
        if not line:
            continue

        if line.startswith("status="):
            snapshot = TranspileStageTimingSnapshot(
                parse_ms=snapshot.parse_ms,
                export_ms=snapshot.export_ms,
                enrichment_ms=snapshot.enrichment_ms,
                codegen_ms=snapshot.codegen_ms,
                total_ms=snapshot.total_ms,
                status=line[len("status=") :].strip() or None,
                last_stage_started=snapshot.last_stage_started,
                last_stage_completed=snapshot.last_stage_completed,
            )
            continue

        if line.startswith("event=stage_start "):
            stage = parse_key_value_token(line, "stage")
            if stage:
                snapshot = TranspileStageTimingSnapshot(
                    parse_ms=snapshot.parse_ms,
                    export_ms=snapshot.export_ms,
                    enrichment_ms=snapshot.enrichment_ms,
                    codegen_ms=snapshot.codegen_ms,
                    total_ms=snapshot.total_ms,
                    status=snapshot.status,
                    last_stage_started=stage,
                    last_stage_completed=snapshot.last_stage_completed,
                )
            continue

        if line.startswith("event=stage_end ") or line.startswith("event=stage_skip "):
            stage = parse_key_value_token(line, "stage")
            elapsed_ms = parse_key_value_token(line, "elapsed_ms")
            parsed_elapsed = int(elapsed_ms) if elapsed_ms and elapsed_ms.isdigit() else None
            if stage:
                snapshot = assign_transpile_stage_elapsed(snapshot, stage, parsed_elapsed)
                snapshot = TranspileStageTimingSnapshot(
                    parse_ms=snapshot.parse_ms,
                    export_ms=snapshot.export_ms,
                    enrichment_ms=snapshot.enrichment_ms,
                    codegen_ms=snapshot.codegen_ms,
                    total_ms=snapshot.total_ms,
                    status=snapshot.status,
                    last_stage_started=snapshot.last_stage_started,
                    last_stage_completed=stage,
                )
            continue

        if line.startswith("summary "):
            parse_ms = parse_key_value_token(line, "parse_ms")
            export_ms = parse_key_value_token(line, "export_ms")
            enrichment_ms = parse_key_value_token(line, "enrichment_ms")
            codegen_ms = parse_key_value_token(line, "codegen_ms")
            total_ms = parse_key_value_token(line, "total_ms")
            snapshot = TranspileStageTimingSnapshot(
                parse_ms=int(parse_ms) if parse_ms and parse_ms.isdigit() else snapshot.parse_ms,
                export_ms=int(export_ms)
                if export_ms and export_ms.isdigit()
                else snapshot.export_ms,
                enrichment_ms=int(enrichment_ms)
                if enrichment_ms and enrichment_ms.isdigit()
                else snapshot.enrichment_ms,
                codegen_ms=int(codegen_ms)
                if codegen_ms and codegen_ms.isdigit()
                else snapshot.codegen_ms,
                total_ms=int(total_ms) if total_ms and total_ms.isdigit() else snapshot.total_ms,
                status=snapshot.status,
                last_stage_started=snapshot.last_stage_started,
                last_stage_completed=snapshot.last_stage_completed,
            )
            continue

    return (True, snapshot)


def classify_first_failing_compile_stderr(
    stderr: str,
    *,
    timed_out: bool,
    status: int,
) -> str:
    if status == 0:
        return "none"
    if timed_out:
        return "compile_timeout"

    normalized = stderr.strip()
    if not normalized:
        return "none"
    if "error[E0428]" in normalized:
        return "duplicate_definition_e0428"
    if "error[E0425]" in normalized:
        return "unresolved_name_or_type_e0425"
    if "error[E" in normalized:
        return "other_rustc_error"
    return "non_rustc_error"


def classify_runtime_status(*, status: int, timed_out: bool) -> str:
    if timed_out:
        return "not_run_compile_timeout"
    if status != 0:
        return "not_run_compile_failed"
    return "not_run_compile_only"


def count_error_e0425_occurrences(text: str) -> int:
    return text.count("error[E0425]")


def run_capture(
    argv: Sequence[str],
    *,
    env: Mapping[str, str],
    cwd: Path,
    timeout_seconds: int | None = None,
) -> CommandResult:
    started = time.monotonic()
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
        elapsed_ms = int((time.monotonic() - started) * 1000)
        return CommandResult(
            status=output.returncode,
            stdout=output.stdout,
            stderr=output.stderr,
            timed_out=False,
            elapsed_ms=elapsed_ms,
        )
    except subprocess.TimeoutExpired as exc:
        elapsed_ms = int((time.monotonic() - started) * 1000)
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
            elapsed_ms=elapsed_ms,
        )
    except OSError as exc:
        elapsed_ms = int((time.monotonic() - started) * 1000)
        return CommandResult(
            status=127,
            stdout="",
            stderr=(
                f"error: failed to run command: {shell_join(list(argv))} ({exc})\n"
            ),
            timed_out=False,
            elapsed_ms=elapsed_ms,
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


def first_failure_class_for_backend(results: Sequence[FixtureReplayResult]) -> str:
    for item in results:
        if item.status != 0:
            return item.first_failure_class
    return "none"


def sum_optional(values: Sequence[int | None]) -> tuple[int, int]:
    present = [value for value in values if value is not None]
    return (sum(present), len(present))


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
    transpile_timing_path = log_dir / "transpile_stage_timing.log"
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
    env[FRAGILEC_TRANSPILE_STAGE_TIMING_PATH_ENV] = str(transpile_timing_path)
    result = run_capture(
        cmd,
        env=env,
        cwd=workspace_root,
        timeout_seconds=compile_timeout_seconds,
    )

    first_failure_class = classify_first_failing_compile_stderr(
        result.stderr,
        timed_out=result.timed_out,
        status=result.status,
    )
    unresolved_name_e0425_count = count_error_e0425_occurrences(result.stderr)
    runtime_status = classify_runtime_status(status=result.status, timed_out=result.timed_out)
    transpile_timing_exists, transpile_timing = parse_transpile_stage_timing_trace(
        transpile_timing_path
    )

    write_text(log_dir / "compile.command", shell_join(cmd))
    write_text(log_dir / "compile.fixture_relpath", fixture_relpath)
    write_text(log_dir / "compile.backend", backend)
    write_text(log_dir / "compile.timed_out", "true" if result.timed_out else "false")
    write_command_result(log_dir, "compile", result)
    write_text(log_dir / "compile.elapsed_ms", str(result.elapsed_ms))
    write_text(log_dir / "compile.first_failure_class", first_failure_class)
    write_text(
        log_dir / "compile.unresolved_name_e0425_count",
        str(unresolved_name_e0425_count),
    )
    write_text(log_dir / "compile.runtime_status", runtime_status)
    write_text(
        log_dir / "compile.transpile_timing_exists",
        "true" if transpile_timing_exists else "false",
    )
    write_text(log_dir / "compile.transpile_timing_path", str(transpile_timing_path))
    write_text(log_dir / "compile.transpile_parse_ms", format_optional_int(transpile_timing.parse_ms))
    write_text(
        log_dir / "compile.transpile_export_ms", format_optional_int(transpile_timing.export_ms)
    )
    write_text(
        log_dir / "compile.transpile_enrichment_ms",
        format_optional_int(transpile_timing.enrichment_ms),
    )
    write_text(
        log_dir / "compile.transpile_codegen_ms", format_optional_int(transpile_timing.codegen_ms)
    )
    write_text(log_dir / "compile.transpile_total_ms", format_optional_int(transpile_timing.total_ms))
    write_text(log_dir / "compile.transpile_status", format_optional_str(transpile_timing.status))
    write_text(
        log_dir / "compile.transpile_last_stage_started",
        format_optional_str(transpile_timing.last_stage_started),
    )
    write_text(
        log_dir / "compile.transpile_last_stage_completed",
        format_optional_str(transpile_timing.last_stage_completed),
    )

    return FixtureReplayResult(
        fixture_relpath=fixture_relpath,
        backend=backend,
        status=result.status,
        timed_out=result.timed_out,
        log_dir=log_dir,
        output_object=output_object,
        compile_elapsed_ms=result.elapsed_ms,
        first_failure_class=first_failure_class,
        unresolved_name_e0425_count=unresolved_name_e0425_count,
        runtime_status=runtime_status,
        transpile_timing_exists=transpile_timing_exists,
        transpile_timing_path=transpile_timing_path,
        transpile_timing=transpile_timing,
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
                    f"{base}/compile.elapsed_ms",
                    f"{base}/compile.first_failure_class",
                    f"{base}/compile.unresolved_name_e0425_count",
                    f"{base}/compile.runtime_status",
                    f"{base}/compile.transpile_timing_exists",
                    f"{base}/compile.transpile_timing_path",
                    f"{base}/compile.transpile_parse_ms",
                    f"{base}/compile.transpile_export_ms",
                    f"{base}/compile.transpile_enrichment_ms",
                    f"{base}/compile.transpile_codegen_ms",
                    f"{base}/compile.transpile_total_ms",
                    f"{base}/compile.transpile_status",
                    f"{base}/compile.transpile_last_stage_started",
                    f"{base}/compile.transpile_last_stage_completed",
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
            "non-RPC corpus for TODO leaf M7.2"
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
        default=workspace_root / "target" / "release" / "fragilec",
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

        baseline_first_failure_class = first_failure_class_for_backend(baseline_results)
        candidate_first_failure_class = first_failure_class_for_backend(candidate_results)

        baseline_unresolved_total = sum(
            item.unresolved_name_e0425_count for item in baseline_results
        )
        candidate_unresolved_total = sum(
            item.unresolved_name_e0425_count for item in candidate_results
        )

        baseline_runtime_status_counts = Counter(
            item.runtime_status for item in baseline_results
        )
        candidate_runtime_status_counts = Counter(
            item.runtime_status for item in candidate_results
        )

        baseline_compile_elapsed_ms_sum = sum(item.compile_elapsed_ms for item in baseline_results)
        candidate_compile_elapsed_ms_sum = sum(
            item.compile_elapsed_ms for item in candidate_results
        )

        baseline_transpile_timing_present_count = sum(
            1 for item in baseline_results if item.transpile_timing_exists
        )
        candidate_transpile_timing_present_count = sum(
            1 for item in candidate_results if item.transpile_timing_exists
        )

        baseline_transpile_total_ms_sum, baseline_transpile_total_ms_sample_count = sum_optional(
            [item.transpile_timing.total_ms for item in baseline_results]
        )
        candidate_transpile_total_ms_sum, candidate_transpile_total_ms_sample_count = sum_optional(
            [item.transpile_timing.total_ms for item in candidate_results]
        )

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
            "parity_metrics_version=1",
            "runtime_phase=compile_only_non_rpc",
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
            f"baseline_first_failure_class={baseline_first_failure_class}",
            f"candidate_first_failure_class={candidate_first_failure_class}",
            (
                "first_failure_class_changed_vs_baseline="
                f"{'true' if baseline_first_failure_class != candidate_first_failure_class else 'false'}"
            ),
            f"baseline_unresolved_name_e0425_total={baseline_unresolved_total}",
            f"candidate_unresolved_name_e0425_total={candidate_unresolved_total}",
            (
                "unresolved_name_e0425_delta_vs_baseline="
                f"{candidate_unresolved_total - baseline_unresolved_total}"
            ),
            f"baseline_runtime_status_counts={format_counter(baseline_runtime_status_counts)}",
            f"candidate_runtime_status_counts={format_counter(candidate_runtime_status_counts)}",
            f"baseline_compile_elapsed_ms_sum={baseline_compile_elapsed_ms_sum}",
            f"candidate_compile_elapsed_ms_sum={candidate_compile_elapsed_ms_sum}",
            (
                "compile_elapsed_ms_delta_vs_baseline="
                f"{candidate_compile_elapsed_ms_sum - baseline_compile_elapsed_ms_sum}"
            ),
            (
                "baseline_transpile_timing_present_count="
                f"{baseline_transpile_timing_present_count}"
            ),
            (
                "candidate_transpile_timing_present_count="
                f"{candidate_transpile_timing_present_count}"
            ),
            (
                "baseline_transpile_total_ms_sum="
                f"{baseline_transpile_total_ms_sum}"
            ),
            (
                "candidate_transpile_total_ms_sum="
                f"{candidate_transpile_total_ms_sum}"
            ),
            (
                "baseline_transpile_total_ms_sample_count="
                f"{baseline_transpile_total_ms_sample_count}"
            ),
            (
                "candidate_transpile_total_ms_sample_count="
                f"{candidate_transpile_total_ms_sample_count}"
            ),
            (
                "transpile_total_ms_delta_vs_baseline="
                f"{candidate_transpile_total_ms_sum - baseline_transpile_total_ms_sum}"
            ),
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

            transpile_total_delta = (
                None
                if baseline.transpile_timing.total_ms is None
                or candidate.transpile_timing.total_ms is None
                else candidate.transpile_timing.total_ms - baseline.transpile_timing.total_ms
            )

            summary_lines.extend(
                [
                    f"fixture_{index:03d}_relpath={fixture}",
                    f"fixture_{index:03d}_baseline_status={baseline.status}",
                    f"fixture_{index:03d}_candidate_status={candidate.status}",
                    f"fixture_{index:03d}_baseline_timed_out={'true' if baseline.timed_out else 'false'}",
                    f"fixture_{index:03d}_candidate_timed_out={'true' if candidate.timed_out else 'false'}",
                    f"fixture_{index:03d}_non_worsening={'true' if non_worsening else 'false'}",
                    f"fixture_{index:03d}_baseline_first_failure_class={baseline.first_failure_class}",
                    f"fixture_{index:03d}_candidate_first_failure_class={candidate.first_failure_class}",
                    (
                        f"fixture_{index:03d}_baseline_unresolved_name_e0425_count="
                        f"{baseline.unresolved_name_e0425_count}"
                    ),
                    (
                        f"fixture_{index:03d}_candidate_unresolved_name_e0425_count="
                        f"{candidate.unresolved_name_e0425_count}"
                    ),
                    (
                        f"fixture_{index:03d}_unresolved_name_e0425_delta_vs_baseline="
                        f"{candidate.unresolved_name_e0425_count - baseline.unresolved_name_e0425_count}"
                    ),
                    f"fixture_{index:03d}_baseline_runtime_status={baseline.runtime_status}",
                    f"fixture_{index:03d}_candidate_runtime_status={candidate.runtime_status}",
                    f"fixture_{index:03d}_baseline_compile_elapsed_ms={baseline.compile_elapsed_ms}",
                    f"fixture_{index:03d}_candidate_compile_elapsed_ms={candidate.compile_elapsed_ms}",
                    (
                        f"fixture_{index:03d}_compile_elapsed_ms_delta_vs_baseline="
                        f"{candidate.compile_elapsed_ms - baseline.compile_elapsed_ms}"
                    ),
                    (
                        f"fixture_{index:03d}_baseline_transpile_timing_exists="
                        f"{'true' if baseline.transpile_timing_exists else 'false'}"
                    ),
                    (
                        f"fixture_{index:03d}_candidate_transpile_timing_exists="
                        f"{'true' if candidate.transpile_timing_exists else 'false'}"
                    ),
                    (
                        f"fixture_{index:03d}_baseline_transpile_total_ms="
                        f"{format_optional_int(baseline.transpile_timing.total_ms)}"
                    ),
                    (
                        f"fixture_{index:03d}_candidate_transpile_total_ms="
                        f"{format_optional_int(candidate.transpile_timing.total_ms)}"
                    ),
                    (
                        f"fixture_{index:03d}_transpile_total_ms_delta_vs_baseline="
                        f"{format_optional_int(transpile_total_delta)}"
                    ),
                    (
                        f"fixture_{index:03d}_baseline_transpile_status="
                        f"{format_optional_str(baseline.transpile_timing.status)}"
                    ),
                    (
                        f"fixture_{index:03d}_candidate_transpile_status="
                        f"{format_optional_str(candidate.transpile_timing.status)}"
                    ),
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
