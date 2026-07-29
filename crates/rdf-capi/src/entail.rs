// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf_entail_*`: SPARQL entailment-**regime** materialization and the rule
//! inventories, over the shared [`PurrdfBuffer`].
//!
//! # Not to be confused with [`crate::shacl`]
//!
//! The two modules sit beside each other and both say "entail":
//!
//! * [`purrdf_shacl_entail_to_ntriples`](crate::shacl::purrdf_shacl_entail_to_ntriples)
//!   is **SHACL-AF `sh:rule`** entailment. It needs a *shapes* graph and applies
//!   the rules that graph declares.
//! * [`purrdf_entail_materialize_to_nquads`] is **SPARQL entailment-regime**
//!   materialization (`simple` / `rdf` / `rdfs` / `owl-rl`). It takes no shapes
//!   at all: it closes a document under the regime's own specification rule
//!   table.
//!
//! # One boundary, three hosts
//!
//! Nothing here reimplements the parse → close → serialize sequence. Every entry
//! point routes through [`purrdf_validate::regime`], the same string boundary the
//! WASM and PyO3 hosts call, so a byte difference between the three hosts is one
//! shared golden vector failing rather than three surfaces that quietly stopped
//! agreeing. This module adds the C-ABI byte framing and nothing else.
//!
//! # Ownership
//!
//! Exactly as
//! [`purrdf_shacl_entail_to_ntriples`](crate::shacl::purrdf_shacl_entail_to_ntriples):
//! each `PurrdfBuffer` the library hands out is released with
//! `purrdf_buffer_free`, and its bytes are read (borrowed, never freed by the
//! caller) with `purrdf_buffer_data`. `purrdf_entail_materialize_to_nquads`
//! hands out **two** independent buffers, so it needs **two** frees; on any error
//! it writes neither out-param and there is nothing to free.

use std::os::raw::c_char;

use purrdf_validate::regime::{
    implemented_rules_string, materialize_to_nquads_string, rules_string,
};

use crate::buffer::PurrdfBuffer;
use crate::cstr_to_str;
use crate::error::PurrdfError;
use crate::status::PurrdfStatus;

/// Close `document` (N-Quads, which accepts N-Triples) under the regime spelled
/// `regime`, returning the canonical N-Quads closure and the rendered reasoning
/// report as two byte vectors. Native-testable, pointer-free core.
///
/// The parse → close → canonicalize → render sequence lives in
/// [`materialize_to_nquads_string`]; this only adds the C-ABI byte framing.
fn materialize_to_nquads_bytes(document: &str, regime: &str) -> Result<(Vec<u8>, Vec<u8>), String> {
    let (nquads, report) = materialize_to_nquads_string(regime, document)?.into_parts();
    Ok((nquads.into_bytes(), report.into_bytes()))
}

