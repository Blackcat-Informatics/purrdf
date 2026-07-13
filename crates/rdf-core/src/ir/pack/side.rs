// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RDF 1.2 reifier/annotation side-tables (Task 4 of the succinct-pack-codec
//! feature): a self-contained, succinct encoding of `RdfDataset`'s two side
//! tables — [`ReifierRow`](crate::ir::dataset::ReifierRow)s and
//! [`AnnotationRow`](crate::ir::dataset::AnnotationRow)s — over the unified
//! [`PackTermId`] space [`PackDict`] mints, reproducing
//! [`RdfDataset::reifier_quads`], [`RdfDataset::annotation_quads`], and
//! [`RdfDataset::annotations_of_with_graph`] byte-for-byte (as a SET of
//! `TermValue`-resolved rows; see the exact mapping below).
//!
//! # The exact source mapping this module reproduces
//!
//! `RdfDataset::reifier_quads` (`crates/rdf-core/src/ir/dataset.rs`) maps each
//! frozen `(reifier, triple, graph)` [`ReifierRow`](crate::ir::dataset::ReifierRow)
//! to `QuadIds { s: reifier, p: reifies, o: triple, g: graph }`, where `reifies`
//! is the interned id of the constant IRI `rdf:reifies` — looked up by VALUE,
//! not stored in the row tuple itself, and present iff the dataset has at least
//! one reifier binding (the ingest path interns it as the serialized
//! indirection edge `reifier rdf:reifies <<( s p o )>>`). `annotation_quads`
//! maps each `(reifier, predicate, object, graph)`
//! [`AnnotationRow`](crate::ir::dataset::AnnotationRow) to `QuadIds { s: reifier,
//! p: predicate, o: object, g: graph }` directly. `annotations_of_with_graph`
//! yields `(predicate, object, graph)` for one `reifier`, found via
//! `partition_point` over the annotation table (frozen sorted primarily by
//! `reifier`) — `O(log n)` to locate the run, then a contiguous scan of it.
//!
//! [`SideTablesRef::reifier_quads`]/[`annotation_quads`](SideTablesRef::annotation_quads)/
//! [`annotations_of_with_graph`](SideTablesRef::annotations_of_with_graph)
//! reproduce these three exactly, over unified [`PackTermId`]s instead of
//! dataset-local `TermId`s (a value-level comparison — resolving both sides
//! through `term_value`/`PackDict::term_value` — is therefore expected to
//! agree as a SET; see `tests/pack_side.rs`).
//!
//! # Row order and dedup
//!
//! [`PackDict::encode`]'s Task 4 amendment folds every side-table-referenced
//! term (including `rdf:reifies`, when reifiers are non-empty) into the
//! dictionary, so every reifier/annotation row resolves to unified ids here.
//! `RdfDataset` already deduplicates both side tables at freeze (C0.5); this
//! module re-sorts the translated rows into a CANONICAL order over unified ids
//! — `(reifier_uni, triple_uni, graph_uni)` for reifier rows and
//! `(reifier_uni, predicate_uni, object_uni, graph_uni)` for annotation rows —
//! rather than preserving the source's `TermId`-based order. This is
//! deliberate: a `TermId`'s numeric value is an artifact of interning order
//! (parse order), not a canonical property of the dataset's VALUES, so sorting
//! by it would make [`SideTables::encode`]'s output depend on ingestion order
//! — violating the byte-determinism discipline every other `pack` codec in this
//! tree follows (PFC dictionary sections, bitmap-triples partitions: both sort
//! by canonical `TermValue`/unified-id order, never by `TermId`). Encode-time
//! dedup on the SAME tuple this module reads/writes is `sort_unstable` +
//! `dedup`, a defensive no-op given the source already deduplicates by value.
//!
//! # Storage
//!
//! Two flat column-major tables, each column a bit-packed [`IntVector`]:
//!
//! - **Reifier rows** — `reifier_reifier`/`reifier_triple`/`reifier_graph`
//!   (`0` sentinel for "no graph", matching [`super::triples`]'s
//!   `graph_id_or_zero` convention — unified ids are 1-based so `0` never
//!   collides with a real one).
//! - **Annotation rows** — grouped and sorted by `reifier_uni` (primary key),
//!   `predicate_uni`/`object_uni`/`graph_uni` (same `0` sentinel). A CSR-style
//!   per-reifier index — `local_reifier` (the ascending, DISTINCT `reifier_uni`
//!   values that own at least one annotation) paired with `annotation_offsets`/
//!   `annotation_counts` — lets [`annotations_of_with_graph`](SideTablesRef::annotations_of_with_graph)
//!   binary-search `local_reifier` (`O(log distinct_reifiers)`) and then slice
//!   the annotation columns directly, rather than a linear scan or a
//!   `partition_point` over the full (potentially much larger) annotation
//!   table — the "fast slice, not a full scan" requirement.
//!
//! `SideTablesRef::from_bytes` fails closed: every structural invariant (column
//! lengths agreeing, `local_reifier` strictly ascending, every non-optional id
//! column nonzero, the offset/count index being an exact prefix-sum of the row
//! counts) is checked once at open time, so a later query never panics on a
//! successfully-opened buffer.

