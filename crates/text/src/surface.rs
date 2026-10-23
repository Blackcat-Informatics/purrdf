// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Immutable auxiliary retrieval over source-aligned persistent tables.

use crate::analysis::{AlignedText, Analysis, Projection};
use crate::index::{Document, Table, compare_document_keys, share_index};
use crate::phonetic::{self, DoubleMetaphone, PhoneticCodes, PhoneticRefusal, PreparedDistance};
use crate::{Analyzer, CorpusProfile, GraphLanguage, TextError};
use purrdf_hash::{Domain, blake3, frame::frame_le_into, multiset::Multiset256};
use purrdf_lex::allocation::shared::Shared;
use std::{
    borrow::Borrow,
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    fmt,
    ops::Range,
};

mod delta;
pub(crate) mod postings;
mod substring;
use postings::{FlatPostings, GramPosting, SpanShape};
pub use substring::{
    SubstringLimits, SubstringRefusal, SubstringRefusalReason, SubstringReport, SubstringWork,
};

const GENERATION_DOMAIN: Domain = Domain::new(b"purrdf-text/surface-generation/v2\0");
const SOURCE_DOMAIN: Domain = Domain::new(b"purrdf-text/surface-source/v1\0");
const LITERAL_DOMAIN: Domain = Domain::new(b"purrdf-text/surface-literal/v1\0");

/// The projection in which match offsets are expressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MatchProjection {
    /// A complete pre-stem surface word.
    SurfaceWord,
    /// A punctuation-preserving bounded span.
    SubstringSpan,
}

/// One occurrence traced to its original literal.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SourceMatchEvidence {
    /// Current document identifier.
    pub document: u32,
    /// Identity of the complete original source row.
    pub literal: [u8; 32],
    /// Which analyzer projection matched.
    pub projection: MatchProjection,
    /// Byte range in the source's shared normalized buffer.
    pub projected_range: Range<usize>,
    /// Sorted original UTF-8 contributors.
    pub sources: Vec<Range<usize>>,
    /// Enclosing original UTF-8 range.
    pub highlight: Range<usize>,
    /// Whether correspondence is whole-projection.
    pub coarse: bool,
    /// Resolved analyzer identity.
    pub analyzer: [u8; 32],
    /// Immutable generation producing this evidence.
    pub generation: [u8; 32],
}

#[derive(Clone, Debug)]
struct Spelling(Shared<String>);
impl Spelling {
    fn text(&self) -> &str {
        self.0.as_str()
    }
}
impl Borrow<str> for Spelling {
    fn borrow(&self) -> &str {
        self.text()
    }
}
impl PartialEq for Spelling {
    fn eq(&self, other: &Self) -> bool {
        self.text() == other.text()
    }
}
impl Eq for Spelling {}
impl PartialOrd for Spelling {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Spelling {
    fn cmp(&self, other: &Self) -> Ordering {
        self.text().cmp(other.text())
    }
}

#[derive(Clone, Debug)]
enum SurfaceDocument<P: CorpusProfile> {
    Ordinal(u32),
    Indexed(Shared<Document<P>>),
}
impl<P: CorpusProfile> PartialEq for SurfaceDocument<P> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}
impl<P: CorpusProfile> Eq for SurfaceDocument<P> {}
impl<P: CorpusProfile> PartialOrd for SurfaceDocument<P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<P: CorpusProfile> Ord for SurfaceDocument<P> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Ordinal(a), Self::Ordinal(b)) => a.cmp(b),
            (Self::Indexed(a), Self::Indexed(b)) => compare_document_keys(a, b),
            (Self::Ordinal(_), Self::Indexed(_)) => Ordering::Less,
            (Self::Indexed(_), Self::Ordinal(_)) => Ordering::Greater,
        }
    }
}

