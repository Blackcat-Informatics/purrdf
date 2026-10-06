// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Streamable compaction (GTS-SPEC §10.1): re-author the ordering, only the
//! ordering — mirror of `packages/gts/src/gts/compact.py`.
//!
//! [`compact_streamable`] rewrites an accretive GTS file (or multi-segment
//! composition) into ONE delivery-ordered segment in the streamable layout
//! state (§3.3): a leading streaming index in the `stream` vocabulary
//! (§13.3), the content graph, blobs most-significant-first, and a trailing
//! offset `index` footer. Content signatures ride through untouched; frame
//! signatures are carried *detached* in compaction provenance; the ordering
//! commitment is re-issued — the compactor is the sole attester of the new
//! ordering.
//!
//! The rewrite is byte-deterministic for the same input, parameters and
//! explicit signing randomizers. Composite packaging uses fresh caller entropy;
//! Ed25519 packaging is deterministic. The content identities are unchanged.
//! Apart from those explicit signatures
//! (§14.1): blob order is ascending decoded size with digest tie-break, the
//! agent string is a constant, and the timestamp is a parameter — never
//! ambient time.

use std::borrow::Cow;

use purrdf_lex::cbor::Value;

use crate::dict;
use crate::mmr;
use crate::model::{Graph, Quad, ReifierRow, Suppression, Term, TermKind};
use crate::reader::{read, read_file_segments};
use crate::stream;
use crate::wire::{blake3_256, digest_label, digest_str, map_get};
use crate::writer::{self, FrameOptions, Writer, WriterOptions};

use purrdf_iri::vocab::rdf::TYPE as RDF_TYPE;
use purrdf_xsd::datatype::{XSD_DATE_TIME as XSD_DATETIME, XSD_INTEGER};

/// The conventional in-band pack dictionary name for a single-dictionary plan.
pub const DEFAULT_DICT_NAME: &str = "pack";
/// Target size of a DERIVED in-band pack dictionary. FastCOVER/raw-content
/// truncate the corpus to fit; a fixed value keeps compaction
/// byte-deterministic. [`DictStrategy::Pinned`] bytes are never truncated — they
/// are the caller's, verbatim.
const DICT_TARGET_LEN: usize = 16 * 1024;

/// Where one named in-band dictionary's bytes come from (§8.5 `dct`).
///
/// [`Self::Trained`] and [`Self::RawContent`] DERIVE the bytes from the pack's
/// own content-blob corpus. [`Self::Pinned`] carries caller-supplied bytes used
/// VERBATIM. All three end up pinned in the header `"dct"` map — the variant
/// names where the bytes come from, not whether they are stored in-band.
///
/// The derived strategies are byte-deterministic functions of the corpus: the
/// trained one is the production default (Req 3 headline is "trained"), the
/// raw-content one a named alternate. [`Self::Pinned`] exists because a derived
/// dictionary cannot honour an EXTERNAL dictionary id: a consumer that ships
/// named dictionaries and asks for a pack "under dictionary X" must get X's
/// actual bytes, not a pack-local re-derivation wearing X's name — otherwise one
/// id resolves to many byte sequences and stops identifying a decodable
/// dictionary at all.
///
/// The choice affects only the dictionary bytes, never the fold — a reader
/// decodes the in-band dictionary either way. "No dictionary" is NOT a strategy:
/// it is an empty [`DictPlan::dicts`], so a plan can never name a dictionary
/// that produces nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DictStrategy {
    /// FastCOVER-trained over the content-blob corpus (the production default).
    Trained,
    /// Raw-content over the content-blob corpus (its canonical trailing window).
    RawContent,
    /// Caller-supplied finalized zstd dictionary bytes, used verbatim: no
    /// training, no corpus derivation, no truncation. These exact bytes ride the
    /// header `"dct"` map under the plan's name, so the pinned name resolves to
    /// exactly the dictionary the caller shipped.
    Pinned(Vec<u8>),
}

impl DictStrategy {
    /// Whether this strategy DERIVES its bytes from the pack's content-blob
    /// corpus — and therefore requires the pack to have one.
    fn derives_from_corpus(&self) -> bool {
        match self {
            Self::Trained | Self::RawContent => true,
            Self::Pinned(_) => false,
        }
    }
}

/// Which in-band dictionary primes a frame — TOTAL, never `Option`.
///
/// `Option<String>` would let a fall-through mean "no dictionary" by accident;
/// naming the baseline makes the undicted case a decision someone wrote down.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DictSelection {
    /// Prime with the pinned dictionary of this name.
    Named(String),
    /// Deliberately no dictionary.
    Baseline,
}

impl DictSelection {
    /// The dictionary name, or `None` for [`Self::Baseline`].
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Named(name) => Some(name),
            Self::Baseline => None,
        }
    }
}

/// The named multi-dictionary + transform plan a compaction authors under.
///
/// Replaces the old single `(DICT_NAME, DictStrategy)` pair: §5 has always
/// allowed many named in-band dictionaries, and this is how a pack picks them,
/// selects one for its content frames, and states the transform profile every
/// authored frame rides (previously content blobs were hard-coded to plain
/// `zstd` and the streaming-index frames carried an untransformed payload).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DictPlan {
    /// Named dictionaries to pin in the header `"dct"` map, each with the
    /// strategy that supplies its bytes — derived from the content-blob corpus
    /// ([`DictStrategy::Trained`]/[`DictStrategy::RawContent`]) or supplied
    /// verbatim by the caller ([`DictStrategy::Pinned`]). The two kinds may be
    /// MIXED in one plan: each entry's bytes are obtained independently, and a
    /// corpus is required only if some entry actually derives from one. Empty
    /// pins no dictionary at all.
    pub dicts: Vec<(String, DictStrategy)>,
    /// Which pinned dictionary primes the content-blob frames.
    pub content: DictSelection,
    /// Which pinned dictionary primes the streaming-index / content-graph
    /// frames.
    pub index: DictSelection,
    /// The transform chain EVERY authored frame rides — the index frames and
    /// the content blobs alike. Empty stores payloads untransformed.
    pub transform: Vec<String>,
    /// The zstd level declared in the catalog and used by every zstd-family
    /// frame (§8.5 `level?`).
    pub zstd_level: Option<i32>,
}

impl DictPlan {
    /// No in-band dictionary and no transform: payloads stored as authored.
    pub fn undicted() -> Self {
        Self {
            dicts: Vec::new(),
            content: DictSelection::Baseline,
            index: DictSelection::Baseline,
            transform: Vec::new(),
            zstd_level: None,
        }
    }

    /// One dictionary named [`DEFAULT_DICT_NAME`], priming plain-`zstd` content
    /// frames — the historical single-dictionary shape.
    pub fn single(strategy: DictStrategy) -> Self {
        Self {
            dicts: vec![(DEFAULT_DICT_NAME.to_string(), strategy)],
            content: DictSelection::Named(DEFAULT_DICT_NAME.to_string()),
            index: DictSelection::Baseline,
            transform: vec!["zstd".to_string()],
            zstd_level: None,
        }
    }

    /// The GMEOW frame profile: one `zstd-rsyncable` transform at `level`,
    /// every frame primed by one named dictionary.
    pub fn rsyncable(name: &str, strategy: DictStrategy, level: i32) -> Self {
        Self {
            dicts: vec![(name.to_string(), strategy)],
            content: DictSelection::Named(name.to_string()),
            index: DictSelection::Named(name.to_string()),
            transform: vec!["zstd-rsyncable".to_string()],
            zstd_level: Some(level),
        }
    }

    /// Validate the plan's internal consistency (unique non-empty names, usable
    /// caller-supplied dictionary bytes, every selection naming a pinned
    /// dictionary, a dictionary only where a zstd-family transform can consume
    /// it).
    fn validate(&self) -> Result<(), CompactRefusedError> {
        let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for (name, strategy) in &self.dicts {
            if name.is_empty() {
                return refuse("an in-band dictionary name must be non-empty".to_string());
            }
            if !seen.insert(name.as_str()) {
                return refuse(format!("duplicate in-band dictionary name {name:?}"));
            }
            // Refuse-don't-trust on the ONE input the compactor does not
            // produce itself: caller-supplied bytes are used verbatim, so they
            // must actually be a finalized zstd dictionary a frame can be primed
            // with and a reader can resolve. A derived strategy cannot reach
            // here — its producer already finalizes.
            //
            // The parse IS the whole gate: `dictionary_id` decodes the finalized
            // dictionary, and that decode already rejects a zero `Dictionary_ID`
            // (§8.5 `dct` — a zero id cannot prime a zstd encoder). A separate
            // `id == 0` branch here would be unreachable, and an unreachable
            // guard advertises an invariant it does not enforce.
            if let DictStrategy::Pinned(bytes) = strategy {
                dict::dictionary_id(bytes).map_err(|err| {
                    CompactRefusedError(format!(
                        "the pinned dictionary {name:?} is not a parseable finalized zstd \
                         dictionary: {err}"
                    ))
                })?;
            }
        }
        let zstd_family = self
            .transform
            .iter()
            .any(|name| matches!(name.as_str(), "zstd" | "zstd-rsyncable"));
        for selection in [&self.content, &self.index] {
            let Some(name) = selection.name() else {
                continue;
            };
            if !seen.contains(name) {
                return refuse(format!(
                    "the plan selects dictionary {name:?}, which it does not pin"
                ));
            }
            if !zstd_family {
                return refuse(format!(
                    "the plan selects dictionary {name:?} but its transform chain \
                     {:?} carries no zstd-family codec to prime",
                    self.transform
                ));
            }
        }
        if !self.dicts.is_empty() && !zstd_family {
            return refuse(format!(
                "the plan pins {} dictionar(y/ies) but its transform chain {:?} carries no \
                 zstd-family codec, so nothing could ever use them",
                self.dicts.len(),
                self.transform
            ));
        }
        if self.zstd_level.is_some() && !zstd_family {
            return refuse(format!(
                "the plan declares a zstd level but its transform chain {:?} carries no \
                 zstd-family codec to encode at it",
                self.transform
            ));
        }
        Ok(())
    }
}

