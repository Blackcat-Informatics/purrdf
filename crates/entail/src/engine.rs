// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Running the declared calculus: [`calculus_program`](crate::calculus_program) evaluated
//! by `purrdf-datalog`.
//!
//! # The declaration IS the implementation
//!
//! [`crate::calculus`] renders every rule this crate fires as a [`DlClause`]. This module
//! does not re-state a single one of them: it seeds a [`RelationStore`] from the dataset's
//! default graph, hands `purrdf-datalog` the clauses the calculus declares, and reads the
//! answer back. There is therefore one statement of the calculus, and the digest a
//! [`ReasoningReport`](crate::ReasoningReport) carries is the digest of the very clauses
//! that ran.
//!
//! # The bridge is a lexical surface, and it is injective
//!
//! `purrdf-datalog` interns a term by its **lexical surface** (`&str`), while the RDF 1.2
//! IR identifies a term by a [`TermValue`]. [`surface_of`] is the one translation, and
//! [`Terms`] is the inverse: every surface that enters the store is recorded against the
//! value it came from, so materializing an answer never has to *parse* a surface back.
//!
//! Two properties make that sound, and both are load-bearing:
//!
//! * [`surface_of`] is INJECTIVE — distinct [`TermValue`]s render to distinct surfaces —
//!   so two terms can never collide into one store term. It is the repository's own
//!   canonical N-Quads term spelling (`purrdf_core::canonicalize`'s), with a blank node
//!   qualified by its scope because a scope is part of a blank node's identity (C0.2)
//!   and is not recoverable from a canonical label.
//! * The evaluator MINTS NO TERMS. Every clause is range-restricted (`compile` refuses
//!   one that is not) and none is existential, so every term in the answer is either a
//!   term this module seeded or a constant of the program itself. [`Terms`] records both
//!   before evaluation starts, which is what makes its lookup total.
//!
//! An IRI's surface is `<iri>`, which is exactly what
//! [`ClauseTerm::iri`](purrdf_datalog::clause::ClauseTerm::iri) renders to — so a clause
//! constant and a dataset term compare as the same bytes, without a second convention.
//!
//! # What the RDF 1.2 IR cannot hold
//!
//! The evaluator's term space is wider than RDF 1.2's: nothing there stops a literal
//! reaching subject position or a blank node reaching predicate position, because a
//! [`Fact`](purrdf_datalog::store::Fact) is four terms with no positional restriction.
//! Those conclusions are GENERALIZED-RDF triples, and the [`RdfDataset`] IR cannot
//! represent them. [`close`] therefore drops such a conclusion at the materialization
//! boundary rather than fabricating a term for it, counts the drop, and the count is what
//! raises the [`Construct::GeneralizedRdf`](crate::Construct::GeneralizedRdf) boundary.
//!
//! The drop is at the BOUNDARY, not in the calculus: the generalized fact stays in the
//! store and may still serve as a premise, so a conclusion that is itself representable is
//! not withheld merely because its derivation passed through one that was not.
//!
//! # Determinism
//!
//! Derivations arrive in `purrdf-datalog`'s total order — lexical by `(fact, rule,
//! sources)` — and are emitted in that order, so the derived quads reach the builder in a
//! sequence that is a function of the fact set alone. [`Terms`] is a `BTreeMap`, so no
//! hash iteration reaches anything either.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};
use purrdf_datalog::clause::{ClauseTerm, DlClause};
use purrdf_datalog::seminaive::{compile, evaluate};
use purrdf_datalog::store::RelationStore;

use crate::axioms::axioms_for;
use crate::calculus::program_with_attribution;
use crate::interner::intern_into;
use crate::report::RunStats;
use crate::vocab::XSD_STRING;
use crate::{EntailError, Regime};

/// A faithful copy of `ds` (the identity closure for `Simple`).
pub(crate) fn copy_of(ds: &RdfDataset) -> Result<Arc<RdfDataset>, EntailError> {
    let mut b = RdfDatasetBuilder::new();
    b.push_dataset(ds);
    b.freeze().map_err(|e| EntailError::Build(e.to_string()))
}

