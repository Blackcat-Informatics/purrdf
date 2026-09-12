// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Normative vector corpus for canonicalization profile `purrdf-rdfc12` v2.
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
    CANON_CORPUS_DIGEST, CANON_PRESENTATION_FLAT_ASSERTION_ID,
    CANON_PRESENTATION_FLAT_ASSERTION_VERSION, CANON_PRESENTATION_OVERLAY_ID,
    CANON_PRESENTATION_OVERLAY_VERSION, CANON_PROFILE_ID, CANON_PROFILE_VERSION, CanonError,
    CanonHash, CanonPresentation, RESERVED_NAMESPACE, RdfDatasetBuilder, TermPosition,
    ViewCanonError, parse_dataset, try_canonicalize_flat_view, try_canonicalize_with,
};
use sha2::{Digest, Sha256};

/// `rdf:reifies` — the real predicate a reifier binding denotes once lowered to the
/// flat-assertion presentation. Named locally (not exported by `purrdf-rdf`, which
/// mints no vocabulary): every crate that needs this literal spells it itself.
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

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

/// Parse the manifest. Blank lines and `#` comments are skipped; every other line is
/// `path <TAB> kind` with a third field iff the kind carries an expectation, because a
/// manifest that tolerates a malformed row is one that can silently drop a case.
///
/// The expectation column is required to be ABSENT (or empty) for a golden and
/// NON-EMPTY for a refusal, rather than merely present in both. Keying strictness on
/// the kind is what keeps the check from resting on a trailing tab — invisible in
/// every editor, and stripped by half of them — while still refusing a refusal row
/// whose discriminant went missing, which is the row that could otherwise pass by
/// asserting nothing.
fn load_manifest() -> Vec<Case> {
    let root = corpus_root();
    let text = std::fs::read_to_string(root.join("manifest.tsv")).expect("corpus manifest");
    let mut cases = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        assert!(
            matches!(fields.len(), 2 | 3),
            "manifest.tsv line {} is malformed: {line:?}",
            n + 1
        );
        let expectation = fields.get(2).copied().unwrap_or("");
        let expect = match fields[1] {
            "golden" => {
                assert!(
                    expectation.is_empty(),
                    "manifest.tsv line {}: a golden carries no expectation column, but \
                     this one says {expectation:?}",
                    n + 1
                );
                Expectation::Golden
            }
            "refusal" => {
                assert!(
                    !expectation.is_empty(),
                    "manifest.tsv line {}: a refusal must pin its exact discriminant",
                    n + 1
                );
                Expectation::Refusal(expectation.to_owned())
            }
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
        checked >= 9,
        "the pair relations were not all read: {checked}"
    );
}

/// Canonicalize `turtle` and return the refusal it must have produced.
fn refusal_of(turtle: &str) -> CanonError {
    let dataset = parse_dataset(turtle.as_bytes(), "text/turtle", None).expect("parses");
    match try_canonicalize_with(&dataset, CanonHash::Sha256) {
        Err(err) => err,
        Ok(canonicalized) => panic!(
            "this shape MUST be refused — it canonicalized to:\n{}",
            canonicalized.nquads
        ),
    }
}

