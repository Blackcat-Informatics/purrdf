// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Falsifiable acceptance tests for the on-disk pack container
//! (`purrdf_core::ir::pack::container`): [`PackBuilder::build_bytes`]
//! must be byte-deterministic, its
//! output for a small fixture must match a committed golden file, and
//! [`PackView::from_bytes`] must round-trip a well-formed pack while
//! rejecting a corrupted one (a flipped section byte, a flipped magic byte)
//! fail-closed.

use std::sync::Arc;

use purrdf_core::ir::pack::container::{PackBuilder, PackError, PackView};
use purrdf_core::ir::pack::dict::PackDictError;
use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};
use sha2::{Digest, Sha256};

/// The committed golden fixture's path (see [`golden_bytes_match_committed_fixture`]
/// and [`regenerate_pack_small_golden`] below for how it was produced).
const GOLDEN_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens/pack_small.bin");

/// The SMALL, deterministic golden fixture: `example.org` IRIs only, one of
/// each term kind the pack format has to frame — an IRI-only base quad, a
/// language-tagged literal, a blank-node subject, a quoted triple term used
/// both as a base quad's object AND (via `r1`) a side-table-only reference, a
/// named graph, one reifier binding, and one statement annotation.
fn small_fixture() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();

    let s1 = b.intern_iri("http://example.org/s1");
    let p1 = b.intern_iri("http://example.org/p1");
    let o1 = b.intern_iri("http://example.org/o1");
    b.push_quad(s1, p1, o1, None);

    let s2 = b.intern_iri("http://example.org/s2");
    let p2 = b.intern_iri("http://example.org/p2");
    let lit = b.intern_literal(RdfLiteral {
        lexical_form: "hello".to_owned(),
        datatype: None,
        language: Some("en".to_owned()),
        direction: None,
    });
    b.push_quad(s2, p2, lit, None);

    let blank_s = b.intern_blank("b1", BlankScope::default());
    let p3 = b.intern_iri("http://example.org/p3");
    let o3 = b.intern_iri("http://example.org/o3");
    b.push_quad(blank_s, p3, o3, None);

    // A quoted triple term used as a base quad's object.
    let triple1 = b.intern_triple(s1, p1, o1);
    let s4 = b.intern_iri("http://example.org/s4");
    let p4 = b.intern_iri("http://example.org/p4");
    b.push_quad(s4, p4, triple1, None);

    // A named graph.
    let g1 = b.intern_iri("http://example.org/g1");
    let s5 = b.intern_iri("http://example.org/s5");
    let p5 = b.intern_iri("http://example.org/p5");
    let o5 = b.intern_iri("http://example.org/o5");
    b.push_quad(s5, p5, o5, Some(g1));

    // A reifier binding `r1 rdf:reifies <<s1 p1 o1>>` plus one annotation.
    let r1 = b.intern_iri("http://example.org/r1");
    b.push_reifier(r1, triple1);
    let ap1 = b.intern_iri("http://example.org/ap1");
    let ao1 = b.intern_iri("http://example.org/ao1");
    b.push_annotation(r1, ap1, ao1);

    b.freeze().expect("small fixture is a valid dataset")
}

