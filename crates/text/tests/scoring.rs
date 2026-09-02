// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! BM25 scoring, ranking and selection, end to end over real indexes.
//!
//! Every assertion here is exact. The arithmetic is fixed point, so an expected
//! score is a literal `xsd:decimal` string rather than a value compared within a
//! tolerance, and the sum of a document's term contributions equals its score
//! rather than approximating it.

use std::sync::Arc;

use pretty_assertions::assert_eq;
use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_text::{
    Analyzer, Constraint, Fixed, GraphSelector, PartitionFilter, PartitionKey, Scored, TextError,
    TextIndex, TextIndexConfig, explain, rank_partition, select,
};

/// The one predicate every fixture indexes.
const NOTE: &str = "https://example.org/note";

// ── fixtures ─────────────────────────────────────────────────────────────────

/// Build an index over `(subject, lexical form, language tag)` rows.
fn index_of(rows: &[(&str, &str, Option<&str>)]) -> TextIndex {
    let mut builder = RdfDatasetBuilder::new();
    let note = builder.intern_iri(NOTE);
    for &(subject, text, language) in rows {
        let s = builder.intern_iri(subject);
        let literal = builder.intern_literal(match language {
            Some(tag) => RdfLiteral::language_tagged(text, tag),
            None => RdfLiteral::simple(text),
        });
        builder.push_quad(s, note, literal, None);
    }
    let dataset: Arc<RdfDataset> = builder.freeze().expect("the fixture must validate");
    TextIndex::from_dataset(
        &*dataset,
        &TextIndexConfig::new(vec![TermValue::iri(NOTE)], GraphSelector::Any)
            .expect("the fixture configuration is well formed"),
    )
    .expect("the fixture index must build")
}

/// The analyzed needle for `text`, exactly as a query would supply it.
fn needle(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    Analyzer::new().analyze(text, &mut tokens);
    tokens
        .into_iter()
        .map(|token| token.text.into_owned())
        .collect()
}

/// The default-graph, untagged partition — where most fixtures live.
fn plain() -> PartitionKey {
    PartitionKey::new(None, None)
}

/// A filter that admits every partition.
fn everything() -> PartitionFilter {
    PartitionFilter::unconstrained()
}

/// The golden fixture: four documents of four tokens each, so `avgdl` is exactly
/// four and every document's length normalization is exactly one.
///
/// Document ids follow the subject IRIs' sorted order, so `ex:a` is `0`, `ex:b`
/// is `1`, and so on.
fn golden_index() -> TextIndex {
    index_of(&[
        ("https://example.org/a", "alpha alpha beta gamma", None),
        ("https://example.org/b", "alpha beta gamma delta", None),
        ("https://example.org/c", "epsilon zeta eta theta", None),
        ("https://example.org/d", "iota kappa lambda mu", None),
    ])
}

/// Two documents with byte-identical text under different subjects, listed in
/// `order`, so their scores are exactly equal and only the tie-break separates
/// them.
fn tie_index(order: [&str; 2]) -> TextIndex {
    index_of(&[
        (order[0], "alpha beta", None),
        (order[1], "alpha beta", None),
    ])
}

// ── the golden ───────────────────────────────────────────────────────────────

