// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The dated regular-expression law, selected by name through every `*_xpath_regex`
//! entry point, driven end to end through the `extern "C"` symbols.
//!
//! Each law is selected with a pattern only that law reads as written:
//!
//! - `(?:a)b` is a non-capturing group, which XPath 3.1 defines and XPath 2.0 does not,
//!   so under 2.0 it is a pattern error — a SPARQL `FILTER` keeps no row for it.
//! - `^(a)\1$` is a back-reference, which both dated laws define and match against
//!   `"aa"`, while the compatibility regular expressions refuse back-references.
//!
//! The SPARQL query and update entry points, every SHACL validation door, and the
//! shapes-graph tools that evaluate patterns — entailment, rules (a SHACL shapes graph's
//! or a SPARQL 1.2 RL rule set's) and node expressions — are covered.
//!
//! NULL keeps the compatibility behaviour byte for byte, an inexact name is refused as
//! an invalid argument with its exact neighbour accepted, and a law's resource refusal
//! is an error status rather than an empty answer.

use std::ffi::{CStr, CString};

use purrdf::buffer::{PurrdfBuffer, purrdf_buffer_data, purrdf_buffer_free};
use purrdf::error::{
    PurrdfError, purrdf_error_code, purrdf_error_free, purrdf_error_message,
    purrdf_error_presentation_json,
};
use purrdf::governor::{PurrdfQueryGovernors, purrdf_query_governors_init};
use purrdf::handles::{PurrdfDataset, purrdf_dataset_free};
use purrdf::parse::purrdf_parse;
use purrdf::query::{
    purrdf_query, purrdf_query_entailment_governed_xpath_regex, purrdf_query_governed_xpath_regex,
    purrdf_query_json, purrdf_query_json_xpath_regex, purrdf_query_xpath_regex,
    purrdf_update_governed_xpath_regex,
};
use purrdf::rowcursor::{
    PurrdfRowCursor, purrdf_rowcursor_free, purrdf_rowcursor_next, purrdf_rowcursor_term,
};
use purrdf::shacl::{
    purrdf_shacl_apply_rules, purrdf_shacl_apply_rules_xpath_regex,
    purrdf_shacl_entail_to_ntriples, purrdf_shacl_entail_to_ntriples_xpath_regex,
    purrdf_shacl_eval_node_expr, purrdf_shacl_eval_node_expr_xpath_regex,
    purrdf_shacl_validate_changes_to_sarif, purrdf_shacl_validate_changes_to_sarif_xpath_regex,
    purrdf_shacl_validate_to_sarif, purrdf_shacl_validate_to_sarif_xpath_regex,
    purrdf_shapes_product_admit, purrdf_shapes_product_admit_expecting_xpath_regex,
    purrdf_shapes_product_admit_xpath_regex, purrdf_shapes_product_encode,
    purrdf_shapes_product_open, purrdf_shapes_product_rebuild_expecting_xpath_regex,
    purrdf_shapes_product_rebuild_xpath_regex,
};
use purrdf::status::PurrdfStatus;
use purrdf::term::{PurrdfDirection, PurrdfStr, PurrdfTermKind, PurrdfTermView};

const XPATH_20: &str = "xpath-2.0-2010-12-14";
const XPATH_31: &str = "xpath-3.1-2017-03-21";

/// Names a caller could plausibly mean but no law is spelled as.
const INEXACT_NAMES: [&str; 3] = ["xpath-3.1", "XPATH-3.1-2017-03-21", ""];

/// The non-capturing group: XPath 3.1 only.
const NON_CAPTURING: &str = "^(?:a)b$";
/// The back-reference: both dated laws, refused by compatibility.
const BACK_REFERENCE: &str = "^(a)\\1$";

const DATA_NT: &str = "<http://example.org/s1> <http://example.org/p> \"ab\" .\n\
    <http://example.org/s2> <http://example.org/p> \"aa\" .\n";

/// One call's failure: status, message and presentation record.
#[derive(Debug)]
struct Failure {
    status: i32,
    message: String,
    presentation: Option<String>,
}

unsafe fn take_failure(status: i32, error: *mut PurrdfError) -> Failure {
    unsafe {
        assert!(
            !error.is_null(),
            "a failed call must populate the error out"
        );
        assert_eq!(purrdf_error_code(error), status, "status and error agree");
        let message = CStr::from_ptr(purrdf_error_message(error))
            .to_string_lossy()
            .into_owned();
        let record = purrdf_error_presentation_json(error);
        let presentation =
            (!record.is_null()).then(|| CStr::from_ptr(record).to_string_lossy().into_owned());
        purrdf_error_free(error);
        Failure {
            status,
            message,
            presentation,
        }
    }
}

unsafe fn take_buffer(buffer: *mut PurrdfBuffer) -> String {
    unsafe {
        let mut ptr = std::ptr::null();
        let mut len = 0;
        assert_eq!(
            purrdf_buffer_data(buffer, &raw mut ptr, &raw mut len),
            PurrdfStatus::Ok as i32
        );
        let text =
            String::from_utf8(std::slice::from_raw_parts(ptr, len).to_vec()).expect("UTF-8 buffer");
        purrdf_buffer_free(buffer);
        text
    }
}

fn dataset(nt: &str) -> *mut PurrdfDataset {
    let media = CString::new("application/n-triples").expect("media type");
    let mut out = std::ptr::null_mut();
    let mut error = std::ptr::null_mut();
    let status = unsafe {
        purrdf_parse(
            nt.as_ptr(),
            nt.len(),
            media.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            &raw mut out,
            &raw mut error,
        )
    };
    assert_eq!(status, PurrdfStatus::Ok as i32, "the fixture parses");
    out
}

