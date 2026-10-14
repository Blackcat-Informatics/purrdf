// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Actual allocation measurements for composite blank-label ingress and decoding.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_core::BlankScope;
use purrdf_core::blank_label::{
    LabelAlphabet, decode_blank_label, decode_blank_label_with_memory, encode_blank_label,
};
use purrdf_core::cdt_blank::{
    BlankBinding, CdtBlankError, bind_cdt_blank_labels, bind_cdt_blank_labels_unchecked,
    bind_cdt_blank_labels_unchecked_with_memory, bind_cdt_blank_labels_with_memory,
};
use purrdf_lex::allocation::{Admission, Memory, StorageError};
use std::borrow::Cow;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[derive(Debug)]
struct Account {
    limit: usize,
    live: usize,
    peak: usize,
    refused: Option<usize>,
}
impl Account {
    const fn new(limit: usize) -> Self {
        Self {
            limit,
            live: 0,
            peak: 0,
            refused: None,
        }
    }
}
impl Admission for Account {
    fn resize(&mut self, live: usize) -> Result<(), StorageError> {
        if live > self.limit {
            self.refused = Some(live);
            return Err(StorageError::AdmissionFailed);
        }
        self.live = live;
        self.peak = self.peak.max(live);
        Ok(())
    }
}

fn nested_fixture() -> String {
    let encoded = encode_blank_label("a é.b", BlankScope(29), LabelAlphabet::BlankNodeLabel);
    // Escaped datatypes and escaped content exercise decoding and root maps.
    let inner = format!("[_:{encoded}, _:plain, <urn:\\u00e9>]");
    let quoted = inner.replace('\\', "\\\\").replace('\'', "\\'");
    format!(
        "[_:root, '{quoted}'^^<{}>, <<( _:s <urn:p> _:o )>>]",
        purrdf_cdt::CDT_LIST.replace("List", "Li\\u0073t"),
    )
}

#[test]
fn checked_nested_binding_accounts_for_grammar_and_only_retains_output() {
    let input = nested_fixture();
    let binding = BlankBinding::Ambient(BlankScope(7));
    let expected = bind_cdt_blank_labels(&input, purrdf_cdt::CDT_LIST, binding).unwrap();
    let mut account = Account::new(2_000_000);
    let mut memory = Memory::new(&mut account);
    let window = CurrentThreadWindow::open();
    let output =
        bind_cdt_blank_labels_with_memory(&input, purrdf_cdt::CDT_LIST, binding, &mut memory)
            .unwrap();
    let measured = window.close();
    assert_eq!(output, expected);
    assert_eq!(
        usize::try_from(measured.retained_bytes).unwrap(),
        memory.admitted_bytes()
    );
    let Cow::Owned(output) = output else {
        panic!("ambient binding changes labels")
    };
    assert_eq!(memory.admitted_bytes(), output.capacity());
    memory.release_string(output).unwrap();
    assert_eq!(memory.admitted_bytes(), 0);
    assert!(usize::try_from(measured.peak_working_bytes).unwrap() <= account.peak);

    for limit in [0, 1, 32, 128, 512, 1024] {
        let mut account = Account::new(limit);
        let mut memory = Memory::new(&mut account);
        let window = CurrentThreadWindow::open();
        let result =
            bind_cdt_blank_labels_with_memory(&input, purrdf_cdt::CDT_LIST, binding, &mut memory);
        let measured = window.close();
        assert!(matches!(
            result,
            Err(CdtBlankError::Storage(StorageError::AdmissionFailed))
        ));
        assert_eq!(memory.admitted_bytes(), 0);
        assert_eq!(measured.retained_bytes, 0);
        if limit == 0 {
            assert_eq!(measured.allocations, 0);
        }
    }
}

