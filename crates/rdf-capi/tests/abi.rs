// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests that exercise the `extern "C"` surface directly (the crate
//! exposes an `rlib` so the symbols link without dlopen). These are the primary
//! ABI suite — they call the exact C entry points with C-shaped inputs and
//! assert on status codes, out-params, and free ordering.

use std::ffi::CString;

use purrdf::buffer::{purrdf_buffer_data, purrdf_buffer_free, PurrdfBuffer};
use purrdf::cursor::{
    purrdf_cursor_free, purrdf_cursor_next, purrdf_quads_for_pattern, PurrdfCursor,
};
use purrdf::error::{purrdf_error_code, purrdf_error_free, purrdf_error_message, PurrdfError};
use purrdf::graph::{
    purrdf_graph_free, purrdf_graph_freeze, purrdf_graph_from_dataset, purrdf_graph_insert,
    purrdf_graph_remove, PurrdfGraph,
};
use purrdf::gts::{purrdf_from_gts, purrdf_to_gts};
use purrdf::handles::{
    purrdf_dataset_free, purrdf_dataset_quad_count, purrdf_dataset_term_count, PurrdfDataset,
};
use purrdf::parse::purrdf_parse;
use purrdf::query::{purrdf_query, purrdf_query_json};
use purrdf::rowcursor::{
    purrdf_rowcursor_free, purrdf_rowcursor_next, purrdf_rowcursor_term,
    purrdf_rowcursor_variable_count, purrdf_rowcursor_variable_name, PurrdfRowCursor,
};
use purrdf::serialize::purrdf_serialize;
use purrdf::status::{PurrdfAbiVersion, PurrdfCapabilities, PurrdfStatus};
use purrdf::term::{
    purrdf_term_to_ntriples, PurrdfGraphMatch, PurrdfGraphMatchKind, PurrdfStr, PurrdfTermKind,
    PurrdfTermView,
};
use purrdf::version::{
    purrdf_abi_version, purrdf_capabilities, PURRDF_ABI_MAJOR, PURRDF_ABI_MINOR, PURRDF_ABI_PATCH,
};

/// A zeroed output term view the cursor fills.
fn out_view() -> PurrdfTermView {
    iri_view("")
}

/// An input IRI term view borrowing `s` (which the caller must keep alive).
fn iri_view(s: &str) -> PurrdfTermView {
    PurrdfTermView {
        kind: PurrdfTermKind::Iri as i32,
        lexical: PurrdfStr {
            ptr: s.as_ptr(),
            len: s.len(),
        },
        datatype: PurrdfStr {
            ptr: std::ptr::null(),
            len: 0,
        },
        language: PurrdfStr {
            ptr: std::ptr::null(),
            len: 0,
        },
        direction: purrdf::term::PurrdfDirection::None as i32,
        blank_scope: 0,
        term_id: 0,
    }
}

/// "Match any graph".
fn any_graph() -> PurrdfGraphMatch {
    PurrdfGraphMatch {
        kind: PurrdfGraphMatchKind::Any as i32,
        name: out_view(),
    }
}

unsafe fn view_str(view: &PurrdfTermView) -> String {
    unsafe {
        if view.lexical.len == 0 {
            return String::new();
        }
        let bytes = std::slice::from_raw_parts(view.lexical.ptr, view.lexical.len);
        String::from_utf8_lossy(bytes).into_owned()
    }
}

/// Drain a cursor, returning each row's (subject, predicate, object) lexical and
/// object kind as i32.
unsafe fn drain(cursor: *mut PurrdfCursor) -> Vec<(String, String, String, i32)> {
    unsafe {
        let mut rows = Vec::new();
        loop {
            let (mut s, mut p, mut o, mut g) = (out_view(), out_view(), out_view(), out_view());
            let mut has_graph: u8 = 0;
            let rc = purrdf_cursor_next(
                cursor,
                &raw mut s,
                &raw mut p,
                &raw mut o,
                &raw mut g,
                &raw mut has_graph,
            );
            if rc == PurrdfStatus::CursorExhausted as i32 {
                break;
            }
            assert_eq!(rc, PurrdfStatus::Ok as i32);
            rows.push((view_str(&s), view_str(&p), view_str(&o), o.kind));
        }
        rows
    }
}

