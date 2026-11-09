// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Actual encoded ingestion, original cause transport and admitted conversion bounds.

use core::convert::Infallible;
use purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};
use purrdf_lex::control::Unmetered;
use purrdf_lex::gb18030::{Direction, Extent, Limits, Profile, StreamError, Transcoder};
use purrdf_rdf::{
    EncodedReadError, EncodedReadOptions, SerializeGraph, parse_dataset,
    parse_dataset_from_encoded_reader, parse_dataset_from_reader, serialize_dataset,
};
use std::io::Read;

const PROFILE: Profile = Profile::Gb18030_2022Amendment1;
fn options(profile: Profile) -> EncodedReadOptions {
    EncodedReadOptions {
        profile,
        limits: Limits {
            input_bytes: u64::MAX,
            output_bytes: u64::MAX,
        },
        line_bytes: 1 << 20,
        workspace_bytes: 8 << 20,
    }
}
fn encode(text: &str, profile: Profile) -> (Vec<u8>, Vec<(char, Extent)>) {
    let mut bytes = Vec::new();
    let mut map = Vec::new();
    Transcoder::new(profile, Direction::Encode, options(profile).limits)
        .push(text.as_bytes(), true, &mut Unmetered, |item| {
            map.push((item.scalar, item.output));
            bytes.extend_from_slice(item.bytes);
            Ok::<_, Infallible>(())
        })
        .expect("representable source");
    (bytes, map)
}
struct Chunked<'a> {
    bytes: &'a [u8],
    chunk: usize,
}
impl Read for Chunked<'_> {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let width = out.len().min(self.chunk);
        self.bytes.read(&mut out[..width])
    }
}

#[test]
fn explicit_profiles_preserve_native_datasets_source_contributions_and_line_rules() {
    for profile in [Profile::Gb18030_2005, PROFILE] {
        for (media, text) in [
            (
                "application/n-quads",
                "<https://example.org/中> <https://example.org/p> \"龴𠀇\"@zh .\r<https://example.org/中> <https://example.org/p> \"ordinary\" .\r\n# comment\n",
            ),
            (
                "text/turtle",
                "@prefix ex: <https://example.org/> .\r\nex:中 ex:p \"龴𠀇\"@zh .",
            ),
            (
                "application/ld+json",
                "{\"@id\":\"https://example.org/中\",\"https://example.org/p\":{\"@value\":\"龴𠀇\"}}",
            ),
            (
                "application/x-hextuples",
                "[\"https://example.org/中\",\"https://example.org/p\",\"龴𠀇\",\"http://www.w3.org/2001/XMLSchema#string\",\"\",\"\"]\r\n",
            ),
        ] {
            let native = parse_dataset(text.as_bytes(), media, None).expect("native grammar");
            let canonical =
                serialize_dataset(&native, "application/n-quads", SerializeGraph::Dataset)
                    .expect("native export");
            let (bytes, expected) = encode(text, profile);
            for chunk in [1, 2, 3, 4, 7, 8192] {
                let mut resident = Resident;
                let mut memory = Memory::new(&mut resident);
                let mut observed = Vec::new();
                let parsed = parse_dataset_from_encoded_reader(
                    Chunked {
                        bytes: &bytes,
                        chunk,
                    },
                    media,
                    None,
                    options(profile),
                    &mut memory,
                    &mut Unmetered,
                    |item| {
                        observed.push((item.scalar, item.source));
                        Ok::<_, Infallible>(())
                    },
                )
                .expect("explicit encoded input");
                assert_eq!(parsed.profile, profile);
                assert_eq!(parsed.totals.input_bytes, bytes.len() as u64);
                assert_eq!(parsed.totals.output_bytes, text.len() as u64);
                assert_eq!(observed, expected);
                assert_eq!(memory.admitted_bytes(), 0);
                assert_eq!(
                    serialize_dataset(
                        &parsed.dataset,
                        "application/n-quads",
                        SerializeGraph::Dataset
                    )
                    .expect("encoded export"),
                    canonical
                );
                assert_eq!(
                    parsed.dataset.quads().collect::<Vec<_>>(),
                    native.quads().collect::<Vec<_>>()
                );
            }
            if media == "application/n-quads" {
                assert!(
                    parse_dataset_from_reader(bytes.as_slice(), media, None).is_err(),
                    "UTF8 default must not sniff GB"
                );
            }
        }
    }
    let media = "application/x-hextuples";
    let row = "[\"https://example.org/中\",\"https://example.org/p\",\"https://example.org/o\",\"globalId\",\"\",\"\"]";
    for (separator, valid) in [("\r", false), ("\r\n", true), ("\n", true)] {
        let (bytes, _) = encode(&format!("{row}{separator}{row}"), PROFILE);
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let result = parse_dataset_from_encoded_reader(
            Chunked {
                bytes: &bytes,
                chunk: 1,
            },
            media,
            None,
            options(PROFILE),
            &mut memory,
            &mut Unmetered,
            |_| Ok::<_, Infallible>(()),
        );
        assert_eq!(result.is_ok(), valid, "NDJSON separator {separator:?}");
        assert_eq!(memory.admitted_bytes(), 0);
    }
}

