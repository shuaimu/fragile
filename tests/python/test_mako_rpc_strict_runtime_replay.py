import os
import subprocess
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "mako_rpc_strict_runtime_replay.py"


class MakoRpcStrictRuntimeReplayTests(unittest.TestCase):
    def _write_fake_harness(self, path: Path) -> None:
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
                    "parser.add_argument('--trials', required=True)",
                    "args, _ = parser.parse_known_args()",
                    "",
                    "run_root = Path(args.run_root)",
                    "run_root.mkdir(parents=True, exist_ok=True)",
                    "trials = int(args.trials)",
                    "lane = 'fragilec'",
                    "build_status = os.environ.get('FAKE_LANE_BUILD_STATUS', '0')",
                    "test_rpc_status = os.environ.get('FAKE_TEST_RPC_STATUS', '0')",
                    "completed_trials = os.environ.get('FAKE_COMPLETED_TRIALS', str(trials))",
                    "failure_class = os.environ.get('FAKE_FAILURE_CLASS', 'none')",
                    "no_regression = os.environ.get('FAKE_NO_REGRESSION_VERDICT', 'insufficient_data')",
                    "comparison_verdict = os.environ.get('FAKE_COMPARISON_VERDICT', no_regression)",
                    "",
                    "(run_root / 'benchmark_harness_command_plan.txt').write_text('plan\\n', encoding='utf-8')",
                    "(run_root / 'benchmark_expected_artifacts.txt').write_text('expected\\n', encoding='utf-8')",
                    "(run_root / 'benchmark_qps_comparison_manifest.txt').write_text(",
                    "    '\\n'.join([",
                    "        'version=1',",
                    "        f'no_regression_verdict={comparison_verdict}',",
                    "    ]) + '\\n',",
                    "    encoding='utf-8'",
                    ")",
                    "",
                    "manifest_lines = [",
                    "    'version=1',",
                    "    'lanes=fragilec',",
                    "    f'trials={trials}',",
                    "    f'no_regression_verdict={no_regression}',",
                    "    f'lane_{lane}_build_status={build_status}',",
                    "    f'lane_{lane}_test_rpc_status={test_rpc_status}',",
                    "    f'lane_{lane}_completed_trials={completed_trials}',",
                    "    f'lane_{lane}_failure_class={failure_class}',",
                    "]",
                    "(run_root / 'benchmark_harness_manifest.txt').write_text('\\n'.join(manifest_lines) + '\\n', encoding='utf-8')",
                    "",
                    "lane_dir = run_root / f'lane_{lane}'",
                    "lane_dir.mkdir(parents=True, exist_ok=True)",
                    "for step in ('configure', 'clean', 'build', 'test_rpc'):",
                    "    status = '0'",
                    "    if step == 'build':",
                    "        status = build_status",
                    "    if step == 'test_rpc':",
                    "        status = test_rpc_status",
                    "    (lane_dir / f'{step}.status').write_text(status + '\\n', encoding='utf-8')",
                    "    (lane_dir / f'{step}.stdout').write_text(f'{step} stdout\\n', encoding='utf-8')",
                    "    (lane_dir / f'{step}.stderr').write_text(f'{step} stderr\\n', encoding='utf-8')",
                    "",
                    "for trial in range(1, trials + 1):",
                    "    trial_dir = lane_dir / f'trial_{trial:02d}'",
                    "    trial_dir.mkdir(parents=True, exist_ok=True)",
                    "    server_status = os.environ.get('FAKE_RPC_SERVER_STATUS', '0')",
                    "    client_status = os.environ.get('FAKE_RPC_CLIENT_STATUS', '0')",
                    "    (trial_dir / 'rpc_server.status').write_text(server_status + '\\n', encoding='utf-8')",
                    "    (trial_dir / 'rpc_server.stdout').write_text('rpc server stdout\\n', encoding='utf-8')",
                    "    (trial_dir / 'rpc_server.stderr').write_text('rpc server stderr\\n', encoding='utf-8')",
                    "    (trial_dir / 'rpc_client.status').write_text(client_status + '\\n', encoding='utf-8')",
                    "    (trial_dir / 'rpc_client.stdout').write_text('rpc client stdout\\n', encoding='utf-8')",
                    "    (trial_dir / 'rpc_client.stderr').write_text('rpc client stderr\\n', encoding='utf-8')",
                    "",
                    "print(run_root)",
                    "raise SystemExit(int(os.environ.get('FAKE_HARNESS_EXIT', '1')))",
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
        harness_script: Path,
        trials: int = 2,
        extra_env: dict[str, str] | None = None,
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
            "--harness-script",
            str(harness_script),
            "--fragile-cxx",
            str(harness_script),
            "--skip-fragilec-build",
            "--trials",
            str(trials),
            "--jobs",
            "1",
            "--base-port",
            "23000",
            "--rpc-duration-seconds",
            "1",
        ]
        env = os.environ.copy()
        if extra_env:
            env.update(extra_env)
        return subprocess.run(cmd, check=False, text=True, capture_output=True, env=env)

    def test_runtime_replay_accepts_insufficient_data_qps_verdict_when_lane_passes(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root = tmp_path / "workspace"
            mako_root = workspace_root / "vendor" / "mako"
            mako_root.mkdir(parents=True, exist_ok=True)
            (mako_root / "CMakeLists.txt").write_text(
                "cmake_minimum_required(VERSION 3.16)\n",
                encoding="utf-8",
            )
            run_root = tmp_path / "run"
            harness_script = tmp_path / "fake_harness.py"
            self._write_fake_harness(harness_script)

            result = self._run_script(
                run_root=run_root,
                workspace_root=workspace_root,
                mako_root=mako_root,
                harness_script=harness_script,
                trials=2,
                extra_env={
                    "FAKE_HARNESS_EXIT": "1",
                    "FAKE_NO_REGRESSION_VERDICT": "insufficient_data",
                    "FAKE_COMPARISON_VERDICT": "insufficient_data",
                },
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            manifest = self._parse_manifest(run_root / "strict_runtime_replay_manifest.txt")
            self.assertEqual(manifest["task_leaf"], "M9.2")
            self.assertEqual(manifest["strict_env_mode"], "strict")
            self.assertEqual(manifest["strict_env_parser_backend"], "fragile-parser-clang")
            self.assertEqual(manifest["strict_env_force_native_sources"], "unset")
            self.assertEqual(
                manifest["strict_env_parser_core_codegen_escape_hatch"], "unset"
            )
            self.assertEqual(manifest["harness_status"], "1")
            self.assertEqual(manifest["lane_fragilec_build_status"], "0")
            self.assertEqual(manifest["lane_fragilec_test_rpc_status"], "0")
            self.assertEqual(manifest["lane_fragilec_completed_trials"], "2")
            self.assertEqual(manifest["lane_fragilec_failure_class"], "none")
            self.assertEqual(manifest["runtime_all_trials_passed"], "true")
            self.assertEqual(manifest["runtime_trial_passed_count"], "2")
            self.assertEqual(manifest["runtime_trial_failed_count"], "0")
            self.assertEqual(manifest["harness_no_regression_verdict"], "insufficient_data")
            self.assertEqual(manifest["comparison_no_regression_verdict"], "insufficient_data")
            self.assertEqual(manifest["missing_required_artifact_count"], "0")

            commands = (run_root / "strict_runtime_replay_commands.txt").read_text(
                encoding="utf-8"
            )
            self.assertIn(
                "strict_env=FRAGILEC_MODE=strict FRAGILEC_PARSER_BACKEND=fragile-parser-clang",
                commands,
            )
            self.assertIn("strict_env_force_native_sources=unset", commands)
            self.assertIn("--lanes fragilec", commands)

    def test_runtime_replay_rejects_lane_failure_contract(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root = tmp_path / "workspace"
            mako_root = workspace_root / "vendor" / "mako"
            mako_root.mkdir(parents=True, exist_ok=True)
            (mako_root / "CMakeLists.txt").write_text(
                "cmake_minimum_required(VERSION 3.16)\n",
                encoding="utf-8",
            )
            run_root = tmp_path / "run"
            harness_script = tmp_path / "fake_harness.py"
            self._write_fake_harness(harness_script)

            result = self._run_script(
                run_root=run_root,
                workspace_root=workspace_root,
                mako_root=mako_root,
                harness_script=harness_script,
                trials=1,
                extra_env={
                    "FAKE_HARNESS_EXIT": "1",
                    "FAKE_LANE_BUILD_STATUS": "2",
                    "FAKE_TEST_RPC_STATUS": "-1",
                    "FAKE_COMPLETED_TRIALS": "0",
                    "FAKE_FAILURE_CLASS": "build_failed",
                    "FAKE_RPC_SERVER_STATUS": "-1",
                    "FAKE_RPC_CLIENT_STATUS": "-1",
                },
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("lane contract failed", result.stderr)

    def test_runtime_replay_rejects_non_insufficient_data_nonzero_harness(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root = tmp_path / "workspace"
            mako_root = workspace_root / "vendor" / "mako"
            mako_root.mkdir(parents=True, exist_ok=True)
            (mako_root / "CMakeLists.txt").write_text(
                "cmake_minimum_required(VERSION 3.16)\n",
                encoding="utf-8",
            )
            run_root = tmp_path / "run"
            harness_script = tmp_path / "fake_harness.py"
            self._write_fake_harness(harness_script)

            result = self._run_script(
                run_root=run_root,
                workspace_root=workspace_root,
                mako_root=mako_root,
                harness_script=harness_script,
                trials=1,
                extra_env={
                    "FAKE_HARNESS_EXIT": "1",
                    "FAKE_NO_REGRESSION_VERDICT": "fail",
                    "FAKE_COMPARISON_VERDICT": "fail",
                },
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("without insufficient_data verdict", result.stderr)

    def test_force_native_sources_truthy_parent_env_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root = tmp_path / "workspace"
            mako_root = workspace_root / "vendor" / "mako"
            mako_root.mkdir(parents=True, exist_ok=True)
            (mako_root / "CMakeLists.txt").write_text(
                "cmake_minimum_required(VERSION 3.16)\n",
                encoding="utf-8",
            )
            run_root = tmp_path / "run"
            harness_script = tmp_path / "fake_harness.py"
            self._write_fake_harness(harness_script)

            result = self._run_script(
                run_root=run_root,
                workspace_root=workspace_root,
                mako_root=mako_root,
                harness_script=harness_script,
                trials=1,
                extra_env={"FRAGILEC_FORCE_NATIVE_SOURCES": "1"},
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("FRAGILEC_FORCE_NATIVE_SOURCES", result.stderr)


class ScriptDefaultConfigTests(unittest.TestCase):
    """Verify that orchestration scripts default to release fragilec binary
    and reasonable build timeouts for real-world mako builds."""

    def _read_script(self, name: str) -> str:
        return (REPO_ROOT / "scripts" / name).read_text(encoding="utf-8")

    def test_strict_runtime_replay_defaults_to_release_fragilec(self) -> None:
        src = self._read_script("mako_rpc_strict_runtime_replay.py")
        self.assertIn('"release" / "fragilec"', src)
        self.assertNotIn('"debug" / "fragilec"', src)

    def test_benchmark_comparison_defaults_to_release_fragilec(self) -> None:
        src = self._read_script("mako_rpc_benchmark_comparison.py")
        self.assertIn('"release" / "fragilec"', src)
        self.assertNotIn('"debug" / "fragilec"', src)

    def test_parser_shadow_defaults_to_release_fragilec(self) -> None:
        src = self._read_script("parser_shadow_non_rpc_corpus.py")
        self.assertIn('"release" / "fragilec"', src)
        self.assertNotIn('"debug" / "fragilec"', src)

    def test_harness_defaults_to_release_fragilec(self) -> None:
        src = self._read_script("mako_rpcbench_harness.py")
        self.assertIn('"release" / "fragilec"', src)
        self.assertNotIn('"debug" / "fragilec"', src)

    def test_strict_runtime_replay_build_timeout_at_least_3600(self) -> None:
        """Build timeout must be >= 3600s for mako builds with release fragilec."""
        src = self._read_script("mako_rpc_strict_runtime_replay.py")
        import re
        m = re.search(r"--build-timeout-seconds.*?default=(\d+)", src)
        self.assertIsNotNone(m, "build-timeout-seconds default not found")
        timeout = int(m.group(1))
        self.assertGreaterEqual(
            timeout, 3600,
            f"build timeout default {timeout}s is too low for mako; need >= 3600s"
        )

    def test_all_scripts_consistent_release_fragilec(self) -> None:
        """No orchestration script should default to debug fragilec binary."""
        scripts = [
            "mako_rpc_strict_runtime_replay.py",
            "mako_rpc_benchmark_comparison.py",
            "mako_rpcbench_harness.py",
            "parser_shadow_non_rpc_corpus.py",
        ]
        for name in scripts:
            src = self._read_script(name)
            self.assertNotIn(
                '"debug" / "fragilec"',
                src,
                f"{name} still defaults to debug fragilec",
            )


if __name__ == "__main__":
    unittest.main()
