// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The bounded numeric contract gives one answer on every target.
//!
//! The target is `harness = false` and hands its named cases to
//! `purrdf_testkit::harness`, so they run natively under `cargo test` and on
//! `wasm32-unknown-unknown` in Node:
//!
//! ```text
//! CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=scripts/wasm-test-runner.sh \
//!     cargo test -p purrdf-xsd --target wasm32-unknown-unknown --test bounded_wasm
//! ```
//!
//! Each case folds every answer over a seeded corpus — canonical forms, exact
//! comparisons, the correctly rounded `f64`/`f32` conversions of numerals of any size,
//! decimal arithmetic under the precision rule and the mean of totals past the bounds,
//! each typed error by its code — into one FNV-1a digest pinned here. A target that
//! answers any of them differently changes the digest; the oracle tests in
//! `bounded_numerics.rs` prove the answers right, this target proves them the same.

use std::process::ExitCode;

use purrdf_hash::fnv;
use purrdf_testkit::harness::{self, Failed, Trial};
use purrdf_testkit::rng::splitmix64_next;
use purrdf_xsd::{
    BigInt, DecimalDigits, LiteralValue, XsdDatatype, XsdError, XsdValue, decimal_mean,
    literal_cmp, literal_total_cmp, numeric_add, numeric_div, numeric_mul, numeric_sub, parse,
};

/// Seeded corpus size per case.
const DRAWS: usize = 4_000;

/// A seeded numeral of up to `int_max` integer digits and `frac_max` fractional ones.
fn numeral(state: &mut u64, int_max: u64, frac_max: u64) -> String {
    let mut text = String::new();
    if splitmix64_next(state) & 1 == 1 {
        text.push('-');
    }
    let int_len = 1 + splitmix64_next(state) % int_max;
    for _ in 0..int_len {
        text.push(char::from(b'0' + (splitmix64_next(state) % 10) as u8));
    }
    let frac_len = splitmix64_next(state) % (frac_max + 1);
    if frac_len > 0 {
        text.push('.');
        for _ in 0..frac_len {
            text.push(char::from(b'0' + (splitmix64_next(state) % 10) as u8));
        }
    }
    text
}

/// Fold one answer into the digest, with a separator so answers cannot run together.
fn fold(digest: u64, answer: &str) -> u64 {
    fnv::fold(fnv::fold(digest, answer.as_bytes()), b"\n")
}

/// An arithmetic answer as text: the canonical value, or the F&O code of the error.
fn answer(result: Result<XsdValue, XsdError>) -> String {
    match result {
        Ok(value) => value.canonical_lexical(),
        Err(error) => error.code().map_or("none", |code| code.qname()).to_owned(),
    }
}

fn check(case: &str, digest: u64, pinned: u64) -> Result<(), Failed> {
    if digest == pinned {
        Ok(())
    } else {
        Err(format!("{case}: digest {digest:#018x}, pinned {pinned:#018x}").into())
    }
}

/// Numerals of any size: canonical form, exact order, and the correctly rounded
/// conversions to `f64` and `f32`, by their bits.
fn digits_answer_the_same() -> Result<(), Failed> {
    let mut state = 0x0042_2001_u64;
    let mut digest = fnv::BASIS;
    let mut previous = DecimalDigits::parse("0").ok_or("zero parses")?;
    for _ in 0..DRAWS {
        let text = numeral(&mut state, 70, 70);
        let value = DecimalDigits::parse(&text).ok_or("a numeral parses")?;
        digest = fold(digest, &value.canonical_lexical());
        digest = fold(digest, &format!("{:?}", value.cmp(&previous)));
        digest = fold(digest, &format!("{:016x}", value.to_f64().to_bits()));
        digest = fold(digest, &format!("{:08x}", value.to_f32().to_bits()));
        digest = fold(digest, &format!("{:?}", value.cmp_f64(previous.to_f64())));
        previous = value;
    }
    check("digits", digest, 0x3424_c986_73e0_b526)
}

/// Literal values straddling the bounds, by the SPARQL order and the sort order.
fn literal_order_answers_the_same() -> Result<(), Failed> {
    let mut state = 0x0042_2002_u64;
    let mut digest = fnv::BASIS;
    let literal = |text: &str| {
        let datatype = if text.contains('.') {
            XsdDatatype::Decimal
        } else {
            XsdDatatype::Integer
        };
        LiteralValue::parse(text, datatype)
    };
    for _ in 0..DRAWS {
        let a = literal(&numeral(&mut state, 45, 22)).map_err(|e| e.to_string())?;
        let b = literal(&numeral(&mut state, 45, 22)).map_err(|e| e.to_string())?;
        let bits = f64::from_bits(splitmix64_next(&mut state));
        let double =
            LiteralValue::Bounded(XsdValue::Double(if bits.is_finite() { bits } else { 1.5 }));
        digest = fold(digest, &format!("{:?}", literal_cmp(&a, &b)));
        digest = fold(digest, &format!("{:?}", literal_total_cmp(&a, &double)));
        digest = fold(digest, &format!("{:?}", literal_cmp(&a, &double)));
    }
    check("literal order", digest, 0x10db_861a_bd57_e98c)
}

/// Decimal `+ − × ÷` under the precision rule, every overflow by its code.
fn decimal_arithmetic_answers_the_same() -> Result<(), Failed> {
    let mut state = 0x0042_2003_u64;
    let mut digest = fnv::BASIS;
    let mut drawn = 0;
    while drawn < DRAWS {
        let (Ok(a), Ok(b)) = (
            parse(&numeral(&mut state, 38, 18), XsdDatatype::Decimal),
            parse(&numeral(&mut state, 38, 18), XsdDatatype::Decimal),
        ) else {
            continue;
        };
        drawn += 1;
        digest = fold(digest, &answer(numeric_add(&a, &b)));
        digest = fold(digest, &answer(numeric_sub(&a, &b)));
        digest = fold(digest, &answer(numeric_mul(&a, &b)));
        digest = fold(digest, &answer(numeric_div(&a, &b)));
    }
    check("decimal arithmetic", digest, 0x505e_0f59_1476_88a9)
}

/// The mean of exact running totals of any size and scale.
fn the_mean_answers_the_same() -> Result<(), Failed> {
    let mut state = 0x0042_2004_u64;
    let mut digest = fnv::BASIS;
    for _ in 0..DRAWS {
        let digits = numeral(&mut state, 60, 0);
        let sum = BigInt::from_digits(&digits).ok_or("digits parse")?;
        let scale = u32::try_from(splitmix64_next(&mut state) % 30).map_err(|e| e.to_string())?;
        let count = 1 + splitmix64_next(&mut state) % 1_000_000;
        let mean = decimal_mean(&sum, scale, count).map(XsdValue::Decimal);
        digest = fold(digest, &answer(mean));
    }
    check("mean", digest, 0xc20a_fd49_9d9f_a883)
}

fn main() -> ExitCode {
    harness::main(vec![
        Trial::test("digits_answer_the_same", digits_answer_the_same),
        Trial::test(
            "literal_order_answers_the_same",
            literal_order_answers_the_same,
        ),
        Trial::test(
            "decimal_arithmetic_answers_the_same",
            decimal_arithmetic_answers_the_same,
        ),
        Trial::test("the_mean_answers_the_same", the_mean_answers_the_same),
    ])
}
