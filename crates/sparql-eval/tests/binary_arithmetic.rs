// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `xsd:double` and `xsd:float` arithmetic returns the IEEE-754 bits from query text, on
//! every target.
//!
//! Each query below is built on a double-rounding witness: an operand pair whose exact
//! result lies within a rounding of the x87's register format of a binary64 (binary32)
//! midpoint without being one, so a unit that rounds twice -- the x87 at its default
//! 64-bit precision, or at 53 bits in the subnormal range -- answers other bits. The
//! integer software reference (`purrdf_xsd::ieee::reference`) is the oracle: it shows
//! each witness IS one (the doubly rounded result differs), and gives the correctly
//! rounded bits the query must return as its canonical lexical form. On an IEEE unit
//! these pass because the unit is IEEE; on the x87 (`i586`, or `i686` without SSE2) they
//! pass because every operation runs through `purrdf_xsd::ieee`'s precision guard and
//! subnormal scaling.

use std::fmt::Write as _;
use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, SparqlRequest, SparqlResult, TermValue};
use purrdf_sparql_eval::{AggregateRegistry, ExtensionEnv, NativeSparqlEngine, QueryOptions};
use purrdf_xsd::ieee::reference as soft;
use purrdf_xsd::numeric::{canonical_double, canonical_float};

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const STAT_NS: &str = "http://example.org/agg/";

fn empty() -> Arc<RdfDataset> {
    RdfDatasetBuilder::new().freeze().expect("an empty dataset")
}

/// Every row of `query`, over the empty dataset, under `options`.
fn run(query: &str, options: QueryOptions<'_>) -> Vec<Vec<Option<TermValue>>> {
    let dataset = empty();
    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            &*dataset,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            options,
        )
        .unwrap_or_else(|e| panic!("evaluate `{query}`: {e}"));
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected solutions");
    };
    rows
}

fn lexical(cell: Option<&TermValue>) -> (String, String) {
    match cell {
        Some(TermValue::Literal {
            lexical_form,
            datatype,
            ..
        }) => (lexical_form.clone(), datatype.clone()),
        other => panic!("expected a bound literal, got {other:?}"),
    }
}

/// The one value `SELECT (<expression> AS ?y) {}` binds: its lexical form and datatype.
fn select(expression: &str) -> (String, String) {
    let rows = run(
        &format!("SELECT ({expression} AS ?y) WHERE {{}}"),
        QueryOptions::EMPTY,
    );
    assert_eq!(rows.len(), 1, "{expression}");
    lexical(rows[0][0].as_ref())
}

/// `x` written as the typed literal a query spells it with.
fn double(x: f64) -> String {
    format!("\"{}\"^^<{XSD}double>", canonical_double(x))
}

fn float(x: f32) -> String {
    format!("\"{}\"^^<{XSD}float>", canonical_float(x))
}

/// The binary64 witnesses: `(expression, a, b, the reference's once-rounded result,
/// the result rounded through the x87's register format)`.
fn double_witnesses() -> Vec<(&'static str, f64, f64, f64, f64)> {
    let one = 1.0_f64;
    let succ = f64::from_bits(one.to_bits() + 1);
    let a_little_more = f64::from_bits(0x3ca0_0000_0800_0000); // 2^-53 + 2^-78
    let a_little_less = f64::from_bits(0x3c9f_ffff_f000_0000); // 2^-53 - 2^-78
    let pred = f64::from_bits(one.to_bits() - 1); // 1 - 2^-53
    let succ2 = f64::from_bits(one.to_bits() + 2); // 1 + 2^-51
    let tiny_a = f64::from_bits(((1023 - 512) << 52) + 4); // 2^-512 (1 + 4 ulp)
    let tiny_b = f64::from_bits(((1023 - 511) << 52) - 2); // 2^-511 (1 - ulp)
    let tiny = f64::from_bits((1023 - 512) << 52); // 2^-512
    let huge = f64::from_bits(((1023 + 511) << 52) - 2); // 2^511 (1 - ulp)
    vec![
        (
            "+",
            one,
            a_little_more,
            soft::add(one, a_little_more),
            soft::add_via(one, a_little_more, soft::X87_EXTENDED),
        ),
        (
            "-",
            succ,
            -a_little_less,
            soft::sub(succ, -a_little_less),
            soft::add_via(succ, a_little_less, soft::X87_EXTENDED),
        ),
        (
            "*",
            pred,
            succ2,
            soft::mul(pred, succ2),
            soft::mul_via(pred, succ2, soft::X87_EXTENDED),
        ),
        (
            "*",
            tiny_a,
            tiny_b,
            soft::mul(tiny_a, tiny_b),
            soft::mul_via(tiny_a, tiny_b, soft::X87_DOUBLE),
        ),
        (
            "/",
            tiny,
            huge,
            soft::div(tiny, huge),
            soft::div_via(tiny, huge, soft::X87_DOUBLE),
        ),
    ]
}

