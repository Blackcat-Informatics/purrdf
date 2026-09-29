// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Round trips, hand-built edge streams, refusals beside their valid
//! neighbours, and the kernel differentials.
//!
//! Every refusal here is executed twice: once on the stream believed invalid,
//! and once on the nearest stream that is valid, which must decode. The
//! target is `harness = false` on `purrdf_testkit`'s runner, so the same named
//! cases run natively under `cargo test` and on wasm32-unknown-unknown in
//! Node (`make wasm-test`, baseline and `+simd128`).

use std::io::{Read, Write};

use purrdf_deflate::backend::Backend;
use purrdf_deflate::{
    Deflater, Error, GzipDecoder, GzipReader, GzipWriter, Inflater, Level, Status, deflate, gzip,
    inflate,
};
use purrdf_hash::Backend as _;
use purrdf_hash::crc32::Crc32;
use purrdf_hash::dispatch::{assert_required_available, host_advertises};
use purrdf_testkit::rng::Xoshiro256;

// --- Inputs ------------------------------------------------------------------

fn random_bytes(len: usize, seed: u64) -> Vec<u8> {
    let mut rng = Xoshiro256::from_seed(seed);
    let mut out = Vec::with_capacity(len + 8);
    while out.len() < len {
        out.extend_from_slice(&rng.next_u64().to_le_bytes());
    }
    out.truncate(len);
    out
}

fn text(len: usize, seed: u64) -> Vec<u8> {
    const WORDS: [&str; 16] = [
        "the",
        "subject",
        "predicate",
        "object",
        "graph",
        "literal",
        "of",
        "and",
        "a",
        "resource",
        "language",
        "datatype",
        "blank",
        "node",
        "triple",
        "quad",
    ];
    let mut rng = Xoshiro256::from_seed(seed);
    let mut out = Vec::with_capacity(len + 16);
    while out.len() < len {
        out.extend_from_slice(WORDS[rng.up_to(15) as usize].as_bytes());
        out.push(if rng.up_to(11) == 0 { b'\n' } else { b' ' });
    }
    out.truncate(len);
    out
}

fn repeated_token(len: usize, seed: u64) -> Vec<u8> {
    let mut rng = Xoshiro256::from_seed(seed);
    let mut out = Vec::with_capacity(len + 128);
    while out.len() < len {
        let n = rng.up_to(999);
        out.extend_from_slice(
            format!("<http://example.org/s{n}> <http://example.org/p> \"v{n}\" .\n").as_bytes(),
        );
    }
    out.truncate(len);
    out
}

const SIZES: [usize; 14] = [
    0, 1, 2, 3, 4, 5, 259, 32_767, 32_768, 32_769, 65_535, 65_536, 65_537, 200_003,
];

fn inputs() -> Vec<(String, Vec<u8>)> {
    let mut all = Vec::new();
    for (i, &size) in SIZES.iter().enumerate() {
        let seed = 0x5EED_0000 + i as u64;
        all.push((format!("text/{size}"), text(size, seed)));
        all.push((format!("random/{size}"), random_bytes(size, seed)));
        all.push((format!("tokens/{size}"), repeated_token(size, seed)));
    }
    all.push(("zeros/100000".into(), vec![0; 100_000]));
    all
}

fn crc32(data: &[u8]) -> u32 {
    Crc32::checksum(data)
}

// --- Hand-built streams ------------------------------------------------------

/// An LSB-first bit writer for crafting streams by hand.
#[derive(Default)]
struct Bits {
    out: Vec<u8>,
    acc: u64,
    n: u32,
}

impl Bits {
    fn put(&mut self, value: u32, count: u32) {
        for i in 0..count {
            self.acc |= u64::from((value >> i) & 1) << self.n;
            self.n += 1;
            if self.n == 8 {
                self.out.push(self.acc as u8);
                self.acc = 0;
                self.n = 0;
            }
        }
    }

    /// A Huffman code, most significant bit first.
    fn code(&mut self, code: u32, len: u32) {
        for i in (0..len).rev() {
            self.put((code >> i) & 1, 1);
        }
    }

    fn align(&mut self) {
        if self.n > 0 {
            self.out.push(self.acc as u8);
            self.acc = 0;
            self.n = 0;
        }
    }

    fn bytes(&mut self, bytes: &[u8]) {
        assert_eq!(self.n, 0);
        self.out.extend_from_slice(bytes);
    }

    fn finish(mut self) -> Vec<u8> {
        self.align();
        self.out
    }

    /// A fixed-code literal/length symbol (RFC 1951 §3.2.6).
    fn fixed(&mut self, symbol: u32) {
        match symbol {
            0..=143 => self.code(0x30 + symbol, 8),
            144..=255 => self.code(0x190 + symbol - 144, 9),
            256..=279 => self.code(symbol - 256, 7),
            _ => self.code(0xC0 + symbol - 280, 8),
        }
    }

    /// A fixed-code distance symbol.
    fn fixed_distance(&mut self, symbol: u32) {
        self.code(symbol, 5);
    }

    fn header(&mut self, final_block: bool, kind: u32) {
        self.put(u32::from(final_block), 1);
        self.put(kind, 2);
    }
}

/// A stored block holding `data` (at most 65535 bytes).
fn stored(bits: &mut Bits, data: &[u8], final_block: bool) {
    bits.header(final_block, 0);
    bits.align();
    let len = data.len() as u16;
    bits.bytes(&len.to_le_bytes());
    bits.bytes(&(!len).to_le_bytes());
    bits.bytes(data);
}

/// Canonical codes for `lengths` (RFC 1951 §3.2.2), MSB-first values.
fn canonical(lengths: &[u8]) -> Vec<u32> {
    let mut count = [0u32; 16];
    for &l in lengths {
        count[usize::from(l)] += 1;
    }
    count[0] = 0;
    let mut next = [0u32; 16];
    let mut code = 0;
    for len in 1..16 {
        code = (code + count[len - 1]) << 1;
        next[len] = code;
    }
    lengths
        .iter()
        .map(|&l| {
            if l == 0 {
                0
            } else {
                let c = next[usize::from(l)];
                next[usize::from(l)] += 1;
                c
            }
        })
        .collect()
}

