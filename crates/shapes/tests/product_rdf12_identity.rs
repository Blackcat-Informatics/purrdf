// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RDF 1.2 term IDENTITY across a prepared-product round trip.
//!
//! A product is written, opened and admitted; the restored validator must then
//! produce the *same report* as the validator the product was written from. The
//! comparison surface is the report graph's N-Triples, so `sh:focusNode`,
//! `sh:value` and `sh:sourceShape` are all compared at once and byte for byte.
//!
//! # Why the string form is not the property
//!
//! Two RDF 1.2 terms can render alike and still be different terms, and the codec
//! failures worth catching here are exactly the ones that a string comparison
//! cannot see:
//!
//! * a quoted triple restored as *some other* term that prints the same way;
//! * two blank-node-identified shapes restored as one, or one restored as two —
//!   the AST carries a bare label and the DATASET supplies its scope, so a product
//!   that restored the two halves independently could merge or split a shape with
//!   nothing appearing to fail;
//! * a language tag whose subtag sequence was truncated;
//! * a base direction dropped, which MERGES `"x"@ar--ltr` and `"x"@ar--rtl` into
//!   one term.
//!
//! That last one is why a whole-report comparison is never the only assertion
//! below. A dictionary keyed on lexical form plus language alone merges the two
//! directional literals on *both* sides of the round trip, and the two reports
//! then agree perfectly — on the wrong answer. Every direction test therefore
//! interrogates the restored literal's direction independently, through
//! [`Literal::direction`], and every other test likewise asserts the shape of the
//! term it is about rather than only that two reports matched.
//!
//! # Anti-vacuity
//!
//! A round-trip test over a fixture that never contained the feature passes for
//! the wrong reason. Each test therefore first proves its fixture EXERCISES the
//! feature — the report is non-empty, and the term under test really is a quoted
//! triple / a blank node / language-tagged / directional — before it asserts that
//! anything survived.

use std::collections::BTreeSet;
use std::sync::Arc;

use purrdf::{RdfDataset, RdfTextDirection};
use purrdf_shapes::engine::{PreparedShapes, parse_shapes};
use purrdf_shapes::model::sh;
use purrdf_shapes::product::{HostBindings, ShapesProduct, ShapesProfile};
use purrdf_shapes::report::{ValidationReport, ValidationResult};
use purrdf_shapes::term::{Literal, Term};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

// ── Fixtures (example.org, per the repository's fixture rule) ──────────────────

/// The prefix header every fixture below opens with.
const PREFIXES: &str = r"
@prefix sh:  <http://www.w3.org/ns/shacl#> .
@prefix ex:  <http://example.org/ns#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
";

/// The IRI a fixture name expands to.
fn ex(local: &str) -> String {
    format!("http://example.org/ns#{local}")
}

// ── Harness ───────────────────────────────────────────────────────────────────

/// One fixture validated twice: once by the shapes graph as parsed, once by the
/// shapes graph restored from a prepared product written from it.
struct RoundTrip {
    /// The report the PARSED validator produced.
    parsed: ValidationReport,
    /// The report the RESTORED validator produced.
    restored: ValidationReport,
}

/// Prepare `shapes_body`, write it out as a product, admit it back, and validate
/// `data_body` with both validators.
///
/// The byte-identity of the two reports is asserted here, so every test below adds
/// only the assertions that identify WHICH term it is about — and every test gets
/// the non-vacuity floor (a report that actually says something) for free.
fn round_trip(shapes_body: &str, data_body: &str) -> RoundTrip {
    let data = parse_turtle_to_dataset(&format!("{PREFIXES}{data_body}"), None)
        .expect("the fixture data graph parses");
    let shapes =
        parse_shapes(&format!("{PREFIXES}{shapes_body}"), None).expect("the fixture shapes parse");
    let parsed = PreparedShapes::new(Arc::new(shapes));

    let bytes = parsed
        .to_product(&ShapesProfile::CORE)
        .expect("the fixture is representable as a product");
    let restored = ShapesProduct::open(&bytes)
        .expect("the product opens")
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("the product admits");

    let before = validate(&parsed, &data);
    let after = validate(&restored, &data);

    assert!(
        !before.results.is_empty(),
        "anti-vacuity: a fixture that reports nothing proves nothing about what survived a \
         round trip"
    );
    assert_eq!(
        after.to_ntriples(),
        before.to_ntriples(),
        "the restored validator must produce the parsed validator's report, byte for byte"
    );

    RoundTrip {
        parsed: before,
        restored: after,
    }
}

