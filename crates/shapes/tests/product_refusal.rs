// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The complete refusal matrix of the prepared-SHACL-product admission boundary,
//! executed from OUTSIDE the crate — through exactly the surface a host has: write
//! a product, open it, admit it, rebuild it, certify it.
//!
//! # Every dimension is tested TWICE
//!
//! This boundary is a validation surface, and the failure a validation surface
//! actually suffers from is not under-refusal — it is OVER-refusal: turning away
//! input that was fine. That defect hides perfectly. Every test passes, the
//! refusal reads as correct strictness, and nothing looks broken until a user
//! loads the product that should restore and doesn't.
//!
//! So each [`ProductDimension`] appears here as a PAIR: `refuses_<dimension>`
//! executes the case believed invalid, and `accepts_<dimension>_neighbour`
//! executes a NEIGHBOURING case that is valid and must still restore. A dimension
//! with only a negative test is an incomplete test, and the pairing is spelled in
//! the names so the pairs can be counted by reading the file.
//!
//! One dimension carries TWO pairs. `shapes-graph` names both disagreements about
//! which shapes graph is being executed: the `sh:shapesGraph` IRI a product was
//! prepared under, and — through the bound admission entry point — a whole input
//! binding that is not the one the consumer required. The second is a question
//! about the ARTIFACT rather than the environment, and no other refusal in this
//! file asks it, so it gets its own pair rather than riding the first one's.
//!
//! Four further neighbours have no refusal to pair with, and are here because they
//! are where over-refusal actually bites: an empty shapes graph, a shapes graph
//! that is 100% SHACL-SPARQL, one whose shapes are `sh:deactivated`, and one that
//! declares expression-bodied functions WHILE the host also injects natives — the
//! mixed population the identity's partitioned function fingerprints exist for.
//!
//! # How the invalid cases are built
//!
//! The container is a `purrdf_core::artifact` envelope, so this file reconstructs
//! the codec's [`ArtifactSpec`] from the two facts a reader can observe — the
//! header magic and the format version — and uses the envelope's own builder to
//! REPACK a product after mutating it. Repacking recomputes every digest, which is
//! the point: a tamper the envelope itself would refuse proves nothing about the
//! checks that come after it. Raw byte edits are used only where the claim IS
//! about the envelope (a section digest, the container digest, the trailer).

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use purrdf::{RdfDataset, TermValue};
use purrdf_core::artifact::{ArtifactBuilder, ArtifactSpec, ArtifactView, Identity};
use purrdf_shapes::engine::{PreparedShapes, parse_shapes};
use purrdf_shapes::model::BoxRoleVocab;
use purrdf_shapes::product::{
    HostBindings, ProductDimension, ShapesProduct, ShapesProductError, ShapesProfile,
};
use purrdf_shapes::shapes::{Shapes, from_dataset_with_config_and_graph};
use purrdf_shapes::text_ingest::{extract_prefixes, parse_turtle_to_dataset};
use purrdf_sparql_eval::user_fn::{self, FnPopulation};
use purrdf_sparql_eval::{
    AggregateAccumulator, AggregateRegistry, AlgebraicClass, Arity, BindingPattern,
    CustomAggregate, EvalError, PfArgs, PfArity, PfCursor, PfRow, PropertyFunction,
    PropertyFunctionRegistry, UserFunctionRegistry, Volatility,
    property_function_content_fingerprint,
};
use purrdf_testkit::prop::prelude::*;

// ── The container, as a reader outside the crate sees it ───────────────────────

/// The header magic of a prepared SHACL product. Observable from any product this
/// build writes, and re-declared here so the tamper below can put something else
/// in its place.
const MAGIC: [u8; 8] = *b"PURRSHP1";

/// The container format version this build writes.
const FORMAT_VERSION: u32 = 1;

/// The section directory is TOTAL: identity, dataset and AST, always all three.
const SECTION_COUNT: usize = 3;

/// The envelope declaration, reconstructed from the two observable facts.
const SPEC: ArtifactSpec = ArtifactSpec::new(MAGIC, FORMAT_VERSION, SECTION_COUNT);

/// The section carrying the product's self-description.
const SECTION_IDENTITY: u32 = 0;

/// The section carrying the shapes dataset.
const SECTION_DATASET: u32 = 1;

/// The section carrying the declarative model.
const SECTION_AST: u32 = 2;

/// The envelope's fixed trailer length.
const TRAILER_LEN: usize = 64;

/// The offset, within the trailer, of the recorded whole-file length.
const TRAILER_FILE_LEN_OFFSET: usize = 16;

/// The offset, within the trailer, of the stored whole-container digest.
const TRAILER_CONTAINER_DIGEST_OFFSET: usize = 24;

/// The offset of the format version field in the header.
const HEADER_VERSION_OFFSET: usize = 8;

// ── Fixtures (example.org, per the repository's fixture rule) ──────────────────

/// The prefix header most fixtures open with.
const PREFIXES: &str = r"
@prefix sh:     <http://www.w3.org/ns/shacl#> .
@prefix ex:     <http://example.org/ns#> .
@prefix rdfs:   <http://www.w3.org/2000/01/rdf-schema#> .
@prefix shnex:  <http://www.w3.org/ns/shacl-node-expr#> .
@prefix sparql: <http://www.w3.org/ns/sparql#> .
@prefix xsd:    <http://www.w3.org/2001/XMLSchema#> .
";

/// A shapes graph with a class target and a property shape.
const PLAIN_SHAPES: &str = r"
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ; sh:datatype xsd:string ] .
";

/// Two `ex:Person` nodes, one of which is missing its `ex:name`.
const PLAIN_DATA: &str = r#"
ex:alice a ex:Person ; ex:name "Alice" .
ex:bob   a ex:Person .
"#;

/// The box-role vocabulary namespace the vocabulary fixtures run under. PurRDF
/// mints no vocabulary IRIs, so this is caller-supplied configuration.
const ROLE_NS: &str = "http://example.org/roles#";

/// A shapes graph annotated with a box role under [`ROLE_NS`].
const ROLE_SHAPES: &str = r"
@prefix roles: <http://example.org/roles#> .
ex:PersonShape a sh:NodeShape ;
    roles:graphBoxRole roles:boxTBox ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] .
";

/// SHACL 1.2 SPARQL Extensions §7.2: a list-parameter declaration, whose body is
/// an expression the AST can carry. Used for the mixed-population case.
const EXPRESSION_FN_SHAPES: &str = r"
ex:incomeTotal a sh:ListParameterExpressionFunction ;
  rdfs:subClassOf sh:ListParameterExpression ;
  sh:bodyExpression [ shnex:sum [ shnex:pathValues ex:income ; shnex:focusNode [ shnex:arg 0 ] ] ] ;
  sh:parameter [ a sh:Parameter ; sh:path shnex:arg0 ; sh:nodeKind sh:IRI ] .

ex:S a sh:NodeShape ;
  sh:targetNode ex:alice, ex:bob ;
  sh:expression [ sparql:equals ( [ ex:incomeTotal ( sh:this ) ] 30 ) ] .
";

/// The data the expression-function fixture validates over.
const EXPRESSION_FN_DATA: &str = r"
ex:alice ex:income 10, 20 .
ex:bob   ex:income 5 .
";

/// One native SPARQL function IRI a host may inject.
const NATIVE_A: &str = "http://example.org/ns#nativeA";

/// A second native SPARQL function IRI. See [`NATIVE_A`].
const NATIVE_B: &str = "http://example.org/ns#nativeB";

/// The custom-aggregate IRI the aggregate pair registers under.
const AGG_IRI: &str = "http://example.org/ns#agg";

/// One host relation IRI a property-function registry may declare.
const RELATION_A: &str = "http://example.org/ns#relA";

/// A second host relation IRI. See [`RELATION_A`].
const RELATION_B: &str = "http://example.org/ns#relB";

/// The implementation identity the host-injecting fixtures are prepared under: an
/// opaque byte string standing in for whatever a real host uses to tell its own
/// builds of its native code apart — a release version, a commit digest, a build id.
const IMPL_A: &[u8] = b"example.org/host@1";

/// A SECOND build of the very same declarations. Every native it wires declares the
/// identical IRI, arity and volatility as [`IMPL_A`]'s, so no declaration
/// fingerprint can tell the two apart — which is exactly the gap the implementation
/// identity exists to close.
const IMPL_B: &[u8] = b"example.org/host@2";

// ── Parsing, preparing, restoring ──────────────────────────────────────────────

/// Parse a shapes graph from text, under the prefix header the fixtures share.
fn shapes_of(body: &str) -> Shapes {
    parse_shapes(&format!("{PREFIXES}{body}"), None).expect("the fixture shapes parse")
}

/// Freeze a data graph from text.
fn data_of(body: &str) -> Arc<RdfDataset> {
    parse_turtle_to_dataset(&format!("{PREFIXES}{body}"), None).expect("the fixture data parses")
}

/// Write a product for `body` under the only profile this build implements.
fn product_of(body: &str) -> Vec<u8> {
    product_for(shapes_of(body))
}

/// Write a product for an already-parsed shapes graph that injects NOTHING.
fn product_for(shapes: Shapes) -> Vec<u8> {
    PreparedShapes::new(Arc::new(shapes))
        .to_product(&ShapesProfile::CORE)
        .expect("the fixture is representable")
}

/// Write a product bound to the parse configuration (and registries) `host` declares.
fn to_product_for_host(body: &str, host: &HostBindings<'_>) -> Vec<u8> {
    PreparedShapes::new(Arc::new(shapes_of(body)))
        .to_product_for_host(&ShapesProfile::CORE, host)
        .expect("the fixture is representable")
}

/// Write a product for an already-parsed shapes graph that a host injected native
/// functions or custom aggregates into, binding it to the build of those
/// implementations.
fn product_for_host(shapes: Shapes, implementation_identity: &[u8]) -> Vec<u8> {
    PreparedShapes::new(Arc::new(shapes))
        .to_product_with_implementation_identity(&ShapesProfile::CORE, implementation_identity)
        .expect("the fixture is representable")
}

/// Open a product, expecting it to open.
fn opened(bytes: &[u8]) -> purrdf_shapes::product::ShapesProductView<'_> {
    ShapesProduct::open(bytes).expect("the product opens")
}

/// Open and admit `bytes` under the empty host.
fn admit(bytes: &[u8]) -> Result<PreparedShapes, ShapesProductError> {
    admit_with(bytes, &HostBindings::empty())
}

/// Open and admit `bytes` under `host`, reporting the open's refusal as the
/// admission's: a caller restoring a product sees one outcome, not two.
fn admit_with(bytes: &[u8], host: &HostBindings<'_>) -> Result<PreparedShapes, ShapesProductError> {
    ShapesProduct::open(bytes)?.admit(&ShapesProfile::CORE, host)
}

/// Open and admit `bytes` under the empty host, bound to the input binding
/// `expected` — the caller's statement of WHICH product it wanted.
fn admit_expecting(
    bytes: &[u8],
    expected: &[u8; 32],
) -> Result<PreparedShapes, ShapesProductError> {
    ShapesProduct::open(bytes)?.admit_expecting(
        &ShapesProfile::CORE,
        &HostBindings::empty(),
        expected,
    )
}

/// The input binding `bytes` declares — the selector a consumer reads off a product
/// in order to require it later, and the same value `shacl explain` renders.
fn declared_selector(bytes: &[u8]) -> [u8; 32] {
    *opened(bytes).declared_identity().digest()
}

/// Open and rebuild `bytes` under the empty host.
fn rebuild(bytes: &[u8]) -> Result<PreparedShapes, ShapesProductError> {
    ShapesProduct::open(bytes)?.rebuild(&ShapesProfile::CORE, &HostBindings::empty())
}

/// Open and certify `bytes`.
fn certify(bytes: &[u8]) -> Result<(), ShapesProductError> {
    ShapesProduct::open(bytes)?.certify()
}

