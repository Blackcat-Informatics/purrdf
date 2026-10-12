// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Independently generate and certify the native cube-grid constants.

use purrdf_geo_kernel::exact::{Int, Rat};

#[path = "cube/constants.rs"]
mod derivation;

fn main() {
    let constants = derivation::certified_constants();
    println!("PI192 little-endian limbs = {:016x?}", constants.pi192);
    println!("PI96 = {:#x}", constants.pi96);
    println!("SINE Q96 = {:?}", constants.sine);
    println!("COSINE Q96 = {:?}", constants.cosine);
    println!("Machin interval width < 2^-200; both endpoints round identically at Q192");
}
