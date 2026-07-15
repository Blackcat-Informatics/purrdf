// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end `convert` coverage that drives the BUILT `purrdf` binary
//! (`env!("CARGO_BIN_EXE_purrdf")`) — never the library — so every assertion pins the
//! shipped executable's behavior.
//!
//! ## What the three seeds isolate
//!
//! Each conversion contract gets its own seed so a failure names exactly one behavior:
//!
//! * **SEED A** — a default-graph-only, star-free dataset with rich term shapes (IRIs,
//!   a blank node, and plain / typed / language-tagged literals). Every one of the nine
//!   native syntaxes represents it losslessly, so it drives the full 9×9 any↔any matrix:
//!   for each ordered `(src, dst)` pair the output must be RDFC-1.0 isomorphic to SEED A
//!   (checked by comparing the `--canonical` N-Quads of the output to SEED A's, which is
//!   order-independent and thus a true isomorphism test).
//! * **SEED B** — a default graph + one named graph (star-free). It isolates the
//!   named-graph PROJECTION contract: dataset-capable targets (N-Quads / TriG) preserve
//!   the named graph; single-graph targets (Turtle / N-Triples) drop it and the loss
//!   ledger records the drop.
//! * **SEED C** — a star-free base quad + one reifier + one annotation. It isolates the
//!   RDF-1.2 statement-layer contract: star-capable text targets round-trip the layer;
//!   the star-INcapable `carries_star = false` targets (RDF/XML, TriX, HexTuples) PROJECT
//!   it to base quads and report the dropped rows in the ledger.
//!
//! ## A note on the "TriX / HexTuples fail-closed" expectation
//!
//! An earlier design note held that TriX and HexTuples HARD-ERROR on any RDF-1.2
//! statement layer. That is NOT how the shipped `convert` behaves, and the tests here
//! assert the REAL, intended behavior instead of the stale note: `serialize_dataset_to_format`
//! (the CLI's serialization entry point) routes every `carries_star = false` format —
//! RDF/XML, TriX, and HexTuples alike — through the base-only PROJECTION path, dropping
//! the statement layer and reporting the dropped-row count. The fail-closed branch that
//! does exist in the TriX codec is only reachable through the full-statement-layer
//! `serialize_dataset` entry point, which the CLI never calls. TriX and HexTuples are
//! also `supports_datasets = true`, so (unlike RDF/XML) they PRESERVE named graphs;
//! only the star layer projects. See `crates/rdf/src/native_codecs/{serialize,media_type}.rs`.
//!
//! ## A note on directional literals
//!
//! RDF-1.2 base-direction literals (`"x"@en--ltr`) are NOT universally lossless: TriX's
//! `<plainLiteral xml:lang>` and the HexTuples 6-field row carry a language slot but no
//! direction surface, so both silently drop the direction (keeping the language tag).
//! SEED A therefore omits a directional literal (it must round-trip through ALL nine
//! formats); `directional_literal_per_format_behavior` covers direction separately and
//! documents that TriX/HexTuples degrade it to a plain language tag.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Output, Stdio};

/// The nine native RDF syntaxes, by their `--from`/`--to` token. Every format appears
/// as BOTH a source and a destination in the matrix.
const FORMATS: [&str; 9] = [
    "turtle",
    "trig",
    "ntriples",
    "nquads",
    "rdfxml",
    "trix",
    "hextuples",
    "jsonld",
    "yamlld",
];

/// SEED A — default-graph-only, star-free, `example.org` only. Rich term shapes that
/// EVERY one of the nine formats represents losslessly: IRIs, a blank node, and plain /
/// typed / language-tagged literals. (A directional literal is deliberately excluded —
/// see the module docs.)
const SEED_A: &str = concat!(
    "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n",
    "<http://example.org/s> <http://example.org/name> \"Alice\" .\n",
    "<http://example.org/s> <http://example.org/age> ",
    "\"42\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
    "<http://example.org/s> <http://example.org/greeting> \"bonjour\"@fr .\n",
    "<http://example.org/s> <http://example.org/rel> _:b0 .\n",
    "_:b0 <http://example.org/kind> \"node\" .\n",
);

/// SEED B — a default-graph quad + one named-graph quad (star-free). Isolates the
/// named-graph projection contract.
const SEED_B: &str = concat!(
    "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n",
    "<http://example.org/gs> <http://example.org/gp> <http://example.org/go> ",
    "<http://example.org/g1> .\n",
);

