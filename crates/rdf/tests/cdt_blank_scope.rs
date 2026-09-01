// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SEP-0009 blank-node scoping for `cdt:List` / `cdt:Map` literals, end to end.
//!
//! A composite literal's lexical form may hold `BLANK_NODE_LABEL` tokens, and
//! those tokens denote blank nodes in the SAME scope as the surrounding
//! document. This file pins that rule from the codec surface — the shape a
//! consumer actually sees — case by case against the upstream `bnodes`
//! conformance group, and then pins the round trip: a composite literal holding
//! a blank node survives parse → store → serialize → parse with legal,
//! non-conflating labels and preserved identity, in every native format.
//!
//! The scoping rule, as the corpus states it:
//!
//! 1. One document is one blank-node scope, and EVERY `BLANK_NODE_LABEL` in it
//!    resolves through that one scope, whether written as a term or inside a
//!    composite literal (`bnodes-turtle-01`, `-05`).
//! 2. Nesting never opens a new scope — not a direct `[…]` / `{…}` sub-value
//!    (`bnodes-turtle-21`), and not a composite-typed literal embedded as a
//!    string (`bnodes-turtle-41`).
//! 3. Different documents are different scopes (`bnodes-turtle-15`, `-17`).

use std::collections::BTreeSet;
use std::sync::Arc;

use purrdf_rdf::{
    NativeRdfFormat, RdfDataset, RdfDatasetBuilder, SerializeGraph, TermId, TermRef, parse_dataset,
    serialize_dataset,
};

const LIST: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List";
const MAP: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/Map";

const PREFIXES: &str = "\
PREFIX cdt: <http://w3id.org/awslabs/neptune/SPARQL-CDTs/>
PREFIX ex:  <http://example.org/>
";

/// Parse a Turtle document body (the corpus prefixes are prepended).
fn turtle(body: &str) -> Arc<RdfDataset> {
    let text = format!("{PREFIXES}{body}");
    parse_dataset(text.as_bytes(), "text/turtle", None)
        .unwrap_or_else(|e| panic!("the fixture must parse: {e}\n{text}"))
}

/// The composite literal object of the quad whose predicate is `ex:{local}`,
/// together with its datatype IRI.
fn composite_of(ds: &RdfDataset, local: &str) -> (String, String) {
    let predicate = format!("http://example.org/{local}");
    for quad in ds.quads() {
        let TermRef::Iri(p) = ds.resolve(quad.p) else {
            continue;
        };
        if p != predicate {
            continue;
        }
        if let TermRef::Literal {
            lexical, datatype, ..
        } = ds.resolve(quad.o)
        {
            let TermRef::Iri(dt) = ds.resolve(datatype) else {
                panic!("a literal datatype is always an IRI");
            };
            return (lexical.to_owned(), dt.to_owned());
        }
    }
    panic!("no composite literal object on ex:{local}");
}

/// The dataset node ids the composite literal on `ex:{local}` names, in
/// occurrence order. Every one MUST resolve: a label that does not is a blank
/// node the store failed to intern.
fn embedded_ids(ds: &RdfDataset, local: &str) -> Vec<TermId> {
    let (lexical, datatype) = composite_of(ds, local);
    purrdf_core::cdt_blank::cdt_embedded_blanks(&lexical, &datatype)
        .into_iter()
        .map(|(label, scope)| {
            ds.term_id_by_blank(&label, scope).unwrap_or_else(|| {
                panic!("the embedded label {label:?} at {scope:?} names no node of the dataset")
            })
        })
        .collect()
}

/// The subject of the quad on `ex:{local}`.
fn subject_of(ds: &RdfDataset, local: &str) -> TermId {
    let predicate = format!("http://example.org/{local}");
    ds.quads()
        .find(|q| matches!(ds.resolve(q.p), TermRef::Iri(p) if p == predicate))
        .map(|q| q.s)
        .expect("a quad on that predicate")
}

// ── Rule 1: one document, one scope ─────────────────────────────────────────

