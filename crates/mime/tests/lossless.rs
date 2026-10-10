// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original-byte, malformed-input, graph-refusal and real SPARQL contracts.

use purrdf_mime::{
    Document, Limits, MimeError, ProblemKind, Profile, SourceDocument, Vocabulary, analyze,
    decode_document, project,
};

fn profile() -> Profile {
    Profile::new(
        "original",
        Vocabulary::under("https://example.org/mime#").unwrap(),
        Limits::unbounded(),
    )
    .unwrap()
}

fn round_trip(bytes: &[u8]) -> Document<'_> {
    let profile = profile();
    let model = analyze(
        SourceDocument {
            id: "urn:example:message",
            bytes,
        },
        &profile,
    )
    .unwrap();
    assert_eq!(model.cover.reconstruct().unwrap(), bytes);
    let dataset = project(&model, &profile).unwrap();
    assert_eq!(
        decode_document(&dataset, model.id(), &profile).unwrap(),
        bytes
    );
    model
}

fn identity_transfer_defects_are_observed_in_leaves_and_containers() {
    for (encoding, body, kind) in [
        (
            b"7bit".as_slice(),
            b"one\ntwo".as_slice(),
            ProblemKind::BareLf,
        ),
        (b"8bit", b"one\rtwo", ProblemKind::BareCr),
        (b"7bit", b"\0", ProblemKind::InvalidTransferEncoding),
        (b"8bit", b"\0", ProblemKind::InvalidTransferEncoding),
        (b"7bit", b"\xff", ProblemKind::InvalidTransferEncoding),
    ] {
        let mut bytes = b"Content-Transfer-Encoding: ".to_vec();
        bytes.extend_from_slice(encoding);
        bytes.extend_from_slice(b"\r\n\r\n");
        bytes.extend_from_slice(body);
        let model = round_trip(&bytes);
        assert!(model.problems.iter().any(|problem| problem.kind == kind));
        assert!(
            model
                .problems
                .iter()
                .any(|problem| problem.kind == ProblemKind::InvalidTransferEncoding)
        );
        assert!(matches!(
            model.decoded_attachment(0),
            Err(MimeError::Transfer { .. })
        ));
    }
    for bytes in [
        b"Content-Type: multipart/mixed; boundary=x\r\n\r\n--x\r\nContent-Transfer-Encoding: binary\r\n\r\n\xff\r\n--x--\r\n".as_slice(),
        b"Content-Type: message/rfc822\r\n\r\nContent-Transfer-Encoding: 8bit\r\n\r\n\xff\r\n".as_slice(),
    ] {
        let model = round_trip(bytes);
        assert_eq!(model.parts.len(), 2);
        assert!(model.problems.iter().any(|problem| problem.part == 0 && problem.kind == ProblemKind::InvalidTransferEncoding));
        assert!(matches!(model.decoded_attachment(0), Err(MimeError::Transfer { .. })));
        assert_eq!(model.decoded_attachment(1).unwrap().bytes[0], 255);
    }
    let binary = round_trip(b"Content-Transfer-Encoding: binary\r\n\r\n\0\xff\n\r");
    assert_eq!(binary.problems, [] as [purrdf_mime::Problem; 0]);
    assert_eq!(binary.decoded_attachment(0).unwrap().bytes, b"\0\xff\n\r");
    let eight = round_trip(b"Content-Transfer-Encoding: 8bit\r\n\r\n\xff\r\n");
    assert_eq!(eight.problems, [] as [purrdf_mime::Problem; 0]);
    assert_eq!(eight.decoded_attachment(0).unwrap().bytes, b"\xff\r\n");
    for length in [998, 999] {
        let mut bytes = b"\r\n".to_vec();
        bytes.extend(std::iter::repeat_n(b'a', length));
        let model = round_trip(&bytes);
        assert_eq!(model.problems.is_empty(), length == 998);
        assert_eq!(model.decoded_attachment(0).is_ok(), length == 998);
    }
}

