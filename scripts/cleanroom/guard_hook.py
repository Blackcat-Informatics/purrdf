#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""A pre-tool-use hook that blocks a tool call touching the forbidden set.

The coding agent runs this before every tool call its matcher selects, with the hook
event on stdin: ``{"tool_name": ..., "tool_input": {...}, "cwd": ...}``. The call is
judged by ``policy.check_call`` -- the matcher ``transcript_audit.py`` replays after
the fact -- against the policy named by ``--policy`` (schema in ``policy.py``).

Exit status is the hook protocol's: 0 lets the call run; 2 blocks it, and the
reason printed to stderr is shown to the agent. A hook input that cannot be read, a
policy that does not validate, or an audited tool whose input lacks the field its
rule reads also exits 2: a guard that fails open is not a guard.

``deny_settings.py`` writes the settings entry that installs this hook.

    python3 scripts/cleanroom/guard_hook.py --policy policy.toml < event.json
    python3 scripts/cleanroom/guard_hook.py --self-test
"""

from __future__ import annotations

import argparse
import io
import json
import shutil
import sys
from contextlib import redirect_stderr
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import policy as policy_module  # noqa: E402  (the shared matcher beside this file)

REPO_ROOT = Path(__file__).resolve().parents[2]
BLOCK = 2


def decide(event_text: str, policy: policy_module.Policy) -> int:
    """0 to allow the call described by *event_text*, 2 to block it (reason on stderr)."""
    try:
        event = json.loads(event_text)
    except json.JSONDecodeError as error:
        print(f"clean-room guard: unreadable hook input ({error.msg}); blocking", file=sys.stderr)
        return BLOCK
    if not isinstance(event, dict) or not isinstance(event.get("tool_name"), str):
        print("clean-room guard: hook input has no tool_name; blocking", file=sys.stderr)
        return BLOCK
    cwd = event.get("cwd") if isinstance(event.get("cwd"), str) else None
    try:
        hits = policy_module.check_call(event["tool_name"], event.get("tool_input"), policy, cwd)
    except policy_module.ShapeError as error:
        print(f"clean-room guard: {error}; blocking", file=sys.stderr)
        return BLOCK
    if hits:
        for hit in hits:
            print(
                f"clean-room guard: {hit.tool} blocked by {hit.rule}: {hit.subject[:120]}",
                file=sys.stderr,
            )
        return BLOCK
    return 0


def self_test() -> int:
    """The hook blocks a forbidden call and passes its neighbour."""
    scratch = REPO_ROOT / "target" / "cleanroom-selftest" / "guard_hook"
    if scratch.exists():
        shutil.rmtree(scratch)
    scratch.mkdir(parents=True)
    policy_path = scratch / "policy.toml"
    policy_path.write_text(
        f'[paths]\ndeny = ["{scratch}/upstream/**"]\n'
        '[bash]\ndeny_substrings = ["rustlib/src"]\n'
        '[web]\nallow_hosts = ["rfc-editor.org"]\n',
        encoding="utf-8",
    )
    policy = policy_module.load_policy(policy_path)
    cases = [
        ("Read inside the forbidden tree", {"tool_name": "Read", "tool_input": {"file_path": f"{scratch}/upstream/lib.rs"}}, BLOCK),
        ("Read beside it", {"tool_name": "Read", "tool_input": {"file_path": f"{scratch}/own/lib.rs"}}, 0),
        ("Bash into rustlib/src", {"tool_name": "Bash", "tool_input": {"command": "ls ~/.rustup/toolchains/x/lib/rustlib/src"}}, BLOCK),
        ("Bash into rustlib/bin", {"tool_name": "Bash", "tool_input": {"command": "ls ~/.rustup/toolchains/x/lib/rustlib/bin"}}, 0),
        (
            "cd into an allowed clone, then a relative find, from a forbidden session cwd",
            {"tool_name": "Bash", "tool_input": {"command": f"cd {scratch}/allowed/clone && find ."}, "cwd": f"{scratch}/upstream"},
            0,
        ),
        (
            "cd into an allowed clone, then a ..-relative cat that lands in the forbidden tree",
            {
                "tool_name": "Bash",
                "tool_input": {"command": f"cd {scratch}/allowed/clone && cat ../../upstream/x.rs"},
                "cwd": f"{scratch}/upstream",
            },
            BLOCK,
        ),
        (
            "cd straight into the forbidden tree",
            {"tool_name": "Bash", "tool_input": {"command": f"cd {scratch}/upstream/tree && ls"}, "cwd": f"{scratch}/upstream"},
            BLOCK,
        ),
        (
            "a heredoc body assignment after cd is not a path-shaped word",
            {
                "tool_name": "Bash",
                "tool_input": {"command": f"cd {scratch}/allowed/clone && python3 - <<'EOF'\np='crates/a.rs'\nEOF"},
                "cwd": f"{scratch}/upstream",
            },
            0,
        ),
        ("WebSearch", {"tool_name": "WebSearch", "tool_input": {"query": "q"}}, BLOCK),
        ("an unaudited tool", {"tool_name": "TodoWrite", "tool_input": {"todos": []}}, 0),
        ("an unreadable event", None, BLOCK),
    ]
    ok = True
    for label, event, expected in cases:
        text = "{not json" if event is None else json.dumps(event)
        captured = io.StringIO()
        with redirect_stderr(captured):
            status = decide(text, policy)
        if status == expected and (expected == 0) == (captured.getvalue() == ""):
            print(f"OK: self-test — {label}: {'blocked' if expected else 'allowed'}")
        else:
            print(f"SELF-TEST FAIL: {label}: exit {status}, expected {expected}; {captured.getvalue()!r}")
            ok = False
    print("SELF-TEST PASS" if ok else "SELF-TEST FAIL")
    return 0 if ok else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--policy", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    if args.policy is None:
        print("clean-room guard: --policy is required; blocking", file=sys.stderr)
        return BLOCK
    try:
        policy = policy_module.load_policy(args.policy)
    except policy_module.PolicyError as error:
        print(f"clean-room guard: {error}; blocking", file=sys.stderr)
        return BLOCK
    return decide(sys.stdin.read(), policy)


if __name__ == "__main__":
    raise SystemExit(main())
