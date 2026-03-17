import sys
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPTS_DIR = REPO_ROOT / "scripts"
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from mako_rpc_milestone_contract import (  # pylint: disable=wrong-import-position
    RUN_ROOT_NAME_PATTERN,
    default_run_root_name,
    required_artifacts_m0_1,
    run_root_name_is_contract_valid,
    write_artifact_contract_manifest,
)


class MakoRpcMilestoneContractTests(unittest.TestCase):
    def test_default_run_root_name_matches_contract_pattern(self) -> None:
        fixed_now = datetime(2026, 3, 16, 21, 5, 11, tzinfo=timezone.utc)
        name = default_run_root_name(
            "m0_1_strict_baseline",
            now=fixed_now,
            pid=4242,
        )
        self.assertEqual(
            name,
            "fragile_m0_1_strict_baseline_20260316T210511Z_p4242",
        )
        self.assertTrue(run_root_name_is_contract_valid(name), msg=RUN_ROOT_NAME_PATTERN)

    def test_invalid_run_root_name_is_rejected(self) -> None:
        self.assertFalse(run_root_name_is_contract_valid("run"))
        self.assertFalse(
            run_root_name_is_contract_valid(
                "fragile_m0_1_strict_baseline_2026_03_16_p4242"
            )
        )

    def test_write_artifact_contract_manifest_reports_missing_files(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            run_root = Path(tmp) / "fragile_m0_1_strict_baseline_20260316T210511Z_p4242"
            run_root.mkdir(parents=True, exist_ok=True)
            required = required_artifacts_m0_1()
            for rel in required[:-1]:
                path = run_root / rel
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("ok\n", encoding="utf-8")

            manifest_path = run_root / "strict_baseline_required_artifacts_manifest.txt"
            summary = write_artifact_contract_manifest(
                manifest_path=manifest_path,
                task_leaf="M0.1",
                run_root=run_root,
                required_relpaths=required,
            )
            self.assertEqual(summary.expected_count, len(required))
            self.assertEqual(summary.missing_count, 1)

            payload = manifest_path.read_text(encoding="utf-8")
            self.assertIn("task_leaf=M0.1", payload)
            self.assertIn("required_artifact_count=14", payload)
            self.assertIn("missing_required_artifact_count=1", payload)
            self.assertIn("required_artifact_014_relpath=strict_baseline_manifest.txt", payload)
            self.assertIn("required_artifact_014_exists=false", payload)


if __name__ == "__main__":
    unittest.main()

