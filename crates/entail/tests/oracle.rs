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
    /// The input quads.
    quads: &'static [Quad],
}

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
        quads: &[],
    },
    Fixture {
        name: "plain_triple",
        doc: &[
            "One triple with no schema at all. Under RDF this is the whole of rdfD2: the",
            "predicate is typed rdf:Property. Under Simple it is the identity closure.",
        ],
        exercises: &["rdfD2"],
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
        quads: &[t_in(EX_X, EX_P, EX_Y, EX_G), t(EX_A, RDFS_SUBCLASSOF, EX_B)],
    },
    Fixture {
        name: "domain",
        doc: &[
            "rdfs2 / prp-dom: a domain declaration types the subject of every triple with",
            "that predicate.",
        ],
        exercises: &["rdfs2", "prp-dom"],
        quads: &[t(EX_P, RDFS_DOMAIN, EX_A), t(EX_X, EX_P, EX_Y)],
    },
    Fixture {
        name: "domain_near_miss",
        doc: &[
            "NEAR MISS for rdfs2 / prp-dom: the domain is declared on a DIFFERENT property",
            "(q, not p), so the data triple's subject is not typed.",
        ],
        exercises: &["rdfs2", "prp-dom"],
        quads: &[t(EX_Q, RDFS_DOMAIN, EX_A), t(EX_X, EX_P, EX_Y)],
    },
    Fixture {
        name: "range",
        doc: &[
            "rdfs3 / prp-rng: a range declaration types the object of every triple with",
            "that predicate.",
        ],
        exercises: &["rdfs3", "prp-rng"],
        quads: &[t(EX_P, RDFS_RANGE, EX_B), t(EX_X, EX_P, EX_Y)],
    },
    Fixture {
        name: "range_near_miss",
        doc: &[
            "NEAR MISS for rdfs3 / prp-rng: the range is declared on a DIFFERENT property",
            "(q, not p), so the data triple's object is not typed.",
        ],
        exercises: &["rdfs3", "prp-rng"],
        quads: &[t(EX_Q, RDFS_RANGE, EX_B), t(EX_X, EX_P, EX_Y)],
    },
    Fixture {
        name: "subproperty_chain",
        doc: &["rdfs5 / scm-spo: rdfs:subPropertyOf is transitive."],
        exercises: &["rdfs5", "scm-spo"],
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
        quads: &[t(EX_P, RDFS_SUBPROPERTYOF, EX_Q), t(EX_X, EX_P, EX_Y)],
    },
    Fixture {
        name: "subproperty_rewrite_near_miss",
        doc: &[
            "NEAR MISS for rdfs7 / prp-spo1: the data triple uses r, which is not the",
            "declared sub-property, so nothing is re-predicated into q.",
        ],
        exercises: &["rdfs7", "prp-spo1"],
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
        quads: &[t(EX_P, RDF_TYPE, RDF_PROPERTY)],
    },
    Fixture {
        name: "property_typed_near_miss",
        doc: &[
            "NEAR MISS for rdfs6: the rdf:Property typing names q instead of p, and p is",
            "absent from the graph entirely, so `p subPropertyOf p` is not concluded.",
            "",
            "Note WHY the near miss removes p rather than merely un-typing it: the chase",
            "fires the reflexive rule on every PREDICATE as well, which is one of the two",
            "documented divergences from the spec-correct rule. The fixture",
            "`divergence_broad_triggers` isolates that; this one isolates rdfs6 proper.",
        ],
        exercises: &["rdfs6"],
        quads: &[t(EX_Q, RDF_TYPE, RDF_PROPERTY)],
    },
    Fixture {
        name: "class_typed",
        doc: &[
            "rdfs8 and rdfs10: a resource typed rdfs:Class is a sub-class of rdfs:Resource",
            "and of itself.",
        ],
        exercises: &["rdfs8", "rdfs10"],
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
        quads: &[t(EX_C, RDF_TYPE, EX_NOT_A_CLASS)],
    },
    Fixture {
        name: "subclass_instance",
        doc: &["rdfs9 / cax-sco: a sub-class assertion re-types an instance."],
        exercises: &["rdfs9", "cax-sco"],
        quads: &[t(EX_A, RDFS_SUBCLASSOF, EX_B), t(EX_X, RDF_TYPE, EX_A)],
    },
    Fixture {
        name: "subclass_instance_near_miss",
        doc: &[
            "NEAR MISS for rdfs9 / cax-sco: x is an instance of D, which is not the",
            "sub-class the axiom names, so it is not re-typed into B.",
        ],
        exercises: &["rdfs9", "cax-sco"],
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
        quads: &[t(EX_A, OWL_EQUIVALENTCLASS, EX_B)],
    },
    Fixture {
        name: "equivalent_property",
        doc: &[
            "scm-eqp1: owl:equivalentProperty is mutual rdfs:subPropertyOf. Also the NEAR",
            "MISS for scm-eqc1; see `equivalent_class`.",
        ],
        exercises: &["scm-eqp1"],
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
            "The chase interns a triple term as one atomic term and never looks inside it,",
            "so rdfs14 / rdfs14a do not fire and a triple-term boundary is reported. The",
            "second quad makes the harder thing happen: rdfs7 re-predicates",
            "`x says <<( A ⊑ B )>>` into a `mentions` triple, and the object of that",
            "conclusion has to be re-interned.",
            "",
            "WHY THIS GOLDEN CHANGED — one closure line, restated once in each of the two",
            "regimes that derive it (RDFS and OWL-RL), and nothing else:",
            "",
            "  was: <example.org/x> <example.org/mentions> <rdfs:Resource> .",
            "  now: <example.org/x> <example.org/mentions>",
            "       <<( <example.org/A> <rdfs:subClassOf> <example.org/B> )>> .",
            "",
            "The old line was UNSOUND. Re-interning folded EVERY triple term to",
            "rdfs:Resource on the way back into the dataset builder, on the stated",
            "assumption that the RDFS/OWL-RL rules never derive one in that position.",
            "rdfs7 / prp-spo1 does: it rewrites a triple's PREDICATE and copies its object",
            "through unchanged. Nothing in this input entails `x mentions rdfs:Resource` —",
            "the premises are `says ⊑ mentions` and `x says <<( A ⊑ B )>>`, and what rdfs7",
            "licenses from them is `x mentions <<( A ⊑ B )>>`, for the object that was",
            "actually there. That is the new line: a triple term is now rebuilt",
            "structurally and recursively, so it re-materializes as itself.",
            "",
            "Nothing else in this golden moves, and nothing should. The chase is",
            "untouched — it never looked inside the triple term before and still does not",
            "— so both closures still hold 10 lines, the changed line keeps its sort",
            "position (`<<(` sorts where `<http` did, immediately before the `x says`",
            "line), and the rule tallies, the join-step / stored-fact /",
            "term-arena budgets, the boundary lists and the contract hashes are all",
            "byte-identical. Only the object term the re-interner emitted for that one",
            "conclusion differs. The Simple and RDF closures are unchanged too: neither",
            "regime fires rdfs7, so neither ever re-interned the triple term.",
        ],
        exercises: &["rdfs7", "prp-spo1"],
        quads: &[
            t(EX_SAYS, RDFS_SUBPROPERTYOF, EX_MENTIONS),
            t_quoted(EX_X, EX_SAYS, EX_A, RDFS_SUBCLASSOF, EX_B),
            t(EX_A, RDFS_SUBCLASSOF, EX_B),
        ],
    },
    Fixture {
        name: "divergence_literal_subject",
        doc: &[
            "DOCUMENTED DIVERGENCE 1 of 2 — NARROWER CONCLUSIONS.",
            "",
            "`p rdfs:range A` with `x p \"cat\"^^xsd:string` makes rdfs3 / prp-rng conclude",
            "`\"cat\" rdf:type A`, whose subject is a literal. That is a GENERALIZED-RDF",
            "triple, which the RDF 1.2 dataset IR cannot represent, so the chase abandons",
            "the conclusion, counts the drop, and reports a generalized-rdf boundary. The",
            "golden captures that answer.",
            "",
            "EXPECTED DIRECTION OF CHANGE: the closure must NOT change — no engine may put",
            "a literal in subject position, so the triple stays absent. What is at risk is",
            "the EVIDENCE: a Datalog engine derives the generalized triple in its own term",
            "space and only meets the boundary when it materializes back into the IR, so",
            "the failure mode to watch for is the generalized-rdf boundary silently",
            "disappearing from the report. The drop tally, and therefore join-steps, may",
            "legitimately move; the boundary may not.",
        ],
        exercises: &["rdfs3", "prp-rng"],
        quads: &[t(EX_P, RDFS_RANGE, EX_A), t_lit(EX_X, EX_P, "cat")],
    },
    Fixture {
        name: "divergence_broad_triggers",
        doc: &[
            "DOCUMENTED DIVERGENCE 2 of 2 — BROADER TRIGGERS.",
            "",
            "Nothing here is typed rdfs:Class or rdf:Property. The spec-correct rules",
            "rdfs10 (`?c rdf:type rdfs:Class ⇒ ?c ⊑ ?c`) and rdfs6 (`?p rdf:type",
            "rdf:Property ⇒ ?p subPropertyOf ?p`) therefore have no premise to fire on.",
            "The current chase fires them anyway: reflexive subClassOf on every",
            "subClassOf ENDPOINT, and reflexive subPropertyOf on every PREDICATE. Each is",
            "sound — the first is rdfs10 composed with the domain and range axioms of",
            "rdfs:subClassOf, the second is rdfs6 composed with rdfD2 — but neither is the",
            "declared rule.",
            "",
            "EXPECTED DIRECTION OF CHANGE: FEWER triples. A spec-correct engine will emit",
            "none of `A ⊑ A`, `B ⊑ B`, `rdfs:subClassOf subPropertyOf rdfs:subClassOf` or",
            "`p subPropertyOf p` for this input, so the closure SHRINKS and the rdfs6 /",
            "rdfs10 tallies fall. That is a deliberate output change, and the diff against",
            "this golden is where it must be acknowledged.",
        ],
        exercises: &["rdfs6", "rdfs10"],
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
    for line in fixture.doc {
        if line.is_empty() {
            out.push_str("#\n");
        } else {
            let _ = writeln!(out, "# {line}");
        }
    }
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
/// Exactly two states, and the gap between them is the point. `NotYetImplemented` is a
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

/// `Regime::Rdfs`'s registered rules, in specification table order.
const RDFS_ROWS: &[Row] = &[
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
    rows(regime).iter().find(|row| row.0 == id).map_or(
        RuleFixtures::NotYetImplemented,
        |&(_, positive, conclusion, near_miss)| RuleFixtures::Registered {
            positive: Case {
                fixture: positive,
                conclusion,
            },
            near_miss: Case {
                fixture: near_miss,
                conclusion,
            },
        },
    )
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
            let registered = rows(regime).len();
            (regime_label(regime), total, registered, total - registered)
        })
        .collect();
    assert_eq!(
        shape,
        vec![
            ("Simple", 0, 0, 0),
            ("RDF", 3, 1, 2),
            ("RDFS", 18, 9, 9),
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
            "empty",
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
/// abandoned, counted, and reported; the closure gains nothing.
///
/// EXPECTED DIRECTION OF CHANGE: none, in the closure. No engine may put a literal in
/// subject position, so `"cat" rdf:type A` must stay absent. The thing that can regress is
/// the EVIDENCE — a Datalog engine derives the generalized triple in its own term space
/// and only meets the boundary when it materializes back into the RDF 1.2 IR, so the
/// failure mode is the generalized-rdf boundary quietly disappearing. The drop tally, and
/// through it join-steps, may legitimately move.
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

/// DOCUMENTED DIVERGENCE 2 — BROADER TRIGGERS. The chase fires the two reflexive rules on
/// premises the specification does not license.
///
/// `divergence_broad_triggers` types nothing as `rdfs:Class` or `rdf:Property`, so
/// spec-correct rdfs10 and rdfs6 have nothing to fire on. Every triple asserted below is
/// therefore one the current engine emits UNLICENSED. Each is sound as a composition —
/// rdfs10 with the domain/range axioms of `rdfs:subClassOf`, rdfs6 with rdfD2 — but none is
/// the declared rule.
///
/// EXPECTED DIRECTION OF CHANGE: FEWER triples. A spec-correct engine emits none of these,
/// so the closure SHRINKS and the rdfs6 / rdfs10 tallies fall. This test is written to
/// FAIL at that moment, on purpose: it is where the output change has to be acknowledged
/// rather than absorbed.
#[test]
fn the_reflexive_rules_fire_on_unlicensed_premises() {
    let lines = closure_lines("divergence_broad_triggers", Regime::Rdfs);
    for (s, p, o) in [
        // rdfs10 fired on subClassOf ENDPOINTS, not on rdfs:Class instances.
        (EX_A, RDFS_SUBCLASSOF, EX_A),
        (EX_B, RDFS_SUBCLASSOF, EX_B),
        // rdfs6 fired on every PREDICATE, not on rdf:Property instances.
        (EX_P, RDFS_SUBPROPERTYOF, EX_P),
        (RDFS_SUBCLASSOF, RDFS_SUBPROPERTYOF, RDFS_SUBCLASSOF),
    ] {
        assert!(
            lines.contains(&nquads_line(s, p, o)),
            "the broader trigger is gone: <{s}> <{p}> <{o}> is no longer emitted. That is \
             the spec-correct answer — regenerate the goldens and retire this test."
        );
    }
    // The premises the SPECIFICATION requires are genuinely absent, which is what makes
    // the four conclusions above unlicensed rather than merely redundant.
    assert!(!lines.contains(&nquads_line(EX_A, RDF_TYPE, RDFS_CLASS)));
    assert!(!lines.contains(&nquads_line(EX_P, RDF_TYPE, RDF_PROPERTY)));
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
