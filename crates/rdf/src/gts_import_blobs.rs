// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Explicit, bounded blob selection alongside the authoritative native import.

use std::collections::BTreeMap;
use std::sync::Arc;

use ciborium::Value;
use purrdf_gts::model::ByteRange;
use purrdf_gts::reader::{BlobPayload, FrameContext};

use crate::{GtsBundle, RdfDiagnostic};

/// A required inline blob, selected independently of RDF statement order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GtsBlobSelector<'a> {
    /// Match the exact content digest.
    Digest(&'a str),
    /// Match the final public metadata's `rep` text value.
    Representation(&'a str),
}

/// Explicit retention and decode budgets for selected inline blobs.
///
/// These bounds cover selected payloads and their metadata, not the native RDF
/// dataset, input file, reader frame buffers or codec working memory. Every
/// decoded intermediate is bounded, including compression-chain intermediates.
#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub struct GtsBlobLimits {
    /// Maximum selected encoded payload bytes in one occurrence.
    pub max_encoded_bytes: usize,
    /// Maximum decoded bytes in one blob or intermediate transform buffer.
    pub max_decoded_bytes: usize,
    /// Maximum sum of decoded bytes retained across selected unique digests.
    pub max_total_decoded_bytes: usize,
    /// Maximum CBOR-encoded public metadata bytes retained for one blob.
    pub max_metadata_bytes: usize,
}

impl GtsBlobLimits {
    /// Set payload budgets; metadata defaults to a bounded 64 KiB per blob.
    ///
    /// The encoded limit initially equals `max_decoded_bytes`; callers may set
    /// it separately when incompressible or chained encodings need more room.
    #[must_use]
    pub const fn new(max_decoded_bytes: usize, max_total_decoded_bytes: usize) -> Self {
        Self {
            max_encoded_bytes: max_decoded_bytes,
            max_decoded_bytes,
            max_total_decoded_bytes,
            max_metadata_bytes: 64 * 1024,
        }
    }
}

/// Physical source of the effective public metadata for a selected blob.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct GtsBlobMetadataSource {
    /// Segment containing the last explicit public metadata declaration.
    pub segment_index: usize,
    /// Frame containing that declaration.
    pub frame_index: usize,
    /// Verified frame content id.
    pub frame_id: Vec<u8>,
    /// Original source range of that frame.
    pub frame_range: ByteRange,
    /// Verified completed head of the metadata's segment.
    pub segment_head: Vec<u8>,
}

/// Authenticated selected bytes and exact public metadata from one GTS import.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct GtsImportedBlob {
    /// Content digest, checked against the decoded bytes.
    pub digest: String,
    /// Final effective public metadata, preserved without a lossy projection.
    pub metadata: Option<Value>,
    /// Exact source of effective metadata, including metadata-only updates.
    pub metadata_source: Option<GtsBlobMetadataSource>,
    /// Decoded payload, shared without copying by retained consumer views.
    pub bytes: Arc<[u8]>,
    /// Zero-based segment containing the selected payload occurrence.
    pub segment_index: usize,
    /// Zero-based frame within that segment.
    pub frame_index: usize,
    /// Exact frame content id verified by the reader.
    pub frame_id: Vec<u8>,
    /// Frame's original byte range in the imported source.
    pub frame_range: ByteRange,
    /// Verified completed head of the payload's segment.
    pub segment_head: Vec<u8>,
}

/// Native RDF 1.2 bundle and its explicitly requested inline blobs.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct GtsImportWithBlobs {
    /// The same scoped native dataset and envelope as [`crate::import_gts_events`].
    pub bundle: GtsBundle,
    /// Selected unique digests in deterministic digest order.
    pub blobs: Vec<GtsImportedBlob>,
}

