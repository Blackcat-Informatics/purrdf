// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Graph-partitioned succinct bitmap-triples: an HDT-style two-level
//! adjacency (subject → predicates → objects)
//! plus FoQ ("Focus on Querying") auxiliary indexes, answering all 8
//! `(s, p, o)` bound/unbound pattern shapes without ever decompressing a whole
//! partition.
//!
//! # Graph partitioning
//!
//! The dataset's quads are partitioned by graph: **partition 0 is always the
//! default graph** (`g == None`), even when it holds zero triples, so
//! [`GraphMatch::Default`] always has a well-defined (possibly empty) target.
//! Every distinct named graph gets its own partition, stored **strictly
//! ascending by graph unified id** after partition 0 — this is both the on-disk
//! order and the [`GraphMatch::Any`] union order (`partition 0 first, then named
//! graphs ascending`), so iteration order is deterministic byte-for-byte.
//!
//! # Per-partition local numbering
//!
//! Role numbering in [`super::dict::PackDict`] is global (one unified id space
//! for the whole dataset), but a single partition typically touches only a small
//! subset of it. Each partition therefore mints its OWN dense local numbering:
//! the distinct subjects / predicates / objects **used in that partition** are
//! each mapped, in ascending unified-id order, to a contiguous local id
//! `[0..n)`. The three `local -> unified` maps are stored as monotonically
//! increasing [`IntVector`]s, so `unified -> local` is a binary search over the
//! map (implemented by the private `local_lookup` helper below) — no separate
//! reverse map is stored.
//!
//! # Two-level adjacency (Sp / Bp / So / Bo)
//!
//! A partition's triples are sorted by `(local_s, local_p, local_o)` and encoded
//! as two adjacency lists:
//!
//! - **Sp** — the sequence of `local_p`, grouped by `local_s` (subjects visited
//!   in local order, each subject's predicates ascending and de-duplicated).
//!   **Bp** is a boundary bitmap, the same length as `Sp`, using the **mark-last**
//!   convention: bit `i` is `1` iff `Sp[i]` is the LAST predicate of its
//!   subject's group. Subject `s`'s slice is therefore
//!   `[select1(s == 0 ? -1 : s - 1) + 1, select1(s) + 1)` (the first term
//!   omitted when `s == 0`), and the subject owning `Sp` position `i` is
//!   `rank1(i)` (the count of subject-groups already closed before `i`).
//! - **So** — the sequence of `local_o`, grouped by EACH `Sp` position (i.e. by
//!   distinct `(local_s, local_p)` pair, ascending and de-duplicated within the
//!   pair). **Bo** uses the identical mark-last convention over `So`, so an `Sp`
//!   position's object slice is found exactly like a subject's `Sp` slice above,
//!   substituting `Bo`/`So` for `Bp`/`Sp`.
//!
//! This two-level structure alone answers every SUBJECT-LED shape natively:
//! `(s,?,?)`, `(s,p,?)`, `(s,p,o)`, and the full scan `(?,?,?)`.
//!
//! # FoQ auxiliary indexes
//!
//! Two more indexes make the remaining (unbound-subject) shapes equally cheap:
//!
//! - **Predicate index** — for each local predicate id, the ascending list of
//!   `Sp` POSITIONS where it occurs (each position identifies one `(s, p)`
//!   pair). Delta-encoded (see [`super::bits::write_delta_list`]) with a
//!   per-predicate byte-offset/length pair, so a lookup is one indexed slice,
//!   not a scan.
//! - **Object index** — for each local object id, the ascending list of `Sp`
//!   POSITIONS (i.e. `(s, p)` pairs) that reach it, encoded identically. Because
//!   `So` groups are de-duplicated per `(s, p)` pair and every stored triple is
//!   already unique (dataset-level dedup, C0.5), one object-index entry
//!   corresponds to EXACTLY one triple — so an object's index-entry COUNT is
//!   also its exact triple count (used by
//!   [`TriplesRef::cardinality_upper_bound`]).
//!
//! # Per-shape access path
//!
//! | Pattern      | Path                                                          |
//! |--------------|----------------------------------------------------------------|
//! | `(s,p,o)`    | subject-led: `Sp` slice for `s`, binary-search `p`, then binary-search `o` in that position's `So` slice |
//! | `(s,p,?)`    | subject-led: same, then yield the whole `So` slice              |
//! | `(s,?,o)`    | subject-led: walk `s`'s whole `Sp` slice, binary-search `o` in each position's `So` slice |
//! | `(s,?,?)`    | subject-led: walk `s`'s whole `Sp` slice and every position's `So` slice |
//! | `(?,p,o)`    | intersect the predicate index for `p` with the object index for `o` (both ascending `Sp`-position lists — a linear merge) |
//! | `(?,p,?)`    | predicate index for `p`: for each position, derive the subject via `rank1` on `Bp` and yield the whole `So` slice |
//! | `(?,?,o)`    | object index for `o`: for each position, derive `(s, p)` via `rank1`/`Sp.get`  |
//! | `(?,?,?)`    | full scan: every local subject, its whole `Sp` slice, every position's whole `So` slice |
//!
//! # Serialization
//!
//! [`Triples::encode`] builds the whole self-contained on-disk buffer (format
//! version byte, partition count, then each partition length-framed); [`Triples::to_bytes`]
//! returns it. [`TriplesRef::from_bytes`] is the borrowed, zero-copy reader: the
//! `Sp`/`Bp`/`So`/`Bo` arrays and the FoQ index bytes all alias directly into the
//! caller's buffer (no allocation, no copy) — only the small per-partition
//! bookkeeping (the `Vec<PartitionRef>` and the named-graph lookup `BTreeMap`) is
//! owned. `from_bytes` fails closed: every structural invariant (ascending local
//! maps, in-range `Sp`/`So` entries, boundary-bitmap popcounts, FoQ delta-list
//! decodability/ordering/range) is validated once at open time, so a query never
//! panics on a successfully-opened buffer.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;

use crate::dataset_view::GraphMatch;
use crate::{RdfDataset, TermId};

use super::bits::{
    BitVec, DeltaListRef, IntVector, IntVectorRef, PackBitsError, RankSelectRef, bits_for,
    write_delta_list,
};
use super::dict::{PackDict, PackTermId};
use crate::hash::FastMap;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Why decoding a [`Triples`] byte buffer failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackTriplesError {
    /// The buffer ended before all the bytes a header promised were present.
    Truncated {
        /// The total leading byte count the format required.
        needed: usize,
        /// The byte count actually available.
        found: usize,
    },
    /// The buffer's header was internally inconsistent, an id/offset reference
    /// fell outside its documented domain, or a boundary bitmap's popcount
    /// disagreed with the structure it is supposed to index.
    Malformed(&'static str),
}

impl fmt::Display for PackTriplesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { needed, found } => write!(
                f,
                "pack-triples: truncated input: needed at least {needed} bytes, found {found}"
            ),
            Self::Malformed(reason) => write!(f, "pack-triples: malformed input: {reason}"),
        }
    }
}

impl std::error::Error for PackTriplesError {}

impl From<PackBitsError> for PackTriplesError {
    fn from(e: PackBitsError) -> Self {
        match e {
            PackBitsError::Truncated { needed, found } => Self::Truncated { needed, found },
            PackBitsError::Malformed(reason) => Self::Malformed(reason),
        }
    }
}

