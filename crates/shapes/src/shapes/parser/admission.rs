// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Parser-owned query admission, with typed dated and exact compatibility errors.

use std::fmt;

use purrdf_sparql_algebra::{Query, SparqlParser};

use crate::model::{rdf, sh};
use crate::profile::{AdmissionReason, AdmissionRefusal, QueryPurpose, ShaclProfile};
use crate::shapes::{Parser, ShaclInstances};
use crate::term::Term;

/// A checked declaration's refusal, retained until the selected law requires it.
#[derive(Clone, Debug)]
pub(crate) enum QueryRefusal {
    /// The compatibility wrapper's original diagnostic, including its context.
    Legacy(String),
    /// A dated policy's structural refusal, independent of diagnostic wording.
    Dated(AdmissionRefusal),
}

impl QueryRefusal {
    /// Preserve the compatibility wrapper's context without changing dated data.
    #[must_use]
    pub(crate) fn legacy_context(self, context: impl FnOnce(String) -> String) -> Self {
        match self {
            Self::Legacy(message) => Self::Legacy(context(message)),
            dated @ Self::Dated(_) => dated,
        }
    }

    /// Separate declaration failure from a restriction whose applicability
    /// depends on actual execution. Query-form well-formedness always applies;
    /// REC20170720 additionally requires graph-wide Core/component admission.
    pub(crate) fn requires_parse_failure(&self) -> bool {
        match self {
            Self::Legacy(_) => true,
            Self::Dated(refusal) => {
                refusal.reason() == AdmissionReason::QueryForm
                    || (refusal.profile() == ShaclProfile::REC_20170720
                        && matches!(
                            refusal.purpose(),
                            QueryPurpose::SelectConstraint
                                | QueryPurpose::SelectValidator
                                | QueryPurpose::AskValidator
                        ))
            }
        }
    }
}

impl fmt::Display for QueryRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Legacy(message) => f.write_str(message),
            Self::Dated(refusal) => refusal.fmt(f),
        }
    }
}

/// Both laws call the existing pre-binding walker; only its declared policy varies.
pub(crate) fn audit_query(
    profile: ShaclProfile,
    purpose: QueryPurpose,
    query: &Query,
    parameters: &[&str],
    compatibility: impl FnOnce() -> Result<(), String>,
) -> Result<(), QueryRefusal> {
    if profile == ShaclProfile::LEGACY {
        compatibility().map_err(QueryRefusal::Legacy)
    } else {
        profile
            .admit_query(purpose, query, parameters)
            .map_err(QueryRefusal::Dated)
    }
}

