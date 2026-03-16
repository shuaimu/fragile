import os
import subprocess
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "mako_rpc_parser_backend_ab.py"


class MakoRpcParserBackendAbTests(unittest.TestCase):
    def _write_fake_strict_baseline(self, path: Path) -> None:
        path.write_text(
            "\n".join(
                [
                    "#!/usr/bin/env python3",
                    "import argparse",
                    "import os",
                    "from pathlib import Path",
                    "",
                    "parser = argparse.ArgumentParser()",
                    "parser.add_argument('--run-root', required=True)",
                    "parser.add_argument('--lanes', required=True)",
                    "args, _ = parser.parse_known_args()",
                    "",
                    "backend = os.environ.get('FRAGILEC_PARSER_BACKEND', 'unset')",
                    "fail_backend = os.environ.get('FAKE_FAIL_BACKEND', '')",
                    "skip_manifest_backend = os.environ.get('FAKE_SKIP_MANIFEST_BACKEND', '')",
                    "identical = os.environ.get('FAKE_IDENTICAL', '0') == '1'",
                    "",
                    "run_root = Path(args.run_root)",
                    "run_root.mkdir(parents=True, exist_ok=True)",
                    "",
                    "if backend == skip_manifest_backend:",
                    "    print(run_root)",
                    "    raise SystemExit(0)",
                    "",
                    "if identical:",
                    "    build_status = '124'",
                    "    failure_class = 'build_timeout'",
                    "    marker = 'same'",
                    "else:",
                    "    if backend == 'libtooling':",
                    "        build_status = '124'",
                    "        failure_class = 'build_timeout'",
                    "    else:",
                    "        build_status = '0'",
                    "        failure_class = 'none'",
                    "    marker = backend",
                    "",
                    "lines = [",
                    "    'version=1',",
                    "    'task_leaf=M0.1',",
                    "    f'run_root={run_root}',",
                    "    f'lanes={args.lanes}',",
                    "    f'backend_observed={marker}',",
                    "    f'backend_marker={marker}',",
                    "    'strict_mode=true',",
                    "    'harness_status=0',",
                    "    'inventory_status=0',",
                    "    'replay_status=0',",
                    "    f'lane_fragilec_build_status={build_status}',",
                    "    f'lane_fragilec_failure_class={failure_class}',",
                    "    f'harness_manifest={run_root / \"benchmark_harness_manifest.txt\"}',",
                    "    f'inventory_manifest={run_root / \"rpc_compile_blocker_inventory_manifest.txt\"}',",
                    "    f'replay_manifest={run_root / \"rpc_compile_blocker_replay_manifest.txt\"}',",
                    "    f'stage_timing_path={run_root / \"fragilec_transpile_stage_timing.log\"}',",
                    "]",
                    "(run_root / 'strict_baseline_manifest.txt').write_text('\\n'.join(lines) + '\\n', encoding='utf-8')",
                    "print(run_root)",
                    "if backend == fail_backend:",
                    "    raise SystemExit(3)",
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
        run_root: Path,
        workspace_root: Path,
        mako_root: Path,
        strict_baseline_script: Path,
        extra_env: dict[str, str] | None = None,
        baseline_backend: str = "libtooling",
        candidate_backend: str = "libclang",
    ) -> subprocess.CompletedProcess[str]:
        cmd = [
            "python3",
            str(SCRIPT_PATH),
            "--workspace-root",
            str(workspace_root),
            "--mako-root",
            str(mako_root),
            "--run-root",
            str(run_root),
            "--baseline-backend",
            baseline_backend,
            "--candidate-backend",
            candidate_backend,
            "--lanes",
            "fragilec",
            "--jobs",
            "1",
            "--trials",
            "1",
            "--base-port",
            "23000",
            "--strict-baseline-script",
            str(strict_baseline_script),
            "--harness-script",
            str(strict_baseline_script),
            "--inventory-script",
            str(strict_baseline_script),
            "--replay-script",
            str(strict_baseline_script),
            "--replay-timeout-seconds",
            "15",
            "--replay-max-replays",
            "1",
        ]
        env = os.environ.copy()
        if extra_env:
            env.update(extra_env)
        return subprocess.run(cmd, check=False, text=True, capture_output=True, env=env)

    def test_manifest_diff_records_backend_specific_changes(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root = tmp_path / "workspace"
            mako_root = workspace_root / "vendor" / "mako"
            mako_root.mkdir(parents=True, exist_ok=True)
            run_root = tmp_path / "run"
            strict_baseline_script = tmp_path / "fake_strict_baseline.py"
            self._write_fake_strict_baseline(strict_baseline_script)

            result = self._run_script(
                run_root=run_root,
                workspace_root=workspace_root,
                mako_root=mako_root,
                strict_baseline_script=strict_baseline_script,
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            manifest = self._parse_manifest(run_root / "parser_backend_ab_manifest.txt")
            self.assertEqual(manifest["task_leaf"], "M0.2")
            self.assertEqual(manifest["baseline_backend"], "libtooling")
            self.assertEqual(manifest["candidate_backend"], "libclang")
            self.assertEqual(manifest["baseline_command_status"], "0")
            self.assertEqual(manifest["candidate_command_status"], "0")
            self.assertEqual(manifest["comparable_equal"], "false")
            self.assertEqual(manifest["different_001_key"], "backend_marker")
            self.assertEqual(manifest["different_001_baseline"], "libtooling")
            self.assertEqual(manifest["different_001_candidate"], "libclang")
            self.assertEqual(manifest["different_002_key"], "backend_observed")
            self.assertEqual(manifest["different_002_baseline"], "libtooling")
            self.assertEqual(manifest["different_002_candidate"], "libclang")

            baseline_comp = self._parse_manifest(
                run_root / "parser_backend_ab_baseline_comparable_manifest.txt"
            )
            self.assertNotIn("run_root", baseline_comp)
            self.assertNotIn("harness_manifest", baseline_comp)
            self.assertIn("backend_observed", baseline_comp)

    def test_identical_comparable_manifests_report_equal(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root = tmp_path / "workspace"
            mako_root = workspace_root / "vendor" / "mako"
            mako_root.mkdir(parents=True, exist_ok=True)
            run_root = tmp_path / "run"
            strict_baseline_script = tmp_path / "fake_strict_baseline.py"
            self._write_fake_strict_baseline(strict_baseline_script)

            result = self._run_script(
                run_root=run_root,
                workspace_root=workspace_root,
                mako_root=mako_root,
                strict_baseline_script=strict_baseline_script,
                extra_env={"FAKE_IDENTICAL": "1"},
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            manifest = self._parse_manifest(run_root / "parser_backend_ab_manifest.txt")
            self.assertEqual(manifest["different_key_count"], "0")
            self.assertEqual(manifest["missing_in_baseline_count"], "0")
            self.assertEqual(manifest["missing_in_candidate_count"], "0")
            self.assertEqual(manifest["comparable_equal"], "true")

    def test_missing_candidate_manifest_fails(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root = tmp_path / "workspace"
            mako_root = workspace_root / "vendor" / "mako"
            mako_root.mkdir(parents=True, exist_ok=True)
            run_root = tmp_path / "run"
            strict_baseline_script = tmp_path / "fake_strict_baseline.py"
            self._write_fake_strict_baseline(strict_baseline_script)

            result = self._run_script(
                run_root=run_root,
                workspace_root=workspace_root,
                mako_root=mako_root,
                strict_baseline_script=strict_baseline_script,
                extra_env={"FAKE_SKIP_MANIFEST_BACKEND": "libclang"},
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("missing candidate strict manifest artifact", result.stderr)

    def test_backend_command_failure_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root = tmp_path / "workspace"
            mako_root = workspace_root / "vendor" / "mako"
            mako_root.mkdir(parents=True, exist_ok=True)
            run_root = tmp_path / "run"
            strict_baseline_script = tmp_path / "fake_strict_baseline.py"
            self._write_fake_strict_baseline(strict_baseline_script)

            result = self._run_script(
                run_root=run_root,
                workspace_root=workspace_root,
                mako_root=mako_root,
                strict_baseline_script=strict_baseline_script,
                extra_env={"FAKE_FAIL_BACKEND": "libclang"},
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("strict baseline command failed", result.stderr)
            status = (
                run_root / "parser_backend_ab_candidate.status"
            ).read_text(encoding="utf-8")
            self.assertEqual(status.strip(), "3")


if __name__ == "__main__":
    unittest.main()
