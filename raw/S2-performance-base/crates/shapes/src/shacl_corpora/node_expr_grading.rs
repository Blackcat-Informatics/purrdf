// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **The one grader for W3C SHACL 1.2 `sht:EvalNodeExpr` entries**, shared by every
//! harness that runs them — the library harness (`w3c12_conformance.rs`) and the
//! command-line harness in `purrdf-cli`, which runs the same entries through the built
//! binary — so the two cannot mean different things by "the output agrees", and the
//! entries whose approved result spells a decimal non-canonically are one table, not
//! two.
//!
//! The output must equal the `mf:result` list TERM FOR TERM (RDF 1.2 term equality:
//! lexical form, datatype, language and direction), in order unless the entry sets
//! `sht:ignoreOrder true`, in which case as a multiset ([`compare_outputs`]). The only
//! departure from exact equality is [`NON_CANONICAL_EXPECTATIONS`], applied by
//! [`canonical_expectation`].

use crate::term::{Literal, Term};

/// The clause every current entry cites. XSD 1.1 Part 2 §3.3.3.1 states that
/// "for integers, the decimal point and fractional part are prohibited" in the
/// canonical representation and that the mapping "is given formally in
/// decimalCanonicalMap", which §E.1 defines: "If d is an integer, then return
/// noDecimalPtCanonicalMap(d)". SPARQL's CEIL, FLOOR, ROUND, numeric division,
/// SECONDS and SUM over decimals all yield `xsd:decimal`.
pub const DECIMAL_CANONICAL_MAP: &str = "XSD 1.1 Part 2 §3.3.3.1 + §E.1 decimalCanonicalMap: \
     an integer-valued xsd:decimal maps through noDecimalPtCanonicalMap (no decimal point)";

/// NON-CANONICAL EXPECTED DECIMALS: node-expression entries whose approved result
/// spells a computed `xsd:decimal` in a lexical form that is not its XSD 1.1 canonical
/// form: `(test id, expected lexical, XSD 1.1 canonical lexical, canonical-mapping
/// clause)`.
///
/// The expected VALUE is correct in every entry. Each is an integer-valued decimal the
/// approved result spells `"4.0"`, `"3.0"`, `"42.0"` or `"00"`, where the XSD 1.1
/// canonical form is `"4"`, `"3"`, `"42"` or `"0"`. (`"00"` is the canonical form in
/// neither XSD 1.1 nor XSD 1.0, whose canonical decimal zero is `"0.0"`.) The suite says
/// the output must be "equal" to the expected result without saying whether that means
/// term equality or value equality. PurRDF emits the XSD 1.1 canonical form, as the
/// approved W3C SPARQL suite itself expects of CEIL, FLOOR, ROUND and SECONDS
/// (`functions#ceil01`, `floor01`, `round01`, `seconds` expect `"3"`, `"2"`, `"1"`,
/// `"0"`).
///
/// Each entry is graded by substituting the canonical spelling for the expected one and
/// comparing the output against THAT, term for term. For these six, where the two
/// spellings name the same value, that is exactly value comparison; it is not a general
/// relaxation to value equality, which would also accept a non-canonical engine output.
/// They are counted on their own scoreboard line, `non-canonical-expected-decimal`,
/// pinned by [`NON_CANONICAL_EXPECTATIONS_COUNT`], and never among the passes of the
/// approved suite as written. The table is checked in both directions:
/// `w3c12_conformance::non_canonical_expectations_are_really_non_canonical` proves each
/// expected form is non-canonical and that the stated canonical form is what the XSD 1.1
/// canonical mapping produces, and each harness proves the engine's output equals the
/// canonical form term for term.
pub const NON_CANONICAL_EXPECTATIONS: &[(&str, &str, &str, &str)] = &[
    (
        "node-expr/shnex-sparql/ceil-example",
        "4.0",
        "4",
        DECIMAL_CANONICAL_MAP,
    ),
    (
        "node-expr/shnex-sparql/floor-example",
        "3.0",
        "3",
        DECIMAL_CANONICAL_MAP,
    ),
    (
        "node-expr/shnex-sparql/round-example",
        "4.0",
        "4",
        DECIMAL_CANONICAL_MAP,
    ),
    (
        "node-expr/shnex-sparql/divide-example",
        "42.0",
        "42",
        DECIMAL_CANONICAL_MAP,
    ),
    (
        "node-expr/shnex-sparql/seconds-example",
        "00",
        "0",
        DECIMAL_CANONICAL_MAP,
    ),
    (
        "node-expr/shnex/sum-totalRevenue",
        "42.0",
        "42",
        DECIMAL_CANONICAL_MAP,
    ),
];

/// [`NON_CANONICAL_EXPECTATIONS`] pinned by count, so an entry cannot be added or
/// dropped without this number moving with it.
pub const NON_CANONICAL_EXPECTATIONS_COUNT: usize = 6;

/// Replace each expected literal an entry of [`NON_CANONICAL_EXPECTATIONS`]
/// names with its canonical form. An entry that matches no expected literal of
/// its test is a stale entry and an error.
pub fn canonical_expectation(id: &str, expected: &[Term]) -> Result<Vec<Term>, String> {
    let Some((_, lexical, canonical, _)) = NON_CANONICAL_EXPECTATIONS
        .iter()
        .find(|(entry, ..)| *entry == id)
    else {
        return Ok(expected.to_vec());
    };
    let mut replaced = 0usize;
    let out = expected
        .iter()
        .map(|term| match term {
            Term::Literal(l) if l.value() == *lexical && l.language().is_none() => {
                replaced += 1;
                Term::Literal(Literal::new_typed_literal(*canonical, l.datatype()))
            }
            other => other.clone(),
        })
        .collect();
    if replaced == 0 {
        return Err(format!(
            "NON_CANONICAL_EXPECTATIONS names {id} with expected lexical {lexical:?}, which \
             is not among its expected results — stale entry"
        ));
    }
    Ok(out)
}

/// Compare produced node-expression output against the expected list: exact RDF
/// 1.2 term equality, in order, or as a multiset under `ignore_order`.
pub fn compare_outputs(
    produced: &[Term],
    expected: &[Term],
    ignore_order: bool,
) -> Result<(), String> {
    let agree = if ignore_order {
        let mut p: Vec<&Term> = produced.iter().collect();
        let mut e: Vec<&Term> = expected.iter().collect();
        p.sort_by_key(ToString::to_string);
        e.sort_by_key(ToString::to_string);
        p == e
    } else {
        produced == expected
    };
    if agree {
        return Ok(());
    }
    let render = |terms: &[Term]| {
        terms
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    };
    Err(format!(
        "node-expression output mismatch{}:\n  produced ( {} )\n  expected ( {} )",
        if ignore_order { " (order ignored)" } else { "" },
        render(produced),
        render(expected)
    ))
}

/// The focus node of an entry that gives no `sht:focusNode`: a blank node each harness
/// proves occurs nowhere in the test graph (see `w3c12_conformance.rs`'s module docs).
pub const ABSENT_FOCUS: &str = "shacl12-harness-absent-focus-node";
