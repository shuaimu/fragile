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
                    "fail_backend = os.environ.get('FAKE_FAIL_BACKEND', '')",
                    "fail_fixture = os.environ.get('FAKE_FAIL_FIXTURE', '')",
                    "timeout_backend = os.environ.get('FAKE_TIMEOUT_BACKEND', '')",
                    "timeout_fixture = os.environ.get('FAKE_TIMEOUT_FIXTURE', '')",
                    "timeout_sleep = float(os.environ.get('FAKE_TIMEOUT_SLEEP_SECS', '2.0'))",
                    "",
                    "if backend == timeout_backend and source_name == timeout_fixture:",
                    "    time.sleep(timeout_sleep)",
                    "",
                    "if backend == fail_backend and source_name == fail_fixture:",
                    "    print(f'simulated compile failure backend={backend} fixture={source_name}', file=sys.stderr)",
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

    def test_manifest_records_backend_status_and_worsening(self) -> None:
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
                },
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            manifest = self._parse_manifest(run_root / "shadow_non_rpc_manifest.txt")
            self.assertEqual(manifest["task_leaf"], "M7.1")
            self.assertEqual(manifest["fixture_count"], "2")
            self.assertEqual(manifest["baseline_backend"], "libtooling")
            self.assertEqual(manifest["candidate_backend"], "fragile-parser-clang")
            self.assertEqual(manifest["fixture_001_relpath"], "tests/shadow/a.cpp")
            self.assertEqual(manifest["fixture_002_relpath"], "tests/shadow/b.cpp")
            self.assertEqual(manifest["fixture_001_baseline_status"], "0")
            self.assertEqual(manifest["fixture_001_candidate_status"], "0")
            self.assertEqual(manifest["fixture_002_baseline_status"], "0")
            self.assertEqual(manifest["fixture_002_candidate_status"], "11")
            self.assertEqual(manifest["fixture_002_non_worsening"], "false")
            self.assertEqual(manifest["candidate_non_worsening_vs_baseline"], "false")

            queue_manifest = self._parse_manifest(run_root / "rpc_corpus_queue_for_m9.txt")
            self.assertEqual(queue_manifest["task_leaf"], "M7.1")
            self.assertEqual(queue_manifest["queued_item_count"], "3")
            self.assertEqual(queue_manifest["queued_item_001_todo"], "M9.1")
            self.assertEqual(queue_manifest["queued_item_002_todo"], "M9.2")
            self.assertEqual(queue_manifest["queued_item_003_todo"], "M9.3")
            self.assertIn("test_rpc", queue_manifest["rpc_targets"])
            self.assertIn("rpcbench", queue_manifest["rpc_targets"])

            required_manifest = self._parse_manifest(
                run_root / "shadow_non_rpc_required_artifacts_manifest.txt"
            )
            self.assertEqual(required_manifest["task_leaf"], "M7.1")
            self.assertEqual(required_manifest["missing_required_artifact_count"], "0")

    def test_timeout_is_recorded_with_deterministic_status(self) -> None:
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
            self.assertEqual(manifest["fixture_002_candidate_status"], "124")
            self.assertEqual(manifest["fixture_002_candidate_timed_out"], "true")

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
