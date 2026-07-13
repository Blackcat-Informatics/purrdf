// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A single, unified PFC value dictionary: ONE [`PackTermId`](self)
//! (a plain `u64`, 1-based) per distinct
//! [`TermValue`] scanned from an [`RdfDataset`] — REGARDLESS of which role
//! (subject, predicate, object, graph name, literal datatype, triple-term
//! component, reifier/annotation side-table term) it plays — PFC-compressed on
//! disk and decoded into OWNED structures at [`PackDict::open`].
//!
//! # Id layout: one unified id per distinct value
//!
//! Every distinct [`TermValue`] the dataset ever references, in ANY position,
//! gets EXACTLY ONE unified id. Terms sort in the canonical [`TermValue`] order
//! (`Ord`) and unified ids are assigned `1..=N` by that order. There is no
//! per-role partitioning: a term that is used as BOTH a predicate and a
//! subject/object still has exactly one id, and a term used ONLY as a predicate
//! (a "pure predicate", never a subject or object anywhere) still gets an id —
//! there is no role for which a term can be "invisible".
//!
//! This single-id-space design is required by the seam
//! [`crate::DatasetView::term_id_by_value`] exposes: a caller resolves EVERY
//! pattern constant (subject, predicate, OR object) through that one
//! position-agnostic method, and the production `RdfDataset` backend already
//! mints exactly one [`crate::TermId`] per distinct value, matched in any
//! triple position. A classic-HDT split (a separate id space for predicates,
//! as an earlier revision of this module used) is incompatible with that seam:
//! a pure predicate would resolve to `None` there, and a dual-role term would
//! resolve to an id that does not match its predicate-position occurrences.
//! Collapsing to one id space removes that landmine; the triples layer
//! ([`super::triples`]) does not need a split id space either — it already
//! remaps every unified id to a dense, per-partition LOCAL id, so the global
//! id's width/role is irrelevant to triple compression.
//!
//! [`encode`](PackDict::encode) folds in every term the dataset references,
//! including ones with no base-quad S/P/O role of their own:
//!
//! - A quad's named-graph term (`g` slot).
//! - A literal's datatype IRI, and a triple term's `s`/`p`/`o` components
//!   (structural references — a record holds these by id, not by embedded
//!   value, so each must have its OWN entry), transitively.
//! - Every term the RDF 1.2 reifier/annotation side-tables reference (a
//!   reifier resource, a reified triple-term, an annotation's
//!   predicate/object, and any of their graph names) — see the
//!   "auxiliary-value closure" doc on `encode` for the exact mechanism
//!   ([`super::side::SideTables`] is the consumer that needs every
//!   such reference to resolve to a unified id).
//!
//! # Lookup rule (id_by_value vs. predicate_id_by_value)
//!
//! [`PackDict::id_by_value`] resolves ANY value — subject, predicate, object,
//! graph name, or structural reference — to its single unified id.
//! [`PackDict::predicate_id_by_value`] is kept for source-compatibility with
//! callers written against the earlier split-id-space design; it now simply
//! DELEGATES to `id_by_value` (both methods always agree — see
//! [`predicate_id_by_value`](PackDict::predicate_id_by_value)'s doc). A caller
//! resolving a pattern's predicate constant may use either method
//! interchangeably.
//!
//! # Seam for later tasks
//!
//! This module works entirely in raw `u64` unified ids (`PackTermId` is a plain
//! type alias here). The `PackView`/`ViewTermId` newtype wraps these
//! `u64`s.

use std::cmp::Ordering;
use std::fmt;

use crate::hash::{FastMap, FastSet, IdSet};
use crate::ir::term::{StrRange, arena_str};
use crate::{BlankScope, RdfDataset, RdfTextDirection, TermRef, TermValue};

use super::bits::{IntVector, IntVectorRef, PackBitsError, bits_for, read_varint, write_varint};

/// The `rdf:reifies` predicate IRI — the RDF 1.2 reification indirection edge
/// (`reifier rdf:reifies <<( s p o )>>`). A local mirror of the same private
/// constant in `crate::ir::dataset` (also duplicated in `crate::ir::mutable`):
/// the ingest path interns this exact IRI as a term whenever at least one
/// reifier binding exists, even though no [`ReifierRow`](crate::ir::dataset::ReifierRow)
/// tuple stores it directly (`RdfDataset::reifier_quads` looks it up by value).
/// [`PackDict::encode`]'s side-table closure fold-in (below) mirrors that same
/// condition so [`super::side::SideTables`] can mint a unified id for it.
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// The unified term-identity space this module mints: a plain, 1-based `u64` (id `0`
/// is never assigned). A pure type alias, not a newtype — the outer `PackView` seam
/// wraps this in a real [`ViewTermId`](crate::ViewTermId) newtype once it lands (see
/// the [module docs](self)).
pub type PackTermId = u64;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Why decoding a [`PackDict`] byte buffer failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackDictError {
    /// The buffer ended before all the bytes a header promised were present.
    Truncated {
        /// The total leading byte count the format required.
        needed: usize,
        /// The byte count actually available.
        found: usize,
    },
    /// The buffer's header was internally inconsistent, an id reference fell outside
    /// the dictionary's own range, a string was not valid UTF-8, or a front-coded
    /// record failed to reconstruct.
    Malformed(&'static str),
}

impl fmt::Display for PackDictError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { needed, found } => write!(
                f,
                "pack-dict: truncated input: needed at least {needed} bytes, found {found}"
            ),
            Self::Malformed(reason) => write!(f, "pack-dict: malformed input: {reason}"),
        }
    }
}

impl std::error::Error for PackDictError {}

impl From<PackBitsError> for PackDictError {
    fn from(e: PackBitsError) -> Self {
        match e {
            PackBitsError::Truncated { needed, found } => Self::Truncated { needed, found },
            PackBitsError::Malformed(reason) => Self::Malformed(reason),
        }
    }
}

/// Read an 8-byte little-endian header field at `*pos`, advancing `*pos` past it.
/// A small local mirror of `bits::read_header_u64` (private to that module).
fn read_u64_header(bytes: &[u8], pos: &mut usize) -> Result<u64, PackDictError> {
    let end = *pos + 8;
    let slice = bytes.get(*pos..end).ok_or(PackDictError::Truncated {
        needed: end,
        found: bytes.len(),
    })?;
    let value = u64::from_le_bytes(slice.try_into().expect("slice is exactly 8 bytes"));
    *pos = end;
    Ok(value)
}

// ---------------------------------------------------------------------------
// Canonical byte-record codec: tag + self-terminating payload.
// ---------------------------------------------------------------------------

const TAG_IRI: u8 = 0;
const TAG_BLANK: u8 = 1;
const TAG_LITERAL: u8 = 2;
const TAG_TRIPLE: u8 = 3;

const DIR_NONE: u8 = 0;
const DIR_LTR: u8 = 1;
const DIR_RTL: u8 = 2;

/// Number of consecutive canonically-sorted terms per PFC bucket. A bucket's first
/// term is stored as a full record (the "header"); the rest store a shared-prefix
/// length against the immediately PRECEDING record plus their own suffix bytes. 8 is
/// the classic HDT bucket size: small enough that decoding a whole bucket from its
/// offset is cheap, large enough that the header-record overhead amortizes well.
const BUCKET_SIZE: usize = 8;

