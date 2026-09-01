// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL rule entailment → canonical N-Triples in one call — the shared boundary
//! the language bindings (C-ABI, WASM, and the Python caller) all route through.
//!
//! That claim is checkable, not decorative: `purrdf-rdf-capi`'s
//! `purrdf_shacl_entail_to_ntriples`, `purrdf-wasm`'s `shacl_entail`, and
//! `purrdf-python`'s `shacl.entail` each call
//! [`entail_to_ntriples_string`] and add only their own platform wrapping (a
//! caller-owned buffer, a `JsError`, a GIL release plus a `ValueError`). The
//! Python binding used to inline the two-line body instead; the golden in
//! [`PYTHON_BINDING_GOLDEN`] pins the bytes that inline copy produced, so the
//! re-point is provably byte-identical rather than merely believed to be.
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
/// The writer is [`purrdf_rdf::canonical_flat_nquads`], which is graph-CARRYING —
/// it spells out a fourth term for any row in a named graph, and it re-materializes
/// the RDF 1.2 statement layer in the graph that asserted it. The result is
/// nonetheless N-Triples, byte for byte, and that is a property of the INPUTS
/// rather than of the writer: an N-Triples data graph and a Turtle shapes graph
/// are both single-graph syntaxes and `sh:rule` inferences land beside the base
/// graph, so no row this function can produce has a graph name to write. Naming
/// the writer here rather than only its output is deliberate — a caller who
/// widened either input to a quad syntax would get quads, not a silent drop.
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
/// let nt = entail_to_ntriples_string(shapes, None, data).expect("entailment produced");
/// assert!(nt.contains("<http://example.org/adult>"));
/// ```
pub fn entail_to_ntriples_string(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    data_nt: &str,
) -> Result<String, String> {
    let dataset = engine::entail_graphs(data_nt, shapes_ttl, shapes_base)?;
    purrdf_rdf::canonical_flat_nquads(dataset.as_ref())
}

/// The shapes graph of the Python-binding byte-identity golden.
///
/// A `sh:TripleRule` that types every `ex:Person` as `ex:adult`.
pub const PYTHON_BINDING_GOLDEN_SHAPES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
    @prefix ex: <http://example.org/> .\n\
    ex:PersonRule a sh:NodeShape ;\n\
      sh:targetClass ex:Person ;\n\
      sh:rule [ a sh:TripleRule ;\n\
        sh:subject sh:this ; sh:predicate ex:adult ; sh:object ex:yes ] .\n";

/// The data graph of the Python-binding byte-identity golden.
///
/// Two targets (so the rule fires more than once) and a blank node (so the
/// RDFC-1.0 canonical relabelling is exercised, which is the part of the output
/// most likely to move if the two spellings ever stopped agreeing).
pub const PYTHON_BINDING_GOLDEN_DATA: &str = "<http://example.org/alice> \
    <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n\
    <http://example.org/bob> \
    <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n\
    <http://example.org/bob> <http://example.org/knows> _:b0 .\n";

/// The exact bytes `purrdf-python`'s `shacl.entail` produced for
/// [`PYTHON_BINDING_GOLDEN_SHAPES`] / [`PYTHON_BINDING_GOLDEN_DATA`] while it
/// still inlined `engine::entail_graphs` + `canonical_flat_nquads` itself.
///
/// Captured by running that inline sequence verbatim *before* the binding was
/// re-pointed at [`entail_to_ntriples_string`], so the equality test below is a
/// before/after comparison rather than a restatement of the current code.
pub const PYTHON_BINDING_GOLDEN: &str = "\
<http://example.org/alice> <http://example.org/adult> <http://example.org/yes> .
<http://example.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .
<http://example.org/bob> <http://example.org/adult> <http://example.org/yes> .
<http://example.org/bob> <http://example.org/knows> _:c14n0 .
<http://example.org/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .
";

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
        let nt = entail_to_ntriples_string(SHAPES, None, DATA).expect("entailment produced");
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
        let a = entail_to_ntriples_string(SHAPES, None, DATA).expect("entailment produced");
        let b = entail_to_ntriples_string(SHAPES, None, DATA).expect("entailment produced");
        assert_eq!(a, b, "entailment serialization must be byte-stable");
    }

    #[test]
    fn malformed_shapes_is_an_error() {
        assert!(entail_to_ntriples_string("@@@ not turtle", None, DATA).is_err());
    }

    /// Re-pointing `purrdf-python`'s `shacl.entail` at this function changed no
    /// bytes.
    ///
    /// [`PYTHON_BINDING_GOLDEN`] was captured by running the binding's OLD inline
    /// body — `purrdf_shapes::engine::entail_graphs(data, shapes, None)` followed by
    /// `purrdf_rdf::canonical_flat_nquads(dataset.as_ref())`, in that order —
    /// against these two fixtures before the edit. Asserting the shared function
    /// reproduces it is therefore a before/after equality, and it is the fixture
    /// a later refactor of either spelling has to keep satisfying.
    #[test]
    fn the_python_binding_repoint_is_byte_identical() {
        let produced = entail_to_ntriples_string(
            PYTHON_BINDING_GOLDEN_SHAPES,
            None,
            PYTHON_BINDING_GOLDEN_DATA,
        )
        .expect("entailment produced");
        assert_eq!(produced, PYTHON_BINDING_GOLDEN);
        // The golden is not vacuous: the rule fired for both targets, and the
        // blank node really was canonically relabelled.
        assert_eq!(produced.matches("<http://example.org/adult>").count(), 2);
        assert!(produced.contains("_:c14n0"));
        assert!(!produced.contains("_:b0"));
    }
}
