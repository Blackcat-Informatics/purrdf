// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The pyo3-free GTS snapshot compose core.
//!
//! This is the byte-emitting heart of `src/purrdf_tools/gts_producer.py::_Builder`,
//! lifted out of the Python binding surface so the non-python
//! Rust consumers (`purrdf-pipeline`) can author a full multi-named-graph `dist`
//! snapshot — default graph + named graphs + RDF-1.2 reifier/annotation tables +
//! content-addressed blobs — without pulling pyo3.
//!
//! [`SnapshotBuilder`] interns terms (append-order, scope-aware blank nodes),
//! content-sorts scalars (`(kind, value, datatype-IRI, lang, direction)`, IRIs first),
//! then dependency-orders native quoted terms, and
//! [`emit_gts`] authors the single `dist`-profile `snapshot` frame preceded by the
//! blob frames (sorted by `(rep, decoded-bytes)`). All CBOR encoding,
//! canonicalization, frame-id chaining, and signing is delegated to `purrdf-gts`.
//!
//! The Python wrapper delegates to THIS core; there is one
//! definition of "the snapshot".

mod error;
use error::term_evidence;
pub use error::{GtsEmitError, GtsIngestError, IngestCheckpoint, OffendingTerm};
use purrdf_hash::error::StructuredError;

use std::collections::BTreeMap;
use std::hash::BuildHasher;

use purrdf_core::hash::table::HashTable;
use purrdf_gts::model::{AnnotationRow, ReifierRow, Term, TermKind, is_literal_direction};
use purrdf_gts::wire::digest_str;
use purrdf_gts::writer::Writer;
use purrdf_lex::allocation::{Admission, Memory, StorageError};
use purrdf_lex::cbor::{Value, head};
use purrdf_lex::walk::WorkList;

use crate::{
    BlankScope, DatasetView, DrainCheckpoint, FallibleDatasetView, FastHasher, RdfTextDirection,
    TermRef, checkpointed_drain,
};

use purrdf_core::dataset_view::TermGuard as _;
/// The `rdf:reifies` predicate IRI (RDF 1.2 statement layer).
pub use purrdf_iri::vocab::rdf::REIFIES as RDF_REIFIES;
use purrdf_xsd::datatype::XSD_STRING;
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
    /// Native components, hash-consed after their scoped identities are interned.
    triple: Option<(usize, usize, usize)>,
    /// Zero for scalar terms; quoted terms follow all their dependencies.
    nesting: usize,
}

/// The borrowed intern key of a NON-blank term row:
/// `(kind, value, datatype-or-empty, lang-or-empty, direction-or-empty, triple)`.
///
/// Borrowed, not owned: this is the key a per-row LOOKUP hashes and compares,
/// and a lookup that owned its key allocated five strings per ingested term
/// position just to discover the term was already interned. Insertion of a
/// genuinely new term still owns its strings — that is the dictionary itself,
/// not scratch.
type RowKey<'a> = (
    u8,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    Option<(usize, usize, usize)>,
);