#[test]
fn native_diagnostics_keep_scalar_columns_and_original_byte_extents() {
    for profile in [Profile::Gb18030_2005, PROFILE] {
        for media in ["application/n-triples", "text/turtle"] {
            for prefix in [
                "# 中\r\n",
                "<https://example.org/中> <https://example.org/p> <https://example.org/o> .\r",
            ] {
                let text = format!("{prefix}<https://example.org/𠀇> <https://example.org/p> ！ .");
                let native = parse_dataset_from_reader(text.as_bytes(), media, None)
                    .expect_err("malformed object");
                let location = native.location.as_ref().expect("located native error");
                let column = location.column.expect("scalar column");
                let (bytes, map) = encode(&text, profile);
                let scalar_index = if media == "application/n-triples" {
                    prefix.chars().count() + column as usize - 1
                } else {
                    let positions = purrdf_iri::LineIndex::new(&text);
                    text.char_indices()
                        .enumerate()
                        .find_map(|(index, (byte, _))| {
                            let position = positions.locate(&text, byte);
                            (Some(position.line) == location.line && position.column == column)
                                .then_some(index)
                        })
                        .expect("native buffered position")
                };
                let expected = map[scalar_index].1;
                for chunk in [1, 3, 8192] {
                    let mut resident = Resident;
                    let mut memory = Memory::new(&mut resident);
                    let error = parse_dataset_from_encoded_reader(
                        Chunked {
                            bytes: &bytes,
                            chunk,
                        },
                        media,
                        None,
                        options(profile),
                        &mut memory,
                        &mut Unmetered,
                        |_| Ok::<_, Infallible>(()),
                    )
                    .expect_err("native refusal");
                    let EncodedReadError::Parse { diagnostic, source } = error else {
                        panic!("native diagnostic");
                    };
                    assert_eq!(diagnostic.code, native.code);
                    assert_eq!(diagnostic.message, native.message);
                    assert_eq!(diagnostic.location, native.location);
                    assert_eq!(source, Some(expected), "native byte extent with {prefix:?}");
                    assert_eq!(memory.admitted_bytes(), 0);
                }
            }
        }
    }
    // A malformed later line must not replace the first line's native grammar error.
    let text = "<https://example.org/中> <https://example.org/p> ！ .\r";
    let (mut bytes, _) = encode(text, PROFILE);
    bytes.push(0x80);
    let native = parse_dataset_from_reader(text.as_bytes(), "application/n-triples", None)
        .expect_err("first line grammar");
    for chunk in [1, 8192] {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let error = parse_dataset_from_encoded_reader(
            Chunked {
                bytes: &bytes,
                chunk,
            },
            "application/n-triples",
            None,
            options(PROFILE),
            &mut memory,
            &mut Unmetered,
            |_| Ok::<_, Infallible>(()),
        )
        .expect_err("first failure in document order");
        let EncodedReadError::Parse { diagnostic, .. } = error else {
            panic!("later byte must stay undecoded");
        };
        assert_eq!(diagnostic.message, native.message);
        assert_eq!(diagnostic.location, native.location);
        assert_eq!(memory.admitted_bytes(), 0);
    }
}

#[derive(Debug)]
struct HostFailure(std::sync::Arc<()>);
impl core::fmt::Display for HostFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("original host failure")
    }
}
impl std::error::Error for HostFailure {}
struct FailedRead {
    token: Option<std::sync::Arc<()>>,
}
impl Read for FailedRead {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other(HostFailure(
            self.token.take().expect("one failed read"),
        )))
    }
}

