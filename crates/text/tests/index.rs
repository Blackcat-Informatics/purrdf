// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The index build, end to end over real datasets.
//!
//! The headline case is the RDF 1.2 **annotation layer**. `DatasetView::quads`
//! and `quads_for_pattern` expose the asserted triple table and nothing else;
//! reifier bindings and statement annotations live in separate, capability-gated
//! side tables. An index that reads only the asserted table therefore contains
//! zero annotation literals, and
//!
//! ```text
//! :s :p :o {| :note "annotation text" |}
//! ```
//!
//! is unsearchable with nothing anywhere reporting it. These tests assert the
//! opposite by construction.

use std::sync::Arc;

use pretty_assertions::assert_eq;
use purrdf_core::{
    BlankScope, DatasetView, GraphMatch, QuadIds, QuadRef, RdfDataset, RdfDatasetBuilder,
    RdfLiteral, RdfStoreCapabilities, RdfTextDirection, TermId, TermRef, TermValue,
};
use purrdf_text::{
    Analyzer, FINGERPRINT_BYTES, GraphSelector, PartitionFilter, PartitionKey, TextError,
    TextIndex, TextIndexConfig, rank_partition, select, verify_binding,
};

const S: &str = "https://example.org/s";
const O: &str = "https://example.org/o";
const P: &str = "https://example.org/p";
const NOTE: &str = "https://example.org/note";
const LABEL: &str = "https://example.org/label";
const REIFIER: &str = "https://example.org/r";
const GRAPH: &str = "https://example.org/g";

/// A configuration over `predicates` covering every graph.
fn config(predicates: &[&str]) -> TextIndexConfig {
    TextIndexConfig::new(
        predicates.iter().map(|p| TermValue::iri(*p)).collect(),
        GraphSelector::Any,
    )
    .expect("the fixture configurations are well formed")
}

/// Build an index over `dataset` for `predicates`, expecting success.
fn index_of(dataset: &Arc<RdfDataset>, predicates: &[&str]) -> TextIndex {
    TextIndex::from_dataset(&**dataset, &config(predicates)).expect("the fixture index must build")
}

/// The subjects of every document whose partition matches `partition`, in id
/// order.
fn subjects(index: &TextIndex) -> Vec<TermValue> {
    index
        .documents()
        .map(|document| document.subject().clone())
        .collect()
}

/// The default-graph, untagged partition — the one most fixtures live in.
fn plain() -> PartitionKey {
    PartitionKey::new(None, None)
}

// ── the annotation layer ─────────────────────────────────────────────────────

/// `:s :p :o {| :note "annotation text" |}` — the reifier is a document and its
/// text is searchable.
///
/// Pre-fix this found nothing at all: the annotation row is invisible to
/// `quads_for_pattern`, so the reifier had no literals, no document, and no
/// dictionary entries.
#[test]
fn annotation_layer_literals_are_indexed() {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(S);
    let p = builder.intern_iri(P);
    let o = builder.intern_iri(O);
    let note = builder.intern_iri(NOTE);
    let reifier = builder.intern_iri(REIFIER);
    let text = builder.intern_literal(RdfLiteral::simple("annotation text"));

    builder.push_quad(s, p, o, None);
    let statement = builder.intern_triple(s, p, o);
    builder.push_reifier(reifier, statement);
    builder.push_annotation(reifier, note, text);
    let dataset = builder.freeze().expect("the fixture must validate");

    // The asserted table really does hide the annotation: nothing in it carries
    // `ex:note` at all, so an index that read only that table would be empty.
    let note_id = dataset
        .term_id_by_value(&TermValue::iri(NOTE))
        .expect("ex:note is interned");
    assert_eq!(
        dataset
            .quads_for_pattern(None, Some(note_id), None, GraphMatch::Any)
            .count(),
        0,
        "the asserted table must not carry the annotation; that is the whole point"
    );

    let index = index_of(&dataset, &[NOTE]);
    assert_eq!(
        subjects(&index),
        vec![TermValue::iri(REIFIER)],
        "the reifier must be the document subject"
    );
    assert_eq!(
        index.document_frequency(&plain(), "annotation"),
        1,
        "the annotation's text must be searchable"
    );
    assert_eq!(index.document_frequency(&plain(), "text"), 1);
    assert_eq!(
        index.postings(&plain(), "text").collect::<Vec<_>>(),
        vec![(0_u32, &[1_u32][..])],
        "positions must be recorded for the annotation text too"
    );
}

