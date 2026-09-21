// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! **A restore READS the reusable class analysis; it does not compute it again.**
//!
//! A prepared product promises a consumer three things it will not have to do:
//! reparse RDF, extract shapes, and repeat the shared analysis. The first two are
//! easy to observe — the product carries a dataset and a model, and a restore that
//! parsed text would have to find text to parse. The third is not, and its failure
//! mode is uniquely quiet: a restore that re-ran the class walk and then checked its
//! own answer against a digest the product pinned would pass every existing test,
//! produce every correct report, and still do the work on every restore in every
//! process forever. Proving the analysis is RIGHT and not having to perform it are
//! different properties, and only the second is what the artifact was for.
//!
//! So this file does not assert that the analysis is carried. It OBSERVES it, by
//! constructing a product whose carried analysis is one this build's own walk would
//! never produce, and whose identity pins that analysis rather than the walk's.
//! Exactly one implementation admits such a product:
//!
//! | If `admit` … | then the forged product … |
//! |--------------|---------------------------|
//! | reads the carried body | ADMITS — the body it digests is the body the identity pins |
//! | re-derives and compares | REFUSES on `class-catalog` — the body it digests is the walk's, and the identity pins the other one |
//!
//! There is no third behaviour, and no way to pass the first row by accident.
//!
//! # The two shapes graphs
//!
//! [`SHAPES`] is the graph every product here is written from; its class analysis
//! is `{ex:Employee → 0, ex:Manager → 1}`. [`SHAPES_WITH_GHOST`] is the same graph
//! plus one node shape constraining an otherwise unreachable class, so its analysis
//! is `{ex:Employee → 0, ex:Ghost → 1, ex:Manager → 2}`. Neither graph can produce
//! the other's analysis, which is what makes the forgery a real discriminator; and
//! the extra class is inert at validation time — nothing resolves it — so a restore
//! that accepts it still answers exactly as a fresh parse does. That matters,
//! because a forgery that changed the answer would leave "did it read the body?"
//! confounded with "did it break?".
//!
//! # Forging is REPACKING, never editing
//!
//! Every construction below rebuilds the container through [`ArtifactBuilder`], so
//! every section digest, the identity region and the whole-container digest are
//! recomputed. A refusal reported here is therefore a refusal from the check under
//! test and never from the envelope noticing an edit — which would prove nothing
//! about the analysis at all.
//!
//! Everything is under `example.org`: PurRDF mints no vocabulary IRIs.

use std::sync::Arc;

use purrdf::RdfDataset;
use purrdf_core::artifact::{ArtifactBuilder, ArtifactSpec, ArtifactView, Identity};
use purrdf_shapes::engine::{PreparedShapes, parse_shapes};
use purrdf_shapes::product::{
    HostBindings, ProductDimension, ShapesProduct, ShapesProductError, ShapesProfile,
};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

// ── The container, as a reader outside the crate sees it ───────────────────────

/// The header magic of a prepared SHACL product.
const MAGIC: [u8; 8] = *b"PURRSHP1";

/// The container format version this build writes.
///
/// Re-declared here, and the value is load-bearing for this file's subject: the
/// class analysis travels inside the AST section rather than a fourth section of
/// its own precisely so this number — and the section count beside it — did not
/// have to move when the analysis started travelling.
const FORMAT_VERSION: u32 = 1;

/// The section count. Total: every kind in every product.
const SECTION_COUNT: usize = 3;

/// The artifact declaration a reader outside the crate reconstructs.
const SPEC: ArtifactSpec = ArtifactSpec::new(MAGIC, FORMAT_VERSION, SECTION_COUNT);

/// The section carrying the product's self-description.
const SECTION_IDENTITY: u32 = 0;

/// The section carrying the shapes dataset.
const SECTION_DATASET: u32 = 1;

/// The section carrying the declarative model and the reusable class analysis.
const SECTION_AST: u32 = 2;

/// The identity component the class analysis is bound by.
const CLASS_CATALOG_LABEL: &str = "class-catalog";