/// `regex` as the nullable C argument, kept alive by the returned `CString`.
fn profile_arg(regex: Option<&str>) -> (Option<CString>, *const std::os::raw::c_char) {
    let owned = regex.map(|name| CString::new(name).expect("no NUL"));
    let ptr = owned
        .as_ref()
        .map_or(std::ptr::null(), |name| name.as_ptr());
    (owned, ptr)
}

/// The objects whose `REGEX` against `pattern` holds.
fn filter_query(pattern: &str) -> String {
    format!(
        "SELECT ?o WHERE {{ ?s <http://example.org/p> ?o FILTER(REGEX(?o, {})) }} ORDER BY ?o",
        purrdf_testkit::text::sparql_string(pattern)
    )
}

unsafe fn drain_first_column(rows: *mut PurrdfRowCursor) -> Vec<String> {
    unsafe {
        let mut values = Vec::new();
        loop {
            match purrdf_rowcursor_next(rows) {
                status if status == PurrdfStatus::Ok as i32 => {}
                status if status == PurrdfStatus::CursorExhausted as i32 => break,
                status => panic!("unexpected row-cursor status {status}"),
            }
            let mut view = empty_view();
            let mut bound = 0;
            assert_eq!(
                purrdf_rowcursor_term(rows, 0, &raw mut view, &raw mut bound),
                PurrdfStatus::Ok as i32
            );
            values.push(if bound == 0 {
                "UNBOUND".to_owned()
            } else {
                let bytes = std::slice::from_raw_parts(view.lexical.ptr, view.lexical.len);
                String::from_utf8(bytes.to_vec()).expect("UTF-8 term")
            });
        }
        purrdf_rowcursor_free(rows);
        values
    }
}

fn empty_view() -> PurrdfTermView {
    let none = PurrdfStr {
        ptr: std::ptr::null(),
        len: 0,
    };
    PurrdfTermView {
        kind: PurrdfTermKind::Iri as i32,
        lexical: none,
        datatype: none,
        language: none,
        direction: PurrdfDirection::None as i32,
        blank_scope: 0,
        term_id: 0,
    }
}

fn governors() -> PurrdfQueryGovernors {
    let mut out = std::mem::MaybeUninit::uninit();
    assert_eq!(
        unsafe { purrdf_query_governors_init(out.as_mut_ptr()) },
        PurrdfStatus::Ok as i32
    );
    unsafe { out.assume_init() }
}

/// The SELECT entry points a law is selected through.
#[derive(Clone, Copy, Debug)]
enum Select {
    Plain,
    Json,
    Governed,
    Entailment,
}

const SELECTS: [Select; 4] = [
    Select::Plain,
    Select::Json,
    Select::Governed,
    Select::Entailment,
];

/// Run `query` through `entry` with `regex` (NULL when `None`), returning the first
/// column of every row.
fn select(entry: Select, regex: Option<&str>, query: &str) -> Result<Vec<String>, Failure> {
    let data = dataset(DATA_NT);
    let query = CString::new(query).expect("query");
    let (_owned, regex) = profile_arg(regex);
    let governors = governors();
    let mut error = std::ptr::null_mut();
    let mut kind = -1;
    let mut rows = std::ptr::null_mut();
    let mut outcome = -1;
    let result = unsafe {
        let status = match entry {
            Select::Plain => purrdf_query_xpath_regex(
                data,
                query.as_ptr(),
                std::ptr::null(),
                regex,
                &raw mut kind,
                &raw mut rows,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &raw mut error,
            ),
            Select::Json => {
                let mut buffer = std::ptr::null_mut();
                let status = purrdf_query_json_xpath_regex(
                    data,
                    query.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    regex,
                    &raw mut buffer,
                    &raw mut error,
                );
                if status == PurrdfStatus::Ok as i32 {
                    purrdf_dataset_free(data);
                    return Ok(json_first_column(&take_buffer(buffer)));
                }
                assert!(buffer.is_null(), "a failed call writes no buffer");
                status
            }
            Select::Governed => {
                let mut evidence = std::mem::MaybeUninit::uninit();
                let mut partial = std::mem::MaybeUninit::uninit();
                purrdf_query_governed_xpath_regex(
                    data,
                    query.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    regex,
                    &raw const governors,
                    &raw mut outcome,
                    &raw mut kind,
                    &raw mut rows,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    evidence.as_mut_ptr(),
                    partial.as_mut_ptr(),
                    &raw mut error,
                )
            }
            Select::Entailment => {
                let regime = CString::new("simple").expect("regime");
                let program = CString::new("").expect("program");
                let mut evidence = std::mem::MaybeUninit::uninit();
                let mut partial = std::mem::MaybeUninit::uninit();
                let mut report = std::ptr::null_mut();
                let status = purrdf_query_entailment_governed_xpath_regex(
                    data,
                    query.as_ptr(),
                    std::ptr::null(),
                    regime.as_ptr(),
                    program.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    regex,
                    &raw const governors,
                    &raw mut outcome,
                    &raw mut kind,
                    &raw mut rows,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    evidence.as_mut_ptr(),
                    partial.as_mut_ptr(),
                    &raw mut report,
                    &raw mut error,
                );
                if status == PurrdfStatus::Ok as i32 {
                    take_buffer(report);
                } else {
                    assert!(report.is_null(), "a failed call writes no report");
                }
                status
            }
        };
        if status == PurrdfStatus::Ok as i32 {
            assert!(error.is_null());
            assert_eq!(kind, 0, "{entry:?}: a SELECT result");
            if matches!(entry, Select::Governed | Select::Entailment) {
                assert_eq!(outcome, 0, "{entry:?}: the request completed");
            }
            Ok(drain_first_column(rows))
        } else {
            assert!(rows.is_null(), "{entry:?}: a failed call writes no rows");
            assert!(kind == -1 || matches!(entry, Select::Json));
            Err(take_failure(status, error))
        }
    };
    unsafe { purrdf_dataset_free(data) };
    result
}

