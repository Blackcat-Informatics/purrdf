// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Frozen numeric and row-loop fixtures shared by cost lanes and full-answer guards.

use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SolutionRow, SparqlEngine, SparqlRequest,
    SparqlResult, TermValue,
};
use purrdf_sparql_eval::{
    CancellationFlag, GovernedOutcome, NativeSparqlEngine, QueryGovernors, QueryOptions,
};
use purrdf_xsd::bigint::BigInt;
use purrdf_xsd::datatype::{XSD_DECIMAL, XSD_INTEGER};

/// The fixtures' caller-selected namespace.
pub const EX: &str = "https://example.org/";
/// Numeric input rows.
pub const NUMERIC_ROWS: usize = 20_000;
/// Exact SUM groups.
pub const GROUPS: usize = 5_000;
/// Row-loop input rows, beyond the evaluator's parallel threshold.
pub const LOOP_ROWS: usize = 16_384;
/// The original exact grouped expression.
pub const GROUPED: &str = "PREFIX ex: <https://example.org/> SELECT ?g (SUM(?b * ?b) AS ?t) \
                       WHERE { ?s ex:b ?b ; ex:g ?g } GROUP BY ?g";

fn big_lexical(n: usize) -> String {
    let i = (n * 7919) % 1_000_000;
    format!("{}{:020}", 10_000_000_000_000_000_000_u128 + n as u128, i)
}

/// Per row `n`: an integer, a four-place decimal, a forty-digit integer and a group.
pub fn numeric_dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p_i = b.intern_iri(&format!("{EX}i"));
    let p_d = b.intern_iri(&format!("{EX}d"));
    let p_b = b.intern_iri(&format!("{EX}b"));
    let p_g = b.intern_iri(&format!("{EX}g"));
    let groups: Vec<_> = (0..GROUPS)
        .map(|g| b.intern_iri(&format!("{EX}group{g}")))
        .collect();
    for n in 0..NUMERIC_ROWS {
        let s = b.intern_iri(&format!("{EX}row{n}"));
        let i = (n * 7919) % 1_000_000;
        let int = b.intern_literal(RdfLiteral::typed(i.to_string(), XSD_INTEGER));
        b.push_quad(s, p_i, int, None);
        let dec = b.intern_literal(RdfLiteral::typed(
            format!("{}.{:04}", i % 1000, (n * 31) % 10_000),
            XSD_DECIMAL,
        ));
        b.push_quad(s, p_d, dec, None);
        let big = b.intern_literal(RdfLiteral::typed(big_lexical(n), XSD_INTEGER));
        b.push_quad(s, p_b, big, None);
        b.push_quad(s, p_g, groups[n % GROUPS], None);
    }
    b.freeze().expect("freeze numeric dataset")
}

/// Per subject `i`: its integer modulo 100 and one three-element composite list.
pub fn loop_dataset(rows: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let v = builder.intern_iri(&format!("{EX}v"));
    let list = builder.intern_iri(&format!("{EX}list"));
    let three = builder.intern_literal(RdfLiteral::typed("[1,2,3]", purrdf_cdt::CDT_LIST));
    for index in 0..rows {
        let subject = builder.intern_iri(&format!("{EX}s{index}"));
        let value =
            builder.intern_literal(RdfLiteral::typed((index % 100).to_string(), XSD_INTEGER));
        builder.push_quad(subject, v, value, None);
        builder.push_quad(subject, list, three, None);
    }
    builder.freeze().expect("freeze the row-loop dataset")
}

/// The original `(shape, public prepared query text, expected row count)` lanes.
pub const LOOP_QUERIES: &[(&str, &str, usize)] = &[
    (
        "filter",
        "SELECT ?s WHERE { ?s <https://example.org/v> ?v FILTER(?v >= 10) }",
        LOOP_ROWS - 10 * LOOP_ROWS.div_ceil(100),
    ),
    (
        "bind",
        "SELECT ?s ?x WHERE { ?s <https://example.org/v> ?v BIND(?v * 2 + 1 AS ?x) }",
        LOOP_ROWS,
    ),
    (
        "unfold",
        "SELECT ?s ?e WHERE { ?s <https://example.org/list> ?l UNFOLD(?l AS ?e) }",
        3 * LOOP_ROWS,
    ),
];