/// Validate `data` with `prepared`.
fn validate(prepared: &PreparedShapes, data: &Arc<RdfDataset>) -> ValidationReport {
    prepared
        .bind_shared_dataset(Arc::clone(data))
        .expect("binding the data graph")
        .validate()
        .expect("validation runs")
}

/// The results a report carries for one constraint component, in report order.
fn results_for<'a>(report: &'a ValidationReport, component: &str) -> Vec<&'a ValidationResult> {
    report
        .results
        .iter()
        .filter(|r| r.source_constraint_component.as_str() == component)
        .collect()
}

/// Every `sh:value` a report carries, in report order.
fn values(report: &ValidationReport) -> Vec<&Term> {
    report
        .results
        .iter()
        .filter_map(|r| r.value.as_ref())
        .collect()
}

/// The literal behind a term, or a failure naming what was found instead.
fn literal(term: &Term) -> &Literal {
    match term {
        Term::Literal(literal) => literal,
        other => panic!("expected a literal, found {other}"),
    }
}

/// Every DISTINCT `sh:sourceShape` blank-node label a report carries.
///
/// Every result is required to be sourced from a blank node: a fixture whose
/// shapes are IRI-identified would make the distinctness assertions below say
/// nothing about blank-node scoping.
fn blank_source_labels(report: &ValidationReport) -> BTreeSet<&str> {
    report
        .results
        .iter()
        .map(|r| match &r.source_shape {
            Term::BlankNode(label) => label.as_str(),
            other => panic!("expected a blank-node-identified source shape, found {other}"),
        })
        .collect()
}

/// Assert that `term` is the quoted triple `<<( ex:s ex:p ex:o )>>` — structurally,
/// not by its rendering.
fn assert_is_quoted_s_p_o(term: &Term, what: &str) {
    let Term::Triple(triple) = term else {
        panic!("{what} must be a quoted triple, found {term}");
    };
    assert_eq!(triple.subject, Term::NamedNode(ex("s").as_str().into()));
    assert_eq!(triple.predicate.as_str(), ex("p"));
    assert_eq!(triple.object, Term::NamedNode(ex("o").as_str().into()));
}

// ── 1. Quoted triples ─────────────────────────────────────────────────────────

/// A quoted triple used as `sh:targetNode`.
const QUOTED_TARGET_SHAPES: &str = r"
ex:QuotedTargetShape a sh:NodeShape ;
    sh:targetNode <<( ex:s ex:p ex:o )>> ;
    sh:nodeKind sh:IRI .
";

/// The data graph that asserts the quoted triple, so the target resolves against an
/// interned term rather than a term only the shapes graph ever mentions.
const QUOTED_TARGET_DATA: &str = r"
ex:witness ex:asserts <<( ex:s ex:p ex:o )>> .
";