/// Close `ds` under `regime`'s declared calculus and emit `original + inferred`.
///
/// # The axiomatic triples are seeded, not concluded
///
/// [`crate::axioms`]'s finite table is inserted into the fact store beside `ds`'s own
/// quads, because that is what the definition of RDFS entailment says it is: a premise
/// every interpretation satisfies, not something a rule derives. Its CONSEQUENCES are
/// derivations like any other and are credited to the rule that drew them —
/// `:a rdfs:subClassOf :b` reaches `:a rdfs:subClassOf :a` through `rdfs3` and then
/// `rdfs10`, and both firings appear in the report.
///
/// The axioms themselves are therefore NOT emitted: they are neither in `ds` nor
/// derivations, and inventing a rule id to credit them to would put a firing in the tally
/// that no rule of the specification's tables licenses. A closure that omits an entailed
/// triple is an incompleteness, so it is reported —
/// [`Construct::AxiomaticTriples`](crate::Construct::AxiomaticTriples) says both this and
/// the unbounded `rdf:_n` family in one boundary.
///
/// The default graph alone supplies premises and receives conclusions — every atom of
/// every declared clause names
/// [`ClauseTerm::DefaultGraph`](purrdf_datalog::clause::ClauseTerm::DefaultGraph), which
/// `the_declared_programs_read_and_write_the_default_graph_only` asserts — so quads in a
/// named graph are carried through untouched and the run reports the
/// [`Construct::NamedGraph`](crate::Construct::NamedGraph) boundary.
pub(crate) fn close(
    ds: &RdfDataset,
    regime: Regime,
) -> Result<(Arc<RdfDataset>, RunStats), EntailError> {
    let (program, attribution) = program_with_attribution(regime);

    let mut terms = Terms::default();
    terms.record_program(&program);
    let mut edb = RelationStore::new();
    // The axiomatic triples are PREMISES, not conclusions: `S RDFS entails E` is defined
    // over the interpretations satisfying S *and* the axioms, and no rule of §9.2.1
    // concludes one. Seeding them beside the dataset's own quads is that definition,
    // written down. See `crate::axioms` for the table and for which lanes assert it.
    for &(subject, predicate, object) in axioms_for(regime) {
        let subject = terms.record(&TermValue::iri(subject));
        let predicate = terms.record(&TermValue::iri(predicate));
        let object = terms.record(&TermValue::iri(object));
        let _ = edb.insert(&subject, &predicate, &object, RelationStore::DEFAULT_GRAPH);
    }
    for quad in ds.quads() {
        if quad.g.is_some() {
            continue; // entailment operates over the default graph
        }
        let subject = terms.record(&ds.term_value(quad.s));
        let predicate = terms.record(&ds.term_value(quad.p));
        let object = terms.record(&ds.term_value(quad.o));
        let _ = edb.insert(&subject, &predicate, &object, RelationStore::DEFAULT_GRAPH);
    }

    let executable = compile(program).map_err(EntailError::Evaluate)?;
    let evaluation = evaluate(&executable, edb).map_err(EntailError::Evaluate)?;

    // The budget is the evaluator's own measurement, not a second tally kept alongside it.
    let mut stats = RunStats::of_budget(evaluation.budget());
    let mut b = RdfDatasetBuilder::new();
    b.push_dataset(ds);
    for derivation in evaluation.derivations() {
        let fact = derivation.fact();
        let subject = terms.value(&fact.subject);
        let predicate = terms.value(&fact.predicate);
        if !admits_subject(subject) || !admits_predicate(predicate) {
            stats.drop_generalized();
            continue;
        }
        // Credited only once the conclusion is known to be representable, so the per-rule
        // counts sum to exactly the inferred triples a caller can see.
        stats.commit(attribution[derivation.rule()]);
        let s = intern_into(&mut b, subject);
        let p = intern_into(&mut b, predicate);
        let o = intern_into(&mut b, terms.value(&fact.object));
        b.push_quad(s, p, o, None);
    }
    let dataset = b.freeze().map_err(|e| EntailError::Build(e.to_string()))?;
    Ok((dataset, stats))
}

/// Whether `value` may occupy a triple SUBJECT position in RDF 1.2 — an IRI or a blank
/// node, never a literal and never a triple term.
fn admits_subject(value: &TermValue) -> bool {
    matches!(value, TermValue::Iri(_) | TermValue::Blank { .. })
}

/// Whether `value` may occupy a triple PREDICATE position in RDF 1.2 — an IRI, and
/// nothing else.
///
/// Checked as well as the subject because a rule may conclude into predicate position
/// too: `rdfs7` / `prp-spo1` writes the OBJECT of a `rdfs:subPropertyOf` triple there, and
/// `prp-inv1` / `prp-inv2` write the object of an `owl:inverseOf` triple. Neither
/// specification requires that object to be an IRI, so a graph that declares
/// `p rdfs:subPropertyOf "cat"` licenses a conclusion the IR cannot hold, in exactly the
/// way a literal subject does.
fn admits_predicate(value: &TermValue) -> bool {
    matches!(value, TermValue::Iri(_))
}

