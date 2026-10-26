// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native signed dependencies over original model positions.

use super::{RecursionSemantics, graph};
use crate::error::{PreparationRefusal, RecursionPolarity, ShapesError};
use crate::expression::{CustomFunction, FnCall, NodeExpr, ShapeArg};
use crate::model::{sh, shnex, sparql_ns};
use crate::product::ShapesProductError;
use crate::rules::Rule;
use crate::shapes::link::{ModelWalk, ShapeIndexRef};
use crate::shapes::{Constraint, ModelOwner, PropertyShape, Shape, Target};
use purrdf_lex::allocation::{Memory, Resident, StorageError};
use purrdf_rdf::{FastMap, FastSet};
use std::sync::Arc;

pub(crate) fn plain_handle(shape: &Shape) -> bool {
    matches!(shape.constraints.as_slice(), [Constraint::Reference(id)] if *id == shape.id)
        && shape.property_shapes.is_empty()
        && shape.deactivated_when.is_empty()
        && shape.constraint_annotations.is_empty()
}

/// An exported root can be replaced only when it still has the exact metadata
/// copied by ModelOwner::root. Closed catalogs contain no executable rule or
/// computed target, so their remaining target operands compare natively.
pub(crate) fn canonical_handle(shape: &Shape) -> bool {
    if !plain_handle(shape) {
        return false;
    }
    let Some(owner) = &shape.owner else {
        return !shape.deactivated
            && shape.severity == crate::report::Severity::Violation
            && shape.messages.is_empty()
            && shape.box_roles.is_empty()
            && shape.targets.is_empty()
            && shape.rules.is_empty();
    };
    let Some(definition) = owner.definition(&shape.id) else {
        return false;
    };
    shape.deactivated == definition.deactivated
        && shape.severity == definition.severity
        && shape.messages.is_empty()
        && shape.box_roles.is_empty()
        && shape.rules.is_empty()
        && definition.rules.is_empty()
        && shape.targets.len() == definition.targets.len()
        && shape
            .targets
            .iter()
            .zip(&definition.targets)
            .all(|(left, right)| match (left, right) {
                (Target::Class(a), Target::Class(b))
                | (Target::SubjectsOf(a), Target::SubjectsOf(b))
                | (Target::ObjectsOf(a), Target::ObjectsOf(b)) => a == b,
                (Target::Node(a), Target::Node(b))
                | (Target::ImplicitClass(a), Target::ImplicitClass(b)) => a == b,
                _ => false,
            })
}

pub(crate) fn admit_shape(
    shape: &Shape,
    semantics: Option<RecursionSemantics>,
) -> Result<(), ShapesError> {
    if let Some(owner) = &shape.owner {
        owner.admit_recursion(semantics)?;
        if if semantics.is_some() {
            canonical_handle(shape)
        } else {
            plain_handle(shape)
        } {
            return Ok(());
        }
    }
    if semantics.is_some()
        && let Some(owner) = &shape.owner
        && crate::constraints::fixed_point::occurrence::maybe_shape(
            owner,
            std::ptr::from_ref(shape).addr(),
        )?
        .is_none()
    {
        return Err(
            PreparationRefusal::UnqualifiedRecursion(super::ClosedRecursionRefusal::new(
                &shape.id,
                "original-model-occurrence",
                Some(owner),
                None,
            )?)
            .into(),
        );
    }
    let mut builder = Builder {
        closed: semantics.is_some(),
        ..Builder::default()
    };
    builder
        .shape(shape)
        .and_then(|()| builder.drain())
        .map_err(WalkError::into_shape)?;
    builder.analysis()?.admit(semantics)
}

pub(crate) fn admit_plan(plan: crate::plan::ShapePlan<'_>) -> Result<(), ShapesError> {
    let semantics = plan
        .binding()
        .validation_request()
        .recursion
        .as_ref()
        .map(|options| options.semantics);
    if let Some(origin) = plan.origin() {
        origin.admit_recursion(semantics)?;
    }
    if let Some(owner) = plan.model() {
        if !plan
            .origin()
            .is_some_and(|origin| Arc::ptr_eq(origin, owner))
        {
            owner.admit_recursion(semantics)?;
        }
    } else if plan.origin().is_none() {
        admit_shape(plan.shape(), semantics)?;
    }
    Ok(())
}