/// Read an 8-byte little-endian header field at `*pos`, advancing `*pos` past it.
/// A small local mirror of `bits::read_header_u64` (private to that module).
fn read_u64_header(bytes: &[u8], pos: &mut usize) -> Result<u64, PackTriplesError> {
    let end = *pos + 8;
    let slice = bytes.get(*pos..end).ok_or(PackTriplesError::Truncated {
        needed: end,
        found: bytes.len(),
    })?;
    let value = u64::from_le_bytes(slice.try_into().expect("slice is exactly 8 bytes"));
    *pos = end;
    Ok(value)
}

// ---------------------------------------------------------------------------
// Small owned-vector helpers shared by every FoQ/local-map builder.
// ---------------------------------------------------------------------------

/// Build a bit-packed [`IntVector`] wide enough for `values`' maximum element.
fn build_int_vector(values: &[u64]) -> IntVector {
    let max = values.iter().copied().max().unwrap_or(0);
    let mut v = IntVector::with_width(bits_for(max));
    for &x in values {
        v.push(x);
    }
    v
}

// ---------------------------------------------------------------------------
// Encoding: dataset+dict -> per-partition triple lists -> byte blocks.
// ---------------------------------------------------------------------------

/// The on-disk format version [`Triples::encode`] writes and
/// [`TriplesRef::from_bytes`] requires.
const TRIPLES_FORMAT_VERSION: u8 = 1;

/// Resolve `id`'s unified [`PackTermId`] via `dict`'s single id-space lookup
/// (`id_by_value` — see [`super::dict`]'s module docs: one unified id per
/// distinct value, regardless of role), memoized per `TermId` so repeated
/// subjects/predicates/objects/graph names in the quad scan cost one
/// `term_value`+lookup each, not one per quad. Used for every quad component —
/// `s`, `p`, `o`, and `g` alike — since the dictionary no longer splits
/// predicates into a separate id space.
fn resolve_unified(
    dataset: &RdfDataset,
    dict: &PackDict,
    cache: &mut FastMap<TermId, PackTermId>,
    id: TermId,
) -> PackTermId {
    if let Some(&u) = cache.get(&id) {
        return u;
    }
    let value = dataset.term_value(id);
    let u = dict.id_by_value(&value).expect(
        "PackDict::encode covers every role a quad component can play (incl. the \
         graph-name amendment), so every quad's s/p/o/g term resolves here",
    );
    cache.insert(id, u);
    u
}

/// Encode ONE partition's already-collected `(s_uni, p_uni, o_uni)` triples into
/// its self-contained byte block. `graph_id` is `None` for the default-graph
/// partition (stored as the `0` sentinel — safe since unified ids are 1-based)
/// and `Some(unified graph id)` for a named-graph partition.
///
/// Field order (see the [module docs](self) for what each holds):
/// `graph_id_or_zero: u64`, `n_triples: u64`, `local_s`, `local_p`, `local_o`
/// (each an [`IntVector`]), `sp` (`IntVector`), `bp` ([`RankSelect`]), `so`
/// (`IntVector`), `bo` (`RankSelect`), `pred_offsets`/`pred_counts`/`pred_totals`
/// (each an `IntVector`), `pred_index_data_len: u64` + bytes,
/// `obj_offsets`/`obj_counts` (each an `IntVector`), `obj_index_data_len: u64` +
/// bytes.
fn encode_partition(graph_id: Option<PackTermId>, triples: &[(u64, u64, u64)]) -> Vec<u8> {
    let mut s_set: Vec<u64> = triples.iter().map(|t| t.0).collect();
    s_set.sort_unstable();
    s_set.dedup();
    let mut p_set: Vec<u64> = triples.iter().map(|t| t.1).collect();
    p_set.sort_unstable();
    p_set.dedup();
    let mut o_set: Vec<u64> = triples.iter().map(|t| t.2).collect();
    o_set.sort_unstable();
    o_set.dedup();

    // Translate every triple to (local_s, local_p, local_o) and sort — triples
    // are already unique (dataset-level dedup, C0.5, is per-graph so this holds
    // within one partition), so `dedup` after sorting is a defensive no-op.
    let mut sorted: Vec<(u64, u64, u64)> = triples
        .iter()
        .map(|&(s, p, o)| {
            (
                s_set
                    .binary_search(&s)
                    .expect("s came from this triple list") as u64,
                p_set
                    .binary_search(&p)
                    .expect("p came from this triple list") as u64,
                o_set
                    .binary_search(&o)
                    .expect("o came from this triple list") as u64,
            )
        })
        .collect();
    sorted.sort_unstable();
    sorted.dedup();

    let n_p = p_set.len();
    let n_o = o_set.len();
    let mut sp = IntVector::with_width(bits_for(n_p.saturating_sub(1) as u64));
    let mut bp = BitVec::new();
    let mut so = IntVector::with_width(bits_for(n_o.saturating_sub(1) as u64));
    let mut bo = BitVec::new();

    // FoQ raw collectors, indexed by local predicate/object id. Each `Sp`
    // position `i` is pushed onto `pred_positions[Sp[i]]` exactly once (when the
    // position is created) and onto `obj_positions[o]` once per distinct object
    // reached at that position — both in strictly ascending `Sp`-position order
    // by construction (the single pass below visits positions in increasing
    // order), which is exactly what `write_delta_list` requires.
    let mut pred_positions: Vec<Vec<u64>> = vec![Vec::new(); n_p];
    let mut pred_totals: Vec<u64> = vec![0; n_p];
    let mut obj_positions: Vec<Vec<u64>> = vec![Vec::new(); n_o];

    let mut i = 0usize;
    while i < sorted.len() {
        let s = sorted[i].0;
        let mut j = i;
        while j < sorted.len() && sorted[j].0 == s {
            let p = sorted[j].1;
            let sp_pos = sp.len() as u64;
            sp.push(p);
            pred_positions[p as usize].push(sp_pos);

            let mut k = j;
            while k < sorted.len() && sorted[k].0 == s && sorted[k].1 == p {
                let o = sorted[k].2;
                so.push(o);
                obj_positions[o as usize].push(sp_pos);
                pred_totals[p as usize] += 1;
                let last_in_pair_group =
                    k + 1 == sorted.len() || sorted[k + 1].0 != s || sorted[k + 1].1 != p;
                bo.push(last_in_pair_group);
                k += 1;
            }
            let last_for_subject = k == sorted.len() || sorted[k].0 != s;
            bp.push(last_for_subject);
            j = k;
        }
        i = j;
    }

    let local_s = build_int_vector(&s_set);
    let local_p = build_int_vector(&p_set);
    let local_o = build_int_vector(&o_set);

    let mut pred_index_data = Vec::new();
    let mut pred_offsets = Vec::with_capacity(n_p);
    let mut pred_counts = Vec::with_capacity(n_p);
    for positions in &pred_positions {
        pred_offsets.push(pred_index_data.len() as u64);
        pred_counts.push(positions.len() as u64);
        write_delta_list(&mut pred_index_data, positions);
    }

    let mut obj_index_data = Vec::new();
    let mut obj_offsets = Vec::with_capacity(n_o);
    let mut obj_counts = Vec::with_capacity(n_o);
    for positions in &obj_positions {
        obj_offsets.push(obj_index_data.len() as u64);
        obj_counts.push(positions.len() as u64);
        write_delta_list(&mut obj_index_data, positions);
    }

    let mut out = Vec::new();
    out.extend_from_slice(&graph_id.unwrap_or(0).to_le_bytes());
    out.extend_from_slice(&(sorted.len() as u64).to_le_bytes());
    out.extend_from_slice(&local_s.to_bytes());
    out.extend_from_slice(&local_p.to_bytes());
    out.extend_from_slice(&local_o.to_bytes());
    out.extend_from_slice(&sp.to_bytes());
    out.extend_from_slice(&bp.freeze().to_bytes());
    out.extend_from_slice(&so.to_bytes());
    out.extend_from_slice(&bo.freeze().to_bytes());
    out.extend_from_slice(&build_int_vector(&pred_offsets).to_bytes());
    out.extend_from_slice(&build_int_vector(&pred_counts).to_bytes());
    out.extend_from_slice(&build_int_vector(&pred_totals).to_bytes());
    out.extend_from_slice(&(pred_index_data.len() as u64).to_le_bytes());
    out.extend_from_slice(&pred_index_data);
    out.extend_from_slice(&build_int_vector(&obj_offsets).to_bytes());
    out.extend_from_slice(&build_int_vector(&obj_counts).to_bytes());
    out.extend_from_slice(&(obj_index_data.len() as u64).to_le_bytes());
    out.extend_from_slice(&obj_index_data);
    out
}