/// The binary32 witnesses, subnormal products and quotients that round twice at the
/// x87's 24-bit precision without the scaling.
fn float_witnesses() -> Vec<(&'static str, f32, f32, f32, f32)> {
    let tiny_a = f32::from_bits(((127 - 64) << 23) + 4); // 2^-64 (1 + 4 ulp)
    let tiny_b = f32::from_bits(((127 - 63) << 23) - 2); // 2^-63 (1 - ulp)
    let tiny = f32::from_bits((127 - 64) << 23); // 2^-64
    let huge = f32::from_bits(((127 + 63) << 23) - 2); // 2^63 (1 - ulp)
    vec![
        (
            "*",
            tiny_a,
            tiny_b,
            soft::mul32(tiny_a, tiny_b),
            soft::mul32_via(tiny_a, tiny_b, soft::X87_SINGLE),
        ),
        (
            "/",
            tiny,
            huge,
            soft::div32(tiny, huge),
            soft::div32_via(tiny, huge, soft::X87_SINGLE),
        ),
    ]
}

/// Every double witness, from query text, answers the correctly rounded lexical form.
fn assert_double_witnesses() {
    for (op, a, b, once, twice) in double_witnesses() {
        assert_ne!(
            once.to_bits(),
            twice.to_bits(),
            "{a:e} {op} {b:e} is a double-rounding witness"
        );
        let expression = format!("({} {op} {})", double(a), double(b));
        assert_eq!(
            select(&expression),
            (canonical_double(once), format!("{XSD}double")),
            "{expression}"
        );
    }
}

fn assert_float_witnesses() {
    for (op, a, b, once, twice) in float_witnesses() {
        assert_ne!(
            once.to_bits(),
            twice.to_bits(),
            "{a:e} {op} {b:e} is a double-rounding witness"
        );
        assert!(once.is_subnormal());
        let expression = format!("({} {op} {})", float(a), float(b));
        assert_eq!(
            select(&expression),
            (canonical_float(once), format!("{XSD}float")),
            "{expression}"
        );
    }
}

#[test]
fn double_arithmetic_on_double_rounding_witnesses_returns_the_ieee_lexical() {
    // Pinned in words as well as through the reference: `1 + (2^-53 + 2^-78)` is the
    // successor of one, which a doubly rounded sum loses.
    let little = canonical_double(f64::from_bits(0x3ca0_0000_0800_0000));
    assert_eq!(
        select(&format!(
            "(\"1.0E0\"^^<{XSD}double> + \"{little}\"^^<{XSD}double>)"
        )),
        ("1.0000000000000002E0".to_owned(), format!("{XSD}double"))
    );
    // The valid neighbour: `1 + 2^-53` exactly is a tie, which goes to even (one) on
    // every unit -- a result the doubly rounding x87 also gets right, so the witness
    // above, not this row, is what can tell the two apart.
    let half_ulp = canonical_double(f64::EPSILON / 2.0);
    assert_eq!(
        select(&format!(
            "(\"1.0E0\"^^<{XSD}double> + \"{half_ulp}\"^^<{XSD}double>)"
        )),
        ("1.0E0".to_owned(), format!("{XSD}double"))
    );
    assert_double_witnesses();
}

#[test]
fn float_arithmetic_on_double_rounding_witnesses_returns_the_ieee_lexical() {
    assert_float_witnesses();
}

