// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Positional phrase rows through the ordinary property-function ingress.

use crate::dictionary::{
    Dictionary, DictionaryCache, DictionaryEntry, DictionaryMatch, DictionaryMatches, Occurrence,
};
use crate::{CorpusProfile, GraphLanguage, TextIndex};
use purrdf_core::datatype::{XSD_INTEGER, XSD_STRING};
use purrdf_core::{TermValue, binding_pattern::BindingPattern, small::Shared};
use purrdf_sparql_eval::property_fn::{
    AdmittedPfRow, BatchInvocation, DictionaryBatch, DictionaryShape, IndexGeneration, PfArgs,
    PfArity, PfCursor, PfRow, PropertyFunction,
};
use purrdf_sparql_eval::{
    AdmittedVec, EvalError, Volatility, WorkspaceAllocation, WorkspaceCapability, WorkspaceCursor,
    WorkspaceTerm,
};
use std::sync::Arc;

/// `?doc caller:phrase(?phrase ?language ?start ?end)`, with half-open positions.
/// The IRI is supplied by the caller. The existing single-token relation is unchanged.
#[derive(Debug)]
pub struct PhraseOccurrenceRelation<P: CorpusProfile = GraphLanguage> {
    index: Arc<TextIndex<P>>,
    cache: DictionaryCache,
    generation: Arc<str>,
    modes: [BindingPattern; 1],
    predicate: Option<u32>,
    rows: u64,
}
impl<P: CorpusProfile> PhraseOccurrenceRelation<P> {
    /// Match phrases over all configured indexed predicates, preserving original positions.
    pub fn new(index: Arc<TextIndex<P>>) -> Self {
        let generation = crate::relation::index_generation(&index);
        let rows = index.phrase_row_bound();
        Self {
            index,
            cache: DictionaryCache::default(),
            generation,
            modes: [BindingPattern::from_code("fbfff")],
            predicate: None,
            rows,
        }
    }
    /// Restrict the original token stream to one indexed predicate. Missing predicates
    /// hard-error at registration rather than acquiring an invented default.
    /// # Errors
    /// Refuses a predicate absent from the explicit index configuration.
    pub fn on_predicate(mut self, predicate: &TermValue) -> Result<Self, EvalError> {
        let at = self
            .index
            .config()
            .predicates()
            .iter()
            .position(|configured| configured == predicate)
            .ok_or_else(|| {
                EvalError::from(
                    crate::configuration::ConfigurationRefusal::PhrasePredicate {
                        predicate: predicate.clone(),
                    }
                    .into_error(),
                )
            })?;
        self.predicate = Some(u32::try_from(at).map_err(|_| EvalError::WorkspaceBoundOverflow)?);
        Ok(self)
    }
    /// Configure compilation reuse without limiting the authored dictionary.
    pub fn retain_dictionary(&self, retain: bool) {
        self.cache.set_retention(retain);
    }
    fn owned_cursor(
        &self,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
        workspace: &WorkspaceCapability,
    ) -> Result<WorkspaceCursor<'static>, EvalError> {
        let needle = phrase(args, workspace)?;
        let entries = [DictionaryEntry {
            identity: args.get(1).expect("required phrase"),
            phrase: needle,
            payload: &[],
        }];
        let dictionary = Dictionary::compile(&self.index, &entries, workspace)?;
        let matches = literal_matches(
            self.index.clone(),
            dictionary,
            needle,
            self.predicate,
            workspace,
        )?;
        let results = share(matches, workspace)?;
        cursor(
            results,
            0,
            args,
            ceiling,
            self.generation.clone(),
            false,
            workspace,
        )
    }
}
impl<P: CorpusProfile> PropertyFunction for PhraseOccurrenceRelation<P> {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }
    fn arity(&self) -> PfArity {
        PfArity::new(1, 4)
    }
    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }
    fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
        self.rows
    }
    fn open<'cursor>(
        &self,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor + 'cursor>, EvalError>
    where
        Self: 'cursor,
    {
        self.owned_cursor(args, ceiling, &WorkspaceCapability::resident())
            .and_then(WorkspaceCursor::into_resident)
    }
    fn open_admitted<'cursor>(
        &self,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
        workspace: WorkspaceCapability,
    ) -> Result<WorkspaceCursor<'cursor>, EvalError>
    where
        Self: 'cursor,
    {
        self.owned_cursor(args, ceiling, &workspace)
    }
    fn dictionary_shape(&self) -> Option<DictionaryShape> {
        Some(DictionaryShape {
            phrase: 1,
            start: 3,
            end: 4,
            analyzer: self.index.analyzer_fingerprint(),
        })
    }
    fn prepare_dictionary(
        &self,
        inputs: &[BatchInvocation],
        workspace: &WorkspaceCapability,
    ) -> Result<Option<Box<dyn DictionaryBatch>>, EvalError> {
        if inputs.is_empty() {
            return Ok(None);
        }
        let mut payloads = AdmittedVec::with_capacity(inputs.len(), workspace)?;
        for input in inputs {
            let mut payload = AdmittedVec::with_capacity(input.identity.len(), workspace)?;
            for cell in &input.identity {
                payload.push(cell.as_deref())?;
            }
            payloads.push(payload)?;
        }
        let mut entries = AdmittedVec::with_capacity(inputs.len(), workspace)?;
        for (input, payload) in inputs.iter().zip(&payloads) {
            let Some(Some(needle)) = input.arguments.get(1) else {
                return Ok(None);
            };
            let Some(phrase) = plain_string(needle) else {
                return Ok(None);
            };
            entries.push(DictionaryEntry {
                identity: needle,
                phrase,
                payload,
            })?;
        }
        let dictionary = self.cache.get(&self.index, &entries, workspace)?;
        let results = share(
            dictionary.scan(self.index.clone(), self.predicate, workspace)?,
            workspace,
        )?;
        let control = workspace.charge(
            u64::try_from(size_of::<PhraseBatch<P>>())
                .map_err(|_| EvalError::WorkspaceBoundOverflow)?,
        )?;
        let batch = purrdf_core::small::try_boxed(PhraseBatch {
            results,
            generation: self.generation.clone(),
            _allocation: control,
        })
        .map_err(|_| EvalError::AllocationFailed {
            construct: "phrase dictionary batch",
        })?;
        Ok(Some(batch))
    }
}
fn plain_string(value: &TermValue) -> Option<&str> {
    match value {
        TermValue::Literal {
            lexical_form,
            datatype,
            language: None,
            direction: None,
        } if datatype == XSD_STRING => Some(lexical_form),
        _ => None,
    }
}
fn phrase<'a>(args: &PfArgs<'a>, workspace: &WorkspaceCapability) -> Result<&'a str, EvalError> {
    if args.arity() != PfArity::new(1, 4) {
        return Err(purrdf_sparql_eval::NativeDiagnostic::error(
            purrdf_sparql_eval::NativeDiagnosticKind::Function,
            "phrase occurrence expects one subject and four object arguments",
            workspace,
        ));
    }
    args.get(1).and_then(plain_string).ok_or_else(|| {
        purrdf_sparql_eval::NativeDiagnostic::error(
            purrdf_sparql_eval::NativeDiagnosticKind::Function,
            "phrase occurrence requires a bound xsd:string phrase",
            workspace,
        )
    })
}
#[derive(Debug)]
struct Results<P: CorpusProfile> {
    matches: DictionaryMatches<P>,
    _allocation: WorkspaceAllocation,
}
fn share<P: CorpusProfile>(
    matches: DictionaryMatches<P>,
    workspace: &WorkspaceCapability,
) -> Result<Shared<Results<P>>, EvalError> {
    let allocation = workspace.charge(
        u64::try_from(Shared::<Results<P>>::allocation_layout().size())
            .map_err(|_| EvalError::WorkspaceBoundOverflow)?,
    )?;
    Shared::try_new(Results {
        matches,
        _allocation: allocation,
    })
    .map_err(|_| EvalError::AllocationFailed {
        construct: "dictionary matches owner",
    })
}
struct PhraseBatch<P: CorpusProfile> {
    results: Shared<Results<P>>,
    generation: Arc<str>,
    _allocation: WorkspaceAllocation,
}
impl<P: CorpusProfile> DictionaryBatch for PhraseBatch<P> {
    fn work(&self) -> purrdf_sparql_eval::shapes::ShapeWork {
        let work = self.results.matches.work();
        purrdf_sparql_eval::shapes::ShapeWork {
            source_visits: work.source_visits,
            transitions: work.transitions,
            output_rows: work.output_rows,
        }
    }
    fn open<'cursor>(
        &self,
        ordinal: usize,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
        workspace: &WorkspaceCapability,
    ) -> Result<WorkspaceCursor<'cursor>, EvalError>
    where
        Self: 'cursor,
    {
        cursor(
            self.results.clone(),
            ordinal,
            args,
            ceiling,
            self.generation.clone(),
            ordinal == 0,
            workspace,
        )
    }
}
struct PhraseCursor<P: CorpusProfile> {
    results: Shared<Results<P>>,
    at: usize,
    end: usize,
    bound: AdmittedVec<Option<WorkspaceTerm>>,
    remaining: Option<u64>,
    generation: Arc<str>,
    work: u64,
    workspace: WorkspaceCapability,
}
fn cursor<P: CorpusProfile>(
    results: Shared<Results<P>>,
    ordinal: usize,
    args: &PfArgs<'_>,
    ceiling: Option<u64>,
    generation: Arc<str>,
    count_work: bool,
    workspace: &WorkspaceCapability,
) -> Result<WorkspaceCursor<'static>, EvalError> {
    phrase(args, workspace)?;
    let rows = results.matches.rows();
    let begin = rows.partition_point(|row| row.entry < ordinal);
    let end = rows.partition_point(|row| row.entry <= ordinal);
    let mut bound = AdmittedVec::with_capacity(5, workspace)?;
    for value in args.flattened() {
        bound.push(value.map(|value| workspace.clone_term(value)).transpose()?)?;
    }
    let work = if count_work {
        results
            .matches
            .work()
            .source_visits
            .checked_add(results.matches.work().transitions)
            .ok_or(EvalError::WorkspaceBoundOverflow)?
    } else {
        0
    };
    WorkspaceCursor::new(
        PhraseCursor {
            results,
            at: begin,
            end,
            bound,
            remaining: ceiling,
            generation,
            work,
            workspace: workspace.clone(),
        },
        workspace,
        "phrase occurrence cursor",
    )
}
impl<P: CorpusProfile> PhraseCursor<P> {
    fn next_owned(&mut self) -> Result<Option<AdmittedPfRow>, EvalError> {
        while self.at < self.end && self.remaining != Some(0) {
            let matched = &self.results.matches.rows()[self.at];
            self.at += 1;
            let document = self
                .results
                .matches
                .index()
                .document(matched.path[0].document)
                .expect("retained match document");
            let terms = [
                self.workspace.clone_term(document.subject())?,
                self.workspace
                    .clone_term(self.bound[1].as_ref().expect("required phrase"))?,
                self.workspace
                    .literal(document.language().unwrap_or(""), XSD_STRING)?,
                self.workspace.literal(matched.start, XSD_INTEGER)?,
                self.workspace.literal(matched.end, XSD_INTEGER)?,
            ];
            let mut agrees = true;
            for (term, bound) in terms.iter().zip(&self.bound) {
                if let Some(bound) = bound {
                    agrees &= self.workspace.terms_equal(term, bound)?;
                }
            }
            if !agrees {
                continue;
            }
            if let Some(remaining) = self.remaining.as_mut() {
                *remaining -= 1;
            }
            return Ok(Some(AdmittedPfRow::from_terms(
                terms.into_iter(),
                &self.workspace,
            )?));
        }
        Ok(None)
    }
}
impl<P: CorpusProfile> PfCursor for PhraseCursor<P> {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        self.next_owned()?
            .map(|row| {
                row.try_into_resident().map_err(|_| {
                    EvalError::WorkspaceUnpriced(
                        "an owned phrase row cannot discard its original admission",
                    )
                })
            })
            .transpose()
    }
    fn next_admitted(&mut self) -> Result<Option<AdmittedPfRow>, EvalError> {
        self.next_owned()
    }
    fn take_work(&mut self) -> u64 {
        std::mem::take(&mut self.work)
    }
    fn generation(&self) -> IndexGeneration {
        IndexGeneration::Declared(self.generation.clone())
    }
}

