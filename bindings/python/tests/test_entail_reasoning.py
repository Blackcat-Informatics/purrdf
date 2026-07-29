# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""`purrdf.entail` — the OWL 2 Direct-Semantics reasoning services from Python.

A different LANE from `test_entail_regimes.py`. That file covers the **chase**:
`materialize`/`materialize_nt` close a document under a regime's rule table and
report `completeness exact | sound-incomplete <n>`, which is a difference of two
rule tables. This file covers the **tableau**: nine services that each answer a
Description-Logic question and each carry a certificate whose completeness is
`decided | decided-within-boundaries | budget-exhausted`.

The distinction is the point. The DL lane has no rule table to subtract, so
reusing the chase's completeness notion would report "exact" for a search that
ran out of budget. The two renderings therefore carry different banners, and a
test here asserts that neither can be parsed as the other.

What is asserted, and why:

* **Every service is reachable, and none of them can drop its certificate.**
  Each returns `(answer, certificate)`, so a caller must unpack the evidence.
* **A certificate names its own service and ends with its own honesty gate** —
  `overclaims false` for a tableau service, `one-directional true` for the
  purely syntactic profile certification, `conservative false` for a module
  extraction that decided every keep by the locality rules.
* **`unknown` is never collapsed to `false`.** A narrowed step cap drives the
  third completeness state, and the answer says `unknown` — reporting a resource
  limit as an entailment is the defect the third value exists to prevent.
* **The explanations are CHECKED, not asserted.** A justification's sufficiency
  and minimality are re-decided over the justification alone; a chase proof's
  conclusion is re-derived from the clause program and reported beside the one
  the proof claims.
* **Byte determinism.** Every service is a function of its input alone, so
  repeated calls produce identical bytes.
