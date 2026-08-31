// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end coverage of `convert`'s INPUT ADMISSION contract: GTS bundles, an ordered
//! multi-source list merged by the deterministic dataset union, and gzip / zstd transport
//! on both files and stdin. Drives the real binary via `CARGO_BIN_EXE_purrdf`.
//!
//! Three properties are load-bearing throughout and each has its own test rather than
//! being assumed by the others:
//!
//! * a GTS file is read through the AUTHORITATIVE importer, so the same blank-node label
//!   in two segments stays two DISTINCT nodes, and the envelope material the RDF dataset
//!   cannot carry is reported with the counts the importer actually produced;
//! * a multi-source union is order-independent, collapses duplicate ground quads, and
//!   keeps each source's blank nodes in its own scope — while the SINGLE-source path is
//!   left byte-identical, because the union re-scopes blank nodes even for one input;
//! * a truncated or mis-declared transport stream fails closed, with no partial output
//!   file left behind.

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use purrdf_gts::model::{Term, TermKind};
use purrdf_gts::writer::Writer;

/// A default-graph N-Triples fixture. Shares one quad with [`SEED_RIGHT`] (so the union
/// has something to deduplicate) and carries a blank node labelled `_:b0` (so the
/// standardize-apart discipline has something to keep apart).
const SEED_LEFT: &str = concat!(
    "<http://example.org/s> <http://example.org/p> <http://example.org/shared> .\n",
    "<http://example.org/left> <http://example.org/p> \"left\" .\n",
    "<http://example.org/left> <http://example.org/rel> _:b0 .\n",
    "_:b0 <http://example.org/kind> \"left-node\" .\n",
);

/// The second fixture: the SAME shared quad, its own quad, and its own `_:b0`.
const SEED_RIGHT: &str = concat!(
    "<http://example.org/s> <http://example.org/p> <http://example.org/shared> .\n",
    "<http://example.org/right> <http://example.org/p> \"right\" .\n",
    "<http://example.org/right> <http://example.org/rel> _:b0 .\n",
    "_:b0 <http://example.org/kind> \"right-node\" .\n",
);

/// A third fixture in a DIFFERENT syntax, so a mixed-format list is exercised.
const SEED_THIRD_TRIG: &str = concat!(
    "<http://example.org/third> <http://example.org/p> \"third\" .\n",
    "<http://example.org/g> { <http://example.org/gs> <http://example.org/gp> \"g\" . }\n",
);

/// A `Command` for the built `purrdf` binary.
fn purrdf() -> Command {
    Command::new(env!("CARGO_BIN_EXE_purrdf"))
}

/// Run `purrdf` with `args`, returning the captured [`Output`].
fn run(args: &[&str]) -> Output {
    purrdf()
        .args(args)
        .output()
        .expect("spawn the built purrdf binary")
}

/// Run `purrdf args` with `stdin_bytes` piped to standard input.
///
/// The stdin writer runs on its own thread so the parent can drain stdout/stderr
/// concurrently. Writing a whole compressed payload inline before reading the child's
/// output would deadlock the moment the child's output fills the OS pipe buffer — which
/// is invisible with a tiny fixture and real with a large one.
fn run_with_stdin(args: &[&str], stdin_bytes: &[u8]) -> Output {
    let mut child = purrdf()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the built purrdf binary");
    let mut stdin = child.stdin.take().expect("child stdin is piped");
    let bytes = stdin_bytes.to_vec();
    let writer = std::thread::spawn(move || {
        // A broken pipe (the child rejected the input before draining stdin) is not a
        // test failure: the child's exit status carries the verdict.
        let _ = stdin.write_all(&bytes);
    });
    let output = child.wait_with_output().expect("wait for purrdf");
    writer.join().expect("join the stdin writer thread");
    output
}

/// stderr of an [`Output`] as a `String`, for diagnostics + refusal assertions.
fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Join a name onto `dir`, returning it as an owned `String`.
fn path(dir: &Path, name: &str) -> String {
    dir.join(name)
        .to_str()
        .expect("temp path is valid UTF-8")
        .to_owned()
}

/// Write `contents` to `dir/name`, returning the path.
fn write_file(dir: &Path, name: &str, contents: &str) -> String {
    let p = path(dir, name);
    std::fs::write(&p, contents).expect("write fixture file");
    p
}

/// Write raw `bytes` to `dir/name`, returning the path.
fn write_bytes(dir: &Path, name: &str, bytes: &[u8]) -> String {
    let p = path(dir, name);
    std::fs::write(&p, bytes).expect("write fixture bytes");
    p
}

/// The RDFC-1.0 canonical N-Quads document of a source list, produced BY THE BINARY.
/// Byte-equality of two such documents is an isomorphism test.
fn canonical_of(dir: &Path, args: &[&str]) -> Vec<u8> {
    let out = path(dir, "canonical.scratch.nq");
    let mut full: Vec<&str> = vec!["convert", "--canonical"];
    full.extend_from_slice(args);
    full.push(&out);
    let o = run(&full);
    assert!(o.status.success(), "canonicalizing failed: {}", stderr(&o));
    std::fs::read(&out).expect("read canonical scratch output")
}

/// gzip-frame `payload`.
fn gzip(payload: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(payload).expect("gzip write");
    encoder.finish().expect("gzip finish")
}

