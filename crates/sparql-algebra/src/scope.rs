// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Binding-role invariants for non-distinguished match identities.
//!
//! A hidden variable is a match witness, not an expression input or an explicitly
//! observable output. Its identity may connect match positions across a generated
//! `UNION`, including internal `VALUES` bindings. Ordinary variables, pattern
//! blanks, concrete dataset blanks and template blanks keep their own contracts.
//! In particular, another internal NUL-prefixed namespace is not a match witness
//! merely because its name cannot be written in SPARQL.
//!
//! These borrowed checks establish roles in the supplied algebra. A lost intended
//! connection, a changed solution bag or incorrect runtime template allocation
//! requires transformation provenance or semantic evidence in addition to a tree.
//! Whole-tree role checks visit recursive children through the shared iterative
//! [`NodeRef`] walk. Source ownership and label-exposure visitors follow the
//! positive basic-graph-pattern spine over work lists as well; deeper input
//! consumes no additional machine stack.

use core::fmt;

use crate::walk::{Flow, NodeRef, Visit, walk_pre_post};
use crate::{
    Expression, GraphPattern, NamedNodePattern, QuadPattern, Query, TermPattern, Variable,
};

/// The binding invariant a checker refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ScopeHazard {
    /// A non-distinguished match identity was named in an observable role.
    HiddenObservation,
}

/// The position that explicitly observes a non-distinguished identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ObserverRole {
    /// A `SELECT` projection variable.
    Projection,
    /// A `GROUP BY` variable.
    GroupKey,
    /// The output variable of an aggregate.
    AggregateOutput,
    /// A `BIND` output variable.
    BindTarget,
    /// The element output of `UNFOLD`.
    UnfoldElement,
    /// The companion output of `UNFOLD`.
    UnfoldCompanion,
    /// An expression variable reference.
    ExpressionRead,
    /// The variable tested by `BOUND`.
    BoundTest,
    /// A template triple's subject, including a nested triple term's subject.
    TemplateSubject,
    /// A template triple's predicate, including a nested triple term's predicate.
    TemplatePredicate,
    /// A template triple's object, including a nested triple term's object.
    TemplateObject,
    /// A template's graph-name variable.
    TemplateGraph,
    /// A named `DESCRIBE` variable target.
    DescribeTarget,
}

impl ObserverRole {
    /// A description of this observer position suitable for a diagnostic.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Projection => "projection",
            Self::GroupKey => "group key",
            Self::AggregateOutput => "aggregate output",
            Self::BindTarget => "BIND target",
            Self::UnfoldElement => "UNFOLD element",
            Self::UnfoldCompanion => "UNFOLD companion",
            Self::ExpressionRead => "expression input",
            Self::BoundTest => "BOUND test",
            Self::TemplateSubject => "template subject",
            Self::TemplatePredicate => "template predicate",
            Self::TemplateObject => "template object",
            Self::TemplateGraph => "template graph",
            Self::DescribeTarget => "DESCRIBE target",
        }
    }
}

/// The part of a query in which a binding-role violation occurs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScopeRegion {
    /// The query's graph pattern, including expressions and subqueries.
    Pattern,
    /// An output template quad, including its nested triple terms and graph name.
    Template,
    /// An explicit `DESCRIBE` target.
    Description,
}

/// An address in borrowed algebra, independent of pointers and allocation order.
///
/// All indices are zero-based. `item` identifies a template quad or description
/// target, and is zero for a graph pattern. `node` is the owning node's preorder
/// index in the shared [`NodeRef`] walk. A template graph name is the synthetic
/// leaf immediately after its triple's walk; a description target uses node zero.
/// `slot` identifies the variable in an output list, or subject/predicate/object
/// position (`0`/`1`/`2`) in a template triple.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScopeSite {
    /// The query region containing the violation.
    pub region: ScopeRegion,
    /// The template quad or description target index; zero for a graph pattern.
    pub item: usize,
    /// The deterministic preorder index of the node owning the variable.
    pub node: usize,
    /// The variable's position within the named observer role.
    pub slot: usize,
}

impl ScopeSite {
    pub(crate) const fn pattern(node: usize) -> Self {
        Self {
            region: ScopeRegion::Pattern,
            item: 0,
            node,
            slot: 0,
        }
    }

    const fn at_slot(mut self, slot: usize) -> Self {
        self.slot = slot;
        self
    }
}

