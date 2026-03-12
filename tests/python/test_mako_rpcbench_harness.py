import subprocess
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "mako_rpcbench_harness.py"


class MakoRpcBenchHarnessPlanTests(unittest.TestCase):
    def _create_workspace_fixture(self, root: Path) -> tuple[Path, Path]:
        workspace = root / "workspace"
        mako_root = workspace / "vendor" / "mako"
        mako_root.mkdir(parents=True, exist_ok=True)
        (mako_root / "CMakeLists.txt").write_text("cmake_minimum_required(VERSION 3.16)\n", encoding="utf-8")
        (workspace / "target" / "release").mkdir(parents=True, exist_ok=True)
        return workspace, mako_root

    def _run_harness(self, workspace: Path, mako_root: Path, run_root: Path, *extra_args: str):
        cmd = [
            "python3",
            str(SCRIPT_PATH),
            "--workspace-root",
            str(workspace),
            "--mako-root",
            str(mako_root),
            "--run-root",
            str(run_root),
            "--plan-only",
            "--trials",
            "2",
            "--jobs",
            "4",
            "--base-port",
            "23000",
            "--fragile-cxx",
            str(workspace / "target" / "release" / "fragilec"),
            *extra_args,
        ]
        return subprocess.run(cmd, check=False, text=True, capture_output=True)

    def test_plan_files_and_artifact_contract_are_emitted(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace, mako_root = self._create_workspace_fixture(tmp_path)
            run_root = tmp_path / "run"

            result = self._run_harness(workspace, mako_root, run_root)
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            manifest_path = run_root / "benchmark_harness_manifest.txt"
            plan_path = run_root / "benchmark_harness_command_plan.txt"
            expected_artifacts_path = run_root / "benchmark_expected_artifacts.txt"
            self.assertTrue(manifest_path.exists())
            self.assertTrue(plan_path.exists())
            self.assertTrue(expected_artifacts_path.exists())

            manifest = manifest_path.read_text(encoding="utf-8")
            plan = plan_path.read_text(encoding="utf-8")
            expected_artifacts = expected_artifacts_path.read_text(encoding="utf-8")

            self.assertIn("task_leaf=1.1", manifest)
            self.assertIn("lanes=clang,fragilec", manifest)
            self.assertIn("lane_clang_trial_01_port=23000", manifest)
            self.assertIn("lane_clang_trial_02_port=23001", manifest)
            self.assertIn("lane_fragilec_trial_01_port=23100", manifest)
            self.assertIn("lane_fragilec_trial_02_port=23101", manifest)

            self.assertIn("[lane:clang]", plan)
            self.assertIn("[lane:fragilec]", plan)
            self.assertIn("--target test_rpc rpcbench masstree_perf", plan)
            self.assertIn("trial_01_port=23000", plan)
            self.assertIn("trial_01_port=23100", plan)

            self.assertIn("lane_clang/configure.status", expected_artifacts)
            self.assertIn("lane_fragilec/build.stderr", expected_artifacts)
            self.assertIn("lane_clang/trial_02/rpc_client.stdout", expected_artifacts)
            self.assertIn("lane_fragilec/trial_01/rpc_server.stderr", expected_artifacts)

    def test_plan_generation_is_idempotent_for_same_inputs(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace, mako_root = self._create_workspace_fixture(tmp_path)
            run_root = tmp_path / "run"

            first = self._run_harness(workspace, mako_root, run_root)
            self.assertEqual(first.returncode, 0, msg=first.stderr)
            first_manifest = (run_root / "benchmark_harness_manifest.txt").read_text(encoding="utf-8")
            first_plan = (run_root / "benchmark_harness_command_plan.txt").read_text(encoding="utf-8")
            first_expected = (run_root / "benchmark_expected_artifacts.txt").read_text(encoding="utf-8")

            second = self._run_harness(workspace, mako_root, run_root)
            self.assertEqual(second.returncode, 0, msg=second.stderr)
            second_manifest = (run_root / "benchmark_harness_manifest.txt").read_text(encoding="utf-8")
            second_plan = (run_root / "benchmark_harness_command_plan.txt").read_text(encoding="utf-8")
            second_expected = (run_root / "benchmark_expected_artifacts.txt").read_text(encoding="utf-8")

            self.assertEqual(first_manifest, second_manifest)
            self.assertEqual(first_plan, second_plan)
            self.assertEqual(first_expected, second_expected)

    def test_invalid_base_port_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace, mako_root = self._create_workspace_fixture(tmp_path)
            run_root = tmp_path / "run"

            result = self._run_harness(workspace, mako_root, run_root, "--base-port", "100")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("base-port", result.stderr)


if __name__ == "__main__":
    unittest.main()
