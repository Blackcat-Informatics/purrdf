// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The arbitrary-precision tower's answers on `wasm32-unknown-unknown`, held to
//! the native ones byte for byte.
//!
//! WebAssembly has 32- and 64-bit integers only: every `i128` operation of the
//! inline fast path, every `u64` carry lane of the limb arithmetic and the
//! `u128` binary-rounding step of the float conversions are lowered through
//! narrower operations there. A lowering bug would not crash; it would produce a
//! different digit or a different last bit. The pinned answers are numeric
//! expectations, which native Rust owns (`docs/WASM_TESTING.md`): `cargo test
//! --workspace` runs these cases on every `make check`. The target is
//! `harness = false` on `purrdf_testkit`'s runner, so the same named cases also
//! run on wasm32 in Node through `scripts/wasm-test-runner.sh`, against the same
//! pinned digest, when the lowering itself is in question:
//!
//! ```text
//! CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=scripts/wasm-test-runner.sh \
//!     cargo test -p purrdf-xsd --target wasm32-unknown-unknown --test exact_wasm_determinism
//! ```
//!
//! # What is pinned
//!
//! * `the_hand_values_are_reproduced_on_this_target`: values computed by hand
//!   (34!, the exact decimal of the smallest subnormal, `1/3` under the default
//!   policy, the exact binary value of `0.1f32`) — each checkable without the
//!   implementation.
//! * `the_transcript_digest_is_reproduced_on_this_target`: a seeded transcript of
//!   every operation of the tower over operands from one digit to hundreds —
//!   canonical lexical forms and float bit patterns, one line per result — folded
//!   by FNV-1a. Its correctness is the oracle property suite's job
//!   (`tests/exact_tower.rs`); this case holds the wasm32 transcript to the
//!   native one, by a digest both targets must reproduce and that is reported on
//!   a `determinism-digest` line.
//! * `the_oracle_vectors_replay_on_this_target`: the 7,164 frozen records of
//!   `tests/vectors/exact_numeric_vectors.txt`, generated from the exact rational
//!   oracle, replayed through the `XsdValue` operators (`+ − ×`, both division
//!   policies, comparison, the conversions to `f64`/`f32` and canonical forms).

use std::fmt::Write as _;
use std::str::FromStr;

use purrdf_testkit::harness::report_digest;
use purrdf_testkit::rng::splitmix64_next;
use purrdf_xsd::exact::{Decimal, DivisionPolicy, Integer, Rational, Rounding};

/// The transcript's FNV-1a digest, as both targets compute it.
const GOLDEN_DIGEST: u64 = 0x7d6b_1f15_734f_09aa;

/// Operand pairs in the transcript.
const PAIRS: usize = 160;

/// `len` seeded digits.
fn seeded_digits(state: &mut u64, len: u64) -> String {
    (0..len)
        .map(|_| char::from(b'0' + u8::try_from(splitmix64_next(state) % 10).expect("a digit")))
        .collect()
}

/// A seeded decimal lexical: up to `max` digits on each side of the point.
fn decimal_text(state: &mut u64, max: u64) -> String {
    let whole_len = splitmix64_next(state) % max + 1;
    let fraction_len = splitmix64_next(state) % max;
    let whole = seeded_digits(state, whole_len);
    let fraction = seeded_digits(state, fraction_len);
    let sign = if splitmix64_next(state).is_multiple_of(2) {
        ""
    } else {
        "-"
    };
    if fraction.is_empty() {
        format!("{sign}{whole}")
    } else {
        format!("{sign}{whole}.{fraction}")
    }
}

