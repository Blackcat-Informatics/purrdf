// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Public SHAKE APIs against complete NIST output and independent OpenSSL
//! boundary answers. Frozen answers are inputs to the tests, never regenerated
//! from the implementation under test. The shared vector reader checks their
//! body checksum and record count before replay.

use purrdf_hash::hex::decode;
use purrdf_hash::sha3::Shake;
use purrdf_testkit::vectors::VectorFile;

/// Check one frozen answer through all absorption splits, output splits at
/// lane/rate edges, irregular output calls, defaults and state snapshots.
fn check<const SECURITY: usize>(length: usize, expected: &[u8]) {
    let input = vec![0xa3; length];
    let rate = 200 - SECURITY / 4;
    let mut whole = vec![0; expected.len()];
    Shake::<SECURITY>::digest(&input, &mut whole);
    assert_eq!(whole, expected, "SHAKE{SECURITY}, {length} input bytes");
    Shake::<SECURITY>::digest(&input, &mut []);

    for split in 0..=length {
        let mut shake = Shake::<SECURITY>::default();
        shake.update(&[]);
        shake.update(&input[..split]);
        shake.update(&[]);
        let mut snapshot = shake.clone();
        shake.update(&input[split..]);
        shake.update(&[]);
        let mut actual = vec![0; expected.len()];
        shake.finalize().squeeze(&mut actual);
        assert_eq!(actual, expected, "input split {split}");
        snapshot.update(&input[split..]);
        snapshot.finalize().squeeze(&mut actual);
        assert_eq!(actual, expected, "absorber snapshot at {split}");
    }

    let mut shake = Shake::<SECURITY>::new();
    // Byte-at-a-time input crosses every block boundary independently of the
    // two-piece checks and includes empty calls with a partially filled buffer.
    for byte in &input {
        shake.update(core::slice::from_ref(byte));
        shake.update(&[]);
    }
    let start = shake.finalize();
    let output_splits = [
        0,
        1,
        7,
        8,
        9,
        rate - 1,
        rate,
        rate + 1,
        2 * rate - 1,
        2 * rate,
        2 * rate + 1,
        expected.len() - 1,
        expected.len(),
    ];
    for split in output_splits {
        let mut reader = start.clone();
        let mut actual = vec![0; expected.len()];
        reader.squeeze(&mut []);
        reader.squeeze(&mut actual[..split]);
        reader.squeeze(&mut []);
        let mut snapshot = reader.clone();
        reader.squeeze(&mut actual[split..]);
        assert_eq!(actual, expected, "output split {split}");
        let mut suffix = vec![0; expected.len() - split];
        snapshot.squeeze(&mut []);
        snapshot.squeeze(&mut suffix);
        assert_eq!(suffix, expected[split..], "reader snapshot at {split}");
    }

    for step in [1, 7, 8, 9, 17, rate - 1, rate, rate + 1] {
        let mut reader = start.clone();
        let mut actual = vec![0; expected.len()];
        for chunk in actual.chunks_mut(step) {
            reader.squeeze(&mut []);
            reader.squeeze(chunk);
            reader.squeeze(&mut []);
        }
        assert_eq!(actual, expected, "squeeze chunk size {step}");
    }
}

/// Replay every frozen record through the matching public parameterization.
fn replay(text: &str, count: usize) {
    let file = VectorFile::parse(text).expect("a checksummed frozen SHAKE fixture");
    for record in file.records() {
        let fields = &record.fields;
        assert_eq!(fields.len(), 3, "strength, input length and output");
        let length = fields[1].parse().expect("a decimal input length");
        let expected = decode(fields[2]).expect("a hexadecimal output");
        match fields[0] {
            "128" => check::<128>(length, &expected),
            "256" => check::<256>(length, &expected),
            other => panic!("unexpected SHAKE strength {other}"),
        }
    }
    assert_eq!(
        file.records().len(),
        count,
        "every independent answer replayed"
    );
}

fn complete_nist_512_byte_answers() {
    let text = include_str!("vectors/shake_nist_vectors.txt");
    let file = VectorFile::parse(text).expect("a frozen NIST fixture");
    assert_eq!(file.records().len(), 4);
    for (record, input) in
        file.records()
            .iter()
            .zip([["128", "0"], ["128", "200"], ["256", "0"], ["256", "200"]])
    {
        assert_eq!(record.fields[..2], input);
        assert_eq!(decode(record.fields[2]).unwrap().len(), 512);
    }
    replay(text, 4);
}

fn independent_absorption_boundaries_and_long_output() {
    let text = include_str!("vectors/shake_boundary_vectors.txt");
    let file = VectorFile::parse(text).expect("a frozen independent boundary fixture");
    for (records, strength) in file.records().as_chunks::<8>().0.iter().zip([128, 256]) {
        let rate = 200 - strength / 4;
        for (record, length) in records.iter().zip([
            rate - 1,
            rate,
            rate + 1,
            2 * rate - 1,
            2 * rate,
            2 * rate + 1,
            3 * rate,
            17,
        ]) {
            assert_eq!(record.fields[0], strength.to_string());
            assert_eq!(record.fields[1], length.to_string());
            let output = decode(record.fields[2]).unwrap();
            assert_eq!(
                output.len(),
                if length == 17 { 4097 } else { 3 * rate + 17 }
            );
        }
    }
    replay(text, 16);
}

purrdf_testkit::harness_main!(
    complete_nist_512_byte_answers,
    independent_absorption_boundaries_and_long_output,
);