/// The fold pair, asserted as the profile states it rather than only as bytes.
///
/// `poison-forgery.ttl` writes, as an ordinary quad, exactly the reifier row that
/// `reifier-simple.ttl`'s genuine reifier lowers to. Under profile §3.1 that quad is
/// not a forgery of the row: written in the exact shape the lowering emits, it **is**
/// that row spelled out, carries the same content, and is folded back into the
/// statement layer rather than refused — which is what makes canonicalization
/// idempotent over its own output.
///
/// This checks BOTH halves, because a test that asserted only the co-canonicalization
/// would keep passing if the lowering changed and the fixture quietly stopped spelling
/// anything real: that the genuine structure still produces that row, and that the
/// spelled row canonicalizes to it. It then checks the two NEAREST misses — one shape
/// over in each direction that the fold must NOT reach — so the fold cannot widen into
/// the hole the refusal rule exists to close.
#[test]
fn a_spelled_reifier_co_canonicalizes_with_the_row_it_spells() {
    let root = corpus_root();
    let genuine = std::fs::read(root.join("cases/reifier-simple.ttl")).expect("genuine case");
    let dataset = parse_dataset(&genuine, "text/turtle", None).expect("parses");
    let lowered = try_canonicalize_with(&dataset, CanonHash::Sha256)
        .expect("a genuine reifier canonicalizes")
        .nquads;
    let row = lowered
        .lines()
        .find(|line| line.contains("<urn:purrdf:rdfc:reifies>"))
        .unwrap_or_else(|| {
            panic!(
                "the genuine case must still lower through the sentinel, or the fold \
                 fixture no longer spells a real row: {lowered}"
            )
        });

    let spelled = std::fs::read(root.join("cases/poison-forgery.ttl")).expect("fold case");
    let dataset = parse_dataset(&spelled, "text/turtle", None).expect("parses");
    let folded = try_canonicalize_with(&dataset, CanonHash::Sha256)
        .expect("a quad in the exact emitted shape folds rather than refusing")
        .nquads;
    assert_eq!(
        folded,
        format!("{row}\n"),
        "the spelled reifier row must canonicalize to the row it spells; the genuine \
         structure produces:\n{lowered}"
    );

    // Idempotence, taken over the canonical document rather than over one row: the
    // bytes a consumer minted identity from must re-derive that identity, or the
    // identity is not one the consumer can check.
    let reparsed =
        parse_dataset(lowered.as_bytes(), "text/turtle", None).expect("canonical bytes re-parse");
    assert_eq!(
        try_canonicalize_with(&reparsed, CanonHash::Sha256)
            .expect("the canonical document canonicalizes")
            .nquads,
        lowered,
        "canonicalizing the canonical document moved its bytes"
    );

    // Near miss one: the sentinel predicate over a NON-triple object. It resembles the
    // emitted shape and is not it, so it is still reserved vocabulary.
    match refusal_of("<http://example.org/r> <urn:purrdf:rdfc:reifies> <http://example.org/o> .\n")
    {
        CanonError::ReservedVocabulary(err) => {
            assert_eq!(&*err.iri, "urn:purrdf:rdfc:reifies");
            assert_eq!(err.position, TermPosition::Predicate);
        }
        other => panic!("refused for the wrong reason: {other}"),
    }

    // Near miss two: the folded shape itself, carrying a reserved IRI in a slot the
    // fold does not consume. The fold must not become a smuggling route — the triple
    // term is swept like any other term.
    match refusal_of(
        "<http://example.org/r> <urn:purrdf:rdfc:reifies> <<( <http://example.org/s> \
         <urn:purrdf:rdfc:annotation> <http://example.org/o> )>> .\n",
    ) {
        CanonError::ReservedVocabulary(err) => {
            assert_eq!(&*err.iri, "urn:purrdf:rdfc:annotation");
        }
        other => panic!("refused for the wrong reason: {other}"),
    }
}

