#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Hold ``helpers-ledger.toml`` true: one home per job, every exemption reasoned.

The ledger names, for each job the workspace implements once, its home, what it
replaces, and the sanctioned second implementations (variants). This gate runs
``crates/helper-census`` over the workspace's shipping Rust and fails on:

* a ``home`` or ``entry_points`` path that does not resolve to a defined item;
* a variant whose symbol does not resolve, or does not live in its ``file``;
* a variant whose ``anchor`` does not resolve to a documented item;
* a ``vectors`` file that does not exist, or a ``bench`` id that is not a bench
  target of its package;
* a ``sites`` id missing from ``scripts/simd-asm-manifest.toml``, or one that
  measures a symbol outside the home crate;
* a ``replaces_external`` package that ``check-banned-deps.py`` does not ban;
* a STALE row: a variant that no longer matches what it is exempt from, a
  ``body:``/``shim:`` fingerprint that matches nothing, or a job marked
  ``enforced = false`` that has no copies left;
* for an ``enforced`` job, a forbidden match outside the home crate that is not a
  variant.

A job may also forbid a rule another gate computes (``DELEGATED_RULES``):
``rule:raw-hash-domain`` and ``rule:shared-hash-domain`` are
``check-hash-domains.py``'s. The census accepts the ids and reports nothing for
them; this gate adds that script's hits to the job as matches outside the home
before judging it. A variant row whose detector is one of that script's rules
(``shared-hash-domain``, ``undomained-digest``) sanctions the hit it names and is
STALE when the script no longer matches it.

The census computes two literal rules itself, over shipping code:
``rule:vocabulary-literal`` (a string literal equal to or starting with a
namespace the job's home and entry-point modules declare as a ``NS``/``*_NS``
constant, outside those modules; an embedded Turtle/SPARQL document is exempt)
and ``rule:home-literal`` (a string literal equal to one the home item's own
bodies spell, outside the home item). The self-test drives both over a seeded
workspace.

It also enforces the cross-package include rule over every first-party ``.rs``
file, tests and benches included: a ``#[path = "…"]`` attribute whose resolved
path leaves the declaring crate's own directory compiles one crate's source into
another — a second copy the compiler cannot see as one.

The census's reading of the ledger is compared with ``tomllib``'s on every run,
so the two readers can never disagree about what the ledger says.

``--census JOB`` and ``--count JOB`` pass through to the census: the listing of a
job's matches and its ``copies=<n>``.
"""

from __future__ import annotations

import importlib.util
import json
import os
import re
import subprocess
import sys
import tempfile
import tomllib
from collections.abc import Callable
from pathlib import Path, PurePosixPath
from types import ModuleType

REPO_ROOT = Path(__file__).resolve().parent.parent
LEDGER_PATH = REPO_ROOT / "helpers-ledger.toml"
MANIFEST_PATH = REPO_ROOT / "scripts" / "simd-asm-manifest.toml"
PATH_ATTRIBUTE = re.compile(r'#\[path\s*=\s*"([^"\\]+)"\s*\]')


def load_script(file_name: str, module_name: str) -> ModuleType:
    """A sibling script as a module (script names carry hyphens)."""
    path = Path(__file__).resolve().parent / file_name
    spec = importlib.util.spec_from_file_location(module_name, path)
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules.setdefault(module_name, module)
    spec.loader.exec_module(module)
    return module


BANNED_DEPS = load_script("check-banned-deps.py", "check_banned_deps")
IRI_GATE = load_script("check-iri-resolver-singleton.py", "check_iri_resolver_singleton")
HASH_DOMAINS = load_script("check-hash-domains.py", "check_hash_domains")

# Census rule ids another gate computes (crates/helper-census accepts them and
# reports nothing of its own for them).
DELEGATED_RULES = (f"rule:{HASH_DOMAINS.RULE_ID}", f"rule:{HASH_DOMAINS.SHARED_RULE_ID}")


class CensusError(RuntimeError):
    """helper-census could not be run or answered nonsense."""


def run_census(*arguments: str, root: Path = REPO_ROOT) -> str:
    """Run helper-census over ``root`` (this repository by default) and return
    its stdout."""
    command = ["cargo", "run", "-q", "--locked", "-p", "helper-census", "--", "--root", str(root), *arguments]
    completed = subprocess.run(command, cwd=REPO_ROOT, capture_output=True, text=True, check=False)
    if completed.returncode != 0:
        raise CensusError(f"`{' '.join(command)}` failed:\n{completed.stderr.strip()}")
    return completed.stdout


# ---------------------------------------------------------------------------
# The cross-package include rule.


def strip_line_comment(line: str) -> str:
    """The line with a trailing ``//`` comment removed (not inside a string)."""
    in_string = False
    escaped = False
    for index, char in enumerate(line):
        if escaped:
            escaped = False
        elif char == "\\":
            escaped = True
        elif char == '"':
            in_string = not in_string
        elif not in_string and line.startswith("//", index):
            return line[:index]
    return line


def crate_dir_of(relative_file: str, manifests: set[str]) -> str | None:
    """The nearest ancestor directory of ``relative_file`` holding a Cargo.toml."""
    parts = PurePosixPath(relative_file).parts[:-1]
    for depth in range(len(parts), 0, -1):
        candidate = "/".join(parts[:depth])
        if f"{candidate}/Cargo.toml" in manifests:
            return candidate
    return None


def normalise(path: PurePosixPath) -> str | None:
    """``path`` with ``.`` and ``..`` folded; ``None`` if it climbs above the root."""
    parts: list[str] = []
    for part in path.parts:
        if part == "..":
            if not parts:
                return None
            parts.pop()
        elif part not in ("", "."):
            parts.append(part)
    return "/".join(parts)


def path_include_findings(files: dict[str, str], manifests: set[str]) -> list[str]:
    """Every ``#[path]`` attribute in ``files`` (repo-relative path -> text) whose
    target leaves the declaring file's crate directory."""
    findings: list[str] = []
    for relative, text in sorted(files.items()):
        crate_dir = crate_dir_of(relative, manifests)
        for number, line in enumerate(text.splitlines(), start=1):
            code = strip_line_comment(line)
            if code.lstrip().startswith(("*", "/*")):
                continue
            for match in PATH_ATTRIBUTE.finditer(code):
                target = normalise(PurePosixPath(relative).parent / match.group(1))
                inside = (
                    crate_dir is not None
                    and target is not None
                    and (target == crate_dir or target.startswith(crate_dir + "/"))
                )
                if not inside:
                    findings.append(
                        f"FAIL: {relative}:{number}: #[path = \"{match.group(1)}\"] resolves to "
                        f"{target or 'a path above the repository'}, outside its crate "
                        f"{crate_dir or '(none)'}; one crate's source compiled into another is a "
                        "second copy — publish the shared code from one crate and depend on it"
                    )
    return findings