/// A decoded canonical byte-record, before its string parts are pushed into a
/// [`PackDict`]'s owned arena. Mirrors [`TermValue`]/[`TermRef`] but every id-carrying
/// component (a literal's datatype, a triple term's `s`/`p`/`o`) is already a
/// resolved unified [`PackTermId`].
#[derive(Debug, Clone)]
enum RawRecord {
    /// An IRI, by its full string.
    Iri(String),
    /// A blank node, `(label, scope)`.
    Blank {
        /// The blank-node label.
        label: String,
        /// The blank-node scope ordinal.
        scope: u32,
    },
    /// A literal: lexical form, datatype's unified id, optional language, optional
    /// base direction.
    Literal {
        /// The lexical form, byte-for-byte.
        lexical: String,
        /// The datatype IRI's unified [`PackTermId`] (a dictionary entry in its own
        /// right).
        datatype: PackTermId,
        /// The (already-lowercased) language tag, if any.
        language: Option<String>,
        /// The base direction byte: `0`=none, `1`=ltr, `2`=rtl.
        direction: u8,
    },
    /// A triple term, by its `s`/`p`/`o` unified [`PackTermId`]s.
    Triple {
        /// The quoted triple's subject unified id.
        s: PackTermId,
        /// The quoted triple's predicate unified id.
        p: PackTermId,
        /// The quoted triple's object unified id.
        o: PackTermId,
    },
}

/// Encode `value`'s canonical byte-record: a 1-byte tag then a self-terminating
/// payload (every variable-length field is length-prefixed via [`write_varint`], so
/// the record needs no external length to decode). `value_to_id` resolves a literal's
/// datatype IRI and a triple term's `s`/`p`/`o` components to their unified ids —
/// [`PackDict::encode`] builds it so every such reference is guaranteed present
/// (see that method's closure step).
fn encode_record(value: &TermValue, value_to_id: &FastMap<TermValue, PackTermId>) -> Vec<u8> {
    let mut out = Vec::new();
    match value {
        TermValue::Iri(s) => {
            out.push(TAG_IRI);
            write_varint(&mut out, s.len() as u64);
            out.extend_from_slice(s.as_bytes());
        }
        TermValue::Blank { label, scope } => {
            out.push(TAG_BLANK);
            write_varint(&mut out, label.len() as u64);
            out.extend_from_slice(label.as_bytes());
            write_varint(&mut out, u64::from(scope.ordinal()));
        }
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => {
            out.push(TAG_LITERAL);
            write_varint(&mut out, lexical_form.len() as u64);
            out.extend_from_slice(lexical_form.as_bytes());
            let datatype_id = *value_to_id.get(&TermValue::Iri(datatype.clone())).expect(
                "PackDict::encode's closure guarantees a literal's datatype is a dictionary entry",
            );
            write_varint(&mut out, datatype_id);
            match language {
                Some(lang) => {
                    out.push(1);
                    write_varint(&mut out, lang.len() as u64);
                    out.extend_from_slice(lang.as_bytes());
                }
                None => out.push(0),
            }
            out.push(match direction {
                None => DIR_NONE,
                Some(RdfTextDirection::Ltr) => DIR_LTR,
                Some(RdfTextDirection::Rtl) => DIR_RTL,
            });
        }
        TermValue::Triple { s, p, o } => {
            out.push(TAG_TRIPLE);
            let sid = *value_to_id.get(s.as_ref()).expect(
                "PackDict::encode's closure guarantees a triple term's subject is a dictionary entry",
            );
            let pid = *value_to_id.get(p.as_ref()).expect(
                "PackDict::encode's closure guarantees a triple term's predicate is a dictionary entry",
            );
            let oid = *value_to_id.get(o.as_ref()).expect(
                "PackDict::encode's closure guarantees a triple term's object is a dictionary entry",
            );
            write_varint(&mut out, sid);
            write_varint(&mut out, pid);
            write_varint(&mut out, oid);
        }
    }
    out
}

/// Decode one self-terminating canonical byte-record from the START of `bytes`.
/// Returns the decoded record and the number of leading bytes it consumed — `bytes`
/// may carry trailing data after the record (the caller slices to that length).
fn decode_record(bytes: &[u8]) -> Result<(RawRecord, usize), PackDictError> {
    let tag = *bytes.first().ok_or(PackDictError::Truncated {
        needed: 1,
        found: 0,
    })?;
    let mut pos = 1usize;
    let record = match tag {
        TAG_IRI => {
            let s = read_len_prefixed_str(bytes, &mut pos)?;
            RawRecord::Iri(s)
        }
        TAG_BLANK => {
            let label = read_len_prefixed_str(bytes, &mut pos)?;
            let scope = read_varint(bytes, &mut pos)?;
            let scope = u32::try_from(scope)
                .map_err(|_| PackDictError::Malformed("dict: blank scope exceeds u32"))?;
            RawRecord::Blank { label, scope }
        }
        TAG_LITERAL => {
            let lexical = read_len_prefixed_str(bytes, &mut pos)?;
            let datatype = read_varint(bytes, &mut pos)?;
            let has_language = *bytes.get(pos).ok_or(PackDictError::Truncated {
                needed: pos + 1,
                found: bytes.len(),
            })?;
            pos += 1;
            let language = match has_language {
                0 => None,
                1 => Some(read_len_prefixed_str(bytes, &mut pos)?),
                _ => return Err(PackDictError::Malformed("dict: bad literal language flag")),
            };
            let direction = *bytes.get(pos).ok_or(PackDictError::Truncated {
                needed: pos + 1,
                found: bytes.len(),
            })?;
            pos += 1;
            if !matches!(direction, DIR_NONE | DIR_LTR | DIR_RTL) {
                return Err(PackDictError::Malformed("dict: bad literal direction byte"));
            }
            RawRecord::Literal {
                lexical,
                datatype,
                language,
                direction,
            }
        }
        TAG_TRIPLE => {
            let s = read_varint(bytes, &mut pos)?;
            let p = read_varint(bytes, &mut pos)?;
            let o = read_varint(bytes, &mut pos)?;
            RawRecord::Triple { s, p, o }
        }
        _ => return Err(PackDictError::Malformed("dict: unknown term tag")),
    };
    Ok((record, pos))
}

/// Read a `varint(len)` followed by `len` UTF-8 bytes, advancing `*pos` past both.
fn read_len_prefixed_str(bytes: &[u8], pos: &mut usize) -> Result<String, PackDictError> {
    let len = read_varint(bytes, pos)? as usize;
    let end = *pos + len;
    let slice = bytes.get(*pos..end).ok_or(PackDictError::Truncated {
        needed: end,
        found: bytes.len(),
    })?;
    let s = std::str::from_utf8(slice)
        .map_err(|_| PackDictError::Malformed("dict: string is not valid utf-8"))?
        .to_owned();
    *pos = end;
    Ok(s)
}

/// The length, in bytes, of the common leading prefix of `a` and `b`.
fn common_prefix_len(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b.iter()).take_while(|(x, y)| x == y).count()
}

// ---------------------------------------------------------------------------
// PFC encode/decode of the single, unified value list.
// ---------------------------------------------------------------------------

