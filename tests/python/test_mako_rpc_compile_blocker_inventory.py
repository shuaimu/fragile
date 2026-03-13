import subprocess
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "mako_rpc_compile_blocker_inventory.py"


class MakoRpcCompileBlockerInventoryTests(unittest.TestCase):
    def _write_lane_build_artifacts(
        self,
        run_root: Path,
        lane: str,
        *,
        build_status: int,
        build_stderr: str,
    ) -> None:
        lane_dir = run_root / f"lane_{lane}"
        lane_dir.mkdir(parents=True, exist_ok=True)
        (lane_dir / "build.status").write_text(f"{build_status}\n", encoding="utf-8")
        (lane_dir / "build.stderr").write_text(build_stderr, encoding="utf-8")

    def _run_inventory(
        self,
        run_root: Path,
        *,
        lanes: str | None = None,
        baseline_manifest: Path | None = None,
        enforce_nonincreasing: bool = False,
    ) -> subprocess.CompletedProcess[str]:
        cmd = [
            "python3",
            str(SCRIPT_PATH),
            "--run-root",
            str(run_root),
        ]
        if lanes is not None:
            cmd.extend(["--lanes", lanes])
        if baseline_manifest is not None:
            cmd.extend(["--baseline-manifest", str(baseline_manifest)])
        if enforce_nonincreasing:
            cmd.append("--enforce-nonincreasing")
        return subprocess.run(cmd, check=False, text=True, capture_output=True)

    def _parse_key_values(self, path: Path) -> dict[str, str]:
        pairs: dict[str, str] = {}
        for line in path.read_text(encoding="utf-8").splitlines():
            if "=" not in line:
                continue
            key, value = line.split("=", 1)
            pairs[key.strip()] = value.strip()
        return pairs

    def test_inventory_extracts_e0425_blocker_class_and_first_failing_file(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            run_root = Path(tmp) / "run"
            run_root.mkdir(parents=True, exist_ok=True)
            self._write_lane_build_artifacts(
                run_root,
                "clang",
                build_status=0,
                build_stderr="",
            )
            self._write_lane_build_artifacts(
                run_root,
                "fragilec",
                build_status=2,
                build_stderr="\n".join(
                    [
                        "[fragilec] fragile rustc object compile failed for /tmp/mako/src/rpcbench.cpp",
                        "error[E0425]: cannot find value `rpc` in this scope",
                        "error[E0425]: cannot find function `bench` in this scope",
                    ]
                ),
            )

            result = self._run_inventory(run_root)
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            fragile_lane_dir = run_root / "lane_fragilec"
            self.assertEqual(
                (fragile_lane_dir / "first_failing_compile_class.txt").read_text(encoding="utf-8").strip(),
                "unresolved_name_or_type_e0425",
            )
            self.assertEqual(
                (fragile_lane_dir / "first_failing_compile_file.txt").read_text(encoding="utf-8").strip(),
                "/tmp/mako/src/rpcbench.cpp",
            )
            self.assertEqual(
                (fragile_lane_dir / "first_failing_compile_e0425_count.txt").read_text(encoding="utf-8").strip(),
                "2",
            )

            manifest = self._parse_key_values(run_root / "rpc_compile_blocker_inventory_manifest.txt")
            self.assertEqual(manifest["task_leaf"], "2.1")
            self.assertEqual(manifest["lanes"], "clang,fragilec")
            self.assertEqual(manifest["lane_fragilec_build_status"], "2")
            self.assertEqual(
                manifest["lane_fragilec_first_failing_compile_class"],
                "unresolved_name_or_type_e0425",
            )

    def test_inventory_sets_none_file_and_zero_count_for_skipped_or_success_build(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            run_root = Path(tmp) / "run"
            run_root.mkdir(parents=True, exist_ok=True)
            self._write_lane_build_artifacts(
                run_root,
                "clang",
                build_status=0,
                build_stderr="error[E0425]: should be ignored due to successful build",
            )
            self._write_lane_build_artifacts(
                run_root,
                "fragilec",
                build_status=-1,
                build_stderr="\n".join(
                    [
                        "[fragilec] fragile rustc object compile failed for /tmp/mako/src/test_rpc.cpp",
                        "error[E0425]: should be ignored due to skipped build",
                    ]
                ),
            )

            result = self._run_inventory(run_root)
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            clang_lane_dir = run_root / "lane_clang"
            fragile_lane_dir = run_root / "lane_fragilec"
            self.assertEqual(
                (clang_lane_dir / "first_failing_compile_class.txt").read_text(encoding="utf-8").strip(),
                "none",
            )
            self.assertEqual(
                (clang_lane_dir / "first_failing_compile_file.txt").read_text(encoding="utf-8").strip(),
                "none",
            )
            self.assertEqual(
                (clang_lane_dir / "first_failing_compile_e0425_count.txt").read_text(encoding="utf-8").strip(),
                "0",
            )
            self.assertEqual(
                (fragile_lane_dir / "first_failing_compile_class.txt").read_text(encoding="utf-8").strip(),
                "build_not_executed",
            )
            self.assertEqual(
                (fragile_lane_dir / "first_failing_compile_file.txt").read_text(encoding="utf-8").strip(),
                "none",
            )
            self.assertEqual(
                (fragile_lane_dir / "first_failing_compile_e0425_count.txt").read_text(encoding="utf-8").strip(),
                "0",
            )

    def test_inventory_classifies_transpile_failure_and_extracts_file(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            run_root = Path(tmp) / "run"
            run_root.mkdir(parents=True, exist_ok=True)
            self._write_lane_build_artifacts(
                run_root,
                "fragilec",
                build_status=1,
                build_stderr="\n".join(
                    [
                        "[fragilec] failed to transpile /tmp/mako/src/test_rpc.cpp with parser backend libtooling",
                        "error: parse failure",
                    ]
                ),
            )

            result = self._run_inventory(run_root, lanes="fragilec")
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            fragile_lane_dir = run_root / "lane_fragilec"
            self.assertEqual(
                (fragile_lane_dir / "first_failing_compile_class.txt").read_text(encoding="utf-8").strip(),
                "transpile_failure",
            )
            self.assertEqual(
                (fragile_lane_dir / "first_failing_compile_file.txt").read_text(encoding="utf-8").strip(),
                "/tmp/mako/src/test_rpc.cpp",
            )
            self.assertEqual(
                (fragile_lane_dir / "first_failing_compile_e0425_count.txt").read_text(encoding="utf-8").strip(),
                "0",
            )

            manifest = self._parse_key_values(run_root / "rpc_compile_blocker_inventory_manifest.txt")
            self.assertEqual(manifest["lanes"], "fragilec")
            self.assertEqual(
                manifest["lane_fragilec_first_failing_compile_class"],
                "transpile_failure",
            )

    def test_inventory_classifies_missing_method_rustc_error_family(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            run_root = Path(tmp) / "run"
            run_root.mkdir(parents=True, exist_ok=True)
            self._write_lane_build_artifacts(
                run_root,
                "fragilec",
                build_status=1,
                build_stderr="\n".join(
                    [
                        "[fragilec] fragile rustc object compile failed for /tmp/mako/src/client.cpp",
                        "error[E0599]: no method named `push` found for struct `Sender`",
                    ]
                ),
            )

            result = self._run_inventory(run_root, lanes="fragilec")
            self.assertEqual(result.returncode, 0, msg=result.stderr)
            self.assertEqual(
                (run_root / "lane_fragilec" / "first_failing_compile_class.txt")
                .read_text(encoding="utf-8")
                .strip(),
                "missing_method_e0599",
            )

    def test_inventory_fails_when_required_lane_artifact_is_missing(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            run_root = Path(tmp) / "run"
            lane_dir = run_root / "lane_clang"
            lane_dir.mkdir(parents=True, exist_ok=True)
            (lane_dir / "build.status").write_text("0\n", encoding="utf-8")

            result = self._run_inventory(run_root, lanes="clang")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("missing build stderr artifact", result.stderr)

    def test_inventory_nonincrease_gate_passes_for_better_or_equal_baseline(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            baseline_run_root = tmp_path / "baseline_run"
            baseline_run_root.mkdir(parents=True, exist_ok=True)
            self._write_lane_build_artifacts(
                baseline_run_root,
                "clang",
                build_status=0,
                build_stderr="",
            )
            self._write_lane_build_artifacts(
                baseline_run_root,
                "fragilec",
                build_status=2,
                build_stderr="\n".join(
                    [
                        "[fragilec] fragile rustc object compile failed for /tmp/mako/src/rpcbench.cpp",
                        "error[E0425]: cannot find value `rpc` in this scope",
                        "error[E0425]: cannot find value `bench` in this scope",
                    ]
                ),
            )
            baseline_result = self._run_inventory(baseline_run_root)
            self.assertEqual(baseline_result.returncode, 0, msg=baseline_result.stderr)
            baseline_manifest = (
                baseline_run_root / "rpc_compile_blocker_inventory_manifest.txt"
            )

            current_run_root = tmp_path / "current_run"
            current_run_root.mkdir(parents=True, exist_ok=True)
            self._write_lane_build_artifacts(
                current_run_root,
                "clang",
                build_status=0,
                build_stderr="",
            )
            self._write_lane_build_artifacts(
                current_run_root,
                "fragilec",
                build_status=1,
                build_stderr="\n".join(
                    [
                        "[fragilec] failed to transpile /tmp/mako/src/rpcbench.cpp with parser backend libtooling",
                        "error: parse failure",
                    ]
                ),
            )
            current_result = self._run_inventory(
                current_run_root,
                baseline_manifest=baseline_manifest,
                enforce_nonincreasing=True,
            )
            self.assertEqual(current_result.returncode, 0, msg=current_result.stderr)
            manifest = self._parse_key_values(
                current_run_root / "rpc_compile_blocker_inventory_manifest.txt"
            )
            self.assertEqual(manifest["task_leaf"], "2.5")
            self.assertEqual(manifest["nonincrease_gate_pass"], "true")
            self.assertEqual(manifest["lane_fragilec_nonincrease_gate_pass"], "true")
            self.assertEqual(
                manifest["lane_fragilec_class_nonworsening_vs_baseline"],
                "true",
            )
            self.assertEqual(
                manifest["lane_fragilec_e0425_nonincrease_vs_baseline"],
                "true",
            )

    def test_inventory_nonincrease_gate_fails_when_class_severity_worsens(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            baseline_run_root = tmp_path / "baseline_run"
            baseline_run_root.mkdir(parents=True, exist_ok=True)
            self._write_lane_build_artifacts(
                baseline_run_root,
                "fragilec",
                build_status=0,
                build_stderr="",
            )
            baseline_result = self._run_inventory(baseline_run_root, lanes="fragilec")
            self.assertEqual(baseline_result.returncode, 0, msg=baseline_result.stderr)
            baseline_manifest = (
                baseline_run_root / "rpc_compile_blocker_inventory_manifest.txt"
            )

            current_run_root = tmp_path / "current_run"
            current_run_root.mkdir(parents=True, exist_ok=True)
            self._write_lane_build_artifacts(
                current_run_root,
                "fragilec",
                build_status=2,
                build_stderr="\n".join(
                    [
                        "[fragilec] fragile rustc object compile failed for /tmp/mako/src/rpcbench.cpp",
                        "error[E0425]: cannot find value `rpc` in this scope",
                    ]
                ),
            )
            current_result = self._run_inventory(
                current_run_root,
                lanes="fragilec",
                baseline_manifest=baseline_manifest,
                enforce_nonincreasing=True,
            )
            self.assertNotEqual(current_result.returncode, 0)
            self.assertIn("nonincrease gate failed", current_result.stderr)
            manifest = self._parse_key_values(
                current_run_root / "rpc_compile_blocker_inventory_manifest.txt"
            )
            self.assertEqual(manifest["nonincrease_gate_pass"], "false")
            self.assertEqual(manifest["lane_fragilec_nonincrease_gate_pass"], "false")
            self.assertEqual(
                manifest["lane_fragilec_class_nonworsening_vs_baseline"],
                "false",
            )

    def test_inventory_nonincrease_gate_fails_when_e0425_count_increases(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            baseline_run_root = tmp_path / "baseline_run"
            baseline_run_root.mkdir(parents=True, exist_ok=True)
            self._write_lane_build_artifacts(
                baseline_run_root,
                "fragilec",
                build_status=2,
                build_stderr="\n".join(
                    [
                        "[fragilec] fragile rustc object compile failed for /tmp/mako/src/rpcbench.cpp",
                        "error[E0425]: cannot find value `rpc` in this scope",
                    ]
                ),
            )
            baseline_result = self._run_inventory(baseline_run_root, lanes="fragilec")
            self.assertEqual(baseline_result.returncode, 0, msg=baseline_result.stderr)
            baseline_manifest = (
                baseline_run_root / "rpc_compile_blocker_inventory_manifest.txt"
            )

            current_run_root = tmp_path / "current_run"
            current_run_root.mkdir(parents=True, exist_ok=True)
            self._write_lane_build_artifacts(
                current_run_root,
                "fragilec",
                build_status=2,
                build_stderr="\n".join(
                    [
                        "[fragilec] fragile rustc object compile failed for /tmp/mako/src/rpcbench.cpp",
                        "error[E0425]: cannot find value `rpc` in this scope",
                        "error[E0425]: cannot find function `bench` in this scope",
                    ]
                ),
            )
            current_result = self._run_inventory(
                current_run_root,
                lanes="fragilec",
                baseline_manifest=baseline_manifest,
                enforce_nonincreasing=True,
            )
            self.assertNotEqual(current_result.returncode, 0)
            manifest = self._parse_key_values(
                current_run_root / "rpc_compile_blocker_inventory_manifest.txt"
            )
            self.assertEqual(manifest["nonincrease_gate_pass"], "false")
            self.assertEqual(manifest["lane_fragilec_nonincrease_gate_pass"], "false")
            self.assertEqual(
                manifest["lane_fragilec_e0425_nonincrease_vs_baseline"],
                "false",
            )
            self.assertEqual(manifest["lane_fragilec_e0425_delta_vs_baseline"], "1")

    def test_inventory_nonincrease_gate_fails_for_missing_baseline_keys(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            baseline_manifest = tmp_path / "baseline_manifest.txt"
            baseline_manifest.write_text(
                "\n".join(
                    [
                        "version=1",
                        "task_leaf=2.1",
                        "lanes=fragilec",
                        "lane_fragilec_build_status=2",
                    ]
                )
                + "\n",
                encoding="utf-8",
            )

            current_run_root = tmp_path / "current_run"
            current_run_root.mkdir(parents=True, exist_ok=True)
            self._write_lane_build_artifacts(
                current_run_root,
                "fragilec",
                build_status=2,
                build_stderr="\n".join(
                    [
                        "[fragilec] fragile rustc object compile failed for /tmp/mako/src/rpcbench.cpp",
                        "error[E0425]: cannot find value `rpc` in this scope",
                    ]
                ),
            )

            current_result = self._run_inventory(
                current_run_root,
                lanes="fragilec",
                baseline_manifest=baseline_manifest,
                enforce_nonincreasing=True,
            )
            self.assertNotEqual(current_result.returncode, 0)
            self.assertIn("missing baseline manifest key", current_result.stderr)


if __name__ == "__main__":
    unittest.main()
