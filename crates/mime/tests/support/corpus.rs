// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The same original-byte/RDF transcript for tests and its native golden writer.

use purrdf_core::ContentDigest;
use purrdf_mime::{Limits, Profile, SourceDocument, Vocabulary, analyze, decode_document, project};
use purrdf_rdf::{NativeRdfFormat, parse_dataset, serialize_dataset_to_format};

pub(crate) fn serialization_transcript() -> String {
    let profile = Profile::new(
        "original",
        Vocabulary::under("https://example.org/mime#").unwrap(),
        Limits::unbounded(),
    )
    .unwrap();
    let corpus = [
        b"From: sender@example.org\r\nTo: recipient@example.org\r\nSubject: original\r\nReceived: same\r\nReceived: same\r\n\r\nbody\r\n".as_slice(),
        b"Content-Type: multipart/mixed; boundary=x\r\n\r\npreamble\r\n--x\r\nContent-Transfer-Encoding: base64\r\n\r\nYWJj\r\n--x--\r\nepilogue".as_slice(),
        b"Content-Type: message/rfc822\r\n\r\nSubject: forwarded\r\n\r\ninner\r\n".as_slice(),
        b" broken\nReceived: same\nReceived: same\nContent-Transfer-Encoding: quoted-printable\n\n=ZZ\xff".as_slice(),
        include_bytes!("../corpus/cpython/msg_01.txt").as_slice(),
        include_bytes!("../corpus/cpython/msg_02.txt").as_slice(),
    ];
    let mut transcript = Vec::new();
    for (index, bytes) in corpus.into_iter().enumerate() {
        let model = analyze(
            SourceDocument {
                id: "urn:example:message",
                bytes,
            },
            &profile,
        )
        .unwrap();
        if index == 5 {
            assert_eq!(model.parts.len(), 15);
            assert_eq!(model.parts[0].children.len(), 4);
        }
        let dataset = project(&model, &profile).unwrap();
        for format in [
            NativeRdfFormat::Turtle,
            NativeRdfFormat::NTriples,
            NativeRdfFormat::JsonLd,
        ] {
            let emitted = serialize_dataset_to_format(&*dataset, format, None).unwrap();
            let repeated = serialize_dataset_to_format(&*dataset, format, None).unwrap();
            assert_eq!(emitted.bytes, repeated.bytes);
            let reparsed = parse_dataset(&emitted.bytes, format.media_type(), None).unwrap();
            assert_eq!(
                decode_document(&reparsed, model.id(), &profile).unwrap(),
                bytes
            );
            purrdf_hash::frame::frame_le(&mut transcript, bytes);
            purrdf_hash::frame::frame_le(&mut transcript, format.media_type().as_bytes());
            purrdf_hash::frame::frame_le(&mut transcript, &emitted.bytes);
        }
    }
    format!("{}\n", ContentDigest::of(&transcript).to_hex())
}
