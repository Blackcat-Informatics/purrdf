# Native TEXT owner draft integration

Stage-only implementation proposal for issue 508. The patch is a standard unified diff built from the shipping WIP source read on 2026-10-09. It is not applied, compiled, tested, or acceptance evidence. The sole source writer must reconcile concurrent edits before integrating.

The native search and occurrence entry points override `open_admitted`; resident `open` calls the same helper with the resident capability. Cursor pulls share one `next_owned` body; resident `next` consumes only an actually resident `AdmittedPfRow` through the shipping `try_into_resident`, while `next_admitted` retains every term and cell owner. The draft uses the shipping `WorkspaceCapability::{reserve_vec,reserve_string,clone_term}`, public `AdmittedVec`, `WorkspaceTerm::new` and `AdmittedPfRow::from_terms`. No new dependency, feature, eval-to-text edge, opaque certificate or unsupported native path is introduced.

Ranking admits the complete posting-occurrence population and exact distinct candidate count before filling buffers. `LIMIT` bounds the ranking heap and emitted prefix; it never prices away candidate scoring. A bound document/score/matched count still requires full ranking wherever rank is observed. Holdings borrow partition keys and posting counts. Cursor partition ordinals and native borrowed posting access avoid copying immutable prebuilt index storage. The caller's index, dictionaries and profile are not query allocations.

Analyzer owners cover decoded HTML, normalization output/cleaned/scalar/pending/working/order buffers, segmentation lattice, lexical word/stem work, token strings and Han run storage. Unicode decomposition/fold sizing uses the existing scalar kernels; indexed canonical ordering preserves stable equal-class order without std stable-sort allocation. No stored-term multiplier or guessed expansion factor is used.

Mandatory final integration checks: preserve operational `WorkspaceStopped`/layout overflow/allocator failure through TEXT conversion and the evaluator's final typed boundary; wire admitted diagnostic rendering for data-dependent error messages before formatting; retain analyzer diagnostics until their real owner dies; compile the proposed generic lifetimes and public API callers. Existing `TextError` message construction is not itself a physical certificate. Test the actual production native relation registration and all projections/modes, not just direct ranking helpers. The draft contains no executed checks.

## Draft source units

### query_workspace
```rust
// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Concrete query owners; all vector/string growth stays in the evaluator home.

use purrdf_sparql_eval::{EvalError, WorkspaceAllocation, WorkspaceCapability};

use crate::TextError;

/// The workspace capability's allocation-free stop classes, preserved by TEXT.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CapacityFailure {
    Stopped,
    LayoutOverflow,
    Allocator { construct: &'static str },
    UnstableDiagnostic,
}

impl CapacityFailure {
    /// `charge`, `resize`, and native reserve helpers return only these classes.
    /// Adding another workspace failure requires extending this explicit mapping.
    fn from_workspace(error: EvalError) -> Self {
        match error {
            EvalError::WorkspaceStopped => Self::Stopped,
            EvalError::WorkspaceBoundOverflow => Self::LayoutOverflow,
            EvalError::AllocationFailed { construct } => Self::Allocator { construct },
            EvalError::UnstableNativeDiagnostic => Self::UnstableDiagnostic,
            _ => unreachable!("native workspace helpers changed their failure contract"),
        }
    }

    pub(crate) fn into_eval(self) -> EvalError {
        match self {
            Self::Stopped => EvalError::WorkspaceStopped,
            Self::LayoutOverflow => EvalError::WorkspaceBoundOverflow,
            Self::Allocator { construct } => EvalError::AllocationFailed { construct },
            Self::UnstableDiagnostic => EvalError::UnstableNativeDiagnostic,
        }
    }
}

impl core::fmt::Display for CapacityFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.into_eval(), f)
    }
}

pub(crate) fn admitted<T>(value: Result<T, EvalError>) -> Result<T, TextError> {
    value.map_err(|error| TextError::Capacity(CapacityFailure::from_workspace(error)))
}

pub(crate) fn overflow() -> TextError {
    TextError::Capacity(CapacityFailure::LayoutOverflow)
}

/// One fresh token or generated lexical string, without a lease-dropping clone.
pub(crate) struct QueryString {
    text: String,
    allocation: Option<WorkspaceAllocation>,
}

impl QueryString {
    pub(crate) fn from_parts(text: String, allocation: Option<WorkspaceAllocation>) -> Self {
        Self { text, allocation }
    }
    pub(crate) fn copy(text: &str, workspace: &WorkspaceCapability) -> Result<Self, TextError> {
        let mut result = Self { text: String::new(), allocation: None };
        admitted(workspace.reserve_string(&mut result.text, &mut result.allocation, text.len()))?;
        result.text.push_str(text);
        Ok(result)
    }

    pub(crate) fn chars(
        chars: impl Iterator<Item = char> + Clone, workspace: &WorkspaceCapability,
    ) -> Result<Self, TextError> {
        let bytes = chars.clone().try_fold(0usize, |bytes, c| bytes.checked_add(c.len_utf8()))
            .ok_or_else(overflow)?;
        let mut result = Self { text: String::new(), allocation: None };
        admitted(workspace.reserve_string(&mut result.text, &mut result.allocation, bytes))?;
        result.text.extend(chars);
        Ok(result)
    }

    pub(crate) fn as_str(&self) -> &str { &self.text }
}

impl core::ops::Deref for QueryString {
    type Target = str;
    fn deref(&self) -> &str { &self.text }
}

impl core::fmt::Debug for QueryString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.text, f)
    }
}
```

### score_select
```rust
pub(crate) fn select_counted(
    index: &TextIndex,
    needle: &[String],
    filter: &PartitionFilter,
    ceiling: Option<u64>,
    partition_rank: Option<u32>,
    work: &mut ScoringWork,
) -> Result<Vec<Scored>, TextError> {
    let rows = select_counted_owned(
        index, needle.iter().map(String::as_str), |key| filter.matches(key),
        ceiling, partition_rank, work, &WorkspaceCapability::default(),
    )?;
    // This API creates only resident owners; moving through their iterator cannot
    // shed an operational grant. Bounded callers use the carrier below directly.
    Ok(rows.into_iter().collect())
}

pub(crate) fn select_counted_owned<'a>(
    index: &TextIndex,
    needle: impl ExactSizeIterator<Item = &'a str>,
    matches: impl Fn(&PartitionKey) -> bool,
    ceiling: Option<u64>,
    partition_rank: Option<u32>,
    work: &mut ScoringWork,
    workspace: &WorkspaceCapability,
) -> Result<AdmittedVec<Scored>, TextError> {
    let terms = distinct_terms_owned(needle, workspace)?;
    let limit = partition_rank.map_or(ceiling, |wanted| Some(u64::from(wanted)));
    let emitted = ceiling.map_or(usize::MAX, |k| usize::try_from(k).unwrap_or(usize::MAX));
    let mut rows = AdmittedVec::new(workspace);
    if terms.is_empty() { return Ok(rows); }
    for (key, _) in index.partitions() {
        if rows.len() >= emitted { break; }
        if !matches(key) { continue; }
        let partition_rows = rank_terms_owned(index, key, &terms, limit, work, workspace)?;
        // Rank the entire required working population first. Move only the prefix
        // this output can emit; no oversized append/truncate allocation remains.
        for row in partition_rows {
            if partition_rank.is_none_or(|wanted| row.partition_rank == wanted) {
                if rows.len() == emitted { break; }
                admitted(rows.push(row))?;
            }
        }
    }
    Ok(rows)
}
```

### score_distinct
```rust
pub(crate) fn distinct_terms(needle: &[String]) -> Result<Vec<&str>, TextError> {
    Ok(distinct_terms_owned(needle.iter().map(String::as_str), &WorkspaceCapability::default())?
        .into_iter().collect())
}

pub(crate) fn distinct_terms_owned<'a>(
    needle: impl ExactSizeIterator<Item = &'a str>, workspace: &WorkspaceCapability,
) -> Result<AdmittedVec<&'a str>, TextError> {
    let mut terms = admitted(AdmittedVec::with_capacity(needle.len(), workspace))?;
    for term in needle { admitted(terms.push(term))?; }
    terms.as_mut_slice().sort_unstable();
    let mut distinct = 0;
    for at in 0..terms.len() {
        if distinct == 0 || terms[at] != terms[distinct - 1] {
            terms.as_mut_slice()[distinct] = terms[at];
            distinct += 1;
        }
    }
    terms.truncate(distinct);
    if terms.len() > QUERY_TERMS_MAX {
        return Err(TextError::data(format!(
            "the needle holds {} distinct terms, which exceeds the profile bound of 1024",
            terms.len()
        )));
    }
    Ok(terms)
}
```

