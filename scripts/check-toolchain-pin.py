#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Hygiene gate: `rust-toolchain.toml` must name the floating `nightly` channel,
every CI job that runs a workspace gate must install the channel it names, and
the jobs that deliberately do NOT must say so out loud.

The workspace names a FLOATING `nightly` for development and CI. Nightly clippy
and rustdoc carry lints stable lacks, and the current default borrow checker is
markedly stronger than an older one, so a finding is a real finding rather than a
channel artifact. Floating is the point: a dated channel freezes that analysis
surface as of one day, and every lint sharpened afterwards stops being a finding
and becomes invisible debt. So a date in `rust-toolchain.toml` is drift, and
fails here.

Byte-determinism is not a counter-argument. It is a property of the code --
sorted, deduplicated, explicitly ordered output, identities content-addressed
over this workspace's own declared law -- and the goldens and vectors prove it on
whatever compiler CI runs. A golden that moved under a compiler bump would be a
serializer defect to fix, not a reason to stop bumping.

What must still agree is the CHANNEL NAME: every CI job that runs a workspace
gate must install the channel `rust-toolchain.toml` names, and the jobs that
deliberately do NOT must say so out loud. A job left on `stable` while developers
run nightly makes a failure impossible to attribute -- green on one machine, red
on another, with no way to tell a genuine failure from a channel difference.

Drift here is invisible without a gate. `dtolnay/rust-toolchain` selects a
toolchain with `rustup default`, which sits at the BOTTOM of rustup's precedence
order -- below `rust-toolchain.toml`. So a workflow whose install step says one
thing and whose repo pin says another does not fail; it silently runs the pin,
and the workflow reads as if it tested something it never tested. The `msrv` job
was doing precisely that: it installed 1.96, then every `cargo` invocation
resolved back to the repo pin.

Two escapes exist, and both must be explicit:

* `RUSTUP_TOOLCHAIN` outranks `rust-toolchain.toml`, so a workflow that sets it
  really does use the toolchain it names. That is how the `msrv` job holds the
  1.96 floor and how the release lanes stay on stable.
* Anything else -- an install step naming a toolchain the pin does not name,
  with no `RUSTUP_TOOLCHAIN` to back it -- is drift, and fails here.

