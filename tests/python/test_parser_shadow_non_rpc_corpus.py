import os
import subprocess
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "parser_shadow_non_rpc_corpus.py"


class ParserShadowNonRpcCorpusTests(unittest.TestCase):
    def _write_fake_fragilec(self, path: Path) -> None:
        path.write_text(
            "\n".join(
                [
                    "#!/usr/bin/env python3",
                    "import os",
                    "import sys",
                    "import time",
                    "from pathlib import Path",
                    "",
                    "def write_stage_trace(path, status):",
                    "    trace_path = Path(path)",
                    "    trace_path.parent.mkdir(parents=True, exist_ok=True)",
                    "    if status == 'completed':",
                    "        lines = [",
                    "            'status=started',",
                    "            'event=stage_start stage=parse',",
                    "            'event=stage_end stage=parse status=ok elapsed_ms=11',",
                    "            'event=stage_skip stage=export elapsed_ms=0 reason=backend_without_export',",
                    "            'event=stage_skip stage=enrichment elapsed_ms=0 reason=backend_without_enrichment',",
                    "            'event=stage_start stage=codegen',",
                    "            'event=stage_end stage=codegen status=ok elapsed_ms=9',",
                    "            'summary parse_ms=11 export_ms=0 enrichment_ms=0 codegen_ms=9 total_ms=20',",
                    "            'status=completed',",
                    "        ]",
                    "    else:",
                    "        lines = [",
                    "            'status=started',",
                    "            'event=stage_start stage=parse',",
                    "            'event=stage_end stage=parse status=ok elapsed_ms=2',",
                    "            'event=stage_start stage=codegen',",
                    "            'event=stage_end stage=codegen status=error elapsed_ms=1',",
                    "            'summary parse_ms=2 export_ms=0 enrichment_ms=0 codegen_ms=1 total_ms=3',",
                    "            'status=failed',",
                    "        ]",
                    "    trace_path.write_text('\\n'.join(lines) + '\\n', encoding='utf-8')",
                    "",
                    "args = sys.argv[1:]",
                    "source = None",
                    "output = None",
                    "i = 0",
                    "while i < len(args):",
                    "    token = args[i]",
                    "    if token == '-o' and i + 1 < len(args):",
                    "        output = args[i + 1]",
                    "        i += 2",
                    "        continue",
                    "    if source is None and not token.startswith('-'):",
                    "        source = token",
                    "    i += 1",
                    "",
                    "if source is None:",
                    "    print('missing source argument', file=sys.stderr)",
                    "    raise SystemExit(2)",
                    "",
                    "backend = os.environ.get('FRAGILEC_PARSER_BACKEND', '<unset>')",
                    "source_name = Path(source).name",
                    "stage_timing_path = os.environ.get('FRAGILEC_TRANSPILE_STAGE_TIMING_PATH', '')",
                    "fail_backend = os.environ.get('FAKE_FAIL_BACKEND', '')",
                    "fail_fixture = os.environ.get('FAKE_FAIL_FIXTURE', '')",
                    "fail_error_kind = os.environ.get('FAKE_FAIL_ERROR_KIND', 'generic')",
                    "timeout_backend = os.environ.get('FAKE_TIMEOUT_BACKEND', '')",
                    "timeout_fixture = os.environ.get('FAKE_TIMEOUT_FIXTURE', '')",
                    "timeout_sleep = float(os.environ.get('FAKE_TIMEOUT_SLEEP_SECS', '2.0'))",
                    "",
                    "is_timeout = backend == timeout_backend and source_name == timeout_fixture",
                    "is_fail = backend == fail_backend and source_name == fail_fixture",
                    "",
                    "if is_timeout:",
                    "    time.sleep(timeout_sleep)",
                    "",
                    "if stage_timing_path:",
                    "    write_stage_trace(stage_timing_path, 'failed' if is_fail else 'completed')",
                    "",
                    "if is_fail:",
                    "    if fail_error_kind == 'e0425':",
                    "        print('error[E0425]: cannot find value `missing_symbol` in this scope', file=sys.stderr)",
                    "    elif fail_error_kind == 'e0428':",
                    "        print('error[E0428]: the name `dupe_symbol` is defined multiple times', file=sys.stderr)",
                    "    elif fail_error_kind == 'other_rustc':",
                    "        print('error[E0599]: no method named `x` found for struct `Point`', file=sys.stderr)",
                    "    else:",
                    "        print(f'simulated compile failure backend={backend} fixture={source_name}', file=sys.stderr)",
                    "    raise SystemExit(11)",
                    "",
                    "if output is not None:",
                    "    output_path = Path(output)",
                    "    output_path.parent.mkdir(parents=True, exist_ok=True)",
                    "    output_path.write_text(f'backend={backend}\\nsource={source}\\n', encoding='utf-8')",
                    "",
                    "print(f'compiled source={source} backend={backend}')",
                    "raise SystemExit(0)",
                ]
            ),
            encoding="utf-8",
        )
        path.chmod(0o755)

    def _parse_manifest(self, path: Path) -> dict[str, str]:
        values: dict[str, str] = {}
        for line in path.read_text(encoding="utf-8").splitlines():
            if "=" not in line:
                continue
            key, value = line.split("=", 1)
            values[key.strip()] = value.strip()
        return values

    def _run_script(
        self,
        *,
        workspace_root: Path,
        run_root: Path,
        fragilec_bin: Path,
        fixtures: list[str],
        compile_timeout_seconds: int = 10,
        extra_env: dict[str, str] | None = None,
    ) -> subprocess.CompletedProcess[str]:
        cmd = [
            "python3",
            str(SCRIPT_PATH),
            "--workspace-root",
            str(workspace_root),
            "--run-root",
            str(run_root),
            "--fragilec-bin",
            str(fragilec_bin),
            "--compile-timeout-seconds",
            str(compile_timeout_seconds),
            "--skip-fragilec-build",
        ]
        for fixture in fixtures:
            cmd.extend(["--fixture", fixture])

        env = os.environ.copy()
        if extra_env:
            env.update(extra_env)
        return subprocess.run(cmd, check=False, text=True, capture_output=True, env=env)

    def test_manifest_records_m7_2_metrics(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root = tmp_path / "workspace"
            source_dir = workspace_root / "tests" / "shadow"
            source_dir.mkdir(parents=True, exist_ok=True)
            (source_dir / "a.cpp").write_text("int a() { return 1; }\n", encoding="utf-8")
            (source_dir / "b.cpp").write_text("int b() { return 2; }\n", encoding="utf-8")

            fake_fragilec = tmp_path / "fake_fragilec.py"
            self._write_fake_fragilec(fake_fragilec)
            run_root = tmp_path / "run"

            result = self._run_script(
                workspace_root=workspace_root,
                run_root=run_root,
                fragilec_bin=fake_fragilec,
                fixtures=["tests/shadow/a.cpp", "tests/shadow/b.cpp"],
                extra_env={
                    "FAKE_FAIL_BACKEND": "fragile-parser-clang",
                    "FAKE_FAIL_FIXTURE": "b.cpp",
                    "FAKE_FAIL_ERROR_KIND": "e0425",
                },
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            manifest = self._parse_manifest(run_root / "shadow_non_rpc_manifest.txt")
            self.assertEqual(manifest["task_leaf"], "M7.2")
            self.assertEqual(manifest["parity_metrics_version"], "1")
            self.assertEqual(manifest["fixture_count"], "2")
            self.assertEqual(manifest["baseline_backend"], "libtooling")
            self.assertEqual(manifest["candidate_backend"], "fragile-parser-clang")
            self.assertEqual(manifest["baseline_first_failure_class"], "none")
            self.assertEqual(
                manifest["candidate_first_failure_class"],
                "unresolved_name_or_type_e0425",
            )
            self.assertEqual(manifest["baseline_unresolved_name_e0425_total"], "0")
            self.assertEqual(manifest["candidate_unresolved_name_e0425_total"], "1")
            self.assertEqual(manifest["unresolved_name_e0425_delta_vs_baseline"], "1")
            self.assertEqual(manifest["baseline_runtime_status_counts"], "not_run_compile_only:2")
            self.assertIn("not_run_compile_failed:1", manifest["candidate_runtime_status_counts"])
            self.assertIn("not_run_compile_only:1", manifest["candidate_runtime_status_counts"])
            self.assertEqual(manifest["baseline_transpile_timing_present_count"], "2")
            self.assertEqual(manifest["candidate_transpile_timing_present_count"], "2")
            self.assertEqual(manifest["baseline_transpile_total_ms_sum"], "40")
            self.assertEqual(manifest["candidate_transpile_total_ms_sum"], "23")
            self.assertEqual(manifest["candidate_failure_count"], "1")
            self.assertEqual(manifest["fixture_002_candidate_first_failure_class"], "unresolved_name_or_type_e0425")
            self.assertEqual(manifest["fixture_002_candidate_unresolved_name_e0425_count"], "1")
            self.assertEqual(manifest["fixture_002_candidate_runtime_status"], "not_run_compile_failed")
            self.assertEqual(manifest["fixture_002_candidate_transpile_timing_exists"], "true")
            self.assertEqual(manifest["missing_required_artifact_count"], "0")

            queue_manifest = self._parse_manifest(run_root / "rpc_corpus_queue_for_m9.txt")
            self.assertEqual(queue_manifest["task_leaf"], "M7.2")
            self.assertEqual(queue_manifest["queued_item_count"], "3")
            self.assertEqual(queue_manifest["queued_item_001_todo"], "M9.1")
            self.assertEqual(queue_manifest["queued_item_002_todo"], "M9.2")
            self.assertEqual(queue_manifest["queued_item_003_todo"], "M9.3")

            required_manifest = self._parse_manifest(
                run_root / "shadow_non_rpc_required_artifacts_manifest.txt"
            )
            self.assertEqual(required_manifest["task_leaf"], "M7.2")
            self.assertEqual(required_manifest["missing_required_artifact_count"], "0")

    def test_timeout_metrics_are_recorded(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root = tmp_path / "workspace"
            source_dir = workspace_root / "tests" / "shadow"
            source_dir.mkdir(parents=True, exist_ok=True)
            (source_dir / "a.cpp").write_text("int a() { return 1; }\n", encoding="utf-8")
            (source_dir / "b.cpp").write_text("int b() { return 2; }\n", encoding="utf-8")

            fake_fragilec = tmp_path / "fake_fragilec.py"
            self._write_fake_fragilec(fake_fragilec)
            run_root = tmp_path / "run"

            result = self._run_script(
                workspace_root=workspace_root,
                run_root=run_root,
                fragilec_bin=fake_fragilec,
                fixtures=["tests/shadow/a.cpp", "tests/shadow/b.cpp"],
                compile_timeout_seconds=1,
                extra_env={
                    "FAKE_TIMEOUT_BACKEND": "fragile-parser-clang",
                    "FAKE_TIMEOUT_FIXTURE": "b.cpp",
                    "FAKE_TIMEOUT_SLEEP_SECS": "2.5",
                },
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            manifest = self._parse_manifest(run_root / "shadow_non_rpc_manifest.txt")
            self.assertEqual(manifest["task_leaf"], "M7.2")
            self.assertEqual(manifest["fixture_002_candidate_status"], "124")
            self.assertEqual(manifest["fixture_002_candidate_timed_out"], "true")
            self.assertEqual(manifest["candidate_first_failure_class"], "compile_timeout")
            self.assertEqual(manifest["fixture_002_candidate_runtime_status"], "not_run_compile_timeout")
            self.assertEqual(manifest["fixture_002_candidate_transpile_timing_exists"], "false")
            self.assertEqual(manifest["candidate_transpile_timing_present_count"], "1")
            self.assertEqual(manifest["candidate_transpile_total_ms_sample_count"], "1")
            self.assertEqual(manifest["candidate_unresolved_name_e0425_total"], "0")

            stderr_log = (
                run_root
                / "backend_fragile-parser-clang"
                / "fixture_002_tests_shadow_b_cpp"
                / "compile.stderr.log"
            )
            self.assertTrue(stderr_log.exists())
            self.assertIn("timed out", stderr_log.read_text(encoding="utf-8"))

    def test_missing_fixture_path_fails(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root = tmp_path / "workspace"
            workspace_root.mkdir(parents=True, exist_ok=True)

            fake_fragilec = tmp_path / "fake_fragilec.py"
            self._write_fake_fragilec(fake_fragilec)
            run_root = tmp_path / "run"

            result = self._run_script(
                workspace_root=workspace_root,
                run_root=run_root,
                fragilec_bin=fake_fragilec,
                fixtures=["tests/shadow/missing.cpp"],
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("fixture source does not exist", result.stderr)


if __name__ == "__main__":
    unittest.main()