def tracked_rust_files() -> tuple[dict[str, str], set[str]]:
    """Every tracked or new first-party ``.rs`` file and ``Cargo.toml`` under
    ``crates/`` and ``bindings/``."""
    listing = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "--", "crates", "bindings"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.splitlines()
    files = {
        path: (REPO_ROOT / path).read_text(encoding="utf-8")
        for path in listing
        if path.endswith(".rs") and (REPO_ROOT / path).is_file()
    }
    manifests = {path for path in listing if path.endswith("Cargo.toml")}
    return files, manifests


# ---------------------------------------------------------------------------
# The ledger rules, as pure functions over the ledger, the census index and the
# repository facts, so the self-test can drive every rule with fixtures.


def bench_targets(metadata: dict) -> set[str]:
    """``package:target`` for every bench target in the workspace."""
    members = set(metadata["workspace_members"])
    return {
        f"{package['name']}:{target['name']}"
        for package in metadata["packages"]
        if package["id"] in members
        for target in package["targets"]
        if "bench" in target["kind"]
    }


def site_crates(manifest: dict) -> dict[str, set[str]]:
    """Site id -> the crates its measures name."""
    return {site["id"]: {measure["crate"] for measure in site.get("measure", [])} for site in manifest.get("site", [])}


def ledger_findings(
    ledger: dict,
    index: dict,
    *,
    exists: Callable[[str], bool],
    benches: set[str],
    sites: dict[str, set[str]],
    banned: set[str],
    iri_matched: set[tuple[str, str]],
    iri_rules: set[str],
    hash_matched: set[tuple[str, str, str]] = frozenset(),
    hash_rules: set[str] = frozenset(),
) -> list[str]:
    """Every ledger rule's failures."""
    findings: list[str] = [f"FAIL: helper-census could not read {error}" for error in index.get("errors", [])]
    symbols = index["symbols"]
    grouped = set(index["grouped_variants"])
    for job in ledger.get("job", []):
        job_id = job["id"]
        census = index["jobs"].get(job_id)

        def symbol(path: str) -> dict | None:
            return symbols.get(path)

        for key, path in [("home", job["home"])] + [("entry_points", entry) for entry in job["entry_points"]]:
            if symbol(path) is None:
                findings.append(f"FAIL: job `{job_id}`: {key} `{path}` does not resolve to a defined item")
        if census is None:
            continue
        home_crate = census["home"].split("::", 1)[0]
        for vector in job["vectors"]:
            if not exists(vector):
                findings.append(f"FAIL: job `{job_id}`: vectors file `{vector}` does not exist")
        for bench in job["bench"]:
            if bench not in benches:
                findings.append(f"FAIL: job `{job_id}`: bench `{bench}` is not a `package:target` bench target")
        for site in job["sites"]:
            if site not in sites:
                findings.append(f"FAIL: job `{job_id}`: site `{site}` is not in scripts/simd-asm-manifest.toml")
            elif sites[site] != {home_crate}:
                outside = ", ".join(sorted(sites[site] - {home_crate}))
                findings.append(
                    f"FAIL: job `{job_id}`: site `{site}` measures symbols in {outside or 'no crate'}, "
                    f"not in the home crate `{home_crate}`; move the [[site]] row with the kernel"
                )
        for package in job["replaces_external"]:
            if package not in banned:
                findings.append(
                    f"FAIL: job `{job_id}`: replaces_external `{package}` is not banned in "
                    "scripts/check-banned-deps.py; ban it or drop the claim"
                )
        for variant in job.get("variant", []):
            where = f"job `{job_id}` variant `{variant['symbol']}`"
            resolved = symbol(variant["symbol"])
            if resolved is None:
                findings.append(f"FAIL: {where}: the symbol does not resolve to a defined item")
            elif resolved["file"] != variant["file"]:
                findings.append(f"FAIL: {where}: defined in {resolved['file']}, not in {variant['file']}")
            anchor = symbol(variant["anchor"])
            if anchor is None:
                findings.append(f"FAIL: {where}: anchor `{variant['anchor']}` does not resolve")
            elif not anchor["documented"]:
                findings.append(f"FAIL: {where}: anchor `{variant['anchor']}` carries no documentation")
            detector = variant["detector"]
            if detector == "isomorphic":
                stale = resolved is None or resolved["symbol"] not in grouped
            elif detector == "forbidden":
                # A census rule hit is named by its file and enclosing items; the
                # variant it matches is the one defined in that file under that name.
                item = variant["symbol"].rsplit("::", 1)[-1]
                stale = not any(
                    found["variant"]
                    and (
                        resolved is None
                        or found["symbol"] == resolved["symbol"]
                        or (found["file"] == variant["file"] and found["symbol"].rsplit("::", 1)[-1] == item)
                    )
                    for found in census["matches"]
                )
            elif detector in iri_rules:
                stale = (variant["file"], detector) not in iri_matched
            elif detector in hash_rules:
                stale = (variant["file"], detector, HASH_DOMAINS.item_name(variant["symbol"])) not in hash_matched
            else:
                findings.append(f"FAIL: {where}: unknown detector `{detector}`")
                continue
            if stale:
                findings.append(
                    f"FAIL: {where}: STALE — it no longer matches the `{detector}` detector; remove the row"
                )
        live = {reason for found in census["matches"] for reason in found["reasons"]}
        for fingerprint in job["forbidden"]["fingerprints"]:
            if fingerprint.startswith(("body:", "shim:")) and f"fingerprint {fingerprint}" not in live:
                findings.append(
                    f"FAIL: job `{job_id}`: forbidden fingerprint `{fingerprint}` matches nothing (STALE)"
                )
        if not job["enforced"] and census["copies"] == 0:
            findings.append(
                f"FAIL: job `{job_id}`: no copies are left outside the home (STALE); set `enforced = true`"
            )
        if job["enforced"]:
            for found in census["matches"]:
                if not found["in_home"] and not found["variant"]:
                    findings.append(
                        f"FAIL: job `{job_id}`: {found['symbol']} ({found['file']}:{found['line']}) re-derives "
                        f"the job outside its home `{census['home']}`: {', '.join(found['reasons'])}"
                    )
    return findings


def merge_delegated(ledger: dict, index: dict, hits: dict[str, list[tuple[str, int, str, str]]]) -> None:
    """Add each delegated rule's ``(file, line, item, detail)`` hits to every job
    naming the rule, as matches outside the home (a rule forbids its pattern
    everywhere, as the census's own rules do) and as copies. A hit a variant row
    of the job names — detector the rule's id, same file and item — is that
    variant's."""
    for job in ledger.get("job", []):
        census = index["jobs"].get(job["id"])
        if census is None:
            continue
        sanctioned = {
            (f"rule:{variant['detector']}", variant["file"], HASH_DOMAINS.item_name(variant["symbol"]))
            for variant in job.get("variant", [])
        }
        for rule in job["forbidden"]["fingerprints"]:
            for file, line, item, detail in hits.get(rule, []):
                census["matches"].append(
                    {
                        "symbol": f"{file}::{item}" if item else file,
                        "file": file,
                        "line": line,
                        "reasons": [f"{rule}: {detail}"],
                        "in_home": False,
                        "variant": (rule, file, item) in sanctioned,
                    }
                )
                census["copies"] += 1


# ---------------------------------------------------------------------------
# Self-test.

