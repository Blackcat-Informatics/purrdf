// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Immutable auxiliary retrieval over shared, source-aligned projections.
//! Scalar grams and phonetic codes have no lexical BM25 counts or positions.

use crate::analysis::{AlignedText, Analysis, Projection};
use crate::{
    Analyzer, TextError,
    phonetic::{self, DoubleMetaphone, PhoneticCodes, PhoneticRefusal, PreparedDistance},
};
use purrdf_hash::{Domain, blake3, frame::frame_le_into};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

mod delta;
pub(crate) mod postings;
mod substring;
use postings::{FlatPostings, GramPosting, SpanShape};
pub use substring::{
    SubstringLimits, SubstringRefusal, SubstringRefusalReason, SubstringReport, SubstringWork,
};

const GENERATION_DOMAIN: Domain = Domain::new(b"purrdf-text/surface-generation/v1\0");
const LITERAL_DOMAIN: Domain = Domain::new(b"purrdf-text/surface-literal/v1\0");

/// The projection in which match offsets are expressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MatchProjection {
    /// A complete pre-stem surface word, used by phonetic refinement.
    SurfaceWord,
    /// A punctuation-preserving bounded span, used by exact substrings.
    SubstringSpan,
}

/// One occurrence traced to its original literal, without conflating offsets.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SourceMatchEvidence {
    /// Document identifier, resolving to the owning index's document table.
    pub document: u32,
    /// Caller-supplied stable source-row identity, or the standalone literal hash.
    pub literal: [u8; 32],
    /// Which analyzer projection matched.
    pub projection: MatchProjection,
    /// Byte range in the source's shared normalized buffer.
    pub projected_range: Range<usize>,
    /// Sorted merged original UTF-8 contributors, possibly noncontiguous.
    pub sources: Vec<Range<usize>>,
    /// Enclosing original UTF-8 range, including gaps between contributors.
    pub highlight: Range<usize>,
    /// Whether correspondence is whole-projection rather than per-character.
    pub coarse: bool,
    /// Exact resolved analyzer identity.
    pub analyzer: [u8; 32],
    /// Identity of the immutable index generation producing this evidence.
    pub generation: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Occurrence {
    source: usize,
    range: Range<usize>,
    contributors: Vec<Range<usize>>,
    highlight: Range<usize>,
    coarse: bool,
}
#[derive(Clone, Debug)]
struct Source {
    document: u32,
    identity: [u8; 32],
    normalized: AlignedText,
}

/// A normalized projection and its sorted distinct document membership.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceTerm {
    text: String,
    documents: Vec<u32>,
    occurrences: Vec<Occurrence>,
}
impl SurfaceTerm {
    /// The complete bounded normalized spelling.
    pub fn text(&self) -> &str {
        &self.text
    }
    /// Sorted distinct document identifiers.
    pub fn documents(&self) -> &[u32] {
        &self.documents
    }
}

/// Exact overlapping substring occurrences, plus original-source evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubstringMatch<'a> {
    /// The matching distinct normalized span.
    pub term: &'a SurfaceTerm,
    /// UTF-8 byte ranges relative to `term`, in increasing start order.
    pub ranges: Vec<Range<usize>>,
    /// Original literal occurrences and their exact source contributors.
    pub evidence: Vec<SourceMatchEvidence>,
}

/// Shared pronunciation codes refined by exact canonical spelling distance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhoneticMatch<'a> {
    /// The pre-stem surface word and its document membership.
    pub term: &'a SurfaceTerm,
    /// Exact unit-cost distance between canonical widened spellings.
    pub distance: usize,
    /// Candidate primary and alternate codes, including equal-code cases.
    pub codes: &'a PhoneticCodes,
    /// Shared nonempty codes, sorted and distinct.
    pub shared_codes: Vec<String>,
    /// Original word occurrences, with their complete source contributors.
    pub evidence: Vec<SourceMatchEvidence>,
}

/// Compact positional gram and phonetic indexes over immutable projections.
///
/// Distinct terms sort by UTF-8 bytes. Gram keys pack one to three 21-bit
/// scalars into a u64, with separate indexes by width. Postings occupy flat
/// sorted arrays; no string is allocated for a gram. Original normalized text
/// and alignment are stored once per source literal.
#[derive(Clone, Debug)]
pub struct SurfaceIndex {
    analyzer: Analyzer,
    generation: [u8; 32],
    words: Vec<SurfaceTerm>,
    spans: Vec<SurfaceTerm>,
    sources: Vec<Source>,
    shapes: Vec<SpanShape>,
    grams: [FlatPostings<u64, GramPosting>; 3],
    codes: Vec<Option<PhoneticCodes>>,
    canonical: Vec<Option<String>>,
    phonetic_postings: FlatPostings<String, u32>,
}

