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
                stale = not any(
                    found["variant"] and (resolved is None or found["symbol"] == resolved["symbol"])
                    for found in census["matches"]
                )
            elif detector in iri_rules:
                stale = (variant["file"], detector) not in iri_matched
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


# ---------------------------------------------------------------------------
# Self-test.

# A seeded workspace for the `rule:std-default-hasher` job, run through the real
# census: every line marked POSITIVE must be reported and nothing else.
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
fingerprints = ["rule:std-default-hasher"]
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

    cases.extend(rule_fixture_cases())

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
