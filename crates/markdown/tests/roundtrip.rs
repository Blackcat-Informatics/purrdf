// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The codec's round trip, byte for byte: Markdown to RDF to Markdown.
//!
//! The decode leg is the real one. Every claim's Turtle is parsed back
//! through `purrdf-rdf` — the same parser any consumer holds — and the
//! spans are read off the *graph* by the decode law's two extraction
//! rules, never off the model or the `Claim` structs. What these
//! vectors prove is therefore the codec's whole promise: a consumer
//! holding only the triples rebuilds the document exactly, and the
//! digest proves it did.
//!
//! The refusals are proven in pairs, per the workspace discipline:
//! every typed refusal of [`reconstruct`] is executed beside the
//! neighbouring input that must still succeed, because an over-refusal
//! hides behind passing tests exactly as a silent drop does.

use purrdf_core::ContentDigest;
use purrdf_markdown::{
    Claim, Profile, ReconstructError, SourceDocument, VerbatimSpan, Vocabulary, reconstruct,
    slice_markdown,
};
use purrdf_rdf::parse_dataset;

const DOC_ID: &str = "https://example.org/doc/roundtrip";
const SLICE_BASE: &str = "https://example.org/slice/";
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

fn v() -> Vocabulary {
    Vocabulary::under(SLICE_BASE).expect("a vocabulary")
}

fn profile() -> Profile {
    Profile::new("roundtrip-md-v1", 1, v())
}

/// The declared law under a reduced bound, so long units split and the
/// split's declared overlap enters the graph.
fn tight(max_bytes: usize, overlap: usize) -> Profile {
    let mut p = profile();
    p.max_bytes = max_bytes;
    p.overlap = overlap;
    p
}

fn slice(text: &str, profile: &Profile) -> Vec<Claim> {
    slice_markdown(
        &SourceDocument {
            id: DOC_ID,
            bytes: text.as_bytes(),
        },
        profile,
    )
    .expect("slices")
}

/// One text-carrying span as read back off the graph, owned.
struct GraphSpan {
    byte_start: u64,
    byte_end: u64,
    text: String,
    continues: Option<(u64, u64)>,
}

/// What the decode law needs, extracted from the parsed triples alone:
/// the declared byte length, the stated source digest, and one span per
/// text-carrying fact — the two extraction rules of the law, and no
/// class dispatch.
struct GraphDocument {
    byte_length: u64,
    source_digest: ContentDigest,
    spans: Vec<GraphSpan>,
}

