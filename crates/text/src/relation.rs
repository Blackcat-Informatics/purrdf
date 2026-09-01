// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The two relations this crate exposes through the evaluator's
//! property-function seam.
//!
//! [`TextSearchRelation`] is ranked retrieval: one row per matching document,
//! carrying the BM25 score and the document's per-partition rank.
//! [`TermOccurrenceRelation`] is positional matching: one row per occurrence of
//! one term, carrying the token ordinal.
//!
//! # Why two relations rather than one with a mode switch
//!
//! They are different relations. A ranked row and an occurrence row have
//! different widths, different positions and different cardinalities — a
//! document contributes exactly one ranked row and arbitrarily many occurrence
//! rows — so a single type would have to carry a discriminator that changed its
//! own arity, and `PfArity` is declared once per relation and checked before any
//! host code runs. The shipped precedent in this workspace is two registered
//! values rather than one type with a switch, and the two are registered under
//! two caller-supplied IRIs exactly as any other pair of relations would be.
//!
//! # PurRDF still mints no vocabulary here
//!
//! Neither relation names an IRI. The predicate a query calls them by is the
//! caller's, supplied at registration; the `example.org` IRIs in the
//! documentation below are fixtures, not defaults.
//!
//! # Wiring
//!
//! A consumer registers the relation in a
//! [`PropertyFunctionRegistry`](purrdf_sparql_eval::PropertyFunctionRegistry)
//! and passes that registry in `QueryOptions::property_functions`. Nothing else
//! is required: the engine derives the parser's property-function IRI set from
//! the registry's own keys, so a registered relation is reachable from query
//! text without any second declaration that could disagree with the first.
//!
//! The one thing the seam cannot check for a consumer is that the index and the
//! dataset are the same data — see [`verify_binding`].

use std::sync::Arc;

use purrdf_core::binding_pattern::BindingPattern;
use purrdf_core::{DatasetView, TermValue};
use purrdf_sparql_eval::{
    EvalError, PfArgs, PfArity, PfCursor, PfRow, PropertyFunction, Volatility,
};

use crate::analysis::Analyzer;
use crate::error::TextError;
use crate::index::{PartitionKey, TextIndex, TextIndexConfig, source_digest};
use crate::score::{Constraint, PartitionFilter, Scored, select};

/// The datatype of a plain string literal.
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
/// The datatype `?score` is emitted as.
const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
/// The datatype `?rank`, `?matched` and `?position` are emitted as.
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
/// The datatype of a language-tagged string.
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// [`TextSearchRelation`]'s `?doc` position.
const SEARCH_DOC: usize = 0;
/// [`TextSearchRelation`]'s needle position — always an input.
const SEARCH_NEEDLE: usize = 1;
/// [`TextSearchRelation`]'s `?score` position.
const SEARCH_SCORE: usize = 2;
/// [`TextSearchRelation`]'s `?rank` position.
const SEARCH_RANK: usize = 3;
/// [`TextSearchRelation`]'s `?lang` position.
const SEARCH_LANG: usize = 4;
/// [`TextSearchRelation`]'s `?matched` position.
const SEARCH_MATCHED: usize = 5;
/// The one access pattern [`TextSearchRelation`] declares.
const SEARCH_MODE: &str = "fbffff";

/// [`TermOccurrenceRelation`]'s `?doc` position.
const OCCURRENCE_DOC: usize = 0;
/// [`TermOccurrenceRelation`]'s term position — always an input.
const OCCURRENCE_TERM: usize = 1;
/// [`TermOccurrenceRelation`]'s `?lang` position.
const OCCURRENCE_LANG: usize = 2;
/// [`TermOccurrenceRelation`]'s `?position` position.
const OCCURRENCE_POSITION: usize = 3;
/// The one access pattern [`TermOccurrenceRelation`] declares.
const OCCURRENCE_MODE: &str = "fbff";

// ---------------------------------------------------------------------------
// Shared argument reading
// ---------------------------------------------------------------------------

/// A bound `?rank`, reduced to what the ranker can act on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RankBound {
    /// `?rank` is free; every rank qualifies.
    Unbound,
    /// `?rank` is bound to this 1-based per-partition position.
    At(u32),
    /// `?rank` is bound to an integer beyond the position space an index
    /// numbers, so no partition can hold that row. An honest empty answer, not
    /// an error — see [`TextSearchRelation::open`].
    BeyondTheIndex,
}

/// The lexical form of a needle bound at `position`.
fn needle_text(value: &TermValue, position: usize) -> Result<&str, EvalError> {
    match value {
        TermValue::Literal {
            lexical_form,
            datatype,
            ..
        } if datatype == XSD_STRING || datatype == RDF_LANG_STRING => Ok(lexical_form),
        TermValue::Literal { datatype, .. } => Err(EvalError::function(format!(
            "the needle at position {position} is a literal of datatype <{datatype}>; a text \
             search reads a string, so the needle must be an xsd:string or an rdf:langString"
        ))),
        other => Err(EvalError::function(format!(
            "the needle at position {position} is {other:?}; only a literal carries text, so an \
             IRI, a blank node or a triple term names nothing this relation can search for"
        ))),
    }
}

/// The partition constraint a bound `?lang` at `position` compiles to.
///
/// The empty string selects the untagged partition, which is
/// [`Constraint::Absent`] rather than [`Constraint::Exactly`] of an empty tag:
/// the two are distinct partitions, and collapsing them would make a query for
/// untagged text answer with text tagged by an empty string, or the reverse.
fn language_constraint(
    value: &TermValue,
    position: usize,
) -> Result<Constraint<String>, EvalError> {
    match value {
        TermValue::Literal {
            lexical_form,
            datatype,
            ..
        } if datatype == XSD_STRING => Ok(if lexical_form.is_empty() {
            Constraint::Absent
        } else {
            Constraint::Exactly(lexical_form.clone())
        }),
        other => Err(EvalError::function(format!(
            "the language at position {position} is {other:?}; this relation emits an xsd:string \
             there — the tag itself, or the empty string for an untagged document — so nothing \
             else can name a language"
        ))),
    }
}

/// An `xsd:integer` lexical form split into its sign and its digits, or `None`
/// if it is not one.
///
/// The production is `[+-]? [0-9]+` — XSD's own, with no whitespace, no
/// exponent and no radix point. Leading zeros and a leading `+` are both
/// permitted (`"+007"` and `"7"` denote the same integer), so they are stripped
/// here rather than refused.
///
/// This exists because the value space of `xsd:integer` is **unbounded** while
/// every Rust integer type is not, and conflating the two is how a validator
/// starts refusing values that are perfectly valid. Deciding well-formedness
/// from the lexical form itself keeps that judgement independent of whatever
/// width the code happens to parse into.
fn xsd_integer_parts(lexical_form: &str) -> Option<(bool, &str)> {
    let (negative, digits) = match lexical_form.as_bytes().first() {
        Some(b'-') => (true, &lexical_form[1..]),
        Some(b'+') => (false, &lexical_form[1..]),
        _ => (false, lexical_form),
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    // Leading zeros are lexical noise, not magnitude.
    let trimmed = digits.trim_start_matches('0');
    Some((negative, trimmed))
}

/// The rank a bound `?rank` at [`SEARCH_RANK`] compiles to.
///
/// # The three answers, and why a large rank is not an error
///
/// `xsd:integer` has an **unbounded** value space. A rank of `10^40` is
/// therefore a perfectly well-formed `xsd:integer` — it is simply larger than
/// any position an index numbers, which is exactly the
/// [`RankBound::BeyondTheIndex`] case the relation already answers emptily for
/// `4294967296`. Deciding this by whether the lexical form fits an `i128` would
/// make the boundary between "empty answer" and "aborted query" an artefact of
/// the width this function happens to parse into: `2^127 - 1` would be an empty
/// answer and `2^127` a hard error, with nothing about `xsd:integer` to justify
/// the difference. So magnitude is read from the digits, and only a lexical form
/// that is not an `xsd:integer` at all is refused as malformed.
///
/// A negative rank stays a refusal at every magnitude. It is not a row the index
/// might have and does not: 1-based positions have no negative region, so the
/// request is outside the domain rather than empty within it.
fn rank_bound(value: &TermValue) -> Result<RankBound, EvalError> {
    let TermValue::Literal {
        lexical_form,
        datatype,
        ..
    } = value
    else {
        return Err(EvalError::function(format!(
            "the rank at position {SEARCH_RANK} is {value:?}; a rank is a 1-based position, which \
             only an xsd:integer literal can name"
        )));
    };
    if datatype != XSD_INTEGER {
        return Err(EvalError::function(format!(
            "the rank at position {SEARCH_RANK} is a literal of datatype <{datatype}>; this \
             relation emits an xsd:integer there, so only an xsd:integer can name a rank"
        )));
    }
    let Some((negative, digits)) = xsd_integer_parts(lexical_form) else {
        return Err(EvalError::function(format!(
            "the rank at position {SEARCH_RANK} has lexical form {lexical_form:?}, which is not in \
             the lexical space of xsd:integer (an optional sign followed by one or more digits)"
        )));
    };
    // `digits` has had its leading zeros stripped, so it is empty exactly when
    // the value is zero — whatever sign was written in front of it.
    if digits.is_empty() || negative {
        return Err(EvalError::function(format!(
            "the rank at position {SEARCH_RANK} is {lexical_form}; ranks are 1-based, so zero and \
             every negative value are outside the domain rather than an empty answer within it"
        )));
    }
    // Strictly positive from here, so the only question left is whether it names
    // a position an index can number. One that does not is a question the index
    // answers with nothing rather than a request it refuses.
    Ok(digits
        .parse::<u32>()
        .map_or(RankBound::BeyondTheIndex, RankBound::At))
}

/// The analyzed needle — the terms the index's own pipeline produces for `text`.
fn analyze(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    Analyzer::new().analyze(text, &mut tokens);
    tokens
        .into_iter()
        .map(|token| token.text.into_owned())
        .collect()
}

/// Whether `row` agrees with every bound position of the invocation.
fn agrees(bound: &[Option<TermValue>], row: &[TermValue]) -> bool {
    bound
        .iter()
        .zip(row)
        .all(|(want, have)| want.as_ref().is_none_or(|want| want == have))
}

/// The invocation's bound values by flattened position, cloned out of the
/// borrow `args` lends for the duration of `open`.
fn bound_values(args: &PfArgs<'_>) -> Vec<Option<TermValue>> {
    args.flattened().map(<Option<&TermValue>>::cloned).collect()
}

/// A `?lang` cell: the tag itself, or the empty string for an untagged document.
fn language_term(language: Option<&str>) -> TermValue {
    TermValue::simple_literal(language.unwrap_or(""))
}

/// An `xsd:integer` cell.
fn integer_term(value: u32) -> TermValue {
    TermValue::typed_literal(value.to_string(), XSD_INTEGER)
}

/// Refuse an invocation whose argument vectors do not match `declared`.
///
/// `open_contained` already checked this for every engine-driven call; a direct
/// caller gets the same answer rather than an out-of-range read.
fn check_arity(args: &PfArgs<'_>, declared: PfArity, what: &str) -> Result<(), EvalError> {
    let supplied = args.arity();
    if supplied == declared {
        return Ok(());
    }
    Err(EvalError::function(format!(
        "the {what} relation expects {declared} argument(s), got {supplied}"
    )))
}

// ---------------------------------------------------------------------------
// Partition grouping, shared by both row bounds
// ---------------------------------------------------------------------------

/// The partitions of `keys` grouped by language, as index lists.
///
/// Built by sorting rather than by hashing, so the grouping — and therefore
/// every row bound derived from it — is a pure function of the index's
/// contents.
fn language_groups(keys: &[PartitionKey]) -> Vec<Vec<usize>> {
    let mut order: Vec<usize> = (0..keys.len()).collect();
    order.sort_by(|&left, &right| keys[left].language().cmp(&keys[right].language()));

    let mut groups: Vec<Vec<usize>> = Vec::new();
    for at in order {
        match groups.last_mut() {
            Some(group) if keys[group[0]].language() == keys[at].language() => group.push(at),
            _ => groups.push(vec![at]),
        }
    }
    groups
}

/// How many distinct graphs the index's partitions name, counting the default
/// graph as one of them.
fn distinct_graphs(keys: &[PartitionKey]) -> u64 {
    let mut graphs: Vec<Option<&TermValue>> = keys.iter().map(PartitionKey::graph).collect();
    graphs.sort_unstable();
    graphs.dedup();
    graphs.len() as u64
}

/// Every partition key of `index`, ascending — the order both relations emit in.
fn partition_keys(index: &TextIndex) -> Vec<PartitionKey> {
    index.partitions().map(|(key, _)| key.clone()).collect()
}

// ---------------------------------------------------------------------------
// Ranked retrieval
// ---------------------------------------------------------------------------

/// The maxima [`TextSearchRelation::rows_per_invocation`] is computed from,
/// measured once at relation construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SearchBounds {
    /// Every document the index retains.
    documents: u64,
    /// Every partition the index holds.
    partitions: u64,
    /// How many distinct graphs those partitions name.
    graphs: u64,
    /// The largest number of partitions any one language spans.
    partitions_per_language: u64,
    /// The largest number of documents any one language holds, summed across
    /// the graphs that language appears in.
    documents_per_language: u64,
}