use std::cmp::Ordering;
use std::fmt;

use crate::{RdfDataset, RdfStoreCapabilities, TermValue};

use super::bits::{IntVector, IntVectorRef, PackBitsError, bits_for};
use super::dict::{PackDict, PackTermId};

/// The `rdf:reifies` predicate IRI — see the identical local constant (and its
/// doc comment explaining the duplication) in `super::dict`.
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Why decoding a [`SideTables`] byte buffer failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackSideError {
    /// The buffer ended before all the bytes a header promised were present.
    Truncated {
        /// The total leading byte count the format required.
        needed: usize,
        /// The byte count actually available.
        found: usize,
    },
    /// The buffer's header was internally inconsistent, an id/offset reference
    /// fell outside its documented domain, or the offset/count index disagreed
    /// with the row data it is supposed to address.
    Malformed(&'static str),
}

impl fmt::Display for PackSideError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { needed, found } => write!(
                f,
                "pack-side: truncated input: needed at least {needed} bytes, found {found}"
            ),
            Self::Malformed(reason) => write!(f, "pack-side: malformed input: {reason}"),
        }
    }
}

impl std::error::Error for PackSideError {}

impl From<PackBitsError> for PackSideError {
    fn from(e: PackBitsError) -> Self {
        match e {
            PackBitsError::Truncated { needed, found } => Self::Truncated { needed, found },
            PackBitsError::Malformed(reason) => Self::Malformed(reason),
        }
    }
}

/// Read an 8-byte little-endian header field at `*pos`, advancing `*pos` past it.
/// A small local mirror of `bits::read_header_u64` (private to that module).
fn read_u64_header(bytes: &[u8], pos: &mut usize) -> Result<u64, PackSideError> {
    let end = *pos + 8;
    let slice = bytes.get(*pos..end).ok_or(PackSideError::Truncated {
        needed: end,
        found: bytes.len(),
    })?;
    let value = u64::from_le_bytes(slice.try_into().expect("slice is exactly 8 bytes"));
    *pos = end;
    Ok(value)
}

/// Build a bit-packed [`IntVector`] wide enough for `values`' maximum element —
/// a local mirror of `triples::build_int_vector` (private to that module).
fn build_int_vector(values: &[u64]) -> IntVector {
    let max = values.iter().copied().max().unwrap_or(0);
    let mut v = IntVector::with_width(bits_for(max));
    for &x in values {
        v.push(x);
    }
    v
}

/// Resolve `value` to its unified [`PackTermId`], preferring a non-predicate
/// role and falling back to the predicate-section id — the SAME preference
/// order [`PackDict`]'s own "Lookup rule" documents for resolving a structural
/// reference (a literal's datatype, a triple term's components): prefer
/// `id_by_value`, fall back to `predicate_id_by_value` only if the value has no
/// non-predicate role. A side-table reference (a reifier, a reified
/// triple-term, an annotation predicate/object, a graph name) is exactly such a
/// structural reference, so it follows the same rule.
///
/// # Panics
///
/// Panics if `value` resolves to neither section — a caller-side contract
/// violation: `dict` MUST be [`PackDict::encode`]'s output for the SAME
/// `dataset` `value` was read from (its Task 4 side-table closure amendment
/// guarantees every such reference resolves).
fn resolve_any(dict: &PackDict, value: &TermValue) -> PackTermId {
    dict.id_by_value(value)
        .or_else(|| dict.predicate_id_by_value(value))
        .expect(
            "PackDict::encode's side-table closure amendment guarantees every reifier/annotation \
             term (and rdf:reifies, when reifiers are non-empty) resolves to a unified id",
        )
}

