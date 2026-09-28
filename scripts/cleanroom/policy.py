# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""The forbidden-set policy of a clean-room implementation, and its one matcher.

A clean-room replacement of a third-party crate is only as clean as what its author
was able to read while writing it. This module owns the definition of "forbidden"
and the single function that decides whether one agent tool call touches it. Three
programs share it so they cannot disagree:

* ``transcript_audit.py`` replays a finished session's transcript through it;
* ``guard_hook.py`` runs it as a pre-tool-use hook, before the call;
* ``deny_settings.py`` renders the path part of it as permission deny rules.

POLICY SCHEMA (TOML; every table and key is optional, unknown keys are an error)::

    [paths]
    # Globs over filesystem paths. `~` expands to the home directory of the process
    # evaluating the policy. `*` and `?` stay within one path component, `**` spans
    # any number of them, and a trailing `/**` also matches the directory itself.
    deny = ["~/.cargo/registry/**", "~/.rustup/toolchains/*/lib/rustlib/src/**"]

    [bash]
    # Substrings refused anywhere in a Bash command line.
    deny_substrings = [".cargo/registry", "rustlib/src", "git show", "git cat-file"]

    [git]
    # Commit ids (at least 7 hex digits). A hex run of 7+ digits in a Bash command
    # matches when either is a prefix of the other, so an abbreviation of a
    # forbidden commit is refused and so is a longer spelling of a forbidden prefix.
    deny_shas = ["0123abc"]
    # Branch names refused as a whole token in a Bash command.
    deny_branches = ["upstream-port"]

    [web]
    # Hosts WebFetch may reach. A host matches itself and its subdomains. Every
    # other host is a violation, and WebSearch is a violation unconditionally,
    # because a search result page is unbounded third-party text.
    allow_hosts = ["rfc-editor.org"]

WHAT IS MATCHED, per tool:

* ``Read``/``Edit``/``Write``/``NotebookEdit``: the file path, against ``[paths]``.
* ``Grep``/``Glob``: the search root (``path``, joined with the literal leading
  directories of a ``Glob`` pattern). It is a violation when the root lies inside a
  forbidden glob OR is an ancestor of one, because a recursive search from an
  ancestor reads the forbidden tree too.
* ``Bash``: the command, against ``[bash]`` substrings, ``[git]`` commit ids and
  branch names, and every path-shaped word against ``[paths]``.
* ``WebFetch``: the URL's host against ``[web]``; ``WebSearch``: always.

Relative paths are resolved against the session's working directory when it is
known. A path is checked both lexically normalized and with symlinks resolved, so a
symlink into a forbidden tree does not launder it.

A Bash command line is walked as ``&&``/``;``-separated segments, left to right. A
segment that is `cd <dir>` updates the directory later segments resolve relative
words against (an absolute *dir* is used as-is; a relative one is resolved against
whatever directory is currently in effect). `cd` with no argument or `cd -` makes the
effective directory unknown again, and later relative words fall back to the
session's working directory rather than going unresolved. A word in the same segment
as a `cd` is not affected by that `cd` -- the directory only changes for segments
that follow.
"""

from __future__ import annotations

import os
import re
import shlex
import tomllib
from dataclasses import dataclass, field
from pathlib import Path
from urllib.parse import urlsplit

AUDITED_TOOLS = frozenset(
    {"Read", "Grep", "Glob", "Edit", "Write", "NotebookEdit", "WebFetch", "WebSearch", "Bash"}
)
SCHEMA: dict[str, frozenset[str]] = {
    "paths": frozenset({"deny"}),
    "bash": frozenset({"deny_substrings"}),
    "git": frozenset({"deny_shas", "deny_branches"}),
    "web": frozenset({"allow_hosts"}),
}
HEX_RUN = re.compile(r"(?<![0-9A-Fa-f])[0-9A-Fa-f]{7,64}(?![0-9A-Fa-f])")
SHA_SPEC = re.compile(r"^[0-9a-f]{7,64}$")


class PolicyError(ValueError):
    """The policy document does not follow the schema."""


@dataclass(frozen=True)
class Violation:
    """One tool call that touches the forbidden set."""

    tool: str
    rule: str
    subject: str


@dataclass
class Policy:
    """A parsed, validated forbidden-set policy."""

    path_globs: list[str] = field(default_factory=list)
    bash_substrings: list[str] = field(default_factory=list)
    shas: list[str] = field(default_factory=list)
    branches: list[str] = field(default_factory=list)
    allow_hosts: list[str] = field(default_factory=list)
    home: str = field(default_factory=lambda: os.path.expanduser("~"))

    def expanded_globs(self) -> list[str]:
        """The path globs with ``~`` expanded against this policy's home."""
        return [expand_home(glob, self.home) for glob in self.path_globs]


