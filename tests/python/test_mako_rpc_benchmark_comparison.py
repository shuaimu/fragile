import os
import subprocess
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "mako_rpc_benchmark_comparison.py"


class MakoRpcBenchmarkComparisonTests(unittest.TestCase):
    def _write_fake_harness(self, path: Path) -> None:
        """Write a fake dual-lane harness that produces deterministic artifacts.

        Environment controls:
        - FAKE_CLANG_QPS: QPS for clang lane (default 1000.0)
        - FAKE_FRAGILEC_QPS: QPS for fragilec lane (default 1100.0)
        - FAKE_LANE_BUILD_STATUS: build status for both lanes (default 0)
        - FAKE_TEST_RPC_STATUS: test_rpc status for both lanes (default 0)
        - FAKE_FAILURE_CLASS: failure class for both lanes (default "none")
        - FAKE_HARNESS_EXIT: harness exit code (default 0)
        - FAKE_NO_REGRESSION_VERDICT: verdict string (auto-computed if not set)
        """
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
                    "parser.add_argument('--trials', required=True, type=int)",
                    "parser.add_argument('--lanes', default='clang,fragilec')",
                    "args, _ = parser.parse_known_args()",
                    "",
                    "run_root = Path(args.run_root)",
                    "run_root.mkdir(parents=True, exist_ok=True)",
                    "trials = args.trials",
                    "lanes = args.lanes.split(',')",
                    "",
                    "clang_qps = float(os.environ.get('FAKE_CLANG_QPS', '1000.0'))",
                    "fragilec_qps = float(os.environ.get('FAKE_FRAGILEC_QPS', '1100.0'))",
                    "build_status = os.environ.get('FAKE_LANE_BUILD_STATUS', '0')",
                    "test_rpc_status = os.environ.get('FAKE_TEST_RPC_STATUS', '0')",
                    "failure_class = os.environ.get('FAKE_FAILURE_CLASS', 'none')",
                    "",
                    "# Auto-compute verdict unless explicitly overridden",
                    "if 'FAKE_NO_REGRESSION_VERDICT' in os.environ:",
                    "    verdict = os.environ['FAKE_NO_REGRESSION_VERDICT']",
                    "elif build_status != '0' or test_rpc_status != '0':",
                    "    verdict = 'insufficient_data'",
                    "elif fragilec_qps >= clang_qps:",
                    "    verdict = 'pass'",
                    "else:",
                    "    verdict = 'fail'",
                    "",
                    "for lane in lanes:",
                    "    lane_dir = run_root / f'lane_{lane}'",
                    "    lane_dir.mkdir(parents=True, exist_ok=True)",
                    "    for step in ('configure', 'clean', 'build', 'test_rpc'):",
                    "        status = '0'",
                    "        if step == 'build': status = build_status",
                    "        if step == 'test_rpc': status = test_rpc_status",
                    "        (lane_dir / f'{step}.status').write_text(status + '\\n')",
                    "        (lane_dir / f'{step}.stdout').write_text('ok\\n')",
                    "        (lane_dir / f'{step}.stderr').write_text('\\n')",
                    "    (lane_dir / 'failure_class.txt').write_text(failure_class + '\\n')",
                    "",
                    "    qps = clang_qps if lane == 'clang' else fragilec_qps",
                    "    for trial in range(1, trials + 1):",
                    "        trial_dir = lane_dir / f'trial_{trial:02d}'",
                    "        trial_dir.mkdir(parents=True, exist_ok=True)",
                    "        (trial_dir / 'rpc_server.status').write_text('0\\n')",
                    "        (trial_dir / 'rpc_server.stdout').write_text('ok\\n')",
                    "        (trial_dir / 'rpc_server.stderr').write_text('\\n')",
                    "        (trial_dir / 'rpc_client.status').write_text('0\\n')",
                    "        (trial_dir / 'rpc_client.stdout').write_text(f'QPS: {qps}\\n')",
                    "        (trial_dir / 'rpc_client.stderr').write_text('\\n')",
                    "",
                    "delta = fragilec_qps - clang_qps",
                    "ratio = fragilec_qps / clang_qps if clang_qps else 0",
                    "",
                    "harness_lines = [",
                    "    'version=1', 'task_leaf=1.4',",
                    "    f'run_root={run_root}', 'plan_only=false',",
                    "    f'lanes={args.lanes}', 'build_only=false',",
                    "    f'trials={trials}',",
                    "    f'clang_avg_qps={clang_qps:.6f}',",
                    "    f'fragile_avg_qps={fragilec_qps:.6f}',",
                    "    f'fragile_minus_clang_qps={delta:.6f}',",
                    "    f'fragile_over_clang_ratio={ratio:.6f}',",
                    "    f'no_regression_verdict={verdict}',",
                    "]",
                    "for lane in lanes:",
                    "    qps = clang_qps if lane == 'clang' else fragilec_qps",
                    "    harness_lines.extend([",
                    "        f'lane_{lane}_configure_status=0',",
                    "        f'lane_{lane}_clean_status=0',",
                    "        f'lane_{lane}_build_status={build_status}',",
                    "        f'lane_{lane}_test_rpc_status={test_rpc_status}',",
                    "        f'lane_{lane}_completed_trials={trials}',",
                    "        f'lane_{lane}_avg_qps={qps:.6f}',",
                    "        f'lane_{lane}_failure_class={failure_class}',",
                    "    ])",
                    "    for trial in range(1, trials + 1):",
                    "        harness_lines.append(f'lane_{lane}_trial_{trial:02d}_qps={qps:.6f}')",
                    "",
                    "(run_root / 'benchmark_harness_manifest.txt').write_text(",
                    "    '\\n'.join(harness_lines) + '\\n')",
                    "",
                    "comparison_lines = [",
                    "    'version=1', 'task_leaf=1.4',",
                    "    f'run_root={run_root}', 'plan_only=false',",
                    "    f'trials={trials}',",
                    "    f'clang_avg_qps={clang_qps:.6f}',",
                    "    f'fragile_avg_qps={fragilec_qps:.6f}',",
                    "    f'fragile_minus_clang_qps={delta:.6f}',",
                    "    f'fragile_over_clang_ratio={ratio:.6f}',",
                    "    f'no_regression_verdict={verdict}',",
                    "]",
                    "for lane in lanes:",
                    "    qps = clang_qps if lane == 'clang' else fragilec_qps",
                    "    for trial in range(1, trials + 1):",
                    "        comparison_lines.append(f'lane_{lane}_trial_{trial:02d}_qps={qps:.6f}')",
                    "",
                    "(run_root / 'benchmark_qps_comparison_manifest.txt').write_text(",
                    "    '\\n'.join(comparison_lines) + '\\n')",
                    "(run_root / 'benchmark_harness_command_plan.txt').write_text('# fake\\n')",
                    "(run_root / 'benchmark_expected_artifacts.txt').write_text('# fake\\n')",
                    "",
                    "print(str(run_root))",
                    "exit_code = int(os.environ.get('FAKE_HARNESS_EXIT', '0' if verdict == 'pass' else '1'))",
                    "raise SystemExit(exit_code)",
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
        trials: int = 3,
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
            str(harness_script),  # reuse as fake binary
            "--skip-fragilec-build",
            "--trials",
            str(trials),
            "--jobs",
            "1",
            "--base-port",
            "24000",
            "--rpc-duration-seconds",
            "1",
        ]
        env = os.environ.copy()
        if extra_env:
            env.update(extra_env)
        return subprocess.run(cmd, check=False, text=True, capture_output=True, env=env)

    def _setup_workspace(self, tmp_path: Path) -> tuple[Path, Path, Path, Path]:
        """Create workspace, mako_root, run_root, fake_harness and return them."""
        workspace_root = tmp_path / "workspace"
        mako_root = workspace_root / "vendor" / "mako"
        mako_root.mkdir(parents=True, exist_ok=True)
        (mako_root / "CMakeLists.txt").write_text(
            "cmake_minimum_required(VERSION 3.16)\n", encoding="utf-8"
        )
        run_root = tmp_path / "run"
        harness_script = tmp_path / "fake_harness.py"
        self._write_fake_harness(harness_script)
        return workspace_root, mako_root, run_root, harness_script

    def test_benchmark_comparison_pass_verdict_when_fragile_faster(self) -> None:
        """Pass case: fragile QPS > clang QPS -> verdict=pass, all gates pass."""
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root, mako_root, run_root, harness_script = self._setup_workspace(tmp_path)

            result = self._run_script(
                run_root=run_root,
                workspace_root=workspace_root,
                mako_root=mako_root,
                harness_script=harness_script,
                trials=3,
                extra_env={
                    "FAKE_CLANG_QPS": "1000.0",
                    "FAKE_FRAGILEC_QPS": "1100.0",
                },
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            manifest = self._parse_manifest(
                run_root / "benchmark_comparison_manifest.txt"
            )
            self.assertEqual(manifest["task_leaf"], "M9.3")
            self.assertEqual(manifest["no_regression_verdict"], "pass")
            self.assertEqual(manifest["m9_a1_test_rpc_gate"], "pass")
            self.assertEqual(manifest["m9_a2_rpcbench_runtime_gate"], "pass")
            self.assertEqual(manifest["m9_a3_performance_gate"], "pass")
            self.assertEqual(manifest["lanes"], "clang,fragilec")
            self.assertEqual(manifest["strict_env_mode"], "strict")
            self.assertEqual(manifest["strict_env_parser_backend"], "fragile-parser-clang")
            self.assertEqual(manifest["strict_env_force_native_sources"], "unset")

    def test_benchmark_comparison_fail_verdict_when_regression_detected(self) -> None:
        """Fail case: fragile QPS < clang QPS -> verdict=fail, M9.A3 gate fails."""
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root, mako_root, run_root, harness_script = self._setup_workspace(tmp_path)

            result = self._run_script(
                run_root=run_root,
                workspace_root=workspace_root,
                mako_root=mako_root,
                harness_script=harness_script,
                trials=3,
                extra_env={
                    "FAKE_CLANG_QPS": "1200.0",
                    "FAKE_FRAGILEC_QPS": "900.0",
                },
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("M9.A3 gate failed", result.stderr)

    def test_benchmark_comparison_rejects_build_failure(self) -> None:
        """Both lanes must build; build failure -> harness nonzero -> script fails."""
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root, mako_root, run_root, harness_script = self._setup_workspace(tmp_path)

            result = self._run_script(
                run_root=run_root,
                workspace_root=workspace_root,
                mako_root=mako_root,
                harness_script=harness_script,
                trials=1,
                extra_env={
                    "FAKE_LANE_BUILD_STATUS": "2",
                    "FAKE_FAILURE_CLASS": "build_failed",
                    "FAKE_HARNESS_EXIT": "1",
                },
            )
            self.assertNotEqual(result.returncode, 0)

    def test_force_native_sources_truthy_parent_env_is_rejected(self) -> None:
        """FRAGILEC_FORCE_NATIVE_SOURCES must be rejected."""
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root, mako_root, run_root, harness_script = self._setup_workspace(tmp_path)

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

    def test_escape_hatch_env_is_rejected(self) -> None:
        """FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH must be rejected."""
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root, mako_root, run_root, harness_script = self._setup_workspace(tmp_path)

            result = self._run_script(
                run_root=run_root,
                workspace_root=workspace_root,
                mako_root=mako_root,
                harness_script=harness_script,
                trials=1,
                extra_env={"FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH": "libtooling"},
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH", result.stderr)


if __name__ == "__main__":
    unittest.main()