/// `bnodes-turtle-01`: the two `_:b` occurrences inside ONE `cdt:List` literal
/// are the same blank node — `cdt:get(?list,1) = cdt:get(?list,3)`.
#[test]
fn one_label_twice_in_one_literal_is_one_node() {
    let ds = turtle(r#"ex:s ex:p "[_:b, 42, _:b]"^^cdt:List ."#);
    let ids = embedded_ids(&ds, "p");
    assert_eq!(ids.len(), 2);
    assert_eq!(ids[0], ids[1], "the two occurrences must be one node");
}

/// `bnodes-turtle-02`: the `cdt:Map` twin of the above.
#[test]
fn one_label_twice_in_one_map_literal_is_one_node() {
    let ds = turtle(r#"ex:s ex:p "{ '1': _:b, '2': 42, '3': _:b }"^^cdt:Map ."#);
    let ids = embedded_ids(&ds, "p");
    assert_eq!(ids.len(), 2);
    assert_eq!(ids[0], ids[1]);
}

/// `bnodes-turtle-03` / `-04`: distinct labels are distinct nodes.
#[test]
fn distinct_labels_are_distinct_nodes() {
    let ds = turtle(r#"ex:s ex:p "[_:b1, 42, _:b2]"^^cdt:List ."#);
    let ids = embedded_ids(&ds, "p");
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
}

/// `bnodes-turtle-05`: a label inside a composite literal and a label written as
/// a TERM in the same document, spelled the same, are the same node —
/// `FILTER(?s = ?e1)`. This is the rule's whole point.
#[test]
fn a_label_inside_a_literal_is_the_document_scope_node() {
    let ds = turtle(r#"_:b ex:p "[_:b, 42]"^^cdt:List ."#);
    assert_eq!(embedded_ids(&ds, "p")[0], subject_of(&ds, "p"));
}

/// `bnodes-turtle-06`: the `cdt:Map` twin.
#[test]
fn a_label_inside_a_map_literal_is_the_document_scope_node() {
    let ds = turtle(r#"_:b ex:p "{ '1': _:b, '2': 42 }"^^cdt:Map ."#);
    assert_eq!(embedded_ids(&ds, "p")[0], subject_of(&ds, "p"));
}

/// `bnodes-turtle-07` / `-08`: differently spelled ones are NOT.
#[test]
fn a_differently_spelled_label_is_a_different_node() {
    let ds = turtle(r#"_:b1 ex:p "[_:b2, 42]"^^cdt:List ."#);
    assert_ne!(embedded_ids(&ds, "p")[0], subject_of(&ds, "p"));
}

/// `bnodes-turtle-09` / `-10`: the same label in TWO composite literals of one
/// document is one node.
#[test]
fn one_label_across_two_literals_is_one_node() {
    let ds = turtle(
        r#"ex:s ex:p1 "[_:b, 42]"^^cdt:List .
ex:s ex:p2 "[_:b, 43]"^^cdt:List ."#,
    );
    assert_eq!(embedded_ids(&ds, "p1")[0], embedded_ids(&ds, "p2")[0]);
}

/// `bnodes-turtle-11` / `-12`: distinct labels across two literals stay
/// distinct.
#[test]
fn distinct_labels_across_two_literals_stay_distinct() {
    let ds = turtle(
        r#"ex:s ex:p1 "[_:b1, 42]"^^cdt:List .
ex:s ex:p2 "[_:b2, 43]"^^cdt:List ."#,
    );
    assert_ne!(embedded_ids(&ds, "p1")[0], embedded_ids(&ds, "p2")[0]);
}

/// `bnodes-turtle-13`: the scope spans the two DATATYPES too — a label in a
/// `cdt:List` and the same label in a `cdt:Map` are one node.
#[test]
fn one_label_across_a_list_and_a_map_is_one_node() {
    let ds = turtle(
        r#"ex:s ex:p1 "[      _:b,      42 ]"^^cdt:List .
ex:s ex:p2 "{ '1': _:b, '2': 43 }"^^cdt:Map ."#,
    );
    assert_eq!(embedded_ids(&ds, "p1")[0], embedded_ids(&ds, "p2")[0]);
}

// ── Rule 2: nesting opens no new scope ──────────────────────────────────────

/// `bnodes-turtle-21`: a directly nested composite is in the same scope.
#[test]
fn direct_nesting_is_the_same_scope() {
    let ds = turtle(r#"ex:s ex:p "[_:b, 42, [_:b] ]"^^cdt:List ."#);
    let ids = embedded_ids(&ds, "p");
    assert_eq!(ids.len(), 2);
    assert_eq!(ids[0], ids[1]);
}

/// `bnodes-turtle-25`: a nested composite reaches the document's term-position
/// blank too.
#[test]
fn direct_nesting_reaches_the_document_subject() {
    let ds = turtle(r#"_:b ex:p " [ [_:b], 42]"^^cdt:List ."#);
    assert_eq!(embedded_ids(&ds, "p")[0], subject_of(&ds, "p"));
}

/// `bnodes-turtle-41`: nesting through an EMBEDDED composite-typed literal is
/// the same scope. The corpus gives `-41` the very query it gives `-21`, so the
/// two spellings must produce the same verdict.
#[test]
fn embedded_composite_literal_nesting_is_the_same_scope() {
    let ds = turtle(
        r#"ex:s ex:p "[_:b, 42, '[_:b]'^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/List> ]"^^cdt:List ."#,
    );
    let ids = embedded_ids(&ds, "p");
    assert_eq!(ids.len(), 2);
    assert_eq!(ids[0], ids[1]);
}

/// `bnodes-turtle-45`: and it reaches the document's term-position blank.
#[test]
fn embedded_composite_literal_nesting_reaches_the_document_subject() {
    let ds = turtle(
        r#"_:b ex:p "[ '[_:b]'^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/List>, 42]"^^cdt:List ."#,
    );
    assert_eq!(embedded_ids(&ds, "p")[0], subject_of(&ds, "p"));
}

/// `bnodes-turtle-43`: distinct labels through an embedded literal stay
/// distinct.
#[test]
fn embedded_composite_literal_nesting_keeps_distinct_labels_distinct() {
    let ds = turtle(
        r#"ex:s ex:p "[_:b1, 42, '[_:b2]'^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/List> ]"^^cdt:List ."#,
    );
    let ids = embedded_ids(&ds, "p");
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
}

// ── Rule 3: a different document is a different scope ───────────────────────

/// `bnodes-turtle-15`: the same label inside composite literals in TWO documents
/// names two DIFFERENT nodes — `FILTER(?e1 != ?e2)`. Merging assigns each source
/// a fresh scope and the embedded labels must follow it.
#[test]
fn two_documents_are_two_scopes() {
    let a = turtle(r#"ex:s ex:p1 "[_:b, 42]"^^cdt:List ."#);
    let b = turtle(r#"ex:s ex:p2 "[_:b, 43]"^^cdt:List ."#);
    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(&a);
    builder.push_dataset(&b);
    let merged = builder.freeze().expect("a valid merge");

    assert_ne!(
        embedded_ids(&merged, "p1")[0],
        embedded_ids(&merged, "p2")[0],
        "the same label in two documents must be two nodes"
    );
}

/// `bnodes-turtle-17`: the same holds when one document writes the label as a
/// term and the other writes it inside a composite literal.
#[test]
fn two_documents_are_two_scopes_across_the_literal_boundary() {
    let a = turtle(r#"ex:s ex:p1 "[_:b, 42]"^^cdt:List ."#);
    let b = turtle(r"ex:s ex:p2 _:b .");
    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(&a);
    builder.push_dataset(&b);
    let merged = builder.freeze().expect("a valid merge");

    let bare = merged
        .quads()
        .find(|q| matches!(merged.resolve(q.o), TermRef::Blank { .. }))
        .map(|q| q.o)
        .expect("the bare blank object");
    assert_ne!(embedded_ids(&merged, "p1")[0], bare);
}

/// A merge is still a MERGE: within one source the identity holds after it.
#[test]
fn a_merge_preserves_identity_within_each_source() {
    let a = turtle(r#"_:b ex:p1 "[_:b, 42]"^^cdt:List ."#);
    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(&a);
    let merged = builder.freeze().expect("a valid merge");
    assert_eq!(embedded_ids(&merged, "p1")[0], subject_of(&merged, "p1"));
}

// ── Egress: the round trip ──────────────────────────────────────────────────

/// Every native format that carries a plain typed literal on a blank subject.
/// RDF/XML has no named-graph surface but that is irrelevant here; every format
/// in this list round-trips a one-quad default-graph dataset.
fn round_trip_formats() -> Vec<NativeRdfFormat> {
    NativeRdfFormat::all().collect()
}

/// **The round trip.** A composite literal holding a blank node that is ALSO the
/// quad's subject survives parse → store → serialize → parse in every native
/// format, with labels that are legal in the target syntax and that do not
/// conflate two nodes into one.
///
/// Identity, not merely isomorphism, is what is checked on the far side: the
/// re-parsed literal's embedded label must resolve to the re-parsed dataset's
/// own subject node.
#[test]
fn a_composite_literal_with_a_blank_node_round_trips_through_every_format() {
    let source = turtle(r#"_:b ex:p "[_:b, 42, _:b]"^^cdt:List ."#);
    assert_eq!(embedded_ids(&source, "p")[0], subject_of(&source, "p"));

    for format in round_trip_formats() {
        let bytes = serialize_dataset(&source, format.media_type(), SerializeGraph::Dataset)
            .unwrap_or_else(|e| panic!("{} must serialize: {e}", format.media_type()));
        let back = parse_dataset(&bytes, format.media_type(), None).unwrap_or_else(|e| {
            panic!(
                "{} output must re-parse: {e}\n{}",
                format.media_type(),
                String::from_utf8_lossy(&bytes)
            )
        });

        let media = format.media_type();
        let ids = embedded_ids(&back, "p");
        assert_eq!(ids.len(), 2, "{media}: both occurrences must survive");
        assert_eq!(
            ids[0], ids[1],
            "{media}: the two occurrences must still be one node"
        );
        assert_eq!(
            ids[0],
            subject_of(&back, "p"),
            "{media}: the embedded node must still be the subject"
        );

        // The emitted labels must be legal `BLANK_NODE_LABEL`s: the composite
        // grammar admits nothing else, so an illegal one would not re-parse.
        let (lexical, datatype) = composite_of(&back, "p");
        assert_eq!(datatype, LIST, "{media}: the datatype must survive");
        assert!(
            purrdf_cdt::parse_cdt_by_iri(&lexical, LIST).is_ok(),
            "{media}: the round-tripped lexical form must still parse: {lexical}"
        );

        // Two distinct nodes must not collapse into one on the way out.
        assert_eq!(
            distinct_blanks(&back),
            distinct_blanks(&source),
            "{media}: the blank-node count changed: {lexical}"
        );
    }
}

/// The same round trip for `cdt:Map`, whose lexical form carries quoted keys and
/// so exercises a different escaping path in the XML and JSON codecs.
#[test]
fn a_map_literal_with_a_blank_node_round_trips_through_every_format() {
    let source = turtle(r#"_:b ex:p "{ '1': _:b, '2': 42 }"^^cdt:Map ."#);
    for format in round_trip_formats() {
        let bytes = serialize_dataset(&source, format.media_type(), SerializeGraph::Dataset)
            .unwrap_or_else(|e| panic!("{} must serialize: {e}", format.media_type()));
        let back = parse_dataset(&bytes, format.media_type(), None)
            .unwrap_or_else(|e| panic!("{} output must re-parse: {e}", format.media_type()));
        let media = format.media_type();
        assert_eq!(
            embedded_ids(&back, "p")[0],
            subject_of(&back, "p"),
            "{media}: identity lost"
        );
        let (lexical, datatype) = composite_of(&back, "p");
        assert_eq!(datatype, MAP);
        assert!(
            purrdf_cdt::parse_cdt_by_iri(&lexical, MAP).is_ok(),
            "{media}: {lexical}"
        );
    }
}

/// Two distinct embedded blank nodes stay two after a round trip: a serializer
/// that emitted one label for both would silently merge them.
#[test]
fn two_embedded_blank_nodes_do_not_conflate_on_egress() {
    let source = turtle(r#"ex:s ex:p "[_:b1, 42, _:b2]"^^cdt:List ."#);
    for format in round_trip_formats() {
        let bytes = serialize_dataset(&source, format.media_type(), SerializeGraph::Dataset)
            .unwrap_or_else(|e| panic!("{} must serialize: {e}", format.media_type()));
        let back = parse_dataset(&bytes, format.media_type(), None)
            .unwrap_or_else(|e| panic!("{} output must re-parse: {e}", format.media_type()));
        let ids = embedded_ids(&back, "p");
        assert_eq!(ids.len(), 2);
        assert_ne!(
            ids[0],
            ids[1],
            "{}: two nodes conflated into one",
            format.media_type()
        );
    }
}

/// Every distinct blank node the dataset holds, counted from the IR — including
/// the ones that occur only inside a composite literal.
fn distinct_blanks(ds: &RdfDataset) -> usize {
    let mut set: BTreeSet<TermId> = BTreeSet::new();
    for quad in ds.quads() {
        for id in [quad.s, quad.p, quad.o].into_iter().chain(quad.g) {
            match ds.resolve(id) {
                TermRef::Blank { .. } => {
                    set.insert(id);
                }
                TermRef::Literal {
                    lexical, datatype, ..
                } => {
                    let TermRef::Iri(dt) = ds.resolve(datatype) else {
                        continue;
                    };
                    for (label, scope) in purrdf_core::cdt_blank::cdt_embedded_blanks(lexical, dt) {
                        if let Some(id) = ds.term_id_by_blank(&label, scope) {
                            set.insert(id);
                        }
                    }
                }
                TermRef::Iri(_) | TermRef::Triple { .. } => {}
            }
        }
    }
    set.len()
}

// ── Ingress refuses an ill-formed composite literal ─────────────────────────

/// A `cdt:List`-typed lexical form that does not parse refuses the WHOLE
/// document, even though the document is otherwise valid Turtle. It is never
/// admitted as an opaque literal and never panics.
#[test]
fn an_ill_formed_composite_literal_refuses_the_document() {
    let text = format!("{PREFIXES}ex:s ex:ok \"plain\" .\nex:s ex:p \"[_:b, 42\"^^cdt:List .\n");
    let err = parse_dataset(text.as_bytes(), "text/turtle", None)
        .expect_err("an unparseable composite literal must refuse the document");
    let rendered = format!("{err:?}");
    assert!(
        rendered.contains("cdt-literal-malformed"),
        "expected the composite refusal, got {rendered}"
    );
}

/// The refusal reaches every text codec, not just Turtle.
#[test]
fn the_refusal_reaches_the_line_and_xml_codecs() {
    let nt = concat!(
        "<http://example.org/s> <http://example.org/p> ",
        "\"{ '1': \"^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/Map> .\n"
    );
    assert!(
        parse_dataset(nt.as_bytes(), "application/n-triples", None).is_err(),
        "N-Triples must refuse an ill-formed composite literal"
    );

    let trix = concat!(
        "<TriX xmlns=\"http://www.w3.org/2004/03/trix/trix-1/\"><graph><triple>",
        "<uri>http://example.org/s</uri><uri>http://example.org/p</uri>",
        "<typedLiteral datatype=\"http://w3id.org/awslabs/neptune/SPARQL-CDTs/List\">",
        "[_:b, 42</typedLiteral></triple></graph></TriX>"
    );
    assert!(
        parse_dataset(trix.as_bytes(), "application/trix", None).is_err(),
        "TriX must refuse an ill-formed composite literal"
    );
}

/// A composite literal that nests past `purrdf-cdt`'s depth limit is refused
/// rather than looped on or truncated — the limit is a hard fail, and the walk
/// that enforces it is iterative, so a hostile document cannot abort the process
/// through stack exhaustion.
#[test]
fn an_over_deep_composite_literal_is_refused_not_truncated() {
    let depth = purrdf_cdt::MAX_NESTING_DEPTH + 4;
    let lexical: String = "[".repeat(depth) + "_:b" + &"]".repeat(depth);
    let text = format!("{PREFIXES}ex:s ex:p \"{lexical}\"^^cdt:List .\n");
    assert!(
        parse_dataset(text.as_bytes(), "text/turtle", None).is_err(),
        "an over-deep composite literal must be refused"
    );
}