/// A quoted triple reaching the report as `sh:focusNode` survives the round trip AS
/// a quoted triple.
///
/// `sh:nodeKind sh:IRI` is the instrument: a quoted triple is not an IRI, so the
/// target fires exactly one violation whose focus node IS the quoted triple. A
/// product that restored the target as anything else — a blank node standing in for
/// the statement, an IRI, a literal rendering of it — would either target nothing
/// (an empty report, refused by the harness) or report a different focus node.
#[test]
fn quoted_triple_target_identity_survives() {
    let run = round_trip(QUOTED_TARGET_SHAPES, QUOTED_TARGET_DATA);

    // Anti-vacuity: the fixture really did target a quoted triple, and the report
    // really does carry one.
    assert_eq!(
        run.parsed.results.len(),
        1,
        "the quoted-triple target selects exactly one focus node"
    );
    assert_is_quoted_s_p_o(&run.parsed.results[0].focus_node, "the parsed focus node");

    // The property: the restored validator reports the SAME quoted triple, still a
    // quoted triple after the round trip.
    assert_eq!(run.restored.results.len(), 1);
    assert_is_quoted_s_p_o(
        &run.restored.results[0].focus_node,
        "the restored focus node",
    );
    assert_eq!(
        run.restored.results[0].focus_node, run.parsed.results[0].focus_node,
        "the focus node is the same term on both sides"
    );
}

/// A quoted triple used as a constraint VALUE (`sh:hasValue`), alongside a
/// `sh:nodeKind` shape that surfaces every value node as `sh:value`.
const QUOTED_VALUE_SHAPES: &str = r"
ex:ClaimShape a sh:NodeShape ;
    sh:targetClass ex:Claim ;
    sh:property [ sh:path ex:says ; sh:hasValue <<( ex:s ex:p ex:o )>> ] ;
    sh:property [ sh:path ex:says ; sh:nodeKind sh:IRI ] .
";

/// Two claims quoting two DIFFERENT triples: one the constraint requires, one it
/// does not.
const QUOTED_VALUE_DATA: &str = r"
ex:matching    a ex:Claim ; ex:says <<( ex:s ex:p ex:o )>> .
ex:mismatching a ex:Claim ; ex:says <<( ex:s ex:p ex:other )>> .
";

/// A quoted triple held as a CONSTRAINT VALUE keeps the identity that makes it
/// discriminate between two quoted triples.
///
/// The `sh:hasValue` parameter is the quoted triple, and the data supplies one claim
/// that quotes it and one that quotes a triple differing only in its object. So the
/// constraint must fire on exactly one of the two focus nodes. A product that
/// coarsened the parameter (restoring it as its rendering, or as a term that
/// compares equal to both) would fire on neither or on both.
#[test]
fn quoted_triple_constraint_value_survives() {
    let run = round_trip(QUOTED_VALUE_SHAPES, QUOTED_VALUE_DATA);

    for (side, report) in [("parsed", &run.parsed), ("restored", &run.restored)] {
        // Anti-vacuity: the values reaching the report really are quoted triples,
        // and they really are two DIFFERENT ones.
        let node_kind = results_for(report, sh::NODE_KIND_CONSTRAINT_COMPONENT);
        assert_eq!(
            node_kind.len(),
            2,
            "{side}: both claims' quoted-triple objects are non-IRI value nodes"
        );
        let quoted: BTreeSet<String> = node_kind
            .iter()
            .map(|r| {
                let value = r
                    .value
                    .as_ref()
                    .expect("sh:nodeKind reports the value node");
                assert!(
                    matches!(value, Term::Triple(_)),
                    "{side}: the value node must be a quoted triple, found {value}"
                );
                value.to_string()
            })
            .collect();
        assert_eq!(
            quoted.len(),
            2,
            "{side}: the two claims quote two distinct triples"
        );

        // The property: the constraint value discriminated, and it discriminated
        // the right way round.
        let has_value = results_for(report, sh::HAS_VALUE_CONSTRAINT_COMPONENT);
        assert_eq!(
            has_value.len(),
            1,
            "{side}: exactly one claim fails `sh:hasValue <<( ex:s ex:p ex:o )>>`"
        );
        assert_eq!(
            has_value[0].focus_node,
            Term::NamedNode(ex("mismatching").as_str().into()),
            "{side}: the claim that quotes a DIFFERENT triple is the one that fails"
        );
    }
}

// ── 2. Scoped blank nodes ─────────────────────────────────────────────────────