/// Import the native dataset and retain required blobs during the same GTS read.
///
/// `selectors` names exact digests or public `rep` values, never RDF predicates.
/// Each selector must resolve to exactly one final blob; multiple selectors may
/// name the same blob, which is decoded and retained once per occurrence. At most
/// 1024 nonempty, distinct selectors of at most 4096 bytes each are accepted.
/// Unselected payloads are neither copied nor decoded by this collector. Blobs
/// without a public digest require the reader to decode before selection and
/// obey both the encoded and decoded per-blob limits before eager decoding,
/// including when ultimately unselected. Snapshot-entry bounds cover their
/// embedded blob bytes; enclosing RDF frame decoding remains the reader's scope.
///
/// Later public metadata replaces earlier metadata for that digest; absent
/// metadata preserves it, including across segment boundaries. A newly selected
/// metadata-only occurrence is explicitly refused: this bounded one-pass API
/// does not retain unrelated earlier payloads to recover that ordering. A selected
/// metadata-only update can reuse bytes already retained for the same digest.
/// All legacy reader diagnostics, term resolution and per-segment blank scopes
/// retain [`crate::import_gts_events`]'s behavior. Existing importers do not opt
/// into this retention or its additional payload-integrity checks.
///
/// # Errors
/// Returns a diagnostic for invalid selection, missing or ambiguous final blobs,
/// selected malformed metadata, absent payloads, corrupt decoded digests, reader
/// or codec failures, or a retention/decode bound violation.
pub fn import_gts_events_with_blobs(
    bytes: &[u8],
    selectors: &[GtsBlobSelector<'_>],
    limits: GtsBlobLimits,
) -> Result<GtsImportWithBlobs, RdfDiagnostic> {
    let collector = BlobCollector::new(selectors, limits)?;
    let (bundle, collector) =
        crate::gts_import_sink::import_with_collector(bytes, Some(collector))?;
    let blobs = collector
        .expect("selected importer retains its collector")
        .finish()?;
    Ok(GtsImportWithBlobs { bundle, blobs })
}

#[derive(Clone, Debug)]
struct BlobFrame {
    segment_index: usize,
    frame_index: usize,
    frame_id: Vec<u8>,
    range: ByteRange,
}

pub(crate) struct BlobCollector<'a> {
    selectors: Vec<GtsBlobSelector<'a>>,
    limits: GtsBlobLimits,
    selected: BTreeMap<String, GtsImportedBlob>,
    frame: Option<BlobFrame>,
    total: usize,
}

fn fail(code: &str, message: impl Into<String>) -> RdfDiagnostic {
    RdfDiagnostic::error(code, message)
}

fn metadata_field<'a>(
    meta: Option<&'a Value>,
    key: &str,
) -> Result<Option<&'a Value>, RdfDiagnostic> {
    let Some(Value::Map(fields)) = meta else {
        return Ok(None);
    };
    let mut values = fields
        .iter()
        .filter(|(name, _)| name.as_text() == Some(key))
        .map(|(_, value)| value);
    let value = values.next();
    if values.next().is_some() {
        return Err(fail(
            "rdf-ir-gts-blob-metadata",
            format!("selected blob repeats metadata key {key}"),
        ));
    }
    Ok(value)
}

fn matches(selector: GtsBlobSelector<'_>, digest: &str, meta: Option<&Value>) -> bool {
    match selector {
        GtsBlobSelector::Digest(expected) => expected == digest,
        GtsBlobSelector::Representation(expected) => {
            let Some(Value::Map(fields)) = meta else {
                return false;
            };
            fields.iter().any(|(key, value)| {
                key.as_text() == Some("rep") && value.as_text() == Some(expected)
            })
        }
    }
}

