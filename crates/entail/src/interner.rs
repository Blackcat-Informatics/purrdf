// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Dataset-independent term interner shared by every entailment engine.
//!
//! Reasoning runs over dense `u32` ids rather than [`TermValue`]s so joins, label
//! sets, and adjacency indices stay cheap. Keys are [`TermValue`] — a value type
//! independent of any particular dataset's term table — which is what lets an
//! engine intern terms from the source dataset and re-materialize them into a fresh
//! builder soundly.

use std::collections::{BTreeMap, BTreeSet};
use std::hash::BuildHasher;

use hashbrown::HashTable;

use purrdf_core::{RdfDatasetBuilder, TermFactory, TermId, TermValue};

/// Local `TermValue`→`u32` interner over dataset-independent terms.
#[derive(Default)]
pub(crate) struct Interner {
    index: HashTable<u32>,
    values: Vec<TermValue>,
}

impl Interner {
    pub(crate) fn len(&self) -> usize {
        self.values.len()
    }

    /// Intern `v`, returning its stable dense id (assigned in first-seen order).
    pub(crate) fn intern(&mut self, v: TermValue) -> u32 {
        let hash = purrdf_core::FastHasher::default().hash_one(&v);
        if let Some(&id) = self.index.find(hash, |&id| self.values[id as usize] == v) {
            return id;
        }
        let id = u32::try_from(self.values.len()).expect("term count fits u32");
        self.values.push(v);
        self.index.insert_unique(hash, id, |&id| {
            purrdf_core::FastHasher::default().hash_one(&self.values[id as usize])
        });
        id
    }

    /// Intern an IRI by string.
    pub(crate) fn intern_iri(&mut self, iri: &str) -> u32 {
        self.intern(TermValue::Iri(iri.to_owned()))
    }

    /// The `TermValue` behind an id.
    pub(crate) fn value(&self, id: u32) -> &TermValue {
        &self.values[id as usize]
    }

    /// The id already assigned to `iri`, if it has been interned (lookup only).
    pub(crate) fn id_of_iri(&self, iri: &str) -> Option<u32> {
        // Vocabulary lookup borrows spelling without a temporary owned term.
        self.values.iter().enumerate().find_map(|(index, value)| {
            matches!(value, TermValue::Iri(value) if value == iri)
                .then(|| u32::try_from(index).expect("interned term count fits u32"))
        })
    }

    /// Borrowed term lookup at the sole interner home.
    pub(crate) fn id_of(&self, value: &TermValue) -> Option<u32> {
        let hash = purrdf_core::FastHasher::default().hash_one(value);
        self.index
            .find(hash, |&id| self.values[id as usize] == *value)
            .copied()
    }

    /// Every interned blank node's label, in interning order.
    ///
    /// Read by the DL reasoner's refutation-symbol generator, which must pick a label
    /// prefix no data blank node begins with: a colliding label would alias a scaffolding
    /// witness with a blank node the ontology already constrains, which is an unsoundness
    /// rather than a cosmetic clash. Deciding freshness against the actual term table is
    /// what turns that from a hope about naming conventions into a checked property.
    pub(crate) fn blank_labels(&self) -> impl Iterator<Item = &str> {
        self.values.iter().filter_map(|value| match value {
            TermValue::Blank { label, .. } => Some(label.as_str()),
            _ => None,
        })
    }

    /// Whether `id` may occupy a triple *subject* position (an IRI or blank node —
    /// never a literal or triple term reached by an inverse/range rule).
    pub(crate) fn is_subject(&self, id: u32) -> bool {
        matches!(
            self.values[id as usize],
            TermValue::Iri(_) | TermValue::Blank { .. }
        )
    }

    /// Whether `id` denotes a LITERAL — an element of the DATA domain rather than of the
    /// object domain.
    ///
    /// Total over `u32` rather than indexing: the OWL-Direct tableau's own unit tests build a
    /// knowledge base from bare symbol ids without interning a term for each, and an
    /// abstract-domain symbol the interner never saw is exactly the "not a literal" answer.
    pub(crate) fn is_literal(&self, id: u32) -> bool {
        matches!(
            self.values.get(id as usize),
            Some(TermValue::Literal { .. })
        )
    }
}