// ── The fixtures ────────────────────────────────────────────────────────────────

/// The shapes graph every product here is written from.
///
/// Two classes, reached by the two different routes the walk has to cover: a
/// `sh:targetClass` on the node shape and a `sh:class` constraint on a property
/// shape. Two is the minimum that makes a POSITION observable at all — with one
/// class every arrangement is the same arrangement.
const SHAPES: &str = r"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/ns#> .

ex:EmployeeShape a sh:NodeShape ;
    sh:targetClass ex:Employee ;
    sh:property [ sh:path ex:manager ; sh:class ex:Manager ] .
";

/// [`SHAPES`] plus one node shape naming a class nothing else reaches.
///
/// The added shape targets a node the data graph does not contain, so it changes
/// the class analysis and nothing else: a preparation from this graph validates
/// [`DATA`] exactly as a preparation from [`SHAPES`] does.
const SHAPES_WITH_GHOST: &str = r"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/ns#> .

ex:EmployeeShape a sh:NodeShape ;
    sh:targetClass ex:Employee ;
    sh:property [ sh:path ex:manager ; sh:class ex:Manager ] .

ex:AuditShape a sh:NodeShape ;
    sh:targetNode ex:nobody ;
    sh:class ex:Ghost .
";

/// [`SHAPES`]'s class analysis, as class IRIs in the order the encoding sorts them.
const CLASSES: [&str; 2] = [
    "http://example.org/ns#Employee",
    "http://example.org/ns#Manager",
];

/// [`SHAPES_WITH_GHOST`]'s class analysis, in the same sorted order.
const CLASSES_WITH_GHOST: [&str; 3] = [
    "http://example.org/ns#Employee",
    "http://example.org/ns#Ghost",
    "http://example.org/ns#Manager",
];

/// A data graph that violates the `sh:class` constraint, so every report compared
/// below carries a result and no comparison can pass by both sides being empty.
const DATA: &str = r"
@prefix ex: <http://example.org/ns#> .
ex:n1 a ex:Employee ; ex:manager ex:m1 .
ex:m1 a ex:Other .
";

// ── Reading and rebuilding a product from outside the crate ────────────────────

/// Pack `turtle` as a prepared product.
fn product_of(turtle: &str) -> Vec<u8> {
    let shapes = parse_shapes(turtle, None).expect("the fixture shapes parse");
    PreparedShapes::new(Arc::new(shapes))
        .to_product(&ShapesProfile::CORE)
        .expect("the fixture is representable as a product")
}

/// One section's bytes, copied out of a product.
fn section_of(bytes: &[u8], kind: u32) -> Vec<u8> {
    ArtifactView::from_bytes(SPEC, bytes)
        .expect("the product's envelope opens")
        .section(kind)
        .expect("every section kind is present in every product")
        .to_vec()
}

/// One identity component's value, copied out of a product.
fn component_of(bytes: &[u8], label: &str) -> Vec<u8> {
    ArtifactView::from_bytes(SPEC, bytes)
        .expect("the product's envelope opens")
        .identity()
        .components()
        .iter()
        .find(|component| component.label() == label)
        .unwrap_or_else(|| panic!("the identity carries no component labelled {label}"))
        .value()
        .to_vec()
}

/// Rebuild a product with a replacement AST section, and optionally a replacement
/// `class-catalog` identity component.
///
/// A full repack: every digest is recomputed, so nothing that refuses a product
/// built here is refusing an edit the envelope could have caught on its own.
fn reframe(bytes: &[u8], ast: &[u8], class_catalog: Option<&[u8]>) -> Vec<u8> {
    let view = ArtifactView::from_bytes(SPEC, bytes).expect("the product's envelope opens");

    let mut identity = Identity::new();
    let mut replaced = class_catalog.is_none();
    for component in view.identity().components() {
        match class_catalog {
            Some(value) if component.label() == CLASS_CATALOG_LABEL => {
                identity.push(CLASS_CATALOG_LABEL, value);
                replaced = true;
            }
            _ => {
                identity.push(component.label(), component.value());
            }
        }
    }
    assert!(
        replaced,
        "the identity carries no component labelled {CLASS_CATALOG_LABEL}, so the forgery is \
         replacing nothing and would prove nothing"
    );

    let mut builder = ArtifactBuilder::new(SPEC);
    builder
        .identity(identity)
        .section(
            SECTION_IDENTITY,
            view.section(SECTION_IDENTITY).expect("identity section"),
        )
        .section(
            SECTION_DATASET,
            view.section(SECTION_DATASET).expect("dataset section"),
        )
        .section(SECTION_AST, ast);
    builder.build_bytes().expect("the repack frames")
}