impl SurfaceIndex {
    /// Analyze every original literal once, retaining source alignment.
    /// Repeated identical `(document, text)` rows coalesce deterministically.
    ///
    /// # Errors
    /// Propagates analysis refusals and invalid index-size errors atomically.
    pub fn from_texts<'a>(
        analyzer: Analyzer,
        texts: impl IntoIterator<Item = (u32, &'a str)>,
    ) -> Result<Self, TextError> {
        let rows = texts
            .into_iter()
            .map(|(document, text)| {
                let mut digest = blake3::Hasher::new();
                digest.update(LITERAL_DOMAIN.as_bytes());
                frame_le_into(&mut digest, &document.to_le_bytes());
                frame_le_into(&mut digest, text.as_bytes());
                Ok((
                    document,
                    *digest.finalize().as_bytes(),
                    analyzer.projections(text)?,
                ))
            })
            .collect::<Result<Vec<_>, TextError>>()?;
        Self::from_analyses(analyzer, rows)
    }

    /// Build from shared analyses, preserving caller-supplied source-row identity.
    /// A source identity must identify exactly one analysis in a document.
    ///
    /// # Errors
    /// Refuses conflicting source identities, invalid projection ranges, empty
    /// terms and an index exceeding the u32 distinct-term address space.
    pub(crate) fn from_analyses(
        analyzer: Analyzer,
        rows: impl IntoIterator<Item = (u32, [u8; 32], Analysis)>,
    ) -> Result<Self, TextError> {
        let mut rows: Vec<_> = rows.into_iter().collect();
        rows.sort_by_key(|(document, identity, _)| (*document, *identity));
        for pair in rows.windows(2) {
            if pair[0].0 == pair[1].0 && pair[0].1 == pair[1].1 && pair[0].2 != pair[1].2 {
                return Err(TextError::data(
                    "one source identity names conflicting analyses",
                ));
            }
        }
        rows.dedup_by(|right, left| right.0 == left.0 && right.1 == left.1);
        let mut sources = Vec::with_capacity(rows.len());
        let mut words = BTreeMap::new();
        let mut spans = BTreeMap::new();
        for (document, identity, analysis) in rows {
            let source = sources.len();
            for projection in analysis.surface {
                add_projection(
                    &mut words,
                    projection,
                    document,
                    source,
                    &analysis.normalized,
                )?;
            }
            for projection in analysis.spans {
                add_projection(
                    &mut spans,
                    projection,
                    document,
                    source,
                    &analysis.normalized,
                )?;
            }
            sources.push(Source {
                document,
                identity,
                normalized: analysis.normalized,
            });
        }
        Self::assemble(analyzer, finish(words), finish(spans), sources)
    }

    fn assemble(
        analyzer: Analyzer,
        words: Vec<SurfaceTerm>,
        spans: Vec<SurfaceTerm>,
        sources: Vec<Source>,
    ) -> Result<Self, TextError> {
        if words.iter().chain(&spans).any(|term| term.text.is_empty()) {
            return Err(TextError::data(
                "surface indexes cannot contain empty terms",
            ));
        }
        if u32::try_from(words.len()).is_err() || u32::try_from(spans.len()).is_err() {
            return Err(TextError::data(
                "surface index exceeds u32 distinct-term identifiers",
            ));
        }
        let (grams, shapes) = postings::build_grams(&spans);
        let coder = DoubleMetaphone::new(analyzer.profile().code_length())?;
        let mut codes = Vec::with_capacity(words.len());
        let mut canonical = Vec::with_capacity(words.len());
        let mut code_rows = Vec::new();
        for (id, word) in words.iter().enumerate() {
            let admitted = match phonetic::canonicalize(&word.text) {
                Ok(spelling) => match coder.encode_canonical(&spelling) {
                    Ok(code) => Some((spelling, code)),
                    Err(PhoneticRefusal::EmptyCode) => None,
                    Err(error) => return Err(error.into()),
                },
                Err(
                    PhoneticRefusal::UnsupportedScalar { .. }
                    | PhoneticRefusal::InvalidApostrophe
                    | PhoneticRefusal::EmptySpelling
                    | PhoneticRefusal::InputTooLong { .. },
                ) => None,
                Err(error) => return Err(error.into()),
            };
            if let Some((spelling, encoded)) = admitted {
                for code in distinct_codes(&encoded) {
                    code_rows.push((code.to_owned(), id as u32));
                }
                codes.push(Some(encoded));
                canonical.push(Some(spelling));
            } else {
                codes.push(None);
                canonical.push(None);
            }
        }
        let mut result = Self {
            analyzer,
            generation: [0; 32],
            words,
            spans,
            sources,
            shapes,
            grams,
            codes,
            canonical,
            phonetic_postings: FlatPostings::build(code_rows),
        };
        result.generation = result.content_generation();
        Ok(result)
    }

