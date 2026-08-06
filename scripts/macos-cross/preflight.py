#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from project_metadata import inspect_project


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate FyAgent for the WSL macOS Universal DMG workflow."
    )
    parser.add_argument("root", nargs="?", default=".", help="project root")
    parser.add_argument("--json", action="store_true", help="emit structured JSON")
    parser.add_argument("--field", help="print one project metadata field")
    args = parser.parse_args()

    inspection = inspect_project(Path(args.root))
    project = inspection.metadata.to_json() if inspection.metadata else {}

    if args.field:
        if inspection.errors:
            for error in inspection.errors:
                print(f"error: {error}", file=sys.stderr)
            return 1
        if args.field not in project:
            print(f"error: unknown field: {args.field}", file=sys.stderr)
            return 2
        value = project[args.field]
        if isinstance(value, (dict, list)):
            print(json.dumps(value, ensure_ascii=False))
        else:
            print(value)
        return 0

    if args.json:
        print(
            json.dumps(
                {
                    "project": project,
                    "errors": list(inspection.errors),
                    "warnings": list(inspection.warnings),
                },
                ensure_ascii=False,
                indent=2,
            )
        )
    else:
        print("FyAgent WSL macOS Universal DMG preflight")
        print("=" * 46)
        for key, value in project.items():
            print(f"{key}: {value}")
        for warning in inspection.warnings:
            print(f"warning: {warning}", file=sys.stderr)
        for error in inspection.errors:
            print(f"error: {error}", file=sys.stderr)

    return 1 if inspection.errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