fn transcript() -> String {
    let mut state = 0x7A5C_0D1E_u64;
    let mut out = String::new();
    let roundings = [
        Rounding::TowardZero,
        Rounding::AwayFromZero,
        Rounding::Floor,
        Rounding::Ceiling,
        Rounding::HalfEven,
        Rounding::HalfAwayFromZero,
        Rounding::HalfTowardZero,
        Rounding::HalfCeiling,
        Rounding::HalfFloor,
    ];
    for index in 0..PAIRS {
        // Sizes cycle from a few digits through the i128 edge to hundreds.
        let max = [3_u64, 20, 40, 120, 400][index % 5];
        let a = Decimal::from_str(&decimal_text(&mut state, max)).expect("valid");
        let b = Decimal::from_str(&decimal_text(&mut state, max)).expect("valid");
        let rounding = roundings[index % roundings.len()];
        let scale = u32::try_from(index % 40).expect("small");
        writeln!(out, "{a} {b}").expect("a String");
        writeln!(out, "+ {}", &a + &b).expect("a String");
        writeln!(out, "- {}", &a - &b).expect("a String");
        writeln!(out, "* {}", a.try_mul(&b).expect("small scales")).expect("a String");
        writeln!(out, "< {:?}", a.cmp(&b)).expect("a String");
        match a.div(&b, DivisionPolicy::scale(scale, rounding)) {
            Ok(q) => writeln!(out, "/ {q}").expect("a String"),
            Err(error) => writeln!(out, "/ {}", error.code().local_name()).expect("a String"),
        }
        match a.div(&b, DivisionPolicy::Exact) {
            Ok(q) => writeln!(out, "/= {q}").expect("a String"),
            Err(error) => writeln!(out, "/= {}", error.code().local_name()).expect("a String"),
        }
        let precision = i32::try_from(index % 30).expect("small") - 5;
        writeln!(out, "r {}", a.round(precision, rounding)).expect("a String");
        writeln!(
            out,
            "f {:016x} {:08x}",
            a.to_f64().to_bits(),
            a.to_f32().to_bits()
        )
        .expect("a String");
        let (x, y) = (a.to_integer_truncated(), b.to_integer_truncated());
        writeln!(out, "i* {}", &x * &y).expect("a String");
        match x.div_rem(&y) {
            Ok((q, r)) => writeln!(out, "i/ {q} {r}").expect("a String"),
            Err(error) => writeln!(out, "i/ {}", error.code().local_name()).expect("a String"),
        }
        writeln!(
            out,
            "i^ {}",
            x.pow(u32::try_from(index % 7).expect("small"))
        )
        .expect("a String");
        writeln!(out, "if {:016x}", x.to_f64().to_bits()).expect("a String");
        let (p, q) = (Rational::from_decimal(&a), Rational::from_decimal(&b));
        writeln!(out, "q+ {}", &p + &q).expect("a String");
        match p.checked_div(&q) {
            Ok(ratio) => {
                writeln!(out, "q/ {ratio} {:016x}", ratio.to_f64().to_bits()).expect("a String");
            }
            Err(error) => writeln!(out, "q/ {}", error.code().local_name()).expect("a String"),
        }
        let bits = splitmix64_next(&mut state);
        match Decimal::from_f64(f64::from_bits(bits)) {
            Ok(exact) => writeln!(out, "d {exact}").expect("a String"),
            Err(error) => writeln!(out, "d {}", error.code().local_name()).expect("a String"),
        }
    }
    out
}

/// The seeded transcript folds to the pinned digest on this target.
fn the_transcript_digest_is_reproduced_on_this_target() {
    let text = transcript();
    let digest = report_digest("exact-tower-transcript", text.len(), || {
        purrdf_hash::fnv::fnv1a64(text.as_bytes())
    });
    assert_eq!(
        digest,
        GOLDEN_DIGEST,
        "transcript digest {digest:016x} over {} bytes",
        text.len()
    );
}

/// Values checkable by hand reproduce on this target.
fn the_hand_values_are_reproduced_on_this_target() {
    let factorial = (1..=34_i128).fold(Integer::ONE, |acc, k| &acc * &Integer::from(k));
    assert_eq!(
        factorial.to_string(),
        "295232799039604140847618609643520000000"
    );
    // 2^-1074 = 5^1074 / 10^1074: 1074 fractional digits, ending in ...625.
    let smallest = Decimal::from_f64(f64::from_bits(1)).expect("finite");
    assert_eq!(smallest.scale(), 1074);
    let written = smallest.to_string();
    assert!(written.starts_with(&format!(
        "0.{}4940656458412465441765687928682213723650598",
        "0".repeat(323)
    )));
    assert!(written.ends_with("538682506419718265533447265625"));
    assert_eq!(smallest.to_f64().to_bits(), 1);
    // Half the smallest subnormal ties to even, which is zero; a hair above it
    // rounds up to the smallest subnormal.
    let half = smallest
        .div(&Decimal::from(2), DivisionPolicy::Exact)
        .expect("terminates");
    assert_eq!(half.to_f64().to_bits(), 0);
    let above = &half + &Decimal::new(Integer::ONE, 1200);
    assert_eq!(above.to_f64().to_bits(), 1);
    let third = Decimal::ONE
        .div(&Decimal::from(3), DivisionPolicy::default())
        .expect("nonzero");
    assert_eq!(third.to_string(), "0.333333333333333333");
    assert_eq!(
        Decimal::from_f32(0.1).expect("finite").to_string(),
        "0.100000001490116119384765625"
    );
    let past = "1".repeat(400);
    let big = Integer::from_str(&past).expect("valid");
    assert_eq!(big.to_f64(), f64::INFINITY);
    assert_eq!(
        Rational::from_str("-22/7").expect("valid").to_f64(),
        -3.142_857_142_857_143
    );
}