/// Materialize an RDF document under a SPARQL entailment regime.
///
/// `document` is parsed as N-Quads, which accepts an N-Triples document
/// unchanged, so a document that names a graph keeps naming it. `regime` is one
/// of `simple`, `rdf`, `rdfs`, `owl-rl`, `owl-direct`, `rif`, `d` — the same
/// spellings the CLI, WASM and the Python surface accept. Exactly two of them —
/// `owl-direct` and `rif` — cannot be forward-materialized, and are refused with a
/// message naming the five that can (`simple`, `rdf`, `rdfs`, `owl-rl`, `d`).
///
/// On success `*out_nquads` receives the canonical (RDFC-1.0) N-Quads closure —
/// every input quad plus every triple the regime's implemented rules infer — and
/// `*out_report` receives the byte-stable rendered reasoning report, which names
/// which rules fired and how often, which specification rules did NOT fire, which
/// constructs were left at a boundary, the evaluation budget, and the calculus's
/// contract hash. **Free BOTH with `purrdf_buffer_free`.** The report is not
/// optional: all seventy-eight OWL 2 RL rules now run, so a caller that reported
/// "OWL-RL entailment" without saying which CONSTRUCTS were left at a boundary
/// would be making exactly the overclaim the report exists to prevent — a
/// complete rule table is not a complete closure.
///
/// On any error neither out-param is written, so there is nothing to free.
///
/// # Safety
/// `document` and `regime` must be non-null, NUL-terminated C strings;
/// `out_nquads` and `out_report` must be writable pointers; `out_error` must be
/// null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_entail_materialize_to_nquads(
    document: *const c_char,
    regime: *const c_char,
    out_nquads: *mut *mut PurrdfBuffer,
    out_report: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: the caller's contract above — `document`/`regime` are NUL-terminated
    // C strings (or null, checked here), and the three out-pointers are writable
    // (or null, checked here). The two `*out_* =` writes happen only after every
    // fallible step has succeeded, so a failing call leaves both untouched and
    // hands out no buffer the caller would have to free.
    unsafe {
        ffi_try!(out_error, {
            if document.is_null()
                || regime.is_null()
                || out_nquads.is_null()
                || out_report.is_null()
            {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_entail_materialize_to_nquads",
                ));
            }
            let document = cstr_to_str(document)?;
            let regime = cstr_to_str(regime)?;
            let (nquads, report) = materialize_to_nquads_bytes(document, regime)
                .map_err(|message| PurrdfError::new(PurrdfStatus::ParseError, message))?;
            *out_nquads = PurrdfBuffer::into_raw(nquads);
            *out_report = PurrdfBuffer::into_raw(report);
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// The rule table `regime` is *defined by*, one name per line. Native-testable,
/// pointer-free core.
fn rules_bytes(regime: &str) -> Result<Vec<u8>, String> {
    Ok(rules_string(regime)?.into_bytes())
}

/// Write the rule table the specification *defines* `regime` by — one canonical
/// rule name per newline-terminated line, in specification table order — to
/// `*out_buffer` (free with `purrdf_buffer_free`).
///
/// An empty buffer for a regime with no rule table (`simple`, and the two that
/// are not forward-materializable). `owl-rl` yields all 78 rules of OWL 2
/// Profiles §4.3 Tables 4–9 whether or not this build fires them — that is the
/// point: diff it against `purrdf_entail_implemented_rules` to measure the gap.
///
/// # Safety
/// `regime` must be a non-null, NUL-terminated C string; `out_buffer` must be a
/// writable pointer; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_entail_rules(
    regime: *const c_char,
    out_buffer: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: as `purrdf_entail_materialize_to_nquads` — `regime` is a
    // NUL-terminated C string (or null, checked here), `out_buffer`/`out_error`
    // are writable (or null, checked here), and `*out_buffer` is written only
    // after the lookup has succeeded.
    unsafe {
        ffi_try!(out_error, {
            if regime.is_null() || out_buffer.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_entail_rules",
                ));
            }
            let regime = cstr_to_str(regime)?;
            let bytes = rules_bytes(regime)
                .map_err(|message| PurrdfError::new(PurrdfStatus::ParseError, message))?;
            *out_buffer = PurrdfBuffer::into_raw(bytes);
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// The subset of the rule table this build fires, one name per line.
/// Native-testable, pointer-free core.
fn implemented_rules_bytes(regime: &str) -> Result<Vec<u8>, String> {
    Ok(implemented_rules_string(regime)?.into_bytes())
}

/// Write the subset of `purrdf_entail_rules(regime)` this build's chase actually
/// fires today to `*out_buffer` (free with `purrdf_buffer_free`).
///
/// `purrdf_entail_rules(r)` minus `purrdf_entail_implemented_rules(r)` is the
/// regime's measurable gap — the same set the rendered report's `missing` lines
/// name.
///
/// # Safety
/// `regime` must be a non-null, NUL-terminated C string; `out_buffer` must be a
/// writable pointer; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_entail_implemented_rules(
    regime: *const c_char,
    out_buffer: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: identical contract to `purrdf_entail_rules` above.
    unsafe {
        ffi_try!(out_error, {
            if regime.is_null() || out_buffer.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_entail_implemented_rules",
                ));
            }
            let regime = cstr_to_str(regime)?;
            let bytes = implemented_rules_bytes(regime)
                .map_err(|message| PurrdfError::new(PurrdfStatus::ParseError, message))?;
            *out_buffer = PurrdfBuffer::into_raw(bytes);
            Ok(PurrdfStatus::Ok)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use purrdf_validate::regime::check_regime_golden_vectors;

    use super::*;

    /// `A ⊑ B` and one typed instance — enough for `rdfs9` to re-type it.
    const SCHEMA: &str = "\
<http://example.org/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/B> .
<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> .
";

    /// The C-ABI host's leg of the tri-host assertion.
    ///
    /// The `purrdf-validate` test and the WASM host's `entailCheckGoldenVectors`
    /// call this SAME checker over the SAME committed artifact, so a host that
    /// produces different bytes fails here in the same words.
    #[test]
    fn the_golden_vector_matches() {
        check_regime_golden_vectors().expect("the regime golden vector");
    }

    #[test]
    fn materialize_emits_closure_and_report() {
        let (nquads, report) = materialize_to_nquads_bytes(SCHEMA, "rdfs").expect("rdfs closure");
        let nquads = String::from_utf8(nquads).expect("utf8");
        let report = String::from_utf8(report).expect("utf8");
        assert!(nquads.contains(
            "<http://example.org/x> \
             <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/B> ."
        ));
        assert!(report.starts_with("purrdf-reasoning-report 1\n"));
        // The report says what the run could NOT do. Asserted as the invariant
        // rather than as a `sound-incomplete <n>` literal: the count moves every
        // time a rule lands, and the honesty gate does not.
        assert!(report.contains("\ncompleteness "));
        assert!(report.contains("\nboundary "));
        assert!(report.ends_with("overclaims false\n"));
    }

    #[test]
    fn an_unknown_regime_names_the_accepted_set() {
        for error in [
            materialize_to_nquads_bytes(SCHEMA, "RDFS").expect_err("case-sensitive"),
            rules_bytes("rdfs-plus").expect_err("unknown"),
            implemented_rules_bytes("rdfs-plus").expect_err("unknown"),
        ] {
            assert!(error.contains("accepted: simple, rdf, rdfs"), "{error}");
        }
    }

    #[test]
    fn a_non_materializable_regime_is_refused_by_name() {
        for regime in ["rif", "owl-direct"] {
            let error = materialize_to_nquads_bytes(SCHEMA, regime).expect_err("unsupported");
            assert!(
                error.contains("materializable regimes: simple, rdf, rdfs, owl-rl, d"),
                "{error}"
            );
        }
        // `d` is on the other side of that line: the C ABI materializes it too.
        assert!(materialize_to_nquads_bytes(SCHEMA, "d").is_ok());
    }

    #[test]
    fn a_malformed_document_is_an_error() {
        assert!(materialize_to_nquads_bytes("this is not n-quads\n", "rdfs").is_err());
    }

    #[test]
    fn the_inventories_are_the_specification_tables() {
        let rules = String::from_utf8(rules_bytes("owl-rl").expect("known")).expect("utf8");
        assert_eq!(rules.lines().count(), 78);
        let rdfs = String::from_utf8(rules_bytes("rdfs").expect("known")).expect("utf8");
        assert_eq!(rdfs.lines().count(), 18);
        let fired =
            String::from_utf8(implemented_rules_bytes("rdfs").expect("known")).expect("utf8");
        // The gap is MEASURED, never asserted as a literal: the implemented set is
        // a subsequence of the defined one, so the two ways of counting the gap
        // must agree — and it is legitimately empty once the table is complete.
        let missing = rdfs
            .lines()
            .filter(|rule| !fired.lines().any(|f| f == *rule))
            .count();
        assert_eq!(missing, rdfs.lines().count() - fired.lines().count());
        assert!(rules_bytes("simple").expect("known").is_empty());
    }

    // ── The pointer surface ─────────────────────────────────────────────────

    /// Read a buffer handle's bytes as a `String` and free the handle.
    ///
    /// # Safety
    /// `buf` must be a live buffer handle holding UTF-8, not already freed.
    unsafe fn take(buf: *mut PurrdfBuffer) -> String {
        // SAFETY: the caller's contract — `buf` is a live handle. `purrdf_buffer_data`
        // borrows its bytes for as long as the handle lives; the slice is copied
        // into an owned `String` BEFORE the handle is freed.
        unsafe {
            let mut ptr: *const u8 = std::ptr::null();
            let mut len: usize = 0;
            assert_eq!(
                crate::buffer::purrdf_buffer_data(buf, &raw mut ptr, &raw mut len),
                PurrdfStatus::Ok as i32
            );
            let text = String::from_utf8(std::slice::from_raw_parts(ptr, len).to_vec())
                .expect("the boundary emits UTF-8");
            crate::buffer::purrdf_buffer_free(buf);
            text
        }
    }

    #[test]
    fn the_pointer_surface_hands_out_two_buffers() {
        let document = CString::new(SCHEMA).expect("no interior NUL");
        let regime = CString::new("rdfs").expect("no interior NUL");
        let mut nquads: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut report: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        // SAFETY: both C strings are live for the call, and the three out-pointers
        // address live, writable locals.
        unsafe {
            assert_eq!(
                purrdf_entail_materialize_to_nquads(
                    document.as_ptr(),
                    regime.as_ptr(),
                    &raw mut nquads,
                    &raw mut report,
                    &raw mut error,
                ),
                PurrdfStatus::Ok as i32
            );
            assert!(error.is_null());
            assert!(take(nquads).contains(
                "<http://example.org/x> \
                 <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/B> ."
            ));
            assert!(take(report).starts_with("purrdf-reasoning-report 1\n"));
        }
    }

    #[test]
    fn the_pointer_surface_hands_out_the_inventories() {
        let regime = CString::new("rdfs").expect("no interior NUL");
        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        // SAFETY: `regime` is live for both calls; `buffer`/`error` address live,
        // writable locals, and each handed-out buffer is freed by `take`.
        unsafe {
            assert_eq!(
                purrdf_entail_rules(regime.as_ptr(), &raw mut buffer, &raw mut error),
                PurrdfStatus::Ok as i32
            );
            // 18 is the SPECIFICATION's count and does not move.
            assert_eq!(take(buffer).lines().count(), 18);
            assert_eq!(
                purrdf_entail_implemented_rules(regime.as_ptr(), &raw mut buffer, &raw mut error),
                PurrdfStatus::Ok as i32
            );
            // The implemented count DOES move as rules land, so it is bounded, not
            // pinned: never empty, never more than the table it is a subset of.
            let implemented = take(buffer).lines().count();
            assert!((1..=18).contains(&implemented), "{implemented}");
            assert!(error.is_null());
        }
    }

    #[test]
    fn a_failing_call_writes_no_buffer_to_free() {
        let document = CString::new(SCHEMA).expect("no interior NUL");
        let regime = CString::new("owl-direct").expect("no interior NUL");
        let mut nquads: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut report: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        // SAFETY: both C strings are live for the call, and the three out-pointers
        // address live, writable locals; the error handle is freed below.
        unsafe {
            assert_eq!(
                purrdf_entail_materialize_to_nquads(
                    document.as_ptr(),
                    regime.as_ptr(),
                    &raw mut nquads,
                    &raw mut report,
                    &raw mut error,
                ),
                PurrdfStatus::ParseError as i32
            );
            // Neither out-param was written, so the caller has nothing to free.
            assert!(nquads.is_null());
            assert!(report.is_null());
            assert!(!error.is_null());
            crate::error::purrdf_error_free(error);
        }
    }

    #[test]
    fn null_arguments_are_refused_not_dereferenced() {
        let mut nquads: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut report: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        // SAFETY: every pointer passed is either null (the case under test) or
        // addresses a live, writable local; no error channel is requested, so the
        // status code is the whole observable result.
        unsafe {
            assert_eq!(
                purrdf_entail_materialize_to_nquads(
                    std::ptr::null(),
                    std::ptr::null(),
                    &raw mut nquads,
                    &raw mut report,
                    std::ptr::null_mut(),
                ),
                PurrdfStatus::NullPointer as i32
            );
            assert_eq!(
                purrdf_entail_rules(std::ptr::null(), &raw mut buffer, std::ptr::null_mut()),
                PurrdfStatus::NullPointer as i32
            );
            assert_eq!(
                purrdf_entail_implemented_rules(
                    std::ptr::null(),
                    &raw mut buffer,
                    std::ptr::null_mut(),
                ),
                PurrdfStatus::NullPointer as i32
            );
        }
    }
}
