// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Post-parse structural well-formedness (ShEx 2.1 spec §5.7, "Schema
//! Requirements") — what the `negativeStructure` conformance suite checks.
//!
//! A schema that parses may still be rejected for:
//!
//! * **dangling references** — a `@label` shape-expression reference or
//!   `&label` triple-expression inclusion with no matching declaration;
//! * **label collisions** — a label declared twice, or used as both a
//!   shape-expression and a triple-expression label;
//! * **reference-only cycles** — a shape reference reachable from itself
//!   without ever crossing a triple constraint (`S1 = @S1 AND …`);
//! * **the negation requirement** — a negated reference (under `NOT`, or
//!   through a triple constraint whose predicate is `EXTRA` on the
//!   enclosing shape) inside a dependency-graph strongly-connected
//!   component. SCCs are computed with a hand-rolled iterative Tarjan.
//!
//! [`check_structure`] reports **all** violations, not just the first.

use std::collections::{HashMap, HashSet};

use core::fmt;

use crate::ast::{Schema, Shape, ShapeExpr, TripleExpr};

/// A structural well-formedness violation (spec §5.7).
#[derive(Clone, PartialEq, Eq)]
pub enum StructureError {
    /// A `@label` reference with no matching shape declaration.
    DanglingShapeRef(String),
    /// A `&label` inclusion with no matching triple-expression label (also
    /// raised when the label names a shape expression instead — an inclusion
    /// must reference a *triple* expression).
    DanglingTripleExprRef(String),
    /// The same shape-expression label declared more than once.
    DuplicateShapeLabel(String),
    /// The same triple-expression label declared more than once.
    DuplicateTripleExprLabel(String),
    /// One label used as both a shape-expression and a triple-expression
    /// label.
    LabelCollision(String),
    /// A shape reference reachable from itself without crossing a triple
    /// constraint.
    ReferenceCycle(String),
    /// A negated reference inside a dependency cycle (negation
    /// stratification failure).
    NegatedCycle(String),
}

impl fmt::Display for StructureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DanglingShapeRef(l) => write!(f, "reference to undeclared shape {l}"),
            Self::DanglingTripleExprRef(l) => {
                write!(
                    f,
                    "inclusion of {l}, which is not a declared triple expression"
                )
            }
            Self::DuplicateShapeLabel(l) => write!(f, "shape label {l} declared more than once"),
            Self::DuplicateTripleExprLabel(l) => {
                write!(f, "triple expression label {l} declared more than once")
            }
            Self::LabelCollision(l) => write!(
                f,
                "{l} used as both a shape label and a triple expression label"
            ),
            Self::ReferenceCycle(l) => write!(
                f,
                "shape {l} references itself without an intervening triple constraint"
            ),
            Self::NegatedCycle(l) => {
                write!(f, "negated reference to {l} inside a dependency cycle")
            }
        }
    }
}

impl fmt::Debug for StructureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for StructureError {}

/// Check the spec §5.7 structural requirements, reporting every violation.
///
/// # Examples
///
/// A reference to an undeclared shape parses, but fails the structural check:
///
/// ```
/// use purrdf_shex::{check_structure, parse_shexc};
///
/// let ok = parse_shexc(
///     "<http://example.org/S> { <http://example.org/p> . }",
///     None,
/// )
/// .expect("a well-formed schema parses");
/// assert!(check_structure(&ok).is_ok());
///
/// let dangling = parse_shexc("<http://example.org/S> @<http://example.org/Missing>", None)
///     .expect("syntactically fine");
/// assert!(check_structure(&dangling).is_err());
/// ```
pub fn check_structure(schema: &Schema) -> Result<(), Vec<StructureError>> {
    let mut errors = Vec::new();
    let checker = Checker::new(schema, &mut errors);
    checker.run();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// A shape-to-shape dependency edge; `negated` marks references under `NOT`
/// or through an `EXTRA`-covered triple constraint.
struct Edge {
    from: usize,
    to: usize,
    negated: bool,
    /// `true` when the reference never crossed a triple constraint (the
    /// reference-only cycle graph is the sub-graph of these edges).
    ref_only: bool,
}

struct Checker<'a> {
    schema: &'a Schema,
    shape_labels: HashMap<&'a str, usize>,
    triple_exprs: HashMap<&'a str, &'a TripleExpr>,
    errors: &'a mut Vec<StructureError>,
    edges: Vec<Edge>,
}

