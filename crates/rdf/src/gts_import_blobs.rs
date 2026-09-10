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
    /// Whether the digest was computed from the payload, not merely claimed.
    ///
    /// A payload refused before any decode cannot have a container-declared
    /// digest checked against it, so a false value here is an assertion by the
    /// file, not an identity.
    pub digest_computed: bool,
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
    refused: Vec<RefusedOccurrence>,
    /// Digests some occurrence in this container carried bytes for, whether or
    /// not they were selected. Distinguishes a payload we declined to retain
    /// from one the container never held.
    inlined: BTreeSet<String>,
    /// Selected occurrences that named no payload and had none retained yet.
    ///
    /// Whether that is an external blob or an ordering problem cannot be decided
    /// mid-stream: the bytes may still arrive in a later frame. Resolved in
    /// [`Self::finish`], once the whole container has been read. A set, not a
    /// list: a repeated metadata-only row must not repeat its entry.
    unresolved: BTreeSet<String>,
    /// The final name each digest is published under, for selection only.
    ///
    /// Selection reads the *final* metadata, so an occurrence's verdict is only
    /// provisional until the container ends. Written only for digests with a
    /// verified identity: an unproved claim renaming a real digest would let a
    /// garbage frame decide what a caller's selector resolves to.
    declared: BTreeMap<String, SelectionKey>,
    /// Last admitted full declaration per digest, with its physical source.
    ///
    /// This is what an absent-metadata occurrence inherits ACROSS segment
    /// boundaries, since the reader's own inheritance resets per segment. It
    /// feeds only what the caller gets back, so the metadata retention ceiling
    /// applies per entry; an over-budget declaration REMOVES the entry rather
    /// than leaving a superseded one to answer in its place. Selection never
    /// reads this map.
    last_declared: BTreeMap<String, (Value, GtsBlobMetadataSource)>,
    frame: Option<BlobFrame>,
    /// Total retained bytes: selected payloads plus retained refusal metadata.
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

/// Longest value a selector may carry, so anything longer cannot match one.
const MAX_SELECTOR_BYTES: usize = 4096;

/// The part of a blob's public metadata that selection is allowed to read.
///
/// Kept apart from the metadata a caller gets back, because the two answer
/// different questions and only one of them is the caller's to bound. A
/// retention ceiling says how many bytes this import will KEEP; it must never
/// change how many blobs a selector COUNTS, or the size of a budget silently
/// decides which payload the caller receives.
///
/// Bounded by construction rather than by a ceiling: a selector is at most
/// [`MAX_SELECTOR_BYTES`], so a representation longer than that cannot equal any
/// selector, and discarding it is lossless for matching. Nothing here is ever
/// dropped to satisfy a budget.
#[derive(Clone, Debug, Default)]
struct SelectionKey {
    rep: Option<String>,
}

/// Project the selectable part out of a public metadata map.
///
/// Only `rep` is carried: those are the only two selector kinds, and `digest` is
/// matched against the blob's own identity rather than its metadata. A field a
/// selector cannot name has no business in a structure retained for matching.
fn selection_key(meta: Option<&Value>) -> SelectionKey {
    let Some(Value::Map(fields)) = meta else {
        return SelectionKey::default();
    };
    let rep = fields
        .iter()
        .find(|(key, _)| key.as_text() == Some("rep"))
        .and_then(|(_, value)| value.as_text())
        .filter(|text| text.len() <= MAX_SELECTOR_BYTES)
        .map(ToString::to_string);
    SelectionKey { rep }
}

/// A refused occurrence and the name it was published under.
///
/// The projection travels with the occurrence because a refusal without a
/// computable identity has no entry in the by-digest record, and losing its name
/// would let a ceiling decide that it never matched anything.
struct RefusedOccurrence {
    blob: GtsRefusedBlob,
    key: SelectionKey,
}

