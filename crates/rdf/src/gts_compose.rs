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
//! content-sorts them (`(kind, value, datatype-IRI, lang)`, IRIs first), and
//! [`emit_gts`] authors the single `dist`-profile `snapshot` frame preceded by the
//! blob frames (sorted by `(rep, decoded-bytes)`). All CBOR encoding,
//! canonicalization, frame-id chaining, and signing is delegated to `purrdf-gts`.
//!
//! The Python wrapper delegates to THIS core; there is one
//! definition of "the snapshot".

use std::collections::{BTreeMap, HashMap};

use ciborium::value::Value;
use purrdf_gts::model::{Term, TermKind};
use purrdf_gts::wire::{blake3_256, canonical, hex};
use purrdf_gts::writer::{Writer, term_to_wire};

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
/// A remapped `(reifier, (s, p, o))` reifies binding in canonical term ids.
type CanonReifies = (usize, (usize, usize, usize));
/// A remapped `(reifier, predicate, object)` annotation in canonical term ids.
type CanonAnnot = (usize, usize, usize);
/// The fully canonical snapshot tables (`_Builder._canonical_tables`).
type CanonTables = (
    Vec<Term>,
    Vec<CanonQuad>,
    Vec<CanonReifies>,
    Vec<CanonAnnot>,
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
}

/// An accumulating snapshot builder mirroring `gts_producer._Builder`.
///
/// Term ids are append-order during ingestion (process-unstable), then re-id'd
/// by content in `Self::canonical_tables` so the emitted bytes are a pure
/// function of the inputs.
#[derive(Debug, Default)]
pub struct SnapshotBuilder {
    terms: Vec<TermRow>,
    /// Intern index keyed by `(kind, value, datatype-or-empty, lang-or-empty)`,
    /// matching the Python `_Interner` keys exactly.
    index: HashMap<(u8, String, String, String), usize>,
    /// Blank-node intern index keyed by `(scope, label)` (C0.2): two equal
    /// labels in different ingest scopes stay distinct terms.
    bnode_index: HashMap<(Option<String>, String), usize>,
    quads: Vec<(usize, usize, usize, Option<usize>)>,
    /// reifier-id → (s, p, o); a `Vec` preserving first-bind, dedup on rebind.
    reifies: Vec<(usize, (usize, usize, usize))>,
    annot: Vec<(usize, usize, usize)>,
}

