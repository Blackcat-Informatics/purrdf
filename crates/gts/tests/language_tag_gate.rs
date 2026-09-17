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

/// The `rdf:dirLangString` form: a literal carrying both halves. The direction
/// is a *valid* `ltr` throughout — the question these fixtures ask is never
/// whether a bad direction is caught, but what happens to a good one when the
/// tag beside it is refused.
fn dir_lang_literal(value: &str, lang: &str) -> Term {
    Term {
        kind: TermKind::Literal,
        value: Some(value.to_owned()),
        datatype: None,
        lang: Some(lang.to_owned()),
        direction: Some("ltr".to_owned()),
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

/// The same container, but the object is an `rdf:dirLangString` — both `"l"`
/// and `"dir"` present on the wire.
fn dir_container_with_tag(lang: &str) -> Vec<u8> {
    let mut writer = Writer::new("generic");
    writer.add_terms(&[
        iri("https://example.org/s"),
        iri("https://example.org/p"),
        dir_lang_literal("Purr", lang),
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

/// A refused tag must take its base direction with it, and must say that it did.
///
/// This is the refusal's own blast radius rather than a second bad input. A base
/// direction is only ever the second half of an `rdf:dirLangString`; once the tag
/// is gone it qualifies nothing, and a term carrying a direction with no language
/// is outside RDF 1.2's value space. Carrying it on is not an inert remnant — the
/// importer downstream refuses such a term with `gts-direction-without-language`,
/// which names a defect the container never had and points a reader away from the
/// tag that actually failed.
#[test]
fn a_refused_tag_takes_its_base_direction_with_it() {
    for tag in REFUSED {
        let graph = read(&dir_container_with_tag(tag), true, None);
        assert_eq!(
            graph.terms[2].lang, None,
            "{tag:?} must not become a folded language tag"
        );
        assert_eq!(
            graph.terms[2].direction, None,
            "{tag:?} must not leave a base direction standing with no language"
        );
        // The lexical form still survives: the damaged items are the tag and the
        // direction that depended on it, never the row.
        assert_eq!(graph.terms[2].value.as_deref(), Some("Purr"));
        assert_eq!(graph.quads.len(), 1, "{tag:?} must not cost the quad");

        // Two drops, two diagnostics. Dropping the direction silently would be
        // the documented mirror of over-refusal, and would strip a reader of the
        // one field needed to make sense of the first diagnostic.
        let codes: Vec<&str> = graph.diagnostics.iter().map(|d| d.code.as_str()).collect();
        assert_eq!(
            codes,
            ["DamagedFrame", "DamagedFrame"],
            "{tag:?} must diagnose the dropped direction as well as the refused tag"
        );
        let detail = &graph.diagnostics[1].detail;
        assert!(
            detail.contains("base direction") && detail.contains("term 2"),
            "the second diagnostic must name the dropped direction and its term, got {detail}"
        );
    }
}

/// The over-refusal half, which is the load-bearing one: an accepted tag keeps
/// its direction. The rule above must fire on a refusal, not on the presence of
/// a direction.
#[test]
fn an_accepted_tag_keeps_its_base_direction() {
    for tag in ACCEPTED {
        let graph = read(&dir_container_with_tag(tag), true, None);
        assert_eq!(
            graph.terms[2].lang.as_deref(),
            Some(*tag),
            "{tag} must survive alongside a base direction"
        );
        assert_eq!(
            graph.terms[2].direction.as_deref(),
            Some("ltr"),
            "{tag} is accepted, so its base direction must survive untouched"
        );
        assert!(
            graph.diagnostics.is_empty(),
            "{tag} with a valid direction must be silent, got {:?}",
            graph.diagnostics
        );
    }
}

/// A `"dir"` with no `"l"` at all is a different thing, and must stay different:
/// that is a genuine defect in the container rather than fallout from a refusal,
/// so nothing here should rename it. It is left to travel on and be reported
/// accurately downstream.
#[test]
fn a_direction_with_no_tag_at_all_is_left_alone() {
    let mut writer = Writer::new("generic");
    let mut object = dir_lang_literal("Purr", "en");
    object.lang = None;
    writer.add_terms(&[
        iri("https://example.org/s"),
        iri("https://example.org/p"),
        object,
    ]);
    writer.add_quads(&[(0, 1, 2, None)]);
    let graph = read(&writer.into_bytes(), true, None);

    assert_eq!(graph.terms[2].lang, None);
    assert_eq!(
        graph.terms[2].direction.as_deref(),
        Some("ltr"),
        "a direction that never had a tag beside it is the container's own defect, \
         not this gate's to rewrite"
    );
    assert!(
        graph.diagnostics.is_empty(),
        "no tag was refused here, so this gate must stay silent, got {:?}",
        graph.diagnostics
    );
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
