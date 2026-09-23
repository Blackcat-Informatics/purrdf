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

use std::cmp::Ordering;

use purrdf_core::{DatasetView, FastMap, GraphMatch, RdfTextDirection, TermRef, TermValue};

use crate::analysis::{Analyzer, UnicodeVersions, unicode_versions};
use crate::error::TextError;
use crate::fixed::Fixed;
use crate::ranking::{FIELD_LENGTH_MAX, FieldInput, MAX_FIELDS, PreparedCorpus, RankingProfile};
use crate::term_bytes::{FINGERPRINT_BYTES, MAX_TRIPLE_DEPTH, encode_term, push_str};

/// Domain-separation prefix for [`TextIndex::fingerprint`].
const INDEX_DIGEST_DOMAIN: &str = "purrdf-text/index/v2";
/// Domain-separation prefix for [`TextIndex::source_fingerprint`].
const SOURCE_DIGEST_DOMAIN: &str = "purrdf-text/source/v1";

/// Digest tag for [`GraphSelector::Any`].
const SELECTOR_ANY: u8 = 0x01;
/// Digest tag for [`GraphSelector::Default`].
const SELECTOR_DEFAULT: u8 = 0x02;
/// Digest tag for [`GraphSelector::Named`].
const SELECTOR_NAMED: u8 = 0x03;

/// Digest presence byte for an absent optional field.
const ABSENT: u8 = 0x00;
/// Digest presence byte for a present optional field.
const PRESENT: u8 = 0x01;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Which graph's literals an index is built over.
///
/// This is deliberately in **value** space rather than
/// [`GraphMatch`](purrdf_core::GraphMatch) space. `GraphMatch::Named` holds a
/// dataset-local term id, which means something only inside the one dataset that
/// minted it; a configuration is a statement the caller writes down once and may
/// apply to several datasets, so it must name a graph by its IRI.
/// [`TextIndex::from_dataset`] resolves the selector to a `GraphMatch` against
/// the dataset in hand.
///
/// Deliberately exhaustive, like `GraphMatch`: a quad's graph is the default
/// graph or exactly one named graph, so the three cases are closed.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GraphSelector {
    /// Index literals from every graph, default and named alike. Each graph
    /// still yields its own documents and its own partitions.
    Any,
    /// Index literals from the default graph only.
    Default,
    /// Index literals from the one named graph this IRI identifies.
    Named(TermValue),
}

/// The caller's complete, dataset-independent statement of what to index.
///
/// There is no [`Default`] implementation and there never will be one: a default
/// would have to name predicate IRIs, and PurRDF mints none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextIndexConfig {
    /// The indexed predicates, sorted and known to be distinct IRIs.
    predicates: Vec<TermValue>,
    /// Which graph the index is drawn from.
    graph: GraphSelector,
}

impl TextIndexConfig {
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
    pub fn new(predicates: Vec<TermValue>, graph: GraphSelector) -> Result<Self, TextError> {
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
        if let GraphSelector::Named(name) = &graph
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

        Ok(Self { predicates, graph })
    }

    /// The indexed predicates, in sorted order.
    pub fn predicates(&self) -> &[TermValue] {
        &self.predicates
    }

    /// The graph this configuration draws from.
    pub const fn graph(&self) -> &GraphSelector {
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
pub struct PartitionKey {
    /// The graph name, or `None` for the default graph.
    graph: Option<TermValue>,
    /// The literals' language tag, or `None` for untagged literals.
    language: Option<String>,
}

impl PartitionKey {
    /// The partition for `graph` (`None` = the default graph) and `language`
    /// (`None` = untagged literals).
    pub const fn new(graph: Option<TermValue>, language: Option<String>) -> Self {
        Self { graph, language }
    }

    /// The graph name, or `None` for the default graph.
    pub const fn graph(&self) -> Option<&TermValue> {
        self.graph.as_ref()
    }

    /// The language tag, or `None` for untagged literals.
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }
}

/// One partition's corpus statistics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartitionStats {
    /// How many documents this partition retains.
    document_count: u64,
    /// The total number of analyzed tokens across those documents.
    total_tokens: u64,
    /// `total_tokens / document_count`, exactly, in fixed point.
    average_document_length: Fixed,
}

impl PartitionStats {
    /// How many documents this partition retains.
    pub const fn document_count(&self) -> u64 {
        self.document_count
    }

    /// The total number of analyzed tokens across this partition's documents.
    pub const fn total_tokens(&self) -> u64 {
        self.total_tokens
    }

    /// The average document length, in tokens.
    ///
    /// Guaranteed strictly positive for every partition an index retains: a
    /// document that analyzes to zero tokens is not retained at all (see
    /// [`TextIndex`]), so neither the numerator nor the denominator of this
    /// quotient can be zero. BM25 divides by this value, and the guarantee is
    /// what makes that division safe without a special case that would have to
    /// invent a score.
    ///
    /// The guarantee is over *retained* partitions, and that is the whole of it:
    /// an index holding no documents holds no partitions either, so there is no
    /// `PartitionStats` to read a zero out of. An empty corpus cannot reach this
    /// divisor rather than reaching it with a zero.
    pub const fn average_document_length(&self) -> Fixed {
        self.average_document_length
    }
}

/// One indexed document: the text of one `(graph, subject, language)` triple
/// key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    /// The graph the document's literals were read from, `None` for the default
    /// graph.
    graph: Option<TermValue>,
    /// The subject the literals hang off — an IRI, a blank node, or (for a
    /// backend that admits one) a triple term. In the annotation layer this is
    /// the reifier.
    subject: TermValue,
    /// The language tag shared by the document's literals, `None` when they are
    /// untagged.
    language: Option<String>,
    /// How many analyzed tokens the document holds. Never zero.
    length: u64,
    /// Analyzed token counts by selected predicate ordinal.
    predicate_lengths: Vec<(u32, u64)>,
    /// Which of the index's partitions this document belongs to.
    partition: u32,
}