/// A RICHER fixture (default + named graphs, literals, blanks, triple terms,
/// reifiers, annotations) for the determinism and round-trip/corruption
/// tests — deliberately more varied than [`small_fixture`] so those tests
/// exercise more of the format than the (intentionally small, golden-file-
/// friendly) fixture above.
fn rich_fixture() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();

    let s1 = b.intern_iri("http://example.org/s1");
    let p1 = b.intern_iri("http://example.org/p1");
    let o1 = b.intern_iri("http://example.org/o1");
    b.push_quad(s1, p1, o1, None);

    let s2 = b.intern_iri("http://example.org/s2");
    let p2 = b.intern_iri("http://example.org/p2");
    let lit_en = b.intern_literal(RdfLiteral {
        lexical_form: "hello".to_owned(),
        datatype: None,
        language: Some("en".to_owned()),
        direction: None,
    });
    b.push_quad(s2, p2, lit_en, None);

    let lit_typed = b.intern_literal(RdfLiteral {
        lexical_form: "42".to_owned(),
        datatype: Some("http://www.w3.org/2001/XMLSchema#integer".to_owned()),
        language: None,
        direction: None,
    });
    let p2b = b.intern_iri("http://example.org/p2b");
    b.push_quad(s2, p2b, lit_typed, None);

    let blank1 = b.intern_blank("blank1", BlankScope::default());
    let blank2 = b.intern_blank("blank2", BlankScope::default());
    let p3 = b.intern_iri("http://example.org/p3");
    b.push_quad(blank1, p3, blank2, None);

    let triple1 = b.intern_triple(s1, p1, o1);
    let s4 = b.intern_iri("http://example.org/s4");
    let p4 = b.intern_iri("http://example.org/p4");
    b.push_quad(s4, p4, triple1, None);

    // A second, side-table-only triple term (never a base quad's S/P/O).
    let s6 = b.intern_iri("http://example.org/s6");
    let p6 = b.intern_iri("http://example.org/p6");
    let o6 = b.intern_iri("http://example.org/o6");
    let triple2 = b.intern_triple(s6, p6, o6);

    let g1 = b.intern_iri("http://example.org/g1");
    let g2 = b.intern_iri("http://example.org/g2");
    let s5 = b.intern_iri("http://example.org/s5");
    let p5 = b.intern_iri("http://example.org/p5");
    let o5 = b.intern_iri("http://example.org/o5");
    b.push_quad(s5, p5, o5, Some(g1));
    b.push_quad(s5, p5, triple1, Some(g2));

    let r1 = b.intern_iri("http://example.org/r1");
    let r2 = b.intern_iri("http://example.org/r2");
    b.push_reifier(r1, triple1);
    // r2 binds a side-table-only triple term, inside a named graph that owns
    // no base quad of its own.
    let g3 = b.intern_iri("http://example.org/g3");
    b.push_reifier_in_graph(r2, triple2, Some(g3));

    let ap1 = b.intern_iri("http://example.org/ap1");
    let ao1 = b.intern_iri("http://example.org/ao1");
    b.push_annotation(r1, ap1, ao1);
    let ap2 = b.intern_iri("http://example.org/ap2");
    let ao2 = b.intern_iri("http://example.org/ao2");
    b.push_annotation_in_graph(r1, ap2, ao2, Some(g2));
    let ap3 = b.intern_iri("http://example.org/ap3");
    let ao3 = b.intern_iri("http://example.org/ao3");
    b.push_annotation(r2, ap3, ao3);

    b.freeze().expect("rich fixture is a valid dataset")
}

#[test]
fn build_bytes_is_deterministic_for_a_rich_dataset() {
    let dataset = rich_fixture();
    let a = PackBuilder::build_bytes(&dataset).expect("builds");
    let b = PackBuilder::build_bytes(&dataset).expect("builds");
    assert_eq!(a, b, "PackBuilder::build_bytes must be byte-deterministic");
}

/// Golden-bytes acceptance: [`small_fixture`]'s pack bytes must match the
/// committed `tests/goldens/pack_small.bin` file exactly.
///
/// # Regenerating the golden file
///
/// If (and ONLY if) an intentional, reviewed format change alters
/// [`small_fixture`]'s encoded bytes, regenerate the golden with:
///
/// ```text
/// cargo test -p purrdf-core --test pack_container regenerate_pack_small_golden -- --ignored
/// ```
///
/// then commit the updated `tests/goldens/pack_small.bin` alongside the
/// format change that caused it to differ.
#[test]
fn golden_bytes_match_committed_fixture() {
    let dataset = small_fixture();
    let bytes = PackBuilder::build_bytes(&dataset).expect("builds");
    let golden = std::fs::read(GOLDEN_PATH).unwrap_or_else(|e| {
        panic!(
            "failed to read golden fixture {GOLDEN_PATH}: {e} — see \
             `regenerate_pack_small_golden`'s doc comment to produce it"
        )
    });
    assert_eq!(
        bytes, golden,
        "PackBuilder::build_bytes(small_fixture()) no longer matches the committed golden \
         bytes at {GOLDEN_PATH} — if this is an intentional, reviewed format change, \
         regenerate via `cargo test -p purrdf-core --test pack_container \
         regenerate_pack_small_golden -- --ignored` and commit the result"
    );
}

/// NOT part of the normal test run (`#[ignore]`): (re)writes
/// `tests/goldens/pack_small.bin` from [`small_fixture`]'s current encoded
/// bytes. Run explicitly (see [`golden_bytes_match_committed_fixture`]'s doc
/// comment) only after an intentional, reviewed format change.
#[test]
#[ignore = "regenerates the committed golden fixture; run explicitly with -- --ignored"]
fn regenerate_pack_small_golden() {
    let dataset = small_fixture();
    let bytes = PackBuilder::build_bytes(&dataset).expect("builds");
    std::fs::write(GOLDEN_PATH, &bytes).expect("writes the golden fixture file");
}

