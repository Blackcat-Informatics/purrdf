// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `ORDER BY` over SEP-0009 composite literals (`cdt:List` / `cdt:Map`), end to
//! end through the PUBLIC [`NativeSparqlEngine`] query entry — never reaching into
//! the crate.
//!
//! # What has to be true, and why each case is here
//!
//! A composite literal sorts by the VALUE it denotes, not by its lexical form.
//! That is the whole point: `"[ 2]"^^cdt:List` and `"[2]"^^cdt:List` are the same
//! list written two ways, and a sort that compared the strings would put `"[ 2]"`
//! before `"[1]"` because a space precedes a digit in Unicode. Every case below
//! therefore varies the whitespace deliberately, so a lexical-form comparison
//! cannot pass it by accident.
//!
//! The order itself is `purrdf_cdt`'s SYNTACTIC total order
//! ([`purrdf_cdt::total_value_cmp`]), not SEP-0009's value relations: the value
//! relations are partial and RAISE, and `ORDER BY` must be total and must never
//! error. Two elements that are value-equal but lexically distinct
//! (`'01'^^xsd:integer` vs `'001'^^xsd:integer`) therefore do NOT tie — the
//! syntactic order separates them — and the corpus these cases mirror accepts
//! either winner for exactly that pair.
//!
//! Each case is an `ASK` around a sorted sub-`SELECT` with `LIMIT 1`: sort, take
//! the extreme, and assert which row won. That reads the ORDER off the production
//! evaluator rather than off a comparator called directly.

use std::fmt::Write as _;
use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlEngine, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::NativeSparqlEngine;

const CDT_LIST: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List";
const CDT_MAP: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/Map";

/// An empty default graph — every case's data comes from its own `VALUES` block.
fn empty_dataset() -> Arc<RdfDataset> {
    RdfDatasetBuilder::new().freeze().expect("empty dataset")
}

/// Evaluate one `ASK` and answer its boolean.
fn ask(query: &str) -> bool {
    let engine = NativeSparqlEngine::new();
    let result = engine
        .query(
            &empty_dataset(),
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
        )
        .expect("the query evaluates");
    match result {
        SparqlResult::Boolean(b) => b,
        other => panic!("expected a boolean ASK result, got {other:?}"),
    }
}

/// Sort `rows` — `(id, lexical form)` pairs of one composite datatype — with the
/// production `ORDER BY`, and answer the `?id` of the row the given direction puts
/// first.
///
/// `ORDER BY` is a STABLE sort, so a tie keeps the earlier `VALUES` row; a case
/// that expects a tie asserts on that.
fn extreme(datatype: &str, direction: &str, rows: &[(u32, &str)]) -> u32 {
    let mut bindings = String::new();
    for (id, lexical) in rows {
        writeln!(bindings, "({id} \"{lexical}\"^^<{datatype}>)").expect("writing to a String");
    }
    let query = format!(
        "SELECT ?id WHERE {{\n\
           {{ SELECT * WHERE {{ VALUES (?id ?v) {{ {bindings} }} }} \
             ORDER BY {direction}(?v) LIMIT 1 }}\n\
         }}"
    );
    let engine = NativeSparqlEngine::new();
    let result = engine
        .query(
            &empty_dataset(),
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
        )
        .expect("the query evaluates");
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected solutions")
    };
    assert_eq!(rows.len(), 1, "LIMIT 1 must yield exactly one row");
    let Some(Some(TermValue::Literal { lexical_form, .. })) = rows[0].first() else {
        panic!("?id must be bound to a literal")
    };
    lexical_form.parse().expect("?id is an integer lexical")
}

/// Lists: empty first, then first difference, then length — and never the lexical
/// form.
#[test]
fn list_order_is_elementwise_then_by_length() {
    // An empty list is the minimum, and the two spellings of it TIE, so the
    // stable sort keeps the earlier row.
    assert_eq!(
        extreme(
            CDT_LIST,
            "ASC",
            &[
                (1, "[1]"),
                (2, "[ ]"),
                (3, "[    2]"),
                (4, "[   ]"),
                (5, "[3]")
            ],
        ),
        2,
    );
    // A single element decides, whatever the whitespace around it.
    assert_eq!(extreme(CDT_LIST, "ASC", &[(1, "[1]"), (2, "[ 2]")]), 1);
    assert_eq!(extreme(CDT_LIST, "DESC", &[(1, "[1]"), (2, "[ 2]")]), 2);
    // A prefix sorts before its extension: length is the LAST tie-break.
    assert_eq!(extreme(CDT_LIST, "ASC", &[(1, "[ 1, 1]"), (2, "[1]")]), 2);
    // ...but the first difference beats length outright.
    assert_eq!(extreme(CDT_LIST, "ASC", &[(1, "[1, 1]"), (2, "[2]")]), 1);
    // The second element decides once the first ties.
    assert_eq!(extreme(CDT_LIST, "ASC", &[(1, "[2, 1]"), (2, "[2, 2]")]), 1);
}