/// The input is not safely compactable (§10.1/§14.1 refuse-don't-trust).
#[derive(Debug)]
pub struct CompactRefusedError(pub String);

impl std::fmt::Display for CompactRefusedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for CompactRefusedError {}

fn refuse<T>(msg: String) -> Result<T, CompactRefusedError> {
    Err(CompactRefusedError(msg))
}

fn target_text<'a>(target: &'a Value, key: &str) -> Option<&'a str> {
    if let Value::Map(entries) = target
        && let Some(Value::Text(t)) = map_get(entries, key)
    {
        return Some(t);
    }
    None
}

/// Verify the input cleanly and return its union fold + single profile.
fn refusal_gate(data: &[u8], seal_original: bool) -> Result<(Graph, String), CompactRefusedError> {
    let fs = read_file_segments(data);
    if let Some(fatal) = &fs.fatal {
        return refuse(format!(
            "input is not a clean GTS file: {}: {}",
            fatal.code, fatal.detail
        ));
    }
    if let Some(torn) = fs.torn {
        return refuse(format!("input has a torn append at byte {torn}"));
    }
    for (idx, seg) in fs.segments.iter().enumerate() {
        if let Some(first) = seg.diagnostics.first() {
            return refuse(format!(
                "segment {idx} does not verify cleanly: {}: {}",
                first.code, first.detail
            ));
        }
    }
    let mut profiles: Vec<&str> = fs
        .segments
        .iter()
        .flat_map(|seg| seg.segment_profiles.iter().map(String::as_str))
        .collect();
    profiles.sort_unstable();
    profiles.dedup();
    if profiles.len() > 1 {
        // Python renders sorted(profiles) as a list repr — keep the text.
        let listed = profiles
            .iter()
            .map(|p| format!("'{p}'"))
            .collect::<Vec<_>>()
            .join(", ");
        return refuse(format!(
            "mixed segment profiles [{listed}] are not compactable (v1)"
        ));
    }
    let profile = profiles.first().copied().unwrap_or("generic").to_string();
    if profile == "evidence" && !seal_original {
        return refuse(
            "an 'evidence' artifact's signed chain IS the artifact; refusing \
             to re-order it without --seal-original (§10.1)"
                .to_string(),
        );
    }
    let g = read(data, true, None);
    for sup in &g.suppressions {
        for target in &sup.targets {
            if target_text(target, "kind") == Some("frame") {
                return refuse(
                    "input carries a frame-addressed suppression; the rewrite \
                     assigns new frame ids, so the target would silently \
                     dangle (§10.1)"
                        .to_string(),
                );
            }
        }
    }
    Ok((g, profile))
}

/// Accumulates the streaming-index terms and quads with stable ids.
#[derive(Default)]
struct GraphBuilder {
    terms: Vec<Term>,
    quads: Vec<Quad>,
}

impl GraphBuilder {
    fn add(&mut self, kind: TermKind, value: &str) -> usize {
        self.terms.push(Term {
            kind,
            value: Some(value.to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
            triple: None,
        });
        self.terms.len() - 1
    }

    fn literal(&mut self, value: &str, datatype: Option<usize>) -> usize {
        self.terms.push(Term {
            kind: TermKind::Literal,
            value: Some(value.to_string()),
            datatype,
            lang: None,
            direction: None,
            reifier: None,
            triple: None,
        });
        self.terms.len() - 1
    }

    fn quad(&mut self, s: usize, p: usize, o: usize) {
        self.quads.push((s, p, o, None));
    }
}

fn blob_decode_refused(digest: &str, err: impl std::fmt::Debug) -> CompactRefusedError {
    CompactRefusedError(format!("cannot decode blob {digest}: {err:?}"))
}

/// Look up a blob's decoded length in the insertion-ordered table.
fn blob_decoded_len(g: &Graph, digest: &str) -> Result<Option<usize>, CompactRefusedError> {
    purrdf_lex::assoc::get(&g.blobs, digest)
        .map(|entry| {
            entry
                .decoded_len()
                .map_err(|err| blob_decode_refused(digest, err))
        })
        .transpose()
}

/// Look up a blob's decoded bytes in the insertion-ordered table.
fn blob_bytes<'a>(
    g: &'a Graph,
    digest: &str,
) -> Result<Option<Cow<'a, [u8]>>, CompactRefusedError> {
    purrdf_lex::assoc::get(&g.blobs, digest)
        .map(|entry| {
            entry
                .decoded_bytes()
                .map_err(|err| blob_decode_refused(digest, err))
        })
        .transpose()
}

/// A declared text field (`mt`/`rep`) from a blob's `pub` metadata (§12).
fn blob_meta_text(g: &Graph, digest: &str, key: &str) -> Option<String> {
    purrdf_lex::assoc::get(&g.blob_meta, digest).and_then(|meta| {
        if let Value::Map(entries) = meta
            && let Some(Value::Text(t)) = map_get(entries, key)
        {
            return Some(t.clone());
        }
        None
    })
}

/// Base64url WITHOUT padding (RFC 4648 §5) — the `stream:cose` literal form.
pub fn base64url_unpadded(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[(n >> 6) as usize & 63] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[n as usize & 63] as char);
        }
    }
    out
}

/// Decode a base64url WITHOUT padding (RFC 4648 §5) string — the inverse of
/// the [`base64url_unpadded`] encoder, used to recover a `stream:cose`
/// literal's raw COSE_Sign1 bytes for signature verification.
///
/// # Errors
/// HARD-ERRORS on a padding character (`=`) or any byte outside the unpadded
/// alphabet (`A-Za-z0-9-_`) — a malformed literal must fail loudly rather than
/// silently decode to truncated or garbage bytes (refuse-don't-trust).
pub fn base64url_decode(s: &str) -> Result<Vec<u8>, String> {
    fn sextet(byte: u8) -> Result<u32, String> {
        match byte {
            b'A'..=b'Z' => Ok(u32::from(byte - b'A')),
            b'a'..=b'z' => Ok(u32::from(byte - b'a') + 26),
            b'0'..=b'9' => Ok(u32::from(byte - b'0') + 52),
            b'-' => Ok(62),
            b'_' => Ok(63),
            b'=' => Err("unexpected padding character '=' in unpadded base64url".to_string()),
            other => Err(format!(
                "byte {other:#04x} is outside the base64url (RFC 4648 §5) alphabet"
            )),
        }
    }
    let bytes = s.as_bytes();
    if bytes.len() % 4 == 1 {
        return Err(format!(
            "base64url input length {} leaves a single trailing character (invalid)",
            bytes.len()
        ));
    }
    let mut out = Vec::with_capacity((bytes.len() / 4 + 1) * 3);
    let (chunks, remainder) = bytes.as_chunks::<4>();
    for chunk in chunks {
        let n = (sextet(chunk[0])? << 18)
            | (sextet(chunk[1])? << 12)
            | (sextet(chunk[2])? << 6)
            | sextet(chunk[3])?;
        out.push((n >> 16) as u8);
        out.push((n >> 8) as u8);
        out.push(n as u8);
    }
    match remainder {
        [] => {}
        [a, b] => {
            let n = (sextet(*a)? << 18) | (sextet(*b)? << 12);
            out.push((n >> 16) as u8);
        }
        [a, b, c] => {
            let n = (sextet(*a)? << 18) | (sextet(*b)? << 12) | (sextet(*c)? << 6);
            out.push((n >> 16) as u8);
            out.push((n >> 8) as u8);
        }
        _ => unreachable!("as_chunks::<4> remainder is always shorter than 4"),
    }
    Ok(out)
}

/// The CLOSED predicate vocabulary `purrdf_gts::compact::streaming_index`
/// ever puts a `stream:Compaction` node (the `c` bnode) on the subject side
/// of — see `crates/gts/src/compact.rs`'s `streaming_index`, the sole minter
/// of this shape.
const COMPACTION_PREDICATES: &[&str] = &[
    RDF_TYPE,
    stream::AGENT,
    stream::TIMESTAMP,
    stream::SOURCE_HEAD,
    stream::SEALED_SOURCE,
    stream::CONTENT_REFOLD_DIGEST,
    stream::DETACHED_SIGNATURE_ROOT,
];

