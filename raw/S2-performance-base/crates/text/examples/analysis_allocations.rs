// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Host allocation measurements, with setup and report formatting outside each window.
use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow, Measurement};
use purrdf_core::{RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_text::{
    Analyzer, AnalyzerProfile, AnalyzerScratch, GraphSelector, HanCharacterIndex, SurfaceIndex,
    TextIndex, TextIndexConfig,
};
use std::{error::Error, hint::black_box, path::Path};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn measure<T>(operation: impl FnOnce() -> T) -> (T, Measurement) {
    let window = CurrentThreadWindow::open();
    let result = operation();
    (result, window.close())
}
fn report(name: &str, count: usize, measurement: Measurement) {
    println!(
        "{name}\t{count}\t{}\t{}\t{}\t{}",
        measurement.allocations,
        measurement.requested_bytes,
        measurement.retained_bytes,
        measurement.peak_working_bytes
    );
}
fn main() -> Result<(), Box<dyn Error>> {
    let directory = std::env::args()
        .nth(1)
        .ok_or("usage: analysis_allocations ARTIFACT_DIRECTORY")?;
    let mut artifacts = std::fs::read_dir(Path::new(&directory))?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "cbor")
        })
        .map(std::fs::read)
        .collect::<Result<Vec<_>, _>>()?;
    artifacts.sort();
    let bytes = artifacts.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let (baseline, memory) = measure(|| Analyzer::resolve(AnalyzerProfile::standard(), &bytes));
    let baseline = baseline?;
    println!("operation\titerations\tallocations\trequested_bytes\tretained_bytes\tpeak_bytes");
    report("baseline_resolve", 1, memory);
    let empty = Analyzer::empty_lexicon();
    for (profile, analyzer) in [("empty", &empty), ("baseline", &baseline)] {
        for (class, text) in [
            (
                "ascii",
                "The quick brown fox ground.logic.ttl rdf:type 3.14 can't.",
            ),
            ("latin", "Straße Café naïve résumé œuvres coöperate Łódź."),
            (
                "mixed",
                "σοφός Москва عربي नमस्ते e\u{301}\u{323} ｒｕｓｔ 👩‍💻 🇨🇦",
            ),
            ("chinese", "中华人民共和国自然语言处理人工智能记忆检索"),
            ("thai", "กินภาษาไทยทดสอบการค้นหา"),
        ] {
            let mut scratch = AnalyzerScratch::default();
            analyzer.analyze_each_with_scratch(text, &mut scratch, |_| {})?;
            let (result, memory) = measure(|| {
                for _ in 0..1_000 {
                    analyzer.analyze_each_with_scratch(text, &mut scratch, |word| {
                        black_box(word);
                    })?;
                }
                Ok::<_, purrdf_text::TextError>(())
            });
            result?;
            report(&format!("{profile}/stream/{class}"), 1_000, memory);
            let (result, memory) = measure(|| analyzer.projections(text));
            black_box(result?);
            report(&format!("{profile}/aligned/{class}"), 1, memory);
        }
    }
    let rows = (0..1_024)
        .map(|id| format!("rdf{id:04}:中文记忆检索 ground.logic.ttl 👩‍💻"))
        .collect::<Vec<_>>();
    let (surface, memory) = measure(|| {
        SurfaceIndex::from_texts(
            baseline.clone(),
            rows.iter()
                .enumerate()
                .map(|(id, text)| (id as u32, text.as_str())),
        )
    });
    let surface = surface?;
    report("surface_build", rows.len(), memory);
    let (matches, memory) = measure(|| surface.substring_report("rdf0000"));
    black_box(matches?);
    report("substring_selective", 1, memory);
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("https://example.org/allocation/text");
    for (id, text) in rows.iter().enumerate() {
        let subject = builder.intern_iri(&format!("https://example.org/allocation/doc/{id}"));
        let literal = builder.intern_literal(RdfLiteral::simple(text));
        builder.push_quad(subject, predicate, literal, None);
    }
    let dataset = builder.freeze()?;
    let config = TextIndexConfig::new(
        vec![TermValue::iri("https://example.org/allocation/text")],
        GraphSelector::Any,
        baseline,
    )?;
    let (index, memory) = measure(|| TextIndex::from_dataset(&dataset, &config));
    black_box(index?);
    report("lexical_build", rows.len(), memory);
    let (index, memory) = measure(|| HanCharacterIndex::from_dataset(&dataset, &config));
    black_box(index?);
    report("han_build", rows.len(), memory);
    for count in [256, 512, 1_024, 2_048, 4_096] {
        let input = format!("A{}", "😀".repeat(count));
        let (analysis, memory) = measure(|| empty.projections(&input));
        black_box(analysis?);
        report(&format!("aligned/emoji/{count}"), 1, memory);
    }
    Ok(())
}