/// Four blank-node-identified shapes: two nested `sh:property [ … ]` shapes that
/// both violate, and two `sh:node [ … ]` shapes that must not be confused with each
/// other.
const BLANK_DISTINCT_SHAPES: &str = r"
ex:GateShape a sh:NodeShape ;
    sh:targetClass ex:Gate ;
    sh:property [ sh:path ex:left  ; sh:minCount 1 ] ;
    sh:property [ sh:path ex:right ; sh:minCount 1 ] ;
    sh:property [ sh:path ex:alpha ;
                  sh:node [ a sh:NodeShape ; sh:property [ sh:path ex:x ; sh:minCount 1 ] ] ] ;
    sh:property [ sh:path ex:beta ;
                  sh:node [ a sh:NodeShape ; sh:property [ sh:path ex:y ; sh:minCount 1 ] ] ] .
";

/// One gate with neither `ex:left` nor `ex:right`, whose `ex:alpha` and `ex:beta`
/// both point at a node carrying `ex:x` but not `ex:y`.
const BLANK_DISTINCT_DATA: &str = r#"
ex:gate a ex:Gate ; ex:alpha ex:onlyX ; ex:beta ex:onlyX .
ex:onlyX ex:x "present" .
"#;

/// Blank-node-identified shapes that are DISTINCT stay distinct across the round
/// trip — neither merged into one nor confused with one another.
///
/// Two independent instruments, because a blank shape can be observed two ways:
///
/// * the two `sh:property [ … ]` shapes reach the report as `sh:sourceShape`, so
///   their distinctness is directly visible as two distinct blank labels;
/// * the two `sh:node [ … ]` shapes never reach the report (a `sh:node` violation
///   is sourced from the property shape that carries it), so they are observed
///   BEHAVIOURALLY: `ex:onlyX` satisfies the `ex:x` shape and fails the `ex:y` one,
///   and both are reached from the same data node. Merge them and the report gains
///   or loses a result depending on which way they merged.
#[test]
fn nested_blank_property_shapes_stay_distinct() {
    let run = round_trip(BLANK_DISTINCT_SHAPES, BLANK_DISTINCT_DATA);

    for (side, report) in [("parsed", &run.parsed), ("restored", &run.restored)] {
        // Anti-vacuity: every result really is sourced from a BLANK-node-identified
        // shape (`blank_source_labels` refuses any other source shape), and there
        // really are several of them.
        let labels = blank_source_labels(report);
        assert_eq!(
            report.results.len(),
            3,
            "{side}: two missing-cardinality results and one `sh:node` result"
        );
        assert_eq!(
            labels.len(),
            3,
            "{side}: three distinct blank shapes sourced three results, not one shape three \
             times: {labels:?}"
        );

        // The `sh:node` half: exactly one of the two inner shapes rejected
        // `ex:onlyX`, and it was the one demanding `ex:y`.
        let node_results = results_for(report, sh::NODE_CONSTRAINT_COMPONENT);
        assert_eq!(
            node_results.len(),
            1,
            "{side}: `ex:onlyX` conforms to the `ex:x` shape and fails the `ex:y` shape, so the \
             two `sh:node` shapes are two shapes"
        );
        assert_eq!(
            node_results[0].result_path,
            Some(Term::NamedNode(ex("beta").as_str().into())),
            "{side}: the failing `sh:node` is the one reached through `ex:beta`"
        );
        assert_eq!(
            node_results[0].value,
            Some(Term::NamedNode(ex("onlyX").as_str().into())),
            "{side}: the rejected value node is `ex:onlyX`"
        );
    }
}

/// ONE nested blank property shape, reached from two focus nodes and reporting on
/// two value nodes.
const BLANK_SHARED_SHAPES: &str = r"
ex:TaggedShape a sh:NodeShape ;
    sh:targetClass ex:Tagged ;
    sh:property [ sh:path ex:tag ; sh:nodeKind sh:IRI ] .
";