fn multipart_syntax_defects_preserve_every_original_byte() {
    for boundary in [
        Vec::new(),
        b"x@".to_vec(),
        b"x ".to_vec(),
        vec![255],
        vec![b'x'; 71],
    ] {
        let mut bytes = b"Content-Type: multipart/mixed; boundary=\"".to_vec();
        bytes.extend_from_slice(&boundary);
        bytes.extend_from_slice(b"\"\r\n\r\noriginal");
        let model = round_trip(&bytes);
        assert_eq!(model.parts.len(), 1);
        assert!(
            model
                .problems
                .iter()
                .any(|problem| problem.kind == ProblemKind::InvalidContentType)
        );
    }
    for boundary in [vec![b'x'; 70], b"=?utf-8?Q?a?=".to_vec()] {
        let mut bytes = b"Content-Type: multipart/mixed; boundary=\"".to_vec();
        bytes.extend_from_slice(&boundary);
        bytes.extend_from_slice(b"\"\r\n\r\n--");
        bytes.extend_from_slice(&boundary);
        bytes.extend_from_slice(b"\r\n\r\npayload\r\n--");
        bytes.extend_from_slice(&boundary);
        bytes.extend_from_slice(b"--\r\n");
        let model = round_trip(&bytes);
        assert_eq!(model.parts.len(), 2);
        assert_eq!(model.problems, [] as [purrdf_mime::Problem; 0]);
        assert_eq!(model.decoded_attachment(1).unwrap().bytes, b"payload");
    }
    for (bytes, kind) in [
        (b"Content-Type: multipart/mixed; boundary=x\r\n\r\n--x--\r\n".as_slice(), ProblemKind::MissingOpeningBoundary),
        (b"Content-Type: multipart/mixed; boundary=x\r\n\r\n--x\r\n\r\nbody\r\n--x-oops\r\n--x--\r\n".as_slice(), ProblemKind::UnexpectedBoundary),
    ] {
        let model = round_trip(bytes);
        assert!(model.problems.iter().any(|problem| problem.kind == kind));
    }
    let model = round_trip(
        b"Content-Type: multipart/mixed; boundary=x\r\n\r\n--other\r\n--x\r\n\r\nbody\r\n--x--\r\n",
    );
    assert_eq!(model.problems, [] as [purrdf_mime::Problem; 0]);
    assert_eq!(model.parts.len(), 2);
    assert_eq!(model.decoded_attachment(1).unwrap().bytes, b"body");
}

fn encoded_words_are_checked_only_in_their_actual_header_positions() {
    for bytes in [
        b"Subject: =?ut:f-8?Q?a?=\r\n\r\n".as_slice(),
        b"Subject: =?ut\\f-8?Q?a?=\r\n\r\n".as_slice(),
        b"From: =?utf-8?Q?a?=<sender@example.org>\r\n\r\n".as_slice(),
        b"From: group:=?utf-8?Q?a?= <sender@example.org>;\r\n\r\n".as_slice(),
        b"Subject: =?utf-8?Q?a?=next\r\n\r\n".as_slice(),
        b"From: =?utf-8?Q?a,b?= <sender@example.org>\r\n\r\n".as_slice(),
        b"From: sender@example.org (=?utf-8?Q?a\\b?=)\r\n\r\n".as_slice(),
    ] {
        let model = round_trip(bytes);
        assert!(
            model
                .problems
                .iter()
                .any(|problem| problem.kind == ProblemKind::InvalidEncodedWord)
        );
    }
    for bytes in [
        b"Subject: math x=? is ordinary\r\n\r\n".as_slice(),
        b"Subject: =?utf-8?Q?a=2Cb?=\r\n\r\n".as_slice(),
        b"From: =?utf-8?Q?a=2Cb?= <sender@example.org>\r\n\r\n".as_slice(),
        b"From: group: =?utf-8?Q?a?= <sender@example.org>;\r\n\r\n".as_slice(),
        b"From: sender@example.org (=?utf-8?Q?a=5Cb?=)\r\n\r\n".as_slice(),
        b"From: \"=?ut:f-8?Q?a?=\" <sender@example.org>\r\n\r\n".as_slice(),
        b"Received: from example.org (literal =?ut:f-8?Q?a?=)\r\n\r\n".as_slice(),
        b"Content-Type: text/plain; name=\"=?ut:f-8?Q?a?=\"\r\n\r\n".as_slice(),
    ] {
        let model = round_trip(bytes);
        assert!(
            !model
                .problems
                .iter()
                .any(|problem| problem.kind == ProblemKind::InvalidEncodedWord)
        );
    }
    // RFC2047's75-character word fits a folded76-character physical line.
    for length in [63, 64] {
        let mut bytes = b"Subject: folded\r\n =?utf-8?Q?".to_vec();
        bytes.extend(std::iter::repeat_n(b'a', length));
        bytes.extend_from_slice(b"?=\r\n\r\n");
        let model = round_trip(&bytes);
        assert_eq!(
            model
                .problems
                .iter()
                .any(|problem| problem.kind == ProblemKind::InvalidEncodedWord),
            length == 64
        );
    }
}

