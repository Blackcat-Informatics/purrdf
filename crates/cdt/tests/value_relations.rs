// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The SEP-0009 equality and ordering relations, and the syntactic total order.

use core::cmp::Ordering;

use pretty_assertions::assert_eq;
use purrdf_cdt::{
    CdtEntry, CdtKey, CdtLiteral, CdtTerm, CdtTypeErrorKind, CdtValue, list_equal, list_less_than,
    map_equal, map_less_than, parse_list, parse_map, term_equal, term_less_than, total_term_cmp,
    total_value_cmp,
};

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

fn typed(lexical: &str, local: &str) -> CdtTerm {
    CdtTerm::Literal(CdtLiteral::typed(lexical, format!("{XSD}{local}")))
}

fn items(lexical: &str) -> Vec<CdtTerm> {
    parse_list(lexical)
        .expect("the lexical form is well formed")
        .into_list()
        .expect("parse_list yields a list")
}

fn entries(lexical: &str) -> Vec<CdtEntry> {
    parse_map(lexical)
        .expect("the lexical form is well formed")
        .into_map()
        .expect("parse_map yields a map")
}

/// A composite element, refused by the constructor only when it would break one of
/// the crate's three bounds — which no fixture in this file does.
fn composite(value: CdtValue) -> CdtTerm {
    CdtTerm::composite(value).expect("the fixture is within every bound")
}

/// A triple-term element, under the same standing as [`composite`].
fn triple(subject: CdtTerm, predicate: CdtTerm, object: CdtTerm) -> CdtTerm {
    CdtTerm::triple(subject, predicate, object).expect("the fixture is within every bound")
}

// ── list-equal ────────────────────────────────────────────────────────────────

#[test]
fn list_equal_treats_nulls_as_mutually_indistinguishable() {
    assert_eq!(list_equal(&items("[null]"), &items("[null]")), Ok(true));
    assert_eq!(
        list_equal(&items("[1,null,2]"), &items("[1,null,2]")),
        Ok(true)
    );
    // A null is distinguishable from everything that is not a null.
    assert_eq!(list_equal(&items("[null]"), &items("[1]")), Ok(false));
}

#[test]
fn list_equal_compares_by_value_not_by_lexical_form() {
    // One value, two datatypes, two lexical forms.
    assert_eq!(list_equal(&items("[1]"), &items("[1.0]")), Ok(true));
    assert_eq!(
        list_equal(
            &items("[1]"),
            &items("[\"01\"^^<http://www.w3.org/2001/XMLSchema#integer>]")
        ),
        Ok(true)
    );
    assert_eq!(list_equal(&items("[1]"), &items("[2]")), Ok(false));
}

#[test]
fn list_equal_is_false_on_a_length_mismatch_and_never_an_error() {
    // The right-hand element would raise if it were ever compared; the length check
    // reaches its verdict first, so the answer is a clean `false`.
    let unknown = items("[\"x\"^^<http://example.org/unmodelled>]");
    assert_eq!(list_equal(&items("[]"), &unknown), Ok(false));
    assert_eq!(list_equal(&unknown, &items("[1,2]")), Ok(false));
}

#[test]
fn list_equal_propagates_a_type_error_but_a_definite_false_dominates_it() {
    let raising = items("[\"x\"^^<http://example.org/unmodelled>]");
    let other = items("[\"y\"^^<http://example.org/unmodelled>]");
    // Nothing is definitely unequal, so the type error is the answer.
    assert!(list_equal(&raising, &other).is_err());
    // Here position 0 is definitely unequal, so `false` dominates the raise at
    // position 1.
    let left = items("[1,\"x\"^^<http://example.org/unmodelled>]");
    let right = items("[2,\"y\"^^<http://example.org/unmodelled>]");
    assert_eq!(list_equal(&left, &right), Ok(false));
}

#[test]
fn list_equal_walks_nested_composites() {
    assert_eq!(
        list_equal(&items("[[1,[2]]]"), &items("[[1.0,[2.0]]]")),
        Ok(true)
    );
    assert_eq!(
        list_equal(&items("[[1,[2]]]"), &items("[[1,[3]]]")),
        Ok(false)
    );
    // A list is never equal to a map.
    assert_eq!(
        list_equal(&items("[[1]]"), &items("[{1: 1}]")),
        Ok(false),
        "a nested list is never equal to a nested map"
    );
}

// ── map-equal ─────────────────────────────────────────────────────────────────

