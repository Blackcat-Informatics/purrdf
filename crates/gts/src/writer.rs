// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A GTS writer: build frames, maintain the id/prev chain, emit a CBOR
//! Sequence.
//!
//! This is the encoder counterpart to [`crate::reader`]. It supports the core
//! graph/file frame families plus transformed, encrypted, and signed payloads.
//! Deterministic CBOR and BLAKE3 self-hashes are handled by [`crate::wire`].

use std::collections::HashMap;
use std::fmt;

use ciborium::value::Value;

use crate::codec::{encode_chain_with_options, Codec, CodecError, EncodeOptions};
use crate::model::{
    is_literal_direction, AnnotationRow, Graph, Quad, ReifierRow, Suppression, Term, TermKind,
};
use crate::wire::{
    append_canonical, canonical, content_id, digest_str, header_id, SELF_DESCRIBE_TAG,
};

/// Payloads larger than this select `zstd-rsyncable` over `zstd` in snapshot helpers.
pub const DEFAULT_RSYNCABLE_THRESHOLD: usize = 65_536;

fn iv(n: i64) -> Value {
    Value::Integer(ciborium::value::Integer::from(n))
}

fn uv(n: usize) -> Value {
    Value::Integer(ciborium::value::Integer::from(n as u64))
}

/// Serialise a [`Term`] to its wire map (dropping absent fields).
pub fn term_to_wire(t: &Term) -> Value {
    let mut entries: Vec<(Value, Value)> = Vec::with_capacity(6);
    entries.push(("k".into(), iv(t.kind as i64)));
    if let Some(v) = &t.value {
        entries.push(("v".into(), v.clone().into()));
    }
    if let Some(dt) = t.datatype {
        entries.push(("dt".into(), iv(dt as i64)));
    }
    if let Some(l) = &t.lang {
        entries.push(("l".into(), l.clone().into()));
    }
    if let Some(direction) = t.direction.as_deref().filter(|d| is_literal_direction(d)) {
        entries.push(("dir".into(), direction.to_string().into()));
    }
    if let Some(rf) = t.reifier {
        entries.push(("rf".into(), iv(rf as i64)));
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
}

impl Default for WriterOptions {
    fn default() -> Self {
        Self {
            catalog: None,
            meta: None,
            magic_tag: true,
            layout: None,
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
    pub zstd_level: Option<i32>,
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotSigner {
    /// 32-byte Ed25519 secret seed.
    pub secret: [u8; 32],
    /// COSE key id used in frame signatures and `gts:transportKey`.
    pub kid: String,
    /// ASCII-armored OpenPGP Ed25519 public-key certificate embedded as transport metadata.
    pub public_key_armor: String,
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
pub enum WriterError {
    /// Invalid caller options.
    InvalidFrame(String),
    /// The writer catalog cannot satisfy a requested codec.
    MissingCatalogEntry(String),
    /// Codec encode failure.
    Codec(CodecError),
}

impl fmt::Display for WriterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFrame(detail) => f.write_str(detail),
            Self::MissingCatalogEntry(name) => {
                write!(f, "writer catalog has no entry for codec '{name}'")
            }
            Self::Codec(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for WriterError {}

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

    if let Some(signer) = signer {
        writer.sign_with(
            ed25519_dalek::SigningKey::from_bytes(&signer.secret),
            &signer.kid,
        );
        writer.add_meta(Value::Map(vec![(
            "gts:transportKey".into(),
            Value::Map(vec![
                ("kid".into(), Value::Text(signer.kid)),
                ("gpg".into(), Value::Text(signer.public_key_armor)),
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

fn default_catalog() -> Vec<(i64, Codec)> {
    vec![
        (
            0,
            Codec {
                name: "identity".to_string(),
                cls: "encode".to_string(),
            },
        ),
        (
            1,
            Codec {
                name: "gzip".to_string(),
                cls: "compress".to_string(),
            },
        ),
        (
            2,
            Codec {
                name: "zstd".to_string(),
                cls: "compress".to_string(),
            },
        ),
        (
            3,
            Codec {
                name: "zstd-rsyncable".to_string(),
                cls: "compress".to_string(),
            },
        ),
        (
            7,
            Codec {
                name: "cose-encrypt0".to_string(),
                cls: "encrypt".to_string(),
            },
        ),
    ]
}

/// Accumulate a GTS log as a CBOR Sequence.
// `SigningKey`'s `Debug` impl redacts the secret scalar, so deriving is safe here.
#[derive(Debug)]
pub struct Writer {
    name_to_id: HashMap<String, i64>,
    prev: Vec<u8>,
    buf: Vec<u8>,
    // Per-frame byte offsets and types, in append order — the raw material
    // of an `index` footer (§6.2): offsets enable random access/parallel
    // verify, types the "ti" locator map.
    offsets: Vec<usize>,
    types: Vec<String>,
    frame_ids: Vec<Vec<u8>>,
    // When set, every appended frame is COSE_Sign1-signed over its id (§9.2).
    signer: Option<(ed25519_dalek::SigningKey, String)>,
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
        quads.sort_by_key(quad_key);
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
        reifiers.sort_by_key(reifier_key);
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
        annotations.sort_by_key(annotation_key);
        if !annotations.is_empty() {
            writer.add_annot(&annotations);
        }

        let mut blobs: Vec<(String, Vec<u8>)> = graph
            .blobs
            .iter()
            .map(|(digest, entry)| Ok((digest.clone(), entry.decoded_vec()?)))
            .collect::<Result<_, CodecError>>()?;
        blobs.sort_by(|a, b| a.0.cmp(&b.0));
        for (digest, data) in blobs {
            let meta = graph
                .blob_meta
                .iter()
                .find(|(candidate, _)| candidate == &digest)
                .map(|(_, meta)| meta);
            let mt = meta
                .and_then(|value| map_text(value, "mt"))
                .map(str::to_string);
            let rep = meta
                .and_then(|value| map_text(value, "rep"))
                .map(str::to_string);
            writer.add_blob_owned(data, mt.as_deref(), rep.as_deref());
        }

        if !graph.meta.is_empty() {
            let mut entries: Vec<(Value, Value)> = graph
                .meta
                .iter()
                .map(|(key, value)| (key.clone().into(), value.clone()))
                .collect();
            entries.sort_by_key(|(key, _)| canonical(key));
            writer.add_meta(Value::Map(entries));
        }

        let mut suppressions: Vec<Suppression> = graph
            .suppressions
            .iter()
            .map(|suppression| remap_suppression(suppression, &remap.old_to_new))
            .collect();
        suppressions.sort_by_key(suppression_key);
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
            return Err(WriterError::InvalidFrame(format!(
                "unsupported layout claim {:?} (§3.3)",
                options.layout
            )));
        }
        let catalog: HashMap<i64, Codec> = options
            .catalog
            .unwrap_or_else(default_catalog)
            .into_iter()
            .collect();
        let name_to_id: HashMap<String, i64> = catalog
            .iter()
            .map(|(id, c)| (c.name.clone(), *id))
            .collect();

        let cat_entries: Vec<(Value, Value)> = catalog
            .iter()
            .map(|(id, c)| {
                let mut ce: Vec<(Value, Value)> = vec![
                    ("name".into(), c.name.clone().into()),
                    ("cls".into(), c.cls.clone().into()),
                ];
                ce.sort_by_key(|a| canonical(&a.0));
                (iv(*id), Value::Map(ce))
            })
            .collect();

        let mut header: Vec<(Value, Value)> = vec![
            ("gts".into(), "GTS1".into()),
            ("v".into(), iv(1)),
            ("prof".into(), profile.into()),
            ("cat".into(), Value::Map(cat_entries)),
        ];
        if let Some(layout) = options.layout {
            // The layout-state claim is part of the header content, so it is
            // covered by the genesis self-hash (§3.3, §5).
            header.push(("layout".into(), layout.into()));
        }
        if let Some(meta) = options.meta {
            header.push(("meta".into(), meta));
        }
        header.sort_by_key(|a| canonical(&a.0));
        let id = header_id(&header);
        header.push(("id".into(), Value::Bytes(id.clone())));
        header.sort_by_key(|a| canonical(&a.0));

        let header_value = Value::Map(header);
        let buf = if options.magic_tag {
            canonical(&Value::Tag(SELF_DESCRIBE_TAG, Box::new(header_value)))
        } else {
            canonical(&header_value)
        };

        Ok(Self {
            name_to_id,
            prev: id,
            buf,
            offsets: Vec::new(),
            types: Vec::new(),
            frame_ids: Vec::new(),
            signer: None,
        })
    }

    /// Sign every subsequently appended frame's id with this Ed25519 key (§9.2).
    pub fn sign_with(&mut self, key: ed25519_dalek::SigningKey, kid: &str) {
        self.signer = Some((key, kid.to_string()));
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

    fn chain_ids(&self, chain: &[String]) -> Result<Vec<i64>, WriterError> {
        chain
            .iter()
            .map(|name| {
                self.name_to_id
                    .get(name)
                    .copied()
                    .ok_or_else(|| WriterError::MissingCatalogEntry(name.clone()))
            })
            .collect()
    }

    /// Append one frame and return its `"id"`.
    pub fn add_frame(
        &mut self,
        frame_type: &str,
        payload: Option<Value>,
        raw: Option<Vec<u8>>,
        transform: Option<&[String]>,
        pub_meta: Option<Value>,
    ) -> Vec<u8> {
        let mut options = FrameOptions {
            payload,
            raw,
            pub_meta,
            ..FrameOptions::default()
        };
        if let Some(transform) = transform {
            options.transform = transform.to_vec();
        }
        self.add_frame_with_options(frame_type, options)
            .expect("invalid frame options")
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
            pub_meta,
            mut recipients,
            signature,
            encrypt,
        } = options;

        if payload.is_some() && raw.is_some() {
            return Err(WriterError::InvalidFrame(
                "payload and raw are mutually exclusive".to_string(),
            ));
        }
        let transforms_data = !transform.is_empty() || encrypt.is_some();
        if transforms_data && payload.is_none() && raw.is_none() {
            return Err(WriterError::InvalidFrame(
                "transform/encrypt requires a payload or raw source".to_string(),
            ));
        }
        if zstd_level.is_some()
            && !transform
                .iter()
                .any(|name| matches!(name.as_str(), "zstd" | "zstd-rsyncable"))
        {
            return Err(WriterError::InvalidFrame(
                "zstd_level requires a zstd or zstd-rsyncable transform".to_string(),
            ));
        }
        let mut frame: Vec<(Value, Value)> = vec![("t".into(), frame_type.into())];

        let data: Option<Value> = if transforms_data {
            let mut source = match (raw, payload) {
                (Some(raw), _) => raw,
                (None, Some(payload)) => canonical(&payload),
                (None, None) => unreachable!("validated transform source above"),
            };
            let mut x_ids: Vec<i64> = self.chain_ids(&transform)?;
            if !transform.is_empty() {
                source =
                    encode_chain_with_options(&transform, &source, EncodeOptions { zstd_level })?;
            }
            if let Some(encrypt) = encrypt {
                let encrypt_id = self
                    .name_to_id
                    .get("cose-encrypt0")
                    .copied()
                    .ok_or_else(|| WriterError::MissingCatalogEntry("cose-encrypt0".into()))?;
                source = crate::cose::encrypt0(&source, &encrypt.kid, &encrypt.key, &encrypt.iv);
                x_ids.push(encrypt_id);
                recipients.push(Value::Map(vec![("kid".into(), encrypt.kid.into())]));
            }
            frame.push((
                "x".into(),
                Value::Array(x_ids.into_iter().map(iv).collect()),
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

        frame.sort_by_key(|a| canonical(&a.0));
        // The signature is not part of the frame-id preimage (§9.2). This lets
        // signatures be verified against the same id after streamable
        // compaction carries them as detached evidence.
        let id = content_id(&frame);
        frame.push(("id".into(), Value::Bytes(id.clone())));
        let sig = match signature {
            Some(sig) => Some(sig),
            None => self
                .signer
                .as_ref()
                .map(|(key, kid)| crate::cose::sign_id(&id, key, kid)),
        };
        if let Some(sig) = sig {
            frame.push(("sig".into(), Value::Bytes(sig)));
        }
        frame.sort_by_key(|a| canonical(&a.0));

        self.offsets.push(self.buf.len());
        self.types.push(frame_type.to_string());
        self.frame_ids.push(id.clone());
        append_canonical(&Value::Map(frame), &mut self.buf);
        self.prev.clone_from(&id);
        Ok(id)
    }

    /// Append a `terms` frame.
    pub fn add_terms(&mut self, terms: &[Term]) -> Vec<u8> {
        let payload = Value::Array(terms.iter().map(term_to_wire).collect());
        self.add_frame("terms", Some(payload), None, None, None)
    }

    /// Append a `quads` frame (graph slot dropped when `None`).
    pub fn add_quads(&mut self, quads: &[Quad]) -> Vec<u8> {
        let rows: Vec<Value> = quads
            .iter()
            .map(|&(s, p, o, g)| {
                let mut row = Vec::with_capacity(3 + usize::from(g.is_some()));
                row.push(iv(s as i64));
                row.push(iv(p as i64));
                row.push(iv(o as i64));
                if let Some(gv) = g {
                    row.push(iv(gv as i64));
                }
                Value::Array(row)
            })
            .collect();
        self.add_frame("quads", Some(Value::Array(rows)), None, None, None)
    }

    /// Append a `reifies` frame.
    pub fn add_reifies(&mut self, bindings: &[ReifierRow]) -> Vec<u8> {
        let rows: Vec<Value> = bindings
            .iter()
            .map(|&(rid, (s, p, o), g)| {
                let mut row = Vec::with_capacity(4 + usize::from(g.is_some()));
                row.push(iv(rid as i64));
                row.push(iv(s as i64));
                row.push(iv(p as i64));
                row.push(iv(o as i64));
                if let Some(gv) = g {
                    row.push(iv(gv as i64));
                }
                Value::Array(row)
            })
            .collect();
        self.add_frame("reifies", Some(Value::Array(rows)), None, None, None)
    }

    /// Append an `annot` frame.
    pub fn add_annot(&mut self, rows: &[AnnotationRow]) -> Vec<u8> {
        let rows: Vec<Value> = rows
            .iter()
            .map(|&(s, p, o, g)| {
                let mut row = Vec::with_capacity(3 + usize::from(g.is_some()));
                row.push(iv(s as i64));
                row.push(iv(p as i64));
                row.push(iv(o as i64));
                if let Some(gv) = g {
                    row.push(iv(gv as i64));
                }
                Value::Array(row)
            })
            .collect();
        self.add_frame("annot", Some(Value::Array(rows)), None, None, None)
    }

    /// Append an inline `blob` frame; metadata goes in `pub` (§12).
    pub fn add_blob(&mut self, data: &[u8], mt: Option<&str>, rep: Option<&str>) -> Vec<u8> {
        self.add_blob_owned(data.to_vec(), mt, rep)
    }

    /// Append an owned inline `blob` frame without cloning the payload first.
    pub fn add_blob_owned(
        &mut self,
        data: Vec<u8>,
        mt: Option<&str>,
        rep: Option<&str>,
    ) -> Vec<u8> {
        let mut pub_entries: Vec<(Value, Value)> =
            vec![("digest".into(), digest_str(&data).into())];
        if let Some(m) = mt {
            pub_entries.push(("mt".into(), m.into()));
        }
        if let Some(r) = rep {
            pub_entries.push(("rep".into(), r.into()));
        }
        let pub_meta = Some(Value::Map(pub_entries));
        self.add_frame("blob", None, Some(data), None, pub_meta)
    }

    /// Append a `meta` frame.
    pub fn add_meta(&mut self, meta: Value) -> Vec<u8> {
        self.add_frame("meta", Some(meta), None, None, None)
    }

    /// Append a `suppress` frame.
    pub fn add_suppress(
        &mut self,
        targets: Vec<Value>,
        reason: Option<&str>,
        by: Option<usize>,
    ) -> Vec<u8> {
        let mut payload: Vec<(Value, Value)> = vec![("targets".into(), Value::Array(targets))];
        if let Some(r) = reason {
            payload.push(("reason".into(), r.into()));
        }
        if let Some(b) = by {
            payload.push(("by".into(), Value::from(b as u64)));
        }
        payload.sort_by_key(|a| canonical(&a.0));
        self.add_frame("suppress", Some(Value::Map(payload)), None, None, None)
    }

    /// Append an `index` footer covering every frame appended so far (§6.2).
    ///
    /// `count`/`head` delimit the covered region (the streamable boundary,
    /// §3.3); `off` carries each covered frame's byte offset from the start
    /// of this writer's output; `ti` locates frames by type (0-based frame
    /// positions). A later `add_index` covers the earlier one too — the last
    /// index wins (§6.2).
    fn add_index_impl(&mut self, include_mmr: bool) -> Vec<u8> {
        let mut payload: Vec<(Value, Value)> = vec![
            ("count".into(), iv(self.types.len() as i64)),
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
            let off: Vec<Value> = self.offsets.iter().map(|&o| iv(o as i64)).collect();
            let mut ti: Vec<(Value, Value)> = Vec::new();
            for (pos, ftype) in self.types.iter().enumerate() {
                match ti
                    .iter_mut()
                    .find(|(k, _)| matches!(k, Value::Text(t) if t == ftype))
                {
                    Some((_, Value::Array(positions))) => positions.push(iv(pos as i64)),
                    _ => ti.push((ftype.clone().into(), Value::Array(vec![iv(pos as i64)]))),
                }
            }
            payload.push(("off".into(), Value::Array(off)));
            payload.push(("ti".into(), Value::Map(ti)));
        }
        self.add_frame("index", Some(Value::Map(payload)), None, None, None)
    }

    pub fn add_index(&mut self) -> Vec<u8> {
        self.add_index_impl(false)
    }

    /// Append an `index` footer with the optional `mmr` root over covered frame ids.
    ///
    /// This is opt-in so existing byte-oracle corpus vectors and cross-engine
    /// compact output remain stable until other engines claim the proof tier.
    pub fn add_index_with_mmr(&mut self) -> Vec<u8> {
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

/// Deterministic term-id remapping for canonical graph authorship.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TermRemap {
    /// New term id for each old term id.
    pub old_to_new: Vec<usize>,
    /// Old term ids in new-id order.
    pub old_by_new: Vec<usize>,
}

/// Return the deterministic term-id remapping used by canonical graph writers.
pub fn deterministic_term_remap(graph: &Graph) -> TermRemap {
    let mut old_by_new: Vec<usize> = (0..graph.terms.len()).collect();
    let keys: Vec<Vec<u8>> = old_by_new
        .iter()
        .map(|&tid| canonical(&term_identity_value(graph, tid, &mut Vec::new())))
        .collect();
    old_by_new.sort_by(|a, b| keys[*a].cmp(&keys[*b]).then_with(|| a.cmp(b)));
    let mut old_to_new = vec![0; graph.terms.len()];
    for (new, old) in old_by_new.iter().enumerate() {
        old_to_new[*old] = new;
    }
    TermRemap {
        old_to_new,
        old_by_new,
    }
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
    quads.sort_by_key(|quad| (quad.3, quad.0, quad.1, quad.2));

    let mut entries: Vec<(Value, Value)> = vec![
        ("terms".into(), Value::Array(terms)),
        (
            "quads".into(),
            Value::Array(
                quads
                    .iter()
                    .map(|&(s, p, o, g)| {
                        let mut row = vec![uv(s), uv(p), uv(o)];
                        if let Some(graph_name) = g {
                            row.push(uv(graph_name));
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
    reifiers.sort_by_key(reifier_key);
    if !reifiers.is_empty() {
        entries.push((
            "reifies".into(),
            Value::Array(
                reifiers
                    .iter()
                    .map(|&(rid, (s, p, o), g)| {
                        let mut row = vec![uv(rid), uv(s), uv(p), uv(o)];
                        if let Some(graph_name) = g {
                            row.push(uv(graph_name));
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
    annotations.sort_by_key(annotation_key);
    if !annotations.is_empty() {
        entries.push((
            "annot".into(),
            Value::Array(
                annotations
                    .iter()
                    .map(|&(r, p, v, g)| {
                        let mut row = vec![uv(r), uv(p), uv(v)];
                        if let Some(graph_name) = g {
                            row.push(uv(graph_name));
                        }
                        Value::Array(row)
                    })
                    .collect(),
            ),
        ));
    }

    Value::Map(entries)
}

fn term_identity_value(graph: &Graph, tid: usize, stack: &mut Vec<usize>) -> Value {
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
            graph.datatype_iri(term).into(),
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
        TermKind::Triple => match term.reifier.and_then(|rid| graph.reifier(rid)) {
            Some((s, p, o)) => Value::Array(vec![
                "triple".into(),
                term_identity_value(graph, s, stack),
                term_identity_value(graph, p, stack),
                term_identity_value(graph, o, stack),
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
    }
}

fn quad_key(quad: &Quad) -> Vec<u8> {
    let mut row = vec![iv(quad.0 as i64), iv(quad.1 as i64), iv(quad.2 as i64)];
    if let Some(graph_name) = quad.3 {
        row.push(iv(graph_name as i64));
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
                if let Some(tid) = value_idx(value) {
                    return (key.clone(), Value::from(remap_id(old_to_new, tid) as u64));
                }
            } else if kind == "quad" && key_text == "q" {
                if let Value::Array(ids) = value {
                    let remapped = ids
                        .iter()
                        .map(|id| {
                            value_idx(id).map_or_else(
                                || id.clone(),
                                |tid| Value::from(remap_id(old_to_new, tid) as u64),
                            )
                        })
                        .collect();
                    return (key.clone(), Value::Array(remapped));
                }
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

fn value_idx(value: &Value) -> Option<usize> {
    if let Value::Integer(i) = value {
        usize::try_from(i128::from(*i)).ok()
    } else {
        None
    }
}

/// Pack bytes into a `blake3:<hex>` digest string.
pub fn digest_string(data: &[u8]) -> String {
    digest_str(data)
}