/// Two tagged nodes, three literal tags between them — three violations, all from
/// the one blank property shape.
const BLANK_SHARED_DATA: &str = r#"
ex:first  a ex:Tagged ; ex:tag "one", "two" .
ex:second a ex:Tagged ; ex:tag "three" .
"#;

/// A blank-node-identified shape that is ONE shape stays one — it is not split into
/// a copy per result, per focus node, or per reference.
///
/// The split is the mirror of the merge in
/// [`nested_blank_property_shapes_stay_distinct`], and it hides just as well: a
/// product that restored the property shape's bare label without its dataset scope
/// could mint a fresh shape at each use, and every violation would still be
/// reported — just attributed to a shape nobody wrote. Three results carrying one
/// `sh:sourceShape` is what distinguishes the two outcomes.
#[test]
fn one_blank_shape_stays_one() {
    let run = round_trip(BLANK_SHARED_SHAPES, BLANK_SHARED_DATA);

    for (side, report) in [("parsed", &run.parsed), ("restored", &run.restored)] {
        // Anti-vacuity: three results really were reported, from two distinct focus
        // nodes, and each really is sourced from a blank node.
        assert_eq!(
            report.results.len(),
            3,
            "{side}: three literal tags violate"
        );
        let focus: BTreeSet<String> = report
            .results
            .iter()
            .map(|r| r.focus_node.to_string())
            .collect();
        assert_eq!(
            focus.len(),
            2,
            "{side}: the one shape was reached from two focus nodes"
        );

        // The property: one shape, however many results it sourced.
        let labels = blank_source_labels(report);
        assert_eq!(
            labels.len(),
            1,
            "{side}: three results from ONE blank property shape, not three copies of it: \
             {labels:?}"
        );
    }

    assert_eq!(
        blank_source_labels(&run.restored),
        blank_source_labels(&run.parsed),
        "the restored shape carries the label the parsed shape carried"
    );
}

// ── 3. Language-tagged literals ───────────────────────────────────────────────

/// A `sh:in` over a subtag-rich language-tagged literal, plus a `sh:nodeKind` shape
/// that surfaces every value node as `sh:value`.
const LANG_SHAPES: &str = r#"
ex:DocShape a sh:NodeShape ;
    sh:targetClass ex:Doc ;
    sh:property [ sh:path ex:label ; sh:in ( "colour"@en-GB-oxendict ) ] ;
    sh:property [ sh:path ex:label ; sh:nodeKind sh:IRI ] .
"#;

/// The same lexical form under three language tags, two of which are PREFIXES of
/// the third. Truncating the subtag sequence anywhere would collide them.
const LANG_DATA: &str = r#"
ex:doc a ex:Doc ; ex:label "colour"@en-GB-oxendict , "colour"@en-GB , "colour"@en .
"#;

