import json
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCHEMA_PATH = REPO_ROOT / "docs" / "schemas" / "parser_output_v1.schema.json"
FIXTURE_PATH = REPO_ROOT / "docs" / "fixtures" / "parser_output_v1_full_placeholders.json"
TODO_PATH = REPO_ROOT / "TODO.md"

REQUIRED_STL_PLACEHOLDER_KINDS = {
    "stl_vector_placeholder": "vector",
    "stl_map_placeholder": "map",
    "stl_unordered_map_placeholder": "unordered_map",
    "stl_string_placeholder": "string",
    "stl_optional_placeholder": "optional",
    "stl_variant_placeholder": "variant",
    "stl_tuple_placeholder": "tuple",
    "stl_shared_ptr_placeholder": "shared_ptr",
    "stl_unique_ptr_placeholder": "unique_ptr",
}


class ParserOutputV1SchemaTests(unittest.TestCase):
    def test_schema_file_exists_and_pins_v1_version(self) -> None:
        self.assertTrue(SCHEMA_PATH.exists())
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        self.assertEqual(schema["properties"]["schema_version"]["const"], "1.0.0")

    def test_schema_defines_explicit_stl_placeholder_node_kinds(self) -> None:
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        node_kinds = schema["$defs"]["node"]["properties"]["node_kind"]["enum"]

        self.assertEqual(len(node_kinds), len(set(node_kinds)))
        for node_kind in REQUIRED_STL_PLACEHOLDER_KINDS:
            self.assertIn(node_kind, node_kinds)

    def test_fixture_covers_all_required_placeholder_kinds_with_family_mapping(self) -> None:
        fixture = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))
        nodes = fixture["nodes"]

        observed = {}
        for node in nodes:
            node_kind = node.get("node_kind")
            if node_kind not in REQUIRED_STL_PLACEHOLDER_KINDS:
                continue

            expected_family = REQUIRED_STL_PLACEHOLDER_KINDS[node_kind]
            payload = node.get("stl_placeholder")
            self.assertIsInstance(payload, dict)
            self.assertEqual(payload.get("family"), expected_family)
            observed[node_kind] = payload.get("family")

        self.assertEqual(observed, REQUIRED_STL_PLACEHOLDER_KINDS)

    def test_fixture_uses_only_node_kinds_defined_in_schema(self) -> None:
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        fixture = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))

        allowed_kinds = set(schema["$defs"]["node"]["properties"]["node_kind"]["enum"])
        fixture_kinds = {node["node_kind"] for node in fixture["nodes"]}
        self.assertTrue(fixture_kinds.issubset(allowed_kinds))

    def test_todo_marks_m1_1_as_closed(self) -> None:
        todo_text = TODO_PATH.read_text(encoding="utf-8")
        self.assertIn(
            "- [x] M1.1 Define `ParserOutput v1` schema with explicit STL placeholder node kinds.",
            todo_text,
        )


if __name__ == "__main__":
    unittest.main()
