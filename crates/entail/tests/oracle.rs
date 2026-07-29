// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The engine-swap oracle: a fixture corpus, committed goldens, and a per-rule registry.
//!
//! # Why this file exists
//!
//! `purrdf-entail`'s closure is produced by a hand-written forward chase
//! (`crates/entail/src/rdfs.rs`). A later change replaces that chase with the DL-clause
//! program `calculus_program(regime)` already declares. That is an engine swap on a live
//! calculus, and the only way to make it reviewable is to know — byte for byte, before the
//! swap — what the current engine answers for a corpus that reaches every rule it fires.
//!
//! The oracle is **committed golden files**, not a retained copy of the old chase. A second
//! implementation kept alive "for comparison" is a second implementation: it has to be
//! maintained, it can itself drift, and when the two disagree there is no third party to
//! say which is right. A golden file is inert. It cannot drift, it diffs in review, and a
//! deliberate change to it is a line in a commit rather than an argument.
//!
//! # What is captured, and what deliberately is not
//!
//! Each golden holds, for one fixture and for each of the four regimes `materialize` can
//! run: the closure as **canonical N-Quads** (`purrdf_core::canonicalize` — the repository's
//! RDFC-1.0 canonicalizer, not a serializer written here) and the [`ReasoningReport`]
//! rendered field by field in the report's own documented order.
//!
//! Canonical N-Quads is a statement about the closure's *quad set*, sorted bytewise. The
//! chase's *emission order* is deliberately not pinned here: it is an internal property of
//! one evaluation strategy that a different engine may legitimately choose differently, and
//! it already has its own guard (`rdfs_emission_order_is_deterministic` in the crate's unit
//! tests). What an engine swap must preserve is the closure, and that is what these
//! goldens are.
//!
//! # Fixture inputs are Rust, not data files
//!
//! There is no N-Quads *parser* in this crate's dependency set — parsers live in
//! `purrdf-rdf`, which `purrdf-entail` does not depend on and, under this branch's
//! constraints, may not — so a fixture is declared as a small table of [`Quad`] values and
//! built with `RdfDatasetBuilder`. The input's canonical N-Quads form is nevertheless
//! written into the golden, so the fixture a golden was captured from is itself pinned and
//! readable; editing a fixture table without regenerating fails the gate.
//!
//! # The shipped wasm artifact
//!
//! Nothing here reaches it. This is an integration test (`tests/`), which is never part of
//! the library `cdylib`/`rlib`, and the goldens are read from disk with [`std::fs`] at test
//! time rather than embedded with `include_str!`, so their bytes are not in any compiled
//! artifact at all — shipped or otherwise.
//!
//! # Regenerating
//!
//! ```text
//! cargo test -p purrdf-entail --test oracle -- --ignored --exact regenerate_goldens
//! ```
//!
//! Deliberately `#[ignore]`d so a normal `cargo test` can only ever *compare*.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::PathBuf;
use std::str::FromStr as _;
use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, TermId, canonicalize};
use purrdf_entail::{ReasoningReport, Regime, RuleId, implemented, materialize, rules};

// ── Vocabulary ──────────────────────────────────────────────────────────────────
//
// Specification IRIs, spelled out. The crate's own `vocab` module is `pub(crate)`, and
// that is the right shape: an oracle that imported the engine's constants would agree with
// the engine by construction. Fixture-local terms are all `example.org` — PurRDF mints no
// vocabulary.

/// `rdf:type`.
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
/// `rdf:Property`.
const RDF_PROPERTY: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Property";
/// `rdfs:subClassOf`.
const RDFS_SUBCLASSOF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
/// `rdfs:subPropertyOf`.
const RDFS_SUBPROPERTYOF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
/// `rdfs:domain`.
const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
/// `rdfs:range`.
const RDFS_RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
/// `rdfs:Class`.
const RDFS_CLASS: &str = "http://www.w3.org/2000/01/rdf-schema#Class";
/// `rdfs:Resource`.
const RDFS_RESOURCE: &str = "http://www.w3.org/2000/01/rdf-schema#Resource";
/// `rdfs:Literal`.
const RDFS_LITERAL: &str = "http://www.w3.org/2000/01/rdf-schema#Literal";
/// `rdfs:Datatype`.
const RDFS_DATATYPE: &str = "http://www.w3.org/2000/01/rdf-schema#Datatype";
/// `rdfs:ContainerMembershipProperty`.
const RDFS_CONTAINERMEMBERSHIPPROPERTY: &str =
    "http://www.w3.org/2000/01/rdf-schema#ContainerMembershipProperty";
/// `rdfs:member`.
const RDFS_MEMBER: &str = "http://www.w3.org/2000/01/rdf-schema#member";
/// `owl:SymmetricProperty`.
const OWL_SYMMETRIC: &str = "http://www.w3.org/2002/07/owl#SymmetricProperty";
/// `owl:TransitiveProperty`.
const OWL_TRANSITIVE: &str = "http://www.w3.org/2002/07/owl#TransitiveProperty";
/// `owl:inverseOf`.
const OWL_INVERSEOF: &str = "http://www.w3.org/2002/07/owl#inverseOf";
/// `owl:equivalentClass`.
const OWL_EQUIVALENTCLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
/// `owl:equivalentProperty`.
const OWL_EQUIVALENTPROPERTY: &str = "http://www.w3.org/2002/07/owl#equivalentProperty";
/// `xsd:string`.
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// Fixture class `example.org/A`.
const EX_A: &str = "http://example.org/A";
/// Fixture class `example.org/B`.
const EX_B: &str = "http://example.org/B";
/// Fixture class `example.org/C`.
const EX_C: &str = "http://example.org/C";
/// Fixture class `example.org/D`.
const EX_D: &str = "http://example.org/D";
/// Fixture class `example.org/E`.
const EX_E: &str = "http://example.org/E";
/// Fixture class `example.org/F`.
const EX_F: &str = "http://example.org/F";
/// A fixture class that is deliberately NOT typed `rdfs:Class`.
const EX_NOT_A_CLASS: &str = "http://example.org/NotAClass";
/// A fixture datatype IRI. It is `example.org`, so it is NOT one of the datatypes
/// RDF 1.2 Semantics §8 makes every interpretation recognize — which is what makes it a
/// usable control for `rdfs1`.
const EX_DT: &str = "http://example.org/dt";
/// Fixture property `example.org/p`.
const EX_P: &str = "http://example.org/p";
/// Fixture property `example.org/q`.
const EX_Q: &str = "http://example.org/q";
/// Fixture property `example.org/r`.
const EX_R: &str = "http://example.org/r";
/// Fixture property `example.org/says`.
const EX_SAYS: &str = "http://example.org/says";
/// Fixture property `example.org/mentions`.
const EX_MENTIONS: &str = "http://example.org/mentions";
/// Fixture individual `example.org/x`.
const EX_X: &str = "http://example.org/x";
/// Fixture individual `example.org/y`.
const EX_Y: &str = "http://example.org/y";
/// Fixture individual `example.org/z`.
const EX_Z: &str = "http://example.org/z";
/// Fixture individual `example.org/u`.
const EX_U: &str = "http://example.org/u";
/// Fixture individual `example.org/v`.
const EX_V: &str = "http://example.org/v";
/// Fixture named graph `example.org/g`.
const EX_G: &str = "http://example.org/g";

// ── The fixture model ───────────────────────────────────────────────────────────

/// One object-position term a fixture quad can hold.
///
/// Subjects are always IRIs and predicates are always IRIs, so only the object slot needs
/// a sum type. That is exactly the RDF 1.2 shape the chase can *consume*; the interesting
/// cases are the ones it cannot *conclude into*, which is what the object slot supplies.
#[derive(Debug, Clone, Copy)]
enum Term {
    /// An IRI.
    Iri(&'static str),
    /// A datatyped literal.
    Literal {
        /// The lexical form.
        lexical: &'static str,
        /// The datatype IRI.
        datatype: &'static str,
    },
    /// An RDF 1.2 triple term over three IRIs.
    Quoted(&'static str, &'static str, &'static str),
}

/// One quad of a fixture: an IRI subject, an IRI predicate, an object, and a graph.
#[derive(Debug, Clone, Copy)]
struct Quad {
    /// The subject IRI.
    s: &'static str,
    /// The predicate IRI.
    p: &'static str,
    /// The object term.
    o: Term,
    /// The graph IRI; `None` is the default graph.
    g: Option<&'static str>,
}

/// A default-graph triple over three IRIs — the common case.
const fn t(s: &'static str, p: &'static str, o: &'static str) -> Quad {
    Quad {
        s,
        p,
        o: Term::Iri(o),
        g: None,
    }
}

/// A default-graph triple whose object is a datatyped literal.
const fn t_lit(s: &'static str, p: &'static str, lexical: &'static str) -> Quad {
    Quad {
        s,
        p,
        o: Term::Literal {
            lexical,
            datatype: XSD_STRING,
        },
        g: None,
    }
}

/// A default-graph triple whose object is an RDF 1.2 triple term.
const fn t_quoted(
    s: &'static str,
    p: &'static str,
    qs: &'static str,
    qp: &'static str,
    qo: &'static str,
) -> Quad {
    Quad {
        s,
        p,
        o: Term::Quoted(qs, qp, qo),
        g: None,
    }
}

/// A triple over three IRIs, placed in the named graph `g`.
const fn t_in(s: &'static str, p: &'static str, o: &'static str, g: &'static str) -> Quad {
    Quad {
        s,
        p,
        o: Term::Iri(o),
        g: Some(g),
    }
}

/// One input dataset of the corpus, with the reason it exists.
#[derive(Debug, Clone, Copy)]
struct Fixture {
    /// The fixture's name; also the golden file's stem.
    name: &'static str,
    /// Why this fixture exists, one line per element. Rendered into the golden header.
    doc: &'static [&'static str],
    /// The specification rule ids this fixture is meant to reach, by canonical spelling.
    ///
    /// Checked to parse as [`RuleId`]s; documentation for the reader, not an assertion
    /// about the closure (the registry below makes those, per rule).
    exercises: &'static [&'static str],
    /// What moved in THIS golden at the engine swap, and why the new answer is licensed.
    ///
    /// Rendered into the golden under [`ENGINE_SWAP`], which states the three causes once;
    /// these lines name the actual triples and tallies each cause moved here. A golden
    /// regenerated without an entry is the defect this corpus exists to catch, so the
    /// field is not `Option` and an empty slice is a claim — "this golden did not move" —
    /// rather than an omission.
    changed: &'static [&'static str],
    /// The input quads.
    quads: &'static [Quad],
}

/// The three causes of every byte that moved when `materialize` stopped running a
/// hand-written chase and started evaluating [`purrdf_entail::calculus_program`].
///
/// Written into every golden's header, once, above the fixture's own [`Fixture::changed`]
/// accounting — so a reader of one golden file sees both the general reason and the
/// specific triples without having to hold the other twenty-eight in their head.
///
/// A fourth thing that could have moved did NOT, and its absence is asserted rather than
/// assumed: `divergence_literal_subject` still reports the `generalized-rdf` boundary. A
/// Datalog engine derives a literal-subject conclusion in its own term space and meets the
/// RDF 1.2 IR only when the answer is materialized, so the failure mode was the boundary
/// quietly disappearing while the triples still looked right. See
/// `a_would_be_literal_subject_is_abandoned_and_reported`.
const ENGINE_SWAP: &[&str] = &[
    "EVERY GOLDEN IN THIS CORPUS MOVED AT THE ENGINE SWAP — `materialize` stopped",
    "running a hand-written chase and started evaluating the DL-clause program",
    "`calculus_program(regime)` already declared. Three causes account for every byte,",
    "and the fixture's own accounting below names which triples each one moved here.",
    "",
    "  1. THE UNLICENSED REFLEXIVES ARE GONE — a SPEC-CONFORMANCE FIX. The chase",
    "     emitted `c rdfs:subClassOf c` for every subClassOf ENDPOINT and",
    "     `p rdfs:subPropertyOf p` for every PREDICATE. rdfs10 requires `?c rdf:type",
    "     rdfs:Class` and rdfs6 requires `?p rdf:type rdf:Property`, and the declared",
    "     clauses say so, so those conclusions are drawn only where the specification",
    "     licenses them. Both rules still fire — `property_typed` and `class_typed` are",
    "     the fixtures that prove they were narrowed rather than switched off.",
    "",
    "  2. THE RDF LANE IS NOW A FIXPOINT — a BUG FIX. `close_rdf` walked the INPUT",
    "     quads once and typed each predicate it saw, so it never applied rdfD2 to its",
    "     own conclusions. `rdf:type` is a predicate of every one of them, so",
    "     `rdf:type rdf:type rdf:Property` is entailed and was missing. It appears now",
    "     in every RDF closure whose input did not already use `rdf:type` as a",
    "     predicate; where the input did, the RDF closure is unchanged.",
    "",
    "  3. THE BUDGET IS THE EVALUATOR'S OWN MEASUREMENT. `join-steps`,",
    "     `stored-facts` and `term-arena-bytes` are now `purrdf-datalog`'s",
    "     `BudgetReport` rather than a tally the chase kept beside it, so all three",
    "     move in every regime. They are the same three coordinates under the same",
    "     three definitions, counted by the engine that did the work: `stored-facts`",
    "     is the whole saturated store rather than one lane's private index, and",
    "     `term-arena-bytes` counts the terms that actually entered the store rather",
    "     than a vocabulary table interned whether or not the data mentioned it.",
];

/// The four causes of every byte that moved when the `RDFS` lane started asserting the
/// axiomatic triples and firing five more rules.
///
/// Written into every golden's header beneath [`ENGINE_SWAP`], for the same reason that
/// one is: the general cause once, the fixture's own triples in its own
/// [`Fixture::changed`].
///
/// The single strongest fact about this diff is stated first, because it makes the other
/// thirty-two goldens checkable at a glance: EVERY changed golden changed in the `RDFS`
/// section and in no other. `Simple`, `RDF` and `OWL-RL` are byte-identical across all
/// twenty-nine goldens that already existed.
const AXIOMATIC_PATH: &[&str] = &[
    "AND EVERY GOLDEN MOVED AGAIN, IN THE RDFS SECTION AND NOWHERE ELSE — the RDFS lane",
    "now asserts the axiomatic triples and fires five more rules. `Simple`, `RDF` and",
    "`OWL-RL` are byte-identical to the previous golden in all twenty-nine files that",
    "already existed. Four causes account for the new bytes.",
    "",
    "  A. THE FINITE AXIOMATIC TRIPLES ARE ASSERTED, AS PREMISES — a SPEC-CONFORMANCE",
    "     FIX, and the one that motivated this change. RDF 1.2 Semantics §8 (table `RDF",
    "     axioms`, 9 triples) and §9 (table `RDFS axiomatic triples`, 40) are seeded into",
    "     the fact store beside the input, because `S RDFS entails E` is defined over the",
    "     interpretations satisfying S AND those axioms. They are premises, so they are",
    "     NOT in the closure; everything they license is. Above all `rdfs:subClassOf` has",
    "     an axiomatic domain and range of `rdfs:Class`, so a subClassOf edge now types",
    "     BOTH its endpoints and `c rdfs:subClassOf c` comes back — from the premise the",
    "     specification names, not from the endpoint shortcut the engine swap removed.",
    "     Three W3C SPARQL entailment cases (rdfs05, rdfs11, paper-sparqldl-Q1-rdfs) were",
    "     failing for exactly the missing half of that path and now pass.",
    "",
    "  B. FIVE MORE RULES FIRE IN THE RDFS LANE: `rdfD2` (RDFS entailment subsumes RDF",
    "     entailment, and `rules(Rdfs)` always said so), `rdfs1`, `rdfs4`, `rdfs12` and",
    "     `rdfs13`. `implemented(Rdfs)` is 14 of 18 where it was 9, and the four still",
    "     missing — `rdfD1`, `rdfD1a`, `rdfs14`, `rdfs14a` — are missing for one reason:",
    "     each concludes about a FRESH blank node, which is an existential head this",
    "     crate's Datalog evaluator refuses.",
    "",
    "  C. EVERY RDFS CLOSURE CARRIES THE SAME 113-LINE VOCABULARY SATURATION. `rdfs4`",
    "     types every term of every triple an `rdfs:Resource`, and the axioms are triples,",
    "     so the axioms' own vocabulary saturates in every run. That block is INPUT-",
    "     INDEPENDENT: it is exactly `empty.golden`'s RDFS closure, it is a subset of every",
    "     other fixture's, and `the_rdfs_closure_of_every_fixture_contains_the_empty_one`",
    "     asserts so. Each golden below therefore accounts only for the lines BEYOND it.",
    "",
    "  D. THE BUDGET ROSE, AND IS STILL NEGLIGIBLE. An RDFS run now costs about 2,000",
    "     join steps and 165-210 stored facts against ceilings of 1,048,576 and 131,072 —",
    "     0.25% and 0.16% at this corpus's worst case (`subclass_chain`, 2,577 steps).",
];