/// The flat-presentation counterpart of
/// [`a_spelled_reifier_co_canonicalizes_with_the_row_it_spells`]: under
/// [`try_canonicalize_flat_view`], the sentinel-spelled quad `poison-forgery.ttl`
/// carries must canonicalize to the SAME ordinary `rdf:reifies` row the genuine
/// reifier in `reifier-simple.ttl` lowers to — never to a row still carrying the
/// overlay's sentinel. This is the exact defect the CLI demonstrated: `ex:r
/// <urn:purrdf:rdfc:reifies> <<( ex:s ex:p ex:o )>>` flat-canonicalized WITH the
/// sentinel while the native reifier emitted `rdf:reifies`, splitting one row's
/// identity in two.
///
/// This is an IDENTITY law, not a differential against
/// [`purrdf_rdf::flat_rdf_quads_from_dataset`]'s pre-delegation route: that route
/// shares this exact defect (it never folds a base quad's sentinel spelling back
/// into a statement-layer row either), so comparing against it here would only
/// prove the two agree on being wrong.
#[test]
fn a_spelled_reifier_co_canonicalizes_with_the_flat_row_it_spells() {
    let root = corpus_root();
    let genuine = std::fs::read(root.join("cases/reifier-simple.ttl")).expect("genuine case");
    let dataset = parse_dataset(&genuine, "text/turtle", None).expect("parses");
    let lowered = try_canonicalize_flat_view(&*dataset, CanonHash::Sha256)
        .expect("a genuine reifier canonicalizes under the flat presentation")
        .nquads;
    let row = lowered
        .lines()
        .find(|line| line.contains(RDF_REIFIES))
        .unwrap_or_else(|| {
            panic!(
                "the genuine case must still lower to an ordinary rdf:reifies row, or \
                 the fold fixture no longer spells a real row: {lowered}"
            )
        });

    let spelled = std::fs::read(root.join("cases/poison-forgery.ttl")).expect("fold case");
    let dataset = parse_dataset(&spelled, "text/turtle", None).expect("parses");
    let folded = try_canonicalize_flat_view(&*dataset, CanonHash::Sha256)
        .expect(
            "a quad in the exact emitted shape folds rather than refusing, under the \
             flat presentation too",
        )
        .nquads;
    assert_eq!(
        folded,
        format!("{row}\n"),
        "under the flat presentation, the spelled reifier row must canonicalize to \
         the SAME row the genuine structure lowers to — the sentinel must not \
         leak and the two spellings must not split identity. Genuine structure's \
         flat form:\n{lowered}"
    );
    assert!(
        !folded.contains(RESERVED_NAMESPACE),
        "the flat presentation must never mint the reserved sentinel: {folded}"
    );

    // Idempotence, taken over the flat canonical document.
    let reparsed =
        parse_dataset(lowered.as_bytes(), "text/turtle", None).expect("canonical bytes re-parse");
    assert_eq!(
        try_canonicalize_flat_view(&*reparsed, CanonHash::Sha256)
            .expect("the flat canonical document canonicalizes")
            .nquads,
        lowered,
        "canonicalizing the flat canonical document moved its bytes"
    );

    // Near miss: the reserved namespace in a NON-foldable shape (the sentinel
    // predicate over a plain IRI object, not a triple term) is still refused,
    // typed, under the flat presentation exactly as under the overlay.
    let mut b = RdfDatasetBuilder::new();
    let r = b.intern_iri("http://example.org/r");
    let reifies = b.intern_iri("urn:purrdf:rdfc:reifies");
    let o = b.intern_iri("http://example.org/o");
    b.push_quad(r, reifies, o, None);
    let near_miss = b.freeze().expect("valid");
    match try_canonicalize_flat_view(&*near_miss, CanonHash::Sha256) {
        Err(ViewCanonError::Refused(CanonError::ReservedVocabulary(err))) => {
            assert_eq!(&*err.iri, "urn:purrdf:rdfc:reifies");
            assert_eq!(err.position, TermPosition::Predicate);
        }
        other => panic!("refused for the wrong reason: {other:?}"),
    }

    // Its neighbouring VALID case: the same subject/object but a TRIPLE-TERM object
    // instead of a plain IRI — the exact folded shape — is ADMITTED and lowers.
    let mut b = RdfDatasetBuilder::new();
    let r = b.intern_iri("http://example.org/r");
    let s = b.intern_iri("http://example.org/s");
    let p = b.intern_iri("http://example.org/p");
    let o = b.intern_iri("http://example.org/o");
    let triple = b.intern_triple(s, p, o);
    let reifies = b.intern_iri("urn:purrdf:rdfc:reifies");
    b.push_quad(r, reifies, triple, None);
    let neighbour = b.freeze().expect("valid");
    let admitted = try_canonicalize_flat_view(&*neighbour, CanonHash::Sha256)
        .expect("the exact folded shape must be admitted, not refused");
    assert!(admitted.nquads.contains(RDF_REIFIES));
    assert!(!admitted.nquads.contains(RESERVED_NAMESPACE));
}

