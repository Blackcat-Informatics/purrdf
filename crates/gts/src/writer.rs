// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A GTS writer: build frames, maintain the id/prev chain, emit a CBOR
//! Sequence.
//!
//! This is the encoder counterpart to [`crate::reader`]. It supports the core
//! graph/file frame families plus transformed, encrypted, and signed payloads.
//! Deterministic CBOR and BLAKE3 self-hashes are handled by [`crate::wire`].

mod error;
pub use error::FrameError;

use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::mem::{size_of, size_of_val};

use purrdf_lex::cbor::Value;

use crate::FastMap;
use crate::codec::{Codec, CodecError, EncodeOptions, encode_chain_with_options};
use crate::model::{
    AnnotationRow, Graph, Quad, ReifierRow, Suppression, Term, TermKind, is_literal_direction,
};
use crate::wire::{
    SELF_DESCRIBE_TAG, append_canonical, canonical, content_id, digest_str, header_id,
};

/// Payloads larger than this select `zstd-rsyncable` over `zstd` in snapshot helpers.
pub const DEFAULT_RSYNCABLE_THRESHOLD: usize = 65_536;

/// Serialise a [`Term`] to its wire map (dropping absent fields).
pub fn term_to_wire(t: &Term) -> Value {
    let mut entries: Vec<(Value, Value)> = Vec::with_capacity(6);
    entries.push(("k".into(), Value::from(t.kind as i64)));
    if let Some(v) = &t.value {
        entries.push(("v".into(), v.clone().into()));
    }
    if let Some(dt) = t.datatype {
        entries.push(("dt".into(), Value::from(dt)));
    }
    if let Some(l) = &t.lang {
        entries.push(("l".into(), l.clone().into()));
    }
    if let Some(direction) = t.direction.as_deref().filter(|d| is_literal_direction(d)) {
        entries.push(("dir".into(), direction.to_string().into()));
    }
    if let Some(rf) = t.reifier {
        entries.push(("rf".into(), Value::from(rf)));
    }
    // A self-describing quoted triple carries its own components, so the wire
    // never has to route a triple TERM's identity through a reifier id (which
    // RDF 1.2 lets bind several different triples).
    if let Some((s, p, o)) = t.triple {
        entries.push((
            "tt".into(),
            Value::Array(vec![Value::from(s), Value::from(p), Value::from(o)]),
        ));
    }
    Value::Map(entries)
}

/// Writer construction options for header-level parity with the Python writer.
///
/// These values affect the header, so they are part of the segment genesis
/// hash. Changing them after bytes are emitted would change every downstream
/// `prev` link; construct a new writer instead.
#[derive(Clone, Debug)]
pub struct WriterOptions {
    /// Optional transform catalog. When omitted, the default GTS catalog is used.
    pub catalog: Option<Vec<(i64, Codec)>>,
    /// Optional header metadata carried in the header `"meta"` key.
    pub meta: Option<Value>,
    /// Prefix the header with CBOR self-describe tag 55799.
    pub magic_tag: bool,
    /// Optional layout-state claim. Only `"streamable"` is defined in this revision.
    pub layout: Option<String>,
    /// In-band pack dictionaries for the zstd-family `dct` codecs, as
    /// `(dict_name, dict_bytes)` rows (§5 header `"dct"`, §8.5 `zstd` /
    /// `zstd-rsyncable` `dct` parameter).
    ///
    /// §5 has always allowed MANY named dictionaries (`"dct": { * tstr =>
    /// bstr }`) and the reader has always resolved them; this is the writer
    /// catching up. Each dictionary's bytes are stored uncompressed and in-band
    /// under its name, and the catalog gains one entry per
    /// `(zstd-family codec, dictionary)` pair so a frame can name exactly the
    /// dictionary it was primed with. A frame selects one by name through
    /// [`FrameOptions::dict`]; names must be unique and non-empty.
    pub dicts: Vec<(String, Vec<u8>)>,
    /// The zstd compression level this pack declares for its zstd-family
    /// catalog entries (§8.5 `level?`).
    ///
    /// Declaring the level makes it a WIRE FACT: a reader recovers it from the
    /// catalog ([`crate::codec::Codec::level`]) and a deployment profile can
    /// require, say, level 12 without trusting the producer's word. Because the
    /// catalog would otherwise be able to lie, a frame that requests a
    /// different [`FrameOptions::zstd_level`] than the declared one is a hard
    /// error rather than a silent divergence.
    pub zstd_level: Option<i32>,
}

impl Default for WriterOptions {
    fn default() -> Self {
        Self {
            catalog: None,
            meta: None,
            magic_tag: true,
            layout: None,
            dicts: Vec::new(),
            zstd_level: None,
        }
    }
}

/// COSE_Encrypt0 frame-authorship options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Encrypt0Options {
    /// Recipient key id.
    pub kid: String,
    /// 32-byte AES-256-GCM content key.
    pub key: [u8; 32],
    /// 12-byte AES-GCM IV/nonce. Callers that need randomized encryption generate
    /// this outside the core crate so the writer remains wasm-portable.
    pub iv: [u8; 12],
}

/// Advanced frame-authorship options matching Python `Writer.add_frame`.
///
/// `payload` values are canonical-CBOR encoded before transforms; `raw` values
/// are used as provided. The final encrypted/transformed bytes are carried in
/// the frame `"d"` field and authenticated by the frame content id.
#[derive(Clone, Debug, Default)]
pub struct FrameOptions {
    /// Structured CBOR payload. Mutually exclusive with [`FrameOptions::raw`].
    pub payload: Option<Value>,
    /// Raw byte payload. Mutually exclusive with [`FrameOptions::payload`].
    pub raw: Option<Vec<u8>>,
    /// Codec names applied in array order before optional encryption.
    pub transform: Vec<String>,
    /// Optional per-frame zstd level for `zstd` and `zstd-rsyncable` transforms.
    ///
    /// When the writer declares [`WriterOptions::zstd_level`], this must either
    /// be `None` (inherit the declared level) or equal it — a frame encoded at
    /// a level the catalog does not declare would make the catalog a lie.
    pub zstd_level: Option<i32>,
    /// Name of the in-band pack dictionary priming this frame's zstd-family
    /// transform (§5 header `"dct"`), selected from [`WriterOptions::dicts`].
    ///
    /// A name absent from the writer's dictionary table is a HARD ERROR, never
    /// a silent no-dictionary encode: the frame's `"x"` chain would then point
    /// at a plain codec entry and the pack would quietly lose the density the
    /// caller asked for. `None` means "no dictionary" — an explicit choice.
    pub dict: Option<String>,
    /// Public frame metadata (`"pub"`).
    pub pub_meta: Option<Value>,
    /// Recipient metadata rows (`"to"`).
    pub recipients: Vec<Value>,
    /// Explicit COSE_Sign1 bytes. When omitted, the writer's configured signer is used.
    pub signature: Option<Vec<u8>>,
    /// Encrypt the transformed payload as COSE_Encrypt0 and append `cose-encrypt0` to `"x"`.
    pub encrypt: Option<Encrypt0Options>,
}

/// A content-addressed blob row emitted before a snapshot frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlobRow {
    /// Decoded blob bytes.
    pub data: Vec<u8>,
    /// Declared media type (`pub.mt`).
    pub media_type: String,
    /// Content representation tag (`pub.rep`).
    pub rep: String,
}

/// Signing inputs for snapshot bundle authorship.
#[derive(PartialEq, Eq)]
pub struct SnapshotSigner {
    /// 32-byte Ed25519 secret seed.
    pub secret: [u8; 32],
    /// COSE key id used in frame signatures and `gts:transportKey`.
    pub kid: String,
    /// ASCII-armored OpenPGP Ed25519 public-key certificate embedded as transport metadata.
    pub public_key_armor: String,
}

impl Clone for SnapshotSigner {
    fn clone(&self) -> Self {
        // Finish potentially allocating public clones before copying the seed
        // into the new Drop-protected owner.
        let kid = self.kid.clone();
        let public_key_armor = self.public_key_armor.clone();
        Self {
            secret: self.secret,
            kid,
            public_key_armor,
        }
    }
}

purrdf_hash::debug_non_exhaustive!(SnapshotSigner {
    kid,
    public_key_armor
});

impl Drop for SnapshotSigner {
    fn drop(&mut self) {
        purrdf_ed25519::wipe_secret(&mut self.secret);
    }
}

/// Options for [`snapshot_from_graph`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotOptions {
    /// Base transform chain for blob and snapshot payloads.
    pub transform: Vec<String>,
    /// Payloads above this size switch from `zstd` to `zstd-rsyncable`.
    pub rsyncable_threshold: usize,
    /// Optional zstd level used for documentation/report blob frames.
    pub blob_zstd_level: Option<i32>,
    /// Optional zstd level used for the snapshot frame.
    pub snapshot_zstd_level: Option<i32>,
    /// Documentation/content blobs emitted ahead of the snapshot frame.
    pub doc_blobs: Vec<BlobRow>,
    /// Report/evidence blobs emitted ahead of the snapshot frame.
    pub report_blobs: Vec<BlobRow>,
    /// Optional signing identity. When present, a signed `gts:transportKey` meta frame is emitted.
    pub signer: Option<SnapshotSigner>,
}

impl Default for SnapshotOptions {
    fn default() -> Self {
        Self {
            transform: vec!["zstd".to_string()],
            rsyncable_threshold: DEFAULT_RSYNCABLE_THRESHOLD,
            blob_zstd_level: None,
            snapshot_zstd_level: None,
            doc_blobs: Vec::new(),
            report_blobs: Vec::new(),
            signer: None,
        }
    }
}

/// Errors raised by advanced writer construction.
#[derive(Debug)]
#[non_exhaustive]
pub enum WriterError {
    /// Invalid caller options.
    InvalidFrame(FrameError),
    /// Recovery of the existing append chain failed.
    Append(crate::reader::AppendError),
    /// The writer catalog cannot satisfy a requested codec.
    MissingCatalogEntry(String),
    /// Codec encode failure.
    Codec(CodecError),
    /// A composite signer has no installed randomness provider.
    MissingRandomnessProvider,
    /// The caller's cryptographic randomness provider failed.
    Randomness(RandomnessError),
    /// The native signing operation refused to produce a signature.
    Signing(crate::cose::Sign1Error),
}

impl fmt::Display for WriterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFrame(detail) => detail.fmt(f),
            Self::Append(cause) => cause.fmt(f),
            Self::MissingCatalogEntry(name) => {
                write!(f, "writer catalog has no entry for codec '{name}'")
            }
            Self::Codec(err) => write!(f, "{err}"),
            Self::MissingRandomnessProvider => {
                f.write_str("composite signing requires a fresh cryptographic randomness provider")
            }
            Self::Randomness(err) => write!(f, "composite signing randomness: {err}"),
            Self::Signing(err) => write!(f, "frame signing: {err}"),
        }
    }
}

purrdf_lex::error_source!(WriterError {
    InvalidFrame,
    Append,
    Codec,
    Randomness,
    Signing
});

impl purrdf_hash::error::StructuredError for WriterError {
    purrdf_hash::error_visit! {
        InvalidFrame(cause) => "gts-writer-invalid-frame" { "cause" => Cause(cause) },
        Append(cause) => "gts-writer-append" { "cause" => Cause(cause) },
        MissingCatalogEntry(name) => "gts-writer-missing-codec" { "name" => Text(name) },
        Codec(cause) => "gts-writer-codec" { "cause" => Cause(cause) },
        MissingRandomnessProvider => "gts-writer-missing-randomness" {},
        Randomness(cause) => "gts-writer-randomness" { "cause" => Cause(cause) },
        Signing(cause) => "gts-writer-signing" { "cause" => Cause(cause) },
    }
}

impl From<CodecError> for WriterError {
    fn from(value: CodecError) -> Self {
        Self::Codec(value)
    }
}

/// Choose `zstd-rsyncable` for large payloads when the base chain is exactly `["zstd"]`.
pub fn choose_snapshot_transform(
    base_chain: &[String],
    payload_len: usize,
    threshold: usize,
) -> Vec<String> {
    if base_chain.len() == 1 && base_chain[0] == "zstd" && payload_len > threshold {
        vec!["zstd-rsyncable".to_string()]
    } else {
        base_chain.to_vec()
    }
}