impl SearchBounds {
    /// Measure `index`.
    fn of(index: &TextIndex) -> Self {
        let keys = partition_keys(index);
        let documents_in: Vec<u64> = index
            .partitions()
            .map(|(_, stats)| stats.document_count())
            .collect();
        let groups = language_groups(&keys);
        Self {
            documents: index.document_count(),
            partitions: index.partition_count(),
            graphs: distinct_graphs(&keys),
            partitions_per_language: groups
                .iter()
                .map(|group| group.len() as u64)
                .max()
                .unwrap_or(0),
            documents_per_language: groups
                .iter()
                .map(|group| {
                    group
                        .iter()
                        .map(|&at| documents_in[at])
                        .fold(0_u64, u64::saturating_add)
                })
                .max()
                .unwrap_or(0),
        }
    }

    /// The declared bound for an invocation binding the positions named.
    ///
    /// Every step is a `min` of bounds that each hold independently, so the
    /// result is the tightest of them and can never exceed any one of them.
    fn for_mode(self, document: bool, rank: bool, language: bool) -> u64 {
        let mut bound = self.documents;
        if document {
            bound = bound.min(self.partitions);
        }
        if rank {
            bound = bound.min(self.partitions);
        }
        if language {
            bound = bound.min(self.documents_per_language);
        }
        if rank && language {
            bound = bound.min(self.partitions_per_language);
        }
        if document && language {
            bound = bound.min(self.graphs);
        }
        bound
    }
}

/// Ranked retrieval over a frozen [`TextIndex`], as a SPARQL relation.
///
/// One subject-side argument and five object-side ones, so a call reads
///
/// ```text
/// ?doc <ex:search> ( "needle" ?score ?rank ?lang ?matched ) .
/// ```
///
/// over the six flattened positions:
///
/// | pos | name | role | emitted term |
/// |---|---|---|---|
/// | 0 | `?doc` | the document's subject | the subject verbatim — an IRI, a blank node, or an RDF 1.2 reifier |
/// | 1 | needle | **input, must be bound** | echoed back |
/// | 2 | `?score` | the exact BM25 score | `xsd:decimal` |
/// | 3 | `?rank` | 1-based rank **within the document's partition** | `xsd:integer` |
/// | 4 | `?lang` | the document's language tag | `xsd:string` |
/// | 5 | `?matched` | how many distinct needle terms the document holds | `xsd:integer` |
///
/// # Position 3 is a per-partition rank, and nothing about the value says so
///
/// The Rust type names it [`Scored::partition_rank`](crate::Scored) precisely so
/// it cannot be read as a global position. A SPARQL row cannot carry that name —
/// the caller writes the variable — so it is stated here instead, and it is the
/// one thing about this relation worth reading twice.
///
/// A partition is a `(graph, language)` pair, and corpus statistics are computed
/// inside one. A `1` from the English partition and a `1` from the French one are
/// both first-in-their-corpus, and **an answer over a multi-partition index
/// therefore contains one row of rank 1 per partition.** `LIMIT 10` after
/// `ORDER BY ?rank` over a three-language index is not the ten best documents; it
/// is the first ten of a sequence that opens with three rank-1 rows.
///
/// What makes a single ranked list is naming the partition:
///
/// ```text
/// ?doc <ex:search> ( "needle" ?score ?rank "en" ?matched ) .
/// ```
///
/// or an index whose configuration already spans one
/// ([`GraphSelector::Named`](crate::GraphSelector::Named) or
/// [`GraphSelector::Default`](crate::GraphSelector::Default) over a corpus in one
/// language), which is the common case. Ranking across partitions is not offered
/// because the numbers being ordered would have been computed against different
/// corpora — see [`crate::rank_partition`]'s module documentation for why that is
/// an arrangement rather than a ranking.
///
/// # `?lang` uses the empty string for an untagged document
///
/// A fixed-width row needs a term in every position, and an untagged document
/// has no tag. The empty string is the value that cannot be mistaken for one:
/// BCP 47 requires a language tag to carry at least one primary subtag, so `""`
/// is not a well-formed tag and no real tag can collide with it. A query for
/// untagged text is therefore `FILTER(?lang = "")`, and the relation reads a
/// bound `""` back as the untagged partition rather than as a tag.
///
/// Tags reaching this position are the ones the IR stored, which are
/// lowercased, so `"EN"` bound at `?lang` selects nothing.
///
/// # There is deliberately no `?graph` position
///
/// The graph **is** part of a document's key, so text is never merged across
/// graphs and a score is never computed over another graph's corpus. What is
/// missing is only the ability to *read the graph back out* as a row value, and
/// that absence is deliberate.
///
/// RDF provides no term that denotes the default graph. A fixed-width
/// [`PfRow`] must put something in every position, so a `?graph` position would
/// force PurRDF to mint a sentinel IRI for "the default graph" — a vocabulary
/// IRI of this project's own, appearing in query answers as data, which this
/// workspace forbids outright. Encoding the default graph as an unbound cell is
/// not available either: a row is a value per position, not an optional one.
///
/// A caller that needs per-graph search states the graph in the *configuration*
/// instead, with [`GraphSelector::Named`](crate::GraphSelector::Named) or
/// [`GraphSelector::Default`](crate::GraphSelector::Default), and gets an index
/// whose every document is from that graph. That is a stronger guarantee than a
/// `?graph` column would give, because it also removes the other graphs from
/// the corpus statistics.
///
/// # It is [`Volatility::Stable`]
///
/// The index is frozen and the arithmetic is exact fixed point, so an
/// invocation's rows are a pure function of its arguments for the lifetime of a
/// query — the same answer on the main thread and on a fork-join worker, and
/// the same answer on `wasm32-unknown-unknown` as on a native build. That is
/// what the stable class asserts, so the relation may run across workers.
#[derive(Clone, Debug)]
pub struct TextSearchRelation {
    /// The index every invocation is answered from.
    index: Arc<TextIndex>,
    /// The single declared mode, materialized once so [`PropertyFunction::modes`]
    /// can hand out a slice.
    modes: [BindingPattern; 1],
    /// The row maxima, measured once at construction.
    bounds: SearchBounds,
}

impl TextSearchRelation {
    /// A ranked-retrieval relation over `index`.
    ///
    /// The row bounds this relation declares are measured here, once, rather
    /// than recomputed per invocation: they are a function of the index, and
    /// the index is frozen.
    #[must_use]
    pub fn new(index: Arc<TextIndex>) -> Self {
        let bounds = SearchBounds::of(&index);
        Self {
            index,
            modes: [BindingPattern::from_code(SEARCH_MODE)],
            bounds,
        }
    }

    /// The index this relation answers from.
    #[must_use]
    pub fn index(&self) -> &TextIndex {
        &self.index
    }
}

