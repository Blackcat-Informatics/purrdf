// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! The arbitrary-precision tower (`purrdf_xsd::exact`): its small-value fast
//! path, directly and through the `XsdValue` operators, and how its cost grows
//! with operand size.
//!
//! Report-only, `cargo bench -p purrdf-xsd --bench exact`. Nothing here asserts
//! a threshold.
//!
//! - `xsd_exact_small/*/{exact,value}` — the same 1024 operations over values
//!   inside `i128`, on the tower (`Integer`, `Decimal`) and through the
//!   `XsdValue` operators (`numeric_add`, `numeric_mul`, `numeric_div`,
//!   `parse_decimal`, `Decimal::to_f64`) that every consumer calls. The `value`
//!   lanes are the ones the 3.x `bounded` lanes measured, when those operators
//!   still ran on the bounded `i128` types: comparing the two across releases is
//!   the small-value regression check.
//! - `xsd_exact_growth/*/<digits>` — one operation on operands of 40 to 40 000
//!   decimal digits: addition, parsing and rendering stay linear, products grow
//!   as Karatsuba's `n^1.585`, division as quotient × divisor length.
//! - `xsd_exact_karatsuba/{schoolbook,karatsuba}/<limbs>` — the two product
//!   algorithms on either side of `KARATSUBA_THRESHOLD`, the evidence for where
//!   it sits.
//! - `xsd_exact_fine_products/{bounded,tower}` — products of two machine-word
//!   decimals whose scales sum to eighteen and to twenty (past the 3.x bound).
//!
//! Each routine in `--test` mode runs its body once, so
//! `perf stat -e instructions:u <bench binary> --test --exact <id>` minus the
//! same for `--list` is a load-independent instruction count for the body.

use std::str::FromStr;

use purrdf_testkit::bench::{Bench, Throughput, bench_group, bench_main, black_box};
use purrdf_testkit::rng::splitmix64_next;
use purrdf_xsd::bigint::BigInt;
use purrdf_xsd::exact::{Decimal, DivisionPolicy, Integer};
use purrdf_xsd::numeric::parse_decimal;
use purrdf_xsd::{XsdValue, numeric_add, numeric_div, numeric_mul};

/// Operations per small-path routine.
const SMALL_OPS: usize = 1024;

/// Operand sizes, in decimal digits, of the growth group.
const GROWTH_DIGITS: [usize; 4] = [40, 400, 4_000, 40_000];

/// A seeded digit string of exactly `len` digits (no leading zero).
fn digit_string(state: &mut u64, len: usize) -> String {
    let mut text = String::with_capacity(len);
    while text.len() < len {
        let digit = u8::try_from(splitmix64_next(state) % 10).expect("a digit");
        if text.is_empty() && digit == 0 {
            continue;
        }
        text.push(char::from(b'0' + digit));
    }
    text
}

/// Small signed integer pairs (inside `i64`, so every product fits `i128`).
fn small_integers() -> Vec<(i128, i128)> {
    let mut state = 0x5EED_E8AC_u64;
    (0..SMALL_OPS)
        .map(|_| {
            let a = i128::from(splitmix64_next(&mut state) as i64 >> 8);
            let b = i128::from(splitmix64_next(&mut state) as i64 >> 8) | 1;
            (a, b)
        })
        .collect()
}

/// Small decimal lexical pairs: up to twelve digits, scales 0 to 6.
fn small_decimals() -> Vec<(String, String)> {
    let mut state = 0xDEC1_3A15_u64;
    let one = |state: &mut u64| {
        let coefficient = (splitmix64_next(state) % 1_000_000_000_000) as i64 - 500_000_000_000;
        let scale = (splitmix64_next(state) % 7) as usize;
        let digits = format!("{:0>width$}", coefficient.unsigned_abs(), width = scale + 1);
        let sign = if coefficient < 0 { "-" } else { "" };
        let split = digits.len() - scale;
        if scale == 0 {
            format!("{sign}{digits}")
        } else {
            format!("{sign}{}.{}", &digits[..split], &digits[split..])
        }
    };
    (0..SMALL_OPS)
        .map(|_| (one(&mut state), one(&mut state)))
        .collect()
}

