// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! THE CENSUS IS TOTAL: every `sh:` / `shnex:` term the engine can meet is
//! classified, and the two universes it is total over are read out of the files
//! themselves — never out of a second, hand-maintained list.
//!
//! * The vocabulary half: every term the vendored `shacl.ttl` and `shnex.ttl`
//!   define ([`purrdf_shapes::spec::declared_terms`] reads the RDF).
//! * The engine half: every `sh::` / `shnex::` string constant
//!   `crates/shapes/src/model.rs` declares — every term PurRDF reads anywhere,
//!   SHACL Advanced Features and SHACL-SPARQL 1.0 spellings included — scraped
//!   with `syn`, so a constant added to `model.rs` without a census row fails
//!   here, naming it.
//!
//! And the mirror: a census row outside both universes must be one of the SHACL
//! JavaScript Extensions terms, which neither declares and the census names only
//! to refuse.

use std::collections::{BTreeMap, BTreeSet};

use purrdf_shapes::spec::census::{Role, TermClass, census, classify};
use purrdf_shapes::spec::declared_terms;

/// Every string constant declared in `mod {module}` of `model.rs`, as
/// `(name, value)`, excluding the namespace constant `NS` itself.
fn model_constants(module: &str) -> BTreeMap<String, String> {
    let source = include_str!("../src/model.rs");
    let file = syn::parse_file(source).expect("model.rs parses as Rust");
    let mut out = BTreeMap::new();
    for item in &file.items {
        let syn::Item::Mod(module_item) = item else {
            continue;
        };
        if module_item.ident != module {
            continue;
        }
        let Some((_, items)) = &module_item.content else {
            panic!("mod {module} in model.rs has no inline body");
        };
        for inner in items {
            let syn::Item::Const(konst) = inner else {
                continue;
            };
            let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(value),
                ..
            }) = konst.expr.as_ref()
            else {
                panic!("sh::{} is not a string literal", konst.ident);
            };
            if konst.ident != "NS" {
                out.insert(konst.ident.to_string(), value.value());
            }
        }
    }
    assert!(
        !out.is_empty(),
        "the scrape found no constants in mod {module} of model.rs, so this census test reads \
         nothing"
    );
    out
}

/// The SHACL JavaScript Extensions terms: in neither universe, classified only so
/// the loader can refuse them by name.
const SHACL_JS: [&str; 13] = [
    "http://www.w3.org/ns/shacl#js",
    "http://www.w3.org/ns/shacl#jsFunctionName",
    "http://www.w3.org/ns/shacl#jsLibrary",
    "http://www.w3.org/ns/shacl#jsLibraryURL",
    "http://www.w3.org/ns/shacl#JSConstraint",
    "http://www.w3.org/ns/shacl#JSConstraintComponent",
    "http://www.w3.org/ns/shacl#JSExecutable",
    "http://www.w3.org/ns/shacl#JSFunction",
    "http://www.w3.org/ns/shacl#JSLibrary",
    "http://www.w3.org/ns/shacl#JSRule",
    "http://www.w3.org/ns/shacl#JSTarget",
    "http://www.w3.org/ns/shacl#JSTargetType",
    "http://www.w3.org/ns/shacl#JSValidator",
];

#[test]
fn every_declared_vocabulary_term_is_classified() {
    let terms = declared_terms().expect("the vendored vocabularies read");
    assert!(
        terms.len() > 250,
        "the vocabulary scan found only {} terms, which cannot be SHACL 1.2 Core plus Node \
         Expressions",
        terms.len()
    );
    let missing: Vec<&String> = terms.iter().filter(|iri| classify(iri).is_none()).collect();
    assert!(
        missing.is_empty(),
        "vocabulary terms with no census class: {missing:?}"
    );
}