#[test]
fn checked_malformed_payload_keeps_exact_original_grant_until_drop() {
    let nested = format!("['[<relative>]'^^<{}>]", purrdf_cdt::CDT_LIST);
    for (input, datatype) in [
        ("[<relative>]", purrdf_cdt::CDT_LIST),
        ("{\"a\": 1, \"a\": 2}", purrdf_cdt::CDT_MAP),
        (nested.as_str(), purrdf_cdt::CDT_LIST),
    ] {
        let mut account = Account::new(2_000_000);
        let mut memory = Memory::new(&mut account);
        let window = CurrentThreadWindow::open();
        let error = bind_cdt_blank_labels_with_memory(
            input,
            datatype,
            BlankBinding::Ambient(BlankScope(7)),
            &mut memory,
        )
        .unwrap_err();
        let measured = window.close();
        assert!(matches!(
            &error,
            CdtBlankError::Malformed {
                source: purrdf_cdt::CdtError::NotAbsoluteIri { .. }
                    | purrdf_cdt::CdtError::DuplicateMapKey { .. },
                ..
            }
        ));
        let bytes = error.owned_bytes().unwrap();
        assert!(
            bytes > 0,
            "the original lexical diagnostic owns its payload"
        );
        assert_eq!(memory.admitted_bytes(), bytes);
        assert_eq!(usize::try_from(measured.retained_bytes).unwrap(), bytes);
        drop(error);
        memory.release_bytes(bytes).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
    }
}

#[test]
fn nested_binding_peak_and_surviving_output_use_original_admission() {
    let input = nested_fixture();
    for binding in [
        BlankBinding::Ambient(BlankScope(7)),
        BlankBinding::Decoded(LabelAlphabet::BlankNodeLabel),
    ] {
        let expected = bind_cdt_blank_labels_unchecked(&input, purrdf_cdt::CDT_LIST, binding);
        let mut account = Account::new(2_000_000);
        let window = CurrentThreadWindow::open();
        let mut memory = Memory::new(&mut account);
        let output = bind_cdt_blank_labels_unchecked_with_memory(
            &input,
            purrdf_cdt::CDT_LIST,
            binding,
            &mut memory,
        )
        .unwrap();
        let retained = memory.admitted_bytes();
        let measured = window.close();
        assert_eq!(output.as_ref(), expected.as_ref());
        assert_eq!(usize::try_from(measured.retained_bytes).unwrap(), retained);
        let capacity = match &output {
            Cow::Owned(text) => text.capacity(),
            Cow::Borrowed(_) => 0,
        };
        assert_eq!(retained, capacity, "only the original destination survives");
        let end = CurrentThreadWindow::open();
        if let Cow::Owned(text) = output {
            memory.release_string(text).unwrap();
        }
        assert_eq!(memory.admitted_bytes(), 0);
        assert!(usize::try_from(measured.peak_working_bytes).unwrap() <= account.peak);
        assert_eq!(
            end.close().retained_bytes,
            -(i64::try_from(capacity).unwrap())
        );
    }
}

#[test]
fn plain_and_non_composite_binding_preserve_borrowed_bytes() {
    for (input, datatype) in [
        ("[_:plain, _:é]", purrdf_cdt::CDT_LIST),
        ("anything _:label", "urn:ordinary"),
        ("['[_:hidden]'@en, _:visible]", purrdf_cdt::CDT_LIST),
        ("[unterminated <iri _:ignored", purrdf_cdt::CDT_LIST),
    ] {
        let binding = BlankBinding::Decoded(LabelAlphabet::BlankNodeLabel);
        let expected = bind_cdt_blank_labels_unchecked(input, datatype, binding);
        let mut account = Account::new(100_000);
        let window = CurrentThreadWindow::open();
        let mut memory = Memory::new(&mut account);
        let output =
            bind_cdt_blank_labels_unchecked_with_memory(input, datatype, binding, &mut memory)
                .unwrap();
        assert_eq!(output, expected);
        assert!(matches!(output, Cow::Borrowed(_)));
        assert_eq!(memory.admitted_bytes(), 0);
        let measured = window.close();
        assert_eq!(measured.retained_bytes, 0);
        assert!(usize::try_from(measured.peak_working_bytes).unwrap() <= account.peak);
        if datatype == "urn:ordinary" {
            assert_eq!(measured.allocations, 0);
        }
    }
}