/// The borrowed intern key of an already-stored row.
fn row_key_of(row: &TermRow) -> RowKey<'_> {
    (
        row.kind as u8,
        row.value.as_str(),
        row.datatype.as_deref().unwrap_or(""),
        row.lang.as_deref().unwrap_or(""),
        row.direction.as_deref().unwrap_or(""),
        row.triple,
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

/// Exact requested-layout peak of one ingestion's temporary term traversal.
/// Dictionary rows retain their existing owner; this account covers only the
/// source-id memo and continuation spill, including replacement allocations.
#[derive(Default)]
struct TermScratchAccount {
    peak: usize,
}

impl Admission for TermScratchAccount {
    fn resize(&mut self, live_bytes: usize) -> Result<(), StorageError> {
        self.peak = self.peak.max(live_bytes);
        Ok(())
    }
}

struct SourceTerms<I> {
    /// None is active on the current chain; Some is its completed target id.
    ids: crate::FastMap<I, Option<usize>>,
    /// Original physical map charge, retained across removals and reuse.
    bytes: usize,
}

/// An accumulating snapshot builder mirroring `gts_producer._Builder`.
///
/// Term ids are append-order during ingestion (process-unstable), then re-id'd
/// by content in `Self::canonical_tables` so the emitted bytes are a pure
/// function of the inputs.
#[derive(Debug)]
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

purrdf_hash::default_from_new!(SnapshotBuilder);

impl SnapshotBuilder {
    /// A fresh, empty builder; [`Default`] delegates here.
    #[must_use]
    pub fn new() -> Self {
        Self {
            terms: Vec::new(),
            index: HashTable::new(),
            bnode_keys: Vec::new(),
            bnode_index: HashTable::new(),
            bnode_wire: HashTable::new(),
            quads: Vec::new(),
            reifies: Vec::new(),
            annot: Vec::new(),
            poison: None,
            totals: IngestReport::default(),
        }
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
        let key = (TermKind::Iri as u8, iri, "", "", "", None);
        if let Some(id) = lookup_term_row(&self.terms, &self.index, key) {
            return id;
        }
        self.push_term_row(TermRow {
            kind: TermKind::Iri,
            value: iri.to_owned(),
            datatype: None,
            lang: None,
            direction: None,
            triple: None,
            nesting: 0,
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
            triple: None,
            nesting: 0,
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
            None,
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
            triple: None,
            nesting: 0,
        })
    }

    /// Hash-cons a quoted term by its already-interned component identities.
    fn intern_triple(&mut self, s: usize, p: usize, o: usize) -> usize {
        let triple = Some((s, p, o));
        let key = (TermKind::Triple as u8, "", "", "", "", triple);
        if let Some(id) = lookup_term_row(&self.terms, &self.index, key) {
            return id;
        }
        self.push_term_row(TermRow {
            kind: TermKind::Triple,
            value: String::new(),
            datatype: None,
            lang: None,
            direction: None,
            triple,
            nesting: self.terms[o].nesting + 1,
        })
    }

    /// Ingest a native [`RdfDataset`](crate::RdfDataset) carrier DIRECTLY — interning
    /// its quads and its folded RDF-1.2 reifier/annotation side-tables — without the
    /// flat-quad round-trip. This is how the in-memory carrier is serialized at the
    /// single exit: the dataset is already canonical (frozen, blank-nodes standardized
    /// apart by union), so every named graph and the statement layer fold in as-is. The
    /// reifier/annotation side-tables map straight onto `reifies`/`annot` — there is no
    /// `rdf:reifies` re-materialization (the native parse already folded them).
    ///
    /// # Errors
    /// A term cannot be represented by the snapshot's native tables, the builder
    /// is already poisoned by an earlier ingestion failure, or one of the
    /// refusals [`GtsIngestError`] enumerates applies. Native causes remain owned.
    pub fn add_dataset(&mut self, dataset: &crate::RdfDataset) -> Result<(), GtsIngestError> {
        self.add_dataset_scoped(dataset, None, None)
    }

    /// Ingest a native [`RdfDataset`](crate::RdfDataset) with the same source-partitioning
    /// hooks as flat-quad ingestion: `default_graph_name` assigns base
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
    /// refusals [`GtsIngestError`] enumerates applies. Native causes remain owned.
    pub fn add_dataset_scoped(
        &mut self,
        dataset: &crate::RdfDataset,
        default_graph_name: Option<&str>,
        scope: Option<&str>,
    ) -> Result<(), GtsIngestError> {
        // ONE PATH: the frozen carrier is itself a `DatasetView`/`FallibleDatasetView`,
        // so the flat surface is a delegation through the same ingestion core the view
        // surface uses, not a second definition of "ingest a dataset". The receipt is
        // discarded here; the cumulative figures stay
        // readable through [`Self::ingest_totals`].
        let _ = self.ingest_view(dataset, default_graph_name, scope)?;
        Ok(())
    }

    /// Ingest ANY [`FallibleDatasetView`] directly — no temporary
    /// [`RdfDataset`](crate::RdfDataset), no text round trip, no per-row owned term
    /// reconstruction. See [`Self::add_view_scoped`] for the partitioning hooks.
    ///
    /// # Errors
    /// [`GtsIngestError`], which is also what poisons the builder.
    pub fn add_view<D: FallibleDatasetView<ReadError: StructuredError>>(
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
    pub fn add_view_scoped<D: FallibleDatasetView<ReadError: StructuredError>>(
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
    fn ingest_view<D: FallibleDatasetView<ReadError: StructuredError>>(
        &mut self,
        view: &D,
        default_graph_name: Option<&str>,
        scope: Option<&str>,
    ) -> Result<IngestReport, GtsIngestError> {
        if let Some(poison) = &self.poison {
            // Naming the EARLIER failure, not this call: the only honest thing a
            // poisoned builder can say is which ingestion left it that way.
            return Err(GtsIngestError::poisoned(poison));
        }
        // ATOMIC PER INGESTION. A refusal can fire after an arbitrary number of
        // rows have already been interned, and the accessors that answer over the
        // tables — `snapshot_payload` and `snapshot_content_id` — are infallible
        // by frozen signature, so they cannot themselves report the poison. They
        // must therefore never have a truncated interior to describe: the failing
        // ingestion is taken back out, leaving the builder at exactly the state
        // the last fully-accepted ingestion left it in.
        let mark = self.mark();
        match view
            .checked_read(|view| self.ingest_view_rows(view, default_graph_name, scope))
            .map_err(GtsIngestError::source_read)
            .and_then(std::convert::identity)
        {
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
    fn ingest_view_rows<D: FallibleDatasetView<ReadError: StructuredError>>(
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
        let mut term_scratch = 0;
        let drained = checkpointed_drain(view, |v| {
            match self.consume_rows(v, default_graph_name, scope) {
                Ok((rows, graphs, scratch)) => {
                    rows_consumed = rows;
                    occupied_graphs = graphs;
                    term_scratch = scratch;
                }
                Err(err) => row_error = Some(err),
            }
        });
        if let Some(err) = row_error {
            return Err(err);
        }
        if let Err(failure) = drained {
            return Err(GtsIngestError::not_ready(
                match failure.checkpoint {
                    DrainCheckpoint::Before => IngestCheckpoint::BeforeRows,
                    DrainCheckpoint::After => IngestCheckpoint::AfterRows,
                },
                failure.error,
            ));
        }

        let mut declarations_omitted: Vec<String> = view
            .named_graphs()
            .filter(|graph| !occupied_graphs.contains(graph))
            .map(|graph| term_evidence(view, graph).map(|term| term.to_string()))
            .collect::<Result<_, _>>()?;
        declarations_omitted.sort_unstable();
        declarations_omitted.dedup();

        let report = IngestReport {
            rows_consumed,
            terms_interned: self.terms.len() - terms_before,
            declarations_omitted,
            scratch_bytes: self.scratch_bytes().saturating_add(term_scratch),
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
    fn consume_rows<D: DatasetView<ReadError: StructuredError>>(
        &mut self,
        view: &D,
        default_graph_name: Option<&str>,
        scope: Option<&str>,
    ) -> Result<(usize, std::collections::BTreeSet<D::Id>, usize), GtsIngestError> {
        let mut account = TermScratchAccount::default();
        let mut memory = Memory::new(&mut account);
        let mut terms = SourceTerms {
            ids: crate::FastMap::default(),
            bytes: 0,
        };
        let result = (|| {
            let mut rows_consumed = 0_usize;
            let mut occupied_graphs: std::collections::BTreeSet<D::Id> =
                std::collections::BTreeSet::new();
            let default_gid = default_graph_name.map(|name| self.intern_iri(name));

            // Objects admit native quoted terms. Subject, predicate and graph slots
            // retain their RDF 1.2 position restrictions; no refused row is omitted.
            for quad in view.quads() {
                let sid = self.intern_view_term(view, quad.s, scope, "quad subject")?;
                let pid = self.intern_view_iri(view, quad.p, "quad predicate")?;
                let oid = self.intern_view_object(
                    view,
                    quad.o,
                    scope,
                    "quad object",
                    &mut terms,
                    &mut memory,
                )?;
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
            // quoted-triple object into an ordinary quad.
            for binding in view.reifier_quads() {
                let rid = self.intern_view_term(view, binding.s, scope, "reifier term")?;
                let guard = view
                    .resolve(binding.o)
                    .map_err(GtsIngestError::source_read)?;
                let TermRef::Triple { s, p, o } = guard.term() else {
                    return Err(GtsIngestError::UnrepresentableTerm {
                        position: "reifier binding object",
                        term: term_evidence(view, binding.o)?,
                    });
                };
                let qs = self.intern_view_term(view, s, scope, "reified subject")?;
                let qp = self.intern_view_iri(view, p, "reified predicate")?;
                let qo = self.intern_view_object(
                    view,
                    o,
                    scope,
                    "reified object",
                    &mut terms,
                    &mut memory,
                )?;
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
                let oid = self.intern_view_object(
                    view,
                    annotation.o,
                    scope,
                    "annotation object",
                    &mut terms,
                    &mut memory,
                )?;
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
        })();
        drop(terms.ids);
        let released = memory
            .release_bytes(terms.bytes)
            .map_err(GtsIngestError::from);
        let (rows, graphs) = result?;
        released?;
        Ok((rows, graphs, memory.admission().peak))
    }

    /// Intern a scalar view term in a slot that does not admit quoted triples.
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
    fn intern_view_term<D: DatasetView<ReadError: StructuredError>>(
        &mut self,
        view: &D,
        id: D::Id,
        scope: Option<&str>,
        position: &'static str,
    ) -> Result<usize, GtsIngestError> {
        let guard = view.resolve(id).map_err(GtsIngestError::source_read)?;
        self.intern_resolved_term(view, id, guard.term(), scope, position)
    }

    /// Intern an object, preserving nested native triples without machine-stack
    /// recursion. RDF 1.2 allows a quoted triple only in the object component.
    fn intern_view_object<D: DatasetView<ReadError: StructuredError>>(
        &mut self,
        view: &D,
        id: D::Id,
        scope: Option<&str>,
        position: &'static str,
        terms: &mut SourceTerms<D::Id>,
        memory: &mut Memory<'_, TermScratchAccount>,
    ) -> Result<usize, GtsIngestError> {
        let mut parents: WorkList<(D::Id, usize, usize), 16> = WorkList::new();
        let result = (|| {
            let mut current = id;
            let mut object = loop {
                if let Some(&held) = terms.ids.get(&current) {
                    match held {
                        Some(id) => break id,
                        None => {
                            return Err(GtsIngestError::UnrepresentableTerm {
                                position: "cyclic quoted triple",
                                term: term_evidence(view, current)?,
                            });
                        }
                    }
                }
                let guard = view.resolve(current).map_err(GtsIngestError::source_read)?;
                let term = guard.term();
                let required = terms
                    .ids
                    .len()
                    .checked_add(1)
                    .ok_or(StorageError::SizeOverflow)?;
                purrdf_core::hash::reserve_map_with_retained_memory(
                    &mut terms.ids,
                    &mut terms.bytes,
                    required,
                    memory,
                )?;
                let TermRef::Triple { s, p, o } = term else {
                    let id = self.intern_resolved_term(view, current, term, scope, position)?;
                    terms.ids.insert(current, Some(id));
                    break id;
                };
                terms.ids.insert(current, None);
                let subject = view.resolve(s).map_err(GtsIngestError::source_read)?;
                if !matches!(subject.term(), TermRef::Iri(_) | TermRef::Blank { .. }) {
                    return Err(GtsIngestError::UnrepresentableTerm {
                        position: "quoted triple subject",
                        term: term_evidence(view, s)?,
                    });
                }
                let sid =
                    self.intern_resolved_term(view, s, subject.term(), scope, "quoted subject")?;
                let pid = self.intern_view_iri(view, p, "quoted predicate")?;
                parents.try_push_admitted((current, sid, pid), memory)?;
                current = o;
            };
            while let Some((source, s, p)) = parents.pop() {
                object = self.intern_triple(s, p, object);
                terms.ids.insert(source, Some(object));
            }
            Ok(object)
        })();
        let released = parents
            .release_admitted(memory)
            .map_err(GtsIngestError::from);
        let object = result?;
        released?;
        Ok(object)
    }

    /// Shared scalar normalization for bare slots and quoted-term leaves.
    fn intern_resolved_term<D: DatasetView<ReadError: StructuredError>>(
        &mut self,
        view: &D,
        id: D::Id,
        term: TermRef<'_, D::Id>,
        scope: Option<&str>,
        position: &'static str,
    ) -> Result<usize, GtsIngestError> {
        match term {
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
                let datatype_guard = view
                    .resolve(datatype)
                    .map_err(GtsIngestError::source_read)?;
                let TermRef::Iri(datatype_iri) = datatype_guard.term() else {
                    return Err(GtsIngestError::UnrepresentableTerm {
                        position: "literal datatype",
                        term: term_evidence(view, datatype)?,
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
                term: term_evidence(view, id)?,
            }),
        }
    }

    /// Intern a view term that MUST be an IRI (a predicate slot).
    fn intern_view_iri<D: DatasetView<ReadError: StructuredError>>(
        &mut self,
        view: &D,
        id: D::Id,
        position: &'static str,
    ) -> Result<usize, GtsIngestError> {
        let guard = view.resolve(id).map_err(GtsIngestError::source_read)?;
        match guard.term() {
            TermRef::Iri(iri) => Ok(self.intern_iri(iri)),
            _ => Err(GtsIngestError::UnrepresentableTerm {
                position,
                term: term_evidence(view, id)?,
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
    /// snapshot payload. Scalars retain their content order, with IRIs before
    /// literals; quoted terms follow all their component dependencies.
    fn canonical_tables(&self) -> CanonTables {
        let (order, remap) = self.canonical_order();
        let wire_terms = self.wire_terms(&order, &remap);
        let (quads, reifies, annot) = self.canonical_rows(&remap);
        (wire_terms, quads, reifies, annot)
    }

    /// The content order of the term table: `order[new_id]` is the old id, and
    /// `remap[old_id]` is the new one.
    fn canonical_order(&self) -> (Vec<usize>, Vec<usize>) {
        let n = self.terms.len();
        let mut order: Vec<usize> = (0..n).collect();
        // Scalars keep their existing order. Every quoted term follows its
        // components, and equal-height triples sort by canonical child ids.
        order.sort_unstable_by_key(|&id| self.terms[id].nesting);
        let mut remap = vec![0usize; n];
        let mut first = 0;
        while first < n {
            let nesting = self.terms[order[first]].nesting;
            let end =
                first + order[first..].partition_point(|&id| self.terms[id].nesting == nesting);
            order[first..end].sort_unstable_by(|&a, &b| {
                match (self.terms[a].triple, self.terms[b].triple) {
                    (Some((as_, ap, ao)), Some((bs, bp, bo))) => {
                        (remap[as_], remap[ap], remap[ao]).cmp(&(remap[bs], remap[bp], remap[bo]))
                    }
                    _ => term_order(&self.terms[a], &self.terms[b]),
                }
            });
            for (offset, &old) in order[first..end].iter().enumerate() {
                remap[old] = first + offset;
            }
            first = end;
        }
        (order, remap)
    }

    /// The new id of a term row's datatype IRI term (interned earlier, so it has
    /// an old id and thus a new id).
    fn datatype_id(&self, row: &TermRow, remap: &[usize]) -> Option<usize> {
        row.datatype.as_deref().map(|dt| {
            let key = (TermKind::Iri as u8, dt, "", "", "", None);
            let old_dt = lookup_term_row(&self.terms, &self.index, key)
                .expect("a literal's datatype IRI is interned before the literal");
            remap[old_dt]
        })
    }

    /// Wire terms in new-id order.
    fn wire_terms(&self, order: &[usize], remap: &[usize]) -> Vec<Term> {
        order
            .iter()
            .map(|&old| {
                let row = &self.terms[old];
                let datatype = self.datatype_id(row, remap);
                Term {
                    kind: row.kind,
                    value: Some(row.value.clone()),
                    datatype,
                    lang: row.lang.clone(),
                    direction: row.direction.clone(),
                    reifier: None,
                    triple: row.triple.map(|(s, p, o)| (remap[s], remap[p], remap[o])),
                }
            })
            .collect()
    }

    /// The quad table under `remap`, packed as `[graph + 1 (0 for the default
    /// graph), s, p, o]` in 32-bit ids, deduplicated and sorted by that key.
    ///
    /// The key orders the default graph first, then graphs by id, then `(s, p, o)`:
    /// the order the wire fixes. One flat vector sorted in place, 16 bytes a row,
    /// is all the memory this holds; term ids index a `u32` table, so they fit.
    fn canonical_quad_keys(&self, remap: &[usize]) -> Vec<[u32; 4]> {
        let id = |term: usize| {
            u32::try_from(remap[term]).expect("INVARIANT: term ids index a u32 table")
        };
        let mut keys: Vec<[u32; 4]> = self
            .quads
            .iter()
            .map(|&(s, p, o, g)| [g.map_or(0, |g| id(g) + 1), id(s), id(p), id(o)])
            .collect();
        keys.sort_unstable();
        keys.dedup();
        keys
    }

    /// The reifier and annotation tables under `remap`: remapped, sorted by the
    /// whole row and deduplicated.
    fn canonical_binding_rows(&self, remap: &[usize]) -> (Vec<ReifierRow>, Vec<AnnotationRow>) {
        // Reifies: sort by the WHOLE row. One reifier id may carry several
        // bindings (`rdf:reifies` is not functional), so sorting by the reifier
        // id alone would leave their order to the ingestion order — and the
        // emitted bytes must be a pure function of the content.
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

        let mut annot: Vec<AnnotationRow> = self
            .annot
            .iter()
            .map(|&(r, p, v, g)| (remap[r], remap[p], remap[v], g.map(|g| remap[g])))
            .collect();
        annot.sort_unstable();
        annot.dedup();
        (reifies, annot)
    }

    /// The quad, reifier and annotation tables under `remap`: remapped, deduplicated
    /// and sorted.
    fn canonical_rows(
        &self,
        remap: &[usize],
    ) -> (Vec<CanonQuad>, Vec<ReifierRow>, Vec<AnnotationRow>) {
        let quads = self
            .canonical_quad_keys(remap)
            .into_iter()
            .map(|[g, s, p, o]| {
                (
                    s as usize,
                    p as usize,
                    o as usize,
                    g.checked_sub(1).map(|g| g as usize),
                )
            })
            .collect();
        let (reifies, annot) = self.canonical_binding_rows(remap);
        (quads, reifies, annot)
    }

    /// Feed the legacy canonical writer from the same flat native interner.
    /// Its graph spelling omits a triple term's unused value field; the frozen
    /// snapshot payload retains its own existing empty-string spelling.
    pub(crate) fn into_graph(self) -> Result<purrdf_gts::model::Graph, GtsIngestError> {
        if let Some(poison) = &self.poison {
            return Err(poison.clone());
        }
        let (mut terms, quads, reifiers, annotations) = self.canonical_tables();
        for term in &mut terms {
            if term.kind == TermKind::Triple {
                term.value = None;
            }
        }
        Ok(purrdf_gts::model::Graph {
            terms,
            quads,
            reifiers,
            annotations,
            ..purrdf_gts::model::Graph::default()
        })
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
                            let mut row = vec![Value::from(s), Value::from(p), Value::from(o)];
                            if let Some(g) = g {
                                row.push(Value::from(g));
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
                            let mut row = vec![
                                Value::from(rid),
                                Value::from(s),
                                Value::from(p),
                                Value::from(o),
                            ];
                            if let Some(g) = g {
                                row.push(Value::from(g));
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
                            let mut row = vec![Value::from(r), Value::from(p), Value::from(v)];
                            if let Some(g) = g {
                                row.push(Value::from(g));
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
    ///
    /// The BLAKE3 opens with no hash domain: the preimage is the canonical CBOR
    /// snapshot payload alone. The id is its own kind, a snapshot content id,
    /// compared only against another snapshot content id; and it is a
    /// published identity recorded against the GTS archives it attests, so a
    /// domain cannot be added without breaking every recorded id for an
    /// existing archive.
    pub fn snapshot_content_id(&self) -> String {
        digest_str(&self.snapshot_canonical_bytes())
    }

    /// The canonical CBOR encoding of [`Self::snapshot_payload`], written straight
    /// from the tables: no intermediate `Value` tree and no copy of any term
    /// string. The encoder runs twice over the same tables, once to count and once
    /// to fill a buffer of exactly that size, so the buffer never grows.
    ///
    /// Byte-for-byte `canonical(&self.snapshot_payload())`; the pinned bundle
    /// digests and `snapshot_canonical_bytes_match_the_payload_tree` hold it there.
    pub(crate) fn snapshot_canonical_bytes(&self) -> Vec<u8> {
        let (order, remap) = self.canonical_order();
        let quads = self.canonical_quad_keys(&remap);
        let (reifies, annot) = self.canonical_binding_rows(&remap);
        let mut len = Count(0);
        self.write_snapshot(&mut len, &order, &remap, &quads, &reifies, &annot)
            .expect("counting accepts every write");
        let mut out = Vec::with_capacity(len.0);
        self.write_snapshot(&mut out, &order, &remap, &quads, &reifies, &annot)
            .expect("a Vec accepts every write");
        debug_assert_eq!(out.len(), len.0);
        out
    }

    /// The canonical payload map. Keys sit in the order of their encodings (RFC
    /// 8949 §4.2.1): equal-length text keys bytewise, shorter before longer.
    fn write_snapshot<W: std::io::Write>(
        &self,
        out: &mut W,
        order: &[usize],
        remap: &[usize],
        quads: &[[u32; 4]],
        reifies: &[ReifierRow],
        annot: &[AnnotationRow],
    ) -> std::io::Result<()> {
        let entries = 2 + u64::from(!reifies.is_empty()) + u64::from(!annot.is_empty());
        head::write_head(out, head::MAP, entries)?;
        if !annot.is_empty() {
            head::write_text(out, "annot")?;
            head::write_head(out, head::ARRAY, annot.len() as u64)?;
            for &(r, p, v, g) in annot {
                write_id_row(out, &[r, p, v], g)?;
            }
        }
        head::write_text(out, "quads")?;
        head::write_head(out, head::ARRAY, quads.len() as u64)?;
        for &[g, s, p, o] in quads {
            let graph = g.checked_sub(1).map(|g| g as usize);
            write_id_row(out, &[s as usize, p as usize, o as usize], graph)?;
        }
        head::write_text(out, "terms")?;
        head::write_head(out, head::ARRAY, order.len() as u64)?;
        for &old in order {
            self.write_term(out, &self.terms[old], remap)?;
        }
        if !reifies.is_empty() {
            head::write_text(out, "reifies")?;
            head::write_head(out, head::ARRAY, reifies.len() as u64)?;
            for &(rid, (s, p, o), g) in reifies {
                write_id_row(out, &[rid, s, p, o], g)?;
            }
        }
        Ok(())
    }

    /// One term as the writer spells it: the map [`term_into_wire`] builds, keys
    /// in canonical order (`k`, `l`, `v`, `dt`, `tt`, `dir`; `rf` is never set).
    fn write_term<W: std::io::Write>(
        &self,
        out: &mut W,
        row: &TermRow,
        remap: &[usize],
    ) -> std::io::Result<()> {
        let datatype = self.datatype_id(row, remap);
        let direction = row
            .direction
            .as_deref()
            .filter(|direction| is_literal_direction(direction));
        let entries = 2
            + u64::from(row.lang.is_some())
            + u64::from(datatype.is_some())
            + u64::from(row.triple.is_some())
            + u64::from(direction.is_some());
        head::write_head(out, head::MAP, entries)?;
        head::write_text(out, "k")?;
        head::write_head(out, head::UNSIGNED, row.kind as u64)?;
        if let Some(lang) = &row.lang {
            head::write_text(out, "l")?;
            head::write_text(out, lang)?;
        }
        head::write_text(out, "v")?;
        head::write_text(out, &row.value)?;
        if let Some(datatype) = datatype {
            head::write_text(out, "dt")?;
            head::write_head(out, head::UNSIGNED, datatype as u64)?;
        }
        if let Some((s, p, o)) = row.triple {
            head::write_text(out, "tt")?;
            write_id_row(out, &[remap[s], remap[p], remap[o]], None)?;
        }
        if let Some(direction) = direction {
            head::write_text(out, "dir")?;
            head::write_text(out, direction)?;
        }
        Ok(())
    }
}

/// An array of term ids, with the graph id appended when there is one.
fn write_id_row<W: std::io::Write>(
    out: &mut W,
    ids: &[usize],
    graph: Option<usize>,
) -> std::io::Result<()> {
    head::write_head(
        out,
        head::ARRAY,
        (ids.len() + usize::from(graph.is_some())) as u64,
    )?;
    for &id in ids.iter().chain(graph.as_ref()) {
        head::write_head(out, head::UNSIGNED, id as u64)?;
    }
    Ok(())
}

/// A sink that keeps only how many bytes it was given.
struct Count(usize);

impl std::io::Write for Count {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0 += buf.len();
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
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
    Value::Integer(purrdf_lex::cbor::Integer::from(n as i64))
}

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
/// `["zstd"]`: the GTS writer's snapshot rule, so a composed archive and a
/// written one pick the same transform.
pub use purrdf_gts::writer::choose_snapshot_transform as choose_transform;

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
    fn select(&self, slot: &FrameSlot) -> Result<Option<&str>, GtsEmitError> {
        if self.dicts.is_empty() {
            return Ok(None);
        }
        match self.assignment.get(slot) {
            Some(DictSelection::Named(name)) => Ok(Some(name)),
            Some(DictSelection::Baseline) => Ok(None),
            None => Err(GtsEmitError::MissingAssignment {
                dictionaries: self.dicts.len(),
                slot: slot.clone(),
            }),
        }
    }
}

/// Emit the snapshot bundle bytes from an accumulated builder (`_Builder.to_gts`).
///
/// The snapshot payload is encoded once, straight from the builder's tables, and
/// handed to the writer as the frame source. The builder is only borrowed, so it
/// stays alive until the call returns and can be reused after a refusal; a caller
/// that owns a builder it is done with should use [`emit_gts_owned`], which
/// releases the tables before the transform runs.
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
) -> Result<Vec<u8>, GtsEmitError> {
    emit_held(
        Held {
            borrowed: Some(builder),
            owned: None,
        },
        profile,
        transform,
        doc_blobs,
        report_blobs,
        signer_secret,
        signer_kid,
        public_key_armor,
        rsyncable_threshold,
        plan,
    )
}

/// [`emit_gts`] for a caller that is done with its builder.
///
/// Byte-for-byte the same bundle. The builder's tables are released once the
/// payload is encoded, before the transform runs, so compression does not run on
/// top of them. The cost is the retry: the builder is gone after a refusal, and a
/// caller that wants to try again ingests again.
#[allow(clippy::too_many_arguments)]
pub fn emit_gts_owned(
    builder: SnapshotBuilder,
    profile: &str,
    transform: Option<Vec<String>>,
    doc_blobs: Vec<BlobRow>,
    report_blobs: Vec<BlobRow>,
    signer_secret: Option<[u8; 32]>,
    signer_kid: Option<String>,
    public_key_armor: Option<String>,
    rsyncable_threshold: usize,
    plan: &MediumPlan,
) -> Result<Vec<u8>, GtsEmitError> {
    emit_held(
        Held {
            borrowed: None,
            owned: Some(builder),
        },
        profile,
        transform,
        doc_blobs,
        report_blobs,
        signer_secret,
        signer_kid,
        public_key_armor,
        rsyncable_threshold,
        plan,
    )
}

/// The builder an emission reads: borrowed, or owned and released early. Exactly
/// one of the two is set.
struct Held<'a> {
    borrowed: Option<&'a SnapshotBuilder>,
    owned: Option<SnapshotBuilder>,
}

impl Held<'_> {
    fn get(&self) -> &SnapshotBuilder {
        self.owned
            .as_ref()
            .or(self.borrowed)
            .expect("INVARIANT: a held builder is borrowed or owned")
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_held(
    held: Held<'_>,
    profile: &str,
    transform: Option<Vec<String>>,
    doc_blobs: Vec<BlobRow>,
    report_blobs: Vec<BlobRow>,
    signer_secret: Option<[u8; 32]>,
    signer_kid: Option<String>,
    public_key_armor: Option<String>,
    rsyncable_threshold: usize,
    plan: &MediumPlan,
) -> Result<Vec<u8>, GtsEmitError> {
    // TERMINAL POISON. Rollback restored the last accepted tables, but publishing
    // that accepted subset would omit content the caller requested. Refuse before
    // inspecting any authoring configuration, retaining the original failure.
    if let Some(poison) = held.get().poison() {
        let cause = purrdf_lex::allocation::retain_native_cause(poison.clone())
            .map_err(GtsEmitError::Storage)?;
        return Err(GtsEmitError::Poisoned(cause));
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
            return Err(GtsEmitError::SigningFields {
                secret_present: signer_secret.is_some(),
                kid_present: signer_kid.is_some(),
                armor_present: public_key_armor.is_some(),
            });
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
    if let Some(level) = plan.zstd_level.filter(|_| !chain_is_zstd) {
        return Err(GtsEmitError::ZstdLevelTransform {
            level,
            transform: base_chain,
        });
    }
    if !plan.dicts.is_empty() && !chain_is_zstd {
        return Err(GtsEmitError::DictionariesTransform {
            dictionaries: plan.dicts.len(),
            transform: base_chain,
        });
    }

    let mut writer = Writer::with_options(
        profile,
        purrdf_gts::writer::WriterOptions {
            dicts: plan.dicts.clone(),
            zstd_level,
            ..purrdf_gts::writer::WriterOptions::default()
        },
    )?;
    if signing {
        // The exhaustive presence match above sets signing only when all three
        // owned fields are Some; none has been consumed or changed since then.
        let secret = signer_secret.expect("signing implies a secret");
        let kid = signer_kid.expect("signing implies a kid");
        writer.sign_with(purrdf_ed25519::SigningKey::from_bytes(&secret), &kid);
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
        writer.add_frame_with_options("blob", options)?;
    }

    // The payload is encoded once. With a transform the canonical bytes go to the
    // writer as the frame's source and the builder is released before they are
    // compressed. With the empty chain the frame embeds the payload unencoded, so
    // the writer takes the tree.
    let (payload, raw, chain) = if base_chain.is_empty() {
        (Some(held.get().snapshot_payload()), None, Vec::new())
    } else {
        let bytes = held.get().snapshot_canonical_bytes();
        let chain = choose_transform(&base_chain, bytes.len(), rsyncable_threshold);
        (None, Some(bytes), chain)
    };
    // An owned builder is released here, before the transform; a borrowed one
    // is the caller's and this drops nothing.
    drop(held);
    let snapshot_dict = plan.select(&FrameSlot::Snapshot)?;
    let options = purrdf_gts::writer::FrameOptions {
        payload,
        raw,
        transform: chain,
        zstd_level,
        dict: snapshot_dict.map(str::to_string),
        ..Default::default()
    };
    writer.add_frame_with_options("snapshot", options)?;

    Ok(writer.into_bytes())
}

#[cfg(test)]
mod tests {
    //! Pure-Rust coverage of the `SnapshotBuilder` core (no Python interpreter):
    //! interning order, content sort, the snapshot payload, and the content-id.
    use super::*;
    use crate::parse_dataset;
    use purrdf_gts::wire::canonical;

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

    /// The direct encoder is the canonical encoding of the payload tree: the
    /// frame's source bytes, its content id and the embedded empty-chain tree all
    /// describe one payload, over every term kind the builder holds.
    #[test]
    fn snapshot_canonical_bytes_match_the_payload_tree() {
        let builder = ingest(
            &format!(
                "<https://e/g> {{ <https://e/s> <https://e/p> \"a\"@en--rtl . }}\n\
                 _:b <https://e/p> \"1\"^^<{}> .\n\
                 <https://e/s> <https://e/p> \"plain\" .\n\
                 <https://e/r> <{RDF_REIFIES}> <<( <https://e/a> <https://e/b> <<( <https://e/c> <https://e/d> \"x\" )>> )>> ;\n\
                    <https://e/evidence> \"e\" .\n",
                purrdf_xsd::datatype::XSD_INTEGER
            ),
            "application/trig",
        );
        assert_eq!(
            builder.snapshot_canonical_bytes(),
            canonical(&builder.snapshot_payload())
        );
        assert_eq!(
            SnapshotBuilder::new().snapshot_canonical_bytes(),
            canonical(&SnapshotBuilder::new().snapshot_payload())
        );
    }

    /// Text is measured in bytes, not characters, on the wire: multi-byte IRIs
    /// and literals, and a literal long enough to need a wider length head, encode
    /// as the payload tree encodes them.
    #[test]
    fn snapshot_canonical_bytes_match_the_payload_tree_for_non_ascii_text() {
        let long = "ü日".repeat(200);
        let builder = ingest(
            &format!(
                "<http://example.org/s/é日本> <http://example.org/p/ß> \"ünï 日本語 🐈\"@de .\n\
                 <http://example.org/s/é日本> <http://example.org/p/ß> \"{long}\" .\n\
                 <http://example.org/s/é日本> <http://example.org/p/ß> \"مرحبا\"@ar--rtl <http://example.org/g/ü> .\n"
            ),
            "application/n-quads",
        );
        assert!(builder.snapshot_canonical_bytes().len() > 600);
        assert_eq!(
            builder.snapshot_canonical_bytes(),
            canonical(&builder.snapshot_payload())
        );
    }

    /// A direction token the wire does not admit is left off the term, as the tree
    /// builder leaves it off; the neighbouring valid token is written.
    #[test]
    fn a_direction_that_is_not_a_base_direction_is_left_off_the_wire() {
        let builder = SnapshotBuilder::new();
        for (direction, written) in [("rtl", true), ("sideways", false)] {
            let row = TermRow {
                kind: TermKind::Literal,
                value: "x".to_owned(),
                datatype: None,
                lang: Some("en".to_owned()),
                direction: Some(direction.to_owned()),
                triple: None,
                nesting: 0,
            };
            let mut bytes = Vec::new();
            builder
                .write_term(&mut bytes, &row, &[])
                .expect("a Vec accepts every write");
            let tree = term_into_wire(Term {
                kind: TermKind::Literal,
                value: Some("x".to_owned()),
                datatype: None,
                lang: Some("en".to_owned()),
                direction: Some(direction.to_owned()),
                reifier: None,
                triple: None,
            });
            assert_eq!(bytes, canonical(&tree), "{direction}");
            assert_eq!(tree.get("dir").is_some(), written, "{direction}");
        }
    }

    /// The borrowing and the consuming entry points emit one bundle, and the
    /// borrowing one leaves the builder usable for another emission.
    #[test]
    fn borrowing_and_consuming_emission_agree_and_only_borrowing_keeps_the_builder() {
        let text = "<https://e/s> <https://e/p> \"v\"@en <https://e/g> .\n";
        let builder = ingest_nq(text);
        let emit = |chain: Option<Vec<String>>, owned: Option<SnapshotBuilder>| {
            let plan = MediumPlan::dist_default(chain.as_deref());
            match owned {
                None => emit_gts(
                    &builder,
                    "dist",
                    chain,
                    Vec::new(),
                    Vec::new(),
                    None,
                    None,
                    None,
                    DEFAULT_RSYNCABLE_THRESHOLD,
                    &plan,
                ),
                Some(owned) => emit_gts_owned(
                    owned,
                    "dist",
                    chain,
                    Vec::new(),
                    Vec::new(),
                    None,
                    None,
                    None,
                    DEFAULT_RSYNCABLE_THRESHOLD,
                    &plan,
                ),
            }
        };
        for chain in [None, Some(vec!["identity".to_owned()]), Some(Vec::new())] {
            let borrowed = emit(chain.clone(), None).expect("borrowed emits");
            let again = emit(chain.clone(), None).expect("a borrowed builder emits again");
            let owned = emit(chain.clone(), Some(ingest_nq(text))).expect("owned emits");
            assert_eq!(borrowed, again, "{chain:?}");
            assert_eq!(borrowed, owned, "{chain:?}");
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
        let bytes = emit_gts_owned(
            builder,
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
            crate::RdfDataset::union(&sources.iter().map(AsRef::as_ref).collect::<Vec<_>>())
                .unwrap();
        // LAW: `actual`/`expected` are round-tripped from the test's own self-built
        // `sources` fixture — never caller-supplied and never reachable through a
        // binding, so the panicking wrapper is sound here.
        assert_eq!(
            crate::canonical_flat_nquads(
                &actual,
                &mut purrdf_core::CanonicalizationControl::new(
                    purrdf_core::RDFC_V2_CORPUS_POLICY,
                    None
                )
            )
            .unwrap()
            .nquads(),
            crate::canonical_flat_nquads(
                &expected,
                &mut purrdf_core::CanonicalizationControl::new(
                    purrdf_core::RDFC_V2_CORPUS_POLICY,
                    None
                )
            )
            .unwrap()
            .nquads(),
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
        let bytes = emit_gts_owned(
            builder,
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
        let ingest = || ingest_nq("<https://e/s> <https://e/p> <https://e/o> .\n");
        let base = emit_gts_owned(
            ingest(),
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
        let with_blobs = emit_gts_owned(
            ingest(),
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
        assert!(err.to_string().contains("assigns none"), "{err}");
        assert!(matches!(
            err,
            GtsEmitError::MissingAssignment {
                dictionaries: 1,
                slot: FrameSlot::Snapshot
            }
        ));
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
        let bytes = emit_gts_owned(
            builder,
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
        let emit = |profile: &str, chain: Vec<String>, plan: &MediumPlan| {
            emit_gts_owned(
                ingest_nq("<https://e/s> <https://e/p> <https://e/o> .\n"),
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
        assert!(err.to_string().contains("no zstd-family codec"), "{err}");
        assert!(
            matches!(err, GtsEmitError::ZstdLevelTransform { level: 12, transform } if transform == ["identity"])
        );
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
        let err = emit_gts_owned(
            builder,
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
        assert!(err.to_string().contains("all three or none"), "{err}");
        assert!(matches!(
            err,
            GtsEmitError::SigningFields {
                secret_present: false,
                kid_present: true,
                armor_present: false
            }
        ));
    }
}