/// zstd-frame `payload`.
fn zstd(payload: &[u8]) -> Vec<u8> {
    structured_zstd::encoding::compress_to_vec(
        payload,
        structured_zstd::encoding::CompressionLevel::Fastest,
    )
}

// --------------------------------------------------------------------------------
// GTS fixtures
// --------------------------------------------------------------------------------

/// A GTS IRI term.
fn gts_iri(value: &str) -> Term {
    Term {
        kind: TermKind::Iri,
        value: Some(value.to_owned()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
        triple: None,
    }
}

/// A GTS blank-node term.
fn gts_blank(label: &str) -> Term {
    Term {
        kind: TermKind::Bnode,
        value: Some(label.to_owned()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
        triple: None,
    }
}

/// One GTS segment carrying `(:s :p object)`, `(_:label :kind object)`, and one inline
/// blob. Both segments of [`two_segment_gts`] use the SAME blank label deliberately.
fn gts_segment(object: &str, blank_label: &str, blob: &[u8]) -> Vec<u8> {
    let mut writer = Writer::new("purrdf.gts");
    writer.add_terms(&[
        gts_iri("http://example.org/s"),
        gts_iri("http://example.org/p"),
        gts_iri(object),
        gts_blank(blank_label),
        gts_iri("http://example.org/kind"),
    ]);
    writer.add_quads(&[(0, 1, 2, None), (3, 4, 2, None)]);
    writer.add_blob(blob, Some("text/plain"), None);
    writer.into_bytes()
}

/// A two-segment GTS file: a CBOR sequence of two independent segments, each with its own
/// blob and each labelling its blank node `_:b`.
fn two_segment_gts() -> Vec<u8> {
    let mut bytes = gts_segment("http://example.org/o1", "b", b"first-payload");
    bytes.extend_from_slice(&gts_segment(
        "http://example.org/o2",
        "b",
        b"second-payload",
    ));
    bytes
}

// --------------------------------------------------------------------------------
// (a) GTS input
// --------------------------------------------------------------------------------

/// A `.gts` source converts through the authoritative importer, with the format inferred
/// from the extension alone.
#[test]
fn gts_input_is_admitted_and_inferred_from_its_extension() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let gts = write_bytes(dir, "in.gts", &two_segment_gts());
    let out = path(dir, "out.nq");

    let o = run(&["convert", &gts, &out]);
    assert!(o.status.success(), "gts convert failed: {}", stderr(&o));

    let text = std::fs::read_to_string(&out).expect("read output");
    assert!(text.contains("<http://example.org/o1>"), "{text}");
    assert!(text.contains("<http://example.org/o2>"), "{text}");
    assert_eq!(
        text.lines().filter(|l| !l.trim().is_empty()).count(),
        4,
        "both segments' quads must survive: {text}"
    );
}

/// THE CORRECTNESS PROPERTY OF THE AUTHORITATIVE IMPORTER: the same blank-node label in
/// two different segments names two DIFFERENT nodes, and both survive as distinct labels.
///
/// `import_gts_graph` folds every segment into one term table and would collapse these
/// two into one node; this is the test that says the CLI does not use it.
#[test]
fn gts_per_segment_blank_node_scope_is_preserved() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let gts = write_bytes(dir, "in.gts", &two_segment_gts());
    let out = path(dir, "out.nq");

    let o = run(&["convert", &gts, &out]);
    assert!(o.status.success(), "gts convert failed: {}", stderr(&o));

    let text = std::fs::read_to_string(&out).expect("read output");
    let mut labels: Vec<&str> = text
        .lines()
        .filter_map(|line| line.strip_prefix("_:"))
        .filter_map(|rest| rest.split_whitespace().next())
        .collect();
    labels.sort_unstable();
    labels.dedup();
    assert_eq!(
        labels.len(),
        2,
        "the same `_:b` in two segments must stay two distinct nodes: {text}"
    );
}

/// The GTS envelope is REPORTED, with counts read off the bundle the importer returned.
///
/// The two segment records and the two blob references are material a GTS file carries
/// beside its hot graph and an RDF dataset has no place for; each is a ledger entry, and
/// the segment entry additionally carries the segment head ids as provenance. The
/// `bnode-scope-flatten` code is NOT among them, because that loss did not occur.
#[test]
fn gts_envelope_is_surfaced_in_the_loss_ledger_with_exact_counts() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let gts = write_bytes(dir, "in.gts", &two_segment_gts());
    let out = path(dir, "out.nq");
    let ledger_path = path(dir, "ledger.json");
    let ledger_flag = format!("--loss-ledger={ledger_path}");

    let o = run(&["convert", &ledger_flag, &gts, &out]);
    assert!(o.status.success(), "gts convert failed: {}", stderr(&o));

    let ledger = std::fs::read_to_string(&ledger_path).expect("read ledger");
    assert!(
        ledger.contains("gts-segment-ledger-dropped"),
        "the segment ledger must be reported: {ledger}"
    );
    assert!(
        ledger.contains("2 segment record(s)"),
        "the segment count must be the one the importer produced: {ledger}"
    );
    assert!(
        ledger.contains("Segment head id(s), in segment order:"),
        "the segment head ids are the provenance the envelope carried: {ledger}"
    );
    assert!(
        ledger.contains("gts-blob-references-dropped") && ledger.contains("2 content-addressed"),
        "both blob references must be reported: {ledger}"
    );
    // Material the file did NOT carry is not claimed.
    for absent in [
        "gts-metadata-dropped",
        "gts-sidecar-resources-dropped",
        "gts-suppressions-dropped",
        "gts-opaque-nodes-dropped",
        "gts-signature-records-dropped",
    ] {
        assert!(
            !ledger.contains(absent),
            "an empty envelope table must not claim a loss (`{absent}`): {ledger}"
        );
    }
    // The scope-flattening loss belongs to the OTHER importer and did not happen here.
    assert!(
        !ledger.contains("bnode-scope-flatten"),
        "the event importer preserves scope; claiming otherwise would be a false loss: {ledger}"
    );
}