/// The annotation-side twin of
/// [`a_spelled_reifier_co_canonicalizes_with_the_flat_row_it_spells`]:
/// `poison-sentinel-graph.trig` writes, as an ordinary quad using the ANNOTATION
/// sentinel as its GRAPH name, exactly the row a native annotation over the same
/// `(r, p, o)` lowers to. The native counterpart is built directly (rather than
/// through Turtle's `{| … |}` syntax, which binds a FRESH reifier to an asserted
/// triple rather than to an arbitrary explicit term) so its content matches
/// `poison-sentinel-graph.trig`'s `(ex:s, ex:p, ex:o)` exactly.
#[test]
fn a_spelled_annotation_co_canonicalizes_with_the_flat_row_it_spells() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/s");
    let p = b.intern_iri("http://example.org/p");
    let o = b.intern_iri("http://example.org/o");
    b.push_annotation(s, p, o);
    let native = b.freeze().expect("valid dataset");
    let lowered = try_canonicalize_flat_view(&*native, CanonHash::Sha256)
        .expect("a genuine annotation canonicalizes under the flat presentation")
        .nquads;
    assert_eq!(
        lowered, "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n",
        "the native annotation's flat form must be the ordinary quad, with no \
         sentinel and no extra graph token"
    );

    let root = corpus_root();
    let spelled = std::fs::read(root.join("cases/poison-sentinel-graph.trig")).expect("fold case");
    let dataset = parse_dataset(&spelled, "application/trig", None).expect("parses");
    let folded = try_canonicalize_flat_view(&*dataset, CanonHash::Sha256)
        .expect(
            "a quad in the exact emitted shape folds rather than refusing, under the \
             flat presentation too",
        )
        .nquads;
    assert_eq!(
        folded, lowered,
        "under the flat presentation, the spelled annotation row (the sentinel used \
         as the quad's GRAPH) must canonicalize to the SAME ordinary quad the native \
         annotation lowers to — the annotation-graph sentinel must not leak as a \
         graph name"
    );
    assert!(
        !folded.contains(RESERVED_NAMESPACE),
        "the flat presentation must never mint the reserved sentinel: {folded}"
    );
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
///
/// The presentation coordinates (profile §3.3, §9) are pinned here alongside the
/// profile id/version because a consumer's complete pin is the four coordinates
/// `(profile, version, presentation, hash)` — a presentation id or version that
/// drifted from the document silently would leave that pin unverifiable.
///
/// The fourth coordinate is also checked the OTHER way here: not just that the two
/// presentation ids/versions above are what the document says, but that a produced
/// [`purrdf_rdf::Canonicalized`] REPORTS the presentation that made it, via
/// `Canonicalized::presentation` — the field a consumer actually reads to verify its
/// pin against a value in hand, rather than only against the two constants above.
#[test]
fn the_profile_identity_is_readable_from_the_api() {
    assert_eq!(CANON_PROFILE_ID, "purrdf-rdfc12");
    assert_eq!(CANON_PROFILE_VERSION, 2);
    assert_eq!(CANON_PRESENTATION_OVERLAY_ID, "overlay");
    assert_eq!(CANON_PRESENTATION_OVERLAY_VERSION, 1);
    assert_eq!(CANON_PRESENTATION_FLAT_ASSERTION_ID, "flat-assertion");
    assert_eq!(CANON_PRESENTATION_FLAT_ASSERTION_VERSION, 1);

    let empty = RdfDatasetBuilder::new()
        .freeze()
        .expect("empty dataset is valid");
    assert_eq!(
        try_canonicalize_with(&empty, CanonHash::Sha256)
            .expect("empty dataset canonicalizes")
            .presentation,
        CanonPresentation::Overlay
    );
    assert_eq!(
        try_canonicalize_flat_view(&*empty, CanonHash::Sha256)
            .expect("empty dataset canonicalizes")
            .presentation,
        CanonPresentation::FlatAssertion
    );
}