/// Parse a Turtle/N-Triples snippet, returning the owned dataset handle.
unsafe fn parse(media: &str, doc: &str) -> *mut PurrdfDataset {
    unsafe {
        let media = CString::new(media).unwrap();
        let mut dataset: *mut PurrdfDataset = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        let status = purrdf_parse(
            doc.as_ptr(),
            doc.len(),
            media.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            &raw mut dataset,
            &raw mut error,
        );
        assert_eq!(status, PurrdfStatus::Ok as i32, "parse should succeed");
        assert!(error.is_null());
        assert!(!dataset.is_null());
        dataset
    }
}

unsafe fn buffer_bytes(buf: *const PurrdfBuffer) -> Vec<u8> {
    unsafe {
        let mut ptr: *const u8 = std::ptr::null();
        let mut len: usize = 0;
        assert_eq!(
            purrdf_buffer_data(buf, &raw mut ptr, &raw mut len),
            PurrdfStatus::Ok as i32
        );
        std::slice::from_raw_parts(ptr, len).to_vec()
    }
}

#[test]
fn abi_version_is_beta_0_1_0() {
    let mut version = PurrdfAbiVersion {
        major: 9,
        minor: 9,
        patch: 9,
    };
    let status = unsafe { purrdf_abi_version(&raw mut version) };
    assert_eq!(status, PurrdfStatus::Ok as i32);
    assert_eq!(version.major, PURRDF_ABI_MAJOR);
    assert_eq!(version.minor, PURRDF_ABI_MINOR);
    assert_eq!(version.patch, PURRDF_ABI_PATCH);
    assert_eq!((version.major, version.minor, version.patch), (0, 1, 0));
}

#[test]
fn abi_version_null_out_is_handled() {
    let status = unsafe { purrdf_abi_version(std::ptr::null_mut()) };
    assert_eq!(status, PurrdfStatus::NullPointer as i32);
}

#[test]
fn status_discriminants_are_frozen() {
    // The ABI is SemVer-frozen: these numbers must never change.
    assert_eq!(PurrdfStatus::Ok as i32, 0);
    assert_eq!(PurrdfStatus::NullPointer as i32, 1);
    assert_eq!(PurrdfStatus::InvalidUtf8 as i32, 2);
    assert_eq!(PurrdfStatus::CursorExhausted as i32, 9);
    assert_eq!(PurrdfStatus::GtsError as i32, 10);
    assert_eq!(PurrdfStatus::Panic as i32, 100);
}

#[test]
fn parse_counts_quads_and_terms() {
    unsafe {
        let dataset = parse("text/turtle", "<http://a> <http://b> <http://c> .");
        let mut quads: usize = 0;
        let mut terms: usize = 0;
        assert_eq!(
            purrdf_dataset_quad_count(dataset, &raw mut quads),
            PurrdfStatus::Ok as i32
        );
        assert_eq!(
            purrdf_dataset_term_count(dataset, &raw mut terms),
            PurrdfStatus::Ok as i32
        );
        assert_eq!(quads, 1);
        assert_eq!(terms, 3);
        purrdf_dataset_free(dataset);
    }
}

#[test]
fn serialize_round_trips_through_ntriples() {
    unsafe {
        let dataset = parse("text/turtle", "<http://a> <http://b> <http://c> .");
        let media = CString::new("application/n-triples").unwrap();
        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut dropped: usize = 999;
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        let status = purrdf_serialize(
            dataset,
            media.as_ptr(),
            std::ptr::null(),
            &raw mut buffer,
            &raw mut dropped,
            &raw mut error,
        );
        assert_eq!(status, PurrdfStatus::Ok as i32);
        assert!(error.is_null());
        // N-Triples is star-capable: no statement rows dropped.
        assert_eq!(dropped, 0);
        let bytes = buffer_bytes(buffer);
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("<http://a>"));
        assert!(text.contains("<http://c>"));

        // Re-parse the serialized output; it must yield the same single quad.
        let reparsed = parse("application/n-triples", &text);
        let mut quads: usize = 0;
        purrdf_dataset_quad_count(reparsed, &raw mut quads);
        assert_eq!(quads, 1);

        purrdf_buffer_free(buffer);
        purrdf_dataset_free(reparsed);
        purrdf_dataset_free(dataset);
    }
}