/// A reifier that is also an ordinary subject collects both layers' text into
/// one document, because both layers name the same `(graph, subject, language)`.
#[test]
fn asserted_and_annotation_text_merge_into_one_document() {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(S);
    let p = builder.intern_iri(P);
    let o = builder.intern_iri(O);
    let note = builder.intern_iri(NOTE);
    let reifier = builder.intern_iri(REIFIER);
    let asserted = builder.intern_literal(RdfLiteral::simple("asserted"));
    let annotated = builder.intern_literal(RdfLiteral::simple("annotated"));

    builder.push_quad(s, p, o, None);
    builder.push_quad(reifier, note, asserted, None);
    let statement = builder.intern_triple(s, p, o);
    builder.push_reifier(reifier, statement);
    builder.push_annotation(reifier, note, annotated);
    let dataset = builder.freeze().expect("the fixture must validate");

    let index = index_of(&dataset, &[NOTE]);
    assert_eq!(index.document_count(), 1, "both layers name one document");
    assert_eq!(
        index.document_length(0),
        Some(2),
        "the document holds one token from each layer"
    );
    assert_eq!(index.term_frequency(0, "asserted"), 1);
    assert_eq!(index.term_frequency(0, "annotated"), 1);
}

// ── subject shapes ───────────────────────────────────────────────────────────

/// A blank-node subject is a document like any other.
#[test]
fn blank_node_subjects_are_indexed() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_blank("b0", BlankScope::DEFAULT);
    let note = builder.intern_iri(NOTE);
    let text = builder.intern_literal(RdfLiteral::simple("blank subject text"));
    builder.push_quad(subject, note, text, None);
    let dataset = builder.freeze().expect("the fixture must validate");

    let index = index_of(&dataset, &[NOTE]);
    assert_eq!(
        subjects(&index),
        vec![TermValue::Blank {
            label: "b0".to_owned(),
            scope: BlankScope::DEFAULT,
        }]
    );
    assert_eq!(index.document_frequency(&plain(), "blank"), 1);
}

/// A view that re-resolves one id as a triple term, leaving everything else
/// alone.
///
/// This exists because `RdfDataset` **cannot** hold a triple-term subject:
/// `crates/rdf-core/src/ir/validate.rs` hard-fails one with the diagnostic code
/// `rdf-ir-triple-subject`, and the same `require_asserted_subject` check gates
/// reifiers and annotation rows, so no builder can produce the case. `DatasetView`
/// is a trait, though, and a backend that admits a triple term in that position
/// must not make the index panic or mis-resolve — hence a view that produces
/// exactly that shape.
#[derive(Debug)]
struct TripleSubjectView {
    /// The real dataset every other id is resolved against.
    inner: Arc<RdfDataset>,
    /// The id whose resolution is replaced.
    disguised: TermId,
    /// The triple term `disguised` resolves to instead.
    parts: (TermId, TermId, TermId),
}

impl DatasetView for TripleSubjectView {
    type Id = TermId;
    type ProbePlan = <RdfDataset as DatasetView>::ProbePlan;

    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        DatasetView::quads(&*self.inner)
    }

    fn quad_refs(&self) -> impl Iterator<Item = QuadRef<'_>> + '_ {
        DatasetView::quad_refs(&*self.inner)
    }

    fn resolve(&self, id: TermId) -> TermRef<'_> {
        if id == self.disguised {
            let (s, p, o) = self.parts;
            return TermRef::Triple { s, p, o };
        }
        DatasetView::resolve(&*self.inner, id)
    }

    fn quads_for_pattern(
        &self,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        DatasetView::quads_for_pattern(&*self.inner, s, p, o, g)
    }

    fn term_id_by_value(&self, value: &TermValue) -> Option<TermId> {
        DatasetView::term_id_by_value(&*self.inner, value)
    }

    fn capabilities(&self) -> RdfStoreCapabilities {
        DatasetView::capabilities(&*self.inner)
    }

    fn probe_plan(
        &self,
        s_bound: bool,
        p_bound: bool,
        o_bound: bool,
        g: GraphMatch,
    ) -> Self::ProbePlan {
        DatasetView::probe_plan(&*self.inner, s_bound, p_bound, o_bound, g)
    }

    fn quads_for_pattern_with_plan(
        &self,
        plan: &Self::ProbePlan,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        DatasetView::quads_for_pattern_with_plan(&*self.inner, plan, s, p, o, g)
    }

    fn term_count(&self) -> usize {
        DatasetView::term_count(&*self.inner)
    }
}