// ── The class-analysis tail, written from the format's own definition ──────────

/// Append a LEB128 varint — the one integer form the AST stream uses.
fn write_varint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let byte = u8::try_from(value & 0x7f).expect("seven bits fit in a byte");
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

/// The AST section's final field: `count`, then `(iri, position)` sorted by IRI.
///
/// Written here from the format's own definition rather than sliced off a product,
/// so that [`the_ast_section_ends_with_the_class_analysis`] is a real statement
/// about where the analysis lives and what it says, and every forgery below is
/// built from a tail this file can account for byte by byte.
fn catalog_tail(classes: &[(&str, usize)]) -> Vec<u8> {
    let mut out = Vec::new();
    write_varint(&mut out, classes.len() as u64);
    for (class, position) in classes {
        write_varint(&mut out, class.len() as u64);
        out.extend_from_slice(class.as_bytes());
        write_varint(&mut out, *position as u64);
    }
    out
}

/// The tail for `classes`, each at the position its sorted rank gives it — the
/// arrangement the class walk itself produces.
fn ranked_tail(classes: &[&str]) -> Vec<u8> {
    let ranked: Vec<(&str, usize)> = classes
        .iter()
        .enumerate()
        .map(|(position, class)| (*class, position))
        .collect();
    catalog_tail(&ranked)
}

/// `bytes` with its trailing [`ranked_tail`] for [`CLASSES`] replaced by `tail`.
///
/// # Panics
///
/// Panics when the product's AST section does not END with that tail, which would
/// mean the analysis is no longer the last field of the stream and every forgery
/// below is splicing bytes into the middle of the model.
fn with_catalog_tail(bytes: &[u8], tail: &[u8]) -> Vec<u8> {
    let ast = section_of(bytes, SECTION_AST);
    let original = ranked_tail(&CLASSES);
    assert!(
        ast.ends_with(&original),
        "the AST section no longer ends with the fixture's class analysis, so this file is \
         splicing bytes it cannot account for"
    );
    let mut forged = ast[..ast.len() - original.len()].to_vec();
    forged.extend_from_slice(tail);
    reframe(bytes, &forged, None)
}

// ── Validating, as the comparison surface ──────────────────────────────────────

/// Validate [`DATA`] with `prepared` and render the report as canonical N-Triples.
fn report_nt(prepared: &PreparedShapes) -> String {
    let data: Arc<RdfDataset> =
        parse_turtle_to_dataset(DATA, None).expect("the fixture data parses");
    let report = prepared
        .bind_shared_dataset(data)
        .expect("the data graph binds")
        .validate()
        .expect("validation runs")
        .to_ntriples();
    assert!(
        report.contains("ClassConstraintComponent"),
        "the fixture must produce a `sh:class` violation, or every report comparison in this file \
         passes by both sides being empty: {report}"
    );
    report
}

/// The report a fresh parse of [`SHAPES`] produces — the answer every restore must
/// reproduce.
fn expected_report_nt() -> String {
    let shapes = parse_shapes(SHAPES, None).expect("the fixture shapes parse");
    report_nt(&PreparedShapes::new(Arc::new(shapes)))
}

/// Admit `bytes`, or return the refusal.
fn admit(bytes: &[u8]) -> Result<PreparedShapes, ShapesProductError> {
    ShapesProduct::open(bytes)
        .expect("the product's envelope opens")
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
}

