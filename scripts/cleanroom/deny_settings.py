#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Render a clean-room policy as a coding-agent session's settings file.

Reads a forbidden-set policy TOML (``--policy``) and prints to stdout a settings
JSON document for the clean-room session's project directory, with two layers:

* ``permissions.deny`` -- ``Read(<glob>)``, ``Grep(<glob>)`` and ``Glob(<glob>)`` for
  every ``[paths] deny`` glob, and ``WebSearch``. These are refused by the harness
  itself, before any hook runs.
* ``hooks.PreToolUse`` -- one entry whose matcher is
  ``Bash|Read|Grep|Glob|WebFetch|WebSearch|Edit|Write`` and whose command runs
  ``guard_hook.py --policy <policy>`` with both paths absolute. The hook is the
  layer that sees what a deny rule cannot: Bash command lines, commit ids, branch
  names and the WebFetch host allowlist.

POLICY SCHEMA. The same document ``transcript_audit.py`` and ``guard_hook.py`` read;
it is defined, with its matching semantics, in ``policy.py``::

    [paths]
    deny = ["~/.cargo/registry/**"]          # path globs; `~` is the home directory
    [bash]
    deny_substrings = ["rustlib/src"]        # refused anywhere in a Bash command
    [git]
    deny_shas = ["0123abc"]                  # 7+ hex digits; prefix match either way
    deny_branches = ["upstream-port"]        # refused as a whole Bash token
    [web]
    allow_hosts = ["rfc-editor.org"]         # WebFetch hosts (and subdomains) allowed

GLOB SPELLING. A permission rule reads ``/path`` as relative to the settings file and
``//path`` as an absolute path, so an absolute policy glob is written with a leading
``//``; ``~/`` globs are written as they are, and relative globs are written as they
are and so resolve against the project the settings file belongs to.

    python3 scripts/cleanroom/deny_settings.py --policy policy.toml > <settings-file>
    python3 scripts/cleanroom/deny_settings.py --self-test
"""

from __future__ import annotations

import argparse
import json
import shlex
import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import policy as policy_module  # noqa: E402  (the shared matcher beside this file)

REPO_ROOT = Path(__file__).resolve().parents[2]
HOOK = Path(__file__).resolve().parent / "guard_hook.py"
MATCHER = "Bash|Read|Grep|Glob|WebFetch|WebSearch|Edit|Write"
RULE_TOOLS = ("Read", "Grep", "Glob")


def rule_glob(glob: str) -> str:
    """A policy glob in permission-rule spelling."""
    if glob.startswith("/") and not glob.startswith("//"):
        return "/" + glob
    return glob


def settings(policy: policy_module.Policy, policy_path: Path) -> dict:
    """The settings document for *policy*, whose file is *policy_path*."""
    deny = [f"{tool}({rule_glob(glob)})" for glob in policy.path_globs for tool in RULE_TOOLS]
    deny.append("WebSearch")
    command = " ".join(
        shlex.quote(part)
        for part in ("python3", str(HOOK), "--policy", str(policy_path.resolve()))
    )
    return {
        "permissions": {"deny": deny},
        "hooks": {
            "PreToolUse": [
                {"matcher": MATCHER, "hooks": [{"type": "command", "command": command}]}
            ]
        },
    }


def structural_problems(document: object) -> list[str]:
    """Every way *document* departs from the settings shape this program promises."""
    problems: list[str] = []
    if not isinstance(document, dict) or set(document) != {"permissions", "hooks"}:
        return ["top level must be an object with exactly `permissions` and `hooks`"]
    deny = document["permissions"].get("deny") if isinstance(document["permissions"], dict) else None
    if not isinstance(deny, list) or not all(isinstance(rule, str) for rule in deny):
        problems.append("permissions.deny must be a list of strings")
    elif "WebSearch" not in deny:
        problems.append("permissions.deny must refuse WebSearch")
    pre = document["hooks"].get("PreToolUse") if isinstance(document["hooks"], dict) else None
    if not isinstance(pre, list) or len(pre) != 1:
        return problems + ["hooks.PreToolUse must hold exactly one entry"]
    entry = pre[0]
    if not isinstance(entry, dict) or entry.get("matcher") != MATCHER:
        problems.append(f"hooks.PreToolUse[0].matcher must be {MATCHER!r}")
    hooks = entry.get("hooks") if isinstance(entry, dict) else None
    if (
        not isinstance(hooks, list)
        or len(hooks) != 1
        or hooks[0].get("type") != "command"
        or not isinstance(hooks[0].get("command"), str)
    ):
        problems.append("hooks.PreToolUse[0].hooks must be one command hook")
    else:
        words = shlex.split(hooks[0]["command"])
        if (
            len(words) != 4
            or words[0] != "python3"
            or not Path(words[1]).is_absolute()
            or not words[1].endswith("scripts/cleanroom/guard_hook.py")
            or words[2] != "--policy"
            or not Path(words[3]).is_absolute()
        ):
            problems.append(f"the hook command is not `python3 <abs guard_hook.py> --policy <abs>`: {words}")
    return problems


def self_test() -> int:
    """The document validates, and a broken one is refused by the same validator."""
    scratch = REPO_ROOT / "target" / "cleanroom-selftest" / "deny_settings"
    if scratch.exists():
        shutil.rmtree(scratch)
    scratch.mkdir(parents=True)
    policy_path = scratch / "policy with space.toml"
    policy_path.write_text(
        '[paths]\ndeny = ["~/.cargo/registry/**", "/srv/upstream/**", "vendor/**"]\n',
        encoding="utf-8",
    )
    document = settings(policy_module.load_policy(policy_path), policy_path)
    ok = True
    round_tripped = json.loads(json.dumps(document))
    problems = structural_problems(round_tripped)
    expected_rules = {
        "Read(~/.cargo/registry/**)",
        "Grep(//srv/upstream/**)",
        "Glob(vendor/**)",
        "WebSearch",
    }
    if not problems and expected_rules <= set(round_tripped["permissions"]["deny"]):
        print("OK: self-test — the generated settings validate and spell every glob as a rule")
    else:
        print(f"SELF-TEST FAIL: generated settings: {problems} {round_tripped['permissions']}")
        ok = False
    command = round_tripped["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
    if shlex.split(command)[3] == str(policy_path.resolve()):
        print("OK: self-test — a policy path with a space survives the hook command's quoting")
    else:
        print(f"SELF-TEST FAIL: hook command quoting: {command}")
        ok = False

    broken = json.loads(json.dumps(document))
    broken["hooks"]["PreToolUse"][0]["matcher"] = "Read"
    broken["permissions"]["deny"].remove("WebSearch")
    found = structural_problems(broken)
    if len(found) == 2:
        print("OK: self-test — a narrowed matcher and a missing WebSearch rule are both refused")
    else:
        print(f"SELF-TEST FAIL: broken settings reported {found}")
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
        parser.error("--policy is required")
    try:
        policy = policy_module.load_policy(args.policy)
    except policy_module.PolicyError as error:
        print(f"deny_settings: {error}", file=sys.stderr)
        return 2
    document = settings(policy, args.policy)
    problems = structural_problems(document)
    if problems:
        print(f"deny_settings: generated settings failed validation: {problems}", file=sys.stderr)
        return 1
    print(json.dumps(document, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
