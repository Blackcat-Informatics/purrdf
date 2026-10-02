// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fixture data for the relation-prebinding tests: an N-Triples document read
//! into a dataset, and the id a dataset gave one of its nodes.

// The module is included into more than one integration-test binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use std::sync::Arc;

use purrdf_core::{TermId, TermValue};
use purrdf_rdf::RdfDataset;

/// The dataset the N-Triples document `triples` reads as; a parse error fails
/// the test with every message.
pub fn ntriples(triples: &str) -> Arc<RdfDataset> {
    purrdf_shapes::text_ingest::parse_ntriples_to_dataset(triples)
        .unwrap_or_else(|errors| panic!("fixture data: {}", errors.join("\n")))
}

/// The id `dataset` holds `value` under; `what` names the node in the failure.
pub fn id_in(dataset: &RdfDataset, value: &TermValue, what: &str) -> TermId {
    dataset
        .term_id_by_value(value)
        .unwrap_or_else(|| panic!("{what} {value:?} is a node the dataset holds"))
}