/// The first column of an SRJ document's bindings, in order.
fn json_first_column(srj: &str) -> Vec<String> {
    let document = purrdf_lex::json::read(srj).expect("SRJ parses");
    let variable = document
        .pointer("/head/vars/0")
        .and_then(purrdf_lex::json::Value::as_str)
        .expect("one variable")
        .to_owned();
    let mut values = Vec::new();
    for index in 0.. {
        let Some(row) = document.pointer(&format!("/results/bindings/{index}")) else {
            break;
        };
        values.push(
            row.get(&variable)
                .and_then(|cell| cell.get("value"))
                .and_then(purrdf_lex::json::Value::as_str)
                .unwrap_or("UNBOUND")
                .to_owned(),
        );
    }
    values
}

fn rows(entry: Select, regex: Option<&str>, query: &str) -> Vec<String> {
    select(entry, regex, query).unwrap_or_else(|failure| panic!("{entry:?}: {failure:?}"))
}

#[test]
fn each_selected_law_decides_sparql_regex_on_every_select_entry_point() {
    for entry in SELECTS {
        // XPath 3.1 reads the non-capturing group; XPath 2.0 does not, so the FILTER
        // expression errors and keeps no row — and the request itself still succeeds.
        assert_eq!(
            rows(entry, Some(XPATH_31), &filter_query(NON_CAPTURING)),
            ["ab"],
            "{entry:?}"
        );
        assert_eq!(
            rows(entry, Some(XPATH_20), &filter_query(NON_CAPTURING)),
            Vec::<String>::new(),
            "{entry:?}"
        );
        // Both dated laws match the back-reference; compatibility refuses it.
        for law in [XPATH_20, XPATH_31] {
            assert_eq!(
                rows(entry, Some(law), &filter_query(BACK_REFERENCE)),
                ["aa"],
                "{entry:?} {law}"
            );
        }
        assert_eq!(
            rows(entry, None, &filter_query(BACK_REFERENCE)),
            Vec::<String>::new(),
            "{entry:?}: NULL keeps compatibility"
        );
    }
}

#[test]
fn each_selected_law_decides_sparql_replace() {
    let query = format!(
        "SELECT ?r WHERE {{ BIND(REPLACE(\"aa\", {}, \"x\") AS ?r) }}",
        purrdf_testkit::text::sparql_string(BACK_REFERENCE)
    );
    for law in [XPATH_20, XPATH_31] {
        assert_eq!(rows(Select::Plain, Some(law), &query), ["x"], "{law}");
    }
    assert_eq!(rows(Select::Plain, None, &query), ["UNBOUND"]);
    let query = format!(
        "SELECT ?r WHERE {{ BIND(REPLACE(\"ab\", {}, \"x\") AS ?r) }}",
        purrdf_testkit::text::sparql_string(NON_CAPTURING)
    );
    assert_eq!(rows(Select::Plain, Some(XPATH_31), &query), ["x"]);
    assert_eq!(rows(Select::Plain, Some(XPATH_20), &query), ["UNBOUND"]);
}

#[test]
fn null_regex_profile_is_the_compatibility_entry_point_byte_for_byte() {
    for pattern in [NON_CAPTURING, BACK_REFERENCE, "^a"] {
        let query = CString::new(filter_query(pattern)).expect("query");
        let data = dataset(DATA_NT);
        let mut legacy = std::ptr::null_mut();
        let mut selected = std::ptr::null_mut();
        let mut error = std::ptr::null_mut();
        unsafe {
            assert_eq!(
                purrdf_query_json(
                    data,
                    query.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    &raw mut legacy,
                    &raw mut error,
                ),
                PurrdfStatus::Ok as i32
            );
            assert_eq!(
                purrdf_query_json_xpath_regex(
                    data,
                    query.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    &raw mut selected,
                    &raw mut error,
                ),
                PurrdfStatus::Ok as i32
            );
            assert_eq!(take_buffer(legacy), take_buffer(selected), "{pattern}");

            let mut kind = -1;
            let mut rows = std::ptr::null_mut();
            assert_eq!(
                purrdf_query(
                    data,
                    query.as_ptr(),
                    std::ptr::null(),
                    &raw mut kind,
                    &raw mut rows,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &raw mut error,
                ),
                PurrdfStatus::Ok as i32
            );
            assert_eq!(
                drain_first_column(rows),
                filtered(Select::Plain, None, pattern)
            );
            purrdf_dataset_free(data);
        }
    }
}

fn filtered(entry: Select, regex: Option<&str>, pattern: &str) -> Vec<String> {
    rows(entry, regex, &filter_query(pattern))
}

