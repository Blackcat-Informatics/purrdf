// scratch, not a deliverable
use purrdf_core::{RdfDatasetBuilder, canonicalize};
use purrdf_entail::{Materialization, materialize};

fn main() {
    let builder = RdfDatasetBuilder::new();
    let ds = builder.freeze().expect("freeze empty");
    let (closed, report) = materialize(&ds, Materialization::OwlRl).expect("materialize");
    let nq = canonicalize(&closed).nquads;
    println!("=== PurRDF OwlRl closure of EMPTY dataset: {} lines ===", nq.lines().count());
    for l in nq.lines() {
        println!("{l}");
    }
    println!("=== rules fired ===");
    for (r, c) in report.rules_fired() {
        println!("{}={c}", r.as_str());
    }
}