/// Whether a selector names a blob with this identity and this name.
///
/// `digest` is `None` when the blob has no PROVED identity: an unverified claim
/// is not an identity, so it can never satisfy a digest selector. Selector
/// values are validated non-empty, so absence and emptiness cannot collide.
fn matches(
    selector: GtsBlobSelector<'_>,
    digest: Option<&str>,
    key: Option<&SelectionKey>,
) -> bool {
    match selector {
        GtsBlobSelector::Digest(expected) => digest == Some(expected),
        GtsBlobSelector::Representation(expected) => {
            key.and_then(|key| key.rep.as_deref()) == Some(expected)
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
                value.is_empty()
                    || value.len() > MAX_SELECTOR_BYTES
                    || selectors[..index].contains(selector)
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
            unresolved: BTreeSet::new(),
            declared: BTreeMap::new(),
            last_declared: BTreeMap::new(),
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
        // The record only holds declarations seen BEFORE this event, but the
        // declaration that first makes a digest selected under a representation
        // selector is the one arriving now — matching the record alone would
        // leave this gate inert for exactly the copy it exists to bound.
        let incoming = selection_key(metadata);
        let metadata = metadata.or_else(|| {
            self.selected
                .get(digest)
                .and_then(|blob| blob.metadata.as_ref())
        });
        if self.selectors.iter().any(|selector| {
            matches(*selector, Some(digest), self.declared.get(digest))
                || matches(*selector, Some(digest), Some(&incoming))
        }) {
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
        if payload.metadata_declared {
            self.declared
                .insert(payload.digest.to_string(), selection_key(payload.metadata));
            // The full declaration is retention, so the metadata ceiling applies
            // here — and an inadmissible declaration must also EVICT any older
            // admitted one, or the superseded value would keep answering for a
            // digest whose current declaration could not be kept. Selection is
            // unaffected either way: it reads the projection above, which no
            // ceiling touches.
            let source = self
                .frame
                .as_ref()
                .filter(|frame| frame.segment_index == payload.segment_index)
                .map(|frame| GtsBlobMetadataSource {
                    segment_index: frame.segment_index,
                    frame_index: frame.frame_index,
                    frame_id: frame.frame_id.clone(),
                    frame_range: frame.range.clone(),
                    segment_head: Vec::new(),
                });
            match (payload.metadata, source) {
                (Some(meta), Some(source))
                    if check_metadata_bound(Some(meta), self.limits.max_metadata_bytes).is_ok() =>
                {
                    self.last_declared
                        .insert(payload.digest.to_string(), (meta.clone(), source));
                }
                _ => {
                    self.last_declared.remove(payload.digest);
                }
            }
        }
        let previous = self.selected.get(payload.digest);
        // The reader's per-occurrence metadata inherits only within a segment —
        // each segment starts a fresh lookaside — so an occurrence in a later
        // segment arrives bare even though a declaration exists earlier in the
        // container. The documented contract is that absent metadata preserves
        // the previous declaration *including across segment boundaries*, so a
        // bare occurrence inherits the last admitted declaration, and carries
        // that declaration's own physical source rather than inventing one.
        let inherited = payload
            .metadata
            .is_none()
            .then(|| self.last_declared.get(payload.digest))
            .flatten();
        let metadata = payload
            .metadata
            .or_else(|| inherited.map(|(value, _)| value))
            .or_else(|| previous.and_then(|blob| blob.metadata.as_ref()));
        let inherited_source = inherited.map(|(_, source)| source.clone());
        if !self.selectors.iter().any(|selector| {
            matches(
                *selector,
                Some(payload.digest),
                self.declared.get(payload.digest),
            )
        }) {
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
        // matching frame and no container reaches the error arm below. It is
        // kept as a fail-closed guard rather than an assumption, and is
        // deliberately not counted as a tested refusal.
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
            // Metadata inherited from an earlier declaration keeps THAT
            // declaration's source; a retained copy's source is the fallback.
            inherited_source.or_else(|| previous.and_then(|blob| blob.metadata_source.clone()))
        };
        let Some(wire_bytes) = payload.bytes else {
            let Some(previous) = self.selected.get_mut(payload.digest) else {
                // No bytes retained for this digest yet. Whether that means the
                // blob is external, or merely that its payload has not been read
                // yet, is not decidable here — a later frame in this same
                // container may still carry it, and the authoritative importer
                // accepts exactly that file. Defer to finish(), which sees the
                // whole container.
                self.unresolved.insert(payload.digest.to_string());
                return Ok(());
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
        // A refused occurrence never reaches `payload`, so its own declaration
        // must be recorded here or a staler one would answer for it — letting
        // the ceiling, rather than the container, decide what a representation
        // finally names. But only a PROVED identity may write a by-digest
        // record: an unproved claim writing under someone else's digest would
        // let one garbage frame rename a real, retained blob and change what a
        // caller's selector resolves to. An unproved refusal keeps its name on
        // the occurrence itself, where it can fail a selection closed but never
        // redirect one.
        //
        // Projected from the raw declaration, before the retention bound below:
        // what a blob is called must survive a ceiling that discards how it is
        // described.
        if refusal.digest_computed
            && let (Some(digest), Some(meta)) = (refusal.digest, refusal.metadata)
        {
            self.declared
                .insert(digest.to_string(), selection_key(Some(meta)));
        }
        // A budget bounds what this import retains, and a refused declaration is
        // retained like any other: it obeys the per-blob metadata ceiling AND
        // counts against the retained total, or a stream of refusals would
        // accumulate container-chosen bytes no ceiling ever saw. Selection is
        // untouched — the projection above and the key below were taken first.
        let per_blob_ok = refusal.metadata.is_some_and(|meta| {
            check_metadata_bound(Some(meta), self.limits.max_metadata_bytes).is_ok()
        });
        let retained_len = if per_blob_ok {
            refusal.metadata.map_or(0, encoded_metadata_len)
        } else {
            0
        };
        let within_total =
            self.total.saturating_add(retained_len) <= self.limits.max_total_decoded_bytes;
        let metadata = refusal.metadata.filter(|_| per_blob_ok && within_total);
        let detail = if metadata.is_none() && refusal.metadata.is_some() {
            format!(
                "{}; its public metadata exceeded the retention budgets and was not retained",
                refusal.detail
            )
        } else {
            self.total = self.total.saturating_add(retained_len);
            refusal.detail.to_string()
        };
        let key = selection_key(refusal.metadata);
        self.refused.push(RefusedOccurrence {
            key,
            blob: GtsRefusedBlob {
                digest: refusal.digest.map(ToString::to_string),
                digest_computed: refusal.digest_computed,
                metadata: metadata.cloned(),
                segment_index: refusal.segment_index,
                encoded_len: refusal.encoded_len,
                detail,
            },
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
        // A declaration held for a later segment to inherit needs its head too:
        // it is recorded while its own segment is still open, and an occurrence
        // that inherits it hands the caller that source verbatim.
        for (_, source) in self.last_declared.values_mut() {
            if source.segment_index == segment_index {
                source.segment_head = head.to_vec();
            }
        }
    }

    fn finish(self) -> Result<(Vec<GtsImportedBlob>, Vec<GtsRefusedBlob>), RdfDiagnostic> {
        // A selected occurrence that named no payload is only a problem if the
        // container never produced one. Deciding this here, rather than at the
        // occurrence, is what lets a metadata-only frame precede its own inline
        // payload — a file the authoritative importer accepts.
        for digest in &self.unresolved {
            if self.selected.contains_key(digest) {
                continue;
            }
            // The verdict at the occurrence was provisional: a selector matches
            // the FINAL metadata for a digest, and a later frame may have
            // retagged this one so that nothing names it any more. Refusing over
            // a blob the selection does not finally reach would reject a file
            // the authoritative importer accepts.
            let key = self.declared.get(digest);
            if !self
                .selectors
                .iter()
                .any(|selector| matches(*selector, Some(digest), key))
            {
                continue;
            }
            // A refusal is proof the container carried a payload under this
            // label. Calling it "held elsewhere" would let the byte budget decide
            // what this importer claims about the archive's contents, so presence
            // is answered from any refusal that named the digest. Identity is a
            // separate question, and the message says which one it can vouch for.
            if let Some(refusal) = self
                .refused
                .iter()
                .find(|refused| refused.blob.digest.as_deref() == Some(digest.as_str()))
            {
                let identity = if refusal.blob.digest_computed {
                    "is present in this container"
                } else {
                    "is declared by this container, though its identity was not verified before refusal,"
                };
                return Err(fail(
                    "rdf-ir-gts-blob-limit",
                    format!(
                        "blob {digest} {identity} and its {} encoded bytes exceeded this import's budget: {}",
                        refusal.blob.encoded_len, refusal.blob.detail
                    ),
                ));
            }
            if self.inlined.contains(digest) {
                return Err(fail(
                    "rdf-ir-gts-blob-selection-order",
                    format!(
                        "blob {digest} became selected without a payload; earlier unselected bytes are not retained"
                    ),
                ));
            }
            // "External" is a strong factual claim about the archive, and a
            // payload refused before identification could be these very bytes.
            // Externality is therefore only assertable when nothing unidentified
            // was refused; otherwise the truthful verdict is the budget's.
            let unidentified = self
                .refused
                .iter()
                .filter(|refused| !refused.blob.digest_computed)
                .count();
            if unidentified > 0 {
                return Err(fail(
                    "rdf-ir-gts-blob-limit",
                    format!(
                        "blob {digest} was selected but never retained, and {unidentified} payload(s) were refused before identification, so whether it is inline cannot be determined; raise the budgets or supply the payload"
                    ),
                ));
            }
            return Err(fail(
                "rdf-ir-gts-blob-external",
                format!(
                    "blob {digest} is external to this container; its bytes are held elsewhere and this importer returns only inline payloads"
                ),
            ));
        }

        for selector in &self.selectors {
            let retained = self
                .selected
                .values()
                .filter(|blob| {
                    matches(
                        *selector,
                        Some(&blob.digest),
                        self.declared.get(&blob.digest),
                    )
                })
                .count();
            // A refused candidate still counts. It was in the archive; only its
            // bytes were declined. Ignoring it here would make the byte budget a
            // selection rule, so a tighter ceiling could quietly resolve an
            // ambiguous selector to one arbitrary survivor.
            //
            // ONE RULE governs everything below: identity requires proof, naming
            // does not, and an unproved claim can fail a selection closed but
            // never resolve or redirect one. Concretely — a proved digest
            // matches digest selectors, reads the container-global final name,
            // and deduplicates against a retained copy of the same payload; an
            // unproved refusal matches only through its own occurrence's name,
            // never through anything keyed by the digest it merely claims.
            let refused: Vec<&RefusedOccurrence> = self
                .refused
                .iter()
                .filter(|blob| {
                    let proved = blob
                        .blob
                        .digest
                        .as_deref()
                        .filter(|_| blob.blob.digest_computed);
                    let key = proved
                        .and_then(|digest| self.declared.get(digest))
                        .unwrap_or(&blob.key);
                    matches(*selector, proved, Some(key))
                })
                .filter(|blob| match blob.blob.digest.as_deref() {
                    // A container may store one blob twice — compressed and not
                    // — and refuse only the copy that did not fit. A PROVED
                    // digest already retained is that same payload, not a second
                    // candidate.
                    Some(digest) if blob.blob.digest_computed => {
                        !self.selected.contains_key(digest)
                    }
                    // Unproved: may be a different payload. Count it and fail
                    // closed; silently returning one of two possible answers is
                    // the worse error.
                    _ => true,
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
                    .filter(|refused| refused.blob.digest.is_none())
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
                        blob.blob.encoded_len, blob.blob.detail
                    ),
                ));
            }
        }
        // The type permits an empty head or frame id and the doc calls both
        // "verified". The reader publishes a segment head at every segment close
        // and fires its frame event before any of that frame's rows, so neither
        // can be empty here; this is an invariant, not a refusal, and is checked
        // as one rather than shipped as a branch no input can reach.
        debug_assert!(
            self.selected
                .values()
                .all(|blob| !blob.segment_head.is_empty() && !blob.frame_id.is_empty()),
            "selected blob provenance must be populated before it is returned"
        );
        Ok((
            self.selected.into_values().collect(),
            self.refused
                .into_iter()
                .map(|refused| refused.blob)
                .collect(),
        ))
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

/// Exact CBOR-encoded size of a metadata map, without materializing the bytes.
fn encoded_metadata_len(meta: &Value) -> usize {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter(0);
    // Counting cannot fail; a serialization error simply under-counts, and the
    // per-blob bound has already accepted this value.
    let _ = ciborium::ser::into_writer(meta, &mut counter);
    counter.0
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