/// Apply `INSERT { ?s ex:matched true } WHERE { ... FILTER(REGEX(...)) }` and return
/// the subjects the update marked.
fn update_marks(regex: Option<&str>, pattern: &str) -> Result<Vec<String>, Failure> {
    let data = dataset(DATA_NT);
    let request = CString::new(format!(
        "INSERT {{ ?s <http://example.org/matched> true }} WHERE {{ \
         ?s <http://example.org/p> ?o FILTER(REGEX(?o, {})) }}",
        purrdf_testkit::text::sparql_string(pattern)
    ))
    .expect("request");
    let (_owned, regex) = profile_arg(regex);
    let governors = governors();
    let mut outcome = -1;
    let mut evidence = std::mem::MaybeUninit::uninit();
    let mut error = std::ptr::null_mut();
    let status = unsafe {
        purrdf_update_governed_xpath_regex(
            data,
            request.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            regex,
            &raw const governors,
            &raw mut outcome,
            evidence.as_mut_ptr(),
            &raw mut error,
        )
    };
    let result = if status == PurrdfStatus::Ok as i32 {
        assert_eq!(outcome, 0, "the update applied");
        let query =
            CString::new("SELECT ?s WHERE { ?s <http://example.org/matched> true } ORDER BY ?s")
                .expect("query");
        let mut kind = -1;
        let mut rows = std::ptr::null_mut();
        unsafe {
            assert_eq!(
                purrdf_query(
                    data,
                    query.as_ptr(),
                    std::ptr::null(),
                    &raw mut kind,
                    &raw mut rows,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &raw mut error,
                ),
                PurrdfStatus::Ok as i32
            );
            Ok(drain_first_column(rows))
        }
    } else {
        Err(unsafe { take_failure(status, error) })
    };
    unsafe { purrdf_dataset_free(data) };
    result
}

#[test]
fn each_selected_law_decides_the_update_where_clause() {
    let marked = |regex, pattern| update_marks(regex, pattern).expect("update applies");
    assert_eq!(
        marked(Some(XPATH_31), NON_CAPTURING),
        ["http://example.org/s1"]
    );
    assert_eq!(marked(Some(XPATH_20), NON_CAPTURING), Vec::<String>::new());
    for law in [XPATH_20, XPATH_31] {
        assert_eq!(
            marked(Some(law), BACK_REFERENCE),
            ["http://example.org/s2"],
            "{law}"
        );
    }
    assert_eq!(marked(None, BACK_REFERENCE), Vec::<String>::new());
}

fn assert_invalid_name(failure: &Failure, name: &str) {
    assert_eq!(
        failure.status,
        PurrdfStatus::InvalidArgument as i32,
        "{name:?}: {failure:?}"
    );
    assert!(
        failure.message.contains(&format!("{XPATH_20:?}"))
            && failure.message.contains(&format!("{XPATH_31:?}")),
        "{name:?}: the refusal lists the accepted names: {}",
        failure.message
    );
}

#[test]
fn an_inexact_profile_name_is_refused_on_every_sparql_entry_point() {
    let query = filter_query("^a");
    for name in INEXACT_NAMES {
        for entry in SELECTS {
            let failure = select(entry, Some(name), &query).expect_err(name);
            assert_invalid_name(&failure, name);
        }
        let failure = update_marks(Some(name), "^a").expect_err(name);
        assert_invalid_name(&failure, name);
    }
    // The exact neighbour of each refused name is accepted everywhere.
    for entry in SELECTS {
        assert_eq!(
            rows(entry, Some(XPATH_31), &query),
            ["aa", "ab"],
            "{entry:?}"
        );
    }
    assert_eq!(
        update_marks(Some(XPATH_31), "^a").expect("exact name"),
        ["http://example.org/s1", "http://example.org/s2"]
    );
}

#[test]
fn a_native_resource_refusal_is_an_error_status_never_an_empty_answer() {
    // The production pattern-source bound is 64 KiB: one byte more is withheld before
    // parsing, and exactly the bound is admitted.
    let admitted = "a".repeat(64 * 1024);
    let withheld = "a".repeat(64 * 1024 + 1);
    for entry in SELECTS {
        for law in [XPATH_20, XPATH_31] {
            assert_eq!(
                rows(entry, Some(law), &filter_query(&admitted)),
                Vec::<String>::new(),
                "{entry:?} {law}: the bound itself is admitted"
            );
            let failure = select(entry, Some(law), &filter_query(&withheld)).expect_err("withheld");
            assert_eq!(
                failure.status,
                PurrdfStatus::RegexResourceError as i32,
                "{entry:?} {law}: {failure:?}"
            );
            assert!(
                failure.message.contains("xpath-pattern-bytes"),
                "{failure:?}"
            );
            assert!(
                failure
                    .presentation
                    .as_deref()
                    .is_some_and(|record| record.contains("xpath-pattern-bytes")),
                "{failure:?}"
            );
        }
    }
    let failure = update_marks(Some(XPATH_31), &withheld).expect_err("withheld");
    assert_eq!(failure.status, PurrdfStatus::RegexResourceError as i32);
    assert_eq!(
        update_marks(Some(XPATH_31), &admitted).expect("admitted"),
        Vec::<String>::new()
    );
}

// ---------------------------------------------------------------------------
// SHACL
// ---------------------------------------------------------------------------

