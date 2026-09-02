// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Prove the two built-in-function tables cannot drift apart.
//!
//! `parser::builtin_function_keyword` — the name-to-function seam SHACL 1.2 Node
//! Expressions §5's `sparql:<NAME>` call form resolves through — documents itself
//! as answering "from the very table `builtin_function` resolves the grammar
//! with, so … the two can never drift apart into 'the resolver claims a function
//! the parser then rejects'".
//!
//! That claim was true and untested, and it is only true in ONE direction.
//! `serialize::function_keyword` is compiler-exhaustive over `Function` (it ends
//! `Function::Purrdf(_) | Function::Custom(_) => return None`, with no `_` arm),
//! so a new variant is FORCED to declare a keyword. `parser::builtin_function`
//! matches on `&str` and ends `_ => return None`, so a new variant is forced into
//! nothing. Add `Function::Foo` plus `"FOO"` to the serializer and the workspace
//! compiles green while `FOO(…)` is unparseable and `sparql:foo` resolves to
//! `None` — a built-in reachable only by writing it, never by naming it.
//!
//! A `&str` match cannot be made exhaustive, so the binding is made here instead:
//! the two tables are read out of their own source text and their `Function`
//! variant sets compared. A variant in one table and not the other fails this
//! test, naming it.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// Every `Function::Variant` named inside the body of `fn_name` in `source`.
///
/// The body is taken from the function signature to the next line that starts a
/// new top-level item (`^}` closes it), which is enough structure for two flat
/// `match` tables and keeps this from needing a Rust parser.
fn variants_in_table(source: &str, fn_name: &str) -> BTreeSet<String> {
    let start = source
        .find(fn_name)
        .unwrap_or_else(|| panic!("{fn_name} must exist in its source file"));
    let body = &source[start..];
    let end = body
        .find("\n}\n")
        .unwrap_or_else(|| panic!("{fn_name} must have a closing brace at column 0"));
    let body = &body[..end];

    let mut found = BTreeSet::new();
    let mut rest = body;
    while let Some(at) = rest.find("Function::") {
        rest = &rest[at + "Function::".len()..];
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            found.insert(name);
        }
    }
    found
}

/// The parser's name table and the serializer's keyword table must name exactly
/// the same `Function` variants.
///
/// The two documented exclusions are the IRI-spelled variants: `Function::Purrdf`
/// and `Function::Custom` are not keyword-callable, so the serializer names them
/// only to return `None` and the parser never names them at all.
#[test]
fn the_parser_and_serializer_function_tables_name_the_same_variants() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let parser =
        fs::read_to_string(root.join("src/parser.rs")).expect("parser.rs must be readable");
    let serialize =
        fs::read_to_string(root.join("src/serialize.rs")).expect("serialize.rs must be readable");

    let mut parsed = variants_in_table(&parser, "fn builtin_function(upper: &str)");
    let mut emitted = variants_in_table(&serialize, "pub(crate) fn function_keyword(f: &Function)");

    for iri_spelled in ["Purrdf", "Custom"] {
        parsed.remove(iri_spelled);
        emitted.remove(iri_spelled);
    }

    let only_emitted: Vec<&String> = emitted.difference(&parsed).collect();
    assert!(
        only_emitted.is_empty(),
        "these Function variants have a serializer keyword but no parser name, so they are \
         unparseable and unreachable through `sparql:<NAME>`: {only_emitted:?}"
    );
    let only_parsed: Vec<&String> = parsed.difference(&emitted).collect();
    assert!(
        only_parsed.is_empty(),
        "these Function variants are parseable but have no canonical keyword, so \
         `builtin_function_keyword` would answer `None` for a name the parser accepts: \
         {only_parsed:?}"
    );
    // A non-trivial table, so a scrape that silently found nothing cannot pass.
    assert!(
        parsed.len() > 50,
        "the scrape found only {} variants, which cannot be the whole built-in table — the \
         table's shape changed and this test is no longer reading it",
        parsed.len()
    );
}

/// Every name the parser accepts round-trips to a keyword the parser also
/// accepts, and that keyword is the CANONICAL spelling.
///
/// This is the property `sparql:<NAME>` actually relies on: a resolver answer must
/// be a token the grammar takes. It is checked over the parser's own name list,
/// scraped from the same table, so it cannot go stale as the table grows.
#[test]
fn every_parseable_builtin_name_round_trips_to_a_canonical_keyword() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let parser =
        fs::read_to_string(root.join("src/parser.rs")).expect("parser.rs must be readable");
    let start = parser
        .find("fn builtin_function(upper: &str)")
        .expect("the parser name table must exist");
    let body = &parser[start..];
    let body = &body[..body.find("\n}\n").expect("table must close")];

    // Each arm is `"NAME" => Function::Variant,`.
    let names: Vec<String> = body
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix('"')?;
            let close = rest.find('"')?;
            rest[close..]
                .contains("=>")
                .then(|| rest[..close].to_owned())
        })
        .collect();
    assert!(
        names.len() > 50,
        "the scrape found only {} names, which cannot be the whole built-in table",
        names.len()
    );

    for name in &names {
        let keyword = purrdf_sparql_algebra::builtin_function_keyword(name).unwrap_or_else(|| {
            panic!("the parser accepts `{name}` but the name-to-keyword seam refuses it")
        });
        // The keyword the seam returns must itself resolve — that is what makes
        // the answer usable as query text rather than merely non-empty.
        let again = purrdf_sparql_algebra::builtin_function_keyword(keyword).unwrap_or_else(|| {
            panic!("`{name}` resolved to `{keyword}`, which the seam then refuses")
        });
        assert_eq!(
            keyword, again,
            "resolution must be idempotent: `{name}` -> `{keyword}` -> `{again}`"
        );
    }

    // Case-insensitivity is part of the contract, and the lowercase spelling is
    // the one `sparql:<NAME>` actually supplies.
    for name in &names {
        assert_eq!(
            purrdf_sparql_algebra::builtin_function_keyword(&name.to_ascii_lowercase()),
            purrdf_sparql_algebra::builtin_function_keyword(name),
            "`{name}` must resolve the same in either case"
        );
    }
}

/// The neighbouring INVALID case: a name that is NOT a keyword-callable built-in
/// still answers `None`. Widening the table would have made the sweep above pass
/// vacuously.
#[test]
fn a_name_that_is_not_a_callable_builtin_still_resolves_to_none() {
    for name in [
        "NOT_A_FUNCTION",
        // Grammar productions and operators, which are not function CALLS: there
        // is no keyword for the seam to return and a caller must spell them.
        "BOUND",
        "IF",
        "COALESCE",
        "EXISTS",
        "sameTerm",
        "COUNT",
        "SUM",
        "+",
        "&&",
        "",
    ] {
        assert_eq!(
            purrdf_sparql_algebra::builtin_function_keyword(name),
            None,
            "`{name}` is not a keyword-callable built-in function"
        );
    }
}