/// GTS is an INPUT container: named as a `--to` target it is refused BY NAME, with the
/// reason, rather than accepted and written as something else.
#[test]
fn gts_is_refused_as_an_output_target_by_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "seed.nt", SEED_LEFT);
    let out = path(dir, "out.gts");

    for args in [
        vec!["convert", "--from", "ntriples", "--to", "gts"],
        vec!["convert", "--from", "ntriples"],
    ] {
        let mut full = args;
        full.push(&seed);
        full.push(&out);
        let o = run(&full);
        assert!(!o.status.success(), "a GTS target must be refused");
        assert_eq!(o.status.code(), Some(2), "usage errors exit 2");
        assert!(
            stderr(&o).contains("GTS"),
            "the refusal must name GTS: {}",
            stderr(&o)
        );
        assert!(
            !Path::new(&out).exists(),
            "a refused target must leave no file behind"
        );
    }
}

/// A corrupt GTS source fails closed: the authoritative importer hard-fails on a reader
/// diagnostic, and no partial output is written.
#[test]
fn corrupt_gts_input_fails_closed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let mut damaged = two_segment_gts();
    // Corrupt the middle of the first segment's frame chain.
    let midpoint = damaged.len() / 4;
    for byte in &mut damaged[midpoint..midpoint + 16] {
        *byte ^= 0xff;
    }
    let gts = write_bytes(dir, "bad.gts", &damaged);
    let out = path(dir, "out.nq");

    let o = run(&["convert", &gts, &out]);
    assert!(!o.status.success(), "a corrupt GTS file must fail closed");
    assert!(!stderr(&o).is_empty(), "the failure must be diagnosed");
    assert!(
        !Path::new(&out).exists(),
        "a failed read must not leave a partial output file"
    );
}

/// A GTS bundle arrives on STDIN under `--from gts`, and arrives GZIPPED on stdin too:
/// the two admissions compose, with the transport decided by the leading bytes and the
/// container by the explicit `--from` stdin has always required.
#[test]
fn a_gts_bundle_is_admitted_on_stdin_plain_and_compressed() {
    let raw = two_segment_gts();
    for bytes in [raw.clone(), gzip(&raw)] {
        let out = run_with_stdin(
            &["convert", "--from", "gts", "--to", "nquads", "-", "-"],
            &bytes,
        );
        assert!(
            out.status.success(),
            "gts on stdin failed: {}",
            stderr(&out)
        );
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(
            text.contains("<http://example.org/o1>") && text.contains("<http://example.org/o2>"),
            "{text}"
        );
    }
}

/// `--base` against a GTS source is refused by name, exactly as it is for a pack: both
/// containers store fully-resolved terms and have no relative-IRI syntax.
#[test]
fn base_with_a_gts_source_is_refused_by_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let gts = write_bytes(dir, "in.gts", &two_segment_gts());
    let out = path(dir, "out.nq");

    let o = run(&["convert", "--base", "http://example.org/base/", &gts, &out]);
    assert!(!o.status.success(), "--base with a GTS source is refused");
    assert_eq!(o.status.code(), Some(2), "usage errors exit 2");
    assert!(stderr(&o).contains("--base"), "{}", stderr(&o));
    assert!(stderr(&o).contains("gts"), "{}", stderr(&o));
}

// --------------------------------------------------------------------------------
// (b) Multi-source union
// --------------------------------------------------------------------------------

/// Two sources merge, sharing quads deduplicate, and each source's blank nodes stay in
/// its own scope.
#[test]
fn two_sources_merge_with_dedup_and_per_source_blank_scopes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let left = write_file(dir, "left.nt", SEED_LEFT);
    let right = write_file(dir, "right.nt", SEED_RIGHT);
    let out = path(dir, "merged.nq");

    let o = run(&["convert", "--input", &right, &left, &out]);
    assert!(o.status.success(), "union convert failed: {}", stderr(&o));

    let text = std::fs::read_to_string(&out).expect("read merged output");
    // 4 + 4 source quads, minus the ONE ground quad both sources assert.
    assert_eq!(
        text.lines().filter(|l| !l.trim().is_empty()).count(),
        7,
        "duplicate ground quads collapse once: {text}"
    );
    assert_eq!(
        text.matches("<http://example.org/shared>").count(),
        1,
        "the shared quad appears exactly once: {text}"
    );
    assert!(text.contains("\"left\""), "{text}");
    assert!(text.contains("\"right\""), "{text}");

    // Both sources labelled their blank node `_:b0`; standardize-apart keeps them apart.
    let mut labels: Vec<&str> = text
        .lines()
        .filter_map(|line| line.strip_prefix("_:"))
        .filter_map(|rest| rest.split_whitespace().next())
        .collect();
    labels.sort_unstable();
    labels.dedup();
    assert_eq!(
        labels.len(),
        2,
        "each source's `_:b0` must stay its own node: {text}"
    );
    assert!(
        text.contains("\"left-node\"") && text.contains("\"right-node\""),
        "{text}"
    );
}