/// Verify `map`'s stored values are strictly ascending — the invariant
/// [`local_lookup`]'s binary search depends on. Mirrors
/// `triples::assert_strictly_ascending` (private to that module).
fn assert_strictly_ascending(
    map: IntVectorRef<'_>,
    what: &'static str,
) -> Result<(), PackSideError> {
    let mut prev: Option<u64> = None;
    for i in 0..map.len() {
        let cur = map.get(i);
        if let Some(p) = prev
            && cur <= p
        {
            return Err(PackSideError::Malformed(what));
        }
        prev = Some(cur);
    }
    Ok(())
}

/// Verify every value in `vec` is nonzero — every id column here EXCEPT the
/// graph columns (which reserve `0` as the "no graph" sentinel) holds a real,
/// 1-based unified [`PackTermId`], so a stored `0` can only be corruption.
fn assert_nonzero(vec: IntVectorRef<'_>, what: &'static str) -> Result<(), PackSideError> {
    for i in 0..vec.len() {
        if vec.get(i) == 0 {
            return Err(PackSideError::Malformed(what));
        }
    }
    Ok(())
}

/// Binary-search `map` (ascending) for `unified`, returning its index. Mirrors
/// `triples::local_lookup` (private to that module).
fn local_lookup(map: IntVectorRef<'_>, unified: PackTermId) -> Option<usize> {
    let mut lo = 0usize;
    let mut hi = map.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        match map.get(mid).cmp(&unified) {
            Ordering::Less => lo = mid + 1,
            Ordering::Greater => hi = mid,
            Ordering::Equal => return Some(mid),
        }
    }
    None
}

/// Decode a `0`-sentinel graph column entry to `Option<PackTermId>` (`0` ⇒
/// `None`, the default graph — unified ids are 1-based so this never collides
/// with a real graph id).
fn decode_graph(raw: u64) -> Option<PackTermId> {
    if raw == 0 { None } else { Some(raw) }
}

// ---------------------------------------------------------------------------
// SideTables — the owned, self-contained encoded form.
// ---------------------------------------------------------------------------

/// The on-disk format version [`SideTables::encode`] writes and
/// [`SideTablesRef::from_bytes`] requires.
const SIDE_FORMAT_VERSION: u8 = 1;

/// The output of [`SideTables::encode`]: the self-contained, versioned byte
/// buffer [`SideTablesRef::from_bytes`] reads. See the [module docs](self) for
/// the exact source mapping and on-disk layout.
#[derive(Debug, Clone)]
pub struct SideTables {
    bytes: Vec<u8>,
}