#[test]
fn map_equal_requires_identical_key_sets() {
    assert_eq!(
        map_equal(&entries("{\"a\":1,\"b\":2}"), &entries("{\"b\":2,\"a\":1}")),
        Ok(true)
    );
    // Same values, different keys.
    assert_eq!(
        map_equal(&entries("{\"a\":1}"), &entries("{\"b\":1}")),
        Ok(false)
    );
    // Keys are distinguished by LEXICAL form, so these key sets differ even though
    // the two keys denote one value.
    assert_eq!(
        map_equal(
            &entries("{1:\"v\"}"),
            &entries("{\"01\"^^<http://www.w3.org/2001/XMLSchema#integer>:\"v\"}")
        ),
        Ok(false)
    );
    // Values, however, are compared by value.
    assert_eq!(
        map_equal(&entries("{\"a\":1}"), &entries("{\"a\":1.0}")),
        Ok(true)
    );
}

// ── list-less-than ────────────────────────────────────────────────────────────

#[test]
fn list_less_than_stops_at_the_first_unequal_position() {
    assert_eq!(list_less_than(&items("[1,2]"), &items("[1,3]")), Ok(true));
    assert_eq!(list_less_than(&items("[1,3]"), &items("[1,2]")), Ok(false));
    // The first position decides; later positions are never consulted.
    assert_eq!(list_less_than(&items("[1,9]"), &items("[2,0]")), Ok(true));
}

#[test]
fn list_less_than_continues_past_two_nulls() {
    assert_eq!(
        list_less_than(&items("[null,1]"), &items("[null,2]")),
        Ok(true)
    );
    assert_eq!(
        list_less_than(&items("[null,2]"), &items("[null,1]")),
        Ok(false)
    );
}

/// `map-functions/map-less-than-error-01.rq` and `map-less-than-error-02.rq`.
///
/// Two IRIs are definitely unequal and SPARQL `<` has no answer for them. The corpus
/// requires the whole comparison to be **unbound**, not `false`: the first unequal
/// position decides the comparison, and if `<` has no answer there then neither does
/// the comparison.
#[test]
fn list_less_than_propagates_the_operators_error_at_the_first_unequal_position() {
    let left = items("[<http://example.org/a>]");
    let right = items("[<http://example.org/b>]");
    assert_eq!(term_equal(&left[0], &right[0]), Ok(false));
    assert!(term_less_than(&left[0], &right[0]).is_err());
    assert!(list_less_than(&left, &right).is_err());
    assert!(list_less_than(&right, &left).is_err());

    // The map form the corpus actually writes: `{1: <ex:b>} < {1: <ex:a>}`.
    let map_left = entries("{1: <http://example.org/b>}");
    let map_right = entries("{1: <http://example.org/a>}");
    assert!(map_less_than(&map_left, &map_right).is_err());
    // …and `map-less-than-error-02.rq`: an IRI value against a numeric one.
    assert!(map_less_than(&map_left, &entries("{1: 42}")).is_err());

    // `map-functions/map-less-than-null-01.rq`: a stored null against a term.
    assert!(map_less_than(&entries("{1: null}"), &entries("{1: 42}")).is_err());
    // …while `map-equals-null-02.rq` requires `=` on that same pair to be a bound
    // `false`. The two relations genuinely disagree here, and both are pinned.
    assert_eq!(
        map_equal(&entries("{1: null}"), &entries("{1: 44}")),
        Ok(false)
    );

    // Equal IRIs, by contrast, are simply equal, and the walk carries on to a clean
    // `false` — `list-functions/list-less-than-31.rq`.
    let same = items("[<http://example.org/a>]");
    assert_eq!(
        list_less_than(&same, &items("[<http://example.org/a>]")),
        Ok(false)
    );
}

/// `list-functions/list-less-than-27.rq`, `-28.rq` and `-29.rq`.
///
/// A blank node at a compared position has no order — not even against the very same
/// blank node — while the length tie-break is reached first and is unaffected
/// (`list-less-than-25.rq`, `-26.rq`).
#[test]
fn a_blank_node_at_a_compared_position_has_no_order() {
    assert!(list_less_than(&items("[_:b]"), &items("[_:b]")).is_err());
    assert!(list_less_than(&items("[_:b1]"), &items("[_:b2]")).is_err());
    // `list-less-than-null-06.rq`: a null on one side does not rescue it either.
    assert!(list_less_than(&items("[null]"), &items("[_:b]")).is_err());

    // The length tie-break never looks at the element.
    assert_eq!(list_less_than(&items("[_:b]"), &items("[]")), Ok(false));
    assert_eq!(list_less_than(&items("[]"), &items("[_:b]")), Ok(true));
}