/// The union is DETERMINISTIC and ORDER-INDEPENDENT: the same list run twice is
/// byte-identical, and the two orderings canonicalize to the same document.
#[test]
fn the_union_is_deterministic_and_order_independent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let left = write_file(dir, "left.nt", SEED_LEFT);
    let right = write_file(dir, "right.nt", SEED_RIGHT);

    let a = path(dir, "a.nq");
    let b = path(dir, "b.nq");
    for out in [&a, &b] {
        let o = run(&["convert", "--input", &right, &left, out]);
        assert!(o.status.success(), "union failed: {}", stderr(&o));
    }
    assert_eq!(
        std::fs::read(&a).expect("a"),
        std::fs::read(&b).expect("b"),
        "the same merge run twice must be byte-identical"
    );

    assert_eq!(
        canonical_of(dir, &["--input", &right, &left]),
        canonical_of(dir, &["--input", &left, &right]),
        "the merge must canonicalize identically regardless of source order"
    );
}

/// A mixed-format, mixed-container list: Turtle + N-Triples + a pack + a GTS file, all
/// classified from their own extensions, all merged.
#[test]
fn a_mixed_format_source_list_is_admitted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let left = write_file(dir, "left.nt", SEED_LEFT);
    let third = write_file(dir, "third.trig", SEED_THIRD_TRIG);
    let gts = write_bytes(dir, "in.gts", &two_segment_gts());

    // Seed a pack from `left`, then merge a DIFFERENT text source with it.
    let pack = path(dir, "right.purrpck");
    let o = run(&[
        "convert", "--from", "ntriples", "--to", "pack", &left, &pack,
    ]);
    assert!(
        o.status.success(),
        "seeding the pack failed: {}",
        stderr(&o)
    );

    let out = path(dir, "merged.nq");
    let o = run(&["convert", "--input", &pack, "--input", &gts, &third, &out]);
    assert!(
        o.status.success(),
        "mixed-format union failed: {}",
        stderr(&o)
    );

    let text = std::fs::read_to_string(&out).expect("read merged output");
    assert!(text.contains("\"third\""), "trig source present: {text}");
    assert!(
        text.contains("<http://example.org/g>"),
        "the named graph survives the merge: {text}"
    );
    assert!(text.contains("\"left\""), "pack source present: {text}");
    assert!(
        text.contains("<http://example.org/o1>") && text.contains("<http://example.org/o2>"),
        "both GTS segments present: {text}"
    );
}

/// A GTS source inside a MULTI-source list still reports its envelope.
#[test]
fn a_gts_source_in_a_list_still_reports_its_envelope() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let left = write_file(dir, "left.nt", SEED_LEFT);
    let gts = write_bytes(dir, "in.gts", &two_segment_gts());
    let out = path(dir, "merged.nq");
    let ledger_path = path(dir, "ledger.json");
    let ledger_flag = format!("--loss-ledger={ledger_path}");

    let o = run(&["convert", &ledger_flag, "--input", &gts, &left, &out]);
    assert!(o.status.success(), "union failed: {}", stderr(&o));
    let ledger = std::fs::read_to_string(&ledger_path).expect("read ledger");
    assert!(
        ledger.contains("gts-segment-ledger-dropped"),
        "a GTS source in a list reports its envelope too: {ledger}"
    );
}

/// THE SINGLE-SOURCE PATH IS UNTOUCHED. `RdfDataset::union` re-scopes blank nodes even
/// for one input, so a one-source convert must never enter the merge lane: the blank
/// label the source wrote is the blank label the output carries.
#[test]
fn a_single_source_convert_is_not_routed_through_the_union() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "left.nt", SEED_LEFT);
    let out = path(dir, "out.nq");

    let o = run(&[
        "convert", "--from", "ntriples", "--to", "nquads", &seed, &out,
    ]);
    assert!(o.status.success(), "single-source convert: {}", stderr(&o));
    let text = std::fs::read_to_string(&out).expect("read output");
    assert!(
        text.contains("_:b0 "),
        "the source's own blank label survives an ordinary conversion: {text}"
    );
}

/// `--from` applies to EVERY source in the list, not only the first: it is one flag, and
/// a per-source override would need a per-source flag.
#[test]
fn an_explicit_from_applies_to_every_source_in_the_list() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    // Both files hold N-Triples under a misleading extension; `--from` decides both.
    let left = write_file(dir, "left.data", SEED_LEFT);
    let right = write_file(dir, "right.data", SEED_RIGHT);
    let out = path(dir, "merged.nq");

    let o = run(&[
        "convert", "--from", "ntriples", "--to", "nquads", "--input", &right, &left, &out,
    ]);
    assert!(
        o.status.success(),
        "one --from must cover the whole list: {}",
        stderr(&o)
    );
    let text = std::fs::read_to_string(&out).expect("read merged output");
    assert!(
        text.contains("\"left\"") && text.contains("\"right\""),
        "{text}"
    );
}

