// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! W3C RDF Dataset Canonicalization (RDFC-1.0) conformance gate (#910).
//!
//! The vendored W3C `rdf-canon` test suite (`tests/fixtures/rdfc/`, see
//! `SOURCE.md`) is the acceptance gate for the native canonicalizer. Each
//! `testNNN-in.nq` input is parsed with the native [`purrdf_rdf::parse_dataset`] codec
//! (oxigraph-free, EPIC #906), canonicalized graph-preservingly by
//! [`purrdf_rdf::canonicalize_with`], and its canonical N-Quads compared to
//! the expected `testNNN-rdfc10.nq`. Inputs WITHOUT an expected output are
//! **negative** (poison / complexity-limit) tests that must abort rather than
//! canonicalize.
//!
//! The suite includes the hard automorphism vectors (test053–test058 etc.) whose
//! blank-node symmetries can only be resolved by RDFC-1.0's n-degree permutation
//! backtracking — a weaker (hash-only) implementation fails them.
//!
//! ## Sharding + heavy carve-out for the 25 s per-test budget (#1045)
//!
//! The full 65-fixture suite runs ~28–32 s on CI, over the always-on 25 s budget.
//! Per-fixture timing (2026-06-26) showed the cost is NOT spread: exactly one
//! vector — `test074` (the sole negative/poison fixture; ~5.3 s isolated-local on
//! the call-budget guard) — dominates, while every other fixture canonicalizes in
//! ≤ ~0.02 s. So:
//!
//! - `test074` is carved into the OFF-GATE [`w3c_rdfc10_heavy_offgate`] test (maint
//!   lane only; excluded from the per-commit gate via `default-filter` in
//!   `.config/nextest.toml`), mirroring the `corpus_parity_heavy_offgate` precedent.
//! - The remaining 64 (all cheap) vectors are split across [`NUM_SHARDS`] gated
//!   `#[test]` fns (`w3c_rdfc10_shard_{0..3}`) by a stable FNV-1a hash of the test
//!   stem so nextest runs them in parallel; each gated shard is well under 1 s.
//! - A cheap [`w3c_inventory`] test guards against fixture loss (incl. the carved
//!   heavy stems) without running any canonicalization.

use std::path::{Path, PathBuf};

use purrdf_rdf::{canonicalize_with, parse_dataset, CanonHash, NativeRdfFormat};

/// Tests that specify `rdfc:hashAlgorithm "SHA384"` in the W3C manifest (the rest
/// use the SHA-256 default). As of the vendored suite this is exactly `test075`
/// ("blank node - diamond (uses SHA-384)").
const SHA384_TESTS: &[&str] = &["test075"];

/// How many independent shard tests the GATED (non-heavy) subset is split across (#1045).
const NUM_SHARDS: usize = 4;

/// OFF-GATE heavy fixtures: stems whose canonicalization is too slow to fit the 25 s
/// always-on per-test budget (#1045). Carved out of the gated shards and exercised
/// only on the maint lane via [`w3c_rdfc10_heavy_offgate`].
///
/// Current entry: `test074` — the suite's sole negative/poison vector. The native
/// canonicalizer takes ~5.3 s isolated-local before the call-budget guard aborts (the
/// pathological all-blank graph); on a contended 4-core CI runner that can balloon
/// past budget. Every other vector canonicalizes in ≤ ~0.02 s. Mirrors the
/// `corpus_parity_heavy_offgate` precedent in `sparql_eval_parity.rs`.
const HEAVY_OFFGATE_STEMS: &[&str] = &["test074"];

fn is_heavy_offgate(stem: &str) -> bool {
    HEAVY_OFFGATE_STEMS.contains(&stem)
}

fn hash_for(stem: &str) -> CanonHash {
    if SHA384_TESTS.contains(&stem) {
        CanonHash::Sha384
    } else {
        CanonHash::Sha256
    }
}

/// Stable FNV-1a hash of a test stem → shard id. Identical algorithm to
/// `sparql_eval_parity.rs` so the sharding pattern is uniform across the codebase.
fn shard_of(stem: &str) -> usize {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in stem.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    (h % NUM_SHARDS as u64) as usize
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rdfc")
}

