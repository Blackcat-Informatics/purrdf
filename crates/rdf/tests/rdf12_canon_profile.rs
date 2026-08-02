// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Normative vector corpus for canonicalization profile `purrdf-rdfc12` v1.
//!
//! The corpus at `vectors/rdf12-canon/` is the executable half of
//! `docs/RDF12-CANON-PROFILE.md`: every clause a consumer pins is a case here, so a
//! consumer running the same corpus against a linked build gets a receipt rather
//! than a promise. It carries both halves of the contract — **goldens** that must
//! canonicalize to exact bytes, and **refusals** that must be rejected with an exact
//! typed discriminant, including the position §5.3 says must be deterministic.
//!
//! ## What the goldens do and do not prove
//!
//! The expected canonical bytes are GENERATED from this implementation, so they
//! cannot be evidence that the implementation is correct — only that it is
//! **stable**. That is deliberate and it is what a pinning corpus is for: a consumer
//! minting identity from these bytes needs to know they will not move under it, and
//! the goldens are what makes a change that moves them impossible to land quietly.
//!
//! Correctness evidence comes from elsewhere and is not duplicated here: the vendored
//! W3C `rdf-canon` suite gates the RDF 1.1 subset (`rdfc_w3c.rs`), and the overlay's
//! properties — isomorphism, reifier-count observability, the refusal rule — are
//! asserted as relations in `pairs.tsv` and as unit tests in `purrdf-core`.
//!
//! ## Regenerating
//!
//! `PURRDF_UPDATE_CANON_CORPUS=1 cargo test -p purrdf-rdf --test rdf12_canon_profile`
//!
//! Regeneration is a loud, reviewable act: it rewrites the goldens, after which
//! `python3 scripts/check-corpus-frozen.py --update` must be run and
//! `CANON_CORPUS_DIGEST` re-pinned, and per profile §7 a change that moves canonical
//! bytes REQUIRES a `CANON_PROFILE_VERSION` increment. The three-step friction is the
//! point — it makes an accidental golden refresh impossible to mistake for a no-op.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use purrdf_rdf::{
    CANON_CORPUS_DIGEST, CANON_PROFILE_ID, CANON_PROFILE_VERSION, CanonError, CanonHash,
    TermPosition, parse_dataset, try_canonicalize_with,
};
use sha2::{Digest, Sha256};

/// What the manifest says must happen to a case.
#[derive(Debug, PartialEq, Eq)]
enum Expectation {
    /// Canonicalizes; the bytes are pinned in `<stem>.canonical`.
    Golden,
    /// Refused with this exact discriminant (profile §5.2).
    Refusal(String),
}

struct Case {
    file: PathBuf,
    rel: String,
    expect: Expectation,
}

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vectors/rdf12-canon")
}

fn updating() -> bool {
    std::env::var_os("PURRDF_UPDATE_CANON_CORPUS").is_some()
}

/// Parse the manifest. Blank lines and `#` comments are skipped; every other line
/// must have exactly three tab-separated fields, because a manifest that tolerates a
/// malformed row is one that can silently drop a case.
fn load_manifest() -> Vec<Case> {
    let root = corpus_root();
    let text = std::fs::read_to_string(root.join("manifest.tsv")).expect("corpus manifest");
    let mut cases = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(
            fields.len(),
            3,
            "manifest.tsv line {} is malformed: {line:?}",
            n + 1
        );
        let expect = match fields[1] {
            "golden" => Expectation::Golden,
            "refusal" => Expectation::Refusal(fields[2].to_owned()),
            other => panic!("manifest.tsv line {}: unknown kind {other:?}", n + 1),
        };
        cases.push(Case {
            file: root.join(fields[0]),
            rel: fields[0].to_owned(),
            expect,
        });
    }
    assert!(!cases.is_empty(), "the corpus manifest is empty");
    cases
}

/// The media type a case's extension selects. Driven off the extension rather than
/// recorded in the manifest so a case cannot be listed under a syntax it is not
/// written in.
fn media_type_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("trig") => "application/trig",
        Some("ttl") => "text/turtle",
        other => panic!("unhandled corpus input extension {other:?} for {path:?}"),
    }
}