/// The surface → value dictionary that lets an answer be read back as RDF 1.2 terms.
///
/// Every surface the store can ever hold is recorded here before evaluation begins: the
/// constants of the program ([`Self::record_program`]) and the terms of the seeded facts
/// ([`Self::record`]). The evaluator mints no terms, so those two sets are exhaustive and
/// [`Self::value`] is total — see the [module docs](self).
#[derive(Debug, Default)]
struct Terms {
    /// Surfaces to the values they were rendered from, in lexical surface order.
    by_surface: BTreeMap<String, TermValue>,
}

impl Terms {
    /// Record `value` and return its surface.
    fn record(&mut self, value: &TermValue) -> String {
        let surface = surface_of(value);
        if !self.by_surface.contains_key(&surface) {
            self.by_surface.insert(surface.clone(), value.clone());
        }
        surface
    }

    /// Record every constant term of `program`.
    ///
    /// Every constant this crate's calculus names is an IRI — PurRDF mints no vocabulary,
    /// and the rules quantify over data rather than comparing it against literals — which
    /// `every_clause_constant_is_an_iri` asserts over every declared program. A literal
    /// constant is therefore NOT handled here, and deliberately so: a
    /// [`ClauseTerm::Literal`] carries an already-rendered surface with no structure to
    /// recover a [`TermValue`] from, so guessing one would put a term in the dictionary
    /// that [`surface_of`] does not agree with. The rule that introduces the first literal
    /// constant has to give this module a way to read it back, and the test is what makes
    /// that a failure rather than a silent wrong term.
    fn record_program(&mut self, program: &[DlClause]) {
        for clause in program {
            for atom in clause.body().iter().chain(clause.head_atoms()) {
                for term in atom.terms() {
                    if let ClauseTerm::Iri(iri) = term {
                        let _ = self.record(&TermValue::iri(iri.clone()));
                    }
                }
            }
        }
    }

    /// The value behind a surface the store produced.
    ///
    /// # Panics
    ///
    /// Panics if the surface was never recorded. That is unreachable rather than merely
    /// unlikely: `compile` refuses a clause whose head carries a variable no positive body
    /// atom binds, and no declared clause is existential, so every term of every derived
    /// fact came from a seeded fact or from a program constant — and both were recorded
    /// before evaluation started.
    fn value(&self, surface: &str) -> &TermValue {
        self.by_surface.get(surface).unwrap_or_else(|| {
            panic!("the evaluator mints no terms, so {surface} must have been recorded")
        })
    }
}

/// The lexical surface `purrdf-datalog` interns `value` under.
///
/// This is the repository's canonical N-Quads term spelling — the same bytes
/// `purrdf_core::canonicalize` writes — with one deliberate difference: a blank node is
/// qualified by its scope ordinal, because C0.2 makes the scope part of the node's
/// identity while a canonical label is assigned by the canonicalization algorithm rather
/// than carried by the term.
///
/// # Injectivity
///
/// Distinct [`TermValue`]s render to distinct surfaces, which is what stops two terms
/// collapsing into one store term:
///
/// * the four kinds are told apart by their first byte — `<` for an IRI, `_` for a blank
///   node, `"` for a literal, and `<<(` for a triple term, whose second byte is a `<` no
///   IRI surface can carry because [`write_iri_escaped`] escapes `<` and `>`;
/// * an IRI's surface is its escaped text bracketed once, and the escape is injective;
/// * a blank node's scope is decimal digits terminated by the `.` that no digit can be,
///   and the label is the verbatim remainder;
/// * a literal's lexical form is escaped so it carries no bare `"`, so the closing quote
///   is unambiguous, and what follows is either `@` (a language tag, hence the datatype
///   `rdf:langString` by C0.1) or `^^<` (a datatype IRI) or nothing (`xsd:string`);
/// * a triple term's three components are separated by the spaces its delimiters reserve,
///   and each recurses through the same argument.
fn surface_of(value: &TermValue) -> String {
    let mut out = String::new();
    write_surface(value, &mut out);
    out
}