#[test]
fn integer_and_decimal_operands_promote_into_the_same_correctly_rounded_double() {
    // `xsd:integer + xsd:double`: the integer is promoted exactly, then one rounding.
    let little = double(f64::from_bits(0x3ca0_0000_0800_0000));
    assert_eq!(
        select(&format!("(1 + {little})")),
        ("1.0000000000000002E0".to_owned(), format!("{XSD}double"))
    );
    // `AVG` divides the correctly rounded sum by the count: `(1 + ulp) / 2`, where a
    // doubly rounded sum of 1 would give `0.5`.
    let rows = run(
        &format!(
            "SELECT (AVG(?x) AS ?y) WHERE {{ VALUES ?x {{ \"1.0E0\"^^<{XSD}double> {little} }} }}"
        ),
        QueryOptions::EMPTY,
    );
    assert_eq!(
        lexical(rows[0][0].as_ref()),
        (
            canonical_double(soft::div(
                soft::add(1.0, f64::from_bits(0x3ca0_0000_0800_0000)),
                2.0
            )),
            format!("{XSD}double")
        )
    );
    assert_eq!(lexical(rows[0][0].as_ref()).0, "5.000000000000001E-1");
}

#[test]
fn stddev_is_the_correctly_rounded_square_root_of_the_variance() {
    // Groups `{0, 0, k}` of integers: the variance `(k² − k²/3) / 3` stays in the exact
    // decimal tower, eighteen fractional digits, so its double has a full significand,
    // and `STDDEV_POP` is that double's square root, rounded once. Some `k` make the root
    // a double-rounding witness through the x87's 64-bit register (a short significand
    // cannot be one: its square root is never that close to a midpoint); the count below
    // asserts the fixture holds some, so the test observes the defect it guards.
    let mut registry = AggregateRegistry::new();
    registry.register_statistical_aggregates(STAT_NS);
    let env = ExtensionEnv::over_aggregates(registry).expect("the statistical set reads cleanly");
    let options = QueryOptions::new().with_env(&env);
    // About one double in four thousand has a square root that rounds twice through 64
    // bits; this window holds two (`k` = 17 327 and 18 933).
    let groups = 17_000..=19_000_u32;
    let mut values = String::new();
    for k in groups.clone() {
        for value in [0, 0, k] {
            write!(values, "({k} {value}) ").expect("a String accepts every write");
        }
    }
    let query = format!(
        "SELECT ?g (AGG(<{STAT_NS}VAR_POP>, ?v) AS ?var) (AGG(<{STAT_NS}STDDEV_POP>, ?v) AS ?sd) \
         WHERE {{ VALUES (?g ?v) {{ {values} }} }} GROUP BY ?g ORDER BY ?g"
    );
    let rows = run(&query, options);
    assert_eq!(rows.len(), groups.clone().count());
    let mut witnesses = 0;
    for (row, k) in rows.iter().zip(groups) {
        let (var_lexical, datatype) = lexical(row[1].as_ref());
        assert_eq!(
            datatype,
            format!("{XSD}decimal"),
            "VAR_POP of {{0, 0, {k}}}"
        );
        // The exact decimal's correctly rounded double, by the standard library's parser.
        let variance: f64 = var_lexical.parse().expect("a decimal lexical");
        let root = soft::sqrt(variance);
        assert_eq!(
            lexical(row[2].as_ref()),
            (canonical_double(root), format!("{XSD}double")),
            "STDDEV_POP of {{0, 0, {k}}}, variance {var_lexical}"
        );
        witnesses +=
            usize::from(soft::sqrt_via(variance, soft::X87_EXTENDED).to_bits() != root.to_bits());
    }
    assert_eq!(
        witnesses, 2,
        "the fixture holds its two square-root witnesses"
    );
}

/// On the x87, a caller that left the control word at another precision is served the
/// same bits: every operation sets the precision its format needs and gives the caller's
/// back.
#[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
#[test]
fn a_callers_x87_precision_does_not_reach_the_results() {
    use purrdf_xsd::ieee::x87;

    struct Loaded(u16);
    impl Drop for Loaded {
        fn drop(&mut self) {
            // SAFETY: restores the word this thread had.
            unsafe { x87::load_control_word(self.0) };
        }
    }

    let saved = x87::control_word();
    for precision in [x87::PRECISION_24, x87::PRECISION_53, x87::PRECISION_64] {
        let word = (saved & !x87::PRECISION_CONTROL) | precision;
        // SAFETY: only the precision field differs from this thread's word, and `Loaded`
        // restores it on every exit.
        unsafe { x87::load_control_word(word) };
        let loaded = Loaded(saved);
        assert_double_witnesses();
        assert_float_witnesses();
        assert_eq!(
            x87::control_word(),
            word,
            "the caller's word is theirs again"
        );
        drop(loaded);
    }
    assert_eq!(x87::control_word(), saved);
}