/// PFC-encode the dictionary's already-sorted term values into a self-contained
/// byte block: `u64 term_count`, `u64 bucket_count`, a serialized [`IntVector`]
/// of per-bucket BYTE offsets into the bucket-data stream (so a bucket can be
/// located without scanning from the start), then the bucket-data stream
/// itself.
fn encode_values(values: &[TermValue], value_to_id: &FastMap<TermValue, PackTermId>) -> Vec<u8> {
    let term_count = values.len();
    let bucket_count = term_count.div_ceil(BUCKET_SIZE);
    let mut bucket_data = Vec::new();
    let mut offsets: Vec<u64> = Vec::with_capacity(bucket_count);
    let mut prev_record: Vec<u8> = Vec::new();

    for (i, value) in values.iter().enumerate() {
        let record = encode_record(value, value_to_id);
        if i % BUCKET_SIZE == 0 {
            offsets.push(bucket_data.len() as u64);
            bucket_data.extend_from_slice(&record);
        } else {
            let shared_len = common_prefix_len(&prev_record, &record);
            let suffix = &record[shared_len..];
            write_varint(&mut bucket_data, shared_len as u64);
            write_varint(&mut bucket_data, suffix.len() as u64);
            bucket_data.extend_from_slice(suffix);
        }
        prev_record = record;
    }

    let max_offset = offsets.iter().copied().max().unwrap_or(0);
    let mut offset_vec = IntVector::with_width(bits_for(max_offset));
    for &o in &offsets {
        offset_vec.push(o);
    }

    let mut out = Vec::new();
    out.extend_from_slice(&(term_count as u64).to_le_bytes());
    out.extend_from_slice(&(bucket_count as u64).to_le_bytes());
    out.extend_from_slice(&offset_vec.to_bytes());
    out.extend_from_slice(&bucket_data);
    out
}

/// Decode the PFC-encoded value list, pushing each entry into `dict` in unified-id
/// order (`1..=n_terms`). Returns the term count.
fn decode_values(bytes: &[u8], dict: &mut PackDict) -> Result<u64, PackDictError> {
    let mut pos = 0usize;
    let term_count = read_u64_header(bytes, &mut pos)?;
    let bucket_count = read_u64_header(bytes, &mut pos)?;
    let offsets = IntVectorRef::from_bytes(&bytes[pos..])?;
    if offsets.len() as u64 != bucket_count {
        return Err(PackDictError::Malformed(
            "dict: bucket offset count disagrees with section header",
        ));
    }
    pos += offsets.serialized_len();
    let bucket_data = &bytes[pos..];

    let mut term_idx = 0u64;
    for bucket_idx in 0..bucket_count as usize {
        let bucket_start = usize::try_from(offsets.get(bucket_idx))
            .map_err(|_| PackDictError::Malformed("dict: bucket offset exceeds usize"))?;
        let items_in_bucket = (term_count - term_idx).min(BUCKET_SIZE as u64) as usize;
        let mut cursor = bucket_start;
        let mut prev_bytes: Vec<u8> = Vec::new();
        for j in 0..items_in_bucket {
            if j == 0 {
                let slice = bucket_data
                    .get(cursor..)
                    .ok_or(PackDictError::Malformed("dict: bucket offset out of range"))?;
                let (raw, consumed) = decode_record(slice)?;
                prev_bytes = slice[..consumed].to_vec();
                cursor += consumed;
                push_entry(dict, raw)?;
            } else {
                let mut p = cursor;
                let shared_len = read_varint(bucket_data, &mut p)? as usize;
                let suffix_len = read_varint(bucket_data, &mut p)? as usize;
                let suffix_end = p + suffix_len;
                let suffix = bucket_data
                    .get(p..suffix_end)
                    .ok_or(PackDictError::Truncated {
                        needed: suffix_end,
                        found: bucket_data.len(),
                    })?;
                if shared_len > prev_bytes.len() {
                    return Err(PackDictError::Malformed(
                        "dict: front-coded shared-prefix length exceeds previous record",
                    ));
                }
                let mut record = prev_bytes[..shared_len].to_vec();
                record.extend_from_slice(suffix);
                let (raw, consumed) = decode_record(&record)?;
                if consumed != record.len() {
                    return Err(PackDictError::Malformed(
                        "dict: front-coded record has trailing garbage",
                    ));
                }
                prev_bytes = record;
                cursor = suffix_end;
                push_entry(dict, raw)?;
            }
            term_idx += 1;
        }
    }
    Ok(term_idx)
}

/// Push a decoded [`RawRecord`] into `dict`'s owned arena/entry table as the NEXT
/// unified id (the caller must call this in strict unified-id order).
fn push_entry(dict: &mut PackDict, raw: RawRecord) -> Result<(), PackDictError> {
    let entry = match raw {
        RawRecord::Iri(s) => DictEntry::Iri(dict.push_str(&s)?),
        RawRecord::Blank { label, scope } => DictEntry::Blank {
            label: dict.push_str(&label)?,
            scope: BlankScope(scope),
        },
        RawRecord::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => DictEntry::Literal {
            lexical: dict.push_str(&lexical)?,
            datatype,
            language: match language {
                Some(l) => Some(dict.push_str(&l)?),
                None => None,
            },
            direction: match direction {
                DIR_LTR => Some(RdfTextDirection::Ltr),
                DIR_RTL => Some(RdfTextDirection::Rtl),
                _ => None,
            },
        },
        RawRecord::Triple { s, p, o } => DictEntry::Triple { s, p, o },
    };
    dict.entries.push(entry);
    Ok(())
}

// ---------------------------------------------------------------------------
// Owned decoded entry (the storage form behind a unified PackTermId).
// ---------------------------------------------------------------------------

/// One decoded dictionary entry, addressed by unified id. Mirrors
/// [`InternedTerm`](crate::ir::term) / `GlobalInternedTerm`: strings are
/// [`StrRange`]s into [`PackDict`]'s owned arena; id-carrying components are other
/// unified [`PackTermId`]s in THIS dictionary.
#[derive(Debug, Clone, Copy)]
enum DictEntry {
    /// An IRI, by its arena range.
    Iri(StrRange),
    /// A blank node, `(label, scope)`.
    Blank {
        /// The blank-node label's arena range.
        label: StrRange,
        /// The blank-node scope.
        scope: BlankScope,
    },
    /// A literal.
    Literal {
        /// The lexical form's arena range.
        lexical: StrRange,
        /// The datatype IRI's unified id.
        datatype: PackTermId,
        /// The language tag's arena range, if any.
        language: Option<StrRange>,
        /// The base direction, if any.
        direction: Option<RdfTextDirection>,
    },
    /// A triple term, by its component unified ids.
    Triple {
        /// The subject's unified id.
        s: PackTermId,
        /// The predicate's unified id.
        p: PackTermId,
        /// The object's unified id.
        o: PackTermId,
    },
}

// ---------------------------------------------------------------------------
// EncodedDict — the self-contained, versioned on-disk form.
// ---------------------------------------------------------------------------

/// The on-disk format version [`EncodedDict::to_bytes`] writes and
/// [`EncodedDict::from_bytes`] requires.
const DICT_FORMAT_VERSION: u8 = 1;