impl SideTables {
    /// Scan `dataset`'s reifier and annotation side-tables and build the
    /// self-contained, unified-id encoding (see the [module docs](self)).
    /// `dict` MUST be [`PackDict::encode`]'s output for this exact `dataset`
    /// (its Task 4 side-table closure amendment guarantees every reference
    /// resolves) — see [`resolve_any`].
    ///
    /// # Panics
    ///
    /// Panics (via [`resolve_any`]'s `expect`) if `dict` was not built from
    /// `dataset` — a caller-side contract violation, not a data-dependent error.
    #[must_use]
    pub fn encode(dict: &PackDict, dataset: &RdfDataset) -> Self {
        let mut reifier_rows: Vec<(PackTermId, PackTermId, PackTermId)> = dataset
            .reifiers_with_graph()
            .map(|(reifier, triple, graph)| {
                let r = resolve_any(dict, &dataset.term_value(reifier));
                let t = resolve_any(dict, &dataset.term_value(triple));
                let g = graph.map_or(0, |g| resolve_any(dict, &dataset.term_value(g)));
                (r, t, g)
            })
            .collect();
        reifier_rows.sort_unstable();
        reifier_rows.dedup();

        let mut annotation_rows: Vec<(PackTermId, PackTermId, PackTermId, PackTermId)> = dataset
            .annotations_with_graph()
            .map(|(reifier, pred, obj, graph)| {
                let r = resolve_any(dict, &dataset.term_value(reifier));
                let p = resolve_any(dict, &dataset.term_value(pred));
                let o = resolve_any(dict, &dataset.term_value(obj));
                let g = graph.map_or(0, |g| resolve_any(dict, &dataset.term_value(g)));
                (r, p, o, g)
            })
            .collect();
        annotation_rows.sort_unstable();
        annotation_rows.dedup();

        // The virtual `reifies` predicate: present iff at least one reifier row
        // exists (see the [`RDF_REIFIES`] doc comment).
        let reifies_predicate = if reifier_rows.is_empty() {
            0
        } else {
            resolve_any(dict, &TermValue::Iri(RDF_REIFIES.to_owned()))
        };

        // CSR-style per-reifier grouping: `annotation_rows` is already sorted by
        // its first (`reifier_uni`) key, so one linear pass finds every group's
        // extent.
        let mut local_reifier: Vec<u64> = Vec::new();
        let mut offsets: Vec<u64> = Vec::new();
        let mut counts: Vec<u64> = Vec::new();
        let mut i = 0usize;
        while i < annotation_rows.len() {
            let r = annotation_rows[i].0;
            let start = i;
            while i < annotation_rows.len() && annotation_rows[i].0 == r {
                i += 1;
            }
            local_reifier.push(r);
            offsets.push(start as u64);
            counts.push((i - start) as u64);
        }

        let mut out = Vec::new();
        out.push(SIDE_FORMAT_VERSION);
        out.extend_from_slice(&reifies_predicate.to_le_bytes());
        out.extend_from_slice(
            &build_int_vector(&reifier_rows.iter().map(|r| r.0).collect::<Vec<_>>()).to_bytes(),
        );
        out.extend_from_slice(
            &build_int_vector(&reifier_rows.iter().map(|r| r.1).collect::<Vec<_>>()).to_bytes(),
        );
        out.extend_from_slice(
            &build_int_vector(&reifier_rows.iter().map(|r| r.2).collect::<Vec<_>>()).to_bytes(),
        );
        out.extend_from_slice(&build_int_vector(&local_reifier).to_bytes());
        out.extend_from_slice(&build_int_vector(&offsets).to_bytes());
        out.extend_from_slice(&build_int_vector(&counts).to_bytes());
        out.extend_from_slice(
            &build_int_vector(&annotation_rows.iter().map(|r| r.1).collect::<Vec<_>>()).to_bytes(),
        );
        out.extend_from_slice(
            &build_int_vector(&annotation_rows.iter().map(|r| r.2).collect::<Vec<_>>()).to_bytes(),
        );
        out.extend_from_slice(
            &build_int_vector(&annotation_rows.iter().map(|r| r.3).collect::<Vec<_>>()).to_bytes(),
        );

        Self { bytes: out }
    }

    /// The serialized byte buffer [`SideTablesRef::from_bytes`] reads.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }
}

// ---------------------------------------------------------------------------
// SideTablesRef — the borrowed, zero-copy reader.
// ---------------------------------------------------------------------------

/// The borrowed, zero-copy reader over [`SideTables::to_bytes`]'s output: every
/// column aliases `bytes` directly (no allocation, no copy). See the
/// [module docs](self) for the exact source mapping and on-disk layout.
#[derive(Debug, Clone, Copy)]
pub struct SideTablesRef<'a> {
    /// The unified id of `rdf:reifies`, or `0` if the reifier table is empty
    /// (no query ever reads this in that case — [`reifier_quads`](Self::reifier_quads)'s
    /// iterator is empty too).
    reifies_predicate: PackTermId,
    reifier_reifier: IntVectorRef<'a>,
    reifier_triple: IntVectorRef<'a>,
    reifier_graph: IntVectorRef<'a>,
    /// Ascending, DISTINCT `reifier_uni` values that own `>= 1` annotation.
    local_reifier: IntVectorRef<'a>,
    /// Per `local_reifier` entry: the start index into the annotation columns.
    annotation_offsets: IntVectorRef<'a>,
    /// Per `local_reifier` entry: the row count at that start index.
    annotation_counts: IntVectorRef<'a>,
    annotation_pred: IntVectorRef<'a>,
    annotation_obj: IntVectorRef<'a>,
    annotation_graph: IntVectorRef<'a>,
}

