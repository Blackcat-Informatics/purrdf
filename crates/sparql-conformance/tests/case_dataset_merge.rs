// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! How a case's dataset is ASSEMBLED, and the two defects that assembly had.
//!
//! A case may name several source documents (`qt:data`, `qt:graphData`), and one
//! may name none at all and have its data CONSTRUCTED instead
//! (`qt:constructDataFile`). Both are graded here, because both used to be wrong
//! in a way no failing case pointed at:
//!
//! * **The merge was a text concatenation.** Each `qt:data` file was serialized
//!   to N-Quads, the texts were joined, and the join was re-parsed as ONE
//!   document — so every source's `_:b` collapsed onto a single blank node.
//!   Combining documents is an RDF 1.1 §4.1 *merge*: the sources are standardized
//!   apart first, and two files that both write `_:b` name two different nodes.
//!   `bnodes-turtle-15`…`-19` of the vendored SEP-0009 corpus state exactly this
//!   and all five failed.
//! * **`qt:constructDataFile` was not modeled at all.** The three
//!   `bnodes-export-*` cases loaded ZERO quads and failed as empty-result
//!   mismatches rather than as the unmodeled action they were.
//!
//! # Both directions
//!
//! Standardizing apart is a *separation*, so every separation case below is
//! paired with the agreement it must not break: within ONE document a bare `_:b`
//! and the `_:b` of a `cdt:List` / `cdt:Map` lexical form are still one node
//! (which is what `bnodes-turtle-01`…`-14` and `-21`…`-27` rest on), and distinct
//! labels stay distinct.
//!
//! The fixtures are the vendored corpus's own `.ttl` files — frozen and
//! digest-pinned by `scripts/check-corpus-frozen.py` — rather than copies, so
//! these tests and the corpus cannot drift apart.

use std::path::PathBuf;
use std::sync::Arc;

use purrdf_core::{RdfDataset, TermId, TermValue};
use purrdf_sparql_conformance::manifest::{self, SparqlTestCase};
use purrdf_sparql_conformance::run;

/// `vectors/sparql-cdt/bnodes/` — the vendored SEP-0009 blank-node group.
fn bnodes(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vectors/sparql-cdt/bnodes")
        .join(file)
}

/// The sentinel base the manifest loader derives for that directory. Passed
/// explicitly so these tests resolve file IRIs exactly as a real case does.
const BASE: &str = "http://purrdf.test/manifest/vectors/sparql-cdt/bnodes/";

/// Merge `files` as a case's `qt:data` documents.
fn merged(files: &[&str]) -> Arc<RdfDataset> {
    let data: Vec<PathBuf> = files.iter().map(|f| bnodes(f)).collect();
    run::build_dataset(BASE, &data, &[])
        .unwrap_or_else(|e| panic!("the vendored fixtures {files:?} must merge: {e}"))
}

/// The object of the quad whose predicate is `ex:{local}`.
fn object_of(ds: &RdfDataset, local: &str) -> TermId {
    let predicate = format!("http://example.org/{local}");
    for quad in ds.quads() {
        if matches!(ds.term_value(quad.p), TermValue::Iri(p) if p == predicate) {
            return quad.o;
        }
    }
    panic!("no quad on ex:{local}");
}

/// The subject of the quad whose predicate is `ex:{local}`.
fn subject_of(ds: &RdfDataset, local: &str) -> TermId {
    let predicate = format!("http://example.org/{local}");
    for quad in ds.quads() {
        if matches!(ds.term_value(quad.p), TermValue::Iri(p) if p == predicate) {
            return quad.s;
        }
    }
    panic!("no quad on ex:{local}");
}

/// The dataset term the FIRST blank node embedded in the composite literal object
/// of `ex:{local}` denotes.
///
/// Resolved the way any consumer resolves one — `cdt_embedded_blanks` reads the
/// `(label, scope)` pairs out of the stored lexical form and `term_id_by_blank`
/// looks each up — so this asserts against real interned identity, not against a
/// spelling.
fn embedded_blank(ds: &RdfDataset, local: &str) -> TermId {
    let TermValue::Literal {
        lexical_form,
        datatype,
        ..
    } = ds.term_value(object_of(ds, local))
    else {
        panic!("ex:{local} must carry a composite literal object");
    };
    let (label, scope) = purrdf_core::cdt_blank::cdt_embedded_blanks(&lexical_form, &datatype)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("ex:{local}'s literal embeds no blank node: {lexical_form}"));
    ds.term_id_by_blank(&label, scope).unwrap_or_else(|| {
        panic!(
            "the label {label} at {scope:?} embedded in ex:{local} must be a blank node the \
             merged dataset actually holds"
        )
    })
}

