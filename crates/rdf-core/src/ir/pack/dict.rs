// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A four-section HDT-style value dictionary (Task 2 of the succinct-pack-codec
//! feature): one unified [`PackTermId`](self)-space (a plain `u64`, 1-based) per
//! distinct [`TermValue`] scanned from an [`RdfDataset`], PFC-compressed on disk and
//! decoded into OWNED structures at [`PackDict::open`].
//!
//! # Id layout
//!
//! Terms are scanned by ROLE from the dataset's base quads (subject / predicate /
//! object; RDF 1.2 side-table-only terms are out of scope — Task 4 owns that
//! section) and partitioned into four disjoint groups:
//!
//! - **shared** — appears as both a subject and an object somewhere (`S ∩ O`).
//! - **subject_only** — subject, never object.
//! - **object_only** — object, never subject. This section ALSO absorbs any
//!   "auxiliary" value that is referenced only structurally — a literal's datatype
//!   IRI, a triple term's `s`/`p`/`o` component, or a quad's named-graph term (a
//!   value used ONLY in a quad's `g` slot, never as an S/P/O) — and has no S/P/O
//!   role of its own (see [`encode`](PackDict::encode)'s doc for the exact closure
//!   rule). A graph-name value that ALSO plays an S/P/O role keeps its existing id
//!   from that role; it is never duplicated. This is how [`PackDict::id_by_value`]
//!   resolves a `GraphMatch::Named` graph-name constant to a unified id (Task 3).
//! - **predicates** — appears as a predicate. A term that is BOTH a predicate and a
//!   subject/object gets a separate entry in the predicate section IN ADDITION to
//!   its shared/subject_only/object_only entry — HDT keeps the predicate id space
//!   independent, so the same value can carry two distinct unified ids.
//!
//! Within a section, terms sort in the canonical [`TermValue`] order (`Ord`); unified
//! ids are then assigned `1..=N` by concatenating the four sections in the FIXED
//! order `[shared, subject_only, object_only, predicates]`.
//!
//! # Lookup rule (id_by_value vs. predicate_id_by_value)
//!
//! A value may hold up to two unified ids (one from `{shared, subject_only,
//! object_only}`, one from `predicates`). [`PackDict::id_by_value`] searches ONLY the
//! three non-predicate sections (in that priority order — but they are mutually
//! exclusive by construction, so there is never more than one hit) and is the
//! lookup an evaluator uses for a pattern's subject/object constant.
//! [`PackDict::predicate_id_by_value`] searches ONLY the predicate section, for a
//! pattern's predicate constant. This mirrors how a value's REFERENCE from inside an
//! encoded record (a literal's datatype id, a triple term's component ids) is
//! resolved at encode time: prefer a non-predicate id, fall back to the predicate id
//! only if the value has no non-predicate role.
//!
//! # Seam for later tasks
//!
//! This module works entirely in raw `u64` unified ids (`PackTermId` is a plain type
//! alias here). Task 6's `PackView`/`ViewTermId` newtype wraps these `u64`s; Task 3's
//! bitmap-triples layer uses the `*_role_to_unified` / `unified_to_*_role`
//! conversions below to translate between its dense per-role numbering and this
//! dictionary's unified id space via pure range arithmetic (no stored permutation).

use std::cmp::Ordering;
use std::fmt;

use crate::hash::{FastMap, FastSet, IdSet};
use crate::ir::term::{StrRange, arena_str};
use crate::{BlankScope, RdfDataset, RdfTextDirection, TermRef, TermValue};

use super::bits::{IntVector, IntVectorRef, PackBitsError, bits_for, read_varint, write_varint};

/// The unified term-identity space this module mints: a plain, 1-based `u64` (id `0`
/// is never assigned). A pure type alias, not a newtype — Task 6 wraps this in a real
/// [`ViewTermId`](crate::ViewTermId) newtype once the outer `PackView` seam lands (see
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
// Per-section PFC encode/decode.
// ---------------------------------------------------------------------------