/// SEED C — a star-free base quad + one reifier (`rdf:reifies` a quoted triple) + one
/// annotation on that reifier. Isolates the RDF-1.2 statement-layer contract.
const SEED_C: &str = concat!(
    "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n",
    "<http://example.org/r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ",
    "<<( <http://example.org/s> <http://example.org/p> <http://example.org/o> )>> .\n",
    "<http://example.org/r> <http://example.org/certainty> \"0.9\" .\n",
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

/// stderr of an [`Output`] as a `String`, for diagnostics + ledger assertions.
fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Join a name onto `dir`, returning it as an owned `String` (the shape [`run`] wants).
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

/// The RDFC-1.0 canonical N-Quads document of `input` (parsed as `from`), produced BY
/// THE BINARY via `--canonical`. Byte-equality of two such documents is an isomorphism
/// test (canonical N-Quads are order- and blank-label-independent).
fn canonical(dir: &Path, from: &str, input: &str) -> Vec<u8> {
    let out = path(dir, "canonical.scratch.nq");
    let o = run(&["convert", "--from", from, "--canonical", input, &out]);
    assert!(
        o.status.success(),
        "canonicalizing {input} as {from} failed: {}",
        stderr(&o)
    );
    std::fs::read(&out).expect("read canonical scratch output")
}

/// The full 9×9 any↔any matrix over SEED A: every ordered `(src, dst)` pair converts
/// with exit 0, non-empty output, and output RDFC-1.0-isomorphic to SEED A.
#[test]
fn matrix_seed_a_every_pair_is_isomorphic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed_a = write_file(dir, "seedA.nq", SEED_A);
    let canon_a = canonical(dir, "nquads", &seed_a);

    // Render SEED A into each of the nine source syntaxes, and confirm each rendering is
    // itself isomorphic to SEED A before it feeds the matrix.
    for src in FORMATS {
        let src_path = path(dir, &format!("seed.{src}"));
        let o = run(&[
            "convert", "--from", "nquads", "--to", src, &seed_a, &src_path,
        ]);
        assert!(
            o.status.success(),
            "rendering SEED A to {src} failed: {}",
            stderr(&o)
        );
        assert_eq!(
            canonical(dir, src, &src_path),
            canon_a,
            "SEED A rendered as {src} is not isomorphic to SEED A"
        );
    }

    // The matrix proper: convert every source rendering to every destination format.
    for src in FORMATS {
        let src_path = path(dir, &format!("seed.{src}"));
        for dst in FORMATS {
            let out_path = path(dir, &format!("out.{src}.to.{dst}"));
            let o = run(&["convert", "--from", src, "--to", dst, &src_path, &out_path]);
            assert!(
                o.status.success(),
                "{src} -> {dst} exited nonzero: {}",
                stderr(&o)
            );
            let bytes = std::fs::read(&out_path).expect("read matrix output");
            assert!(!bytes.is_empty(), "{src} -> {dst} produced empty output");
            assert_eq!(
                canonical(dir, dst, &out_path),
                canon_a,
                "{src} -> {dst} output is not isomorphic to SEED A"
            );
        }
    }
}

/// SEED B to a dataset-capable target (N-Quads, TriG) preserves the named-graph quad
/// (the whole dataset stays isomorphic).
#[test]
fn seed_b_named_graph_preserved_by_dataset_targets() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed_b = write_file(dir, "seedB.nq", SEED_B);
    let canon_b = canonical(dir, "nquads", &seed_b);

    for dst in ["nquads", "trig"] {
        let out_path = path(dir, &format!("b.{dst}"));
        let o = run(&[
            "convert", "--from", "nquads", "--to", dst, &seed_b, &out_path,
        ]);
        assert!(o.status.success(), "SEED B -> {dst}: {}", stderr(&o));
        let text = std::fs::read_to_string(&out_path).expect("read output");
        assert!(
            text.contains("http://example.org/g1"),
            "{dst} must preserve the named graph IRI"
        );
        assert_eq!(
            canonical(dir, dst, &out_path),
            canon_b,
            "SEED B -> {dst} must stay isomorphic (nothing dropped)"
        );
    }
}

/// SEED B to a single-graph target (Turtle, N-Triples) drops the named-graph quad
/// (only the default-graph quad survives) and, under `--loss-ledger`, records the drop.
#[test]
fn seed_b_named_graph_projected_away_by_single_graph_targets() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed_b = write_file(dir, "seedB.nq", SEED_B);

    for dst in ["turtle", "ntriples"] {
        let out_path = path(dir, &format!("b.{dst}"));
        let o = run(&[
            "--loss-ledger",
            "convert",
            "--from",
            "nquads",
            "--to",
            dst,
            &seed_b,
            &out_path,
        ]);
        assert!(o.status.success(), "SEED B -> {dst}: {}", stderr(&o));
        let text = std::fs::read_to_string(&out_path).expect("read output");
        // The default-graph quad survives; the named-graph quad's terms do NOT.
        assert!(
            text.contains("http://example.org/s"),
            "{dst} must keep the default-graph quad"
        );
        assert!(
            !text.contains("http://example.org/g1"),
            "{dst} must drop the named graph IRI (projection)"
        );
        assert!(
            !text.contains("http://example.org/gs"),
            "{dst} must drop the named-graph quad's subject (projection)"
        );
        // The loss ledger (on stderr for a bare `--loss-ledger`) records the drop.
        assert!(
            stderr(&o).contains("named-graph-dropped"),
            "{dst} loss ledger must record the named-graph drop; got: {}",
            stderr(&o)
        );
    }
}

