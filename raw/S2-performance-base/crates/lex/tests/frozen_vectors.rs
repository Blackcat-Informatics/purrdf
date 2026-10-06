// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The complete frozen lexical vectors, replayed natively against `purrdf-lex`.
//! Release-crate WASM compilation is checked separately by `make wasm`.
//!
//! Each file in `tests/vectors/` was recorded from the implementation that
//! did the job before it moved here: UCHAR and ECHAR decoding from the
//! SPARQL/Turtle lexer, JSON string decoding from the GeoJSON reader, JSON
//! Pointer tokens from the JSON Schema pointer module, each percent-encoding
//! set from the encoder that kept it, and needle search from the IRI
//! component splitter. A disagreement is a defect here, never a reason to
//! edit a vector; the headers say what each covers.

use purrdf_lex::scan::needle_table;
use std::borrow::Cow;

use purrdf_lex::json_escape;
use purrdf_lex::json_pointer;
use purrdf_lex::percent::{self, EncodeSet};
use purrdf_lex::scan::{find_byte, find_byte2};
use purrdf_lex::terminals::{ByteClass, byte_run_count, decode_uchar, echar_value};
use purrdf_testkit::vectors::{VectorFile, answer_digest, decode_bytes, decode_str, encode_str};

/// A hex field of a vector record.
fn hex_field(field: &str) -> u32 {
    purrdf_hash::hex::parse_u32(field.as_bytes()).expect("a hex field")
}

fn show(decoded: Option<(char, usize)>) -> String {
    decoded.map_or_else(
        || "-".to_owned(),
        |(c, n)| format!("{:04X}/{n}", u32::from(c)),
    )
}

/// The escape at the start of `input` read as a UCHAR alone.
fn uchar(input: &str) -> String {
    show(decode_uchar(input.as_bytes()).ok())
}

/// The escape at the start of `input` read as a string-literal escape.
fn string_escape(input: &str) -> String {
    let bytes = input.as_bytes();
    let echar = (bytes.first() == Some(&b'\\'))
        .then(|| bytes.get(1).copied().and_then(echar_value))
        .flatten()
        .map(|c| (c, 2));
    show(echar.or_else(|| decode_uchar(bytes).ok()))
}

fn escapes_replay_the_frozen_vectors() {
    let file =
        VectorFile::parse(include_str!("vectors/escape_vectors.txt")).expect("escape vectors");
    let replayed = file
        .replay(1, |fields| {
            let input = decode_str(fields[0]).expect("an encoded input");
            vec![uchar(&input), string_escape(&input)]
        })
        .unwrap_or_else(|mismatch| panic!("{mismatch}"));
    assert_eq!(replayed, file.records().len());
}

fn every_uchar_value_replays_the_frozen_digests() {
    let file = VectorFile::parse(include_str!("vectors/uchar_scalar_vectors.txt"))
        .expect("scalar vectors");
    for record in file.records() {
        let (first, last) = (hex_field(record.fields[2]), hex_field(record.fields[3]));
        let answers = (first..=last).map(|cp| {
            let text = match (record.fields[0], record.fields[1]) {
                ("u", "upper") => format!("\\u{cp:04X}"),
                ("u", _) => format!("\\u{cp:04x}"),
                (_, "upper") => format!("\\U{cp:08X}"),
                _ => format!("\\U{cp:08x}"),
            };
            uchar(&text)
        });
        assert_eq!(
            answer_digest(answers),
            record.fields[4],
            "{:?}",
            record.fields
        );
    }
}

fn json_answer(body: &str) -> String {
    json_escape::unescape(body)
        .map_or_else(|_| "-".to_owned(), |text| encode_str(&format!("={text}")))
}

fn json_strings_replay_the_frozen_vectors() {
    let file =
        VectorFile::parse(include_str!("vectors/json_string_vectors.txt")).expect("json vectors");
    let replayed = file
        .replay(1, |fields| {
            vec![json_answer(&decode_str(fields[0]).expect("a body"))]
        })
        .unwrap_or_else(|mismatch| panic!("{mismatch}"));
    assert_eq!(replayed, file.records().len());
}