/// PFC-encode one section's already-sorted term values into a self-contained byte
/// block: `u64 term_count`, `u64 bucket_count`, a serialized [`IntVector`] of
/// per-bucket BYTE offsets into the bucket-data stream (so a bucket can be located
/// without scanning from the start), then the bucket-data stream itself.
fn encode_section(values: &[TermValue], value_to_id: &FastMap<TermValue, PackTermId>) -> Vec<u8> {
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

/// Decode one PFC section, pushing each entry into `dict` in unified-id order (the
/// caller is responsible for calling this once per section, in the fixed
/// `[shared, subject_only, object_only, predicates]` order, so unified ids come out
/// `1..=n_terms`). Returns the section's term count.
fn decode_section(bytes: &[u8], dict: &mut PackDict) -> Result<u64, PackDictError> {
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

/// The output of [`PackDict::encode`]: the four PFC-encoded section byte blocks plus
/// their term counts. Self-contained and independently round-trippable via
/// [`to_bytes`](Self::to_bytes)/[`from_bytes`](Self::from_bytes) — a later container
/// format (Task 5) frames these bytes alongside its other blocks, or a caller can
/// treat an `EncodedDict` as a standalone dictionary file.
#[derive(Debug, Clone)]
pub struct EncodedDict {
    /// The number of terms in the `shared` (subject ∩ object) section.
    pub n_shared: u64,
    /// The number of terms in the `subject_only` section.
    pub n_subject_only: u64,
    /// The number of terms in the `object_only` section (including closure-added
    /// auxiliary values — see the [module docs](self)).
    pub n_object_only: u64,
    /// The number of terms in the `predicates` section.
    pub n_predicates: u64,
    shared_bytes: Vec<u8>,
    subject_only_bytes: Vec<u8>,
    object_only_bytes: Vec<u8>,
    predicates_bytes: Vec<u8>,
}

impl EncodedDict {
    /// The total number of unified ids this dictionary mints (`n_shared +
    /// n_subject_only + n_object_only + n_predicates`).
    #[must_use]
    pub fn n_terms(&self) -> u64 {
        self.n_shared + self.n_subject_only + self.n_object_only + self.n_predicates
    }

    /// Serialize to the self-contained, versioned on-disk form: a 1-byte version tag,
    /// then the four sections in fixed order, each as an 8-byte LE length prefix
    /// followed by that many bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(DICT_FORMAT_VERSION);
        for section in [
            &self.shared_bytes,
            &self.subject_only_bytes,
            &self.object_only_bytes,
            &self.predicates_bytes,
        ] {
            out.extend_from_slice(&(section.len() as u64).to_le_bytes());
            out.extend_from_slice(section);
        }
        out
    }

    /// Parse [`to_bytes`](Self::to_bytes)'s output.
    ///
    /// # Errors
    ///
    /// [`PackDictError::Truncated`]/[`PackDictError::Malformed`] on a short buffer,
    /// an unsupported version tag, or a section whose own header is inconsistent.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PackDictError> {
        let version = *bytes.first().ok_or(PackDictError::Truncated {
            needed: 1,
            found: 0,
        })?;
        if version != DICT_FORMAT_VERSION {
            return Err(PackDictError::Malformed("dict: unsupported format version"));
        }
        let mut pos = 1usize;
        let mut sections: [Vec<u8>; 4] = Default::default();
        for section in &mut sections {
            let len = read_u64_header(bytes, &mut pos)? as usize;
            let end = pos + len;
            let slice = bytes.get(pos..end).ok_or(PackDictError::Truncated {
                needed: end,
                found: bytes.len(),
            })?;
            *section = slice.to_vec();
            pos = end;
        }
        let [
            shared_bytes,
            subject_only_bytes,
            object_only_bytes,
            predicates_bytes,
        ] = sections;
        let n_shared = peek_term_count(&shared_bytes)?;
        let n_subject_only = peek_term_count(&subject_only_bytes)?;
        let n_object_only = peek_term_count(&object_only_bytes)?;
        let n_predicates = peek_term_count(&predicates_bytes)?;
        Ok(Self {
            n_shared,
            n_subject_only,
            n_object_only,
            n_predicates,
            shared_bytes,
            subject_only_bytes,
            object_only_bytes,
            predicates_bytes,
        })
    }

    /// Decode all four PFC sections into an owned [`PackDict`], ready for
    /// [`resolve`](PackDict::resolve)/[`id_by_value`](PackDict::id_by_value) queries.
    ///
    /// # Errors
    ///
    /// [`PackDictError`] if a section is malformed, truncated, or contains an
    /// out-of-range id reference.
    pub fn decode(&self) -> Result<PackDict, PackDictError> {
        let mut dict = PackDict {
            n_shared: 0,
            n_subject_only: 0,
            n_object_only: 0,
            n_predicates: 0,
            arena: Vec::new(),
            entries: Vec::new(),
        };
        dict.n_shared = decode_section(&self.shared_bytes, &mut dict)?;
        dict.n_subject_only = decode_section(&self.subject_only_bytes, &mut dict)?;
        dict.n_object_only = decode_section(&self.object_only_bytes, &mut dict)?;
        dict.n_predicates = decode_section(&self.predicates_bytes, &mut dict)?;
        dict.validate_references()?;
        Ok(dict)
    }
}

