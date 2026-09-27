#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Audit agent session transcripts against a clean-room forbidden-set policy.

Input: one or more agent session transcripts (JSONL, one JSON object per line) and a
policy TOML (``--policy``; the schema is documented in ``policy.py``). Every tool
call in every transcript is replayed through the same matcher ``guard_hook.py``
enforces live, so an audit and a guard cannot disagree about what was forbidden.

WHERE TOOL CALLS ARE FOUND. A tool call is any JSON object of the form
``{"type": "tool_use", "name": <tool>, "input": {...}}``, found by walking the whole
line rather than one expected path. The top-level shape is
``{"type": "assistant", "message": {"content": [<tool_use>, ...]}}``, but subagent
activity is carried nested inside other records, and a walker keyed on one path
would silently miss exactly the calls a delegated agent made. A call repeated in
several records of the same file (same ``id``) is counted once, at its first line.

NOTHING IS SILENTLY SKIPPED:

* a line that is not valid JSON is an ERROR, and so is an audited tool call whose
  input lacks the field its rule reads -- the audit is then incomplete, exit 2;
* a line whose top-level ``type`` is not a known record type is listed under
  ``unknown_shapes`` (it is still walked for tool calls);
* a tool the policy has no rule for (for example an MCP tool) is counted under
  ``unaudited_tools``, so a reviewer sees what the audit did not judge.

Output (stdout): ``{"report": {...}, "report_sha256": "<hex>"}`` where the digest is
over the report's canonical JSON (sorted keys, no whitespace). The report holds
``transcripts``, ``tool_calls_seen``, ``violations`` (``file``, ``line``, ``tool``,
``matched_rule``, ``excerpt`` of at most 120 characters), ``errors``,
``unknown_shapes`` and ``unaudited_tools``.

Exit status: 2 when any error made the audit incomplete, else 1 on any violation,
else 0.

    python3 scripts/cleanroom/transcript_audit.py --policy policy.toml session.jsonl ...
    python3 scripts/cleanroom/transcript_audit.py --self-test