    /// The analyzer shared by construction and query projections.
    pub const fn analyzer(&self) -> &Analyzer {
        &self.analyzer
    }
    /// Identity carried by evidence and typed substring refusals.
    pub const fn generation(&self) -> [u8; 32] {
        self.generation
    }

    /// Remove complete documents, preserving the identifiers of every retained
    /// document and source literal. An absent document is a no-op.
    ///
    /// Existing normalized text, alignment, span shapes, gram positions and
    /// pronunciation codes are retained. The sorted postings are filtered and
    /// their term ordinals compacted in place; no analysis, gram construction,
    /// phonetic encoding or posting sort is performed. The returned standalone
    /// surface generation describes the remaining content. With no actual
    /// removal the original generation is preserved, including an enclosing
    /// text-index binding.
    #[must_use]
    pub fn without_documents(self, documents: impl IntoIterator<Item = u32>) -> Self {
        let removed: BTreeSet<_> = documents.into_iter().collect();
        self.map_documents(
            |document| (!removed.contains(&document)).then_some(document),
            true,
        )
    }

    /// The enclosing index supplies an order-preserving canonical document map
    /// and binds its own generation after the complete update.
    pub(crate) fn rebase_documents(self, documents: &[Option<u32>]) -> Self {
        self.map_documents(|document| documents[document as usize], false)
    }

    fn map_documents(mut self, mut map: impl FnMut(u32) -> Option<u32>, hash: bool) -> Self {
        let mut source_ordinals = Vec::with_capacity(self.sources.len());
        let mut retained = 0;
        let mut changed = false;
        self.sources.retain_mut(|source| {
            let document = map(source.document);
            changed |= document != Some(source.document);
            let keep = document.is_some();
            if let Some(document) = document {
                source.document = document;
            }
            source_ordinals.push(keep.then_some(retained));
            retained += usize::from(keep);
            keep
        });
        if !changed {
            return self;
        }

        let words = retain_terms(&mut self.words, &source_ordinals, &self.sources);
        let spans = retain_terms(&mut self.spans, &source_ordinals, &self.sources);
        retain_aligned(&mut self.codes, &words);
        retain_aligned(&mut self.canonical, &words);
        retain_aligned(&mut self.shapes, &spans);

        let remap_grams = |postings: FlatPostings<u64, GramPosting>| {
            postings.retain_remapped(|posting| {
                spans[posting.span as usize].map(|span| GramPosting { span, ..posting })
            })
        };
        let [one, two, three] = self.grams;
        self.grams = [remap_grams(one), remap_grams(two), remap_grams(three)];
        self.phonetic_postings = self
            .phonetic_postings
            .retain_remapped(|word| words[word as usize]);
        if hash {
            self.generation = self.content_generation();
        }
        self
    }
    /// Bind this auxiliary index to the enclosing immutable text index.
    pub(crate) const fn bind_generation(&mut self, generation: [u8; 32]) {
        self.generation = generation;
    }
    /// Distinct pre-stem words in lexical order.
    pub fn words(&self) -> &[SurfaceTerm] {
        &self.words
    }
    /// Distinct punctuation-bearing spans in lexical order.
    pub fn spans(&self) -> &[SurfaceTerm] {
        &self.spans
    }

