// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end pack-execution coverage, driving the BUILT `purrdf` binary
//! (`env!("CARGO_BIN_EXE_purrdf")`) — never the library — so every assertion pins the
//! shipped executable's behavior.
//!
//! These are the correctness tests for the pack-execution contract (the Criterion
//! bench is separate, report-only evidence):
//!
//! * **entailment-over-pack semantic parity** — an entailment closure materialized
//!   over a zero-copy `PackView` is BYTE-IDENTICAL to the same closure over the
//!   equivalent text source (the reasoner produced the same answer whether or not the
//!   pack was rebuilt into an owned dataset).
//! * **deterministic output** — the same pack operation run twice is byte-identical.
//! * **stdin packs** — a pack piped on `-` (the owned-buffer acquisition tier) reasons
//!   identically to the same pack on disk.
//! * **large packs** — a pack materially larger than a page runs end-to-end.
//! * **fail-closed integrity** — a byte-tampered pack is rejected by the explicit
//!   `pack verify` verb AND fails closed on the ordinary read/reason path.
//!
//! The memory-safety guarantee under a hostile concurrent truncate/replace of the
//! source file is proven deterministically by the unit tests in
//! `crate::immutable` (the acquired bytes are bound to the opened descriptor and
//! survive truncation of the source).

use std::fmt::Write as _;
use std::io::Write as _;
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

/// A minimal RDFS-bearing document: `tom a Cat`, `Cat subClassOf Animal`.
const SAMPLE_TTL: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
    "ex:tom a ex:Cat .\n",
    "ex:Cat rdfs:subClassOf ex:Animal .\n",
);

/// A `Command` for the built `purrdf` binary.
fn purrdf() -> Command {
    Command::new(env!("CARGO_BIN_EXE_purrdf"))
}

/// Run `purrdf args`, returning the captured [`Output`].
fn run(args: &[&str]) -> Output {
    purrdf()
        .args(args)
        .output()
        .expect("spawn the built purrdf binary")
}

/// Run `purrdf args` with `stdin_bytes` piped to standard input.
fn run_with_stdin(args: &[&str], stdin_bytes: &[u8]) -> Output {
    let mut child = purrdf()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the built purrdf binary");
    child
        .stdin
        .take()
        .expect("child stdin is piped")
        .write_all(stdin_bytes)
        .expect("write pack bytes to stdin");
    child.wait_with_output().expect("wait for purrdf")
}

