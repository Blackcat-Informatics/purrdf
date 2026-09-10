// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Explicit, bounded blob selection alongside the authoritative native import.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use ciborium::Value;
use purrdf_gts::model::ByteRange;
use purrdf_gts::reader::{BlobPayload, BlobRefusal, FrameContext};

use crate::{GtsBundle, RdfDiagnostic};

/// A required inline blob, selected independently of RDF statement order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GtsBlobSelector<'a> {
    /// Match the exact content digest, in the canonical `blake3:<hex>` spelling.
    ///
    /// The reader normalizes the wire's three published spellings to this one
    /// before matching, so a bare hex selector resolves to nothing rather than
    /// to the blob it looks like. Use [`purrdf_gts::wire::digest_str`].
    Digest(&'a str),
    /// Match the final public metadata's `rep` text value.
    Representation(&'a str),
}

/// Explicit retention and decode budgets for selected inline blobs.
///
/// These bounds cover selected payloads, their metadata, and the frames that
/// carry them — not the native RDF dataset, the input file, reader frame
/// buffers or codec working memory. Every decoded intermediate is bounded,
/// including compression-chain intermediates and the enclosing frame decode
/// that a `snapshot` uses to reach its embedded blobs.
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
    /// Maximum decoded bytes for one enclosing, non-blob frame.
    ///
    /// A `snapshot` frame carries embedded blob bytes inside an RDF frame, so
    /// the blob ceilings alone would leave the decode that actually spends the
    /// memory unbounded. This bounds that decode. It applies to every non-blob
    /// frame, so it must be large enough for the caller's legitimate RDF frames;
    /// the default matches the 1 GiB retained-payload ceiling this workspace
    /// already uses for immutable views.
    pub max_frame_decoded_bytes: usize,
}

/// Default per-blob public-metadata ceiling: 64 KiB.
pub const DEFAULT_MAX_METADATA_BYTES: usize = 64 * 1024;

/// Default enclosing-frame decode ceiling: 1 GiB.
///
/// Matches the retained-payload ceiling `purrdf-core` already uses for immutable
/// views, so one workspace-wide number governs "how many bytes may one thing
/// cost". Finite by design: an unbounded frame decode is how a compressed
/// snapshot defeats every blob ceiling beneath it.
pub const DEFAULT_MAX_FRAME_DECODED_BYTES: usize = 1_073_741_824;

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
            max_metadata_bytes: DEFAULT_MAX_METADATA_BYTES,
            max_frame_decoded_bytes: DEFAULT_MAX_FRAME_DECODED_BYTES,
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

/// One payload a byte budget refused, reported rather than silently dropped.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct GtsRefusedBlob {
    /// Declared content digest, absent when the container published none.
    ///
    /// Refusing precedes the decode that would compute an identity, so an
    /// undeclared digest is unknowable here. Such a payload is matchable by
    /// representation, never by digest.
    pub digest: Option<String>,
    /// Public metadata declared at the refused occurrence.
    pub metadata: Option<Value>,
    /// Zero-based segment containing the refused occurrence.
    pub segment_index: usize,
    /// Encoded byte length that was refused.
    pub encoded_len: usize,
    /// Which ceiling fired, in the reader's own words.
    pub detail: String,
}

/// Native RDF 1.2 bundle and its explicitly requested inline blobs.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct GtsImportWithBlobs {
    /// The same scoped native dataset as [`crate::import_gts_events`].
    ///
    /// The envelope matches too, with one documented exception: a payload this
    /// import's budget refused is recorded as an opaque node with reason
    /// `over-budget` instead of a blob record, because the digest that would
    /// identify it is only computable by performing the decode the budget
    /// declined. Such payloads are listed in [`Self::refused`].
    pub bundle: GtsBundle,
    /// Selected unique digests in deterministic digest order.
    pub blobs: Vec<GtsImportedBlob>,
    /// Payloads the budget refused, in encounter order.
    ///
    /// Present so a successful import never hides a refusal: a caller that set a
    /// ceiling can see exactly what it cost them.
    pub refused: Vec<GtsRefusedBlob>,
}