    /// Retrieve shared-code candidates and refine their canonical spellings.
    /// Results sort by distance, then surface spelling.
    ///
    /// # Errors
    /// Refuses anything other than one analyzed word, unsupported phonetic
    /// spelling, or the profile's explicit distance resource bounds.
    pub fn phonetic(&self, input: &str) -> Result<Vec<PhoneticMatch<'_>>, TextError> {
        let terms = self.analyzer.surface_terms(input)?;
        let [needle] = terms.as_slice() else {
            return Err(PhoneticRefusal::SurfaceWordCount {
                observed: terms.len(),
            }
            .into());
        };
        let spelling = phonetic::canonicalize(needle)?;
        let query = DoubleMetaphone::new(self.analyzer.profile().code_length())?
            .encode_canonical(&spelling)?;
        let primary = self
            .phonetic_postings
            .get(query.primary.as_str())
            .unwrap_or_default();
        let alternate = self
            .phonetic_postings
            .get(query.alternate.as_str())
            .unwrap_or_default();
        let mut prepared =
            PreparedDistance::new(&spelling, self.analyzer.profile().edit_distance())?;
        let mut matches = Vec::new();
        for id in postings::join_by(primary.iter().copied(), alternate.iter().copied(), u32::cmp)
            .map(postings::Joined::one)
        {
            let id = id as usize;
            let term = &self.words[id];
            let canonical = self.canonical[id]
                .as_deref()
                .expect("code posting has canonical spelling");
            let Some(distance) = prepared.distance(canonical)? else {
                continue;
            };
            let codes = self.codes[id]
                .as_ref()
                .expect("code posting has pronunciation");
            let mut shared_codes: Vec<_> = distinct_codes(&query)
                .filter(|&code| code == codes.primary || code == codes.alternate)
                .map(str::to_owned)
                .collect();
            shared_codes.sort_unstable();
            let range = 0..term.text.len();
            let evidence = self.evidence(
                term,
                std::slice::from_ref(&range),
                MatchProjection::SurfaceWord,
            );
            matches.push(PhoneticMatch {
                term,
                distance,
                codes,
                shared_codes,
                evidence,
            });
        }
        matches.sort_by(|left, right| {
            left.distance
                .cmp(&right.distance)
                .then_with(|| left.term.text.cmp(&right.term.text))
        });
        Ok(matches)
    }

    fn evidence(
        &self,
        term: &SurfaceTerm,
        ranges: &[Range<usize>],
        projection: MatchProjection,
    ) -> Vec<SourceMatchEvidence> {
        let analyzer = self.analyzer.fingerprint();
        let mut result = Vec::new();
        for occurrence in &term.occurrences {
            let source = &self.sources[occurrence.source];
            for range in ranges {
                let projected_range = if occurrence.coarse {
                    occurrence.range.clone()
                } else {
                    occurrence.range.start + range.start..occurrence.range.start + range.end
                };
                let contributors = if occurrence.coarse {
                    occurrence.contributors.clone()
                } else {
                    source.normalized.contributors(projected_range.clone())
                };
                let highlight = contributors.first().zip(contributors.last()).map_or_else(
                    || occurrence.highlight.clone(),
                    |(first, last)| first.start..last.end,
                );
                result.push(SourceMatchEvidence {
                    document: source.document,
                    literal: source.identity,
                    projection,
                    projected_range,
                    sources: contributors,
                    highlight,
                    coarse: occurrence.coarse,
                    analyzer,
                    generation: self.generation,
                });
            }
        }
        result
    }

    fn content_generation(&self) -> [u8; 32] {
        let mut digest = blake3::Hasher::new();
        digest.update(GENERATION_DOMAIN.as_bytes());
        frame_le_into(&mut digest, &self.analyzer.fingerprint());
        frame_le_into(&mut digest, &(self.sources.len() as u64).to_le_bytes());
        for source in &self.sources {
            frame_le_into(&mut digest, &source.document.to_le_bytes());
            frame_le_into(&mut digest, &source.identity);
            frame_le_into(&mut digest, source.normalized.text.as_bytes());
        }
        for terms in [&self.words, &self.spans] {
            frame_le_into(&mut digest, &(terms.len() as u64).to_le_bytes());
            for term in terms {
                frame_le_into(&mut digest, term.text.as_bytes());
                frame_le_into(&mut digest, &(term.documents.len() as u64).to_le_bytes());
                for document in &term.documents {
                    frame_le_into(&mut digest, &document.to_le_bytes());
                }
                frame_le_into(&mut digest, &(term.occurrences.len() as u64).to_le_bytes());
                for occurrence in &term.occurrences {
                    frame_le_into(&mut digest, &(occurrence.source as u64).to_le_bytes());
                    frame_le_into(&mut digest, &(occurrence.range.start as u64).to_le_bytes());
                    frame_le_into(&mut digest, &(occurrence.range.end as u64).to_le_bytes());
                    frame_le_into(&mut digest, &[u8::from(occurrence.coarse)]);
                    frame_le_into(
                        &mut digest,
                        &(occurrence.contributors.len() as u64).to_le_bytes(),
                    );
                    for range in &occurrence.contributors {
                        frame_le_into(&mut digest, &(range.start as u64).to_le_bytes());
                        frame_le_into(&mut digest, &(range.end as u64).to_le_bytes());
                    }
                }
            }
        }
        *digest.finalize().as_bytes()
    }
}