#[test]
fn round_trip_exposes_capabilities_and_digest() {
    let dataset = rich_fixture();
    let bytes = PackBuilder::build_bytes(&dataset).expect("builds");

    let view = PackView::from_bytes(&bytes).expect("a freshly built pack opens");
    assert_eq!(
        view.capabilities(),
        dataset.capabilities(),
        "PackView's capabilities must match the source dataset's"
    );
    assert!(view.capabilities().named_graphs);
    assert!(view.capabilities().quoted_triples);
    assert!(view.capabilities().reifiers);
    assert!(view.capabilities().annotations);
    assert_ne!(
        view.rdfc_digest(),
        [0u8; 32],
        "a non-empty dataset must not produce an all-zero RDFC-1.0 digest"
    );

    // The dictionary/triples/side views agree on basic counts (a light sanity
    // check that the sections were framed and re-opened correctly, not a
    // full duplicate of pack_side.rs/pack_triples.rs's own set-equality
    // coverage).
    assert!(view.side().reifier_count() >= 2);
    assert!(view.side().annotation_count() >= 3);
    assert!(view.triples().named_graph_ids().count() >= 2);
}

#[test]
fn from_bytes_rejects_a_corrupted_section_body() {
    let dataset = rich_fixture();
    let bytes = PackBuilder::build_bytes(&dataset).expect("builds");

    // Flip one byte strictly inside the DICT section's body (the section
    // bytes start right after the fixed header + 3-entry directory; any
    // offset comfortably past that boundary and before the file's end lands
    // inside SOME section's body).
    let flip_at = bytes.len() / 2;
    let mut corrupted = bytes;
    corrupted[flip_at] ^= 0xFF;

    let err = PackView::from_bytes(&corrupted).expect_err("a flipped section byte must be caught");
    assert!(
        matches!(err, PackError::SectionDigestMismatch { .. }),
        "expected SectionDigestMismatch, got {err:?}"
    );
}

#[test]
fn from_bytes_rejects_a_flipped_magic_byte() {
    let dataset = rich_fixture();
    let bytes = PackBuilder::build_bytes(&dataset).expect("builds");

    let mut corrupted = bytes;
    corrupted[0] ^= 0xFF;

    let err = PackView::from_bytes(&corrupted).expect_err("a flipped magic byte must be caught");
    assert_eq!(err, PackError::BadMagic);
}

#[test]
fn from_bytes_rejects_truncated_input() {
    let dataset = small_fixture();
    let bytes = PackBuilder::build_bytes(&dataset).expect("builds");

    let err = PackView::from_bytes(&bytes[..bytes.len() - 1])
        .expect_err("a truncated buffer must be rejected");
    assert!(
        matches!(
            err,
            PackError::Truncated | PackError::SectionDigestMismatch { .. }
        ),
        "expected Truncated or SectionDigestMismatch, got {err:?}"
    );

    let err_empty =
        PackView::from_bytes(&[]).expect_err("an empty buffer must be rejected as truncated");
    assert_eq!(err_empty, PackError::Truncated);
}

