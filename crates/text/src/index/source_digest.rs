// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Order-independent, multiplicity-sensitive selected-source accounting.

use purrdf_hash::multiset::Multiset256;

use super::{CorpusProfile, SOURCE_DIGEST_DOMAIN, SourceRow, TRIPLE_SOURCE_DIGEST_DOMAIN};

/// Sum of the selected row hashes in Z/(2^256), plus their exact population.
///
/// A repeated row contributes repeatedly; unlike XOR, two equal rows do not
/// disappear. The sum's encoding is explicitly little-endian on every target.
/// This is a multiset fingerprint, not an authentication proof. The observed
/// source summary must still be supplied independently of the proposed delta.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct SourceDigest {
    state: Multiset256,
}

impl SourceDigest {
    pub(super) const fn from_state(state: Multiset256) -> Self {
        Self { state }
    }

    /// Apply a literal observed by the original source owner, independently of
    /// the consumer's requested delta. Nonliteral coverage is handled separately.
    pub(super) fn observe<P: CorpusProfile>(
        &mut self,
        side: purrdf_core::source::SourceSide,
        row: purrdf_core::source::SourceRowRef<'_>,
    ) -> Result<(), crate::TextError> {
        let purrdf_core::TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } = row.object
        else {
            return Ok(());
        };
        let mut resident = purrdf_lex::allocation::Resident;
        let mut memory = purrdf_lex::allocation::Memory::new(&mut resident);
        let hash = purrdf_core::source::LiteralSourceRow {
            graph: row.graph,
            subject: row.subject,
            predicate: row.predicate,
            lexical: lexical_form,
            datatype,
            language: language.as_deref(),
            direction: *direction,
        }
        .hashes_with_memory(&mut memory)
        .map_err(|error| crate::TextError::data(error.to_string()))?[usize::from(P::GRAPH_FREE)];
        match side {
            purrdf_core::source::SourceSide::Previous => self.state.remove(hash),
            purrdf_core::source::SourceSide::Next => self.state.insert(hash),
        }
        .map_err(|error| crate::TextError::data(error.to_string()))
    }

    pub(super) fn from_rows<'a, P: CorpusProfile + 'a>(
        rows: impl IntoIterator<Item = &'a SourceRow<P>>,
    ) -> Self {
        let mut digest = Self::default();
        for hash in row_hashes(rows) {
            digest
                .state
                .insert(hash)
                .expect("a finite source population fits u128");
        }
        digest
    }

    pub(super) fn insert<P: CorpusProfile>(&mut self, row: &SourceRow<P>) {
        self.state
            .insert(row_hash(row))
            .expect("a finite source population fits u128");
    }

    /// The caller establishes row membership before subtracting it. Population
    /// alone cannot prove membership in a multiset.
    pub(super) fn remove<P: CorpusProfile>(&mut self, row: &SourceRow<P>) {
        self.state
            .remove(row_hash(row))
            .expect("source membership was validated");
    }

    pub(super) fn fingerprint<P: CorpusProfile>(&self) -> [u8; 32] {
        self.state.fingerprint(if P::GRAPH_FREE {
            TRIPLE_SOURCE_DIGEST_DOMAIN
        } else {
            SOURCE_DIGEST_DOMAIN
        })
    }
}

fn row_hash<P: CorpusProfile>(row: &SourceRow<P>) -> [u8; 32] {
    row_hashes(std::iter::once(row))
        .next()
        .expect("one source row was supplied")
}

/// Use the original core row law so independently sealed native metadata and
/// selected text rows consume exactly the same preimages. No literal text is
/// copied into an owned term or output buffer.
fn row_hashes<'a, P: CorpusProfile + 'a>(
    rows: impl IntoIterator<Item = &'a SourceRow<P>>,
) -> impl Iterator<Item = [u8; 32]> {
    rows.into_iter().map(|row| {
        let mut resident = purrdf_lex::allocation::Resident;
        let mut memory = purrdf_lex::allocation::Memory::new(&mut resident);
        purrdf_core::source::LiteralSourceRow {
            graph: P::graph(&row.graph),
            subject: &row.subject,
            predicate: &row.predicate,
            lexical: &row.lexical_form,
            datatype: &row.datatype,
            language: row.language.as_deref(),
            direction: row.direction,
        }
        .hashes_with_memory(&mut memory)
        .expect("resident source row traversal storage")[usize::from(P::GRAPH_FREE)]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GraphLanguage;
    use purrdf_core::TermValue;

    fn row() -> SourceRow<GraphLanguage> {
        SourceRow {
            graph: None,
            subject: TermValue::iri("urn:subject"),
            predicate: TermValue::iri("urn:predicate"),
            datatype: purrdf_core::datatype::XSD_STRING.to_owned(),
            lexical_form: "river\0bank".to_owned(),
            language: None,
            direction: None,
        }
    }

    #[test]
    fn source_summary_reverses_changes_and_preserves_multiplicity() {
        let first = row();
        let mut second = first.clone();
        second.lexical_form = "valley".to_owned();
        let old = SourceDigest::from_rows([&first, &first, &second]);
        assert_eq!(old, SourceDigest::from_rows([&second, &first, &first]));
        assert_ne!(old, SourceDigest::from_rows([&second]));
        let mut changed = old;
        changed.remove(&first);
        assert_eq!(changed, SourceDigest::from_rows([&second, &first]));
        changed.remove(&first);
        assert_eq!(changed, SourceDigest::from_rows([&second]));
        changed.insert(&first);
        changed.insert(&first);
        assert_eq!(changed, old);
        changed.remove(&first);
        changed.remove(&first);
        changed.remove(&second);
        assert_eq!(changed, SourceDigest::default());
    }

    #[test]
    fn every_source_coordinate_participates_in_the_row_identity() {
        let original = row();
        let expected = row_hash(&original);
        let mut variants = vec![original; 7];
        variants[0].graph = Some(TermValue::iri("urn:graph"));
        variants[1].subject = TermValue::iri("urn:other-subject");
        variants[2].predicate = TermValue::iri("urn:other-predicate");
        variants[3].datatype = purrdf_core::datatype::XSD_INTEGER.to_owned();
        variants[4].lexical_form.push('\0');
        variants[5].language = Some("en".to_owned());
        variants[6].direction = Some(purrdf_core::RdfTextDirection::Rtl);
        for variant in variants {
            assert_ne!(row_hash(&variant), expected);
        }
    }

    #[test]
    fn empty_source_profiles_have_distinct_identities() {
        let empty = SourceDigest::default();
        assert_ne!(
            empty.fingerprint::<GraphLanguage>(),
            empty.fingerprint::<crate::GraphFree>()
        );
    }
}