/// `-` names standard input, which can be consumed once: naming it twice is refused
/// rather than silently reading an empty second source.
#[test]
fn stdin_named_twice_in_a_source_list_is_refused() {
    let o = run(&[
        "convert", "--from", "ntriples", "--to", "nquads", "--input", "-", "-", "-",
    ]);
    assert!(!o.status.success(), "`-` twice must be refused");
    assert_eq!(o.status.code(), Some(2), "usage errors exit 2");
    assert!(
        stderr(&o).contains("exactly once"),
        "the refusal must say why: {}",
        stderr(&o)
    );
}

/// Every source in a list is resolved BEFORE any is read: an unreadable third entry
/// fails the run rather than writing a partial merge of the first two.
#[test]
fn an_unclassifiable_source_in_a_list_fails_before_anything_is_written() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let left = write_file(dir, "left.nt", SEED_LEFT);
    let mystery = write_file(dir, "mystery.unknown", SEED_RIGHT);
    let out = path(dir, "merged.nq");

    let o = run(&["convert", "--input", &mystery, &left, &out]);
    assert!(!o.status.success(), "an unclassifiable source must fail");
    assert_eq!(o.status.code(), Some(2), "usage errors exit 2");
    assert!(
        !Path::new(&out).exists(),
        "nothing may be written when a later source cannot be classified"
    );
}

// --------------------------------------------------------------------------------
// (d) gzip / zstd transport
// --------------------------------------------------------------------------------

/// A gzip-wrapped file is detected, decoded, and its PAYLOAD extension classified —
/// `left.nt.gz` is N-Triples, not an unknown `gz` format.
#[test]
fn a_gzip_file_is_detected_decoded_and_classified_by_its_payload_extension() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let gz = write_bytes(dir, "left.nt.gz", &gzip(SEED_LEFT.as_bytes()));
    let out = path(dir, "out.nq");

    let o = run(&["convert", "--to", "nquads", &gz, &out]);
    assert!(o.status.success(), "gzip convert failed: {}", stderr(&o));
    let text = std::fs::read_to_string(&out).expect("read output");
    assert!(text.contains("\"left\""), "{text}");
}

/// The same for zstd.
#[test]
fn a_zstd_file_is_detected_decoded_and_classified_by_its_payload_extension() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let zst = write_bytes(dir, "left.nt.zst", &zstd(SEED_LEFT.as_bytes()));
    let out = path(dir, "out.nq");

    let o = run(&["convert", "--to", "nquads", &zst, &out]);
    assert!(o.status.success(), "zstd convert failed: {}", stderr(&o));
    let text = std::fs::read_to_string(&out).expect("read output");
    assert!(text.contains("\"left\""), "{text}");
}

/// Compressed STDIN is decoded from its leading bytes alone — stdin has no filename to
/// consult, so the magic-byte sniff is the whole decision.
#[test]
fn compressed_stdin_is_detected_from_its_leading_bytes() {
    for framed in [gzip(SEED_LEFT.as_bytes()), zstd(SEED_LEFT.as_bytes())] {
        let out = run_with_stdin(
            &["convert", "--from", "ntriples", "--to", "nquads", "-", "-"],
            &framed,
        );
        assert!(
            out.status.success(),
            "compressed stdin failed: {}",
            stderr(&out)
        );
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(text.contains("\"left\""), "{text}");
    }
}

/// A TRUNCATED compressed stream hard-fails with NO partial success: the decoder is
/// drained to completion, so the prefix it managed to inflate never reaches the parser
/// and no output file is written.
#[test]
fn a_truncated_compressed_stream_fails_closed_without_partial_output() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();

    for (name, framed) in [
        ("left.nt.gz", gzip(SEED_LEFT.as_bytes())),
        ("left.nt.zst", zstd(SEED_LEFT.as_bytes())),
    ] {
        let truncated = &framed[..framed.len() / 2];
        let input = write_bytes(dir, name, truncated);
        let out = path(dir, "out.nq");
        let _ = std::fs::remove_file(&out);

        let o = run(&["convert", "--to", "nquads", &input, &out]);
        assert!(
            !o.status.success(),
            "a truncated {name} must fail: {}",
            stderr(&o)
        );
        assert!(
            !Path::new(&out).exists(),
            "a truncated {name} must leave no partial output"
        );
    }
}

/// A stream whose bytes are garbage inside an intact-looking wrapper also fails closed.
#[test]
fn a_corrupt_compressed_stream_fails_closed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let mut framed = gzip(SEED_LEFT.as_bytes());
    let midpoint = framed.len() / 2;
    framed[midpoint] ^= 0xff;
    let input = write_bytes(dir, "left.nt.gz", &framed);
    let out = path(dir, "out.nq");

    let o = run(&["convert", "--to", "nquads", &input, &out]);
    assert!(!o.status.success(), "a corrupt gzip stream must fail");
    assert!(!Path::new(&out).exists(), "no partial output");
}