/// Serialize a folded [`Graph`] as a single-frame snapshot bundle.
///
/// The writer emits, in order: optional signed transport-key metadata, sorted
/// content-addressed blob frames, then the canonical single `snapshot` frame.
pub fn snapshot_from_graph(
    graph: &Graph,
    profile: &str,
    options: SnapshotOptions,
) -> Result<Vec<u8>, WriterError> {
    let mut writer = Writer::new(profile);
    let SnapshotOptions {
        transform,
        rsyncable_threshold,
        blob_zstd_level,
        snapshot_zstd_level,
        doc_blobs,
        report_blobs,
        signer,
    } = options;

    if let Some(mut signer) = signer {
        writer.sign_with(
            purrdf_ed25519::SigningKey::from_bytes(&signer.secret),
            &signer.kid,
        );
        writer.add_meta(Value::Map(vec![(
            "gts:transportKey".into(),
            Value::Map(vec![
                ("kid".into(), Value::Text(std::mem::take(&mut signer.kid))),
                (
                    "gpg".into(),
                    Value::Text(std::mem::take(&mut signer.public_key_armor)),
                ),
            ]),
        )]));
    }

    let mut blobs = doc_blobs;
    blobs.extend(report_blobs);
    blobs.sort_by(|a, b| {
        a.rep
            .cmp(&b.rep)
            .then_with(|| a.data.cmp(&b.data))
            .then_with(|| a.media_type.cmp(&b.media_type))
    });
    for blob in blobs {
        let chain = choose_snapshot_transform(&transform, blob.data.len(), rsyncable_threshold);
        let pub_meta = Value::Map(vec![
            ("digest".into(), Value::Text(digest_str(&blob.data))),
            ("mt".into(), Value::Text(blob.media_type)),
            ("rep".into(), Value::Text(blob.rep)),
        ]);
        writer.add_frame_with_options(
            "blob",
            FrameOptions {
                raw: Some(blob.data),
                transform: chain,
                zstd_level: blob_zstd_level,
                pub_meta: Some(pub_meta),
                ..FrameOptions::default()
            },
        )?;
    }

    let payload = graph.snapshot_payload();
    let (payload, raw, chain) = if transform.len() == 1 && transform[0] == "zstd" {
        let bytes = canonical(&payload);
        let chain = choose_snapshot_transform(&transform, bytes.len(), rsyncable_threshold);
        (None, Some(bytes), chain)
    } else {
        (Some(payload), None, transform)
    };
    writer.add_frame_with_options(
        "snapshot",
        FrameOptions {
            payload,
            raw,
            transform: chain,
            zstd_level: snapshot_zstd_level,
            ..FrameOptions::default()
        },
    )?;
    Ok(writer.into_bytes())
}

/// The canonical `terms`-frame payload.
pub(crate) fn terms_payload(terms: &[Term]) -> Value {
    Value::Array(terms.iter().map(term_to_wire).collect())
}

/// The canonical `quads`-frame payload (graph slot dropped when `None`), and the
/// `annot`-frame one ([`annot_payload`]).
pub(crate) fn quads_payload(quads: &[Quad]) -> Value {
    Value::Array(
        quads
            .iter()
            .map(|&(s, p, o, g)| {
                let mut row = Vec::with_capacity(3 + usize::from(g.is_some()));
                row.push(Value::from(s));
                row.push(Value::from(p));
                row.push(Value::from(o));
                if let Some(gv) = g {
                    row.push(Value::from(gv));
                }
                Value::Array(row)
            })
            .collect(),
    )
}

/// The canonical `reifies`-frame payload.
pub(crate) fn reifies_payload(bindings: &[ReifierRow]) -> Value {
    Value::Array(
        bindings
            .iter()
            .map(|&(rid, (s, p, o), g)| {
                let mut row = Vec::with_capacity(4 + usize::from(g.is_some()));
                row.push(Value::from(rid));
                row.push(Value::from(s));
                row.push(Value::from(p));
                row.push(Value::from(o));
                if let Some(gv) = g {
                    row.push(Value::from(gv));
                }
                Value::Array(row)
            })
            .collect(),
    )
}

/// The canonical `annot`-frame payload. An annotation row
/// `(reifier, predicate, value, graph?)` has a quad's four columns and the same
/// optional graph slot, so it is written by [`quads_payload`].
pub(crate) use self::quads_payload as annot_payload;

/// The canonical `suppress`-frame payload.
pub(crate) fn suppress_payload(
    targets: Vec<Value>,
    reason: Option<&str>,
    by: Option<usize>,
) -> Value {
    let mut payload: Vec<(Value, Value)> = vec![("targets".into(), Value::Array(targets))];
    if let Some(r) = reason {
        payload.push(("reason".into(), r.into()));
    }
    if let Some(b) = by {
        payload.push(("by".into(), Value::from(b as u64)));
    }
    Value::Map(payload)
}

fn default_catalog() -> Vec<(i64, Codec)> {
    vec![
        (0, Codec::new("identity", "encode")),
        (1, Codec::new("gzip", "compress")),
        (2, Codec::new("zstd", "compress")),
        (3, Codec::new("zstd-rsyncable", "compress")),
        (7, Codec::new("cose-encrypt0", "encrypt")),
    ]
}

purrdf_lex::message_error! {
    /// Failure to supply a fresh, complete cryptographic randomizer.
    /// The caller's explanation is retained in [`WriterError::Randomness`].
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct RandomnessError, detail;
}

/// Portable caller-owned cryptographic randomness source. Each successful call
/// must replace all 32 bytes with fresh cryptographic randomness. The writer
/// supplies no system RNG, clock, deterministic fallback or quality policy.
pub trait RandomnessProvider {
    /// Fill one randomizer completely, or return an actionable failure. Partial
    /// writes on failure are allowed: the writer clears its owned storage.
    fn fill_randomizer(&mut self, randomizer: &mut [u8; 32]) -> Result<(), RandomnessError>;
}

impl<F> RandomnessProvider for F
where
    F: FnMut(&mut [u8; 32]) -> Result<(), RandomnessError>,
{
    fn fill_randomizer(&mut self, randomizer: &mut [u8; 32]) -> Result<(), RandomnessError> {
        self(randomizer)
    }
}

/// Default signing mode: unsigned frames and deterministic Ed25519 authorship.
/// Composite keys can only be installed by consuming the writer into [`Hedged`].
#[derive(Debug)]
pub struct Infallible;

/// Signing mode with a required caller-owned cryptographic randomness source.
/// All convenience append methods return `Result` in this mode, including after
/// switching back to an Ed25519 signing key.
pub struct Hedged<P> {
    provider: P,
}

impl<P> fmt::Debug for Hedged<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Hedged").finish_non_exhaustive()
    }
}

mod mode {
    use super::{Hedged, Infallible, RandomnessProvider, WriterError};

    pub trait Sealed {
        fn fill(&mut self, output: &mut [u8; 32]) -> Result<(), WriterError>;
    }

    impl Sealed for Infallible {
        fn fill(&mut self, _: &mut [u8; 32]) -> Result<(), WriterError> {
            Err(WriterError::MissingRandomnessProvider)
        }
    }

    impl<P: RandomnessProvider> Sealed for Hedged<P> {
        fn fill(&mut self, output: &mut [u8; 32]) -> Result<(), WriterError> {
            self.provider
                .fill_randomizer(output)
                .map_err(WriterError::Randomness)
        }
    }
}

/// Sealed append-result contract. The default mode retains the existing
/// convenience API; a hedged mode exposes every append error as `Result`.
pub trait FrameMode: mode::Sealed {
    /// Result returned by simple append conveniences in this signing mode.
    type FrameResult;
    #[doc(hidden)]
    fn finish(result: Result<Vec<u8>, WriterError>) -> Self::FrameResult;
}

impl FrameMode for Infallible {
    type FrameResult = Vec<u8>;
    fn finish(result: Result<Vec<u8>, WriterError>) -> Self::FrameResult {
        result.expect("invalid frame options")
    }
}

impl<P: RandomnessProvider> FrameMode for Hedged<P> {
    type FrameResult = Result<Vec<u8>, WriterError>;
    fn finish(result: Result<Vec<u8>, WriterError>) -> Self::FrameResult {
        result
    }
}

#[derive(Debug)]
enum FrameSigner {
    Ed25519(Box<purrdf_ed25519::SigningKey>, Vec<u8>),
    Composite(Box<crate::cose::composite::SigningKey>, Vec<u8>),
}

struct Randomizer([u8; 32]);
impl Drop for Randomizer {
    fn drop(&mut self) {
        purrdf_ed25519::wipe_secret(&mut self.0);
    }
}

/// Accumulate a GTS log as a CBOR Sequence.
///
/// # Examples
///
/// Each `add_*` call appends one frame and returns its BLAKE3 content id;
/// [`Writer::head`] tracks the id of the last appended frame:
///
/// ```
/// use purrdf_gts::writer::Writer;
///
/// let mut writer = Writer::new("purrdf.gts");
/// let frame_id = writer.add_blob(b"nine lives", Some("text/plain"), None);
/// assert_eq!(writer.head(), frame_id.as_slice());
///
/// let bytes = writer.into_bytes();
/// assert!(!bytes.is_empty());
/// ```
// `SigningKey`'s `Debug` impl redacts the secret scalar, so deriving is safe here.
#[derive(Debug)]
pub struct Writer<M: FrameMode = Infallible> {
    // Catalog lookup keyed by `(codec_name, dictionary_name)` — see
    // `assign_catalog_ids`. A plain `HashMap<String, i64>` cannot express a
    // catalog carrying N `zstd-rsyncable` entries (one per dictionary): the
    // same-named entries collapse and the surviving id becomes whichever one
    // HashMap iteration happened to visit last, which would make the emitted
    // bundle bytes NONDETERMINISTIC.
    catalog_ids: BTreeMap<CatalogKey, i64>,
    prev: Vec<u8>,
    buf: Vec<u8>,
    // Per-frame byte offsets and types, in append order — the raw material
    // of an `index` footer (§6.2): offsets enable random access/parallel
    // verify, types the "ti" locator map.
    offsets: Vec<usize>,
    types: Vec<String>,
    frame_ids: Vec<Vec<u8>>,
    // When set, every appended frame is COSE_Sign1-signed over its id (§9.2).
    signer: Option<FrameSigner>,
    mode: M,
    // The pinned in-band pack dictionaries by name (§5 header `"dct"`), which a
    // frame selects through `FrameOptions::dict`.
    dicts: BTreeMap<String, Vec<u8>>,
    // The level declared in the zstd-family catalog entries (§8.5 `level?`).
    declared_zstd_level: Option<i32>,
}

/// A catalog lookup key: the canonical codec name plus the in-band dictionary
/// (if any) that entry is bound to (§5, §8.5).
type CatalogKey = (String, Option<String>);

/// Assign a catalog id to every `(codec_name, dictionary_name)` pair, as a PURE
/// FUNCTION of the base catalog and the sorted dictionary-name set.
///
/// Base (dictionary-free) entries keep the ids the caller supplied. Each
/// zstd-family codec then gains one entry per dictionary; those synthetic ids
/// are handed out in ascending `(codec_name, dict_name)` order starting above
/// every id already in use. Nothing here depends on hash iteration order, so the
/// same `(catalog, dicts)` inputs always yield the same catalog — and therefore
/// the same header bytes.
fn assign_catalog_ids(
    base: &[(i64, Codec)],
    dict_names: &BTreeSet<&str>,
) -> Result<BTreeMap<CatalogKey, i64>, WriterError> {
    let mut ids: BTreeMap<CatalogKey, i64> = BTreeMap::new();
    let mut seen_names: BTreeSet<&str> = BTreeSet::new();
    let mut seen_ids: BTreeSet<i64> = BTreeSet::new();
    for (id, codec) in base {
        if !seen_names.insert(&codec.name) {
            return Err(WriterError::InvalidFrame(FrameError::DuplicateCodecName {
                name: codec.name.clone(),
            }));
        }
        if !seen_ids.insert(*id) {
            return Err(WriterError::InvalidFrame(FrameError::DuplicateCodecId {
                id: *id,
            }));
        }
        ids.insert((codec.name.clone(), None), *id);
    }
    if dict_names.is_empty() {
        return Ok(ids);
    }
    let maximum = base.iter().map(|(id, _)| *id).max().unwrap_or(-1);
    let mut next = maximum
        .checked_add(1)
        .ok_or_else(|| WriterError::InvalidFrame(FrameError::DictionaryIdStart { maximum }))?;
    let dict_capable: BTreeSet<String> = ids
        .keys()
        .filter(|(name, _)| is_dict_capable(name))
        .map(|(name, _)| name.clone())
        .collect();
    let mut remaining = dict_capable
        .len()
        .checked_mul(dict_names.len())
        .ok_or_else(|| {
            WriterError::InvalidFrame(FrameError::DictionaryEntryCount {
                codecs: dict_capable.len(),
                dictionaries: dict_names.len(),
            })
        })?;
    for name in dict_capable {
        for dict in dict_names {
            ids.insert((name.clone(), Some((*dict).to_string())), next);
            remaining -= 1;
            if remaining > 0 {
                next = next.checked_add(1).ok_or_else(|| {
                    WriterError::InvalidFrame(FrameError::DictionaryIdRange {
                        last: next,
                        remaining,
                    })
                })?;
            }
        }
    }
    Ok(ids)
}

