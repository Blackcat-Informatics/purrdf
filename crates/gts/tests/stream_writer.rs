// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The streaming writer's contract: the bytes [`StreamWriter`] hands a sink
//! are the bytes [`Writer`] accumulates for the same frames, a chain resumes
//! from its stored head alone, a clear payload read from a reader is held one
//! chunk at a time, and a failed sink leaves no half-advanced chain.

use std::io::Read;

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_ed25519::SigningKey;
use purrdf_gts::codec::Codec;
use purrdf_gts::cose;
use purrdf_gts::reader::read;
use purrdf_gts::writer::{
    Encrypt0Options, FnSink, FrameError, FrameOptions, FrameSigning, FrameSink, PayloadError,
    StreamError, StreamWriter, Writer, WriterError, WriterOptions,
};
use purrdf_lex::cbor::Value;

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

const PROFILE: &str = "purrdf.gts";
const KID: &str = "test-key";

/// A fixed test signing key; never a production secret.
fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[41; 32])
}

/// Deterministic bytes with enough structure to compress.
fn bytes(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|i| (i % 251) as u8 ^ seed ^ ((i / 4096) as u8))
        .collect()
}

fn seal(iv: u8) -> Encrypt0Options {
    Encrypt0Options {
        kid: "vault".to_string(),
        key: [9; 32],
        iv: [iv; 12],
    }
}

fn raw(data: Vec<u8>) -> FrameOptions {
    FrameOptions {
        raw: Some(data),
        ..FrameOptions::default()
    }
}

fn transformed(data: Vec<u8>, codec: &str) -> FrameOptions {
    FrameOptions {
        transform: vec![codec.to_string()],
        ..raw(data)
    }
}

/// One frame of each kind the first version covers. Index 5 is the frame a
/// round ends on; index 12 carries explicit signature bytes.
fn frames() -> Vec<(&'static str, FrameOptions)> {
    let pub_meta = Value::Map(vec![
        ("mt".into(), "application/octet-stream".into()),
        ("round".into(), Value::from(7)),
    ]);
    let recipients = vec![Value::Map(vec![
        ("kid".into(), "reader-1".into()),
        ("role".into(), "auditor".into()),
    ])];
    vec![
        (
            "meta",
            FrameOptions {
                payload: Some(Value::Map(vec![("note".into(), "structured".into())])),
                ..FrameOptions::default()
            },
        ),
        (
            "blob",
            FrameOptions {
                pub_meta: Some(pub_meta.clone()),
                recipients: recipients.clone(),
                ..raw(bytes(1000, 1))
            },
        ),
        ("blob", transformed(bytes(5000, 2), "identity")),
        ("blob", transformed(bytes(40_000, 3), "gzip")),
        ("blob", transformed(bytes(40_000, 4), "zstd")),
        (
            "blob",
            FrameOptions {
                zstd_level: Some(5),
                ..transformed(bytes(150_000, 5), "zstd-rsyncable")
            },
        ),
        (
            "blob",
            FrameOptions {
                encrypt: Some(seal(1)),
                pub_meta: Some(pub_meta),
                ..raw(bytes(3000, 6))
            },
        ),
        (
            "blob",
            FrameOptions {
                encrypt: Some(seal(2)),
                recipients,
                ..transformed(bytes(30_000, 7), "zstd")
            },
        ),
        // Longer than two payload chunks, so a streamed copy crosses chunks.
        ("blob", raw(bytes(300_000, 8))),
        ("blob", raw(Vec::new())),
        (
            "meta",
            FrameOptions {
                pub_meta: Some(Value::Map(vec![("empty".into(), Value::Bool(true))])),
                ..FrameOptions::default()
            },
        ),
        ("blob", raw(bytes(64, 9))),
        (
            "blob",
            FrameOptions {
                signature: Some(b"caller-supplied signature bytes".to_vec()),
                ..raw(bytes(64, 10))
            },
        ),
    ]
}