fn bench_small(c: &mut Bench) {
    let integers = small_integers();
    let exact_integers: Vec<(Integer, Integer)> = integers
        .iter()
        .map(|&(a, b)| (Integer::from(a), Integer::from(b)))
        .collect();
    let value_integers: Vec<(XsdValue, XsdValue)> = integers
        .iter()
        .map(|&(a, b)| (XsdValue::integer(a), XsdValue::integer(b)))
        .collect();
    let decimals = small_decimals();
    let exact_decimals: Vec<(Decimal, Decimal)> = decimals
        .iter()
        .map(|(a, b)| {
            (
                Decimal::from_str(a).expect("valid"),
                Decimal::from_str(b).expect("valid"),
            )
        })
        .collect();
    let value_decimals: Vec<(XsdValue, XsdValue)> = decimals
        .iter()
        .map(|(a, b)| {
            (
                XsdValue::Decimal(parse_decimal(a).expect("valid")),
                XsdValue::Decimal(parse_decimal(b).expect("valid")),
            )
        })
        .collect();

    let mut group = c.benchmark_group("xsd_exact_small");
    group.throughput(Throughput::Elements(SMALL_OPS as u64));
    group.bench_function("integer_add/exact", |b| {
        b.iter(|| {
            for (x, y) in &exact_integers {
                black_box(black_box(x) + black_box(y));
            }
        });
    });
    group.bench_function("integer_add/value", |b| {
        b.iter(|| {
            for (x, y) in &value_integers {
                black_box(numeric_add(black_box(x), black_box(y)).expect("in range"));
            }
        });
    });
    group.bench_function("integer_mul/exact", |b| {
        b.iter(|| {
            for (x, y) in &exact_integers {
                black_box(black_box(x) * black_box(y));
            }
        });
    });
    group.bench_function("integer_mul/value", |b| {
        b.iter(|| {
            for (x, y) in &value_integers {
                black_box(numeric_mul(black_box(x), black_box(y)).expect("in range"));
            }
        });
    });
    group.bench_function("decimal_add/exact", |b| {
        b.iter(|| {
            for (x, y) in &exact_decimals {
                black_box(black_box(x) + black_box(y));
            }
        });
    });
    group.bench_function("decimal_add/value", |b| {
        b.iter(|| {
            for (x, y) in &value_decimals {
                black_box(numeric_add(black_box(x), black_box(y)).expect("in range"));
            }
        });
    });
    group.bench_function("decimal_mul/exact", |b| {
        b.iter(|| {
            for (x, y) in &exact_decimals {
                black_box(black_box(x).try_mul(black_box(y)).expect("small scale"));
            }
        });
    });
    group.bench_function("decimal_mul/value", |b| {
        b.iter(|| {
            for (x, y) in &value_decimals {
                black_box(numeric_mul(black_box(x), black_box(y)).expect("in range"));
            }
        });
    });
    group.bench_function("decimal_div/exact", |b| {
        b.iter(|| {
            for (x, y) in &exact_decimals {
                if !y.is_zero() {
                    black_box(
                        black_box(x)
                            .div(black_box(y), DivisionPolicy::default())
                            .expect("nonzero"),
                    );
                }
            }
        });
    });
    group.bench_function("decimal_div/value", |b| {
        b.iter(|| {
            for (x, y) in &value_decimals {
                black_box(numeric_div(black_box(x), black_box(y)).ok());
            }
        });
    });
    group.bench_function("decimal_parse_canonical/exact", |b| {
        b.iter(|| {
            let mut len = 0;
            for (text, _) in &decimals {
                len += Decimal::from_str(black_box(text))
                    .expect("valid")
                    .canonical_lexical()
                    .len();
            }
            black_box(len)
        });
    });
    group.bench_function("decimal_parse_canonical/value", |b| {
        b.iter(|| {
            let mut len = 0;
            for (text, _) in &decimals {
                len += parse_decimal(black_box(text))
                    .expect("valid")
                    .canonical_lexical()
                    .len();
            }
            black_box(len)
        });
    });
    group.bench_function("decimal_to_f64/exact", |b| {
        b.iter(|| {
            for (x, _) in &exact_decimals {
                black_box(black_box(x).to_f64());
            }
        });
    });
    group.bench_function("decimal_to_f64/value", |b| {
        b.iter(|| {
            for (x, _) in &value_decimals {
                if let XsdValue::Decimal(decimal) = black_box(x) {
                    black_box(decimal.to_f64());
                }
            }
        });
    });
    group.finish();
}

