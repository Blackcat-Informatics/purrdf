// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Order-independent, multiplicity-sensitive selected-source accounting.

use purrdf_hash::multiset::{Multiset256, MultisetError};

use super::{
    CorpusProfile, SOURCE_DIGEST_DOMAIN, SOURCE_IDENTITY_DOMAIN, Shared, SourceRow,
    TRIPLE_SOURCE_DIGEST_DOMAIN, TRIPLE_SOURCE_IDENTITY_DOMAIN, share_index,
};
use purrdf_core::TermValue;
use purrdf_core::source::{LiteralSourceRow, SourceRowRef, SourceSide};

/// Sum of the selected row hashes in Z/(2^256), plus their exact population.
///
/// A repeated row contributes repeatedly; unlike XOR, two equal rows do not
/// disappear. The sum's encoding is explicitly little-endian on every target.
/// This is a population and drift check between structures this process
/// built, not an identity: the index's source identity is BLAKE3 over the
/// sorted row directory. The observed source summary must still be supplied
/// independently of the proposed delta.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct SourceDigest {
    state: Multiset256,
}

impl SourceDigest {
    pub(super) const fn from_state(state: Multiset256) -> Self {
        Self { state }
    }

    pub(super) const fn population(self) -> u128 {
        self.state.population()
    }

    /// Update diagnostic accounting after independently reading a change.
    pub(super) fn observe(
        &mut self,
        side: SourceSide,
        digest: [u8; 32],
    ) -> Result<(), MultisetError> {
        match side {
            SourceSide::Previous => self.state.remove(digest),
            SourceSide::Next => self.state.insert(digest),
        }
    }

    #[cfg(test)]
    pub(super) fn from_rows<'a, P: CorpusProfile + 'a>(
        rows: impl IntoIterator<Item = &'a SourceRow<P>>,
    ) -> Self {
        Self::from_hashes(row_hashes(rows))
    }

    /// Sum already computed row hashes, so a caller that also needs them for
    /// another purpose hashes each row once.
    pub(super) fn from_hashes(hashes: impl IntoIterator<Item = [u8; 32]>) -> Self {
        let mut digest = Self::default();
        for hash in hashes {
            digest.insert_hash(hash);
        }
        digest
    }

    pub(super) fn insert_hash(&mut self, hash: [u8; 32]) {
        self.state
            .insert(hash)
            .expect("a finite source population fits u128");
    }

    /// The caller establishes row membership before subtracting it.
    pub(super) fn remove_hash(&mut self, hash: [u8; 32]) {
        self.state
            .remove(hash)
            .expect("source membership was validated");
    }

    #[cfg(test)]
    pub(super) fn insert<P: CorpusProfile>(&mut self, row: &SourceRow<P>) {
        self.state
            .insert(row_hash(row))
            .expect("a finite source population fits u128");
    }

    /// The caller establishes row membership before subtracting it. Population
    /// alone cannot prove membership in a multiset.
    #[cfg(test)]
    pub(super) fn remove<P: CorpusProfile>(&mut self, row: &SourceRow<P>) {
        self.state
            .remove(row_hash(row))
            .expect("source membership was validated");
    }

    /// The accounting check value a delta mismatch reports. It is a drift
    /// check between structures this process built, not a source identity.
    pub(super) fn check_value<P: CorpusProfile>(&self) -> [u8; 32] {
        additive_check_value::<P>(self.state)
    }
}

/// Digest the literal read through the source-owned change feed in the same
/// law as a selected text row. Nonliteral records participate in coverage only.
pub(super) fn observed_hash<P: CorpusProfile>(
    row: SourceRowRef<'_>,
) -> Result<Option<[u8; 32]>, crate::TextError> {
    let TermValue::Literal {
        lexical_form,
        datatype,
        language,
        direction,
    } = row.object
    else {
        return Ok(None);
    };
    let row = LiteralSourceRow {
        graph: row.graph,
        subject: row.subject,
        predicate: row.predicate,
        lexical: lexical_form,
        datatype,
        language: language.as_deref(),
        direction: *direction,
    };
    literal_hash::<P>(&row).map(Some)
}

fn additive_check_value<P: CorpusProfile>(state: Multiset256) -> [u8; 32] {
    state.accounting_check(if P::GRAPH_FREE {
        TRIPLE_SOURCE_DIGEST_DOMAIN
    } else {
        SOURCE_DIGEST_DOMAIN
    })
}

