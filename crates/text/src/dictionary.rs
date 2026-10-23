// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Analyzer-bound dictionary matching over the index's original token streams.
//! All matches preserve the ordinary positional join's ordered bag. Greedy
//! longest-first selection is an explicit logical consumer, not a shape flag.

use crate::{CorpusProfile, GraphLanguage, TextIndex};
use purrdf_core::{TermValue, small::Shared};
use purrdf_lex::allocation::SharedText;
use purrdf_sparql_eval::{
    AdmittedMap, AdmittedVec, EvalError, WorkspaceAllocation, WorkspaceCapability, WorkspaceTerm,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// One authored dictionary row. Duplicate entries are distinct derivations.
#[derive(Clone, Copy, Debug)]
pub struct DictionaryEntry<'a> {
    /// Original entry identity, independently of its normalized spelling.
    pub identity: &'a TermValue,
    /// Original phrase, analyzed by the index's own analyzer.
    pub phrase: &'a str,
    /// Complete ordered authored payload, including unbound columns. Direct
    /// callers without extra entry columns supply an empty slice.
    pub payload: &'a [Option<&'a TermValue>],
}
#[derive(Debug)]
struct Entry {
    identity: WorkspaceTerm,
    phrase: SharedText,
    payload: AdmittedVec<Option<WorkspaceTerm>>,
}
#[derive(Debug)]
struct State {
    edges: AdmittedMap<SharedText, usize>,
    failure: usize,
    depth: usize,
    terminal: AdmittedVec<usize>,
    outputs: AdmittedVec<usize>,
}
impl State {
    fn new(depth: usize, workspace: &WorkspaceCapability) -> Self {
        Self {
            edges: AdmittedMap::default(),
            failure: 0,
            depth,
            terminal: AdmittedVec::new(workspace),
            outputs: AdmittedVec::new(workspace),
        }
    }
}
#[derive(Debug)]
struct Compiled {
    states: AdmittedVec<State>,
    entries: AdmittedVec<Entry>,
    lengths: AdmittedVec<usize>,
    analyzer: [u8; 32],
    _allocation: WorkspaceAllocation,
}
/// Reusable compiled automaton: no index-local occurrence or token IDs.
#[derive(Clone, Debug)]
pub struct Dictionary(Shared<Compiled>);
impl Dictionary {
    /// Compile once with the exact analyzer owned by `index`.
    /// There is no chosen dictionary-size or phrase-length ceiling.
    ///
    /// # Errors
    /// Preserves analyzer input errors and before-allocation physical refusals.
    pub fn compile<P: CorpusProfile>(
        index: &TextIndex<P>,
        entries: &[DictionaryEntry<'_>],
        workspace: &WorkspaceCapability,
    ) -> Result<Self, EvalError> {
        let mut states = AdmittedVec::new(workspace);
        states.push(State::new(0, workspace))?;
        let mut retained = AdmittedVec::with_capacity(entries.len(), workspace)?;
        let mut lengths = AdmittedVec::with_capacity(entries.len(), workspace)?;
        for (entry, authored) in entries.iter().enumerate() {
            let mut payload = AdmittedVec::with_capacity(authored.payload.len(), workspace)?;
            for value in authored.payload {
                payload.push(value.map(|value| workspace.clone_term(value)).transpose()?)?;
            }
            retained.push(Entry {
                identity: workspace.clone_term(authored.identity)?,
                phrase: workspace.authored_text(&authored.phrase)?,
                payload,
            })?;
            let tokens = index
                .query_terms_owned(authored.phrase, workspace)
                .map_err(EvalError::from)?;
            lengths.push(tokens.len())?;
            let mut state = 0;
            for token in &tokens {
                let next = states[state].edges.get(token.as_str()).copied();
                state = match next {
                    Some(next) => next,
                    None => {
                        let next = states.len();
                        let depth = states[state]
                            .depth
                            .checked_add(1)
                            .ok_or(EvalError::WorkspaceBoundOverflow)?;
                        states.push(State::new(depth, workspace))?;
                        states.as_mut_slice()[state].edges.insert_admitted(
                            workspace.authored_text(token.as_str())?,
                            next,
                            workspace,
                        )?;
                        next
                    }
                };
            }
            // An empty analyzed phrase emits nothing, exactly as positional calls do.
            if !tokens.is_empty() {
                states.as_mut_slice()[state].terminal.push(entry)?;
                states.as_mut_slice()[state].outputs.push(entry)?;
            }
        }
        let mut queue = AdmittedVec::new(workspace);
        for (_, &child) in states[0].edges.iter() {
            queue.push(child)?;
        }
        let mut at = 0;
        while at < queue.len() {
            let parent = queue[at];
            at += 1;
            let mut edges = AdmittedVec::new(workspace);
            for (token, &child) in states[parent].edges.iter() {
                edges.push((token.clone(), child))?;
            }
            for (token, child) in edges {
                let mut failure = states[parent].failure;
                while failure != 0 && states[failure].edges.get(token.as_str()).is_none() {
                    failure = states[failure].failure;
                }
                let fallback = states[failure]
                    .edges
                    .get(token.as_str())
                    .copied()
                    .unwrap_or(0);
                let mut inherited = AdmittedVec::new(workspace);
                for &entry in &states[fallback].outputs {
                    inherited.push(entry)?;
                }
                let target = &mut states.as_mut_slice()[child];
                target.failure = fallback;
                for entry in inherited {
                    target.outputs.push(entry)?;
                }
                target.outputs.as_mut_slice().sort_unstable();
                queue.push(child)?;
            }
        }
        let allocation = workspace.charge(
            u64::try_from(Shared::<Compiled>::allocation_layout().size())
                .map_err(|_| EvalError::WorkspaceBoundOverflow)?,
        )?;
        let compiled = Shared::try_new(Compiled {
            states,
            entries: retained,
            lengths,
            analyzer: index.analyzer_fingerprint(),
            _allocation: allocation,
        })
        .map_err(|_| EvalError::AllocationFailed {
            construct: "dictionary automaton owner",
        })?;
        Ok(Self(compiled))
    }
    /// The complete authored entry retained by this compilation.
    pub fn entry(&self, ordinal: usize) -> Option<(&TermValue, &str)> {
        self.0
            .entries
            .get(ordinal)
            .map(|entry| (&*entry.identity, &*entry.phrase))
    }
    /// Number of distinct authored rows, duplicates included.
    pub fn len(&self) -> usize {
        self.0.entries.len()
    }
    /// Whether no rows were authored.
    pub fn is_empty(&self) -> bool {
        self.0.entries.is_empty()
    }
    pub(crate) fn payload(&self, entry: usize) -> &[Option<WorkspaceTerm>] {
        &self.0.entries[entry].payload
    }
    pub(crate) fn token_count(&self, entry: usize) -> usize {
        self.0.lengths[entry]
    }
    fn matches_key<P: CorpusProfile>(
        &self,
        index: &TextIndex<P>,
        entries: &[DictionaryEntry<'_>],
        workspace: &WorkspaceCapability,
    ) -> Result<bool, EvalError> {
        if self.0.analyzer != index.analyzer_fingerprint() || self.len() != entries.len() {
            return Ok(false);
        }
        for (a, entry) in entries.iter().zip(&self.0.entries) {
            if a.phrase != &*entry.phrase
                || !workspace.terms_equal(a.identity, &entry.identity)?
                || a.payload.len() != entry.payload.len()
            {
                return Ok(false);
            }
            for (a, b) in a.payload.iter().zip(&entry.payload) {
                match (a, b) {
                    (None, None) => {}
                    (Some(a), Some(b)) if workspace.terms_equal(a, b)? => {}
                    _ => return Ok(false),
                }
            }
        }
        Ok(true)
    }
    /// Bind to this actual index generation and scan every source occurrence once
    /// for the complete dictionary. Hidden graph aliases retain positional alternatives.
    ///
    /// # Errors
    /// Refuses an analyzer mismatch, overflow or actual storage failure.
    pub fn scan<P: CorpusProfile>(
        &self,
        index: Arc<TextIndex<P>>,
        predicate: Option<u32>,
        workspace: &WorkspaceCapability,
    ) -> Result<DictionaryMatches<P>, EvalError> {
        if self.0.analyzer != index.analyzer_fingerprint() {
            return Err(purrdf_sparql_eval::NativeDiagnostic::error(
                purrdf_sparql_eval::NativeDiagnosticKind::Config,
                "dictionary analyzer differs from index analyzer",
                workspace,
            ));
        }
        let mut result = DictionaryMatches {
            index,
            dictionary: self.clone(),
            rows: AdmittedVec::new(workspace),
            work: DictionaryWork::default(),
        };
        if self.is_empty() {
            return Ok(result);
        }
        let order = result.index.occurrence_order();
        let mut begin = 0;
        while begin < order.len() {
            let first = result
                .index
                .document(
                    order
                        .get_owned(begin, workspace)?
                        .expect("index-owned document"),
                )
                .expect("index-owned document");
            let mut end = begin + 1;
            while end < order.len() {
                let next = result
                    .index
                    .document(
                        order
                            .get_owned(end, workspace)?
                            .expect("index-owned document"),
                    )
                    .expect("index-owned document");
                if next.language() != first.language()
                    || !workspace.terms_equal(next.subject(), first.subject())?
                {
                    break;
                }
                end += 1;
            }
            let mut occurrences = AdmittedVec::new(workspace);
            for document in order.range_owned(begin, end, workspace) {
                let document = document?;
                let partition = result
                    .index
                    .partition_ordinal_of_owned(document, workspace)?
                    .expect("index-owned partition");
                for token in result
                    .index
                    .token_occurrences(document)
                    .expect("index-owned stream")
                {
                    result.work.source_visits = result
                        .work
                        .source_visits
                        .checked_add(1)
                        .ok_or(EvalError::WorkspaceBoundOverflow)?;
                    if predicate.is_none_or(|predicate| predicate == token.predicate) {
                        occurrences.push(Occurrence {
                            document,
                            partition,
                            position: token.position,
                            ordinal: token.ordinal,
                            term: token.term,
                        })?;
                    }
                }
            }
            occurrences
                .as_mut_slice()
                .sort_unstable_by_key(|occurrence| {
                    (
                        occurrence.position,
                        occurrence.partition,
                        occurrence.document,
                        occurrence.ordinal,
                    )
                });
            if end == begin + 1 {
                scan_unique(
                    self,
                    &occurrences,
                    &mut result.rows,
                    &mut result.work,
                    &result.index,
                    workspace,
                )?;
            } else {
                scan_aliases(
                    self,
                    &occurrences,
                    &mut result.rows,
                    &mut result.work,
                    &result.index,
                    workspace,
                )?;
            }
            begin = end;
        }
        // The ordinary driver is entry-major, then the positional join's nested
        // partition/document/position traversal. Required alias derivations survive.
        result.rows.as_mut_slice().sort_unstable_by(|a, b| {
            a.entry
                .cmp(&b.entry)
                .then_with(|| a.path[..].cmp(&b.path[..]))
        });
        result.work.output_rows =
            u64::try_from(result.rows.len()).map_err(|_| EvalError::WorkspaceBoundOverflow)?;
        Ok(result)
    }
}

/// A bounded, explicit one-entry compilation cache. Disabled retention changes
/// reuse only. Rebinding always uses the current index and never cached occurrence IDs.
#[derive(Debug)]
pub struct DictionaryCache {
    latest: Mutex<Option<Dictionary>>,
    retain: AtomicBool,
}
impl Default for DictionaryCache {
    fn default() -> Self {
        Self {
            latest: Mutex::new(None),
            retain: AtomicBool::new(true),
        }
    }
}
impl DictionaryCache {
    /// Enable or disable the explicit one-entry retention policy. Disabling
    /// evicts this cache's owner, while outstanding matches retain theirs.
    pub fn set_retention(&self, retain: bool) {
        self.retain.store(retain, Ordering::Release);
        if !retain {
            self.clear();
        }
    }
    /// Get the analyzer/authored-dictionary compilation, or build it once.
    ///
    /// # Errors
    /// Preserves analyzer, allocation and checked-layout failures.
    pub fn get<P: CorpusProfile>(
        &self,
        index: &TextIndex<P>,
        entries: &[DictionaryEntry<'_>],
        workspace: &WorkspaceCapability,
    ) -> Result<Dictionary, EvalError> {
        if !self.retain.load(Ordering::Acquire) {
            return Dictionary::compile(index, entries, workspace);
        }
        let mut latest = self
            .latest
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(compiled) = latest.as_ref()
            && compiled.matches_key(index, entries, workspace)?
        {
            return Ok(compiled.clone());
        }
        let compiled = Dictionary::compile(index, entries, workspace)?;
        *latest = Some(compiled.clone());
        drop(latest);
        Ok(compiled)
    }
    /// Evict after payload destruction; existing shallow users keep original owners.
    pub fn clear(&self) {
        self.latest
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
    }
}

/// The physical source identity of one token in a positional derivation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Occurrence {
    /// Physical partition ordinal in the literal cursor's emission order.
    pub partition: usize,
    /// Physical document ordinal; the logical join key omits this graph identity.
    pub document: u32,
    /// Original token position.
    pub position: u32,
    /// Original occurrence ordinal within its stream.
    pub ordinal: usize,
    /// Sorted term dictionary ordinal.
    pub term: u32,
}
/// One match, including the token parents which certify alias multiplicity.
#[derive(Debug)]
pub struct DictionaryMatch {
    /// Authored dictionary row, including duplicate normalized entries.
    pub entry: usize,
    /// Inclusive starting position.
    pub start: u32,
    /// Exclusive ending position.
    pub end: u32,
    /// Original token derivation in positional-call order.
    pub path: AdmittedVec<Occurrence>,
}
/// Actual work separates source visits from inherently larger alias/output work.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DictionaryWork {
    /// Every physical source occurrence is read once for the entire dictionary.
    pub source_visits: u64,
    /// Actual automaton/frontier edge work.
    pub transitions: u64,
    /// Required bag cardinality, not distinct geometry.
    pub output_rows: u64,
}
/// Retained index/compilation and the admitted ordered match bag.
#[derive(Debug)]
pub struct DictionaryMatches<P: CorpusProfile = GraphLanguage> {
    index: Arc<TextIndex<P>>,
    dictionary: Dictionary,
    rows: AdmittedVec<DictionaryMatch>,
    work: DictionaryWork,
}
impl<P: CorpusProfile> DictionaryMatches<P> {
    pub(crate) fn from_literal(
        index: Arc<TextIndex<P>>,
        dictionary: Dictionary,
        rows: AdmittedVec<DictionaryMatch>,
    ) -> Self {
        let output_rows = rows.len() as u64;
        Self {
            index,
            dictionary,
            rows,
            work: DictionaryWork {
                output_rows,
                ..DictionaryWork::default()
            },
        }
    }
    /// The literal ordered bag, borrowed without detaching any native owner.
    pub fn rows(&self) -> &[DictionaryMatch] {
        &self.rows
    }
    /// Original index containing this result's source identity.
    pub fn index(&self) -> &TextIndex<P> {
        &self.index
    }
    /// Reusable analyzer-bound dictionary, retained through result extraction.
    pub const fn dictionary(&self) -> &Dictionary {
        &self.dictionary
    }
    /// Actual physical counts.
    pub const fn work(&self) -> DictionaryWork {
        self.work
    }
    /// Explicit longest-first non-overlap, preserving all derivations attached
    /// to a selected geometry and the original input bag order.
    /// This differs from suppressing only strictly longer CONTAINING spans.
    ///
    /// # Errors
    /// Checked layout or native admission failure before selection scratch growth.
    pub fn longest_first(self, workspace: &WorkspaceCapability) -> Result<Self, EvalError> {
        let mut geometries = AdmittedVec::new(workspace);
        let mut domains = AdmittedVec::<usize>::new(workspace);
        for (ordinal, row) in self.rows.iter().enumerate() {
            let (document, language) = domain(&self.index, row);
            let mut found = None;
            for (at, &representative) in domains.iter().enumerate() {
                let (other, other_language) = domain(&self.index, &self.rows[representative]);
                if language == other_language && workspace.terms_equal(document, other)? {
                    found = Some(at);
                    break;
                }
            }
            let domain = match found {
                Some(at) => at,
                None => {
                    let at = domains.len();
                    domains.push(ordinal)?;
                    at
                }
            };
            geometries.push(purrdf_sparql_eval::shapes::spans::SpanGeometry {
                domain,
                start: u64::from(row.start),
                end: u64::from(row.end),
            })?;
        }
        let selected =
            purrdf_sparql_eval::shapes::spans::longest_first_indices(&geometries, workspace)?;
        let mut result = self;
        let mut keep = selected.into_iter().peekable();
        let mut rows = AdmittedVec::new(workspace);
        for (ordinal, row) in result.rows.into_iter().enumerate() {
            if keep.peek() == Some(&ordinal) {
                keep.next();
                rows.push(row)?;
            }
        }
        result.rows = rows;
        Ok(result)
    }
}
fn domain<'a, P: CorpusProfile>(
    index: &'a TextIndex<P>,
    row: &DictionaryMatch,
) -> (&'a TermValue, Option<&'a str>) {
    let document = index
        .document(row.path[0].document)
        .expect("retained occurrence");
    (document.subject(), document.language())
}
fn transition(
    dictionary: &Dictionary,
    mut state: usize,
    token: &str,
    work: &mut DictionaryWork,
) -> Result<usize, EvalError> {
    work.transitions = work
        .transitions
        .checked_add(1)
        .ok_or(EvalError::WorkspaceBoundOverflow)?;
    while state != 0 && dictionary.0.states[state].edges.get(token).is_none() {
        work.transitions = work
            .transitions
            .checked_add(1)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        state = dictionary.0.states[state].failure;
    }
    Ok(dictionary.0.states[state]
        .edges
        .get(token)
        .copied()
        .unwrap_or(0))
}
fn append_match(
    entry: usize,
    path: impl ExactSizeIterator<Item = Occurrence>,
    rows: &mut AdmittedVec<DictionaryMatch>,
    workspace: &WorkspaceCapability,
) -> Result<(), EvalError> {
    let mut owned = AdmittedVec::with_capacity(path.len(), workspace)?;
    for occurrence in path {
        owned.push(occurrence)?;
    }
    let start = owned[0].position;
    let end = owned
        .last()
        .expect("nonempty normalized phrase")
        .position
        .checked_add(1)
        .ok_or(EvalError::WorkspaceBoundOverflow)?;
    rows.push(DictionaryMatch {
        entry,
        start,
        end,
        path: owned,
    })
}
fn scan_unique<P: CorpusProfile>(
    dictionary: &Dictionary,
    occurrences: &[Occurrence],
    rows: &mut AdmittedVec<DictionaryMatch>,
    work: &mut DictionaryWork,
    index: &TextIndex<P>,
    workspace: &WorkspaceCapability,
) -> Result<(), EvalError> {
    let mut state = 0;
    let mut previous = None;
    for (at, &occurrence) in occurrences.iter().enumerate() {
        if previous
            .is_some_and(|position: u32| position.checked_add(1) != Some(occurrence.position))
        {
            state = 0;
        }
        state = transition(
            dictionary,
            state,
            index.occurrence_term(occurrence.term).expect("index term"),
            work,
        )?;
        for &entry in &dictionary.0.states[state].outputs {
            let length = dictionary.0.lengths[entry];
            append_match(
                entry,
                occurrences[at + 1 - length..=at].iter().copied(),
                rows,
                workspace,
            )?;
        }
        previous = Some(occurrence.position);
    }
    Ok(())
}
#[derive(Clone, Copy)]
struct Parent {
    occurrence: Occurrence,
    parent: Option<usize>,
}
fn scan_aliases<P: CorpusProfile>(
    dictionary: &Dictionary,
    occurrences: &[Occurrence],
    rows: &mut AdmittedVec<DictionaryMatch>,
    work: &mut DictionaryWork,
    index: &TextIndex<P>,
    workspace: &WorkspaceCapability,
) -> Result<(), EvalError> {
    let mut parents = AdmittedVec::new(workspace);
    let mut frontier = AdmittedVec::new(workspace);
    let mut previous = None;
    let mut begin = 0;
    while begin < occurrences.len() {
        let position = occurrences[begin].position;
        if previous.is_some_and(|prior: u32| prior.checked_add(1) != Some(position)) {
            frontier.clear();
        }
        let mut end = begin + 1;
        while end < occurrences.len() && occurrences[end].position == position {
            end += 1;
        }
        let mut next = AdmittedVec::new(workspace);
        // Explicit prefix frontiers enumerate actual alias parent alternatives,
        // avoiding suffix duplication from discarded unrelated histories.
        for occurrence in &occurrences[begin..end] {
            let token = index.occurrence_term(occurrence.term).expect("index term");
            for (state, parent) in std::iter::once((0, None)).chain(
                frontier
                    .iter()
                    .copied()
                    .map(|(state, parent)| (state, Some(parent))),
            ) {
                work.transitions = work
                    .transitions
                    .checked_add(1)
                    .ok_or(EvalError::WorkspaceBoundOverflow)?;
                let Some(&child) = dictionary.0.states[state].edges.get(token) else {
                    continue;
                };
                let parent_id = parents.len();
                parents.push(Parent {
                    occurrence: *occurrence,
                    parent,
                })?;
                next.push((child, parent_id))?;
                for &entry in &dictionary.0.states[child].terminal {
                    let mut reversed =
                        AdmittedVec::with_capacity(dictionary.0.lengths[entry], workspace)?;
                    let mut cursor = Some(parent_id);
                    while let Some(at) = cursor {
                        reversed.push(parents[at].occurrence)?;
                        cursor = parents[at].parent;
                    }
                    reversed.as_mut_slice().reverse();
                    append_match(entry, reversed.iter().copied(), rows, workspace)?;
                }
            }
        }
        frontier = next;
        previous = Some(position);
        begin = end;
    }
    Ok(())
}
