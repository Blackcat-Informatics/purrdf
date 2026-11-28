// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fixed work of the text paths that publish a source identity or a
//! generation: allocations, requested and retained bytes, and retired
//! user-mode instructions.
//!
//! `cargo bench -p purrdf-text --bench source_identity_counts [-- <row filter>]`
//! prints one allocation line per row, measured in process. With `--counts`
//! (Linux and `perf`), each row also runs in fresh child processes under the
//! test kit's counter protocol and prints its instruction samples. Rows:
//!
//! * `index_build`: `TextIndex::from_dataset` over the fixture.
//! * `index_delta`: `TextIndex::with_delta` adding one row to that index.
//! * `source_digest`: `purrdf_text::source_digest` over the fixture, the walk
//!   `verify_binding` compares against an index.
//! * `surface_build`: `SurfaceIndex::from_texts` over the fixture's texts.
//! * `surface_delta`: `SurfaceIndex::with_documents` replacing one document.
//!
//! The fixture has `PURRDF_TEXT_IDENTITY_ROWS` rows (2048 by default). Fixture
//! construction and every input index are outside the window. The counting
//! allocator also runs in the instruction samples; its work is the same for
//! every build compared.

use std::hint::black_box;
use std::sync::Arc;

use purrdf_alloc_probe::CountingAllocator;
use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermFactory, TermValue};
use purrdf_text::{Analyzer, GraphSelector, SurfaceIndex, TextIndex, TextIndexConfig};

use purrdf_testkit::bench::counter::{CountRows, CountWork, run_count_rows};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const ROWS: [&str; 5] = [
    "index_build",
    "index_delta",
    "source_digest",
    "surface_build",
    "surface_delta",
];

const PREDICATE: &str = "https://example.org/resource/text";

fn text(index: usize) -> String {
    format!("river bank {index} 北京大学")
}

fn dataset(rows: impl IntoIterator<Item = (String, String)>) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(PREDICATE).unwrap();
    for (subject, text) in rows {
        let s = builder.intern_value(&TermValue::iri(subject)).unwrap();
        let o = builder
            .intern_value(&TermValue::simple_literal(text))
            .unwrap();
        builder.push_quad(s, p, o, None).unwrap();
    }
    builder.freeze().unwrap()
}

fn corpus(count: usize) -> Arc<RdfDataset> {
    dataset((0..count).map(|index| {
        (
            format!("https://example.org/document/{index:08}"),
            text(index),
        )
    }))
}

fn config() -> TextIndexConfig {
    TextIndexConfig::new(
        vec![TermValue::iri(PREDICATE)],
        GraphSelector::Default,
        Analyzer::empty_lexicon(),
    )
    .unwrap()
}

fn surface(texts: &[String]) -> SurfaceIndex {
    SurfaceIndex::from_texts(
        Analyzer::empty_lexicon(),
        texts
            .iter()
            .enumerate()
            .map(|(id, text)| (u32::try_from(id).unwrap(), text.as_str())),
    )
    .unwrap()
}

/// Prepare one row outside the window; the returned closure is the measured
/// work and returns its witness.
fn prepare(row: &str, count: usize) -> CountWork {
    let config = config();
    match row {
        "index_build" => {
            let source = corpus(count);
            Box::new(move || {
                let index = TextIndex::from_dataset(&*source, &config).unwrap();
                u64::from(black_box(index.source_fingerprint())[0])
            })
        }
        "index_delta" => {
            let source = corpus(count);
            let index = TextIndex::from_dataset(&*source, &config).unwrap();
            let addition = (
                "https://example.org/document/added".to_owned(),
                "aardvark bank".to_owned(),
            );
            let added = dataset([addition.clone()]);
            let empty = dataset([]);
            let next = dataset(
                (0..count)
                    .map(|index| {
                        (
                            format!("https://example.org/document/{index:08}"),
                            text(index),
                        )
                    })
                    .chain([addition]),
            );
            Box::new(move || {
                let updated = index.with_delta(&*next, &*added, &*empty).unwrap();
                u64::from(black_box(updated.fingerprint())[0])
            })
        }
        "source_digest" => {
            let source = corpus(count);
            Box::new(move || {
                let (digest, _) = purrdf_text::source_digest(&*source, &config).unwrap();
                u64::from(black_box(digest)[0])
            })
        }
        "surface_build" => {
            let texts: Vec<String> = (0..count).map(text).collect();
            Box::new(move || {
                let index = surface(&texts);
                u64::try_from(black_box(index).words().count()).unwrap()
            })
        }
        "surface_delta" => {
            let texts: Vec<String> = (0..count).map(text).collect();
            let index = surface(&texts);
            let middle = u32::try_from(count / 2).unwrap();
            let added =
                SurfaceIndex::from_texts(Analyzer::empty_lexicon(), [(middle, "aardvark bank")])
                    .unwrap();
            Box::new(move || {
                let updated = index.with_documents(added).unwrap();
                u64::try_from(black_box(updated).words().count()).unwrap()
            })
        }
        _ => panic!("unknown row {row}"),
    }
}

const SPEC: CountRows<'static> = CountRows {
    bench: "source_identity_counts",
    rows: &ROWS,
    size: ("PURRDF_TEXT_IDENTITY_ROWS", 2048),
    samples: "PURRDF_TEXT_IDENTITY_COUNT_SAMPLES",
};

fn main() {
    run_count_rows(&SPEC, &prepare, &purrdf_alloc_probe::whole_process_report);
}