/// `list-functions/list-equals-06.rq` … `-09.rq`.
///
/// `=` on two blank nodes is `true` for the same node and **unbound** for two distinct
/// ones — SEP-0009 treats distinct blank nodes as two unknowns rather than as two known
/// different terms.
#[test]
fn equality_of_two_distinct_blank_nodes_has_no_answer() {
    assert_eq!(list_equal(&items("[   _:b   ]"), &items("[_:b]")), Ok(true));
    assert!(list_equal(&items("[_:b1]"), &items("[_:b2]")).is_err());
    // A blank node against anything that is not a blank node is a clean `false`.
    assert_eq!(list_equal(&items("[_:b]"), &items("[1]")), Ok(false));
    assert_eq!(list_equal(&items("[_:b]"), &items("[null]")), Ok(false));
}

#[test]
fn list_less_than_propagates_an_equality_error() {
    let left = items("[\"x\"^^<http://example.org/unmodelled>]");
    let right = items("[\"y\"^^<http://example.org/unmodelled>]");
    assert!(list_less_than(&left, &right).is_err());
}

/// The tri-state `purrdf_xsd::parse_by_iri` reports is preserved all the way to the
/// comparison's diagnostic: an ill-typed operand is a **defect** and says so, while an
/// unmodelled datatype is well-formed RDF the relation simply cannot decide.
#[test]
fn an_ill_typed_operand_is_distinguishable_from_an_unmodelled_one() {
    let ill = typed("abc", "integer");
    let one = typed("1", "integer");
    let unmodelled = CdtTerm::Literal(CdtLiteral::typed("x", "http://example.org/unmodelled"));

    let ill_error = term_equal(&ill, &one).expect_err("an ill-typed literal denotes nothing");
    assert_eq!(ill_error.kind, CdtTypeErrorKind::IllTyped);
    assert!(ill_error.is_ill_typed());

    let unmodelled_error =
        term_equal(&unmodelled, &one).expect_err("an unmodelled datatype cannot be decided");
    assert_eq!(unmodelled_error.kind, CdtTypeErrorKind::Unmodelled);
    assert!(!unmodelled_error.is_ill_typed());

    // Two IRIs denote perfectly well; `<` simply has nothing to say about them.
    let iri = CdtTerm::Iri("http://example.org/a".into());
    let other = CdtTerm::Iri("http://example.org/b".into());
    assert_eq!(
        term_less_than(&iri, &other)
            .expect_err("IRIs have no order")
            .kind,
        CdtTypeErrorKind::Undefined
    );

    // `<` carries the same distinction.
    assert_eq!(
        term_less_than(&ill, &one).expect_err("ill-typed").kind,
        CdtTypeErrorKind::IllTyped
    );
    assert_eq!(
        term_less_than(&unmodelled, &one)
            .expect_err("unmodelled")
            .kind,
        CdtTypeErrorKind::Unmodelled
    );

    // An ill-typed `cdt:List` literal is ill-typed for the same reason, and is NOT
    // merely "not a composite" — `list-functions/list-less-than-error-03.rq`.
    let bad_list = CdtTerm::Literal(CdtLiteral::typed(
        "1",
        "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List",
    ));
    let good_list = &items("[[2]]")[0];
    assert_eq!(
        term_equal(&bad_list, good_list)
            .expect_err("ill-typed")
            .kind,
        CdtTypeErrorKind::IllTyped
    );
    // A well-formed composite against an XSD value is a clean `false`, though: the two
    // value spaces are disjoint and PurRDF models both — `contains-07.rq`.
    assert_eq!(term_equal(good_list, &one), Ok(false));
    // Against an unmodelled datatype it is not, because PurRDF cannot know.
    assert!(term_equal(good_list, &unmodelled).is_err());
}

#[test]
fn a_shorter_list_is_smaller_than_a_list_it_prefixes() {
    assert_eq!(list_less_than(&items("[1]"), &items("[1,2]")), Ok(true));
    assert_eq!(list_less_than(&items("[1,2]"), &items("[1]")), Ok(false));
    assert_eq!(list_less_than(&items("[]"), &items("[1]")), Ok(true));
    // Equal lists are not less than each other.
    assert_eq!(list_less_than(&items("[1]"), &items("[1]")), Ok(false));
}

