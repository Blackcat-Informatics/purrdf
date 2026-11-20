// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The loss a GTS read incurs when a language tag is folded.
//!
//! The GTS format compares a language text whole and bytewise, so `"x"@EN-us`
//! and `"x"@en-us` are two terms of one file. A dataset folds a tag to ASCII
//! lower case in its term interner and holds one term. The importers record
//! each affected literal as `language-tag-case-folded` in the envelope's loss
//! ledger.

use std::fmt;
use std::hash::BuildHasher as _;

use purrdf_core::{LossEntry, LossLedger, RdfLocation};
use purrdf_iri::langtag::is_identity_folded;
use purrdf_lex::allocation::{Admission, Memory, StorageError};

/// The written case form of a tag.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Form {
    Lower,
    Upper,
    Mixed,
}

/// The case forms of the language-tagged literals a read produced, one per distinct literal.
///
/// A literal is its lexical form, its folded tag and its direction, so case variants of one
/// tag and repeats across segments are one literal. It is kept under the greatest form any
/// of its written tags has, so the figure does not depend on read order. A literal is
/// identified by a 128-bit fixed-key hash of that identity, not by a copy of its text.
#[derive(Default)]
pub(crate) struct LanguageFormCensus {
    forms: purrdf_core::FastMap<u128, Form>,
}

impl LanguageFormCensus {
    /// Count the literal `lexical` written with `tag` and `direction`.
    pub(crate) fn record<D: std::hash::Hash>(&mut self, lexical: &str, tag: &str, direction: &D) {
        let upper = tag.bytes().any(|b| b.is_ascii_uppercase());
        let lower = tag.bytes().any(|b| b.is_ascii_lowercase());
        let form = match (upper, lower) {
            (false, _) => Form::Lower,
            (true, false) => Form::Upper,
            (true, true) => Form::Mixed,
        };
        let folded = purrdf_iri::langtag::identity_fold(tag);
        let state = purrdf_core::FastHasher::default();
        let half =
            |salt: u8| u128::from(state.hash_one((salt, lexical, folded.as_str(), direction)));
        let key = half(0) << 64 | half(1);
        let slot = self.forms.entry(key).or_insert(form);
        *slot = (*slot).max(form);
    }

    /// The counts of the literals recorded.
    pub(crate) fn finish(&self) -> purrdf_core::LanguageTagForms {
        let count =
            |wanted: Form| self.forms.values().filter(|&&form| form == wanted).count() as u64;
        purrdf_core::LanguageTagForms::new(
            count(Form::Lower),
            count(Form::Upper),
            count(Form::Mixed),
        )
    }
}

/// A tag as the dataset holds it, written without building the folded copy.
struct Folded<'a>(&'a str);

impl fmt::Debug for Folded<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("\"")?;
        for c in self.0.chars() {
            write!(f, "{}", c.to_ascii_lowercase().escape_debug())?;
        }
        f.write_str("\"")
    }
}

/// Where a GTS literal sits: the logical label of the reader, the segment when
/// the reader is per segment, and the GTS term id.
#[derive(Clone, Copy)]
pub(crate) struct FoldSite {
    pub(crate) logical: &'static str,
    pub(crate) segment: Option<u64>,
    pub(crate) term: u64,
}

/// Record the loss for a language-tagged literal whose written tag is not
/// already lower case into `losses`. A tag the dataset holds as written returns
/// before anything is built or allocated.
///
/// Otherwise the note text, the location's label text and the boxed location
/// are admitted through `memory`, and the ledger slot through
/// `record_with_memory`. The numeric location fields and the entry's static
/// code and pair names hold no allocation. The note names the literal, the
/// written tag and the tag the dataset holds, so the term the fold merges with
/// its case variant is identified.
///
/// # Errors
/// The storage refusal of `memory`; the ledger is unchanged.
pub(crate) fn record_language_fold_loss<S: Admission + ?Sized>(
    losses: &mut LossLedger,
    lexical: &str,
    tag: &str,
    site: FoldSite,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    if is_identity_folded(tag) {
        return Ok(());
    }
    let note = memory.format(&format_args!(
        "the literal {lexical:?} with written language tag {tag:?} is held as {:?}; \
         a term of this file that differs only in tag case is the same dataset term",
        Folded(tag)
    ))?;
    let logical = memory.string(site.logical)?;
    let location = memory.boxed(RdfLocation {
        logical: Some(logical),
        gts_segment_index: site.segment,
        gts_term_id: Some(site.term),
        ..RdfLocation::default()
    })?;
    losses.record_with_memory(
        LossEntry {
            code: "language-tag-case-folded".into(),
            from: "gts".into(),
            to: "rdf-1.2-dataset".into(),
            note: note.into(),
            location: Some(location),
        },
        memory,
    )
}

