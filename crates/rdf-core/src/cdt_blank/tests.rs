// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The SEP-0009 blank-node scoping rule, pinned case by case against the
//! upstream `bnodes` conformance group. Each test names the corpus entry whose
//! assertion it reproduces at the lexical layer.

use super::{
    BlankBinding, CdtBlankError, bind_cdt_blank_labels, cdt_embedded_blanks, is_cdt_datatype,
    rewrite_cdt_blank_terms,
};
use crate::blank_label::{LabelAlphabet, encode_blank_label};
use crate::ir::term::BlankScope;

use pretty_assertions::assert_eq;

const LIST: &str = purrdf_cdt::CDT_LIST;
const MAP: &str = purrdf_cdt::CDT_MAP;

/// The text-syntax binding: what every Turtle-family document uses.
const TEXT: BlankBinding = BlankBinding::Decoded(LabelAlphabet::BlankNodeLabel);

fn labels(lexical: &str, datatype: &str) -> Vec<String> {
    cdt_embedded_blanks(lexical, datatype)
        .into_iter()
        .map(|(label, _)| label)
        .collect()
}

fn bound(lexical: &str, datatype: &str, binding: BlankBinding) -> String {
    bind_cdt_blank_labels(lexical, datatype, binding)
        .expect("the fixture is a well-formed composite literal")
        .into_owned()
}

#[test]
fn only_the_two_composite_datatypes_dispatch() {
    assert!(is_cdt_datatype(LIST));
    assert!(is_cdt_datatype(MAP));
    assert!(!is_cdt_datatype("http://www.w3.org/2001/XMLSchema#string"));
    // A prefix of a composite IRI is not a composite IRI.
    assert!(!is_cdt_datatype(
        "http://w3id.org/awslabs/neptune/SPARQL-CDTs/"
    ));
    assert!(!is_cdt_datatype(
        "http://w3id.org/awslabs/neptune/SPARQL-CDTs/Lists"
    ));
}

// ── Which labels a lexical form names ───────────────────────────────────────

/// `bnodes-turtle-01`: the same label twice in one `cdt:List` literal names one
/// node; `bnodes-turtle-02` is the `cdt:Map` twin.
#[test]
fn one_label_twice_in_one_literal_names_one_node() {
    assert_eq!(labels("[_:b, 42, _:b]", LIST), ["b", "b"]);
    assert_eq!(labels("{ '1': _:b, '2': 42, '3': _:b }", MAP), ["b", "b"]);
}

/// `bnodes-turtle-03` / `-04`: distinct labels stay distinct.
#[test]
fn distinct_labels_stay_distinct() {
    assert_eq!(labels("[_:b1, 42, _:b2]", LIST), ["b1", "b2"]);
    assert_eq!(
        labels("{ '1': _:b1, '2': 42, '3': _:b2 }", MAP),
        ["b1", "b2"]
    );
}

/// `bnodes-turtle-21` / `-22`: a directly nested composite opens no new scope.
#[test]
fn direct_nesting_opens_no_new_scope() {
    assert_eq!(labels("[_:b, 42, [_:b] ]", LIST), ["b", "b"]);
    assert_eq!(
        labels("{ '1': _:b, '2': 42, '3': {'4': _:b} }", MAP),
        ["b", "b"]
    );
}

/// `bnodes-turtle-41` / `-42`: nesting through an EMBEDDED composite-typed
/// literal opens no new scope either — the corpus asserts the identical
/// `?e1 = ?e3` verdict for both spellings.
#[test]
fn embedded_composite_literal_nesting_opens_no_new_scope() {
    let list = "[_:b, 42, '[_:b]'^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/List> ]";
    assert_eq!(labels(list, LIST), ["b", "b"]);

    let map = "{ '1': _:b, '2': 42, '3': \"{'4': _:b}\"\
                ^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/Map> }";
    assert_eq!(labels(map, MAP), ["b", "b"]);
}

/// `bnodes-turtle-45`: the label inside a doubly-embedded literal is still the
/// document's node, so it must be found at that depth too.
#[test]
fn doubly_embedded_labels_are_found() {
    let lexical = "[ '[_:b]'^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/List>, 42]";
    assert_eq!(labels(lexical, LIST), ["b"]);
}

/// A non-composite embedded literal is opaque text: `"_:b"^^xsd:string` inside a
/// list names no blank node, because its datatype is not composite.
#[test]
fn a_non_composite_embedded_literal_stays_opaque() {
    let lexical = "[ '_:b'^^<http://www.w3.org/2001/XMLSchema#string>, 42]";
    assert!(labels(lexical, LIST).is_empty());
    // The same bytes typed as a plain string literal are opaque as well.
    assert!(labels("[ '_:b', 42]", LIST).is_empty());
}