/// A triple-term subject resolves through the recursive walk and becomes a
/// document keyed by the whole triple term.
#[test]
fn triple_term_subjects_are_indexed() {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(S);
    let p = builder.intern_iri(P);
    let o = builder.intern_iri(O);
    let note = builder.intern_iri(NOTE);
    let text = builder.intern_literal(RdfLiteral::simple("quoted subject text"));
    builder.push_quad(s, note, text, None);
    // Keeps the triple term's components referenced by an ordinary statement,
    // under a predicate the configuration does not name.
    builder.push_quad(o, p, o, None);
    let dataset = builder.freeze().expect("the fixture must validate");

    // The disguise must not name `s` itself: a term that contains its own id is
    // an infinite structure, and the resolver's depth bound would (correctly)
    // refuse it.
    let view = TripleSubjectView {
        inner: Arc::clone(&dataset),
        disguised: s,
        parts: (o, p, o),
    };
    let index = TextIndex::from_dataset(&view, &config(&[NOTE]))
        .expect("a triple-term subject must not fail the build");

    assert_eq!(
        subjects(&index),
        vec![TermValue::Triple {
            s: Box::new(TermValue::iri(O)),
            p: Box::new(TermValue::iri(P)),
            o: Box::new(TermValue::iri(O)),
        }],
        "the subject must be the whole triple term, resolved component by component"
    );
    assert_eq!(index.document_frequency(&plain(), "quoted"), 1);
}

// ── the document key ─────────────────────────────────────────────────────────

/// Language splits documents; base direction does not.
///
/// Direction is presentational — it says which way glyphs run, never which text
/// is present — so it cannot change what a query matches, and splitting on it
/// would halve the statistics of any corpus that annotates it inconsistently.
#[test]
fn language_splits_documents_but_base_direction_does_not() {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(S);
    let note = builder.intern_iri(NOTE);
    let plain_en = builder.intern_literal(RdfLiteral::language_tagged("alpha", "en"));
    let directional_en = builder.intern_literal(RdfLiteral {
        lexical_form: "beta".to_owned(),
        datatype: None,
        language: Some("en".to_owned()),
        direction: Some(RdfTextDirection::Ltr),
    });
    let french = builder.intern_literal(RdfLiteral::language_tagged("gamma", "fr"));
    builder.push_quad(s, note, plain_en, None);
    builder.push_quad(s, note, directional_en, None);
    builder.push_quad(s, note, french, None);
    let dataset = builder.freeze().expect("the fixture must validate");

    let index = index_of(&dataset, &[NOTE]);
    assert_eq!(
        index.document_count(),
        2,
        "two languages must be two documents; the direction must not add a third"
    );

    let english = PartitionKey::new(None, Some("en".to_owned()));
    let french_key = PartitionKey::new(None, Some("fr".to_owned()));
    assert_eq!(
        index
            .partition_stats(&english)
            .map(purrdf_text::PartitionStats::document_count),
        Some(1)
    );
    assert_eq!(
        index
            .partition_stats(&english)
            .map(purrdf_text::PartitionStats::total_tokens),
        Some(2),
        "both the plain and the directional literal belong to the one English document"
    );
    assert_eq!(index.document_frequency(&english, "alpha"), 1);
    assert_eq!(index.document_frequency(&english, "beta"), 1);
    assert_eq!(
        index.document_frequency(&english, "gamma"),
        0,
        "French text must not be findable in the English partition"
    );
    assert_eq!(index.document_frequency(&french_key, "gamma"), 1);
}