fn original_octets_and_repeated_folded_headers_survive() {
    let bytes = b"Received: same\r\nReceived: same\r\nSubject: fold\r\n\t retained\r\nContent-Transfer-Encoding: binary\r\n\r\n\0\xff\x80\r\n";
    let profile = profile();
    let model = analyze(
        SourceDocument {
            id: "urn:example:message",
            bytes,
        },
        &profile,
    )
    .unwrap();
    assert_eq!(model.headers.len(), 4);
    assert_eq!(model.headers[0].ordinal, 0);
    assert_eq!(model.headers[1].ordinal, 1);
    assert_eq!(model.headers[2].unfolded(bytes), b" fold\t retained");
    assert_eq!(model.headers[2].segments.len(), 2);
    assert_eq!(
        model.decoded_attachment(0).unwrap().bytes,
        b"\0\xff\x80\r\n"
    );
    round_trip(bytes);
}

fn malformed_source_is_typed_without_a_repair() {
    for bytes in [
        b" bad fold\nInvalid\nN\xffme: octet\n\nbody".as_slice(),
        b"Subject: unterminated".as_slice(),
        b"".as_slice(),
    ] {
        let profile = profile();
        let model = analyze(
            SourceDocument {
                id: "urn:example:message",
                bytes,
            },
            &profile,
        )
        .unwrap();
        assert_ne!(model.problems, [] as [purrdf_mime::Problem; 0]);
        round_trip(bytes);
    }
}