impl<'a> BlobCollector<'a> {
    fn new(
        selectors: &[GtsBlobSelector<'a>],
        limits: GtsBlobLimits,
    ) -> Result<Self, RdfDiagnostic> {
        if selectors.len() > 1024
            || selectors.iter().enumerate().any(|(index, selector)| {
                let (GtsBlobSelector::Digest(value) | GtsBlobSelector::Representation(value)) =
                    selector;
                value.is_empty() || value.len() > 4096 || selectors[..index].contains(selector)
            })
        {
            return Err(fail(
                "rdf-ir-gts-blob-selection",
                "selectors must be distinct, nonempty, bounded exact identities",
            ));
        }
        Ok(Self {
            selectors: selectors.to_vec(),
            limits,
            selected: BTreeMap::new(),
            frame: None,
            total: 0,
        })
    }

    pub(crate) const fn encoded_limit(&self) -> usize {
        self.limits.max_encoded_bytes
    }

    pub(crate) fn check_metadata_before_retention(
        &self,
        digest: &str,
        metadata: Option<&Value>,
    ) -> Result<(), RdfDiagnostic> {
        let metadata = metadata.or_else(|| {
            self.selected
                .get(digest)
                .and_then(|blob| blob.metadata.as_ref())
        });
        if self
            .selectors
            .iter()
            .any(|selector| matches(*selector, digest, metadata))
        {
            check_metadata_bound(metadata, self.limits.max_metadata_bytes)?;
            validate_metadata(metadata, digest)?;
        }
        Ok(())
    }

    pub(crate) const fn decode_limit(&self) -> usize {
        self.limits.max_decoded_bytes
    }

    pub(crate) fn frame(&mut self, ctx: FrameContext<'_>) {
        self.frame = Some(BlobFrame {
            segment_index: ctx.segment_index,
            frame_index: ctx.frame_index,
            frame_id: ctx.content_id.to_vec(),
            range: ctx.range,
        });
    }

    pub(crate) fn payload(&mut self, payload: BlobPayload<'_>) -> Result<(), RdfDiagnostic> {
        let previous = self.selected.get(payload.digest);
        let metadata = payload
            .metadata
            .or_else(|| previous.and_then(|blob| blob.metadata.as_ref()));
        if !self
            .selectors
            .iter()
            .any(|selector| matches(*selector, payload.digest, metadata))
        {
            if let Some(old) = self.selected.remove(payload.digest) {
                self.total -= old.bytes.len();
            }
            return Ok(());
        }
        if payload.payload_present && payload.bytes.is_none() {
            return Err(fail(
                "rdf-ir-gts-blob-payload",
                "selected blob payload field is not a byte string",
            ));
        }
        let metadata = bounded_metadata(metadata, self.limits.max_metadata_bytes)?;
        validate_metadata(metadata.as_ref(), payload.digest)?;
        let frame = self
            .frame
            .as_ref()
            .filter(|frame| frame.segment_index == payload.segment_index)
            .ok_or_else(|| {
                fail(
                    "rdf-ir-gts-blob-provenance",
                    "selected blob has no matching source frame",
                )
            })?;
        let metadata_source = if payload.metadata_declared {
            Some(GtsBlobMetadataSource {
                segment_index: frame.segment_index,
                frame_index: frame.frame_index,
                frame_id: frame.frame_id.clone(),
                frame_range: frame.range.clone(),
                segment_head: Vec::new(),
            })
        } else {
            previous.and_then(|blob| blob.metadata_source.clone())
        };
        let Some(wire_bytes) = payload.bytes else {
            let Some(previous) = self.selected.get_mut(payload.digest) else {
                return Err(fail(
                    "rdf-ir-gts-blob-selection-order",
                    format!(
                        "blob {} became selected without a payload; earlier unselected bytes are not retained",
                        payload.digest
                    ),
                ));
            };
            validate_length(metadata.as_ref(), previous.bytes.len())?;
            previous.metadata = metadata;
            previous.metadata_source = metadata_source;
            return Ok(());
        };
        if payload.encoded_len > self.limits.max_encoded_bytes {
            return Err(fail(
                "rdf-ir-gts-blob-limit",
                "selected encoded blob exceeds its byte budget",
            ));
        }
        // A representation can temporarily match many different digests before
        // later metadata updates. Bound that transient retention as well.
        if previous.is_none() && self.selected.len() >= 1024 {
            return Err(fail(
                "rdf-ir-gts-blob-limit",
                "more than 1024 selected unique blob digests",
            ));
        }
        let old_len = previous.map_or(0, |blob| blob.bytes.len());
        let other_bytes = self.total - old_len;
        let remaining = self
            .limits
            .max_total_decoded_bytes
            .saturating_sub(other_bytes);
        let limit = self.limits.max_decoded_bytes.min(remaining);
        let bytes = purrdf_gts::codec::decode_chain_bounded(payload.codecs, wire_bytes, limit)
            .map_err(|error| fail("rdf-ir-gts-blob-decode", error.to_string()))?;
        let digest = purrdf_gts::wire::digest_str(&bytes);
        if digest != payload.digest {
            return Err(fail(
                "rdf-ir-gts-blob-digest",
                format!(
                    "selected blob digest {digest} differs from declared {}",
                    payload.digest
                ),
            ));
        }
        validate_length(metadata.as_ref(), bytes.len())?;
        self.total = other_bytes + bytes.len();
        self.selected.insert(
            digest.clone(),
            GtsImportedBlob {
                digest,
                metadata,
                metadata_source,
                bytes: bytes.into(),
                segment_index: payload.segment_index,
                frame_index: frame.frame_index,
                frame_id: frame.frame_id.clone(),
                frame_range: frame.range.clone(),
                segment_head: Vec::new(),
            },
        );
        Ok(())
    }

    pub(crate) fn segment_head(&mut self, segment_index: usize, head: &[u8]) {
        for blob in self.selected.values_mut() {
            if blob.segment_index == segment_index {
                blob.segment_head = head.to_vec();
            }
            if let Some(source) = &mut blob.metadata_source
                && source.segment_index == segment_index
            {
                source.segment_head = head.to_vec();
            }
        }
    }

    fn finish(self) -> Result<Vec<GtsImportedBlob>, RdfDiagnostic> {
        for selector in self.selectors {
            let count = self
                .selected
                .values()
                .filter(|blob| matches(selector, &blob.digest, blob.metadata.as_ref()))
                .count();
            if count != 1 {
                return Err(fail(
                    "rdf-ir-gts-blob-selection",
                    format!(
                        "required selector {selector:?} resolves to {count} blobs; expected exactly one"
                    ),
                ));
            }
        }
        Ok(self.selected.into_values().collect())
    }
}

fn validate_metadata(meta: Option<&Value>, digest: &str) -> Result<(), RdfDiagnostic> {
    if let Some(meta) = meta {
        if !matches!(meta, Value::Map(_)) {
            return Err(fail(
                "rdf-ir-gts-blob-metadata",
                "selected blob public metadata must be a map",
            ));
        }
        if metadata_field(Some(meta), "digest")?.is_some()
            && purrdf_gts::reader::public_blob_digest(meta).as_deref() != Some(digest)
        {
            return Err(fail(
                "rdf-ir-gts-blob-digest",
                "selected blob metadata digest differs from its identity",
            ));
        }
    }
    for key in ["rep", "mt"] {
        if let Some(value) = metadata_field(meta, key)? {
            value.as_text().ok_or_else(|| {
                fail(
                    "rdf-ir-gts-blob-metadata",
                    format!("selected blob metadata {key} must be text"),
                )
            })?;
        }
    }
    Ok(())
}

fn validate_length(meta: Option<&Value>, length: usize) -> Result<(), RdfDiagnostic> {
    if let Some(value) = metadata_field(meta, "len")? {
        let declared = value
            .as_integer()
            .and_then(|value| usize::try_from(value).ok());
        if declared != Some(length) {
            return Err(fail(
                "rdf-ir-gts-blob-length",
                "selected blob declared length differs from decoded length",
            ));
        }
    }
    Ok(())
}

fn bounded_metadata(meta: Option<&Value>, limit: usize) -> Result<Option<Value>, RdfDiagnostic> {
    check_metadata_bound(meta, limit)?;
    Ok(meta.cloned())
}

fn check_metadata_bound(meta: Option<&Value>, limit: usize) -> Result<(), RdfDiagnostic> {
    struct Counter {
        remaining: usize,
    }
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.remaining = self.remaining.checked_sub(bytes.len()).ok_or_else(|| {
                std::io::Error::other("selected blob metadata exceeds its byte budget")
            })?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    if let Some(meta) = meta {
        ciborium::ser::into_writer(meta, Counter { remaining: limit })
            .map_err(|error| fail("rdf-ir-gts-blob-limit", error.to_string()))?;
    }
    Ok(())
}
