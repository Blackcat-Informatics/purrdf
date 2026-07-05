// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RDF text/bytes ingress into the frozen [`RdfDataset`] IR.
//!
//! The text codec is the oxigraph-free native [`parse_dataset`]
//! path (S3); the read model handed to PurRDF consumers is the
//! concrete IR. This module is deliberately PyO3-free so logic, SHACL, and pipeline
//! stages can route parsed inputs through the same `RdfDataset` path as the Python
//! `RdfDataset` handle. The already-parsed-quads → IR fold lives in
//! [`crate::native_quads::dataset_from_quads`] (it reuses the SHARED
//! `fold_statement_layer` so the
//! native-quads path and the text path can never drift).

use std::sync::Arc;

use crate::{parse_dataset, NativeRdfFormat, RdfDataset};

/// Parse RDF bytes and freeze them into a validated [`RdfDataset`] via the native
/// codec path.
///
/// `format` is a [`NativeRdfFormat`] codec selector — the workspace-wide sweep
/// (Tasks 2–6) routed every call site onto the native enum. The RDF 1.2 statement
/// layer is folded in: a `rdf:reifies` triple-term object becomes a reifier binding,
/// and a reifier subject's other triples become annotations (matching the GTS
/// producer's `add_rdf12` pass structure).
pub fn dataset_from_bytes(
    bytes: &[u8],
    format: NativeRdfFormat,
) -> Result<Arc<RdfDataset>, String> {
    let media_type = format.media_type();
    parse_dataset(bytes, media_type, None).map_err(|e| format!("parse error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dataset_from_bytes_counts_quads() {
        let nt = "<https://e/s> <https://e/p> <https://e/o> .\n\
                  <https://e/s> <https://e/p2> \"lit\" .\n";
        let ds = dataset_from_bytes(nt.as_bytes(), NativeRdfFormat::NTriples).expect("build");
        assert_eq!(ds.quad_count(), 2);
        assert!(ds.term_count() >= 4);
    }

    #[test]
    fn dataset_from_bytes_classifies_rdf12_statement_layer() {
        // A reifier's reifies binding + an annotation: the base quad table is
        // empty, the reifier binding and annotation land in their own tables.
        let nt = concat!(
            "<https://e/r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ",
            "<<( <https://e/s> <https://e/p> <https://e/o> )>> .\n",
            "<https://e/r> <https://e/confidence> \"0.9\" .\n",
        );
        let ds = dataset_from_bytes(nt.as_bytes(), NativeRdfFormat::NTriples).expect("build");
        assert_eq!(ds.quad_count(), 0, "reifier rows are not base quads");
        assert_eq!(ds.reifiers().count(), 1);
        assert_eq!(ds.annotations().count(), 1);
    }

    #[test]
    fn dataset_from_bytes_routes_each_native_format() {
        // The codec selector is the native enum across every format — the sweep
        // removed the temporary oxigraph::io::RdfFormat From shim entirely.
        let nq = "<https://e/s> <https://e/p> <https://e/o> <https://e/g> .\n";
        let ds = dataset_from_bytes(nq.as_bytes(), NativeRdfFormat::NQuads).expect("build nquads");
        assert_eq!(ds.quad_count(), 1);
    }
}
