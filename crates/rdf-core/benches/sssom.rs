// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(
    missing_docs,
    reason = "criterion_group! expands to a public harness function that is not library API"
)]

//! Native SSSOM document-layout hot-path benchmark.
//!
//! Report-only: layout validation and deterministic serialization are measured
//! without asserting a timing threshold.

use std::{collections::BTreeMap, time::Duration};

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf_core::{
    SssomColumnLayout, SssomMapping, SssomMappingSet, SssomMeta, sssom::serialize_tsv,
};

const ROWS: usize = 4_096;
const LAYOUT_COLUMNS: [&str; 32] = [
    "subject_id",
    "subject_label",
    "predicate_id",
    "object_id",
    "object_label",
    "mapping_justification",
    "confidence",
    "comment",
    "x_extension_00",
    "x_extension_01",
    "x_extension_02",
    "x_extension_03",
    "x_extension_04",
    "x_extension_05",
    "x_extension_06",
    "x_extension_07",
    "x_extension_08",
    "x_extension_09",
    "x_extension_10",
    "x_extension_11",
    "x_extension_12",
    "x_extension_13",
    "x_extension_14",
    "x_extension_15",
    "x_extension_16",
    "x_extension_17",
    "x_extension_18",
    "x_extension_19",
    "x_extension_20",
    "x_extension_21",
    "x_extension_22",
    "x_extension_23",
];

fn sparse_mapping_set() -> SssomMappingSet {
    let mappings = (0..ROWS)
        .map(|index| {
            let mut extras = BTreeMap::new();
            extras.insert(format!("x_group_{:02}", index % 4), index.to_string());
            SssomMapping {
                subject_id: format!("ex:subject-{index:04}"),
                subject_label: (index + 1 == ROWS).then(|| "last subject".to_owned()),
                predicate_id: "skos:exactMatch".to_owned(),
                object_id: format!("ex:object-{index:04}"),
                object_label: (index + 2 == ROWS).then(|| "penultimate object".to_owned()),
                mapping_justification: "semapv:ManualMappingCuration".to_owned(),
                confidence: (index % 17 == 0).then_some(0.75),
                comment: (index == ROWS / 2).then(|| "sparse comment".to_owned()),
                extras,
            }
        })
        .collect();
    let layout = SssomColumnLayout::new(["object_id", "x_retained", "subject_id"])
        .expect("benchmark layout is valid");
    SssomMappingSet::new(SssomMeta::default(), mappings).with_column_layout(layout)
}

fn bench_layout_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("sssom_layout");
    group.throughput(Throughput::Elements(LAYOUT_COLUMNS.len() as u64));
    group.bench_function("validated_32_columns", |bencher| {
        bencher.iter(|| {
            let layout = SssomColumnLayout::new(black_box(LAYOUT_COLUMNS))
                .expect("benchmark layout is valid");
            black_box(layout);
        });
    });
    group.finish();
}

fn bench_sparse_serialization(c: &mut Criterion) {
    let set = sparse_mapping_set();
    let mut group = c.benchmark_group("sssom_serialize");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(3));
    group.throughput(Throughput::Elements(set.mappings.len() as u64));
    group.bench_function("sparse_4k_rows", |bencher| {
        bencher.iter(|| {
            let serialized = serialize_tsv(black_box(&set));
            black_box(serialized);
        });
    });
    group.finish();
}

criterion_group!(benches, bench_layout_validation, bench_sparse_serialization);
criterion_main!(benches);
