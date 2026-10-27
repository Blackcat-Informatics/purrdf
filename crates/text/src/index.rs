// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The inverted index: documents, partitions, postings and the two digests
//! that identify them.
//!
//! # The index mints no vocabulary
//!
//! Which predicates carry indexable text is entirely the caller's decision, and
//! there is no default set — [`TextIndexConfig::new`] refuses an empty one
//! rather than substituting a guess. PurRDF is not an ontology: it has no
//! namespace of its own to fall back on, so a fallback here would have to be
//! somebody else's vocabulary silently imposed on the caller's data.
//!
//! # A document is `(graph, subject, language)`
//!
//! Not `(graph, subject)`. A subject that carries `"the cat"@en` and
//! `"le chat"@fr` is two documents, because term statistics computed across both
//! would describe neither language. Not `(subject)` either: the same IRI in two
//! named graphs is two statements about two things, and a search restricted to
//! one graph must not be scored by the other's text.
//!
//! Base direction is deliberately **not** part of the key. `"x"@en` and
//! `"x"@en--ltr` are one document, because direction is presentational — it
//! says which way the glyphs run, never which text is present — so it cannot
//! change what a query matches. Splitting on it would halve the statistics of
//! any corpus that annotates direction inconsistently, for no retrieval benefit.
//! An untagged literal is its own key (`None`), distinct from every real tag
//! rather than colliding with a sentinel empty string.
//!
//! # Both layers of RDF 1.2 are read
//!
//! [`DatasetView::quads_for_pattern`] exposes the **asserted** triple table and
//! nothing else. RDF 1.2's reifier and annotation rows live in separate,
//! capability-gated side tables ([`DatasetView::annotation_quads`]), whose trait
//! defaults are empty and which [`purrdf_core::RdfDataset`] overrides with real
//! implementations. An index built from the asserted table alone would therefore
//! contain **zero** annotation literals, and
//!
//! ```text
//! :s :p :o {| :note "some text" |}
//! ```
//!
//! would be unsearchable — with no error anywhere, because a retrieval that
//! returns fewer rows looks exactly like a retrieval that legitimately matched
//! fewer rows. So [`TextIndex::from_dataset`] walks both layers. In the
//! annotation layer the row's subject **is the reifier**, so the reifier is the
//! document; a reifier that is also an ordinary subject collects text from both
//! layers into one document, which is the merge RDF 1.2 intends.
//!
//! # Determinism, and exactly how far it goes
//!
//! Document ids are assigned only after every document has been sorted by
//! `(graph, subject, language)` under [`TermValue`]'s total order, so an id is a
//! function of the content rather than of the order the dataset happened to
//! intern its terms in. The term dictionary is sorted by the analyzed term
//! string, postings by document id, positions ascending. Two independently built
//! indexes over the same content agree byte for byte, including on both digests.
//!
//! **The limit of that claim, stated precisely**: blank-node labels are a
//! parsing artifact rather than content — two isomorphic datasets can label the
//! same blank node `b0` and `b17` — and [`TermValue::Blank`] orders and encodes
//! by label. A blank-node subject therefore carries its label into the sort key,
//! into the document table and into both fingerprints. Two isomorphic datasets
//! parsed from differently labelled sources produce **different** document ids
//! and **different** fingerprints. This crate does not canonicalize blank nodes
//! (that is `purrdf-core`'s RDFC-1.0 canonicalizer, a much more expensive
//! operation with its own contract), so the honest statement is: ids and digests
//! are a pure function of the dataset's *terms*, which for blank nodes includes
//! their labels. The crate's test suite pins this with an adversarial test
//! rather than leaving it implied.

use crate::query_workspace::{data_error, query_error};
use purrdf_hash::Domain;
use purrdf_hash::frame::frame_le;
use purrdf_lex::allocation::shared::Shared;
use purrdf_sparql_eval::NativeDiagnosticKind;
use std::cmp::Ordering;
use std::collections::BTreeSet;

mod delta;

use crate::SurfaceIndex;
use crate::{CorpusProfile, GraphFree, GraphLanguage};

use purrdf_core::{DatasetView, FastMap, RdfTextDirection, TermValue};

use crate::analysis::{Analyzer, UnicodeVersions, unicode_versions};
use crate::error::TextError;
use crate::fixed::Fixed;
use crate::ranking::{FieldInputs, PreparedCorpus, RankingProfile};
use crate::term_bytes::{FINGERPRINT_BYTES, encode_term};

/// Domain-separation prefix for [`TextIndex::fingerprint`].
const INDEX_DIGEST_DOMAIN: Domain = Domain::new(b"purrdf-text/index/v6");
/// Domain-separation prefix for [`TextIndex::source_fingerprint`].
const SOURCE_DIGEST_DOMAIN: Domain = Domain::new(b"purrdf-text/source/v2");
/// Graph-free source and index identities omit every graph coordinate.
const TRIPLE_INDEX_DIGEST_DOMAIN: Domain = Domain::new(b"purrdf-text/triples/index/v3");
const TRIPLE_SOURCE_DIGEST_DOMAIN: Domain = Domain::new(b"purrdf-text/triples/source/v2");

mod source_digest;
use source_digest::SourceDigest;
mod token_streams;
use token_streams::TokenTable;
pub(crate) use token_streams::{Slice, Table};

/// Digest tag for [`GraphSelector::Any`].
const SELECTOR_ANY: u8 = 0x01;
/// Digest tag for [`GraphSelector::Default`].
const SELECTOR_DEFAULT: u8 = 0x02;
/// Digest tag for [`GraphSelector::Named`].
const SELECTOR_NAMED: u8 = 0x03;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

pub use purrdf_core::GraphSelector;

/// Which independent corpus statistics and query projection the index uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IndexProjection {
    Lexical,
    Han,
}

/// The caller's complete, dataset-independent statement of what to index.
///
/// There is no [`Default`] implementation and there never will be one: a default
/// would have to name predicate IRIs, and PurRDF mints none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextIndexConfig<P: CorpusProfile = GraphLanguage> {
    /// The indexed predicates, sorted and known to be distinct IRIs.
    predicates: Vec<TermValue>,
    /// Which graph the index is drawn from.
    graph: P::Selector,
    /// Independent lexical or Han-character corpus law.
    projection: IndexProjection,
    /// The immutable analysis law used on both sides of retrieval.
    analyzer: Analyzer,
    /// Whether to build the independent substring and phonetic index.
    surface: bool,
}

impl TextIndexConfig<GraphLanguage> {
    /// Select the existing graph-language corpus profile.
    ///
    /// # Errors
    /// Refuses empty/duplicate predicates, non-IRI predicates or a non-IRI named graph.
    pub fn new(
        predicates: Vec<TermValue>,
        graph: GraphSelector,
        analyzer: Analyzer,
    ) -> Result<Self, TextError> {
        Self::build(predicates, graph, analyzer)
    }
}

impl TextIndexConfig<GraphFree> {
    /// Select graph-free subject/language documents and language partitions.
    /// There is no graph selector or default-graph sentinel in this profile.
    ///
    /// # Errors
    /// Refuses empty/duplicate predicates or a non-IRI predicate.
    pub fn for_triples(predicates: Vec<TermValue>, analyzer: Analyzer) -> Result<Self, TextError> {
        Self::build(predicates, (), analyzer)
    }
}

impl<P: CorpusProfile> TextIndexConfig<P> {
    /// A configuration over `predicates`, restricted to `graph`.
    ///
    /// The predicate list is sorted on the way in, so two callers who name the
    /// same predicates in different orders build byte-identical indexes.
    ///
    /// # Errors
    ///
    /// [`TextError::Config`] if `predicates` is empty (there is no default set
    /// to fall back on), if any entry is not an IRI (only an IRI can be an RDF
    /// predicate, so anything else names nothing and would index nothing while
    /// looking like it indexed something), if any predicate is repeated (a
    /// repeat is a caller mistake, and silently deduplicating it would hide the
    /// mistake), or if a [`GraphSelector::Named`] does not hold an IRI.
    fn build(
        predicates: Vec<TermValue>,
        graph: P::Selector,
        analyzer: Analyzer,
    ) -> Result<Self, TextError> {
        if predicates.is_empty() {
            return Err(TextError::config(
                "no indexed predicates supplied; PurRDF mints no vocabulary, so there is no \
                 default predicate set to fall back on",
            ));
        }
        for predicate in &predicates {
            if !matches!(predicate, TermValue::Iri(_)) {
                return Err(TextError::config(format!(
                    "indexed predicate {predicate:?} is not an IRI; only an IRI can occupy the \
                     predicate position of an RDF statement"
                )));
            }
        }
        if let Some(GraphSelector::Named(name)) = P::selector(&graph)
            && !matches!(name, TermValue::Iri(_))
        {
            return Err(TextError::config(format!(
                "named graph selector {name:?} is not an IRI"
            )));
        }

        let mut predicates = predicates;
        predicates.sort();
        for pair in predicates.windows(2) {
            let [left, right] = pair else {
                unreachable!("windows(2) yields pairs")
            };
            if left == right {
                return Err(TextError::config(format!(
                    "indexed predicate {left:?} is listed more than once"
                )));
            }
        }

        Ok(Self {
            predicates,
            graph,
            projection: IndexProjection::Lexical,
            analyzer,
            surface: true,
        })
    }

    pub(crate) fn for_han(mut self) -> Self {
        self.projection = IndexProjection::Han;
        self
    }

    /// Select whether substring and phonetic retrieval are indexed.
    ///
    /// Construction enables this by default. Disabling it omits the surface
    /// postings while retaining identical exact ranked-term retrieval. The
    /// resulting index reports absence through [`TextIndex::surface_index`]
    /// and records the selection in its fingerprint.
    #[must_use]
    pub const fn with_surface_index(mut self, enabled: bool) -> Self {
        self.surface = enabled;
        self
    }

    /// Select the immutable analyzer before building the index.
    #[must_use]
    pub fn with_analyzer(mut self, analyzer: Analyzer) -> Self {
        self.analyzer = analyzer;
        self
    }

    /// The complete analysis law this configuration binds.
    pub const fn analyzer(&self) -> &Analyzer {
        &self.analyzer
    }

    /// The indexed predicates, in sorted order.
    pub fn predicates(&self) -> &[TermValue] {
        &self.predicates
    }

    /// The graph this configuration draws from.
    pub const fn graph(&self) -> &P::Selector {
        &self.graph
    }
}

// ---------------------------------------------------------------------------
// What the configuration found
// ---------------------------------------------------------------------------

/// Which of a configuration's predicates the dataset carried a statement for.
///
/// [`TextIndex::from_dataset`] reads a configured predicate the dataset carries
/// no statement for as **no rows** rather than as a refusal, because the limiting
/// case decides it: a dataset holding nothing carries a statement for nothing, so
/// a refusal would mean no index could exist before the documents it will hold.
/// This type pays that relaxation's cost. Without it a mistyped predicate IRI is
/// a corpus quietly one predicate short; with it the shortfall is a value a host
/// can read, off the index it already holds ([`TextIndex::source_coverage`]) or
/// off the dataset in hand ([`verify_binding`](crate::verify_binding)).
///
/// # The two empty states are distinguishable, and that is the whole design
///
/// * The dataset holds **no statement at all**. Every configured predicate is
///   unrepresented, for the single reason that nothing has landed yet. This is the
///   index-standing-ready state the relaxation exists for, so
///   [`Self::shortfall`] is `None` and nothing anywhere complains.
/// * The dataset **holds statements** and a configured predicate carries none of
///   them in the configured graph scope. [`Self::shortfall`] names that
///   predicate. A mistyped predicate IRI lands here; so does a
///   [`GraphSelector::Named`] graph the dataset does not hold, which leaves every
///   configured predicate with nothing in scope and therefore names all of them.
///
/// # What counts as represented
///
/// A **statement** carrying the predicate, in either RDF 1.2 layer, inside the
/// configured graph scope — not a literal row, and not the predicate IRI merely
/// being interned somewhere. Those two weaker tests are why the refusal this
/// replaced was never a sound detector: an interned-anywhere check passes for an
/// IRI that appears only as a subject or an object, and a literal-row check fails
/// for a correctly spelled predicate whose objects are all IRIs — which carries no
/// text, has never been an error, and is not a wiring mistake.
///
/// # What it cannot distinguish, and why it reports rather than refuses
///
/// A predicate that legitimately has nothing under it yet — a host loading one
/// predicate's data before another's — is the *same observation* as a typo: a
/// configured predicate with no statement, in a dataset that holds other things.
/// Nothing in the data separates them, so this is a report and never a refusal at
/// construction. The index builds, answers, and attests its generation either way;
/// a host that means it reads the shortfall and moves on, and a host that does not
/// mean it has the one signal a typo ever gives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceCoverage {
    /// Whether the dataset held a single statement in any of its layers.
    dataset_holds_statements: bool,
    /// The configured predicates no in-scope statement carried, in configuration
    /// order.
    unrepresented: Vec<TermValue>,
    /// Exact asserted/annotation statement populations in configuration order,
    /// including nonliteral objects and multiplicity across the two layers.
    represented: Vec<u128>,
}