// The literal path is the original positional-call join, not the automaton.
// Terms are visited in phrase order and occurrences in cursor traversal order.
fn literal_matches<P: CorpusProfile>(
    index: Arc<TextIndex<P>>,
    dictionary: Dictionary,
    phrase: &str,
    predicate: Option<u32>,
    workspace: &WorkspaceCapability,
) -> Result<DictionaryMatches<P>, EvalError> {
    let terms = index
        .query_terms_owned(phrase, workspace)
        .map_err(EvalError::from)?;
    let mut candidates = AdmittedVec::with_capacity(terms.len(), workspace)?;
    for term in &terms {
        let mut occurrences = AdmittedVec::new(workspace);
        for (partition, (key, _)) in index.partitions().enumerate() {
            for (document, positions) in index.postings(key, term) {
                for &position in positions {
                    for token in index
                        .token_occurrences(document)
                        .expect("index stream")
                        .filter(|token| {
                            token.position == position
                                && index.occurrence_term(token.term) == Some(term.as_str())
                                && predicate.is_none_or(|predicate| token.predicate == predicate)
                        })
                    {
                        occurrences.push(Occurrence {
                            partition,
                            document,
                            position,
                            ordinal: token.ordinal,
                            term: token.term,
                        })?;
                    }
                }
            }
        }
        candidates.push(occurrences)?;
    }
    let mut rows = AdmittedVec::new(workspace);
    if !terms.is_empty() {
        let mut stack = AdmittedVec::new(workspace);
        let mut path: AdmittedVec<Occurrence> = AdmittedVec::new(workspace);
        stack.push((0usize, 0usize))?;
        while let Some((depth, next)) = stack.pop() {
            path.truncate(depth);
            let Some(&occurrence) = candidates[depth].get(next) else {
                continue;
            };
            stack.push((depth, next + 1))?;
            if let Some(first) = path.first() {
                let a = index.document(first.document).expect("retained source");
                let b = index
                    .document(occurrence.document)
                    .expect("retained source");
                if !workspace.terms_equal(a.subject(), b.subject())?
                    || a.language() != b.language()
                    || u64::from(first.position).checked_add(
                        u64::try_from(depth).map_err(|_| EvalError::WorkspaceBoundOverflow)?,
                    ) != Some(u64::from(occurrence.position))
                {
                    continue;
                }
            }
            path.push(occurrence)?;
            if depth + 1 == candidates.len() {
                let mut owned = AdmittedVec::with_capacity(path.len(), workspace)?;
                for &token in &path {
                    owned.push(token)?;
                }
                rows.push(DictionaryMatch {
                    entry: 0,
                    start: path[0].position,
                    end: occurrence
                        .position
                        .checked_add(1)
                        .ok_or(EvalError::WorkspaceBoundOverflow)?,
                    path: owned,
                })?;
            } else {
                stack.push((depth + 1, 0))?;
            }
        }
    }
    Ok(DictionaryMatches::from_literal(index, dictionary, rows))
}