/// A typed, actionable refusal of a binding-role invariant.
///
/// The identity is cloned only on refusal; a successful check borrows the tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeError {
    /// The violated invariant.
    pub hazard: ScopeHazard,
    /// The position that observes the hidden binding.
    pub role: ObserverRole,
    /// The deterministic address of that position.
    pub site: ScopeSite,
    /// The offending canonical binding identity, including its hidden category.
    pub identity: Variable,
}

impl ScopeError {
    /// The concrete correction required at this observer position.
    pub const fn action(&self) -> &'static str {
        match self.role {
            ObserverRole::Projection | ObserverRole::GroupKey | ObserverRole::AggregateOutput => {
                "Keep the witness in match positions and output only distinguished variables."
            }
            ObserverRole::BindTarget
            | ObserverRole::UnfoldElement
            | ObserverRole::UnfoldCompanion => "Allocate a distinguished output variable.",
            ObserverRole::ExpressionRead | ObserverRole::BoundTest => {
                "Use a distinguished variable for expression input."
            }
            ObserverRole::TemplateSubject | ObserverRole::TemplateObject => {
                "Use a distinguished result variable or a blank allocated by the template rule."
            }
            ObserverRole::TemplatePredicate => "Use a distinguished predicate variable or an IRI.",
            ObserverRole::TemplateGraph => "Use a distinguished graph variable or a named graph.",
            ObserverRole::DescribeTarget => "Describe a distinguished result variable.",
        }
    }
}

impl fmt::Display for ScopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "a non-distinguished variable cannot be explicitly observed: {:?} in {} at {:?} item {} node {} slot {}; {}",
            self.identity,
            self.role.name(),
            self.site.region,
            self.site.item,
            self.site.node,
            self.site.slot,
            self.action(),
        )
    }
}

impl std::error::Error for ScopeError {}

impl From<ScopeError> for crate::ParseError {
    fn from(error: ScopeError) -> Self {
        Self::syntax(error.to_string(), 0)
    }
}

/// Check the match-witness observer contract in a graph pattern.
///
/// Match positions and internal match `VALUES` bindings may share hidden
/// identities, including across generated `UNION` arms. No ordinary variable-name
/// or term admission is performed by this identity-only check.
///
/// # Errors
/// Returns the first hidden observation in deterministic preorder.
pub fn validate_pattern(pattern: &GraphPattern) -> Result<(), ScopeError> {
    walk_nodes(NodeRef::Pattern(pattern), |node, index| {
        check_node(node, ScopeSite::pattern(index))
    })
}

/// Check a query's template or description targets, then its graph pattern.
///
/// # Errors
/// Returns a typed hidden observation with its query region and owning node.
pub fn validate_query(query: &Query) -> Result<(), ScopeError> {
    validate_query_head(query)?;
    validate_pattern(query.pattern())
}

/// Check every output slot of one template quad, including quoted triple terms.
///
/// The template has item index zero when checked independently. A whole-query
/// check reports its actual quad index instead.
///
/// # Errors
/// Refuses a hidden identity in a template subject, predicate, object or graph.
pub fn validate_quad(quad: &QuadPattern) -> Result<(), ScopeError> {
    validate_quad_at(quad, 0)
}

/// The head checks shared by full structural admission and identity-only queries.
pub(crate) fn validate_query_head(query: &Query) -> Result<(), ScopeError> {
    match query {
        Query::Construct { template, .. } => {
            for (index, quad) in template.iter().enumerate() {
                validate_quad_at(quad, index)?;
            }
        }
        Query::Describe { targets, .. } => {
            for (index, target) in targets.iter().enumerate() {
                if let NamedNodePattern::Variable(variable) = target {
                    check_identity(
                        variable,
                        ObserverRole::DescribeTarget,
                        ScopeSite {
                            region: ScopeRegion::Description,
                            item: index,
                            node: 0,
                            slot: 0,
                        },
                    )?;
                }
            }
        }
        Query::Select { .. } | Query::Ask { .. } => {}
    }
    Ok(())
}