impl<'a> Checker<'a> {
    fn new(schema: &'a Schema, errors: &'a mut Vec<StructureError>) -> Self {
        Self {
            schema,
            shape_labels: HashMap::new(),
            triple_exprs: HashMap::new(),
            errors,
            edges: Vec::new(),
        }
    }

    fn run(mut self) {
        self.collect_labels();
        self.collect_edges_and_check_refs();
        self.check_reference_cycles();
        self.check_negation_requirement();
    }

    // ── labels ──────────────────────────────────────────────────────────────

    fn collect_labels(&mut self) {
        for (index, decl) in self.schema.shapes.iter().enumerate() {
            if self.shape_labels.insert(decl.id.as_str(), index).is_some() {
                self.errors
                    .push(StructureError::DuplicateShapeLabel(decl.id.clone()));
            }
        }
        let mut triple_labels: Vec<(&'a str, &'a TripleExpr)> = Vec::new();
        for decl in &self.schema.shapes {
            collect_triple_labels_shape_expr(&decl.expr, &mut triple_labels);
        }
        if let Some(start) = &self.schema.start {
            collect_triple_labels_shape_expr(start, &mut triple_labels);
        }
        for (label, expr) in triple_labels {
            if self.triple_exprs.insert(label, expr).is_some() {
                self.errors
                    .push(StructureError::DuplicateTripleExprLabel(label.to_owned()));
            }
        }
        for label in self.triple_exprs.keys() {
            if self.shape_labels.contains_key(label) {
                self.errors
                    .push(StructureError::LabelCollision((*label).to_owned()));
            }
        }
    }

    // ── dependency edges + dangling references ──────────────────────────────

    fn collect_edges_and_check_refs(&mut self) {
        let mut dangling_shape: Vec<String> = Vec::new();
        let mut dangling_triple: Vec<String> = Vec::new();
        for (index, decl) in self.schema.shapes.iter().enumerate() {
            let mut walker = Walker {
                shape_labels: &self.shape_labels,
                triple_exprs: &self.triple_exprs,
                edges: &mut self.edges,
                dangling_shape: &mut dangling_shape,
                dangling_triple: &mut dangling_triple,
                from: Some(index),
                include_stack: Vec::new(),
            };
            walker.shape_expr(&decl.expr, false, true);
        }
        if let Some(start) = &self.schema.start {
            let mut walker = Walker {
                shape_labels: &self.shape_labels,
                triple_exprs: &self.triple_exprs,
                edges: &mut self.edges,
                dangling_shape: &mut dangling_shape,
                dangling_triple: &mut dangling_triple,
                from: None,
                include_stack: Vec::new(),
            };
            walker.shape_expr(start, false, true);
        }
        dangling_shape.sort();
        dangling_shape.dedup();
        dangling_triple.sort();
        dangling_triple.dedup();
        self.errors.extend(
            dangling_shape
                .into_iter()
                .map(StructureError::DanglingShapeRef),
        );
        self.errors.extend(
            dangling_triple
                .into_iter()
                .map(StructureError::DanglingTripleExprRef),
        );
    }

    // ── reference-only cycles ────────────────────────────────────────────────

    fn check_reference_cycles(&mut self) {
        let n = self.schema.shapes.len();
        let mut adjacency = vec![Vec::new(); n];
        for edge in self.edges.iter().filter(|e| e.ref_only) {
            adjacency[edge.from].push(edge.to);
        }
        let components = tarjan_scc(&adjacency);
        let mut flagged: HashSet<usize> = HashSet::new();
        for component in &components {
            if component.len() > 1 {
                flagged.extend(component.iter().copied());
            }
        }
        for edge in self.edges.iter().filter(|e| e.ref_only) {
            if edge.from == edge.to {
                flagged.insert(edge.from);
            }
        }
        let mut labels: Vec<&str> = flagged
            .into_iter()
            .map(|i| self.schema.shapes[i].id.as_str())
            .collect();
        labels.sort_unstable();
        self.errors.extend(
            labels
                .into_iter()
                .map(|l| StructureError::ReferenceCycle(l.to_owned())),
        );
    }