pub(crate) fn prove_plan(
    plan: crate::plan::ShapePlan<'_>,
) -> Result<super::closed::ClosedProof, ShapesError> {
    if let Some(owner) = plan.model().or_else(|| plan.origin()) {
        super::cache::prove(owner)?;
        let original = if canonical_handle(plan.shape()) {
            plan.definition()?.shape()
        } else {
            plan.shape()
        };
        let origin = plan.origin().unwrap_or(owner);
        if crate::constraints::fixed_point::occurrence::maybe_shape(
            origin,
            std::ptr::from_ref(original).addr(),
        )?
        .is_none()
        {
            return Err(PreparationRefusal::UnqualifiedRecursion(
                super::ClosedRecursionRefusal::new(
                    &plan.shape().id,
                    "original-model-occurrence",
                    Some(origin),
                    None,
                )?,
            )
            .into());
        }
    } else {
        let mut builder = Builder {
            closed: true,
            ..Builder::default()
        };
        builder
            .shape(plan.shape())
            .and_then(|()| builder.drain())
            .map_err(WalkError::into_shape)?;
    }
    Ok(super::closed::ClosedProof(()))
}

pub(crate) fn analyze_closed(owner: &Arc<ModelOwner>) -> Result<graph::Analysis, ShapesError> {
    let mut builder = Builder {
        closed: true,
        ..Builder::default()
    };
    builder
        .model(owner)
        .and_then(|()| builder.drain())
        .map_err(WalkError::into_shape)?;
    builder.analysis()
}

pub(crate) fn analyze(owner: &Arc<ModelOwner>) -> Result<graph::Analysis, ShapesError> {
    // Flat terminal catalogs contain no reference, expression, rule or target
    // capable of returning to another definition. Prove this without building
    // adjacency storage; root export must not allocate to copy such bodies.
    if flat_terminals(owner) {
        return Ok(graph::Analysis::default());
    }
    let mut builder = Builder::default();
    builder
        .model(owner)
        .and_then(|()| builder.drain())
        .map_err(WalkError::into_shape)?;
    builder.analysis()
}

pub(super) fn flat_terminals(owner: &ModelOwner) -> bool {
    let Some(definitions) = owner.index.get() else {
        return false;
    };
    owner.functions.iter().all(|function| {
        matches!(
            function.body.get(),
            Some(
                NodeExpr::Empty
                    | NodeExpr::This
                    | NodeExpr::Constant(_)
                    | NodeExpr::Arg(_)
                    | NodeExpr::Var(_)
                    | NodeExpr::Path(_)
                    | NodeExpr::List(_)
            )
        )
    }) && definitions.values().all(|shape| {
        shape.owner.is_none()
            && shape.property_shapes.is_empty()
            && shape.rules.is_empty()
            && shape.deactivated_when.is_empty()
            && shape.constraint_annotations.is_empty()
            && shape.targets.iter().all(|target| {
                matches!(
                    target,
                    Target::Class(_)
                        | Target::SubjectsOf(_)
                        | Target::ObjectsOf(_)
                        | Target::Node(_)
                        | Target::ImplicitClass(_)
                )
            })
            && shape.constraints.iter().all(|constraint| {
                matches!(
                    constraint,
                    Constraint::Class(_)
                        | Constraint::Datatype(_)
                        | Constraint::NodeKind(_)
                        | Constraint::MinCount(_)
                        | Constraint::MaxCount(_)
                        | Constraint::In(_)
                        | Constraint::HasValue(_)
                )
            })
    })
}

