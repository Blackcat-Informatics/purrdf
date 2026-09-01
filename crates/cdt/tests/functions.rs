// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The SEP-0009 function library, checked against SEP-0009's own conformance corpus.
//!
//! Every test below names the corpus file it reproduces, in
//! `vectors/sparql-cdt/list-functions` or `vectors/sparql-cdt/map-functions`. The
//! corpus is a set of `ASK` queries, so a case that expects a SPARQL expression error
//! is written there as `FILTER(!BOUND(?x))`; here that is
//! [`CdtOutcome::Error`](purrdf_cdt::CdtOutcome::Error), and a case that expects a
//! value asserts the value rather than merely that something was produced.

use pretty_assertions::assert_eq;
use purrdf_cdt::{
    CDT_FUNCTIONS, CDT_LIST, CDT_MAP, CDT_NS, CdtArity, CdtEntry, CdtError, CdtFn, CdtKey,
    CdtLiteral, CdtOutcome, CdtTerm, CdtValue, MAX_ELEMENTS, MAX_LEXICAL_BYTES, MAX_NESTING_DEPTH,
    MapRemoval, concat, contains, contains_key, get, head, integer_argument, keys, list_concat,
    list_constructor, list_contains, list_get, list_head, list_reverse, list_size, list_subseq,
    list_tail, map_constructor, map_contains_key, map_get, map_keys, map_merge, map_put,
    map_remove, map_size, merge, parse_list, parse_map, put, remove, reverse, size, subseq, tail,
};

const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";

// ── Fixtures ──────────────────────────────────────────────────────────────────

fn list(lexical: &str) -> CdtValue {
    parse_list(lexical).expect("the list lexical form is well formed")
}

fn map(lexical: &str) -> CdtValue {
    parse_map(lexical).expect("the map lexical form is well formed")
}

fn items(lexical: &str) -> Vec<CdtTerm> {
    match list(lexical) {
        CdtValue::List(items) => items,
        CdtValue::Map(_) => unreachable!("parse_list yields a list"),
    }
}

fn entries(lexical: &str) -> Vec<CdtEntry> {
    match map(lexical) {
        CdtValue::Map(entries) => entries,
        CdtValue::List(_) => unreachable!("parse_map yields a map"),
    }
}

fn int(lexical: &str) -> CdtTerm {
    CdtTerm::Literal(CdtLiteral::typed(lexical, XSD_INTEGER))
}

fn dec(lexical: &str) -> CdtTerm {
    CdtTerm::Literal(CdtLiteral::typed(lexical, XSD_DECIMAL))
}

fn dbl(lexical: &str) -> CdtTerm {
    CdtTerm::Literal(CdtLiteral::typed(lexical, XSD_DOUBLE))
}

fn text(lexical: &str) -> CdtTerm {
    CdtTerm::Literal(CdtLiteral::plain(lexical))
}

fn lang(lexical: &str, tag: &str) -> CdtTerm {
    CdtTerm::Literal(CdtLiteral::lang(lexical, tag))
}

fn iri(iri: &str) -> CdtTerm {
    CdtTerm::Iri(iri.into())
}

/// The value of an outcome that must have one, so a failure names what happened.
fn value<T: core::fmt::Debug>(outcome: CdtOutcome<T>) -> T {
    match outcome {
        CdtOutcome::Value(value) => value,
        other => panic!("expected a value, got {other:?}"),
    }
}

// ── The registry ──────────────────────────────────────────────────────────────

#[test]
fn every_function_iri_lives_under_the_spec_namespace_and_round_trips() {
    for function in CDT_FUNCTIONS {
        assert!(
            function.iri().starts_with(CDT_NS),
            "{} is not under the SEP-0009 namespace",
            function.iri()
        );
        assert_eq!(
            function.iri(),
            format!("{CDT_NS}{}", function.local_name()),
            "the IRI is the namespace followed by the local name"
        );
        assert_eq!(CdtFn::from_iri(function.iri()), Some(function));
    }
    assert_eq!(CDT_FUNCTIONS.len(), 15);
}

#[test]
fn the_two_constructors_share_their_iris_with_the_two_datatypes() {
    // Intended by SEP-0009: `cdt:List` is a datatype in datatype position and the
    // list constructor in call position. `from_iri` is the call-position resolver.
    assert_eq!(CdtFn::ListConstructor.iri(), CDT_LIST);
    assert_eq!(CdtFn::MapConstructor.iri(), CDT_MAP);
    assert_eq!(CdtFn::from_iri(CDT_LIST), Some(CdtFn::ListConstructor));
    assert_eq!(CdtFn::from_iri(CDT_MAP), Some(CdtFn::MapConstructor));
}

#[test]
fn an_iri_outside_the_library_is_not_a_cdt_function() {
    assert_eq!(CdtFn::from_iri("http://example.org/get"), None);
    // Right namespace, wrong local name.
    assert_eq!(CdtFn::from_iri(&format!("{CDT_NS}fold")), None);
    // Right local name, wrong namespace.
    assert_eq!(CdtFn::from_iri("http://example.org/ns/size"), None);
}

#[test]
fn arities_match_the_call_shapes_the_corpus_uses() {
    // list-functions/list-constructor-01.rq calls `cdt:List()` with no arguments.
    assert!(CdtFn::ListConstructor.arity().admits(0));
    assert!(CdtFn::ListConstructor.arity().admits(7));
    // list-functions/concat-08.rq, concat-09.rq and concat-10.rq: zero, one, three.
    assert!(CdtFn::Concat.arity().admits(0));
    assert!(CdtFn::Concat.arity().admits(1));
    assert!(CdtFn::Concat.arity().admits(3));
    // list-functions/subseq-03.rq omits the length; subseq-02.rq supplies it.
    assert_eq!(CdtFn::Subseq.arity(), CdtArity::Range { min: 2, max: 3 });
    assert!(!CdtFn::Subseq.arity().admits(1));
    assert!(!CdtFn::Subseq.arity().admits(4));
    // map-functions/put-03.rq omits the value.
    assert_eq!(CdtFn::Put.arity(), CdtArity::Range { min: 2, max: 3 });
    // map-functions/map-constructor-02.rq passes two key/value pairs.
    assert!(CdtFn::MapConstructor.arity().admits(0));
    assert!(CdtFn::MapConstructor.arity().admits(4));
    assert!(!CdtFn::MapConstructor.arity().admits(5));
    // map-functions/merge-01.rq passes two maps.
    assert!(CdtFn::Merge.arity().admits(2));
    assert!(!CdtFn::Merge.arity().admits(1));
    for one_argument in [
        CdtFn::Head,
        CdtFn::Tail,
        CdtFn::Reverse,
        CdtFn::Size,
        CdtFn::Keys,
    ] {
        assert_eq!(one_argument.arity(), CdtArity::Fixed(1));
    }
    for two_arguments in [
        CdtFn::Contains,
        CdtFn::ContainsKey,
        CdtFn::Get,
        CdtFn::Remove,
    ] {
        assert_eq!(two_arguments.arity(), CdtArity::Fixed(2));
    }
}

// ── Argument coercion ─────────────────────────────────────────────────────────

