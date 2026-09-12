// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The pyo3-free GTS snapshot compose core (P6).
//!
//! This is the byte-emitting heart of `src/purrdf_tools/gts_producer.py::_Builder`,
//! lifted out of the Python binding surface so the non-python
//! Rust consumers (`purrdf-pipeline`) can author a full multi-named-graph `dist`
//! snapshot — default graph + named graphs + RDF-1.2 reifier/annotation tables +
//! content-addressed blobs — without pulling pyo3.
//!
//! [`SnapshotBuilder`] interns terms (append-order, scope-aware blank nodes),
//! content-sorts them (`(kind, value, datatype-IRI, lang, direction)`, IRIs first), and
//! [`emit_gts`] authors the single `dist`-profile `snapshot` frame preceded by the
//! blob frames (sorted by `(rep, decoded-bytes)`). All CBOR encoding,
//! canonicalization, frame-id chaining, and signing is delegated to `purrdf-gts`.
//!
//! The Python wrapper delegates to THIS core; there is one
//! definition of "the snapshot".

use std::collections::BTreeMap;
use std::hash::BuildHasher;

use ciborium::value::Value;
use hashbrown::HashTable;
use purrdf_gts::model::{AnnotationRow, ReifierRow, Term, TermKind, is_literal_direction};
use purrdf_gts::wire::{blake3_256, canonical, hex};
use purrdf_gts::writer::Writer;

use crate::{
    BlankScope, DatasetView, DrainCheckpoint, FallibleDatasetView, FastHasher, RdfTextDirection,
    TermRef, checkpointed_drain,
};

/// The `rdf:reifies` predicate IRI (RDF 1.2 statement layer).
pub const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
/// Payloads larger than this select `zstd-rsyncable` over `zstd`.
pub const DEFAULT_RSYNCABLE_THRESHOLD: usize = 65536;
/// zstd compression level for the committed `dist` bundle's frames (purrdf-gts 0.9.11
/// per-frame level). The writer's `Fastest` default left the rsyncable bundle at
/// 27 MB; level 12 is the measured knee.
///
/// MEASURED 2026-06-29 (dist `purrdf.gts`, sink = the terminal stage):
///   Fastest : 27.1 MB,  sink ~7.0s
///   level 12: 18.6 MB,  sink ~7.1s   ← here: −31% size for ~0 added sink time
///   level 19: 17.7 MB,  sink ~44s    (+37s for only 0.9 MB more — not worth it)
/// rsyncable (set at the `dist` call site) already gives stable git deltas at any
/// level; 12 also shrinks the absolute blob/working-tree size essentially for free.
pub const DIST_ZSTD_LEVEL: i32 = 12;

/// A remapped quad row in canonical term ids (`g == None` is the default graph).
type CanonQuad = (usize, usize, usize, Option<usize>);
/// The fully canonical snapshot tables (`_Builder._canonical_tables`).
type CanonTables = (
    Vec<Term>,
    Vec<CanonQuad>,
    Vec<ReifierRow>,
    Vec<AnnotationRow>,
);

/// One interned term plus its content-sort key. Mirrors `gts.model.Term` rows
/// in the Python `_Interner`, but carries the datatype as the IRI STRING (the
/// post-canonicalization id is assigned later) so the sort key is value-stable.
#[derive(Clone, Debug)]
struct TermRow {
    kind: TermKind,
    value: String,
    /// The datatype IRI string for a typed literal (interned later as a term).
    datatype: Option<String>,
    lang: Option<String>,
    /// RDF 1.2 literal base direction (`"ltr"` / `"rtl"`).
    ///
    /// Part of the term's IDENTITY, not decoration: `"Cat"@en--ltr` and
    /// `"Cat"@en--rtl` are different terms, so this participates in both the
    /// intern key and the content-sort key. Omitting it from the key merged them
    /// into one term and silently rewrote the graph.
    direction: Option<String>,
}

/// The borrowed intern key of a NON-blank term row:
/// `(kind, value, datatype-or-empty, lang-or-empty, direction-or-empty)`.
///
/// Borrowed, not owned: this is the key a per-row LOOKUP hashes and compares,
/// and a lookup that owned its key allocated five strings per ingested term
/// position just to discover the term was already interned. Insertion of a
/// genuinely new term still owns its strings — that is the dictionary itself,
/// not scratch.
type RowKey<'a> = (u8, &'a str, &'a str, &'a str, &'a str);

/// The borrowed intern key of an already-stored row.
fn row_key_of(row: &TermRow) -> RowKey<'_> {
    (
        row.kind as u8,
        row.value.as_str(),
        row.datatype.as_deref().unwrap_or(""),
        row.lang.as_deref().unwrap_or(""),
        row.direction.as_deref().unwrap_or(""),
    )
}

/// The fixed-key hash of a borrowed row key (see [`FastHasher`]).
fn row_key_hash(key: RowKey<'_>) -> u64 {
    FastHasher::default().hash_one(key)
}

/// Hash-consed lookup of a non-blank term row: hash the borrowed components,
/// then confirm the candidate bucket by borrowed equality. Allocates nothing.
fn lookup_term_row(terms: &[TermRow], index: &HashTable<u32>, key: RowKey<'_>) -> Option<usize> {
    index
        .find(row_key_hash(key), |&id| {
            row_key_of(&terms[id as usize]) == key
        })
        .map(|&id| id as usize)
}

/// One blank-node intern key and the term row it minted.
///
/// The key is the view's own `(label, blank-scope)` pair (C0.2) under the
/// caller's ingest `scope` — the PRE-IMAGE of the qualified label, never the
/// qualified label itself. That is deliberate and it is exactly equivalent:
/// [`BlankScope::qualify_label`] is injective over `(label, scope)`, so keying on
/// the pair and keying on its rendering identify precisely the same blanks. What
/// it buys is that a LOOKUP — the common case, once per ingested blank position —
/// hashes and compares three borrowed components and allocates nothing, where
/// keying on the rendering had to materialize a qualified `String` for every row
/// at any non-default scope just to discover the blank was already interned.
///
/// Kept beside the term table because the stored term value is the WIRE rendering
/// (`"{ingest-scope}-{qualified-label}"`), from which the key cannot be recovered.
#[derive(Debug)]
struct BnodeKey {
    /// The caller-supplied ingest scope (`None` = no ingest prefix).
    scope: Option<String>,
    /// The blank node's own dataset-level scope, as the view reported it.
    blank_scope: BlankScope,
    /// The blank node's RAW label, as the view reported it — not qualified.
    label: String,
    /// The `terms` index this key minted.
    term: u32,
}

/// The fixed-key hash of a borrowed blank intern key.
fn bnode_key_hash(scope: Option<&str>, blank_scope: BlankScope, label: &str) -> u64 {
    FastHasher::default().hash_one((scope, blank_scope.ordinal(), label))
}

/// Hash-consed lookup of a blank-node intern key. Allocates nothing — in
/// particular it never renders the qualified label the wire value is built from.
fn lookup_bnode_key(
    keys: &[BnodeKey],
    index: &HashTable<u32>,
    scope: Option<&str>,
    blank_scope: BlankScope,
    label: &str,
) -> Option<usize> {
    index
        .find(bnode_key_hash(scope, blank_scope, label), |&slot| {
            let key = &keys[slot as usize];
            key.scope.as_deref() == scope && key.blank_scope == blank_scope && key.label == label
        })
        .map(|&slot| keys[slot as usize].term as usize)
}

/// Hash-consed lookup of a blank term row by its WIRE value — the injectivity
/// guard's probe (see [`GtsIngestError::BlankWireCollision`]). Allocates nothing.
fn lookup_bnode_wire(terms: &[TermRow], wire: &HashTable<u32>, value: &str) -> Option<usize> {
    wire.find(FastHasher::default().hash_one(value), |&id| {
        terms[id as usize].value == value
    })
    .map(|&id| id as usize)
}

/// The content-sort order of two term rows, compared over BORROWED components.
///
/// Exactly the tuple order `(kind, value, datatype-IRI, lang, direction)` the
/// owned sort key expressed, with absent columns reading as the empty string —
/// but without materializing five owned strings per element on every comparison.
fn term_order(a: &TermRow, b: &TermRow) -> std::cmp::Ordering {
    row_key_of(a).cmp(&row_key_of(b))
}

/// The length of every accumulating table at one instant.
///
/// This is the whole of an ingestion's undo record. Each table is append-only
/// for the duration of one ingestion — nothing is ever rewritten in place, and
/// canonical re-identification happens later, over a copy — so the lengths alone
/// name the state to return to, and taking the mark costs five loads.
#[derive(Clone, Copy, Debug)]
struct TableMark {
    terms: usize,
    bnode_keys: usize,
    quads: usize,
    reifies: usize,
    annot: usize,
}

/// An accumulating snapshot builder mirroring `gts_producer._Builder`.
///
/// Term ids are append-order during ingestion (process-unstable), then re-id'd
/// by content in `Self::canonical_tables` so the emitted bytes are a pure
/// function of the inputs.
#[derive(Debug, Default)]
pub struct SnapshotBuilder {
    terms: Vec<TermRow>,
    /// Hash-consed intern index over NON-blank term rows, holding `terms`
    /// indices keyed by the borrowed [`RowKey`]. Blank rows are deliberately
    /// absent — their identity is `(scope, label)`, not the stored wire value.
    index: HashTable<u32>,
    /// Blank-node intern keys in mint order (C0.2): two equal labels in
    /// different ingest scopes stay distinct terms.
    bnode_keys: Vec<BnodeKey>,
    /// Hash-consed index into [`Self::bnode_keys`].
    bnode_index: HashTable<u32>,
    /// Hash-consed index of blank `terms` rows keyed by their WIRE value, so a
    /// second intern key encoding onto an existing wire value is caught at the
    /// moment it would mint an indistinguishable second row.
    bnode_wire: HashTable<u32>,
    quads: Vec<(usize, usize, usize, Option<usize>)>,
    /// Graph-qualified bindings; canonicalization deduplicates complete rows.
    reifies: Vec<ReifierRow>,
    annot: Vec<AnnotationRow>,
    /// The first ingestion failure, if any. Terminal: once set, every later
    /// `add_*` and [`emit_gts`] refuses rather than publishing over a partially
    /// ingested builder.
    ///
    /// The failing ingestion is also ROLLED BACK (see [`TableMark`]), so the
    /// tables a poisoned builder still exposes through
    /// [`snapshot_payload`](Self::snapshot_payload) are the last FULLY ACCEPTED
    /// state and never a truncated interior.
    poison: Option<GtsIngestError>,
    /// The cumulative ingestion receipt across every `add_*` call.
    totals: IngestReport,
}

impl SnapshotBuilder {
    /// A fresh, empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a freshly built non-blank term row and index it. The insertion
    /// path owns its strings; that is the dictionary, not scratch.
    fn push_term_row(&mut self, row: TermRow) -> usize {
        let hash = row_key_hash(row_key_of(&row));
        let id = u32::try_from(self.terms.len()).expect("snapshot term ids fit u32");
        self.terms.push(row);
        let terms = &self.terms;
        self.index.insert_unique(hash, id, |&other| {
            row_key_hash(row_key_of(&terms[other as usize]))
        });
        id as usize
    }