enum WalkError {
    Shape(ShapesError),
    Product(ShapesProductError),
}
purrdf_lex::variant_from!(WalkError { Product(ShapesProductError) });
purrdf_lex::variant_from!(WalkError { Shape(PreparationRefusal) via ShapesError::from });
purrdf_lex::variant_from!(WalkError { Shape(StorageError) via ShapesError::Storage });
impl WalkError {
    fn into_shape(self) -> ShapesError {
        match self {
            Self::Shape(error) => error,
            Self::Product(error) => error.preparation_refusal().cloned().map_or_else(
                || ShapesError::internal(format!("native dependency walk: {error}")),
                ShapesError::Preparation,
            ),
        }
    }
}

#[derive(Clone, Copy)]
enum Kind<'a> {
    Shape(&'a Shape),
    Property(&'a PropertyShape),
    Function(&'a Arc<CustomFunction>),
}
#[derive(Clone, Copy)]
struct Node<'a> {
    kind: Kind<'a>,
    owner: Option<&'a Arc<ModelOwner>>,
}
impl Node<'_> {
    fn key(self) -> (u8, usize, usize) {
        let domain = self.owner.map_or(0, |owner| Arc::as_ptr(owner).addr());
        let (kind, address) = match self.kind {
            Kind::Shape(shape) => (0, std::ptr::from_ref(shape).addr()),
            Kind::Property(property) => (1, std::ptr::from_ref(property).addr()),
            Kind::Function(function) => (2, Arc::as_ptr(function).addr()),
        };
        (kind, domain, address)
    }
    fn label(self) -> Result<String, ShapesError> {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        Ok(match self.kind {
            Kind::Shape(shape) => shape.id.try_spelling()?,
            Kind::Property(property) => property.id.try_spelling()?,
            Kind::Function(function) => {
                memory.format(&format_args!("<{}>", function.iri.as_str()))?
            }
        })
    }
}

#[derive(Clone, Copy)]
struct Context {
    polarity: RecursionPolarity,
    shape_polarity: RecursionPolarity,
    parameter: &'static str,
    expression: bool,
    qualified: Option<(usize, RecursionPolarity, bool)>,
}
impl Default for Context {
    fn default() -> Self {
        Self {
            polarity: RecursionPolarity::Positive,
            shape_polarity: RecursionPolarity::Positive,
            parameter: "Reference",
            expression: false,
            qualified: None,
        }
    }
}

#[derive(Default)]
struct Builder<'a> {
    closed: bool,
    live: usize,
    nodes: Vec<Node<'a>>,
    indexes: FastMap<(u8, usize, usize), usize>,
    pending: Vec<usize>,
    models: FastSet<usize>,
    adjacency: Vec<Vec<usize>>,
    edges: Vec<graph::Dependency>,
    current: Option<usize>,
    owner: Option<&'a Arc<ModelOwner>>,
    context: Context,
    depth: u32,
}

