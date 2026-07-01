// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The result-provenance carrier — the "maximal information flow" extension that
//! travels alongside a SPARQL result.
//!
//! When [`ResultProvenance`] is empty (the common case today) the serializers
//! emit pure-W3C output. When populated, the JSON/XML writers (Tasks 2–3) append
//! an additive `purrdf` extension block; the CSV/TSV writers cannot carry it and
//! flag the drop via `SerializeOutcome::provenance_dropped`.
//!
//! Honesty note: population of this structure is **incremental**. The fields are
//! typed and threaded through the serializer surface now, but the evaluator and
//! the S11 (#917) derivation graph fill them in progressively — most results
//! today carry an empty value.

/// Result-level provenance carried alongside a SPARQL result. Default is empty →
/// pure-W3C serialization. Populated → additive `purrdf` extension in JSON/XML.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResultProvenance {
    /// Optional opaque query identity (e.g. a content hash of the source query).
    pub query_hash: Option<String>,
    /// Optional engine/producer label.
    pub engine: Option<String>,
    /// Per-solution provenance, index-aligned with `SparqlResult::Solutions.rows`
    /// when present. Empty = none carried (the common case today).
    pub solutions: Vec<SolutionProvenance>,
}

impl ResultProvenance {
    /// True when no provenance is carried: no query hash, no engine label, and no
    /// per-solution entries. Serializers use this to decide whether to append the
    /// additive `purrdf` extension at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.query_hash.is_none() && self.engine.is_none() && self.solutions.is_empty()
    }
}

/// Per-solution provenance hook. Typed-but-mostly-empty today; populated as the
/// evaluator / S11 (#917) derivation graph begins producing it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SolutionProvenance {
    /// Source references (e.g. named-graph / quad IRIs) that produced this solution.
    pub sources: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_provenance_is_empty() {
        assert!(ResultProvenance::default().is_empty());
    }

    #[test]
    fn query_hash_makes_it_non_empty() {
        let prov = ResultProvenance {
            query_hash: Some("abc".to_string()),
            ..Default::default()
        };
        assert!(!prov.is_empty());
    }

    #[test]
    fn engine_makes_it_non_empty() {
        let prov = ResultProvenance {
            engine: Some("purrdf-sparql-eval".to_string()),
            ..Default::default()
        };
        assert!(!prov.is_empty());
    }

    #[test]
    fn solutions_make_it_non_empty() {
        let prov = ResultProvenance {
            solutions: vec![SolutionProvenance::default()],
            ..Default::default()
        };
        assert!(!prov.is_empty());
    }
}