# A seeded workspace for the `rule:std-default-hasher` job, run through the real
# census: every line marked POSITIVE must be reported and nothing else. The job
# also names every delegated rule, so the census is shown to accept each id and
# to report nothing of its own for it.
RULE_FIXTURE_LEDGER = """
[[job]]
id = "fixed-hasher-everywhere"
summary = "s"
home = "fixture_hash::FixedState"
entry_points = []
spec = "s"
vectors = []
bench = []
sites = []
replaces_external = []
enforced = true

[job.forbidden]
constants = []
fingerprints = ["rule:std-default-hasher", "rule:raw-hash-domain", "rule:shared-hash-domain"]
names = []
"""

RULE_FIXTURE_FILES = {
    "crates/hash/Cargo.toml": '[package]\nname = "fixture-hash"\n',
    "crates/hash/src/lib.rs": "/// The fixed hasher.\npub struct FixedState;\n",
    "crates/user/Cargo.toml": '[package]\nname = "fixture-user"\n',
    "crates/user/src/lib.rs": (
        "use std::collections::{HashMap, HashSet};\n"
        "use std::collections;\n"
        "pub type FastMap<K, V> = HashMap<K, V, fixture_hash::FixedState>;\n"
        "pub struct Table { rows: HashMap<u8, u8> } // POSITIVE\n"
        "pub struct Fixed { rows: FastMap<u8, u8>, seen: HashSet<u8, fixture_hash::FixedState> }\n"
        "pub fn build() -> FastMap<u8, u8> { FastMap::default() }\n"
        "pub fn annotated() { let m: HashMap<u8, u8, fixture_hash::FixedState> = HashMap::default(); }\n"
        "pub fn defaulted() { let m = HashMap::<u8, u8>::default(); } // POSITIVE\n"
        "pub fn qualified() -> collections::HashSet<u8> { todo!() } // POSITIVE\n"
        "pub fn foreign(m: hashbrown::HashMap<u8, u8>) {}\n"
    ),
    "crates/user/tests/it.rs": (
        "use std::collections::HashMap;\n"
        "#[test]\n"
        "fn keyring() { assert_eq!(HashMap::from([(1, 2)]).len(), 1); } // POSITIVE\n"
        "#[test]\n"
        "fn collected() { let s = [1].into_iter().collect::<std::collections::HashSet<u8>>(); } // POSITIVE\n"
    ),
}


def rule_fixture_cases() -> list[tuple[str, bool]]:
    """Run the census over the seeded workspace and compare its hits with the
    lines marked POSITIVE."""
    with tempfile.TemporaryDirectory(prefix="helper-census-fixture-") as directory:
        root = Path(directory)
        expected: set[tuple[str, int]] = set()
        for relative, text in RULE_FIXTURE_FILES.items():
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            expected |= {
                (relative, number)
                for number, line in enumerate(text.splitlines(), start=1)
                if line.endswith("// POSITIVE")
            }
        (root / "helpers-ledger.toml").write_text(RULE_FIXTURE_LEDGER, encoding="utf-8")
        try:
            index = json.loads(run_census("--index", root=root))
        except (CensusError, json.JSONDecodeError) as exc:
            return [(f"the census runs over the seeded std-default-hasher workspace ({exc})", False)]
    found = index["jobs"]["fixed-hasher-everywhere"]["matches"]
    reported = {(match["file"], match["line"]) for match in found}
    return [
        ("every seeded std-default-hasher spelling is reported", expected <= reported),
        (
            "a named hasher, a hasher alias, an annotated let and a foreign map are not reported",
            reported <= expected,
        ),
        ("each hit is a copy of the enforced job", index["jobs"]["fixed-hasher-everywhere"]["copies"] == len(expected)),
    ]


# A seeded workspace for the literal rules, run through the real census:
# `rule:vocabulary-literal` reads its namespaces from the `NS`/`*_NS` constants of
# the job's home and entry-point modules, and `rule:home-literal` reads its tokens
# from the home item's own bodies. Every line marked POSITIVE must be reported for
# its job and nothing else; the NEGATIVE lines are the neighbours that must not be.
LITERAL_FIXTURE_LEDGER = """
[[job]]
id = "vocab"
summary = "s"
home = "fixture_vocab::vocab"
entry_points = ["fixture_vocab::datatype"]
spec = "s"
vectors = []
bench = []
sites = []
replaces_external = []
enforced = true

[job.forbidden]
constants = []
fingerprints = ["rule:vocabulary-literal"]
names = []

[[job]]
id = "direction"
summary = "s"
home = "fixture_vocab::Direction"
entry_points = []
spec = "s"
vectors = []
bench = []
sites = []
replaces_external = []
enforced = true

[job.forbidden]
constants = []
fingerprints = ["rule:home-literal"]
names = []

[[job]]
id = "nothing"
summary = "s"
home = "fixture_vocab::Direction"
entry_points = []
spec = "s"
vectors = []
bench = []
sites = []
replaces_external = []
enforced = true

[job.forbidden]
constants = []
fingerprints = ["rule:vocabulary-literal"]
names = []
"""

LITERAL_FIXTURE_FILES = {
    "crates/vocab/Cargo.toml": '[package]\nname = "fixture-vocab"\n',
    "crates/vocab/src/lib.rs": (
        "/// Terms.\npub mod vocab {\n"
        "    /// The namespace.\n    pub mod ex {\n"
        '        pub const NS: &str = "http://example.org/ns#";\n'
        '        pub const THING: &str = "http://example.org/ns#Thing";\n'
        "    }\n}\n"
        "/// Datatypes.\npub mod datatype {\n"
        '    pub const DT_NS: &str = "http://example.org/dt#";\n'
        '    pub const DT_TEXT: &str = "http://example.org/dt#text";\n'
        "}\n"
        "/// A direction.\npub enum Direction { Up, Down }\n"
        "impl Direction {\n"
        '    pub const fn as_str(&self) -> &str { match self { Self::Up => "up", Self::Down => "down" } }\n'
        "}\n"
    ),
    "crates/user/Cargo.toml": '[package]\nname = "fixture-user"\n',
    "crates/user/src/lib.rs": (
        'pub const THING: &str = "http://example.org/ns#Thing"; // POSITIVE vocab\n'
        'pub fn ns() -> &\'static str { "http://example.org/ns#" } // POSITIVE vocab\n'
        'pub fn typed(x: &str) -> String { format!("http://example.org/dt#{x}") } // POSITIVE vocab\n'
        'pub fn raw() -> &\'static str { r"http://example.org/dt#text" } // POSITIVE vocab\n'
        'pub fn parse(s: &str) -> bool { s == "up" } // POSITIVE direction\n'
        'pub fn bracketed() -> &\'static str { "<http://example.org/ns#Thing>" } // NEGATIVE\n'
        'pub fn sibling() -> &\'static str { "http://example.org/nsX" } // NEGATIVE\n'
        'pub fn upper() -> &\'static str { "upward" } // NEGATIVE\n'
        'pub const DOC: &str = "PREFIX ex: <http://example.org/ns#>\\nSELECT * { ?s ?p ?o }"; // NEGATIVE\n'
        'pub const EMBEDDED: &str = "http://example.org/ns#\\nSELECT * { ?s ?p ?o }"; // NEGATIVE\n'
        "#[cfg(test)]\nmod tests {\n"
        '    const THING: &str = "http://example.org/ns#Thing"; // NEGATIVE\n'
        "}\n"
    ),
    "crates/user/tests/it.rs": 'const THING: &str = "http://example.org/ns#Thing"; // NEGATIVE\n',
}