impl SourceCoverage {
    /// Whether the dataset held a statement at all, in any layer and any graph.
    ///
    /// `false` is the index-before-its-documents state: every configured
    /// predicate is unrepresented and none of them can be a mistake yet.
    pub const fn dataset_holds_statements(&self) -> bool {
        self.dataset_holds_statements
    }

    /// Every configured predicate no in-scope statement carried, in
    /// configuration order — the raw observation, including the case where the
    /// dataset holds nothing and so carried nothing for anything.
    pub fn unrepresented_predicates(&self) -> &[TermValue] {
        &self.unrepresented
    }

    /// Exact in-scope statement populations, in the index configuration's
    /// predicate order. Both asserted and annotation rows count, including
    /// nonliteral objects; reifier declarations are not predicate assertions.
    pub fn predicate_statement_counts(&self) -> &[u128] {
        &self.represented
    }

    /// The configured predicates whose absence is a fact about the *data* rather
    /// than about an empty dataset, or `None` when there is no such fact.
    ///
    /// This is the judgement [`Self::unrepresented_predicates`] is the evidence
    /// for: it is `None` both when every configured predicate was found and when
    /// the dataset holds no statement at all, and `Some` non-empty otherwise.
    pub fn shortfall(&self) -> Option<&[TermValue]> {
        if !self.dataset_holds_statements || self.unrepresented.is_empty() {
            return None;
        }
        Some(&self.unrepresented)
    }
}

// ---------------------------------------------------------------------------
// Partitions and documents
// ---------------------------------------------------------------------------

/// The `(graph, language)` pair whose corpus statistics are computed together.
///
/// Every BM25 input that describes a *corpus* rather than a document — the
/// document count, the average document length, a term's document frequency — is
/// computed within one partition and never across partitions. That is a
/// correctness decision, not a partitioning convenience. Pooling an English
/// corpus with a Japanese one makes an English needle's inverse document
/// frequency a function of how much Japanese happens to sit beside it, so adding
/// unrelated documents in another language silently reorders the English
/// results; and it prints two documents' scores as if they were comparable when
/// they were computed against different vocabularies.
///
/// `graph` is `None` for the default graph and `language` is `None` for
/// untagged literals; neither is a sentinel that a real value could collide
/// with.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartitionKey<P: CorpusProfile = GraphLanguage> {
    /// The graph name, or `None` for the default graph.
    graph: P::Graph,
    /// The literals' language tag, or `None` for untagged literals.
    language: Option<String>,
}

impl PartitionKey<GraphLanguage> {
    /// The partition for `graph` (`None` = the default graph) and `language`
    /// (`None` = untagged literals).
    pub const fn new(graph: Option<TermValue>, language: Option<String>) -> Self {
        Self { graph, language }
    }

    /// The graph name, or `None` for the default graph.
    pub const fn graph(&self) -> Option<&TermValue> {
        self.graph.as_ref()
    }
}

impl PartitionKey<GraphFree> {
    /// A language-only partition, with no graph coordinate.
    pub const fn for_language(language: Option<String>) -> Self {
        Self {
            graph: (),
            language,
        }
    }
}

impl<P: CorpusProfile> PartitionKey<P> {
    /// The graph name, or `None` for the default graph.
    pub(crate) fn graph_coordinate(&self) -> Option<&TermValue> {
        P::graph(&self.graph)
    }

    /// The language tag, or `None` for untagged literals.
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }
}

/// One partition's corpus statistics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartitionStats {
    /// How many documents contain terms in this independent scoring projection.
    document_count: u64,
    /// The total number of analyzed tokens across those documents.
    total_tokens: u64,
    /// `total_tokens / document_count`, exactly, in fixed point.
    average_document_length: Fixed,
}

impl PartitionStats {
    /// How many documents contain terms in this independent scoring projection.
    pub const fn document_count(&self) -> u64 {
        self.document_count
    }

    /// The total number of analyzed tokens across this partition's documents.
    pub const fn total_tokens(&self) -> u64 {
        self.total_tokens
    }

    /// The average document length, in tokens.
    ///
    /// Positive when the partition contains lexical documents; zero for a
    /// partition retained solely for auxiliary surface retrieval. Such documents
    /// contribute neither population nor tokens to this independent ranking law.
    pub const fn average_document_length(&self) -> Fixed {
        self.average_document_length
    }
}

/// One indexed document: the text of one `(graph, subject, language)` triple
/// key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document<P: CorpusProfile = GraphLanguage> {
    /// The graph the document's literals were read from, `None` for the default
    /// graph.
    graph: P::Graph,
    /// The subject the literals hang off — an IRI, a blank node, or (for a
    /// backend that admits one) a triple term. In the annotation layer this is
    /// the reifier.
    subject: TermValue,
    /// The language tag shared by the document's literals, `None` when they are
    /// untagged.
    language: Option<String>,
    /// How many tokens this scoring projection holds; zero for auxiliary-only documents.
    length: u64,
    /// Analyzed token counts by selected predicate ordinal.
    predicate_lengths: Vec<(u32, u64)>,
}

#[derive(Clone, Debug)]
struct PartitionRow<P: CorpusProfile> {
    key: PartitionKey<P>,
    stats: PartitionStats,
    members: u64,
    field_totals: Vec<u128>,
    field_populations: Vec<u64>,
}

impl Document<GraphLanguage> {
    /// The graph, or `None` for the default graph.
    pub const fn graph(&self) -> Option<&TermValue> {
        self.graph.as_ref()
    }
}

impl<P: CorpusProfile> Document<P> {
    /// The subject — the reifier, for text read from the annotation layer.
    pub const fn subject(&self) -> &TermValue {
        &self.subject
    }

    /// The language tag, or `None` for untagged literals.
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// How many tokens this scoring projection holds; zero for auxiliary-only documents.
    pub const fn length(&self) -> u64 {
        self.length
    }
}

/// One term's occurrences in one document.
#[derive(Clone, Debug)]
struct Posting<P: CorpusProfile> {
    /// Immutable document facts, independent of its current public rank.
    document: Shared<Document<P>>,
    /// The token positions the term occupies, ascending.
    positions: Vec<u32>,
    /// Frequencies by predicate ordinal, retained independently of ranking.
    predicate_frequencies: Vec<(u32, u64)>,
}

/// One shared spelling and its immutable ordered postings.
#[derive(Clone, Debug)]
struct TermEntry<P: CorpusProfile> {
    /// The analyzed term text.
    term: Shared<String>,
    /// Postings ordered by `(partition, document)`.
    postings: Table<Shared<Posting<P>>>,
}

// ---------------------------------------------------------------------------
// The index
// ---------------------------------------------------------------------------

/// A built, immutable inverted index over one dataset's literals.
///
/// # Auxiliary-only documents
///
/// Documents with surface or substring projections remain addressable even when
/// their scoring projection has no terms. They have length zero and do not enter
/// the BM25 population or length totals. An auxiliary-only partition has zero
/// scoring population and cannot contribute a posting to lexical retrieval.
/// # The empty index
///
/// An index holding no documents is well formed and needs no special case
/// anywhere. It holds zero documents, zero terms and **zero partitions** — a
/// partition is a `(graph, language)` pair carrying at least one document, so
/// there is no such thing as an empty partition and nothing divides by an average
/// document length that does not exist. It still attests a generation:
/// [`Self::fingerprint`] digests the configuration, the ranking law and the
/// analyzer's Unicode versions before it digests any content, so two empty
/// indexes under different configurations are distinguishable and the value
/// changes the moment the first document lands.
///
/// This is reachable rather than hypothetical. An empty dataset has interned no
/// term, so every configured predicate is absent from it, and
/// [`Self::from_dataset`] reads an absent predicate as no rows rather than as a
/// refusal — an index built before the documents it will hold have landed is an
/// ordinary operating state. A query against such an index returns zero rows
/// through exactly the path a needle that matches nothing takes.
///
/// An index that is empty because a configured predicate was *misspelled* is a
/// different state and reads as one: the dataset holds statements while the
/// configuration found none of them, which is a
/// [`SourceCoverage::shortfall`] rather than a corpus that has not landed yet.
#[derive(Clone, Debug)]
pub struct TextIndex<P: CorpusProfile = GraphLanguage> {
    /// The configuration this index was built under.
    config: TextIndexConfig<P>,
    /// Complete immutable fielded ranking law.
    ranking: RankingProfile,
    /// Selected predicate ordinals mapped to ranking fields.
    predicate_fields: Vec<usize>,
    /// Field lengths by document; derived without reanalysis.
    field_lengths: Table<Shared<Vec<u64>>>,
    /// The Unicode table versions the analyzer resolved against at build time.
    unicode: UnicodeVersions,
    /// Documents in id order — that is, sorted by `(graph, subject, language)`.
    documents: Table<Shared<Document<P>>>,
    /// Document ids sorted by `(subject, id)` — the side index a subject lookup
    /// binary-searches.
    ///
    /// Derived from [`Self::documents`] and therefore **not** part of either
    /// fingerprint: it holds no information the document table does not, and
    /// digesting it would only make the digest depend on an implementation
    /// detail. It exists because the document table is sorted with the *graph*
    /// first, so the documents sharing one subject are not contiguous in it.
    subject_order: Table<Shared<Document<P>>>,
    /// Partitions sorted by [`PartitionKey`], with their statistics.
    partitions: Table<Shared<PartitionRow<P>>>,
    /// The term dictionary, sorted by term text.
    terms: Table<Shared<TermEntry<P>>>,
    /// Original streams, sharing dictionary strings instead of re-analyzing documents.
    token_streams: TokenTable,
    /// Physical documents grouped by the logical occurrence key (subject, language).
    occurrence_order: Table<Shared<Document<P>>>,
    /// Distinct pre-stem words and bounded punctuation-bearing spans.
    surface: Option<SurfaceIndex<P>>,
    /// Source-aligned Han unigram and bigram occurrences for the independent producer.
    han_occurrences: Option<Table<SharedBuffer<crate::character::HanOccurrence>>>,
    /// The digest of everything that can change an answer.
    fingerprint: [u8; FINGERPRINT_BYTES],
    /// The digest of the source rows walked.
    source_fingerprint: [u8; FINGERPRINT_BYTES],
    /// Reversible source accounting, retained independently of its published hash.
    source_digest: SourceDigest,
    /// Original source-owned observation for independently discovering changes.
    source_version: Option<purrdf_core::source::SourceVersion>,
    /// Which configured predicates the build's walk found a statement for.
    ///
    /// Part of **neither** fingerprint, and deliberately: this says nothing about
    /// what the index answers. Two datasets whose literal rows agree answer alike
    /// whether or not one of them also holds a non-literal statement under a
    /// configured predicate, so folding this into either digest would make two
    /// indexes that give identical answers claim different identities. It is an
    /// observation about the source the walk saw, kept so a host holding only the
    /// index can still read it.
    coverage: SourceCoverage,
}

pub(crate) struct DocumentOrder<'a, P: CorpusProfile> {
    index: &'a TextIndex<P>,
    entries: &'a Table<Shared<Document<P>>>,
}

impl<P: CorpusProfile> DocumentOrder<'_, P> {
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
    pub(crate) fn get_owned(
        &self,
        at: usize,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<Option<u32>, purrdf_sparql_eval::EvalError> {
        self.entries
            .get(at)
            .map(|document| self.index.document_rank_owned(document, workspace))
            .transpose()
    }
    pub(crate) fn range_owned<'a>(
        &'a self,
        start: usize,
        end: usize,
        workspace: &'a purrdf_sparql_eval::WorkspaceCapability,
    ) -> impl Iterator<Item = Result<u32, purrdf_sparql_eval::EvalError>> + 'a {
        self.entries
            .slice(start, end)
            .iter()
            .map(move |document| self.index.document_rank_owned(document, workspace))
    }
}