impl PropertyFunction for TextSearchRelation {
    fn volatility(&self) -> Volatility {
        // A frozen index is deterministic for a query's lifetime, so this
        // relation may run across fork-join workers. See the type's docs.
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        PfArity::new(1, 5)
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    /// The declared row bound, as a real function of the mode.
    ///
    /// Only three of the six positions bound anything. `?doc` (0), `?rank` (3)
    /// and `?lang` (4) each restrict which documents can appear; `?score` (2)
    /// and `?matched` (5) restrict nothing, because arbitrarily many documents
    /// can share either value, and the needle (1) is bound in every admitted
    /// invocation.
    ///
    /// | bound positions | declared bound | why |
    /// |---|---|---|
    /// | 3 and 4 | largest number of partitions any one language spans | a rank names at most one row per partition, and the language names the partitions |
    /// | 0 and 4 | number of distinct graphs | `(graph, subject, language)` is a document's key, so one `(subject, language)` pair occurs at most once per graph |
    /// | 3 only | number of partitions | one row per partition |
    /// | 0 only | number of partitions | the same subject may appear in several partitions, but at most once in each |
    /// | 4 only | largest number of documents any one language holds, summed across graphs | |
    /// | none of them | number of documents | one row per document |
    ///
    /// The first two rows are **exactly one** for a single-graph index, which
    /// is what a [`GraphSelector::Named`](crate::GraphSelector::Named) or
    /// [`GraphSelector::Default`](crate::GraphSelector::Default) configuration
    /// always produces. That assumption is not asserted; it is *measured*, so a
    /// [`GraphSelector::Any`](crate::GraphSelector::Any) index spanning three
    /// graphs declares three rather than claiming one it could exceed.
    ///
    /// Combinations not listed take the minimum of the bounds that apply, and
    /// every accumulation is saturating, so no bound can wrap to a dishonest
    /// zero.
    fn rows_per_invocation(&self, mode: BindingPattern) -> u64 {
        self.bounds.for_mode(
            mode.is_bound(SEARCH_DOC),
            mode.is_bound(SEARCH_RANK),
            mode.is_bound(SEARCH_LANG),
        )
    }

    /// Begin one ranked-retrieval invocation.
    ///
    /// # Refusals
    ///
    /// Each of these aborts the query rather than contributing zero rows, which
    /// would be indistinguishable from an honest empty answer:
    ///
    /// * the needle at position 1 is free — this relation cannot enumerate
    ///   needles;
    /// * the needle is not an `xsd:string` or `rdf:langString` literal;
    /// * `?rank` is bound to something other than an `xsd:integer`, or to an
    ///   integer below one, which is outside a 1-based domain;
    /// * `?lang` is bound to something other than an `xsd:string`.
    ///
    /// # What is an empty answer rather than a refusal
    ///
    /// A needle that analyzes to **no terms** — `"---"`, say — is a well-formed
    /// request that names no terms, so it matches nothing. Refusing it would
    /// contradict the index side, which drops a document whose literals analyze
    /// to nothing rather than failing the build, and would turn a legitimate
    /// empty result into an aborted query over one row of data-dependent input.
    ///
    /// A `?rank` past the end of every partition is likewise empty: asking for
    /// a row a partition does not have is a question with an answer.
    fn open(
        &self,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        check_arity(args, self.arity(), "text search")?;

        let Some(needle) = args.get(SEARCH_NEEDLE) else {
            return Err(EvalError::function(format!(
                "the needle at position {SEARCH_NEEDLE} is free; this relation retrieves documents \
                 for a needle and cannot enumerate needles for a document, which is why its only \
                 declared mode is `{SEARCH_MODE}`"
            )));
        };
        let text = needle_text(needle, SEARCH_NEEDLE)?;

        let rank = match args.get(SEARCH_RANK) {
            Some(value) => rank_bound(value)?,
            None => RankBound::Unbound,
        };
        let language = match args.get(SEARCH_LANG) {
            Some(value) => language_constraint(value, SEARCH_LANG)?,
            None => Constraint::Any,
        };
        let mut filter = PartitionFilter::unconstrained().with_language(language);

        // A bound `?doc` is pushed down to the partitions that subject actually
        // appears in, which is what keeps a bound-document call from being a
        // whole-index ranking.
        //
        // The engine drives a property function once per left row, so a pattern
        // like `?doc ex:label ?l . ?doc <ex:search> ( "cat" … )` opens this
        // relation once for every candidate document. Without the pushdown each
        // of those invocations ranks EVERY partition in full — `?doc` is a
        // post-rank position, so the ceiling is withheld (see below) and nothing
        // else narrows the work — and then the cursor discards all but the one
        // row whose subject matches. That is `O(left rows × whole index)`, and
        // the discarded fraction grows with the corpus.
        //
        // A subject is one of `(graph, subject, language)`, so it occupies at
        // most one document per partition and usually appears in one or two
        // partitions out of however many the index holds. Restricting to exactly
        // those turns the per-invocation cost into `O(that subject's partitions)`
        // and removes the whole-index term from the product.
        //
        // It is sound for the same reason the language pushdown is: ranks are
        // per-partition, so dropping whole partitions cannot change the rank of
        // any row that survives, and every row dropped here is one the cursor's
        // equality filter on position 0 would have dropped anyway. A subject the
        // index holds no text for admits no partition at all, which is the
        // correct empty answer reached without ranking anything.
        if let Some(subject) = args.get(SEARCH_DOC) {
            filter = filter.restricted_to(self.index.partitions_holding_subject(subject));
        }

        // The ceiling handling, and why it is not simply passed through.
        //
        // `args_are_admission_transparent` treats a CONSTANT at any position as
        // transparent, so the engine offers a ceiling even for a call written
        // `ex:a <search> ( "cat" ?s ?r ?l ?m )`. Positions 0, 2 and 5 are
        // filtered *after* ranking, so handing that ceiling to `select` would
        // truncate the ranking first and let the cursor filter a prefix — it
        // could then emit fewer than `k` rows and report exhaustion while
        // matching rows sat beyond the truncation, which the engine reads as a
        // complete answer. So on any of those the ranking is computed in full
        // and the cursor does the cutting.
        //
        // A bound `?lang` (4) is different: it is pushed into the partition
        // filter *before* ranking, which is sound because ranks are
        // per-partition — dropping whole partitions cannot change a surviving
        // row's rank. A bound `?rank` (3) is likewise handed to `select`, which
        // applies it before its own truncation. Neither can drop a row the
        // cursor would have emitted, so the ceiling stays valid on those paths.
        let post_rank_filtered = args.get(SEARCH_DOC).is_some()
            || args.get(SEARCH_SCORE).is_some()
            || args.get(SEARCH_MATCHED).is_some();
        let select_ceiling = if post_rank_filtered { None } else { ceiling };

        let analyzed = analyze(text);
        let rows = match rank {
            RankBound::BeyondTheIndex => Vec::new(),
            RankBound::Unbound => select(&self.index, &analyzed, &filter, select_ceiling, None)?,
            RankBound::At(at) => select(&self.index, &analyzed, &filter, select_ceiling, Some(at))?,
        };

        Ok(Box::new(SearchCursor {
            index: Arc::clone(&self.index),
            needle: needle.clone(),
            rows,
            at: 0,
            bound: bound_values(args),
            remaining: ceiling,
        }))
    }
}

/// The cursor [`TextSearchRelation::open`] returns: the ranked rows, filtered
/// on every bound position and cut at the engine's licence.
///
/// Two properties make this sound, and both are load-bearing.
///
/// * **It filters on every bound position, not only the ones `select` already
///   applied.** `?doc`, `?score` and `?matched` are invisible to the ranker, so
///   the equality check runs here for all six positions. A relation is entitled
///   to generate candidates and let the engine's own filter cut them, but a
///   relation that also *spends a ceiling* on them would hand back fewer usable
///   rows than the engine asked for.
/// * **It decrements the licence only on rows it actually emits.** A row this
///   cursor skips disagrees with a bound position and would have been dropped
///   by the engine anyway, so counting it would be exactly the miscount the
///   seam's ceiling contract warns about.
///
/// Those two together are why `open` withholds the ceiling from `select`
/// whenever a post-rank position is bound — including when it is bound by a
/// **constant** written at the call site, which the engine still treats as
/// admission-transparent and so still offers a ceiling for.
#[derive(Debug)]
struct SearchCursor {
    /// The index the rows' subjects and languages are read from.
    index: Arc<TextIndex>,
    /// The needle, echoed verbatim into position 1 of every row.
    needle: TermValue,
    /// The ranked rows, in `(partition ASC, rank ASC)` order.
    rows: Vec<Scored>,
    /// How far into `rows` the cursor has read.
    at: usize,
    /// The invocation's bound values by flattened position (`None` = free).
    bound: Vec<Option<TermValue>>,
    /// The rows this invocation may still emit under the engine's licence.
    remaining: Option<u64>,
}

impl SearchCursor {
    /// The full row for one ranked document.
    fn build(&self, scored: &Scored) -> Result<PfRow, EvalError> {
        let document = self.index.document(scored.document).ok_or_else(|| {
            EvalError::data(format!(
                "the ranker named document {}, which the index does not hold",
                scored.document
            ))
        })?;
        Ok(vec![
            document.subject().clone(),
            self.needle.clone(),
            TermValue::typed_literal(scored.score.to_decimal_lexical(), XSD_DECIMAL),
            integer_term(scored.partition_rank),
            language_term(document.language()),
            integer_term(scored.matched),
        ])
    }
}

impl PfCursor for SearchCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        if self.remaining == Some(0) {
            return Ok(None);
        }
        while let Some(scored) = self.rows.get(self.at) {
            self.at += 1;
            let row = self.build(scored)?;
            if agrees(&self.bound, &row) {
                if let Some(remaining) = self.remaining.as_mut() {
                    *remaining = remaining.saturating_sub(1);
                }
                return Ok(Some(row));
            }
        }
        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// Positional matching
// ---------------------------------------------------------------------------

/// The maxima [`TermOccurrenceRelation::rows_per_invocation`] is computed from,
/// measured once at relation construction by walking the postings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OccurrenceBounds {
    /// The largest total occurrence count of any one term, across every
    /// partition. Occurrences, not documents: a term occurring four times in
    /// one document is four rows.
    occurrences: u64,
    /// The largest number of documents holding any one term.
    documents_holding: u64,
    /// The largest number of times any one term occurs in any one document.
    per_document: u64,
    /// The largest number of occurrences of any one term within any one
    /// language, summed across the graphs that language appears in.
    per_language: u64,
    /// Every partition the index holds.
    partitions: u64,
    /// How many distinct graphs those partitions name.
    graphs: u64,
}

impl OccurrenceBounds {
    /// Measure `index` by walking every term's postings once.
    fn of(index: &TextIndex) -> Self {
        let keys = partition_keys(index);
        let groups = language_groups(&keys);

        let mut occurrences = 0_u64;
        let mut documents_holding = 0_u64;
        let mut per_document = 0_u64;
        let mut per_language = 0_u64;
        let mut in_partition = vec![0_u64; keys.len()];

        for term in index.terms() {
            in_partition.fill(0);
            let mut total = 0_u64;
            let mut holding = 0_u64;
            for (at, key) in keys.iter().enumerate() {
                for (_, positions) in index.postings(key, term) {
                    let count = positions.len() as u64;
                    total = total.saturating_add(count);
                    holding = holding.saturating_add(1);
                    per_document = per_document.max(count);
                    in_partition[at] = in_partition[at].saturating_add(count);
                }
            }
            occurrences = occurrences.max(total);
            documents_holding = documents_holding.max(holding);
            for group in &groups {
                let summed = group
                    .iter()
                    .map(|&at| in_partition[at])
                    .fold(0_u64, u64::saturating_add);
                per_language = per_language.max(summed);
            }
        }

        Self {
            occurrences,
            documents_holding,
            per_document,
            per_language,
            partitions: index.partition_count(),
            graphs: distinct_graphs(&keys),
        }
    }

    /// The declared bound for an invocation binding the positions named.
    fn for_mode(self, document: bool, language: bool, position: bool) -> u64 {
        let mut bound = self.occurrences;
        if position {
            bound = bound.min(self.documents_holding);
        }
        if language {
            bound = bound.min(self.per_language);
        }
        if document {
            bound = bound.min(self.partitions.saturating_mul(self.per_document));
        }
        if document && language {
            bound = bound.min(self.graphs.saturating_mul(self.per_document));
        }
        if document && position {
            bound = bound.min(self.partitions);
        }
        if document && language && position {
            bound = bound.min(self.graphs);
        }
        bound
    }
}