fn every_json_unit_and_pair_replays_the_frozen_digests() {
    let file =
        VectorFile::parse(include_str!("vectors/json_unit_vectors.txt")).expect("unit vectors");
    for record in file.records() {
        let upper = record.fields[1] == "upper";
        let unit = |u: u32| {
            if upper {
                format!("\\u{u:04X}")
            } else {
                format!("\\u{u:04x}")
            }
        };
        let (first, last) = (hex_field(record.fields[2]), hex_field(record.fields[3]));
        let answers = (first..=last).map(|value| {
            if record.fields[0] == "unit" {
                json_answer(&unit(value))
            } else {
                let offset = value - 0x1_0000;
                json_answer(&format!(
                    "{}{}",
                    unit(0xD800 + (offset >> 10)),
                    unit(0xDC00 + (offset & 0x3FF))
                ))
            }
        });
        assert_eq!(
            answer_digest(answers),
            record.fields[4],
            "{:?}",
            record.fields
        );
    }
}

fn json_pointers_replay_the_frozen_vectors() {
    let file = VectorFile::parse(include_str!("vectors/json_pointer_vectors.txt"))
        .expect("pointer vectors");
    let replayed = file
        .replay(2, |fields| match fields[0] {
            "escape" => {
                let token = decode_str(fields[1]).expect("a token");
                vec![encode_str(&json_pointer::escape_token(&token))]
            }
            "parse" => {
                let pointer = decode_str(fields[1]).expect("a pointer");
                json_pointer::tokens(&pointer).map_or_else(
                    || vec!["-".to_owned()],
                    |tokens| {
                        let mut answer = vec![tokens.len().to_string()];
                        answer.extend(tokens.iter().map(|token| encode_str(token)));
                        answer
                    },
                )
            }
            _ => {
                let plane = hex_field(fields[1]);
                let answers = ((plane << 16)..=((plane << 16) | 0xFFFF))
                    .filter_map(char::from_u32)
                    .map(|c| encode_str(&json_pointer::escape_token(&format!("{c}~{c}/"))));
                vec![answer_digest(answers)]
            }
        })
        .unwrap_or_else(|mismatch| panic!("{mismatch}"));
    assert_eq!(replayed, file.records().len());
}

/// Replay one percent vector file: `run` answers an input (`None` for a
/// refusal); `encoded` says whether answers are encoded text rather than
/// `=`-prefixed decodings; a `plane` record covers every scalar of the plane
/// that `recorded` admits.
fn replay_percent(
    vectors: &str,
    encoded: bool,
    run: impl Fn(&str) -> Option<String>,
    recorded: impl Fn(&str) -> bool,
) {
    let answer = |input: &str| match run(input) {
        Some(value) if encoded => encode_str(&value),
        Some(value) => encode_str(&format!("={value}")),
        None => "-".to_owned(),
    };
    let file = VectorFile::parse(vectors).expect("a percent vector file");
    let replayed = file
        .replay(2, |fields| {
            if fields[0] == "plane" {
                let plane = hex_field(fields[1]);
                let answers = ((plane << 16)..=((plane << 16) | 0xFFFF))
                    .filter_map(char::from_u32)
                    .map(|c| c.to_string())
                    .filter(|text| recorded(text))
                    .map(|text| answer(&text));
                vec![answer_digest(answers)]
            } else {
                vec![answer(&decode_str(fields[1]).expect("an input"))]
            }
        })
        .unwrap_or_else(|mismatch| panic!("{mismatch}"));
    assert_eq!(replayed, file.records().len());
}