struct InterruptedRead<R> {
    reader: R,
    remaining: usize,
    attempts: std::rc::Rc<std::cell::Cell<usize>>,
}
impl<R: Read> Read for InterruptedRead<R> {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        self.attempts.set(self.attempts.get() + 1);
        if self.remaining > 0 {
            self.remaining -= 1;
            return Err(std::io::ErrorKind::Interrupted.into());
        }
        self.reader.read(bytes)
    }
}

#[test]
fn actual_reader_preserves_original_codec_control_output_io_and_admission_causes() {
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let attempts = std::rc::Rc::new(std::cell::Cell::new(0));
    let parsed = parse_dataset_from_encoded_reader(
        InterruptedRead {
            reader: b"<https://example.org/s> <https://example.org/p> <https://example.org/o> ."
                .as_slice(),
            remaining: 2,
            attempts: attempts.clone(),
        },
        "application/n-triples",
        None,
        options(PROFILE),
        &mut memory,
        &mut Unmetered,
        |_| Ok::<_, Infallible>(()),
    )
    .expect("interrupted reads retry before valid RDF");
    assert_eq!(parsed.totals.input_bytes, 73);
    assert_eq!(attempts.get(), 4);
    assert_eq!(memory.admitted_bytes(), 0);
    let attempts = std::rc::Rc::new(std::cell::Cell::new(0));
    let token = std::sync::Arc::new(());
    let error = parse_dataset_from_encoded_reader(
        InterruptedRead {
            reader: b"#".as_slice(),
            remaining: usize::MAX,
            attempts: attempts.clone(),
        },
        "application/n-triples",
        None,
        options(PROFILE),
        &mut memory,
        &mut |units| {
            assert_eq!(units, 0);
            if attempts.get() == 2 {
                Err(token.clone())
            } else {
                Ok(())
            }
        },
        |_| Ok::<_, Infallible>(()),
    )
    .expect_err("cancellation polled before each read retry");
    let EncodedReadError::Control(original) = error else {
        panic!("unchanged retry cancellation");
    };
    assert!(std::sync::Arc::ptr_eq(&original, &token));
    assert_eq!(attempts.get(), 2);
    assert_eq!(memory.admitted_bytes(), 0);
    let error = parse_dataset_from_encoded_reader(
        b"#a".as_slice(),
        "application/n-triples",
        None,
        options(PROFILE),
        &mut memory,
        &mut Unmetered,
        |item| {
            assert_ne!(item.scalar, 'a', "unwind with conversion buffers live");
            Ok::<_, Infallible>(())
        },
    )
    .expect_err("native boundary catches an unwind before refunding workspace");
    let EncodedReadError::Parse { diagnostic, source } = error else {
        panic!("native unwind diagnostic");
    };
    assert_eq!(diagnostic.code, "native-codec-panic");
    assert!(source.is_none());
    assert_eq!(memory.admitted_bytes(), 0);
    let error = parse_dataset_from_encoded_reader(
        &[b'#', 0x81, 0x30][..],
        "application/n-triples",
        None,
        options(PROFILE),
        &mut memory,
        &mut Unmetered,
        |_| Ok::<_, Infallible>(()),
    )
    .expect_err("incomplete source");
    assert!(
        matches!(error,EncodedReadError::Codec(StreamError::Native(error)) if error.profile==PROFILE && error.source==Extent {start:1,end:3} && error.kind==purrdf_lex::gb18030::ErrorKind::Incomplete)
    );
    assert_eq!(memory.admitted_bytes(), 0);
    let token = std::sync::Arc::new(());
    let error = parse_dataset_from_encoded_reader(
        FailedRead {
            token: Some(token.clone()),
        },
        "application/n-triples",
        None,
        options(PROFILE),
        &mut memory,
        &mut Unmetered,
        |_| Ok::<_, Infallible>(()),
    )
    .expect_err("original read error");
    let EncodedReadError::Read { cause, offset } = error else {
        panic!("original I/O cause");
    };
    assert_eq!(offset, 0);
    assert!(std::sync::Arc::ptr_eq(
        &cause
            .get_ref()
            .expect("cause")
            .downcast_ref::<HostFailure>()
            .expect("typed original")
            .0,
        &token
    ));
    let mut stopped = Some(token.clone());
    let error = parse_dataset_from_encoded_reader(
        b"#".as_slice(),
        "application/n-triples",
        None,
        options(PROFILE),
        &mut memory,
        &mut |_: u64| Err(stopped.take().expect("one stop poll")),
        |_| Ok::<_, Infallible>(()),
    )
    .expect_err("original cancellation");
    let EncodedReadError::Control(original) = error else {
        panic!("unchanged control");
    };
    assert!(std::sync::Arc::ptr_eq(&original, &token));
    let mut output = Some(token.clone());
    let error = parse_dataset_from_encoded_reader(
        b"#".as_slice(),
        "application/n-triples",
        None,
        options(PROFILE),
        &mut memory,
        &mut Unmetered,
        |_| Err(output.take().expect("one output failure")),
    )
    .expect_err("original output");
    let EncodedReadError::Codec(StreamError::Output(original)) = error else {
        panic!("unchanged output");
    };
    assert!(std::sync::Arc::ptr_eq(&original, &token));
    let mut ceiling = purrdf_lex::allocation::Ceiling(0);
    let mut memory = Memory::new(&mut ceiling);
    let error = parse_dataset_from_encoded_reader(
        b"#".as_slice(),
        "application/n-triples",
        None,
        options(PROFILE),
        &mut memory,
        &mut Unmetered,
        |_| Ok::<_, Infallible>(()),
    )
    .expect_err("original admission");
    assert!(matches!(
        error,
        EncodedReadError::Storage(StorageError::AdmissionFailed)
    ));
    assert_eq!(memory.admitted_bytes(), 0);
}