// ── The corpus ──────────────────────────────────────────────────────────────────
//
// Every fixture is minimal on purpose: the smallest input that reaches the rule, so a
// golden diff names one thing. Near-miss fixtures differ from their positive in exactly
// one term — the one the rule's premise binds — so "the rule did not fire" is attributable.

/// Every fixture, in the order the goldens are written and compared.
const CORPUS: &[Fixture] = &[
    Fixture {
        name: "empty",
        doc: &[
            "The empty dataset. Nothing fires, but the two INHERENT boundaries (the",
            "infinite axiomatic-triple schemas and the datatype value spaces) still hold",
            "for the lanes that meet them, so this pins that a boundary list is a property",
            "of the lane and not of the data.",
        ],
        exercises: &[],
        changed: &[
            "The CLOSURE does not move: nothing fires on nothing, in any of the four regimes.",
            "Cause 3 only — RDFS and OWL-RL report term-arena-bytes=0 where they reported 594.",
            "The chase pre-interned its thirteen vocabulary constants before looking at the",
            "data; a store interns a term when a term enters it, and none does here.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 0 -> 113 lines.",
            "THE 113 LINES ARE THE WHOLE OF CAUSE C, AND THIS GOLDEN IS WHERE THEY ARE PINNED.",
            "Nothing here is about data — there is none. The closure is what the axiomatic",
            "triples entail about the RDF and RDFS vocabulary itself: rdfs4 types all 32",
            "vocabulary terms rdfs:Resource, rdfs2/rdfs3 type the axioms' subjects and objects",
            "(13 + 8), rdfD2 types the three axiom predicates rdf:Property, rdfs1 types the",
            "three mandatory datatypes rdfs:Datatype, rdfs13 makes each a sub-class of",
            "rdfs:Literal, and rdfs6/rdfs8/rdfs10 close the reflexives. It is a fixpoint of the",
            "vocabulary and cannot depend on the input, which is what makes it a subset of",
            "every other RDFS closure in the corpus.",
            "The two INHERENT boundaries still hold and the report still says sound-incomplete,",
            "so this fixture still pins what it was written to pin.",
        ],
        quads: &[],
    },
    Fixture {
        name: "plain_triple",
        doc: &[
            "One triple with no schema at all. Under RDF this is the whole of rdfD2: the",
            "predicate is typed rdf:Property. Under Simple it is the identity closure.",
        ],
        exercises: &["rdfD2"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 1 -> 2).",
            "Cause 1 — RDFS and OWL-RL lose `p rdfs:subPropertyOf p` and",
            "`rdfs:subPropertyOf rdfs:subPropertyOf rdfs:subPropertyOf` (rdfs6 2 -> 0). Nothing",
            "in this input is typed rdf:Property, so rdfs6 has no premise: the RDFS closure is",
            "now the input alone, which is what one untyped triple entails under RDFS.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 1 -> 119 lines: the 113-line input-independent block cause C",
            "describes, plus 6 about this fixture's own terms —",
            "",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:x ex:p ex:y",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y rdf:type rdfs:Resource",
            "",
            "`x p y` puts three terms in the graph, so rdfs4 types all three, rdfD2 types p an",
            "rdf:Property, and rdfs6 draws the reflexive sub-property from THAT premise — the",
            "triple the engine swap removed as unlicensed is back, now with `p rdf:type",
            "rdf:Property` present in the same closure to license it.",
        ],
        quads: &[t(EX_X, EX_P, EX_Y)],
    },
    Fixture {
        name: "named_graph",
        doc: &[
            "AWKWARD CASE — a quad outside the default graph. The chase reads and writes",
            "the default graph only, so `x p y` in graph g supplies no premise and receives",
            "no conclusion; it is carried through unchanged and a named-graph boundary is",
            "reported. This is also rdfD2's near-miss: p is a predicate, but not a",
            "DEFAULT-GRAPH predicate, so it is not typed rdf:Property.",
        ],
        exercises: &["rdfD2"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 1 -> 2).",
            "Cause 1 — RDFS and OWL-RL lose four triples (rdfs6 2 -> 0, rdfs10 2 -> 0):",
            "`A rdfs:subClassOf A` and `B rdfs:subClassOf B` fired on the ENDPOINTS of the",
            "input's subClassOf edge rather than on rdfs:Class instances, and",
            "`rdfs:subClassOf rdfs:subPropertyOf rdfs:subClassOf` /",
            "`rdfs:subPropertyOf rdfs:subPropertyOf rdfs:subPropertyOf` fired on predicates",
            "rather than on rdf:Property instances.",
            "What this fixture is FOR is untouched: the named-graph quad is still carried",
            "through unchanged, still supplies no premise, and the boundary is still reported.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 2 -> 123 lines: the 113-line input-independent block cause C",
            "describes, plus 10 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Class",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A rdfs:subClassOf ex:A",
            "  ex:A rdfs:subClassOf ex:B",
            "  ex:A rdfs:subClassOf rdfs:Resource",
            "  ex:B rdf:type rdfs:Class",
            "  ex:B rdf:type rdfs:Resource",
            "  ex:B rdfs:subClassOf ex:B",
            "  ex:B rdfs:subClassOf rdfs:Resource",
            "  ex:x ex:p ex:y ex:g",
            "",
            "What this fixture is FOR is untouched: the named-graph quad is still carried",
            "through, still supplies no premise, and the boundary is still reported — which",
            "is now a sharper claim, because rdfs4 would type x, p and y rdfs:Resource if",
            "the chase read that graph, and it does not. The default-graph subClassOf edge",
            "gets the full axiomatic treatment: both endpoints typed rdfs:Class and both",
            "reflexives drawn.",
        ],
        quads: &[t_in(EX_X, EX_P, EX_Y, EX_G), t(EX_A, RDFS_SUBCLASSOF, EX_B)],
    },
    Fixture {
        name: "domain",
        doc: &[
            "rdfs2 / prp-dom: a domain declaration types the subject of every triple with",
            "that predicate.",
        ],
        exercises: &["rdfs2", "prp-dom"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 2 -> 3).",
            "Cause 1 — RDFS and OWL-RL lose the four reflexive subPropertyOf triples on the",
            "predicates p, rdf:type, rdfs:domain and rdfs:subPropertyOf (rdfs6 4 -> 0).",
            "rdfs2 / prp-dom is untouched: `x rdf:type A` is still concluded and still credited.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 3 -> 125 lines: the 113-line input-independent block cause C",
            "describes, plus 12 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Class",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A rdfs:subClassOf ex:A",
            "  ex:A rdfs:subClassOf rdfs:Resource",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:domain ex:A",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:x ex:p ex:y",
            "  ex:x rdf:type ex:A",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y rdf:type rdfs:Resource",
            "",
            "rdfs2 / prp-dom is untouched — `x rdf:type A` is still concluded and still credited.",
            "Around it, A is now an rdfs:Class (rdfs3 over the axiomatic range of rdfs:domain)",
            "and p an rdf:Property (rdfs2 over its axiomatic domain), so both reflexives follow.",
        ],
        quads: &[t(EX_P, RDFS_DOMAIN, EX_A), t(EX_X, EX_P, EX_Y)],
    },
    Fixture {
        name: "domain_near_miss",
        doc: &[
            "NEAR MISS for rdfs2 / prp-dom: the domain is declared on a DIFFERENT property",
            "(q, not p), so the data triple's subject is not typed.",
        ],
        exercises: &["rdfs2", "prp-dom"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 2 -> 3).",
            "Cause 1 — RDFS and OWL-RL lose the three reflexive subPropertyOf triples on the",
            "predicates p, rdfs:domain and rdfs:subPropertyOf (rdfs6 3 -> 0). The near miss",
            "still holds: `x rdf:type A` is absent, because the domain is declared on q.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 2 -> 127 lines: the 113-line input-independent block cause C",
            "describes, plus 14 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Class",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A rdfs:subClassOf ex:A",
            "  ex:A rdfs:subClassOf rdfs:Resource",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:q rdf:type rdf:Property",
            "  ex:q rdf:type rdfs:Resource",
            "  ex:q rdfs:domain ex:A",
            "  ex:q rdfs:subPropertyOf ex:q",
            "  ex:x ex:p ex:y",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y rdf:type rdfs:Resource",
            "",
            "The near miss still holds: `x rdf:type A` is absent, because the domain is declared",
            "on q. Note it is now a STRONGER near miss — q is typed rdf:Property and A an",
            "rdfs:Class exactly as in the positive, so the only difference left between the two",
            "closures is the one triple the rule is about.",
        ],
        quads: &[t(EX_Q, RDFS_DOMAIN, EX_A), t(EX_X, EX_P, EX_Y)],
    },
    Fixture {
        name: "range",
        doc: &[
            "rdfs3 / prp-rng: a range declaration types the object of every triple with",
            "that predicate.",
        ],
        exercises: &["rdfs3", "prp-rng"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 2 -> 3).",
            "Cause 1 — RDFS and OWL-RL lose the four reflexive subPropertyOf triples on the",
            "predicates p, rdf:type, rdfs:range and rdfs:subPropertyOf (rdfs6 4 -> 0).",
            "rdfs3 / prp-rng is untouched: `y rdf:type B` is still concluded and still credited.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 3 -> 125 lines: the 113-line input-independent block cause C",
            "describes, plus 12 about this fixture's own terms —",
            "",
            "  ex:B rdf:type rdfs:Class",
            "  ex:B rdf:type rdfs:Resource",
            "  ex:B rdfs:subClassOf ex:B",
            "  ex:B rdfs:subClassOf rdfs:Resource",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:range ex:B",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:x ex:p ex:y",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y rdf:type ex:B",
            "  ex:y rdf:type rdfs:Resource",
            "",
            "rdfs3 / prp-rng is untouched — `y rdf:type B` is still concluded and still credited.",
        ],
        quads: &[t(EX_P, RDFS_RANGE, EX_B), t(EX_X, EX_P, EX_Y)],
    },
    Fixture {
        name: "range_near_miss",
        doc: &[
            "NEAR MISS for rdfs3 / prp-rng: the range is declared on a DIFFERENT property",
            "(q, not p), so the data triple's object is not typed.",
        ],
        exercises: &["rdfs3", "prp-rng"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 2 -> 3).",
            "Cause 1 — RDFS and OWL-RL lose the three reflexive subPropertyOf triples on the",
            "predicates p, rdfs:range and rdfs:subPropertyOf (rdfs6 3 -> 0). The near miss",
            "still holds: `y rdf:type B` is absent, because the range is declared on q.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 2 -> 127 lines: the 113-line input-independent block cause C",
            "describes, plus 14 about this fixture's own terms —",
            "",
            "  ex:B rdf:type rdfs:Class",
            "  ex:B rdf:type rdfs:Resource",
            "  ex:B rdfs:subClassOf ex:B",
            "  ex:B rdfs:subClassOf rdfs:Resource",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:q rdf:type rdf:Property",
            "  ex:q rdf:type rdfs:Resource",
            "  ex:q rdfs:range ex:B",
            "  ex:q rdfs:subPropertyOf ex:q",
            "  ex:x ex:p ex:y",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y rdf:type rdfs:Resource",
            "",
            "The near miss still holds: `y rdf:type B` is absent, because the range is on q.",
        ],
        quads: &[t(EX_Q, RDFS_RANGE, EX_B), t(EX_X, EX_P, EX_Y)],
    },
    Fixture {
        name: "subproperty_chain",
        doc: &["rdfs5 / scm-spo: rdfs:subPropertyOf is transitive."],
        exercises: &["rdfs5", "scm-spo"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 1 -> 2).",
            "Cause 1 — RDFS and OWL-RL lose `p subPropertyOf p`, `q subPropertyOf q`,",
            "`r subPropertyOf r` and `rdfs:subPropertyOf subPropertyOf rdfs:subPropertyOf`",
            "(rdfs6 4 -> 0): p, q and r appear only as subPropertyOf ENDPOINTS here, and an",
            "endpoint is not an rdf:Property instance. rdfs5 / scm-spo is untouched — the",
            "transitive `p subPropertyOf r` is still concluded and still credited.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 3 -> 125 lines: the 113-line input-independent block cause C",
            "describes, plus 12 about this fixture's own terms —",
            "",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:p rdfs:subPropertyOf ex:q",
            "  ex:p rdfs:subPropertyOf ex:r",
            "  ex:q rdf:type rdf:Property",
            "  ex:q rdf:type rdfs:Resource",
            "  ex:q rdfs:subPropertyOf ex:q",
            "  ex:q rdfs:subPropertyOf ex:r",
            "  ex:r rdf:type rdf:Property",
            "  ex:r rdf:type rdfs:Resource",
            "  ex:r rdfs:subPropertyOf ex:r",
            "",
            "rdfs5 / scm-spo is untouched — the transitive `p subPropertyOf r` is still",
            "concluded and still credited (rdfs5=1). The three reflexives are back, but from",
            "the licensed premise this time: rdfs:subPropertyOf has an axiomatic domain AND",
            "range of rdf:Property, so p, q and r are all typed rdf:Property first.",
        ],
        quads: &[
            t(EX_P, RDFS_SUBPROPERTYOF, EX_Q),
            t(EX_Q, RDFS_SUBPROPERTYOF, EX_R),
        ],
    },
    Fixture {
        name: "subproperty_chain_near_miss",
        doc: &[
            "NEAR MISS for rdfs5 / scm-spo: the chain is broken at the join point — the",
            "second edge starts at D rather than at q — so p is not a sub-property of r.",
        ],
        exercises: &["rdfs5", "scm-spo"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 1 -> 2).",
            "Cause 1 — RDFS and OWL-RL lose the five reflexive subPropertyOf triples on D, p,",
            "q, r and rdfs:subPropertyOf (rdfs6 5 -> 0). The near miss still holds: the broken",
            "chain concludes nothing, and now the closure says exactly that.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 2 -> 127 lines: the 113-line input-independent block cause C",
            "describes, plus 14 about this fixture's own terms —",
            "",
            "  ex:D rdf:type rdf:Property",
            "  ex:D rdf:type rdfs:Resource",
            "  ex:D rdfs:subPropertyOf ex:D",
            "  ex:D rdfs:subPropertyOf ex:r",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:p rdfs:subPropertyOf ex:q",
            "  ex:q rdf:type rdf:Property",
            "  ex:q rdf:type rdfs:Resource",
            "  ex:q rdfs:subPropertyOf ex:q",
            "  ex:r rdf:type rdf:Property",
            "  ex:r rdf:type rdfs:Resource",
            "  ex:r rdfs:subPropertyOf ex:r",
            "",
            "The near miss still holds: the chain is broken at the join point, so `p",
            "subPropertyOf r` is absent while every reflexive is present.",
        ],
        quads: &[
            t(EX_P, RDFS_SUBPROPERTYOF, EX_Q),
            t(EX_D, RDFS_SUBPROPERTYOF, EX_R),
        ],
    },
    Fixture {
        name: "subproperty_rewrite",
        doc: &[
            "rdfs7 / prp-spo1: a sub-property assertion re-predicates every triple that",
            "uses the sub-property.",
        ],
        exercises: &["rdfs7", "prp-spo1"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 2 -> 3).",
            "Cause 1 — RDFS and OWL-RL lose `p subPropertyOf p`, `q subPropertyOf q` and",
            "`rdfs:subPropertyOf subPropertyOf rdfs:subPropertyOf` (rdfs6 3 -> 0).",
            "rdfs7 / prp-spo1 is untouched: `x q y` is still concluded and still credited.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 3 -> 124 lines: the 113-line input-independent block cause C",
            "describes, plus 11 about this fixture's own terms —",
            "",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:p rdfs:subPropertyOf ex:q",
            "  ex:q rdf:type rdf:Property",
            "  ex:q rdf:type rdfs:Resource",
            "  ex:q rdfs:subPropertyOf ex:q",
            "  ex:x ex:p ex:y",
            "  ex:x ex:q ex:y",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y rdf:type rdfs:Resource",
            "",
            "rdfs7 / prp-spo1 is untouched: `x q y` is still concluded and still credited.",
        ],
        quads: &[t(EX_P, RDFS_SUBPROPERTYOF, EX_Q), t(EX_X, EX_P, EX_Y)],
    },
    Fixture {
        name: "subproperty_rewrite_near_miss",
        doc: &[
            "NEAR MISS for rdfs7 / prp-spo1: the data triple uses r, which is not the",
            "declared sub-property, so nothing is re-predicated into q.",
        ],
        exercises: &["rdfs7", "prp-spo1"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 2 -> 3).",
            "Cause 1 — RDFS and OWL-RL lose the four reflexive subPropertyOf triples on p, q,",
            "r and rdfs:subPropertyOf (rdfs6 4 -> 0). The near miss still holds: `x q y` is",
            "absent, because the data triple uses r.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 2 -> 126 lines: the 113-line input-independent block cause C",
            "describes, plus 13 about this fixture's own terms —",
            "",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:p rdfs:subPropertyOf ex:q",
            "  ex:q rdf:type rdf:Property",
            "  ex:q rdf:type rdfs:Resource",
            "  ex:q rdfs:subPropertyOf ex:q",
            "  ex:r rdf:type rdf:Property",
            "  ex:r rdf:type rdfs:Resource",
            "  ex:r rdfs:subPropertyOf ex:r",
            "  ex:x ex:r ex:y",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y rdf:type rdfs:Resource",
            "",
            "The near miss still holds: `x q y` is absent, because the data triple uses r.",
        ],
        quads: &[t(EX_P, RDFS_SUBPROPERTYOF, EX_Q), t(EX_X, EX_R, EX_Y)],
    },
    Fixture {
        name: "property_typed",
        doc: &[
            "rdfs6: a resource typed rdf:Property is a sub-property of itself. p appears",
            "ONLY as the subject of that typing, never as a predicate, so the conclusion",
            "`p subPropertyOf p` is licensed by rdfs6 and by nothing else here.",
        ],
        exercises: &["rdfs6"],
        changed: &[
            "Cause 2 does NOT apply: the input already uses rdf:type as a predicate, so the RDF",
            "closure is unchanged and only its budget moves.",
            "Cause 1 — RDFS and OWL-RL lose `rdf:type subPropertyOf rdf:type` and",
            "`rdfs:subPropertyOf subPropertyOf rdfs:subPropertyOf` (rdfs6 3 -> 1). The one",
            "LICENSED conclusion stays: `p rdfs:subPropertyOf p`, from the premise",
            "`p rdf:type rdf:Property` this fixture asserts. That is the whole point — rdfs6",
            "was narrowed to its specification premise, not switched off.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 2 -> 116 lines: the 113-line input-independent block cause C",
            "describes, plus 3 about this fixture's own terms —",
            "",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "",
            "The smallest diff in the corpus, and the most direct: p is typed rdf:Property by",
            "the input itself, so rdfs6's premise never needed the axioms. Only rdfs4's",
            "rdfs:Resource typing is new.",
        ],
        quads: &[t(EX_P, RDF_TYPE, RDF_PROPERTY)],
    },
    Fixture {
        name: "property_typed_near_miss",
        doc: &[
            "NEAR MISS for rdfs6: the rdf:Property typing names q instead of p, and p is",
            "absent from the graph entirely, so `p subPropertyOf p` is not concluded.",
            "",
            "Note WHY the near miss removes p rather than merely un-typing it: the chase",
            "USED to fire the reflexive rule on every PREDICATE as well, so an un-typed p",
            "still standing in predicate position would have been re-concluded anyway.",
            "That is no longer so — `divergence_broad_triggers` is where the change is",
            "accounted for — but the fixture stays as it is: a near miss that would still",
            "hold under a broader rule is the stronger control, not the weaker one.",
        ],
        exercises: &["rdfs6"],
        changed: &[
            "Cause 2 does NOT apply: the input already uses rdf:type as a predicate, so the RDF",
            "closure is unchanged and only its budget moves.",
            "Cause 1 — RDFS and OWL-RL lose `rdf:type subPropertyOf rdf:type` and",
            "`rdfs:subPropertyOf subPropertyOf rdfs:subPropertyOf` (rdfs6 3 -> 1); the licensed",
            "`q rdfs:subPropertyOf q` stays. The near miss still holds: `p subPropertyOf p` is",
            "absent, and now it is absent because p is absent rather than in spite of it.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 2 -> 116 lines: the 113-line input-independent block cause C",
            "describes, plus 3 about this fixture's own terms —",
            "",
            "  ex:q rdf:type rdf:Property",
            "  ex:q rdf:type rdfs:Resource",
            "  ex:q rdfs:subPropertyOf ex:q",
            "",
            "The near miss still holds: `p subPropertyOf p` is absent because p is absent.",
        ],
        quads: &[t(EX_Q, RDF_TYPE, RDF_PROPERTY)],
    },
    Fixture {
        name: "class_typed",
        doc: &[
            "rdfs8 and rdfs10: a resource typed rdfs:Class is a sub-class of rdfs:Resource",
            "and of itself.",
        ],
        exercises: &["rdfs8", "rdfs10"],
        changed: &[
            "Cause 2 does NOT apply: the input already uses rdf:type as a predicate, so the RDF",
            "closure is unchanged and only its budget moves.",
            "Cause 1 — RDFS and OWL-RL lose `rdfs:Resource rdfs:subClassOf rdfs:Resource`",
            "(rdfs10 2 -> 1), which fired on the ENDPOINT of rdfs8's own conclusion and not on",
            "an rdfs:Class instance, and the three reflexive subPropertyOf triples on rdf:type,",
            "rdfs:subClassOf and rdfs:subPropertyOf (rdfs6 3 -> 0). Both licensed conclusions",
            "stay: `C rdfs:subClassOf rdfs:Resource` (rdfs8) and `C rdfs:subClassOf C`",
            "(rdfs10, on the premise `C rdf:type rdfs:Class` this fixture asserts).",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 3 -> 117 lines: the 113-line input-independent block cause C",
            "describes, plus 4 about this fixture's own terms —",
            "",
            "  ex:C rdf:type rdfs:Class",
            "  ex:C rdf:type rdfs:Resource",
            "  ex:C rdfs:subClassOf ex:C",
            "  ex:C rdfs:subClassOf rdfs:Resource",
            "",
            "rdfs8 and rdfs10 are untouched: `C rdfs:subClassOf rdfs:Resource` and `C",
            "rdfs:subClassOf C` are still concluded from the input's own rdfs:Class typing.",
        ],
        quads: &[t(EX_C, RDF_TYPE, RDFS_CLASS)],
    },
    Fixture {
        name: "class_typed_near_miss",
        doc: &[
            "NEAR MISS for rdfs8 and rdfs10: C is typed, but not as rdfs:Class, and it is",
            "not a subClassOf endpoint either — so neither the rdfs:Resource conclusion nor",
            "the reflexive one is licensed.",
        ],
        exercises: &["rdfs8", "rdfs10"],
        changed: &[
            "Cause 2 does NOT apply: the input already uses rdf:type as a predicate, so the RDF",
            "closure is unchanged and only its budget moves.",
            "Cause 1 — RDFS and OWL-RL lose `rdf:type subPropertyOf rdf:type` and",
            "`rdfs:subPropertyOf subPropertyOf rdfs:subPropertyOf` (rdfs6 2 -> 0). The near",
            "miss still holds: neither `C rdfs:subClassOf rdfs:Resource` nor",
            "`C rdfs:subClassOf C` is concluded, because C is not typed rdfs:Class.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 1 -> 119 lines: the 113-line input-independent block cause C",
            "describes, plus 6 about this fixture's own terms —",
            "",
            "  ex:C rdf:type ex:NotAClass",
            "  ex:C rdf:type rdfs:Resource",
            "  ex:NotAClass rdf:type rdfs:Class",
            "  ex:NotAClass rdf:type rdfs:Resource",
            "  ex:NotAClass rdfs:subClassOf ex:NotAClass",
            "  ex:NotAClass rdfs:subClassOf rdfs:Resource",
            "",
            "The near miss still holds for C: it is not typed rdfs:Class, so neither `C ⊑",
            "rdfs:Resource` nor `C ⊑ C` appears. NotAClass itself now IS an rdfs:Class — it",
            "is the object of an rdf:type triple, whose axiomatic range is rdfs:Class — which",
            "is a conclusion about NotAClass and not about C, and leaves the control intact.",
        ],
        quads: &[t(EX_C, RDF_TYPE, EX_NOT_A_CLASS)],
    },
    Fixture {
        name: "container_membership",
        doc: &[
            "rdfs12: a resource typed rdfs:ContainerMembershipProperty is a sub-property",
            "of rdfs:member. The typing is the graph's own — the axiomatic typings of",
            "rdf:_1, rdf:_2, … are members of the one family this chase cannot assert, so",
            "this is the only way the rule reaches a premise.",
        ],
        exercises: &["rdfs12"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[t(EX_P, RDF_TYPE, RDFS_CONTAINERMEMBERSHIPPROPERTY)],
    },
    Fixture {
        name: "container_membership_near_miss",
        doc: &[
            "NEAR MISS for rdfs12: the container-membership typing names q instead of p,",
            "and p is absent from the graph entirely, so `p rdfs:subPropertyOf rdfs:member`",
            "is not concluded.",
        ],
        exercises: &["rdfs12"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[t(EX_Q, RDF_TYPE, RDFS_CONTAINERMEMBERSHIPPROPERTY)],
    },
    Fixture {
        name: "datatype_declared",
        doc: &[
            "rdfs13: a resource typed rdfs:Datatype is a sub-class of rdfs:Literal. The",
            "datatype is an example.org IRI rather than one of the three RDF 1.2 makes",
            "mandatory, so the conclusion is attributable to the graph's own typing and",
            "not to rdfs1's premise-free one.",
        ],
        exercises: &["rdfs13"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[t(EX_DT, RDF_TYPE, RDFS_DATATYPE)],
    },
    Fixture {
        name: "datatype_declared_near_miss",
        doc: &[
            "NEAR MISS for rdfs13: the datatype IRI is typed, but not as rdfs:Datatype, so",
            "`dt rdfs:subClassOf rdfs:Literal` is not concluded.",
        ],
        exercises: &["rdfs13"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[t(EX_DT, RDF_TYPE, EX_NOT_A_CLASS)],
    },
    Fixture {
        name: "subclass_instance",
        doc: &["rdfs9 / cax-sco: a sub-class assertion re-types an instance."],
        exercises: &["rdfs9", "cax-sco"],
        changed: &[
            "Cause 2 does NOT apply: the input already uses rdf:type as a predicate, so the RDF",
            "closure is unchanged and only its budget moves.",
            "Cause 1 — RDFS and OWL-RL lose `A rdfs:subClassOf A` and `B rdfs:subClassOf B`",
            "(rdfs10 2 -> 0, fired on subClassOf endpoints) and the three reflexive",
            "subPropertyOf triples on rdf:type, rdfs:subClassOf and rdfs:subPropertyOf",
            "(rdfs6 3 -> 0). rdfs9 / cax-sco is untouched: `x rdf:type B` is still concluded.",
            "",
            "AT THE AXIOMATIC PATH — NEW FIXTURE, so nothing moved: this golden did not exist.",
            "Its RDFS closure is 118 lines: the 113-line input-independent block cause C",
            "describes, plus 5 about this fixture:",
            "",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:ContainerMembershipProperty",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:p rdfs:subPropertyOf rdfs:member",
            "",
            "NEW FIXTURE. rdfs12's only premise is a typing the graph asserts: the axiomatic",
            "typings of rdf:_1, rdf:_2, … are in the one family this chase cannot assert.",
            "",
            "AT THE AXIOMATIC PATH — NEW FIXTURE, so nothing moved: this golden did not exist.",
            "Its RDFS closure is 118 lines: the 113-line input-independent block cause C",
            "describes, plus 5 about this fixture:",
            "",
            "  ex:q rdf:type rdf:Property",
            "  ex:q rdf:type rdfs:ContainerMembershipProperty",
            "  ex:q rdf:type rdfs:Resource",
            "  ex:q rdfs:subPropertyOf ex:q",
            "  ex:q rdfs:subPropertyOf rdfs:member",
            "",
            "NEW FIXTURE.",
            "",
            "AT THE AXIOMATIC PATH — NEW FIXTURE, so nothing moved: this golden did not exist.",
            "Its RDFS closure is 119 lines: the 113-line input-independent block cause C",
            "describes, plus 6 about this fixture:",
            "",
            "  ex:dt rdf:type rdfs:Class",
            "  ex:dt rdf:type rdfs:Datatype",
            "  ex:dt rdf:type rdfs:Resource",
            "  ex:dt rdfs:subClassOf ex:dt",
            "  ex:dt rdfs:subClassOf rdfs:Literal",
            "  ex:dt rdfs:subClassOf rdfs:Resource",
            "",
            "NEW FIXTURE. rdfs13 fires four times: once for ex:dt, and once each for the three",
            "datatypes rdfs1 types premise-free — which is why the positive uses an",
            "example.org IRI, so its conclusion is attributable to the graph's own typing.",
            "",
            "AT THE AXIOMATIC PATH — NEW FIXTURE, so nothing moved: this golden did not exist.",
            "Its RDFS closure is 119 lines: the 113-line input-independent block cause C",
            "describes, plus 6 about this fixture:",
            "",
            "  ex:NotAClass rdf:type rdfs:Class",
            "  ex:NotAClass rdf:type rdfs:Resource",
            "  ex:NotAClass rdfs:subClassOf ex:NotAClass",
            "  ex:NotAClass rdfs:subClassOf rdfs:Resource",
            "  ex:dt rdf:type ex:NotAClass",
            "  ex:dt rdf:type rdfs:Resource",
            "",
            "NEW FIXTURE.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 3 -> 125 lines: the 113-line input-independent block cause C",
            "describes, plus 12 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Class",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A rdfs:subClassOf ex:A",
            "  ex:A rdfs:subClassOf ex:B",
            "  ex:A rdfs:subClassOf rdfs:Resource",
            "  ex:B rdf:type rdfs:Class",
            "  ex:B rdf:type rdfs:Resource",
            "  ex:B rdfs:subClassOf ex:B",
            "  ex:B rdfs:subClassOf rdfs:Resource",
            "  ex:x rdf:type ex:A",
            "  ex:x rdf:type ex:B",
            "  ex:x rdf:type rdfs:Resource",
            "",
            "rdfs9 / cax-sco is untouched: `x rdf:type B` is still concluded.",
        ],
        quads: &[t(EX_A, RDFS_SUBCLASSOF, EX_B), t(EX_X, RDF_TYPE, EX_A)],
    },
    Fixture {
        name: "subclass_instance_near_miss",
        doc: &[
            "NEAR MISS for rdfs9 / cax-sco: x is an instance of D, which is not the",
            "sub-class the axiom names, so it is not re-typed into B.",
        ],
        exercises: &["rdfs9", "cax-sco"],
        changed: &[
            "Cause 2 does NOT apply: the input already uses rdf:type as a predicate, so the RDF",
            "closure is unchanged and only its budget moves.",
            "Cause 1 — RDFS and OWL-RL lose the same five triples as `subclass_instance`:",
            "`A rdfs:subClassOf A`, `B rdfs:subClassOf B` (rdfs10 2 -> 0) and the reflexive",
            "subPropertyOf triples on rdf:type, rdfs:subClassOf and rdfs:subPropertyOf",
            "(rdfs6 3 -> 0). The near miss still holds: `x rdf:type B` is absent.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 2 -> 128 lines: the 113-line input-independent block cause C",
            "describes, plus 15 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Class",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A rdfs:subClassOf ex:A",
            "  ex:A rdfs:subClassOf ex:B",
            "  ex:A rdfs:subClassOf rdfs:Resource",
            "  ex:B rdf:type rdfs:Class",
            "  ex:B rdf:type rdfs:Resource",
            "  ex:B rdfs:subClassOf ex:B",
            "  ex:B rdfs:subClassOf rdfs:Resource",
            "  ex:D rdf:type rdfs:Class",
            "  ex:D rdf:type rdfs:Resource",
            "  ex:D rdfs:subClassOf ex:D",
            "  ex:D rdfs:subClassOf rdfs:Resource",
            "  ex:x rdf:type ex:D",
            "  ex:x rdf:type rdfs:Resource",
            "",
            "The near miss still holds: `x rdf:type B` is absent.",
        ],
        quads: &[t(EX_A, RDFS_SUBCLASSOF, EX_B), t(EX_X, RDF_TYPE, EX_D)],
    },
    Fixture {
        name: "subclass_chain",
        doc: &[
            "AWKWARD CASE — a subClassOf chain deep enough that a single round cannot",
            "close it. A ⊑ B ⊑ C ⊑ D ⊑ E ⊑ F with x an A: the semi-naive frontier must",
            "carry derived edges into later rounds for `A ⊑ F` and `x a F` to appear, so",
            "this fixture is the fixpoint's own test as well as rdfs11's.",
        ],
        exercises: &["rdfs11", "scm-sco", "rdfs9", "cax-sco"],
        changed: &[
            "Cause 2 does NOT apply: the input already uses rdf:type as a predicate, so the RDF",
            "closure is unchanged and only its budget moves.",
            "Cause 1 — RDFS and OWL-RL lose nine triples, 30 lines down to 21: the six",
            "reflexive `Ci rdfs:subClassOf Ci` for A, B, C, D, E and F (rdfs10 6 -> 0, all six",
            "fired on subClassOf endpoints) and the three reflexive subPropertyOf triples on",
            "rdf:type, rdfs:subClassOf and rdfs:subPropertyOf (rdfs6 3 -> 0).",
            "What this fixture is FOR is untouched: rdfs11 / scm-sco still contributes 10",
            "triples and rdfs9 / cax-sco still contributes 5, so `A rdfs:subClassOf F` and",
            "`x rdf:type F` are still reached — the multi-round fixpoint still closes.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 21 -> 159 lines: the 113-line input-independent block cause C",
            "describes, plus 46 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Class",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A rdfs:subClassOf ex:A",
            "  ex:A rdfs:subClassOf ex:B",
            "  ex:A rdfs:subClassOf ex:C",
            "  ex:A rdfs:subClassOf ex:D",
            "  ex:A rdfs:subClassOf ex:E",
            "  ex:A rdfs:subClassOf ex:F",
            "  ex:A rdfs:subClassOf rdfs:Resource",
            "  ex:B rdf:type rdfs:Class",
            "  ex:B rdf:type rdfs:Resource",
            "  ex:B rdfs:subClassOf ex:B",
            "  ex:B rdfs:subClassOf ex:C",
            "  ex:B rdfs:subClassOf ex:D",
            "  ex:B rdfs:subClassOf ex:E",
            "  ex:B rdfs:subClassOf ex:F",
            "  ex:B rdfs:subClassOf rdfs:Resource",
            "  ex:C rdf:type rdfs:Class",
            "  ex:C rdf:type rdfs:Resource",
            "  ex:C rdfs:subClassOf ex:C",
            "  ex:C rdfs:subClassOf ex:D",
            "  ex:C rdfs:subClassOf ex:E",
            "  ex:C rdfs:subClassOf ex:F",
            "  ex:C rdfs:subClassOf rdfs:Resource",
            "  ex:D rdf:type rdfs:Class",
            "  ex:D rdf:type rdfs:Resource",
            "  ex:D rdfs:subClassOf ex:D",
            "  ex:D rdfs:subClassOf ex:E",
            "  ex:D rdfs:subClassOf ex:F",
            "  ex:D rdfs:subClassOf rdfs:Resource",
            "  ex:E rdf:type rdfs:Class",
            "  ex:E rdf:type rdfs:Resource",
            "  ex:E rdfs:subClassOf ex:E",
            "  ex:E rdfs:subClassOf ex:F",
            "  ex:E rdfs:subClassOf rdfs:Resource",
            "  ex:F rdf:type rdfs:Class",
            "  ex:F rdf:type rdfs:Resource",
            "  ex:F rdfs:subClassOf ex:F",
            "  ex:F rdfs:subClassOf rdfs:Resource",
            "  ex:x rdf:type ex:A",
            "  ex:x rdf:type ex:B",
            "  ex:x rdf:type ex:C",
            "  ex:x rdf:type ex:D",
            "  ex:x rdf:type ex:E",
            "  ex:x rdf:type ex:F",
            "  ex:x rdf:type rdfs:Resource",
            "",
            "The largest closure in the corpus, and what it is FOR is untouched: rdfs11 /",
            "scm-sco still contributes 10 triples, rdfs9 still reaches `x rdf:type F`, and the",
            "multi-round fixpoint still closes. The six reflexive `Ci ⊑ Ci` are back — every",
            "Ci is now typed rdfs:Class from the axiomatic domain and range of rdfs:subClassOf",
            "— and this is the corpus's budget worst case at 2,577 join steps.",
        ],
        quads: &[
            t(EX_A, RDFS_SUBCLASSOF, EX_B),
            t(EX_B, RDFS_SUBCLASSOF, EX_C),
            t(EX_C, RDFS_SUBCLASSOF, EX_D),
            t(EX_D, RDFS_SUBCLASSOF, EX_E),
            t(EX_E, RDFS_SUBCLASSOF, EX_F),
            t(EX_X, RDF_TYPE, EX_A),
        ],
    },
    Fixture {
        name: "subclass_chain_near_miss",
        doc: &[
            "NEAR MISS for rdfs11 / scm-sco: the two edges do not meet — A ⊑ B and E ⊑ F",
            "share no endpoint — so `A ⊑ F` is not derivable at any depth.",
        ],
        exercises: &["rdfs11", "scm-sco"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 1 -> 2).",
            "Cause 1 — RDFS and OWL-RL lose `A rdfs:subClassOf A`, `B rdfs:subClassOf B`,",
            "`E rdfs:subClassOf E` and `F rdfs:subClassOf F` (rdfs10 4 -> 0) and the reflexive",
            "subPropertyOf triples on rdfs:subClassOf and rdfs:subPropertyOf (rdfs6 2 -> 0).",
            "The near miss still holds: `A rdfs:subClassOf F` is absent at every depth.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 2 -> 131 lines: the 113-line input-independent block cause C",
            "describes, plus 18 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Class",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A rdfs:subClassOf ex:A",
            "  ex:A rdfs:subClassOf ex:B",
            "  ex:A rdfs:subClassOf rdfs:Resource",
            "  ex:B rdf:type rdfs:Class",
            "  ex:B rdf:type rdfs:Resource",
            "  ex:B rdfs:subClassOf ex:B",
            "  ex:B rdfs:subClassOf rdfs:Resource",
            "  ex:E rdf:type rdfs:Class",
            "  ex:E rdf:type rdfs:Resource",
            "  ex:E rdfs:subClassOf ex:E",
            "  ex:E rdfs:subClassOf ex:F",
            "  ex:E rdfs:subClassOf rdfs:Resource",
            "  ex:F rdf:type rdfs:Class",
            "  ex:F rdf:type rdfs:Resource",
            "  ex:F rdfs:subClassOf ex:F",
            "  ex:F rdfs:subClassOf rdfs:Resource",
            "",
            "The near miss still holds: `A rdfs:subClassOf F` is absent at every depth.",
        ],
        quads: &[
            t(EX_A, RDFS_SUBCLASSOF, EX_B),
            t(EX_E, RDFS_SUBCLASSOF, EX_F),
        ],
    },
    Fixture {
        name: "symmetric",
        doc: &[
            "prp-symp: a symmetric property mirrors its triples. Also the NEAR MISS for",
            "prp-trp — it differs from `transitive` in exactly one term, the property",
            "characteristic — so the two fixtures are each other's control.",
        ],
        exercises: &["prp-symp"],
        changed: &[
            "Cause 2 does NOT apply: the input already uses rdf:type as a predicate, so the RDF",
            "closure is unchanged and only its budget moves.",
            "Cause 1 — RDFS and OWL-RL lose the three reflexive subPropertyOf triples on p,",
            "rdf:type and rdfs:subPropertyOf (rdfs6 3 -> 0). Under RDFS the closure is now the",
            "input alone, which is correct: RDFS has no rule for owl:SymmetricProperty.",
            "prp-symp is untouched — OWL-RL still mirrors both triples.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 3 -> 126 lines: the 113-line input-independent block cause C",
            "describes, plus 13 about this fixture's own terms —",
            "",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdf:type owl:SymmetricProperty",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:x ex:p ex:y",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y ex:p ex:z",
            "  ex:y rdf:type rdfs:Resource",
            "  ex:z rdf:type rdfs:Resource",
            "  owl:SymmetricProperty rdf:type rdfs:Class",
            "  owl:SymmetricProperty rdf:type rdfs:Resource",
            "  owl:SymmetricProperty rdfs:subClassOf rdfs:Resource",
            "  owl:SymmetricProperty rdfs:subClassOf owl:SymmetricProperty",
            "",
            "Under RDFS the closure is no longer the input alone, but RDFS still has no rule",
            "for owl:SymmetricProperty: owl:SymmetricProperty appears only as an rdf:type",
            "object, so the axiomatic range of rdf:type makes it an rdfs:Class and nothing",
            "more is said about it. prp-symp is untouched — the OWL-RL section is identical.",
        ],
        quads: &[
            t(EX_P, RDF_TYPE, OWL_SYMMETRIC),
            t(EX_X, EX_P, EX_Y),
            t(EX_Y, EX_P, EX_Z),
        ],
    },
    Fixture {
        name: "transitive",
        doc: &[
            "prp-trp: a transitive property composes its triples. Also the NEAR MISS for",
            "prp-symp; see `symmetric`.",
        ],
        exercises: &["prp-trp"],
        changed: &[
            "Cause 2 does NOT apply: the input already uses rdf:type as a predicate, so the RDF",
            "closure is unchanged and only its budget moves.",
            "Cause 1 — RDFS and OWL-RL lose the three reflexive subPropertyOf triples on p,",
            "rdf:type and rdfs:subPropertyOf (rdfs6 3 -> 0). Under RDFS the closure is now the",
            "input alone, which is correct: RDFS has no rule for owl:TransitiveProperty.",
            "prp-trp is untouched — OWL-RL still composes `x p z`.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 3 -> 126 lines: the 113-line input-independent block cause C",
            "describes, plus 13 about this fixture's own terms —",
            "",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdf:type owl:TransitiveProperty",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:x ex:p ex:y",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y ex:p ex:z",
            "  ex:y rdf:type rdfs:Resource",
            "  ex:z rdf:type rdfs:Resource",
            "  owl:TransitiveProperty rdf:type rdfs:Class",
            "  owl:TransitiveProperty rdf:type rdfs:Resource",
            "  owl:TransitiveProperty rdfs:subClassOf rdfs:Resource",
            "  owl:TransitiveProperty rdfs:subClassOf owl:TransitiveProperty",
            "",
            "As `symmetric`: RDFS says nothing about owl:TransitiveProperty beyond its being",
            "the object of an rdf:type triple and therefore an rdfs:Class. prp-trp is",
            "untouched — the OWL-RL section is identical.",
        ],
        quads: &[
            t(EX_P, RDF_TYPE, OWL_TRANSITIVE),
            t(EX_X, EX_P, EX_Y),
            t(EX_Y, EX_P, EX_Z),
        ],
    },
    Fixture {
        name: "inverse_pair",
        doc: &[
            "AWKWARD CASE — one owl:inverseOf axiom exercised in BOTH directions. `x p y`",
            "drives prp-inv1 (mirroring a p-triple into q) and `u q v` drives prp-inv2",
            "(mirroring a q-triple into p) from the same axiom, so the split of the inverse",
            "index into its two halves is observable rather than merely asserted.",
        ],
        exercises: &["prp-inv1", "prp-inv2"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 3 -> 4).",
            "Cause 1 — RDFS and OWL-RL lose the four reflexive subPropertyOf triples on p, q,",
            "rdfs:subPropertyOf and owl:inverseOf (rdfs6 4 -> 0). Both halves of the axiom are",
            "untouched: prp-inv1 still mirrors `x p y` into q and prp-inv2 still mirrors",
            "`u q v` into p, each still credited under its own id.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 3 -> 129 lines: the 113-line input-independent block cause C",
            "describes, plus 16 about this fixture's own terms —",
            "",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:p owl:inverseOf ex:q",
            "  ex:q rdf:type rdf:Property",
            "  ex:q rdf:type rdfs:Resource",
            "  ex:q rdfs:subPropertyOf ex:q",
            "  ex:u ex:q ex:v",
            "  ex:u rdf:type rdfs:Resource",
            "  ex:v rdf:type rdfs:Resource",
            "  ex:x ex:p ex:y",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y rdf:type rdfs:Resource",
            "  owl:inverseOf rdf:type rdf:Property",
            "  owl:inverseOf rdf:type rdfs:Resource",
            "  owl:inverseOf rdfs:subPropertyOf owl:inverseOf",
            "",
            "Both halves of the axiom are untouched — the OWL-RL section is identical.",
            "owl:inverseOf is now typed rdf:Property because it is a PREDICATE (rdfD2), which",
            "is the one place this fixture shows rdfD2 reaching something the axioms do not.",
        ],
        quads: &[
            t(EX_P, OWL_INVERSEOF, EX_Q),
            t(EX_X, EX_P, EX_Y),
            t(EX_U, EX_Q, EX_V),
        ],
    },
    Fixture {
        name: "inverse_pair_near_miss",
        doc: &[
            "NEAR MISS for prp-inv1 and prp-inv2: the axiom names r as p's inverse, not q,",
            "so neither mirror between p and q is licensed.",
        ],
        exercises: &["prp-inv1", "prp-inv2"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 3 -> 4).",
            "Cause 1 — RDFS loses the four reflexive subPropertyOf triples on p, q,",
            "rdfs:subPropertyOf and owl:inverseOf (rdfs6 4 -> 0); OWL-RL loses those four and",
            "`r subPropertyOf r` as well (rdfs6 5 -> 0), r being a predicate only because",
            "prp-inv1's own conclusion uses it. The near miss still holds: neither mirror",
            "between p and q appears.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 3 -> 130 lines: the 113-line input-independent block cause C",
            "describes, plus 17 about this fixture's own terms —",
            "",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:p owl:inverseOf ex:r",
            "  ex:q rdf:type rdf:Property",
            "  ex:q rdf:type rdfs:Resource",
            "  ex:q rdfs:subPropertyOf ex:q",
            "  ex:r rdf:type rdfs:Resource",
            "  ex:u ex:q ex:v",
            "  ex:u rdf:type rdfs:Resource",
            "  ex:v rdf:type rdfs:Resource",
            "  ex:x ex:p ex:y",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y rdf:type rdfs:Resource",
            "  owl:inverseOf rdf:type rdf:Property",
            "  owl:inverseOf rdf:type rdfs:Resource",
            "  owl:inverseOf rdfs:subPropertyOf owl:inverseOf",
            "",
            "The near miss still holds: neither mirror between p and q appears.",
        ],
        quads: &[
            t(EX_P, OWL_INVERSEOF, EX_R),
            t(EX_X, EX_P, EX_Y),
            t(EX_U, EX_Q, EX_V),
        ],
    },
    Fixture {
        name: "equivalent_class",
        doc: &[
            "scm-eqc1: owl:equivalentClass is mutual rdfs:subClassOf. Also the NEAR MISS",
            "for scm-eqp1 — it differs from `equivalent_property` in exactly one term, the",
            "equivalence predicate.",
        ],
        exercises: &["scm-eqc1"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 1 -> 2).",
            "Cause 1 — RDFS loses `rdfs:subPropertyOf subPropertyOf rdfs:subPropertyOf` and",
            "`owl:equivalentClass subPropertyOf owl:equivalentClass` (rdfs6 2 -> 0), leaving the",
            "input alone, which is correct: RDFS has no rule for owl:equivalentClass. OWL-RL",
            "loses those two and `rdfs:subClassOf subPropertyOf rdfs:subClassOf` (rdfs6 3 -> 0).",
            "`A rdfs:subClassOf A` and `B rdfs:subClassOf B` STAY, and are not reflexive-rule",
            "survivors: scm-eqc1 gives both `A subClassOf B` and `B subClassOf A`, and",
            "rdfs11 / scm-sco composes each pair — the tally still reads scm-sco=2.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 1 -> 119 lines: the 113-line input-independent block cause C",
            "describes, plus 6 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A owl:equivalentClass ex:B",
            "  ex:B rdf:type rdfs:Resource",
            "  owl:equivalentClass rdf:type rdf:Property",
            "  owl:equivalentClass rdf:type rdfs:Resource",
            "  owl:equivalentClass rdfs:subPropertyOf owl:equivalentClass",
            "",
            "RDFS still has no rule for owl:equivalentClass — it is typed rdf:Property because",
            "it is a predicate, and that is all. `A ⊑ A` and `B ⊑ B` are NOT here: nothing",
            "types A or B an rdfs:Class under RDFS, since owl:equivalentClass has no",
            "axiomatic range. The OWL-RL section, where scm-eqc1 does license them, is",
            "identical to before.",
        ],
        quads: &[t(EX_A, OWL_EQUIVALENTCLASS, EX_B)],
    },
    Fixture {
        name: "equivalent_property",
        doc: &[
            "scm-eqp1: owl:equivalentProperty is mutual rdfs:subPropertyOf. Also the NEAR",
            "MISS for scm-eqc1; see `equivalent_class`.",
        ],
        exercises: &["scm-eqp1"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 1 -> 2).",
            "Cause 1 — RDFS and OWL-RL lose",
            "`rdfs:subPropertyOf subPropertyOf rdfs:subPropertyOf` and",
            "`owl:equivalentProperty subPropertyOf owl:equivalentProperty` (rdfs6 2 -> 0). The",
            "RDFS closure is now the input alone, which is correct: RDFS has no rule for",
            "owl:equivalentProperty. `A subPropertyOf A` and `B subPropertyOf B` STAY under",
            "OWL-RL, licensed by rdfs5 / scm-spo over scm-eqp1's two edges (scm-spo=2), not by",
            "the reflexive rule.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 1 -> 119 lines: the 113-line input-independent block cause C",
            "describes, plus 6 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A owl:equivalentProperty ex:B",
            "  ex:B rdf:type rdfs:Resource",
            "  owl:equivalentProperty rdf:type rdf:Property",
            "  owl:equivalentProperty rdf:type rdfs:Resource",
            "  owl:equivalentProperty rdfs:subPropertyOf owl:equivalentProperty",
            "",
            "As `equivalent_class`, with owl:equivalentProperty. The OWL-RL section is",
            "identical.",
        ],
        quads: &[t(EX_A, OWL_EQUIVALENTPROPERTY, EX_B)],
    },
    Fixture {
        name: "shared_conclusion",
        doc: &[
            "AWKWARD CASE — two rules that both conclude the SAME triple. `x rdf:type C`",
            "follows from rdfs9 / cax-sco (x is an A, A ⊑ C) and independently from",
            "rdfs2 / prp-dom (p has domain C, and x is the subject of a p-triple). Exactly",
            "one of them is credited — whichever reached it first in the chase's firing",
            "order — and the golden's per-rule tally is where that choice is pinned. The",
            "count a report gives is 'triples this rule was FIRST to add', so a re-derived",
            "triple contributes to neither total.",
        ],
        exercises: &["rdfs9", "cax-sco", "rdfs2", "prp-dom"],
        changed: &[
            "Cause 2 does NOT apply: the input already uses rdf:type as a predicate, so the RDF",
            "closure is unchanged and only its budget moves.",
            "Cause 1 — RDFS and OWL-RL lose `A rdfs:subClassOf A` and `C rdfs:subClassOf C`",
            "(rdfs10 2 -> 0) and the five reflexive subPropertyOf triples on p, rdf:type,",
            "rdfs:domain, rdfs:subClassOf and rdfs:subPropertyOf (rdfs6 5 -> 0).",
            "THE SHARED CONCLUSION ITSELF DID NOT MOVE. `x rdf:type C` is still concluded and",
            "is still credited to rdfs9 / cax-sco rather than to rdfs2 / prp-dom — but for a",
            "stated reason now rather than by firing order: the evaluator picks a round's",
            "winner by a total order over observable provenance, and rdfs9's sources",
            "(`A subClassOf C`, `x a A`) sort before rdfs2's (`p domain C`, `x p y`).",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 5 -> 131 lines: the 113-line input-independent block cause C",
            "describes, plus 18 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Class",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A rdfs:subClassOf ex:A",
            "  ex:A rdfs:subClassOf ex:C",
            "  ex:A rdfs:subClassOf rdfs:Resource",
            "  ex:C rdf:type rdfs:Class",
            "  ex:C rdf:type rdfs:Resource",
            "  ex:C rdfs:subClassOf ex:C",
            "  ex:C rdfs:subClassOf rdfs:Resource",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:domain ex:C",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:x ex:p ex:y",
            "  ex:x rdf:type ex:A",
            "  ex:x rdf:type ex:C",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y rdf:type rdfs:Resource",
            "",
            "THE SHARED CONCLUSION ITSELF DID NOT MOVE: `x rdf:type C` is still concluded and",
            "still credited to rdfs9 rather than to rdfs2, on the same provenance ordering as",
            "before. rdfs9's tally rose from 1 to 4 because the axioms give it three more",
            "conclusions (through `⊑ rdfs:Resource`), and the sum of the tally is still",
            "exactly the inferred-triple count, which is the invariant this fixture guards.",
        ],
        quads: &[
            t(EX_A, RDFS_SUBCLASSOF, EX_C),
            t(EX_X, RDF_TYPE, EX_A),
            t(EX_P, RDFS_DOMAIN, EX_C),
            t(EX_X, EX_P, EX_Y),
        ],
    },
    Fixture {
        name: "triple_term",
        doc: &[
            "AWKWARD CASE — an RDF 1.2 triple term in object position, under a",
            "sub-property axiom that forces a conclusion to be built AROUND it.",
            "",
            "rdfs14 and rdfs14a conclude about a FRESH blank node standing for the triple",
            "term, which is an existential head this crate's Datalog evaluator refuses, so",
            "neither fires and a triple-term boundary is reported; the chase therefore",
            "interns the term as one atomic term and never looks inside it. The",
            "second quad makes the harder thing happen: rdfs7 re-predicates",
            "`x says <<( A ⊑ B )>>` into a `mentions` triple, and the object of that",
            "conclusion has to be re-interned.",
            "",
            "AN EARLIER FIX THIS FIXTURE PINS. The engine used to emit",
            "`x mentions rdfs:Resource` for that conclusion: re-interning folded EVERY",
            "triple term to rdfs:Resource on the way back into the dataset builder, on the",
            "stated assumption that the RDFS/OWL-RL rules never derive one in that",
            "position. rdfs7 / prp-spo1 does, and the substitution was UNSOUND — nothing",
            "in this input entails `x mentions rdfs:Resource`. A triple term is now",
            "rebuilt structurally and recursively, so it re-materializes as itself.",
        ],
        exercises: &["rdfs7", "prp-spo1"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 3 -> 4).",
            "Cause 1 — RDFS and OWL-RL lose `A rdfs:subClassOf A` and `B rdfs:subClassOf B`",
            "(rdfs10 2 -> 0) and the four reflexive subPropertyOf triples on says, mentions,",
            "rdfs:subClassOf and rdfs:subPropertyOf (rdfs6 4 -> 0).",
            "The line this fixture exists for is untouched: rdfs7 / prp-spo1 still concludes",
            "`x mentions <<( A rdfs:subClassOf B )>>`, with the triple term carried through as",
            "itself. The engine now interns a triple term as one lexical surface rather than as",
            "one interner id, which is the same opacity by a different mechanism — rdfs14 /",
            "rdfs14a still do not fire, and the triple-term boundary is still reported.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 4 -> 132 lines: the 113-line input-independent block cause C",
            "describes, plus 19 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Class",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A rdfs:subClassOf ex:A",
            "  ex:A rdfs:subClassOf ex:B",
            "  ex:A rdfs:subClassOf rdfs:Resource",
            "  ex:B rdf:type rdfs:Class",
            "  ex:B rdf:type rdfs:Resource",
            "  ex:B rdfs:subClassOf ex:B",
            "  ex:B rdfs:subClassOf rdfs:Resource",
            "  ex:mentions rdf:type rdf:Property",
            "  ex:mentions rdf:type rdfs:Resource",
            "  ex:mentions rdfs:subPropertyOf ex:mentions",
            "  ex:says rdf:type rdf:Property",
            "  ex:says rdf:type rdfs:Resource",
            "  ex:says rdfs:subPropertyOf ex:mentions",
            "  ex:says rdfs:subPropertyOf ex:says",
            "  ex:x ex:mentions ( ex:A rdfs:subClassOf ex:B )",
            "  ex:x ex:says ( ex:A rdfs:subClassOf ex:B )",
            "  ex:x rdf:type rdfs:Resource",
            "",
            "The line this fixture exists for is untouched: rdfs7 still concludes `x mentions",
            "<<( A rdfs:subClassOf B )>>` with the triple term carried through as itself, and",
            "there is still exactly one `x mentions` triple. rdfs14 / rdfs14a still do not",
            "fire and the triple-term boundary is still reported — and the boundary's REASON",
            "is now the accurate one: not that the term is opaque, but that both rules",
            "conclude about a fresh blank node the evaluator may not mint. Note that rdfs4",
            "does NOT type the triple term rdfs:Resource: that conclusion has a triple term",
            "in subject position, so it is generalized RDF and is dropped at the boundary.",
        ],
        quads: &[
            t(EX_SAYS, RDFS_SUBPROPERTYOF, EX_MENTIONS),
            t_quoted(EX_X, EX_SAYS, EX_A, RDFS_SUBCLASSOF, EX_B),
            t(EX_A, RDFS_SUBCLASSOF, EX_B),
        ],
    },
    Fixture {
        name: "divergence_literal_subject",
        doc: &[
            "DOCUMENTED DIVERGENCE 1 of 2 — NARROWER CONCLUSIONS. It is not a divergence",
            "of the CALCULUS: it is the RDF 1.2 IR declining to hold what the calculus",
            "concludes, and it survives the engine swap for that reason.",
            "",
            "`p rdfs:range A` with `x p \"cat\"^^xsd:string` makes rdfs3 / prp-rng conclude",
            "`\"cat\" rdf:type A`, whose subject is a literal. That is a GENERALIZED-RDF",
            "triple, which the RDF 1.2 dataset IR cannot represent, so the conclusion is",
            "abandoned when the answer is materialized, the drop is counted, and a",
            "generalized-rdf boundary is reported. The golden captures that answer.",
            "",
            "rdfs4 reaches the same wall from the other side — its object clause concludes",
            "`\"cat\" rdf:type rdfs:Resource`, which is a literal subject for the same",
            "reason — so the boundary now has two independent producers here rather than",
            "one, and the literal still never starts a line of the closure.",
            "",
            "The generalized fact is NOT withheld from the calculus — it stays in the",
            "evaluator's own term space and may still serve as a premise. Nothing here",
            "gives it one, so this closure is the input alone either way; what the",
            "distinction buys is that a REPRESENTABLE conclusion is never lost merely",
            "because its derivation passed through an unrepresentable one.",
        ],
        exercises: &["rdfs3", "prp-rng"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 2 -> 3).",
            "Cause 1 — RDFS and OWL-RL lose the three reflexive subPropertyOf triples on p,",
            "rdfs:range and rdfs:subPropertyOf (rdfs6 3 -> 0).",
            "THE DIVERGENCE THIS FIXTURE ISOLATES DID NOT MOVE, in either direction. No",
            "conclusion puts the literal in subject position, `\"cat\" rdf:type A` is still",
            "absent, and — the observable that was actually at risk — the generalized-rdf",
            "boundary is still REPORTED in both regimes. The evaluator now derives that",
            "conclusion in its own term space and abandons it when the answer is materialized",
            "back into the RDF 1.2 IR, so the boundary had to survive a mechanism change, not",
            "merely a rule change.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 2 -> 123 lines: the 113-line input-independent block cause C",
            "describes, plus 10 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Class",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A rdfs:subClassOf ex:A",
            "  ex:A rdfs:subClassOf rdfs:Resource",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:range ex:A",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:x ex:p \"cat\"",
            "  ex:x rdf:type rdfs:Resource",
            "",
            "THE DIVERGENCE THIS FIXTURE ISOLATES DID NOT MOVE: `\"cat\" rdf:type A` is still",
            "absent and the generalized-rdf boundary is still REPORTED. The claim that the",
            "closure equals the input is gone, and had to go — it was never the divergence,",
            "and rdfs4 now types x, p and A rdfs:Resource. rdfs4 also strengthens the",
            "boundary: its object clause concludes into subject position too, so the literal",
            "is now dropped twice over rather than once.",
        ],
        quads: &[t(EX_P, RDFS_RANGE, EX_A), t_lit(EX_X, EX_P, "cat")],
    },
    Fixture {
        name: "divergence_broad_triggers",
        doc: &[
            "DOCUMENTED DIVERGENCE 2 of 2 — BROADER TRIGGERS. This fixture has now been",
            "the control for two different questions, and holding both is what it is for.",
            "",
            "Nothing here is ASSERTED to be an rdfs:Class or an rdf:Property. The",
            "hand-written chase nevertheless emitted reflexive subClassOf on every",
            "subClassOf ENDPOINT and reflexive subPropertyOf on every PREDICATE; narrowing",
            "rdfs10 and rdfs6 to their specification premises removed those, correctly.",
            "",
            "But the triples were RDFS-entailed all along, by the longer path that",
            "narrowing left unwalked: rdfs:subClassOf has an AXIOMATIC rdfs:domain and",
            "rdfs:range of rdfs:Class, so rdfs2 and rdfs3 type both endpoints and only",
            "THEN does rdfs10 apply; rdfD2 types the predicate and only then does rdfs6.",
            "With the axiomatic triples asserted, the RDFS closure contains all five",
            "conclusions AND the premises that license each of them — while OWL-RL, which",
            "omits the axiomatic triples, still closes to the input alone.",
            "",
            "The property that tells the path from the shortcut is x and y: ex:p declares",
            "no domain and no range, so nothing types them rdfs:Class, and `x ⊑ x` and",
            "`y ⊑ y` are absent under BOTH lanes. A chase that had merely restored the",
            "endpoint shortcut would emit them.",
        ],
        exercises: &["rdfs6", "rdfs10"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 2 -> 3).",
            "Cause 1, IN FULL — this is the fixture that isolates it. All five unlicensed",
            "conclusions are gone from RDFS and OWL-RL (rdfs6 3 -> 0, rdfs10 2 -> 0):",
            "",
            "  A rdfs:subClassOf A                                    (rdfs10 on an endpoint)",
            "  B rdfs:subClassOf B                                    (rdfs10 on an endpoint)",
            "  p rdfs:subPropertyOf p                                 (rdfs6 on a predicate)",
            "  rdfs:subClassOf rdfs:subPropertyOf rdfs:subClassOf     (rdfs6 on a predicate)",
            "  rdfs:subPropertyOf rdfs:subPropertyOf rdfs:subPropertyOf  (rdfs6 on a predicate)",
            "",
            "Nothing here is typed rdfs:Class or rdf:Property, so the specification licenses",
            "neither rule, and the closure is now the input alone. The direction is the one",
            "predicted: FEWER triples.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 2 -> 128 lines: the 113-line input-independent block cause C",
            "describes, plus 15 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Class",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A rdfs:subClassOf ex:A",
            "  ex:A rdfs:subClassOf ex:B",
            "  ex:A rdfs:subClassOf rdfs:Resource",
            "  ex:B rdf:type rdfs:Class",
            "  ex:B rdf:type rdfs:Resource",
            "  ex:B rdfs:subClassOf ex:B",
            "  ex:B rdfs:subClassOf rdfs:Resource",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:x ex:p ex:y",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y rdf:type rdfs:Resource",
            "",
            "THIS IS THE FIXTURE THE WHOLE CHANGE IS ABOUT, AND IT HAS CHANGED SIDES UNDER",
            "RDFS. All five conclusions the engine swap removed are back — but each is now",
            "accompanied, in this same closure, by the premise the specification requires:",
            "`A rdf:type rdfs:Class` and `B rdf:type rdfs:Class` (rdfs2 / rdfs3 over the",
            "axiomatic domain and range of rdfs:subClassOf) before `A ⊑ A` and `B ⊑ B`, and",
            "`p rdf:type rdf:Property` (rdfD2) before `p ⊑ p`. The two rdfs:subClassOf /",
            "rdfs:subPropertyOf self-edges are in the 113-line block, from the same path.",
            "THE CONTROL SURVIVES AND IS WHAT THE TEST NOW CHECKS: x and y are typed",
            "rdfs:Resource by rdfs4 and NOTHING ELSE — ex:p declares no domain and no range,",
            "so neither is an rdfs:Class and neither `x ⊑ x` nor `y ⊑ y` appears. A chase",
            "that had simply restored the endpoint shortcut would emit them.",
            "UNDER OWL-RL NOTHING CHANGED AT ALL: that lane asserts no axiomatic triples,",
            "its closure is still the input alone, and the shortcut is still gone.",
        ],
        quads: &[t(EX_A, RDFS_SUBCLASSOF, EX_B), t(EX_X, EX_P, EX_Y)],
    },
];

