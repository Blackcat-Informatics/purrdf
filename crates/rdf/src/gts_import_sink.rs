// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The **authoritative** GTS ingestion path: an [`RdfDatasetBuilder`]-backed
//! [`ResolvedSink`] that preserves per-segment blank-node scope while folding
//! into the immutable IR (C2.a).
//!
//! `purrdf_gts::reader::read()` folds every segment into one append-order term
//! table, which destroys per-segment blank-node scope (the same `_:b1` label in
//! two different segments names two *different* nodes, but the folded table loses
//! that). The only place segment identity survives is the streaming sink
//! callbacks, each of which carries a `segment_index`. This importer therefore
//! drives [`purrdf_gts::reader::read_to_sink`] through a
//! [`SegmentResolver`](purrdf_gts::segment_decode::SegmentResolver) and interns
//! each segment's blank nodes under a per-segment [`BlankScope`] — making this the
//! correctness-bearing ingestion path for blank-node scope (see
//! `docs/design/819-rdf-ir-dataflow.md`, *Appendix C0.2* and the
//! `bnode-scope-flatten` loss code).
//!
//! # Subsumed decode core
//!
//! GTS-frame decode and per-segment two-phase resolution — the event-order
//! independence that lets a quoted-triple `Term` resolve regardless of whether
//! its `reifies` binding arrived before or after it — live EXACTLY ONCE in
//! [`purrdf_gts::segment_decode`]. [`SinkImporter`] is the immutable-IR emit
//! target: it implements [`ResolvedSink`], interning resolved terms into an
//! [`RdfDatasetBuilder`], pushing resolved quad / reifier / annotation rows, and
//! mapping the four structured decode failures onto rich [`RdfDiagnostic`]
//! locations. The [`SegmentResolver`](purrdf_gts::segment_decode::SegmentResolver)
//! owns the buffering, sorted-key resolution order, and per-segment flush.
//!
//! Per the no-optionality / hard-fail doctrine, a *genuinely* dangling term id or
//! reifier binding — one still unresolved after ALL of a segment's events are
//! seen — is an `Err`, never a silent skip. Only the merely out-of-order case
//! resolves.

use ciborium::value::Value;
use purrdf_gts::model::{Diagnostic, OpaqueNode, Signature, StreamableInfo, Suppression};
use purrdf_gts::segment_decode::{ResolvedSink, SegmentResolver};

use crate::{
    BlankScope, GtsBundle, RdfDatasetBuilder, RdfDiagnostic, RdfEnvelope, RdfLiteral, RdfLocation,
    RdfLookaside, RdfMetadataValue, RdfOpaqueNodeRecord, RdfSegmentRecord, RdfSignatureRecord,
    RdfSuppressionRecord, TermId, TermRef,
};

/// The immutable-IR emit target for GTS ingestion: an [`RdfDatasetBuilder`] that
/// interns each segment's blank nodes under a per-segment [`BlankScope`]. The
/// GTS-frame decode and two-phase resolution that drive it live once in
/// [`purrdf_gts::segment_decode`]; this type only interns resolved terms, pushes
/// resolved rows, and maps decode failures onto [`RdfDiagnostic`] locations.
struct SinkImporter {
    /// The fallible IR builder we intern terms and push structure into.
    builder: RdfDatasetBuilder,
    /// Out-of-band material accumulated from blob / signature / suppression /
    /// segment-head / opaque events.
    lookaside: RdfLookaside,
}

impl SinkImporter {
    fn new() -> Self {
        Self {
            builder: RdfDatasetBuilder::new(),
            lookaside: RdfLookaside::default(),
        }
    }
}

impl ResolvedSink for SinkImporter {
    type Id = TermId;
    type Error = RdfDiagnostic;

    fn intern_iri(
        &mut self,
        segment_index: usize,
        gts_id: usize,
        iri: &str,
    ) -> Result<TermId, RdfDiagnostic> {
        if iri.is_empty() {
            return Err(RdfDiagnostic::error(
                "rdf-ir-iri-missing-value",
                "GTS IRI term requires a non-empty value",
            )
            .with_location(
                RdfLocation::logical("gts:sink")
                    .with_gts_segment(segment_index)
                    .with_gts_term(gts_id),
            ));
        }
        Ok(self.builder.intern_iri(iri))
    }

