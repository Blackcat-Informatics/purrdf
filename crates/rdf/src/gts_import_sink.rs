// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The **authoritative** GTS ingestion path: a [`StreamingSink`] that preserves
//! per-segment blank-node scope while folding into the immutable IR (C2.a).
//!
//! `purrdf_gts::reader::read()` folds every segment into one append-order term
//! table, which destroys per-segment blank-node scope (the same `_:b1` label in
//! two different segments names two *different* nodes, but the folded table loses
//! that). The only place segment identity survives is the streaming sink
//! callbacks, each of which carries a `segment_index`. This importer therefore
//! drives [`purrdf_gts::reader::read_to_sink`] and interns each segment's blank
//! nodes under a per-segment [`BlankScope`] — making this the correctness-bearing
//! ingestion path for blank-node scope (see `docs/design/819-rdf-ir-dataflow.md`,
//! *Appendix C0.2* and the `bnode-scope-flatten` loss code).
//!
//! # Two-phase resolution (event-order independence)
//!
//! `purrdf_gts::writer::Writer::deterministic` emits frames in the order `terms →
//! quads → reifies → annot`, and the reader dispatches frames in stream order. A
//! quoted-triple `Term` therefore arrives (in the `terms` frame) carrying only its
//! `reifier` id — the `reifies` frame that binds that reifier to the triple's
//! `(s, p, o)` arrives **later**. A single-pass importer that resolves a triple
//! term the instant its `term` event fires cannot succeed: the binding is not yet
//! known (`rdf-ir-missing-reifier-binding`).
//!
//! This importer is therefore **two-phase**:
//!
//! 1. **Streaming phase** (during `read_to_sink`): record RAW per-segment
//!    descriptors — for each gts term id its kind plus, for triple terms, the
//!    reifier id linking it to its components — and RAW reifier bindings and
//!    quad / reifier / annotation rows as gts-id rows. Nothing is resolved or
//!    failed eagerly on a not-yet-known binding.
//! 2. **Resolution phase** (after `read_to_sink` returns, before `freeze`): now
//!    that every `term` and `reifies` event has been seen, resolve each gts term
//!    to a [`TermId`] — non-triple terms intern directly; triple terms resolve
//!    their now-complete `(s, p, o)` recursively, inner-first, depth-bounded by
//!    [`MAX_GTS_TERM_NESTING_DEPTH`] — then push the raw quad / reifier /
//!    annotation rows through the per-segment remap.
//!
//! Per the no-optionality / hard-fail doctrine, a *genuinely* dangling term id or
//! reifier binding — one still unresolved after ALL events are seen — is an `Err`,
//! never a silent skip. Only the merely out-of-order case now resolves.

use std::collections::HashMap;

use ciborium::value::Value;
use purrdf_gts::model::{OpaqueNode, Quad, Signature, StreamableInfo, Suppression, Term, TermKind};
use purrdf_gts::reader::StreamingSink;

use crate::{
    BlankScope, GtsBundle, RdfDatasetBuilder, RdfDiagnostic, RdfEnvelope, RdfLiteral, RdfLocation,
    RdfLookaside, RdfMetadataValue, RdfOpaqueNodeRecord, RdfSegmentRecord, RdfSignatureRecord,
    RdfSuppressionRecord, TermId, TermRef,
};

/// Depth bound for resolving nested quoted-triple terms, mirroring the
/// `MAX_GTS_TERM_NESTING_DEPTH` guard in [`crate::gts`]. A cyclic or absurdly
/// nested triple term hard-fails rather than recursing without bound.
const MAX_GTS_TERM_NESTING_DEPTH: usize = 16;

/// A [`StreamingSink`] that folds GTS events into the immutable IR with per-segment
/// blank-node scope isolation, two-phase so it is independent of `term`/`reifier`/
/// `quad` event order (see the module docs).
struct SinkImporter {
    /// The fallible IR builder we intern terms and push structure into.
    builder: RdfDatasetBuilder,
    /// RAW per-segment terms recorded during the streaming phase, keyed by
    /// `(segment_index, gts_term_id)`. Resolved to [`TermId`]s in phase 2. The
    /// streaming callbacks cannot resolve a quoted-triple term yet (its reifier
    /// binding may arrive later), so each term is stashed verbatim.
    raw_terms: HashMap<(usize, usize), Term>,
    /// Per-segment map from GTS segment-local term id → our [`TermId`], populated
    /// during phase-2 resolution. The key is `(segment_index, gts_term_id)`.
    remaps: HashMap<(usize, usize), TermId>,
    /// Per-segment reifier bindings (`(segment_index, reifier gts-id) → (s, p, o)
    /// gts-ids`), recorded from `reifier` events so a Triple term delivered earlier
    /// (or later) can recover its components THROUGH the segment's remap in phase 2.
    reifier_bindings: HashMap<(usize, usize), purrdf_gts::model::Triple3>,
    /// RAW quad rows `(segment_index, (s, p, o, g) gts-ids)`, resolved in phase 2.
    raw_quads: Vec<(usize, Quad)>,
    /// RAW reifier rows `(segment_index, reifier gts-id, (s, p, o) gts-ids, graph
    /// gts-id?)`, resolved in phase 2 to bind the reifier resource (in its named graph)
    /// to the interned triple.
    raw_reifiers: Vec<(usize, usize, purrdf_gts::model::Triple3, Option<usize>)>,
    /// RAW annotation rows `(segment_index, (r, p, v) gts-ids, graph gts-id?)`, resolved
    /// in phase 2.
    raw_annotations: Vec<(usize, purrdf_gts::model::Triple3, Option<usize>)>,
    /// Out-of-band material accumulated from blob / signature / suppression /
    /// segment-head / opaque events.
    lookaside: RdfLookaside,
    /// First deferred error. `StreamingSink` methods return `()`, so a referential
    /// failure is parked here and surfaced after the reader returns.
    error: Option<RdfDiagnostic>,
}

