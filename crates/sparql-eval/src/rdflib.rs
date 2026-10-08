// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Compile contextual mappings into immutable aliases and shared native operators.

use std::collections::{BTreeMap, BTreeSet};

use purrdf_sparql_algebra::algebra::{ApplicationPolicy, OptionalApplication};
use purrdf_sparql_algebra::tree::Child;
use purrdf_sparql_algebra::walk::{Flow, NodeRef, Visit, fold_post_order, walk_pre_post};
use purrdf_sparql_algebra::{
    AggregateExpression, Expression, GraphPattern, GroundTerm, Literal, NamedNode,
    NamedNodePattern, OrderExpression, Query, TermPattern, TriplePattern, Variable,
};
use purrdf_xsd::datatype::XSD_BOOLEAN;

/// Materialize the compatibility query's graph selection as one RDF value set.
/// Named source graphs remain available for a dataset-scoped query; the selected
/// graph or union is also projected into the default graph. The ordinary query
/// entry does not call this materialization boundary.
///
/// # Errors
/// Returns the native diagnostic when declaration or freeze fails.
pub fn select_query_dataset(
    dataset: &std::sync::Arc<purrdf_core::RdfDataset>,
    dataset_scope: bool,
    default_graph: Option<&purrdf_core::TermValue>,
    default_union: bool,
) -> Result<std::sync::Arc<purrdf_core::RdfDataset>, purrdf_core::RdfDiagnostic> {
    use purrdf_core::{
        DatasetMut,
        ir::{MutableDataset, QuadValues},
    };
    // Even a default-only source can hold equal values in different physical
    // statement tables. Compatibility graphs expose each RDF value once.
    let surface = QuadValues::surface_of(dataset);
    let mut emptied = MutableDataset::new(std::sync::Arc::clone(dataset));
    if let Some(graph) = default_graph {
        // Reuse the core graph-name admission; the temporary slot is withdrawn
        // below, so a valid missing selector never fabricates a source graph.
        emptied.declare_named_graph(graph.clone())?;
    }
    for quad in &surface {
        emptied.remove(quad);
    }
    emptied.withdraw_named_graph_declarations();
    // Keep the original content-addressing configuration in an empty frozen base.
    // Fresh delta insertion cannot resurrect excluded physical base occurrences.
    let mut selected = MutableDataset::new(emptied.freeze()?);
    let insertion_error = |error: purrdf_iri::IriError| {
        purrdf_core::RdfDiagnostic::error(error.diagnostic_code(), error.to_string())
    };
    if dataset_scope {
        for graph in dataset.named_graphs() {
            selected.declare_named_graph(dataset.as_ref().term_value(graph))?;
        }
    }
    for quad in surface {
        if dataset_scope && quad.g.is_some() || default_graph.is_none() && quad.g.is_none() {
            selected.insert(quad.clone()).map_err(insertion_error)?;
        }
        if default_union || (quad.g.as_ref() == default_graph && default_graph.is_some()) {
            selected
                .insert(QuadValues::triple(quad.s, quad.p, quad.o))
                .map_err(insertion_error)?;
        }
    }
    selected.freeze()
}

type Mapping = BTreeMap<Variable, Variable>;

/// Shared authored policies for native visitor and admission controls.
#[cfg(test)]
// Apply's policy slot is boxed; this fixture home constructs that complete slot
// once rather than making every independent visitor fixture repack the policy.
#[allow(clippy::unnecessary_box_returns)]
pub(crate) fn test_application_policy(
    inputs: Vec<(Variable, Variable)>,
    optional: bool,
) -> Box<ApplicationPolicy> {
    Box::new(ApplicationPolicy {
        dataset_required: false,
        row_pipeline: false,
        reduced_adjacent: false,
        group_domain: None,
        optional: optional.then(|| OptionalApplication {
            retry_inputs: inputs.clone(),
            forget_marker: Variable::new("remembered"),
        }),
        inputs,
    })
}

#[derive(Default)]
struct Facts {
    names: BTreeSet<Variable>,
    lazy: bool,
    exists_root_filter: bool,
}

struct Compiled {
    pattern: GraphPattern,
    mapping: Mapping,
}

struct Compiler<const SINGLE_GRAPH: bool> {
    prefix: String,
    next: usize,
    context: Mapping,
    initial: Mapping,
    forget_marker: Option<Variable>,
    facts: BTreeMap<*const GraphPattern, Facts>,
    built: BTreeMap<*const GraphPattern, Compiled>,
}

fn value(variable: &Variable) -> Expression {
    Expression::Variable(variable.clone())
}

fn boolean(truth: bool) -> Expression {
    Expression::Literal(Literal::new_typed(
        if truth { "true" } else { "false" },
        NamedNode::new_unchecked(XSD_BOOLEAN),
    ))
}

fn bound(variable: &Variable) -> Expression {
    Expression::Bound(variable.clone())
}

fn coalesce(values: impl IntoIterator<Item = Expression>) -> Expression {
    Expression::Coalesce(values.into_iter().collect())
}

fn any(mapping: &Mapping) -> Expression {
    mapping
        .values()
        .map(bound)
        .reduce(Expression::or)
        .unwrap_or_else(|| boolean(false))
}

fn extend(pattern: GraphPattern, variable: Variable, expression: Expression) -> GraphPattern {
    if let GraphPattern::Apply {
        left,
        right,
        policy,
    } = pattern
    {
        if policy.row_pipeline {
            return GraphPattern::Apply {
                left,
                right: Child::new(extend(right.into_inner(), variable, expression)),
                policy,
            };
        }
        return GraphPattern::Extend {
            inner: Child::new(GraphPattern::Apply {
                left,
                right,
                policy,
            }),
            variable,
            expression,
        };
    }
    GraphPattern::Extend {
        inner: Child::new(pattern),
        variable,
        expression,
    }
}