/// Whether a codec accepts an in-band `dct` dictionary (§8.5).
fn is_dict_capable(name: &str) -> bool {
    matches!(name, "zstd" | "zstd-rsyncable")
}

impl Writer {
    /// Create a writer and emit the Header (the chain genesis).
    pub fn new(profile: &str) -> Self {
        Self::with_layout(profile, None)
    }

    /// Build a deterministic single-segment writer from folded graph state.
    ///
    /// This high-level authoring path remaps terms by semantic value, emits
    /// authorable graph frames in a fixed order, and relies on deterministic
    /// CBOR for every hashed frame. It does not replay reader observations such
    /// as diagnostics, signatures, opaque nodes, or segment ledgers.
    pub fn deterministic(graph: &Graph, profile: &str) -> Result<Self, CodecError> {
        let remap = deterministic_term_remap(graph);
        let mut writer = Self::new(profile);

        if !remap.old_by_new.is_empty() {
            let terms: Vec<Term> = remap
                .old_by_new
                .iter()
                .map(|&old| remap_term(&graph.terms[old], &remap.old_to_new))
                .collect();
            writer.add_terms(&terms);
        }

        let mut quads: Vec<Quad> = graph
            .quads
            .iter()
            .map(|&(s, p, o, g)| {
                (
                    remap_id(&remap.old_to_new, s),
                    remap_id(&remap.old_to_new, p),
                    remap_id(&remap.old_to_new, o),
                    g.map(|term| remap_id(&remap.old_to_new, term)),
                )
            })
            .collect();
        // `quad_key` encodes a CBOR row per call; cache it once per element
        // instead of re-encoding on every comparison (stable sort, same order).
        quads.sort_by_cached_key(quad_key);
        if !quads.is_empty() {
            writer.add_quads(&quads);
        }

        let mut reifiers: Vec<ReifierRow> = graph
            .reifiers
            .iter()
            .map(|&(rid, (s, p, o), g)| {
                (
                    remap_id(&remap.old_to_new, rid),
                    (
                        remap_id(&remap.old_to_new, s),
                        remap_id(&remap.old_to_new, p),
                        remap_id(&remap.old_to_new, o),
                    ),
                    g.map(|term| remap_id(&remap.old_to_new, term)),
                )
            })
            .collect();
        // The key is a permutation of every field, so equal keys are identical
        // rows and an unstable sort yields the same array without the buffer.
        reifiers.sort_unstable_by_key(reifier_key);
        if !reifiers.is_empty() {
            writer.add_reifies(&reifiers);
        }

        let mut annotations: Vec<AnnotationRow> = graph
            .annotations
            .iter()
            .map(|&(r, p, v, g)| {
                (
                    remap_id(&remap.old_to_new, r),
                    remap_id(&remap.old_to_new, p),
                    remap_id(&remap.old_to_new, v),
                    g.map(|term| remap_id(&remap.old_to_new, term)),
                )
            })
            .collect();
        // Key covers every field (see `reifiers` above): unstable == stable here.
        annotations.sort_unstable_by_key(annotation_key);
        if !annotations.is_empty() {
            writer.add_annot(&annotations);
        }

        let mut blobs: Vec<(String, Vec<u8>)> = graph
            .blobs
            .iter()
            .map(|(digest, entry)| Ok((digest.clone(), entry.decoded_vec()?)))
            .collect::<Result<_, CodecError>>()?;
        blobs.sort_by(|a, b| a.0.cmp(&b.0));
        // One O(m log m) index instead of an O(n*m) linear `find` per blob;
        // `or_insert` keeps the FIRST entry for a digest, exactly as `find` did.
        let mut meta_by_digest: BTreeMap<&str, &Value> = BTreeMap::new();
        for (digest, meta) in &graph.blob_meta {
            meta_by_digest.entry(digest.as_str()).or_insert(meta);
        }
        for (digest, data) in blobs {
            let meta = meta_by_digest.get(digest.as_str()).copied();
            // Borrow the media type / representation straight out of the meta
            // map; `add_blob_owned` takes `Option<&str>`, so no copies needed.
            let mt = meta.and_then(|value| map_text(value, "mt"));
            let rep = meta.and_then(|value| map_text(value, "rep"));
            writer.add_blob_owned(data, mt, rep);
        }

        if !graph.meta.is_empty() {
            let entries: Vec<(Value, Value)> = graph
                .meta
                .iter()
                .map(|(key, value)| (key.clone().into(), value.clone()))
                .collect();
            writer.add_meta(Value::Map(entries));
        }

        let mut suppressions: Vec<Suppression> = graph
            .suppressions
            .iter()
            .map(|suppression| remap_suppression(suppression, &remap.old_to_new))
            .collect();
        // `suppression_key` canonicalises a CBOR map (cloning every target) per
        // call; cache it once per element rather than per comparison.
        suppressions.sort_by_cached_key(suppression_key);
        for suppression in suppressions {
            writer.add_suppress(
                suppression.targets,
                suppression.reason.as_deref(),
                suppression.by,
            );
        }

        Ok(writer)
    }

    /// Create a writer with a header layout-state claim (§3.3;
    /// `"streamable"` is the only value this revision defines).
    pub fn with_layout(profile: &str, layout: Option<&str>) -> Self {
        let options = WriterOptions {
            layout: layout.map(str::to_string),
            ..WriterOptions::default()
        };
        Self::with_options(profile, options).expect("unsupported layout claim")
    }

    /// Create a writer with explicit header options.
    pub fn with_options(profile: &str, options: WriterOptions) -> Result<Self, WriterError> {
        // §5: "streamable" is the only layout this revision defines; a typo'd
        // claim would persist into the tamper-evident header.
        if options
            .layout
            .as_deref()
            .is_some_and(|layout| layout != "streamable")
        {
            return Err(WriterError::InvalidFrame(FrameError::Layout {
                actual: options.layout,
            }));
        }
        let base_catalog = options.catalog.unwrap_or_else(default_catalog);

        // Pinned dictionaries, keyed by name. A duplicate or empty name is a
        // hard error: `"dct"` is a map, so a duplicate would silently drop one
        // dictionary's bytes while frames still referenced it by name.
        let mut dicts: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        for (name, bytes) in options.dicts {
            if name.is_empty() {
                return Err(WriterError::InvalidFrame(FrameError::EmptyDictionaryName));
            }
            if dicts.insert(name.clone(), bytes).is_some() {
                return Err(WriterError::InvalidFrame(
                    FrameError::DuplicateDictionaryName { name },
                ));
            }
        }
        let dict_names: BTreeSet<&str> = dicts.keys().map(String::as_str).collect();
        if !dict_names.is_empty()
            && !base_catalog
                .iter()
                .any(|(_, codec)| is_dict_capable(&codec.name))
        {
            // Fail closed: pinning a dictionary no catalog entry can reference
            // would ship dead weight no frame could ever name.
            return Err(WriterError::MissingCatalogEntry("zstd".to_string()));
        }
        let catalog_ids = assign_catalog_ids(&base_catalog, &dict_names)?;

        // The declared level is only meaningful on a zstd-family entry.
        if let Some(level) = options.zstd_level
            && !catalog_ids.keys().any(|(name, _)| is_dict_capable(name))
        {
            return Err(WriterError::InvalidFrame(FrameError::CatalogLevel {
                level,
            }));
        }

        let cls_by_name: BTreeMap<&str, &str> = base_catalog
            .iter()
            .map(|(_, codec)| (codec.name.as_str(), codec.cls.as_str()))
            .collect();
        let cat_entries: Vec<(Value, Value)> = catalog_ids
            .iter()
            .map(|((name, dict), id)| {
                let cls = cls_by_name.get(name.as_str()).copied().unwrap_or("encode");
                let mut ce: Vec<(Value, Value)> = vec![
                    ("name".into(), name.clone().into()),
                    ("cls".into(), cls.to_string().into()),
                ];
                if let Some(dict) = dict {
                    // §5 codec `"dct"` references the header `"dct"` map key.
                    ce.push(("dct".into(), dict.clone().into()));
                }
                if let Some(level) = options.zstd_level.filter(|_| is_dict_capable(name)) {
                    // §8.5 `level?`: declared so the authoring level is
                    // recoverable from the wire, not merely asserted.
                    ce.push(("level".into(), Value::from(i64::from(level))));
                }
                (Value::from(*id), Value::Map(ce))
            })
            .collect();

        let mut header: Vec<(Value, Value)> = vec![
            ("gts".into(), "GTS1".into()),
            ("v".into(), Value::from(1)),
            ("prof".into(), profile.into()),
            ("cat".into(), Value::Map(cat_entries)),
        ];
        if let Some(layout) = options.layout {
            // The layout-state claim is part of the header content, so it is
            // covered by the genesis self-hash (§3.3, §5).
            header.push(("layout".into(), layout.into()));
        }
        if !dicts.is_empty() {
            // §5: dictionaries are stored uncompressed and in-band; the header
            // "dct" map covers the genesis self-hash below like every other key.
            header.push((
                "dct".into(),
                Value::Map(
                    dicts
                        .iter()
                        .map(|(name, bytes)| (name.clone().into(), Value::Bytes(bytes.clone())))
                        .collect(),
                ),
            ));
        }
        if let Some(meta) = options.meta {
            header.push(("meta".into(), meta));
        }
        let id = header_id(&header);
        header.push(("id".into(), Value::Bytes(id.clone())));

        let header_value = Value::Map(header);
        let buf = if options.magic_tag {
            canonical(&Value::Tag(SELF_DESCRIBE_TAG, Box::new(header_value)))
        } else {
            canonical(&header_value)
        };

        Ok(Self {
            catalog_ids,
            prev: id,
            buf,
            offsets: Vec::new(),
            types: Vec::new(),
            frame_ids: Vec::new(),
            signer: None,
            mode: Infallible,
            dicts,
            declared_zstd_level: options.zstd_level,
        })
    }

    /// Continue an EXISTING segment: no header is emitted, and appended frames
    /// chain from `head` against `catalog`/`dicts` recovered from that header.
    ///
    /// This is the counterpart to the header-minting constructors: a store that
    /// grows one claim at a time must not pay a full GTS header (plus a repeat
    /// of every in-band dictionary — kilobytes) per claim. [`Self::into_bytes`]
    /// returns ONLY the appended frames, ready to append to the existing file.
    ///
    /// # Errors
    /// Returns [`WriterError::Append`] when `existing` carries no readable
    /// segment header, or its frame chain cannot be walked to a head id.
    pub fn appending(existing: &[u8]) -> Result<Self, WriterError> {
        let state = crate::reader::segment_append_state(existing).map_err(WriterError::Append)?;
        Self::continuing(state)
    }