/// The output of [`PackDict::encode`]: the single PFC-encoded value-list byte
/// block plus its term count. Self-contained and independently
/// round-trippable via [`to_bytes`](Self::to_bytes)/[`from_bytes`](Self::from_bytes)
/// — the on-disk container format frames these bytes alongside its other
/// blocks, or a caller can treat an `EncodedDict` as a standalone dictionary
/// file.
#[derive(Debug, Clone)]
pub struct EncodedDict {
    /// The total number of unified ids this dictionary mints — one per
    /// distinct [`TermValue`] the dataset references, in ANY role.
    pub n_terms: u64,
    values_bytes: Vec<u8>,
}

impl EncodedDict {
    /// The total number of unified ids this dictionary mints.
    #[must_use]
    pub fn n_terms(&self) -> u64 {
        self.n_terms
    }

    /// Serialize to the self-contained, versioned on-disk form: a 1-byte version tag
    /// followed by the PFC-encoded value-list bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(1 + self.values_bytes.len());
        out.push(DICT_FORMAT_VERSION);
        out.extend_from_slice(&self.values_bytes);
        out
    }

    /// Parse [`to_bytes`](Self::to_bytes)'s output.
    ///
    /// # Errors
    ///
    /// [`PackDictError::Truncated`]/[`PackDictError::Malformed`] on a short buffer,
    /// an unsupported version tag, or a header whose own fields are inconsistent.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PackDictError> {
        let version = *bytes.first().ok_or(PackDictError::Truncated {
            needed: 1,
            found: 0,
        })?;
        if version != DICT_FORMAT_VERSION {
            return Err(PackDictError::Malformed("dict: unsupported format version"));
        }
        let values_bytes = bytes[1..].to_vec();
        let n_terms = peek_term_count(&values_bytes)?;
        Ok(Self {
            n_terms,
            values_bytes,
        })
    }

    /// Decode the PFC value list into an owned [`PackDict`], ready for
    /// [`resolve`](PackDict::resolve)/[`id_by_value`](PackDict::id_by_value) queries.
    ///
    /// # Errors
    ///
    /// [`PackDictError`] if the buffer is malformed, truncated, or contains an
    /// out-of-range id reference.
    pub fn decode(&self) -> Result<PackDict, PackDictError> {
        let mut dict = PackDict {
            arena: Vec::new(),
            entries: Vec::new(),
        };
        let decoded = decode_values(&self.values_bytes, &mut dict)?;
        if decoded != self.n_terms {
            return Err(PackDictError::Malformed(
                "dict: decoded term count disagrees with the header",
            ));
        }
        dict.validate_references()?;
        Ok(dict)
    }
}

/// Peek the value list's own leading `u64 term_count` header field without
/// decoding its records.
fn peek_term_count(values_bytes: &[u8]) -> Result<u64, PackDictError> {
    let mut pos = 0usize;
    read_u64_header(values_bytes, &mut pos)
}

// ---------------------------------------------------------------------------
// PackDict — the owned, decoded, query-ready dictionary.
// ---------------------------------------------------------------------------

/// The decoded, query-ready value dictionary: one unified [`PackTermId`] per
/// distinct [`TermValue`], resolved from an owned byte arena + entry table built by
/// [`EncodedDict::decode`]/[`PackDict::open`]. See the [module docs](self) for the
/// single-id-space model and the `id_by_value`/`predicate_id_by_value` equivalence.
#[derive(Debug, Clone)]
pub struct PackDict {
    /// The byte arena owning every interned string ONCE; entries hold ranges.
    arena: Vec<u8>,
    /// Dense table of decoded entries; unified id `i` (1-based) lives at
    /// `entries[i - 1]`.
    entries: Vec<DictEntry>,
}

impl PackDict {
    /// Scan `dataset`'s base quads and build the unified dictionary (see the
    /// [module docs](self) for the exact id-assignment rule), returning the
    /// PFC-encoded, not-yet-parsed [`EncodedDict`].
    ///
    /// # The auxiliary-value closure
    ///
    /// A literal's datatype IRI and a triple term's `s`/`p`/`o` components must each
    /// hold their OWN unified id (records reference them by id, not by embedded
    /// value), but they do not necessarily appear as a subject/predicate/object of
    /// any base quad (e.g. `xsd:integer` as a literal's datatype is rarely itself a
    /// triple's subject or object). After the base-role scan, this method computes
    /// the closure of every such auxiliary reference, transitively (an auxiliary
    /// value can itself be a literal or a nested triple term), folding in any value
    /// not already collected. A value already present keeps its existing id and is
    /// never duplicated.
    #[must_use]
    pub fn encode(dataset: &RdfDataset) -> EncodedDict {
        // Step 1: every distinct term id used in ANY base-quad role — subject,
        // predicate, object, or graph name — collapsed into ONE set (this is
        // the crux of the single-id-space fix: unlike an HDT-style split, a
        // predicate and a subject/object share the very same membership test).
        let mut base_ids: IdSet = IdSet::default();
        for q in dataset.quads() {
            base_ids.insert(q.s);
            base_ids.insert(q.p);
            base_ids.insert(q.o);
            if let Some(g) = q.g {
                base_ids.insert(g);
            }
        }
        let mut values: Vec<TermValue> =
            base_ids.iter().map(|&id| dataset.term_value(id)).collect();

        // Step 1.5: RDF 1.2 side-table term closure roots. A
        // reifier row (`reifier, triple-term, graph`) and an annotation row
        // (`reifier, predicate, object, graph`) may reference terms that hold NO
        // base-quad role at all — e.g. a reifier resource that is never itself a
        // triple's subject/object. Collect every such reference here as an
        // ADDITIONAL root, so [`super::side::SideTables`] always finds a unified
        // id for every side-table reference it needs to resolve. A referenced
        // triple term's own `s`/`p`/`o` components are handled transitively by
        // the shared `while qi < queue.len()` worklist loop below — a
        // `TermValue::Triple` entry always expands its components there,
        // whatever put it in the queue.
        for (reifier, triple, graph) in dataset.reifiers_with_graph() {
            values.push(dataset.term_value(reifier));
            values.push(dataset.term_value(triple));
            if let Some(g) = graph {
                values.push(dataset.term_value(g));
            }
        }
        for (reifier, pred, obj, graph) in dataset.annotations_with_graph() {
            values.push(dataset.term_value(reifier));
            values.push(dataset.term_value(pred));
            values.push(dataset.term_value(obj));
            if let Some(g) = graph {
                values.push(dataset.term_value(g));
            }
        }
        // The `rdf:reifies` indirection predicate itself: see the [`RDF_REIFIES`]
        // doc comment for why it must be folded in on the SAME condition
        // (reifiers non-empty) the ingest path uses to intern it.
        if dataset.reifiers_with_graph().next().is_some() {
            values.push(TermValue::Iri(RDF_REIFIES.to_owned()));
        }

        // Deterministic regardless of hash-set iteration order: `values` is
        // sorted+deduped here (and again after the closure step below) before
        // any id is assigned — no hash-iteration order ever reaches the output
        // (byte-determinism discipline).
        values.sort();
        values.dedup();

        // Step 2: closure over auxiliary structural references (literal
        // datatypes, triple components) not already collected — transitively,
        // since an auxiliary value can itself be a literal or a nested triple
        // term.
        let mut present: FastSet<TermValue> = values.iter().cloned().collect();
        let mut queue: Vec<TermValue> = values.clone();
        let mut extra: Vec<TermValue> = Vec::new();
        let mut qi = 0usize;
        while qi < queue.len() {
            let current = queue[qi].clone();
            match &current {
                TermValue::Literal { datatype, .. } => {
                    let dt_val = TermValue::Iri(datatype.clone());
                    if present.insert(dt_val.clone()) {
                        extra.push(dt_val.clone());
                        queue.push(dt_val);
                    }
                }
                TermValue::Triple { s, p, o } => {
                    for comp in [s.as_ref(), p.as_ref(), o.as_ref()] {
                        if !present.contains(comp) {
                            present.insert(comp.clone());
                            extra.push(comp.clone());
                            queue.push(comp.clone());
                        }
                    }
                }
                TermValue::Iri(_) | TermValue::Blank { .. } => {}
            }
            qi += 1;
        }
        values.extend(extra);
        values.sort();
        values.dedup();

        // Step 3: assign unified ids 1..=N in canonical TermValue order.
        let mut value_to_id: FastMap<TermValue, PackTermId> = FastMap::default();
        for (i, v) in values.iter().enumerate() {
            value_to_id.insert(v.clone(), (i + 1) as PackTermId);
        }

        EncodedDict {
            n_terms: values.len() as u64,
            values_bytes: encode_values(&values, &value_to_id),
        }
    }