def literal_fixture_cases() -> list[tuple[str, bool]]:
    """Run the census over the seeded literal-rule workspace and compare each
    job's hits with the lines marked POSITIVE for it."""
    with tempfile.TemporaryDirectory(prefix="helper-census-literals-") as directory:
        root = Path(directory)
        expected: dict[str, set[tuple[str, int]]] = {"vocab": set(), "direction": set()}
        for relative, text in LITERAL_FIXTURE_FILES.items():
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            for number, line in enumerate(text.splitlines(), start=1):
                for job in expected:
                    if line.endswith(f"// POSITIVE {job}"):
                        expected[job].add((relative, number))
        (root / "helpers-ledger.toml").write_text(LITERAL_FIXTURE_LEDGER, encoding="utf-8")
        try:
            index = json.loads(run_census("--index", root=root))
        except (CensusError, json.JSONDecodeError) as exc:
            return [(f"the census runs over the seeded literal-rule workspace ({exc})", False)]
    reported = {
        job: {(match["file"], match["line"]) for match in index["jobs"][job]["matches"]} for job in expected
    }
    return [
        (
            "every seeded vocabulary literal (a retyped term, a namespace, a format string, a raw string) is reported",
            reported["vocab"] == expected["vocab"],
        ),
        (
            "a bracketed term, a sibling namespace, an embedded document and test-only code are not vocabulary hits",
            not any(
                line.endswith("// NEGATIVE")
                for relative, number in reported["vocab"] | reported["direction"]
                for line in [LITERAL_FIXTURE_FILES[relative].splitlines()[number - 1]]
            ),
        ),
        (
            "a token the home spells is reported outside it, and a longer word is not",
            reported["direction"] == expected["direction"],
        ),
        (
            "a vocabulary job whose modules declare no namespace constant is itself a copy",
            index["jobs"]["nothing"]["copies"] == 1
            and "declare no namespace constant" in index["jobs"]["nothing"]["matches"][0]["reasons"][0],
        ),
        (
            "each literal hit is a copy of its enforced job",
            index["jobs"]["vocab"]["copies"] == len(expected["vocab"])
            and index["jobs"]["direction"]["copies"] == len(expected["direction"]),
        ),
    ]


# A seeded workspace for the base16 rules, run through the real census: the
# `rule:hex-format-loop` lines marked POSITIVE (a two-digit hex format inside a
# loop or closure, in shipping code and in a test) must be reported and the
# neighbours beside them (one pair outside any loop, a pair in an assertion
# message, an escaped brace, a wider spec) must not; likewise a byte-pair radix
# parse and a hex-digit table outside the home. The home's own table is its.
# The tables' digits are joined when the fixture is written, so this file
# spells none.
HEX_FIXTURE_LEDGER = """
[[job]]
id = "hex"
summary = "s"
home = "fixture_hash::hex::encode"
entry_points = []
spec = "s"
vectors = []
bench = []
sites = []
replaces_external = []
enforced = true

[job.forbidden]
constants = []
fingerprints = ["rule:hex-format-loop", "rule:hex-pair-radix", "rule:hex-table"]
names = []
"""

HEX_DIGITS_LOWER = "01234567" + "89abcdef"

HEX_FIXTURE_FILES = {
    "crates/hash/Cargo.toml": '[package]\nname = "fixture-hash"\n',
    "crates/hash/src/lib.rs": "pub mod hex;\n",
    "crates/hash/src/hex.rs": (
        f'const ALPHABET: &[u8; 16] = b"{HEX_DIGITS_LOWER}";\n'
        "/// The one encoder.\npub fn encode(bytes: &[u8]) -> String { String::new() }\n"
    ),
    "crates/user/Cargo.toml": '[package]\nname = "fixture-user"\n',
    "crates/user/src/lib.rs": (
        "use std::fmt::Write as _;\n"
        "pub fn render(bytes: &[u8], out: &mut String) {\n"
        '    for byte in bytes { let _ = write!(out, "{byte:02x}"); } // POSITIVE\n'
        '    let pairs: Vec<String> = bytes.iter().map(|b| format!("%{b:02X}")).collect(); // POSITIVE\n'
        '    let one = format!("{:02x}", bytes[0]);\n'
        '    for b in bytes { assert!(*b < 255, "byte {b:02x}"); }\n'
        '    for b in bytes { let _ = write!(out, "{{:02x}} U+{:04X}", u32::from(*b)); }\n'
        "}\n"
        'pub fn pair(text: &str) -> u8 { u8::from_str_radix(text, 16).unwrap_or(0) } // POSITIVE\n'
        "pub fn point(text: &str) -> u32 { u32::from_str_radix(text, 16).unwrap_or(0) }\n"
        f'pub const DIGITS: &str = "{HEX_DIGITS_LOWER.upper()}"; // POSITIVE\n'
        f'pub const NEAR: &str = "{HEX_DIGITS_LOWER}g";\n'
    ),
    "crates/user/tests/it.rs": (
        "#[test]\n"
        "fn renders() {\n"
        '    let s = [1u8, 2].iter().fold(String::new(), |mut o, b| { o.push_str(&format!("{b:02x}")); o }); // POSITIVE\n'
        "}\n"
    ),
}


def hex_rule_fixture_cases() -> list[tuple[str, bool]]:
    """Run the census over the seeded base16 workspace and compare its hits with
    the lines marked POSITIVE."""
    with tempfile.TemporaryDirectory(prefix="helper-census-hex-fixture-") as directory:
        root = Path(directory)
        expected: set[tuple[str, int]] = set()
        for relative, text in HEX_FIXTURE_FILES.items():
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            expected |= {
                (relative, number)
                for number, line in enumerate(text.splitlines(), start=1)
                if line.endswith("// POSITIVE")
            }
        (root / "helpers-ledger.toml").write_text(HEX_FIXTURE_LEDGER, encoding="utf-8")
        try:
            index = json.loads(run_census("--index", root=root))
        except (CensusError, json.JSONDecodeError) as exc:
            return [(f"the census runs over the seeded base16 workspace ({exc})", False)]
    job = index["jobs"]["hex"]
    copies = {(match["file"], match["line"]) for match in job["matches"] if not match["in_home"]}
    home = {(match["file"], match["line"]) for match in job["matches"] if match["in_home"]}
    loops = {
        (match["file"], match["line"])
        for match in job["matches"]
        if any(reason.startswith("rule:hex-format-loop") for reason in match["reasons"])
    }
    loop_positives = {hit for hit in expected if "02" in HEX_FIXTURE_FILES[hit[0]].splitlines()[hit[1] - 1]}
    return [
        ("every seeded two-digit hex format inside a loop or closure is reported", loop_positives <= loops),
        (
            "a pair formatted once, a pair in an assertion message, an escaped brace and a wider spec are not",
            loops <= loop_positives,
        ),
        ("every seeded base16 copy (loop, pair parse, table) is reported", expected <= copies),
        ("a u32 code-point parse and a near-miss table are not reported", copies <= expected),
        ("the home's own digit table is the home's, not a copy", home == {("crates/hash/src/hex.rs", 1)}),
        ("each base16 hit is a copy of the enforced job", job["copies"] == len(expected)),
    ]