/// A blank node nested inside an RDF 1.2 triple term element is in scope.
#[test]
fn triple_term_components_are_in_scope() {
    let lexical = "[ <<( _:b <http://example.org/p> _:c )>> ]";
    let mut found = labels(lexical, LIST);
    found.sort();
    assert_eq!(found, ["b", "c"]);
}

/// A `cdt:Map` key can never be a blank node, so a `_:`-looking key is text.
#[test]
fn a_map_key_never_contributes_a_blank_node() {
    assert_eq!(labels("{ '_:b': _:c }", MAP), ["c"]);
}

// ── Binding ─────────────────────────────────────────────────────────────────

/// A plain label read from a text syntax at the default scope already spells its
/// own `(label, scope)` pair, so binding is the identity — and returns the input
/// BORROWED, with not one byte copied.
#[test]
fn text_ingress_at_the_default_scope_is_a_byte_identity() {
    let lexical = "[_:b, 42, _:b]";
    let out = bind_cdt_blank_labels(lexical, LIST, TEXT).expect("well formed");
    assert!(matches!(out, std::borrow::Cow::Borrowed(_)));
    assert_eq!(out, lexical);
}

/// Binding is idempotent under a text binding: re-binding an already-bound form
/// reproduces it byte for byte.
#[test]
fn text_binding_is_a_fixpoint() {
    for lexical in [
        "[_:b, 42, _:b]",
        "{ '1': _:b, '2': 42 }",
        "[_:b, 42, '[_:b]'^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/List> ]",
    ] {
        let datatype = datatype_of(lexical);
        let once = bound(lexical, datatype, TEXT);
        let twice = bound(&once, datatype, TEXT);
        assert_eq!(once, lexical, "binding changed a plain text form");
        assert_eq!(twice, once, "binding is not a fixpoint");
    }
}

/// Pick the datatype a fixture's opening delimiter implies.
fn datatype_of(lexical: &str) -> &'static str {
    if lexical.trim_start().starts_with('{') {
        MAP
    } else {
        LIST
    }
}

/// A carrier that assigns one ambient scope to a source binds the embedded
/// labels into that scope with EXACTLY the spelling
/// [`encode_blank_label`] gives the same source's bare terms — which is what
/// makes an embedded occurrence and a term occurrence the same node.
#[test]
fn an_ambient_scope_is_spelled_exactly_as_a_bare_term_would_be() {
    let scope = BlankScope(7);
    let binding = BlankBinding::Ambient(scope);
    let out = bound("[_:b, 42, _:b]", LIST, binding);

    let expected_token = encode_blank_label("b", scope, LabelAlphabet::BlankNodeLabel);
    assert_eq!(out, format!("[_:{expected_token}, 42, _:{expected_token}]"));

    // And reading it back yields the very `(label, scope)` pair the bare term
    // was interned under.
    assert_eq!(
        cdt_embedded_blanks(&out, LIST),
        vec![("b".to_owned(), scope), ("b".to_owned(), scope)]
    );
}

/// The same ambient binding reaches labels at every nesting depth, including
/// inside an embedded composite-typed literal — and rewrites them by splicing,
/// so the embedded literal's quote style and escapes survive untouched.
#[test]
fn an_ambient_scope_reaches_embedded_literals_without_re_escaping() {
    let lexical =
        "[_:b, \"a\\u0041b\", '[_:b]'^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/List> ]";
    let out = bound(lexical, LIST, BlankBinding::Ambient(BlankScope(3)));
    assert_eq!(
        out,
        "[_:purrdfesc3_b, \"a\\u0041b\", '[_:purrdfesc3_b]'\
         ^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/List> ]"
    );
}

/// Only the label tokens move. Whitespace, map-entry order, numeric spellings,
/// quote style, escape spellings and the datatype IRI are all preserved byte for
/// byte — which is what makes this a scope binding rather than a
/// canonicalization.
#[test]
fn binding_touches_the_label_tokens_and_nothing_else() {
    let lexical = "{   '2':  +0.70e1 ,  '1' :_:b ,  '3': \"x\\ty\"@EN  }";
    let out = bound(lexical, MAP, BlankBinding::Ambient(BlankScope(1)));
    assert_eq!(
        out,
        "{   '2':  +0.70e1 ,  '1' :_:purrdfesc1_b ,  '3': \"x\\ty\"@EN  }"
    );
    // The canonical form would have reordered and renormalized all of that.
    let canonical = purrdf_cdt::canonical_lexical(
        &purrdf_cdt::parse_cdt_by_iri(lexical, MAP)
            .expect("well formed")
            .expect("a map"),
    );
    assert_ne!(canonical, out, "binding must not canonicalize");
}