#[test]
fn every_model_constant_is_classified() {
    let mut missing: Vec<String> = Vec::new();
    for module in ["sh", "shnex"] {
        for (name, iri) in model_constants(module) {
            if classify(&iri).is_none() {
                missing.push(format!("{module}::{name} = {iri}"));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "model.rs constants with no census class: {missing:?}"
    );
}

/// The mirror of the two totality tests: the census classifies nothing outside
/// the two universes except the SHACL-JS terms it names to refuse.
#[test]
fn the_census_classifies_nothing_outside_the_two_universes_but_shacl_js() {
    let mut universe: BTreeSet<String> = declared_terms().expect("the vocabularies read");
    for module in ["sh", "shnex"] {
        universe.extend(model_constants(module).into_values());
    }
    let outside: BTreeSet<&str> = census()
        .iter()
        .map(|row| row.iri)
        .filter(|iri| !universe.contains(*iri))
        .collect();
    let expected: BTreeSet<&str> = SHACL_JS.into_iter().collect();
    assert_eq!(outside, expected);
    for iri in SHACL_JS {
        assert!(
            matches!(
                classify(iri).map(|row| row.class),
                Some(TermClass::Unimplemented(_))
            ),
            "{iri} must be refused"
        );
    }
}

/// The census, counted by class — pinned, so a reclassification is a reviewed
/// change rather than a silent one.
#[test]
fn census_counts_per_class_are_pinned() {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for row in census() {
        let class = match row.class {
            TermClass::ConstraintParameter { .. } => "constraint-parameter",
            TermClass::NonValidating => "non-validating",
            TermClass::Structural(Role::Vocabulary) => "structural/vocabulary",
            TermClass::Structural(Role::Report) => "structural/report",
            TermClass::Structural(Role::Declaration | Role::ParameterDeclaration) => {
                "structural/declaration"
            }
            TermClass::Structural(Role::NodeExpression) => "structural/node-expression",
            TermClass::Structural(Role::Builtin) => "structural/builtin",
            TermClass::Structural(
                Role::ShapeCharacteristic | Role::Path | Role::Prefixes | Role::Graph,
            ) => "structural/shape-path-prefix-graph",
            TermClass::Target => "target",
            TermClass::Rule => "rule",
            TermClass::Unimplemented(_) => "unimplemented",
        };
        *counts.entry(class).or_insert(0) += 1;
    }
    println!("census counts: {counts:?} (total {})", census().len());
    let expected: BTreeMap<&str, usize> = EXPECTED_COUNTS.into_iter().collect();
    assert_eq!(counts, expected);
    assert_eq!(census().len(), expected.values().sum::<usize>());
}

/// The pinned per-class counts; see [`census_counts_per_class_are_pinned`].
///
/// `sh:singleLine`, `sh:rootClass` and `sh:someValue` are constraint parameters
/// of components the engine evaluates, so they count there and not among the
/// unimplemented terms: 42 + 3 and 48 − 3. `sh:subsetOf` followed when its
/// component became evaluated: 45 + 1 and 45 − 1.
const EXPECTED_COUNTS: [(&str, usize); 11] = [
    ("constraint-parameter", 46),
    ("non-validating", 10),
    ("rule", 9),
    ("structural/builtin", 68),
    ("structural/declaration", 32),
    ("structural/node-expression", 47),
    ("structural/report", 20),
    ("structural/shape-path-prefix-graph", 16),
    ("structural/vocabulary", 19),
    ("target", 7),
    ("unimplemented", 44),
];

/// Where a term may appear is part of its class: a constraint parameter and a
/// target belong on shapes, node-expression vocabulary on node expressions, a
/// rule's `sh:subject` on neither.
#[test]
fn sites_follow_the_class() {
    let min_count = classify("http://www.w3.org/ns/shacl#minCount").expect("classified");
    assert!(min_count.on_shape() && !min_count.on_node_expression());
    let target = classify("http://www.w3.org/ns/shacl#targetClass").expect("classified");
    assert!(target.on_shape());
    let count = classify("http://www.w3.org/ns/shacl-node-expr#count").expect("classified");
    assert!(count.on_node_expression() && !count.on_shape());
    let subject = classify("http://www.w3.org/ns/shacl#subject").expect("classified");
    assert!(!subject.on_shape() && !subject.on_node_expression());
    let message = classify("http://www.w3.org/ns/shacl#message").expect("classified");
    assert!(message.on_shape() && message.on_node_expression());
    let optional = classify("http://www.w3.org/ns/shacl#optional").expect("classified");
    assert!(!optional.on_shape() && optional.on_parameter_declaration());
    let default_value = classify("http://www.w3.org/ns/shacl#defaultValue").expect("classified");
    assert!(!default_value.on_shape() && default_value.on_parameter_declaration());
}