/// BM25 against a hand-computed golden, worked out here so a reader can check
/// every digit.
///
/// The fixture is four documents of four tokens each, in one partition:
///
/// ```text
/// d0  ex:a  "alpha alpha beta gamma"    tf(alpha) = 2, tf(beta) = 1
/// d1  ex:b  "alpha beta gamma delta"    tf(alpha) = 1, tf(beta) = 1
/// d2  ex:c  "epsilon zeta eta theta"
/// d3  ex:d  "iota kappa lambda mu"
/// ```
///
/// so `N = 4`, the token total is `16`, and `avgdl = 16/4 = 4` exactly. Every
/// document has `|D| = 4`, so `|D|/avgdl = 1` and the length normalization
/// `1 − b + b·1` is exactly `1`, whatever `b` is.
///
/// The needle is `alpha beta`. Both terms occur in `d0` and `d1` and nowhere
/// else, so `df = 2` for each, and
///
/// ```text
/// IDF = ln(1 + (4 − 2 + 1/2) / (2 + 1/2)) = ln(1 + 2.5/2.5) = ln 2
/// ```
///
/// `2.5/2.5` is exactly `1`, so no truncation happens before the logarithm, and
/// `ln 2 = 0.693147180559` at twelve digits — the same constant this crate's
/// fixed-point tests pin independently.
///
/// With the normalization equal to one, the saturation is `tf·(k1+1)/(tf+k1)`:
///
/// ```text
/// tf = 1:  2.2 / 2.2                     = 1
/// tf = 2:  4.4 / (2 + 1.2) = 4.4 / 3.2   = 1.375
/// ```
///
/// both exact. So:
///
/// ```text
/// d0: alpha  0.693147180559 × 1.375 = 0.953077373268625 → 0.953077373268
///     beta   0.693147180559 × 1     = 0.693147180559
///     score                           1.646224553827
///
/// d1: alpha  0.693147180559
///     beta   0.693147180559
///     score  1.386294361118
/// ```
///
/// `d2` and `d3` hold neither term and are not rows at all: their score would be
/// a sum over nothing, and a zero-scoring row is not a retrieval result.
#[test]
fn bm25_matches_a_hand_computed_golden() {
    let index = golden_index();
    let stats = index
        .partition_stats(&plain())
        .expect("the fixture has one partition");
    assert_eq!(stats.document_count(), 4);
    assert_eq!(stats.total_tokens(), 16);
    assert_eq!(
        stats.average_document_length(),
        Fixed::from_integer(4).expect("representable"),
        "the fixture is built so that avgdl is exactly four"
    );
    assert_eq!(index.document_frequency(&plain(), "alpha"), 2);
    assert_eq!(index.document_frequency(&plain(), "beta"), 2);

    let rows = rank_partition(&index, &plain(), &needle("alpha beta"), None)
        .expect("the fixture scores without overflowing");

    assert_eq!(rows.len(), 2, "only two documents hold a needle term");
    assert_eq!(rows[0].document, 0);
    assert_eq!(rows[0].partition_rank, 1);
    assert_eq!(rows[0].matched, 2);
    assert_eq!(rows[0].score.to_decimal_lexical(), "1.646224553827");
    assert_eq!(rows[1].document, 1);
    assert_eq!(rows[1].partition_rank, 2);
    assert_eq!(rows[1].matched, 2);
    assert_eq!(rows[1].score.to_decimal_lexical(), "1.386294361118");
}

/// The golden's individual term contributions, digit for digit.
#[test]
fn each_term_contribution_matches_the_hand_computation() {
    let index = golden_index();
    let rows = explain(&index, 0, &needle("alpha beta")).expect("document 0 exists");

    assert_eq!(rows.len(), 2, "two distinct needle terms");
    assert_eq!(rows[0].term, "alpha", "terms come back in sorted order");
    assert_eq!(rows[0].term_frequency, 2);
    assert_eq!(rows[0].document_frequency, 2);
    assert_eq!(
        rows[0].inverse_document_frequency.to_decimal_lexical(),
        "0.693147180559"
    );
    assert_eq!(rows[0].contribution.to_decimal_lexical(), "0.953077373268");

    assert_eq!(rows[1].term, "beta");
    assert_eq!(rows[1].term_frequency, 1);
    assert_eq!(rows[1].contribution.to_decimal_lexical(), "0.693147180559");
}

// ── the total order ──────────────────────────────────────────────────────────

