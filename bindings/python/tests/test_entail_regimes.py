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
  `implemented_rules(OWL_RL)` is the strictly smaller set this workspace fires.
  The gap is asserted to be non-empty and to be exactly the report's `missing`
  lines — the report and the inventory cannot drift apart.
* **Refusals are typed and legible.** A malformed document, an unknown regime
  spelling, and a regime that cannot be forward-materialized all raise
  `ValueError`, and the spelling error names the whole accepted set.
"""

from __future__ import annotations

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
MATERIALIZABLE = [
    entail.Regime.SIMPLE,
    entail.Regime.RDF,
    entail.Regime.RDFS,
    entail.Regime.OWL_RL,
]
# Exactly two regimes are refused, both for spec-inherent reasons: OWL-Direct
# needs the query's class expressions and RIF needs a parsed rule set. D was
# once here; it is materializable now, and a test asserting otherwise would
# pin a capability gap that has been closed.
NOT_MATERIALIZABLE = [
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
    closure, _report = entail.materialize(_dataset(), entail.Regime.OWL_RL)
    closed = closure.to_nquads()

    assert SUBCLASS_INFERENCE in closed, closed
    assert SYMMETRY_INFERENCE in closed, closed
    # …and neither is in the input, so the closure is doing the work.
    assert SUBCLASS_INFERENCE not in SCHEMA
    assert SYMMETRY_INFERENCE not in SCHEMA
    # `simple` is the identity closure: the same input yields neither.
    identity, _ = entail.materialize(_dataset(), entail.Regime.SIMPLE)
    assert SUBCLASS_INFERENCE not in identity.to_nquads()
    assert SYMMETRY_INFERENCE not in identity.to_nquads()


def test_symmetry_is_owl_only_not_rdfs() -> None:
    """`prp-symp` is an OWL rule: RDFS must NOT license the symmetric triple."""
    rdfs, _ = entail.materialize(_dataset(), entail.Regime.RDFS)
    assert SUBCLASS_INFERENCE in rdfs.to_nquads(), "rdfs9/cax-sco still fires"
    assert SYMMETRY_INFERENCE not in rdfs.to_nquads()


def test_base_triples_survive_the_closure() -> None:
    """A closure adds; it never drops. Every input triple is still present."""
    closure, _ = entail.materialize(_dataset(), entail.Regime.OWL_RL)
    closed = closure.to_nquads()
    for line in SCHEMA.splitlines():
        assert line in closed, f"input triple dropped by the closure: {line}"
    assert closure.quad_count() > len(SCHEMA.splitlines())


def test_report_names_the_rules_that_fired() -> None:
    """The report is not optional and says what the run actually did."""
    _closure, report = entail.materialize(_dataset(), entail.Regime.OWL_RL)
    assert report.startswith("purrdf-reasoning-report 1\n")
    assert "\nregime owl-rl\n" in report
    # The conclusion counts are the engine's to report, so only the fact that
    # these two rules ran is asserted here — the counts live in the Rust golden
    # vector, where a change to them is a reviewed diff rather than a surprise.
    assert "\nfired cax-sco " in report
    assert "\nfired prp-symp " in report
    assert "\ncontract-hash " in report
    assert report.endswith("overclaims false\n")


# ── The two entry points are one path ───────────────────────────────────────────


def test_materialize_nt_matches_the_dataset_path_byte_for_byte() -> None:
    """The string wrapper is the same boundary call, not a second engine path."""
    text_closure, text_report = entail.materialize_nt(SCHEMA, entail.Regime.OWL_RL)
    dataset_closure, dataset_report = entail.materialize(
        _dataset(), entail.Regime.OWL_RL
    )
    assert text_closure == dataset_closure.to_nquads()
    assert text_report == dataset_report


def test_to_nquads_round_trips() -> None:
    """A serialized closure re-parses, and re-serializes to the same bytes."""
    closure, _ = entail.materialize(_dataset(), entail.Regime.OWL_RL)
    serialized = closure.to_nquads()
    reparsed = RdfDataset(serialized, RdfFormat.N_QUADS)
    assert reparsed.to_nquads() == serialized
    assert reparsed.quad_count() == closure.quad_count()
    # The same handle type the root surface exposes, not a private twin.
    assert isinstance(reparsed, purrdf.RdfDataset)


# ── Byte determinism ────────────────────────────────────────────────────────────


@pytest.mark.parametrize("regime", MATERIALIZABLE, ids=lambda r: str(r))
def test_repeated_calls_are_byte_identical(regime: entail.Regime) -> None:
    """Twelve runs of each materializable regime produce identical bytes.

    A closure that leaked hash iteration order, a clock, a path or an address
    would diverge across these calls; a one-in-two divergence cannot pass by luck
    at this repetition count.
    """
    first_closure, first_report = entail.materialize(_dataset(), regime)
    first = first_closure.to_nquads()
    for _ in range(11):
        closure, report = entail.materialize(_dataset(), regime)
        assert closure.to_nquads() == first
        assert report == first_report
    # …and the text path is stable in the same way.
    text = entail.materialize_nt(SCHEMA, regime)
    assert entail.materialize_nt(SCHEMA, regime) == text


# ── Every regime member is accepted ─────────────────────────────────────────────


@pytest.mark.parametrize("regime", ALL_REGIMES, ids=lambda r: str(r))
def test_every_regime_member_is_accepted(regime: entail.Regime) -> None:
    """No `Regime` member is rejected as an unknown value.

    The two that cannot be forward-materialized are refused for *that* reason —
    the message names the regimes that can be — and never as a bad argument.
    """
    assert isinstance(entail.rules(regime), list)
    assert isinstance(entail.implemented_rules(regime), list)
    if regime in NOT_MATERIALIZABLE:
        with pytest.raises(ValueError, match="materializable regimes:") as refusal:
            entail.materialize(_dataset(), regime)
        assert "cannot be forward-materialized" in str(refusal.value)
        with pytest.raises(ValueError, match="materializable regimes:"):
            entail.materialize_nt(SCHEMA, regime)
    else:
        closure, report = entail.materialize(_dataset(), regime)
        assert closure.quad_count() >= 4
        assert report.startswith("purrdf-reasoning-report 1\n")


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
    # D is the datatype table; Simple and the two refused regimes state no rules.
    assert len(entail.rules(entail.Regime.D)) == 5
    for regime in [entail.Regime.SIMPLE, *NOT_MATERIALIZABLE]:
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


@pytest.mark.parametrize("regime", MATERIALIZABLE, ids=lambda r: str(r))
def test_the_gap_is_exactly_the_reports_missing_lines(regime: entail.Regime) -> None:
    """The inventory and the report cannot drift apart."""
    spec = entail.rules(regime)
    fired = entail.implemented_rules(regime)
    gap = [rule for rule in spec if rule not in fired]

    _closure, report = entail.materialize(_dataset(), regime)
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
        entail.materialize_nt("this is not n-quads\n", entail.Regime.RDFS)
    with pytest.raises(ValueError):
        RdfDataset("this is not n-quads\n", RdfFormat.N_QUADS)


def test_unknown_regime_spelling_names_the_accepted_set() -> None:
    """The error a caller three language boundaries away has to act on."""
    for call in (
        lambda: entail.materialize(_dataset(), "rdfs-plus"),
        lambda: entail.materialize_nt(SCHEMA, "rdfs-plus"),
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