/// An untagged literal keys its own partition, distinct from every real tag —
/// `None`, never a sentinel empty string a real tag could collide with.
#[test]
fn untagged_literals_do_not_collide_with_a_real_language_tag() {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(S);
    let note = builder.intern_iri(NOTE);
    let untagged = builder.intern_literal(RdfLiteral::simple("untagged"));
    let tagged = builder.intern_literal(RdfLiteral::language_tagged("tagged", "en"));
    builder.push_quad(s, note, untagged, None);
    builder.push_quad(s, note, tagged, None);
    let dataset = builder.freeze().expect("the fixture must validate");

    let index = index_of(&dataset, &[NOTE]);
    assert_eq!(index.document_count(), 2);
    assert_eq!(index.partition_count(), 2);

    assert!(
        index.document(0).is_some_and(|d| d.language().is_none())
            || index.document(1).is_some_and(|d| d.language().is_none()),
        "one document must be the untagged one"
    );
    assert_eq!(index.document_frequency(&plain(), "untagged"), 1);
    assert_eq!(
        index.document_frequency(&plain(), "tagged"),
        0,
        "tagged text must not leak into the untagged partition"
    );

    // The empty string is not a partition of this index at all: it is not a
    // stand-in for "no tag", so nothing was filed under it.
    let empty_tag = PartitionKey::new(None, Some(String::new()));
    assert!(
        index.partition_stats(&empty_tag).is_none(),
        "an empty language tag must not be the untagged partition under another name"
    );
    assert_eq!(index.document_frequency(&empty_tag, "untagged"), 0);
}

/// A subject whose only literal analyzes to nothing is not a document.
///
/// This is the invariant that keeps every retained partition's average document
/// length non-zero, which the BM25 denominator depends on.
#[test]
fn a_document_with_no_analyzable_tokens_is_excluded() {
    let mut builder = RdfDatasetBuilder::new();
    let punctuation_only = builder.intern_iri("https://example.org/quiet");
    let real = builder.intern_iri(S);
    let note = builder.intern_iri(NOTE);
    let noise = builder.intern_literal(RdfLiteral::simple("--- ... !!!"));
    let text = builder.intern_literal(RdfLiteral::simple("real text"));
    builder.push_quad(punctuation_only, note, noise, None);
    builder.push_quad(real, note, text, None);
    let dataset = builder.freeze().expect("the fixture must validate");

    let index = index_of(&dataset, &[NOTE]);
    assert_eq!(
        subjects(&index),
        vec![TermValue::iri(S)],
        "the punctuation-only subject must not be a document"
    );
    let average = index
        .partition_stats(&plain())
        .expect("the default partition exists")
        .average_document_length();
    assert!(
        average > purrdf_text::Fixed::ZERO,
        "every retained partition must have a non-zero average document length"
    );
}

// ── statistics ───────────────────────────────────────────────────────────────

/// Corpus statistics are per partition and never pooled.
///
/// Pooling would make an English needle's inverse document frequency a function
/// of a mostly-French corpus, and would print two documents' scores as
/// comparable when they were computed against different vocabularies.
#[test]
fn statistics_are_per_partition_not_pooled() {
    let mut builder = RdfDatasetBuilder::new();
    let note = builder.intern_iri(NOTE);
    // One short English document, two long French ones.
    let en_subject = builder.intern_iri("https://example.org/en");
    let en_text = builder.intern_literal(RdfLiteral::language_tagged("one two", "en"));
    builder.push_quad(en_subject, note, en_text, None);
    for (index, lexical) in ["un deux trois quatre", "cinq six sept huit"]
        .into_iter()
        .enumerate()
    {
        let subject = builder.intern_iri(&format!("https://example.org/fr{index}"));
        let literal = builder.intern_literal(RdfLiteral::language_tagged(lexical, "fr"));
        builder.push_quad(subject, note, literal, None);
    }
    let dataset = builder.freeze().expect("the fixture must validate");

    let index = index_of(&dataset, &[NOTE]);
    assert_eq!(index.partition_count(), 2);
    assert_eq!(index.document_count(), 3);
    assert_eq!(index.max_documents_in_any_partition(), 2);

    let english = index
        .partition_stats(&PartitionKey::new(None, Some("en".to_owned())))
        .copied()
        .expect("the English partition exists");
    let french = index
        .partition_stats(&PartitionKey::new(None, Some("fr".to_owned())))
        .copied()
        .expect("the French partition exists");

    assert_eq!(english.document_count(), 1);
    assert_eq!(english.total_tokens(), 2);
    assert_eq!(french.document_count(), 2);
    assert_eq!(french.total_tokens(), 8);
    assert_ne!(
        english.average_document_length(),
        french.average_document_length(),
        "a pooled average would give both partitions the same number"
    );
    assert_eq!(
        english.average_document_length(),
        purrdf_text::Fixed::from_integer(2).expect("2 is representable")
    );
    assert_eq!(
        french.average_document_length(),
        purrdf_text::Fixed::from_integer(4).expect("4 is representable")
    );
}

// ── identity ─────────────────────────────────────────────────────────────────