// ── Building and running a fixture ──────────────────────────────────────────────

/// Intern one fixture object term into `builder`.
fn intern_object(builder: &mut RdfDatasetBuilder, term: Term) -> TermId {
    match term {
        Term::Iri(iri) => builder.intern_iri(iri),
        Term::Literal { lexical, datatype } => {
            builder.intern_literal(RdfLiteral::typed(lexical, datatype))
        }
        Term::Quoted(s, p, o) => {
            let s = builder.intern_iri(s);
            let p = builder.intern_iri(p);
            let o = builder.intern_iri(o);
            builder.intern_triple(s, p, o)
        }
    }
}

/// Freeze `fixture`'s quads into a dataset.
fn build(fixture: &Fixture) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for quad in fixture.quads {
        let s = builder.intern_iri(quad.s);
        let p = builder.intern_iri(quad.p);
        let o = intern_object(&mut builder, quad.o);
        let g = quad.g.map(|g| builder.intern_iri(g));
        builder.push_quad(s, p, o, g);
    }
    builder.freeze().expect("fixture dataset freezes")
}

/// The fixture named `name`.
fn fixture(name: &str) -> &'static Fixture {
    CORPUS
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no fixture named {name} in the corpus"))
}

/// The canonical N-Quads lines of `regime`'s closure over the fixture named `name`.
fn closure_lines(name: &str, regime: Regime) -> BTreeSet<String> {
    let ds = build(fixture(name));
    let (closed, _report) = materialize(&ds, regime).expect("the four oracle regimes run");
    canonicalize(&closed)
        .nquads
        .lines()
        .map(ToOwned::to_owned)
        .collect()
}