/// §3.3's first table row: a genuine quad is unchanged by the flat presentation. A
/// dataset with no reifiers, annotations or triple terms must canonicalize to the
/// SAME bytes under either presentation, because presentation only changes how an
/// already-admitted reifier or annotation row is spelled — it has nothing to say
/// about content the overlay never touched.
#[test]
fn flat_view_leaves_an_ordinary_quad_unchanged() {
    let root = corpus_root();
    let bytes = std::fs::read(root.join("cases/plain-rdf11.ttl")).expect("plain RDF 1.1 case");
    let dataset = parse_dataset(&bytes, "text/turtle", None).expect("parses");
    let overlay = try_canonicalize_with(&dataset, CanonHash::Sha256)
        .expect("plain RDF 1.1 canonicalizes under the overlay presentation")
        .nquads;
    let flat = try_canonicalize_flat_view(&*dataset, CanonHash::Sha256)
        .expect("plain RDF 1.1 canonicalizes under the flat presentation")
        .nquads;
    assert_eq!(
        flat, overlay,
        "a dataset carrying no reifiers or annotations must produce identical bytes \
         under either presentation (profile §3.3, first table row)"
    );
}

/// §3.3's second table row: a reifier declared in a NAMED graph lowers to an
/// ordinary `rdf:reifies` row carrying that SAME graph — the graph the reifier was
/// DECLARED in, not merely the graph its underlying triple happens to be asserted
/// in. The two are made to differ on purpose: the triple is asserted as an ordinary
/// quad in one named graph, and the reifier binding over that same triple term is
/// declared in a DIFFERENT named graph, so the row landing in the declaring graph —
/// rather than the assertion's graph — is the only way both expected lines can
/// appear.
#[test]
fn a_named_graph_reifier_lowers_to_flat_rdf_reifies_in_its_declaring_graph() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/s");
    let p = b.intern_iri("http://example.org/p");
    let o = b.intern_iri("http://example.org/o");
    let r = b.intern_iri("http://example.org/r");
    let g_asserted = b.intern_iri("http://example.org/g-asserted");
    let g_declared = b.intern_iri("http://example.org/g-declared");
    b.push_quad(s, p, o, Some(g_asserted));
    let triple = b.intern_triple(s, p, o);
    b.push_reifier_in_graph(r, triple, Some(g_declared));
    let dataset = b.freeze().expect("valid dataset");
    let flat = try_canonicalize_flat_view(&*dataset, CanonHash::Sha256)
        .expect("a named-graph reifier canonicalizes under the flat presentation")
        .nquads;

    let expected_assertion_row = "<http://example.org/s> <http://example.org/p> <http://example.org/o> \
         <http://example.org/g-asserted> .";
    let expected_reifies_row = format!(
        "<http://example.org/r> <{RDF_REIFIES}> <<( <http://example.org/s> \
         <http://example.org/p> <http://example.org/o> )>> <http://example.org/g-declared> ."
    );
    assert!(
        flat.lines().any(|line| line == expected_assertion_row),
        "the base assertion must stay in its own graph, unchanged: {flat}"
    );
    assert!(
        flat.lines().any(|line| line == expected_reifies_row),
        "the reifier row must land in the graph it was DECLARED in, not the \
         assertion's graph: {flat}"
    );
    assert_eq!(
        flat.lines().count(),
        2,
        "exactly these two rows are expected, no more: {flat}"
    );
    assert!(!flat.contains(RESERVED_NAMESPACE));
}

/// §3.3's fourth table row: an annotation declared in a NAMED graph lowers to an
/// ordinary `(r, p, o)` quad carrying that SAME graph, with no sentinel and no
/// extra token — the annotation-side twin of the reifier test above.
#[test]
fn a_named_graph_annotation_lowers_to_a_flat_row_in_its_own_graph() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/s");
    let p = b.intern_iri("http://example.org/p");
    let o = b.intern_iri("http://example.org/o");
    let g = b.intern_iri("http://example.org/g");
    b.push_annotation_in_graph(s, p, o, Some(g));
    let dataset = b.freeze().expect("valid dataset");
    let flat = try_canonicalize_flat_view(&*dataset, CanonHash::Sha256)
        .expect("a named-graph annotation canonicalizes under the flat presentation")
        .nquads;
    assert_eq!(
        flat,
        "<http://example.org/s> <http://example.org/p> <http://example.org/o> \
         <http://example.org/g> .\n",
        "an annotation declared in a named graph must lower to an ordinary quad \
         carrying that SAME graph, with no annotation sentinel and no extra token \
         (profile §3.3, fourth table row)"
    );
    assert!(!flat.contains(RESERVED_NAMESPACE));
}

