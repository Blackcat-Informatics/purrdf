// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original selected implication steps licensing a checked causal schema clash.

use super::super::{ResolvedSchema, ResolvedSchemaStep};
use super::CausalClash;
use crate::owl_dl::proof::SchemaClashEvidence;

/// Checked bounds and their selected original class/qualifier derivations.
pub struct CausalSchema<'a> {
    pub(super) scope: CausalClash<'a>,
    pub(super) evidence: &'a SchemaClashEvidence,
}

impl<'a> CausalSchema<'a> {
    /// Complete checked native bound record and its normalized parameters.
    /// The selected iterators below identify the actual causal implication steps.
    #[must_use]
    pub fn bounds(&self) -> ResolvedSchema<'a> {
        ResolvedSchema {
            kb: self.scope.kb,
            bounds: &self.evidence.bounds,
        }
    }

    fn derivation(
        &self,
        qualifier: bool,
    ) -> impl Iterator<Item = ResolvedSchemaStep<'a>> + 'a + use<'a> {
        let scope = self.scope;
        let bounds = &self.evidence.bounds;
        scope
            .support
            .schema_reads
            .iter()
            .filter(move |read| read.is_qualifier() == qualifier)
            .map(move |read| super::super::schema::step(scope.kb, read.step(bounds)))
    }

    /// Selected class implications in their original checked dependency order.
    pub fn class_derivation(&self) -> impl Iterator<Item = ResolvedSchemaStep<'a>> + 'a + use<'a> {
        self.derivation(false)
    }

    /// Selected qualifier implications in their original checked dependency order.
    pub fn qualifier_derivation(
        &self,
    ) -> impl Iterator<Item = ResolvedSchemaStep<'a>> + 'a + use<'a> {
        self.derivation(true)
    }
}

impl std::fmt::Debug for CausalSchema<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CausalSchema")
            .field("bounds", &self.bounds())
            .field("selected_steps", &self.scope.support.schema_reads.len())
            .finish()
    }
}
