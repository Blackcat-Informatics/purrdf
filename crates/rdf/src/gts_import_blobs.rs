// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit, bounded blob selection alongside the authoritative native import.

use std::sync::Arc;

use purrdf_gts::model::ByteRange;
use purrdf_gts::reader::{BlobPayload, BlobRefusal, FrameContext};
use purrdf_lex::allocation::{Memory, OrderedTable, Resident, StorageError};
use purrdf_lex::cbor::Value;

use crate::{GtsBundle, RdfDiagnostic};

mod error;
mod export;
pub use error::GtsBlobRetentionRefusal;
pub use export::{
    GtsBlobOmission, GtsBlobOmissionKind, GtsInlineExport, import_gts_inline_for_export,
};

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
    /// Maximum combined selected decoded payload and refused-metadata charge.
    /// Ordinary selected metadata and index storage are outside this charge.
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
    /// No chosen byte ceiling. Address-space and physical storage refusals still
    /// apply. Each field may independently be set to `usize::MAX` with the same
    /// meaning; zero still refuses the first byte of that kind.
    pub const UNBOUNDED: Self = Self {
        max_encoded_bytes: usize::MAX,
        max_decoded_bytes: usize::MAX,
        max_total_decoded_bytes: usize::MAX,
        max_metadata_bytes: usize::MAX,
        max_frame_decoded_bytes: usize::MAX,
    };

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

/// Logical retained bytes and input-derived cardinalities of one selected import.
/// These are payload and existing refused-metadata charges, not dataset, index,
/// allocator capacity, frame-buffer or codec-workspace measurements.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GtsBlobUsage {
    /// Decoded bytes currently held by selected unique digests.
    pub retained_payload_bytes: usize,
    /// Encoded metadata bytes held by recorded refused occurrences.
    pub retained_refusal_metadata_bytes: usize,
    /// Largest simultaneous selected decoded-payload charge.
    pub peak_payload_bytes: usize,
    /// Largest simultaneous sum of both retained byte charges.
    pub peak_retained_bytes: usize,
    /// Largest simultaneous number of retained selected unique digests.
    pub peak_selected_digests: usize,
}

impl GtsBlobUsage {
    fn retained_bytes(self) -> Result<usize, StorageError> {
        self.retained_payload_bytes
            .checked_add(self.retained_refusal_metadata_bytes)
            .ok_or(StorageError::SizeOverflow)
    }

    fn replace_payload(
        &mut self,
        previous: usize,
        next: usize,
        digests: usize,
    ) -> Result<(), StorageError> {
        self.retained_payload_bytes = self
            .retained_payload_bytes
            .checked_sub(previous)
            .and_then(|bytes| bytes.checked_add(next))
            .ok_or(StorageError::SizeOverflow)?;
        self.observe(digests)
    }

    fn retain_refusal_metadata(
        &mut self,
        bytes: usize,
        digests: usize,
    ) -> Result<(), StorageError> {
        self.retained_refusal_metadata_bytes = self
            .retained_refusal_metadata_bytes
            .checked_add(bytes)
            .ok_or(StorageError::SizeOverflow)?;
        self.observe(digests)
    }

    fn observe(&mut self, digests: usize) -> Result<(), StorageError> {
        self.peak_payload_bytes = self.peak_payload_bytes.max(self.retained_payload_bytes);
        self.peak_retained_bytes = self.peak_retained_bytes.max(self.retained_bytes()?);
        self.peak_selected_digests = self.peak_selected_digests.max(digests);
        Ok(())
    }
}

/// Physical source of the effective public metadata for a selected blob.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct GtsBlobMetadataSource {
    /// Segment containing the last explicit public metadata declaration.
    pub segment_index: u64,
    /// Frame containing that declaration.
    pub frame_index: u64,
    /// Verified frame content id.
    pub frame_id: Vec<u8>,
    /// Original source range of that frame.
    pub frame_range: ByteRange,
    /// Verified completed head of the metadata's segment.
    pub segment_head: Vec<u8>,
}

impl GtsBlobMetadataSource {
    fn try_copy(&self) -> Result<Self, StorageError> {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        Ok(Self {
            segment_index: self.segment_index,
            frame_index: self.frame_index,
            frame_id: memory.collect(self.frame_id.iter().copied())?,
            frame_range: self.frame_range.clone(),
            segment_head: memory.collect(self.segment_head.iter().copied())?,
        })
    }
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
    pub segment_index: u64,
    /// Zero-based frame within that segment.
    pub frame_index: u64,
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
    pub segment_index: u64,
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
    /// Exact logical byte charges and transient selected cardinality observed.
    pub usage: GtsBlobUsage,
}