/// Fixtures the NATIVE N-Quads text codec cannot yet ingest — carved out of the
/// gated shards so the suite stays green while the gap stays VISIBLE and tracked
/// (not silently dropped).
///
/// NOW EMPTY (EPIC #906): the first-party N-Triples/N-Quads/Turtle/TriG parser decodes
/// `\u`/`\U` UCHAR escapes inside IRIREFs (reusing the sparql-algebra term lexer), so
/// `test060` (`<urn:ex:s:000:s⁰1>` etc.) parses and canonicalizes correctly. The slot
/// is retained (empty) so a future native-parse gap can be tracked here rather than
/// silently dropped.
const NATIVE_PARSE_GAP_STEMS: &[&str] = &[];

fn is_native_parse_gap(stem: &str) -> bool {
    NATIVE_PARSE_GAP_STEMS.contains(&stem)
}

/// Compare canonical N-Quads as sorted non-empty line sets (robust to trailing
/// newline conventions; the spec already mandates sorted output, so a real
/// difference still surfaces).
fn norm_lines(s: &str) -> Vec<String> {
    let mut v: Vec<String> = s
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_owned)
        .collect();
    v.sort();
    v
}

/// All `testNNN-in.nq` input fixture paths, sorted.
fn all_inputs(dir: &Path) -> Vec<PathBuf> {
    let mut inputs: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("fixtures/rdfc present")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with("-in.nq"))
        })
        .collect();
    inputs.sort();
    inputs
}

fn stem_of(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_suffix("-in.nq"))
        .expect("input stem")
        .to_owned()
}