/// SEED C round-trips the RDF-1.2 statement layer through every star-capable text
/// target (isomorphism holds, so the reifier + annotation survive).
#[test]
fn seed_c_statement_layer_roundtrips_star_capable_targets() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed_c = write_file(dir, "seedC.nq", SEED_C);
    let canon_c = canonical(dir, "nquads", &seed_c);

    for dst in ["nquads", "trig", "turtle", "ntriples", "jsonld", "yamlld"] {
        let out_path = path(dir, &format!("c.{dst}"));
        let o = run(&[
            "convert", "--from", "nquads", "--to", dst, &seed_c, &out_path,
        ]);
        assert!(o.status.success(), "SEED C -> {dst}: {}", stderr(&o));
        let text = std::fs::read_to_string(&out_path).expect("read output");
        // The annotation predicate surfaces in EVERY star-capable rendering (as a
        // triple in the line/Turtle family, as an `@annotation` object in JSON-LD /
        // YAML-LD). The reifier itself is proven present by the isomorphism check below
        // — its literal spelling differs per syntax (JSON-LD nests it structurally and
        // never writes the `reifies` token), so it is not asserted textually.
        assert!(
            text.contains("certainty"),
            "{dst} must carry the annotation predicate"
        );
        assert_eq!(
            canonical(dir, dst, &out_path),
            canon_c,
            "SEED C -> {dst} must round-trip the statement layer (isomorphic: reifier + annotation)"
        );
    }
}

/// SEED C to a `carries_star = false` target (RDF/XML, TriX, HexTuples) PROJECTS the
/// statement layer: exit 0, base quad present, star layer gone, and the ledger records
/// the dropped statement rows. (This is the real behavior; none of these fail-close in
/// the CLI's serialization path — see the module docs.)
#[test]
fn seed_c_statement_layer_projected_by_star_incapable_targets() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed_c = write_file(dir, "seedC.nq", SEED_C);

    for dst in ["rdfxml", "trix", "hextuples"] {
        let out_path = path(dir, &format!("c.{dst}"));
        let o = run(&[
            "--loss-ledger",
            "convert",
            "--from",
            "nquads",
            "--to",
            dst,
            &seed_c,
            &out_path,
        ]);
        assert!(
            o.status.success(),
            "SEED C -> {dst} must PROJECT (exit 0), not fail-close: {}",
            stderr(&o)
        );
        let text = std::fs::read_to_string(&out_path).expect("read output");
        assert!(
            text.contains("http://example.org/s"),
            "{dst} must keep the base quad"
        );
        assert!(
            !text.contains("reifies"),
            "{dst} must drop the reifier binding (projection)"
        );
        // The realized dropped-row count is recorded (never a silent drop).
        assert!(
            stderr(&o).contains("statement-rows-dropped"),
            "{dst} loss ledger must record the dropped statement rows; got: {}",
            stderr(&o)
        );
    }
}

/// Directional (base-direction) literals round-trip through the seven direction-capable
/// formats, but TriX and HexTuples degrade them to a plain language tag (they have a
/// language slot but no direction surface). Documents the real per-format behavior.
#[test]
fn directional_literal_per_format_behavior() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(
        dir,
        "dir.nq",
        "<http://example.org/s> <http://example.org/greeting> \"hello\"@en--ltr .\n",
    );
    let canon = canonical(dir, "nquads", &seed);

    // Direction-capable: full round-trip (isomorphic).
    for dst in [
        "turtle", "trig", "ntriples", "nquads", "rdfxml", "jsonld", "yamlld",
    ] {
        let out_path = path(dir, &format!("dir.{dst}"));
        let o = run(&["convert", "--from", "nquads", "--to", dst, &seed, &out_path]);
        assert!(o.status.success(), "dir -> {dst}: {}", stderr(&o));
        assert_eq!(
            canonical(dir, dst, &out_path),
            canon,
            "{dst} must preserve the base direction"
        );
    }

    // TriX / HexTuples: direction is dropped, language tag retained — so the round-trip
    // is NOT isomorphic, and the re-canonicalized form loses the `--ltr`.
    for dst in ["trix", "hextuples"] {
        let out_path = path(dir, &format!("dir.{dst}"));
        let o = run(&["convert", "--from", "nquads", "--to", dst, &seed, &out_path]);
        assert!(o.status.success(), "dir -> {dst}: {}", stderr(&o));
        let back = canonical(dir, dst, &out_path);
        assert_ne!(
            back, canon,
            "{dst} is expected to drop the base direction (known projection)"
        );
        let back_text = String::from_utf8(back).expect("utf-8");
        assert!(
            back_text.contains("@en") && !back_text.contains("--ltr"),
            "{dst} must degrade the directional literal to a plain @en tag; got: {back_text}"
        );
    }
}