#[derive(Debug)]
struct Source<P: CorpusProfile> {
    document: SurfaceDocument<P>,
    identity: [u8; 32],
    normalized: AlignedText,
    words: Vec<Spelling>,
    spans: Vec<Spelling>,
    digest: [u8; 32],
}
fn compare_sources<P: CorpusProfile>(a: &Source<P>, b: &Source<P>) -> Ordering {
    a.document
        .cmp(&b.document)
        .then_with(|| a.identity.cmp(&b.identity))
}

#[derive(Debug, PartialEq, Eq)]
struct OccurrenceData {
    range: Range<usize>,
    contributors: Vec<Range<usize>>,
    highlight: Range<usize>,
    coarse: bool,
    ordinal: usize,
}
#[derive(Debug)]
struct OccurrenceChunk<P: CorpusProfile> {
    source: Shared<Source<P>>,
    data: Vec<OccurrenceData>,
}
#[derive(Clone, Debug)]
struct Occurrence<P: CorpusProfile> {
    chunk: Shared<OccurrenceChunk<P>>,
    at: u8,
}
impl<P: CorpusProfile> Occurrence<P> {
    fn source(&self) -> &Source<P> {
        &self.chunk.source
    }
    fn data(&self) -> &OccurrenceData {
        &self.chunk.data[usize::from(self.at)]
    }
    fn cmp(&self, other: &Self) -> Ordering {
        compare_sources(self.source(), other.source()).then_with(|| {
            let (a, b) = (self.data(), other.data());
            (a.range.start, a.range.end, a.ordinal).cmp(&(b.range.start, b.range.end, b.ordinal))
        })
    }
    fn same_data(&self, other: &Self) -> bool {
        let (a, b) = (self.data(), other.data());
        compare_sources(self.source(), other.source()).is_eq()
            && a.range == b.range
            && a.contributors == b.contributors
            && a.highlight == b.highlight
            && a.coarse == b.coarse
    }
}

#[derive(Clone, Debug)]
struct Member<P: CorpusProfile> {
    document: SurfaceDocument<P>,
    count: usize,
}
#[derive(Debug)]
enum TermMetadata {
    Word {
        codes: Option<PhoneticCodes>,
        canonical: Option<String>,
    },
    Span(SpanShape),
}
#[derive(Clone, Debug)]
struct StoredTerm<P: CorpusProfile> {
    text: Spelling,
    members: Table<Member<P>>,
    occurrences: Table<Occurrence<P>>,
    metadata: Shared<TermMetadata>,
}
impl<P: CorpusProfile> StoredTerm<P> {
    fn pronunciation(&self) -> Option<(&PhoneticCodes, &str)> {
        match &*self.metadata {
            TermMetadata::Word {
                codes: Some(codes),
                canonical: Some(canonical),
            } => Some((codes, canonical.as_str())),
            TermMetadata::Word { .. } | TermMetadata::Span(_) => None,
        }
    }
    fn shape(&self) -> &SpanShape {
        let TermMetadata::Span(shape) = &*self.metadata else {
            unreachable!("a span has its shape")
        };
        shape
    }
}

/// A borrowed normalized term and current sorted document membership.
#[derive(Clone, Copy)]
pub struct SurfaceTerm<'a, P: CorpusProfile = GraphLanguage> {
    owner: &'a SurfaceIndex<P>,
    stored: &'a StoredTerm<P>,
}
impl<'a, P: CorpusProfile> SurfaceTerm<'a, P> {
    /// The complete bounded normalized spelling.
    pub fn text(&self) -> &'a str {
        self.stored.text.text()
    }
    /// Current sorted distinct document identifiers, without materializing a table.
    pub fn documents(&self) -> impl ExactSizeIterator<Item = u32> + Clone + use<'a, P> {
        let owner = self.owner;
        self.stored
            .members
            .iter()
            .map(move |member| owner.document_id(&member.document))
    }
}
impl<P: CorpusProfile> fmt::Debug for SurfaceTerm<'_, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let occurrences: Vec<_> = self
            .stored
            .occurrences
            .iter()
            .map(|o| {
                (
                    self.owner.document_id(&o.source().document),
                    o.source().identity,
                    o.data(),
                )
            })
            .collect();
        f.debug_struct("SurfaceTerm")
            .field("text", &self.text())
            .field("documents", &self.documents().collect::<Vec<_>>())
            .field("occurrences", &occurrences)
            .finish()
    }
}
impl<P: CorpusProfile> PartialEq for SurfaceTerm<'_, P> {
    fn eq(&self, other: &Self) -> bool {
        self.text() == other.text()
            && self.documents().eq(other.documents())
            && self
                .stored
                .occurrences
                .iter()
                .map(|o| {
                    (
                        self.owner.document_id(&o.source().document),
                        o.source().identity,
                        o.data(),
                    )
                })
                .eq(other.stored.occurrences.iter().map(|o| {
                    (
                        other.owner.document_id(&o.source().document),
                        o.source().identity,
                        o.data(),
                    )
                }))
    }
}
impl<P: CorpusProfile> Eq for SurfaceTerm<'_, P> {}