impl<'a> SideTablesRef<'a> {
    /// Parse [`SideTables::to_bytes`]'s output.
    ///
    /// # Errors
    ///
    /// [`PackSideError`] on truncation or any structural inconsistency (see the
    /// [module docs](self) for what is validated).
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, PackSideError> {
        let version = *bytes.first().ok_or(PackSideError::Truncated {
            needed: 1,
            found: 0,
        })?;
        if version != SIDE_FORMAT_VERSION {
            return Err(PackSideError::Malformed("side: unsupported format version"));
        }
        let mut pos = 1usize;
        let reifies_predicate = read_u64_header(bytes, &mut pos)?;

        let reifier_reifier = IntVectorRef::from_bytes(&bytes[pos..])?;
        pos += reifier_reifier.serialized_len();
        let reifier_triple = IntVectorRef::from_bytes(&bytes[pos..])?;
        pos += reifier_triple.serialized_len();
        let reifier_graph = IntVectorRef::from_bytes(&bytes[pos..])?;
        pos += reifier_graph.serialized_len();
        let local_reifier = IntVectorRef::from_bytes(&bytes[pos..])?;
        pos += local_reifier.serialized_len();
        let annotation_offsets = IntVectorRef::from_bytes(&bytes[pos..])?;
        pos += annotation_offsets.serialized_len();
        let annotation_counts = IntVectorRef::from_bytes(&bytes[pos..])?;
        pos += annotation_counts.serialized_len();
        let annotation_pred = IntVectorRef::from_bytes(&bytes[pos..])?;
        pos += annotation_pred.serialized_len();
        let annotation_obj = IntVectorRef::from_bytes(&bytes[pos..])?;
        pos += annotation_obj.serialized_len();
        let annotation_graph = IntVectorRef::from_bytes(&bytes[pos..])?;
        pos += annotation_graph.serialized_len();
        let _ = pos; // no trailing-garbage check: a later container may frame more after us.

        // -- Structural validation (fail-closed) -----------------------------
        if reifier_triple.len() != reifier_reifier.len()
            || reifier_graph.len() != reifier_reifier.len()
        {
            return Err(PackSideError::Malformed(
                "side: reifier column lengths disagree",
            ));
        }
        if annotation_offsets.len() != local_reifier.len()
            || annotation_counts.len() != local_reifier.len()
        {
            return Err(PackSideError::Malformed(
                "side: annotation index length disagrees with local_reifier",
            ));
        }
        if annotation_obj.len() != annotation_pred.len()
            || annotation_graph.len() != annotation_pred.len()
        {
            return Err(PackSideError::Malformed(
                "side: annotation column lengths disagree",
            ));
        }

        assert_strictly_ascending(
            local_reifier,
            "side: local_reifier map is not strictly ascending",
        )?;
        assert_nonzero(reifier_reifier, "side: reifier id is zero")?;
        assert_nonzero(reifier_triple, "side: reifier triple-term id is zero")?;
        assert_nonzero(local_reifier, "side: local_reifier id is zero")?;
        assert_nonzero(annotation_pred, "side: annotation predicate id is zero")?;
        assert_nonzero(annotation_obj, "side: annotation object id is zero")?;

        if reifies_predicate == 0 && !reifier_reifier.is_empty() {
            return Err(PackSideError::Malformed(
                "side: reifies-predicate id missing despite a non-empty reifier table",
            ));
        }
        if reifies_predicate != 0 && reifier_reifier.is_empty() {
            return Err(PackSideError::Malformed(
                "side: reifies-predicate id present despite an empty reifier table",
            ));
        }

        // The offset/count index must be an EXACT prefix sum of the counts
        // (this both proves the offsets are monotone and cross-checks them
        // against the counts in one pass).
        let mut running = 0u64;
        for i in 0..local_reifier.len() {
            if annotation_offsets.get(i) != running {
                return Err(PackSideError::Malformed(
                    "side: annotation offsets are not a correct prefix sum of the counts",
                ));
            }
            running =
                running
                    .checked_add(annotation_counts.get(i))
                    .ok_or(PackSideError::Malformed(
                        "side: annotation offset overflows u64",
                    ))?;
        }
        if running != annotation_pred.len() as u64 {
            return Err(PackSideError::Malformed(
                "side: annotation counts do not sum to the annotation row count",
            ));
        }

        Ok(Self {
            reifies_predicate,
            reifier_reifier,
            reifier_triple,
            reifier_graph,
            local_reifier,
            annotation_offsets,
            annotation_counts,
            annotation_pred,
            annotation_obj,
            annotation_graph,
        })
    }

    /// The number of reifier rows.
    #[must_use]
    pub fn reifier_count(&self) -> usize {
        self.reifier_reifier.len()
    }

    /// The number of annotation rows.
    #[must_use]
    pub fn annotation_count(&self) -> usize {
        self.annotation_pred.len()
    }