/// A language tag's full SUBTAG SEQUENCE survives the round trip, and keeps
/// separating literals that differ only in their trailing subtags.
///
/// NOTE on casing: language tags are normalized to lowercase at intern time, so the
/// authored `@en-GB-oxendict` interns — on both sides of the round trip, and
/// identically — as `@en-gb-oxendict`. The authored casing is therefore NOT the
/// property under test and asserting it would be asserting a bug. The subtag
/// SEQUENCE is: `en-gb-oxendict` must not come back as `en`, as `en-gb`, or with
/// its subtags reordered.
#[test]
fn lang_tagged_literal_with_subtags_survives() {
    let run = round_trip(LANG_SHAPES, LANG_DATA);

    for (side, report) in [("parsed", &run.parsed), ("restored", &run.restored)] {
        // Anti-vacuity: the values reaching the report really are language-tagged,
        // and the three tags really are three distinct tags.
        let node_kind = results_for(report, sh::NODE_KIND_CONSTRAINT_COMPONENT);
        assert_eq!(
            node_kind.len(),
            3,
            "{side}: all three language-tagged literals are non-IRI value nodes"
        );
        let tags: BTreeSet<&str> = node_kind
            .iter()
            .map(|r| {
                let value = r
                    .value
                    .as_ref()
                    .expect("sh:nodeKind reports the value node");
                let literal = literal(value);
                assert_eq!(
                    literal.value(),
                    "colour",
                    "{side}: the three literals share one lexical form, so only the tag can \
                     separate them"
                );
                literal
                    .language()
                    .unwrap_or_else(|| panic!("{side}: the value node must carry a language tag"))
            })
            .collect();
        assert_eq!(
            tags.len(),
            3,
            "{side}: three distinct language tags, so no prefix collapsed into another: {tags:?}"
        );

        // The property, asserted on the tag itself: the subtag sequence is intact.
        assert!(
            tags.contains("en-gb-oxendict"),
            "{side}: the three-subtag tag survives in full: {tags:?}"
        );
        let subtags: Vec<&str> = "en-gb-oxendict".split('-').collect();
        assert_eq!(
            subtags,
            ["en", "gb", "oxendict"],
            "{side}: the subtag sequence is ordered, not a set"
        );

        // …and behaviourally: `sh:in` holds the three-subtag literal, so exactly the
        // two shorter-tagged literals fail it. A truncated tag on either side would
        // move that count.
        let in_results = results_for(report, sh::IN_CONSTRAINT_COMPONENT);
        assert_eq!(
            in_results.len(),
            2,
            "{side}: only `\"colour\"@en-gb-oxendict` is in the permitted set"
        );
        let refused: BTreeSet<&str> = in_results
            .iter()
            .map(|r| {
                literal(r.value.as_ref().expect("sh:in reports the value node"))
                    .language()
                    .expect("the refused value node carries a language tag")
            })
            .collect();
        assert_eq!(
            refused,
            BTreeSet::from(["en", "en-gb"]),
            "{side}: the two prefix-tagged literals are the ones refused"
        );
    }
}

// ── 4. Base direction ─────────────────────────────────────────────────────────

/// A `sh:in` over a left-to-right directional literal, plus a `sh:nodeKind` shape
/// that surfaces every value node as `sh:value`.
const DIR_SHAPES: &str = r#"
ex:NoteShape a sh:NodeShape ;
    sh:targetClass ex:Note ;
    sh:property [ sh:path ex:text ; sh:in ( "note"@ar--ltr ) ] ;
    sh:property [ sh:path ex:text ; sh:nodeKind sh:IRI ] .
"#;

/// One lexical form, one language tag, BOTH base directions. A dictionary keyed on
/// lexical form plus language alone sees one term here instead of two.
const DIR_DATA: &str = r#"
ex:note a ex:Note ; ex:text "note"@ar--ltr , "note"@ar--rtl .
"#;

/// The directional literals a report carries, as `(language, direction)` pairs,
/// after proving each really is language-tagged and really carries a direction.
fn directions(report: &ValidationReport, side: &str) -> Vec<(String, RdfTextDirection)> {
    results_for(report, sh::NODE_KIND_CONSTRAINT_COMPONENT)
        .iter()
        .map(|r| {
            let literal = literal(
                r.value
                    .as_ref()
                    .expect("sh:nodeKind reports the value node"),
            );
            assert_eq!(
                literal.value(),
                "note",
                "{side}: one lexical form, so only the direction can separate the two"
            );
            let language = literal
                .language()
                .unwrap_or_else(|| panic!("{side}: an `rdf:dirLangString` carries a language tag"))
                .to_owned();
            let direction = literal.direction().unwrap_or_else(|| {
                panic!("{side}: the value node must carry a base direction, found {literal:?}")
            });
            (language, direction)
        })
        .collect()
}

