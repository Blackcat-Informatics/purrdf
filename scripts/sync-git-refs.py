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


# These are fixed labels, not excerpts from a server's response. Unknown values
# are deliberately omitted. CURLcode identities: curl.se/libcurl/c/libcurl-errors.html.
HTTP_STATUSES = {
    status: f"HTTP status {status} observed"
    for status in (100, 101, 200, 201, 202, 204, 301, 302, 303, 307, 308,
                   400, 401, 403, 404, 405, 408, 409, 411, 413, 429,
                   500, 501, 502, 503, 504)
}
HTTP_PROTOCOLS = {
    protocol: f"HTTP/{protocol} observed" for protocol in ("1.0", "1.1", "2", "3")
}
HTTP_FINAL_STATUSES = {
    status: f"latest HTTP request final response {status} observed"
    for status in HTTP_STATUSES if status >= 200
}
PACK_WRITE_PROGRESS = {
    percent: f"last pack write progress {percent}%" for percent in range(101)
}
CURL_ERRORS = {
    "0": "libcurl success (0)",
    "5": "libcurl proxy resolution failed (5)",
    "6": "libcurl host resolution failed (6)",
    "7": "libcurl connection failed (7)",
    "16": "libcurl HTTP/2 framing error (16)",
    "18": "libcurl partial transfer (18)",
    "22": "libcurl HTTP error response (22)",
    "23": "libcurl write error (23)",
    "25": "libcurl upload failed (25)",
    "26": "libcurl read error (26)",
    "27": "libcurl out of memory (27)",
    "28": "libcurl operation timed out (28)",
    "35": "libcurl TLS connection failed (35)",
    "43": "libcurl bad function argument (43)",
    "47": "libcurl redirect limit exceeded (47)",
    "52": "libcurl empty server response (52)",
    "55": "libcurl send failed (55)",
    "56": "libcurl receive failed (56)",
    "60": "libcurl peer verification failed (60)",
    "65": "libcurl upload rewind failed (65)",
    "67": "libcurl login denied (67)",
    "77": "libcurl certificate file error (77)",
    "90": "libcurl public key mismatch (90)",
    "91": "libcurl certificate status invalid (91)",
    "92": "libcurl HTTP/2 stream error (92)",
    "94": "libcurl authentication error (94)",
    "95": "libcurl HTTP/3 error (95)",
    "96": "libcurl QUIC connection failed (96)",
    "97": "libcurl proxy handshake failed (97)",
    "98": "libcurl client certificate required (98)",
}


def transport_summary(stderr: str) -> str:
    """Translate recognized transport milestones to a closed label vocabulary."""
    labels = []
    latest_request = None
    latest_response = None
    pack_progress = None

    def add(label: str | None) -> None:
        if label is not None and label not in labels:
            labels.append(label)

    for line in stderr.splitlines():
        trace = re.match(r"(?:[0-9]{2}:[0-9]{2}:[0-9]{2}(?:\.[0-9]+)? +)?"
                         r"http\.c:[0-9]+ +(.+)$", line)
        payload = trace[1] if trace else ""
        # RFC 9110 sections 9.1 and 5.6.2: every method is a case-sensitive
        # token. Even an unrecognized method starts a new response context.
        request = re.fullmatch(r"=> Send header: ([!#$%&'*+.^_`|~0-9A-Za-z-]+) "
                               r"\S+ HTTP/([0-9](?:\.[0-9])?)", payload)
        if request:
            latest_request = {
                "GET": "latest HTTP request is GET",
                "POST": "latest HTTP request is POST",
                "CONNECT": "latest HTTP request is CONNECT",
            }.get(request[1], "latest HTTP request has an unrecognized method")
            latest_response = None
            add(HTTP_PROTOCOLS.get(request[2]))
        header = re.match(r"<= Recv header: HTTP/(1\.0|1\.1|2|3) ([0-9]{3})(?:\s|$)",
                          payload)
        if header:
            add(HTTP_PROTOCOLS.get(header[1]))
            add(HTTP_STATUSES.get(int(header[2])))
            if latest_request is not None and int(header[2]) >= 200:
                latest_response = HTTP_FINAL_STATUSES.get(int(header[2]))
        if re.match(r"=> Send header: POST /\S*/git-receive-pack HTTP/(?:1\.0|1\.1|2|3)(?:\s|$)",
                    payload):
            add("receive-pack POST observed")
        if payload.startswith(("== Info: upload completely sent off:",
                               "== Info: We are completely uploaded and fine")):
            add("HTTP upload completion observed")
        rpc = re.match(r"error: RPC failed; (?:HTTP ([0-9]{3}) )?curl ([0-9]+)(?:\s|$)",
                       line)
        if rpc:
            if rpc[1] is not None:
                add(HTTP_STATUSES.get(int(rpc[1])))
            add(CURL_ERRORS.get(rpc[2]))
        for prefix, label in (
            ("Enumerating objects:", "pack enumeration observed"),
            ("Counting objects:", "pack counting observed"),
            ("Compressing objects:", "pack compression observed"),
            ("Writing objects:", "pack writing observed"),
        ):
            if line.startswith(prefix):
                add(label)
        if re.match(r"Writing objects: +100% \([0-9]+/[0-9]+\),.*done\.$", line):
            add("pack write completion observed")
        writing = re.match(r"Writing objects: +([0-9]{1,3})% \([0-9]+/[0-9]+\)", line)
        if writing:
            pack_progress = PACK_WRITE_PROGRESS.get(int(writing[1]))
    add(pack_progress)
    add(latest_request)
    if latest_request is not None:
        add(latest_response or "latest HTTP request has no recognized final response")
    return "; ".join(labels) or "no recognized transport milestones"