/// Every claim parsed back through the workspace parser and folded into
/// the decode law's inputs.
fn graph_document(claims: &[Claim]) -> GraphDocument {
    use purrdf_core::ir::TermRef;

    let turtle: String = claims.iter().map(|c| c.turtle.as_str()).collect();
    let dataset = parse_dataset(turtle.as_bytes(), "text/turtle", None).expect("the graph parses");

    // (subject, predicate) -> object, gathered as strings; the decode
    // law reads five predicates and two literal shapes and nothing
    // else.
    let mut byte_length: Option<u64> = None;
    let mut source_digest: Option<ContentDigest> = None;
    // subject -> facts the two extraction rules read.
    #[derive(Default)]
    struct Node {
        text: Option<String>,
        verbatim: Option<String>,
        byte_start: Option<u64>,
        byte_end: Option<u64>,
        verbatim_start: Option<u64>,
        verbatim_end: Option<u64>,
        continues: Option<String>,
    }
    let mut nodes: std::collections::BTreeMap<String, Node> = std::collections::BTreeMap::new();

    let voc = v();
    for quad in dataset.iter() {
        let TermRef::Iri(subject) = quad.s else {
            continue;
        };
        let TermRef::Iri(predicate) = quad.p else {
            continue;
        };
        let literal_value = match quad.o {
            TermRef::Literal { lexical, .. } => Some(lexical.to_owned()),
            _ => None,
        };
        let int_value = literal_value.as_deref().and_then(|s| s.parse::<u64>().ok());
        if subject == DOC_ID {
            if predicate == voc.byte_length {
                byte_length = int_value;
            }
            if predicate == voc.source_digest {
                let stated = literal_value.clone().expect("a digest literal");
                let hex = stated.strip_prefix("sha256:").expect("the algorithm tag");
                source_digest = Some(ContentDigest::from_hex(hex).expect("a digest"));
            }
        }
        let node = nodes.entry(subject.to_owned()).or_default();
        if predicate == voc.text {
            // The unit rule reads the one *plain* literal: `xsd:string`,
            // no language. Anything else under the text predicate is
            // not the unit's text.
            if let TermRef::Literal {
                lexical,
                datatype,
                language: None,
                ..
            } = quad.o
                && matches!(dataset.resolve(datatype), TermRef::Iri(XSD_STRING))
            {
                node.text = Some(lexical.to_owned());
            }
        } else if predicate == voc.verbatim {
            node.verbatim = literal_value;
        } else if predicate == voc.byte_start {
            node.byte_start = int_value;
        } else if predicate == voc.byte_end {
            node.byte_end = int_value;
        } else if predicate == voc.verbatim_start {
            node.verbatim_start = int_value;
        } else if predicate == voc.verbatim_end {
            node.verbatim_end = int_value;
        } else if predicate == voc.continues
            && let TermRef::Iri(piece) = quad.o
        {
            node.continues = Some(piece.to_owned());
        }
    }

    let mut spans = Vec::new();
    for node in nodes.values() {
        // Rule one: a node stating `verbatim` contributes it over its
        // verbatim span.
        if let Some(text) = &node.verbatim {
            spans.push(GraphSpan {
                byte_start: node.verbatim_start.expect("a verbatim span start"),
                byte_end: node.verbatim_end.expect("a verbatim span end"),
                text: text.clone(),
                continues: None,
            });
        }
        // Rule two: a unit contributes its plain text over its byte
        // span, with `continues` resolved to the named piece's own
        // span.
        if let Some(text) = &node.text {
            let continues = node.continues.as_ref().map(|piece| {
                let it = nodes
                    .get(piece)
                    .expect("the continued piece is in the graph");
                (
                    it.byte_start.expect("the piece's span start"),
                    it.byte_end.expect("the piece's span end"),
                )
            });
            spans.push(GraphSpan {
                byte_start: node.byte_start.expect("a span start"),
                byte_end: node.byte_end.expect("a span end"),
                text: text.clone(),
                continues,
            });
        }
    }
    GraphDocument {
        byte_length: byte_length.expect("the document states its length"),
        source_digest: source_digest.expect("the document states its digest"),
        spans,
    }
}

/// Markdown to RDF to Markdown, through the parser, byte for byte.
fn assert_roundtrips(text: &str, profile: &Profile) {
    let claims = slice(text, profile);
    let graph = graph_document(&claims);
    let spans: Vec<VerbatimSpan<'_>> = graph
        .spans
        .iter()
        .map(|s| VerbatimSpan {
            byte_start: s.byte_start,
            byte_end: s.byte_end,
            text: &s.text,
            continues: s.continues,
        })
        .collect();
    let rebuilt = reconstruct(graph.byte_length, &graph.source_digest, &spans)
        .expect("the graph decodes back to a document");
    assert_eq!(rebuilt, text, "and the document is the very bytes sliced");
    // The encode leg is deterministic in the bytes and the profile:
    // the same input renders the same graph, byte for byte.
    let again: String = slice(text, profile)
        .iter()
        .map(|c| c.turtle.as_str())
        .collect();
    let first: String = claims.iter().map(|c| c.turtle.as_str()).collect();
    assert_eq!(first, again);
}

/// The authored fixture, and the same fixture under bounds that split
/// its long units so the declared overlap enters the graph.
const GUIDE: &str = include_str!("fixtures/field-guide.md");

#[test]
fn the_field_guide_roundtrips_under_the_declared_law_and_under_splitting_bounds() {
    assert_roundtrips(GUIDE, &profile());
    for (max_bytes, overlap) in [(200, 40), (200, 0), (64, 63), (5, 2)] {
        assert_roundtrips(GUIDE, &tight(max_bytes, overlap));
    }
    let mut based = profile();
    based.canon_base = Some("https://example.org/canon#".to_owned());
    assert_roundtrips(GUIDE, &based);
}

