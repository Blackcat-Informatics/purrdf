// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `owlrl` differential oracle: an INDEPENDENT second implementation of OWL 2 RL,
//! checked against PurRDF's `OWL-RL` closure over a shared corpus.
//!
//! # Why this file exists, and how it differs from `oracle.rs`
//!
//! [`oracle.rs`](./oracle.rs) pins this crate's own closures against goldens written by
//! this crate's own author — an engine-swap guard, not an independence check. Every rule
//! in [`purrdf_entail::RuleId`] was read from the same specification by the same person
//! who wrote the fixtures that exercise it, so agreement there proves internal
//! consistency and nothing else. This file adds a SECOND implementation that nobody here
//! wrote: the `owlrl` PyPI package (pinned to **7.1.4**), the reference a Python audience
//! actually holds. Where the two engines agree on a fixture, that is real, independent
//! evidence the closure is right; where they disagree, the disagreement is either a
//! defensible profile difference (recorded in `owlrl-divergences.toml`, loaded by
//! [`load_ledger`] below) or a bug this crate must fix.
//!
//! # `owlrl` is TEST DATA, not a dependency
//!
//! Nothing in this repository imports `owlrl`, conditionally or otherwise — this crate,
//! and every crate that depends on it, builds and tests identically with `owlrl` absent
//! from the machine. Its closures were captured OFFLINE, once, with
//! `regenerate_owlrl_goldens.py` (a standalone script, run by hand with
//! `uv run --with owlrl==7.1.4 --with rdflib`), and frozen into the committed files under
//! `tests/owlrl-goldens/`. `cargo test` only ever reads those bytes from disk; see
//! [`goldens_match_owlrl_reference`].
//!
//! # The comparison basis: `owlrl.OWLRL_Semantics`, not the combined `RDFS_OWLRL_Semantics`
//!
//! `owlrl` ships two closure classes an OWL 2 RL caller might reach for.
//! `RDFS_OWLRL_Semantics` always runs the FULL eighteen-rule RDFS entailment alongside
//! the OWL 2 RL rules (its own docstring calls this "a full extension of RDFS"), which
//! would type every subject and object `rdfs:Resource` and, through the OWL/RDFS
//! "full binding" triples it injects (`owl:Thing owl:equivalentClass rdfs:Resource`, …),
//! `owl:Thing` as well — none of which OWL 2 Profiles §4.3 Tables 4-9 license, and none of
//! which `Materialization::OwlRl` computes (that regime is exactly Tables 4-9, plus the
//! three RDFS-shaped rules — `rdfs6`, `rdfs8`, `rdfs10` — the profile itself borrows). Using
//! the combined class would manufacture thousands of "divergences" that are really just a
//! wider regime being compared against a narrower one — a self-inflicted noise problem, not
//! evidence about this crate's OWL-RL implementation.
//!
//! `owlrl.OWLRL_Semantics` alone is the right comparison: it runs exactly the OWL 2 RL/RDF
//! ruleset (borrowing the same three RDFS-shaped rules the profile borrows, unconditionally,
//! the same way `Materialization::OwlRl` does) and nothing more. Measured empirically over
//! the empty dataset, the two engines' background closures already agree to within a
//! four-datatype difference this ledger explains (see `dt-type1-datatype-map-superset`)
//! — the clearest sign this is the matching pair.
//!
//! # The corpus is blank-node-free
//!
//! Every fixture writes its `rdf:List` cells (for `owl:intersectionOf`, `owl:unionOf` and
//! `owl:propertyChainAxiom`) over NAMED `example.org` list nodes rather than blank nodes,
//! exactly like [`oracle.rs`](./oracle.rs) does. `owl:` class-expression rules read a list
//! by its `rdf:first`/`rdf:rest` structure alone and do not care whether its head is a
//! blank node or an IRI, so nothing about the closure changes — and it sidesteps a real
//! problem a blank-node corpus would create: `rdflib`'s parser mints fresh, unstable blank
//! node labels on every run, and freezing a golden against them would need a canonical
//! relabeling this crate has no reason to build. A corpus with no blank nodes needs none.
//!
//! # Consistent fixtures only
//!
//! Seventeen OWL 2 RL rules conclude `false`. `owlrl` has no notion of that: it silently
//! completes a clashing graph without asserting anything resembling
//! [`EntailError::Inconsistent`], while `materialize` refuses and returns a witness instead
//! of a closure. The two engines are then answering different questions, and diffing their
//! outputs would compare a closure against a refusal. Every fixture here is therefore
//! satisfiable, and the inconsistency rules already have their own, better oracle: see
//! `CLASH_CORPUS` in [`oracle.rs`](./oracle.rs).
//!
//! # The ledger
//!
//! `owlrl-divergences.toml` ([`load_ledger`]) is the closed set of ways the two engines
//! are allowed to differ.
//! [`goldens_match_owlrl_reference`] classifies every triple present in exactly one
//! engine's closure into one of the ledger's kinds; the SET of kinds actually observed
//! across the whole corpus must equal the ledger exactly — an unlisted kind fails the
//! gate (a real divergence nobody explained), and a listed kind that stops occurring also
//! fails it (XPASS discipline: a ledger only ever shrinks on purpose). A triple that
//! matches no known kind is reported immediately as an unexplained divergence, which this
//! corpus treats as a bug report rather than something to paper over with a new entry.
//!
//! [`EXPECTED_DIVERGENCE_COUNT`] pins the ledger's size to a literal, so a change to the
//! set of allowed divergences is a reviewed edit to this file rather than something that
//! creeps in unnoticed.
//!
//! # Regenerating
//!
//! ```text
//! cargo test -p purrdf-entail --test owlrl_oracle -- --ignored --exact write_owlrl_corpus_inputs
//! uv run --with owlrl==7.1.4 --with rdflib crates/entail/tests/regenerate_owlrl_goldens.py
//! ```
//!
//! The first step is Rust and deterministic (it re-renders each fixture's canonical input
//! and writes it under `tests/owlrl-corpus/`); the second needs `owlrl` on the `PATH` via
//! `uv` and is what actually calls the reference engine. Both are `#[ignore]`d or external
//! to a normal `cargo test`, which can only ever compare.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::str::FromStr as _;
use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, TermId, canonicalize};
use purrdf_entail::{Materialization, materialize};