/// The owned, self-contained encoded form of the whole graph-partitioned
/// bitmap-triples structure, built by [`Triples::encode`]. Not itself
/// queryable — [`to_bytes`](Self::to_bytes) hands the buffer to
/// [`TriplesRef::from_bytes`], the zero-copy queryable reader (see the
/// [module docs](self)).
#[derive(Debug, Clone)]
pub struct Triples {
    bytes: Vec<u8>,
}

impl Triples {
    /// Scan `dataset`'s quads, partition them by graph (partition 0 = default
    /// graph, always present even if empty; each named graph gets its own
    /// partition, stored ascending by graph unified id), and build each
    /// partition's bitmap-triples + FoQ indexes. `dict` resolves every quad
    /// component — subject, predicate, object, and graph name alike — to its
    /// single unified [`PackTermId`] via [`PackDict::id_by_value`] (see
    /// [`super::dict`]'s module docs: this dictionary mints ONE id per distinct
    /// value, regardless of role) — `dict` MUST be the dictionary built from
    /// this exact `dataset` (via [`PackDict::encode`]), so every reference
    /// resolves; see the [module docs](self).
    ///
    /// # Panics
    ///
    /// Panics (via the internal resolver's `expect`) if `dict` was not built
    /// from `dataset` (a quad component has no unified id) — a caller-side
    /// contract violation, not a data-dependent error.
    #[must_use]
    pub fn encode(dict: &PackDict, dataset: &RdfDataset) -> Self {
        let mut cache: FastMap<TermId, PackTermId> = FastMap::default();

        let mut default_triples: Vec<(u64, u64, u64)> = Vec::new();
        let mut named: BTreeMap<PackTermId, Vec<(u64, u64, u64)>> = BTreeMap::new();

        for q in dataset.quads() {
            let s_uni = resolve_unified(dataset, dict, &mut cache, q.s);
            let p_uni = resolve_unified(dataset, dict, &mut cache, q.p);
            let o_uni = resolve_unified(dataset, dict, &mut cache, q.o);
            match q.g {
                None => default_triples.push((s_uni, p_uni, o_uni)),
                Some(g) => {
                    let g_uni = resolve_unified(dataset, dict, &mut cache, g);
                    named.entry(g_uni).or_default().push((s_uni, p_uni, o_uni));
                }
            }
        }

        let mut out = Vec::new();
        out.push(TRIPLES_FORMAT_VERSION);
        out.extend_from_slice(&(1 + named.len() as u64).to_le_bytes());

        let default_bytes = encode_partition(None, &default_triples);
        out.extend_from_slice(&(default_bytes.len() as u64).to_le_bytes());
        out.extend_from_slice(&default_bytes);

        for (&g_uni, triples) in &named {
            let bytes = encode_partition(Some(g_uni), triples);
            out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
            out.extend_from_slice(&bytes);
        }

        Self { bytes: out }
    }

    /// The serialized byte buffer [`TriplesRef::from_bytes`] reads.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }
}

// ---------------------------------------------------------------------------
// Borrowed, zero-copy partition reader.
// ---------------------------------------------------------------------------

/// A borrowed, zero-copy view of one partition's bitmap-triples + FoQ indexes,
/// parsed by [`PartitionRef::from_bytes`]. `Copy` (every field either is a
/// borrowed `*Ref` reader or fits in a machine word), so partition-local helper
/// functions take it BY VALUE — letting closures capture an owned copy tied to
/// the buffer's lifetime `'a` without holding a live borrow of the parent
/// [`TriplesRef`].
#[derive(Debug, Clone, Copy)]
struct PartitionRef<'a> {
    /// `None` for the default-graph partition; `Some(unified graph id)` for a
    /// named-graph partition.
    graph_id: Option<PackTermId>,
    /// The partition's total triple count (== `so.len()`).
    n_triples: u64,
    /// Local subject id -> unified id, ascending.
    local_s: IntVectorRef<'a>,
    /// Local predicate id -> unified id, ascending.
    local_p: IntVectorRef<'a>,
    /// Local object id -> unified id, ascending.
    local_o: IntVectorRef<'a>,
    /// `Sp`: local predicate ids, grouped by subject.
    sp: IntVectorRef<'a>,
    /// `Bp`: mark-last boundary bitmap over `sp`.
    bp: RankSelectRef<'a>,
    /// `So`: local object ids, grouped by `Sp` position (i.e. by `(s, p)` pair).
    so: IntVectorRef<'a>,
    /// `Bo`: mark-last boundary bitmap over `so`.
    bo: RankSelectRef<'a>,
    /// Predicate index: per local predicate id, the byte offset into
    /// `pred_index_data` where its delta-encoded `Sp`-position list starts.
    pred_offsets: IntVectorRef<'a>,
    /// Predicate index: per local predicate id, its delta-list element count.
    pred_counts: IntVectorRef<'a>,
    /// Predicate index: per local predicate id, its EXACT total triple count
    /// (sum of `So`-group sizes across its `Sp` positions) — used by
    /// [`TriplesRef::cardinality_upper_bound`] for `(?,p,?)`.
    pred_totals: IntVectorRef<'a>,
    /// The predicate index's concatenated delta-list bytes.
    pred_index_data: &'a [u8],
    /// Object index: per local object id, the byte offset into `obj_index_data`.
    obj_offsets: IntVectorRef<'a>,
    /// Object index: per local object id, its delta-list element count — which,
    /// because each entry corresponds to exactly one triple (see the
    /// [module docs](self)), is ALSO its exact triple count.
    obj_counts: IntVectorRef<'a>,
    /// The object index's concatenated delta-list bytes.
    obj_index_data: &'a [u8],
}

/// Verify `map`'s stored values are strictly ascending — the invariant
/// `local_lookup`'s binary search (and the ascending-order guarantees the FoQ
/// indexes rely on) depend on.
fn assert_strictly_ascending(
    map: IntVectorRef<'_>,
    what: &'static str,
) -> Result<(), PackTriplesError> {
    let mut prev: Option<u64> = None;
    for i in 0..map.len() {
        let cur = map.get(i);
        if let Some(p) = prev
            && cur <= p
        {
            return Err(PackTriplesError::Malformed(what));
        }
        prev = Some(cur);
    }
    Ok(())
}

/// Verify every value in `vec` is `< bound` — validates that `Sp`/`So` entries
/// address a real local predicate/object id.
fn assert_in_range(
    vec: IntVectorRef<'_>,
    bound: usize,
    what: &'static str,
) -> Result<(), PackTriplesError> {
    for i in 0..vec.len() {
        if vec.get(i) as usize >= bound {
            return Err(PackTriplesError::Malformed(what));
        }
    }
    Ok(())
}

