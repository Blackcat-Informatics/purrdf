// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! **What a RESTORED preparation must still behave like.**
//!
//! Every other proof about the prepared-product codec is about BYTES: that the
//! container frames, that the digests cover, that the model decodes, that a
//! restore refuses what it should. Those establish that a restored preparation is
//! the same VALUE the writer had. They establish nothing whatsoever about how that
//! value behaves when it is used, and "reuse the existing prepared-query machinery,
//! retaining bounded caches, immutable shared handles, deterministic output, and
//! data-dependent target binding that stays tied to the actual input dataset" is
//! entirely a claim about behaviour.
//!
//! That gap is not academic. Each of the four properties below has a specific,
//! silent way to be lost across a restore, and in every case the bytes still verify
//! and the report still looks well formed:
//!
//! 1. **Bounded caches stay LAZY.** A decoder that eagerly filled a lazy cell would
//!    pay the compile cost for every constraint in the graph on every restore —
//!    including the ones the data never reaches — turning a product into something
//!    slower than the parse it replaces while every existing test passed.
//! 2. **Shared handles stay SHARED.** A binding that deep-copied the shape tree per
//!    dataset would make the "prepare once, validate many" contract a lie, at a cost
//!    proportional to the shapes graph and invisible in any report.
//! 3. **Output stays DETERMINISTIC under reuse.** A lazy cache is process state, and
//!    process state that leaks into an answer makes the FIRST validation from a
//!    preparation disagree with the second — the worst possible failure for a
//!    content-addressed artifact, because the disagreement is invisible to anyone
//!    who validates once.
//! 4. **Target binding stays DATA-DEPENDENT.** A preparation that memoised its
//!    resolved focus nodes would validate a new snapshot against the target set of
//!    an old one: a green report about nodes that are not in the dataset, and
//!    silence about the ones that are.
//!
//! # How each property is OBSERVED, and where the observation is indirect
//!
//! Property 1 is observed DIRECTLY. `Constraint::Pattern::compiled` is a public
//! field of a public variant, and `OnceLock::get` returns `None` for a cell nothing
//! has initialised, so "lazy after a restore" and "hot after a validation that
//! reached it" are both readable off the public API with no proxy in between.
//!
//! Property 2 is observed through the REFERENCE COUNT of the preparation's own
//! `Arc<Shapes>` rather than through `Arc::ptr_eq` against the handle a binding
//! keeps, because `PreparedValidator` deliberately exposes no shapes accessor and
//! this test does not invent one to be tested with. The count is not a weaker
//! statement: `Arc::strong_count` over the preparation's handle counts references to
//! exactly one allocation, so N live bindings that share the tree raise it by N,
//! while N bindings that each cloned the tree leave it untouched — which is the
//! defect, stated in the negative. `Arc::ptr_eq` IS used where a handle is reachable
//! on both sides: across a `PreparedShapes` clone, and across the provenance handle
//! a binding really does hand back.
//!
//! Properties 3 and 4 are observed through the reports themselves, in their
//! canonical N-Triples form, because that is the artifact a consumer actually
//! receives.
//!
//! # The fixture
//!
//! One shapes graph with a `sh:pattern` (the lazy cell property 1 reads) and two
//! DATA-DEPENDENT target kinds (`sh:targetClass`, `sh:targetSubjectsOf`), and three
//! data graphs chosen so that memoising the first one's answers would be VISIBLE:
//! the three do not share a focus node between them except deliberately, so a
//! preparation that reused dataset A's target set would report about nodes dataset
//! B does not contain and stay silent about the ones it does.
//!
//! Everything is under `example.org`: PurRDF mints no vocabulary IRIs.

use std::sync::Arc;

use purrdf::RdfDataset;
use purrdf_shapes::engine::{PreparedShapes, parse_shapes};
use purrdf_shapes::product::{HostBindings, ShapesProduct, ShapesProfile};
use purrdf_shapes::provenance::{ProductRestore, ValidatorProvenance};
use purrdf_shapes::shapes::{Constraint, Shapes};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

/// The shapes graph every proof here restores.
///
/// `ex:code`'s `sh:pattern` is the lazy cell; `sh:targetClass` and
/// `sh:targetSubjectsOf` are the two target kinds that can only be answered against
/// a real dataset.
const SHAPES: &str = r"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/ns#> .