#[test]
fn every_shape_of_document_roundtrips_byte_for_byte() {
    let fixtures: &[&str] = &[
        // Nothing at all, and nothing but structure.
        "",
        "\n",
        "\n\n\n",
        "---\n",
        // A bare paragraph, with and without its trailing newline.
        "one paragraph\n",
        "no trailing newline",
        "# T\n\n1. ends without a newline",
        // Two byte-identical paragraphs: two units, two spans, one
        // text — the case that catches any decoder keyed on content
        // rather than position.
        "same\n\nsame\n",
        // CRLF endings: the `\r` is in every span and every literal.
        "# T\r\n\r\n1. one\r\n\r\n2. two\r\n",
        // A leading byte order mark: encoding at byte zero, structure
        // afterwards, every byte still covered.
        "\u{feff}# T\u{ed}tulo\n\n1. \u{4e2d} \u{1f41a}\n",
        // Markers behind leading runs, and a run too deep to be one.
        "   # T\n\n  \u{2042} *m*\n\n 1. one\n\n    # content, not a heading\n",
        // A tab in the run: content, never a marker.
        "\t# content\n\nbody\n",
        // Horizontal rules and a table no concordance holds: structure
        // that lifts nothing, carried all the same.
        "# T\n\n---\n\n| a | b |\n| 1 | 2 |\n\n***\n\ntail\n",
        // A concordance that lifts, wide verses, an escaped pipe.
        "# T\n\n1. One.\n\n## Concordance\n\n| Verses | Canon source | Anchors |\n\
         | --- | --- | --- |\n| 1 | `atlas/x.ttl` | `a-one` |\n\
         | 2-9 | `atlas/y\\|z.ttl` | `a-two` |\n",
        // Movements as siblings, empty headings, seven hashes as prose.
        "## \u{201c}Deep\u{201d}\n\n\u{2042} *first*\n\n\u{2042} *second*\n\n####\n\n####### prose\n",
    ];
    for text in fixtures {
        assert_roundtrips(text, &profile());
        assert_roundtrips(text, &tight(6, 3));
    }
}

/// The A1 rider of the convergence: "maximal run" in its checkable
/// form. No two structure nodes may be adjacent — one's end equal to
/// the next's start — because a maximal run would have absorbed its
/// neighbour. Held over the graph, so a per-line emitter cannot
/// regress in behind the model's back.
#[test]
fn no_two_structure_nodes_are_adjacent_and_none_is_empty() {
    for p in [profile(), tight(6, 3)] {
        let claims = slice(GUIDE, &p);
        let mut runs: Vec<(u64, u64)> = claims
            .iter()
            .filter(|c| c.kind == purrdf_markdown::ClaimKind::Structure)
            .map(|c| c.span.expect("a structure span"))
            .collect();
        assert!(
            !runs.is_empty(),
            "the guide has structure between its units"
        );
        runs.sort_unstable();
        // Every run, the last included: `windows(2)` alone would leave
        // the final run unasserted and a one-run document unasserted
        // entirely.
        for &(start, end) in &runs {
            assert!(start < end, "a structure span covers bytes");
        }
        for pair in runs.windows(2) {
            assert!(
                pair[0].1 < pair[1].0,
                "two structure runs never touch: a maximal run absorbed its neighbour"
            );
        }
    }
}

/// "Plain means content, typed means bytes" — the A2 rider, held over
/// every literal of the graph: a unit's text and a section's heading
/// words are the only plain literals a claim carries, so an index that
/// selects plain strings sees content and not a byte of structure.
#[test]
fn the_only_plain_literals_are_unit_texts_and_heading_words() {
    use purrdf_core::ir::TermRef;
    let claims = slice(GUIDE, &tight(200, 40));
    let voc = v();
    let turtle: String = claims.iter().map(|c| c.turtle.as_str()).collect();
    let dataset = parse_dataset(turtle.as_bytes(), "text/turtle", None).expect("parses");
    let mut plain = 0usize;
    for quad in dataset.iter() {
        let TermRef::Iri(predicate) = quad.p else {
            continue;
        };
        if let TermRef::Literal {
            datatype,
            language: None,
            ..
        } = quad.o
            && matches!(dataset.resolve(datatype), TermRef::Iri(XSD_STRING))
        {
            plain += 1;
            assert!(
                predicate == voc.text || predicate == voc.heading,
                "a plain literal is a unit's text or a heading's words, never {predicate}"
            );
        }
    }
    assert!(plain > 0, "the guide has content to index");
}
