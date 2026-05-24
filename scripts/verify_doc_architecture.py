#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Verify that the reconstructed repository still matches the documented architecture.",
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="Repository root to inspect.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Print the full JSON report instead of a short summary.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = args.root.resolve()
    cmd = [
        sys.executable,
        str(root / "rust" / "scripts" / "run_mock_parity_diff.py"),
        "--root",
        str(root),
        "--pretty",
    ]
    completed = subprocess.run(cmd, check=False, capture_output=True, text=True)
    if completed.stdout:
        report = json.loads(completed.stdout)
    else:
        report = {
            "status": "fail",
            "errors": [completed.stderr.strip() or "parity diff script produced no output"],
            "summary": {},
        }

    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print(f"[verify-doc-architecture] {report['status'].upper()}")
        for error in report.get("errors", []):
            print(f"[verify-doc-architecture] error: {error}")
        if not report.get("errors"):
            summary = report.get("summary", {})
            print(
                "[verify-doc-architecture] crates=%d surfaces=%d modules=%d"
                % (
                    len(summary.get("workspace_members", [])),
                    len(summary.get("rust_surfaces", [])),
                    len(summary.get("runtime_module_map", [])),
                )
            )

    if completed.stderr:
        print(completed.stderr, file=sys.stderr, end="")
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main())