    /// Every reifier binding as a `(reifier, rdf:reifies, triple, graph)` row of
    /// unified ids — the unified-id twin of `RdfDataset::reifier_quads` (see the
    /// exact mapping in the [module docs](self)).
    pub fn reifier_quads(
        &self,
    ) -> impl Iterator<Item = (PackTermId, PackTermId, PackTermId, Option<PackTermId>)> + '_ {
        let reifies = self.reifies_predicate;
        (0..self.reifier_reifier.len()).map(move |i| {
            (
                self.reifier_reifier.get(i),
                reifies,
                self.reifier_triple.get(i),
                decode_graph(self.reifier_graph.get(i)),
            )
        })
    }

    /// Every statement annotation as a `(reifier, predicate, object, graph)` row
    /// of unified ids — the unified-id twin of `RdfDataset::annotation_quads`
    /// (see the exact mapping in the [module docs](self)).
    pub fn annotation_quads(
        &self,
    ) -> impl Iterator<Item = (PackTermId, PackTermId, PackTermId, Option<PackTermId>)> + '_ {
        (0..self.local_reifier.len()).flat_map(move |g| {
            let reifier = self.local_reifier.get(g);
            let start = self.annotation_offsets.get(g) as usize;
            let count = self.annotation_counts.get(g) as usize;
            (start..start + count).map(move |i| {
                (
                    reifier,
                    self.annotation_pred.get(i),
                    self.annotation_obj.get(i),
                    decode_graph(self.annotation_graph.get(i)),
                )
            })
        })
    }

    /// The `(predicate, object, graph)` annotations attached to `reifier` — the
    /// unified-id twin of `RdfDataset::annotations_of_with_graph`.
    /// `O(log distinct_reifiers)` to locate the group (binary search over
    /// `local_reifier`), then a direct slice of the annotation columns — never a
    /// full scan.
    pub fn annotations_of_with_graph(
        &self,
        reifier: PackTermId,
    ) -> impl Iterator<Item = (PackTermId, PackTermId, Option<PackTermId>)> + '_ {
        let range = local_lookup(self.local_reifier, reifier)
            .map(|g| {
                let start = self.annotation_offsets.get(g) as usize;
                let count = self.annotation_counts.get(g) as usize;
                start..start + count
            })
            .unwrap_or(0..0);
        range.map(move |i| {
            (
                self.annotation_pred.get(i),
                self.annotation_obj.get(i),
                decode_graph(self.annotation_graph.get(i)),
            )
        })
    }

    /// `true` iff any reifier or annotation row carries a named-graph slot
    /// (`graph != None`) — used by [`capabilities`] to compute the
    /// `named_graphs` flag over side-table-only graph references (a reifier or
    /// annotation MAY be declared inside a `GRAPH g { … }` block that owns no
    /// base quad of its own, so [`super::triples::TriplesRef`]'s partitions
    /// alone would miss it).
    fn has_graph_reference(&self) -> bool {
        (0..self.reifier_graph.len()).any(|i| self.reifier_graph.get(i) != 0)
            || (0..self.annotation_graph.len()).any(|i| self.annotation_graph.get(i) != 0)
    }
}