// ── Vocabulary ──────────────────────────────────────────────────────────────────
//
// Spelled out rather than imported from `purrdf_entail`'s `pub(crate)` vocab module, for
// the same reason `oracle.rs` spells its own out: an oracle that read the engine's own
// constants would agree with the engine by construction.

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
const RDFS_SUBCLASSOF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
const RDFS_SUBPROPERTYOF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
const RDFS_RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
const OWL_OBJECTPROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";
const OWL_SYMMETRICPROPERTY: &str = "http://www.w3.org/2002/07/owl#SymmetricProperty";
const OWL_TRANSITIVEPROPERTY: &str = "http://www.w3.org/2002/07/owl#TransitiveProperty";
const OWL_FUNCTIONALPROPERTY: &str = "http://www.w3.org/2002/07/owl#FunctionalProperty";
const OWL_INVERSEFUNCTIONALPROPERTY: &str =
    "http://www.w3.org/2002/07/owl#InverseFunctionalProperty";
const OWL_INVERSEOF: &str = "http://www.w3.org/2002/07/owl#inverseOf";
const OWL_EQUIVALENTCLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
const OWL_EQUIVALENTPROPERTY: &str = "http://www.w3.org/2002/07/owl#equivalentProperty";
const OWL_PROPERTYCHAINAXIOM: &str = "http://www.w3.org/2002/07/owl#propertyChainAxiom";
const OWL_HASKEY: &str = "http://www.w3.org/2002/07/owl#hasKey";
const OWL_ONPROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
const OWL_SOMEVALUESFROM: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";
const OWL_ALLVALUESFROM: &str = "http://www.w3.org/2002/07/owl#allValuesFrom";
const OWL_HASVALUE: &str = "http://www.w3.org/2002/07/owl#hasValue";
const OWL_INTERSECTIONOF: &str = "http://www.w3.org/2002/07/owl#intersectionOf";
const OWL_UNIONOF: &str = "http://www.w3.org/2002/07/owl#unionOf";
const OWL_DIFFERENTFROM: &str = "http://www.w3.org/2002/07/owl#differentFrom";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

// ── Fixture corpus ──────────────────────────────────────────────────────────────

/// One object-position term a fixture quad can hold.
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
}

/// One default-graph triple: an IRI subject, an IRI predicate, an object.
///
/// Every fixture here lives in the default graph — see the [module docs](self) for why
/// named graphs are out of scope for this particular oracle.
#[derive(Debug, Clone, Copy)]
struct Quad {
    s: &'static str,
    p: &'static str,
    o: Term,
}

/// A triple over three IRIs.
const fn t(s: &'static str, p: &'static str, o: &'static str) -> Quad {
    Quad {
        s,
        p,
        o: Term::Iri(o),
    }
}

/// A triple whose object is a datatyped literal.
const fn t_lit(
    s: &'static str,
    p: &'static str,
    lexical: &'static str,
    datatype: &'static str,
) -> Quad {
    Quad {
        s,
        p,
        o: Term::Literal { lexical, datatype },
    }
}

/// One input dataset of the corpus.
#[derive(Debug, Clone, Copy)]
struct Fixture {
    /// The fixture's name; also the corpus/golden file stem.
    name: &'static str,
    /// The rule families this fixture is meant to reach, by canonical `RuleId` spelling —
    /// documentation for the reader, checked to parse in
    /// [`fixture_exercise_lists_name_real_rules`].
    exercises: &'static [&'static str],
    /// The input quads.
    quads: &'static [Quad],
}