    // ── negation requirement ─────────────────────────────────────────────────

    fn check_negation_requirement(&mut self) {
        let n = self.schema.shapes.len();
        let mut adjacency = vec![Vec::new(); n];
        for edge in &self.edges {
            adjacency[edge.from].push(edge.to);
        }
        let components = tarjan_scc(&adjacency);
        let mut component_of = vec![0usize; n];
        for (id, component) in components.iter().enumerate() {
            for &node in component {
                component_of[node] = id;
            }
        }
        let mut labels: Vec<&str> = Vec::new();
        for edge in &self.edges {
            if edge.negated && component_of[edge.from] == component_of[edge.to] {
                // A self-loop is trivially in its own SCC; a cross-node edge is
                // cyclic only when both ends share a non-trivial SCC.
                let same_node = edge.from == edge.to;
                let in_cycle = same_node || components[component_of[edge.from]].len() > 1;
                if in_cycle {
                    labels.push(self.schema.shapes[edge.to].id.as_str());
                }
            }
        }
        labels.sort_unstable();
        labels.dedup();
        self.errors.extend(
            labels
                .into_iter()
                .map(|l| StructureError::NegatedCycle(l.to_owned())),
        );
    }
}

/// Collect `$label` declarations under one shape expression. Crate-visible:
/// the validator reuses it to build its inclusion table.
pub(crate) fn collect_triple_labels_shape_expr<'a>(
    expr: &'a ShapeExpr,
    out: &mut Vec<(&'a str, &'a TripleExpr)>,
) {
    match expr {
        ShapeExpr::And(parts) | ShapeExpr::Or(parts) => {
            for part in parts {
                collect_triple_labels_shape_expr(part, out);
            }
        }
        ShapeExpr::Not(inner) => collect_triple_labels_shape_expr(inner, out),
        ShapeExpr::Shape(shape) => {
            if let Some(expr) = &shape.expression {
                collect_triple_labels_triple_expr(expr, out);
            }
        }
        ShapeExpr::Node(_) | ShapeExpr::External | ShapeExpr::Ref(_) => {}
    }
}

fn collect_triple_labels_triple_expr<'a>(
    expr: &'a TripleExpr,
    out: &mut Vec<(&'a str, &'a TripleExpr)>,
) {
    match expr {
        TripleExpr::EachOf(group) | TripleExpr::OneOf(group) => {
            if let Some(id) = &group.id {
                out.push((id.as_str(), expr));
            }
            for member in &group.expressions {
                collect_triple_labels_triple_expr(member, out);
            }
        }
        TripleExpr::TripleConstraint(tc) => {
            if let Some(id) = &tc.id {
                out.push((id.as_str(), expr));
            }
            if let Some(ve) = &tc.value_expr {
                collect_triple_labels_shape_expr(ve, out);
            }
        }
        TripleExpr::Ref(_) => {}
    }
}

/// Walks one declaration, recording dependency edges and dangling references.
struct Walker<'a, 'b> {
    shape_labels: &'b HashMap<&'a str, usize>,
    triple_exprs: &'b HashMap<&'a str, &'a TripleExpr>,
    edges: &'b mut Vec<Edge>,
    dangling_shape: &'b mut Vec<String>,
    dangling_triple: &'b mut Vec<String>,
    /// The declaration index edges originate from (`None` for `start`).
    from: Option<usize>,
    /// Inclusion labels on the walk stack (guards include cycles).
    include_stack: Vec<&'a str>,
}