### score_rank
```rust
fn rank_terms(
    index: &TextIndex, partition: &PartitionKey, terms: &[&str],
    limit: Option<u64>, work: &mut ScoringWork,
) -> Result<Vec<Scored>, TextError> {
    Ok(rank_terms_owned(index, partition, terms, limit, work, &WorkspaceCapability::default())?
        .into_iter().collect())
}

fn rank_terms_owned(
    index: &TextIndex, partition: &PartitionKey, terms: &[&str],
    limit: Option<u64>, work: &mut ScoringWork, workspace: &WorkspaceCapability,
) -> Result<AdmittedVec<Scored>, TextError> {
    let candidates = candidates_owned(index, partition, terms, work, workspace)?;
    let keep = limit.map(|n| usize::try_from(n).unwrap_or(usize::MAX));
    let ordered = order_owned(candidates, keep, workspace)?;
    let mut rows = admitted(AdmittedVec::with_capacity(ordered.len(), workspace))?;
    for (position, ByRank(candidate)) in ordered.into_iter().enumerate() {
        let rank = position.checked_add(1).ok_or_else(overflow)?;
        let partition_rank = u32::try_from(rank).map_err(|_| {
            TextError::overflow("a partition holds more ranked rows than a u32 can number")
        })?;
        admitted(rows.push(Scored {
            document: candidate.document, score: candidate.score, partition_rank,
            matched: candidate.matched,
        }))?;
    }
    Ok(rows)
}
```

### score_candidates
```rust
fn candidates_owned(
    index: &TextIndex, partition: &PartitionKey, terms: &[&str],
    work: &mut ScoringWork, workspace: &WorkspaceCapability,
) -> Result<AdmittedVec<Candidate>, TextError> {
    if index.partition_stats(partition).is_none() { return Ok(AdmittedVec::new(workspace)); }
    let occurrences_count = terms.iter().try_fold(0usize, |count, term| {
        let run = usize::try_from(index.document_frequency(partition, term))
            .map_err(|_| overflow())?;
        count.checked_add(run).ok_or_else(overflow)
    })?;
    let mut occurrences = admitted(AdmittedVec::with_capacity(occurrences_count, workspace))?;
    for (ordinal, term) in terms.iter().enumerate() {
        let ordinal = u32::try_from(ordinal).map_err(|_| overflow())?;
        for (document, counts) in index.field_postings(partition, term) {
            admitted(occurrences.push(CandidateOccurrence { document, ordinal, counts }))?;
        }
    }
    work.posting_lists = work.posting_lists.checked_add(u64::try_from(terms.len()).map_err(|_| overflow())?)
        .ok_or_else(overflow)?;
    work.postings = work.postings.checked_add(u64::try_from(occurrences.len()).map_err(|_| overflow())?)
        .ok_or_else(overflow)?;
    occurrences.as_mut_slice().sort_unstable_by_key(|entry| (entry.document, entry.ordinal));
    let mut count = 0usize;
    let mut prior = None;
    for entry in occurrences.iter() {
        if prior != Some(entry.document) { count = count.checked_add(1).ok_or_else(overflow)?; }
        prior = Some(entry.document);
    }
    let corpus = index.prepared_corpus_owned(partition, workspace)?;
    let query = prepare_terms_owned(index, partition, &corpus, terms, workspace)?;
    let mut out = admitted(AdmittedVec::with_capacity(count, workspace))?;
    let mut at = 0;
    while at < occurrences.len() {
        let document = occurrences[at].document;
        let run = occurrences[at..].iter().take_while(|entry| entry.document == document).count();
        let held = occurrences[at..at + run].iter().map(|entry| (entry.ordinal as usize, entry.counts));
        admitted(out.push(score_document(index, &query, document, held, work)?))?;
        at += run;
    }
    Ok(out)
}
```

### score_prepare
```rust
fn prepare_terms<'c, 'p>(
    index: &TextIndex, partition: &PartitionKey, corpus: &'c PreparedCorpus<'p>, terms: &[&str],
) -> Result<PreparedQuery<'c, 'p>, TextError> {
    prepare_terms_owned(index, partition, corpus, terms, &WorkspaceCapability::default())
}

fn prepare_terms_owned<'c, 'p>(
    index: &TextIndex, partition: &PartitionKey, corpus: &'c PreparedCorpus<'p>, terms: &[&str],
    workspace: &WorkspaceCapability,
) -> Result<PreparedQuery<'c, 'p>, TextError> {
    let mut frequencies = admitted(AdmittedVec::with_capacity(terms.len(), workspace))?;
    for &term in terms {
        admitted(frequencies.push((term, index.document_frequency(partition, term))))?;
    }
    corpus.prepare_query_owned(&frequencies, workspace)
}
```

### score_order
```rust
struct OwnedOrder {
    values: Vec<ByRank>,
    allocation: Option<WorkspaceAllocation>,
}
struct OwnedOrderIter {
    values: std::vec::IntoIter<ByRank>,
    _allocation: Option<WorkspaceAllocation>,
}
impl IntoIterator for OwnedOrder {
    type Item = ByRank;
    type IntoIter = OwnedOrderIter;
    fn into_iter(self) -> Self::IntoIter {
        OwnedOrderIter { values: self.values.into_iter(), _allocation: self.allocation }
    }
}
impl Iterator for OwnedOrderIter {
    type Item = ByRank;
    fn next(&mut self) -> Option<ByRank> { self.values.next() }
}
impl core::ops::Deref for OwnedOrder {
    type Target = [ByRank];
    fn deref(&self) -> &[ByRank] { &self.values }
}

/// The existing native BinaryHeap and strict ByRank law, with its actual Vec
/// backing pre-admitted. `from`/`into_vec` move that backing without copying it.
fn order_owned(
    candidates: AdmittedVec<Candidate>, keep: Option<usize>, workspace: &WorkspaceCapability,
) -> Result<OwnedOrder, TextError> {
    let wanted = keep.map_or(candidates.len(), |keep| keep.min(candidates.len()));
    let capacity = if keep.is_some_and(|keep| keep < candidates.len()) {
        wanted.checked_add(1).ok_or_else(overflow)?
    } else { wanted };
    let mut values = Vec::new();
    let mut allocation = None;
    admitted(workspace.reserve_vec(&mut values, &mut allocation, capacity))?;
    if wanted == 0 { return Ok(OwnedOrder { values, allocation }); }
    let mut ordered = if keep.is_some() {
        let mut heap = BinaryHeap::from(values);
        for candidate in candidates {
            heap.push(ByRank(candidate));
            if heap.len() > wanted { heap.pop(); }
        }
        heap.into_vec()
    } else {
        for candidate in candidates { values.push(ByRank(candidate)); }
        values
    };
    ordered.sort_unstable();
    Ok(OwnedOrder { values: ordered, allocation })
}
```

### native_diagnostic
```rust
/// Existing request/error meanings with immutable, admission-carrying text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NativeDiagnosticKind {
    /// The relation received an invalid invocation.
    Function,
    /// The relation failed operationally while executing its invocation.
    FunctionOperational,
    /// The requested data violated the native producer's input contract.
    Data,
    /// The supplied native configuration was invalid.
    Config,
    /// A native evaluator invariant failed.
    Internal,
}

#[derive(Debug)]
struct NativeDiagnosticPayload {
    // Payload dies before its string+Shared-header admission is released.
    text: String,
    _allocation: crate::WorkspaceAllocation,
}

/// An engine-supplied diagnostic clones its immutable allocation and lease.
#[derive(Debug, Clone)]
pub struct NativeDiagnostic {
    kind: NativeDiagnosticKind,
    payload: purrdf_core::small::Shared<NativeDiagnosticPayload>,
}

impl NativeDiagnostic {
    /// Stable, allocation-free sizing followed by fallible admitted rendering.
    /// `message` must render borrowed fields without private heap allocation;
    /// a term Debug walker must acquire its own actual work-list grant first.
    pub fn render(
        kind: NativeDiagnosticKind,
        message: impl core::fmt::Display,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<Self, EvalError> {
        let length = crate::workspace::display_len(&message)?;
        let control = purrdf_core::small::Shared::<NativeDiagnosticPayload>::allocation_layout().size();
        let bytes = length.checked_add(control).ok_or(EvalError::WorkspaceBoundOverflow)?;
        let allocation = workspace.charge(u64::try_from(bytes).map_err(|_| EvalError::WorkspaceBoundOverflow)?)?;
        let text = crate::workspace::format_exact(&message, length, "native diagnostic text")?;
        let payload = purrdf_core::small::Shared::try_new(NativeDiagnosticPayload { text, _allocation: allocation })
            .map_err(|_| EvalError::AllocationFailed { construct: "native diagnostic owner" })?;
        Ok(Self { kind, payload })
    }

    /// Stable typed classification, independent of its English message.
    pub const fn kind(&self) -> NativeDiagnosticKind { self.kind }
    /// Borrow the immutable body while retaining its shared owner.
    pub fn message(&self) -> &str { &self.payload.text }
    /// One native diagnostic adaptation for `ok_or_else` and `map_err` callers.
    /// A refused render returns its original static operational failure.
    pub fn error(
        kind: NativeDiagnosticKind, message: impl core::fmt::Display,
        workspace: &crate::WorkspaceCapability,
    ) -> EvalError {
        match Self::render(kind, message, workspace) {
            Ok(diagnostic) => EvalError::NativeDiagnostic(diagnostic),
            Err(error) => error,
        }
    }
    fn preserve_function_failure(mut self) -> Self {
        if self.kind == NativeDiagnosticKind::Function { self.kind = NativeDiagnosticKind::FunctionOperational; }
        self
    }
}

impl PartialEq for NativeDiagnostic {
    fn eq(&self, other: &Self) -> bool { self.kind == other.kind && self.message() == other.message() }
}
impl Eq for NativeDiagnostic {}
impl core::hash::Hash for NativeDiagnostic {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::hash::Hash::hash(&self.kind, state);
        core::hash::Hash::hash(self.message(), state);
    }
}
impl core::fmt::Display for NativeDiagnostic {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let prefix = match self.kind {
            NativeDiagnosticKind::Function | NativeDiagnosticKind::FunctionOperational => "host function error",
            NativeDiagnosticKind::Data => "malformed RDF input",
            NativeDiagnosticKind::Config => "invalid evaluation configuration",
            NativeDiagnosticKind::Internal => "internal evaluator error",
        };
        write!(f, "{prefix}: {}", self.message())
    }
}
impl std::error::Error for NativeDiagnostic {}
```