/// `--transport` explicitly SELECTS the encoding: naming the wrong one hard-fails rather
/// than silently falling back to the sniff, and `none` reads the bytes verbatim.
#[test]
fn transport_is_explicitly_selectable_and_a_mismatch_hard_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let gz = write_bytes(dir, "left.nt.gz", &gzip(SEED_LEFT.as_bytes()));
    let plain = write_file(dir, "plain.nt", SEED_LEFT);
    let out = path(dir, "out.nq");

    // The right encoding, named explicitly, works.
    let o = run(&[
        "convert",
        "--transport",
        "gzip",
        "--to",
        "nquads",
        &gz,
        &out,
    ]);
    assert!(o.status.success(), "explicit gzip failed: {}", stderr(&o));

    // The wrong encoding, named explicitly, fails rather than sniffing its way out.
    let _ = std::fs::remove_file(&out);
    let o = run(&[
        "convert",
        "--transport",
        "zstd",
        "--to",
        "nquads",
        &gz,
        &out,
    ]);
    assert!(!o.status.success(), "a declared-zstd gzip stream must fail");
    assert!(!Path::new(&out).exists(), "no partial output");

    // An encoding named over a stream that carries none also fails.
    let _ = std::fs::remove_file(&out);
    let o = run(&[
        "convert",
        "--transport",
        "gzip",
        "--from",
        "ntriples",
        "--to",
        "nquads",
        &plain,
        &out,
    ]);
    assert!(
        !o.status.success(),
        "a declared-gzip plain stream must fail"
    );

    // `none` reads the bytes verbatim: the gzip frame is then not valid N-Triples.
    let _ = std::fs::remove_file(&out);
    let o = run(&[
        "convert",
        "--transport",
        "none",
        "--from",
        "ntriples",
        "--to",
        "nquads",
        &gz,
        &out,
    ]);
    assert!(
        !o.status.success(),
        "`--transport none` must not decode a wrapped stream"
    );
}

/// A compressed source inside a MIXED multi-source list is decoded like any other.
#[test]
fn a_compressed_source_in_a_mixed_list_is_decoded() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let plain = write_file(dir, "left.nt", SEED_LEFT);
    let gz = write_bytes(dir, "right.nt.gz", &gzip(SEED_RIGHT.as_bytes()));
    let out = path(dir, "merged.nq");

    let o = run(&["convert", "--input", &gz, &plain, &out]);
    assert!(o.status.success(), "mixed list failed: {}", stderr(&o));
    let text = std::fs::read_to_string(&out).expect("read output");
    assert!(
        text.contains("\"left\"") && text.contains("\"right\""),
        "{text}"
    );
}

/// A transport-suffixed OUTPUT name is refused BY NAME: this pipeline decodes on input
/// and never compresses on output, so writing plain bytes to `out.nt.gz` would be a file
/// whose name lies about its contents.
#[test]
fn a_transport_suffixed_output_is_refused_by_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "left.nt", SEED_LEFT);

    for name in ["out.nq.gz", "out.nq.zst"] {
        let out = path(dir, name);
        let o = run(&[
            "convert", "--from", "ntriples", "--to", "nquads", &seed, &out,
        ]);
        assert!(!o.status.success(), "`{name}` must be refused");
        assert_eq!(o.status.code(), Some(2), "usage errors exit 2");
        assert!(
            !Path::new(&out).exists(),
            "a refused target must leave no file behind"
        );
    }
}

/// A transport-wrapped PACK is refused by name rather than handed to the integrity
/// verifier as garbage: a pack is acquired immutably and verified in place, and decoding
/// it into a fresh buffer would discard exactly that guarantee.
#[test]
fn a_compressed_pack_source_is_refused_by_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "left.nt", SEED_LEFT);
    let pack = path(dir, "in.purrpck");
    let o = run(&[
        "convert", "--from", "ntriples", "--to", "pack", &seed, &pack,
    ]);
    assert!(
        o.status.success(),
        "seeding the pack failed: {}",
        stderr(&o)
    );

    let packed = std::fs::read(&pack).expect("read pack");
    let wrapped = write_bytes(dir, "in.purrpck.gz", &gzip(&packed));
    let out = path(dir, "out.nq");

    let o = run(&["convert", "--to", "nquads", &wrapped, &out]);
    assert!(!o.status.success(), "a gzip-wrapped pack must be refused");
    assert!(
        stderr(&o).contains("gzip"),
        "the refusal must name the encoding: {}",
        stderr(&o)
    );
    assert!(!Path::new(&out).exists(), "no partial output");
}

/// An explicit `--transport` against a PACK source is refused by name: a pack never
/// reaches the transport decoder, so the flag would be accepted and never read.
#[test]
fn transport_against_a_pack_source_is_refused_by_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "left.nt", SEED_LEFT);
    let pack = path(dir, "in.purrpck");
    let o = run(&[
        "convert", "--from", "ntriples", "--to", "pack", &seed, &pack,
    ]);
    assert!(
        o.status.success(),
        "seeding the pack failed: {}",
        stderr(&o)
    );

    let out = path(dir, "out.nq");
    let o = run(&[
        "convert",
        "--transport",
        "gzip",
        "--to",
        "nquads",
        &pack,
        &out,
    ]);
    assert!(!o.status.success(), "--transport on a pack must be refused");
    assert_eq!(o.status.code(), Some(2), "usage errors exit 2");
    assert!(stderr(&o).contains("--transport"), "{}", stderr(&o));

    // `auto` (the default) is still fine: it decodes nothing and refuses nothing.
    let o = run(&["convert", "--to", "nquads", &pack, &out]);
    assert!(
        o.status.success(),
        "the default must still work: {}",
        stderr(&o)
    );
}

// --------------------------------------------------------------------------------
// Threading: base, transforms and ledgers over every admission
// --------------------------------------------------------------------------------