/// Import the native dataset and retain required blobs during the same GTS read.
///
/// `selectors` names exact digests or public `rep` values, never RDF predicates.
/// Each selector must resolve to exactly one final blob; multiple selectors may
/// name the same blob, which is decoded and retained once per occurrence. At most
/// 1024 nonempty, distinct selectors of at most 4096 bytes each are accepted.
/// Unselected payloads are neither copied nor decoded by this collector. A blob
/// without a public digest cannot be selected before it is decoded, so it obeys
/// the encoded and decoded per-blob ceilings first; exceeding either refuses
/// that payload without failing the import, and the refusal is reported through
/// [`GtsImportWithBlobs::refused`] and still counts as a selection candidate.
/// Snapshot entries obey the same per-blob ceiling, and the enclosing frame
/// decode obeys [`GtsBlobLimits::max_frame_decoded_bytes`].
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
/// or codec failures, or a retention/decode bound violation. Selecting a blob
/// the container declares as external — published digest, no payload field —
/// returns `rdf-ir-gts-blob-external`: its bytes are held outside this file, so
/// no inline read can produce them.
pub fn import_gts_events_with_blobs(
    bytes: &[u8],
    selectors: &[GtsBlobSelector<'_>],
    limits: GtsBlobLimits,
) -> Result<GtsImportWithBlobs, RdfDiagnostic> {
    let collector = BlobCollector::new(selectors, limits)?;
    let (bundle, collector) =
        crate::gts_import_sink::import_with_collector(bytes, Some(collector))?;
    // The fold hands the collector straight back, so this cannot be None today.
    // It is still a hard error rather than a panic: an internal invariant that
    // fails should surface as a diagnostic on a Result-returning API, not abort
    // the caller's process.
    let Some(collector) = collector else {
        return Err(fail(
            "rdf-ir-gts-import-internal",
            "the selected importer did not retain its blob collector",
        ));
    };
    let (blobs, refused) = collector.finish()?;
    Ok(GtsImportWithBlobs {
        bundle,
        blobs,
        refused,
    })
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
    refused: Vec<GtsRefusedBlob>,
    /// Digests some occurrence in this container carried bytes for, whether or
    /// not they were selected. Distinguishes a payload we declined to retain
    /// from one the container never held.
    inlined: BTreeSet<String>,
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
            refused: Vec::new(),
            inlined: BTreeSet::new(),
            frame: None,
            total: 0,
        })
    }

    pub(crate) const fn encoded_limit(&self) -> usize {
        self.limits.max_encoded_bytes
    }

    /// Bound and validate a selected blob's metadata before the sink keeps it.
    ///
    /// Deliberately runs earlier than, and in addition to, the check inside
    /// [`Self::payload`]. The importer deep-copies public metadata into its
    /// lookaside when the legacy blob event arrives, which is before any payload
    /// event; bounding it only in `payload` would let an unbounded metadata map
    /// be copied first and bounded afterwards, which is not a bound.
    ///
    /// The two see different snapshots on purpose. This one inherits from what
    /// has been *retained* so far, because that is all that exists at this point
    /// in the read; `payload` inherits from the occurrence it is processing,
    /// which is more specific. Where they disagree, `payload` is authoritative:
    /// it decides what is returned to the caller, while this one exists solely
    /// to keep the earlier copy bounded.
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

    pub(crate) const fn frame_decode_limit(&self) -> usize {
        self.limits.max_frame_decoded_bytes
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
        if payload.payload_present {
            self.inlined.insert(payload.digest.to_string());
        }
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
                // Release builds leave overflow checks off, so a future break of
                // the "total is the sum of retained lengths" invariant would wrap
                // to near usize::MAX and silently disable the total budget,
                // surfacing as spurious decode refusals far from the cause.
                debug_assert!(
                    self.total >= old.bytes.len(),
                    "retained-byte total underflow"
                );
                self.total = self.total.saturating_sub(old.bytes.len());
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
        // The reader fires its frame event for every frame before any of that
        // frame's rows, with the same segment index, so a payload always has a
        // matching frame. The refusal stays as the shared provenance check in
        // finish(), where it is reachable and tested.
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
                // An occurrence with no payload field at all, publishing a
                // digest, is an EXTERNAL blob: the spec places its bytes outside
                // this container. That is a different fact from a metadata-only
                // update to a payload we declined to retain, and reporting it as
                // one claims something false about retention ordering.
                // No occurrence anywhere in this container carried bytes for
                // this digest, so the spec's external form is the only reading:
                // the payload lives outside the file. Reporting that as a
                // retention-ordering problem would assert something false.
                if !self.inlined.contains(payload.digest) {
                    return Err(fail(
                        "rdf-ir-gts-blob-external",
                        format!(
                            "blob {} is external to this container; its bytes are held elsewhere and this importer returns only inline payloads",
                            payload.digest
                        ),
                    ));
                }
                return Err(fail(
                    "rdf-ir-gts-blob-selection-order",
                    format!(
                        "blob {} became selected without a payload; earlier unselected bytes are not retained",
                        payload.digest
                    ),
                ));
            };
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
        debug_assert!(self.total >= old_len, "retained-byte total underflow");
        let other_bytes = self.total.saturating_sub(old_len);
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

    /// Record a payload the reader's ceiling refused before decoding it.
    ///
    /// Kept as a selection candidate, not discarded. A budget bounds what this
    /// import *retains*; letting a refusal remove a candidate would let the
    /// budget decide which blob a selector resolves to, so a tight ceiling could
    /// silently turn an ambiguous match into a confident wrong answer.
    pub(crate) fn refused(&mut self, refusal: BlobRefusal<'_>) {
        self.refused.push(GtsRefusedBlob {
            digest: refusal.digest.map(ToString::to_string),
            metadata: refusal.metadata.cloned(),
            segment_index: refusal.segment_index,
            encoded_len: refusal.encoded_len,
            detail: refusal.detail.to_string(),
        });
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

    fn finish(self) -> Result<(Vec<GtsImportedBlob>, Vec<GtsRefusedBlob>), RdfDiagnostic> {
        for selector in &self.selectors {
            let retained = self
                .selected
                .values()
                .filter(|blob| matches(*selector, &blob.digest, blob.metadata.as_ref()))
                .count();
            // A refused candidate still counts. It was in the archive; only its
            // bytes were declined. Ignoring it here would make the byte budget a
            // selection rule, so a tighter ceiling could quietly resolve an
            // ambiguous selector to one arbitrary survivor.
            let refused: Vec<&GtsRefusedBlob> = self
                .refused
                .iter()
                .filter(|blob| {
                    matches(
                        *selector,
                        blob.digest.as_deref().unwrap_or_default(),
                        blob.metadata.as_ref(),
                    )
                })
                .collect();
            let count = retained + refused.len();
            if count != 1 {
                // A payload refused before it could be hashed has no identity to
                // match a digest selector against, so say that rather than let
                // "resolves to 0" imply the archive did not contain it.
                let unidentified = self
                    .refused
                    .iter()
                    .filter(|blob| blob.digest.is_none())
                    .count();
                let note = if unidentified == 0 {
                    String::new()
                } else {
                    format!(
                        "; {unidentified} further payload(s) were refused by this import's budget before a digest could be computed, so they could not be matched"
                    )
                };
                return Err(fail(
                    "rdf-ir-gts-blob-selection",
                    format!(
                        "required selector {selector:?} resolves to {count} blobs; expected exactly one{note}"
                    ),
                ));
            }
            // Exactly one match, and it is the one we declined to decode. Say so
            // precisely: reporting "resolves to 0 blobs" would be a false claim
            // about an archive that does contain the payload.
            if let [blob] = refused.as_slice() {
                return Err(fail(
                    "rdf-ir-gts-blob-limit",
                    format!(
                        "required selector {selector:?} matches a payload of {} encoded bytes that this import's budget refused: {}",
                        blob.encoded_len, blob.detail
                    ),
                ));
            }
        }
        // The type permits an empty head or frame id, and the doc calls both
        // "verified". Assert rather than trust: a silently empty provenance
        // field is indistinguishable from a real one at the call site.
        for blob in self.selected.values() {
            if blob.segment_head.is_empty() || blob.frame_id.is_empty() {
                return Err(fail(
                    "rdf-ir-gts-blob-provenance",
                    format!(
                        "selected blob {} lacks the verified frame or segment provenance it reports",
                        blob.digest
                    ),
                ));
            }
        }
        Ok((self.selected.into_values().collect(), self.refused))
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