#[derive(Default)]
struct Account {
    live: usize,
    peak: usize,
    resizes: usize,
}
impl Admission for Account {
    fn resize(&mut self, live: usize) -> Result<(), StorageError> {
        self.live = live;
        self.peak = self.peak.max(live);
        self.resizes += 1;
        Ok(())
    }
}

#[test]
fn explicit_conversion_ceilings_have_valid_neighbors_and_growing_input_keeps_one_line_live() {
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut config = options(PROFILE);
    config.workspace_bytes = 0;
    let error = parse_dataset_from_encoded_reader(
        b"".as_slice(),
        "application/n-triples",
        None,
        config,
        &mut memory,
        &mut Unmetered,
        |_| Ok::<_, Infallible>(()),
    )
    .expect_err("fixed workspace");
    let EncodedReadError::Limit { required, .. } = error else {
        panic!("workspace limit");
    };
    config.workspace_bytes = required as usize + 1 + size_of::<Extent>();
    config.line_bytes = 1;
    parse_dataset_from_encoded_reader(
        b"#".as_slice(),
        "application/n-triples",
        None,
        config,
        &mut memory,
        &mut Unmetered,
        |_| Ok::<_, Infallible>(()),
    )
    .expect("exact one-scalar neighbor");
    config = options(PROFILE);
    config.line_bytes = 1;
    let error = parse_dataset_from_encoded_reader(
        b"#x".as_slice(),
        "application/n-triples",
        None,
        config,
        &mut memory,
        &mut Unmetered,
        |_| Ok::<_, Infallible>(()),
    )
    .expect_err("line ceiling");
    assert!(matches!(
        error,
        EncodedReadError::Limit {
            resource: "transcode-line-bytes",
            required: 2,
            limit: 1
        }
    ));
    config.line_bytes = 2;
    parse_dataset_from_encoded_reader(
        b"#x".as_slice(),
        "application/n-triples",
        None,
        config,
        &mut memory,
        &mut Unmetered,
        |_| Ok::<_, Infallible>(()),
    )
    .expect("exact line neighbor");
    let line = "<https://example.org/中> <https://example.org/p> \"𠀇龴\" .\r\n";
    let mut peaks = Vec::new();
    for rows in [64, 257, 1025] {
        let (bytes, _) = encode(&line.repeat(rows), PROFILE);
        let mut account = Account::default();
        let mut memory = Memory::new(&mut account);
        let parsed = parse_dataset_from_encoded_reader(
            bytes.as_slice(),
            "application/n-triples",
            None,
            options(PROFILE),
            &mut memory,
            &mut Unmetered,
            |_| Ok::<_, Infallible>(()),
        )
        .expect("growing stream");
        assert_eq!(parsed.totals.input_bytes, bytes.len() as u64);
        assert_eq!(memory.admitted_bytes(), 0);
        assert_eq!(account.live, 0);
        eprintln!(
            "rows={rows} input={} utf8={} scalars={} conversion_peak={} resizes={}",
            bytes.len(),
            parsed.totals.output_bytes,
            parsed.totals.scalars,
            account.peak,
            account.resizes
        );
        peaks.push(account.peak);
    }
    assert!(
        peaks.windows(2).all(|pair| pair[0] == pair[1]),
        "conversion workspace grows with a line, not input size"
    );
}