# A seeded workspace for the lexical-terminal rules, run through the real census.
# Each job names one rule; every line marked POSITIVE must be reported against its
# job and nothing else may be. The neighbours beside each positive are the valid
# spellings the rule must leave alone: the shared predicate, a decimal radix, a
# pointer escape inside a longer literal or inside the home package, and the same
# patterns in test code, which may name a trap to pin it.
LEX_FIXTURE_LEDGER = """
[[job]]
id = "grammar-ws"
summary = "s"
home = "fixture_lex::skip_ws"
entry_points = []
spec = "s"
vectors = []
bench = []
sites = []
replaces_external = []
enforced = true

[job.forbidden]
constants = []
fingerprints = ["rule:grammar-ws"]
names = []

[[job]]
id = "json-pointer"
summary = "s"
home = "fixture_lex::escape_token"
entry_points = []
spec = "s"
vectors = []
bench = []
sites = []
replaces_external = []
enforced = true

[job.forbidden]
constants = []
fingerprints = ["rule:json-pointer-escape"]
names = []

[[job]]
id = "escape-decode"
summary = "s"
home = "fixture_lex::skip_ws"
entry_points = []
spec = "s"
vectors = []
bench = []
sites = []
replaces_external = []
enforced = true

[job.forbidden]
constants = []
fingerprints = ["rule:hex-digit-radix"]
names = []
"""

LEX_FIXTURE_FILES = {
    "crates/lex/Cargo.toml": '[package]\nname = "fixture-lex"\n',
    "crates/lex/src/lib.rs": (
        "/// The one WS skip.\npub fn skip_ws(b: &[u8]) -> usize { b.len() }\n"
        "/// The one token escape.\n"
        'pub fn escape_token(t: &str) -> String { t.replace(\'~\', "~0").replace(\'/\', "~1") }\n'
    ),
    "crates/user/Cargo.toml": '[package]\nname = "fixture-user"\n',
    "crates/user/src/lib.rs": (
        "pub fn gap(b: u8) -> bool { b.is_ascii_whitespace() } // GRAMMAR-WS\n"
        "pub fn all_gap(s: &str) -> bool { s.bytes().all(|b| u8::is_ascii_whitespace(&b)) } // GRAMMAR-WS\n"
        "pub fn ws(b: u8) -> bool { matches!(b, b' ' | b'\\t' | b'\\n' | b'\\r') }\n"
        'pub fn esc(t: &str) -> String { t.replace(\'~\', "~0") } // JSON-POINTER\n'
        'pub fn slash(t: &str) -> String { t.replace(\'/\', "~1") } // JSON-POINTER\n'
        'pub fn path() -> &\'static str { "/a~1b/c~0d" }\n'
        "pub fn digit(c: char) -> Option<u32> { c.to_digit(16) } // HEX-DIGIT\n"
        "pub fn point(t: &str) -> Option<u32> { u32::from_str_radix(t, 16).ok() } // HEX-DIGIT\n"
        "pub fn decimal(c: char) -> Option<u32> { c.to_digit(10) }\n"
        "pub fn count(t: &str) -> Option<u32> { u32::from_str_radix(t, 10).ok() }\n"
        "#[cfg(test)]\n"
        "mod tests {\n"
        '    fn traps(b: u8) -> bool { b.is_ascii_whitespace() && "~1".is_empty() && char::from(b).to_digit(16).is_some() }\n'
        "}\n"
    ),
    "crates/user/tests/it.rs": (
        "#[test]\n"
        'fn pins() { assert!(b\' \'.is_ascii_whitespace() && "~0".len() == 2 && \'a\'.to_digit(16) == Some(10)); }\n'
    ),
}


def lex_rule_fixture_cases() -> list[tuple[str, bool]]:
    """Run the census over the seeded lexical workspace and compare each job's
    hits with the lines marked for it."""
    markers = {"grammar-ws": "// GRAMMAR-WS", "json-pointer": "// JSON-POINTER", "escape-decode": "// HEX-DIGIT"}
    with tempfile.TemporaryDirectory(prefix="helper-census-lex-fixture-") as directory:
        root = Path(directory)
        expected: dict[str, set[tuple[str, int]]] = {job: set() for job in markers}
        for relative, text in LEX_FIXTURE_FILES.items():
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            for number, line in enumerate(text.splitlines(), start=1):
                for job, marker in markers.items():
                    if line.endswith(marker):
                        expected[job].add((relative, number))
        (root / "helpers-ledger.toml").write_text(LEX_FIXTURE_LEDGER, encoding="utf-8")
        try:
            index = json.loads(run_census("--index", root=root))
        except (CensusError, json.JSONDecodeError) as exc:
            return [(f"the census runs over the seeded lexical workspace ({exc})", False)]
    cases: list[tuple[str, bool]] = []
    for job, lines in expected.items():
        found = index["jobs"][job]
        copies = {(match["file"], match["line"]) for match in found["matches"] if not match["in_home"]}
        cases.append((f"every seeded {job} spelling in shipping code is reported", lines <= copies))
        cases.append(
            (
                f"the {job} neighbours (the shared spelling, test code, the home) are not reported",
                copies <= lines,
            )
        )
        cases.append((f"each {job} hit is a copy of the enforced job", found["copies"] == len(lines)))
    pointer_home = {
        (match["file"], match["line"]) for match in index["jobs"]["json-pointer"]["matches"] if match["in_home"]
    }
    cases.append(("the pointer escape inside its home is the home's, not a copy", pointer_home == {("crates/lex/src/lib.rs", 4)}))
    return cases


# A seeded workspace for the byte-layout rules, run through the real census:
# every line marked POSITIVE must be reported and nothing else. Beside each
# refusal sits a valid neighbour: a count written before a loop (not the field
# itself), a varint length, a hasher's own integer writer, a framing inside the
# home package, a read through first_chunk, a fixed-array conversion, a flag
# mask, test code, and a forbidden variant row that sanctions its hit.
LAYOUT_FIXTURE_LEDGER = """
[[job]]
id = "frame-le"
summary = "s"
home = "fixture_hash::frame::frame_le"
entry_points = []
spec = "s"
vectors = []
bench = []
sites = []
replaces_external = []
enforced = true

[job.forbidden]
constants = []
fingerprints = ["rule:length-prefix-frame"]
names = []

[[job.variant]]
symbol = "fixture_user::decimal"
file = "crates/user/src/lib.rs"
detector = "forbidden"
criterion = "a"
anchor = "fixture_user::decimal"
reason = "r"

[[job]]
id = "le-bytes"
summary = "s"
home = "fixture_hash::frame::frame_le"
entry_points = []
spec = "s"
vectors = []
bench = []
sites = []
replaces_external = []
enforced = true

[job.forbidden]
constants = []
fingerprints = ["rule:le-slice-int", "rule:align-up-mask", "rule:div-ceil-by-hand"]
names = []
"""