    fn intern_iri(&mut self, iri: &str) -> usize {
        let key = (TermKind::Iri as u8, iri, "", "", "");
        if let Some(id) = lookup_term_row(&self.terms, &self.index, key) {
            return id;
        }
        self.push_term_row(TermRow {
            kind: TermKind::Iri,
            value: iri.to_owned(),
            datatype: None,
            lang: None,
            direction: None,
        })
    }

    /// Intern a blank node reported as `(label, blank_scope)` under the caller's
    /// ingest `scope`.
    ///
    /// The qualified label — and with it the wire value — is materialized ONLY on
    /// the path that actually mints a row (or refuses to). A lookup that finds the
    /// blank already interned renders nothing at all.
    ///
    /// # Errors
    /// [`GtsIngestError::BlankWireCollision`] when this NEW intern key encodes
    /// onto a wire value an EXISTING, different key already minted.
    fn intern_bnode(
        &mut self,
        label: &str,
        blank_scope: BlankScope,
        scope: Option<&str>,
    ) -> Result<usize, GtsIngestError> {
        if let Some(id) = lookup_bnode_key(
            &self.bnode_keys,
            &self.bnode_index,
            scope,
            blank_scope,
            label,
        ) {
            return Ok(id);
        }
        // Scope-prefix the stored value exactly as Python's `_Interner.bnode`:
        // `None` keeps the qualified label; a scope yields `"{scope}-{label}"`.
        // The encoding is FROZEN — every existing scoped caller's bytes ride on
        // it — and it is not injective over `(scope, label)`, so the collision it
        // can produce is refused here rather than encoded.
        let qualified = blank_scope.qualify_label(label);
        let value = match scope {
            None => qualified.as_ref().to_owned(),
            Some(scope) => format!("{scope}-{qualified}"),
        };
        if let Some(existing) = lookup_bnode_wire(&self.terms, &self.bnode_wire, &value) {
            let held = self
                .bnode_keys
                .iter()
                .find(|key| key.term as usize == existing)
                .expect("every blank term row was minted by a blank intern key");
            return Err(GtsIngestError::BlankWireCollision {
                wire_value: value,
                held_scope: held.scope.clone(),
                // The refusal is about the WIRE encoding, so both labels are named
                // in the spelling that encoding consumes: the qualified one.
                held_label: held.blank_scope.qualify_label(&held.label).into_owned(),
                incoming_scope: scope.map(str::to_owned),
                incoming_label: qualified.into_owned(),
            });
        }
        let id = u32::try_from(self.terms.len()).expect("snapshot term ids fit u32");
        let wire_hash = FastHasher::default().hash_one(value.as_str());
        self.terms.push(TermRow {
            kind: TermKind::Bnode,
            value,
            datatype: None,
            lang: None,
            direction: None,
        });
        let terms = &self.terms;
        self.bnode_wire.insert_unique(wire_hash, id, |&other| {
            FastHasher::default().hash_one(terms[other as usize].value.as_str())
        });
        let slot = u32::try_from(self.bnode_keys.len()).expect("blank intern keys fit u32");
        self.bnode_keys.push(BnodeKey {
            scope: scope.map(str::to_owned),
            blank_scope,
            label: label.to_owned(),
            term: id,
        });
        let keys = &self.bnode_keys;
        self.bnode_index
            .insert_unique(bnode_key_hash(scope, blank_scope, label), slot, |&other| {
                let key = &keys[other as usize];
                bnode_key_hash(key.scope.as_deref(), key.blank_scope, &key.label)
            });
        Ok(id as usize)
    }

    fn intern_literal(
        &mut self,
        lex: &str,
        datatype: Option<&str>,
        lang: Option<&str>,
        direction: Option<&str>,
    ) -> usize {
        // Ensure the datatype IRI is interned (IRIs sort before literals, so the
        // datatype id always precedes the literal — §7.5, preserved here).
        if let Some(dt) = datatype {
            self.intern_iri(dt);
        }
        let key = (
            TermKind::Literal as u8,
            lex,
            datatype.unwrap_or(""),
            lang.unwrap_or(""),
            direction.unwrap_or(""),
        );
        if let Some(id) = lookup_term_row(&self.terms, &self.index, key) {
            return id;
        }
        self.push_term_row(TermRow {
            kind: TermKind::Literal,
            value: lex.to_owned(),
            datatype: datatype.map(str::to_owned),
            lang: lang.map(str::to_owned),
            direction: direction.map(str::to_owned),
        })
    }

    /// Ingest a native [`RdfDataset`](crate::RdfDataset) carrier DIRECTLY — interning
    /// its quads and its folded RDF-1.2 reifier/annotation side-tables — without the
    /// oxigraph quad round-trip. This is how the in-memory carrier is serialized at the
    /// single exit: the dataset is already canonical (frozen, blank-nodes standardized
    /// apart by union), so every named graph and the statement layer fold in as-is. The
    /// reifier/annotation side-tables map straight onto `reifies`/`annot` — there is no
    /// `rdf:reifies` re-materialization (the native parse already folded them).
    ///
    /// # Errors
    /// A term cannot be represented by the snapshot's native tables, the builder
    /// is already poisoned by an earlier ingestion failure, or one of the
    /// refusals [`GtsIngestError`] enumerates applies — rendered through its
    /// `Display`, because this signature is frozen for the Python producer.
    pub fn add_dataset(&mut self, dataset: &crate::RdfDataset) -> Result<(), String> {
        self.add_dataset_scoped(dataset, None, None)
    }

    /// Ingest a native [`RdfDataset`](crate::RdfDataset) with the same source-partitioning
    /// hooks the legacy oxigraph ingestion exposed: `default_graph_name` assigns base
    /// quads, reifiers and annotations carrying no graph of their own to a named graph,
    /// and `scope` prefixes
    /// blank-node labels (`"{scope}-{label}"`) so two equal labels in different ingest
    /// scopes stay distinct terms. With both `None` this is the plain carrier ingestion
    /// ([`Self::add_dataset`]). The blank scope applies to EVERY blank position (quads,
    /// reifiers, annotations) exactly as the old `add_quads`/`add_rdf12` did.
    ///
    /// # Errors
    /// A term cannot be represented by the snapshot's native tables, the builder
    /// is already poisoned by an earlier ingestion failure, or one of the
    /// refusals [`GtsIngestError`] enumerates applies — rendered through its
    /// `Display`, because this signature is frozen for the Python producer.
    pub fn add_dataset_scoped(
        &mut self,
        dataset: &crate::RdfDataset,
        default_graph_name: Option<&str>,
        scope: Option<&str>,
    ) -> Result<(), String> {
        // ONE PATH: the frozen carrier is itself a `DatasetView`/`FallibleDatasetView`,
        // so the flat surface is a delegation through the same ingestion core the view
        // surface uses, not a second definition of "ingest a dataset". The receipt is
        // discarded here (this signature is frozen); the cumulative figures stay
        // readable through [`Self::ingest_totals`].
        let _ = self
            .ingest_view(dataset, default_graph_name, scope)
            .map_err(|err| err.to_string())?;
        Ok(())
    }

    /// Ingest ANY [`FallibleDatasetView`] directly — no temporary
    /// [`RdfDataset`](crate::RdfDataset), no text round trip, no per-row owned term
    /// reconstruction. See [`Self::add_view_scoped`] for the partitioning hooks.
    ///
    /// # Errors
    /// [`GtsIngestError`], which is also what poisons the builder.
    pub fn add_view<D: FallibleDatasetView>(
        &mut self,
        view: &D,
    ) -> Result<IngestReport, GtsIngestError> {
        self.add_view_scoped(view, None, None)
    }

    /// Ingest any [`FallibleDatasetView`] with the same source-partitioning hooks the
    /// flat carrier surface exposes: `default_graph_name` assigns base quads, reifiers
    /// and annotations carrying no graph of their own to a named graph, and `scope`
    /// prefixes blank-node labels (`"{scope}-{label}"`) so two equal labels in
    /// different ingest scopes stay distinct terms.
    ///
    /// The returned [`IngestReport`] is the caller's receipt: a snapshot cannot be
    /// obtained without it, so the declaration-only graphs this ingestion deliberately
    /// did NOT intern are named rather than silently dropped.
    ///
    /// # Errors
    /// [`GtsIngestError`], which is also what poisons the builder.
    pub fn add_view_scoped<D: FallibleDatasetView>(
        &mut self,
        view: &D,
        default_graph_name: Option<&str>,
        scope: Option<&str>,
    ) -> Result<IngestReport, GtsIngestError> {
        self.ingest_view(view, default_graph_name, scope)
    }

    /// The cumulative ingestion receipt across every `add_*` call on this builder.
    /// Row and term counts are additive; `scratch_bytes` is the PEAK, so the figure
    /// is monotone across calls.
    pub fn ingest_totals(&self) -> IngestReport {
        self.totals.clone()
    }

    /// The terminal failure that poisoned this builder, if any. Once set, every
    /// later `add_*` and [`emit_gts`] refuses.
    ///
    /// The failing ingestion was also rolled back, so the tables underneath are
    /// the last fully accepted state — a poisoned builder still answers
    /// [`Self::snapshot_content_id`] with a content id over complete content, it
    /// just will not publish it.
    pub fn poison(&self) -> Option<&GtsIngestError> {
        self.poison.as_ref()
    }

    /// THE one ingestion core. Both public surfaces — the frozen flat
    /// `add_dataset[_scoped]` and the generic `add_view[_scoped]` — run through
    /// exactly this body, so a refusal, a checkpoint and the poison flag mean the
    /// same thing on both.
    fn ingest_view<D: FallibleDatasetView>(
        &mut self,
        view: &D,
        default_graph_name: Option<&str>,
        scope: Option<&str>,
    ) -> Result<IngestReport, GtsIngestError> {
        if let Some(poison) = &self.poison {
            // Naming the EARLIER failure, not this call: the only honest thing a
            // poisoned builder can say is which ingestion left it that way.
            return Err(GtsIngestError::Poisoned {
                cause: poison.to_string(),
            });
        }
        // ATOMIC PER INGESTION. A refusal can fire after an arbitrary number of
        // rows have already been interned, and the accessors that answer over the
        // tables — `snapshot_payload` and `snapshot_content_id` — are infallible
        // by frozen signature, so they cannot themselves report the poison. They
        // must therefore never have a truncated interior to describe: the failing
        // ingestion is taken back out, leaving the builder at exactly the state
        // the last fully-accepted ingestion left it in.
        let mark = self.mark();
        match self.ingest_view_rows(view, default_graph_name, scope) {
            Ok(report) => Ok(report),
            Err(err) => {
                self.rollback_to(mark);
                self.poison = Some(err.clone());
                Err(err)
            }
        }
    }

    /// A cheap length-only mark of every accumulating table, taken before a
    /// single row of an ingestion is consumed.
    fn mark(&self) -> TableMark {
        TableMark {
            terms: self.terms.len(),
            bnode_keys: self.bnode_keys.len(),
            quads: self.quads.len(),
            reifies: self.reifies.len(),
            annot: self.annot.len(),
        }
    }