#[test]
fn physical_refusals_never_become_unmodified_composite_success() {
    let input = nested_fixture().repeat(16);
    let mut healthy = Account::new(2_000_000);
    let mut memory = Memory::new(&mut healthy);
    let output = bind_cdt_blank_labels_unchecked_with_memory(
        &input,
        purrdf_cdt::CDT_LIST,
        BlankBinding::Ambient(BlankScope(8)),
        &mut memory,
    )
    .unwrap();
    if let Cow::Owned(text) = output {
        memory.release_string(text).unwrap();
    }
    assert_eq!(memory.admitted_bytes(), 0);
    for limit in [0, input.len(), input.len() + 128, healthy.peak - 1] {
        let mut account = Account::new(limit);
        let window = CurrentThreadWindow::open();
        let failure = {
            let mut memory = Memory::new(&mut account);
            bind_cdt_blank_labels_unchecked_with_memory(
                &input,
                purrdf_cdt::CDT_LIST,
                BlankBinding::Ambient(BlankScope(8)),
                &mut memory,
            )
            .unwrap_err()
        };
        let measured = window.close();
        assert_eq!(failure, StorageError::AdmissionFailed);
        assert!(account.refused.unwrap() > limit);
        assert!(account.peak <= limit);
        assert!(usize::try_from(measured.peak_working_bytes).unwrap() <= account.peak);
        assert_eq!(measured.retained_bytes, 0);
    }
}

#[test]
fn decoder_image_check_is_allocation_free_and_refusal_is_typed() {
    let token = encode_blank_label(
        &"x é._".repeat(512),
        BlankScope(u32::MAX),
        LabelAlphabet::BlankNodeLabel,
    )
    .into_owned();
    let expected = decode_blank_label(&token, LabelAlphabet::BlankNodeLabel);
    let mut account = Account::new(100_000);
    let window = CurrentThreadWindow::open();
    let mut memory = Memory::new(&mut account);
    let (label, scope) =
        decode_blank_label_with_memory(&token, LabelAlphabet::BlankNodeLabel, &mut memory).unwrap();
    assert_eq!((label.as_ref(), scope), (expected.0.as_ref(), expected.1));
    let measured = window.close();
    let Cow::Owned(label) = label else {
        panic!("canonical envelope must decode");
    };
    assert_eq!(memory.admitted_bytes(), label.capacity());
    assert_eq!(
        usize::try_from(measured.retained_bytes).unwrap(),
        label.capacity()
    );
    let required = label.capacity();
    memory.release_string(label).unwrap();
    assert_eq!(memory.admitted_bytes(), 0);
    assert!(usize::try_from(measured.peak_working_bytes).unwrap() <= account.peak);
    assert_eq!(
        account.peak, required,
        "canonical image validation needs no encoded copy"
    );
    for limit in [
        0,
        token.len() / 2,
        token.len(),
        account.peak - 1,
        account.peak,
    ] {
        let mut account = Account::new(limit);
        let window = CurrentThreadWindow::open();
        let mut memory = Memory::new(&mut account);
        let result =
            decode_blank_label_with_memory(&token, LabelAlphabet::BlankNodeLabel, &mut memory);
        let measured = window.close();
        if limit < required {
            assert_eq!(result.unwrap_err(), StorageError::AdmissionFailed);
            assert_eq!(measured.retained_bytes, 0);
        } else {
            let (label, scope) = result.unwrap();
            assert_eq!((label.as_ref(), scope), (expected.0.as_ref(), expected.1));
            assert_eq!(memory.admitted_bytes(), required);
            assert_eq!(usize::try_from(measured.retained_bytes).unwrap(), required);
            let Cow::Owned(label) = label else {
                panic!("escaped canonical envelope owns decoded bytes")
            };
            memory.release_string(label).unwrap();
        }
        assert_eq!(memory.admitted_bytes(), 0);
        assert!(usize::try_from(measured.peak_working_bytes).unwrap() <= account.peak);
    }
}

#[test]
fn malformed_and_non_image_envelopes_keep_identity_and_release_scratch() {
    for token in [
        "plain",
        "purrdfesc_abc",
        "purrdfesc01_abc",
        "purrdfesc9__00d800",
        "purrdfesc9__00D800",
        "purrdfesc9__FFFFFF",
        "purrdfesc9__00005",
        "purrdfesc9_a!",
    ] {
        let expected = decode_blank_label(token, LabelAlphabet::BlankNodeLabel);
        let mut account = Account::new(10_000);
        let window = CurrentThreadWindow::open();
        let mut memory = Memory::new(&mut account);
        let result =
            decode_blank_label_with_memory(token, LabelAlphabet::BlankNodeLabel, &mut memory)
                .unwrap();
        assert_eq!(result, expected);
        assert!(matches!(result.0, Cow::Borrowed(_)));
        assert_eq!(memory.admitted_bytes(), 0);
        let measured = window.close();
        assert_eq!(measured.retained_bytes, 0);
        assert!(usize::try_from(measured.peak_working_bytes).unwrap() <= account.peak);
    }
}
