// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Whole-read admission for hostile GTS input.

use super::{
    AnnotationRow, BlobPayload, BlobRefusal, Diagnostic, DictionaryError, FrameContext, OpaqueNode,
    Quad, ReifierRow, Signature, StreamableInfo, StreamingSink, Suppression, Term, TermLease,
    Value,
};
use std::cell::Cell;
use std::sync::Arc;

const MIB: usize = 1024 * 1024;

/// Complete resource contract for one GTS read.
///
/// New bounded entry points use the finite default when their caller does not
/// provide a record. Legacy v4 entry points retain their unbounded contract;
/// hostile input must use a bounded entry point and keep finite limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GtsReadLimits {
    /// Complete encoded input bytes.
    pub encoded_input_bytes: usize,
    /// CBOR sequence items, including segment headers.
    pub items: usize,
    /// Encoded bytes in any one header or frame.
    pub encoded_frame_bytes: usize,
    /// Decoded bytes in any one non-blob payload.
    pub decoded_frame_bytes: usize,
    /// Decoded bytes in any one blob payload.
    pub decoded_blob_bytes: usize,
    /// Aggregate decoded payload bytes across the read.
    pub total_decoded_bytes: usize,
    /// Accepted term rows.
    pub terms: usize,
    /// Accepted quad, reifier, annotation and suppression rows.
    pub rows: usize,
    /// Retained reader metadata entries.
    pub metadata_entries: usize,
    /// Reader diagnostics.
    pub diagnostics: usize,
    /// Measurable reader work events.
    pub work: usize,
}

impl GtsReadLimits {
    /// Finite host-safe defaults for ordinary GTS imports.
    pub const DEFAULT: Self = Self {
        encoded_input_bytes: 256 * MIB,
        items: 1_000_000,
        encoded_frame_bytes: 64 * MIB,
        decoded_frame_bytes: 64 * MIB,
        decoded_blob_bytes: 64 * MIB,
        total_decoded_bytes: 256 * MIB,
        terms: 2_000_000,
        rows: 8_000_000,
        metadata_entries: 262_144,
        diagnostics: 4_096,
        // Encoded and decoded bytes each count as work, alongside item and row
        // events. Keep the independent work ceiling above every combination
        // admitted by the other defaults so it binds pathological structure,
        // not ordinary byte throughput.
        work: 600_000_000,
    };

    /// Explicit trusted-input behavior matching the historical reader.
    pub const UNBOUNDED: Self = Self {
        encoded_input_bytes: usize::MAX,
        items: usize::MAX,
        encoded_frame_bytes: usize::MAX,
        decoded_frame_bytes: usize::MAX,
        decoded_blob_bytes: usize::MAX,
        total_decoded_bytes: usize::MAX,
        terms: usize::MAX,
        rows: usize::MAX,
        metadata_entries: usize::MAX,
        diagnostics: usize::MAX,
        work: usize::MAX,
    };
}

impl Default for GtsReadLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Exact counters observed before a whole-read refusal.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GtsReadUsage {
    /// Complete encoded input bytes.
    pub encoded_input_bytes: usize,
    /// CBOR sequence items encountered.
    pub items: usize,
    /// Largest encoded header or frame encountered.
    pub largest_encoded_frame_bytes: usize,
    /// Largest decoded non-blob payload encountered.
    pub largest_decoded_frame_bytes: usize,
    /// Largest decoded blob payload encountered.
    pub largest_decoded_blob_bytes: usize,
    /// Aggregate decoded payload bytes.
    pub total_decoded_bytes: usize,
    /// Accepted term rows.
    pub terms: usize,
    /// Accepted quad, reifier, annotation and suppression rows.
    pub rows: usize,
    /// Peak retained reader metadata entries.
    pub metadata_entries: usize,
    /// Reader diagnostics encountered.
    pub diagnostics: usize,
    /// Counted reader work events.
    pub work: usize,
}