ex:EmployeeShape a sh:NodeShape ;
    sh:targetClass ex:Employee ;
    sh:property [ sh:path ex:code ; sh:pattern '^EMP-[0-9][0-9][0-9][0-9]$' ] .

ex:SiteShape a sh:NodeShape ;
    sh:targetSubjectsOf ex:locatedIn ;
    sh:property [ sh:path ex:label ; sh:minCount 1 ] .
";

/// A dataset whose one employee CONFORMS — its code matches the pattern, so
/// validating it compiles the regex without producing a result.
const DATA_CONFORMING: &str = r#"
@prefix ex: <http://example.org/ns#> .
ex:n1 a ex:Employee ; ex:code "EMP-0001" .
"#;

/// A dataset with a DIFFERENT employee whose code violates the pattern.
///
/// Shares no focus node with [`DATA_CONFORMING`]: a preparation that had memoised
/// that dataset's target set would find nothing to validate here.
const DATA_PATTERN_VIOLATION: &str = r#"
@prefix ex: <http://example.org/ns#> .
ex:n2 a ex:Employee ; ex:code "not-a-code" .
"#;

/// A dataset that re-uses [`DATA_CONFORMING`]'s focus node with a violating code
/// AND adds a site the other two graphs have no node for.
///
/// The overlap is deliberate: a memoised target set would still find `ex:n1` here,
/// so the thing that distinguishes correct re-binding from a stale answer is the
/// SECOND result — the `sh:targetSubjectsOf` shape's `ex:n3`, which no earlier
/// dataset could have put in a cached target set.
const DATA_TWO_VIOLATIONS: &str = r#"
@prefix ex: <http://example.org/ns#> .
ex:n1 a ex:Employee ; ex:code "not-a-code" .
ex:n3 ex:locatedIn ex:city .
"#;

/// Pack [`SHAPES`] as a prepared product.
fn product_bytes() -> Vec<u8> {
    let shapes = parse_shapes(SHAPES, None).expect("the fixture shapes parse");
    PreparedShapes::new(Arc::new(shapes))
        .to_product(&ShapesProfile::CORE)
        .expect("the fixture is representable as a product")
}

/// Restore a preparation the way a consumer does: open the product and admit it.
///
/// Every proof in this file starts from one of these rather than from
/// `PreparedShapes::new`, because a property that only holds for a freshly parsed
/// preparation is exactly the property a restore can silently lose.
fn restored() -> PreparedShapes {
    ShapesProduct::open(&product_bytes())
        .expect("the packed product opens")
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("the packed product admits in the process that wrote it")
}

/// Freeze one of the fixture data graphs.
fn dataset(turtle: &str) -> Arc<RdfDataset> {
    parse_turtle_to_dataset(turtle, None).expect("the fixture data parses")
}

/// Validate `data` with `prepared` and render the report as canonical N-Triples.
///
/// The RDF form is the comparison surface rather than a field-by-field walk: two
/// runs agree exactly when the graphs they produce are the same bytes, and a lost
/// constraint shows up as a missing result rather than as a field nobody thought to
/// compare.
fn report_nt(prepared: &PreparedShapes, data: &Arc<RdfDataset>) -> String {
    prepared
        .bind_shared_dataset(Arc::clone(data))
        .expect("the data graph binds")
        .validate()
        .expect("validation runs")
        .to_ntriples()
}

/// Every `sh:pattern` lazy cell reachable from `shapes`, in shape and property
/// order.
///
/// A walk rather than an index: the point of the assertions below is that NOTHING
/// initialised these cells, so the test has to be able to say "none of them" rather
/// than "not the one I remembered the position of".
fn pattern_cells(
    shapes: &Shapes,
) -> Vec<
    &Arc<
        std::sync::OnceLock<
            Result<purrdf_core::xsd_regex::CompiledPattern, purrdf_core::xsd_regex::XsdRegexError>,
        >,
    >,
> {
    let mut cells = Vec::new();
    for shape in &shapes.node_shapes {
        for property in &shape.property_shapes {
            for constraint in &property.constraints {
                if let Constraint::Pattern { compiled, .. } = constraint {
                    cells.push(compiled);
                }
            }
        }
    }
    assert!(
        !cells.is_empty(),
        "the fixture must carry at least one sh:pattern, or every assertion about the lazy \
         regex cache is vacuous"
    );
    cells
}