impl SinkImporter {
    fn new() -> Self {
        Self {
            builder: RdfDatasetBuilder::new(),
            raw_terms: HashMap::new(),
            remaps: HashMap::new(),
            reifier_bindings: HashMap::new(),
            raw_quads: Vec::new(),
            raw_reifiers: Vec::new(),
            raw_annotations: Vec::new(),
            lookaside: RdfLookaside::default(),
            error: None,
        }
    }

    /// Record the first deferred error; later errors do not overwrite it.
    fn fail(&mut self, diagnostic: RdfDiagnostic) {
        if self.error.is_none() {
            self.error = Some(diagnostic);
        }
    }

    /// Resolve a GTS segment-local term id to its interned [`TermId`], resolving it
    /// (and any nested quoted triples) on demand if not yet interned. Fails if the
    /// id was never introduced by a `term` event (a genuinely dangling reference).
    ///
    /// This is the phase-2 primitive: it is memoizing (through `self.remaps`) and
    /// depth-bounded against cyclic quoted triples.
    fn resolve_term(
        &mut self,
        segment_index: usize,
        gts_id: usize,
        role: &str,
        depth: usize,
    ) -> Result<TermId, RdfDiagnostic> {
        if let Some(&id) = self.remaps.get(&(segment_index, gts_id)) {
            return Ok(id);
        }
        if depth > MAX_GTS_TERM_NESTING_DEPTH {
            return Err(RdfDiagnostic::error(
                "rdf-ir-term-nesting-limit",
                "GTS triple-term nesting depth limit exceeded",
            )
            .with_location(
                RdfLocation::logical("gts:sink")
                    .with_gts_segment(segment_index)
                    .with_gts_term(gts_id),
            ));
        }
        // MOVE the raw term out: it resolves at most once (every later reference
        // hits the `remaps` cache above), so taking ownership lets interning
        // mutably borrow `self.builder` without a borrow conflict AND without
        // cloning the term's owned strings. A (pathological) cyclic reference now
        // surfaces as a dangling-term-ref — the term is already removed — rather
        // than the nesting-limit; both are hard failures and a GTS Writer never
        // emits cycles. The streaming path is the scope-correct authority, not the
        // zero-alloc one (that is the `import_graph` contract).
        let Some(term) = self.raw_terms.remove(&(segment_index, gts_id)) else {
            return Err(RdfDiagnostic::error(
                "rdf-ir-dangling-term-ref",
                format!(
                    "GTS {role} references segment-{segment_index} term id {gts_id}, \
                     which no `term` event introduced"
                ),
            )
            .with_location(
                RdfLocation::logical("gts:sink")
                    .with_gts_segment(segment_index)
                    .with_gts_term(gts_id),
            ));
        };
        let our_id = self.intern_raw_term(segment_index, gts_id, term, depth)?;
        self.remaps.insert((segment_index, gts_id), our_id);
        Ok(our_id)
    }

    /// Intern one already-located raw term, recursing (inner-first) for quoted
    /// triples through their reifier binding.
    ///
    /// Takes the raw [`Term`] BY VALUE (it was just `remove()`d from `raw_terms`,
    /// so the sink uniquely owns it) and MOVES its owned strings (`value` / `lang` /
    /// `direction`) into the interner instead of cloning — completing the
    /// no-clone path the streaming design promises.
    fn intern_raw_term(
        &mut self,
        segment_index: usize,
        gts_id: usize,
        term: Term,
        depth: usize,
    ) -> Result<TermId, RdfDiagnostic> {
        let our_id = match term.kind {
            TermKind::Iri => {
                let Some(iri) = term.value.filter(|value| !value.is_empty()) else {
                    return Err(RdfDiagnostic::error(
                        "rdf-ir-iri-missing-value",
                        "GTS IRI term requires a non-empty value",
                    )
                    .with_location(
                        RdfLocation::logical("gts:sink")
                            .with_gts_segment(segment_index)
                            .with_gts_term(gts_id),
                    ));
                };
                self.builder.intern_iri(&iri)
            }
            // Per-segment scope isolation (C0.2): scope = segment_index + 1 so the
            // SAME blank label in different segments interns to DISTINCT ids, while
            // scope 0 stays reserved for the default/global scope.
            TermKind::Bnode => {
                let label = term
                    .value
                    .unwrap_or_else(|| format!("gts_bnode_{segment_index}_{gts_id}"));
                let scope = BlankScope(segment_index as u32 + 1);
                self.builder.intern_blank(&label, scope)
            }
            TermKind::Literal => {
                let literal = self.literal_from_term(segment_index, term, depth)?;
                self.builder.intern_literal(literal)
            }
            TermKind::Triple => {
                let Some(reifier_gts_id) = term.reifier else {
                    return Err(RdfDiagnostic::error(
                        "rdf-ir-unbound-triple-term",
                        "GTS triple term has no reifier binding",
                    )
                    .with_location(
                        RdfLocation::logical("gts:sink")
                            .with_gts_segment(segment_index)
                            .with_gts_term(gts_id),
                    ));
                };
                let (s, p, o) =
                    self.resolve_triple_components(segment_index, reifier_gts_id, depth + 1)?;
                self.builder.intern_triple(s, p, o)
            }
        };
        Ok(our_id)
    }