/// Import the native dataset and retain required blobs during the same GTS read.
///
/// `selectors` names exact digests or public `rep` values, never RDF predicates.
/// Each selector must resolve to exactly one final blob; multiple selectors may
/// name the same blob, which is decoded and retained once per occurrence.
/// Nonempty, distinct selectors of at most 4096 bytes each are accepted, without
/// a chosen limit on their number or on retained unique digests.
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
        return Err(RdfDiagnostic::error(
            "rdf-ir-gts-import-internal",
            "the selected importer did not retain its blob collector",
        ));
    };
    let (blobs, refused, usage) = collector.finish()?;
    Ok(GtsImportWithBlobs {
        bundle,
        blobs,
        refused,
        usage,
    })
}

#[derive(Clone, Debug)]
struct BlobFrame {
    segment_index: u64,
    frame_index: u64,
    frame_id: Vec<u8>,
    range: ByteRange,
}

pub(crate) struct BlobCollector<'a> {
    export: Option<Vec<GtsBlobOmission>>,
    selectors: Vec<GtsBlobSelector<'a>>,
    selector_index: Vec<(u8, &'a str)>,
    limits: GtsBlobLimits,
    selected: OrderedTable<String, GtsImportedBlob>,
    refused: Vec<RefusedOccurrence>,
    /// Digests some occurrence in this container carried bytes for, whether or
    /// not they were selected. Distinguishes a payload we declined to retain
    /// from one the container never held.
    inlined: OrderedTable<String, ()>,
    /// Selected occurrences that named no payload and had none retained yet.
    ///
    /// Whether that is an external blob or an ordering problem cannot be decided
    /// mid-stream: the bytes may still arrive in a later frame. Resolved in
    /// [`Self::finish`], once the whole container has been read. A set, not a
    /// list: a repeated metadata-only row must not repeat its entry.
    unresolved: OrderedTable<String, ()>,
    /// The final name each digest is published under, for selection only.
    ///
    /// Selection reads the *final* metadata, so an occurrence's verdict is only
    /// provisional until the container ends. Written only for digests with a
    /// verified identity: an unproved claim renaming a real digest would let a
    /// garbage frame decide what a caller's selector resolves to.
    declared: OrderedTable<String, SelectionKey>,
    /// Last admitted full declaration per digest, with its physical source.
    ///
    /// This is what an absent-metadata occurrence inherits ACROSS segment
    /// boundaries, since the reader's own inheritance resets per segment. It
    /// feeds only what the caller gets back, so the metadata retention ceiling
    /// applies per entry; an over-budget declaration REMOVES the entry rather
    /// than leaving a superseded one to answer in its place. Selection never
    /// reads this map.
    last_declared: OrderedTable<String, (Value, GtsBlobMetadataSource)>,
    frame: Option<BlobFrame>,
    usage: GtsBlobUsage,
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
        return Err(RdfDiagnostic::error(
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
fn selection_key(meta: Option<&Value>) -> Result<SelectionKey, StorageError> {
    let Some(Value::Map(fields)) = meta else {
        return Ok(SelectionKey::default());
    };
    let rep = fields
        .iter()
        .find(|(key, _)| key.as_text() == Some("rep"))
        .and_then(|(_, value)| value.as_text())
        .filter(|text| text.len() <= MAX_SELECTOR_BYTES)
        .map(|text| Memory::new(&mut Resident).string(text))
        .transpose()?;
    Ok(SelectionKey { rep })
}

/// A refused occurrence and the name it was published under.
///
/// The projection travels with the occurrence because a refusal without a
/// computable identity has no entry in the by-digest record, and losing its name
/// would let a ceiling decide that it never matched anything.
struct RefusedOccurrence {
    blob: GtsRefusedBlob,
    key: SelectionKey,
    frame_index: u64,
    usage: GtsBlobUsage,
    available_decoded_bytes: usize,
    cause: Option<purrdf_gts::model::DiagnosticCause>,
    kind: purrdf_gts::reader::BlobRefusalKind,
    source: Option<GtsBlobMetadataSource>,
}

/// Borrow the selector's kind and exact spelling for ordered lookup.
fn selector_key(selector: GtsBlobSelector<'_>) -> (u8, &str) {
    match selector {
        GtsBlobSelector::Digest(text) => (0, text),
        GtsBlobSelector::Representation(text) => (1, text),
    }
}

#[derive(Default)]
struct SelectionOutcome {
    count: usize,
    first_refused: Option<usize>,
}

impl<'a> BlobCollector<'a> {
    fn selects(&self, digest: &str, key: Option<&SelectionKey>) -> bool {
        self.export.is_some()
            || self.selector_index.binary_search(&(0, digest)).is_ok()
            || key
                .and_then(|key| key.rep.as_deref())
                .is_some_and(|rep| self.selector_index.binary_search(&(1, rep)).is_ok())
    }

    fn selection_outcomes(&self) -> Result<Vec<SelectionOutcome>, StorageError> {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut outcomes = memory.collect(
            core::iter::repeat_with(SelectionOutcome::default).take(self.selector_index.len()),
        )?;
        let mut record = |digest: Option<&str>,
                          key: Option<&SelectionKey>,
                          refused: Option<usize>|
         -> Result<(), StorageError> {
            for candidate in [
                digest.map(|digest| (0, digest)),
                key.and_then(|key| key.rep.as_deref()).map(|rep| (1, rep)),
            ]
            .into_iter()
            .flatten()
            {
                if let Ok(index) = self.selector_index.binary_search(&candidate) {
                    let outcome = &mut outcomes[index];
                    outcome.count = outcome
                        .count
                        .checked_add(1)
                        .ok_or(StorageError::SizeOverflow)?;
                    outcome.first_refused = outcome.first_refused.or(refused);
                }
            }
            Ok(())
        };
        for (digest, _) in self.selected.iter() {
            record(Some(digest), self.declared.get(digest), None)?;
        }
        // Proved retained identities deduplicate. Unproved claims keep their
        // own occurrence's name and cannot redirect an authenticated digest.
        let mut proved_refusals = OrderedTable::default();
        for (index, refusal) in self.refused.iter().enumerate() {
            let proved = refusal
                .blob
                .digest
                .as_deref()
                .filter(|_| refusal.blob.digest_computed);
            if proved.is_some_and(|digest| self.selected.get_by(|key| digest.cmp(key)).is_some()) {
                continue;
            }
            if let Some(digest) = proved {
                if proved_refusals.get(&digest).is_some() {
                    continue;
                }
                proved_refusals.insert(digest, (), &mut memory)?;
            }
            let key = proved
                .and_then(|digest| self.declared.get_by(|key| digest.cmp(key)))
                .unwrap_or(&refusal.key);
            record(proved, Some(key), Some(index))?;
        }
        Ok(outcomes)
    }
    /// Resume the actual backing arrays before a later event grows them.
    fn storage_bytes(&self) -> Result<usize, StorageError> {
        let refused = core::alloc::Layout::array::<RefusedOccurrence>(self.refused.capacity())
            .map_err(|_| StorageError::SizeOverflow)?
            .size();
        [
            self.selected.storage_bytes()?,
            self.inlined.storage_bytes()?,
            self.unresolved.storage_bytes()?,
            self.declared.storage_bytes()?,
            self.last_declared.storage_bytes()?,
            refused,
        ]
        .into_iter()
        .try_fold(0_usize, |sum, bytes| {
            sum.checked_add(bytes).ok_or(StorageError::SizeOverflow)
        })
    }

    fn new(
        selectors: &[GtsBlobSelector<'a>],
        limits: GtsBlobLimits,
    ) -> Result<Self, RdfDiagnostic> {
        if selectors.iter().any(|selector| {
            let (GtsBlobSelector::Digest(value) | GtsBlobSelector::Representation(value)) =
                selector;
            value.is_empty() || value.len() > MAX_SELECTOR_BYTES
        }) {
            return Err(RdfDiagnostic::error(
                "rdf-ir-gts-blob-selection",
                "selectors must be distinct, nonempty, bounded exact identities",
            ));
        }
        // Sort only borrowed selector values, preserving the caller's original
        // order for final matching. Validation is O(n log n), not quadratic.
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut ordered = memory.collect(selectors.iter().copied().map(selector_key))?;
        ordered.sort_unstable();
        if ordered.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(RdfDiagnostic::error(
                "rdf-ir-gts-blob-selection",
                "selectors must be distinct, nonempty, bounded exact identities",
            ));
        }
        let selectors = memory.collect(selectors.iter().copied())?;
        Ok(Self {
            export: None,
            selectors,
            selector_index: ordered,
            limits,
            selected: OrderedTable::default(),
            refused: Vec::new(),
            inlined: OrderedTable::default(),
            unresolved: OrderedTable::default(),
            declared: OrderedTable::default(),
            last_declared: OrderedTable::default(),
            frame: None,
            usage: GtsBlobUsage::default(),
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
        let incoming = selection_key(metadata)?;
        let metadata = metadata.or_else(|| {
            self.selected
                .get_by(|key| digest.cmp(key))
                .and_then(|blob| blob.metadata.as_ref())
        });
        if self.selects(digest, self.declared.get_by(|key| digest.cmp(key)))
            || self.selects(digest, Some(&incoming))
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

    pub(crate) fn frame(&mut self, ctx: FrameContext<'_>) -> Result<(), RdfDiagnostic> {
        self.frame = Some(BlobFrame {
            segment_index: ctx.segment_index,
            frame_index: ctx.frame_index,
            frame_id: Memory::new(&mut Resident).collect(ctx.content_id.iter().copied())?,
            range: ctx.range,
        });
        Ok(())
    }

    pub(crate) fn payload(&mut self, payload: BlobPayload<'_>) -> Result<(), RdfDiagnostic> {
        let mut resident = Resident;
        let mut memory = Memory::resume(&mut resident, self.storage_bytes()?);
        if payload.payload_present {
            self.inlined
                .insert(memory.string(payload.digest)?, (), &mut memory)?;
        }
        if payload.metadata_declared {
            self.declare(
                payload.digest,
                payload.metadata,
                payload.segment_index,
                &mut memory,
            )?;
        }
        let previous = self.selected.get_by(|key| payload.digest.cmp(key));
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
            .then(|| self.last_declared.get_by(|key| payload.digest.cmp(key)))
            .flatten();
        let metadata = payload
            .metadata
            .or_else(|| inherited.map(|(value, _)| value))
            .or_else(|| previous.and_then(|blob| blob.metadata.as_ref()));
        let inherited_source = inherited.map(|(_, source)| source.try_copy()).transpose()?;
        if !self.selects(
            payload.digest,
            self.declared.get_by(|key| payload.digest.cmp(key)),
        ) {
            if let Some((_, old)) = self.selected.remove_entry_by(|key| payload.digest.cmp(key)) {
                // Release builds leave overflow checks off, so a future break of
                // the "total is the sum of retained lengths" invariant would wrap
                // to near usize::MAX and silently disable the total budget,
                // surfacing as spurious decode refusals far from the cause.
                self.usage
                    .replace_payload(old.bytes.len(), 0, self.selected.len())?;
            }
            return Ok(());
        }
        if payload.payload_present && payload.bytes.is_none() {
            return Err(RdfDiagnostic::error(
                "rdf-ir-gts-blob-payload",
                "selected blob payload field is not a byte string",
            ));
        }
        validate_metadata(metadata, payload.digest)?;
        if self.export.is_some()
            && payload.codecs.is_empty()
            && let Some(bytes) = payload.bytes
            && purrdf_gts::wire::digest_str(bytes) != payload.digest
        {
            return Err(RdfDiagnostic::error(
                "rdf-ir-gts-blob-digest",
                "inline blob bytes differ from their declared digest",
            ));
        }
        if let Err(failure) = check_metadata_bound(metadata, self.limits.max_metadata_bytes) {
            let error = failure.into_diagnostic()?;
            return self.omit_or_error(payload, GtsBlobOmissionKind::MetadataBudget, error);
        }
        let metadata = metadata.cloned();
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
                RdfDiagnostic::error(
                    "rdf-ir-gts-blob-provenance",
                    "selected blob has no matching source frame",
                )
            })?;
        let metadata_source = if payload.metadata_declared {
            Some(GtsBlobMetadataSource {
                segment_index: frame.segment_index,
                frame_index: frame.frame_index,
                frame_id: memory.collect(frame.frame_id.iter().copied())?,
                frame_range: frame.range.clone(),
                segment_head: Vec::new(),
            })
        } else {
            // Metadata inherited from an earlier declaration keeps THAT
            // declaration's source; a retained copy's source is the fallback.
            match inherited_source {
                Some(source) => Some(source),
                None => previous
                    .and_then(|blob| blob.metadata_source.as_ref())
                    .map(GtsBlobMetadataSource::try_copy)
                    .transpose()?,
            }
        };
        let Some(wire_bytes) = payload.bytes else {
            let Some(previous) = self.selected.get_mut_by(|key| payload.digest.cmp(key)) else {
                // No bytes retained for this digest yet. Whether that means the
                // blob is external, or merely that its payload has not been read
                // yet, is not decidable here — a later frame in this same
                // container may still carry it, and the authoritative importer
                // accepts exactly that file. Defer to finish(), which sees the
                // whole container.
                if self.export.is_some() {
                    self.record_export_omission(
                        payload,
                        GtsBlobOmissionKind::MissingInlinePayload,
                        None,
                    )?;
                }
                self.unresolved
                    .insert(memory.string(payload.digest)?, (), &mut memory)?;
                return Ok(());
            };
            previous.metadata = metadata;
            previous.metadata_source = metadata_source;
            return Ok(());
        };
        let old_len = previous.map_or(0, |blob| blob.bytes.len());
        let other_bytes = self
            .usage
            .retained_bytes()?
            .checked_sub(old_len)
            .ok_or(StorageError::SizeOverflow)?;
        let remaining = self
            .limits
            .max_total_decoded_bytes
            .checked_sub(other_bytes)
            .ok_or(StorageError::SizeOverflow)?;
        let refusal =
            |cause: Option<purrdf_gts::codec::CodecError>| -> Result<RdfDiagnostic, RdfDiagnostic> {
                let failure = GtsBlobRetentionRefusal {
                    digest: Some(Memory::new(&mut Resident).string(payload.digest)?),
                    digest_computed: payload.codecs.is_empty()
                        && purrdf_gts::wire::digest_str(wire_bytes) == payload.digest,
                    encoded_bytes: payload.encoded_len,
                    limits: self.limits,
                    available_decoded_bytes: remaining,
                    usage: self.usage,
                    segment_index: payload.segment_index,
                    frame_index: frame.frame_index,
                    cause: cause
                        .map(purrdf_gts::model::retain_native_cause)
                        .transpose()?
                        .map(purrdf_gts::model::DiagnosticCause::Native),
                };
                let message = Memory::new(&mut Resident).format(&failure)?;
                let code = if failure.cause.is_some() {
                    "rdf-ir-gts-blob-decode"
                } else {
                    "rdf-ir-gts-blob-limit"
                };
                let cause = purrdf_gts::model::retain_native_cause(failure)?;
                Ok(RdfDiagnostic::error(code, message)
                    .with_location(
                        crate::RdfLocation::logical("gts:blob")
                            .with_gts_segment(payload.segment_index)
                            .with_gts_frame(frame.frame_index),
                    )
                    .with_native_cause(cause, &mut Memory::new(&mut Resident))?)
            };
        if payload.encoded_len > self.limits.max_encoded_bytes {
            let error = refusal(None)?;
            return self.omit_or_error(payload, GtsBlobOmissionKind::ByteBudget, error);
        }
        let limit = self.limits.max_decoded_bytes.min(remaining);
        let bytes = match purrdf_gts::codec::decode_chain_bounded(payload.codecs, wire_bytes, limit)
        {
            Ok(bytes) => bytes,
            Err(error) => {
                let kind = if matches!(error, purrdf_gts::codec::CodecError::Limit { .. }) {
                    Some(GtsBlobOmissionKind::ByteBudget)
                } else if error.is_capability_unavailable() {
                    Some(GtsBlobOmissionKind::UnavailableCapability)
                } else {
                    None
                };
                let error = refusal(Some(error))?;
                return match kind {
                    Some(kind) => self.omit_or_error(payload, kind, error),
                    None => Err(error),
                };
            }
        };
        let digest = purrdf_gts::wire::digest_str(&bytes);
        if digest != payload.digest {
            return Err(RdfDiagnostic::error(
                "rdf-ir-gts-blob-digest",
                format!(
                    "selected blob digest {digest} differs from declared {}",
                    payload.digest
                ),
            ));
        }
        let next_len = bytes.len();
        self.selected.insert(
            memory.string(&digest)?,
            GtsImportedBlob {
                digest,
                metadata,
                metadata_source,
                bytes: bytes.into(),
                segment_index: payload.segment_index,
                frame_index: frame.frame_index,
                frame_id: memory.collect(frame.frame_id.iter().copied())?,
                frame_range: frame.range.clone(),
                segment_head: Vec::new(),
            },
            &mut memory,
        )?;
        self.usage
            .replace_payload(old_len, next_len, self.selected.len())?;
        Ok(())
    }

    fn declare(
        &mut self,
        digest: &str,
        metadata: Option<&Value>,
        segment: u64,
        memory: &mut Memory<'_, Resident>,
    ) -> Result<(), RdfDiagnostic> {
        self.declared
            .insert(memory.string(digest)?, selection_key(metadata)?, memory)?;
        // A superseded declaration must never answer for an inadmissible one.
        let source = self
            .frame
            .as_ref()
            .filter(|frame| frame.segment_index == segment)
            .map(|frame| -> Result<_, StorageError> {
                Ok(GtsBlobMetadataSource {
                    segment_index: frame.segment_index,
                    frame_index: frame.frame_index,
                    frame_id: memory.collect(frame.frame_id.iter().copied())?,
                    frame_range: frame.range.clone(),
                    segment_head: Vec::new(),
                })
            })
            .transpose()?;
        match (metadata, source) {
            (Some(meta), Some(source))
                if check_metadata_bound(Some(meta), self.limits.max_metadata_bytes).is_ok() =>
            {
                self.last_declared.insert(
                    memory.string(digest)?,
                    (meta.clone(), source),
                    memory,
                )?;
            }
            _ => {
                self.last_declared.remove_entry_by(|key| digest.cmp(key));
            }
        }
        Ok(())
    }

    /// Record a payload the reader's ceiling refused before decoding it.
    ///
    /// Kept as a selection candidate, not discarded. A budget bounds what this
    /// import *retains*; letting a refusal remove a candidate would let the
    /// budget decide which blob a selector resolves to, so a tight ceiling could
    /// silently turn an ambiguous match into a confident wrong answer.
    pub(crate) fn refused(&mut self, refusal: BlobRefusal<'_>) -> Result<(), RdfDiagnostic> {
        let mut resident = Resident;
        let mut memory = Memory::resume(&mut resident, self.storage_bytes()?);
        if self.export.is_some()
            && let Some(digest) = refusal.digest
        {
            self.inlined
                .insert(memory.string(digest)?, (), &mut memory)?;
        }
        if self.export.is_some() {
            validate_metadata(refusal.metadata, refusal.digest.unwrap_or(""))?;
        }
        let usage = self.usage;
        let previous_bytes = refusal
            .digest
            .filter(|_| refusal.digest_computed)
            .and_then(|digest| self.selected.get_by(|key| digest.cmp(key)))
            .map_or(0, |blob| blob.bytes.len());
        let available_decoded_bytes = self
            .limits
            .max_total_decoded_bytes
            .checked_sub(
                usage
                    .retained_bytes()?
                    .checked_sub(previous_bytes)
                    .ok_or(StorageError::SizeOverflow)?,
            )
            .ok_or(StorageError::SizeOverflow)?;
        let frame_index = self
            .frame
            .as_ref()
            .filter(|frame| frame.segment_index == refusal.segment_index)
            .ok_or_else(|| {
                RdfDiagnostic::error(
                    "rdf-ir-gts-blob-provenance",
                    "refused blob has no matching source frame",
                )
            })?
            .frame_index;
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
            self.declare(digest, Some(meta), refusal.segment_index, &mut memory)?;
            if !self.selects(digest, self.declared.get_by(|key| digest.cmp(key))) {
                if let Some((_, old)) = self.selected.remove_entry_by(|key| digest.cmp(key)) {
                    self.usage
                        .replace_payload(old.bytes.len(), 0, self.selected.len())?;
                }
            } else if self.selected.get_by(|key| digest.cmp(key)).is_some()
                && (self.export.is_none()
                    || check_metadata_bound(Some(meta), self.limits.max_metadata_bytes).is_ok())
            {
                let metadata = bounded_metadata(Some(meta), self.limits.max_metadata_bytes)?;
                validate_metadata(metadata.as_ref(), digest)?;
                let source = self
                    .last_declared
                    .get_by(|key| digest.cmp(key))
                    .map(|(_, source)| source.try_copy())
                    .transpose()?;
                if let Some(blob) = self.selected.get_mut_by(|key| digest.cmp(key)) {
                    blob.metadata = metadata;
                    blob.metadata_source = source;
                }
            }
        }
        // A budget bounds what this import retains, and a refused declaration is
        // retained like any other: it obeys the per-blob metadata ceiling AND
        // counts against the retained total, or a stream of refusals would
        // accumulate container-chosen bytes no ceiling ever saw. Selection is
        // untouched — the projection above and the key below were taken first.
        let admitted_metadata = refusal
            .metadata
            .and_then(|meta| check_metadata_bound(Some(meta), self.limits.max_metadata_bytes).ok());
        let per_blob_ok = admitted_metadata.is_some();
        let retained_len = admitted_metadata.unwrap_or(0);
        let within_total = self
            .usage
            .retained_bytes()?
            .checked_add(retained_len)
            .ok_or(StorageError::SizeOverflow)?
            <= self.limits.max_total_decoded_bytes;
        let metadata = refusal.metadata.filter(|_| per_blob_ok && within_total);
        let detail = if metadata.is_none() && refusal.metadata.is_some() {
            memory.format(&format_args!(
                "{}; its public metadata exceeded the retention budgets and was not retained",
                refusal.detail
            ))?
        } else {
            self.usage
                .retain_refusal_metadata(retained_len, self.selected.len())?;
            memory.string(refusal.detail)?
        };
        let key = selection_key(refusal.metadata)?;
        let source = self
            .export
            .as_ref()
            .map(|_| self.export_source())
            .transpose()?;
        let occurrence = RefusedOccurrence {
            kind: refusal.kind,
            source,
            key,
            frame_index,
            usage,
            available_decoded_bytes,
            cause: None,
            blob: GtsRefusedBlob {
                digest: refusal
                    .digest
                    .map(|digest| memory.string(digest))
                    .transpose()?,
                digest_computed: refusal.digest_computed,
                metadata: metadata.cloned(),
                segment_index: refusal.segment_index,
                encoded_len: refusal.encoded_len,
                detail,
            },
        };
        memory.push(&mut self.refused, occurrence)?;
        Ok(())
    }

    pub(crate) fn segment_head(
        &mut self,
        segment_index: u64,
        head: &[u8],
    ) -> Result<(), RdfDiagnostic> {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        for blob in self.selected.values_mut() {
            if blob.segment_index == segment_index {
                blob.segment_head = memory.collect(head.iter().copied())?;
            }
            if let Some(source) = &mut blob.metadata_source
                && source.segment_index == segment_index
            {
                source.segment_head = memory.collect(head.iter().copied())?;
            }
        }
        // A declaration held for a later segment to inherit needs its head too:
        // it is recorded while its own segment is still open, and an occurrence
        // that inherits it hands the caller that source verbatim.
        for (_, source) in self.last_declared.values_mut() {
            if source.segment_index == segment_index {
                source.segment_head = memory.collect(head.iter().copied())?;
            }
        }
        if let Some(omissions) = &mut self.export {
            for omission in omissions {
                if omission.source.segment_index == segment_index {
                    omission.source.segment_head = memory.collect(head.iter().copied())?;
                }
            }
        }
        for refusal in &mut self.refused {
            if let Some(source) = &mut refusal.source
                && source.segment_index == segment_index
            {
                source.segment_head = memory.collect(head.iter().copied())?;
            }
        }
        Ok(())
    }

    pub(crate) fn refusal_diagnostic(
        &mut self,
        diagnostic: &purrdf_gts::model::Diagnostic,
    ) -> Result<(), RdfDiagnostic> {
        // The reader emits the diagnostic immediately after its refusal event.
        // Diagnostic indices include CBOR header items; FrameContext indices
        // identify frames within a segment, so those indices are not comparable.
        let refusal = self
            .refused
            .last_mut()
            .filter(|refusal| {
                self.frame.as_ref().is_some_and(|frame| {
                    frame.frame_index == refusal.frame_index
                        && frame.segment_index == refusal.blob.segment_index
                })
            })
            .ok_or_else(|| {
                RdfDiagnostic::error(
                    "rdf-ir-gts-import-internal",
                    "blob budget diagnostic has no matching recorded refusal",
                )
            })?;
        refusal.cause.clone_from(&diagnostic.cause);
        Ok(())
    }

    fn finish(
        mut self,
    ) -> Result<(Vec<GtsImportedBlob>, Vec<GtsRefusedBlob>, GtsBlobUsage), RdfDiagnostic> {
        // A selected occurrence that named no payload is only a problem if the
        // container never produced one. Deciding this here, rather than at the
        // occurrence, is what lets a metadata-only frame precede its own inline
        // payload — a file the authoritative importer accepts.
        for (digest, ()) in self.unresolved.iter() {
            if self.selected.get(digest).is_some() {
                continue;
            }
            // The verdict at the occurrence was provisional: a selector matches
            // the FINAL metadata for a digest, and a later frame may have
            // retagged this one so that nothing names it any more. Refusing over
            // a blob the selection does not finally reach would reject a file
            // the authoritative importer accepts.
            let key = self.declared.get(digest);
            if !self.selects(digest, key) {
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
                return Err(self.with_refusal_cause(RdfDiagnostic::error(
                    "rdf-ir-gts-blob-limit",
                    format!(
                        "blob {digest} {identity} and its {} encoded bytes exceeded this import's budget: {}",
                        refusal.blob.encoded_len, refusal.blob.detail
                    ),
                ), refusal)?);
            }
            if self.inlined.get(digest).is_some() {
                return Err(RdfDiagnostic::error(
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
                return Err(RdfDiagnostic::error(
                    "rdf-ir-gts-blob-limit",
                    format!(
                        "blob {digest} was selected but never retained, and {unidentified} payload(s) were refused before identification, so whether it is inline cannot be determined; raise the budgets or supply the payload"
                    ),
                ));
            }
            return Err(RdfDiagnostic::error(
                "rdf-ir-gts-blob-external",
                format!(
                    "blob {digest} is external to this container; its bytes are held elsewhere and this importer returns only inline payloads"
                ),
            ));
        }

        let outcomes = self.selection_outcomes()?;
        for selector in &self.selectors {
            let index = self
                .selector_index
                .binary_search(&selector_key(*selector))
                .map_err(|_| {
                    RdfDiagnostic::error(
                        "rdf-ir-gts-import-internal",
                        "validated selector is absent from its index",
                    )
                })?;
            let outcome = &outcomes[index];
            let count = outcome.count;
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
                return Err(RdfDiagnostic::error(
                    "rdf-ir-gts-blob-selection",
                    format!(
                        "required selector {selector:?} resolves to {count} blobs; expected exactly one{note}"
                    ),
                ));
            }
            // Exactly one match, and it is the one we declined to decode. Say so
            // precisely: reporting "resolves to 0 blobs" would be a false claim
            // about an archive that does contain the payload.
            if let Some(index) = outcome.first_refused {
                let blob = &self.refused[index];
                return Err(self.with_refusal_cause(RdfDiagnostic::error(
                    "rdf-ir-gts-blob-limit",
                    format!(
                        "required selector {selector:?} matches a payload of {} encoded bytes that this import's budget refused: {}",
                        blob.blob.encoded_len, blob.blob.detail
                    ),
                ), blob)?);
            }
        }
        // The type permits an empty head or frame id and the doc calls both
        // "verified". The reader publishes a segment head at every segment close
        // and fires its frame event before any of that frame's rows, so neither
        // can be empty here; this is an invariant, not a refusal, and is checked
        // as one rather than shipped as a branch no input can reach.
        debug_assert!(
            self.selected
                .iter()
                .map(|(_, blob)| blob)
                .all(|blob| !blob.segment_head.is_empty() && !blob.frame_id.is_empty()),
            "selected blob provenance must be populated before it is returned"
        );
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let blobs = self.take_export_blobs()?;
        let refused = memory.collect(self.refused.into_iter().map(|refused| refused.blob))?;
        Ok((blobs, refused, self.usage))
    }

    fn with_refusal_cause(
        &self,
        diagnostic: RdfDiagnostic,
        refusal: &RefusedOccurrence,
    ) -> Result<RdfDiagnostic, RdfDiagnostic> {
        let failure = GtsBlobRetentionRefusal {
            digest: refusal
                .blob
                .digest
                .as_deref()
                .map(|digest| Memory::new(&mut Resident).string(digest))
                .transpose()?,
            digest_computed: refusal.blob.digest_computed,
            encoded_bytes: refusal.blob.encoded_len,
            limits: self.limits,
            available_decoded_bytes: refusal.available_decoded_bytes,
            usage: refusal.usage,
            segment_index: refusal.blob.segment_index,
            frame_index: refusal.frame_index,
            cause: refusal.cause.clone(),
        };
        let cause = purrdf_gts::model::retain_native_cause(failure)?;
        Ok(diagnostic
            .with_location(
                crate::RdfLocation::logical("gts:blob")
                    .with_gts_segment(refusal.blob.segment_index)
                    .with_gts_frame(refusal.frame_index),
            )
            .with_native_cause(cause, &mut Memory::new(&mut Resident))?)
    }
}

fn validate_metadata(meta: Option<&Value>, digest: &str) -> Result<(), RdfDiagnostic> {
    if let Some(meta) = meta
        && metadata_field(Some(meta), "digest")?.is_some()
        && purrdf_gts::reader::public_blob_digest(meta).as_deref() != Some(digest)
    {
        return Err(RdfDiagnostic::error(
            "rdf-ir-gts-blob-digest",
            "selected blob metadata digest differs from its identity",
        ));
    }
    for key in ["rep", "mt"] {
        if let Some(value) = metadata_field(meta, key)? {
            value.as_text().ok_or_else(|| {
                RdfDiagnostic::error(
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

fn check_metadata_bound(
    meta: Option<&Value>,
    limit: usize,
) -> Result<usize, error::MetadataBudgetRefusal> {
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
    let mut counter = Counter { remaining: limit };
    if let Some(meta) = meta {
        purrdf_lex::cbor::write(meta, &mut counter).map_err(|cause| {
            error::MetadataBudgetRefusal {
                limit,
                written: limit - counter.remaining,
                cause: cause.into(),
            }
        })?;
    }
    // Every successful write subtracts from this initial allowance.
    Ok(limit - counter.remaining)
}