impl SnapshotBuilder {
    /// A fresh, empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    fn intern_key(
        kind: u8,
        value: &str,
        datatype: Option<&str>,
        lang: Option<&str>,
    ) -> (u8, String, String, String) {
        (
            kind,
            value.to_owned(),
            datatype.unwrap_or("").to_owned(),
            lang.unwrap_or("").to_owned(),
        )
    }

    fn intern_iri(&mut self, iri: &str) -> usize {
        let key = Self::intern_key(TermKind::Iri as u8, iri, None, None);
        if let Some(&id) = self.index.get(&key) {
            return id;
        }
        let id = self.terms.len();
        self.terms.push(TermRow {
            kind: TermKind::Iri,
            value: iri.to_owned(),
            datatype: None,
            lang: None,
        });
        self.index.insert(key, id);
        id
    }

    fn intern_bnode(&mut self, label: &str, scope: Option<&str>) -> usize {
        // Scope-prefix the stored value exactly as Python's `_Interner.bnode`:
        // `None` keeps the raw label; a scope yields `"{scope}-{label}"`.
        let bkey = (scope.map(str::to_owned), label.to_owned());
        if let Some(&id) = self.bnode_index.get(&bkey) {
            return id;
        }
        let value = match scope {
            None => label.to_owned(),
            Some(scope) => format!("{scope}-{label}"),
        };
        let id = self.terms.len();
        self.terms.push(TermRow {
            kind: TermKind::Bnode,
            value,
            datatype: None,
            lang: None,
        });
        self.bnode_index.insert(bkey, id);
        id
    }

    fn intern_literal(&mut self, lex: &str, datatype: Option<&str>, lang: Option<&str>) -> usize {
        // Ensure the datatype IRI is interned (IRIs sort before literals, so the
        // datatype id always precedes the literal — §7.5, preserved here).
        if let Some(dt) = datatype {
            self.intern_iri(dt);
        }
        let key = Self::intern_key(TermKind::Literal as u8, lex, datatype, lang);
        if let Some(&id) = self.index.get(&key) {
            return id;
        }
        let id = self.terms.len();
        self.terms.push(TermRow {
            kind: TermKind::Literal,
            value: lex.to_owned(),
            datatype: datatype.map(str::to_owned),
            lang: lang.map(str::to_owned),
        });
        self.index.insert(key, id);
        id
    }

    /// Record a reifier binding, idempotent on an identical rebind.
    ///
    /// `rdf:reifies` is NOT a functional property, so one reifier id may bind
    /// several distinct triples; refusing the second binding would refuse
    /// ordinary RDF 1.2. Every binding is emitted as its own `reifies` row.
    fn bind_reifier(&mut self, rid: usize, spo: (usize, usize, usize)) {
        if !self
            .reifies
            .iter()
            .any(|&(r, existing)| r == rid && existing == spo)
        {
            self.reifies.push((rid, spo));
        }
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
    /// A conflicting reifier rebind (one reifier id bound to two different statements).
    pub fn add_dataset(&mut self, dataset: &crate::RdfDataset) -> Result<(), String> {
        self.add_dataset_scoped(dataset, None, None)
    }

    /// Ingest a native [`RdfDataset`](crate::RdfDataset) with the same source-partitioning
    /// hooks the legacy oxigraph ingestion exposed: `default_graph_name` assigns base
    /// quads carrying no graph of their own to a named graph, and `scope` prefixes
    /// blank-node labels (`"{scope}-{label}"`) so two equal labels in different ingest
    /// scopes stay distinct terms. With both `None` this is the plain carrier ingestion
    /// ([`Self::add_dataset`]). The blank scope applies to EVERY blank position (quads,
    /// reifiers, annotations) exactly as the old `add_quads`/`add_rdf12` did.
    ///
    /// # Errors
    /// A conflicting reifier rebind (one reifier id bound to two different statements).
    pub fn add_dataset_scoped(
        &mut self,
        dataset: &crate::RdfDataset,
        default_graph_name: Option<&str>,
        scope: Option<&str>,
    ) -> Result<(), String> {
        let default_gid = default_graph_name.map(|name| self.intern_iri(name));
        for quad in dataset.owned_quads() {
            // FAIL CLOSED (no-optionality): a carrier quad whose subject/object/graph is
            // not directly representable in the snapshot frame (a quoted-triple term, or
            // a non-IRI/blank graph name) is NOT silently dropped — that would make the
            // emitted `purrdf.gts` diverge from the canonical carrier. Quoted triples are
            // representable ONLY via the reifier/annotation tables (handled below), so a
            // Triple term in plain-quad position is genuine loss and aborts the emit.
            let sid = self.intern_required_native_term(&quad.subject, scope, "quad subject")?;
            let pid = self.intern_iri(&quad.predicate);
            let oid = self.intern_required_native_term(&quad.object, scope, "quad object")?;
            let gid = match &quad.graph_name {
                None => default_gid,
                Some(graph) => {
                    Some(self.intern_required_native_term(graph, scope, "quad graph name")?)
                }
            };
            self.quads.push((sid, pid, oid, gid));
        }
        for reifier in dataset.owned_reifiers() {
            let rid = self.intern_required_native_term(&reifier.reifier, scope, "reifier term")?;
            let qs = self.intern_required_native_term(
                &reifier.statement.subject,
                scope,
                "reified subject",
            )?;
            let qp = self.intern_iri(&reifier.statement.predicate);
            let qo = self.intern_required_native_term(
                &reifier.statement.object,
                scope,
                "reified object",
            )?;
            self.bind_reifier(rid, (qs, qp, qo));
        }
        for annot in dataset.owned_annotations() {
            let rid =
                self.intern_required_native_term(&annot.reifier, scope, "annotation reifier")?;
            let pid = self.intern_iri(&annot.predicate);
            let oid =
                self.intern_required_native_term(&annot.object, scope, "annotation object")?;
            self.annot.push((rid, pid, oid));
        }
        Ok(())
    }

    /// Intern a native term that MUST be representable in the snapshot frame, or fail
    /// closed. `position` names the slot for the diagnostic. A quoted-triple term has no
    /// direct term row (it rides the reifier/annotation tables), so it is an error here.
    fn intern_required_native_term(
        &mut self,
        term: &crate::RdfTerm,
        scope: Option<&str>,
        position: &str,
    ) -> Result<usize, String> {
        self.intern_native_term(term, scope).ok_or_else(|| {
            format!(
                "carrier {position} is not directly representable in the gts snapshot frame \
                 (quoted-triple terms must ride the reifier/annotation tables): {term:?}"
            )
        })
    }

    /// Intern a native term in subject/object/graph position (triple-terms are NOT
    /// interned — the RDF-1.2 layer rides the reifies/annot tables). Mirrors the legacy
    /// oxigraph ingestion's literal normalization (a language tag implies no datatype;
    /// `xsd:string` is implied and stored without a datatype) so the term rows are
    /// byte-identical. `scope` prefixes blank labels (`None` keeps the raw label) — a
    /// frozen carrier dataset has already standardized its blanks apart, so the carrier
    /// exit passes `None`; the Python multi-source producer passes per-source scopes.
    fn intern_native_term(&mut self, term: &crate::RdfTerm, scope: Option<&str>) -> Option<usize> {
        match term {
            crate::RdfTerm::Iri(iri) => Some(self.intern_iri(iri)),
            crate::RdfTerm::BlankNode(label) => Some(self.intern_bnode(label, scope)),
            crate::RdfTerm::Literal(literal) => {
                if let Some(lang) = &literal.language {
                    Some(self.intern_literal(&literal.lexical_form, None, Some(lang)))
                } else {
                    let datatype = match literal.datatype.as_deref() {
                        Some(dt) if dt == XSD_STRING => None,
                        other => other,
                    };
                    Some(self.intern_literal(&literal.lexical_form, datatype, None))
                }
            }
            crate::RdfTerm::Triple(_) => None,
        }
    }

    /// Re-id every term by content and sort every row (`_Builder._canonical_tables`).
    ///
    /// Returns the canonical `(wire_terms, quads, reifies, annot)` ready for the
    /// snapshot payload. Terms sort by `(kind, value, datatype-IRI, lang)` with
    /// IRIs first, so every literal's datatype IRI precedes it.
    fn canonical_tables(&self) -> CanonTables {
        let n = self.terms.len();
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by_key(|&a| self.sort_key(a));
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
                let datatype = row.datatype.as_ref().map(|dt| {
                    let old_dt = self.index[&Self::intern_key(TermKind::Iri as u8, dt, None, None)];
                    remap[old_dt]
                });
                Term {
                    kind: row.kind,
                    value: Some(row.value.clone()),
                    datatype,
                    lang: row.lang.clone(),
                    direction: None,
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
        let mut reifies: Vec<(usize, (usize, usize, usize))> = self
            .reifies
            .iter()
            .map(|&(rid, (s, p, o))| (remap[rid], (remap[s], remap[p], remap[o])))
            .collect();
        reifies.sort_unstable();
        reifies.dedup();

        // Annot: remap, dedup, sort.
        let mut annot_set: std::collections::BTreeSet<(usize, usize, usize)> =
            std::collections::BTreeSet::new();
        for &(r, p, v) in &self.annot {
            annot_set.insert((remap[r], remap[p], remap[v]));
        }
        let annot: Vec<(usize, usize, usize)> = annot_set.into_iter().collect();

        (wire_terms, quads, reifies, annot)
    }

    fn sort_key(&self, tid: usize) -> (u8, String, String, String) {
        let t = &self.terms[tid];
        let dt = t.datatype.clone().unwrap_or_default();
        (
            t.kind as u8,
            t.value.clone(),
            dt,
            t.lang.clone().unwrap_or_default(),
        )
    }

    /// The canonical `snapshot` frame payload (`_Builder._snapshot_payload`).
    pub fn snapshot_payload(&self) -> Value {
        let (terms, quads, reifies, annot) = self.canonical_tables();
        let mut entries: Vec<(Value, Value)> = vec![
            (
                "terms".into(),
                Value::Array(terms.iter().map(term_to_wire).collect()),
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
            // purrdf-gts 0.9.11 wire: `reifies` is a row-array `[[rid, s, p, o, g?], …]`
            // (was a reifier-id map). purrdf reification is standpoint-scoped, never
            // graph-scoped, so no row carries the optional trailing graph term-id —
            // matching the gts writer's `add_reifies` / snapshot payload byte-for-byte.
            entries.push((
                "reifies".into(),
                Value::Array(
                    reifies
                        .iter()
                        .map(|&(rid, (s, p, o))| Value::Array(vec![iv(rid), iv(s), iv(p), iv(o)]))
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
                        .map(|&(r, p, v)| Value::Array(vec![iv(r), iv(p), iv(v)]))
                        .collect(),
                ),
            ));
        }
        Value::Map(entries)
    }

    /// The `blake3:<hex>` content address of the snapshot payload
    /// (`_Builder.snapshot_content_id`).
    pub fn snapshot_content_id(&self) -> String {
        let bytes = canonical(&self.snapshot_payload());
        format!("blake3:{}", hex(&blake3_256(&bytes)))
    }
}

fn iv(n: usize) -> Value {
    Value::Integer(ciborium::value::Integer::from(n as u64))
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

    fn ingest(text: &str, media_type: &str) -> SnapshotBuilder {
        let ds = parse_dataset(text.as_bytes(), media_type, None).expect("parse dataset");
        let mut b = SnapshotBuilder::default();
        b.add_dataset(&ds).expect("add_dataset");
        b
    }

    fn ingest_nq(nq: &str) -> SnapshotBuilder {
        ingest(nq, "application/n-quads")
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