/// Intern a [`TermValue`] into `b`, returning its dataset-local id.
///
/// Re-materialization is **structural and total**: every component of the term's identity
/// survives the round trip, so the id this returns denotes the same term the engine
/// interned. That is the whole contract, and each arm below earns it.
///
/// * A literal is rebuilt from all four identity coordinates — lexical form, datatype,
///   language tag, and RDF 1.2 **base direction**. Direction participates in literal
///   identity (`purrdf-core`'s C0.1), so dropping it would silently substitute a
///   *different* literal for the one that went in. [`RdfLiteral`] is a plain struct with
///   public fields, so it is built as a struct literal rather than through one of the
///   `simple`/`typed`/`language_tagged` constructors: every one of those hard-codes
///   `direction: None`, which is exactly the loss this arm exists to prevent. (The
///   builder derives the language datatype from the tag and direction, and lowercases
///   the tag when a language is
///   present, so passing the already-expanded datatype through is a no-op, not a
///   conflict.)
/// * A triple term is rebuilt component by component, at every depth. Its subject,
///   predicate, or object may itself be a triple term, so the reconstruction nests to
///   whatever depth the source term has. Folding one to a stand-in IRI would assert a
///   triple nothing entails — unsound, and strictly worse than deriving nothing.
///
/// Triple terms stay **opaque to the rules**: the chase interns one as a single atomic
/// term and never reasons into it. `rdfs14` / `rdfs14a` DO fire over it — they conclude
/// about a fresh surrogate blank node typed `rdfs:Proposition` — but that surrogate is not
/// an answer a SPARQL entailment regime admits, so it is withheld at the materialization
/// boundary and the quoted triple's own content is still never looked at. Opacity only ever
/// withholds conclusions, and it is reported as a
/// [`Construct::TripleTerm`](crate::Construct::TripleTerm) boundary so a caller is told
/// the closure is incomplete rather than left to assume it is exact. Re-materializing the
/// term faithfully is what keeps the conclusions the rules DO draw around it correct.
///
/// The builder's own value interning ([`TermFactory::intern_value`]), which takes a
/// triple term apart over a work list rather than the machine stack.
pub(crate) fn intern_into(b: &mut RdfDatasetBuilder, v: &TermValue) -> TermId {
    b.intern_value(v)
}

#[cfg(test)]
mod tests {
    use purrdf_core::TermBox;
    use purrdf_core::{BlankScope, RdfTextDirection};

    use super::{Interner, blank_closure, blank_subjects, intern_into};
    use purrdf_core::{RdfDatasetBuilder, TermValue};

    /// The closure follows blank objects through a cycle once each, stops at a named
    /// object, and a named starting term contributes nothing.
    #[test]
    fn blank_closure_follows_blank_objects_once_and_ignores_named_terms() {
        let mut interner = Interner::default();
        let s = interner.intern(TermValue::iri(EX_S));
        let p = interner.intern(TermValue::iri(EX_P));
        let o = interner.intern(TermValue::iri(EX_O));
        let a = interner.intern(TermValue::blank("a"));
        let b = interner.intern(TermValue::blank("b"));
        let triples = [(s, p, a), (a, p, b), (b, p, a), (b, p, o), (o, p, s)];
        let blanks = blank_subjects(&interner, &triples);
        assert_eq!(blanks.len(), 2);
        let mut out = std::collections::BTreeSet::new();
        blank_closure(&interner, &triples, &blanks, a, &mut out);
        assert_eq!(out.into_iter().collect::<Vec<_>>(), [1, 2, 3]);
        let mut named = std::collections::BTreeSet::new();
        blank_closure(&interner, &triples, &blanks, s, &mut named);
        assert!(named.is_empty());
    }

    /// A fixture IRI. PurRDF mints no vocabulary, so every fixture term is `example.org`.
    const EX_S: &str = "http://example.org/s";
    /// A fixture predicate IRI.
    const EX_P: &str = "http://example.org/p";
    /// A fixture object IRI.
    const EX_O: &str = "http://example.org/o";
    /// The predicate the round-trip fixtures hang their object term under.
    const EX_HOLDS: &str = "http://example.org/holds";
    /// RDF 1.2 datatype for language-tagged literals with a base direction.
    use purrdf_iri::vocab::rdf::DIR_LANG_STRING as RDF_DIR_LANG_STRING;

    /// A triple term over three IRIs, by value.
    fn quoted(s: &str, p: &str, o: &str) -> TermValue {
        TermValue::Triple {
            s: TermBox::new(TermValue::iri(s)),
            p: TermBox::new(TermValue::iri(p)),
            o: TermBox::new(TermValue::iri(o)),
        }
    }

    /// A language-tagged literal carrying `direction`.
    fn directional(lexical: &str, language: &str, direction: RdfTextDirection) -> TermValue {
        TermValue::Literal {
            lexical_form: lexical.to_owned(),
            datatype: RDF_DIR_LANG_STRING.to_owned(),
            language: Some(language.to_owned()),
            direction: Some(direction),
        }
    }