// ── The analysis is really in there ────────────────────────────────────────────

/// The AST section ends with the fixture's class analysis, exactly once.
///
/// The precondition every other proof here rests on, and not a tautology: it says
/// the product carries the analysis as DATA a reader can point at, in the
/// arrangement the walk produces, at the place the format says it is. A codec that
/// had quietly stopped writing field 7 would fail here rather than silently making
/// every forgery below a no-op that passed.
#[test]
fn the_ast_section_ends_with_the_class_analysis() {
    let ast = section_of(&product_of(SHAPES), SECTION_AST);
    let tail = ranked_tail(&CLASSES);

    assert!(
        ast.ends_with(&tail),
        "the AST section does not end with `{{ex:Employee → 0, ex:Manager → 1}}`, so a restore has \
         no carried analysis to read"
    );
    let occurrences = ast
        .windows(tail.len())
        .filter(|window| *window == tail)
        .count();
    assert_eq!(
        occurrences, 1,
        "the class analysis occurs {occurrences} times in the AST section, so a splice cannot say \
         which copy it replaced"
    );
}

// ── The observation: a restore READS the body ──────────────────────────────────

/// **A restore uses the carried analysis verbatim, rather than re-deriving one.**
///
/// The product forged here carries [`SHAPES`]'s model and dataset — so this build's
/// class walk over it yields `{ex:Employee → 0, ex:Manager → 1}` and nothing else —
/// alongside [`SHAPES_WITH_GHOST`]'s three-class analysis, and an identity whose
/// `class-catalog` component is the digest of THAT analysis, lifted from a real
/// product of that other graph rather than invented here.
///
/// A restore that recomputed the analysis would digest its own two-class walk,
/// compare it against a pinned three-class digest, and refuse on `class-catalog`.
/// A restore that reads the carried body digests the three-class body, matches, and
/// admits. The outcome below is therefore not consistent with any implementation
/// that performs the walk — which is the claim, stated as an observation instead of
/// an assertion about code nobody re-checks.
///
/// The restored preparation must still answer exactly as a fresh parse does. The
/// third class is inert — nothing in the model resolves it — so an answer that
/// moved would mean the forgery broke something, and "it read the body" would be
/// confounded with "it fell over".
#[test]
fn a_restore_uses_the_carried_analysis_verbatim() {
    let ghost_product = product_of(SHAPES_WITH_GHOST);
    let forged = with_catalog_tail(&product_of(SHAPES), &ranked_tail(&CLASSES_WITH_GHOST));
    let forged = reframe(
        &forged,
        &section_of(&forged, SECTION_AST),
        Some(&component_of(&ghost_product, CLASS_CATALOG_LABEL)),
    );

    let restored = admit(&forged).expect(
        "the forged product carries a class analysis this build's walk would never derive, and an \
         identity that pins exactly that analysis; a restore that re-derived the walk would refuse \
         it on `class-catalog`, so admitting it is the observation that the carried body was read \
         rather than recomputed",
    );

    assert_eq!(
        report_nt(&restored),
        expected_report_nt(),
        "the restore read the carried analysis but then answered differently from a fresh parse, \
         so the observation above is confounded: the product broke rather than being read",
    );
}

/// The NEIGHBOUR of the forgery: an untouched repack still admits.
///
/// Mandatory, and it is what makes every refusal in this file meaningful. The
/// forgeries are repacks, and a repack that could not itself survive admission
/// would explain every refusal below without anything being wrong with the analysis
/// at all. This runs the same [`reframe`] over the same product with nothing
/// changed and requires a successful restore that answers like a fresh parse.
#[test]
fn an_untouched_repack_still_admits() {
    let product = product_of(SHAPES);
    let repacked = reframe(&product, &section_of(&product, SECTION_AST), None);

    assert_eq!(
        repacked, product,
        "repacking a product with nothing replaced did not reproduce its own bytes, so the forging \
         machinery in this file is changing something it does not declare",
    );
    let restored = admit(&repacked).expect("an untouched repack admits");
    assert_eq!(restored.shapes().node_shapes.len(), 1);
    assert_eq!(report_nt(&restored), expected_report_nt());
}