### ranking_validate
```rust
    fn validate(
        profile: &'p RankingProfile, documents: u64, totals: &[u128], populations: &[u64],
    ) -> Result<Self, TextError> {
        Self::validate_owned(profile, documents, totals, populations, &WorkspaceCapability::default())
    }

    pub(crate) fn from_index_owned(
        profile: &'p RankingProfile, documents: u64, totals: &[u128], populations: &[u64],
        workspace: &WorkspaceCapability,
    ) -> Result<Self, TextError> {
        if profile.uses_field_populations() {
            if populations.len() != profile.fields.len() {
                return Err(query_error(workspace, NativeDiagnosticKind::Data,
                    format_args!("field populations require their ranking mode and one count per field"))?);
            }
            Self::validate_owned(profile, documents, totals, populations, workspace)
        } else {
            Self::validate_owned(profile, documents, totals, &[], workspace)
        }
    }

    fn validate_owned(
        profile: &'p RankingProfile, documents: u64, totals: &[u128], populations: &[u64],
        workspace: &WorkspaceCapability,
    ) -> Result<Self, TextError> {
        if documents > DOCUMENTS_MAX || totals.len() != profile.fields.len() {
            return Err(query_error(workspace, NativeDiagnosticKind::Data,
                format_args!("corpus population or field count exceeds the ranking profile"))?);
        }
        for (at, &total) in totals.iter().enumerate() {
            let population = populations.get(at).copied().unwrap_or(documents);
            if population > documents || total > u128::from(population) * u128::from(FIELD_LENGTH_MAX) {
                return Err(query_error(workspace, NativeDiagnosticKind::Data,
                    format_args!("a field population or token total exceeds its corpus bound"))?);
            }
        }
        let mut owned_totals = admitted(AdmittedVec::with_capacity(totals.len(), workspace))?;
        for &total in totals { admitted(owned_totals.push(total))?; }
        let mut owned_populations = admitted(AdmittedVec::with_capacity(populations.len(), workspace))?;
        for &population in populations { admitted(owned_populations.push(population))?; }
        Ok(Self { profile, documents, totals: owned_totals, populations: owned_populations })
    }
```

### ranking_prepare
```rust
    pub fn prepare_query(
        &self, frequencies: &[(&str, u64)],
    ) -> Result<PreparedQuery<'_, 'p>, TextError> {
        self.prepare_query_owned(frequencies, &WorkspaceCapability::default())
    }

    pub(crate) fn prepare_query_owned(
        &self, frequencies: &[(&str, u64)], workspace: &WorkspaceCapability,
    ) -> Result<PreparedQuery<'_, 'p>, TextError> {
        if frequencies.len() > QUERY_TERMS_MAX {
            return Err(query_error(workspace, NativeDiagnosticKind::Data,
                format_args!("query exceeds 1024 distinct analyzed terms"))?);
        }
        let mut terms = admitted(AdmittedVec::with_capacity(frequencies.len(), workspace))?;
        let total: u128 = self.totals.iter().sum();
        let mut prior: Option<&str> = None;
        let mut frequency_sum = 0_u128;
        for &(term, frequency) in frequencies {
            if term.is_empty() || prior.is_some_and(|prior| prior >= term) {
                return Err(query_error(workspace, NativeDiagnosticKind::Data,
                    format_args!("prepared query terms must be nonempty, distinct and strictly sorted"))?);
            }
            prior = Some(term);
            frequency_sum += u128::from(frequency);
            if frequency_sum > total {
                return Err(query_error(workspace, NativeDiagnosticKind::Data,
                    format_args!("distinct query document frequencies exceed all tokens in their corpus"))?);
            }
            admitted(terms.push((QueryString::copy(term, workspace)?, frequency,
                inverse_document_frequency(self.documents, frequency)?)))?;
        }
        Ok(PreparedQuery { corpus: self, terms })
    }
```

### index_corpus
```rust
    pub(crate) fn prepared_corpus_owned(
        &self, partition: &PartitionKey, workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<PreparedCorpus<'_>, TextError> {
        let at = self.partition_index(partition).ok_or_else(|| TextError::data("ranking partition is absent"))? as usize;
        PreparedCorpus::from_index_owned(&self.ranking, self.partitions[at].1.document_count,
            &self.field_totals[at], &self.field_populations[at], workspace)
    }
```

### query_error
```rust
pub(crate) fn query_error(
    workspace: &WorkspaceCapability,
    kind: purrdf_sparql_eval::NativeDiagnosticKind,
    message: impl core::fmt::Display,
) -> Result<TextError, TextError> {
    Ok(TextError::Diagnostic(admitted(purrdf_sparql_eval::NativeDiagnostic::render(
        kind, message, workspace,
    ))?))
}
```

### html_preallocated
```rust
/// Exact counts produced by the existing reference reader, without allocation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResolutionLayout {
    pub text_bytes: usize,
    pub sources: usize,
    pub diagnostics: usize,
    pub replaced: bool,
}

/// A caller destination was too small or a checked count overflowed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolveIntoError { Capacity, LayoutOverflow }

fn visit_resolution(
    input: &str,
    mode: Mode,
    mut scalar: impl FnMut(char, Range<usize>),
    mut diagnostic: impl FnMut(Diagnostic),
) -> bool {
    if mode == Mode::Plain {
        for (at, c) in input.char_indices() { scalar(c, at..at + c.len_utf8()); }
        return false;
    }
    let mut cursor = 0;
    let mut copied = 0;
    let mut replaced = false;
    while let Some(relative) = crate::scan::find_byte(&input.as_bytes()[cursor..], b'&') {
        let start = cursor + relative;
        let (end, replacement) = reference(input, start, mode, &mut diagnostic);
        if let Some((first, second)) = replacement {
            for (relative, c) in input[copied..start].char_indices() {
                let at = copied + relative;
                scalar(c, at..at + c.len_utf8());
            }
            scalar(first, start..end);
            if second != '\0' { scalar(second, start..end); }
            copied = end;
            replaced = true;
        }
        cursor = end;
    }
    for (relative, c) in input[copied..].char_indices() {
        let at = copied + relative;
        scalar(c, at..at + c.len_utf8());
    }
    replaced
}

/// Inspect actual decoded bytes, scalar ranges and diagnostics at the native home.
pub fn resolution_layout(input: &str, mode: Mode) -> Result<ResolutionLayout, ResolveIntoError> {
    let mut text_bytes = Some(0usize);
    let mut sources = Some(0usize);
    let mut diagnostics = Some(0usize);
    let replaced = visit_resolution(input, mode, |c, _| {
        text_bytes = text_bytes.and_then(|n| n.checked_add(c.len_utf8()));
        sources = sources.and_then(|n| n.checked_add(1));
    }, |_| { diagnostics = diagnostics.and_then(|n| n.checked_add(1)); });
    Ok(ResolutionLayout {
        text_bytes: if replaced { text_bytes.ok_or(ResolveIntoError::LayoutOverflow)? } else { 0 },
        sources: if replaced { sources.ok_or(ResolveIntoError::LayoutOverflow)? } else { 0 },
        diagnostics: diagnostics.ok_or(ResolveIntoError::LayoutOverflow)?,
        replaced,
    })
}

/// Fill caller-preallocated storage through the same reference reader.
/// The result moves those buffers, so their caller keeps the acquired grants.
pub fn resolve_preallocated<'a>(
    input: &'a str,
    mode: Mode,
    layout: ResolutionLayout,
    mut text: String,
    mut sources: Vec<Range<usize>>,
    mut diagnostics: Vec<Diagnostic>,
) -> Result<Resolution<'a>, ResolveIntoError> {
    if text.capacity() < layout.text_bytes || sources.capacity() < layout.sources
        || diagnostics.capacity() < layout.diagnostics { return Err(ResolveIntoError::Capacity); }
    text.clear(); sources.clear(); diagnostics.clear();
    let failed = core::cell::Cell::new(false);
    let replaced = visit_resolution(input, mode, |c, source| {
        if !layout.replaced { return; }
        if text.len().checked_add(c.len_utf8()).is_none_or(|n| n > layout.text_bytes)
            || sources.len() == layout.sources { failed.set(true); return; }
        text.push(c); sources.push(source);
    }, |diagnostic| {
        if diagnostics.len() == layout.diagnostics { failed.set(true); return; }
        diagnostics.push(diagnostic);
    });
    if failed.get() || replaced != layout.replaced || text.len() != layout.text_bytes
        || sources.len() != layout.sources || diagnostics.len() != layout.diagnostics {
        return Err(ResolveIntoError::Capacity);
    }
    Ok(Resolution { decoded: Decoded {
        text: if replaced { Cow::Owned(text) } else { Cow::Borrowed(input) }, sources,
    }, diagnostics })
}
```

