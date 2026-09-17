// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The extension seams, through the one choke point they share.
//!
//! `STRLANG` asks the grammar and `RdfLiteral::validate_components` asks it on
//! the way into a dataset, but a host that injects Rust reaches neither: a
//! `NativeFunction`, a `ServiceResolver`, a `PropertyFunction` and a
//! `CustomAggregate` each hand the evaluator a whole `TermValue`, and the
//! `language` field of that value is a plain `Option<String>` with no constraint
//! on it anywhere in those four traits. The value then goes straight to the
//! results writers, which spell a tag as `"x"@<tag>` in TSV, as
//! `"xml:lang": "<tag>"` in JSON and as `xml:lang="<tag>"` in XML — bytes no
//! reader takes back.
//!
//! All four converge on `ScratchInterner::intern_checked`, the arena's door for
//! a caller-supplied `TermValue`, and that is where the grammar is asked. This
//! file drives the seam a host reaches most easily, a native function, from
//! real query text through the public engine — and it drives it with BOTH
//! halves, because a gate that refused `x-purrdf-afrikaans` or `en-fr-jura`
//! would be a worse bug than the one it closes.

use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::{
    Arity, NativeSparqlEngine, QueryOptions, ShaclPrebinding, UserFunctionRegistry, Volatility,
};

/// The fixture namespace (AGENTS.md: fixtures live under `example.org`).
const EX: &str = "https://example.org/langtag#";

/// Tags real data carries, which the seam must keep binding.
const ACCEPTED: &[&str] = &[
    "en",
    "en-US",
    "zh-Hans-CN",
    "de-CH-x-phonebk",
    "i-enochian",
    "x-purrdf-afrikaans",
    "x-gmeow-english",
    "en-fr-jura",
    "fr-be-fbcl",
    "abcdefgh",
    "en-x-cantbethislong",
];

/// Tags no RDF concrete syntax would have lexed.
const REFUSED: &[&str] = &[
    "en us",
    "1",
    "9-9",
    "123-456",
    "en-",
    "-",
    "!!!",
    "abcdefghi",
];

/// A single-quad dataset, so the query below evaluates over exactly one row and
/// an unbound answer is distinguishable from no answer at all.
fn dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&format!("{EX}s"));
    let p = b.intern_iri(&format!("{EX}p"));
    let o = b.intern_iri(&format!("{EX}o"));
    b.push_quad(s, p, o, None);
    b.freeze().expect("freeze the fixture")
}

/// Register `ex:tagged()` as a native function that returns
/// `"purr"@<tag>` — a `TermValue` the evaluator did not build and never
/// type-checked — then evaluate `SELECT (ex:tagged() AS ?v) WHERE { ?s ?p ?o }`
/// and report what `?v` was bound to.
fn tagged_by_a_native_function(tag: &str) -> Option<TermValue> {
    let value = TermValue::Literal {
        lexical_form: "purr".to_owned(),
        datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
        language: Some(tag.to_owned()),
        direction: None,
    };
    let mut functions = UserFunctionRegistry::new();
    functions.register_native(
        format!("{EX}tagged"),
        Arity::Exact(0),
        Volatility::Stable,
        Arc::new(move |_args: &[&TermValue]| Ok(Some(value.clone()))),
    );

    let dataset = dataset();
    let query = format!("SELECT (<{EX}tagged>() AS ?v) WHERE {{ ?s ?p ?o }}");
    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            &*dataset,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                functions: &functions,
                ..QueryOptions::EMPTY
            },
        )
        .expect("the query evaluates whatever the function returned");
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected solutions");
    };
    assert_eq!(rows.len(), 1, "the fixture always yields exactly one row");
    rows[0].first().cloned().flatten()
}

#[test]
fn a_native_function_cannot_bind_an_ungrammatical_language_tag() {
    for tag in REFUSED {
        assert_eq!(
            tagged_by_a_native_function(tag),
            None,
            "a native function returning {tag:?} must leave the expression unbound \
             (SPARQL 1.1 §17.2), never write it into a results row"
        );
    }
}

#[test]
fn a_native_function_still_binds_every_tag_real_data_carries() {
    for tag in ACCEPTED {
        let bound = tagged_by_a_native_function(tag);
        assert_eq!(
            bound,
            Some(TermValue::Literal {
                lexical_form: "purr".to_owned(),
                datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
                language: Some((*tag).to_owned()),
                direction: None,
            }),
            "{tag} is a tag real data carries and must still bind, verbatim"
        );
    }
}