fn transfer_decoding_is_exact_and_content_identity_ignores_filenames() {
    let first = b"Content-Disposition: attachment; filename=first\r\nContent-Transfer-Encoding: base64\r\n\r\nY W\r\nJj";
    let second = b"Content-Disposition: attachment; filename=second\r\nContent-Transfer-Encoding: quoted-printable\r\n\r\na=62=\r\nc";
    let profile = profile();
    let left = analyze(
        SourceDocument {
            id: "urn:example:first",
            bytes: first,
        },
        &profile,
    )
    .unwrap();
    let right = analyze(
        SourceDocument {
            id: "urn:example:second",
            bytes: second,
        },
        &profile,
    )
    .unwrap();
    let left = left.decoded_attachment(0).unwrap();
    let right = right.decoded_attachment(0).unwrap();
    assert_eq!(left.bytes, b"abc");
    assert_eq!(left, right);
    assert_eq!(
        left.digest.to_hex(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    round_trip(first);
    round_trip(second);
}

fn broken_transfers_refuse_payload_but_preserve_original_cover() {
    for bytes in [
        b"Content-Transfer-Encoding: base64\r\n\r\nY===".as_slice(),
        b"Content-Transfer-Encoding: quoted-printable\r\n\r\n=ZZ".as_slice(),
        b"Content-Transfer-Encoding: x-unsupported\r\n\r\nabc".as_slice(),
        b"Content-Transfer-Encoding: binary\r\nContent-Transfer-Encoding: base64\r\n\r\nYWJj"
            .as_slice(),
    ] {
        let profile = profile();
        let model = analyze(
            SourceDocument {
                id: "urn:example:message",
                bytes,
            },
            &profile,
        )
        .unwrap();
        assert!(matches!(
            model.decoded_attachment(0),
            Err(MimeError::Transfer { .. })
        ));
        assert_ne!(model.problems, [] as [purrdf_mime::Problem; 0]);
        round_trip(bytes);
    }
}

fn nested_multipart_keeps_parent_crlf_and_original_structure() {
    let bytes = b"Content-Type: multipart/mixed; boundary=outer\r\n\r\npre\r\n--outer\r\nContent-Type: multipart/digest; boundary=inner\r\n\r\n--inner\r\n\r\nSubject: attached\r\n\r\nbody\r\n--inner--\r\n--outer--\r\nepilogue";
    let profile = profile();
    let model = analyze(
        SourceDocument {
            id: "urn:example:message",
            bytes,
        },
        &profile,
    )
    .unwrap();
    assert_eq!(model.parts.len(), 4);
    assert_eq!(model.parts[2].media_type.as_ref().unwrap().main, b"message");
    assert_eq!(model.parts[3].parent, Some(2));
    assert_eq!(
        model.part_cover(2).unwrap().reconstruct().unwrap(),
        &bytes[model.parts[2].span.clone()]
    );
    assert!(
        model
            .structures
            .iter()
            .any(|item| item.kind == purrdf_mime::StructureKind::Preamble)
    );
    assert!(
        model
            .structures
            .iter()
            .any(|item| item.kind == purrdf_mime::StructureKind::Epilogue)
    );
    assert!(model.problems.is_empty(), "{:?}", model.problems);
    round_trip(bytes);
}

fn adjacent_delimiters_and_missing_closures_preserve_ranges() {
    for bytes in [
        b"Content-Type: multipart/mixed; boundary=x\r\n\r\n--x\r\n--x\r\n\r\na\r\n--x--\r\n"
            .as_slice(),
        b"Content-Type: multipart/mixed; boundary=x\r\n\r\n--wrong\r\n--x\r\n\r\na".as_slice(),
    ] {
        let profile = profile();
        let model = analyze(
            SourceDocument {
                id: "urn:example:message",
                bytes,
            },
            &profile,
        )
        .unwrap();
        assert!(
            model
                .parts
                .iter()
                .all(|part| part.span.start <= part.span.end)
        );
        round_trip(bytes);
    }
}

fn conflicting_headers_and_encoded_words_are_explicit() {
    let bytes = b"Subject: =?utf-8?B?===?=\r\nContent-Type: text/plain\r\nContent-Type: application/octet-stream\r\n\r\nbody";
    let profile = profile();
    let model = analyze(
        SourceDocument {
            id: "urn:example:message",
            bytes,
        },
        &profile,
    )
    .unwrap();
    assert!(model.parts[0].media_type.is_none());
    assert!(
        model
            .problems
            .iter()
            .any(|problem| problem.kind == ProblemKind::ConflictingStructuralHeaders)
    );
    assert!(
        model
            .problems
            .iter()
            .any(|problem| problem.kind == ProblemKind::InvalidEncodedWord)
    );
    round_trip(bytes);
}

fn caller_limits_are_exact_neighbors_and_no_depth_cap_is_inserted() {
    let mut bytes = Vec::new();
    for _ in 0..257 {
        bytes.extend_from_slice(b"Content-Type: message/rfc822\r\n\r\n");
    }
    bytes.extend_from_slice(b"\r\nbody");
    let vocabulary = Vocabulary::under("https://example.org/mime#").unwrap();
    let unbounded = profile();
    let model = analyze(
        SourceDocument {
            id: "urn:example:message",
            bytes: &bytes,
        },
        &unbounded,
    )
    .unwrap();
    assert_eq!(model.parts.len(), 258);
    assert_eq!(model.parts.last().unwrap().depth, 257);
    let mut limits = Limits::unbounded();
    limits.depth = Some(257);
    let admitted = Profile::new("bounded", vocabulary.clone(), limits).unwrap();
    assert!(
        analyze(
            SourceDocument {
                id: "urn:example:message",
                bytes: &bytes
            },
            &admitted
        )
        .is_ok()
    );
    limits.depth = Some(256);
    let refused = Profile::new("bounded", vocabulary, limits).unwrap();
    assert!(matches!(
        analyze(
            SourceDocument {
                id: "urn:example:message",
                bytes: &bytes
            },
            &refused
        ),
        Err(MimeError::Limit {
            resource: "part depth",
            limit: 256
        })
    ));
}

fn altered_model_and_different_profile_refuse_publication() {
    let profile = profile();
    let mut model = analyze(
        SourceDocument {
            id: "urn:example:message",
            bytes: b"X: y\r\n\r\nz",
        },
        &profile,
    )
    .unwrap();
    model.headers[0].ordinal = 2;
    assert!(matches!(
        project(&model, &profile),
        Err(MimeError::Metadata { .. })
    ));
    let other = Profile::new("other", profile.vocabulary().clone(), Limits::unbounded()).unwrap();
    assert!(matches!(
        project(&model, &other),
        Err(MimeError::ProfileMismatch)
    ));
}

fn graph_mutations_refuse_before_source_bytes_escape() {
    use purrdf_core::{TermValue, ir::RdfDatasetBuilder};
    let profile = profile();
    let model = analyze(
        SourceDocument {
            id: "urn:example:message",
            bytes: b"X: y\r\nX: y\r\n\r\nz",
        },
        &profile,
    )
    .unwrap();
    let original = project(&model, &profile).unwrap();
    for mode in 0..6 {
        let mut builder = RdfDatasetBuilder::new();
        for quad in original.quads() {
            let subject = original.term_value(quad.s);
            let predicate = original.term_value(quad.p);
            let mut object = original.term_value(quad.o);
            let subject_iri = subject.as_iri().unwrap();
            let header = subject_iri.ends_with("/header/1");
            if mode == 0 && header {
                continue;
            }
            if mode == 1
                && header
                && predicate.as_iri() == Some(profile.vocabulary().term("ordinal").unwrap())
            {
                object = TermValue::integer(0);
            }
            if mode == 2
                && subject_iri.ends_with("/span/0")
                && predicate.as_iri() == Some(profile.vocabulary().term("verbatim").unwrap())
            {
                object = TermValue::typed_literal("00", purrdf_core::datatype::XSD_HEX_BINARY);
            }
            if mode == 3
                && subject_iri == model.id()
                && predicate.as_iri() == Some(profile.vocabulary().term("sourceDigest").unwrap())
            {
                object =
                    TermValue::simple_literal(purrdf_core::ContentDigest::of(b"other").to_hex());
            }
            let s = builder.intern_owned_term(&subject.to_rdf_term().unwrap());
            let p = builder.intern_owned_term(&predicate.to_rdf_term().unwrap());
            let o = builder.intern_owned_term(&object.to_rdf_term().unwrap());
            let graph = (mode == 4).then(|| builder.intern_iri("urn:example:foreign-graph"));
            builder.push_quad(s, p, o, graph);
        }
        if mode == 5 {
            let s = builder.intern_iri(model.id());
            let p = builder.intern_iri("https://example.org/invented");
            let o = builder.intern_iri("urn:example:unexpected");
            builder.push_quad(s, p, o, None);
        }
        let altered = builder.freeze().unwrap();
        assert!(
            decode_document(&altered, model.id(), &profile).is_err(),
            "mutation {mode}"
        );
    }
}

fn real_sparql_sees_repeated_occurrences_in_original_order() {
    use purrdf_sparql_algebra::SparqlParser;
    use purrdf_sparql_eval::{EvalCtx, Outcome, evaluate_query};
    let profile = profile();
    let model = analyze(
        SourceDocument {
            id: "urn:example:message",
            bytes: b"Received: same\r\nReceived: same\r\n\r\nz",
        },
        &profile,
    )
    .unwrap();
    let dataset = project(&model, &profile).unwrap();
    let query = SparqlParser::new().parse_query(
        "PREFIX m: <https://example.org/mime#> SELECT ?header ?ordinal WHERE { ?header a m:Header ; m:ordinal ?ordinal . } ORDER BY ?ordinal"
    ).unwrap();
    let Outcome::Solutions(rows) = evaluate_query(&query, &mut EvalCtx::new(&dataset)).unwrap()
    else {
        panic!("SELECT must return solutions");
    };
    assert_eq!(rows.rows.len(), 2);
    assert_ne!(rows.rows[0], rows.rows[1]);
}

#[path = "support/corpus.rs"]
mod corpus;

fn production_rdf_round_trip_is_deterministic_for_original_and_broken_messages() {
    assert_eq!(
        corpus::serialization_transcript(),
        include_str!("golden/serialized-corpus.sha256")
    );
}

fn large_original_source_and_header_multiset_have_only_caller_limits() {
    let profile = profile();
    let mut bytes = b"Content-Transfer-Encoding: binary\r\n\r\n".to_vec();
    bytes.resize((1 << 24) + 129, b'a');
    let model = analyze(
        SourceDocument {
            id: "urn:example:large",
            bytes: &bytes,
        },
        &profile,
    )
    .unwrap();
    assert_eq!(model.cover.reconstruct().unwrap(), bytes);
    let mut limits = Limits::unbounded();
    limits.source_bytes = Some(bytes.len() as u64 - 1);
    let bounded = Profile::new("size", profile.vocabulary().clone(), limits).unwrap();
    assert!(matches!(
        analyze(
            SourceDocument {
                id: "urn:example:large",
                bytes: &bytes
            },
            &bounded
        ),
        Err(MimeError::Limit {
            resource: "source bytes",
            ..
        })
    ));
    limits.source_bytes = None;
    limits.line_bytes = Some((1 << 24) - 1);
    let bounded = Profile::new("line", profile.vocabulary().clone(), limits).unwrap();
    assert!(matches!(
        analyze(
            SourceDocument {
                id: "urn:example:large",
                bytes: &bytes
            },
            &bounded
        ),
        Err(MimeError::Limit {
            resource: "line bytes",
            ..
        })
    ));

    let mut headers = Vec::new();
    for _ in 0..65_537 {
        headers.extend_from_slice(b"Received: identical\r\n");
    }
    headers.extend_from_slice(b"\r\nbody");
    let model = analyze(
        SourceDocument {
            id: "urn:example:headers",
            bytes: &headers,
        },
        &profile,
    )
    .unwrap();
    assert_eq!(model.headers.len(), 65_537);
    assert_eq!(model.headers.last().unwrap().ordinal, 65_536);
    let mut limits = Limits::unbounded();
    limits.headers = Some(65_536);
    let bounded = Profile::new("headers", profile.vocabulary().clone(), limits).unwrap();
    assert!(matches!(
        analyze(
            SourceDocument {
                id: "urn:example:headers",
                bytes: &headers
            },
            &bounded
        ),
        Err(MimeError::Limit {
            resource: "headers",
            limit: 65_536
        })
    ));
}

purrdf_testkit::harness_main!(
    identity_transfer_defects_are_observed_in_leaves_and_containers,
    multipart_syntax_defects_preserve_every_original_byte,
    encoded_words_are_checked_only_in_their_actual_header_positions,
    original_octets_and_repeated_folded_headers_survive,
    malformed_source_is_typed_without_a_repair,
    transfer_decoding_is_exact_and_content_identity_ignores_filenames,
    broken_transfers_refuse_payload_but_preserve_original_cover,
    nested_multipart_keeps_parent_crlf_and_original_structure,
    adjacent_delimiters_and_missing_closures_preserve_ranges,
    conflicting_headers_and_encoded_words_are_explicit,
    caller_limits_are_exact_neighbors_and_no_depth_cap_is_inserted,
    altered_model_and_different_profile_refuse_publication,
    graph_mutations_refuse_before_source_bytes_escape,
    real_sparql_sees_repeated_occurrences_in_original_order,
    production_rdf_round_trip_is_deterministic_for_original_and_broken_messages,
    large_original_source_and_header_multiset_have_only_caller_limits,
);