    /// Parse and decode a dictionary from [`EncodedDict::to_bytes`]'s output in one
    /// step.
    ///
    /// # Errors
    ///
    /// [`PackDictError`] if the buffer is truncated, malformed, or contains an
    /// out-of-range internal id reference.
    pub fn open(bytes: &[u8]) -> Result<Self, PackDictError> {
        EncodedDict::from_bytes(bytes)?.decode()
    }

    /// The total number of unified ids this dictionary mints — one per distinct
    /// [`TermValue`] the dataset references, in ANY role.
    #[must_use]
    pub fn n_terms(&self) -> u64 {
        self.entries.len() as u64
    }

    /// `true` iff at least one dictionary entry is an RDF 1.2 triple term (quoted
    /// triple) — mirrors `RdfDataset::capabilities`'s `quoted_triples` flag
    /// (`terms.iter().any(|t| matches!(t, InternedTerm::Triple { .. }))`), but
    /// scoped to this dictionary's entries (every triple term that is reachable
    /// from a base quad, a literal datatype, another triple term, a graph name, or
    /// an RDF 1.2 reifier/annotation side-table reference; see
    /// [`encode`](Self::encode)). Used by [`super::side::capabilities`].
    #[must_use]
    pub fn has_triple_term(&self) -> bool {
        self.entries
            .iter()
            .any(|e| matches!(e, DictEntry::Triple { .. }))
    }

    /// Append a string to the arena, returning its range.
    ///
    /// # Errors
    ///
    /// [`PackDictError::Malformed`] if the arena would exceed `u32::MAX` bytes.
    fn push_str(&mut self, s: &str) -> Result<StrRange, PackDictError> {
        let offset = u32::try_from(self.arena.len())
            .map_err(|_| PackDictError::Malformed("dict: term arena exceeds u32::MAX bytes"))?;
        let len = u32::try_from(s.len())
            .map_err(|_| PackDictError::Malformed("dict: term string exceeds u32::MAX bytes"))?;
        offset.checked_add(len).ok_or(PackDictError::Malformed(
            "dict: term arena exceeds u32::MAX bytes",
        ))?;
        self.arena.extend_from_slice(s.as_bytes());
        Ok(StrRange { offset, len })
    }

    /// The decoded entry addressed by unified id `id` (1-based).
    ///
    /// # Panics
    ///
    /// Panics if `id` is `0` or exceeds [`n_terms`](Self::n_terms) — a caller-side
    /// bug (an id from a DIFFERENT dictionary, or one never minted), not a decoding
    /// concern (bounds on decoded ids are enforced once at
    /// [`EncodedDict::decode`]-time via [`validate_references`](Self::validate_references)).
    fn entry(&self, id: PackTermId) -> &DictEntry {
        let idx = id
            .checked_sub(1)
            .expect("PackDict: id 0 is never a valid unified id");
        &self.entries[usize::try_from(idx).expect("PackDict: id exceeds usize on this platform")]
    }