/// A dynamic block header whose code-length code gives symbols 0–15 four
/// bits each, then every length written literally.
fn dynamic_header(bits: &mut Bits, final_block: bool, litlen: &[u8], dist: &[u8]) {
    bits.header(final_block, 2);
    bits.put((litlen.len() - 257) as u32, 5);
    bits.put((dist.len() - 1) as u32, 5);
    bits.put(19 - 4, 4);
    let mut cl = [0u8; 19];
    cl[..16].fill(4);
    const ORDER: [usize; 19] = [
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    for symbol in ORDER {
        bits.put(u32::from(cl[symbol]), 3);
    }
    let codes = canonical(&cl);
    for &l in litlen.iter().chain(dist) {
        bits.code(codes[usize::from(l)], 4);
    }
}

/// A gzip member around `body` whose trailer describes `payload`.
fn member(flags: u8, body: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut out = vec![0x1f, 0x8b, 8, flags, 0, 0, 0, 0, 0, 255];
    out.extend_from_slice(body);
    out.extend_from_slice(&crc32(payload).to_le_bytes());
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out
}

/// A fixed block: the literals of `prefix`, then one match, then end.
fn fixed_with_match(
    prefix: &[u8],
    length_symbol: u32,
    extra: (u32, u32),
    dist_symbol: u32,
) -> Vec<u8> {
    let mut bits = Bits::default();
    bits.header(true, 1);
    for &b in prefix {
        bits.fixed(u32::from(b));
    }
    bits.fixed(length_symbol);
    bits.put(extra.0, extra.1);
    bits.fixed_distance(dist_symbol);
    bits.fixed(256);
    bits.finish()
}

// --- Round trips -------------------------------------------------------------

fn round_trips_across_levels_and_window_limits() {
    for (name, data) in inputs() {
        let levels: &[u8] = if data.len() > 70_000 {
            &[0, 1, 6, 9]
        } else {
            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
        };
        for &level in levels {
            let level = Level::new(level).expect("a level");
            let raw = deflate::compress(&data, level);
            assert_eq!(
                inflate::decompress(&raw).unwrap_or_else(|e| panic!("{name} {level:?}: {e}")),
                data,
                "{name} at {level:?}"
            );
            let framed = gzip::compress(&data, level);
            assert_eq!(gzip::decompress(&framed).expect("gzip"), data, "{name}");
        }
    }
}

fn encoder_output_is_independent_of_write_chunking() {
    let data = repeated_token(150_000, 7);
    for level in [Level::NONE, Level::FASTEST, Level::DEFAULT, Level::BEST] {
        let whole = deflate::compress(&data, level);
        let mut rng = Xoshiro256::from_seed(99);
        let mut deflater = Deflater::new(level);
        let mut gzip_writer = GzipWriter::new(Vec::new(), level);
        let mut out = Vec::new();
        let mut rest = data.as_slice();
        while !rest.is_empty() {
            let n = (rng.up_to(5000) as usize + 1).min(rest.len());
            deflater.write(&rest[..n], &mut out);
            gzip_writer.write_all(&rest[..n]).expect("gzip chunk");
            rest = &rest[n..];
        }
        deflater.finish(&mut out);
        assert_eq!(out, whole, "{level:?}");
        assert_eq!(
            gzip_writer.finish().expect("gzip finish"),
            gzip::compress(&data, level),
            "one-shot and chunked gzip at {level:?}"
        );
    }
}

fn every_kernel_path_encodes_and_decodes_the_same_bytes() {
    let data = [
        text(90_000, 3),
        random_bytes(20_000, 4),
        repeated_token(90_000, 5),
    ]
    .concat();
    let reference = {
        let mut d = Deflater::with_backend(Level::DEFAULT, Backend::Portable).expect("portable");
        let mut out = Vec::new();
        d.write(&data, &mut out);
        d.finish(&mut out);
        out
    };
    let mut ran = 0;
    for backend in Backend::all_available() {
        let mut d = Deflater::with_backend(Level::DEFAULT, backend).expect("available");
        let mut out = Vec::new();
        d.write(&data, &mut out);
        d.finish(&mut out);
        assert_eq!(out, reference, "encoder on {}", backend.name());
        let mut inflater = Inflater::with_backend(backend).expect("available");
        let mut decoded = Vec::new();
        let progress = inflater.feed_to_vec(&out, &mut decoded).expect("decode");
        assert_eq!(progress.status, Status::StreamEnd);
        assert_eq!(decoded, data, "decoder on {}", backend.name());
        purrdf_testkit::harness::print_line(&format!("deflate: {} agrees", backend.name()));
        ran += 1;
    }
    assert!(ran >= 1);
}

fn push_decoding_in_tiny_pieces_matches_one_shot() {
    let data = [text(70_000, 11), repeated_token(40_000, 12)].concat();
    let framed = [
        gzip::compress(&data, Level::DEFAULT),
        gzip::compress(&data[..1000], Level::NONE),
    ]
    .concat();
    let expected = [data.as_slice(), &data[..1000]].concat();
    let mut rng = Xoshiro256::from_seed(5);
    let mut decoder = GzipDecoder::new();
    let mut out = Vec::new();
    let mut rest = framed.as_slice();
    let mut buf = [0u8; 97];
    loop {
        let take = (rng.up_to(13) as usize).min(rest.len());
        let room = rng.up_to(96) as usize + 1;
        let progress = decoder.feed(&rest[..take], &mut buf[..room]).expect("feed");
        out.extend_from_slice(&buf[..progress.written]);
        rest = &rest[progress.consumed..];
        if rest.is_empty() && progress.status == Status::NeedsInput {
            break;
        }
    }
    decoder.finish().expect("complete");
    assert_eq!(decoder.members(), 2);
    assert_eq!(out, expected);

    // The Read adapter over a reader that yields one byte at a time.
    struct Trickle<'a>(&'a [u8]);
    impl Read for Trickle<'_> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.0.is_empty() || buf.is_empty() {
                return Ok(0);
            }
            buf[0] = self.0[0];
            self.0 = &self.0[1..];
            Ok(1)
        }
    }
    let mut via_reader = Vec::new();
    GzipReader::new(Trickle(&framed))
        .read_to_end(&mut via_reader)
        .expect("reader");
    assert_eq!(via_reader, expected);
}

