import os
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

    def _run_harness(
        self,
        workspace: Path,
        mako_root: Path,
        run_root: Path,
        *,
        plan_only: bool = True,
        cmake_bin: Path | None = None,
        env: dict[str, str] | None = None,
        extra_args: list[str] | None = None,
    ):
        cmd = [
            "python3",
            str(SCRIPT_PATH),
            "--workspace-root",
            str(workspace),
            "--mako-root",
            str(mako_root),
            "--run-root",
            str(run_root),
            "--trials",
            "2",
            "--jobs",
            "4",
            "--base-port",
            "23000",
            "--fragile-cxx",
            str(workspace / "target" / "release" / "fragilec"),
        ]
        if plan_only:
            cmd.append("--plan-only")
        if cmake_bin is not None:
            cmd.extend(["--cmake-bin", str(cmake_bin)])
        if extra_args:
            cmd.extend(extra_args)
        merged_env = os.environ.copy()
        if env:
            merged_env.update(env)
        return subprocess.run(
            cmd,
            check=False,
            text=True,
            capture_output=True,
            env=merged_env,
        )

    def _create_fake_cmake(self, root: Path) -> Path:
        fake_cmake = root / "fake_cmake.sh"
        fake_cmake.write_text(
            "\n".join(
                [
                    "#!/usr/bin/env bash",
                    "set -euo pipefail",
                    "args=\"$*\"",
                    "echo \"fake-cmake ${args}\"",
                    "lane=\"clang\"",
                    "if [[ \"${args}\" == *\"build_fragilec\"* ]]; then lane=\"fragilec\"; fi",
                    "step=\"build\"",
                    "if [[ \"${args}\" == *\"--target clean\"* ]]; then",
                    "  step=\"clean\"",
                    "elif [[ \"${args}\" == *\" -S \"* ]] || [[ \"${args}\" == \"-S \"* ]]; then",
                    "  step=\"configure\"",
                    "fi",
                    "var_name=\"FAKE_${step^^}_${lane^^}_RC\"",
                    "rc=\"${!var_name:-0}\"",
                    "echo \"lane=${lane} step=${step} rc=${rc}\" >&2",
                    "exit \"${rc}\"",
                    "",
                ]
            ),
            encoding="utf-8",
        )
        fake_cmake.chmod(0o755)
        return fake_cmake

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

            result = self._run_harness(
                workspace,
                mako_root,
                run_root,
                extra_args=["--base-port", "100"],
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("base-port", result.stderr)

    def test_execution_mode_captures_configure_clean_build_for_both_lanes(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace, mako_root = self._create_workspace_fixture(tmp_path)
            run_root = tmp_path / "run"
            fake_cmake = self._create_fake_cmake(tmp_path)

            result = self._run_harness(
                workspace,
                mako_root,
                run_root,
                plan_only=False,
                cmake_bin=fake_cmake,
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            for lane in ("clang", "fragilec"):
                lane_dir = run_root / f"lane_{lane}"
                self.assertEqual((lane_dir / "configure.status").read_text(encoding="utf-8").strip(), "0")
                self.assertEqual((lane_dir / "clean.status").read_text(encoding="utf-8").strip(), "0")
                self.assertEqual((lane_dir / "build.status").read_text(encoding="utf-8").strip(), "0")
                self.assertEqual((lane_dir / "failure_class.txt").read_text(encoding="utf-8").strip(), "none")
                self.assertIn(
                    "fake-cmake",
                    (lane_dir / "configure.stdout").read_text(encoding="utf-8"),
                )
                self.assertIn(
                    f"lane={lane} step=build rc=0",
                    (lane_dir / "build.stderr").read_text(encoding="utf-8"),
                )

            manifest = (run_root / "benchmark_harness_manifest.txt").read_text(encoding="utf-8")
            self.assertIn("task_leaf=1.2", manifest)
            self.assertIn("plan_only=false", manifest)
            self.assertIn("lane_clang_failure_class=none", manifest)
            self.assertIn("lane_fragilec_failure_class=none", manifest)

    def test_execution_mode_records_failure_class_and_skips_followup_steps(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace, mako_root = self._create_workspace_fixture(tmp_path)
            run_root = tmp_path / "run"
            fake_cmake = self._create_fake_cmake(tmp_path)

            result = self._run_harness(
                workspace,
                mako_root,
                run_root,
                plan_only=False,
                cmake_bin=fake_cmake,
                env={"FAKE_CONFIGURE_FRAGILEC_RC": "5"},
            )
            self.assertNotEqual(result.returncode, 0)

            clang_dir = run_root / "lane_clang"
            self.assertEqual((clang_dir / "configure.status").read_text(encoding="utf-8").strip(), "0")
            self.assertEqual((clang_dir / "build.status").read_text(encoding="utf-8").strip(), "0")
            self.assertEqual((clang_dir / "failure_class.txt").read_text(encoding="utf-8").strip(), "none")

            fragile_dir = run_root / "lane_fragilec"
            self.assertEqual((fragile_dir / "configure.status").read_text(encoding="utf-8").strip(), "5")
            self.assertEqual((fragile_dir / "clean.status").read_text(encoding="utf-8").strip(), "-1")
            self.assertEqual((fragile_dir / "build.status").read_text(encoding="utf-8").strip(), "-1")
            self.assertEqual(
                (fragile_dir / "failure_class.txt").read_text(encoding="utf-8").strip(),
                "configure_failed",
            )
            self.assertIn(
                "skipped: configure step failed",
                (fragile_dir / "clean.stderr").read_text(encoding="utf-8"),
            )

            manifest = (run_root / "benchmark_harness_manifest.txt").read_text(encoding="utf-8")
            self.assertIn("lane_fragilec_failure_class=configure_failed", manifest)
            self.assertIn("lane_fragilec_build_status=-1", manifest)


if __name__ == "__main__":
    unittest.main()
