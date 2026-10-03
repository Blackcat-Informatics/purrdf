#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Atomically copy main and tags, retaining their exact Git object identities.

HTTPS authentication belongs in the caller's GIT_ASKPASS environment, never in
the remote URL. Local repository paths are also supported for offline checks.
"""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import time
from urllib.parse import urlsplit


class SyncError(Exception):
    """Synchronization did not establish the expected remote refs."""


def git(arguments: list[str], timeout: float) -> str:
    """Bound the entire Git process group and never print transport diagnostics.

    A server can echo credentials in diagnostics, so failed Git commands expose
    only a static failure category, stage and status. Readback proves success.
    """
    try:
        process = subprocess.Popen(
            ["git", *arguments],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            start_new_session=True,
        )
    except OSError:
        raise SyncError(f"git {arguments[0]} could not start") from None
    try:
        stdout, stderr = process.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.communicate()
        raise SyncError(f"git {arguments[0]} timed out") from None
    if process.returncode:
        categories = (
            ("atomic pushes unsupported", ("does not support --atomic push",)),
            ("authentication or permission denied",
             ("Authentication failed", "authentication failed", "Permission denied",
              "could not read Username", "could not read Password",
              "The requested URL returned error: 403",
              "The requested URL returned error: 401")),
            ("ref conflict", ("non-fast-forward", "already exists", "fetch first")),
            ("network failure", ("Failed to connect", "Could not resolve host",
                                 "unable to access", "Connection timed out")),
            ("remote rejected refs", ("remote rejected", "pre-receive hook declined")),
        )
        category = next(
            (label for label, needles in categories
             if any(needle in stderr for needle in needles)),
            "Git command failed",
        )
        raise SyncError(
            f"git {arguments[0]}: {category} (exit status {process.returncode})"
        )
    return stdout


def destination(value: str) -> str:
    """Reject credentials and option-like destinations before invoking Git."""
    if value.startswith("https://"):
        try:
            parsed = urlsplit(value)
        except ValueError:
            raise SyncError("remote must be a valid credential-free HTTPS URL") from None
        if (
            not parsed.hostname
            or not parsed.path
            or parsed.username is not None
            or parsed.password is not None
            or parsed.query
            or parsed.fragment
        ):
            raise SyncError("remote must be an HTTPS URL without credentials or query")
        return value
    path = Path(value)
    if not value.startswith("-") and path.is_dir():
        return str(path.resolve())
    raise SyncError("remote must be an HTTPS URL or an existing local repository")


def refs(output: str) -> dict[str, str]:
    result = {}
    for line in output.splitlines():
        fields = line.split("\t")
        if (
            len(fields) != 2
            or not re.fullmatch(r"(?:[0-9a-f]{40}|[0-9a-f]{64})", fields[0])
            or not fields[1].startswith("refs/")
            or fields[1] in result
        ):
            raise SyncError("Git returned an invalid ref advertisement")
        identity, name = fields
        result[name] = identity
    return result


def sync(
    remote: str,
    *,
    timeout: float | None = None,
    retry_delays: tuple[float, float] = (2, 4),
) -> dict[str, str]:
    remote = destination(remote)
    read_timeout = 45 if timeout is None else timeout
    push_timeout = 300 if timeout is None else timeout
    if git(["rev-parse", "--is-shallow-repository"], read_timeout).strip() != "false":
        raise SyncError("source repository must have complete history")
    source = refs(
        git(
            ["for-each-ref", "--format=%(objectname)%09%(refname)",
             "refs/heads/main", "refs/tags/"],
            read_timeout,
        )
    )
    expected = {
        name: identity
        for name, identity in sorted(source.items())
        if name == "refs/heads/main" or name.startswith("refs/tags/")
    }
    if "refs/heads/main" not in expected:
        raise SyncError("source repository has no refs/heads/main")
    refspecs = [f"{identity}:{name}" for name, identity in expected.items()]
    for attempt in range(3):
        try:
            git(["push", "--atomic", "--", remote, *refspecs], push_timeout)
            actual = refs(git(["ls-remote", "--refs", "--", remote], read_timeout))
            mismatches = [name for name, identity in expected.items()
                          if actual.get(name) != identity]
            if mismatches:
                raise SyncError("remote ref readback differed: " + ", ".join(mismatches))
            print(f"Verified main and {len(expected) - 1} tags at their exact object IDs.")
            return expected
        except SyncError as error:
            print(f"Sync attempt {attempt + 1}/3 failed: {error}", file=sys.stderr)
            if attempt == 2:
                raise SyncError("synchronization failed after three attempts") from error
            time.sleep(retry_delays[attempt])
    raise AssertionError("unreachable synchronization state")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("remote", help="credential-free HTTPS URL or local repository")
    arguments = parser.parse_args()
    try:
        sync(arguments.remote)
    except SyncError as error:
        print(f"Synchronization failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