/// §3.3's row-level dedup: a reifier binding asserted BOTH as a declared reifier
/// AND as the literal `rdf:reifies` quad it denotes (the REAL predicate, not the
/// overlay's sentinel) must canonicalize identically to the SAME binding asserted
/// only one of those two ways, and the row must be emitted exactly once — never
/// counted twice for being spelled twice.
#[test]
fn a_doubly_spelled_reifier_row_is_emitted_once_under_the_flat_presentation() {
    let mut only_native = RdfDatasetBuilder::new();
    let s = only_native.intern_iri("http://example.org/s");
    let p = only_native.intern_iri("http://example.org/p");
    let o = only_native.intern_iri("http://example.org/o");
    let r = only_native.intern_iri("http://example.org/r");
    let triple = only_native.intern_triple(s, p, o);
    only_native.push_reifier(r, triple);
    let dataset = only_native.freeze().expect("valid dataset");
    let single = try_canonicalize_flat_view(&*dataset, CanonHash::Sha256)
        .expect("a genuine reifier canonicalizes under the flat presentation")
        .nquads;

    let mut doubly_spelled = RdfDatasetBuilder::new();
    let s2 = doubly_spelled.intern_iri("http://example.org/s");
    let p2 = doubly_spelled.intern_iri("http://example.org/p");
    let o2 = doubly_spelled.intern_iri("http://example.org/o");
    let r2 = doubly_spelled.intern_iri("http://example.org/r");
    let triple2 = doubly_spelled.intern_triple(s2, p2, o2);
    doubly_spelled.push_reifier(r2, triple2);
    let rdf_reifies = doubly_spelled.intern_iri(RDF_REIFIES);
    doubly_spelled.push_quad(r2, rdf_reifies, triple2, None);
    let dataset2 = doubly_spelled.freeze().expect("valid dataset");
    let doubled = try_canonicalize_flat_view(&*dataset2, CanonHash::Sha256)
        .expect("a reifier binding spelled both ways still canonicalizes")
        .nquads;

    assert_eq!(
        doubled, single,
        "a reifier binding asserted both as a declared reifier and as the literal \
         rdf:reifies quad it denotes must canonicalize identically to the binding \
         asserted only one of those two ways (profile §3.3)"
    );
    assert_eq!(
        doubled.lines().count(),
        1,
        "the doubly-spelled row must be emitted exactly once: {doubled}"
    );
}

/// §3.3's row-level guarantee — "no reserved namespace IRI ever participates in a
/// flat canonical document" — checked across the WHOLE golden corpus rather than
/// only the two cases that exercise the fold: every golden must still admit under
/// the flat presentation (presentation is orthogonal to admissibility), and none
/// of its flat canonical bytes may mention `RESERVED_NAMESPACE`.
#[test]
fn no_corpus_golden_leaks_the_reserved_namespace_under_the_flat_presentation() {
    let mut checked = 0usize;
    for case in load_manifest() {
        if !matches!(case.expect, Expectation::Golden) {
            continue;
        }
        let bytes =
            std::fs::read(&case.file).unwrap_or_else(|e| panic!("read {:?}: {e}", case.file));
        let dataset = parse_dataset(&bytes, media_type_for(&case.file), None)
            .unwrap_or_else(|e| panic!("{} must parse: {e}", case.rel));
        let flat = try_canonicalize_flat_view(&*dataset, CanonHash::Sha256).unwrap_or_else(|e| {
            panic!(
                "{} must canonicalize under the flat presentation too: {e:?}",
                case.rel
            )
        });
        assert!(
            !flat.nquads.contains(RESERVED_NAMESPACE),
            "{} leaked the reserved namespace under the flat presentation: {}",
            case.rel,
            flat.nquads
        );
        checked += 1;
    }
    assert!(
        checked >= 15,
        "not every corpus golden was checked under the flat presentation: {checked}"
    );
}
