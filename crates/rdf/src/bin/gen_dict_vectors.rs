// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Freeze the in-band-dictionary corpus vectors.
//!
//! Maintainer-only binary, mirroring `capture_sparql_goldens`. Every fixed
//! source and authoring recipe lives in
//! [`purrdf_rdf::gts_dict_vectors`] — this binary only writes the bytes out, so
//! the drift-guard test (`tests/dict_vectors.rs`) regenerates from exactly the
//! same definitions.
//!
//! - `vectors/30-dict-rawcontent.gts` — one raw-content dictionary, plain `zstd`.
//! - `vectors/31-dict-trained.gts` — one FastCOVER-trained dictionary, plain `zstd`.
//! - `vectors/32-dict-rsyncable.gts` — one raw-content dictionary priming a
//!   `zstd-rsyncable` chain at level 12 (GMEOW's mandated frame profile): the
//!   dictionary primes EVERY independent block, so density improves while the
//!   block-boundary/delta property is preserved exactly.
//! - `vectors/33-multi-dict.gts` — TWO named in-band dictionaries in ONE pack
//!   with per-frame selection between them (§5 `"dct"` has always been a map;
//!   this is the writer using it).
//!
//! 30/32/33 regenerate byte-identically (the raw-content producer has no
//! platform-dependent floating point); `31-dict-trained.gts` is expected to
//! reproduce on the SAME authoring platform but MAY differ across platforms
//! because FastCOVER's scoring involves transcendental floating point (see
//! `crates/gts/src/dict.rs`). `crates/rdf/tests/dict_vectors.rs` is the drift
//! guard: byte-equality for 30/32/33, fold-equality for 31.
//!
//! `compact_and_certify` is used for 30/31/32 (rather than the bare
//! `purrdf_gts::compact::compact_streamable`) so the source's detached
//! authorship signature is carried forward, bound under
//! `stream:detachedSignatureRoot`, and the pack itself carries a mandatory
//! packaging (index/head) signature — those vectors exercise the WHOLE
//! streamable-compaction + in-band-dictionary feature, not just the codec.
//! Every `.gts` output is accompanied by an `.expected.json` oracle rendered
//! from that exact file's own fold in the shared cross-engine corpus format.

use std::path::Path;

use purrdf_gts::compact::{DictPlan, DictStrategy};
use purrdf_rdf::capture_support::corpus_repo_root;
use purrdf_rdf::gts_certify::compact_and_certify;
use purrdf_rdf::gts_dict_vectors::{
    TIMESTAMP, expected_fold_json, fixed_source, multi_dict_pack, packaging_key,
    render_expected_json, rsyncable_plan,
};

fn write_vector_and_expected(path: &Path, bytes: &[u8]) {
    std::fs::write(path, bytes).unwrap_or_else(|err| panic!("write {}: {err}", path.display()));
    println!("wrote {} ({} bytes)", path.display(), bytes.len());

    let expected = render_expected_json(&expected_fold_json(bytes));
    let mut expected_path = path.to_path_buf();
    expected_path.set_extension("expected.json");
    std::fs::write(&expected_path, &expected)
        .unwrap_or_else(|err| panic!("write {}: {err}", expected_path.display()));
    println!(
        "wrote {} ({} bytes)",
        expected_path.display(),
        expected.len()
    );
}

fn main() {
    let source = fixed_source();
    let vectors_dir = corpus_repo_root().join("vectors");

    for (name, plan) in [
        (
            "30-dict-rawcontent.gts",
            DictPlan::single(DictStrategy::RawContent),
        ),
        (
            "31-dict-trained.gts",
            DictPlan::single(DictStrategy::Trained),
        ),
        ("32-dict-rsyncable.gts", rsyncable_plan()),
    ] {
        let (pack, _cert) = compact_and_certify(
            &source,
            plan,
            TIMESTAMP,
            false,
            (packaging_key(), "pack".to_string()),
        )
        .unwrap_or_else(|err| panic!("compaction for {name} succeeds: {err:?}"));
        write_vector_and_expected(&vectors_dir.join(name), &pack);
    }

    write_vector_and_expected(&vectors_dir.join("33-multi-dict.gts"), &multi_dict_pack());
}