/// Sum every value in `vec` (used to cross-check FoQ index bookkeeping against
/// `sp.len()`/`so.len()`). Returns `None` on `u64` overflow rather than
/// panicking (debug builds) or silently wrapping (release builds) — `vec`
/// comes straight from an untrusted pack, so an adversarial vector whose
/// values overflow when summed must fail closed instead of risking a wrapped
/// total that coincidentally passes its caller's cross-check.
fn sum_int_vector(vec: IntVectorRef<'_>) -> Option<u64> {
    (0..vec.len()).try_fold(0u64, |acc, i| acc.checked_add(vec.get(i)))
}

/// Validate and, implicitly, fully decode ONE FoQ index's delta-list bytes: for
/// every local id `i`, the list at `offsets.get(i)` must decode `counts.get(i)`
/// values, ascending, each `< position_bound`, with the byte span from one
/// offset to the next (or to `data.len()` for the last) consumed EXACTLY (no
/// gap, no overlap, no trailing garbage). Run once at [`PartitionRef::from_bytes`]
/// time so a later query never has to propagate a decode error through the
/// infallible `pattern`/`all_quads` iterators.
fn validate_delta_index(
    data: &[u8],
    offsets: IntVectorRef<'_>,
    counts: IntVectorRef<'_>,
    position_bound: u64,
) -> Result<(), PackTriplesError> {
    let n = offsets.len();
    let mut prev_offset = 0u64;
    for i in 0..n {
        let offset = offsets.get(i);
        if i > 0 && offset < prev_offset {
            return Err(PackTriplesError::Malformed(
                "triples: foq index offsets are not ascending",
            ));
        }
        prev_offset = offset;
        let count = counts.get(i) as usize;
        let offset_usize = usize::try_from(offset)
            .map_err(|_| PackTriplesError::Malformed("triples: foq index offset exceeds usize"))?;
        let slice = data.get(offset_usize..).ok_or(PackTriplesError::Malformed(
            "triples: foq index offset out of range",
        ))?;
        let mut list = DeltaListRef::new(slice, count);
        let mut last: Option<u64> = None;
        for item in &mut list {
            let v = item?;
            if let Some(l) = last
                && v < l
            {
                return Err(PackTriplesError::Malformed(
                    "triples: foq index positions are not ascending",
                ));
            }
            if v >= position_bound {
                return Err(PackTriplesError::Malformed(
                    "triples: foq index position out of range",
                ));
            }
            last = Some(v);
        }
        let consumed = list.consumed_len();
        let next_boundary = if i + 1 < n {
            usize::try_from(offsets.get(i + 1)).map_err(|_| {
                PackTriplesError::Malformed("triples: foq index offset exceeds usize")
            })?
        } else {
            data.len()
        };
        if offset_usize + consumed != next_boundary {
            return Err(PackTriplesError::Malformed(
                "triples: foq index list length disagrees with the next offset",
            ));
        }
    }
    Ok(())
}

impl<'a> PartitionRef<'a> {
    /// Parse one [`encode_partition`]-produced byte block. Returns the parsed
    /// partition and the number of leading bytes of `bytes` it consumed — the
    /// caller (`TriplesRef::from_bytes`) checks that equals `bytes.len()`
    /// exactly (no trailing garbage inside a partition's own framed span).
    ///
    /// # Errors
    ///
    /// [`PackTriplesError`] on any truncation or structural inconsistency (see
    /// the [module docs](self) for what is validated).
    fn from_bytes(bytes: &'a [u8]) -> Result<(Self, usize), PackTriplesError> {
        let mut pos = 0usize;
        let graph_id_raw = read_u64_header(bytes, &mut pos)?;
        let graph_id = if graph_id_raw == 0 {
            None
        } else {
            Some(graph_id_raw)
        };
        let n_triples = read_u64_header(bytes, &mut pos)?;

        let local_s = IntVectorRef::from_bytes(&bytes[pos..])?;
        pos += local_s.serialized_len();
        let local_p = IntVectorRef::from_bytes(&bytes[pos..])?;
        pos += local_p.serialized_len();
        let local_o = IntVectorRef::from_bytes(&bytes[pos..])?;
        pos += local_o.serialized_len();
        let sp = IntVectorRef::from_bytes(&bytes[pos..])?;
        pos += sp.serialized_len();
        let bp = RankSelectRef::from_bytes(&bytes[pos..])?;
        pos += bp.serialized_len();
        let so = IntVectorRef::from_bytes(&bytes[pos..])?;
        pos += so.serialized_len();
        let bo = RankSelectRef::from_bytes(&bytes[pos..])?;
        pos += bo.serialized_len();
        let pred_offsets = IntVectorRef::from_bytes(&bytes[pos..])?;
        pos += pred_offsets.serialized_len();
        let pred_counts = IntVectorRef::from_bytes(&bytes[pos..])?;
        pos += pred_counts.serialized_len();
        let pred_totals = IntVectorRef::from_bytes(&bytes[pos..])?;
        pos += pred_totals.serialized_len();
        let pred_data_len = read_u64_header(bytes, &mut pos)? as usize;
        let pred_index_data =
            bytes
                .get(pos..pos + pred_data_len)
                .ok_or(PackTriplesError::Truncated {
                    needed: pos + pred_data_len,
                    found: bytes.len(),
                })?;
        pos += pred_data_len;
        let obj_offsets = IntVectorRef::from_bytes(&bytes[pos..])?;
        pos += obj_offsets.serialized_len();
        let obj_counts = IntVectorRef::from_bytes(&bytes[pos..])?;
        pos += obj_counts.serialized_len();
        let obj_data_len = read_u64_header(bytes, &mut pos)? as usize;
        let obj_index_data =
            bytes
                .get(pos..pos + obj_data_len)
                .ok_or(PackTriplesError::Truncated {
                    needed: pos + obj_data_len,
                    found: bytes.len(),
                })?;
        pos += obj_data_len;

        // -- Structural validation (fail-closed) ---------------------------
        assert_strictly_ascending(local_s, "triples: local_s map is not strictly ascending")?;
        assert_strictly_ascending(local_p, "triples: local_p map is not strictly ascending")?;
        assert_strictly_ascending(local_o, "triples: local_o map is not strictly ascending")?;

        if bp.len() != sp.len() {
            return Err(PackTriplesError::Malformed(
                "triples: bp length disagrees with sp",
            ));
        }
        if bo.len() != so.len() {
            return Err(PackTriplesError::Malformed(
                "triples: bo length disagrees with so",
            ));
        }
        if so.len() as u64 != n_triples {
            return Err(PackTriplesError::Malformed(
                "triples: so length disagrees with n_triples",
            ));
        }
        if bp.total_ones() != local_s.len() {
            return Err(PackTriplesError::Malformed(
                "triples: bp popcount disagrees with the subject count",
            ));
        }
        if bo.total_ones() != sp.len() {
            return Err(PackTriplesError::Malformed(
                "triples: bo popcount disagrees with the (s,p)-pair count",
            ));
        }
        assert_in_range(
            sp,
            local_p.len(),
            "triples: sp entry addresses no local predicate",
        )?;
        assert_in_range(
            so,
            local_o.len(),
            "triples: so entry addresses no local object",
        )?;

        if pred_offsets.len() != local_p.len()
            || pred_counts.len() != local_p.len()
            || pred_totals.len() != local_p.len()
        {
            return Err(PackTriplesError::Malformed(
                "triples: predicate index length disagrees with local_p",
            ));
        }
        if obj_offsets.len() != local_o.len() || obj_counts.len() != local_o.len() {
            return Err(PackTriplesError::Malformed(
                "triples: object index length disagrees with local_o",
            ));
        }
        if sum_int_vector(pred_counts) != Some(sp.len() as u64) {
            return Err(PackTriplesError::Malformed(
                "triples: predicate index counts do not sum to sp.len()",
            ));
        }
        if sum_int_vector(pred_totals) != Some(n_triples) {
            return Err(PackTriplesError::Malformed(
                "triples: predicate index totals do not sum to n_triples",
            ));
        }
        if sum_int_vector(obj_counts) != Some(so.len() as u64) {
            return Err(PackTriplesError::Malformed(
                "triples: object index counts do not sum to so.len()",
            ));
        }
        // Both FoQ indexes store Sp-POSITIONS (not local ids), so both are
        // bounded by sp.len().
        validate_delta_index(pred_index_data, pred_offsets, pred_counts, sp.len() as u64)?;
        validate_delta_index(obj_index_data, obj_offsets, obj_counts, sp.len() as u64)?;

        Ok((
            Self {
                graph_id,
                n_triples,
                local_s,
                local_p,
                local_o,
                sp,
                bp,
                so,
                bo,
                pred_offsets,
                pred_counts,
                pred_totals,
                pred_index_data,
                obj_offsets,
                obj_counts,
                obj_index_data,
            },
            pos,
        ))
    }
}