// ── The merge separates documents ───────────────────────────────────────────

/// `bnodes-turtle-15`: two documents each writing `_:b` inside a `cdt:List`
/// literal name two nodes.
#[test]
fn two_documents_that_embed_one_label_are_two_nodes() {
    let ds = merged(&["bnodes-turtle-15a.ttl", "bnodes-turtle-15b.ttl"]);
    assert_ne!(
        embedded_blank(&ds, "p1"),
        embedded_blank(&ds, "p2"),
        "each qt:data file is its own blank-node document; the text concatenation \
         this merge replaced collapsed both onto one node"
    );
}

/// `bnodes-turtle-17`/`-18`: the separation holds across the literal boundary
/// too — an EMBEDDED `_:b` in one document is not the BARE `_:b` of another.
///
/// This is the case a purely textual label rewrite would miss: a rewriter that
/// renamed bare terms but not the labels inside composite lexical forms would
/// leave these two equal (or, worse, leave the embedded label dangling).
#[test]
fn an_embedded_label_and_a_bare_label_in_two_documents_are_two_nodes() {
    let list = merged(&["bnodes-turtle-17a.ttl", "bnodes-turtle-17b.ttl"]);
    assert_ne!(embedded_blank(&list, "p1"), object_of(&list, "p2"));

    let map = merged(&["bnodes-turtle-18a.ttl", "bnodes-turtle-18b.ttl"]);
    assert_ne!(embedded_blank(&map, "p1"), object_of(&map, "p2"));
}

/// `bnodes-turtle-19`: a `cdt:List` in one document and a `cdt:Map` in another,
/// both writing `_:b` — the datatype is irrelevant, the document is what scopes.
#[test]
fn a_list_and_a_map_in_two_documents_are_two_nodes() {
    let ds = merged(&["bnodes-turtle-19a.ttl", "bnodes-turtle-19b.ttl"]);
    assert_ne!(embedded_blank(&ds, "p1"), embedded_blank(&ds, "p2"));
}

// ── …and keeps each document's own agreements ───────────────────────────────

/// `bnodes-turtle-05`: within ONE document a bare `_:b` written as the subject and
/// the `_:b` embedded in that quad's composite literal are the SAME node.
///
/// The neighbouring valid case for every separation above: one scope per
/// document, applied to bare terms and embedded labels alike, is what makes this
/// hold while the cross-document cases do not.
#[test]
fn one_document_binds_its_bare_and_embedded_labels_to_one_node() {
    let ds = merged(&["bnodes-turtle-05.ttl"]);
    assert_eq!(subject_of(&ds, "p"), embedded_blank(&ds, "p"));
}

/// `bnodes-turtle-13`: one label across TWO literals of one document is one node.
#[test]
fn one_document_keeps_one_label_across_two_literals_as_one_node() {
    let ds = merged(&["bnodes-turtle-13.ttl"]);
    assert_eq!(embedded_blank(&ds, "p1"), embedded_blank(&ds, "p2"));
}

/// `bnodes-turtle-14`: distinct labels in one document stay distinct — the
/// per-document scope must be injective on labels, not collapse them.
#[test]
fn one_document_keeps_distinct_labels_distinct() {
    let ds = merged(&["bnodes-turtle-14.ttl"]);
    assert_ne!(embedded_blank(&ds, "p1"), embedded_blank(&ds, "p2"));
}

/// The merge is a union of CONTENT, not only of blank nodes: both files' quads
/// arrive. A separation that dropped a source would satisfy every `assert_ne!`
/// above vacuously.
#[test]
fn the_merge_carries_every_sources_quads() {
    assert_eq!(merged(&["bnodes-turtle-15a.ttl"]).quad_count(), 1);
    assert_eq!(
        merged(&["bnodes-turtle-15a.ttl", "bnodes-turtle-15b.ttl"]).quad_count(),
        2
    );
}

// ── qt:constructDataFile ────────────────────────────────────────────────────