/// IRI elements order by their string, in any position.
#[test]
fn list_iri_elements_order_by_their_string() {
    assert_eq!(
        extreme(
            CDT_LIST,
            "ASC",
            &[
                (1, "[<http://example.org>, <http://example.org/1>]"),
                (2, "[<http://example.org>, <http://example.org/2>]"),
            ],
        ),
        1,
    );
    assert_eq!(
        extreme(
            CDT_LIST,
            "ASC",
            &[
                (1, "[<http://example.org>, 1]"),
                (2, "[<http://example.org>, 2]"),
            ],
        ),
        1,
    );
}

/// `null` sorts BELOW any non-null at the same position — asserted from both ends,
/// so a `DESC` run proves the non-null element really is the maximum rather than
/// merely not the minimum.
#[test]
fn null_sorts_below_every_other_element() {
    assert_eq!(
        extreme(
            CDT_LIST,
            "DESC",
            &[
                (1, "[   null ,  1]"),
                (2, "[ 'hello',  2]"),
                (3, "[null    ,  3]"),
            ],
        ),
        2,
    );
    assert_eq!(
        extreme(CDT_LIST, "ASC", &[(1, "[null, 1]"), (2, "[null,  2]")]),
        1,
    );
    assert_eq!(
        extreme(
            CDT_MAP,
            "DESC",
            &[
                (1, "{   1:null,  2:41}"),
                (2, "{  1:'hello', 2:42}"),
                (3, "{ 1:null,  2:43}"),
            ],
        ),
        2,
    );
    assert_eq!(
        extreme(
            CDT_MAP,
            "ASC",
            &[(1, "{ 1:null, 2:42}"), (2, "{1:null,  2:41}")],
        ),
        2,
    );
}

/// Maps compare entries in KEY order, not authoring order — key first, then value,
/// then length.
#[test]
fn map_order_is_by_sorted_entries_key_before_value() {
    // Authored `{3:41, 1:42, 2:43}` vs `{3:42, 1:42, 2:42}`: read in key order the
    // first entries tie, and the SECOND entry's value (43 vs 42) decides. Comparing
    // in authoring order would have let `3:41 < 3:42` decide the other way.
    assert_eq!(
        extreme(
            CDT_MAP,
            "ASC",
            &[(1, "{3: 41, 1: 42, 2: 43}"), (2, "{3: 42, 1: 42, 2: 42}")],
        ),
        2,
    );
    // The empty map is the minimum, and its two spellings tie.
    assert_eq!(
        extreme(
            CDT_MAP,
            "ASC",
            &[
                (1, "{ 1:42 }"),
                (2, "{ }"),
                (3, "{    2:42}"),
                (4, "{   }"),
                (5, "{ 3:42 }"),
            ],
        ),
        2,
    );
    // Key decides before value.
    assert_eq!(
        extreme(CDT_MAP, "ASC", &[(1, "{1: 42}"), (2, "{ 2: 42}")]),
        1
    );
    assert_eq!(
        extreme(CDT_MAP, "ASC", &[(1, "{1: 42}"), (2, "{ 1: 43}")]),
        1
    );
    // A prefix sorts before its extension...
    assert_eq!(
        extreme(CDT_MAP, "ASC", &[(1, "{ 1: 42, 3: 42}"), (2, "{1: 42}")]),
        2,
    );
    // ...but the first key difference beats length.
    assert_eq!(
        extreme(CDT_MAP, "ASC", &[(1, "{1: 42, 3: 42}"), (2, "   {2: 42}")]),
        1,
    );
    // And a later entry still decides once the earlier ones tie.
    assert_eq!(
        extreme(
            CDT_MAP,
            "ASC",
            &[(1, "   {1: 42, 3: 42}"), (2, "{1: 42, 2: 42}")],
        ),
        2,
    );
}