/// Original row governors; each configured lane owns a never-cancelled signal.
pub fn loop_governors() -> [(&'static str, Option<QueryGovernors>); 3] {
    let stop = || Arc::new(CancellationFlag::new()) as Arc<dyn purrdf_sparql_eval::StopSignal>;
    [
        ("ungoverned", None),
        (
            "stop_signal",
            Some(QueryGovernors::UNBOUNDED.with_stop_signal(stop())),
        ),
        (
            "fuel_and_stop_signal",
            Some(
                QueryGovernors::UNBOUNDED
                    .with_fuel(u64::MAX - 1)
                    .with_stop_signal(stop()),
            ),
        ),
    ]
}

/// All original group cells, canonicalized only outside measurement.
pub fn grouped_answer(
    engine: &NativeSparqlEngine,
    ds: &Arc<RdfDataset>,
    governors: Option<&QueryGovernors>,
) -> Vec<SolutionRow> {
    let request = SparqlRequest {
        query: GROUPED,
        base_iri: None,
        substitutions: &[],
    };
    let result = match governors {
        None => engine.query(ds, request).expect("plain group query"),
        Some(governors) => {
            let outcome = engine
                .query_governed(ds, request, QueryOptions::EMPTY, governors)
                .expect("governed group query");
            let GovernedOutcome::Complete { result, .. } = outcome else {
                panic!("the grouped fold must complete: {outcome:?}");
            };
            result
        }
    };
    AnswerGuard::grouped().check(&result);
    let SparqlResult::Solutions { mut rows, .. } = result else {
        unreachable!()
    };
    rows.sort_by_cached_key(|row| format!("{row:?}"));
    rows
}

/// A complete typed bag oracle built outside the query measurement window.
#[derive(Debug)]
pub struct AnswerGuard {
    variables: Vec<String>,
    rows: Vec<SolutionRow>,
}

impl AnswerGuard {
    /// Every forty-digit square summed into its actual group with the shared BigInt home.
    pub fn grouped() -> Self {
        let mut totals = vec![BigInt::zero(); GROUPS];
        for n in 0..NUMERIC_ROWS {
            let value = BigInt::from_digits(&big_lexical(n)).expect("fixture integer");
            totals[n % GROUPS].add_assign(&value.mul(&value));
        }
        let rows = totals
            .iter()
            .enumerate()
            .map(|(group, sum)| {
                vec![
                    Some(TermValue::iri(format!("{EX}group{group}"))),
                    Some(TermValue::typed_literal(
                        sum.to_decimal_string(),
                        XSD_INTEGER,
                    )),
                ]
            })
            .collect();
        Self::canonical(vec!["g".to_owned(), "t".to_owned()], rows)
    }

    /// Every original row subject and BIND value, including datatype and bag multiplicity.
    pub fn row_loop(shape: &str) -> Self {
        assert!(
            matches!(shape, "filter" | "bind"),
            "unsupported count shape {shape}"
        );
        let mut rows = Vec::new();
        for n in 0..LOOP_ROWS {
            if shape == "filter" && n % 100 < 10 {
                continue;
            }
            let mut row = vec![Some(TermValue::iri(format!("{EX}s{n}")))];
            if shape == "bind" {
                row.push(Some(TermValue::typed_literal(
                    (2 * (n % 100) + 1).to_string(),
                    XSD_INTEGER,
                )));
            }
            rows.push(row);
        }
        let variables = if shape == "filter" {
            vec!["s".to_owned()]
        } else {
            vec!["s".to_owned(), "x".to_owned()]
        };
        Self::canonical(variables, rows)
    }

    fn canonical(variables: Vec<String>, mut rows: Vec<SolutionRow>) -> Self {
        rows.sort_by_cached_key(|row| format!("{row:?}"));
        Self { variables, rows }
    }

    /// Assert the full typed bag, after counters and allocation windows close.
    pub fn check(&self, result: &SparqlResult) {
        let SparqlResult::Solutions {
            variables, rows, ..
        } = result
        else {
            panic!("expected a full solution bag");
        };
        assert_eq!(variables, &self.variables);
        let mut actual = rows.clone();
        actual.sort_by_cached_key(|row| format!("{row:?}"));
        assert_eq!(
            actual, self.rows,
            "every typed result cell and multiplicity"
        );
    }

    /// Frozen typed-answer identity, shared by all compiled source projections.
    pub fn signature(&self) -> String {
        let hash =
            purrdf_hash::blake3::hash(format!("{:?}:{:?}", self.variables, self.rows).as_bytes());
        purrdf_hash::hex::encode(hash.as_bytes())
    }
}
