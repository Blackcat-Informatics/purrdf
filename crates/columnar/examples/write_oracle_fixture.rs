// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Write a deterministic five-file fixture for external Parquet oracles.

use std::env;
use std::fs;
use std::path::PathBuf;

use purrdf_columnar::{Compression, write};
use purrdf_core::{BlankScope, ContentStore, RdfDatasetBuilder, RdfLiteral, RdfTextDirection};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: write_oracle_fixture OUTPUT_DIRECTORY")?;
    fs::create_dir_all(&output)?;

    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_blank("subject", BlankScope(5));
    let predicate = builder.intern_iri("https://example.org/predicate");
    let object = builder.intern_literal(RdfLiteral::typed("17", "https://example.org/integer"));
    let directional = builder.intern_literal(RdfLiteral {
        lexical_form: "مرحبا".to_owned(),
        datatype: None,
        language: Some("ar".to_owned()),
        direction: Some(RdfTextDirection::Rtl),
    });
    let graph = builder.intern_iri("https://example.org/graph");
    let empty_graph = builder.intern_iri("https://example.org/empty");
    builder.declare_named_graph(empty_graph);
    builder.push_quad(subject, predicate, object, None);
    builder.push_quad(subject, predicate, directional, Some(graph));

    let triple = builder.intern_triple(subject, predicate, directional);
    let reifier = builder.intern_blank("reifier", BlankScope(5));
    builder.push_reifier_in_graph(reifier, triple, Some(graph));
    builder.push_annotation_in_graph(reifier, predicate, object, None);
    let dataset = builder.freeze()?;

    let mut blobs = ContentStore::new();
    blobs.insert(b"first payload".to_vec());
    blobs.insert(b"second payload".to_vec());
    let encoded = write(&*dataset, &blobs, Compression::Zstd)?;
    assert!(
        encoded.losses.is_empty(),
        "columnar fixture must be lossless"
    );
    for (table, bytes) in encoded.files.iter() {
        fs::write(output.join(table.file_name()), bytes)?;
    }

    let empty_output = output.join("empty");
    fs::create_dir_all(&empty_output)?;
    let empty = RdfDatasetBuilder::new().freeze()?;
    let empty_encoded = write(&*empty, &ContentStore::new(), Compression::Uncompressed)?;
    for (table, bytes) in empty_encoded.files.iter() {
        fs::write(empty_output.join(table.file_name()), bytes)?;
    }
    Ok(())
}
