// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Candidate checking, contract preparation and reuse; timings are report-only.

#[path = "../tests/support/scope_candidates.rs"]
mod candidates;

use std::collections::BTreeMap;

use purrdf_testkit::bench::{Bench, bench_group, bench_main, black_box};

fn scope_checks(bench: &mut Bench) {
    for (name, evidence) in candidates::measured_inputs() {
        let contract = candidates::Contract::prepare(&evidence).expect("valid input");
        let correspondence = BTreeMap::new();
        let mut group = bench.benchmark_group(format!("scope_checks/{name}"));
        group.bench_function("existing_check", |sample| {
            sample.iter(|| {
                black_box(candidates::existing(black_box(&evidence)))
                    .expect("valid existing input");
            });
        });
        group.bench_function("explicit_check", |sample| {
            sample.iter(|| {
                black_box(candidates::explicit(black_box(&evidence)))
                    .expect("valid explicit input");
            });
        });
        group.bench_function("encoded_variable_declaration_prepare", |sample| {
            sample
                .iter_with_large_drop(|| candidates::declare_variables(black_box(&evidence.query)));
        });
        group.bench_function("contract_prepare", |sample| {
            sample.iter_with_large_drop(|| {
                candidates::Contract::prepare(black_box(&evidence)).expect("valid contract input")
            });
        });
        group.bench_function("contract_reuse_check", |sample| {
            sample.iter(|| {
                black_box(contract.check(black_box(&evidence), black_box(&correspondence)))
                    .expect("valid contract output");
            });
        });
        group.finish();
    }
}

bench_group!(benches, scope_checks);
bench_main!(benches);
