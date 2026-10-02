// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Pure positive join normalization connects UNION arms without enlarging budgets.

mod support;

use std::{fmt::Write, sync::Arc};

use purrdf_core::{
    BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, ResourceDimension, SparqlRequest,
    SparqlResult, TermValue, TrippedGovernor,
};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryGovernors, QueryOptions, ShaclPrebinding};
use support::{render_cell, row, sorted_rows};

const EX: &str = "http://example.org/";
const COUNT: usize = 64;
const CELLS: u64 = 4_096;

fn controls() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let predicates = [
        "node",
        "controlsA",
        "controlsB",
        "mode",
        "enabled",
        "target",
        "connected",
    ]
    .map(|name| b.intern_iri(&format!("{EX}{name}")));
    let voltage = b.intern_iri(&format!("{EX}Voltage"));
    let yes = b.intern_literal(RdfLiteral::typed(
        "true".to_owned(),
        purrdf_xsd::datatype::XSD_BOOLEAN.to_owned(),
    ));
    for i in 0..COUNT {
        let terminal = b.intern_iri(&format!("{EX}terminal{i}"));
        let control = b.intern_iri(&format!("{EX}control{i}"));
        let subject = if i == 0 {
            b.intern_blank("s0", BlankScope::DEFAULT)
        } else {
            b.intern_iri(&format!("{EX}s{i}"))
        };
        b.push_quad(terminal, predicates[0], subject, None);
        b.push_quad(control, predicates[1], terminal, None);
        b.push_quad(control, predicates[2], terminal, None);
        b.push_quad(control, predicates[3], voltage, None);
        b.push_quad(control, predicates[4], yes, None);
        b.push_quad(terminal, predicates[6], yes, None);
        for suffix in ["a", "b"] {
            let target = b.intern_iri(&format!("{EX}target{i}{suffix}"));
            b.push_quad(control, predicates[5], target, None);
        }
    }
    b.freeze().expect("control fixture freezes")
}

#[test]
fn union_and_connected_predicates_are_position_independent_under_the_same_ceiling() {
    let engine = NativeSparqlEngine::new();
    let dataset = controls();
    let governors = QueryGovernors::METERED.with_max_intermediate_cells(CELLS);
    for focus in [
        None,
        Some(TermValue::Iri(format!("{EX}s1"))),
        Some(TermValue::Blank {
            label: "s0".to_owned(),
            scope: BlankScope::DEFAULT,
        }),
    ] {
        let substitutions = focus
            .iter()
            .map(|value| ("this".to_owned(), value.clone()))
            .collect::<Vec<_>>();
        let mut expected = if let Some(focus) = &focus {
            vec![row(&[("this", &render_cell(Some(focus))), ("n", "2")])]
        } else {
            (0..COUNT)
                .map(|index| {
                    let subject = if index == 0 {
                        "_:s0".to_owned()
                    } else {
                        format!("<{EX}s{index}>")
                    };
                    row(&[("this", &subject), ("n", "2")])
                })
                .collect()
        };
        expected.sort();
        let options = if focus.is_some() {
            vec![
                QueryOptions::EMPTY,
                QueryOptions::EMPTY.with_prebinding(ShaclPrebinding::Applied),
            ]
        } else {
            vec![QueryOptions::EMPTY]
        };
        for options in options {
            for (before, after) in [
                ("", "?terminal ex:connected true ."),
                ("?terminal ex:connected true .", ""),
            ] {
                let text = format!(
                    "PREFIX ex: <{EX}> SELECT $this (COUNT(DISTINCT ?target) AS ?n) WHERE {{ \
             ?terminal ex:node $this . {before} \
             {{ ?control ex:controlsA ?terminal }} UNION {{ ?control ex:controlsB ?terminal }} \
             ?control ex:mode ex:Voltage ; ex:enabled true ; ex:target ?target . {after} \
             }} GROUP BY $this HAVING (COUNT(DISTINCT ?target) > 1)"
                );
                let outcome = engine
                    .query_governed(
                        &dataset,
                        SparqlRequest {
                            query: &text,
                            base_iri: None,
                            substitutions: &substitutions,
                        },
                        options,
                        &governors,
                    )
                    .expect("the query has an outcome");
                assert!(outcome.is_complete(), "{text}: {outcome:?}");
                assert!(
                    outcome
                        .evidence()
                        .consumed_in(ResourceDimension::IntermediateCells)
                        <= CELLS
                );
                let result = outcome.into_complete().expect("complete");
                let SparqlResult::Solutions {
                    variables, rows, ..
                } = &result
                else {
                    panic!("COUNT returns solutions");
                };
                assert_eq!(variables, &["this", "n"]);
                for cells in rows {
                    assert!(
                        matches!(&cells[1], Some(TermValue::Literal { lexical_form, datatype, .. })
                        if lexical_form == "2" && datatype == purrdf_xsd::datatype::XSD_INTEGER)
                    );
                }
                assert_eq!(sorted_rows(&result, render_cell), expected);
            }
        }
    }
}