#[test]
fn parse_rejects_malformed_turtle_without_aborting() {
    unsafe {
        let media = CString::new("text/turtle").unwrap();
        let doc = "<http://a> <http://b> @@@ not-valid";
        let mut dataset: *mut PurrdfDataset = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        let status = purrdf_parse(
            doc.as_ptr(),
            doc.len(),
            media.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            &raw mut dataset,
            &raw mut error,
        );
        assert_eq!(status, PurrdfStatus::ParseError as i32);
        assert!(dataset.is_null());
        assert!(!error.is_null());
        assert_eq!(purrdf_error_code(error), PurrdfStatus::ParseError as i32);
        let msg = std::ffi::CStr::from_ptr(purrdf_error_message(error));
        assert!(!msg.to_bytes().is_empty());
        purrdf_error_free(error);
    }
}

#[test]
fn serialize_rejects_unknown_media_type() {
    unsafe {
        let dataset = parse("text/turtle", "<http://a> <http://b> <http://c> .");
        let media = CString::new("application/x-made-up").unwrap();
        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        let status = purrdf_serialize(
            dataset,
            media.as_ptr(),
            std::ptr::null(),
            &raw mut buffer,
            std::ptr::null_mut(),
            &raw mut error,
        );
        assert_eq!(status, PurrdfStatus::UnsupportedFormat as i32);
        assert!(buffer.is_null());
        assert!(!error.is_null());
        purrdf_error_free(error);
        purrdf_dataset_free(dataset);
    }
}

const THREE_QUADS: &str = concat!(
    "<http://s1> <http://p> <http://o1> .\n",
    "<http://s1> <http://p> <http://o2> .\n",
    "<http://s2> <http://p> <http://o3> .\n",
);

#[test]
fn cursor_iterates_all_quads() {
    unsafe {
        let dataset = parse("application/n-triples", THREE_QUADS);
        let graph = any_graph();
        let mut cursor: *mut PurrdfCursor = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        let status = purrdf_quads_for_pattern(
            dataset,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            &raw const graph,
            &raw mut cursor,
            &raw mut error,
        );
        assert_eq!(status, PurrdfStatus::Ok as i32);
        assert!(error.is_null());
        let rows = drain(cursor);
        assert_eq!(rows.len(), 3);
        assert!(rows.iter().all(|(_, p, _, _)| p == "http://p"));
        assert!(rows
            .iter()
            .any(|(s, _, o, _)| s == "http://s1" && o == "http://o1"));
        purrdf_cursor_free(cursor);
        purrdf_dataset_free(dataset);
    }
}

#[test]
fn cursor_filters_by_subject() {
    unsafe {
        let dataset = parse("application/n-triples", THREE_QUADS);
        let subject = String::from("http://s1");
        let s_view = iri_view(&subject);
        let graph = any_graph();
        let mut cursor: *mut PurrdfCursor = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        purrdf_quads_for_pattern(
            dataset,
            &raw const s_view,
            std::ptr::null(),
            std::ptr::null(),
            &raw const graph,
            &raw mut cursor,
            &raw mut error,
        );
        let rows = drain(cursor);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|(s, _, _, _)| s == "http://s1"));
        purrdf_cursor_free(cursor);
        purrdf_dataset_free(dataset);
    }
}

#[test]
fn cursor_for_absent_term_is_empty_not_error() {
    unsafe {
        let dataset = parse("application/n-triples", THREE_QUADS);
        let subject = String::from("http://not-present");
        let s_view = iri_view(&subject);
        let graph = any_graph();
        let mut cursor: *mut PurrdfCursor = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        let status = purrdf_quads_for_pattern(
            dataset,
            &raw const s_view,
            std::ptr::null(),
            std::ptr::null(),
            &raw const graph,
            &raw mut cursor,
            &raw mut error,
        );
        assert_eq!(status, PurrdfStatus::Ok as i32);
        assert!(error.is_null());
        assert_eq!(drain(cursor).len(), 0);
        purrdf_cursor_free(cursor);
        purrdf_dataset_free(dataset);
    }
}