    /// Take a failed ingestion back out, restoring the builder to `mark`.
    ///
    /// Every table this ingestion could have grown is append-only within the
    /// ingestion, so truncation restores the rows exactly. The three hash-consed
    /// indexes hold positions into those tables, so each drops precisely the
    /// entries that now point past the end — no rehash, and nothing that survived
    /// the mark is disturbed.
    fn rollback_to(&mut self, mark: TableMark) {
        self.terms.truncate(mark.terms);
        self.bnode_keys.truncate(mark.bnode_keys);
        self.quads.truncate(mark.quads);
        self.reifies.truncate(mark.reifies);
        self.annot.truncate(mark.annot);
        self.index.retain(|&mut id| (id as usize) < mark.terms);
        self.bnode_wire.retain(|&mut id| (id as usize) < mark.terms);
        self.bnode_index
            .retain(|&mut slot| (slot as usize) < mark.bnode_keys);
    }

    /// The row-consuming body of [`Self::ingest_view`], separated only so that
    /// EVERY early return through it is caught by one poison assignment.
    fn ingest_view_rows<D: FallibleDatasetView>(
        &mut self,
        view: &D,
        default_graph_name: Option<&str>,
        scope: Option<&str>,
    ) -> Result<IngestReport, GtsIngestError> {
        // NO CAPABILITY GATE. The `DatasetView` snapshot-ingestion obligation —
        // a claimed capability must be answerable through its accessor — is a
        // PROSE contract on the trait, and it stays one, because it is not
        // decidable here. A gate used to stand at this point refusing any view
        // whose claimed statement layer enumerated no row, on the theory that an
        // empty answer to a claimed capability was a lie. It is not a lie
        // detector: "this view cannot enumerate its reifiers" and "this view has
        // no reifiers" are the SAME observation at runtime, and the second is an
        // ordinary valid state. `DeltaDatasetView::capabilities()` is the union of
        // its base's and its delta's, so a delta that removes the base's only
        // reifier still claims the layer while correctly enumerating nothing — and
        // the gate refused it. That is an over-refusal of valid data, the mirror
        // of the silent drop the gate was reaching for, and a check that cannot
        // tell the two apart is worse than no check at all.
        //
        // What remains is what IS detectable, and it is checked below: a view that
        // faults while yielding rows (both checkpoints), a term that no snapshot
        // slot can represent, and a blank encoding collision. The snapshot is a
        // function of the rows a view enumerates and of nothing it merely claims.

        // TWO CHECKPOINTS bracket row consumption — sampled BEFORE a single row is
        // read and again AFTER every row (ordinary, reifier and annotation alike)
        // has been consumed — via the shared completeness law
        // [`checkpointed_drain`] rather than a private hand-rolled twin of it.
        //
        // `checkpointed_drain` samples its AFTER checkpoint unconditionally, once
        // the drain closure returns, with no visibility into whatever the closure
        // computed — see its own doc for why that is the right law for a closure
        // that reports its outcome purely through `checkpointed_drain`'s
        // `Ok`/`Err`. This ingestion's drain does not: a term-shape refusal from
        // [`Self::consume_rows`] must take priority over the AFTER checkpoint
        // exactly as it always did, because the pre-refactor body returned such a
        // refusal immediately via `?`, without ever sampling the view's post-drain
        // status. `row_error`, set from inside the closure, carries that refusal
        // out so it can be checked FIRST — before the drain's own before/after
        // verdict — preserving that priority under the shared helper.
        let terms_before = self.terms.len();
        let mut rows_consumed = 0_usize;
        // The graph terms that actually own a row. Everything `named_graphs()`
        // reports beyond this set is declaration-only: NOT interned (interning it
        // would add a term row and shift `snapshot_content_id`), but named in the
        // report so the omission is stated rather than silent.
        let mut occupied_graphs: std::collections::BTreeSet<D::Id> =
            std::collections::BTreeSet::new();
        let mut row_error: Option<GtsIngestError> = None;
        let drained = checkpointed_drain(view, |v| {
            match self.consume_rows(v, default_graph_name, scope) {
                Ok((rows, graphs)) => {
                    rows_consumed = rows;
                    occupied_graphs = graphs;
                }
                Err(err) => row_error = Some(err),
            }
        });
        if let Some(err) = row_error {
            return Err(err);
        }
        if let Err(failure) = drained {
            return Err(GtsIngestError::ViewNotReady {
                checkpoint: match failure.checkpoint {
                    DrainCheckpoint::Before => IngestCheckpoint::BeforeRows,
                    DrainCheckpoint::After => IngestCheckpoint::AfterRows,
                },
                cause: failure.error.to_string(),
            });
        }

        let mut declarations_omitted: Vec<String> = view
            .named_graphs()
            .filter(|graph| !occupied_graphs.contains(graph))
            .map(|graph| render_term(view, graph))
            .collect();
        declarations_omitted.sort_unstable();
        declarations_omitted.dedup();

        let report = IngestReport {
            rows_consumed,
            terms_interned: self.terms.len() - terms_before,
            declarations_omitted,
            scratch_bytes: self.scratch_bytes(),
        };
        self.totals.rows_consumed = self.totals.rows_consumed.saturating_add(rows_consumed);
        self.totals.terms_interned = self
            .totals
            .terms_interned
            .saturating_add(report.terms_interned);
        self.totals
            .declarations_omitted
            .extend(report.declarations_omitted.iter().cloned());
        self.totals.declarations_omitted.sort_unstable();
        self.totals.declarations_omitted.dedup();
        self.totals.scratch_bytes = self.totals.scratch_bytes.max(report.scratch_bytes);
        Ok(report)
    }

    /// The row-consuming body of one ingestion pass: interns every ordinary
    /// quad, reifier binding and annotation row, returning the row count and
    /// the graph terms that actually own a row (everything `named_graphs()`
    /// reports beyond that set is declaration-only — see the caller).
    ///
    /// Pulled out of [`Self::ingest_view_rows`] so the [`checkpointed_drain`]
    /// closure that wraps it stays a single call rather than a restated loop
    /// body, and its own early returns (`?`) stay ordinary function returns —
    /// exactly as they were before this was factored out — rather than needing
    /// to break out of a closure mid-loop.
    ///
    /// # Errors
    /// [`GtsIngestError::UnrepresentableTerm`] or
    /// [`GtsIngestError::BlankWireCollision`] — exactly the refusals
    /// [`Self::intern_view_term`]/[`Self::intern_view_iri`] can raise.
    fn consume_rows<D: DatasetView>(
        &mut self,
        view: &D,
        default_graph_name: Option<&str>,
        scope: Option<&str>,
    ) -> Result<(usize, std::collections::BTreeSet<D::Id>), GtsIngestError> {
        let mut rows_consumed = 0_usize;
        let mut occupied_graphs: std::collections::BTreeSet<D::Id> =
            std::collections::BTreeSet::new();
        let default_gid = default_graph_name.map(|name| self.intern_iri(name));

        // FAIL CLOSED (no-optionality): a row whose subject/object/graph is not
        // directly representable in the snapshot frame (a quoted-triple term, or a
        // non-IRI/blank graph name) is NOT silently dropped — that would make the
        // emitted `purrdf.gts` diverge from the view. Quoted triples are
        // representable ONLY via the reifier/annotation tables (below), so a Triple
        // term in plain-quad position is genuine loss and aborts the ingestion.
        for quad in view.quads() {
            let sid = self.intern_view_term(view, quad.s, scope, "quad subject")?;
            let pid = self.intern_view_iri(view, quad.p, "quad predicate")?;
            let oid = self.intern_view_term(view, quad.o, scope, "quad object")?;
            let gid = match quad.g {
                None => default_gid,
                Some(graph) => {
                    occupied_graphs.insert(graph);
                    Some(self.intern_view_term(view, graph, scope, "quad graph name")?)
                }
            };
            self.quads.push((sid, pid, oid, gid));
            rows_consumed += 1;
        }

        // The RDF 1.2 statement layer rides the NATIVE side-table accessors, never
        // `quads()`: its virtual rows are excluded from the ordinary row partition,
        // so reading them here neither double-ingests them nor routes a binding's
        // quoted-triple object through the plain-slot guard above.
        for binding in view.reifier_quads() {
            let rid = self.intern_view_term(view, binding.s, scope, "reifier term")?;
            let TermRef::Triple { s, p, o } = view.resolve(binding.o) else {
                return Err(GtsIngestError::UnrepresentableTerm {
                    position: "reifier binding object",
                    term: render_term(view, binding.o),
                });
            };
            let qs = self.intern_view_term(view, s, scope, "reified subject")?;
            let qp = self.intern_view_iri(view, p, "reified predicate")?;
            let qo = self.intern_view_term(view, o, scope, "reified object")?;
            let gid = match binding.g {
                None => default_gid,
                Some(graph) => {
                    occupied_graphs.insert(graph);
                    Some(self.intern_view_term(view, graph, scope, "reifier graph name")?)
                }
            };
            self.reifies.push((rid, (qs, qp, qo), gid));
            rows_consumed += 1;
        }

        for annotation in view.annotation_quads() {
            let rid = self.intern_view_term(view, annotation.s, scope, "annotation reifier")?;
            let pid = self.intern_view_iri(view, annotation.p, "annotation predicate")?;
            let oid = self.intern_view_term(view, annotation.o, scope, "annotation object")?;
            let gid = match annotation.g {
                None => default_gid,
                Some(graph) => {
                    occupied_graphs.insert(graph);
                    Some(self.intern_view_term(view, graph, scope, "annotation graph name")?)
                }
            };
            self.annot.push((rid, pid, oid, gid));
            rows_consumed += 1;
        }

        Ok((rows_consumed, occupied_graphs))
    }