fn project(pattern: GraphPattern, mapping: &Mapping) -> GraphPattern {
    GraphPattern::Project {
        inner: Child::new(pattern),
        variables: mapping.values().cloned().collect(),
    }
}

fn apply(left: GraphPattern, right: GraphPattern, policy: ApplicationPolicy) -> GraphPattern {
    if let GraphPattern::Apply {
        left: driver,
        right: row,
        policy: mut previous,
    } = left
    {
        if previous.row_pipeline {
            previous.row_pipeline = false;
            return GraphPattern::Apply {
                left: driver,
                right: Child::new(apply(row.into_inner(), right, policy)),
                policy: previous,
            };
        }
        return GraphPattern::Apply {
            left: Child::new(GraphPattern::Apply {
                left: driver,
                right: row,
                policy: previous,
            }),
            right: Child::new(right),
            policy: Box::new(policy),
        };
    }
    GraphPattern::Apply {
        left: Child::new(left),
        right: Child::new(right),
        policy: Box::new(policy),
    }
}

fn row_operation(
    pattern: GraphPattern,
    mapping: &Mapping,
    operation: impl FnOnce(GraphPattern) -> GraphPattern,
) -> GraphPattern {
    if let GraphPattern::Apply {
        left,
        right,
        policy,
    } = pattern
    {
        if policy.row_pipeline {
            return GraphPattern::Apply {
                left,
                right: Child::new(operation(right.into_inner())),
                policy,
            };
        }
        return row_operation_base(
            GraphPattern::Apply {
                left,
                right,
                policy,
            },
            mapping,
            operation,
        );
    }
    row_operation_base(pattern, mapping, operation)
}

fn row_operation_base(
    pattern: GraphPattern,
    mapping: &Mapping,
    operation: impl FnOnce(GraphPattern) -> GraphPattern,
) -> GraphPattern {
    GraphPattern::Apply {
        left: Child::new(pattern),
        right: Child::new(operation(GraphPattern::empty_bgp())),
        policy: Box::new(ApplicationPolicy {
            dataset_required: false,
            row_pipeline: true,
            reduced_adjacent: false,
            group_domain: None,
            inputs: mapping
                .values()
                .map(|slot| (slot.clone(), slot.clone()))
                .collect(),
            optional: None,
        }),
    }
}

impl<const SINGLE_GRAPH: bool> Compiler<SINGLE_GRAPH> {
    fn fresh(&mut self) -> Variable {
        let variable = Variable::new(format!("{}{:016x}", self.prefix, self.next));
        self.next += 1;
        variable
    }

    fn take(&mut self, pattern: &GraphPattern) -> Compiled {
        self.built
            .remove(&std::ptr::from_ref::<GraphPattern>(pattern))
            .expect("post-order compilation produced the operand")
    }

    fn forgetting(&self, context: &Variable) -> Expression {
        self.forget_marker.as_ref().map_or_else(
            || bound(context),
            |marker| Expression::and(value(marker), bound(context)),
        )
    }

    fn lookup(
        &self,
        name: &Variable,
        mapping: &Mapping,
        except: Option<&BTreeSet<Variable>>,
    ) -> Expression {
        let mut choices = Vec::new();
        if let Some(slot) = mapping.get(name) {
            let current = value(slot);
            let current = if let Some(except) = except
                && !except.contains(name)
                && !self.initial.contains_key(name)
                && let Some(context) = self.context.get(name)
            {
                Expression::If(
                    Child::new(self.forgetting(context)),
                    Child::new(Expression::Coalesce(Vec::new().into())),
                    Child::new(current),
                )
            } else {
                current
            };
            choices.push(current);
        }
        if let Some(slot) = self.initial.get(name) {
            choices.push(value(slot));
        }
        coalesce(choices)
    }

    fn expression(
        &mut self,
        source: &Expression,
        mapping: &Mapping,
        except: Option<&BTreeSet<Variable>>,
    ) -> Expression {
        let mut expression = source.map_exists_bodies(|body| {
            let compiled = self.take(body);
            let mut driver = GraphPattern::empty_bgp();
            let mut visible = Mapping::new();
            for (name, source) in mapping {
                let expression = if let Some(except) = except
                    && !except.contains(name)
                    && !self.initial.contains_key(name)
                {
                    Expression::If(
                        Child::new(self.forgetting(&self.context[name])),
                        Child::new(Expression::Coalesce(Vec::new().into())),
                        Child::new(value(source)),
                    )
                } else {
                    value(source)
                };
                let slot = self.fresh();
                driver = extend(driver, slot.clone(), expression);
                visible.insert(name.clone(), slot);
            }
            let (driver, inputs) = self.thaw(driver, &visible, None);
            apply(
                driver,
                compiled.pattern,
                ApplicationPolicy {
                    dataset_required: false,
                    row_pipeline: false,
                    reduced_adjacent: false,
                    group_domain: None,
                    inputs,
                    optional: None,
                },
            )
        });
        crate::substitute::for_each_expression_mut([&mut expression], |node| match node {
            Expression::Variable(name) if self.context.contains_key(name) => {
                *node = self.lookup(name, mapping, except);
            }
            Expression::Bound(name) if self.context.contains_key(name) => {
                let present = mapping.get(name).map_or_else(|| boolean(false), bound);
                let present = if let Some(except) = except
                    && !except.contains(name)
                    && !self.initial.contains_key(name)
                {
                    Expression::and(
                        present,
                        Expression::Not(Child::new(self.forgetting(&self.context[name]))),
                    )
                } else {
                    present
                };
                *node = if self.initial.contains_key(name) {
                    boolean(true)
                } else {
                    present
                };
            }
            _ => {}
        });
        expression
    }