### unicode_order
```rust
/// Stable canonical ordering through a caller-preallocated index permutation.
/// Sorting `(class, original ordinal)` is stable in meaning while its unstable
/// in-place sort allocates nothing. A permutation cycle moves metadata too.
fn canonical_order_by_index<T>(
    run: &mut [T], indices: &mut Vec<usize>, class: impl Fn(&T) -> u8,
) {
    assert!(indices.capacity() >= run.len(), "canonical-order caller must preallocate indices");
    indices.clear();
    indices.extend(0..run.len());
    indices.sort_unstable_by_key(|&at| (class(&run[at]), at));
    // Invert destination->source in place. The admitted usize array layout puts
    // ordinals below isize::MAX, so bitwise-negated visited entries are distinct.
    for start in 0..indices.len() {
        if indices[start] >= indices.len() { continue; }
        let mut previous = start;
        let mut at = indices[start];
        while at != start {
            let next = indices[at];
            indices[at] = !previous;
            previous = at;
            at = next;
        }
        indices[start] = !previous;
    }
    for index in indices.iter_mut() { *index = !*index; }
    for start in 0..run.len() {
        while indices[start] != start {
            let next = indices[start];
            run.swap(start, next);
            indices.swap(start, next);
        }
    }
}
```

### unicode_decompose
```rust
/// Exact native decomposition count, including pinned compatibility expansions.
pub fn decomposed_len<const COMPAT: bool, M>(scalars: &[TaggedScalar<M>]) -> Option<usize> {
    let mut count = Some(0usize);
    for scalar in scalars {
        decompose_scalar::<COMPAT>(scalar.value, |_| { count = count.and_then(|n| n.checked_add(1)); });
    }
    count
}

pub fn decompose_tagged<const COMPAT: bool, M: Clone>(
    scalars: &mut Vec<TaggedScalar<M>>, scratch: &mut Vec<TaggedScalar<M>>,
) {
    decompose_tagged_with::<COMPAT, M>(scalars, scratch, |run| canonical_order(run, |scalar| ccc(scalar.value)));
}

/// The same decomposition law with already-admitted destination and sort work.
pub fn decompose_tagged_preallocated<const COMPAT: bool, M: Clone>(
    scalars: &mut Vec<TaggedScalar<M>>, scratch: &mut Vec<TaggedScalar<M>>, order: &mut Vec<usize>,
) {
    assert!(scratch.capacity() >= decomposed_len::<COMPAT, _>(scalars).expect("admitted decomposition count"));
    decompose_tagged_with::<COMPAT, M>(scalars, scratch,
        |run| super::canonical_order_by_index(run, order, |scalar| ccc(scalar.value)));
}

fn decompose_tagged_with<const COMPAT: bool, M: Clone>(
    scalars: &mut Vec<TaggedScalar<M>>, scratch: &mut Vec<TaggedScalar<M>>,
    mut order: impl FnMut(&mut [TaggedScalar<M>]),
) {
    scratch.clear();
    for scalar in scalars.iter() {
        decompose_scalar::<COMPAT>(scalar.value, |value| {
            scratch.push(TaggedScalar { value, metadata: scalar.metadata.clone() });
        });
    }
    let mut start = 0;
    for at in 0..scratch.len() {
        if ccc(scratch[at].value) == 0 { order(&mut scratch[start..at]); start = at + 1; }
    }
    order(&mut scratch[start..]);
    std::mem::swap(scalars, scratch);
}
```

### normalization_owners
```rust
#[derive(Debug, Default)]
struct NormalizationOwners {
    output: Option<WorkspaceAllocation>,
    cleaned: Option<WorkspaceAllocation>,
    scalars: Option<WorkspaceAllocation>,
    pending: Option<WorkspaceAllocation>,
    working: Option<WorkspaceAllocation>,
    order: Option<WorkspaceAllocation>,
}

#[derive(Debug, Default)]
struct NormalizationScratch<M> {
    output: String,
    cleaned: String,
    scalars: Vec<TaggedScalar<M>>,
    pending: Vec<TaggedScalar<M>>,
    working: Vec<TaggedScalar<M>>,
    order: Vec<usize>,
    // Keep all payload buffers before the capabilities that admit them.
    owners: NormalizationOwners,
    workspace: WorkspaceCapability,
}

struct DecodedOwner<'a> {
    resolution: purrdf_lex::html::Resolution<'a>,
    _text: Option<WorkspaceAllocation>,
    _sources: Option<WorkspaceAllocation>,
    _diagnostics: Option<WorkspaceAllocation>,
}

fn decode_owned<'a>(
    input: &'a str, mode: purrdf_lex::html::Mode, workspace: &WorkspaceCapability,
) -> Result<DecodedOwner<'a>, TextError> {
    // The reader/counter are shared at the HTML home; this does not decode once
    // unpriced just to discover what the subsequent allocation would require.
    let layout = purrdf_lex::html::resolution_layout(input, mode).map_err(|_| overflow())?;
    let (mut text_owner, mut source_owner, mut diagnostic_owner) = (None, None, None);
    let mut text = String::new();
    let mut sources = Vec::new();
    let mut diagnostics = Vec::new();
    admitted(workspace.reserve_string(&mut text, &mut text_owner, layout.text_bytes))?;
    admitted(workspace.reserve_vec(&mut sources, &mut source_owner, layout.sources))?;
    admitted(workspace.reserve_vec(&mut diagnostics, &mut diagnostic_owner, layout.diagnostics))?;
    let resolution = purrdf_lex::html::resolve_preallocated(input, mode, layout, text, sources, diagnostics)
        .map_err(|_| TextError::Capacity(crate::CapacityFailure::UnstableDiagnostic))?;
    let owner = DecodedOwner {
        resolution, _text: text_owner, _sources: source_owner, _diagnostics: diagnostic_owner,
    };
    if !owner.resolution.diagnostics.is_empty() {
        // The diagnostic vector and every decoded buffer remain live and covered
        // through rendering. Once this immutable message exists they can die.
        return Err(query_error(workspace, NativeDiagnosticKind::Data,
            format_args!("HTML reference errors: {:?}", owner.resolution.diagnostics))?);
    }
    Ok(owner)
}
```