/// A malformed pack whose literal datatype id points at a Blank entry (rather
/// than an Iri entry) must be REJECTED by `from_bytes` with a clean
/// `Err(PackError::Dict(PackDictError::Malformed(_)))` — never panic. Before
/// `validate_references` learned to check the referenced entry's KIND (and
/// not merely that its id was in range), this exact byte shape reached
/// `PackDict::term_value`'s `unreachable!()` on ANY later query, a DoS on
/// untrusted pack bytes.
///
/// Built end-to-end through the production surface: a real dataset, a real
/// `PackBuilder::build_bytes` encoding, then ONE targeted byte flip (the
/// literal's datatype-id varint, located by searching for its lexical form's
/// own bytes — both the real datatype id and the blank node's id are single-
/// byte LEB128 varints here, so the flip changes no length/offset anywhere
/// else), with the DICT section's stored SHA-256 recomputed so the
/// section-digest check (which runs first) does not itself catch the tamper
/// before dict validation gets a chance to.
#[test]
fn from_bytes_rejects_literal_datatype_not_referencing_an_iri() {
    let mut b = RdfDatasetBuilder::new();

    let s1 = b.intern_iri("http://example.org/s1");
    let p1 = b.intern_iri("http://example.org/p1");
    const MARKER: &str = "ZZZ_TAMPER_MARKER_LITERAL_LEXICAL_FORM_UNIQUE_0001";
    let lit = b.intern_literal(RdfLiteral {
        lexical_form: MARKER.to_owned(),
        datatype: Some("http://example.org/customtype".to_owned()),
        language: None,
        direction: None,
    });
    b.push_quad(s1, p1, lit, None);

    let blank = b.intern_blank("btamper", BlankScope::default());
    let p2 = b.intern_iri("http://example.org/p2");
    let o2 = b.intern_iri("http://example.org/o2");
    b.push_quad(blank, p2, o2, None);

    let dataset = b.freeze().expect("valid dataset");
    let bytes = PackBuilder::build_bytes(&dataset).expect("builds");

    let view = PackView::from_bytes(&bytes).expect("a freshly built pack opens");
    let datatype_id = view
        .dict()
        .id_by_value(&TermValue::Iri("http://example.org/customtype".to_owned()))
        .expect("the datatype IRI was interned as its own dictionary entry");
    let blank_id = view
        .dict()
        .id_by_value(&TermValue::Blank {
            label: "btamper".to_owned(),
            scope: BlankScope::default(),
        })
        .expect("the blank node was interned");
    assert_ne!(datatype_id, blank_id);
    let datatype_byte = u8::try_from(datatype_id)
        .expect("test fixture keeps every unified id under 128 (single-byte varint)");
    let blank_byte = u8::try_from(blank_id)
        .expect("test fixture keeps every unified id under 128 (single-byte varint)");

    // Locate the literal's lexical-form bytes verbatim in the encoded pack;
    // per `encode_record`'s layout the very next byte is the datatype id's
    // (single-byte, since `datatype_byte < 128`) varint.
    let marker_bytes = MARKER.as_bytes();
    let marker_pos = bytes
        .windows(marker_bytes.len())
        .position(|w| w == marker_bytes)
        .expect("the literal's lexical form is present verbatim in the encoded pack");
    let datatype_byte_pos = marker_pos + marker_bytes.len();

    let mut corrupted = bytes;
    assert_eq!(
        corrupted[datatype_byte_pos], datatype_byte,
        "byte immediately after the lexical form must be the literal's single-byte \
         datatype-id varint"
    );
    corrupted[datatype_byte_pos] = blank_byte;

    // Recompute the DICT section's stored SHA-256 over the tampered bytes —
    // see container.rs's module doc comment for the fixed header/directory
    // layout (DICT is directory entry 0: `kind`@64 (4B), `offset`@68 (8B),
    // `len`@76 (8B), `sha256`@84 (32B)). Without this, `from_bytes` would
    // (correctly) reject the pack as `SectionDigestMismatch` before dict
    // validation ever runs, proving nothing about THIS gap.
    let dict_offset = u64::from_le_bytes(corrupted[68..76].try_into().unwrap()) as usize;
    let dict_len = u64::from_le_bytes(corrupted[76..84].try_into().unwrap()) as usize;
    assert!(
        (dict_offset..dict_offset + dict_len).contains(&datatype_byte_pos),
        "the tampered byte must fall inside the DICT section"
    );
    let new_digest: [u8; 32] =
        Sha256::digest(&corrupted[dict_offset..dict_offset + dict_len]).into();
    corrupted[84..116].copy_from_slice(&new_digest);

    let err = PackView::from_bytes(&corrupted).expect_err(
        "a literal datatype id that resolves to a Blank (not Iri) dictionary entry must be \
         rejected, not panic",
    );
    assert!(
        matches!(err, PackError::Dict(PackDictError::Malformed(_))),
        "expected Dict(Malformed(_)), got {err:?}"
    );
}

#[test]
fn from_bytes_rejects_unsupported_version() {
    let dataset = small_fixture();
    let bytes = PackBuilder::build_bytes(&dataset).expect("builds");

    let mut corrupted = bytes;
    // Version is the u32 immediately after the 8-byte magic.
    corrupted[8..12].copy_from_slice(&99u32.to_le_bytes());

    let err = PackView::from_bytes(&corrupted).expect_err("an unknown version must be rejected");
    assert_eq!(err, PackError::UnsupportedVersion(99));
}
