#!/usr/bin/env python3
"""Escape hatch usage report generator.

Parses escape hatch log files written by fragilec's `log_escape_hatch_usage()`
and produces a structured usage report for CI trend monitoring.

Log line format:
    timestamp={secs} escape_kind={kind} source={source} pid={pid}

Usage:
    # Generate report from a log file
    python3 scripts/escape_hatch_usage_report.py /path/to/escape_hatch.log

    # Compare two snapshots for trending-to-zero gate
    python3 scripts/escape_hatch_usage_report.py --gate \
        --previous /path/to/previous.log \
        --current /path/to/current.log

    # Report from an empty/nonexistent log (expected: total=0)
    python3 scripts/escape_hatch_usage_report.py --gate --current /nonexistent.log

Exit codes:
    0 — report generated, or gate passed (non-increasing usage)
    1 — gate failed (usage increased)
    2 — invalid arguments
"""

import sys
import os
from collections import Counter


def parse_log_line(line):
    """Parse a single escape hatch log line into a dict."""
    line = line.strip()
    if not line:
        return None
    fields = {}
    for part in line.split():
        if "=" in part:
            key, _, value = part.partition("=")
            fields[key] = value
    required = {"timestamp", "escape_kind", "source", "pid"}
    if not required.issubset(fields.keys()):
        return None
    return fields


def parse_log_file(path):
    """Parse all entries from an escape hatch log file. Returns [] if file doesn't exist."""
    if not os.path.exists(path):
        return []
    with open(path, "r") as f:
        entries = []
        for line in f:
            entry = parse_log_line(line)
            if entry is not None:
                entries.append(entry)
        return entries


def generate_report(entries):
    """Generate a usage report from parsed entries."""
    by_kind = Counter(e["escape_kind"] for e in entries)
    by_source = Counter(e["source"] for e in entries)
    pids = set(e["pid"] for e in entries)
    timestamps = [int(e["timestamp"]) for e in entries if e["timestamp"].isdigit()]

    return {
        "total_count": len(entries),
        "distinct_pids": len(pids),
        "earliest_timestamp": min(timestamps) if timestamps else 0,
        "latest_timestamp": max(timestamps) if timestamps else 0,
        "by_kind": dict(by_kind),
        "by_source": dict(by_source),
    }


def format_report(report):
    """Format report as key=value lines for CI consumption."""
    lines = []
    lines.append(f"escape_hatch_total_count={report['total_count']}")
    lines.append(f"escape_hatch_distinct_pids={report['distinct_pids']}")
    lines.append(f"escape_hatch_earliest_timestamp={report['earliest_timestamp']}")
    lines.append(f"escape_hatch_latest_timestamp={report['latest_timestamp']}")
    for kind, count in sorted(report["by_kind"].items()):
        lines.append(f"escape_hatch_kind_{kind}={count}")
    for source, count in sorted(report["by_source"].items()):
        lines.append(f"escape_hatch_source_{source}={count}")
    return "\n".join(lines)


def main():
    args = sys.argv[1:]

    if "--gate" in args:
        args_without_gate = [a for a in args if a != "--gate"]
        previous_path = None
        current_path = None
        i = 0
        while i < len(args_without_gate):
            if args_without_gate[i] == "--previous" and i + 1 < len(args_without_gate):
                previous_path = args_without_gate[i + 1]
                i += 2
            elif args_without_gate[i] == "--current" and i + 1 < len(args_without_gate):
                current_path = args_without_gate[i + 1]
                i += 2
            else:
                i += 1

        if current_path is None:
            print("error: --gate requires --current <path>", file=sys.stderr)
            sys.exit(2)

        previous_entries = parse_log_file(previous_path) if previous_path else []
        current_entries = parse_log_file(current_path)

        previous_count = len(previous_entries)
        current_count = len(current_entries)

        current_report = generate_report(current_entries)
        print(format_report(current_report))
        print(f"escape_hatch_previous_count={previous_count}")
        print(f"escape_hatch_trending_to_zero={'true' if current_count <= previous_count else 'false'}")

        if current_count > previous_count:
            print(
                f"\nGATE FAILED: escape hatch usage increased {previous_count} -> {current_count}",
                file=sys.stderr,
            )
            sys.exit(1)
        else:
            print(
                f"\nGATE PASSED: escape hatch usage non-increasing {previous_count} -> {current_count}",
                file=sys.stderr,
            )
            sys.exit(0)

    elif len(args) == 1 and not args[0].startswith("-"):
        entries = parse_log_file(args[0])
        report = generate_report(entries)
        print(format_report(report))
        sys.exit(0)

    else:
        print(__doc__, file=sys.stderr)
        sys.exit(2)


if __name__ == "__main__":
    main()