/// Two builders given the same triples in different orders mint different
/// `TermId`s and still produce the same index — both digests included.
///
/// Ids are assigned after the documents are sorted by content, so nothing about
/// the dataset's intern order survives into the index.
#[test]
fn two_independently_built_datasets_share_a_fingerprint() {
    let triples = [
        ("https://example.org/a", "alpha beta"),
        ("https://example.org/b", "gamma delta"),
        ("https://example.org/c", "epsilon zeta"),
    ];

    let forward = {
        let mut builder = RdfDatasetBuilder::new();
        let note = builder.intern_iri(NOTE);
        for (subject, text) in triples {
            let s = builder.intern_iri(subject);
            let literal = builder.intern_literal(RdfLiteral::simple(text));
            builder.push_quad(s, note, literal, None);
        }
        builder.freeze().expect("the fixture must validate")
    };
    let backward = {
        let mut builder = RdfDatasetBuilder::new();
        for (subject, text) in triples.into_iter().rev() {
            let s = builder.intern_iri(subject);
            let note = builder.intern_iri(NOTE);
            let literal = builder.intern_literal(RdfLiteral::simple(text));
            builder.push_quad(s, note, literal, None);
        }
        builder.freeze().expect("the fixture must validate")
    };

    let note_forward = forward
        .term_id_by_value(&TermValue::iri(NOTE))
        .expect("interned");
    let note_backward = backward
        .term_id_by_value(&TermValue::iri(NOTE))
        .expect("interned");
    assert_ne!(
        note_forward, note_backward,
        "the two builders must mint different ids, or this test proves nothing"
    );

    let left = index_of(&forward, &[NOTE]);
    let right = index_of(&backward, &[NOTE]);
    assert_eq!(
        left.fingerprint(),
        right.fingerprint(),
        "the same content must fingerprint identically"
    );
    assert_eq!(
        left.source_fingerprint(),
        right.source_fingerprint(),
        "the same rows must digest identically"
    );
    assert_eq!(subjects(&left), subjects(&right));
}

/// The caveat, pinned rather than papered over: blank-node labels are a parsing
/// artifact, not content, but `TermValue::Blank` orders and encodes by label —
/// so two isomorphic datasets that label the same node differently disagree on
/// both digests. This crate does not canonicalize blank nodes.
#[test]
fn blank_node_labels_change_the_fingerprint() {
    let build = |label: &str| {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_blank(label, BlankScope::DEFAULT);
        let note = builder.intern_iri(NOTE);
        let literal = builder.intern_literal(RdfLiteral::simple("identical text"));
        builder.push_quad(subject, note, literal, None);
        builder.freeze().expect("the fixture must validate")
    };

    let left = index_of(&build("b0"), &[NOTE]);
    let right = index_of(&build("b17"), &[NOTE]);
    assert_eq!(
        left.document_count(),
        right.document_count(),
        "the two datasets are isomorphic"
    );
    assert_ne!(
        left.fingerprint(),
        right.fingerprint(),
        "a blank label reaches the fingerprint; the determinism claim is scoped to terms"
    );
    assert_ne!(
        left.source_fingerprint(),
        right.source_fingerprint(),
        "and it reaches the source digest for the same reason"
    );
}

// ── configuration ────────────────────────────────────────────────────────────

/// Only an IRI can be an RDF predicate, so anything else names nothing.
#[test]
fn a_non_iri_predicate_is_a_config_error() {
    let error = TextIndexConfig::new(
        vec![TermValue::simple_literal("not a predicate")],
        GraphSelector::Any,
    )
    .expect_err("a literal cannot be a predicate");
    assert!(matches!(error, TextError::Config(_)), "got {error:?}");

    // The neighbouring valid case, executed rather than assumed: the same
    // refusal must not be firing on an IRI. A refusal that is never checked
    // against the input it is supposed to admit looks identical to one that
    // rejects everything.
    TextIndexConfig::new(vec![TermValue::iri(NOTE)], GraphSelector::Any)
        .expect("an IRI predicate is exactly what this configuration takes");

    // Including IRIs whose spelling is unusual — a refusal keyed on shape
    // rather than on term kind would reject these.
    for iri in [
        "urn:example:note",
        "https://example.org/a#b",
        "tag:x,2026:y",
    ] {
        TextIndexConfig::new(vec![TermValue::iri(iri)], GraphSelector::Any)
            .unwrap_or_else(|error| panic!("{iri} is an IRI and must be accepted: {error}"));
    }
}