/// Peek a PFC section's own leading `u64 term_count` header field without decoding
/// its records.
fn peek_term_count(section_bytes: &[u8]) -> Result<u64, PackDictError> {
    let mut pos = 0usize;
    read_u64_header(section_bytes, &mut pos)
}

// ---------------------------------------------------------------------------
// PackDict — the owned, decoded, query-ready dictionary.
// ---------------------------------------------------------------------------

/// The decoded, query-ready value dictionary: one unified [`PackTermId`] per
/// distinct [`TermValue`], resolved from an owned byte arena + entry table built by
/// [`EncodedDict::decode`]/[`PackDict::open`]. See the [module docs](self) for the
/// four-section id layout and the `id_by_value` vs. `predicate_id_by_value` lookup
/// rule.
#[derive(Debug, Clone)]
pub struct PackDict {
    n_shared: u64,
    n_subject_only: u64,
    n_object_only: u64,
    n_predicates: u64,
    /// The byte arena owning every interned string ONCE; entries hold ranges.
    arena: Vec<u8>,
    /// Dense table of decoded entries; unified id `i` (1-based) lives at
    /// `entries[i - 1]`.
    entries: Vec<DictEntry>,
}

impl PackDict {
    /// Scan `dataset`'s base quads and build the four-section dictionary (see the
    /// [module docs](self) for the exact section rule), returning the PFC-encoded,
    /// not-yet-parsed [`EncodedDict`].
    ///
    /// # The auxiliary-value closure
    ///
    /// A literal's datatype IRI and a triple term's `s`/`p`/`o` components must each
    /// hold their OWN unified id (records reference them by id, not by embedded
    /// value), but they do not necessarily appear as a subject/predicate/object of
    /// any base quad (e.g. `xsd:integer` as a literal's datatype is rarely itself a
    /// triple's subject or object). After the base S/P/O role scan, this method
    /// computes the closure of every such auxiliary reference, transitively (an
    /// auxiliary value can itself be a literal or a nested triple term), and folds
    /// any NOT already present in one of the four sections into `object_only`. A
    /// value already present anywhere keeps its existing id and is never duplicated.
    #[must_use]
    pub fn encode(dataset: &RdfDataset) -> EncodedDict {
        // Step 1: base S/P/O role sets, by TermId (cheap membership tests), from the
        // base quads only. Also collect the distinct set of graph-name TermIds (a
        // quad's `g` slot) — see the graph-name amendment in Step 2 below.
        let mut subj_ids: IdSet = IdSet::default();
        let mut pred_ids: IdSet = IdSet::default();
        let mut obj_ids: IdSet = IdSet::default();
        let mut graph_ids: IdSet = IdSet::default();
        for q in dataset.quads() {
            subj_ids.insert(q.s);
            pred_ids.insert(q.p);
            obj_ids.insert(q.o);
            if let Some(g) = q.g {
                graph_ids.insert(g);
            }
        }

        let mut shared: Vec<TermValue> = subj_ids
            .intersection(&obj_ids)
            .map(|&id| dataset.term_value(id))
            .collect();
        let mut subject_only: Vec<TermValue> = subj_ids
            .difference(&obj_ids)
            .map(|&id| dataset.term_value(id))
            .collect();
        let mut object_only: Vec<TermValue> = obj_ids
            .difference(&subj_ids)
            .map(|&id| dataset.term_value(id))
            .collect();
        let mut predicates: Vec<TermValue> =
            pred_ids.iter().map(|&id| dataset.term_value(id)).collect();
        // Sorted (not merely deterministic-order) so the graph-name fold-in below can
        // walk it in canonical order like every other base-role Vec.
        let mut graph_values: Vec<TermValue> =
            graph_ids.iter().map(|&id| dataset.term_value(id)).collect();

        shared.sort();
        subject_only.sort();
        object_only.sort();
        predicates.sort();
        graph_values.sort();

        // Step 2: closure over auxiliary references (literal datatypes, triple
        // components) not already covered by a base role, PLUS graph-name terms (a
        // quad's `g` slot) not already covered by a base S/P/O role — see the
        // "Graph-name terms" note in the [module docs](self). A graph-name value
        // already present under some role keeps its existing id (no new entry); one
        // with no other role is folded into `object_only`, using the exact same
        // worklist/dedup machinery as the literal-datatype/triple-component closure
        // (so a spec-illegal nested graph-name value, were one ever produced, would
        // still be handled correctly). Deterministic regardless of hash-set
        // iteration order: every worklist source here is an already-sorted Vec, and
        // the result is re-sorted before use — no hash-iteration order ever reaches
        // the output (byte-determinism discipline).
        let mut present: FastSet<TermValue> = FastSet::default();
        let mut queue: Vec<TermValue> = Vec::new();
        for v in shared
            .iter()
            .chain(subject_only.iter())
            .chain(object_only.iter())
            .chain(predicates.iter())
        {
            present.insert(v.clone());
            queue.push(v.clone());
        }
        let mut extra: Vec<TermValue> = Vec::new();
        for v in &graph_values {
            if present.insert(v.clone()) {
                extra.push(v.clone());
                queue.push(v.clone());
            }
        }
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
        extra.sort();
        object_only.extend(extra);
        object_only.sort();

        // Step 3: assign unified ids 1..=N in fixed section order, and build the
        // encode-time reference map (first-section-wins: shared > subject_only >
        // object_only > predicates — the SAME priority id_by_value's lookup uses).
        let mut value_to_id: FastMap<TermValue, PackTermId> = FastMap::default();
        let mut next_id: PackTermId = 1;
        for section in [&shared, &subject_only, &object_only, &predicates] {
            for v in section {
                let id = next_id;
                next_id += 1;
                value_to_id.entry(v.clone()).or_insert(id);
            }
        }

        EncodedDict {
            n_shared: shared.len() as u64,
            n_subject_only: subject_only.len() as u64,
            n_object_only: object_only.len() as u64,
            n_predicates: predicates.len() as u64,
            shared_bytes: encode_section(&shared, &value_to_id),
            subject_only_bytes: encode_section(&subject_only, &value_to_id),
            object_only_bytes: encode_section(&object_only, &value_to_id),
            predicates_bytes: encode_section(&predicates, &value_to_id),
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

    /// The number of terms in the `shared` section.
    #[must_use]
    pub fn n_shared(&self) -> u64 {
        self.n_shared
    }

    /// The number of terms in the `subject_only` section.
    #[must_use]
    pub fn n_subject_only(&self) -> u64 {
        self.n_subject_only
    }

    /// The number of terms in the `object_only` section (including closure-added
    /// auxiliary values).
    #[must_use]
    pub fn n_object_only(&self) -> u64 {
        self.n_object_only
    }

    /// The number of terms in the `predicates` section.
    #[must_use]
    pub fn n_predicates(&self) -> u64 {
        self.n_predicates
    }

    /// The total number of unified ids this dictionary mints.
    #[must_use]
    pub fn n_terms(&self) -> u64 {
        self.n_shared + self.n_subject_only + self.n_object_only + self.n_predicates
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

    /// Binary-search a contiguous unified-id range `[first_id, first_id + count)` for
    /// `value`, using [`term_value`](Self::term_value) as the (canonically-ordered)
    /// comparator. `O(log count)` calls to `term_value`, each `O(term depth)`.
    fn search_section(
        &self,
        first_id: PackTermId,
        count: u64,
        value: &TermValue,
    ) -> Option<PackTermId> {
        let mut lo = 0u64;
        let mut hi = count;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let id = first_id + mid;
            match self.term_value(id).cmp(value) {
                Ordering::Less => lo = mid + 1,
                Ordering::Greater => hi = mid,
                Ordering::Equal => return Some(id),
            }
        }
        None
    }

    /// The unified id of `value`, searching ONLY the non-predicate sections (`shared`,
    /// `subject_only`, `object_only` — mutually exclusive by construction, so at most
    /// one search hits). `None` if `value` has no non-predicate role in this
    /// dictionary (it may still have a predicate-only id — see
    /// [`predicate_id_by_value`](Self::predicate_id_by_value)). This is the lookup an
    /// evaluator uses for a pattern's subject/object constant; see the
    /// [module docs](self) for the full rule.
    #[must_use]
    pub fn id_by_value(&self, value: &TermValue) -> Option<PackTermId> {
        self.search_section(1, self.n_shared, value)
            .or_else(|| self.search_section(self.n_shared + 1, self.n_subject_only, value))
            .or_else(|| {
                self.search_section(
                    self.n_shared + self.n_subject_only + 1,
                    self.n_object_only,
                    value,
                )
            })
    }

    /// The unified id of `value` in the `predicates` section ONLY. `None` if `value`
    /// is never used as a predicate in this dictionary. This is the lookup an
    /// evaluator uses for a pattern's predicate constant; see the
    /// [module docs](self) for the full rule.
    #[must_use]
    pub fn predicate_id_by_value(&self, value: &TermValue) -> Option<PackTermId> {
        self.search_section(
            self.n_shared + self.n_subject_only + self.n_object_only + 1,
            self.n_predicates,
            value,
        )
    }

    /// Validate that every decoded id reference (a literal's datatype, a triple
    /// term's `s`/`p`/`o`) falls within `1..=n_terms()`. Called once by
    /// [`EncodedDict::decode`] after all four sections are decoded (id ranges are
    /// only fully known once the whole dictionary is assembled).
    fn validate_references(&self) -> Result<(), PackDictError> {
        let n = self.n_terms();
        let in_range = |id: PackTermId| id >= 1 && id <= n;
        for entry in &self.entries {
            match entry {
                DictEntry::Literal { datatype, .. } if !in_range(*datatype) => {
                    return Err(PackDictError::Malformed(
                        "dict: literal datatype id out of range",
                    ));
                }
                DictEntry::Triple { s, p, o }
                    if !in_range(*s) || !in_range(*p) || !in_range(*o) =>
                {
                    return Err(PackDictError::Malformed(
                        "dict: triple component id out of range",
                    ));
                }
                _ => {}
            }
        }
        Ok(())
    }

    // -- Role-id <-> unified-id conversions (pure range arithmetic; Task 3) -------
    //
    // Dense per-role numbering is 0-based; unified ids are 1-based. See the
    // [module docs](self) for why the object-role range splits into two blocks
    // (the shared prefix, then the object_only block after subject_only).

    /// Subject-role dense id `r` (`0..n_shared() + n_subject_only()`) → unified id.
    ///
    /// # Panics
    ///
    /// Debug-asserts `r` is in range.
    #[must_use]
    pub fn subject_role_to_unified(&self, r: u64) -> PackTermId {
        debug_assert!(r < self.n_shared + self.n_subject_only);
        r + 1
    }

    /// The inverse of [`subject_role_to_unified`](Self::subject_role_to_unified):
    /// `None` if `u` does not address a subject-role entry (i.e. it is a
    /// `predicates`-only or `object_only`-only unified id).
    #[must_use]
    pub fn unified_to_subject_role(&self, u: PackTermId) -> Option<u64> {
        if u >= 1 && u <= self.n_shared + self.n_subject_only {
            Some(u - 1)
        } else {
            None
        }
    }

    /// Object-role dense id `r` (`0..n_shared() + n_object_only()`) → unified id.
    /// `r < n_shared()` addresses the shared prefix (unified `r + 1`); otherwise it
    /// addresses the `object_only` block, which sits AFTER `subject_only` in the
    /// unified space, so the id jumps over it.
    ///
    /// # Panics
    ///
    /// Debug-asserts `r` is in range.
    #[must_use]
    pub fn object_role_to_unified(&self, r: u64) -> PackTermId {
        debug_assert!(r < self.n_shared + self.n_object_only);
        if r < self.n_shared {
            r + 1
        } else {
            self.n_shared + self.n_subject_only + (r - self.n_shared) + 1
        }
    }

    /// The inverse of [`object_role_to_unified`](Self::object_role_to_unified):
    /// `None` if `u` does not address an object-role entry (i.e. it is a
    /// `subject_only`-only or `predicates`-only unified id).
    #[must_use]
    pub fn unified_to_object_role(&self, u: PackTermId) -> Option<u64> {
        if u >= 1 && u <= self.n_shared {
            Some(u - 1)
        } else if u > self.n_shared + self.n_subject_only
            && u <= self.n_shared + self.n_subject_only + self.n_object_only
        {
            Some(self.n_shared + (u - (self.n_shared + self.n_subject_only) - 1))
        } else {
            None
        }
    }

    /// Predicate-role dense id `r` (`0..n_predicates()`) → unified id.
    ///
    /// # Panics
    ///
    /// Debug-asserts `r` is in range.
    #[must_use]
    pub fn predicate_role_to_unified(&self, r: u64) -> PackTermId {
        debug_assert!(r < self.n_predicates);
        self.n_shared + self.n_subject_only + self.n_object_only + r + 1
    }

    /// The inverse of [`predicate_role_to_unified`](Self::predicate_role_to_unified):
    /// `None` if `u` does not address a predicate-role entry.
    #[must_use]
    pub fn unified_to_predicate_role(&self, u: PackTermId) -> Option<u64> {
        let base = self.n_shared + self.n_subject_only + self.n_object_only;
        if u > base && u <= base + self.n_predicates {
            Some(u - base - 1)
        } else {
            None
        }
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
        let id = dict.id_by_value(&iri("s")).expect("subject_only present");
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

    // -- Section boundary arithmetic edge cases ----------------------------------

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
    fn single_subject_only_term() {
        // "s" is never an object anywhere, so it is subject_only, not shared.
        let dataset = build_dataset(&[(iri("s"), iri("p"), iri("o"))]);
        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("opens");
        assert_eq!(dict.n_shared(), 0);
        assert_eq!(dict.n_subject_only(), 1);
        assert_eq!(dict.n_object_only(), 1);
        assert_eq!(dict.n_predicates(), 1);
        let id = dict.id_by_value(&iri("s")).expect("present");
        // subject_only is the second section (after the empty shared section), so
        // its first (only) entry is unified id 1.
        assert_eq!(id, 1);
    }

    #[test]
    fn shared_term_appears_once_in_shared_section() {
        // "x" is a subject in the first triple and an object in the second: shared.
        let dataset = build_dataset(&[
            (iri("x"), iri("p"), iri("o1")),
            (iri("s1"), iri("p"), iri("x")),
        ]);
        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("opens");
        assert_eq!(dict.n_shared(), 1);
        let id = dict.id_by_value(&iri("x")).expect("present");
        assert_eq!(id, 1, "shared is the first section");
    }

    #[test]
    fn predicate_that_is_also_object_gets_two_ids() {
        // "p" is used as a predicate in one triple and as an object in another: it
        // must get a unified id from BOTH the predicates section and the
        // object_only section.
        let dataset = build_dataset(&[
            (iri("s"), iri("p"), iri("o")),
            (iri("s2"), iri("about"), iri("p")),
        ]);
        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("opens");
        let non_predicate_id = dict.id_by_value(&iri("p")).expect("object-role id present");
        let predicate_id = dict
            .predicate_id_by_value(&iri("p"))
            .expect("predicate-role id present");
        assert_ne!(non_predicate_id, predicate_id);
        assert_eq!(dict.term_value(non_predicate_id), iri("p"));
        assert_eq!(dict.term_value(predicate_id), iri("p"));
    }

    #[test]
    fn graph_only_term_gets_unified_id_in_object_only() {
        // "g" appears ONLY as a quad's graph-name slot — never as a subject,
        // predicate, or object — so the graph-name amendment must still mint it a
        // unified id (folded into `object_only`) and round-trip it via
        // `id_by_value`/`term_value`, with no spurious predicate-role id.
        let mut b = RdfDatasetBuilder::new();
        let s = intern_value(&mut b, &iri("s"));
        let p = intern_value(&mut b, &iri("p"));
        let o = intern_value(&mut b, &iri("o"));
        let g = intern_value(&mut b, &iri("g"));
        b.push_quad(s, p, o, Some(g));
        let dataset = b.freeze().expect("valid dataset");

        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("opens");
        assert_eq!(dict.n_object_only(), 2, "\"o\" and the graph-only \"g\"");
        let id = dict
            .id_by_value(&iri("g"))
            .expect("graph-name term present");
        assert_eq!(dict.term_value(id), iri("g"));
        // "g" plays no predicate role anywhere, so it must not hold a predicate id.
        assert_eq!(dict.predicate_id_by_value(&iri("g")), None);
    }

    #[test]
    fn graph_name_that_is_also_subject_keeps_its_existing_id() {
        // "g" names the graph of the second quad AND is the subject of the first
        // quad: it must get exactly ONE non-predicate unified id (its
        // subject_only/shared id), not a second duplicate object_only entry.
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
        // "g" and "s2" are both subjects, never objects, so subject_only holds
        // exactly those two — the graph-name fold-in must not have ALSO placed "g"
        // in object_only as a duplicate entry.
        assert_eq!(dict.n_subject_only(), 2);
        assert_eq!(dict.n_object_only(), 2, "\"o1\" and \"o2\" only");
        let id = dict
            .id_by_value(&iri("g"))
            .expect("present via its subject role");
        assert_eq!(dict.term_value(id), iri("g"));
    }

    #[test]
    fn absent_value_yields_none() {
        let dataset = build_dataset(&[(iri("s"), iri("p"), iri("o"))]);
        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("opens");
        assert_eq!(dict.id_by_value(&iri("never-interned")), None);
        assert_eq!(dict.predicate_id_by_value(&iri("never-interned")), None);
    }

    #[test]
    fn role_conversions_round_trip_and_reject_foreign_ranges() {
        let dataset = build_dataset(&[
            (iri("x"), iri("p"), iri("o1")), // x: subject
            (iri("s1"), iri("p"), iri("x")), // x: also object -> shared
            (iri("s2"), iri("p"), iri("only-object")),
            (iri("only-subject"), iri("p"), iri("o2")),
        ]);
        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("opens");

        for r in 0..dict.n_shared() + dict.n_subject_only() {
            let u = dict.subject_role_to_unified(r);
            assert_eq!(dict.unified_to_subject_role(u), Some(r));
        }
        for r in 0..dict.n_shared() + dict.n_object_only() {
            let u = dict.object_role_to_unified(r);
            assert_eq!(dict.unified_to_object_role(u), Some(r));
        }
        for r in 0..dict.n_predicates() {
            let u = dict.predicate_role_to_unified(r);
            assert_eq!(dict.unified_to_predicate_role(u), Some(r));
        }

        // A unified id that lives purely in subject_only must not answer as an
        // object-role id (unless it happens to also be < n_shared, which the fixture
        // avoids by construction: subject_only ids are all > n_shared).
        let subject_only_unified = dict.n_shared() + 1; // first subject_only id
        assert!(dict.unified_to_object_role(subject_only_unified).is_none());
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
            prop_assert_eq!(dict.n_shared(), encoded.n_shared);
            prop_assert_eq!(dict.n_subject_only(), encoded.n_subject_only);
            prop_assert_eq!(dict.n_object_only(), encoded.n_object_only);
            prop_assert_eq!(dict.n_predicates(), encoded.n_predicates);

            // Ground truth: every value the dataset itself ever interned.
            let truth: HashSet<TermValue> = (0..dataset.term_count())
                .map(|i| dataset.term_value(TermId::from_index(i as u32)))
                .collect();

            let non_predicate_bound = dict.n_shared() + dict.n_subject_only() + dict.n_object_only();
            for id in 1..=dict.n_terms() {
                let value = dict.term_value(id);
                // Fidelity: every resolved value must be one the dataset actually
                // interned (no corruption, no drift).
                prop_assert!(truth.contains(&value), "resolved value {value:?} not in dataset truth set");

                // The documented lookup rule round-trips this id exactly.
                let looked_up = if id <= non_predicate_bound {
                    dict.id_by_value(&value)
                } else {
                    dict.predicate_id_by_value(&value)
                };
                prop_assert_eq!(looked_up, Some(id));
            }

            // A freshly-minted, never-interned value is absent.
            let absent = TermValue::iri("http://example.org/definitely-not-present-in-this-dict");
            prop_assert_eq!(dict.id_by_value(&absent), None);
            prop_assert_eq!(dict.predicate_id_by_value(&absent), None);

            // Role-id <-> unified-id bijections over their full documented ranges.
            for r in 0..dict.n_shared() + dict.n_subject_only() {
                let u = dict.subject_role_to_unified(r);
                prop_assert_eq!(dict.unified_to_subject_role(u), Some(r));
            }
            for r in 0..dict.n_shared() + dict.n_object_only() {
                let u = dict.object_role_to_unified(r);
                prop_assert_eq!(dict.unified_to_object_role(u), Some(r));
            }
            for r in 0..dict.n_predicates() {
                let u = dict.predicate_role_to_unified(r);
                prop_assert_eq!(dict.unified_to_predicate_role(u), Some(r));
            }
        }
    }
}