/// The header variants the first version covers, each with every codec the
/// frames name in its catalog.
fn headers() -> Vec<(&'static str, WriterOptions)> {
    vec![
        (
            "default header with the self-describe tag",
            WriterOptions::default(),
        ),
        (
            "no tag, meta naming a previous segment",
            WriterOptions {
                magic_tag: false,
                meta: Some(Value::Map(vec![(
                    "previousSegment".into(),
                    Value::Bytes(vec![0xa5; 32]),
                )])),
                ..WriterOptions::default()
            },
        ),
        (
            "caller-supplied catalog",
            WriterOptions {
                catalog: Some(vec![
                    (20, Codec::new("cose-encrypt0", "encrypt")),
                    (11, Codec::new("gzip", "compress")),
                    (10, Codec::new("identity", "encode")),
                    (13, Codec::new("zstd-rsyncable", "compress")),
                    (12, Codec::new("zstd", "compress")),
                ]),
                ..WriterOptions::default()
            },
        ),
        (
            "streamable layout claim",
            WriterOptions {
                layout: Some("streamable".to_string()),
                ..WriterOptions::default()
            },
        ),
    ]
}

/// Which frames the installed signer signs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Signed {
    None,
    Every,
    Only(usize),
}

impl Signed {
    fn choice(self, index: usize) -> FrameSigning {
        match self {
            Self::Only(chosen) if chosen != index => FrameSigning::Unsigned,
            _ => FrameSigning::Installed,
        }
    }
}

/// How a raw payload reaches the stream writer.
#[derive(Clone, Copy, Debug)]
enum Feed {
    Whole,
    ReaderOneByte,
    ReaderAllAtOnce,
}

/// A reader that yields at most `step` bytes per call.
struct Stepped<'a> {
    data: &'a [u8],
    step: usize,
}

impl Read for Stepped<'_> {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let count = self.step.min(out.len()).min(self.data.len());
        out[..count].copy_from_slice(&self.data[..count]);
        self.data = &self.data[count..];
        Ok(count)
    }
}