impl purrdf_hash::error::StructuredError for GtsReadUsage {
    purrdf_hash::error_visit! {
        @record self { encoded_input_bytes, items, largest_encoded_frame_bytes,
            largest_decoded_frame_bytes, largest_decoded_blob_bytes,
            total_decoded_bytes, terms, rows, metadata_entries, diagnostics, work }
        => "gts-read-usage" {
            "encoded_input_bytes" => ByteCount(self.encoded_input_bytes as u128),
            "items" => Unsigned(self.items as u128),
            "largest_encoded_frame_bytes" => ByteCount(self.largest_encoded_frame_bytes as u128),
            "largest_decoded_frame_bytes" => ByteCount(self.largest_decoded_frame_bytes as u128),
            "largest_decoded_blob_bytes" => ByteCount(self.largest_decoded_blob_bytes as u128),
            "total_decoded_bytes" => ByteCount(self.total_decoded_bytes as u128),
            "terms" => Unsigned(self.terms as u128),
            "rows" => Unsigned(self.rows as u128),
            "metadata_entries" => Unsigned(self.metadata_entries as u128),
            "diagnostics" => Unsigned(self.diagnostics as u128),
            "work" => Unsigned(self.work as u128)
        }
    }
}

/// Resource dimension refused by [`GtsReadLimits`].
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GtsReadResource {
    /// Complete encoded input bytes.
    EncodedInputBytes,
    /// CBOR sequence item count.
    Items,
    /// Encoded bytes in one header or frame.
    EncodedFrameBytes,
    /// Decoded bytes in one non-blob payload.
    DecodedFrameBytes,
    /// Decoded bytes in one blob payload.
    DecodedBlobBytes,
    /// Aggregate decoded payload bytes.
    TotalDecodedBytes,
    /// Aggregate decoded nested-artifact bytes retained by recursive reading.
    NestedDecodedBytes,
    /// Recursive nested-artifact depth.
    NestedDepth,
    /// Accepted term rows.
    Terms,
    /// Accepted RDF and suppression rows.
    Rows,
    /// Retained reader metadata entries.
    MetadataEntries,
    /// Reader diagnostics.
    Diagnostics,
    /// Measurable reader work events.
    Work,
}

impl GtsReadResource {
    /// Stable machine-readable resource label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::EncodedInputBytes => "encoded-input-bytes",
            Self::Items => "items",
            Self::EncodedFrameBytes => "encoded-frame-bytes",
            Self::DecodedFrameBytes => "decoded-frame-bytes",
            Self::DecodedBlobBytes => "decoded-blob-bytes",
            Self::TotalDecodedBytes => "total-decoded-bytes",
            Self::NestedDecodedBytes => "nested-decoded-bytes",
            Self::NestedDepth => "nested-depth",
            Self::Terms => "terms",
            Self::Rows => "rows",
            Self::MetadataEntries => "metadata-entries",
            Self::Diagnostics => "diagnostics",
            Self::Work => "work",
        }
    }
}

impl std::fmt::Display for GtsReadResource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label())
    }
}

impl purrdf_hash::error::StructuredError for GtsReadResource {
    fn visit<'error>(&'error self, visitor: &mut dyn purrdf_hash::error::ErrorVisitor<'error>) {
        visitor.variant("gts-read-resource");
        visitor.value(
            Some("resource"),
            purrdf_hash::error::ErrorValue::Text(self.label()),
        );
    }
}

/// Typed terminal refusal from a bounded GTS read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GtsReadLimitError {
    /// Refused resource.
    pub resource: GtsReadResource,
    /// Amount required by the first refused event.
    pub required: usize,
    /// Configured ceiling.
    pub limit: usize,
    /// Exact counters observed at refusal.
    pub usage: GtsReadUsage,
}

impl std::fmt::Display for GtsReadLimitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "GTS read {} requires {}, limit {}",
            self.resource, self.required, self.limit
        )
    }
}

