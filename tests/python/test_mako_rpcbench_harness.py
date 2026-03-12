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
            "--test-rpc-timeout-seconds",
            "5",
            "--rpc-client-timeout-seconds",
            "5",
            "--rpc-server-startup-wait-seconds",
            "0.01",
            "--rpc-server-shutdown-timeout-seconds",
            "2",
            "--rpc-duration-seconds",
            "1",
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
                    "lane_upper=\"${lane^^}\"",
                    "step=\"build\"",
                    "if [[ \"${args}\" == *\"--target clean\"* ]]; then",
                    "  step=\"clean\"",
                    "elif [[ \"${args}\" == *\" -S \"* ]] || [[ \"${args}\" == \"-S \"* ]]; then",
                    "  step=\"configure\"",
                    "fi",
                    "build_dir=\"\"",
                    "for ((i=1; i<=$#; i++)); do",
                    "  token=\"${!i}\"",
                    "  if [[ \"${token}\" == \"-B\" ]] || [[ \"${token}\" == \"--build\" ]]; then",
                    "    next_index=$((i + 1))",
                    "    build_dir=\"${!next_index:-}\"",
                    "  fi",
                    "done",
                    "var_name=\"FAKE_${step^^}_${lane_upper}_RC\"",
                    "rc=\"${!var_name:-0}\"",
                    "if [[ \"${step}\" == \"build\" ]] && [[ \"${rc}\" == \"0\" ]]; then",
                    "  mkdir -p \"${build_dir}\"",
                    "  cat > \"${build_dir}/test_rpc\" <<EOF",
                    "#!/usr/bin/env bash",
                    "set -euo pipefail",
                    "sleep_s=\"\\${FAKE_TEST_RPC_${lane_upper}_SLEEP_SECONDS:-0}\"",
                    "if [[ \"\\${sleep_s}\" != \"0\" ]]; then sleep \"\\${sleep_s}\"; fi",
                    "rc=\"\\${FAKE_TEST_RPC_${lane_upper}_RC:-0}\"",
                    "echo \"fake-test-rpc lane=${lane} rc=\\${rc}\"",
                    "exit \"\\${rc}\"",
                    "EOF",
                    "  cat > \"${build_dir}/rpcbench\" <<EOF",
                    "#!/usr/bin/env bash",
                    "set -euo pipefail",
                    "mode=\"\\${1:-}\"",
                    "if [[ \"\\${mode}\" == \"-s\" ]]; then",
                    "  if [[ \"\\${FAKE_RPC_SERVER_${lane_upper}_EXIT_IMMEDIATE:-0}\" == \"1\" ]]; then",
                    "    echo \"fake-rpc-server lane=${lane} immediate-exit\"",
                    "    exit 3",
                    "  fi",
                    "  trap 'exit 0' TERM INT",
                    "  echo \"fake-rpc-server lane=${lane} started\"",
                    "  while true; do sleep 1; done",
                    "fi",
                    "endpoint=\"\"",
                    "for ((i=1; i<=$#; i++)); do",
                    "  token=\"\\${!i}\"",
                    "  if [[ \"\\${token}\" == \"-c\" ]]; then",
                    "    next_index=$((i + 1))",
                    "    endpoint=\"\\${!next_index:-}\"",
                    "  fi",
                    "done",
                    "port=\"\\${endpoint##*:}\"",
                    "sleep_s=\"\\${FAKE_RPC_CLIENT_${lane_upper}_SLEEP_SECONDS:-0}\"",
                    "if [[ \"\\${sleep_s}\" != \"0\" ]]; then sleep \"\\${sleep_s}\"; fi",
                    "rc=\"\\${FAKE_RPC_CLIENT_${lane_upper}_RC:-0}\"",
                    "qps_var=\"FAKE_RPC_CLIENT_${lane_upper}_PORT_\\${port}_QPS\"",
                    "qps=\"\\${!qps_var:-\\${FAKE_RPC_CLIENT_${lane_upper}_QPS:-1000}}\"",
                    "emit_qps=\"\\${FAKE_RPC_CLIENT_${lane_upper}_EMIT_QPS:-1}\"",
                    "echo \"fake-rpc-client lane=${lane} rc=\\${rc}\"",
                    "if [[ \"\\${emit_qps}\" == \"1\" ]]; then",
                    "  echo \"summary qps=\\${qps}\"",
                    "fi",
                    "exit \"\\${rc}\"",
                    "EOF",
                    "  chmod +x \"${build_dir}/test_rpc\" \"${build_dir}/rpcbench\"",
                    "fi",
                    "echo \"lane=${lane} step=${step} rc=${rc}\" >&2",
                    "exit \"${rc}\"",
                    "",
                ]
            ),
            encoding="utf-8",
        )
        fake_cmake.chmod(0o755)
        return fake_cmake

    def _parse_key_value_file(self, path: Path) -> dict[str, str]:
        pairs: dict[str, str] = {}
        for line in path.read_text(encoding="utf-8").splitlines():
            if "=" not in line:
                continue
            key, value = line.split("=", 1)
            pairs[key.strip()] = value.strip()
        return pairs

    def _assert_expected_artifacts_exist(self, run_root: Path) -> None:
        expected_paths = (
            run_root / "benchmark_expected_artifacts.txt"
        ).read_text(encoding="utf-8").splitlines()
        for rel_path in expected_paths:
            self.assertTrue((run_root / rel_path).exists(), msg=f"missing artifact: {rel_path}")

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
            comparison_manifest_path = run_root / "benchmark_qps_comparison_manifest.txt"
            self.assertTrue(manifest_path.exists())
            self.assertTrue(plan_path.exists())
            self.assertTrue(expected_artifacts_path.exists())
            self.assertTrue(comparison_manifest_path.exists())

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
            self.assertIn("benchmark_qps_comparison_manifest.txt", expected_artifacts)

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
                env={
                    "FAKE_RPC_CLIENT_CLANG_QPS": "1000",
                    "FAKE_RPC_CLIENT_FRAGILEC_QPS": "1200",
                },
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            for lane in ("clang", "fragilec"):
                lane_dir = run_root / f"lane_{lane}"
                self.assertEqual((lane_dir / "configure.status").read_text(encoding="utf-8").strip(), "0")
                self.assertEqual((lane_dir / "clean.status").read_text(encoding="utf-8").strip(), "0")
                self.assertEqual((lane_dir / "build.status").read_text(encoding="utf-8").strip(), "0")
                self.assertEqual((lane_dir / "test_rpc.status").read_text(encoding="utf-8").strip(), "0")
                self.assertEqual((lane_dir / "failure_class.txt").read_text(encoding="utf-8").strip(), "none")
                self.assertIn(
                    "fake-cmake",
                    (lane_dir / "configure.stdout").read_text(encoding="utf-8"),
                )
                self.assertIn(
                    f"lane={lane} step=build rc=0",
                    (lane_dir / "build.stderr").read_text(encoding="utf-8"),
                )
                self.assertIn(
                    f"fake-test-rpc lane={lane} rc=0",
                    (lane_dir / "test_rpc.stdout").read_text(encoding="utf-8"),
                )
                for trial in (1, 2):
                    trial_dir = lane_dir / f"trial_{trial:02d}"
                    self.assertEqual(
                        (trial_dir / "rpc_server.status").read_text(encoding="utf-8").strip(),
                        "0",
                    )
                    self.assertEqual(
                        (trial_dir / "rpc_client.status").read_text(encoding="utf-8").strip(),
                        "0",
                    )
                    self.assertIn(
                        f"fake-rpc-client lane={lane} rc=0",
                        (trial_dir / "rpc_client.stdout").read_text(encoding="utf-8"),
                    )
                    expected_qps = "1000" if lane == "clang" else "1200"
                    self.assertIn(
                        f"summary qps={expected_qps}",
                        (trial_dir / "rpc_client.stdout").read_text(encoding="utf-8"),
                    )

            manifest = (run_root / "benchmark_harness_manifest.txt").read_text(encoding="utf-8")
            self.assertIn("task_leaf=1.4", manifest)
            self.assertIn("plan_only=false", manifest)
            self.assertIn("lane_clang_failure_class=none", manifest)
            self.assertIn("lane_fragilec_failure_class=none", manifest)
            self.assertIn("lane_clang_test_rpc_status=0", manifest)
            self.assertIn("lane_fragilec_test_rpc_status=0", manifest)
            self.assertIn("lane_clang_completed_trials=2", manifest)
            self.assertIn("lane_fragilec_completed_trials=2", manifest)
            self.assertIn("lane_clang_trial_01_qps=1000.000000", manifest)
            self.assertIn("lane_fragilec_trial_02_qps=1200.000000", manifest)
            self.assertIn("lane_clang_avg_qps=1000.000000", manifest)
            self.assertIn("lane_fragilec_avg_qps=1200.000000", manifest)
            self.assertIn("clang_avg_qps=1000.000000", manifest)
            self.assertIn("fragile_avg_qps=1200.000000", manifest)
            self.assertIn("no_regression_verdict=pass", manifest)

            comparison_manifest = (
                run_root / "benchmark_qps_comparison_manifest.txt"
            ).read_text(encoding="utf-8")
            self.assertIn("no_regression_verdict=pass", comparison_manifest)
            self.assertIn("lane_clang_trial_01_qps=1000.000000", comparison_manifest)
            self.assertIn("lane_fragilec_trial_02_qps=1200.000000", comparison_manifest)

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
            self.assertEqual((fragile_dir / "test_rpc.status").read_text(encoding="utf-8").strip(), "-1")
            self.assertEqual(
                (fragile_dir / "failure_class.txt").read_text(encoding="utf-8").strip(),
                "configure_failed",
            )
            self.assertIn(
                "skipped: configure step failed",
                (fragile_dir / "clean.stderr").read_text(encoding="utf-8"),
            )
            self.assertIn(
                "skipped: build step failed",
                (fragile_dir / "test_rpc.stderr").read_text(encoding="utf-8"),
            )
            fragile_trial = fragile_dir / "trial_01"
            self.assertEqual(
                (fragile_trial / "rpc_server.status").read_text(encoding="utf-8").strip(),
                "-1",
            )
            self.assertEqual(
                (fragile_trial / "rpc_client.status").read_text(encoding="utf-8").strip(),
                "-1",
            )

            manifest = (run_root / "benchmark_harness_manifest.txt").read_text(encoding="utf-8")
            self.assertIn("lane_fragilec_failure_class=configure_failed", manifest)
            self.assertIn("lane_fragilec_build_status=-1", manifest)
            self.assertIn("lane_fragilec_test_rpc_status=-1", manifest)
            self.assertIn("lane_fragilec_completed_trials=0", manifest)
            self.assertIn("no_regression_verdict=insufficient_data", manifest)

    def test_execution_mode_records_test_rpc_failure_and_skips_trials(self):
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
                env={"FAKE_TEST_RPC_FRAGILEC_RC": "7"},
            )
            self.assertNotEqual(result.returncode, 0)

            fragile_dir = run_root / "lane_fragilec"
            self.assertEqual((fragile_dir / "build.status").read_text(encoding="utf-8").strip(), "0")
            self.assertEqual((fragile_dir / "test_rpc.status").read_text(encoding="utf-8").strip(), "7")
            self.assertEqual(
                (fragile_dir / "failure_class.txt").read_text(encoding="utf-8").strip(),
                "test_rpc_failed",
            )
            self.assertIn(
                "fake-test-rpc lane=fragilec rc=7",
                (fragile_dir / "test_rpc.stdout").read_text(encoding="utf-8"),
            )

            for trial in (1, 2):
                trial_dir = fragile_dir / f"trial_{trial:02d}"
                self.assertEqual(
                    (trial_dir / "rpc_server.status").read_text(encoding="utf-8").strip(),
                    "-1",
                )
                self.assertEqual(
                    (trial_dir / "rpc_client.status").read_text(encoding="utf-8").strip(),
                    "-1",
                )
                self.assertIn(
                    "skipped: test_rpc step failed",
                    (trial_dir / "rpc_client.stderr").read_text(encoding="utf-8"),
                )

            manifest = (run_root / "benchmark_harness_manifest.txt").read_text(encoding="utf-8")
            self.assertIn("lane_fragilec_failure_class=test_rpc_failed", manifest)
            self.assertIn("lane_fragilec_test_rpc_status=7", manifest)
            self.assertIn("lane_fragilec_completed_trials=0", manifest)
            self.assertIn("no_regression_verdict=insufficient_data", manifest)

    def test_execution_mode_records_first_runtime_trial_failure_class(self):
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
                env={"FAKE_RPC_CLIENT_CLANG_RC": "9"},
            )
            self.assertNotEqual(result.returncode, 0)

            clang_dir = run_root / "lane_clang"
            self.assertEqual((clang_dir / "test_rpc.status").read_text(encoding="utf-8").strip(), "0")
            self.assertEqual(
                (clang_dir / "failure_class.txt").read_text(encoding="utf-8").strip(),
                "rpc_trial_01_rpc_client_failed",
            )
            self.assertEqual(
                (clang_dir / "trial_01" / "rpc_client.status").read_text(encoding="utf-8").strip(),
                "9",
            )
            self.assertEqual(
                (clang_dir / "trial_02" / "rpc_client.status").read_text(encoding="utf-8").strip(),
                "9",
            )

            manifest = (run_root / "benchmark_harness_manifest.txt").read_text(encoding="utf-8")
            self.assertIn("lane_clang_failure_class=rpc_trial_01_rpc_client_failed", manifest)
            self.assertIn("lane_clang_completed_trials=0", manifest)
            self.assertIn("no_regression_verdict=insufficient_data", manifest)

    def test_execution_mode_fails_when_fragile_qps_regresses(self):
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
                env={
                    "FAKE_RPC_CLIENT_CLANG_QPS": "2000",
                    "FAKE_RPC_CLIENT_FRAGILEC_QPS": "1500",
                },
            )
            self.assertNotEqual(result.returncode, 0)

            for lane in ("clang", "fragilec"):
                lane_dir = run_root / f"lane_{lane}"
                self.assertEqual(
                    (lane_dir / "failure_class.txt").read_text(encoding="utf-8").strip(),
                    "none",
                )

            manifest = (run_root / "benchmark_harness_manifest.txt").read_text(encoding="utf-8")
            self.assertIn("clang_avg_qps=2000.000000", manifest)
            self.assertIn("fragile_avg_qps=1500.000000", manifest)
            self.assertIn("no_regression_verdict=fail", manifest)

            comparison_manifest = (
                run_root / "benchmark_qps_comparison_manifest.txt"
            ).read_text(encoding="utf-8")
            self.assertIn("no_regression_verdict=fail", comparison_manifest)

    def test_execution_mode_fails_with_insufficient_qps_data(self):
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
                env={
                    "FAKE_RPC_CLIENT_CLANG_EMIT_QPS": "0",
                    "FAKE_RPC_CLIENT_FRAGILEC_EMIT_QPS": "0",
                },
            )
            self.assertNotEqual(result.returncode, 0)

            for lane in ("clang", "fragilec"):
                lane_dir = run_root / f"lane_{lane}"
                self.assertEqual(
                    (lane_dir / "failure_class.txt").read_text(encoding="utf-8").strip(),
                    "none",
                )

            comparison_manifest = (
                run_root / "benchmark_qps_comparison_manifest.txt"
            ).read_text(encoding="utf-8")
            self.assertIn("clang_avg_qps=none", comparison_manifest)
            self.assertIn("fragile_avg_qps=none", comparison_manifest)
            self.assertIn("no_regression_verdict=insufficient_data", comparison_manifest)

    def test_regression_gate_local_fixture_asserts_full_leaf_1_1_to_1_4_contract(self):
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
                env={
                    "FAKE_RPC_CLIENT_CLANG_QPS": "1000",
                    "FAKE_RPC_CLIENT_FRAGILEC_QPS": "1200",
                },
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            self._assert_expected_artifacts_exist(run_root)

            manifest = self._parse_key_value_file(run_root / "benchmark_harness_manifest.txt")
            comparison = self._parse_key_value_file(
                run_root / "benchmark_qps_comparison_manifest.txt"
            )
            plan = (run_root / "benchmark_harness_command_plan.txt").read_text(encoding="utf-8")

            self.assertEqual(manifest["task_leaf"], "1.4")
            self.assertEqual(manifest["plan_only"], "false")
            self.assertEqual(manifest["comparison_manifest_file"], "benchmark_qps_comparison_manifest.txt")
            self.assertEqual(manifest["no_regression_verdict"], "pass")
            self.assertEqual(manifest["lane_clang_failure_class"], "none")
            self.assertEqual(manifest["lane_fragilec_failure_class"], "none")
            self.assertEqual(manifest["lane_clang_trial_01_qps"], "1000.000000")
            self.assertEqual(manifest["lane_clang_trial_02_qps"], "1000.000000")
            self.assertEqual(manifest["lane_fragilec_trial_01_qps"], "1200.000000")
            self.assertEqual(manifest["lane_fragilec_trial_02_qps"], "1200.000000")
            self.assertEqual(manifest["lane_clang_avg_qps"], "1000.000000")
            self.assertEqual(manifest["lane_fragilec_avg_qps"], "1200.000000")
            self.assertEqual(manifest["clang_avg_qps"], "1000.000000")
            self.assertEqual(manifest["fragile_avg_qps"], "1200.000000")
            self.assertEqual(manifest["fragile_minus_clang_qps"], "200.000000")

            self.assertEqual(comparison["task_leaf"], "1.4")
            self.assertEqual(comparison["no_regression_verdict"], "pass")
            self.assertEqual(comparison["lane_clang_trial_01_qps"], "1000.000000")
            self.assertEqual(comparison["lane_fragilec_trial_02_qps"], "1200.000000")

            self.assertIn("[lane:clang]", plan)
            self.assertIn("[lane:fragilec]", plan)
            self.assertIn("test_rpc=", plan)
            self.assertIn("trial_01_server=", plan)
            self.assertIn("trial_01_client=", plan)

    @unittest.skipUnless(
        os.environ.get("FRAGILE_RUN_REAL_WORLD_RPCBENCH_HARNESS") == "1",
        "set FRAGILE_RUN_REAL_WORLD_RPCBENCH_HARNESS=1 to run real-world replay gate",
    )
    def test_regression_gate_real_world_replay_emits_required_artifacts_and_manifests(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root = REPO_ROOT
            mako_root = REPO_ROOT / "vendor" / "mako"
            run_root = tmp_path / "real_world_run"

            result = self._run_harness(
                workspace_root,
                mako_root,
                run_root,
                plan_only=False,
                extra_args=[
                    "--trials",
                    "1",
                    "--jobs",
                    "4",
                    "--rpc-duration-seconds",
                    "1",
                    "--test-rpc-timeout-seconds",
                    "30",
                    "--rpc-client-timeout-seconds",
                    "30",
                    "--rpc-server-startup-wait-seconds",
                    "0.5",
                    "--rpc-server-shutdown-timeout-seconds",
                    "10",
                ],
            )
            self.assertNotEqual(result.stdout.strip(), "")
            self.assertTrue(run_root.exists())

            self._assert_expected_artifacts_exist(run_root)
            manifest = self._parse_key_value_file(run_root / "benchmark_harness_manifest.txt")
            comparison = self._parse_key_value_file(
                run_root / "benchmark_qps_comparison_manifest.txt"
            )
            self.assertEqual(manifest["task_leaf"], "1.4")
            self.assertEqual(manifest["comparison_manifest_file"], "benchmark_qps_comparison_manifest.txt")
            self.assertIn(manifest["no_regression_verdict"], {"pass", "fail", "insufficient_data"})
            self.assertIn(comparison["no_regression_verdict"], {"pass", "fail", "insufficient_data"})
            self.assertEqual(manifest["no_regression_verdict"], comparison["no_regression_verdict"])


if __name__ == "__main__":
    unittest.main()