/// The row a refused value would have occupied is still a row: the solution is
/// one binding short, not missing, and every other variable keeps its value.
/// That is the difference between §17.2's unbound result and a dropped row.
#[test]
fn a_refused_tag_costs_the_binding_and_nothing_else() {
    let value = TermValue::Literal {
        lexical_form: "purr".to_owned(),
        datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
        language: Some("en us".to_owned()),
        direction: None,
    };
    let mut functions = UserFunctionRegistry::new();
    functions.register_native(
        format!("{EX}tagged"),
        Arity::Exact(0),
        Volatility::Stable,
        Arc::new(move |_args: &[&TermValue]| Ok(Some(value.clone()))),
    );

    let dataset = dataset();
    let query = format!("SELECT ?s (<{EX}tagged>() AS ?v) WHERE {{ ?s ?p ?o }}");
    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            &*dataset,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                functions: &functions,
                ..QueryOptions::EMPTY
            },
        )
        .expect("the query evaluates");
    let SparqlResult::Solutions {
        variables, rows, ..
    } = result
    else {
        panic!("expected solutions");
    };
    assert_eq!(variables.len(), 2, "both projected variables are declared");
    assert_eq!(rows.len(), 1, "the row survives the refusal");
    assert_eq!(
        rows[0][0],
        Some(TermValue::Iri(format!("{EX}s"))),
        "?s keeps its binding"
    );
    assert_eq!(rows[0][1], None, "?v alone is unbound");
}

/// The other half of "the gate is the grammar, not a list": the verdict the
/// interner reaches is the verdict `purrdf_iri::langtag` reaches on the profile
/// every other stage names, over every tag both lists hold. Without this, the
/// two could drift apart in silence — a refusal is a claim, and this is the
/// claim's oracle.
#[test]
fn the_seam_verdict_is_the_grammars_verdict() {
    for tag in ACCEPTED.iter().chain(REFUSED) {
        let grammar = purrdf_iri::langtag::is_well_formed_with(
            tag,
            purrdf_iri::langtag::Profile::ConcreteSyntaxLangtagBounded,
        );
        assert_eq!(
            tagged_by_a_native_function(tag).is_some(),
            grammar,
            "the seam and the grammar must agree about {tag:?}"
        );
    }
}

/// The OTHER caller-supplied `TermValue` ingress:
/// `SparqlRequest::substitutions`, the SHACL focus-node pre-binding. It is
/// public API, it carries an unconstrained `Option<String>` language, and it
/// reaches the algebra through `substitute::literal_from_value` — which is why
/// the gate there refuses with a diagnostic rather than degrading the value.
///
/// Two tagged literals on the same predicate, so a pre-binding of `?v` is a real
/// constraint whose loss is VISIBLE as extra rows rather than as a missing one.
fn two_tagged_objects() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s1 = b.intern_iri(&format!("{EX}s1"));
    let s2 = b.intern_iri(&format!("{EX}s2"));
    let p = b.intern_iri(&format!("{EX}p"));
    let purr = b.intern_literal(RdfLiteral::language_tagged("purr", "en"));
    let meow = b.intern_literal(RdfLiteral::language_tagged("meow", "en"));
    b.push_quad(s1, p, purr, None);
    b.push_quad(s2, p, meow, None);
    b.freeze().expect("freeze the fixture")
}

/// Run `SELECT ?s ?v WHERE { ?s ?p ?v }` with `?v` pre-bound to `"purr"@<tag>`,
/// returning the row count or the refusal's diagnostic code.
fn prebind_purr_tagged(tag: &str) -> Result<usize, String> {
    prebind_purr_tagged_via(tag, ShaclPrebinding::None)
}

/// [`prebind_purr_tagged`] over either pre-binding rewrite.
///
/// `apply_shacl_prebinding` is the SHACL `$this` door, and it needs no gate of
/// its own: it calls `apply_substitutions` first and then routes every value
/// through `ground_term_from_value` a second time to build the expression-position
/// constant, so both of its uses of a caller's `TermValue` pass the one ingress.
/// Driving it here proves that rather than asserting it.
fn prebind_purr_tagged_via(tag: &str, prebinding: ShaclPrebinding) -> Result<usize, String> {
    let dataset = two_tagged_objects();
    let substitutions = [(
        "v".to_owned(),
        TermValue::Literal {
            lexical_form: "purr".to_owned(),
            datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
            language: Some(tag.to_owned()),
            direction: None,
        },
    )];
    let result = NativeSparqlEngine::new().query_with_options_view(
        &*dataset,
        SparqlRequest {
            query: "SELECT ?s ?v WHERE { ?s ?p ?v }",
            base_iri: None,
            substitutions: &substitutions,
        },
        QueryOptions {
            prebinding,
            ..QueryOptions::EMPTY
        },
    );
    match result {
        Ok(SparqlResult::Solutions { rows, .. }) => Ok(rows.len()),
        Ok(other) => panic!("expected solutions, got {other:?}"),
        Err(diagnostic) => Err(diagnostic.code),
    }
}