/// Dropping a base direction is NEVER silent: converting a directional literal to TriX
/// or HexTuples (the only two formats with a language slot but no direction surface)
/// records the `rdf12-direction-dropped` loss code, while a direction-capable target
/// (N-Quads) leaves the ledger empty. Complements
/// [`directional_literal_per_format_behavior`], which pins the emitted bytes.
#[test]
fn directional_literal_drop_recorded_in_loss_ledger() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(
        dir,
        "dir.nt",
        "<http://example.org/s> <http://example.org/p> \"hello\"@en--ltr .\n",
    );

    // TriX / HexTuples: the direction is dropped, so the ledger is non-empty and names
    // the `rdf12-direction-dropped` code. The emitted bytes still carry the language tag.
    for dst in ["trix", "hextuples"] {
        let out_path = path(dir, &format!("dir.{dst}"));
        let ledger_path = path(dir, &format!("dir.{dst}.ledger.json"));
        let o = run(&[
            &format!("--loss-ledger={ledger_path}"),
            "convert",
            "--from",
            "ntriples",
            "--to",
            dst,
            &seed,
            &out_path,
        ]);
        assert!(o.status.success(), "dir -> {dst}: {}", stderr(&o));
        let ledger = std::fs::read_to_string(&ledger_path).expect("read ledger json");
        assert!(
            ledger.contains("rdf12-direction-dropped"),
            "{dst} ledger must record the dropped base direction; got: {ledger}"
        );
        assert!(
            ledger.contains("\"code\""),
            "{dst} ledger must be non-empty (carry a loss entry); got: {ledger}"
        );
        // Bytes are unchanged: the language tag survives, only the direction is lost.
        let out_text = std::fs::read_to_string(&out_path).expect("read output");
        assert!(
            out_text.contains("en") && !out_text.contains("ltr"),
            "{dst} must keep the language tag and drop the direction; got: {out_text}"
        );
    }

    // N-Quads carries the base direction: the ledger stays empty (no direction drop).
    let out_path = path(dir, "dir.nq");
    let ledger_path = path(dir, "dir.nq.ledger.json");
    let o = run(&[
        &format!("--loss-ledger={ledger_path}"),
        "convert",
        "--from",
        "ntriples",
        "--to",
        "nquads",
        &seed,
        &out_path,
    ]);
    assert!(o.status.success(), "dir -> nquads: {}", stderr(&o));
    let ledger = std::fs::read_to_string(&ledger_path).expect("read ledger json");
    assert!(
        !ledger.contains("rdf12-direction-dropped"),
        "nquads preserves direction — must NOT record a drop; got: {ledger}"
    );
    assert!(
        !ledger.contains("\"code\""),
        "nquads ledger must be empty (no loss entries — direction preserved); got: {ledger}"
    );
}

/// An extensionless input with explicit `--from`/`--to` converts correctly.
#[test]
fn explicit_formats_on_extensionless_input() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "seedA_no_ext", SEED_A);
    let out = path(dir, "out.ttl");
    let o = run(&["convert", "--from", "nquads", "--to", "turtle", &seed, &out]);
    assert!(
        o.status.success(),
        "explicit convert failed: {}",
        stderr(&o)
    );
    assert_eq!(
        canonical(dir, "turtle", &out),
        canonical(dir, "nquads", &seed),
        "explicit-format convert must be isomorphic"
    );
}

/// An extensionless input with NO `--from` is unresolvable — a usage error (exit 2).
#[test]
fn extensionless_input_without_from_is_a_usage_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "seedA_no_ext", SEED_A);
    let out = path(dir, "out.ttl");
    let o = run(&["convert", &seed, &out]);
    assert!(
        !o.status.success(),
        "an unresolvable input format must fail"
    );
    assert_eq!(o.status.code(), Some(2), "usage errors exit 2");
}