def git(arguments: list[str], timeout: float) -> str:
    """Bound the entire Git process group and print only allowlisted milestones.

    A server can echo credentials in diagnostics, so failed Git commands expose
    only fixed labels, stage and status. Exact ref readback alone proves success.
    """
    network = arguments[0] in ("push", "ls-remote")
    environment = dict(os.environ)
    if network:
        # Force traces into the private stderr pipe rather than an inherited file.
        # Redaction and omitted payloads are additional defenses; the allowlist is
        # the output boundary. See git-scm.com/docs/git, GIT_TRACE_CURL.
        environment.update(GIT_TRACE_CURL="1", GIT_TRACE_CURL_NO_DATA="1",
                           GIT_TRACE_REDACT="1")
    started = time.monotonic()
    try:
        process = subprocess.Popen(
            ["git", *arguments],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="replace",
            start_new_session=True,
            env=environment,
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
        _, stderr = process.communicate()
        if network:
            print(f"git {arguments[0]} transport after {time.monotonic() - started:.1f} seconds: "
                  f"{transport_summary(stderr)}",
                  file=sys.stderr, flush=True)
        raise SyncError(f"git {arguments[0]} timed out after {timeout:g} seconds") from None
    if network:
        print(f"git {arguments[0]} transport after {time.monotonic() - started:.1f} seconds: "
              f"{transport_summary(stderr)}",
              file=sys.stderr, flush=True)
    if process.returncode:
        categories = (
            ("atomic pushes unsupported", ("does not support --atomic push",)),
            ("credential prompt refused",
             ("could not read Username", "could not read Password",
              "unable to read askpass response")),
            ("authentication or permission rejected",
             ("Authentication failed", "authentication failed", "Permission denied",
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
    print(f"Source snapshot: main and {len(expected) - 1} tags.", flush=True)
    refspecs = [f"{identity}:{name}" for name, identity in expected.items()]
    for attempt in range(3):
        try:
            print(f"Destination preflight {attempt + 1}/3: reading refs "
                  f"({read_timeout:g} second deadline).", flush=True)
            actual = refs(git(["ls-remote", "--refs", "--", remote], read_timeout))
            print(f"Destination preflight read {len(actual)} refs.", flush=True)
            if all(actual.get(name) == identity for name, identity in expected.items()):
                print(f"Verified main and {len(expected) - 1} tags at their exact object IDs.",
                      flush=True)
                return expected
            print(f"Atomic push {attempt + 1}/3: main and {len(expected) - 1} tags "
                  f"({push_timeout:g} second deadline).", flush=True)
            git(["push", "--atomic", "--progress", "--", remote, *refspecs], push_timeout)
            print(f"Destination readback: reading refs ({read_timeout:g} second deadline).",
                  flush=True)
            actual = refs(git(["ls-remote", "--refs", "--", remote], read_timeout))
            mismatches = [name for name, identity in expected.items()
                          if actual.get(name) != identity]
            if mismatches:
                raise SyncError(f"remote ref readback differed for {len(mismatches)} refs")
            print(f"Verified main and {len(expected) - 1} tags at their exact object IDs.",
                  flush=True)
            return expected
        except SyncError as error:
            print(f"Sync attempt {attempt + 1}/3 failed: {error}", file=sys.stderr, flush=True)
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