    /// Build an [`RdfLiteral`] from a GTS literal term, resolving its datatype id
    /// THROUGH the segment's terms (phase 2).
    ///
    /// GTS *does* carry RDF 1.2 base direction (`purrdf-gts`'s `Term` has a
    /// `direction: Option<String>` slot), so it is read here via
    /// [`parse_gts_direction`](crate::gts_resolve::parse_gts_direction) and preserved
    /// onto the literal — direction is NOT a projection loss on this path.
    fn literal_from_term(
        &mut self,
        segment_index: usize,
        term: Term,
        depth: usize,
    ) -> Result<RdfLiteral, RdfDiagnostic> {
        let datatype = match term.datatype {
            Some(dt_gts_id) => {
                let dt_id =
                    self.resolve_term(segment_index, dt_gts_id, "literal datatype", depth + 1)?;
                match self.builder.resolve(dt_id) {
                    TermRef::Iri(iri) => Some(iri.to_string()),
                    other => {
                        return Err(RdfDiagnostic::error(
                            "rdf-ir-literal-datatype-not-iri",
                            format!("GTS literal datatype must resolve to an IRI, got {other:?}"),
                        )
                        .with_location(
                            RdfLocation::logical("gts:sink")
                                .with_gts_segment(segment_index)
                                .with_gts_term(dt_gts_id),
                        ));
                    }
                }
            }
            None => None,
        };
        // Parse direction (which requires the language tag) BEFORE moving the owned
        // `value`/`lang`/`direction` strings out of the term.
        let direction = crate::gts_resolve::parse_gts_direction(
            term.direction.as_deref(),
            term.lang.as_deref(),
        )?;
        Ok(RdfLiteral {
            lexical_form: term.value.unwrap_or_default(),
            datatype,
            language: term.lang,
            direction,
        })
    }

    /// Resolve the `(s, p, o)` of the triple a reifier binds, THROUGH this
    /// segment's terms, with a depth bound against cyclic quoted triples. The
    /// reifier binding MUST exist by phase 2 (all `reifies` events are in); a
    /// missing binding here is a genuine dangling reference, hence an `Err`.
    fn resolve_triple_components(
        &mut self,
        segment_index: usize,
        reifier: usize,
        depth: usize,
    ) -> Result<(TermId, TermId, TermId), RdfDiagnostic> {
        if depth > MAX_GTS_TERM_NESTING_DEPTH {
            return Err(RdfDiagnostic::error(
                "rdf-ir-term-nesting-limit",
                "GTS triple-term nesting depth limit exceeded",
            )
            .with_location(
                RdfLocation::logical("gts:sink")
                    .with_gts_segment(segment_index)
                    .with_gts_reifier(reifier),
            ));
        }
        let Some(&(s, p, o)) = self.reifier_bindings.get(&(segment_index, reifier)) else {
            return Err(RdfDiagnostic::error(
                "rdf-ir-missing-reifier-binding",
                format!(
                    "GTS triple term references reifier {reifier} in segment \
                     {segment_index} with no recorded binding"
                ),
            )
            .with_location(
                RdfLocation::logical("gts:sink")
                    .with_gts_segment(segment_index)
                    .with_gts_reifier(reifier),
            ));
        };
        let s = self.resolve_term(segment_index, s, "reified subject", depth)?;
        let p = self.resolve_term(segment_index, p, "reified predicate", depth)?;
        let o = self.resolve_term(segment_index, o, "reified object", depth)?;
        Ok((s, p, o))
    }