#[test]
fn cursor_survives_dataset_free() {
    unsafe {
        let dataset = parse("application/n-triples", THREE_QUADS);
        let graph = any_graph();
        let mut cursor: *mut PurrdfCursor = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        purrdf_quads_for_pattern(
            dataset,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            &raw const graph,
            &raw mut cursor,
            &raw mut error,
        );
        // Free the dataset BEFORE iterating — the cursor's Arc pin keeps the arena alive.
        purrdf_dataset_free(dataset);
        let rows = drain(cursor);
        assert_eq!(rows.len(), 3);
        assert!(rows.iter().any(|(s, _, _, _)| s == "http://s2"));
        purrdf_cursor_free(cursor);
    }
}

#[test]
fn quoted_triple_object_renders_to_ntriples() {
    unsafe {
        // A quoted triple as an ordinary object (NOT an `rdf:reifies` statement,
        // which the native codec folds into the reifier layer rather than the
        // base-quad set) — so it iterates as a base quad with a Triple object.
        let doc = concat!(
            "<https://e/a> <https://e/b> ",
            "<<( <https://e/s> <https://e/p> <https://e/o> )>> .\n",
        );
        let dataset = parse("application/n-triples", doc);
        let graph = any_graph();
        let mut cursor: *mut PurrdfCursor = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        purrdf_quads_for_pattern(
            dataset,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            &raw const graph,
            &raw mut cursor,
            &raw mut error,
        );

        let (mut s, mut p, mut o, mut g) = (out_view(), out_view(), out_view(), out_view());
        let mut has_graph: u8 = 0;
        let rc = purrdf_cursor_next(
            cursor,
            &raw mut s,
            &raw mut p,
            &raw mut o,
            &raw mut g,
            &raw mut has_graph,
        );
        assert_eq!(rc, PurrdfStatus::Ok as i32);
        // The object is a quoted triple: kind Triple, empty lexical, non-zero id.
        assert_eq!(o.kind, PurrdfTermKind::Triple as i32);
        assert_eq!(o.lexical.len, 0);
        assert_ne!(o.term_id, 0);

        // Materialize it via the N-Triples convenience path.
        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut term_error: *mut PurrdfError = std::ptr::null_mut();
        let status =
            purrdf_term_to_ntriples(dataset, &raw const o, &raw mut buffer, &raw mut term_error);
        assert_eq!(status, PurrdfStatus::Ok as i32);
        assert!(term_error.is_null());
        let token = String::from_utf8(buffer_bytes(buffer)).unwrap();
        assert!(token.contains("<https://e/s>"), "got: {token}");
        assert!(token.contains("<https://e/o>"), "got: {token}");

        purrdf_buffer_free(buffer);
        purrdf_cursor_free(cursor);
        purrdf_dataset_free(dataset);
    }
}

unsafe fn quad_count(dataset: *const PurrdfDataset) -> usize {
    unsafe {
        let mut count: usize = 0;
        assert_eq!(
            purrdf_dataset_quad_count(dataset, &raw mut count),
            PurrdfStatus::Ok as i32
        );
        count
    }
}

unsafe fn graph_of(doc: &str) -> *mut PurrdfGraph {
    unsafe {
        let dataset = parse("application/n-triples", doc);
        let mut graph: *mut PurrdfGraph = std::ptr::null_mut();
        assert_eq!(
            purrdf_graph_from_dataset(dataset, &raw mut graph),
            PurrdfStatus::Ok as i32
        );
        purrdf_dataset_free(dataset);
        graph
    }
}

unsafe fn insert(graph: *mut PurrdfGraph, s: &str, p: &str, o: &str) -> u8 {
    unsafe {
        let (sv, pv, ov) = (iri_view(s), iri_view(p), iri_view(o));
        let mut changed: u8 = 0;
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        let status = purrdf_graph_insert(
            graph,
            &raw const sv,
            &raw const pv,
            &raw const ov,
            std::ptr::null(),
            &raw mut changed,
            &raw mut error,
        );
        assert_eq!(status, PurrdfStatus::Ok as i32);
        assert!(error.is_null());
        changed
    }
}