Pure text over committed files: no cargo, no network, no rustup.
Run standalone, or as part of `make check` / CI.
"""

from __future__ import annotations

import re
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
TOOLCHAIN_FILE = REPO / "rust-toolchain.toml"
WORKFLOW_DIR = REPO / ".github" / "workflows"

CHANNEL_RE = re.compile(r'^\s*channel\s*=\s*"([^"]+)"\s*$', re.MULTILINE)
# The one channel this workspace develops and gates on, floating by design.
FLOATING_CHANNEL = "nightly"
# `nightly-2026-09-02` and friends: a channel welded to one day's analysis surface.
DATED_CHANNEL_RE = re.compile(r"^(?:nightly|beta|stable)-\d{4}-\d{2}-\d{2}$")
# `1.96`, `1.96.0`, `1.96.0-x86_64-unknown-linux-gnu`: an exact release, equally frozen.
RELEASE_CHANNEL_RE = re.compile(r"^\d+\.\d+(?:\.\d+)?\b")
ACTION_RE = re.compile(r"uses:\s*dtolnay/rust-toolchain@")
LIST_ITEM_RE = re.compile(r"^(\s*)-\s")
TOOLCHAIN_INPUT_RE = re.compile(r'^\s*toolchain:\s*"?([^"\s#]+)"?', re.MULTILINE)
RUSTUP_ENV_RE = re.compile(r'^\s*RUSTUP_TOOLCHAIN:\s*"?([^"\s#]+)"?', re.MULTILINE)


def steps(text: str) -> list[str]:
    """Split a workflow into YAML sequence items (steps), by indentation.

    Deliberately not a regex over the whole file: an earlier version captured
    "every following indented line", which ran one step's body across the rest of
    its job and let a NEIGHBOURING step's `RUSTUP_TOOLCHAIN` excuse a drifting
    install. A step ends at the next `- ` at the same or shallower indent, or at
    the next non-blank line indented less than the step marker.
    """
    lines = text.splitlines(keepends=True)
    blocks: list[str] = []
    current: list[str] | None = None
    indent = 0
    for line in lines:
        item = LIST_ITEM_RE.match(line)
        stripped = line.strip()
        if current is not None:
            ends = (item is not None and len(item.group(1)) <= indent) or (
                stripped != ""
                and len(line) - len(line.lstrip()) <= indent
                and item is None
            )
            if ends:
                blocks.append("".join(current))
                current = None
        if item is not None:
            current = [line]
            indent = len(item.group(1))
        elif current is not None:
            current.append(line)
    if current is not None:
        blocks.append("".join(current))
    return blocks


def pinned_channel(text: str) -> str:
    """Return the channel `rust-toolchain.toml` names, rejecting a frozen one.

    A floating `nightly` is the required state. A dated channel, or an exact
    release, is the drift: it freezes the lint and borrow-checker surface as of
    one day, so every check sharpened afterwards silently stops running and the
    findings it would have made turn into debt nobody sees.
    """
    match = CHANNEL_RE.search(text)
    if match is None:
        raise SystemExit("rust-toolchain.toml: no [toolchain] channel = \"...\" found")
    channel = match.group(1)
    if channel == FLOATING_CHANNEL:
        return channel
    if DATED_CHANNEL_RE.match(channel) or RELEASE_CHANNEL_RE.match(channel):
        raise SystemExit(
            f'rust-toolchain.toml: channel = "{channel}" is frozen. It pins the\n'
            "lint and borrow-checker surface to a single day, so every check that\n"
            "gets sharper afterwards stops being a finding and becomes invisible\n"
            "debt while the gates still report green. Byte-determinism is not a\n"
            "reason to freeze: it is a property of the code -- sorted, deduplicated,\n"
            "explicitly ordered output, identities content-addressed over this\n"
            "workspace's own declared law -- and the goldens and vectors prove it on\n"
            "whatever compiler runs them. A golden that moves under a compiler bump\n"
            f'is a serializer defect to fix. Use channel = "{FLOATING_CHANNEL}".'
        )
    raise SystemExit(
        f'rust-toolchain.toml: channel = "{channel}" is not the development\n'
        "channel. Nightly clippy and rustdoc carry lints the other channels lack,\n"
        "and its default borrow checker is the stronger one; that analysis surface\n"
        "is the whole reason this workspace builds on nightly, while the `msrv` CI\n"
        "job holds the stable floor and proves no nightly-only feature crept in.\n"
        f'Use channel = "{FLOATING_CHANNEL}".'
    )


def audit(text: str, channel: str) -> list[str]:
    """Return one problem string per toolchain-install step that drifts.

    A step's toolchain may be excused by `RUSTUP_TOOLCHAIN` set on the step
    itself, on the run step that follows it, or at workflow level (which applies
    to every step in the file). Anything else is silent drift.
    """
    problems: list[str] = []
    blocks = steps(text)
    # Workflow-level `env:` sits at column 0 and applies file-wide; step- and
    # job-level `env:` are indented, so anchoring at column 0 separates them.
    #
    # The indented-line group is `[ \t][^\n]*\n`, deliberately NOT `[ \t]+[^\n]*\n`.
    # The two match exactly the same lines, but the `+` version is ambiguous — a
    # run of spaces can be split between `[ \t]+` and `[^\n]*` in exponentially
    # many ways — so on an `env:` block with no `RUSTUP_TOOLCHAIN` in it the
    # regex engine backtracks through every one of them before failing. One
    # indent character followed by "rest of line" is unambiguous and linear.
    file_escape = re.search(
        r'^env:\n(?:[ \t][^\n]*\n)*?[ \t]+RUSTUP_TOOLCHAIN:\s*"?([^"\s#]+)"?',
        text,
        re.MULTILINE,
    )
    for index, body in enumerate(blocks):
        if not ACTION_RE.search(body):
            continue
        requested = TOOLCHAIN_INPUT_RE.search(body)
        if requested is None:
            # No `toolchain:` input means the action's own default, `stable`,
            # which is never what this workspace wants stated implicitly.
            problems.append(
                "installs the action default (stable) with no explicit `toolchain:` input"
            )
            continue
        name = requested.group(1)
        if name == channel:
            continue
        following = "".join(blocks[index + 1 : index + 4])
        escape = (
            RUSTUP_ENV_RE.search(body)
            or RUSTUP_ENV_RE.search(following)
            or file_escape
        )
        if escape is None:
            problems.append(
                f"installs `{name}` but the repo pins `{channel}`, and nothing sets\n"
                "      RUSTUP_TOOLCHAIN -- `rustup default` loses to rust-toolchain.toml,\n"
                f"      so cargo would silently run `{channel}` instead of `{name}`"
            )
        elif escape.group(1) != name:
            problems.append(
                f"installs `{name}` but RUSTUP_TOOLCHAIN selects `{escape.group(1)}`"
            )
    return problems


def self_test() -> None:
    """Prove the gate is capable of failing, per the repo's self-test convention."""
    pin = FLOATING_CHANNEL
    drifting = (
        "jobs:\n  a:\n    steps:\n"
        "      - uses: dtolnay/rust-toolchain@abc # v1\n"
        "        with:\n"
        '          toolchain: "1.96"\n'
    )
    assert audit(drifting, pin), "a drifting step must be caught"
    assert not audit(
        drifting + '        env:\n          RUSTUP_TOOLCHAIN: "1.96"\n', pin
    ), "an escape on the install step itself must pass"
    assert not audit(
        drifting + "\n      - name: Check\n        run: cargo check\n"
        '        env:\n          RUSTUP_TOOLCHAIN: "1.96"\n',
        pin,
    ), "an escape on the following run step must pass"
    assert not audit(
        'env:\n  RUSTUP_TOOLCHAIN: "1.96"\n\n' + drifting, pin
    ), "a workflow-level escape must pass"
    # An `env:` block that never reaches a `RUSTUP_TOOLCHAIN` is the worst case for
    # the file-escape regex: it must fail, and it must fail in linear time. With the
    # ambiguous `[ \t]+[^\n]*` spelling this input backtracked exponentially and hung.
    # Timed rather than merely run, so a future "tidy-up" of the regex cannot quietly
    # reintroduce the blow-up while still returning the right answer.
    start = time.monotonic()
    assert audit("env:\n" + " \t\n" * 40 + drifting, pin), (
        "an env: block with no RUSTUP_TOOLCHAIN excuses nothing"
    )
    elapsed = time.monotonic() - start
    assert elapsed < 1.0, f"file-escape regex backtracked catastrophically ({elapsed:.1f}s)"
    # Regression: an earlier body regex captured every following indented line,
    # so a LATER, unrelated job's `RUSTUP_TOOLCHAIN` silently excused this drift.
    far_away = (
        drifting
        + "\n  b:\n    steps:\n      - name: Unrelated\n        run: true\n"
        "      - name: Also unrelated\n        run: true\n"
        "      - name: Still unrelated\n        run: true\n"
        "      - name: Far away\n        run: true\n"
        '        env:\n          RUSTUP_TOOLCHAIN: "1.96"\n'
    )
    assert audit(far_away, pin), "a distant job's env must NOT excuse this step"
    implicit = "jobs:\n  a:\n    steps:\n      - uses: dtolnay/rust-toolchain@abc # v1\n\n"
    assert audit(implicit, pin), "an implicit stable step must be caught"
    matching = (
        "jobs:\n  a:\n    steps:\n"
        "      - uses: dtolnay/rust-toolchain@abc # v1\n"
        "        with:\n"
        f'          toolchain: "{pin}"\n'
        "          targets: wasm32-unknown-unknown\n"
    )
    assert not audit(matching, pin), "a matching step must pass"
    # A neighbouring VALID case: two matching steps in a row, the second of which
    # would be a false positive if step splitting mis-attributed the first's
    # `with:` block. Over-refusal is as much a bug as under-refusal.
    assert not audit(matching + matching, pin), "back-to-back matching steps must pass"
    # The channel rule, in BOTH directions. A refusal is a claim too: it is not
    # enough to show the frozen spellings are rejected, the floating one this
    # workspace actually uses must be shown to pass, end to end -- accepted by
    # `pinned_channel` AND agreeing with a workflow that installs it.
    for frozen in ("nightly-2026-09-02", "beta-2026-09-02", "1.96", "1.96.0"):
        try:
            pinned_channel(f'[toolchain]\nchannel = "{frozen}"\n')
        except SystemExit:
            pass
        else:  # pragma: no cover - guards the guard
            raise AssertionError(f"frozen channel {frozen!r} must be rejected")
    for wrong in ("stable", "beta"):
        try:
            pinned_channel(f'[toolchain]\nchannel = "{wrong}"\n')
        except SystemExit:
            pass
        else:  # pragma: no cover - guards the guard
            raise AssertionError(f"non-development channel {wrong!r} must be rejected")
    floating = pinned_channel(f'[toolchain]\nchannel = "{FLOATING_CHANNEL}"\n')
    assert floating == FLOATING_CHANNEL, "a floating nightly must be accepted"
    assert not audit(matching, floating), (
        "a floating nightly with agreeing workflows must pass"
    )
    print("check-toolchain-pin.py: self-test OK")


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        self_test()
        return 0
    channel = pinned_channel(TOOLCHAIN_FILE.read_text(encoding="utf-8"))
    failures: list[str] = []
    for workflow in sorted(WORKFLOW_DIR.glob("*.y*ml")):
        text = workflow.read_text(encoding="utf-8")
        for problem in audit(text, channel):
            failures.append(f"  {workflow.relative_to(REPO)}: {problem}")
    if failures:
        print(
            f"Toolchain drift: rust-toolchain.toml pins `{channel}`, but:",
            file=sys.stderr,
        )
        print("\n".join(failures), file=sys.stderr)
        return 1
    print(f"check-toolchain-pin.py: every workflow agrees with the `{channel}` pin")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