/// One default-graph triple over three IRIs, in the canonical N-Quads spelling.
fn nquads_line(s: &str, p: &str, o: &str) -> String {
    format!("<{s}> <{p}> <{o}> .")
}

// ── Rendering a golden ──────────────────────────────────────────────────────────

/// The four regimes `materialize` can run, with the names the goldens use.
///
/// `OWL-Direct`, `RIF` and `D` are refused by the façade — they need inputs it does not
/// have — so an oracle over `materialize` cannot and must not include them.
const ORACLE_REGIMES: [(Regime, &str); 4] = [
    (Regime::Simple, "Simple"),
    (Regime::Rdf, "RDF"),
    (Regime::Rdfs, "RDFS"),
    (Regime::OwlRl, "OWL-RL"),
];

/// How many space-separated tokens a wrapped list puts on one line.
///
/// A fixed count rather than a column budget: the wrap must not depend on how long a rule
/// id happens to be, or renaming one would reflow every golden.
const TOKENS_PER_LINE: usize = 8;

/// Append `tokens` under `label`, wrapped at [`TOKENS_PER_LINE`] with a fixed indent.
fn write_wrapped(out: &mut String, indent: &str, label: &str, tokens: &[String]) {
    let _ = writeln!(out, "{indent}{label} ({})", tokens.len());
    for chunk in tokens.chunks(TOKENS_PER_LINE) {
        let _ = writeln!(out, "{indent}    {}", chunk.join(" "));
    }
}