impl<'a> Builder<'a> {
    fn unqualified(
        &self,
        parameter: &'static str,
        index: Option<&ShapeIndexRef>,
    ) -> Result<(), WalkError> {
        if let Some(index) = index {
            let owner = self.owner.ok_or(PreparationRefusal::UnboundModel)?;
            if index.strong_count() == 0 {
                return Err(PreparationRefusal::ExpiredIndex.into());
            }
            if !std::sync::Weak::ptr_eq(index, &Arc::downgrade(&owner.index)) {
                return Err(PreparationRefusal::ForeignIndex.into());
            }
        }
        let node = self.current.map(|index| self.nodes[index]);
        let term;
        let definition = match node.map(|node| node.kind) {
            Some(Kind::Shape(shape)) => &shape.id,
            Some(Kind::Property(property)) => &property.id,
            Some(Kind::Function(function)) => {
                let mut resident = Resident;
                term = crate::term::Term::NamedNode(crate::term::NamedNode::new_unchecked(
                    Memory::new(&mut resident).string(function.iri.as_str())?,
                ));
                &term
            }
            None => return Err(PreparationRefusal::UnboundModel.into()),
        };
        Err(
            PreparationRefusal::UnqualifiedRecursion(super::ClosedRecursionRefusal::new(
                definition, parameter, self.owner, index,
            )?)
            .into(),
        )
    }
    fn intern(&mut self, node: Node<'a>) -> Result<usize, WalkError> {
        let key = node.key();
        if let Some(&index) = self.indexes.get(&key) {
            return Ok(index);
        }
        self.indexes
            .try_reserve(1)
            .map_err(|_| StorageError::AllocationFailed)?;
        let index = self.nodes.len();
        let mut resident = Resident;
        let mut memory = Memory::resume(&mut resident, self.live);
        memory.push(&mut self.nodes, node)?;
        memory.push(&mut self.pending, index)?;
        memory.push(&mut self.adjacency, Vec::new())?;
        self.live = memory.admitted_bytes();
        self.indexes.insert(key, index);
        Ok(index)
    }
    fn edge(
        &mut self,
        node: Node<'a>,
        parameter: &'static str,
        polarity: RecursionPolarity,
    ) -> Result<(), WalkError> {
        let target = self.intern(node)?;
        if let Some(source) = self.current {
            let mut resident = Resident;
            let mut memory = Memory::resume(&mut resident, self.live);
            memory.push(&mut self.adjacency[source], target)?;
            memory.push(&mut self.edges, (source, target, parameter, polarity))?;
            self.live = memory.admitted_bytes();
        }
        Ok(())
    }
    fn reference(
        &mut self,
        target: &crate::term::Term,
        parameter: &'static str,
    ) -> Result<(), WalkError> {
        let owner = self.owner.ok_or(PreparationRefusal::UnboundReference)?;
        let definition = owner.definition_required(target)?;
        self.edge(
            Node {
                kind: Kind::Shape(definition),
                owner: Some(owner),
            },
            parameter,
            self.context.polarity,
        )
    }
    fn drain(&mut self) -> Result<(), WalkError> {
        while let Some(index) = self.pending.pop() {
            let node = self.nodes[index];
            self.current = Some(index);
            self.owner = node.owner;
            self.context = Context::default();
            match node.kind {
                Kind::Shape(shape) => self.walk_shape(shape)?,
                Kind::Property(property) => {
                    self.context.polarity = RecursionPolarity::Both;
                    self.context.parameter = sh::VALUES;
                    self.walk_property_shape(property)?;
                }
                Kind::Function(function) => {
                    let body = function.body.get().ok_or_else(|| {
                        PreparationRefusal::MissingFunctionBody(function.iri.clone())
                    })?;
                    self.context.parameter = sh::EXPRESSION;
                    self.node_expr(body)?;
                }
            }
        }
        self.current = None;
        Ok(())
    }
    fn indexed_targets(
        &mut self,
        expression: &'a NodeExpr,
        index: &ShapeIndexRef,
        parameter: &'static str,
        polarity: RecursionPolarity,
    ) -> Result<bool, WalkError> {
        let owner = self.owner.ok_or(PreparationRefusal::UnboundModel)?;
        if index.strong_count() == 0 {
            return Err(PreparationRefusal::ExpiredIndex.into());
        }
        if !std::sync::Weak::ptr_eq(index, &Arc::downgrade(&owner.index)) {
            return Err(PreparationRefusal::ForeignIndex.into());
        }
        crate::expression::named_outputs::visit(expression, |target| {
            let definition = owner.definition_required(target)?;
            self.edge(
                Node {
                    kind: Kind::Shape(definition),
                    owner: Some(owner),
                },
                parameter,
                polarity,
            )
        })
    }
    fn analysis(mut self) -> Result<graph::Analysis, ShapesError> {
        for row in &mut self.adjacency {
            row.sort_unstable();
            row.dedup();
        }
        graph::analyze(
            &self.adjacency,
            &self.edges,
            |node| self.nodes[node].label(),
            |node| !matches!(self.nodes[node].kind, Kind::Function(_)),
            |node| {
                let node = self.nodes[node];
                match node.kind {
                    Kind::Shape(shape) => Ok(super::closed::WeakDefinition::new(
                        &shape.id,
                        node.owner,
                        std::ptr::from_ref(shape).addr(),
                        super::closed::RecursionDefinitionKind::Shape,
                    )?),
                    Kind::Property(property) => Ok(super::closed::WeakDefinition::new(
                        &property.id,
                        node.owner,
                        std::ptr::from_ref(property).addr(),
                        super::closed::RecursionDefinitionKind::Property,
                    )?),
                    Kind::Function(function) => {
                        let term =
                            crate::term::Term::NamedNode(crate::term::NamedNode::new_unchecked(
                                Memory::new(&mut Resident).string(function.iri.as_str())?,
                            ));
                        Ok(super::closed::WeakDefinition::new(
                            &term,
                            node.owner,
                            Arc::as_ptr(function).addr(),
                            super::closed::RecursionDefinitionKind::Function,
                        )?)
                    }
                }
            },
        )
    }
}

