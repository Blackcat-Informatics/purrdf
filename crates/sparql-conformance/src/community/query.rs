// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native query routes over independently admitted community artifacts.

use purrdf_conformance_kit::outcome::{self, Observation, OutcomeKind, Record};
use purrdf_conformance_kit::{GradeError, bag, graph::Reader, inventory};
use purrdf_core::{GraphMatch, SparqlRequest, SparqlResult, TermValue};
use purrdf_iri::vocab::{mf, qt, rdf, rdfs};

use super::{Case, Corpus, Execution, expected_observation, read};
use crate::{
    manifest::{ExpectedResult, SparqlTestCase, TestKind},
    run, service,
};

impl Corpus {
    fn query_case(&self, case: &Case<'_>) -> Result<SparqlTestCase, String> {
        let query = case.case.query.as_deref().ok_or("query path is absent")?;
        let inventory::Expected::File(expected) = &case.case.expected else {
            return Err("a query case requires an expected result artifact".to_owned());
        };
        let mut data = Vec::new();
        let mut graph_data = Vec::new();
        for input in &case.case.data {
            if input
                .media_type
                .as_deref()
                .is_some_and(|media| media != "text/turtle")
            {
                return Err("query data must declare the native Turtle carrier".to_owned());
            }
            let path = self.path(case, &input.path)?;
            if let Some(graph) = &input.graph {
                graph_data.push((graph.clone(), path));
            } else {
                data.push(path);
            }
        }
        let service_data = case
            .case
            .services
            .iter()
            .map(|input| {
                if input.data.graph.is_some()
                    || input
                        .data
                        .media_type
                        .as_deref()
                        .is_some_and(|media| media != "text/turtle")
                {
                    return Err("SERVICE inputs must be independent Turtle documents".to_owned());
                }
                Ok((input.endpoint.clone(), self.path(case, &input.data.path)?))
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(SparqlTestCase {
            iri: case.iri().to_owned(),
            base: Self::artifact_iri(&self.relative(&self.path(case, query)?)?),
            name: case.case.id.clone(),
            kind: TestKind::QueryEval,
            query: self.path(case, query)?,
            data,
            graph_data,
            service_data,
            construct_data: None,
            regime: None,
            aggregate_namespace: None,
            expected: ExpectedResult::Srj(self.path(case, expected)?),
            lax_cardinality: false,
            requires: Vec::new(),
        })
    }

    pub(super) fn admit_query_entry(&self, case: &Case<'_>) -> Result<(), String> {
        if case.case.shapes.is_some()
            || !matches!(case.case.options, purrdf_lex::json::Value::Null)
            || case
                .case
                .requires
                .iter()
                .any(|capability| capability != "sparql11-service")
        {
            return Err("query case declares unsupported operation metadata".to_owned());
        }
        let reader = Reader {
            dataset: &case.document.dataset,
            graph: GraphMatch::Default,
        };
        let error = |error: GradeError| error.to_string();
        let iri_of = |path: &str| -> Result<String, String> {
            Ok(Self::artifact_iri(&self.relative(&self.path(case, path)?)?))
        };
        let kind = reader.one(case.entry, rdf::TYPE).map_err(error)?;
        if reader.iri(kind).map_err(error)? != mf::QUERY_EVALUATION_TEST {
            return Err("query inventory and manifest operation disagree".to_owned());
        }
        let action = reader.one(case.entry, mf::ACTION).map_err(error)?;
        let query = reader
            .iri(reader.one(action, qt::QUERY).map_err(error)?)
            .map_err(error)?;
        if query != iri_of(case.case.query.as_deref().ok_or("query input absent")?)? {
            return Err("query inventory and manifest query input disagree".to_owned());
        }
        let mut data = reader
            .objects(action, qt::DATA)
            .into_iter()
            .map(|id| Ok((reader.iri(id).map_err(error)?, None)))
            .collect::<Result<Vec<_>, String>>()?;
        for node in reader.objects(action, qt::GRAPH_DATA) {
            let (path, graph) = match reader.dataset.term_value(node) {
                TermValue::Iri(iri) => (iri.clone(), iri),
                _ => (
                    reader
                        .iri(reader.one(node, qt::GRAPH).map_err(error)?)
                        .map_err(error)?,
                    reader
                        .lexical(reader.one(node, rdfs::LABEL).map_err(error)?)
                        .map_err(error)?,
                ),
            };
            data.push((path, Some(graph)));
        }
        let mut expected_data = case
            .case
            .data
            .iter()
            .map(|input| Ok((iri_of(&input.path)?, input.graph.clone())))
            .collect::<Result<Vec<_>, String>>()?;
        data.sort();
        expected_data.sort();
        if data != expected_data {
            return Err("query inventory and manifest data/graph placement disagree".to_owned());
        }
        let mut services = Vec::new();
        for node in reader.objects(action, qt::SERVICE_DATA) {
            services.push((
                reader
                    .iri(reader.one(node, qt::ENDPOINT).map_err(error)?)
                    .map_err(error)?,
                reader
                    .iri(reader.one(node, qt::DATA).map_err(error)?)
                    .map_err(error)?,
            ));
        }
        let mut expected_services = case
            .case
            .services
            .iter()
            .map(|service| Ok((service.endpoint.clone(), iri_of(&service.data.path)?)))
            .collect::<Result<Vec<_>, String>>()?;
        services.sort();
        expected_services.sort();
        if services != expected_services || services.windows(2).any(|pair| pair[0].0 == pair[1].0) {
            return Err("query inventory and manifest SERVICE endpoints disagree".to_owned());
        }
        let inventory::Expected::File(expected) = &case.case.expected else {
            return Err("query expectation is not a result artifact".to_owned());
        };
        let result = reader
            .iri(reader.one(case.entry, mf::RESULT).map_err(error)?)
            .map_err(error)?;
        if result != iri_of(expected)? {
            return Err("query inventory and manifest result artifact disagree".to_owned());
        }
        self.query_case(case)?;
        Ok(())
    }

    pub(super) fn execute_query(
        &self,
        case: &Case<'_>,
        profile: &str,
        control: &mut purrdf_core::CanonicalizationControl<'_>,
    ) -> Result<Execution, String> {
        if case.case.kind != "query-evaluation" {
            return Err("the query profile cannot execute a SHACL operation".to_owned());
        }
        let spec = self.query_case(case)?;
        let text = std::fs::read_to_string(&spec.query).map_err(|error| error.to_string())?;
        let ExpectedResult::Srj(expected_path) = &spec.expected else {
            unreachable!("query_case selects the native SRJ reader")
        };
        let expected = purrdf_sparql_results::from_json(&read(expected_path)?)
            .map_err(|error| error.to_string())?;
        let ordered = run::query_eval_is_ordered(&text);
        let data = run::query_eval_dataset(&spec, &text)?;
        let remote = service::build(&spec)?;
        let env = run::query_eval_env(&spec)?;
        let options = run::query_eval_options(&spec, &env).with_remote(Some(&remote));
        let engine = run::query_eval_engine();
        let declared = expected_observation(case.case);
        let mut records = Vec::new();
        let mut record =
            |surface: &str, result: Result<SparqlResult, purrdf_core::RdfDiagnostic>| {
                let observed = match result {
                    Err(error) => Observation {
                        kind: if error.storage_error().is_some() {
                            OutcomeKind::ResourceExhausted
                        } else {
                            OutcomeKind::Crash
                        },
                        reason: None,
                        message: error.to_string(),
                    },
                    Ok(SparqlResult::Solutions {
                        variables, rows, ..
                    }) => {
                        match bag::compare_solutions(
                            &expected.variables,
                            &expected.rows,
                            &variables,
                            &rows,
                            ordered,
                            control,
                        ) {
                            Ok(()) => Observation {
                                kind: OutcomeKind::Compared,
                                reason: None,
                                message: "complete query bag equals the reviewed expectation"
                                    .to_owned(),
                            },
                            Err(error) => Observation {
                                kind: match &error {
                                    GradeError::Mismatch(_) => OutcomeKind::Mismatch,
                                    GradeError::Malformed(_)
                                    | GradeError::DuplicateAssertion { .. } => {
                                        OutcomeKind::Malformed
                                    }
                                    GradeError::Storage(_)
                                    | GradeError::Canonicalization(_)
                                    | GradeError::ProjectionLimit(_) => {
                                        OutcomeKind::ResourceExhausted
                                    }
                                },
                                reason: None,
                                message: error.to_string(),
                            },
                        }
                    }
                    Ok(_) => Observation {
                        kind: OutcomeKind::Mismatch,
                        reason: None,
                        message: "a SELECT case returned a different result kind".to_owned(),
                    },
                };
                records.push(Record {
                    case: case.iri().to_owned(),
                    profile: profile.to_owned(),
                    surface: surface.to_owned(),
                    passed: outcome::grade(&declared, &observed).is_ok(),
                    expected: declared.clone(),
                    observed,
                    artifacts: Vec::new(),
                });
            };
        record(
            "native/query",
            engine.query_with_source(
                &data,
                SparqlRequest {
                    query: &text,
                    base_iri: Some(&spec.base),
                    substitutions: &[],
                },
                &remote,
                &options,
            ),
        );
        let prepared = engine
            .prepare_query_with_options(&text, Some(&spec.base), &options)
            .map_err(|error| error.to_string())?;
        let parsed = purrdf_sparql_algebra::SparqlParser::new()
            .with_base_iri(spec.base.clone())
            .parse_query(&text)
            .map_err(|error| error.to_string())?;
        let algebra = engine
            .prepare_algebra(parsed, &options)
            .map_err(|error| error.to_string())?;
        for repetition in 0..2 {
            for (route, plan) in [("prepared", &prepared), ("algebra", &algebra)] {
                record(
                    &format!("native/{route}/{repetition}"),
                    engine.query_prepared(&data, plan, &[], &options),
                );
            }
        }
        Ok(Execution {
            kind: case.case.kind.clone(),
            case: case.iri().to_owned(),
            profile: profile.to_owned(),
            records,
        })
    }
}