#[test]
fn list_less_than_descends_into_nested_lists() {
    assert_eq!(
        list_less_than(&items("[[1,2]]"), &items("[[1,3]]")),
        Ok(true)
    );
    assert_eq!(
        list_less_than(&items("[[1,3]]"), &items("[[1,2]]")),
        Ok(false)
    );
    assert_eq!(list_less_than(&items("[[1]]"), &items("[[1,0]]")), Ok(true));
}

// ── map-less-than ─────────────────────────────────────────────────────────────

#[test]
fn map_less_than_walks_in_key_order() {
    // Same key set: the values at the first differing key decide.
    assert_eq!(
        map_less_than(&entries("{\"a\":1,\"b\":2}"), &entries("{\"b\":2,\"a\":9}")),
        Ok(true)
    );
    // Authoring order is irrelevant on both sides.
    assert_eq!(
        map_less_than(&entries("{\"b\":2,\"a\":1}"), &entries("{\"a\":9,\"b\":2}")),
        Ok(true)
    );
    // Different key sets: the smaller first key wins, in the SYNTACTIC key order.
    assert_eq!(
        map_less_than(&entries("{1:\"x\"}"), &entries("{2:\"x\"}")),
        Ok(true)
    );
    assert_eq!(
        map_less_than(&entries("{2:\"x\"}"), &entries("{1:\"x\"}")),
        Ok(false)
    );
    // A shorter map is smaller than a map it prefixes in key order.
    assert_eq!(
        map_less_than(&entries("{1:\"x\"}"), &entries("{1:\"x\",2:\"y\"}")),
        Ok(true)
    );
}

/// The map `<` walk compares keys **syntactically**, and the corpus is what forces it.
///
/// `map-functions/map-less-than-15.rq` is the load-bearing case: the two keys are the
/// same integer *value* written two ways, so a value comparison could only ever have
/// answered `false`, and the corpus requires `true`.
#[test]
fn map_less_than_compares_keys_in_the_syntactic_order() {
    const INT: &str = "<http://www.w3.org/2001/XMLSchema#integer>";
    const STR: &str = "<http://www.w3.org/2001/XMLSchema#string>";

    // map-less-than-15: two spellings of the integer 1, and `'001'` sorts first.
    assert_eq!(
        map_less_than(
            &entries(&format!("{{'001'^^{INT}: 41}}")),
            &entries(&format!("{{'01'^^{INT}: 41}}"))
        ),
        Ok(true)
    );
    // map-less-than-16: the same two spellings as VALUES compare by value, so the
    // maps are equal and `<` is `false`. The two components use different orders.
    assert_eq!(
        map_less_than(
            &entries(&format!("{{1: '001'^^{INT}}}")),
            &entries(&format!("{{1: '01'^^{INT}}}"))
        ),
        Ok(false)
    );
    // map-less-than-17: the datatype IRI is the first key component, and
    // `xsd:integer` sorts before `xsd:string`.
    assert_eq!(
        map_less_than(
            &entries(&format!("{{'2'^^{INT}: 41}}")),
            &entries(&format!("{{'1'^^{STR}: 41}}"))
        ),
        Ok(true)
    );
    // map-less-than-18 and -19: the lexical form is compared BEFORE the language tag.
    assert_eq!(
        map_less_than(&entries("{'1'@sv: 41}"), &entries("{'2'@en: 41}")),
        Ok(true)
    );
    assert_eq!(
        map_less_than(&entries("{'1'@sv: 41}"), &entries("{'1'@en: 41}")),
        Ok(false)
    );
    // map-less-than-20 and -21: an IRI key sorts before any literal key, and two IRI
    // keys compare by their IRI text.
    assert_eq!(
        map_less_than(&entries("{1: 41}"), &entries("{<http://example.org>: 41}")),
        Ok(false)
    );
    assert_eq!(
        map_less_than(
            &entries("{<http://example.org/b>: 41}"),
            &entries("{<http://example.org/a>: 41}")
        ),
        Ok(false)
    );
}

// ── The syntactic total order ─────────────────────────────────────────────────

