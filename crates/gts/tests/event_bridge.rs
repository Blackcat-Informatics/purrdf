// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests for the GTS → `RdfEventSink` bridge
//! (`purrdf_gts::event_stream::{stream_events, GtsEventSink, EventEmitter}`).
//!
//! These exercise ONLY public API: a self-contained `CollectSink` folds the
//! event stream back into canonical `(s, p, o, g)` string tuples and cross-checks
//! them against the offline `reader::read` union graph, plus the id/scope,
//! provenance, cancellation, and hard-failure contracts.

use std::collections::HashMap;

use core::ops::ControlFlow;

use purrdf_events::{
    EventError, EventQuad, EventTerm, EventTermId, EventTriple, RdfEventSink, ScopeId,
    TextDirection,
};
use purrdf_gts::event_stream::{EventEmitter, GtsEventSink, stream_events};
use purrdf_gts::model::{Graph, Term, TermKind, Triple3};
use purrdf_gts::reader::{FrameContext, ReadOptions, StreamingSink, read};
use purrdf_gts::replication::{DiffStatus, diff, inventory, splice};
use purrdf_gts::segment_decode::SegmentResolver;
use purrdf_gts::writer::Writer;

// -- fixtures ----------------------------------------------------------------

fn iri(value: &str) -> Term {
    Term {
        kind: TermKind::Iri,
        value: Some(value.to_owned()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
    }
}

fn plain_literal(value: &str) -> Term {
    Term {
        kind: TermKind::Literal,
        value: Some(value.to_owned()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
    }
}

fn lang_literal(value: &str, lang: &str) -> Term {
    Term {
        kind: TermKind::Literal,
        value: Some(value.to_owned()),
        datatype: None,
        lang: Some(lang.to_owned()),
        direction: None,
        reifier: None,
    }
}

fn blank(label: &str) -> Term {
    Term {
        kind: TermKind::Bnode,
        value: Some(label.to_owned()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
    }
}

/// A blank-free two-segment fixture: IRIs + literals + quads across two
/// independently-rooted segments.
fn two_segment_rdf_fixture() -> Vec<u8> {
    let mut a = Writer::new("generic");
    a.add_terms(&[
        iri("http://example.org/a/s"),
        iri("http://example.org/a/p"),
        plain_literal("alpha"),
        iri("http://example.org/name"),
        lang_literal("Purr", "en"),
    ]);
    a.add_quads(&[(0, 1, 2, None), (0, 3, 4, None)]);
    let mut data = a.to_bytes();

    let mut b = Writer::new("generic");
    b.add_terms(&[
        iri("http://example.org/b/s"),
        iri("http://example.org/b/p"),
        plain_literal("beta"),
    ]);
    b.add_quads(&[(0, 1, 2, None)]);
    data.extend_from_slice(&b.to_bytes());
    data
}

// -- canonical rendering -----------------------------------------------------

/// An owned copy of one declared [`EventTerm`], resolvable to a canonical string.
#[derive(Clone, PartialEq, Eq, Debug)]
enum OwnedTerm {
    Iri(String),
    Blank {
        label: String,
        scope: ScopeId,
    },
    Literal {
        lexical: String,
        datatype: String,
        language: Option<String>,
        direction: Option<TextDirection>,
    },
    Triple {
        s: EventTermId,
        p: EventTermId,
        o: EventTermId,
    },
}

impl OwnedTerm {
    fn capture(term: EventTerm<'_>) -> Self {
        match term {
            EventTerm::Iri(iri) => Self::Iri(iri.to_owned()),
            EventTerm::Blank { label, scope } => Self::Blank {
                label: label.to_owned(),
                scope,
            },
            EventTerm::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => Self::Literal {
                lexical: lexical.to_owned(),
                datatype: datatype.to_owned(),
                language: language.map(str::to_owned),
                direction,
            },
            EventTerm::Triple(EventTriple { s, p, o }) => Self::Triple { s, p, o },
        }
    }
}

fn render_literal(
    lexical: &str,
    datatype: &str,
    language: Option<&str>,
    direction: Option<TextDirection>,
) -> String {
    let lang = language.map_or_else(String::new, |l| format!("@{l}"));
    let dir = match direction {
        Some(TextDirection::Ltr) => "--ltr",
        Some(TextDirection::Rtl) => "--rtl",
        None => "",
    };
    format!("\"{lexical}\"^^<{datatype}>{lang}{dir}")
}

// -- collecting sink ---------------------------------------------------------

/// A self-contained sink that buffers declarations and rows, then resolves every
/// forward reference into canonical `(s, p, o, g)` string tuples on `finish`.
#[derive(Default)]
struct CollectSink {
    terms: HashMap<EventTermId, OwnedTerm>,
    declaration_order: Vec<(EventTermId, OwnedTerm)>,
    quads: Vec<EventQuad>,
    reifiers: Vec<(EventTermId, EventTriple)>,
    annotations: Vec<(EventTermId, EventTermId, EventTermId)>,
    resolved_quads: Vec<String>,
    finish_count: usize,
    next_scope: u32,
    break_on_first_quad: bool,
    frames: Vec<(Vec<u8>, usize, bool)>,
}

impl CollectSink {
    fn render(&self, id: EventTermId) -> String {
        match self
            .terms
            .get(&id)
            .expect("term declared before resolution")
        {
            OwnedTerm::Iri(iri) => format!("<{iri}>"),
            OwnedTerm::Blank { label, scope } => format!("_:{label}@{}", scope.0),
            OwnedTerm::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => render_literal(lexical, datatype, language.as_deref(), *direction),
            OwnedTerm::Triple { s, p, o } => {
                format!(
                    "<<{} {} {}>>",
                    self.render(*s),
                    self.render(*p),
                    self.render(*o)
                )
            }
        }
    }

    fn quad_tuple(&self, q: &EventQuad) -> String {
        let graph = q.g.map_or_else(|| "default".to_owned(), |g| self.render(g));
        format!(
            "{} {} {} {}",
            self.render(q.s),
            self.render(q.p),
            self.render(q.o),
            graph
        )
    }
}

impl RdfEventSink for CollectSink {
    fn term(
        &mut self,
        id: EventTermId,
        term: EventTerm<'_>,
    ) -> Result<ControlFlow<()>, EventError> {
        let owned = OwnedTerm::capture(term);
        self.declaration_order.push((id, owned.clone()));
        self.terms.insert(id, owned);
        Ok(ControlFlow::Continue(()))
    }

    fn quad(&mut self, q: EventQuad) -> Result<ControlFlow<()>, EventError> {
        if self.break_on_first_quad {
            return Ok(ControlFlow::Break(()));
        }
        self.quads.push(q);
        Ok(ControlFlow::Continue(()))
    }

    fn reifier(
        &mut self,
        reifier: EventTermId,
        triple: EventTriple,
    ) -> Result<ControlFlow<()>, EventError> {
        self.reifiers.push((reifier, triple));
        Ok(ControlFlow::Continue(()))
    }

    fn annotation(
        &mut self,
        reifier: EventTermId,
        p: EventTermId,
        o: EventTermId,
    ) -> Result<ControlFlow<()>, EventError> {
        self.annotations.push((reifier, p, o));
        Ok(ControlFlow::Continue(()))
    }

    fn open_scope(&mut self) -> Result<ScopeId, EventError> {
        self.next_scope += 1;
        Ok(ScopeId(self.next_scope))
    }

    fn close_scope(&mut self, _scope: ScopeId) -> Result<ControlFlow<()>, EventError> {
        Ok(ControlFlow::Continue(()))
    }

    fn finish(&mut self) -> Result<(), EventError> {
        self.finish_count += 1;
        let quads = std::mem::take(&mut self.quads);
        for q in &quads {
            self.resolved_quads.push(self.quad_tuple(q));
        }
        self.quads = quads;
        Ok(())
    }
}

impl GtsEventSink for CollectSink {
    fn frame(&mut self, ctx: FrameContext<'_>) -> Result<ControlFlow<()>, EventError> {
        self.frames
            .push((ctx.content_id.to_vec(), ctx.range.start, ctx.valid));
        Ok(ControlFlow::Continue(()))
    }
}

// -- offline rendering (mirror of CollectSink::render over a folded Graph) ----

fn render_offline(graph: &Graph, id: usize) -> String {
    let term = &graph.terms[id];
    match term.kind {
        TermKind::Iri => format!("<{}>", term.value.as_deref().unwrap_or_default()),
        TermKind::Bnode => format!(
            "_:{}@offline",
            term.value.clone().unwrap_or_else(|| format!("b{id}"))
        ),
        TermKind::Literal => render_literal(
            term.value.as_deref().unwrap_or_default(),
            &graph.datatype_iri(term),
            term.lang.as_deref(),
            match term.direction.as_deref() {
                Some("ltr") => Some(TextDirection::Ltr),
                Some("rtl") => Some(TextDirection::Rtl),
                _ => None,
            },
        ),
        TermKind::Triple => {
            let reifier = term.reifier.expect("triple term carries a reifier");
            let (s, p, o) = graph.reifier(reifier).expect("reifier binding present");
            format!(
                "<<{} {} {}>>",
                render_offline(graph, s),
                render_offline(graph, p),
                render_offline(graph, o)
            )
        }
    }
}

fn offline_quad_tuple(graph: &Graph, quad: (usize, usize, usize, Option<usize>)) -> String {
    let (s, p, o, g) = quad;
    let graph_name = g.map_or_else(|| "default".to_owned(), |g| render_offline(graph, g));
    format!(
        "{} {} {} {}",
        render_offline(graph, s),
        render_offline(graph, p),
        render_offline(graph, o),
        graph_name
    )
}

// -- tests -------------------------------------------------------------------

/// R7-exec: the bridge's resolved quad-set equals the offline `read` union graph.
#[test]
fn bridge_events_equal_offline_read() {
    let data = two_segment_rdf_fixture();

    let mut sink = CollectSink::default();
    let result = stream_events(&data, ReadOptions::new(true, None), &mut sink)
        .expect("stream_events must succeed on a clean fixture");
    assert!(result.diagnostics.is_empty(), "fixture must stream cleanly");
    assert_eq!(sink.finish_count, 1, "finish runs exactly once");

    let bridge: std::collections::HashSet<String> = sink.resolved_quads.iter().cloned().collect();

    let graph = read(&data, true, None);
    assert!(graph.diagnostics.is_empty(), "fixture must fold cleanly");
    let offline: std::collections::HashSet<String> = graph
        .quads
        .iter()
        .map(|&q| offline_quad_tuple(&graph, q))
        .collect();

    assert_eq!(
        bridge, offline,
        "bridged quads must equal offline union quads"
    );
    assert_eq!(bridge.len(), 3, "fixture declares three quads");
}

/// R7-exec2: the same blank label in two segments resolves to distinct scopes.
#[test]
fn per_segment_blank_scopes_are_distinct() {
    let mut a = Writer::new("generic");
    a.add_terms(&[
        iri("http://example.org/a/s"),
        iri("http://example.org/a/p"),
        blank("b0"),
    ]);
    a.add_quads(&[(0, 1, 2, None)]);
    let mut data = a.to_bytes();

    let mut b = Writer::new("generic");
    b.add_terms(&[
        iri("http://example.org/b/s"),
        iri("http://example.org/b/p"),
        blank("b0"),
    ]);
    b.add_quads(&[(0, 1, 2, None)]);
    data.extend_from_slice(&b.to_bytes());

    let mut sink = CollectSink::default();
    stream_events(&data, ReadOptions::new(true, None), &mut sink).expect("stream ok");

    let blanks: Vec<(String, ScopeId)> = sink
        .declaration_order
        .iter()
        .filter_map(|(_, term)| match term {
            OwnedTerm::Blank { label, scope } => Some((label.clone(), *scope)),
            _ => None,
        })
        .collect();
    assert_eq!(blanks.len(), 2, "each segment declares one blank");
    assert_eq!(blanks[0].0, blanks[1].0, "labels are the same _:b0");
    assert_ne!(
        blanks[0].1, blanks[1].1,
        "the two _:b0 must live in distinct scopes"
    );
}

/// A `GtsEventSink` recording only `content_id -> byte-offset` from the frame hook.
#[derive(Default)]
struct IndexSink {
    offsets: HashMap<Vec<u8>, usize>,
    diagnostics: usize,
}

impl RdfEventSink for IndexSink {
    fn term(
        &mut self,
        _id: EventTermId,
        _term: EventTerm<'_>,
    ) -> Result<ControlFlow<()>, EventError> {
        Ok(ControlFlow::Continue(()))
    }
    fn quad(&mut self, _q: EventQuad) -> Result<ControlFlow<()>, EventError> {
        Ok(ControlFlow::Continue(()))
    }
    fn reifier(
        &mut self,
        _reifier: EventTermId,
        _triple: EventTriple,
    ) -> Result<ControlFlow<()>, EventError> {
        Ok(ControlFlow::Continue(()))
    }
    fn annotation(
        &mut self,
        _reifier: EventTermId,
        _p: EventTermId,
        _o: EventTermId,
    ) -> Result<ControlFlow<()>, EventError> {
        Ok(ControlFlow::Continue(()))
    }
    fn open_scope(&mut self) -> Result<ScopeId, EventError> {
        Ok(ScopeId::DEFAULT)
    }
    fn close_scope(&mut self, _scope: ScopeId) -> Result<ControlFlow<()>, EventError> {
        Ok(ControlFlow::Continue(()))
    }
    fn finish(&mut self) -> Result<(), EventError> {
        Ok(())
    }
}

impl GtsEventSink for IndexSink {
    fn frame(&mut self, ctx: FrameContext<'_>) -> Result<ControlFlow<()>, EventError> {
        if ctx.valid {
            self.offsets
                .insert(ctx.content_id.to_vec(), ctx.range.start);
        }
        Ok(ControlFlow::Continue(()))
    }

    fn gts_diagnostic(
        &mut self,
        _ctx: Option<FrameContext<'_>>,
        _diagnostic: &purrdf_gts::model::Diagnostic,
    ) -> Result<ControlFlow<()>, EventError> {
        self.diagnostics += 1;
        Ok(ControlFlow::Continue(()))
    }
}

fn blob_fixture() -> Vec<u8> {
    let mut w = Writer::new("generic");
    w.add_blob(b"nine", Some("text/plain"), None);
    w.add_blob(b"lives", Some("text/plain"), None);
    w.add_blob(b"purr", Some("text/plain"), None);
    w.into_bytes()
}

/// R8-exec: the single-pass `content_id -> offset` index equals the offline
/// inventory, and a damaged frame surfaces a diagnostic rather than a silent gap.
#[test]
fn claimid_offset_index_single_pass() {
    let data = blob_fixture();

    let mut sink = IndexSink::default();
    let result = stream_events(&data, ReadOptions::new(true, None), &mut sink).expect("stream ok");
    assert!(result.diagnostics.is_empty(), "clean fixture");

    let inv = inventory(&data);
    assert!(!inv.has_problems(), "clean inventory");
    let expected: HashMap<Vec<u8>, usize> = inv
        .segments
        .iter()
        .flat_map(|s| s.frames.iter())
        .filter(|f| f.valid)
        .map(|f| (f.id.clone(), f.start))
        .collect();
    assert_eq!(
        sink.offsets, expected,
        "streamed frame offsets must equal the offline inventory"
    );
    assert!(!sink.offsets.is_empty(), "fixture has valid frames");

    // Damaged variant: corrupt a byte inside the last frame's payload; a
    // diagnostic must be surfaced, not a silent gap.
    let last = inv
        .segments
        .last()
        .and_then(|s| s.frames.last())
        .expect("fixture has frames");
    let mut damaged = data.clone();
    let target = last.start + (last.end - last.start) / 2;
    damaged[target] ^= 0xFF;

    let mut damaged_sink = IndexSink::default();
    let damaged_result =
        stream_events(&damaged, ReadOptions::new(true, None), &mut damaged_sink).expect("total");
    assert!(
        !damaged_result.diagnostics.is_empty() || damaged_sink.diagnostics > 0,
        "a damaged frame must surface a diagnostic, not a silent gap"
    );
    assert!(
        damaged_sink.offsets.len() < expected.len(),
        "the damaged frame must not appear as a clean valid frame"
    );
}

/// A successful drive calls `finish` exactly once.
#[test]
fn bridge_calls_finish_once() {
    let data = two_segment_rdf_fixture();
    let mut sink = CollectSink::default();
    stream_events(&data, ReadOptions::new(true, None), &mut sink).expect("stream ok");
    assert_eq!(sink.finish_count, 1);
    assert_eq!(sink.resolved_quads.len(), 3, "all quads resolved once");
}

/// A `Break` cancels the drive: `stream_events` returns `Ok`, but the sink is
/// never finalized and its collected output stays incomplete.
#[test]
fn bridge_break_cancels_drive() {
    let data = two_segment_rdf_fixture();
    let mut sink = CollectSink {
        break_on_first_quad: true,
        ..CollectSink::default()
    };
    let result = stream_events(&data, ReadOptions::new(true, None), &mut sink);
    assert!(result.is_ok(), "cancellation is not an error");
    assert_eq!(sink.finish_count, 0, "a cancelled drive is never finalized");
    assert!(
        sink.resolved_quads.is_empty(),
        "no quads were resolved (finish never ran)"
    );
}

/// Two drives over the same bytes mint an identical `EventTermId` sequence.
#[test]
fn stream_ids_are_file_order_deterministic() {
    let data = two_segment_rdf_fixture();

    let mut first = CollectSink::default();
    stream_events(&data, ReadOptions::new(true, None), &mut first).expect("stream ok");
    let mut second = CollectSink::default();
    stream_events(&data, ReadOptions::new(true, None), &mut second).expect("stream ok");

    assert_eq!(
        first.declaration_order, second.declaration_order,
        "id assignment must be a deterministic function of file order"
    );
    // And ids really are the dense 0..n minted in order.
    let minted: Vec<u32> = first.declaration_order.iter().map(|(id, _)| id.0).collect();
    assert_eq!(minted, (0..minted.len() as u32).collect::<Vec<_>>());
}

/// R9: re-indexing the whole segment(s) a diff fetched yields the same
/// `content_id -> offset` map as inventorying just those fetched bytes.
#[test]
fn incremental_reindex_from_diff_fetch() {
    let mut a = Writer::new("generic");
    a.add_blob(b"local-a", Some("text/plain"), None);
    a.add_blob(b"local-b", Some("text/plain"), None);
    let local = a.into_bytes();

    let mut b = Writer::new("generic");
    b.add_blob(b"remote-c", Some("text/plain"), None);
    b.add_blob(b"remote-d", Some("text/plain"), None);
    let mut remote = local.clone();
    remote.extend_from_slice(&b.to_bytes());

    let inv_local = inventory(&local);
    let inv_remote = inventory(&remote);
    let result = diff(&inv_local, &inv_remote);
    assert_eq!(result.status, DiffStatus::Fetch, "remote extends local");

    // Splice reconstructs the whole remote (sanity on the fetch plan).
    let spliced = splice(&local, &remote, &result).expect("splice ok");
    assert_eq!(spliced, remote, "splice must reconstruct remote");

    // The fetched bytes are the whole appended segment(s).
    let mut fetched = Vec::new();
    for f in &result.fetch {
        fetched.extend_from_slice(&remote[f.range.start..f.range.end]);
    }
    assert!(!fetched.is_empty(), "there is something to fetch");

    let mut sink = IndexSink::default();
    stream_events(&fetched, ReadOptions::new(true, None), &mut sink).expect("stream ok");

    let inv_fetched = inventory(&fetched);
    assert!(
        !inv_fetched.has_problems(),
        "fetched bytes inventory cleanly"
    );
    let expected: HashMap<Vec<u8>, usize> = inv_fetched
        .segments
        .iter()
        .flat_map(|s| s.frames.iter())
        .filter(|f| f.valid)
        .map(|f| (f.id.clone(), f.start))
        .collect();

    assert_eq!(
        sink.offsets, expected,
        "segment-aligned re-index of the fetched bytes must match their inventory"
    );
}

/// A graph-scoped reifier cannot be carried on the graph-less RDF event
/// protocol, so the bridge hard-fails rather than dropping the graph.
#[test]
fn graph_scoped_reifier_is_hard_error() {
    let mut w = Writer::new("generic");
    w.add_terms(&[
        iri("http://example.org/s"),
        iri("http://example.org/p"),
        iri("http://example.org/o"),
        iri("http://example.org/g"),
        iri("http://example.org/reifier"),
    ]);
    // reifier id 4 binds (0,1,2) in named graph 3.
    let binding: Vec<purrdf_gts::model::ReifierRow> = vec![(4, (0, 1, 2), Some(3))];
    w.add_reifies(&binding);
    let data = w.into_bytes();

    // The fixture itself is clean at the GTS layer (the graph-scoped reifier is
    // legal on the container; it is only the graph-less event protocol that
    // cannot carry it).
    let graph = read(&data, true, None);
    assert!(graph.diagnostics.is_empty(), "container fixture is clean");
    let _: Triple3 = (0, 1, 2);

    let mut sink = CollectSink::default();
    let err = stream_events(&data, ReadOptions::new(true, None), &mut sink)
        .expect_err("a graph-scoped reifier must hard-fail on the event protocol");
    assert!(
        err.to_string().contains("graph-scoped reifier"),
        "error names the unrepresentable graph slot: {err}"
    );
    assert_eq!(sink.finish_count, 0, "a failed drive is never finalized");
}

/// R6-exec2 (real bytes): a quad naming an undeclared segment-local term id,
/// authored through the PUBLIC `Writer`, is a hard `Err` when driven through
/// the PUBLIC `stream_events` byte path — no silent skip, no partial index.
///
/// `Writer::add_quads` takes raw `usize` ids and performs NO client-side
/// validation, so a quad naming an id no `term` event ever introduced
/// round-trips into a fully well-formed, correctly content-hashed, correctly
/// chained GTS frame (contrary to the `dangling_term_ref_is_err` doc comment
/// below in an earlier revision of this suite, which assumed no such fixture
/// was authorable). The raw GTS reader is a deliberately permissive Baseline
/// Reader (GTS-SPEC §7.4/§7.5): `reader::read`/`read_to_sink` themselves stay
/// `Ok`, diagnosing the row as `PositionConstraint` and dropping it — the
/// SAME behavior the frozen conformance vector
/// `vectors/13-position-constraint.expected.json` pins. `EventEmitter`'s
/// `ResolvedSink::diagnostic` escalates exactly that diagnostic (and its
/// `ForwardReference` sibling) to a hard `EventError`, so the public
/// `stream_events` byte path fails closed instead of silently finalizing a
/// sink that never even saw the dropped quad.
#[test]
fn dangling_term_ref_real_bytes_is_err() {
    let mut w = Writer::new("generic");
    w.add_terms(&[
        iri("http://example.org/s"),
        iri("http://example.org/p"),
        iri("http://example.org/o"),
    ]);
    // Object id 99 was never introduced by any `term` event in this segment.
    w.add_quads(&[(0, 1, 99, None)]);
    let data = w.into_bytes();

    // Sanity: the raw GTS reader is intentionally permissive here (Baseline
    // Reader degrade-and-continue, §7.6) — confirms the fixture reaches
    // exactly the dangling-reference diagnostic, not some unrelated earlier
    // hard-fail (a bad chain/hash would abort before position checking runs).
    let graph = read(&data, true, None);
    assert_eq!(
        graph.diagnostics.len(),
        1,
        "the raw reader diagnoses exactly the dangling quad: {:?}",
        graph.diagnostics
    );
    assert_eq!(graph.diagnostics[0].code, "PositionConstraint");
    assert!(
        graph.quads.is_empty(),
        "the raw reader drops the dangling quad rather than materializing it"
    );

    // The public RDF event byte path must fail closed.
    let mut sink = CollectSink::default();
    let err = stream_events(&data, ReadOptions::new(true, None), &mut sink)
        .expect_err("a dangling quad reference must hard-fail stream_events on real bytes");
    assert!(
        err.to_string().contains("dangling term reference"),
        "error names the dangling reference: {err}"
    );

    // No partial index: the sink never received a single event.
    assert_eq!(sink.finish_count, 0, "a failed drive is never finalized");
    assert!(
        sink.declaration_order.is_empty(),
        "no term was ever declared to the sink — the segment never resolved"
    );
    assert!(
        sink.quads.is_empty(),
        "no quad was ever declared to the sink"
    );
}

/// R6-exec2 (defense in depth): the SAME `SegmentResolver` + `EventEmitter`
/// bridge `stream_events` builds internally ALSO hard-fails a dangling quad
/// reference when driven directly with hand-fed streaming events — pinning
/// the resolver's own no-optionality contract independent of whatever the
/// raw byte reader happens to filter first. See
/// `dangling_term_ref_real_bytes_is_err` above for the real-byte-path proof.
#[test]
fn dangling_term_ref_is_err() {
    let mut sink = CollectSink::default();
    let mut resolver = SegmentResolver::new(EventEmitter::new(&mut sink));

    resolver.term(0, 0, &iri("http://example.org/s"));
    resolver.term(0, 1, &iri("http://example.org/p"));
    resolver.term(0, 2, &iri("http://example.org/o"));
    // Quad references gts id 99, which no `term` event ever introduced.
    resolver.quad(0, (0, 1, 99, None));

    assert!(
        resolver.take_error().is_none(),
        "buffering phase latches no error"
    );
    let err = resolver
        .finish()
        .expect_err("a dangling quad reference must surface at resolution");
    assert!(
        err.to_string().contains("dangling term reference"),
        "error names the dangling reference: {err}"
    );
    assert_eq!(sink.finish_count, 0, "the failed drive was never finalized");
}

/// Feed ONLY the first term of segment `seg`. This single event is what trips
/// `SegmentResolver::advance_segment` (the incoming `segment_index` exceeds
/// the currently-buffered one), which synchronously flushes whichever
/// segment was previously buffered — resolving its terms/quads/reifiers into
/// the sink and, per the bounded-memory fix, clearing `reifier_bindings` and
/// `remaps` — before this segment starts accumulating anything of its own.
fn open_bounded_memory_segment(resolver: &mut SegmentResolver<EventEmitter<'_>>, seg: usize) {
    resolver.term(seg, 0, &iri(&format!("http://example.org/seg{seg}/s")));
}

/// Feed the REST of segment `seg` (4 more IRI terms, one quad, and two
/// reifiers binding that quad's triple) — everything [`open_bounded_memory_segment`]
/// did not already send. Together the two functions introduce 5 distinct IRI
/// terms and 2 reifier bindings per segment, all segment-qualified
/// (`http://example.org/seg{seg}/...`) so no cross-segment id reuse could
/// accidentally mask a leak as "the same entry, re-seen".
fn fill_bounded_memory_segment(resolver: &mut SegmentResolver<EventEmitter<'_>>, seg: usize) {
    resolver.term(seg, 1, &iri(&format!("http://example.org/seg{seg}/p")));
    resolver.term(seg, 2, &iri(&format!("http://example.org/seg{seg}/o")));
    resolver.term(seg, 3, &iri(&format!("http://example.org/seg{seg}/r1")));
    resolver.term(seg, 4, &iri(&format!("http://example.org/seg{seg}/r2")));
    resolver.quad(seg, (0, 1, 2, None));
    resolver.reifier(seg, (3, (0, 1, 2), None));
    resolver.reifier(seg, (4, (0, 1, 2), None));
}

/// R8-exec: bounded-memory streaming fold (GTS-SPEC §7.7). `SegmentResolver`'s
/// per-segment `reifier_bindings` / `remaps` maps, and the bridged
/// `EventEmitter`'s `iri_map`, must stay bounded to O(one segment) as MANY
/// segments stream through — never grow with the cumulative segment count.
///
/// This drives a hand-built 12-segment fixture directly through
/// `SegmentResolver`'s public `StreamingSink` callbacks (no GTS byte
/// encoding needed — the same technique `dangling_term_ref_is_err` above
/// uses) and samples the resolver's and emitter's retained sizes immediately
/// after each segment's flush (i.e. right after the NEXT segment's first
/// event, before that next segment has contributed anything of its own). A
/// regression that stops clearing `reifier_bindings`/`remaps`
/// (`resolve_buffered`) or `iri_map` (`EventEmitter::ensure_scope`) makes
/// these counts grow with the segment index instead of staying flat, so this
/// assertion is falsifiable: reverting either clear turns it red.
#[test]
fn bounded_memory_across_many_segments() {
    const SEGMENTS: usize = 12;
    const IRIS_PER_SEGMENT: usize = 5;

    let mut sink = CollectSink::default();
    let mut resolver = SegmentResolver::new(EventEmitter::new(&mut sink));

    open_bounded_memory_segment(&mut resolver, 0);
    fill_bounded_memory_segment(&mut resolver, 0);
    for seg in 1..SEGMENTS {
        let flushed = seg - 1;
        // Trips the flush of `flushed`; segment `seg` has contributed
        // nothing yet at this point (only its first term is buffered).
        open_bounded_memory_segment(&mut resolver, seg);
        assert_eq!(
            resolver.buffered_reifier_binding_count(),
            0,
            "flushing segment {flushed} (triggered by segment {seg}'s first \
             event) must have emptied reifier_bindings (cleared at flush), \
             not accumulated the {expected} reifiers introduced across all \
             segments seen so far",
            expected = 2 * seg,
        );
        assert_eq!(
            resolver.buffered_remap_count(),
            0,
            "flushing segment {flushed} (triggered by segment {seg}'s first \
             event) must have emptied the gts-id remap memo (cleared at \
             flush), not accumulated resolutions across segments"
        );
        assert_eq!(
            resolver.sink().interned_iri_count(),
            IRIS_PER_SEGMENT,
            "after flushing segment {flushed}, EventEmitter.iri_map must \
             hold only that segment's own {IRIS_PER_SEGMENT} IRIs, not the \
             cumulative total across all segments flushed so far"
        );
        fill_bounded_memory_segment(&mut resolver, seg);
    }

    // No StreamingSink callback failure was latched.
    assert!(
        resolver.take_error().is_none(),
        "the hand-fed fixture is well-formed"
    );
    // Flush the final (12th) segment explicitly — nothing downstream ever
    // starts segment 12 to trigger it automatically — and re-check the same
    // bound on the last segment too.
    resolver
        .finish()
        .expect("finish resolves the final buffered segment");
    assert_eq!(
        resolver.buffered_reifier_binding_count(),
        0,
        "the final segment's reifier_bindings must also be cleared at finish()"
    );
    assert_eq!(
        resolver.buffered_remap_count(),
        0,
        "the final segment's remap memo must also be cleared at finish()"
    );
    assert_eq!(
        resolver.sink().interned_iri_count(),
        IRIS_PER_SEGMENT,
        "the final segment's iri_map must hold only its own \
         {IRIS_PER_SEGMENT} IRIs, not the cumulative total across all \
         {SEGMENTS} segments"
    );
}