/// A misleading extension is overridden by an explicit `--from`: a `.txt` file that
/// actually holds Turtle, and a `.nt` file that actually holds N-Quads, both convert.
#[test]
fn explicit_from_overrides_a_misleading_extension() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();

    // A `.txt` file that is really Turtle.
    let turtle = "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n";
    let txt = write_file(dir, "data.txt", turtle);
    let out_a = path(dir, "from_txt.nq");
    let o = run(&[
        "convert", "--from", "turtle", "--to", "nquads", &txt, &out_a,
    ]);
    assert!(o.status.success(), ".txt-as-turtle failed: {}", stderr(&o));

    // A `.nt` file that is really N-Quads (a named graph — which N-Triples could not
    // carry), converted with `--from nquads` overriding the `.nt` extension.
    let nt = write_file(dir, "mislabelled.nt", SEED_B);
    let out_b = path(dir, "from_nt.trig");
    let o = run(&["convert", "--from", "nquads", "--to", "trig", &nt, &out_b]);
    assert!(o.status.success(), ".nt-as-nquads failed: {}", stderr(&o));
    let text = std::fs::read_to_string(&out_b).expect("read output");
    assert!(
        text.contains("http://example.org/g1"),
        "the N-Quads override must preserve the named graph"
    );
}

/// stdin `-` in with explicit `--from`, stdout `-` out with explicit `--to`: bytes piped
/// through the process convert correctly.
#[test]
fn stdin_to_stdout_with_explicit_formats() {
    let mut child = purrdf()
        .args(["convert", "--from", "nquads", "--to", "turtle", "-", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn purrdf for the stdin pipe");
    child
        .stdin
        .take()
        .expect("child stdin")
        .write_all(SEED_A.as_bytes())
        .expect("write to child stdin");
    let out = child.wait_with_output().expect("await child");
    assert!(
        out.status.success(),
        "stdin->stdout convert failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).expect("utf-8 stdout");
    assert!(
        stdout.contains("http://example.org/s") && stdout.contains("bonjour"),
        "the Turtle piped to stdout must carry the converted triples; got: {stdout}"
    );
}

/// A text→pack→text round-trip is byte-identical to the equivalent direct text→text
/// conversion (the pack is a lossless container; the zero-copy pack read re-serializes
/// identically).
#[test]
fn text_via_pack_equals_direct_text() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "seedA.nq", SEED_A);

    let pack = path(dir, "mid.purrpck");
    let via_pack = path(dir, "via_pack.nt");
    let direct = path(dir, "direct.nt");

    assert!(
        run(&["convert", "--from", "nquads", "--to", "pack", &seed, &pack])
            .status
            .success()
    );
    assert!(
        run(&[
            "convert", "--from", "pack", "--to", "ntriples", &pack, &via_pack
        ])
        .status
        .success()
    );
    assert!(
        run(&[
            "convert", "--from", "nquads", "--to", "ntriples", &seed, &direct
        ])
        .status
        .success()
    );

    assert_eq!(
        std::fs::read(&via_pack).expect("read via-pack"),
        std::fs::read(&direct).expect("read direct"),
        "text -> pack -> text must equal direct text -> text (byte-identical)"
    );
}

/// A pack read (mmap, zero-copy) serialized to TriG is byte-identical to the same
/// dataset serialized to TriG directly from text — exercising the zero-copy pack→text
/// serialization path.
#[test]
fn pack_to_trig_equals_direct_dataset_to_trig() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "seedB.nq", SEED_B);

    let pack = path(dir, "b.purrpck");
    let via_pack = path(dir, "via_pack.trig");
    let direct = path(dir, "direct.trig");

    assert!(
        run(&["convert", "--from", "nquads", "--to", "pack", &seed, &pack])
            .status
            .success()
    );
    assert!(
        run(&[
            "convert", "--from", "pack", "--to", "trig", &pack, &via_pack
        ])
        .status
        .success()
    );
    assert!(
        run(&[
            "convert", "--from", "nquads", "--to", "trig", &seed, &direct
        ])
        .status
        .success()
    );

    assert_eq!(
        std::fs::read(&via_pack).expect("read via-pack trig"),
        std::fs::read(&direct).expect("read direct trig"),
        "zero-copy pack -> trig must equal direct dataset -> trig (byte-identical)"
    );
}

/// A DISK pack → pack conversion is a verified byte passthrough: the output file is
/// byte-identical to the input pack (`convert` mmap-borrows the disk pack rather than
/// re-encoding it — see `crates/cli/src/convert.rs`'s pack→pack branch and
/// `crates/cli/src/source.rs::verified_pack_mmap`).
#[test]
fn disk_pack_to_pack_is_byte_identical_passthrough() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "seedC.nq", SEED_C);

    let pack_in = path(dir, "in.purrpck");
    let pack_out = path(dir, "out.purrpck");

    assert!(
        run(&[
            "convert", "--from", "nquads", "--to", "pack", &seed, &pack_in
        ])
        .status
        .success(),
        "seeding the source pack must succeed"
    );

    let o = run(&["convert", &pack_in, &pack_out]);
    assert!(
        o.status.success(),
        "pack -> pack passthrough must exit 0: {}",
        stderr(&o)
    );

    assert_eq!(
        std::fs::read(&pack_out).expect("read passthrough output"),
        std::fs::read(&pack_in).expect("read source pack"),
        "pack -> pack must be a byte-identical passthrough of the input pack"
    );
}