// ---------------------------------------------------------------------------
// Partition-local query primitives (all operate on a `PartitionRef` BY VALUE).
// ---------------------------------------------------------------------------

/// Binary-search `map` (ascending) for `unified`, returning its local index.
fn local_lookup(map: IntVectorRef<'_>, unified: PackTermId) -> Option<u64> {
    let mut lo = 0usize;
    let mut hi = map.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        match map.get(mid).cmp(&unified) {
            Ordering::Less => lo = mid + 1,
            Ordering::Greater => hi = mid,
            Ordering::Equal => return Some(mid as u64),
        }
    }
    None
}

/// Binary-search `vec[start..end)` (ascending within that range) for `target`.
fn binary_search_range(
    vec: IntVectorRef<'_>,
    start: usize,
    end: usize,
    target: u64,
) -> Option<usize> {
    let mut lo = start;
    let mut hi = end;
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        match vec.get(mid).cmp(&target) {
            Ordering::Less => lo = mid + 1,
            Ordering::Greater => hi = mid,
            Ordering::Equal => return Some(mid),
        }
    }
    None
}

/// Subject `local_s`'s slice `[start, end)` into `sp` (mark-last convention over
/// `bp` — see the [module docs](self)).
fn subject_slice(part: &PartitionRef<'_>, local_s: u64) -> (usize, usize) {
    let ls = local_s as usize;
    let start = if ls == 0 {
        0
    } else {
        part.bp
            .select1(ls - 1)
            .expect("dense local subject numbering guarantees a bp boundary bit per subject")
            + 1
    };
    let end = part
        .bp
        .select1(ls)
        .expect("dense local subject numbering guarantees a bp boundary bit per subject")
        + 1;
    (start, end)
}

/// `Sp` position `sp_pos`'s object slice `[start, end)` into `so` (identical
/// mark-last convention over `bo`, substituting `Sp`-position for subject).
fn sp_pair_slice(part: &PartitionRef<'_>, sp_pos: u64) -> (usize, usize) {
    let i = sp_pos as usize;
    let start = if i == 0 {
        0
    } else {
        part.bo
            .select1(i - 1)
            .expect("every sp position has a bo boundary bit")
            + 1
    };
    let end = part
        .bo
        .select1(i)
        .expect("every sp position has a bo boundary bit")
        + 1;
    (start, end)
}

/// The local subject id owning `Sp` position `sp_pos` (the count of
/// subject-groups already closed strictly before it).
fn subject_of(part: &PartitionRef<'_>, sp_pos: u64) -> u64 {
    part.bp.rank1(sp_pos as usize) as u64
}

/// The exact triple count for subject `local_s`: the union of its per-predicate
/// `So` slices is itself contiguous (triples are visited subject-major during
/// encoding), so this is one subtraction after two `select1` calls — no walk.
fn subject_triple_count(part: &PartitionRef<'_>, local_s: u64) -> usize {
    let (start, end) = subject_slice(part, local_s);
    if start == end {
        return 0;
    }
    let (o_first_start, _) = sp_pair_slice(part, start as u64);
    let (_, o_last_end) = sp_pair_slice(part, (end - 1) as u64);
    o_last_end - o_first_start
}

/// Decode a FoQ index's delta list for local id `local_id`, already validated
/// at [`PartitionRef::from_bytes`] time (so every `.expect` here is a
/// broken-invariant bug, not a data-dependent error).
fn foq_positions(
    data: &[u8],
    offsets: IntVectorRef<'_>,
    counts: IntVectorRef<'_>,
    local_id: u64,
) -> Vec<u64> {
    let count = counts.get(local_id as usize) as usize;
    let offset = offsets.get(local_id as usize) as usize;
    DeltaListRef::new(&data[offset..], count)
        .map(|r| r.expect("validated at PartitionRef::from_bytes time"))
        .collect()
}

/// Iterate one partition's triples, as LOCAL `(local_s, local_p, local_o)`
/// rows, matching the unified-id pattern `(s, p, o)` (each `None` = unbound). A
/// bound axis whose unified id has no local id in this partition yields nothing
/// (no partial match is possible). Dispatches to the access path documented in
/// the [module docs](self) table, based on which axes are bound.
fn partition_rows<'a>(
    part: &PartitionRef<'a>,
    s: Option<PackTermId>,
    p: Option<PackTermId>,
    o: Option<PackTermId>,
) -> Box<dyn Iterator<Item = (u64, u64, u64)> + 'a> {
    let local_s = match s {
        Some(u) => match local_lookup(part.local_s, u) {
            Some(l) => Some(l),
            None => return Box::new(std::iter::empty()),
        },
        None => None,
    };
    let local_p = match p {
        Some(u) => match local_lookup(part.local_p, u) {
            Some(l) => Some(l),
            None => return Box::new(std::iter::empty()),
        },
        None => None,
    };
    let local_o = match o {
        Some(u) => match local_lookup(part.local_o, u) {
            Some(l) => Some(l),
            None => return Box::new(std::iter::empty()),
        },
        None => None,
    };
    // Own a fresh copy (cheap: `PartitionRef` is `Copy`) for the closures below
    // to move into the returned `Box<dyn Iterator + 'a>` — they must outlive this
    // call, which a borrow of the `&PartitionRef<'a>` parameter cannot.
    let part = *part;

    match (local_s, local_p, local_o) {
        (Some(ls), Some(lp), Some(lo)) => {
            let (start, end) = subject_slice(&part, ls);
            let found = binary_search_range(part.sp, start, end, lp).and_then(|i| {
                let (ostart, oend) = sp_pair_slice(&part, i as u64);
                binary_search_range(part.so, ostart, oend, lo)
            });
            Box::new(found.map(|_| (ls, lp, lo)).into_iter())
        }
        (Some(ls), Some(lp), None) => {
            let (start, end) = subject_slice(&part, ls);
            let (ostart, oend) = binary_search_range(part.sp, start, end, lp)
                .map_or((0, 0), |i| sp_pair_slice(&part, i as u64));
            Box::new((ostart..oend).map(move |oi| (ls, lp, part.so.get(oi))))
        }
        (Some(ls), None, Some(lo)) => {
            let (start, end) = subject_slice(&part, ls);
            Box::new((start..end).filter_map(move |i| {
                let p = part.sp.get(i);
                let (ostart, oend) = sp_pair_slice(&part, i as u64);
                binary_search_range(part.so, ostart, oend, lo).map(|_| (ls, p, lo))
            }))
        }
        (Some(ls), None, None) => {
            let (start, end) = subject_slice(&part, ls);
            Box::new((start..end).flat_map(move |i| {
                let p = part.sp.get(i);
                let (ostart, oend) = sp_pair_slice(&part, i as u64);
                (ostart..oend).map(move |oi| (ls, p, part.so.get(oi)))
            }))
        }
        (None, Some(lp), Some(lo)) => {
            let p_positions = foq_positions(
                part.pred_index_data,
                part.pred_offsets,
                part.pred_counts,
                lp,
            );
            let o_positions =
                foq_positions(part.obj_index_data, part.obj_offsets, part.obj_counts, lo);
            let mut common = Vec::new();
            let (mut i, mut j) = (0usize, 0usize);
            while i < p_positions.len() && j < o_positions.len() {
                match p_positions[i].cmp(&o_positions[j]) {
                    Ordering::Less => i += 1,
                    Ordering::Greater => j += 1,
                    Ordering::Equal => {
                        common.push(p_positions[i]);
                        i += 1;
                        j += 1;
                    }
                }
            }
            Box::new(
                common
                    .into_iter()
                    .map(move |pos| (subject_of(&part, pos), lp, lo)),
            )
        }
        (None, Some(lp), None) => {
            let positions = foq_positions(
                part.pred_index_data,
                part.pred_offsets,
                part.pred_counts,
                lp,
            );
            Box::new(positions.into_iter().flat_map(move |pos| {
                let ls = subject_of(&part, pos);
                let (ostart, oend) = sp_pair_slice(&part, pos);
                (ostart..oend).map(move |oi| (ls, lp, part.so.get(oi)))
            }))
        }
        (None, None, Some(lo)) => {
            let positions =
                foq_positions(part.obj_index_data, part.obj_offsets, part.obj_counts, lo);
            Box::new(positions.into_iter().map(move |pos| {
                let ls = subject_of(&part, pos);
                let lp = part.sp.get(pos as usize);
                (ls, lp, lo)
            }))
        }
        (None, None, None) => {
            let n_s = part.local_s.len() as u64;
            Box::new((0..n_s).flat_map(move |ls| {
                let (start, end) = subject_slice(&part, ls);
                (start..end).flat_map(move |i| {
                    let p = part.sp.get(i);
                    let (ostart, oend) = sp_pair_slice(&part, i as u64);
                    (ostart..oend).map(move |oi| (ls, p, part.so.get(oi)))
                })
            }))
        }
    }
}

