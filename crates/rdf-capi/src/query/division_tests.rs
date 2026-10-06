// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The dataset handle's division policy and the evidence's expression-error counts,
//! through the exported entry points alone: every handle is read back through its
//! accessor (`purrdf_buffer_data`, `purrdf_error_code`, `purrdf_error_message`,
//! `purrdf_rowcursor_term`), never dereferenced.

use std::ffi::{CStr, CString};
use std::mem::MaybeUninit;

use purrdf_core::RdfDatasetBuilder;

use super::*;
use crate::buffer::{purrdf_buffer_data, purrdf_buffer_free};
use crate::error::{purrdf_error_code, purrdf_error_free, purrdf_error_message};
use crate::governor::{
    PURRDF_EXPRESSION_ERROR_CODE_COUNT, PurrdfExpressionErrorCode, purrdf_query_governors_init,
};
use crate::handles::purrdf_dataset_free;
use crate::rowcursor::{purrdf_rowcursor_free, purrdf_rowcursor_next, purrdf_rowcursor_term};
use crate::term::PurrdfTermView;

/// A dataset handle with one triple, so an UPDATE has something to sit beside.
fn dataset() -> *mut PurrdfDataset {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("http://example.org/s");
    let predicate = builder.intern_iri("http://example.org/p");
    let object = builder.intern_iri("http://example.org/o");
    builder.push_quad(subject, predicate, object, None);
    into_handle(PurrdfDataset::new(builder.freeze().expect("freeze")))
}

/// The bytes of a buffer handle, read through `purrdf_buffer_data`, then the buffer freed.
fn take_buffer_text(buffer: *mut PurrdfBuffer) -> String {
    let mut ptr = std::ptr::null();
    let mut len = 0;
    assert_eq!(
        unsafe { purrdf_buffer_data(buffer, &raw mut ptr, &raw mut len) },
        PurrdfStatus::Ok as i32
    );
    let text = String::from_utf8(unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec())
        .expect("UTF-8 buffer");
    unsafe { purrdf_buffer_free(buffer) };
    text
}

/// The status and message of an error handle, read through its accessors, then freed.
fn take_error(error: *mut PurrdfError) -> (i32, String) {
    assert!(!error.is_null(), "a failing call stores an error");
    let code = unsafe { purrdf_error_code(error) };
    let message = unsafe { CStr::from_ptr(purrdf_error_message(error)) }
        .to_string_lossy()
        .into_owned();
    unsafe { purrdf_error_free(error) };
    (code, message)
}

/// Set `policy` on `dataset`: `Ok` or the stored error's status and message.
fn set_policy(dataset: *mut PurrdfDataset, policy: &str) -> Result<(), (i32, String)> {
    let policy = CString::new(policy).expect("policy C string");
    let mut error = std::ptr::null_mut();
    let status =
        unsafe { purrdf_dataset_set_division_policy(dataset, policy.as_ptr(), &raw mut error) };
    if status == PurrdfStatus::Ok as i32 {
        assert!(error.is_null());
        Ok(())
    } else {
        let (code, message) = take_error(error);
        assert_eq!(code, status, "the error carries the returned status");
        Err((code, message))
    }
}

/// The policy in force on `dataset`, read back through `purrdf_dataset_division_policy`.
fn policy_of(dataset: *const PurrdfDataset) -> String {
    let mut buffer = std::ptr::null_mut();
    let mut error = std::ptr::null_mut();
    assert_eq!(
        unsafe { purrdf_dataset_division_policy(dataset, &raw mut buffer, &raw mut error) },
        PurrdfStatus::Ok as i32
    );
    assert!(error.is_null());
    take_buffer_text(buffer)
}

/// `purrdf_query_json` over `dataset`: the results document, or the error.
fn query_json(dataset: *const PurrdfDataset, query: &str) -> Result<String, (i32, String)> {
    let query = CString::new(query).expect("query C string");
    let mut buffer = std::ptr::null_mut();
    let mut error = std::ptr::null_mut();
    let status = unsafe {
        purrdf_query_json(
            dataset,
            query.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            &raw mut buffer,
            &raw mut error,
        )
    };
    if status == PurrdfStatus::Ok as i32 {
        Ok(take_buffer_text(buffer))
    } else {
        Err(take_error(error))
    }
}