impl std::error::Error for GtsReadLimitError {}

impl purrdf_hash::error::StructuredError for GtsReadLimitError {
    purrdf_hash::error_visit! {
        @record self { resource, required, limit, usage } => "gts-read-limit" {
            "resource" => Cause(&self.resource),
            "required" => Unsigned(self.required as u128),
            "limit" => Unsigned(self.limit as u128),
            "usage" => Cause(&self.usage)
        }
    }
}

impl GtsReadLimitError {
    pub(super) fn input(actual: usize, limits: GtsReadLimits) -> Self {
        Self {
            resource: GtsReadResource::EncodedInputBytes,
            required: actual,
            limit: limits.encoded_input_bytes,
            usage: GtsReadUsage {
                encoded_input_bytes: actual,
                ..GtsReadUsage::default()
            },
        }
    }

    pub(super) fn diagnostic(&self) -> Diagnostic {
        Diagnostic {
            code: "ResourceLimit".to_owned(),
            detail: self.to_string(),
            frame_index: None,
            cause: None,
        }
        .with_native_cause(self.clone())
    }
}

pub(super) struct ReadLimiter<'a> {
    inner: &'a mut dyn StreamingSink,
    limits: GtsReadLimits,
    usage: GtsReadUsage,
    charge_encoded_work: bool,
    current_metadata_entries: usize,
    error: Option<GtsReadLimitError>,
    cancelled: Cell<bool>,
}

impl<'a> ReadLimiter<'a> {
    pub(super) fn new(
        inner: &'a mut dyn StreamingSink,
        limits: GtsReadLimits,
        encoded_input_bytes: usize,
    ) -> Self {
        Self {
            inner,
            limits,
            usage: GtsReadUsage {
                encoded_input_bytes,
                ..GtsReadUsage::default()
            },
            charge_encoded_work: true,
            current_metadata_entries: 0,
            error: None,
            cancelled: Cell::new(false),
        }
    }

    pub(super) fn with_admitted_envelope(
        inner: &'a mut dyn StreamingSink,
        limits: GtsReadLimits,
        encoded_input_bytes: usize,
        envelope_work: usize,
    ) -> Self {
        let mut limiter = Self::new(inner, limits, encoded_input_bytes);
        limiter.usage.work = envelope_work;
        limiter.charge_encoded_work = false;
        limiter
    }

    pub(super) const fn usage(&self) -> GtsReadUsage {
        self.usage
    }

    pub(super) fn complete_input(&mut self, actual: usize) {
        self.usage.encoded_input_bytes = self.usage.encoded_input_bytes.max(actual);
        if let Some(error) = self.error.as_mut() {
            error.usage.encoded_input_bytes = error.usage.encoded_input_bytes.max(actual);
        }
        self.trip(
            GtsReadResource::EncodedInputBytes,
            self.usage.encoded_input_bytes,
            self.limits.encoded_input_bytes,
        );
    }

    pub(super) fn take_error(&mut self) -> Option<GtsReadLimitError> {
        self.error.take()
    }

    fn trip(&mut self, resource: GtsReadResource, required: usize, limit: usize) -> bool {
        if required <= limit {
            return false;
        }
        if self.error.is_none() {
            self.error = Some(GtsReadLimitError {
                resource,
                required,
                limit,
                usage: self.usage,
            });
        }
        true
    }

    fn charge_work(&mut self, amount: usize) -> bool {
        let required = self.usage.work.saturating_add(amount);
        self.usage.work = required;
        self.trip(GtsReadResource::Work, required, self.limits.work)
    }

    fn effective_limit(configured: usize, inner: Option<usize>, remaining: usize) -> Option<usize> {
        let limit = configured.min(remaining).min(inner.unwrap_or(usize::MAX));
        (limit != usize::MAX).then_some(limit)
    }