    /// Resolve a unified id to its borrowed [`TermRef`] (arena-borrow; no
    /// allocation). A literal's datatype and a triple term's `s`/`p`/`o` resolve to
    /// their unified `u64` ids — recurse via [`resolve`](Self::resolve) again to
    /// follow them.
    ///
    /// # Panics
    ///
    /// Panics if `id` is out of range — see [`entry`](Self::entry).
    #[must_use]
    pub fn resolve(&self, id: PackTermId) -> TermRef<'_, PackTermId> {
        match self.entry(id) {
            DictEntry::Iri(r) => TermRef::Iri(arena_str(&self.arena, *r)),
            DictEntry::Blank { label, scope } => TermRef::Blank {
                label: arena_str(&self.arena, *label),
                scope: *scope,
            },
            DictEntry::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => TermRef::Literal {
                lexical: arena_str(&self.arena, *lexical),
                datatype: *datatype,
                language: language.map(|r| arena_str(&self.arena, r)),
                direction: *direction,
            },
            DictEntry::Triple { s, p, o } => TermRef::Triple {
                s: *s,
                p: *p,
                o: *o,
            },
        }
    }

    /// Resolve a unified id to its self-contained, dataset-independent
    /// [`TermValue`], recursing through a literal's datatype and a triple term's
    /// components (the inverse of the value→id assignment in
    /// [`encode`](Self::encode)).
    ///
    /// # Panics
    ///
    /// Panics if `id` is out of range — see [`entry`](Self::entry).
    #[must_use]
    pub fn term_value(&self, id: PackTermId) -> TermValue {
        match self.entry(id) {
            DictEntry::Iri(r) => TermValue::Iri(arena_str(&self.arena, *r).to_owned()),
            DictEntry::Blank { label, scope } => TermValue::Blank {
                label: arena_str(&self.arena, *label).to_owned(),
                scope: *scope,
            },
            DictEntry::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                let datatype_id = *datatype;
                let datatype_str = match self.entry(datatype_id) {
                    DictEntry::Iri(r) => arena_str(&self.arena, *r).to_owned(),
                    _ => unreachable!("dict: a literal's datatype entry must be an IRI"),
                };
                TermValue::Literal {
                    lexical_form: arena_str(&self.arena, *lexical).to_owned(),
                    datatype: datatype_str,
                    language: language.map(|r| arena_str(&self.arena, r).to_owned()),
                    direction: *direction,
                }
            }
            DictEntry::Triple { s, p, o } => TermValue::Triple {
                s: Box::new(self.term_value(*s)),
                p: Box::new(self.term_value(*p)),
                o: Box::new(self.term_value(*o)),
            },
        }
    }

    /// The unified id of `value`, searching the WHOLE dictionary (there is only
    /// one id space — see the [module docs](self)). `None` if `value` was never
    /// interned by [`encode`](Self::encode). `O(log n_terms)`, using
    /// [`term_value`](Self::term_value) as the (canonically-ordered) comparator.
    #[must_use]
    pub fn id_by_value(&self, value: &TermValue) -> Option<PackTermId> {
        let mut lo = 0u64;
        let mut hi = self.n_terms();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let id = mid + 1;
            match self.term_value(id).cmp(value) {
                Ordering::Less => lo = mid + 1,
                Ordering::Greater => hi = mid,
                Ordering::Equal => return Some(id),
            }
        }
        None
    }

    /// The unified id of `value` in its predicate role. Kept for
    /// source-compatibility with callers written against an earlier
    /// split-id-space design; this dictionary mints exactly ONE id per value
    /// regardless of role, so this method now simply DELEGATES to
    /// [`id_by_value`](Self::id_by_value) — the two are always equal for every
    /// `value`, including a "pure predicate" (a value used ONLY as a predicate,
    /// never a subject or object), which now resolves here too instead of
    /// yielding `None`. See the [module docs](self).
    #[must_use]
    pub fn predicate_id_by_value(&self, value: &TermValue) -> Option<PackTermId> {
        self.id_by_value(value)
    }

    /// Validate that every decoded id reference (a literal's datatype, a triple
    /// term's `s`/`p`/`o`) falls within `1..=n_terms()`, and that a literal's
    /// datatype id specifically resolves to an [`DictEntry::Iri`] (never a
    /// blank node, a literal, or a triple term). Called once by
    /// [`EncodedDict::decode`] after the value list is fully decoded (id ranges
    /// are only fully known once the whole dictionary is assembled).
    ///
    /// This is what lets [`term_value`](Self::term_value) treat a literal's
    /// datatype entry as an IRI unconditionally: any pack that survived this
    /// check can never violate that invariant.
    fn validate_references(&self) -> Result<(), PackDictError> {
        let n = self.n_terms();
        let in_range = |id: PackTermId| id >= 1 && id <= n;
        for entry in &self.entries {
            match entry {
                DictEntry::Literal { datatype, .. } => {
                    if !in_range(*datatype) {
                        return Err(PackDictError::Malformed(
                            "dict: literal datatype id out of range",
                        ));
                    }
                    if !matches!(self.entry(*datatype), DictEntry::Iri(_)) {
                        return Err(PackDictError::Malformed(
                            "dict: literal datatype id does not reference an IRI",
                        ));
                    }
                }
                DictEntry::Triple { s, p, o } => {
                    if !in_range(*s) || !in_range(*p) || !in_range(*o) {
                        return Err(PackDictError::Malformed(
                            "dict: triple component id out of range",
                        ));
                    }
                }
                DictEntry::Iri(_) | DictEntry::Blank { .. } => {}
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RdfDatasetBuilder, RdfLiteral, TermId};
    use proptest::prelude::*;
    use proptest::strategy::BoxedStrategy;
    use std::collections::HashSet;

    /// Intern one dataset-independent value into a builder, recursing for triple
    /// terms (mirrors `paged_backend.rs`'s `intern_value` helper).
    fn intern_value(b: &mut RdfDatasetBuilder, v: &TermValue) -> TermId {
        match v {
            TermValue::Iri(s) => b.intern_iri(s),
            TermValue::Blank { label, scope } => b.intern_blank(label, *scope),
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            } => b.intern_literal(RdfLiteral {
                lexical_form: lexical_form.clone(),
                datatype: Some(datatype.clone()),
                language: language.clone(),
                direction: *direction,
            }),
            TermValue::Triple { s, p, o } => {
                let s = intern_value(b, s);
                let p = intern_value(b, p);
                let o = intern_value(b, o);
                b.intern_triple(s, p, o)
            }
        }
    }

    /// Build a frozen dataset from `(s, p, o)` triples in the default graph.
    fn build_dataset(triples: &[(TermValue, TermValue, TermValue)]) -> std::sync::Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        for (s, p, o) in triples {
            let s = intern_value(&mut b, s);
            let p = intern_value(&mut b, p);
            let o = intern_value(&mut b, o);
            b.push_quad(s, p, o, None);
        }
        b.freeze().expect("valid dataset")
    }

    fn iri(name: &str) -> TermValue {
        TermValue::iri(format!("http://example.org/{name}"))
    }

    // -- Unit tests: one per TermValue kind --------------------------------------

    #[test]
    fn iri_round_trips() {
        let dataset = build_dataset(&[(iri("s"), iri("p"), iri("o"))]);
        let encoded = PackDict::encode(&dataset);
        let dict = PackDict::open(&encoded.to_bytes()).expect("opens");
        let id = dict.id_by_value(&iri("s")).expect("present");
        assert_eq!(dict.term_value(id), iri("s"));
        assert_eq!(dict.resolve(id), TermRef::Iri("http://example.org/s"));
    }

    #[test]
    fn blank_round_trips_with_scope() {
        let blank = TermValue::Blank {
            label: "b0".to_string(),
            scope: BlankScope(2),
        };
        let dataset = build_dataset(&[(blank.clone(), iri("p"), iri("o"))]);
        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("opens");
        let id = dict.id_by_value(&blank).expect("present");
        assert_eq!(dict.term_value(id), blank);
        assert_eq!(
            dict.resolve(id),
            TermRef::Blank {
                label: "b0",
                scope: BlankScope(2),
            }
        );
    }

    #[test]
    fn literal_kinds_round_trip() {
        let simple = TermValue::simple_literal("plain");
        let typed = TermValue::typed_literal("42", "http://www.w3.org/2001/XMLSchema#integer");
        let lang = TermValue::lang_literal("bonjour", "FR");
        let directional = TermValue::Literal {
            lexical_form: "hello".to_string(),
            datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_string(),
            language: Some("en".to_string()),
            direction: Some(RdfTextDirection::Rtl),
        };
        let dataset = build_dataset(&[
            (iri("s1"), iri("p"), simple.clone()),
            (iri("s2"), iri("p"), typed.clone()),
            (iri("s3"), iri("p"), lang.clone()),
            (iri("s4"), iri("p"), directional.clone()),
        ]);
        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("opens");
        for value in [&simple, &typed, &lang, &directional] {
            let id = dict
                .id_by_value(value)
                .unwrap_or_else(|| panic!("{value:?} present"));
            assert_eq!(&dict.term_value(id), value);
        }
        // The language tag was lowercased at intern time (C0.1); the dict must
        // preserve that, not the original "FR".
        let lang_id = dict.id_by_value(&lang).expect("present");
        let TermRef::Literal { language, .. } = dict.resolve(lang_id) else {
            panic!("expected a literal");
        };
        assert_eq!(language, Some("fr"));
    }

    #[test]
    fn triple_term_round_trips_recursively() {
        let inner = TermValue::Triple {
            s: Box::new(iri("a")),
            p: Box::new(iri("b")),
            o: Box::new(TermValue::simple_literal("leaf")),
        };
        let outer = TermValue::Triple {
            s: Box::new(inner),
            p: Box::new(iri("meta")),
            o: Box::new(iri("target")),
        };
        let dataset = build_dataset(&[(iri("subj"), iri("about"), outer.clone())]);
        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("opens");
        let id = dict.id_by_value(&outer).expect("outer triple present");
        assert_eq!(dict.term_value(id), outer);
    }

    // -- Unified id-space invariants ----------------------------------------------

    #[test]
    fn empty_dataset_yields_empty_dictionary() {
        let dataset = build_dataset(&[]);
        let encoded = PackDict::encode(&dataset);
        assert_eq!(encoded.n_terms(), 0);
        let dict = PackDict::open(&encoded.to_bytes()).expect("opens");
        assert_eq!(dict.n_terms(), 0);
        assert_eq!(dict.id_by_value(&iri("anything")), None);
        assert_eq!(dict.predicate_id_by_value(&iri("anything")), None);
    }

    #[test]
    fn every_distinct_value_gets_exactly_one_id() {
        // "s"/"p"/"o" are three distinct terms; the unified dictionary must mint
        // exactly three ids, one per value, regardless of role.
        let dataset = build_dataset(&[(iri("s"), iri("p"), iri("o"))]);
        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("opens");
        assert_eq!(dict.n_terms(), 3);
        let mut ids: Vec<PackTermId> = [iri("s"), iri("p"), iri("o")]
            .iter()
            .map(|v| dict.id_by_value(v).expect("present"))
            .collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(
            ids,
            vec![1, 2, 3],
            "three distinct values, three distinct ids"
        );
    }

    #[test]
    fn a_value_used_as_both_subject_and_object_gets_one_id() {
        // "x" is a subject in the first triple and an object in the second; the
        // unified dictionary must mint it exactly ONE id (no more, no less).
        let dataset = build_dataset(&[
            (iri("x"), iri("p"), iri("o1")),
            (iri("s1"), iri("p"), iri("x")),
        ]);
        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("opens");
        // Distinct values: x, p, o1, s1 -> 4 ids total.
        assert_eq!(dict.n_terms(), 4);
        let id = dict.id_by_value(&iri("x")).expect("present");
        assert_eq!(dict.term_value(id), iri("x"));
    }

    #[test]
    fn predicate_that_is_also_object_shares_one_unified_id() {
        // "p" is used as a predicate in one triple and as an object in another.
        // Under the earlier split-id-space design this minted TWO distinct ids;
        // the unified design must mint exactly ONE, resolvable via either
        // lookup method.
        let dataset = build_dataset(&[
            (iri("s"), iri("p"), iri("o")),
            (iri("s2"), iri("about"), iri("p")),
        ]);
        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("opens");
        let via_id_by_value = dict.id_by_value(&iri("p")).expect("present");
        let via_predicate_id_by_value = dict
            .predicate_id_by_value(&iri("p"))
            .expect("present via the predicate-role lookup too");
        assert_eq!(
            via_id_by_value, via_predicate_id_by_value,
            "id_by_value and predicate_id_by_value must agree: one unified id space"
        );
        assert_eq!(dict.term_value(via_id_by_value), iri("p"));
    }

    #[test]
    fn pure_predicate_resolves_via_id_by_value() {
        // "q" is used ONLY as a predicate, never a subject or object. Under the
        // earlier split-id-space design `id_by_value` returned `None` for it —
        // exactly the seam-breaking bug this fix eliminates.
        let dataset = build_dataset(&[(iri("s"), iri("q"), iri("o"))]);
        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("opens");
        let id = dict
            .id_by_value(&iri("q"))
            .expect("a pure predicate must resolve via id_by_value");
        assert_eq!(
            dict.predicate_id_by_value(&iri("q")),
            Some(id),
            "predicate_id_by_value must agree with id_by_value"
        );
        assert_eq!(dict.term_value(id), iri("q"));
    }

    #[test]
    fn graph_only_term_gets_a_unified_id() {
        // "g" appears ONLY as a quad's graph-name slot — never as a subject,
        // predicate, or object — so it must still mint a unified id and round-trip
        // via `id_by_value`/`term_value`, agreeing with `predicate_id_by_value`.
        let mut b = RdfDatasetBuilder::new();
        let s = intern_value(&mut b, &iri("s"));
        let p = intern_value(&mut b, &iri("p"));
        let o = intern_value(&mut b, &iri("o"));
        let g = intern_value(&mut b, &iri("g"));
        b.push_quad(s, p, o, Some(g));
        let dataset = b.freeze().expect("valid dataset");

        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("opens");
        assert_eq!(dict.n_terms(), 4, "s, p, o, and the graph-only g");
        let id = dict
            .id_by_value(&iri("g"))
            .expect("graph-name term present");
        assert_eq!(dict.term_value(id), iri("g"));
        assert_eq!(dict.predicate_id_by_value(&iri("g")), Some(id));
    }

    #[test]
    fn graph_name_that_is_also_subject_keeps_its_existing_id() {
        // "g" names the graph of the second quad AND is the subject of the first
        // quad: it must get exactly ONE unified id, not a second duplicate entry.
        let mut b = RdfDatasetBuilder::new();
        let g = intern_value(&mut b, &iri("g"));
        let p = intern_value(&mut b, &iri("p"));
        let o1 = intern_value(&mut b, &iri("o1"));
        let s2 = intern_value(&mut b, &iri("s2"));
        let o2 = intern_value(&mut b, &iri("o2"));
        b.push_quad(g, p, o1, None);
        b.push_quad(s2, p, o2, Some(g));
        let dataset = b.freeze().expect("valid dataset");

        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("opens");
        // Distinct values: g, p, o1, s2, o2 -> 5 ids total, NOT 6 (which a
        // duplicate graph-name entry would produce).
        assert_eq!(dict.n_terms(), 5);
        let id = dict
            .id_by_value(&iri("g"))
            .expect("present via its subject role");
        assert_eq!(dict.term_value(id), iri("g"));
    }

    #[test]
    fn reifier_only_term_gets_unified_id_via_side_table_closure() {
        // The reifier resource and the reified triple-term both hold NO base-quad
        // role: the reifier binds `<< s p o >>` PURELY as a side-table row (no
        // base quad is ever pushed — reification lives entirely in the side table).
        // The side-table term closure must still fold both into the dictionary
        // and round-trip them.
        let mut b = RdfDatasetBuilder::new();
        let s = intern_value(&mut b, &iri("s"));
        let p = intern_value(&mut b, &iri("p"));
        let o = intern_value(&mut b, &iri("o"));
        let triple = b.intern_triple(s, p, o);
        let reifier = intern_value(&mut b, &iri("r"));
        b.push_reifier(reifier, triple);
        let dataset = b.freeze().expect("valid dataset");
        assert_eq!(dataset.quad_count(), 0, "reification is side-table only");

        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("opens");
        let reifier_id = dict.id_by_value(&iri("r")).expect("reifier term present");
        assert_eq!(dict.term_value(reifier_id), iri("r"));
        let triple_value = TermValue::Triple {
            s: Box::new(iri("s")),
            p: Box::new(iri("p")),
            o: Box::new(iri("o")),
        };
        let triple_id = dict
            .id_by_value(&triple_value)
            .expect("triple-term present via the reifier row");
        assert_eq!(dict.term_value(triple_id), triple_value);
    }

    #[test]
    fn annotation_only_predicate_and_object_get_unified_ids_via_side_table_closure() {
        // The annotation's predicate and object appear ONLY in the annotation
        // side-table (never a base quad's subject/predicate/object).
        let mut b = RdfDatasetBuilder::new();
        let s = intern_value(&mut b, &iri("s"));
        let p = intern_value(&mut b, &iri("p"));
        let o = intern_value(&mut b, &iri("o"));
        let triple = b.intern_triple(s, p, o);
        let reifier = intern_value(&mut b, &iri("r"));
        b.push_reifier(reifier, triple);
        let ap = intern_value(&mut b, &iri("confidence"));
        let ao = intern_value(&mut b, &TermValue::simple_literal("0.9"));
        b.push_annotation(reifier, ap, ao);
        let dataset = b.freeze().expect("valid dataset");

        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("opens");
        let ao_id = dict
            .id_by_value(&TermValue::simple_literal("0.9"))
            .expect("annotation object present via the side table");
        assert_eq!(dict.term_value(ao_id), TermValue::simple_literal("0.9"));
        let ap_id = dict
            .id_by_value(&iri("confidence"))
            .expect("annotation predicate present via the side table");
        assert_eq!(dict.term_value(ap_id), iri("confidence"));
        // "confidence" is used as this annotation's predicate and NOWHERE else,
        // so it is a "pure predicate" too — must resolve identically both ways.
        assert_eq!(dict.predicate_id_by_value(&iri("confidence")), Some(ap_id));
    }

    #[test]
    fn rdf_reifies_predicate_gets_unified_id_when_reifiers_present() {
        let mut b = RdfDatasetBuilder::new();
        let s = intern_value(&mut b, &iri("s"));
        let p = intern_value(&mut b, &iri("p"));
        let o = intern_value(&mut b, &iri("o"));
        let triple = b.intern_triple(s, p, o);
        let reifier = intern_value(&mut b, &iri("r"));
        // Mirror the ingest path: `rdf:reifies` is interned even though it never
        // appears in any base quad or side-table row tuple directly.
        b.intern_iri(RDF_REIFIES);
        b.push_reifier(reifier, triple);
        let dataset = b.freeze().expect("valid dataset");

        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("opens");
        let id = dict
            .id_by_value(&TermValue::Iri(RDF_REIFIES.to_owned()))
            .expect("rdf:reifies present when reifiers are non-empty");
        assert_eq!(dict.term_value(id), TermValue::Iri(RDF_REIFIES.to_owned()));
        // `rdf:reifies` is itself used purely as a predicate (via `side.rs`'s
        // synthesized reifier rows) — the predicate-role lookup must agree.
        assert_eq!(
            dict.predicate_id_by_value(&TermValue::Iri(RDF_REIFIES.to_owned())),
            Some(id)
        );
    }

    #[test]
    fn rdf_reifies_absent_when_no_reifiers() {
        let dataset = build_dataset(&[(iri("s"), iri("p"), iri("o"))]);
        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("opens");
        assert_eq!(
            dict.id_by_value(&TermValue::Iri(RDF_REIFIES.to_owned())),
            None
        );
    }

    #[test]
    fn absent_value_yields_none() {
        let dataset = build_dataset(&[(iri("s"), iri("p"), iri("o"))]);
        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("opens");
        assert_eq!(dict.id_by_value(&iri("never-interned")), None);
        assert_eq!(dict.predicate_id_by_value(&iri("never-interned")), None);
    }

    #[test]
    fn dict_bytes_round_trip_via_encoded_dict() {
        let dataset = build_dataset(&[
            (iri("s"), iri("p"), iri("o")),
            (iri("s"), iri("p2"), TermValue::simple_literal("v")),
        ]);
        let encoded = PackDict::encode(&dataset);
        let bytes = encoded.to_bytes();
        let reparsed = EncodedDict::from_bytes(&bytes).expect("parses");
        assert_eq!(reparsed.n_terms(), encoded.n_terms());
        let dict = reparsed.decode().expect("decodes");
        assert_eq!(dict.n_terms(), encoded.n_terms());
    }

    // -- Proptest: full generative round trip ------------------------------------

    fn arb_iri_value() -> impl Strategy<Value = TermValue> {
        (0u32..10).prop_map(|i| TermValue::iri(format!("http://example.org/i{i}")))
    }

    fn arb_blank_value() -> impl Strategy<Value = TermValue> {
        ("[a-z]{1,4}", 0u32..4).prop_map(|(label, scope)| TermValue::Blank {
            label,
            scope: BlankScope(scope),
        })
    }

    fn arb_literal_value() -> BoxedStrategy<TermValue> {
        let datatypes = vec![
            "http://www.w3.org/2001/XMLSchema#string".to_string(),
            "http://www.w3.org/2001/XMLSchema#integer".to_string(),
            "http://example.org/customDatatype".to_string(),
        ];
        let languages = vec!["en".to_string(), "fr".to_string(), "de-ch".to_string()];
        (
            "[a-zA-Z0-9 ]{0,8}",
            prop::sample::select(datatypes),
            prop::option::of(prop::sample::select(languages)),
        )
            .prop_flat_map(|(lexical_form, datatype, language)| {
                let dir_strategy: BoxedStrategy<Option<RdfTextDirection>> = if language.is_some() {
                    prop::option::of(prop_oneof![
                        Just(RdfTextDirection::Ltr),
                        Just(RdfTextDirection::Rtl),
                    ])
                    .boxed()
                } else {
                    Just(None).boxed()
                };
                dir_strategy.prop_map(move |direction| TermValue::Literal {
                    lexical_form: lexical_form.clone(),
                    datatype: datatype.clone(),
                    language: language.clone(),
                    direction,
                })
            })
            .boxed()
    }

    fn arb_object_value() -> BoxedStrategy<TermValue> {
        let leaf = prop_oneof![arb_iri_value(), arb_blank_value(), arb_literal_value()];
        leaf.prop_recursive(3, 20, 3, |inner| {
            (
                prop_oneof![arb_iri_value(), arb_blank_value()],
                arb_iri_value(),
                inner,
            )
                .prop_map(|(s, p, o)| TermValue::Triple {
                    s: Box::new(s),
                    p: Box::new(p),
                    o: Box::new(o),
                })
        })
        .boxed()
    }

    fn arb_quad() -> impl Strategy<Value = (TermValue, TermValue, TermValue)> {
        (
            prop_oneof![arb_iri_value(), arb_blank_value()],
            arb_iri_value(),
            arb_object_value(),
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn proptest_pack_dict_round_trips(
            quads in prop::collection::vec(arb_quad(), 1..24)
        ) {
            let dataset = build_dataset(&quads);

            let encoded = PackDict::encode(&dataset);
            let bytes = encoded.to_bytes();
            let dict = PackDict::open(&bytes).expect("round trip parses");

            prop_assert_eq!(dict.n_terms(), encoded.n_terms());

            // Ground truth: every value the dataset itself ever interned.
            let truth: HashSet<TermValue> = (0..dataset.term_count())
                .map(|i| dataset.term_value(TermId::from_index(i as u32)))
                .collect();

            for id in 1..=dict.n_terms() {
                let value = dict.term_value(id);
                // Fidelity: every resolved value must be one the dataset actually
                // interned (no corruption, no drift).
                prop_assert!(truth.contains(&value), "resolved value {value:?} not in dataset truth set");

                // The single unified id space round-trips this id exactly, via
                // EITHER lookup method — they must always agree.
                prop_assert_eq!(dict.id_by_value(&value), Some(id));
                prop_assert_eq!(dict.predicate_id_by_value(&value), Some(id));
            }

            // A freshly-minted, never-interned value is absent.
            let absent = TermValue::iri("http://example.org/definitely-not-present-in-this-dict");
            prop_assert_eq!(dict.id_by_value(&absent), None);
            prop_assert_eq!(dict.predicate_id_by_value(&absent), None);
        }
    }
}