/// Two documents that score **exactly** equal are separated by ascending
/// document id, and the answer does not depend on the order they were inserted
/// in.
///
/// The fixture is two subjects carrying byte-identical text, so `N = 2`,
/// `avgdl = 2`, `df(alpha) = 2`, and every input to both scores is the same
/// number. The scores are therefore equal in exact arithmetic — not nearly
/// equal — and the order is decided entirely by the tie-break.
///
/// Ascending document id is ascending canonical `(graph, subject, language)`
/// order, because the index assigns ids only after sorting on that key. So the
/// insertion order cannot reach the answer: pushing `ex:b` before `ex:a` still
/// makes `ex:a` document `0`, and still puts it first.
#[test]
fn equal_scores_are_broken_by_document_id_deterministically() {
    let subjects = ["https://example.org/a", "https://example.org/b"];
    let forward = tie_index(subjects);
    let backward = tie_index([subjects[1], subjects[0]]);

    for index in [&forward, &backward] {
        let rows = rank_partition(index, &plain(), &needle("alpha"), None)
            .expect("the fixture scores without overflowing");
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].score, rows[1].score,
            "the fixture is engineered so the two scores are exactly equal"
        );
        assert_eq!(rows[0].score.to_decimal_lexical(), "0.182321556793");
        assert_eq!(
            (rows[0].document, rows[0].partition_rank),
            (0, 1),
            "the lower document id must rank first"
        );
        assert_eq!((rows[1].document, rows[1].partition_rank), (1, 2));
        assert_eq!(
            index.document(0).expect("document 0 exists").subject(),
            &TermValue::iri(subjects[0]),
            "document 0 is the canonically first subject, whatever the insertion order was"
        );
    }
}

/// The `?score` a consumer reads is a decimal of fixed width, and two rows can
/// carry the same one while carrying different `?rank`.
///
/// Two ways round, both pinned:
///
/// * an exact tie — the scores really are equal, and the printed score cannot
///   show the document id that separated them;
/// * a coarser rendering — two genuinely different scores that agree once a
///   consumer keeps fewer fractional digits than separate them.
///
/// In both cases `ORDER BY DESC(?score)` leaves the two rows' relative order
/// undetermined while `ORDER BY ?rank` fixes it. `ORDER BY ?rank` is the
/// reproducing idiom.
#[test]
fn order_by_score_and_order_by_rank_can_disagree_under_rounding() {
    /// `value` rendered with `digits` fractional digits, truncated — what a
    /// consumer that keeps fewer digits than this crate computes would show.
    fn rendered(value: Fixed, digits: usize) -> String {
        let lexical = value.to_decimal_lexical();
        let (integer, fraction) = lexical
            .split_once('.')
            .expect("the lexical form always carries a point");
        if digits == 0 {
            integer.to_owned()
        } else {
            format!("{integer}.{}", &fraction[..digits])
        }
    }

    let tied = tie_index(["https://example.org/a", "https://example.org/b"]);
    let rows = rank_partition(&tied, &plain(), &needle("alpha"), None).expect("scores");
    assert_eq!(
        rows[0].score.to_decimal_lexical(),
        rows[1].score.to_decimal_lexical(),
        "an exact tie reports one score for two rows"
    );
    assert_ne!(
        rows[0].partition_rank, rows[1].partition_rank,
        "but the ranks are still distinct"
    );
    // At every rendering width, not only the full one: an exact tie agrees
    // however many fractional digits a consumer keeps, so `ORDER BY DESC(?score)`
    // cannot separate these two rows at any precision.
    for digits in [0, 1, 6, 12] {
        assert_eq!(
            rendered(rows[0].score, digits),
            rendered(rows[1].score, digits),
            "an exact tie must agree at {digits} fractional digits too"
        );
    }

    let golden = golden_index();
    let rows = rank_partition(&golden, &plain(), &needle("alpha beta"), None).expect("scores");
    assert_ne!(
        rows[0].score, rows[1].score,
        "these two scores really are different"
    );
    assert_eq!(
        rendered(rows[0].score, 0),
        rendered(rows[1].score, 0),
        "and a consumer keeping no fractional digits cannot tell them apart"
    );
    assert_eq!(
        (rows[0].partition_rank, rows[1].partition_rank),
        (1, 2),
        "the rank still can"
    );
}