/// `--base` is threaded into EVERY source of a list, not just the first.
#[test]
fn the_base_iri_is_threaded_into_every_source() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let one = write_file(dir, "one.ttl", "<a> <http://example.org/p> \"one\" .\n");
    let two = write_file(dir, "two.ttl", "<b> <http://example.org/p> \"two\" .\n");
    let out = path(dir, "merged.nq");

    let o = run(&[
        "convert",
        "--base",
        "http://example.org/base/",
        "--input",
        &two,
        &one,
        &out,
    ]);
    assert!(o.status.success(), "based union failed: {}", stderr(&o));
    let text = std::fs::read_to_string(&out).expect("read output");
    assert!(text.contains("<http://example.org/base/a>"), "{text}");
    assert!(text.contains("<http://example.org/base/b>"), "{text}");
}

/// `--canonical` and `--entailment` run over the MERGED dataset, not only the first
/// source, and the merged canonical form is stable.
#[test]
fn the_transform_lane_runs_over_the_whole_source_list() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let schema = write_file(
        dir,
        "schema.nt",
        "<http://example.org/Cat> \
         <http://www.w3.org/2000/01/rdf-schema#subClassOf> \
         <http://example.org/Animal> .\n",
    );
    let data = write_file(
        dir,
        "data.nt",
        "<http://example.org/tom> \
         <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
         <http://example.org/Cat> .\n",
    );
    let out = path(dir, "closure.nq");

    let o = run(&[
        "convert",
        "--entailment",
        "rdfs",
        "--to",
        "nquads",
        "--input",
        &data,
        &schema,
        &out,
    ]);
    assert!(o.status.success(), "entailed union failed: {}", stderr(&o));
    let text = std::fs::read_to_string(&out).expect("read output");
    assert!(
        text.contains(
            "<http://example.org/tom> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
                       <http://example.org/Animal>"
        ),
        "the closure must be over BOTH sources: {text}"
    );

    // `--canonical` over the merged list is stable across runs.
    assert_eq!(
        canonical_of(dir, &["--input", &data, &schema]),
        canonical_of(dir, &["--input", &data, &schema]),
        "canonical output over a merged list must be byte-stable"
    );
}

// --------------------------------------------------------------------------------
// Streaming the line-oriented codecs
// --------------------------------------------------------------------------------

/// A line-oriented document large enough that the source buffer the streaming lane
/// removes is a real quantity rather than a rounding error, and shaped so that the
/// statement layer's forward references cross every plausible read-buffer boundary:
/// reifiers are declared in one place and annotated far away.
fn streamable_nquads(rows: usize) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(rows * 160);
    out.push_str("# streaming fixture\n\n");
    for i in 0..rows {
        match i % 4 {
            0 => writeln!(
                out,
                "<http://example.org/s{}> <http://example.org/p{}> \
                 <http://example.org/o{}> <http://example.org/g{}> .",
                i % 313,
                i % 11,
                i % 307,
                i % 5
            ),
            // Multi-byte UTF-8, so a sequence straddles the 64 KiB read window.
            1 => writeln!(
                out,
                "<http://example.org/s{}> <http://example.org/label> \
                 \"\u{6f22}\u{5b57} \u{1f408} {i}\"@ja .",
                i % 313
            ),
            2 => writeln!(
                out,
                "_:r{} <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> \
                 <<( <http://example.org/a{}> <http://example.org/p{}> \
                 <http://example.org/c{}> )>> .",
                i % 97,
                i % 97,
                (i % 97) % 11,
                i % 97
            ),
            _ => writeln!(
                out,
                "_:r{} <http://example.org/confidence> \"0.{}\" .",
                (i + 50) % 97,
                i % 100
            ),
        }
        .expect("write row");
    }
    out
}

/// The streamed lane and the buffered lane must produce the SAME document.
///
/// The comparison is made across a syntax boundary the CLI itself draws: the same
/// content is offered as N-Quads (line-oriented → streamed) and as TriG (not
/// line-oriented → read whole), and both are canonicalized by the binary. Byte-equal
/// canonical output is an isomorphism proof that streaming changed nothing.
#[test]
fn a_streamed_line_source_and_a_buffered_source_agree() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();

    // TriG is a superset of N-Quads' default-graph statements for this fixture's
    // default-graph rows, so the same text is legal in both — but only the N-Quads
    // reading is streamed.
    let body = concat!(
        "<http://example.org/s> <http://example.org/p> \"o\" .\n",
        "<http://example.org/s> <http://example.org/q> \"\u{6f22}\u{5b57}\"@ja .\n",
        "_:r <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ",
        "<<( <http://example.org/a> <http://example.org/b> <http://example.org/c> )>> .\n",
        "_:r <http://example.org/confidence> \"0.9\" .\n",
    );
    let streamed = write_file(dir, "seed.nq", body);
    let buffered = write_file(dir, "seed.trig", body);

    assert_eq!(
        canonical_of(dir, &[&streamed]),
        canonical_of(dir, &[&buffered]),
        "the streamed line-oriented reading must equal the buffered reading"
    );
}

