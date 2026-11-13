// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The shared snapshot-emission fixture: a varied RDF 1.2 dataset of a chosen
//! size and the one emission call both the allocation bench and the byte-identity
//! test route through.

use std::fmt::Write as _;
use std::sync::Arc;

use purrdf_iri::vocab::rdf::REIFIES;
use purrdf_rdf::gts_compose::{
    BlobRow, DEFAULT_RSYNCABLE_THRESHOLD, MediumPlan, SnapshotBuilder, emit_gts,
};
use purrdf_rdf::{RdfDataset, parse_dataset};
use purrdf_xsd::datatype::XSD_INTEGER;

/// Quads covering IRIs, plain, language, directional and typed literals, blank
/// subjects and objects, and named graphs; every eighth quad is a reifier with
/// an annotation row in a named graph or the default graph.
pub(super) fn datasets(quads: usize) -> Vec<Arc<RdfDataset>> {
    let mut nquads = String::new();
    for i in 0..quads {
        let subject = if i % 11 == 10 {
            format!("_:s{}", i % 32)
        } else {
            format!("<http://example.org/s/{}>", i / 2)
        };
        let object = match i % 8 {
            0 => format!("<http://example.org/o/{i}>"),
            1 => format!("\"text {i}\""),
            2 => format!("\"label {i}\"@en"),
            3 => format!("\"merhaba {i}\"@ar--rtl"),
            4 => format!("\"{i}\"^^<{XSD_INTEGER}>"),
            5 => format!("_:b{}", i % 64),
            6 => format!("\"valeur {i}\"@fr--ltr"),
            _ => "<http://example.org/o/shared>".to_owned(),
        };
        let graph = if i % 4 == 0 {
            format!(" <http://example.org/g/{}>", i % 5)
        } else {
            String::new()
        };
        writeln!(
            nquads,
            "{subject} <http://example.org/p/{}> {object}{graph} .",
            i % 7
        )
        .expect("a String accepts formatted text");
    }
    let mut trig = String::new();
    for k in 0..quads / 8 {
        let statement = format!(
            "<<( <http://example.org/s/{k}> <http://example.org/p/{}> <http://example.org/o/{k}> )>>",
            k % 7
        );
        let body = format!(
            "<http://example.org/r/{k}> <{REIFIES}> {statement} ; <http://example.org/evidence> \"e{k}\" ."
        );
        if k % 3 == 0 {
            writeln!(trig, "<http://example.org/g/{}> {{ {body} }}", k % 5)
                .expect("a String accepts formatted text");
        } else {
            writeln!(trig, "{body}").expect("a String accepts formatted text");
        }
    }
    // A triple term nested inside another, in object position.
    writeln!(
        trig,
        "<http://example.org/n> <http://example.org/p/0> <<( <http://example.org/a> <http://example.org/b> <<( <http://example.org/c> <http://example.org/d> \"e\"@en--rtl )>> )>> ."
    )
    .expect("a String accepts formatted text");
    vec![
        parse_dataset(nquads.as_bytes(), "application/n-quads", None).expect("fixture quads parse"),
        parse_dataset(trig.as_bytes(), "application/trig", None).expect("fixture statements parse"),
    ]
}

/// A builder holding every dataset.
pub(super) fn builder(datasets: &[Arc<RdfDataset>]) -> SnapshotBuilder {
    let mut builder = SnapshotBuilder::new();
    for dataset in datasets {
        builder.add_dataset(dataset).expect("fixture ingests");
    }
    builder
}

/// Two blobs ahead of the snapshot: one under, one over the rsyncable threshold.
pub(super) fn blobs() -> Vec<BlobRow> {
    vec![
        BlobRow {
            data: b"blob under the threshold".to_vec(),
            media_type: "text/plain".to_owned(),
            rep: "rep-a".to_owned(),
        },
        BlobRow {
            data: vec![7; DEFAULT_RSYNCABLE_THRESHOLD + 1],
            media_type: "application/octet-stream".to_owned(),
            rep: "rep-b".to_owned(),
        },
    ]
}

/// The signing material of the signed cases: a fixed secret, key id and armor.
pub(super) fn signer() -> ([u8; 32], String, String) {
    (
        [0x42; 32],
        "kid-1".to_owned(),
        "-----BEGIN PGP PUBLIC KEY BLOCK-----\nfixture\n-----END PGP PUBLIC KEY BLOCK-----"
            .to_owned(),
    )
}

/// The one emission call. `signed` adds the transport-key meta frame and
/// signatures; `with_blobs` adds the two blob frames.
pub(super) fn emit(
    builder: SnapshotBuilder,
    chain: Option<Vec<String>>,
    signed: bool,
    with_blobs: bool,
) -> Vec<u8> {
    let plan = MediumPlan::dist_default(chain.as_deref());
    let (secret, kid, armor) = signer();
    let doc_blobs = if with_blobs { blobs() } else { Vec::new() };
    emit_gts(
        &builder,
        "dist",
        chain,
        doc_blobs,
        Vec::new(),
        signed.then_some(secret),
        signed.then_some(kid),
        signed.then_some(armor),
        DEFAULT_RSYNCABLE_THRESHOLD,
        &plan,
    )
    .expect("fixture emits")
}