#[derive(Default)]
struct TermBuild {
    documents: Vec<u32>,
    occurrences: Vec<Occurrence>,
}

fn add_projection(
    terms: &mut BTreeMap<String, TermBuild>,
    projection: Projection,
    document: u32,
    source: usize,
    normalized: &AlignedText,
) -> Result<(), TextError> {
    let Some(origin) = normalized.text.get(projection.range.clone()) else {
        return Err(TextError::data(
            "surface projection has an invalid normalized range",
        ));
    };
    if projection.text.is_empty() || !projection.coarse && origin != projection.text {
        return Err(TextError::data(
            "surface projection spelling disagrees with its normalized range",
        ));
    }
    let entry = terms.entry(projection.text).or_default();
    entry.documents.push(document);
    entry.occurrences.push(Occurrence {
        source,
        range: projection.range,
        contributors: projection.sources,
        highlight: projection.highlight,
        coarse: projection.coarse,
    });
    Ok(())
}
/// Retain the source-backed members of one sorted projection table and return
/// the order-preserving old-to-new ordinal map used by its physical postings.
fn retain_terms(
    terms: &mut Vec<SurfaceTerm>,
    sources: &[Option<usize>],
    retained_sources: &[Source],
) -> Vec<Option<u32>> {
    let mut ordinals = Vec::with_capacity(terms.len());
    let mut retained = 0;
    terms.retain_mut(|term| {
        term.occurrences.retain_mut(|occurrence| {
            if let Some(source) = sources[occurrence.source] {
                occurrence.source = source;
                true
            } else {
                false
            }
        });
        term.documents.clear();
        for occurrence in &term.occurrences {
            let document = retained_sources[occurrence.source].document;
            if term.documents.last() != Some(&document) {
                term.documents.push(document);
            }
        }
        let keep = !term.occurrences.is_empty();
        ordinals.push(keep.then_some(retained));
        retained += u32::from(keep);
        keep
    });
    ordinals
}

/// Projection metadata follows the same retained ordinal order as its terms.
fn retain_aligned<T>(rows: &mut Vec<T>, ordinals: &[Option<u32>]) {
    let mut position = 0;
    rows.retain(|_| {
        let keep = ordinals[position].is_some();
        position += 1;
        keep
    });
}

fn finish(terms: BTreeMap<String, TermBuild>) -> Vec<SurfaceTerm> {
    terms
        .into_iter()
        .map(|(text, mut term)| {
            term.documents.sort_unstable();
            term.documents.dedup();
            term.occurrences
                .sort_by_key(|item| (item.source, item.range.start, item.range.end));
            term.occurrences.dedup();
            SurfaceTerm {
                text,
                documents: term.documents,
                occurrences: term.occurrences,
            }
        })
        .collect()
}
fn distinct_codes(codes: &PhoneticCodes) -> impl Iterator<Item = &str> {
    [codes.primary.as_str(), codes.alternate.as_str()]
        .into_iter()
        .enumerate()
        .filter_map(|(at, code)| {
            (!code.is_empty() && (at == 0 || code != codes.primary)).then_some(code)
        })
}

#[cfg(test)]
mod tests {
    use super::{Analyzer, SurfaceIndex};

