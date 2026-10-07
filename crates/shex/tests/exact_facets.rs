// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Numeric facets compare against their bounds exactly as written.
//!
//! The AST keeps a facet bound as an `i64` or an `f64` (ShExJ's JSON numbers), so a
//! bound like `100000000000000000001` or `5.0000000000000000001` loses its digits
//! there, and distinct bounds can share one AST value. [`ExactSchema`] keeps each
//! bound on the node constraint it was written on, and [`validate_exact`] gives the
//! verdict the bound as written implies, for both schema syntaxes and through
//! imports. Every refusal here runs beside a neighbour that is accepted.

use purrdf_core::TermValue;
use purrdf_core::xsd_regex::xpath::{Limits, Profile};
use purrdf_rdf::parse_dataset;
use purrdf_shex::{
    ConformanceStatus, ExactSchema, ShapeSelector, ShexError, ValidationOptions, XPathValidator,
    to_shexj, validate, validate_exact, validate_exact_with_xpath, validate_shape_map_exact,
    validate_shape_map_exact_with_xpath, validate_with_xpath,
};

use ConformanceStatus::{Conformant, Nonconformant};

const EX: &str = "http://example.org/";

/// The verdict for `ex:x`, whose `ex:n` is `value`, against the shape `ex:<shape>`.
fn verdict_of(schema: &ExactSchema, shape: &str, value: &str) -> ConformanceStatus {
    let data = parse_dataset(
        format!("<{EX}x> <{EX}n> {value} .").as_bytes(),
        "text/turtle",
        None,
    )
    .expect("data parses");
    let map = vec![(
        TermValue::Iri(format!("{EX}x")),
        ShapeSelector::Label(format!("{EX}{shape}")),
    )];
    validate_exact(schema, &data, &map, &ValidationOptions::default()).entries[0].status
}

/// The verdict against the one shape `ex:S`.
fn verdict(schema: &ExactSchema, value: &str) -> ConformanceStatus {
    verdict_of(schema, "S", value)
}

/// `ex:S { ex:n <facet> }` in ShExC.
fn shexc(facet: &str) -> ExactSchema {
    ExactSchema::parse_shexc(&shexc_src(facet), None).expect("schema parses")
}

/// One ShExJ shape declaration `ex:<shape> { ex:n <facets> }`, `facets` being the
/// node constraint's numeric members as JSON (`"maxinclusive":5`).
fn shexj_decl(shape: &str, facets: &str) -> String {
    format!(
        r#"{{"type":"Shape","id":"{EX}{shape}","expression":{{"type":"TripleConstraint",
        "predicate":"{EX}n","valueExpr":{{"type":"NodeConstraint",{facets}}}}}}}"#
    )
}

/// A ShExJ schema of `decls`.
fn shexj_schema(decls: &[String]) -> ExactSchema {
    ExactSchema::parse_shexj(&shexj_doc(decls), None).expect("schema parses")
}

/// The ShExJ document of `decls`.
fn shexj_doc(decls: &[String]) -> String {
    format!(
        r#"{{"@context":"http://www.w3.org/ns/shex.jsonld","type":"Schema","shapes":[{}]}}"#,
        decls.join(",")
    )
}

/// The ShExC source `ex:S { ex:n <facet> }`.
fn shexc_src(facet: &str) -> String {
    format!("<{EX}S> {{ <{EX}n> {facet} }}")
}