impl<'a> ModelWalk<'a, WalkError> for Builder<'a> {
    fn depth(&mut self) -> &mut u32 {
        &mut self.depth
    }
    fn enter(&mut self) -> Result<(), WalkError> {
        if purrdf_stack::is_low() {
            return Err(PreparationRefusal::StackMargin.into());
        }
        self.depth = self
            .depth
            .checked_add(1)
            .ok_or(StorageError::SizeOverflow)?;
        Ok(())
    }
    fn first_owner(&mut self, owner: &'a Arc<ModelOwner>) -> Result<bool, WalkError> {
        let key = Arc::as_ptr(owner).addr();
        if self.models.contains(&key) {
            return Ok(false);
        }
        self.models
            .try_reserve(1)
            .map_err(|_| StorageError::AllocationFailed)?;
        self.models.insert(key);
        Ok(true)
    }
    fn model(&mut self, owner: &'a Arc<ModelOwner>) -> Result<(), WalkError> {
        if !self.first_owner(owner)? {
            return Ok(());
        }
        let mut definitions = Vec::new();
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        if let Some(entries) = owner.index.get() {
            for definition in entries.values() {
                memory.push(&mut definitions, definition)?;
            }
        }
        definitions.sort_unstable_by(|left, right| crate::term::canonical_cmp(&left.id, &right.id));
        for definition in definitions {
            self.intern(Node {
                kind: Kind::Shape(definition),
                owner: Some(owner),
            })?;
        }
        for function in &owner.functions {
            if self.closed {
                self.intern(Node {
                    kind: Kind::Function(function),
                    owner: Some(owner),
                })?;
            } else if function.body.get().is_none() {
                return Err(PreparationRefusal::MissingFunctionBody(function.iri.clone()).into());
            }
        }
        Ok(())
    }
    fn first_declaration(&mut self, function: &'a Arc<CustomFunction>) -> Result<bool, WalkError> {
        if !self.closed {
            return Ok(false);
        }
        self.edge(
            Node {
                kind: Kind::Function(function),
                owner: self.owner,
            },
            self.context.parameter,
            self.context.polarity,
        )?;
        Ok(false)
    }
    fn shape_index(&mut self, _: &'a ShapeIndexRef) -> Result<(), WalkError> {
        Ok(())
    }
    fn shape(&mut self, original: &'a Shape) -> Result<(), WalkError> {
        let owner = original.owner.as_ref().or(self.owner);
        if let Some(owner) = owner {
            self.model(owner)?;
        }
        let shape = if (if self.closed {
            canonical_handle(original)
        } else {
            plain_handle(original)
        }) && !original.deactivated
        {
            owner
                .ok_or(PreparationRefusal::UnboundReference)?
                .definition_required(&original.id)?
        } else {
            original
        };
        let (parameter, polarity) = if let Some((own, polarity, disjoint)) = self.context.qualified
        {
            if std::ptr::from_ref(original).addr() == own {
                (sh::QUALIFIED_VALUE_SHAPE, polarity)
            } else {
                if !disjoint {
                    return Ok(());
                }
                (sh::QUALIFIED_VALUE_SHAPES_DISJOINT, polarity.inverted())
            }
        } else if !self.context.expression && self.context.parameter == sh::VALUES {
            (sh::REIFIER_SHAPE, RecursionPolarity::Positive)
        } else {
            (self.context.parameter, self.context.polarity)
        };
        self.edge(
            Node {
                kind: Kind::Shape(shape),
                owner,
            },
            parameter,
            polarity,
        )
    }
    fn property_shape(&mut self, property: &'a PropertyShape) -> Result<(), WalkError> {
        self.edge(
            Node {
                kind: Kind::Property(property),
                owner: self.owner,
            },
            sh::PROPERTY,
            RecursionPolarity::Positive,
        )
    }
    fn targets(&mut self, targets: &'a [Target]) -> Result<(), WalkError> {
        if self.closed {
            for target in targets {
                match target {
                    Target::Sparql { .. } => return self.unqualified(sh::TARGET, None),
                    Target::NodeExpression(_) => return self.unqualified(sh::TARGET_NODE, None),
                    Target::Where(_) => return self.unqualified(sh::TARGET_WHERE, None),
                    _ => {}
                }
            }
        }
        let previous = self.context;
        if self.context.parameter != sh::UNIQUE_VALUES_FOR {
            self.context.polarity = RecursionPolarity::Target;
            self.context.parameter = sh::TARGET_WHERE;
        }
        let result = self.walk_targets(targets);
        self.context = previous;
        result
    }
    fn rule(&mut self, rule: &'a Rule) -> Result<(), WalkError> {
        if self.closed {
            return self.unqualified(sh::RULE, None);
        }
        let previous = self.current.take();
        let result = <Self as ModelWalk<'a, WalkError>>::walk_rule(self, rule);
        self.current = previous;
        result
    }
    fn deactivations(
        &mut self,
        expressions: &'a [NodeExpr],
        annotations: &'a [crate::shapes::ConstraintAnnotation],
    ) -> Result<(), WalkError> {
        let previous = self.context;
        self.context.polarity = RecursionPolarity::Positive;
        self.context.parameter = sh::DEACTIVATED;
        let result = self.walk_deactivations(expressions, annotations);
        self.context = previous;
        result
    }
    fn constraint(&mut self, constraint: &'a Constraint) -> Result<(), WalkError> {
        if self.closed {
            match constraint {
                Constraint::Expression { .. } => return self.unqualified(sh::EXPRESSION, None),
                Constraint::Sparql { .. } => return self.unqualified(sh::SPARQL, None),
                Constraint::NodeByExpression { shapes, .. } => {
                    return self.unqualified(sh::NODE_BY_EXPRESSION, Some(shapes));
                }
                Constraint::UniqueValuesFor { .. } => {
                    return self.unqualified(sh::UNIQUE_VALUES_FOR, None);
                }
                Constraint::Component { .. } => return self.unqualified(sh::PARAMETER, None),
                _ => {}
            }
        }
        let previous = self.context;
        self.context = Context::default();
        match constraint {
            Constraint::Reference(target) => {
                self.reference(target, "Reference")?;
            }
            Constraint::PropertyReference(edge) => {
                self.reference(&edge.target, sh::PROPERTY)?;
            }
            Constraint::Not(_) => {
                self.context.polarity = RecursionPolarity::Negative;
                self.context.parameter = sh::NOT;
            }
            Constraint::Xone(operands) => {
                let positive = self.current.is_some_and(|index| {
                    let Kind::Shape(source) = self.nodes[index].kind else {
                        return false;
                    };
                    super::guards::xone_positive(source, operands, self.owner.map(Arc::as_ref))
                });
                self.context.polarity = if positive {
                    RecursionPolarity::Positive
                } else {
                    RecursionPolarity::Both
                };
                self.context.parameter = sh::XONE;
            }
            Constraint::Node(_) => self.context.parameter = sh::NODE,
            Constraint::And(_) => self.context.parameter = sh::AND,
            Constraint::Or(_) => self.context.parameter = sh::OR,
            Constraint::MemberShape(_) => self.context.parameter = sh::MEMBER_SHAPE,
            Constraint::SomeValue(_) => self.context.parameter = sh::SOME_VALUE,
            Constraint::QualifiedValueShape {
                shape,
                min_count,
                max_count,
                disjoint,
                ..
            } => {
                let polarity = match (min_count.is_some(), max_count.is_some()) {
                    (true, true) => RecursionPolarity::Both,
                    (false, true) => RecursionPolarity::Negative,
                    _ => RecursionPolarity::Positive,
                };
                self.context.qualified = Some((
                    std::ptr::from_ref(shape.as_ref()).addr(),
                    polarity,
                    *disjoint,
                ));
            }
            Constraint::Expression { .. } => self.context.parameter = sh::EXPRESSION,
            Constraint::NodeByExpression { .. } => {}
            Constraint::UniqueValuesFor { .. } => {
                self.context.parameter = sh::UNIQUE_VALUES_FOR;
                self.context.polarity = RecursionPolarity::Both;
            }
            Constraint::Class(_)
            | Constraint::Datatype(_)
            | Constraint::NodeKind(_)
            | Constraint::MinCount(_)
            | Constraint::MaxCount(_)
            | Constraint::In(_)
            | Constraint::HasValue(_)
            | Constraint::MinLength(_)
            | Constraint::MaxLength(_)
            | Constraint::UniqueLang(_)
            | Constraint::LanguageIn(_)
            | Constraint::Closed { .. }
            | Constraint::MinInclusive(_)
            | Constraint::MaxInclusive(_)
            | Constraint::MinExclusive(_)
            | Constraint::MaxExclusive(_)
            | Constraint::Sparql { .. }
            | Constraint::Equals(_)
            | Constraint::Disjoint(_)
            | Constraint::SubsetOf(_)
            | Constraint::LessThan(_)
            | Constraint::LessThanOrEquals(_)
            | Constraint::MinListLength(_)
            | Constraint::MaxListLength(_)
            | Constraint::UniqueMembers(_)
            | Constraint::SingleLine(_)
            | Constraint::RootClass(_)
            | Constraint::Component { .. }
            | Constraint::Pattern { .. } => {}
        }
        let result = self.walk_constraint(constraint);
        self.context = previous;
        result
    }
    fn shape_arg(&mut self, arg: &'a ShapeArg) -> Result<(), WalkError> {
        if !self.closed {
            return Ok(());
        }
        let previous = self.context;
        if matches!(arg, ShapeArg::Named(_)) {
            self.context.polarity = self.context.shape_polarity;
        } else if let ShapeArg::Computed { expr, shapes } = arg {
            self.indexed_targets(
                expr,
                shapes,
                self.context.parameter,
                self.context.shape_polarity,
            )?;
        }
        let result = self.walk_shape_arg(arg);
        self.context = previous;
        result
    }
    fn node_expr(&mut self, expression: &'a NodeExpr) -> Result<(), WalkError> {
        if !self.closed {
            return Ok(());
        }
        if !matches!(expression, NodeExpr::Constant(_) | NodeExpr::Empty) {
            return self.unqualified(self.context.parameter, None);
        }
        let previous = (self.context, self.owner);
        self.context.expression = true;
        match expression {
            NodeExpr::ConformsToShape { .. } => {
                self.context.parameter = shnex::CONFORMS_TO_SHAPE;
                self.context.shape_polarity = self.context.polarity;
                self.context.polarity = RecursionPolarity::Both;
            }
            NodeExpr::Call(FnCall::Sparql { iri, .. })
                if iri.as_str().strip_prefix(sparql_ns::NS) == Some("logical-not") =>
            {
                self.context.polarity = self.context.polarity.inverted();
            }
            NodeExpr::CustomCall { func, .. }
            | NodeExpr::Call(FnCall::UserDefined { iri: func, .. }) => {
                if let Some(function) = self.owner.and_then(|owner| owner.function(func)) {
                    self.declare(function)?;
                }
                if self.context.polarity != RecursionPolarity::Target {
                    self.context.polarity = RecursionPolarity::Both;
                }
            }
            NodeExpr::Owned { owner, .. } => {
                self.owner = Some(owner);
                self.model(owner)?;
            }
            _ if self.context.polarity != RecursionPolarity::Target => {
                self.context.polarity = RecursionPolarity::Both;
            }
            _ => {}
        }
        let result = self.walk_node_expr(expression);
        (self.context, self.owner) = previous;
        result
    }
}