// ── The body is CHECKED, not trusted ───────────────────────────────────────────

/// A carried analysis the product's own identity does not pin is refused.
///
/// The mirror of [`a_restore_uses_the_carried_analysis_verbatim`], and the reason
/// reading the body is safe: the same three-class forgery, with the identity left
/// alone. Nothing about "the restore reads the body" implies it believes it, and a
/// codec that read an analysis without binding it would have swapped a re-derivation
/// for a blank cheque.
#[test]
fn a_carried_analysis_the_identity_does_not_pin_is_refused() {
    let forged = with_catalog_tail(&product_of(SHAPES), &ranked_tail(&CLASSES_WITH_GHOST));

    let refusal = admit(&forged).expect_err(
        "a carried class analysis that is not the one the product's identity pins must be refused, \
         or a restore would execute against any analysis anybody put in the section",
    );
    assert_eq!(
        refusal.dimension(),
        ProductDimension::ClassCatalog,
        "a carried analysis the identity does not pin was refused on `{}` rather than on the \
         dimension named for it: {}",
        refusal.dimension().label(),
        refusal.message(),
    );
}

/// A REARRANGED analysis — the fixture's own classes at each other's positions — is
/// refused too.
///
/// The class set is untouched here, so nothing but the positions can explain the
/// refusal. That is the point: a dataset binding indexes its resolved-term row by
/// the position the catalog assigns, so two catalogs over the same classes with
/// different assignments are different analyses, and an identity that bound only
/// the IRI set would let one pass for the other.
#[test]
fn a_rearranged_analysis_is_refused() {
    let swapped = catalog_tail(&[(CLASSES[0], 1), (CLASSES[1], 0)]);
    let forged = with_catalog_tail(&product_of(SHAPES), &swapped);

    let refusal = admit(&forged).expect_err(
        "a class analysis that gives the fixture's own classes each other's binding-row positions \
         is a different analysis and must be refused",
    );
    assert_eq!(
        refusal.dimension(),
        ProductDimension::ClassCatalog,
        "a rearranged analysis was refused on `{}` rather than on the dimension named for it: {}",
        refusal.dimension().label(),
        refusal.message(),
    );
}

// ── The decoder refuses only what it must ──────────────────────────────────────

/// An analysis whose positions are not an arrangement of its own slots is refused
/// as MALFORMED, before anything indexes by them.
///
/// Two spellings, and both are structural rather than a matter of content: a
/// repeated position leaves a slot of the binding row unwritten, and a position at
/// or past the row's length is an out-of-bounds index — an abort, which is not an
/// outcome any caller of a decoder could handle, arriving from bytes a caller
/// supplied.
#[test]
fn an_analysis_that_is_not_an_arrangement_of_its_own_slots_is_refused() {
    let product = product_of(SHAPES);
    for (what, tail) in [
        (
            "a repeated position",
            catalog_tail(&[(CLASSES[0], 0), (CLASSES[1], 0)]),
        ),
        (
            "a position past the binding row",
            catalog_tail(&[(CLASSES[0], 0), (CLASSES[1], 7)]),
        ),
        (
            "a repeated class IRI",
            catalog_tail(&[(CLASSES[0], 0), (CLASSES[0], 1)]),
        ),
    ] {
        let Err(refusal) = admit(&with_catalog_tail(&product, &tail)) else {
            panic!(
                "{what} in a carried class analysis must be refused; a restore that accepted it \
                 would hand a validation plan an index into a binding row that has no such slot"
            );
        };
        assert_eq!(
            refusal.dimension(),
            ProductDimension::Malformed,
            "{what} was refused on `{}` rather than as structurally malformed: {}",
            refusal.dimension().label(),
            refusal.message(),
        );
    }
}