/// Positional matching over a frozen [`TextIndex`], as a SPARQL relation.
///
/// One subject-side argument and three object-side ones, so a call reads
///
/// ```text
/// ?doc <ex:occurs> ( "term" ?lang ?position ) .
/// ```
///
/// over the four flattened positions:
///
/// | pos | name | role | emitted term |
/// |---|---|---|---|
/// | 0 | `?doc` | the document's subject | the subject verbatim |
/// | 1 | term | **input, must be bound** | echoed back |
/// | 2 | `?lang` | the document's language tag | `xsd:string`, `""` when untagged |
/// | 3 | `?position` | the token ordinal of one occurrence | `xsd:integer` |
///
/// One row per occurrence, in `(partition ASC, document ASC, position ASC)`
/// order.
///
/// # This is how phrase and proximity are expressed
///
/// PurRDF mints no query dialect: there is no phrase operator, no slop
/// parameter and no `NEAR`. There does not need to be, because a caller already
/// has a language for stating a relationship between two numbers. Adjacency is
///
/// ```text
/// ?doc <ex:occurs> ( "quick" ?l ?p1 ) .
/// ?doc <ex:occurs> ( "brown" ?l ?p2 ) .
/// FILTER(?p2 = ?p1 + 1)
/// ```
///
/// and proximity within three tokens is the same two calls under
/// `FILTER(ABS(?p2 - ?p1) <= 3)`. Repeating `?l` across both calls is what
/// keeps the two occurrences in the same document rather than in two documents
/// that merely share a subject in different languages.
///
/// Token positions run consecutively across the whole of a document's
/// concatenated literals, so a phrase may span two literals of one subject
/// exactly as it spans two sentences of one literal.
///
/// # One term per invocation, by contract
///
/// The term is analyzed through the index's own pipeline, so the caller writes
/// the word rather than its folded form. If that analysis yields **more than
/// one** term — a compound the tokenizer splits, or a CJK run that bigrams —
/// the invocation is refused rather than silently answered about one of them.
/// A multi-term needle is written as one call per term, joined on `?doc`, which
/// is the same shape the phrase example above already has.
///
/// A term analyzing to **zero** terms is an empty answer rather than a refusal,
/// exactly as it is for [`TextSearchRelation`].
///
/// # It is [`Volatility::Stable`]
///
/// A frozen index's postings are the same postings on every worker and on every
/// target, so an invocation's rows are a pure function of its arguments for the
/// lifetime of a query and the relation may run across fork-join workers.
#[derive(Clone, Debug)]
pub struct TermOccurrenceRelation {
    /// The index every invocation is answered from.
    index: Arc<TextIndex>,
    /// The single declared mode, materialized once.
    modes: [BindingPattern; 1],
    /// The row maxima, measured once at construction.
    bounds: OccurrenceBounds,
}

impl TermOccurrenceRelation {
    /// A positional-matching relation over `index`.
    ///
    /// Walks every term's postings once to measure the row bounds it declares;
    /// the index is frozen, so they are measured here rather than per
    /// invocation.
    #[must_use]
    pub fn new(index: Arc<TextIndex>) -> Self {
        let bounds = OccurrenceBounds::of(&index);
        Self {
            index,
            modes: [BindingPattern::from_code(OCCURRENCE_MODE)],
            bounds,
        }
    }

    /// The index this relation answers from.
    #[must_use]
    pub fn index(&self) -> &TextIndex {
        &self.index
    }
}

impl PropertyFunction for TermOccurrenceRelation {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        PfArity::new(1, 3)
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    /// The declared row bound, as a real function of the mode.
    ///
    /// The analogue of [`TextSearchRelation::rows_per_invocation`]'s table, but
    /// counted in **occurrences** rather than documents, because that is what a
    /// row is here.
    ///
    /// | bound positions | declared bound | why |
    /// |---|---|---|
    /// | none of them | largest total occurrence count of any one term | one row per occurrence |
    /// | 3 (`?position`) only | largest number of documents holding any one term | a position names at most one occurrence per document |
    /// | 2 (`?lang`) only | largest occurrence count of any one term within one language | |
    /// | 0 (`?doc`) only | partitions × largest occurrence count in one document | a subject occurs at most once per partition |
    /// | 0 and 2 | graphs × largest occurrence count in one document | a `(subject, language)` pair occurs at most once per graph |
    /// | 0 and 3 | number of partitions | one occurrence per document, one document per partition |
    /// | 0, 2 and 3 | number of distinct graphs | the three together name at most one occurrence per graph |
    ///
    /// The last row is **exactly one** for a single-graph index. As with the
    /// search relation, the graph count is measured rather than assumed.
    /// Combinations not listed take the minimum of the bounds that apply, and
    /// every product and sum is saturating.
    fn rows_per_invocation(&self, mode: BindingPattern) -> u64 {
        self.bounds.for_mode(
            mode.is_bound(OCCURRENCE_DOC),
            mode.is_bound(OCCURRENCE_LANG),
            mode.is_bound(OCCURRENCE_POSITION),
        )
    }

    /// Begin one positional-matching invocation.
    ///
    /// # Refusals
    ///
    /// * the term at position 1 is free, or is not an `xsd:string` or
    ///   `rdf:langString` literal;
    /// * the term analyzes to more than one term — see the type's docs;
    /// * `?lang` is bound to something other than an `xsd:string`.
    ///
    /// `?position` is not on that list. It is applied as a plain equality
    /// filter over the emitted rows, which is precisely what the engine itself
    /// would do with a bound position, so a value outside the ordinal space
    /// matches no occurrence and the answer is honestly empty.
    ///
    /// # The ceiling is taken as offered
    ///
    /// Unlike the search relation there is no truncation-before-filtering
    /// hazard here: rows are generated lazily from the postings, in emission
    /// order, and nothing upstream of the cursor drops a row. The cursor
    /// therefore filters on every bound position and spends the licence only on
    /// rows it emits, and the prefix it produces is the prefix of the unbounded
    /// answer whatever is bound.
    fn open(
        &self,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        check_arity(args, self.arity(), "term occurrence")?;

        let Some(needle) = args.get(OCCURRENCE_TERM) else {
            return Err(EvalError::function(format!(
                "the term at position {OCCURRENCE_TERM} is free; this relation enumerates the \
                 occurrences of a term and cannot enumerate terms, which is why its only declared \
                 mode is `{OCCURRENCE_MODE}`"
            )));
        };
        let text = needle_text(needle, OCCURRENCE_TERM)?;

        let language = match args.get(OCCURRENCE_LANG) {
            Some(value) => language_constraint(value, OCCURRENCE_LANG)?,
            None => Constraint::Any,
        };
        let mut filter = PartitionFilter::unconstrained().with_language(language);

        // The same document pushdown the search relation does, for the same
        // reason and with a smaller saving: this cursor is already lazy per
        // partition, so a bound `?doc` without it walks each partition's posting
        // list for the term and discards every row but that subject's. Naming
        // the subject's own partitions skips those lists rather than reading
        // them.
        if let Some(subject) = args.get(OCCURRENCE_DOC) {
            filter = filter.restricted_to(self.index.partitions_holding_subject(subject));
        }

        let mut analyzed = analyze(text);
        if analyzed.len() > 1 {
            return Err(EvalError::function(format!(
                "the term at position {OCCURRENCE_TERM} is {text:?}, which analyzes to \
                 {count} terms ({analyzed:?}); this relation matches ONE term per invocation by \
                 contract, so a multi-term needle is written as one call per term joined on the \
                 document position",
                count = analyzed.len()
            )));
        }
        // A needle of pure punctuation names no term, which matches nothing.
        // That is the same answer the index side gives, and the two ends must
        // agree about what "no text" means.
        let term = analyzed.pop().unwrap_or_default();
        let partitions = if term.is_empty() {
            Vec::new()
        } else {
            partition_keys(&self.index)
                .into_iter()
                .filter(|key| filter.matches(key))
                .collect()
        };

        Ok(Box::new(OccurrenceCursor {
            index: Arc::clone(&self.index),
            term,
            needle: needle.clone(),
            partitions,
            partition_at: 0,
            postings: Vec::new(),
            posting_at: 0,
            position_at: 0,
            bound: bound_values(args),
            remaining: ceiling,
        }))
    }
}

/// The cursor [`TermOccurrenceRelation::open`] returns: a walk of one term's
/// postings, one partition at a time.
///
/// The postings of the partition being read are materialized; the rest are not,
/// so a ceiling of ten over a term with a million occurrences reads one
/// partition's list rather than all of them.
///
/// The same two properties the search cursor documents hold here: the equality
/// filter runs on **every** bound position before a row is emitted, and the
/// licence is decremented **only** on emitted rows.
#[derive(Debug)]
struct OccurrenceCursor {
    /// The index the postings are read from.
    index: Arc<TextIndex>,
    /// The analyzed term, empty when the needle named none.
    term: String,
    /// The needle, echoed verbatim into position 1 of every row.
    needle: TermValue,
    /// The admitted partitions, ascending.
    partitions: Vec<PartitionKey>,
    /// How far into `partitions` the cursor has read.
    partition_at: usize,
    /// The current partition's postings: `(document, positions)`.
    postings: Vec<(u32, Vec<u32>)>,
    /// How far into `postings` the cursor has read.
    posting_at: usize,
    /// How far into the current posting's positions the cursor has read.
    position_at: usize,
    /// The invocation's bound values by flattened position (`None` = free).
    bound: Vec<Option<TermValue>>,
    /// The rows this invocation may still emit under the engine's licence.
    remaining: Option<u64>,
}

impl OccurrenceCursor {
    /// The full row for one occurrence.
    fn build(&self, document: u32, position: u32) -> Result<PfRow, EvalError> {
        let held = self.index.document(document).ok_or_else(|| {
            EvalError::data(format!(
                "a posting named document {document}, which the index does not hold"
            ))
        })?;
        Ok(vec![
            held.subject().clone(),
            self.needle.clone(),
            language_term(held.language()),
            integer_term(position),
        ])
    }

    /// Read the next partition's postings into `postings`, or report that there
    /// is no next partition.
    fn load_next_partition(&mut self) -> bool {
        let Some(key) = self.partitions.get(self.partition_at).cloned() else {
            return false;
        };
        self.partition_at += 1;
        let loaded: Vec<(u32, Vec<u32>)> = self
            .index
            .postings(&key, &self.term)
            .map(|(document, positions)| (document, positions.to_vec()))
            .collect();
        self.postings = loaded;
        self.posting_at = 0;
        self.position_at = 0;
        true
    }
}

