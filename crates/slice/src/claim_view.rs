// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native claim-view emission — PURRDF's internal
//! `generated/queries/observation-claim-view.rq` SPARQL CONSTRUCT.
//!
//! Unlike the standpoint projections ([`crate::standpoint_emit`], which re-express
//! PURRDF in *external* peer models) and the per-profile SPARQL projections
//! (the correspondence lowerings), this is an INTERNAL purrdf→purrdf view: it materialises
//! the legacy `purrdf:Observation` / `purrdf:StandpointClaim` query surface FROM the
//! canonical `purrdf:ClaimToken` layer, so generic "all observations about X"
//! consumers keep working after the proposition / claim-token / attitude /
//! evaluation separation. The unified observation is "a projected union view over
//! the four constructs — a convenience surface, generated, never the canonical
//! record" (the foundation). `purrdf:observedFeature` is back-filled from
//! `purrdf:expresses`; `purrdf:vantage` from the asserting agent
//! (`purrdf:wasAssociatedWith`). The output is byte-identical to the committed
//! `observation-claim-view.rq` (the parity gate).

use crate::mapping_support::{prefix_block, GENERATED_BANNER};

/// The committed file name of the internal observation union view.
pub const CLAIM_VIEW_FILE: &str = "observation-claim-view.rq";

/// Emit the internal observation union view: a CONSTRUCT that materialises
/// `purrdf:Observation` / `purrdf:StandpointClaim` triples from each
/// `purrdf:ClaimToken`, back-filling `purrdf:observedFeature` from `purrdf:expresses`
/// and `purrdf:vantage` from the asserting agent. Suppressed tokens
/// (`purrdf:displayable false`) are excluded (Principle 10).
///
/// Takes no DSL input — it is a constant template-coded query — so it is
/// infallible, matching the individual standpoint emitters.
pub fn emit_claim_view() -> String {
    let body = "CONSTRUCT {\n\
         \x20   ?tok a purrdf:Observation , purrdf:StandpointClaim ;\n\
         \x20       purrdf:observedFeature ?prop ;\n\
         \x20       purrdf:vantage ?who .\n\
         }\n\
         WHERE {\n\
         \x20   ?tok a purrdf:ClaimToken ;\n\
         \x20       purrdf:expresses ?prop .\n\
         \x20   OPTIONAL { ?tok purrdf:wasAssociatedWith ?who }\n\
         \x20   FILTER NOT EXISTS { ?tok purrdf:displayable false }\n\
         }\n"
    .to_string();
    let header = format!(
        "# Projection: PURRDF claim-token layer → Observation / StandpointClaim union view. {GENERATED_BANNER}\n\
         # Internal purrdf→purrdf view: materialises the legacy purrdf:Observation /\n\
         # purrdf:StandpointClaim query surface from canonical purrdf:ClaimToken individuals,\n\
         # so generic \"all observations about X\" consumers keep working after the\n\
         # proposition / claim-token / attitude / evaluation separation. observedFeature\n\
         # is back-filled from purrdf:expresses; vantage from the asserting agent\n\
         # (purrdf:wasAssociatedWith). Suppressed tokens (purrdf:displayable false) drop out.\n"
    );
    format!("{header}{}\n\n{body}", prefix_block(&body))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The repo root (two levels up from crates/slice).
    fn repo_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .unwrap()
    }

    #[test]
    fn claim_view_matches_committed() {
        let text = emit_claim_view();
        let committed_path = repo_root()
            .join("generated")
            .join("queries")
            .join(CLAIM_VIEW_FILE);
        if !committed_path.exists() {
            eprintln!(
                "skipping committed claim-view comparison; {} is absent",
                committed_path.display()
            );
            return;
        }
        let committed = std::fs::read_to_string(&committed_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", committed_path.display()));
        assert_eq!(text, committed, "claim view drifted from committed");
    }

    #[test]
    fn claim_view_constructs_observation_surface_from_claim_tokens() {
        let text = emit_claim_view();
        // Reads the canonical layer...
        assert!(text.contains("?tok a purrdf:ClaimToken"));
        assert!(text.contains("purrdf:expresses ?prop"));
        // ...and materialises the legacy observation surface.
        assert!(text.contains("purrdf:Observation , purrdf:StandpointClaim"));
        assert!(text.contains("purrdf:observedFeature ?prop"));
        // Suppression is honoured.
        assert!(text.contains("FILTER NOT EXISTS { ?tok purrdf:displayable false }"));
    }
}