/// The NEIGHBOUR of the structural refusals: a VALID arrangement that the walk
/// itself would never have chosen must reach the identity check rather than being
/// refused as malformed.
///
/// This is the over-refusal case, and it is the one that hides. The decoder could
/// cheaply insist that every class sit at its sorted rank — that IS what the walk
/// produces — and every test would pass, because no honest product carries anything
/// else. It would also be a second transcription of the derivation rule inside the
/// reader, which is the drift a derived stage id spends its whole existence
/// preventing. So the decoder checks only what the binding row's own indexing
/// requires, and content is left to the digest that already binds it: the
/// rearranged analysis is refused on `class-catalog`, NOT on `malformed`.
#[test]
fn a_valid_but_underived_arrangement_is_not_refused_as_malformed() {
    let swapped = catalog_tail(&[(CLASSES[0], 1), (CLASSES[1], 0)]);
    let refusal = admit(&with_catalog_tail(&product_of(SHAPES), &swapped))
        .expect_err("a rearranged analysis is still not the pinned one");

    assert_ne!(
        refusal.dimension(),
        ProductDimension::Malformed,
        "a permutation the walk would not have chosen was refused as structurally malformed, so \
         the decoder is re-stating the derivation rule instead of reading a catalog: {}",
        refusal.message(),
    );
}

// ── Rebuild ignores the carried analysis entirely ──────────────────────────────

/// **`rebuild` re-derives the analysis and never reads the carried one.**
///
/// The product forged here carries a class analysis that does not decode at all —
/// two classes at one position, which `admit` refuses as malformed before it can
/// reach any identity check. `rebuild` restores it anyway, and answers exactly as a
/// fresh parse does.
///
/// That asymmetry is the whole reason `rebuild` is the remedy for an unknown
/// preparation stage. The stage id covers the class walk's own source, so a product
/// `rebuild` exists to rescue is BY CONSTRUCTION one whose carried analysis came
/// from a different reachability rule; reading it would be serving a stale analysis
/// through the door built to prevent exactly that. A section this path never touches
/// cannot mislead it.
#[test]
fn rebuild_ignores_the_carried_analysis() {
    let undecodable = catalog_tail(&[(CLASSES[0], 0), (CLASSES[1], 0)]);
    let forged = with_catalog_tail(&product_of(SHAPES), &undecodable);

    let refusal = admit(&forged).expect_err("an undecodable class analysis is refused by `admit`");
    assert_eq!(
        refusal.dimension(),
        ProductDimension::Malformed,
        "an analysis that is not an arrangement of its own slots must be refused as malformed: {}",
        refusal.message(),
    );

    let rebuilt = ShapesProduct::open(&forged)
        .expect("the forged product's envelope opens")
        .rebuild(&ShapesProfile::CORE, &HostBindings::empty())
        .expect(
            "`rebuild` re-derives the shapes graph — and the analysis with it — from the dataset \
             the product carries, so a carried analysis it never reads cannot refuse it",
        );

    assert_eq!(
        report_nt(&rebuilt),
        expected_report_nt(),
        "rebuilding past an undecodable carried analysis produced a validator that answers \
         differently from a fresh parse",
    );
}

// ── The three routes to a validator still agree ────────────────────────────────