#[test]
fn only_an_integer_is_an_index() {
    // list-functions/get-error-05.rq: `cdt:get(?list, "invalid")` is unbound.
    assert_eq!(integer_argument(&text("invalid")), None);
    // list-functions/get-error-06.rq: `cdt:get(?list, 2.0)` is unbound, even though
    // 2.0 denotes an integer — it is an `xsd:decimal`, a different value space.
    assert_eq!(integer_argument(&dec("2.0")), None);
    assert_eq!(integer_argument(&dbl("2e0")), None);
    assert_eq!(integer_argument(&int("2")), Some(2));
    assert_eq!(integer_argument(&int("-1")), Some(-1));
    // The derived integer datatypes are integers too.
    let long = CdtTerm::Literal(CdtLiteral::typed(
        "3",
        "http://www.w3.org/2001/XMLSchema#long",
    ));
    assert_eq!(integer_argument(&long), Some(3));
    // Nothing else denotes an index at all.
    assert_eq!(integer_argument(&CdtTerm::Null), None);
    assert_eq!(integer_argument(&iri("http://example.org/1")), None);
    assert_eq!(integer_argument(&lang("1", "en")), None);
}

// ── cdt:size ──────────────────────────────────────────────────────────────────

#[test]
fn list_size_counts_every_element_including_nulls() {
    // list-functions/size-01.rq … size-12.rq.
    assert_eq!(list_size(&items("[]")), 0);
    assert_eq!(list_size(&items("[1]")), 1);
    assert_eq!(list_size(&items("[1, 2]")), 2);
    assert_eq!(list_size(&items("['a', 1]")), 2);
    // size-07.rq / size-08.rq: a null is an element.
    assert_eq!(list_size(&items("[null]")), 1);
    // size-12.rq.
    assert_eq!(list_size(&items("[null, 2]")), 2);
    // size-09.rq … size-11.rq: nesting does not flatten.
    assert_eq!(list_size(&items("[[]]")), 1);
    assert_eq!(list_size(&items("[[1]]")), 1);
    assert_eq!(list_size(&items("[[1], 2]")), 2);
}

#[test]
fn map_size_counts_every_entry_including_null_values() {
    // map-functions/size-01.rq … size-05.rq.
    assert_eq!(map_size(&entries("{}")), 0);
    assert_eq!(map_size(&entries("{1: 'one'}")), 1);
    assert_eq!(map_size(&entries("{1: 'one', 2: 'two'}")), 2);
    assert_eq!(map_size(&entries("{1: 'one', 'hello'@en: 2.5}")), 2);
    // size-05.rq: an entry whose value is null still counts.
    assert_eq!(map_size(&entries("{1: 'one', 2: null}")), 2);
}

#[test]
fn size_is_one_function_over_both_composite_datatypes() {
    // `cdt:size` is overloaded on the runtime datatype of its argument and neither
    // datatype is an error — that is why there is one `cdt:size` IRI, not two.
    assert_eq!(size(&list("[1, 2]")), 2);
    assert_eq!(size(&map("{1: 'one'}")), 1);
    // list-functions/size-error-01.rq binds the plain string "[1,2]" and requires
    // `cdt:size` on it to be unbound. A plain string is not a composite lexical
    // form, so the failure happens before `size` is reachable at all.
    assert!(parse_list("[1,2]  trailing").is_err());
}

// ── cdt:get on a list ─────────────────────────────────────────────────────────

#[test]
fn list_get_is_one_based() {
    // list-functions/get-01.rq … get-05.rq.
    assert_eq!(value(list_get(&items("[1]"), &int("1"))), int("1"));
    let two = items("[1, 3]");
    assert_eq!(value(list_get(&two, &int("1"))), int("1"));
    assert_eq!(value(list_get(&two, &int("2"))), int("3"));
    let mixed = items("['a', 1]");
    assert_eq!(value(list_get(&mixed, &int("1"))), text("a"));
    assert_eq!(value(list_get(&mixed, &int("2"))), int("1"));
}

#[test]
fn list_get_returns_a_nested_composite_and_a_blank_node_as_they_are() {
    // list-functions/get-06.rq … get-08.rq: the element of `[[1]]` is the list `[1]`.
    let outer = items("[[1], 3]");
    let nested = value(list_get(&outer, &int("1")));
    assert_eq!(nested, CdtTerm::composite(list("[1]")));
    let CdtTerm::Composite(inner) = &nested else {
        panic!("the first element is a nested list")
    };
    let CdtValue::List(inner_items) = inner.as_ref() else {
        panic!("the nested composite is a list")
    };
    assert_eq!(value(list_get(inner_items, &int("1"))), int("1"));
    // get-06.rq: `cdt:get` on the empty nested list is unbound.
    assert!(list_get(&items("[]"), &int("1")).is_error());

    // get-10.rq … get-13.rq: blank nodes come back as blank nodes, and two reads of
    // one position give the same label while two distinct labels stay distinct.
    let blanks = items("[_:b1,_:b2]");
    assert_eq!(
        value(list_get(&blanks, &int("1"))),
        CdtTerm::Blank("b1".into())
    );
    assert_eq!(
        value(list_get(&blanks, &int("1"))),
        CdtTerm::Blank("b1".into())
    );
    assert_ne!(
        value(list_get(&blanks, &int("1"))),
        value(list_get(&blanks, &int("2")))
    );
}

#[test]
fn list_get_out_of_range_is_a_sparql_error_not_a_null() {
    let three = items("[1,2,3]");
    // list-functions/get-error-02.rq: past the end.
    assert!(list_get(&three, &int("10")).is_error());
    // get-error-03.rq: zero is out of range because the index is 1-based.
    assert!(list_get(&three, &int("0")).is_error());
    // get-error-04.rq: negative.
    assert!(list_get(&three, &int("-1")).is_error());
    // get-null-02.rq: the empty list has no position 1.
    assert!(list_get(&items("[]"), &int("1")).is_error());
    // None of these is a bound refusal — they are ordinary expression errors.
    assert!(!list_get(&three, &int("10")).is_bound());
}

#[test]
fn list_get_with_a_non_integer_index_is_an_error() {
    let three = items("[1,2,3]");
    // list-functions/get-error-05.rq.
    assert!(list_get(&three, &text("invalid")).is_error());
    // get-error-06.rq: an `xsd:decimal` is not an index.
    assert!(list_get(&three, &dec("2.0")).is_error());
}

#[test]
fn list_get_on_a_null_position_is_an_error() {
    // list-functions/get-null-01.rq: `cdt:get("[null]", 1)` is unbound. A null is a
    // position that carries no term, so there is nothing to return.
    assert!(list_get(&items("[null]"), &int("1")).is_error());
    // get-09.rq: the null at position 1 raises while position 2 still answers.
    let mixed = items("[null, 3]");
    assert!(list_get(&mixed, &int("1")).is_error());
    assert_eq!(value(list_get(&mixed, &int("2"))), int("3"));
}

// ── cdt:head and cdt:tail ─────────────────────────────────────────────────────

#[test]
fn list_head_is_the_first_element() {
    // list-functions/head-02.rq … head-06.rq, head-08.rq, head-09.rq.
    assert_eq!(value(list_head(&items("[1, 2]"))), int("1"));
    assert_eq!(value(list_head(&items("['a', 1]"))), text("a"));
    assert_eq!(
        value(list_head(&items("[[]]"))),
        CdtTerm::composite(list("[]"))
    );
    assert_eq!(
        value(list_head(&items("[[1], 2]"))),
        CdtTerm::composite(list("[1]"))
    );
    // head-11.rq / head-12.rq: a blank node is an answer, and the same one each time.
    assert_eq!(
        value(list_head(&items("[_:b]"))),
        CdtTerm::Blank("b".into())
    );
    assert_eq!(
        value(list_head(&items("[_:b]"))),
        CdtTerm::Blank("b".into())
    );
}