#[test]
fn a_genuinely_disconnected_product_is_still_refused_before_work() {
    let dataset = controls();
    let text = format!(
        "PREFIX ex: <{EX}> SELECT * WHERE {{ ?terminal ex:connected true . ?control ex:target ?target }}"
    );
    let governors = QueryGovernors::METERED.with_max_intermediate_cells(CELLS);
    let outcome = NativeSparqlEngine::new()
        .query_governed(
            &dataset,
            SparqlRequest {
                query: &text,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
            &governors,
        )
        .expect("the query has an outcome");
    assert!(matches!(outcome.exhausted().expect("refusal").tripped,
        TrippedGovernor::Refused { dimension: ResourceDimension::IntermediateCells, limit: CELLS, estimate } if estimate > CELLS));
    for dimension in ResourceDimension::ALL {
        assert_eq!(outcome.evidence().consumed_in(dimension), 0);
    }
}

#[test]
fn independent_union_factors_run_once_and_preserve_cartesian_bags_and_cuts() {
    use purrdf_sparql_eval::{ChargePoint, PartialAnswers};

    let mut b = RdfDatasetBuilder::new();
    let edge = b.intern_iri(&format!("{EX}edge"));
    let attr = b.intern_iri(&format!("{EX}attr"));
    for index in 0..3 {
        let s = b.intern_iri(&format!("{EX}s{index}"));
        let o = b.intern_iri(&format!("{EX}o{index}"));
        b.push_quad(s, edge, o, None);
    }
    for index in 0..5 {
        let x = b.intern_iri(&format!("{EX}x{index}"));
        let value = b.intern_iri(&format!("{EX}v{index}"));
        b.push_quad(x, attr, value, None);
    }
    let dataset = b.freeze().expect("independent factor fixture freezes");
    let query = format!(
        "PREFIX ex: <{EX}> SELECT ?s ?o ?x ?v WHERE {{ {{ ?s ex:edge ?o }} UNION {{ ?s ex:edge ?o }} ?x ex:attr ?v }}"
    );
    let engine = NativeSparqlEngine::new();
    let mut expected = Vec::new();
    for edge_index in 0..3 {
        for attr_index in 0..5 {
            let answer = row(&[
                ("s", &format!("<{EX}s{edge_index}>")),
                ("o", &format!("<{EX}o{edge_index}>")),
                ("x", &format!("<{EX}x{attr_index}>")),
                ("v", &format!("<{EX}v{attr_index}>")),
            ]);
            expected.extend([answer.clone(), answer]);
        }
    }
    expected.sort();
    let full = engine
        .query_governed(
            &dataset,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
            &QueryGovernors::METERED.with_max_intermediate_cells(120),
        )
        .expect("exact capacity outcome");
    let fuel = full.evidence().consumed_in(ResourceDimension::Fuel);
    assert_eq!(
        sorted_rows(
            &full.into_complete().expect("complete product"),
            render_cell
        ),
        expected
    );
    let receipt = engine
        .explain_query(&dataset, &query, None)
        .expect("independent factors have a receipt");
    assert_eq!(
        receipt
            .ledger()
            .iter()
            .map(|node| node.fuel_at(ChargePoint::BgpCandidateQuad))
            .sum::<u64>(),
        11,
        "each UNION arm scans three edges; the independent BGP scans five attributes once"
    );
    let refused = engine
        .query_governed(
            &dataset,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
            &QueryGovernors::METERED.with_max_intermediate_cells(119),
        )
        .expect("below-capacity outcome");
    assert!(matches!(
        refused.exhausted().expect("product refusal").tripped,
        TrippedGovernor::Refused {
            dimension: ResourceDimension::IntermediateCells,
            limit: 119,
            estimate: 120,
        }
    ));
    for dimension in ResourceDimension::ALL {
        assert_eq!(refused.evidence().consumed_in(dimension), 0);
    }
    for ceiling in 0..=fuel {
        let outcome = engine
            .query_governed(
                &dataset,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &QueryGovernors::UNBOUNDED.with_fuel(ceiling),
            )
            .expect("independent product cut outcome");
        let rows = if let Some(exhausted) = outcome.exhausted() {
            let PartialAnswers::Certain(partial) = &exhausted.partial else {
                panic!("a positive product cut is a lower bound: {outcome:?}");
            };
            sorted_rows(partial.result(), render_cell)
        } else {
            sorted_rows(&outcome.into_complete().expect("complete cut"), render_cell)
        };
        let mut available = expected.clone();
        for answer in rows {
            let at = available
                .iter()
                .position(|candidate| *candidate == answer)
                .expect("a certified product row retains complete bindings and multiplicity");
            available.remove(at);
        }
    }
    let units =
        format!("PREFIX ex: <{EX}> SELECT ?x ?v WHERE {{ {{ }} UNION {{ }} ?x ex:attr ?v }}");
    let result = engine
        .query_governed(
            &dataset,
            SparqlRequest {
                query: &units,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
        .expect("unit arms outcome")
        .into_complete()
        .expect("unit arms complete");
    assert_eq!(sorted_rows(&result, render_cell).len(), 10);
    let receipt = engine
        .explain_query(&dataset, &units, None)
        .expect("unit arms receipt");
    assert_eq!(
        receipt
            .ledger()
            .iter()
            .map(|node| node.fuel_at(ChargePoint::BgpCandidateQuad))
            .sum::<u64>(),
        5,
        "zero-column duplicate unit rows multiply answers without repeating the independent scan"
    );
}

#[test]
fn retained_unit_order_preserves_the_structural_override_and_ordinary_cache_path() {
    use purrdf_sparql_algebra::{SparqlParser, Variable};
    use purrdf_sparql_eval::{
        EvalCtx, EvalOptions, Outcome, Solution, SolutionTerm, evaluate_query,
        governor::GovernorState,
    };

    let mut builder = RdfDatasetBuilder::new();
    let common = builder.intern_iri(&format!("{EX}common"));
    let rare = builder.intern_iri(&format!("{EX}rare"));
    let mut chosen = None;
    for index in 0..8 {
        let subject = builder.intern_iri(&format!("{EX}s{index}"));
        let object = builder.intern_iri(&format!("{EX}o{index}"));
        builder.push_quad(subject, common, object, None);
        if index == 0 {
            builder.push_quad(subject, rare, object, None);
            chosen = Some((subject, object));
        }
    }
    let dataset = builder.freeze().expect("order override fixture freezes");
    let (subject, object) = chosen.expect("known matching pair");
    let query = SparqlParser::new()
        .parse_query(&format!(
            "PREFIX ex: <{EX}> SELECT ?s ?o WHERE {{ ?s ex:common ?o ; ex:rare ?o . {{ }} UNION {{ }} }}"
        ))
        .expect("positive order query");
    let mut consumption = Vec::new();
    for force_structural_bgp_order in [false, true] {
        let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
        let mut ctx = EvalCtx::new(&dataset)
            .with_governors(Arc::clone(&state))
            .with_eval_options(EvalOptions {
                force_structural_bgp_order,
                ..EvalOptions::default()
            });
        let Outcome::Solutions(rows) = evaluate_query(&query, &mut ctx).expect("order execution")
        else {
            unreachable!()
        };
        assert_eq!(
            rows.schema.vars(),
            &[Variable::new("s"), Variable::new("o")]
        );
        let expected: Solution = [
            Some(SolutionTerm::Existing(subject)),
            Some(SolutionTerm::Existing(object)),
        ]
        .into_iter()
        .collect();
        assert_eq!(rows.rows, vec![expected; 2]);
        consumption.push(state.evidence().consumed_in(ResourceDimension::Fuel));
    }
    assert!(
        consumption[1] > consumption[0],
        "the structural override must execute its broader first pattern, not the retained rare-first order"
    );
    let engine = NativeSparqlEngine::new();
    let ordinary =
        format!("PREFIX ex: <{EX}> SELECT ?s ?o WHERE {{ ?s ex:common ?o ; ex:rare ?o }}");
    let mut bags = Vec::new();
    for _ in 0..2 {
        let result = engine
            .query_governed(
                &dataset,
                SparqlRequest {
                    query: &ordinary,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &QueryGovernors::METERED,
            )
            .expect("ordinary outcome")
            .into_complete()
            .expect("ordinary completion");
        bags.push(sorted_rows(&result, render_cell));
    }
    assert_eq!(bags[0], bags[1]);
    assert_eq!(bags[0].len(), 1);
    assert_eq!(engine.order_cache_stats().entries, 1);
    assert!(engine.order_cache_stats().hits >= 1);
}

#[test]
fn tiny_union_drives_large_attributes_without_changing_the_ceiling() {
    const POPULATION: usize = 4096;
    let mut b = RdfDatasetBuilder::new();
    let left_p = b.intern_iri(&format!("{EX}left"));
    let right_p = b.intern_iri(&format!("{EX}right"));
    let a = b.intern_iri(&format!("{EX}a"));
    let bb = b.intern_iri(&format!("{EX}b"));
    for i in 0..POPULATION {
        let left = b.intern_iri(&format!("{EX}left{i}"));
        let right = b.intern_iri(&format!("{EX}right{i}"));
        let value = b.intern_iri(&format!("{EX}value{i}"));
        b.push_quad(left, left_p, value, None);
        b.push_quad(right, right_p, value, None);
        if i < 2 {
            b.push_quad(left, if i == 0 { a } else { bb }, right, None);
        }
    }
    let dataset = b.freeze().expect("tiny connector fixture freezes");
    let ordinary =
        "?left ex:left ?lv . { ?left ex:a ?right } UNION { ?left ex:b ?right } ?right ex:right ?rv";
    let distributed = "{ ?left ex:a ?right . ?left ex:left ?lv . ?right ex:right ?rv } UNION { ?left ex:b ?right . ?left ex:left ?lv . ?right ex:right ?rv }";
    let governors = QueryGovernors::METERED.with_max_intermediate_cells(CELLS);
    let engine = NativeSparqlEngine::new();
    let mut outcomes = Vec::new();
    for body in [ordinary, distributed] {
        let text = format!("PREFIX ex: <{EX}> SELECT ?left ?right ?lv ?rv WHERE {{ {body} }}");
        outcomes.push(
            engine
                .query_governed(
                    &dataset,
                    SparqlRequest {
                        query: &text,
                        base_iri: None,
                        substitutions: &[],
                    },
                    QueryOptions::EMPTY,
                    &governors,
                )
                .expect("an outcome"),
        );
    }
    let bags: Vec<_> = outcomes
        .into_iter()
        .map(|outcome| {
            assert!(
                outcome.is_complete(),
                "same-budget selective query: {outcome:?}"
            );
            sorted_rows(&outcome.into_complete().expect("complete"), render_cell)
        })
        .collect();
    assert_eq!(bags[0].len(), 2);
    assert_eq!(bags[0], bags[1]);
}

#[test]
fn tiny_attributes_drive_large_union_without_changing_the_ceiling() {
    let mut b = RdfDatasetBuilder::new();
    let left_p = b.intern_iri(&format!("{EX}left"));
    let right_p = b.intern_iri(&format!("{EX}right"));
    let a = b.intern_iri(&format!("{EX}a"));
    let bb = b.intern_iri(&format!("{EX}b"));
    for i in 0..4096 {
        let left = b.intern_iri(&format!("{EX}left{i}"));
        let right = b.intern_iri(&format!("{EX}right{i}"));
        let value = b.intern_iri(&format!("{EX}value{i}"));
        b.push_quad(left, if i % 2 == 0 { a } else { bb }, right, None);
        if i < 2 {
            b.push_quad(left, left_p, value, None);
            b.push_quad(right, right_p, value, None);
        }
    }
    let dataset = b.freeze().expect("mirror freezes");
    let engine = NativeSparqlEngine::new();
    let governors = QueryGovernors::METERED.with_max_intermediate_cells(CELLS);
    let original =
        "?left ex:left ?lv . { ?left ex:a ?right } UNION { ?left ex:b ?right } ?right ex:right ?rv";
    let distributed = "{ ?left ex:a ?right . ?left ex:left ?lv . ?right ex:right ?rv } UNION { ?left ex:b ?right . ?left ex:left ?lv . ?right ex:right ?rv }";
    let mut bags = Vec::new();
    for body in [original, distributed] {
        let text = format!("PREFIX ex: <{EX}> SELECT ?left ?right ?lv ?rv WHERE {{ {body} }}");
        let outcome = engine
            .query_governed(
                &dataset,
                SparqlRequest {
                    query: &text,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &governors,
            )
            .expect("mirror outcome");
        assert!(
            outcome.is_complete(),
            "mirrored same-budget query: {outcome:?}"
        );
        bags.push(sorted_rows(
            &outcome.into_complete().expect("complete"),
            render_cell,
        ));
    }
    assert_eq!(bags[0].len(), 2);
    assert_eq!(bags[0], bags[1]);
}

#[test]
fn selective_attribute_components_connect_before_their_product() {
    let mut builder = RdfDatasetBuilder::new();
    let predicates =
        ["left", "right", "a", "b"].map(|name| builder.intern_iri(&format!("{EX}{name}")));
    for index in 0..10_000 {
        let left = builder.intern_iri(&format!("{EX}left{index}"));
        let right = builder.intern_iri(&format!("{EX}right{index}"));
        let value = builder.intern_iri(&format!("{EX}value{index}"));
        builder.push_quad(left, predicates[2 + index % 2], right, None);
        if index < 100 {
            builder.push_quad(left, predicates[0], value, None);
            builder.push_quad(right, predicates[1], value, None);
        }
    }
    let data = builder.freeze().expect("selective components fixture");
    let bodies = [
        "{ ?left ex:a ?right . ?left ex:left ?lv . ?right ex:right ?rv } UNION { ?left ex:b ?right . ?left ex:left ?lv . ?right ex:right ?rv }",
        "?left ex:left ?lv . { ?left ex:a ?right } UNION { ?left ex:b ?right } ?right ex:right ?rv",
        "?right ex:right ?rv . { ?left ex:a ?right } UNION { ?left ex:b ?right } ?left ex:left ?lv",
    ];
    let engine = NativeSparqlEngine::new();
    let mut bags = Vec::new();
    for body in bodies {
        let text = format!("PREFIX ex: <{EX}> SELECT ?left ?right ?lv ?rv WHERE {{ {body} }}");
        let outcome = engine
            .query_governed(
                &data,
                SparqlRequest {
                    query: &text,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &QueryGovernors::METERED.with_max_intermediate_cells(CELLS),
            )
            .expect("selective components outcome");
        assert!(
            outcome.is_complete(),
            "selective connected factors must fit the unchanged ceiling: {outcome:?}"
        );
        bags.push(sorted_rows(
            &outcome.into_complete().expect("selective completion"),
            render_cell,
        ));
        assert_eq!(bags.last().expect("a measured bag").len(), 100);
    }
    assert_eq!(bags[0].len(), 100);
    assert_eq!(bags[0], bags[1]);
    assert_eq!(bags[0], bags[2]);
}

#[test]
fn heterogeneous_union_bindings_choose_native_probes_per_row() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&format!("{EX}s"));
    let left = b.intern_iri(&format!("{EX}left"));
    let right = b.intern_iri(&format!("{EX}right"));
    let attr = b.intern_iri(&format!("{EX}attr"));
    let tag = b.intern_iri(&format!("{EX}tag"));
    for i in 0..3 {
        let object = b.intern_iri(&format!("{EX}o{i}"));
        let value = b.intern_iri(&format!("{EX}v{i}"));
        b.push_quad(object, attr, value, None);
        if i == 0 {
            b.push_quad(s, left, object, None);
        }
    }
    b.push_quad(s, right, tag, None);
    let dataset = b.freeze().expect("mask fixture freezes");
    let ordinary = "{ ?s ex:left ?o } UNION { ?s ex:right ?tag } ?o ex:attr ?value";
    let control =
        "{ ?s ex:left ?o . ?o ex:attr ?value } UNION { ?s ex:right ?tag . ?o ex:attr ?value }";
    let engine = NativeSparqlEngine::new();
    let mut bags = Vec::new();
    for body in [ordinary, control] {
        let query = format!("PREFIX ex: <{EX}> SELECT ?s ?o ?tag ?value WHERE {{ {body} }}");
        let result = engine
            .query_governed(
                &dataset,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &QueryGovernors::METERED.with_max_intermediate_cells(CELLS),
            )
            .expect("mask outcome")
            .into_complete()
            .expect("mask complete");
        bags.push(sorted_rows(&result, render_cell));
    }
    assert_eq!(bags[0].len(), 4);
    assert_eq!(bags[0], bags[1]);
}

#[test]
fn all_eight_spo_masks_preserve_union_bags() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&format!("{EX}s"));
    let p = b.intern_iri(&format!("{EX}p"));
    let o = b.intern_iri(&format!("{EX}o"));
    b.push_quad(s, p, o, None);
    let marker =
        ["subject", "predicate", "object"].map(|name| b.intern_iri(&format!("{EX}{name}")));
    let present = b.intern_iri(&format!("{EX}present"));
    let zero = b.intern_iri(&format!("{EX}zero"));
    let mut arms = Vec::new();
    for mask in 0..8 {
        let arm = b.intern_iri(&format!("{EX}arm{mask}"));
        let mut patterns = Vec::new();
        for (axis, (name, value)) in ["s", "p", "o"].into_iter().zip([s, p, o]).enumerate() {
            if mask & (1 << axis) != 0 {
                b.push_quad(arm, marker[axis], value, None);
                patterns.push(format!(
                    "ex:arm{mask} ex:{} ?{name} .",
                    ["subject", "predicate", "object"][axis]
                ));
            }
        }
        if patterns.is_empty() {
            b.push_quad(arm, zero, present, None);
            patterns.push(format!("ex:arm{mask} ex:zero ex:present ."));
        }
        arms.push(patterns.join(" "));
    }
    let dataset = b.freeze().expect("all masks freeze");
    let union = arms
        .iter()
        .map(|arm| format!("{{ {arm} }}"))
        .collect::<Vec<_>>()
        .join(" UNION ");
    let distributed = arms
        .iter()
        .map(|arm| format!("{{ {arm} ?s ?p ?o }}"))
        .collect::<Vec<_>>()
        .join(" UNION ");
    let mut bags = Vec::new();
    for body in [format!("{union} ?s ?p ?o"), distributed] {
        let query = format!("PREFIX ex: <{EX}> SELECT ?s ?p ?o WHERE {{ {body} }}");
        let result = NativeSparqlEngine::new()
            .query_governed(
                &dataset,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &QueryGovernors::METERED.with_max_intermediate_cells(CELLS),
            )
            .expect("all mask outcome")
            .into_complete()
            .expect("all mask complete");
        bags.push(sorted_rows(&result, render_cell));
    }
    assert_eq!(bags[0], bags[1]);
    assert!(
        bags[0].len() >= 8,
        "duplicate compatible witnesses remain in the bag"
    );
}

#[test]
fn four_arm_connector_keeps_twenty_ambient_attributes_connected() {
    let mut b = RdfDatasetBuilder::new();
    let edges = ["a", "b", "c", "d"].map(|name| b.intern_iri(&format!("{EX}{name}")));
    let left = (0..10)
        .map(|i| b.intern_iri(&format!("{EX}left{i}")))
        .collect::<Vec<_>>();
    let right = (0..10)
        .map(|i| b.intern_iri(&format!("{EX}right{i}")))
        .collect::<Vec<_>>();
    for i in 0..16 {
        let l = b.intern_iri(&format!("{EX}l{i}"));
        let r = b.intern_iri(&format!("{EX}r{i}"));
        let value = b.intern_iri(&format!("{EX}v{i}"));
        for (&lp, &rp) in left.iter().zip(&right) {
            b.push_quad(l, lp, value, None);
            b.push_quad(r, rp, value, None);
        }
        for &edge in &edges {
            b.push_quad(l, edge, r, None);
        }
    }
    let dataset = b.freeze().expect("wide fixture freezes");
    let mut l = String::new();
    let mut r = String::new();
    let mut header = String::new();
    for i in 0..10 {
        write!(l, "?l ex:left{i} ?lv{i} . ").expect("String writes are infallible");
        write!(r, "?r ex:right{i} ?rv{i} . ").expect("String writes are infallible");
        write!(header, "?lv{i} ?rv{i} ").expect("String writes are infallible");
    }
    let union = ["a", "b", "c", "d"]
        .map(|edge| format!("{{ ?l ex:{edge} ?r }}"))
        .join(" UNION ");
    let control = ["a", "b", "c", "d"]
        .map(|edge| format!("{{ ?l ex:{edge} ?r . {l} {r} }}"))
        .join(" UNION ");
    let mut bags = Vec::new();
    for body in [
        format!("{l} {union} {r}"),
        format!("{r} {union} {l}"),
        control,
    ] {
        let query = format!("PREFIX ex: <{EX}> SELECT ?l ?r {header} WHERE {{ {body} }}");
        let outcome = NativeSparqlEngine::new()
            .query_governed(
                &dataset,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &QueryGovernors::METERED.with_max_intermediate_cells(CELLS),
            )
            .expect("wide outcome");
        assert!(outcome.is_complete(), "wide connected query: {outcome:?}");
        bags.push(sorted_rows(
            &outcome.into_complete().expect("complete"),
            render_cell,
        ));
    }
    assert_eq!(bags[0].len(), 64);
    assert_eq!(bags[0], bags[1]);
    assert_eq!(bags[0], bags[2]);
}

#[test]
fn exact_report_having_alias_keeps_its_normative_empty_bag() {
    let dataset = controls();
    let mut bags = Vec::new();
    for (before, after) in [
        ("", "?terminal ex:connected true ."),
        ("?terminal ex:connected true .", ""),
    ] {
        let query = format!(
            "PREFIX ex: <{EX}> SELECT $this (COUNT(DISTINCT ?target) AS ?n) WHERE {{ ?terminal ex:node $this . {before} {{ ?control ex:controlsA ?terminal }} UNION {{ ?control ex:controlsB ?terminal }} ?control ex:mode ex:Voltage ; ex:enabled true ; ex:target ?target . {after} }} GROUP BY $this HAVING (?n > 1)"
        );
        let outcome = NativeSparqlEngine::new()
            .query_governed(
                &dataset,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &QueryGovernors::METERED.with_max_intermediate_cells(CELLS),
            )
            .expect("literal report outcome");
        assert!(
            outcome.is_complete(),
            "literal report admission: {outcome:?}"
        );
        bags.push(sorted_rows(
            &outcome.into_complete().expect("complete"),
            render_cell,
        ));
    }
    assert_eq!(bags[0], Vec::new());
    assert_eq!(bags[0], bags[1]);
}

#[test]
fn every_fuel_cut_is_a_complete_binding_lower_bound() {
    use purrdf_sparql_eval::PartialAnswers;
    let mut b = RdfDatasetBuilder::new();
    let predicates =
        ["left", "extra", "right", "a", "b"].map(|name| b.intern_iri(&format!("{EX}{name}")));
    for i in 0..32 {
        let l = b.intern_iri(&format!("{EX}l{i}"));
        let r = b.intern_iri(&format!("{EX}r{i}"));
        let value = b.intern_iri(&format!("{EX}v{i}"));
        b.push_quad(l, predicates[0], value, None);
        b.push_quad(l, predicates[1], value, None);
        b.push_quad(r, predicates[2], value, None);
        if i < 2 {
            b.push_quad(l, predicates[3 + i], r, None);
        }
    }
    let dataset = b.freeze().expect("fuel fixture");
    let query = format!(
        "PREFIX ex: <{EX}> SELECT ?l ?r ?lv ?extra ?rv WHERE {{ ?l ex:left ?lv ; ex:extra ?extra . {{ ?l ex:a ?r }} UNION {{ ?l ex:b ?r }} ?r ex:right ?rv }}"
    );
    let engine = NativeSparqlEngine::new();
    let full = engine
        .query_governed(
            &dataset,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
        .expect("full fuel outcome");
    let fuel = full.evidence().consumed_in(ResourceDimension::Fuel);
    let expected = sorted_rows(&full.into_complete().expect("full complete"), render_cell);
    assert_eq!(expected.len(), 2);
    let mut nonempty_cut = false;
    for ceiling in 0..=fuel {
        let outcome = engine
            .query_governed(
                &dataset,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &QueryGovernors::UNBOUNDED.with_fuel(ceiling),
            )
            .expect("cut outcome");
        let rows = if let Some(exhausted) = outcome.exhausted() {
            let PartialAnswers::Certain(partial) = &exhausted.partial else {
                panic!("a positive cut is a lower bound: {outcome:?}");
            };
            sorted_rows(partial.result(), render_cell)
        } else {
            sorted_rows(
                &outcome.into_complete().expect("completed cut"),
                render_cell,
            )
        };
        nonempty_cut |= !rows.is_empty() && rows.len() < expected.len();
        let mut available = expected.clone();
        for row in rows {
            let at = available
                .iter()
                .position(|candidate| *candidate == row)
                .expect("each certified row is a full query answer with available multiplicity");
            available.remove(at);
        }
    }
    assert!(
        nonempty_cut,
        "the sweep must exercise a final-stage nonempty certified cut"
    );
}

#[test]
fn raw_projectless_select_does_not_restore_hidden_columns() {
    use purrdf_sparql_algebra::{
        GraphPattern, NamedNode, NamedNodePattern, Query, SparqlParser, TermPattern, TriplePattern,
        Variable,
    };
    use purrdf_sparql_eval::{EvalCtx, Outcome, evaluate_query};
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&format!("{EX}s"));
    let p = b.intern_iri(&format!("{EX}p"));
    let o = b.intern_iri(&format!("{EX}o"));
    b.push_quad(s, p, o, None);
    let data = b.freeze().expect("raw hidden fixture");
    let leaf = GraphPattern::Bgp {
        patterns: vec![TriplePattern {
            subject: TermPattern::Variable(Variable::new("s")),
            predicate: NamedNodePattern::NamedNode(
                NamedNode::new(format!("{EX}p")).expect("fixture IRI"),
            ),
            object: TermPattern::Variable(Variable::hidden("raw-witness")),
        }],
    };
    let mut query = SparqlParser::new()
        .parse_query("SELECT * WHERE {}")
        .expect("empty query");
    let Query::Select { pattern, .. } = &mut query else {
        unreachable!()
    };
    *pattern = GraphPattern::union(leaf.clone(), leaf);
    let Outcome::Solutions(rows) =
        evaluate_query(&query, &mut EvalCtx::new(&data)).expect("raw hidden query")
    else {
        unreachable!()
    };
    assert_eq!(rows.schema.vars(), &[Variable::new("s")]);
    assert_eq!(rows.rows.len(), 2);
}

#[test]
fn projectless_raw_local_blank_slots_do_not_reappear_after_a_positive_join() {
    use purrdf_sparql_algebra::{GraphPattern, Query, SparqlParser, Variable};
    use purrdf_sparql_eval::{EvalCtx, Outcome, PreparedQuery, evaluate_query};

    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri(&format!("{EX}subject"));
    let value = builder.intern_iri(&format!("{EX}value"));
    let object = builder.intern_iri(&format!("{EX}object"));
    let p = builder.intern_iri(&format!("{EX}p"));
    let q = builder.intern_iri(&format!("{EX}q"));
    builder.push_quad(subject, p, value, None);
    builder.push_quad(value, q, object, None);
    let data = builder.freeze().expect("raw local blank fixture");
    let mut query = SparqlParser::new().parse_query(&format!(
        "PREFIX ex: <{EX}> SELECT * WHERE {{ {{ _:left ex:p ?value }} UNION {{ _:right ex:p ?value }} ?value ex:q ?object }}"
    )).expect("raw local blank query");
    let Query::Select { pattern, .. } = &mut query else {
        unreachable!()
    };
    let GraphPattern::Project { inner, .. } = std::mem::replace(pattern, GraphPattern::empty_bgp())
    else {
        unreachable!()
    };
    *pattern = inner.into_inner();
    let Outcome::Solutions(rows) =
        evaluate_query(&query, &mut EvalCtx::new(&data)).expect("raw local blank")
    else {
        unreachable!()
    };
    assert_eq!(
        rows.schema.vars(),
        &[Variable::new("value"), Variable::new("object")]
    );
    assert_eq!(rows.rows.len(), 2);
    assert!(
        rows.rows
            .iter()
            .all(|row| row.len() == 2 && row.iter().all(Option::is_some))
    );
    let rewritten = PreparedQuery::rewritten(query.clone(), QueryOptions::EMPTY)
        .expect("rewritten raw blank query");
    let engine = NativeSparqlEngine::new();
    let compiled = engine
        .prepare_algebra(query, QueryOptions::EMPTY)
        .expect("compiled raw blank query");
    for prepared in [&rewritten, &*compiled] {
        let result = engine
            .query_prepared(&data, prepared, &[], QueryOptions::EMPTY)
            .expect("prepared raw blank query");
        let SparqlResult::Solutions {
            variables, rows, ..
        } = result
        else {
            unreachable!()
        };
        assert_eq!(variables, ["value", "object"]);
        assert_eq!(rows.len(), 2);
        assert!(
            rows.iter()
                .all(|row| row.len() == 2 && row.iter().all(Option::is_some))
        );
    }
}

#[test]
fn local_blank_working_width_is_refused_before_work_without_exposing_the_slot() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri(&format!("{EX}subject"));
    let predicate = builder.intern_iri(&format!("{EX}p"));
    let value = builder.intern_iri(&format!("{EX}value"));
    builder.push_quad(subject, predicate, value, None);
    let data = builder.freeze().expect("local blank width fixture");
    let query = format!("PREFIX ex: <{EX}> SELECT ?value WHERE {{ _:local ex:p ?value }}");
    let request = SparqlRequest {
        query: &query,
        base_iri: None,
        substitutions: &[],
    };
    let engine = NativeSparqlEngine::new();
    let refused = engine
        .query_governed(
            &data,
            request,
            QueryOptions::EMPTY,
            &QueryGovernors::METERED.with_max_intermediate_cells(1),
        )
        .expect("blank width refusal");
    assert!(matches!(
        refused.exhausted().expect("blank width refused").tripped,
        TrippedGovernor::Refused {
            dimension: ResourceDimension::IntermediateCells,
            limit: 1,
            estimate: 2
        }
    ));
    for dimension in ResourceDimension::ALL {
        assert_eq!(refused.evidence().consumed_in(dimension), 0);
    }
    let result = engine
        .query_governed(
            &data,
            request,
            QueryOptions::EMPTY,
            &QueryGovernors::METERED.with_max_intermediate_cells(2),
        )
        .expect("blank width admitted")
        .into_complete()
        .expect("blank width completion");
    assert_eq!(
        sorted_rows(&result, render_cell),
        [row(&[("value", &format!("<{EX}value>"))])]
    );
    let SparqlResult::Solutions { variables, .. } = result else {
        unreachable!()
    };
    assert_eq!(variables, ["value"]);
    let empty = RdfDatasetBuilder::new()
        .freeze()
        .expect("empty width fixture");
    let outcome = engine
        .query_governed(
            &empty,
            request,
            QueryOptions::EMPTY,
            &QueryGovernors::METERED.with_max_intermediate_cells(0),
        )
        .expect("empty blank width outcome");
    assert!(outcome.is_complete());
    assert_eq!(
        outcome
            .evidence()
            .consumed_in(ResourceDimension::IntermediateCells),
        0
    );
    assert_eq!(
        sorted_rows(
            &outcome.into_complete().expect("empty blank completion"),
            render_cell
        ),
        Vec::new()
    );
}

#[test]
fn empty_native_seed_does_not_charge_a_unit_row() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri(&format!("{EX}s"));
    let present = builder.intern_iri(&format!("{EX}present"));
    let value = builder.intern_iri(&format!("{EX}value"));
    builder.push_quad(subject, present, value, None);
    let data = builder.freeze().expect("empty seed fixture");
    let query = format!(
        "PREFIX ex: <{EX}> SELECT ?s ?o ?v WHERE {{ {{ ?s ex:absent ?o }} UNION {{ ?s ex:absent ?o }} ?s ex:present ?v }}"
    );
    let engine = NativeSparqlEngine::new();
    for ceiling in [0, 2] {
        let outcome = engine
            .query_governed(
                &data,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &QueryGovernors::METERED.with_max_intermediate_cells(ceiling),
            )
            .expect("empty seed outcome");
        assert!(
            outcome.is_complete(),
            "an empty driver has no live cells: {outcome:?}"
        );
        assert_eq!(
            outcome
                .evidence()
                .consumed_in(ResourceDimension::IntermediateCells),
            0
        );
        let result = outcome.into_complete().expect("empty completion");
        assert_eq!(sorted_rows(&result, render_cell), Vec::new());
    }
    let query = format!("PREFIX ex: <{EX}> SELECT ?s ?v WHERE {{ ?s ex:present ?v }}");
    for ceiling in [0, 1] {
        let outcome = engine
            .query_governed(
                &data,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &QueryGovernors::METERED.with_max_intermediate_cells(ceiling),
            )
            .expect("unit seed outcome");
        assert!(
            matches!(
                outcome.exhausted().expect("unit refusal").tripped,
                TrippedGovernor::Refused {
                    dimension: ResourceDimension::IntermediateCells,
                    ..
                }
            ),
            "the ordinary nonempty unit seed remains governed"
        );
    }
}