/// Validate `data` with `prepared` and render the report's canonical N-Triples.
///
/// The report's RDF form is the comparison surface rather than a field-by-field
/// walk: two restores are equal exactly when the graphs they produce are the same
/// bytes.
fn report_nt(prepared: &PreparedShapes, data: &Arc<RdfDataset>) -> String {
    prepared
        .bind_shared_dataset(Arc::clone(data))
        .expect("binding the data graph")
        .validate()
        .expect("validation runs")
        .to_ntriples()
}

/// The report the PLAIN fixture produces when it is parsed rather than restored —
/// the answer every restore of that fixture has to reproduce.
fn plain_expected_report() -> String {
    let data = data_of(PLAIN_DATA);
    let report = report_nt(
        &PreparedShapes::new(Arc::new(shapes_of(PLAIN_SHAPES))),
        &data,
    );
    assert!(
        report.contains("http://example.org/ns#bob"),
        "the fixture must actually report a violation, or every comparison against it is vacuous",
    );
    report
}

/// Assert that `bytes` restores to a preparation that answers exactly what parsing
/// the PLAIN fixture answers.
fn assert_restores_plain(bytes: &[u8]) {
    let restored = admit(bytes).expect("the valid neighbour must still restore");
    assert_eq!(
        report_nt(&restored, &data_of(PLAIN_DATA)),
        plain_expected_report(),
        "a restored product must answer exactly what the parsed shapes graph answers",
    );
}

// ── Reading and rewriting the container ────────────────────────────────────────

/// One section's raw bytes, read through the envelope rather than by offset.
fn section(bytes: &[u8], kind: u32) -> Vec<u8> {
    ArtifactView::from_bytes(SPEC, bytes)
        .expect("the product's envelope opens")
        .section(kind)
        .expect("the directory is total")
        .to_vec()
}

/// One identity component's value.
fn identity_component(bytes: &[u8], label: &str) -> Vec<u8> {
    ArtifactView::from_bytes(SPEC, bytes)
        .expect("the product's envelope opens")
        .identity()
        .component(label)
        .expect("the identity carries this component")
        .to_vec()
}

/// Rewrite a product's sections into a NEW, fully self-consistent container.
///
/// Every digest is recomputed, so the result is a product the envelope accepts.
/// That is the point: a tamper the envelope itself refuses would prove nothing
/// about the checks that run after it.
fn repack(bytes: &[u8], mutate: impl FnOnce(&mut [Vec<u8>; 3])) -> Vec<u8> {
    let view = ArtifactView::from_bytes(SPEC, bytes).expect("the product's envelope opens");
    let mut sections = [
        view.section(SECTION_IDENTITY).expect("identity").to_vec(),
        view.section(SECTION_DATASET).expect("dataset").to_vec(),
        view.section(SECTION_AST).expect("ast").to_vec(),
    ];
    mutate(&mut sections);
    frame(view.identity(), &sections)
}

/// Rewrite a product with ONE identity component replaced, every section intact.
///
/// This is how a "prepared under A, executed under B" mismatch is expressed as
/// bytes: the value spliced in is a REAL component, taken from a product genuinely
/// prepared under the other configuration, never a hand-invented byte string.
fn splice_identity(bytes: &[u8], label: &str, value: &[u8]) -> Vec<u8> {
    let view = ArtifactView::from_bytes(SPEC, bytes).expect("the product's envelope opens");
    let mut identity = Identity::new();
    let mut spliced = false;
    for component in view.identity().components() {
        if component.label() == label {
            identity.push(label, value);
            spliced = true;
        } else {
            identity.push(component.label(), component.value());
        }
    }
    assert!(
        spliced,
        "the identity carries no component labelled {label}"
    );
    let sections = [
        view.section(SECTION_IDENTITY).expect("identity").to_vec(),
        view.section(SECTION_DATASET).expect("dataset").to_vec(),
        view.section(SECTION_AST).expect("ast").to_vec(),
    ];
    frame(&identity, &sections)
}

/// Frame an identity and three section bodies as a product.
fn frame(identity: &Identity, sections: &[Vec<u8>; 3]) -> Vec<u8> {
    let mut copied = Identity::new();
    for component in identity.components() {
        copied.push(component.label(), component.value());
    }
    let mut builder = ArtifactBuilder::new(SPEC);
    builder
        .identity(copied)
        .section(SECTION_IDENTITY, &sections[0])
        .section(SECTION_DATASET, &sections[1])
        .section(SECTION_AST, &sections[2]);
    builder.build_bytes().expect("the repack frames")
}

/// `bytes` with the byte at `offset` flipped — a raw edit, NOT repacked.
fn flip(bytes: &[u8], offset: usize) -> Vec<u8> {
    let mut out = bytes.to_vec();
    out[offset] ^= 0xFF;
    out
}

/// Every offset at which `needle` occurs in `haystack`.
fn occurrences(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    haystack
        .windows(needle.len())
        .enumerate()
        .filter(|(_, window)| *window == needle)
        .map(|(index, _)| index)
        .collect()
}

/// The offset at which `needle` occurs in `haystack`, requiring exactly one
/// occurrence so a tamper can never land on the wrong copy.
fn sole_offset(haystack: &[u8], needle: &[u8]) -> usize {
    let found = occurrences(haystack, needle);
    assert_eq!(
        found.len(),
        1,
        "the tamper target must occur exactly once, found {} occurrences",
        found.len(),
    );
    found[0]
}

/// Whether `haystack` contains `needle` anywhere.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

// ── Host bindings a test can hand to a restore ─────────────────────────────────

/// A registry carrying one native SPARQL function per IRI in `iris`, registered in
/// the order given.
fn natives(iris: &[&str]) -> UserFunctionRegistry {
    let mut registry = UserFunctionRegistry::new();
    for iri in iris {
        registry.register_native(
            (*iri).to_owned(),
            Arity::Exact(1),
            Volatility::Stable,
            Arc::new(|args: &[&TermValue]| Ok(args.first().map(|value| (*value).clone()))),
        );
    }
    registry
}

/// An accumulator that folds nothing: the aggregate pair is about DECLARATIONS —
/// the arity and volatility a registry states — not about what a group computes.
#[derive(Debug)]
struct NullAccumulator;

impl AggregateAccumulator for NullAccumulator {
    fn step(&mut self, _args: &[TermValue]) -> Result<(), EvalError> {
        Ok(())
    }

    fn combine(&mut self, _other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        Ok(())
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        Ok(None)
    }
}

/// A custom aggregate that declares exactly the arity it was built with.
#[derive(Debug)]
struct DeclaredAggregate {
    /// The arity this aggregate reports to the registry's fingerprint.
    arity: Arity,
}

impl CustomAggregate for DeclaredAggregate {
    fn arity(&self) -> Arity {
        self.arity
    }

    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::Commutative
    }

    fn state_bound(&self) -> u64 {
        0
    }

    fn init(&self, _scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
        Box::new(NullAccumulator)
    }
}

/// A registry declaring [`AGG_IRI`] with `arity`.
fn aggregates(arity: Arity) -> AggregateRegistry {
    let mut registry = AggregateRegistry::new();
    registry.register(AGG_IRI, Arc::new(DeclaredAggregate { arity }));
    registry
}

/// A relation that yields no rows; it exists to be DECLARED.
#[derive(Debug)]
struct EmptyRelation {
    /// The single binding pattern this relation admits.
    modes: [BindingPattern; 1],
}

/// A cursor over no rows; see [`EmptyRelation`].
#[derive(Debug)]
struct EmptyCursor;

impl PfCursor for EmptyCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(None)
    }
}

impl PropertyFunction for EmptyRelation {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        PfArity::new(1, 1)
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
        0
    }

    fn open(
        &self,
        _args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        Ok(Box::new(EmptyCursor))
    }
}

/// A property-function registry declaring each IRI in `iris`, in the order given.
fn relations(iris: &[&str]) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    for iri in iris {
        registry.register(
            (*iri).to_owned(),
            Arc::new(EmptyRelation {
                modes: [BindingPattern::from_code("bf")],
            }),
        );
    }
    registry
}

/// The identity value a product prepared against `registry` would carry at the
/// property-function row.
///
/// The CORE profile always prepares against the EMPTY relation table — host
/// relations are wiring no shapes graph can describe — so a product that requires
/// a non-empty one cannot be WRITTEN by this build. Splicing the fingerprint in is
/// what makes the "prepared against relations A, executed against relations B"
/// case expressible at all, and the value is computed by the same public function
/// the codec itself fingerprints with, never transcribed.
fn relation_fingerprint(registry: &PropertyFunctionRegistry) -> Vec<u8> {
    property_function_content_fingerprint(registry)
        .expect("a relation registry states its own declarations")
        .as_bytes()
        .to_vec()
}

// ═══════════════════════════════════════════════════════════════════════════════
// magic
// ═══════════════════════════════════════════════════════════════════════════════

/// Provoke: the leading bytes are not the magic.
fn refusal_magic() -> ShapesProductError {
    let bytes = flip(&product_of(PLAIN_SHAPES), 0);
    ShapesProduct::open(&bytes)
        .expect_err("bytes that do not open with the magic are not a product")
}

#[test]
fn refuses_magic() {
    assert_eq!(refusal_magic().dimension(), ProductDimension::Magic);

    // ...and the magic occurring SOMEWHERE is not the magic occurring at the
    // front: a reader that resynchronized on it would open arbitrary buffers.
    let mut foreign = vec![0x5A_u8; 256];
    foreign[128..136].copy_from_slice(&MAGIC);
    let error = ShapesProduct::open(&foreign).expect_err("a buried magic is not a header");
    assert_eq!(error.dimension(), ProductDimension::Magic);
}

#[test]
fn accepts_magic_neighbour() {
    // A product whose PAYLOAD carries the magic byte sequence. The bytes cannot be
    // made to begin with it — every section body opens with content the writer
    // chooses (a stage id, a pack header, a declaration count) — so the strongest
    // constructible neighbour is a product that carries the sequence inside its
    // own data, which must not confuse the header check at either end.
    let bytes = product_of(
        r"
ex:PURRSHP1Shape a sh:NodeShape ;
    sh:targetClass ex:PURRSHP1Class ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] .
",
    );
    let at = occurrences(&bytes, &MAGIC);
    assert_eq!(
        at.first(),
        Some(&0),
        "the header is the first occurrence, by construction",
    );
    assert!(
        at.iter().skip(1).any(|offset| *offset > 0),
        "the fixture must place the magic sequence inside the payload as well as the header",
    );
    admit(&bytes).expect("a product whose payload carries the magic sequence must still restore");
}

// ═══════════════════════════════════════════════════════════════════════════════
// format-version
// ═══════════════════════════════════════════════════════════════════════════════

/// Provoke: the container format version is one past the one this build decodes.
fn refusal_format_version() -> ShapesProductError {
    let mut bytes = product_of(PLAIN_SHAPES);
    bytes[HEADER_VERSION_OFFSET..HEADER_VERSION_OFFSET + 4]
        .copy_from_slice(&(FORMAT_VERSION + 1).to_le_bytes());
    ShapesProduct::open(&bytes).expect_err("a version this build does not decode is refused")
}

#[test]
fn refuses_format_version() {
    assert_eq!(
        refusal_format_version().dimension(),
        ProductDimension::FormatVersion
    );
}

#[test]
fn accepts_format_version_neighbour() {
    let bytes = product_of(PLAIN_SHAPES);
    assert_eq!(
        opened(&bytes).format_version(),
        FORMAT_VERSION,
        "the version this build writes is the version it reports",
    );
    assert_restores_plain(&bytes);
}

// ═══════════════════════════════════════════════════════════════════════════════
// stage-id
// ═══════════════════════════════════════════════════════════════════════════════

/// A product carrying a preparation stage id from some other build.
fn foreign_stage_product() -> Vec<u8> {
    repack(&product_of(PLAIN_SHAPES), |sections| {
        sections[0][..32].copy_from_slice(&[0xAB; 32]);
    })
}