#[test]
fn list_head_of_an_empty_list_or_a_leading_null_is_an_error() {
    // list-functions/head-01.rq and head-null-02.rq.
    assert!(list_head(&items("[]")).is_error());
    // head-07.rq and head-null-01.rq.
    assert!(list_head(&items("[null]")).is_error());
    // head-10.rq: still an error even though a term follows the null.
    assert!(list_head(&items("[null, 2]")).is_error());
}

#[test]
fn list_tail_drops_the_first_element_even_when_it_is_a_null() {
    // list-functions/tail-02.rq … tail-09.rq.
    assert_eq!(value(list_tail(&items("[1]"))), list("[]"));
    assert_eq!(value(list_tail(&items("[1, 2]"))), list("[2]"));
    assert_eq!(value(list_tail(&items("['a', 'b']"))), list("['b']"));
    assert_eq!(value(list_tail(&items("[[1], 2]"))), list("[2]"));
    // tail-07.rq / tail-10.rq: the asymmetry with `cdt:head` — `cdt:tail` never
    // looks at the element it drops, so a leading null is fine.
    assert_eq!(value(list_tail(&items("[null]"))), list("[]"));
    assert_eq!(value(list_tail(&items("[null, 2]"))), list("[2]"));
}

#[test]
fn list_tail_of_an_empty_list_is_an_error() {
    // list-functions/tail-01.rq.
    assert!(list_tail(&items("[]")).is_error());
}

// ── cdt:reverse ───────────────────────────────────────────────────────────────

#[test]
fn list_reverse_reverses_and_never_raises() {
    // list-functions/reverse-01.rq … reverse-09.rq.
    assert_eq!(list_reverse(&items("[]")), list("[]"));
    assert_eq!(list_reverse(&items("[1]")), list("[1]"));
    assert_eq!(list_reverse(&items("[1, 2]")), list("[2,1]"));
    assert_eq!(list_reverse(&items("['a', 1]")), list("[1,'a']"));
    assert_eq!(list_reverse(&items("[[1], 2]")), list("[2,[1]]"));
}

#[test]
fn list_reverse_carries_nulls_through() {
    // list-functions/reverse-07.rq compares the reverse of `[null]` against
    // `cdt:List(?unbound)` with SAMETERM, and reverse-10.rq does the same for
    // `[null, 2]` against `cdt:List(2, ?unbound)`. SAMETERM is byte identity of the
    // lexical form, so the reversal must produce the same bytes the constructor does.
    let reversed = list_reverse(&items("[null]"));
    let constructed = value(list_constructor(vec![CdtTerm::Null]));
    assert_eq!(
        reversed.canonical_lexical(),
        constructed.canonical_lexical()
    );

    let reversed = list_reverse(&items("[null, 2]"));
    let constructed = value(list_constructor(vec![int("2"), CdtTerm::Null]));
    assert_eq!(
        reversed.canonical_lexical(),
        constructed.canonical_lexical()
    );
}

// ── cdt:subseq ────────────────────────────────────────────────────────────────

#[test]
fn list_subseq_takes_a_one_based_start_and_a_length() {
    let ten = items("[1,2,3,4,5,6,7,8,9,10]");
    // list-functions/subseq-01.rq.
    assert_eq!(
        value(list_subseq(&ten, &int("1"), Some(&int("1")))),
        list("[1]")
    );
    // subseq-02.rq: the third argument is a LENGTH. Read as an end position this
    // would be `[2, 3]`; the corpus expects three elements.
    assert_eq!(
        value(list_subseq(&ten, &int("2"), Some(&int("3")))),
        list("[2, 3, 4]")
    );
    // subseq-03.rq: omitting the length runs to the end.
    assert_eq!(
        value(list_subseq(&ten, &int("7"), None)),
        list("[7,8,9,10]")
    );
}

#[test]
fn list_subseq_admits_a_start_one_past_the_end() {
    // list-functions/subseq-04.rq and subseq-05.rq, on the empty list.
    let empty = items("[]");
    assert_eq!(
        value(list_subseq(&empty, &int("1"), Some(&int("0")))),
        list("[]")
    );
    assert_eq!(value(list_subseq(&empty, &int("1"), None)), list("[]"));
    // subseq-06.rq and subseq-07.rq: start 4 on a three-element list.
    let three = items("[1,2,3]");
    assert_eq!(
        value(list_subseq(&three, &int("4"), Some(&int("0")))),
        list("[]")
    );
    assert_eq!(value(list_subseq(&three, &int("4"), None)), list("[]"));
}

#[test]
fn list_subseq_refuses_a_range_that_leaves_the_list() {
    let three = items("[1,2,3]");
    // list-functions/subseq-08.rq and subseq-09.rq: two past the end.
    assert!(list_subseq(&three, &int("5"), None).is_error());
    assert!(list_subseq(&three, &int("5"), Some(&int("0"))).is_error());
    // subseq-10.rq: a legal start with a length that reaches past the end. The
    // corpus expects an error, not a truncation to `[]`.
    assert!(list_subseq(&three, &int("4"), Some(&int("1"))).is_error());
    // subseq-11.rq.
    let ten = items("[1,2,3,4,5,6,7,8,9,10]");
    assert!(list_subseq(&ten, &int("10"), Some(&int("2"))).is_error());
    // subseq-12.rq and subseq-13.rq: a start below 1.
    assert!(list_subseq(&ten, &int("0"), Some(&int("2"))).is_error());
    assert!(list_subseq(&ten, &int("-2"), None).is_error());
    // subseq-error-02.rq: a non-integer argument.
    assert!(list_subseq(&ten, &text("invalid"), Some(&int("3"))).is_error());
    assert!(list_subseq(&ten, &int("1"), Some(&text("invalid"))).is_error());
    // Not pinned by the corpus: a negative length is refused rather than treated as
    // an empty range or a truncation.
    assert!(list_subseq(&ten, &int("1"), Some(&int("-1"))).is_error());
}

// ── cdt:concat ────────────────────────────────────────────────────────────────

#[test]
fn list_concat_is_variadic_from_zero() {
    // list-functions/concat-08.rq.
    assert_eq!(value(list_concat(&[])), list("[]"));
    // concat-09.rq.
    let one = items("[1]");
    assert_eq!(value(list_concat(&[&one])), list("[1]"));
    // concat-10.rq: three arguments, one of them repeated.
    let two_three = items("[2,3]");
    assert_eq!(
        value(list_concat(&[&one, &two_three, &one])),
        list("[1,2,3,1]")
    );
}

#[test]
fn list_concat_joins_in_argument_order() {
    // list-functions/concat-01.rq … concat-07.rq.
    let empty = items("[]");
    let one = items("[1]");
    assert_eq!(value(list_concat(&[&empty, &empty])), list("[]"));
    assert_eq!(value(list_concat(&[&empty, &one])), list("[1]"));
    assert_eq!(value(list_concat(&[&one, &empty])), list("[1]"));
    assert_eq!(value(list_concat(&[&one, &one])), list("[1,1]"));
    let one_two = items("[1, 2]");
    assert_eq!(value(list_concat(&[&one_two, &one_two])), list("[1,2,1,2]"));
    let nested_one = items("[[1]]");
    let nested_two = items("[[2]]");
    assert_eq!(
        value(list_concat(&[&nested_one, &nested_two])),
        list("[[1],[2]]")
    );
}