/// A truncated/garbage `.purrpck` fed to `convert`'s pack→pack passthrough fails closed
/// (exit non-zero): `verified_pack_mmap` runs the pack integrity verifier before any
/// bytes are written to the output.
#[test]
fn disk_pack_to_pack_corrupt_input_fails_closed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let bad = write_file(dir, "bad.purrpck", "not a pack file at all — pure garbage");
    let out = path(dir, "out.purrpck");

    let o = run(&["convert", &bad, &out]);
    assert!(
        !o.status.success(),
        "a corrupt pack must fail closed (exit non-zero)"
    );
    assert!(
        !stderr(&o).is_empty(),
        "the pack-integrity failure must print a diagnostic to stderr"
    );
    assert!(
        !Path::new(&out).exists(),
        "a failed passthrough must not leave a partial output file"
    );
}

/// A large N-Triples ingress (5000 `example.org` triples) converts to N-Quads with every
/// triple present and the exact count preserved (correctness at volume, not zero-copy).
#[test]
fn large_ntriples_ingress_preserves_every_triple() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();

    let mut big = String::new();
    for i in 0..5000 {
        writeln!(
            big,
            "<http://example.org/s{i}> <http://example.org/p> <http://example.org/o{i}> ."
        )
        .expect("write into String is infallible");
    }
    let seed = write_file(dir, "big.nt", &big);
    let out = path(dir, "big.nq");
    let o = run(&[
        "convert", "--from", "ntriples", "--to", "nquads", &seed, &out,
    ]);
    assert!(o.status.success(), "large ingress failed: {}", stderr(&o));

    let text = std::fs::read_to_string(&out).expect("read big output");
    let lines = text.lines().filter(|l| !l.trim().is_empty()).count();
    assert_eq!(lines, 5000, "every triple must survive the large ingress");
    assert!(
        text.contains("http://example.org/s0>"),
        "first triple present"
    );
    assert!(
        text.contains("http://example.org/s4999>"),
        "last triple present"
    );
}

/// `--canonical` is stable (two runs are byte-identical) and order-independent (two
/// different serializations of the same data canonicalize to the same bytes).
#[test]
fn canonical_output_is_stable_and_serialization_independent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed_nq = write_file(dir, "seedA.nq", SEED_A);

    // Stable: canonicalizing the same input twice yields identical bytes.
    let c1 = path(dir, "c1.nq");
    let c2 = path(dir, "c2.nq");
    assert!(
        run(&["convert", "--from", "nquads", "--canonical", &seed_nq, &c1])
            .status
            .success()
    );
    assert!(
        run(&["convert", "--from", "nquads", "--canonical", &seed_nq, &c2])
            .status
            .success()
    );
    assert_eq!(
        std::fs::read(&c1).expect("c1"),
        std::fs::read(&c2).expect("c2"),
        "--canonical must be byte-stable across runs"
    );

    // Serialization-independent: a Turtle rendering of the same data canonicalizes to the
    // SAME bytes as the N-Quads original.
    let seed_ttl = path(dir, "seedA.ttl");
    assert!(
        run(&[
            "convert", "--from", "nquads", "--to", "turtle", &seed_nq, &seed_ttl
        ])
        .status
        .success()
    );
    assert_eq!(
        canonical(dir, "turtle", &seed_ttl),
        canonical(dir, "nquads", &seed_nq),
        "canonical output must be independent of the input serialization"
    );
}

/// A plain conversion run twice is byte-identical: the serializers are deterministic.
#[test]
fn conversion_is_deterministic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "seedA.nq", SEED_A);
    let a = path(dir, "a.trig");
    let b = path(dir, "b.trig");
    assert!(
        run(&["convert", "--from", "nquads", "--to", "trig", &seed, &a])
            .status
            .success()
    );
    assert!(
        run(&["convert", "--from", "nquads", "--to", "trig", &seed, &b])
            .status
            .success()
    );
    assert_eq!(
        std::fs::read(&a).expect("a"),
        std::fs::read(&b).expect("b"),
        "a conversion run twice must be byte-identical"
    );
}

/// `--base` resolves relative IRIs in the input against the supplied base while parsing.
#[test]
fn base_resolves_relative_iris_on_parse() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    // Turtle with a relative IRI subject/object, resolved against `--base`.
    let ttl = write_file(dir, "rel.ttl", "<thing> <http://example.org/p> <other> .\n");
    let out = path(dir, "resolved.nt");
    let o = run(&[
        "convert",
        "--from",
        "turtle",
        "--to",
        "ntriples",
        "--base",
        "http://example.org/base/",
        &ttl,
        &out,
    ]);
    assert!(o.status.success(), "--base convert failed: {}", stderr(&o));
    let text = std::fs::read_to_string(&out).expect("read resolved");
    assert!(
        text.contains("http://example.org/base/thing"),
        "relative subject must resolve against --base; got: {text}"
    );
    assert!(
        text.contains("http://example.org/base/other"),
        "relative object must resolve against --base; got: {text}"
    );
}

