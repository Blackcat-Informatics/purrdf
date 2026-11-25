// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native work and allocation comparison for an unbound planar relation.
//! Run the same example on each revision being compared. Verify complete
//! output against the resident exact query before using the resource receipt.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_core::{RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_geo_kernel::{
    Crs, ExecutionLimits, ExecutionPolicy, GeoError, GeoVocabBuilder, MetricContext,
    MetricWorkObserver, SpatialRelation,
    binding::standard_reference,
    index::{GeoIndex, GeoIndexConfig, GraphSelector},
};
use std::error::Error;

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

const GEO: &str = "http://example.org/geo#";
const AS_WKT: &str = "http://example.org/geo#asWKT";
const WKT: &str = "http://example.org/geo#wktLiteral";
const ENTRIES: i32 = 48;

struct Observe;

impl MetricWorkObserver for Observe {
    fn charge_chunk(&mut self, _: u64, _: u64) -> Result<(), GeoError> {
        Ok(())
    }
}

fn index(spacing: i32, width: i32) -> Result<GeoIndex, Box<dyn Error>> {
    let crs = Crs::new("http://example.org/crs/planar")?;
    let vocab = GeoVocabBuilder::new(GEO, "http://example.org/geof/", crs.clone(), crs)?.build();
    let config = GeoIndexConfig::new(vec![TermValue::iri(AS_WKT)], GraphSelector::Any)?;
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri(AS_WKT)?;
    for ordinal in 0..ENTRIES {
        let left = ordinal * spacing;
        let right = left + width;
        let polygon = format!("POLYGON(({left} 0,{right} 0,{right} 2,{left} 2,{left} 0))");
        let subject = builder.intern_iri(&format!("http://example.org/shape/{ordinal:02}"))?;
        let geometry = builder.intern_literal(RdfLiteral::typed(&polygon, WKT))?;
        builder.push_quad(subject, predicate, geometry, None)?;
    }
    Ok(GeoIndex::from_dataset(
        &*builder.freeze()?,
        &vocab,
        &config,
    )?)
}

fn main() -> Result<(), Box<dyn Error>> {
    let policy = ExecutionPolicy::new(ExecutionLimits {
        max_work_items: 1_000_000_000_000,
        ..ExecutionLimits::GEOMETRY
    })?;
    let layout = [("sparse", 8, 2), ("overlap", 1, ENTRIES + 1)];
    let relation = SpatialRelation::SfIntersects;
    for (name, spacing, width) in layout {
        let index = index(spacing, width)?;
        let expected = index.relation_pair_references(relation, None, None, None)?;
        let mut context = MetricContext::from_reference(standard_reference(), policy)?;
        let window = CurrentThreadWindow::open();
        let actual = index.relation_pair_references_metered(
            relation,
            None,
            None,
            &mut context,
            &mut Observe,
        )?;
        let memory = window.close();
        assert_eq!(&*actual, &*expected, "ordered source ordinals for {name}");
        println!(
            "layout={name} entries={ENTRIES} rows={} work={} workspace={} allocation={memory:?}",
            actual.len(),
            context.work_items(),
            context.workspace_peak(),
        );
    }
    Ok(())
}