// ── partitions ───────────────────────────────────────────────────────────────

/// `?rank` counts within a partition, so two languages each have their own rank
/// one. A global rank would order scores computed against different corpora.
#[test]
fn rank_is_per_partition_not_global() {
    let index = index_of(&[
        ("https://example.org/a", "alpha alpha beta", Some("en")),
        ("https://example.org/b", "alpha beta gamma", Some("en")),
        ("https://example.org/c", "alpha alpha beta", Some("fr")),
        ("https://example.org/d", "alpha beta gamma", Some("fr")),
    ]);
    assert_eq!(index.partition_count(), 2);

    let rows = select(&index, &needle("alpha"), &everything(), None, None).expect("scores");
    assert_eq!(rows.len(), 4);
    assert_eq!(
        rows.iter().filter(|row| row.partition_rank == 1).count(),
        2,
        "each partition must have exactly one rank-one row"
    );
    assert_eq!(
        rows.iter()
            .map(|row| row.partition_rank)
            .collect::<Vec<_>>(),
        vec![1, 2, 1, 2],
        "emission is (partition key ASC, rank ASC), so the ranks restart"
    );
    assert_eq!(
        rows.iter().map(|row| row.document).collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
}

/// Filtering partitions before ranking cannot change a surviving row's rank,
/// because ranks are per-partition and no surviving row was ever compared
/// against a dropped one.
#[test]
fn a_partition_filter_selects_without_changing_any_rank() {
    let index = index_of(&[
        ("https://example.org/a", "alpha alpha beta", Some("en")),
        ("https://example.org/b", "alpha beta gamma", Some("en")),
        ("https://example.org/c", "alpha alpha beta", Some("fr")),
        ("https://example.org/d", "alpha beta gamma", Some("fr")),
    ]);
    let everything_rows =
        select(&index, &needle("alpha"), &everything(), None, None).expect("scores");

    let english = everything().with_language(Constraint::Exactly("en".to_owned()));
    let english_rows = select(&index, &needle("alpha"), &english, None, None).expect("scores");

    let expected: Vec<Scored> = everything_rows
        .iter()
        .filter(|row| {
            index
                .partition_key_of(row.document)
                .and_then(PartitionKey::language)
                == Some("en")
        })
        .copied()
        .collect();
    assert_eq!(english_rows, expected);
    assert_eq!(
        english_rows
            .iter()
            .map(|row| row.partition_rank)
            .collect::<Vec<_>>(),
        vec![1, 2],
        "the surviving partition still ranks from one"
    );
}

/// A term common in one language and rare in the other has a different inverse
/// document frequency in each. Pooling the two corpora would give it one.
#[test]
fn statistics_are_not_pooled_across_partitions() {
    let index = index_of(&[
        // English: "alpha" is rare — one document in four.
        ("https://example.org/e1", "alpha one", Some("en")),
        ("https://example.org/e2", "two three", Some("en")),
        ("https://example.org/e3", "four five", Some("en")),
        ("https://example.org/e4", "six seven", Some("en")),
        // French: "alpha" is common — three documents in four.
        ("https://example.org/f1", "alpha un", Some("fr")),
        ("https://example.org/f2", "alpha deux", Some("fr")),
        ("https://example.org/f3", "alpha trois", Some("fr")),
        ("https://example.org/f4", "quatre cinq", Some("fr")),
    ]);

    let english = PartitionKey::new(None, Some("en".to_owned()));
    let french = PartitionKey::new(None, Some("fr".to_owned()));
    assert_eq!(index.document_frequency(&english, "alpha"), 1);
    assert_eq!(index.document_frequency(&french, "alpha"), 3);

    let english_document =
        rank_partition(&index, &english, &needle("alpha"), None).expect("scores")[0].document;
    let french_document =
        rank_partition(&index, &french, &needle("alpha"), None).expect("scores")[0].document;

    let english_idf = explain(&index, english_document, &needle("alpha")).expect("explains")[0]
        .inverse_document_frequency;
    let french_idf = explain(&index, french_document, &needle("alpha")).expect("explains")[0]
        .inverse_document_frequency;

    assert_ne!(
        english_idf, french_idf,
        "a pooled corpus would give the term one inverse document frequency"
    );
    assert!(
        english_idf > french_idf,
        "the rarer term must weigh more: {english_idf:?} vs {french_idf:?}"
    );
    // And the two values themselves, digit for digit, hand-computed from each
    // partition's own `N` and `df`. An ordering assertion is satisfied by two
    // wrong numbers that happen to be ordered, and this file's contract is
    // exactness rather than plausibility.
    //
    // English: N = 4, df = 1, so IDF = ln(1 + 3.5/1.5) = ln(10/3)
    //          = 1.2039728043259361… → 1.203972804325 truncated.
    // French:  N = 4, df = 3, so IDF = ln(1 + 1.5/3.5) = ln(10/7)
    //          = 0.3566749439387324… → 0.356674943938 truncated.
    assert_eq!(
        (
            english_idf.to_decimal_lexical(),
            french_idf.to_decimal_lexical()
        ),
        ("1.203972804325".to_owned(), "0.356674943938".to_owned()),
        "each partition's inverse document frequency is computed from its own corpus"
    );
}

// ── selection ────────────────────────────────────────────────────────────────

/// A deterministic linear congruential generator — a fixture needs a spread of
/// scores, not randomness, and a seeded integer recurrence gives the same spread
/// on every target with no floating point anywhere.
struct Lcg(u64);

impl Lcg {
    /// The next value, in `0..bound`.
    fn next_below(&mut self, bound: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) % bound
    }
}