LAYOUT_FIXTURE_FILES = {
    "crates/hash/Cargo.toml": '[package]\nname = "fixture-hash"\n',
    "crates/hash/src/lib.rs": "pub mod frame;\n",
    "crates/hash/src/frame.rs": (
        "/// The one framing.\n"
        "pub fn frame_le(out: &mut Vec<u8>, bytes: &[u8]) {\n"
        "    out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());\n"
        "    out.extend_from_slice(bytes);\n"
        "}\n"
    ),
    "crates/user/Cargo.toml": '[package]\nname = "fixture-user"\n',
    "crates/user/src/lib.rs": (
        "pub fn framed(out: &mut Vec<u8>, bytes: &[u8]) {\n"
        "    out.extend_from_slice(&(bytes.len() as u64).to_le_bytes()); // POSITIVE\n"
        "    out.extend_from_slice(bytes);\n"
        "}\n"
        "pub fn streamed(h: &mut H, field: &[u8]) {\n"
        "    let length = u64::try_from(field.len()).expect(\"fits\");\n"
        "    h.update(length.to_le_bytes()); // POSITIVE\n"
        "    h.update(field);\n"
        "}\n"
        "pub fn counted(out: &mut Vec<u8>, items: &[u8]) {\n"
        "    out.extend_from_slice(&(items.len() as u64).to_le_bytes());\n"
        "    for item in items { out.push(*item); }\n"
        "}\n"
        "pub fn varint(out: &mut Vec<u8>, bytes: &[u8]) {\n"
        "    write_varint(out, bytes.len() as u64);\n"
        "    out.extend_from_slice(bytes);\n"
        "}\n"
        "pub fn hashed(h: &mut H, s: &str) {\n"
        "    h.write_u64(s.len() as u64);\n"
        "    h.write(s.as_bytes());\n"
        "}\n"
        "/// A decimal framing a variant row sanctions.\n"
        "pub fn decimal(out: &mut String, s: &str) {\n"
        "    let _ = write!(out, \"{}:{}\", s.len(), s);\n"
        "}\n"
        "pub fn read(b: &[u8], i: usize) -> u32 {\n"
        "    u32::from_le_bytes(b[i..i + 4].try_into().unwrap()) // POSITIVE\n"
        "}\n"
        "pub fn read_chunk(b: &[u8], i: usize) -> u32 { u32::from_le_bytes(*b[i..].first_chunk().unwrap()) }\n"
        "pub fn read_array(b: [u8; 4]) -> u32 { u32::from_le_bytes(b) }\n"
        "pub fn put(b: &mut [u8], i: usize, v: u32) {\n"
        "    b[i..i + 4].copy_from_slice(&v.to_le_bytes()); // POSITIVE\n"
        "}\n"
        "pub fn align(v: usize) -> usize { (v + 7) & !7 } // POSITIVE\n"
        "pub fn up(n: i128, d: i128) -> i128 { (n + d - 1) / d } // POSITIVE\n"
        "pub fn up_std(n: u64, d: u64) -> u64 { n.div_ceil(d) }\n"
        "pub fn flags(f: u32) -> u32 { f & !(A | B) }\n"
        "pub fn low(f: u64, n: u32) -> u64 { f.bits() & !((1 << n) - 1) }\n"
        "#[cfg(test)]\n"
        "mod tests { fn oracle(b: &[u8]) -> u32 { u32::from_le_bytes(b[0..4].try_into().unwrap()) } }\n"
    ),
    "crates/user/tests/it.rs": "fn oracle(b: &[u8]) -> u32 { u32::from_le_bytes(b[0..4].try_into().unwrap()) }\n",
}


def layout_rule_fixture_cases() -> list[tuple[str, bool]]:
    """Run the census over the seeded byte-layout workspace and compare its hits
    with the lines marked POSITIVE."""
    with tempfile.TemporaryDirectory(prefix="helper-census-layout-fixture-") as directory:
        root = Path(directory)
        expected: set[tuple[str, int]] = set()
        for relative, text in LAYOUT_FIXTURE_FILES.items():
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            expected |= {
                (relative, number)
                for number, line in enumerate(text.splitlines(), start=1)
                if line.endswith("// POSITIVE")
            }
        (root / "helpers-ledger.toml").write_text(LAYOUT_FIXTURE_LEDGER, encoding="utf-8")
        try:
            index = json.loads(run_census("--index", root=root))
        except (CensusError, json.JSONDecodeError) as exc:
            return [(f"the census runs over the seeded byte-layout workspace ({exc})", False)]
    copies = {
        (match["file"], match["line"])
        for job in ("frame-le", "le-bytes")
        for match in index["jobs"][job]["matches"]
        if not match["in_home"] and not match["variant"]
    }
    variants = {match["symbol"] for match in index["jobs"]["frame-le"]["matches"] if match["variant"]}
    home = [match for match in index["jobs"]["frame-le"]["matches"] if match["in_home"]]
    return [
        ("every seeded framing, slice-indexed integer, alignment mask and hand ceiling division is reported", expected <= copies),
        (
            "a count before a loop, a varint, a hasher's integer, a first_chunk read, a flag mask, std div_ceil and test code are not",
            copies <= expected,
        ),
        ("the framing inside the home package is the home's", len(home) == 1),
        ("a forbidden variant row sanctions its rule hit", variants == {"crates/user/src/lib.rs::decimal"}),
    ]


def fixture_ledger() -> dict:
    return {
        "job": [
            {
                "id": "demo",
                "summary": "s",
                "home": "home_crate::f",
                "entry_points": ["home_crate::g"],
                "spec": "x",
                "vectors": ["v.txt"],
                "bench": ["home-crate:b"],
                "sites": ["home.site"],
                "replaces_external": ["banned-pkg"],
                "enforced": True,
                "forbidden": {"constants": [], "fingerprints": [], "names": []},
                "variant": [
                    {
                        "symbol": "other::v",
                        "file": "crates/other/src/lib.rs",
                        "detector": "forbidden",
                        "criterion": "a",
                        "anchor": "other::v",
                        "reason": "r",
                    }
                ],
            }
        ]
    }


def fixture_index() -> dict:
    documented = {"symbol": "", "file": "", "line": 1, "module": "", "package": "", "documented": True}
    return {
        "symbols": {
            "home_crate::f": dict(documented, symbol="home_crate::f", file="crates/home/src/lib.rs"),
            "home_crate::g": dict(documented, symbol="home_crate::g", file="crates/home/src/lib.rs"),
            "other::v": dict(documented, symbol="other::v", file="crates/other/src/lib.rs"),
        },
        "jobs": {
            "demo": {
                "home": "home_crate::f",
                "home_package": "home-crate",
                "copies": 0,
                "matches": [
                    {"symbol": "home_crate::f", "file": "crates/home/src/lib.rs", "line": 1, "reasons": ["constant 0x1"], "in_home": True, "variant": False},
                    {"symbol": "other::v", "file": "crates/other/src/lib.rs", "line": 1, "reasons": ["constant 0x1"], "in_home": False, "variant": True},
                ],
            }
        },
        "grouped_variants": [],
        "errors": [],
    }