#[test]
fn list_concat_copies_nulls_as_positions() {
    // list-functions/concat-null-01.rq: concatenating `[null]` with itself gives a
    // list of size 2 whose every `cdt:get` is unbound.
    let nulls = items("[null]");
    let joined = value(list_concat(&[&nulls, &nulls]));
    assert_eq!(size(&joined), 2);
    let CdtValue::List(joined_items) = &joined else {
        panic!("concat yields a list")
    };
    assert!(list_get(joined_items, &int("1")).is_error());
    assert!(list_get(joined_items, &int("2")).is_error());
}

#[test]
fn concat_raises_for_a_map_argument() {
    // list-functions/concat-error-01.rq and concat-error-02.rq require a
    // non-`cdt:List` argument to make the whole call unbound.
    assert!(concat(&[list("[1]"), map("{}")]).is_error());
    assert!(concat(&[map("{}")]).is_error());
    assert_eq!(value(concat(&[list("[1]"), list("[2]")])), list("[1,2]"));
}

// ── cdt:contains ──────────────────────────────────────────────────────────────

#[test]
fn list_contains_compares_by_value() {
    // list-functions/contains-01.rq and contains-02.rq.
    assert_eq!(value(list_contains(&items("[]"), &int("1"))), false);
    let one = items("[1]");
    assert_eq!(value(list_contains(&one, &int("1"))), true);
    assert_eq!(value(list_contains(&one, &int("2"))), false);

    // contains-03.rq: one value, many spellings, all of them found.
    let mixed = items("[1,'a','b'@en,2.0]");
    for sought in [
        int("1"),
        int("+1"),
        int("01"),
        dec("1.0"),
        dbl("1e0"),
        int("2"),
        dec("2.0"),
        dbl("2e0"),
    ] {
        assert_eq!(
            value(list_contains(&mixed, &sought)),
            true,
            "{sought:?} should be found by value"
        );
    }
    assert_eq!(value(list_contains(&mixed, &text("a"))), true);
    assert_eq!(value(list_contains(&mixed, &lang("b", "en"))), true);
    // A language-tagged string is not the plain string.
    assert_eq!(value(list_contains(&mixed, &text("b"))), false);

    // contains-04.rq: IRIs.
    let with_iri = items("[<http://example.org/test>,1]");
    assert_eq!(
        value(list_contains(&with_iri, &iri("http://example.org/test"))),
        true
    );
}

#[test]
fn list_contains_compares_blank_nodes_by_identity() {
    // list-functions/contains-05.rq: a blank node that is not in the list gives a
    // bound `false`, not an error. contains-06.rq: the very term `cdt:head` returned
    // from the list is found.
    let with_blank = items("[_:b,null,'_:b']");
    let other = CdtTerm::Blank("fresh".into());
    assert_eq!(value(list_contains(&with_blank, &other)), false);

    let two = items("[_:b,2]");
    let first = value(list_head(&two));
    assert_eq!(value(list_contains(&two, &first)), true);
}

#[test]
fn list_contains_finds_nested_composites_in_either_spelling() {
    // list-functions/contains-07.rq: a nested `[2]`.
    let nested = items("[1,[2]]");
    let sought = CdtTerm::composite(list("[2]"));
    assert_eq!(value(list_contains(&nested, &int("1"))), true);
    assert_eq!(value(list_contains(&nested, &int("2"))), false);
    assert_eq!(value(list_contains(&nested, &sought)), true);

    // contains-08.rq: the SAME element written as a `cdt:List`-typed literal must be
    // found by the same query. Two spellings, one value.
    let as_literal = items("[1,'[2]'^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/List>]");
    assert_eq!(value(list_contains(&as_literal, &int("1"))), true);
    assert_eq!(value(list_contains(&as_literal, &int("2"))), false);
    assert_eq!(value(list_contains(&as_literal, &sought)), true);

    // contains-09.rq and contains-10.rq: the same pair of spellings for a map.
    let sought_map = CdtTerm::composite(map("{2: 3}"));
    let nested_map = items("[1,{2: 3}]");
    assert_eq!(value(list_contains(&nested_map, &int("2"))), false);
    assert_eq!(value(list_contains(&nested_map, &int("3"))), false);
    assert_eq!(value(list_contains(&nested_map, &sought_map)), true);
    let map_literal = items("[1,'{2: 3}'^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/Map>]");
    assert_eq!(value(list_contains(&map_literal, &int("2"))), false);
    assert_eq!(value(list_contains(&map_literal, &int("3"))), false);
    assert_eq!(value(list_contains(&map_literal, &sought_map)), true);
}

#[test]
fn a_null_element_neither_matches_nor_poisons_the_search() {
    // list-functions/contains-null-01.rq.
    let with_null = items("[1,null,2]");
    assert_eq!(value(list_contains(&with_null, &dec("1.0"))), true);
    assert_eq!(value(list_contains(&with_null, &dec("2.0"))), true);
    // A term that is not there is a definite `false`, not an error, even with the
    // null sitting between the elements that were compared.
    assert_eq!(value(list_contains(&with_null, &dec("3.0"))), false);
}

#[test]
fn a_definite_hit_dominates_an_undecidable_comparison() {
    // An element in a datatype with no reachable value space cannot be compared with
    // an integer at all. If what is sought is present anyway, the answer is still
    // `true`; if it is not, the deferred type error surfaces rather than being
    // silently reported as "absent".
    let opaque = CdtTerm::Literal(CdtLiteral::typed("zzz", "http://example.org/opaque"));
    let with_opaque = vec![opaque, int("1")];
    assert_eq!(value(list_contains(&with_opaque, &int("1"))), true);
    assert!(list_contains(&with_opaque, &int("9")).is_error());
}

#[test]
fn contains_raises_for_a_map_and_defers_to_contains_key() {
    // The corpus places `cdt:contains` under list-functions only, and gives maps
    // `cdt:containsKey`. Applying it to a map is refused rather than guessed at.
    assert!(contains(&map("{1: 'one'}"), &int("1")).is_error());
    assert_eq!(value(contains(&list("[1]"), &int("1"))), true);
}

// ── cdt:get on a map ──────────────────────────────────────────────────────────

#[test]
fn map_get_matches_keys_by_term_not_by_value() {
    // map-functions/get-01.rq.
    let mixed = entries("{'hello'@en:'there'@en,1:'one',2:'two'}");
    assert_eq!(value(map_get(&mixed, &int("1"))), text("one"));
    assert_eq!(value(map_get(&mixed, &int("2"))), text("two"));
    assert_eq!(
        value(map_get(&mixed, &lang("hello", "en"))),
        lang("there", "en")
    );

    // get-02.rq: `1` and `"02"^^xsd:integer` are two keys of one map, even though
    // `"02"^^xsd:integer` and `2` are one value.
    let lexical =
        entries("{1:'one', 2:'two', '02'^^<http://www.w3.org/2001/XMLSchema#integer>: 'also two'}");
    assert_eq!(value(map_get(&lexical, &int("1"))), text("one"));
    assert_eq!(value(map_get(&lexical, &int("2"))), text("two"));
    assert_eq!(value(map_get(&lexical, &int("02"))), text("also two"));
}

#[test]
fn map_get_returns_blank_node_values_as_they_are() {
    // map-functions/get-03.rq and get-04.rq.
    let shared = entries("{ 1: _:b, 2: _:b }");
    assert_eq!(
        value(map_get(&shared, &int("1"))),
        CdtTerm::Blank("b".into())
    );
    assert_eq!(
        value(map_get(&shared, &int("1"))),
        value(map_get(&shared, &int("2")))
    );
    let distinct = entries("{ 1: _:b1, 2: _:b2 }");
    assert_ne!(
        value(map_get(&distinct, &int("1"))),
        value(map_get(&distinct, &int("2")))
    );
}