impl<P: CorpusProfile> TextIndex<P> {
    /// The explicit physical document and partition law for this index.
    pub const fn corpus_profile(&self) -> &'static str {
        P::ID
    }

    /// Build an index over `dataset` as `config` describes.
    ///
    /// Both RDF 1.2 layers are read: the asserted triple table, and the
    /// annotation side table (whose subject is the reifier). An object that does
    /// not resolve to a literal is skipped in silence — a predicate may
    /// legitimately carry both literals and IRIs, and that is data rather than
    /// an error.
    ///
    /// # Errors
    ///
    /// [`TextError::Data`] if a term nests triple terms past the encoder's depth
    /// bound, or if the dataset yields more than `u32::MAX` documents or a
    /// document longer than `u32::MAX` tokens. [`TextError::Overflow`] if a
    /// partition's average document length does not fit in [`Fixed`].
    ///
    /// ## An absent predicate contributes no rows rather than refusing
    ///
    /// A configured predicate the dataset has not interned, and a
    /// [`GraphSelector::Named`] graph the dataset has not interned, each
    /// contribute nothing to the walk. Neither is an error, for a reason the
    /// limiting case makes plain: a dataset holding no quads has interned no term
    /// at all, so refusing an absent predicate would mean **no index can be built
    /// over an empty dataset** — and an index that exists before the documents it
    /// will hold have landed is an ordinary operating state, not a fault. It would
    /// also make the recommended single-partition configuration
    /// ([`GraphSelector::Named`] over one graph) the hardest one to start from,
    /// because the graph IRI is interned only once something is in the graph.
    ///
    /// So the index over an empty corpus builds, holds zero documents and zero
    /// partitions, still attests a generation (see [`Self::fingerprint`]), and
    /// answers every query with zero rows. Emptiness travels as the answer to a
    /// query rather than as a verdict at construction, which is the only form a
    /// consumer can read: it arrives with the generation it was computed against,
    /// and that generation moves the moment the first document lands.
    ///
    /// The cost is real and paid rather than named: a mistyped predicate IRI
    /// removes that predicate's share of the corpus, and without a detector it
    /// would do so quietly. The detector is not the presence check this replaced,
    /// which was never sound — it accepted any IRI the dataset interned *anywhere*,
    /// as a subject, as an object, in another predicate's statement, and it
    /// condemned a correctly spelled predicate whose objects are all IRIs, which
    /// carries no text and has never been an error. It is
    /// [`Self::source_coverage`], built out of the walk itself: every configured
    /// predicate no in-scope statement carried, together with whether the dataset
    /// held any statement at all. Those two facts separate the state this
    /// relaxation exists for from the mistake it would otherwise hide — a dataset
    /// holding nothing is an index waiting for its documents, and a dataset holding
    /// statements with none under a configured predicate is a
    /// [`SourceCoverage::shortfall`]. A host that needs to know it is holding the
    /// intended data asks [`verify_binding`](crate::verify_binding), which checks
    /// both the digest of the rows walked and that shortfall against the dataset in
    /// hand.
    pub fn from_dataset<D: DatasetView>(
        dataset: &D,
        config: &TextIndexConfig<P>,
    ) -> Result<Self, TextError>
    where
        D::ReadError: purrdf_hash::error::StructuredError,
    {
        let (rows, coverage) = collect_rows(dataset, config)?;
        let version = dataset.source_version().map_err(TextError::source_read)?;
        let source = SourceDigest::from_rows(&rows);
        let (documents, dictionary) = analyze_rows(&rows, config)?;
        let mut index = Self::assemble(
            config.clone(),
            documents,
            &dictionary,
            source,
            coverage,
            RankingProfile::single_field(),
        )?;
        index.source_version = version;
        dataset
            .checked_read(|_| ())
            .map_err(TextError::source_read)?;
        Ok(index)
    }

    /// Build directly under a fielded ranking law, including documents whose
    /// combined length exceeds the per-field bound.
    ///
    /// # Errors
    /// Propagates indexing errors and refuses incomplete routing or field bounds.
    pub fn from_dataset_with_ranking<D: DatasetView>(
        dataset: &D,
        config: &TextIndexConfig<P>,
        profile: RankingProfile,
    ) -> Result<Self, TextError>
    where
        D::ReadError: purrdf_hash::error::StructuredError,
    {
        for predicate in config.predicates() {
            profile.field_for(predicate)?;
        }
        let (rows, coverage) = collect_rows(dataset, config)?;
        let version = dataset.source_version().map_err(TextError::source_read)?;
        let source = SourceDigest::from_rows(&rows);
        let (documents, dictionary) = analyze_rows(&rows, config)?;
        let mut index = Self::assemble(
            config.clone(),
            documents,
            &dictionary,
            source,
            coverage,
            profile,
        )?;
        index.source_version = version;
        dataset
            .checked_read(|_| ())
            .map_err(TextError::source_read)?;
        Ok(index)
    }

    /// Apply a complete ranking profile without re-tokenizing or rebuilding
    /// postings. Predicate-level facts remain authoritative; only field lengths,
    /// corpus totals and the answer fingerprint are recomputed.
    ///
    /// # Errors
    /// Refuses incomplete predicate routing or a merged field longer than the
    /// profile permits. The consumed index is returned only after validation.
    pub fn with_ranking_profile(mut self, profile: RankingProfile) -> Result<Self, TextError> {
        self.ranking = profile;
        self.rebuild_field_statistics()?;
        self.fingerprint = self.compute_fingerprint();
        if let Some(surface) = &mut self.surface {
            surface.bind_generation(self.fingerprint);
        }
        Ok(self)
    }

    /// The named, immutable ranking law used for every score.
    pub const fn ranking_profile(&self) -> &RankingProfile {
        &self.ranking
    }

    /// Complete tokenization and surface-retrieval identity, independent of
    /// corpus, ranking and predicate routing.
    pub fn analyzer_fingerprint(&self) -> [u8; FINGERPRINT_BYTES] {
        self.analyzer().fingerprint()
    }

    /// Analyze a query under the same projection law used to build this index.
    ///
    /// # Errors
    /// Propagates configured analyzer input and encoding refusals.
    pub fn query_terms(&self, input: &str) -> Result<Vec<String>, TextError> {
        match self.config.projection {
            IndexProjection::Lexical => self.analyzer().terms(input),
            IndexProjection::Han => crate::character::query_terms(self.analyzer(), input),
        }
    }

    pub(crate) fn query_terms_owned(
        &self,
        input: &str,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<purrdf_sparql_eval::AdmittedVec<crate::query_workspace::QueryString>, TextError>
    {
        match self.config.projection {
            IndexProjection::Lexical => self.analyzer().terms_owned(input, workspace),
            IndexProjection::Han => {
                crate::character::query_terms_owned(self.analyzer(), input, workspace)
            }
        }
    }

    /// The exact analyzer every query relation uses.
    pub const fn analyzer(&self) -> &Analyzer {
        self.config.analyzer()
    }

    /// Substring and phonetic retrieval over distinct surface terms, including
    /// sorted document membership and inspectable exact match evidence.
    /// Returns `None` when the caller disabled surface indexing.
    pub const fn surface_index(&self) -> Option<&SurfaceIndex<P>> {
        self.surface.as_ref()
    }

    /// Exact field token totals for a partition, in ranking field order.
    pub fn field_totals(&self, partition: &PartitionKey<P>) -> Option<&[u128]> {
        self.partition_index(partition)
            .map(|at| self.partitions[at as usize].field_totals.as_slice())
    }

    /// Documents with positive analyzed length in each field, in ranking order.
    pub fn field_populations(&self, partition: &PartitionKey<P>) -> Option<&[u64]> {
        self.partition_index(partition)
            .map(|at| self.partitions[at as usize].field_populations.as_slice())
    }

    /// Admit a partition under its selected population law without walking documents.
    pub(crate) fn prepared_corpus(
        &self,
        partition: &PartitionKey<P>,
    ) -> Result<PreparedCorpus<'_>, TextError> {
        let at = self
            .partition_index(partition)
            .ok_or_else(|| TextError::data("ranking partition is absent"))?
            as usize;
        let documents = self.partitions[at].stats.document_count;
        let totals = &self.partitions[at].field_totals;
        if self.ranking.uses_field_populations() {
            PreparedCorpus::with_field_populations(
                &self.ranking,
                documents,
                totals,
                &self.partitions[at].field_populations,
            )
        } else {
            PreparedCorpus::new(&self.ranking, documents, totals)
        }
    }

    pub(crate) fn prepared_corpus_owned(
        &self,
        partition: &PartitionKey<P>,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<PreparedCorpus<'_>, TextError> {
        let partition =
            crate::query_workspace::admitted(self.partition_key_owned(partition, workspace))?
                .ok_or_else(|| {
                    query_error(
                        workspace,
                        NativeDiagnosticKind::Data,
                        "ranking partition is absent",
                    )
                    .unwrap_or_else(|failure| failure)
                })?;
        let at = self
            .local_partition_ordinal(partition)
            .expect("resolved index-owned partition");
        PreparedCorpus::from_index_owned(
            &self.ranking,
            self.partitions[at].stats.document_count,
            &self.partitions[at].field_totals,
            &self.partitions[at].field_populations,
            workspace,
        )
    }

    /// Original token stream in the current canonical dictionary order.
    /// Projection consumers must keep these positions. The iterator translates
    /// private stable addresses without copying or rewriting retained buffers.
    pub fn token_occurrences(
        &self,
        document: u32,
    ) -> Option<impl ExactSizeIterator<Item = TokenOccurrence> + Clone + '_> {
        self.token_streams.get(document as usize).map(|stream| {
            stream
                .as_slice()
                .iter()
                .map(move |token| token.projected(&self.terms))
        })
    }
    /// Bound all consecutive positional derivations, including graph aliases.
    /// Each start contributes the product of physical choices through the rest
    /// of its contiguous run. Any one nonempty phrase selects a subset of those
    /// choices; empty phrases emit nothing. A run without aliases costs one row
    /// per start. Saturation denotes a genuinely unrepresentable worst-case bag.
    pub(crate) fn phrase_row_bound(&self) -> u64 {
        let mut positions = std::collections::BTreeMap::new();
        for (document, stream) in self.documents.iter().zip(self.token_streams.iter()) {
            for token in stream.as_slice() {
                let count = positions
                    .entry((
                        &document.subject,
                        document.language.as_deref(),
                        token.position,
                    ))
                    .or_insert(0_u64);
                *count = count.saturating_add(1);
            }
        }
        let mut next: Option<(&TermValue, Option<&str>, u32)> = None;
        let mut suffix = 1_u64;
        let mut bound = 0_u64;
        for ((subject, language, position), count) in positions.into_iter().rev() {
            if !next.is_some_and(|(next_subject, next_language, next_position)| {
                subject == next_subject
                    && language == next_language
                    && position.checked_add(1) == Some(next_position)
            }) {
                suffix = 1;
            }
            suffix = suffix.saturating_mul(count);
            bound = bound.saturating_add(suffix);
            next = Some((subject, language, position));
        }
        bound
    }
    /// The sole normalized dictionary spelling of an occurrence's term.
    pub fn occurrence_term(&self, term: u32) -> Option<&str> {
        self.terms.get(term as usize).map(|term| term.term.as_str())
    }
    pub(crate) fn occurrence_order(&self) -> DocumentOrder<'_, P> {
        DocumentOrder {
            index: self,
            entries: &self.occurrence_order,
        }
    }

    pub(crate) fn han_occurrences(
        &self,
        document: u32,
    ) -> impl Iterator<Item = &crate::character::HanOccurrence> {
        self.han_occurrences
            .as_ref()
            .and_then(|table| table.get(document as usize))
            .map_or(&[][..], SharedBuffer::as_slice)
            .iter()
    }

    /// A document's field lengths, in ranking field order.
    pub fn field_lengths(&self, document: u32) -> Option<&[u64]> {
        self.field_lengths
            .get(document as usize)
            .map(|lengths| lengths.as_slice())
    }

    /// Nonzero `(predicate ordinal, token count)` facts, in configuration order. These
    /// facts do not change when the ranking profile changes.
    pub fn predicate_lengths(&self, document: u32) -> Option<&[(u32, u64)]> {
        self.documents
            .get(document as usize)
            .map(|doc| doc.predicate_lengths.as_slice())
    }

    /// Fill an inline field buffer, spilling for the profile's actual field count.
    pub(crate) fn field_inputs(&self, document: u32, term: &str) -> Result<FieldInputs, TextError> {
        if self.document(document).is_none() {
            return Err(TextError::data("field input names an absent document"));
        }
        let frequencies = self.posted_frequencies(document, term).unwrap_or(&[]);
        self.field_inputs_from_counts(document, frequencies)
    }

    /// The predicate frequencies of `term`'s posting for `document`, or `None`
    /// when the document holds no posting for it — including a term the
    /// dictionary does not hold and an id naming no document.
    ///
    /// One search over the dictionary and one over the ordered postings:
    /// the point lookup that decides
    /// membership, returning the very facts the scorer reads, so a caller that
    /// both decides and scores searches once rather than twice.
    pub(crate) fn posted_frequencies(&self, document: u32, term: &str) -> Option<&[(u32, u64)]> {
        self.document_posting(document, term)
            .map(|posting| posting.predicate_frequencies.as_slice())
    }

    pub(crate) fn posted_frequencies_owned(
        &self,
        document: u32,
        term: &str,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<Option<&[(u32, u64)]>, purrdf_sparql_eval::EvalError> {
        let Some(document) = self.document(document) else {
            return Ok(None);
        };
        let Some(entry) = self.term_entry(term) else {
            return Ok(None);
        };
        let found = entry.postings.try_binary_search_by(|posting| {
            compare_documents_owned(&posting.document, document, true, workspace)
        })?;
        Ok(found
            .ok()
            .map(|at| entry.postings[at].predicate_frequencies.as_slice()))
    }

    /// Assemble field inputs from a posting already located by the query walk.
    /// This avoids a dictionary lookup per candidate and query term.
    pub(crate) fn field_inputs_from_counts(
        &self,
        document: u32,
        frequencies: &[(u32, u64)],
    ) -> Result<FieldInputs, TextError> {
        self.field_inputs_from_counts_with(document, frequencies, None)
    }

    pub(crate) fn field_inputs_from_counts_owned(
        &self,
        document: u32,
        frequencies: &[(u32, u64)],
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<FieldInputs, TextError> {
        self.field_inputs_from_counts_with(
            document,
            frequencies,
            workspace.is_bounded().then_some(workspace),
        )
    }

    pub(crate) fn field_inputs_from_counts_with(
        &self,
        document: u32,
        frequencies: &[(u32, u64)],
        workspace: Option<&purrdf_sparql_eval::WorkspaceCapability>,
    ) -> Result<FieldInputs, TextError> {
        let lengths = self
            .field_lengths
            .get(document as usize)
            .ok_or_else(|| data_error("field input names an absent document", workspace))?;
        let resident = purrdf_sparql_eval::WorkspaceCapability::default();
        let mut fields = FieldInputs::new(lengths.len(), workspace.unwrap_or(&resident))?;
        for (input, &length) in fields.iter_mut().zip(lengths.iter()) {
            input.length = length;
        }
        for &(predicate, frequency) in frequencies {
            let input = &mut fields[self.predicate_fields[predicate as usize]];
            input.term_frequency =
                input.term_frequency.checked_add(frequency).ok_or_else(|| {
                    crate::query_workspace::arithmetic_error(
                        crate::arithmetic::ArithmeticRefusal::FieldFrequency {
                            document,
                            predicate,
                            current: input.term_frequency,
                            added: frequency,
                        },
                        workspace,
                    )
                })?;
        }
        Ok(fields)
    }

    /// Retained predicate frequencies along a term's existing posting walk.
    ///
    /// The walk is [`Self::partition_postings`], shared with [`Self::postings`]; this
    /// projection yields each posting's per-predicate frequencies, the BM25F field
    /// statistics input, where `postings` yields its token positions.
    pub(crate) fn field_postings_owned<'a>(
        &'a self,
        partition: &PartitionKey<P>,
        term: &str,
        workspace: &'a purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<
        impl Iterator<Item = Result<FieldPosting<'a>, purrdf_sparql_eval::EvalError>> + use<'a, P>,
        purrdf_sparql_eval::EvalError,
    > {
        Ok(self
            .partition_postings_owned(partition, term, workspace)?
            .iter()
            .map(move |posting| {
                self.document_rank_owned(&posting.document, workspace)
                    .map(|document| (document, posting.predicate_frequencies.as_slice()))
            }))
    }

    /// Derive field statistics from retained predicate facts, validating every
    /// document before any query (including an empty query) can observe it.
    fn rebuild_field_statistics(&mut self) -> Result<(), TextError> {
        self.predicate_fields = self
            .config
            .predicates
            .iter()
            .map(|predicate| self.ranking.field_for(predicate))
            .collect::<Result<_, _>>()?;
        self.partitions = Table::default();
        let documents = self.documents.clone();
        let mut field_lengths = Vec::with_capacity(documents.len());
        for document in documents.iter() {
            let mut lengths = vec![0_u64; self.ranking.fields().len()];
            for &(predicate, length) in &document.predicate_lengths {
                let field = self.predicate_fields[predicate as usize];
                lengths[field] = lengths[field].checked_add(length).ok_or_else(|| {
                    crate::query_workspace::arithmetic_error(
                        crate::arithmetic::ArithmeticRefusal::FieldLength {
                            field,
                            current: lengths[field],
                            added: length,
                        },
                        None,
                    )
                })?;
            }
            self.adjust_partition(document, &lengths, true)?;
            field_lengths.push(share_index(lengths)?);
        }
        self.field_lengths = Table::from_streams(field_lengths)?;
        for row in self.partitions.iter() {
            self.prepared_corpus(&row.key)?;
        }
        Ok(())
    }

    fn adjust_partition(
        &mut self,
        document: &Document<P>,
        fields: &[u64],
        added: bool,
    ) -> Result<(), TextError> {
        let position = self.partitions.binary_search_by(|row| {
            (&row.key.graph, &row.key.language).cmp(&(&document.graph, &document.language))
        });
        let mut row = if let Ok(at) = position {
            (*self.partitions[at]).clone()
        } else {
            if !added {
                return Err(TextError::data("removed document has no partition"));
            }
            PartitionRow {
                key: PartitionKey {
                    graph: document.graph.clone(),
                    language: document.language.clone(),
                },
                stats: PartitionStats {
                    document_count: 0,
                    total_tokens: 0,
                    average_document_length: Fixed::ZERO,
                },
                members: 0,
                field_totals: vec![0; self.ranking.fields().len()],
                field_populations: vec![0; self.ranking.fields().len()],
            }
        };
        let adjust = |before: u64, amount| {
            if added {
                before.checked_add(amount)
            } else {
                before.checked_sub(amount)
            }
            .ok_or_else(|| TextError::data("partition population or token count is inconsistent"))
        };
        row.members = adjust(row.members, 1)?;
        row.stats.document_count =
            adjust(row.stats.document_count, u64::from(document.length != 0))?;
        row.stats.total_tokens = adjust(row.stats.total_tokens, document.length)?;
        if fields.len() != row.field_totals.len() {
            return Err(TextError::data("partition field shape is inconsistent"));
        }
        for ((total, population), &length) in row
            .field_totals
            .iter_mut()
            .zip(&mut row.field_populations)
            .zip(fields)
        {
            *total = if added {
                total.checked_add(u128::from(length))
            } else {
                total.checked_sub(u128::from(length))
            }
            .ok_or_else(|| TextError::data("partition field token total is inconsistent"))?;
            *population = adjust(*population, u64::from(length != 0))?;
        }
        if row.members == 0 {
            self.partitions = self
                .partitions
                .remove(position.expect("a removed partition was present"))?;
        } else {
            row.stats.average_document_length = if row.stats.document_count == 0 {
                Fixed::ZERO
            } else {
                exact_average(row.stats.total_tokens, row.stats.document_count)?
            };
            let row = share_index(row)?;
            self.partitions = match position {
                Ok(at) => self.partitions.replace(at, row)?,
                Err(at) => self.partitions.insert(at, row)?,
            };
        }
        Ok(())
    }

    /// The configuration this index was built under.
    pub const fn config(&self) -> &TextIndexConfig<P> {
        &self.config
    }

    /// The Unicode table versions the analyzer resolved against at build time.
    ///
    /// Part of [`Self::fingerprint`], because raising any of them can change
    /// which literals produce which terms.
    pub const fn unicode_versions(&self) -> UnicodeVersions {
        self.unicode
    }

    /// How many documents the index retains, across every partition.
    pub fn document_count(&self) -> u64 {
        self.documents.len() as u64
    }

    /// How many partitions the index holds.
    pub fn partition_count(&self) -> u64 {
        self.partitions.len() as u64
    }

    /// How many distinct terms the dictionary holds.
    pub fn term_count(&self) -> u64 {
        self.terms.len() as u64
    }

    /// The largest [`PartitionStats::document_count`] of any partition, or `0`
    /// for an empty index.
    ///
    /// A scorer sizing a working set once, rather than per partition, wants this
    /// bound.
    pub fn max_documents_in_any_partition(&self) -> u64 {
        self.partitions
            .iter()
            .map(|row| row.stats.document_count)
            .max()
            .unwrap_or(0)
    }

    /// Every partition and its statistics, ascending by [`PartitionKey`].
    pub fn partitions(&self) -> impl Iterator<Item = (&PartitionKey<P>, &PartitionStats)> {
        self.partitions.iter().map(|row| (&row.key, &row.stats))
    }

    /// One partition's statistics, or `None` if the index has no such partition.
    pub fn partition_stats(&self, partition: &PartitionKey<P>) -> Option<&PartitionStats> {
        self.partition_index(partition)
            .and_then(|index| self.partitions.get(index as usize))
            .map(|row| &row.stats)
    }

    /// Every document, in id order.
    pub fn documents(&self) -> impl Iterator<Item = &Document<P>> {
        self.documents.iter().map(|document| &**document)
    }

    /// The document `id` names, or `None` if there is no such document.
    pub fn document(&self, id: u32) -> Option<&Document<P>> {
        self.documents.get(id as usize).map(|document| &**document)
    }

    /// Every document id whose subject is `subject`, ascending.
    ///
    /// A subject is not a document: `(graph, subject, language)` is, so one
    /// subject carrying `"the cat"@en` and `"le chat"@fr` in two named graphs is
    /// four documents. This is the lookup that turns the one into the other, in
    /// Ordered tree lookups avoid scanning the document table; result ordinals
    /// are projected from the current immutable document order.
    ///
    /// Empty for a subject the index holds no text for, which is the true answer
    /// rather than a failure: a query may legitimately name a subject that has
    /// no indexed literals.
    pub fn documents_with_subject<'a>(
        &'a self,
        subject: &TermValue,
    ) -> impl ExactSizeIterator<Item = u32> + Clone + use<'a, P> {
        let compare = |document: &Shared<Document<P>>| document.subject.cmp(subject);
        let start = self
            .subject_order
            .partition_point(|id| compare(id) == Ordering::Less);
        let end = self
            .subject_order
            .partition_point(|id| compare(id) != Ordering::Greater);
        self.subject_order
            .slice(start, end)
            .iter()
            .map(move |document| self.document_rank(document))
    }

    /// The existing two subject-side binary searches with admitted native term
    /// ordering. After a comparator failure the partition result is discarded;
    /// the first typed failure is returned before any caller can use the slice.
    pub(crate) fn documents_with_subject_owned(
        &self,
        subject: &TermValue,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<purrdf_sparql_eval::AdmittedVec<u32>, purrdf_sparql_eval::EvalError> {
        fn partition<P: CorpusProfile>(
            index: &TextIndex<P>,
            subject: &TermValue,
            workspace: &purrdf_sparql_eval::WorkspaceCapability,
            upper: bool,
        ) -> Result<usize, purrdf_sparql_eval::EvalError> {
            let mut failure = None;
            let result = index.subject_order.partition_point(|document| {
                if failure.is_some() {
                    return false;
                }
                match workspace.terms_cmp(&document.subject, subject) {
                    Ok(order) => {
                        if upper {
                            order != Ordering::Greater
                        } else {
                            order == Ordering::Less
                        }
                    }
                    Err(error) => {
                        failure = Some(error);
                        false
                    }
                }
            });
            match failure {
                Some(error) => Err(error),
                None => Ok(result),
            }
        }
        let start = partition(self, subject, workspace, false)?;
        let end = partition(self, subject, workspace, true)?;
        let mut result = purrdf_sparql_eval::AdmittedVec::with_capacity(end - start, workspace)?;
        for document in self.subject_order.slice(start, end).iter() {
            result.push(self.document_rank_owned(document, workspace)?)?;
        }
        Ok(result)
    }

    /// Borrow one immutable positional posting without copying the caller's index.
    pub(crate) fn posting_at_owned(
        &self,
        partition: usize,
        term: &str,
        ordinal: usize,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<Option<(u32, &[u32])>, purrdf_sparql_eval::EvalError> {
        let Some(row) = self.partitions.get(partition) else {
            return Ok(None);
        };
        let Some(posting) = self
            .partition_postings_owned(&row.key, term, workspace)?
            .get(ordinal)
        else {
            return Ok(None);
        };
        Ok(Some((
            self.document_rank_owned(&posting.document, workspace)?,
            &posting.positions,
        )))
    }

    /// The partitions holding a document whose subject is `subject`, ascending
    /// and distinct.
    ///
    /// This is what makes a bound document position a **partition** restriction
    /// rather than a per-row filter. A query that names a document is answered
    /// over the partitions that subject actually appears in instead of every
    /// partition the index holds — and because ranks are per-partition,
    /// dropping the rest cannot change the rank of any row that survives.
    ///
    /// It is the whole of the narrowing for a relation whose rows are the
    /// postings themselves. A relation whose rows are *ranked documents* narrows
    /// further, because for it a partition is worth ranking only where the
    /// subject's document in it holds a needle term at all: see
    /// [`Self::term_frequency`], which decides that in one binary search per
    /// term, and `TextSearchRelation::open`, which answers without ranking
    /// anything where the answer is none.
    ///
    /// Empty for a subject the index holds no text for, which admits no
    /// partition and so retrieves nothing — the same answer a per-row filter
    /// would have reached, arrived at without ranking anything.
    pub fn partitions_holding_subject(&self, subject: &TermValue) -> Vec<PartitionKey<P>> {
        let mut keys: Vec<PartitionKey<P>> = self
            .documents_with_subject(subject)
            .filter_map(|id| self.partition_key_of(id).cloned())
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }

    /// The partition the document `id` names belongs to.
    pub fn partition_key_of(&self, id: u32) -> Option<&PartitionKey<P>> {
        let document = self.documents.get(id as usize)?;
        self.partitions
            .get(self.document_partition(document))
            .map(|row| &row.key)
    }

    pub(crate) fn partition_ordinal_of_owned(
        &self,
        document: u32,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<Option<usize>, purrdf_sparql_eval::EvalError> {
        let Some(document) = self.document(document) else {
            return Ok(None);
        };
        let result = self.partitions.try_binary_search_by(|row| {
            Ok::<_, purrdf_sparql_eval::EvalError>(
                compare_graphs_owned(
                    P::graph(&row.key.graph),
                    P::graph(&document.graph),
                    workspace,
                )?
                .then_with(|| row.key.language.cmp(&document.language)),
            )
        })?;
        Ok(result.ok())
    }

    pub(crate) fn partition_key_of_owned(
        &self,
        document: u32,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<Option<&PartitionKey<P>>, purrdf_sparql_eval::EvalError> {
        Ok(self
            .partition_ordinal_of_owned(document, workspace)?
            .map(|at| &self.partitions[at].key))
    }

    /// Resolve an actual index-owned key borrow to its stable ordinal.
    pub(crate) fn local_partition_ordinal(&self, partition: &PartitionKey<P>) -> Option<usize> {
        let at = self
            .partitions
            .binary_search_by(|row| row.key.cmp(partition))
            .ok()?;
        core::ptr::eq(&raw const self.partitions[at].key, partition).then_some(at)
    }

    /// Resolve an external key through fallible native graph-term comparison.
    /// A failed comparator's search result is discarded before publication.
    pub(crate) fn partition_key_owned(
        &self,
        partition: &PartitionKey<P>,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<Option<&PartitionKey<P>>, purrdf_sparql_eval::EvalError> {
        let mut failure = None;
        let result = self.partitions.binary_search_by(|row| {
            let key = &row.key;
            if failure.is_some() {
                return Ordering::Equal;
            }
            let graph = match (key.graph_coordinate(), partition.graph_coordinate()) {
                (None, None) => Ok(Ordering::Equal),
                (None, Some(_)) => Ok(Ordering::Less),
                (Some(_), None) => Ok(Ordering::Greater),
                (Some(left), Some(right)) => workspace.terms_cmp(left, right),
            };
            match graph {
                Ok(graph) => graph.then_with(|| key.language().cmp(&partition.language())),
                Err(error) => {
                    failure = Some(error);
                    Ordering::Equal
                }
            }
        });
        match failure {
            Some(error) => Err(error),
            None => Ok(result.ok().map(|ordinal| &self.partitions[ordinal].key)),
        }
    }

    /// How many analyzed tokens the document `id` names holds, or `None` if
    /// there is no such document. Auxiliary-only documents have length zero.
    pub fn document_length(&self, id: u32) -> Option<u64> {
        self.document(id).map(Document::length)
    }

    /// Every analyzed term in the dictionary, ascending.
    pub fn terms(&self) -> impl Iterator<Item = &str> {
        self.terms.iter().map(|entry| entry.term.as_str())
    }

    /// How many of `partition`'s documents contain `term`.
    ///
    /// Zero for a term the dictionary does not hold and for a partition the
    /// index does not hold — in both readings that is the true count, because no
    /// document there contains it.
    pub fn document_frequency(&self, partition: &PartitionKey<P>, term: &str) -> u64 {
        self.partition_postings(partition, term).len() as u64
    }

    pub(crate) fn document_frequency_owned(
        &self,
        partition: &PartitionKey<P>,
        term: &str,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<u64, purrdf_sparql_eval::EvalError> {
        Ok(self
            .partition_postings_owned(partition, term, workspace)?
            .len() as u64)
    }

    /// The postings of `term` within `partition`, ascending by document id.
    ///
    /// Each item is a document id and that document's ascending token positions
    /// for the term — everything a phrase or proximity match needs, and the
    /// occurrence rows a property function emits.
    pub fn postings<'a>(
        &'a self,
        partition: &PartitionKey<P>,
        term: &str,
    ) -> impl Iterator<Item = (u32, &'a [u32])> + use<'a, P> {
        self.partition_postings(partition, term)
            .iter()
            .map(|posting| {
                (
                    self.document_rank(&posting.document),
                    posting.positions.as_slice(),
                )
            })
    }

    /// How many times `term` occurs in the document `id` names.
    ///
    /// Zero when the term does not occur there, and zero for an id that names no
    /// document — a caller that needs to tell those apart asks
    /// [`Self::document`] first.
    pub fn term_frequency(&self, id: u32, term: &str) -> u64 {
        self.document_posting(id, term)
            .map_or(0, |posting| posting.positions.len() as u64)
    }

    /// Query-time membership of a token in a caller-declared predicate field.
    ///
    /// This reads retained predicate facts, never a ranking weight or a field id
    /// stored in postings. A predicate absent from the selected index configuration
    /// is a configuration error; a declared predicate with no matching occurrence
    /// has frequency zero.
    ///
    /// # Errors
    /// Refuses a predicate this index was not configured to carry.
    pub fn term_frequency_in_predicate(
        &self,
        id: u32,
        term: &str,
        predicate: &TermValue,
    ) -> Result<u64, TextError> {
        let ordinal = self.predicate_ordinal(predicate)?;
        Ok(self.document_posting(id, term).map_or(0, |posting| {
            predicate_frequency(&posting.predicate_frequencies, ordinal)
        }))
    }

    pub(crate) fn term_frequency_in_predicate_owned(
        &self,
        id: u32,
        term: &str,
        predicate: &TermValue,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<u64, TextError> {
        let ordinal = self.predicate_ordinal(predicate)?;
        Ok(
            crate::query_workspace::admitted(self.posted_frequencies_owned(id, term, workspace))?
                .map_or(0, |frequencies| predicate_frequency(frequencies, ordinal)),
        )
    }

    fn predicate_ordinal(&self, predicate: &TermValue) -> Result<u32, TextError> {
        let ordinal = self
            .config
            .predicates
            .binary_search(predicate)
            .map_err(|_| {
                TextError::config("field scope predicate is not selected by this text index")
            })?;
        u32::try_from(ordinal).map_err(|cause| {
            crate::query_workspace::arithmetic_error(
                crate::arithmetic::ArithmeticRefusal::PredicateOrdinal { ordinal, cause },
                None,
            )
        })
    }

    fn document_posting(&self, id: u32, term: &str) -> Option<&Posting<P>> {
        let document = self.documents.get(id as usize)?;
        let entry = self.term_entry(term)?;
        entry
            .postings
            .binary_search_by(|posting| compare_posting_documents(&posting.document, document))
            .ok()
            .and_then(|at| entry.postings.get(at))
            .map(|posting| &**posting)
    }

    fn document_rank(&self, document: &Document<P>) -> u32 {
        self.documents
            .binary_search_by(|candidate| compare_document_keys(candidate, document))
            .expect("a posting's immutable document is live") as u32
    }

    fn document_rank_owned(
        &self,
        document: &Document<P>,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<u32, purrdf_sparql_eval::EvalError> {
        let result = self.documents.try_binary_search_by(|candidate| {
            compare_documents_owned(candidate, document, false, workspace)
        })?;
        result.map(|at| at as u32).map_err(|_| {
            purrdf_sparql_eval::NativeDiagnostic::error(
                NativeDiagnosticKind::Internal,
                format_args!("an index-owned document is absent from canonical order"),
                workspace,
            )
        })
    }

    fn document_partition(&self, document: &Document<P>) -> usize {
        self.partitions
            .binary_search_by(|row| {
                (&row.key.graph, &row.key.language).cmp(&(&document.graph, &document.language))
            })
            .expect("a live document has its partition")
    }

    fn edit_document_orders(
        &mut self,
        document: &Shared<Document<P>>,
        added: bool,
    ) -> Result<(), TextError> {
        for (table, compare) in [
            (
                &mut self.subject_order,
                compare_subject_documents::<P> as fn(&Document<P>, &Document<P>) -> Ordering,
            ),
            (
                &mut self.occurrence_order,
                compare_occurrence_documents::<P> as fn(&Document<P>, &Document<P>) -> Ordering,
            ),
        ] {
            let at = table.binary_search_by(|candidate| compare(candidate, document));
            *table = if added {
                table.insert(
                    at.expect_err("an inserted document is absent from its side order"),
                    document.clone(),
                )?
            } else {
                table.remove(at.expect("a removed document is present in its side order"))?
            };
        }
        Ok(())
    }

    /// The digest of everything about this index that can change an answer.
    ///
    /// It covers the configuration (the sorted predicates and the graph
    /// selector and enabled projections), the complete analyzer and ranking
    /// laws, all four Unicode table versions, and the exact source multiset
    /// identity. These inputs determine the document table, dictionary, postings,
    /// positions and partition statistics. Independent builds over the same
    /// inputs agree; changing an input that can move an answer moves the identity.
    /// Computing it visits only the configuration, independent of corpus size.
    ///
    /// An index holding nothing still has one, and it is not a placeholder: the
    /// configuration, the ranking law and the Unicode versions are digested before
    /// any content, so an empty index attests a generation that distinguishes it
    /// from an empty index under another configuration, and that moves as soon as
    /// the first document lands. Emptiness is a state of the index, never an
    /// absence of its identity.
    ///
    /// See this module's documentation for the one caveat: blank-node labels are
    /// terms here, so two isomorphic datasets with different labels disagree.
    pub const fn fingerprint(&self) -> [u8; FINGERPRINT_BYTES] {
        self.fingerprint
    }

    /// The digest of the `(graph, subject, predicate, literal)` rows this index
    /// was built by walking.
    ///
    /// [`Self::fingerprint`] identifies the index; this identifies the *data
    /// under* it. The two are separate because pairing an index with the wrong
    /// dataset is otherwise a silent wrong answer rather than a failure: the
    /// relation emits perfectly well-formed document terms, those terms join
    /// back by basic graph pattern against a dataset that never contained them,
    /// zero rows come out, and no layer anywhere has anything to report.
    /// Recomputing this digest over the dataset in hand and comparing detects
    /// exactly that.
    pub const fn source_fingerprint(&self) -> [u8; FINGERPRINT_BYTES] {
        self.source_fingerprint
    }

    /// Which configured predicates the build's walk found a statement for.
    ///
    /// [`Self::source_fingerprint`] identifies the data under this index; this says
    /// whether the configuration found all of it. A configured predicate with no
    /// in-scope statement is not an error at construction (see
    /// [`Self::from_dataset`]), so this is where a mistyped predicate IRI becomes
    /// visible: [`SourceCoverage::shortfall`] names it, and names nothing for an
    /// index built over a dataset that simply holds nothing yet.
    ///
    /// This is the build's observation, so it answers without the dataset in hand
    /// and without re-walking a corpus. To ask the same question of the dataset a
    /// query is about to run against — which is a different question, because the
    /// dataset may have moved — use [`verify_binding`](crate::verify_binding).
    pub const fn source_coverage(&self) -> &SourceCoverage {
        &self.coverage
    }

    /// The index of `partition` in [`Self::partitions`], if the index holds it.
    fn partition_index(&self, partition: &PartitionKey<P>) -> Option<u32> {
        if let Some(ordinal) = self.local_partition_ordinal(partition) {
            return u32::try_from(ordinal).ok();
        }
        self.partitions
            .binary_search_by(|row| row.key.cmp(partition))
            .ok()
            .map(|at| at as u32)
    }

    /// The dictionary entry for `term`, if it holds one.
    fn term_entry(&self, term: &str) -> Option<&TermEntry<P>> {
        self.terms
            .binary_search_by(|entry| entry.term.as_str().cmp(term))
            .ok()
            .and_then(|at| self.terms.get(at))
            .map(|entry| &**entry)
    }

    /// `term`'s postings restricted to `partition`; empty when either is absent.
    fn partition_postings(
        &self,
        partition: &PartitionKey<P>,
        term: &str,
    ) -> Slice<'_, Shared<Posting<P>>> {
        let Some(entry) = self.term_entry(term) else {
            return Slice::empty();
        };
        let compare = |posting: &Shared<Posting<P>>| {
            (&posting.document.graph, &posting.document.language)
                .cmp(&(&partition.graph, &partition.language))
        };
        let start = entry
            .postings
            .partition_point(|posting| compare(posting).is_lt());
        let end = entry
            .postings
            .partition_point(|posting| !compare(posting).is_gt());
        entry.postings.slice(start, end)
    }

    fn partition_postings_owned(
        &self,
        partition: &PartitionKey<P>,
        term: &str,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<Slice<'_, Shared<Posting<P>>>, purrdf_sparql_eval::EvalError> {
        let Some(entry) = self.term_entry(term) else {
            return Ok(Slice::empty());
        };
        let compare =
            |posting: &Shared<Posting<P>>| -> Result<Ordering, purrdf_sparql_eval::EvalError> {
                Ok(compare_graphs_owned(
                    P::graph(&posting.document.graph),
                    P::graph(&partition.graph),
                    workspace,
                )?
                .then_with(|| posting.document.language.cmp(&partition.language)))
            };
        let start = entry
            .postings
            .try_partition_point(|posting| compare(posting).map(Ordering::is_lt))?;
        let end = entry
            .postings
            .try_partition_point(|posting| compare(posting).map(|order| !order.is_gt()))?;
        Ok(entry.postings.slice(start, end))
    }

    /// Turn sorted documents and an intern-ordered dictionary into the finished
    /// index: partitions, statistics, postings and the fingerprint.
    fn assemble(
        config: TextIndexConfig<P>,
        documents: Vec<AnalyzedDocument<P>>,
        dictionary: &[String],
        source: SourceDigest,
        coverage: SourceCoverage,
        ranking: RankingProfile,
    ) -> Result<Self, TextError> {
        check_document_count(documents.len())?;

        let table: Vec<Shared<Document<P>>> = documents
            .iter()
            .map(|document| {
                share_index(Document {
                    graph: document.key.graph.clone(),
                    subject: document.key.subject.clone(),
                    language: document.key.language.clone(),
                    length: document.tokens.len() as u64,
                    predicate_lengths: document.predicate_lengths.clone(),
                })
            })
            .collect::<Result<_, _>>()?;
        let (terms, token_streams) = build_terms(&documents, dictionary, &table)?;
        let table = Table::from_streams(table)?;
        let occurrence_order = build_occurrence_order(&table)?;
        let subject_order = build_subject_order(&table)?;
        let mut analyses = Vec::new();
        let mut han_buffers = Vec::new();
        for (id, document) in documents.into_iter().enumerate() {
            let mut han_occurrences = Vec::new();
            for (literal, analysis) in document.analyses {
                if config.projection == IndexProjection::Han {
                    han_occurrences.extend(
                        crate::character::index_projections(&analysis)
                            .into_iter()
                            .map(|projection| crate::character::HanOccurrence {
                                literal,
                                projection,
                            }),
                    );
                }
                if config.surface {
                    analyses.push((id as u32, literal, analysis));
                }
            }
            if config.projection == IndexProjection::Han {
                han_buffers.push(share_buffer(han_occurrences)?);
            }
        }
        let han_occurrences = if config.projection == IndexProjection::Han {
            Some(Table::from_streams(han_buffers)?)
        } else {
            None
        };
        let surface = if config.surface {
            Some(SurfaceIndex::from_index_analyses(
                config.analyzer.clone(),
                table.clone(),
                analyses,
            )?)
        } else {
            None
        };

        let mut index = Self {
            config,
            ranking,
            predicate_fields: Vec::new(),
            field_lengths: Table::default(),
            unicode: unicode_versions(),
            documents: table,
            subject_order,
            partitions: Table::default(),
            terms: Table::from_streams(terms)?,
            token_streams: TokenTable::from_streams(token_streams)?,
            occurrence_order,
            surface,
            han_occurrences,
            fingerprint: [0; FINGERPRINT_BYTES],
            source_fingerprint: source.fingerprint::<P>(),
            source_digest: source,
            source_version: None,
            coverage,
        };
        index.rebuild_field_statistics()?;
        index.fingerprint = index.compute_fingerprint();
        if let Some(surface) = &mut index.surface {
            surface.bind_generation(index.fingerprint);
        }
        Ok(index)
    }

    /// Bind the complete inputs of the deterministic index construction.
    /// Derived tables add no independent information: hashing them again would
    /// turn a one-document update into a whole-corpus traversal. In particular,
    /// the source identity binds original literals, not merely corpus statistics.
    fn compute_fingerprint(&self) -> [u8; FINGERPRINT_BYTES] {
        let mut digest = Digest::new(if P::GRAPH_FREE {
            TRIPLE_INDEX_DIGEST_DOMAIN
        } else {
            INDEX_DIGEST_DOMAIN
        });
        digest.text(crate::ANALYZER_PROFILE_ID);
        digest.text(match self.config.projection {
            IndexProjection::Lexical => "lexical",
            IndexProjection::Han => crate::character::PROFILE_ID,
        });
        for byte in self.analyzer_fingerprint() {
            digest.tag(byte);
        }
        for byte in self.ranking.fingerprint() {
            digest.tag(byte);
        }
        // Alignment evidence distinguishes original spellings even when their
        // normalized terms coincide. The generation binds those source rows.
        for byte in self.source_fingerprint {
            digest.tag(byte);
        }

        digest.count(self.config.predicates.len());
        for predicate in &self.config.predicates {
            digest.term(predicate);
        }
        match P::selector(&self.config.graph) {
            Some(GraphSelector::Any) => digest.tag(SELECTOR_ANY),
            Some(GraphSelector::Default) => digest.tag(SELECTOR_DEFAULT),
            Some(GraphSelector::Named(name)) => {
                digest.tag(SELECTOR_NAMED);
                digest.term(name);
            }
            None => {}
        }

        for version in [
            self.unicode.core,
            self.unicode.normalization,
            self.unicode.case_folding,
            self.unicode.segmentation,
        ] {
            digest.number(version.major);
            digest.number(version.minor);
            digest.number(version.patch);
        }

        digest.tag(u8::from(self.config.surface));

        digest.finish()
    }
}

/// Document ids sorted by `(subject, id)`.
///
/// Sorted rather than hashed, and by a total order the document table already
/// carries, so the side index is a pure function of the document table exactly
/// as the document table is a pure function of the content. The `id` tiebreak
/// keeps a subject's run ascending, so a lookup's result is ascending too.
fn build_subject_order<P: CorpusProfile>(
    table: &Table<Shared<Document<P>>>,
) -> Result<Table<Shared<Document<P>>>, TextError> {
    let mut order: Vec<_> = table.iter().cloned().collect();
    order.sort_by(|a, b| compare_subject_documents(a, b));
    Table::from_streams(order)
}

fn compare_subject_documents<P: CorpusProfile>(a: &Document<P>, b: &Document<P>) -> Ordering {
    (&a.subject, &a.graph, &a.language).cmp(&(&b.subject, &b.graph, &b.language))
}

fn compare_occurrence_documents<P: CorpusProfile>(a: &Document<P>, b: &Document<P>) -> Ordering {
    (&a.subject, &a.language, &a.graph).cmp(&(&b.subject, &b.language, &b.graph))
}

pub(crate) fn compare_document_keys<P: CorpusProfile>(
    a: &Document<P>,
    b: &Document<P>,
) -> Ordering {
    (&a.graph, &a.subject, &a.language).cmp(&(&b.graph, &b.subject, &b.language))
}

fn compare_posting_documents<P: CorpusProfile>(a: &Document<P>, b: &Document<P>) -> Ordering {
    (&a.graph, &a.language, &a.subject).cmp(&(&b.graph, &b.language, &b.subject))
}

fn compare_graphs_owned(
    a: Option<&TermValue>,
    b: Option<&TermValue>,
    workspace: &purrdf_sparql_eval::WorkspaceCapability,
) -> Result<Ordering, purrdf_sparql_eval::EvalError> {
    match (a, b) {
        (None, None) => Ok(Ordering::Equal),
        (None, Some(_)) => Ok(Ordering::Less),
        (Some(_), None) => Ok(Ordering::Greater),
        (Some(a), Some(b)) => workspace.terms_cmp(a, b),
    }
}

fn compare_documents_owned<P: CorpusProfile>(
    a: &Document<P>,
    b: &Document<P>,
    posting: bool,
    workspace: &purrdf_sparql_eval::WorkspaceCapability,
) -> Result<Ordering, purrdf_sparql_eval::EvalError> {
    let graph = compare_graphs_owned(P::graph(&a.graph), P::graph(&b.graph), workspace)?;
    if graph != Ordering::Equal {
        return Ok(graph);
    }
    let language = a.language.cmp(&b.language);
    if posting && language != Ordering::Equal {
        return Ok(language);
    }
    Ok(workspace.terms_cmp(&a.subject, &b.subject)?.then(language))
}

fn predicate_frequency(frequencies: &[(u32, u64)], ordinal: u32) -> u64 {
    frequencies
        .binary_search_by_key(&ordinal, |entry| entry.0)
        .ok()
        .map_or(0, |at| frequencies[at].1)
}

// ---------------------------------------------------------------------------
// Building: reading the two layers
// ---------------------------------------------------------------------------

/// One `(graph, subject, predicate, literal)` row read out of the dataset.
///
/// The literal is held decomposed rather than as a [`TermValue`] so the type
/// system, not a runtime check, guarantees that every row really is a literal
/// row; [`SourceRow::literal`] rebuilds the term when the digest needs it.
///
/// The derived order is the field order, which is what makes the row list — and
/// so the source digest — independent of the order the dataset yielded rows in.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SourceRow<P: CorpusProfile = GraphLanguage> {
    /// The graph the row was read from, `None` for the default graph.
    graph: P::Graph,
    /// The subject; in the annotation layer, the reifier.
    subject: TermValue,
    /// The predicate, always one of the configured IRIs.
    predicate: TermValue,
    /// The literal's datatype IRI.
    datatype: String,
    /// The literal's lexical form.
    lexical_form: String,
    /// The literal's language tag, already lowercased by the IR.
    language: Option<String>,
    /// The literal's base direction.
    direction: Option<RdfTextDirection>,
}

/// The `(graph, subject, language)` key that identifies a document.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct DocumentKey<P: CorpusProfile = GraphLanguage> {
    /// The graph, `None` for the default graph.
    graph: P::Graph,
    /// The subject.
    subject: TermValue,
    /// The language tag, `None` for untagged literals.
    language: Option<String>,
}

/// A retained token occurrence in original analyzed position coordinates.
/// This is a derived index view; strings remain in the sole term dictionary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenOccurrence {
    /// Sorted term dictionary ordinal.
    pub term: u32,
    /// Original token position, including gaps removed by predicate projection.
    pub position: u32,
    /// Original configured predicate ordinal.
    pub predicate: u32,
    /// Original occurrence ordinal within the physical document.
    pub ordinal: usize,
}

/// A private term address. Reuse never crosses an index generation's map.
/// Stored coordinates retain stable addresses; public ordinals are projected
/// only for the requested document, without rewriting any retained buffer.
#[derive(Clone, Debug)]
struct StoredToken {
    term: Shared<String>,
    position: u32,
    predicate: u32,
    ordinal: usize,
}

impl StoredToken {
    fn projected<P: CorpusProfile>(&self, terms: &Table<Shared<TermEntry<P>>>) -> TokenOccurrence {
        TokenOccurrence {
            term: terms
                .binary_search_by(|entry| entry.term.as_str().cmp(self.term.as_str()))
                .expect("a retained token's spelling is live") as u32,
            position: self.position,
            predicate: self.predicate,
            ordinal: self.ordinal,
        }
    }
}

/// Immutable buffers share across index snapshots; empty streams allocate none.
#[derive(Clone, Debug)]
struct SharedBuffer<T>(Option<Shared<Vec<T>>>);

impl<T> Default for SharedBuffer<T> {
    fn default() -> Self {
        Self(None)
    }
}

type TokenStream = SharedBuffer<StoredToken>;

type FieldPosting<'a> = (u32, &'a [(u32, u64)]);
type BuiltTerms<P> = (Vec<Shared<TermEntry<P>>>, Vec<TokenStream>);

impl<T> SharedBuffer<T> {
    fn as_slice(&self) -> &[T] {
        self.0.as_ref().map_or(&[], |tokens| tokens.as_slice())
    }
}

fn share_buffer<T>(tokens: Vec<T>) -> Result<SharedBuffer<T>, TextError> {
    if tokens.is_empty() {
        return Ok(SharedBuffer::default());
    }
    share_index(tokens).map(|tokens| SharedBuffer(Some(tokens)))
}

pub(crate) fn share_index<T>(value: T) -> Result<Shared<T>, TextError> {
    Shared::try_new(value).map_err(|_| {
        TextError::Capacity(crate::query_workspace::CapacityFailure::Allocator {
            construct: "text index value",
        })
    })
}

/// A document with its analyzed token stream, before ids are assigned.
#[derive(Clone, Debug)]
struct AnalyzedDocument<P: CorpusProfile = GraphLanguage> {
    /// The document's identity.
    key: DocumentKey<P>,
    /// `(dictionary ordinal, position)` for each token, in stream order.
    tokens: Vec<(u32, u32, u32)>,
    /// Token counts by predicate ordinal, before field routing.
    predicate_lengths: Vec<(u32, u64)>,
    /// Once-computed, source-aligned projections and source row identities.
    analyses: Vec<([u8; 32], crate::analysis::Analysis)>,
}

fn document_key<P: CorpusProfile>(
    graph: &P::Graph,
    subject: &TermValue,
    language: Option<&str>,
) -> DocumentKey<P> {
    DocumentKey {
        graph: graph.clone(),
        subject: subject.clone(),
        language: language.map(str::to_owned),
    }
}

/// Read every configured predicate's literal rows out of both RDF 1.2 layers,
/// and report which configured predicates the walk found a statement for.
///
/// A configured predicate the dataset has not interned contributes **no rows**
/// rather than raising: the dataset holds no statement with that predicate, so
/// there is no text under it, and that is an answer rather than a fault. The
/// limiting case is the one that decides it — an empty dataset has interned no
/// term at all, so a refusal here would make an index over an empty dataset
/// impossible, and an index that exists before its documents land is an ordinary
/// operating state. The same holds for a [`GraphSelector::Named`] graph the
/// dataset has not interned: nothing is in a graph that is not there, so the walk
/// yields nothing.
///
/// The relaxation is not silent, which is what the returned [`SourceCoverage`] is
/// for. A predicate is recorded as represented when an in-scope **statement**
/// carries it — before the literal filter, so a predicate whose objects are all
/// IRIs is represented and carries no text, which is data rather than a mistake —
/// and the coverage separates "this dataset holds nothing yet" from "this dataset
/// holds things, and has none of these".
fn collect_rows<D: DatasetView, P: CorpusProfile>(
    dataset: &D,
    config: &TextIndexConfig<P>,
) -> Result<(Vec<SourceRow<P>>, SourceCoverage), TextError>
where
    D::ReadError: purrdf_hash::error::StructuredError,
{
    dataset
        .checked_read(|dataset| {
            let (rows, counts) = collect_selected_rows(dataset, config, None)?;
            Ok((rows, SourceCoverage::of_walk(dataset, config, counts)))
        })
        .map_err(TextError::source_read)?
}

/// Read replacement documents through the source's subject indexes in both
/// physical text-bearing layers. Predicate and graph selection use the same
/// production collector as a fresh build and a delta operand.
fn collect_subject_rows<D: DatasetView, P: CorpusProfile>(
    dataset: &D,
    config: &TextIndexConfig<P>,
    subjects: &BTreeSet<&TermValue>,
) -> Result<Vec<SourceRow<P>>, TextError>
where
    D::ReadError: purrdf_hash::error::StructuredError,
{
    dataset
        .checked_read(|dataset| {
            collect_selected_rows(dataset, config, Some(subjects)).map(|(rows, _)| rows)
        })
        .map_err(TextError::source_read)?
}

fn collect_selected_rows<D: DatasetView, P: CorpusProfile>(
    dataset: &D,
    config: &TextIndexConfig<P>,
    subjects: Option<&BTreeSet<&TermValue>>,
) -> Result<(Vec<SourceRow<P>>, Vec<u128>), TextError>
where
    D::ReadError: purrdf_hash::error::StructuredError,
{
    if P::GRAPH_FREE && dataset.supports_named_graphs() {
        return Err(TextError::config(
            "the graph-free text profile requires a graph-free source",
        ));
    }
    let Some(graph) = P::resolve(config.graph(), dataset).map_err(TextError::source_read)? else {
        return Ok((Vec::new(), vec![0; config.predicates().len()]));
    };
    let mut predicate_ids: Vec<(D::Id, &TermValue, usize)> =
        Vec::with_capacity(config.predicates().len());
    for (ordinal, predicate) in config.predicates().iter().enumerate() {
        if let Some(id) = dataset
            .term_id_by_value(predicate)
            .map_err(TextError::source_read)?
        {
            predicate_ids.push((id, predicate, ordinal));
        }
    }
    if predicate_ids.is_empty() {
        // No configured predicate is interned, so no statement in either layer
        // can carry one. Returning here rather than sweeping the annotation side
        // table to match every row against an empty id set.
        return Ok((Vec::new(), vec![0; config.predicates().len()]));
    }

    let subject_ids = subjects
        .map(|subjects| {
            subjects
                .iter()
                .map(|subject| {
                    dataset
                        .term_id_by_value(subject)
                        .map_err(TextError::source_read)
                })
                .filter_map(Result::transpose)
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;

    let mut rows = Vec::new();
    let mut represented = vec![0; config.predicates().len()];

    // Layer one: the asserted triple table.
    let asserted_subjects = subject_ids
        .iter()
        .flat_map(|ids| ids.iter().copied())
        .map(Some)
        .chain(std::iter::once(None).filter(|_| subject_ids.is_none()));
    for subject in asserted_subjects {
        for &(predicate_id, predicate, ordinal) in &predicate_ids {
            for quad in dataset.quads_for_pattern(subject, Some(predicate_id), None, graph) {
                represented[ordinal] += 1;
                push_row(dataset, &mut rows, quad.g, quad.s, predicate, quad.o)?;
            }
        }
    }

    // Layer two: the RDF 1.2 annotation side table, whose subject IS the
    // reifier. `quads_for_pattern` above cannot see these rows at all.
    let mut annotation = |quad: purrdf_core::QuadIds<D::Id>| {
        if !graph.matches(quad.g) {
            return Ok(());
        }
        let Some(&(_, predicate, ordinal)) = predicate_ids.iter().find(|&&(id, _, _)| id == quad.p)
        else {
            return Ok(());
        };
        represented[ordinal] += 1;
        push_row(dataset, &mut rows, quad.g, quad.s, predicate, quad.o)
    };
    if let Some(subjects) = subject_ids {
        for subject in subjects {
            for (p, o, g) in dataset.annotations_of_with_graph(subject) {
                annotation(purrdf_core::QuadIds {
                    s: subject,
                    p,
                    o,
                    g,
                })?;
            }
        }
    } else {
        for quad in dataset.annotation_quads() {
            annotation(quad)?;
        }
    }

    rows.sort();
    Ok((rows, represented))
}

impl SourceCoverage {
    /// The coverage of a walk that counted `represented[ordinal]` for each of
    /// `config`'s predicates, before filtering literal objects.
    ///
    /// The dataset is probed for a single statement only when something came up
    /// unrepresented, because that probe exists solely to tell the two
    /// unrepresented cases apart: where every configured predicate was found there
    /// is nothing to tell apart, and a configuration holds at least one predicate,
    /// so finding them all is itself proof the dataset holds statements.
    fn of_walk<D: DatasetView, P: CorpusProfile>(
        dataset: &D,
        config: &TextIndexConfig<P>,
        represented: Vec<u128>,
    ) -> Self {
        let dataset_holds_statements =
            represented.iter().all(|&count| count != 0) || holds_a_statement(dataset);
        Self::of_counts(dataset_holds_statements, config, represented)
    }

    /// Coverage from exact counts and an independently observed nonempty bit.
    /// The delta path can reuse that source observation without probing again.
    fn of_counts<P: CorpusProfile>(
        dataset_holds_statements: bool,
        config: &TextIndexConfig<P>,
        represented: Vec<u128>,
    ) -> Self {
        let unrepresented: Vec<TermValue> = config
            .predicates()
            .iter()
            .zip(&represented)
            .filter(|&(_, &count)| count == 0)
            .map(|(predicate, _)| predicate.clone())
            .collect();
        Self {
            dataset_holds_statements,
            unrepresented,
            represented,
        }
    }
}

/// Whether `dataset` holds a single statement, in any layer and any graph.
///
/// All three layers are asked because each is invisible to the others:
/// [`DatasetView::quads`] exposes the asserted triple table alone, and the RDF 1.2
/// reifier and annotation side tables are separate, capability-gated iterators. A
/// dataset whose only content is an annotation holds content, and answering "this
/// dataset is empty" for it would call a mistyped predicate an index waiting for
/// its documents.
///
/// Each iterator stops at its first row. A backend may still initialize traversal
/// metadata before yielding that row; delta accounting reuses its original
/// source observation instead of repeating this probe.
fn holds_a_statement<D: DatasetView>(dataset: &D) -> bool {
    dataset.quads().next().is_some()
        || dataset.reifier_quads().next().is_some()
        || dataset.annotation_quads().next().is_some()
}

/// Resolve one row's terms and append it, unless its object is not a literal.
fn push_row<D: DatasetView, P: CorpusProfile>(
    dataset: &D,
    rows: &mut Vec<SourceRow<P>>,
    graph: Option<D::Id>,
    subject: D::Id,
    predicate: &TermValue,
    object: D::Id,
) -> Result<(), TextError>
where
    D::ReadError: purrdf_hash::error::StructuredError,
{
    // A predicate may legitimately carry literals in one statement and an IRI in
    // the next; that is data, not a fault, so a non-literal object contributes
    // no text and raises nothing.
    let TermValue::Literal {
        lexical_form,
        datatype,
        language,
        direction,
    } = resolve_value(dataset, object)?
    else {
        return Ok(());
    };
    let graph = match graph {
        Some(id) => Some(resolve_value(dataset, id)?),
        None => None,
    };
    rows.push(SourceRow {
        graph: P::retain_graph(graph)?,
        subject: resolve_value(dataset, subject)?,
        predicate: predicate.clone(),
        datatype,
        lexical_form,
        language,
        direction,
    });
    Ok(())
}

/// Resolve a dataset-local id to its dataset-independent [`TermValue`] through
/// [`DatasetView::term_value`], which follows a literal's datatype and a triple term's
/// `(s, p, o)` to any depth.
///
/// # Errors
///
/// [`TextError`] naming the inconsistency when the view hands back an id that is not
/// its own — a literal whose datatype does not resolve to an IRI.
fn resolve_value<D: DatasetView>(dataset: &D, id: D::Id) -> Result<TermValue, TextError>
where
    D::ReadError: purrdf_hash::error::StructuredError,
{
    dataset.term_value(id).map_err(|error| match error {
        foreign @ purrdf_core::TermLookupError::ForeignId => TextError::data(foreign.to_string()),
        purrdf_core::TermLookupError::Read(error) => TextError::source_read(error),
    })
}

// ---------------------------------------------------------------------------
// Building: analysis
// ---------------------------------------------------------------------------

/// Group `rows` into documents, analyze each once, retain searchable projections, and sort
/// what is left into id order.
///
/// Returns the documents and the term dictionary in **intern** order; the
/// dictionary is re-sorted and the ordinals remapped in [`build_terms`].
fn analyze_rows<P: CorpusProfile>(
    rows: &[SourceRow<P>],
    config: &TextIndexConfig<P>,
) -> Result<(Vec<AnalyzedDocument<P>>, Vec<String>), TextError> {
    let mut groups: FastMap<DocumentKey<P>, Vec<usize>> = FastMap::default();
    for (index, row) in rows.iter().enumerate() {
        let key = document_key(&row.graph, &row.subject, row.language.as_deref());
        groups.entry(key).or_default().push(index);
    }

    let analyzer = config.analyzer();
    let mut ordinals: FastMap<String, u32> = FastMap::default();
    let mut dictionary: Vec<String> = Vec::new();
    let mut documents: Vec<AnalyzedDocument<P>> = Vec::with_capacity(groups.len());

    for (key, mut indices) in groups {
        // The token stream is the document's literals concatenated in sorted
        // `(predicate, lexical form, direction)` order, so it does not depend on
        // the order the dataset yielded them in.
        indices.sort_by(|&left, &right| {
            let (left, right) = (&rows[left], &rows[right]);
            left.predicate
                .cmp(&right.predicate)
                .then_with(|| left.lexical_form.cmp(&right.lexical_form))
                .then_with(|| left.direction.cmp(&right.direction))
        });

        let mut tokens: Vec<(u32, u32, u32)> = Vec::new();
        let mut predicate_lengths: Vec<(u32, u64)> = Vec::new();
        let mut analyses = Vec::new();
        // Ordinals belong only to this index's declared projection. Auxiliary
        // terms never advance lexical phrase positions or document lengths.
        let mut position: u32 = 0;
        let mut has_auxiliary = false;
        for index in indices {
            let row = &rows[index];
            let predicate = config
                .predicates
                .binary_search(&row.predicate)
                .expect("walk selected configured predicates") as u32;
            let mut analysis = analyzer.projections(&row.lexical_form)?;
            has_auxiliary |= !analysis.surface.is_empty() || !analysis.spans.is_empty();
            let character;
            let projections = match config.projection {
                IndexProjection::Lexical => &analysis.lexical,
                IndexProjection::Han => {
                    character = crate::character::index_projections(&analysis);
                    &character
                }
            };
            for projection in projections {
                let ordinal = match ordinals.get(&projection.text) {
                    Some(&ordinal) => ordinal,
                    None => {
                        let ordinal = u32::try_from(dictionary.len()).map_err(|_| {
                            TextError::data("term dictionary exceeds the u32 ordinal space")
                        })?;
                        dictionary.push(projection.text.clone());
                        ordinals.insert(projection.text.clone(), ordinal);
                        ordinal
                    }
                };
                tokens.push((ordinal, position, predicate));
                match predicate_lengths.last_mut() {
                    Some((prior, length)) if *prior == predicate => {
                        *length += 1;
                    }
                    _ => {
                        predicate_lengths.push((predicate, 1));
                    }
                }
                position = position.checked_add(1).ok_or_else(|| {
                    TextError::data("document exceeds the u32 token position space")
                })?;
            }
            // The native token walk has consumed the lexical view. Han replay
            // reads normalized text, while SurfaceIndex consumes surface/spans
            // and their original source alignment. Release this actual scratch
            // buffer, including capacity, before retaining the remaining views.
            drop(core::mem::take(&mut analysis.lexical));
            if config.surface || config.projection == IndexProjection::Han {
                analyses.push((digest_rows(std::iter::once(row)), analysis));
            }
        }
        // Retain auxiliary-only documents without admitting them into the
        // independent lexical or Han scoring population.
        if tokens.is_empty() && !has_auxiliary {
            continue;
        }

        documents.push(AnalyzedDocument {
            key,
            tokens,
            predicate_lengths,
            analyses,
        });
    }

    // Ids are assigned by content order, never by intern order.
    documents.sort_by(|left, right| left.key.cmp(&right.key));
    Ok((documents, dictionary))
}

/// The physical stream order used by phrase occurrence traversal.
fn build_occurrence_order<P: CorpusProfile>(
    table: &Table<Shared<Document<P>>>,
) -> Result<Table<Shared<Document<P>>>, TextError> {
    let mut order: Vec<_> = table.iter().cloned().collect();
    order.sort_by(|a, b| compare_occurrence_documents(a, b));
    Table::from_streams(order)
}

/// `total / count`, exactly, in fixed point.
fn check_document_count(count: usize) -> Result<(), TextError> {
    u32::try_from(count).map(|_| ()).map_err(|_| TextError::data(format!(
        "the dataset yields {count} documents, which exceeds the u32 document-id space this index addresses"
    )))
}

fn exact_average(total: u64, count: u64) -> Result<Fixed, TextError> {
    Fixed::from_raw(i128::from(total) * crate::fixed::SCALE)
        .checked_div(Fixed::from_raw(i128::from(count) * crate::fixed::SCALE))
}

/// Sort the dictionary, remap the token ordinals onto it, and build the postings.
fn build_terms<P: CorpusProfile>(
    documents: &[AnalyzedDocument<P>],
    dictionary: &[String],
    table: &[Shared<Document<P>>],
) -> Result<BuiltTerms<P>, TextError> {
    let mut order: Vec<u32> = (0..dictionary.len() as u32).collect();
    order.sort_by(|&left, &right| dictionary[left as usize].cmp(&dictionary[right as usize]));
    let mut rank = vec![0_u32; dictionary.len()];
    for (sorted, &interned) in order.iter().enumerate() {
        rank[interned as usize] = sorted as u32;
    }

    let words: Vec<_> = order
        .iter()
        .map(|&interned| share_index(dictionary[interned as usize].clone()))
        .collect::<Result<_, _>>()?;

    let token_streams = documents
        .iter()
        .map(|document| {
            share_buffer(
                document
                    .tokens
                    .iter()
                    .enumerate()
                    .map(|(ordinal, &(term, position, predicate))| StoredToken {
                        term: words[rank[term as usize] as usize].clone(),
                        position,
                        predicate,
                        ordinal,
                    })
                    .collect(),
            )
        })
        .collect::<Result<_, _>>()?;
    let mut postings: Vec<Vec<Shared<Posting<P>>>> = vec![Vec::new(); dictionary.len()];
    for (id, document) in documents.iter().enumerate() {
        let mut tokens: Vec<(u32, u32, u32)> = document
            .tokens
            .iter()
            .map(|&(ordinal, position, predicate)| (rank[ordinal as usize], position, predicate))
            .collect();
        tokens.sort_unstable();

        let mut at = 0;
        while at < tokens.len() {
            let term = tokens[at].0;
            let mut positions = Vec::new();
            let mut predicate_frequencies: Vec<(u32, u64)> = Vec::new();
            while at < tokens.len() && tokens[at].0 == term {
                positions.push(tokens[at].1);
                let predicate = tokens[at].2;
                match predicate_frequencies.last_mut() {
                    Some((prior, frequency)) if *prior == predicate => *frequency += 1,
                    _ => predicate_frequencies.push((predicate, 1)),
                }
                at += 1;
            }
            postings[term as usize].push(share_index(Posting {
                document: table[id].clone(),
                positions,
                predicate_frequencies,
            })?);
        }
    }

    let mut entries = Vec::with_capacity(words.len());
    for (term, mut list) in words.into_iter().zip(postings) {
        // Documents were pushed in ascending id order, so a *stable* sort by
        // partition leaves each partition's run ascending by document id too.
        list.sort_by(|a, b| compare_posting_documents(&a.document, &b.document));
        entries.push(share_index(TermEntry {
            term,
            postings: Table::from_streams(list)?,
        })?);
    }
    Ok((entries, token_streams))
}

// ---------------------------------------------------------------------------
// Digests
// ---------------------------------------------------------------------------

/// The source digest and the coverage `dataset` yields under `config` — the two
/// values an index built from that pairing holds
/// ([`TextIndex::source_fingerprint`] and [`TextIndex::source_coverage`]).
///
/// Exposed to the crate so [`crate::verify_binding`] can ask both of its
/// questions — "is this the data under that index?" and "did this configuration
/// find all of it?" — without building a second index: both come off the walk,
/// which needs no analysis.
///
/// # Errors
///
/// Whatever the walk raises — [`TextError::Data`] for a term the encoder cannot
/// represent. A configured predicate, or a named graph, the dataset does not
/// carry contributes no rows rather than raising, so an empty dataset digests to
/// the digest of an empty row set: the value an index built over it holds. The
/// coverage beside it is what distinguishes that dataset from one holding
/// statements under everything but a configured predicate.
pub(crate) fn source_digest<D: DatasetView, P: CorpusProfile>(
    dataset: &D,
    config: &TextIndexConfig<P>,
) -> Result<([u8; FINGERPRINT_BYTES], SourceCoverage), TextError>
where
    D::ReadError: purrdf_hash::error::StructuredError,
{
    let (rows, coverage) = collect_rows(dataset, config)?;
    Ok((digest_rows(rows.iter()), coverage))
}

/// Digest the source rows: what the index actually walked, and nothing else.
fn digest_rows<'a, P: CorpusProfile + 'a>(
    rows: impl ExactSizeIterator<Item = &'a SourceRow<P>>,
) -> [u8; FINGERPRINT_BYTES] {
    SourceDigest::from_rows(rows).fingerprint::<P>()
}

/// A streaming digest over the crate's one term encoding.
///
/// Every field is self-delimiting — fixed-width for a number or a tag, length
/// prefixed for a string, and [`encode_term`]'s own prefix-free form for a term
/// — so the whole stream is injective for the same reason
/// [`crate::term_bytes`]'s encoding is, and no payload byte can be mistaken for
/// structure. The scratch buffer is reused across every field, so digesting a
/// large index allocates a bounded amount rather than a buffer per term.
struct Digest {
    /// The hash state.
    hasher: purrdf_hash::blake3::Hasher,
    /// The reused encoding buffer.
    scratch: Vec<u8>,
}

#[cfg(test)]
std::thread_local! {
    /// Actual bytes fed to this identity encoder on the current test thread.
    static DIGEST_BYTES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

impl Digest {
    /// A digest opened under `domain`, so two digests of different things over
    /// the same bytes cannot coincide. The domain is absorbed as the first
    /// length-prefixed string.
    fn new(domain: Domain) -> Self {
        let mut digest = Self::bare();
        digest.text(domain.as_str());
        digest
    }

    /// A digest with no domain of its own, for an identity whose first absorbed
    /// field is the published identifier it is named by.
    fn bare() -> Self {
        Self {
            hasher: purrdf_hash::blake3::Hasher::new(),
            scratch: Vec::new(),
        }
    }

    /// Absorb a one-byte tag.
    fn tag(&mut self, tag: u8) {
        self.absorb(&[tag]);
    }

    /// Absorb a `u64`, little-endian.
    fn number(&mut self, value: u64) {
        self.absorb(&value.to_le_bytes());
    }

    fn absorb(&mut self, bytes: &[u8]) {
        #[cfg(test)]
        DIGEST_BYTES.set(DIGEST_BYTES.get() + bytes.len());
        self.hasher.update(bytes);
    }

    /// Absorb a collection's length.
    fn count(&mut self, value: usize) {
        self.number(value as u64);
    }

    /// Absorb a length-prefixed string.
    fn text(&mut self, value: &str) {
        self.scratch.clear();
        frame_le(&mut self.scratch, value.as_bytes());
        #[cfg(test)]
        DIGEST_BYTES.set(DIGEST_BYTES.get() + self.scratch.len());
        self.hasher.update(&self.scratch);
    }

    /// Absorb a term in the crate's injective encoding.
    fn term(&mut self, value: &TermValue) {
        self.scratch.clear();
        encode_term(value, &mut self.scratch);
        #[cfg(test)]
        DIGEST_BYTES.set(DIGEST_BYTES.get() + self.scratch.len());
        self.hasher.update(&self.scratch);
    }

    /// The finished digest.
    fn finish(self) -> [u8; FINGERPRINT_BYTES] {
        *self.hasher.finalize().as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use purrdf_core::TermValue;

    use super::{
        Digest, GraphSelector, PartitionKey, SOURCE_DIGEST_DOMAIN, TextIndexConfig, exact_average,
    };
    use crate::error::TextError;
    use crate::fixed::Fixed;

    /// The predicate list is sorted, so two callers who name the same
    /// predicates in different orders build the same index.
    #[test]
    fn the_predicate_list_is_sorted() {
        let config = TextIndexConfig::new(
            vec![
                TermValue::iri("https://example.org/z"),
                TermValue::iri("https://example.org/a"),
            ],
            GraphSelector::Any,
            crate::Analyzer::empty_lexicon(),
        )
        .expect("two distinct IRIs are a valid configuration");
        assert_eq!(
            config.predicates(),
            &[
                TermValue::iri("https://example.org/a"),
                TermValue::iri("https://example.org/z"),
            ]
        );
    }

    /// A `Named` selector that is not an IRI is refused, for the same reason a
    /// non-IRI predicate is: nothing else can name a graph.
    #[test]
    fn a_non_iri_graph_selector_is_refused() {
        let error = TextIndexConfig::new(
            vec![TermValue::iri("https://example.org/p")],
            GraphSelector::Named(TermValue::simple_literal("g")),
            crate::Analyzer::empty_lexicon(),
        )
        .expect_err("a literal cannot name a graph");
        assert!(matches!(error, TextError::Config(_)), "got {error:?}");
    }

    /// The averaging helper is exact rather than truncated to an integer.
    #[test]
    fn the_average_is_exact() {
        assert_eq!(
            exact_average(7, 2).expect("7/2 is representable"),
            Fixed::from_integer(7)
                .and_then(
                    |total| total.checked_div(Fixed::from_integer(2).expect("2 is representable"))
                )
                .expect("7/2 is representable")
        );
    }

    /// The digest's fields are self-delimiting: a length-prefixed string cannot
    /// be confused with a differently split pair of them.
    #[test]
    fn digest_fields_are_self_delimiting() {
        let split = |left: &str, right: &str| {
            let mut digest = Digest::new(SOURCE_DIGEST_DOMAIN);
            digest.text(left);
            digest.text(right);
            digest.finish()
        };
        assert_ne!(split("ab", "c"), split("a", "bc"));
    }

    /// The partition key's accessors report the distinction its documentation
    /// claims: `None` is the default graph and untagged text, not a sentinel.
    #[test]
    fn the_partition_key_reports_its_absences() {
        let key = PartitionKey::new(None, None);
        assert!(key.graph().is_none());
        assert!(key.language().is_none());

        let named = PartitionKey::new(
            Some(TermValue::iri("https://example.org/g")),
            Some("en".to_owned()),
        );
        assert_eq!(
            named.graph(),
            Some(&TermValue::iri("https://example.org/g"))
        );
        assert_eq!(named.language(), Some("en"));
    }
}
