pub use purrdf_core::{DatasetView, FastMap, GraphMatch, RdfDataset, TermId, TermRef};
pub mod model { pub use purrdf_core::model::RdfTextDirection; }
#[path = "baseline-turtle-render.rs"]
mod original;
#[path = "../../crates/rdf/tests/support/turtle_chains.rs"]
mod inputs;

fn main() {
    let output_dir = std::path::Path::new(".stage/turtle-writer-deep-blank-node-chains");
    let prefixes = vec![("ex".into(), "http://example.org/".into()),
        ("rdf".into(), purrdf_iri::vocab::rdf::NS.into())];
    for depth in [0, 1, 2, 31, 32, 33, 39, 40, 41] {
        let dataset = inputs::chain(depth, false, false);
        let original = original::render(&dataset, &prefixes);
        let current = purrdf_core::render_canonical_turtle(&dataset, &prefixes);
        let original_path = output_dir.join(format!("baseline-{depth}.ttl"));
        let current_path = output_dir.join(format!("current-{depth}.ttl"));
        std::fs::write(original_path, &original).unwrap();
        std::fs::write(current_path, &current).unwrap();
        println!("depth={depth} original={} current={} equal={}", original.len(), current.len(), original == current);
        // The saturation changes only lines whose original indentation exceeded
        // forty levels, never term spelling or punctuation.
        assert_eq!(original.lines().count(), current.lines().count());
        for (a, b) in original.lines().zip(current.lines()) {
            assert_eq!(a.trim_start(), b.trim_start());
            if a.len() - a.trim_start().len() <= 160 {
                assert_eq!(a, b);
            } else {
                assert_eq!(b.len() - b.trim_start().len(), 160);
            }
        }
    }
}