    fn thaw(
        &mut self,
        mut driver: GraphPattern,
        mapping: &Mapping,
        remembered: Option<&BTreeSet<Variable>>,
    ) -> (GraphPattern, Vec<(Variable, Variable)>) {
        let selected: Mapping = mapping
            .iter()
            .filter(|(name, _)| remembered.is_none_or(|names| names.contains(*name)))
            .map(|(name, slot)| (name.clone(), slot.clone()))
            .collect();
        let present = any(&selected);
        let marker = self.fresh();
        driver = extend(driver, marker.clone(), present);
        let context = self.context.clone();
        let mut inputs = Vec::new();
        for (name, input) in context {
            let fallback = value(&input);
            // A bound selected cell implies `marker` is true. Put that cell first
            // so certainty survives transport, while a missing cell still cannot
            // fall back when another selected cell makes the mapping nonempty.
            let choice = selected.get(&name).map_or_else(
                || {
                    Expression::If(
                        Child::new(value(&marker)),
                        Child::new(Expression::Coalesce(Vec::new().into())),
                        Child::new(fallback.clone()),
                    )
                },
                |actual| {
                    coalesce([
                        value(actual),
                        Expression::If(
                            Child::new(value(&marker)),
                            Child::new(Expression::Coalesce(Vec::new().into())),
                            Child::new(fallback.clone()),
                        ),
                    ])
                },
            );
            let choice = if let Some(initial) = self.initial.get(&name) {
                coalesce([value(initial), choice])
            } else {
                choice
            };
            let slot = self.fresh();
            driver = extend(driver, slot.clone(), choice);
            inputs.push((input, slot));
        }
        (driver, inputs)
    }

    fn aligned(&self, mut compiled: Compiled, names: &Mapping) -> Compiled {
        for (name, slot) in names {
            if let Some(source) = compiled.mapping.get(name) {
                compiled.pattern = extend(compiled.pattern, slot.clone(), value(source));
            }
        }
        Compiled {
            pattern: project(compiled.pattern, names),
            mapping: names.clone(),
        }
    }

    fn merge(&mut self, mut pattern: GraphPattern, left: &Mapping, right: &Mapping) -> Compiled {
        let mut mapping = Mapping::new();
        let names: BTreeSet<_> = left.keys().chain(right.keys()).cloned().collect();
        for name in names {
            let slot = self.fresh();
            let choices = left
                .get(&name)
                .into_iter()
                .chain(right.get(&name))
                .map(value);
            pattern = extend(pattern, slot.clone(), coalesce(choices));
            mapping.insert(name, slot);
        }
        Compiled {
            pattern: project(pattern, &mapping),
            mapping,
        }
    }

    fn leaf(&mut self, source: &GraphPattern) -> Compiled {
        let mut matched = Mapping::new();
        walk_pre_post(NodeRef::Pattern(source), |visit, node| {
            if visit == Visit::Enter {
                node.for_each_variable(|name| {
                    if !matched.contains_key(name) {
                        matched.insert(name.clone(), self.fresh());
                    }
                });
            }
            Flow::Descend
        });
        if matches!(source, GraphPattern::PropertyFunction(_)) {
            for (name, slot) in &mut matched {
                *slot = self.context[name].clone();
            }
        }
        let mut pattern = source.clone();
        fn rename(term: &mut TermPattern, mapping: &Mapping) {
            let mut pending = vec![term];
            while let Some(term) = pending.pop() {
                match term {
                    TermPattern::Variable(name) => *name = mapping[name].clone(),
                    TermPattern::Triple(triple) => {
                        let triple: &mut TriplePattern = triple;
                        pending.extend([&mut triple.object, &mut triple.subject]);
                        if let NamedNodePattern::Variable(name) = &mut triple.predicate {
                            *name = mapping[name].clone();
                        }
                    }
                    _ => {}
                }
            }
        }
        match &mut pattern {
            GraphPattern::Bgp { patterns } => {
                for triple in patterns {
                    rename(&mut triple.subject, &matched);
                    rename(&mut triple.object, &matched);
                    if let NamedNodePattern::Variable(name) = &mut triple.predicate {
                        *name = matched[name].clone();
                    }
                }
            }
            GraphPattern::Path {
                subject, object, ..
            } => {
                rename(subject, &matched);
                rename(object, &matched);
            }
            GraphPattern::Values { variables, .. } => {
                for name in variables {
                    *name = matched[name].clone();
                }
            }
            GraphPattern::PropertyFunction(call) => {
                for term in call.subject_args.iter_mut().chain(&mut call.object_args) {
                    rename(term, &matched);
                }
            }
            _ => unreachable!("only matching leaves enter leaf compilation"),
        }
        for (name, slot) in &matched {
            let context = &self.context[name];
            let allowed = Expression::or(
                Expression::Not(Child::new(bound(context))),
                Expression::or(
                    Expression::Not(Child::new(bound(slot))),
                    Expression::SameTerm(Child::new(value(context)), Child::new(value(slot))),
                ),
            );
            pattern = GraphPattern::Filter {
                inner: Child::new(pattern),
                expr: allowed,
            };
        }
        let context = self.context.clone();
        self.merge(pattern, &matched, &context)
    }