#[test]
fn both_reported_alternatives_preserve_focus_and_duplicate_bags() {
    let mut builder = RdfDatasetBuilder::new();
    let predicates = [
        "tableOfA", "tableOfB", "pointOfA", "pointOfB", "low", "high", "step",
    ]
    .map(|name| builder.intern_iri(&format!("{EX}{name}")));
    let numbers = [10, 20, 0].map(|number| {
        builder.intern_literal(RdfLiteral::typed(
            number.to_string(),
            purrdf_xsd::datatype::XSD_INTEGER.to_owned(),
        ))
    });
    for index in 0..COUNT {
        let focus = if index == 0 {
            builder.intern_blank("this0", BlankScope::DEFAULT)
        } else {
            builder.intern_iri(&format!("{EX}this{index}"))
        };
        let changer = builder.intern_iri(&format!("{EX}changer{index}"));
        let point = builder.intern_iri(&format!("{EX}point{index}"));
        for predicate in &predicates[..2] {
            builder.push_quad(changer, *predicate, focus, None);
        }
        for predicate in &predicates[2..4] {
            builder.push_quad(point, *predicate, focus, None);
        }
        builder.push_quad(changer, predicates[4], numbers[0], None);
        builder.push_quad(changer, predicates[5], numbers[1], None);
        builder.push_quad(point, predicates[6], numbers[2], None);
    }
    let data = builder.freeze().expect("two alternative fixture");
    let original = "$this (^ex:tableOfA)|(^ex:tableOfB) ?changer . ?changer ex:low ?low . ?changer ex:high ?high . $this ^ex:pointOfA|^ex:pointOfB ?point . ?point ex:step ?step .";
    let explicit = "{ ?changer ex:tableOfA $this } UNION { ?changer ex:tableOfB $this } ?changer ex:low ?low . ?changer ex:high ?high . { ?point ex:pointOfA $this } UNION { ?point ex:pointOfB $this } ?point ex:step ?step .";
    let engine = NativeSparqlEngine::new();
    let focuses = [
        None,
        Some(TermValue::Iri(format!("{EX}this1"))),
        Some(TermValue::Blank {
            label: "this0".to_owned(),
            scope: BlankScope::DEFAULT,
        }),
    ];
    for focus in focuses {
        let substitutions = focus
            .iter()
            .map(|term| ("this".to_owned(), term.clone()))
            .collect::<Vec<_>>();
        let options = if focus.is_some() {
            vec![
                QueryOptions::EMPTY,
                QueryOptions::EMPTY.with_prebinding(ShaclPrebinding::Applied),
            ]
        } else {
            vec![QueryOptions::EMPTY]
        };
        let subjects: Vec<_> = focus.as_ref().map_or_else(
            || {
                (0..COUNT)
                    .map(|index| {
                        if index == 0 {
                            "_:this0".to_owned()
                        } else {
                            format!("<{EX}this{index}>")
                        }
                    })
                    .collect()
            },
            |focus| vec![render_cell(Some(focus))],
        );
        let mut expected: Vec<_> = subjects
            .into_iter()
            .flat_map(|subject| {
                std::iter::repeat_n(
                    row(&[
                        ("this", &subject),
                        ("step", "0"),
                        ("low", "10"),
                        ("high", "20"),
                    ]),
                    4,
                )
            })
            .collect();
        expected.sort();
        for options in options {
            for body in [original, explicit] {
                let text = format!(
                    "PREFIX ex: <{EX}> SELECT $this ?step ?low ?high WHERE {{ {body} FILTER (?step < ?low || ?step > ?high) }}"
                );
                let outcome = engine
                    .query_governed(
                        &data,
                        SparqlRequest {
                            query: &text,
                            base_iri: None,
                            substitutions: &substitutions,
                        },
                        options,
                        &QueryGovernors::METERED.with_max_intermediate_cells(CELLS),
                    )
                    .expect("two alternatives outcome");
                assert!(
                    outcome.is_complete(),
                    "both connectors must complete under the fixed budget: {outcome:?}"
                );
                let bag = sorted_rows(
                    &outcome.into_complete().expect("two alternatives complete"),
                    render_cell,
                );
                assert_eq!(bag, expected);
            }
        }
    }
}