    /// Phase 2: resolve every recorded term and push every recorded row, after the
    /// reader has delivered ALL `term` / `reifies` / `quad` / `annot` events.
    fn finish(&mut self) -> Result<(), RdfDiagnostic> {
        // Resolve every introduced term (idempotent through `remaps`). Iterate in a
        // deterministic order so the interner's allocation order — and thus the
        // frozen term order — is reproducible for a fixed event stream.
        let mut keys: Vec<(usize, usize)> = self.raw_terms.keys().copied().collect();
        keys.sort_unstable();
        for (segment_index, gts_id) in keys {
            self.resolve_term(segment_index, gts_id, "term", 0)?;
        }

        // Quads.
        let raw_quads = std::mem::take(&mut self.raw_quads);
        for (segment_index, (s, p, o, g)) in raw_quads {
            let s = self.resolve_term(segment_index, s, "quad subject", 0)?;
            let p = self.resolve_term(segment_index, p, "quad predicate", 0)?;
            let o = self.resolve_term(segment_index, o, "quad object", 0)?;
            let g = match g {
                Some(g) => Some(self.resolve_term(segment_index, g, "quad graph name", 0)?),
                None => None,
            };
            self.builder.push_quad(s, p, o, g);
        }

        // Reifier bindings: bind the reifier resource to the interned triple term.
        let raw_reifiers = std::mem::take(&mut self.raw_reifiers);
        for (segment_index, reifier, (s, p, o), graph) in raw_reifiers {
            let reifier_id = self.resolve_term(segment_index, reifier, "reifier", 0)?;
            let s = self.resolve_term(segment_index, s, "reified subject", 0)?;
            let p = self.resolve_term(segment_index, p, "reified predicate", 0)?;
            let o = self.resolve_term(segment_index, o, "reified object", 0)?;
            let g = match graph {
                Some(g) => Some(self.resolve_term(segment_index, g, "reifier graph name", 0)?),
                None => None,
            };
            let triple_term = self.builder.intern_triple(s, p, o);
            self.builder
                .push_reifier_in_graph(reifier_id, triple_term, g);
        }

        // Annotations `(reifier, predicate, object, graph?)`.
        let raw_annotations = std::mem::take(&mut self.raw_annotations);
        for (segment_index, (r, p, v), graph) in raw_annotations {
            let r = self.resolve_term(segment_index, r, "annotation reifier", 0)?;
            let p = self.resolve_term(segment_index, p, "annotation predicate", 0)?;
            let v = self.resolve_term(segment_index, v, "annotation object", 0)?;
            let g = match graph {
                Some(g) => Some(self.resolve_term(segment_index, g, "annotation graph name", 0)?),
                None => None,
            };
            self.builder.push_annotation_in_graph(r, p, v, g);
        }

        Ok(())
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Convert a CBOR [`Value`] into the crate's [`RdfMetadataValue`].
fn metadata_value_from_cbor(value: &Value) -> RdfMetadataValue {
    match value {
        Value::Integer(integer) => RdfMetadataValue::Integer(i128::from(*integer)),
        Value::Bytes(bytes) => RdfMetadataValue::Bytes(bytes.clone()),
        Value::Float(value) => RdfMetadataValue::Float(*value),
        Value::Text(value) => RdfMetadataValue::Text(value.clone()),
        Value::Bool(value) => RdfMetadataValue::Bool(*value),
        Value::Null => RdfMetadataValue::Null,
        Value::Tag(tag, value) => RdfMetadataValue::Tagged {
            tag: *tag,
            value: Box::new(metadata_value_from_cbor(value)),
        },
        Value::Array(values) => {
            RdfMetadataValue::Array(values.iter().map(metadata_value_from_cbor).collect())
        }
        Value::Map(entries) => RdfMetadataValue::Map(
            entries
                .iter()
                .map(|(key, value)| {
                    let key = match key {
                        Value::Text(text) => text.clone(),
                        other => format!("{other:?}"),
                    };
                    (key, metadata_value_from_cbor(value))
                })
                .collect(),
        ),
        other => RdfMetadataValue::Opaque(format!("{other:?}")),
    }
}

impl StreamingSink for SinkImporter {
    fn term(&mut self, segment_index: usize, gts_term_id: usize, term: &Term) {
        if self.error.is_some() {
            return;
        }
        // Streaming phase: stash the raw term verbatim. A quoted-triple term cannot
        // be resolved yet — its reifier binding may arrive in a later frame — so we
        // defer ALL resolution to phase 2, where every event has been seen.
        self.raw_terms
            .insert((segment_index, gts_term_id), term.clone());
    }

    fn quad(&mut self, segment_index: usize, quad: Quad) {
        if self.error.is_some() {
            return;
        }
        self.raw_quads.push((segment_index, quad));
    }

    fn reifier(&mut self, segment_index: usize, reifier: purrdf_gts::model::ReifierRow) {
        if self.error.is_some() {
            return;
        }
        // purrdf-gts row-array: `(reifier_id, (s, p, o), graph?)`. The graph slot
        // records the named graph the reifier was declared in (`None` = default graph)
        // and is threaded into the IR's graph-scoped statement layer in phase 2.
        let (reifier_id, triple, graph) = reifier;
        // Record the reifier → (s, p, o) binding for this segment so a Triple term
        // (delivered in any order) can resolve its components in phase 2, and stash
        // the row (with its named graph, if any) so the reifier resource is bound to
        // the interned triple in phase 2.
        self.reifier_bindings
            .insert((segment_index, reifier_id), triple);
        self.raw_reifiers
            .push((segment_index, reifier_id, triple, graph));
    }

    fn annotation(&mut self, segment_index: usize, annotation: purrdf_gts::model::AnnotationRow) {
        if self.error.is_some() {
            return;
        }
        // Row-array: `(reifier, predicate, value, graph?)`. The graph slot records the
        // named graph the annotation was asserted in and is threaded into the IR's
        // graph-scoped statement layer in phase 2.
        let (reifier, predicate, value, graph) = annotation;
        self.raw_annotations
            .push((segment_index, (reifier, predicate, value), graph));
    }

    fn suppression(&mut self, _segment_index: usize, suppression: &Suppression) {
        self.lookaside.suppressions.push(RdfSuppressionRecord {
            reason: suppression.reason.clone(),
            // `by` is a segment-local term id; we record it as a display hint only,
            // never as a cross-dataset id (C0.8).
            by: suppression.by.map(|term_id| format!("term#{term_id}")),
            targets: suppression
                .targets
                .iter()
                .map(metadata_value_from_cbor)
                .collect(),
        });
    }

    fn blob(&mut self, _segment_index: usize, digest: &str, meta: Option<&Value>) {
        let metadata = match meta.map(metadata_value_from_cbor) {
            Some(RdfMetadataValue::Map(map)) => map,
            Some(value) => {
                let mut map = std::collections::BTreeMap::new();
                map.insert("value".to_owned(), value);
                map
            }
            None => std::collections::BTreeMap::new(),
        };
        let media_type = metadata
            .get("mt")
            .and_then(RdfMetadataValue::as_text)
            .map(str::to_owned);
        let representation = metadata
            .get("rep")
            .and_then(RdfMetadataValue::as_text)
            .map(str::to_owned);
        self.lookaside.blobs.push(crate::RdfBlobRecord {
            digest: digest.to_owned(),
            media_type,
            representation,
            decoded_len: None,
            metadata,
            // The streaming sink delivers a blob's digest + metadata, not its
            // payload. The content digest above is the blob_id reference; richer
            // origin (segment-head) enrichment for the streaming path is a
            // follow-up — the folded read path (`gts::blob_records`) populates it.
            origin: None,
        });
    }

    fn opaque(&mut self, _segment_index: usize, opaque: &OpaqueNode) {
        self.lookaside.opaque_nodes.push(RdfOpaqueNodeRecord {
            id: hex_bytes(&opaque.id),
            frame_type: opaque.frame_type.clone(),
            reason: opaque.reason.clone(),
            signature_status: opaque.sigstat.clone(),
            public_metadata: opaque.pub_meta.as_ref().map(metadata_value_from_cbor),
        });
    }

    fn signature(&mut self, _segment_index: usize, signature: &Signature) {
        self.lookaside.signatures.push(RdfSignatureRecord {
            frame_id: hex_bytes(&signature.frame_id),
            key_id: signature.kid.clone(),
            status: signature.status.clone(),
            has_cose: signature.cose.is_some(),
        });
    }

    fn segment_head(&mut self, segment_index: usize, head: &[u8]) {
        // Grow/patch the per-segment record with its head id.
        self.ensure_segment_record(segment_index).head = Some(hex_bytes(head));
    }

    fn streamable_layout(&mut self, segment_index: usize, info: &StreamableInfo) {
        let record = self.ensure_segment_record(segment_index);
        record.claimed_streamable = info.claimed;
        record.covered = info.covered;
        record.tail = info.tail;
    }

    fn diagnostic(&mut self, diagnostic: &purrdf_gts::model::Diagnostic) {
        // A reader diagnostic is a hard fold failure on the IR path (the IR is the
        // authority, no degraded fold). Record the first as the deferred error.
        self.fail(
            RdfDiagnostic::error(
                "rdf-ir-gts-fold-diagnostic",
                format!(
                    "GTS fold diagnostic {}: {}",
                    diagnostic.code, diagnostic.detail
                ),
            )
            .with_location({
                let location = RdfLocation::logical("gts:sink");
                match diagnostic.frame_index {
                    Some(frame_index) => location.with_gts_frame(frame_index),
                    None => location,
                }
            }),
        );
    }
}

impl SinkImporter {
    /// Ensure a [`RdfSegmentRecord`] exists for `segment_index`, returning it.
    fn ensure_segment_record(&mut self, segment_index: usize) -> &mut RdfSegmentRecord {
        if let Some(position) = self
            .lookaside
            .segments
            .iter()
            .position(|record| record.index == segment_index)
        {
            return &mut self.lookaside.segments[position];
        }
        self.lookaside.segments.push(RdfSegmentRecord {
            index: segment_index,
            head: None,
            profile: None,
            claimed_streamable: false,
            covered: 0,
            tail: 0,
        });
        self.lookaside
            .segments
            .last_mut()
            .expect("segment record just pushed")
    }
}

/// The authoritative GTS ingestion path: folds GTS bytes into a [`GtsBundle`],
/// preserving per-segment blank-node scope (C2.a).
///
/// Drives [`purrdf_gts::reader::read_to_sink`] with `allow_segments = true` so a
/// multi-segment file is delivered as per-segment events (the only place segment
/// identity survives). Resolution is **two-phase** (see the module docs) so a
/// quoted-triple term is resolved regardless of whether its `reifies` binding
/// arrived before or after the term itself. Any reader diagnostic or genuinely
/// dangling term reference is a HARD failure (`Err`); on success the interned terms
/// are frozen via [`RdfDatasetBuilder::freeze`] and paired with the envelope.
pub fn import_gts_events(bytes: &[u8]) -> Result<GtsBundle, RdfDiagnostic> {
    let mut importer = SinkImporter::new();
    let _ = purrdf_gts::reader::read_to_sink(bytes, true, None, &mut importer);

    if let Some(error) = importer.error {
        return Err(error);
    }

    // Phase 2: now that ALL term / reifier / quad / annotation events are seen,
    // resolve every term and push every row.
    importer.finish()?;

    let lookaside = importer.lookaside;
    let dataset = importer.builder.freeze()?;
    Ok(GtsBundle::new(dataset, RdfEnvelope::new(lookaside)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_gts::model::{Graph, Term, Term as GtsTerm, TermKind as GtsKind};
    use purrdf_gts::writer::Writer;

    fn iri_term(value: &str) -> Term {
        Term {
            kind: TermKind::Iri,
            value: Some(value.to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        }
    }

    fn blank_term(label: &str) -> Term {
        Term {
            kind: TermKind::Bnode,
            value: Some(label.to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        }
    }

    /// Drive the streaming events then run phase 2, returning the importer for
    /// post-resolution assertions (the public `import_gts_events` does both phases
    /// from bytes; this exercises the hand-ordered direct-sink path).
    fn finish_direct(mut importer: SinkImporter) -> SinkImporter {
        if importer.error.is_none() {
            if let Err(diagnostic) = importer.finish() {
                importer.fail(diagnostic);
            }
        }
        importer
    }

    /// Resolve `our_id` back to its interned blank `(label, scope)` for assertions.
    fn blank_scope(importer: &SinkImporter, id: TermId) -> (String, BlankScope) {
        match importer.builder.resolve(id) {
            TermRef::Blank { label, scope } => (label.to_string(), scope),
            other => panic!("expected blank node, got {other:?}"),
        }
    }

    /// GATE 2 (a) — multi-segment blank-node scope isolation, driven DIRECTLY
    /// through the `StreamingSink` callbacks (no GTS bytes needed).
    ///
    /// Segment 0 and segment 1 both carry a blank labelled `b1` (DIFFERENT nodes)
    /// and an IRI `ex:s` (the SAME node). After freeze the two `b1` blanks MUST be
    /// distinct ids (per-segment scope) while `ex:s` MUST be one shared id.
    #[test]
    fn gate2_multi_segment_blank_scope_isolation_direct() {
        let mut importer = SinkImporter::new();

        // Segment 0: term 0 = ex:s, term 1 = ex:p, term 2 = _:b1, quad (s p b1).
        importer.term(0, 0, &iri_term("http://example.org/s"));
        importer.term(0, 1, &iri_term("http://example.org/p"));
        importer.term(0, 2, &blank_term("b1"));
        importer.quad(0, (0, 1, 2, None));

        // Segment 1: term 0 = ex:s (same IRI value), term 1 = ex:p2, term 2 = _:b1
        // (SAME label, DIFFERENT node), quad (s p2 b1).
        importer.term(1, 0, &iri_term("http://example.org/s"));
        importer.term(1, 1, &iri_term("http://example.org/p2"));
        importer.term(1, 2, &blank_term("b1"));
        importer.quad(1, (0, 1, 2, None));

        let mut importer = finish_direct(importer);
        assert!(
            importer.error.is_none(),
            "no error expected: {:?}",
            importer.error
        );

        let seg0_s = importer.remaps[&(0, 0)];
        let seg1_s = importer.remaps[&(1, 0)];
        let seg0_b1 = importer.remaps[&(0, 2)];
        let seg1_b1 = importer.remaps[&(1, 2)];

        // The shared IRI interns to ONE id across both segments (value identity).
        assert_eq!(seg0_s, seg1_s, "ex:s is the same node across segments");

        // The same blank label in different segments interns to DISTINCT ids.
        assert_ne!(
            seg0_b1, seg1_b1,
            "_:b1 in segment 0 and segment 1 are DIFFERENT nodes (scope isolation)"
        );
        let (label0, scope0) = blank_scope(&importer, seg0_b1);
        let (label1, scope1) = blank_scope(&importer, seg1_b1);
        assert_eq!(label0, "b1");
        assert_eq!(label1, "b1");
        assert_eq!(scope0, BlankScope(1), "segment 0 → scope 1");
        assert_eq!(scope1, BlankScope(2), "segment 1 → scope 2");

        let dataset = std::mem::take(&mut importer.builder)
            .freeze()
            .expect("freeze");
        // 2 quads, distinct blanks: ex:s, ex:p, b1@s0, ex:p2, b1@s1 = 5 terms.
        assert_eq!(dataset.quad_count(), 2);
        assert_eq!(dataset.term_count(), 5);
    }

    /// A quad that references a GTS term id no `term` event introduced MUST surface
    /// as a hard `Err` (no silent skip), even though resolution is now deferred to
    /// phase 2.
    #[test]
    fn gate2_unknown_term_reference_is_err_direct() {
        let mut importer = SinkImporter::new();
        importer.term(0, 0, &iri_term("http://example.org/s"));
        importer.term(0, 1, &iri_term("http://example.org/p"));
        // Object id 9 was never introduced.
        importer.quad(0, (0, 1, 9, None));
        let importer = finish_direct(importer);
        assert!(
            importer.error.is_some(),
            "dangling reference must defer an error"
        );
        let err = importer.error.unwrap();
        assert_eq!(err.code, "rdf-ir-dangling-term-ref");
    }

    /// Directional literals: GTS `Term` carries no base direction, so the sink path
    /// yields `direction == None`, but lexical form, datatype, and language survive.
    #[test]
    fn directional_literal_lexical_lang_survive_sink_path() {
        let mut importer = SinkImporter::new();
        importer.term(0, 0, &iri_term("http://example.org/s"));
        importer.term(0, 1, &iri_term("http://example.org/p"));
        importer.term(
            0,
            2,
            &Term {
                kind: TermKind::Literal,
                value: Some("Bonjour".to_owned()),
                datatype: None,
                lang: Some("FR".to_owned()),
                direction: None,
                reifier: None,
            },
        );
        importer.quad(0, (0, 1, 2, None));
        let mut importer = finish_direct(importer);
        assert!(importer.error.is_none(), "{:?}", importer.error);

        let lit_id = importer.remaps[&(0, 2)];
        let dataset = std::mem::take(&mut importer.builder)
            .freeze()
            .expect("freeze");
        match dataset.resolve(lit_id) {
            TermRef::Literal {
                lexical,
                language,
                direction,
                ..
            } => {
                assert_eq!(lexical, "Bonjour", "lexical preserved verbatim");
                assert_eq!(language, Some("fr"), "language lowercased per C0.1");
                assert_eq!(direction, None, "GTS cannot carry base direction");
            }
            other => panic!("expected literal, got {other:?}"),
        }
    }

    /// A nested quoted-triple term survives the sink path: the inner triple is an
    /// object position of the outer triple. (Hand-ordered direct-sink events.)
    #[test]
    fn nested_triple_term_survives_sink_path() {
        let mut importer = SinkImporter::new();
        // Inner triple (ex:a ex:p ex:b) reified by reifier r0; outer triple
        // (ex:a ex:asserts <<inner>>) reified by reifier r1.
        importer.term(0, 0, &iri_term("http://example.org/a"));
        importer.term(0, 1, &iri_term("http://example.org/p"));
        importer.term(0, 2, &iri_term("http://example.org/b"));
        importer.term(0, 3, &iri_term("http://example.org/r0"));
        importer.reifier(0, (3, (0, 1, 2), None));
        // Inner triple TERM bound to reifier r0 (gts id 3).
        importer.term(
            0,
            4,
            &Term {
                kind: TermKind::Triple,
                value: None,
                datatype: None,
                lang: None,
                direction: None,
                reifier: Some(3),
            },
        );
        importer.term(0, 5, &iri_term("http://example.org/asserts"));
        importer.term(0, 6, &iri_term("http://example.org/r1"));
        importer.reifier(0, (6, (0, 5, 4), None));
        importer.term(
            0,
            7,
            &Term {
                kind: TermKind::Triple,
                value: None,
                datatype: None,
                lang: None,
                direction: None,
                reifier: Some(6),
            },
        );
        importer.quad(0, (0, 5, 7, None));
        let mut importer = finish_direct(importer);
        assert!(importer.error.is_none(), "{:?}", importer.error);

        let inner = importer.remaps[&(0, 4)];
        let outer = importer.remaps[&(0, 7)];
        let dataset = std::mem::take(&mut importer.builder)
            .freeze()
            .expect("freeze");
        match dataset.resolve(outer) {
            TermRef::Triple { o, .. } => {
                assert_eq!(o, inner, "outer triple's object IS the inner triple term");
            }
            other => panic!("expected triple term, got {other:?}"),
        }
    }

    /// Multiple distinct reifiers binding ONE triple all survive.
    #[test]
    fn multiple_reifiers_for_one_triple_survive_sink_path() {
        let mut importer = SinkImporter::new();
        importer.term(0, 0, &iri_term("http://example.org/s"));
        importer.term(0, 1, &iri_term("http://example.org/p"));
        importer.term(0, 2, &iri_term("http://example.org/o"));
        importer.term(0, 3, &iri_term("http://example.org/r1"));
        importer.term(0, 4, &iri_term("http://example.org/r2"));
        importer.reifier(0, (3, (0, 1, 2), None));
        importer.reifier(0, (4, (0, 1, 2), None));
        let mut importer = finish_direct(importer);
        assert!(importer.error.is_none(), "{:?}", importer.error);

        let dataset = std::mem::take(&mut importer.builder)
            .freeze()
            .expect("freeze");
        let reifiers: Vec<_> = dataset.reifiers().collect();
        assert_eq!(reifiers.len(), 2, "two distinct reifiers survive");
        // Both bind the same interned triple term.
        assert_eq!(reifiers[0].1, reifiers[1].1, "same triple term bound twice");
    }

    /// REAL Writer-serialized quoted triple `<<ex:s ex:p ex:o>>`, used as the
    /// OBJECT of an outer quad AND as a reifier target, round-trips through bytes.
    ///
    /// This is the regression the two-phase importer fixes: `Writer::deterministic`
    /// emits the `terms` frame (carrying the triple term) BEFORE the `reifies`
    /// frame that binds its components, so the single-pass importer failed with
    /// `rdf-ir-missing-reifier-binding`. The two-phase importer resolves it.
    #[test]
    fn writer_quoted_triple_roundtrips_through_bytes() {
        let mut graph = Graph::default();
        graph.terms.push(iri("http://example.org/s")); // 0
        graph.terms.push(iri("http://example.org/p")); // 1
        graph.terms.push(iri("http://example.org/o")); // 2
        graph.terms.push(iri("http://example.org/stmt")); // 3 reifier resource
        graph.reifiers.push((3, (0, 1, 2), None));
        graph.terms.push(GtsTerm {
            kind: GtsKind::Triple,
            value: None,
            datatype: None,
            lang: None,
            direction: None,
            reifier: Some(3),
        }); // 4 quoted triple <<ex:s ex:p ex:o>>
        graph.terms.push(iri("http://example.org/asserts")); // 5
                                                             // Outer quad: (ex:s ex:asserts <<ex:s ex:p ex:o>>) — the quoted triple
                                                             // sits in OBJECT position.
        graph.quads.push((0, 5, 4, None));

        let bytes = Writer::deterministic(&graph, "purrdf-test")
            .expect("deterministic writer")
            .to_bytes();

        let bundle = import_gts_events(&bytes).expect("quoted-triple round-trip");
        let dataset = &bundle.dataset;

        // The outer quad's object must be the quoted triple, with the right (s,p,o).
        let quad = dataset
            .quad_refs()
            .find(|q| matches!(q.o, TermRef::Triple { .. }))
            .expect("a quad whose object is the quoted triple");
        let TermRef::Triple { s, p, o } = quad.o else {
            unreachable!("filtered for Triple above");
        };
        let iri_of = |id: TermId| match dataset.resolve(id) {
            TermRef::Iri(iri) => iri.to_owned(),
            other => panic!("expected IRI, got {other:?}"),
        };
        assert_eq!(iri_of(s), "http://example.org/s");
        assert_eq!(iri_of(p), "http://example.org/p");
        assert_eq!(iri_of(o), "http://example.org/o");

        // The reifier resource ex:stmt binds the SAME interned triple term.
        let reifiers: Vec<_> = dataset.reifiers().collect();
        assert_eq!(reifiers.len(), 1, "one reifier binding survives the bytes");
        let TermRef::Triple {
            s: rs,
            p: rp,
            o: ro,
        } = dataset.resolve(reifiers[0].1)
        else {
            panic!("reifier must bind a triple term");
        };
        assert_eq!(
            (rs, rp, ro),
            (s, p, o),
            "reifier binds the SAME quoted triple"
        );
    }

    /// REAL Writer-serialized NESTED quoted triple `<< <<ex:a ex:b ex:c>> ex:p ex:o >>`.
    /// The 0.9.2 `Writer::deterministic` DOES support nested triple terms (it emits
    /// all terms in one `terms` frame and all reifier bindings in one `reifies`
    /// frame, regardless of nesting), so this exercises inner-first resolution from
    /// real bytes.
    #[test]
    fn writer_nested_quoted_triple_roundtrips_through_bytes() {
        let mut graph = Graph::default();
        graph.terms.push(iri("http://example.org/a")); // 0
        graph.terms.push(iri("http://example.org/b")); // 1
        graph.terms.push(iri("http://example.org/c")); // 2
        graph.terms.push(iri("http://example.org/r0")); // 3 inner reifier resource
        graph.reifiers.push((3, (0, 1, 2), None));
        graph.terms.push(GtsTerm {
            kind: GtsKind::Triple,
            value: None,
            datatype: None,
            lang: None,
            direction: None,
            reifier: Some(3),
        }); // 4 inner <<ex:a ex:b ex:c>>
        graph.terms.push(iri("http://example.org/p")); // 5
        graph.terms.push(iri("http://example.org/o")); // 6
        graph.terms.push(iri("http://example.org/r1")); // 7 outer reifier resource
                                                        // Outer triple: << <<ex:a ex:b ex:c>> ex:p ex:o >> — inner triple (id 4) is
                                                        // the SUBJECT of the outer triple.
        graph.reifiers.push((7, (4, 5, 6), None));
        graph.terms.push(GtsTerm {
            kind: GtsKind::Triple,
            value: None,
            datatype: None,
            lang: None,
            direction: None,
            reifier: Some(7),
        }); // 8 outer << <<...>> ex:p ex:o >>
        graph.terms.push(iri("http://example.org/says")); // 9
        graph.quads.push((0, 9, 8, None));

        let bytes = Writer::deterministic(&graph, "purrdf-test")
            .expect("deterministic writer")
            .to_bytes();

        let bundle = import_gts_events(&bytes).expect("nested quoted-triple round-trip");
        let dataset = &bundle.dataset;

        let quad = dataset
            .quad_refs()
            .find(|q| matches!(q.o, TermRef::Triple { .. }))
            .expect("a quad whose object is the outer quoted triple");
        let TermRef::Triple { s: outer_s, .. } = quad.o else {
            unreachable!("filtered for Triple above");
        };
        // The outer triple's SUBJECT is itself a quoted triple <<ex:a ex:b ex:c>>.
        let iri_of = |id: TermId| match dataset.resolve(id) {
            TermRef::Iri(iri) => iri.to_owned(),
            other => panic!("expected IRI, got {other:?}"),
        };
        match dataset.resolve(outer_s) {
            TermRef::Triple { s, p, o } => {
                assert_eq!(iri_of(s), "http://example.org/a");
                assert_eq!(iri_of(p), "http://example.org/b");
                assert_eq!(iri_of(o), "http://example.org/c");
            }
            other => panic!("outer triple subject must be the inner triple, got {other:?}"),
        }
    }

    /// A genuinely DANGLING reifier binding — a triple term whose reifier no
    /// `reifies` event ever bound — is STILL a hard `Err` after ALL events are
    /// seen (no-optionality). This distinguishes a truly missing binding from a
    /// merely out-of-order one (which now resolves).
    #[test]
    fn dangling_reifier_binding_is_err_after_all_events() {
        let mut importer = SinkImporter::new();
        importer.term(0, 0, &iri_term("http://example.org/s"));
        importer.term(0, 1, &iri_term("http://example.org/asserts"));
        // Triple term bound to reifier id 99, which NO `reifies` event ever supplies.
        importer.term(
            0,
            2,
            &Term {
                kind: TermKind::Triple,
                value: None,
                datatype: None,
                lang: None,
                direction: None,
                reifier: Some(99),
            },
        );
        importer.quad(0, (0, 1, 2, None));
        let importer = finish_direct(importer);
        assert!(
            importer.error.is_some(),
            "a genuinely dangling reifier binding must STILL fail after phase 2"
        );
        assert_eq!(
            importer.error.unwrap().code,
            "rdf-ir-missing-reifier-binding"
        );
    }

    /// GATE 2 (b) — REAL multi-segment GTS bytes. Two `Writer::deterministic`
    /// segments are concatenated (the reader splits at header-shaped items), each
    /// reusing the blank label `b1` for a DIFFERENT node and sharing `ex:s`.
    /// `import_gts_events` MUST preserve scope: distinct `b1`, shared `ex:s`.
    #[test]
    fn gate2_multi_segment_blank_scope_isolation_roundtrip() {
        fn segment(predicate: &str) -> Graph {
            let mut graph = Graph::default();
            graph.terms.push(GtsTerm {
                kind: GtsKind::Iri,
                value: Some("http://example.org/s".to_owned()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            });
            graph.terms.push(GtsTerm {
                kind: GtsKind::Iri,
                value: Some(predicate.to_owned()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            });
            graph.terms.push(GtsTerm {
                kind: GtsKind::Bnode,
                value: Some("b1".to_owned()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            });
            graph.quads.push((0, 1, 2, None));
            graph
        }

        let seg0 = Writer::deterministic(&segment("http://example.org/p"), "purrdf-test")
            .expect("segment 0 writer");
        let seg1 = Writer::deterministic(&segment("http://example.org/p2"), "purrdf-test")
            .expect("segment 1 writer");
        let mut bytes = seg0.to_bytes();
        bytes.extend_from_slice(&seg1.to_bytes());

        let bundle = import_gts_events(&bytes).expect("two-segment import");
        let dataset = &bundle.dataset;

        // Collect the blank-node (label, scope) pairs and the IRI subjects.
        let mut blank_scopes: Vec<(String, BlankScope)> = Vec::new();
        let mut subjects: Vec<&str> = Vec::new();
        for quad in dataset.quad_refs() {
            if let TermRef::Iri(iri) = quad.s {
                subjects.push(iri);
            }
            if let TermRef::Blank { label, scope } = quad.o {
                blank_scopes.push((label.to_owned(), scope));
            }
        }

        assert_eq!(dataset.quad_count(), 2, "two quads, one per segment");
        // ex:s appears as subject in BOTH quads but is one interned term.
        assert_eq!(subjects.len(), 2);
        assert!(subjects.iter().all(|s| *s == "http://example.org/s"));
        // Both quad objects are blank `b1`, but in DISTINCT scopes.
        assert_eq!(blank_scopes.len(), 2);
        assert!(blank_scopes.iter().all(|(label, _)| label == "b1"));
        let scope_a = blank_scopes[0].1;
        let scope_b = blank_scopes[1].1;
        assert_ne!(
            scope_a, scope_b,
            "the two _:b1 blanks are in distinct scopes"
        );

        // term_count: ex:s, ex:p, b1@seg0, ex:p2, b1@seg1 = 5 distinct terms.
        assert_eq!(dataset.term_count(), 5);
    }

    /// `import_gts_events` surfaces a malformed-bytes fold diagnostic as `Err`.
    #[test]
    fn import_rejects_malformed_bytes() {
        let err = import_gts_events(b"not a valid gts file").expect_err("must fail");
        assert_eq!(err.code, "rdf-ir-gts-fold-diagnostic");
    }

    /// Small helper: an IRI `purrdf_gts` `Term`.
    fn iri(value: &str) -> GtsTerm {
        GtsTerm {
            kind: GtsKind::Iri,
            value: Some(value.to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        }
    }
}