impl Document {
    /// The graph, or `None` for the default graph.
    pub const fn graph(&self) -> Option<&TermValue> {
        self.graph.as_ref()
    }

    /// The subject — the reifier, for text read from the annotation layer.
    pub const fn subject(&self) -> &TermValue {
        &self.subject
    }

    /// The language tag, or `None` for untagged literals.
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// How many analyzed tokens this document holds. Never zero.
    pub const fn length(&self) -> u64 {
        self.length
    }
}

/// One term's occurrences in one document.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Posting {
    /// The document id.
    document: u32,
    /// The token positions the term occupies, ascending.
    positions: Vec<u32>,
    /// Frequencies by predicate ordinal, retained independently of ranking.
    predicate_frequencies: Vec<(u32, u64)>,
}

/// The half-open run of one term's postings that belongs to one partition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PartitionSpan {
    /// The partition index.
    partition: u32,
    /// The first posting in the run.
    start: u32,
    /// One past the last posting in the run.
    end: u32,
}

/// One dictionary entry: a term, its postings, and where each partition's run
/// of them begins.
#[derive(Clone, Debug, PartialEq, Eq)]
struct TermEntry {
    /// The analyzed term text.
    term: String,
    /// Postings ordered by `(partition, document)`.
    postings: Vec<Posting>,
    /// The partition runs of `postings`, ascending by partition.
    spans: Vec<PartitionSpan>,
}

// ---------------------------------------------------------------------------
// The index
// ---------------------------------------------------------------------------

/// A built, immutable inverted index over one dataset's literals.
///
/// # The zero-token invariant
///
/// A `(graph, subject, language)` key whose literals analyze to **no** tokens is
/// not a document and is not retained. A subject whose only literal is `"---"`
/// contributes nothing a query could ever match, so keeping it would add a row
/// to `N` and a zero to the length sum — dragging every partition's average
/// document length toward zero and, in a partition made entirely of such
/// subjects, driving it *to* zero and putting a division by zero in the BM25
/// denominator. Excluding them is what makes
/// [`PartitionStats::average_document_length`] non-zero for every retained
/// partition, which is a property later stages are entitled to rely on.
///
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
pub struct TextIndex {
    /// The configuration this index was built under.
    config: TextIndexConfig,
    /// Complete immutable fielded ranking law.
    ranking: RankingProfile,
    /// Selected predicate ordinals mapped to ranking fields.
    predicate_fields: Vec<usize>,
    /// Field lengths by document; derived without reanalysis.
    field_lengths: Vec<Vec<u64>>,
    /// Exact field token totals by partition.
    field_totals: Vec<Vec<u128>>,
    /// The Unicode table versions the analyzer resolved against at build time.
    unicode: UnicodeVersions,
    /// Documents in id order — that is, sorted by `(graph, subject, language)`.
    documents: Vec<Document>,
    /// Document ids sorted by `(subject, id)` — the side index a subject lookup
    /// binary-searches.
    ///
    /// Derived from [`Self::documents`] and therefore **not** part of either
    /// fingerprint: it holds no information the document table does not, and
    /// digesting it would only make the digest depend on an implementation
    /// detail. It exists because the document table is sorted with the *graph*
    /// first, so the documents sharing one subject are not contiguous in it.
    subject_order: Vec<u32>,
    /// Partitions sorted by [`PartitionKey`], with their statistics.
    partitions: Vec<(PartitionKey, PartitionStats)>,
    /// The term dictionary, sorted by term text.
    terms: Vec<TermEntry>,
    /// The digest of everything that can change an answer.
    fingerprint: [u8; FINGERPRINT_BYTES],
    /// The digest of the source rows walked.
    source_fingerprint: [u8; FINGERPRINT_BYTES],
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

impl TextIndex {
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
        config: &TextIndexConfig,
    ) -> Result<Self, TextError> {
        let (rows, coverage) = collect_rows(dataset, config)?;
        let source_fingerprint = digest_rows(&rows)?;
        let (documents, dictionary) = analyze_rows(&rows, config)?;
        Self::assemble(
            config.clone(),
            &documents,
            &dictionary,
            source_fingerprint,
            coverage,
            RankingProfile::single_field(),
        )
    }

    /// Build directly under a fielded ranking law, including documents whose
    /// combined length exceeds the per-field bound.
    ///
    /// # Errors
    /// Propagates indexing errors and refuses incomplete routing or field bounds.
    pub fn from_dataset_with_ranking<D: DatasetView>(
        dataset: &D,
        config: &TextIndexConfig,
        profile: RankingProfile,
    ) -> Result<Self, TextError> {
        for predicate in config.predicates() {
            profile.field_for(predicate)?;
        }
        let (rows, coverage) = collect_rows(dataset, config)?;
        let source_fingerprint = digest_rows(&rows)?;
        let (documents, dictionary) = analyze_rows(&rows, config)?;
        Self::assemble(
            config.clone(),
            &documents,
            &dictionary,
            source_fingerprint,
            coverage,
            profile,
        )
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
        self.fingerprint = self.compute_fingerprint()?;
        Ok(self)
    }

    /// The named, immutable ranking law used for every score.
    pub const fn ranking_profile(&self) -> &RankingProfile {
        &self.ranking
    }

    /// Tokenization identity, independent of ranking and predicate routing.
    /// Includes the complete Unicode table versions.
    pub fn analyzer_fingerprint(&self) -> [u8; FINGERPRINT_BYTES] {
        let mut digest = Digest::new(crate::ANALYZER_PROFILE_ID);
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
        digest.finish()
    }