/// `--entailment rdfs` materializes the closure so an inferred `rdf:type` triple appears
/// that is ABSENT without the flag — from a TEXT input.
#[test]
fn entailment_rdfs_infers_type_from_text_input() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(
        dir,
        "sub.ttl",
        concat!(
            "@prefix ex: <http://example.org/> .\n",
            "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
            "ex:Dog rdfs:subClassOf ex:Animal .\n",
            "ex:rex a ex:Dog .\n",
        ),
    );

    // The inferred triple: `ex:rex a ex:Animal`.
    let inferred = "<http://example.org/rex> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Animal>";

    // Without --entailment: the inference is absent.
    let plain = path(dir, "plain.nt");
    assert!(
        run(&[
            "convert", "--from", "turtle", "--to", "ntriples", &ttl, &plain
        ])
        .status
        .success()
    );
    assert!(
        !std::fs::read_to_string(&plain)
            .expect("read plain")
            .contains(inferred),
        "the inferred type must be ABSENT without --entailment"
    );

    // With --entailment rdfs: the inference appears.
    let entailed = path(dir, "entailed.nt");
    let o = run(&[
        "convert",
        "--from",
        "turtle",
        "--to",
        "ntriples",
        "--entailment",
        "rdfs",
        &ttl,
        &entailed,
    ]);
    assert!(
        o.status.success(),
        "--entailment rdfs failed: {}",
        stderr(&o)
    );
    assert!(
        std::fs::read_to_string(&entailed)
            .expect("read entailed")
            .contains(inferred),
        "the inferred type must appear with --entailment rdfs"
    );
}

/// `--entailment rdfs` over a PACK input reconstructs the dataset and materializes the
/// same inferred triple (the reconstruct-then-entail path).
#[test]
fn entailment_rdfs_infers_type_from_pack_input() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(
        dir,
        "sub.ttl",
        concat!(
            "@prefix ex: <http://example.org/> .\n",
            "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
            "ex:Dog rdfs:subClassOf ex:Animal .\n",
            "ex:rex a ex:Dog .\n",
        ),
    );
    let inferred = "<http://example.org/rex> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Animal>";

    // Build a pack, then entail from the pack.
    let pack = path(dir, "sub.purrpck");
    assert!(
        run(&["convert", "--from", "turtle", "--to", "pack", &ttl, &pack])
            .status
            .success()
    );
    let entailed = path(dir, "entailed_from_pack.nt");
    let o = run(&[
        "convert",
        "--from",
        "pack",
        "--to",
        "ntriples",
        "--entailment",
        "rdfs",
        &pack,
        &entailed,
    ]);
    assert!(
        o.status.success(),
        "--entailment rdfs over a pack failed: {}",
        stderr(&o)
    );
    assert!(
        std::fs::read_to_string(&entailed)
            .expect("read entailed-from-pack")
            .contains(inferred),
        "the inferred type must appear when entailing a reconstructed pack"
    );
}

/// `--entailment rdfs --canonical` composes: entail first, then emit canonical N-Quads
/// containing the inferred triple.
#[test]
fn entailment_then_canonical_composes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(
        dir,
        "sub.ttl",
        concat!(
            "@prefix ex: <http://example.org/> .\n",
            "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
            "ex:Dog rdfs:subClassOf ex:Animal .\n",
            "ex:rex a ex:Dog .\n",
        ),
    );
    let out = path(dir, "closure.nq");
    let o = run(&[
        "convert",
        "--from",
        "turtle",
        "--entailment",
        "rdfs",
        "--canonical",
        &ttl,
        &out,
    ]);
    assert!(
        o.status.success(),
        "--entailment rdfs --canonical failed: {}",
        stderr(&o)
    );
    let text = std::fs::read_to_string(&out).expect("read closure");
    assert!(
        text.contains(
            "<http://example.org/rex> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Animal>"
        ),
        "canonical closure must contain the inferred type; got: {text}"
    );
    // Canonical output is stable even after entailment.
    let out2 = path(dir, "closure2.nq");
    assert!(
        run(&[
            "convert",
            "--from",
            "turtle",
            "--entailment",
            "rdfs",
            "--canonical",
            &ttl,
            &out2,
        ])
        .status
        .success()
    );
    assert_eq!(
        std::fs::read(&out).expect("closure"),
        std::fs::read(&out2).expect("closure2"),
        "entail+canonical must be byte-stable"
    );
}