    /// Continue from an already-recovered append state without re-scanning the
    /// underlying file. Used by append-heavy in-crate facades that cache the
    /// last segment's catalog and head between writes.
    ///
    /// # Errors
    /// Returns [`WriterError::InvalidFrame`] when the cached catalog is
    /// ambiguous or declares conflicting zstd authoring levels.
    pub(crate) fn continuing(
        state: crate::reader::SegmentAppendState,
    ) -> Result<Self, WriterError> {
        // Adopt the catalog that is actually ON THE WIRE — appended frames name
        // ids from the existing header, never a freshly-derived assignment.
        let mut catalog_ids: BTreeMap<CatalogKey, i64> = BTreeMap::new();
        let mut seen_ids: BTreeSet<i64> = BTreeSet::new();
        for row in &state.catalog {
            if !seen_ids.insert(row.id) {
                return Err(WriterError::InvalidFrame(FrameError::AppendDuplicateId {
                    id: row.id,
                }));
            }
            let key = (row.name.clone(), row.dct.clone());
            if catalog_ids.insert(key.clone(), row.id).is_some() {
                return Err(WriterError::InvalidFrame(
                    FrameError::AppendDuplicateEntry {
                        name: key.0,
                        dictionary: key.1,
                    },
                ));
            }
        }
        let declared_levels: BTreeSet<i32> = state
            .catalog
            .iter()
            .filter(|row| is_dict_capable(&row.name))
            .filter_map(|row| row.level)
            .collect();
        if declared_levels.len() > 1 {
            return Err(WriterError::InvalidFrame(FrameError::AppendLevels {
                levels: declared_levels,
            }));
        }
        let declared_zstd_level = declared_levels.first().copied();
        let (dicts, head) = (state.dicts, state.head);
        Ok(Self {
            catalog_ids,
            prev: head,
            buf: Vec::new(),
            offsets: Vec::new(),
            types: Vec::new(),
            frame_ids: Vec::new(),
            signer: None,
            mode: Infallible,
            dicts,
            declared_zstd_level,
        })
    }
}

impl<M: FrameMode> Writer<M> {
    /// Consume this writer into fallible hedged authorship, preserving its exact
    /// output, chain and index state. Each composite signature requests a fresh
    /// randomizer from `provider`. No provider can be omitted.
    ///
    /// ```compile_fail
    /// use purrdf_gts::{writer::Writer, cose::composite::SigningKey};
    /// let writer = Writer::new("purrdf.gts");
    /// let key = SigningKey::from_bytes(&[0; 64]).unwrap();
    /// let writer = writer.with_composite_signer(key, b"key");
    /// ```
    pub fn with_composite_signer<P: RandomnessProvider>(
        self,
        key: crate::cose::composite::SigningKey,
        kid: impl AsRef<[u8]>,
        provider: P,
    ) -> Writer<Hedged<P>> {
        Writer {
            catalog_ids: self.catalog_ids,
            prev: self.prev,
            buf: self.buf,
            offsets: self.offsets,
            types: self.types,
            frame_ids: self.frame_ids,
            signer: Some(FrameSigner::Composite(Box::new(key), kid.as_ref().to_vec())),
            mode: Hedged { provider },
            dicts: self.dicts,
            declared_zstd_level: self.declared_zstd_level,
        }
    }

    /// Sign every subsequently appended frame's id with this Ed25519 key (§9.2).
    pub fn sign_with(&mut self, key: purrdf_ed25519::SigningKey, kid: &str) {
        self.sign_ed25519_with(key, kid.as_bytes());
    }

    /// Sign subsequent frames with Ed25519 and an explicit opaque byte-string id.
    pub fn sign_ed25519_with(&mut self, key: purrdf_ed25519::SigningKey, kid: &[u8]) {
        self.signer = Some(FrameSigner::Ed25519(Box::new(key), kid.to_vec()));
    }

    /// Sign every subsequently appended frame with an unencrypted OpenPGP Ed25519 secret key.
    ///
    /// When `kid_override` is `None`, the COSE key id defaults to the key's
    /// OpenPGP v4 fingerprint.
    pub fn sign_with_openpgp_secret_key(
        &mut self,
        armored: &str,
        kid_override: Option<&str>,
    ) -> Result<(), crate::openpgp::OpenPgpError> {
        let signer = crate::openpgp::parse_secret_signing_key(armored, kid_override)?;
        let (key, kid) = signer.into_parts();
        self.sign_with(key, &kid);
        Ok(())
    }

    /// The id the next appended frame must reference as `"prev"`.
    pub fn head(&self) -> &[u8] {
        &self.prev
    }

    /// Resolve a transform chain to catalog ids, binding every zstd-family
    /// codec to `dict` when one is selected.
    ///
    /// A dict-capable codec with a selected dictionary MUST resolve to the
    /// `(codec, dict)` catalog entry — silently falling back to the plain entry
    /// would emit frames whose declared chain does not describe how they were
    /// actually encoded, and no reader could recover the payload.
    fn chain_ids(&self, chain: &[String], dict: Option<&str>) -> Result<Vec<i64>, WriterError> {
        chain
            .iter()
            .map(|name| {
                let key = match dict {
                    Some(dict) if is_dict_capable(name) => (name.clone(), Some(dict.to_string())),
                    _ => (name.clone(), None),
                };
                self.catalog_ids
                    .get(&key)
                    .copied()
                    .ok_or_else(|| WriterError::MissingCatalogEntry(name.clone()))
            })
            .collect()
    }

    /// Look up a codec id by canonical name with no dictionary bound.
    fn plain_codec_id(&self, name: &str) -> Result<i64, WriterError> {
        self.catalog_ids
            .get(&(name.to_string(), None))
            .copied()
            .ok_or_else(|| WriterError::MissingCatalogEntry(name.to_string()))
    }

    /// Append one frame and return its `"id"`.
    pub fn add_frame(
        &mut self,
        frame_type: &str,
        payload: Option<Value>,
        raw: Option<Vec<u8>>,
        transform: Option<&[String]>,
        pub_meta: Option<Value>,
    ) -> M::FrameResult {
        let mut options = FrameOptions {
            payload,
            raw,
            pub_meta,
            ..FrameOptions::default()
        };
        if let Some(transform) = transform {
            options.transform = transform.to_vec();
        }
        M::finish(self.add_frame_with_options(frame_type, options))
    }

    /// Append one frame with explicit transform/encryption/signature options.
    ///
    /// The writer computes the content id before adding `sig`, matching the
    /// frame-id preimage used by readers and detached verifiers. The stored
    /// `prev` link always names the prior head in this writer, maintaining a
    /// single append-only segment chain.
    pub fn add_frame_with_options(
        &mut self,
        frame_type: &str,
        options: FrameOptions,
    ) -> Result<Vec<u8>, WriterError> {
        let FrameOptions {
            payload,
            raw,
            transform,
            zstd_level,
            dict,
            pub_meta,
            mut recipients,
            signature,
            encrypt,
        } = options;

        if payload.is_some() && raw.is_some() {
            return Err(WriterError::InvalidFrame(FrameError::ConflictingSources));
        }
        let transforms_data = !transform.is_empty() || encrypt.is_some();
        if transforms_data && payload.is_none() && raw.is_none() {
            return Err(WriterError::InvalidFrame(FrameError::MissingSource {
                transforms: transform.len(),
                encrypted: encrypt.is_some(),
            }));
        }
        if let Some(level) = zstd_level
            && !transform.iter().any(|name| is_dict_capable(name))
        {
            return Err(WriterError::InvalidFrame(FrameError::FrameLevel { level }));
        }
        // A frame may not encode at a level the catalog does not declare — the
        // declared `level` is a wire fact readers gate on, not a hint.
        if let (Some(declared), Some(requested)) = (self.declared_zstd_level, zstd_level)
            && declared != requested
        {
            return Err(WriterError::InvalidFrame(FrameError::LevelMismatch {
                requested,
                declared,
            }));
        }
        // Fail closed on an unknown dictionary name: never a silent no-dict encode.
        let dict_bytes: Option<&[u8]> = match dict.as_deref() {
            None => None,
            Some(name) => Some(self.dicts.get(name).map(Vec::as_slice).ok_or_else(|| {
                WriterError::InvalidFrame(FrameError::UnknownDictionary {
                    name: name.to_owned(),
                })
            })?),
        };
        if let Some(name) = dict.as_ref()
            && !transform.iter().any(|name| is_dict_capable(name))
        {
            return Err(WriterError::InvalidFrame(FrameError::DictionaryTransform {
                name: name.clone(),
            }));
        }
        let mut frame: Vec<(Value, Value)> = vec![("t".into(), frame_type.into())];

        let data: Option<Value> = if transforms_data {
            let mut source = match (raw, payload) {
                (Some(raw), _) => raw,
                (None, Some(payload)) => canonical(&payload),
                (None, None) => unreachable!("validated transform source above"),
            };
            let mut x_ids: Vec<i64> = self.chain_ids(&transform, dict.as_deref())?;
            if !transform.is_empty() {
                // §8.5: BOTH zstd-family transforms take a dictionary. For
                // `zstd-rsyncable` the dictionary primes each independent block,
                // so the block-boundary/delta property survives intact.
                source = encode_chain_with_options(
                    &transform,
                    &source,
                    EncodeOptions {
                        // Inherit the declared pack level, but only for a chain
                        // that actually carries a zstd-family transform.
                        zstd_level: zstd_level.or_else(|| {
                            self.declared_zstd_level
                                .filter(|_| transform.iter().any(|name| is_dict_capable(name)))
                        }),
                        dict: dict_bytes,
                    },
                )?;
            }
            if let Some(encrypt) = encrypt {
                let encrypt_id = self.plain_codec_id("cose-encrypt0")?;
                source = crate::cose::encrypt0(&source, &encrypt.kid, &encrypt.key, &encrypt.iv);
                x_ids.push(encrypt_id);
                recipients.push(Value::Map(vec![("kid".into(), encrypt.kid.into())]));
            }
            frame.push((
                "x".into(),
                Value::Array(x_ids.into_iter().map(Value::from).collect()),
            ));
            Some(Value::Bytes(source))
        } else {
            match (raw, payload) {
                (Some(raw), _) => Some(Value::Bytes(raw)),
                (None, Some(payload)) => Some(payload),
                _ => None,
            }
        };
        if let Some(data) = data {
            frame.push(("d".into(), data));
        }

        if let Some(meta) = pub_meta {
            frame.push(("pub".into(), meta));
        }
        if !recipients.is_empty() {
            frame.push(("to".into(), Value::Array(recipients)));
        }
        frame.push(("prev".into(), Value::Bytes(self.prev.clone())));

        // The signature is not part of the frame-id preimage (§9.2). This lets
        // signatures be verified against the same id after streamable
        // compaction carries them as detached evidence.
        let id = content_id(&frame);
        frame.push(("id".into(), Value::Bytes(id.clone())));
        let sig = match signature {
            Some(sig) => Some(sig),
            None => match &self.signer {
                None => None,
                Some(FrameSigner::Ed25519(key, kid)) => Some(
                    crate::cose::sign_id_hedged(
                        &id,
                        crate::cose::SigningKeyRef::Ed25519(key),
                        kid,
                        &[0; 32],
                    )
                    .map_err(WriterError::Signing)?,
                ),
                Some(FrameSigner::Composite(key, kid)) => {
                    let mut randomizer = Randomizer([0; 32]);
                    self.mode.fill(&mut randomizer.0)?;
                    Some(
                        crate::cose::sign_id_hedged(
                            &id,
                            crate::cose::SigningKeyRef::Composite(key),
                            kid,
                            &randomizer.0,
                        )
                        .map_err(WriterError::Signing)?,
                    )
                }
            },
        };
        if let Some(sig) = sig {
            frame.push(("sig".into(), Value::Bytes(sig)));
        }
        self.offsets.push(self.buf.len());
        self.types.push(frame_type.to_string());
        self.frame_ids.push(id.clone());
        append_canonical(&Value::Map(frame), &mut self.buf);
        self.prev.clone_from(&id);
        Ok(id)
    }

    /// Append a `terms` frame.
    pub fn add_terms(&mut self, terms: &[Term]) -> M::FrameResult {
        self.add_frame("terms", Some(terms_payload(terms)), None, None, None)
    }

    /// Append a `quads` frame (graph slot dropped when `None`).
    pub fn add_quads(&mut self, quads: &[Quad]) -> M::FrameResult {
        self.add_frame("quads", Some(quads_payload(quads)), None, None, None)
    }

    /// Append a `reifies` frame.
    pub fn add_reifies(&mut self, bindings: &[ReifierRow]) -> M::FrameResult {
        self.add_frame("reifies", Some(reifies_payload(bindings)), None, None, None)
    }

    /// Append an `annot` frame.
    pub fn add_annot(&mut self, rows: &[AnnotationRow]) -> M::FrameResult {
        self.add_frame("annot", Some(annot_payload(rows)), None, None, None)
    }

    /// Build the `pub` metadata map (`digest`/`mt`/`rep`) shared by every blob-frame writer.
    fn blob_pub_meta(data: &[u8], mt: Option<&str>, rep: Option<&str>) -> Option<Value> {
        let mut pub_entries: Vec<(Value, Value)> = vec![("digest".into(), digest_str(data).into())];
        if let Some(m) = mt {
            pub_entries.push(("mt".into(), m.into()));
        }
        if let Some(r) = rep {
            pub_entries.push(("rep".into(), r.into()));
        }
        Some(Value::Map(pub_entries))
    }