/// A cheap (`O(1)`/`O(log n)`, index-sizes-only) upper bound on the number of
/// rows [`partition_rows`] would yield for the same pattern — see
/// [`TriplesRef::cardinality_upper_bound`].
fn partition_upper_bound(
    part: &PartitionRef<'_>,
    s: Option<PackTermId>,
    p: Option<PackTermId>,
    o: Option<PackTermId>,
) -> usize {
    let local_s = s.map(|u| local_lookup(part.local_s, u));
    let local_p = p.map(|u| local_lookup(part.local_p, u));
    let local_o = o.map(|u| local_lookup(part.local_o, u));
    if local_s == Some(None) || local_p == Some(None) || local_o == Some(None) {
        return 0;
    }
    let local_s = local_s.flatten();
    let local_p = local_p.flatten();
    let local_o = local_o.flatten();

    match (local_s, local_p, local_o) {
        (Some(ls), Some(lp), Some(_)) => {
            let (start, end) = subject_slice(part, ls);
            usize::from(binary_search_range(part.sp, start, end, lp).is_some())
        }
        (Some(ls), Some(lp), None) => {
            let (start, end) = subject_slice(part, ls);
            binary_search_range(part.sp, start, end, lp).map_or(0, |i| {
                let (os, oe) = sp_pair_slice(part, i as u64);
                oe - os
            })
        }
        (Some(ls), None, Some(_)) => {
            let (start, end) = subject_slice(part, ls);
            end - start
        }
        (Some(ls), None, None) => subject_triple_count(part, ls),
        (None, Some(lp), Some(lo)) => {
            let pred_groups = part.pred_counts.get(lp as usize) as usize;
            let obj_triples = part.obj_counts.get(lo as usize) as usize;
            pred_groups.min(obj_triples)
        }
        (None, Some(lp), None) => part.pred_totals.get(lp as usize) as usize,
        (None, None, Some(lo)) => part.obj_counts.get(lo as usize) as usize,
        (None, None, None) => part.n_triples as usize,
    }
}

// ---------------------------------------------------------------------------
// TriplesRef — the borrowed, queryable, zero-copy reader.
// ---------------------------------------------------------------------------

/// The borrowed, zero-copy, queryable form of a graph-partitioned bitmap-triples
/// buffer, opened via [`from_bytes`](Self::from_bytes). Partition 0 is always
/// the default graph (possibly empty); every other partition is a named graph,
/// stored strictly ascending by graph unified id. See the [module docs](self)
/// for the two-level adjacency, the FoQ indexes, and the per-shape access path.
#[derive(Debug, Clone)]
pub struct TriplesRef<'a> {
    /// Partition 0 = default graph; partitions `[1..]` = named graphs, ascending
    /// by graph unified id (also the [`GraphMatch::Any`] union order).
    partitions: Vec<PartitionRef<'a>>,
    /// Named-graph unified id -> partition index (never `0`, that is always the
    /// default graph).
    graph_index: BTreeMap<PackTermId, usize>,
}

impl<'a> TriplesRef<'a> {
    /// Parse [`Triples::to_bytes`]'s output. Zero-copy: every `Sp`/`Bp`/`So`/`Bo`
    /// array and every FoQ index's bytes alias `bytes` directly.
    ///
    /// # Errors
    ///
    /// [`PackTriplesError`] on truncation or any structural inconsistency (see
    /// the [module docs](self); every invariant is checked once, here, so a
    /// later query never panics on a successfully-opened buffer).
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, PackTriplesError> {
        let version = *bytes.first().ok_or(PackTriplesError::Truncated {
            needed: 1,
            found: 0,
        })?;
        if version != TRIPLES_FORMAT_VERSION {
            return Err(PackTriplesError::Malformed(
                "triples: unsupported format version",
            ));
        }
        let mut pos = 1usize;
        let partition_count = read_u64_header(bytes, &mut pos)? as usize;
        if partition_count == 0 {
            return Err(PackTriplesError::Malformed(
                "triples: missing the mandatory default-graph partition",
            ));
        }
        let mut partitions = Vec::with_capacity(partition_count);
        for idx in 0..partition_count {
            let plen = read_u64_header(bytes, &mut pos)? as usize;
            let pbytes = bytes
                .get(pos..pos + plen)
                .ok_or(PackTriplesError::Truncated {
                    needed: pos + plen,
                    found: bytes.len(),
                })?;
            let (partition, consumed) = PartitionRef::from_bytes(pbytes)?;
            if consumed != pbytes.len() {
                return Err(PackTriplesError::Malformed(
                    "triples: partition block has trailing garbage",
                ));
            }
            if idx == 0 {
                if partition.graph_id.is_some() {
                    return Err(PackTriplesError::Malformed(
                        "triples: partition 0 must be the default graph",
                    ));
                }
            } else if partition.graph_id.is_none() {
                return Err(PackTriplesError::Malformed(
                    "triples: only partition 0 may be the default graph",
                ));
            }
            pos += plen;
            partitions.push(partition);
        }

        let mut graph_index = BTreeMap::new();
        let mut prev_g: Option<PackTermId> = None;
        for (i, part) in partitions.iter().enumerate().skip(1) {
            let g = part
                .graph_id
                .expect("checked above: partitions[1..] are all named-graph");
            if let Some(p) = prev_g
                && g <= p
            {
                return Err(PackTriplesError::Malformed(
                    "triples: named-graph partitions are not strictly ascending by graph id",
                ));
            }
            prev_g = Some(g);
            graph_index.insert(g, i);
        }

        Ok(Self {
            partitions,
            graph_index,
        })
    }