#[test]
fn map_get_raises_for_an_absent_key_and_for_a_null_value() {
    // map-functions/get-error-01.rq: the key is not there.
    let gappy = entries("{1:'one',2:'two',4:'four'}");
    assert!(map_get(&gappy, &int("3")).is_error());
    // get-null-01.rq: the key IS there, and its value is a null.
    let with_null = entries("{1:'one',2:'two',3:null,4:'four'}");
    assert!(map_get(&with_null, &int("3")).is_error());
    // A term that could never be a map key is likewise an error.
    assert!(map_get(&with_null, &CdtTerm::Blank("b".into())).is_error());
}

#[test]
fn get_is_one_function_over_both_composite_datatypes() {
    // On a list the second argument is a position, on a map it is a key.
    assert_eq!(value(get(&list("[7, 8]"), &int("2"))), int("8"));
    assert_eq!(value(get(&map("{2: 8}"), &int("2"))), int("8"));
}

// ── cdt:containsKey ───────────────────────────────────────────────────────────

#[test]
fn map_contains_key_is_total_and_matches_by_term() {
    // map-functions/containsKey-01.rq: bound `false`, not an error.
    assert!(!map_contains_key(&entries("{}"), &int("1")));
    // containsKey-02.rq.
    let one = entries("{1: 'one'}");
    assert!(map_contains_key(&one, &int("1")));
    assert!(!map_contains_key(&one, &int("01")));
    // containsKey-03.rq: values are not searched.
    let two = entries("{1: 'one', 2: 'two'}");
    assert!(map_contains_key(&two, &int("1")));
    assert!(map_contains_key(&two, &int("2")));
    assert!(!map_contains_key(&two, &text("one")));
    assert!(!map_contains_key(&two, &text("two")));
    // Not corpus-pinned, but forced by production [7]: a blank node is not a key, so
    // no map holds it.
    assert!(!map_contains_key(&two, &CdtTerm::Blank("b".into())));
}

#[test]
fn contains_key_is_not_the_same_question_as_get() {
    // map-functions/put-02.rq and merge-null-01.rq both build a map with a key whose
    // value is a null, then require `cdt:containsKey` to be true while `cdt:get` on
    // the same key is unbound. An implementation that defined one in terms of the
    // other would fail both.
    let with_null = entries("{1: null}");
    assert!(map_contains_key(&with_null, &int("1")));
    assert!(map_get(&with_null, &int("1")).is_error());
}

#[test]
fn contains_key_raises_for_a_list() {
    assert!(contains_key(&list("[1]"), &int("1")).is_error());
    assert_eq!(value(contains_key(&map("{1: 'one'}"), &int("1"))), true);
}

// ── cdt:keys ──────────────────────────────────────────────────────────────────

#[test]
fn map_keys_is_a_list_of_the_keys() {
    // map-functions/keys-01.rq and keys-02.rq.
    assert_eq!(map_keys(&entries("{}")), list("[]"));
    assert_eq!(map_keys(&entries("{1: 'one'}")), list("[1]"));
    // keys-03.rq checks size and membership rather than an order, because a map is
    // unordered. This crate nonetheless writes a definite order — its own key order
    // — so that the result is byte-deterministic.
    let keys_list = map_keys(&entries("{2: 'two', 1: 'one'}"));
    assert_eq!(size(&keys_list), 2);
    let CdtValue::List(key_items) = &keys_list else {
        panic!("cdt:keys yields a list")
    };
    assert_eq!(value(list_contains(key_items, &int("1"))), true);
    assert_eq!(value(list_contains(key_items, &int("2"))), true);
    // The order is the key order, and it does not depend on how the map was written.
    assert_eq!(keys_list, map_keys(&entries("{1: 'one', 2: 'two'}")));
    assert_eq!(
        keys_list.canonical_lexical(),
        list("[1, 2]").canonical_lexical()
    );
}

#[test]
fn keys_raises_for_a_list() {
    assert!(keys(&list("[1]")).is_error());
}

// ── cdt:put ───────────────────────────────────────────────────────────────────

#[test]
fn map_put_sets_an_entry() {
    // map-functions/put-01.rq.
    assert_eq!(
        value(map_put(&entries("{}"), &int("1"), &text("one"))),
        map("{1:'one'}")
    );
    // put-09.rq: a new key on a non-empty map.
    assert_eq!(
        value(map_put(&entries("{1:'one'}"), &int("2"), &text("two"))),
        map("{1:'one', 2:'two'}")
    );
    // put-04.rq: putting a key's own value back changes nothing.
    let two = entries("{1:'one', 2:'two'}");
    assert_eq!(
        value(map_put(&two, &int("1"), &text("one"))),
        map("{1:'one', 2:'two'}")
    );
    // put-05.rq: an existing key is overwritten.
    assert_eq!(
        value(map_put(&two, &int("1"), &text("alsoOne"))),
        map("{1:'alsoOne', 2:'two'}")
    );
    // put-06.rq: a stored null is replaced by a real term.
    assert_eq!(
        value(map_put(
            &entries("{1:null, 2:'two'}"),
            &int("1"),
            &text("one")
        )),
        map("{1:'one', 2:'two'}")
    );
}

#[test]
fn map_put_stores_a_null_value_without_raising() {
    // map-functions/put-02.rq, put-03.rq, put-07.rq, put-08.rq, put-10.rq and
    // put-11.rq: the two-argument call and an unbound third argument both store a
    // null, and both are BOUND — the entry exists and only `cdt:get` on it raises.
    let out = value(map_put(&entries("{}"), &int("1"), &CdtTerm::Null));
    assert_eq!(size(&out), 1);
    let CdtValue::Map(out_entries) = &out else {
        panic!("cdt:put yields a map")
    };
    assert!(map_contains_key(out_entries, &int("1")));
    assert!(map_get(out_entries, &int("1")).is_error());

    let out = value(map_put(
        &entries("{1:'one', 2:'two'}"),
        &int("1"),
        &CdtTerm::Null,
    ));
    let CdtValue::Map(out_entries) = &out else {
        panic!("cdt:put yields a map")
    };
    assert_eq!(map_size(out_entries), 2);
    assert!(map_contains_key(out_entries, &int("1")));
    assert!(map_get(out_entries, &int("1")).is_error());
    assert_eq!(value(map_get(out_entries, &int("2"))), text("two"));
}

#[test]
fn map_put_treats_the_key_as_a_term() {
    // map-functions/put-12.rq: `"01"^^xsd:integer` lands BESIDE `1`, not on it.
    assert_eq!(
        value(map_put(
            &entries("{1:'one', 2:'two'}"),
            &int("01"),
            &text("alsoOne")
        )),
        map("{1:'one', '01'^^<http://www.w3.org/2001/XMLSchema#integer>:'alsoOne', 2:'two'}")
    );
    // put-13.rq: the same language-tagged key replaces.
    assert_eq!(
        value(map_put(
            &entries("{'hello'@en:'one', 2:'two'}"),
            &lang("hello", "en"),
            &text("alsoOne")
        )),
        map("{'hello'@en:'alsoOne', 2:'two'}")
    );
    // put-14.rq: the plain string is a different key from the tagged one.
    assert_eq!(
        value(map_put(
            &entries("{'hello'@en:'one', 2:'two'}"),
            &text("hello"),
            &text("alsoOne")
        )),
        map("{'hello'@en:'one', 'hello':'alsoOne', 2:'two'}")
    );
    // put-15.rq: an IRI key.
    assert_eq!(
        value(map_put(
            &entries("{<http://example.org/>:'one', 2:'two'}"),
            &iri("http://example.org/"),
            &text("alsoOne")
        )),
        map("{<http://example.org/>:'alsoOne', 2:'two'}")
    );
}