/// How many of [`spread_index`]'s documents hold at least one of
/// `"alpha beta gamma"`.
///
/// A constant, because the fixture is: [`Lcg`] is a seeded integer recurrence
/// with no floating point in it, so the corpus — and therefore this count — is a
/// pure function of the seed and the vocabulary. Asserting a floor instead would
/// be satisfied by a sweep that had quietly shrunk.
const SPREAD_MATCHES: usize = 50;

/// Sixty documents of varying length over a small vocabulary, in one partition.
fn spread_index() -> TextIndex {
    const VOCABULARY: [&str; 6] = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta"];
    let mut generator = Lcg(0x5eed_1234_9abc_def0);
    let mut subjects = Vec::with_capacity(60);
    let mut texts = Vec::with_capacity(60);
    for id in 0..60_u32 {
        subjects.push(format!("https://example.org/d{id:03}"));
        let length = generator.next_below(8) + 1;
        let mut words = Vec::with_capacity(length as usize);
        for _ in 0..length {
            words.push(VOCABULARY[generator.next_below(VOCABULARY.len() as u64) as usize]);
        }
        texts.push(words.join(" "));
    }
    let rows: Vec<(&str, &str, Option<&str>)> = subjects
        .iter()
        .zip(&texts)
        .map(|(subject, text)| (subject.as_str(), text.as_str(), None))
        .collect();
    index_of(&rows)
}