/// An unsupported entailment regime (`d` / `owl-direct` / `rif`) is the exit-3 boundary,
/// matching `reason`'s classification.
#[test]
fn unsupported_entailment_regime_exits_three() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "seedA.nq", SEED_A);
    let out = path(dir, "out.nq");
    for regime in ["d", "owl-direct", "rif"] {
        let o = run(&[
            "convert",
            "--from",
            "nquads",
            "--to",
            "nquads",
            "--entailment",
            regime,
            &seed,
            &out,
        ]);
        assert_eq!(
            o.status.code(),
            Some(3),
            "regime {regime} must exit 3 (unsupported); stderr: {}",
            stderr(&o)
        );
    }
}

/// A downstream consumer closing its end of the stdout pipe early (the ubiquitous
/// `purrdf … | head` idiom) must NOT surface as a runtime failure: `write_out`
/// (`crates/cli/src/sink.rs`) treats a `BrokenPipe` write error on stdout as a clean
/// success. Drives this through a real shell pipe (not a simulated close) so the OS
/// actually delivers the short write / EPIPE: a large (>64 KiB, past the pipe buffer)
/// N-Triples output piped into `head -c 5`, which reads a few bytes and exits,
/// closing its end of the pipe while `purrdf` is still writing. Assert `purrdf`
/// itself (via `PIPESTATUS[0]`, not the pipeline's overall status) exits 0 and prints
/// no "Broken pipe" text to stderr — falsifiable: reverting the `sink.rs` fix makes
/// this fail with a nonzero `PIPESTATUS[0]` and a "Broken pipe" stderr message.
#[test]
fn stdout_broken_pipe_exits_clean() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();

    // A big-enough N-Triples source (well past the OS pipe buffer, typically 64 KiB
    // on Linux) so `purrdf` is still mid-write when `head -c 5` closes its end.
    let mut big = String::new();
    for i in 0..20_000 {
        writeln!(
            big,
            "<http://example.org/s{i}> <http://example.org/p> <http://example.org/o{i}> ."
        )
        .expect("write into String is infallible");
    }
    let seed = write_file(dir, "big.nt", &big);

    let purrdf_bin = env!("CARGO_BIN_EXE_purrdf");
    let stderr_path = path(dir, "purrdf.stderr");
    // `PIPESTATUS[0]` is the FIRST command's (purrdf's) exit status, independent of
    // `head`'s own (which is what a bare `$?` would give). Redirect purrdf's stderr
    // to a file so it is inspectable after the shell exits (Output::stderr would
    // otherwise capture the whole pipeline's, mixed with `head`'s).
    let script = format!(
        "{purrdf_bin:?} convert --from ntriples --to ntriples {seed:?} - 2> {stderr_path:?} | \
         head -c 5 > /dev/null; exit \"${{PIPESTATUS[0]}}\""
    );
    let out = Command::new("bash")
        .arg("-c")
        .arg(&script)
        .output()
        .expect("spawn bash to drive the real pipe");

    let purrdf_stderr = std::fs::read_to_string(&stderr_path).unwrap_or_default();
    assert!(
        out.status.success(),
        "purrdf must exit 0 when stdout's downstream reader closes early; \
         PIPESTATUS[0]={:?}, purrdf stderr: {purrdf_stderr}",
        out.status.code()
    );
    assert!(
        !purrdf_stderr.to_lowercase().contains("broken pipe"),
        "purrdf must not report BrokenPipe as an error; stderr: {purrdf_stderr}"
    );
}

/// `--canonical` overrides `--to`: even with `--to turtle` requested, the output is
/// RDFC-1.0 canonical N-Quads (documented precedence).
#[test]
fn canonical_overrides_to_format() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "seedA.nq", SEED_A);
    let out = path(dir, "out.canon");
    let o = run(&[
        "convert",
        "--from",
        "nquads",
        "--to",
        "turtle",
        "--canonical",
        &seed,
        &out,
    ]);
    assert!(
        o.status.success(),
        "--canonical over --to failed: {}",
        stderr(&o)
    );
    let text = std::fs::read_to_string(&out).expect("read canonical");
    // Canonical N-Quads use `<...>` triples with a trailing ` .`, and NEVER Turtle's
    // `@prefix` directives — proving `--to turtle` was ignored.
    assert!(
        !text.contains("@prefix"),
        "canonical output must NOT be Turtle; got: {text}"
    );
    assert!(
        text.contains("_:c14n0") || text.contains("http://example.org/s>"),
        "canonical output must be N-Quads; got: {text}"
    );
    assert_eq!(
        text.into_bytes(),
        canonical(dir, "nquads", &seed),
        "output must equal the RDFC-1.0 canonical N-Quads of the input"
    );
}