unsafe fn remove(graph: *mut PurrdfGraph, s: &str, p: &str, o: &str) -> u8 {
    unsafe {
        let (sv, pv, ov) = (iri_view(s), iri_view(p), iri_view(o));
        let mut changed: u8 = 0;
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        let status = purrdf_graph_remove(
            graph,
            &raw const sv,
            &raw const pv,
            &raw const ov,
            std::ptr::null(),
            &raw mut changed,
            &raw mut error,
        );
        assert_eq!(status, PurrdfStatus::Ok as i32);
        assert!(error.is_null());
        changed
    }
}

unsafe fn freeze(graph: *const PurrdfGraph) -> *mut PurrdfDataset {
    unsafe {
        let mut frozen: *mut PurrdfDataset = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        assert_eq!(
            purrdf_graph_freeze(graph, &raw mut frozen, &raw mut error),
            PurrdfStatus::Ok as i32
        );
        assert!(error.is_null());
        frozen
    }
}

#[test]
fn graph_insert_grows_the_frozen_count() {
    unsafe {
        let graph = graph_of(THREE_QUADS);
        assert_eq!(insert(graph, "http://s3", "http://p", "http://o4"), 1);
        // Re-inserting the same quad is a no-op.
        assert_eq!(insert(graph, "http://s3", "http://p", "http://o4"), 0);
        let frozen = freeze(graph);
        assert_eq!(quad_count(frozen), 4);
        purrdf_dataset_free(frozen);
        purrdf_graph_free(graph);
    }
}

#[test]
fn graph_remove_base_quad_shrinks_the_frozen_count() {
    unsafe {
        let graph = graph_of(THREE_QUADS);
        assert_eq!(remove(graph, "http://s1", "http://p", "http://o1"), 1);
        // Removing an absent quad is a no-op.
        assert_eq!(remove(graph, "http://s1", "http://p", "http://o1"), 0);
        let frozen = freeze(graph);
        assert_eq!(quad_count(frozen), 2);
        purrdf_dataset_free(frozen);
        purrdf_graph_free(graph);
    }
}

#[test]
fn graph_reinsert_unsuppresses_a_removed_base_quad() {
    unsafe {
        let graph = graph_of(THREE_QUADS);
        // Remove a base quad (suppresses it), then re-insert it (un-suppresses).
        assert_eq!(remove(graph, "http://s2", "http://p", "http://o3"), 1);
        assert_eq!(insert(graph, "http://s2", "http://p", "http://o3"), 1);
        let frozen = freeze(graph);
        assert_eq!(quad_count(frozen), 3);
        purrdf_dataset_free(frozen);
        purrdf_graph_free(graph);
    }
}

unsafe fn run_select(dataset: *const PurrdfDataset, query: &str) -> *mut PurrdfRowCursor {
    unsafe {
        let cq = CString::new(query).unwrap();
        let mut kind: i32 = -1;
        let mut rows: *mut PurrdfRowCursor = std::ptr::null_mut();
        let mut graph: *mut PurrdfDataset = std::ptr::null_mut();
        let mut boolean: u8 = 0;
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        let status = purrdf_query(
            dataset,
            cq.as_ptr(),
            std::ptr::null(),
            &raw mut kind,
            &raw mut rows,
            &raw mut graph,
            &raw mut boolean,
            &raw mut error,
        );
        assert_eq!(status, PurrdfStatus::Ok as i32);
        assert!(error.is_null());
        assert_eq!(kind, 0, "expected a SELECT (Solutions) result");
        rows
    }
}

#[test]
fn select_lists_subjects() {
    unsafe {
        let dataset = parse("application/n-triples", THREE_QUADS);
        let rows = run_select(dataset, "SELECT ?s WHERE { ?s ?p ?o }");

        let mut var_count: usize = 0;
        assert_eq!(
            purrdf_rowcursor_variable_count(rows, &raw mut var_count),
            PurrdfStatus::Ok as i32
        );
        assert_eq!(var_count, 1);
        let mut name_ptr: *const std::os::raw::c_char = std::ptr::null();
        assert_eq!(
            purrdf_rowcursor_variable_name(rows, 0, &raw mut name_ptr),
            PurrdfStatus::Ok as i32
        );
        assert_eq!(std::ffi::CStr::from_ptr(name_ptr).to_str().unwrap(), "s");

        let mut subjects = Vec::new();
        while purrdf_rowcursor_next(rows) == PurrdfStatus::Ok as i32 {
            let mut view = out_view();
            let mut bound: u8 = 0;
            assert_eq!(
                purrdf_rowcursor_term(rows, 0, &raw mut view, &raw mut bound),
                PurrdfStatus::Ok as i32
            );
            assert_eq!(bound, 1);
            subjects.push(view_str(&view));
        }
        subjects.sort();
        subjects.dedup();
        assert_eq!(
            subjects,
            vec!["http://s1".to_string(), "http://s2".to_string()]
        );

        purrdf_rowcursor_free(rows);
        purrdf_dataset_free(dataset);
    }
}