#[test]
fn map_put_refuses_a_key_that_cannot_be_a_key() {
    // map-functions/put-error-03.rq (a blank node) and put-error-04.rq (unbound,
    // which the consumer offers as a null). `cdt:put` raises where the map
    // constructor drops the pair and `cdt:remove` returns the map untouched.
    let empty = entries("{}");
    assert!(map_put(&empty, &CdtTerm::Blank("b".into()), &text("one")).is_error());
    assert!(map_put(&empty, &CdtTerm::Null, &text("one")).is_error());
    assert!(map_put(&empty, &CdtTerm::composite(list("[1]")), &text("one")).is_error());
}

#[test]
fn put_raises_for_a_list() {
    assert!(put(&list("[1]"), &int("1"), &text("one")).is_error());
}

// ── cdt:remove ────────────────────────────────────────────────────────────────

#[test]
fn map_remove_drops_the_entry_under_a_key() {
    // map-functions/remove-03.rq, remove-04.rq, remove-05.rq, remove-08.rq,
    // remove-11.rq.
    assert_eq!(
        map_remove(&entries("{1:'one'}"), &int("1")),
        MapRemoval::Removed(map("{}"))
    );
    assert_eq!(
        map_remove(&entries("{1:'one',2:'two'}"), &int("1")),
        MapRemoval::Removed(map("{2:'two'}"))
    );
    assert_eq!(
        map_remove(
            &entries("{1:'one', '02'^^<http://www.w3.org/2001/XMLSchema#integer>:'two'}"),
            &int("02")
        ),
        MapRemoval::Removed(map("{1:'one'}"))
    );
    assert_eq!(
        map_remove(
            &entries("{'hello'@en:'there'@en, 1:'one', 2:'two'}"),
            &lang("hello", "en")
        ),
        MapRemoval::Removed(map("{1:'one', 2:'two'}"))
    );
    assert_eq!(
        map_remove(
            &entries("{<http://example.org/>:'there'@en, 1:'one', 2:'two'}"),
            &iri("http://example.org/")
        ),
        MapRemoval::Removed(map("{1:'one', 2:'two'}"))
    );
}

#[test]
fn map_remove_reports_that_it_changed_nothing() {
    // map-functions/remove-01.rq requires `SAMETERM(?mapIn, ?mapOut)` after removing
    // a `BNODE()` key — so a consumer must hand back its ORIGINAL literal, with its
    // original lexical form, rather than an equal map re-rendered canonically. That
    // is what `MapRemoval::Unchanged` exists to say.
    let two = entries("{1:'one',2:'two'}");
    assert_eq!(
        map_remove(&two, &CdtTerm::Blank("b".into())),
        MapRemoval::Unchanged
    );
    // remove-02.rq: the empty map.
    assert_eq!(map_remove(&entries("{}"), &int("1")), MapRemoval::Unchanged);
    // remove-06.rq and remove-07.rq: a key that differs only in lexical form.
    let lexical = entries("{1:'one', '02'^^<http://www.w3.org/2001/XMLSchema#integer>:'two'}");
    assert_eq!(map_remove(&lexical, &int("2")), MapRemoval::Unchanged);
    assert_eq!(map_remove(&two, &int("02")), MapRemoval::Unchanged);
    // remove-09.rq and remove-10.rq: a key that differs only in the language tag.
    let tagged = entries("{'hello'@en:'there'@en, 1:'one', 2:'two'}");
    assert_eq!(map_remove(&tagged, &text("hello")), MapRemoval::Unchanged);
    let plain = entries("{'hello':'there', 1:'one', 2:'two'}");
    assert_eq!(
        map_remove(&plain, &lang("hello", "en")),
        MapRemoval::Unchanged
    );
}

#[test]
fn remove_raises_for_a_list() {
    assert!(remove(&list("[1]"), &int("1")).is_error());
    assert_eq!(
        value(remove(&map("{1:'one'}"), &int("1"))),
        MapRemoval::Removed(map("{}"))
    );
}

// ── cdt:merge ─────────────────────────────────────────────────────────────────

#[test]
fn map_merge_unions_the_arguments() {
    // map-functions/merge-01.rq … merge-04.rq.
    let empty = entries("{}");
    let one = entries("{1: 'one'}");
    assert_eq!(value(map_merge(&[&empty, &empty])), map("{}"));
    assert_eq!(value(map_merge(&[&one, &empty])), map("{1: 'one'}"));
    assert_eq!(value(map_merge(&[&empty, &one])), map("{1: 'one'}"));
    let two = entries("{2: 'two'}");
    assert_eq!(value(map_merge(&[&one, &two])), map("{1: 'one', 2: 'two'}"));
    // merge-07.rq: keys of every category.
    let left = entries("{<http://example.org/1>: 'one', 2: 'two'}");
    let right = entries("{'hello'@en: <http://example.org/>}");
    assert_eq!(
        value(map_merge(&[&left, &right])),
        map("{<http://example.org/1>: 'one', 2: 'two', 'hello'@en: <http://example.org/>}")
    );
}

#[test]
fn map_merge_keeps_the_first_map_on_a_conflict() {
    // map-functions/merge-05.rq: the LEFT argument's value survives.
    let left = entries("{1: 'one', 2: 'two'}");
    let right = entries("{1: 'another one', 3: 'three'}");
    assert_eq!(
        value(map_merge(&[&left, &right])),
        map("{1: 'one', 2: 'two', 3: 'three'}")
    );
    // merge-08.rq, with an IRI key in conflict and two string keys that are not.
    let left = entries("{<http://example.org/1>: 'one', 'hello': 42}");
    let right = entries("{<http://example.org/1>: 'ONE', 'hello'@en: <http://example.org/>}");
    assert_eq!(
        value(map_merge(&[&left, &right])),
        map("{<http://example.org/1>: 'one', 'hello': 42, 'hello'@en: <http://example.org/>}")
    );
    // merge-06.rq: `1` and `"01"^^xsd:integer` are not in conflict at all.
    let left = entries("{1: 'one', 2: 'two'}");
    let right =
        entries("{'01'^^<http://www.w3.org/2001/XMLSchema#integer>: 'another one', 3: 'three'}");
    assert_eq!(
        value(map_merge(&[&left, &right])),
        map(
            "{1: 'one', '01'^^<http://www.w3.org/2001/XMLSchema#integer>: 'another one', \
             2: 'two', 3: 'three'}"
        )
    );
}

#[test]
fn a_stored_null_wins_a_merge_conflict_like_any_other_value() {
    // map-functions/merge-null-03.rq: `{1: null}` merged with `{1: 'one'}` keeps the
    // NULL, because the left map wins and a null is a value, not an absence.
    let left = entries("{1: null}");
    let right = entries("{1: 'one'}");
    let merged = value(map_merge(&[&left, &right]));
    assert_eq!(size(&merged), 1);
    let CdtValue::Map(merged_entries) = &merged else {
        panic!("cdt:merge yields a map")
    };
    assert!(map_contains_key(merged_entries, &int("1")));
    assert!(map_get(merged_entries, &int("1")).is_error());

    // merge-null-04.rq: the other way round, the left map's real value wins.
    let left = entries("{1: 'one'}");
    let right = entries("{1: null}");
    assert_eq!(value(map_merge(&[&left, &right])), map("{1: 'one'}"));

    // merge-null-01.rq and merge-null-02.rq: a null on either side survives when it
    // is not in conflict.
    let left = entries("{1: null, 2: 'two'}");
    let right = entries("{3: 'three'}");
    let merged = value(map_merge(&[&left, &right]));
    let CdtValue::Map(merged_entries) = &merged else {
        panic!("cdt:merge yields a map")
    };
    assert_eq!(map_size(merged_entries), 3);
    assert!(map_contains_key(merged_entries, &int("1")));
    assert!(map_get(merged_entries, &int("1")).is_error());
    assert_eq!(value(map_get(merged_entries, &int("2"))), text("two"));
    assert_eq!(value(map_get(merged_entries, &int("3"))), text("three"));
}