    // Per-segment scope isolation (C0.2): scope = segment_index + 1 so the SAME
    // blank label in different segments interns to DISTINCT ids, while scope 0
    // stays reserved for the default/global scope.
    fn intern_blank(
        &mut self,
        segment_index: usize,
        _gts_id: usize,
        label: &str,
    ) -> Result<TermId, RdfDiagnostic> {
        let scope = BlankScope(segment_index as u32 + 1);
        Ok(self.builder.intern_blank(label, scope))
    }

    /// GTS *does* carry RDF 1.2 base direction (`purrdf-gts`'s `Term` has a
    /// `direction: Option<String>` slot), so it is parsed here via
    /// [`parse_gts_direction`](crate::gts_resolve::parse_gts_direction) and
    /// preserved onto the literal — direction is NOT a projection loss on this
    /// path. The datatype id is already resolved; it MUST resolve to an IRI.
    fn intern_literal(
        &mut self,
        segment_index: usize,
        gts_id: usize,
        lexical: String,
        datatype: Option<TermId>,
        lang: Option<String>,
        direction: Option<String>,
    ) -> Result<TermId, RdfDiagnostic> {
        let datatype = match datatype {
            Some(dt_id) => match self.builder.resolve(dt_id) {
                TermRef::Iri(iri) => Some(iri.to_string()),
                other => {
                    return Err(RdfDiagnostic::error(
                        "rdf-ir-literal-datatype-not-iri",
                        format!("GTS literal datatype must resolve to an IRI, got {other:?}"),
                    )
                    .with_location(
                        RdfLocation::logical("gts:sink")
                            .with_gts_segment(segment_index)
                            .with_gts_term(gts_id),
                    ));
                }
            },
            None => None,
        };
        // Parse direction (which requires the language tag) before moving the
        // owned `lexical`/`lang` strings into the literal.
        let direction =
            crate::gts_resolve::parse_gts_direction(direction.as_deref(), lang.as_deref())?;
        Ok(self.builder.intern_literal(RdfLiteral {
            lexical_form: lexical,
            datatype,
            language: lang,
            direction,
        }))
    }

    fn intern_triple(
        &mut self,
        _segment_index: usize,
        _gts_id: usize,
        s: TermId,
        p: TermId,
        o: TermId,
    ) -> Result<TermId, RdfDiagnostic> {
        Ok(self.builder.intern_triple(s, p, o))
    }

    fn push_quad(
        &mut self,
        _segment_index: usize,
        s: TermId,
        p: TermId,
        o: TermId,
        g: Option<TermId>,
    ) -> Result<(), RdfDiagnostic> {
        self.builder.push_quad(s, p, o, g);
        Ok(())
    }

    fn push_reifier(
        &mut self,
        _segment_index: usize,
        reifier: TermId,
        s: TermId,
        p: TermId,
        o: TermId,
        g: Option<TermId>,
    ) -> Result<(), RdfDiagnostic> {
        let triple_term = self.builder.intern_triple(s, p, o);
        self.builder.push_reifier_in_graph(reifier, triple_term, g);
        Ok(())
    }

    fn push_annotation(
        &mut self,
        _segment_index: usize,
        reifier: TermId,
        p: TermId,
        o: TermId,
        g: Option<TermId>,
    ) -> Result<(), RdfDiagnostic> {
        self.builder.push_annotation_in_graph(reifier, p, o, g);
        Ok(())
    }

    fn err_dangling_term(&self, segment_index: usize, gts_id: usize, role: &str) -> RdfDiagnostic {
        RdfDiagnostic::error(
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
        )
    }

    fn err_nesting_limit(&self, segment_index: usize, gts_id: usize) -> RdfDiagnostic {
        RdfDiagnostic::error(
            "rdf-ir-term-nesting-limit",
            "GTS triple-term nesting depth limit exceeded",
        )
        .with_location(
            RdfLocation::logical("gts:sink")
                .with_gts_segment(segment_index)
                .with_gts_term(gts_id),
        )
    }

