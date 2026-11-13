// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The emitted snapshot bundle is a pure function of the builder's content.
//!
//! The wire format is frozen, so the bytes `emit_gts` writes for a given content
//! and chain must never move with how the writer is fed. Each case pins the
//! digest of the whole bundle, taken before the emission path was restructured,
//! over content that carries IRIs, plain, language, directional and typed
//! literals, blank nodes, named graphs, reifiers with annotations and nested
//! triple terms. The chains cover the default, a named pass-through, the empty
//! chain (which embeds the payload unencoded) and an explicit rsyncable chain;
//! the large case crosses the rsyncable threshold on its own.

#[path = "support/gts_emit.rs"]
mod fixture;

struct Case {
    name: &'static str,
    quads: usize,
    chain: Option<&'static [&'static str]>,
    signed: bool,
    blobs: bool,
    digest: &'static str,
}

const CASES: &[Case] = &[
    Case {
        name: "default chain",
        quads: 200,
        chain: None,
        signed: false,
        blobs: false,
        digest: "blake3:7aba6cd613b90cdbd8d56dbb1f1e6e93593b93b4c6284697b9762a2a0d3c2b8a",
    },
    Case {
        name: "pass-through chain",
        quads: 200,
        chain: Some(&["identity"]),
        signed: false,
        blobs: false,
        digest: "blake3:1ac437d26ba1d348c21ec42b2cd2d5a31f006810795ddf99d0bc949c31265b68",
    },
    Case {
        name: "empty chain",
        quads: 200,
        chain: Some(&[]),
        signed: false,
        blobs: false,
        digest: "blake3:4e9553792b66864e836963f4510ac42fe0e49d8fb4f78ae02b8931f34e78c6a9",
    },
    Case {
        name: "explicit rsyncable chain",
        quads: 200,
        chain: Some(&["zstd-rsyncable"]),
        signed: false,
        blobs: false,
        digest: "blake3:4445fc67f0f0a85450f086ff687b0a355d88a1a11d3a3c28dcd43b4b288f8c90",
    },
    Case {
        name: "signed with blobs",
        quads: 200,
        chain: None,
        signed: true,
        blobs: true,
        digest: "blake3:147be29d8e9e0fac2aad6d8f03a6552ffad35b6d46820e778ad3ea67a9cb63d1",
    },
    Case {
        name: "payload over the rsyncable threshold",
        quads: 3000,
        chain: None,
        signed: false,
        blobs: false,
        digest: "blake3:2e70e32d8a664b4e117a0406d67502acb5bef6b20b9198d499a95ca07bfb6970",
    },
    Case {
        name: "no quads",
        quads: 0,
        chain: None,
        signed: false,
        blobs: false,
        digest: "blake3:cabde71aaf2b2acc10e3c3af2b3d6033679fec2e2a48b7331d2cd75ed2fe48f7",
    },
];

#[test]
fn emitted_bundle_bytes_are_pinned_for_every_chain() {
    let mut mismatches = Vec::new();
    for case in CASES {
        let datasets = fixture::datasets(case.quads);
        let builder = fixture::builder(&datasets);
        let chain = case
            .chain
            .map(|names| names.iter().map(|&name| name.to_owned()).collect());
        let bytes = fixture::emit(builder, chain, case.signed, case.blobs);
        let actual = purrdf_gts::writer::digest_string(&bytes);
        if actual != case.digest {
            mismatches.push(format!("{}: {actual}", case.name));
        }
    }
    assert!(
        mismatches.is_empty(),
        "emitted bytes moved from the pinned digests:\n{}",
        mismatches.join("\n")
    );
}