    /// Select the partitions `g` addresses, in the fixed deterministic order
    /// (`Any` = partition 0 then named graphs ascending; `Default` = just
    /// partition 0; `Named` = just that one partition, or none if absent).
    fn selected_partitions(&self, g: GraphMatch<PackTermId>) -> Vec<PartitionRef<'a>> {
        match g {
            GraphMatch::Default => vec![self.partitions[0]],
            GraphMatch::Named(gid) => self
                .graph_index
                .get(&gid)
                .map(|&idx| self.partitions[idx])
                .into_iter()
                .collect(),
            GraphMatch::Any => self.partitions.clone(),
        }
    }

    /// Every quad matching the unified-id pattern `(s, p, o)` (each `None` =
    /// unbound) and `g`, as `(s_uni, p_uni, o_uni, g_uni)` rows (`g_uni` is
    /// `None` for a default-graph row). Iterates without decompressing any
    /// partition it does not need to touch — see the [module docs](self) for
    /// the access path chosen per pattern shape.
    pub fn pattern(
        &self,
        s: Option<PackTermId>,
        p: Option<PackTermId>,
        o: Option<PackTermId>,
        g: GraphMatch<PackTermId>,
    ) -> impl Iterator<Item = (PackTermId, PackTermId, PackTermId, Option<PackTermId>)> + '_ {
        self.selected_partitions(g)
            .into_iter()
            .flat_map(move |part| {
                let graph_id = part.graph_id;
                partition_rows(&part, s, p, o).map(move |(ls, lp, lo)| {
                    (
                        part.local_s.get(ls as usize),
                        part.local_p.get(lp as usize),
                        part.local_o.get(lo as usize),
                        graph_id,
                    )
                })
            })
    }

    /// Every quad in every partition (default graph first, then named graphs
    /// ascending by graph id) — equivalent to
    /// `pattern(None, None, None, GraphMatch::Any)`.
    pub fn all_quads(
        &self,
    ) -> impl Iterator<Item = (PackTermId, PackTermId, PackTermId, Option<PackTermId>)> + '_ {
        self.pattern(None, None, None, GraphMatch::Any)
    }

    /// Every named graph's unified id, ascending — the source for
    /// [`DatasetView::named_graphs`](crate::DatasetView::named_graphs) over a
    /// `PackView`-backed dataset.
    pub fn named_graph_ids(&self) -> impl Iterator<Item = PackTermId> + '_ {
        self.graph_index.keys().copied()
    }

    /// A cheap (`O(partitions)` × `O(1)`/`O(log n)` per partition, derived only
    /// from index SIZES — never a materialized count) upper bound on
    /// [`pattern`](Self::pattern)'s row count for the same arguments. For cost
    /// ranking only (`cardinality_estimate`), never an exact `COUNT`.
    #[must_use]
    pub fn cardinality_upper_bound(
        &self,
        s: Option<PackTermId>,
        p: Option<PackTermId>,
        o: Option<PackTermId>,
        g: GraphMatch<PackTermId>,
    ) -> usize {
        self.selected_partitions(g)
            .into_iter()
            .map(|part| partition_upper_bound(&part, s, p, o))
            .sum()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::pack::dict::PackDict;
    use crate::{RdfDatasetBuilder, TermValue};

    fn iri(name: &str) -> TermValue {
        TermValue::iri(format!("http://example.org/{name}"))
    }

    /// Encode `dataset` end-to-end (dict + triples) and open the borrowed
    /// reader over freshly-serialized bytes — the standard test fixture path.
    fn build_and_open(dataset: &RdfDataset) -> (PackDict, Vec<u8>) {
        let dict_bytes = PackDict::encode(dataset).to_bytes();
        let dict = PackDict::open(&dict_bytes).expect("dict opens");
        let triples_bytes = Triples::encode(&dict, dataset).to_bytes();
        (dict, triples_bytes)
    }

    #[test]
    fn single_subject_single_predicate_single_object_partition() {
        // The minimal non-empty partition: one triple. Boundary convention check
        // (subject with exactly one predicate group, predicate with exactly one
        // object) at the smallest possible scale.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s");
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri("http://example.org/o");
        b.push_quad(s, p, o, None);
        let dataset = b.freeze().expect("valid dataset");

        let (dict, bytes) = build_and_open(&dataset);
        let triples = TriplesRef::from_bytes(&bytes).expect("opens");

        let s_id = dict.id_by_value(&iri("s")).expect("present");
        let p_id = dict.predicate_id_by_value(&iri("p")).expect("present");
        let o_id = dict.id_by_value(&iri("o")).expect("present");

        let rows: Vec<_> = triples.pattern(None, None, None, GraphMatch::Any).collect();
        assert_eq!(rows, vec![(s_id, p_id, o_id, None)]);

        // Every one of the 8 shapes on this single triple.
        assert_eq!(
            triples
                .pattern(Some(s_id), Some(p_id), Some(o_id), GraphMatch::Any)
                .count(),
            1
        );
        assert_eq!(
            triples
                .pattern(Some(s_id), Some(p_id), None, GraphMatch::Any)
                .count(),
            1
        );
        assert_eq!(
            triples
                .pattern(Some(s_id), None, Some(o_id), GraphMatch::Any)
                .count(),
            1
        );
        assert_eq!(
            triples
                .pattern(Some(s_id), None, None, GraphMatch::Any)
                .count(),
            1
        );
        assert_eq!(
            triples
                .pattern(None, Some(p_id), Some(o_id), GraphMatch::Any)
                .count(),
            1
        );
        assert_eq!(
            triples
                .pattern(None, Some(p_id), None, GraphMatch::Any)
                .count(),
            1
        );
        assert_eq!(
            triples
                .pattern(None, None, Some(o_id), GraphMatch::Any)
                .count(),
            1
        );
        assert_eq!(
            triples.pattern(None, None, None, GraphMatch::Any).count(),
            1
        );
    }

    #[test]
    fn multi_predicate_subject_groups_boundaries_correctly() {
        // "s" has two predicates, each with two objects: exercises the Bp/Bo
        // mark-last boundary convention across a group with >1 member.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s");
        let p1 = b.intern_iri("http://example.org/p1");
        let p2 = b.intern_iri("http://example.org/p2");
        let o1 = b.intern_iri("http://example.org/o1");
        let o2 = b.intern_iri("http://example.org/o2");
        let o3 = b.intern_iri("http://example.org/o3");
        let o4 = b.intern_iri("http://example.org/o4");
        b.push_quad(s, p1, o1, None);
        b.push_quad(s, p1, o2, None);
        b.push_quad(s, p2, o3, None);
        b.push_quad(s, p2, o4, None);
        let dataset = b.freeze().expect("valid dataset");

        let (dict, bytes) = build_and_open(&dataset);
        let triples = TriplesRef::from_bytes(&bytes).expect("opens");

        let s_id = dict.id_by_value(&iri("s")).expect("present");
        let p1_id = dict.predicate_id_by_value(&iri("p1")).expect("present");
        let p2_id = dict.predicate_id_by_value(&iri("p2")).expect("present");

        assert_eq!(
            triples
                .pattern(Some(s_id), Some(p1_id), None, GraphMatch::Any)
                .count(),
            2
        );
        assert_eq!(
            triples
                .pattern(Some(s_id), Some(p2_id), None, GraphMatch::Any)
                .count(),
            2
        );
        assert_eq!(
            triples
                .pattern(Some(s_id), None, None, GraphMatch::Any)
                .count(),
            4
        );
    }

    #[test]
    fn empty_default_graph_with_only_named_graph_quads() {
        // The default-graph partition (0) must still exist and be queryable
        // (empty), while a named graph carries all the data.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s");
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri("http://example.org/o");
        let g = b.intern_iri("http://example.org/g");
        b.push_quad(s, p, o, Some(g));
        let dataset = b.freeze().expect("valid dataset");

        let (dict, bytes) = build_and_open(&dataset);
        let triples = TriplesRef::from_bytes(&bytes).expect("opens");

        assert_eq!(
            triples
                .pattern(None, None, None, GraphMatch::Default)
                .count(),
            0
        );
        let g_id = dict
            .id_by_value(&iri("g"))
            .expect("graph-name term present");
        assert_eq!(
            triples
                .pattern(None, None, None, GraphMatch::Named(g_id))
                .count(),
            1
        );
        assert_eq!(
            triples.pattern(None, None, None, GraphMatch::Any).count(),
            1
        );
        assert_eq!(triples.named_graph_ids().collect::<Vec<_>>(), vec![g_id]);
    }

    #[test]
    fn absent_unified_id_yields_no_rows() {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s");
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri("http://example.org/o");
        b.push_quad(s, p, o, None);
        let dataset = b.freeze().expect("valid dataset");

        let (dict, bytes) = build_and_open(&dataset);
        let triples = TriplesRef::from_bytes(&bytes).expect("opens");

        // A unified id from a SEPARATE dictionary/dataset addresses nothing here.
        let mut other = RdfDatasetBuilder::new();
        let other_s = other.intern_iri("http://example.org/nowhere");
        let other_p = other.intern_iri("http://example.org/p");
        let other_o = other.intern_iri("http://example.org/o");
        other.push_quad(other_s, other_p, other_o, None);
        let other_dataset = other.freeze().expect("valid dataset");
        let other_dict =
            PackDict::open(&PackDict::encode(&other_dataset).to_bytes()).expect("opens");
        let absent_id = other_dict
            .id_by_value(&iri("nowhere"))
            .expect("present in the other dict");
        // `absent_id` is unified id 1 in ITS dict; whether or not it happens to
        // collide numerically with a real id in `dict`, it must not resolve to
        // "nowhere" in `triples` — the true guarantee is the id-not-found path:
        let never_used = dict.n_terms() + 1;
        assert_eq!(
            triples
                .pattern(Some(never_used), None, None, GraphMatch::Any)
                .count(),
            0
        );
        let _ = absent_id; // silence unused-binding lints while keeping the setup documented
    }

    #[test]
    fn cardinality_upper_bound_never_undershoots_actual_rows() {
        let mut b = RdfDatasetBuilder::new();
        let s1 = b.intern_iri("http://example.org/s1");
        let s2 = b.intern_iri("http://example.org/s2");
        let p = b.intern_iri("http://example.org/p");
        let o1 = b.intern_iri("http://example.org/o1");
        let o2 = b.intern_iri("http://example.org/o2");
        b.push_quad(s1, p, o1, None);
        b.push_quad(s1, p, o2, None);
        b.push_quad(s2, p, o1, None);
        let dataset = b.freeze().expect("valid dataset");

        let (dict, bytes) = build_and_open(&dataset);
        let triples = TriplesRef::from_bytes(&bytes).expect("opens");
        let p_id = dict.predicate_id_by_value(&iri("p")).expect("present");
        let o1_id = dict.id_by_value(&iri("o1")).expect("present");

        for (s, p, o) in [
            (None, None, None),
            (None, Some(p_id), None),
            (None, None, Some(o1_id)),
            (None, Some(p_id), Some(o1_id)),
        ] {
            let actual = triples.pattern(s, p, o, GraphMatch::Any).count();
            let bound = triples.cardinality_upper_bound(s, p, o, GraphMatch::Any);
            assert!(
                bound >= actual,
                "bound {bound} < actual {actual} for ({s:?},{p:?},{o:?})"
            );
        }
    }

    #[test]
    fn to_bytes_round_trips_via_from_bytes() {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s");
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri("http://example.org/o");
        b.push_quad(s, p, o, None);
        let dataset = b.freeze().expect("valid dataset");
        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("opens");
        let encoded = Triples::encode(&dict, &dataset);
        let bytes_a = encoded.to_bytes();
        let bytes_b = encoded.to_bytes();
        assert_eq!(bytes_a, bytes_b, "to_bytes is deterministic");
        let a = TriplesRef::from_bytes(&bytes_a).expect("opens");
        let b_ref = TriplesRef::from_bytes(&bytes_b).expect("opens");
        let rows_a: Vec<_> = a.pattern(None, None, None, GraphMatch::Any).collect();
        let rows_b: Vec<_> = b_ref.pattern(None, None, None, GraphMatch::Any).collect();
        assert_eq!(rows_a, rows_b);
    }

    /// `sum_int_vector` is the private cross-check helper `PartitionRef::from_bytes`
    /// uses to verify a FoQ index's per-entry counts add up to the expected total
    /// (e.g. `pred_counts` summing to `sp.len()`). Built directly from an
    /// [`IntVector`]/[`IntVectorRef`] pair (rather than round-tripped through a
    /// whole encoded pack) because a REAL pack's counts vectors are always sized
    /// off actual (small) triple counts, so no realistic dataset can ever push
    /// their sum past `u64::MAX` — this unit test constructs the overflow
    /// directly, at the width-64 ceiling the format allows, to prove the helper
    /// itself fails closed rather than panicking or wrapping. The end-to-end
    /// byte-tamper proof that a pack whose stored counts overflow is REJECTED
    /// (not merely that this helper returns `None`) lives in
    /// `tests/pack_triples.rs`'s
    /// `from_bytes_rejects_a_triples_index_whose_counts_overflow`.
    #[test]
    fn sum_int_vector_returns_none_on_overflow() {
        let mut v = IntVector::with_width(64);
        v.push(u64::MAX);
        v.push(u64::MAX);
        let bytes = v.to_bytes();
        let vec_ref = IntVectorRef::from_bytes(&bytes).expect("valid int_vector encoding");
        assert_eq!(
            sum_int_vector(vec_ref),
            None,
            "two u64::MAX entries must overflow, not wrap, when summed"
        );
    }

    #[test]
    fn sum_int_vector_returns_some_correct_sum_when_not_overflowing() {
        let mut v = IntVector::with_width(64);
        v.push(3);
        v.push(4);
        v.push(5);
        let bytes = v.to_bytes();
        let vec_ref = IntVectorRef::from_bytes(&bytes).expect("valid int_vector encoding");
        assert_eq!(sum_int_vector(vec_ref), Some(12));
    }

    #[test]
    fn from_bytes_rejects_truncated_input() {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s");
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri("http://example.org/o");
        b.push_quad(s, p, o, None);
        let dataset = b.freeze().expect("valid dataset");
        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("opens");
        let bytes = Triples::encode(&dict, &dataset).to_bytes();
        let err = TriplesRef::from_bytes(&bytes[..bytes.len() - 1]).unwrap_err();
        assert!(matches!(
            err,
            PackTriplesError::Truncated { .. } | PackTriplesError::Malformed(_)
        ));
    }
}