/// Keys order IRI before literal, and among typed literals the datatype outranks
/// the lexical form.
#[test]
fn map_keys_rank_iris_before_literals_and_datatype_before_lexical() {
    assert_eq!(
        extreme(
            CDT_MAP,
            "ASC",
            &[
                (1, "{ <http://example.org> : 42 }"),
                (2, "{ '2'^^<http://www.w3.org/2001/XMLSchema#integer>: 42}"),
            ],
        ),
        1,
    );
    // `xsd:integer` precedes `xsd:string` as a datatype IRI, so the integer key
    // wins even though its lexical form ('2') is the larger of the two.
    assert_eq!(
        extreme(
            CDT_MAP,
            "ASC",
            &[
                (
                    1,
                    "  { '1'^^<http://www.w3.org/2001/XMLSchema#string>: 42 }"
                ),
                (
                    2,
                    "{  '2'^^<http://www.w3.org/2001/XMLSchema#integer>: 42 }"
                ),
            ],
        ),
        2,
    );
    assert_eq!(
        extreme(
            CDT_MAP,
            "ASC",
            &[
                (1, "   { <http://example.org/2> : 42 }"),
                (2, "{  <http://example.org/1> : 42 }"),
            ],
        ),
        2,
    );
    // A map VALUE ranks an IRI before a literal too.
    assert_eq!(
        extreme(
            CDT_MAP,
            "ASC",
            &[(1, "{ 1: 1 }"), (2, "{ 1: <http://example.org> }")],
        ),
        2,
    );
}

/// Value-equal but lexically distinct elements are SEPARATED by the syntactic
/// order rather than tied — which is what makes the order total — and the
/// separation is by the crate's documented component order (lexical form, after
/// the datatype).
#[test]
fn value_equal_elements_are_separated_syntactically_not_tied() {
    const ONE: &str = "'01'^^<http://www.w3.org/2001/XMLSchema#integer>";
    const OH_ONE: &str = "'001'^^<http://www.w3.org/2001/XMLSchema#integer>";
    // '001' precedes '01' by Unicode scalar order on the lexical form.
    assert_eq!(
        extreme(
            CDT_LIST,
            "ASC",
            &[(1, &format!("[{ONE}]")), (2, &format!("[{OH_ONE}]"))],
        ),
        2,
    );
    assert_eq!(
        extreme(
            CDT_MAP,
            "ASC",
            &[
                (1, &format!("{{     {ONE}: 42 }}")),
                (2, &format!("{{ {OH_ONE}: 42 }}")),
            ],
        ),
        2,
    );
    // The whole point: the two are NOT equal, in either direction.
    assert_eq!(
        extreme(
            CDT_LIST,
            "DESC",
            &[(1, &format!("[{ONE}]")), (2, &format!("[{OH_ONE}]"))],
        ),
        1,
    );
}

/// A composite whose lexical form does NOT parse is an ordinary opaque literal —
/// sorted by its lexical form, ranked below every composite that does parse — and
/// the sort stays total over the mixture instead of erroring.
#[test]
fn an_unparsable_composite_literal_stays_an_ordinary_opaque_literal() {
    // `[1,` is not a list. It must not raise, and it must not outrank a real one.
    assert_eq!(
        extreme(CDT_LIST, "ASC", &[(1, "[0]"), (2, "[1,")]),
        2,
        "an opaque literal ranks below every parsed composite"
    );
    assert_eq!(extreme(CDT_LIST, "DESC", &[(1, "[0]"), (2, "[1,")]), 1);
    // Two opaque ones fall back to the lexical form, deterministically.
    assert_eq!(extreme(CDT_LIST, "ASC", &[(1, "[b,"), (2, "[a,")]), 2);
}