/// The CLOSED predicate vocabulary `streaming_index` ever puts a
/// `stream:Manifestation` node (an `m{order}` bnode) on the subject side of.
const MANIFESTATION_PREDICATES: &[&str] = &[
    RDF_TYPE,
    stream::DIGEST,
    stream::MEDIA_TYPE,
    stream::SIZE,
    stream::ROLE,
    stream::ORDER,
];

/// The CLOSED predicate vocabulary `streaming_index` ever puts a
/// `stream:DetachedSignature` node (an `s{n}` bnode) on the subject side of.
const DETACHED_SIGNATURE_PREDICATES: &[&str] = &[RDF_TYPE, stream::SOURCE_FRAME, stream::COSE];

/// Incremental closed-vocabulary provenance classification. This retains only
/// per-subject shape facts, so evented readers need not retain content quads.
/// Reserved class names with foreign predicates remain ordinary RDF content.
#[derive(Debug, Default)]
pub struct ProvenanceSubjects {
    // Reserved classes, permitted closed shapes, mandatory compaction fields.
    subjects: crate::FastMap<usize, (u8, u8, u8)>,
}

const PROVENANCE_SHAPES: &[(&str, &[&str])] = &[
    (stream::COMPACTION, COMPACTION_PREDICATES),
    (stream::MANIFESTATION, MANIFESTATION_PREDICATES),
    (stream::DETACHED_SIGNATURE, DETACHED_SIGNATURE_PREDICATES),
];

impl ProvenanceSubjects {
    /// Observe one validated quad in its owning segment's term table.
    pub fn observe(&mut self, g: &Graph, s: usize, p: usize, o: usize) {
        let predicate = g.terms.get(p).and_then(Term::iri_value);
        let class = g.terms.get(o).and_then(Term::iri_value);
        let state = self.subjects.entry(s).or_insert((0, 0b111, 0));
        for (index, &(name, allowed)) in PROVENANCE_SHAPES.iter().enumerate() {
            let bit = 1 << index;
            if predicate == Some(RDF_TYPE) && class == Some(name) {
                state.0 |= bit;
            }
            if !predicate.is_some_and(|value| allowed.contains(&value)) {
                state.1 &= !bit;
            }
        }
        if g.terms
            .get(s)
            .is_none_or(|term| term.kind != TermKind::Bnode)
        {
            state.1 &= !1;
        }
        let field = match predicate {
            Some(stream::AGENT) => 1,
            Some(stream::TIMESTAMP) => 2,
            Some(stream::SOURCE_HEAD) => 4,
            _ => return,
        };
        let Some(object) = g.terms.get(o) else {
            state.1 &= !1;
            return;
        };
        let string = object.kind == TermKind::Literal
            && object.value.is_some()
            && object.lang.is_none()
            && object.direction.is_none()
            && object
                .datatype
                .is_none_or(|id| iri_is(g, id, crate::model::XSD_STRING));
        let valid = match field {
            1 => string,
            2 => {
                object.kind == TermKind::Literal
                    && object.value.is_some()
                    && object.lang.is_none()
                    && object.direction.is_none()
                    && object
                        .datatype
                        .is_some_and(|id| iri_is(g, id, XSD_DATETIME))
            }
            4 => {
                string
                    && object
                        .value
                        .as_deref()
                        .and_then(|head| head.strip_prefix("blake3:"))
                        .is_some_and(|hex| mmr::parse_hex_32(hex).is_ok())
            }
            _ => unreachable!("only mandatory compaction fields reach this match"),
        };
        if !valid || (field != 4 && state.2 & field != 0) {
            state.1 &= !1;
        }
        state.2 |= field;
    }

    /// Classify a materialized graph by the same rule as an evented reader.
    #[must_use]
    pub fn from_graph(g: &Graph) -> Self {
        let mut subjects = Self::default();
        for &(s, p, o, _) in &g.quads {
            subjects.observe(g, s, p, o);
        }
        subjects
    }

    /// Whether the subject has one reserved class and its closed shape. A
    /// Compaction additionally needs the normative agent, typed timestamp and
    /// source-head fields; a bare class assertion stays ordinary content.
    #[must_use]
    pub fn contains(&self, subject: usize) -> bool {
        self.subjects
            .get(&subject)
            .is_some_and(|&(types, allowed, fields)| {
                types.is_power_of_two() && types & allowed != 0 && (types != 1 || fields == 7)
            })
    }

    fn class_subjects(&self, class: &str) -> impl Iterator<Item = usize> + '_ {
        let bit = PROVENANCE_SHAPES
            .iter()
            .position(|&(name, _)| name == class)
            .map_or(0, |index| 1 << index);
        self.subjects
            .iter()
            .filter_map(move |(&subject, &(types, _, _))| {
                (types == bit && self.contains(subject)).then_some(subject)
            })
    }

    pub(crate) fn has_compaction(&self) -> bool {
        self.class_subjects(stream::COMPACTION).next().is_some()
    }
}

/// Observe a packaging role in the frame's own segment, using the shared closed
/// provenance shape rather than a reserved class name in ordinary content.
pub(crate) fn packaging_role(
    provenance: &ProvenanceSubjects,
    frame_type: &str,
    streamable: bool,
) -> bool {
    streamable && frame_type == "index" && provenance.has_compaction()
}

fn iri_is(g: &Graph, id: usize, value: &str) -> bool {
    g.terms.get(id).and_then(Term::iri_value) == Some(value)
}

fn literal_fields<'a>(
    g: &'a Graph,
    node: usize,
    predicate: &str,
) -> Result<Vec<&'a str>, CompactRefusedError> {
    g.quads
        .iter()
        .filter(|&&(s, p, _, _)| s == node && iri_is(g, p, predicate))
        .map(|&(_, _, o, _)| {
            g.terms
                .get(o)
                .filter(|term| term.kind == TermKind::Literal)
                .and_then(|term| term.value.as_deref())
                .ok_or_else(|| {
                    CompactRefusedError(format!(
                        "provenance node {node} needs a literal {predicate}"
                    ))
                })
        })
        .collect()
}

/// Decode the detached root of the actual compaction of `source_heads`.
/// Historical compaction nodes may retain different roots. Every such root
/// must still be a single well-formed literal; the current node is selected by
/// its complete source-head list, never by a matching digest elsewhere.
///
/// # Errors
/// Refuses malformed roots or source heads, and absent or ambiguous current
/// compaction provenance.
pub fn compaction_signature_roots(
    g: &Graph,
    source_heads: &[Vec<u8>],
) -> Result<Vec<String>, CompactRefusedError> {
    let mut expected_heads = source_heads.to_vec();
    expected_heads.sort_unstable();
    let mut current = None;
    for record in compaction_root_records(g)? {
        if record.source_heads == expected_heads {
            if current.is_some() {
                return Err(CompactRefusedError(
                    "ambiguous compaction provenance for source heads".into(),
                ));
            }
            current = Some(record.roots);
        }
    }
    current
        .ok_or_else(|| CompactRefusedError("missing compaction provenance for source heads".into()))
}

struct CompactionRootRecord {
    source_heads: Vec<Vec<u8>>,
    roots: Vec<String>,
}