/// The corpus: one fixture per rule family this oracle exercises. Every fixture is
/// satisfiable — see the [module docs](self) for why a clashing fixture has no place
/// here — and every fixture's `rdf:List` cells, where it has any, are named IRIs rather
/// than blank nodes.
const CORPUS: &[Fixture] = &[
    Fixture {
        name: "empty",
        exercises: &["cls-thing", "cls-nothing1", "prp-ap", "dt-type1"],
        quads: &[],
    },
    Fixture {
        name: "eq_sameas_replacement",
        exercises: &[
            "eq-ref", "eq-sym", "eq-trans", "eq-rep-s", "eq-rep-p", "eq-rep-o",
        ],
        quads: &[
            t(
                "http://example.org/a",
                "http://www.w3.org/2002/07/owl#sameAs",
                "http://example.org/b",
            ),
            t(
                "http://example.org/b",
                "http://www.w3.org/2002/07/owl#sameAs",
                "http://example.org/c",
            ),
            t(
                "http://example.org/p1",
                "http://www.w3.org/2002/07/owl#sameAs",
                "http://example.org/p2",
            ),
            t(
                "http://example.org/a",
                "http://example.org/p1",
                "http://example.org/x",
            ),
            t(
                "http://example.org/x",
                "http://www.w3.org/2002/07/owl#sameAs",
                "http://example.org/y",
            ),
        ],
    },
    Fixture {
        name: "eq_different_from_symmetry",
        exercises: &["ext-eq-diff-sym"],
        quads: &[t(
            "http://example.org/a",
            OWL_DIFFERENTFROM,
            "http://example.org/b",
        )],
    },
    Fixture {
        name: "prp_domain_range",
        exercises: &["prp-dom", "prp-rng"],
        quads: &[
            t("http://example.org/p", RDFS_DOMAIN, "http://example.org/C1"),
            t("http://example.org/p", RDFS_RANGE, "http://example.org/C2"),
            t(
                "http://example.org/a",
                "http://example.org/p",
                "http://example.org/b",
            ),
        ],
    },
    Fixture {
        name: "prp_symmetric_transitive",
        exercises: &["prp-symp", "prp-trp"],
        quads: &[
            t("http://example.org/p", RDF_TYPE, OWL_SYMMETRICPROPERTY),
            t("http://example.org/p", RDF_TYPE, OWL_TRANSITIVEPROPERTY),
            t(
                "http://example.org/a",
                "http://example.org/p",
                "http://example.org/b",
            ),
            t(
                "http://example.org/b",
                "http://example.org/p",
                "http://example.org/c",
            ),
        ],
    },
    Fixture {
        name: "prp_inverse",
        exercises: &["prp-inv1", "prp-inv2"],
        quads: &[
            t(
                "http://example.org/p",
                OWL_INVERSEOF,
                "http://example.org/q",
            ),
            t(
                "http://example.org/a",
                "http://example.org/p",
                "http://example.org/b",
            ),
            t(
                "http://example.org/c",
                "http://example.org/q",
                "http://example.org/d",
            ),
        ],
    },
    Fixture {
        name: "prp_functional_and_inverse_functional",
        exercises: &["prp-fp", "prp-ifp"],
        quads: &[
            t(
                "http://example.org/hasSsn",
                RDF_TYPE,
                OWL_FUNCTIONALPROPERTY,
            ),
            t_lit(
                "http://example.org/p1",
                "http://example.org/hasSsn",
                "111-11-1111",
                XSD_STRING,
            ),
            t_lit(
                "http://example.org/p2",
                "http://example.org/hasSsn",
                "111-11-1111",
                XSD_STRING,
            ),
            t(
                "http://example.org/hasOwner",
                RDF_TYPE,
                OWL_INVERSEFUNCTIONALPROPERTY,
            ),
            t(
                "http://example.org/carX",
                "http://example.org/hasOwner",
                "http://example.org/alice",
            ),
            t(
                "http://example.org/carY",
                "http://example.org/hasOwner",
                "http://example.org/alice",
            ),
        ],
    },
    Fixture {
        name: "prp_subproperty_and_equivalent",
        exercises: &["prp-spo1", "prp-eqp1", "prp-eqp2"],
        quads: &[
            t(
                "http://example.org/parentOf",
                RDFS_SUBPROPERTYOF,
                "http://example.org/ancestorOf",
            ),
            t(
                "http://example.org/carol",
                "http://example.org/parentOf",
                "http://example.org/dave",
            ),
            t(
                "http://example.org/name",
                OWL_EQUIVALENTPROPERTY,
                "http://example.org/fullName",
            ),
            t_lit(
                "http://example.org/bob",
                "http://example.org/name",
                "Bob",
                XSD_STRING,
            ),
        ],
    },
    Fixture {
        name: "prp_property_chain",
        exercises: &["prp-spo2"],
        quads: &[
            t(
                "http://example.org/hasGrandparent",
                OWL_PROPERTYCHAINAXIOM,
                "http://example.org/chain0",
            ),
            t(
                "http://example.org/chain0",
                RDF_FIRST,
                "http://example.org/parentOf",
            ),
            t(
                "http://example.org/chain0",
                RDF_REST,
                "http://example.org/chain1",
            ),
            t(
                "http://example.org/chain1",
                RDF_FIRST,
                "http://example.org/parentOf",
            ),
            t("http://example.org/chain1", RDF_REST, RDF_NIL),
            t(
                "http://example.org/erin",
                "http://example.org/parentOf",
                "http://example.org/frank",
            ),
            t(
                "http://example.org/frank",
                "http://example.org/parentOf",
                "http://example.org/grace",
            ),
        ],
    },
    Fixture {
        name: "prp_key",
        exercises: &["prp-key"],
        quads: &[
            t(
                "http://example.org/KeyedClass",
                OWL_HASKEY,
                "http://example.org/keylist0",
            ),
            t(
                "http://example.org/keylist0",
                RDF_FIRST,
                "http://example.org/ssn",
            ),
            t("http://example.org/keylist0", RDF_REST, RDF_NIL),
            t(
                "http://example.org/x1",
                RDF_TYPE,
                "http://example.org/KeyedClass",
            ),
            t_lit(
                "http://example.org/x1",
                "http://example.org/ssn",
                "999-99-9999",
                XSD_STRING,
            ),
            t(
                "http://example.org/x2",
                RDF_TYPE,
                "http://example.org/KeyedClass",
            ),
            t_lit(
                "http://example.org/x2",
                "http://example.org/ssn",
                "999-99-9999",
                XSD_STRING,
            ),
        ],
    },
    Fixture {
        name: "cax_subclass_and_equivalent",
        exercises: &["cax-sco", "cax-eqc1", "cax-eqc2"],
        quads: &[
            t(
                "http://example.org/C1",
                RDFS_SUBCLASSOF,
                "http://example.org/C2",
            ),
            t("http://example.org/a", RDF_TYPE, "http://example.org/C1"),
            t(
                "http://example.org/C3",
                OWL_EQUIVALENTCLASS,
                "http://example.org/C4",
            ),
            t("http://example.org/b", RDF_TYPE, "http://example.org/C3"),
        ],
    },
    Fixture {
        name: "cls_intersection_and_union",
        exercises: &["cls-int1", "cls-int2", "cls-uni"],
        quads: &[
            t(
                "http://example.org/IntAB",
                OWL_INTERSECTIONOF,
                "http://example.org/intlist0",
            ),
            t(
                "http://example.org/intlist0",
                RDF_FIRST,
                "http://example.org/A",
            ),
            t(
                "http://example.org/intlist0",
                RDF_REST,
                "http://example.org/intlist1",
            ),
            t(
                "http://example.org/intlist1",
                RDF_FIRST,
                "http://example.org/B",
            ),
            t("http://example.org/intlist1", RDF_REST, RDF_NIL),
            t("http://example.org/heidi", RDF_TYPE, "http://example.org/A"),
            t("http://example.org/heidi", RDF_TYPE, "http://example.org/B"),
            t(
                "http://example.org/UniAB",
                OWL_UNIONOF,
                "http://example.org/unilist0",
            ),
            t(
                "http://example.org/unilist0",
                RDF_FIRST,
                "http://example.org/A",
            ),
            t(
                "http://example.org/unilist0",
                RDF_REST,
                "http://example.org/unilist1",
            ),
            t(
                "http://example.org/unilist1",
                RDF_FIRST,
                "http://example.org/B",
            ),
            t("http://example.org/unilist1", RDF_REST, RDF_NIL),
        ],
    },
    Fixture {
        name: "cls_restrictions",
        exercises: &["cls-svf1", "cls-svf2", "cls-avf", "cls-hv1", "cls-hv2"],
        quads: &[
            t(
                "http://example.org/R1",
                OWL_ONPROPERTY,
                "http://example.org/hasFriend",
            ),
            t(
                "http://example.org/R1",
                OWL_SOMEVALUESFROM,
                "http://example.org/Person",
            ),
            t(
                "http://example.org/ivan",
                "http://example.org/hasFriend",
                "http://example.org/judy",
            ),
            t(
                "http://example.org/judy",
                RDF_TYPE,
                "http://example.org/Person",
            ),
            t(
                "http://example.org/R2",
                OWL_ONPROPERTY,
                "http://example.org/hasFriend2",
            ),
            t(
                "http://example.org/R2",
                OWL_ALLVALUESFROM,
                "http://example.org/Trusted",
            ),
            t(
                "http://example.org/kevin",
                RDF_TYPE,
                "http://example.org/R2",
            ),
            t(
                "http://example.org/kevin",
                "http://example.org/hasFriend2",
                "http://example.org/linda",
            ),
            t(
                "http://example.org/R3",
                OWL_ONPROPERTY,
                "http://example.org/hasStatus",
            ),
            t(
                "http://example.org/R3",
                OWL_HASVALUE,
                "http://example.org/Active",
            ),
            t(
                "http://example.org/mallory",
                RDF_TYPE,
                "http://example.org/R3",
            ),
            t(
                "http://example.org/nick",
                "http://example.org/hasStatus",
                "http://example.org/Active",
            ),
        ],
    },
    Fixture {
        name: "scm_hierarchy",
        exercises: &[
            "scm-cls", "scm-sco", "scm-spo", "scm-eqc1", "scm-eqc2", "scm-eqp1", "scm-eqp2",
            "scm-dom1", "scm-dom2", "scm-rng1", "scm-rng2", "scm-op", "scm-dp",
        ],
        quads: &[
            t("http://example.org/C1", RDF_TYPE, OWL_CLASS),
            t("http://example.org/C2", RDF_TYPE, OWL_CLASS),
            t("http://example.org/C3", RDF_TYPE, OWL_CLASS),
            t(
                "http://example.org/C1",
                RDFS_SUBCLASSOF,
                "http://example.org/C2",
            ),
            t(
                "http://example.org/C2",
                RDFS_SUBCLASSOF,
                "http://example.org/C3",
            ),
            t("http://example.org/C4", RDF_TYPE, OWL_CLASS),
            t("http://example.org/C5", RDF_TYPE, OWL_CLASS),
            t(
                "http://example.org/C4",
                OWL_EQUIVALENTCLASS,
                "http://example.org/C5",
            ),
            t("http://example.org/P1", RDF_TYPE, OWL_OBJECTPROPERTY),
            t("http://example.org/P2", RDF_TYPE, OWL_OBJECTPROPERTY),
            t("http://example.org/P3", RDF_TYPE, OWL_OBJECTPROPERTY),
            t(
                "http://example.org/P1",
                RDFS_SUBPROPERTYOF,
                "http://example.org/P2",
            ),
            t(
                "http://example.org/P2",
                RDFS_SUBPROPERTYOF,
                "http://example.org/P3",
            ),
            t("http://example.org/P4", RDF_TYPE, OWL_OBJECTPROPERTY),
            t("http://example.org/P5", RDF_TYPE, OWL_OBJECTPROPERTY),
            t(
                "http://example.org/P4",
                OWL_EQUIVALENTPROPERTY,
                "http://example.org/P5",
            ),
            t("http://example.org/P6", RDF_TYPE, OWL_OBJECTPROPERTY),
            t(
                "http://example.org/P6",
                RDFS_DOMAIN,
                "http://example.org/C1",
            ),
            t("http://example.org/P7", RDF_TYPE, OWL_OBJECTPROPERTY),
            t(
                "http://example.org/P7",
                RDFS_SUBPROPERTYOF,
                "http://example.org/P6",
            ),
            t("http://example.org/P8", RDF_TYPE, OWL_OBJECTPROPERTY),
            t("http://example.org/P8", RDFS_RANGE, "http://example.org/C1"),
            t("http://example.org/P9", RDF_TYPE, OWL_OBJECTPROPERTY),
            t(
                "http://example.org/P9",
                RDFS_SUBPROPERTYOF,
                "http://example.org/P8",
            ),
        ],
    },
    Fixture {
        name: "scm_restrictions",
        exercises: &[
            "scm-svf1", "scm-svf2", "scm-avf1", "scm-avf2", "scm-hv", "scm-int", "scm-uni",
        ],
        quads: &[
            t(
                "http://example.org/C1",
                RDFS_SUBCLASSOF,
                "http://example.org/C2",
            ),
            t(
                "http://example.org/SVF1",
                OWL_ONPROPERTY,
                "http://example.org/p",
            ),
            t(
                "http://example.org/SVF1",
                OWL_SOMEVALUESFROM,
                "http://example.org/C1",
            ),
            t(
                "http://example.org/SVF2",
                OWL_ONPROPERTY,
                "http://example.org/p",
            ),
            t(
                "http://example.org/SVF2",
                OWL_SOMEVALUESFROM,
                "http://example.org/C2",
            ),
            t(
                "http://example.org/q1",
                RDFS_SUBPROPERTYOF,
                "http://example.org/q2",
            ),
            t(
                "http://example.org/SVF3",
                OWL_ONPROPERTY,
                "http://example.org/q1",
            ),
            t(
                "http://example.org/SVF3",
                OWL_SOMEVALUESFROM,
                "http://example.org/C3",
            ),
            t(
                "http://example.org/SVF4",
                OWL_ONPROPERTY,
                "http://example.org/q2",
            ),
            t(
                "http://example.org/SVF4",
                OWL_SOMEVALUESFROM,
                "http://example.org/C3",
            ),
            t(
                "http://example.org/AVF1",
                OWL_ONPROPERTY,
                "http://example.org/p",
            ),
            t(
                "http://example.org/AVF1",
                OWL_ALLVALUESFROM,
                "http://example.org/C1",
            ),
            t(
                "http://example.org/AVF2",
                OWL_ONPROPERTY,
                "http://example.org/p",
            ),
            t(
                "http://example.org/AVF2",
                OWL_ALLVALUESFROM,
                "http://example.org/C2",
            ),
            t(
                "http://example.org/AVF3",
                OWL_ONPROPERTY,
                "http://example.org/q1",
            ),
            t(
                "http://example.org/AVF3",
                OWL_ALLVALUESFROM,
                "http://example.org/C4",
            ),
            t(
                "http://example.org/AVF4",
                OWL_ONPROPERTY,
                "http://example.org/q2",
            ),
            t(
                "http://example.org/AVF4",
                OWL_ALLVALUESFROM,
                "http://example.org/C4",
            ),
            t(
                "http://example.org/HV1",
                OWL_ONPROPERTY,
                "http://example.org/q1",
            ),
            t(
                "http://example.org/HV1",
                OWL_HASVALUE,
                "http://example.org/v",
            ),
            t(
                "http://example.org/HV2",
                OWL_ONPROPERTY,
                "http://example.org/q2",
            ),
            t(
                "http://example.org/HV2",
                OWL_HASVALUE,
                "http://example.org/v",
            ),
            t(
                "http://example.org/IntCD",
                OWL_INTERSECTIONOF,
                "http://example.org/scmintlist0",
            ),
            t(
                "http://example.org/scmintlist0",
                RDF_FIRST,
                "http://example.org/C1",
            ),
            t(
                "http://example.org/scmintlist0",
                RDF_REST,
                "http://example.org/scmintlist1",
            ),
            t(
                "http://example.org/scmintlist1",
                RDF_FIRST,
                "http://example.org/C2",
            ),
            t("http://example.org/scmintlist1", RDF_REST, RDF_NIL),
            t(
                "http://example.org/UniCD",
                OWL_UNIONOF,
                "http://example.org/scmunilist0",
            ),
            t(
                "http://example.org/scmunilist0",
                RDF_FIRST,
                "http://example.org/C1",
            ),
            t(
                "http://example.org/scmunilist0",
                RDF_REST,
                "http://example.org/scmunilist1",
            ),
            t(
                "http://example.org/scmunilist1",
                RDF_FIRST,
                "http://example.org/C2",
            ),
            t("http://example.org/scmunilist1", RDF_REST, RDF_NIL),
        ],
    },
    Fixture {
        name: "dt_literal_typing",
        exercises: &["dt-type1", "dt-type2", "dt-eq"],
        quads: &[
            t_lit(
                "http://example.org/a",
                "http://example.org/age",
                "42",
                XSD_INTEGER,
            ),
            t_lit(
                "http://example.org/b",
                "http://example.org/age",
                "42",
                XSD_INTEGER,
            ),
            t_lit(
                "http://example.org/c",
                "http://example.org/age",
                "43",
                XSD_INTEGER,
            ),
        ],
    },
];