/// Provoke: the memo was written against a model this build no longer has.
fn refusal_stage_id() -> ShapesProductError {
    admit(&foreign_stage_product()).expect_err("a foreign stage id must not be admitted")
}

#[test]
fn refuses_stage_id() {
    assert_eq!(refusal_stage_id().dimension(), ProductDimension::StageId);

    let foreign = foreign_stage_product();
    assert_eq!(
        opened(&foreign).stage_id(),
        &[0xAB; 32],
        "a product this build will not admit must still be able to explain itself",
    );
}

#[test]
fn accepts_stage_id_neighbour() {
    // The exact stage id this build writes admits...
    let bytes = product_of(PLAIN_SHAPES);
    assert_restores_plain(&bytes);

    // ...and the FOREIGN one is rescued by the other seam, which re-derives the
    // shapes from the authenticated dataset the product carries. Refusing `admit`
    // there is only correct because `rebuild` still answers.
    let rebuilt = rebuild(&foreign_stage_product())
        .expect("a stage id this build does not know is what rebuild exists for");
    assert_eq!(
        report_nt(&rebuilt, &data_of(PLAIN_DATA)),
        plain_expected_report(),
        "the rescued preparation must answer what the original did",
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// profile
// ═══════════════════════════════════════════════════════════════════════════════

/// A product whose self-description names a different preparation profile.
///
/// [`ShapesProfile`] is opaque — a caller cannot mint one, which is the point — so
/// the other side of this comparison has to be written into the product's own
/// identity section, in place, at the same length.
fn other_profile_product() -> Vec<u8> {
    let declared = ShapesProfile::CORE.id().as_bytes().to_vec();
    repack(&product_of(PLAIN_SHAPES), |sections| {
        let at = sole_offset(&sections[0], &declared);
        let last = at + declared.len() - 1;
        assert_ne!(
            sections[0][last], b'2',
            "the tamper must change the profile"
        );
        sections[0][last] = b'2';
    })
}

/// Provoke: the product was prepared under another profile.
fn refusal_profile() -> ShapesProductError {
    admit(&other_profile_product()).expect_err("another profile's product must not be admitted")
}

#[test]
fn refuses_profile() {
    assert_eq!(refusal_profile().dimension(), ProductDimension::Profile);

    // The profile is the one thing `rebuild` checks too: when the profile's
    // meaning moves, the carried dataset no longer means what this build would
    // make of it, so the forward-compatibility seam must not rescue it.
    let error = rebuild(&other_profile_product()).expect_err("rebuild checks the profile as well");
    assert_eq!(error.dimension(), ProductDimension::Profile);
}

#[test]
fn accepts_profile_neighbour() {
    let bytes = product_of(PLAIN_SHAPES);
    assert_eq!(
        identity_component(&bytes, "profile"),
        ShapesProfile::CORE.id().as_bytes(),
        "CORE on both sides is the one combination this build prepares and restores",
    );
    assert_restores_plain(&bytes);
    rebuild(&bytes).expect("CORE rebuilds as well as it admits");
}

// ═══════════════════════════════════════════════════════════════════════════════
// truncated
// ═══════════════════════════════════════════════════════════════════════════════

/// Provoke: the buffer ends before a structure it declared is complete.
fn refusal_truncated() -> ShapesProductError {
    let bytes = product_of(PLAIN_SHAPES);
    ShapesProduct::open(&bytes[..bytes.len() - 1]).expect_err("an interrupted write is refused")
}

#[test]
fn refuses_truncated() {
    assert_eq!(
        refusal_truncated().dimension(),
        ProductDimension::Truncated,
        "a short buffer is an interrupted write, not corruption in place",
    );
}

#[test]
fn accepts_truncated_neighbour() {
    // The whole buffer restores...
    assert_restores_plain(&product_of(PLAIN_SHAPES));

    // ...and a section with NO BYTES is valid framing rather than a truncation:
    // the directory is total, so "this section is empty" and "this buffer ends
    // early" are different facts and the envelope must not conflate them. A
    // reader that reported `truncated` here would refuse a legitimate container
    // shape the format's own totality law permits.
    let empty_section = repack(&product_of(PLAIN_SHAPES), |sections| sections[2].clear());
    let view = ShapesProduct::open(&empty_section)
        .expect("a product carrying a zero-length section still opens");
    assert_eq!(
        view.section_kinds(),
        vec![SECTION_IDENTITY, SECTION_DATASET, SECTION_AST],
        "all three kinds are present; one of them simply has no bytes",
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// trailer
// ═══════════════════════════════════════════════════════════════════════════════

/// Provoke: the trailer no longer describes the buffer it closes.
fn refusal_trailer() -> ShapesProductError {
    let bytes = product_of(PLAIN_SHAPES);
    let at = bytes.len() - TRAILER_LEN + TRAILER_FILE_LEN_OFFSET;
    ShapesProduct::open(&flip(&bytes, at))
        .expect_err("a trailer that lies about the file is refused")
}

#[test]
fn refuses_trailer() {
    assert_eq!(refusal_trailer().dimension(), ProductDimension::Trailer);

    // The trailer's magic is the other half of the same claim.
    let bytes = product_of(PLAIN_SHAPES);
    let error = ShapesProduct::open(&flip(&bytes, bytes.len() - TRAILER_LEN))
        .expect_err("a trailer that does not start with the trailer magic is refused");
    assert_eq!(error.dimension(), ProductDimension::Trailer);
}

#[test]
fn accepts_trailer_neighbour() {
    // A product whose final section ends FLUSH at the trailer — no padding
    // between them at all. The trailer check has to hold at that boundary too; a
    // reader that assumed at least one pad byte would refuse every product whose
    // last section happens to be a multiple of the alignment.
    let (bytes, ast_len) = flush_at_trailer_product();
    let ast_at = sole_offset(&bytes, &section(&bytes, SECTION_AST));
    assert_eq!(
        ast_at + ast_len + TRAILER_LEN,
        bytes.len(),
        "the fixture must leave NO padding between the last section and the trailer",
    );
    admit(&bytes).expect("a product with no trailing padding must still restore");
}

/// A product whose AST section length is a multiple of the envelope's 8-byte
/// alignment, so the trailer begins immediately after it.
///
/// Found by search rather than pinned: the AST length is a function of the model's
/// encoding, and a hard-coded fixture would silently stop exercising the flush
/// case the day the encoding gains a byte.
fn flush_at_trailer_product() -> (Vec<u8>, usize) {
    for pad in 0..16_usize {
        let name = "x".repeat(pad);
        let bytes = product_of(&format!(
            "ex:Shape{name} a sh:NodeShape ; sh:targetClass ex:Person ;
                sh:property [ sh:path ex:name ; sh:minCount 1 ] .\n"
        ));
        let ast_len = section(&bytes, SECTION_AST).len();
        if ast_len.is_multiple_of(8) {
            return (bytes, ast_len);
        }
    }
    panic!("no fixture in the family lands the AST section flush against the trailer");
}

// ═══════════════════════════════════════════════════════════════════════════════
// section-digest
// ═══════════════════════════════════════════════════════════════════════════════

/// Provoke: one payload byte differs from the digest recorded over it.
fn refusal_section_digest() -> ShapesProductError {
    let bytes = product_of(PLAIN_SHAPES);
    let at = sole_offset(&bytes, &section(&bytes, SECTION_AST));
    ShapesProduct::open(&flip(&bytes, at)).expect_err("a section edited in place is corrupt")
}

#[test]
fn refuses_section_digest() {
    assert_eq!(
        refusal_section_digest().dimension(),
        ProductDimension::SectionDigest,
    );
}

#[test]
fn accepts_section_digest_neighbour() {
    // The unflipped original: the same bytes, one edit away from the refusal
    // above, must restore and answer exactly what parsing answers.
    assert_restores_plain(&product_of(PLAIN_SHAPES));
}

// ═══════════════════════════════════════════════════════════════════════════════
// container-digest
// ═══════════════════════════════════════════════════════════════════════════════

/// Provoke: the whole-container digest disagrees with the container's own bytes.
fn refusal_container_digest() -> ShapesProductError {
    let bytes = product_of(PLAIN_SHAPES);
    let at = bytes.len() - TRAILER_LEN + TRAILER_CONTAINER_DIGEST_OFFSET;
    ShapesProduct::open(&flip(&bytes, at))
        .expect_err("the outermost seal covers everything ahead of the trailer")
}

#[test]
fn refuses_container_digest() {
    assert_eq!(
        refusal_container_digest().dimension(),
        ProductDimension::ContainerDigest,
    );
}

#[test]
fn accepts_container_digest_neighbour() {
    // Unaltered — and re-framed from its own parts, which is the case that proves
    // the seal is over CONTENT rather than over a particular buffer: a product
    // rebuilt byte-for-byte from the same sections and identity must still open.
    let bytes = product_of(PLAIN_SHAPES);
    let reframed = repack(&bytes, |_| {});
    assert_eq!(
        reframed, bytes,
        "the writer is deterministic, so a repack of untouched parts is the identity",
    );
    assert_restores_plain(&reframed);
}

// ═══════════════════════════════════════════════════════════════════════════════
// dataset-identity
// ═══════════════════════════════════════════════════════════════════════════════

/// A shapes graph over different classes, for the donor identities below.
const OTHER_SHAPES: &str = r"
ex:AnimalShape a sh:NodeShape ;
    sh:targetClass ex:Animal ;
    sh:property [ sh:path ex:legs ; sh:minCount 1 ] .
";

/// A product built from one shapes dataset but CLAIMING another's canonical
/// identity.
fn dataset_identity_product() -> Vec<u8> {
    let donor = identity_component(&product_of(OTHER_SHAPES), "source-dataset");
    let bytes = product_of(PLAIN_SHAPES);
    assert_ne!(
        identity_component(&bytes, "source-dataset"),
        donor,
        "the donor must genuinely be another dataset's identity",
    );
    splice_identity(&bytes, "source-dataset", &donor)
}

/// Provoke: the shapes dataset does not canonicalize to the identity claimed.
///
/// The refusal comes from [`certify`], and it has to: the restore path takes row 0
/// from the product's own binding rather than re-canonicalizing a shapes graph's
/// blank nodes, because that computation would plausibly cost more than the parse
/// the whole feature exists to eliminate. Certification is the only statement that
/// the carried bytes canonicalize to what the binding claims.
fn refusal_dataset_identity() -> ShapesProductError {
    certify(&dataset_identity_product())
        .expect_err("certification recomputes, so it sees what admission deliberately does not")
}

#[test]
fn refuses_dataset_identity() {
    assert_eq!(
        refusal_dataset_identity().dimension(),
        ProductDimension::DatasetIdentity,
    );

    // The same product ADMITS, and that is the documented cold-path split rather
    // than a hole: moving certification onto the restore path would be caught
    // here, because this assertion would start failing.
    admit(&dataset_identity_product())
        .expect("admit must not canonicalize, so a false dataset claim is invisible to it");
}

#[test]
fn accepts_dataset_identity_neighbour() {
    // The SAME dataset re-parsed from equivalent-but-reordered Turtle: different
    // source text, different statement order, one graph. The identity is over the
    // dataset's canonical form, so it must not move — an identity that depended on
    // serialization order would refuse every product whose source was reformatted.
    let ordered = product_of(
        r#"
ex:alice a ex:Person ; ex:name "Alice" .
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] .
"#,
    );
    let reordered = product_of(
        r#"
ex:PersonShape a sh:NodeShape ;
    sh:property [ sh:minCount 1 ; sh:path ex:name ] ;
    sh:targetClass ex:Person .
ex:alice ex:name "Alice" ; a ex:Person .
"#,
    );
    assert_eq!(
        identity_component(&ordered, "source-dataset"),
        identity_component(&reordered, "source-dataset"),
        "one graph, two spellings: the canonical identity must be the same fact",
    );
    certify(&ordered).expect("the ordered product certifies");
    certify(&reordered).expect("the reordered product certifies");
    admit(&reordered).expect("the reordered product restores");
}

// ═══════════════════════════════════════════════════════════════════════════════
// shapes-graph
// ═══════════════════════════════════════════════════════════════════════════════

/// Parse the PLAIN fixture with an explicit `sh:shapesGraph` IRI.
fn shapes_with_graph(iri: Option<&str>) -> Shapes {
    let ttl = format!("{PREFIXES}{PLAIN_SHAPES}");
    let dataset = parse_turtle_to_dataset(&ttl, None).expect("the fixture parses");
    from_dataset_with_config_and_graph(
        &dataset,
        &extract_prefixes(&ttl),
        None,
        iri.map(ToOwned::to_owned),
    )
    .expect("the fixture shapes parse")
}

/// Provoke: the product was prepared against another shapes-graph IRI.
fn refusal_shapes_graph() -> ShapesProductError {
    let donor = identity_component(
        &product_for(shapes_with_graph(Some("https://example.org/shapes"))),
        "shapes-graph",
    );
    let bytes = product_for(shapes_with_graph(None));
    admit(&splice_identity(&bytes, "shapes-graph", &donor))
        .expect_err("a product prepared against another shapes graph must not restore")
}

#[test]
fn refuses_shapes_graph() {
    assert_eq!(
        refusal_shapes_graph().dimension(),
        ProductDimension::ShapesGraph,
    );
}

#[test]
fn accepts_shapes_graph_neighbour() {
    // Absent on both sides. "No shapes-graph IRI" is a configuration, not a
    // missing value, and two products that both name none must agree — a codec
    // that spelled absence ambiguously would refuse the commonest case of all.
    let left = product_for(shapes_with_graph(None));
    let right = product_for(shapes_with_graph(None));
    assert_eq!(
        identity_component(&left, "shapes-graph"),
        identity_component(&right, "shapes-graph"),
    );
    let crossed = splice_identity(
        &left,
        "shapes-graph",
        &identity_component(&right, "shapes-graph"),
    );
    assert_restores_plain(&crossed);

    // ...and a product that DOES name one restores under its own IRI.
    admit(&product_for(shapes_with_graph(Some(
        "https://example.org/shapes",
    ))))
    .expect("a product restores under the shapes-graph IRI it was prepared with");
}

// ═══════════════════════════════════════════════════════════════════════════════
// shapes-graph, as an EXPECTATION the consumer states
// ═══════════════════════════════════════════════════════════════════════════════
//
// Every other refusal in this file is a question about the executing environment:
// is this the build that wrote the memo, are these the registries, does this
// build's class walk re-derive the pinned analysis. None of them asks whether the
// bytes in hand are the ones the CONSUMER wanted, because nothing in a product
// states which product was meant — so an unbound `admit` of the wrong file
// succeeds, and returns a well-formed report about a shapes graph nobody asked
// about. `admit_expecting` is the consumer's half of that statement.

/// Provoke: the product is a perfectly valid one, and it is not the one required.
fn refusal_expected_identity() -> ShapesProductError {
    let wanted = declared_selector(&product_of(OTHER_SHAPES));
    let held = product_of(PLAIN_SHAPES);
    assert_ne!(
        declared_selector(&held),
        wanted,
        "the two fixtures must genuinely be two products, or the expectation is vacuous",
    );
    admit_expecting(&held, &wanted)
        .expect_err("a product that is not the one required must not restore")
}

#[test]
fn refuses_expected_identity() {
    assert_eq!(
        refusal_expected_identity().dimension(),
        ProductDimension::ShapesGraph,
    );

    // The gap this closes, stated as a passing assertion: the very same product
    // admits when nothing says which product was wanted. That is not a hole in
    // `admit` — it is the question `admit` cannot ask, and the reason the bound
    // entry point has to exist for a consumer to be able to ask it.
    admit(&product_of(PLAIN_SHAPES))
        .expect("an unbound admit cannot know which product was meant, and does not pretend to");
}

#[test]
fn accepts_expected_identity_neighbour() {
    // A product required to be ITSELF restores, and restores to the same answer
    // the unbound path reaches. A matrix of refusals alone is satisfied by
    // refusing everything, and an expectation nobody can satisfy is worse than no
    // expectation at all: it would push every consumer straight back to the
    // unbound call this exists to replace.
    let bytes = product_of(PLAIN_SHAPES);
    let restored = admit_expecting(&bytes, &declared_selector(&bytes))
        .expect("a product required to be itself must restore");
    assert_eq!(
        report_nt(&restored, &data_of(PLAIN_DATA)),
        plain_expected_report(),
        "a bound restore must answer exactly what the parsed shapes graph answers",
    );

    // ...and the binding is a property of the product's INPUTS, not of the bytes
    // that happen to carry them: the writer is deterministic, so a second pack of
    // the same shapes graph is required by the same selector. A selector that
    // moved per-pack would be unusable in a manifest, which is the only place
    // anyone would write one down.
    let packed_again = product_of(PLAIN_SHAPES);
    admit_expecting(&packed_again, &declared_selector(&bytes))
        .expect("two packs of one shapes graph share one input binding");
}

// ═══════════════════════════════════════════════════════════════════════════════
// prefixes
// ═══════════════════════════════════════════════════════════════════════════════

/// The prefix-pair fixtures share this body, spelled through whichever prefix the
/// document declares for `http://example.org/ns#`.
fn prefix_fixture(prefix: &str, declarations: &[&str]) -> String {
    let mut header = String::new();
    for line in declarations {
        header.push_str(line);
        header.push('\n');
    }
    format!(
        "{header}{prefix}:PersonShape a sh:NodeShape ;\n    \
         sh:targetClass {prefix}:Person ;\n    \
         sh:property [ sh:path {prefix}:name ; sh:minCount 1 ] .\n"
    )
}

/// A product for a prefix fixture.
fn prefix_product(prefix: &str, declarations: &[&str]) -> Vec<u8> {
    let ttl = prefix_fixture(prefix, declarations);
    product_for(parse_shapes(&ttl, None).expect("the prefix fixture parses"))
}

/// The `sh:` declaration every prefix fixture needs.
const SH_DECL: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .";

/// The `ex:` declaration.
const EX_DECL: &str = "@prefix ex: <http://example.org/ns#> .";

/// The same namespace under a different prefix — one prefix renamed.
const EXX_DECL: &str = "@prefix exx: <http://example.org/ns#> .";

/// Provoke: the product was prepared under a different prefix map.
fn refusal_prefixes() -> ShapesProductError {
    let donor = identity_component(&prefix_product("exx", &[SH_DECL, EXX_DECL]), "doc-prefixes");
    let bytes = prefix_product("ex", &[SH_DECL, EX_DECL]);
    assert_ne!(
        identity_component(&bytes, "doc-prefixes"),
        donor,
        "renaming a prefix must move the binding, or this provocation is vacuous",
    );
    admit(&splice_identity(&bytes, "doc-prefixes", &donor))
        .expect_err("a product prepared under another prefix map must not restore")
}

#[test]
fn refuses_prefixes() {
    assert_eq!(refusal_prefixes().dimension(), ProductDimension::Prefixes);
}

#[test]
fn accepts_prefixes_neighbour() {
    // The SAME logical prefix map, declared in a different order in the SOURCE
    // TEXT. A prefix map is a map; if the binding depended on declaration order,
    // reformatting a shapes document would invalidate every product built from it.
    let declared = prefix_product("ex", &[SH_DECL, EX_DECL]);
    let reordered = prefix_product("ex", &[EX_DECL, SH_DECL]);
    assert_eq!(
        identity_component(&declared, "doc-prefixes"),
        identity_component(&reordered, "doc-prefixes"),
        "one logical prefix map, two orderings: the binding must be the same fact",
    );
    admit(&reordered).expect("a reordered prefix header must still restore");
    admit(&splice_identity(
        &declared,
        "doc-prefixes",
        &identity_component(&reordered, "doc-prefixes"),
    ))
    .expect("the reordered map is the same map, so it admits into the other product");
}

// ═══════════════════════════════════════════════════════════════════════════════
// base
// ═══════════════════════════════════════════════════════════════════════════════

/// A product for the PLAIN fixture parsed under `base`.
fn based_product(base: Option<&str>) -> Vec<u8> {
    product_for(
        parse_shapes(&format!("{PREFIXES}{PLAIN_SHAPES}"), base).expect("the fixture parses"),
    )
}

/// Provoke: the product was prepared against a different base IRI.
fn refusal_base() -> ShapesProductError {
    let donor = identity_component(&based_product(Some("https://example.org/base/")), "base");
    let bytes = based_product(None);
    assert_ne!(
        identity_component(&bytes, "base"),
        donor,
        "a base and no base must encode distinctly, or this provocation is vacuous",
    );
    admit(&splice_identity(&bytes, "base", &donor))
        .expect_err("a product prepared under another base must not restore")
}

#[test]
fn refuses_base() {
    assert_eq!(refusal_base().dimension(), ProductDimension::Base);
}

#[test]
fn accepts_base_neighbour() {
    // No base on either side. Most shapes documents have none, so a check that
    // could not express "absent equals absent" would refuse nearly everything.
    let left = based_product(None);
    let right = based_product(None);
    assert_eq!(
        identity_component(&left, "base"),
        identity_component(&right, "base"),
    );
    assert_restores_plain(&splice_identity(
        &left,
        "base",
        &identity_component(&right, "base"),
    ));

    // ...and a product prepared WITH a base restores under that base.
    admit(&based_product(Some("https://example.org/base/")))
        .expect("a product restores under the base it was prepared with");
}

// ═══════════════════════════════════════════════════════════════════════════════
// vocabulary
// ═══════════════════════════════════════════════════════════════════════════════

/// Parse the box-role fixture under `vocab`. PurRDF mints no vocabulary IRIs, so
/// `None` means the feature is INACTIVE rather than defaulted.
fn vocab_shapes(vocab: Option<BoxRoleVocab>) -> Shapes {
    let ttl = format!("{PREFIXES}{ROLE_SHAPES}");
    let dataset = parse_turtle_to_dataset(&ttl, None).expect("the role fixture parses");
    from_dataset_with_config_and_graph(&dataset, &extract_prefixes(&ttl), vocab, None)
        .expect("the role fixture shapes parse")
}

/// The six box-role terms of [`ROLE_NS`], written in a different field order from
/// [`BoxRoleVocab::for_namespace`]'s.
///
/// The encoding sorts its terms by field, so this must fingerprint identically:
/// the vocabulary is a set of named terms, not a sequence, and a binding that
/// depended on the order a caller happened to write them in would refuse a
/// correctly configured host.
fn vocab_written_backwards() -> BoxRoleVocab {
    BoxRoleVocab {
        box_config_box: format!("{ROLE_NS}boxConfigBox"),
        box_cbox: format!("{ROLE_NS}boxCBox"),
        box_rbox: format!("{ROLE_NS}boxRBox"),
        box_tbox: format!("{ROLE_NS}boxTBox"),
        box_abox: format!("{ROLE_NS}boxABox"),
        graph_box_role: format!("{ROLE_NS}graphBoxRole"),
    }
}

/// Provoke: the product was prepared under a different box-role vocabulary.
fn refusal_vocabulary() -> ShapesProductError {
    let donor = identity_component(
        &product_for(vocab_shapes(Some(BoxRoleVocab::for_namespace(
            "http://example.org/other-roles#",
        )))),
        "box-role-vocab",
    );
    let bytes = product_for(vocab_shapes(Some(BoxRoleVocab::for_namespace(ROLE_NS))));
    assert_ne!(
        identity_component(&bytes, "box-role-vocab"),
        donor,
        "another namespace must move the binding, or this provocation is vacuous",
    );
    admit(&splice_identity(&bytes, "box-role-vocab", &donor))
        .expect_err("a product prepared under another vocabulary must not restore")
}

#[test]
fn refuses_vocabulary() {
    assert_eq!(
        refusal_vocabulary().dimension(),
        ProductDimension::Vocabulary
    );
}

#[test]
fn accepts_vocabulary_neighbour() {
    // The same six terms, written in a different order.
    let derived = product_for(vocab_shapes(Some(BoxRoleVocab::for_namespace(ROLE_NS))));
    let backwards = product_for(vocab_shapes(Some(vocab_written_backwards())));
    assert_eq!(
        identity_component(&derived, "box-role-vocab"),
        identity_component(&backwards, "box-role-vocab"),
        "one vocabulary, two spellings: the binding must be the same fact",
    );
    admit(&backwards).expect("the same terms in another order must still restore");

    // ...and `None` on both sides, which is the INACTIVE feature rather than a
    // fabricated default, and is what nearly every product carries.
    let left = product_for(vocab_shapes(None));
    let right = product_for(vocab_shapes(None));
    assert_eq!(
        identity_component(&left, "box-role-vocab"),
        identity_component(&right, "box-role-vocab"),
    );
    admit(&splice_identity(
        &left,
        "box-role-vocab",
        &identity_component(&right, "box-role-vocab"),
    ))
    .expect("two products with no vocabulary at all must agree");
}

// ═══════════════════════════════════════════════════════════════════════════════
// function-registry
// ═══════════════════════════════════════════════════════════════════════════════

/// A product prepared against a host that injects both natives, out of the build
/// [`IMPL_A`] names.
fn two_native_product() -> Vec<u8> {
    let mut shapes = shapes_of(PLAIN_SHAPES);
    shapes.functions = Arc::new(natives(&[NATIVE_A, NATIVE_B]));
    product_for_host(shapes, IMPL_A)
}

/// Provoke: the host supplies one fewer native than the product was prepared with.
fn refusal_function_registry() -> ShapesProductError {
    let fewer = natives(&[NATIVE_A]);
    let aggregates = AggregateRegistry::new();
    let relations = PropertyFunctionRegistry::new();
    admit_with(
        &two_native_product(),
        &HostBindings::without_declarations(&fewer, &aggregates, &relations, IMPL_A),
    )
    .expect_err("a host that lost a native is not the host this product was prepared against")
}

#[test]
fn refuses_function_registry() {
    assert_eq!(
        refusal_function_registry().dimension(),
        ProductDimension::FunctionRegistry,
    );
}

#[test]
fn accepts_function_registry_neighbour() {
    // The same set, registered in the opposite order. The fingerprint is over the
    // registry's CONTENT — a host wires its functions in whatever order its own
    // startup happens to run, and a binding sensitive to that would refuse the
    // same host on its next boot.
    let reordered = natives(&[NATIVE_B, NATIVE_A]);
    let aggregates = AggregateRegistry::new();
    let relations = PropertyFunctionRegistry::new();
    admit_with(
        &two_native_product(),
        &HostBindings::without_declarations(&reordered, &aggregates, &relations, IMPL_A),
    )
    .expect("the same natives in another order are the same registry");
}

// ═══════════════════════════════════════════════════════════════════════════════
// aggregate-registry
// ═══════════════════════════════════════════════════════════════════════════════

/// A product prepared against a host aggregate of arity one, out of the build
/// [`IMPL_A`] names.
fn unary_aggregate_product() -> Vec<u8> {
    let mut shapes = shapes_of(PLAIN_SHAPES);
    shapes.aggregates = Arc::new(aggregates(Arity::Exact(1)));
    product_for_host(shapes, IMPL_A)
}

/// Provoke: the host's aggregate declares a different arity.
fn refusal_aggregate_registry() -> ShapesProductError {
    let functions = UserFunctionRegistry::new();
    let binary = aggregates(Arity::Exact(2));
    let relations = PropertyFunctionRegistry::new();
    admit_with(
        &unary_aggregate_product(),
        &HostBindings::without_declarations(&functions, &binary, &relations, IMPL_A),
    )
    .expect_err("an aggregate of another arity is another aggregate")
}

#[test]
fn refuses_aggregate_registry() {
    assert_eq!(
        refusal_aggregate_registry().dimension(),
        ProductDimension::AggregateRegistry,
    );
}

#[test]
fn accepts_aggregate_registry_neighbour() {
    // The same declarations in a FRESH registry instance. A restore happens in
    // another process by definition, so the instance is never the writer's; a
    // check that compared instance identity would refuse every restore there is.
    let functions = UserFunctionRegistry::new();
    let fresh = aggregates(Arity::Exact(1));
    let relations = PropertyFunctionRegistry::new();
    admit_with(
        &unary_aggregate_product(),
        &HostBindings::without_declarations(&functions, &fresh, &relations, IMPL_A),
    )
    .expect("a different instance declaring the same aggregate is the same registry");
}

// ═══════════════════════════════════════════════════════════════════════════════
// property-function-registry
// ═══════════════════════════════════════════════════════════════════════════════

/// A product whose property-function row requires exactly the relations `iris`
/// declares. See [`relation_fingerprint`] for why this row is spliced.
fn relation_product(iris: &[&str]) -> Vec<u8> {
    let required = relation_fingerprint(&relations(iris));
    let bytes = product_of(PLAIN_SHAPES);
    assert_ne!(
        identity_component(&bytes, "property-function-registry"),
        required,
        "the CORE profile binds the EMPTY relation table, so this must genuinely differ",
    );
    splice_identity(&bytes, "property-function-registry", &required)
}

/// A parse configuration that declares one relation namespace.
///
/// Held as a `static` so a `HostBindings` borrowing it can outlive the expression
/// that built it, exactly as the empty registries above are.
static DECLARING_HOST: std::sync::LazyLock<purrdf_shapes::product::ParserOptions> =
    std::sync::LazyLock::new(|| purrdf_shapes::product::ParserOptions {
        property_fn_namespaces: vec!["http://example.org/rel/".to_owned()],
        ..purrdf_shapes::product::ParserOptions::default()
    });

/// Provoke: the host DECLARES a relation namespace the product was not written under.
///
/// No splicing is needed. A product of the CORE profile declares nothing — a
/// declaration is host wiring no shapes graph can describe — so a host that declares
/// a namespace is asking for a different parse than the one the product was written
/// with, and that is the whole condition.
fn refusal_parse_configuration() -> ShapesProductError {
    let functions = UserFunctionRegistry::new();
    let aggregates = AggregateRegistry::new();
    let relations = PropertyFunctionRegistry::new();
    admit_with(
        &product_of(PLAIN_SHAPES),
        &HostBindings::new(&functions, &aggregates, &relations, &[], &DECLARING_HOST),
    )
    .expect_err("a host declaring a namespace the product never was is a different parse")
}

/// A declared namespace is refused, and NOT declaring one is not.
///
/// The second half is the case that matters: a declaration decides which predicate
/// IRIs are calls for a whole prefix, so refusing on it is only correct if the
/// overwhelmingly common host — the one that declared nothing — still restores. An
/// over-refusal here would break every existing product.
#[test]
fn refuses_parse_configuration() {
    assert_eq!(
        refusal_parse_configuration().dimension(),
        ProductDimension::ParseConfiguration,
    );

    let functions = UserFunctionRegistry::new();
    let aggregates = AggregateRegistry::new();
    let relations = PropertyFunctionRegistry::new();
    admit_with(
        &product_of(PLAIN_SHAPES),
        &HostBindings::without_declarations(&functions, &aggregates, &relations, &[]),
    )
    .expect(
        "a host that declares nothing is the parse configuration the product was written under",
    );
}

/// Provoke: the host declares a different relation IRI than the product requires.
fn refusal_property_function_registry() -> ShapesProductError {
    let functions = UserFunctionRegistry::new();
    let aggregates = AggregateRegistry::new();
    let wrong = relations(&[RELATION_B]);
    admit_with(
        &relation_product(&[RELATION_A]),
        &HostBindings::without_declarations(&functions, &aggregates, &wrong, &[]),
    )
    .expect_err("a relation table declaring another IRI is another table")
}

#[test]
fn refuses_property_function_registry() {
    assert_eq!(
        refusal_property_function_registry().dimension(),
        ProductDimension::PropertyFunctionRegistry,
    );

    // The plain case a host actually meets: a product of the CORE profile is
    // prepared against NO relations, so wiring one is refused.
    let functions = UserFunctionRegistry::new();
    let aggregates = AggregateRegistry::new();
    let wired = relations(&[RELATION_A]);
    let error = admit_with(
        &product_of(PLAIN_SHAPES),
        &HostBindings::without_declarations(&functions, &aggregates, &wired, &[]),
    )
    .expect_err("a CORE product was not prepared against any host relation");
    assert_eq!(
        error.dimension(),
        ProductDimension::PropertyFunctionRegistry
    );
}

#[test]
fn accepts_property_function_registry_neighbour() {
    // The same IRIs, registered in the opposite order.
    let functions = UserFunctionRegistry::new();
    let aggregates = AggregateRegistry::new();
    let reordered = relations(&[RELATION_B, RELATION_A]);
    admit_with(
        &relation_product(&[RELATION_A, RELATION_B]),
        &HostBindings::without_declarations(&functions, &aggregates, &reordered, &[]),
    )
    .expect("the same relations in another order are the same table");

    // ...and an EMPTY table from a fresh instance restores a CORE product, which
    // is the case every ordinary host hits.
    let fresh = PropertyFunctionRegistry::new();
    admit_with(
        &product_of(PLAIN_SHAPES),
        &HostBindings::without_declarations(&functions, &aggregates, &fresh, &[]),
    )
    .expect("a different but equally empty relation table must still admit");
}

// ═══════════════════════════════════════════════════════════════════════════════
// the implementation identity
// ═══════════════════════════════════════════════════════════════════════════════

/// [`PLAIN_SHAPES`] with two host natives wired into it — the preparation a host
/// that injects implementations actually holds.
///
/// Built fresh on every call rather than cloned: `Shapes` owns registries full of
/// `Arc<dyn Fn>` and is deliberately not `Clone`, and a test that needs both the
/// prepared-side run and the product re-derives instead.
fn native_plain_shapes() -> Shapes {
    let mut shapes = shapes_of(PLAIN_SHAPES);
    shapes.functions = Arc::new(natives(&[NATIVE_A, NATIVE_B]));
    shapes
}

/// Provoke: the host declares every native the product requires, at the same
/// arities and the same volatilities, out of a DIFFERENT build.
///
/// This is the case no declaration fingerprint can see. `IMPL_A` and `IMPL_B` wire
/// registries that are indistinguishable by every fact a registry can state about
/// itself, so without the implementation identity this restore would succeed and
/// validate under semantics the product was never compiled against.
fn refusal_implementation_identity() -> ShapesProductError {
    let same_declarations = natives(&[NATIVE_A, NATIVE_B]);
    let aggregates = AggregateRegistry::new();
    let relations = PropertyFunctionRegistry::new();
    admit_with(
        &two_native_product(),
        &HostBindings::without_declarations(&same_declarations, &aggregates, &relations, IMPL_B),
    )
    .expect_err("another build of the same declarations is another host")
}

#[test]
fn refuses_another_implementation_build() {
    let error = refusal_implementation_identity();
    assert_eq!(
        error.dimension(),
        ProductDimension::FunctionRegistry,
        "a host row disagreement is reported on the registry dimension that carries it",
    );
    assert!(
        error.message().contains("implementation identity"),
        "the refusal must name the half that moved, got {:?}",
        error.message(),
    );

    // The gap this closes, stated as a fact rather than an argument: the two hosts
    // are identical everywhere a declaration fingerprint can look.
    let left = natives(&[NATIVE_A, NATIVE_B]);
    let right = natives(&[NATIVE_A, NATIVE_B]);
    assert_eq!(
        user_fn::content_fingerprint(&left, FnPopulation::Injected).expect("fingerprints"),
        user_fn::content_fingerprint(&right, FnPopulation::Injected).expect("fingerprints"),
        "two builds of the same declarations fingerprint alike; only the identity separates them",
    );
}

#[test]
fn accepts_the_same_implementation_build_neighbour() {
    // The host that PREPARED the product restores it, and reaches the report the
    // preparation itself produces. An identity nobody can satisfy would send every
    // host back to the unbound restore it exists to replace.
    let data = data_of(PLAIN_DATA);
    let expected = report_nt(&PreparedShapes::new(Arc::new(native_plain_shapes())), &data);

    let bytes = product_for_host(native_plain_shapes(), IMPL_A);
    let injected = natives(&[NATIVE_A, NATIVE_B]);
    let aggregates = AggregateRegistry::new();
    let relations = PropertyFunctionRegistry::new();
    let restored = admit_with(
        &bytes,
        &HostBindings::without_declarations(&injected, &aggregates, &relations, IMPL_A),
    )
    .expect("the build a product was prepared against must restore it");

    assert_eq!(
        report_nt(&restored, &data),
        expected,
        "naming the build must change which restores are refused, never what a restore answers",
    );
}

#[test]
fn accepts_an_unidentified_common_path_neighbour() {
    // The case every ordinary caller hits: nothing injected, nothing identified.
    // The three host rows must stay the bare declaration fingerprints, so a product
    // written before implementation identities existed is bound by exactly the same
    // rows — and `HostBindings::empty` must still admit it, exactly as it always has.
    let bytes = product_of(PLAIN_SHAPES);
    for label in [
        "user-functions-injected",
        "aggregate-registry",
        "property-function-registry",
    ] {
        assert_eq!(
            identity_component(&bytes, label).len(),
            32,
            "{label} must carry the bare fingerprint when the host identifies nothing",
        );
    }

    let restored = admit(&bytes).expect("the empty host still admits the common product");
    assert_eq!(
        report_nt(&restored, &data_of(PLAIN_DATA)),
        plain_expected_report(),
    );

    // ...and an explicitly EMPTY identity is the same fact as no identity at all:
    // one spelling, so there is no second branch for a caller to land on.
    let functions = UserFunctionRegistry::new();
    let aggregates = AggregateRegistry::new();
    let relations = PropertyFunctionRegistry::new();
    admit_with(
        &bytes,
        &HostBindings::without_declarations(&functions, &aggregates, &relations, &[]),
    )
    .expect("an empty implementation identity is the absent one");
}

#[test]
fn refuses_writing_a_product_that_cannot_name_its_implementations() {
    // A preparation that injects natives and names no build of them would restore
    // against any host declaring the same IRIs at the same arities. The writer
    // refuses instead of emitting a product whose host half nothing could check.
    let error = PreparedShapes::new(Arc::new(native_plain_shapes()))
        .to_product(&ShapesProfile::CORE)
        .expect_err("an injected population with no identity is not writable");
    assert_eq!(error.dimension(), ProductDimension::UnsupportedCapability);
    assert!(
        error
            .message()
            .contains("to_product_with_implementation_identity"),
        "the refusal must name the entry point that writes it, got {:?}",
        error.message(),
    );

    // A custom aggregate is an injected implementation too, and is refused the same
    // way — the rule is about the population, not about one registry.
    let mut aggregating = shapes_of(PLAIN_SHAPES);
    aggregating.aggregates = Arc::new(aggregates(Arity::Exact(1)));
    assert_eq!(
        PreparedShapes::new(Arc::new(aggregating))
            .to_product(&ShapesProfile::CORE)
            .expect_err("an injected aggregate with no identity is not writable")
            .dimension(),
        ProductDimension::UnsupportedCapability,
    );

    // ...and the EMPTY identity is refused at the identifying entry point too: it
    // is the spelling for "nothing injected", so a caller passing it there has
    // asked to identify implementations with a value that identifies nothing.
    assert_eq!(
        PreparedShapes::new(Arc::new(native_plain_shapes()))
            .to_product_with_implementation_identity(&ShapesProfile::CORE, b"")
            .expect_err("an empty identity identifies nothing")
            .dimension(),
        ProductDimension::UnsupportedCapability,
    );
}

#[test]
fn accepts_writing_an_identified_injected_population_neighbour() {
    // The same preparation, named: the refusal above is about the missing identity
    // and never about the mere presence of an injected population.
    let bytes = PreparedShapes::new(Arc::new(native_plain_shapes()))
        .to_product_with_implementation_identity(&ShapesProfile::CORE, IMPL_A)
        .expect("an injected population with an identity is writable");
    opened(&bytes);

    // ...and a shapes graph that injects NOTHING still writes through the plain
    // entry point, which is the path every ordinary caller takes.
    product_of(PLAIN_SHAPES);
}

// ═══════════════════════════════════════════════════════════════════════════════
// class-catalog
// ═══════════════════════════════════════════════════════════════════════════════

/// Provoke: the product pinned a class analysis this build does not re-derive.
fn refusal_class_catalog() -> ShapesProductError {
    let donor = identity_component(&product_of(OTHER_SHAPES), "class-catalog");
    let bytes = product_of(PLAIN_SHAPES);
    assert_ne!(
        identity_component(&bytes, "class-catalog"),
        donor,
        "another class population must move the pinned digest",
    );
    admit(&splice_identity(&bytes, "class-catalog", &donor))
        .expect_err("a product whose pinned class analysis is not this build's must not restore")
}

#[test]
fn refuses_class_catalog() {
    assert_eq!(
        refusal_class_catalog().dimension(),
        ProductDimension::ClassCatalog,
    );
}

#[test]
fn accepts_class_catalog_neighbour() {
    // A catalog with EXACTLY ONE class: the smallest non-empty analysis there is,
    // and the shape of nearly every real shapes graph. A check that only agreed
    // with itself on larger catalogs would refuse the common case.
    let one_class = product_of(
        r"
ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] .
",
    );
    let no_class = product_of(
        r"
ex:S a sh:NodeShape ; sh:targetNode ex:alice ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] .
",
    );
    assert_ne!(
        identity_component(&one_class, "class-catalog"),
        identity_component(&no_class, "class-catalog"),
        "a one-class catalog must not digest like an empty one, or this case is vacuous",
    );
    admit(&one_class).expect("a product with exactly one planned class must restore");
    admit(&no_class).expect("a product with no planned class at all must restore");
}

// ═══════════════════════════════════════════════════════════════════════════════
// unsupported-capability
// ═══════════════════════════════════════════════════════════════════════════════

/// A product whose model selects a variant tag this build does not implement.
///
/// The mutated byte is the FIRST shape's id term tag, and the assertions pin the
/// two counted fields ahead of it, so this cannot quietly become a test that
/// mutates some other field the day the stream's prologue changes.
fn unknown_tag_product() -> Vec<u8> {
    repack(&product_of(PLAIN_SHAPES), |sections| {
        let ast = &mut sections[2];
        assert_eq!(ast[0], 0, "the fixture declares no custom functions");
        assert_eq!(ast[1], 1, "the fixture carries exactly one node shape");
        assert!(
            ast[2] < 4,
            "byte 2 is the shape id's term tag, and every implemented tag is below 4",
        );
        ast[2] = 4;
    })
}

/// Provoke: the bytes ask for a capability this build does not implement.
fn refusal_unsupported_capability() -> ShapesProductError {
    admit(&unknown_tag_product()).expect_err("a variant tag this build lacks cannot be honoured")
}

#[test]
fn refuses_unsupported_capability() {
    let error = refusal_unsupported_capability();
    assert_eq!(error.dimension(), ProductDimension::UnsupportedCapability);
    assert!(
        error.message().contains("variant 4"),
        "the refusal must name the tag it refused: {error}",
    );
}

#[test]
fn accepts_unsupported_capability_neighbour() {
    // A supported-but-RARELY-EXERCISED capability: a SHACL-AF `sh:SPARQLTargetType`
    // declaration, which occupies its own field near the end of the model stream
    // and which almost no shapes graph carries. Pairing the refusal with a common
    // capability would prove only that the codec handles what it always handles;
    // the interesting question is whether the tag walk refuses the rare arms.
    let ttl = format!(
        "{PREFIXES}{}",
        r#"
ex:ByKind a sh:SPARQLTargetType ;
    sh:parameter [ sh:path ex:kind ] ;
    sh:select """SELECT ?this WHERE { ?this <http://example.org/ns#kind> $kind }""" .
ex:S a sh:NodeShape ; sh:target ex:inst ; sh:class ex:Allowed .
ex:inst a ex:ByKind ; ex:kind ex:Kind .
"#
    );
    let shapes = parse_shapes(&ttl, None).expect("the target-type fixture parses");
    assert_eq!(
        shapes.target_types.len(),
        1,
        "the fixture must actually declare a custom target type, or this case is vacuous",
    );
    admit(&product_for(shapes))
        .expect("a rarely exercised but implemented capability must still restore");

    // A second rare-but-implemented capability, and the one over-refusal bites
    // hardest on: a SHACL-AF §5 `sh:SPARQLFunction` whose body a constraint calls.
    // The declaration is not in the model a product encodes — it is re-derived from
    // the shapes dataset the product carries — so nothing about it is visible to the
    // tag walk, and a restore that dropped it would report NOTHING rather than fail.
    let ttl = format!(
        "{PREFIXES}{}",
        r#"
ex:double a sh:SPARQLFunction ;
    sh:parameter [ sh:path ex:arg ; sh:datatype xsd:integer ] ;
    sh:returnType xsd:integer ;
    sh:select """SELECT ?result WHERE { BIND(?arg * 2 AS ?result) }""" .

ex:CapShape a sh:NodeShape ;
    sh:targetClass ex:Thing ;
    sh:sparql [ sh:select """SELECT $this ?value WHERE { $this ex:n ?value . FILTER (ex:double(?value) > 10) }""" ] .
"#
    );
    let data = data_of(r#"ex:high a ex:Thing ; ex:n "7"^^xsd:integer ."#);
    let document = parse_shapes(&ttl, None).expect("the SPARQL-function fixture parses");
    let expected = report_nt(&PreparedShapes::new(Arc::new(document)), &data);
    assert!(
        expected.contains("http://example.org/ns#high"),
        "the declared function must decide the verdict, or this case is vacuous: {expected}",
    );

    let shapes = parse_shapes(&ttl, None).expect("the SPARQL-function fixture parses");
    let restored =
        admit(&product_for(shapes)).expect("a declared SPARQL function must still restore");
    assert_eq!(
        report_nt(&restored, &data),
        expected,
        "the restored product must answer what the document answers",
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// depth-limit
// ═══════════════════════════════════════════════════════════════════════════════

/// A shapes graph whose `sh:expression` wraps a path in `wraps` distinct
/// operators.
fn nested_expression(wraps: usize) -> String {
    let open = "[ shnex:distinct ".repeat(wraps);
    let close = "]".repeat(wraps);
    format!(
        "ex:S a sh:NodeShape ;\n  sh:targetNode ex:alice ;\n  \
         sh:expression {open}[ shnex:pathValues ex:p ]{close} .\n"
    )
}

/// The one-byte tag that one more level of nesting costs, and the offset in the
/// model stream at which inserting it adds that level.
///
/// Derived by DIFFERENCE rather than transcribed: two products whose shapes graphs
/// differ by exactly one nesting level differ by exactly one byte, and this asserts
/// that. A codec change that made the encoding something other than "one tag, then
/// the child" fails here loudly instead of leaving the nesting cases silently
/// testing nothing.
fn nesting_tag() -> (Vec<u8>, usize, u8) {
    let one = product_of(&nested_expression(1));
    let two = product_of(&nested_expression(2));
    let shallow = section(&one, SECTION_AST);
    let deep = section(&two, SECTION_AST);
    assert_eq!(
        deep.len(),
        shallow.len() + 1,
        "one more nesting level must cost exactly one byte",
    );
    let at = shallow
        .iter()
        .zip(deep.iter())
        .position(|(left, right)| left != right)
        .expect("the two encodings must differ somewhere");
    assert_eq!(
        shallow[at..],
        deep[at + 1..],
        "the deeper encoding must be the shallower one with a single tag inserted",
    );
    (one, at, deep[at])
}

/// A product whose model nests `extra` levels deeper than the fixture's.
fn nested_product(base: &[u8], at: usize, tag: u8, extra: usize) -> Vec<u8> {
    repack(base, |sections| {
        let insert = vec![tag; extra];
        sections[2].splice(at..at, insert);
    })
}

/// The smallest number of extra nesting levels this build refuses, together with
/// the product that provokes it.
///
/// Searched rather than pinned, so "past the ceiling" and "one below the ceiling"
/// stay adjacent to the real ceiling even if it moves.
fn depth_boundary() -> (Vec<u8>, ShapesProductError) {
    let (base, at, tag) = nesting_tag();
    let mut deepest_admitted = 0_usize;
    for extra in 0..512_usize {
        let bytes = nested_product(&base, at, tag, extra);
        match admit(&bytes) {
            Ok(_) => deepest_admitted = extra,
            Err(error) => {
                assert_eq!(
                    deepest_admitted + 1,
                    extra,
                    "the ceiling must be a single boundary, not a scattered set of refusals",
                );
                return (bytes, error);
            }
        }
    }
    panic!("no amount of nesting reached the decoder's ceiling");
}

#[test]
fn refuses_depth_limit() {
    let (_, error) = depth_boundary();
    assert_eq!(error.dimension(), ProductDimension::DepthLimit);
}

#[test]
fn accepts_depth_limit_neighbour() {
    // Exactly ONE level below the ceiling. The ceiling exists so untrusted bytes
    // cannot drive the decoder into unbounded recursion; a ceiling that refused
    // the level below it would be refusing models this build can genuinely decode.
    let (base, at, tag) = nesting_tag();
    let refused = (0..512_usize)
        .find(|extra| admit(&nested_product(&base, at, tag, *extra)).is_err())
        .expect("the ceiling is reachable");
    assert!(
        refused > 0,
        "the unmodified fixture must itself be decodable"
    );
    admit(&nested_product(&base, at, tag, refused - 1))
        .expect("a model one level below the ceiling must still restore");
}

// ═══════════════════════════════════════════════════════════════════════════════
// malformed
// ═══════════════════════════════════════════════════════════════════════════════

/// Provoke: the bytes are structurally invalid in a way no other dimension names.
///
/// An identity section with one byte appended: the writer that produced that
/// section produced exactly its bytes, and ignoring a remainder is how a partial
/// write passes for a whole one.
fn refusal_malformed() -> ShapesProductError {
    let padded = repack(&product_of(PLAIN_SHAPES), |sections| sections[0].push(0));
    ShapesProduct::open(&padded).expect_err("a section with a remainder is not this writer's")
}

#[test]
fn refuses_malformed() {
    assert_eq!(refusal_malformed().dimension(), ProductDimension::Malformed);

    // A directory that disagrees with the format's own section table describes a
    // different format, and lands on the same dimension.
    let mut bytes = product_of(PLAIN_SHAPES);
    bytes[12..16].copy_from_slice(&2_u32.to_le_bytes());
    let error = ShapesProduct::open(&bytes).expect_err("a short directory is not this format");
    assert_eq!(error.dimension(), ProductDimension::Malformed);

    // Arbitrary bytes are refused too — on the MAGIC, because that is the
    // outermost unmet precondition and a refusal always names the first thing
    // that failed rather than a downstream symptom of it.
    let random: Vec<u8> = (0..512_u32)
        .map(|i| (i.wrapping_mul(97) % 251) as u8)
        .collect();
    let error = ShapesProduct::open(&random).expect_err("random bytes were never a product");
    assert_eq!(error.dimension(), ProductDimension::Magic);
}

#[test]
fn accepts_malformed_neighbour() {
    // The smallest well-formed product this build writes: one node shape, no
    // targets, no constraints. Everything about it is empty or zero, which is
    // exactly the shape a fail-closed decoder is most likely to mistake for
    // garbage.
    let minimal = product_of("ex:S a sh:NodeShape .\n");
    let view = ShapesProduct::open(&minimal).expect("a minimal product opens");
    assert_eq!(
        view.section_kinds(),
        vec![SECTION_IDENTITY, SECTION_DATASET, SECTION_AST],
    );
    view.admit(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("a minimal product restores");
}

// ═══════════════════════════════════════════════════════════════════════════════
// The four neighbours with no refusal to pair against
// ═══════════════════════════════════════════════════════════════════════════════

/// An EMPTY shapes graph — zero shapes — must produce a product and admit it.
///
/// Nothing about it is malformed: a shapes graph that constrains nothing is a
/// legitimate configuration, and it is the exact input a fail-closed decoder is
/// most tempted to refuse for being empty.
#[test]
fn accepts_an_empty_shapes_graph() {
    let shapes = parse_shapes(PREFIXES, None).expect("an empty shapes document parses");
    assert!(
        shapes.node_shapes.is_empty(),
        "the fixture must genuinely carry no shapes",
    );
    let bytes = product_for(shapes);
    let restored = admit(&bytes).expect("an empty shapes graph must restore");
    assert_eq!(
        report_nt(&restored, &data_of(PLAIN_DATA)).trim(),
        report_nt(
            &PreparedShapes::new(Arc::new(parse_shapes(PREFIXES, None).expect("parses"))),
            &data_of(PLAIN_DATA),
        )
        .trim(),
        "an empty shapes graph conforms, restored exactly as parsed",
    );
    certify(&bytes).expect("an empty shapes graph certifies");
}

/// A 100% SHACL-SPARQL shapes graph — not one Core constraint — must round-trip.
#[test]
fn accepts_a_pure_shacl_sparql_shapes_graph() {
    let ttl = r#"
ex:S a sh:NodeShape ;
    sh:targetClass ex:Thing ;
    sh:sparql [ a sh:SPARQLConstraint ; sh:message "negative" ; sh:select """
        SELECT $this WHERE { $this <http://example.org/ns#amount> ?v . FILTER(?v < 0) }
    """ ] .
"#;
    let data = data_of(
        r#"
ex:v0 a ex:Thing ; ex:amount "-1"^^xsd:integer .
ex:v1 a ex:Thing ; ex:amount "1"^^xsd:integer .
"#,
    );
    let expected = report_nt(&PreparedShapes::new(Arc::new(shapes_of(ttl))), &data);
    assert!(
        expected.contains("http://example.org/ns#v0"),
        "the SPARQL constraint must actually fire, or the round-trip is vacuous",
    );
    let restored = admit(&product_of(ttl)).expect("a SPARQL-only shapes graph must restore");
    assert_eq!(report_nt(&restored, &data), expected);
}

/// `sh:deactivated` shapes must round-trip.
///
/// Deactivation short-circuits both target resolution and the class-catalog walk,
/// so a deactivated graph exercises the paths where a product is most likely to
/// pin an analysis the restore then re-derives differently.
#[test]
fn accepts_deactivated_shapes() {
    let ttl = r"
ex:Off a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:deactivated true ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ; sh:class ex:Named ] .
";
    let data = data_of(PLAIN_DATA);
    let expected = report_nt(&PreparedShapes::new(Arc::new(shapes_of(ttl))), &data);
    let bytes = product_of(ttl);
    let restored = admit(&bytes).expect("a deactivated shapes graph must restore");
    assert_eq!(
        report_nt(&restored, &data),
        expected,
        "a deactivated shape reports nothing, restored exactly as parsed",
    );
    certify(&bytes).expect("a deactivated shapes graph certifies");
    rebuild(&bytes).expect("a deactivated shapes graph rebuilds");
}

/// A shapes graph declaring expression-bodied functions WHILE the host injects
/// natives — the mixed population the identity's two function rows exist for.
///
/// The declared half is rebuilt from the product's own content and the injected
/// half is the host's. A restore that folded them into one fingerprint would
/// refuse this configuration outright, and it is the configuration a host with any
/// custom functions at all actually runs.
#[test]
fn accepts_declared_functions_alongside_host_natives() {
    let mut shapes = shapes_of(EXPRESSION_FN_SHAPES);
    let mut registry = (*shapes.functions).clone();
    registry.register_native(
        NATIVE_A.to_owned(),
        Arity::Exact(1),
        Volatility::Stable,
        Arc::new(|args: &[&TermValue]| Ok(args.first().map(|value| (*value).clone()))),
    );
    shapes.functions = Arc::new(registry);
    let bytes = product_for_host(shapes, IMPL_A);

    let injected = natives(&[NATIVE_A]);
    let aggregates = AggregateRegistry::new();
    let relations = PropertyFunctionRegistry::new();
    let restored = admit_with(
        &bytes,
        &HostBindings::without_declarations(&injected, &aggregates, &relations, IMPL_A),
    )
    .expect("a declared population plus an injected one must restore");

    // ...and the DECLARED half still evaluates: the restored expression-bodied
    // function computes ex:alice's total as 30 and ex:bob's as 5.
    let report = restored
        .bind_shared_dataset(data_of(EXPRESSION_FN_DATA))
        .expect("binding")
        .validate()
        .expect("validation runs");
    let focus: Vec<String> = report
        .results
        .iter()
        .map(|result| result.focus_node.to_string())
        .collect();
    assert_eq!(focus, vec!["<http://example.org/ns#bob>".to_owned()]);

    // The pair: a host that injects NOTHING is not the host this product was
    // prepared against, and is told so on the function dimension rather than
    // silently restored without the native.
    let error = admit(&bytes).expect_err("the injected population is part of the binding");
    assert_eq!(error.dimension(), ProductDimension::FunctionRegistry);
}

// ═══════════════════════════════════════════════════════════════════════════════
// No Turtle fallback
// ═══════════════════════════════════════════════════════════════════════════════

/// A corrupt product never yields a validator, and no Turtle entry point can be
/// reached during `admit` — BY CONSTRUCTION, because there is no Turtle to reach.
///
/// Asserted structurally rather than by counting calls: a process-global counter
/// would be read by tests running concurrently on other threads, which makes the
/// assertion a flake generator rather than a proof. Two facts are checked instead.
///
/// 1. The product carries no source text at all. Its shapes travel as a pack
///    container and a declarative model, so the document's own syntax — its
///    `@prefix` lines, its prefixed names — is absent from the bytes. A fallback
///    that wanted to re-parse Turtle would have nothing to parse.
/// 2. Every corruption refuses. Notably, a corrupt MODEL section refuses `admit`
///    outright rather than quietly degrading into `rebuild` — which would have
///    succeeded on the very same bytes. That silent degradation is what "no
///    fallback" actually forbids, and it is observable exactly here.
#[test]
fn no_turtle_fallback() {
    let bytes = product_of(PLAIN_SHAPES);
    for syntax in [
        "@prefix",
        "sh:NodeShape",
        "sh:targetClass",
        "ex:PersonShape",
    ] {
        assert!(
            !contains(&bytes, syntax.as_bytes()),
            "a product must carry no Turtle source text, found {syntax:?}",
        );
    }
    assert!(
        contains(&bytes, b"http://www.w3.org/ns/shacl#targetClass"),
        "...while the shapes themselves DID travel, as expanded terms",
    );

    // A corrupt model refuses admission, on a named dimension, with no validator
    // produced — even though the authenticated dataset beside it would have
    // rebuilt perfectly.
    let corrupt_model = repack(&bytes, |sections| {
        let last = sections[2].len() - 1;
        sections[2][last] ^= 0xFF;
    });
    let error = admit(&corrupt_model).expect_err("a corrupt model yields no validator");
    assert!(
        ProductDimension::ALL.contains(&error.dimension()),
        "every refusal names a declared dimension",
    );
    rebuild(&corrupt_model).expect(
        "rebuild ignores the memo, so the same bytes rescue — admit did NOT take that path",
    );

    // And the whole corruption family yields no validator either.
    for corrupt in corruptions() {
        assert!(
            admit(&corrupt).is_err(),
            "a corrupt product must never produce a validator",
        );
    }
}

/// Every corrupt product this file knows how to build.
fn corruptions() -> Vec<Vec<u8>> {
    let bytes = product_of(PLAIN_SHAPES);
    let trailer = bytes.len() - TRAILER_LEN;
    vec![
        flip(&bytes, 0),
        flip(&bytes, trailer + TRAILER_FILE_LEN_OFFSET),
        flip(&bytes, trailer + TRAILER_CONTAINER_DIGEST_OFFSET),
        flip(&bytes, sole_offset(&bytes, &section(&bytes, SECTION_AST))),
        bytes[..bytes.len() - 1].to_vec(),
        repack(&bytes, |sections| sections[0].push(0)),
        repack(&bytes, |sections| sections[1].clear()),
        repack(&bytes, |sections| sections[2].clear()),
        foreign_stage_product(),
        other_profile_product(),
        unknown_tag_product(),
        Vec::new(),
        vec![0u8; 64],
    ]
}

// ═══════════════════════════════════════════════════════════════════════════════
// The census: every label this file can provoke, against every label declared
// ═══════════════════════════════════════════════════════════════════════════════

/// Every dimension this file actually provokes, collected by RUNNING each
/// provocation rather than by transcribing a list of the ones it means to.
fn provoked_dimensions() -> BTreeSet<ProductDimension> {
    [
        refusal_magic(),
        refusal_format_version(),
        refusal_stage_id(),
        refusal_profile(),
        refusal_truncated(),
        refusal_trailer(),
        refusal_section_digest(),
        refusal_container_digest(),
        refusal_dataset_identity(),
        refusal_shapes_graph(),
        refusal_expected_identity(),
        refusal_prefixes(),
        refusal_base(),
        refusal_vocabulary(),
        refusal_function_registry(),
        refusal_aggregate_registry(),
        refusal_property_function_registry(),
        refusal_implementation_identity(),
        refusal_class_catalog(),
        refusal_parse_configuration(),
        refusal_unsupported_capability(),
        depth_boundary().1,
        refusal_malformed(),
    ]
    .iter()
    .map(ShapesProductError::dimension)
    .collect()
}

/// The dimensions no source file in this crate ever CONSTRUCTS.
///
/// The scan skips two kinds of file: `product/error.rs`, which declares the enum
/// and therefore names every variant by definition, and any `tests.rs`, whose
/// mentions are assertions about refusals rather than refusals.
fn unemitted_dimensions() -> BTreeSet<ProductDimension> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut sources = Vec::new();
    collect_sources(&root, &mut sources);
    assert!(
        sources.len() > 20,
        "the source scan found only {} files; it is not reading this crate",
        sources.len(),
    );

    let mut emitted = BTreeSet::new();
    for source in &sources {
        for dimension in ProductDimension::ALL {
            if source.contains(&format!("ProductDimension::{dimension:?}")) {
                emitted.insert(dimension);
            }
        }
    }
    ProductDimension::ALL
        .into_iter()
        .filter(|dimension| !emitted.contains(dimension))
        .collect()
}

/// Read every `*.rs` under `dir` that can construct a refusal.
fn collect_sources(dir: &Path, out: &mut Vec<String>) {
    let entries = std::fs::read_dir(dir).expect("the crate's source tree is readable");
    for entry in entries {
        let path = entry.expect("a readable directory entry").path();
        if path.is_dir() {
            collect_sources(&path, out);
            continue;
        }
        if path.extension().and_then(std::ffi::OsStr::to_str) != Some("rs") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or_default();
        if name == "tests.rs" || name == "error.rs" {
            continue;
        }
        out.push(std::fs::read_to_string(&path).expect("a readable source file"));
    }
}

/// The set of dimensions this file can provoke is EXACTLY the set the codec
/// declares, with no exception list.
///
/// Both halves matter. Without the first, a dimension could be declared, shipped
/// and never once exercised. Without the second, a dimension could be declared
/// and unconstructible — a refusal label the surface promises and can never
/// deliver — so the source scan restates the emptiness as a fact about the
/// crate's own sources rather than an unchecked excuse.
#[test]
fn refusal_labels_emitted_equal_labels_declared() {
    let provoked = provoked_dimensions();
    let unemitted = unemitted_dimensions();

    assert_eq!(
        unemitted
            .iter()
            .map(|dimension| dimension.label())
            .collect::<Vec<_>>(),
        Vec::<&str>::new(),
        "every declared dimension must be constructed by some source path in this crate",
    );

    let declared: BTreeSet<ProductDimension> = ProductDimension::ALL.into_iter().collect();
    let missing: Vec<&str> = declared
        .difference(&provoked)
        .map(|dimension| dimension.label())
        .collect();
    assert!(
        missing.is_empty(),
        "these dimensions are emittable but no test here provokes them: {missing:?}",
    );

    assert_eq!(
        provoked.len(),
        ProductDimension::COUNT,
        "every declared dimension is provoked here",
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Byte fuzz
// ═══════════════════════════════════════════════════════════════════════════════

/// One valid product, built once: the mutation generator needs a base, and paying
/// a parse and a canonicalization per case would make the property test a
/// benchmark instead of a fuzz.
fn base_product() -> &'static [u8] {
    static BASE: OnceLock<Vec<u8>> = OnceLock::new();
    BASE.get_or_init(|| product_of(PLAIN_SHAPES))
}

/// Open and admit `bytes`, asserting the ONE thing an admission boundary owes
/// hostile input: it answers. Never a panic, never an abort, always either a
/// restored preparation or a refusal that names a declared dimension and explains
/// itself.
fn outcome_is_total(bytes: &[u8]) {
    match ShapesProduct::open(bytes)
        .and_then(|view| view.admit(&ShapesProfile::CORE, &HostBindings::empty()))
    {
        Ok(_) => {}
        Err(error) => {
            assert!(
                ProductDimension::ALL.contains(&error.dimension()),
                "a refusal must name a declared dimension, got {error}",
            );
            assert!(
                !error.message().is_empty(),
                "a refusal must explain itself: {error}",
            );
            assert!(
                error
                    .to_string()
                    .starts_with(&format!("{}: ", error.dimension().label())),
                "a refusal renders as `label: message`, got {error}",
            );
        }
    }
}

/// The property-test budget: 256 cases, or `PURRDF_PROP_CASES` when set.
fn config() -> Config {
    Config::with_cases(prop::cases_from_env(256))
}

prop_test! {
    #![prop_config(config())]

    /// Arbitrary bytes: nothing a caller can hand this boundary may panic it.
    #[test]
    fn arbitrary_bytes_never_panic(bytes in prop::collection::vec(any::<u8>(), 0..1024)) {
        outcome_is_total(&bytes);
    }

    /// Arbitrary bytes that OPEN with the product magic — the hostile case that
    /// gets past the cheapest check and reaches the framing decoders behind it.
    #[test]
    fn arbitrary_bytes_behind_the_magic_never_panic(
        tail in prop::collection::vec(any::<u8>(), 0..1024),
    ) {
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&tail);
        outcome_is_total(&bytes);
    }

    /// Single-byte mutations of a VALID product: the neighbourhood of a real
    /// artifact, where every field is plausible and only one is wrong.
    #[test]
    fn single_byte_mutations_never_panic(index in any::<prop::sample::Index>(), delta in 1u8..=255) {
        let mut bytes = base_product().to_vec();
        let at = index.index(bytes.len());
        bytes[at] = bytes[at].wrapping_add(delta);
        outcome_is_total(&bytes);
    }
}

/// Two extension-function namespaces, in the two possible orders.
///
/// The pair is chosen so the order is OBSERVABLE: `.../a/` is a prefix of `.../a/b/`,
/// and stripping is first-match-wins, so `http://example.org/a/b/f` becomes `b/f`
/// under this order and `f` under the reverse. Two different function names for one
/// IRI, decided by declaration order alone.
static EXT_ORDER_FORWARD: std::sync::LazyLock<purrdf_shapes::product::ParserOptions> =
    std::sync::LazyLock::new(|| purrdf_shapes::product::ParserOptions {
        extension_fn_namespaces: vec![
            "http://example.org/a/".to_owned(),
            "http://example.org/a/b/".to_owned(),
        ],
        ..purrdf_shapes::product::ParserOptions::default()
    });

/// The same two namespaces, reversed.
static EXT_ORDER_REVERSED: std::sync::LazyLock<purrdf_shapes::product::ParserOptions> =
    std::sync::LazyLock::new(|| purrdf_shapes::product::ParserOptions {
        extension_fn_namespaces: vec![
            "http://example.org/a/b/".to_owned(),
            "http://example.org/a/".to_owned(),
        ],
        ..purrdf_shapes::product::ParserOptions::default()
    });

/// Two relation namespaces, and the same two in the other order. Relation
/// recognition is order-INDEPENDENT — an IRI is a call iff it prefix-matches any
/// entry, and nothing is stripped — so these two must be the same configuration.
static REL_ORDER_FORWARD: std::sync::LazyLock<purrdf_shapes::product::ParserOptions> =
    std::sync::LazyLock::new(|| purrdf_shapes::product::ParserOptions {
        property_fn_namespaces: vec![
            "http://example.org/p/".to_owned(),
            "http://example.org/q/".to_owned(),
        ],
        ..purrdf_shapes::product::ParserOptions::default()
    });

/// The same two relation namespaces, reversed.
static REL_ORDER_REVERSED: std::sync::LazyLock<purrdf_shapes::product::ParserOptions> =
    std::sync::LazyLock::new(|| purrdf_shapes::product::ParserOptions {
        property_fn_namespaces: vec![
            "http://example.org/q/".to_owned(),
            "http://example.org/p/".to_owned(),
        ],
        ..purrdf_shapes::product::ParserOptions::default()
    });

/// Extension-function namespace ORDER is part of the parse configuration, and
/// relation namespace order is not.
///
/// These two halves must disagree, and that asymmetry is the whole content of the
/// test. `extension_fn_namespaces` is first-match-wins for prefix STRIPPING, so
/// reversing it renames functions; folding it order-insensitively would encode both
/// orders identically, admit a product written under one into a host running the
/// other, and let the restored preparation resolve extension-function calls
/// differently from the environment it was written for — the silent wrong answer this
/// row exists to refuse.
///
/// `property_fn_namespaces` is a set: recognition is order-independent and nothing is
/// stripped. Refusing on ITS order would be the mirror defect — an over-refusal that
/// stops two hosts with identical configuration from opening each other's products.
#[test]
fn extension_namespace_order_is_configuration_and_relation_namespace_order_is_not() {
    let functions = UserFunctionRegistry::new();
    let aggregates = AggregateRegistry::new();
    let relations = PropertyFunctionRegistry::new();

    let written_ext = to_product_for_host(
        PLAIN_SHAPES,
        &HostBindings::new(&functions, &aggregates, &relations, &[], &EXT_ORDER_FORWARD),
    );
    let refused = admit_with(
        &written_ext,
        &HostBindings::new(
            &functions,
            &aggregates,
            &relations,
            &[],
            &EXT_ORDER_REVERSED,
        ),
    )
    .expect_err(
        "reversing extension-function namespaces renames functions, so it is a different parse",
    );
    assert_eq!(refused.dimension(), ProductDimension::ParseConfiguration);

    // The valid neighbour, in two directions. Same order: opens.
    admit_with(
        &written_ext,
        &HostBindings::new(&functions, &aggregates, &relations, &[], &EXT_ORDER_FORWARD),
    )
    .expect("the order it was written under is the order it opens under");

    // And relation-namespace order, which is NOT configuration: opens either way.
    let written_rel = to_product_for_host(
        PLAIN_SHAPES,
        &HostBindings::new(&functions, &aggregates, &relations, &[], &REL_ORDER_FORWARD),
    );
    admit_with(
        &written_rel,
        &HostBindings::new(
            &functions,
            &aggregates,
            &relations,
            &[],
            &REL_ORDER_REVERSED,
        ),
    )
    .expect(
        "relation recognition is order-independent, so the same two namespaces in either \
         order are one configuration",
    );
}