    /// Append an inline `blob` frame; metadata goes in `pub` (§12).
    pub fn add_blob(&mut self, data: &[u8], mt: Option<&str>, rep: Option<&str>) -> M::FrameResult {
        self.add_blob_owned(data.to_vec(), mt, rep)
    }

    /// Append an owned inline `blob` frame without cloning the payload first.
    pub fn add_blob_owned(
        &mut self,
        data: Vec<u8>,
        mt: Option<&str>,
        rep: Option<&str>,
    ) -> M::FrameResult {
        let pub_meta = Self::blob_pub_meta(&data, mt, rep);
        self.add_frame("blob", None, Some(data), None, pub_meta)
    }

    /// Append an owned inline `blob` frame whose payload is carried through
    /// `transform` (e.g. `["zstd-rsyncable"]`) before being stored in `"d"`,
    /// primed by the named in-band pack dictionary when `dict` is supplied.
    ///
    /// Metadata (`digest`/`mt`/`rep`) is computed over the pre-transform bytes,
    /// so readers observe the same content digest as an untransformed blob.
    ///
    /// # Errors
    /// Returns [`WriterError`] when `dict` names a dictionary this pack does not
    /// pin, or the transform chain cannot be resolved or applied.
    pub fn add_blob_transformed(
        &mut self,
        data: Vec<u8>,
        mt: Option<&str>,
        rep: Option<&str>,
        transform: &[String],
        dict: Option<&str>,
    ) -> Result<Vec<u8>, WriterError> {
        let pub_meta = Self::blob_pub_meta(&data, mt, rep);
        self.add_frame_with_options(
            "blob",
            FrameOptions {
                raw: Some(data),
                transform: transform.to_vec(),
                dict: dict.map(str::to_string),
                pub_meta,
                ..FrameOptions::default()
            },
        )
    }

    /// Append a `meta` frame.
    pub fn add_meta(&mut self, meta: Value) -> M::FrameResult {
        self.add_frame("meta", Some(meta), None, None, None)
    }

    /// Append a `suppress` frame.
    pub fn add_suppress(
        &mut self,
        targets: Vec<Value>,
        reason: Option<&str>,
        by: Option<usize>,
    ) -> M::FrameResult {
        let payload = suppress_payload(targets, reason, by);
        self.add_frame("suppress", Some(payload), None, None, None)
    }

    /// Append an `index` footer covering every frame appended so far (§6.2).
    ///
    /// `count`/`head` delimit the covered region (the streamable boundary,
    /// §3.3); `off` carries each covered frame's byte offset from the start
    /// of this writer's output; `ti` locates frames by type (0-based frame
    /// positions). A later `add_index` covers the earlier one too — the last
    /// index wins (§6.2).
    fn add_index_impl(&mut self, include_mmr: bool) -> M::FrameResult {
        let mut payload: Vec<(Value, Value)> = vec![
            ("count".into(), Value::from(self.types.len())),
            ("head".into(), Value::Bytes(self.prev.clone())),
        ];
        if include_mmr {
            payload.push((
                "mmr".into(),
                Value::Bytes(crate::mmr::root(&self.frame_ids)),
            ));
        }
        if !self.offsets.is_empty() {
            // "off"/"ti" are [+ uint]-shaped — omit when empty
            let off: Vec<Value> = self.offsets.iter().map(|&o| Value::from(o)).collect();
            let mut ti: Vec<(Value, Value)> = Vec::new();
            for (pos, ftype) in self.types.iter().enumerate() {
                match ti
                    .iter_mut()
                    .find(|(k, _)| matches!(k, Value::Text(t) if t == ftype))
                {
                    Some((_, Value::Array(positions))) => positions.push(Value::from(pos)),
                    _ => ti.push((ftype.clone().into(), Value::Array(vec![Value::from(pos)]))),
                }
            }
            payload.push(("off".into(), Value::Array(off)));
            payload.push(("ti".into(), Value::Map(ti)));
        }
        self.add_frame("index", Some(Value::Map(payload)), None, None, None)
    }

    /// Append an `index` footer covering the frames written so far and return its frame id.
    pub fn add_index(&mut self) -> M::FrameResult {
        self.add_index_impl(false)
    }

    /// Append an `index` footer with the optional `mmr` root over covered frame ids.
    ///
    /// This is opt-in so existing byte-oracle corpus vectors and cross-engine
    /// compact output remain stable until other engines claim the proof tier.
    pub fn add_index_with_mmr(&mut self) -> M::FrameResult {
        self.add_index_impl(true)
    }

    /// Return the complete GTS file bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.buf.clone()
    }

    /// Consume the writer and return the complete GTS file bytes without cloning.
    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }
}

impl<P: RandomnessProvider> Writer<Hedged<P>> {
    /// Rotate to a dedicated composite key while retaining the provider and
    /// fallible append contract. Provider freshness is required per signature.
    pub fn sign_composite_with(&mut self, key: crate::cose::composite::SigningKey, kid: &[u8]) {
        self.signer = Some(FrameSigner::Composite(Box::new(key), kid.to_vec()));
    }

    /// Access the caller's provider, for explicit recovery or reconfiguration
    /// after a failed request. No writer state is appended on provider failure.
    pub fn randomness_provider_mut(&mut self) -> &mut P {
        &mut self.mode.provider
    }
}

/// Deterministic term-id remapping for canonical graph authorship.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TermRemap {
    /// New term id for each old term id.
    pub old_to_new: Vec<usize>,
    /// Old term ids in new-id order.
    pub old_by_new: Vec<usize>,
}

/// Structural nesting depth of a term: `0` for everything that is not a quoted
/// triple, and one more than its deepest component for one that is.
///
/// This is what makes the canonical id order safe for a self-describing triple
/// term (wire `"tt"`), whose components MUST be introduced before it. Content
/// order alone does not guarantee that: `<<( <a> <p> <<( <z> <p> <b> )>> )>>`
/// sorts BEFORE its own inner triple, because the comparison reaches the
/// subjects (`<a>` vs `<z>`) before it ever notices the nesting. Ordering by
/// depth first fixes that without disturbing anything else — every non-triple
/// term is depth `0`, so their relative order is untouched, and quoted triples
/// already sorted after them (`"triple"` is last among the kind tags).
///
/// A malformed cyclic graph cannot make this walk forever: a term already on
/// `stack` — the path from the term the walk started at — contributes depth `0`.
///
/// The walk runs over a work list: a quoted triple's subject, predicate and object
/// are each walked fully, in that order, with the triple on `stack` while they are.
#[cfg(test)]
fn term_nesting_depth(graph: &Graph, tid: usize, stack: &mut Vec<usize>) -> usize {
    let mut active = vec![false; graph.terms.len()];
    term_nesting_depth_marked(graph, tid, stack, &mut active)
}

fn term_nesting_depth_marked(
    graph: &Graph,
    tid: usize,
    stack: &mut Vec<usize>,
    active: &mut [bool],
) -> usize {
    /// The depth of `tid` if it has no components to walk, or its components.
    fn enter(graph: &Graph, tid: usize, active: &[bool]) -> Result<usize, [usize; 3]> {
        if active.get(tid).copied().unwrap_or(false) {
            return Ok(0);
        }
        let Some(term) = graph.terms.get(tid) else {
            return Ok(0);
        };
        if term.kind != TermKind::Triple {
            return Ok(0);
        }
        graph
            .triple_of(tid)
            .map_or(Ok(1), |components| Err(components.into()))
    }
    /// A quoted triple being walked: its components, how many have been entered, and
    /// the deepest of those finished.
    struct Frame {
        components: [usize; 3],
        entered: usize,
        deepest: usize,
    }
    let components = match enter(graph, tid, active) {
        Ok(depth) => return depth,
        Err(components) => components,
    };
    active[tid] = true;
    stack.push(tid);
    let mut frames = vec![Frame {
        components,
        entered: 0,
        deepest: 0,
    }];
    loop {
        let frame = frames.last_mut().expect("a triple is being walked");
        if frame.entered < 3 {
            let component = frame.components[frame.entered];
            frame.entered += 1;
            match enter(graph, component, active) {
                Ok(depth) => frame.deepest = frame.deepest.max(depth),
                Err(components) => {
                    active[component] = true;
                    stack.push(component);
                    frames.push(Frame {
                        components,
                        entered: 0,
                        deepest: 0,
                    });
                }
            }
            continue;
        }
        let depth = frame.deepest + 1;
        frames.pop();
        let finished = stack.pop().expect("the active triple is on the path");
        active[finished] = false;
        match frames.last_mut() {
            Some(parent) => parent.deepest = parent.deepest.max(depth),
            None => return depth,
        }
    }
}

/// Return the deterministic term-id remapping used by canonical graph writers.
pub fn deterministic_term_remap(graph: &Graph) -> TermRemap {
    remap_with_identity_work(graph).0
}

/// Work and retained identity bytes of one remap. The ordinary writer discards
/// this record; scaling controls read the same production traversal's counts.
#[derive(Default)]
struct IdentityWork {
    source_terms: usize,
    source_edges: usize,
    compared_nodes: usize,
    interned_nodes: usize,
    interned_edges: usize,
    flat_identity_bytes: usize,
    path_membership_probes: usize,
    peak_identity_bytes: usize,
}

fn remap_with_identity_work(graph: &Graph) -> (TermRemap, IdentityWork) {
    let mut work = IdentityWork::default();
    let mut old_by_new: Vec<usize> = (0..graph.terms.len()).collect();
    let mut stack = Vec::new();
    let mut active = vec![false; graph.terms.len()];
    if let Some(depths) = acyclic_term_depths(graph, &mut work) {
        // A triple identity is a canonical CBOR array of three child identities.
        // Keep the one flat encoding per leaf; the child edges already in `Graph`
        // share the nested identity. No expanded key is retained per ancestor.
        let flat_keys: Vec<Option<Vec<u8>>> = graph
            .terms
            .iter()
            .enumerate()
            .map(|(tid, term)| {
                (term.kind != TermKind::Triple || graph.term_triple(term).is_none())
                    .then(|| term_identity_key_marked(graph, tid, &mut stack, &mut active))
            })
            .collect();
        work.flat_identity_bytes = flat_keys
            .iter()
            .filter_map(Option::as_ref)
            .map(Vec::len)
            .sum();
        work.path_membership_probes = flat_keys.iter().filter(|key| key.is_some()).count();
        let retained = flat_keys.capacity() * size_of::<Option<Vec<u8>>>()
            + flat_keys
                .iter()
                .filter_map(Option::as_ref)
                .map(Vec::capacity)
                .sum::<usize>()
            + depths.capacity() * size_of::<usize>()
            + active.capacity();
        work.peak_identity_bytes = work.peak_identity_bytes.max(retained);
        let classes = acyclic_identity_classes(graph, &depths, &flat_keys, &mut work);
        let mut compared_pairs: FastMap<(usize, usize), Ordering> = FastMap::default();
        old_by_new.sort_unstable_by(|&a, &b| {
            depths[a]
                .cmp(&depths[b])
                .then_with(|| {
                    compare_acyclic_identity(
                        graph,
                        &flat_keys,
                        &classes,
                        &mut compared_pairs,
                        a,
                        b,
                        &mut work,
                    )
                })
                .then_with(|| a.cmp(&b))
        });
        work.peak_identity_bytes +=
            compared_pairs.capacity() * (size_of::<((usize, usize), Ordering)>() + 1);
    } else {
        // A cyclic term's identity contains a path-relative ["cycle", id]
        // marker. Preserve that established malformed-input law exactly.
        let keys: Vec<(usize, Vec<u8>)> = old_by_new
            .iter()
            .map(|&tid| {
                (
                    term_nesting_depth_marked(graph, tid, &mut stack, &mut active),
                    term_identity_key_marked(graph, tid, &mut stack, &mut active),
                )
            })
            .collect();
        old_by_new.sort_unstable_by(|a, b| keys[*a].cmp(&keys[*b]).then_with(|| a.cmp(b)));
    }
    let mut old_to_new = vec![0; graph.terms.len()];
    for (new, old) in old_by_new.iter().enumerate() {
        old_to_new[*old] = new;
    }
    (
        TermRemap {
            old_to_new,
            old_by_new,
        },
        work,
    )
}