/// Intern one fixture object term into `builder`.
fn intern_object(builder: &mut RdfDatasetBuilder, term: Term) -> TermId {
    match term {
        Term::Iri(iri) => builder.intern_iri(iri),
        Term::Literal { lexical, datatype } => {
            builder.intern_literal(RdfLiteral::typed(lexical, datatype))
        }
    }
}

/// Freeze `fixture`'s quads into a dataset, all in the default graph.
fn build(fixture: &Fixture) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for quad in fixture.quads {
        let s = builder.intern_iri(quad.s);
        let p = builder.intern_iri(quad.p);
        let o = intern_object(&mut builder, quad.o);
        builder.push_quad(s, p, o, None);
    }
    builder.freeze().expect("fixture dataset freezes")
}

/// `fixture`'s canonical N-Quads input — degenerate N-Triples, since every fixture lives
/// entirely in the default graph.
fn input_text(fixture: &Fixture) -> String {
    canonicalize(&build(fixture)).nquads
}

/// The committed corpus file's full text: an N-Triples `#`-comment header (the grammar
/// both N-Triples and `rdflib`'s "nt" parser accept a whole-line `#` comment on) followed
/// by [`input_text`]. `write_owlrl_corpus_inputs` writes exactly this,
/// `corpus_inputs_match_fixtures` compares against exactly this, and
/// `regenerate_owlrl_goldens.py` reads it straight into `rdflib.Graph.parse(..., "nt")`.
fn corpus_file_text(fixture: &Fixture) -> String {
    format!(
        "# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>\n\
         # SPDX-License-Identifier: MIT OR Apache-2.0\n\
         #\n\
         # GENERATED — by crates/entail/tests/owlrl_oracle.rs's write_owlrl_corpus_inputs. Do not\n\
         # hand-edit; see that test's doc comment to regenerate.\n\
         #\n\
         # fixture: {}\n\
         # canonical N-Triples input fed to the owlrl reference engine.\n\
         {}",
        fixture.name,
        input_text(fixture)
    )
}