/// The 64 lowercase hexadecimal digits of a 32-byte digest.
///
/// An independent transcription of the rendering under test: the provenance's
/// `Display` must agree with a spelling written here from the raw bytes, rather
/// than with a second call to itself.
fn hex(digest: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    digest.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

// ── Property 1: bounded caches are LAZY after a restore ──────────────────────────

/// A restore must hand back a preparation whose regex caches are cold, and a
/// validation that reaches a pattern must warm exactly those it reached.
///
/// The two halves are one test because either alone is satisfiable by a defect:
/// asserting only "cold after restore" passes for a cache that is never used at
/// all, and asserting only "hot after validation" passes for a decoder that
/// compiled every pattern in the graph eagerly.
#[test]
fn a_restored_preparation_compiles_no_pattern_until_one_is_reached() {
    let restored = restored();

    for cell in pattern_cells(restored.shapes()) {
        assert!(
            cell.get().is_none(),
            "a restore must not compile a pattern the data may never reach; this cell was \
             already initialised before any dataset was bound"
        );
    }

    let data = dataset(DATA_CONFORMING);
    assert_conforming(&report_nt(&restored, &data));

    for cell in pattern_cells(restored.shapes()) {
        assert!(
            cell.get().is_some(),
            "a validation that evaluated this pattern must have filled its cache, or the \
             bounded cache the preparation carries is not the one validation uses"
        );
    }
}

/// The warmed cache is the PREPARATION's, not a per-binding copy of it — so a
/// second binding from the same restored preparation compiles nothing again.
///
/// This is the half that makes property 1 worth having across a restore: a cell
/// that were cloned per binding would still read "hot" through the binding that
/// warmed it, and cold through the preparation, and every regex would be compiled
/// once per dataset for the life of the process.
#[test]
fn a_second_binding_reuses_the_cache_the_first_binding_warmed() {
    let restored = restored();
    assert_conforming(&report_nt(&restored, &dataset(DATA_CONFORMING)));

    // Read the cells off the PREPARATION, which the first binding is no longer
    // alive to be confused with.
    let cells = pattern_cells(restored.shapes());
    let before: Vec<*const _> = cells
        .iter()
        .map(|cell| {
            std::ptr::from_ref(
                cell.get()
                    .expect("the first validation warmed every pattern the fixture has"),
            )
        })
        .collect();

    let second = report_nt(&restored, &dataset(DATA_PATTERN_VIOLATION));
    assert!(
        second.contains("http://example.org/ns#n2"),
        "the second dataset must actually exercise the pattern, or the cache was never \
         consulted a second time: {second}"
    );

    let after: Vec<*const _> = pattern_cells(restored.shapes())
        .iter()
        .map(|cell| std::ptr::from_ref(cell.get().expect("the cache stays warm across bindings")))
        .collect();
    assert_eq!(
        before, after,
        "the second binding must have read the compiled pattern the first one produced, \
         rather than compiling its own"
    );
}

// ── Property 2: immutable shared handles ─────────────────────────────────────────

/// Binding ONE restored preparation to several datasets shares one shape tree.
///
/// Stated as the reference count of the preparation's own `Arc<Shapes>` — see the
/// [module docs](self) for why that is the observation and what it rules out. The
/// count is taken as a DELTA from the live baseline rather than against a literal,
/// so the proof is about what binding does and not about how many handles the
/// restore happened to leave behind.
#[test]
fn binding_one_restored_preparation_to_many_datasets_shares_one_shape_tree() {
    let restored = restored();
    let baseline = Arc::strong_count(restored.shapes());

    let datasets = [
        dataset(DATA_CONFORMING),
        dataset(DATA_PATTERN_VIOLATION),
        dataset(DATA_TWO_VIOLATIONS),
    ];
    let bindings: Vec<_> = datasets
        .iter()
        .map(|data| {
            restored
                .bind_shared_dataset(Arc::clone(data))
                .expect("the data graph binds")
        })
        .collect();

    assert_eq!(
        Arc::strong_count(restored.shapes()),
        baseline + bindings.len(),
        "each live binding must hold a reference to the preparation's ONE shape tree; a count \
         that stayed at the baseline means every binding deep-copied the shapes instead"
    );

    // Every binding really does validate — a set of bindings that could not run
    // would satisfy the count above while proving nothing about reuse.
    for binding in &bindings {
        binding.validate().expect("each binding validates");
    }

    drop(bindings);
    assert_eq!(
        Arc::strong_count(restored.shapes()),
        baseline,
        "dropping the bindings must release exactly the references they took, or the \
         preparation leaks a handle per dataset"
    );
}

/// Cloning a restored preparation shares its shape tree rather than copying it —
/// the `Arc::ptr_eq` half of property 2, on the one handle that IS reachable from
/// both sides.
#[test]
fn cloning_a_restored_preparation_shares_its_shape_tree() {
    let restored = restored();
    let clone = restored.clone();

    assert!(
        Arc::ptr_eq(restored.shapes(), clone.shapes()),
        "a cloned preparation must point at the same shape tree; a deep copy here would make \
         every clone pay the shapes graph's storage again and warm its own caches"
    );

    // Not vacuous: the clone is a working preparation, not just a pointer.
    assert_eq!(
        report_nt(&clone, &dataset(DATA_PATTERN_VIOLATION)),
        report_nt(&restored, &dataset(DATA_PATTERN_VIOLATION)),
        "the clone must reach the same verdict as the preparation it came from"
    );
}

/// A binding hands back the SAME provenance value the preparation holds, not a copy
/// of it — so attributing a report to its artifact costs a pointer, and cannot
/// drift from what the preparation says.
#[test]
fn every_binding_shares_the_preparations_provenance() {
    let restored = restored();
    let first = restored
        .bind_shared_dataset(dataset(DATA_CONFORMING))
        .expect("the data graph binds");
    let second = restored
        .bind_shared_dataset(dataset(DATA_TWO_VIOLATIONS))
        .expect("the data graph binds");

    assert!(
        std::ptr::eq(restored.provenance(), first.provenance()),
        "a binding must share the preparation's provenance handle"
    );
    assert!(
        std::ptr::eq(first.provenance(), second.provenance()),
        "two bindings of one preparation must share ONE provenance handle"
    );
}

// ── Property 3: deterministic output under reuse ─────────────────────────────────

/// Ten validations from ONE restored preparation produce byte-identical reports.
///
/// The repetition is the point: the first validation runs against cold caches and
/// the other nine against warm ones, so a lazy cell whose state reached an answer
/// would make run 1 disagree with run 2 — a divergence nobody who validates once
/// can see.
#[test]
fn ten_validations_from_one_restored_preparation_agree_byte_for_byte() {
    let restored = restored();
    let data = dataset(DATA_TWO_VIOLATIONS);

    let first = report_nt(&restored, &data);
    assert_non_vacuous(&first);

    for run in 1..10 {
        assert_eq!(
            report_nt(&restored, &data),
            first,
            "validation {run} from the same restored preparation disagreed with the first; \
             reuse must not carry process state into an answer"
        );
    }
}

/// Ten validations over ten SEPARATELY PARSED copies of one data graph also agree.
///
/// The test above reuses one frozen dataset, so it cannot distinguish "the
/// preparation carries no state" from "the dataset carries the state". Re-freezing
/// the same Turtle each time gives every run its own interning tables and id
/// assignment, which is where a report that depended on anything but the graph's
/// content would diverge.
#[test]
fn ten_validations_over_independently_parsed_data_agree_byte_for_byte() {
    let restored = restored();
    let first = report_nt(&restored, &dataset(DATA_TWO_VIOLATIONS));
    assert_non_vacuous(&first);

    for run in 1..10 {
        assert_eq!(
            report_nt(&restored, &dataset(DATA_TWO_VIOLATIONS)),
            first,
            "run {run} over an independently frozen copy of the same data graph disagreed \
             with the first"
        );
    }
}

/// Refuse anything but a CONFORMING report that reached the shapes at all.
///
/// A conforming SHACL report is not an empty graph — it is a `sh:ValidationReport`
/// carrying `sh:conforms true` and no results — so both halves are asserted. The
/// negative half is what keeps the fixture's "this data conforms" claim from being
/// satisfied by a preparation that simply found nothing to validate.
fn assert_conforming(report_nt: &str) {
    assert!(
        report_nt.contains(
            "<http://www.w3.org/ns/shacl#conforms> \
             \"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>"
        ),
        "this dataset must conform: {report_nt}"
    );
    assert!(
        !report_nt.contains("shacl#ValidationResult"),
        "a conforming report carries no result: {report_nt}"
    );
}

/// Refuse a report that could not distinguish a working restore from a broken one.
///
/// Both of [`DATA_TWO_VIOLATIONS`]'s violations must be present. Without this the
/// equality assertions above are satisfied by ten identically EMPTY reports, which
/// is exactly the shape a restore that lost the whole shapes graph would produce.
fn assert_non_vacuous(report_nt: &str) {
    for focus in ["http://example.org/ns#n1", "http://example.org/ns#n3"] {
        assert!(
            report_nt.contains(focus),
            "the fixture report must name {focus}, or every equality assertion over it is \
             vacuous: {report_nt}"
        );
    }
}

// ── Property 4: target binding stays tied to the actual input dataset ────────────

/// One restored preparation bound to three different datasets reaches three
/// different, correctly re-bound verdicts.
///
/// This is the property the codec's section directory promises structurally — there
/// is no target-resolution section and there never will be — proved BEHAVIOURALLY
/// on the restored value, because "the artifact carries no cached targets" and "the
/// restored preparation resolves targets per dataset" are two different statements
/// and only the second one is what a consumer relies on.
///
/// The three verdicts are asserted by content, not merely asserted to differ: three
/// distinct WRONG answers would also differ from each other.
#[test]
fn one_restored_preparation_rebinds_targets_per_dataset() {
    let restored = restored();

    let conforming = report_nt(&restored, &dataset(DATA_CONFORMING));
    let pattern = report_nt(&restored, &dataset(DATA_PATTERN_VIOLATION));
    let two = report_nt(&restored, &dataset(DATA_TWO_VIOLATIONS));

    assert_conforming(&conforming);

    // `ex:n2` exists in NO other fixture dataset. A preparation that had memoised
    // the first dataset's target set would have had nothing to validate here.
    assert!(
        pattern.contains("http://example.org/ns#n2")
            && pattern.contains("PatternConstraintComponent"),
        "the second dataset's employee must be found and refused on its code; a memoised \
         target set from the first dataset would have missed it entirely: {pattern}"
    );
    assert!(
        !pattern.contains("http://example.org/ns#n1"),
        "the second dataset does not contain ex:n1, so no result may name it — a report that \
         does is validating the PREVIOUS dataset's focus nodes: {pattern}"
    );

    // The third dataset re-uses `ex:n1` (which the first dataset had, conforming)
    // and adds `ex:n3` under a target kind no earlier dataset populated at all.
    assert!(
        two.contains("http://example.org/ns#n1") && two.contains("PatternConstraintComponent"),
        "ex:n1 conformed in the first dataset and violates in the third; a preparation that \
         reused an earlier verdict would still call it conforming: {two}"
    );
    assert!(
        two.contains("http://example.org/ns#n3") && two.contains("MinCountConstraintComponent"),
        "the sh:targetSubjectsOf shape must find ex:n3, which only this dataset contains: {two}"
    );

    assert_ne!(conforming, pattern);
    assert_ne!(pattern, two);
    assert_ne!(conforming, two);
}

/// A restored preparation and a freshly parsed one reach the SAME three verdicts.
///
/// Property 4 would be satisfiable by a restore that re-resolved targets per dataset
/// and got them wrong every time, consistently. This pins the three answers to the
/// ones the parse route produces, which is the only definition of "correctly
/// re-bound" that does not restate the implementation.
#[test]
fn a_restored_preparation_rebinds_targets_exactly_as_a_parse_does() {
    let restored = restored();
    let parsed = PreparedShapes::new(Arc::new(
        parse_shapes(SHAPES, None).expect("the fixture shapes parse"),
    ));

    for turtle in [DATA_CONFORMING, DATA_PATTERN_VIOLATION, DATA_TWO_VIOLATIONS] {
        let data = dataset(turtle);
        assert_eq!(
            report_nt(&restored, &data),
            report_nt(&parsed, &data),
            "a restored preparation must reach the parse route's verdict on every dataset"
        );
    }
}

// ── Provenance: which artifact a restored preparation came from ──────────────────

/// A preparation built from a parsed shapes graph says so, and names no artifact.
#[test]
fn a_parsed_preparation_reports_a_parsed_provenance() {
    let prepared = PreparedShapes::new(Arc::new(
        parse_shapes(SHAPES, None).expect("the fixture shapes parse"),
    ));

    assert_eq!(prepared.provenance(), &ValidatorProvenance::Parsed);
    assert_eq!(
        prepared.provenance().product_identity(),
        None,
        "nothing was restored, so there is no artifact to name and none may be invented"
    );
    assert_eq!(prepared.provenance().to_string(), "parsed");
}

/// An ADMITTED preparation names the artifact it came from, with the identity the
/// product declares — the same digest `purrdf shacl explain` prints.
#[test]
fn an_admitted_preparation_names_the_product_it_came_from() {
    let bytes = product_bytes();
    let declared = *ShapesProduct::open(&bytes)
        .expect("the packed product opens")
        .declared_identity()
        .digest();

    let restored = ShapesProduct::open(&bytes)
        .expect("the packed product opens")
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("the packed product admits");

    let ValidatorProvenance::Restored { identity, restore } = restored.provenance() else {
        panic!("an admitted preparation must report a restored provenance");
    };
    assert_eq!(*restore, ProductRestore::Admitted);
    assert_eq!(
        identity.digest(),
        &declared,
        "the provenance must name the artifact the caller opened, byte for byte"
    );
    assert!(
        !identity.components().is_empty(),
        "the product binds real inputs, so the recorded identity must carry them"
    );

    let rendered = restored.provenance().to_string();
    let expected_digest = hex(&declared);
    assert_eq!(rendered, format!("restored-admitted {expected_digest}"));
}

/// A REBUILT preparation names the same artifact, and says which seam produced it.
///
/// The two restore tokens are distinct because the identity means something
/// different on each: admitted means it was checked against this environment,
/// rebuilt means it was recorded from the product and deliberately not checked.
#[test]
fn a_rebuilt_preparation_names_the_product_and_its_seam() {
    let bytes = product_bytes();
    let declared = *ShapesProduct::open(&bytes)
        .expect("the packed product opens")
        .declared_identity()
        .digest();

    let rebuilt = ShapesProduct::open(&bytes)
        .expect("the packed product opens")
        .rebuild(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("the packed product rebuilds");

    let ValidatorProvenance::Restored { identity, restore } = rebuilt.provenance() else {
        panic!("a rebuilt preparation must report a restored provenance");
    };
    assert_eq!(*restore, ProductRestore::Rebuilt);
    assert_eq!(identity.digest(), &declared);

    let expected_digest = hex(&declared);
    assert_eq!(
        rebuilt.provenance().to_string(),
        format!("restored-rebuilt {expected_digest}")
    );

    // The two seams are distinguishable, which is the whole reason they are two
    // tokens rather than one.
    let admitted = restored();
    assert_ne!(admitted.provenance(), rebuilt.provenance());
}

/// The provenance survives every route a preparation travels: cloning it, and
/// binding it to a dataset.
///
/// A provenance that were reset by `bind` would answer "parsed" for a validator
/// executing a restored product, which is worse than no accessor at all: it is a
/// confident wrong attribution.
#[test]
fn provenance_survives_cloning_and_binding() {
    let restored = restored();
    let clone = restored.clone();
    assert_eq!(clone.provenance(), restored.provenance());

    let bound = restored
        .bind_shared_dataset(dataset(DATA_TWO_VIOLATIONS))
        .expect("the data graph binds");
    assert_eq!(bound.provenance(), restored.provenance());
    assert!(matches!(
        bound.provenance(),
        ValidatorProvenance::Restored { .. }
    ));

    // …and a validator built the parse way still says so, so the accessor is not
    // simply reporting whatever the last restore in the process did.
    let parsed = PreparedShapes::new(Arc::new(
        parse_shapes(SHAPES, None).expect("the fixture shapes parse"),
    ))
    .bind_shared_dataset(dataset(DATA_TWO_VIOLATIONS))
    .expect("the data graph binds");
    assert_eq!(parsed.provenance(), &ValidatorProvenance::Parsed);
}
