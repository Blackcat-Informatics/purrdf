// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Opt-in triple → source-position side table for the native RDF text codec.
//!
//! The parser normally throws away where each statement came from — the frozen
//! [`RdfDataset`](crate::RdfDataset) carries no source coordinates. Some consumers
//! (SARIF reporting) need to answer "which source line/column did the triple with
//! subject X come from?", so this module adds a side table that records the first
//! source [`Position`] of every statement's subject WITHOUT touching the frozen IR.
//!
//! ## Designed to be zero-cost when off
//!
//! Collection is gated by the [`SpanCollector`] associated const [`SpanCollector::ENABLED`].
//! Every statement producer is generic over the collector and guards recording with
//! `if S::ENABLED { … }`. The default [`NoSpans`] collector is a zero-sized type with
//! `ENABLED = false`, so under monomorphization the guard becomes `if false` and the
//! optimizer is expected to delete the subject-key construction and the `record` call
//! entirely, leaving the pre-existing [`parse_dataset`](crate::parse_dataset) path (which
//! threads `NoSpans`) the same as the code that existed before this feature. Recording is
//! a RUNTIME option ([`ParseOptions::track_source_spans`]), never a Cargo feature.
//!
//! The `native_codecs_parse_span_tracking` group in the `native_codecs` bench
//! is the REPORT-ONLY reference for observing the off path: it runs the tracking-off and
//! tracking-on parses side by side so the disabled path can be watched in the report. It
//! asserts nothing about timing (benches are report-only here). The behavioural guarantee
//! — that the frozen dataset is identical whether or not tracking is requested — is proven
//! by the `parse::tests` (`tracking_off_returns_no_table`, `dataset_is_identical_with_tracking`),
//! not by the bench.

use purrdf_iri::Position;
use purrdf_lex::allocation::{Admission, Memory, StorageError};

/// Sink for per-statement source positions, gated by an associated const so the
/// disabled ([`NoSpans`]) implementation is compiled out entirely — the enabled
/// check `if S::ENABLED { … }` becomes `if false` under monomorphization, so the
/// subject-key construction and the record call vanish on the hot path.
pub(crate) trait SpanCollector {
    /// Whether this collector records anything. `false` makes every `record` call
    /// dead code the optimizer removes.
    const ENABLED: bool;
    /// Record that a statement whose subject has lexical key `subject_key` was
    /// asserted at `position`.
    /// Record through the parser's original account.
    fn record_with_memory<A: Admission + ?Sized>(
        &mut self,
        subject_key: &str,
        position: Position,
        memory: &mut Memory<'_, A>,
    ) -> Result<(), StorageError>;
    /// Transfer the native grammar's exact occurrence table without copying it.
    fn finish_sources(&mut self, _source: super::source_occurrence::SourceOccurrences) {}
    /// Borrow the native source associations during the same original lowering.
    fn sources_mut(&mut self) -> Option<&mut super::source_occurrence::SourceOccurrences> {
        None
    }
}

/// Zero-sized no-op collector: the default, and what every pre-existing parse path
/// uses. `ENABLED = false` makes all recording dead code.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct NoSpans;

impl SpanCollector for NoSpans {
    const ENABLED: bool = false;
    #[inline]
    fn record_with_memory<A: Admission + ?Sized>(
        &mut self,
        _subject_key: &str,
        _position: Position,
        _memory: &mut Memory<'_, A>,
    ) -> Result<(), StorageError> {
        Ok(())
    }
}

/// Opt-in mapping from a data-graph subject to the source [`Position`] where it was
/// first asserted, plus the ordered list of every recorded `(subject, position)`.
///
/// ## Subject-key convention
///
/// A subject is keyed by its LEXICAL string, chosen so a SHACL focus node — rendered
/// as a bare IRI string — joins against it directly:
///
/// * a named node is keyed by its **bare IRI string** (e.g. `http://example.org/alice`),
///   with NO surrounding angle brackets;
/// * a blank node is keyed as `_:label` (the N-Triples blank-node form);
/// * a quoted triple subject is NOT recorded (it has no single lexical key).
#[derive(Debug, Default, Clone)]
pub struct SpanTable {
    source: super::source_occurrence::SourceOccurrences,
    ordered: Vec<(String, Position)>,
    by_subject: purrdf_core::hash::table::HashTable<usize>,
}