/// Append [`surface_of`]'s rendering of `value` to `out`.
fn write_surface(value: &TermValue, out: &mut String) {
    match value {
        TermValue::Iri(iri) => {
            out.push('<');
            write_iri_escaped(iri, out);
            out.push('>');
        }
        TermValue::Blank { label, scope } => {
            let _ = write!(out, "_:{}.{label}", scope.ordinal());
        }
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => {
            out.push('"');
            write_literal_escaped(lexical_form, out);
            out.push('"');
            if let Some(language) = language {
                // A language-tagged literal's datatype is `rdf:langString` by C0.1 — the
                // builder re-derives it — so the tag determines it and spelling it out
                // would add bytes that carry no identity.
                out.push('@');
                out.push_str(language);
                if let Some(direction) = direction {
                    out.push_str("--");
                    out.push_str(direction.as_str());
                }
            } else if datatype != XSD_STRING {
                out.push_str("^^<");
                write_iri_escaped(datatype, out);
                out.push('>');
            }
        }
        TermValue::Triple { s, p, o } => {
            out.push_str("<<( ");
            write_surface(s, out);
            out.push(' ');
            write_surface(p, out);
            out.push(' ');
            write_surface(o, out);
            out.push_str(" )>>");
        }
    }
}

/// Escape an IRI for a `<…>` surface, matching canonical N-Quads.
///
/// Every character the IRIREF grammar forbids becomes a `\uXXXX` escape, so no IRI's
/// surface can carry a bare `<` or `>` — which is what keeps a bracketed IRI and a
/// `<<( … )>>` triple term apart. A spec `rdf:`/`rdfs:`/`owl:` IRI contains none of them,
/// so a clause constant's surface is its plain bracketed text.
fn write_iri_escaped(iri: &str, out: &mut String) {
    for ch in iri.chars() {
        match ch {
            c if c.is_control() || c == ' ' => write_u_escape(c, out),
            '<' | '>' | '"' | '{' | '}' | '|' | '^' | '`' | '\\' => write_u_escape(ch, out),
            _ => out.push(ch),
        }
    }
}

/// Escape a literal's lexical form for a `"…"` surface, matching canonical N-Quads.
fn write_literal_escaped(value: &str, out: &mut String) {
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => write_u_escape(c, out),
            c => out.push(c),
        }
    }
}

/// Write `\uXXXX` (or `\UXXXXXXXX` beyond the BMP) for `ch`.
fn write_u_escape(ch: char, out: &mut String) {
    let cp = ch as u32;
    if cp <= 0xFFFF {
        let _ = write!(out, "\\u{cp:04X}");
    } else {
        let _ = write!(out, "\\U{cp:08X}");
    }
}

#[cfg(test)]
mod tests {
    use super::{admits_predicate, admits_subject, surface_of};
    use crate::calculus::ALL_REGIMES;
    use crate::calculus_program;
    use crate::vocab::{RDF_TYPE, XSD_STRING};
    use purrdf_core::{BlankScope, RdfTextDirection, TermValue};
    use purrdf_datalog::clause::{ClauseTerm, DlClause};
    use purrdf_datalog::store::RelationStore;
    use std::collections::BTreeSet;

    /// A fixture IRI. PurRDF mints no vocabulary, so every fixture term is `example.org`.
    const EX_S: &str = "http://example.org/s";
    /// A fixture predicate IRI.
    const EX_P: &str = "http://example.org/p";
    /// A fixture object IRI.
    const EX_O: &str = "http://example.org/o";

    /// A triple term over three IRIs, by value.
    fn quoted(s: &str, p: &str, o: &str) -> TermValue {
        TermValue::Triple {
            s: Box::new(TermValue::iri(s)),
            p: Box::new(TermValue::iri(p)),
            o: Box::new(TermValue::iri(o)),
        }
    }

    /// A clause constant IRI renders to the SAME surface the store will hold for the
    /// dataset term of that IRI — the property that lets a rule constant join against
    /// data at all.
    #[test]
    fn a_clause_constant_and_a_dataset_iri_share_one_surface() {
        assert_eq!(
            surface_of(&TermValue::iri(RDF_TYPE)),
            ClauseTerm::iri(RDF_TYPE)
                .surface()
                .expect("a constant has a surface")
        );
    }

    /// The default graph is the empty surface on both sides of the bridge, so a
    /// default-graph atom addresses the partition the seeded quads went into.
    #[test]
    fn the_default_graph_is_the_empty_surface_on_both_sides() {
        assert_eq!(RelationStore::DEFAULT_GRAPH, "");
        assert_eq!(
            ClauseTerm::DefaultGraph.surface().as_deref(),
            Some(RelationStore::DEFAULT_GRAPH)
        );
    }