/// The bounded heap and the full sort produce identical output, swept over every
/// ceiling from none at all to past the end of the result.
///
/// This is the claim the heap path lives or dies on: it is a work reduction, not
/// a different answer. The order is strict and total — document ids are distinct
/// — so there is no pair whose relative order the two paths could legitimately
/// disagree about.
#[test]
fn bounded_heap_equals_the_full_sort_prefix() {
    let index = spread_index();
    let query = needle("alpha beta gamma");
    let full = select(&index, &query, &everything(), None, None).expect("scores");
    assert_eq!(
        full.len(),
        SPREAD_MATCHES,
        "the fixture is a seeded generator with no float in it, so its match count is knowable \
         exactly; a floor here would be satisfied by a sweep that had shrunk to two rows"
    );

    for k in 0..=(full.len() as u64 + 5) {
        let bounded = select(&index, &query, &everything(), Some(k), None).expect("scores");
        let expected = &full[..(k as usize).min(full.len())];
        assert_eq!(
            bounded.as_slice(),
            expected,
            "the heap disagreed at k = {k}"
        );
    }

    // The same sweep straight through the per-partition entry point.
    let all = rank_partition(&index, &plain(), &query, None).expect("scores");
    for k in 0..=(all.len() as u64 + 5) {
        let bounded = rank_partition(&index, &plain(), &query, Some(k)).expect("scores");
        assert_eq!(
            bounded.as_slice(),
            &all[..(k as usize).min(all.len())],
            "rank_partition's heap disagreed at k = {k}"
        );
    }
}

/// A bound `?rank` yields exactly that one per-partition position.
#[test]
fn a_bound_rank_selects_that_rank() {
    let index = golden_index();
    let query = needle("alpha beta");

    let first = select(&index, &query, &everything(), None, Some(1)).expect("scores");
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].document, 0);
    assert_eq!(first[0].partition_rank, 1);
    assert_eq!(first[0].score.to_decimal_lexical(), "1.646224553827");

    let second = select(&index, &query, &everything(), None, Some(2)).expect("scores");
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].document, 1);
    assert_eq!(second[0].partition_rank, 2);
    assert_eq!(second[0].score.to_decimal_lexical(), "1.386294361118");
}

/// A rank past the end of every partition, and the 1-based sequence's absent
/// zero, both yield nothing rather than an error.
#[test]
fn an_out_of_range_rank_yields_nothing() {
    let index = golden_index();
    let query = needle("alpha beta");
    for rank in [0, 3, 99] {
        assert!(
            select(&index, &query, &everything(), None, Some(rank))
                .expect("an absent rank is not a failure")
                .is_empty(),
            "rank {rank} names no row in a two-row partition"
        );
    }
}

/// A bound rank yields one row per admitted partition, not one row overall.
#[test]
fn a_bound_rank_applies_to_every_admitted_partition() {
    let index = index_of(&[
        ("https://example.org/a", "alpha alpha beta", Some("en")),
        ("https://example.org/b", "alpha beta gamma", Some("en")),
        ("https://example.org/c", "alpha alpha beta", Some("fr")),
        ("https://example.org/d", "alpha beta gamma", Some("fr")),
    ]);
    let rows = select(&index, &needle("alpha"), &everything(), None, Some(1)).expect("scores");
    assert_eq!(
        rows.iter().map(|row| row.document).collect::<Vec<_>>(),
        vec![0, 2],
        "each partition contributes its own rank one"
    );

    let ceiling =
        select(&index, &needle("alpha"), &everything(), Some(1), Some(1)).expect("scores");
    // WHICH row survives, not merely how many: a ceiling that truncated from
    // the wrong end would emit the French partition's rank one and still be one
    // row long. Emission is `(partition key ASC, rank ASC)`, so the surviving
    // row is the English partition's.
    assert_eq!(
        ceiling.iter().map(|row| row.document).collect::<Vec<_>>(),
        vec![0],
        "a ceiling keeps the PREFIX of the emission order, not an arbitrary row of it"
    );
}

// ── explain ──────────────────────────────────────────────────────────────────

