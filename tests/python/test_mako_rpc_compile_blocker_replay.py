import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "mako_rpc_compile_blocker_replay.py"


class MakoRpcCompileBlockerReplayTests(unittest.TestCase):
    def _write_inventory_manifest(
        self,
        run_root: Path,
        *,
        lanes: str,
        lane_entries: dict[str, dict[str, str | int]],
    ) -> None:
        lines = [
            "version=1",
            "task_leaf=2.1",
            f"run_root={run_root}",
            f"lanes={lanes}",
        ]
        for lane in [value.strip() for value in lanes.split(",") if value.strip()]:
            entry = lane_entries[lane]
            lines.extend(
                [
                    f"lane_{lane}_build_status={entry['build_status']}",
                    f"lane_{lane}_first_failing_compile_class={entry['blocker_class']}",
                    f"lane_{lane}_first_failing_compile_file={entry['blocker_file']}",
                    f"lane_{lane}_first_failing_compile_e0425_count={entry['e0425_count']}",
                ]
            )
        (run_root / "rpc_compile_blocker_inventory_manifest.txt").write_text(
            "\n".join(lines) + "\n", encoding="utf-8"
        )

    def _write_harness_manifest(
        self,
        run_root: Path,
        *,
        workspace_root: Path,
        mako_root: Path | None = None,
        clang_cxx: str,
        fragile_cxx: str,
    ) -> None:
        lines = [
            "version=1",
            "task_leaf=1.4",
            f"run_root={run_root}",
            f"workspace_root={workspace_root}",
            f"mako_root={mako_root if mako_root is not None else workspace_root}",
            f"clang_cxx={clang_cxx}",
            f"fragile_cxx={fragile_cxx}",
        ]
        (run_root / "benchmark_harness_manifest.txt").write_text(
            "\n".join(lines) + "\n", encoding="utf-8"
        )

    def _create_fake_compiler(self, root: Path) -> Path:
        fake = root / "fake_compiler.sh"
        fake.write_text(
            "\n".join(
                [
                    "#!/usr/bin/env bash",
                    "set -euo pipefail",
                    "out=''",
                    "for ((i=1; i<=$#; i++)); do",
                    "  token=\"${!i}\"",
                    "  if [[ \"${token}\" == \"-o\" ]]; then",
                    "    next_index=$((i + 1))",
                    "    out=\"${!next_index:-}\"",
                    "  fi",
                    "done",
                    "if [[ -n \"${out}\" ]]; then",
                    "  mkdir -p \"$(dirname \"${out}\")\"",
                    "  : > \"${out}\"",
                    "fi",
                    "if [[ -n \"${FAKE_REPLAY_STDERR:-}\" ]]; then",
                    "  echo \"${FAKE_REPLAY_STDERR}\" >&2",
                    "fi",
                    "if [[ -n \"${FAKE_REPLAY_STDOUT:-}\" ]]; then",
                    "  echo \"${FAKE_REPLAY_STDOUT}\"",
                    "fi",
                    "exit \"${FAKE_REPLAY_RC:-0}\"",
                ]
            )
            + "\n",
            encoding="utf-8",
        )
        fake.chmod(0o755)
        return fake

    def _run_script(
        self,
        run_root: Path,
        *,
        lanes: str | None = None,
        max_replays: int = 1,
        timeout_seconds: int = 30,
        env: dict[str, str] | None = None,
    ) -> subprocess.CompletedProcess[str]:
        cmd = [
            "python3",
            str(SCRIPT_PATH),
            "--run-root",
            str(run_root),
            "--max-replays",
            str(max_replays),
            "--timeout-seconds",
            str(timeout_seconds),
        ]
        if lanes is not None:
            cmd.extend(["--lanes", lanes])
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

    def _parse_key_values(self, path: Path) -> dict[str, str]:
        pairs: dict[str, str] = {}
        for line in path.read_text(encoding="utf-8").splitlines():
            if "=" not in line:
                continue
            key, value = line.split("=", 1)
            pairs[key.strip()] = value.strip()
        return pairs

    def test_replay_selects_top_ranked_blocker_and_uses_compile_commands(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            run_root = tmp_path / "run"
            run_root.mkdir(parents=True, exist_ok=True)
            fake_compiler = self._create_fake_compiler(tmp_path)

            clang_file = tmp_path / "src" / "test_rpc.cpp"
            fragile_file = tmp_path / "src" / "rpcbench.cpp"
            clang_file.parent.mkdir(parents=True, exist_ok=True)
            clang_file.write_text("int x = 0;\n", encoding="utf-8")
            fragile_file.write_text("int y = 0;\n", encoding="utf-8")

            self._write_inventory_manifest(
                run_root,
                lanes="clang,fragilec",
                lane_entries={
                    "clang": {
                        "build_status": 1,
                        "blocker_class": "missing_method_e0599",
                        "blocker_file": str(clang_file),
                        "e0425_count": 0,
                    },
                    "fragilec": {
                        "build_status": 1,
                        "blocker_class": "unresolved_name_or_type_e0425",
                        "blocker_file": str(fragile_file),
                        "e0425_count": 3,
                    },
                },
            )
            self._write_harness_manifest(
                run_root,
                workspace_root=tmp_path,
                clang_cxx=str(fake_compiler),
                fragile_cxx=str(fake_compiler),
            )
            build_fragile = run_root / "build_fragilec"
            build_fragile.mkdir(parents=True, exist_ok=True)
            (build_fragile / "compile_commands.json").write_text(
                json.dumps(
                    [
                        {
                            "directory": str(tmp_path),
                            "arguments": [
                                str(fake_compiler),
                                "-c",
                                str(fragile_file),
                                "-o",
                                str(tmp_path / "obj" / "fragile.o"),
                            ],
                            "file": str(fragile_file),
                        }
                    ]
                ),
                encoding="utf-8",
            )

            result = self._run_script(
                run_root,
                max_replays=1,
                env={
                    "FAKE_REPLAY_RC": "1",
                    "FAKE_REPLAY_STDERR": "error[E0425]: cannot find value `rpc` in this scope",
                },
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            manifest = self._parse_key_values(
                run_root / "rpc_compile_blocker_replay_manifest.txt"
            )
            self.assertEqual(manifest["selected_count"], "1")
            self.assertEqual(manifest["replay_01_lane"], "fragilec")
            self.assertEqual(manifest["replay_01_command_source"], "compile_commands")
            self.assertEqual(
                manifest["replay_01_first_failure_class"],
                "unresolved_name_or_type_e0425",
            )
            self.assertEqual(manifest["replay_01_status"], "1")
            command_text = (run_root / "replay_01" / "command.txt").read_text(
                encoding="utf-8"
            )
            self.assertIn(str(fake_compiler), command_text)
            self.assertIn(str(fragile_file), command_text)

    def test_replay_falls_back_to_lane_compiler_when_compile_db_missing(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            run_root = tmp_path / "run"
            run_root.mkdir(parents=True, exist_ok=True)
            fake_compiler = self._create_fake_compiler(tmp_path)

            blocker_file = tmp_path / "src" / "client.cpp"
            blocker_file.parent.mkdir(parents=True, exist_ok=True)
            blocker_file.write_text("int z = 0;\n", encoding="utf-8")

            self._write_inventory_manifest(
                run_root,
                lanes="fragilec",
                lane_entries={
                    "fragilec": {
                        "build_status": 2,
                        "blocker_class": "other_build_failure",
                        "blocker_file": str(blocker_file),
                        "e0425_count": 0,
                    }
                },
            )
            self._write_harness_manifest(
                run_root,
                workspace_root=tmp_path,
                clang_cxx=str(fake_compiler),
                fragile_cxx=str(fake_compiler),
            )

            result = self._run_script(
                run_root,
                lanes="fragilec",
                env={
                    "FAKE_REPLAY_RC": "1",
                    "FAKE_REPLAY_STDERR": "error[E0308]: mismatched types",
                },
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            manifest = self._parse_key_values(
                run_root / "rpc_compile_blocker_replay_manifest.txt"
            )
            self.assertEqual(manifest["replay_01_command_source"], "fallback_compiler")
            self.assertEqual(manifest["replay_01_first_failure_class"], "type_mismatch_e0308")
            command_text = (run_root / "replay_01" / "command.txt").read_text(
                encoding="utf-8"
            )
            self.assertIn(str(fake_compiler), command_text)
            self.assertIn("-std=gnu++17", command_text)
            self.assertIn(str(blocker_file), command_text)

    def test_replay_writes_zero_selection_manifest_when_no_candidates(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            run_root = tmp_path / "run"
            run_root.mkdir(parents=True, exist_ok=True)
            fake_compiler = self._create_fake_compiler(tmp_path)

            self._write_inventory_manifest(
                run_root,
                lanes="clang,fragilec",
                lane_entries={
                    "clang": {
                        "build_status": 0,
                        "blocker_class": "none",
                        "blocker_file": "none",
                        "e0425_count": 0,
                    },
                    "fragilec": {
                        "build_status": -1,
                        "blocker_class": "build_not_executed",
                        "blocker_file": "none",
                        "e0425_count": 0,
                    },
                },
            )
            self._write_harness_manifest(
                run_root,
                workspace_root=tmp_path,
                clang_cxx=str(fake_compiler),
                fragile_cxx=str(fake_compiler),
            )

            result = self._run_script(run_root)
            self.assertEqual(result.returncode, 0, msg=result.stderr)
            manifest = self._parse_key_values(
                run_root / "rpc_compile_blocker_replay_manifest.txt"
            )
            self.assertEqual(manifest["selected_count"], "0")
            self.assertFalse((run_root / "replay_01").exists())

    def test_replay_timeout_derived_relative_blocker_uses_compile_db_suffix_match(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root = tmp_path / "workspace"
            mako_root = tmp_path / "mako"
            run_root = tmp_path / "run"
            run_root.mkdir(parents=True, exist_ok=True)
            workspace_root.mkdir(parents=True, exist_ok=True)
            fake_compiler = self._create_fake_compiler(tmp_path)

            blocker_file = "src/rrr/base/misc.cpp"
            compile_source = mako_root / blocker_file
            compile_source.parent.mkdir(parents=True, exist_ok=True)
            compile_source.write_text("int misc = 0;\n", encoding="utf-8")

            self._write_inventory_manifest(
                run_root,
                lanes="fragilec",
                lane_entries={
                    "fragilec": {
                        "build_status": 124,
                        "blocker_class": "build_timeout",
                        "blocker_file": blocker_file,
                        "e0425_count": 0,
                    }
                },
            )
            self._write_harness_manifest(
                run_root,
                workspace_root=workspace_root,
                mako_root=mako_root,
                clang_cxx=str(fake_compiler),
                fragile_cxx=str(fake_compiler),
            )

            build_fragile = run_root / "build_fragilec"
            build_fragile.mkdir(parents=True, exist_ok=True)
            (build_fragile / "compile_commands.json").write_text(
                json.dumps(
                    [
                        {
                            "directory": str(mako_root),
                            "arguments": [
                                str(fake_compiler),
                                "-c",
                                str(compile_source),
                                "-o",
                                str(tmp_path / "obj" / "misc.o"),
                            ],
                            "file": str(compile_source),
                        }
                    ]
                ),
                encoding="utf-8",
            )

            result = self._run_script(
                run_root,
                lanes="fragilec",
                env={
                    "FAKE_REPLAY_RC": "1",
                    "FAKE_REPLAY_STDERR": "error[E0425]: cannot find value `rpc` in this scope",
                },
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            manifest = self._parse_key_values(
                run_root / "rpc_compile_blocker_replay_manifest.txt"
            )
            self.assertEqual(manifest["replay_01_command_source"], "compile_commands")
            self.assertEqual(
                manifest["replay_01_first_failure_class"],
                "unresolved_name_or_type_e0425",
            )
            command_text = (run_root / "replay_01" / "command.txt").read_text(
                encoding="utf-8"
            )
            self.assertIn(str(compile_source), command_text)

    def test_replay_timeout_derived_relative_blocker_fallback_prefers_mako_source(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            workspace_root = tmp_path / "workspace"
            mako_root = tmp_path / "mako"
            run_root = tmp_path / "run"
            run_root.mkdir(parents=True, exist_ok=True)
            workspace_root.mkdir(parents=True, exist_ok=True)
            fake_compiler = self._create_fake_compiler(tmp_path)

            blocker_file = "src/rrr/base/misc.cpp"
            compile_source = mako_root / blocker_file
            compile_source.parent.mkdir(parents=True, exist_ok=True)
            compile_source.write_text("int misc = 0;\n", encoding="utf-8")

            self._write_inventory_manifest(
                run_root,
                lanes="fragilec",
                lane_entries={
                    "fragilec": {
                        "build_status": 124,
                        "blocker_class": "build_timeout",
                        "blocker_file": blocker_file,
                        "e0425_count": 0,
                    }
                },
            )
            self._write_harness_manifest(
                run_root,
                workspace_root=workspace_root,
                mako_root=mako_root,
                clang_cxx=str(fake_compiler),
                fragile_cxx=str(fake_compiler),
            )

            result = self._run_script(
                run_root,
                lanes="fragilec",
                env={
                    "FAKE_REPLAY_RC": "1",
                    "FAKE_REPLAY_STDERR": "error[E0308]: mismatched types",
                },
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            manifest = self._parse_key_values(
                run_root / "rpc_compile_blocker_replay_manifest.txt"
            )
            self.assertEqual(manifest["replay_01_command_source"], "fallback_compiler")
            self.assertEqual(manifest["replay_01_first_failure_class"], "type_mismatch_e0308")
            command_text = (run_root / "replay_01" / "command.txt").read_text(
                encoding="utf-8"
            )
            self.assertIn(str(compile_source), command_text)

    def test_replay_fails_when_inventory_manifest_missing(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            run_root = Path(tmp) / "run"
            run_root.mkdir(parents=True, exist_ok=True)
            result = self._run_script(run_root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("missing inventory manifest artifact", result.stderr)


if __name__ == "__main__":
    unittest.main()