    fn build(&mut self, source: &GraphPattern) -> Compiled {
        use GraphPattern as G;
        match source {
            G::Bgp { .. } | G::Path { .. } | G::Values { .. } | G::PropertyFunction(_) => {
                self.leaf(source)
            }
            G::Extend {
                inner,
                variable,
                expression,
            } => {
                let mut compiled = self.take(inner);
                let facts = &self.facts[&std::ptr::from_ref::<GraphPattern>(source)]
                    .names
                    .clone();
                let expr = self.expression(expression, &compiled.mapping, Some(facts));
                let old = compiled.mapping.get(variable).cloned();
                let temporary = self.fresh();
                let target = self.fresh();
                let replacement =
                    coalesce(std::iter::once(value(&temporary)).chain(old.as_ref().map(value)));
                compiled.pattern = row_operation(compiled.pattern, &compiled.mapping, |row| {
                    extend(extend(row, temporary, expr), target.clone(), replacement)
                });
                compiled.mapping.insert(variable.clone(), target);
                compiled
            }
            G::Filter { inner, expr } => {
                let mut compiled = self.take(inner);
                let facts = self.facts[&std::ptr::from_ref::<GraphPattern>(source)]
                    .names
                    .clone();
                let isolated =
                    !self.facts[&std::ptr::from_ref::<GraphPattern>(source)].exists_root_filter;
                // A leading EXISTS filter reads its retained query context as
                // well as the operand's projected mapping. Keep that visibility
                // out of the logical returned domain and row-pipeline inputs.
                let visible = (!isolated).then(|| {
                    let mut visible = self.context.clone();
                    visible.extend(compiled.mapping.clone());
                    visible
                });
                let expr = self.expression(
                    expr,
                    visible.as_ref().unwrap_or(&compiled.mapping),
                    isolated.then_some(&facts),
                );
                compiled.pattern =
                    row_operation(compiled.pattern, &compiled.mapping, |row| G::Filter {
                        inner: Child::new(row),
                        expr,
                    });
                compiled
            }
            G::Join { left, right }
            | G::Lateral { left, right }
            | G::LeftJoin { left, right, .. } => {
                let left_compiled = self.take(left);
                let mut right_compiled = self.take(right);
                let contextual = self.facts[&std::ptr::from_ref::<GraphPattern>(source)].lazy;
                if matches!(source, G::Join { .. }) && !contextual {
                    let names: BTreeSet<_> = left_compiled
                        .mapping
                        .keys()
                        .chain(right_compiled.mapping.keys())
                        .cloned()
                        .collect();
                    let canonical: Mapping =
                        names.into_iter().map(|name| (name, self.fresh())).collect();
                    let left_compiled = self.aligned(left_compiled, &canonical);
                    let right_compiled = self.aligned(right_compiled, &canonical);
                    return Compiled {
                        pattern: G::Join {
                            left: Child::new(left_compiled.pattern),
                            right: Child::new(G::Distinct {
                                inner: Child::new(right_compiled.pattern),
                            }),
                        },
                        mapping: canonical,
                    };
                }
                let (driver, inputs) =
                    self.thaw(left_compiled.pattern, &left_compiled.mapping, None);
                let (driver, optional) = if let G::LeftJoin { expression, .. } = source {
                    let forget_marker = self.fresh();
                    self.forget_marker = Some(forget_marker.clone());
                    let condition = expression.as_ref().map_or_else(
                        || boolean(true),
                        |expr| {
                            self.expression(expr, &right_compiled.mapping, Some(&BTreeSet::new()))
                        },
                    );
                    self.forget_marker = None;
                    right_compiled.pattern =
                        row_operation(right_compiled.pattern, &right_compiled.mapping, |row| {
                            G::Filter {
                                inner: Child::new(row),
                                expr: condition,
                            }
                        });
                    let remembered = self.facts[&std::ptr::from_ref::<GraphPattern>(left)]
                        .names
                        .clone();
                    let (driver, retry_inputs) =
                        self.thaw(driver, &left_compiled.mapping, Some(&remembered));
                    (
                        driver,
                        Some(OptionalApplication {
                            retry_inputs,
                            forget_marker,
                        }),
                    )
                } else {
                    (driver, None)
                };
                let applied = apply(
                    driver,
                    right_compiled.pattern,
                    ApplicationPolicy {
                        dataset_required: false,
                        row_pipeline: false,
                        reduced_adjacent: false,
                        group_domain: None,
                        inputs,
                        optional,
                    },
                );
                self.merge(applied, &left_compiled.mapping, &right_compiled.mapping)
            }
            G::Minus { left, right } => {
                let left = self.take(left);
                let right = self.take(right);
                let names: BTreeSet<_> = left
                    .mapping
                    .keys()
                    .chain(right.mapping.keys())
                    .cloned()
                    .collect();
                let canonical: Mapping =
                    names.into_iter().map(|name| (name, self.fresh())).collect();
                let left = self.aligned(left, &canonical);
                let right = self.aligned(right, &canonical);
                Compiled {
                    pattern: G::Minus {
                        left: Child::new(left.pattern),
                        right: Child::new(G::Distinct {
                            inner: Child::new(right.pattern),
                        }),
                    },
                    mapping: canonical,
                }
            }
            G::Union { arms } => {
                let arms: Vec<_> = arms.iter().map(|arm| self.take(arm)).collect();
                let names: BTreeSet<_> = arms
                    .iter()
                    .flat_map(|arm| arm.mapping.keys())
                    .cloned()
                    .collect();
                let canonical: Mapping =
                    names.into_iter().map(|name| (name, self.fresh())).collect();
                let mut arms = arms
                    .into_iter()
                    .map(|arm| self.aligned(arm, &canonical).pattern);
                let first = arms.next().expect("a union has two or more arms");
                Compiled {
                    pattern: arms.fold(first, G::union),
                    mapping: canonical,
                }
            }
            G::Project { inner, variables } => {
                let compiled = self.take(inner);
                let mapping = variables
                    .iter()
                    .map(|name| {
                        (
                            name.clone(),
                            compiled
                                .mapping
                                .get(name)
                                .cloned()
                                .unwrap_or_else(|| self.fresh()),
                        )
                    })
                    .collect();
                Compiled {
                    pattern: project(compiled.pattern, &mapping),
                    mapping,
                }
            }
            G::Distinct { inner } => {
                let mut compiled = self.take(inner);
                compiled.pattern = G::Distinct {
                    inner: Child::new(project(compiled.pattern, &compiled.mapping)),
                };
                compiled
            }
            G::Reduced { inner } => {
                let mut compiled = self.take(inner);
                compiled.pattern = apply(
                    G::empty_bgp(),
                    G::Reduced {
                        inner: Child::new(project(compiled.pattern, &compiled.mapping)),
                    },
                    ApplicationPolicy {
                        dataset_required: false,
                        row_pipeline: false,
                        reduced_adjacent: true,
                        group_domain: None,
                        inputs: Vec::new(),
                        optional: None,
                    },
                );
                compiled
            }
            G::Slice {
                inner,
                start,
                length,
            } => {
                let mut compiled = self.take(inner);
                compiled.pattern = G::Slice {
                    inner: Child::new(compiled.pattern),
                    start: *start,
                    length: *length,
                };
                compiled
            }
            G::OrderBy { inner, expression } => {
                let mut compiled = self.take(inner);
                let expression = expression
                    .iter()
                    .map(|order| match order {
                        OrderExpression::Asc(expr) => {
                            OrderExpression::Asc(self.expression(expr, &compiled.mapping, None))
                        }
                        OrderExpression::Desc(expr) => {
                            OrderExpression::Desc(self.expression(expr, &compiled.mapping, None))
                        }
                    })
                    .collect();
                compiled.pattern = G::OrderBy {
                    inner: Child::new(compiled.pattern),
                    expression,
                };
                compiled
            }
            G::Group {
                inner,
                variables,
                aggregates,
            } => {
                let mut compiled = self.take(inner);
                let mut keys = Vec::new();
                for name in variables {
                    let slot = self.fresh();
                    let expr = self.lookup(name, &compiled.mapping, None);
                    compiled.pattern = extend(compiled.pattern, slot.clone(), expr);
                    keys.push(slot);
                }
                let mut mapping = Mapping::new();
                let mut lowered = Vec::new();
                for (name, aggregate) in aggregates {
                    let slot = self.fresh();
                    let args = aggregate
                        .args()
                        .iter()
                        .map(|expr| self.expression(expr, &compiled.mapping, None))
                        .collect();
                    let order =
                        aggregate
                            .order_by()
                            .iter()
                            .map(|order| match order {
                                OrderExpression::Asc(expr) => OrderExpression::Asc(
                                    self.expression(expr, &compiled.mapping, None),
                                ),
                                OrderExpression::Desc(expr) => OrderExpression::Desc(
                                    self.expression(expr, &compiled.mapping, None),
                                ),
                            })
                            .collect();
                    lowered.push((
                        slot.clone(),
                        AggregateExpression::new(
                            aggregate.function().clone(),
                            args,
                            aggregate.scalarvals().to_vec(),
                            order,
                            aggregate.distinct,
                        )
                        .expect("rewriting preserves aggregate arity"),
                    ));
                    mapping.insert(name.clone(), slot);
                }
                let domain: Box<[_]> = compiled.mapping.values().cloned().collect();
                let grouped = G::Group {
                    inner: Child::new(compiled.pattern),
                    variables: keys,
                    aggregates: lowered,
                };
                let grouped = apply(
                    G::empty_bgp(),
                    grouped,
                    ApplicationPolicy {
                        dataset_required: false,
                        row_pipeline: false,
                        reduced_adjacent: false,
                        group_domain: Some(domain),
                        inputs: Vec::new(),
                        optional: None,
                    },
                );
                let grouped = if variables.is_empty() {
                    grouped
                } else {
                    G::LeftJoin {
                        left: Child::new(G::empty_bgp()),
                        right: Child::new(grouped),
                        expression: None,
                    }
                };
                Compiled {
                    pattern: grouped,
                    mapping,
                }
            }
            G::Graph { name, inner } => {
                let mut compiled = self.take(inner);
                if SINGLE_GRAPH {
                    compiled.pattern = apply(
                        G::empty_bgp(),
                        G::Graph {
                            name: name.clone(),
                            inner: Child::new(compiled.pattern),
                        },
                        ApplicationPolicy {
                            dataset_required: true,
                            row_pipeline: false,
                            reduced_adjacent: false,
                            group_domain: None,
                            inputs: Vec::new(),
                            optional: None,
                        },
                    );
                    return compiled;
                }
                match name {
                    NamedNodePattern::NamedNode(_) => Compiled {
                        pattern: G::Graph {
                            name: name.clone(),
                            inner: Child::new(compiled.pattern),
                        },
                        mapping: compiled.mapping,
                    },
                    NamedNodePattern::Variable(name) => {
                        // Select the graph before the body without introducing its
                        // logical variable into context. An initially unbound graph
                        // variable is compatibility-joined onto the returned mapping.
                        let matched = self.fresh();
                        let mut driver = G::Graph {
                            name: NamedNodePattern::Variable(matched.clone()),
                            inner: Child::new(G::empty_bgp()),
                        };
                        let input = self.context[name].clone();
                        driver = G::Filter {
                            inner: Child::new(driver),
                            expr: Expression::or(
                                Expression::Not(Child::new(bound(&input))),
                                Expression::SameTerm(
                                    Child::new(value(&input)),
                                    Child::new(value(&matched)),
                                ),
                            ),
                        };
                        let selected = self.fresh();
                        let context = self.context.clone();
                        let mut inputs = Vec::new();
                        for (logical, input) in context {
                            let _ = logical;
                            let slot = self.fresh();
                            driver = extend(driver, slot.clone(), value(&input));
                            inputs.push((input, slot));
                        }
                        inputs.push((selected.clone(), matched));
                        let returned = compiled.mapping.get(name).cloned();
                        if let Some(returned) = &returned {
                            compiled.pattern = G::Filter {
                                inner: Child::new(compiled.pattern),
                                expr: Expression::or(
                                    bound(&input),
                                    Expression::or(
                                        Expression::Not(Child::new(bound(returned))),
                                        Expression::SameTerm(
                                            Child::new(value(returned)),
                                            Child::new(value(&selected)),
                                        ),
                                    ),
                                ),
                            };
                        }
                        let target = self.fresh();
                        compiled.pattern = extend(
                            compiled.pattern,
                            target.clone(),
                            Expression::If(
                                Child::new(bound(&input)),
                                Child::new(returned.as_ref().map_or_else(
                                    || Expression::Coalesce(Vec::new().into()),
                                    value,
                                )),
                                Child::new(returned.map_or_else(
                                    || value(&selected),
                                    |returned| coalesce(vec![value(&returned), value(&selected)]),
                                )),
                            ),
                        );
                        compiled.mapping.insert(name.clone(), target);
                        let body = G::Graph {
                            name: NamedNodePattern::Variable(selected),
                            inner: Child::new(compiled.pattern),
                        };
                        Compiled {
                            pattern: apply(
                                driver,
                                body,
                                ApplicationPolicy {
                                    dataset_required: false,
                                    row_pipeline: false,
                                    reduced_adjacent: false,
                                    group_domain: None,
                                    inputs,
                                    optional: None,
                                },
                            ),
                            mapping: compiled.mapping,
                        }
                    }
                }
            }
            G::Service {
                name,
                inner,
                silent,
            } => {
                // The endpoint executes its own query semantics. Send source
                // algebra plus the current context, never local application IR.
                // Context is an independent one-row bag, matching protocol VALUES.
                let mut carrier = G::empty_bgp();
                for (name, slot) in &self.context {
                    carrier = extend(carrier, name.clone(), value(slot));
                }
                let remote = G::Join {
                    left: Child::new((**inner).clone()),
                    right: Child::new(carrier),
                };
                let name = match name {
                    NamedNodePattern::Variable(name) => {
                        NamedNodePattern::Variable(self.context[name].clone())
                    }
                    name @ NamedNodePattern::NamedNode(_) => name.clone(),
                };
                let mut pattern = G::Service {
                    name,
                    inner: Child::new(remote),
                    silent: *silent,
                };
                let context = self.context.clone();
                let mut mapping = Mapping::new();
                for name in context.keys() {
                    let output = self.fresh();
                    pattern = extend(pattern, output.clone(), value(name));
                    mapping.insert(name.clone(), output);
                }
                Compiled {
                    pattern: project(pattern, &mapping),
                    mapping,
                }
            }
            G::Unfold {
                inner,
                expression,
                element,
                companion,
            } => {
                let mut compiled = self.take(inner);
                let expression = self.expression(expression, &compiled.mapping, None);
                let target = self.fresh();
                let other = companion.as_ref().map(|_| self.fresh());
                compiled.pattern = G::Unfold {
                    inner: Child::new(compiled.pattern),
                    expression,
                    element: target.clone(),
                    companion: other.clone(),
                };
                compiled.mapping.insert(element.clone(), target);
                if let Some(name) = companion {
                    compiled
                        .mapping
                        .insert(name.clone(), other.expect("the companion was allocated"));
                }
                compiled
            }
            G::Apply { .. } => unreachable!("compatibility admission returns source algebra"),
        }
    }
}