    /// Intern a view term in subject/object/graph position (triple terms are NOT
    /// interned — the RDF 1.2 layer rides the reifies/annot tables).
    ///
    /// Reproduces the flat carrier path's literal normalization exactly: a view's
    /// `TermRef::Literal` ALWAYS carries a datatype id, but a language tag implies
    /// no datatype and `xsd:string` is implicit, so in both cases the datatype IRI
    /// is dropped — and, because it is dropped, it is never interned either. An
    /// extra dictionary row there would shift `snapshot_content_id`.
    ///
    /// `scope` prefixes blank labels after the view's OWN dataset-level scope
    /// qualification, so two composite sources that both spell `_:b0` stay two
    /// terms (C0.2) and a caller scope separates whole sources on top of that.
    fn intern_view_term<D: DatasetView>(
        &mut self,
        view: &D,
        id: D::Id,
        scope: Option<&str>,
        position: &'static str,
    ) -> Result<usize, GtsIngestError> {
        match view.resolve(id) {
            TermRef::Iri(iri) => Ok(self.intern_iri(iri)),
            TermRef::Blank {
                label,
                scope: blank_scope,
            } => self.intern_bnode(label, blank_scope, scope),
            TermRef::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                let TermRef::Iri(datatype_iri) = view.resolve(datatype) else {
                    return Err(GtsIngestError::UnrepresentableTerm {
                        position: "literal datatype",
                        term: render_term(view, datatype),
                    });
                };
                if let Some(language) = language {
                    // Base direction rides with the language tag — it exists only on
                    // a language-tagged literal — and is carried here rather than
                    // dropped, which is what merged `@en--ltr` with `@en--rtl`.
                    Ok(self.intern_literal(
                        lexical,
                        None,
                        Some(language),
                        direction.map(RdfTextDirection::as_str),
                    ))
                } else {
                    let datatype = (datatype_iri != XSD_STRING).then_some(datatype_iri);
                    Ok(self.intern_literal(lexical, datatype, None, None))
                }
            }
            TermRef::Triple { .. } => Err(GtsIngestError::UnrepresentableTerm {
                position,
                term: render_term(view, id),
            }),
        }
    }

    /// Intern a view term that MUST be an IRI (a predicate slot).
    fn intern_view_iri<D: DatasetView>(
        &mut self,
        view: &D,
        id: D::Id,
        position: &'static str,
    ) -> Result<usize, GtsIngestError> {
        match view.resolve(id) {
            TermRef::Iri(iri) => Ok(self.intern_iri(iri)),
            _ => Err(GtsIngestError::UnrepresentableTerm {
                position,
                term: render_term(view, id),
            }),
        }
    }

    /// Peak scratch bytes attributable to ingestion bookkeeping: the hash-consed
    /// intern indexes, the canonical-table sort buffers, and the wire-term
    /// staging. Capacity-based and payload-only — allocator overhead and the term
    /// dictionary's own string bytes are NOT counted, matching the accounting
    /// convention `ir::view_accounting` states for retained bytes.
    fn scratch_bytes(&self) -> usize {
        let buckets = self
            .index
            .capacity()
            .saturating_add(self.bnode_index.capacity())
            .saturating_add(self.bnode_wire.capacity());
        let index_bytes = buckets.saturating_mul(size_of::<u32>()).saturating_add(
            self.bnode_keys
                .capacity()
                .saturating_mul(size_of::<BnodeKey>()),
        );
        // `canonical_tables` allocates one `order` and one `remap` index vector…
        let sort_bytes = self.terms.len().saturating_mul(2 * size_of::<usize>());
        // …and stages one wire `Term` per term before the payload moves them out.
        let staging_bytes = self.terms.len().saturating_mul(size_of::<Term>());
        index_bytes
            .saturating_add(sort_bytes)
            .saturating_add(staging_bytes)
    }

    /// Re-id every term by content and sort every row (`_Builder._canonical_tables`).
    ///
    /// Returns the canonical `(wire_terms, quads, reifies, annot)` ready for the
    /// snapshot payload. Terms sort by `(kind, value, datatype-IRI, lang)` with
    /// IRIs first, so every literal's datatype IRI precedes it.
    fn canonical_tables(&self) -> CanonTables {
        let n = self.terms.len();
        let mut order: Vec<usize> = (0..n).collect();
        // Compare BORROWED rows: the owned sort key allocated five strings per
        // element on every comparison to express an order the rows already carry.
        order.sort_by(|&a, &b| term_order(&self.terms[a], &self.terms[b]));
        let mut remap = vec![0usize; n];
        for (new_id, &old) in order.iter().enumerate() {
            remap[old] = new_id;
        }

        // Wire terms in new-id order; the datatype field becomes the remapped id
        // of its IRI term (interned earlier, so it has an old id and thus a new id).
        let wire_terms: Vec<Term> = order
            .iter()
            .map(|&old| {
                let row = &self.terms[old];
                let datatype = row.datatype.as_deref().map(|dt| {
                    let key = (TermKind::Iri as u8, dt, "", "", "");
                    let old_dt = lookup_term_row(&self.terms, &self.index, key)
                        .expect("a literal's datatype IRI is interned before the literal");
                    remap[old_dt]
                });
                Term {
                    kind: row.kind,
                    value: Some(row.value.clone()),
                    datatype,
                    lang: row.lang.clone(),
                    direction: row.direction.clone(),
                    reifier: None,
                    triple: None,
                }
            })
            .collect();

        // Quads: remap, dedup, sort by (graph[None=-1], s, p, o).
        let mut quad_set: std::collections::BTreeSet<(i64, usize, usize, usize, Option<usize>)> =
            std::collections::BTreeSet::new();
        for &(s, p, o, g) in &self.quads {
            let g = g.map(|g| remap[g]);
            let gkey = g.map_or(-1, |g| g as i64);
            quad_set.insert((gkey, remap[s], remap[p], remap[o], g));
        }
        let quads: Vec<(usize, usize, usize, Option<usize>)> = quad_set
            .into_iter()
            .map(|(_, s, p, o, g)| (s, p, o, g))
            .collect();

        // Reifies: remap, then sort by the WHOLE row. One reifier id may carry
        // several bindings (`rdf:reifies` is not functional), so sorting by the
        // reifier id alone would leave their order to the ingestion order —
        // and the emitted bytes must be a pure function of the content.
        let mut reifies: Vec<ReifierRow> = self
            .reifies
            .iter()
            .map(|&(rid, (s, p, o), g)| {
                (
                    remap[rid],
                    (remap[s], remap[p], remap[o]),
                    g.map(|g| remap[g]),
                )
            })
            .collect();
        reifies.sort_unstable();
        reifies.dedup();

        // Annot: remap, dedup, sort.
        let mut annot_set: std::collections::BTreeSet<AnnotationRow> =
            std::collections::BTreeSet::new();
        for &(r, p, v, g) in &self.annot {
            annot_set.insert((remap[r], remap[p], remap[v], g.map(|g| remap[g])));
        }
        let annot: Vec<AnnotationRow> = annot_set.into_iter().collect();

        (wire_terms, quads, reifies, annot)
    }

    /// The canonical `snapshot` frame payload (`_Builder._snapshot_payload`).
    ///
    /// Infallible, and honestly so: a failed ingestion is rolled back before its
    /// error is returned (see [`GtsIngestError`]), so these tables are always the
    /// last FULLY ACCEPTED state and never a truncated interior. What this does
    /// NOT mean is that a poisoned builder may be published — it may not; that
    /// refusal lives in [`emit_gts`], because a complete description of the wrong
    /// content is still the wrong content.
    pub fn snapshot_payload(&self) -> Value {
        let (terms, quads, reifies, annot) = self.canonical_tables();
        let mut entries: Vec<(Value, Value)> = vec![
            (
                "terms".into(),
                // The staged wire terms are consumed here, so their strings MOVE
                // into the payload rather than being cloned a second time.
                Value::Array(terms.into_iter().map(term_into_wire).collect()),
            ),
            (
                "quads".into(),
                Value::Array(
                    quads
                        .iter()
                        .map(|&(s, p, o, g)| {
                            let mut row = vec![iv(s), iv(p), iv(o)];
                            if let Some(g) = g {
                                row.push(iv(g));
                            }
                            Value::Array(row)
                        })
                        .collect(),
                ),
            ),
        ];
        if !reifies.is_empty() {
            // The optional trailing graph term-id is part of each binding's identity.
            // Default-graph rows retain their existing four-item encoding.
            entries.push((
                "reifies".into(),
                Value::Array(
                    reifies
                        .iter()
                        .map(|&(rid, (s, p, o), g)| {
                            let mut row = vec![iv(rid), iv(s), iv(p), iv(o)];
                            if let Some(g) = g {
                                row.push(iv(g));
                            }
                            Value::Array(row)
                        })
                        .collect(),
                ),
            ));
        }
        if !annot.is_empty() {
            entries.push((
                "annot".into(),
                Value::Array(
                    annot
                        .iter()
                        .map(|&(r, p, v, g)| {
                            let mut row = vec![iv(r), iv(p), iv(v)];
                            if let Some(g) = g {
                                row.push(iv(g));
                            }
                            Value::Array(row)
                        })
                        .collect(),
                ),
            ));
        }
        Value::Map(entries)
    }

    /// The `blake3:<hex>` content address of the snapshot payload
    /// (`_Builder.snapshot_content_id`).
    ///
    /// A stable content id over complete content in every reachable state,
    /// poisoned included — see [`Self::snapshot_payload`] for why, and for why
    /// that is not permission to publish a poisoned builder.
    pub fn snapshot_content_id(&self) -> String {
        let bytes = canonical(&self.snapshot_payload());
        format!("blake3:{}", hex(&blake3_256(&bytes)))
    }
}

fn iv(n: usize) -> Value {
    Value::Integer(ciborium::value::Integer::from(n as u64))
}

/// The by-VALUE twin of `purrdf_gts::writer::term_to_wire`.
///
/// Byte-for-byte the same CBOR map — same keys, same order, same `dir`
/// admission filter — but it CONSUMES the term, so the staged strings move into
/// the payload instead of being cloned into it. The equivalence is not asserted
/// by construction, it is pinned by a test (`term_into_wire_matches_the_writer`).
fn term_into_wire(term: Term) -> Value {
    let mut entries: Vec<(Value, Value)> = Vec::with_capacity(6);
    entries.push(("k".into(), wire_id(term.kind as usize)));
    if let Some(value) = term.value {
        entries.push(("v".into(), Value::Text(value)));
    }
    if let Some(datatype) = term.datatype {
        entries.push(("dt".into(), wire_id(datatype)));
    }
    if let Some(lang) = term.lang {
        entries.push(("l".into(), Value::Text(lang)));
    }
    if let Some(direction) = term
        .direction
        .filter(|direction| is_literal_direction(direction))
    {
        entries.push(("dir".into(), Value::Text(direction)));
    }
    if let Some(reifier) = term.reifier {
        entries.push(("rf".into(), wire_id(reifier)));
    }
    if let Some((s, p, o)) = term.triple {
        entries.push((
            "tt".into(),
            Value::Array(vec![wire_id(s), wire_id(p), wire_id(o)]),
        ));
    }
    Value::Map(entries)
}

/// A term id as the writer spells it on the wire (a signed CBOR integer).
fn wire_id(n: usize) -> Value {
    Value::Integer(ciborium::value::Integer::from(n as i64))
}

/// How deep [`render_term`] follows a term's constituents.
///
/// A diagnostic is not a serializer: it must terminate over ANY view, including
/// one whose `resolve` is cyclic, and it does not need to be exhaustive to be
/// useful. Quoted triples nest far more shallowly than this in practice; past it
/// the rendering says so rather than recursing.
const RENDER_TERM_DEPTH: usize = 8;

/// Render a view term for a diagnostic, in its own TEXT rather than in the view's
/// internal ids.
///
/// Every constituent is followed: a quoted triple renders its subject, predicate
/// and object, and a typed literal renders its datatype IRI. This is the whole
/// point of the function — a message reading `Triple { s: Id(41), p: Id(7), ... }`
/// names the offending term in a vocabulary only the view itself speaks, and
/// leaves the reader unable to find the row it came from.
fn render_term<D: DatasetView>(view: &D, id: D::Id) -> String {
    render_term_within(view, id, RENDER_TERM_DEPTH)
}