impl Parser<'_> {
    /// The one law supplied when this parse was constructed.
    pub(crate) const fn profile(&self) -> ShaclProfile {
        self.profile
    }

    /// Record an actual failure under the selected law's applicability rule.
    pub(crate) fn refuse_query(&self, declaration: String, refusal: QueryRefusal) -> String {
        let message = match refusal {
            QueryRefusal::Legacy(message) => message,
            QueryRefusal::Dated(refusal) => {
                let message = refusal.to_string();
                self.admission_refusal.borrow_mut().get_or_insert(refusal);
                message
            }
        };
        self.refuse_prebinding(crate::error::PrebindingViolation::new(declaration, message))
    }

    /// Parse and audit one recognized `sh:sparql` constraint. The owning shape
    /// supplies its prefixes; an unattached typed constraint uses its own node.
    pub(crate) fn sparql_constraint_query(
        &self,
        owner: &Term,
        constraint: &Term,
    ) -> Result<String, String> {
        let raw_select = self
            .first_object_of(constraint, sh::SELECT)
            .and_then(|term| match term {
                Term::Literal(literal) => Some(literal.value().to_owned()),
                _ => None,
            })
            .ok_or_else(|| {
                format!(
                    "sh:sparql constraint on shape {owner} is missing a sh:select string literal"
                )
            })?;
        let select = format!("{}{raw_select}", self.prefix_header(&[owner, constraint])?);
        // The query runs with `$this` and the shape context pre-bound.
        match SparqlParser::new()
            .with_prebound_variables(crate::sparql::THIS_AND_SHAPE_CONTEXT)
            .parse_query(&select)
        {
            Ok(query @ Query::Select { .. }) => {
                if let Err(refusal) = audit_query(
                    self.profile,
                    QueryPurpose::SelectConstraint,
                    &query,
                    &[],
                    || {
                        crate::prebinding::check_select(&query, &["this"]).and_then(|()| {
                            crate::prebinding::check_shape_context_unassigned(&query)
                        })
                    },
                ) && refusal.requires_parse_failure()
                {
                    return Err(self.refuse_query(
                        format!("sh:sparql constraint {constraint} on shape {owner}"),
                        refusal,
                    ));
                }
                Ok(select)
            }
            Ok(_) => Err(format!(
                "sh:sparql constraint on shape {owner} must be a SELECT query (ASK/CONSTRUCT/DESCRIBE are not valid SHACL-SPARQL)"
            )),
            Err(error) => Err(format!(
                "sh:sparql constraint on shape {owner} has an unparsable sh:select query: {error}"
            )),
        }
    }

    /// REC20170720 Appendix A requires failure for recognized SHACL-SPARQL
    /// declarations anywhere in the shapes graph. WD20260918 instead qualifies
    /// its restrictions by execution with pre-bound variables. Unrelated SELECT
    /// literals and AF functions/targets retain their own purpose contracts.
    pub(crate) fn refuse_rec_declarations(&mut self) -> Result<(), String> {
        if self.profile != ShaclProfile::REC_20170720 {
            return Ok(());
        }
        if let Some((declaration, refusal)) = self.component_registry.rec_declaration_refusal.take()
        {
            return Err(self.refuse_query(declaration, QueryRefusal::Dated(refusal)));
        }

        let mut constraints: Vec<(Term, Term)> = self
            .quads_with(None, Some(sh::SPARQL), None)
            .into_iter()
            .map(|(owner, _, constraint)| (owner, constraint))
            .collect();
        let mut attached: purrdf_rdf::FastSet<Term> = constraints
            .iter()
            .map(|(_, constraint)| constraint.clone())
            .collect();
        let mut instances = ShaclInstances::new(self.data);
        for (constraint, _, class) in self.quads_with(None, Some(rdf::TYPE), None) {
            if crate::data::resolve_id(self.data, &class)
                .is_some_and(|class| instances.reaches_iri(class, sh::SPARQL_CONSTRAINT))
                && attached.insert(constraint.clone())
            {
                constraints.push((constraint.clone(), constraint));
            }
        }
        constraints.sort_by(|(left_owner, left), (right_owner, right)| {
            crate::term::canonical_cmp(left_owner, right_owner)
                .then_with(|| {
                    crate::data::resolve_id(self.data, left_owner)
                        .cmp(&crate::data::resolve_id(self.data, right_owner))
                })
                .then_with(|| crate::term::canonical_cmp(left, right))
                .then_with(|| {
                    crate::data::resolve_id(self.data, left)
                        .cmp(&crate::data::resolve_id(self.data, right))
                })
        });
        constraints.dedup();
        for (owner, constraint) in constraints {
            self.sparql_constraint_query(&owner, &constraint)?;
        }
        Ok(())
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use std::sync::Arc;

    use crate::profile::{AdmissionReason, AdmissionRefusal, QueryPurpose, ShaclProfile};
    use crate::shapes::{Parser, Preparation, Shapes};

    const PREFIXES: &str =
        "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .";
    const DATED: [ShaclProfile; 2] = [ShaclProfile::REC_20170720, ShaclProfile::WD_20260918];

    #[derive(Debug)]
    struct ParseFailure {
        error: Box<crate::error::ShapesError>,
        admission: Option<AdmissionRefusal>,
    }

    fn parse(profile: ShaclProfile, body: &str) -> Result<Shapes, ParseFailure> {
        let data =
            crate::text_ingest::parse_turtle_to_dataset(&format!("{PREFIXES}\n{body}"), None)
                .unwrap();
        let mut parser = Parser::new(&data, None, &[], None, Arc::clone(&data), None, profile);
        parser.parse().map_err(|error| ParseFailure {
            error: Box::new(error),
            admission: parser.admission_refusal.borrow_mut().take(),
        })
    }

    fn constraint(query: &str) -> String {
        format!(
            "ex:S a sh:NodeShape; sh:targetNode ex:n; sh:sparql ex:C . \
             ex:C sh:select '{query}' ."
        )
    }

    fn refusal(profile: ShaclProfile, body: &str, purpose: QueryPurpose) -> AdmissionRefusal {
        let failure = parse(profile, body).unwrap_err();
        let admission = failure
            .admission
            .unwrap_or_else(|| panic!("expected typed {profile} admission, got {}", failure.error));
        assert_eq!(admission.profile(), profile);
        assert_eq!(admission.purpose(), purpose);
        admission
    }

    /// Exercise the production binding/execution doors under the parser's
    /// admitted occurrence context. These native fixtures contain no regex;
    /// full dated XPath request admission is tested at its composed boundary.
    fn in_query_scope<R>(
        profile: ShaclProfile,
        body: &str,
        run: impl FnOnce(Shapes) -> Result<R, crate::report::CompleteValidationError>,
    ) -> Result<R, crate::report::CompleteValidationError> {
        let shapes = parse(profile, body).map_err(|failure| {
            failure.admission.map_or_else(
                || crate::report::CompleteValidationError::Shapes(*failure.error),
                |refusal| crate::report::CompleteValidationError::Admission(Box::new(refusal)),
            )
        })?;
        let sources = shapes.report_sources_with_profile(profile)?;
        let scope = crate::query_law::enter(profile, Some(&sources));
        run(shapes).map_err(|error| {
            scope
                .runtime
                .as_ref()
                .and_then(|law| law.take_failure())
                .map_or(error, crate::report::ReportFailure::into_public)
        })
    }

    fn validate(
        profile: ShaclProfile,
        body: &str,
    ) -> Result<crate::report::CompleteValidationReport, crate::report::CompleteValidationError>
    {
        in_query_scope(profile, body, |shapes| {
            let data = purrdf_rdf::RdfDatasetBuilder::new().freeze().unwrap();
            let bound = crate::engine::PreparedShapes::new(Arc::new(shapes))
                .bind_complete_shared_dataset(data)?;
            bound.validate_report_with_focus_filter(profile, |_, _| true)
        })
    }

    fn execution_refusal(
        profile: ShaclProfile,
        body: &str,
        purpose: QueryPurpose,
    ) -> AdmissionRefusal {
        let error = validate(profile, body).unwrap_err();
        let crate::report::CompleteValidationError::Admission(refusal) = error else {
            panic!("expected actual {purpose:?} execution admission, got {error}")
        };
        assert_eq!(refusal.profile(), profile);
        assert_eq!(refusal.purpose(), purpose);
        *refusal
    }

    fn infer(
        profile: ShaclProfile,
        body: &str,
    ) -> Result<crate::srl::Inference, crate::report::CompleteValidationError> {
        in_query_scope(profile, body, |shapes| {
            let data = purrdf_rdf::RdfDatasetBuilder::new().freeze().unwrap();
            let data = crate::data::ShaclData::new(Arc::clone(&data), data, None);
            crate::rules::infer(&data, &shapes, &crate::rules::RuleOptions::default())
                .map_err(crate::report::CompleteValidationError::Execution)
        })
    }

    fn rule_refusal(profile: ShaclProfile, body: &str, purpose: QueryPurpose) -> AdmissionRefusal {
        let error = infer(profile, body).unwrap_err();
        let crate::report::CompleteValidationError::Admission(refusal) = error else {
            panic!("expected actual {purpose:?} rule admission, got {error}")
        };
        assert_eq!(refusal.profile(), profile);
        assert_eq!(refusal.purpose(), purpose);
        *refusal
    }

    #[test]
    fn mismatched_target_evidence_retains_the_actual_authored_declaration() {
        let graph = r#"
            ex:S a sh:NodeShape; sh:target ex:T .
            ex:T a sh:SPARQLTarget; sh:select "SELECT ?this WHERE {}" .
        "#;
        for profile in DATED {
            let shapes = parse(profile, graph).unwrap();
            let sources = shapes.report_sources_with_profile(profile).unwrap();
            let shape = &shapes.node_shapes[0];
            let scope = crate::query_law::enter(profile, Some(&sources));
            let law = scope.runtime.as_ref().unwrap();
            assert!(
                law.target_invocation(&shape.id, 0, &shape.targets[0])
                    .is_ok()
            );
            assert!(law.take_failure().is_none());
            let mut target = shape.targets[0].clone();
            let crate::shapes::Target::Sparql { select, .. } = &mut target else {
                panic!("fixture declares a SPARQL target")
            };
            select.push_str(" LIMIT 1");
            assert!(law.target_invocation(&shape.id, 0, &target).is_err());
            let Some(crate::report::ReportFailure::QuerySource(refusal)) = law.take_failure()
            else {
                panic!("mismatched source must retain a typed declaration refusal")
            };
            assert_eq!(refusal.profile(), profile);
            assert_eq!(refusal.purpose(), Some(QueryPurpose::SelectTarget));
            assert_eq!(
                refusal.source(),
                &sources.targets[&shape.id][&0].source_target
            );
            assert_eq!(refusal.index(), Some(0));
        }
    }

    #[test]
    fn occurrence_metadata_keeps_source_roles_and_absent_optional_declarations() {
        let graph = r#"
            ex:T a sh:SPARQLTargetType; sh:parameter [ sh:path ex:optional; sh:optional true ];
                sh:select "SELECT ?this WHERE { BIND(<http://example.org/n> AS ?this) }" .
            ex:plain a sh:SPARQLTarget;
                sh:select "SELECT ?this WHERE { BIND(<http://example.org/n> AS ?this) }" .
            ex:typed a ex:T .
            ex:Template a sh:SPARQLRuleTemplate;
                sh:parameter [ sh:path ex:optional; sh:optional true ];
                sh:construct "CONSTRUCT { <http://example.org/s> <http://example.org/p> <http://example.org/o> } WHERE {}" .
            ex:global a ex:Template . ex:linked a ex:Template .
            ex:S a sh:NodeShape; sh:target ex:plain, ex:typed; sh:rule ex:linked .
        "#;
        for profile in DATED {
            let shapes = parse(profile, graph).unwrap();
            let sources = shapes.report_sources_with_profile(profile).unwrap();
            let shape = shapes
                .node_shapes
                .iter()
                .find(|shape| shape.id.to_string() == "<http://example.org/S>")
                .unwrap();
            let targets = &sources.targets[&shape.id];
            assert_eq!(targets.len(), 2);
            let mut actual = Vec::new();
            for (index, occurrence) in targets {
                let crate::shapes::Target::Sparql {
                    select,
                    ask,
                    substitutions,
                } = &shape.targets[*index]
                else {
                    panic!("source occurrence must address the actual SPARQL target slot")
                };
                assert_eq!(&occurrence.select, select);
                assert_eq!(&occurrence.ask, ask);
                assert_eq!(&occurrence.substitutions, substitutions);
                actual.push((
                    occurrence.source_target.to_string(),
                    occurrence.purpose,
                    occurrence.parameters.clone(),
                ));
            }
            actual.sort_by(|left, right| left.0.cmp(&right.0));
            assert_eq!(
                actual,
                [
                    (
                        "<http://example.org/plain>".to_owned(),
                        QueryPurpose::SelectTarget,
                        vec![]
                    ),
                    (
                        "<http://example.org/typed>".to_owned(),
                        QueryPurpose::TargetType,
                        vec!["optional".to_owned()]
                    ),
                ]
            );
            assert_eq!(sources.rules.len(), 2);
            for (rule, uses) in &sources.rules {
                for (owner, occurrence) in uses
                    .linked
                    .iter()
                    .map(|(owner, occurrence)| (Some(owner), occurrence))
                    .chain(uses.global.as_ref().map(|occurrence| (None, occurrence)))
                {
                    assert_eq!(occurrence.parameters, ["optional"]);
                    assert_eq!(occurrence.substitutions, []);
                    let (model, purpose) = match owner {
                        Some(owner) => {
                            assert_eq!(owner, &shape.id);
                            (
                                shape
                                    .rules
                                    .iter()
                                    .find(|candidate| &candidate.id == rule)
                                    .unwrap(),
                                QueryPurpose::ConstructRule,
                            )
                        }
                        None => (
                            shapes
                                .rules
                                .global_rules
                                .iter()
                                .find(|candidate| &candidate.id == rule)
                                .unwrap(),
                            QueryPurpose::GlobalConstructRule,
                        ),
                    };
                    assert_eq!(occurrence.purpose, purpose);
                    let crate::rules::RuleBody::Sparql {
                        construct,
                        parameters,
                    } = &model.body
                    else {
                        panic!("source occurrence must match its actual SPARQL rule")
                    };
                    assert_eq!(&occurrence.construct, construct);
                    assert_eq!(&occurrence.substitutions, parameters);
                }
            }
        }
    }

    #[test]
    fn a_warm_other_law_cache_cannot_grant_rec_declaration_admission() {
        let declaration = r#"
            ex:C a sh:ConstraintComponent; sh:parameter [ sh:path ex:argument ];
                sh:nodeValidator [ a sh:SPARQLSelectValidator;
                    sh:select "SELECT $this WHERE { VALUES ?argument { true } }" ] .
        "#;
        for original in [ShaclProfile::LEGACY, ShaclProfile::WD_20260918] {
            let shapes = parse(original, declaration).unwrap();
            let original_sources = Arc::clone(shapes.sparql_sources.get().unwrap());
            let error = shapes
                .report_sources_with_profile(ShaclProfile::REC_20170720)
                .unwrap_err();
            let crate::report::CompleteValidationError::Admission(refusal) = error else {
                panic!("the requested source law must retain its actual typed refusal")
            };
            assert_eq!(refusal.profile(), ShaclProfile::REC_20170720);
            assert_eq!(refusal.purpose(), QueryPurpose::SelectValidator);
            assert_eq!(refusal.reason(), AdmissionReason::Values);
            assert!(Arc::ptr_eq(
                &original_sources,
                shapes.sparql_sources.get().unwrap()
            ));
        }
    }

    #[test]
    fn dated_product_sources_reconstruct_without_legacy_readmission_or_model_relinking() {
        use crate::engine::PreparedShapes;
        use crate::product::{HostBindings, ShapesProduct, ShapesProfile};

        let shapes = parse(
            DATED[1],
            &constraint("SELECT $this WHERE { VALUES ?local { 1 } }"),
        )
        .unwrap();
        let original = shapes.report_sources_with_profile(DATED[1]).unwrap();
        assert!(Arc::ptr_eq(
            &original,
            &shapes.report_sources_with_profile(DATED[1]).unwrap()
        ));
        let prepared = PreparedShapes::new(Arc::new(shapes));
        let bytes = prepared.to_product(&ShapesProfile::CORE).unwrap();
        let restored = ShapesProduct::open(&bytes)
            .unwrap()
            .admit(&ShapesProfile::CORE, &HostBindings::empty())
            .unwrap();
        assert!(restored.shapes().sparql_sources.get().is_none());
        let original_classes = restored.class_catalog();
        let source_dataset = Arc::clone(&restored.shapes().shapes_dataset);
        let provenance = restored.shapes().provenance();
        let mut parser = Parser::new(
            &source_dataset,
            provenance.base().map(ToOwned::to_owned),
            provenance.doc_prefixes(),
            provenance.box_role_vocab().cloned(),
            Arc::clone(&source_dataset),
            provenance.shapes_graph().map(ToOwned::to_owned),
            DATED[1],
        );
        parser
            .prepare_with_expressions(&[], &[], Preparation::SourceOccurrences)
            .unwrap();
        assert!(
            parser.node_shape_index.get().is_none(),
            "occurrence admission must not install the shared model index"
        );
        assert!(!parser.sparql_sources.borrow().constraints.is_empty());
        let sources = restored
            .shapes()
            .report_sources_with_profile(DATED[1])
            .unwrap();
        assert_eq!(sources.constraints.len(), original.constraints.len());
        let repeated = restored
            .shapes()
            .report_sources_with_profile(DATED[1])
            .unwrap();
        assert!(Arc::ptr_eq(&sources, &repeated));
        assert!(Arc::ptr_eq(
            &source_dataset,
            &restored.shapes().shapes_dataset
        ));
        assert!(Arc::ptr_eq(&original_classes, &restored.class_catalog()));
        assert_eq!(restored.to_product(&ShapesProfile::CORE).unwrap(), bytes);
        let rejected = restored
            .shapes()
            .report_sources_with_profile(DATED[0])
            .unwrap_err();
        assert!(matches!(
            rejected,
            crate::report::CompleteValidationError::Admission(_)
        ));
    }

    #[test]
    fn dated_constraint_uses_local_values_and_projection_rules_of_its_own_law() {
        let local = constraint("SELECT $this WHERE { VALUES ?local { 1 } }");
        let rejected = refusal(DATED[0], &local, QueryPurpose::SelectConstraint);
        assert_eq!(rejected.reason(), AdmissionReason::Values);
        parse(DATED[1], &local).expect("draft local VALUES is admitted by the actual parser");
        let compatibility = parse(ShaclProfile::LEGACY, &local).unwrap_err();
        assert!(compatibility.error.as_prebinding().is_some());
        assert!(compatibility.admission.is_none());

        let hidden = constraint("SELECT $this WHERE { { SELECT ?v WHERE { $this ?p ?v } } }");
        let rejected = refusal(DATED[0], &hidden, QueryPurpose::SelectConstraint);
        assert_eq!(rejected.reason(), AdmissionReason::SubqueryProjection);
        assert_eq!(rejected.variable(), Some("this"));
        parse(DATED[1], &hidden).unwrap();
        for profile in DATED {
            parse(
                profile,
                &constraint("SELECT $this WHERE { { SELECT $this WHERE { $this ?p ?v } } }"),
            )
            .expect("projected neighbor is admitted under each dated parser law");
        }
    }

    #[test]
    fn component_declarations_obey_rec_graph_wide_and_draft_execution_restrictions() {
        let declaration = r#"
            ex:C a sh:ConstraintComponent; sh:parameter [ sh:path ex:argument ];
                sh:nodeValidator [ a sh:SPARQLSelectValidator; sh:select "SELECT $this WHERE { VALUES ?argument { true } }" ] .
        "#;
        let declared = refusal(DATED[0], declaration, QueryPurpose::SelectValidator);
        assert_eq!(declared.reason(), AdmissionReason::Values);
        parse(DATED[1], declaration).expect("draft unused validators are not executed");
        parse(ShaclProfile::LEGACY, declaration).expect("compatibility admission is unchanged");
        for profile in DATED {
            let selected = format!(
                "{declaration} ex:S a sh:NodeShape; sh:targetNode ex:n; ex:argument true ."
            );
            let rejected = execution_refusal(profile, &selected, QueryPurpose::SelectValidator);
            assert_eq!(rejected.reason(), AdmissionReason::Values);
            assert_eq!(
                rejected.variable(),
                (profile == DATED[1]).then_some("argument")
            );
            let neighbor = selected.replace("VALUES ?argument { true }", "FILTER(?argument)");
            validate(profile, &neighbor)
                .expect("using a pre-bound parameter executes successfully");
        }
    }

    #[test]
    fn declared_function_refusal_is_captured_only_when_reachable() {
        let declaration = r#"
            ex:F a sh:SPARQLFunction; sh:parameter [ sh:path ex:argument ];
                sh:select "SELECT ?out WHERE { VALUES ?argument { true } BIND(?argument AS ?out) }" .
        "#;
        for profile in DATED {
            parse(profile, declaration).expect("an uncalled function is not executed");
            let reached = format!(
                "{declaration} {}",
                constraint("SELECT $this WHERE { FILTER(<http://example.org/F>(true)) }")
            );
            let rejected = execution_refusal(profile, &reached, QueryPurpose::Function);
            assert_eq!(rejected.reason(), AdmissionReason::Values);
            let neighbor = reached.replace("VALUES ?argument { true }", "FILTER(?argument)");
            validate(profile, &neighbor).expect("called valid neighbor executes successfully");
        }
    }

    #[test]
    fn ask_validator_audits_its_actual_value_binding() {
        let selected = r#"
            ex:C a sh:ConstraintComponent; sh:parameter [ sh:path ex:argument ];
                sh:validator [ a sh:SPARQLAskValidator;
                    sh:ask "ASK { VALUES ?value { true } FILTER(?argument) }" ] .
            ex:S a sh:NodeShape; sh:targetNode ex:n; ex:argument true .
        "#;
        for profile in DATED {
            let rejected = execution_refusal(profile, selected, QueryPurpose::AskValidator);
            assert_eq!(rejected.reason(), AdmissionReason::Values);
            assert_eq!(
                rejected.variable(),
                (profile == DATED[1]).then_some("value")
            );
            validate(
                profile,
                &selected.replace("VALUES ?value { true }", "FILTER(BOUND(?value))"),
            )
            .expect("reading the admitted value binding executes successfully");
        }
    }

    #[test]
    fn instantiated_target_types_and_plain_target_inverses_keep_their_roles() {
        let declaration = r#"
            ex:T a sh:SPARQLTargetType; sh:parameter [ sh:path ex:argument ];
                sh:select """SELECT ?this WHERE {
                    VALUES ?argument { true } BIND(<http://example.org/n> AS ?this)
                }""" .
        "#;
        for profile in DATED {
            let selected = format!(
                "{declaration} ex:S a sh:NodeShape; sh:target [ a ex:T; ex:argument true ] ."
            );
            let rejected = execution_refusal(profile, &selected, QueryPurpose::TargetType);
            assert_eq!(rejected.reason(), AdmissionReason::Values);
            validate(
                profile,
                &selected.replace("VALUES ?argument { true }", "FILTER(?argument)"),
            )
            .expect("producing this executes when only target parameters are pre-bound");

            let inverse = r#"
                ex:S a sh:NodeShape; sh:target [ a sh:SPARQLTarget;
                    sh:select "SELECT ?this WHERE { BIND(<http://example.org/n> AS ?this) }";
                    sh:ask "ASK { VALUES $this { <http://example.org/n> } }"
                ] .
            "#;
            let check = |body: &str| {
                in_query_scope(profile, body, |shapes| {
                    let data = purrdf_rdf::RdfDatasetBuilder::new().freeze().unwrap();
                    let bound = crate::engine::PreparedShapes::new(Arc::new(shapes))
                        .bind_complete_shared_dataset(data)?;
                    bound
                        .legacy_binding()
                        .validate_focus_nodes(&[crate::term::Term::NamedNode(
                            crate::term::NamedNode::new_unchecked("http://example.org/n"),
                        )])
                        .map_err(crate::report::CompleteValidationError::Shapes)
                })
            };
            let error = check(inverse).unwrap_err();
            let crate::report::CompleteValidationError::Admission(rejected) = error else {
                panic!("ASK target must refuse at actual candidate dispatch: {error}")
            };
            assert_eq!(rejected.purpose(), QueryPurpose::AskTarget);
            assert_eq!(rejected.reason(), AdmissionReason::Values);
            check(&inverse.replace(
                "VALUES $this { <http://example.org/n> }",
                "FILTER($this = <http://example.org/n>)",
            ))
            .expect("an ASK inverse executes with its candidate focus binding");
        }
    }

    #[test]
    fn reached_select_and_scalar_expressions_use_the_same_dated_audit() {
        for (key, body, purpose, neighbor) in [
            (
                "sh:select",
                "SELECT ?result WHERE { VALUES $this { <http://example.org/n> } BIND(true AS ?result) }",
                QueryPurpose::SelectExpression,
                "SELECT ?result WHERE { FILTER(BOUND($this)) BIND(true AS ?result) }",
            ),
            (
                "sh:sparqlExpr",
                "EXISTS { VALUES $this { <http://example.org/n> } }",
                QueryPurpose::ScalarExpression,
                "BOUND($this)",
            ),
        ] {
            let selected = format!(
                "ex:S a sh:NodeShape; sh:targetNode ex:n; sh:expression [ {key} '{body}' ] ."
            );
            for profile in DATED {
                let rejected = execution_refusal(profile, &selected, purpose);
                assert_eq!(rejected.reason(), AdmissionReason::Values);
                validate(profile, &selected.replace(body, neighbor))
                    .expect("a reached expression executes with its focus binding");
            }
        }
    }

    #[test]
    fn an_unvalued_shape_context_is_the_drafts_own_variable() {
        // The draft's potentially pre-bound variables are `this`, `value` and the
        // parameters (SHACL 1.2 SPARQL Extensions, Appendix A), so with no
        // shapes-graph IRI an assignment of `?shapesGraph` binds the query's own
        // variable and its row is a result. The Recommendation refuses it.
        let assigns =
            constraint("SELECT $this WHERE { BIND(<http://example.org/g> AS ?shapesGraph) }");
        let [rec, wd] = DATED;
        let refused = execution_refusal(rec, &assigns, QueryPurpose::SelectConstraint);
        assert_eq!(refused.reason(), AdmissionReason::Assignment);
        assert_eq!(refused.variable(), Some("shapesGraph"));
        let report = validate(wd, &assigns).expect("the draft admits the assignment");
        assert_eq!(
            report.legacy().results.len(),
            1,
            "the assigned row is a result"
        );
        // Its reading neighbour: the unvalued name is unbound under both laws.
        let reads = constraint("SELECT $this WHERE { FILTER(!BOUND(?shapesGraph)) }");
        for profile in DATED {
            let report = validate(profile, &reads).expect("reading the shape context executes");
            assert_eq!(report.legacy().results.len(), 1);
        }
    }

    /// [`validate`] with the shapes graph exposed under `shapes_graph`, so a law that
    /// pre-binds the shape context has a `$shapesGraph` value to bind.
    fn validate_named(
        profile: ShaclProfile,
        body: &str,
        shapes_graph: &str,
    ) -> Result<crate::report::CompleteValidationReport, crate::report::CompleteValidationError>
    {
        in_query_scope(profile, body, |shapes| {
            let data = purrdf_rdf::RdfDatasetBuilder::new().freeze().unwrap();
            let bound = crate::engine::PreparedShapes::new(Arc::new(shapes))
                .bind_complete_shared_dataset_with_shapes_graph(data, Some(shapes_graph))?;
            bound.validate_report_with_focus_filter(profile, |_, _| true)
        })
    }

    #[test]
    fn the_draft_pre_binds_no_shape_context_even_when_it_has_a_value() {
        // SHACL 1.2 SPARQL Extensions (18 September 2026), Appendix A: the potentially
        // pre-bound variables are `this`, `value` and the parameters. With a
        // shapes-graph IRI set, the draft's `?shapesGraph` is still the query's own
        // variable, so its assignment produces the row that is the violation.
        const GRAPH: &str = "http://example.org/shapes";
        let assigns =
            constraint("SELECT $this WHERE { BIND(<http://example.org/g> AS ?shapesGraph) }");
        let unbound = constraint("SELECT $this WHERE { FILTER(!BOUND(?shapesGraph)) }");
        let bound =
            constraint("SELECT $this WHERE { FILTER(?shapesGraph = <http://example.org/shapes>) }");
        let [rec, wd] = DATED;
        let report = validate_named(wd, &assigns, GRAPH).expect("the draft admits it");
        assert_eq!(
            report.legacy().results.len(),
            1,
            "the assigned row is a result"
        );
        let report = validate_named(wd, &unbound, GRAPH).expect("the draft reads it");
        assert_eq!(report.legacy().results.len(), 1, "the draft binds no value");
        // The Recommendation and the compatibility law keep pre-binding it.
        let refused = execution_refusal(rec, &assigns, QueryPurpose::SelectConstraint);
        assert_eq!(refused.reason(), AdmissionReason::Assignment);
        let report = validate_named(rec, &bound, GRAPH).expect("the REC reads it");
        assert_eq!(report.legacy().results.len(), 1, "the REC binds the IRI");
        assert!(parse(ShaclProfile::LEGACY, &assigns).is_err());
        parse(ShaclProfile::LEGACY, &bound).expect("the compatibility law reads it");
    }

    #[test]
    fn shape_global_and_template_rules_audit_only_their_actual_bindings() {
        let global = r#"
            ex:R a sh:SPARQLRule; sh:construct """CONSTRUCT {
                ?this <http://example.org/p> true
            } WHERE { BIND(<http://example.org/n> AS ?this) }""" .
        "#;
        for profile in DATED {
            parse(profile, global).expect("a global rule has no initial focus-node binding");
            let shape_rule =
                format!("{global} ex:S a sh:NodeShape; sh:targetNode ex:n; sh:rule ex:R .");
            let rejected = rule_refusal(profile, &shape_rule, QueryPurpose::ConstructRule);
            assert_eq!(rejected.reason(), AdmissionReason::Assignment);
            assert_eq!(rejected.variable(), Some("this"));
            infer(
                profile,
                &shape_rule.replace(
                    "BIND(<http://example.org/n> AS ?this)",
                    "FILTER(BOUND(?this))",
                ),
            )
            .expect("the shape-rule neighbor executes with its focus binding");

            let template = r#"
                ex:T a sh:SPARQLRuleTemplate; sh:parameter [ sh:path ex:argument ];
                    sh:construct """CONSTRUCT {
                        <http://example.org/n> <http://example.org/p> ?argument
                    } WHERE { VALUES ?argument { true } }""" .
                ex:R a ex:T; ex:argument true .
            "#;
            let rejected = rule_refusal(profile, template, QueryPurpose::GlobalConstructRule);
            assert_eq!(rejected.reason(), AdmissionReason::Values);
            infer(
                profile,
                &template.replace("VALUES ?argument { true }", "FILTER(?argument)"),
            )
            .expect("a global template executes with its admitted parameters");
        }
    }

    #[test]
    fn absent_optional_parameters_do_not_become_local_variables_at_execution() {
        let profile = DATED[1];
        let target = r#"
            ex:T a sh:SPARQLTargetType;
                sh:parameter [ sh:path ex:optional; sh:optional true ];
                sh:select "SELECT ?this WHERE { VALUES ?optional { true } BIND(<http://example.org/n> AS ?this) }" .
            ex:S a sh:NodeShape; sh:target [ a ex:T ] .
        "#;
        let refused = execution_refusal(profile, target, QueryPurpose::TargetType);
        assert_eq!(refused.reason(), AdmissionReason::Values);
        assert_eq!(refused.variable(), Some("optional"));
        validate(
            profile,
            &target.replace("VALUES ?optional { true }", "FILTER(!BOUND(?optional))"),
        )
        .unwrap();
        let plain = target
            .replace("ex:T a sh:SPARQLTargetType;", "ex:T a sh:SPARQLTarget;")
            .replace(
                "sh:parameter [ sh:path ex:optional; sh:optional true ];",
                "",
            )
            .replace("sh:target [ a ex:T ]", "sh:target ex:T");
        validate(profile, &plain).expect("the same text has a local variable in a plain target");

        let component = r#"
            ex:C a sh:ConstraintComponent;
                sh:parameter [ sh:path ex:argument ], [ sh:path ex:optional; sh:optional true ];
                sh:nodeValidator [ a sh:SPARQLSelectValidator;
                    sh:select "SELECT $this WHERE { VALUES ?optional { true } FILTER(?argument) }" ] .
            ex:S a sh:NodeShape; sh:targetNode ex:n; ex:argument true .
        "#;
        let refused = execution_refusal(profile, component, QueryPurpose::SelectValidator);
        assert_eq!(refused.reason(), AdmissionReason::Values);
        assert_eq!(refused.variable(), Some("optional"));
        validate(
            profile,
            &component.replace("VALUES ?optional { true }", "FILTER(!BOUND(?optional))"),
        )
        .unwrap();

        let function = r#"
            ex:F a sh:SPARQLFunction;
                sh:parameter [ sh:path ex:optional; sh:optional true ];
                sh:select "SELECT ?out WHERE { VALUES ?optional { true } BIND(true AS ?out) }" .
            ex:S a sh:NodeShape; sh:targetNode ex:n;
                sh:sparql [ sh:select "SELECT $this WHERE { FILTER(<http://example.org/F>()) }" ] .
        "#;
        let refused = execution_refusal(profile, function, QueryPurpose::Function);
        assert_eq!(refused.reason(), AdmissionReason::Values);
        assert_eq!(refused.variable(), Some("optional"));
        validate(
            profile,
            &function.replace("VALUES ?optional { true }", "FILTER(!BOUND(?optional))"),
        )
        .unwrap();
    }

    #[test]
    fn draft_execution_restrictions_do_not_reject_deactivated_empty_or_short_circuited_queries() {
        let profile = DATED[1];
        let invalid = constraint("SELECT $this WHERE { VALUES $this { <http://example.org/n> } }");
        assert_eq!(
            execution_refusal(profile, &invalid, QueryPurpose::SelectConstraint).variable(),
            Some("this")
        );
        for inactive in [
            invalid.replace("sh:targetNode ex:n;", ""),
            invalid.replace(
                "ex:S a sh:NodeShape;",
                "ex:S a sh:NodeShape; sh:deactivated true;",
            ),
            invalid.replace("ex:C sh:select", "ex:C sh:deactivated true; sh:select"),
        ] {
            assert!(validate(profile, &inactive).unwrap().legacy().conforms);
        }
        let function = r#"
            ex:F a sh:SPARQLFunction; sh:parameter [ sh:path ex:argument ];
                sh:select "SELECT ?out WHERE { VALUES ?argument { true } BIND(true AS ?out) }" .
            ex:S a sh:NodeShape; sh:targetNode ex:n;
                sh:sparql [ sh:select "SELECT $this WHERE { FILTER(false && <http://example.org/F>(true)) }" ] .
        "#;
        // AND evaluates its operand in the current engine; IF owns an actual
        // non-invoked branch. Keep both neighbors so a visited body cannot
        // masquerade as an unexecuted declaration.
        execution_refusal(profile, function, QueryPurpose::Function);
        let uncalled = function.replace(
            "false && <http://example.org/F>(true)",
            "IF(false, <http://example.org/F>(true), false)",
        );
        assert!(validate(profile, &uncalled).unwrap().legacy().conforms);
        let called = uncalled.replace("IF(false,", "IF(true,");
        execution_refusal(profile, &called, QueryPurpose::Function);
    }

    #[test]
    fn a_legacy_warm_or_restored_rule_keeps_dated_admission_before_an_empty_join() {
        use crate::engine::PreparedShapes;
        use crate::product::{HostBindings, ShapesProduct, ShapesProfile};
        use crate::srl::ir::IrRuleBody;

        let declaration = r#"
            ex:T a sh:SPARQLRuleTemplate;
                sh:parameter [ sh:path ex:optional; sh:optional true ];
                sh:construct "CONSTRUCT { ?s <http://example.org/q> ?optional } WHERE { ?s <http://example.org/p> ?o BIND(true AS ?optional) }" .
            ex:R a ex:T; sh:runOnce true .
        "#;
        let prepared =
            PreparedShapes::new(Arc::new(parse(ShaclProfile::LEGACY, declaration).unwrap()));
        let options = crate::rules::RuleOptions::default();
        let warm = crate::rules::shacl_rule_set(prepared.shapes(), &options).unwrap();
        assert!(matches!(warm.rules[0].body, IrRuleBody::Elements(_)));
        let bytes = prepared.to_product(&ShapesProfile::CORE).unwrap();
        let restored = ShapesProduct::open(&bytes)
            .unwrap()
            .admit(&ShapesProfile::CORE, &HostBindings::empty())
            .unwrap();
        for prepared in [&prepared, &restored] {
            let classes = prepared.class_catalog();
            for profile in DATED {
                let sources = prepared
                    .shapes()
                    .report_sources_with_profile(profile)
                    .unwrap();
                let scope = crate::query_law::enter(profile, Some(&sources));
                let current = crate::rules::shacl_rule_set(prepared.shapes(), &options).unwrap();
                for graph in ["", "ex:n ex:p 1 ."] {
                    let data = crate::text_ingest::parse_turtle_to_dataset(
                        &format!("{PREFIXES}{graph}"),
                        None,
                    )
                    .unwrap();
                    let data = crate::data::ShaclData::new(Arc::clone(&data), data, None);
                    crate::rules::infer(&data, prepared.shapes(), &options).expect_err(
                        "the whole query is admitted even when its BGP produces no row",
                    );
                    let crate::report::ReportFailure::Admission(refusal) =
                        scope.runtime.as_ref().unwrap().take_failure().unwrap()
                    else {
                        panic!("the actual selected rule must preserve the typed admission cause")
                    };
                    assert_eq!(refusal.profile(), profile);
                    assert_eq!(refusal.purpose(), QueryPurpose::GlobalConstructRule);
                    assert_eq!(refusal.reason(), AdmissionReason::Assignment);
                    assert_eq!(refusal.variable(), Some("optional"));
                }
                assert!(matches!(current.rules[0].body, IrRuleBody::Shacl(_)));
                drop(scope);
                let compatibility =
                    crate::rules::shacl_rule_set(prepared.shapes(), &options).unwrap();
                assert!(matches!(
                    compatibility.rules[0].body,
                    IrRuleBody::Elements(_)
                ));
            }
            assert!(Arc::ptr_eq(&classes, &prepared.class_catalog()));
            assert_eq!(prepared.to_product(&ShapesProfile::CORE).unwrap(), bytes);
        }
    }

    #[test]
    fn dated_rules_preserve_selected_firing_conditions_layers_and_run_once() {
        let conditional = r#"
            ex:S a sh:NodeShape; sh:targetNode ex:n; sh:rule ex:R .
            ex:R a sh:SPARQLRule; sh:layer 1; sh:runOnce true;
                sh:condition [ sh:property [ sh:path ex:enabled; sh:hasValue true ] ];
                sh:construct "CONSTRUCT { ?this <http://example.org/q> true } WHERE { BIND(<http://example.org/n> AS ?this) }" .
        "#;
        let enabling = r#"
            ex:Enable a sh:SPARQLRule; sh:layer 0; sh:runOnce true;
                sh:construct "CONSTRUCT { <http://example.org/n> <http://example.org/enabled> true } WHERE {}" .
        "#;
        for profile in DATED {
            assert_eq!(
                infer(profile, conditional).unwrap().inferred(),
                [] as [[crate::term::Term; 3]; 0]
            );
            let reached = format!("{conditional}{enabling}");
            rule_refusal(profile, &reached, QueryPurpose::ConstructRule);
            for inactive in [
                reached.replace(
                    "ex:R a sh:SPARQLRule;",
                    "ex:R a sh:SPARQLRule; sh:deactivated true;",
                ),
                reached.replace("sh:targetNode ex:n;", ""),
                reached.replace(
                    "ex:S a sh:NodeShape;",
                    "ex:S a sh:NodeShape; sh:deactivated true;",
                ),
            ] {
                assert_eq!(infer(profile, &inactive).unwrap().inferred().len(), 1);
            }
            let mint_once = r#"
                ex:Once a sh:SPARQLRule; sh:runOnce true;
                    sh:construct "CONSTRUCT { <http://example.org/s> <http://example.org/p> [] } WHERE {}" .
            "#;
            assert_eq!(infer(profile, mint_once).unwrap().inferred().len(), 1);
        }
    }

    #[test]
    fn a_dated_unselected_global_rule_has_no_execution_refusal() {
        let graph = r#"
            ex:T a sh:SPARQLRuleTemplate;
                sh:parameter [ sh:path ex:optional; sh:optional true ];
                sh:construct "CONSTRUCT { ?s <http://example.org/q> ?optional } WHERE { ?s <http://example.org/p> ?o BIND(true AS ?optional) }" .
            ex:Invalid a ex:T; sh:runOnce true .
            ex:Good a sh:SPARQLRule; sh:runOnce true;
                sh:construct "CONSTRUCT { <http://example.org/s> <http://example.org/p> true } WHERE {}" .
            ex:Chosen a sh:RuleSet; sh:hasRule ex:Good .
        "#;
        for profile in DATED {
            let result = in_query_scope(profile, graph, |shapes| {
                let data = purrdf_rdf::RdfDatasetBuilder::new().freeze().unwrap();
                let data = crate::data::ShaclData::new(Arc::clone(&data), data, None);
                let options = crate::rules::RuleOptions::default().with_rule_set(
                    crate::term::NamedNode::new_unchecked("http://example.org/Chosen"),
                );
                crate::rules::infer(&data, &shapes, &options)
                    .map_err(crate::report::CompleteValidationError::Execution)
            })
            .unwrap();
            assert_eq!(result.inferred().len(), 1);
            rule_refusal(profile, graph, QueryPurpose::GlobalConstructRule);
        }
    }

    #[test]
    fn dated_global_bgp_executes_empty_and_nonempty_data_and_obeys_actual_query_fuel() {
        let graph = r#"
            ex:R a sh:SPARQLRule; sh:runOnce true;
                sh:construct "CONSTRUCT { ?s <http://example.org/q> ?o } WHERE { ?s <http://example.org/p> ?o }" .
        "#;
        for profile in DATED {
            in_query_scope(profile, graph, |shapes| {
                for (triples, expected) in [("", 0), ("ex:n ex:p true .", 1)] {
                    let data = crate::text_ingest::parse_turtle_to_dataset(
                        &format!("{PREFIXES}{triples}"),
                        None,
                    )
                    .unwrap();
                    let data = crate::data::ShaclData::new(Arc::clone(&data), data, None);
                    let options = crate::rules::RuleOptions::default();
                    let inference = crate::rules::infer(&data, &shapes, &options)
                        .map_err(crate::report::CompleteValidationError::Execution)?;
                    assert_eq!(inference.inferred().len(), expected);
                    let state = Arc::new(purrdf_sparql_eval::GovernorState::new(
                        &purrdf_sparql_eval::QueryGovernors::UNBOUNDED.with_fuel(0),
                    ));
                    let _governors = crate::sparql::enter_governor_scope(Arc::clone(&state));
                    crate::rules::infer(&data, &shapes, &options)
                        .expect_err("the authored query still consumes the operation's fuel");
                    assert!(crate::report::CompleteValidationError::resource(&state).is_some());
                    assert!(
                        crate::query_law::current()
                            .unwrap()
                            .take_failure()
                            .is_none()
                    );
                }
                Ok(())
            })
            .unwrap();
        }
    }

    #[test]
    fn rec_audits_unreached_constraint_declarations_but_not_unrelated_selects() {
        let declaration = r#"
            [] sh:sparql [ sh:select "SELECT $this WHERE { VALUES ?local { 1 } }" ] .
        "#;
        let rejected = refusal(DATED[0], declaration, QueryPurpose::SelectConstraint);
        assert_eq!(rejected.reason(), AdmissionReason::Values);
        parse(DATED[1], declaration).expect("the draft restriction applies to executed queries");
        parse(ShaclProfile::LEGACY, declaration).expect("compatibility discovery is unchanged");
        parse(
            DATED[0],
            &declaration.replace("VALUES ?local { 1 }", "FILTER(BOUND($this))"),
        )
        .expect("a valid recognized constraint declaration is admitted");
        parse(
            DATED[0],
            r#"ex:Other sh:select "SELECT ?v WHERE { VALUES ?v { 1 } }" ."#,
        )
        .expect("an unrelated select literal is not a constraint declaration");
        let typed = r#"
            ex:C a sh:SPARQLConstraint;
                sh:select "SELECT $this WHERE { VALUES ?local { 1 } }" .
        "#;
        let rejected = refusal(DATED[0], typed, QueryPurpose::SelectConstraint);
        assert_eq!(rejected.reason(), AdmissionReason::Values);
        parse(DATED[1], typed).expect("an unattached draft constraint is not executed");
        parse(
            DATED[0],
            &typed.replace("VALUES ?local { 1 }", "FILTER(BOUND($this))"),
        )
        .expect("a valid standalone typed constraint is admitted");
    }

    #[test]
    fn rec_rejects_prebinding_in_a_superseded_native_validator() {
        let declaration = r#"
            sh:MinCountConstraintComponent a sh:ConstraintComponent;
                sh:parameter [ sh:path sh:minCount ];
                sh:propertyValidator [ a sh:SPARQLSelectValidator;
                    sh:select "SELECT $this WHERE { VALUES ?minCount { 1 } }" ] .
        "#;
        let rejected = refusal(DATED[0], declaration, QueryPurpose::SelectValidator);
        assert_eq!(rejected.reason(), AdmissionReason::Values);
        parse(DATED[1], declaration).expect("the native implementation supersedes this query");
        parse(ShaclProfile::LEGACY, declaration).expect("compatibility supersession is unchanged");
        parse(
            DATED[0],
            &declaration.replace("VALUES ?minCount { 1 }", "FILTER(?minCount)"),
        )
        .expect("a valid native alternative is admitted");
    }
}