    /// Push `value` through [`intern_into`] into a fresh dataset, freeze it, and read the
    /// object term back out — the round trip the chase performs when it re-materializes a
    /// derived triple.
    fn round_trip(value: &TermValue) -> TermValue {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(EX_S);
        let p = b.intern_iri(EX_HOLDS);
        let o = intern_into(&mut b, value);
        b.push_quad(s, p, o, None);
        let ds = b.freeze().expect("the round-trip dataset freezes");
        let quad = ds.quads().next().expect("one quad");
        ds.term_value(quad.o)
    }

    /// Every term kind survives re-materialization unchanged — including a triple term,
    /// which used to be folded to a stand-in IRI.
    #[test]
    fn intern_into_round_trips_every_term_kind() {
        for value in [
            TermValue::iri(EX_O),
            TermValue::Blank {
                label: "b0".to_owned(),
                scope: BlankScope(7),
            },
            TermValue::simple_literal("cat"),
            TermValue::typed_literal("42", "http://www.w3.org/2001/XMLSchema#integer"),
            TermValue::lang_literal("chat", "fr"),
            quoted(EX_S, EX_P, EX_O),
        ] {
            assert_eq!(round_trip(&value), value, "{value:?} did not round-trip");
        }
    }

    /// A triple term nests: one whose object is itself a triple term is rebuilt to full
    /// depth, not truncated at the first level.
    #[test]
    fn intern_into_round_trips_a_nested_triple_term() {
        let nest = |inner: TermValue| TermValue::Triple {
            s: TermBox::new(TermValue::iri(EX_S)),
            p: TermBox::new(TermValue::iri(EX_P)),
            o: TermBox::new(inner),
        };
        // Nested in the object slot — the one position RDF 1.2 nests a triple term
        // in, so it is the whole of the nesting the chase can ever re-materialize.
        let outer = nest(quoted(EX_S, EX_P, EX_O));
        assert_eq!(round_trip(&outer), outer);
        // Depth 3, still through the object slot: the nesting is not special-cased
        // at one level, and a chase that folded the inner-most term to a stand-in
        // IRI would show up here rather than at depth 2.
        let deeper = nest(outer);
        assert_eq!(round_trip(&deeper), deeper);
    }

    /// Base direction participates in literal identity (C0.1), so it must survive the
    /// round trip — both directions, not just the first one somebody tested.
    #[test]
    fn intern_into_preserves_a_literal_base_direction() {
        for direction in [RdfTextDirection::Ltr, RdfTextDirection::Rtl] {
            let value = directional("hello", "en", direction);
            let back = round_trip(&value);
            assert_eq!(back, value);
            let TermValue::Literal { direction: got, .. } = back else {
                panic!("a literal round-tripped as something else");
            };
            assert_eq!(got, Some(direction));
        }
    }

    /// Two literals that differ ONLY in base direction are two different terms, on both
    /// sides of the round trip: as values, and as ids in a frozen dataset.
    #[test]
    fn literals_differing_only_in_direction_are_distinct_terms() {
        let ltr = directional("hello", "en", RdfTextDirection::Ltr);
        let rtl = directional("hello", "en", RdfTextDirection::Rtl);
        let none = TermValue::lang_literal("hello", "en");
        assert_ne!(ltr, rtl);
        assert_ne!(ltr, none);

        let mut b = RdfDatasetBuilder::new();
        let ltr_id = intern_into(&mut b, &ltr);
        let rtl_id = intern_into(&mut b, &rtl);
        let none_id = intern_into(&mut b, &none);
        assert_ne!(ltr_id, rtl_id, "ltr and rtl collapsed into one term");
        assert_ne!(ltr_id, none_id, "ltr collapsed into the undirected literal");
        assert_eq!(
            ltr_id,
            intern_into(&mut b, &ltr),
            "re-interning the same literal must be idempotent"
        );
    }

    /// Re-interning one triple term twice yields one term, so a closure that mentions it
    /// repeatedly does not grow the term table.
    #[test]
    fn intern_into_is_idempotent_for_a_triple_term() {
        let value = quoted(EX_S, EX_P, EX_O);
        let mut b = RdfDatasetBuilder::new();
        assert_eq!(intern_into(&mut b, &value), intern_into(&mut b, &value));
    }