/// PurRDF mints no vocabulary, so there is no default predicate set to fall
/// back on and an empty one is refused rather than guessed at.
#[test]
fn an_empty_predicate_set_is_a_config_error() {
    let error = TextIndexConfig::new(Vec::new(), GraphSelector::Any)
        .expect_err("an empty predicate set has no fallback");
    assert!(matches!(error, TextError::Config(_)), "got {error:?}");
}

/// A repeated predicate is a caller mistake; deduplicating it silently would
/// hide the mistake.
#[test]
fn a_duplicate_predicate_is_a_config_error() {
    let error = TextIndexConfig::new(
        vec![
            TermValue::iri(NOTE),
            TermValue::iri(LABEL),
            TermValue::iri(NOTE),
        ],
        GraphSelector::Any,
    )
    .expect_err("a repeated predicate is refused");
    assert!(matches!(error, TextError::Config(_)), "got {error:?}");

    // The neighbouring valid case: the same two predicates WITHOUT the repeat.
    // The duplicate check compares adjacent entries of a sorted list, so an
    // off-by-one in it would reject every list of two distinct predicates —
    // and the refusal above would still pass.
    let config = TextIndexConfig::new(
        vec![TermValue::iri(NOTE), TermValue::iri(LABEL)],
        GraphSelector::Any,
    )
    .expect("two distinct predicates are a valid configuration");
    assert_eq!(config.predicates().len(), 2);
}

/// A configured predicate the dataset does not carry contributes **no rows**.
///
/// The dataset holds no statement with that predicate, so there is no text under
/// it, and that is an answer rather than a fault. The limiting case is what
/// decides it: an empty dataset has interned no term at all, so refusing an
/// absent predicate would mean no index could be built over one — and an index
/// that exists before its documents land is an ordinary operating state, not an
/// error.
#[test]
fn a_predicate_absent_from_the_dataset_contributes_no_rows() {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(S);
    let note = builder.intern_iri(NOTE);
    let text = builder.intern_literal(RdfLiteral::simple("present"));
    builder.push_quad(s, note, text, None);
    let dataset = builder.freeze().expect("the fixture must validate");

    let widened = TextIndex::from_dataset(&*dataset, &config(&[NOTE, LABEL]))
        .expect("ex:label is absent from the dataset, which is no text rather than a fault");
    assert_eq!(
        widened.document_count(),
        1,
        "the one document ex:note yields, and nothing invented for ex:label"
    );
    assert_eq!(
        widened.document_length(0),
        Some(1),
        "only the present predicate's single token is indexed"
    );

    // The neighbouring valid case, against the SAME dataset: a configuration
    // naming only the predicate that IS present. Its rows are identical — an
    // absent predicate is not a filter that could have dropped anything — while
    // its fingerprint is NOT, because the configuration is part of the index's
    // identity. A build that quietly dropped the absent predicate from the
    // configuration instead of from the walk would agree here and must not.
    let narrow = TextIndex::from_dataset(&*dataset, &config(&[NOTE]))
        .expect("ex:note IS in the dataset, so this configuration must build");
    assert_eq!(narrow.document_count(), 1);
    assert_eq!(
        subjects(&narrow),
        subjects(&widened),
        "the same rows either way"
    );
    assert_ne!(
        narrow.fingerprint(),
        widened.fingerprint(),
        "the configuration is part of the index's identity, so the two are different indexes \
         over the same rows"
    );
    assert_eq!(narrow.config().predicates().len(), 1);
    assert_eq!(
        widened.config().predicates().len(),
        2,
        "the absent predicate stays in the configuration; it is the walk that finds nothing"
    );

    // And the absent predicate is genuinely inert rather than aliased onto the
    // present one: text that lands under ex:label later is found by the SAME
    // configuration, which is the whole reason the widened build is allowed.
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(S);
    let note = builder.intern_iri(NOTE);
    let label = builder.intern_iri(LABEL);
    let text = builder.intern_literal(RdfLiteral::simple("present"));
    let later = builder.intern_literal(RdfLiteral::simple("arrived"));
    builder.push_quad(s, note, text, None);
    builder.push_quad(s, label, later, None);
    let grown = builder.freeze().expect("the fixture must validate");
    let grown = TextIndex::from_dataset(&*grown, &config(&[NOTE, LABEL]))
        .expect("the same configuration builds once ex:label carries text");
    assert_eq!(grown.document_frequency(&plain(), "arrived"), 1);
    assert_ne!(
        grown.fingerprint(),
        widened.fingerprint(),
        "the arriving document moves the generation"
    );
}

