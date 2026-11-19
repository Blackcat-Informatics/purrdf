// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The loss a GTS read incurs when a language tag is folded.
//!
//! The GTS format compares a language text whole and bytewise, so `"x"@EN-us`
//! and `"x"@en-us` are two terms of one file. A dataset folds a tag to ASCII
//! lower case in its term interner and holds one term. The importers record
//! each affected literal as `language-tag-case-folded` in the envelope's loss
//! ledger.

use purrdf_core::{LossEntry, RdfLocation};
use purrdf_iri::langtag::{identity_fold, is_identity_folded};

/// The loss entry for a language-tagged literal whose written tag is not
/// already lower case, or `None` when the dataset holds the term as written.
///
/// The note names the literal, the written tag and the tag the dataset holds,
/// so the term the fold merges with its case variant is identified.
pub(crate) fn language_fold_loss(
    lexical: &str,
    tag: &str,
    location: RdfLocation,
) -> Option<LossEntry> {
    if is_identity_folded(tag) {
        return None;
    }
    Some(LossEntry {
        code: "language-tag-case-folded".into(),
        from: "gts".into(),
        to: "rdf-1.2-dataset".into(),
        note: format!(
            "the literal {lexical:?} with written language tag {tag:?} is held as {:?}; \
             a term of this file that differs only in tag case is the same dataset term",
            identity_fold(tag)
        )
        .into(),
        location: Some(Box::new(location)),
    })
}

#[cfg(test)]
mod tests {
    use purrdf_gts::model::{Graph, Term};

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
}
