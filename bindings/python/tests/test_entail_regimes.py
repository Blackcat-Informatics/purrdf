# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""`purrdf.entail` — SPARQL entailment-regime materialization from Python.

This is the surface the originating request asked for: the OWL-RL (and RDFS, RDF,
Simple) materialization the Rust and C-ABI hosts already had, reachable from
Python without reaching into `purrdf_native`.

Not to be confused with `tests/test_entail.py`, which covers SHACL-AF `sh:rule`
entailment — a different mechanism that needs a shapes graph.

What is asserted here, and why:

* **A closure really closes.** OWL-RL derives triples a plain parse does not, and
  every base triple survives.
* **Byte determinism.** Repeated calls produce identical bytes, for both the
  closure and the report. A chase that leaked hash order, a clock or an address
  would diverge here.
* **The honest gap.** `rules(OWL_RL)` is the 78-rule specification table;
  `implemented_rules(OWL_RL)` is the subset this workspace fires. The two are
  asserted to PARTITION the table — fired and missing cover it exactly, without
  overlap, in specification order — and the missing half is asserted to be
  exactly the report's `missing` lines, so the report and the inventory cannot
  drift apart. The gap is legitimately EMPTY for a regime whose table is
  complete, which OWL-RL's and D's now are; an assertion that it is non-empty
  would pin a capability gap that has been closed.
* **One artifact, four hosts.** The committed tri-host golden vector
  (`crates/validate/tests/fixtures/regime-boundary.vectors`) is walked through
  `materialize_nt` here, so Python checks the same bytes the Rust test, the C ABI
  and the WASM module do rather than a fixture of its own.
* **Every regime materializes.** All seven close; none is refused for being the
  regime it is. Two of them need an INPUT rather than permission — `rif` its rule
  document and `owl-direct` a query's class expressions — and `program` is the
  parameter that carries the first. `owl-direct` takes none here because this
  surface closes a dataset rather than answering a query, so what runs is the
  query-independent tableau augmentation.
* **Refusals are typed and legible.** A malformed document, an unknown regime
  spelling, and a `program` that is wrong for the regime all raise `ValueError`,
  and the spelling error names the whole accepted set.