    fn count_row(&mut self) -> bool {
        let required = self.usage.rows.saturating_add(1);
        self.usage.rows = required;
        self.charge_work(1) || self.trip(GtsReadResource::Rows, required, self.limits.rows)
    }

    fn count_decoded(&mut self, blob: bool, bytes: usize, charge_work: bool) -> bool {
        if blob {
            self.usage.largest_decoded_blob_bytes =
                self.usage.largest_decoded_blob_bytes.max(bytes);
        } else {
            self.usage.largest_decoded_frame_bytes =
                self.usage.largest_decoded_frame_bytes.max(bytes);
        }
        let required = self.usage.total_decoded_bytes.saturating_add(bytes);
        self.usage.total_decoded_bytes = required;
        (charge_work && self.charge_work(bytes))
            || self.trip(
                if blob {
                    GtsReadResource::DecodedBlobBytes
                } else {
                    GtsReadResource::DecodedFrameBytes
                },
                bytes,
                if blob {
                    self.limits.decoded_blob_bytes
                } else {
                    self.limits.decoded_frame_bytes
                },
            )
            || self.trip(
                GtsReadResource::TotalDecodedBytes,
                required,
                self.limits.total_decoded_bytes,
            )
    }
}

impl StreamingSink for ReadLimiter<'_> {
    fn continue_reading(&self) -> bool {
        if self.error.is_some() || self.cancelled.get() {
            return false;
        }
        let accepting = self.inner.continue_reading();
        if !accepting {
            self.cancelled.set(true);
        }
        accepting
    }

    fn item(&mut self, encoded_bytes: usize) {
        let required = self.usage.items.saturating_add(1);
        self.usage.items = required;
        self.usage.largest_encoded_frame_bytes =
            self.usage.largest_encoded_frame_bytes.max(encoded_bytes);
        let work = encoded_bytes.saturating_add(1);
        if (self.charge_encoded_work && self.charge_work(work))
            || self.trip(GtsReadResource::Items, required, self.limits.items)
            || self.trip(
                GtsReadResource::EncodedFrameBytes,
                encoded_bytes,
                self.limits.encoded_frame_bytes,
            )
        {
            return;
        }
        self.inner.item(encoded_bytes);
    }

    fn encoded_frame_refused(&mut self, required: usize, limit: usize) {
        self.usage.largest_encoded_frame_bytes =
            self.usage.largest_encoded_frame_bytes.max(required);
        if self.charge_work(required)
            || self.trip(
                GtsReadResource::EncodedFrameBytes,
                required,
                self.limits.encoded_frame_bytes,
            )
        {
            return;
        }
        self.inner.encoded_frame_refused(required, limit);
    }

    fn encoded_input_refused(&mut self, required: usize, limit: usize) {
        self.usage.encoded_input_bytes = required;
        if !self.trip(
            GtsReadResource::EncodedInputBytes,
            required,
            self.limits.encoded_input_bytes,
        ) {
            self.inner.encoded_input_refused(required, limit);
        }
    }

    fn decoded_payload(&mut self, blob: bool, bytes: usize) {
        if self.count_decoded(blob, bytes, true) {
            return;
        }
        self.inner.decoded_payload(blob, bytes);
    }

    fn transform_work(&mut self, units: usize) {
        if !self.charge_work(units) {
            self.inner.transform_work(units);
        }
    }

    fn decoded_payload_accounted(&mut self, blob: bool, bytes: usize) {
        if self.count_decoded(blob, bytes, false) {
            return;
        }
        self.inner.decoded_payload_accounted(blob, bytes);
    }

    fn transform_work_limit(&self) -> Option<usize> {
        let remaining = self.limits.work.saturating_sub(self.usage.work);
        let limit = remaining.min(self.inner.transform_work_limit().unwrap_or(usize::MAX));
        (limit != usize::MAX).then_some(limit)
    }

    fn decoded_payload_refused(&mut self, blob: bool, required: usize, limit: usize) {
        if self.count_decoded(blob, required, true) {
            return;
        }
        self.inner.decoded_payload_refused(blob, required, limit);
    }

    fn decoded_payload_refused_accounted(&mut self, blob: bool, required: usize, limit: usize) {
        if self.count_decoded(blob, required, false) {
            return;
        }
        self.inner
            .decoded_payload_refused_accounted(blob, required, limit);
    }

    fn retained_metadata(&mut self, entries: usize) {
        let added = entries.saturating_sub(self.current_metadata_entries);
        self.current_metadata_entries = entries;
        self.usage.metadata_entries = self.usage.metadata_entries.max(entries);
        if !self.charge_work(added)
            && !self.trip(
                GtsReadResource::MetadataEntries,
                entries,
                self.limits.metadata_entries,
            )
        {
            self.inner.retained_metadata(entries);
        }
    }

    fn metadata_entries(&mut self, additional: usize) {
        let required = self.current_metadata_entries.saturating_add(additional);
        self.current_metadata_entries = required;
        self.usage.metadata_entries = self.usage.metadata_entries.max(required);
        if !self.charge_work(additional)
            && !self.trip(
                GtsReadResource::MetadataEntries,
                required,
                self.limits.metadata_entries,
            )
        {
            self.inner.metadata_entries(additional);
        }
    }

    fn work(&mut self, units: usize) {
        if !self.charge_work(units) {
            self.inner.work(units);
        }
    }

    fn encoded_input_limit(&self) -> Option<usize> {
        let limit = self
            .limits
            .encoded_input_bytes
            .min(self.inner.encoded_input_limit().unwrap_or(usize::MAX));
        (limit != usize::MAX).then_some(limit)
    }

    fn encoded_frame_limit(&self) -> Option<usize> {
        let limit = self
            .limits
            .encoded_frame_bytes
            .min(self.inner.encoded_frame_limit().unwrap_or(usize::MAX));
        (limit != usize::MAX).then_some(limit)
    }

    fn frame_scratch(
        &mut self,
    ) -> Option<std::io::Result<Box<dyn purrdf_lex::cbor::ReplayStorage>>> {
        self.inner.frame_scratch()
    }

    fn frame(&mut self, ctx: FrameContext<'_>) {
        if !self.charge_work(1) {
            self.inner.frame(ctx);
        }
    }

    fn term(&mut self, segment_index: u64, term_id: usize, term: Arc<Term>) {
        let required = self.usage.terms.saturating_add(1);
        self.usage.terms = required;
        if !self.charge_work(1) && !self.trip(GtsReadResource::Terms, required, self.limits.terms) {
            self.inner.term(segment_index, term_id, term);
        }
    }

    fn stored_term(
        &mut self,
        segment_index: u64,
        term_id: usize,
        term: TermLease,
    ) -> Result<(), DictionaryError> {
        let required = self.usage.terms.saturating_add(1);
        self.usage.terms = required;
        if self.charge_work(1) || self.trip(GtsReadResource::Terms, required, self.limits.terms) {
            return Ok(());
        }
        self.inner.stored_term(segment_index, term_id, term)
    }

    fn quad(&mut self, segment_index: u64, quad: Quad) {
        if !self.count_row() {
            self.inner.quad(segment_index, quad);
        }
    }

    fn reifier(&mut self, segment_index: u64, reifier: ReifierRow) {
        if !self.count_row() {
            self.inner.reifier(segment_index, reifier);
        }
    }

    fn annotation(&mut self, segment_index: u64, annotation: AnnotationRow) {
        if !self.count_row() {
            self.inner.annotation(segment_index, annotation);
        }
    }

    fn suppression(&mut self, segment_index: u64, suppression: &Suppression) {
        if !self.count_row() {
            self.inner.suppression(segment_index, suppression);
        }
    }

    fn blob(&mut self, segment_index: u64, digest: &str, meta: Option<&Value>) {
        if !self.charge_work(1) {
            self.inner.blob(segment_index, digest, meta);
        }
    }

    fn blob_payload(&mut self, payload: BlobPayload<'_>) {
        if !self.charge_work(1) {
            self.inner.blob_payload(payload);
        }
    }

    fn blob_refused(&mut self, refusal: BlobRefusal<'_>) {
        if !self.charge_work(1) {
            self.inner.blob_refused(refusal);
        }
    }

    fn blob_encoded_limit(&self) -> Option<usize> {
        Self::effective_limit(
            self.limits.encoded_frame_bytes,
            self.inner.blob_encoded_limit(),
            usize::MAX,
        )
    }

    fn blob_decode_limit(&self) -> Option<usize> {
        let remaining = self
            .limits
            .total_decoded_bytes
            .saturating_sub(self.usage.total_decoded_bytes);
        let remaining_work = self.limits.work.saturating_sub(self.usage.work);
        Self::effective_limit(
            self.limits.decoded_blob_bytes,
            self.inner.blob_decode_limit(),
            remaining.min(remaining_work),
        )
    }

    fn eagerly_validate_declared_blobs(&self) -> bool {
        self.limits.decoded_blob_bytes != usize::MAX
            || self.limits.total_decoded_bytes != usize::MAX
            || self.limits.work != usize::MAX
    }

    fn embedded_blob_limit(&self) -> Option<usize> {
        let limit = self
            .limits
            .decoded_blob_bytes
            .min(self.inner.embedded_blob_limit().unwrap_or(usize::MAX));
        (limit != usize::MAX).then_some(limit)
    }

    fn embedded_blob(&mut self, bytes: usize) {
        self.usage.largest_decoded_blob_bytes = self.usage.largest_decoded_blob_bytes.max(bytes);
        if !self.trip(
            GtsReadResource::DecodedBlobBytes,
            bytes,
            self.limits.decoded_blob_bytes,
        ) {
            self.inner.embedded_blob(bytes);
        }
    }

    fn embedded_blob_refused(&mut self, required: usize, limit: usize) {
        self.usage.largest_decoded_blob_bytes = self.usage.largest_decoded_blob_bytes.max(required);
        if !self.trip(
            GtsReadResource::DecodedBlobBytes,
            required,
            self.limits.decoded_blob_bytes,
        ) {
            self.inner.embedded_blob_refused(required, limit);
        }
    }

    fn frame_decode_limit(&self) -> Option<usize> {
        let remaining = self
            .limits
            .total_decoded_bytes
            .saturating_sub(self.usage.total_decoded_bytes);
        let remaining_work = self.limits.work.saturating_sub(self.usage.work);
        Self::effective_limit(
            self.limits.decoded_frame_bytes,
            self.inner.frame_decode_limit(),
            remaining.min(remaining_work),
        )
    }

    fn opaque(&mut self, segment_index: u64, opaque: &OpaqueNode) {
        if !self.charge_work(1) {
            self.inner.opaque(segment_index, opaque);
        }
    }

    fn signature(&mut self, segment_index: u64, signature: &Signature) {
        if !self.charge_work(1) {
            self.inner.signature(segment_index, signature);
        }
    }

    fn diagnostic(&mut self, diagnostic: &Diagnostic) {
        let required = self.usage.diagnostics.saturating_add(1);
        self.usage.diagnostics = required;
        if !self.charge_work(1)
            && !self.trip(
                GtsReadResource::Diagnostics,
                required,
                self.limits.diagnostics,
            )
        {
            self.inner.diagnostic(diagnostic);
        }
    }

    fn segment_head(&mut self, segment_index: u64, head: &[u8]) {
        if !self.charge_work(1) {
            self.inner.segment_head(segment_index, head);
        }
    }

    fn streamable_layout(&mut self, segment_index: u64, info: &StreamableInfo) {
        if !self.charge_work(1) {
            self.inner.streamable_layout(segment_index, info);
        }
    }
}