def _string_list(table: dict, key: str, where: str) -> list[str]:
    value = table.get(key, [])
    if not isinstance(value, list) or not all(isinstance(item, str) and item for item in value):
        raise PolicyError(f"{where}.{key} must be a list of non-empty strings")
    return list(value)


def parse_policy(document: dict, home: str | None = None) -> Policy:
    """Validate a decoded policy document and return the ``Policy`` it states."""
    unknown = sorted(set(document) - set(SCHEMA))
    if unknown:
        raise PolicyError(f"unknown policy table(s): {unknown}")
    tables: dict[str, dict] = {}
    for name, keys in SCHEMA.items():
        table = document.get(name, {})
        if not isinstance(table, dict):
            raise PolicyError(f"[{name}] must be a table")
        extra = sorted(set(table) - keys)
        if extra:
            raise PolicyError(f"unknown key(s) in [{name}]: {extra}")
        tables[name] = table
    shas = [sha.lower() for sha in _string_list(tables["git"], "deny_shas", "git")]
    for sha in shas:
        if not SHA_SPEC.match(sha):
            raise PolicyError(f"git.deny_shas entry {sha!r} is not 7 to 64 hex digits")
    policy = Policy(
        path_globs=_string_list(tables["paths"], "deny", "paths"),
        bash_substrings=_string_list(tables["bash"], "deny_substrings", "bash"),
        shas=shas,
        branches=_string_list(tables["git"], "deny_branches", "git"),
        allow_hosts=[host.lower() for host in _string_list(tables["web"], "allow_hosts", "web")],
    )
    if home is not None:
        policy.home = home
    return policy


def load_policy(path: Path, home: str | None = None) -> Policy:
    """Read and validate the policy TOML at *path*."""
    try:
        document = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise PolicyError(f"cannot read policy {path}: {error}") from error
    return parse_policy(document, home)


def expand_home(text: str, home: str) -> str:
    """Expand a leading ``~`` or ``~/`` against *home*."""
    if text == "~":
        return home
    if text.startswith("~/"):
        return home.rstrip("/") + text[1:]
    return text


def glob_regex(glob: str) -> re.Pattern[str]:
    """Compile a path glob (``*``, ``?``, ``**``, ``[...]``) to an anchored regex."""
    out: list[str] = []
    index = 0
    if glob.endswith("/**"):
        body, tail = glob[:-3], "(?:/.*)?"
    else:
        body, tail = glob, ""
    while index < len(body):
        char = body[index]
        if body.startswith("**/", index):
            out.append("(?:.*/)?")
            index += 3
        elif body.startswith("**", index):
            out.append(".*")
            index += 2
        elif char == "*":
            out.append("[^/]*")
            index += 1
        elif char == "?":
            out.append("[^/]")
            index += 1
        elif char == "[":
            close = body.find("]", index + 1)
            if close == -1:
                out.append(re.escape(char))
                index += 1
            else:
                inner = body[index + 1 : close]
                if inner.startswith("!"):
                    inner = "^" + inner[1:]
                out.append(f"[{inner}]")
                index = close + 1
        else:
            out.append(re.escape(char))
            index += 1
    return re.compile("^" + "".join(out) + tail + "$")