### normalize_into
```rust
fn normalize_into<A: Alignment>(
    input: &str, mode: InputMode, accent: AccentFold,
    scratch: &mut NormalizationScratch<A::Metadata>, alignment: &mut A,
) -> Result<(), TextError> {
    admit_source_size_owned(input.len(), "analysis input", &scratch.workspace)?;
    scratch.output.clear();
    let mode = match mode {
        InputMode::Plain => purrdf_lex::html::Mode::Plain,
        InputMode::HtmlText => purrdf_lex::html::Mode::Text,
        InputMode::HtmlAttribute => purrdf_lex::html::Mode::Attribute,
    };
    let decoded_owner;
    let resident_decoded;
    let decoded = if scratch.workspace.is_bounded() {
        decoded_owner = decode_owned(input, mode, &scratch.workspace)?;
        &decoded_owner.resolution.decoded
    } else {
        resident_decoded = purrdf_lex::html::resolve_strict(input, mode).map_err(TextError::Html)?;
        &resident_decoded
    };
    admit_source_size_owned(decoded.text.len(), "decoded input", &scratch.workspace)?;
    let mut cleanup = CleanupCursor::new(&decoded.text);
    let no_cleanup = if decoded.text.is_ascii() {
        !decoded.text.char_indices().any(|(at, c)| !cleanup.survives(at, c, false))
    } else {
        !unicode::grapheme_bounds(&decoded.text).any(|(at, cluster)| {
            unicode::emoji_scalars(cluster).any(|(relative, c, protected)| !cleanup.survives(at + relative, c, protected))
        })
    };
    if decoded.sources.is_empty() && no_cleanup {
        if decoded.text.is_ascii() {
            admitted(scratch.workspace.reserve_string(&mut scratch.output, &mut scratch.owners.output, decoded.text.len()))?;
            scratch.output.extend(decoded.text.bytes().map(|byte| char::from(byte.to_ascii_lowercase())));
            return Ok(());
        }
        // The resident streaming compare has private Unicode stage storage.
        // Bounded execution follows this same normalizer's admitted tagged path.
        if !scratch.workspace.is_bounded() {
            let scripts = accent.scripts();
            let accent_safe = scripts.is_empty() || !decoded.text.chars().any(|c| {
                unicode::is_nonspacing_mark(c) || !c.is_ascii() && scripts.iter().any(|script| script.contains(c))
            });
            if accent_safe {
                let mut compare = unicode::Compare::new(&decoded.text);
                unicode::analysis_form(&decoded.text, &mut compare);
                if compare.finish() {
                    admitted(scratch.workspace.reserve_string(&mut scratch.output, &mut scratch.owners.output, decoded.text.len()))?;
                    scratch.output.push_str(&decoded.text);
                    return Ok(());
                }
            }
        }
    }
    scratch.scalars.clear(); scratch.cleaned.clear();
    let scalar_count = decoded.text.chars().count();
    admitted(scratch.workspace.reserve_vec(&mut scratch.scalars, &mut scratch.owners.scalars, scalar_count))?;
    admitted(scratch.workspace.reserve_string(&mut scratch.cleaned, &mut scratch.owners.cleaned, decoded.text.len()))?;
    let mut ordinal = 0;
    let mut cleanup = CleanupCursor::new(&decoded.text);
    for (at, cluster) in unicode::grapheme_bounds(&decoded.text) {
        for (relative, c, protected) in unicode::emoji_scalars(cluster) {
            let offset = at + relative;
            let source = decoded.sources.get(ordinal).cloned().unwrap_or_else(|| offset..offset + c.len_utf8());
            ordinal += 1;
            if !cleanup.survives(offset, c, protected) { continue; }
            let metadata = alignment.source(source);
            scratch.scalars.push(TaggedScalar { value: c, metadata });
            scratch.cleaned.push(c);
        }
    }
    scratch.pending.clear();
    let mut offset = 0;
    // Normalization mutates other scratch fields. Borrow this owner's cleaned
    // text/scalars separately so no clone of either query-sized buffer is needed.
    for (_, cluster) in unicode::grapheme_bounds(&scratch.cleaned) {
        let count = cluster.chars().count();
        let slice = &scratch.scalars[offset..offset + count];
        offset += count;
        if unicode::is_emoji_grapheme(cluster) {
            normalize_run_owned(&mut scratch.pending, &mut scratch.working, &mut scratch.order,
                &mut scratch.owners.pending, &mut scratch.owners.working, &mut scratch.owners.order,
                accent, &scratch.workspace, |left, right| alignment.merge(left, right))?;
            emit_scalars_owned(&mut scratch.output, &scratch.pending, alignment, &scratch.workspace, &mut scratch.owners.output)?;
            scratch.pending.clear();
            emit_scalars_owned(&mut scratch.output, slice, alignment, &scratch.workspace, &mut scratch.owners.output)?;
        } else {
            let needed = scratch.pending.len().checked_add(slice.len()).ok_or_else(overflow)?;
            admitted(scratch.workspace.reserve_vec(&mut scratch.pending, &mut scratch.owners.pending, needed))?;
            scratch.pending.extend_from_slice(slice);
        }
    }
    normalize_run_owned(&mut scratch.pending, &mut scratch.working, &mut scratch.order,
        &mut scratch.owners.pending, &mut scratch.owners.working, &mut scratch.owners.order,
        accent, &scratch.workspace, |left, right| alignment.merge(left, right))?;
    emit_scalars_owned(&mut scratch.output, &scratch.pending, alignment, &scratch.workspace, &mut scratch.owners.output)?;
    scratch.pending.clear();
    Ok(())
}

fn emit_scalars_owned<A: Alignment>(
    output: &mut String, scalars: &[TaggedScalar<A::Metadata>], alignment: &mut A,
    workspace: &WorkspaceCapability, allocation: &mut Option<WorkspaceAllocation>,
) -> Result<(), TextError> {
    let added = scalars.iter().try_fold(0usize, |bytes, scalar| bytes.checked_add(scalar.value.len_utf8())).ok_or_else(overflow)?;
    let needed = output.len().checked_add(added).ok_or_else(overflow)?;
    admitted(workspace.reserve_string(output, allocation, needed))?;
    alignment.reserve_output(scalars.len());
    for scalar in scalars {
        alignment.emit(output.len(), scalar.metadata);
        output.push(scalar.value);
    }
    Ok(())
}
```

### normalize_run
```rust
fn normalize_run_owned<M: Copy>(
    scalars: &mut Vec<TaggedScalar<M>>, scratch: &mut Vec<TaggedScalar<M>>, order: &mut Vec<usize>,
    scalar_owner: &mut Option<WorkspaceAllocation>, scratch_owner: &mut Option<WorkspaceAllocation>,
    order_owner: &mut Option<WorkspaceAllocation>, accent: AccentFold,
    workspace: &WorkspaceCapability, merge: impl FnMut(&mut M, M),
) -> Result<(), TextError> {
    fn decompose<const COMPAT: bool, M: Copy>(
        scalars: &mut Vec<TaggedScalar<M>>, scratch: &mut Vec<TaggedScalar<M>>, order: &mut Vec<usize>,
        scalar_owner: &mut Option<WorkspaceAllocation>, scratch_owner: &mut Option<WorkspaceAllocation>,
        order_owner: &mut Option<WorkspaceAllocation>, workspace: &WorkspaceCapability,
    ) -> Result<(), TextError> {
        let count = purrdf_lex::unicode::decomposed_len::<COMPAT, _>(scalars).ok_or_else(overflow)?;
        admitted(workspace.reserve_vec(scratch, scratch_owner, count))?;
        admitted(workspace.reserve_vec(order, order_owner, count))?;
        purrdf_lex::unicode::decompose_tagged_preallocated::<COMPAT, _>(scalars, scratch, order);
        std::mem::swap(scalar_owner, scratch_owner);
        Ok(())
    }
    fn fold<M: Copy>(
        scalars: &mut Vec<TaggedScalar<M>>, scratch: &mut Vec<TaggedScalar<M>>,
        scalar_owner: &mut Option<WorkspaceAllocation>, scratch_owner: &mut Option<WorkspaceAllocation>,
        workspace: &WorkspaceCapability,
    ) -> Result<(), TextError> {
        let mut count = Some(0usize);
        for scalar in scalars.iter() {
            unicode::fold_scalar(scalar.value, |_| { count = count.and_then(|n| n.checked_add(1)); });
        }
        admitted(workspace.reserve_vec(scratch, scratch_owner, count.ok_or_else(overflow)?))?;
        fold_tagged(scalars, scratch);
        std::mem::swap(scalar_owner, scratch_owner);
        Ok(())
    }
    decompose::<false, _>(scalars, scratch, order, scalar_owner, scratch_owner, order_owner, workspace)?;
    fold(scalars, scratch, scalar_owner, scratch_owner, workspace)?;
    decompose::<true, _>(scalars, scratch, order, scalar_owner, scratch_owner, order_owner, workspace)?;
    fold(scalars, scratch, scalar_owner, scratch_owner, workspace)?;
    decompose::<true, _>(scalars, scratch, order, scalar_owner, scratch_owner, order_owner, workspace)?;
    retain_accents(scalars, accent);
    // Composition only removes scalars; reserve the real source length first.
    admitted(workspace.reserve_vec(scratch, scratch_owner, scalars.len()))?;
    compose_tagged(scalars, scratch, merge);
    std::mem::swap(scalar_owner, scratch_owner);
    Ok(())
}

fn admit_source_size_owned(bytes: usize, kind: &str, workspace: &WorkspaceCapability) -> Result<(), TextError> {
    if bytes > u32::MAX as usize {
        return Err(query_error(workspace, NativeDiagnosticKind::Data,
            format_args!("{kind} exceeds source record space"))?);
    }
    Ok(())
}
```