#[cfg(test)]
mod tests {
    use purrdf_core::LossLedger;
    use purrdf_gts::model::{Graph, Term};
    use purrdf_lex::allocation::{Ceiling, Memory, StorageError};

    use super::{FoldSite, record_language_fold_loss};

    fn graph_with(tags: &[&str]) -> Graph {
        let mut graph = Graph::default();
        graph.terms.push(Term::iri("http://example.org/s"));
        graph.terms.push(Term::iri("http://example.org/p"));
        for (i, tag) in tags.iter().enumerate() {
            graph.terms.push(Term::lang_literal("x", *tag));
            graph.quads.push((0, 1, 2 + i, None));
        }
        graph
    }

    /// Reading a case-variant file into a dataset reports the written tag in the
    /// loss ledger instead of merging silently; an all-lower-case file reports
    /// nothing.
    #[test]
    fn reading_a_case_variant_file_into_a_dataset_reports_the_fold() {
        let bytes = |tags: &[&str]| {
            purrdf_gts::writer::Writer::deterministic(&graph_with(tags), "purrdf-test")
                .expect("writer")
                .to_bytes()
        };
        let variant = bytes(&["EN-us", "en-us"]);
        let bundle = crate::import_gts_events(&variant).expect("import");
        let losses = bundle.envelope.losses.entries();
        assert_eq!(losses.len(), 1);
        assert_eq!(losses[0].code, "language-tag-case-folded");
        assert!(losses[0].note.contains("\"EN-us\"") && losses[0].note.contains("\"en-us\""));
        assert!(
            purrdf_core::check_ledger_sound(&bundle.envelope.losses, "gts", "rdf-1.2-dataset")
                .is_ok()
        );

        let forms = bundle.envelope.language_tag_forms;
        // One literal under two case variants of its tag counts once, as mixed.
        assert_eq!((forms.lower, forms.upper, forms.mixed), (0, 0, 1));
        let shout = crate::import_gts_events(&bytes(&["EN", "En-us"])).expect("import");
        let forms = shout.envelope.language_tag_forms;
        assert_eq!((forms.lower, forms.upper, forms.mixed), (0, 1, 1));

        let lower = crate::import_gts_events(&bytes(&["en-us", "fr"])).expect("import");
        assert!(lower.envelope.losses.is_empty());
        assert_eq!(lower.envelope.language_tag_forms.lower, 2);
        let graph = crate::import_gts_graph(crate::gts::read_graph(&variant, true).expect("read"))
            .expect("graph import");
        assert_eq!(graph.envelope.losses.entries().len(), 1);
    }

    /// The note, the location's label text, the boxed location and the ledger
    /// slot are admitted through the caller's memory. A budget that fits the note
    /// and a short label but not a long label is the typed storage refusal and
    /// leaves the ledger empty; the same budget admits the short label; a tag
    /// the dataset holds as written needs no budget at all.
    #[test]
    fn the_loss_entry_is_admitted_through_the_callers_memory() {
        let long: &'static str = Box::leak("g".repeat(4096).into_boxed_str());
        let site = |logical| FoldSite {
            logical,
            segment: Some(0),
            term: 3,
        };
        let record = |tag: &str, logical, budget: usize| {
            let mut losses = LossLedger::new();
            let mut ceiling = Ceiling(budget);
            let result = record_language_fold_loss(
                &mut losses,
                "x",
                tag,
                site(logical),
                &mut Memory::new(&mut ceiling),
            );
            (result, losses.entries().len())
        };
        let (refused, held) = record("EN-us", "gts:sink", 8);
        assert!(
            matches!(refused, Err(StorageError::AdmissionFailed)),
            "{refused:?}"
        );
        assert_eq!(held, 0);
        let (admitted, held) = record("EN-us", "gts:sink", 1024);
        assert!(admitted.is_ok(), "{admitted:?}");
        assert_eq!(held, 1);
        let (refused, held) = record("EN-us", long, 1024);
        assert!(
            matches!(refused, Err(StorageError::AdmissionFailed)),
            "the label text is charged: {refused:?}"
        );
        assert_eq!(held, 0);
        let (folded, held) = record("en-us", long, 0);
        assert!(folded.is_ok());
        assert_eq!(held, 0);
    }
}