/// PurRDF's `OWL-RL` closure over `fixture`, as a set of canonical N-Triples lines.
fn purrdf_closure_lines(fixture: &Fixture) -> BTreeSet<String> {
    let ds = build(fixture);
    let (closed, _report) = materialize(&ds, Materialization::OwlRl)
        .unwrap_or_else(|e| panic!("{}: OWL-RL materialization refused: {e}", fixture.name));
    canonicalize(&closed)
        .nquads
        .lines()
        .map(ToOwned::to_owned)
        .collect()
}

// ── Corpus paths ────────────────────────────────────────────────────────────────

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("owlrl-corpus")
}

fn corpus_path(name: &str) -> PathBuf {
    corpus_dir().join(format!("{name}.nt"))
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("owlrl-goldens")
}

fn golden_path(name: &str) -> PathBuf {
    golden_dir().join(format!("{name}.owlrl.golden"))
}

/// Maintainer-only, step 1 of 2: write every fixture's canonical N-Triples input to
/// `tests/owlrl-corpus/`.
///
/// `#[ignore]`d for the same reason `oracle.rs`'s `regenerate_goldens` is: a normal
/// `cargo test` run must only ever compare committed bytes, never rewrite them. Step 2 —
/// running `owlrl` itself over the freshly-written inputs — is
/// `regenerate_owlrl_goldens.py`; see the [module docs](self).
#[test]
#[ignore = "maintainer-only: rewrites the committed corpus inputs; run deliberately"]
fn write_owlrl_corpus_inputs() {
    std::fs::create_dir_all(corpus_dir()).expect("create the corpus directory");
    for fixture in CORPUS {
        std::fs::write(corpus_path(fixture.name), corpus_file_text(fixture))
            .expect("write the input");
    }
}

/// Every golden's embedded `--- input ---` block equals the corpus file it names.
///
/// The golden records the input `owlrl` was actually run over, and that record is the only
/// thing tying its closure to today's fixture. `corpus_inputs_match_fixtures` proves the
/// `.nt` file matches the [`CORPUS`] table; this proves the GOLDEN matches the `.nt` file.
/// Without it, regenerating the corpus without re-running the Python script leaves a
/// closure of a superseded ontology, and the divergence set becomes a diff between two
/// different questions — with the evidence of that sitting unread inside the golden.
#[test]
fn goldens_embed_the_input_they_were_generated_from() {
    let mut stale = Vec::new();
    for fixture in CORPUS {
        let golden = std::fs::read_to_string(golden_path(fixture.name))
            .unwrap_or_else(|e| panic!("{}: read golden: {e}", fixture.name));
        let embedded: String = golden
            .lines()
            .skip_while(|l| *l != "--- input ---")
            .skip(1)
            .take_while(|l| *l != "--- owlrl closure ---")
            .map(|l| format!("{l}\n"))
            .collect();
        assert!(
            !embedded.is_empty(),
            "{}: the golden has no `--- input ---` block, so nothing ties its closure to \
             the committed input",
            fixture.name
        );
        let committed = std::fs::read_to_string(corpus_path(fixture.name))
            .unwrap_or_else(|e| panic!("{}: read corpus input: {e}", fixture.name));
        if embedded.trim_end() != committed.trim_end() {
            stale.push(fixture.name);
        }
    }
    assert!(
        stale.is_empty(),
        "{} golden(s) record an input that is no longer the committed one, so their \
         closures answer a superseded question: {}\nRegenerate with \
         `uv run --script crates/entail/tests/regenerate_owlrl_goldens.py`.",
        stale.len(),
        stale.join(", ")
    );
}