/// [`render_term`] with the remaining recursion budget carried explicitly.
fn render_term_within<D: DatasetView>(view: &D, id: D::Id, depth: usize) -> String {
    let Some(next) = depth.checked_sub(1) else {
        return "…".to_owned();
    };
    match view.resolve(id) {
        TermRef::Iri(iri) => iri.to_owned(),
        TermRef::Blank { label, scope } => format!("_:{}", scope.qualify_label(label)),
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => {
            // N-Triples-shaped, and deliberately: a language tag suppresses the
            // datatype exactly as the ingestion path does, so the rendering
            // describes the term the snapshot would have stored.
            let mut rendered = format!("{lexical:?}");
            match (language, direction) {
                (Some(language), Some(direction)) => {
                    rendered.push('@');
                    rendered.push_str(language);
                    rendered.push_str("--");
                    rendered.push_str(direction.as_str());
                }
                (Some(language), None) => {
                    rendered.push('@');
                    rendered.push_str(language);
                }
                (None, _) => {
                    rendered.push_str("^^<");
                    rendered.push_str(&render_term_within(view, datatype, next));
                    rendered.push('>');
                }
            }
            rendered
        }
        TermRef::Triple { s, p, o } => format!(
            "<<( {} {} {} )>>",
            render_term_within(view, s, next),
            render_term_within(view, p, next),
            render_term_within(view, o, next),
        ),
    }
}

/// Which ingestion boundary sampled a fallible view's operational status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum IngestCheckpoint {
    /// Sampled before the first row was consumed.
    BeforeRows,
    /// Sampled after every row — ordinary, reifier and annotation alike — had
    /// been consumed, and before any figure derived from them was published.
    AfterRows,
}

impl std::fmt::Display for IngestCheckpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BeforeRows => f.write_str("before row consumption"),
            Self::AfterRows => f.write_str("after row consumption"),
        }
    }
}

/// A terminal, typed ingestion refusal.
///
/// Every variant is a REFUSAL, never a degraded success. Two things follow from
/// one, and both are load-bearing:
///
/// * The failing ingestion is ROLLED BACK. Every row it had already interned is
///   taken back out, so the builder is left at exactly the state the last fully
///   accepted ingestion left it in. This is what makes the infallible
///   [`SnapshotBuilder::snapshot_content_id`] and
///   [`SnapshotBuilder::snapshot_payload`] safe to keep infallible: there is no
///   half-ingested interior for them to describe.
/// * The builder is POISONED. It is nonetheless refused for all further use:
///   every later `add_*` returns [`Self::Poisoned`] and [`emit_gts`] refuses to
///   publish. A caller asked for content that could not be ingested, and shipping
///   the subset that could — however internally consistent — would answer a
///   question nobody asked.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GtsIngestError {
    /// The view's operational status was not `Ready` at an ingestion checkpoint,
    /// so the rows read are a truncation, not the view.
    ViewNotReady {
        /// Which boundary observed the failure.
        checkpoint: IngestCheckpoint,
        /// The view's own sticky root cause, rendered.
        cause: String,
    },
    /// A term occupies a plain snapshot slot it cannot be represented in — a
    /// quoted-triple term outside the reifier/annotation tables, or a non-IRI in
    /// a predicate or datatype slot.
    UnrepresentableTerm {
        /// The slot the term was read from.
        position: &'static str,
        /// The offending term, rendered.
        term: String,
    },
    /// Two DISTINCT blank intern keys encode onto one wire value.
    ///
    /// The frozen wire encoding `"{scope}-{label}"` is not injective over
    /// `(scope, label)`: `(Some("a"), "b-c")` and `(Some("a-b"), "c")` both
    /// spell `a-b-c`. Minting the second row would leave two indistinguishable
    /// blank terms whose relative order the stable canonical sort takes from the
    /// INGESTION order, so the emitted bytes would stop being a pure function of
    /// the content. Changing the encoding would move every existing scoped
    /// caller's bytes, so the collision is refused instead.
    BlankWireCollision {
        /// The wire value both keys encode onto.
        wire_value: String,
        /// The ingest scope of the key already holding that wire value.
        held_scope: Option<String>,
        /// The label of the key already holding that wire value.
        held_label: String,
        /// The ingest scope of the key that collided with it.
        incoming_scope: Option<String>,
        /// The label of the key that collided with it.
        incoming_label: String,
    },
    /// An earlier ingestion failed; this builder can no longer ingest or publish.
    /// Its tables still describe the last fully accepted state — the failed
    /// ingestion was rolled back — but that state is not what the caller asked
    /// for, so it is not publishable.
    Poisoned {
        /// The earlier failure, rendered.
        cause: String,
    },
}

impl std::fmt::Display for GtsIngestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ViewNotReady { checkpoint, cause } => write!(
                f,
                "the view reported an operational failure {checkpoint}, so the rows read are \
                 a truncation rather than the view: {cause}"
            ),
            Self::UnrepresentableTerm { position, term } => write!(
                f,
                "carrier {position} is not directly representable in the gts snapshot frame \
                 (quoted-triple terms must ride the reifier/annotation tables): {term}"
            ),
            Self::BlankWireCollision {
                wire_value,
                held_scope,
                held_label,
                incoming_scope,
                incoming_label,
            } => write!(
                f,
                "two distinct blank-node intern keys encode onto the single wire value \
                 {wire_value:?}: ({held_scope:?}, {held_label:?}) already holds it and \
                 ({incoming_scope:?}, {incoming_label:?}) would mint an indistinguishable \
                 second term row"
            ),
            Self::Poisoned { cause } => write!(
                f,
                "the snapshot builder is poisoned by an earlier ingestion failure and can \
                 neither ingest nor publish: {cause}"
            ),
        }
    }
}

impl std::error::Error for GtsIngestError {}

/// What ONE ingestion consumed, minted and deliberately omitted.
///
/// A caller cannot obtain a snapshot without receiving this: the omitted
/// declaration-only graphs in particular are a decision, not an accident, and a
/// surface that returned only `Ok(())` would make that decision invisible.
#[must_use = "the ingest report names the declaration-only graphs that were deliberately not interned"]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IngestReport {
    /// Rows consumed across all three tables (ordinary, reifier, annotation).
    pub rows_consumed: usize,
    /// Term rows this ingestion added to the snapshot dictionary.
    pub terms_interned: usize,
    /// The declaration-only graph names this ingestion did NOT intern, sorted
    /// and deduplicated. Interning them would add term rows and shift
    /// `snapshot_content_id`; naming them here keeps the omission stated.
    pub declarations_omitted: Vec<String>,
    /// Peak scratch bytes held by the intern indexes, the canonical-table sort
    /// buffers and the wire-term staging. Capacity-based and payload-only.
    pub scratch_bytes: usize,
}

/// A `(data, media_type, rep)` content-addressed blob row riding ahead of the
/// snapshot frame.
#[derive(Debug)]
pub struct BlobRow {
    /// The decoded blob bytes.
    pub data: Vec<u8>,
    /// The blob's declared media type (`mt`).
    pub media_type: String,
    /// The blob's content representation tag (`rep`).
    pub rep: String,
}

/// Choose `zstd-rsyncable` for large payloads when the base chain is the default
/// `["zstd"]` (`_Builder.to_gts.choose_transform`).
pub fn choose_transform(
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

/// Which frame slot an assignment row addresses.
///
/// One total assignment covers EVERY authored frame: the snapshot itself and
/// each blob, keyed by its content-representation tag.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FrameSlot {
    /// The single canonical `snapshot` frame.
    Snapshot,
    /// A `blob` frame carrying this `pub.rep` representation tag.
    Blob(String),
}

/// Which in-band dictionary primes a frame — TOTAL, never `Option`.
///
/// An `Option<&str>` fall-through would let "the caller forgot this rep" and
/// "this rep is deliberately undicted" be the same value. They are not: the
/// first is a bug that silently costs density, the second is a decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DictSelection {
    /// Prime with the pinned dictionary of this name.
    Named(String),
    /// Deliberately no dictionary.
    Baseline,
}

/// The medium-level authoring plan: which in-band dictionaries the bundle pins,
/// which one primes which frame, and the zstd level it declares.
///
/// This replaces two pieces of implicit behaviour: a bundle used to be able to
/// carry at most one dictionary applied by name-matching inside the writer, and
/// its zstd level was INFERRED from the profile string (`profile == "dist"`),
/// so level 12 was unreachable under any other profile no matter what the
/// caller wanted. Both are now caller-stated data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediumPlan {
    /// Named in-band dictionaries pinned in the header `"dct"` map (§5).
    pub dicts: Vec<(String, Vec<u8>)>,
    /// The TOTAL frame → dictionary assignment. When [`Self::dicts`] is
    /// non-empty, every authored frame slot must appear here; a missing slot is
    /// a hard error, never a silent baseline encode.
    pub assignment: BTreeMap<FrameSlot, DictSelection>,
    /// The zstd level for every zstd-family frame, declared in the catalog
    /// (§8.5 `level?`). Explicit — never derived from the profile name.
    pub zstd_level: Option<i32>,
}

impl MediumPlan {
    /// A plan with no dictionaries at `level`.
    pub fn undicted(zstd_level: Option<i32>) -> Self {
        Self {
            dicts: Vec::new(),
            assignment: BTreeMap::new(),
            zstd_level,
        }
    }

    /// The standard bundle plan: no dictionaries, [`DIST_ZSTD_LEVEL`] declared
    /// whenever `transform` is a zstd-family chain.
    ///
    /// CHAIN-gated, never profile-gated: a level is meaningful exactly when
    /// there is a zstd codec to apply it to, and has nothing to do with what
    /// the header's profile string says.
    pub fn dist_default(transform: Option<&[String]>) -> Self {
        let chain: &[String] = transform.unwrap_or(&[]);
        let zstd_family = transform.is_none()
            || chain
                .iter()
                .any(|name| name == "zstd" || name == "zstd-rsyncable");
        Self::undicted(zstd_family.then_some(DIST_ZSTD_LEVEL))
    }

    /// Resolve the dictionary for `slot`.
    ///
    /// With no dictionaries pinned there is nothing to choose between, so the
    /// baseline is the only inhabitant of the choice and needs no row. With
    /// dictionaries pinned, a missing row is a hard error.
    fn select(&self, slot: &FrameSlot) -> Result<Option<&str>, String> {
        if self.dicts.is_empty() {
            return Ok(None);
        }
        match self.assignment.get(slot) {
            Some(DictSelection::Named(name)) => Ok(Some(name)),
            Some(DictSelection::Baseline) => Ok(None),
            None => Err(format!(
                "the medium plan pins {} dictionar(y/ies) but assigns none to {slot:?}; \
                 every frame slot needs an explicit DictSelection",
                self.dicts.len()
            )),
        }
    }
}

