// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The ordinary four-cell token relation's exact expanded dictionary access.
//! Logical VALUES identity and each authored token are part of compilation identity.

use crate::{
    CorpusProfile, TextIndex,
    dictionary::{Dictionary, DictionaryCache, DictionaryEntry, DictionaryMatches},
};
use purrdf_core::TermValue;
use purrdf_core::datatype::{XSD_INTEGER, XSD_STRING};
use purrdf_core::vocab::rdf::{DIR_LANG_STRING, LANG_STRING};
use purrdf_sparql_eval::property_fn::{
    AdmittedPfRow, IndexGeneration, OccurrenceDictionary, OccurrenceDictionaryInput,
    OccurrenceMatches,
};
use purrdf_sparql_eval::{AdmittedVec, EvalError, WorkspaceAllocation, WorkspaceCapability};
use std::sync::Arc;

fn text(value: &TermValue) -> Option<&str> {
    match value {
        TermValue::Literal {
            lexical_form,
            datatype,
            ..
        } if matches!(
            datatype.as_str(),
            XSD_STRING | LANG_STRING | DIR_LANG_STRING
        ) =>
        {
            Some(lexical_form)
        }
        _ => None,
    }
}

/// Validates the ORIGINAL token calls, then compiles the authored phrases.
/// This does not traverse an occurrence stream. A semantic mismatch declines
/// before physical source execution; physical refusals are never downgraded.
pub(crate) fn prepare<P: CorpusProfile>(
    index: &Arc<TextIndex<P>>,
    cache: &DictionaryCache,
    generation: &Arc<str>,
    inputs: &[OccurrenceDictionaryInput],
    workspace: &WorkspaceCapability,
) -> Result<Option<Box<dyn OccurrenceDictionary>>, EvalError> {
    let mut chosen = AdmittedVec::with_capacity(inputs.len(), workspace)?;
    let mut payloads = AdmittedVec::with_capacity(inputs.len(), workspace)?;
    for input in inputs {
        if input.tokens.is_empty() {
            return Ok(None);
        }
        let mut tokens = AdmittedVec::with_capacity(input.tokens.len(), workspace)?;
        for token in &input.tokens {
            let Some(lexical) = text(token) else {
                return Ok(None);
            };
            let normalized = index.query_terms_owned(lexical, workspace)?;
            if normalized.len() != 1 {
                return Ok(None);
            }
            tokens.push(normalized)?;
        }
        let mut phrase = None;
        for value in input.payload.iter().filter_map(Option::as_ref) {
            let Some(lexical) = text(value) else {
                continue;
            };
            let normalized = index.query_terms_owned(lexical, workspace)?;
            if normalized.len() == tokens.len()
                && normalized
                    .iter()
                    .zip(&tokens)
                    .all(|(a, b)| a.as_str() == b[0].as_str())
            {
                phrase = Some((&**value, lexical));
                break;
            }
        }
        let Some(phrase) = phrase else {
            return Ok(None);
        };
        chosen.push(phrase)?;
        let mut payload = AdmittedVec::with_capacity(
            input
                .payload
                .len()
                .checked_add(input.tokens.len())
                .ok_or(EvalError::WorkspaceBoundOverflow)?,
            workspace,
        )?;
        for value in &input.payload {
            payload.push(value.as_deref())?;
        }
        // Original token lexical/datatype identities are preserved even if
        // analyzer normalization makes them equal to another spelling.
        for value in &input.tokens {
            payload.push(Some(&**value))?;
        }
        payloads.push(payload)?;
    }
    let mut entries = AdmittedVec::with_capacity(inputs.len(), workspace)?;
    for ((identity, phrase), payload) in chosen.iter().zip(&payloads) {
        entries.push(DictionaryEntry {
            identity,
            phrase,
            payload,
        })?;
    }
    let dictionary = cache.get(index, &entries, workspace)?;
    let allocation = workspace.charge(
        u64::try_from(size_of::<Prepared<P>>()).map_err(|_| EvalError::WorkspaceBoundOverflow)?,
    )?;
    let prepared = purrdf_core::small::try_boxed(Prepared {
        dictionary,
        index: index.clone(),
        generation: generation.clone(),
        _allocation: allocation,
    })
    .map_err(|_| EvalError::AllocationFailed {
        construct: "expanded occurrence dictionary",
    })?;
    Ok(Some(prepared))
}

struct Prepared<P: CorpusProfile> {
    dictionary: Dictionary,
    index: Arc<TextIndex<P>>,
    generation: Arc<str>,
    _allocation: WorkspaceAllocation,
}
impl<P: CorpusProfile> OccurrenceDictionary for Prepared<P> {
    fn scan(
        &self,
        workspace: &WorkspaceCapability,
    ) -> Result<Box<dyn OccurrenceMatches>, EvalError> {
        let matches = self.dictionary.scan(self.index.clone(), None, workspace)?;
        let allocation = workspace.charge(
            u64::try_from(size_of::<Matches<P>>())
                .map_err(|_| EvalError::WorkspaceBoundOverflow)?,
        )?;
        purrdf_core::small::try_boxed(Matches {
            found: matches,
            at: 0,
            generation: self.generation.clone(),
            _allocation: allocation,
        })
        .map(|matches| matches as Box<dyn OccurrenceMatches>)
        .map_err(|_| EvalError::AllocationFailed {
            construct: "expanded dictionary matches",
        })
    }
}

struct Matches<P: CorpusProfile> {
    found: DictionaryMatches<P>,
    at: usize,
    generation: Arc<str>,
    _allocation: WorkspaceAllocation,
}
impl<P: CorpusProfile> OccurrenceMatches for Matches<P> {
    fn work(&self) -> purrdf_sparql_eval::shapes::ShapeWork {
        let work = self.found.work();
        purrdf_sparql_eval::shapes::ShapeWork {
            source_visits: work.source_visits,
            transitions: work.transitions,
            output_rows: work.output_rows,
        }
    }
    fn generation(&self) -> IndexGeneration {
        IndexGeneration::Declared(self.generation.clone())
    }
    fn next(
        &mut self,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<(usize, AdmittedVec<AdmittedPfRow>)>, EvalError> {
        let Some(matched) = self.found.rows().get(self.at) else {
            return Ok(None);
        };
        let dictionary = self.found.dictionary();
        let payload = dictionary.payload(matched.entry);
        let count = dictionary.token_count(matched.entry);
        let tokens = payload
            .get(
                payload
                    .len()
                    .checked_sub(count)
                    .ok_or(EvalError::WorkspaceBoundOverflow)?..,
            )
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        if tokens.len() != matched.path.len() {
            return Err(EvalError::WorkspaceBoundOverflow);
        }
        let mut rows = AdmittedVec::with_capacity(count, workspace)?;
        for (occurrence, token) in matched.path.iter().zip(tokens) {
            let document = self
                .found
                .index()
                .document(occurrence.document)
                .expect("index-owned occurrence");
            let token = token.as_ref().ok_or(EvalError::WorkspaceBoundOverflow)?;
            rows.push(AdmittedPfRow::from_terms(
                [
                    workspace.clone_term(document.subject())?,
                    workspace.clone_term(token)?,
                    workspace.literal(document.language().unwrap_or(""), XSD_STRING)?,
                    workspace.literal(occurrence.position, XSD_INTEGER)?,
                ]
                .into_iter(),
                workspace,
            )?)?;
        }
        self.at = self
            .at
            .checked_add(1)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        Ok(Some((matched.entry, rows)))
    }
}