#[test]
fn merge_raises_for_a_list_argument() {
    assert!(merge(&[map("{}"), list("[1]")]).is_error());
    assert_eq!(
        value(merge(&[map("{1: 2}"), map("{3: 4}")])),
        map("{1:2,3:4}")
    );
}

// ── cdt:List ──────────────────────────────────────────────────────────────────

#[test]
fn the_list_constructor_builds_in_argument_order() {
    // list-functions/list-constructor-01.rq … list-constructor-11.rq.
    assert_eq!(value(list_constructor(Vec::new())), list("[]"));
    assert_eq!(value(list_constructor(vec![int("1")])), list("[1]"));
    assert_eq!(
        value(list_constructor(vec![int("1"), int("2")])),
        list("[1, 2]")
    );
    assert_eq!(value(list_constructor(vec![text("a")])), list("['a']"));
    assert_eq!(
        value(list_constructor(vec![text("a"), int("1")])),
        list("['a', 1]")
    );
    assert_eq!(
        value(list_constructor(vec![CdtTerm::composite(list("[]"))])),
        list("[[]]")
    );
    assert_eq!(
        value(list_constructor(vec![
            CdtTerm::composite(list("[1]")),
            int("2")
        ])),
        list("[[1], 2]")
    );
}

#[test]
fn a_failed_constructor_argument_becomes_a_null_element() {
    // list-functions/list-constructor-null-01.rq requires `cdt:List(?unbound)` to
    // spell `[null]`, and list-constructor-null-02.rq requires the same of
    // `cdt:List(1/0)` — an argument whose own evaluation RAISED. So the constructor
    // never fails because of an argument; the consumer maps each failed argument to
    // `CdtTerm::Null` and the value carries the position.
    let built = value(list_constructor(vec![CdtTerm::Null]));
    assert_eq!(built.canonical_lexical(), "[null]");
    // list-constructor-12.rq: mixed with a real term.
    let built = value(list_constructor(vec![
        CdtTerm::Null,
        iri("http://example.org/"),
    ]));
    assert_eq!(built.canonical_lexical(), "[null,<http://example.org/>]");
    // sameterm-null-01.rq: two lists built from different failed arguments are the
    // same term, because nulls are indistinguishable.
    let a = value(list_constructor(vec![int("1"), CdtTerm::Null, int("2")]));
    let b = value(list_constructor(vec![int("1"), CdtTerm::Null, int("2")]));
    assert_eq!(a.canonical_lexical(), b.canonical_lexical());
}

#[test]
fn a_constructed_list_keeps_a_blank_node_argument() {
    // list-functions/list-constructor-16.rq: the constructor does not raise on a
    // blank node, and `cdt:get` returns the very same blank node.
    let built = value(list_constructor(vec![CdtTerm::Blank("b".into())]));
    let CdtValue::List(built_items) = &built else {
        panic!("the list constructor yields a list")
    };
    assert_eq!(
        value(list_get(built_items, &int("1"))),
        CdtTerm::Blank("b".into())
    );
}

// ── cdt:Map ───────────────────────────────────────────────────────────────────

#[test]
fn the_map_constructor_pairs_keys_with_values() {
    // map-functions/map-constructor-01.rq, map-constructor-02.rq.
    assert_eq!(value(map_constructor(&[])), map("{}"));
    assert_eq!(
        value(map_constructor(&[
            (int("1"), int("2")),
            (int("3"), int("4"))
        ])),
        map("{1:2, 3:4}")
    );
    // map-constructor-07.rq: IRIs on both sides.
    assert_eq!(
        value(map_constructor(&[
            (iri("http://example.org/"), text("uri")),
            (text("hello"), iri("http://example.org/string"))
        ])),
        map("{<http://example.org/>:'uri', 'hello':<http://example.org/string>}")
    );
}

#[test]
fn the_map_constructor_lets_the_last_binding_of_a_key_win() {
    // map-functions/map-constructor-03.rq.
    assert_eq!(
        value(map_constructor(&[
            (int("1"), int("2")),
            (int("1"), int("4"))
        ])),
        map("{1: 4}")
    );
    // map-constructor-04.rq: `1` and `"01"^^xsd:integer` are two keys, so neither
    // wins over the other.
    assert_eq!(
        value(map_constructor(&[
            (int("1"), text("one")),
            (int("01"), text("also one"))
        ])),
        map("{1:'one', '01'^^<http://www.w3.org/2001/XMLSchema#integer>: 'also one'}")
    );
    // map-constructor-05.rq: three pairs, the repeat wins over its own earlier entry
    // and leaves the distinct key alone.
    assert_eq!(
        value(map_constructor(&[
            (int("1"), text("one")),
            (int("01"), text("also one")),
            (int("1"), text("one again"))
        ])),
        map("{1:'one again', '01'^^<http://www.w3.org/2001/XMLSchema#integer>: 'also one'}")
    );
    // map-constructor-06.rq: a language tag makes a distinct key.
    assert_eq!(
        value(map_constructor(&[
            (lang("hello", "en"), text("one")),
            (text("hello"), text("also one"))
        ])),
        map("{'hello'@en:'one', 'hello':'also one'}")
    );
}

#[test]
fn the_map_constructor_drops_a_pair_whose_key_cannot_be_a_key() {
    // map-functions/map-constructor-08.rq: an unbound key in the middle of three
    // pairs. The pair vanishes and the call does NOT raise.
    assert_eq!(
        value(map_constructor(&[
            (int("1"), int("2")),
            (CdtTerm::Null, int("4")),
            (int("5"), int("6"))
        ])),
        map("{1:2, 5:6}")
    );
    // map-constructor-09.rq: the same for a blank node, which is not a valid key.
    assert_eq!(
        value(map_constructor(&[
            (int("1"), int("2")),
            (CdtTerm::Blank("b".into()), int("4")),
            (int("5"), int("6"))
        ])),
        map("{1:2, 5:6}")
    );
}

#[test]
fn the_map_constructor_keeps_an_entry_whose_value_failed() {
    // map-functions/map-constructor-10.rq: an unbound VALUE keeps the entry and
    // stores a null, which is the opposite of what an unbound KEY does.
    let built = value(map_constructor(&[
        (int("1"), int("2")),
        (int("3"), CdtTerm::Null),
    ]));
    assert_eq!(size(&built), 2);
    let CdtValue::Map(built_entries) = &built else {
        panic!("the map constructor yields a map")
    };
    assert_eq!(value(map_get(built_entries, &int("1"))), int("2"));
    assert!(map_contains_key(built_entries, &int("3")));
    assert!(map_get(built_entries, &int("3")).is_error());

    // map-constructor-11.rq: a blank node is a fine VALUE, and comes back unchanged.
    let built = value(map_constructor(&[
        (int("1"), int("2")),
        (int("3"), CdtTerm::Blank("b".into())),
    ]));
    let CdtValue::Map(built_entries) = &built else {
        panic!("the map constructor yields a map")
    };
    assert_eq!(
        value(map_get(built_entries, &int("3"))),
        CdtTerm::Blank("b".into())
    );
}