/// A pre-binding that cannot be made into a term is REPORTED. It must never
/// become an `UNDEF` `VALUES` cell, because that is compatible with everything:
/// the constraint would vanish and the query would answer with the whole
/// relation — more rows than the caller asked for, silently.
#[test]
fn an_ungrammatical_pre_binding_is_refused_not_silently_widened() {
    assert_eq!(
        prebind_purr_tagged("en"),
        Ok(1),
        "a grammatical tag that matches constrains the answer to one row"
    );
    assert_eq!(
        prebind_purr_tagged("fr"),
        Ok(0),
        "a grammatical tag that matches nothing still constrains — to no rows"
    );
    for tag in REFUSED {
        assert_eq!(
            prebind_purr_tagged(tag),
            Err("native-sparql-subst-langtag".to_owned()),
            "the pre-binding {tag:?} must be refused by code, never widen the answer"
        );
    }
}

/// The same three cases through the SHACL `$this` door, which shares the ingress.
#[test]
fn the_shacl_pre_binding_door_shares_the_ingress() {
    assert_eq!(
        prebind_purr_tagged_via("en", ShaclPrebinding::Applied),
        Ok(1),
        "a grammatical focus value constrains to one row"
    );
    assert_eq!(
        prebind_purr_tagged_via("fr", ShaclPrebinding::Applied),
        Ok(0),
        "a grammatical focus value that matches nothing still constrains"
    );
    for tag in REFUSED {
        assert_eq!(
            prebind_purr_tagged_via(tag, ShaclPrebinding::Applied),
            Err("native-sparql-subst-langtag".to_owned()),
            "the SHACL focus value {tag:?} must be refused by the same code"
        );
    }
}

/// A zero-length path over a ground endpoint that is ABSENT from the data still
/// yields its reflexive row (SPARQL 1.1 §18.5.1: `(x, x)` is a solution for a
/// ground `x` whether or not `x` occurs in the graph). The endpoint is interned
/// unconditionally for exactly this reason — at that position the only thing a
/// refusal could cost is the whole row, and no refusal in this crate costs a row.
#[test]
fn a_zero_length_path_over_an_absent_tagged_endpoint_keeps_its_row() {
    let dataset = two_tagged_objects();
    let query = format!("SELECT ?s WHERE {{ ?s <{EX}p>* \"absent\"@en }}");
    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            &*dataset,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
        )
        .expect("the query evaluates");
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected solutions");
    };
    assert_eq!(rows.len(), 1, "the zero-length identity row survives");
    assert_eq!(
        rows[0][0],
        Some(TermValue::Literal {
            lexical_form: "absent".to_owned(),
            datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
            language: Some("en".to_owned()),
            direction: None,
        }),
        "?s is bound to the absent endpoint itself"
    );
}

/// The third ingress for an algebra ground term, and the reason `eval_values`
/// and `eval_path` may keep interning on the plain door: a `Query` a host built
/// in Rust — bypassing the SPARQL parser entirely — is judged by
/// `purrdf_sparql_algebra`'s algebra validator at admission, on the same
/// profile, BEFORE any of it can reach a solution row.
///
/// The accept half is driven with it, because an admission gate that refused
/// `en` would take every property path down with it.
#[test]
fn a_hand_built_algebra_is_judged_at_admission() {
    use purrdf_sparql_algebra::{
        GraphPattern, Literal, NamedNode, PropertyPathExpression, Query, QueryDataset, TermPattern,
        Variable,
    };

    let admit = |tag: &str| {
        let subject = Variable::new("s".to_owned());
        let query = Query::Select {
            pattern: GraphPattern::Project {
                inner: Box::new(GraphPattern::Path {
                    subject: TermPattern::Variable(subject.clone()),
                    path: PropertyPathExpression::ZeroOrMore(Box::new(
                        PropertyPathExpression::NamedNode(
                            NamedNode::new(format!("{EX}p")).expect("a valid predicate IRI"),
                        ),
                    )),
                    object: TermPattern::Literal(Literal::new_lang("absent", tag, None)),
                }),
                variables: vec![subject],
            },
            dataset: QueryDataset::default(),
            base_iri: None,
            version: None,
        };
        purrdf_sparql_eval::PreparedQuery::rewritten(query, QueryOptions::EMPTY)
            .map(|_| ())
            .map_err(|diagnostic| diagnostic.code)
    };

    for tag in ACCEPTED {
        assert_eq!(admit(tag), Ok(()), "{tag} must still be admitted");
    }
    for tag in REFUSED {
        assert_eq!(
            admit(tag),
            Err("native-sparql-algebra".to_owned()),
            "{tag:?} must be refused at admission, before it can reach a row"
        );
    }
}

/// The mirror half: every tag real data carries must still pre-bind, and a
/// pre-binding that matches nothing must still narrow to zero rows rather than
/// erroring. Over-refusal here would break every host passing a focus node.
#[test]
fn every_tag_real_data_carries_still_pre_binds() {
    for tag in ACCEPTED {
        let rows = prebind_purr_tagged(tag)
            .unwrap_or_else(|code| panic!("{tag} must be admitted as a pre-binding, got {code}"));
        assert_eq!(
            rows,
            usize::from(*tag == "en"),
            "{tag} must constrain: one row when it matches the data, none when it does not"
        );
    }
}