/// Exact overlapping substring occurrences and original-source evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubstringMatch<'a, P: CorpusProfile = GraphLanguage> {
    /// The matching normalized span and current membership.
    pub term: SurfaceTerm<'a, P>,
    /// UTF-8 byte ranges relative to the term.
    pub ranges: Vec<Range<usize>>,
    /// Original literal occurrences.
    pub evidence: Vec<SourceMatchEvidence>,
}
/// Shared pronunciation codes refined by exact spelling distance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhoneticMatch<'a, P: CorpusProfile = GraphLanguage> {
    /// The pre-stem surface word.
    pub term: SurfaceTerm<'a, P>,
    /// Exact unit-cost spelling distance.
    pub distance: usize,
    /// Candidate primary and alternate codes.
    pub codes: &'a PhoneticCodes,
    /// Shared nonempty codes, sorted and distinct.
    pub shared_codes: Vec<String>,
    /// Original word occurrences.
    pub evidence: Vec<SourceMatchEvidence>,
}

/// Persistent scalar-gram and pronunciation indexes over immutable sources.
#[derive(Clone, Debug)]
pub struct SurfaceIndex<P: CorpusProfile = GraphLanguage> {
    analyzer: Analyzer,
    generation: [u8; 32],
    coordinate_generation: Option<[u8; 32]>,
    document_order: Option<Table<Shared<Document<P>>>>,
    words: Table<StoredTerm<P>>,
    spans: Table<StoredTerm<P>>,
    sources: Table<Shared<Source<P>>>,
    source_state: Multiset256,
    grams: [FlatPostings<u64, GramPosting>; 3],
    phonetic_postings: FlatPostings<String, Spelling>,
}

impl SurfaceIndex<GraphLanguage> {
    /// Analyze each original literal. Equal document/text rows coalesce.
    /// # Errors
    /// Propagates analysis, allocation and index-size refusals.
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

    pub(crate) fn from_analyses(
        analyzer: Analyzer,
        rows: impl IntoIterator<Item = (u32, [u8; 32], Analysis)>,
    ) -> Result<Self, TextError> {
        Self::from_rows(
            analyzer,
            None,
            rows.into_iter()
                .map(|(d, i, a)| (SurfaceDocument::Ordinal(d), i, a)),
        )
    }
}

impl<P: CorpusProfile> SurfaceIndex<P> {
    pub(crate) fn from_index_analyses(
        analyzer: Analyzer,
        order: Table<Shared<Document<P>>>,
        rows: impl IntoIterator<Item = (u32, [u8; 32], Analysis)>,
    ) -> Result<Self, TextError> {
        let rows = rows
            .into_iter()
            .map(|(d, i, a)| (SurfaceDocument::Indexed(order[d as usize].clone()), i, a))
            .collect::<Vec<_>>();
        Self::from_rows(analyzer, Some(order), rows)
    }

