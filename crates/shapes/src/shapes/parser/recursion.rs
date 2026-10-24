// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SHACL Core §6.6.3 refers graph, before recursive model expansion.

use purrdf_core::graph::{GraphError, tarjan_scc_with_memory};
use purrdf_lex::allocation::{Memory, Resident};
use purrdf_rdf::FastMap;

use crate::error::{RecursionDependency, RecursionPolarity, RecursionRefusal, ShapesError};
use crate::model::sh;
use crate::shapes::Parser;
use crate::spec::ValueRule;
use crate::term::Term;

impl Parser<'_> {
    /// Reject a recursive catalog without unfolding any of its simple paths.
    pub(crate) fn check_shape_recursion(&self) -> Result<(), ShapesError> {
        // Refers is defined over every shape, not only shapes an explicit data
        // target can name (the latter intentionally excludes declarations).
        let shapes = self.spec_shapes();
        let index: FastMap<&Term, usize> = shapes
            .iter()
            .enumerate()
            .map(|(index, shape)| (shape, index))
            .collect();
        let destination = |target: &Term| {
            index.get(target).copied().ok_or_else(|| {
                ShapesError::Invalid(format!(
                    "shape reference {target} is absent from the shapes graph's shape census"
                ))
            })
        };
        let mut adjacency = vec![Vec::new(); shapes.len()];
        let mut dependencies = Vec::new();
        // The table is authoritative about shape-expecting parameters, including
        // memberShape, someValue and reifierShape. Qualified min/max share one
        // parameter; visiting it once preserves the combined polarity.
        let mut parameters = Vec::new();
        for component in crate::spec::implemented().components() {
            for parameter in component.params() {
                if matches!(parameter.value, ValueRule::Shape | ValueRule::ShapeList)
                    && !parameters.iter().any(|&(path, _)| path == parameter.path)
                {
                    parameters.push((parameter.path, parameter.value));
                }
            }
        }
        parameters.push((sh::TARGET_WHERE, ValueRule::Shape));
        for (source, shape) in shapes.iter().enumerate() {
            for &(predicate, value_rule) in &parameters {
                for object in self.objects_of(shape, predicate) {
                    let targets = if matches!(value_rule, ValueRule::ShapeList) {
                        self.walk_rdf_list(&object, shape)?
                    } else {
                        vec![object]
                    };
                    let polarity = self.shape_dependency_polarity(shape, predicate);
                    for target in targets {
                        let target_index = destination(&target)?;
                        adjacency[source].push(target_index);
                        dependencies.push((source, target_index, predicate, polarity));
                        if predicate == sh::QUALIFIED_VALUE_SHAPE
                            && self
                                .first_object_of(shape, sh::QUALIFIED_VALUE_SHAPES_DISJOINT)
                                .as_ref()
                                .and_then(super::node_expr::boolean_value)
                                == Some(true)
                        {
                            // Count(v : own(v) && !sibling(v)): a lower bound is
                            // antitone in a sibling, an upper bound monotone.
                            // Both bounds together are nonmonotone. These are
                            // implicit evaluation dependencies in addition to
                            // the explicit §6.6.3 refers edges.
                            let sibling_polarity = match polarity {
                                RecursionPolarity::Positive => RecursionPolarity::Negative,
                                RecursionPolarity::Negative => RecursionPolarity::Positive,
                                other => other,
                            };
                            for sibling in self.sibling_qualified_shape_nodes(shape, &target) {
                                let sibling_index = destination(&sibling)?;
                                adjacency[source].push(sibling_index);
                                dependencies.push((
                                    source,
                                    sibling_index,
                                    sh::QUALIFIED_VALUE_SHAPES_DISJOINT,
                                    sibling_polarity,
                                ));
                            }
                        }
                    }
                }
            }
            adjacency[source].sort_unstable();
            adjacency[source].dedup();
        }
        let mut components = tarjan_scc_with_memory(&adjacency, &mut Memory::new(&mut Resident))
            .map_err(|error| match error {
                GraphError::Storage(error) => ShapesError::Storage(error),
                error @ GraphError::InvalidEdge { .. } => ShapesError::Invalid(error.to_string()),
            })?;
        for component in &mut components {
            component.sort_unstable();
        }
        components.sort();
        for component in components {
            if component.len() == 1 && !adjacency[component[0]].contains(&component[0]) {
                continue;
            }
            let members = component.iter().map(|&i| shapes[i].to_string()).collect();
            let mut edges: Vec<_> = dependencies
                .iter()
                .filter(|(from, to, _, _)| {
                    component.binary_search(from).is_ok() && component.binary_search(to).is_ok()
                })
                .map(|&(from, to, predicate, polarity)| RecursionDependency {
                    source: shapes[from].to_string(),
                    target: shapes[to].to_string(),
                    parameter: predicate.to_owned(),
                    polarity,
                })
                .collect();
            edges.sort();
            edges.dedup();
            return Err(ShapesError::Recursion(RecursionRefusal { members, edges }));
        }
        Ok(())
    }

    fn shape_dependency_polarity(&self, shape: &Term, predicate: &str) -> RecursionPolarity {
        match predicate {
            sh::NOT => RecursionPolarity::Negative,
            sh::XONE => RecursionPolarity::Both,
            // Where-target membership does not enter the conformance formula.
            // Keep it in the normative refers graph without claiming monotonicity
            // of a constraint that does not exist.
            sh::TARGET_WHERE => RecursionPolarity::Target,
            sh::QUALIFIED_VALUE_SHAPE => {
                let min = self
                    .first_object_of(shape, sh::QUALIFIED_MIN_COUNT)
                    .is_some();
                let max = self
                    .first_object_of(shape, sh::QUALIFIED_MAX_COUNT)
                    .is_some();
                match (min, max) {
                    (true, true) => RecursionPolarity::Both,
                    (false, true) => RecursionPolarity::Negative,
                    _ => RecursionPolarity::Positive,
                }
            }
            _ => RecursionPolarity::Positive,
        }
    }
}
