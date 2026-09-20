#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Normalise Lehigh's 14 published LUBM queries into valid SPARQL 1.1, auditably.

``queries-sparql.txt`` is fetched by digest by ``scripts/benchmark-acquire.py`` and
is NEVER vendored: it carries no redistribution grant. It also predates the final
SPARQL 1.1 Recommendation, so four of its spellings no longer parse, and its ``ub:``
prefix names a 2004 draft namespace that no generated LUBM dataset has ever used.

The queries must therefore be transformed before they can run — and a transformed
benchmark query is worthless unless a reader can check that it still asks what the
publisher asked. So every change here is a NUMBERED, MECHANICAL RULE applied by
this program, and every application is RECORDED with the text before and after it.
Nothing is hand-rewritten. ``--provenance`` prints the whole audit trail; a reader
who distrusts a rule can read its applications and see that no triple pattern, no
variable, and no term ever changed meaning.

THE FIVE RULES
==============

``R1 PROJECTION-COMMA``
    SPARQL 1.1 §16.1 spells a SELECT projection as a WHITESPACE-separated list of
    variables. The published file separates them with commas (``SELECT ?X, ?Y, ?Z``),
    which is the draft-era spelling and a parse error against the Recommendation.
    The commas are dropped. The variables, their spelling, and their ORDER are
    untouched, so the projection denotes exactly the same tuple it always did.

``R2 PATTERN-COMMA``
    Query 7's third triple pattern separates its three terms with commas rather than
    whitespace (``<...AssociateProfessor0>,  ub:teacherOf, ?Y``). A comma IS legal in
    a SPARQL triples block — it introduces an object list — so this does not merely
    fail to parse, it would parse as an object list with no subject or predicate. The
    separators are replaced with whitespace, yielding ``<...AssociateProfessor0>
    ub:teacherOf ?Y``: the subject-predicate-object triple the paper's Query 7 states.
    No other query's body contains a comma, and the self-test ASSERTS that, so this
    rule cannot quietly rewrite a legitimate object list somewhere else.