impl SpanCollector for SpanTable {
    const ENABLED: bool = true;
    fn sources_mut(&mut self) -> Option<&mut super::source_occurrence::SourceOccurrences> {
        Some(&mut self.source)
    }
    fn finish_sources(&mut self, source: super::source_occurrence::SourceOccurrences) {
        self.source = source;
    }
    fn record_with_memory<A: Admission + ?Sized>(
        &mut self,
        subject_key: &str,
        position: Position,
        memory: &mut Memory<'_, A>,
    ) -> Result<(), StorageError> {
        let hash = purrdf_hash::fixed::FixedHasher::try_hash_terminal(
            0,
            subject_key.as_bytes(),
            &mut |units| memory.admission_mut().checkpoint(units),
        )?;
        let found = self
            .by_subject
            .find_with_control(
                hash,
                |&index, memory| {
                    purrdf_lex::control::compare_bytes(
                        self.ordered[index].0.as_bytes(),
                        subject_key.as_bytes(),
                        &mut |units| memory.admission_mut().checkpoint(units),
                    )
                    .map(core::cmp::Ordering::is_eq)
                },
                memory,
            )?
            .is_some();
        if !found {
            let required = self
                .by_subject
                .len()
                .checked_add(1)
                .ok_or(StorageError::SizeOverflow)?;
            purrdf_core::hash::try_reserve_table_with_memory(
                &mut self.by_subject,
                required,
                |&index, memory| {
                    purrdf_hash::fixed::FixedHasher::try_hash_terminal(
                        0,
                        self.ordered[index].0.as_bytes(),
                        &mut |units| memory.admission_mut().checkpoint(units),
                    )
                },
                memory,
            )?;
        }
        memory.reserve_for_push(&mut self.ordered)?;
        let key = memory.string_with_progress(subject_key)?;
        let index = self.ordered.len();
        self.ordered.push((key, position));
        if !found {
            self.by_subject
                .try_insert_unique_with_control(
                    hash,
                    index,
                    |&index, memory| {
                        purrdf_hash::fixed::FixedHasher::try_hash_terminal(
                            0,
                            self.ordered[index].0.as_bytes(),
                            &mut |units| memory.admission_mut().checkpoint(units),
                        )
                    },
                    memory,
                )
                .map_err(|refusal| purrdf_core::hash::table_storage_error(&refusal.cause))?;
        }
        Ok(())
    }
}

impl SpanTable {
    /// Exact native Turtle/TriG occurrences, before diversion/deduplication.
    /// Other syntax families currently leave this table empty.
    pub fn source_occurrences(&self) -> &super::source_occurrence::SourceOccurrences {
        &self.source
    }
    pub(crate) fn source_occurrences_mut(
        &mut self,
    ) -> &mut super::source_occurrence::SourceOccurrences {
        &mut self.source
    }

    /// The source position where `subject_key` was FIRST asserted, if tracked.
    #[must_use]
    pub fn position_for_subject(&self, subject_key: &str) -> Option<Position> {
        let hash = purrdf_hash::fixed::FixedHasher::hash_terminal(0, subject_key.as_bytes());
        self.by_subject
            .find(hash, |&index| self.ordered[index].0 == subject_key)
            .map(|&index| self.ordered[index].1)
    }

    /// The number of recorded `(subject, position)` entries (document order).
    #[must_use]
    pub fn len(&self) -> usize {
        self.ordered.len()
    }

    /// Whether nothing was recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ordered.is_empty()
    }

    /// Every recorded `(subject key, position)` in document order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, Position)> + '_ {
        self.ordered.iter().map(|(k, p)| (k.as_str(), *p))
    }
}

/// Runtime options for [`parse_dataset_with`](crate::parse_dataset_with).
#[derive(Debug, Default, Clone, Copy)]
pub struct ParseOptions<'a> {
    /// When true, `parse_dataset_with` returns a populated [`SpanTable`]. Opt-in
    /// because it costs memory and pins the sequential line pipeline; off by default
    /// so the hot path is untouched.
    pub track_source_spans: bool,
    /// Explicit YAML-LD Basic controls. Refused for every other input format.
    pub yaml_ld: Option<super::jsonld::YamlLdOptions>,
    /// Initial YAML-LD context and offline registry. Its original compilation
    /// document IRI must match the caller base; explicit context `@base` may differ.
    /// Refused for other formats. Without Basic controls, native defaults apply.
    pub yaml_ld_context: Option<&'a super::jsonld::CompiledJsonLdContext>,
}