/// Render `report` deterministically, field by field, in the report's documented order.
///
/// Every sequence a report carries already has a fixed order — missing and fired rules in
/// specification table order, boundaries in `Construct` declaration order — so this
/// function adds no sorting of its own. A boundary is rendered by name only: its reason is
/// a pure function of the construct (`Boundary::of` is the only constructor), pinned by the
/// crate's unit tests, and repeating a paragraph of prose in thirty golden files would make
/// the goldens harder to diff without making them say more.
fn render_report(out: &mut String, report: &ReasoningReport) {
    let indent = "  ";
    let _ = writeln!(
        out,
        "{indent}completeness: {}",
        if report.completeness().is_exact() {
            "exact"
        } else {
            "sound-incomplete"
        }
    );
    let missing: Vec<String> = report
        .completeness()
        .missing()
        .iter()
        .map(|rule| rule.as_str().to_owned())
        .collect();
    write_wrapped(out, indent, "missing:", &missing);
    let fired: Vec<String> = report
        .rules_fired()
        .iter()
        .map(|&(rule, count)| format!("{}={count}", rule.as_str()))
        .collect();
    write_wrapped(out, indent, "rules-fired:", &fired);
    let boundaries: Vec<String> = report
        .boundaries()
        .iter()
        .map(|boundary| boundary.construct().as_str().to_owned())
        .collect();
    write_wrapped(out, indent, "boundaries:", &boundaries);
    let budget = report.budget();
    let _ = writeln!(
        out,
        "{indent}budget: join-steps={} stored-facts={} term-arena-bytes={}",
        budget.join_steps(),
        budget.stored_facts(),
        budget.term_arena_bytes()
    );
    let _ = writeln!(out, "{indent}contract-hash: {}", report.contract_hash());
    let _ = writeln!(
        out,
        "{indent}inconsistency: {}",
        report
            .inconsistency()
            .map_or_else(|| "none".to_owned(), |w| w.rule().as_str().to_owned())
    );
    let _ = writeln!(out, "{indent}overclaims: {}", report.overclaims());
}