### segmentation_capacity
```rust
    /// Reserve a complete analyzer-call lattice before the unchanged dictionary
    /// body fills any chunk. Each chunk has at most this many input scalars;
    /// `chars().count()` is a certified bound on graphemes, not a guessed factor.
    pub(crate) fn reserve_for_query(
        &mut self, input: &str, workspace: &WorkspaceCapability,
    ) -> Result<(), TextError> {
        let units = input.chars().count();
        let endpoints = units.checked_add(1).ok_or_else(overflow)?;
        let clean_bytes = input.chars().filter(|&c| !unicode::is_word_internal_control(c))
            .try_fold(0usize, |bytes, c| bytes.checked_add(c.len_utf8())).ok_or_else(overflow)?;
        admitted(workspace.reserve_vec(&mut self.boundaries, &mut self.owners.boundaries, endpoints))?;
        admitted(workspace.reserve_vec(&mut self.clean_offsets, &mut self.owners.clean_offsets, endpoints))?;
        admitted(workspace.reserve_string(&mut self.clean, &mut self.owners.clean, clean_bytes))?;
        admitted(workspace.reserve_vec(&mut self.fallbacks, &mut self.owners.fallbacks, units))?;
        admitted(workspace.reserve_vec(&mut self.barriers, &mut self.owners.barriers, units))?;
        admitted(workspace.reserve_vec(&mut self.paths, &mut self.owners.paths, endpoints))?;
        Ok(())
    }
```

### analyzer_terms
```rust
    pub(crate) fn terms_owned(
        &self, input: &str, workspace: &WorkspaceCapability,
    ) -> Result<AdmittedVec<QueryString>, TextError> {
        let mut scratch = AnalyzerScratch::default();
        scratch.normalization.workspace = workspace.clone();
        let mut terms = AdmittedVec::new(workspace);
        self.analyze_into(input, &mut scratch, &mut |token| {
            admitted(terms.push(QueryString::copy(&token.text, workspace)?))
        })?;
        Ok(terms)
    }

    pub(crate) fn analysis_form_owned(
        &self, input: &str, workspace: &WorkspaceCapability,
    ) -> Result<QueryString, TextError> {
        let mut scratch = NormalizationScratch::default();
        scratch.workspace = workspace.clone();
        normalize_into(input, self.profile.input_mode(), self.profile.accent_fold(), &mut scratch, &mut Unaligned)?;
        // Move the exact existing output and grant; copying it would add another
        // string owner. The other scratch buffers die here.
        Ok(QueryString::from_parts(scratch.output, scratch.owners.output))
    }
```

### native_literal
```rust
    /// Construct a literal from stable borrowed formatting under one payload grant.
    /// This calls the existing term constructor with owned strings, avoiding
    /// a rendered temporary followed by a second lexical copy.
    pub fn literal(
        &self, lexical: impl core::fmt::Display, datatype: &str,
    ) -> Result<WorkspaceTerm, EvalError> {
        let length = display_len(&lexical)?;
        let bytes = length.checked_add(datatype.len()).ok_or(EvalError::WorkspaceBoundOverflow)?;
        let allocation = self.charge(u64::try_from(bytes).map_err(|_| EvalError::WorkspaceBoundOverflow)?)?;
        let lexical = format_exact(&lexical, length, "native literal lexical text")?;
        let datatype = string(datatype, "native literal datatype")?;
        Ok(WorkspaceTerm::new(purrdf_core::TermValue::typed_literal(lexical, datatype), allocation))
    }
```

### format_helpers
```rust
pub(crate) fn display_len(value: &impl core::fmt::Display) -> Result<usize, EvalError> {
    use core::fmt::Write;
    struct Counter(usize);
    impl Write for Counter {
        fn write_str(&mut self, text: &str) -> core::fmt::Result {
            self.0 = self.0.checked_add(text.len()).ok_or(core::fmt::Error)?;
            Ok(())
        }
    }
    let mut counter = Counter(0);
    write!(&mut counter, "{value}").map_err(|_| EvalError::WorkspaceBoundOverflow)?;
    Ok(counter.0)
}

pub(crate) fn format_exact(
    value: &impl core::fmt::Display, length: usize, construct: &'static str,
) -> Result<String, EvalError> {
    use core::fmt::Write;
    let mut text = String::new();
    text.try_reserve_exact(length).map_err(|_| EvalError::AllocationFailed { construct })?;
    struct Bounded<'a> { text: &'a mut String, length: usize }
    impl Write for Bounded<'_> {
        fn write_str(&mut self, text: &str) -> core::fmt::Result {
            if self.text.len().checked_add(text.len()).is_none_or(|n| n > self.length) {
                return Err(core::fmt::Error);
            }
            self.text.push_str(text);
            Ok(())
        }
    }
    write!(&mut Bounded { text: &mut text, length }, "{value}")
        .map_err(|_| EvalError::UnstableNativeDiagnostic)?;
    if text.len() != length { return Err(EvalError::UnstableNativeDiagnostic); }
    Ok(text)
}
```

### fixed_decimal
```rust
/// Formatting the native fixed value borrows it and allocates no private buffer.
pub(crate) struct DecimalDisplay(Fixed);
impl core::fmt::Display for DecimalDisplay {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let magnitude = self.0.0.unsigned_abs();
        let integer = magnitude / SCALE_U;
        let fraction = magnitude % SCALE_U;
        let sign = if self.0.0 < 0 { "-" } else { "" };
        let width = SCALE_DIGITS as usize;
        write!(f, "{sign}{integer}.{fraction:0width$}")
    }
}
impl Fixed {
    pub(crate) const fn decimal_display(self) -> DecimalDisplay { DecimalDisplay(self) }
}
```

### bound_terms
```rust
fn bound_terms(
    args: &PfArgs<'_>, workspace: &WorkspaceCapability,
) -> Result<AdmittedVec<Option<WorkspaceTerm>>, EvalError> {
    let mut bound = AdmittedVec::with_capacity(args.arity().total(), workspace)?;
    for value in args.flattened() {
        bound.push(value.map(|value| workspace.clone_term(value)).transpose()?)?;
    }
    Ok(bound)
}

fn agrees_owned(
    bound: &[Option<WorkspaceTerm>], row: &[TermValue], workspace: &WorkspaceCapability,
) -> Result<bool, EvalError> {
    for (want, have) in bound.iter().zip(row) {
        if let Some(want) = want {
            if !workspace.terms_equal(want, have)? { return Ok(false); }
        }
    }
    Ok(true)
}

fn language_matches(language: &Constraint<&str>, key: &PartitionKey) -> bool {
    match language {
        Constraint::Any => true,
        Constraint::Absent => key.language().is_none(),
        Constraint::Exactly(language) => key.language() == Some(*language),
    }
}
```

