import json
import sys
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPTS_DIR = REPO_ROOT / "scripts"
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from parser_output_v1_contract import (  # pylint: disable=wrong-import-position
    canonical_round_trip,
    check_canonical_parser_output,
    dumps_parser_output_canonical,
    load_parser_output,
)


FIXTURE_PATH = REPO_ROOT / "docs" / "fixtures" / "parser_output_v1_full_placeholders.json"
CANONICAL_PATH = (
    REPO_ROOT / "docs" / "fixtures" / "parser_output_v1_full_placeholders.canonical.json"
)
TODO_PATH = REPO_ROOT / "TODO.md"


class ParserOutputV1SerializationTests(unittest.TestCase):
    def test_canonical_fixture_matches_serializer_output(self) -> None:
        self.assertTrue(
            check_canonical_parser_output(FIXTURE_PATH, CANONICAL_PATH),
            msg="canonical fixture should match deterministic serializer output",
        )

    def test_canonical_round_trip_is_stable(self) -> None:
        payload = load_parser_output(FIXTURE_PATH)
        first = dumps_parser_output_canonical(payload)
        second = dumps_parser_output_canonical(canonical_round_trip(payload))
        self.assertEqual(first, second)

    def test_canonical_serializer_is_key_order_invariant(self) -> None:
        payload = load_parser_output(FIXTURE_PATH)
        shuffled = {
            "diagnostics": payload["diagnostics"],
            "nodes": payload["nodes"],
            "schema_version": payload["schema_version"],
            "translation_unit": {
                "parser_backend": payload["translation_unit"]["parser_backend"],
                "include_directives": payload["translation_unit"]["include_directives"],
                "source_path": payload["translation_unit"]["source_path"],
                "defines": payload["translation_unit"]["defines"],
                "language": payload["translation_unit"]["language"],
                "frontend_args": payload["translation_unit"]["frontend_args"],
            },
        }

        shuffled_text = dumps_parser_output_canonical(shuffled)
        canonical_text = CANONICAL_PATH.read_text(encoding="utf-8")
        self.assertEqual(shuffled_text, canonical_text)

    def test_canonical_output_is_valid_json_and_semantically_equivalent(self) -> None:
        canonical_payload = json.loads(CANONICAL_PATH.read_text(encoding="utf-8"))
        fixture_payload = load_parser_output(FIXTURE_PATH)
        self.assertEqual(canonical_payload, fixture_payload)

    def test_todo_marks_m1_3_closed(self) -> None:
        todo_text = TODO_PATH.read_text(encoding="utf-8")
        self.assertIn(
            "- [x] M1.3 Add deterministic serialization + fixture tests for placeholder IR.",
            todo_text,
        )
        self.assertIn(
            "- [x] M1.A1 Schema docs and fixture corpus are checked in.",
            todo_text,
        )
        self.assertIn(
            "- [x] M1.A2 ParserOutput round-trip tests pass with deterministic output.",
            todo_text,
        )


if __name__ == "__main__":
    unittest.main()