/// A shape requiring `ex:s1 ex:p` to match `pattern`, written as a Turtle literal.
fn pattern_shapes(pattern: &str) -> String {
    format!(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
         @prefix ex: <http://example.org/> .\n\
         ex:S a sh:NodeShape ; sh:targetNode ex:{node} ;\n\
           sh:property [ sh:path ex:p ; sh:pattern \"{literal}\" ] .\n",
        node = if pattern == BACK_REFERENCE {
            "s2"
        } else {
            "s1"
        },
        literal = pattern.replace('\\', "\\\\"),
    )
}

fn conforms(sarif: &str) -> bool {
    if sarif.contains("\"shaclConforms\": true") {
        true
    } else {
        assert!(sarif.contains("\"shaclConforms\": false"), "{sarif}");
        false
    }
}

fn validate(regex: Option<&str>, shapes: &str) -> Result<String, Failure> {
    let shapes = CString::new(shapes).expect("shapes");
    let data = CString::new(DATA_NT).expect("data");
    let (_owned, regex) = profile_arg(regex);
    let mut buffer = std::ptr::null_mut();
    let mut error = std::ptr::null_mut();
    let status = unsafe {
        purrdf_shacl_validate_to_sarif_xpath_regex(
            shapes.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            data.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null(),
            std::ptr::null(),
            0,
            false,
            regex,
            &raw mut buffer,
            &raw mut error,
        )
    };
    if status == PurrdfStatus::Ok as i32 {
        Ok(unsafe { take_buffer(buffer) })
    } else {
        assert!(buffer.is_null(), "a failed validation writes no report");
        Err(unsafe { take_failure(status, error) })
    }
}

/// The change path: `DATA_NT` is the base, and the whole delta is an addition of a row
/// the shape reads, so the run is a bounded one over the shape's focus node.
fn validate_change(regex: Option<&str>, shapes: &str) -> Result<(String, i32), Failure> {
    let shapes = CString::new(shapes).expect("shapes");
    let data = CString::new("").expect("data");
    let added = CString::new(DATA_NT).expect("added");
    let (_owned, regex) = profile_arg(regex);
    let mut buffer = std::ptr::null_mut();
    let mut scope = -1;
    let mut focus_nodes = 0;
    let mut reason = std::ptr::null_mut();
    let mut error = std::ptr::null_mut();
    let status = unsafe {
        purrdf_shacl_validate_changes_to_sarif_xpath_regex(
            shapes.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            data.as_ptr(),
            added.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            std::ptr::null(),
            std::ptr::null(),
            0,
            false,
            regex,
            &raw mut buffer,
            &raw mut scope,
            &raw mut focus_nodes,
            &raw mut reason,
            &raw mut error,
        )
    };
    if status == PurrdfStatus::Ok as i32 {
        if !reason.is_null() {
            unsafe { take_buffer(reason) };
        }
        Ok((unsafe { take_buffer(buffer) }, scope))
    } else {
        assert!(buffer.is_null(), "a failed validation writes no report");
        Err(unsafe { take_failure(status, error) })
    }
}

/// Encode `shapes` as a prepared product.
fn product(shapes: &str) -> Vec<u8> {
    let shapes = CString::new(shapes).expect("shapes");
    let mut buffer = std::ptr::null_mut();
    let mut error = std::ptr::null_mut();
    let status = unsafe {
        purrdf_shapes_product_encode(
            shapes.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            &raw mut buffer,
            &raw mut error,
        )
    };
    assert_eq!(status, PurrdfStatus::Ok as i32, "the product encodes");
    let mut ptr = std::ptr::null();
    let mut len = 0;
    unsafe {
        assert_eq!(
            purrdf_buffer_data(buffer, &raw mut ptr, &raw mut len),
            PurrdfStatus::Ok as i32
        );
        let bytes = std::slice::from_raw_parts(ptr, len).to_vec();
        purrdf_buffer_free(buffer);
        bytes
    }
}

/// The product's `identity-digest` line, as `purrdf_shapes_product_open` renders it.
fn identity(product: &[u8]) -> CString {
    let mut buffer = std::ptr::null_mut();
    let mut error = std::ptr::null_mut();
    let status = unsafe {
        purrdf_shapes_product_open(
            product.as_ptr(),
            product.len(),
            &raw mut buffer,
            &raw mut error,
        )
    };
    assert_eq!(status, PurrdfStatus::Ok as i32, "the product opens");
    let described = unsafe { take_buffer(buffer) };
    let digest = described
        .lines()
        .find_map(|line| line.strip_prefix("identity-digest "))
        .expect("an identity-digest line");
    CString::new(digest).expect("digest")
}

/// The four product doors that take a law.
#[derive(Clone, Copy, Debug)]
enum ProductDoor {
    Admit,
    AdmitExpecting,
    Rebuild,
    RebuildExpecting,
}

const PRODUCT_DOORS: [ProductDoor; 4] = [
    ProductDoor::Admit,
    ProductDoor::AdmitExpecting,
    ProductDoor::Rebuild,
    ProductDoor::RebuildExpecting,
];

fn validate_product(
    door: ProductDoor,
    regex: Option<&str>,
    shapes: &str,
) -> Result<String, Failure> {
    let product = product(shapes);
    let expected = identity(&product);
    let data = CString::new(DATA_NT).expect("data");
    let (_owned, regex) = profile_arg(regex);
    let mut buffer = std::ptr::null_mut();
    let mut error = std::ptr::null_mut();
    let status = unsafe {
        match door {
            ProductDoor::Admit => purrdf_shapes_product_admit_xpath_regex(
                product.as_ptr(),
                product.len(),
                data.as_ptr(),
                regex,
                &raw mut buffer,
                &raw mut error,
            ),
            ProductDoor::AdmitExpecting => purrdf_shapes_product_admit_expecting_xpath_regex(
                product.as_ptr(),
                product.len(),
                data.as_ptr(),
                expected.as_ptr(),
                regex,
                &raw mut buffer,
                &raw mut error,
            ),
            ProductDoor::Rebuild => purrdf_shapes_product_rebuild_xpath_regex(
                product.as_ptr(),
                product.len(),
                data.as_ptr(),
                regex,
                &raw mut buffer,
                &raw mut error,
            ),
            ProductDoor::RebuildExpecting => purrdf_shapes_product_rebuild_expecting_xpath_regex(
                product.as_ptr(),
                product.len(),
                data.as_ptr(),
                expected.as_ptr(),
                regex,
                &raw mut buffer,
                &raw mut error,
            ),
        }
    };
    if status == PurrdfStatus::Ok as i32 {
        Ok(unsafe { take_buffer(buffer) })
    } else {
        assert!(buffer.is_null(), "a failed validation writes no report");
        Err(unsafe { take_failure(status, error) })
    }
}

/// One SHACL entry point: does validation under `(regex, shapes)` conform?
type Door = Box<dyn Fn(Option<&str>, &str) -> Result<bool, Failure>>;

/// Every SHACL door, as one closure over `(regex, shapes)` answering "conforms?".
fn shacl_doors() -> Vec<(String, Door)> {
    let mut doors: Vec<(String, Door)> = vec![
        (
            "validate".to_owned(),
            Box::new(|regex, shapes| validate(regex, shapes).map(|sarif| conforms(&sarif))),
        ),
        (
            "validate_changes".to_owned(),
            Box::new(|regex, shapes| {
                validate_change(regex, shapes).map(|(sarif, _)| conforms(&sarif))
            }),
        ),
    ];
    for door in PRODUCT_DOORS {
        doors.push((
            format!("{door:?}"),
            Box::new(move |regex, shapes| {
                validate_product(door, regex, shapes).map(|sarif| conforms(&sarif))
            }),
        ));
    }
    doors
}

#[test]
fn each_selected_law_decides_sh_pattern_on_every_shacl_entry_point() {
    let non_capturing = pattern_shapes(NON_CAPTURING);
    let back_reference = pattern_shapes(BACK_REFERENCE);
    for (name, door) in shacl_doors() {
        let verdict = |regex, shapes: &str| {
            door(regex, shapes).unwrap_or_else(|failure| panic!("{name}: {failure:?}"))
        };
        assert!(verdict(Some(XPATH_31), &non_capturing), "{name}");
        assert!(!verdict(Some(XPATH_20), &non_capturing), "{name}");
        for law in [XPATH_20, XPATH_31] {
            assert!(verdict(Some(law), &back_reference), "{name} {law}");
        }
        assert!(
            !verdict(None, &back_reference),
            "{name}: NULL keeps compatibility"
        );
    }
}

#[test]
fn the_change_path_stays_bounded_under_a_selected_law() {
    let (_, scope) =
        validate_change(Some(XPATH_31), &pattern_shapes(NON_CAPTURING)).expect("validates");
    assert_eq!(scope, 0, "BOUNDED");
}

#[test]
fn null_regex_profile_is_the_compatibility_shacl_entry_point_byte_for_byte() {
    for pattern in [NON_CAPTURING, BACK_REFERENCE, "^a"] {
        let shapes_text = pattern_shapes(pattern);
        let shapes = CString::new(shapes_text.as_str()).expect("shapes");
        let data = CString::new(DATA_NT).expect("data");
        let mut buffer = std::ptr::null_mut();
        let mut error = std::ptr::null_mut();
        let status = unsafe {
            purrdf_shacl_validate_to_sarif(
                shapes.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                data.as_ptr(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                0,
                false,
                &raw mut buffer,
                &raw mut error,
            )
        };
        assert_eq!(status, PurrdfStatus::Ok as i32);
        let legacy = unsafe { take_buffer(buffer) };
        assert_eq!(
            legacy,
            validate(None, &shapes_text).expect("NULL"),
            "{pattern}"
        );

        let empty = CString::new("").expect("empty");
        let mut scope = -1;
        let mut focus_nodes = 0;
        let mut reason = std::ptr::null_mut();
        let status = unsafe {
            purrdf_shacl_validate_changes_to_sarif(
                shapes.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                empty.as_ptr(),
                data.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                0,
                false,
                &raw mut buffer,
                &raw mut scope,
                &raw mut focus_nodes,
                &raw mut reason,
                &raw mut error,
            )
        };
        assert_eq!(status, PurrdfStatus::Ok as i32);
        assert!(reason.is_null());
        let legacy = unsafe { take_buffer(buffer) };
        let (selected, selected_scope) = validate_change(None, &shapes_text).expect("NULL");
        assert_eq!((legacy, scope), (selected, selected_scope), "{pattern}");

        let product = product(&shapes_text);
        let status = unsafe {
            purrdf_shapes_product_admit(
                product.as_ptr(),
                product.len(),
                data.as_ptr(),
                &raw mut buffer,
                &raw mut error,
            )
        };
        assert_eq!(status, PurrdfStatus::Ok as i32);
        let legacy = unsafe { take_buffer(buffer) };
        assert_eq!(
            legacy,
            validate_product(ProductDoor::Admit, None, &shapes_text).expect("NULL"),
            "{pattern}"
        );
    }
}

#[test]
fn a_selected_law_reports_what_compatibility_reports_for_an_ordinary_pattern() {
    // `^a` means the same under every law, so the report must not move with the law.
    let shapes = pattern_shapes("^a");
    let compatibility = validate(None, &shapes).expect("NULL");
    for law in [XPATH_20, XPATH_31] {
        assert_eq!(
            validate(Some(law), &shapes).expect(law),
            compatibility,
            "{law}"
        );
    }
}

#[test]
fn an_inexact_profile_name_is_refused_on_every_shacl_entry_point() {
    let shapes = pattern_shapes("^a");
    for (door_name, door) in shacl_doors() {
        for name in INEXACT_NAMES {
            let failure = door(Some(name), &shapes).expect_err(name);
            assert_invalid_name(&failure, name);
        }
        assert!(
            door(Some(XPATH_31), &shapes).expect("the exact neighbour"),
            "{door_name}"
        );
    }
}

#[test]
fn a_native_resource_refusal_is_an_error_status_never_a_conforming_report() {
    let admitted = pattern_shapes(&"a".repeat(64 * 1024));
    let withheld = pattern_shapes(&"a".repeat(64 * 1024 + 1));
    for (name, door) in shacl_doors() {
        for law in [XPATH_20, XPATH_31] {
            // The bound itself compiles and simply does not match "ab".
            assert!(
                !door(Some(law), &admitted).unwrap_or_else(|f| panic!("{name}: {f:?}")),
                "{name} {law}"
            );
            let failure = door(Some(law), &withheld).expect_err("withheld");
            assert_eq!(
                failure.status,
                PurrdfStatus::RegexResourceError as i32,
                "{name} {law}: {failure:?}"
            );
            assert!(
                failure.message.contains("xpath-pattern-bytes"),
                "{name}: {failure:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Shapes-graph tools: entailment, rules, node expressions
// ---------------------------------------------------------------------------

/// A shapes graph whose SPARQL rule infers `?this ex:hit ?v` for each `ex:p` value
/// `pattern` matches.
fn rule_shapes(pattern: &str) -> String {
    let construct = format!(
        "CONSTRUCT {{ $this <http://example.org/hit> ?v }} \
         WHERE {{ $this <http://example.org/p> ?v FILTER(REGEX(?v, {})) }}",
        purrdf_testkit::text::sparql_string(pattern)
    );
    format!(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
         @prefix ex: <http://example.org/> .\n\
         ex:S a sh:NodeShape ; sh:targetSubjectsOf ex:p ;\n\
           sh:rule [ a sh:SPARQLRule ; sh:construct \"{}\" ] .\n",
        construct.replace('\\', "\\\\").replace('"', "\\\"")
    )
}

/// The SPARQL 1.2 RL twin of [`rule_shapes`].
fn rule_set(pattern: &str) -> String {
    format!(
        "PREFIX : <http://example.org/>\n\
         RULE {{ ?s :hit ?v }} WHERE {{ ?s :p ?v FILTER(REGEX(?v, {})) }}\n",
        purrdf_testkit::text::sparql_string(pattern)
    )
}

/// A shapes graph whose `_:e` keeps the focus node only when it matches `pattern`.
fn expression_shapes(pattern: &str) -> String {
    format!(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
         @prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .\n\
         _:e shnex:filterShape [ sh:pattern \"{}\" ] ; shnex:nodes [ shnex:var \"focusNode\" ] .\n",
        pattern.replace('\\', "\\\\")
    )
}

/// One shapes-graph tool, `(regex, legacy, pattern)` → its output text: `legacy` calls the
/// entry point without the suffix (and then `regex` must be `None`).
type Tool = fn(Option<&str>, bool, &str) -> Result<String, Failure>;

fn entail(regex: Option<&str>, legacy: bool, pattern: &str) -> Result<String, Failure> {
    let shapes = CString::new(rule_shapes(pattern)).expect("shapes");
    let data = CString::new(DATA_NT).expect("data");
    let (_owned, regex) = profile_arg(regex);
    let mut buffer = std::ptr::null_mut();
    let mut error = std::ptr::null_mut();
    let null = std::ptr::null();
    let status = unsafe {
        if legacy {
            purrdf_shacl_entail_to_ntriples(
                shapes.as_ptr(),
                null,
                null,
                data.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                &raw mut buffer,
                std::ptr::null_mut(),
                &raw mut error,
            )
        } else {
            purrdf_shacl_entail_to_ntriples_xpath_regex(
                shapes.as_ptr(),
                null,
                null,
                data.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                regex,
                &raw mut buffer,
                std::ptr::null_mut(),
                &raw mut error,
            )
        }
    };
    if status == PurrdfStatus::Ok as i32 {
        Ok(unsafe { take_buffer(buffer) })
    } else {
        assert!(buffer.is_null(), "a failed entailment writes no dataset");
        Err(unsafe { take_failure(status, error) })
    }
}

fn apply_rules(
    regex: Option<&str>,
    legacy: bool,
    pattern: &str,
    srl: bool,
) -> Result<String, Failure> {
    let shapes = CString::new(rule_shapes(pattern)).expect("shapes");
    let rules = CString::new(rule_set(pattern)).expect("rules");
    let (shapes, rules) = if srl {
        (std::ptr::null(), rules.as_ptr())
    } else {
        (shapes.as_ptr(), std::ptr::null())
    };
    let data = CString::new(DATA_NT).expect("data");
    let (_owned, regex) = profile_arg(regex);
    let mut inferred = std::ptr::null_mut();
    let mut error = std::ptr::null_mut();
    let null = std::ptr::null();
    let status = unsafe {
        if legacy {
            purrdf_shacl_apply_rules(
                data.as_ptr(),
                shapes,
                null,
                null,
                rules,
                null,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                &raw mut inferred,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &raw mut error,
            )
        } else {
            purrdf_shacl_apply_rules_xpath_regex(
                data.as_ptr(),
                shapes,
                null,
                null,
                rules,
                null,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                regex,
                &raw mut inferred,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &raw mut error,
            )
        }
    };
    if status == PurrdfStatus::Ok as i32 {
        Ok(unsafe { take_buffer(inferred) })
    } else {
        assert!(inferred.is_null(), "a failed rules run writes no inference");
        Err(unsafe { take_failure(status, error) })
    }
}

fn shapes_rules(regex: Option<&str>, legacy: bool, pattern: &str) -> Result<String, Failure> {
    apply_rules(regex, legacy, pattern, false)
}

fn srl_rules(regex: Option<&str>, legacy: bool, pattern: &str) -> Result<String, Failure> {
    apply_rules(regex, legacy, pattern, true)
}

/// The focus each expression fixture is evaluated at: the value the pattern's case reads.
fn focus_for(pattern: &str) -> &'static str {
    if pattern == BACK_REFERENCE {
        "\"aa\""
    } else {
        "\"ab\""
    }
}

fn node_expr(regex: Option<&str>, legacy: bool, pattern: &str) -> Result<String, Failure> {
    let shapes = CString::new(expression_shapes(pattern)).expect("shapes");
    let data = CString::new(DATA_NT).expect("data");
    let expr = CString::new("_:e").expect("expr");
    let focus = CString::new(focus_for(pattern)).expect("focus");
    let (_owned, regex) = profile_arg(regex);
    let mut terms = std::ptr::null_mut();
    let mut error = std::ptr::null_mut();
    let null = std::ptr::null();
    let status = unsafe {
        if legacy {
            purrdf_shacl_eval_node_expr(
                shapes.as_ptr(),
                null,
                data.as_ptr(),
                expr.as_ptr(),
                null,
                std::ptr::null(),
                0,
                null,
                focus.as_ptr(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                0,
                &raw mut terms,
                std::ptr::null_mut(),
                &raw mut error,
            )
        } else {
            purrdf_shacl_eval_node_expr_xpath_regex(
                shapes.as_ptr(),
                null,
                data.as_ptr(),
                expr.as_ptr(),
                null,
                std::ptr::null(),
                0,
                null,
                focus.as_ptr(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                0,
                regex,
                &raw mut terms,
                std::ptr::null_mut(),
                &raw mut error,
            )
        }
    };
    if status == PurrdfStatus::Ok as i32 {
        Ok(unsafe { take_buffer(terms) })
    } else {
        assert!(terms.is_null(), "a failed evaluation writes no output");
        Err(unsafe { take_failure(status, error) })
    }
}

/// Whether a tool's output for a pattern shows the pattern matched.
type Matched = fn(&str, &str) -> bool;

/// Every pattern-evaluating shapes-graph tool, and how its output shows a match.
const TOOLS: [(&str, Tool, Matched); 4] = [
    ("entail", entail, inferred_hit),
    ("apply_rules shapes", shapes_rules, inferred_hit),
    ("apply_rules srl", srl_rules, inferred_hit),
    ("eval_node_expr", node_expr, kept_focus),
];

fn inferred_hit(output: &str, _pattern: &str) -> bool {
    output.contains("<http://example.org/hit>")
}

fn kept_focus(output: &str, pattern: &str) -> bool {
    output == format!("{}\n", focus_for(pattern))
}

#[test]
fn each_selected_law_decides_patterns_on_every_shapes_graph_tool() {
    for (name, tool, matched) in TOOLS {
        let verdict = |regex, pattern: &str| {
            let output =
                tool(regex, false, pattern).unwrap_or_else(|failure| panic!("{name}: {failure:?}"));
            matched(&output, pattern)
        };
        assert!(verdict(Some(XPATH_31), NON_CAPTURING), "{name}");
        assert!(!verdict(Some(XPATH_20), NON_CAPTURING), "{name}");
        for law in [XPATH_20, XPATH_31] {
            assert!(verdict(Some(law), BACK_REFERENCE), "{name} {law}");
        }
        assert!(
            !verdict(None, BACK_REFERENCE),
            "{name}: NULL keeps compatibility"
        );
    }
}

#[test]
fn null_regex_profile_is_the_compatibility_tool_entry_point_byte_for_byte() {
    for (name, tool, _) in TOOLS {
        for pattern in [NON_CAPTURING, BACK_REFERENCE, "^a"] {
            assert_eq!(
                tool(None, true, pattern).expect("legacy"),
                tool(None, false, pattern).expect("NULL"),
                "{name} {pattern}"
            );
        }
    }
}

#[test]
fn an_inexact_profile_name_is_refused_on_every_shapes_graph_tool() {
    for (name, tool, _) in TOOLS {
        for inexact in INEXACT_NAMES {
            let failure = tool(Some(inexact), false, "^a").expect_err(inexact);
            assert_invalid_name(&failure, inexact);
        }
        // The exact neighbours are accepted.
        for law in [XPATH_20, XPATH_31] {
            tool(Some(law), false, "^a").unwrap_or_else(|failure| panic!("{name}: {failure:?}"));
        }
    }
}

#[test]
fn a_native_resource_refusal_is_an_error_status_never_an_empty_tool_answer() {
    let admitted = "a".repeat(64 * 1024);
    let withheld = "a".repeat(64 * 1024 + 1);
    for (name, tool, matched) in TOOLS {
        for law in [XPATH_20, XPATH_31] {
            let output = tool(Some(law), false, &admitted)
                .unwrap_or_else(|failure| panic!("{name}: {failure:?}"));
            assert!(!matched(&output, &admitted), "{name} {law}");
            let failure = tool(Some(law), false, &withheld).expect_err("withheld");
            assert_eq!(
                failure.status,
                PurrdfStatus::RegexResourceError as i32,
                "{name} {law}: {failure:?}"
            );
            assert!(
                failure.message.contains("xpath-pattern-bytes"),
                "{name}: {failure:?}"
            );
        }
    }
}