/// The in-memory writer's bytes, ids and head for `frames`: the oracle.
///
/// With a signer on every frame the oracle is a writer with that signer
/// installed. `Writer` cannot leave a chosen frame unsigned, so for
/// [`Signed::Only`] the oracle supplies explicit signature bytes over the
/// frame's id, which an unsigned twin of the same log yields first: a
/// signature is outside the id preimage.
fn oracle(
    header: &WriterOptions,
    frames: &[(&'static str, FrameOptions)],
    signed: Signed,
) -> (Vec<u8>, Vec<Vec<u8>>) {
    let mut writer = Writer::with_options(PROFILE, header.clone()).expect("header");
    let mut twin = Writer::with_options(PROFILE, header.clone()).expect("header");
    if signed == Signed::Every {
        writer.sign_with(signing_key(), KID);
    }
    let mut ids = Vec::new();
    for (index, (frame_type, options)) in frames.iter().enumerate() {
        let mut options = options.clone();
        let id = twin
            .add_frame_with_options(frame_type, options.clone())
            .expect("twin frame");
        if signed == Signed::Only(index) && options.signature.is_none() {
            options.signature = Some(cose::sign_id(&id, &signing_key(), KID));
        }
        let written = writer
            .add_frame_with_options(frame_type, options)
            .expect("oracle frame");
        assert_eq!(written, id, "a signature never changes a frame id");
        ids.push(written);
    }
    assert_eq!(writer.head(), ids.last().expect("frames").as_slice());
    (writer.into_bytes(), ids)
}

/// Append `frames[range]` to `stream`, feeding raw payloads as `feed` says.
fn append<S: FrameSink>(
    stream: &mut StreamWriter<S>,
    frames: &[(&'static str, FrameOptions)],
    range: std::ops::Range<usize>,
    signed: Signed,
    feed: Feed,
) -> Vec<Vec<u8>>
where
    S::Error: std::fmt::Debug,
{
    let mut ids = Vec::new();
    for index in range {
        let (frame_type, options) = &frames[index];
        let mut options = options.clone();
        // Explicit signature bytes are the frame's signature whatever the
        // installed signer would do, and are refused on an unsigned frame.
        let signing = if options.signature.is_some() {
            FrameSigning::Installed
        } else {
            signed.choice(index)
        };
        let step = match feed {
            Feed::Whole => None,
            Feed::ReaderOneByte => Some(1),
            Feed::ReaderAllAtOnce => Some(usize::MAX),
        };
        let id = match (step, options.raw.take()) {
            (Some(step), Some(data)) => stream.add_frame_from_reader(
                frame_type,
                data.len() as u64,
                Stepped { data: &data, step },
                options,
                signing,
            ),
            (_, data) => {
                options.raw = data;
                if signing == FrameSigning::Installed {
                    stream.add_frame_with_options(frame_type, options)
                } else {
                    stream.add_frame_signing(frame_type, options, signing)
                }
            }
        }
        .expect("stream frame");
        assert_eq!(stream.head(), id.as_slice());
        ids.push(id);
    }
    ids
}

fn stream_all(
    header: &WriterOptions,
    frames: &[(&'static str, FrameOptions)],
    signed: Signed,
    feed: Feed,
) -> (Vec<u8>, Vec<Vec<u8>>) {
    let mut log = Vec::new();
    let mut stream = StreamWriter::new(&mut log, PROFILE, header.clone()).expect("header");
    if signed != Signed::None {
        stream.sign_with(signing_key(), KID);
    }
    let ids = append(&mut stream, frames, 0..frames.len(), signed, feed);
    let written = stream.bytes_written();
    drop(stream);
    assert_eq!(written, log.len() as u64);
    (log, ids)
}

/// Byte identity is the contract: for every in-scope header, signing choice
/// and way of supplying a payload, the stream is the in-memory writer's log.
/// This is the test that fails if the two front ends ever frame differently.
#[test]
fn stream_bytes_equal_writer_bytes_for_each_in_scope_configuration() {
    let frames = frames();
    for (label, header) in headers() {
        for signed in [Signed::None, Signed::Every, Signed::Only(5)] {
            let expected = oracle(&header, &frames, signed);
            for feed in [Feed::Whole, Feed::ReaderOneByte, Feed::ReaderAllAtOnce] {
                let actual = stream_all(&header, &frames, signed, feed);
                assert!(
                    actual == expected,
                    "{label}, {signed:?}, {feed:?}: stream differs from Writer::into_bytes()"
                );
            }
        }
    }

    // The chosen-frame oracle above is built from explicit signature bytes.
    // Pin what it stands for: frame 5 alone carries the installed signer's
    // signature, and the unsigned frames are the unsigned log's frames.
    let header = WriterOptions::default();
    let (chosen, ids) = stream_all(&header, &frames, Signed::Only(5), Feed::Whole);
    let (unsigned, unsigned_ids) = stream_all(&header, &frames, Signed::None, Feed::Whole);
    let (every, _) = stream_all(&header, &frames, Signed::Every, Feed::Whole);
    assert_eq!(ids, unsigned_ids);
    let signature = cose::sign_id(&ids[5], &signing_key(), KID);
    assert_eq!(chosen.len(), unsigned.len() + signature.len() + 6);
    assert!(every.len() > chosen.len());
}

/// A typed sink error is not `std::io::Error`, and a closure sink frames the
/// same bytes as an `io::Write` one.
#[test]
fn a_closure_sink_receives_the_same_bytes() {
    let frames = frames();
    let header = WriterOptions::default();
    let (expected, _) = oracle(&header, &frames, Signed::None);
    let mut log = Vec::new();
    let sink = FnSink(|chunk: &[u8]| -> Result<(), Boom> {
        log.extend_from_slice(chunk);
        Ok(())
    });
    let mut stream = StreamWriter::new(sink, PROFILE, header).expect("header");
    append(
        &mut stream,
        &frames,
        0..frames.len(),
        Signed::None,
        Feed::ReaderAllAtOnce,
    );
    drop(stream);
    assert_eq!(log, expected);
}

/// The restart story: cut the log at any frame boundary, keep only the head
/// and the catalog options, and the resumed stream completes the same log.
#[test]
fn resume_from_stored_head_equals_the_uninterrupted_stream() {
    let frames = frames();
    for (label, header) in headers() {
        let signed = Signed::Only(5);
        let (whole, ids) = oracle(&header, &frames, signed);
        for cut in 0..=frames.len() {
            let mut log = Vec::new();
            let mut first = StreamWriter::new(&mut log, PROFILE, header.clone()).expect("header");
            first.sign_with(signing_key(), KID);
            append(&mut first, &frames, 0..cut, signed, Feed::Whole);
            let head = first.head().to_vec();
            let durable = first.bytes_written();
            drop(first);
            assert_eq!(durable, log.len() as u64);

            let mut resumed =
                StreamWriter::resume(&mut log, header.clone(), &head).expect("resume");
            resumed.sign_with(signing_key(), KID);
            let rest = append(
                &mut resumed,
                &frames,
                cut..frames.len(),
                signed,
                Feed::ReaderAllAtOnce,
            );
            let appended = resumed.bytes_written();
            drop(resumed);
            assert_eq!(rest, ids[cut..], "{label}, cut {cut}");
            assert_eq!(durable + appended, log.len() as u64);
            assert!(log == whole, "{label}, cut {cut}: resumed log differs");
        }
    }

    // Cross-check against the existing append path, which recovers the head
    // and catalog by reading the segment, and against the reader's chain walk.
    let header = WriterOptions::default();
    let blob = |seed: u8, codec: Option<&str>| {
        let data = bytes(2000, seed);
        let digest = purrdf_gts::writer::digest_string(&data);
        let source = match codec {
            None => raw(data),
            Some(codec) => transformed(data, codec),
        };
        let options = FrameOptions {
            pub_meta: Some(Value::Map(vec![("digest".into(), digest.into())])),
            ..source
        };
        ("blob", options)
    };
    let clear = vec![
        blob(1, None),
        blob(2, Some("zstd")),
        blob(3, None),
        blob(4, Some("gzip")),
        blob(5, None),
    ];
    let (whole, ids) = stream_all(&header, &clear, Signed::None, Feed::Whole);
    let (prefix, _) = stream_all(&header, &clear[..3], Signed::None, Feed::Whole);
    let mut appending = Writer::appending(&prefix).expect("append state");
    let mut tail = Vec::new();
    let mut resumed = StreamWriter::resume(&mut tail, header, &ids[2]).expect("resume");
    for (frame_type, options) in &clear[3..] {
        appending
            .add_frame_with_options(frame_type, options.clone())
            .expect("appended frame");
        resumed
            .add_frame_with_options(frame_type, options.clone())
            .expect("resumed frame");
    }
    drop(resumed);
    assert_eq!(tail, appending.into_bytes());
    assert_eq!([prefix, tail].concat(), whole);
    let graph = read(&whole, false, ids.last().map(Vec::as_slice));
    assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
}

/// A reader that yields `remaining` patterned bytes without holding them.
struct Generated {
    remaining: u64,
}

impl Read for Generated {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let count = out
            .len()
            .min(usize::try_from(self.remaining).unwrap_or(usize::MAX));
        for (offset, byte) in out[..count].iter_mut().enumerate() {
            *byte = (self.remaining as usize).wrapping_add(offset) as u8;
        }
        self.remaining -= count as u64;
        Ok(count)
    }
}

/// Peak working bytes of one streamed frame of `len` payload bytes into a
/// sink that only counts.
fn streamed_peak(len: u64, options: FrameOptions, signing: FrameSigning) -> (i64, u64) {
    let mut accepted = 0_u64;
    let sink = FnSink(|chunk: &[u8]| -> Result<(), Boom> {
        accepted += chunk.len() as u64;
        Ok(())
    });
    let mut stream = StreamWriter::new(sink, PROFILE, WriterOptions::default()).expect("header");
    stream.sign_with(signing_key(), KID);
    let window = CurrentThreadWindow::open();
    stream
        .add_frame_from_reader("blob", len, Generated { remaining: len }, options, signing)
        .expect("streamed frame");
    let peak = window.close().peak_working_bytes;
    drop(stream);
    (peak, accepted)
}

/// A clear payload read from a reader is held one chunk at a time: the peak
/// while streaming it does not grow with its length. Public metadata,
/// recipients and a signature do not change that.
#[test]
fn a_clear_payload_from_a_reader_is_held_one_chunk_at_a_time() {
    const CHUNK: u64 = 128 * 1024;
    let options = || FrameOptions {
        pub_meta: Some(Value::Map(vec![("mt".into(), "text/plain".into())])),
        recipients: vec![Value::Map(vec![("kid".into(), "reader-1".into())])],
        ..FrameOptions::default()
    };
    for signing in [FrameSigning::Unsigned, FrameSigning::Installed] {
        let (small, small_bytes) = streamed_peak(4 * CHUNK, options(), signing);
        let (large, large_bytes) = streamed_peak(256 * CHUNK, options(), signing);
        assert!(large_bytes > small_bytes + 250 * CHUNK);
        println!("streamed peak, {signing:?}: 4 chunks {small} bytes, 256 chunks {large} bytes");
        assert_eq!(
            large, small,
            "{signing:?}: peak working bytes grew with the payload"
        );
        assert!(
            small < 2 * CHUNK as i64,
            "{signing:?}: more than a chunk and its bookkeeping is held: {small}"
        );
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Boom(u8);

/// A sink with its own error type that refuses one chosen call.
#[derive(Debug, Default)]
struct Refusing {
    accepted: Vec<u8>,
    calls: usize,
    refuse_call: Option<usize>,
}

impl FrameSink for Refusing {
    type Error = Boom;

    fn write_bytes(&mut self, chunk: &[u8]) -> Result<(), Boom> {
        let call = self.calls;
        self.calls += 1;
        if self.refuse_call == Some(call) {
            // A destination may keep part of what it refused.
            self.accepted.extend_from_slice(&chunk[..chunk.len() / 2]);
            return Err(Boom(7));
        }
        self.accepted.extend_from_slice(chunk);
        Ok(())
    }
}

/// The rule: once part of a frame is in the sink and the rest cannot follow,
/// the writer refuses every later frame; the head and `bytes_written` stay at
/// the last whole frame, which is where the caller cuts back and resumes. The
/// sink's own error value comes out unchanged.
#[test]
fn sink_failure_passes_through_poisons_and_leaves_the_chain_unadvanced() {
    let header = WriterOptions::default();
    let script = vec![
        ("blob", raw(bytes(500, 1))),
        ("blob", raw(bytes(400_000, 2))),
        ("blob", raw(bytes(700, 3))),
    ];
    let (expected, _) = oracle(&header, &script, Signed::None);

    // Healthy neighbour: the same sink type, refusing nothing.
    let mut healthy = StreamWriter::new(Refusing::default(), PROFILE, header.clone()).unwrap();
    append(&mut healthy, &script, 0..3, Signed::None, Feed::Whole);
    assert!(!healthy.is_poisoned());
    assert_eq!(healthy.into_sink().accepted, expected);

    // Call 0 is the header, 1 the first frame, 2 the whole second frame.
    for feed in [Feed::Whole, Feed::ReaderAllAtOnce] {
        // Whole: call 2 is the second frame. From a reader: calls 2.. are its
        // prefix and payload chunks, so call 4 is inside the payload.
        let refuse_call = match feed {
            Feed::Whole => 2,
            _ => 4,
        };
        let sink = Refusing {
            refuse_call: Some(refuse_call),
            ..Refusing::default()
        };
        let mut stream = StreamWriter::new(sink, PROFILE, header.clone()).unwrap();
        append(&mut stream, &script, 0..1, Signed::None, Feed::Whole);
        let head = stream.head().to_vec();
        let durable = stream.bytes_written();

        let (frame_type, options) = script[1].clone();
        let data = options.raw.clone().unwrap();
        let error = match feed {
            Feed::Whole => stream.add_frame_with_options(frame_type, options),
            _ => stream.add_frame_from_reader(
                frame_type,
                data.len() as u64,
                data.as_slice(),
                FrameOptions::default(),
                FrameSigning::Installed,
            ),
        }
        .unwrap_err();
        let (acknowledged_bytes, attempted_bytes, cause) = match error {
            StreamError::Sink {
                acknowledged_bytes,
                attempted_bytes,
                cause,
            } => (acknowledged_bytes, attempted_bytes, cause),
            other => panic!("{feed:?}: expected the sink's failure, got {other:?}"),
        };
        assert_eq!(cause, Boom(7));
        assert!(attempted_bytes > 0);
        assert!(acknowledged_bytes >= durable);
        assert_eq!(
            acknowledged_bytes > durable,
            matches!(feed, Feed::ReaderAllAtOnce),
            "{feed:?}: only a streamed frame has earlier bytes of its own in the sink"
        );
        assert!(stream.is_poisoned());
        assert_eq!(stream.head(), head.as_slice());
        assert_eq!(stream.bytes_written(), durable);

        // The next frame is valid, and is refused without touching the sink.
        let (frame_type, options) = script[2].clone();
        let poisoned = stream
            .add_frame_with_options(frame_type, options)
            .unwrap_err();
        assert!(matches!(poisoned, StreamError::Poisoned), "{poisoned:?}");
        let poisoned = stream
            .add_frame_from_reader(
                "blob",
                0,
                std::io::empty(),
                FrameOptions::default(),
                FrameSigning::Installed,
            )
            .unwrap_err();
        assert!(matches!(poisoned, StreamError::Poisoned), "{poisoned:?}");
        let sink = stream.into_sink();
        assert_eq!(sink.calls, refuse_call + 1);

        // Recovery: cut back to the last whole frame and resume from its id.
        let mut log = sink.accepted;
        assert!(log.len() as u64 > durable);
        log.truncate(usize::try_from(durable).unwrap());
        let mut resumed = StreamWriter::resume(&mut log, header.clone(), &head).unwrap();
        append(&mut resumed, &script, 1..3, Signed::None, Feed::Whole);
        drop(resumed);
        assert!(log == expected, "{feed:?}: recovered log differs");
    }
}

/// An `io::Write` that accepts three bytes per call and is interrupted once.
struct Grudging {
    accepted: Vec<u8>,
    interrupted: bool,
}

impl std::io::Write for Grudging {
    fn write(&mut self, chunk: &[u8]) -> std::io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(std::io::ErrorKind::Interrupted.into());
        }
        let count = chunk.len().min(3);
        self.accepted.extend_from_slice(&chunk[..count]);
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// An `io::Write` sink has `write_all` semantics: partial and interrupted
/// writes change nothing in the bytes or the chain.
#[test]
fn partial_and_interrupted_writes_do_not_change_the_bytes() {
    let header = WriterOptions::default();
    let script = vec![("blob", raw(bytes(900, 1))), ("blob", raw(bytes(50, 2)))];
    let (expected, ids) = oracle(&header, &script, Signed::None);
    let sink = Grudging {
        accepted: Vec::new(),
        interrupted: false,
    };
    let mut stream = StreamWriter::new(sink, PROFILE, header).unwrap();
    let streamed = append(
        &mut stream,
        &script,
        0..2,
        Signed::None,
        Feed::ReaderOneByte,
    );
    assert_eq!(streamed, ids);
    assert_eq!(stream.bytes_written(), expected.len() as u64);
    assert_eq!(stream.into_sink().accepted, expected);
}

/// A reader that fails after yielding its bytes.
struct Failing<'a>(&'a [u8]);

impl Read for Failing<'_> {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if self.0.is_empty() {
            return Err(std::io::ErrorKind::ConnectionReset.into());
        }
        let count = out.len().min(self.0.len());
        out[..count].copy_from_slice(&self.0[..count]);
        self.0 = &self.0[count..];
        Ok(count)
    }
}

/// A payload reader that disagrees with its declared length, or fails, is a
/// typed refusal. It poisons the writer exactly when the frame was being
/// streamed; a frame that is read whole first (transformed here) leaves the
/// writer usable, and the next frame continues the chain.
#[test]
fn a_reader_that_fails_or_disagrees_with_its_length_is_refused() {
    let header = WriterOptions::default();
    let data = bytes(1000, 4);
    let clear = FrameOptions::default;
    let gzip = || FrameOptions {
        transform: vec!["gzip".to_string()],
        ..FrameOptions::default()
    };
    type Case = (u64, bool, fn(&PayloadError) -> bool);
    let cases: [Case; 3] = [
        // Declared longer than the reader: it ends early.
        (1001, false, |error| {
            matches!(
                error,
                PayloadError::Length {
                    declared: 1001,
                    observed: 1000
                }
            )
        }),
        // Declared shorter than the reader: it runs on.
        (
            999,
            false,
            |error| matches!(error, PayloadError::Length { declared: 999, observed } if *observed > 999),
        ),
        // The reader itself fails after its bytes.
        (1001, true, |error| {
            matches!(error, PayloadError::Read(cause)
                if cause.kind() == std::io::ErrorKind::ConnectionReset)
        }),
    ];
    for (declared, failing, expected) in cases {
        for (options, streamed) in [(clear(), true), (gzip(), false)] {
            let mut log = Vec::new();
            let mut stream = StreamWriter::new(&mut log, PROFILE, header.clone()).unwrap();
            let head = stream.head().to_vec();
            let durable = stream.bytes_written();
            let signing = FrameSigning::Installed;
            let error = if failing {
                stream.add_frame_from_reader("blob", declared, Failing(&data), options, signing)
            } else {
                stream.add_frame_from_reader("blob", declared, data.as_slice(), options, signing)
            }
            .unwrap_err();
            let error = match error {
                StreamError::Payload(error) => error,
                other => panic!("expected a payload refusal, got {other:?}"),
            };
            assert!(
                expected(&error),
                "{declared} {failing} {streamed}: {error:?}"
            );
            assert_eq!(stream.is_poisoned(), streamed);
            assert_eq!(stream.head(), head.as_slice());
            assert_eq!(stream.bytes_written(), durable);
            if streamed {
                continue;
            }
            // The valid neighbour: the exact length, on the same writer.
            stream
                .add_frame_from_reader("blob", 1000, data.as_slice(), gzip(), signing)
                .expect("the exact length is accepted");
            drop(stream);
            let script = vec![("blob", transformed(data.clone(), "gzip"))];
            assert_eq!(log, oracle(&header, &script, Signed::None).0);
        }
    }
}

fn refusal<T: std::fmt::Debug, E: std::fmt::Debug>(
    result: Result<T, StreamError<E>>,
) -> FrameError {
    match result {
        Err(StreamError::Frame(WriterError::InvalidFrame(detail))) => detail,
        other => panic!("expected a refused frame, got {other:?}"),
    }
}

/// A refused frame writes nothing and leaves the writer usable, and each
/// refusal has a valid neighbour that succeeds: the stream after the refusals
/// is the log of a writer that never saw them.
#[test]
fn refused_frame_writes_nothing_and_the_valid_neighbour_succeeds() {
    let header = WriterOptions::default();
    let data = bytes(300, 5);
    let explicit = || FrameOptions {
        signature: Some(b"explicit".to_vec()),
        ..raw(data.clone())
    };
    let leveled = |transform: &[&str]| FrameOptions {
        zstd_level: Some(3),
        transform: transform.iter().map(|name| (*name).to_string()).collect(),
        ..FrameOptions::default()
    };
    let mut log = Vec::new();
    let mut stream = StreamWriter::new(&mut log, PROFILE, header.clone()).unwrap();
    stream.sign_with(signing_key(), KID);
    let header_len = stream.bytes_written();
    let len = data.len() as u64;
    let unsigned = FrameSigning::Unsigned;
    let installed = FrameSigning::Installed;

    // Two sources at once.
    let both = FrameOptions {
        payload: Some(Value::from(1)),
        ..raw(data.clone())
    };
    assert!(matches!(
        refusal(stream.add_frame_with_options("blob", both)),
        FrameError::ConflictingSources
    ));
    // Explicit signature bytes on a frame left unsigned, by either entry.
    assert!(matches!(
        refusal(stream.add_frame_signing("blob", explicit(), unsigned)),
        FrameError::ConflictingSigning
    ));
    let explicit_reader = FrameOptions {
        raw: None,
        ..explicit()
    };
    assert!(matches!(
        refusal(stream.add_frame_from_reader(
            "blob",
            len,
            data.as_slice(),
            explicit_reader,
            unsigned
        )),
        FrameError::ConflictingSigning
    ));
    // A reader beside another source.
    assert!(matches!(
        refusal(stream.add_frame_from_reader(
            "blob",
            len,
            data.as_slice(),
            raw(data.clone()),
            installed
        )),
        FrameError::ReaderSource
    ));
    // A compression level with nothing to compress.
    assert!(matches!(
        refusal(stream.add_frame_from_reader(
            "blob",
            len,
            data.as_slice(),
            leveled(&[]),
            installed
        )),
        FrameError::FrameLevel { level: 3 }
    ));
    assert_eq!(stream.bytes_written(), header_len);
    assert!(!stream.is_poisoned());

    // The neighbours: unsigned alone, explicit bytes alone, a reader alone,
    // and the level with its transform.
    stream
        .add_frame_signing("blob", raw(data.clone()), unsigned)
        .unwrap();
    stream
        .add_frame_signing("blob", explicit(), installed)
        .unwrap();
    stream
        .add_frame_from_reader(
            "blob",
            len,
            data.as_slice(),
            FrameOptions::default(),
            installed,
        )
        .unwrap();
    stream
        .add_frame_from_reader("blob", len, data.as_slice(), leveled(&["zstd"]), unsigned)
        .unwrap();
    drop(stream);

    let mut expected = Writer::with_options(PROFILE, header).unwrap();
    expected
        .add_frame_with_options("blob", raw(data.clone()))
        .unwrap();
    expected.add_frame_with_options("blob", explicit()).unwrap();
    expected.sign_with(signing_key(), KID);
    expected
        .add_frame_with_options("blob", raw(data.clone()))
        .unwrap();
    let mut last = Writer::appending(&expected.to_bytes()).unwrap();
    last.add_frame_with_options(
        "blob",
        FrameOptions {
            raw: Some(data),
            ..leveled(&["zstd"])
        },
    )
    .unwrap();
    assert_eq!(log, [expected.into_bytes(), last.into_bytes()].concat());
}

/// A stored previous-frame id is one 32-byte digest: any other length is
/// refused, and the id of a real stream is accepted.
#[test]
fn resume_refuses_a_previous_hash_that_is_not_32_bytes_and_accepts_32() {
    let stream = StreamWriter::new(Vec::new(), PROFILE, WriterOptions::default()).unwrap();
    let head = stream.head().to_vec();
    assert_eq!(head.len(), 32);
    for length in [0, 31, 33] {
        let mut previous = head.clone();
        previous.resize(length, 0);
        let refused = StreamWriter::resume(Vec::new(), WriterOptions::default(), &previous);
        assert!(matches!(
            refusal(refused),
            FrameError::PreviousHashLength { actual } if actual == length
        ));
    }
    let resumed = StreamWriter::resume(Vec::new(), WriterOptions::default(), &head).unwrap();
    assert_eq!(resumed.head(), head.as_slice());
    assert_eq!(resumed.bytes_written(), 0);
}