def literal_prefix(glob: str) -> str:
    """The leading path components of *glob* that contain no wildcard."""
    parts = []
    for part in glob.split("/"):
        if any(char in part for char in "*?["):
            break
        parts.append(part)
    return "/".join(parts) or "/"


def path_forms(raw: str, cwd: str | None, home: str) -> list[str]:
    """The lexical and symlink-resolved absolute spellings of a tool's path argument."""
    text = expand_home(raw.strip(), home)
    if not os.path.isabs(text):
        if cwd is None:
            return [os.path.normpath(text)]
        text = os.path.join(cwd, text)
    lexical = os.path.normpath(text)
    forms = [lexical]
    resolved = os.path.realpath(lexical)
    if resolved != lexical:
        forms.append(resolved)
    return forms


def path_violation(raw: str, policy: Policy, cwd: str | None) -> str | None:
    """The deny glob *raw* falls inside, or ``None``."""
    for form in path_forms(raw, cwd, policy.home):
        for original, glob in zip(policy.path_globs, policy.expanded_globs(), strict=True):
            if glob_regex(glob).match(form):
                return f"paths.deny {original}"
    return None


def search_root_violation(root: str, policy: Policy, cwd: str | None) -> str | None:
    """A search from *root* reaches a forbidden tree: inside one, or an ancestor of one."""
    inside = path_violation(root, policy, cwd)
    if inside is not None:
        return inside
    for form in path_forms(root, cwd, policy.home):
        base = form.rstrip("/") + "/"
        for original, glob in zip(policy.path_globs, policy.expanded_globs(), strict=True):
            prefix = literal_prefix(glob)
            if prefix == form or prefix.startswith(base):
                return f"paths.deny {original} (search root contains it)"
    return None


SEGMENT_SEPARATORS = ("&&", ";")


def _segments(command: str) -> list[list[str]]:
    """*command* tokenized with ``shlex`` and split into top-level ``&&``/``;`` runs."""
    try:
        words = shlex.split(command, comments=False, posix=True)
    except ValueError:
        words = command.split()
    segments: list[list[str]] = [[]]
    for word in words:
        if word in SEGMENT_SEPARATORS:
            segments.append([])
        else:
            segments[-1].append(word)
    return segments


def _path_words(words: list[str]) -> list[str]:
    found = []
    for word in words:
        # `--file=/x` and `VAR=/x` carry a path after the `=`.
        for piece in [word, *word.split("=")[1:]]:
            if piece.startswith(("/", "~", "./", "../")):
                found.append(piece)
    return found


def _cd_target(words: list[str]) -> str | None:
    """The directory argument of a leading ``cd``, or ``None`` when it has none."""
    if not words or words[0] != "cd":
        return None
    args = [word for word in words[1:] if not word.startswith("-")]
    if not args or args[0] == "-":
        return None
    return args[0]


def _next_cwd(words: list[str], effective: str | None, session_cwd: str | None, policy: Policy) -> str | None:
    """The directory in effect for segments after *words*, given it currently is *effective*."""
    if not words or words[0] != "cd":
        return effective
    target = _cd_target(words)
    if target is None:
        # No argument, or `cd -`: the destination is unknown, not the session cwd.
        return None
    resolve_against = effective if effective is not None else session_cwd
    return path_forms(target, resolve_against, policy.home)[0]