/// The contributions sum **exactly** to the score. In exact fixed-point
/// arithmetic that is an equality, and asserting it as one is the point: a
/// tolerance here would hide precisely the drift the fixed-point layer exists to
/// rule out.
#[test]
fn explain_contributions_sum_exactly_to_the_score() {
    let index = spread_index();
    let query = needle("alpha beta gamma");
    let rows = select(&index, &query, &everything(), None, None).expect("scores");
    assert_eq!(
        rows.len(),
        SPREAD_MATCHES,
        "the loop below carries the real claim, so a collapse to one row must fail here rather \
         than pass vacuously"
    );

    for row in rows {
        let contributions = explain(&index, row.document, &query).expect("the document exists");
        assert_eq!(
            contributions.len(),
            3,
            "every distinct needle term is reported, present or not"
        );
        let mut total = Fixed::ZERO;
        for contribution in &contributions {
            total = total
                .checked_add(contribution.contribution)
                .expect("the sum is representable");
        }
        assert_eq!(
            total,
            row.score,
            "document {} summed to {} but scored {}",
            row.document,
            total.to_decimal_lexical(),
            row.score.to_decimal_lexical()
        );
    }
}

/// A term the document does not hold is still reported, with a zero frequency
/// and a contribution of exactly zero.
#[test]
fn explain_reports_a_term_the_document_does_not_hold() {
    let index = golden_index();
    let rows = explain(&index, 0, &needle("alpha omicron")).expect("document 0 exists");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].term, "omicron");
    assert_eq!(rows[1].term_frequency, 0);
    assert_eq!(rows[1].document_frequency, 0);
    assert_eq!(rows[1].contribution, Fixed::ZERO);
}

/// Explaining a document the index does not hold is a `Data` error — the caller
/// asked about something that is not there, which is a different thing from a
/// document that scored nothing.
#[test]
fn explaining_an_unknown_document_is_a_data_error() {
    let index = golden_index();
    let error = explain(&index, 999, &needle("alpha")).expect_err("there is no document 999");
    assert!(matches!(error, TextError::Data(_)), "got {error:?}");

    // One past the end is the boundary that matters, and 999 is nowhere near
    // it: a bound written `id >= count - 1` would refuse the LAST document of
    // every partition and this test would still pass. So the neighbouring valid
    // case is the highest id the index holds, and the first id past it.
    let last = u32::try_from(index.document_count() - 1).expect("a small fixture");
    explain(&index, last, &needle("alpha"))
        .unwrap_or_else(|error| panic!("document {last} is the last one the index holds: {error}"));
    let past = last + 1;
    assert!(
        matches!(
            explain(&index, past, &needle("alpha")),
            Err(TextError::Data(_))
        ),
        "document {past} is one past the end and must be refused"
    );
}

// ── the needle ───────────────────────────────────────────────────────────────

/// `matched` counts the **distinct** needle terms present, so a repeated term
/// counts once and an absent one does not count at all.
#[test]
fn matched_counts_distinct_needle_terms_present() {
    let index = golden_index();

    let plain_rows = rank_partition(&index, &plain(), &needle("alpha beta"), None).expect("scores");
    let padded = rank_partition(&index, &plain(), &needle("alpha beta alpha omicron"), None)
        .expect("scores");

    assert_eq!(
        padded.iter().map(|row| row.matched).collect::<Vec<_>>(),
        vec![2, 2],
        "the repeat counts once and the absent term counts not at all"
    );
    assert_eq!(
        padded, plain_rows,
        "a repeated term and a term nobody holds change neither score nor rank"
    );

    // A needle only one of the two documents holds.
    let single = rank_partition(&index, &plain(), &needle("delta"), None).expect("scores");
    assert_eq!(single.len(), 1);
    assert_eq!(single[0].document, 1);
    assert_eq!(single[0].matched, 1);
}