/// Every committed corpus input equals what the fixture table renders today.
///
/// Catches the failure mode `oracle.rs` guards against for its own goldens: editing a
/// `Quad` in [`CORPUS`] without regenerating leaves a stale, unreachable input on disk
/// that nothing else would notice drifted.
#[test]
fn corpus_inputs_match_fixtures() {
    let mut mismatched = Vec::new();
    for fixture in CORPUS {
        let path = corpus_path(fixture.name);
        match std::fs::read_to_string(&path) {
            Ok(committed) if committed == corpus_file_text(fixture) => {}
            Ok(_) => mismatched.push(format!("{}: committed input is stale", fixture.name)),
            Err(_) => mismatched.push(format!(
                "{}: no committed input at {}",
                fixture.name,
                path.display()
            )),
        }
    }
    assert!(
        mismatched.is_empty(),
        "corpus/fixture drift for {} fixture(s):\n{}\n\nRegenerate with:\n  cargo test -p \
         purrdf-entail --test owlrl_oracle -- --ignored --exact write_owlrl_corpus_inputs",
        mismatched.len(),
        mismatched.join("\n")
    );
}

/// The corpus directory holds exactly [`CORPUS`] and nothing else.
#[test]
fn corpus_directory_is_exactly_the_fixture_table() {
    let expected: BTreeSet<String> = CORPUS.iter().map(|f| format!("{}.nt", f.name)).collect();
    let found: BTreeSet<String> = std::fs::read_dir(corpus_dir())
        .expect("read the corpus directory")
        .map(|entry| {
            entry
                .expect("read dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(
        found, expected,
        "tests/owlrl-corpus/ holds a file CORPUS does not name, or vice versa"
    );
}

/// The golden directory holds exactly [`CORPUS`] and nothing else.
#[test]
fn golden_directory_is_exactly_the_fixture_table() {
    let expected: BTreeSet<String> = CORPUS
        .iter()
        .map(|f| format!("{}.owlrl.golden", f.name))
        .collect();
    let found: BTreeSet<String> = std::fs::read_dir(golden_dir())
        .expect("read the golden directory")
        .map(|entry| {
            entry
                .expect("read dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(
        found, expected,
        "tests/owlrl-goldens/ holds a file CORPUS does not name, or vice versa"
    );
}

/// Read a committed `owlrl` golden, returning its frozen closure as a set of N-Triples
/// lines. The header is every line up to and including the `--- owlrl closure ---`
/// banner; everything after it is one triple per line.
fn read_golden_closure(name: &str) -> BTreeSet<String> {
    let path = golden_path(name);
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut lines = text.lines();
    for line in lines.by_ref() {
        if line == "--- owlrl closure ---" {
            break;
        }
    }
    lines
        .map(ToOwned::to_owned)
        .filter(|l| !l.is_empty())
        .collect()
}

// ── Divergence classification ────────────────────────────────────────────────────

/// The four datatypes `owlrl`'s own `OWL_RL_Datatypes` list includes beyond the 32 OWL 2
/// Profiles §4.2.1 names as supported in OWL 2 RL — see
/// `crates/entail/src/calculus/dt.rs`'s `SUPPORTED_DATATYPES` for that list, and the
/// `dt-type1-datatype-map-superset` ledger entry for the citation.
const OWLRL_EXTRA_DATATYPES: [&str; 4] = [
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#HTML",
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString",
    "http://www.w3.org/2001/XMLSchema#date",
    "http://www.w3.org/2001/XMLSchema#time",
];

/// The header of the committed divergence artifact, so the file explains itself.
const DIFF_HEADER: &str = "\
# Every triple on which PurRDF's OWL-RL closure and the `owlrl` reference engine differ,
# over crates/entail/tests/owlrl-corpus/, as `<fixture> <direction> <triple>`.
#
# This is the SET the gate asserts, and it is asserted in both directions: a newly
# divergent triple fails, and a divergence that stops occurring fails too. The typed
# ledger beside it (owlrl-divergences.toml) says which CATEGORY each of these belongs to
# and why PurRDF's answer is the correct one; a category alone cannot do this job, because
# `classify_divergence` recognizes a shape and would absorb any triple that merely looks
# like a known divergence.
#
# Regenerate with PURRDF_WRITE_OWLRL_DIFF=1 after confirming every change is intended.
";

/// The committed divergence artifact's path.
fn diff_artifact_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("owlrl-divergence-triples.txt")
}

/// The recorded divergence set, comment and blank lines skipped.
fn read_expected_diff() -> BTreeSet<String> {
    let path = diff_artifact_path();
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}. It records the exact set of triples the two engines \
             differ on; regenerate with PURRDF_WRITE_OWLRL_DIFF=1.",
            path.display()
        )
    });
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