/// A non-composite datatype is never scanned at all.
#[test]
fn a_non_composite_literal_is_untouched() {
    let lexical = "[_:b, 42]";
    let xsd = "http://www.w3.org/2001/XMLSchema#string";
    let out = bind_cdt_blank_labels(lexical, xsd, BlankBinding::Ambient(BlankScope(9)))
        .expect("not composite");
    assert_eq!(out, lexical);
    assert!(cdt_embedded_blanks(lexical, xsd).is_empty());
}

// ── Refusals ────────────────────────────────────────────────────────────────

/// An ill-formed composite lexical form is a typed refusal, never a silently
/// opaque literal and never a panic.
#[test]
fn an_ill_formed_composite_literal_is_refused() {
    for lexical in ["[_:b, 42", "[1 2]", "{ '1' 2 }", "[_:]", "[_:b.]", "]["] {
        let err = bind_cdt_blank_labels(lexical, LIST, TEXT).expect_err(lexical);
        assert!(
            matches!(err, CdtBlankError::Malformed { .. }),
            "{lexical} produced {err:?}"
        );
    }
}

/// A `cdt:Map` lexical form is not a `cdt:List` lexical form.
#[test]
fn the_datatype_selects_the_grammar() {
    assert!(bind_cdt_blank_labels("[]", MAP, TEXT).is_err());
    assert!(bind_cdt_blank_labels("{}", LIST, TEXT).is_err());
}

/// An EMBEDDED composite literal is validated too: its blank nodes are in the
/// same scope, so a broken one leaves that scope undefined exactly as a broken
/// outer form does.
#[test]
fn a_malformed_embedded_composite_literal_is_refused() {
    let lexical = "[ '[_:b'^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/List>, 42]";
    let err = bind_cdt_blank_labels(lexical, LIST, TEXT).expect_err("the inner form is broken");
    assert!(matches!(err, CdtBlankError::Malformed { .. }), "{err:?}");
}

/// The `purrdf-cdt` resource limits are enforced at ingress rather than looped
/// on or truncated: over-deep nesting is a refusal.
#[test]
fn over_deep_nesting_is_refused_not_truncated() {
    let depth = purrdf_cdt::MAX_NESTING_DEPTH + 2;
    let mut lexical = String::new();
    for _ in 0..depth {
        lexical.push('[');
    }
    lexical.push_str("_:b");
    for _ in 0..depth {
        lexical.push(']');
    }
    let err = bind_cdt_blank_labels(&lexical, LIST, TEXT).expect_err("too deep");
    assert!(matches!(err, CdtBlankError::Malformed { .. }), "{err:?}");
}

/// The scanner is TOTAL: it never panics, whatever bytes it is handed. Only the
/// validating entry point refuses; the scan itself always terminates.
#[test]
fn the_scanner_never_panics_on_hostile_bytes() {
    for lexical in [
        "_:",
        "_:\u{0}",
        "[_:b",
        "\"unterminated",
        "'''_:b",
        "<unterminated",
        "[ '' ^^< ]",
        "[_:b, \"\\",
        "[\u{1f600}_:b]",
        "<<(_:b",
        "\"\"\"_:b\"\"\"^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/List>",
    ] {
        // Reached through the public surface that does not validate first.
        let _ = cdt_embedded_blanks(lexical, LIST);
        let _ = rewrite_cdt_blank_terms(lexical, LIST, &mut |_| Some("_:x".to_owned()));
    }
}

// ── Rewriting for the whole-dataset passes ──────────────────────────────────

/// A relabeling pass renames the embedded occurrence in lockstep with the term
/// occurrence, so the embedded blank never dangles.
#[test]
fn rewriting_renames_every_occurrence() {
    let lexical = "[_:b, 42, [_:b], _:c]";
    let out = rewrite_cdt_blank_terms(lexical, LIST, &mut |label| match label {
        "b" => Some("_:c14n0".to_owned()),
        "c" => Some("_:c14n1".to_owned()),
        _ => None,
    });
    assert_eq!(out, "[_:c14n0, 42, [_:c14n0], _:c14n1]");
}

/// Skolemization replaces the embedded blank with an IRI element, which the CDT
/// grammar admits in exactly that position.
#[test]
fn rewriting_can_skolemize_an_embedded_blank() {
    let out = rewrite_cdt_blank_terms("[_:b, 42]", LIST, &mut |_| {
        Some("<http://example.org/.well-known/genid/1>".to_owned())
    });
    assert_eq!(out, "[<http://example.org/.well-known/genid/1>, 42]");
    // The result is still a well-formed composite value.
    assert!(purrdf_cdt::parse_cdt_by_iri(&out, LIST).is_ok());
}

/// A rewrite that changes nothing returns the input borrowed.
#[test]
fn a_no_op_rewrite_borrows() {
    let out = rewrite_cdt_blank_terms("[_:b, 42]", LIST, &mut |_| None);
    assert!(matches!(out, std::borrow::Cow::Borrowed(_)));
}