/// An index over an **empty** dataset builds, holds nothing, still attests a
/// generation, and answers a search with zero rows.
///
/// This is the case the absent-predicate refusal used to make unreachable. An
/// empty dataset interns no term, so every configured predicate is absent from
/// it; if that were an error, an index could never exist before the documents it
/// will hold, and a text producer could never be in the state its own retrieval
/// contract describes — invoked, and reporting zero rows on its own receipt.
#[test]
fn an_index_over_an_empty_dataset_holds_nothing_and_still_attests_a_generation() {
    let empty = RdfDatasetBuilder::new()
        .freeze()
        .expect("a dataset with no quads is a valid dataset");
    let over_note = config(&[NOTE]);

    let index = TextIndex::from_dataset(&*empty, &over_note)
        .expect("an index over an empty dataset is an ordinary operating state");
    assert_eq!(index.document_count(), 0);
    assert_eq!(
        index.partition_count(),
        0,
        "a partition carries at least one document, so an empty index holds none"
    );
    assert_eq!(index.term_count(), 0);
    assert_eq!(index.max_documents_in_any_partition(), 0);
    assert!(
        index.partitions().next().is_none(),
        "and there is no PartitionStats to read a zero average document length out of"
    );

    // A query against it is an answer, not a panic and not a division by zero:
    // BM25's denominator is a partition's average document length, and there is
    // no partition to divide within.
    assert!(
        select(
            &index,
            &needle_terms("anything at all"),
            &PartitionFilter::unconstrained(),
            None,
            None,
        )
        .expect("a search over an empty index answers")
        .is_empty(),
        "a needle no document holds yields no rows rather than a panic"
    );
    assert_eq!(index.document_frequency(&plain(), "anything"), 0);
    assert!(
        rank_partition(&index, &plain(), &needle_terms("anything"), None)
            .expect("ranking a partition the index does not hold is an empty answer")
            .is_empty(),
        "naming a partition an empty index does not hold yields no rows"
    );

    // It attests a generation, and the generation is a real content identity:
    // the configuration is digested before any content, so an empty index under
    // another configuration is a different index, and the value moves the moment
    // the first document lands.
    let other = TextIndex::from_dataset(&*empty, &config(&[LABEL]))
        .expect("an empty dataset indexes under any configuration");
    assert_ne!(
        index.fingerprint(),
        other.fingerprint(),
        "two empty indexes under different configurations are distinguishable"
    );
    assert_ne!(
        index.fingerprint(),
        [0; FINGERPRINT_BYTES],
        "the generation of an empty index is a digest, not a placeholder"
    );

    // The same configuration once one document lands: a non-zero corpus, an
    // answer, and a generation that differs from the empty index's.
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(S);
    let note = builder.intern_iri(NOTE);
    let text = builder.intern_literal(RdfLiteral::simple("anything at all"));
    builder.push_quad(s, note, text, None);
    let landed = builder.freeze().expect("the fixture must validate");
    let landed = TextIndex::from_dataset(&*landed, &over_note)
        .expect("the SAME configuration must build once the document lands");
    assert_eq!(landed.document_count(), 1);
    assert_eq!(landed.partition_count(), 1);
    assert_eq!(
        select(
            &landed,
            &needle_terms("anything at all"),
            &PartitionFilter::unconstrained(),
            None,
            None,
        )
        .expect("a search over the landed corpus answers")
        .len(),
        1,
        "the document is retrievable, so the empty answer above was emptiness rather than a \
         search that cannot find anything"
    );
    assert_ne!(
        index.fingerprint(),
        landed.fingerprint(),
        "the arriving document moves the attested generation"
    );

    // And the empty index is honestly bound to the empty dataset: the source
    // digest is a value, so a host can still ask whether it is holding the data
    // its index was built over.
    verify_binding(&index, &*empty, &over_note)
        .expect("the empty index is bound to the empty dataset");
    let error = verify_binding(&index, &*landed_dataset(), &over_note)
        .expect_err("the empty index is not bound to a dataset that has text in it");
    assert!(matches!(error, TextError::Data(_)), "got {error:?}");
}