/// Classify one line present in exactly one engine's closure into a ledger kind id, or
/// `None` if it matches no known, explained divergence.
///
/// A line is standard canonical-N-Triples: `<s> <p> <o-or-literal> .`. Matching is
/// substring-based on purpose rather than a full N-Triples parse — every kind this
/// classifier looks for is identified by an IRI or a syntactic marker that cannot occur
/// anywhere else in this corpus's closures, and a full term parser would be more apparatus
/// than four fixed, well-understood patterns need.
fn classify_divergence(line: &str) -> Option<&'static str> {
    // The literal's own lexical/datatype quoting is `"lexical"^^<datatype>`; a line whose
    // FIRST character is `"` has a literal in subject position — generalized RDF, valid
    // for `owlrl` (rdflib places no restriction on subject shape) but withheld by
    // `materialize` at the `Construct::GeneralizedRdf` boundary. `dt-eq` is `sameAs`;
    // anything else with a literal subject is `dt-type2`. `dt-diff` (`differentFrom`)
    // would classify the same way, but no fixture in this corpus reaches it — owlrl's own
    // source skips the literal-value comparison dt-diff needs — so it has no ledger entry;
    // the branch is kept so a future fixture that DOES reach it fails loudly as an
    // unlisted kind rather than silently falling through to `None` ("unexplained").
    if line.starts_with('"') {
        if line.contains(" <http://www.w3.org/2002/07/owl#differentFrom> ") {
            return Some("dt-diff-generalized-rdf");
        }
        if line.contains(" <http://www.w3.org/2002/07/owl#sameAs> ") {
            return Some("dt-eq-generalized-rdf");
        }
        if line.contains(" <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ") {
            return Some("dt-type2-generalized-rdf");
        }
        return None;
    }
    // A triple naming one of the four extra datatypes as its subject (typed
    // `rdfs:Datatype`, or reflexively `sameAs` itself) is the datatype-map superset.
    if OWLRL_EXTRA_DATATYPES
        .iter()
        .any(|dt| line.starts_with(&format!("<{dt}> ")))
    {
        return Some("dt-type1-datatype-map-superset");
    }
    // The one direction PurRDF over-asserts relative to `owlrl`: the symmetric
    // `differentFrom` extension.
    if line.contains(" <http://www.w3.org/2002/07/owl#differentFrom> ") {
        return Some("ext-eq-diff-sym");
    }
    // `owlrl`'s `OWLRL.py` hard-codes an "optimize out the trivial identity case" skip in
    // its `scm-sco`/`scm-spo` handlers, on the stated assumption that a reflexive
    // `?c rdfs:subClassOf ?c` (or `?p rdfs:subPropertyOf ?p`) is "set elsewhere already" —
    // true only when `scm-cls` (or `scm-op`/`scm-dp`) already fired, which needs an
    // explicit `rdf:type owl:Class` (or `owl:ObjectProperty`/`owl:DatatypeProperty`) this
    // corpus's `cax_subclass_and_equivalent` and `prp_subproperty_and_equivalent` fixtures
    // deliberately omit. Neither OWL 2 Profiles §4.3 Table 9's `scm-sco` nor `scm-spo`
    // carries that premise, so the reflexive conclusion IS licensed here, and the
    // downstream `scm-eqc2`/`scm-eqp2` reflexive `equivalentClass`/`equivalentProperty`
    // that never gets a chance to fire in `owlrl` (because the triple it reads is never
    // asserted) follows from the same root cause.
    if let Some((s, p, o)) = parse_iri_triple(line)
        && s == o
    {
        match p {
            "http://www.w3.org/2000/01/rdf-schema#subClassOf" => {
                return Some("scm-sco-reflexive-through-equivalence");
            }
            "http://www.w3.org/2002/07/owl#equivalentClass" => {
                return Some("scm-eqc2-reflexive-through-equivalence");
            }
            "http://www.w3.org/2000/01/rdf-schema#subPropertyOf" => {
                return Some("scm-spo-reflexive-through-equivalence");
            }
            "http://www.w3.org/2002/07/owl#equivalentProperty" => {
                return Some("scm-eqp2-reflexive-through-equivalence");
            }
            _ => {}
        }
    }
    None
}

/// Parse a canonical `<s> <p> <o> .` line of three IRIs, returning the bare IRI text of
/// each (no angle brackets). `None` for anything else (a literal object, most triples in
/// this corpus) — callers that reach here have already excluded the literal-subject case.
fn parse_iri_triple(line: &str) -> Option<(&str, &str, &str)> {
    let rest = line.strip_prefix('<')?;
    let (s, rest) = rest.split_once("> <")?;
    let (p, rest) = rest.split_once("> <")?;
    let o = rest.strip_suffix("> .")?;
    Some((s, p, o))
}

// ── The divergence ledger ────────────────────────────────────────────────────────

/// One entry of `owlrl-divergences.toml`: a named, explained way the two engines are
/// allowed to differ.
#[derive(Debug, Clone)]
struct Divergence {
    /// The kind id [`classify_divergence`] returns.
    id: String,
    /// The `RuleId` responsible for this side of the divergence.
    rule_id: String,
    /// The specification clause that makes PurRDF's answer the correct one.
    citation: String,
    /// Free prose: what each engine concludes and why they differ.
    description: String,
}

/// The path to the committed ledger.
fn ledger_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("owlrl-divergences.toml")
}

/// Strip one layer of `"…"` quoting from a TOML scalar string value. This parser accepts
/// exactly the single-line, unescaped double-quoted strings this ledger is written with —
/// see the note on [`load_ledger`] for why a full TOML parser is not pulled in for this.
fn unquote(value: &str) -> String {
    let value = value.trim();
    let stripped = value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or_else(|| panic!("expected a quoted TOML string, got: {value}"));
    stripped.to_owned()
}

/// Parse `owlrl-divergences.toml` into [`Divergence`] entries.
///
/// Hand-rolled rather than a `toml` crate dependency: this file is the ledger's only
/// reader, its own grammar (repeated `[[divergence]]` tables of flat, single-line, quoted
/// string keys) is controlled by this same test rather than authored by a third party,
/// and the workspace's other Rust-side conformance ledger
/// (`crates/sparql-conformance/src/xfail.rs`) is a `const` array for exactly this reason —
/// a general parser earns its keep on a format this crate does not also write.
fn load_ledger() -> Vec<Divergence> {
    let path = ledger_path();
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));

    let mut entries = Vec::new();
    let mut id = None;
    let mut rule_id = None;
    let mut citation = None;
    let mut description = None;

    fn flush(
        entries: &mut Vec<Divergence>,
        id: &mut Option<String>,
        rule_id: &mut Option<String>,
        citation: &mut Option<String>,
        description: &mut Option<String>,
    ) {
        if let Some(id) = id.take() {
            entries.push(Divergence {
                id,
                rule_id: rule_id
                    .take()
                    .unwrap_or_else(|| panic!("ledger entry missing rule_id")),
                citation: citation
                    .take()
                    .unwrap_or_else(|| panic!("ledger entry missing citation")),
                description: description
                    .take()
                    .unwrap_or_else(|| panic!("ledger entry missing description")),
            });
        }
    }

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[[divergence]]" {
            flush(
                &mut entries,
                &mut id,
                &mut rule_id,
                &mut citation,
                &mut description,
            );
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            panic!("owlrl-divergences.toml: unparseable line: {line}");
        };
        match key.trim() {
            "id" => id = Some(unquote(value)),
            "rule_id" => rule_id = Some(unquote(value)),
            "citation" => citation = Some(unquote(value)),
            "description" => description = Some(unquote(value)),
            other => panic!("owlrl-divergences.toml: unknown key: {other}"),
        }
    }
    flush(
        &mut entries,
        &mut id,
        &mut rule_id,
        &mut citation,
        &mut description,
    );
    entries
}