### search_owned_helpers
```rust
    fn holdings_owned(
        &self, subject: &TermValue, terms: &[&str], language: &Constraint<&str>,
        workspace: &WorkspaceCapability,
    ) -> Result<AdmittedVec<Holding<'_>>, TextError> {
        let documents = admitted(self.index.documents_with_subject_owned(subject, workspace))?;
        let mut held = admitted(AdmittedVec::with_capacity(documents.len(), workspace))?;
        for &document in documents {
            let Some(key) = self.index.partition_key_of(document) else { continue; };
            if !language_matches(language, key) { continue; }
            let mut located = admitted(AdmittedVec::with_capacity(terms.len(), workspace))?;
            for (ordinal, term) in terms.iter().enumerate() {
                self.observations.membership_lookups.fetch_add(1, Ordering::Relaxed);
                if let Some(counts) = self.index.posted_frequencies(document, term) {
                    admitted(located.push((ordinal, counts)))?;
                }
            }
            if !located.is_empty() { admitted(held.push(Holding { document, key, located }))?; }
        }
        Ok(held)
    }

    fn scored_in_place_owned(
        &self, terms: &[&str], holdings: &[Holding<'_>], workspace: &WorkspaceCapability,
    ) -> Result<AdmittedVec<Hit>, TextError> {
        self.observations.point_scorings.fetch_add(1, Ordering::Relaxed);
        let mut work = ScoringWork::default();
        let result = (|| {
            let mut hits = admitted(AdmittedVec::with_capacity(holdings.len(), workspace))?;
            for holding in holdings {
                let (score, matched) = score_located_owned(&self.index, holding.document, terms, &holding.located, &mut work, workspace)?;
                admitted(hits.push(Hit { document: holding.document, score, matched, rank: None }))?;
            }
            Ok(hits)
        })();
        self.observations.record(work);
        result
    }

    fn ranked_owned(
        &self, needle: &[QueryString], matches: impl Fn(&PartitionKey) -> bool,
        ceiling: Option<u64>, partition_rank: Option<u32>, workspace: &WorkspaceCapability,
    ) -> Result<AdmittedVec<Scored>, TextError> {
        self.observations.rankings.fetch_add(1, Ordering::Relaxed);
        let mut work = ScoringWork::default();
        let result = select_counted_owned(&self.index, needle.iter().map(QueryString::as_str), matches,
            ceiling, partition_rank, &mut work, workspace);
        self.observations.record(work);
        result
    }

    fn open_owned(
        &self, args: &PfArgs<'_>, ceiling: Option<u64>, workspace: WorkspaceCapability,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        check_arity_owned(args, self.arity(), "text search", &workspace)?;
        let needle = args.get(SEARCH_NEEDLE).ok_or_else(|| NativeDiagnostic::error(NativeDiagnosticKind::Function,
            format_args!("the needle at position {SEARCH_NEEDLE} is free; this relation retrieves documents \
                for a needle and cannot enumerate needles for a document, which is why both of \
                its declared modes — `{SEARCH_MODE}` and `{SEARCH_CANDIDATE_MODE}` — demand it"), &workspace))?;
        let text = needle_text_owned(needle, SEARCH_NEEDLE, &workspace)?;
        let rank = args.get(SEARCH_RANK).map(|value| rank_bound_owned(value, &workspace)).transpose()?.unwrap_or(RankBound::Unbound);
        let language = args.get(SEARCH_LANG).map(|value| language_constraint_owned(value, SEARCH_LANG, &workspace))
            .transpose()?.unwrap_or(Constraint::Any);
        let post_rank_filtered = args.get(SEARCH_DOC).is_some() || args.get(SEARCH_SCORE).is_some() || args.get(SEARCH_MATCHED).is_some();
        let select_ceiling = if post_rank_filtered { None } else { ceiling };
        let analyzed = self.index.query_terms_owned(text, &workspace)?;
        let rank_at = match rank { RankBound::At(at) => Some(at), _ => None };
        let mut rows = AdmittedVec::new(&workspace);
        match args.get(SEARCH_DOC) {
            _ if rank == RankBound::BeyondTheIndex => {}
            None => {
                let scored = self.ranked_owned(&analyzed, |key| language_matches(&language, key), select_ceiling, rank_at, &workspace)?;
                rows = AdmittedVec::with_capacity(scored.len(), &workspace)?;
                for row in scored { rows.push(Hit::from(row))?; }
            }
            Some(subject) => {
                let terms = distinct_terms_owned(analyzed.iter().map(QueryString::as_str), &workspace)?;
                let holdings = self.holdings_owned(subject, &terms, &language, &workspace)?;
                if args.is_unobserved(SEARCH_RANK) {
                    if !holdings.is_empty() { rows = self.scored_in_place_owned(&terms, &holdings, &workspace)?; }
                } else if !holdings.is_empty() {
                    let mut documents = AdmittedVec::with_capacity(holdings.len(), &workspace)?;
                    let mut keys = AdmittedVec::with_capacity(holdings.len(), &workspace)?;
                    for holding in holdings.iter() { documents.push(holding.document)?; keys.push(holding.key)?; }
                    drop(holdings);
                    let scored = self.ranked_owned(&analyzed,
                        |key| language_matches(&language, key) && keys.iter().any(|held| *held == key),
                        select_ceiling, rank_at, &workspace)?;
                    rows = AdmittedVec::with_capacity(documents.len().min(scored.len()), &workspace)?;
                    for row in scored {
                        if documents.contains(&row.document) { rows.push(Hit::from(row))?; }
                    }
                }
            }
        }
        let bound = bound_terms(args, &workspace)?;
        let control = workspace.charge(u64::try_from(size_of::<SearchCursor>()).map_err(|_| EvalError::WorkspaceBoundOverflow)?)?;
        let cursor = SearchCursor { index: Arc::clone(&self.index), generation: Arc::clone(&self.generation),
            rows, at: 0, bound, remaining: ceiling, workspace, _control: control };
        let cursor = purrdf_core::small::try_boxed_one(cursor)
            .map_err(|_| EvalError::AllocationFailed { construct: "text search cursor" })?;
        Ok(cursor)
    }
```

### search_cursor
```rust
#[derive(Debug)]
struct SearchCursor {
    index: Arc<TextIndex>,
    generation: Arc<str>,
    rows: AdmittedVec<Hit>,
    at: usize,
    bound: AdmittedVec<Option<WorkspaceTerm>>,
    remaining: Option<u64>,
    workspace: WorkspaceCapability,
    _control: WorkspaceAllocation,
}

impl SearchCursor {
    fn build(&self, hit: &Hit) -> Result<AdmittedPfRow, EvalError> {
        let document = self.index.document(hit.document).ok_or_else(|| NativeDiagnostic::error(
            NativeDiagnosticKind::Data,
            format_args!("a scored row named document {}, which the index does not hold", hit.document), &self.workspace))?;
        let needle = self.bound[SEARCH_NEEDLE].as_ref().expect("validated required needle");
        let terms = [
            self.workspace.clone_term(document.subject())?,
            self.workspace.clone_term(needle)?,
            self.workspace.literal(hit.score.decimal_display(), XSD_DECIMAL)?,
            self.workspace.literal(hit.rank.unwrap_or(0), XSD_INTEGER)?,
            self.workspace.literal(document.language().unwrap_or(""), XSD_STRING)?,
            self.workspace.literal(hit.matched, XSD_INTEGER)?,
        ];
        AdmittedPfRow::from_terms(terms.into_iter(), &self.workspace)
    }

    fn next_owned(&mut self) -> Result<Option<AdmittedPfRow>, EvalError> {
        if self.remaining == Some(0) { return Ok(None); }
        while let Some(hit) = self.rows.get(self.at) {
            self.at += 1;
            let row = self.build(hit)?;
            if agrees_owned(&self.bound, row.cells(), &self.workspace)? {
                if let Some(remaining) = self.remaining.as_mut() { *remaining -= 1; }
                return Ok(Some(row));
            }
        }
        Ok(None)
    }
}

impl PfCursor for [SearchCursor; 1] {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        self[0].next_owned()?.map(|row| row.try_into_resident().map_err(|_| EvalError::WorkspaceStopped)).transpose()
    }
    fn next_admitted(&mut self) -> Result<Option<AdmittedPfRow>, EvalError> { self[0].next_owned() }
    fn generation(&self) -> IndexGeneration { IndexGeneration::Declared(Arc::clone(&self[0].generation)) }
}
```

### han_runs
```rust
fn runs(text: &str, mut sink: impl FnMut(&[Unit])) {
    runs_owned(text, &WorkspaceCapability::default(), |run| { sink(run); Ok(()) })
        .expect("resident Han run allocation");
}

fn runs_owned(
    text: &str, workspace: &WorkspaceCapability, mut sink: impl FnMut(&[Unit]) -> Result<(), TextError>,
) -> Result<(), TextError> {
    let mut run = AdmittedVec::new(workspace);
    for (start, grapheme) in unicode::grapheme_bounds(text) {
        if unicode::is_emoji_grapheme(grapheme) { sink(&run)?; run.clear(); continue; }
        for (offset, scalar) in grapheme.char_indices() {
            if unicode::is_word_internal_control(scalar) || unicode::is_combining_mark(scalar) { continue; }
            if unicode::segmentation_script(scalar) == Some(SegmentationScript::Han) {
                admitted(run.push(Unit { scalar, range: start + offset..start + offset + scalar.len_utf8(),
                    grapheme: start..start + grapheme.len() }))?;
            } else { sink(&run)?; run.clear(); }
        }
    }
    sink(&run)
}

pub(crate) fn query_terms_owned(
    analyzer: &Analyzer, input: &str, workspace: &WorkspaceCapability,
) -> Result<AdmittedVec<QueryString>, TextError> {
    let analysis = analyzer.analysis_form_owned(input, workspace)?;
    let mut out = AdmittedVec::new(workspace);
    runs_owned(&analysis, workspace, |run| {
        if run.len() == 1 {
            admitted(out.push(QueryString::chars([run[0].scalar].into_iter(), workspace)?))?;
        } else {
            for pair in run.windows(2) {
                admitted(out.push(QueryString::chars([pair[0].scalar, pair[1].scalar].into_iter(), workspace)?))?;
            }
        }
        Ok(())
    })?;
    Ok(out)
}
```

### index_terms
```rust
    pub(crate) fn query_terms_owned(
        &self, input: &str, workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<purrdf_sparql_eval::AdmittedVec<crate::query_workspace::QueryString>, TextError> {
        match self.config.projection {
            IndexProjection::Lexical => self.analyzer().terms_owned(input, workspace),
            IndexProjection::Han => crate::character::query_terms_owned(self.analyzer(), input, workspace),
        }
    }
```

### stem_owned
```rust
/// Admit the native stem's selected Letter capacity before its existing body.
/// Native suffix replacements never exceed the original scalar count; the
/// existing byte-length Letter capacity is therefore a certified upper bound.
pub(crate) fn english_in_place_owned(
    word: &mut String, workspace: &purrdf_sparql_eval::WorkspaceCapability,
) -> Result<(), crate::TextError> {
    let mut owner = None;
    english_with(word, |input| {
        let mut letters = Vec::new();
        crate::query_workspace::admitted(workspace.reserve_vec(&mut letters, &mut owner, input.len()))?;
        Ok(Stem::from_letters(input, letters))
    })
}
```