"""

from __future__ import annotations

from pathlib import Path

import pytest

import purrdf
from purrdf import RdfDataset, RdfFormat, entail

# ── Fixtures (example.org, per the repository's vocabulary rule) ────────────────

RDF_TYPE = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"
RDFS_SUB_CLASS_OF = "http://www.w3.org/2000/01/rdf-schema#subClassOf"
OWL_SYMMETRIC_PROPERTY = "http://www.w3.org/2002/07/owl#SymmetricProperty"

# `A ⊑ B`, one instance of `A`, and a symmetric property with one use of it.
SCHEMA = (
    f"<https://example.org/A> <{RDFS_SUB_CLASS_OF}> <https://example.org/B> .\n"
    f"<https://example.org/x> <{RDF_TYPE}> <https://example.org/A> .\n"
    f"<https://example.org/p> <{RDF_TYPE}> <{OWL_SYMMETRIC_PROPERTY}> .\n"
    "<https://example.org/x> <https://example.org/p> <https://example.org/y> .\n"
)

# cax-sco: the instance of `A` is also an instance of `B`.
SUBCLASS_INFERENCE = f"<https://example.org/x> <{RDF_TYPE}> <https://example.org/B> ."
# prp-symp: a symmetric property's use holds in the other direction too. Purely
# OWL — no RDFS rule licenses it.
SYMMETRY_INFERENCE = (
    "<https://example.org/y> <https://example.org/p> <https://example.org/x> ."
)

# Every member of the Python-visible enum, so a member added without a test is a
# failure rather than an omission.
ALL_REGIMES = [
    entail.Regime.SIMPLE,
    entail.Regime.RDF,
    entail.Regime.RDFS,
    entail.Regime.OWL_RL,
    entail.Regime.OWL_DIRECT,
    entail.Regime.RIF,
    entail.Regime.D,
]
# A normative RIF-in-XML rule document: `?x a ex:A` ⟹ `?x a ex:B`.
#
# `rif` is the one regime whose calculus is the CALLER's rather than a
# specification's, so it is the one spelling whose `program` argument is a document.
RIF_PROGRAM = (
    '<Document xmlns="http://www.w3.org/2007/rif#"><payload><Group><sentence><Forall><declare><Var>x</Var></declare><formula><Implies><if><Frame><object><Var>x</Var></object><slot><Const type="http://www.w3.org/2007/rif#iri">http://www.w3.org/1999/02/22-rdf-syntax-ns#type</Const><Const type="http://www.w3.org/2007/rif#iri">https://example.org/A</Const></slot></Frame></if><then><Frame><object><Var>x</Var></object><slot><Const type="http://www.w3.org/2007/rif#iri">http://www.w3.org/1999/02/22-rdf-syntax-ns#type</Const><Const type="http://www.w3.org/2007/rif#iri">https://example.org/B</Const></slot></Frame></then></Implies></formula></Forall></sentence></Group></payload></Document>'
)
# The conclusion RIF_PROGRAM licenses over SCHEMA, which is also what cax-sco
# licenses under OWL-RL — so the `rif` lane deriving it shows the RULE fired.
RIF_INFERENCE = SUBCLASS_INFERENCE

# Every regime, with the `program` its call takes.
#
# THE POINT OF THIS TABLE is that it has seven rows. It replaces a
# `MATERIALIZABLE`/`NOT_MATERIALIZABLE` split in which two of the seven were
# exempt from every cross-cutting assertion below and covered only by a refusal
# test. `materialize` is total over its parameter now, so they are not exempt.
REGIME_CALLS = [
    (entail.Regime.SIMPLE, ""),
    (entail.Regime.RDF, ""),
    (entail.Regime.RDFS, ""),
    (entail.Regime.OWL_RL, ""),
    (entail.Regime.OWL_DIRECT, ""),
    (entail.Regime.RIF, RIF_PROGRAM),
    (entail.Regime.D, ""),
]
# The regimes whose whole input is a rule table this workspace states — the ones
# `rules()` minus `implemented_rules()` is arithmetic about. The two
# query-directed lanes have no such table (`rules()` is `[]` for both), so the
# inventory assertions below range over these and say why.
RULE_TABLE_REGIMES = [
    entail.Regime.SIMPLE,
    entail.Regime.RDF,
    entail.Regime.RDFS,
    entail.Regime.OWL_RL,
    entail.Regime.D,
]
# The regimes with no rule table of their own.
NO_RULE_TABLE = [
    entail.Regime.OWL_DIRECT,
    entail.Regime.RIF,
]
ACCEPTED_SPELLINGS = [
    "simple",
    "rdf",
    "rdfs",
    "owl-rl",
    "owl-direct",
    "rif",
    "d",
]


def _dataset(text: str = SCHEMA) -> RdfDataset:
    """Freeze `text` (N-Quads) into the native dataset handle."""
    return RdfDataset(text, RdfFormat.N_QUADS)


# ── The closure really closes ───────────────────────────────────────────────────


def test_owl_rl_derives_what_parsing_does_not() -> None:
    """OWL-RL infers triples that are in neither the input nor a Simple closure."""
    closure, _report = entail.materialize(_dataset(), entail.Regime.OWL_RL, "")
    closed = closure.to_nquads()

    assert SUBCLASS_INFERENCE in closed, closed
    assert SYMMETRY_INFERENCE in closed, closed
    # …and neither is in the input, so the closure is doing the work.
    assert SUBCLASS_INFERENCE not in SCHEMA
    assert SYMMETRY_INFERENCE not in SCHEMA
    # `simple` is the identity closure: the same input yields neither.
    identity, _ = entail.materialize(_dataset(), entail.Regime.SIMPLE, "")
    assert SUBCLASS_INFERENCE not in identity.to_nquads()
    assert SYMMETRY_INFERENCE not in identity.to_nquads()


def test_symmetry_is_owl_only_not_rdfs() -> None:
    """`prp-symp` is an OWL rule: RDFS must NOT license the symmetric triple."""
    rdfs, _ = entail.materialize(_dataset(), entail.Regime.RDFS, "")
    assert SUBCLASS_INFERENCE in rdfs.to_nquads(), "rdfs9/cax-sco still fires"
    assert SYMMETRY_INFERENCE not in rdfs.to_nquads()


def test_base_triples_survive_the_closure() -> None:
    """A closure adds; it never drops. Every input triple is still present."""
    closure, _ = entail.materialize(_dataset(), entail.Regime.OWL_RL, "")
    closed = closure.to_nquads()
    for line in SCHEMA.splitlines():
        assert line in closed, f"input triple dropped by the closure: {line}"
    assert closure.quad_count() > len(SCHEMA.splitlines())


def test_report_names_the_rules_that_fired() -> None:
    """The report is not optional and says what the run actually did."""
    _closure, report = entail.materialize(_dataset(), entail.Regime.OWL_RL, "")
    assert report.startswith("purrdf-reasoning-report 4\n")
    assert "\nregime owl-rl\n" in report
    # The conclusion counts are the engine's to report, so only the fact that
    # these two rules ran is asserted here — the counts live in the Rust golden
    # vector, where a change to them is a reviewed diff rather than a surprise.
    assert "\nfired cax-sco " in report
    assert "\nfired prp-symp " in report
    assert "\ncontract-hash " in report
    assert report.endswith("inconsistency none\n")


# ── The two entry points are one path ───────────────────────────────────────────


def test_materialize_nt_matches_the_dataset_path_byte_for_byte() -> None:
    """The string wrapper is the same boundary call, not a second engine path."""
    text_closure, text_report = entail.materialize_nt(SCHEMA, entail.Regime.OWL_RL, "")
    dataset_closure, dataset_report = entail.materialize(
        _dataset(), entail.Regime.OWL_RL, ""
    )
    assert text_closure == dataset_closure.to_nquads()
    assert text_report == dataset_report


def test_to_nquads_round_trips() -> None:
    """A serialized closure re-parses, and re-serializes to the same bytes."""
    closure, _ = entail.materialize(_dataset(), entail.Regime.OWL_RL, "")
    serialized = closure.to_nquads()
    reparsed = RdfDataset(serialized, RdfFormat.N_QUADS)
    assert reparsed.to_nquads() == serialized
    assert reparsed.quad_count() == closure.quad_count()
    # The same handle type the root surface exposes, not a private twin.
    assert isinstance(reparsed, purrdf.RdfDataset)


# ── Byte determinism ────────────────────────────────────────────────────────────


@pytest.mark.parametrize("regime,program", REGIME_CALLS, ids=lambda r: str(r))
def test_repeated_calls_are_byte_identical(regime: entail.Regime, program: str) -> None:
    """Twelve runs of EVERY regime produce identical bytes.

    A closure that leaked hash iteration order, a clock, a path or an address
    would diverge across these calls; a one-in-two divergence cannot pass by luck
    at this repetition count.
    """
    first_closure, first_report = entail.materialize(_dataset(), regime, program)
    first = first_closure.to_nquads()
    for _ in range(11):
        closure, report = entail.materialize(_dataset(), regime, program)
        assert closure.to_nquads() == first
        assert report == first_report
    # …and the text path is stable in the same way.
    text = entail.materialize_nt(SCHEMA, regime, program)
    assert entail.materialize_nt(SCHEMA, regime, program) == text


# ── Every regime member is accepted ─────────────────────────────────────────────


@pytest.mark.parametrize("regime,program", REGIME_CALLS, ids=lambda r: str(r))
def test_every_regime_member_materializes(regime: entail.Regime, program: str) -> None:
    """EVERY `Regime` member closes. None is refused for being the regime it is.

    Falsifiable against the behaviour this replaced: `OWL_DIRECT` and `RIF` raised
    `ValueError` here with a message listing the five regimes that "can be
    forward-materialized". `materialize` is total over its parameter now — what
    those two needed was an input, and `program` is where it goes.
    """
    assert isinstance(entail.rules(regime), list)
    assert isinstance(entail.implemented_rules(regime), list)
    closure, report = entail.materialize(_dataset(), regime, program)
    assert closure.quad_count() >= 4
    assert report.startswith("purrdf-reasoning-report 4\n")
    assert report.endswith("inconsistency none\n")
    # …and the text path agrees, byte for byte, on the same regime and program.
    text_closure, text_report = entail.materialize_nt(SCHEMA, regime, program)
    assert text_closure == closure.to_nquads()
    assert text_report == report


def test_the_rif_lane_entails_under_the_supplied_rules() -> None:
    """The `rif` lane runs the CALLER's rules, and nothing else's.

    The rule set is the whole calculus for this regime: the conclusion appears
    because the rule fired, and the RDFS axiomatic vocabulary does not, because
    no rule table ran.
    """
    closure, report = entail.materialize(_dataset(), entail.Regime.RIF, RIF_PROGRAM)
    closed = closure.to_nquads()
    assert RIF_INFERENCE in closed, closed
    assert "\nregime rif\n" in report
    assert "http://www.w3.org/2000/01/rdf-schema#Resource" not in closed


def test_owl_direct_materializes_the_query_independent_augmentation() -> None:
    """`OWL_DIRECT` closes: the tableau states what it decides about named terms."""
    closure, report = entail.materialize(_dataset(), entail.Regime.OWL_DIRECT, "")
    assert SUBCLASS_INFERENCE in closure.to_nquads(), closure.to_nquads()
    assert "\nregime owl-direct\n" in report


@pytest.mark.parametrize("regime,program", REGIME_CALLS, ids=lambda r: str(r))
def test_a_rule_document_belongs_to_rif_alone(
    regime: entail.Regime, program: str
) -> None:
    """A `program` for a regime that takes none is refused, never discarded."""
    if program:
        return
    for call in (
        lambda: entail.materialize(_dataset(), regime, RIF_PROGRAM),
        lambda: entail.materialize_nt(SCHEMA, regime, RIF_PROGRAM),
    ):
        with pytest.raises(ValueError, match="takes no rule document"):
            call()


@pytest.mark.parametrize("spelling", ACCEPTED_SPELLINGS)
def test_cli_spellings_are_accepted_too(spelling: str) -> None:
    """One spelling works from the CLI, the C ABI, WASM and Python."""
    member = getattr(entail.Regime, spelling.upper().replace("-", "_"))
    assert entail.rules(spelling) == entail.rules(member)
    assert entail.implemented_rules(spelling) == entail.implemented_rules(member)


# ── The honest gap ──────────────────────────────────────────────────────────────


def test_owl_rl_rule_inventory_is_the_specification_table() -> None:
    """`rules(OWL_RL)` is all 78 rules of OWL 2 Profiles §4.3, Tables 4–9."""
    spec = entail.rules(entail.Regime.OWL_RL)
    assert len(spec) == 78
    assert len(set(spec)) == 78, "the table has no duplicates"
    assert "cax-sco" in spec
    assert "prp-symp" in spec
    # RDFS is its own 18-rule table; RDF is 3; the rest have none.
    assert len(entail.rules(entail.Regime.RDFS)) == 18
    assert len(entail.rules(entail.Regime.RDF)) == 3
    # D is the datatype table; Simple and the two query-directed lanes state none.
    assert len(entail.rules(entail.Regime.D)) == 5
    for regime in [entail.Regime.SIMPLE, *NO_RULE_TABLE]:
        assert entail.rules(regime) == []
        assert entail.implemented_rules(regime) == []


def test_implemented_rules_are_measurable_against_the_specification_table() -> None:
    """The point of exposing both: a caller can MEASURE coverage, not trust prose.

    The assertions here must survive the table filling up. An earlier version
    demanded a *strict* subset and a *non-empty* gap — which reads as
    count-independent but is not: both encode "the gap never closes", so
    completing the table failed the test that existed to track it. What is
    actually invariant is the partition (fired and missing cover the table
    exactly, without overlap) and the ordering.
    """
    spec = entail.rules(entail.Regime.OWL_RL)
    fired = entail.implemented_rules(entail.Regime.OWL_RL)

    assert fired, "OWL-RL fires at least one rule"
    assert set(fired) <= set(spec), "every implemented rule is a specification rule"
    gap = [rule for rule in spec if rule not in fired]
    assert len(fired) + len(gap) == len(spec), "fired and missing partition the table"
    # Same relative order as the specification table (a subsequence, not a set).
    assert [rule for rule in spec if rule in fired] == fired


def test_extensions_name_what_this_build_adds_beyond_the_table() -> None:
    """A third inventory, disjoint from the other two, answerable without materializing.

    `rules()` and `implemented_rules()` are both statements about the
    specification table. Neither can express "this build also fires a sound rule
    the table omits", and before this was bound the only way to find that out was
    to close a dataset and read the report's `extension` line. Asking is now a
    question in its own right.
    """
    added = entail.extensions(entail.Regime.OWL_RL)
    assert added == ["ext-eq-diff-sym"]

    # Extending a lane is a decision taken per lane; only one has been taken.
    for regime in [entail.Regime.SIMPLE, entail.Regime.RDF, entail.Regime.RDFS,
                   entail.Regime.D, *NO_RULE_TABLE]:
        assert entail.extensions(regime) == [], str(regime)

    # The load-bearing invariant, over EVERY regime: an extension appears in
    # neither normative inventory. The 78 stays 78 because what a sound rule the
    # table omits does to this build does not change what the table says.
    for regime in [*RULE_TABLE_REGIMES, *NO_RULE_TABLE]:
        spec = entail.rules(regime)
        fired = entail.implemented_rules(regime)
        for rule in entail.extensions(regime):
            assert rule not in spec, f"{regime}: {rule} is not a specification rule"
            assert rule not in fired, f"{regime}: {rule} is not an implemented rule"
    assert len(entail.rules(entail.Regime.OWL_RL)) == 78
    assert len(entail.implemented_rules(entail.Regime.OWL_RL)) == 78

    # And the report names the same rules the inventory does, so the two
    # disclosures cannot drift apart.
    _closure, report = entail.materialize(_dataset(), entail.Regime.OWL_RL, "")
    reported = [
        line.removeprefix("extension ")
        for line in report.splitlines()
        if line.startswith("extension ")
    ]
    assert reported == added


def test_extensions_rejects_an_unknown_regime() -> None:
    """The same hard failure the other two inventories give, naming the accepted set."""
    with pytest.raises(ValueError, match="accepted: simple, rdf, rdfs"):
        entail.extensions("rdfs-plus")


@pytest.mark.parametrize("regime", RULE_TABLE_REGIMES, ids=lambda r: str(r))
def test_the_gap_is_exactly_the_reports_missing_lines(regime: entail.Regime) -> None:
    """The inventory and the report cannot drift apart."""
    spec = entail.rules(regime)
    fired = entail.implemented_rules(regime)
    gap = [rule for rule in spec if rule not in fired]

    _closure, report = entail.materialize(_dataset(), regime, "")
    missing = [
        line.removeprefix("missing ")
        for line in report.splitlines()
        if line.startswith("missing ")
    ]
    assert missing == gap
    if gap:
        assert f"\ncompleteness sound-incomplete {len(gap)}\n" in report
    else:
        # A complete rule table reports `exact`, or `exact-within-boundaries`
        # when the run still met a construct outside the table — a distinction
        # the report draws precisely so a complete table cannot claim more than
        # it delivered.
        completeness = next(
            line for line in report.splitlines() if line.startswith("completeness ")
        )
        assert completeness in ("completeness exact", "completeness exact-within-boundaries")


# ── Refusals ────────────────────────────────────────────────────────────────────


def test_malformed_input_raises_value_error() -> None:
    """A malformed document is an error, not an empty closure."""
    with pytest.raises(ValueError):
        entail.materialize_nt("this is not n-quads\n", entail.Regime.RDFS, "")
    with pytest.raises(ValueError):
        RdfDataset("this is not n-quads\n", RdfFormat.N_QUADS)


def test_unknown_regime_spelling_names_the_accepted_set() -> None:
    """The error a caller three language boundaries away has to act on."""
    for call in (
        lambda: entail.materialize(_dataset(), "rdfs-plus", ""),
        lambda: entail.materialize_nt(SCHEMA, "rdfs-plus", ""),
        lambda: entail.rules("rdfs-plus"),
        lambda: entail.implemented_rules("rdfs-plus"),
    ):
        with pytest.raises(ValueError) as raised:
            call()
        message = str(raised.value)
        assert "rdfs-plus" in message
        for spelling in ACCEPTED_SPELLINGS:
            assert spelling in message, f"{message} omits {spelling}"


def test_regime_spellings_are_case_sensitive() -> None:
    """Matching is exact, as the CLI writes the names."""
    with pytest.raises(ValueError, match="OWL-RL"):
        entail.rules("OWL-RL")


def test_a_non_regime_argument_names_the_accepted_set() -> None:
    """An argument that is neither a member nor a spelling is refused legibly."""
    with pytest.raises(ValueError) as raised:
        entail.rules(object())  # type: ignore[arg-type]
    message = str(raised.value)
    assert "purrdf.entail.Regime" in message
    for spelling in ACCEPTED_SPELLINGS:
        assert spelling in message, f"{message} omits {spelling}"


# ── One artifact, four hosts ────────────────────────────────────────────────────

# The COMMITTED tri-host golden vector. The `purrdf-validate` Rust test, the C
# smoke (`crates/rdf-capi/tests/smoke.c`) and the WASM module's
# `entailCheckGoldenVectors` all check these very bytes; this walks them from
# Python, so "one artifact, four hosts" is a claim Python participates in rather
# than one made on its behalf.
GOLDEN_VECTOR = (
    Path(__file__).resolve().parents[3]
    / "crates"
    / "validate"
    / "tests"
    / "fixtures"
    / "regime-boundary.vectors"
)

# The directives that open a body, mapped to the case field they fill. `@program`
# is the regime's own rule document; absent is the empty program, which is what
# every regime but `rif` takes.
_BODY_DIRECTIVES = {
    "@input": "input",
    "@program": "program",
    "@closure": "closure",
    "@report": "report",
}


def _golden_cases() -> list[dict[str, str]]:
    """Parse the committed artifact into its cases.

    The format is line-oriented and deliberately dependency-free (see
    `parse_regime_vectors` in `purrdf-validate`): a line starting with `@` is a
    directive, every other line belongs to the body the last body-directive
    opened, and outside a body only blank lines and `#` comments are legal.
    """
    cases: list[dict[str, str]] = []
    case: dict[str, str] = {}
    section: str | None = None
    for raw in GOLDEN_VECTOR.read_text(encoding="utf-8").splitlines(keepends=True):
        line = raw.rstrip("\n")
        if not line.startswith("@"):
            if section is not None:
                case[section] = case.get(section, "") + raw
            continue
        section = None
        keyword, _, argument = line.partition(" ")
        if keyword == "@case":
            case = {"name": argument.strip()}
        elif keyword == "@regime":
            case["regime"] = argument.strip()
        elif keyword in _BODY_DIRECTIVES:
            section = _BODY_DIRECTIVES[keyword]
            case.setdefault(section, "")
        elif keyword == "@end":
            cases.append(case)
            case = {}
    return cases


def test_the_golden_vector_artifact_is_readable() -> None:
    """The artifact exists, parses, and covers every regime — all seven."""
    cases = _golden_cases()
    assert cases, GOLDEN_VECTOR
    covered = {case["regime"] for case in cases}
    for regime in ALL_REGIMES:
        spelling = str(regime).rsplit(".", 1)[-1].lower().replace("_", "-")
        assert spelling in covered, f"{spelling} is not covered by {GOLDEN_VECTOR}"


@pytest.mark.parametrize(
    "case", _golden_cases(), ids=lambda case: str(case["name"])
)
def test_the_golden_vector_matches_through_python(case: dict[str, str]) -> None:
    """Python produces the committed bytes, for both the closure and the report.

    A divergence here is the same one failing artifact the other three hosts
    report, not a fourth fixture that quietly stopped agreeing with them.
    """
    closure, report = entail.materialize_nt(
        case["input"], case["regime"], case.get("program", "")
    )
    assert closure == case["closure"]
    assert report == case["report"]


# ── The two things the report could not say from Python ─────────────────────────

OWL_DISJOINT_WITH = "http://www.w3.org/2002/07/owl#disjointWith"

# Two disjoint classes and one instance of both: OWL 2 RL's `cax-dw`, whose three
# premises are exactly these three triples, in this order.
INCONSISTENT = (
    f"<https://example.org/A> <{OWL_DISJOINT_WITH}> <https://example.org/B> .\n"
    f"<https://example.org/x> <{RDF_TYPE}> <https://example.org/A> .\n"
    f"<https://example.org/x> <{RDF_TYPE}> <https://example.org/B> .\n"
)


def test_the_withheld_surrogate_count_is_visible_from_python() -> None:
    """`rdfD1`, `rdfD1a`, `rdfs14` and `rdfs14a` have exactly one observable, and it
    reaches Python.

    All four fire, and every conclusion they reach mentions a blank node the chase
    minted — which a SPARQL entailment regime may not answer with, because its
    answers are drawn from the scoping graph. So none of them can ever appear in a
    `fired` line, and this count is the only evidence they ran at all. It used to
    be emitted by the CLI's own renderer alone, so from Python the four rules were
    invisible: the `boundary surrogate` paragraph said its conclusions were
    "counted here" and pointed at a number that was not in the string.
    """
    def withheld(regime: entail.Regime) -> int:
        _closure, report = entail.materialize_nt(SCHEMA, regime, "")
        for line in report.splitlines():
            if line.startswith("withheld-surrogates "):
                return int(line.removeprefix("withheld-surrogates "))
        raise AssertionError(f"no withheld-surrogates line:\n{report}")

    # RDFS states all four existential rules, and withholds what they conclude.
    assert withheld(entail.Regime.RDFS) > 0
    # OWL 2 RL states none of them, so there is nothing to withhold; the identity
    # closure evaluates nothing at all. Both are facts about the LANE, which is what
    # makes the RDFS number a measurement rather than a constant.
    assert withheld(entail.Regime.OWL_RL) == 0
    assert withheld(entail.Regime.SIMPLE) == 0


def test_an_inconsistent_run_raises_with_its_report_and_witness_triples() -> None:
    """An inconsistent knowledge base has no closure — and still has a run.

    The refusal used to be a `Display` one-liner that read only the premise COUNT,
    so the caller whose data was bad was the only caller who got no report at all
    and `inconsistency` was the constant `none` on every host. The message now
    carries the whole certificate: the rule that refused, the graph whose closure
    refused, and the asserted triples that satisfied the rule in that rule's own
    premise order.
    """
    with pytest.raises(ValueError) as raised:
        entail.materialize_nt(INCONSISTENT, entail.Regime.OWL_RL, "")
    message = str(raised.value)

    assert "cax-dw was satisfied by 3 asserted triples" in message
    # The certificate begins at the banner, so a caller splits there rather than
    # parsing prose.
    banner = "purrdf-reasoning-report 4\n"
    assert banner in message
    report = message[message.index(banner) :]
    assert report.startswith(f"{banner}regime owl-rl\n")
    assert "\ninconsistency cax-dw premises 3\n" in report
    assert "\ninconsistency-graph default\n" in report
    assert (
        f"\ninconsistency-premise <https://example.org/A> <{OWL_DISJOINT_WITH}> "
        "<https://example.org/B>\n"
    ) in report
    assert report.count("\ninconsistency-premise ") == 3
    # The run is DESCRIBED, not merely refused: it cost a budget and named a calculus.
    assert "\nbudget join-steps " in report
    assert "\ncontract-hash " in report


def test_the_dataset_path_refuses_an_inconsistent_run_the_same_way() -> None:
    """The parsed-dataset entry point carries the same evidence as the text one."""
    dataset = _dataset(INCONSISTENT)
    with pytest.raises(ValueError) as raised:
        entail.materialize(dataset, entail.Regime.OWL_RL, "")
    message = str(raised.value)
    assert "purrdf-reasoning-report 4\n" in message
    assert message.count("\ninconsistency-premise ") == 3


# ── Conclusion-directed entailment: the chase lane's three services ─────────────


def test_certain_answers_enumerate_entailed_bindings_and_disclose_completeness() -> None:
    """A row is a substitution the knowledge base ENTAILS the pattern under.

    Not "a substitution present in one closure": SPARQL's entailment regimes define
    the answers to a basic graph pattern as the CERTAIN answers, true in every model.
    `?c` therefore ranges over the entailed types of `x`, which is a strict superset
    of the asserted one.
    """
    pattern = f"<https://example.org/x> <{RDF_TYPE}> ?c .\n"
    answer, certificate = entail.certain_answers(entail.Regime.OWL_RL, SCHEMA, pattern)

    assert answer.startswith("mechanism strict-table\nvar c\n")
    # `A` is asserted; `B` is derived by cax-sco and is a certain answer all the same.
    assert "\nrow <https://example.org/A>\n" in answer
    assert "\nrow <https://example.org/B>\n" in answer
    # No `limit` line IS the claim that the row set is exhaustive. There is
    # deliberately no `complete true` line beside it: that would be a boolean
    # function of lines already rendered.
    assert "\nlimit " not in answer

    # The rows arrive with the run that produced them, on the materialization lane's
    # own banner — an empty row set is the answer a caller is most likely to act on
    # and the one that says least on its own.
    assert certificate.startswith("purrdf-reasoning-report 4\n")
    assert "\nmechanism strict-table " in certificate


def test_graph_entails_gives_three_verdicts_and_names_the_mechanism() -> None:
    """THREE verdicts, never two — and the answer says which mechanism reached one.

    `not-entailed` is a PROOF: the procedure was complete for this premise, so the
    absence of a mapping is the absence of an entailment. Collapsing an `undecided`
    into it would turn a limitation of this library into a false statement about the
    caller's data.
    """
    entailed = f"{SUBCLASS_INFERENCE}\n"
    answer, certificate = entail.graph_entails(entail.Regime.OWL_RL, SCHEMA, entailed)
    assert answer == "mechanism strict-table\nentailment entailed\n"
    assert "\nfired cax-sco " in certificate

    never = f"<https://example.org/x> <{RDF_TYPE}> <https://example.org/Never> .\n"
    answer, _ = entail.graph_entails(entail.Regime.OWL_RL, SCHEMA, never)
    assert answer.startswith("mechanism strict-table\nentailment not-entailed\n")
    assert "\nmiss " in answer

    # `D` realizes datatype entailment as the five dt-* rules and states no theorem
    # that they are all of it, so it can PROVE an entailment and never refute one.
    answer, _ = entail.graph_entails(entail.Regime.D, SCHEMA, never)
    assert answer.startswith("mechanism strict-table\nentailment undecided\n")
    assert "\nundecided " in answer


def test_verify_entailment_re_decides_its_own_warrant() -> None:
    """The warrant is re-decided without running a reasoner.

    `warrant absent` / `verified not-applicable` is a not-entailed or an undecided:
    there is no evidence to re-decide, and a `false` there would read as a check that
    ran and failed rather than one that never applied.
    """
    entailed = f"{SUBCLASS_INFERENCE}\n"
    answer, certificate = entail.verify_entailment(
        entail.Regime.OWL_RL, SCHEMA, entailed
    )
    assert answer.startswith("mechanism strict-table\nentailment entailed\n")
    assert answer.endswith("warrant present\nverified true\n")
    assert certificate.startswith("purrdf-reasoning-report 4\n")

    never = f"<https://example.org/x> <{RDF_TYPE}> <https://example.org/Never> .\n"
    answer, _ = entail.verify_entailment(entail.Regime.OWL_RL, SCHEMA, never)
    assert answer.endswith("warrant absent\nverified not-applicable\n")


def test_the_regimes_defined_by_a_missing_input_are_refused_by_name() -> None:
    """`OWL_DIRECT` and `RIF` are refused rather than served by a weaker lane.

    Each is defined by an input these signatures do not carry — a query's class
    expressions, and the caller's rule document — so accepting them and quietly doing
    something else would be worse than refusing.
    """
    entailed = f"{SUBCLASS_INFERENCE}\n"
    pattern = f"<https://example.org/x> <{RDF_TYPE}> ?c .\n"
    for regime, spelling in [
        (entail.Regime.OWL_DIRECT, "owl-direct"),
        (entail.Regime.RIF, "rif"),
    ]:
        with pytest.raises(ValueError) as raised:
            entail.graph_entails(regime, SCHEMA, entailed)
        assert spelling in str(raised.value)
        with pytest.raises(ValueError) as raised:
            entail.certain_answers(regime, SCHEMA, pattern)
        assert spelling in str(raised.value)
