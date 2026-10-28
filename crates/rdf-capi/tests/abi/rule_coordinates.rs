// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The actual C rule boundaries own supplied coordinates and preserve refusals.

use std::ffi::{CStr, CString};
use std::ptr::{null, null_mut};

use purrdf::shacl::{
    purrdf_shacl_apply_rules, purrdf_shacl_apply_rules_xpath_regex,
    purrdf_shacl_entail_to_ntriples, purrdf_shacl_entail_to_ntriples_xpath_regex,
};

use super::{PurrdfBuffer, PurrdfStatus, buffer_bytes, purrdf_buffer_free, purrdf_error_free};

const NOW: &str = "-0001-02-03T04:05:06.98765432109876543210987654321-04:00";
const SHAPES: &str = r#"@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
ex:S a sh:NodeShape ; sh:targetNode ex:s ; sh:rule [ a sh:SPARQLRule ;
 sh:runOnce true ; sh:construct """CONSTRUCT {
 <http://example.org/s> <http://example.org/time> ?t ; <http://example.org/id> ?u }
 WHERE { BIND(NOW() AS ?t) BIND(UUID() AS ?u) FILTER(REGEX("ok", "^ok$")) }""" ] ."#;
const SRL: &str = "PREFIX : <http://example.org/> RULE { :s :time ?t ; :id ?u } \
 WHERE { SET(?t := NOW()) SET(?u := UUID()) FILTER(REGEX(\"ok\", \"^ok$\")) }";

/// Six routes: the two entail exports, then both exports for each rule source.
fn execute(
    route: usize,
    time: Option<&CStr>,
    seed: Option<&u64>,
    valid_document: bool,
    regex: &CStr,
    max_facts: Option<u64>,
) -> Result<String, i32> {
    let sentinel = std::ptr::NonNull::<PurrdfBuffer>::dangling().as_ptr();
    let mut output = sentinel;
    let mut proof = sentinel;
    let mut diagnostics = sentinel;
    let mut error = null_mut();
    let mut receipt = crate::receipt::Output::default();
    let status = {
        let shapes = CString::new(if valid_document {
            SHAPES
        } else {
            "invalid shapes"
        })
        .unwrap();
        let srl = CString::new(if valid_document { SRL } else { "invalid rules" }).unwrap();
        // Copy the caller's inputs here, then destroy them before buffer inspection.
        let time = time.map(CStr::to_owned);
        let seed = seed.copied().map(Box::new);
        let now = time.as_ref().map_or(null(), |time| time.as_ptr());
        let seed = seed.as_deref().map_or(null(), std::ptr::from_ref);
        let facts = max_facts.as_ref().map_or(null(), std::ptr::from_ref);
        unsafe {
            match route {
                0 => purrdf_shacl_entail_to_ntriples(
                    shapes.as_ptr(),
                    null(),
                    null(),
                    c"".as_ptr(),
                    null(),
                    null(),
                    0,
                    null(),
                    null(),
                    facts,
                    null(),
                    now,
                    seed,
                    &raw mut receipt.0,
                    &raw mut output,
                    &raw mut diagnostics,
                    &raw mut error,
                ),
                1 => purrdf_shacl_entail_to_ntriples_xpath_regex(
                    shapes.as_ptr(),
                    null(),
                    null(),
                    c"".as_ptr(),
                    null(),
                    null(),
                    0,
                    null(),
                    null(),
                    facts,
                    null(),
                    regex.as_ptr(),
                    now,
                    seed,
                    &raw mut receipt.0,
                    &raw mut output,
                    &raw mut diagnostics,
                    &raw mut error,
                ),
                2 | 4 => purrdf_shacl_apply_rules(
                    c"".as_ptr(),
                    if route == 2 { shapes.as_ptr() } else { null() },
                    null(),
                    null(),
                    if route == 4 { srl.as_ptr() } else { null() },
                    null(),
                    null(),
                    null(),
                    facts,
                    null(),
                    null(),
                    null(),
                    0,
                    now,
                    seed,
                    &raw mut receipt.0,
                    &raw mut output,
                    &raw mut proof,
                    &raw mut diagnostics,
                    &raw mut error,
                ),
                3 | 5 => purrdf_shacl_apply_rules_xpath_regex(
                    c"".as_ptr(),
                    if route == 3 { shapes.as_ptr() } else { null() },
                    null(),
                    null(),
                    if route == 5 { srl.as_ptr() } else { null() },
                    null(),
                    null(),
                    null(),
                    facts,
                    null(),
                    null(),
                    null(),
                    0,
                    regex.as_ptr(),
                    now,
                    seed,
                    &raw mut receipt.0,
                    &raw mut output,
                    &raw mut proof,
                    &raw mut diagnostics,
                    &raw mut error,
                ),
                _ => unreachable!("six C rule routes"),
            }
        }
    };
    if status != PurrdfStatus::Ok as i32 {
        assert!(!error.is_null());
        assert_eq!(output, sentinel);
        assert_eq!(proof, sentinel);
        assert_eq!(diagnostics, sentinel);
        unsafe { purrdf_error_free(error) };
        return Err(status);
    }
    assert!(error.is_null());
    unsafe {
        let receipt = String::from_utf8(buffer_bytes(receipt.0)).unwrap();
        assert!(receipt.contains("executionIdentity"));
        if let Some(time) = time {
            assert!(receipt.contains(time.to_str().unwrap()));
        }
        assert_eq!(buffer_bytes(diagnostics), b"");
        purrdf_buffer_free(diagnostics);
        if route >= 2 {
            assert!(
                String::from_utf8(buffer_bytes(proof))
                    .unwrap()
                    .contains("derived")
            );
            purrdf_buffer_free(proof);
        }
        let text = String::from_utf8(buffer_bytes(output)).unwrap();
        purrdf_buffer_free(output);
        Ok(text)
    }
}