/// The identity of the selected source rows: BLAKE3 over every selected
/// literal-row hash in ascending order, one per occurrence, under the
/// profile's source domain. `sorted` must be ascending; the row directory
/// already is.
pub(super) fn sorted_source_fingerprint<P: CorpusProfile>(
    sorted: impl IntoIterator<Item = [u8; 32]>,
) -> [u8; 32] {
    purrdf_hash::multiset::sorted_fingerprint(
        if P::GRAPH_FREE {
            TRIPLE_SOURCE_IDENTITY_DOMAIN
        } else {
            SOURCE_IDENTITY_DOMAIN
        },
        sorted,
    )
}

#[derive(Clone, Debug)]
pub(super) struct SourceEntry {
    pub(super) digest: [u8; 32],
    pub(super) subject: Shared<TermValue>,
}

impl SourceEntry {
    pub(super) fn new(digest: [u8; 32], subject: TermValue) -> Result<Self, crate::TextError> {
        Ok(Self {
            digest,
            subject: share_index(subject)?,
        })
    }
}

pub(super) fn row_directory<P: CorpusProfile>(
    rows: Vec<SourceRow<P>>,
) -> Result<super::Table<SourceEntry>, crate::TextError> {
    let mut entries = super::token_streams::handles(rows.len())?;
    for row in rows {
        let digest = try_row_hash(&row)?;
        entries.push(SourceEntry::new(digest, row.subject)?);
    }
    entries.sort_unstable_by_key(|entry| entry.digest);
    super::Table::from_streams(entries)
}

#[cfg(test)]
fn row_hash<P: CorpusProfile>(row: &SourceRow<P>) -> [u8; 32] {
    try_row_hash(row).expect("fixture source row traversal storage")
}

pub(super) fn try_row_hash<P: CorpusProfile>(
    row: &SourceRow<P>,
) -> Result<[u8; 32], crate::TextError> {
    literal_hash::<P>(&LiteralSourceRow {
        graph: P::graph(&row.graph),
        subject: &row.subject,
        predicate: &row.predicate,
        lexical: &row.lexical_form,
        datatype: &row.datatype,
        language: row.language.as_deref(),
        direction: row.direction,
    })
}

fn literal_hash<P: CorpusProfile>(
    row: &LiteralSourceRow<'_>,
) -> Result<[u8; 32], crate::TextError> {
    let mut resident = purrdf_lex::allocation::Resident;
    let mut memory = purrdf_lex::allocation::Memory::new(&mut resident);
    let hashes = row
        .hashes_with_memory(&mut memory)
        .map_err(crate::TextError::source_read)?;
    Ok(hashes[usize::from(P::GRAPH_FREE)])
}

/// Use the original core row law so independently sealed native metadata and
/// selected text rows consume exactly the same preimages. No literal text is
/// copied into an owned term or output buffer.
pub(super) fn row_hashes<'a, P: CorpusProfile + 'a>(
    rows: impl IntoIterator<Item = &'a SourceRow<P>>,
) -> impl Iterator<Item = [u8; 32]> {
    rows.into_iter()
        .map(|row| try_row_hash(row).expect("resident source row traversal storage"))
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
        assert_ne!(
            sorted_source_fingerprint::<GraphLanguage>([]),
            sorted_source_fingerprint::<crate::GraphFree>([])
        );
    }

    /// The source identity is BLAKE3 over the sorted row hashes: two selected
    /// row sets whose additive sums and populations agree have different
    /// identities, while the same rows reached in another order agree.
    #[test]
    fn source_identity_distinguishes_an_additive_collision() {
        let digest = |seed: u8| *purrdf_hash::blake3::hash(&[seed]).as_bytes();
        let (a, b, c) = (digest(1), digest(2), digest(3));
        let mut d = Multiset256::default();
        d.insert(a).unwrap();
        d.insert(b).unwrap();
        d.remove(c).unwrap();
        let d = *d.sum();
        let sorted = |mut hashes: Vec<[u8; 32]>| {
            hashes.sort_unstable();
            hashes
        };
        let (left, right) = (sorted(vec![a, b]), sorted(vec![c, d]));
        assert_eq!(
            SourceDigest::from_hashes(left.iter().copied()),
            SourceDigest::from_hashes(right.iter().copied())
        );
        let identity = |hashes: &[[u8; 32]]| {
            sorted_source_fingerprint::<GraphLanguage>(hashes.iter().copied())
        };
        assert_ne!(identity(&left), identity(&right));
        assert_eq!(identity(&left), identity(&sorted(vec![b, a])));
    }
}
