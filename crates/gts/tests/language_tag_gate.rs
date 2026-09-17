// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The container's language-tag gate, from both halves.
//!
//! A GTS container is bytes from anywhere, and its `"l"` field (GTS-SPEC §7.1,
//! "literal language tag (BCP 47)") is copied into every downstream rendering of
//! the literal — the fold view turns it into an N-Quads `LANGTAG` token, and the
//! event bridge hands it to an `RdfEventSink` that will intern it. So the reader
//! asks `purrdf_iri::langtag` on the profile every RDF codec and the IR kernel
//! already name, at the one point the tag enters from bytes
//! (`reader::h_terms`).
//!
//! Both halves are tested at every site, because a refusal is a claim too: the
//! tags this project's own containers carry — `x-purrdf-*` private-use tags past
//! the RFC 5646 §2.1 eight-character ceiling, and the frozen corpus's
//! `x-gmeow-english` — MUST still read, and the W3C corpora's `en-fr-jura` /
//! `fr-be-fbcl` with them.

use core::ops::ControlFlow;

use purrdf_events::{EventError, EventQuad, EventTerm, EventTermId, RdfEventSink, ScopeId};
use purrdf_gts::event_stream::{GtsEventSink, stream_events};
use purrdf_gts::model::{Term, TermKind, language_tag_refusal};
use purrdf_gts::reader::{ReadOptions, read};
use purrdf_gts::writer::Writer;

/// Tags a container legitimately carries. Refusing any of these would break
/// reading files this workspace itself writes, which is the over-refusal half of
/// the bug this gate exists to fix.
const ACCEPTED: &[&str] = &[
    "en",
    "en-US",
    "zh-Hans-CN",
    "de-CH-x-phonebk",
    "i-enochian",
    "x-purrdf-afrikaans",
    "x-gmeow-english",
    "en-fr-jura",
    "fr-be-fbcl",
    "abcdefgh",
    "en-x-cantbethislong",
];

/// Tags no conformant Turtle/TriG/N-Triples/N-Quads lexer would have produced.
/// Each one, carried through to a serializer, writes bytes no parser reads.
const REFUSED: &[&str] = &[
    "en us",
    "1",
    "9-9",
    "123-456",
    "en-",
    "-",
    "!!!",
    "abcdefghi",
];

