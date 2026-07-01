// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The pyo3-free GTS snapshot compose core (#861 P6).
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

use std::collections::HashMap;

use ciborium::value::Value;
use purrdf_gts::model::{Term, TermKind};
use purrdf_gts::wire::{blake3_256, canonical, hex};
use purrdf_gts::writer::{term_to_wire, Writer};

/// The `rdf:reifies` predicate IRI (RDF 1.2 statement layer).
pub const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
/// Payloads larger than this select `zstd-rsyncable` over `zstd` (#513).
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
#[derive(Clone)]
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
/// by content in [`Self::canonical_tables`] so the emitted bytes are a pure
/// function of the inputs.
#[derive(Default)]
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

    /// Bind a reifier first-wins, erroring on a conflicting rebind (the producer's
    /// strict contract — the native statement-layer ingestion in [`Self::add_dataset`]).
    fn bind_reifier(&mut self, rid: usize, spo: (usize, usize, usize)) -> Result<(), String> {
        if let Some((_, existing)) = self.reifies.iter().find(|(r, _)| *r == rid) {
            if *existing != spo {
                return Err(format!("conflicting reifier rebind for term id {rid}"));
            }
            return Ok(());
        }
        self.reifies.push((rid, spo));
        Ok(())
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
            self.bind_reifier(rid, (qs, qp, qo))?;
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
                }
            })
            .collect();

        // Quads: remap, dedup, sort by (graph[None=-1], s, p, o).
        let mut quad_set: std::collections::BTreeSet<(i64, usize, usize, usize, Option<usize>)> =
            std::collections::BTreeSet::new();
        for &(s, p, o, g) in &self.quads {
            let g = g.map(|g| remap[g]);
            let gkey = g.map(|g| g as i64).unwrap_or(-1);
            quad_set.insert((gkey, remap[s], remap[p], remap[o], g));
        }
        let quads: Vec<(usize, usize, usize, Option<usize>)> = quad_set
            .into_iter()
            .map(|(_, s, p, o, g)| (s, p, o, g))
            .collect();

        // Reifies: remap, sort by reifier id (the Python dict is built in
        // remapped-id-sorted order; CBOR canonical re-sorts map keys anyway).
        let mut reifies: Vec<(usize, (usize, usize, usize))> = self
            .reifies
            .iter()
            .map(|&(rid, (s, p, o))| (remap[rid], (remap[s], remap[p], remap[o])))
            .collect();
        reifies.sort_by_key(|(rid, _)| *rid);

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
            )
        }
    };

    let base_chain = transform.unwrap_or_else(|| vec!["zstd".to_string()]);

    // Per-frame zstd level (purrdf-gts 0.9.11). The writer default is `Fastest` (~level
    // 1) — which is why switching the `dist` bundle to `zstd-rsyncable` bloated it
    // (16.7 MB gzip → 27 MB Fastest-zstd). The committed `dist` bundle is regenerated
    // often and lives in git, so a higher level pays off (smaller blob + smaller
    // git delta), while rsyncable keeps chunk boundaries stable. Other profiles are
    // not committed artifacts and keep the Fastest default.
    //
    // A `zstd_level` is meaningful only for a zstd-family frame; the writer hard-fails
    // a level paired with a non-zstd transform. Gate the level on the actual chain (not
    // just the `dist` profile name) so a caller may still emit a `dist`-profile snapshot
    // under `gzip`/`identity` — the production bundle passes `zstd-rsyncable`, so it
    // keeps level 12.
    let chain_is_zstd = base_chain
        .iter()
        .any(|t| t == "zstd" || t == "zstd-rsyncable");
    let zstd_level: Option<i32> = if profile == "dist" && chain_is_zstd {
        Some(DIST_ZSTD_LEVEL)
    } else {
        None
    };

    let mut writer = Writer::new(profile);
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
            ..Default::default()
        };
        writer
            .add_frame_with_options("blob", options)
            .map_err(|e| e.to_string())?;
    }

    let payload = builder.snapshot_payload();
    let snapshot_bytes = canonical(&payload);
    let chain = choose_transform(&base_chain, snapshot_bytes.len(), rsyncable_threshold);
    let options = purrdf_gts::writer::FrameOptions {
        payload: Some(payload),
        transform: chain,
        zstd_level,
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
    fn conflicting_reifier_rebind_is_rejected() {
        // FINDING (#909): two DIFFERENT `rdf:reifies` triple terms for one reifier
        // subject is HARD-rejected (CONSTITUTION P7, never silently last-write-win).
        // The native `parse_dataset` folds the statement layer during parse and detects
        // the conflicting rebind there ("conflicting rdf:reifies binding"), before the
        // bytes ever reach `SnapshotBuilder::add_dataset`. The conflict is surfaced, not
        // dropped.
        let err = parse_dataset(
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
        .expect_err("conflicting rdf:reifies must hard-fail at parse");
        assert!(
            err.to_string().contains("conflicting rdf:reifies binding"),
            "{err}"
        );
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
        )
        .expect("emit blobs");
        let base_graph = purrdf_gts::reader::read(&base, true, None);
        let blob_graph = purrdf_gts::reader::read(&with_blobs, true, None);
        assert_eq!(graph_nquads(&base_graph), graph_nquads(&blob_graph));
        let reps: std::collections::BTreeSet<String> = blob_graph
            .blob_meta
            .iter()
            .filter_map(|(_, meta)| match meta {
                ciborium::value::Value::Map(items) => items.iter().find_map(|(key, value)| {
                    if matches!(key, ciborium::value::Value::Text(k) if k == "rep") {
                        if let ciborium::value::Value::Text(rep) = value {
                            return Some(rep.clone());
                        }
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
        )
        .expect_err("partial signing must hard-fail");
        assert!(err.contains("all three or none"), "{err}");
    }
}
