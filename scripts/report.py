#!/usr/bin/env python3
"""Question Desk weekly report. Standard library only.

Reads the questions.json written by the app, prints a summary and writes a CSV.
"""
import argparse
import csv
import json
import os
import sys
from collections import Counter
from datetime import datetime, timedelta, timezone
from pathlib import Path

APP_ID = "africa.apologetics.questiondesk"


def default_data_file() -> Path:
    """Where Tauri's app_data_dir() puts questions.json on each OS."""
    if sys.platform == "darwin":
        base = Path.home() / "Library" / "Application Support"
    elif sys.platform.startswith("win"):
        base = Path(os.environ.get("APPDATA", Path.home() / "AppData" / "Roaming"))
    else:
        base = Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local" / "share"))
    return base / APP_ID / "questions.json"


def parse_time(value: str):
    try:
        return datetime.fromisoformat(value.replace("Z", "+00:00"))
    except (ValueError, AttributeError):
        return None


def load_questions(path: Path) -> list:
    if not path.exists():
        sys.exit(
            f"Could not find the questions file at:\n  {path}\n"
            "Open the Question Desk app and save at least one question first, "
            "or point to the file with --file /path/to/questions.json"
        )
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as e:
        sys.exit(f"Could not read {path}: {e}")
    if not isinstance(data, list):
        sys.exit(f"{path} does not look like a Question Desk file (expected a list).")
    return data


def main() -> None:
    p = argparse.ArgumentParser(description="Summarise recent Question Desk questions.")
    p.add_argument("--days", type=int, default=7, help="look back this many days (default 7)")
    p.add_argument("--file", type=Path, help="path to questions.json (default: the app's data folder)")
    p.add_argument("--out", type=Path, help="CSV path (default: report_YYYY-MM-DD.csv in the current folder)")
    p.add_argument("--top", type=int, default=5, help="how many top tags to show (default 5)")
    args = p.parse_args()
    if args.days < 1:
        sys.exit("--days must be 1 or more.")

    path = args.file or default_data_file()
    everything = load_questions(path)

    cutoff = datetime.now(timezone.utc) - timedelta(days=args.days)
    recent = []
    for q in everything:
        if not isinstance(q, dict):
            continue
        t = parse_time(q.get("asked_at", ""))
        if t is not None and t >= cutoff:
            recent.append(q)
    recent.sort(key=lambda q: q.get("asked_at", ""), reverse=True)

    drafted = sum(1 for q in recent if q.get("draft"))
    tags = Counter(t.strip().lower() for q in recent for t in q.get("tags", []) if str(t).strip())

    print(f"Question Desk — last {args.days} days")
    print(f"{len(recent)} questions logged, {drafted} drafted")
    if tags:
        top = ", ".join(f"{t} ({n})" for t, n in tags.most_common(args.top))
        print(f"top tags: {top}")
    else:
        print("top tags: (none)")

    out = args.out or Path(f"report_{datetime.now().strftime('%Y-%m-%d')}.csv")
    try:
        with open(out, "w", newline="", encoding="utf-8") as f:
            w = csv.writer(f)
            w.writerow(["id", "asked_at", "asker", "question", "tags", "has_draft"])
            for q in recent:
                w.writerow([
                    q.get("id", ""),
                    q.get("asked_at", ""),
                    q.get("asker", ""),
                    q.get("question", ""),
                    "; ".join(q.get("tags", [])),
                    "yes" if q.get("draft") else "no",
                ])
    except OSError as e:
        sys.exit(f"Could not write {out}: {e}")
    print(f"wrote {out}")


if __name__ == "__main__":
    main()