``R3 BARE-IRI``
    Queries 1 and 3 write an absolute IRI in object position with no angle brackets
    (``?X ub:takesCourse http://www.Department0.University0.edu/GraduateCourse0``).
    SPARQL has no production for a bare IRI; it is wrapped in ``<>``. The IRI's
    characters are not touched, so the term denotes the same resource.

``R4 PREFIX-REBIND``
    The published file binds ``ub:`` to
    ``http://www.lehigh.edu/~zhp2/2004/0401/univ-bench.owl#`` — the 2004 draft
    namespace. The UBA generator stamps its output with whatever ``-onto`` names,
    and the ontology Lehigh publishes today declares
    ``http://swat.cse.lehigh.edu/onto/univ-bench.owl#``. A query carrying the draft
    namespace matches NOTHING in a dataset carrying the published one: it does not
    fail, it silently answers zero, which is the worst possible benchmark outcome.
    Only the PREFIX declaration's IRI changes. Every ``ub:``-prefixed name in every
    pattern keeps its local part, so each term still denotes the same ontology
    concept — it just denotes it in the namespace the data actually uses. The target
    namespace is a parameter (``--namespace``), because it is a property of the
    dataset, not of this program.

``R5 TRAILING-SPACE``
    Trailing whitespace (Query 1 ends its SELECT line with a tab) is stripped. This
    changes no token; it is recorded only so that a byte-comparison of input against
    output has no unexplained difference.

ENTAILMENT REGIMES ARE PART OF THE QUERY
========================================

Each query is tagged with the entailment regime it REQUIRES, taken from the
canonical description in Guo, Pan and Heflin, "LUBM: A Benchmark for OWL Knowledge
Base Systems", Journal of Web Semantics 3(2), and corroborated by the axioms in
``univ-bench.owl`` itself. The tag is not decoration:

    A RESULT COUNT IS ONLY MEANINGFUL AGAINST ITS REGIME. Two engines may be
    compared on a query ONLY when both answered it under the same regime.

Comparing a no-inference store against an entailment-aware one on Query 6 is not a
performance comparison at all — the first answers 0 because LUBM never asserts
``Student``, and the fast wrong answer wins. That is how benchmark numbers lie, and
it is why every row this lane reports carries its regime and its dataset.
"""

import argparse
import re
import sys
import tempfile
from pathlib import Path
from typing import NamedTuple

REPO_ROOT = Path(__file__).resolve().parent.parent

# Fetched by digest into the ignored cache by scripts/benchmark-acquire.py. It is
# read from there and never copied into the tree: it carries no licence grant.
PUBLISHED = REPO_ROOT / "target" / "bench-artifacts" / "queries-sparql.txt"

# The namespace the published file binds `ub:` to: a 2004 draft that no generated
# LUBM dataset has ever carried.
DRAFT_NAMESPACE = "http://www.lehigh.edu/~zhp2/2004/0401/univ-bench.owl#"

# The namespace the ontology Lehigh publishes today declares, and the one the UBA
# generator stamps into its output when run with the matching `-onto`.
PUBLISHED_NAMESPACE = "http://swat.cse.lehigh.edu/onto/univ-bench.owl#"


class Regime(NamedTuple):
    """What a query must be answered UNDER for its answer to be the LUBM answer.

    ``cli`` is the ``purrdf query --entailment`` value that supplies ``needs``, or
    ``None`` where the query needs no inference at all. ``why`` cites the axiom or
    the paper's own statement that puts the query in this regime, so the mapping can
    be checked against ``univ-bench.owl`` rather than taken on trust.
    """

    needs: str
    cli: str | None
    why: str


NO_INFERENCE = "no inference"
SUBCLASS = "subClassOf"
SUBCLASS_SUBPROPERTY = "subClassOf + subPropertyOf"
IMPLICIT_STUDENT = "implicit GraduateStudent-to-Student"
TRANSITIVE = "transitive property"
REALIZATION = "realization (defined class Chair)"
INVERSE_SUBPROPERTY = "inverseOf + subPropertyOf"

REGIMES: dict[int, Regime] = {
    1: Regime(NO_INFERENCE, None, "Queries one asserted class and one asserted property."),
    2: Regime(NO_INFERENCE, None, "Three asserted classes and three asserted properties."),
    3: Regime(
        SUBCLASS,
        "rdfs",
        "Publication has a wide asserted subclass hierarchy; answers need rdfs9.",
    ),
    4: Regime(
        SUBCLASS,
        "rdfs",
        "Professor has a wide asserted subclass hierarchy; answers need rdfs9.",
    ),
    5: Regime(
        SUBCLASS_SUBPROPERTY,
        "rdfs",
        "Person is a deep asserted hierarchy AND memberOf has subproperties (rdfs7).",
    ),
    6: Regime(
        IMPLICIT_STUDENT,
        "owl-rl",
        "univ-bench.owl defines Student by owl:intersectionOf, so GraduateStudent's "
        "membership is DERIVED, never asserted. RDFS cannot reach it.",
    ),
    7: Regime(IMPLICIT_STUDENT, "owl-rl", "Same derived Student membership as Query 6."),
    8: Regime(IMPLICIT_STUDENT, "owl-rl", "Same derived Student membership as Query 6."),
    9: Regime(IMPLICIT_STUDENT, "owl-rl", "Same derived Student membership as Query 6."),
    10: Regime(
        IMPLICIT_STUDENT,
        "owl-rl",
        "Needs ONLY the derived GraduateStudent-to-Student step, per the paper.",
    ),
    11: Regime(
        TRANSITIVE,
        "owl-rl",
        "subOrganizationOf is an owl:TransitiveProperty; ResearchGroup-to-University "
        "is derived through Department.",
    ),
    12: Regime(
        REALIZATION,
        "owl-rl",
        "LUBM asserts no Chair. Chair is owl:intersectionOf(Person, headOf some "
        "Department), so membership must be realized.",
    ),
    13: Regime(
        INVERSE_SUBPROPERTY,
        "owl-rl",
        "hasAlumnus is owl:inverseOf degreeFrom, and the data states only "
        "degreeFrom's three subproperties.",
    ),
    14: Regime(NO_INFERENCE, None, "One asserted class, no hierarchy."),
}


class Applied(NamedTuple):
    """One recorded application of one rule to one query."""

    rule: str
    before: str
    after: str


class Query(NamedTuple):
    """A normalised query, with the audit trail that produced it."""

    number: int
    text: str
    applications: tuple[Applied, ...]
    regime: Regime


# A query block starts at a `# QueryN` header line and runs to the next one.
_HEADER = re.compile(r"^#\s*Query\s*(\d+)\s*$", re.MULTILINE)

# An absolute http(s) IRI that is NOT already inside angle brackets and is not the
# object of a PREFIX declaration (those are always bracketed upstream). The
# character class stops at whitespace and at the SPARQL delimiters that can never
# appear unescaped in an IRIREF.
_BARE_IRI = re.compile(r"(?<![<\w])(https?://[^\s<>\"{}|\\^`]+)")


def _split_blocks(text: str) -> list[tuple[int, str]]:
    """Split the published file into ``(number, block)`` pairs, in file order."""
    marks = list(_HEADER.finditer(text))
    blocks: list[tuple[int, str]] = []
    for index, mark in enumerate(marks):
        end = marks[index + 1].start() if index + 1 < len(marks) else len(text)
        blocks.append((int(mark.group(1)), text[mark.end() : end]))
    return blocks


def _select_line(head: str) -> str:
    """The SELECT line of a query head, for a provenance record that shows only it."""
    for line in head.splitlines():
        if line.lstrip().startswith("SELECT"):
            return line.strip()
    return head.strip()


def _strip_comments(block: str) -> str:
    """Drop the publisher's descriptive comment lines, keeping the query itself.

    The comments are prose ABOUT the query, not part of it, and they are also the
    part of the file most obviously covered by the absent licence grant. They are
    not reproduced; the regime table cites the paper instead.
    """
    return "\n".join(line for line in block.splitlines() if not line.lstrip().startswith("#"))


def normalise(number: int, block: str, namespace: str) -> Query:
    """Apply every rule to one query block, recording each application."""
    applied: list[Applied] = []
    text = _strip_comments(block).strip("\n")

    # R4 PREFIX-REBIND — before anything else, so the recorded before/after of the
    # later rules already show the namespace the query will actually run under.
    if DRAFT_NAMESPACE in text:
        rebound = text.replace(DRAFT_NAMESPACE, namespace)
        applied.append(Applied("R4 PREFIX-REBIND", DRAFT_NAMESPACE, namespace))
        text = rebound

    # R3 BARE-IRI — wrap absolute IRIs written without angle brackets.
    def wrap(match: re.Match[str]) -> str:
        applied.append(Applied("R3 BARE-IRI", match.group(1), f"<{match.group(1)}>"))
        return f"<{match.group(1)}>"

    text = _BARE_IRI.sub(wrap, text)

    # R1 PROJECTION-COMMA and R2 PATTERN-COMMA — the same draft-era comma, in two
    # different productions, recorded separately because they need different
    # justifications. The split point is the WHERE keyword: everything before it is
    # the projection, everything after it is the group graph pattern.
    head, keyword, body = text.partition("WHERE")
    if not keyword:
        sys.exit(f"FAIL: Query{number} has no WHERE clause; refusing to normalise it")

    if "," in head:
        cleaned = head.replace(",", "")
        # Record only the SELECT line itself. The PREFIX declarations above it are
        # untouched by this rule, and reprinting them would bury the one line that
        # actually changed in text that did not.
        applied.append(
            Applied(
                "R1 PROJECTION-COMMA",
                _select_line(head),
                _select_line(cleaned),
            )
        )
        head = cleaned

    if "," in body:
        # Replace each comma with a space rather than deleting it: the commas here
        # SEPARATE terms, and deleting one would weld two terms into a single token.
        cleaned = body.replace(",", " ")
        applied.append(Applied("R2 PATTERN-COMMA", body.strip(), cleaned.strip()))
        body = cleaned

    text = head + keyword + body

    # R5 TRAILING-SPACE — recorded so nothing in a byte-comparison is unexplained.
    lines = text.splitlines()
    if any(line != line.rstrip() for line in lines):
        applied.append(
            Applied(
                "R5 TRAILING-SPACE",
                f"{sum(1 for line in lines if line != line.rstrip())} line(s) with trailing space",
                "stripped",
            )
        )
        text = "\n".join(line.rstrip() for line in lines)

    regime = REGIMES.get(number)
    if regime is None:
        sys.exit(f"FAIL: Query{number} has no recorded entailment regime")

    return Query(number, text.strip() + "\n", tuple(applied), regime)


def load(source: Path, namespace: str) -> list[Query]:
    """Normalise every query in *source*, or exit non-zero explaining why not."""
    if not source.exists():
        sys.exit(
            f"FAIL: {source} is not in the cache.\n"
            "  Run `make benchmark-acquire` first: the LUBM queries carry no redistribution\n"
            "  grant, so they are fetched by digest at use time and never vendored here."
        )
    blocks = _split_blocks(source.read_text(encoding="utf-8"))
    if len(blocks) != 14:
        sys.exit(
            f"FAIL: expected 14 LUBM queries in {source}, found {len(blocks)}.\n"
            "  The pinned file changed shape; the digest pin and these rules must be "
            "re-checked together."
        )
    queries = [normalise(number, block, namespace) for number, block in blocks]
    seen = [query.number for query in queries]
    if seen != list(range(1, 15)):
        sys.exit(f"FAIL: LUBM queries are not 1..14 in order: {seen}")
    return queries


def emit(queries: list[Query], out: Path, namespace: str) -> None:
    """Write one ``.rq`` per query, the regime index, and the provenance record.

    Everything lands under ``target/``. The published file carries no redistribution
    grant, and a mechanically normalised copy of it is still a copy of it, so the
    output of this program is build output and never a tracked file.
    """
    out.mkdir(parents=True, exist_ok=True)
    for query in queries:
        (out / f"Q{query.number:02d}.rq").write_text(query.text, encoding="utf-8")

    # The index the lane reads: one row per query, tab-separated, in query order.
    index = ["\t".join(("id", "regime", "cli_regime", "file"))]
    for query in queries:
        index.append(
            "\t".join(
                (
                    f"Q{query.number}",
                    query.regime.needs,
                    query.regime.cli or "-",
                    f"Q{query.number:02d}.rq",
                )
            )
        )
    (out / "regimes.tsv").write_text("\n".join(index) + "\n", encoding="utf-8")
    (out / "provenance.txt").write_text(provenance(queries, namespace), encoding="utf-8")


def provenance(queries: list[Query], namespace: str) -> str:
    """Render the full audit trail: every rule application, per query."""
    lines = [
        "LUBM QUERY NORMALISATION -- PROVENANCE",
        "=" * 72,
        "",
        "Source: queries-sparql.txt as published by Lehigh, fetched by digest and NOT",
        "vendored. Every difference between that file and the queries this lane runs is",
        "one of the five mechanical rules below, applied by scripts/lubm-queries.py and",
        "recorded here with its before and after text. No query was hand-rewritten.",
        "",
        f"ub: rebound to {namespace}",
        "",
        "Cite: Y. Guo, Z. Pan and J. Heflin, 'LUBM: A Benchmark for OWL Knowledge Base",
        "Systems', Journal of Web Semantics 3(2).",
        "",
        "A RESULT COUNT IS ONLY MEANINGFUL AGAINST ITS REGIME. Compare two engines on a",
        "query only when both answered it under the same regime.",
        "",
    ]
    counts: dict[str, int] = {}
    for query in queries:
        lines.append("-" * 72)
        lines.append(
            f"Q{query.number}  regime: {query.regime.needs}"
            f"  (purrdf --entailment {query.regime.cli or 'none'})"
        )
        lines.append(f"    why: {query.regime.why}")
        if not query.applications:
            lines.append("    rules applied: none -- the published text already parses")
        for application in query.applications:
            counts[application.rule] = counts.get(application.rule, 0) + 1
            lines.append(f"    {application.rule}")
            lines.append(f"        before: {application.before}")
            lines.append(f"        after:  {application.after}")
        lines.append("")
        lines.append("    normalised query:")
        lines.extend(f"        {line}" for line in query.text.rstrip("\n").splitlines())
        lines.append("")
    lines.append("=" * 72)
    lines.append("RULE APPLICATION TOTALS")
    for rule in sorted(counts):
        lines.append(f"  {rule}: {counts[rule]}")
    lines.append("")
    return "\n".join(lines)


# ── The offline half of the self-test ───────────────────────────────────────────
#
# `self_test()` reads the PUBLISHED file, which is fetched by digest at use time
# and never vendored -- so it runs only inside `make lubm`, after a download, and
# never in a gate. That left the splitter and the loader's refusals uncovered in
# both directions. Everything below drives them over inline fixtures instead, so
# it can live in `make check`.


_PROSE_COMMENT_FIXTURE = """\
# Query 11, 12 and 13 exercise transitivity and inverse properties. This line
# OPENS with the same three words a delimiter does and is prose, which is the
# whole point of the fixture: a splitter matching `# Query` loosely cuts here.
# Query1
SELECT ?X
WHERE {?X rdf:type ub:GraduateStudent}
# Query2
SELECT ?X, ?Y
WHERE {?X ub:memberOf ?Y}
"""


def offline_self_test() -> int:
    """The fetch-free checks: the block splitter and the loader's refusals."""
    ok = True

    def check(condition: bool, label: str) -> None:
        nonlocal ok
        print(f"{'OK' if condition else 'SELF-TEST FAIL'}: {label}")
        ok = ok and condition

    def expect_exit(thunk, label: str, must_contain: list[str]) -> None:
        nonlocal ok
        try:
            thunk()
        except SystemExit as exc:
            message = str(exc.code)
            if not exc.code or isinstance(exc.code, int) and exc.code == 0:
                print(f"SELF-TEST FAIL: {label} exited zero")
                ok = False
                return
            missing = [needle for needle in must_contain if needle not in message]
            if missing:
                print(f"SELF-TEST FAIL: {label} did not say {missing}: {message}")
                ok = False
                return
            print(f"OK: {label}")
            return
        print(f"SELF-TEST FAIL: {label} did not refuse at all")
        ok = False

    # THE DELIMITER SPLIT, PROVEN RATHER THAN ASSERTED. The published file opens
    # with prose that contains the words "Query 11, 12 and 13". A splitter that
    # matched `# Query` loosely would cut there and mis-number everything after
    # it, so this fixture is built around exactly that sentence: the control line
    # differs from the two real delimiters in the one way that matters, and the
    # expected answer is two blocks numbered 1 and 2 -- never three, and never a
    # block numbered 11.
    blocks = _split_blocks(_PROSE_COMMENT_FIXTURE)
    check(
        [number for number, _ in blocks] == [1, 2],
        f"prose naming 'Query 11, 12 and 13' does not split a block (got {[n for n, _ in blocks]})",
    )
    check(
        _PROSE_COMMENT_FIXTURE.startswith("# Query 11, 12 and 13"),
        "the fixture OPENS on the prose line the split must survive, not merely contains it",
    )

    # The valid neighbour for the comma rule: a pre-final projection list loses
    # its commas, and the variables, their spelling and their ORDER survive.
    projection = normalise(2, blocks[1][1], PUBLISHED_NAMESPACE)
    head = projection.text.partition("WHERE")[0]
    check("," not in head, f"projection commas are removed (got {head.strip()!r})")
    # One exact expectation, not a disjunction. `findall` with no capture group
    # returns whole matches, so `["X", "Y"]` was unreachable -- and a disjunction
    # that tolerates two answers tolerates a regex whose meaning changed.
    projected = re.findall(r"\?\w+", head)
    check(
        projected == ["?X", "?Y"],
        f"the projection keeps both variables in order (got {projected})",
    )

    with tempfile.TemporaryDirectory() as raw:
        root = Path(raw)
        expect_exit(
            lambda: load(root / "absent.txt", PUBLISHED_NAMESPACE),
            "a queries file that is not in the cache is refused, naming the fetch step",
            ["is not in the cache"],
        )
        short = root / "short.txt"
        short.write_text(_PROSE_COMMENT_FIXTURE, encoding="utf-8")
        expect_exit(
            lambda: load(short, PUBLISHED_NAMESPACE),
            "a file holding fewer than 14 queries is refused, naming the count",
            ["expected 14 LUBM queries", "found 2"],
        )

    print("OFFLINE SELF-TEST PASS" if ok else "OFFLINE SELF-TEST FAIL")
    return 0 if ok else 1


def self_test(namespace: str) -> int:
    """Check the rules against the pinned file, including what they must NOT touch."""
    ok = True
    queries = load(PUBLISHED, namespace)

    def check(condition: bool, label: str) -> None:
        nonlocal ok
        print(f"{'OK' if condition else 'SELF-TEST FAIL'}: {label}")
        ok = ok and condition

    check(len(queries) == 14, "all 14 queries normalise")

    # R4 must reach every query: a query left on the draft namespace would answer
    # zero against every real dataset, silently.
    check(
        all(namespace in query.text for query in queries),
        f"every query binds ub: to {namespace}",
    )
    check(
        not any(DRAFT_NAMESPACE in query.text for query in queries),
        "no query still carries the 2004 draft namespace",
    )

    # R2 must touch Q7 and ONLY Q7. A rule that rewrote commas anywhere else could
    # silently destroy a legitimate SPARQL object list.
    touched = {q.number for q in queries for a in q.applications if a.rule == "R2 PATTERN-COMMA"}
    check(touched == {7}, f"R2 PATTERN-COMMA touched exactly Query 7 (touched {sorted(touched)})")

    # R3 must touch Q1 and Q3 and only those.
    bare = {q.number for q in queries for a in q.applications if a.rule == "R3 BARE-IRI"}
    check(bare == {1, 3}, f"R3 BARE-IRI touched exactly Queries 1 and 3 (touched {sorted(bare)})")

    # R1 must touch exactly the six queries that project more than one variable.
    projected = {q.number for q in queries for a in q.applications if a.rule == "R1 PROJECTION-COMMA"}
    check(
        projected == {2, 4, 7, 8, 9, 12},
        f"R1 PROJECTION-COMMA touched the six multi-variable projections (touched {sorted(projected)})",
    )

    # No comma may survive anywhere: one left behind is a parse error at run time.
    check(
        not any("," in query.text for query in queries),
        "no normalised query contains a comma",
    )

    # Every IRI is bracketed. A bare IRI that escaped R3 is a parse error.
    check(
        not any(_BARE_IRI.search(query.text) for query in queries),
        "no normalised query contains an unbracketed IRI",
    )

    # The rules must not invent or drop variables. The projected variable NAMES must
    # be exactly those the published file projects -- this is the check that a
    # "normalisation" did not quietly become a rewrite.
    published = _split_blocks(PUBLISHED.read_text(encoding="utf-8"))
    projections_unchanged = True
    for (number, block), query in zip(published, queries, strict=True):
        before = re.findall(r"\?\w+", _strip_comments(block).partition("WHERE")[0])
        after = re.findall(r"\?\w+", query.text.partition("WHERE")[0])
        if before != after:
            print(f"SELF-TEST FAIL: Q{number} projection changed: {before} -> {after}")
            projections_unchanged = False
    # This was `check(True, ...)`. The loop above set `ok` correctly so the exit
    # status was right, but the REPORT printed OK on the very run where the loop
    # had just printed SELF-TEST FAIL -- a self-test whose output contradicted
    # itself, which is worse than one that stays quiet.
    check(projections_unchanged, "every projection keeps its variables, spelling and order")

    # Every query carries a regime, and every regime names a CLI value the binary
    # actually offers (or none at all).
    legal = {None, "simple", "rdf", "rdfs", "owl-rl", "owl-direct", "rif", "d"}
    check(
        all(query.regime.cli in legal for query in queries),
        "every regime maps to an --entailment value the CLI offers",
    )
    check(
        {q.number for q in queries if q.regime.cli is None} == {1, 2, 14},
        "exactly Q1, Q2 and Q14 need no inference",
    )

    print("SELF-TEST PASS" if ok else "SELF-TEST FAIL")
    return 0 if ok else 1


def main() -> int:
    parser = argparse.ArgumentParser(description="Normalise the 14 LUBM queries.")
    parser.add_argument(
        "--namespace",
        default=PUBLISHED_NAMESPACE,
        help="the ub: namespace the target dataset carries (R4's target)",
    )
    parser.add_argument("--out", type=Path, help="directory to write .rq files into")
    parser.add_argument("--provenance", action="store_true", help="print the audit trail")
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument(
        "--offline-self-test",
        action="store_true",
        help="the fetch-free checks, suitable for a gate",
    )
    args = parser.parse_args()

    if args.offline_self_test:
        return offline_self_test()

    if args.self_test:
        return self_test(args.namespace)

    queries = load(PUBLISHED, args.namespace)
    if args.out:
        emit(queries, args.out, args.namespace)
        print(f"wrote {len(queries)} normalised queries to {args.out}")
    if args.provenance or not args.out:
        print(provenance(queries, args.namespace))
    return 0


if __name__ == "__main__":
    sys.exit(main())