impl PfCursor for OccurrenceCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        loop {
            if self.remaining == Some(0) {
                return Ok(None);
            }
            let Some((document, positions)) = self.postings.get(self.posting_at) else {
                if self.load_next_partition() {
                    continue;
                }
                return Ok(None);
            };
            let Some(&position) = positions.get(self.position_at) else {
                self.posting_at += 1;
                self.position_at = 0;
                continue;
            };
            let document = *document;
            self.position_at += 1;

            let row = self.build(document, position)?;
            if agrees(&self.bound, &row) {
                if let Some(remaining) = self.remaining.as_mut() {
                    *remaining = remaining.saturating_sub(1);
                }
                return Ok(Some(row));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Index-versus-dataset binding
// ---------------------------------------------------------------------------

/// Check that `index` was built from `dataset` under `config`.
///
/// # The channel this closes
///
/// [`PropertyFunction::open`] receives no dataset — its signature is
/// `open(&self, args, ceiling)` — and while the engine validates *registry*
/// identity when a plan is prepared, nothing anywhere validates *dataset*
/// identity. An index paired with the wrong dataset is therefore a silent wrong
/// answer rather than a failure: the relation emits perfectly well-formed
/// document subjects, those subjects join back by basic graph pattern against a
/// dataset that never held them, zero rows come out, and no layer has anything
/// to report. Retrieval's failure mode is silence, so the mismatch has to be
/// found by asking rather than by noticing.
///
/// This recomputes the source digest over `dataset` and compares it to
/// [`TextIndex::source_fingerprint`], which is the digest of the
/// `(graph, subject, predicate, literal)` rows the index was actually built by
/// walking.
///
/// # When to call it
///
/// It is **O(corpus)**: it re-walks both RDF 1.2 layers for every configured
/// predicate. Run it once per `(index, dataset)` pairing — where the host wires
/// the registry — never per query and never per invocation.
///
/// # Errors
///
/// * [`TextError::Config`] if `config` is not the configuration `index` was
///   built under. Digesting `dataset` under a different configuration would
///   compare two different questions, so the mismatch is reported instead of
///   producing a verdict that means nothing.
/// * [`TextError::Data`] if `dataset` does not carry a configured predicate or
///   the configured named graph at all — which is itself a wrong-dataset
///   symptom — or if a term cannot be encoded.
/// * [`TextError::Data`] if the digests differ, naming both so a host can see
///   which pairing it made.
pub fn verify_binding<D: DatasetView>(
    index: &TextIndex,
    dataset: &D,
    config: &TextIndexConfig,
) -> Result<(), TextError> {
    if config != index.config() {
        return Err(TextError::config(
            "the configuration supplied to verify_binding is not the one this index was built \
             under; the two would digest different rows, so the comparison would answer a \
             different question than the one asked",
        ));
    }

    let expected = index.source_fingerprint();
    let actual = source_digest(dataset, config)?;
    if expected == actual {
        return Ok(());
    }
    Err(TextError::data(format!(
        "this index was built over a different dataset: its source digest is {expected:02x?} and \
         the supplied dataset digests to {actual:02x?}. Rebuild the index from the dataset the \
         query runs against, or pair the query with the dataset the index was built from — an \
         index joined to the wrong dataset returns no rows and reports nothing."
    )))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pretty_assertions::assert_eq;
    use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};

    use super::{
        OCCURRENCE_MODE, SEARCH_MODE, TermOccurrenceRelation, TextSearchRelation, XSD_DECIMAL,
        XSD_INTEGER, verify_binding,
    };
    use crate::error::TextError;
    use crate::index::{GraphSelector, TextIndex, TextIndexConfig};
    use purrdf_core::binding_pattern::BindingPattern;
    use purrdf_sparql_eval::{EvalError, PfArgs, PfRow, PropertyFunction};

    /// The one predicate every fixture indexes.
    const NOTE: &str = "https://example.org/note";

    // ── term helpers ────────────────────────────────────────────────────────

    fn iri(local: &str) -> TermValue {
        TermValue::iri(format!("https://example.org/{local}"))
    }

    fn string(value: &str) -> TermValue {
        TermValue::simple_literal(value)
    }

    fn integer(value: u32) -> TermValue {
        TermValue::typed_literal(value.to_string(), XSD_INTEGER)
    }

    fn decimal(value: &str) -> TermValue {
        TermValue::typed_literal(value, XSD_DECIMAL)
    }

    // ── fixtures ────────────────────────────────────────────────────────────

    /// A dataset of `(subject local name, text, language tag)` rows.
    fn dataset_of(rows: &[(&str, &str, Option<&str>)]) -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let note = builder.intern_iri(NOTE);
        for &(subject, text, language) in rows {
            let s = builder.intern_iri(&format!("https://example.org/{subject}"));
            let literal = builder.intern_literal(match language {
                Some(tag) => RdfLiteral::language_tagged(text, tag),
                None => RdfLiteral::simple(text),
            });
            builder.push_quad(s, note, literal, None);
        }
        builder.freeze().expect("the fixture must validate")
    }

    /// The configuration every fixture is built under.
    fn config() -> TextIndexConfig {
        TextIndexConfig::new(vec![TermValue::iri(NOTE)], GraphSelector::Any)
            .expect("the fixture configuration is well formed")
    }

    fn index_of(rows: &[(&str, &str, Option<&str>)]) -> TextIndex {
        TextIndex::from_dataset(&*dataset_of(rows), &config()).expect("the fixture index builds")
    }

    /// The hand-computed golden of this crate's scoring suite: four untagged
    /// documents of four tokens each, so `avgdl` is exactly four and every
    /// document's length normalization is exactly one. One partition.
    ///
    /// The needle `"alpha beta"` matches `ex:a` (rank 1) and `ex:b` (rank 2)
    /// and nothing else; the two scores are pinned digit for digit in
    /// `tests/scoring.rs` and repeated here as expected row values.
    fn golden() -> Arc<TextIndex> {
        Arc::new(index_of(&[
            ("a", "alpha alpha beta gamma", None),
            ("b", "alpha beta gamma delta", None),
            ("c", "epsilon zeta eta theta", None),
            ("d", "iota kappa lambda mu", None),
        ]))
    }

    /// Five documents over three partitions — English, French and untagged —
    /// all in the default graph, so the index spans exactly one graph.
    ///
    /// The needle `"alpha"` matches every document, which is what makes this
    /// the fixture the row-bound sweep runs against: every mode has rows to
    /// sample bound values from.
    fn mixed() -> Arc<TextIndex> {
        Arc::new(index_of(&[
            ("a", "alpha beta", Some("en")),
            ("b", "alpha", Some("en")),
            ("c", "alpha beta", Some("fr")),
            ("d", "alpha", None),
            ("e", "alpha", Some("en")),
        ]))
    }

    /// A subject that carries text in **three** partitions, plus two subjects
    /// that carry text in one each.
    ///
    /// Every other fixture here gives each subject exactly one document, which
    /// makes a bound-document invocation trivially a one-row answer and leaves
    /// the interesting case — the one the `?doc`-only row bound is *stated in
    /// terms of* — never exercised. `ex:a` here holds English, French and
    /// untagged text, so binding it emits one row per partition and the declared
    /// bound is attained rather than merely respected.
    ///
    /// It is also the fixture that keeps the bound-document partition pushdown
    /// honest: a pushdown that restricted to one of `ex:a`'s partitions instead
    /// of all three would drop two rows, and every remaining row would still
    /// look perfectly correct.
    fn spread_subject() -> Arc<TextIndex> {
        Arc::new(index_of(&[
            ("a", "alpha beta", Some("en")),
            ("a", "alpha gamma", Some("fr")),
            ("a", "alpha delta", None),
            ("b", "alpha epsilon", Some("en")),
            ("c", "alpha zeta", None),
        ]))
    }

    /// Three documents over two partitions, with `alpha` occurring twice in two
    /// of them so a position sweep has more than one row per document.
    fn occurrences() -> Arc<TextIndex> {
        Arc::new(index_of(&[
            ("a", "alpha beta alpha", None),
            ("b", "alpha", Some("en")),
            ("c", "alpha alpha", Some("en")),
        ]))
    }

    // ── invocation helpers ──────────────────────────────────────────────────

    /// Open `relation` with the given per-position bindings and drain it.
    fn invoke<R: PropertyFunction>(
        relation: &R,
        bound: &[Option<TermValue>],
        ceiling: Option<u64>,
    ) -> Result<Vec<PfRow>, EvalError> {
        let refs: Vec<Option<&TermValue>> = bound.iter().map(Option::as_ref).collect();
        let (subject, object) = refs.split_at(relation.arity().subject);
        let args = PfArgs::new(subject, object);
        let mut cursor = relation.open(&args, ceiling)?;
        let mut rows = Vec::new();
        while let Some(row) = cursor.next()? {
            rows.push(row);
        }
        Ok(rows)
    }

    /// The all-free-but-the-needle binding vector for a search invocation.
    fn search_args(needle: &str) -> Vec<Option<TermValue>> {
        vec![None, Some(string(needle)), None, None, None, None]
    }

    /// The all-free-but-the-term binding vector for an occurrence invocation.
    fn occurrence_args(term: &str) -> Vec<Option<TermValue>> {
        vec![None, Some(string(term)), None, None]
    }