/// The canonical bytes for a case, or its refusal rendered in the manifest's
/// discriminant spelling.
fn run_case(case: &Case) -> Result<String, String> {
    let bytes = std::fs::read(&case.file).unwrap_or_else(|e| panic!("read {:?}: {e}", case.file));
    let dataset = parse_dataset(&bytes, media_type_for(&case.file), None)
        .unwrap_or_else(|e| panic!("{} must parse: {e}", case.rel));
    match try_canonicalize_with(&dataset, CanonHash::Sha256) {
        Ok(canonicalized) => Ok(canonicalized.nquads),
        Err(CanonError::ReservedVocabulary(err)) => {
            let position = match err.position {
                TermPosition::Subject => "subject",
                TermPosition::Predicate => "predicate",
                TermPosition::Object => "object",
                TermPosition::Graph => "graph",
                // The profile fixes exactly four positions; a fifth would be a
                // profile change, and naming it "unknown" in a receipt a consumer
                // pins would be worse than failing here.
                other => panic!("unspecified term position {other:?}"),
            };
            Err(format!("reserved-vocabulary {position} {}", err.iri))
        }
        Err(CanonError::BudgetExceeded(_)) => Err("budget-exceeded".to_owned()),
        Err(other) => panic!("unspecified refusal {other:?}"),
    }
}

/// Every golden canonicalizes to its pinned bytes, and every refusal is refused with
/// its pinned discriminant.
#[test]
fn the_corpus_matches_its_pinned_expectations() {
    for case in load_manifest() {
        let actual = run_case(&case);
        match (&case.expect, actual) {
            (Expectation::Golden, Ok(nquads)) => {
                let canonical = case.file.with_extension("canonical");
                let digest = case.file.with_extension("digest");
                let hex = format!("{:x}", Sha256::digest(nquads.as_bytes()));
                if updating() {
                    std::fs::write(&canonical, &nquads).expect("write golden");
                    std::fs::write(&digest, format!("{hex}\n")).expect("write digest");
                    continue;
                }
                let expected = std::fs::read_to_string(&canonical)
                    .unwrap_or_else(|e| panic!("{} has no pinned golden: {e}", case.rel));
                assert_eq!(
                    nquads, expected,
                    "{} canonicalized to different bytes than the corpus pins",
                    case.rel
                );
                let expected_hex = std::fs::read_to_string(&digest)
                    .unwrap_or_else(|e| panic!("{} has no pinned digest: {e}", case.rel));
                // The digest is pinned SEPARATELY from the bytes rather than derived
                // from them at read time. Deriving it would make the file decorative:
                // it is here so a consumer can compare a digest it computed itself
                // against one this corpus published, which requires the two to be
                // independently recorded.
                assert_eq!(
                    hex,
                    expected_hex.trim(),
                    "{} digest disagrees with its pinned value",
                    case.rel
                );
            }
            (Expectation::Refusal(want), Err(got)) => assert_eq!(
                &got, want,
                "{} was refused, but not with the discriminant the corpus pins",
                case.rel
            ),
            (Expectation::Golden, Err(got)) => {
                panic!("{} must canonicalize, but was refused: {got}", case.rel)
            }
            (Expectation::Refusal(want), Ok(_)) => panic!(
                "{} MUST be refused ({want}) — it canonicalized instead, which for a \
                 reserved-vocabulary case means the profile's collision defense is open",
                case.rel
            ),
        }
    }
}