// ── The oracle gate ─────────────────────────────────────────────────────────────

/// THE ORACLE. For every fixture, PurRDF's live `OWL-RL` closure and `owlrl`'s frozen
/// reference closure differ by nothing outside the committed ledger — and the ledger's
/// full set of kinds is reached at least once somewhere in the corpus.
#[test]
fn goldens_match_owlrl_reference() {
    let mut observed_kinds: BTreeSet<&'static str> = BTreeSet::new();
    let mut unexplained: Vec<String> = Vec::new();

    // Every divergent triple, tagged by fixture and direction, pinned against a committed
    // artifact below. The KIND set alone is not enough: `classify_divergence` recognizes a
    // SHAPE, so a triple that merely looks like a known divergence is absorbed whether or
    // not it was ever expected — a shared triple deleted from a golden, or a fabricated one
    // added to it, both classify and both used to pass. What a reader wants guaranteed is
    // that the two engines differ on exactly these triples.
    let mut observed_diff: BTreeSet<String> = BTreeSet::new();

    for fixture in CORPUS {
        let ours = purrdf_closure_lines(fixture);
        let theirs = read_golden_closure(fixture.name);

        for only_ours in ours.difference(&theirs) {
            observed_diff.insert(format!("{} purrdf-only {only_ours}", fixture.name));
            match classify_divergence(only_ours) {
                Some(kind) => {
                    observed_kinds.insert(kind);
                }
                None => unexplained.push(format!(
                    "{}: PurRDF-only, unexplained: {only_ours}",
                    fixture.name
                )),
            }
        }
        for only_theirs in theirs.difference(&ours) {
            observed_diff.insert(format!("{} owlrl-only {only_theirs}", fixture.name));
            match classify_divergence(only_theirs) {
                Some(kind) => {
                    observed_kinds.insert(kind);
                }
                None => unexplained.push(format!(
                    "{}: owlrl-only, unexplained: {only_theirs}",
                    fixture.name
                )),
            }
        }
    }

    if std::env::var_os("PURRDF_WRITE_OWLRL_DIFF").is_some() {
        let mut out = String::from(DIFF_HEADER);
        for line in &observed_diff {
            out.push_str(line);
            out.push('\n');
        }
        std::fs::write(diff_artifact_path(), out).expect("write the divergence artifact");
    }

    let expected_diff = read_expected_diff();
    let appeared: Vec<&String> = observed_diff.difference(&expected_diff).collect();
    let vanished: Vec<&String> = expected_diff.difference(&observed_diff).collect();
    assert!(
        appeared.is_empty() && vanished.is_empty(),
        "the two engines no longer differ on exactly the recorded triples.\n\
         {} newly divergent (PurRDF changed, or a golden was edited):\n{}\n\
         {} no longer divergent (a divergence closed — record the gain):\n{}\n\
         Regenerate with PURRDF_WRITE_OWLRL_DIFF=1 after confirming each change is intended.",
        appeared.len(),
        appeared
            .iter()
            .map(|l| format!("  + {l}"))
            .collect::<Vec<String>>()
            .join("\n"),
        vanished.len(),
        vanished
            .iter()
            .map(|l| format!("  - {l}"))
            .collect::<Vec<String>>()
            .join("\n"),
    );

    assert!(
        unexplained.is_empty(),
        "{} unexplained divergence(s) between PurRDF's OWL-RL closure and the owlrl \
         reference — investigate each as a potential bug rather than ledgering it:\n{}",
        unexplained.len(),
        unexplained.join("\n")
    );

    let ledger = load_ledger();
    let ledger_ids: BTreeSet<String> = ledger.iter().map(|d| d.id.clone()).collect();
    assert_eq!(
        observed_kinds
            .iter()
            .map(|s| (*s).to_owned())
            .collect::<BTreeSet<String>>(),
        ledger_ids,
        "the corpus's observed divergence kinds must equal the ledger exactly — a kind \
         present in one but not the other means either an unledgered divergence appeared \
         or a ledgered one no longer occurs (XPASS discipline)"
    );
}

/// The ledger's size is pinned to a literal constant, so growing or shrinking the set of
/// allowed divergences is a reviewed edit rather than something that creeps in unnoticed.
const EXPECTED_DIVERGENCE_COUNT: usize = 8;

#[test]
fn ledger_entry_count_is_pinned() {
    assert_eq!(
        load_ledger().len(),
        EXPECTED_DIVERGENCE_COUNT,
        "crates/entail/tests/owlrl-divergences.toml gained or lost an entry — update \
         EXPECTED_DIVERGENCE_COUNT deliberately if the change is reviewed"
    );
}

/// Every ledger entry names a `RuleId` this crate actually declares.
#[test]
fn ledger_entries_name_real_rules() {
    for entry in load_ledger() {
        purrdf_entail::RuleId::from_str(&entry.rule_id).unwrap_or_else(|_| {
            panic!(
                "owlrl-divergences.toml entry `{}` names `{}`, which is not a RuleId this \
                 crate declares",
                entry.id, entry.rule_id
            )
        });
    }
}

/// Every ledger entry carries a non-empty citation and description — a `rule_id` alone
/// names WHICH rule, not why PurRDF's answer is the correct one.
#[test]
fn ledger_entries_carry_a_citation_and_description() {
    for entry in load_ledger() {
        assert!(
            !entry.citation.trim().is_empty(),
            "{}: empty citation",
            entry.id
        );
        assert!(
            !entry.description.trim().is_empty(),
            "{}: empty description",
            entry.id
        );
    }
}

/// Every fixture's `exercises` list names real `RuleId`s — documentation for the reader,
/// checked so a typo or a renamed rule cannot silently rot the doc comment.
#[test]
fn fixture_exercise_lists_name_real_rules() {
    for fixture in CORPUS {
        for spelling in fixture.exercises {
            purrdf_entail::RuleId::from_str(spelling).unwrap_or_else(|_| {
                panic!(
                    "{}: `{spelling}` is not a RuleId this crate declares",
                    fixture.name
                )
            });
        }
    }
}

/// Rendering the input and the closure is a pure function of the fixture table: run
/// twice in one process, they agree byte for byte.
#[test]
fn rendering_is_byte_stable_within_a_run() {
    for fixture in CORPUS {
        assert_eq!(
            input_text(fixture),
            input_text(fixture),
            "{}: input",
            fixture.name
        );
        assert_eq!(
            purrdf_closure_lines(fixture),
            purrdf_closure_lines(fixture),
            "{}: closure",
            fixture.name
        );
    }
}