    #[test]
    fn successive_document_deltas_preserve_fresh_generations_and_queries() {
        let analyzer = Analyzer::empty_lexicon();
        let mut corpus = vec![(0, "alpha a-b"), (2, "café Smith"), (4, "Robert river")];
        let mut current =
            SurfaceIndex::from_texts(analyzer.clone(), corpus.iter().copied()).unwrap();
        let changes: &[&[(u32, &str)]] = &[
            &[(0, "Robert a-b"), (2, "")],
            &[(1, "Smith river"), (9, "alphabet 🧑🏽‍💻")],
            &[(0, "new CAFÉ"), (1, "Robert")],
        ];
        for &changes in changes {
            corpus.retain(|(document, _)| !changes.iter().any(|(changed, _)| document == changed));
            corpus.extend_from_slice(changes);
            let delta =
                SurfaceIndex::from_texts(analyzer.clone(), changes.iter().copied()).unwrap();
            current = current.with_documents(delta).unwrap();
            let fresh = SurfaceIndex::from_texts(analyzer.clone(), corpus.iter().copied()).unwrap();
            assert_eq!(current.words(), fresh.words());
            assert_eq!(current.spans(), fresh.spans());
            assert_eq!(current.generation(), fresh.generation());
            assert_eq!(
                current.substring_report("a-b"),
                fresh.substring_report("a-b")
            );
            assert_eq!(current.phonetic("Smith"), fresh.phonetic("Smith"));
        }
    }

    #[test]
    fn document_delta_matches_fresh_insertion_and_replacement() {
        let analyzer = Analyzer::empty_lexicon();
        let original = [
            (0, "alpha a-b café 🧑🏽‍💻"),
            (2, "alphabet CAFÉ"),
            (3, "SMITH riverbank"),
            (3, "Robert a-b"),
            (4, "Smithy alpha"),
        ];
        let changes = [
            (0, "Smith canal CAFE"),
            (0, "added a-b"),
            (1, "Robert 🧑🏽‍💻"),
            (8, "alphabet CAFÉ"),
            (0, "added a-b"),
        ];
        let base = SurfaceIndex::from_texts(analyzer.clone(), original).unwrap();
        let retained_allocation = base
            .sources
            .iter()
            .find(|source| source.document == 3 && source.normalized.text.contains("smith"))
            .unwrap()
            .normalized
            .text
            .as_ptr();
        let delta = SurfaceIndex::from_texts(analyzer.clone(), changes).unwrap();
        let updated = base.with_documents(delta).unwrap();
        let fresh = SurfaceIndex::from_texts(
            analyzer.clone(),
            original
                .into_iter()
                .filter(|(document, _)| *document != 0)
                .chain(changes),
        )
        .unwrap();
        assert_eq!(updated.words(), fresh.words());
        assert_eq!(updated.spans(), fresh.spans());
        assert_eq!(updated.generation(), fresh.generation());
        for needle in ["a", "a-", "a-b", "alpha", "café", "🧑🏽‍💻", "river", "not here"] {
            assert_eq!(
                updated.substring_report(needle),
                fresh.substring_report(needle),
                "{needle}"
            );
            assert_eq!(
                updated.substring_exhaustive_report(needle),
                fresh.substring_exhaustive_report(needle),
                "{needle}"
            );
        }
        for needle in ["Smith", "Robert", "cafe", "alphabet"] {
            assert_eq!(updated.phonetic(needle), fresh.phonetic(needle), "{needle}");
        }
        let retained = updated
            .sources
            .iter()
            .find(|source| source.document == 3 && source.normalized.text.contains("smith"))
            .unwrap();
        assert_eq!(retained.normalized.text.as_ptr(), retained_allocation);
        let empty = SurfaceIndex::from_texts(analyzer, []).unwrap();
        let generation = updated.generation();
        assert_eq!(
            updated.with_documents(empty).unwrap().generation(),
            generation
        );
    }

    #[test]
    fn document_delta_refuses_a_different_analyzer() {
        let original =
            SurfaceIndex::from_texts(Analyzer::empty_lexicon(), [(1, "running")]).unwrap();
        let analyzer = Analyzer::resolve(
            crate::AnalyzerProfile::empty_lexicon().with_stemming(crate::Stemming::English),
            &[],
        )
        .unwrap();
        let added = SurfaceIndex::from_texts(analyzer, [(2, "run")]).unwrap();
        let expected = original.analyzer().fingerprint();
        let actual = added.analyzer().fingerprint();
        assert!(matches!(
            original.with_documents(added),
            Err(crate::TextDeltaError::AnalyzerMismatch { expected: index, actual: delta })
                if index == expected && delta == actual
        ));
    }