/// Assert the two directional literals survived as TWO terms, and hand back the
/// restored report's `(language, direction)` pairs.
///
/// Shared by both direction tests because the merge they exist to catch is
/// symmetric: dropping the direction fuses `@ar--ltr` and `@ar--rtl`, and the
/// whole-report comparison then passes on both sides because both merged
/// identically. Only a direct reading of [`Literal::direction`] can tell the
/// difference, so both tests take one.
fn surviving_directions(run: &RoundTrip) -> Vec<(String, RdfTextDirection)> {
    // Anti-vacuity: the fixture really produced two directional literals, not one
    // merged term, on the side the product was written FROM.
    let parsed = directions(&run.parsed, "parsed");
    assert_eq!(
        parsed.len(),
        2,
        "the fixture must carry two distinct directional literals, or a merge would be \
         invisible: {parsed:?}"
    );

    let restored = directions(&run.restored, "restored");
    assert_eq!(
        restored.len(),
        2,
        "the restored validator must still see two terms where the fixture wrote two: \
         {restored:?}"
    );
    assert_eq!(
        restored, parsed,
        "each restored literal keeps the direction the parsed literal carried"
    );
    restored
}

/// `rdf:dirLangString` with `ltr` survives the round trip WITH its direction.
///
/// The direction is read off the restored literal directly rather than inferred
/// from the report comparison: a codec that dropped the base direction would merge
/// `"note"@ar--ltr` and `"note"@ar--rtl` into one term on both sides, and two
/// identically-merged reports compare equal. This test can only pass if the
/// direction is genuinely there.
#[test]
fn dir_lang_string_ltr_survives() {
    let run = round_trip(DIR_SHAPES, DIR_DATA);
    let restored = surviving_directions(&run);

    assert!(
        restored.contains(&("ar".to_owned(), RdfTextDirection::Ltr)),
        "the left-to-right literal survives as a left-to-right literal: {restored:?}"
    );

    // …and behaviourally: `sh:in` permits exactly the LTR literal, so the RTL one is
    // the only value node it refuses. A dropped direction would refuse neither.
    let refused = results_for(&run.restored, sh::IN_CONSTRAINT_COMPONENT);
    assert_eq!(
        refused.len(),
        1,
        "`sh:in ( \"note\"@ar--ltr )` admits the LTR literal, so it refuses exactly one value"
    );
    assert_eq!(
        literal(
            refused[0]
                .value
                .as_ref()
                .expect("sh:in reports the value node")
        )
        .direction(),
        Some(RdfTextDirection::Rtl),
        "the refused value is the RTL literal, which is what makes the LTR one admitted"
    );
}

/// `rdf:dirLangString` with `rtl` survives the round trip WITH its direction.
///
/// The mirror of [`dir_lang_string_ltr_survives`], and for the same reason: the
/// direction is asserted on the restored literal itself, because a merge of the two
/// directions is invisible to a report comparison.
#[test]
fn dir_lang_string_rtl_survives() {
    let run = round_trip(DIR_SHAPES, DIR_DATA);
    let restored = surviving_directions(&run);

    assert!(
        restored.contains(&("ar".to_owned(), RdfTextDirection::Rtl)),
        "the right-to-left literal survives as a right-to-left literal: {restored:?}"
    );

    // The RTL literal is the one `sh:in` refuses, and it is refused BECAUSE its
    // direction differs from the permitted term's — same lexical form, same tag.
    let refused = results_for(&run.restored, sh::IN_CONSTRAINT_COMPONENT);
    assert_eq!(refused.len(), 1, "exactly one value node is refused");
    let value = literal(
        refused[0]
            .value
            .as_ref()
            .expect("sh:in reports the value node"),
    );
    assert_eq!(value.value(), "note");
    assert_eq!(value.language(), Some("ar"));
    assert_eq!(
        value.direction(),
        Some(RdfTextDirection::Rtl),
        "direction is part of the literal's identity, so the RTL term is not the permitted LTR one"
    );

    // The two directions did not collapse into one term.
    let ltr = values(&run.restored)
        .into_iter()
        .filter(
            |term| matches!(term, Term::Literal(l) if l.direction() == Some(RdfTextDirection::Ltr)),
        )
        .count();
    assert!(
        ltr > 0,
        "the LTR literal is still present alongside the RTL one"
    );
}
