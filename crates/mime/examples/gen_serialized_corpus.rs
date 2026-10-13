// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Produce the frozen SHA256 transcript of original bytes and actual RDF output.
//! Run from the workspace with `cargo run -p purrdf-mime --example
//! gen_serialized_corpus > crates/mime/tests/golden/serialized-corpus.sha256`.
//! The tests and this writer call one fixture home; no serializer is restated.

#[path = "../tests/support/corpus.rs"]
mod corpus;

fn main() {
    print!("{}", corpus::serialization_transcript());
}
