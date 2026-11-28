// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Report-only numerical context, arithmetic and prepared/batch latency.
#![allow(missing_docs)]

use purrdf_hash::Backend as _;
use purrdf_testkit::bench::{Bench, Throughput, bench_group, bench_main};
use purrdf_xsd::ieee::environment;
use purrdf_xsd::math::{
    CoordinateMath, FixedInterval, FloatInterval, FloatProductBackend, MathLimits,
};
use std::hint::black_box;
use std::time::Duration;

fn limits() -> MathLimits {
    MathLimits {
        precision_bits: 96,
        max_work: u64::MAX,
        ..MathLimits::DEFAULT
    }
}

fn context(c: &mut Bench) {
    let mut group = c.benchmark_group("numerical_context");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(20));
    group.measurement_time(Duration::from_millis(100));
    group.bench_function("construct96", |b| {
        b.iter(|| black_box(CoordinateMath::new(limits()).expect("math admission")));
    });
    group.bench_function("validate", |b| {
        b.iter(|| {
            environment::check().expect("floating environment");
            black_box(());
        });
    });
    let mut math = CoordinateMath::new(limits()).expect("math admission");
    group.bench_function("enter_chunk", |b| {
        b.iter(|| black_box(math.enter_chunk().expect("validated chunk")));
    });
    group.finish();
}

fn arithmetic(c: &mut Bench) {
    let mut group = c.benchmark_group("numerical_arithmetic");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(20));
    group.measurement_time(Duration::from_millis(100));
    for size in [1_usize, 4, 16, 256, 4096] {
        let mut math = CoordinateMath::new(limits()).expect("math admission");
        let fixed = FixedInterval::from_binary64(0.7, &mut math).expect("exact source");
        FixedInterval::pi(&mut math).expect("prepared pi");
        let source = FloatInterval::point(0.7).expect("finite source");
        math.prepare_binary64().expect("prepared binary64");
        let chunk = math.enter_chunk().expect("validated chunk");
        group.throughput(Throughput::Elements(size as u64));
        group.bench_function(format!("fixed_sin_cos/batch{size}"), |b| {
            b.iter(|| {
                for _ in 0..size {
                    black_box(fixed.sin_cos(black_box(&mut math)).expect("certified trig"));
                }
            });
        });
        group.bench_function(format!("float_sin_cos/batch{size}"), |b| {
            b.iter(|| {
                for _ in 0..size {
                    black_box(
                        source
                            .sin_cos(&mut math, &chunk)
                            .expect("certified fast trig"),
                    );
                }
            });
        });
        let left =
            FloatInterval::from_bounds(0.699_999_999_999_999_9, 0.700_000_000_000_000_1).unwrap();
        let right =
            FloatInterval::from_bounds(-1.200_000_000_000_000_2, -1.199_999_999_999_999_7).unwrap();
        group.bench_function(format!("float_mul/batch{size}"), |b| {
            b.iter(|| {
                for _ in 0..size {
                    black_box(black_box(left).mul(black_box(right), &chunk).unwrap());
                }
            });
        });
        group.bench_function(format!("float_square_sqrt/batch{size}"), |b| {
            b.iter(|| {
                for _ in 0..size {
                    black_box(
                        source
                            .square(&chunk)
                            .expect("square")
                            .sqrt(&chunk)
                            .expect("root"),
                    );
                }
            });
        });
    }
    group.finish();
}

fn product_paths(c: &mut Bench) {
    let mut group = c.benchmark_group("numerical_product_paths");
    group.sample_size(20);
    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_millis(500));
    let left =
        FloatInterval::from_bounds(0.699_999_999_999_999_9, 0.700_000_000_000_000_1).unwrap();
    let right =
        FloatInterval::from_bounds(-1.200_000_000_000_000_2, -1.199_999_999_999_999_7).unwrap();
    for path in FloatProductBackend::all_available() {
        let mut math = CoordinateMath::new(limits()).unwrap();
        math.set_binary64_backend(path).unwrap();
        let chunk = math.enter_chunk().unwrap();
        for size in [1_usize, 4, 16, 256, 4096] {
            group.throughput(Throughput::Elements(size as u64));
            group.bench_function(format!("{}/batch{size}", path.name()), |b| {
                b.iter(|| {
                    for _ in 0..size {
                        black_box(black_box(left).mul(black_box(right), &chunk).unwrap());
                    }
                });
            });
        }
    }
    group.finish();
}

fn integer(c: &mut Bench) {
    use purrdf_xsd::integer::Int;
    let mut group = c.benchmark_group("numerical_integer");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(20));
    group.measurement_time(Duration::from_millis(100));
    for (numerator_bits, denominator_bits) in [(191, 127), (255, 160), (511, 231)] {
        let numerator = Int::one()
            .shl(numerator_bits)
            .add(&Int::from_u128(0x81eb_e24d_e521_0389_9bc0_726e_1554_02af));
        let denominator = Int::one()
            .shl(denominator_bits)
            .add(&Int::from_u64(0xc61e_53a4_5e87_19cb));
        group.bench_function(
            format!("div{}by{}", numerator_bits + 1, denominator_bits + 1),
            |b| {
                b.iter(|| {
                    black_box(
                        black_box(&numerator)
                            .div_rem(black_box(&denominator))
                            .expect("nonzero denominator"),
                    )
                });
            },
        );
        group.bench_function(
            format!("gcd{}by{}", numerator_bits + 1, denominator_bits + 1),
            |b| {
                b.iter(|| black_box(black_box(&numerator).gcd(black_box(&denominator))));
            },
        );
    }
    group.finish();
}

bench_group!(benches, context, arithmetic, product_paths, integer);
bench_main!(benches);