/// `dir/name` as an owned UTF-8 path string.
fn path(dir: &TempDir, name: &str) -> String {
    dir.path()
        .join(name)
        .to_str()
        .expect("temp path is valid UTF-8")
        .to_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Write `ttl` to a temp `.ttl` and convert it to a `.purrpck` pack, returning both
/// paths.
fn make_ttl_and_pack(dir: &TempDir, ttl: &str) -> (String, String) {
    let ttl_path = path(dir, "in.ttl");
    std::fs::write(&ttl_path, ttl).expect("write ttl fixture");
    let pack_path = path(dir, "in.purrpck");
    let out = run(&[
        "convert", "--from", "turtle", "--to", "pack", &ttl_path, &pack_path,
    ]);
    assert!(
        out.status.success(),
        "convert ttl -> pack failed: {}",
        stderr(&out)
    );
    (ttl_path, pack_path)
}

#[test]
fn reason_rdfs_over_pack_matches_text_byte_for_byte() {
    let dir = TempDir::new().expect("temp dir");
    let (ttl, pack) = make_ttl_and_pack(&dir, SAMPLE_TTL);

    let from_text = run(&[
        "reason", "--regime", "rdfs", "--from", "turtle", "--to", "nquads", &ttl, "-",
    ]);
    assert!(
        from_text.status.success(),
        "reason over text: {}",
        stderr(&from_text)
    );
    let from_pack = run(&[
        "reason", "--regime", "rdfs", "--from", "pack", "--to", "nquads", &pack, "-",
    ]);
    assert!(
        from_pack.status.success(),
        "reason over pack: {}",
        stderr(&from_pack)
    );

    assert!(
        !from_text.stdout.is_empty(),
        "the rdfs closure is non-empty"
    );
    assert_eq!(
        from_text.stdout, from_pack.stdout,
        "the rdfs closure must be byte-identical over a pack and its equivalent text"
    );
}

#[test]
fn reason_over_pack_is_deterministic() {
    let dir = TempDir::new().expect("temp dir");
    let (_ttl, pack) = make_ttl_and_pack(&dir, SAMPLE_TTL);

    let first = run(&[
        "reason", "--regime", "rdfs", "--from", "pack", "--to", "nquads", &pack, "-",
    ]);
    let second = run(&[
        "reason", "--regime", "rdfs", "--from", "pack", "--to", "nquads", &pack, "-",
    ]);
    assert!(first.status.success() && second.status.success());
    assert_eq!(
        first.stdout, second.stdout,
        "the same pack operation run twice must be byte-identical"
    );
}

#[test]
fn pack_on_stdin_reasons_like_the_same_pack_on_disk() {
    let dir = TempDir::new().expect("temp dir");
    let (_ttl, pack) = make_ttl_and_pack(&dir, SAMPLE_TTL);
    let pack_bytes = std::fs::read(&pack).expect("read pack bytes");

    // The pack arrives on stdin (the owned-buffer acquisition tier), `-` input.
    let from_stdin = run_with_stdin(
        &[
            "reason", "--regime", "rdfs", "--from", "pack", "--to", "nquads", "-", "-",
        ],
        &pack_bytes,
    );
    assert!(
        from_stdin.status.success(),
        "reason over stdin pack: {}",
        stderr(&from_stdin)
    );

    let from_disk = run(&[
        "reason", "--regime", "rdfs", "--from", "pack", "--to", "nquads", &pack, "-",
    ]);
    assert!(from_disk.status.success());
    assert_eq!(
        from_stdin.stdout, from_disk.stdout,
        "a pack on stdin must reason identically to the same pack on disk"
    );
}

#[test]
fn large_pack_reasons_end_to_end() {
    let dir = TempDir::new().expect("temp dir");
    let mut ttl = String::from("@prefix ex: <http://example.org/> .\n");
    for i in 0..20_000 {
        writeln!(ttl, "ex:s{i} ex:p ex:o{i} .").expect("write triple to fixture string");
    }
    let (_ttl, pack) = make_ttl_and_pack(&dir, &ttl);

    let len = std::fs::metadata(&pack).expect("pack metadata").len();
    assert!(len > 4096, "the pack must exceed a page (got {len} bytes)");

    // `simple` is the identity closure, so every input triple appears once.
    let out = run(&[
        "reason", "--regime", "simple", "--from", "pack", "--to", "nquads", &pack, "-",
    ]);
    assert!(
        out.status.success(),
        "reason over large pack: {}",
        stderr(&out)
    );
    let lines = out
        .stdout
        .split(|&b| b == b'\n')
        .filter(|line| !line.is_empty())
        .count();
    assert_eq!(
        lines, 20_000,
        "the simple closure over the large pack carries every triple"
    );
}

#[test]
fn tampered_pack_is_rejected_by_verify_and_fails_closed_on_read() {
    let dir = TempDir::new().expect("temp dir");
    let (_ttl, pack) = make_ttl_and_pack(&dir, SAMPLE_TTL);

    // A good pack verifies and prints its 64-hex canonical digest.
    let good = run(&["pack", "verify", &pack]);
    assert!(good.status.success(), "good pack verify: {}", stderr(&good));
    let digest = String::from_utf8_lossy(&good.stdout).trim().to_owned();
    assert_eq!(
        digest.len(),
        64,
        "the printed digest is 64 hex chars: {digest:?}"
    );
    assert!(digest.bytes().all(|b| b.is_ascii_hexdigit()));

    // Flip a byte mid-pack.
    let mut bytes = std::fs::read(&pack).expect("read pack");
    let mid = bytes.len() / 2;
    bytes[mid] ^= 0xFF;
    let bad = path(&dir, "bad.purrpck");
    std::fs::write(&bad, &bytes).expect("write tampered pack");

    // The explicit verb rejects it...
    let verify_bad = run(&["pack", "verify", &bad]);
    assert!(
        !verify_bad.status.success(),
        "tampered pack must fail `pack verify`"
    );
    // ...and the ordinary read/reason path fails closed too (integrity is unconditional
    // on every open, not only in the verb).
    let reason_bad = run(&[
        "reason", "--regime", "simple", "--from", "pack", "--to", "nquads", &bad, "-",
    ]);
    assert!(
        !reason_bad.status.success(),
        "a tampered pack must fail closed on the ordinary read path"
    );
}
