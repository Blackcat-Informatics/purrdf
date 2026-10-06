// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// One-time import of independent output bytes; never calls the implementation under test.
use std::{fmt::Write, io::Write as IoWrite, process::{Command, Stdio}};
fn command(args: &[&str], input: &[u8]) -> Vec<u8> {
    let mut child = Command::new("openssl").args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success()); output.stdout
}
fn hex(bytes: &[u8]) -> String {
    let mut out = String::new();
    for byte in bytes { write!(&mut out, "{byte:02x}").unwrap(); }
    out
}
fn write_file(name: &str, oracle: &str, body: &str, count: usize) {
    let sha = String::from_utf8(command(&["dgst", "-sha256"], body.as_bytes())).unwrap();
    let sha = sha.split_whitespace().last().unwrap();
    let file = format!("# Frozen SHAKE byte answers. See shake-PROVENANCE.md.\n# Inputs: length copies of byte A3. Fields: security (bits), length (bytes), output (hex).\n# oracle: {oracle}\n# vector-count: {count}\n# body-sha256: {sha}\n{body}");
    std::fs::write(format!("crates/hash-conformance/tests/vectors/{name}"), file).unwrap();
}
fn main() {
    let stage = ".stage/purrdf-gts-composite-ml-dsa-65-ed25519/raw";
    let mut nist = String::new();
    let mut boundary = String::new();
    for strength in [128, 256] {
        for bits in [0, 1600] {
            let text = std::fs::read_to_string(format!("{stage}/shake{strength}_msg{bits}.txt")).unwrap();
            let output = text.rsplit_once("Output val is").unwrap().1;
            let bytes: Vec<u8> = output.split_whitespace().map(|value| u8::from_str_radix(value, 16).unwrap()).collect();
            assert_eq!(bytes.len(), 512);
            writeln!(&mut nist, "{strength}\t{}\t{}", bits / 8, hex(&bytes)).unwrap();
        }
        let rate = 200 - strength / 4;
        for length in [rate - 1, rate, rate + 1, 2 * rate - 1, 2 * rate, 2 * rate + 1, 3 * rate] {
            let algorithm = format!("-shake{strength}");
            let out_len = (3 * rate + 17).to_string();
            let bytes = command(&["dgst", &algorithm, "-xoflen", &out_len, "-binary"], &vec![0xa3; length]);
            assert_eq!(bytes.len(), 3 * rate + 17);
            writeln!(&mut boundary, "{strength}\t{length}\t{}", hex(&bytes)).unwrap();
        }
        let algorithm = format!("-shake{strength}");
        let bytes = command(&["dgst", &algorithm, "-xoflen", "4097", "-binary"], &[0xa3; 17]);
        assert_eq!(bytes.len(), 4097);
        writeln!(&mut boundary, "{strength}\t17\t{}", hex(&bytes)).unwrap();
    }
    write_file("shake_nist_vectors.txt", "NIST FIPS 202 example values, all 512 bytes", &nist, 4);
    write_file("shake_boundary_vectors.txt", "OpenSSL 3.6.4, public dgst SHAKE interface", &boundary, 16);
}