"""

from __future__ import annotations

import pytest

from purrdf import entail

# ── Fixtures (example.org, per the repository's vocabulary rule) ────────────────

RDF_TYPE = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"
RDFS_SUB_CLASS_OF = "http://www.w3.org/2000/01/rdf-schema#subClassOf"
OWL_DISJOINT_WITH = "http://www.w3.org/2002/07/owl#disjointWith"
OWL_COMPLEMENT_OF = "http://www.w3.org/2002/07/owl#complementOf"
OWL_THING = "http://www.w3.org/2002/07/owl#Thing"
OWL_NOTHING = "http://www.w3.org/2002/07/owl#Nothing"

# `Cat ⊑ Mammal ⊑ Animal`, the sibling `Fish ⊑ Animal`, and one cat.
TAXONOMY = (
    f"<https://example.org/Cat> <{RDFS_SUB_CLASS_OF}> <https://example.org/Mammal> .\n"
    f"<https://example.org/Mammal> <{RDFS_SUB_CLASS_OF}> <https://example.org/Animal> .\n"
    f"<https://example.org/Fish> <{RDFS_SUB_CLASS_OF}> <https://example.org/Animal> .\n"
    f"<https://example.org/tom> <{RDF_TYPE}> <https://example.org/Cat> .\n"
)

# `Cat ⊑ Animal` — entailed by the chain, asserted nowhere.
CHAIN_AXIOM = (
    f"<https://example.org/Cat> <{RDFS_SUB_CLASS_OF}> <https://example.org/Animal> .\n"
)
# The other direction, which nothing entails.
REVERSED_AXIOM = (
    f"<https://example.org/Animal> <{RDFS_SUB_CLASS_OF}> <https://example.org/Cat> .\n"
)
# `tom a Animal` — derived by the chase, not asserted.
DERIVED_TRIPLE = (
    f"<https://example.org/tom> <{RDF_TYPE}> <https://example.org/Animal> .\n"
)

# `Cat` and `Fish` are disjoint and `nemo` is in both: an ontology with NO model.
UNSATISFIABLE = (
    f"<https://example.org/Cat> <{OWL_DISJOINT_WITH}> <https://example.org/Fish> .\n"
    f"<https://example.org/nemo> <{RDF_TYPE}> <https://example.org/Cat> .\n"
    f"<https://example.org/nemo> <{RDF_TYPE}> <https://example.org/Fish> .\n"
)


def _every_service(data: str = TAXONOMY) -> list[tuple[str, tuple[str, str]]]:
    """Call every DL service over `data`, as `(service name, (answer, certificate))`.

    One list, so a service added to the surface without certificate coverage is a
    missing entry a reader can see rather than an omission nobody notices.
    """
    return [
        ("consistency", entail.consistency(data)),
        ("classify", entail.classify(data)),
        ("realize", entail.realize(data)),
        ("instances", entail.instances(data, "<https://example.org/Animal>")),
        ("entails", entail.entails(data, CHAIN_AXIOM)),
        ("profile", entail.profile(data)),
        (
            "extract-module",
            entail.extract_module(data, "<https://example.org/Cat>\n", "star"),
        ),
        ("justify", entail.justify(data, CHAIN_AXIOM)),
        (
            "explain-conclusion",
            entail.explain_conclusion(data, entail.Regime.OWL_RL, DERIVED_TRIPLE),
        ),
    ]


# The gate each certificate grammar ends with. A tableau service reports whether
# it claimed more than its evidence supports; the two services that run no tableau
# report their own honesty property instead of a fabricated completeness.
GATES = frozenset({"overclaims false", "one-directional true", "conservative false"})


# ── Every service is reachable, and carries its certificate ─────────────────────


def test_every_service_is_reachable_from_python() -> None:
    """All nine DL services exist on `purrdf.entail` and run."""
    services = _every_service()
    assert len(services) == 9
    for name, (answer, certificate) in services:
        assert isinstance(answer, str), name
        assert isinstance(certificate, str), name
        assert certificate, f"{name} returned no certificate"


def test_every_certificate_names_its_service_and_ends_with_its_gate() -> None:
    """A service that answered without saying how completely fails here.

    This is the invariant the whole surface exists for: an answer with no
    statement of how completely it was decided is the defect, not the missing
    feature.
    """
    for name, (_answer, certificate) in _every_service():
        assert f"\nservice {name}\n" in certificate, f"{name}: {certificate}"
        assert certificate.splitlines()[-1] in GATES, f"{name}: {certificate}"
        assert certificate.endswith("\n"), name


def test_the_dl_certificate_is_not_the_chase_report() -> None:
    """Two lanes, two completeness notions, two banners — never interchanged."""
    _closure, report = entail.materialize_nt(TAXONOMY, entail.Regime.OWL_RL, "")
    _answer, certificate = entail.consistency(TAXONOMY)
    assert report.startswith("purrdf-reasoning-report 1\n")
    assert certificate.startswith("purrdf-dl-certificate 1\n")
    # The chase says `exact`/`sound-incomplete`; the tableau says
    # `decided`/`decided-within-boundaries`/`budget-exhausted`. Neither
    # vocabulary appears in the other's rendering.
    assert "completeness decided" not in report
    assert "completeness exact" not in certificate


@pytest.mark.parametrize(
    "name", [name for name, _ in _every_service()], ids=lambda name: str(name)
)
def test_every_service_is_byte_stable(name: str) -> None:
    """Repeated calls produce identical bytes.

    Each call reverse-maps a freshly-interned knowledge base, so a service that
    leaked interner order, a clock or an address would diverge here.
    """
    first = dict(_every_service())[name]
    for _ in range(4):
        assert dict(_every_service())[name] == first


# ── The individual services ─────────────────────────────────────────────────────


def test_consistency_answers_both_ways() -> None:
    """The one service that answers for an ontology with no model."""
    answer, certificate = entail.consistency(TAXONOMY)
    assert answer == "consistency true\n"
    assert "\ncompleteness decided\n" in certificate

    answer, certificate = entail.consistency(UNSATISFIABLE)
    assert answer == "consistency false\n"
    assert certificate.endswith("overclaims false\n")


def test_classify_emits_the_closure_and_its_reduction() -> None:
    """`subclass` is the full relation; `direct` is its transitive reduction."""
    answer, _certificate = entail.classify(TAXONOMY)
    # Cat ⊑ Animal is entailed but not asserted…
    assert (
        "subclass <https://example.org/Cat> <https://example.org/Animal>\n" in answer
    )
    # …and it is NOT direct: Mammal sits between them.
    assert "direct <https://example.org/Cat> <https://example.org/Animal>\n" not in answer
    assert "direct <https://example.org/Cat> <https://example.org/Mammal>\n" in answer
    # `owl:Nothing` is read as ⊥ rather than as an opaque atomic class, so it is
    # unsatisfiable — the answer the semantics gives.
    assert f"unsatisfiable <{OWL_NOTHING}>\n" in answer


def test_realize_marks_the_most_specific_type() -> None:
    """Every entailed type is listed; exactly one of tom's is most specific."""
    answer, _certificate = entail.realize(TAXONOMY)
    for klass in ("Cat", "Mammal", "Animal"):
        assert f"type <https://example.org/tom> <https://example.org/{klass}>\n" in answer
    # `owl:Thing` is a type of every individual and IS listed: an entailed answer
    # omitted for being obvious is an answer set that is not one.
    assert f"type <https://example.org/tom> <{OWL_THING}>\n" in answer
    direct = [line for line in answer.splitlines() if line.startswith("direct-type ")]
    assert direct == ["direct-type <https://example.org/tom> <https://example.org/Cat>"]