fn gzip_writer_header_is_deterministic() {
    let mut writer = GzipWriter::new(Vec::new(), Level::DEFAULT);
    writer.write_all(b"hello").expect("write");
    let bytes = writer.finish().expect("finish");
    assert_eq!(&bytes[..10], &[0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 255]);
    assert_eq!(bytes, gzip::compress(b"hello", Level::DEFAULT));
    assert_eq!(&bytes[bytes.len() - 4..], &5u32.to_le_bytes());
}

fn gzip_writer_limits_acceptance_and_recovers_sink_progress() {
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Default)]
    struct State {
        bytes: Vec<u8>,
        calls: usize,
        fail_next: bool,
        fail_after_writes: Option<usize>,
    }
    struct Sink(Rc<RefCell<State>>);
    impl Write for Sink {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let mut state = self.0.borrow_mut();
            state.calls += 1;
            if let Some(remaining) = state.fail_after_writes.as_mut() {
                if *remaining == 0 {
                    state.fail_after_writes = None;
                    return Err(std::io::ErrorKind::WouldBlock.into());
                }
                *remaining -= 1;
            }
            if state.fail_next {
                state.fail_next = false;
                return Err(std::io::ErrorKind::WouldBlock.into());
            }
            let taken = bytes.len().min(7);
            state.bytes.extend_from_slice(&bytes[..taken]);
            Ok(taken)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let state = Rc::new(RefCell::new(State::default()));
    let mut writer = GzipWriter::new(Sink(Rc::clone(&state)), Level::NONE);
    let data = vec![0xA5; 2 * 1024 * 1024];
    let first = writer.write(&data).expect("first bounded slice");
    assert!(first > 0 && first < data.len());
    state.borrow_mut().fail_after_writes = Some(2);
    assert_eq!(
        writer.flush().expect_err("partial sink progress").kind(),
        std::io::ErrorKind::WouldBlock
    );
    let before = state.borrow().bytes.len();
    assert!(before > 0);
    state.borrow_mut().fail_next = true;
    assert_eq!(
        writer
            .write(&data[first..])
            .expect_err("sink failure")
            .kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert!(state.borrow().bytes.len() >= before);
    writer
        .write_all(&data[first..])
        .expect("retry without lost input");
    writer.finish().expect("finish");
    assert!(state.borrow().calls > 2);
    assert_eq!(
        gzip::decompress(&state.borrow().bytes).expect("decode"),
        data
    );
}

fn gzip_writer_flush_exposes_accepted_plaintext() {
    use std::cell::RefCell;
    use std::rc::Rc;

    struct Sink(Rc<RefCell<Vec<u8>>>);
    impl Write for Sink {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let seen = Rc::new(RefCell::new(Vec::new()));
    let mut writer = GzipWriter::new(Sink(Rc::clone(&seen)), Level::DEFAULT);
    writer.write_all(b"first segment").expect("write");
    writer.flush().expect("sync flush");
    let partial = seen.borrow().clone();
    // The member has no trailer yet, but its DEFLATE body must already
    // contain all accepted plaintext for an independent streaming decoder.
    let mut inflater = Inflater::new();
    let mut output = [0; 64];
    let progress = inflater
        .feed(&partial[10..], &mut output)
        .expect("decode flush boundary");
    assert_eq!(&output[..progress.written], b"first segment");
    writer.write_all(b" and second").expect("write after flush");
    writer.finish().expect("finish");
    assert_eq!(
        gzip::decompress(&seen.borrow()).expect("whole member"),
        b"first segment and second"
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn gzip_interoperates_with_python_zlib() {
    use std::process::{Command, Stdio};

    fn python(script: &str, input: &[u8]) -> Vec<u8> {
        let mut child = Command::new("python3")
            .args(["-c", script])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("python3 test oracle");
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(input)
            .expect("feed oracle");
        let result = child.wait_with_output().expect("oracle result");
        assert!(result.status.success(), "python oracle failed");
        result.stdout
    }

    let inputs = [
        Vec::new(),
        b"<s> <p> <o> .\n".repeat(1024),
        (0..131_072).map(|i| (i * 17 % 251) as u8).collect(),
    ];
    for input in inputs {
        for level in [Level::NONE, Level::FASTEST, Level::DEFAULT, Level::BEST] {
            let framed = gzip::compress(&input, level);
            assert_eq!(
                python(
                    "import gzip,sys; sys.stdout.buffer.write(gzip.decompress(sys.stdin.buffer.read()))",
                    &framed
                ),
                input
            );
            let external = python(
                "import gzip,sys; sys.stdout.buffer.write(gzip.compress(sys.stdin.buffer.read(), mtime=0))",
                &input,
            );
            assert_eq!(gzip::decompress(&external).expect("external gzip"), input);
            let mut writer = GzipWriter::new(Vec::new(), level);
            let split = input.len() / 2;
            writer.write_all(&input[..split]).expect("first segment");
            writer.flush().expect("sync boundary");
            writer.write_all(&input[split..]).expect("second segment");
            let flushed = writer.finish().expect("finish sync stream");
            assert_eq!(
                python(
                    "import gzip,sys; sys.stdout.buffer.write(gzip.decompress(sys.stdin.buffer.read()))",
                    &flushed
                ),
                input
            );
        }
    }
}

// --- Hand-built edge streams -------------------------------------------------

fn empty_final_stored_block_decodes_to_nothing() {
    let mut bits = Bits::default();
    stored(&mut bits, b"", true);
    let raw = bits.finish();
    assert_eq!(raw, [0x01, 0x00, 0x00, 0xff, 0xff]);
    assert_eq!(inflate::decompress(&raw).expect("empty"), b"");
}

fn fifteen_bit_codes_decode() {
    // Lengths 1..=15 then a second 15: a complete code over 16 symbols. The
    // end-of-block symbol and the literal 'z' get the two 15-bit codes.
    let mut litlen = vec![0u8; 257];
    let symbols: [usize; 16] = [
        b'a' as usize,
        b'b' as usize,
        b'c' as usize,
        b'd' as usize,
        b'e' as usize,
        b'f' as usize,
        b'g' as usize,
        b'h' as usize,
        b'i' as usize,
        b'j' as usize,
        b'k' as usize,
        b'l' as usize,
        b'm' as usize,
        b'n' as usize,
        b'z' as usize,
        256,
    ];
    for (i, &s) in symbols.iter().enumerate() {
        litlen[s] = (i + 1).min(15) as u8;
    }
    let dist = vec![0u8; 1];
    let codes = canonical(&litlen);
    let mut bits = Bits::default();
    dynamic_header(&mut bits, true, &litlen, &dist);
    let message = b"zanzibar is not in this alphabet but z is";
    let mut expected = Vec::new();
    for &b in message {
        if litlen[usize::from(b)] > 0 {
            bits.code(codes[usize::from(b)], u32::from(litlen[usize::from(b)]));
            expected.push(b);
        }
    }
    bits.code(codes[256], 15);
    assert_eq!(litlen[b'z' as usize], 15);
    assert_eq!(
        inflate::decompress(&bits.finish()).expect("15-bit"),
        expected
    );
}

fn distance_32768_with_length_258_decodes() {
    let pattern = random_bytes(32_768, 77);
    let mut bits = Bits::default();
    stored(&mut bits, &pattern, false);
    bits.header(true, 1);
    bits.fixed(285);
    bits.fixed_distance(29);
    bits.put(32_768 - 24_577, 13);
    bits.fixed(256);
    let decoded = inflate::decompress(&bits.finish()).expect("far match");
    assert_eq!(decoded.len(), 32_768 + 258);
    assert_eq!(&decoded[32_768..], &pattern[..258]);
}

fn gzip_header_with_every_optional_field_decodes() {
    let payload = b"payload with every header field";
    let body = deflate::compress(payload, Level::DEFAULT);
    let flags = 0x01 | 0x02 | 0x04 | 0x08 | 0x10;
    let mut header = vec![0x1f, 0x8b, 8, flags, 1, 2, 3, 4, 0, 3];
    header.extend_from_slice(&5u16.to_le_bytes());
    header.extend_from_slice(b"XY\x01\x00Z");
    header.extend_from_slice(b"name.nt\0");
    header.extend_from_slice(b"a comment\0");
    let hcrc = (crc32(&header) & 0xFFFF) as u16;
    let mut good = header.clone();
    good.extend_from_slice(&hcrc.to_le_bytes());
    good.extend_from_slice(&body);
    good.extend_from_slice(&crc32(payload).to_le_bytes());
    good.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    assert_eq!(gzip::decompress(&good).expect("all fields"), payload);
}

fn tar_vector_gzip_files_decode() {
    let data = include_bytes!("../../../vectors/tar/gzip-basic.tar.gz");
    let tar = gzip::decompress(data).expect("the frozen gzip tar vector decodes");
    assert_eq!(tar.len() % 512, 0, "a tar stream is whole 512-byte records");
    assert!(tar.windows(15).any(|w| w == b"docs/readme.txt"));
}

// --- Refusals beside their valid neighbours -----------------------------------

fn truncation_refused_complete_accepted() {
    let data = text(5000, 1);
    let framed = gzip::compress(&data, Level::DEFAULT);
    assert_eq!(gzip::decompress(&framed).expect("complete"), data);
    for cut in 0..framed.len() {
        let err = gzip::decompress(&framed[..cut]).expect_err("truncated");
        let expected = if cut == 0 {
            Error::Empty
        } else {
            Error::Truncated
        };
        assert_eq!(err, expected, "cut at {cut}");
    }
    let raw = deflate::compress(&data, Level::DEFAULT);
    assert_eq!(inflate::decompress(&raw).expect("complete"), data);
    for cut in [0, 1, raw.len() / 2, raw.len() - 1] {
        assert_eq!(inflate::decompress(&raw[..cut]), Err(Error::Truncated));
    }
}

fn bad_crc_refused_in_first_and_second_member() {
    let one = gzip::compress(b"first member", Level::DEFAULT);
    let two = gzip::compress(b"second member", Level::DEFAULT);
    let good = [one.clone(), two.clone()].concat();
    assert_eq!(
        gzip::decompress(&good).expect("good"),
        b"first membersecond member"
    );

    let mut bad_first = one.clone();
    let at = bad_first.len() - 8;
    bad_first[at] ^= 1;
    let err = gzip::decompress(&[bad_first, two.clone()].concat()).expect_err("bad crc");
    assert!(
        matches!(err, Error::CrcMismatch { member: 0, .. }),
        "{err:?}"
    );

    let mut bad_second = two;
    let at = bad_second.len() - 8;
    bad_second[at] ^= 0x80;
    let err = gzip::decompress(&[one, bad_second].concat()).expect_err("bad crc");
    assert!(
        matches!(err, Error::CrcMismatch { member: 1, .. }),
        "{err:?}"
    );
}

fn bad_isize_refused_good_accepted() {
    let good = gzip::compress(b"size matters", Level::DEFAULT);
    assert!(gzip::decompress(&good).is_ok());
    let mut bad = good;
    let at = bad.len() - 4;
    bad[at] = bad[at].wrapping_add(1);
    assert!(matches!(
        gzip::decompress(&bad),
        Err(Error::SizeMismatch {
            member: 0,
            stored: 13,
            computed: 12
        })
    ));
}

fn method_other_than_deflate_refused() {
    let good = gzip::compress(b"cm", Level::DEFAULT);
    assert!(gzip::decompress(&good).is_ok());
    for cm in [0u8, 7, 9, 255] {
        let mut bad = good.clone();
        bad[2] = cm;
        assert_eq!(
            gzip::decompress(&bad),
            Err(Error::UnsupportedMethod { method: cm })
        );
    }
}

fn reserved_flag_bits_refused_clear_accepted() {
    let body = deflate::compress(b"flags", Level::DEFAULT);
    assert_eq!(
        gzip::decompress(&member(0, &body, b"flags")).expect("clear"),
        b"flags"
    );
    // FTEXT alone is a defined bit, not a reserved one.
    assert_eq!(
        gzip::decompress(&member(1, &body, b"flags")).expect("ftext"),
        b"flags"
    );
    for bit in 5..8 {
        let flags = 1u8 << bit;
        assert_eq!(
            gzip::decompress(&member(flags, &body, b"flags")),
            Err(Error::ReservedFlags { flags })
        );
    }
}

fn header_crc_mismatch_refused_match_accepted() {
    let body = deflate::compress(b"hcrc", Level::DEFAULT);
    let header = vec![0x1f, 0x8b, 8, 0x02, 0, 0, 0, 0, 0, 255];
    let hcrc = (crc32(&header) & 0xFFFF) as u16;
    let build = |crc: u16| {
        let mut out = header.clone();
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc32(b"hcrc").to_le_bytes());
        out.extend_from_slice(&4u32.to_le_bytes());
        out
    };
    assert_eq!(gzip::decompress(&build(hcrc)).expect("match"), b"hcrc");
    assert_eq!(
        gzip::decompress(&build(hcrc ^ 1)),
        Err(Error::HeaderCrcMismatch {
            stored: hcrc ^ 1,
            computed: hcrc
        })
    );
}

fn block_type_11_refused_10_accepted() {
    // BTYPE 10 (dynamic) with a valid header.
    let litlen = {
        let mut l = vec![0u8; 257];
        l[b'q' as usize] = 1;
        l[256] = 1;
        l
    };
    let mut bits = Bits::default();
    dynamic_header(&mut bits, true, &litlen, &[0]);
    let codes = canonical(&litlen);
    bits.code(codes[b'q' as usize], 1);
    bits.code(codes[256], 1);
    assert_eq!(inflate::decompress(&bits.finish()).expect("btype 10"), b"q");

    let mut bits = Bits::default();
    bits.header(true, 3);
    bits.put(0, 13);
    assert_eq!(
        inflate::decompress(&bits.finish()),
        Err(Error::InvalidBlockType)
    );
}

fn stored_len_nlen_mismatch_refused_match_accepted() {
    let mut bits = Bits::default();
    stored(&mut bits, b"abc", true);
    let good = bits.finish();
    assert_eq!(inflate::decompress(&good).expect("match"), b"abc");
    let mut bad = good;
    bad[3] ^= 0x01;
    assert!(matches!(
        inflate::decompress(&bad),
        Err(Error::StoredLengthMismatch { len: 3, .. })
    ));
}

fn distance_beyond_output_refused_within_accepted() {
    // "ab" then length 3 (symbol 257) at distance 2 (symbol 1): "ababa".
    let within = fixed_with_match(b"ab", 257, (0, 0), 1);
    assert_eq!(inflate::decompress(&within).expect("within"), b"ababa");
    // Distance 3 (symbol 2) reaches before the first byte.
    let beyond = fixed_with_match(b"ab", 257, (0, 0), 2);
    assert_eq!(
        inflate::decompress(&beyond),
        Err(Error::DistanceTooFar {
            distance: 3,
            available: 2
        })
    );
}

fn length_symbols_286_287_refused_285_accepted() {
    let valid = fixed_with_match(b"x", 285, (0, 0), 0);
    assert_eq!(inflate::decompress(&valid).expect("285"), vec![b'x'; 259]);
    for symbol in [286, 287] {
        let bad = fixed_with_match(b"x", symbol, (0, 0), 0);
        assert_eq!(
            inflate::decompress(&bad),
            Err(Error::InvalidLengthSymbol {
                symbol: symbol as u16
            })
        );
    }
}

fn distance_symbols_30_31_refused_29_accepted() {
    let pattern = random_bytes(30_000, 8);
    let build = |symbol: u32| {
        let mut bits = Bits::default();
        stored(&mut bits, &pattern, false);
        bits.header(true, 1);
        bits.fixed(257);
        bits.fixed_distance(symbol);
        if symbol == 29 {
            bits.put(0, 13);
        }
        bits.fixed(256);
        bits.finish()
    };
    let ok = inflate::decompress(&build(29)).expect("29");
    assert_eq!(
        &ok[30_000..],
        &pattern[30_000 - 24_577..30_000 - 24_577 + 3]
    );
    for symbol in [30, 31] {
        assert_eq!(
            inflate::decompress(&build(symbol)),
            Err(Error::InvalidDistanceSymbol {
                symbol: symbol as u16
            })
        );
    }
}

fn oversubscribed_code_refused_complete_accepted() {
    // Literal/length lengths: three one-bit codes is over-subscribed; two is
    // complete.
    let mut litlen = vec![0u8; 257];
    litlen[b'a' as usize] = 1;
    litlen[256] = 1;
    let mut bits = Bits::default();
    dynamic_header(&mut bits, true, &litlen, &[0]);
    let codes = canonical(&litlen);
    bits.code(codes[b'a' as usize], 1);
    bits.code(codes[256], 1);
    assert_eq!(inflate::decompress(&bits.finish()).expect("complete"), b"a");

    litlen[b'b' as usize] = 1;
    let mut bits = Bits::default();
    dynamic_header(&mut bits, true, &litlen, &[0]);
    bits.put(0, 8);
    assert_eq!(
        inflate::decompress(&bits.finish()),
        Err(Error::OversubscribedCode {
            alphabet: purrdf_deflate::Alphabet::LiteralLength
        })
    );
}

fn single_code_distance_tree_accepted() {
    // RFC 1951 §3.2.7: one distance code is encoded with one bit, leaving
    // one code unused.
    let mut litlen = vec![0u8; 258];
    litlen[b'r' as usize] = 1;
    litlen[256] = 2;
    litlen[257] = 2;
    let dist = [1u8];
    let lcodes = canonical(&litlen);
    let dcodes = canonical(&dist);
    let mut bits = Bits::default();
    dynamic_header(&mut bits, true, &litlen, &dist);
    bits.code(lcodes[b'r' as usize], 1);
    bits.code(lcodes[257], 2); // length 3
    bits.code(dcodes[0], 1); // distance 1
    bits.code(lcodes[256], 2);
    assert_eq!(
        inflate::decompress(&bits.finish()).expect("one distance code"),
        b"rrrr"
    );
}

fn hlit_over_286_refused_286_accepted() {
    let build = |hlit: usize| {
        let mut litlen = vec![0u8; hlit];
        litlen[b'k' as usize] = 1;
        litlen[256] = 1;
        let codes = canonical(&litlen);
        let mut bits = Bits::default();
        dynamic_header(&mut bits, true, &litlen, &[0]);
        bits.code(codes[b'k' as usize], 1);
        bits.code(codes[256], 1);
        bits.finish()
    };
    assert_eq!(inflate::decompress(&build(286)).expect("286"), b"k");
    for hlit in [287, 288] {
        assert_eq!(
            inflate::decompress(&build(hlit)),
            Err(Error::TooManyLengthCodes { count: hlit as u16 })
        );
    }
}

fn missing_end_of_block_refused_present_accepted() {
    let mut litlen = vec![0u8; 257];
    litlen[b'e' as usize] = 1;
    litlen[b'f' as usize] = 1;
    let mut bits = Bits::default();
    dynamic_header(&mut bits, true, &litlen, &[0]);
    bits.put(0, 8);
    assert_eq!(
        inflate::decompress(&bits.finish()),
        Err(Error::MissingEndOfBlock)
    );
    litlen[b'f' as usize] = 0;
    litlen[256] = 1;
    let codes = canonical(&litlen);
    let mut bits = Bits::default();
    dynamic_header(&mut bits, true, &litlen, &[0]);
    bits.code(codes[b'e' as usize], 1);
    bits.code(codes[256], 1);
    assert_eq!(inflate::decompress(&bits.finish()).expect("present"), b"e");
}

/// A final dynamic block whose code lengths are sent as the raw code-length
/// symbols `runs` (with their extra values) under a code-length code giving
/// all 19 symbols five bits, followed by the MSB-first `data` codes.
fn run_coded_block(runs: &[(u32, u32)], hlit: usize, hdist: usize, data: &[(u32, u32)]) -> Vec<u8> {
    let mut bits = Bits::default();
    bits.header(true, 2);
    bits.put((hlit - 257) as u32, 5);
    bits.put((hdist - 1) as u32, 5);
    bits.put(15, 4);
    for _ in 0..19 {
        bits.put(5, 3);
    }
    let codes = canonical(&[5u8; 19]);
    for &(symbol, extra) in runs {
        bits.code(codes[symbol as usize], 5);
        match symbol {
            16 => bits.put(extra, 2),
            17 => bits.put(extra, 3),
            18 => bits.put(extra, 7),
            _ => {}
        }
    }
    for &(code, len) in data {
        bits.code(code, len);
    }
    bits.finish()
}

/// 258 lengths (HLIT 257, HDIST 1): symbol 0 → 1, symbols 1–255 → 0
/// (138 + 117 by two 18-runs), symbol 256 → 1, the one distance → 0. The
/// end-of-block code is then `1`.
const EXACT_RUNS: [(u32, u32); 5] = [(1, 0), (18, 127), (18, 106), (1, 0), (0, 0)];

fn repeat_without_previous_refused_with_previous_accepted() {
    // With a previous length: symbol 0 → 3, `16` copies it to symbols 1–3,
    // 252 zeros, symbol 256 → 3, distance → 0. Five three-bit codes; the
    // end-of-block symbol is the fifth, `100`.
    let with_previous = run_coded_block(
        &[(3, 0), (16, 0), (18, 127), (18, 103), (3, 0), (0, 0)],
        257,
        1,
        &[(0b100, 3)],
    );
    assert_eq!(
        inflate::decompress(&with_previous).expect("with previous"),
        b""
    );
    let without = run_coded_block(
        &[(16, 0), (16, 0), (18, 127), (18, 103), (3, 0), (0, 0)],
        257,
        1,
        &[(0b100, 3)],
    );
    assert_eq!(
        inflate::decompress(&without),
        Err(Error::RepeatWithoutPrevious)
    );
}

fn code_lengths_overrun_refused_exact_accepted() {
    let exact = run_coded_block(&EXACT_RUNS, 257, 1, &[(1, 1)]);
    assert_eq!(inflate::decompress(&exact).expect("exact"), b"");
    // The last run (17: three zeros) runs past the one length left.
    let overrun = run_coded_block(
        &[(1, 0), (18, 127), (18, 106), (1, 0), (17, 0)],
        257,
        1,
        &[(1, 1)],
    );
    assert_eq!(
        inflate::decompress(&overrun),
        Err(Error::CodeLengthsOverrun)
    );
}

fn gzip_two_members_both_decoded() {
    let first = text(40_000, 21);
    let second = repeated_token(50_000, 22);
    let framed = [
        gzip::compress(&first, Level::DEFAULT),
        gzip::compress(&second, Level::FASTEST),
    ]
    .concat();
    let decoded = gzip::decompress(&framed).expect("both members");
    assert_eq!(decoded, [first, second].concat());
}

fn gzip_second_member_accepted() {
    let framed = [
        gzip::compress(b"one ", Level::DEFAULT),
        gzip::compress(b"two", Level::NONE),
    ]
    .concat();
    assert_eq!(gzip::decompress(&framed).expect("second"), b"one two");
    let mut decoder = GzipDecoder::new();
    let mut out = [0u8; 64];
    let progress = decoder.feed(&framed, &mut out).expect("feed");
    decoder.finish().expect("boundary");
    assert_eq!(decoder.members(), 2);
    assert_eq!(&out[..progress.written], b"one two");
}

fn gzip_trailing_garbage_refused() {
    let one = gzip::compress(b"member", Level::DEFAULT);
    assert!(gzip::decompress(&one).is_ok());
    for garbage in [&b"\0"[..], b"\0\0\0\0", b"x", b"\x1f\x00", b"\n"] {
        let err = gzip::decompress(&[one.as_slice(), garbage].concat()).expect_err("garbage");
        assert_eq!(
            err,
            Error::TrailingGarbage {
                offset: one.len() as u64
            },
            "{garbage:?}"
        );
    }
    // A lone first magic byte is an unfinished member, also refused.
    assert_eq!(
        gzip::decompress(&[one.as_slice(), b"\x1f"].concat()),
        Err(Error::Truncated)
    );
}

fn empty_member_accepted_empty_input_refused() {
    let empty = gzip::compress(b"", Level::DEFAULT);
    assert_eq!(gzip::decompress(&empty).expect("empty member"), b"");
    let hand = member(0, &[0x03, 0x00], b"");
    assert_eq!(
        gzip::decompress(&hand).expect("hand-built empty member"),
        b""
    );
    assert_eq!(gzip::decompress(b""), Err(Error::Empty));
    assert_eq!(gzip::decompress(b"plain"), Err(Error::NotGzip));
}

/// A `Write` that decodes gzip as it arrives, keeping only a count.
#[cfg(not(target_arch = "wasm32"))]
struct DecodingSink {
    decoder: GzipDecoder,
    buf: Vec<u8>,
    decoded: u64,
    crc: Crc32,
}

#[cfg(not(target_arch = "wasm32"))]
impl Write for DecodingSink {
    fn write(&mut self, mut data: &[u8]) -> std::io::Result<usize> {
        let n = data.len();
        loop {
            let progress = self.decoder.feed(data, &mut self.buf)?;
            self.crc.update(&self.buf[..progress.written]);
            self.decoded += progress.written as u64;
            data = &data[progress.consumed..];
            if progress.status == Status::NeedsInput {
                return Ok(n);
            }
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn gzip_member_over_4gib_isize_wraps_accepted() {
    // 4 GiB + 12345 zero bytes, encoded and decoded in 1 MiB pieces through
    // one pipe: nothing is ever materialised whole.
    const TOTAL: u64 = (1 << 32) + 12_345;
    let chunk = vec![0u8; 1 << 20];
    let sink = DecodingSink {
        decoder: GzipDecoder::new(),
        buf: vec![0; 1 << 20],
        decoded: 0,
        crc: Crc32::new(),
    };
    let mut writer = GzipWriter::new(sink, Level::FASTEST);
    let mut left = TOTAL;
    let mut expected_crc = Crc32::new();
    while left > 0 {
        let n = left.min(chunk.len() as u64) as usize;
        writer.write_all(&chunk[..n]).expect("stream");
        expected_crc.update(&chunk[..n]);
        left -= n as u64;
    }
    let sink = writer.finish().expect("finish");
    sink.decoder
        .finish()
        .expect("an ISIZE compared modulo 2^32 is accepted");
    assert_eq!(sink.decoded, TOTAL);
    assert_eq!(sink.crc.finalize(), expected_crc.finalize());
}

fn decompressed_limit_refuses_bomb() {
    // 256 MiB of zeros compresses to well under 1 MiB.
    let size: u64 = 256 << 20;
    let mut compressed = Vec::new();
    let mut writer = GzipWriter::new(&mut compressed, Level::DEFAULT);
    let chunk = vec![0u8; 1 << 20];
    for _ in 0..(size >> 20) {
        writer.write_all(&chunk).expect("write");
    }
    writer.finish().expect("finish");
    assert!(compressed.len() < 1 << 20);
    let limit = 16 << 20;
    assert_eq!(
        gzip::decompress_with_limit(&compressed, limit),
        Err(Error::LimitExceeded { limit })
    );
    // The same through the Read adapter.
    let err = GzipReader::with_limit(compressed.as_slice(), limit)
        .read_to_end(&mut Vec::new())
        .expect_err("bomb");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    // One byte under the true size refuses; the true size is accepted.
    let small = gzip::compress(&vec![0u8; 100_000], Level::DEFAULT);
    assert!(matches!(
        gzip::decompress_with_limit(&small, 99_999),
        Err(Error::LimitExceeded { .. })
    ));
    assert_eq!(
        gzip::decompress_with_limit(&small, 100_000)
            .expect("exact")
            .len(),
        100_000
    );
}

fn decompressed_limit_accepts_large_valid() {
    let data = repeated_token(24 << 20, 31);
    let framed = gzip::compress(&data, Level::FASTEST);
    let limit = data.len() as u64;
    assert_eq!(
        gzip::decompress_with_limit(&framed, limit).expect("under"),
        data
    );
    // The limit spans members: two copies need twice the room.
    let two = [framed.clone(), framed].concat();
    assert!(matches!(
        gzip::decompress_with_limit(&two, limit),
        Err(Error::LimitExceeded { .. })
    ));
    assert_eq!(
        gzip::decompress_with_limit(&two, 2 * limit)
            .expect("both")
            .len() as u64,
        2 * limit
    );
}

// --- Kernel differentials ----------------------------------------------------

fn copy_kernels_match_portable() {
    let mut rng = Xoshiro256::from_seed(0xC0FF);
    let slack = Backend::COPY_SLACK;
    for backend in Backend::all_available() {
        for dist in 1..=70usize {
            for len in (1..=300usize)
                .step_by(7)
                .chain([1, 2, 3, 15, 16, 17, 31, 32, 33, 258])
            {
                let prefix = 80;
                let mut base = random_bytes(prefix + len + slack, rng.next_u64());
                let mut expected = base.clone();
                for i in prefix..prefix + len {
                    expected[i] = expected[i - dist];
                }
                backend
                    .copy_match(&mut base, prefix, dist, len)
                    .expect("available");
                assert_eq!(
                    &base[..prefix + len],
                    &expected[..prefix + len],
                    "{} dist {dist} len {len}",
                    backend.name()
                );
            }
        }
    }
}

fn match_length_kernels_match_portable() {
    let mut rng = Xoshiro256::from_seed(0x1E);
    for backend in Backend::all_available() {
        for len in 0..=258usize {
            let a = random_bytes(len, rng.next_u64());
            for mismatch in [0, len / 3, len / 2, len.saturating_sub(1), len] {
                let mut b = a.clone();
                if mismatch < len {
                    b[mismatch] ^= 1 << (rng.up_to(7) as u32);
                }
                let expected = Backend::Portable.match_length(&a, &b).expect("portable");
                assert_eq!(expected, mismatch.min(len));
                assert_eq!(
                    backend.match_length(&a, &b),
                    Some(expected),
                    "{} len {len} mismatch {mismatch}",
                    backend.name()
                );
            }
        }
    }
}

fn hash_kernels_match_portable() {
    for backend in Backend::all_available() {
        for len in 4..300usize {
            let data = random_bytes(len, len as u64);
            for start in [0, 1, 5] {
                if start + 4 > len {
                    continue;
                }
                let count = len - start - 3;
                let mut out = vec![0u32; count];
                backend
                    .hash_windows(&data, start, &mut out)
                    .expect("available");
                for (i, &h) in out.iter().enumerate() {
                    let w =
                        u32::from_le_bytes(data[start + i..start + i + 4].try_into().expect("4"));
                    assert_eq!(
                        h,
                        purrdf_deflate::backend::hash4(w),
                        "{} len {len}",
                        backend.name()
                    );
                }
            }
        }
    }
}

fn selected_backend_is_reported() {
    let selected = Backend::selected();
    assert!(selected.is_available());
    purrdf_testkit::harness::print_line(&format!("deflate: selected path {}", selected.name()));
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    assert_eq!(selected, Backend::Simd128);
    #[cfg(all(target_arch = "wasm32", not(target_feature = "simd128")))]
    assert_eq!(selected, Backend::Portable);
}

/// Whether this host is expected to run a kernel path: its architecture and
/// build, and the processor features it advertises independently of the
/// detection under test.
fn expected_here(backend: Backend) -> bool {
    match backend {
        Backend::Portable => true,
        Backend::Sse2 => cfg!(target_arch = "x86_64"),
        Backend::Avx2 => cfg!(target_arch = "x86_64") && host_advertises(&["avx2"]),
        Backend::Neon => cfg!(target_arch = "aarch64"),
        Backend::Simd128 => cfg!(all(target_arch = "wasm32", target_feature = "simd128")),
    }
}

/// Every `deflate` path `PURRDF_REQUIRE_SIMD_PATHS` requires is available, and
/// therefore exercised by every differential above, which iterates all
/// available paths. CI names the paths of the runners whose vector paths no
/// other job executes.
fn required_paths_are_available() {
    assert_required_available("deflate", expected_here);
}

/// Every available path is one of the family's, the selected path among them.
fn the_selected_path_is_available() {
    let available: Vec<Backend> = Backend::all_available().collect();
    assert!(available.contains(&Backend::selected()), "{available:?}");
    assert!(available.contains(&Backend::Portable), "{available:?}");
}

fn gzip_vector_sink_preserves_prefix_and_member_checks() {
    let first = text(65537, 91);
    let second = random_bytes(131_073, 92);
    let mut encoded = gzip::compress(&first, Level::DEFAULT);
    encoded.extend_from_slice(&gzip::compress(&[], Level::DEFAULT));
    encoded.extend_from_slice(&gzip::compress(&second, Level::DEFAULT));
    let mut expected = b"existing prefix".to_vec();
    expected.extend_from_slice(&first);
    expected.extend_from_slice(&second);
    for width in [1, 7, 64, 8192, encoded.len()] {
        let mut decoder = GzipDecoder::new();
        decoder.set_limit((first.len() + second.len()) as u64);
        let mut output = b"existing prefix".to_vec();
        for part in encoded.chunks(width) {
            let before = output.len();
            let progress = decoder
                .feed_to_vec(part, &mut output)
                .expect("valid members");
            assert_eq!(progress.consumed, part.len());
            assert_eq!(progress.written, output.len() - before);
            assert_eq!(progress.status, Status::NeedsInput);
        }
        decoder.finish().expect("all members verified");
        assert_eq!(decoder.members(), 3);
        assert_eq!(output, expected);
        let error = decoder
            .feed_to_vec(b"garbage after members", &mut output)
            .expect_err("trailing garbage");
        assert_eq!(decoder.feed_to_vec(&[], &mut output), Err(error));
        assert_eq!(output, expected);
    }
}

purrdf_testkit::harness_main!(
    gzip_vector_sink_preserves_prefix_and_member_checks,
    required_paths_are_available,
    the_selected_path_is_available,
    round_trips_across_levels_and_window_limits,
    encoder_output_is_independent_of_write_chunking,
    every_kernel_path_encodes_and_decodes_the_same_bytes,
    push_decoding_in_tiny_pieces_matches_one_shot,
    gzip_writer_header_is_deterministic,
    gzip_writer_limits_acceptance_and_recovers_sink_progress,
    gzip_writer_flush_exposes_accepted_plaintext,
    #[cfg(not(target_arch = "wasm32"))]
    gzip_interoperates_with_python_zlib,
    empty_final_stored_block_decodes_to_nothing,
    fifteen_bit_codes_decode,
    distance_32768_with_length_258_decodes,
    gzip_header_with_every_optional_field_decodes,
    tar_vector_gzip_files_decode,
    truncation_refused_complete_accepted,
    bad_crc_refused_in_first_and_second_member,
    bad_isize_refused_good_accepted,
    method_other_than_deflate_refused,
    reserved_flag_bits_refused_clear_accepted,
    header_crc_mismatch_refused_match_accepted,
    block_type_11_refused_10_accepted,
    stored_len_nlen_mismatch_refused_match_accepted,
    distance_beyond_output_refused_within_accepted,
    length_symbols_286_287_refused_285_accepted,
    distance_symbols_30_31_refused_29_accepted,
    oversubscribed_code_refused_complete_accepted,
    single_code_distance_tree_accepted,
    hlit_over_286_refused_286_accepted,
    missing_end_of_block_refused_present_accepted,
    repeat_without_previous_refused_with_previous_accepted,
    code_lengths_overrun_refused_exact_accepted,
    gzip_two_members_both_decoded,
    gzip_second_member_accepted,
    gzip_trailing_garbage_refused,
    empty_member_accepted_empty_input_refused,
    #[cfg(not(target_arch = "wasm32"))]
    gzip_member_over_4gib_isize_wraps_accepted,
    decompressed_limit_refuses_bomb,
    decompressed_limit_accepts_large_valid,
    copy_kernels_match_portable,
    match_length_kernels_match_portable,
    hash_kernels_match_portable,
    selected_backend_is_reported,
);
