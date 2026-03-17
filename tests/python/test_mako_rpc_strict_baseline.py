import os
import subprocess
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "mako_rpc_strict_baseline.py"


class MakoRpcStrictBaselineTests(unittest.TestCase):
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
                    "parser.add_argument('--lanes', required=True)",
                    "args, _ = parser.parse_known_args()",
                    "",
                    "run_root = Path(args.run_root)",
                    "run_root.mkdir(parents=True, exist_ok=True)",
                    "lanes = [lane for lane in args.lanes.split(',') if lane]",
                    "lines = ['version=1', f'lanes={args.lanes}']",
                    "for lane in lanes:",
                    "    if lane == 'fragilec':",
                    "        build_status = '124'",
                    "        test_rpc_status = '-1'",
                    "        failure_class = 'build_timeout'",
                    "    else:",
                    "        build_status = '0'",
                    "        test_rpc_status = '0'",
                    "        failure_class = 'none'",
                    "    lines.extend([",
                    "        f'lane_{lane}_configure_status=0',",
                    "        f'lane_{lane}_clean_status=0',",
                    "        f'lane_{lane}_build_status={build_status}',",
                    "        f'lane_{lane}_test_rpc_status={test_rpc_status}',",
                    "        f'lane_{lane}_failure_class={failure_class}',",
                    "    ])",
                    "(run_root / 'benchmark_harness_manifest.txt').write_text('\\n'.join(lines) + '\\n', encoding='utf-8')",
                    "print(run_root)",
                    "raise SystemExit(int(os.environ.get('FAKE_HARNESS_EXIT', '1')))",
                ]
            ),
            encoding="utf-8",
        )
        path.chmod(0o755)

    def _write_fake_inventory(self, path: Path) -> None:
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
                    "run_root = Path(args.run_root)",
                    "run_root.mkdir(parents=True, exist_ok=True)",
                    "lanes = [lane for lane in args.lanes.split(',') if lane]",
                    "lines = ['version=1', 'task_leaf=2.1', f'lanes={args.lanes}']",
                    "for lane in lanes:",
                    "    if lane == 'fragilec':",
                    "        blocker_class = 'build_timeout'",
                    "        blocker_file = 'src/rrr/base/misc.cpp'",
                    "        blocker_e0425 = '0'",
                    "    else:",
                    "        blocker_class = 'none'",
                    "        blocker_file = 'none'",
                    "        blocker_e0425 = '0'",
                    "    lines.extend([",
                    "        f'lane_{lane}_first_failing_compile_class={blocker_class}',",
                    "        f'lane_{lane}_first_failing_compile_file={blocker_file}',",
                    "        f'lane_{lane}_first_failing_compile_e0425_count={blocker_e0425}',",
                    "    ])",
                    "(run_root / 'rpc_compile_blocker_inventory_manifest.txt').write_text('\\n'.join(lines) + '\\n', encoding='utf-8')",
                    "print(run_root)",
                    "raise SystemExit(int(os.environ.get('FAKE_INVENTORY_EXIT', '0')))",
                ]
            ),
            encoding="utf-8",
        )
        path.chmod(0o755)

    def _write_fake_replay(self, path: Path) -> None:
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
                    "args, _ = parser.parse_known_args()",
                    "",
                    "run_root = Path(args.run_root)",
                    "run_root.mkdir(parents=True, exist_ok=True)",
                    "lines = [",
                    "    'version=1',",
                    "    'task_leaf=2.2',",
                    "    'selected_count=1',",
                    "    'replay_01_blocker_class=build_timeout',",
                    "    'replay_01_blocker_file=src/rrr/base/misc.cpp',",
                    "    'replay_01_status=124',",
                    "    'replay_01_timed_out=true',",
                    "    'replay_01_first_failure_class=build_timeout',",
                    "]",
                    "(run_root / 'rpc_compile_blocker_replay_manifest.txt').write_text('\\n'.join(lines) + '\\n', encoding='utf-8')",
                    "if os.environ.get('FAKE_REPLAY_WRITE_STAGE_TIMING', '1') == '1':",
                    "    stage_path = os.environ.get('FRAGILEC_TRANSPILE_STAGE_TIMING_PATH', '')",
                    "    if stage_path:",
                    "        Path(stage_path).write_text(",
                    "            '\\n'.join([",
                    "                'source=/tmp/mako/src/rpcbench.cpp',",
                    "                'last_stage_started=codegen',",
                    "                'last_stage_completed=enrichment',",
                    "                'parse_ms=11',",
                    "                'export_ms=22',",
                    "                'enrichment_ms=33',",
                    "                'codegen_ms=44',",
                    "                'total_ms=110',",
                    "                'status=error',",
                    "                'error=command timed out',",
                    "            ]) + '\\n',",
                    "            encoding='utf-8'",
                    "        )",
                    "print(run_root)",
                    "raise SystemExit(int(os.environ.get('FAKE_REPLAY_EXIT', '0')))",
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
        inventory_script: Path,
        replay_script: Path,
        lanes: str,
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
            "--lanes",
            lanes,
            "--jobs",
            "1",
            "--trials",
            "1",
            "--base-port",
            "23000",
            "--harness-script",
            str(harness_script),
            "--inventory-script",
            str(inventory_script),
            "--replay-script",
            str(replay_script),
            "--replay-timeout-seconds",
            "15",
            "--replay-max-replays",
            "1",
        ]
        env = os.environ.copy()
        if extra_env:
            env.update(extra_env)
        return subprocess.run(cmd, check=False, text=True, capture_output=True, env=env)

    def test_baseline_manifest_emitted_when_harness_is_nonzero(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root = tmp_path / "workspace"
            mako_root = workspace_root / "vendor" / "mako"
            mako_root.mkdir(parents=True, exist_ok=True)
            run_root = tmp_path / "run"
            harness_script = tmp_path / "fake_harness.py"
            inventory_script = tmp_path / "fake_inventory.py"
            replay_script = tmp_path / "fake_replay.py"
            self._write_fake_harness(harness_script)
            self._write_fake_inventory(inventory_script)
            self._write_fake_replay(replay_script)

            result = self._run_script(
                run_root=run_root,
                workspace_root=workspace_root,
                mako_root=mako_root,
                harness_script=harness_script,
                inventory_script=inventory_script,
                replay_script=replay_script,
                lanes="clang,fragilec",
                extra_env={"FAKE_HARNESS_EXIT": "1", "FAKE_REPLAY_WRITE_STAGE_TIMING": "1"},
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            manifest = self._parse_manifest(run_root / "strict_baseline_manifest.txt")
            self.assertEqual(manifest["task_leaf"], "M0.1")
            self.assertEqual(manifest["lanes"], "clang,fragilec")
            self.assertEqual(manifest["harness_status"], "1")
            self.assertEqual(manifest["inventory_status"], "0")
            self.assertEqual(manifest["replay_status"], "0")
            self.assertEqual(manifest["stage_timing_exists"], "true")
            self.assertEqual(manifest["stage_timing_status"], "error")
            self.assertEqual(manifest["stage_timing_last_stage_started"], "codegen")
            self.assertEqual(manifest["stage_timing_total_ms"], "110")
            self.assertEqual(manifest["lane_fragilec_build_status"], "124")
            self.assertEqual(manifest["lane_fragilec_failure_class"], "build_timeout")
            self.assertEqual(
                manifest["lane_fragilec_first_failing_compile_file"],
                "src/rrr/base/misc.cpp",
            )
            self.assertEqual(manifest["lane_clang_build_status"], "0")
            self.assertEqual(manifest["lane_clang_failure_class"], "none")
            self.assertEqual(manifest["replay_01_status"], "124")
            self.assertEqual(manifest["run_root_contract_version"], "1")
            self.assertEqual(manifest["run_root_name_is_contract_valid"], "false")
            self.assertEqual(manifest["required_artifact_count"], "14")
            self.assertEqual(manifest["missing_required_artifact_count"], "0")

            required_manifest_path = Path(
                manifest["required_artifact_contract_manifest"]
            )
            required_manifest = self._parse_manifest(required_manifest_path)
            self.assertEqual(required_manifest["task_leaf"], "M0.1")
            self.assertEqual(required_manifest["required_artifact_count"], "14")
            self.assertEqual(required_manifest["missing_required_artifact_count"], "0")
            self.assertEqual(
                required_manifest["required_artifact_014_relpath"],
                "strict_baseline_manifest.txt",
            )
            comparable_manifest_path = Path(manifest["comparable_manifest"])
            self.assertTrue(comparable_manifest_path.exists())
            comparable_manifest = self._parse_manifest(comparable_manifest_path)
            self.assertNotIn("run_root", comparable_manifest)
            self.assertNotIn("stage_timing_path", comparable_manifest)
            self.assertNotIn("required_artifact_contract_manifest", comparable_manifest)
            self.assertEqual(
                manifest["comparable_manifest_key_count"],
                str(len(comparable_manifest)),
            )
            self.assertIn("run_root", manifest["non_comparable_keys"])
            self.assertIn("stage_timing_total_ms", manifest["non_comparable_keys"])

            commands = (run_root / "strict_baseline_commands.txt").read_text(
                encoding="utf-8"
            )
            self.assertIn("strict_env=FRAGILEC_MODE=strict", commands)
            self.assertIn("replay_stage_timing_path=", commands)
            self.assertIn("harness_command=", commands)
            self.assertIn("inventory_command=", commands)
            self.assertIn("replay_command=", commands)

    def test_missing_stage_timing_file_is_recorded_as_none(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root = tmp_path / "workspace"
            mako_root = workspace_root / "vendor" / "mako"
            mako_root.mkdir(parents=True, exist_ok=True)
            run_root = tmp_path / "run"
            harness_script = tmp_path / "fake_harness.py"
            inventory_script = tmp_path / "fake_inventory.py"
            replay_script = tmp_path / "fake_replay.py"
            self._write_fake_harness(harness_script)
            self._write_fake_inventory(inventory_script)
            self._write_fake_replay(replay_script)

            result = self._run_script(
                run_root=run_root,
                workspace_root=workspace_root,
                mako_root=mako_root,
                harness_script=harness_script,
                inventory_script=inventory_script,
                replay_script=replay_script,
                lanes="fragilec",
                extra_env={"FAKE_REPLAY_WRITE_STAGE_TIMING": "0"},
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            manifest = self._parse_manifest(run_root / "strict_baseline_manifest.txt")
            self.assertEqual(manifest["stage_timing_exists"], "false")
            self.assertEqual(manifest["stage_timing_status"], "none")
            self.assertEqual(manifest["stage_timing_total_ms"], "none")
            self.assertEqual(manifest["missing_required_artifact_count"], "0")

    def test_consecutive_runs_emit_identical_comparable_manifests(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root = tmp_path / "workspace"
            mako_root = workspace_root / "vendor" / "mako"
            mako_root.mkdir(parents=True, exist_ok=True)
            harness_script = tmp_path / "fake_harness.py"
            inventory_script = tmp_path / "fake_inventory.py"
            replay_script = tmp_path / "fake_replay.py"
            self._write_fake_harness(harness_script)
            self._write_fake_inventory(inventory_script)
            self._write_fake_replay(replay_script)

            run_one = tmp_path / "run_one"
            run_two = tmp_path / "run_two"
            result_one = self._run_script(
                run_root=run_one,
                workspace_root=workspace_root,
                mako_root=mako_root,
                harness_script=harness_script,
                inventory_script=inventory_script,
                replay_script=replay_script,
                lanes="fragilec",
                extra_env={"FAKE_REPLAY_WRITE_STAGE_TIMING": "1"},
            )
            self.assertEqual(result_one.returncode, 0, msg=result_one.stderr)
            result_two = self._run_script(
                run_root=run_two,
                workspace_root=workspace_root,
                mako_root=mako_root,
                harness_script=harness_script,
                inventory_script=inventory_script,
                replay_script=replay_script,
                lanes="fragilec",
                extra_env={"FAKE_REPLAY_WRITE_STAGE_TIMING": "1"},
            )
            self.assertEqual(result_two.returncode, 0, msg=result_two.stderr)

            manifest_one = self._parse_manifest(run_one / "strict_baseline_manifest.txt")
            manifest_two = self._parse_manifest(run_two / "strict_baseline_manifest.txt")
            comp_one = Path(manifest_one["comparable_manifest"])
            comp_two = Path(manifest_two["comparable_manifest"])
            self.assertEqual(
                comp_one.read_text(encoding="utf-8"),
                comp_two.read_text(encoding="utf-8"),
            )
            self.assertEqual(
                manifest_one["comparable_manifest_sha256"],
                manifest_two["comparable_manifest_sha256"],
            )
            self.assertEqual(
                manifest_one["comparable_manifest_key_count"],
                manifest_two["comparable_manifest_key_count"],
            )

    def test_inventory_failure_causes_nonzero_exit(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root = tmp_path / "workspace"
            mako_root = workspace_root / "vendor" / "mako"
            mako_root.mkdir(parents=True, exist_ok=True)
            run_root = tmp_path / "run"
            harness_script = tmp_path / "fake_harness.py"
            inventory_script = tmp_path / "fake_inventory.py"
            replay_script = tmp_path / "fake_replay.py"
            self._write_fake_harness(harness_script)
            self._write_fake_inventory(inventory_script)
            self._write_fake_replay(replay_script)

            result = self._run_script(
                run_root=run_root,
                workspace_root=workspace_root,
                mako_root=mako_root,
                harness_script=harness_script,
                inventory_script=inventory_script,
                replay_script=replay_script,
                lanes="fragilec",
                extra_env={"FAKE_INVENTORY_EXIT": "2"},
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("inventory command failed", result.stderr)


if __name__ == "__main__":
    unittest.main()