/// A needle that analyzes to no terms at all is an honest empty result, not an
/// error.
///
/// `"--- ... !!!"` is a **well-formed** request: analysis is total, so nothing
/// about it could not be interpreted; it simply names no terms, and a sum over
/// no terms matches no document. The index side already takes the same view —
/// a document whose literals analyze to nothing is excluded rather than failing
/// the build — and the two ends have to agree about what "no text" means. A
/// needle is also usually a bound variable, so an error here would abort a whole
/// SPARQL evaluation over one row of data-dependent input.
#[test]
fn a_needle_analyzing_to_zero_terms() {
    let index = golden_index();
    let empty = needle("--- ... !!!");
    assert!(
        empty.is_empty(),
        "the fixture needle must analyze to nothing"
    );

    assert_eq!(
        rank_partition(&index, &plain(), &empty, None).expect("an empty needle is not a failure"),
        Vec::new()
    );
    assert_eq!(
        select(&index, &empty, &everything(), None, None).expect("nor here"),
        Vec::new()
    );
    assert_eq!(
        explain(&index, 0, &empty).expect("nor here"),
        Vec::new(),
        "a needle with no terms explains nothing about a document that exists"
    );
    assert_eq!(
        rank_partition(&index, &plain(), &Vec::new(), None).expect("an empty slice likewise"),
        Vec::new()
    );
}

/// A partition the index does not hold scores nothing, rather than failing:
/// no document is there, so no document matched.
#[test]
fn an_unknown_partition_scores_nothing() {
    let index = golden_index();
    let absent = PartitionKey::new(None, Some("kl".to_owned()));
    assert!(
        rank_partition(&index, &absent, &needle("alpha"), None)
            .expect("an absent partition is not a failure")
            .is_empty()
    );
}

// ── determinism ──────────────────────────────────────────────────────────────

/// The same index and the same needle produce byte-identical output, every
/// time. This is the crate's headline promise, asserted at the level a caller
/// observes it: the `xsd:decimal` lexical forms, not the internal
/// representation.
#[test]
fn score_is_a_pure_function_of_index_and_needle() {
    let index = spread_index();
    let query = needle("alpha beta gamma");
    let first = select(&index, &query, &everything(), None, None).expect("scores");
    let rendered: Vec<(u32, u32, u32, String)> = first
        .iter()
        .map(|row| {
            (
                row.document,
                row.partition_rank,
                row.matched,
                row.score.to_decimal_lexical(),
            )
        })
        .collect();

    for _ in 0..100 {
        let again = select(&index, &query, &everything(), None, None).expect("scores");
        assert_eq!(again, first);
        let again_rendered: Vec<(u32, u32, u32, String)> = again
            .iter()
            .map(|row| {
                (
                    row.document,
                    row.partition_rank,
                    row.matched,
                    row.score.to_decimal_lexical(),
                )
            })
            .collect();
        assert_eq!(again_rendered, rendered);
    }
}

/// Two independently built indexes over the same content rank identically —
/// including the tie-break, because the document ids they assign agree.
#[test]
fn two_independently_built_indexes_rank_identically() {
    let rows: [(&str, &str, Option<&str>); 3] = [
        ("https://example.org/a", "alpha alpha beta", None),
        ("https://example.org/b", "alpha beta gamma", None),
        ("https://example.org/c", "alpha", None),
    ];
    let forward = index_of(&rows);
    let mut reversed = rows;
    reversed.reverse();
    let backward = index_of(&reversed);

    assert_eq!(forward.fingerprint(), backward.fingerprint());
    let query = needle("alpha beta");
    let ranked = select(&forward, &query, &everything(), None, None).expect("scores");
    // Two empty answers agree, and equal fingerprints do not imply retrieval,
    // so the shape of the answer is pinned before the two are compared.
    assert_eq!(
        ranked
            .iter()
            .map(|row| (row.document, row.partition_rank, row.matched))
            .collect::<Vec<_>>(),
        vec![(0, 1, 2), (1, 2, 2), (2, 3, 1)],
        "all three documents hold a needle term, in this rank order"
    );
    assert_eq!(
        select(&backward, &query, &everything(), None, None).expect("scores"),
        ranked
    );
}
