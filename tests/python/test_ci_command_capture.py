import subprocess
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "ci_command_capture.py"


class CiCommandCaptureTests(unittest.TestCase):
    def _run_script(
        self,
        run_root: Path,
        *,
        name: str,
        inactivity_timeout_seconds: int,
        wall_timeout_seconds: int,
        command: list[str],
        run_timeout_seconds: float | None = None,
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                "python3",
                str(SCRIPT_PATH),
                "--run-root",
                str(run_root),
                "--name",
                name,
                "--inactivity-timeout-seconds",
                str(inactivity_timeout_seconds),
                "--wall-timeout-seconds",
                str(wall_timeout_seconds),
                "--command",
                *command,
            ],
            check=False,
            text=True,
            capture_output=True,
            timeout=run_timeout_seconds,
        )

    def _parse_manifest(self, manifest_path: Path) -> dict[str, str]:
        values: dict[str, str] = {}
        for line in manifest_path.read_text(encoding="utf-8").splitlines():
            if "=" not in line:
                continue
            key, value = line.split("=", 1)
            values[key.strip()] = value.strip()
        return values

    def test_success_command_writes_status_and_logs(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            run_root = Path(tmp) / "run"
            result = self._run_script(
                run_root,
                name="success",
                inactivity_timeout_seconds=5,
                wall_timeout_seconds=5,
                command=[
                    "python3",
                    "-c",
                    "import sys; print('hello'); print('warn', file=sys.stderr)",
                ],
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            status = (run_root / "success.status").read_text(encoding="utf-8").strip()
            self.assertEqual(status, "0")
            stdout_log = (run_root / "success.stdout.log").read_text(encoding="utf-8")
            stderr_log = (run_root / "success.stderr.log").read_text(encoding="utf-8")
            self.assertIn("hello", stdout_log)
            self.assertIn("warn", stderr_log)

            manifest = self._parse_manifest(run_root / "success.manifest.txt")
            self.assertEqual(manifest["status"], "0")
            self.assertEqual(manifest["timed_out"], "false")
            self.assertEqual(manifest["timeout_reason"], "none")

    def test_inactivity_timeout_kills_stalled_command(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            run_root = Path(tmp) / "run"
            result = self._run_script(
                run_root,
                name="inactivity",
                inactivity_timeout_seconds=1,
                wall_timeout_seconds=10,
                command=[
                    "python3",
                    "-c",
                    (
                        "import time; "
                        "print('start', flush=True); "
                        "time.sleep(2); "
                        "print('end', flush=True)"
                    ),
                ],
            )
            self.assertEqual(result.returncode, 124, msg=result.stderr)

            status = (
                run_root / "inactivity.status"
            ).read_text(encoding="utf-8").strip()
            self.assertEqual(status, "124")
            stdout_log = (run_root / "inactivity.stdout.log").read_text(
                encoding="utf-8"
            )
            stderr_log = (run_root / "inactivity.stderr.log").read_text(
                encoding="utf-8"
            )
            self.assertIn("start", stdout_log)
            self.assertNotIn("end", stdout_log)
            self.assertIn("inactivity_timeout", stderr_log)

            manifest = self._parse_manifest(run_root / "inactivity.manifest.txt")
            self.assertEqual(manifest["status"], "124")
            self.assertEqual(manifest["timed_out"], "true")
            self.assertEqual(manifest["timeout_reason"], "inactivity_timeout")

    def test_wall_timeout_preempts_long_running_command(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            run_root = Path(tmp) / "run"
            result = self._run_script(
                run_root,
                name="wall",
                inactivity_timeout_seconds=10,
                wall_timeout_seconds=1,
                command=[
                    "python3",
                    "-c",
                    "import time; print('start', flush=True); time.sleep(3)",
                ],
            )
            self.assertEqual(result.returncode, 124, msg=result.stderr)
            manifest = self._parse_manifest(run_root / "wall.manifest.txt")
            self.assertEqual(manifest["status"], "124")
            self.assertEqual(manifest["timed_out"], "true")
            self.assertEqual(manifest["timeout_reason"], "wall_timeout")

    def test_command_not_found_reports_127(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            run_root = Path(tmp) / "run"
            result = self._run_script(
                run_root,
                name="not_found",
                inactivity_timeout_seconds=5,
                wall_timeout_seconds=5,
                command=["definitely_not_a_real_command_xyz"],
            )
            self.assertEqual(result.returncode, 127)
            manifest = self._parse_manifest(run_root / "not_found.manifest.txt")
            self.assertEqual(manifest["status"], "127")
            self.assertEqual(manifest["timed_out"], "false")
            self.assertEqual(manifest["timeout_reason"], "none")
            stderr_log = (run_root / "not_found.stderr.log").read_text(
                encoding="utf-8"
            )
            self.assertIn("command not found", stderr_log)

    def test_background_descendant_inherited_stdio_does_not_block_exit(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            run_root = Path(tmp) / "run"
            result = self._run_script(
                run_root,
                name="background_stdio",
                inactivity_timeout_seconds=5,
                wall_timeout_seconds=30,
                command=[
                    "python3",
                    "-c",
                    (
                        "import subprocess; "
                        "subprocess.Popen(['python3', '-c', 'import time; time.sleep(2)']); "
                        "print('parent_done', flush=True)"
                    ),
                ],
                run_timeout_seconds=8.0,
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)
            status = (
                run_root / "background_stdio.status"
            ).read_text(encoding="utf-8").strip()
            self.assertEqual(status, "0")
            stdout_log = (run_root / "background_stdio.stdout.log").read_text(
                encoding="utf-8"
            )
            self.assertIn("parent_done", stdout_log)


if __name__ == "__main__":
    unittest.main()