    #[test]
    fn document_retraction_matches_fresh_auxiliary_results_and_source_evidence() {
        let corpus = [
            (0, "alpha a-b café 🧑🏽‍💻"),
            (1, "beta river bank"),
            (2, "alphabet CAFÉ"),
            (3, "SMITH riverbank"),
            (3, "Robert a-b"),
            (4, "Smithy alpha"),
            (4, "Smithy alpha"),
        ];
        let analyzer = Analyzer::empty_lexicon();
        let original = SurfaceIndex::from_texts(analyzer.clone(), corpus).unwrap();
        let snapshot = original.clone();
        let text_allocation = original
            .sources
            .iter()
            .find(|source| source.document == 3 && source.normalized.text.contains("smith"))
            .unwrap()
            .normalized
            .text
            .as_ptr();
        let word = original
            .words
            .iter()
            .position(|word| word.text == "smith")
            .unwrap();
        let code_allocation = original.canonical[word].as_ref().unwrap().as_ptr();
        let updated = original.without_documents([0, 1, 1, 999]);
        let fresh = SurfaceIndex::from_texts(
            analyzer,
            corpus.into_iter().filter(|(document, _)| *document > 1),
        )
        .unwrap();
        assert_eq!(updated.words(), fresh.words());
        assert_eq!(updated.spans(), fresh.spans());
        assert_eq!(updated.generation(), fresh.generation());
        for needle in ["alpha", "café", "🧑🏽‍💻", "a-b", "river", "not here"] {
            assert_eq!(
                updated.substring_report(needle),
                fresh.substring_report(needle),
                "{needle}"
            );
            assert_eq!(
                updated.substring_exhaustive_report(needle),
                fresh.substring_exhaustive_report(needle),
                "{needle}"
            );
        }
        for needle in ["Smith", "Robert", "cafe", "absent"] {
            assert_eq!(updated.phonetic(needle), fresh.phonetic(needle), "{needle}");
        }
        let retained = updated
            .sources
            .iter()
            .find(|source| source.document == 3 && source.normalized.text.contains("smith"))
            .unwrap();
        assert_eq!(retained.normalized.text.as_ptr(), text_allocation);
        let word = updated
            .words
            .iter()
            .position(|word| word.text == "smith")
            .unwrap();
        assert_eq!(
            updated.canonical[word].as_ref().unwrap().as_ptr(),
            code_allocation
        );
        assert!(snapshot.sources.iter().any(|source| source.document == 0));
        assert_ne!(snapshot.generation(), updated.generation());
    }

    #[test]
    fn retraction_handles_noop_bound_generation_and_the_last_document() {
        let analyzer = Analyzer::empty_lexicon();
        let mut original = SurfaceIndex::from_texts(analyzer.clone(), [(7, "café Smith")]).unwrap();
        original.bind_generation([19; 32]);
        let unchanged = original
            .without_documents([99])
            .with_documents(SurfaceIndex::from_texts(analyzer.clone(), []).unwrap())
            .unwrap();
        assert_eq!(unchanged.generation(), [19; 32]);
        let empty = unchanged.without_documents([7, 7]);
        let fresh = SurfaceIndex::from_texts(analyzer, []).unwrap();
        assert_eq!(empty.words(), fresh.words());
        assert_eq!(empty.spans(), fresh.spans());
        assert_eq!(empty.generation(), fresh.generation());
        assert_eq!(
            empty.substring_report("Smith"),
            fresh.substring_report("Smith")
        );
        assert_eq!(empty.phonetic("Smith"), fresh.phonetic("Smith"));
    }

    #[test]
    fn conflicting_source_identity_and_invalid_projection_are_rejected() {
        let analyzer = Analyzer::empty_lexicon();
        let first = analyzer.projections("abc").unwrap();
        let second = analyzer.projections("different").unwrap();
        assert!(
            SurfaceIndex::from_analyses(
                analyzer.clone(),
                [(0, [1; 32], first), (0, [1; 32], second)]
            )
            .is_err()
        );
        let mut analysis = analyzer.projections("abc").unwrap();
        analysis.spans[0].range = 0..99;
        assert!(SurfaceIndex::from_analyses(analyzer, [(0, [2; 32], analysis)]).is_err());
    }
}