/// The lexical form `?x` is bound to in a `purrdf_query_json` SELECT document.
fn bound_x(document: &str) -> String {
    let value = purrdf_lex::json::read(document).expect("results JSON");
    value
        .pointer("/results/bindings/0/x/value")
        .and_then(purrdf_lex::json::Value::as_str)
        .unwrap_or_else(|| panic!("?x is bound in {document}"))
        .to_owned()
}

/// What a governed SELECT of one `?x` answered: the lexical form of `?x` (or `None`
/// when unbound) and the evidence's expression-error counts — or the call's error.
type GovernedAnswer = Result<(Option<String>, [u64; PURRDF_EXPRESSION_ERROR_CODE_COUNT]), String>;

fn governed_x(dataset: *const PurrdfDataset, query: &str) -> GovernedAnswer {
    let query = CString::new(query).expect("query C string");
    let mut governors = MaybeUninit::uninit();
    assert_eq!(
        unsafe { purrdf_query_governors_init(governors.as_mut_ptr()) },
        0
    );
    let governors = unsafe { governors.assume_init() };
    let mut outcome = -1;
    let mut kind = KIND_NONE;
    let mut rows = std::ptr::null_mut();
    let mut evidence = MaybeUninit::uninit();
    let mut partial = MaybeUninit::uninit();
    let mut error = std::ptr::null_mut();
    let status = unsafe {
        purrdf_query_governed(
            dataset,
            query.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
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
    };
    if status != PurrdfStatus::Ok as i32 {
        let (code, message) = take_error(error);
        assert_eq!(code, PurrdfStatus::QueryError as i32);
        return Err(message);
    }
    assert_eq!(outcome, PurrdfQueryOutcomeKind::Complete as i32);
    assert_eq!(kind, KIND_SOLUTIONS);
    let evidence = unsafe { evidence.assume_init() };
    assert_eq!(
        unsafe { purrdf_rowcursor_next(rows) },
        PurrdfStatus::Ok as i32
    );
    let mut view = MaybeUninit::<PurrdfTermView>::uninit();
    let mut bound = 0u8;
    assert_eq!(
        unsafe { purrdf_rowcursor_term(rows, 0, view.as_mut_ptr(), &raw mut bound) },
        PurrdfStatus::Ok as i32
    );
    let lexical = (bound == 1).then(|| {
        let view = unsafe { view.assume_init() };
        unsafe { view.lexical.as_str() }
            .expect("UTF-8 lexical form")
            .to_owned()
    });
    unsafe { purrdf_rowcursor_free(rows) };
    Ok((lexical, evidence.expression_errors))
}

/// `purrdf_update_governed` over `dataset`: `Ok` when applied, or the error message.
fn governed_update(dataset: *mut PurrdfDataset, request: &str) -> Result<(), String> {
    let request = CString::new(request).expect("request C string");
    let mut governors = MaybeUninit::uninit();
    assert_eq!(
        unsafe { purrdf_query_governors_init(governors.as_mut_ptr()) },
        0
    );
    let governors = unsafe { governors.assume_init() };
    let mut outcome = -1;
    let mut evidence = MaybeUninit::uninit();
    let mut error = std::ptr::null_mut();
    let status = unsafe {
        purrdf_update_governed(
            dataset,
            request.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            &raw const governors,
            &raw mut outcome,
            evidence.as_mut_ptr(),
            &raw mut error,
        )
    };
    if status == PurrdfStatus::Ok as i32 {
        assert_eq!(outcome, PurrdfUpdateOutcomeKind::Applied as i32);
        Ok(())
    } else {
        Err(take_error(error).1)
    }
}

#[test]
fn a_dataset_starts_at_eighteen_digits_toward_zero() {
    let dataset = dataset();
    assert_eq!(policy_of(dataset), "18:toward-zero");
    let third = bound_x(&query_json(dataset, "SELECT (1/3 AS ?x) {}").expect("1/3 answers"));
    assert_eq!(third, "0.333333333333333333");
    unsafe { purrdf_dataset_free(dataset) };
}

#[test]
fn an_exact_policy_answers_a_terminating_quotient_and_refuses_the_rest() {
    let dataset = dataset();
    set_policy(dataset, "exact").expect("exact parses");
    assert_eq!(policy_of(dataset), "exact");
    let eighth = query_json(dataset, "SELECT (1/8 AS ?x) {}").expect("1/8 terminates");
    assert_eq!(bound_x(&eighth), "0.125");
    let (code, message) =
        query_json(dataset, "SELECT (1/3 AS ?x) {}").expect_err("1/3 does not terminate");
    assert_eq!(code, PurrdfStatus::QueryError as i32);
    assert!(
        message.contains("FOAR0002"),
        "names err:FOAR0002: {message}"
    );

    // The governed entry runs under the same policy.
    let (x, errors) = governed_x(dataset, "SELECT (1/8 AS ?x) {}").expect("governed 1/8");
    assert_eq!(x.as_deref(), Some("0.125"));
    assert_eq!(errors, [0; PURRDF_EXPRESSION_ERROR_CODE_COUNT]);
    let message = governed_x(dataset, "SELECT (1/3 AS ?x) {}").expect_err("governed 1/3");
    assert!(
        message.contains("FOAR0002"),
        "names err:FOAR0002: {message}"
    );
    unsafe { purrdf_dataset_free(dataset) };
}

#[test]
fn a_scale_and_rounding_policy_rounds_the_quotient() {
    let dataset = dataset();
    set_policy(dataset, "5:half-even").expect("5:half-even parses");
    assert_eq!(policy_of(dataset), "5:half-even");
    let two_thirds = query_json(dataset, "SELECT (2/3 AS ?x) {}").expect("2/3 answers");
    assert_eq!(bound_x(&two_thirds), "0.66667");
    // `N` alone truncates toward zero.
    set_policy(dataset, "5").expect("5 parses");
    assert_eq!(policy_of(dataset), "5:toward-zero");
    let truncated = query_json(dataset, "SELECT (2/3 AS ?x) {}").expect("2/3 answers");
    assert_eq!(bound_x(&truncated), "0.66666");
    unsafe { purrdf_dataset_free(dataset) };
}

#[test]
fn an_unparseable_policy_is_refused_and_leaves_the_previous_one_in_force() {
    let dataset = dataset();
    set_policy(dataset, "exact").expect("exact parses");
    for refused in ["5:sideways", "", "exactly", "-1", "5:"] {
        let (code, message) = set_policy(dataset, refused).expect_err(refused);
        assert_eq!(code, PurrdfStatus::InvalidArgument as i32, "{refused:?}");
        assert!(
            message.contains("division policy"),
            "{refused:?}: {message}"
        );
        assert_eq!(policy_of(dataset), "exact", "{refused:?} changed nothing");
    }
    // Still exact: 1/3 is still refused and 1/8 still answers.
    assert!(query_json(dataset, "SELECT (1/3 AS ?x) {}").is_err());
    let eighth = query_json(dataset, "SELECT (1/8 AS ?x) {}").expect("1/8 terminates");
    assert_eq!(bound_x(&eighth), "0.125");
    // The valid neighbours of the refused texts are accepted.
    for accepted in ["5:half-even", "0", "18:toward-zero", "exact"] {
        set_policy(dataset, accepted).expect(accepted);
    }
    unsafe { purrdf_dataset_free(dataset) };
}

#[test]
fn a_null_policy_is_a_null_pointer() {
    let dataset = dataset();
    let mut error = std::ptr::null_mut();
    let status =
        unsafe { purrdf_dataset_set_division_policy(dataset, std::ptr::null(), &raw mut error) };
    assert_eq!(status, PurrdfStatus::NullPointer as i32);
    assert_eq!(take_error(error).0, PurrdfStatus::NullPointer as i32);
    assert_eq!(policy_of(dataset), "18:toward-zero");
    unsafe { purrdf_dataset_free(dataset) };
}

#[test]
fn a_governed_update_runs_under_the_policy() {
    let dataset = dataset();
    set_policy(dataset, "exact").expect("exact parses");
    let message = governed_update(
        dataset,
        "INSERT { <http://example.org/s> <http://example.org/third> ?x } \
         WHERE { BIND(1/3 AS ?x) }",
    )
    .expect_err("1/3 does not terminate");
    assert!(
        message.contains("FOAR0002"),
        "names err:FOAR0002: {message}"
    );
    governed_update(
        dataset,
        "INSERT { <http://example.org/s> <http://example.org/eighth> ?x } \
         WHERE { BIND(1/8 AS ?x) }",
    )
    .expect("1/8 inserts");
    let ask = query_json(
        dataset,
        "ASK { <http://example.org/s> <http://example.org/eighth> 0.125 \
         FILTER NOT EXISTS { <http://example.org/s> <http://example.org/third> ?any } }",
    )
    .expect("ASK answers");
    assert!(ask.contains("\"boolean\":true"), "{ask}");
    // An UPDATE replaces the snapshot but keeps the handle's policy.
    assert_eq!(policy_of(dataset), "exact");
    unsafe { purrdf_dataset_free(dataset) };
}

#[test]
fn an_entailment_query_runs_under_the_policy() {
    let dataset = dataset();
    let run = |query: &str| -> Result<i32, (i32, String)> {
        let query = CString::new(query).expect("query");
        let regime = CString::new("rdfs").expect("regime");
        let program = CString::new("").expect("program");
        let mut governors = MaybeUninit::uninit();
        assert_eq!(
            unsafe { purrdf_query_governors_init(governors.as_mut_ptr()) },
            0
        );
        let governors = unsafe { governors.assume_init() };
        let mut outcome = -1;
        let mut kind = KIND_NONE;
        let mut rows = std::ptr::null_mut();
        let mut evidence = MaybeUninit::uninit();
        let mut partial = MaybeUninit::uninit();
        let mut report = std::ptr::null_mut();
        let mut error = std::ptr::null_mut();
        let status = unsafe {
            purrdf_query_entailment_governed(
                dataset,
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
            )
        };
        if status != PurrdfStatus::Ok as i32 {
            return Err(take_error(error));
        }
        unsafe {
            purrdf_rowcursor_free(rows);
            purrdf_buffer_free(report);
        }
        Ok(outcome)
    };
    set_policy(dataset, "exact").expect("exact parses");
    assert_eq!(
        run("SELECT (1/8 AS ?x) {}"),
        Ok(PurrdfEntailmentQueryOutcomeKind::Complete as i32)
    );
    let (code, message) = run("SELECT (1/3 AS ?x) {}").expect_err("1/3 does not terminate");
    assert_eq!(code, PurrdfStatus::QueryError as i32);
    assert!(
        message.contains("FOAR0002"),
        "names err:FOAR0002: {message}"
    );
    unsafe { purrdf_dataset_free(dataset) };
}

#[test]
fn evidence_counts_an_absorbed_division_by_zero_per_code() {
    let dataset = dataset();
    let (x, errors) = governed_x(dataset, "SELECT (1/0 AS ?x) {}").expect("1/0 is unbound");
    assert_eq!(x, None, "an expression error leaves ?x unbound");
    let mut expected = [0; PURRDF_EXPRESSION_ERROR_CODE_COUNT];
    expected[PurrdfExpressionErrorCode::Foar0001 as usize] = 1;
    assert_eq!(errors, expected, "1/0 is one err:FOAR0001");
    // The valid neighbour raises nothing.
    let (x, errors) = governed_x(dataset, "SELECT (1/2 AS ?x) {}").expect("1/2 answers");
    assert_eq!(x.as_deref(), Some("0.5"));
    assert_eq!(errors, [0; PURRDF_EXPRESSION_ERROR_CODE_COUNT]);
    unsafe { purrdf_dataset_free(dataset) };
}

/// The discriminants are `ErrorCode::ALL`'s positions, so index `i` of the carrier is the
/// code a C host reads at `i`.
#[test]
fn expression_error_discriminants_follow_the_kernel_code_order() {
    let discriminants = [
        PurrdfExpressionErrorCode::Foar0001,
        PurrdfExpressionErrorCode::Foar0002,
        PurrdfExpressionErrorCode::Foca0001,
        PurrdfExpressionErrorCode::Foca0002,
        PurrdfExpressionErrorCode::Foca0003,
        PurrdfExpressionErrorCode::Foca0006,
        PurrdfExpressionErrorCode::Forg0001,
        PurrdfExpressionErrorCode::Xpty0004,
    ];
    for (index, (code, discriminant)) in purrdf_rs::xsd::ErrorCode::ALL
        .into_iter()
        .zip(discriminants)
        .enumerate()
    {
        assert_eq!(discriminant as usize, index);
        assert_eq!(
            format!("{discriminant:?}").to_ascii_uppercase(),
            code.local_name()
        );
    }
}