    fn from_rows(
        analyzer: Analyzer,
        document_order: Option<Table<Shared<Document<P>>>>,
        rows: impl IntoIterator<Item = (SurfaceDocument<P>, [u8; 32], Analysis)>,
    ) -> Result<Self, TextError> {
        let mut rows: Vec<_> = rows.into_iter().collect();
        rows.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        for pair in rows.windows(2) {
            if pair[0].0 == pair[1].0 && pair[0].1 == pair[1].1 && pair[0].2 != pair[1].2 {
                return Err(TextError::data(
                    "one source identity names conflicting analyses",
                ));
            }
        }
        rows.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
        let mut pool = BTreeMap::new();
        let mut words = BTreeMap::new();
        let mut spans = BTreeMap::new();
        let mut sources = Vec::with_capacity(rows.len());
        let mut source_state = Multiset256::default();
        for (document, identity, analysis) in rows {
            let names = |projections: &[Projection],
                         pool: &mut BTreeMap<String, Spelling>|
             -> Result<Vec<Spelling>, TextError> {
                let mut names = BTreeSet::new();
                for projection in projections {
                    let word = if let Some(word) = pool.get(projection.text.as_str()) {
                        word.clone()
                    } else {
                        let word = Spelling(share_index(projection.text.clone())?);
                        pool.insert(projection.text.clone(), word.clone());
                        word
                    };
                    names.insert(word);
                }
                Ok(names.into_iter().collect())
            };
            let word_names = names(&analysis.surface, &mut pool)?;
            let span_names = names(&analysis.spans, &mut pool)?;
            let mut digest = blake3::Hasher::new();
            digest.update(SOURCE_DOMAIN.as_bytes());
            match &document {
                SurfaceDocument::Ordinal(id) => {
                    digest.update(&[0]);
                    frame_le_into(&mut digest, &id.to_le_bytes());
                }
                SurfaceDocument::Indexed(_) => {
                    digest.update(&[1]);
                    frame_le_into(&mut digest, P::ID.as_bytes());
                }
            }
            frame_le_into(&mut digest, &identity);
            let digest = *digest.finalize().as_bytes();
            source_state
                .insert(digest)
                .map_err(|e| TextError::data(e.to_string()))?;
            let source = share_index(Source {
                document,
                identity,
                normalized: analysis.normalized,
                words: word_names,
                spans: span_names,
                digest,
            })?;
            add_projections(&mut words, &pool, &source, analysis.surface)?;
            add_projections(&mut spans, &pool, &source, analysis.spans)?;
            sources.push(source);
        }
        check_terms(words.len())?;
        check_terms(spans.len())?;
        let coder = DoubleMetaphone::new(analyzer.profile().code_length())?;
        let words = finish(words, |text| word_metadata(text, coder))?;
        let spans = finish(spans, |text| Ok(TermMetadata::Span(SpanShape::new(text))))?;
        let mut gram_rows: [Vec<_>; 3] = std::array::from_fn(|_| Vec::new());
        for term in spans.iter() {
            for (width, rows) in gram_rows.iter_mut().enumerate() {
                rows.extend(postings::gram_rows(&term.text, width + 1));
            }
        }
        let mut code_rows = Vec::new();
        for term in words.iter() {
            if let TermMetadata::Word {
                codes: Some(codes), ..
            } = &*term.metadata
            {
                for code in distinct_codes(codes) {
                    code_rows.push((code.to_owned(), term.text.clone()));
                }
            }
        }
        let mut result = Self {
            analyzer,
            generation: [0; 32],
            coordinate_generation: None,
            document_order,
            words,
            spans,
            sources: Table::from_streams(sources)?,
            source_state,
            grams: [
                FlatPostings::build(std::mem::take(&mut gram_rows[0]))?,
                FlatPostings::build(std::mem::take(&mut gram_rows[1]))?,
                FlatPostings::build(std::mem::take(&mut gram_rows[2]))?,
            ],
            phonetic_postings: FlatPostings::build(code_rows)?,
        };
        result.generation = result.content_generation();
        Ok(result)
    }