    /// The subject guard: only IRIs and blank nodes may occupy subject position. A literal
    /// or a triple term reached there is a generalized-RDF conclusion the IR cannot hold,
    /// and repairing the fold must not have widened this.
    #[test]
    fn is_subject_admits_iris_and_blanks_and_refuses_literals_and_triple_terms() {
        let mut interner = Interner::default();
        let iri = interner.intern(TermValue::iri(EX_S));
        let blank = interner.intern(TermValue::blank("b0"));
        let literal = interner.intern(TermValue::simple_literal("cat"));
        let directional_literal =
            interner.intern(directional("hello", "en", RdfTextDirection::Rtl));
        let triple = interner.intern(quoted(EX_S, EX_P, EX_O));
        assert!(interner.is_subject(iri));
        assert!(interner.is_subject(blank));
        assert!(!interner.is_subject(literal));
        assert!(!interner.is_subject(directional_literal));
        assert!(!interner.is_subject(triple));
    }
}

#[cfg(test)]
mod term_walk_tests {
    //! Re-materialization against its recursive reference, and at a hundred thousand
    //! levels on a 128 KiB thread.

    use purrdf_core::{RdfDatasetBuilder, RdfLiteral, TermId, TermValue};

    use super::intern_into;

    fn reference(b: &mut RdfDatasetBuilder, v: &TermValue) -> TermId {
        match v {
            TermValue::Iri(iri) => b.intern_iri(iri),
            TermValue::Blank { label, scope } => b.intern_blank(label, *scope),
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            } => b.intern_literal(RdfLiteral {
                lexical_form: lexical_form.clone(),
                datatype: Some(datatype.clone()),
                language: language.clone(),
                direction: *direction,
            }),
            TermValue::Triple { s, p, o } => {
                let s = reference(b, s);
                let p = reference(b, p);
                let o = reference(b, o);
                b.intern_triple(s, p, o)
            }
        }
    }

    /// Every generated term interns into a fresh builder in the order the recursive
    /// reference interns it: the same id, and the same next id after it.
    #[test]
    fn interning_agrees_with_its_recursive_reference_on_generated_terms() {
        for seed in 0..400_u64 {
            let mut state = seed;
            let mut budget = 8;
            let value = purrdf_core::term_fixture::term_value(
                &mut state,
                purrdf_testkit::rng::splitmix64_next,
                &mut budget,
                purrdf_core::term_fixture::TermShape::Any,
            );
            let (mut found, mut expected) = (RdfDatasetBuilder::new(), RdfDatasetBuilder::new());
            assert_eq!(
                intern_into(&mut found, &value),
                reference(&mut expected, &value),
                "seed {seed}"
            );
            let sentinel = "http://example.org/sentinel";
            assert_eq!(
                found.intern_iri(sentinel),
                expected.intern_iri(sentinel),
                "seed {seed}"
            );
        }
    }

    /// A triple term a hundred thousand levels deep interns on a thread whose whole stack
    /// is 128 KiB: one term a level beside the chain's three leaves.
    #[test]
    fn a_hundred_thousand_level_term_interns_on_a_128_kib_thread() {
        const LEVELS: usize = 100_000;
        purrdf_stack::on_stack(128 * 1024, || {
            let value = purrdf_core::term_fixture::triple_chain(LEVELS);
            let mut builder = RdfDatasetBuilder::new();
            assert_eq!(intern_into(&mut builder, &value).index(), LEVELS + 2);
        })
        .expect("the thread starts");
    }
}

/// Blank-node subject → the indices of the `triples` it is the subject of, ascending.
///
/// The index every blank-node closure walk over an interned triple list reads; see
/// [`blank_closure`].
pub(crate) fn blank_subjects(
    interner: &Interner,
    triples: &[(u32, u32, u32)],
) -> BTreeMap<u32, Vec<usize>> {
    let mut blanks: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for (index, &(s, _, _)) in triples.iter().enumerate() {
        if matches!(interner.value(s), TermValue::Blank { .. }) {
            blanks.entry(s).or_default().push(index);
        }
    }
    blanks
}

/// Add to `out` the indices of every triple in the blank-node closure reachable from
/// `term`: the triples a blank subject carries, followed through their blank objects.
///
/// A non-blank `term` contributes nothing; each blank node is expanded once, so a cyclic
/// blank structure terminates. The one walk behind axiom explanation and module extraction,
/// which both carry an axiom's anonymous scaffolding along with it.
pub(crate) fn blank_closure(
    interner: &Interner,
    triples: &[(u32, u32, u32)],
    blanks: &BTreeMap<u32, Vec<usize>>,
    term: u32,
    out: &mut BTreeSet<usize>,
) {
    let mut stack = vec![term];
    let mut seen: BTreeSet<u32> = BTreeSet::new();
    while let Some(node) = stack.pop() {
        if !matches!(interner.value(node), TermValue::Blank { .. }) || !seen.insert(node) {
            continue;
        }
        for &index in blanks.get(&node).map_or(&[][..], Vec::as_slice) {
            out.insert(index);
            stack.push(triples[index].2);
        }
    }
}