def test_instances_retrieves_through_the_hierarchy() -> None:
    """Retrieval reaches through subsumption, and an unmentioned class is empty."""
    answer, _certificate = entail.instances(TAXONOMY, "<https://example.org/Animal>")
    assert answer == "instance <https://example.org/tom>\n"
    # A class no axiom constrains is a real question with a real, empty answer —
    # which is what the Direct Semantics says an unconstrained name is.
    answer, certificate = entail.instances(TAXONOMY, "<https://example.org/Unmentioned>")
    assert answer == ""
    assert certificate.endswith("overclaims false\n")


@pytest.mark.parametrize(
    ("predicate", "kind"),
    [
        (RDFS_SUB_CLASS_OF, "SubClassOf"),
        ("http://www.w3.org/2002/07/owl#equivalentClass", "EquivalentClasses"),
        (OWL_DISJOINT_WITH, "DisjointClasses"),
        (RDF_TYPE, "ClassAssertion"),
        ("http://www.w3.org/2002/07/owl#sameAs", "SameIndividual"),
        ("http://www.w3.org/2002/07/owl#differentFrom", "DifferentIndividuals"),
        ("http://www.w3.org/2000/01/rdf-schema#subPropertyOf", "SubObjectPropertyOf"),
        ("https://example.org/knows", "ObjectPropertyAssertion"),
    ],
)
def test_the_axiom_encoding_is_the_owl_2_rdf_mapping(predicate: str, kind: str) -> None:
    """An axiom crosses the boundary as ONE triple of the OWL 2 RDF mapping.

    No mini-language is invented: every axiom kind already HAS an RDF spelling,
    and it is the one the reasoner's own reverse mapping reads. The answer echoes
    which kind the predicate selected, because the predicate DISPATCHES.
    """
    statement = f"<https://example.org/s> <{predicate}> <https://example.org/o> .\n"
    answer, _certificate = entail.entails(TAXONOMY, statement)
    assert f"\naxiom {kind}\n" in answer, answer
    assert answer.count("\nterm <") in (2, 3)


def test_entails_decides_both_directions() -> None:
    """`Cat ⊑ Animal` follows from the chain; `Animal ⊑ Cat` does not."""
    answer, _certificate = entail.entails(TAXONOMY, CHAIN_AXIOM)
    assert answer.startswith("entails true\n")
    answer, _certificate = entail.entails(TAXONOMY, REVERSED_AXIOM)
    assert answer.startswith("entails false\n")


def test_an_exhausted_budget_is_unknown_and_never_false() -> None:
    """A resource limit is reported as a resource limit, not as an entailment.

    `step_cap` can only NARROW the knowledge base's own ceiling, so it cannot be
    used to make a hard instance answerable — only to make this branch, the third
    completeness state, reachable from a test.
    """
    answer, certificate = entail.entails(TAXONOMY, CHAIN_AXIOM, 1)
    assert answer.splitlines()[0] == "entails unknown"
    assert "\ncompleteness budget-exhausted\n" in certificate
    assert "\nbudget 1\n" in certificate
    # The gate still holds: an exhausted run claims nothing it cannot support.
    assert certificate.endswith("overclaims false\n")
    # …and 0 means the knowledge base's own cap, not a cap of zero steps.
    answer, certificate = entail.entails(TAXONOMY, CHAIN_AXIOM, 0)
    assert answer.startswith("entails true\n")
    assert "\ncompleteness decided\n" in certificate