    /// The bindings of `mode`, taking each bound position's value from `row`.
    fn bindings_from(mode: BindingPattern, row: &PfRow, needle: usize) -> Vec<Option<TermValue>> {
        (0..row.len())
            .map(|at| {
                if at == needle || mode.is_bound(at) {
                    Some(row[at].clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Every binding pattern of `arity`, as the sweep tests enumerate them.
    fn every_pattern(arity: usize) -> Vec<BindingPattern> {
        (0..(1_usize << arity))
            .map(|bits| BindingPattern::from_bools((0..arity).map(|at| bits & (1 << at) != 0)))
            .collect()
    }

    // ── the declared shape ──────────────────────────────────────────────────

    #[test]
    fn the_declared_shape_is_the_documented_one() {
        let search = TextSearchRelation::new(golden());
        assert_eq!(search.arity().subject, 1);
        assert_eq!(search.arity().object, 5);
        assert_eq!(
            search
                .modes()
                .iter()
                .map(|mode| mode.code())
                .collect::<Vec<_>>(),
            vec![SEARCH_MODE.to_owned()],
            "the needle is the only input, so there is exactly one mode"
        );

        let occurrence = TermOccurrenceRelation::new(occurrences());
        assert_eq!(occurrence.arity().subject, 1);
        assert_eq!(occurrence.arity().object, 3);
        assert_eq!(
            occurrence
                .modes()
                .iter()
                .map(|mode| mode.code())
                .collect::<Vec<_>>(),
            vec![OCCURRENCE_MODE.to_owned()]
        );
    }

    /// Both relations are stable, which is what lets them run across fork-join
    /// workers: a frozen index plus exact fixed-point arithmetic is a pure
    /// function of the invocation.
    #[test]
    fn both_relations_are_stable() {
        use purrdf_sparql_eval::Volatility;
        assert_eq!(
            TextSearchRelation::new(golden()).volatility(),
            Volatility::Stable
        );
        assert_eq!(
            TermOccurrenceRelation::new(occurrences()).volatility(),
            Volatility::Stable
        );
    }

    // ── ranked retrieval: the rows ──────────────────────────────────────────

    /// The exact rows, term for term, against the hand-computed golden.
    #[test]
    fn search_emits_rank_order_with_exact_rows() {
        let relation = TextSearchRelation::new(golden());
        let rows = invoke(&relation, &search_args("alpha beta"), None).expect("a bound needle");
        assert_eq!(
            rows,
            vec![
                vec![
                    iri("a"),
                    string("alpha beta"),
                    decimal("1.646224553827"),
                    integer(1),
                    string(""),
                    integer(2),
                ],
                vec![
                    iri("b"),
                    string("alpha beta"),
                    decimal("1.386294361118"),
                    integer(2),
                    string(""),
                    integer(2),
                ],
            ],
            "two documents hold a needle term; ex:c and ex:d hold none and are not rows"
        );
    }

    /// An untagged document reports the empty string, and a tagged one reports
    /// its tag — so the two are distinguishable in the same answer.
    #[test]
    fn untagged_documents_emit_the_empty_language() {
        let index = Arc::new(index_of(&[
            ("a", "alpha", None),
            ("b", "alpha", Some("en")),
        ]));
        let relation = TextSearchRelation::new(index);
        let rows = invoke(&relation, &search_args("alpha"), None).expect("a bound needle");
        assert_eq!(rows.len(), 2);
        assert_eq!(
            (rows[0][0].clone(), rows[0][4].clone()),
            (iri("a"), string("")),
            "the untagged partition sorts first and reports the empty language"
        );
        assert_eq!(
            (rows[1][0].clone(), rows[1][4].clone()),
            (iri("b"), string("en"))
        );

        // And the empty string reads back as the untagged partition rather than
        // as a tag, which is the round trip the documentation promises.
        let mut untagged = search_args("alpha");
        untagged[4] = Some(string(""));
        let rows = invoke(&relation, &untagged, None).expect("a bound needle");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], iri("a"));
    }

    // ── ranked retrieval: refusals and honest empties ───────────────────────

    /// A free needle is refused rather than answered with nothing: this
    /// relation retrieves documents for a needle and cannot run the other way.
    #[test]
    fn a_free_needle_is_an_error() {
        let relation = TextSearchRelation::new(golden());
        let error = invoke(&relation, &[None, None, None, None, None, None], None)
            .expect_err("a free needle is not a mode this relation serves");
        assert!(matches!(error, EvalError::Function(_)), "got {error:?}");
        assert!(
            error.to_string().contains("position 1"),
            "the message must name the position: {error}"
        );

        let occurrence = TermOccurrenceRelation::new(occurrences());
        let error = invoke(&occurrence, &[None, None, None, None], None)
            .expect_err("a free term is not a mode this relation serves");
        assert!(
            error.to_string().contains("position 1"),
            "the message must name the position: {error}"
        );
    }

    /// A needle that is not a string names no text, and says which term it was
    /// handed rather than returning an empty bag.
    #[test]
    fn a_non_string_needle_is_an_error() {
        let search = TextSearchRelation::new(golden());
        let occurrence = TermOccurrenceRelation::new(occurrences());
        for needle in [iri("a"), integer(3), TermValue::blank("b0")] {
            let mut bound = search_args("unused");
            bound[1] = Some(needle.clone());
            let error =
                invoke(&search, &bound, None).expect_err("only a string literal carries text");
            assert!(matches!(error, EvalError::Function(_)), "got {error:?}");

            let mut bound = occurrence_args("unused");
            bound[1] = Some(needle);
            let error =
                invoke(&occurrence, &bound, None).expect_err("only a string literal carries text");
            assert!(matches!(error, EvalError::Function(_)), "got {error:?}");
        }

        // A language-tagged string IS text, and is accepted.
        let mut bound = search_args("unused");
        bound[1] = Some(TermValue::lang_literal("alpha", "en"));
        let rows = invoke(&search, &bound, None).expect("an rdf:langString is a needle");
        assert_eq!(rows.len(), 2, "the golden holds `alpha` in two documents");
    }

    /// A needle of pure punctuation analyzes to no terms. That is a well-formed
    /// request naming nothing, so it is an empty answer — the same answer the
    /// index side gives when a document's literals analyze to nothing.
    #[test]
    fn a_zero_term_needle_is_an_honest_empty_result() {
        let search = TextSearchRelation::new(golden());
        assert!(
            invoke(&search, &search_args("---"), None)
                .expect("a punctuation needle is well formed")
                .is_empty()
        );

        let occurrence = TermOccurrenceRelation::new(occurrences());
        assert!(
            invoke(&occurrence, &occurrence_args("---"), None)
                .expect("a punctuation term is well formed")
                .is_empty()
        );
    }

    /// Ranks are 1-based, so zero and every negative value name no row that
    /// could ever exist — a domain violation rather than an empty answer within
    /// the domain.
    #[test]
    fn a_rank_of_zero_is_an_error() {
        let relation = TextSearchRelation::new(golden());
        // Zero written every way `xsd:integer`'s lexical space permits, and a
        // negative at every magnitude — including one far past `i128`, which
        // must stay a domain error rather than becoming an empty answer.
        for lexical in [
            "0",
            "-0",
            "+0",
            "000",
            "-1",
            "-99",
            "-10000000000000000000000000000000000000000000",
        ] {
            let mut bound = search_args("alpha");
            bound[3] = Some(TermValue::typed_literal(lexical, XSD_INTEGER));
            let error = invoke(&relation, &bound, None).expect_err("ranks start at one");
            assert!(matches!(error, EvalError::Function(_)), "got {error:?}");
            assert!(error.to_string().contains("1-based"), "got {error}");
        }

        // A non-integer at `?rank` is refused too, and says what it found.
        for value in [string("1"), iri("a"), decimal("1.0")] {
            let mut bound = search_args("alpha");
            bound[3] = Some(value);
            let error = invoke(&relation, &bound, None).expect_err("only an xsd:integer is a rank");
            assert!(matches!(error, EvalError::Function(_)), "got {error:?}");
        }

        // So is a lexical form outside `xsd:integer`'s lexical space even when
        // the datatype claims otherwise.
        for lexical in ["", "+", "-", " 1", "1 ", "1.0", "1e3", "0x10", "one", "١"] {
            let mut bound = search_args("alpha");
            bound[3] = Some(TermValue::typed_literal(lexical, XSD_INTEGER));
            let error = invoke(&relation, &bound, None).expect_err(&format!(
                "{lexical:?} is not in xsd:integer's lexical space and must be refused"
            ));
            assert!(matches!(error, EvalError::Function(_)), "got {error:?}");
            assert!(
                error.to_string().contains("lexical space"),
                "a malformed lexical form must be named as one: {error}"
            );
        }
    }

    /// The neighbouring case the refusal above must NOT swallow: a rank that is
    /// a perfectly well-formed `xsd:integer` and merely enormous.
    ///
    /// `xsd:integer`'s value space is unbounded, so `10^44` is as valid a rank
    /// as `4294967296` — it simply names a position no index numbers, which the
    /// relation already answers emptily. Deciding this by whether the lexical
    /// form fits an `i128` put the boundary between "empty answer" and "aborted
    /// query" at `2^127`, a number with nothing to do with `xsd:integer`, and
    /// aborted a whole SPARQL evaluation over a value that should have
    /// contributed no rows.
    ///
    /// Leading zeros and a leading `+` are lexical noise in the same production,
    /// so they are exercised here too: `"+002"` denotes the same rank as `"2"`
    /// and must return the same row rather than being refused as malformed.
    #[test]
    fn a_huge_but_well_formed_rank_is_empty_not_an_error() {
        let relation = TextSearchRelation::new(golden());
        for lexical in [
            // One past `i128::MAX`, where the old parse gave up.
            "170141183460469231731687303715884105728",
            "99999999999999999999999999999999999999999999",
            "+99999999999999999999999999999999999999999999",
            // Leading zeros do not change the magnitude, so a padded rank that
            // is past the end is past the end.
            "0000000000000000000000004294967296",
        ] {
            let mut bound = search_args("alpha beta");
            bound[3] = Some(TermValue::typed_literal(lexical, XSD_INTEGER));
            assert!(
                invoke(&relation, &bound, None)
                    .unwrap_or_else(|error| panic!(
                        "rank {lexical} is a valid xsd:integer and must not fail: {error}"
                    ))
                    .is_empty(),
                "rank {lexical} is past the end of the only partition"
            );
        }

        // The neighbouring in-range rank still selects its row, in the canonical
        // lexical form the relation emits.
        let mut bound = search_args("alpha beta");
        bound[3] = Some(integer(2));
        let expected = invoke(&relation, &bound, None).expect("rank two exists");
        assert_eq!(expected.len(), 1, "the fixture must have a rank-two row");

        // A NON-canonical spelling of the same in-range rank yields nothing, and
        // that is correct rather than an over-refusal. The row this relation
        // emits carries the canonical `"2"^^xsd:integer` at position 3, and the
        // seam joins a bound position by RDF **term** identity — the same
        // comparison a basic graph pattern makes — so `"+2"^^xsd:integer` and
        // `"002"^^xsd:integer` are different terms and match no emitted row. The
        // alternative would be for the relation to emit a row whose `?rank` cell
        // is not the value the caller bound, which is a fabricated binding.
        //
        // What the lexical tolerance in `xsd_integer_parts` buys is the
        // classification above it: `"+0"` and `"000"` reach the 1-based domain
        // error rather than the malformed one, and a huge padded value reaches
        // the empty answer rather than an abort.
        for lexical in ["+2", "002", "+0000002"] {
            let mut bound = search_args("alpha beta");
            bound[3] = Some(TermValue::typed_literal(lexical, XSD_INTEGER));
            assert!(
                invoke(&relation, &bound, None)
                    .unwrap_or_else(|error| panic!(
                        "rank {lexical:?} is a valid xsd:integer and must not fail: {error}"
                    ))
                    .is_empty(),
                "{lexical:?} is not the term this relation emits at position 3, so it joins nothing"
            );
        }
    }

    /// Asking for a rank past the end of every partition is a question with an
    /// answer, so it is empty rather than refused — including a rank past the
    /// whole `u32` position space an index numbers.
    #[test]
    fn a_rank_past_the_end_is_empty_not_an_error() {
        let relation = TextSearchRelation::new(golden());
        for lexical in [
            "3",
            "99",
            "4294967296",
            "170141183460469231731687303715884105727",
        ] {
            let mut bound = search_args("alpha beta");
            bound[3] = Some(TermValue::typed_literal(lexical, XSD_INTEGER));
            assert!(
                invoke(&relation, &bound, None)
                    .unwrap_or_else(|error| panic!("rank {lexical} must not fail: {error}"))
                    .is_empty(),
                "rank {lexical} is past the end of the only partition"
            );
        }

        // Rank two is inside it, and is the second-ranked row.
        let mut bound = search_args("alpha beta");
        bound[3] = Some(integer(2));
        let rows = invoke(&relation, &bound, None).expect("rank two exists");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], iri("b"));
    }

    /// A `?lang` bound to something that is not an `xsd:string` is refused.
    #[test]
    fn a_non_string_language_is_an_error() {
        let search = TextSearchRelation::new(mixed());
        let occurrence = TermOccurrenceRelation::new(occurrences());
        for value in [integer(1), iri("en"), TermValue::lang_literal("en", "en")] {
            let mut bound = search_args("alpha");
            bound[4] = Some(value.clone());
            let error =
                invoke(&search, &bound, None).expect_err("only an xsd:string names a language");
            assert!(matches!(error, EvalError::Function(_)), "got {error:?}");

            let mut bound = occurrence_args("alpha");
            bound[2] = Some(value);
            let error =
                invoke(&occurrence, &bound, None).expect_err("only an xsd:string names a language");
            assert!(matches!(error, EvalError::Function(_)), "got {error:?}");
        }
    }

    // ── the ceiling ─────────────────────────────────────────────────────────

    /// A ceiling of `k` yields exactly the first `k` rows of the unbounded
    /// answer, for every `k` — the prefix property the licence is granted
    /// against.
    #[test]
    fn a_ceiling_yields_the_prefix_of_the_unbounded_answer() {
        for index in [golden(), mixed()] {
            let search = TextSearchRelation::new(Arc::clone(&index));
            let full = invoke(&search, &search_args("alpha"), None).expect("a bound needle");
            for k in 0..=(full.len() as u64 + 2) {
                let capped =
                    invoke(&search, &search_args("alpha"), Some(k)).expect("a bound needle");
                let want = full.len().min(k as usize);
                assert_eq!(
                    capped,
                    full[..want].to_vec(),
                    "a ceiling of {k} must be the first {want} rows of the unbounded answer"
                );
            }

            let occurrence = TermOccurrenceRelation::new(index);
            let full = invoke(&occurrence, &occurrence_args("alpha"), None).expect("a bound term");
            for k in 0..=(full.len() as u64 + 2) {
                let capped =
                    invoke(&occurrence, &occurrence_args("alpha"), Some(k)).expect("a bound term");
                let want = full.len().min(k as usize);
                assert_eq!(capped, full[..want].to_vec());
            }
        }
    }

    /// The regression test for the ceiling bug this seam has already been got
    /// wrong once.
    ///
    /// `?doc`, `?score` and `?matched` are filtered **after** ranking, and a
    /// constant at any of them still makes the call admission-transparent, so
    /// the engine still offers a ceiling. A relation that passed that ceiling
    /// straight through to the ranker would rank only the first `k` documents,
    /// find that none of them is the one asked for, emit nothing, and report
    /// exhaustion — a short bag the engine reads as a complete answer.
    ///
    /// Here `ex:b` ranks second and the ceiling is one, so the naive
    /// implementation returns no rows. The correct one returns `ex:b`'s row.
    #[test]
    fn a_ceiling_with_a_bound_doc_still_emits_the_matching_row() {
        let relation = TextSearchRelation::new(golden());
        let full = invoke(&relation, &search_args("alpha beta"), None).expect("a bound needle");
        assert_eq!(
            full.len(),
            2,
            "the fixture must have a row beyond the ceiling"
        );
        let second = full[1].clone();
        assert_eq!(second[3], integer(2), "ex:b is the rank-two row");

        // Bound `?doc` at position 0.
        let mut bound = search_args("alpha beta");
        bound[0] = Some(second[0].clone());
        assert_eq!(
            invoke(&relation, &bound, Some(1)).expect("a bound needle"),
            vec![second.clone()],
            "a rank-two row bound at ?doc must survive a ceiling of one"
        );

        // Bound `?score` at position 2 — the same hazard, a different position.
        let mut bound = search_args("alpha beta");
        bound[2] = Some(second[2].clone());
        assert_eq!(
            invoke(&relation, &bound, Some(1)).expect("a bound needle"),
            vec![second.clone()]
        );

        // Bound `?matched` at position 5 is post-rank too. Both rows match on
        // it here, so a ceiling of one must yield the FIRST of them — the
        // prefix property, unchanged by the withheld ranker ceiling.
        let mut bound = search_args("alpha beta");
        bound[5] = Some(second[5].clone());
        assert_eq!(
            invoke(&relation, &bound, Some(1)).expect("a bound needle"),
            vec![full[0].clone()]
        );
    }

    /// The licence is spent on emitted rows, never on skipped ones: a ceiling
    /// of one over an invocation whose first candidate is filtered out still
    /// yields one row.
    #[test]
    fn the_ceiling_counts_emitted_rows_not_skipped_ones() {
        let relation = TextSearchRelation::new(golden());
        let full = invoke(&relation, &search_args("alpha beta"), None).expect("a bound needle");
        let mut bound = search_args("alpha beta");
        bound[0] = Some(full[1][0].clone());
        let rows = invoke(&relation, &bound, Some(1)).expect("a bound needle");
        assert_eq!(
            rows.len(),
            1,
            "the skipped rank-one row must not spend the licence"
        );
    }

    // ── the row bounds ──────────────────────────────────────────────────────

    /// The declared bounds of the search relation, pinned against the `mixed`
    /// fixture: five documents, three partitions, one graph, three languages of
    /// one partition each, and English holding three of the five documents.
    ///
    /// The table is a real function of the mode — the documented rows differ
    /// where they should — and positions 2 (`?score`) and 5 (`?matched`) bind
    /// nothing, because many documents can share either value.
    #[test]
    fn the_search_row_bound_table_is_the_documented_one() {
        let relation = TextSearchRelation::new(mixed());
        let declared = |code: &str| relation.rows_per_invocation(BindingPattern::from_code(code));

        assert_eq!(declared("fbffff"), 5, "nothing bound: one row per document");
        assert_eq!(declared("bbffff"), 3, "?doc: at most one row per partition");
        assert_eq!(declared("fbfbff"), 3, "?rank: one row per partition");
        assert_eq!(
            declared("fbffbf"),
            3,
            "?lang: English holds three documents"
        );
        assert_eq!(
            declared("fbfbbf"),
            1,
            "?rank and ?lang: one partition, one rank"
        );
        assert_eq!(declared("bbffbf"), 1, "?doc and ?lang: one graph");
        assert_eq!(declared("bbfbbf"), 1, "all three together are no looser");

        assert_eq!(
            declared("fbbfff"),
            5,
            "?score binds nothing: many documents can share a score"
        );
        assert_eq!(
            declared("fbfffb"),
            5,
            "?matched binds nothing: many documents can share a matched-term count"
        );
        assert_eq!(
            declared("fbbffb"),
            5,
            "and the two together still bind nothing"
        );
    }

    /// The same, for the occurrence relation over the `occurrences` fixture:
    /// `alpha` occurs five times in three documents over two partitions and one
    /// graph, at most twice in any one document, and at most three times within
    /// any one language.
    #[test]
    fn the_occurrence_row_bound_table_is_the_documented_one() {
        let relation = TermOccurrenceRelation::new(occurrences());
        let declared = |code: &str| relation.rows_per_invocation(BindingPattern::from_code(code));

        assert_eq!(
            declared("fbff"),
            5,
            "nothing bound: total OCCURRENCES, not documents"
        );
        assert_eq!(
            declared("fbfb"),
            3,
            "?position: one occurrence per document"
        );
        assert_eq!(
            declared("fbbf"),
            3,
            "?lang: English holds three occurrences"
        );
        assert_eq!(
            declared("bbff"),
            4,
            "?doc: partitions times the per-document maximum"
        );
        assert_eq!(
            declared("bbbf"),
            2,
            "?doc and ?lang: graphs times the per-document maximum"
        );
        assert_eq!(declared("bbfb"), 2, "?doc and ?position: one per partition");
        assert_eq!(declared("bbbb"), 1, "all three: one graph, one occurrence");
    }

    /// The bound-document partition pushdown must be a work reduction and
    /// nothing else: the rows it produces are exactly the rows the unrestricted
    /// walk produced, filtered on the bound subject.
    ///
    /// This is the test the pushdown could quietly fail. Restricting to the
    /// partitions a subject appears in is only correct if *every* such partition
    /// is named; naming one of `ex:a`'s three would return a plausible,
    /// correctly-ranked, correctly-scored one-row answer that is missing two
    /// rows, and nothing in the row values would say so. So the answer is
    /// compared against the whole unbound answer filtered by hand — the
    /// definition of what the bound call means — rather than against a
    /// hand-written expectation that could be written to match the bug.
    #[test]
    fn a_bound_document_pushdown_loses_no_row() {
        let index = spread_subject();
        let search = TextSearchRelation::new(Arc::clone(&index));
        let full = invoke(&search, &search_args("alpha"), None).expect("a bound needle");
        assert_eq!(full.len(), 5, "every document holds `alpha`");

        for subject in ["a", "b", "c"] {
            let mut bound = search_args("alpha");
            bound[0] = Some(iri(subject));
            let expected: Vec<PfRow> = full
                .iter()
                .filter(|row| row[0] == iri(subject))
                .cloned()
                .collect();
            assert_eq!(
                invoke(&search, &bound, None).expect("a bound needle"),
                expected,
                "binding ?doc to ex:{subject} must yield exactly its rows of the unbound answer"
            );
        }
        // ex:a is the whole point: three partitions, three rows.
        assert_eq!(
            full.iter().filter(|row| row[0] == iri("a")).count(),
            3,
            "the fixture must put one subject in three partitions, or this proves nothing"
        );

        // A subject the index holds no text for admits no partition, which is an
        // empty answer rather than a refusal or an unfiltered one.
        let mut bound = search_args("alpha");
        bound[0] = Some(iri("nowhere"));
        assert!(
            invoke(&search, &bound, None)
                .expect("an absent subject is a well-formed request")
                .is_empty()
        );

        // The occurrence relation pushes the same restriction down, so it gets
        // the same treatment.
        let occurrence = TermOccurrenceRelation::new(index);
        let full = invoke(&occurrence, &occurrence_args("alpha"), None).expect("a bound term");
        for subject in ["a", "b", "c"] {
            let mut bound = occurrence_args("alpha");
            bound[0] = Some(iri(subject));
            let expected: Vec<PfRow> = full
                .iter()
                .filter(|row| row[0] == iri(subject))
                .cloned()
                .collect();
            assert_eq!(
                invoke(&occurrence, &bound, None).expect("a bound term"),
                expected
            );
        }
        let mut bound = occurrence_args("alpha");
        bound[0] = Some(iri("nowhere"));
        assert!(
            invoke(&occurrence, &bound, None)
                .expect("an absent subject is a well-formed request")
                .is_empty()
        );
    }

    /// The `?doc`-only row bound, attained rather than merely respected.
    ///
    /// `the_search_row_bound_table_is_the_documented_one` pins the number the
    /// table declares, and the sweep proves no invocation exceeds it — but over
    /// `mixed()`, where every subject sits in exactly one partition, no
    /// invocation gets anywhere near it either. A declared bound of three that
    /// nothing can reach is indistinguishable from a declared bound of three
    /// hundred, so the claim is checked here against a fixture where a subject
    /// really does span every partition.
    #[test]
    fn the_bound_document_row_bound_is_attained_not_merely_respected() {
        let index = spread_subject();
        let search = TextSearchRelation::new(Arc::clone(&index));
        let declared = search.rows_per_invocation(BindingPattern::from_code("bbffff"));
        assert_eq!(
            declared, 3,
            "three partitions, and a subject occupies at most one document in each"
        );

        let mut bound = search_args("alpha");
        bound[0] = Some(iri("a"));
        assert_eq!(
            invoke(&search, &bound, None).expect("a bound needle").len() as u64,
            declared,
            "a subject spanning every partition must attain the declared bound exactly"
        );

        // The same for the occurrence relation, whose `?doc` bound is
        // partitions × the per-document maximum. `alpha` occurs once per
        // document here, so three partitions attain three.
        let occurrence = TermOccurrenceRelation::new(index);
        let declared = occurrence.rows_per_invocation(BindingPattern::from_code("bbff"));
        assert_eq!(
            declared, 3,
            "three partitions × one occurrence per document"
        );
        let mut bound = occurrence_args("alpha");
        bound[0] = Some(iri("a"));
        assert_eq!(
            invoke(&occurrence, &bound, None)
                .expect("a bound term")
                .len() as u64,
            declared
        );
    }

    /// Sweep **every** binding pattern of both relations and hold the declared
    /// bound to two properties.
    ///
    /// * **(a) It is an upper bound that is actually respected.** For every
    ///   admitted mode, every invocation the fixture can produce emits at most
    ///   the declared number of rows. A bound that under-states reality turns
    ///   the planner's admission decision into a wrong one, which is what the
    ///   seam's honesty contract forbids.
    /// * **(b) Where the table claims exactly one, it IS exactly one.** An
    ///   assertion that only checks `<=` is satisfied by declaring [`u64::MAX`]
    ///   and therefore says nothing, so every mode declaring `1` must also have
    ///   an invocation that emits `1`. Attainment, not merely non-violation.
    ///
    /// Bound values are drawn from the rows the unbounded answer actually
    /// produced, which is the only place a mode's rows can come from: a value
    /// appearing in no row yields no row and cannot exceed anything.
    ///
    /// The patterns leaving the needle free are not admitted at all — the
    /// single declared mode does not subsume them — and the sweep asserts that
    /// rather than quietly skipping them.
    #[test]
    fn rows_per_invocation_is_a_real_function_of_the_mode() {
        let search = TextSearchRelation::new(mixed());
        check_bounds(&search, 6, 1, &["alpha", "alpha beta", "gamma"]);

        // And over the fixture where one subject spans every partition, which
        // is where a `?doc`-bound mode can actually approach its bound.
        let spread = TextSearchRelation::new(spread_subject());
        check_bounds(&spread, 6, 1, &["alpha", "alpha beta", "gamma"]);

        let occurrence = TermOccurrenceRelation::new(occurrences());
        check_bounds(&occurrence, 4, 1, &["alpha", "beta", "gamma"]);

        let spread = TermOccurrenceRelation::new(spread_subject());
        check_bounds(&spread, 4, 1, &["alpha", "beta", "gamma"]);
    }

    /// Properties (a) and (b) of the sweep above, for one relation.
    fn check_bounds<R: PropertyFunction>(
        relation: &R,
        arity: usize,
        needle: usize,
        needles: &[&str],
    ) {
        for mode in every_pattern(arity) {
            let declared = relation.rows_per_invocation(mode);
            if !relation.admits(mode) {
                assert!(
                    !mode.is_bound(needle),
                    "the only reason to refuse mode {} is a free needle",
                    mode.code()
                );
                continue;
            }

            let mut attained = 0_u64;
            for text in needles {
                let free: Vec<Option<TermValue>> = (0..arity)
                    .map(|at| (at == needle).then(|| string(text)))
                    .collect();
                let full = invoke(relation, &free, None).expect("a bound needle");
                for row in &full {
                    let bound = bindings_from(mode, row, needle);
                    let emitted = invoke(relation, &bound, None)
                        .expect("a bound needle")
                        .len() as u64;
                    assert!(
                        emitted <= declared,
                        "mode {} declares {declared} row(s) but emitted {emitted} for {bound:?}",
                        mode.code()
                    );
                    attained = attained.max(emitted);
                }
            }

            assert!(
                declared != 1 || attained == 1,
                "mode {} declares exactly one row, so one invocation must actually emit one — a \
                 bound nothing attains is not a bound, it is a guess",
                mode.code()
            );
        }
    }

    // ── positional matching ─────────────────────────────────────────────────

    /// One row per occurrence, ascending by position within a document and by
    /// document within a partition — the order a phrase filter reads.
    #[test]
    fn occurrence_emits_one_row_per_position_in_ascending_order() {
        let relation = TermOccurrenceRelation::new(occurrences());
        let rows = invoke(&relation, &occurrence_args("alpha"), None).expect("a bound term");
        assert_eq!(
            rows,
            vec![
                vec![iri("a"), string("alpha"), string(""), integer(0)],
                vec![iri("a"), string("alpha"), string(""), integer(2)],
                vec![iri("b"), string("alpha"), string("en"), integer(0)],
                vec![iri("c"), string("alpha"), string("en"), integer(0)],
                vec![iri("c"), string("alpha"), string("en"), integer(1)],
            ],
            "ex:a holds `alpha` at 0 and 2 with `beta` between them; the untagged partition \
             sorts before the English one"
        );

        // The adjacency the type's documentation is written around: `beta`
        // follows the first `alpha` and precedes the second.
        assert_eq!(
            invoke(&relation, &occurrence_args("beta"), None).expect("a bound term"),
            vec![vec![iri("a"), string("beta"), string(""), integer(1)]]
        );
    }

    /// This relation matches ONE term per invocation by contract, so a needle
    /// that analyzes to two terms is refused rather than silently answered
    /// about one of them.
    #[test]
    fn occurrence_rejects_a_multi_term_needle() {
        let relation = TermOccurrenceRelation::new(occurrences());
        let error = invoke(&relation, &occurrence_args("alpha beta"), None)
            .expect_err("two terms are two calls");
        assert!(matches!(error, EvalError::Function(_)), "got {error:?}");
        assert!(
            error.to_string().contains("ONE term per invocation"),
            "the message must say what the contract is: {error}"
        );
    }

    /// A bound `?position` is a plain equality filter, which is exactly what
    /// the engine would apply itself — so a position no occurrence holds is an
    /// empty answer rather than a refusal.
    #[test]
    fn a_bound_position_selects_one_occurrence() {
        let relation = TermOccurrenceRelation::new(occurrences());
        let mut bound = occurrence_args("alpha");
        bound[3] = Some(integer(2));
        assert_eq!(
            invoke(&relation, &bound, None).expect("a bound term"),
            vec![vec![iri("a"), string("alpha"), string(""), integer(2)]]
        );

        let mut bound = occurrence_args("alpha");
        bound[3] = Some(integer(99));
        assert!(
            invoke(&relation, &bound, None)
                .expect("a bound term")
                .is_empty()
        );
    }

    // ── index-versus-dataset binding ────────────────────────────────────────

    /// The digest check catches the one pairing nothing else can: an index over
    /// data the query does not run against, which otherwise joins back to zero
    /// rows with no diagnostic anywhere.
    #[test]
    fn verify_binding_accepts_the_source_dataset_and_rejects_another() {
        let rows = [("a", "alpha beta", None), ("b", "gamma", None)];
        let source = dataset_of(&rows);
        let index = TextIndex::from_dataset(&*source, &config()).expect("the fixture builds");

        verify_binding(&index, &*source, &config()).expect("the index was built from this dataset");

        // One literal changed is a different corpus, and is refused.
        let other = dataset_of(&[("a", "alpha beta", None), ("b", "delta", None)]);
        let error = verify_binding(&index, &*other, &config())
            .expect_err("a different corpus is a different digest");
        assert!(matches!(error, TextError::Data(_)), "got {error:?}");
        assert!(
            error.to_string().contains("different dataset"),
            "the message must be actionable: {error}"
        );

        // So is one extra subject, even though every original row survives.
        let extended = dataset_of(&[
            ("a", "alpha beta", None),
            ("b", "gamma", None),
            ("c", "epsilon", None),
        ]);
        assert!(verify_binding(&index, &*extended, &config()).is_err());

        // A configuration that is not the one the index was built under would
        // digest different rows, so the comparison is refused rather than
        // answered with a verdict that means nothing.
        let narrower = TextIndexConfig::new(vec![TermValue::iri(NOTE)], GraphSelector::Default)
            .expect("a well-formed configuration");
        let error = verify_binding(&index, &*source, &narrower)
            .expect_err("a different configuration asks a different question");
        assert!(matches!(error, TextError::Config(_)), "got {error:?}");
    }

    // ── determinism ─────────────────────────────────────────────────────────

    /// The emission order is a pure function of the invocation, which is what
    /// the seam requires and what makes a query's answer reproducible. A
    /// hundred repeats of the same call, byte for byte the same rows.
    #[test]
    fn emission_order_is_a_pure_function_of_the_invocation() {
        let search = TextSearchRelation::new(mixed());
        let first = invoke(&search, &search_args("alpha"), None).expect("a bound needle");
        assert!(
            first.len() > 1,
            "the fixture must have an order to preserve"
        );
        for _ in 0..100 {
            assert_eq!(
                invoke(&search, &search_args("alpha"), None).expect("a bound needle"),
                first
            );
        }

        let occurrence = TermOccurrenceRelation::new(occurrences());
        let first = invoke(&occurrence, &occurrence_args("alpha"), None).expect("a bound term");
        assert!(first.len() > 1);
        for _ in 0..100 {
            assert_eq!(
                invoke(&occurrence, &occurrence_args("alpha"), None).expect("a bound term"),
                first
            );
        }

        // And a second relation built over the same content agrees, so the
        // order is a function of the data rather than of one build.
        let rebuilt = TextSearchRelation::new(mixed());
        assert_eq!(
            invoke(&rebuilt, &search_args("alpha"), None).expect("a bound needle"),
            invoke(&search, &search_args("alpha"), None).expect("a bound needle")
        );
    }
}