/// Depths of all terms in one pass over an acyclic component graph. A missing
/// component is a flat marker of depth zero; a cycle selects the path-sensitive
/// fallback above instead of silently inventing a depth for it.
fn acyclic_term_depths(graph: &Graph, work: &mut IdentityWork) -> Option<Vec<usize>> {
    let count = graph.terms.len();
    work.source_terms = count;
    let mut parents = vec![Vec::new(); count];
    let mut pending = vec![0_usize; count];
    let mut depths = vec![0_usize; count];
    for (tid, term) in graph.terms.iter().enumerate() {
        if term.kind != TermKind::Triple {
            continue;
        }
        depths[tid] = 1;
        if let Some(components) = graph.term_triple(term) {
            for child in <[usize; 3]>::from(components) {
                if child < count {
                    work.source_edges += 1;
                    parents[child].push(tid);
                    pending[tid] += 1;
                }
            }
        }
    }
    let mut ready: Vec<usize> = (0..count).filter(|&tid| pending[tid] == 0).collect();
    let mut finished = 0;
    while let Some(child) = ready.pop() {
        finished += 1;
        for &parent in &parents[child] {
            depths[parent] = depths[parent].max(depths[child] + 1);
            pending[parent] -= 1;
            if pending[parent] == 0 {
                ready.push(parent);
            }
        }
    }
    work.peak_identity_bytes = parents.capacity() * size_of::<Vec<usize>>()
        + parents
            .iter()
            .map(|parents| parents.capacity() * size_of::<usize>())
            .sum::<usize>()
        + pending.capacity() * size_of::<usize>()
        + depths.capacity() * size_of::<usize>()
        + ready.capacity() * size_of::<usize>();
    (finished == count).then_some(depths)
}

/// Intern exact structural identities bottom-up. Class numbers are equality
/// tokens only: canonical CBOR byte ordering remains in the comparator below.
/// Borrowing flat keys avoids a second retained copy of every lexical value.
fn acyclic_identity_classes(
    graph: &Graph,
    depths: &[usize],
    flat_keys: &[Option<Vec<u8>>],
    work: &mut IdentityWork,
) -> Vec<usize> {
    let mut flat_classes: FastMap<&[u8], usize> = FastMap::default();
    let mut missing_classes: FastMap<usize, usize> = FastMap::default();
    let mut triple_classes: FastMap<[usize; 3], usize> = FastMap::default();
    let mut classes = vec![0; graph.terms.len()];
    let mut next_class = 0;
    let mut ordered: Vec<usize> = (0..graph.terms.len()).collect();
    ordered.sort_unstable_by_key(|&tid| depths[tid]);
    for &tid in &ordered {
        work.interned_nodes += 1;
        if let Some(key) = flat_keys[tid].as_deref() {
            classes[tid] = *flat_classes.entry(key).or_insert_with(|| {
                let class = next_class;
                next_class += 1;
                class
            });
            continue;
        }
        let components: [usize; 3] = graph.triple_of(tid).expect("nested triple").into();
        let children = components.map(|child| {
            work.interned_edges += 1;
            if child < classes.len() {
                classes[child]
            } else {
                *missing_classes.entry(child).or_insert_with(|| {
                    let class = next_class;
                    next_class += 1;
                    class
                })
            }
        });
        classes[tid] = *triple_classes.entry(children).or_insert_with(|| {
            let class = next_class;
            next_class += 1;
            class
        });
    }
    let retained = classes.capacity() * size_of::<usize>()
        + ordered.capacity() * size_of::<usize>()
        + flat_classes.capacity() * size_of::<(&[u8], usize)>()
        + missing_classes.capacity() * size_of::<(usize, usize)>()
        + triple_classes.capacity() * size_of::<([usize; 3], usize)>();
    work.peak_identity_bytes = work.peak_identity_bytes.max(
        retained
            + size_of_val(flat_keys)
            + flat_keys
                .iter()
                .filter_map(Option::as_ref)
                .map(Vec::capacity)
                .sum::<usize>()
            + size_of_val(depths),
    );
    classes
}

/// Compare canonical CBOR identities without expanding quoted terms. Every
/// complete CBOR child value is self-delimiting, so the lexicographic byte
/// comparison of two four-element triple arrays is the comparison of their
/// first differing child identity. The work list keeps deep chains off the
/// native stack. Equal structural classes need no repeated walk, and bounded
/// pair-order memory avoids revisiting unequal child chains at each ancestor.
fn compare_acyclic_identity(
    graph: &Graph,
    flat_keys: &[Option<Vec<u8>>],
    classes: &[usize],
    compared_pairs: &mut FastMap<(usize, usize), Ordering>,
    left: usize,
    right: usize,
    work: &mut IdentityWork,
) -> Ordering {
    fn flat(flat_keys: &[Option<Vec<u8>>], tid: usize) -> Option<Cow<'_, [u8]>> {
        match flat_keys.get(tid) {
            Some(Some(key)) => Some(Cow::Borrowed(key)),
            Some(None) => None,
            None => Some(Cow::Owned(canonical(&Value::Array(vec![
                "missing".into(),
                Value::from(tid as u64),
            ])))),
        }
    }
    if matches!((classes.get(left), classes.get(right)), (Some(a), Some(b)) if a == b) {
        work.compared_nodes += 1;
        return Ordering::Equal;
    }
    // Most real term tables are flat. Their keys were encoded once above, so a
    // sort comparison must not allocate a work list for this common case.
    match (flat(flat_keys, left), flat(flat_keys, right)) {
        (Some(a), Some(b)) => {
            work.compared_nodes += 1;
            return a.cmp(&b);
        }
        (Some(a), None) => {
            work.compared_nodes += 1;
            return a[0].cmp(&0x84);
        }
        (None, Some(b)) => {
            work.compared_nodes += 1;
            return 0x84.cmp(&b[0]);
        }
        (None, None) => {}
    }
    let pair_key = (left.min(right), left.max(right));
    if let Some(&known) = compared_pairs.get(&pair_key) {
        work.compared_nodes += 1;
        return if left < right { known } else { known.reverse() };
    }
    let mut pending = vec![(left, right)];
    let order = loop {
        let Some((left, right)) = pending.pop() else {
            break Ordering::Equal;
        };
        work.compared_nodes += 1;
        if left == right {
            continue;
        }
        if let (Some(a), Some(b)) = (classes.get(left), classes.get(right))
            && a == b
        {
            continue;
        }
        if let Some(&known) = compared_pairs.get(&(left.min(right), left.max(right))) {
            let known = if left < right { known } else { known.reverse() };
            if known != Ordering::Equal {
                break known;
            }
            continue;
        }
        match (flat(flat_keys, left), flat(flat_keys, right)) {
            (Some(a), Some(b)) => {
                let order = a.cmp(&b);
                if order != Ordering::Equal {
                    break order;
                }
            }
            (Some(a), None) => break a[0].cmp(&0x84),
            (None, Some(b)) => break 0x84.cmp(&b[0]),
            (None, None) => {
                let a = graph.triple_of(left).expect("nested left triple");
                let b = graph.triple_of(right).expect("nested right triple");
                let a: [usize; 3] = a.into();
                let b: [usize; 3] = b.into();
                for index in (0..3).rev() {
                    pending.push((a[index], b[index]));
                }
            }
        }
    };
    if compared_pairs.len() >= graph.terms.len().saturating_mul(4) {
        compared_pairs.clear();
    }
    compared_pairs.insert(pair_key, if left < right { order } else { order.reverse() });
    order
}

impl Graph {
    /// Build the canonical payload for a single `snapshot` frame from this graph.
    pub fn snapshot_payload(&self) -> Value {
        snapshot_payload(self)
    }
}

/// Build the canonical payload for a single `snapshot` frame from a folded graph.
pub fn snapshot_payload(graph: &Graph) -> Value {
    let remap = deterministic_term_remap(graph);
    let terms: Vec<Value> = remap
        .old_by_new
        .iter()
        .map(|&old| term_to_wire(&remap_term(&graph.terms[old], &remap.old_to_new)))
        .collect();

    let mut quads: Vec<Quad> = graph
        .quads
        .iter()
        .map(|&(s, p, o, g)| {
            (
                remap_id(&remap.old_to_new, s),
                remap_id(&remap.old_to_new, p),
                remap_id(&remap.old_to_new, o),
                g.map(|term| remap_id(&remap.old_to_new, term)),
            )
        })
        .collect();
    // Key is a permutation of all four quad fields: equal keys are identical
    // rows, so the unstable sort yields the same array as the stable one.
    quads.sort_unstable_by_key(|quad| (quad.3, quad.0, quad.1, quad.2));

    let mut entries: Vec<(Value, Value)> = vec![
        ("terms".into(), Value::Array(terms)),
        (
            "quads".into(),
            Value::Array(
                quads
                    .iter()
                    .map(|&(s, p, o, g)| {
                        let mut row = vec![Value::from(s), Value::from(p), Value::from(o)];
                        if let Some(graph_name) = g {
                            row.push(Value::from(graph_name));
                        }
                        Value::Array(row)
                    })
                    .collect(),
            ),
        ),
    ];

    let mut reifiers: Vec<ReifierRow> = graph
        .reifiers
        .iter()
        .map(|&(rid, (s, p, o), g)| {
            (
                remap_id(&remap.old_to_new, rid),
                (
                    remap_id(&remap.old_to_new, s),
                    remap_id(&remap.old_to_new, p),
                    remap_id(&remap.old_to_new, o),
                ),
                g.map(|term| remap_id(&remap.old_to_new, term)),
            )
        })
        .collect();
    // Key covers every row field (see `deterministic`): unstable == stable.
    reifiers.sort_unstable_by_key(reifier_key);
    if !reifiers.is_empty() {
        entries.push((
            "reifies".into(),
            Value::Array(
                reifiers
                    .iter()
                    .map(|&(rid, (s, p, o), g)| {
                        let mut row = vec![
                            Value::from(rid),
                            Value::from(s),
                            Value::from(p),
                            Value::from(o),
                        ];
                        if let Some(graph_name) = g {
                            row.push(Value::from(graph_name));
                        }
                        Value::Array(row)
                    })
                    .collect(),
            ),
        ));
    }

    let mut annotations: Vec<AnnotationRow> = graph
        .annotations
        .iter()
        .map(|&(r, p, v, g)| {
            (
                remap_id(&remap.old_to_new, r),
                remap_id(&remap.old_to_new, p),
                remap_id(&remap.old_to_new, v),
                g.map(|term| remap_id(&remap.old_to_new, term)),
            )
        })
        .collect();
    // Key covers every row field (see `deterministic`): unstable == stable.
    annotations.sort_unstable_by_key(annotation_key);
    if !annotations.is_empty() {
        entries.push((
            "annot".into(),
            Value::Array(
                annotations
                    .iter()
                    .map(|&(r, p, v, g)| {
                        let mut row = vec![Value::from(r), Value::from(p), Value::from(v)];
                        if let Some(graph_name) = g {
                            row.push(Value::from(graph_name));
                        }
                        Value::Array(row)
                    })
                    .collect(),
            ),
        ));
    }

    Value::Map(entries)
}

/// The deterministic CBOR (RFC 8949 §4.2) of `tid`'s identity, the key canonical
/// term order sorts by.
///
/// A term that is not a quoted triple, or a quoted triple with no components, is one
/// flat array encoded by [`canonical`]: `["iri", value]`, `["literal", value, datatype,
/// language, direction]`, `["bnode", label]`, `["triple", null, reifier]`, or
/// `["cycle", id]` / `["missing", id]` for a term already on `stack` or out of range.
/// A quoted triple with components is the four-element array `["triple", s, p, o]`,
/// written directly: the array head (major type 4, length 4 — the one byte `0x84`),
/// the text `"triple"`, then each component's identity in turn.
///
/// The walk runs over a work list: each component is written fully, in order, with
/// the triple on `stack` — the path from the term the walk started at — while it is.
#[cfg(test)]
fn term_identity_key(graph: &Graph, tid: usize, stack: &mut Vec<usize>) -> Vec<u8> {
    let mut active = vec![false; graph.terms.len()];
    term_identity_key_marked(graph, tid, stack, &mut active)
}