/// The ShExJ source `ex:S { ex:n <facet>: <lexeme> }`.
fn shexj_src(facet: &str, lexeme: &str) -> String {
    shexj_doc(&[shexj_decl("S", &format!(r#""{facet}":{lexeme}"#))])
}

/// `ex:S { ex:n <facet>: <lexeme> }` in ShExJ.
fn shexj(facet: &str, lexeme: &str) -> ExactSchema {
    ExactSchema::parse_shexj(&shexj_src(facet, lexeme), None).expect("schema parses")
}

/// `MAXINCLUSIVE 5.0000000000000000001` and `MAXINCLUSIVE 5` are one `5` in the AST.
/// Each constraint keeps its own bound: a value between them conforms to the first
/// and not the second, and `5` conforms to both — whichever is declared first.
#[test]
fn a_lossy_bound_does_not_leak_into_an_unrelated_constraint_shexc() {
    for schema in [
        "<http://example.org/Fine> { <http://example.org/n> MAXINCLUSIVE 5.0000000000000000001 }\n\
         <http://example.org/Five> { <http://example.org/n> MAXINCLUSIVE 5 }",
        "<http://example.org/Five> { <http://example.org/n> MAXINCLUSIVE 5 }\n\
         <http://example.org/Fine> { <http://example.org/n> MAXINCLUSIVE 5.0000000000000000001 }",
    ] {
        let schema = ExactSchema::parse_shexc(schema, None).expect("schema parses");
        let fine = schema
            .schema()
            .shapes
            .iter()
            .find(|d| d.id.ends_with("Fine"));
        let five = schema
            .schema()
            .shapes
            .iter()
            .find(|d| d.id.ends_with("Five"));
        assert_eq!(
            fine.map(|d| &d.expr),
            five.map(|d| &d.expr),
            "both bounds are one AST value"
        );
        let between = "5.00000000000000000005";
        assert_eq!(verdict_of(&schema, "Fine", between), Conformant);
        assert_eq!(verdict_of(&schema, "Five", between), Nonconformant);
        assert_eq!(verdict_of(&schema, "Fine", "5"), Conformant);
        assert_eq!(verdict_of(&schema, "Five", "5"), Conformant);
        assert_eq!(
            verdict_of(&schema, "Fine", "5.00000000000000000015"),
            Nonconformant
        );
    }
}

/// The same collision within one constraint: `MININCLUSIVE 5` and `MAXINCLUSIVE
/// 5.0000000000000000001` share the AST value `5`, and each keeps its own bound.
#[test]
fn two_bounds_of_one_constraint_sharing_an_ast_value_stay_apart() {
    let schema = shexc("MININCLUSIVE 5 MAXINCLUSIVE 5.0000000000000000001");
    assert_eq!(verdict(&schema, "5.00000000000000000005"), Conformant);
    assert_eq!(verdict(&schema, "5"), Conformant);
    assert_eq!(verdict(&schema, "4.99999999999999999999"), Nonconformant);
    assert_eq!(verdict(&schema, "5.00000000000000000015"), Nonconformant);
}

/// The collision in ShExJ: `5.0000000000000000001` is the double `5.0` there, the
/// same `NumericLiteral` the lexeme `5.0` gives, and `5` is the integer `5`.
#[test]
fn a_lossy_bound_does_not_leak_into_an_unrelated_constraint_shexj() {
    let schema = shexj_schema(&[
        shexj_decl("Fine", r#""maxinclusive":5.0000000000000000001"#),
        shexj_decl("Five", r#""maxinclusive":5"#),
        shexj_decl("FivePoint", r#""maxinclusive":5.0"#),
    ]);
    let between = "5.00000000000000000005";
    assert_eq!(verdict_of(&schema, "Fine", between), Conformant);
    assert_eq!(verdict_of(&schema, "Five", between), Nonconformant);
    assert_eq!(verdict_of(&schema, "FivePoint", between), Nonconformant);
    for shape in ["Fine", "Five", "FivePoint"] {
        assert_eq!(verdict_of(&schema, shape, "5"), Conformant, "{shape}");
    }
}

/// `0.29999999999999999` and `0.30000000000000001` are one double (`0.3`). A schema
/// holding both parses — no refusal — and each bound decides by its own digits, on
/// different constraints and on one.
#[test]
fn bounds_sharing_one_double_parse_and_decide_by_their_digits() {
    let schema = ExactSchema::parse_shexc(
        "<http://example.org/Low> { <http://example.org/n> MININCLUSIVE 0.29999999999999999 }\n\
         <http://example.org/High> { <http://example.org/n> MAXINCLUSIVE 0.30000000000000001 }\n\
         <http://example.org/S> { <http://example.org/n> \
             MININCLUSIVE 0.29999999999999999 MAXINCLUSIVE 0.30000000000000001 }",
        None,
    )
    .expect("bounds sharing one double are a valid schema");
    assert_eq!(
        verdict_of(&schema, "Low", "0.29999999999999999"),
        Conformant
    );
    assert_eq!(
        verdict_of(&schema, "Low", "0.29999999999999998"),
        Nonconformant
    );
    assert_eq!(
        verdict_of(&schema, "High", "0.30000000000000001"),
        Conformant
    );
    assert_eq!(
        verdict_of(&schema, "High", "0.30000000000000002"),
        Nonconformant
    );
    assert_eq!(verdict(&schema, "0.3"), Conformant);
    assert_eq!(verdict(&schema, "0.29999999999999998"), Nonconformant);
    assert_eq!(verdict(&schema, "0.30000000000000002"), Nonconformant);

    let schema = shexj_schema(&[
        shexj_decl("Low", r#""mininclusive":0.29999999999999999"#),
        shexj_decl("High", r#""maxinclusive":0.30000000000000001"#),
    ]);
    assert_eq!(verdict_of(&schema, "Low", "0.3"), Conformant);
    assert_eq!(
        verdict_of(&schema, "Low", "0.29999999999999998"),
        Nonconformant
    );
    assert_eq!(verdict_of(&schema, "High", "0.3"), Conformant);
    assert_eq!(
        verdict_of(&schema, "High", "0.30000000000000002"),
        Nonconformant
    );
}

/// Integer bounds past `i64` and past `i128` compare exactly, in both syntaxes.
#[test]
fn integer_bounds_past_machine_words_are_exact() {
    let past_i128 = format!("1{}1", "0".repeat(41));
    let below = format!("1{}", "0".repeat(42));
    for schema in [
        shexc("MININCLUSIVE 100000000000000000001"),
        shexj("mininclusive", "100000000000000000001"),
    ] {
        assert_eq!(verdict(&schema, "100000000000000000000"), Nonconformant);
        assert_eq!(verdict(&schema, "100000000000000000001"), Conformant);
    }
    for schema in [
        shexc(&format!("MININCLUSIVE {past_i128}")),
        shexj("mininclusive", &past_i128),
    ] {
        assert_eq!(verdict(&schema, &below), Nonconformant);
        assert_eq!(verdict(&schema, &past_i128), Conformant);
    }
}

/// A decimal bound compares exactly; a `DOUBLE`-spelled bound (ShExC `1.5E3`, or a
/// ShExJ number with an exponent) is a double and compares under the numeric
/// promotion, as SPARQL's `<=` does — so a decimal just past it still conforms,
/// where the same bound written without an exponent refuses it.
#[test]
fn a_double_bound_keeps_binary_semantics_and_a_decimal_bound_is_exact() {
    let just_past = "1500.0000000000000001";
    for schema in [
        shexc("MAXINCLUSIVE 1.5E3"),
        shexj("maxinclusive", "1.5E3"),
        shexj("maxinclusive", "15e2"),
    ] {
        assert_eq!(verdict(&schema, "1500"), Conformant);
        assert_eq!(verdict(&schema, just_past), Conformant);
        assert_eq!(verdict(&schema, "1501"), Nonconformant);
    }
    for schema in [
        shexc("MAXINCLUSIVE 1500"),
        shexc("MAXINCLUSIVE 1500.0"),
        shexj("maxinclusive", "1500"),
        shexj("maxinclusive", "1500.0"),
    ] {
        assert_eq!(verdict(&schema, "1500"), Conformant);
        assert_eq!(verdict(&schema, just_past), Nonconformant);
    }
    let schema = shexc("MINEXCLUSIVE 0.1");
    assert_eq!(verdict(&schema, "0.1000000000000000000000001"), Conformant);
    assert_eq!(verdict(&schema, "0.1"), Nonconformant);
}

/// The plain AST path keeps its documented `i64`/`f64` behaviour; the exact path is
/// the one the CLI and the Python binding validate through, and the shape-map entry
/// point decides alike.
#[test]
fn the_lossy_ast_path_is_unchanged_and_the_shape_map_path_is_exact() {
    let exact = shexc("MININCLUSIVE 100000000000000000001");
    let data = parse_dataset(
        format!("<{EX}x> <{EX}n> 100000000000000000000 .").as_bytes(),
        "text/turtle",
        None,
    )
    .expect("data parses");
    let map = vec![(
        TermValue::Iri(format!("{EX}x")),
        ShapeSelector::Label(format!("{EX}S")),
    )];
    assert_eq!(
        validate(exact.schema(), &data, &map).entries[0].status,
        Conformant
    );
    assert_eq!(verdict(&exact, "100000000000000000000"), Nonconformant);
    let shape_map = format!("<{EX}x>@<{EX}S>");
    let result = validate_shape_map_exact(
        &exact,
        &data,
        &shape_map,
        None,
        &ValidationOptions::default(),
    )
    .expect("shape map parses");
    assert_eq!(result.entries[0].status, Nonconformant);
    // A code-built schema's i64/f64 bounds are exact as the values they are.
    let built = ExactSchema::from_schema(exact.into_schema());
    assert_eq!(verdict(&built, "100000000000000000000"), Conformant);
}

/// A `start` expression, a referenced node-constraint declaration and an `OR`
/// alternative each keep their own bounds: `4` meets only `MAXINCLUSIVE 5`, the
/// value between the two bounds only `ex:T`.
#[test]
fn bounds_survive_start_and_nesting() {
    let schema = ExactSchema::parse_shexc(
        "start = @<http://example.org/S>\n\
         <http://example.org/S> { <http://example.org/n> @<http://example.org/T> \
             OR MAXINCLUSIVE 5 }\n\
         <http://example.org/T> MAXINCLUSIVE 5.0000000000000000001 AND MININCLUSIVE 5",
        None,
    )
    .expect("schema parses");
    assert_eq!(verdict(&schema, "5.00000000000000000005"), Conformant);
    assert_eq!(verdict(&schema, "4"), Conformant);
    assert_eq!(verdict(&schema, "5.00000000000000000015"), Nonconformant);
}

/// The library schema `<lib>` declaring `ex:T` with `facet`.
fn library(facet: &str) -> Result<ExactSchema, ShexError> {
    ExactSchema::parse_shexc(&format!("<{EX}T> {{ <{EX}n> {facet} }}"), None)
}

/// A root importing `<lib1>` and `<lib2>`, each declaring `ex:T` with its facet.
fn import_both(first: &str, second: &str) -> Result<ExactSchema, ShexError> {
    let root = ExactSchema::parse_shexc(
        &format!("IMPORT <{EX}lib1>\nIMPORT <{EX}lib2>\n<{EX}S> {{ <{EX}n> . }}"),
        None,
    )
    .expect("root parses");
    root.resolve_imports(&|iri| match iri.strip_prefix(EX) {
        Some("lib1") => library(first),
        Some("lib2") => library(second),
        _ => Err(ShexError::shexj(format!("unexpected import {iri}"))),
    })
}

/// Two schemas declaring one label with one AST merge when their exact bounds are one
/// value — however spelled — and keep those bounds; the same AST with bounds of
/// different values is a conflict, beside the merging neighbours.
#[test]
fn imports_merge_equal_exact_bounds_and_refuse_different_ones() {
    let fine = "MAXINCLUSIVE 5.0000000000000000001";
    let merged = import_both(fine, fine).expect("identical declarations merge");
    assert_eq!(
        verdict_of(&merged, "T", "5.00000000000000000005"),
        Conformant
    );
    assert_eq!(
        verdict_of(&merged, "T", "5.00000000000000000015"),
        Nonconformant
    );

    let merged = import_both("MAXINCLUSIVE 5", "MAXINCLUSIVE 5.0")
        .expect("one value spelled two ways merges");
    assert_eq!(verdict_of(&merged, "T", "5"), Conformant);
    assert_eq!(
        verdict_of(&merged, "T", "5.00000000000000000005"),
        Nonconformant
    );

    let conflict = import_both("MAXINCLUSIVE 5", fine);
    assert!(
        matches!(&conflict, Err(ShexError::ImportConflict(label)) if label == &format!("{EX}T")),
        "{conflict:?}"
    );
    let conflict = import_both("MAXINCLUSIVE 1.5", "MAXINCLUSIVE 1.5E0");
    assert!(
        matches!(&conflict, Err(ShexError::ImportConflict(_))),
        "a decimal and a double bound decide differently: {conflict:?}"
    );
}

/// Imported declarations keep their bounds through a nested shape reference.
#[test]
fn bounds_survive_imports() {
    let root = ExactSchema::parse_shexc(
        &format!("IMPORT <{EX}lib>\n<{EX}S> {{ <{EX}n> @<{EX}T> }}\nstart = @<{EX}S>"),
        None,
    )
    .expect("root parses");
    let merged = root
        .resolve_imports(&|iri| {
            assert_eq!(iri, format!("{EX}lib"));
            ExactSchema::parse_shexc(
                &format!(
                    "PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>\n\
                     <{EX}T> xsd:decimal MINEXCLUSIVE 0.1 MAXEXCLUSIVE 0.30000000000000001"
                ),
                None,
            )
        })
        .expect("imports resolve");
    assert_eq!(verdict(&merged, "0.3"), Conformant);
    assert_eq!(verdict(&merged, "0.30000000000000001"), Nonconformant);
    assert_eq!(verdict(&merged, "0.1"), Nonconformant);
}

/// A schema exercising every kind of bound the exact writer spells: lossy decimals,
/// a lossy integer, one sharing a double, a double-spelled bound, an exact bound and
/// one past the `f64` range.
fn every_kind_shexc() -> String {
    format!(
        "PREFIX ex: <http://example.org/>\n\
         ex:Fine {{ ex:n MAXINCLUSIVE 5.0000000000000000001 }}\n\
         ex:Five {{ ex:n MAXINCLUSIVE 5 }}\n\
         ex:Big {{ ex:n MININCLUSIVE 100000000000000000001 }}\n\
         ex:Low {{ ex:n MININCLUSIVE 0.29999999999999999 }}\n\
         ex:High {{ ex:n MAXINCLUSIVE 0.30000000000000001 }}\n\
         ex:Double {{ ex:n MAXINCLUSIVE 1.5E3 }}\n\
         ex:Huge {{ ex:n MININCLUSIVE {} }}\n",
        huge(1)
    )
}

/// `10^399 + plus`: 400 digits, past the `f64` range.
fn huge(plus: u8) -> String {
    format!("1{}{plus}", "0".repeat(398))
}

/// `(shape, value, verdict)` cases that tell every bound of [`every_kind_shexc`] from
/// its `i64`/`f64` approximation.
fn every_kind_cases() -> Vec<(&'static str, String, ConformanceStatus)> {
    let cases = [
        ("Fine", "5.00000000000000000005", Conformant),
        ("Fine", "5.00000000000000000015", Nonconformant),
        ("Five", "5", Conformant),
        ("Five", "5.00000000000000000005", Nonconformant),
        ("Big", "100000000000000000000", Nonconformant),
        ("Big", "100000000000000000001", Conformant),
        ("Low", "0.29999999999999999", Conformant),
        ("Low", "0.29999999999999998", Nonconformant),
        ("High", "0.30000000000000001", Conformant),
        ("High", "0.30000000000000002", Nonconformant),
        ("Double", "1500.0000000000000001", Conformant),
        ("Double", "1501", Nonconformant),
    ];
    let mut cases: Vec<_> = cases
        .into_iter()
        .map(|(shape, value, status)| (shape, value.to_owned(), status))
        .collect();
    cases.push(("Huge", huge(0), Nonconformant));
    cases.push(("Huge", huge(1), Conformant));
    cases.push(("Huge", huge(2), Conformant));
    cases
}

fn assert_every_kind(schema: &ExactSchema, what: &str) {
    for (shape, value, want) in every_kind_cases() {
        assert_eq!(
            verdict_of(schema, shape, &value),
            want,
            "{what}: {value}@{shape}"
        );
    }
}

/// ShExC → exact ShExJ → ShExJ keeps every bound: the reread schema decides every
/// case alike, and writing it again gives the same bytes. The plain writer loses the
/// digits the exact one keeps.
#[test]
fn the_exact_writer_round_trips_shexc_through_shexj() {
    let parsed = ExactSchema::parse_shexc(&every_kind_shexc(), None).expect("schema parses");
    assert_every_kind(&parsed, "ShExC");
    let written = parsed.to_shexj();
    for digits in [
        "5.0000000000000000001",
        "100000000000000000001",
        "0.29999999999999999",
        "0.30000000000000001",
        "1.5E3",
        huge(1).as_str(),
    ] {
        assert!(
            written.contains(digits),
            "{digits} written exactly:\n{written}"
        );
    }
    let plain = to_shexj(parsed.schema());
    assert!(!plain.contains("5.0000000000000000001"), "{plain}");
    assert!(!plain.contains(&huge(1)), "{plain}");

    let reread = ExactSchema::parse_shexj(&written, None).expect("written ShExJ parses");
    assert_every_kind(&reread, "ShExC -> ShExJ");
    assert_eq!(reread.to_shexj(), written, "writing is a fixed point");
}

/// ShExJ → exact ShExJ → ShExJ gives back the same schema with the same exact bounds.
#[test]
fn the_exact_writer_round_trips_shexj() {
    let source = ExactSchema::parse_shexc(&every_kind_shexc(), None)
        .expect("schema parses")
        .to_shexj();
    let parsed = ExactSchema::parse_shexj(&source, None).expect("ShExJ parses");
    let reread = ExactSchema::parse_shexj(&parsed.to_shexj(), None).expect("rewritten parses");
    assert_eq!(reread, parsed, "same AST and same exact bounds");
    assert_every_kind(&reread, "ShExJ -> ShExJ");
}

/// A schema whose bounds the AST holds exactly writes the same bytes through both
/// writers — beside one that does not.
#[test]
fn the_exact_writer_matches_the_plain_one_on_exact_bounds() {
    let schema = ExactSchema::parse_shexc(
        "PREFIX ex: <http://example.org/>\n\
         PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>\n\
         start = @ex:S\n\
         ex:S { ex:n xsd:decimal MININCLUSIVE -5 MAXEXCLUSIVE 0.5 ; \
                ex:m LITERAL MAXINCLUSIVE 9007199254740993 TOTALDIGITS 3 ; \
                ex:o @ex:T OR MINEXCLUSIVE 2.25 }\n\
         ex:T [ex:a ex:b]\n",
        None,
    )
    .expect("schema parses");
    assert_eq!(schema.to_shexj(), to_shexj(schema.schema()));
    let lossy = shexc("MAXINCLUSIVE 0.1000000000000000000001");
    assert_ne!(lossy.to_shexj(), to_shexj(lossy.schema()));
}

/// An `INTEGER`/`DECIMAL` bound past the `f64` range is a valid bound: an
/// [`ExactSchema`] keeps it, in both syntaxes, and writes it back exactly. The plain
/// parsers, whose AST cannot hold it, refuse it — beside a 300-digit bound they
/// accept — and a `DOUBLE` past the range has no `xsd:double` value, so both paths
/// refuse `1e400` beside `1e300`.
#[test]
fn a_bound_past_the_f64_range_is_kept_by_the_exact_parsers_only() {
    let bound = huge(1);
    let negative = format!("-{bound}");
    for schema in [
        shexc(&format!("MININCLUSIVE {bound}")),
        shexj("mininclusive", &bound),
    ] {
        assert_eq!(verdict(&schema, &huge(0)), Nonconformant);
        assert_eq!(verdict(&schema, &bound), Conformant);
        assert_eq!(verdict(&schema, &huge(2)), Conformant);
        assert!(schema.to_shexj().contains(&bound), "{}", schema.to_shexj());
        assert_eq!(
            schema.schema().shapes[0].expr,
            shexc("MININCLUSIVE 1.7976931348623157E308")
                .into_schema()
                .shapes[0]
                .expr,
            "the AST stand-in is f64::MAX"
        );
    }
    for schema in [
        shexc(&format!("MAXINCLUSIVE {negative}.5")),
        shexj("maxinclusive", &format!("{negative}.5")),
    ] {
        assert_eq!(verdict(&schema, &negative), Nonconformant);
        assert_eq!(verdict(&schema, &format!("-{}", huge(2))), Conformant);
    }

    let plain_shexc = |facet: &str| purrdf_shex::parse_shexc(&shexc_src(facet), None);
    let plain_shexj =
        |facet: &str, lexeme: &str| purrdf_shex::parse_shexj(&shexj_src(facet, lexeme), None);
    let three_hundred = format!("1{}", "0".repeat(299));
    assert!(plain_shexc(&format!("MININCLUSIVE {bound}")).is_err());
    assert!(plain_shexc(&format!("MININCLUSIVE {three_hundred}")).is_ok());
    assert!(plain_shexj("mininclusive", &bound).is_err());
    assert!(plain_shexj("mininclusive", &three_hundred).is_ok());

    // A DOUBLE past the double range is no such refusal: it is the infinity of its
    // sign (see `a_double_bound_past_the_double_range_is_infinite`), in every parser.
    for facet in ["MININCLUSIVE 1e400", "MININCLUSIVE 1e300"] {
        assert!(plain_shexc(facet).is_ok(), "{facet}");
        assert!(
            ExactSchema::parse_shexc(&shexc_src(facet), None).is_ok(),
            "{facet}"
        );
    }
    for lexeme in ["1e400", "1e300"] {
        assert!(plain_shexj("mininclusive", lexeme).is_ok(), "{lexeme}");
        let doc = shexj_src("mininclusive", lexeme);
        assert!(ExactSchema::parse_shexj(&doc, None).is_ok(), "{lexeme}");
    }
}

/// A DOUBLE bound past the double range is the infinity of its sign, as the same
/// lexical form is in the data (`"1E400"^^xsd:double` is `INF`): `MAXINCLUSIVE 1E400`
/// admits every finite value and `INF` itself, `MAXEXCLUSIVE 1E400` admits the finite
/// values and not `INF`, and `MININCLUSIVE -1E400` admits everything down to `-INF`.
/// The finite neighbour `MAXINCLUSIVE 1E300` still refuses a larger value. Both
/// syntaxes, and the ShExJ export, agree.
#[test]
fn a_double_bound_past_the_double_range_is_infinite() {
    let past = r#""1E400"^^<http://www.w3.org/2001/XMLSchema#double>"#;
    let inf = r#""INF"^^<http://www.w3.org/2001/XMLSchema#double>"#;
    let neg_inf = r#""-INF"^^<http://www.w3.org/2001/XMLSchema#double>"#;
    let large = r#""1E301"^^<http://www.w3.org/2001/XMLSchema#double>"#;
    let huge_integer = format!("1{}", "0".repeat(500));
    for (max_inclusive, max_exclusive, min_inclusive) in [
        (
            shexc("MAXINCLUSIVE 1E400"),
            shexc("MAXEXCLUSIVE 1E400"),
            shexc("MININCLUSIVE -1E400"),
        ),
        (
            shexj("maxinclusive", "1E400"),
            shexj("maxexclusive", "1E400"),
            shexj("mininclusive", "-1E400"),
        ),
    ] {
        assert_eq!(verdict(&max_inclusive, large), Conformant);
        assert_eq!(verdict(&max_inclusive, &huge_integer), Conformant);
        assert_eq!(verdict(&max_inclusive, inf), Conformant);
        assert_eq!(verdict(&max_inclusive, past), Conformant);
        assert_eq!(verdict(&max_exclusive, large), Conformant);
        assert_eq!(verdict(&max_exclusive, inf), Nonconformant);
        assert_eq!(verdict(&min_inclusive, neg_inf), Conformant);
        assert_eq!(verdict(&min_inclusive, "-5"), Conformant);
    }
    // The finite neighbour keeps its bound.
    let finite = shexc("MAXINCLUSIVE 1E300");
    assert_eq!(verdict(&finite, large), Nonconformant);
    assert_eq!(verdict(&finite, "5"), Conformant);
    // The export writes a JSON number that reads back as the same infinite bound.
    let exported = to_shexj(shexc("MAXINCLUSIVE 1E400").schema());
    let reread = ExactSchema::parse_shexj(&exported, None).expect("the export parses");
    assert_eq!(verdict(&reread, inf), Conformant);
    assert_eq!(verdict(&reread, large), Conformant);
}

/// The data `ex:x ex:n <value>`.
fn data_of(value: &str) -> std::sync::Arc<purrdf_core::RdfDataset> {
    parse_dataset(
        format!("<{EX}x> <{EX}n> {value} .").as_bytes(),
        "text/turtle",
        None,
    )
    .expect("data parses")
}

/// A bound past `i64` and a pattern only XPath 3.1 defines (a non-capturing group), on
/// one node constraint: the exact entries keep the bound's digits AND match the pattern
/// under the selected dated law.
#[test]
fn exact_bounds_hold_under_a_selected_xpath_law() {
    let schema = shexc("MININCLUSIVE 100000000000000000001 /^(?:1)/");
    let map = vec![(
        TermValue::Iri(format!("{EX}x")),
        ShapeSelector::Label(format!("{EX}S")),
    )];
    let options = ValidationOptions::default();
    let under = |value: &str, profile: Profile| {
        validate_exact_with_xpath(
            &schema,
            &data_of(value),
            &map,
            &options,
            profile,
            Limits::new(),
        )
        .expect("no operational refusal")
        .entries[0]
            .status
    };
    // The valid neighbour: the bound itself, matching the 3.1 pattern.
    assert_eq!(under("100000000000000000001", Profile::Xpath31), Conformant);
    // One below the bound fails it exactly, though its double is the bound's.
    assert_eq!(
        under("100000000000000000000", Profile::Xpath31),
        Nonconformant
    );
    // The AST path loses those digits and accepts it: the exact path is what refuses.
    let lossy = validate_with_xpath(
        schema.schema(),
        &data_of("100000000000000000000"),
        &map,
        &options,
        Profile::Xpath31,
        Limits::new(),
    )
    .expect("no operational refusal");
    assert_eq!(lossy.entries[0].status, Conformant);
    // The law is really selected: XPath 2.0 defines no non-capturing group.
    assert_eq!(
        under("100000000000000000001", Profile::Xpath20),
        Nonconformant
    );

    // A reused validator keeps the exact bounds across laws.
    let mut validator = XPathValidator::exact(&schema);
    for (value, profile, expected) in [
        ("100000000000000000001", Profile::Xpath31, Conformant),
        ("100000000000000000000", Profile::Xpath31, Nonconformant),
        ("100000000000000000001", Profile::Xpath20, Nonconformant),
        ("100000000000000000001", Profile::Xpath31, Conformant),
    ] {
        let result = validator
            .validate(&data_of(value), &map, &options, profile, Limits::new())
            .expect("no operational refusal");
        assert_eq!(
            result.entries[0].status, expected,
            "{value} under {profile:?}"
        );
    }
}

/// The shape-map form agrees with the fixed-map form, and still refuses a shape the
/// schema does not declare beside a declared one that is answered.
#[test]
fn exact_shape_maps_hold_under_a_selected_xpath_law() {
    let schema = shexc("MININCLUSIVE 100000000000000000001 /^(?:1)/");
    let options = ValidationOptions::default();
    let map = format!("<{EX}x>@<{EX}S>");
    let status = |value: &str| {
        validate_shape_map_exact_with_xpath(
            &schema,
            &data_of(value),
            &map,
            None,
            &options,
            Profile::Xpath31,
            Limits::new(),
        )
        .expect("the map validates")
        .entries[0]
            .status
    };
    assert_eq!(status("100000000000000000001"), Conformant);
    assert_eq!(status("100000000000000000000"), Nonconformant);
    let undeclared = validate_shape_map_exact_with_xpath(
        &schema,
        &data_of("100000000000000000001"),
        &format!("<{EX}x>@<{EX}Missing>"),
        None,
        &options,
        Profile::Xpath31,
        Limits::new(),
    );
    assert!(matches!(
        undeclared,
        Err(purrdf_shex::XPathValidationError::ShapeMap(
            ShexError::UnknownShape(_)
        ))
    ));
}