fn compaction_root_records(g: &Graph) -> Result<Vec<CompactionRootRecord>, CompactRefusedError> {
    let provenance = ProvenanceSubjects::from_graph(g);
    let mut records = Vec::new();
    for node in provenance.class_subjects(stream::COMPACTION) {
        let roots = literal_fields(g, node, stream::DETACHED_SIGNATURE_ROOT)?;
        if roots.len() > 1 {
            return Err(CompactRefusedError(format!(
                "compaction node {node} has repeated detached signature roots"
            )));
        }
        for root in &roots {
            mmr::parse_hex_32(root).map_err(|error| {
                CompactRefusedError(format!("compaction node {node} detached root: {error}"))
            })?;
        }
        let heads = literal_fields(g, node, stream::SOURCE_HEAD)?;
        if heads.is_empty() {
            return Err(CompactRefusedError(format!(
                "compaction node {node} needs source heads"
            )));
        }
        let mut parsed_heads = heads
            .into_iter()
            .map(|head| {
                let hex = head.strip_prefix("blake3:").ok_or_else(|| {
                    CompactRefusedError(format!("compaction node {node} invalid source head"))
                })?;
                mmr::parse_hex_32(hex).map_err(|error| {
                    CompactRefusedError(format!("compaction node {node} source head: {error}"))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        parsed_heads.sort_unstable();
        records.push(CompactionRootRecord {
            source_heads: parsed_heads,
            roots: roots.into_iter().map(str::to_owned).collect(),
        });
    }
    Ok(records)
}

/// Original frame content identity and its verbatim detached COSE envelope.
pub type DetachedSignaturePair = (Vec<u8>, Vec<u8>);

/// The literal object value(s) of every quad `(subject, predicate_iri, ?)` in
/// `g`, for every `subject` typed `stream:DetachedSignature`.
///
/// Parses the input graph's OWN carried `stream:DetachedSignature` provenance
/// nodes back into `(frame_id, cose)` byte pairs — the authorship signatures
/// accumulated by any PRIOR compaction(s), so a repack's detached root keeps
/// binding the ORIGINAL author frame sigs, not just whatever
/// `compact_streamable` observed fresh on this input. Missing, repeated,
/// nonliteral or malformed evidence refuses the entire set.
fn carried_detached_pairs(g: &Graph) -> Result<Vec<DetachedSignaturePair>, CompactRefusedError> {
    let mut nodes: Vec<usize> = g
        .quads
        .iter()
        .filter(|&&(_, p, o, _)| iri_is(g, p, RDF_TYPE) && iri_is(g, o, stream::DETACHED_SIGNATURE))
        .map(|&(s, _, _, _)| s)
        .collect();
    nodes.sort_unstable();
    nodes.dedup();
    let mut out = Vec::new();
    for node in nodes {
        let field = |predicate: &str| -> Result<&str, CompactRefusedError> {
            let values = literal_fields(g, node, predicate).map_err(|error| {
                CompactRefusedError(format!("detached signature node {node}: {error}"))
            })?;
            match values.as_slice() {
                [value] => Ok(*value),
                _ => Err(CompactRefusedError(format!(
                    "detached signature node {node} needs exactly one literal {predicate}"
                ))),
            }
        };
        let frame_id = mmr::parse_hex_32(field(stream::SOURCE_FRAME)?).map_err(|error| {
            CompactRefusedError(format!(
                "detached signature node {node} source frame: {error}"
            ))
        })?;
        let cose = base64url_decode(field(stream::COSE)?).map_err(|error| {
            CompactRefusedError(format!(
                "detached signature node {node} COSE encoding: {error}"
            ))
        })?;
        out.push((frame_id, cose));
    }
    Ok(out)
}

/// Sorted, deduplicated `(frame_id, cose)` pairs over every detached
/// AUTHORSHIP signature in `g` — the union of:
///  - fresh per-frame COSE folded onto `g.signatures`, excluding each reader's
///    segment-local packaging observation (new authored tails remain included), and
///  - the carried `stream:DetachedSignature` provenance already present in
///    `g` (accumulated by any prior compaction — see
///    the strict carried-node decoder).
///
/// A frame may carry multiple co-signatures under key rotation, so `frame_id`
/// alone is not a unique key — the `cose` tie-break is required for a stable,
/// deterministic leaf order. The union is deduplicated so an identical pair
/// surviving both sources (impossible today, but not an invariant this
/// function should assume) contributes exactly one leaf.
///
/// # Errors
/// Refuses malformed or ambiguous carried nodes and invalid COSE envelopes.
pub fn detached_signature_pairs(
    g: &Graph,
) -> Result<Vec<DetachedSignaturePair>, CompactRefusedError> {
    for signature in &g.signatures {
        let Some(cose) = &signature.cose else {
            return Err(CompactRefusedError(
                "frame signature must carry a COSE byte string".into(),
            ));
        };
        crate::cose::parse_sign1(cose)
            .map_err(|error| CompactRefusedError(format!("frame signature COSE: {error}")))?;
    }
    let mut pairs: Vec<(Vec<u8>, Vec<u8>)> = g
        .signatures
        .iter()
        .filter(|signature| !signature.packaging)
        .filter_map(|s| {
            s.cose
                .as_deref()
                .map(|cose| (s.frame_id.clone(), cose.to_vec()))
        })
        .collect();
    pairs.extend(carried_detached_pairs(g)?);
    for (frame_id, cose) in &pairs {
        if frame_id.len() != 32 {
            return Err(CompactRefusedError(
                "detached signature source frame must be 32 bytes".into(),
            ));
        }
        crate::cose::parse_sign1(cose)
            .map_err(|error| CompactRefusedError(format!("detached signature COSE: {error}")))?;
    }
    pairs.sort_unstable();
    pairs.dedup();
    Ok(pairs)
}

/// The MMR leaves committed by `stream:detachedSignatureRoot`: one
/// `blake3(frame_id || cose)` hash per detached signature, in sorted
/// `(frame_id, cose)` order (§10.1 signature preservation).
///
/// Public so a certificate consumer (GTS-SPEC §10.2) can independently derive
/// the same leaf set and prove membership without re-deriving the sort.
///
/// # Errors
/// Refuses malformed detached evidence through [`detached_signature_pairs`].
pub fn detached_signature_leaves(g: &Graph) -> Result<Vec<Vec<u8>>, CompactRefusedError> {
    Ok(detached_signature_pairs(g)?
        .into_iter()
        .map(|(frame_id, cose)| detached_signature_leaf(&frame_id, &cose))
        .collect())
}

/// `blake3(frame_id || cose)` — the leaf preimage for one detached signature.
fn detached_signature_leaf(frame_id: &[u8], cose: &[u8]) -> Vec<u8> {
    let mut preimage = Vec::with_capacity(frame_id.len() + cose.len());
    preimage.extend_from_slice(frame_id);
    preimage.extend_from_slice(cose);
    blake3_256(&preimage).to_vec()
}

/// A selective per-frame authorship proof: the detached inclusion proof for
/// one `(frame_id, cose)` leaf under [`detached_signature_leaves`]'s root.
///
/// Returns `None` when no detached signature matches `(frame_id, cose)`.
///
/// # Errors
/// Refuses malformed detached evidence through [`detached_signature_pairs`].
pub fn detached_signature_proof(
    g: &Graph,
    frame_id: &[u8],
    cose: &[u8],
) -> Result<Option<mmr::Proof>, CompactRefusedError> {
    let pairs = detached_signature_pairs(g)?;
    let Some(leaf_index) = pairs
        .iter()
        .position(|(f, c)| f.as_slice() == frame_id && c.as_slice() == cose)
    else {
        return Ok(None);
    };
    let leaves: Vec<Vec<u8>> = pairs
        .into_iter()
        .map(|(f, c)| detached_signature_leaf(&f, &c))
        .collect();
    Ok(mmr::prove(&leaves, leaf_index))
}

/// Build the leading streaming index + compaction provenance (§3.3, §13.3).
fn streaming_index(
    g: &Graph,
    blob_order: &[String],
    timestamp: &str,
    sealed_digest: Option<&str>,
    sealed_size: Option<usize>,
    content_digest: Option<&str>,
) -> Result<GraphBuilder, CompactRefusedError> {
    // The detached-authorship union (§10.1): fresh frame COSE from
    // `g.signatures` (when `g` is a raw tail — never a repack's own
    // packaging observation) UNIONED with the carried `stream:DetachedSignature`
    // provenance already present in `g` (accumulated by any prior
    // compaction). Computed once, up front, so the boolean drives the fixed
    // vocabulary block's id assignment identically to the pairs used below.
    let detached_pairs = detached_signature_pairs(g)?;

    let mut b = GraphBuilder::default();
    // Fixed vocabulary block — constant ids across engines for determinism.
    let t_type = b.add(TermKind::Iri, RDF_TYPE);
    let t_int = b.add(TermKind::Iri, XSD_INTEGER);
    let t_dt = b.add(TermKind::Iri, XSD_DATETIME);
    let t_manifestation = b.add(TermKind::Iri, stream::MANIFESTATION);
    let t_digest = b.add(TermKind::Iri, stream::DIGEST);
    let t_mt = b.add(TermKind::Iri, stream::MEDIA_TYPE);
    let t_size = b.add(TermKind::Iri, stream::SIZE);
    let t_role = b.add(TermKind::Iri, stream::ROLE);
    let t_order = b.add(TermKind::Iri, stream::ORDER);
    let t_compaction = b.add(TermKind::Iri, stream::COMPACTION);
    let t_agent = b.add(TermKind::Iri, stream::AGENT);
    let t_timestamp = b.add(TermKind::Iri, stream::TIMESTAMP);
    let t_source_head = b.add(TermKind::Iri, stream::SOURCE_HEAD);
    let t_sealed_source = b.add(TermKind::Iri, stream::SEALED_SOURCE);
    let t_detached_sig = b.add(TermKind::Iri, stream::DETACHED_SIGNATURE);
    let t_source_frame = b.add(TermKind::Iri, stream::SOURCE_FRAME);
    let t_cose = b.add(TermKind::Iri, stream::COSE);
    // The content-refold digest term rides the fixed block only when embedded,
    // mirroring how the sealed-source quad is emitted conditionally.
    let t_content_digest =
        content_digest.map(|_| b.add(TermKind::Iri, stream::CONTENT_REFOLD_DIGEST));
    // The detached-signature root term rides the fixed block only when the
    // union carries at least one detached signature (empty set ⇒ no quad,
    // keeping `signatures_bound` vacuously true for unsigned tails).
    let t_detached_root =
        (!detached_pairs.is_empty()).then(|| b.add(TermKind::Iri, stream::DETACHED_SIGNATURE_ROOT));

    // One Manifestation per promised blob, in delivery order.
    for (order, digest) in blob_order.iter().enumerate() {
        let m = b.add(TermKind::Bnode, &format!("m{order}"));
        let sealed = Some(digest.as_str()) == sealed_digest;
        let size = if sealed {
            sealed_size
        } else {
            blob_decoded_len(g, digest)?
        };
        let mt = if sealed {
            Some("application/vnd.blackcat.gts+cbor-seq".to_string())
        } else {
            blob_meta_text(g, digest, "mt")
        };
        b.quad(m, t_type, t_manifestation);
        let o = b.literal(digest, None);
        b.quad(m, t_digest, o);
        if let Some(mt) = mt {
            let o = b.literal(&mt, None);
            b.quad(m, t_mt, o);
        }
        if let Some(size) = size {
            let o = b.literal(&size.to_string(), Some(t_int));
            b.quad(m, t_size, o);
        }
        let o = b.literal(if sealed { "source" } else { "primary" }, None);
        b.quad(m, t_role, o);
        let o = b.literal(&order.to_string(), Some(t_int));
        b.quad(m, t_order, o);
    }

    // The Compaction provenance node (§10.1).
    let c = b.add(TermKind::Bnode, "c");
    b.quad(c, t_type, t_compaction);
    let o = b.literal(stream::COMPACT_AGENT, None);
    b.quad(c, t_agent, o);
    let o = b.literal(timestamp, Some(t_dt));
    b.quad(c, t_timestamp, o);
    for head in &g.segment_heads {
        let o = b.literal(&digest_label(head), None);
        b.quad(c, t_source_head, o);
    }
    if let Some(sealed) = sealed_digest {
        let o = b.literal(sealed, None);
        b.quad(c, t_sealed_source, o);
    }
    // Proof-carrying pack: embed the RDFC-1.0 content digest so a repack
    // certifies without the pre-compaction bytes. Excluded from the content
    // projection at verification time, so it cannot perturb the equivalence.
    if let (Some(t_cd), Some(digest)) = (t_content_digest, content_digest) {
        let o = b.literal(digest, None);
        b.quad(c, t_cd, o);
    }

    // Detached frame signatures (§10.1): checkable claims about the original
    // log — the FULL authorship union, so a re-emitted pack carries forward
    // every original author signature across any number of repacks, not just
    // the ones freshly observed on this input.
    for (j, (frame_id, cose)) in detached_pairs.iter().enumerate() {
        let node = b.add(TermKind::Bnode, &format!("s{j}"));
        let cose_b64 = base64url_unpadded(cose);
        b.quad(node, t_type, t_detached_sig);
        let o = b.literal(&digest_label(frame_id), None);
        b.quad(node, t_source_frame, o);
        let o = b.literal(&cose_b64, None);
        b.quad(node, t_cose, o);
    }

    // One MMR root binding the whole detached-signature set under a single
    // commitment (§10.1 signature preservation) — omitted entirely when the
    // set is empty (no zero-count root emitted for an unsigned tail).
    if let Some(t_root) = t_detached_root {
        let leaves: Vec<Vec<u8>> = detached_pairs
            .iter()
            .map(|(frame_id, cose)| detached_signature_leaf(frame_id, cose))
            .collect();
        let root = mmr::root(&leaves);
        let o = b.literal(&digest_label(&root), None);
        b.quad(c, t_root, o);
    }
    Ok(b)
}

/// Shift a term's id references into the output id space.
///
/// Delegates the column list to [`Term::map_term_ids`] rather than restating it:
/// this enumeration was correct, but it was the SECOND copy in this crate, and
/// the other one had already gone stale (it dropped `triple`). One enumeration,
/// owned by the type that adds the columns.
fn shift_term(t: &Term, base: usize) -> Term {
    t.map_term_ids(|id| id + base)
}

/// Carry suppressions forward, one output suppression per input (§10.1).
///
/// Re-authoring of the ordering only: each original suppression keeps its own
/// frame with its `reason`/`by` metadata intact — blob targets verbatim
/// (content-addressing is layout-independent), id-addressed targets and `by`
/// shifted into the output id space.
fn shifted_suppressions(g: &Graph, base: usize) -> Vec<Suppression> {
    let mut out: Vec<Suppression> = Vec::new();
    for sup in &g.suppressions {
        let mut targets: Vec<Value> = Vec::new();
        for target in &sup.targets {
            let Value::Map(entries) = target else {
                targets.push(target.clone());
                continue;
            };
            let kind = target_text(target, "kind").unwrap_or("");
            let shifted: Vec<(Value, Value)> = entries
                .iter()
                .map(|(k, v)| {
                    let key = if let Value::Text(t) = k {
                        t.as_str()
                    } else {
                        ""
                    };
                    if (kind == "term" || kind == "reifier") && key == "id" {
                        if let Some(tid) = crate::reader::as_idx(v) {
                            return (k.clone(), Value::from((tid + base) as u64));
                        }
                    } else if kind == "quad"
                        && key == "q"
                        && let Value::Array(ids) = v
                    {
                        let remapped: Vec<Value> = ids
                            .iter()
                            .map(|x| match crate::reader::as_idx(x) {
                                Some(tid) => Value::from((tid + base) as u64),
                                None => x.clone(),
                            })
                            .collect();
                        return (k.clone(), Value::Array(remapped));
                    }
                    (k.clone(), v.clone())
                })
                .collect();
            targets.push(Value::Map(shifted));
        }
        out.push(Suppression {
            targets,
            reason: sup.reason.clone(),
            by: sup.by.map(|b| b + base),
        });
    }
    out
}

/// Obtain every in-band pack dictionary the plan names: derived entries over the
/// batched content-blob corpus, [`DictStrategy::Pinned`] entries verbatim.
///
/// The corpus is every content blob's decoded bytes (the sealed original — the
/// whole source log — is excluded, it is not "content"). A pack with no content
/// blobs has no corpus to DERIVE from; a plan whose entries derive is therefore
/// refused rather than silently downgraded to an undicted pack, because the
/// caller asked for a density guarantee the pack could not then honour.
///
/// That refusal is a consequence of derivation, not of pinning a dictionary at
/// all: caller-supplied bytes exist independently of the pack's content, so a
/// wholly-[`DictStrategy::Pinned`] plan never touches the corpus and compacts a
/// blob-less input (a terms/quads-only log) exactly as it compacts any other. A
/// MIXED plan is accepted and each entry is obtained on its own terms — but its
/// derived entries still need a corpus, so a mixed plan over a blob-less input
/// hits the same refusal.
///
/// The derived producers are order-independent, so the emitted `dct` bytes equal
/// `trained_dict`/`raw_content_dict` of the corpus regardless of iteration order.
fn build_pack_dicts(
    g: &Graph,
    blob_order: &[String],
    sealed_digest: Option<&str>,
    plan: &DictPlan,
) -> Result<Vec<(String, Vec<u8>)>, CompactRefusedError> {
    if plan.dicts.is_empty() {
        return Ok(Vec::new());
    }
    // Only DERIVED entries need the corpus; decoding every content blob for a
    // wholly-pinned plan would be pure waste, and demanding one would be the
    // very refusal this function must no longer raise.
    let derives = plan
        .dicts
        .iter()
        .any(|(_, strategy)| strategy.derives_from_corpus());
    let mut corpus: Vec<Vec<u8>> = Vec::new();
    if derives {
        for digest in blob_order {
            if Some(digest.as_str()) == sealed_digest {
                continue;
            }
            if let Some(bytes) = blob_bytes(g, digest)? {
                corpus.push(bytes.into_owned());
            }
        }
        if corpus.is_empty() {
            return refuse(
                "the plan pins in-band dictionaries but the input has no content blobs to \
                 build them from (§8.5 dct)"
                    .to_string(),
            );
        }
    }
    let refs: Vec<&[u8]> = corpus.iter().map(Vec::as_slice).collect();
    let mut out = Vec::with_capacity(plan.dicts.len());
    for (name, strategy) in &plan.dicts {
        let bytes = match strategy {
            // The seed is derived from the corpus, so two names sharing a
            // strategy pin identical bytes — deliberate and deterministic.
            DictStrategy::Trained => {
                dict::trained_dict(&refs, DICT_TARGET_LEN, dict::DictSeed::FromCorpus)
            }
            DictStrategy::RawContent => dict::raw_content_dict(&refs, DICT_TARGET_LEN),
            // Verbatim: the caller's bytes ARE the dictionary. Validated as a
            // parseable finalized dictionary by `DictPlan::validate`.
            DictStrategy::Pinned(bytes) => Ok(bytes.clone()),
        }
        .map_err(|err| {
            CompactRefusedError(format!("cannot build the pack dictionary {name:?}: {err}"))
        })?;
        out.push((name.clone(), bytes));
    }
    Ok(out)
}

mod packaging_sealed {
    pub trait Sealed {}
}

/// Mandatory packaging signer. Only supported signed implementations can
/// implement this sealed contract; an unsigned implementation is unrepresentable.
pub trait PackagingSigner: packaging_sealed::Sealed {
    /// Explicit textual packaging id recorded in compaction certificates.
    fn kid(&self) -> &str;
    /// Sign the final ordering index through the actual Writer.
    ///
    /// # Errors
    /// Propagates caller randomness or native signing failure without a pack.
    fn finish(self, writer: Writer) -> Result<Vec<u8>, writer::WriterError>;
}

impl packaging_sealed::Sealed for (purrdf_ed25519::SigningKey, String) {}
impl PackagingSigner for (purrdf_ed25519::SigningKey, String) {
    fn kid(&self) -> &str {
        &self.1
    }
    fn finish(self, mut writer: Writer) -> Result<Vec<u8>, writer::WriterError> {
        writer.sign_with(self.0, &self.1);
        writer.add_index();
        Ok(writer.into_bytes())
    }
}

/// Composite packaging requires a dedicated key and a caller-owned fresh
/// cryptographic randomizer provider. Packaging ids are deliberately textual;
/// carried authorship ids remain opaque byte strings.
pub struct CompositePackaging<P> {
    key: crate::cose::composite::SigningKey,
    kid: String,
    provider: P,
}

impl<P: writer::RandomnessProvider> CompositePackaging<P> {
    /// Install all required composite packaging inputs. Provider errors refuse
    /// compaction; there is no deterministic or ambient randomness fallback.
    ///
    /// ```no_run
    /// use purrdf_gts::compact::{CompositePackaging, CompactionParams, DictPlan, compact_streamable};
    /// use purrdf_gts::cose::composite;
    /// use purrdf_gts::writer::RandomnessProvider;
    /// fn repack<P: RandomnessProvider>(source: &[u8], key: composite::SigningKey,
    ///     provider: P) -> Result<Vec<u8>, purrdf_gts::compact::CompactRefusedError> {
    ///     compact_streamable(source, CompactionParams {
    ///         timestamp: "2026-01-01T00:00:00Z", seal_original: false,
    ///         plan: DictPlan::undicted(), content_digest: None,
    ///         packaging_signer: CompositePackaging::new(key, "pack-key".into(), provider),
    ///     })
    /// }
    /// ```
    ///
    /// ```compile_fail
    /// use purrdf_gts::compact::CompositePackaging;
    /// # let key = purrdf_gts::cose::composite::SigningKey::from_bytes(&[0;64]).unwrap();
    /// let signer = CompositePackaging::new(key, "pack".into());
    /// ```
    pub fn new(key: crate::cose::composite::SigningKey, kid: String, provider: P) -> Self {
        Self { key, kid, provider }
    }
}

purrdf_hash::debug_non_exhaustive!([P] CompositePackaging<P> { kid });
impl<P: writer::RandomnessProvider> packaging_sealed::Sealed for CompositePackaging<P> {}
impl<P: writer::RandomnessProvider> PackagingSigner for CompositePackaging<P> {
    fn kid(&self) -> &str {
        &self.kid
    }
    fn finish(self, writer: Writer) -> Result<Vec<u8>, writer::WriterError> {
        let mut writer = writer.with_composite_signer(self.key, self.kid.as_bytes(), self.provider);
        writer.add_index()?;
        Ok(writer.into_bytes())
    }
}

/// Parameters for [`compact_streamable`].
// `SigningKey`'s `Debug` impl redacts the secret scalar, so deriving is safe here.
#[derive(Debug)]
pub struct CompactionParams<'a, S = (purrdf_ed25519::SigningKey, String)> {
    /// The rewrite time recorded as `stream:timestamp` — an explicit
    /// parameter so the output is byte-reproducible.
    pub timestamp: &'a str,
    /// Carry the verbatim source bytes as a nested GTS blob (§12.1), role
    /// `"source"` — REQUIRED for `evidence` input.
    pub seal_original: bool,
    /// The named multi-dictionary + transform plan: which dictionaries the
    /// pack pins (derived from the pack's own corpus, or supplied verbatim by
    /// the caller — see [`DictStrategy`]), which one primes which frames, the
    /// transform chain every authored frame rides, and the declared zstd level.
    pub plan: DictPlan,
    /// When supplied by the certifying authoring wrapper, embedded as
    /// `stream:contentRefoldDigest` provenance so a repack certifies without
    /// the pre-compaction bytes.
    pub content_digest: Option<&'a str>,
    /// `(key, kid)` used to sign ONLY the re-issued ordering commitment (the
    /// trailing `index` footer) — the MANDATORY packaging signature attesting
    /// ordering/packaging, never frame authorship. Carried detached
    /// authorship signatures live in provenance quads instead (§10.1); this
    /// is a distinct, separately-verifiable claim.
    ///
    /// REQUIRED, not optional: the format makes the packaging head signature
    /// mandatory, so a pack with no packaging signature must be
    /// unrepresentable through this API rather than merely discouraged — the
    /// field implements sealed [`PackagingSigner`], so an unsigned pack
    /// cannot be constructed by a caller that forgets to supply a signer.
    pub packaging_signer: S,
}

/// Rewrite a GTS file into one streamable segment (§10.1).
///
/// `data` must verify cleanly (refuse-don't-trust). See [`CompactionParams`]
/// for the rewrite parameters.
///
/// # Errors
/// Returns [`CompactRefusedError`] when the input is not safely compactable, a
/// blob cannot be decoded, a pinned dictionary's caller-supplied bytes are not a
/// usable finalized zstd dictionary, a DERIVED pack dictionary cannot be built
/// (including when the input carries no content-blob corpus to derive one from),
/// or the writer rejects the configuration.
pub fn compact_streamable<S: PackagingSigner>(
    data: &[u8],
    params: CompactionParams<'_, S>,
) -> Result<Vec<u8>, CompactRefusedError> {
    let CompactionParams {
        timestamp,
        seal_original,
        plan,
        content_digest,
        packaging_signer,
    } = params;
    plan.validate()?;
    let (mut g, profile) = refusal_gate(data, seal_original)?;
    // Malformed historical commitments must not become a successfully
    // re-authored artifact. This shares the certifier's strict root decoder,
    // while carried signature verification retains its separate crypto claim.
    compaction_root_records(&g)?;

    // Delivery plan: most-significant-first — ascending decoded size, digest
    // tie-break; the sealed original (least significant) always travels last.
    // Decode once here so the streaming index and re-emission reuse cached
    // bytes instead of repeating lazy decode work.
    let mut keyed: Vec<(usize, String)> = g
        .blobs
        .iter_mut()
        .map(|(d, entry)| {
            entry
                .decode()
                .map(|bytes| (bytes.len(), d.clone()))
                .map_err(|err| blob_decode_refused(d, err))
        })
        .collect::<Result<_, _>>()?;
    keyed.sort_unstable();
    let mut blob_order: Vec<String> = keyed.into_iter().map(|(_, d)| d).collect();
    let sealed_digest: Option<String> = if seal_original {
        let sealed = digest_str(data);
        blob_order.retain(|d| *d != sealed);
        blob_order.push(sealed.clone());
        Some(sealed)
    } else {
        None
    };

    // Pack dictionaries: derived entries built over the batched content-blob
    // corpus (the sealed original — the whole source — is excluded), pinned
    // entries taken verbatim from the plan. The derived producers are
    // order-independent, so blob-delivery order need not be threaded here.
    let dicts = build_pack_dicts(&g, &blob_order, sealed_digest.as_deref(), &plan)?;

    let index = streaming_index(
        &g,
        &blob_order,
        timestamp,
        sealed_digest.as_deref(),
        sealed_digest.as_ref().map(|_| data.len()),
        content_digest,
    )?;
    let base = index.terms.len();

    let mut w = Writer::with_options(
        &profile,
        WriterOptions {
            layout: Some("streamable".to_string()),
            dicts,
            zstd_level: plan.zstd_level,
            ..WriterOptions::default()
        },
    )
    .map_err(|err| CompactRefusedError(format!("cannot configure the pack writer: {err}")))?;

    // Every authored frame rides the plan's transform chain — the streaming
    // index and content graph included. Leaving those frames untransformed
    // while the blobs compressed was a silent split profile: the pack claimed
    // one frame profile and shipped two.
    let index_dict = plan.index.name();
    let authored = |writer: &mut Writer,
                    frame_type: &str,
                    payload: Value|
     -> Result<(), CompactRefusedError> {
        writer
            .add_frame_with_options(
                frame_type,
                FrameOptions {
                    payload: Some(payload),
                    transform: plan.transform.clone(),
                    dict: index_dict.map(str::to_string),
                    ..FrameOptions::default()
                },
            )
            .map(|_| ())
            .map_err(|err| {
                CompactRefusedError(format!("cannot author the {frame_type} frame: {err}"))
            })
    };

    // Leading streaming index: the catalog presages everything below it.
    authored(&mut w, "terms", writer::terms_payload(&index.terms))?;
    authored(&mut w, "quads", writer::quads_payload(&index.quads))?;
    // Content graph, re-emitted from the folded union (ids shifted by `base`).
    if !g.terms.is_empty() {
        let shifted: Vec<Term> = g.terms.iter().map(|t| shift_term(t, base)).collect();
        authored(&mut w, "terms", writer::terms_payload(&shifted))?;
    }
    if !g.quads.is_empty() {
        let shifted: Vec<Quad> = g
            .quads
            .iter()
            .map(|&(s, p, o, gr)| (s + base, p + base, o + base, gr.map(|x| x + base)))
            .collect();
        authored(&mut w, "quads", writer::quads_payload(&shifted))?;
    }
    if !g.reifiers.is_empty() {
        let shifted: Vec<ReifierRow> = g
            .reifiers
            .iter()
            .map(|&(r, (s, p, o), gr)| {
                (
                    r + base,
                    (s + base, p + base, o + base),
                    gr.map(|x| x + base),
                )
            })
            .collect();
        authored(&mut w, "reifies", writer::reifies_payload(&shifted))?;
    }
    if !g.annotations.is_empty() {
        let shifted: Vec<(usize, usize, usize, Option<usize>)> = g
            .annotations
            .iter()
            .map(|&(r, p, v, gr)| (r + base, p + base, v + base, gr.map(|x| x + base)))
            .collect();
        authored(&mut w, "annot", writer::annot_payload(&shifted))?;
    }
    for sup in shifted_suppressions(&g, base) {
        let payload = writer::suppress_payload(sup.targets, sup.reason.as_deref(), sup.by);
        authored(&mut w, "suppress", payload)?;
    }
    // Blobs in delivery order; declared metadata rides along, and every content
    // frame is primed by the dictionary the plan selects for content — an
    // unused in-band dictionary would otherwise be dead weight. The sealed
    // original (the nested source GTS) is never dict-compressed: it carries
    // its own framing and is excluded from the dictionary training corpus.
    let content_dict = plan.content.name();
    for digest in &blob_order {
        if Some(digest.as_str()) == sealed_digest.as_deref() {
            w.add_blob(
                data,
                Some("application/vnd.blackcat.gts+cbor-seq"),
                Some("source"),
            );
            continue;
        }
        let mt = blob_meta_text(&g, digest, "mt");
        let rep = blob_meta_text(&g, digest, "rep");
        let Some(bytes) = blob_bytes(&g, digest)? else {
            continue;
        };
        let owned = match bytes {
            Cow::Borrowed(bytes) => bytes.to_vec(),
            Cow::Owned(bytes) => bytes,
        };
        if plan.transform.is_empty() {
            w.add_blob_owned(owned, mt.as_deref(), rep.as_deref());
        } else {
            w.add_blob_transformed(
                owned,
                mt.as_deref(),
                rep.as_deref(),
                &plan.transform,
                content_dict,
            )
            .map_err(|err| {
                CompactRefusedError(format!("cannot author the content blob frame: {err}"))
            })?;
        }
    }
    // The MANDATORY packaging head signature: sign ONLY the re-issued
    // ordering commitment below, never the frames already appended above.
    // This attests ordering/packaging — the compactor is its sole attester —
    // distinct from the carried detached authorship signatures (§10.1).
    // `packaging_signer` is a required field (not `Option`), so this always
    // runs: an unsigned pack is unrepresentable through this API.
    packaging_signer
        .finish(w)
        .map_err(|error| CompactRefusedError(format!("cannot sign the packaging index: {error}")))
}

#[cfg(test)]
mod tests {
    use purrdf_ed25519::SigningKey;

    use super::*;
    use crate::reader::read;

    /// A fixed, deterministic Ed25519 signing key (RFC 8032 signing is
    /// deterministic per key + message, so tests stay byte-reproducible) —
    /// mirrors `crates/gts/tests/compaction_signatures.rs::fixed_key`.
    fn fixed_key(byte: u8) -> SigningKey {
        SigningKey::from_bytes(&[byte; 32])
    }

    /// A source GTS file whose content blobs share structure — the corpus a
    /// pack dictionary trains on.
    fn source_with_blobs() -> Vec<u8> {
        let mut w = Writer::new("purrdf.gts");
        for i in 0..64u32 {
            let blob = format!(
                "<https://example.org/s{}> <https://example.org/p> \"claim {} about cats\" .\n",
                i % 37,
                i
            )
            .into_bytes();
            w.add_blob_owned(blob, Some("text/plain"), None);
        }
        w.into_bytes()
    }

    fn digest_quad_present(bytes: &[u8]) -> bool {
        let g = read(bytes, true, None);
        g.terms
            .iter()
            .any(|t| t.value.as_deref() == Some(stream::CONTENT_REFOLD_DIGEST))
    }

    /// A `CompactionParams` with the shared test defaults: fixed timestamp, no
    /// source seal, a fixed packaging signer (the field is mandatory — see
    /// [`CompactionParams::packaging_signer`]).
    fn params(plan: DictPlan, content_digest: Option<&str>) -> CompactionParams<'_> {
        CompactionParams {
            timestamp: "2026-01-01T00:00:00Z",
            seal_original: false,
            plan,
            content_digest,
            packaging_signer: (fixed_key(99), "pack-test".to_string()),
        }
    }

    #[test]
    fn a_declared_zstd_level_requires_a_zstd_family_transform() {
        let invalid = DictPlan {
            dicts: Vec::new(),
            content: DictSelection::Baseline,
            index: DictSelection::Baseline,
            transform: Vec::new(),
            zstd_level: Some(12),
        };
        let err = invalid
            .validate()
            .expect_err("a level without a zstd-family transform must fail");
        assert!(err.to_string().contains("no zstd-family codec"), "{err}");

        let valid = DictPlan {
            transform: vec!["zstd-rsyncable".to_string()],
            ..invalid
        };
        valid
            .validate()
            .expect("a level with a zstd-family transform is valid");
    }

    #[test]
    fn compaction_is_byte_deterministic_with_a_trained_dict() {
        let source = source_with_blobs();
        let a = compact_streamable(
            &source,
            params(DictPlan::single(DictStrategy::Trained), None),
        )
        .expect("compaction succeeds");
        let b = compact_streamable(
            &source,
            params(DictPlan::single(DictStrategy::Trained), None),
        )
        .expect("compaction succeeds");
        assert_eq!(a, b, "a trained-dict compaction must be byte-reproducible");
    }

    #[test]
    fn compacted_pack_folds_cleanly_and_blobs_decode_through_the_dict() {
        let source = source_with_blobs();
        let packed = compact_streamable(
            &source,
            params(DictPlan::single(DictStrategy::Trained), None),
        )
        .expect("compaction succeeds");
        let g = read(&packed, true, None);
        assert!(
            g.diagnostics.is_empty(),
            "compacted pack must fold cleanly (dict resolves): {:?}",
            g.diagnostics
        );
        assert_eq!(g.blobs.len(), 64, "every content blob survives the repack");
        for (_, entry) in &g.blobs {
            entry
                .decoded_vec()
                .expect("dict-compressed blob must decode against the pinned in-band dictionary");
        }
    }

    #[test]
    fn the_dictionary_is_invisible_to_the_fold() {
        let source = source_with_blobs();
        let trained = compact_streamable(
            &source,
            params(DictPlan::single(DictStrategy::Trained), None),
        )
        .expect("trained compaction");
        let undicted = compact_streamable(&source, params(DictPlan::undicted(), None))
            .expect("undicted compaction");
        assert_ne!(
            trained, undicted,
            "pinning a dictionary changes the header bytes"
        );
        let a = read(&trained, true, None);
        let b = read(&undicted, true, None);
        let a_blobs: Vec<Vec<u8>> = a
            .blobs
            .iter()
            .map(|(_, e)| e.decoded_vec().unwrap())
            .collect();
        let b_blobs: Vec<Vec<u8>> = b
            .blobs
            .iter()
            .map(|(_, e)| e.decoded_vec().unwrap())
            .collect();
        assert_eq!(
            a_blobs, b_blobs,
            "the pack dictionary is a compression detail, invisible to the fold"
        );
    }

    #[test]
    fn embedded_content_digest_appears_only_when_supplied() {
        let source = source_with_blobs();
        let without = compact_streamable(
            &source,
            params(DictPlan::single(DictStrategy::Trained), None),
        )
        .expect("compaction");
        assert!(
            !digest_quad_present(&without),
            "no content-refold digest without one supplied"
        );
        let with = compact_streamable(
            &source,
            params(
                DictPlan::single(DictStrategy::Trained),
                Some("blake3:fedcba9876543210"),
            ),
        )
        .expect("compaction");
        assert!(
            digest_quad_present(&with),
            "the supplied content-refold digest must be embedded as provenance"
        );
    }

    #[test]
    fn base64url_round_trips_through_the_encoder_for_every_remainder_length() {
        for len in 0..=17usize {
            let data: Vec<u8> = (0..len).map(|i| (i * 37 + 5) as u8).collect();
            let encoded = base64url_unpadded(&data);
            let decoded = base64url_decode(&encoded)
                .unwrap_or_else(|err| panic!("length {len} round trip must decode: {err}"));
            assert_eq!(decoded, data, "length {len} round trip must be lossless");
        }
    }

    // -----------------------------------------------------------------
    // Adversarial `shifted_suppressions`
    // coverage across all five suppress-target kinds (GTS-SPEC §11).
    // -----------------------------------------------------------------

    fn iri(v: &str) -> Term {
        Term {
            kind: TermKind::Iri,
            value: Some(v.to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
            triple: None,
        }
    }

    fn bnode(v: &str) -> Term {
        Term {
            kind: TermKind::Bnode,
            value: Some(v.to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
            triple: None,
        }
    }

    /// Build a `suppress-target` map: `{"kind": kind, ...extra}`.
    fn target(kind: &str, extra: Vec<(&str, Value)>) -> Value {
        let mut entries: Vec<(Value, Value)> = vec![("kind".into(), kind.into())];
        entries.extend(extra.into_iter().map(|(k, v)| (k.into(), v)));
        Value::Map(entries)
    }

    fn target_id(t: &Value) -> Option<usize> {
        let Value::Map(entries) = t else { return None };
        crate::reader::as_idx(map_get(entries, "id")?)
    }

    fn target_q(t: &Value) -> Option<Vec<Value>> {
        let Value::Map(entries) = t else { return None };
        match map_get(entries, "q")? {
            Value::Array(items) => Some(items.clone()),
            _ => None,
        }
    }

    fn target_digest(t: &Value) -> Option<String> {
        let Value::Map(entries) = t else { return None };
        match map_get(entries, "digest")? {
            Value::Text(s) => Some(s.clone()),
            _ => None,
        }
    }

    fn find_term_id(g: &Graph, value: &str) -> Option<usize> {
        g.terms
            .iter()
            .position(|t| t.value.as_deref() == Some(value))
    }

    #[test]
    fn term_suppression_is_carried_forward_value_wise_and_non_dangling() {
        let mut w = Writer::new("generic");
        w.add_terms(&[
            iri("https://example.org/s"), // 0
            iri("https://example.org/p"), // 1
            iri("https://example.org/o"), // 2 — the suppressed term
        ]);
        w.add_quads(&[(0, 1, 2, None)]);
        w.add_suppress(
            vec![target("term", vec![("id", Value::from(2u64))])],
            Some("pii"),
            None,
        );
        let source = w.into_bytes();

        let packed = compact_streamable(&source, params(DictPlan::undicted(), None))
            .expect("compaction succeeds");
        let g = read(&packed, true, None);
        assert!(
            g.diagnostics.is_empty(),
            "pack must fold cleanly: {:?}",
            g.diagnostics
        );

        // The suppressed term's VALUE is retained — suppression is a display
        // overlay, never a deletion (GTS-SPEC §11).
        let oid =
            find_term_id(&g, "https://example.org/o").expect("suppressed term value retained");
        // The quad it appeared in is likewise retained verbatim.
        let sid = find_term_id(&g, "https://example.org/s").expect("subject retained");
        let pid = find_term_id(&g, "https://example.org/p").expect("predicate retained");
        assert!(
            g.quads.contains(&(sid, pid, oid, None)),
            "the quad naming the suppressed term must still be present (never deleted)"
        );

        let sup = g
            .suppressions
            .iter()
            .find(|s| {
                s.targets
                    .iter()
                    .any(|t| target_text(t, "kind") == Some("term"))
            })
            .expect("term suppression carried forward into the pack");
        let t = sup
            .targets
            .iter()
            .find(|t| target_text(t, "kind") == Some("term"))
            .unwrap();
        assert_eq!(
            target_id(t),
            Some(oid),
            "the carried term suppression must resolve to the SAME term value in the pack"
        );
    }

    #[test]
    fn quad_suppression_is_carried_forward_value_wise_and_non_dangling() {
        let mut w = Writer::new("generic");
        w.add_terms(&[
            iri("https://example.org/s2"), // 0
            iri("https://example.org/p2"), // 1
            iri("https://example.org/o2"), // 2
        ]);
        w.add_quads(&[(0, 1, 2, None)]);
        w.add_suppress(
            vec![target(
                "quad",
                vec![(
                    "q",
                    Value::Array(vec![
                        Value::from(0u64),
                        Value::from(1u64),
                        Value::from(2u64),
                    ]),
                )],
            )],
            None,
            None,
        );
        let source = w.into_bytes();

        let packed = compact_streamable(&source, params(DictPlan::undicted(), None))
            .expect("compaction succeeds");
        let g = read(&packed, true, None);
        assert!(
            g.diagnostics.is_empty(),
            "pack must fold cleanly: {:?}",
            g.diagnostics
        );

        let sid = find_term_id(&g, "https://example.org/s2").expect("subject retained");
        let pid = find_term_id(&g, "https://example.org/p2").expect("predicate retained");
        let oid = find_term_id(&g, "https://example.org/o2").expect("object retained");
        assert!(
            g.quads.contains(&(sid, pid, oid, None)),
            "the targeted quad must still be present (never deleted)"
        );

        let sup = g
            .suppressions
            .iter()
            .find(|s| {
                s.targets
                    .iter()
                    .any(|t| target_text(t, "kind") == Some("quad"))
            })
            .expect("quad suppression carried forward into the pack");
        let t = sup
            .targets
            .iter()
            .find(|t| target_text(t, "kind") == Some("quad"))
            .unwrap();
        let q = target_q(t).expect("quad target carries a \"q\" array");
        let ids: Vec<usize> = q
            .iter()
            .map(|v| crate::reader::as_idx(v).expect("q element is an id"))
            .collect();
        assert_eq!(
            ids,
            vec![sid, pid, oid],
            "the carried quad suppression must resolve to the SAME (s,p,o) values in the pack"
        );
    }

    #[test]
    fn reifier_suppression_is_carried_forward_value_wise_and_non_dangling() {
        let mut w = Writer::new("generic");
        w.add_terms(&[
            iri("https://example.org/s3"), // 0
            iri("https://example.org/p3"), // 1
            iri("https://example.org/o3"), // 2
            bnode("rf1"),                  // 3 — the reifier
        ]);
        w.add_quads(&[(0, 1, 2, None)]);
        w.add_reifies(&[(3, (0, 1, 2), None)]);
        w.add_suppress(
            vec![target("reifier", vec![("id", Value::from(3u64))])],
            None,
            None,
        );
        let source = w.into_bytes();

        let packed = compact_streamable(&source, params(DictPlan::undicted(), None))
            .expect("compaction succeeds");
        let g = read(&packed, true, None);
        assert!(
            g.diagnostics.is_empty(),
            "pack must fold cleanly: {:?}",
            g.diagnostics
        );

        let sid = find_term_id(&g, "https://example.org/s3").expect("subject retained");
        let pid = find_term_id(&g, "https://example.org/p3").expect("predicate retained");
        let oid = find_term_id(&g, "https://example.org/o3").expect("object retained");
        let rid = g
            .reifiers
            .iter()
            .find(|&&(_, spo, _)| spo == (sid, pid, oid))
            .map(|&(r, _, _)| r)
            .expect("the reifier binding must still be present (never deleted)");

        let sup = g
            .suppressions
            .iter()
            .find(|s| {
                s.targets
                    .iter()
                    .any(|t| target_text(t, "kind") == Some("reifier"))
            })
            .expect("reifier suppression carried forward into the pack");
        let t = sup
            .targets
            .iter()
            .find(|t| target_text(t, "kind") == Some("reifier"))
            .unwrap();
        assert_eq!(
            target_id(t),
            Some(rid),
            "the carried reifier suppression must resolve to the SAME reifier binding in the pack"
        );
    }

    #[test]
    fn blob_suppression_is_carried_verbatim_and_the_blob_bytes_are_retained() {
        let mut w = Writer::new("generic");
        let data = b"classified cat photograph".to_vec();
        let digest = digest_str(&data);
        w.add_blob_owned(data.clone(), Some("text/plain"), None);
        w.add_suppress(
            vec![target("blob", vec![("digest", digest.clone().into())])],
            Some("classified"),
            None,
        );
        let source = w.into_bytes();

        let packed = compact_streamable(&source, params(DictPlan::undicted(), None))
            .expect("compaction succeeds");
        let g = read(&packed, true, None);
        assert!(
            g.diagnostics.is_empty(),
            "pack must fold cleanly: {:?}",
            g.diagnostics
        );

        // The suppressed blob's bytes are PRESENT — suppression hides, it
        // never deletes.
        let (_, entry) = g
            .blobs
            .iter()
            .find(|(d, _)| *d == digest)
            .expect("the suppressed blob is retained in the pack, not deleted");
        assert_eq!(
            entry.decoded_vec().expect("blob decodes"),
            data,
            "the retained blob bytes must be byte-identical to the source"
        );

        let sup = g
            .suppressions
            .iter()
            .find(|s| {
                s.targets
                    .iter()
                    .any(|t| target_text(t, "kind") == Some("blob"))
            })
            .expect("blob suppression carried forward into the pack");
        let t = sup
            .targets
            .iter()
            .find(|t| target_text(t, "kind") == Some("blob"))
            .unwrap();
        assert_eq!(
            target_digest(t),
            Some(digest),
            "a blob suppression's digest must be carried verbatim (content-addressing is \
             layout-independent)"
        );
    }

    #[test]
    fn frame_suppression_refuses_compaction() {
        let mut w = Writer::new("generic");
        w.add_terms(&[iri("https://example.org/s4")]);
        w.add_suppress(
            vec![target("frame", vec![("id", Value::Bytes(vec![7u8; 32]))])],
            None,
            None,
        );
        let source = w.into_bytes();

        let err = compact_streamable(&source, params(DictPlan::undicted(), None))
            .expect_err("a frame-addressed suppression must refuse compaction (§10.1)");
        assert!(
            err.to_string().contains("frame-addressed suppression"),
            "refusal message should name the cause: {err}"
        );
    }

    #[test]
    fn base64url_decode_rejects_padding_and_out_of_alphabet_bytes() {
        assert!(
            base64url_decode("aGVsbG8=").is_err(),
            "a trailing '=' padding character must be rejected"
        );
        assert!(
            base64url_decode("+++=").is_err(),
            "standard-alphabet '+' is not in the base64url alphabet"
        );
        assert!(
            base64url_decode("a/b/").is_err(),
            "standard-alphabet '/' is not in the base64url alphabet"
        );
        assert!(
            base64url_decode("a").is_err(),
            "a single trailing character cannot decode to a whole byte"
        );
    }
}