fn term_identity_key_marked(
    graph: &Graph,
    tid: usize,
    stack: &mut Vec<usize>,
    active: &mut [bool],
) -> Vec<u8> {
    /// A quoted triple being written: its components and how many have begun.
    struct Frame {
        components: [usize; 3],
        entered: usize,
    }
    /// Write `tid`'s identity if it is flat, or open its array and return its frame.
    fn enter(graph: &Graph, tid: usize, active: &[bool], out: &mut Vec<u8>) -> Option<Frame> {
        let flat = if active.get(tid).copied().unwrap_or(false) {
            Value::Array(vec!["cycle".into(), Value::from(tid as u64)])
        } else {
            match graph.terms.get(tid) {
                None => Value::Array(vec!["missing".into(), Value::from(tid as u64)]),
                Some(term) => match term.kind {
                    TermKind::Iri => {
                        Value::Array(vec!["iri".into(), text_or_null(term.value.as_deref())])
                    }
                    TermKind::Literal => Value::Array(vec![
                        "literal".into(),
                        text_or_null(term.value.as_deref()),
                        // Borrowed datatype IRI: the owned `String` was copied into
                        // the `Value` and dropped immediately.
                        graph.datatype_iri_str(term).into(),
                        text_or_null(term.lang.as_deref()),
                        text_or_null(term.direction.as_deref()),
                    ]),
                    TermKind::Bnode => Value::Array(vec![
                        "bnode".into(),
                        match term.value.as_deref() {
                            Some(value) if !value.is_empty() => value.into(),
                            _ => Value::Array(vec!["anonymous".into(), Value::from(tid as u64)]),
                        },
                    ]),
                    TermKind::Triple => match graph.term_triple(term) {
                        Some(components) => {
                            out.push(0x84);
                            append_canonical(&Value::Text("triple".into()), out);
                            return Some(Frame {
                                components: components.into(),
                                entered: 0,
                            });
                        }
                        None => Value::Array(vec![
                            "triple".into(),
                            Value::Null,
                            term.reifier
                                .map_or(Value::Null, |rid| Value::from(rid as u64)),
                        ]),
                    },
                },
            }
        };
        append_canonical(&flat, out);
        None
    }
    let mut out = Vec::new();
    let Some(root) = enter(graph, tid, active, &mut out) else {
        return out;
    };
    active[tid] = true;
    stack.push(tid);
    let mut frames = vec![root];
    while let Some(frame) = frames.last_mut() {
        if frame.entered == 3 {
            frames.pop();
            let finished = stack.pop().expect("the active triple is on the path");
            active[finished] = false;
            continue;
        }
        let component = frame.components[frame.entered];
        frame.entered += 1;
        if let Some(inner) = enter(graph, component, active, &mut out) {
            active[component] = true;
            stack.push(component);
            frames.push(inner);
        }
    }
    out
}

fn text_or_null(value: Option<&str>) -> Value {
    value.map_or(Value::Null, Value::from)
}

fn remap_id(old_to_new: &[usize], tid: usize) -> usize {
    old_to_new.get(tid).copied().unwrap_or(tid)
}

fn remap_term(term: &Term, old_to_new: &[usize]) -> Term {
    Term {
        kind: term.kind,
        value: term.value.clone(),
        datatype: term.datatype.map(|tid| remap_id(old_to_new, tid)),
        lang: term.lang.clone(),
        direction: term.direction.clone(),
        reifier: term.reifier.map(|tid| remap_id(old_to_new, tid)),
        triple: term.triple.map(|(s, p, o)| {
            (
                remap_id(old_to_new, s),
                remap_id(old_to_new, p),
                remap_id(old_to_new, o),
            )
        }),
    }
}

fn quad_key(quad: &Quad) -> Vec<u8> {
    let mut row = vec![
        Value::from(quad.0),
        Value::from(quad.1),
        Value::from(quad.2),
    ];
    if let Some(graph_name) = quad.3 {
        row.push(Value::from(graph_name));
    }
    canonical(&Value::Array(row))
}

fn reifier_key(row: &ReifierRow) -> (Option<usize>, usize, usize, usize, usize) {
    let &(rid, (s, p, o), g) = row;
    (g, rid, s, p, o)
}

fn annotation_key(row: &AnnotationRow) -> (Option<usize>, usize, usize, usize) {
    let &(r, p, v, g) = row;
    (g, r, p, v)
}

fn remap_suppression(suppression: &Suppression, old_to_new: &[usize]) -> Suppression {
    let targets = suppression
        .targets
        .iter()
        .map(|target| remap_suppression_target(target, old_to_new))
        .collect();
    Suppression {
        targets,
        reason: suppression.reason.clone(),
        by: suppression.by.map(|tid| remap_id(old_to_new, tid)),
    }
}

fn remap_suppression_target(target: &Value, old_to_new: &[usize]) -> Value {
    let Value::Map(entries) = target else {
        return target.clone();
    };
    let kind = map_text(target, "kind").unwrap_or("");
    let mapped = entries
        .iter()
        .map(|(key, value)| {
            let key_text = match key {
                Value::Text(text) => text.as_str(),
                _ => "",
            };
            if (kind == "term" || kind == "reifier") && key_text == "id" {
                if let Some(tid) = crate::reader::as_idx(value) {
                    return (key.clone(), Value::from(remap_id(old_to_new, tid) as u64));
                }
            } else if kind == "quad"
                && key_text == "q"
                && let Value::Array(ids) = value
            {
                let remapped = ids
                    .iter()
                    .map(|id| {
                        crate::reader::as_idx(id).map_or_else(
                            || id.clone(),
                            |tid| Value::from(remap_id(old_to_new, tid) as u64),
                        )
                    })
                    .collect();
                return (key.clone(), Value::Array(remapped));
            }
            (key.clone(), value.clone())
        })
        .collect();
    Value::Map(mapped)
}

fn suppression_key(suppression: &Suppression) -> Vec<u8> {
    let mut payload: Vec<(Value, Value)> =
        vec![("targets".into(), Value::Array(suppression.targets.clone()))];
    if let Some(reason) = &suppression.reason {
        payload.push(("reason".into(), reason.clone().into()));
    }
    if let Some(by) = suppression.by {
        payload.push(("by".into(), Value::from(by as u64)));
    }
    canonical(&Value::Map(payload))
}

fn map_text<'a>(value: &'a Value, wanted: &str) -> Option<&'a str> {
    let Value::Map(entries) = value else {
        return None;
    };
    entries.iter().find_map(|(key, value)| match (key, value) {
        (Value::Text(key), Value::Text(text)) if key == wanted => Some(text.as_str()),
        _ => None,
    })
}

/// Pack bytes into a `blake3:<hex>` digest string.
///
/// # Examples
///
/// ```
/// use purrdf_gts::writer::digest_string;
///
/// let digest = digest_string(b"nine lives");
/// assert!(digest.starts_with("blake3:"));
/// assert_eq!(digest.len(), "blake3:".len() + 64);
/// // Deterministic: the same bytes always hash to the same digest.
/// assert_eq!(digest, digest_string(b"nine lives"));
/// ```
pub fn digest_string(data: &[u8]) -> String {
    digest_str(data)
}

#[cfg(test)]
mod term_walk_tests {
    //! The canonical-order walks — nesting depth and identity key — against their
    //! recursive references, on generated term tables with cycles and dangling ids, and
    //! over a chain far deeper than a 128 KiB thread could recurse.

    use purrdf_lex::cbor::Value;

    use super::{
        IdentityWork, Writer, canonical, compare_acyclic_identity, deterministic_term_remap,
        remap_with_identity_work, term_identity_key, term_nesting_depth, text_or_null,
    };
    use crate::model::{Graph, Term, TermKind};

    #[global_allocator]
    static ALLOCATOR: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

    /// The recursive reference of [`term_nesting_depth`].
    fn reference_depth(graph: &Graph, tid: usize, stack: &mut Vec<usize>) -> usize {
        if stack.contains(&tid) {
            return 0;
        }
        let Some(term) = graph.terms.get(tid) else {
            return 0;
        };
        if term.kind != TermKind::Triple {
            return 0;
        }
        let Some(components) = graph.triple_of(tid) else {
            return 1;
        };
        stack.push(tid);
        let deepest = <[usize; 3]>::from(components)
            .into_iter()
            .map(|component| reference_depth(graph, component, stack))
            .max()
            .unwrap_or(0);
        stack.pop();
        deepest + 1
    }

    /// The recursive reference of the identity value [`term_identity_key`] encodes.
    fn reference_identity(graph: &Graph, tid: usize, stack: &mut Vec<usize>) -> Value {
        if stack.contains(&tid) {
            return Value::Array(vec!["cycle".into(), Value::from(tid as u64)]);
        }
        let Some(term) = graph.terms.get(tid) else {
            return Value::Array(vec!["missing".into(), Value::from(tid as u64)]);
        };
        stack.push(tid);
        let value = match term.kind {
            TermKind::Iri => Value::Array(vec!["iri".into(), text_or_null(term.value.as_deref())]),
            TermKind::Literal => Value::Array(vec![
                "literal".into(),
                text_or_null(term.value.as_deref()),
                graph.datatype_iri_str(term).into(),
                text_or_null(term.lang.as_deref()),
                text_or_null(term.direction.as_deref()),
            ]),
            TermKind::Bnode => Value::Array(vec![
                "bnode".into(),
                match term.value.as_deref() {
                    Some(value) if !value.is_empty() => value.into(),
                    _ => Value::Array(vec!["anonymous".into(), Value::from(tid as u64)]),
                },
            ]),
            TermKind::Triple => match graph.term_triple(term) {
                Some((s, p, o)) => Value::Array(vec![
                    "triple".into(),
                    reference_identity(graph, s, stack),
                    reference_identity(graph, p, stack),
                    reference_identity(graph, o, stack),
                ]),
                None => Value::Array(vec![
                    "triple".into(),
                    Value::Null,
                    term.reifier
                        .map_or(Value::Null, |rid| Value::from(rid as u64)),
                ]),
            },
        };
        stack.pop();
        value
    }

    fn term(kind: TermKind, value: Option<&str>) -> Term {
        Term {
            kind,
            value: value.map(str::to_owned),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
            triple: None,
        }
    }

    /// A generated term table: every kind, triple terms naming any id — themselves,
    /// each other in cycles, or one past the end — and some unbound.
    fn generated(seed: u64) -> Graph {
        let mut rng = purrdf_testkit::rng::SplitMix64::new(seed);
        let mut draw = |n: u64| rng.below(n);
        let len = 1 + draw(9);
        let mut graph = Graph::default();
        for _ in 0..len {
            let mut term = match draw(6) {
                0 => term(TermKind::Iri, Some("http://example.org/i")),
                1 => term(TermKind::Bnode, if draw(2) == 0 { Some("b") } else { None }),
                2 => {
                    let mut literal = term(TermKind::Literal, Some("7"));
                    literal.lang = (draw(2) == 0).then(|| "en".to_owned());
                    literal
                }
                _ => term(TermKind::Triple, None),
            };
            if term.kind == TermKind::Triple && draw(5) != 0 {
                let id = |n: u64| usize::try_from(n).expect("a small id fits");
                term.triple = Some((id(draw(len + 1)), id(draw(len + 1)), id(draw(len + 1))));
            }
            graph.terms.push(term);
        }
        graph
    }

    fn acyclic(seed: u64, count: usize) -> Graph {
        let mut rng = purrdf_testkit::rng::SplitMix64::new(seed);
        let mut graph = Graph::default();
        for tid in 0..count {
            let mut entry = match rng.below(5) {
                0 => term(TermKind::Iri, Some("http://example.org/shared")),
                1 => term(TermKind::Bnode, None),
                2 => term(TermKind::Literal, Some("same")),
                _ => term(TermKind::Triple, None),
            };
            if entry.kind == TermKind::Triple && rng.below(7) != 0 {
                let child = |rng: &mut purrdf_testkit::rng::SplitMix64| {
                    if tid == 0 || rng.below(11) == 0 {
                        count + 3
                    } else {
                        usize::try_from(rng.below(tid as u64)).expect("bounded child")
                    }
                };
                let first = child(&mut rng);
                entry.triple = Some(if rng.below(4) == 0 {
                    (first, first, first)
                } else {
                    (first, child(&mut rng), child(&mut rng))
                });
            }
            graph.terms.push(entry);
        }
        graph
    }

    fn reference_remap(graph: &Graph) -> Vec<usize> {
        let mut stack = Vec::new();
        let keys: Vec<_> = (0..graph.terms.len())
            .map(|tid| {
                (
                    term_nesting_depth(graph, tid, &mut stack),
                    term_identity_key(graph, tid, &mut stack),
                )
            })
            .collect();
        let mut ids: Vec<_> = (0..graph.terms.len()).collect();
        ids.sort_unstable_by(|&a, &b| keys[a].cmp(&keys[b]).then_with(|| a.cmp(&b)));
        ids
    }