/// Emit the snapshot bundle bytes from an accumulated builder (`_Builder.to_gts`).
#[allow(clippy::too_many_arguments)]
pub fn emit_gts(
    builder: &SnapshotBuilder,
    profile: &str,
    transform: Option<Vec<String>>,
    doc_blobs: Vec<BlobRow>,
    report_blobs: Vec<BlobRow>,
    signer_secret: Option<[u8; 32]>,
    signer_kid: Option<String>,
    public_key_armor: Option<String>,
    rsyncable_threshold: usize,
    plan: &MediumPlan,
) -> Result<Vec<u8>, String> {
    // TERMINAL POISON. A mid-ingestion failure leaves a partially interned
    // builder; publishing it would emit a bundle that is a prefix of the data the
    // caller asked for and call it a snapshot. Refuse, naming the failure.
    if let Some(poison) = builder.poison() {
        return Err(GtsIngestError::Poisoned {
            cause: poison.to_string(),
        }
        .to_string());
    }

    // No-optionality: signing is all-or-nothing across ALL THREE fields
    // (secret, kid, public key). A partial config — e.g. a `signer_kid` with no
    // secret/armor — would otherwise be silently treated as unsigned, dropping
    // the kid and emitting an unsigned bundle that carries (or implies) signing
    // metadata. Require every signing field together or none; hard-fail between.
    let signing = match (&signer_secret, &signer_kid, &public_key_armor) {
        (Some(_), Some(_), Some(_)) => true,
        (None, None, None) => false,
        _ => {
            return Err(
                "signing requires signer_secret, signer_kid, and public_key_armor together \
                 (all three or none)"
                    .to_string(),
            );
        }
    };

    let base_chain = transform.unwrap_or_else(|| vec!["zstd".to_string()]);

    // The zstd level is whatever the CALLER declared — never inferred from the
    // profile string. The old `profile == "dist"` gate made level 12 reachable
    // only under one literal profile name, so a caller emitting a non-`dist`
    // bundle silently got the writer's ~level-1 default no matter what it
    // wanted. A level is still meaningful only for a zstd-family chain (the
    // writer hard-fails a level paired with a non-zstd transform), so it is
    // gated on the chain and on nothing else.
    let chain_is_zstd = base_chain
        .iter()
        .any(|t| t == "zstd" || t == "zstd-rsyncable");
    let zstd_level: Option<i32> = plan.zstd_level.filter(|_| chain_is_zstd);
    if plan.zstd_level.is_some() && !chain_is_zstd {
        return Err(format!(
            "the medium plan declares zstd level {:?} but the transform chain {base_chain:?} \
             carries no zstd-family codec",
            plan.zstd_level
        ));
    }
    if !plan.dicts.is_empty() && !chain_is_zstd {
        return Err(format!(
            "the medium plan pins in-band dictionaries but the transform chain \
             {base_chain:?} carries no zstd-family codec to prime"
        ));
    }

    let mut writer = Writer::with_options(
        profile,
        purrdf_gts::writer::WriterOptions {
            dicts: plan.dicts.clone(),
            zstd_level,
            ..purrdf_gts::writer::WriterOptions::default()
        },
    )
    .map_err(|err| err.to_string())?;
    if signing {
        let secret = signer_secret.expect("signing implies a secret");
        let kid = signer_kid.ok_or("signing requires a kid")?;
        writer.sign_with(ed25519_dalek::SigningKey::from_bytes(&secret), &kid);
        // The transport-key meta frame, signed along with every later frame.
        let armor = public_key_armor.expect("signing implies a public key");
        let meta = Value::Map(vec![(
            "gts:transportKey".into(),
            Value::Map(vec![
                ("kid".into(), Value::Text(kid)),
                ("gpg".into(), Value::Text(armor)),
            ]),
        )]);
        writer.add_meta(meta);
    }

    // Blob frames ride AHEAD of the snapshot, sorted by (rep, decoded-bytes).
    let mut all_blobs: Vec<BlobRow> = doc_blobs;
    all_blobs.extend(report_blobs);
    all_blobs.sort_by(|a, b| a.rep.cmp(&b.rep).then_with(|| a.data.cmp(&b.data)));
    for blob in all_blobs {
        let chain = choose_transform(&base_chain, blob.data.len(), rsyncable_threshold);
        let dict = plan.select(&FrameSlot::Blob(blob.rep.clone()))?;
        // `add_blob` does not take a transform; author the frame directly so the
        // per-payload rsyncable selection is honored (parity with `_Builder`).
        let pub_meta = Value::Map(vec![
            (
                "digest".into(),
                Value::Text(purrdf_gts::writer::digest_string(&blob.data)),
            ),
            ("mt".into(), Value::Text(blob.media_type.clone())),
            ("rep".into(), Value::Text(blob.rep.clone())),
        ]);
        let options = purrdf_gts::writer::FrameOptions {
            raw: Some(blob.data),
            transform: chain,
            pub_meta: Some(pub_meta),
            zstd_level,
            dict: dict.map(str::to_string),
            ..Default::default()
        };
        writer
            .add_frame_with_options("blob", options)
            .map_err(|e| e.to_string())?;
    }

    let payload = builder.snapshot_payload();
    let snapshot_bytes = canonical(&payload);
    let chain = choose_transform(&base_chain, snapshot_bytes.len(), rsyncable_threshold);
    let snapshot_dict = plan.select(&FrameSlot::Snapshot)?;
    let options = purrdf_gts::writer::FrameOptions {
        payload: Some(payload),
        transform: chain,
        zstd_level,
        dict: snapshot_dict.map(str::to_string),
        ..Default::default()
    };
    writer
        .add_frame_with_options("snapshot", options)
        .map_err(|e| e.to_string())?;

    Ok(writer.into_bytes())
}

#[cfg(test)]
mod tests {
    //! Pure-Rust coverage of the `SnapshotBuilder` core (no Python interpreter):
    //! interning order, content sort, the snapshot payload, and the content-id.
    use super::*;
    use crate::parse_dataset;

