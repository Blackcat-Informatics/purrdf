// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Stable query ownership for borrowed plans and admitted runtime substitutions.

use super::OrdinaryPosition;
use crate::expr::{AdmittedQuery, PreparedPattern};
use crate::workspace::NativeSharedWorkspace;
use purrdf_sparql_algebra::Query;

pub(crate) enum CursorQuery<'q> {
    Borrowed(&'q Query),
    Owned(NativeSharedWorkspace<AdmittedQuery>),
}

impl CursorQuery<'_> {
    pub(super) fn query(&self) -> &Query {
        match self {
            Self::Borrowed(query) => query,
            Self::Owned(query) => &query.query,
        }
    }
}

impl<'q> CursorQuery<'q> {
    pub(super) fn position(&self) -> OrdinaryPosition<'q> {
        match self {
            Self::Borrowed(query) => OrdinaryPosition::Borrowed(super::query_pattern(query)),
            Self::Owned(query) => {
                OrdinaryPosition::Copied(PreparedPattern::Query(query.clone()).into_cursor())
            }
        }
    }
}