/// Every case the vendored `bnodes` manifest declares, by local name.
fn bnodes_case(local: &str) -> SparqlTestCase {
    let cases = manifest::load(&bnodes("manifest.ttl")).expect("the bnodes manifest must load");
    cases
        .into_iter()
        .find(|c| c.name == local)
        .unwrap_or_else(|| panic!("the vendored bnodes manifest declares no case {local}"))
}

/// The action shape is read: query, media type, and — the real hazard — the
/// case's OWN `qt:query` is still the outer one, not the CONSTRUCT query nested
/// inside the `qt:constructDataFile` node.
#[test]
fn the_construct_data_file_action_is_modeled() {
    for (local, format) in [
        ("bnodes-export-turtle-01", "text/turtle"),
        ("bnodes-export-ntriples-01", "application/n-triples"),
        ("bnodes-export-rdfxml-01", "application/rdf+xml"),
    ] {
        let case = bnodes_case(local);
        let construct = case
            .construct_data
            .as_ref()
            .unwrap_or_else(|| panic!("{local} declares qt:constructDataFile"));
        assert_eq!(construct.format, format, "{local}: media type");
        assert_eq!(
            construct.query.file_name().and_then(|n| n.to_str()),
            Some("bnodes-export-rdf-01-construct.rq"),
            "{local}: the CONSTRUCT query"
        );
        assert_eq!(
            case.query.file_name().and_then(|n| n.to_str()),
            Some("bnodes-export-rdf-01.rq"),
            "{local}: the case's own qt:query must stay the outer one — the nested \
             constructDataFile node carries a qt:query too, and binding that one \
             would silently run the CONSTRUCT as the test query"
        );
    }
}

/// **The round trip the action exists to grade.** The CONSTRUCT result is written
/// in the declared syntax and read back, and the blank node that occurs BOTH as
/// the subject and inside the `cdt:List` literal must still be ONE node
/// afterwards, in all three formats.
///
/// A serializer that spelled the bare occurrence and the embedded occurrence with
/// different identifiers would fail here — that is a real writer defect, and this
/// is where it would surface.
#[test]
fn a_constructed_data_file_round_trips_a_blank_through_every_declared_syntax() {
    for local in [
        "bnodes-export-turtle-01",
        "bnodes-export-ntriples-01",
        "bnodes-export-rdfxml-01",
    ] {
        let case = bnodes_case(local);
        let ds = run::load_dataset(&case)
            .unwrap_or_else(|e| panic!("{local}: the constructed data must load: {e}"));
        assert_eq!(
            ds.quad_count(),
            1,
            "{local}: the CONSTRUCT yields exactly one quad; zero would make the \
             case's ASK vacuously false and its failure indistinguishable from a \
             wrong answer"
        );
        assert_eq!(
            subject_of(&ds, "list"),
            embedded_blank(&ds, "list"),
            "{local}: the bare and the CDT-embedded occurrence must denote one node \
             after the serialize/parse round trip"
        );
    }
}

/// `bnodes-export-service-01` shares the `bnodes-export-*` comment but NOT the
/// action shape: it declares `qt:serviceData`, so it is graded through the
/// federation lane and the constructed-data lane must not claim it.
///
/// Pinned because the two are easy to conflate by name, and because a case that
/// silently loaded no data would still answer its ASK from the `SERVICE` body and
/// look green either way.
#[test]
fn the_service_export_case_is_federated_not_constructed() {
    let case = bnodes_case("bnodes-export-service-01");
    assert!(
        case.construct_data.is_none(),
        "it declares no qt:constructDataFile"
    );
    assert_eq!(
        case.service_data.len(),
        1,
        "it declares exactly one qt:serviceData endpoint"
    );
    assert_eq!(case.service_data[0].0, "http://example.org/sparql");
}

/// The neighbouring valid case for the loader change: an ORDINARY case is
/// untouched — no constructed data, and its `qt:data` and `qt:query` still bind.
#[test]
fn an_ordinary_case_declares_no_constructed_data() {
    let case = bnodes_case("bnodes-turtle-01");
    assert!(case.construct_data.is_none());
    assert_eq!(
        case.query.file_name().and_then(|n| n.to_str()),
        Some("bnodes-turtle-01.rq")
    );
    assert_eq!(case.data.len(), 1);
    assert_eq!(
        run::load_dataset(&case)
            .expect("an ordinary case still loads")
            .quad_count(),
        1
    );
}