impl<'a> Walker<'a, '_> {
    /// `negated`: under an odd number of `NOT`s / EXTRA-covered constraints.
    /// `ref_only`: no triple constraint crossed yet.
    fn shape_expr(&mut self, expr: &'a ShapeExpr, negated: bool, ref_only: bool) {
        match expr {
            ShapeExpr::And(parts) | ShapeExpr::Or(parts) => {
                for part in parts {
                    self.shape_expr(part, negated, ref_only);
                }
            }
            ShapeExpr::Not(inner) => self.shape_expr(inner, !negated, ref_only),
            ShapeExpr::Ref(label) => match self.shape_labels.get(label.as_str()) {
                Some(&to) => {
                    if let Some(from) = self.from {
                        self.edges.push(Edge {
                            from,
                            to,
                            negated,
                            ref_only,
                        });
                    }
                }
                None => self.dangling_shape.push(label.clone()),
            },
            ShapeExpr::Shape(shape) => self.shape(shape, negated),
            ShapeExpr::Node(_) | ShapeExpr::External => {}
        }
    }

    fn shape(&mut self, shape: &'a Shape, negated: bool) {
        if let Some(expr) = &shape.expression {
            self.triple_expr(expr, shape, negated);
        }
    }

    fn triple_expr(&mut self, expr: &'a TripleExpr, enclosing: &'a Shape, negated: bool) {
        match expr {
            TripleExpr::EachOf(group) | TripleExpr::OneOf(group) => {
                for member in &group.expressions {
                    self.triple_expr(member, enclosing, negated);
                }
            }
            TripleExpr::TripleConstraint(tc) => {
                if let Some(ve) = &tc.value_expr {
                    // A reference through an EXTRA-covered predicate carries
                    // negative polarity (spec §5.7 negation requirement).
                    let extra_negates = tc.inverse != Some(true)
                        && enclosing.extra.iter().any(|p| p == &tc.predicate);
                    self.shape_expr(ve, negated ^ extra_negates, false);
                }
            }
            TripleExpr::Ref(label) => match self.triple_exprs.get(label.as_str()) {
                Some(&target) => {
                    if !self.include_stack.contains(&label.as_str()) {
                        self.include_stack.push(label.as_str());
                        self.triple_expr(target, enclosing, negated);
                        self.include_stack.pop();
                    }
                }
                None => self.dangling_triple.push(label.clone()),
            },
        }
    }
}

/// Iterative Tarjan strongly-connected components over an adjacency list.
/// Hand-rolled (no `petgraph`), explicit stack (no recursion on user input).
fn tarjan_scc(adjacency: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let n = adjacency.len();
    const UNSET: usize = usize::MAX;
    let mut index = vec![UNSET; n];
    let mut low = vec![0usize; n];
    let mut on_stack = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut next_index = 0usize;
    let mut components: Vec<Vec<usize>> = Vec::new();
    // Work frames: (node, next child position).
    let mut work: Vec<(usize, usize)> = Vec::new();

    for root in 0..n {
        if index[root] != UNSET {
            continue;
        }
        work.push((root, 0));
        while let Some(&mut (node, ref mut child_pos)) = work.last_mut() {
            if *child_pos == 0 {
                index[node] = next_index;
                low[node] = next_index;
                next_index += 1;
                stack.push(node);
                on_stack[node] = true;
            }
            let mut advanced = false;
            while *child_pos < adjacency[node].len() {
                let child = adjacency[node][*child_pos];
                *child_pos += 1;
                if index[child] == UNSET {
                    work.push((child, 0));
                    advanced = true;
                    break;
                }
                if on_stack[child] {
                    low[node] = low[node].min(index[child]);
                }
            }
            if advanced {
                continue;
            }
            // Node finished.
            work.pop();
            if let Some(&(parent, _)) = work.last() {
                low[parent] = low[parent].min(low[node]);
            }
            if low[node] == index[node] {
                let mut component = Vec::new();
                while let Some(member) = stack.pop() {
                    on_stack[member] = false;
                    component.push(member);
                    if member == node {
                        break;
                    }
                }
                components.push(component);
            }
        }
    }
    components
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tarjan_finds_cycle() {
        // 0 → 1 → 2 → 0, 3 isolated.
        let adjacency = vec![vec![1], vec![2], vec![0], vec![]];
        let mut components = tarjan_scc(&adjacency);
        for c in &mut components {
            c.sort_unstable();
        }
        components.sort();
        assert!(components.contains(&vec![0, 1, 2]));
        assert!(components.contains(&vec![3]));
    }
}
