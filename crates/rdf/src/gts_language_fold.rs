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

use purrdf_core::{LossEntry, LossLedger, RdfLocation};
use purrdf_iri::langtag::is_identity_folded;
use purrdf_lex::allocation::{Admission, Memory, StorageError};

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

/// Record the loss for a language-tagged literal whose written tag is not
/// already lower case into `losses`, admitting the entry's note, location and
/// slot through `memory`. A tag the dataset holds as written allocates nothing
/// and records nothing.
///
/// The note names the literal, the written tag and the tag the dataset holds,
/// so the term the fold merges with its case variant is identified.
///
/// # Errors
/// The storage refusal of `memory`; the ledger is unchanged.
pub(crate) fn record_language_fold_loss<S: Admission + ?Sized>(
    losses: &mut LossLedger,
    lexical: &str,
    tag: &str,
    location: RdfLocation,
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
    let location = memory.boxed(location)?;
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
    use purrdf_core::{LossLedger, RdfLocation};
    use purrdf_gts::model::{Graph, Term};
    use purrdf_lex::allocation::{Ceiling, Memory, StorageError};

    use super::record_language_fold_loss;

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

        let lower = crate::import_gts_events(&bytes(&["en-us", "fr"])).expect("import");
        assert!(lower.envelope.losses.is_empty());
        let graph = crate::import_gts_graph(crate::gts::read_graph(&variant, true).expect("read"))
            .expect("graph import");
        assert_eq!(graph.envelope.losses.entries().len(), 1);
    }

    /// The entry's note, location and slot are admitted through the caller's
    /// memory: a budget too small for them is the typed storage refusal and the
    /// ledger stays empty; a sufficient budget records the entry; a tag the
    /// dataset holds as written needs no budget at all.
    #[test]
    fn the_loss_entry_is_admitted_through_the_callers_memory() {
        let record = |tag: &str, budget: usize| {
            let mut losses = LossLedger::new();
            let mut ceiling = Ceiling(budget);
            let result = record_language_fold_loss(
                &mut losses,
                "x",
                tag,
                RdfLocation::logical("gts:sink"),
                &mut Memory::new(&mut ceiling),
            );
            (result, losses.entries().len())
        };
        let (refused, held) = record("EN-us", 8);
        assert!(
            matches!(refused, Err(StorageError::AdmissionFailed)),
            "{refused:?}"
        );
        assert_eq!(held, 0);
        let (admitted, held) = record("EN-us", 64 * 1024);
        assert!(admitted.is_ok());
        assert_eq!(held, 1);
        let (folded, held) = record("en-us", 0);
        assert!(folded.is_ok());
        assert_eq!(held, 0);
    }
}
