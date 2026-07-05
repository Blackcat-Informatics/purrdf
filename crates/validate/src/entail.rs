// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL rule entailment → canonical N-Triples in one call — the shared boundary
//! the language bindings (C-ABI, WASM, and the Python caller) all route through.
//!
//! The entailment twin of [`crate::shacl::validate_to_sarif_string`]: where the
//! validation boundary runs the SHACL engine and renders a
//! [`ValidationReport`](purrdf_shapes::report::ValidationReport) to SARIF, this
//! boundary applies every active `sh:rule` to a fixpoint (via
//! [`engine::entail_graphs`]) and serializes the MATERIALIZED dataset — the base
//! graph plus every inferred triple — to a canonical, byte-deterministic
//! N-Triples string. Hoisting the sequence here keeps each binding to its
//! platform-specific wrapping (buffer, `JsValue`, `str`).
//!
//! Wasm-clean: pure in-memory string work over the wasm-clean SHACL engine and
//! the native RDFC-1.0 serializer — no new dependencies and no ambient I/O.
//!
//! [`engine::entail_graphs`]: purrdf_shapes::engine::entail_graphs

use purrdf_shapes::engine;

/// Entail `data_nt` (N-Triples) under `shapes_ttl` (Turtle) and serialize the
/// materialized dataset (base graph ⊎ every SHACL-AF rule inference) to a
/// canonical N-Triples string.
///
/// This is the single entry point every language binding shares: it parses the
/// two graphs, applies every active `sh:rule` to a fixpoint, and renders the
/// resulting dataset via the native RDFC-1.0 flat serializer (deterministic,
/// blank-node-canonical), returning a `String` error (the engine's own
/// parse/rule error) so callers can map it to whatever their platform expects.
///
/// # Errors
///
/// Returns the SHACL engine's error string if either graph fails to parse or if
/// rule application fails (an illegal head term, an unresolvable `sh:condition`,
/// or a rule set that does not reach a fixpoint).
///
/// # Examples
///
/// ```
/// use purrdf_validate::entail_to_ntriples_string;
///
/// let shapes = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
///     @prefix ex: <http://example.org/> .\n\
///     ex:PersonRule a sh:NodeShape ;\n\
///       sh:targetClass ex:Person ;\n\
///       sh:rule [ a sh:TripleRule ;\n\
///         sh:subject sh:this ; sh:predicate ex:adult ; sh:object ex:yes ] .\n";
/// let data = "<http://example.org/alice> \
///     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n";
///
/// let nt = entail_to_ntriples_string(shapes, data).expect("entailment produced");
/// assert!(nt.contains("<http://example.org/adult>"));
/// ```
pub fn entail_to_ntriples_string(shapes_ttl: &str, data_nt: &str) -> Result<String, String> {
    let dataset = engine::entail_graphs(data_nt, shapes_ttl)?;
    purrdf_rdf::canonical_flat_nquads(dataset.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHAPES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
        @prefix ex: <http://example.org/> .\n\
        ex:PersonRule a sh:NodeShape ;\n\
          sh:targetClass ex:Person ;\n\
          sh:rule [ a sh:TripleRule ;\n\
            sh:subject sh:this ; sh:predicate ex:adult ; sh:object ex:yes ] .\n";

    const DATA: &str = "<http://example.org/alice> \
        <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n";

    #[test]
    fn entail_materializes_the_inferred_triple() {
        let nt = entail_to_ntriples_string(SHAPES, DATA).expect("entailment produced");
        // The inferred head triple appears.
        assert!(nt.contains(
            "<http://example.org/alice> <http://example.org/adult> \
            <http://example.org/yes> ."
        ));
        // The base triple survives into the materialized dataset.
        assert!(nt.contains(
            "<http://example.org/alice> \
            <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> ."
        ));
    }

    #[test]
    fn entail_is_deterministic() {
        let a = entail_to_ntriples_string(SHAPES, DATA).expect("entailment produced");
        let b = entail_to_ntriples_string(SHAPES, DATA).expect("entailment produced");
        assert_eq!(a, b, "entailment serialization must be byte-stable");
    }

    #[test]
    fn malformed_shapes_is_an_error() {
        assert!(entail_to_ntriples_string("@@@ not turtle", DATA).is_err());
    }
}