    fn err_unbound_triple(&self, segment_index: usize, gts_id: usize) -> RdfDiagnostic {
        RdfDiagnostic::error(
            "rdf-ir-unbound-triple-term",
            "GTS triple term has no reifier binding",
        )
        .with_location(
            RdfLocation::logical("gts:sink")
                .with_gts_segment(segment_index)
                .with_gts_term(gts_id),
        )
    }

    fn err_missing_reifier(&self, segment_index: usize, reifier: usize) -> RdfDiagnostic {
        RdfDiagnostic::error(
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
        )
    }

    fn suppression(
        &mut self,
        _segment_index: usize,
        suppression: &Suppression,
    ) -> Result<(), RdfDiagnostic> {
        self.lookaside.suppressions.push(RdfSuppressionRecord {
            reason: suppression.reason.clone(),
            // `by` is a segment-local term id; we record it as a display hint
            // only, never as a cross-dataset id (C0.8).
            by: suppression.by.map(|term_id| format!("term#{term_id}")),
            targets: suppression
                .targets
                .iter()
                .map(metadata_value_from_cbor)
                .collect(),
        });
        Ok(())
    }

    fn blob(
        &mut self,
        _segment_index: usize,
        digest: &str,
        meta: Option<&Value>,
    ) -> Result<(), RdfDiagnostic> {
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
            // payload. The content digest above is the blob_id reference; the
            // streaming path records no segment-head origin here, whereas the
            // folded read path (`gts::blob_records`) populates it.
            origin: None,
        });
        Ok(())
    }

    fn opaque(&mut self, _segment_index: usize, opaque: &OpaqueNode) -> Result<(), RdfDiagnostic> {
        self.lookaside.opaque_nodes.push(RdfOpaqueNodeRecord {
            id: hex_bytes(&opaque.id),
            frame_type: opaque.frame_type.clone(),
            reason: opaque.reason.clone(),
            signature_status: opaque.sigstat.clone(),
            public_metadata: opaque.pub_meta.as_ref().map(metadata_value_from_cbor),
        });
        Ok(())
    }

    fn signature(
        &mut self,
        _segment_index: usize,
        signature: &Signature,
    ) -> Result<(), RdfDiagnostic> {
        self.lookaside.signatures.push(RdfSignatureRecord {
            frame_id: hex_bytes(&signature.frame_id),
            key_id: signature.kid.clone(),
            status: signature.status.clone(),
            has_cose: signature.cose.is_some(),
        });
        Ok(())
    }

    fn segment_head(&mut self, segment_index: usize, head: &[u8]) -> Result<(), RdfDiagnostic> {
        // Grow/patch the per-segment record with its head id.
        self.ensure_segment_record(segment_index).head = Some(hex_bytes(head));
        Ok(())
    }

    fn streamable_layout(
        &mut self,
        segment_index: usize,
        info: &StreamableInfo,
    ) -> Result<(), RdfDiagnostic> {
        let record = self.ensure_segment_record(segment_index);
        record.claimed_streamable = info.claimed;
        record.covered = info.covered;
        record.tail = info.tail;
        Ok(())
    }

    fn diagnostic(&mut self, diagnostic: &Diagnostic) -> Result<(), RdfDiagnostic> {
        // A reader diagnostic is a hard fold failure on the IR path (the IR is
        // the authority, no degraded fold). The `SegmentResolver` latches the
        // first one returned here.
        Err(RdfDiagnostic::error(
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
        }))
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
/// identity survives) through a
/// [`SegmentResolver`](purrdf_gts::segment_decode::SegmentResolver) that owns the
/// two-phase resolution: a quoted-triple term resolves regardless of whether its
/// `reifies` binding arrived before or after the term itself. Any reader
/// diagnostic or genuinely dangling term reference is a HARD failure (`Err`) — a
/// latched streaming error takes precedence over phase-2 resolution; on success
/// the interned terms are frozen via [`RdfDatasetBuilder::freeze`] and paired with
/// the envelope.
pub fn import_gts_events(bytes: &[u8]) -> Result<GtsBundle, RdfDiagnostic> {
    let mut resolver = SegmentResolver::new(SinkImporter::new());
    let _ = purrdf_gts::reader::read_to_sink(bytes, true, None, &mut resolver);

    // A latched streaming error (e.g. a reader diagnostic) precedes phase-2.
    if let Some(error) = resolver.take_error() {
        return Err(error);
    }

    // Flush the final segment: resolve its terms and push its rows.
    resolver.finish()?;

    let importer = resolver.into_sink();
    let lookaside = importer.lookaside;
    let dataset = importer.builder.freeze()?;
    Ok(GtsBundle::new(dataset, RdfEnvelope::new(lookaside)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_gts::model::{Graph, Term, Term as GtsTerm, TermKind, TermKind as GtsKind};
    use purrdf_gts::reader::StreamingSink;
    use purrdf_gts::writer::Writer;

    /// Convenience alias for a resolver wrapping the immutable-IR sink.
    type Resolver = SegmentResolver<SinkImporter>;

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

    /// Drive the streaming events then run phase 2, returning the resolver plus any
    /// error for post-resolution assertions (the public `import_gts_events` does
    /// both phases from bytes; this exercises the hand-ordered direct-sink path). A
    /// latched streaming error takes precedence over the phase-2 flush.
    fn finish_direct(mut resolver: Resolver) -> (Resolver, Option<RdfDiagnostic>) {
        if let Some(error) = resolver.take_error() {
            return (resolver, Some(error));
        }
        let error = resolver.finish().err();
        (resolver, error)
    }

    /// Resolve `our_id` back to its interned blank `(label, scope)` for assertions.
    fn blank_scope(resolver: &Resolver, id: TermId) -> (String, BlankScope) {
        match resolver.sink().builder.resolve(id) {
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
        let mut resolver = SegmentResolver::new(SinkImporter::new());

        // Segment 0: term 0 = ex:s, term 1 = ex:p, term 2 = _:b1, quad (s p b1).
        resolver.term(0, 0, &iri_term("http://example.org/s"));
        resolver.term(0, 1, &iri_term("http://example.org/p"));
        resolver.term(0, 2, &blank_term("b1"));
        resolver.quad(0, (0, 1, 2, None));

        // Segment 1: term 0 = ex:s (same IRI value), term 1 = ex:p2, term 2 = _:b1
        // (SAME label, DIFFERENT node), quad (s p2 b1).
        resolver.term(1, 0, &iri_term("http://example.org/s"));
        resolver.term(1, 1, &iri_term("http://example.org/p2"));
        resolver.term(1, 2, &blank_term("b1"));
        resolver.quad(1, (0, 1, 2, None));

        let (mut resolver, error) = finish_direct(resolver);
        assert!(error.is_none(), "no error expected: {error:?}");

        let seg0_s = resolver.remap(0, 0).expect("segment 0 subject resolved");
        let seg1_s = resolver.remap(1, 0).expect("segment 1 subject resolved");
        let seg0_b1 = resolver.remap(0, 2).expect("segment 0 blank resolved");
        let seg1_b1 = resolver.remap(1, 2).expect("segment 1 blank resolved");

        // The shared IRI interns to ONE id across both segments (value identity).
        assert_eq!(seg0_s, seg1_s, "ex:s is the same node across segments");

        // The same blank label in different segments interns to DISTINCT ids.
        assert_ne!(
            seg0_b1, seg1_b1,
            "_:b1 in segment 0 and segment 1 are DIFFERENT nodes (scope isolation)"
        );
        let (label0, scope0) = blank_scope(&resolver, seg0_b1);
        let (label1, scope1) = blank_scope(&resolver, seg1_b1);
        assert_eq!(label0, "b1");
        assert_eq!(label1, "b1");
        assert_eq!(scope0, BlankScope(1), "segment 0 → scope 1");
        assert_eq!(scope1, BlankScope(2), "segment 1 → scope 2");

        let dataset = std::mem::take(&mut resolver.sink_mut().builder)
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
        let mut resolver = SegmentResolver::new(SinkImporter::new());
        resolver.term(0, 0, &iri_term("http://example.org/s"));
        resolver.term(0, 1, &iri_term("http://example.org/p"));
        // Object id 9 was never introduced.
        resolver.quad(0, (0, 1, 9, None));
        let (_resolver, error) = finish_direct(resolver);
        let err = error.expect("dangling reference must defer an error");
        assert_eq!(err.code, "rdf-ir-dangling-term-ref");
    }

    /// Directional literals: GTS `Term` carries no base direction, so the sink path
    /// yields `direction == None`, but lexical form, datatype, and language survive.
    #[test]
    fn directional_literal_lexical_lang_survive_sink_path() {
        let mut resolver = SegmentResolver::new(SinkImporter::new());
        resolver.term(0, 0, &iri_term("http://example.org/s"));
        resolver.term(0, 1, &iri_term("http://example.org/p"));
        resolver.term(
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
        resolver.quad(0, (0, 1, 2, None));
        let (mut resolver, error) = finish_direct(resolver);
        assert!(error.is_none(), "{error:?}");

        let lit_id = resolver.remap(0, 2).expect("literal resolved");
        let dataset = std::mem::take(&mut resolver.sink_mut().builder)
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
        let mut resolver = SegmentResolver::new(SinkImporter::new());
        // Inner triple (ex:a ex:p ex:b) reified by reifier r0; outer triple
        // (ex:a ex:asserts <<inner>>) reified by reifier r1.
        resolver.term(0, 0, &iri_term("http://example.org/a"));
        resolver.term(0, 1, &iri_term("http://example.org/p"));
        resolver.term(0, 2, &iri_term("http://example.org/b"));
        resolver.term(0, 3, &iri_term("http://example.org/r0"));
        resolver.reifier(0, (3, (0, 1, 2), None));
        // Inner triple TERM bound to reifier r0 (gts id 3).
        resolver.term(
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
        resolver.term(0, 5, &iri_term("http://example.org/asserts"));
        resolver.term(0, 6, &iri_term("http://example.org/r1"));
        resolver.reifier(0, (6, (0, 5, 4), None));
        resolver.term(
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
        resolver.quad(0, (0, 5, 7, None));
        let (mut resolver, error) = finish_direct(resolver);
        assert!(error.is_none(), "{error:?}");

        let inner = resolver.remap(0, 4).expect("inner triple resolved");
        let outer = resolver.remap(0, 7).expect("outer triple resolved");
        let dataset = std::mem::take(&mut resolver.sink_mut().builder)
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
        let mut resolver = SegmentResolver::new(SinkImporter::new());
        resolver.term(0, 0, &iri_term("http://example.org/s"));
        resolver.term(0, 1, &iri_term("http://example.org/p"));
        resolver.term(0, 2, &iri_term("http://example.org/o"));
        resolver.term(0, 3, &iri_term("http://example.org/r1"));
        resolver.term(0, 4, &iri_term("http://example.org/r2"));
        resolver.reifier(0, (3, (0, 1, 2), None));
        resolver.reifier(0, (4, (0, 1, 2), None));
        let (mut resolver, error) = finish_direct(resolver);
        assert!(error.is_none(), "{error:?}");

        let dataset = std::mem::take(&mut resolver.sink_mut().builder)
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
        let mut resolver = SegmentResolver::new(SinkImporter::new());
        resolver.term(0, 0, &iri_term("http://example.org/s"));
        resolver.term(0, 1, &iri_term("http://example.org/asserts"));
        // Triple term bound to reifier id 99, which NO `reifies` event ever supplies.
        resolver.term(
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
        resolver.quad(0, (0, 1, 2, None));
        let (_resolver, error) = finish_direct(resolver);
        let err =
            error.expect("a genuinely dangling reifier binding must STILL fail after phase 2");
        assert_eq!(err.code, "rdf-ir-missing-reifier-binding");
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