    /// [`surface_of`] is injective over terms that differ in ANY identity coordinate,
    /// including the ones a careless rendering drops: a blank node's scope, a literal's
    /// datatype, its language tag and its base direction.
    #[test]
    fn distinct_terms_render_to_distinct_surfaces() {
        let terms = [
            TermValue::iri(EX_S),
            TermValue::iri(EX_O),
            // An IRI whose text is the surface of a triple term: the escape is what
            // stops it colliding with one.
            TermValue::iri("<<( a b c )>>"),
            TermValue::Blank {
                label: "b0".to_owned(),
                scope: BlankScope::DEFAULT,
            },
            TermValue::Blank {
                label: "b0".to_owned(),
                scope: BlankScope(7),
            },
            // A label that itself carries the scope separator.
            TermValue::Blank {
                label: "0.b0".to_owned(),
                scope: BlankScope::DEFAULT,
            },
            TermValue::simple_literal("cat"),
            TermValue::simple_literal("cat\"@en"),
            TermValue::typed_literal("cat", "http://example.org/dt"),
            TermValue::lang_literal("cat", "en"),
            TermValue::Literal {
                lexical_form: "cat".to_owned(),
                datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
                language: Some("en".to_owned()),
                direction: Some(RdfTextDirection::Ltr),
            },
            TermValue::Literal {
                lexical_form: "cat".to_owned(),
                datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
                language: Some("en".to_owned()),
                direction: Some(RdfTextDirection::Rtl),
            },
            quoted(EX_S, EX_P, EX_O),
            quoted(EX_O, EX_P, EX_S),
        ];
        let surfaces: BTreeSet<String> = terms.iter().map(surface_of).collect();
        assert_eq!(
            surfaces.len(),
            terms.len(),
            "two distinct terms share a surface: {surfaces:?}"
        );
        // A plain `xsd:string` literal carries no datatype suffix, exactly as canonical
        // N-Quads writes it — so `simple_literal` and an explicit `xsd:string` literal
        // are ONE term, which is what RDF 1.1 says they are.
        assert_eq!(
            surface_of(&TermValue::simple_literal("cat")),
            surface_of(&TermValue::typed_literal("cat", XSD_STRING))
        );
    }

    /// The positional guards admit exactly what RDF 1.2 admits.
    #[test]
    fn the_positional_guards_match_rdf_12() {
        let iri = TermValue::iri(EX_S);
        let blank = TermValue::blank("b0");
        let literal = TermValue::simple_literal("cat");
        let triple = quoted(EX_S, EX_P, EX_O);
        assert!(admits_subject(&iri) && admits_predicate(&iri));
        assert!(admits_subject(&blank), "a blank node is a legal subject");
        assert!(
            !admits_predicate(&blank),
            "a blank predicate is generalized"
        );
        assert!(!admits_subject(&literal) && !admits_predicate(&literal));
        assert!(!admits_subject(&triple) && !admits_predicate(&triple));
    }

    /// EVERY constant of EVERY declared clause is an IRI.
    ///
    /// [`super::Terms::record_program`] records IRI constants and nothing else, because a
    /// [`ClauseTerm::Literal`] is an already-rendered surface with no structure to recover
    /// a [`TermValue`] from. This is the assertion that turns "no rule uses a literal
    /// constant today" from an assumption into a checked fact, so the change that adds one
    /// fails here instead of materializing a term the surface renderer disagrees with.
    #[test]
    fn every_clause_constant_is_an_iri() {
        for regime in ALL_REGIMES {
            for clause in calculus_program(regime) {
                for atom in clause.body().iter().chain(clause.head_atoms()) {
                    for term in atom.terms() {
                        assert!(
                            !matches!(term, ClauseTerm::Literal(_)),
                            "{regime:?}: a clause names the literal constant {term:?}, which \
                             the surface dictionary cannot read back"
                        );
                    }
                }
            }
        }
    }

    /// EVERY atom of EVERY declared clause names the default graph.
    ///
    /// [`super::close`] seeds the default graph alone and emits every conclusion into it;
    /// this is the statement that makes that a faithful evaluation of the program rather
    /// than a silent restriction of it. A rule that later reasons per-graph fails here,
    /// which is the signal to teach the seeding and the emission about graphs.
    #[test]
    fn the_declared_programs_read_and_write_the_default_graph_only() {
        for regime in ALL_REGIMES {
            let program: Vec<DlClause> = calculus_program(regime);
            for clause in &program {
                for atom in clause.body().iter().chain(clause.head_atoms()) {
                    assert!(
                        atom.graph().is_default_graph(),
                        "{regime:?}: an atom names the graph {:?}",
                        atom.graph()
                    );
                }
            }
        }
    }
}