/// A large line-oriented source converts identically whether it arrives as a FILE or on
/// STDIN — the two open different streams and neither buffers the document.
#[test]
fn a_large_line_source_streams_identically_from_a_file_and_from_stdin() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let body = streamable_nquads(12_000);
    assert!(
        body.len() > (1 << 20),
        "fixture must be larger than one read buffer's worth many times over, got {}",
        body.len()
    );
    let input = write_file(dir, "big.nq", &body);
    let from_file = path(dir, "from-file.nq");
    let from_stdin = path(dir, "from-stdin.nq");

    let o = run(&["convert", "--to", "nquads", &input, &from_file]);
    assert!(o.status.success(), "file convert failed: {}", stderr(&o));

    let o = run_with_stdin(
        &[
            "convert",
            "--from",
            "nquads",
            "--to",
            "nquads",
            "-",
            &from_stdin,
        ],
        body.as_bytes(),
    );
    assert!(o.status.success(), "stdin convert failed: {}", stderr(&o));

    assert_eq!(
        std::fs::read(&from_file).expect("read file output"),
        std::fs::read(&from_stdin).expect("read stdin output"),
        "a file source and a stdin source must stream to the same document"
    );
}

/// The streaming lane pulls THROUGH the transport decoder rather than after it: a
/// gzip/zstd line-oriented source on STDIN — where there is no filename to consult and
/// no seeking back — is sniffed from its leading bytes and decoded incrementally.
#[test]
fn a_compressed_line_source_streams_through_the_decoder_on_stdin() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let body = streamable_nquads(4_000);
    let plain_out = path(dir, "plain.nq");

    let o = run_with_stdin(
        &[
            "convert", "--from", "nquads", "--to", "nquads", "-", &plain_out,
        ],
        body.as_bytes(),
    );
    assert!(o.status.success(), "plain stdin failed: {}", stderr(&o));
    let expected = std::fs::read(&plain_out).expect("read plain output");

    for (label, framed) in [
        ("gzip", gzip(body.as_bytes())),
        ("zstd", zstd(body.as_bytes())),
    ] {
        let out = path(dir, "framed.nq");
        let _ = std::fs::remove_file(&out);
        let o = run_with_stdin(
            &["convert", "--from", "nquads", "--to", "nquads", "-", &out],
            &framed,
        );
        assert!(o.status.success(), "{label} stdin failed: {}", stderr(&o));
        assert_eq!(
            std::fs::read(&out).expect("read framed output"),
            expected,
            "{label}-wrapped stdin must decode to the same document as the plain stream"
        );
    }
}

/// CRLF line endings and a final line with NO trailing newline survive the streamed
/// read, because the line reader reproduces `str::lines` exactly rather than
/// approximating it.
#[test]
fn crlf_and_a_missing_final_newline_survive_streaming() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let lf = concat!(
        "# comment\n",
        "\n",
        "<http://example.org/s> <http://example.org/p> \"a\" .\n",
        "<http://example.org/s> <http://example.org/q> \"b\" .",
    );
    let crlf = lf.replace('\n', "\r\n");
    assert!(!lf.ends_with('\n') && !crlf.ends_with('\n'));

    let lf_path = write_file(dir, "lf.nt", lf);
    let crlf_path = write_file(dir, "crlf.nt", &crlf);
    assert_eq!(
        canonical_of(dir, &[&lf_path]),
        canonical_of(dir, &[&crlf_path]),
        "CRLF and LF documents must stream to the same dataset"
    );

    // And the same content on stdin, where the read boundaries fall differently.
    let out = path(dir, "stdin.nq");
    let o = run_with_stdin(
        &["convert", "--from", "nt", "--to", "nquads", "-", &out],
        crlf.as_bytes(),
    );
    assert!(o.status.success(), "crlf stdin failed: {}", stderr(&o));
    let text = std::fs::read_to_string(&out).expect("read output");
    assert!(text.contains("\"a\""), "first statement survived: {text}");
    assert!(
        text.contains("\"b\""),
        "the unterminated final statement survived: {text}"
    );
}

/// Invalid UTF-8 part-way through a streamed source fails the whole conversion and
/// leaves no partial output: streaming does not turn a hard failure into a short read.
#[test]
fn invalid_utf8_in_a_streamed_source_fails_closed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let mut bytes = b"<http://example.org/s> <http://example.org/p> \"a\" .\n".to_vec();
    bytes.extend_from_slice(b"<http://example.org/s> <http://example.org/q> \"");
    bytes.extend_from_slice(&[0xff, 0xfe]);
    bytes.extend_from_slice(b"\" .\n");
    let input = write_bytes(dir, "bad.nt", &bytes);
    let out = path(dir, "out.nq");

    let o = run(&["convert", "--to", "nquads", &input, &out]);
    assert!(
        !o.status.success(),
        "invalid utf-8 must fail the conversion"
    );
    assert!(
        !Path::new(&out).exists(),
        "invalid utf-8 must leave no partial output"
    );
}

/// Every verb that reads an RDF source — not just `convert` — reaches the streaming
/// lane, because they all share one source seam.
#[test]
fn the_streaming_lane_is_shared_by_every_reading_verb() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let input = write_file(dir, "seed.nq", &streamable_nquads(2_000));

    let o = run(&[
        "query",
        "--data",
        &input,
        "SELECT (COUNT(*) AS ?n) WHERE { GRAPH ?g { ?s ?p ?o } }",
    ]);
    assert!(
        o.status.success(),
        "query over a streamed source: {}",
        stderr(&o)
    );

    let described = path(dir, "described.nq");
    let o = run(&[
        "describe",
        "--iri",
        "http://example.org/s0",
        &input,
        &described,
    ]);
    assert!(
        o.status.success(),
        "describe over a streamed source: {}",
        stderr(&o)
    );
}