/// Where a composite literal sits relative to the OTHER term kinds is NOT pinned
/// by SEP-0009's corpus — every case there sorts composites against composites —
/// so this crate chooses, documents the choice on `SortKey::Composite`, and pins it
/// here so it cannot drift silently.
///
/// The choice: unbound < blank node < IRI < ordinary literal < composite literal <
/// triple term. A composite IS a literal, so it stays on the literal side of
/// §15.1's kind order; it is a container of terms, so it sits beside the other
/// container rather than interleaved with the scalars.
///
/// The two ordinary literals below are in the crate's own comparability-class
/// order (numeric before text — see `ValueClass`), which is settled ground this
/// case does not restate; what it pins is that BOTH of them precede either
/// composite, and that both composites precede the triple term.
#[test]
fn a_composite_literal_sorts_after_every_plain_literal_and_before_a_triple_term() {
    let engine = NativeSparqlEngine::new();
    let query = "\
SELECT ?id WHERE {
  { SELECT * WHERE {
      VALUES (?id ?v) {
        (1 UNDEF)
        (2 <http://example.org/i>)
        (3 \"999\"^^<http://www.w3.org/2001/XMLSchema#integer>)
        (4 \"zzz\")
        (5 \"[1]\"^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/List>)
        (6 \"{ 1:2 }\"^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/Map>)
        (7 <<( <http://example.org/s> <http://example.org/p> <http://example.org/o> )>>)
      }
  } ORDER BY ?v }
}";
    let result = engine
        .query(
            &empty_dataset(),
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
        )
        .expect("the query evaluates");
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected solutions")
    };
    let ids: Vec<String> = rows
        .iter()
        .map(|row| match row.first() {
            Some(Some(TermValue::Literal { lexical_form, .. })) => lexical_form.clone(),
            other => panic!("?id must be a bound literal, got {other:?}"),
        })
        .collect();
    assert_eq!(
        ids,
        vec!["1", "2", "3", "4", "5", "6", "7"],
        "unbound < IRI < ordinary literal < cdt:List < cdt:Map < triple term"
    );
}

/// A blank node sits between unbound and IRI, and a composite still ranks above
/// it — the arm of the placement a `VALUES` block cannot express (`VALUES` admits
/// no blank node), asserted through a dataset instead.
#[test]
fn a_blank_node_still_ranks_below_a_composite_literal() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/s");
    let p = b.intern_iri("http://example.org/p");
    let blank = b.intern_blank("x", purrdf_core::BlankScope::DEFAULT);
    let list = b.intern_literal(RdfLiteral::typed("[1]", CDT_LIST));
    b.push_quad(s, p, blank, None);
    b.push_quad(s, p, list, None);
    let ds = b.freeze().expect("freeze");

    let engine = NativeSparqlEngine::new();
    let result = engine
        .query(
            &ds,
            SparqlRequest {
                query: "SELECT ?o WHERE { ?s ?p ?o } ORDER BY ?o",
                base_iri: None,
                substitutions: &[],
            },
        )
        .expect("the query evaluates");
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected solutions")
    };
    assert_eq!(rows.len(), 2);
    assert!(
        matches!(rows[0].first(), Some(Some(TermValue::Blank { .. }))),
        "the blank node sorts first, below the composite literal"
    );
}

/// The whitespace point, stated as one `ASK` in the corpus's own shape: a sort that
/// compared lexical forms would answer `false` here, because a space precedes a
/// digit in Unicode scalar order.
#[test]
fn the_sort_is_not_over_the_lexical_form() {
    assert!(ask(
        "PREFIX cdt: <http://w3id.org/awslabs/neptune/SPARQL-CDTs/>\n\
         ASK {\n\
           { SELECT * WHERE { VALUES (?id ?list) {\n\
               (1 \"[1]\"^^cdt:List)\n\
               (2 \"[ 2]\"^^cdt:List)\n\
             } } ORDER BY ?list LIMIT 1 }\n\
           FILTER (?id = 1)\n\
         }"
    ));
    assert!(ask(
        "PREFIX cdt: <http://w3id.org/awslabs/neptune/SPARQL-CDTs/>\n\
         ASK {\n\
           { SELECT * WHERE { VALUES (?id ?map) {\n\
               (1 \"{1: 42}\"^^cdt:Map)\n\
               (2 \"{ 2: 42}\"^^cdt:Map)\n\
             } } ORDER BY ?map LIMIT 1 }\n\
           FILTER (?id = 1)\n\
         }"
    ));
}

/// `MIN`/`MAX` are defined via the `ORDER BY` order (§18.6.1.5/.6), so they must
/// see the same composite ordering — the seam that would silently rot if the
/// aggregates ever grew their own comparator.
#[test]
fn min_and_max_agree_with_the_composite_order() {
    let engine = NativeSparqlEngine::new();
    let query = "\
PREFIX cdt: <http://w3id.org/awslabs/neptune/SPARQL-CDTs/>
SELECT (MIN(?v) AS ?lo) (MAX(?v) AS ?hi) WHERE {
  VALUES ?v {
    \"[ 2]\"^^cdt:List
    \"[1]\"^^cdt:List
    \"[3]\"^^cdt:List
  }
}";
    let result = engine
        .query(
            &empty_dataset(),
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
        )
        .expect("the query evaluates");
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected solutions")
    };
    assert_eq!(rows.len(), 1);
    let lexical = |cell: &Option<TermValue>| match cell {
        Some(TermValue::Literal { lexical_form, .. }) => lexical_form.clone(),
        other => panic!("expected a literal, got {other:?}"),
    };
    assert_eq!(lexical(&rows[0][0]), "[1]", "MIN is the ORDER BY minimum");
    assert_eq!(lexical(&rows[0][1]), "[3]", "MAX is the ORDER BY maximum");
}