#[test]
fn the_total_order_is_defined_where_the_operator_raises() {
    let iri_a = CdtTerm::Iri("http://example.org/a".into());
    let iri_b = CdtTerm::Iri("http://example.org/b".into());
    assert!(term_less_than(&iri_a, &iri_b).is_err());
    assert_eq!(total_term_cmp(&iri_a, &iri_b), Ordering::Less);

    // NaN is incomparable to every number, and still totally ordered here.
    let nan = typed("NaN", "double");
    let one = typed("1", "double");
    assert!(term_less_than(&nan, &one).is_err());
    assert_ne!(total_term_cmp(&nan, &one), Ordering::Equal);

    // Category ranks: null < blank node < IRI < literal < triple term < composite.
    let triple = triple(iri_a.clone(), iri_b, one.clone());
    let ranked = [
        CdtTerm::Null,
        CdtTerm::Blank("b".into()),
        iri_a,
        one,
        triple,
        composite(CdtValue::empty_list()),
    ];
    for window in ranked.windows(2) {
        assert_eq!(
            total_term_cmp(&window[0], &window[1]),
            Ordering::Less,
            "{:?} should sort before {:?}",
            window[0],
            window[1]
        );
    }
    // …and a list sorts before a map.
    assert_eq!(
        total_value_cmp(&CdtValue::empty_list(), &CdtValue::empty_map()),
        Ordering::Less
    );
}

/// The counterexample the crate docs cite, run as code.
///
/// The tempting comparator — SPARQL `<` first, syntactic tie-break when it raises —
/// is **not transitive**, so it is not a total order and is unsafe to sort with.
/// This test builds it and exhibits the cycle, then shows the order the crate
/// actually exports has no cycle on the same three elements.
#[test]
fn value_order_with_a_syntactic_tiebreak_is_intransitive_but_the_exported_order_is_not() {
    fn naive(a: &CdtTerm, b: &CdtTerm) -> Ordering {
        if term_equal(a, b) == Ok(true) {
            return Ordering::Equal;
        }
        match term_less_than(a, b) {
            Ok(true) => Ordering::Less,
            Ok(false) => Ordering::Greater,
            // No value order for this pair — fall back to the syntactic one.
            Err(_) => total_term_cmp(a, b),
        }
    }

    let a = typed("9", "double");
    let b = typed("P1D", "duration");
    let c = typed("8", "float");

    // The cycle: a < b < c < a.
    assert_eq!(naive(&a, &b), Ordering::Less, "double vs duration");
    assert_eq!(naive(&b, &c), Ordering::Less, "duration vs float");
    assert_eq!(
        naive(&c, &a),
        Ordering::Less,
        "float 8 vs double 9 by value"
    );

    // The exported order is transitive on the very same three elements.
    assert_eq!(total_term_cmp(&a, &b), Ordering::Less);
    assert_eq!(total_term_cmp(&b, &c), Ordering::Less);
    assert_eq!(total_term_cmp(&a, &c), Ordering::Less);
}

#[test]
fn the_total_order_sorts_and_is_strict_on_distinct_keys() {
    let key = |lexical: &str, local: &str| {
        CdtKey::Literal(CdtLiteral::typed(lexical, format!("{XSD}{local}")))
    };
    // Distinct lexical forms of one value are distinct keys, so the order separates
    // them rather than calling them equal.
    let one = key("1", "integer");
    let oh_one = key("01", "integer");
    assert_ne!(
        purrdf_cdt::total_key_cmp(&one, &oh_one),
        Ordering::Equal,
        "distinct key terms must never compare Equal, or a map could hold both"
    );

    // Sorting a heterogeneous vector terminates with a deterministic arrangement.
    let mut terms = [
        composite(CdtValue::empty_map()),
        typed("NaN", "double"),
        CdtTerm::Null,
        CdtTerm::Iri("http://example.org/z".into()),
        CdtTerm::Blank("b".into()),
        typed("P1D", "duration"),
        typed("8", "float"),
        typed("9", "double"),
    ];
    terms.sort_by(total_term_cmp);

    let sorted_once: Vec<String> = terms
        .iter()
        .map(|term| {
            CdtValue::list(vec![term.clone()])
                .expect("a one-element list is within every bound")
                .canonical_lexical()
        })
        .collect();
    terms.reverse();
    terms.sort_by(total_term_cmp);
    let sorted_twice: Vec<String> = terms
        .iter()
        .map(|term| {
            CdtValue::list(vec![term.clone()])
                .expect("a one-element list is within every bound")
                .canonical_lexical()
        })
        .collect();
    assert_eq!(sorted_once, sorted_twice);
    assert_eq!(sorted_once[0], "[null]");
}