    /// Exact field token totals for a partition, in ranking field order.
    pub fn field_totals(&self, partition: &PartitionKey) -> Option<&[u128]> {
        self.partition_index(partition)
            .map(|at| self.field_totals[at as usize].as_slice())
    }

    /// A document's field lengths, in ranking field order.
    pub fn field_lengths(&self, document: u32) -> Option<&[u64]> {
        self.field_lengths.get(document as usize).map(Vec::as_slice)
    }

    /// Nonzero `(predicate ordinal, token count)` facts, in configuration order. These
    /// facts do not change when the ranking profile changes.
    pub fn predicate_lengths(&self, document: u32) -> Option<&[(u32, u64)]> {
        self.documents
            .get(document as usize)
            .map(|doc| doc.predicate_lengths.as_slice())
    }

    /// Fill a stack-allocated field input buffer for one document and term.
    pub(crate) fn field_inputs(
        &self,
        document: u32,
        term: &str,
    ) -> Result<[FieldInput; MAX_FIELDS], TextError> {
        let doc = self
            .document(document)
            .ok_or_else(|| TextError::data("field input names an absent document"))?;
        let frequencies = self
            .term_entry(term)
            .and_then(|entry| {
                let postings = span_slice(entry, doc.partition);
                postings
                    .binary_search_by_key(&document, |posting| posting.document)
                    .ok()
                    .map(|at| postings[at].predicate_frequencies.as_slice())
            })
            .unwrap_or(&[]);
        self.field_inputs_from_counts(document, frequencies)
    }

    /// Assemble field inputs from a posting already located by the query walk.
    /// This avoids a dictionary lookup per candidate and query term.
    pub(crate) fn field_inputs_from_counts(
        &self,
        document: u32,
        frequencies: &[(u32, u64)],
    ) -> Result<[FieldInput; MAX_FIELDS], TextError> {
        let lengths = self
            .field_lengths
            .get(document as usize)
            .ok_or_else(|| TextError::data("field input names an absent document"))?;
        let mut fields = [FieldInput::default(); MAX_FIELDS];
        for (input, &length) in fields.iter_mut().zip(lengths) {
            input.length = length;
        }
        for &(predicate, frequency) in frequencies {
            fields[self.predicate_fields[predicate as usize]].term_frequency += frequency;
        }
        Ok(fields)
    }