/// Run the W3C RDFC-1.0 fixtures whose stem satisfies `include`, collecting failures.
/// This is the SINGLE correctness path shared by the gated shards and the off-gate
/// heavy test — same eval (positive) and negative (poison call-budget) semantics.
fn run_w3c_fixtures(scope: &str, include: &dyn Fn(&str) -> bool) {
    let dir = fixtures_dir();
    let inputs: Vec<PathBuf> = all_inputs(&dir)
        .into_iter()
        .filter(|p| include(&stem_of(p)))
        .collect();

    // Suppress panic noise; we report failures by test name through `failures`.
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let mut failures: Vec<String> = Vec::new();

    for input in &inputs {
        let stem = stem_of(input);
        let expected_path = dir.join(format!("{stem}-rdfc10.nq"));
        let in_text = std::fs::read_to_string(input).expect("read input");
        let quads = match parse_dataset(
            in_text.as_bytes(),
            NativeRdfFormat::NQuads.media_type(),
            None,
        ) {
            Ok(d) => d,
            Err(e) => {
                failures.push(format!("{stem}: native N-Quads parse failed: {e}"));
                continue;
            }
        };

        if expected_path.exists() {
            let outcome =
                std::panic::catch_unwind(|| canonicalize_with(&quads, hash_for(&stem)).nquads);
            match outcome {
                Ok(actual) => {
                    let expected = std::fs::read_to_string(&expected_path).expect("read expected");
                    if norm_lines(&actual) != norm_lines(&expected) {
                        failures.push(format!(
                            "{stem}: canonical output mismatch\n--- expected ---\n{expected}\n--- actual ---\n{actual}"
                        ));
                    }
                }
                Err(_) => failures.push(format!(
                    "{stem}: canonicalization PANICKED on a positive test"
                )),
            }
        } else {
            // Negative (poison) test: canonicalization must abort (the poison call
            // budget trips on the pathological blank graph).
            let outcome =
                std::panic::catch_unwind(|| canonicalize_with(&quads, hash_for(&stem)).nquads);
            match outcome {
                Ok(_) => failures.push(format!(
                    "{stem}: NEGATIVE poison test did not abort (expected the call-budget guard to trip)"
                )),
                Err(payload) => {
                    // The abort MUST be the poison call-budget guard, not an
                    // incidental parse/bridge panic — otherwise a future
                    // regression would masquerade as a poison abort and pass.
                    let msg = payload
                        .downcast_ref::<String>()
                        .map(String::as_str)
                        .or_else(|| payload.downcast_ref::<&str>().copied())
                        .unwrap_or("<non-string panic payload>");
                    if !msg.contains("call budget") {
                        failures.push(format!(
                            "{stem}: NEGATIVE test panicked, but not via the call-budget guard \
                             (payload: {msg:?}); a non-budget panic must not count as a poison abort"
                        ));
                    }
                }
            }
        }
    }

    std::panic::set_hook(prev_hook);

    assert!(
        failures.is_empty(),
        "W3C RDFC-1.0 [{scope}] conformance failures ({}):\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

/// One gated shard: the cheap fixtures assigned to `shard` by [`shard_of`], with the
/// [`HEAVY_OFFGATE_STEMS`] carved out so the gated wall time stays well under budget.
fn run_w3c_shard(shard: usize) {
    run_w3c_fixtures(&format!("shard {shard}/{NUM_SHARDS}"), &|stem| {
        !is_heavy_offgate(stem) && !is_native_parse_gap(stem) && shard_of(stem) == shard
    });
}

// ---------------------------------------------------------------------------
// Gated shard entry points (#1045)
// ---------------------------------------------------------------------------

#[test]
fn w3c_rdfc10_shard_0() {
    run_w3c_shard(0);
}

#[test]
fn w3c_rdfc10_shard_1() {
    run_w3c_shard(1);
}

#[test]
fn w3c_rdfc10_shard_2() {
    run_w3c_shard(2);
}

#[test]
fn w3c_rdfc10_shard_3() {
    run_w3c_shard(3);
}

// ---------------------------------------------------------------------------
// Off-gate heavy vectors (maint lane only; excluded from the per-commit gate via
// `default-filter` in `.config/nextest.toml`) (#1045)
// ---------------------------------------------------------------------------

/// OFF-GATE: runs exactly the [`HEAVY_OFFGATE_STEMS`] (the sole negative/poison
/// vector `test074`, ~5.3 s) with the SAME eval/negative correctness logic as the
/// gated shards. Kept always-runnable (NOT `#[ignore]`d) and exercised on the
/// `maint-rust-heavy` nextest profile / `make maint-rust-heavy`; the per-commit gate
/// excludes it via the default profile's `default-filter`. Preserves full RDFC-1.0
/// conformance coverage for the heavy vector off the critical path.
#[test]
fn w3c_rdfc10_heavy_offgate() {
    run_w3c_fixtures("heavy-offgate", &|stem| is_heavy_offgate(stem));
}

// ---------------------------------------------------------------------------
// Inventory tripwire (no canonicalization — milliseconds)
// ---------------------------------------------------------------------------

/// Cheap whole-suite guard: counts inputs and verifies the eval/negative split
/// WITHOUT running any canonicalization. Guards against silent fixture loss that
/// a per-shard test (each seeing only its slice) cannot detect on its own — and
/// asserts the carved [`HEAVY_OFFGATE_STEMS`] still exist so they cannot silently
/// vanish off the gate. Bump these counts when the vendored W3C suite is
/// intentionally re-synced.
#[test]
fn w3c_inventory() {
    let dir = fixtures_dir();
    let inputs = all_inputs(&dir);

    // Exact total count — silent fixture loss must fail the gate.
    assert_eq!(
        inputs.len(),
        65,
        "expected exactly 65 vendored W3C rdf-canon inputs, found {}",
        inputs.len()
    );

    // Verify the eval/negative split: eval inputs have a matching `-rdfc10.nq`
    // expected output; negative inputs do not.
    let mut eval = 0usize;
    let mut negative = 0usize;
    for input in &inputs {
        let stem = stem_of(input);
        let expected_path = dir.join(format!("{stem}-rdfc10.nq"));
        if expected_path.exists() {
            eval += 1;
        } else {
            negative += 1;
        }
    }
    // Pin the exact split so a fixture that loses its expected output (silently
    // turning an eval vector into a negative one) fails the gate.
    assert_eq!(
        (eval, negative),
        (64, 1),
        "expected 64 eval + 1 negative W3C vectors, found {eval} eval + {negative} negative"
    );

    // The carved off-gate stems must still exist — otherwise heavy-vector coverage
    // would silently vanish (the off-gate test would run zero fixtures unnoticed).
    let present: std::collections::HashSet<String> = inputs.iter().map(|p| stem_of(p)).collect();
    for stem in HEAVY_OFFGATE_STEMS {
        assert!(
            present.contains(*stem),
            "off-gate heavy fixture {stem:?} is missing — its coverage would silently vanish; \
             update HEAVY_OFFGATE_STEMS if the vendored suite intentionally dropped it"
        );
    }
}
