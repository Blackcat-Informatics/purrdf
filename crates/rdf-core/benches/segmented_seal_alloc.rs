// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Allocation windows for freezing reified claims and sealing them through
//! the public segmented writer. Each claim occupies one quad, one reifier
//! row and one annotation row. Report counts rather than asserting a threshold.

use std::hint::black_box;

use purrdf_alloc_probe::{CountingAllocator, Measurement, WholeProcessWindow};
use purrdf_core::{
    QuadIds, RdfDatasetBuilder, SegmentedBuildLimits, SegmentedBuilder, TermBox, TermValue,
};
use purrdf_lex::allocation::{Memory, Resident};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn claim(index: u32) -> [TermValue; 7] {
    let predicate = TermValue::iri("https://example.org/claim");
    let annotation = TermValue::iri("https://example.org/certainty");
    let subject = TermValue::iri(format!("https://example.org/entity/{index}"));
    let graph = TermValue::iri(format!("https://example.org/graph/{index}"));
    let reifier = TermValue::iri(format!("https://example.org/reifier/{index}"));
    let object = TermValue::typed_literal(index.to_string(), purrdf_xsd::datatype::XSD_INTEGER);
    let triple = TermValue::Triple {
        s: TermBox::new(subject.clone()),
        p: TermBox::new(predicate.clone()),
        o: TermBox::new(object.clone()),
    };
    [
        predicate, annotation, subject, graph, reifier, object, triple,
    ]
}

fn frozen_source(claims: u32) -> Measurement {
    let window = WholeProcessWindow::open();
    let mut builder = RdfDatasetBuilder::new();
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    for index in 0..claims {
        let [p, a, s, g, r, o, t] = claim(index).map(|value| {
            builder
                .intern_value_with_memory(&value, &mut memory)
                .unwrap()
        });
        builder.push_quad(s, p, o, Some(g)).unwrap();
        builder.push_reifier_in_graph(r, t, Some(g)).unwrap();
        builder.push_annotation_in_graph(r, a, o, Some(g)).unwrap();
    }
    let dataset = black_box(builder.freeze().unwrap());
    let measured = window.close();
    assert_eq!(dataset.quad_count(), claims as usize);
    assert_eq!(dataset.rdf_row_count(), claims as usize * 3);
    assert_eq!(dataset.named_graphs().count(), claims as usize);
    assert_eq!(dataset.term_count(), claims as usize * 5 + 4);
    measured
}

fn segmented_fixture(claims: u32) -> SegmentedBuilder {
    let limits = SegmentedBuildLimits::new(8192, 1_000_000, 4096, 512, 8).unwrap();
    let mut builder = SegmentedBuilder::new(limits).unwrap();
    for index in 0..claims {
        let mut ids = [None; 7];
        let mut next = 0;
        builder
            .intern_batch(&claim(index), |_, id| {
                ids[next] = Some(id);
                next += 1;
            })
            .unwrap();
        let [p, a, s, g, r, o, t] = ids.map(Option::unwrap);
        builder
            .push_quad(QuadIds {
                s,
                p,
                o,
                g: Some(g),
            })
            .unwrap();
        builder.push_reifier(r, t, Some(g)).unwrap();
        builder
            .push_annotation(QuadIds {
                s: r,
                p: a,
                o,
                g: Some(g),
            })
            .unwrap();
    }
    assert_eq!(builder.statement_count(), claims as usize * 3);
    builder
}

fn report(phase: &str, claims: u32, bytes: usize, measure: Measurement) {
    println!(
        "[segmented_seal_alloc] {phase}: claims={claims} image_bytes={bytes} allocations={} \
         requested_bytes={} retained_bytes={} peak_working_bytes={}",
        measure.allocations,
        measure.requested_bytes,
        measure.retained_bytes,
        measure.peak_working_bytes,
    );
}

fn main() {
    for claims in [128, 512] {
        report("build_freeze", claims, 0, frozen_source(claims));
        let builder = segmented_fixture(claims);
        let window = WholeProcessWindow::open();
        let image = black_box(builder.seal().unwrap());
        let measured = window.close();
        report("seal", claims, image.bytes().len(), measured);
        let digest = purrdf_hash::blake3::hash(image.bytes());
        let receipt = purrdf_hash::blake3::hash(&image.receipt().encode());
        println!(
            "claims={claims} image={} receipt={}",
            purrdf_hash::hex::encode(digest.as_bytes()),
            purrdf_hash::hex::encode(receipt.as_bytes()),
        );
    }
}