    /// Retained predicate frequencies along a term's existing posting walk.
    pub(crate) fn field_postings<'a>(
        &'a self,
        partition: &PartitionKey,
        term: &str,
    ) -> impl Iterator<Item = (u32, &'a [(u32, u64)])> + use<'a> {
        self.partition_postings(partition, term)
            .iter()
            .map(|posting| (posting.document, posting.predicate_frequencies.as_slice()))
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
        self.field_lengths = Vec::with_capacity(self.documents.len());
        self.field_totals = vec![vec![0; self.ranking.fields().len()]; self.partitions.len()];
        for document in &self.documents {
            let mut lengths = vec![0_u64; self.ranking.fields().len()];
            for &(predicate, length) in &document.predicate_lengths {
                let field = self.predicate_fields[predicate as usize];
                lengths[field] = lengths[field]
                    .checked_add(length)
                    .ok_or_else(|| TextError::overflow("merged field length exceeds u64"))?;
                if lengths[field] > FIELD_LENGTH_MAX {
                    return Err(TextError::data("merged field length exceeds 2^24"));
                }
            }
            for (total, &length) in self.field_totals[document.partition as usize]
                .iter_mut()
                .zip(&lengths)
            {
                *total += u128::from(length);
            }
            self.field_lengths.push(lengths);
        }
        for ((_, stats), totals) in self.partitions.iter().zip(&self.field_totals) {
            PreparedCorpus::new(&self.ranking, stats.document_count, totals)?;
        }
        Ok(())
    }

    /// The configuration this index was built under.
    pub const fn config(&self) -> &TextIndexConfig {
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
            .map(|(_, stats)| stats.document_count)
            .max()
            .unwrap_or(0)
    }

    /// Every partition and its statistics, ascending by [`PartitionKey`].
    pub fn partitions(&self) -> impl Iterator<Item = (&PartitionKey, &PartitionStats)> {
        self.partitions.iter().map(|(key, stats)| (key, stats))
    }

    /// One partition's statistics, or `None` if the index has no such partition.
    pub fn partition_stats(&self, partition: &PartitionKey) -> Option<&PartitionStats> {
        self.partition_index(partition)
            .and_then(|index| self.partitions.get(index as usize))
            .map(|(_, stats)| stats)
    }

    /// Every document, in id order.
    pub fn documents(&self) -> impl Iterator<Item = &Document> {
        self.documents.iter()
    }

    /// The document `id` names, or `None` if there is no such document.
    pub fn document(&self, id: u32) -> Option<&Document> {
        self.documents.get(id as usize)
    }

    /// Every document id whose subject is `subject`, ascending.
    ///
    /// A subject is not a document: `(graph, subject, language)` is, so one
    /// subject carrying `"the cat"@en` and `"le chat"@fr` in two named graphs is
    /// four documents. This is the lookup that turns the one into the other, in
    /// `O(log n)` rather than by scanning the document table.
    ///
    /// Empty for a subject the index holds no text for, which is the true answer
    /// rather than a failure: a query may legitimately name a subject that has
    /// no indexed literals.
    pub fn documents_with_subject(&self, subject: &TermValue) -> &[u32] {
        let compare = |id: &u32| self.documents[*id as usize].subject.cmp(subject);
        let start = self
            .subject_order
            .partition_point(|id| compare(id) == Ordering::Less);
        let end = self
            .subject_order
            .partition_point(|id| compare(id) != Ordering::Greater);
        self.subject_order
            .get(start..end)
            .expect("partition_point yields in-range, ordered offsets")
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
    pub fn partitions_holding_subject(&self, subject: &TermValue) -> Vec<PartitionKey> {
        let mut keys: Vec<PartitionKey> = self
            .documents_with_subject(subject)
            .iter()
            .filter_map(|&id| self.partition_key_of(id).cloned())
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }

    /// The partition the document `id` names belongs to.
    pub fn partition_key_of(&self, id: u32) -> Option<&PartitionKey> {
        let document = self.documents.get(id as usize)?;
        self.partitions
            .get(document.partition as usize)
            .map(|(key, _)| key)
    }

    /// How many analyzed tokens the document `id` names holds, or `None` if
    /// there is no such document. Never `Some(0)`.
    pub fn document_length(&self, id: u32) -> Option<u64> {
        self.documents.get(id as usize).map(Document::length)
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
    pub fn document_frequency(&self, partition: &PartitionKey, term: &str) -> u64 {
        self.partition_postings(partition, term).len() as u64
    }

    /// The postings of `term` within `partition`, ascending by document id.
    ///
    /// Each item is a document id and that document's ascending token positions
    /// for the term — everything a phrase or proximity match needs, and the
    /// occurrence rows a property function emits.
    pub fn postings<'a>(
        &'a self,
        partition: &PartitionKey,
        term: &str,
    ) -> impl Iterator<Item = (u32, &'a [u32])> + use<'a> {
        self.partition_postings(partition, term)
            .iter()
            .map(|posting| (posting.document, posting.positions.as_slice()))
    }

    /// How many times `term` occurs in the document `id` names.
    ///
    /// Zero when the term does not occur there, and zero for an id that names no
    /// document — a caller that needs to tell those apart asks
    /// [`Self::document`] first.
    pub fn term_frequency(&self, id: u32, term: &str) -> u64 {
        let Some(document) = self.documents.get(id as usize) else {
            return 0;
        };
        let Some(entry) = self.term_entry(term) else {
            return 0;
        };
        let postings = span_slice(entry, document.partition);
        postings
            .binary_search_by_key(&id, |posting| posting.document)
            .ok()
            .and_then(|at| postings.get(at))
            .map_or(0, |posting| posting.positions.len() as u64)
    }

    /// The digest of everything about this index that can change an answer.
    ///
    /// It covers the configuration (the sorted predicates and the graph
    /// selector), all four Unicode table versions the analyzer depends on, the
    /// document table, the term dictionary, every posting with its positions,
    /// and every partition's statistics. Two independently built indexes over
    /// the same content agree on it, and any change that would move a ranked
    /// answer moves it.
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
    fn partition_index(&self, partition: &PartitionKey) -> Option<u32> {
        self.partitions
            .binary_search_by(|(key, _)| key.cmp(partition))
            .ok()
            .map(|at| at as u32)
    }

    /// The dictionary entry for `term`, if it holds one.
    fn term_entry(&self, term: &str) -> Option<&TermEntry> {
        self.terms
            .binary_search_by(|entry| entry.term.as_str().cmp(term))
            .ok()
            .and_then(|at| self.terms.get(at))
    }

    /// `term`'s postings restricted to `partition`; empty when either is absent.
    fn partition_postings(&self, partition: &PartitionKey, term: &str) -> &[Posting] {
        let Some(partition) = self.partition_index(partition) else {
            return &[];
        };
        let Some(entry) = self.term_entry(term) else {
            return &[];
        };
        span_slice(entry, partition)
    }

    /// Turn sorted documents and an intern-ordered dictionary into the finished
    /// index: partitions, statistics, postings and the fingerprint.
    fn assemble(
        config: TextIndexConfig,
        documents: &[AnalyzedDocument],
        dictionary: &[String],
        source_fingerprint: [u8; FINGERPRINT_BYTES],
        coverage: SourceCoverage,
        ranking: RankingProfile,
    ) -> Result<Self, TextError> {
        if u32::try_from(documents.len()).is_err() {
            return Err(TextError::data(format!(
                "the dataset yields {} documents, which exceeds the u32 document-id space this \
                 index addresses",
                documents.len()
            )));
        }

        let (partitions, partition_of) = build_partitions(documents)?;
        let table: Vec<Document> = documents
            .iter()
            .map(|document| Document {
                graph: document.key.graph.clone(),
                subject: document.key.subject.clone(),
                language: document.key.language.clone(),
                length: document.tokens.len() as u64,
                predicate_lengths: document.predicate_lengths.clone(),
                partition: partition_of[&document.key],
            })
            .collect();
        let terms = build_terms(documents, dictionary, &table);
        let subject_order = build_subject_order(&table);

        let mut index = Self {
            config,
            ranking,
            predicate_fields: Vec::new(),
            field_lengths: Vec::new(),
            field_totals: Vec::new(),
            unicode: unicode_versions(),
            documents: table,
            subject_order,
            partitions,
            terms,
            fingerprint: [0; FINGERPRINT_BYTES],
            source_fingerprint,
            coverage,
        };
        index.rebuild_field_statistics()?;
        index.fingerprint = index.compute_fingerprint()?;
        Ok(index)
    }

    /// Digest the whole index, in the order [`Self::fingerprint`] documents.
    fn compute_fingerprint(&self) -> Result<[u8; FINGERPRINT_BYTES], TextError> {
        let mut digest = Digest::new(INDEX_DIGEST_DOMAIN);
        digest.text(crate::ANALYZER_PROFILE_ID);
        for byte in self.ranking.fingerprint() {
            digest.tag(byte);
        }

        digest.count(self.config.predicates.len());
        for predicate in &self.config.predicates {
            digest.term(predicate)?;
        }
        match &self.config.graph {
            GraphSelector::Any => digest.tag(SELECTOR_ANY),
            GraphSelector::Default => digest.tag(SELECTOR_DEFAULT),
            GraphSelector::Named(name) => {
                digest.tag(SELECTOR_NAMED);
                digest.term(name)?;
            }
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

        digest.count(self.documents.len());
        for document in &self.documents {
            digest.optional_term(document.graph.as_ref())?;
            digest.term(&document.subject)?;
            digest.optional_text(document.language.as_deref());
            digest.number(document.length);
            digest.count(document.predicate_lengths.len());
            for &(predicate, length) in &document.predicate_lengths {
                digest.number(u64::from(predicate));
                digest.number(length);
            }
            digest.number(u64::from(document.partition));
        }

        digest.count(self.terms.len());
        for entry in &self.terms {
            digest.text(&entry.term);
            digest.count(entry.postings.len());
            for posting in &entry.postings {
                digest.number(u64::from(posting.document));
                digest.count(posting.predicate_frequencies.len());
                for &(predicate, frequency) in &posting.predicate_frequencies {
                    digest.number(u64::from(predicate));
                    digest.number(frequency);
                }
                digest.count(posting.positions.len());
                for position in &posting.positions {
                    digest.number(u64::from(*position));
                }
            }
        }

        digest.count(self.partitions.len());
        for (key, stats) in &self.partitions {
            digest.optional_term(key.graph.as_ref())?;
            digest.optional_text(key.language.as_deref());
            digest.number(stats.document_count);
            digest.number(stats.total_tokens);
            digest.raw(stats.average_document_length.into_raw());
        }

        Ok(digest.finish())
    }
}

/// Document ids sorted by `(subject, id)`.
///
/// Sorted rather than hashed, and by a total order the document table already
/// carries, so the side index is a pure function of the document table exactly
/// as the document table is a pure function of the content. The `id` tiebreak
/// keeps a subject's run ascending, so a lookup's result is ascending too.
fn build_subject_order(table: &[Document]) -> Vec<u32> {
    let mut order: Vec<u32> = (0..table.len())
        .map(|id| u32::try_from(id).expect("the caller has already bounded the id space"))
        .collect();
    order.sort_by(|&left, &right| {
        table[left as usize]
            .subject
            .cmp(&table[right as usize].subject)
            .then_with(|| left.cmp(&right))
    });
    order
}

/// `entry`'s postings that belong to `partition`; empty when it holds none.
fn span_slice(entry: &TermEntry, partition: u32) -> &[Posting] {
    entry
        .spans
        .binary_search_by_key(&partition, |span| span.partition)
        .ok()
        .and_then(|at| entry.spans.get(at))
        .and_then(|span| entry.postings.get(span.start as usize..span.end as usize))
        .unwrap_or(&[])
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
struct SourceRow {
    /// The graph the row was read from, `None` for the default graph.
    graph: Option<TermValue>,
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

impl SourceRow {
    /// The row's object, rebuilt as a term for the source digest.
    fn literal(&self) -> TermValue {
        TermValue::Literal {
            lexical_form: self.lexical_form.clone(),
            datatype: self.datatype.clone(),
            language: self.language.clone(),
            direction: self.direction,
        }
    }
}

/// The `(graph, subject, language)` key that identifies a document.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct DocumentKey {
    /// The graph, `None` for the default graph.
    graph: Option<TermValue>,
    /// The subject.
    subject: TermValue,
    /// The language tag, `None` for untagged literals.
    language: Option<String>,
}

/// A document with its analyzed token stream, before ids are assigned.
#[derive(Clone, Debug)]
struct AnalyzedDocument {
    /// The document's identity.
    key: DocumentKey,
    /// `(dictionary ordinal, position)` for each token, in stream order.
    tokens: Vec<(u32, u32, u32)>,
    /// Token counts by predicate ordinal, before field routing.
    predicate_lengths: Vec<(u32, u64)>,
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
fn collect_rows<D: DatasetView>(
    dataset: &D,
    config: &TextIndexConfig,
) -> Result<(Vec<SourceRow>, SourceCoverage), TextError> {
    let Some(graph) = resolve_graph(dataset, config.graph()) else {
        return Ok((
            Vec::new(),
            SourceCoverage::nothing_in_scope(dataset, config),
        ));
    };
    let mut predicate_ids: Vec<(D::Id, &TermValue, usize)> =
        Vec::with_capacity(config.predicates().len());
    for (ordinal, predicate) in config.predicates().iter().enumerate() {
        if let Some(id) = dataset.term_id_by_value(predicate) {
            predicate_ids.push((id, predicate, ordinal));
        }
    }
    if predicate_ids.is_empty() {
        // No configured predicate is interned, so no statement in either layer
        // can carry one. Returning here rather than sweeping the annotation side
        // table to match every row against an empty id set.
        return Ok((
            Vec::new(),
            SourceCoverage::nothing_in_scope(dataset, config),
        ));
    }

    let mut rows = Vec::new();
    let mut represented = vec![false; config.predicates().len()];

    // Layer one: the asserted triple table.
    for &(predicate_id, predicate, ordinal) in &predicate_ids {
        for quad in dataset.quads_for_pattern(None, Some(predicate_id), None, graph) {
            represented[ordinal] = true;
            push_row(dataset, &mut rows, quad.g, quad.s, predicate, quad.o)?;
        }
    }

    // Layer two: the RDF 1.2 annotation side table, whose subject IS the
    // reifier. `quads_for_pattern` above cannot see these rows at all.
    for quad in dataset.annotation_quads() {
        if !graph.matches(quad.g) {
            continue;
        }
        let Some(&(_, predicate, ordinal)) = predicate_ids.iter().find(|&&(id, _, _)| id == quad.p)
        else {
            continue;
        };
        represented[ordinal] = true;
        push_row(dataset, &mut rows, quad.g, quad.s, predicate, quad.o)?;
    }

    rows.sort();
    let coverage = SourceCoverage::of_walk(dataset, config, &represented);
    Ok((rows, coverage))
}

impl SourceCoverage {
    /// The coverage of a walk that recorded `represented[ordinal]` for each of
    /// `config`'s predicates.
    ///
    /// The dataset is probed for a single statement only when something came up
    /// unrepresented, because that probe exists solely to tell the two
    /// unrepresented cases apart: where every configured predicate was found there
    /// is nothing to tell apart, and a configuration holds at least one predicate,
    /// so finding them all is itself proof the dataset holds statements.
    fn of_walk<D: DatasetView>(
        dataset: &D,
        config: &TextIndexConfig,
        represented: &[bool],
    ) -> Self {
        let unrepresented: Vec<TermValue> = config
            .predicates()
            .iter()
            .zip(represented)
            .filter(|&(_, &seen)| !seen)
            .map(|(predicate, _)| predicate.clone())
            .collect();
        let dataset_holds_statements = unrepresented.is_empty() || holds_a_statement(dataset);
        Self {
            dataset_holds_statements,
            unrepresented,
        }
    }

    /// The coverage of a walk that could not begin: no configured predicate is
    /// interned, or the configured named graph is not a term of this dataset. No
    /// in-scope statement carries any configured predicate, because there is no
    /// in-scope statement to carry one.
    fn nothing_in_scope<D: DatasetView>(dataset: &D, config: &TextIndexConfig) -> Self {
        Self {
            dataset_holds_statements: holds_a_statement(dataset),
            unrepresented: config.predicates().to_vec(),
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
/// Each probe stops at the first row, so this costs one step of up to three lazy
/// iterators rather than a scan.
fn holds_a_statement<D: DatasetView>(dataset: &D) -> bool {
    dataset.quads().next().is_some()
        || dataset.reifier_quads().next().is_some()
        || dataset.annotation_quads().next().is_some()
}

/// Resolve one row's terms and append it, unless its object is not a literal.
fn push_row<D: DatasetView>(
    dataset: &D,
    rows: &mut Vec<SourceRow>,
    graph: Option<D::Id>,
    subject: D::Id,
    predicate: &TermValue,
    object: D::Id,
) -> Result<(), TextError> {
    // A predicate may legitimately carry literals in one statement and an IRI in
    // the next; that is data, not a fault, so a non-literal object contributes
    // no text and raises nothing.
    let TermValue::Literal {
        lexical_form,
        datatype,
        language,
        direction,
    } = resolve_value(dataset, object, 0)?
    else {
        return Ok(());
    };
    let graph = match graph {
        Some(id) => Some(resolve_value(dataset, id, 0)?),
        None => None,
    };
    rows.push(SourceRow {
        graph,
        subject: resolve_value(dataset, subject, 0)?,
        predicate: predicate.clone(),
        datatype,
        lexical_form,
        language,
        direction,
    });
    Ok(())
}

/// Resolve `selector` against `dataset`'s own id space, or `None` when no quad
/// of this dataset can possibly match it.
///
/// `None` arises for exactly one reason: a [`GraphSelector::Named`] graph whose
/// IRI the dataset has not interned. There is then no id a quad's graph could
/// equal, so the match is empty rather than unrepresentable — `GraphMatch` holds
/// dataset-local ids and has no id that names an absent term, so the emptiness is
/// carried here instead of being encoded as one. `Any` and `Default` name no term
/// and so always resolve.
fn resolve_graph<D: DatasetView>(
    dataset: &D,
    selector: &GraphSelector,
) -> Option<GraphMatch<D::Id>> {
    Some(match selector {
        GraphSelector::Any => GraphMatch::Any,
        GraphSelector::Default => GraphMatch::Default,
        GraphSelector::Named(name) => GraphMatch::Named(dataset.term_id_by_value(name)?),
    })
}

/// Resolve a dataset-local id to its dataset-independent [`TermValue`],
/// recursing through a literal's datatype and a triple term's `(s, p, o)`.
///
/// The recursion is the reason a subject may be a triple term without any
/// special case here. `depth` bounds it at the same
/// [`MAX_TRIPLE_DEPTH`](crate::term_bytes::MAX_TRIPLE_DEPTH) the encoder uses:
/// this walks a heap-linked structure with ordinary recursion, and a stack
/// overflow aborts the process rather than raising anything a caller can handle.
fn resolve_value<D: DatasetView>(
    dataset: &D,
    id: D::Id,
    depth: u32,
) -> Result<TermValue, TextError> {
    if depth > MAX_TRIPLE_DEPTH {
        return Err(TextError::data(format!(
            "triple term nests deeper than the resolver's bound of {MAX_TRIPLE_DEPTH}"
        )));
    }
    Ok(match dataset.resolve(id) {
        TermRef::Iri(iri) => TermValue::iri(iri),
        TermRef::Blank { label, scope } => TermValue::Blank {
            label: label.to_owned(),
            scope,
        },
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => {
            let TermRef::Iri(datatype) = dataset.resolve(datatype) else {
                return Err(TextError::data(
                    "a literal's datatype did not resolve to an IRI".to_owned(),
                ));
            };
            TermValue::Literal {
                lexical_form: lexical.to_owned(),
                datatype: datatype.to_owned(),
                language: language.map(str::to_owned),
                direction,
            }
        }
        TermRef::Triple { s, p, o } => TermValue::Triple {
            s: Box::new(resolve_value(dataset, s, depth + 1)?),
            p: Box::new(resolve_value(dataset, p, depth + 1)?),
            o: Box::new(resolve_value(dataset, o, depth + 1)?),
        },
    })
}

// ---------------------------------------------------------------------------
// Building: analysis
// ---------------------------------------------------------------------------

/// Group `rows` into documents, analyze each one, drop the empty ones, and sort
/// what is left into id order.
///
/// Returns the documents and the term dictionary in **intern** order; the
/// dictionary is re-sorted and the ordinals remapped in [`build_terms`].
fn analyze_rows(
    rows: &[SourceRow],
    config: &TextIndexConfig,
) -> Result<(Vec<AnalyzedDocument>, Vec<String>), TextError> {
    let mut groups: FastMap<DocumentKey, Vec<u32>> = FastMap::default();
    for (index, row) in rows.iter().enumerate() {
        let key = DocumentKey {
            graph: row.graph.clone(),
            subject: row.subject.clone(),
            language: row.language.clone(),
        };
        groups.entry(key).or_default().push(index as u32);
    }

    let analyzer = Analyzer::new();
    let mut scratch = String::new();
    let mut ordinals: FastMap<String, u32> = FastMap::default();
    let mut dictionary: Vec<String> = Vec::new();
    let mut documents: Vec<AnalyzedDocument> = Vec::with_capacity(groups.len());

    for (key, mut indices) in groups {
        // The token stream is the document's literals concatenated in sorted
        // `(predicate, lexical form, direction)` order, so it does not depend on
        // the order the dataset yielded them in.
        indices.sort_by(|&left, &right| {
            let (left, right) = (&rows[left as usize], &rows[right as usize]);
            left.predicate
                .cmp(&right.predicate)
                .then_with(|| left.lexical_form.cmp(&right.lexical_form))
                .then_with(|| left.direction.cmp(&right.direction))
        });

        let mut tokens: Vec<(u32, u32, u32)> = Vec::new();
        let mut predicate_lengths: Vec<(u32, u64)> = Vec::new();
        // Positions run consecutively across the whole concatenation rather than
        // restarting per literal, so a phrase can span two of a document's
        // literals exactly as it would span two sentences of one literal.
        let mut position: u32 = 0;
        let mut failure: Option<TextError> = None;
        for index in indices {
            let predicate = config
                .predicates
                .binary_search(&rows[index as usize].predicate)
                .expect("walk selected configured predicates");
            analyzer.analyze_each(&rows[index as usize].lexical_form, &mut scratch, |token| {
                if failure.is_some() {
                    return;
                }
                let ordinal = match ordinals.get(token.text.as_ref()) {
                    Some(&ordinal) => ordinal,
                    None => {
                        let ordinal = dictionary.len() as u32;
                        dictionary.push(token.text.as_ref().to_owned());
                        ordinals.insert(token.text.into_owned(), ordinal);
                        ordinal
                    }
                };
                tokens.push((ordinal, position, predicate as u32));
                let length = match predicate_lengths.last_mut() {
                    Some((prior, length)) if *prior == predicate as u32 => {
                        *length += 1;
                        *length
                    }
                    _ => {
                        predicate_lengths.push((predicate as u32, 1));
                        1
                    }
                };
                if length > FIELD_LENGTH_MAX {
                    failure = Some(TextError::data(
                        "a predicate holds more than 2^24 analyzed tokens in one document",
                    ));
                    return;
                }
                match position.checked_add(1) {
                    Some(next) => position = next,
                    None => {
                        failure = Some(TextError::data(
                            "a document holds more than u32::MAX tokens, which exceeds the \
                             position space the index addresses"
                                .to_owned(),
                        ));
                    }
                }
            });
            if let Some(failure) = failure {
                return Err(failure);
            }
        }

        // A key whose literals analyze to nothing is not a document. See
        // `TextIndex`'s documentation for why this invariant is load-bearing.
        if tokens.is_empty() {
            continue;
        }
        documents.push(AnalyzedDocument {
            key,
            tokens,
            predicate_lengths,
        });
    }

    // Ids are assigned by content order, never by intern order.
    documents.sort_by(|left, right| left.key.cmp(&right.key));
    Ok((documents, dictionary))
}

/// The partition table and the map from a document's key to its partition
/// index — [`build_partitions`]'s two results.
type Partitioning = (
    Vec<(PartitionKey, PartitionStats)>,
    FastMap<DocumentKey, u32>,
);

/// Assign partition indices and compute each partition's statistics.
fn build_partitions(documents: &[AnalyzedDocument]) -> Result<Partitioning, TextError> {
    let mut totals: FastMap<PartitionKey, (u64, u64)> = FastMap::default();
    for document in documents {
        let key = PartitionKey {
            graph: document.key.graph.clone(),
            language: document.key.language.clone(),
        };
        let entry = totals.entry(key).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += document.tokens.len() as u64;
    }

    let mut keys: Vec<PartitionKey> = totals.keys().cloned().collect();
    keys.sort();

    let mut partitions = Vec::with_capacity(keys.len());
    let mut index_of: FastMap<PartitionKey, u32> = FastMap::default();
    for (index, key) in keys.into_iter().enumerate() {
        let (document_count, total_tokens) = totals[&key];
        partitions.push((
            key.clone(),
            PartitionStats {
                document_count,
                total_tokens,
                average_document_length: exact_average(total_tokens, document_count)?,
            },
        ));
        index_of.insert(key, index as u32);
    }

    let mut partition_of: FastMap<DocumentKey, u32> = FastMap::default();
    for document in documents {
        let key = PartitionKey {
            graph: document.key.graph.clone(),
            language: document.key.language.clone(),
        };
        partition_of.insert(document.key.clone(), index_of[&key]);
    }
    Ok((partitions, partition_of))
}

/// `total / count`, exactly, in fixed point.
fn exact_average(total: u64, count: u64) -> Result<Fixed, TextError> {
    let widen = |value: u64| {
        i64::try_from(value)
            .map_err(|_| TextError::overflow(format!("{value} does not fit a fixed-point integer")))
    };
    Fixed::from_integer(widen(total)?)?.checked_div(Fixed::from_integer(widen(count)?)?)
}

/// Sort the dictionary, remap the token ordinals onto it, and build the postings.
fn build_terms(
    documents: &[AnalyzedDocument],
    dictionary: &[String],
    table: &[Document],
) -> Vec<TermEntry> {
    let mut order: Vec<u32> = (0..dictionary.len() as u32).collect();
    order.sort_by(|&left, &right| dictionary[left as usize].cmp(&dictionary[right as usize]));
    let mut rank = vec![0_u32; dictionary.len()];
    for (sorted, &interned) in order.iter().enumerate() {
        rank[interned as usize] = sorted as u32;
    }

    let mut postings: Vec<Vec<Posting>> = vec![Vec::new(); dictionary.len()];
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
            postings[term as usize].push(Posting {
                document: id as u32,
                positions,
                predicate_frequencies,
            });
        }
    }

    let sorted_terms: Vec<String> = order
        .iter()
        .map(|&interned| dictionary[interned as usize].clone())
        .collect();

    let mut entries = Vec::with_capacity(sorted_terms.len());
    for (term, mut list) in sorted_terms.into_iter().zip(postings) {
        // Documents were pushed in ascending id order, so a *stable* sort by
        // partition leaves each partition's run ascending by document id too.
        list.sort_by_key(|posting| table[posting.document as usize].partition);

        let mut spans = Vec::new();
        let mut at = 0;
        while at < list.len() {
            let partition = table[list[at].document as usize].partition;
            let start = at;
            while at < list.len() && table[list[at].document as usize].partition == partition {
                at += 1;
            }
            spans.push(PartitionSpan {
                partition,
                start: start as u32,
                end: at as u32,
            });
        }
        entries.push(TermEntry {
            term,
            postings: list,
            spans,
        });
    }
    entries
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
pub(crate) fn source_digest<D: DatasetView>(
    dataset: &D,
    config: &TextIndexConfig,
) -> Result<([u8; FINGERPRINT_BYTES], SourceCoverage), TextError> {
    let (rows, coverage) = collect_rows(dataset, config)?;
    Ok((digest_rows(&rows)?, coverage))
}

/// Digest the source rows: what the index actually walked, and nothing else.
fn digest_rows(rows: &[SourceRow]) -> Result<[u8; FINGERPRINT_BYTES], TextError> {
    let mut digest = Digest::new(SOURCE_DIGEST_DOMAIN);
    digest.count(rows.len());
    for row in rows {
        digest.optional_term(row.graph.as_ref())?;
        digest.term(&row.subject)?;
        digest.term(&row.predicate)?;
        digest.term(&row.literal())?;
    }
    Ok(digest.finish())
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
    hasher: blake3::Hasher,
    /// The reused encoding buffer.
    scratch: Vec<u8>,
}

impl Digest {
    /// A digest opened under `domain`, so two digests of different things over
    /// the same bytes cannot coincide.
    fn new(domain: &str) -> Self {
        let mut digest = Self {
            hasher: blake3::Hasher::new(),
            scratch: Vec::new(),
        };
        digest.text(domain);
        digest
    }

    /// Absorb a one-byte tag.
    fn tag(&mut self, tag: u8) {
        self.hasher.update(&[tag]);
    }

    /// Absorb a `u64`, little-endian.
    fn number(&mut self, value: u64) {
        self.hasher.update(&value.to_le_bytes());
    }

    /// Absorb an `i128`, little-endian — a [`Fixed`]'s raw representation.
    fn raw(&mut self, value: i128) {
        self.hasher.update(&value.to_le_bytes());
    }

    /// Absorb a collection's length.
    fn count(&mut self, value: usize) {
        self.number(value as u64);
    }

    /// Absorb a length-prefixed string.
    fn text(&mut self, value: &str) {
        self.scratch.clear();
        push_str(value, &mut self.scratch);
        self.hasher.update(&self.scratch);
    }

    /// Absorb an optional string, presence byte first.
    fn optional_text(&mut self, value: Option<&str>) {
        match value {
            Some(value) => {
                self.tag(PRESENT);
                self.text(value);
            }
            None => self.tag(ABSENT),
        }
    }

    /// Absorb a term in the crate's injective encoding.
    fn term(&mut self, value: &TermValue) -> Result<(), TextError> {
        self.scratch.clear();
        encode_term(value, &mut self.scratch)?;
        self.hasher.update(&self.scratch);
        Ok(())
    }

    /// Absorb an optional term, presence byte first.
    fn optional_term(&mut self, value: Option<&TermValue>) -> Result<(), TextError> {
        match value {
            Some(value) => {
                self.tag(PRESENT);
                self.term(value)?;
            }
            None => self.tag(ABSENT),
        }
        Ok(())
    }

    /// The finished digest.
    fn finish(self) -> [u8; FINGERPRINT_BYTES] {
        *self.hasher.finalize().as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
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

    /// An absent optional field is distinguishable from a present empty one, so
    /// an untagged literal's partition cannot collide with a tagged one.
    #[test]
    fn an_absent_language_differs_from_an_empty_one() {
        let of = |language: Option<&str>| {
            let mut digest = Digest::new(SOURCE_DIGEST_DOMAIN);
            digest.optional_text(language);
            digest.finish()
        };
        assert_ne!(of(None), of(Some("")));
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