/// Append `lines` as `#`-prefixed header comment lines, with a bare `#` for a blank.
fn write_comment_block(out: &mut String, lines: &[&str]) {
    for line in lines {
        if line.is_empty() {
            out.push_str("#\n");
        } else {
            let _ = writeln!(out, "# {line}");
        }
    }
}

/// Append a canonical N-Quads block under a `--- label (N lines) ---` banner.
fn write_nquads(out: &mut String, label: &str, nquads: &str) {
    let count = nquads.lines().count();
    let _ = writeln!(out, "--- {label} ({count} lines) ---");
    out.push_str(nquads);
}

/// Render the whole golden for `fixture`: the header, the input, and every regime's
/// closure and report.
fn render_golden(fixture: &Fixture) -> String {
    let mut out = String::new();
    out.push_str(
        "# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. \
         <paudley@blackcatinformatics.ca>\n\
         # SPDX-License-Identifier: MIT OR Apache-2.0\n\
         #\n\
         # GOLDEN — generated by crates/entail/tests/oracle.rs. Do not hand-edit.\n\
         # Regenerate deliberately with:\n\
         #   cargo test -p purrdf-entail --test oracle -- --ignored --exact \
         regenerate_goldens\n\
         #\n",
    );
    let _ = writeln!(out, "# fixture: {}", fixture.name);
    write_comment_block(&mut out, fixture.doc);
    out.push_str("#\n");
    write_comment_block(&mut out, ENGINE_SWAP);
    out.push_str("#\n");
    write_comment_block(&mut out, AXIOMATIC_PATH);
    out.push_str("#\n# WHAT MOVED IN THIS GOLDEN:\n#\n");
    write_comment_block(&mut out, fixture.changed);
    let _ = writeln!(out, "# exercises: {}", fixture.exercises.join(" "));
    out.push('\n');

    let ds = build(fixture);
    write_nquads(&mut out, "input", &canonicalize(&ds).nquads);

    for (regime, name) in ORACLE_REGIMES {
        let (closed, report) = materialize(&ds, regime).expect("the four oracle regimes run");
        let _ = writeln!(out, "\n=== regime {name} ===");
        write_nquads(&mut out, "closure", &canonicalize(&closed).nquads);
        out.push_str("--- report ---\n");
        render_report(&mut out, &report);
    }
    out
}

/// The directory the goldens live in.
fn goldens_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("goldens")
}

/// The golden path for `name`.
fn golden_path(name: &str) -> PathBuf {
    goldens_dir().join(format!("{name}.golden"))
}

// ── The oracle gate ─────────────────────────────────────────────────────────────

/// THE ORACLE. Every fixture's committed golden equals what the engine produces now.
///
/// This is the test the engine swap has to survive. A failure here is not a flaky
/// assertion: it means the closure or the report changed for an input the corpus covers.
/// Either that change is the point of the commit — in which case regenerate, and the diff
/// is the evidence a reviewer reads — or it is a regression.
#[test]
fn goldens_match_the_current_engine() {
    let mut mismatched = Vec::new();
    for fixture in CORPUS {
        let path = golden_path(fixture.name);
        let Ok(committed) = std::fs::read_to_string(&path) else {
            mismatched.push(format!("{}: no committed golden", fixture.name));
            continue;
        };
        let rendered = render_golden(fixture);
        if committed != rendered {
            let first_diff = committed
                .lines()
                .zip(rendered.lines())
                .position(|(a, b)| a != b)
                .map_or_else(
                    || "length differs".to_owned(),
                    |index| {
                        let line = index + 1;
                        let committed_line = committed.lines().nth(index).unwrap_or("");
                        let rendered_line = rendered.lines().nth(index).unwrap_or("");
                        format!("line {line}:\n    committed: {committed_line}\n    now:       {rendered_line}")
                    },
                );
            mismatched.push(format!("{}: {first_diff}", fixture.name));
        }
    }
    assert!(
        mismatched.is_empty(),
        "the closure or the report changed for {} fixture(s):\n{}\n\nIf the change is \
         deliberate, regenerate with:\n  cargo test -p purrdf-entail --test oracle -- \
         --ignored --exact regenerate_goldens",
        mismatched.len(),
        mismatched.join("\n")
    );
}

/// Maintainer-only: rewrite every committed golden from the current engine.
///
/// `#[ignore]`d so a normal test run can only ever compare. Rewriting the oracle is a
/// deliberate act whose whole value is the diff it produces, so it must be typed on
/// purpose and reviewed.
#[test]
#[ignore = "maintainer-only: rewrites the committed goldens; run deliberately"]
fn regenerate_goldens() {
    std::fs::create_dir_all(goldens_dir()).expect("create the goldens directory");
    for fixture in CORPUS {
        std::fs::write(golden_path(fixture.name), render_golden(fixture))
            .expect("write the golden");
    }
}

/// Rendering is a pure function of the fixture: two renders in one process agree byte for
/// byte, for every fixture and therefore for every regime.
///
/// Cheap, but it is the property the goldens rest on. `materialize` seeds a freshly-hashed
/// fact set per call, so an order-sensitive emission would show up here as two different
/// strings from the same input.
#[test]
fn rendering_is_byte_stable_within_a_run() {
    for fixture in CORPUS {
        assert_eq!(
            render_golden(fixture),
            render_golden(fixture),
            "{} rendered differently twice in one process",
            fixture.name
        );
    }
}