#[test]
fn ask_returns_boolean() {
    unsafe {
        let dataset = parse("application/n-triples", THREE_QUADS);
        let cq = CString::new("ASK { ?s ?p ?o }").unwrap();
        let mut kind: i32 = -1;
        let mut rows: *mut PurrdfRowCursor = std::ptr::null_mut();
        let mut graph: *mut PurrdfDataset = std::ptr::null_mut();
        let mut boolean: u8 = 9;
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        let status = purrdf_query(
            dataset,
            cq.as_ptr(),
            std::ptr::null(),
            &raw mut kind,
            &raw mut rows,
            &raw mut graph,
            &raw mut boolean,
            &raw mut error,
        );
        assert_eq!(status, PurrdfStatus::Ok as i32);
        assert_eq!(kind, 2);
        assert_eq!(boolean, 1);
        purrdf_dataset_free(dataset);
    }
}

#[test]
fn construct_returns_graph() {
    unsafe {
        let dataset = parse("application/n-triples", THREE_QUADS);
        let cq = CString::new("CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }").unwrap();
        let mut kind: i32 = -1;
        let mut rows: *mut PurrdfRowCursor = std::ptr::null_mut();
        let mut graph: *mut PurrdfDataset = std::ptr::null_mut();
        let mut boolean: u8 = 0;
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        let status = purrdf_query(
            dataset,
            cq.as_ptr(),
            std::ptr::null(),
            &raw mut kind,
            &raw mut rows,
            &raw mut graph,
            &raw mut boolean,
            &raw mut error,
        );
        assert_eq!(status, PurrdfStatus::Ok as i32);
        assert_eq!(kind, 1);
        assert!(!graph.is_null());
        assert_eq!(quad_count(graph), 3);
        purrdf_dataset_free(graph);
        purrdf_dataset_free(dataset);
    }
}

#[test]
fn query_json_has_sparql_results_shape() {
    unsafe {
        let dataset = parse("application/n-triples", THREE_QUADS);
        let cq = CString::new("SELECT ?s ?o WHERE { ?s ?p ?o }").unwrap();
        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        let status = purrdf_query_json(
            dataset,
            cq.as_ptr(),
            std::ptr::null(),
            &raw mut buffer,
            &raw mut error,
        );
        assert_eq!(status, PurrdfStatus::Ok as i32);
        assert!(error.is_null());
        let json = String::from_utf8(buffer_bytes(buffer)).unwrap();
        assert!(json.contains("\"head\""), "got: {json}");
        assert!(json.contains("\"vars\""), "got: {json}");
        assert!(json.contains("\"bindings\""), "got: {json}");
        assert!(json.contains("\"type\":\"uri\""), "got: {json}");
        assert!(json.contains("http://s1"), "got: {json}");
        purrdf_buffer_free(buffer);
        purrdf_dataset_free(dataset);
    }
}

#[test]
fn rowcursor_reports_unbound_optional() {
    unsafe {
        let dataset = parse("application/n-triples", THREE_QUADS);
        let rows = run_select(
            dataset,
            "SELECT ?s ?missing WHERE { ?s ?p ?o OPTIONAL { ?s <http://never> ?missing } }",
        );
        // Column 1 (?missing) is unbound in every row.
        let mut saw_unbound = false;
        while purrdf_rowcursor_next(rows) == PurrdfStatus::Ok as i32 {
            let mut view = out_view();
            let mut bound: u8 = 1;
            assert_eq!(
                purrdf_rowcursor_term(rows, 1, &raw mut view, &raw mut bound),
                PurrdfStatus::Ok as i32
            );
            if bound == 0 {
                saw_unbound = true;
            }
        }
        assert!(saw_unbound, "expected at least one unbound ?missing");
        purrdf_rowcursor_free(rows);
        purrdf_dataset_free(dataset);
    }
}

