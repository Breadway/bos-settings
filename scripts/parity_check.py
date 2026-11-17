#!/usr/bin/env python3
"""Check the Slint port against every view and command in the original app."""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
STATUS = ROOT / "PORT_STATUS.md"


def original():
    views = sorted(p.stem for p in (ROOT / "frontend/src/lib/views").glob("*.svelte"))
    commands = set()
    for path in sorted((ROOT / "src/src/commands").glob("*.rs")):
        text = path.read_text()
        for match in re.finditer(r"^\s*pub\s+(?:async\s+)?fn\s+([A-Za-z0-9_]+)", text, re.M):
            commands.add(f"{path.stem}::{match.group(1)}")
    return set(views), commands


def checklist():
    entries = {}
    for line in STATUS.read_text().splitlines():
        match = re.match(r"- \[( |x|~)\] (view|command) `([^`]+)`(?:\s+->\s+(\S+))?", line)
        if not match:
            continue
        mark, kind, name, evidence = match.groups()
        entries[(kind, name)] = (mark, evidence)
    return entries


def main():
    views, commands = original()
    entries = checklist()
    failures = []
    for view in sorted(views):
        if ("view", view) not in entries:
            failures.append(f"missing from PORT_STATUS.md: view {view}")
    for command in sorted(commands):
        if ("command", command) not in entries:
            failures.append(f"missing from PORT_STATUS.md: command {command}")
    for view in sorted(views):
        mark, evidence = entries.get(("view", view), (" ", None))
        if mark != "x":
            failures.append(f"view {view}: {'todo' if mark == ' ' else 'partial'}")
        elif evidence and not (ROOT / evidence).exists():
            failures.append(f"view {view}: evidence path does not exist: {evidence}")
    for command in sorted(commands):
        mark, evidence = entries.get(("command", command), (" ", None))
        if mark != "x":
            failures.append(f"command {command}: {'todo' if mark == ' ' else 'partial'}")
        elif evidence and not (ROOT / evidence).exists():
            failures.append(f"command {command}: evidence path does not exist: {evidence}")
    extra = set(entries) - ({("view", v) for v in views} | {("command", c) for c in commands})
    for kind, name in sorted(extra):
        failures.append(f"not in the original app: {kind} {name}")
    if failures:
        print(f"{len(failures)} parity gap(s); {len(views)} views and {len(commands)} commands required")
        for failure in failures:
            print(f"  {failure}")
        return 1
    print(f"parity: {len(views)} views and {len(commands)} commands covered")
    return 0


if __name__ == "__main__":
    sys.exit(main())