/// XPath F&O numeric answers over `xsd:integer`/`xsd:decimal` operands of any
/// size, generated once from the exact rational oracle
/// (`examples/gen_exact_numeric_vectors.rs`) and frozen: arithmetic, both division
/// precisions, comparison, correctly rounded conversion and canonical forms, every
/// record reproduced byte for byte on this target.
fn the_oracle_vectors_replay_on_this_target() {
    use purrdf_testkit::vectors::VectorFile;
    use purrdf_xsd::numeric::numeric_div_with_policy;
    use purrdf_xsd::{
        XsdDatatype, XsdError, XsdValue, numeric_add, numeric_cmp, numeric_mul, numeric_sub,
    };
    use std::cmp::Ordering;

    let file = VectorFile::load(include_str!("vectors/exact_numeric_vectors.txt"));
    let value = |text: &str| {
        let datatype = if text.contains('.') {
            XsdDatatype::Decimal
        } else {
            XsdDatatype::Integer
        };
        purrdf_xsd::parse(text, datatype).unwrap_or_else(|error| panic!("{text}: {error}"))
    };
    let lexical = |result: Result<XsdValue, XsdError>| match result {
        Ok(value) => value.canonical_lexical(),
        Err(XsdError::DivisionByZero { .. }) => "division-by-zero".to_owned(),
        Err(XsdError::Exact(purrdf_xsd::exact::ExactError::NonTerminating)) => {
            "non-terminating".to_owned()
        }
        Err(other) => format!("error: {other}"),
    };
    let replayed = file
        .replay(3, |fields| {
            let a = value(fields[1]);
            let answer = match fields[0] {
                "add" => lexical(numeric_add(&a, &value(fields[2]))),
                "sub" => lexical(numeric_sub(&a, &value(fields[2]))),
                "mul" => lexical(numeric_mul(&a, &value(fields[2]))),
                "div" => lexical(numeric_div_with_policy(
                    &a,
                    &value(fields[2]),
                    DivisionPolicy::default(),
                )),
                "div-exact" => lexical(numeric_div_with_policy(
                    &a,
                    &value(fields[2]),
                    DivisionPolicy::Exact,
                )),
                "cmp" => match numeric_cmp(&a, &value(fields[2])) {
                    Some(Ordering::Less) => "<".to_owned(),
                    Some(Ordering::Equal) => "=".to_owned(),
                    Some(Ordering::Greater) => ">".to_owned(),
                    None => "incomparable".to_owned(),
                },
                "double" => match numeric_add(&a, &XsdValue::Double(0.0)) {
                    Ok(XsdValue::Double(d)) => format!("{:016x}", d.to_bits()),
                    other => format!("{other:?}"),
                },
                "float" => match numeric_add(&a, &XsdValue::Float(0.0)) {
                    Ok(XsdValue::Float(f)) => format!("{:08x}", f.to_bits()),
                    other => format!("{other:?}"),
                },
                "canonical" => a.canonical_lexical(),
                other => panic!("unknown operation {other}"),
            };
            vec![answer]
        })
        .unwrap_or_else(|mismatch| panic!("{mismatch}"));
    assert!(replayed > 7_000, "{replayed} records");
}

purrdf_testkit::harness_main!(
    the_hand_values_are_reproduced_on_this_target,
    the_transcript_digest_is_reproduced_on_this_target,
    the_oracle_vectors_replay_on_this_target,
);
