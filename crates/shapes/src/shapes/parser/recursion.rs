// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Signed refers graph over original Core shape definitions.

use purrdf_lex::allocation::{Memory, Resident, StorageError};
use purrdf_rdf::FastMap;
use std::sync::Arc;

use crate::error::{RecursionPolarity, ShapesError};
use crate::expression::{CustomFunction, NodeExpr};
use crate::model::sh;
use crate::product::ShapesProductError;
use crate::shapes::link::{ModelWalk, ShapeIndexRef};
use crate::shapes::{Constraint, ModelOwner, Parser};
use crate::spec::ValueRule;
use crate::term::Term;

/// A sufficient zero-allocation DAG proof for ordinary restored catalogs. Every
/// source edge must move strictly in one direction through the dataset's finite
/// term order. Lists, implicit sibling edges and expressions use the full graph.
pub(crate) fn source_rank_proves_acyclic(
    dataset: &purrdf_rdf::RdfDataset,
    owner: &Arc<ModelOwner>,
) -> bool {
    if !owner.functions.is_empty() {
        return false;
    }
    let mut expressions = ExpressionPresence::default();
    for definition in owner.definitions() {
        if expressions.walk_shape(definition).is_err() || expressions.found {
            return false;
        }
    }
    if dataset
        .term_id_by_iri(sh::QUALIFIED_VALUE_SHAPES_DISJOINT)
        .is_some()
    {
        return false;
    }
    let mut direction = None;
    let mut ordered = |predicate, list| {
        let Some(predicate) = dataset.term_id_by_iri(predicate) else {
            return true;
        };
        for quad in dataset.quads().filter(|quad| quad.p == predicate) {
            let order = quad.s.cmp(&quad.o);
            if list || order.is_eq() || direction.is_some_and(|previous| previous != order) {
                return false;
            }
            direction = Some(order);
        }
        true
    };
    for component in crate::spec::implemented().components() {
        for parameter in component.params() {
            if matches!(parameter.value, ValueRule::Shape | ValueRule::ShapeList)
                && !ordered(
                    parameter.path,
                    matches!(parameter.value, ValueRule::ShapeList),
                )
            {
                return false;
            }
        }
    }
    ordered(sh::TARGET_WHERE, false)
}

#[derive(Default)]
struct ExpressionPresence {
    depth: u32,
    found: bool,
}
impl<'model> ModelWalk<'model> for ExpressionPresence {
    fn depth(&mut self) -> &mut u32 {
        &mut self.depth
    }
    fn first_owner(&mut self, _: &'model Arc<ModelOwner>) -> Result<bool, ShapesProductError> {
        self.found = true;
        Ok(false)
    }
    fn first_declaration(
        &mut self,
        _: &'model Arc<CustomFunction>,
    ) -> Result<bool, ShapesProductError> {
        self.found = true;
        Ok(false)
    }
    fn shape_index(&mut self, _: &'model ShapeIndexRef) -> Result<(), ShapesProductError> {
        self.found = true;
        Ok(())
    }
    fn node_expr(&mut self, _: &'model NodeExpr) -> Result<(), ShapesProductError> {
        self.found = true;
        Ok(())
    }
}

impl Parser<'_> {
    /// Admit a signed original catalog without unfolding any simple path.
    pub(crate) fn check_shape_recursion(
        &self,
        owner: &Arc<ModelOwner>,
    ) -> Result<Option<purrdf_lex::allocation::Shared<crate::recursion::graph::Source>>, ShapesError>
    {
        // Refers is defined over every shape, not only shapes an explicit data
        // target can name (the latter intentionally excludes declarations).
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut shapes = Vec::new();
        for shape in owner.definitions() {
            let id = shape.id.try_clone()?;
            memory.push(&mut shapes, id)?;
        }
        crate::term::sort_terms_canonical(&mut shapes);
        let mut index: FastMap<&Term, usize> = FastMap::default();
        index
            .try_reserve(shapes.len())
            .map_err(|_| StorageError::AllocationFailed)?;
        index.extend(
            shapes
                .iter()
                .enumerate()
                .map(|(index, shape)| (shape, index)),
        );
        let destination = |target: &Term| {
            index.get(target).copied().ok_or_else(|| {
                ShapesError::Invalid(format!(
                    "shape reference {target} is absent from the shapes graph's shape census"
                ))
            })
        };
        let mut adjacency: Vec<Vec<usize>> =
            memory.collect((0..shapes.len()).map(|_| Vec::new()))?;
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
                    memory.push(&mut parameters, (parameter.path, parameter.value))?;
                }
            }
        }
        memory.push(&mut parameters, (sh::TARGET_WHERE, ValueRule::Shape))?;
        for (source, shape) in shapes.iter().enumerate() {
            for &(predicate, value_rule) in &parameters {
                for object in self.objects_of(shape, predicate) {
                    let targets = if matches!(value_rule, ValueRule::ShapeList) {
                        self.walk_rdf_list(&object, shape)?
                    } else {
                        memory.collect(std::iter::once(object))?
                    };
                    let mut polarity = self.shape_dependency_polarity(shape, predicate);
                    if predicate == sh::XONE {
                        let source = owner.definition_required(shape)?;
                        if source.constraints.iter().any(|constraint| {
                            let Constraint::Xone(operands) = constraint else {
                                return false;
                            };
                            operands
                                .iter()
                                .map(|operand| &operand.id)
                                .eq(targets.iter())
                                && crate::recursion::guards::xone_positive(
                                    source,
                                    operands,
                                    Some(owner),
                                )
                        }) {
                            polarity = RecursionPolarity::Positive;
                        }
                    }
                    for target in targets {
                        let target_index = destination(&target)?;
                        memory.push(&mut adjacency[source], target_index)?;
                        memory.push(
                            &mut dependencies,
                            (source, target_index, predicate, polarity),
                        )?;
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
                            let sibling_polarity = polarity.inverted();
                            for sibling in self.sibling_qualified_shape_nodes(shape, &target) {
                                let sibling_index = destination(&sibling)?;
                                memory.push(&mut adjacency[source], sibling_index)?;
                                memory.push(
                                    &mut dependencies,
                                    (
                                        source,
                                        sibling_index,
                                        sh::QUALIFIED_VALUE_SHAPES_DISJOINT,
                                        sibling_polarity,
                                    ),
                                )?;
                            }
                        }
                    }
                }
            }
        }
        for (source, _) in shapes.iter().enumerate() {
            adjacency[source].sort_unstable();
            adjacency[source].dedup();
        }
        let analysis = crate::recursion::graph::analyze(
            &adjacency,
            &dependencies,
            |node| Ok(shapes[node].try_spelling()?),
            |_| true,
            |node| {
                Ok(crate::recursion::closed::WeakDefinition::new(
                    &shapes[node],
                    Some(owner),
                    std::ptr::from_ref(owner.definition_required(&shapes[node])?).addr(),
                    crate::recursion::closed::RecursionDefinitionKind::Shape,
                )?)
            },
        )?;
        analysis.admit(self.recursion)?;
        // The native graph retains original model/occurrence identity and proves
        // selected closed eligibility. Source-only structural arcs stay separate.
        owner.admit_recursion(self.recursion)?;
        crate::recursion::graph::Source::new(owner, analysis)
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