fn iri(value: &str) -> Term {
    Term {
        kind: TermKind::Iri,
        value: Some(value.to_owned()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
        triple: None,
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
        triple: None,
    }
}

/// A one-quad container whose object is a literal carrying `lang`, authored
/// through the public writer exactly as a hostile (or merely older) producer
/// would author it: the writer copies `Term::lang` to the wire verbatim, so this
/// really is "arbitrary bytes", not a doctored structure.
fn container_with_tag(lang: &str) -> Vec<u8> {
    let mut writer = Writer::new("generic");
    writer.add_terms(&[
        iri("https://example.org/s"),
        iri("https://example.org/p"),
        lang_literal("Purr", lang),
    ]);
    writer.add_quads(&[(0, 1, 2, None)]);
    writer.into_bytes()
}

/// The same fixture folded through a `snapshot` frame, which re-enters the term
/// decode through `h_terms` after shifting ids — the one other way a term row
/// reaches the decoder.
fn snapshot_container_with_tag(lang: &str) -> Vec<u8> {
    let plain = container_with_tag(lang);
    let graph = read(&plain, true, None);
    let mut writer = Writer::new("generic");
    let payload = purrdf_gts::writer::snapshot_payload(&graph);
    writer.add_frame("snapshot", Some(payload), None, None, None);
    writer.into_bytes()
}

// -- the decode point --------------------------------------------------------

#[test]
fn accepted_tags_survive_the_round_trip() {
    for tag in ACCEPTED {
        assert_eq!(
            language_tag_refusal(tag),
            None,
            "{tag} must pass the grammar"
        );
        let graph = read(&container_with_tag(tag), true, None);
        assert_eq!(
            graph.terms[2].lang.as_deref(),
            Some(*tag),
            "{tag} must survive the container round-trip verbatim"
        );
        assert!(
            graph.diagnostics.is_empty(),
            "{tag} must not raise a diagnostic, got {:?}",
            graph.diagnostics
        );
    }
}

#[test]
fn refused_tags_never_reach_the_folded_term() {
    for tag in REFUSED {
        let graph = read(&container_with_tag(tag), true, None);
        assert_eq!(
            graph.terms[2].lang, None,
            "{tag:?} must not become a folded language tag"
        );
        // The lexical form survives — the tag is the damaged item, not the row.
        assert_eq!(graph.terms[2].value.as_deref(), Some("Purr"));
        assert_eq!(graph.quads.len(), 1, "{tag:?} must not cost the quad");
        let codes: Vec<&str> = graph
            .diagnostics
            .iter()
            .map(|d| d.code.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            codes,
            ["DamagedFrame"],
            "{tag:?} must raise exactly one DamagedFrame diagnostic"
        );
        let detail = &graph.diagnostics[0].detail;
        assert!(
            detail.contains(&format!("{tag:?}")),
            "the diagnostic must quote the refused tag verbatim, got {detail}"
        );
        assert!(
            detail.contains("term 2"),
            "the diagnostic must name the term id, got {detail}"
        );
    }
}

#[test]
fn the_snapshot_path_re_enters_the_same_gate() {
    for tag in REFUSED {
        let graph = read(&snapshot_container_with_tag(tag), true, None);
        let langs: Vec<Option<&str>> = graph.terms.iter().map(|t| t.lang.as_deref()).collect();
        assert!(
            langs.iter().all(Option::is_none),
            "{tag:?} must not survive a snapshot frame either, got {langs:?}"
        );
    }
    for tag in ACCEPTED {
        let graph = read(&snapshot_container_with_tag(tag), true, None);
        assert!(
            graph.terms.iter().any(|t| t.lang.as_deref() == Some(*tag)),
            "{tag} must still read through a snapshot frame"
        );
    }
}

// -- the event bridge --------------------------------------------------------

/// Captures every literal the bridge declares, so a test can state exactly what
/// a third-party `GtsEventSink` is handed.
#[derive(Default)]
struct LiteralCapture {
    literals: Vec<(String, String, Option<String>)>,
}

impl RdfEventSink for LiteralCapture {
    fn term(
        &mut self,
        _id: EventTermId,
        term: EventTerm<'_>,
    ) -> Result<ControlFlow<()>, EventError> {
        if let EventTerm::Literal {
            lexical,
            datatype,
            language,
            ..
        } = term
        {
            self.literals.push((
                lexical.to_owned(),
                datatype.to_owned(),
                language.map(str::to_owned),
            ));
        }
        Ok(ControlFlow::Continue(()))
    }

    fn quad(&mut self, _quad: EventQuad) -> Result<ControlFlow<()>, EventError> {
        Ok(ControlFlow::Continue(()))
    }

    fn reifier(
        &mut self,
        _reifier: EventTermId,
        _triple: purrdf_events::EventTriple,
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
        Ok(ScopeId(1))
    }

    fn close_scope(&mut self, _scope: ScopeId) -> Result<ControlFlow<()>, EventError> {
        Ok(ControlFlow::Continue(()))
    }

    fn finish(&mut self) -> Result<(), EventError> {
        Ok(())
    }
}

impl GtsEventSink for LiteralCapture {}

#[test]
fn the_event_bridge_never_emits_a_refused_tag() {
    for tag in REFUSED {
        let mut sink = LiteralCapture::default();
        stream_events(&container_with_tag(tag), ReadOptions::default(), &mut sink)
            .expect("the stream still drives to completion");
        assert_eq!(
            sink.literals,
            vec![(
                "Purr".to_owned(),
                "http://www.w3.org/2001/XMLSchema#string".to_owned(),
                None,
            )],
            "{tag:?} must not reach a GtsEventSink"
        );
    }
}

#[test]
fn the_event_bridge_still_emits_accepted_tags() {
    for tag in ACCEPTED {
        let mut sink = LiteralCapture::default();
        stream_events(&container_with_tag(tag), ReadOptions::default(), &mut sink)
            .expect("the stream drives to completion");
        assert_eq!(
            sink.literals,
            vec![(
                "Purr".to_owned(),
                "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
                Some((*tag).to_owned()),
            )],
            "{tag} must still reach a GtsEventSink as a language-tagged string"
        );
    }
}