/// Compile an admitted source query with immutable initial bindings. No ordinary
/// query-variable prebinding pass may be applied to the returned algebra.
pub(crate) fn compile<const SINGLE_GRAPH: bool>(
    mut query: Query,
    bindings: &[(Variable, GroundTerm)],
) -> (Query, Option<Vec<String>>) {
    let mut head = query.pattern();
    let output_columns = loop {
        match head {
            GraphPattern::Project { variables, .. } => break variables.clone(),
            GraphPattern::Slice { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::OrderBy { inner, .. } => head = inner,
            _ => break purrdf_sparql_algebra::parser::visible_variables(head),
        }
    };
    let select = matches!(query, Query::Select { .. });
    let mut names: BTreeSet<Variable> = bindings.iter().map(|(name, _)| name.clone()).collect();
    walk_pre_post(NodeRef::Pattern(query.pattern()), |visit, node| {
        if visit == Visit::Enter {
            node.for_each_variable(|name| {
                names.insert(name.clone());
            });
        }
        Flow::Descend
    });
    query.for_each_variable(|name| {
        names.insert(name.clone());
    });
    let mut prefix = "__purrdf_mapping_".to_owned();
    while names.iter().any(|name| name.as_str().starts_with(&prefix)) {
        prefix.insert(0, '_');
    }
    let mut compiler = Compiler::<SINGLE_GRAPH> {
        prefix,
        next: 0,
        context: Mapping::new(),
        initial: Mapping::new(),
        forget_marker: None,
        facts: BTreeMap::new(),
        built: BTreeMap::new(),
    };
    for name in names {
        let slot = compiler.fresh();
        compiler.context.insert(name, slot);
    }
    for (name, _) in bindings {
        let slot = compiler.fresh();
        compiler.initial.insert(name.clone(), slot);
    }
    fold_post_order(
        NodeRef::Pattern(query.pattern()),
        |node, children: &mut dyn Iterator<Item = Facts>| {
            let mut children: Vec<_> = children.collect();
            let mut result = Facts {
                names: BTreeSet::new(),
                lazy: children.iter().all(|child| child.lazy),
                exists_root_filter: false,
            };
            if let NodeRef::Pattern(GraphPattern::Extend { .. }) = node {
                children.truncate(1);
            }
            for child in children {
                result.names.extend(child.names);
            }
            node.for_each_variable(|name| {
                result.names.insert(name.clone());
            });
            if matches!(
                node,
                NodeRef::Expr(
                    Expression::Equal(..)
                        | Expression::Greater(..)
                        | Expression::GreaterOrEqual(..)
                        | Expression::Less(..)
                        | Expression::LessOrEqual(..)
                        | Expression::In(..)
                )
            ) {
                result.names.clear();
            }
            if matches!(node, NodeRef::Expr(Expression::Exists(_))) {
                result.lazy = true;
            }
            if let NodeRef::Pattern(pattern) = node {
                compiler.facts.insert(
                    std::ptr::from_ref::<GraphPattern>(pattern),
                    Facts {
                        names: result.names.clone(),
                        lazy: result.lazy,
                        exists_root_filter: false,
                    },
                );
                if matches!(
                    pattern,
                    GraphPattern::Join { .. }
                        | GraphPattern::Slice { .. }
                        | GraphPattern::Distinct { .. }
                ) {
                    result.lazy = false;
                }
            }
            result
        },
    );
    let mut exists_depth = 0usize;
    walk_pre_post(NodeRef::Pattern(query.pattern()), |visit, node| {
        if visit == Visit::Enter
            && let NodeRef::Expr(Expression::Exists(body)) = node
            && matches!(&**body, GraphPattern::Filter { .. })
        {
            compiler
                .facts
                .get_mut(&std::ptr::from_ref::<GraphPattern>(body))
                .expect("every source node was summarized")
                .exists_root_filter = true;
        }
        if matches!(node, NodeRef::Expr(Expression::Exists(_))) {
            match visit {
                Visit::Enter => exists_depth += 1,
                Visit::Exit => exists_depth -= 1,
            }
        }
        if visit == Visit::Enter
            && exists_depth != 0
            && let NodeRef::Pattern(pattern @ GraphPattern::Join { .. }) = node
        {
            // EXISTS holds translated executable graph data separately from the
            // parse-tree summary; these joins have no lazy annotation.
            compiler
                .facts
                .get_mut(&std::ptr::from_ref::<GraphPattern>(pattern))
                .expect("every source node was summarized")
                .lazy = false;
        }
        Flow::Descend
    });
    walk_pre_post(NodeRef::Pattern(query.pattern()), |visit, node| {
        if visit == Visit::Enter && matches!(node, NodeRef::Pattern(GraphPattern::Service { .. })) {
            // The protocol owns this complete source subtree. Copy it once at
            // the service boundary; do not lower and discard its descendants.
            return Flow::Skip;
        }
        if visit == Visit::Exit
            && let NodeRef::Pattern(pattern) = node
        {
            let compiled = compiler.build(pattern);
            compiler
                .built
                .insert(std::ptr::from_ref::<GraphPattern>(pattern), compiled);
        }
        Flow::Descend
    });
    let mut compiled = compiler.take(query.pattern());
    if matches!(query, Query::Ask { .. }) {
        // ASK consumes one yielded witness; empty mappings are witnesses too.
        compiled.pattern = GraphPattern::Slice {
            inner: Child::new(compiled.pattern),
            start: 0,
            length: Some(1),
        };
    }
    if matches!(query, Query::Select { .. }) {
        compiled.pattern = GraphPattern::Filter {
            inner: Child::new(compiled.pattern),
            expr: any(&compiled.mapping),
        };
    }
    let mut public = Mapping::new();
    let mut output_names: BTreeSet<_> = compiled.mapping.keys().cloned().collect();
    if !matches!(query, Query::Select { .. } | Query::Ask { .. }) {
        query.for_each_variable(|name| {
            output_names.insert(name.clone());
        });
    }
    for name in &output_names {
        let expression = compiler.lookup(name, &compiled.mapping, None);
        compiled.pattern = extend(compiled.pattern, name.clone(), expression);
        public.insert(name.clone(), name.clone());
    }
    let output = project(compiled.pattern, &public);
    let seed = GraphPattern::Values {
        variables: bindings
            .iter()
            .map(|(name, _)| compiler.initial[name].clone())
            .collect(),
        bindings: vec![
            bindings
                .iter()
                .map(|(_, value)| Some(value.clone()))
                .collect(),
        ],
    };
    let inputs = compiler
        .context
        .iter()
        .map(|(name, input)| {
            (
                input.clone(),
                compiler
                    .initial
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| input.clone()),
            )
        })
        .chain(
            compiler
                .initial
                .values()
                .map(|slot| (slot.clone(), slot.clone())),
        )
        .collect();
    let mut output = apply(
        seed,
        output,
        ApplicationPolicy {
            dataset_required: false,
            row_pipeline: false,
            reduced_adjacent: false,
            group_domain: None,
            inputs,
            optional: None,
        },
    );
    let mut projection_names = None;
    let columns = if select {
        let mut seen = BTreeSet::new();
        output_columns
            .iter()
            .map(|name| {
                if seen.insert(name.clone()) {
                    return name.clone();
                }
                projection_names.get_or_insert_with(|| {
                    output_columns
                        .iter()
                        .map(|v| v.as_str().to_owned())
                        .collect()
                });
                let slot = compiler.fresh();
                output = extend(
                    std::mem::replace(&mut output, GraphPattern::empty_bgp()),
                    slot.clone(),
                    value(name),
                );
                slot
            })
            .collect()
    } else {
        public.values().cloned().collect()
    };
    let output = GraphPattern::Project {
        inner: Child::new(output),
        variables: columns,
    };
    match &mut query {
        Query::Select { pattern, .. }
        | Query::Ask { pattern, .. }
        | Query::Construct { pattern, .. }
        | Query::Describe { pattern, .. } => *pattern = output,
    }
    (query, projection_names)
}

