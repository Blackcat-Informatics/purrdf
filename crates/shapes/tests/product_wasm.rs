// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **The whole prepared-product lifecycle, EXECUTED on
//! `wasm32-unknown-unknown`, against the same bytes the host writes.**
//!
//! `make wasm` proves the crate builds for wasm32. It cannot prove the codec
//! *answers* the same way there, and for a cache format that is the claim that
//! matters: a product is prepared by a build tool on a host and restored months
//! later by a browser. If those two disagree by one byte, the browser's `open`
//! refuses a product that is perfectly valid, or — far worse — the browser writes
//! products the host cannot read, and the divergence surfaces as a cache that
//! mysteriously never hits.
//!
//! Two runs on one target cannot tell a codec that is target-independent from one
//! that merely agrees with whichever target it was last compiled for. So this file
//! asserts against a byte string that was produced *elsewhere*: the golden
//! committed under `tests/fixtures/`, written by a native build.
//!
//! # How it runs on both
//!
//! One test body, two attributes. Natively these are ordinary `#[test]`s picked up
//! by `cargo test -p purrdf-shapes`; on `wasm32-unknown-unknown` they are
//! `#[wasm_bindgen_test]`s compiled to wasm and executed in Node by
//! `make wasm-test`:
//!
//! ```text
//! cargo test -p purrdf-shapes --target wasm32-unknown-unknown --test product_wasm
//! ```
//!
//! Both runs assert the *same* expectations, so the native run is not a weaker
//! version of the wasm one — it is the other half of the comparison.
//!
//! # No filesystem, on purpose
//!
//! Nothing here opens a file. The shapes graph, the data graph and the golden
//! product are all compile-time constants (`include_bytes!` resolves before the
//! module ever reaches a target), because a wasm32 test that needed a filesystem
//! would be proving something about the runner's shims rather than about the
//! codec.

mod product_fixture;

use purrdf_shapes::product::{ShapesProduct, ShapesProfile};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

/// The bytes this target writes for the fixture are the bytes the host wrote.
///
/// The golden was produced by a native build and committed; the wasm run encodes
/// the same shapes graph from source and compares. A target whose pointer width,
/// endianness or hash seeding reached the writer renders a different product here
/// rather than shipping a cache two engines disagree about.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn encoding_matches_the_committed_bytes_on_this_target() {
    let bytes = product_fixture::encode();

    assert_eq!(
        bytes.len(),
        product_fixture::GOLDEN.len(),
        "this target writes a {}-byte product for the fixture and the committed golden is {} \
         bytes; a size difference is a layout difference, not a rounding one",
        bytes.len(),
        product_fixture::GOLDEN.len(),
    );
    assert!(
        bytes == product_fixture::GOLDEN,
        "this target writes different product bytes than the committed golden (first difference \
         at byte {:?}); the prepared-product codec must be a pure function of the shapes graph \
         on every target, or a product prepared on a host cannot be restored in a browser",
        bytes
            .iter()
            .zip(product_fixture::GOLDEN.iter())
            .position(|(a, b)| a != b),
    );
}

/// Encode, open, admit and validate — the entire lifecycle, on this target.
///
/// The byte comparison above proves the *writer* agrees across targets; it says
/// nothing about the reader, and a product this target can write but not restore
/// is still a broken cache. This runs the round trip end to end and checks the
/// answer, so a decoder that mis-read a field on one target is caught by the
/// report rather than by the bytes.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn encode_open_admit_validate_on_this_target() {
    let expected = product_fixture::expected_report_nt();

    let bytes = product_fixture::encode();
    let restored = ShapesProduct::open(&bytes)
        .expect("the product this target just wrote opens on this target")
        .admit(&ShapesProfile::CORE, &product_fixture::host())
        .expect("the product this target just wrote admits on this target");

    let restored_nt = product_fixture::report_nt(&restored);
    product_fixture::assert_non_vacuous(&restored_nt);
    assert_eq!(
        restored_nt, expected,
        "a product restored on this target validates differently from a fresh parse of the same \
         shapes graph on this target",
    );
}

/// The committed golden — bytes this target did not produce — restores and
/// validates here.
///
/// This is the cross-target *restore* direction: the product travelled from a
/// native build into this target's memory as a constant, and it has to become a
/// working validator. Encoding and decoding could both be target-dependent in the
/// same way and still pass the round trip above; only a product from another
/// target separates them.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn the_committed_golden_restores_on_this_target() {
    let restored = ShapesProduct::open(product_fixture::GOLDEN)
        .expect("the committed golden opens on this target")
        .admit(&ShapesProfile::CORE, &product_fixture::host())
        .expect("the committed golden admits on this target");

    let restored_nt = product_fixture::report_nt(&restored);
    product_fixture::assert_non_vacuous(&restored_nt);
    assert_eq!(
        restored_nt,
        product_fixture::expected_report_nt(),
        "the committed golden restored on this target to a validator that answers differently \
         from a fresh parse of the same shapes graph",
    );
}