def run_fixture(ledger: dict, index: dict, **overrides) -> list[str]:
    facts = {
        "exists": lambda path: path == "v.txt",
        "benches": {"home-crate:b"},
        "sites": {"home.site": {"home_crate"}},
        "banned": {"banned-pkg"},
        "iri_matched": {("crates/gts/src/files.rs", "dot-segment-loop")},
        "iri_rules": {"dot-segment-loop"},
    }
    facts.update(overrides)
    return ledger_findings(ledger, index, **facts)


def mutated(change) -> tuple[dict, dict]:
    ledger, index = fixture_ledger(), fixture_index()
    change(ledger, index)
    return ledger, index


def self_test() -> int:
    cases: list[tuple[str, bool]] = []

    def expect(name: str, findings: list[str], fragment: str | None) -> None:
        if fragment is None:
            cases.append((name, findings == []))
        else:
            cases.append((name, len(findings) == 1 and fragment in findings[0]))

    expect("a ledger that holds passes", run_fixture(fixture_ledger(), fixture_index()), None)

    ledger, index = mutated(lambda _ledger, i: i["symbols"].__setitem__("home_crate::f", None))
    expect("a home that does not resolve fails", run_fixture(ledger, index), "home `home_crate::f` does not resolve")

    ledger, index = mutated(lambda _ledger, i: i["symbols"]["other::v"].__setitem__("documented", False))
    expect("an undocumented anchor fails", run_fixture(ledger, index), "carries no documentation")

    ledger, index = mutated(lambda _ledger, i: i["symbols"]["other::v"].__setitem__("file", "crates/other/src/x.rs"))
    expect("a variant defined outside its file fails", run_fixture(ledger, index), "not in crates/other/src/lib.rs")

    expect(
        "a site measuring another crate fails",
        run_fixture(fixture_ledger(), fixture_index(), sites={"home.site": {"memchr"}}),
        "not in the home crate",
    )
    expect(
        "a site measuring the home crate passes",
        run_fixture(fixture_ledger(), fixture_index(), sites={"home.site": {"home_crate"}}),
        None,
    )
    expect(
        "an unbanned replaces_external fails",
        run_fixture(fixture_ledger(), fixture_index(), banned=set()),
        "is not banned",
    )
    expect("a missing vectors file fails", run_fixture(fixture_ledger(), fixture_index(), exists=lambda _: False), "does not exist")
    expect("an unknown bench id fails", run_fixture(fixture_ledger(), fixture_index(), benches=set()), "bench `home-crate:b`")

    ledger, index = mutated(lambda _ledger, i: i["jobs"]["demo"]["matches"].pop())
    expect("a forbidden variant that matches nothing is STALE", run_fixture(ledger, index), "STALE")

    def outside(_ledger, i):
        i["jobs"]["demo"]["matches"].append(
            {"symbol": "third::copy", "file": "crates/third/src/lib.rs", "line": 9, "reasons": ["constant 0x1"], "in_home": False, "variant": False}
        )
        i["jobs"]["demo"]["copies"] = 1

    ledger, index = mutated(outside)
    expect("a forbidden match outside the home of an enforced job fails", run_fixture(ledger, index), "third::copy")
    ledger["job"][0]["enforced"] = False
    expect("the same match in an unenforced job is reported only by --census", run_fixture(ledger, index), None)

    ledger, index = mutated(lambda l, _index: l["job"][0].__setitem__("enforced", False))
    expect("an unenforced job with no copies left is STALE", run_fixture(ledger, index), "set `enforced = true`")

    ledger, index = mutated(lambda l, _index: l["job"][0]["forbidden"]["fingerprints"].append("body:0123456789abcdef"))
    expect("a body fingerprint that matches nothing is STALE", run_fixture(ledger, index), "matches nothing")

    def isomorphic(l, i, grouped):
        l["job"][0]["variant"][0]["detector"] = "isomorphic"
        i["jobs"]["demo"]["matches"][1]["variant"] = False
        i["jobs"]["demo"]["matches"].pop()
        i["grouped_variants"] = ["other::v"] if grouped else []

    ledger, index = mutated(lambda l, i: isomorphic(l, i, True))
    expect("an isomorphic variant in a group passes", run_fixture(ledger, index), None)
    ledger, index = mutated(lambda l, i: isomorphic(l, i, False))
    expect("an isomorphic variant in no group is STALE", run_fixture(ledger, index), "STALE")

    def iri(l, i, file):
        variant = l["job"][0]["variant"][0]
        variant["detector"] = "dot-segment-loop"
        variant["file"] = file
        i["symbols"]["other::v"]["file"] = file
        i["jobs"]["demo"]["matches"].pop()

    ledger, index = mutated(lambda l, i: iri(l, i, "crates/gts/src/files.rs"))
    expect("an IRI-gate variant the gate still matches passes", run_fixture(ledger, index), None)
    ledger, index = mutated(lambda l, i: iri(l, i, "crates/other/src/lib.rs"))
    expect("an IRI-gate variant the gate no longer matches is STALE", run_fixture(ledger, index), "STALE")

    ledger, index = mutated(lambda l, _index: l["job"][0]["variant"][0].__setitem__("detector", "guess"))
    expect("an unknown detector fails", run_fixture(ledger, index), "unknown detector")

    manifests = {"crates/a/Cargo.toml", "crates/b/Cargo.toml"}
    positive = path_include_findings(
        {"crates/a/tests/it.rs": '#[path = "../../b/tests/support/fixture.rs"]\nmod fixture;\n'}, manifests
    )
    cases.append(("a #[path] include of another crate's file is a finding", len(positive) == 1 and "outside its crate crates/a" in positive[0]))
    negative = path_include_findings(
        {
            "crates/a/tests/it.rs": '#[path = "support/fixture.rs"]\nmod fixture;\n',
            "crates/a/benches/b.rs": '#[path = "../tests/support/corpus.rs"]\nmod corpus;\n',
            "crates/a/src/lib.rs": '// #[path = "../../b/src/x.rs"]\n/// `#[path = "../../b/src/x.rs"]` in prose\nconst NOTE: &str = "#[path = \\"../../b/src/x.rs\\"]";\n',
        },
        manifests,
    )
    cases.append(("a #[path] include inside its own crate, in a comment or in a string passes", negative == []))
    escaping = path_include_findings({"crates/a/src/lib.rs": '#[path = "../../../../x.rs"]\nmod x;\n'}, manifests)
    cases.append(("a #[path] include that climbs above the repository is a finding", len(escaping) == 1))

    def delegated(hits: list[tuple[str, int, str, str]], enforced: bool = True) -> list[str]:
        ledger, index = fixture_ledger(), fixture_index()
        ledger["job"][0]["forbidden"]["fingerprints"].append("rule:raw-hash-domain")
        ledger["job"][0]["enforced"] = enforced
        merge_delegated(ledger, index, {"rule:raw-hash-domain": hits})
        return run_fixture(ledger, index)

    hit = ("crates/third/src/lib.rs", 4, "", 'b"purrdf-third/raw/v1" is handed to `.update(…)` without a `Domain`')
    expect("a delegated rule's hit fails the enforced job naming it", delegated([hit]), "crates/third/src/lib.rs:4")
    expect("the same job with no delegated hit passes", delegated([]), None)
    expect("a delegated hit in an unenforced job is reported only by --census", delegated([hit], enforced=False), None)

    # A ledger-declared hash-domain exception: a variant row whose detector is
    # one of check-hash-domains.py's rules.
    shared_hit = ("crates/other/src/lib.rs", 7, "v", "`v` opens hashers in 2 functions (a, b)")

    def declared(
        detector: str, hits: list[tuple[str, int, str, str]], matched: set[tuple[str, str, str]]
    ) -> list[str]:
        ledger, index = fixture_ledger(), fixture_index()
        ledger["job"][0]["forbidden"]["fingerprints"].append("rule:shared-hash-domain")
        ledger["job"][0]["variant"][0]["detector"] = detector
        index["jobs"]["demo"]["matches"].pop()
        merge_delegated(ledger, index, {"rule:shared-hash-domain": hits})
        return run_fixture(
            ledger, index, hash_matched=matched, hash_rules={"shared-hash-domain", "undomained-digest"}
        )

    shared_match = {("crates/other/src/lib.rs", "shared-hash-domain", "v")}
    expect(
        "a declared shared-domain exception passes",
        declared("shared-hash-domain", [shared_hit], shared_match),
        None,
    )
    expect(
        "an undeclared shared domain fails the job",
        declared("undomained-digest", [shared_hit], {("crates/other/src/lib.rs", "undomained-digest", "v")}),
        "crates/other/src/lib.rs:7",
    )
    expect(
        "a declared shared-domain exception the gate no longer matches is STALE",
        declared("shared-hash-domain", [], set()),
        "STALE",
    )
    expect(
        "a declared undomained digest the gate still matches passes",
        declared("undomained-digest", [], {("crates/other/src/lib.rs", "undomained-digest", "v")}),
        None,
    )
    expect(
        "a declared undomained digest that now opens under a Domain is STALE",
        declared("undomained-digest", [], set()),
        "STALE",
    )

    empty_scan = HASH_DOMAINS.Scan([], [], [], [], set())
    cases.append(
        (
            "every rule check-hash-domains.py reports is a delegated rule id",
            set(HASH_DOMAINS.rule_hits(empty_scan)) == set(DELEGATED_RULES),
        )
    )

    def undocumented(_ledger, i):
        i["symbols"]["other::v"]["documented"] = False

    ledger, index = mutated(undocumented)
    ledger["job"][0]["variant"][0]["detector"] = "undomained-digest"
    index["jobs"]["demo"]["matches"].pop()
    expect(
        "a declared exception whose anchor carries no documentation fails",
        run_fixture(
            ledger,
            index,
            hash_matched={("crates/other/src/lib.rs", "undomained-digest", "v")},
            hash_rules={"undomained-digest"},
        ),
        "carries no documentation",
    )

    cases.extend(rule_fixture_cases())
    cases.extend(literal_fixture_cases())
    cases.extend(hex_rule_fixture_cases())
    cases.extend(lex_rule_fixture_cases())
    cases.extend(layout_rule_fixture_cases())

    failed = 0
    for name, held in cases:
        print(f"{'PASS' if held else 'FAIL'}: {name}")
        failed += not held
    if failed:
        print(f"check-shared-helpers self-test: {failed} case(s) failed")
        return 1
    print("OK: check-shared-helpers self-test")
    return 0