fn bench_growth(c: &mut Bench) {
    let mut state = 0x6E0_57A7E_u64;
    let mut group = c.benchmark_group("xsd_exact_growth");
    for digits in GROWTH_DIGITS {
        let a = Integer::from_str(&digit_string(&mut state, digits)).expect("valid");
        let b = Integer::from_str(&digit_string(&mut state, digits)).expect("valid");
        let wide = Integer::from_str(&digit_string(&mut state, 2 * digits)).expect("valid");
        let text = format!(
            "{}.{}",
            digit_string(&mut state, digits / 2),
            digit_string(&mut state, digits / 2)
        );
        let decimal = Decimal::from_str(&text).expect("valid");
        let divisor = Decimal::from_str(&format!("0.{}", digit_string(&mut state, digits / 2)))
            .expect("valid");
        group.throughput(Throughput::Bytes(digits as u64));
        group.bench_function(format!("add/{digits}"), |bench| {
            bench.iter(|| black_box(black_box(&a) + black_box(&b)));
        });
        group.bench_function(format!("mul/{digits}"), |bench| {
            bench.iter(|| black_box(black_box(&a) * black_box(&b)));
        });
        group.bench_function(format!("div_rem/{digits}"), |bench| {
            bench.iter(|| black_box(black_box(&wide).div_rem(black_box(&a)).expect("nonzero")));
        });
        group.bench_function(format!("decimal_div_scale18/{digits}"), |bench| {
            bench.iter(|| {
                black_box(
                    black_box(&decimal)
                        .div(black_box(&divisor), DivisionPolicy::default())
                        .expect("nonzero"),
                )
            });
        });
        group.bench_function(format!("parse_canonical/{digits}"), |bench| {
            bench.iter(|| {
                black_box(
                    Decimal::from_str(black_box(&text))
                        .expect("valid")
                        .canonical_lexical(),
                )
            });
        });
        group.bench_function(format!("to_f64/{digits}"), |bench| {
            bench.iter(|| black_box(black_box(&decimal).to_f64()));
        });
    }
    group.finish();
}

fn bench_karatsuba(c: &mut Bench) {
    let mut state = 0x4B41_5241_u64;
    let mut group = c.benchmark_group("xsd_exact_karatsuba");
    for limbs in [16_usize, 48, 128, 256, 512, 1024, 2048] {
        let a = BigInt::from_digits(&digit_string(&mut state, limbs * 9)).expect("digits");
        let b = BigInt::from_digits(&digit_string(&mut state, limbs * 9)).expect("digits");
        group.throughput(Throughput::Elements(limbs as u64));
        group.bench_function(format!("schoolbook/{limbs}"), |bench| {
            bench.iter(|| black_box(black_box(&a).mul(black_box(&b))));
        });
        group.bench_function(format!("karatsuba/{limbs}"), |bench| {
            bench.iter(|| black_box(black_box(&a).mul_fast(black_box(&b))));
        });
    }
    group.finish();
}

/// Bounded decimal pairs of `scale` fractional digits each, coefficients of up to
/// twelve digits.
fn scaled_decimals(scale: usize) -> Vec<(XsdValue, XsdValue)> {
    let mut state = 0xF1E5_CA1E_u64 ^ scale as u64;
    let one = |state: &mut u64| {
        let digits = digit_string(state, 12);
        let (whole, fraction) = digits.split_at(12 - scale.min(12));
        let fraction = format!("{fraction:0>scale$}");
        let text = format!("{}.{fraction}", if whole.is_empty() { "0" } else { whole });
        XsdValue::Decimal(parse_decimal(&text).expect("a decimal"))
    };
    (0..SMALL_OPS)
        .map(|_| (one(&mut state), one(&mut state)))
        .collect()
}

/// `xsd_exact_fine_products/{bounded,tower}`: 1024 `numeric_mul` products of two
/// twelve-digit decimals. Nine fractional digits each keep the product at eighteen;
/// ten each make it twenty, which the 3.x bounded decimal could not hold and the
/// exact decimal does, its 24-digit coefficient still inline in `i128`. The pair is
/// the cost of an exact product past eighteen fractional digits against the one
/// beside it that never left eighteen.
fn bench_fine_products(c: &mut Bench) {
    let at_eighteen = scaled_decimals(9);
    let at_twenty = scaled_decimals(10);
    let mut group = c.benchmark_group("xsd_exact_fine_products");
    group.throughput(Throughput::Elements(SMALL_OPS as u64));
    group.bench_function("bounded", |b| {
        b.iter(|| {
            for (x, y) in &at_eighteen {
                black_box(numeric_mul(black_box(x), black_box(y)).expect("exact"));
            }
        });
    });
    group.bench_function("tower", |b| {
        b.iter(|| {
            for (x, y) in &at_twenty {
                black_box(numeric_mul(black_box(x), black_box(y)).expect("exact"));
            }
        });
    });
    group.finish();
}

bench_group!(
    benches,
    bench_small,
    bench_growth,
    bench_karatsuba,
    bench_fine_products
);
bench_main!(benches);