/// The one-document dataset the empty index must NOT verify against.
fn landed_dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(S);
    let note = builder.intern_iri(NOTE);
    let text = builder.intern_literal(RdfLiteral::simple("anything at all"));
    builder.push_quad(s, note, text, None);
    builder.freeze().expect("the fixture must validate")
}

/// The analyzed terms of `text`, as a needle for [`select`].
fn needle_terms(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    Analyzer::new().analyze(text, &mut tokens);
    tokens
        .into_iter()
        .map(|token| token.text.into_owned())
        .collect()
}

/// A predicate may legitimately carry both literals and IRIs. A non-literal
/// object contributes no text and raises nothing — that is data, not a fault.
#[test]
fn non_literal_objects_are_skipped_not_errors() {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(S);
    let note = builder.intern_iri(NOTE);
    let o = builder.intern_iri(O);
    let blank = builder.intern_blank("b0", BlankScope::DEFAULT);
    let text = builder.intern_literal(RdfLiteral::simple("only this counts"));
    builder.push_quad(s, note, o, None);
    builder.push_quad(s, note, blank, None);
    builder.push_quad(s, note, text, None);
    let dataset = builder.freeze().expect("the fixture must validate");

    let index = index_of(&dataset, &[NOTE]);
    assert_eq!(index.document_count(), 1);
    assert_eq!(
        index.document_length(0),
        Some(3),
        "only the literal's three tokens are indexed"
    );
}

/// A named-graph selector restricts the walk, and the graph reaches the
/// document key so the same subject in two graphs is two documents.
#[test]
fn a_named_graph_selector_restricts_the_walk() {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(S);
    let note = builder.intern_iri(NOTE);
    let graph = builder.intern_iri(GRAPH);
    let inside = builder.intern_literal(RdfLiteral::simple("inside"));
    let outside = builder.intern_literal(RdfLiteral::simple("outside"));
    builder.push_quad(s, note, inside, Some(graph));
    builder.push_quad(s, note, outside, None);
    let dataset = builder.freeze().expect("the fixture must validate");

    let everywhere = index_of(&dataset, &[NOTE]);
    assert_eq!(
        everywhere.document_count(),
        2,
        "the graph is part of the document key"
    );

    let named = TextIndex::from_dataset(
        &*dataset,
        &TextIndexConfig::new(
            vec![TermValue::iri(NOTE)],
            GraphSelector::Named(TermValue::iri(GRAPH)),
        )
        .expect("a named-graph configuration is well formed"),
    )
    .expect("the index must build");
    assert_eq!(named.document_count(), 1);
    let partition = PartitionKey::new(Some(TermValue::iri(GRAPH)), None);
    assert_eq!(named.document_frequency(&partition, "inside"), 1);
    assert_eq!(named.document_frequency(&partition, "outside"), 0);

    // A named graph the dataset does not carry selects no quads, for the same
    // reason an absent predicate contributes no rows: nothing is in a graph that
    // is not there. It is the same limiting case too — the graph IRI of an
    // otherwise valid configuration is interned only once something is *in* that
    // graph, so refusing here would make the recommended single-partition
    // configuration the one that cannot be built first. The valid neighbour is
    // the case just above, which names a graph that IS present and found the one
    // document in it, so the walk is shown to discriminate rather than to empty
    // every named selector.
    let absent = TextIndex::from_dataset(
        &*dataset,
        &TextIndexConfig::new(
            vec![TermValue::iri(NOTE)],
            GraphSelector::Named(TermValue::iri("https://example.org/absent")),
        )
        .expect("a named-graph configuration is well formed"),
    )
    .expect("the dataset carries no such graph, which is an empty index");
    assert_eq!(absent.document_count(), 0);
    assert_eq!(absent.partition_count(), 0);
    assert_ne!(
        absent.fingerprint(),
        named.fingerprint(),
        "the two name different graphs, so they are different indexes"
    );

    // And the default graph is still selectable, which is the third neighbour a
    // graph-presence check could wrongly reject: the default graph is named by
    // no term at all.
    let default = TextIndex::from_dataset(
        &*dataset,
        &TextIndexConfig::new(vec![TermValue::iri(NOTE)], GraphSelector::Default)
            .expect("a default-graph configuration is well formed"),
    )
    .expect("the default graph needs no term to be present");
    assert_eq!(default.document_count(), 1);
    assert_eq!(
        default.document_frequency(&PartitionKey::new(None, None), "outside"),
        1
    );
}