def test_an_unsatisfiable_ontology_is_refused_rather_than_answered_vacuously() -> None:
    """Every class subsumes every other with no model, so no answer is given."""
    for call in (
        lambda: entail.classify(UNSATISFIABLE),
        lambda: entail.realize(UNSATISFIABLE),
        lambda: entail.instances(UNSATISFIABLE, "<https://example.org/Cat>"),
    ):
        with pytest.raises(ValueError, match="no model"):
            call()


def test_profile_certifies_most_restrictive_first() -> None:
    """A bare sub-class taxonomy is in every OWL 2 profile."""
    answer, certificate = entail.profile(TAXONOMY)
    assert answer.splitlines() == [
        "certified EL",
        "certified QL",
        "certified RL",
        "certified DL",
        "certified Full",
    ]
    # The most restrictive certified profile, which is what a caller asking
    # "what is this ontology?" wants.
    assert answer.splitlines()[0].removeprefix("certified ") == "EL"
    for profile in ("el", "ql", "rl", "dl", "full"):
        assert f"\ncertifies-{profile} true\n" in certificate
    # A certification PROVES membership; a violation does NOT prove exclusion.
    # Stated on the certificate rather than only in prose a consumer may not read.
    assert certificate.endswith("one-directional true\n")


def test_a_profile_violation_names_its_term_and_reason() -> None:
    """`owl:complementOf` is outside the EL grammar, and the certificate says so."""
    complement = (
        f"<https://example.org/NotCat> <{OWL_COMPLEMENT_OF}> <https://example.org/Cat> .\n"
    )
    answer, certificate = entail.profile(complement)
    violations = [
        line.removeprefix("violation ")
        for line in certificate.splitlines()
        if line.startswith("violation ")
    ]
    assert violations, certificate
    for violation in violations:
        profile, term, subject, reason = violation.split(" ", 3)
        assert profile in {"EL", "QL", "RL", "DL", "Full"}
        assert term.startswith("<")
        assert subject
        assert len(reason) > 4
    # Full is every RDF graph under the RDF-Based Semantics, so it never fails.
    assert answer.endswith("certified Full\n")


def test_extract_module_is_smaller_than_the_ontology() -> None:
    """The ⊥-module for {Cat} follows the chain up and leaves the sibling behind."""
    answer, certificate = entail.extract_module(
        TAXONOMY, "<https://example.org/Cat>\n", "bot"
    )
    assert "<https://example.org/Cat>" in answer
    assert "<https://example.org/Fish>" not in answer, answer
    assert "\nmethod BOT\n" in certificate
    # Every keep was decided by the locality rules, which is the strongest thing
    # an extraction can say; `conservative true` would mean a sound SUPERSET.
    assert certificate.endswith("conservative false\n")


@pytest.mark.parametrize("method", ["bot", "top", "star"])
def test_every_module_method_is_reachable(method: str) -> None:
    """All three locality notions cross the boundary and name themselves."""
    _answer, certificate = entail.extract_module(
        TAXONOMY, "<https://example.org/Cat>\n", method
    )
    assert f"\nmethod {method.upper()}\n" in certificate


def test_an_unknown_module_method_names_the_accepted_set() -> None:
    """The error a caller three language boundaries away has to act on."""
    with pytest.raises(ValueError) as raised:
        entail.extract_module(TAXONOMY, "", "nested")
    message = str(raised.value)
    assert "nested" in message
    for method in ("bot", "top", "star"):
        assert method in message


def test_justify_re_decides_both_halves_of_its_claim() -> None:
    """A justification is sufficient AND minimal, and both are re-decided here."""
    answer, certificate = entail.justify(TAXONOMY, CHAIN_AXIOM)
    # The chain, and NOT the sibling: two axioms of the four.
    assert len(answer.splitlines()) == 2, answer
    assert "<https://example.org/Fish>" not in answer
    assert "\nsufficient true\n" in certificate
    assert "\nminimal true\n" in certificate
    assert certificate.endswith("overclaims false\n")
    # The identity is a CONTENT digest, never an IRI: PurRDF mints no vocabulary.
    digest = next(
        line.removeprefix("digest ")
        for line in certificate.splitlines()
        if line.startswith("digest ")
    )
    assert len(digest) == 64
    assert digest == digest.lower()
    assert all(character in "0123456789abcdef" for character in digest)