// ── Dispatch, and the wrong composite datatype ────────────────────────────────

#[test]
fn the_list_only_functions_raise_for_a_map() {
    let a_map = map("{1: 'one'}");
    assert!(head(&a_map).is_error());
    assert!(tail(&a_map).is_error());
    assert!(reverse(&a_map).is_error());
    assert!(subseq(&a_map, &int("1"), None).is_error());
    assert!(contains(&a_map, &int("1")).is_error());
    // …and answer for a list.
    let a_list = list("[1, 2]");
    assert_eq!(value(head(&a_list)), int("1"));
    assert_eq!(value(tail(&a_list)), list("[2]"));
    assert_eq!(value(reverse(&a_list)), list("[2,1]"));
    assert_eq!(
        value(subseq(&a_list, &int("1"), Some(&int("1")))),
        list("[1]")
    );
}

#[test]
fn the_map_only_functions_raise_for_a_list() {
    let a_list = list("[1]");
    assert!(keys(&a_list).is_error());
    assert!(contains_key(&a_list, &int("1")).is_error());
    assert!(put(&a_list, &int("1"), &int("2")).is_error());
    assert!(remove(&a_list, &int("1")).is_error());
    assert!(merge(&[a_list]).is_error());
}

// ── The three outcomes ────────────────────────────────────────────────────────

#[test]
fn the_three_outcomes_are_distinguishable() {
    let a_list = list("[1]");
    let CdtValue::List(list_items) = &a_list else {
        panic!("parse_list yields a list")
    };
    let ok = list_get(list_items, &int("1"));
    assert!(ok.is_value() && !ok.is_error() && !ok.is_bound());
    let raised = list_get(list_items, &int("9"));
    assert!(!raised.is_value() && raised.is_error() && !raised.is_bound());
    assert_eq!(raised.value(), None);

    // A bound refusal is NOT an expression error: a consumer must fail the query
    // rather than leave a variable unbound.
    let refused = deep_enough_to_refuse();
    assert!(!refused.is_value() && !refused.is_error() && refused.is_bound());
}

/// Nest lists until one more level would exceed [`MAX_NESTING_DEPTH`].
fn deep_enough_to_refuse() -> CdtOutcome<CdtValue> {
    let mut built = value(list_constructor(Vec::new()));
    for _ in 1..MAX_NESTING_DEPTH {
        built = value(list_constructor(vec![CdtTerm::composite(built)]));
    }
    assert_eq!(built.depth(), MAX_NESTING_DEPTH);
    list_constructor(vec![CdtTerm::composite(built)])
}

// ── The three bounds, on values that never passed the scanner ─────────────────

#[test]
fn a_minted_value_may_not_nest_deeper_than_the_bound() {
    let refused = deep_enough_to_refuse();
    assert_eq!(
        refused,
        CdtOutcome::Bound(CdtError::DepthExceeded {
            offset: 0,
            limit: MAX_NESTING_DEPTH,
        })
    );
}

#[test]
fn repeated_self_insertion_hits_the_element_bound_instead_of_memory() {
    // `cdt:put(?m, ?k, ?m)` roughly doubles a map's element count each time it is
    // applied, so a query of a few lines can ask for a value no host can hold. The
    // element bound is checked against the PROSPECTIVE result, from borrowed inputs,
    // so the refusal costs no allocation at all.
    //
    // The seed is sized so that the doubling crosses `MAX_ELEMENTS` on the second
    // application: with N elements in the seed's list, the results have 2N+3 and
    // 4N+7 elements.
    let seed_len = MAX_ELEMENTS / 4;
    let big_list = CdtValue::List(vec![CdtTerm::Null; seed_len]);
    let seed = value(map_constructor(&[(
        text("k0"),
        CdtTerm::composite(big_list),
    )]));
    assert_eq!(seed.element_count(), seed_len + 1);

    // First application: it fits, and it really does produce the doubled value.
    let CdtValue::Map(seed_entries) = &seed else {
        panic!("the map constructor yields a map")
    };
    let once = value(map_put(
        seed_entries,
        &text("k1"),
        &CdtTerm::composite(seed.clone()),
    ));
    assert_eq!(once.element_count(), 2 * seed_len + 3);
    assert!(once.element_count() <= MAX_ELEMENTS);
    assert_eq!(size(&once), 2);

    // Second application: it would be 4N+7 elements, which is over the bound, so it
    // is refused rather than built.
    let CdtValue::Map(once_entries) = &once else {
        panic!("cdt:put yields a map")
    };
    let twice = map_put(once_entries, &text("k2"), &CdtTerm::composite(once.clone()));
    assert_eq!(
        twice,
        CdtOutcome::Bound(CdtError::TooManyElements {
            offset: 0,
            limit: MAX_ELEMENTS,
        })
    );
}

#[test]
fn concat_refuses_to_exceed_the_element_bound() {
    let half = vec![CdtTerm::Null; MAX_ELEMENTS / 2 + 1];
    // Two halves are one element over the bound, and the refusal happens before the
    // joined vector is allocated.
    assert_eq!(
        list_concat(&[&half, &half]),
        CdtOutcome::Bound(CdtError::TooManyElements {
            offset: 0,
            limit: MAX_ELEMENTS,
        })
    );
    // One of them on its own still fits, and really is produced.
    assert_eq!(value(list_concat(&[&half])).len(), MAX_ELEMENTS / 2 + 1);
}

#[test]
fn a_minted_value_may_not_exceed_the_lexical_byte_bound() {
    // One element is enough when its lexical form is large: the canonical form of a
    // list is at least as long as the elements it holds.
    let huge = CdtTerm::Literal(CdtLiteral::plain("a".repeat(MAX_LEXICAL_BYTES + 1)));
    match list_constructor(vec![huge]) {
        CdtOutcome::Bound(CdtError::InputTooLarge { offset, length }) => {
            assert_eq!(offset, MAX_LEXICAL_BYTES);
            assert!(length > MAX_LEXICAL_BYTES);
        }
        other => panic!("expected a byte-bound refusal, got {other:?}"),
    }
    // A large but admissible element is built, and its measured length is exactly
    // the length of the form it would be written as.
    let big = CdtTerm::Literal(CdtLiteral::plain("a".repeat(1024)));
    let built = value(list_constructor(vec![big]));
    assert_eq!(
        purrdf_cdt::canonical_lexical_len(&built),
        built.canonical_lexical().len()
    );
}

// ── Key admissibility ─────────────────────────────────────────────────────────

#[test]
fn only_iris_and_literals_can_be_map_keys() {
    assert_eq!(
        CdtKey::from_term(&iri("http://example.org/k")),
        Some(CdtKey::Iri("http://example.org/k".into()))
    );
    assert_eq!(
        CdtKey::from_term(&text("k")),
        Some(CdtKey::Literal(CdtLiteral::plain("k")))
    );
    assert_eq!(CdtKey::from_term(&CdtTerm::Blank("b".into())), None);
    assert_eq!(CdtKey::from_term(&CdtTerm::Null), None);
    assert_eq!(CdtKey::from_term(&CdtTerm::composite(list("[1]"))), None);
    assert_eq!(
        CdtKey::from_term(&CdtTerm::triple(
            iri("http://example.org/s"),
            iri("http://example.org/p"),
            iri("http://example.org/o")
        )),
        None
    );
}