    #[test]
    fn shared_identity_order_matches_expanded_cbor_on_acyclic_dags_and_malformed_tables() {
        for seed in 0..500 {
            let graph = acyclic(seed, 3 + usize::try_from(seed % 53).expect("small count"));
            assert_eq!(
                deterministic_term_remap(&graph).old_by_new,
                reference_remap(&graph),
                "acyclic seed {seed}"
            );
            let malformed = generated(seed);
            assert_eq!(
                deterministic_term_remap(&malformed).old_by_new,
                reference_remap(&malformed),
                "malformed seed {seed}"
            );
        }
    }

    #[test]
    fn flat_identity_comparisons_allocate_nothing() {
        let mut graph = Graph::default();
        for index in 0..1024 {
            graph
                .terms
                .push(Term::iri(format!("http://example.org/{index:08}")));
        }
        let keys: Vec<_> = (0..graph.terms.len())
            .map(|tid| Some(term_identity_key(&graph, tid, &mut Vec::new())))
            .collect();
        let classes: Vec<_> = (0..graph.terms.len()).collect();
        let mut work = IdentityWork::default();
        let mut compared_pairs = crate::FastMap::default();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let mut ordered = 0;
        for tid in 0..graph.terms.len() - 1 {
            ordered += usize::from(
                compare_acyclic_identity(
                    &graph,
                    &keys,
                    &classes,
                    &mut compared_pairs,
                    tid,
                    tid + 1,
                    &mut work,
                ) == std::cmp::Ordering::Less,
            );
        }
        let measured = window.close();
        assert_eq!(ordered, 1023);
        assert_eq!(work.compared_nodes, 1023);
        assert_eq!(measured.allocations, 0, "{measured:?}");
        assert_eq!(measured.requested_bytes, 0, "{measured:?}");
    }

    fn chain(levels: usize) -> Graph {
        let mut graph = Graph::default();
        graph
            .terms
            .push(term(TermKind::Iri, Some("http://example.org/s")));
        graph
            .terms
            .push(term(TermKind::Iri, Some("http://example.org/p")));
        graph
            .terms
            .push(term(TermKind::Iri, Some("http://example.org/o")));
        let mut below = 2;
        for _ in 0..levels {
            let mut triple = term(TermKind::Triple, None);
            triple.triple = Some((0, 1, below));
            graph.terms.push(triple);
            below = graph.terms.len() - 1;
        }
        graph
    }

    fn disjoint_chains(levels: usize, equal_leaves: bool) -> Graph {
        let mut graph = Graph::default();
        for side in 0..2 {
            let leaf = graph.terms.len();
            graph.terms.push(term(
                TermKind::Iri,
                Some(if equal_leaves || side == 0 {
                    "http://example.org/leaf"
                } else {
                    "http://example.org/leaf-different"
                }),
            ));
            let predicate = graph.terms.len();
            graph
                .terms
                .push(term(TermKind::Iri, Some("http://example.org/p")));
            let mut below = leaf;
            for _ in 0..levels {
                let mut triple = term(TermKind::Triple, None);
                triple.triple = Some((leaf, predicate, below));
                graph.terms.push(triple);
                below = graph.terms.len() - 1;
            }
            assert_eq!(side * (levels + 2) + levels + 1, below);
        }
        graph
    }

    fn round_trip_snapshot(graph: &Graph) {
        let mut writer = Writer::new("purrdf-test");
        writer.add_frame("snapshot", Some(graph.snapshot_payload()), None, None, None);
        let folded = crate::reader::read(&writer.into_bytes(), true, None);
        assert!(folded.diagnostics.is_empty(), "{:?}", folded.diagnostics);
        assert_eq!(folded.terms.len(), graph.terms.len());
    }

    #[test]
    fn shared_identity_work_and_retained_bytes_scale_on_chains_and_wide_dags() {
        let mut previous_chain = None;
        let mut previous_old = None;
        for levels in [128, 256, 512, 1024] {
            let graph = chain(levels);
            let (remap, work) = remap_with_identity_work(&graph);
            round_trip_snapshot(&graph);
            assert_eq!(remap.old_by_new.len(), graph.terms.len());
            assert_eq!(work.source_terms, levels + 3);
            assert_eq!(work.source_edges, levels * 3);
            let new_work = work.source_terms
                + work.source_edges
                + work.interned_nodes
                + work.interned_edges
                + work.compared_nodes
                + work.path_membership_probes;
            let old_bytes: usize = (0..graph.terms.len())
                .map(|tid| term_identity_key(&graph, tid, &mut Vec::new()).len())
                .sum();
            eprintln!(
                "identity chain levels={levels} visits={new_work} source_terms={} source_edges={} compared_nodes={} path_probes={} flat_bytes={} peak_state_bytes={} old_expanded_bytes={old_bytes}",
                work.source_terms,
                work.source_edges,
                work.compared_nodes,
                work.path_membership_probes,
                work.flat_identity_bytes,
                work.peak_identity_bytes,
            );
            if let Some(previous) = previous_chain {
                assert!(
                    new_work < previous * 3,
                    "chain {levels}: {new_work} vs {previous}"
                );
            }
            if let Some(previous) = previous_old {
                assert!(
                    old_bytes > previous * 3,
                    "old expanded keys remain quadratic"
                );
            }
            assert!(work.flat_identity_bytes < graph.terms.len() * 128);
            assert!(work.peak_identity_bytes < graph.terms.len() * 128);
            previous_chain = Some(new_work);
            previous_old = Some(old_bytes);
        }

        let mut previous = None;
        let mut previous_peak = None;
        for width in [64, 128, 256, 512] {
            let mut graph = chain(32);
            let shared = graph.terms.len() - 1;
            for index in 0..width {
                let leaf = graph.terms.len();
                let mut name = term(TermKind::Iri, None);
                name.value = Some(format!("http://example.org/{index:08}"));
                graph.terms.push(name);
                let mut triple = term(TermKind::Triple, None);
                triple.triple = Some((leaf, 1, shared));
                graph.terms.push(triple);
            }
            let (remap, work) = remap_with_identity_work(&graph);
            round_trip_snapshot(&graph);
            assert_eq!(remap.old_by_new, reference_remap(&graph));
            let visits = work.source_terms
                + work.source_edges
                + work.interned_nodes
                + work.interned_edges
                + work.compared_nodes
                + work.path_membership_probes;
            eprintln!(
                "identity wide width={width} visits={visits} source_terms={} source_edges={} compared_nodes={} path_probes={} flat_bytes={} peak_state_bytes={}",
                work.source_terms,
                work.source_edges,
                work.compared_nodes,
                work.path_membership_probes,
                work.flat_identity_bytes,
                work.peak_identity_bytes,
            );
            if let Some(prior) = previous {
                assert!(visits < prior * 3, "width {width}: {visits} vs {prior}");
            }
            if let Some(prior) = previous_peak {
                assert!(
                    work.peak_identity_bytes < prior * 3,
                    "width {width}: {} vs {prior}",
                    work.peak_identity_bytes,
                );
            }
            assert!(work.flat_identity_bytes < graph.terms.len() * 128);
            previous = Some(visits);
            previous_peak = Some(work.peak_identity_bytes);
        }
    }

    #[test]
    fn disjoint_chains_keep_comparison_work_and_state_linear() {
        for equal_leaves in [true, false] {
            let mut previous_visits = None;
            let mut previous_state = None;
            for levels in [128, 256, 512, 1024] {
                let graph = disjoint_chains(levels, equal_leaves);
                let (remap, work) = remap_with_identity_work(&graph);
                round_trip_snapshot(&graph);
                assert_eq!(remap.old_by_new.len(), graph.terms.len());
                if levels <= 256 {
                    assert_eq!(remap.old_by_new, reference_remap(&graph));
                }
                let visits = work.source_terms
                    + work.source_edges
                    + work.interned_nodes
                    + work.interned_edges
                    + work.compared_nodes
                    + work.path_membership_probes;
                eprintln!(
                    "identity disjoint-chain equal_leaves={equal_leaves} levels={levels} visits={visits} compared_nodes={} peak_state_bytes={}",
                    work.compared_nodes, work.peak_identity_bytes,
                );
                if let Some(previous) = previous_visits {
                    assert!(
                        visits < previous * 3,
                        "levels {levels}: {visits} vs {previous}"
                    );
                }
                if let Some(previous) = previous_state {
                    assert!(
                        work.peak_identity_bytes < previous * 3,
                        "levels {levels}: {} vs {previous}",
                        work.peak_identity_bytes,
                    );
                }
                previous_visits = Some(visits);
                previous_state = Some(work.peak_identity_bytes);
            }
        }
    }

    #[test]
    fn shared_identity_snapshot_writer_reads_back_the_same_canonical_graph() {
        let mut graph = chain(96);
        graph.quads.push((graph.terms.len() - 1, 1, 2, None));
        let expected = canonical(&graph.snapshot_payload());
        let mut writer = Writer::new("purrdf-test");
        writer.add_frame("snapshot", Some(graph.snapshot_payload()), None, None, None);
        let folded = crate::reader::read(&writer.into_bytes(), true, None);
        assert!(folded.diagnostics.is_empty(), "{:?}", folded.diagnostics);
        assert_eq!(folded.terms.len(), graph.terms.len());
        assert_eq!(folded.quads.len(), 1);
        assert_eq!(canonical(&folded.snapshot_payload()), expected);
    }

    /// Both work-list walks answer every term of every generated table exactly as their
    /// recursive references do, and leave the shared path stack empty.
    #[test]
    fn the_walks_agree_with_their_recursive_references_on_generated_tables() {
        let (mut cycles, mut nested) = (0, 0);
        for seed in 0..500_u64 {
            let graph = generated(seed);
            for tid in 0..=graph.terms.len() {
                let mut stack = Vec::new();
                let depth = term_nesting_depth(&graph, tid, &mut stack);
                assert_eq!(stack, Vec::<usize>::new(), "seed {seed}, term {tid}");
                assert_eq!(
                    depth,
                    reference_depth(&graph, tid, &mut stack),
                    "seed {seed}, term {tid}"
                );
                nested += usize::from(depth > 1);
                let key = term_identity_key(&graph, tid, &mut stack);
                assert_eq!(stack, Vec::<usize>::new(), "seed {seed}, term {tid}");
                let identity = reference_identity(&graph, tid, &mut stack);
                cycles += usize::from(format!("{identity:?}").contains("cycle"));
                assert_eq!(key, canonical(&identity), "seed {seed}, term {tid}");
            }
        }
        assert!(
            nested > 0,
            "some generated table nests a triple term in one"
        );
        assert!(cycles > 0, "some generated table has a cycle");
    }

    /// A chain several thousand triple terms deep is measured and keyed on a thread
    /// whose whole stack is 128 KiB — a recursion that deep would need several times
    /// that. The legacy expanded-key reference is limited to a few thousand levels;
    /// production remapping reuses flat identities and checks path membership in O(1).
    #[test]
    fn a_deep_chain_is_measured_and_keyed_on_a_128_kib_thread() {
        const LEVELS: usize = 3_000;
        purrdf_stack::on_stack(128 * 1024, || {
            let mut graph = Graph::default();
            graph
                .terms
                .push(term(TermKind::Iri, Some("http://example.org/s")));
            graph
                .terms
                .push(term(TermKind::Iri, Some("http://example.org/p")));
            graph
                .terms
                .push(term(TermKind::Iri, Some("http://example.org/o")));
            let mut below = 2;
            for _ in 0..LEVELS {
                let mut triple = term(TermKind::Triple, None);
                triple.triple = Some((0, 1, below));
                graph.terms.push(triple);
                below = graph.terms.len() - 1;
            }
            let mut stack = Vec::new();
            assert_eq!(term_nesting_depth(&graph, below, &mut stack), LEVELS);
            let key = term_identity_key(&graph, below, &mut stack);
            assert_eq!(stack, Vec::<usize>::new());
            let iri = |value: &str| canonical(&Value::Array(vec!["iri".into(), value.into()]));
            let level = 1
                + canonical(&Value::Text("triple".into())).len()
                + iri("http://example.org/s").len()
                + iri("http://example.org/p").len();
            assert_eq!(
                key.len(),
                LEVELS * level + iri("http://example.org/o").len()
            );
        })
        .expect("the thread starts");
    }
}