fn validate_quad_at(quad: &QuadPattern, item: usize) -> Result<(), ScopeError> {
    let mut next_node = 0;
    walk_nodes(NodeRef::Triple(&quad.triple), |node, index| {
        next_node = index + 1;
        let site = ScopeSite {
            region: ScopeRegion::Template,
            item,
            node: index,
            slot: 0,
        };
        if let NodeRef::Triple(triple) = node {
            if let TermPattern::Variable(variable) = &triple.subject {
                check_identity(variable, ObserverRole::TemplateSubject, site)?;
            }
            if let NamedNodePattern::Variable(variable) = &triple.predicate {
                check_identity(variable, ObserverRole::TemplatePredicate, site.at_slot(1))?;
            }
            if let TermPattern::Variable(variable) = &triple.object {
                check_identity(variable, ObserverRole::TemplateObject, site.at_slot(2))?;
            }
        }
        Ok(())
    })?;
    if let Some(NamedNodePattern::Variable(variable)) = &quad.graph {
        check_identity(
            variable,
            ObserverRole::TemplateGraph,
            ScopeSite {
                region: ScopeRegion::Template,
                item,
                node: next_node,
                slot: 0,
            },
        )?;
    }
    Ok(())
}

/// A template's complete variable census, including quoted slots and graph name.
pub(crate) fn for_each_quad_variable(quad: &QuadPattern, mut visit: impl FnMut(&Variable)) {
    walk_pre_post(NodeRef::Triple(&quad.triple), |phase, node| {
        if phase == Visit::Enter {
            node.for_each_variable(&mut visit);
        }
        Flow::Descend
    });
    if let Some(NamedNodePattern::Variable(variable)) = &quad.graph {
        visit(variable);
    }
}

/// Apply the one identity rule to an observer node during another admission walk.
pub(crate) fn check_node(node: NodeRef<'_>, site: ScopeSite) -> Result<(), ScopeError> {
    match node {
        NodeRef::Pattern(GraphPattern::Project { variables, .. }) => {
            check_variables(variables, ObserverRole::Projection, site)?;
        }
        NodeRef::Pattern(GraphPattern::Group {
            variables,
            aggregates,
            ..
        }) => {
            check_variables(variables, ObserverRole::GroupKey, site)?;
            for (index, (variable, _)) in aggregates.iter().enumerate() {
                check_identity(variable, ObserverRole::AggregateOutput, site.at_slot(index))?;
            }
        }
        NodeRef::Pattern(GraphPattern::Extend { variable, .. }) => {
            check_identity(variable, ObserverRole::BindTarget, site)?;
        }
        NodeRef::Pattern(GraphPattern::Unfold {
            element, companion, ..
        }) => {
            check_identity(element, ObserverRole::UnfoldElement, site)?;
            if let Some(variable) = companion {
                check_identity(variable, ObserverRole::UnfoldCompanion, site)?;
            }
        }
        NodeRef::Expr(Expression::Variable(variable)) => {
            check_identity(variable, ObserverRole::ExpressionRead, site)?;
        }
        NodeRef::Expr(Expression::Bound(variable)) => {
            check_identity(variable, ObserverRole::BoundTest, site)?;
        }
        _ => {}
    }
    Ok(())
}

fn check_variables(
    variables: &[Variable],
    role: ObserverRole,
    site: ScopeSite,
) -> Result<(), ScopeError> {
    for (index, variable) in variables.iter().enumerate() {
        check_identity(variable, role, site.at_slot(index))?;
    }
    Ok(())
}

fn check_identity(
    variable: &Variable,
    role: ObserverRole,
    site: ScopeSite,
) -> Result<(), ScopeError> {
    if variable.is_hidden() {
        return Err(ScopeError {
            hazard: ScopeHazard::HiddenObservation,
            role,
            site,
            identity: variable.clone(),
        });
    }
    Ok(())
}

/// The shared numbered borrowed walk for observer and full structural admission.
pub(crate) fn walk_nodes<'a, E>(
    root: NodeRef<'a>,
    mut check: impl FnMut(NodeRef<'a>, usize) -> Result<(), E>,
) -> Result<(), E> {
    let mut result = Ok(());
    let mut index = 0;
    walk_pre_post(root, |phase, node| {
        if phase == Visit::Enter {
            result = check(node, index);
            if result.is_err() {
                return Flow::Stop;
            }
            index += 1;
        }
        Flow::Descend
    });
    result
}

// Source basic-graph-pattern ownership and exposure are shared with evaluation.