def test_an_unentailed_axiom_has_no_justification() -> None:
    """A refusal, not an empty set — which reads as "nothing is needed"."""
    with pytest.raises(ValueError, match="does not entail"):
        entail.justify(TAXONOMY, REVERSED_AXIOM)


def test_explain_conclusion_re_derives_rather_than_re_reads() -> None:
    """The certificate reports what the CHECKER computed, not what the proof says."""
    answer, certificate = entail.explain_conclusion(
        TAXONOMY, entail.Regime.OWL_RL, DERIVED_TRIPLE
    )
    assert answer.startswith("asserted false\n")
    assert "\nrule cax-sco\n" in answer, answer
    assert "\nchecked true\n" in certificate
    assert certificate.endswith("overclaims false\n")

    def field(key: str) -> str:
        return next(
            line.removeprefix(key)
            for line in certificate.splitlines()
            if line.startswith(key)
        )

    for part in ("subject", "predicate", "object"):
        assert field(f"conclusion-{part} ") == field(f"derived-{part} ")


def test_an_asserted_conclusion_is_explained_by_being_asserted() -> None:
    """A given triple has a real, checkable explanation: that it is given."""
    asserted = f"<https://example.org/tom> <{RDF_TYPE}> <https://example.org/Cat> .\n"
    answer, certificate = entail.explain_conclusion(
        TAXONOMY, entail.Regime.OWL_RL, asserted
    )
    assert answer.startswith("asserted true\n")
    # Checked against the SEEDED store, so a derived fact cannot pass as a given.
    assert "\nchecked true\n" in certificate


@pytest.mark.parametrize("regime", [entail.Regime.RDF, entail.Regime.RDFS])
def test_an_existential_head_has_no_checkable_proof(regime: entail.Regime) -> None:
    """Four RDF/RDFS rules conclude about a FRESH blank node.

    An existentially quantified head has no Datalog semantics, so there is no head
    for the checker to instantiate: a "proof" of such a step could only be
    believed, which is precisely what a proof term exists not to require. The
    refusal names the reason rather than returning an unverifiable derivation.
    """
    with pytest.raises(ValueError, match="existential"):
        entail.explain_conclusion(TAXONOMY, regime, DERIVED_TRIPLE)


def test_an_underivable_conclusion_is_a_hard_error() -> None:
    """An empty explanation would read as "there is nothing to explain"."""
    absent = (
        "<https://example.org/nobody> <https://example.org/nothing> "
        "<https://example.org/nowhere> .\n"
    )
    with pytest.raises(ValueError, match="no derivation"):
        entail.explain_conclusion(TAXONOMY, entail.Regime.OWL_RL, absent)


# ── Refusals ────────────────────────────────────────────────────────────────────


def test_a_malformed_document_is_an_error_not_an_empty_answer() -> None:
    """Every service refuses a document it cannot parse."""
    for call in (
        lambda: entail.consistency("this is not n-quads\n"),
        lambda: entail.classify("this is not n-quads\n"),
        lambda: entail.realize("this is not n-quads\n"),
        lambda: entail.profile("this is not n-quads\n"),
        lambda: entail.extract_module("this is not n-quads\n", "", "bot"),
    ):
        with pytest.raises(ValueError):
            call()


def test_a_malformed_term_or_axiom_is_refused() -> None:
    """A class is ONE N-Triples term; an axiom is ONE ungraphed triple."""
    with pytest.raises(ValueError, match="N-Triples term"):
        entail.instances(TAXONOMY, "not a term")
    with pytest.raises(ValueError, match="N-Triples term"):
        entail.instances(TAXONOMY, "<https://example.org/A> <https://example.org/B>")
    # A literal is not a name, so it is not a class, an individual or a property.
    with pytest.raises(ValueError):
        entail.instances(TAXONOMY, '"Cat"')
    graph_scoped = (
        f"<https://example.org/Cat> <{RDFS_SUB_CLASS_OF}> <https://example.org/Animal> "
        "<https://example.org/g> .\n"
    )
    with pytest.raises(ValueError, match="names a graph"):
        entail.entails(TAXONOMY, graph_scoped)