### subject_lookup
```rust
    /// The existing two subject-side binary searches with admitted native term
    /// ordering. After a comparator failure the partition result is discarded;
    /// the first typed failure is returned before any caller can use the slice.
    pub(crate) fn documents_with_subject_owned(
        &self, subject: &TermValue, workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<&[u32], purrdf_sparql_eval::EvalError> {
        fn partition(
            index: &TextIndex, subject: &TermValue, workspace: &purrdf_sparql_eval::WorkspaceCapability,
            upper: bool,
        ) -> Result<usize, purrdf_sparql_eval::EvalError> {
            let mut failure = None;
            let result = index.subject_order.partition_point(|id| {
                if failure.is_some() { return false; }
                match workspace.terms_cmp(&index.documents[*id as usize].subject, subject) {
                    Ok(order) => if upper { order != Ordering::Greater } else { order == Ordering::Less },
                    Err(error) => { failure = Some(error); false }
                }
            });
            match failure { Some(error) => Err(error), None => Ok(result) }
        }
        let start = partition(self, subject, workspace, false)?;
        let end = partition(self, subject, workspace, true)?;
        Ok(&self.subject_order[start..end])
    }

    /// Borrow one immutable positional posting without copying the caller's index.
    pub(crate) fn posting_at(
        &self, partition: usize, term: &str, ordinal: usize,
    ) -> Option<(u32, &[u32])> {
        let key = &self.partitions.get(partition)?.0;
        let posting = self.partition_postings(key, term).get(ordinal)?;
        Some((posting.document, &posting.positions))
    }

    pub(crate) fn partition_at(&self, ordinal: usize) -> Option<&PartitionKey> {
        self.partitions.get(ordinal).map(|(key, _)| key)
    }
```

### occurrence_open
```rust
    fn open_owned(
        &self, args: &PfArgs<'_>, ceiling: Option<u64>, workspace: WorkspaceCapability,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        check_arity_owned(args, self.arity(), "term occurrence", &workspace)?;
        let needle = args.get(OCCURRENCE_TERM).ok_or_else(|| NativeDiagnostic::error(
            NativeDiagnosticKind::Function,
            format_args!("the term at position {OCCURRENCE_TERM} is free; this relation enumerates the \
                occurrences of a term and cannot enumerate terms, which is why its only declared \
                mode is `{OCCURRENCE_MODE}`"), &workspace))?;
        let text = needle_text_owned(needle, OCCURRENCE_TERM, &workspace)?;
        let language = args.get(OCCURRENCE_LANG).map(|value| language_constraint_owned(value, OCCURRENCE_LANG, &workspace))
            .transpose()?.unwrap_or(Constraint::Any);
        let documents = args.get(OCCURRENCE_DOC).map(|subject| self.index.documents_with_subject_owned(subject, &workspace)).transpose()?;
        let analyzed = self.index.query_terms_owned(text, &workspace)?;
        if analyzed.len() > 1 {
            return Err(NativeDiagnostic::error(NativeDiagnosticKind::Function,
                format_args!("the term at position {OCCURRENCE_TERM} is {text:?}, which analyzes to \
                    {count} terms ({analyzed_terms:?}); this relation matches ONE term per invocation by \
                    contract, so a multi-term needle is written as one call per term joined on the \
                    document position", count = analyzed.len(), analyzed_terms = &*analyzed), &workspace));
        }
        let mut partitions = AdmittedVec::new(&workspace);
        if analyzed.first().is_some_and(|term| !term.is_empty()) {
            for (ordinal, (key, _)) in self.index.partitions().enumerate() {
                if language_matches(&language, key) && documents.is_none_or(|documents| {
                    documents.iter().any(|&document| self.index.partition_key_of(document) == Some(key))
                }) { partitions.push(ordinal)?; }
            }
        }
        let bound = bound_terms(args, &workspace)?;
        let control = workspace.charge(u64::try_from(size_of::<OccurrenceCursor>()).map_err(|_| EvalError::WorkspaceBoundOverflow)?)?;
        let cursor = OccurrenceCursor { index: Arc::clone(&self.index), generation: Arc::clone(&self.generation),
            analyzed, partitions, partition_at: 0, posting_at: 0, position_at: 0, bound,
            remaining: ceiling, workspace, _control: control };
        let cursor = purrdf_core::small::try_boxed_one(cursor)
            .map_err(|_| EvalError::AllocationFailed { construct: "term occurrence cursor" })?;
        Ok(cursor)
    }
```

### occurrence_cursor
```rust
#[derive(Debug)]
struct OccurrenceCursor {
    index: Arc<TextIndex>,
    generation: Arc<str>,
    analyzed: AdmittedVec<QueryString>,
    partitions: AdmittedVec<usize>,
    partition_at: usize,
    posting_at: usize,
    position_at: usize,
    bound: AdmittedVec<Option<WorkspaceTerm>>,
    remaining: Option<u64>,
    workspace: WorkspaceCapability,
    _control: WorkspaceAllocation,
}

impl OccurrenceCursor {
    fn build(&self, document: u32, position: u32) -> Result<AdmittedPfRow, EvalError> {
        let held = self.index.document(document).ok_or_else(|| NativeDiagnostic::error(
            NativeDiagnosticKind::Data,
            format_args!("a posting named document {document}, which the index does not hold"), &self.workspace))?;
        let needle = self.bound[OCCURRENCE_TERM].as_ref().expect("validated required occurrence term");
        let terms = [self.workspace.clone_term(held.subject())?, self.workspace.clone_term(needle)?,
            self.workspace.literal(held.language().unwrap_or(""), XSD_STRING)?, self.workspace.literal(position, XSD_INTEGER)?];
        AdmittedPfRow::from_terms(terms.into_iter(), &self.workspace)
    }

    fn next_owned(&mut self) -> Result<Option<AdmittedPfRow>, EvalError> {
        let Some(term) = self.analyzed.first() else { return Ok(None); };
        loop {
            if self.remaining == Some(0) { return Ok(None); }
            let Some(&partition) = self.partitions.get(self.partition_at) else { return Ok(None); };
            let Some((document, positions)) = self.index.posting_at(partition, term, self.posting_at) else {
                self.partition_at += 1; self.posting_at = 0; self.position_at = 0; continue;
            };
            let Some(&position) = positions.get(self.position_at) else {
                self.posting_at += 1; self.position_at = 0; continue;
            };
            self.position_at += 1;
            let row = self.build(document, position)?;
            if agrees_owned(&self.bound, row.cells(), &self.workspace)? {
                if let Some(remaining) = self.remaining.as_mut() { *remaining -= 1; }
                return Ok(Some(row));
            }
        }
    }
}

impl PfCursor for [OccurrenceCursor; 1] {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        self[0].next_owned()?.map(|row| row.try_into_resident().map_err(|_| EvalError::WorkspaceStopped)).transpose()
    }
    fn next_admitted(&mut self) -> Result<Option<AdmittedPfRow>, EvalError> { self[0].next_owned() }
    fn generation(&self) -> IndexGeneration { IndexGeneration::Declared(Arc::clone(&self[0].generation)) }
}
```

### score_located
```rust
pub(crate) fn score_located(
    index: &TextIndex, document: u32, terms: &[&str], located: &[(usize, &[(u32, u64)])], work: &mut ScoringWork,
) -> Result<(Fixed, u32), TextError> {
    score_located_owned(index, document, terms, located, work, &WorkspaceCapability::default())
}

pub(crate) fn score_located_owned(
    index: &TextIndex, document: u32, terms: &[&str], located: &[(usize, &[(u32, u64)])], work: &mut ScoringWork,
    workspace: &WorkspaceCapability,
) -> Result<(Fixed, u32), TextError> {
    let Some(partition) = index.partition_key_of(document) else {
        return Err(query_error(workspace, NativeDiagnosticKind::Data,
            format_args!("document {document} is not in this index, so there is nothing to score"))?);
    };
    let corpus = index.prepared_corpus_owned(partition, workspace)?;
    let query = prepare_terms_owned(index, partition, &corpus, terms, workspace)?;
    let scored = score_document(index, &query, document, located.iter().copied(), work, workspace)?;
    Ok((scored.score, scored.matched))
}
```

### validate_score
```rust
    pub fn validate_score(&self, score: Fixed) -> Result<Fixed, TextError> {
        self.validate_score_with(score, None)
    }

    pub(crate) fn validate_score_owned(
        &self, score: Fixed, workspace: &WorkspaceCapability,
    ) -> Result<Fixed, TextError> {
        self.validate_score_with(score, Some(workspace))
    }

    fn validate_score_with(
        &self, score: Fixed, workspace: Option<&WorkspaceCapability>,
    ) -> Result<Fixed, TextError> {
        if !(Fixed::ZERO..=SCORE_MAX).contains(&score) {
            return Err(match workspace {
                Some(workspace) => query_error(workspace, NativeDiagnosticKind::Function,
                    format_args!("fixed-point domain error: score {} is outside [0, {}]",
                        score.decimal_display(), SCORE_MAX.decimal_display()))?,
                None => TextError::domain(format!("score {} is outside [0, {}]",
                    score.decimal_display(), SCORE_MAX.decimal_display())),
            });
        }
        Ok(score)
    }
```
