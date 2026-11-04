// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! Timed benchmark of the real CLI pack path, broken into the THREE phases of a pack
//! operation so each cost is legible on its own rather than lumped:
//!
//! * **acquisition** — building an [`ImmutableInput`] from disk (the Tier-1 sealed
//!   `memfd` snapshot the common case takes) versus from an owned buffer (the Tier-2
//!   path stdin and non-Linux take);
//! * **verification** — the unconditional canonical `verify_pack` every read/reason
//!   path runs on open;
//! * **reasoning** — materializing an RDFS closure over the zero-copy `PackView`
//!   directly versus over an owned `RdfDataset` rebuilt from the view
//!   (`dataset_from_view`) — the rebuild the DatasetView boundary now avoids.
//!
//! Report-only (not a gate). Peak-allocation evidence for the same paths lives in the
//! separate `pack_input_alloc` bench, whose tracking allocator would otherwise skew
//! these latencies.

use std::hint::black_box;

use purrdf_cli::immutable::ImmutableInput;
use purrdf_core::{PackView, dataset_from_view, verify_pack};
use purrdf_entail::{Materialization, materialize};
use purrdf_testkit::bench::{Bench, bench_group, bench_main};

#[path = "support/pack.rs"]
mod support;

/// Phase 1: build an [`ImmutableInput`] from disk (Tier-1 sealed `memfd`) vs from an
/// owned buffer (Tier 2).
fn acquisition(c: &mut Bench) {
    // `file` is held for the whole benchmark so `path` stays valid; the acquisition
    // paths read the bytes themselves (Tier 1 maps the fd, Tier 2 reads the file).
    let (file, _bytes) = support::large_pack();
    let path = file.path().to_str().expect("utf-8 path").to_owned();

    let mut group = c.benchmark_group("pack_acquisition");
    group.bench_function("tier1_sealed_memfd", |b| {
        b.iter(|| {
            let input = ImmutableInput::from_disk_path(&path).expect("acquire from disk");
            black_box(input.as_bytes().len())
        });
    });
    // The owned path reads the whole file into a heap buffer each time (what stdin and
    // non-Linux take), a fair counterpart to the Tier-1 disk acquisition above.
    group.bench_function("tier2_owned_read", |b| {
        b.iter(|| {
            let read = std::fs::read(&path).expect("read pack bytes");
            let input = ImmutableInput::from_owned(read);
            black_box(input.as_bytes().len())
        });
    });
    group.finish();
}

/// Phase 2: the unconditional canonical `verify_pack` run on every pack open.
fn verification(c: &mut Bench) {
    let (_file, bytes) = support::large_pack();
    let mut group = c.benchmark_group("pack_verification");
    group.bench_function("verify_pack", |b| {
        b.iter(|| {
            black_box(
                verify_pack(
                    black_box(&bytes),
                    &mut purrdf_core::CanonicalizationControl::new(
                        purrdf_core::RDFC_V2_CORPUS_POLICY,
                        None,
                    ),
                )
                .expect("verify"),
            )
        });
    });
    group.finish();
}

/// Phase 3: materialize an RDFS closure over the zero-copy `PackView` vs over an owned
/// `RdfDataset` rebuilt from it (the rebuild the DatasetView boundary now avoids).
fn reasoning(c: &mut Bench) {
    let (_file, bytes) = support::large_pack();
    let view = PackView::from_bytes(&bytes).expect("open pack view");

    let mut group = c.benchmark_group("pack_reasoning_rdfs");
    // The path this change preserves: the reasoner is seeded from the view directly.
    group.bench_function("over_pack_view", |b| {
        b.iter(|| black_box(materialize(&view, Materialization::Rdfs).expect("materialize")));
    });
    // The path it avoids: rebuild an owned dataset from the view, then reason over it.
    group.bench_function("over_rebuilt_dataset", |b| {
        b.iter(|| {
            let rebuilt = dataset_from_view(&view).expect("rebuild owned dataset");
            black_box(materialize(&*rebuilt, Materialization::Rdfs).expect("materialize"))
        });
    });
    group.finish();
}

