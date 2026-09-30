// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The multimodal fixture the facade's fused-retrieval tests and bench share: a
//! text corpus beside a vector space over the same subjects, its dataset and
//! text index, and the request that asks both.

// The module is included into more than one integration-test binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use std::sync::Arc;

use purrdf::retrieval::fixture::iri;
use purrdf::retrieval::{DomainTag, RequestTerm, Term};
use purrdf::text::{GraphSelector, TextIndex, TextIndexConfig};
use purrdf::{RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};

/// The one predicate the text corpus is indexed over.
pub const NOTE: &str = "https://example.org/note";

/// The one block both producers declare.
pub const SHARED_BLOCK: &str = "https://example.org/domain/shared";

/// The needle the text producer is asked for.
pub const NEEDLE: &str = "alpha beta";

/// How many documents the needle reaches, and how many rows the vector space holds.
pub const CORPUS: usize = 80;

pub fn text_subject(at: usize) -> String {
    format!("https://example.org/doc/text/{at}")
}

pub fn vector_term(at: usize) -> String {
    format!("https://example.org/doc/vec/{at}")
}

pub fn kernel_iri(text: &str) -> purrdf::iri::Iri {
    purrdf::iri::parse(text).expect("fixture IRIs are valid")
}

pub fn shared_block() -> DomainTag {
    DomainTag::parse(SHARED_BLOCK).expect("the fixture domain tag is a valid IRI")
}

/// [`CORPUS`] documents the needle reaches, and one document per vector term whose
/// text shares no term with it — so a text-side lookup about a vector candidate is a
/// real dictionary search that finds no posting.
pub fn text_rows() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = (0..CORPUS)
        .map(|at| {
            (
                text_subject(at),
                format!("alpha beta gamma {}", "alpha ".repeat(at % 4 + 1).trim()),
            )
        })
        .collect();
    out.extend((0..CORPUS).map(|at| (vector_term(at), "zulu yankee xray whiskey".to_owned())));
    out
}

pub fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri(NOTE);
    for (subject, text) in text_rows() {
        let subject = builder.intern_iri(&subject);
        let object = builder.intern_literal(RdfLiteral::simple(&text));
        builder.push_quad(subject, predicate, object, None);
    }
    builder.freeze().expect("the fixture dataset is valid")
}

pub fn text_index(dataset: &RdfDataset) -> Arc<TextIndex> {
    let config = TextIndexConfig::new(vec![TermValue::iri(NOTE)], GraphSelector::Any)
        .expect("the fixture configuration is well formed");
    Arc::new(TextIndex::from_dataset(dataset, &config).expect("the fixture index builds"))
}

pub fn request_terms() -> Vec<RequestTerm> {
    vec![
        RequestTerm::Lexical {
            text: NEEDLE.to_owned(),
            language: None,
            predicate: Some(iri(NOTE)),
        },
        RequestTerm::EntitySeed {
            entity: Term::new(format!("<{}>", vector_term(0))),
        },
    ]
}