/// Whether `pattern` is a node joining two pieces of one basic graph pattern.
#[must_use]
pub fn joins_blank_scope(pattern: &GraphPattern) -> bool {
    match pattern {
        GraphPattern::Join { .. } => true,
        GraphPattern::Lateral { right, .. } => {
            matches!(&**right, GraphPattern::PropertyFunction(_))
        }
        _ => false,
    }
}

/// The leaves under the spine rooted at `pattern`, in written order.
///
/// A work list stands in for the recursion: a spine node's operands are pushed right
/// first, so the left pops first and the leaves come out left to right, however tall
/// the spine.
pub fn spine_leaves<'a>(pattern: &'a GraphPattern, out: &mut Vec<&'a GraphPattern>) {
    visit_spine_leaves(pattern, &mut |leaf| {
        out.push(leaf);
        false
    });
}

/// Whether any leaf under the spine rooted at `pattern` satisfies `test`.
///
/// Leaves are visited left to right, stopping at the first match. The work list
/// holds up to eight pending nodes inline; a deeper spine can spill to heap scratch
/// even when no labels need renaming. This walk runs on every admission, including
/// prepared re-runs.
pub fn visit_spine_leaves<'a>(
    pattern: &'a GraphPattern,
    test: &mut impl FnMut(&'a GraphPattern) -> bool,
) -> bool {
    let mut pending = crate::worklist::WorkList::<_, 8>::with(pattern);
    while let Some(node) = pending.pop() {
        match node {
            GraphPattern::Join { left, right } | GraphPattern::Lateral { left, right }
                if joins_blank_scope(node) =>
            {
                pending.push(right);
                pending.push(left);
            }
            _ => {
                if test(node) {
                    return true;
                }
            }
        }
    }
    false
}

/// Which identities a source-blank exposure walk includes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelSource {
    /// Ordinary existential blank labels local to a pattern leaf.
    Raw,
    /// Canonical hidden identities preserving a source block's blank labels.
    Carried,
    /// Both ordinary and carried identities.
    All,
}

/// Visit a leaf's labels, stopping when the visitor returns true. A UNION exposes
/// only carried parser-block identities, never its arms' local algebra blanks.
pub fn visit_leaf_labels<'a>(
    leaf: &'a GraphPattern,
    source: LabelSource,
    visit: &mut impl FnMut(&'a str) -> bool,
) -> bool {
    let mut pending = crate::worklist::WorkList::<_, 8>::with((leaf, source));
    while let Some((node, source)) = pending.pop() {
        let found = match node {
            GraphPattern::Bgp { patterns } => patterns.iter().any(|triple| {
                visit_term_labels(&triple.subject, source, visit)
                    || visit_term_labels(&triple.object, source, visit)
            }),
            GraphPattern::Path {
                subject, object, ..
            } => {
                visit_term_labels(subject, source, visit)
                    || visit_term_labels(object, source, visit)
            }
            GraphPattern::PropertyFunction(call) => call
                .subject_args
                .iter()
                .chain(&call.object_args)
                .any(|term| visit_term_labels(term, source, visit)),
            GraphPattern::Join { left, right } => {
                pending.push((right, source));
                pending.push((left, source));
                false
            }
            GraphPattern::Union { arms } if source != LabelSource::Raw => {
                pending.extend(arms.iter().rev().map(|arm| (arm, LabelSource::Carried)));
                false
            }
            _ => false,
        };
        if found {
            return true;
        }
    }
    false
}

/// The blank labels in a term, subject before object at every quoted level, over
/// an inline work list and with the same early-exit visitor as the leaf walk.
pub fn visit_term_labels<'a>(
    term: &'a TermPattern,
    source: LabelSource,
    visit: &mut impl FnMut(&'a str) -> bool,
) -> bool {
    let mut pending = crate::worklist::WorkList::<_, 8>::with(term);
    while let Some(term) = pending.pop() {
        match term {
            TermPattern::BlankNode(blank) if source != LabelSource::Carried => {
                if visit(blank.as_str()) {
                    return true;
                }
            }
            TermPattern::Variable(variable) if source != LabelSource::Raw => {
                if variable.source_blank_label().is_some_and(&mut *visit) {
                    return true;
                }
            }
            TermPattern::Triple(triple) => {
                pending.push(&triple.object);
                pending.push(&triple.subject);
            }
            TermPattern::NamedNode(_)
            | TermPattern::BlankNode(_)
            | TermPattern::Literal(_)
            | TermPattern::Variable(_) => {}
        }
    }
    false
}
