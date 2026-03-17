#!/usr/bin/env python3
"""Deterministic serialization helpers for ParserOutput v1 artifacts."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Mapping


def load_parser_output(path: Path) -> Mapping[str, Any]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError(f"expected top-level object in {path}, got {type(payload).__name__}")
    return payload


def dumps_parser_output_canonical(payload: Mapping[str, Any]) -> str:
    """Return stable, pretty canonical JSON with sorted object keys."""
    return json.dumps(payload, ensure_ascii=True, indent=2, sort_keys=True) + "\n"


def canonical_round_trip(payload: Mapping[str, Any]) -> Mapping[str, Any]:
    """Serialize and deserialize through canonical representation."""
    return json.loads(dumps_parser_output_canonical(payload))


def write_canonical_parser_output(input_path: Path, output_path: Path) -> None:
    payload = load_parser_output(input_path)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(
        dumps_parser_output_canonical(payload),
        encoding="utf-8",
    )


def check_canonical_parser_output(input_path: Path, canonical_path: Path) -> bool:
    payload = load_parser_output(input_path)
    expected = dumps_parser_output_canonical(payload)
    if not canonical_path.exists():
        return False
    observed = canonical_path.read_text(encoding="utf-8")
    return observed == expected


def build_arg_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Generate or verify deterministic canonical ParserOutput v1 JSON."
    )
    parser.add_argument("--input", required=True, type=Path, help="input ParserOutput JSON path")
    parser.add_argument(
        "--canonical-output",
        required=True,
        type=Path,
        help="canonical output JSON path",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify canonical output already matches deterministic serialization",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_arg_parser().parse_args(argv)
    if args.check:
        ok = check_canonical_parser_output(args.input, args.canonical_output)
        if ok:
            print("canonical output matches")
            return 0
        print("canonical output mismatch")
        return 1

    write_canonical_parser_output(args.input, args.canonical_output)
    print(args.canonical_output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