#[test]
fn rule_coordinates_cross_every_c_export_and_outlive_inputs() {
    let time = CString::new(NOW).unwrap();
    for route in 0..6 {
        for law in [c"xpath-2.0-2010-12-14", c"xpath-3.1-2017-03-21"] {
            let first = execute(route, Some(&time), Some(&0), true, law, None).unwrap();
            assert!(first.contains(NOW));
            assert!(first.contains("urn:uuid:"));
            assert_eq!(
                first,
                execute(route, Some(&time), Some(&0), true, law, None).unwrap()
            );
            let maximum = execute(route, Some(&time), Some(&u64::MAX), true, law, None).unwrap();
            assert!(maximum.contains(NOW));
            assert_ne!(first, maximum);
        }
        assert!(
            execute(
                route,
                Some(&time),
                None,
                true,
                c"xpath-3.1-2017-03-21",
                None
            )
            .unwrap()
            .contains(NOW)
        );
        let omitted = execute(route, None, Some(&0), true, c"xpath-3.1-2017-03-21", None).unwrap();
        assert!(!omitted.contains(NOW));
        assert!(omitted.contains("urn:uuid:"));
    }
}

#[test]
fn rule_coordinate_refusals_preserve_outputs_and_configuration_precedence() {
    for route in 0..6 {
        let law = c"xpath-3.1-2017-03-21";
        for (time, status) in [
            (c"invalid", PurrdfStatus::InvalidArgument),
            (c"\xff", PurrdfStatus::InvalidUtf8),
        ] {
            assert_eq!(
                execute(route, Some(time), Some(&0), false, law, None),
                Err(status as i32)
            );
        }
        if route % 2 == 1 {
            assert_eq!(
                execute(route, Some(c"\xff"), None, false, c"invalid law", None),
                Err(PurrdfStatus::InvalidArgument as i32)
            );
        }
        let time = CString::new(NOW).unwrap();
        assert!(execute(route, Some(&time), Some(&0), true, law, Some(0)).is_err());
        assert!(execute(route, Some(&time), Some(&0), true, law, None).is_ok());
    }
}