/// A fresh parse, an admitted product and a rebuilt product produce the identical
/// report.
///
/// The property the carried analysis must not have disturbed. `admit` now takes its
/// class analysis from the artifact and `rebuild` derives its own, so the two
/// restore paths no longer obtain it the same way — and two paths that obtain a
/// value differently are two paths that can come to disagree about it. This is the
/// standing check that they have not.
#[test]
fn parsing_admitting_and_rebuilding_agree() {
    let product = product_of(SHAPES);
    let parsed = expected_report_nt();

    let admitted = admit(&product).expect("the product admits in the process that wrote it");
    let rebuilt = ShapesProduct::open(&product)
        .expect("the product opens")
        .rebuild(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("the product rebuilds from the dataset it carries");

    assert_eq!(report_nt(&admitted), parsed);
    assert_eq!(report_nt(&rebuilt), parsed);
}

// ── A restored preparation still CONSTRAINS ────────────────────────────────────

/// A shapes graph whose constraints are the ones a shape lowering resolves against
/// the data graph: `sh:class`, `sh:in` and `sh:hasValue`.
///
/// Every one of them is decided by an interned identity that a preparation resolves
/// when it binds. That makes them exactly the constraints a restored preparation
/// could lose without any existing check noticing: a restore whose lowering was
/// absent would answer "conforms" for all three and produce an EMPTY report, which
/// is indistinguishable from a data graph that happens to be valid.
const LOWERED_SHAPES: &str = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/ns#> .

ex:DocumentShape a sh:NodeShape ;
    sh:targetClass ex:Document ;
    sh:property [ sh:path ex:author ; sh:class ex:Person ] ;
    sh:property [ sh:path ex:status ; sh:in ( "draft" "final" ) ] ;
    sh:property [ sh:path ex:tag ; sh:hasValue ex:required ] .
"#;

/// A data graph that violates all three of [`LOWERED_SHAPES`]'s constraints.
const LOWERED_DATA: &str = r#"
@prefix ex: <http://example.org/ns#> .

ex:d1 a ex:Document ;
    ex:author ex:notAPerson ;
    ex:status "archived" ;
    ex:tag ex:other .
ex:notAPerson a ex:Thing .
"#;

/// Validate [`LOWERED_DATA`] with `prepared`, rendered as canonical N-Triples.
fn lowered_report_nt(prepared: &PreparedShapes) -> String {
    let data: Arc<RdfDataset> =
        parse_turtle_to_dataset(LOWERED_DATA, None).expect("the fixture data parses");
    prepared
        .bind_shared_dataset(data)
        .expect("the data graph binds")
        .validate()
        .expect("validation runs")
        .to_ntriples()
}

/// **A product-admitted preparation constrains exactly as a freshly parsed one
/// does — `sh:class`, `sh:in` and `sh:hasValue` included.**
///
/// This is the check a lowered constraint form makes necessary. The lowering is
/// derived from the shape tree and memoized; a preparation restored from a product
/// reaches it by a DIFFERENT route from one parsed in this process, and a route
/// that never populated it would leave every identity-resolved constraint unable to
/// constrain. That failure is silent by construction: the report simply gets
/// shorter, and a shorter report from a validator is the same shape as a cleaner
/// data graph.
///
/// So the comparison is against a report that is asserted NON-EMPTY first, and
/// asserted to name all three components. A restored preparation that had stopped
/// constraining would produce an empty report and fail the equality; one that had
/// stopped constraining while the fixture also stopped violating would fail the
/// non-vacuity check before the comparison was ever reached.
#[test]
fn an_admitted_product_constrains_exactly_as_a_fresh_parse_does() {
    let shapes = parse_shapes(LOWERED_SHAPES, None).expect("the fixture shapes parse");
    let parsed = PreparedShapes::new(Arc::new(shapes));
    let expected = lowered_report_nt(&parsed);

    for component in [
        "ClassConstraintComponent",
        "InConstraintComponent",
        "HasValueConstraintComponent",
    ] {
        assert!(
            expected.contains(component),
            "the fixture must produce a {component} violation, or this comparison can pass by \
             both sides being empty: {expected}"
        );
    }

    let product = parsed
        .to_product(&ShapesProfile::CORE)
        .expect("the fixture is representable as a product");
    let admitted = admit(&product).expect("the product admits in the process that wrote it");

    assert_eq!(
        lowered_report_nt(&admitted),
        expected,
        "a product-admitted preparation answered differently from a fresh parse of the same \
         shapes graph, so a restore does not carry — or does not derive — everything validation \
         needs",
    );

    // …and again on a SECOND bind of the same restored preparation, because the
    // lowering is memoized on first use: a restore that derived it correctly once
    // and then read a stale or empty memo would pass the comparison above.
    assert_eq!(lowered_report_nt(&admitted), expected);
}