fn percent_sets_replay_the_frozen_vectors() {
    let sets: [(&str, &str, EncodeSet); 7] = [
        (
            "unreserved",
            include_str!("vectors/percent_unreserved_vectors.txt"),
            percent::UNRESERVED,
        ),
        (
            "unreserved-slash",
            include_str!("vectors/percent_unreserved_slash_vectors.txt"),
            percent::UNRESERVED_SLASH,
        ),
        (
            "reg-name",
            include_str!("vectors/percent_sub_delims_vectors.txt"),
            percent::REG_NAME,
        ),
        (
            "path",
            include_str!("vectors/percent_path_vectors.txt"),
            percent::PATH,
        ),
        (
            "fragment",
            include_str!("vectors/percent_fragment_vectors.txt"),
            percent::FRAGMENT,
        ),
        (
            "uri-template-reserved",
            include_str!("vectors/percent_uri_template_reserved_vectors.txt"),
            percent::URI_TEMPLATE_RESERVED,
        ),
        (
            "non-ascii",
            include_str!("vectors/percent_non_ascii_vectors.txt"),
            percent::NON_ASCII,
        ),
    ];
    for (name, vectors, set) in sets {
        eprintln!("percent set {name}");
        replay_percent(
            vectors,
            true,
            |input| Some(percent::encode(input, set).into_owned()),
            |_| true,
        );
    }
}

fn percent_decoders_replay_the_frozen_vectors() {
    replay_percent(
        include_str!("vectors/percent_decode_vectors.txt"),
        false,
        |input| percent::decode(input).ok().map(Cow::into_owned),
        |_| true,
    );
    replay_percent(
        include_str!("vectors/percent_decode_form_vectors.txt"),
        false,
        |input| percent::decode_form(input).ok().map(Cow::into_owned),
        |_| true,
    );
    replay_percent(
        include_str!("vectors/percent_normalize_vectors.txt"),
        false,
        |input| Some(percent::normalize(input).into_owned()),
        |_| true,
    );
}

/// The four needle sets of the needle vectors, as the classes a caller with a
/// fixed set declares.
const HASH: [u8; 256] = needle_table(b"#");
const QUERY_HASH: [u8; 256] = needle_table(b"?#");
const PATH_END: [u8; 256] = needle_table(b"/?#");
const SCHEME_END: [u8; 256] = needle_table(b":/?#");

fn class_find(needles: &[u8], hay: &[u8]) -> Option<usize> {
    match needles {
        b"#" => ByteClass::<{ byte_run_count(&HASH) }>::from_table(HASH).find_first(hay),
        b"?#" => {
            ByteClass::<{ byte_run_count(&QUERY_HASH) }>::from_table(QUERY_HASH).find_first(hay)
        }
        b"/?#" => ByteClass::<{ byte_run_count(&PATH_END) }>::from_table(PATH_END).find_first(hay),
        b":/?#" => {
            ByteClass::<{ byte_run_count(&SCHEME_END) }>::from_table(SCHEME_END).find_first(hay)
        }
        other => panic!("unexpected needle set {other:?}"),
    }
}

fn needle_searches_replay_the_frozen_vectors() {
    let file =
        VectorFile::parse(include_str!("vectors/find_byte_vectors.txt")).expect("needle vectors");
    let show = |at: Option<usize>| at.map_or_else(|| "-".to_owned(), |at| at.to_string());
    let replayed = file
        .replay(2, |fields| {
            let needles = decode_bytes(fields[0]).expect("needles");
            let hay = decode_bytes(fields[1]).expect("a haystack");
            let by_class = show(class_find(&needles, &hay));
            let by_needle = match needles[..] {
                [a] => show(find_byte(&hay, a)),
                [a, b] => show(find_byte2(&hay, a, b)),
                _ => by_class.clone(),
            };
            assert_eq!(by_needle, by_class, "{needles:?} in {hay:?}");
            vec![by_class]
        })
        .unwrap_or_else(|mismatch| panic!("{mismatch}"));
    assert_eq!(replayed, file.records().len());
}

purrdf_testkit::harness_main!(
    escapes_replay_the_frozen_vectors,
    every_uchar_value_replays_the_frozen_digests,
    json_strings_replay_the_frozen_vectors,
    every_json_unit_and_pair_replays_the_frozen_digests,
    json_pointers_replay_the_frozen_vectors,
    percent_sets_replay_the_frozen_vectors,
    percent_decoders_replay_the_frozen_vectors,
    needle_searches_replay_the_frozen_vectors,
);