# ---------------------------------------------------------------------------


def main() -> int:
    arguments = sys.argv[1:]
    if arguments == ["--self-test"]:
        return self_test()
    if len(arguments) == 2 and arguments[0] in ("--census", "--count"):
        try:
            sys.stdout.write(run_census(*arguments))
        except CensusError as exc:
            print(f"FAIL: {exc}")
            return 1
        return 0
    if arguments:
        print("usage: check-shared-helpers.py [--self-test | --census JOB | --count JOB]", file=sys.stderr)
        return 2

    findings: list[str] = []
    try:
        with LEDGER_PATH.open("rb") as handle:
            ledger = tomllib.load(handle)
        with MANIFEST_PATH.open("rb") as handle:
            manifest = tomllib.load(handle)
        census_ledger = json.loads(run_census("--dump-ledger"))
        index = json.loads(run_census("--index"))
        metadata = BANNED_DEPS.cargo_metadata(REPO_ROOT / "Cargo.toml")
    except (OSError, tomllib.TOMLDecodeError, CensusError, BANNED_DEPS.MetadataError, json.JSONDecodeError) as exc:
        print(f"FAIL: {exc}")
        return 1
    if census_ledger != ledger:
        findings.append(
            "FAIL: helper-census and tomllib read helpers-ledger.toml differently; keep the file "
            "inside the subset crates/helper-census/src/toml.rs documents"
        )
    hash_scan = HASH_DOMAINS.scan_workspace(HASH_DOMAINS.shipping_files(REPO_ROOT))
    merge_delegated(ledger, index, HASH_DOMAINS.rule_hits(hash_scan))
    _, iri_matched = IRI_GATE.scan()
    findings.extend(
        ledger_findings(
            ledger,
            index,
            exists=lambda path: (REPO_ROOT / path).is_file(),
            benches=bench_targets(metadata),
            sites=site_crates(manifest),
            banned=set(BANNED_DEPS.BANNED_ANY_EDGE) | set(BANNED_DEPS.BANNED_DIRECT_ONLY),
            iri_matched=iri_matched,
            iri_rules=IRI_GATE.rule_ids(),
            hash_matched=HASH_DOMAINS.exception_matches(hash_scan),
            hash_rules=set(HASH_DOMAINS.EXCEPTION_RULES),
        )
    )
    files, manifests = tracked_rust_files()
    findings.extend(path_include_findings(files, manifests))
    if findings:
        for line in findings:
            print(line)
        return 1
    jobs = ledger.get("job", [])
    enforced = sum(1 for job in jobs if job["enforced"])
    variants = sum(len(job.get("variant", [])) for job in jobs)
    open_copies = sum(index["jobs"][job["id"]]["copies"] for job in jobs if not job["enforced"])
    print(
        f"OK: {len(jobs)} ledger jobs hold ({enforced} enforced, {variants} reasoned variants, "
        f"{open_copies} copies still open in unenforced jobs); no #[path] include leaves its crate "
        f"({len(files)} files)"
    )
    return 0


if __name__ == "__main__":
    os.environ.setdefault("CARGO_TERM_COLOR", "never")
    raise SystemExit(main())