/// Actual command startup plus buffered file conversion; setup is outside timing.
fn word_ids(c: &mut Bench) {
    use purrdf_hash::{hex::Digest32, word_id::Reference};
    use purrdf_testkit::bench::Throughput;
    use std::{process::Command, time::Duration};
    let directory = purrdf_testkit::temp_dir!().expect("word conversion fixture directory");
    let input = directory.path().join("input.txt");
    let words = directory.path().join("words.txt");
    let batch = directory.path().join("batch.txt");
    let batch_words = directory.path().join("batch-words.txt");
    let invalid = directory.path().join("invalid.txt");
    let output = directory.path().join("output.txt");
    let digest = Digest32::new([0x5a; 32]);
    std::fs::write(&input, format!("{digest}\n")).unwrap();
    std::fs::write(&words, Reference::Full(digest).to_text().unwrap()).unwrap();
    std::fs::write(&invalid, b"b3w2: invalid").unwrap();
    let mut hex_rows = String::new();
    let mut word_rows = String::new();
    let mut values = Vec::new();
    for ordinal in 0_u64..256 {
        let digest = Digest32::new(*purrdf_hash::blake3::hash(&ordinal.to_le_bytes()).as_bytes());
        use std::fmt::Write as _;
        writeln!(hex_rows, "{digest}").unwrap();
        writeln!(word_rows, "{}", Reference::Full(digest).to_text().unwrap()).unwrap();
        values.push(digest);
    }
    std::fs::write(&batch, hex_rows).unwrap();
    std::fs::write(&batch_words, word_rows).unwrap();
    use purrdf_core::{
        content_alias::{AliasScope, ScopeId},
        content_prefix::{Length, PrefixPlan},
    };
    use purrdf_lex::allocation::Resident;
    let snapshot = directory.path().join("snapshot.bin");
    let prefix_single = directory.path().join("prefix-single.txt");
    let prefix_hex = directory.path().join("prefix-hex.txt");
    let prefix_batch = directory.path().join("prefix-batch.txt");
    let scope = ScopeId::new(Digest32::new([23; 32]));
    let scope_text = scope.digest().to_string();
    let mut aliases = AliasScope::new(scope, Resident).unwrap();
    aliases.extend(scope, &values).unwrap();
    let binding = aliases.presentation(scope).unwrap();
    let revision_text = binding.revision.digest().to_string();
    let exported = aliases.export(scope).unwrap();
    std::fs::write(&snapshot, &*exported).unwrap();
    exported.release().unwrap();
    let mut plan = PrefixPlan::from_aliases(&mut aliases, Resident).unwrap();
    let mut prefix_rows = String::new();
    for digest in &values {
        use std::fmt::Write as _;
        writeln!(
            prefix_rows,
            "{}",
            plan.shorten(binding, *digest, Length::PerEntry)
                .unwrap()
                .reference
                .to_text()
                .unwrap()
        )
        .unwrap();
    }
    std::fs::write(&prefix_batch, prefix_rows).unwrap();
    std::fs::write(
        &prefix_single,
        plan.shorten(binding, values[0], Length::PerEntry)
            .unwrap()
            .reference
            .to_text()
            .unwrap(),
    )
    .unwrap();
    std::fs::write(&prefix_hex, format!("{}\n", values[0])).unwrap();
    let snapshot_args = [
        "--manifest",
        snapshot.to_str().unwrap(),
        "--scope",
        &scope_text,
        "--revision",
        &revision_text,
    ];
    let mut group = c.benchmark_group("word_ids");
    group
        .sample_size(20)
        .warm_up_time(Duration::from_millis(100))
        .measurement_time(Duration::from_millis(300));
    for (name, arguments, source, count, expected) in [
        (
            "cli_prefix_plan_256",
            vec!["plan", "--scope", &scope_text],
            &batch,
            256,
            0,
        ),
        (
            "cli_prefix_shorten",
            vec!["shorten", "--from", "hex", "--length", "per-entry"],
            &prefix_hex,
            1,
            0,
        ),
        (
            "cli_prefix_expand",
            vec!["expand", "--to", "hex"],
            &prefix_single,
            1,
            0,
        ),
        (
            "cli_prefix_shorten_batch_256",
            vec![
                "batch-scoped",
                "--from",
                "hex-lines",
                "--to",
                "word-lines",
                "--length",
                "per-entry",
            ],
            &batch,
            256,
            0,
        ),
        (
            "cli_prefix_expand_batch_256",
            vec![
                "batch-scoped",
                "--from",
                "word-lines",
                "--to",
                "hex-lines",
                "--length",
                "uniform",
            ],
            &prefix_batch,
            256,
            0,
        ),
        ("cli_encode", vec!["encode", "--from", "hex"], &input, 1, 0),
        ("cli_decode", vec!["decode", "--to", "hex"], &words, 1, 0),
        (
            "cli_decode_invalid",
            vec!["decode", "--to", "hex"],
            &invalid,
            1,
            1,
        ),
        (
            "cli_encode_batch_256",
            vec!["batch", "--from", "hex-lines", "--to", "word-lines"],
            &batch,
            256,
            0,
        ),
        (
            "cli_decode_batch_256",
            vec!["batch", "--from", "word-lines", "--to", "hex-lines"],
            &batch_words,
            256,
            0,
        ),
    ] {
        group.throughput(Throughput::Elements(count));
        group.bench_function(name, |b| {
            b.iter(|| {
                let mut command = Command::new(env!("CARGO_BIN_EXE_purrdf"));
                command.arg("word-id").args(&arguments);
                if name.starts_with("cli_prefix") && name != "cli_prefix_plan_256" {
                    command.args(snapshot_args);
                }
                let result = command
                    .arg("--input")
                    .arg(source)
                    .arg("--output")
                    .arg(&output)
                    .output()
                    .unwrap();
                assert_eq!(
                    result.status.code(),
                    Some(expected),
                    "{}",
                    String::from_utf8_lossy(&result.stderr)
                );
                black_box(result);
            });
        });
    }
}

bench_group!(benches, acquisition, verification, reasoning, word_ids);
bench_main!(benches);