/// Compute the pack's [`RdfStoreCapabilities`] flags for the four fields the
/// side-table + dictionary determine — mirrors `RdfDataset`'s own
/// `compute_capabilities` (`crates/rdf-core/src/ir/builder.rs`):
///
/// - `named_graphs` — `true` iff any BASE quad names a graph (`base_named_graphs`,
///   supplied by the caller from [`super::triples::TriplesRef::named_graph_ids`])
///   OR any reifier/annotation row does (a side-table-only named graph, which
///   owns no base quad and so has no `TriplesRef` partition of its own).
/// - `quoted_triples` — `true` iff any triple term exists in `dict`
///   ([`PackDict::has_triple_term`]).
/// - `reifiers` — `true` iff `side` holds at least one reifier row.
/// - `annotations` — `true` iff `side` holds at least one annotation row.
///
/// `source_locations`/`loss_records`/`lookaside` are NOT computed here — the
/// pack format preserves none of that sidecar material, so a caller (Task 6's
/// `DatasetView::capabilities`) sets those to `false` (or ORs in whatever ITS
/// own container format tracks) rather than this function fabricating a value
/// for a concern side-tables/dictionary have no visibility into.
#[must_use]
pub fn capabilities(
    dict: &PackDict,
    side: &SideTablesRef<'_>,
    base_named_graphs: bool,
) -> RdfStoreCapabilities {
    RdfStoreCapabilities {
        named_graphs: base_named_graphs || side.has_graph_reference(),
        quoted_triples: dict.has_triple_term(),
        reifiers: side.reifier_count() > 0,
        annotations: side.annotation_count() > 0,
        source_locations: false,
        loss_records: false,
        lookaside: false,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RdfDatasetBuilder, TermId};
    use std::collections::HashSet;

    fn iri(name: &str) -> TermValue {
        TermValue::iri(format!("http://example.org/{name}"))
    }

    /// Intern one dataset-independent value into a builder, recursing for triple
    /// terms (mirrors `dict.rs`'s test helper of the same name).
    fn intern_value(b: &mut RdfDatasetBuilder, v: &TermValue) -> TermId {
        match v {
            TermValue::Iri(s) => b.intern_iri(s),
            TermValue::Blank { label, scope } => b.intern_blank(label, *scope),
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            } => b.intern_literal(crate::RdfLiteral {
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

    /// Build+encode a dataset with one reifier binding `r rdf:reifies << s p o >>`
    /// and two annotations on `r`, one in the default graph and one in `g1`.
    fn build_fixture() -> (std::sync::Arc<RdfDataset>, PackDict, Vec<u8>) {
        let mut b = RdfDatasetBuilder::new();
        let s = intern_value(&mut b, &iri("s"));
        let p = intern_value(&mut b, &iri("p"));
        let o = intern_value(&mut b, &iri("o"));
        let triple = b.intern_triple(s, p, o);
        let r = intern_value(&mut b, &iri("r"));
        let g1 = intern_value(&mut b, &iri("g1"));
        b.push_reifier(r, triple);
        let ap1 = intern_value(&mut b, &iri("ap1"));
        let ao1 = intern_value(&mut b, &iri("ao1"));
        b.push_annotation(r, ap1, ao1);
        let ap2 = intern_value(&mut b, &iri("ap2"));
        let ao2 = intern_value(&mut b, &iri("ao2"));
        b.push_annotation_in_graph(r, ap2, ao2, Some(g1));
        let dataset = b.freeze().expect("valid dataset");

        let dict_bytes = PackDict::encode(&dataset).to_bytes();
        let dict = PackDict::open(&dict_bytes).expect("dict opens");
        let side_bytes = SideTables::encode(&dict, &dataset).to_bytes();
        (dataset, dict, side_bytes)
    }

    type ValueQuad = (TermValue, TermValue, TermValue, Option<TermValue>);

    fn to_value_quads(
        dict: &PackDict,
        rows: impl Iterator<Item = (PackTermId, PackTermId, PackTermId, Option<PackTermId>)>,
    ) -> HashSet<ValueQuad> {
        rows.map(|(s, p, o, g)| {
            (
                dict.term_value(s),
                dict.term_value(p),
                dict.term_value(o),
                g.map(|id| dict.term_value(id)),
            )
        })
        .collect()
    }

    #[test]
    fn reifier_quads_matches_source() {
        let (dataset, dict, bytes) = build_fixture();
        let side = SideTablesRef::from_bytes(&bytes).expect("opens");

        let expected: HashSet<ValueQuad> = dataset
            .reifier_quads()
            .map(|q| {
                (
                    dataset.term_value(q.s),
                    dataset.term_value(q.p),
                    dataset.term_value(q.o),
                    q.g.map(|g| dataset.term_value(g)),
                )
            })
            .collect();
        let actual = to_value_quads(&dict, side.reifier_quads());
        assert_eq!(actual, expected);
        assert_eq!(
            actual,
            HashSet::from([(
                iri("r"),
                TermValue::Iri(RDF_REIFIES.to_owned()),
                TermValue::Triple {
                    s: Box::new(iri("s")),
                    p: Box::new(iri("p")),
                    o: Box::new(iri("o")),
                },
                None,
            )])
        );
    }

    #[test]
    fn annotation_quads_matches_source() {
        let (dataset, dict, bytes) = build_fixture();
        let side = SideTablesRef::from_bytes(&bytes).expect("opens");

        let expected: HashSet<ValueQuad> = dataset
            .annotation_quads()
            .map(|q| {
                (
                    dataset.term_value(q.s),
                    dataset.term_value(q.p),
                    dataset.term_value(q.o),
                    q.g.map(|g| dataset.term_value(g)),
                )
            })
            .collect();
        let actual = to_value_quads(&dict, side.annotation_quads());
        assert_eq!(actual, expected);
        assert_eq!(actual.len(), 2);
    }

    #[test]
    fn annotations_of_with_graph_matches_source() {
        let (dataset, dict, bytes) = build_fixture();
        let side = SideTablesRef::from_bytes(&bytes).expect("opens");

        let r_value = iri("r");
        let r_dataset_id = dataset.term_id_by_value(&r_value).expect("interned");
        let r_pack_id = dict.id_by_value(&r_value).expect("in dict");

        let expected: HashSet<(TermValue, TermValue, Option<TermValue>)> = dataset
            .annotations_of_with_graph(r_dataset_id)
            .map(|(p, o, g)| {
                (
                    dataset.term_value(p),
                    dataset.term_value(o),
                    g.map(|g| dataset.term_value(g)),
                )
            })
            .collect();
        let actual: HashSet<(TermValue, TermValue, Option<TermValue>)> = side
            .annotations_of_with_graph(r_pack_id)
            .map(|(p, o, g)| {
                (
                    dict.term_value(p),
                    dict.term_value(o),
                    g.map(|g| dict.term_value(g)),
                )
            })
            .collect();
        assert_eq!(actual, expected);
        assert_eq!(actual.len(), 2);
    }

    #[test]
    fn annotations_of_with_graph_empty_for_unknown_reifier() {
        let (_, dict, bytes) = build_fixture();
        let side = SideTablesRef::from_bytes(&bytes).expect("opens");
        let other = dict.id_by_value(&iri("s")).expect("in dict"); // never a reifier
        assert_eq!(side.annotations_of_with_graph(other).count(), 0);
    }

    #[test]
    fn empty_dataset_yields_empty_side_tables() {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s");
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri("http://example.org/o");
        b.push_quad(s, p, o, None);
        let dataset = b.freeze().expect("valid dataset");
        let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("dict opens");
        let bytes = SideTables::encode(&dict, &dataset).to_bytes();
        let side = SideTablesRef::from_bytes(&bytes).expect("opens");

        assert_eq!(side.reifier_count(), 0);
        assert_eq!(side.annotation_count(), 0);
        assert_eq!(side.reifier_quads().count(), 0);
        assert_eq!(side.annotation_quads().count(), 0);

        let caps = capabilities(&dict, &side, false);
        assert!(!caps.named_graphs);
        assert!(!caps.quoted_triples);
        assert!(!caps.reifiers);
        assert!(!caps.annotations);
    }

    #[test]
    fn round_trip_is_byte_deterministic() {
        let (dataset, dict, bytes) = build_fixture();
        let bytes2 = SideTables::encode(&dict, &dataset).to_bytes();
        assert_eq!(bytes, bytes2, "encode is deterministic");
        SideTablesRef::from_bytes(&bytes).expect("opens");
    }

    #[test]
    fn capabilities_match_source_for_the_fixture() {
        let (dataset, dict, bytes) = build_fixture();
        let side = SideTablesRef::from_bytes(&bytes).expect("opens");
        let expected = dataset.capabilities();
        let actual = capabilities(&dict, &side, false);
        assert_eq!(actual.reifiers, expected.reifiers);
        assert_eq!(actual.annotations, expected.annotations);
        assert_eq!(actual.quoted_triples, expected.quoted_triples);
        // The fixture's g1 named graph is referenced ONLY by an annotation row
        // (no base quad names it), proving the side-table-only path matters.
        assert_eq!(actual.named_graphs, expected.named_graphs);
        assert!(actual.named_graphs);
    }

    #[test]
    fn from_bytes_rejects_truncated_input() {
        let (_, _, bytes) = build_fixture();
        let err = SideTablesRef::from_bytes(&bytes[..bytes.len() - 1]).unwrap_err();
        assert!(matches!(
            err,
            PackSideError::Truncated { .. } | PackSideError::Malformed(_)
        ));
    }

    #[test]
    fn from_bytes_rejects_bad_version() {
        let (_, _, mut bytes) = build_fixture();
        bytes[0] = 0xFF;
        let err = SideTablesRef::from_bytes(&bytes).unwrap_err();
        assert!(matches!(err, PackSideError::Malformed(_)));
    }
}