/// The relations that hold BETWEEN cases: isomorphic inputs must agree byte for byte,
/// and inputs that differ structurally must not.
#[test]
fn the_corpus_pairs_hold_their_declared_relations() {
    let root = corpus_root();
    let text = std::fs::read_to_string(root.join("pairs.tsv")).expect("corpus pairs");
    let manifest: BTreeMap<String, Case> = load_manifest()
        .into_iter()
        .map(|c| (c.rel.clone(), c))
        .collect();
    let mut checked = 0usize;
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields.len(), 3, "pairs.tsv line {} malformed", n + 1);
        let left = run_case(&manifest[fields[0]]).expect("a paired case must canonicalize");
        let right = run_case(&manifest[fields[1]]).expect("a paired case must canonicalize");
        match fields[2] {
            "same" => assert_eq!(
                left, right,
                "{} and {} are declared isomorphic but canonicalized differently",
                fields[0], fields[1]
            ),
            "differ" => assert_ne!(
                left, right,
                "{} and {} are declared distinct but canonicalized identically — a \
                 collision the profile forbids",
                fields[0], fields[1]
            ),
            other => panic!("pairs.tsv line {}: unknown relation {other:?}", n + 1),
        }
        checked += 1;
    }
    assert!(
        checked >= 7,
        "the pair relations were not all read: {checked}"
    );
}

/// The forgery pair, asserted as the profile states it rather than only as a refusal.
///
/// `poison-forgery.ttl` asserts, as an ordinary quad, exactly the row that
/// `reifier-simple.ttl`'s genuine reifier lowers to. This checks BOTH halves: that the
/// genuine structure really does produce that row (so the fixture still reproduces the
/// attack), and that the literal assertion of it is refused (so the attack fails). A
/// test asserting only the refusal would keep passing if the lowering changed and the
/// fixture quietly stopped being a forgery at all.
#[test]
fn the_forgery_pair_does_not_co_canonicalize() {
    let root = corpus_root();
    let genuine = std::fs::read(root.join("cases/reifier-simple.ttl")).expect("genuine case");
    let dataset = parse_dataset(&genuine, "text/turtle", None).expect("parses");
    let lowered = try_canonicalize_with(&dataset, CanonHash::Sha256)
        .expect("a genuine reifier canonicalizes")
        .nquads;
    assert!(
        lowered.contains("<urn:purrdf:rdfc:reifies>"),
        "the genuine case must still lower through the sentinel, or the forgery \
         fixture no longer reproduces the attack: {lowered}"
    );

    let forged = std::fs::read(root.join("cases/poison-forgery.ttl")).expect("forgery case");
    let dataset = parse_dataset(&forged, "text/turtle", None).expect("parses");
    match try_canonicalize_with(&dataset, CanonHash::Sha256) {
        Err(CanonError::ReservedVocabulary(err)) => {
            assert_eq!(&*err.iri, "urn:purrdf:rdfc:reifies");
            assert_eq!(err.position, TermPosition::Predicate);
        }
        Ok(canonicalized) => panic!(
            "the forgery canonicalized instead of being refused; it produced:\n{}\n\
             the genuine structure produces:\n{lowered}",
            canonicalized.nquads
        ),
        Err(other) => panic!("refused for the wrong reason: {other}"),
    }
}

/// The corpus's own content-addressed identity, so a consumer can pin
/// **(profile id, profile version, corpus digest)** and verify all three against the
/// artifact it linked.
///
/// The digest is the SHA-256 of the corpus's freeze manifest — the same file
/// `scripts/check-corpus-frozen.py` maintains — so anyone can reproduce it with a
/// single `sha256sum` and without running this suite.
#[test]
fn the_corpus_digest_matches_the_constant_a_consumer_pins() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/conformance-frozen/vectors-rdf12-canon.sha256");
    let bytes = std::fs::read(&manifest).expect("the corpus freeze manifest must exist");
    let computed = format!("{:x}", Sha256::digest(&bytes));
    assert_eq!(
        computed, CANON_CORPUS_DIGEST,
        "the corpus changed without CANON_CORPUS_DIGEST being re-pinned; a consumer \
         pinning the old digest would validate against a corpus it never agreed to"
    );
}

/// The profile identity is readable from the library, not only from the document.
#[test]
fn the_profile_identity_is_readable_from_the_api() {
    assert_eq!(CANON_PROFILE_ID, "purrdf-rdfc12");
    assert_eq!(CANON_PROFILE_VERSION, 1);
}