#[cfg(test)]
mod tests {
    #[test]
    fn contextual_thaw_preserves_reachable_value_error_and_initial_choices() {
        use super::*;
        use crate::{eval::EvalCtx, expr::eval_expr, scratch::SolutionTerm, solution::VarSchema};
        use purrdf_core::{BlankScope, RdfDatasetBuilder};

        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_blank("scoped", BlankScope(37));
        let predicate = builder.intern_iri("http://example.org/p");
        let object = builder.intern_iri("http://example.org/o");
        let quoted = builder.intern_triple(subject, predicate, object);
        let yes = builder.intern_literal(purrdf_core::RdfLiteral::typed("true", XSD_BOOLEAN));
        let no = builder.intern_literal(purrdf_core::RdfLiteral::typed("false", XSD_BOOLEAN));
        let dataset = builder.freeze().unwrap();
        let name = Variable::new("name");
        let other = Variable::new("other");
        let actual = Variable::new("actual");
        let other_actual = Variable::new("other_actual");
        let input = Variable::new("input");
        let initial = Variable::new("initial");
        let marker = Variable::new("transport0000000000000000");
        let schema = VarSchema::from_vars([
            actual.clone(),
            input.clone(),
            marker.clone(),
            initial.clone(),
        ]);
        for remember_name in [false, true] {
            for has_initial in [false, true] {
                let mut compiler = Compiler::<false> {
                    prefix: "transport".to_owned(),
                    next: 0,
                    context: Mapping::from([(name.clone(), input.clone())]),
                    initial: if has_initial {
                        Mapping::from([(name.clone(), initial.clone())])
                    } else {
                        Mapping::new()
                    },
                    forget_marker: None,
                    facts: BTreeMap::new(),
                    built: BTreeMap::new(),
                };
                let mapping = Mapping::from([
                    (name.clone(), actual.clone()),
                    (other.clone(), other_actual.clone()),
                ]);
                let remembered = if remember_name {
                    BTreeSet::from([name.clone(), other.clone()])
                } else {
                    BTreeSet::from([other.clone()])
                };
                let (driver, _) =
                    compiler.thaw(GraphPattern::empty_bgp(), &mapping, Some(&remembered));
                let GraphPattern::Extend { expression, .. } = driver else {
                    panic!("transport extend")
                };
                for actual_bound in [false, true] {
                    for other_bound in [false, true] {
                        for fallback_bound in [false, true] {
                            for initial_bound in [false, true] {
                                let present = (remember_name && actual_bound) || other_bound;
                                let original = Expression::If(
                                    Child::new(value(&marker)),
                                    Child::new(if remember_name {
                                        value(&actual)
                                    } else {
                                        coalesce([])
                                    }),
                                    Child::new(value(&input)),
                                );
                                let original = if has_initial {
                                    coalesce([value(&initial), original])
                                } else {
                                    original
                                };
                                let row = [
                                    actual_bound.then_some(SolutionTerm::Existing(quoted)),
                                    fallback_bound.then_some(SolutionTerm::Existing(subject)),
                                    Some(SolutionTerm::Existing(if present { yes } else { no })),
                                    initial_bound.then_some(SolutionTerm::Existing(object)),
                                ];
                                let mut ctx = EvalCtx::new(&dataset);
                                let expected =
                                    eval_expr(&original, &row, &schema, &mut ctx).unwrap();
                                let received =
                                    eval_expr(&expression, &row, &schema, &mut ctx).unwrap();
                                assert_eq!(received, expected);
                                let exact = if has_initial && initial_bound {
                                    Some(object)
                                } else if remember_name && actual_bound {
                                    Some(quoted)
                                } else if !present && fallback_bound {
                                    Some(subject)
                                } else {
                                    None
                                };
                                assert_eq!(received, exact.map(SolutionTerm::Existing));
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn deep_compilation_clone_and_drop_use_iterative_homes() {
        purrdf_stack::on_stack(128 * 1024, || {
            use purrdf_sparql_algebra::walk::{Flow, NodeRef, Visit, walk_pre_post};
            let mut body = "BIND(1 AS ?x)".to_owned();
            for _ in 0..1024 {
                body = format!("FILTER EXISTS {{ {body} }}");
            }
            let text = format!("ASK {{ {body} }}");
            let source = purrdf_sparql_algebra::SparqlParser::new()
                .parse_rdflib_query_with(&text, &purrdf_sparql_algebra::ParserOptions::default())
                .unwrap();
            let (query, _) = super::compile::<false>(source, &[]);
            let mut nodes = 0;
            walk_pre_post(NodeRef::Pattern(query.pattern()), |visit, _| {
                if visit == Visit::Enter {
                    nodes += 1;
                }
                Flow::Descend
            });
            assert!(nodes > 1024 && nodes < 100_000);
            assert!(query.retained_size_bytes() < 16 * 1024 * 1024);
            let clone = query.clone();
            assert_eq!(query, clone);
            drop(clone);
        })
        .unwrap();
    }
}