"""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import policy as policy_module  # noqa: E402  (the shared matcher beside this file)

REPO_ROOT = Path(__file__).resolve().parents[2]
EXCERPT_LIMIT = 120
KNOWN_RECORD_TYPES = frozenset(
    {
        "user",
        "assistant",
        "system",
        "summary",
        "progress",
        "result",
        "attachment",
        "file-history-snapshot",
        "queue-operation",
    }
)


def excerpt(text: str) -> str:
    """At most 120 characters of *text*, on one line."""
    flat = " ".join(text.split())
    return flat if len(flat) <= EXCERPT_LIMIT else flat[: EXCERPT_LIMIT - 3] + "..."


def tool_uses(node: object) -> list[dict]:
    """Every ``tool_use`` object anywhere inside *node*, in document order."""
    found: list[dict] = []
    stack = [node]
    while stack:
        current = stack.pop()
        if isinstance(current, dict):
            if current.get("type") == "tool_use" and "name" in current and "input" in current:
                found.append(current)
            stack.extend(reversed(list(current.values())))
        elif isinstance(current, list):
            stack.extend(reversed(current))
    return found


def audit(transcripts: list[Path], policy: policy_module.Policy) -> tuple[dict, int]:
    """The audit report for *transcripts* and the exit status it implies."""
    violations: list[dict] = []
    errors: list[dict] = []
    unknown_shapes: list[dict] = []
    unaudited: dict[str, int] = {}
    seen_calls = 0
    for transcript in transcripts:
        name = str(transcript)
        try:
            lines = transcript.read_text(encoding="utf-8").splitlines()
        except (OSError, UnicodeDecodeError) as error:
            errors.append({"file": name, "line": 0, "error": f"unreadable: {error}"})
            continue
        seen_ids: set[str] = set()
        for number, raw in enumerate(lines, start=1):
            if not raw.strip():
                continue
            try:
                record = json.loads(raw)
            except json.JSONDecodeError as error:
                errors.append({"file": name, "line": number, "error": f"malformed JSON: {error.msg}"})
                continue
            record_type = record.get("type") if isinstance(record, dict) else None
            if record_type not in KNOWN_RECORD_TYPES:
                unknown_shapes.append(
                    {"file": name, "line": number, "type": repr(record_type)[:60]}
                )
            cwd = record.get("cwd") if isinstance(record, dict) else None
            cwd = cwd if isinstance(cwd, str) else None
            for call in tool_uses(record):
                call_id = call.get("id")
                if isinstance(call_id, str):
                    if call_id in seen_ids:
                        continue
                    seen_ids.add(call_id)
                tool = call["name"]
                if not isinstance(tool, str):
                    unknown_shapes.append({"file": name, "line": number, "type": "tool_use without a string name"})
                    continue
                seen_calls += 1
                if tool not in policy_module.AUDITED_TOOLS:
                    unaudited[tool] = unaudited.get(tool, 0) + 1
                    continue
                try:
                    hits = policy_module.check_call(tool, call["input"], policy, cwd)
                except policy_module.ShapeError as error:
                    errors.append({"file": name, "line": number, "error": str(error)})
                    continue
                for hit in hits:
                    violations.append(
                        {
                            "file": name,
                            "line": number,
                            "tool": hit.tool,
                            "matched_rule": hit.rule,
                            "excerpt": excerpt(hit.subject),
                        }
                    )
    report = {
        "transcripts": [str(path) for path in transcripts],
        "tool_calls_seen": seen_calls,
        "violations": violations,
        "errors": errors,
        "unknown_shapes": unknown_shapes,
        "unaudited_tools": dict(sorted(unaudited.items())),
    }
    status = 2 if errors else (1 if violations else 0)
    return report, status


def digest(report: dict) -> str:
    """SHA-256 over the report's canonical JSON."""
    canonical = json.dumps(report, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def _assistant(call_id: str, tool: str, tool_input: dict, cwd: str) -> str:
    return json.dumps(
        {
            "type": "assistant",
            "cwd": cwd,
            "message": {
                "role": "assistant",
                "content": [{"type": "tool_use", "id": call_id, "name": tool, "input": tool_input}],
            },
        }
    )


def self_test() -> int:
    """Each refusal beside the neighbour it must accept, executed."""
    scratch = REPO_ROOT / "target" / "cleanroom-selftest" / "transcript_audit"
    if scratch.exists():
        shutil.rmtree(scratch)
    scratch.mkdir(parents=True)
    home = str(scratch / "home")
    policy = policy_module.parse_policy(
        {
            "paths": {"deny": [f"{scratch}/forbidden/**", "~/.cargo/registry/**"]},
            "bash": {"deny_substrings": [".cargo/registry", "rustlib/src", "git cat-file"]},
            "git": {"deny_shas": ["abcdef0123456"], "deny_branches": ["upstream-port"]},
            "web": {"allow_hosts": ["rfc-editor.org"]},
        },
        home=home,
    )
    cwd = str(scratch)
    cases = [
        ("forbidden Read path", "Read", {"file_path": f"{scratch}/forbidden/lib.rs"}, 1),
        ("allowed neighbour Read path", "Read", {"file_path": f"{scratch}/forbiddenness/lib.rs"}, 0),
        ("relative Read into the forbidden tree", "Read", {"file_path": "forbidden/../forbidden/x.rs"}, 1),
        ("Bash cat of a registry source", "Bash", {"command": "cat ~/.cargo/registry/src/x"}, 1),
        ("Bash cat of cargo config", "Bash", {"command": "cat ~/.cargo/config.toml"}, 0),
        ("forbidden SHA abbreviation in git", "Bash", {"command": "git log -1 abcdef0"}, 1),
        ("a different SHA in git", "Bash", {"command": "git log -1 abcdef1999999"}, 0),
        ("forbidden branch in git", "Bash", {"command": "git diff upstream-port -- src"}, 1),
        ("a branch sharing its prefix", "Bash", {"command": "git diff upstream-port-docs -- src"}, 0),
        ("WebFetch to a non-allowlisted host", "WebFetch", {"url": "https://example.com/src", "prompt": "p"}, 1),
        ("WebFetch to rfc-editor.org", "WebFetch", {"url": "https://www.rfc-editor.org/rfc/rfc3986", "prompt": "p"}, 0),
        ("WebSearch", "WebSearch", {"query": "anything"}, 1),
        ("Grep from an ancestor of the forbidden tree", "Grep", {"pattern": "x", "path": str(scratch)}, 1),
        ("Grep inside an allowed tree", "Grep", {"pattern": "x", "path": f"{scratch}/allowed"}, 0),
    ]
    ok = True
    for index, (label, tool, tool_input, expected) in enumerate(cases):
        path = scratch / f"case{index:02d}.jsonl"
        path.write_text(_assistant(f"toolu_{index}", tool, tool_input, cwd) + "\n", encoding="utf-8")
        report, status = audit([path], policy)
        if status == expected and report["tool_calls_seen"] == 1:
            verdict = "rejected" if expected else "accepted"
            print(f"OK: self-test — {label} is {verdict}")
        else:
            print(f"SELF-TEST FAIL: {label}: exit {status}, expected {expected}; {report}")
            ok = False

    # A Bash `cd` changes the directory later `&&`/`;` segments resolve relative
    # words against, even when the session's own starting cwd sits elsewhere (here,
    # inside the forbidden tree itself).
    cd_cwd = str(scratch / "forbidden")
    clone = str(scratch / "allowed" / "clone")
    cd_cases = [
        (
            "a `cd` into an allowed clone, then a relative `find`, from a forbidden session cwd",
            {"command": f"cd {clone} && find ."},
            0,
        ),
        (
            "a `cd` into an allowed clone, then a `..`-relative `cat` that lands in the forbidden tree",
            {"command": f"cd {clone} && cat ../../forbidden/x.rs"},
            1,
        ),
        (
            "a `cd` straight into the forbidden tree",
            {"command": f"cd {scratch}/forbidden/tree && ls"},
            1,
        ),
        (
            "a heredoc body assignment after `cd` is not a path-shaped word",
            {"command": f"cd {clone} && python3 - <<'EOF'\np='crates/a.rs'\nEOF"},
            0,
        ),
    ]
    for index, (label, tool_input, expected) in enumerate(cd_cases):
        path = scratch / f"cd-case{index:02d}.jsonl"
        path.write_text(_assistant(f"toolu_cd{index}", "Bash", tool_input, cd_cwd) + "\n", encoding="utf-8")
        report, status = audit([path], policy)
        if status == expected and report["tool_calls_seen"] == 1:
            verdict = "rejected" if expected else "accepted"
            print(f"OK: self-test — {label} is {verdict}")
        else:
            print(f"SELF-TEST FAIL: {label}: exit {status}, expected {expected}; {report}")
            ok = False

    nested = scratch / "nested.jsonl"
    wrapped = {
        "type": "progress",
        "data": {"message": {"type": "assistant", "message": {"content": [
            {"type": "tool_use", "id": "toolu_n", "name": "Read",
             "input": {"file_path": f"{scratch}/forbidden/deep.rs"}}]}}},
    }
    nested.write_text(json.dumps(wrapped) + "\n" + json.dumps(wrapped) + "\n", encoding="utf-8")
    report, status = audit([nested], policy)
    if status == 1 and report["tool_calls_seen"] == 1 and report["violations"][0]["line"] == 1:
        print("OK: self-test — a subagent call nested in another record is found, once")
    else:
        print(f"SELF-TEST FAIL: nested subagent call: exit {status}; {report}")
        ok = False

    odd = scratch / "odd.jsonl"
    odd.write_text(
        json.dumps({"type": "novel-record", "payload": 1}) + "\n"
        + _assistant("toolu_m", "mcp__x__read", {"path": "/"}, cwd) + "\n",
        encoding="utf-8",
    )
    report, status = audit([odd], policy)
    if status == 0 and report["unknown_shapes"] and report["unaudited_tools"] == {"mcp__x__read": 1}:
        print("OK: self-test — an unknown record type and an unaudited tool are reported, not dropped")
    else:
        print(f"SELF-TEST FAIL: unknown shape reporting: exit {status}; {report}")
        ok = False

    bad = scratch / "malformed.jsonl"
    bad.write_text(_assistant("toolu_ok", "Read", {"file_path": f"{scratch}/allowed/a.rs"}, cwd)
                   + "\n{\"type\": \"assistant\", \n", encoding="utf-8")
    report, status = audit([bad], policy)
    if status == 2 and report["errors"] and report["errors"][0]["line"] == 2:
        print("OK: self-test — a malformed line is an error and the audit exits 2")
    else:
        print(f"SELF-TEST FAIL: malformed line: exit {status}; {report}")
        ok = False

    shapeless = scratch / "shapeless.jsonl"
    shapeless.write_text(_assistant("toolu_s", "Read", {"path": "/x"}, cwd) + "\n", encoding="utf-8")
    report, status = audit([shapeless], policy)
    if status == 2:
        print("OK: self-test — an audited tool whose input lacks its field is an error, not a pass")
    else:
        print(f"SELF-TEST FAIL: shapeless Read input: exit {status}; {report}")
        ok = False

    first = digest({"a": 1, "b": [1, 2]})
    if first == digest({"b": [1, 2], "a": 1}) and first != digest({"a": 1, "b": [2, 1]}):
        print("OK: self-test — the report digest is key-order independent and content sensitive")
    else:
        print("SELF-TEST FAIL: report digest")
        ok = False

    print("SELF-TEST PASS" if ok else "SELF-TEST FAIL")
    return 0 if ok else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--policy", type=Path, help="forbidden-set policy TOML")
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("transcripts", nargs="*", type=Path)
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    if args.policy is None or not args.transcripts:
        parser.error("--policy and at least one transcript are required")
    try:
        policy = policy_module.load_policy(args.policy)
    except policy_module.PolicyError as error:
        print(f"transcript_audit: {error}", file=sys.stderr)
        return 2
    report, status = audit(args.transcripts, policy)
    print(json.dumps({"report": report, "report_sha256": digest(report)}, indent=2, sort_keys=True))
    return status


if __name__ == "__main__":
    raise SystemExit(main())