def bash_violations(command: str, policy: Policy, cwd: str | None) -> list[str]:
    """Every rule a Bash command line breaks."""
    rules: list[str] = []
    expanded = command.replace("$HOME", policy.home).replace("${HOME}", policy.home)
    for needle in policy.bash_substrings:
        if needle in command or needle in expanded:
            rules.append(f"bash.deny_substrings {needle}")
    for run in HEX_RUN.findall(command):
        token = run.lower()
        for sha in policy.shas:
            if sha.startswith(token) or token.startswith(sha):
                rules.append(f"git.deny_shas {sha}")
    for branch in policy.branches:
        pattern = rf"(?<![\w./-]){re.escape(branch)}(?![\w/-])"
        if re.search(pattern, command):
            rules.append(f"git.deny_branches {branch}")
    effective_cwd = cwd
    for segment in _segments(expanded):
        # An unknown effective directory (after `cd` with no argument) resolves
        # conservatively against the session cwd, same as before this segment tracking
        # existed, rather than going unresolved.
        resolve_cwd = effective_cwd if effective_cwd is not None else cwd
        for word in _path_words(segment):
            hit = path_violation(word, policy, resolve_cwd)
            if hit is not None:
                rules.append(hit)
        effective_cwd = _next_cwd(segment, effective_cwd, cwd, policy)
    return sorted(set(rules))


def host_allowed(url: str, policy: Policy) -> bool:
    """True when *url*'s host is an allowlisted host or a subdomain of one."""
    host = (urlsplit(url).hostname or "").lower()
    if not host:
        return False
    return any(host == allowed or host.endswith("." + allowed) for allowed in policy.allow_hosts)


class ShapeError(ValueError):
    """A known tool's input lacks the field its matcher reads."""


def _required(tool_input: dict, key: str, tool: str) -> str:
    value = tool_input.get(key)
    if not isinstance(value, str):
        raise ShapeError(f"{tool} input has no string {key!r}")
    return value


def subject_of(tool: str, tool_input: dict) -> str:
    """The text that identifies what a tool call touched, for a report excerpt."""
    for key in ("command", "file_path", "notebook_path", "url", "query", "path", "pattern"):
        value = tool_input.get(key)
        if isinstance(value, str) and value:
            return value
    return tool


def check_call(tool: str, tool_input: object, policy: Policy, cwd: str | None) -> list[Violation]:
    """Every violation one tool call commits. Unaudited tools return none.

    Raises ``ShapeError`` when an audited tool's input does not carry the field its
    rule reads, so a changed input shape surfaces instead of passing unexamined.
    """
    if tool not in AUDITED_TOOLS:
        return []
    if not isinstance(tool_input, dict):
        raise ShapeError(f"{tool} input is not an object")
    rules: list[str] = []
    if tool in ("Read", "Edit", "Write"):
        hit = path_violation(_required(tool_input, "file_path", tool), policy, cwd)
        rules.extend([hit] if hit else [])
    elif tool == "NotebookEdit":
        hit = path_violation(_required(tool_input, "notebook_path", tool), policy, cwd)
        rules.extend([hit] if hit else [])
    elif tool == "Grep":
        root = tool_input.get("path") or cwd or "."
        if not isinstance(root, str):
            raise ShapeError("Grep input path is not a string")
        hit = search_root_violation(root, policy, cwd)
        rules.extend([hit] if hit else [])
    elif tool == "Glob":
        pattern = _required(tool_input, "pattern", tool)
        base = tool_input.get("path") or cwd or "."
        if not isinstance(base, str):
            raise ShapeError("Glob input path is not a string")
        prefix = literal_prefix(expand_home(pattern, policy.home))
        if pattern.startswith(("/", "~")):
            root = prefix
        elif prefix in ("", "/"):
            root = base
        else:
            root = os.path.join(base, prefix)
        hit = search_root_violation(root, policy, cwd)
        rules.extend([hit] if hit else [])
    elif tool == "Bash":
        rules.extend(bash_violations(_required(tool_input, "command", tool), policy, cwd))
    elif tool == "WebFetch":
        url = _required(tool_input, "url", tool)
        if not host_allowed(url, policy):
            rules.append(f"web.allow_hosts (host {urlsplit(url).hostname!r} is not allowlisted)")
    elif tool == "WebSearch":
        rules.append("WebSearch is always forbidden")
    subject = subject_of(tool, tool_input)
    return [Violation(tool=tool, rule=rule, subject=subject) for rule in rules]
