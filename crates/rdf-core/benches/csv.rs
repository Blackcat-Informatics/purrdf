// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

#![allow(
    missing_docs,
    reason = "criterion_group! expands to a public harness function that is not library API"
)]

//! `purrdf_core::csv` read and write throughput over 1 MiB inputs.
//!
//! Two shapes: an SSSOM-shaped TSV body (CURIE columns, a label, a
//! confidence, a comment that sometimes holds a tab and so is quoted) read with
//! `Dialect::SSSOM_TSV`, and a CSVW-shaped CSV table (quoted cells holding
//! commas, doubled quotes and line breaks; long unquoted text cells) read both
//! by the streaming `Reader` and by the CSVW table algorithm `read_table`. Each
//! input is also written back. Report-only: no timing threshold is asserted.

use std::fmt::Write as _;
use std::time::Duration;

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf_core::csv::{Dialect, QuoteStyle, Reader, StringRecord, Writer, read_table};

/// The input size each shape is grown to.
const TARGET_BYTES: usize = 1 << 20;

/// An SSSOM mapping-table body of at least [`TARGET_BYTES`].
fn sssom_shaped() -> Vec<u8> {
    let mut text = String::from(
        "subject_id\tsubject_label\tpredicate_id\tobject_id\tobject_label\t\
         mapping_justification\tconfidence\tcomment\n",
    );
    let mut row = 0_usize;
    while text.len() < TARGET_BYTES {
        let comment = if row.is_multiple_of(7) {
            // A tab inside a value is quoted.
            format!("\"reviewed\tbatch {}\"", row / 7)
        } else {
            String::new()
        };
        let _ = writeln!(
            text,
            "ex:S{row:06}\tsubject number {row}\tskos:exactMatch\tex:O{row:06}\t\
             object number {row}\tsemapv:ManualMappingCuration\t0.{}\t{comment}",
            row % 10
        );
        row += 1;
    }
    text.into_bytes()
}

/// A CSVW-shaped table of at least [`TARGET_BYTES`].
fn csvw_shaped() -> Vec<u8> {
    let mut text = String::from("id,name,homepage,description,price\n");
    let mut row = 0_usize;
    while text.len() < TARGET_BYTES {
        let description = match row % 4 {
            0 => "\"a cell with a comma, and \"\"quotes\"\" inside\"".to_owned(),
            1 => "\"a cell that spans\ntwo lines\"".to_owned(),
            _ => "an ordinary unquoted description of moderate length".to_owned(),
        };
        let _ = writeln!(
            text,
            "{row},Item {row},https://example.org/items/{row},{description},{}.{:02}",
            row % 1000,
            row % 100
        );
        row += 1;
    }
    text.into_bytes()
}

fn read_all(dialect: Dialect<'_>, bytes: &[u8]) -> Vec<StringRecord> {
    let mut reader = Reader::new(dialect, bytes);
    reader
        .records()
        .collect::<Result<_, _>>()
        .expect("the benchmark input is well-formed")
}

fn write_all(dialect: Dialect<'_>, records: &[StringRecord]) -> Vec<u8> {
    let mut writer = Writer::new(dialect, Vec::with_capacity(TARGET_BYTES + TARGET_BYTES / 4));
    for record in records {
        writer.write_record(record).expect("in memory");
    }
    writer.into_inner().expect("in memory")
}

fn bench_csv(c: &mut Criterion) {
    let sssom = sssom_shaped();
    let csvw = csvw_shaped();
    let csvw_dialect = Dialect {
        header_row_count: 0,
        comment_prefix: None,
        ..Dialect::CSVW_CELLS
    };
    let sssom_records = read_all(Dialect::SSSOM_TSV, &sssom);
    let csvw_records = read_all(csvw_dialect, &csvw);

    let mut group = c.benchmark_group("csv");
    group.sample_size(20);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(3));

    group.throughput(Throughput::Bytes(sssom.len() as u64));
    group.bench_function("read_sssom_tsv_1mib", |bencher| {
        bencher.iter(|| black_box(read_all(Dialect::SSSOM_TSV, black_box(&sssom))));
    });
    group.bench_function("write_sssom_tsv_1mib", |bencher| {
        bencher.iter(|| black_box(write_all(Dialect::SSSOM_TSV, black_box(&sssom_records))));
    });

    group.throughput(Throughput::Bytes(csvw.len() as u64));
    group.bench_function("read_csvw_records_1mib", |bencher| {
        bencher.iter(|| black_box(read_all(csvw_dialect, black_box(&csvw))));
    });
    group.bench_function("read_csvw_table_1mib", |bencher| {
        bencher.iter(|| {
            black_box(
                read_table(&Dialect::CSVW_CELLS, black_box(&csvw))
                    .expect("the benchmark input is well-formed"),
            )
        });
    });
    group.bench_function("write_csvw_1mib", |bencher| {
        let necessary = Dialect {
            quote_style: QuoteStyle::Necessary,
            ..csvw_dialect
        };
        bencher.iter(|| black_box(write_all(necessary, black_box(&csvw_records))));
    });
    group.finish();
}

criterion_group!(benches, bench_csv);
criterion_main!(benches);