    /// The by-value wire encoder must be indistinguishable from the writer's own
    /// by-reference one — it exists only to MOVE the staged strings, never to
    /// restate the encoding. A drift here would silently change every emitted
    /// bundle, so the equivalence is asserted over every field the writer reads.
    #[test]
    fn term_into_wire_matches_the_writer() {
        let spread = vec![
            Term {
                kind: TermKind::Iri,
                value: Some("https://example.org/s".to_owned()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
                triple: None,
            },
            Term {
                kind: TermKind::Bnode,
                value: Some("scope-b0".to_owned()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
                triple: None,
            },
            Term {
                kind: TermKind::Literal,
                value: Some("plain".to_owned()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
                triple: None,
            },
            Term {
                kind: TermKind::Literal,
                value: Some("7".to_owned()),
                datatype: Some(3),
                lang: None,
                direction: None,
                reifier: None,
                triple: None,
            },
            Term {
                kind: TermKind::Literal,
                value: Some("مرحبا".to_owned()),
                datatype: None,
                lang: Some("ar".to_owned()),
                direction: Some("rtl".to_owned()),
                reifier: None,
                triple: None,
            },
            // A direction the writer REFUSES to emit: the filter must survive.
            Term {
                kind: TermKind::Literal,
                value: Some("x".to_owned()),
                datatype: None,
                lang: Some("en".to_owned()),
                direction: Some("sideways".to_owned()),
                reifier: None,
                triple: None,
            },
            Term {
                kind: TermKind::Triple,
                value: None,
                datatype: None,
                lang: None,
                direction: None,
                reifier: Some(2),
                triple: Some((4, 5, 6)),
            },
        ];
        for term in spread {
            let expected = purrdf_gts::writer::term_to_wire(&term);
            assert_eq!(
                term_into_wire(term.clone()),
                expected,
                "the by-value encoder drifted from the writer for {term:?}"
            );
            assert_eq!(
                canonical(&term_into_wire(term.clone())),
                canonical(&expected),
                "…and its canonical bytes drifted for {term:?}"
            );
        }
    }

    fn ingest(text: &str, media_type: &str) -> SnapshotBuilder {
        let ds = parse_dataset(text.as_bytes(), media_type, None).expect("parse dataset");
        let mut b = SnapshotBuilder::default();
        b.add_dataset(&ds).expect("add_dataset");
        b
    }

    fn ingest_nq(nq: &str) -> SnapshotBuilder {
        ingest(nq, "application/n-quads")
    }

    /// RDF 1.2 base direction is part of a literal's IDENTITY through the composer.
    ///
    /// The intern key was `(kind, value, datatype, lang)` with no direction, so
    /// `"Cat"@en--ltr` and `"Cat"@en--rtl` hashed to one slot and MERGED into a
    /// single term — three distinct literals became one and every quad was
    /// repointed at it. That is a conflation, not a dropped column: the emitted
    /// graph was a different graph.
    ///
    /// `gts_write`'s path already carried direction and had a test for it
    /// (`gts_write::tests`), which is precisely how the two write paths came to
    /// disagree without anything noticing — only one of them was asked.
    #[test]
    fn base_direction_separates_otherwise_identical_literals() {
        let b = ingest_nq(
            "<http://example.org/c> <http://example.org/p> \"Cat\"@en--ltr .\n\
             <http://example.org/c> <http://example.org/p> \"Cat\"@en--rtl .\n\
             <http://example.org/c> <http://example.org/q> \"Cat\"@en .\n",
        );
        let (terms, quads, _reifies, _annot) = b.canonical_tables();

        let mut literals: Vec<(Option<&str>, Option<&str>)> = terms
            .iter()
            .filter(|t| t.kind == TermKind::Literal)
            .map(|t| (t.lang.as_deref(), t.direction.as_deref()))
            .collect();
        literals.sort_unstable();
        assert_eq!(
            literals,
            vec![
                (Some("en"), None),
                (Some("en"), Some("ltr")),
                (Some("en"), Some("rtl")),
            ],
            "three literals differing only in base direction must stay three terms"
        );
        assert_eq!(quads.len(), 3, "and each quad keeps its own object");

        // THE NEIGHBOURING CASE: interning must still COLLAPSE genuine duplicates.
        // A fix that made every literal unique would pass the assertion above and
        // silently stop deduplicating, which is the mirror-image defect.
        let dup = ingest_nq(
            "<http://example.org/c> <http://example.org/p> \"Cat\"@en--ltr .\n\
             <http://example.org/d> <http://example.org/p> \"Cat\"@en--ltr .\n",
        );
        let (dup_terms, dup_quads, _r, _a) = dup.canonical_tables();
        assert_eq!(
            dup_terms
                .iter()
                .filter(|t| t.kind == TermKind::Literal)
                .count(),
            1,
            "two occurrences of the SAME directional literal are still one term"
        );
        assert_eq!(dup_quads.len(), 2);
    }

    /// Re-render a read-back GTS container [`Graph`] to N-Quads through the native
    /// codec (`dataset_from_gts_graph` → `serialize_dataset`), never the purrdf-gts
    /// codec — purrdf-gts is the purrdf.gts container layer only.
    fn graph_nquads(graph: &purrdf_gts::model::Graph) -> String {
        let dataset =
            crate::gts::dataset_from_gts_graph(graph).expect("fold the GTS graph into a dataset");
        let bytes = crate::serialize_dataset(
            &dataset,
            crate::NativeRdfFormat::NQuads.media_type(),
            crate::SerializeGraph::Dataset,
        )
        .expect("serialize the dataset to N-Quads");
        String::from_utf8(bytes).expect("native N-Quads is valid UTF-8")
    }

    #[test]
    fn add_dataset_interns_expected_plain_graph_rows() {
        // Native carrier ingestion (the single-exit path) of a plain multi-graph dataset
        // exercising every term shape: IRI object, bare literal, lang-tagged literal,
        // explicit `xsd:string` (folds with the bare literal), and a named-graph quad.
        let nq = concat!(
            "<https://e/s> <https://e/p> <https://e/o> .\n",
            "<https://e/s> <https://e/p2> \"lit\" .\n",
            "<https://e/s> <https://e/p3> \"tagged\"@en .\n",
            "<https://e/s2> <https://e/p> ",
            "\"x\"^^<http://www.w3.org/2001/XMLSchema#string> .\n",
            "<https://e/s> <https://e/p> <https://e/o2> <https://e/g> .\n",
        );
        let ds = parse_dataset(nq.as_bytes(), "application/n-quads", None).expect("parse dataset");
        let mut native = SnapshotBuilder::default();
        native.add_dataset(&ds).expect("add_dataset");
        let (terms, quads, reifies, annot) = native.canonical_tables();
        assert!(reifies.is_empty(), "no statement layer");
        assert!(annot.is_empty(), "no annotations");
        // Five base quads (the explicit xsd:string literal stays its own quad row).
        assert_eq!(quads.len(), 5, "five base quad rows");
        // One named-graph quad: exactly one row carries a graph id.
        assert_eq!(
            quads.iter().filter(|(_, _, _, g)| g.is_some()).count(),
            1,
            "exactly one named-graph row"
        );
        // Literals: the bare "lit", the explicit `xsd:string` "x" (stored WITHOUT a
        // datatype — xsd:string is implicit), and the lang-tagged "tagged"@en. Three
        // distinct lexical values ⇒ three literal term rows. Every other term is an IRI.
        let literals = terms.iter().filter(|t| t.kind == TermKind::Literal).count();
        assert_eq!(literals, 3, "three distinct literal values");
        assert!(
            terms
                .iter()
                .filter(|t| t.kind == TermKind::Literal)
                .all(|t| t.datatype.is_none()),
            "xsd:string is implicit; no literal carries an explicit datatype id"
        );
        assert!(
            terms.iter().filter(|t| t.kind == TermKind::Iri).count() >= 6,
            "subject/predicate/object/graph IRIs all interned"
        );
    }

    #[test]
    fn add_dataset_folds_statement_layer_into_side_tables() {
        // A reifier with the canonical `rdf:reifies <<( s p o )>>` shape plus annotation
        // properties on the reifier subject — the exact statement-layer pattern. The
        // native `parse_dataset` folds it into the dataset's reifier/annotation side
        // tables, which `add_dataset` maps straight onto `reifies`/`annot`.
        let ttl = concat!(
            "<https://e/claim> ",
            "<http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ",
            "<<( <https://e/s> <https://e/p> <https://e/o> )>> ;\n",
            "  <https://e/accordingTo> <https://e/who> ;\n",
            "  <https://e/confidence> \"0.9\"^^<http://www.w3.org/2001/XMLSchema#decimal> .\n",
            "<https://e/s> <https://e/p> <https://e/o> .\n",
        );
        let ds = parse_dataset(ttl.as_bytes(), "text/turtle", None).expect("parse dataset");
        let mut native = SnapshotBuilder::default();
        native.add_dataset(&ds).expect("add_dataset");
        let (_terms, quads, reifies, annot) = native.canonical_tables();
        assert_eq!(reifies.len(), 1, "one reifies binding");
        assert_eq!(annot.len(), 2, "accordingTo + confidence annotations");
        // The single base quad `<s> <p> <o>` survives as a plain quad row; the reifier
        // subject's other triples ride the annotation table, not the base quads.
        assert_eq!(
            quads.len(),
            1,
            "one base quad; reifier triples are annotations"
        );
    }

    #[test]
    fn content_sort_is_iris_first_then_value() {
        let b = ingest_nq(
            "<https://e/s> <https://e/p> \"z\" .\n<https://e/s> <https://e/p> <https://e/a> .\n",
        );
        let (terms, _quads, _r, _a) = b.canonical_tables();
        let (last, rest) = terms.split_last().expect("non-empty");
        assert_eq!(last.kind, TermKind::Literal);
        assert!(rest.iter().all(|t| t.kind == TermKind::Iri));
    }

    #[test]
    fn xsd_string_datatype_is_implicit() {
        let b = ingest_nq(concat!(
            "<https://e/s> <https://e/p> \"x\" .\n",
            "<https://e/s2> <https://e/p> ",
            "\"x\"^^<http://www.w3.org/2001/XMLSchema#string> .\n",
        ));
        let (terms, _q, _r, _a) = b.canonical_tables();
        let literals = terms.iter().filter(|t| t.kind == TermKind::Literal).count();
        assert_eq!(
            literals, 1,
            "explicit xsd:string folds with the bare literal"
        );
    }

    #[test]
    fn snapshot_content_id_is_order_independent() {
        let a = ingest_nq(
            "<https://e/a> <https://e/p> <https://e/b> .\n<https://e/c> <https://e/p> <https://e/d> .\n",
        );
        let b = ingest_nq(
            "<https://e/c> <https://e/p> <https://e/d> .\n<https://e/a> <https://e/p> <https://e/b> .\n",
        );
        assert_eq!(a.snapshot_content_id(), b.snapshot_content_id());
        assert!(a.snapshot_content_id().starts_with("blake3:"));
    }

    #[test]
    fn rdf12_reifier_classifies_annotations() {
        let ds = parse_dataset(
            concat!(
                "<https://e/r> ",
                "<http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ",
                "<<( <https://e/s> <https://e/p> <https://e/o> )>> .\n",
                "<https://e/r> <https://e/confidence> \"0.9\" .\n",
            )
            .as_bytes(),
            "application/n-triples",
            None,
        )
        .expect("parse rdf12");
        let mut b = SnapshotBuilder::default();
        b.add_dataset(&ds).expect("ingest");
        let (_terms, quads, reifies, annot) = b.canonical_tables();
        assert_eq!(reifies.len(), 1, "one reifies binding");
        assert_eq!(annot.len(), 1, "one annotation row");
        assert!(quads.is_empty(), "reifier subject is not a base quad");
    }

    #[test]
    fn one_reifier_may_bind_several_triples() {
        // `rdf:reifies` is not a functional property, so two DIFFERENT triple
        // terms for one reifier subject are both assertable. Neither the parse
        // nor the snapshot producer may refuse or collapse them.
        let ds = parse_dataset(
            concat!(
                "<https://e/r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ",
                "<<( <https://e/s> <https://e/p> <https://e/o1> )>> .\n",
                "<https://e/r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ",
                "<<( <https://e/s> <https://e/p> <https://e/o2> )>> .\n",
            )
            .as_bytes(),
            "application/n-triples",
            None,
        )
        .expect("two bindings of one reifier are ordinary RDF 1.2");
        assert_eq!(ds.owned_reifiers().count(), 2);

        let mut b = SnapshotBuilder::default();
        b.add_dataset(&ds).expect("ingest");
        let (_terms, _quads, reifies, _annot) = b.canonical_tables();
        assert_eq!(reifies.len(), 2, "both bindings reach the snapshot frame");

        // And the emitted row order is a pure function of the content: the same
        // dataset always yields the same table.
        let mut again = SnapshotBuilder::default();
        again.add_dataset(&ds).expect("ingest");
        assert_eq!(again.canonical_tables().2, reifies);
    }

    #[test]
    fn statement_layer_graphs_survive_snapshot_composition() {
        let graphs = ["", "<https://example.org/world-a>", "_:world-b"];
        let sources: Vec<_> = graphs
            .iter()
            .map(|graph| {
                parse_dataset(
                    format!(
                        r#"{graph} {{
                            <https://example.org/claim>
                                <{RDF_REIFIES}>
                                <<( <https://example.org/s> <https://example.org/p> "مرحبا"@ar--rtl )>> ;
                                <https://example.org/accordingTo> <https://example.org/observer> .
                            <https://example.org/record> <https://example.org/cites> <https://example.org/claim> .
                        }}"#
                    )
                    .as_bytes(),
                    "application/trig",
                    None,
                )
                .expect("parse statement graph")
            })
            .collect();
        let mut builder = SnapshotBuilder::new();
        let mut reversed = SnapshotBuilder::new();
        for source in &sources {
            builder.add_dataset(source).unwrap();
        }
        for source in sources.iter().rev() {
            reversed.add_dataset(source).unwrap();
            reversed.add_dataset(source).unwrap();
        }
        assert_eq!(builder.snapshot_payload(), reversed.snapshot_payload());
        let (_, quads, reifiers, annotations) = builder.canonical_tables();
        assert_eq!(quads.len(), 3);
        assert_eq!(
            reifiers.len(),
            3,
            "the same binding belongs to three graphs"
        );
        assert_eq!(annotations.len(), 3);
        let bytes = emit_gts(
            &builder,
            "dist",
            Some(vec!["identity".to_owned()]),
            Vec::new(),
            Vec::new(),
            None,
            None,
            None,
            DEFAULT_RSYNCABLE_THRESHOLD,
            &MediumPlan::undicted(None),
        )
        .unwrap();
        let actual =
            crate::gts::dataset_from_gts_graph(&purrdf_gts::reader::read(&bytes, true, None))
                .unwrap();
        let expected =
            crate::RdfDataset::union(&sources.iter().map(AsRef::as_ref).collect::<Vec<_>>());
        // LAW: `actual`/`expected` are round-tripped from the test's own self-built
        // `sources` fixture — never caller-supplied and never reachable through a
        // binding, so the panicking wrapper is sound here.
        assert_eq!(
            crate::canonical_flat_nquads(&actual).unwrap(),
            crate::canonical_flat_nquads(&expected).unwrap(),
        );
    }

    #[test]
    fn scoped_ingestion_relocates_all_default_graph_record_kinds() {
        let source = parse_dataset(
            format!(
                r"_:claim <{RDF_REIFIES}> <<( _:s <https://example.org/p> _:o )>> ;
                    <https://example.org/evidence> _:s .
                _:record <https://example.org/cites> _:claim .
                _:world {{
                    _:claim <{RDF_REIFIES}> <<( _:s <https://example.org/p> _:o )>> ;
                        <https://example.org/evidence> _:s .
                    _:record <https://example.org/cites> _:claim .
                }}"
            )
            .as_bytes(),
            "application/trig",
            None,
        )
        .unwrap();
        let mut builder = SnapshotBuilder::new();
        builder
            .add_dataset_scoped(
                &source,
                Some("https://example.org/selected"),
                Some("source"),
            )
            .unwrap();
        let (terms, quads, reifiers, annotations) = builder.canonical_tables();
        let graph_ids: std::collections::BTreeSet<_> = quads.iter().map(|q| q.3).collect();
        assert!(!graph_ids.contains(&None));
        assert_eq!(graph_ids.len(), 2);
        assert_eq!(graph_ids, reifiers.iter().map(|r| r.2).collect());
        assert_eq!(graph_ids, annotations.iter().map(|a| a.3).collect());
        assert!(
            terms
                .iter()
                .filter(|t| t.kind == TermKind::Bnode)
                .all(|t| { t.value.as_deref().unwrap().starts_with("source-") })
        );
        assert!(reifiers.iter().all(|r| r.0 == reifiers[0].0));
        assert!(annotations.iter().all(|a| a.0 == reifiers[0].0));
    }

    #[test]
    fn default_and_named_graphs_round_trip() {
        let ds = parse_dataset(
            concat!(
                "<https://e/default> <https://e/p> <https://e/o> .\n",
                "<https://e/named> <https://e/p> \"v\"@en <https://e/g> .\n",
            )
            .as_bytes(),
            "application/n-quads",
            None,
        )
        .expect("parse");
        let mut builder = SnapshotBuilder::default();
        builder.add_dataset(&ds).expect("add_dataset");
        let bytes = emit_gts(
            &builder,
            "dist",
            Some(vec!["identity".to_string()]),
            Vec::new(),
            Vec::new(),
            None,
            None,
            None,
            DEFAULT_RSYNCABLE_THRESHOLD,
            &MediumPlan::undicted(None),
        )
        .expect("emit");
        let graph = purrdf_gts::reader::read(&bytes, true, None);
        let nquads = graph_nquads(&graph);
        assert!(nquads.contains("<https://e/default> <https://e/p> <https://e/o> ."));
        assert!(nquads.contains("<https://e/named> <https://e/p> \"v\"@en <https://e/g> ."));
    }

    #[test]
    fn blobs_are_additive_and_do_not_change_the_graph() {
        let builder = ingest_nq("<https://e/s> <https://e/p> <https://e/o> .\n");
        let base = emit_gts(
            &builder,
            "dist",
            Some(vec!["identity".to_string()]),
            Vec::new(),
            Vec::new(),
            None,
            None,
            None,
            DEFAULT_RSYNCABLE_THRESHOLD,
            &MediumPlan::undicted(None),
        )
        .expect("emit base");
        let with_blobs = emit_gts(
            &builder,
            "dist",
            Some(vec!["identity".to_string()]),
            vec![BlobRow {
                data: b"# docs\n".to_vec(),
                media_type: "text/markdown".to_string(),
                rep: "purrdf:doc/guide".to_string(),
            }],
            vec![BlobRow {
                data: b"{\"ok\":true}".to_vec(),
                media_type: "application/json".to_string(),
                rep: "purrdf:report/findings".to_string(),
            }],
            None,
            None,
            None,
            DEFAULT_RSYNCABLE_THRESHOLD,
            &MediumPlan::undicted(None),
        )
        .expect("emit blobs");
        let base_graph = purrdf_gts::reader::read(&base, true, None);
        let blob_graph = purrdf_gts::reader::read(&with_blobs, true, None);
        assert_eq!(graph_nquads(&base_graph), graph_nquads(&blob_graph));
        let reps: std::collections::BTreeSet<String> = blob_graph
            .blob_meta
            .iter()
            .filter_map(|(_, meta)| match meta {
                Value::Map(items) => items.iter().find_map(|(key, value)| {
                    if matches!(key, Value::Text(k) if k == "rep")
                        && let Value::Text(rep) = value
                    {
                        return Some(rep.clone());
                    }
                    None
                }),
                _ => None,
            })
            .collect();
        assert!(reps.contains("purrdf:doc/guide"));
        assert!(reps.contains("purrdf:report/findings"));
    }

    #[test]
    fn a_populated_medium_plan_requires_a_total_assignment() {
        let plan = MediumPlan {
            dicts: vec![("docs".to_string(), vec![1, 2, 3])],
            assignment: BTreeMap::new(),
            zstd_level: Some(12),
        };
        let err = plan
            .select(&FrameSlot::Snapshot)
            .expect_err("a populated plan may not omit a frame slot");
        assert!(err.contains("assigns none"), "{err}");
    }

    #[test]
    fn emit_gts_carries_a_populated_plan_into_the_header_and_frames() {
        let builder = ingest_nq("<https://e/s> <https://e/p> <https://e/o> .\n");
        let samples: Vec<Vec<u8>> = (0..100)
            .map(|i| format!("documentation payload row {i} with repeated vocabulary\n").into())
            .collect();
        let refs: Vec<&[u8]> = samples.iter().map(Vec::as_slice).collect();
        let dict = purrdf_gts::dict::raw_content_dict(&refs, 1024).expect("dictionary builds");
        let rep = "purrdf:doc/guide".to_string();
        let plan = MediumPlan {
            dicts: vec![("docs".to_string(), dict)],
            assignment: BTreeMap::from([
                (
                    FrameSlot::Blob(rep.clone()),
                    DictSelection::Named("docs".to_string()),
                ),
                (
                    FrameSlot::Snapshot,
                    DictSelection::Named("docs".to_string()),
                ),
            ]),
            zstd_level: Some(12),
        };
        let bytes = emit_gts(
            &builder,
            "dist",
            Some(vec!["zstd".to_string()]),
            vec![BlobRow {
                data: b"# dictionary-backed documentation\n".to_vec(),
                media_type: "text/markdown".to_string(),
                rep,
            }],
            Vec::new(),
            None,
            None,
            None,
            DEFAULT_RSYNCABLE_THRESHOLD,
            &plan,
        )
        .expect("dict-primed snapshot emits");

        let state = purrdf_gts::reader::segment_append_state(&bytes).expect("header parses");
        assert_eq!(
            state.dicts.keys().map(String::as_str).collect::<Vec<_>>(),
            vec!["docs"],
            "the named dictionary must be pinned in the header"
        );
        let dict_ids: std::collections::BTreeSet<i64> = state
            .catalog
            .iter()
            .filter(|row| row.dct.as_deref() == Some("docs"))
            .map(|row| row.id)
            .collect();
        let (items, torn) = purrdf_gts::wire::iter_items(&bytes);
        assert!(torn.is_none(), "complete emission");
        let selected: Vec<i64> = items
            .iter()
            .filter_map(|(_, item)| {
                let Value::Map(frame) = item else {
                    return None;
                };
                let Some(Value::Array(chain)) = purrdf_gts::wire::map_get(frame, "x") else {
                    return None;
                };
                let [Value::Integer(raw)] = chain.as_slice() else {
                    panic!("each emitted payload must ride one transform");
                };
                Some(i64::try_from(i128::from(*raw)).expect("catalog id fits"))
            })
            .collect();
        assert_eq!(selected.len(), 2, "one blob plus one snapshot payload");
        assert!(
            selected.iter().all(|id| dict_ids.contains(id)),
            "every emitted payload must select the docs-bound catalog entry: {selected:?}"
        );
    }

    #[test]
    fn rsyncable_threshold_only_rewrites_default_zstd() {
        assert_eq!(
            choose_transform(
                &["zstd".to_string()],
                DEFAULT_RSYNCABLE_THRESHOLD,
                DEFAULT_RSYNCABLE_THRESHOLD,
            ),
            vec!["zstd".to_string()]
        );
        assert_eq!(
            choose_transform(&["zstd".to_string()], 10, 1),
            vec!["zstd-rsyncable".to_string()]
        );
        assert_eq!(
            choose_transform(&["identity".to_string()], 10, 1),
            vec!["identity".to_string()]
        );
    }

    /// (i) The declared zstd level is CHAIN-gated, never PROFILE-gated.
    ///
    /// Before, `emit_gts` computed `profile == "dist" && chain_is_zstd` itself,
    /// so level 12 was unreachable under any other profile name no matter what
    /// the caller asked for — a silent capability degradation keyed on a string.
    /// The level now comes from the caller's [`MediumPlan`] and is filtered only
    /// by whether the chain carries a zstd-family codec.
    #[test]
    fn the_declared_zstd_level_is_chain_gated_not_profile_gated() {
        let builder = ingest_nq("<https://e/s> <https://e/p> <https://e/o> .\n");
        let emit = |profile: &str, chain: Vec<String>, plan: &MediumPlan| {
            emit_gts(
                &builder,
                profile,
                Some(chain),
                Vec::new(),
                Vec::new(),
                None,
                None,
                None,
                DEFAULT_RSYNCABLE_THRESHOLD,
                plan,
            )
        };
        let declared_levels = |bytes: &[u8]| -> Vec<Option<i32>> {
            purrdf_gts::reader::segment_append_state(bytes)
                .expect("header parses")
                .catalog
                .iter()
                .filter(|row| matches!(row.name.as_str(), "zstd" | "zstd-rsyncable"))
                .map(|row| row.level)
                .collect()
        };
        let rsyncable = || vec!["zstd-rsyncable".to_string()];

        // A profile that is emphatically NOT "dist" still records level 12.
        let custom = emit(
            "urn:purrdf:profile:not-dist",
            rsyncable(),
            &MediumPlan::undicted(Some(12)),
        )
        .expect("a non-dist profile may declare a level");
        let custom_levels = declared_levels(&custom);
        assert!(!custom_levels.is_empty(), "zstd-family entries exist");
        assert!(
            custom_levels.iter().all(|level| *level == Some(12)),
            "a non-\"dist\" profile with an explicit level must record it: {custom_levels:?}"
        );

        // The OTHER direction, which is what actually falsifies profile
        // inference: the literal "dist" profile with no declared level records
        // NO level. If any path still inferred from the profile string, this
        // would come back as Some(12).
        let dist_unlevelled =
            emit("dist", rsyncable(), &MediumPlan::undicted(None)).expect("emit dist");
        assert!(
            declared_levels(&dist_unlevelled)
                .iter()
                .all(Option::is_none),
            "the \"dist\" profile must NOT conjure a level the caller did not declare"
        );

        // And the profile string is inert: same plan, same chain, two different
        // profile names, identical declared levels.
        let dist_levelled = emit("dist", rsyncable(), &MediumPlan::undicted(Some(12)))
            .expect("emit dist with a level");
        assert_eq!(
            declared_levels(&dist_levelled),
            custom_levels,
            "the profile string must not change the declared level"
        );

        // A level with nothing to apply it to is a HARD ERROR, not a silent drop.
        let err = emit(
            "dist",
            vec!["identity".to_string()],
            &MediumPlan::undicted(Some(12)),
        )
        .expect_err("a level on a non-zstd chain must hard-fail");
        assert!(err.contains("no zstd-family codec"), "{err}");
    }

    /// [`MediumPlan::dist_default`] is likewise gated on the CHAIN it is handed.
    #[test]
    fn the_default_medium_plan_declares_a_level_only_for_a_zstd_chain() {
        assert_eq!(
            MediumPlan::dist_default(Some(&["zstd-rsyncable".to_string()])).zstd_level,
            Some(DIST_ZSTD_LEVEL)
        );
        assert_eq!(
            MediumPlan::dist_default(Some(&["zstd".to_string()])).zstd_level,
            Some(DIST_ZSTD_LEVEL)
        );
        assert_eq!(
            MediumPlan::dist_default(Some(&["identity".to_string()])).zstd_level,
            None,
            "a level is meaningless without a zstd-family codec"
        );
        assert_eq!(
            MediumPlan::dist_default(Some(&["gzip".to_string()])).zstd_level,
            None
        );
        // `None` means "the caller stated no chain", and `emit_gts` then defaults
        // to `["zstd"]` — so the level must ride along with that default.
        assert_eq!(
            MediumPlan::dist_default(None).zstd_level,
            Some(DIST_ZSTD_LEVEL)
        );
    }

    #[test]
    fn partial_signing_configuration_is_rejected() {
        let builder = ingest_nq("<https://e/s> <https://e/p> <https://e/o> .\n");
        let err = emit_gts(
            &builder,
            "dist",
            None,
            Vec::new(),
            Vec::new(),
            None,
            Some("kid".to_string()),
            None,
            DEFAULT_RSYNCABLE_THRESHOLD,
            &MediumPlan::undicted(None),
        )
        .expect_err("partial signing must hard-fail");
        assert!(err.contains("all three or none"), "{err}");
    }
}