/// The goldens directory holds exactly the corpus: no orphans, no duplicates.
///
/// Without this, deleting a fixture would leave a golden nobody compares — an oracle that
/// looks larger than it is.
#[test]
fn the_goldens_directory_is_exactly_the_corpus() {
    let expected: BTreeSet<String> = CORPUS
        .iter()
        .map(|fixture| format!("{}.golden", fixture.name))
        .collect();
    assert_eq!(expected.len(), CORPUS.len(), "two fixtures share a name");
    let found: BTreeSet<String> = std::fs::read_dir(goldens_dir())
        .expect("read the goldens directory")
        .map(|entry| {
            entry
                .expect("directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(found, expected);
}

/// Every rule id a fixture claims to exercise is a real rule id.
#[test]
fn fixture_exercise_lists_name_real_rules() {
    for fixture in CORPUS {
        for spelling in fixture.exercises {
            let rule = RuleId::from_str(spelling)
                .unwrap_or_else(|_| panic!("{}: {spelling} is not a rule id", fixture.name));
            assert_eq!(
                rule.as_str(),
                *spelling,
                "{}: use the canonical spelling",
                fixture.name
            );
        }
    }
}

// ── The rule fixture registry ───────────────────────────────────────────────────

/// One side of a rule's evidence: a fixture, and the conclusion to look for in its closure.
///
/// The conclusion is three IRIs because every conclusion this registry checks is a
/// default-graph triple over IRIs. A rule whose interesting conclusion is not of that shape
/// — the generalized-RDF case — is evidenced by its own dedicated test below, where the
/// absence of a triple is the whole point and a boundary carries the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Case {
    /// The fixture's name.
    fixture: &'static str,
    /// The conclusion triple, as `(subject, predicate, object)` IRIs.
    conclusion: (&'static str, &'static str, &'static str),
}

/// What the corpus can say about one rule of one regime.
///
/// Three states, and the gaps between them are the point. `NotYetImplemented` is a
/// COMPLETE entry: it is a true, checked statement that the chase does not fire this rule,
/// asserted against the inventory rather than assumed. A rule with no entry at all is not
/// a state this type can express, which is what makes "one fixture per rule" hold for the
/// rules nobody has written yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuleFixtures {
    /// The chase fires this rule, and the corpus proves it both ways.
    Registered {
        /// An input where the rule fires: the conclusion must be PRESENT.
        positive: Case,
        /// The same input, changed in exactly the way that removes the rule's premise:
        /// the same conclusion must be ABSENT.
        near_miss: Case,
    },
    /// The chase fires this rule with NO premise, so the two-fixture control above is
    /// not merely unwritten, it is impossible: the conclusion holds for EVERY input, and
    /// an input that denied it would be a bug in the rule rather than a near miss.
    ///
    /// The evidence is therefore rearranged rather than weakened. The presence side is
    /// asserted over the EMPTY dataset, which is the strongest witness available — a
    /// premise-free rule that fires on nothing at all is fired on everything — and the
    /// absence side moves inside that same input: a conclusion of the same SHAPE, over a
    /// term the rule must NOT range over, which is what keeps "the rule fires" from
    /// degenerating into "something types everything".
    Axiomatic {
        /// The empty dataset, and the conclusion that must be PRESENT in its closure.
        holds: Case,
        /// The same dataset, and a same-shaped conclusion that must be ABSENT from it.
        denied: Case,
    },
    /// The chase does not fire this rule, so the corpus has nothing to show.
    NotYetImplemented,
}

/// A registry row: the rule, its positive fixture and conclusion, and its near-miss
/// fixture. The conclusion is shared — a near miss asserts the ABSENCE of the very triple
/// the positive asserts the presence of, which is what makes the pair a control.
type Row = (
    RuleId,
    &'static str,
    (&'static str, &'static str, &'static str),
    &'static str,
);

/// `Regime::Rdf`'s registered rules.
const RDF_ROWS: &[Row] = &[(
    RuleId::RdfD2,
    "plain_triple",
    (EX_P, RDF_TYPE, RDF_PROPERTY),
    "named_graph",
)];

/// A premise-free rule's registry row: the rule, the fixture (always the empty dataset),
/// the conclusion that must be PRESENT in its closure, and a same-shaped conclusion that
/// must be ABSENT from it. See [`RuleFixtures::Axiomatic`] for why the control moves
/// inside one fixture instead of across two.
type AxiomaticRow = (
    RuleId,
    &'static str,
    (&'static str, &'static str, &'static str),
    (&'static str, &'static str, &'static str),
);

/// The premise-free rules the `RDFS` lane fires.
///
/// `rdfs1` types every IRI of the recognized datatype set `D` an `rdfs:Datatype`, and
/// RDF 1.2 Semantics §8 fixes `D` — for the unqualified phrase "RDFS entails" — at
/// `{rdf:langString, rdf:dirLangString, xsd:string}`. `xsd:string` is therefore typed in
/// EVERY closure, including the empty graph's, and `example.org/dt` is typed in none:
/// that pair is the whole content of the rule, stated as one presence and one absence.
const RDFS_AXIOMATIC_ROWS: &[AxiomaticRow] = &[(
    RuleId::Rdfs1,
    "empty",
    (XSD_STRING, RDF_TYPE, RDFS_DATATYPE),
    (EX_DT, RDF_TYPE, RDFS_DATATYPE),
)];

/// The premise-free rules `regime` registers.
fn axiomatic_rows(regime: Regime) -> &'static [AxiomaticRow] {
    match regime {
        Regime::Rdfs => RDFS_AXIOMATIC_ROWS,
        Regime::Simple
        | Regime::Rdf
        | Regime::OwlRl
        | Regime::OwlDirect
        | Regime::Rif
        | Regime::D => &[],
    }
}

/// `Regime::Rdfs`'s registered rules, in specification table order.
///
/// `rdfD2` heads the list because RDFS entailment subsumes RDF entailment: the lane fires
/// the §8.1.1 pattern as well as the §9.2.1 ones, and `rules(Regime::Rdfs)` has always
/// listed it. `rdfs1` is not here — it is premise-free, so it is in
/// [`RDFS_AXIOMATIC_ROWS`] instead.
const RDFS_ROWS: &[Row] = &[
    (
        RuleId::RdfD2,
        "plain_triple",
        (EX_P, RDF_TYPE, RDF_PROPERTY),
        "named_graph",
    ),
    (
        RuleId::Rdfs2,
        "domain",
        (EX_X, RDF_TYPE, EX_A),
        "domain_near_miss",
    ),
    (
        RuleId::Rdfs3,
        "range",
        (EX_Y, RDF_TYPE, EX_B),
        "range_near_miss",
    ),
    (
        RuleId::Rdfs4,
        "plain_triple",
        (EX_X, RDF_TYPE, RDFS_RESOURCE),
        "named_graph",
    ),
    (
        RuleId::Rdfs5,
        "subproperty_chain",
        (EX_P, RDFS_SUBPROPERTYOF, EX_R),
        "subproperty_chain_near_miss",
    ),
    (
        RuleId::Rdfs6,
        "property_typed",
        (EX_P, RDFS_SUBPROPERTYOF, EX_P),
        "property_typed_near_miss",
    ),
    (
        RuleId::Rdfs7,
        "subproperty_rewrite",
        (EX_X, EX_Q, EX_Y),
        "subproperty_rewrite_near_miss",
    ),
    (
        RuleId::Rdfs8,
        "class_typed",
        (EX_C, RDFS_SUBCLASSOF, RDFS_RESOURCE),
        "class_typed_near_miss",
    ),
    (
        RuleId::Rdfs9,
        "subclass_instance",
        (EX_X, RDF_TYPE, EX_B),
        "subclass_instance_near_miss",
    ),
    (
        RuleId::Rdfs10,
        "class_typed",
        (EX_C, RDFS_SUBCLASSOF, EX_C),
        "class_typed_near_miss",
    ),
    (
        RuleId::Rdfs11,
        "subclass_chain",
        (EX_A, RDFS_SUBCLASSOF, EX_F),
        "subclass_chain_near_miss",
    ),
    (
        RuleId::Rdfs12,
        "container_membership",
        (EX_P, RDFS_SUBPROPERTYOF, RDFS_MEMBER),
        "container_membership_near_miss",
    ),
    (
        RuleId::Rdfs13,
        "datatype_declared",
        (EX_DT, RDFS_SUBCLASSOF, RDFS_LITERAL),
        "datatype_declared_near_miss",
    ),
];

/// `Regime::OwlRl`'s registered rules, in specification table order.
///
/// Nine of them are the RDFS rules above under their OWL 2 RL names, evaluated in the OWL
/// lane — the same fixture, a different calculus — and six are the lane's own.
const OWL_RL_ROWS: &[Row] = &[
    (
        RuleId::PrpDom,
        "domain",
        (EX_X, RDF_TYPE, EX_A),
        "domain_near_miss",
    ),
    (
        RuleId::PrpRng,
        "range",
        (EX_Y, RDF_TYPE, EX_B),
        "range_near_miss",
    ),
    (
        RuleId::PrpSymp,
        "symmetric",
        (EX_Y, EX_P, EX_X),
        "transitive",
    ),
    (
        RuleId::PrpTrp,
        "transitive",
        (EX_X, EX_P, EX_Z),
        "symmetric",
    ),
    (
        RuleId::PrpSpo1,
        "subproperty_rewrite",
        (EX_X, EX_Q, EX_Y),
        "subproperty_rewrite_near_miss",
    ),
    (
        RuleId::PrpInv1,
        "inverse_pair",
        (EX_Y, EX_Q, EX_X),
        "inverse_pair_near_miss",
    ),
    (
        RuleId::PrpInv2,
        "inverse_pair",
        (EX_V, EX_P, EX_U),
        "inverse_pair_near_miss",
    ),
    (
        RuleId::CaxSco,
        "subclass_instance",
        (EX_X, RDF_TYPE, EX_B),
        "subclass_instance_near_miss",
    ),
    (
        RuleId::ScmSco,
        "subclass_chain",
        (EX_A, RDFS_SUBCLASSOF, EX_F),
        "subclass_chain_near_miss",
    ),
    (
        RuleId::ScmEqc1,
        "equivalent_class",
        (EX_B, RDFS_SUBCLASSOF, EX_A),
        "equivalent_property",
    ),
    (
        RuleId::ScmSpo,
        "subproperty_chain",
        (EX_P, RDFS_SUBPROPERTYOF, EX_R),
        "subproperty_chain_near_miss",
    ),
    (
        RuleId::ScmEqp1,
        "equivalent_property",
        (EX_B, RDFS_SUBPROPERTYOF, EX_A),
        "equivalent_class",
    ),
];

/// The rows `regime` registers.
fn rows(regime: Regime) -> &'static [Row] {
    match regime {
        Regime::Rdf => RDF_ROWS,
        Regime::Rdfs => RDFS_ROWS,
        Regime::OwlRl => OWL_RL_ROWS,
        Regime::Simple | Regime::OwlDirect | Regime::Rif | Regime::D => &[],
    }
}

/// What the corpus says about `id` under `regime`.
fn registration(regime: Regime, id: RuleId) -> RuleFixtures {
    if let Some(&(_, positive, conclusion, near_miss)) = rows(regime).iter().find(|row| row.0 == id)
    {
        return RuleFixtures::Registered {
            positive: Case {
                fixture: positive,
                conclusion,
            },
            near_miss: Case {
                fixture: near_miss,
                conclusion,
            },
        };
    }
    if let Some(&(_, fixture, holds, denied)) =
        axiomatic_rows(regime).iter().find(|row| row.0 == id)
    {
        return RuleFixtures::Axiomatic {
            holds: Case {
                fixture,
                conclusion: holds,
            },
            denied: Case {
                fixture,
                conclusion: denied,
            },
        };
    }
    RuleFixtures::NotYetImplemented
}

/// The four regimes the registry ranges over — the ones `materialize` can run.
const REGISTRY_REGIMES: [Regime; 4] = [Regime::Simple, Regime::Rdf, Regime::Rdfs, Regime::OwlRl];

/// THE REGISTRY. Every rule of every runnable regime is in exactly one of two states, and
/// the `NotYetImplemented` set is EXACTLY the inventory's gap.
///
/// Two independent statements meet here. The report DERIVES its missing list as
/// `rules(r)` minus `implemented(r)`; this test derives the same set from what the corpus
/// can and cannot demonstrate. They must agree, so:
///
/// * implementing a rule without adding fixtures fails — the id leaves `implemented`'s
///   complement while the registry still calls it `NotYetImplemented`;
/// * adding fixtures without implementing the rule fails the same way, from the other
///   side;
/// * and a `Registered` rule that stops firing fails on its own positive assertion.
///
/// That is the mechanism that makes "one test per rule" non-re-interpretable for the 66
/// OWL 2 RL rules still to come.
#[test]
fn every_rule_is_registered_or_declared_unimplemented() {
    for regime in REGISTRY_REGIMES {
        let mut not_yet: BTreeSet<RuleId> = BTreeSet::new();
        let mut registered: BTreeSet<RuleId> = BTreeSet::new();
        for &id in rules(regime) {
            match registration(regime, id) {
                RuleFixtures::Registered {
                    positive,
                    near_miss,
                } => {
                    assert!(registered.insert(id), "{regime:?} registers {id} twice");
                    assert_ne!(
                        positive.fixture, near_miss.fixture,
                        "{regime:?} / {id}: a near miss must be a DIFFERENT input"
                    );
                    assert_eq!(
                        positive.conclusion, near_miss.conclusion,
                        "{regime:?} / {id}: a near miss must deny the same conclusion the \
                         positive asserts"
                    );
                    let (s, p, o) = positive.conclusion;
                    let line = nquads_line(s, p, o);
                    assert!(
                        closure_lines(positive.fixture, regime).contains(&line),
                        "{regime:?} / {id}: positive fixture {} did not conclude {line}",
                        positive.fixture
                    );
                    assert!(
                        !closure_lines(near_miss.fixture, regime).contains(&line),
                        "{regime:?} / {id}: near-miss fixture {} concluded {line} anyway",
                        near_miss.fixture
                    );
                }
                RuleFixtures::Axiomatic { holds, denied } => {
                    assert!(registered.insert(id), "{regime:?} registers {id} twice");
                    assert_eq!(
                        holds.fixture, denied.fixture,
                        "{regime:?} / {id}: a premise-free rule's control is the SAME input"
                    );
                    assert_eq!(
                        holds.fixture, "empty",
                        "{regime:?} / {id}: a premise-free rule must be witnessed on the \
                         EMPTY dataset, which is what makes it premise-free"
                    );
                    assert_ne!(
                        holds.conclusion, denied.conclusion,
                        "{regime:?} / {id}: the denied conclusion must differ from the one \
                         that holds"
                    );
                    let lines = closure_lines(holds.fixture, regime);
                    let (s, p, o) = holds.conclusion;
                    assert!(
                        lines.contains(&nquads_line(s, p, o)),
                        "{regime:?} / {id}: the empty dataset did not conclude {}",
                        nquads_line(s, p, o)
                    );
                    let (s, p, o) = denied.conclusion;
                    assert!(
                        !lines.contains(&nquads_line(s, p, o)),
                        "{regime:?} / {id}: the rule ranged over a term it must not, \
                         concluding {}",
                        nquads_line(s, p, o)
                    );
                }
                RuleFixtures::NotYetImplemented => {
                    not_yet.insert(id);
                }
            }
        }

        // THE GAP, stated twice and checked once.
        let inventory_gap: BTreeSet<RuleId> = rules(regime)
            .iter()
            .copied()
            .filter(|rule| !implemented(regime).contains(rule))
            .collect();
        assert_eq!(
            not_yet, inventory_gap,
            "{regime:?}: the registry's unimplemented set must equal rules(r) minus \
             implemented(r), exactly"
        );
        let done: BTreeSet<RuleId> = implemented(regime).iter().copied().collect();
        assert_eq!(
            registered, done,
            "{regime:?}: every implemented rule must carry a positive and a near-miss \
             fixture, and nothing else may"
        );
    }
}

/// The registry's shape today, pinned as a ratchet.
///
/// A ratchet, not a drift guard: when a later change teaches the chase a rule these
/// numbers MUST move, in the same commit that adds the rule and its fixtures. Never widen
/// it to an inequality.
#[test]
fn the_registry_shape_is_pinned() {
    let shape: Vec<(&str, usize, usize, usize)> = REGISTRY_REGIMES
        .iter()
        .map(|&regime| {
            let total = rules(regime).len();
            let registered = rows(regime).len() + axiomatic_rows(regime).len();
            (regime_label(regime), total, registered, total - registered)
        })
        .collect();
    assert_eq!(
        shape,
        vec![
            ("Simple", 0, 0, 0),
            ("RDF", 3, 1, 2),
            ("RDFS", 18, 14, 4),
            ("OWL-RL", 78, 12, 66),
        ],
        "(regime, rules the spec defines, rules with fixtures, rules not yet implemented)"
    );
}

/// A regime's name, for messages and the shape ratchet. Exhaustive on purpose: a new
/// `Regime` variant fails to compile here.
const fn regime_label(regime: Regime) -> &'static str {
    match regime {
        Regime::Simple => "Simple",
        Regime::Rdf => "RDF",
        Regime::Rdfs => "RDFS",
        Regime::OwlRl => "OWL-RL",
        Regime::OwlDirect => "OWL-Direct",
        Regime::Rif => "RIF",
        Regime::D => "D",
    }
}

/// Every fixture the registry names is in the corpus, and every corpus fixture is used by
/// the registry or is one of the awkward/divergence cases that carry their own tests.
#[test]
fn the_registry_and_the_corpus_agree() {
    let corpus: BTreeSet<&str> = CORPUS.iter().map(|fixture| fixture.name).collect();
    let mut used: BTreeMap<&str, usize> = BTreeMap::new();
    for regime in REGISTRY_REGIMES {
        for &(_, positive, _, near_miss) in rows(regime) {
            for name in [positive, near_miss] {
                assert!(corpus.contains(name), "{name} is not in the corpus");
                *used.entry(name).or_default() += 1;
            }
        }
        for &(_, fixture, _, _) in axiomatic_rows(regime) {
            assert!(corpus.contains(fixture), "{fixture} is not in the corpus");
            *used.entry(fixture).or_default() += 1;
        }
    }
    // The fixtures the registry does NOT reach, named explicitly. Each one exists for a
    // reason the registry cannot express — a boundary, a fixpoint depth, a shared
    // conclusion, or a documented divergence — and each has its own test below.
    let unreferenced: BTreeSet<&str> = corpus
        .iter()
        .copied()
        .filter(|name| !used.contains_key(name))
        .collect();
    assert_eq!(
        unreferenced,
        [
            "divergence_broad_triggers",
            "divergence_literal_subject",
            "shared_conclusion",
            "triple_term",
        ]
        .into_iter()
        .collect::<BTreeSet<&str>>()
    );
}

// ── The awkward cases, and the two documented divergences ───────────────────────

/// The named-graph boundary: a premise in a named graph fires nothing, and the quad is
/// carried through untouched.
#[test]
fn a_named_graph_supplies_no_premises_and_receives_no_conclusions() {
    for regime in [Regime::Rdf, Regime::Rdfs, Regime::OwlRl] {
        let ds = build(fixture("named_graph"));
        let (closed, report) = materialize(&ds, regime).expect("runnable regime");
        assert!(
            report
                .boundaries()
                .iter()
                .any(|b| b.construct().as_str() == "named-graph"),
            "{regime:?} did not report the named-graph boundary"
        );
        let lines = canonicalize(&closed).nquads;
        assert!(
            lines.contains(&format!("<{EX_X}> <{EX_P}> <{EX_Y}> <{EX_G}> .")),
            "{regime:?} did not carry the named-graph quad through"
        );
        assert!(
            !lines.contains(&nquads_line(EX_P, RDF_TYPE, RDF_PROPERTY)),
            "{regime:?} drew a conclusion from a named-graph premise"
        );
    }
}

/// A triple term is one atomic term to the chase — a reported boundary — and a conclusion
/// built AROUND it carries it through unchanged.
///
/// `x says <<( A ⊑ B )>>` with `says ⊑ mentions` makes rdfs7 / prp-spo1 conclude
/// `x mentions <<( A ⊑ B )>>`: the rule rewrites the PREDICATE and copies the object
/// through, so the object of the conclusion is the object of the premise, whatever kind of
/// term that is.
///
/// The engine used to emit `x mentions rdfs:Resource` instead — the re-interning path
/// folded any triple term to `rdfs:Resource` on the way back into the dataset builder, on
/// the stated assumption that "the RDFS/OWL-RL rules never derive" one there. rdfs7 does,
/// and the substitution was UNSOUND: `x mentions rdfs:Resource` is entailed by this input
/// under none of these regimes, so it was a wrong triple rather than a missing one. Both
/// halves are asserted below — the licensed conclusion present, the fabricated one absent —
/// so a regression in either direction fails here and not only in the golden.
///
/// Opacity itself is NOT the bug and is not repaired: the chase still never reasons INTO
/// the quoted triple (rdfs14 / rdfs14a do not fire), which withholds conclusions rather
/// than inventing them, and the triple-term boundary is what tells a caller so.
#[test]
fn a_derived_triple_term_object_is_carried_through_not_folded() {
    let ds = build(fixture("triple_term"));
    for regime in [Regime::Rdfs, Regime::OwlRl] {
        let (closed, report) = materialize(&ds, regime).expect("runnable regime");
        assert!(
            report
                .boundaries()
                .iter()
                .any(|b| b.construct().as_str() == "triple-term"),
            "{regime:?} did not report the triple-term boundary"
        );
        let lines = canonicalize(&closed).nquads;
        assert!(
            lines.contains(&format!(
                "<{EX_X}> <{EX_MENTIONS}> <<( <{EX_A}> <{RDFS_SUBCLASSOF}> <{EX_B}> )>> ."
            )),
            "{regime:?}: rdfs7 did not carry the triple term through the rewrite"
        );
        assert!(
            !lines.contains(&nquads_line(EX_X, EX_MENTIONS, RDFS_RESOURCE)),
            "{regime:?}: the unsound fold to rdfs:Resource is back"
        );
        // The exact derived set about `x mentions`: one conclusion, and it is that one.
        let mentions: Vec<&str> = lines
            .lines()
            .filter(|line| line.starts_with(&format!("<{EX_X}> <{EX_MENTIONS}> ")))
            .collect();
        assert_eq!(
            mentions,
            vec![format!(
                "<{EX_X}> <{EX_MENTIONS}> <<( <{EX_A}> <{RDFS_SUBCLASSOF}> <{EX_B}> )>> ."
            )],
            "{regime:?}: the rewrite concluded more than the one licensed triple"
        );
    }
}

/// The deep chain closes: several rounds of the fixpoint are genuinely required.
#[test]
fn a_deep_subclass_chain_needs_several_rounds() {
    for regime in [Regime::Rdfs, Regime::OwlRl] {
        let lines = closure_lines("subclass_chain", regime);
        // A ⊑ F is five edges away; x a F is that plus one type hop.
        assert!(
            lines.contains(&nquads_line(EX_A, RDFS_SUBCLASSOF, EX_F)),
            "{regime:?}"
        );
        assert!(
            lines.contains(&nquads_line(EX_X, RDF_TYPE, EX_F)),
            "{regime:?}"
        );
        for class in [EX_B, EX_C, EX_D, EX_E, EX_F] {
            assert!(
                lines.contains(&nquads_line(EX_X, RDF_TYPE, class)),
                "{regime:?}"
            );
        }
    }
}

/// Two rules conclude the same triple; exactly one is credited, and the totals still add up.
#[test]
fn a_shared_conclusion_is_credited_once() {
    let ds = build(fixture("shared_conclusion"));
    for regime in [Regime::Rdfs, Regime::OwlRl] {
        let (closed, report) = materialize(&ds, regime).expect("runnable regime");
        assert!(
            canonicalize(&closed)
                .nquads
                .contains(&nquads_line(EX_X, RDF_TYPE, EX_C)),
            "{regime:?} did not conclude the shared triple"
        );
        // The invariant that makes the tally checkable: one count is one triple this rule
        // was FIRST to add, so the counts sum to the inferred triples — no double credit.
        let inferred = closed.quad_refs().count() - ds.quad_refs().count();
        let total: u64 = report.rules_fired().iter().map(|&(_, n)| n).sum();
        assert_eq!(
            usize::try_from(total).expect("count fits usize"),
            inferred,
            "{regime:?}: a shared conclusion was credited twice"
        );
    }
}

/// DOCUMENTED DIVERGENCE 1 — NARROWER CONCLUSIONS. A would-be literal subject is
/// abandoned, counted, and reported.
///
/// THE OBSERVABLE AT RISK IS THE BOUNDARY, NOT THE TRIPLES. No engine may put a literal in
/// subject position, so `"cat" rdf:type A` stays absent whatever runs — the closure could
/// not have moved and did not. What could have vanished silently is the EVIDENCE: the
/// evaluator derives the generalized triple in its own term space and meets the RDF 1.2 IR
/// only when the answer is materialized, so a materializer that simply skipped what it
/// could not represent would produce a closure that looks exactly right and a report that
/// no longer says anything was dropped. The boundary assertion below is the guard against
/// precisely that, and it is the reason this test asserts the report and not only the
/// quads.
#[test]
fn a_would_be_literal_subject_is_abandoned_and_reported() {
    let ds = build(fixture("divergence_literal_subject"));
    for regime in [Regime::Rdfs, Regime::OwlRl] {
        let (closed, report) = materialize(&ds, regime).expect("runnable regime");
        let nquads = canonicalize(&closed).nquads;
        // The literal is still in the closure — it is an input OBJECT. What may never
        // appear is a line that STARTS with it.
        assert!(
            nquads.contains(&format!("<{EX_X}> <{EX_P}> \"cat\"")),
            "{regime:?}: the input triple must survive the closure"
        );
        assert!(
            !nquads.lines().any(|line| line.starts_with('"')),
            "{regime:?}: no conclusion may put a literal in subject position"
        );
        assert!(
            !nquads.contains(&format!("<{RDF_TYPE}> <{EX_A}> .")),
            "{regime:?}: rdfs3's only candidate conclusion was the abandoned one"
        );
        assert!(
            report
                .boundaries()
                .iter()
                .any(|b| b.construct().as_str() == "generalized-rdf"),
            "{regime:?}: the abandoned conclusion must be reported, not silently dropped"
        );
        assert!(!report.overclaims(), "{regime:?}");
    }
}

/// DOCUMENTED DIVERGENCE 2 — BROADER TRIGGERS, RESOLVED, AND THE LONGER PATH WALKED.
///
/// `divergence_broad_triggers` is `A rdfs:subClassOf B` and `x p y`, and nothing in it is
/// ASSERTED to be an `rdfs:Class` or an `rdf:Property`. It has now been the control for
/// two different questions, and the difference between them is the whole point of this
/// test.
///
/// The hand-written chase emitted `A ⊑ A`, `B ⊑ B` for every subClassOf ENDPOINT and
/// `p ⊑ p` for every PREDICATE. Narrowing `rdfs6` and `rdfs10` to their specification
/// premises removed those, correctly — but it removed real entailments with them, because
/// the triples are RDFS-entailed by a LONGER path the chase did not walk. `rdfs:subClassOf`
/// has an axiomatic `rdfs:domain` and `rdfs:range` of `rdfs:Class`, so `rdfs2` and `rdfs3`
/// type both endpoints, and only THEN does `rdfs10` license the reflexive triple; `rdfD2`
/// types the predicate, and only then does `rdfs6`.
///
/// So the contract this test states is neither "the shortcut" nor "nothing". It is:
///
/// * `OWL-RL` — still nothing. OWL 2 Profiles §4.3 omits the RDF and RDFS axiomatic
///   triples, so that lane has no premise to reach and the closure is the input alone.
///   The shortcut must not come back.
/// * `RDFS` — the reflexive triples are back, each with its premise DERIVED and present
///   in the same closure, so the path is checkable rather than asserted.
/// * And the property that separates "walked the path" from "went back to the shortcut":
///   `x` and `y` are the subject and object of an `ex:p` triple whose predicate declares
///   no domain and no range, so nothing types them `rdfs:Class` — and `x ⊑ x` and
///   `y ⊑ y` must therefore still be absent, under BOTH lanes.
#[test]
fn the_reflexive_rules_fire_on_their_licensed_premises_and_the_axioms_supply_them() {
    // OWL-RL asserts no axiomatic triples, so its lane has no path to the premises and
    // this input still entails nothing at all.
    let owl = closure_lines("divergence_broad_triggers", Regime::OwlRl);
    assert_eq!(
        owl,
        [
            nquads_line(EX_A, RDFS_SUBCLASSOF, EX_B),
            nquads_line(EX_X, EX_P, EX_Y),
        ]
        .into_iter()
        .collect::<BTreeSet<String>>(),
        "OWL-RL: this input entails nothing under a lane that omits the axiomatic triples"
    );

    // RDFS reaches the premises through the axioms, and only then draws the conclusions.
    let rdfs = closure_lines("divergence_broad_triggers", Regime::Rdfs);
    for (s, p, o) in [
        // The PREMISES, derived: `rdfs:subClassOf` has an axiomatic domain and range of
        // `rdfs:Class` (rdfs2, rdfs3), and `p` is a predicate (rdfD2).
        (EX_A, RDF_TYPE, RDFS_CLASS),
        (EX_B, RDF_TYPE, RDFS_CLASS),
        (EX_P, RDF_TYPE, RDF_PROPERTY),
        // …and only then the CONCLUSIONS the reflexive rules license from them.
        (EX_A, RDFS_SUBCLASSOF, EX_A),
        (EX_B, RDFS_SUBCLASSOF, EX_B),
        (EX_P, RDFS_SUBPROPERTYOF, EX_P),
    ] {
        assert!(
            rdfs.contains(&nquads_line(s, p, o)),
            "RDFS: <{s}> <{p}> <{o}> is entailed through the axiomatic triples and is missing"
        );
    }

    // THE CONTROL. `x` and `y` are an ordinary subject and object; `ex:p` declares no
    // domain and no range, so nothing types them `rdfs:Class` and the reflexive rule has
    // no premise about them. A chase that had merely restored the endpoint shortcut would
    // fail here — under RDFS as well as under OWL-RL.
    for regime in [Regime::Rdfs, Regime::OwlRl] {
        let lines = closure_lines("divergence_broad_triggers", regime);
        for term in [EX_X, EX_Y] {
            assert!(
                !lines.contains(&nquads_line(term, RDF_TYPE, RDFS_CLASS)),
                "{regime:?}: <{term}> is typed rdfs:Class on no premise"
            );
            assert!(
                !lines.contains(&nquads_line(term, RDFS_SUBCLASSOF, term)),
                "{regime:?}: <{term}> rdfs:subClassOf itself is emitted on no premise"
            );
        }

        // NARROWED, NOT SWITCHED OFF: given the premise outright, each rule still fires.
        assert!(
            closure_lines("class_typed", regime).contains(&nquads_line(
                EX_C,
                RDFS_SUBCLASSOF,
                EX_C
            )),
            "{regime:?}: rdfs10 no longer fires on an rdfs:Class instance"
        );
        assert!(
            closure_lines("property_typed", regime).contains(&nquads_line(
                EX_P,
                RDFS_SUBPROPERTYOF,
                EX_P
            )),
            "{regime:?}: rdfs6 no longer fires on an rdf:Property instance"
        );
    }
}

/// EVERY fixture's RDFS closure contains the empty graph's RDFS closure, exactly.
///
/// The `RDFS` lane asserts the axiomatic triples as premises, and what they entail about
/// the RDF and RDFS vocabulary itself does not depend on the input at all: `rdfs4` types
/// the axioms' own terms, `rdfs2` / `rdfs3` type their subjects and objects, and the
/// reflexive rules close over those. So `empty.golden`'s 113-line closure is a LOWER BOUND
/// on every other, and that is the fact each golden's accounting leans on when it reports
/// only the lines beyond it.
///
/// It is also the guard against the accounting going stale: a change that made any of
/// those 113 lines input-dependent would leave thirty-two goldens claiming a shared block
/// they no longer share, and it would fail here rather than being spread thinly across
/// thirty-two diffs.
#[test]
fn the_rdfs_closure_of_every_fixture_contains_the_empty_one() {
    let base = closure_lines("empty", Regime::Rdfs);
    assert_eq!(
        base.len(),
        113,
        "the input-independent block every golden's accounting names"
    );
    for fixture in CORPUS {
        let lines = closure_lines(fixture.name, Regime::Rdfs);
        let absent: Vec<&String> = base.difference(&lines).collect();
        assert!(
            absent.is_empty(),
            "{}: the axiomatic block is not input-independent — {absent:?} is missing",
            fixture.name
        );
    }
    // And it really is the AXIOMS that put it there: no other lane asserts them, so no
    // other lane has it. `Simple` copies the input, and `OWL-RL` omits the axiomatic
    // triples by OWL 2 Profiles §4.3's own choice.
    for regime in [Regime::Simple, Regime::Rdf, Regime::OwlRl] {
        assert!(
            closure_lines("empty", regime).is_empty(),
            "{regime:?} closed the empty graph into something"
        );
    }
}

/// The corpus reaches every rule the chase fires, by ATTRIBUTION rather than by outcome.
///
/// The registry above asserts that each rule's conclusion is present; this asserts that
/// the rule was CREDITED for it. The two are not the same claim — `shared_conclusion` is
/// the case that separates them, where one triple has two possible producers and only the
/// first is credited — so a rule could pass the registry on a triple some other rule
/// actually derived. The union of `rules_fired` over the whole corpus closes that gap.
///
/// The expected set is `implemented(regime)`, plus — for `OWL-RL` only — the three
/// RDFS-shaped rules that lane fires under no OWL 2 RL name. Equality, not containment: a
/// rule credited that the inventory does not list is as much a defect as one the corpus
/// never reaches.
#[test]
fn every_rule_the_chase_fires_is_credited_somewhere_in_the_corpus() {
    /// The rules the `OWL-RL` lane fires that OWL 2 Profiles gives no rule id.
    const OWL_RL_RDFS_SHAPED_EXTRAS: [RuleId; 3] = [RuleId::Rdfs6, RuleId::Rdfs8, RuleId::Rdfs10];

    for (regime, label) in ORACLE_REGIMES {
        let mut credited: BTreeSet<RuleId> = BTreeSet::new();
        for fixture in CORPUS {
            let ds = build(fixture);
            let (_, report) = materialize(&ds, regime).expect("runnable regime");
            credited.extend(report.rules_fired().iter().map(|&(rule, _)| rule));
        }
        let mut expected: BTreeSet<RuleId> = implemented(regime).iter().copied().collect();
        if matches!(regime, Regime::OwlRl) {
            expected.extend(OWL_RL_RDFS_SHAPED_EXTRAS);
        }
        assert_eq!(
            credited, expected,
            "{label}: the corpus must credit exactly the rules this lane fires"
        );
    }
}

/// No run over this corpus ever overclaims — `Exact` while naming a boundary.
///
/// The crate's unit tests make this claim over their own fixtures; making it again over
/// every fixture here is cheap and widens the evidence to the awkward cases.
#[test]
fn no_report_over_the_corpus_overclaims() {
    for fixture in CORPUS {
        let ds = build(fixture);
        for (regime, name) in ORACLE_REGIMES {
            let (_, report) = materialize(&ds, regime).expect("runnable regime");
            assert!(
                !report.overclaims(),
                "{}/{name}: Exact alongside {:?}",
                fixture.name,
                report.boundaries()
            );
        }
    }
}