unsafe fn to_gts(dataset: *const PurrdfDataset) -> Vec<u8> {
    unsafe {
        let profile = CString::new("dist").unwrap();
        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        let status = purrdf_to_gts(dataset, profile.as_ptr(), &raw mut buffer, &raw mut error);
        assert_eq!(status, PurrdfStatus::Ok as i32);
        assert!(error.is_null());
        let bytes = buffer_bytes(buffer);
        purrdf_buffer_free(buffer);
        bytes
    }
}

unsafe fn from_gts(bytes: &[u8]) -> *mut PurrdfDataset {
    unsafe {
        let mut dataset: *mut PurrdfDataset = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        let status = purrdf_from_gts(
            bytes.as_ptr(),
            bytes.len(),
            &raw mut dataset,
            &raw mut error,
        );
        assert_eq!(status, PurrdfStatus::Ok as i32);
        assert!(error.is_null());
        assert!(!dataset.is_null());
        dataset
    }
}

#[test]
fn gts_round_trips_a_plain_graph() {
    unsafe {
        let dataset = parse("application/n-triples", THREE_QUADS);
        let gts = to_gts(dataset);
        assert!(!gts.is_empty());
        let restored = from_gts(&gts);
        assert_eq!(quad_count(restored), 3);
        purrdf_dataset_free(restored);
        purrdf_dataset_free(dataset);
    }
}

/// The C-ABI's `purrdf_to_gts` → `purrdf_from_gts` round-trip preserves the
/// RDF-1.2 star layer (quoted triples + reifier bindings) losslessly. The C-ABI
/// calls the canonical kernel path (`to_gts` → `read_graph` → `import_gts_graph`);
/// the earlier `gts-missing-reifier-binding` gap (formerly tracked in #1032) was
/// closed by the native text-codec work (#909), so a reifier-bound quoted triple
/// now survives intact rather than failing with a `GtsError`.
#[test]
fn gts_star_roundtrip_preserves_the_statement_layer() {
    unsafe {
        let doc = concat!(
            "<https://e/r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ",
            "<<( <https://e/s> <https://e/p> <https://e/o> )>> .\n",
        );
        let dataset = parse("application/n-triples", doc);
        let gts = to_gts(dataset);
        assert!(!gts.is_empty());

        let mut restored: *mut PurrdfDataset = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        let status = purrdf_from_gts(gts.as_ptr(), gts.len(), &raw mut restored, &raw mut error);

        // The star round-trip now SUCCEEDS — no GtsError, a live restored handle.
        assert_eq!(
            status,
            PurrdfStatus::Ok as i32,
            "star GTS round-trip should succeed now that the reifier-binding gap is closed"
        );
        assert!(error.is_null());
        assert!(!restored.is_null());

        // The star layer genuinely survived (not silently dropped): the restored
        // dataset still carries the quoted triple and the reifier binding.
        let mut caps = PurrdfCapabilities {
            named_graphs: 0,
            quoted_triples: 0,
            reifiers: 0,
            annotations: 0,
            source_locations: 0,
            loss_records: 0,
            lookaside: 0,
        };
        assert_eq!(
            purrdf_capabilities(restored, &raw mut caps),
            PurrdfStatus::Ok as i32
        );
        assert_eq!(caps.quoted_triples, 1, "the quoted triple must survive");
        assert_eq!(caps.reifiers, 1, "the reifier binding must survive");

        purrdf_dataset_free(restored);
        purrdf_dataset_free(dataset);
    }
}

#[test]
fn capabilities_reflect_the_dataset() {
    unsafe {
        let mut caps = PurrdfCapabilities {
            named_graphs: 9,
            quoted_triples: 9,
            reifiers: 9,
            annotations: 9,
            source_locations: 9,
            loss_records: 9,
            lookaside: 9,
        };

        // A plain graph has no star features.
        let plain = parse("application/n-triples", THREE_QUADS);
        assert_eq!(
            purrdf_capabilities(plain, &raw mut caps),
            PurrdfStatus::Ok as i32
        );
        assert_eq!(caps.quoted_triples, 0);
        purrdf_dataset_free(plain);

        // An in-memory quoted-triple object sets the star capability (this path
        // does NOT depend on the GTS round-trip gap).
        let star = parse(
            "application/n-triples",
            "<https://e/a> <https://e/b> <<( <https://e/s> <https://e/p> <https://e/o> )>> .",
        );
        assert_eq!(
            purrdf_capabilities(star, &raw mut caps),
            PurrdfStatus::Ok as i32
        );
        assert_eq!(
            caps.quoted_triples, 1,
            "an in-memory quoted triple sets the flag"
        );
        purrdf_dataset_free(star);
    }
}

