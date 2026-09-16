#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Fail if a replaced third-party dependency re-enters the workspace closure.

Once a dependency has been replaced by a first-party surface, nothing may pull
it back in — not as a runtime, build, or dev/test dependency of any workspace
member, and not transitively. ``make rdf-core-hygiene`` guards the kernel's
*normal* edges only; this gate reads ``Cargo.lock``, which records the full
resolved closure across every edge kind, so a reintroduction through a
dev-dependency or a transitive edge is caught the same way as a direct one.

The ban list names the replacement so the failure message teaches the fix.
Dependencies that are scheduled for replacement but still present (``hex``,
``petgraph``) are NOT listed here — they join the list in the change that
removes them, so this gate never lies about the present.
"""

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# package name -> the first-party replacement a reintroduction must use instead.
BANNED: dict[str, str] = {
    "oxilangtag": "purrdf_iri::langtag (RFC 5646 well-formedness)",
    "oxiri": "purrdf-iri",
    "oxsdatatypes": "purrdf-xsd",
    "oxrdf": "purrdf-core",
    "oxigraph": "the native purrdf engine",
}

PACKAGE_NAME = re.compile(r'^name = "([^"]+)"$', re.MULTILINE)


def banned_packages(lock_text: str) -> list[str]:
    """Names from the ban list that appear as resolved packages in the lock."""
    resolved = set(PACKAGE_NAME.findall(lock_text))
    return sorted(resolved & BANNED.keys())


def self_test() -> int:
    dirty = 'name = "serde"\nname = "oxilangtag"\nversion = "0.1.6"\n'
    clean = 'name = "serde"\nname = "purrdf-iri"\n'
    if banned_packages(dirty) != ["oxilangtag"]:
        print("SELF-TEST FAIL: a banned package in the lock was not flagged")
        return 1
    if banned_packages(clean):
        print("SELF-TEST FAIL: a clean lock was flagged")
        return 1
    print("OK: check-banned-deps self-test (the gate can still fail)")
    return 0


def main() -> int:
    if "--self-test" in sys.argv[1:]:
        return self_test()
    lock_text = (REPO_ROOT / "Cargo.lock").read_text(encoding="utf-8")
    offenders = banned_packages(lock_text)
    if offenders:
        for name in offenders:
            print(
                f"FAIL: `{name}` is back in Cargo.lock; it was replaced by "
                f"{BANNED[name]} and must not be reintroduced through any "
                "runtime, build, or test edge"
            )
        return 1
    print(f"OK: none of the {len(BANNED)} replaced dependencies re-entered Cargo.lock")
    return 0


if __name__ == "__main__":
    sys.exit(main())