    /// The analyzer shared by construction and query projections.
    pub const fn analyzer(&self) -> &Analyzer {
        &self.analyzer
    }
    /// Identity carried by evidence and refusals.
    pub const fn generation(&self) -> [u8; 32] {
        self.generation
    }
    /// Distinct pre-stem words, in lexical order.
    pub fn words(&self) -> impl ExactSizeIterator<Item = SurfaceTerm<'_, P>> + Clone {
        self.words.iter().map(|term| self.view(term))
    }
    /// Distinct punctuation-bearing spans, in lexical order.
    pub fn spans(&self) -> impl ExactSizeIterator<Item = SurfaceTerm<'_, P>> + Clone {
        self.spans.iter().map(|term| self.view(term))
    }
    pub(crate) fn bind_generation(&mut self, generation: [u8; 32]) {
        self.generation = generation;
        if self.document_order.is_some() {
            self.coordinate_generation = Some(generation);
        }
    }
    fn view<'a>(&'a self, stored: &'a StoredTerm<P>) -> SurfaceTerm<'a, P> {
        SurfaceTerm {
            owner: self,
            stored,
        }
    }
    fn term<'a>(table: &'a Table<StoredTerm<P>>, text: &str) -> Option<&'a StoredTerm<P>> {
        table
            .binary_search_by(|term| term.text.text().cmp(text))
            .ok()
            .map(|at| &table[at])
    }
    fn document_id(&self, document: &SurfaceDocument<P>) -> u32 {
        match document {
            SurfaceDocument::Ordinal(id) => *id,
            SurfaceDocument::Indexed(document) => {
                self.document_order
                    .as_ref()
                    .expect("indexed sources have their document order")
                    .binary_search_by(|candidate| compare_document_keys(candidate, document))
                    .expect("a live surface document has its current rank") as u32
            }
        }
    }

    /// Retrieve shared-code candidates and refine their spellings.
    /// # Errors
    /// Refuses invalid queries and the profile's distance resource bounds.
    pub fn phonetic(&self, input: &str) -> Result<Vec<PhoneticMatch<'_, P>>, TextError> {
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
        for name in
            postings::join_by(primary.iter(), alternate.iter(), Ord::cmp).map(postings::Joined::one)
        {
            let term =
                Self::term(&self.words, name.text()).expect("a code posting has its current term");
            let (codes, canonical) = term
                .pronunciation()
                .expect("a code posting has its word metadata");
            let Some(distance) = prepared.distance(canonical)? else {
                continue;
            };
            let mut shared_codes: Vec<_> = distinct_codes(&query)
                .filter(|&code| code == codes.primary || code == codes.alternate)
                .map(str::to_owned)
                .collect();
            shared_codes.sort_unstable();
            let range = 0..term.text.text().len();
            let evidence = self.evidence(
                term,
                std::slice::from_ref(&range),
                MatchProjection::SurfaceWord,
            );
            matches.push(PhoneticMatch {
                term: self.view(term),
                distance,
                codes,
                shared_codes,
                evidence,
            });
        }
        matches.sort_by(|a, b| {
            a.distance
                .cmp(&b.distance)
                .then_with(|| a.term.text().cmp(b.term.text()))
        });
        Ok(matches)
    }

    fn evidence(
        &self,
        term: &StoredTerm<P>,
        ranges: &[Range<usize>],
        projection: MatchProjection,
    ) -> Vec<SourceMatchEvidence> {
        let analyzer = self.analyzer.fingerprint();
        let mut result = Vec::new();
        for occurrence in term.occurrences.iter() {
            let source = occurrence.source();
            let data = occurrence.data();
            for range in ranges {
                let projected_range = if data.coarse {
                    data.range.clone()
                } else {
                    data.range.start + range.start..data.range.start + range.end
                };
                let contributors = if data.coarse {
                    data.contributors.clone()
                } else {
                    source.normalized.contributors(projected_range.clone())
                };
                let highlight = contributors
                    .first()
                    .zip(contributors.last())
                    .map_or_else(|| data.highlight.clone(), |(a, b)| a.start..b.end);
                result.push(SourceMatchEvidence {
                    document: self.document_id(&source.document),
                    literal: source.identity,
                    projection,
                    projected_range,
                    sources: contributors,
                    highlight,
                    coarse: data.coarse,
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
        match self.coordinate_generation {
            Some(generation) => {
                digest.update(&[1]);
                frame_le_into(&mut digest, &generation);
            }
            None => {
                digest.update(&[0]);
            }
        }
        frame_le_into(&mut digest, &self.source_state.population().to_le_bytes());
        frame_le_into(&mut digest, self.source_state.sum());
        *digest.finalize().as_bytes()
    }
}

struct TermBuild<P: CorpusProfile> {
    occurrences: Vec<Occurrence<P>>,
}
impl<P: CorpusProfile> Default for TermBuild<P> {
    fn default() -> Self {
        Self {
            occurrences: Vec::new(),
        }
    }
}

fn add_projections<P: CorpusProfile>(
    terms: &mut BTreeMap<Spelling, TermBuild<P>>,
    pool: &BTreeMap<String, Spelling>,
    source: &Shared<Source<P>>,
    projections: Vec<Projection>,
) -> Result<(), TextError> {
    let mut input = projections.into_iter().enumerate();
    loop {
        let mut values = Vec::with_capacity(32);
        let mut names = Vec::with_capacity(32);
        for (ordinal, projection) in input.by_ref().take(32) {
            let Some(origin) = source.normalized.text.get(projection.range.clone()) else {
                return Err(TextError::data(
                    "surface projection has an invalid normalized range",
                ));
            };
            if projection.text.is_empty() || !projection.coarse && origin != projection.text {
                return Err(TextError::data(
                    "surface projection spelling disagrees with its normalized range",
                ));
            }
            names.push(pool[&projection.text].clone());
            values.push(OccurrenceData {
                range: projection.range,
                contributors: projection.sources,
                highlight: projection.highlight,
                coarse: projection.coarse,
                ordinal,
            });
        }
        if values.is_empty() {
            break;
        }
        let chunk = share_index(OccurrenceChunk {
            source: source.clone(),
            data: values,
        })?;
        for (at, name) in names.into_iter().enumerate() {
            terms.entry(name).or_default().occurrences.push(Occurrence {
                chunk: chunk.clone(),
                at: at as u8,
            });
        }
    }
    Ok(())
}

fn word_metadata(text: &str, coder: DoubleMetaphone) -> Result<TermMetadata, TextError> {
    let admitted = match phonetic::canonicalize(text) {
        Ok(spelling) => match coder.encode_canonical(&spelling) {
            Ok(code) => Some((spelling, code)),
            Err(PhoneticRefusal::EmptyCode) => None,
            Err(e) => return Err(e.into()),
        },
        Err(
            PhoneticRefusal::UnsupportedScalar { .. }
            | PhoneticRefusal::InvalidApostrophe
            | PhoneticRefusal::EmptySpelling
            | PhoneticRefusal::InputTooLong { .. },
        ) => None,
        Err(e) => return Err(e.into()),
    };
    let (canonical, codes) = admitted.map_or((None, None), |(s, c)| (Some(s), Some(c)));
    Ok(TermMetadata::Word { codes, canonical })
}

fn finish<P: CorpusProfile>(
    terms: BTreeMap<Spelling, TermBuild<P>>,
    mut metadata: impl FnMut(&str) -> Result<TermMetadata, TextError>,
) -> Result<Table<StoredTerm<P>>, TextError> {
    let mut result = Vec::with_capacity(terms.len());
    for (text, mut term) in terms {
        term.occurrences.sort_by(Occurrence::cmp);
        term.occurrences.dedup_by(|a, b| a.same_data(b));
        let mut members: BTreeMap<SurfaceDocument<P>, usize> = BTreeMap::new();
        for occurrence in &term.occurrences {
            let count = members
                .entry(occurrence.source().document.clone())
                .or_default();
            *count = count
                .checked_add(1)
                .ok_or_else(crate::query_workspace::overflow)?;
        }
        let metadata = share_index(metadata(text.text())?)?;
        result.push(StoredTerm {
            text,
            members: Table::from_streams(
                members
                    .into_iter()
                    .map(|(document, count)| Member { document, count })
                    .collect(),
            )?,
            occurrences: Table::from_streams(term.occurrences)?,
            metadata,
        });
    }
    Table::from_streams(result)
}
fn check_terms(count: usize) -> Result<(), TextError> {
    u32::try_from(count)
        .map(|_| ())
        .map_err(|_| TextError::data("surface index exceeds u32 distinct-term identifiers"))
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
            assert_eq!(
                current.words().collect::<Vec<_>>(),
                fresh.words().collect::<Vec<_>>()
            );
            assert_eq!(
                current.spans().collect::<Vec<_>>(),
                fresh.spans().collect::<Vec<_>>()
            );
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
            .find(|source| {
                source.document == super::SurfaceDocument::Ordinal(3)
                    && source.normalized.text.contains("smith")
            })
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
        assert_eq!(
            updated.words().collect::<Vec<_>>(),
            fresh.words().collect::<Vec<_>>()
        );
        assert_eq!(
            updated.spans().collect::<Vec<_>>(),
            fresh.spans().collect::<Vec<_>>()
        );
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
            .find(|source| {
                source.document == super::SurfaceDocument::Ordinal(3)
                    && source.normalized.text.contains("smith")
            })
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
            .find(|source| {
                source.document == super::SurfaceDocument::Ordinal(3)
                    && source.normalized.text.contains("smith")
            })
            .unwrap()
            .normalized
            .text
            .as_ptr();
        let word = original
            .words
            .iter()
            .position(|word| word.text.text() == "smith")
            .unwrap();
        let code_allocation = original.words[word].pronunciation().unwrap().1.as_ptr();
        let updated = original.without_documents([0, 1, 1, 999]).unwrap();
        let fresh = SurfaceIndex::from_texts(
            analyzer,
            corpus.into_iter().filter(|(document, _)| *document > 1),
        )
        .unwrap();
        assert_eq!(
            updated.words().collect::<Vec<_>>(),
            fresh.words().collect::<Vec<_>>()
        );
        assert_eq!(
            updated.spans().collect::<Vec<_>>(),
            fresh.spans().collect::<Vec<_>>()
        );
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
            .find(|source| {
                source.document == super::SurfaceDocument::Ordinal(3)
                    && source.normalized.text.contains("smith")
            })
            .unwrap();
        assert_eq!(retained.normalized.text.as_ptr(), text_allocation);
        let word = updated
            .words
            .iter()
            .position(|word| word.text.text() == "smith")
            .unwrap();
        assert_eq!(
            updated.words[word].pronunciation().unwrap().1.as_ptr(),
            code_allocation
        );
        assert!(
            snapshot
                .sources
                .iter()
                .any(|source| source.document == super::SurfaceDocument::Ordinal(0))
        );
        assert_ne!(snapshot.generation(), updated.generation());
    }

    #[test]
    fn retraction_handles_noop_bound_generation_and_the_last_document() {
        let analyzer = Analyzer::empty_lexicon();
        let mut original = SurfaceIndex::from_texts(analyzer.clone(), [(7, "café Smith")]).unwrap();
        original.bind_generation([19; 32]);
        let unchanged = original
            .without_documents([99])
            .unwrap()
            .with_documents(SurfaceIndex::from_texts(analyzer.clone(), []).unwrap())
            .unwrap();
        assert_eq!(unchanged.generation(), [19; 32]);
        let empty = unchanged.without_documents([7, 7]).unwrap();
        let fresh = SurfaceIndex::from_texts(analyzer, []).unwrap();
        assert_eq!(
            empty.words().collect::<Vec<_>>(),
            fresh.words().collect::<Vec<_>>()
        );
        assert_eq!(
            empty.spans().collect::<Vec<_>>(),
            fresh.spans().collect::<Vec<_>>()
        );
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