// ── Invalid-discriminant tests ────────────────────────────────────────────────
// These tests verify that C-written out-of-range enum values produce
// `PurrdfStatus::InvalidArgument`, not UB/panic/crash.

/// A PurrdfTermView with `kind = 99` (unknown discriminant) passed to
/// `purrdf_term_to_ntriples` (no dataset id, so it goes through `view_to_value`)
/// must return `InvalidArgument`.
#[test]
fn invalid_term_kind_yields_invalid_argument() {
    unsafe {
        let s = "http://example.org/whatever";
        let view = PurrdfTermView {
            kind: 99, // out-of-range discriminant
            lexical: PurrdfStr {
                ptr: s.as_ptr(),
                len: s.len(),
            },
            datatype: PurrdfStr {
                ptr: std::ptr::null(),
                len: 0,
            },
            language: PurrdfStr {
                ptr: std::ptr::null(),
                len: 0,
            },
            direction: purrdf::term::PurrdfDirection::None as i32,
            blank_scope: 0,
            term_id: 0,
        };
        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        let status = purrdf_term_to_ntriples(
            std::ptr::null(),
            &raw const view,
            &raw mut buffer,
            &raw mut error,
        );
        assert_eq!(
            status,
            PurrdfStatus::InvalidArgument as i32,
            "expected InvalidArgument for unknown term kind 99"
        );
        assert!(buffer.is_null());
        assert!(!error.is_null());
        purrdf_error_free(error);
    }
}

/// A PurrdfTermView with `kind = Literal` but `direction = 99` (unknown) must
/// return `InvalidArgument`.
#[test]
fn invalid_direction_yields_invalid_argument() {
    unsafe {
        let lex = "hello";
        let dt = "http://www.w3.org/2001/XMLSchema#string";
        let view = PurrdfTermView {
            kind: PurrdfTermKind::Literal as i32,
            lexical: PurrdfStr {
                ptr: lex.as_ptr(),
                len: lex.len(),
            },
            datatype: PurrdfStr {
                ptr: dt.as_ptr(),
                len: dt.len(),
            },
            language: PurrdfStr {
                ptr: std::ptr::null(),
                len: 0,
            },
            direction: 99, // out-of-range discriminant
            blank_scope: 0,
            term_id: 0,
        };
        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        let status = purrdf_term_to_ntriples(
            std::ptr::null(),
            &raw const view,
            &raw mut buffer,
            &raw mut error,
        );
        assert_eq!(
            status,
            PurrdfStatus::InvalidArgument as i32,
            "expected InvalidArgument for unknown direction 99"
        );
        assert!(buffer.is_null());
        assert!(!error.is_null());
        purrdf_error_free(error);
    }
}

/// A PurrdfGraphMatch with `kind = 99` (unknown discriminant) passed to
/// `purrdf_quads_for_pattern` must return `InvalidArgument`.
#[test]
fn invalid_graph_match_kind_yields_invalid_argument() {
    unsafe {
        let dataset = parse("application/n-triples", THREE_QUADS);
        let graph = PurrdfGraphMatch {
            kind: 99, // out-of-range discriminant
            name: out_view(),
        };
        let mut cursor: *mut PurrdfCursor = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        let status = purrdf_quads_for_pattern(
            dataset,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            &raw const graph,
            &raw mut cursor,
            &raw mut error,
        );
        assert_eq!(
            status,
            PurrdfStatus::InvalidArgument as i32,
            "expected InvalidArgument for unknown graph match kind 99"
        );
        assert!(cursor.is_null());
        assert!(!error.is_null());
        purrdf_error_free(error);
        purrdf_dataset_free(dataset);
    }
}
