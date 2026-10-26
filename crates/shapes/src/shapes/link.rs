// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The model linking pass: the one place a parsed shapes graph's shared,
//! logical references are established and their shared topology is checked.
//!
//! A [`Shapes`](crate::shapes::Shapes) is not a tree. Its model domain owns the
//! declarations and index that exported shapes and expressions retain:
//!
//! * `Constraint::NodeByExpression`'s `shapes` and `ShapeArg::Computed`'s `shapes`
//!   are weak references to ONE index per shapes graph. Its bare definitions
//!   contain no owning backreference.
//! * `CustomFunction`'s `body` is an `OnceLock<NodeExpr>` that may legitimately
//!   call its own IRI (SHACL 1.2 Node Expressions §6.1/§6.2). Calls resolve in
//!   the original model domain without owning the declaration they name.
//!
//! Parsing queues identities and produces each bare definition once. The shared
//! cells are created before parsing and filled here after every definition and
//! function body is available. Logical dependencies never own another body.
//!
//! # Why this is a function and not two copies of a recipe
//!
//! Two callers have to perform this pass: the shapes parser, which reaches it from
//! RDF, and the prepared-product decoder, which reaches it from bytes. The rejected
//! alternative was to let each of them do its own filling — it is three short steps,
//! and each caller already holds the pieces.
//!
//! That alternative fails silently, which is why it was rejected. Suppose the second
//! implementation hands every `NodeByExpression` constraint its OWN
//! `Arc::new(OnceLock::new())` instead of cloning the graph's one handle. It
//! type-checks. It allocates one index per constraint rather than one per graph.
//! Filling "the" index then reaches exactly one constraint and leaves every other
//! handle permanently empty, so each of those constraints resolves NO shape at all —
//! and a `sh:nodeByExpression` that resolves no shape reports no violation. The
//! shapes graph loads green, validates green, and checks nothing. Every existing
//! test still passes, because no test can see an `Arc`'s identity from the outside.
//!
//! [`link_shapes`] is therefore not merely shared code. It **verifies the topology
//! it installs**: it walks the linked model and refuses unless every shape-index
//! handle it reaches is [`Arc::ptr_eq`] to the single index, and every declared
//! function body is filled. The sharing stops being a convention two call sites are
//! trusted to honour and becomes a checked precondition of linking at all.
//!
//! # Why the refusal is a [`ShapesProductError`]
//!
//! Both callers admit a candidate model, so both need the same closed refusal
//! vocabulary; inventing a second error type here would give the decoder two
//! unrelated failure channels for one boundary. The dimension is
//! [`ProductDimension::Malformed`] — "the structure is invalid in a way no other
//! dimension names". An unshared handle or an empty body is not an identity
//! mismatch, not a capability this build lacks, and not a depth overflow: it is the
//! value graph's SHAPE being wrong. `product::ast` already refuses its two sibling
//! topology defects — two declarations under one function IRI, two bodies for one
//! declaration — on that same dimension, so a caller that discards and rebuilds on
//! `Malformed` handles all of them with one arm. The walk's own nesting ceiling is
//! the one exception: it reports [`ProductDimension::DepthLimit`], because that is
//! a bound on the walk rather than a defect in the value.
//!
//! Nothing here touches the filesystem, a clock, a thread, or randomness.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock};

use ::purrdf_rdf::FastMap;
use purrdf_sparql_eval::{Arity, ExprFnCall, UserFunctionRegistry};

use crate::expression::{CustomFnKind, CustomFunction, FnCall, NodeExpr, ShapeArg};
use crate::product::ast::MAX_DEPTH;
use crate::product::error::malformed;
use crate::product::{ProductDimension, ShapesProductError};
use crate::rules::{Rule, RuleBody};
use crate::shapes::parser::functions::{
    NativeListCallee, invoke_expression_function, invoke_native_list_function,
};
use crate::shapes::{Constraint, PropertyShape, Shape, Target};
use crate::term::Term;

/// The shapes graph's ONE `sh:nodeByExpression` resolution table.
///
/// Named so the shared-ness is visible at every use site: this is a handle on a
/// single cell, never a value to clone the contents of.
///
/// Keyed by each shape's own identity TERM, not by a rendering of it, so the
/// evaluator — which resolves a produced node once per value node — looks a shape
/// up with the term it already holds instead of building a string to hash.
pub(crate) type ShapeIndex = Arc<OnceLock<FastMap<Term, Shape>>>;
pub(crate) type ShapeIndexRef = std::sync::Weak<OnceLock<FastMap<Term, Shape>>>;

/// Retain the model only on exported roots, after its bare definitions are frozen.
pub(crate) fn retain_roots(
    shapes: &mut [Shape],
    rules: &mut crate::rules::RuleGraph,
    owner: &Arc<crate::shapes::ModelOwner>,
) {
    for shape in shapes {
        shape.owner = Some(Arc::clone(owner));
        for rule in &mut shape.rules {
            retain_rule(rule, owner);
        }
    }
    for rule in &mut rules.global_rules {
        retain_rule(rule, owner);
    }
}

fn retain_rule(rule: &mut Rule, owner: &Arc<crate::shapes::ModelOwner>) {
    for condition in &mut rule.conditions {
        condition.owner = Some(Arc::clone(owner));
    }
    if let RuleBody::Triple {
        subject,
        predicate,
        object,
    } = &mut rule.body
    {
        for expression in [subject, predicate, object] {
            *expression = expression.take().map(|expr| NodeExpr::Owned {
                expr: Box::new(expr),
                owner: Arc::clone(owner),
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Refusals
// ---------------------------------------------------------------------------

/// Refuse: the model nests past [`MAX_DEPTH`], so walking it would risk the
/// native stack.
fn depth_limit() -> ShapesProductError {
    ShapesProductError::new(
        ProductDimension::DepthLimit,
        format!(
            "this shapes model nests shapes, constraints or node expressions more than \
             {MAX_DEPTH} levels deep; flatten the shapes graph, because the ceiling is what stops \
             a walk over it from exhausting the native stack — an abort no caller could have handled"
        ),
    )
}

// ---------------------------------------------------------------------------
// The linking pass
// ---------------------------------------------------------------------------

/// Link a parsed-or-decoded shapes model: install the shared handles, then prove
/// the sharing topology is the one the model's meaning depends on.
///
/// Three steps, in the order the shapes parser has always performed them:
///
/// 1. **Bodies.** Each declaration in `custom_fns` takes its `sh:bodyExpression`
///    from `bodies`, keyed by function IRI. The table is key-sorted, and a
///    declaration whose body is ALREADY installed keeps it — that is the decoder's
///    case, where the byte stream's own two-pass layout has to install bodies
///    before the shapes that call them can be read at all. Self- and mutual
///    recursion need no special handling here because every declaration was
///    interned before any body was built and calls resolve through that domain.
/// 2. **The shape index.** Every shape-index handle reachable in the linked model
///    is checked against `shape_index`, and the index is then built ONCE from
///    `node_shapes` by moving their original bodies into that single cell. The
///    complete catalog retains definitions even when no exported target uses
///    them; retaining a definition does not activate its constraints.
/// 3. **Registration.** Every `sh:ListParameterExpressionFunction` is registered
///    into `registry` as a callable SPARQL function (SHACL 1.2 SPARQL Extensions
///    §7.3), as an `Arc<dyn Fn>` closure over the declaration handle — a custom
///    one over its body, a declared BUILT-IN (`native_list`) over its native
///    implementation. An IRI already registered is not redefined, as §7.3
///    requires (see [`register_native_list_functions`]).
///
/// # Errors
///
/// [`ProductDimension::Malformed`] when a body arrives for a function that is not
/// declared, when a declaration would be given a second body, when a declared body
/// is missing after installation, or when any reachable shape-index handle is not
/// the one `shape_index` names. [`ProductDimension::DepthLimit`] when the model
/// nests past [`MAX_DEPTH`].
/// Prove the GLOBAL rules' shape-index handles are the graph's one handle, exactly as
/// [`link_shapes`] proves every shape's: a global rule's node expressions and condition
/// shapes reach a `sh:nodeByExpression` constraint just as a shape rule's do.
///
/// # Errors
///
/// A handle that is not the shared one.
pub(crate) fn link_global_rules(
    rules: &[Rule],
    shape_index: &ShapeIndex,
) -> Result<(), ShapesProductError> {
    let mut walk = ShapeIndexWalk {
        index: shape_index,
        depth: 0,
        seen_fns: Vec::new(),
        seen_catalogs: Vec::new(),
    };
    for rule in rules {
        walk.rule(rule)?;
    }
    Ok(())
}

pub(crate) fn link_shapes(
    node_shapes: Vec<Shape>,
    shape_index: &ShapeIndex,
    custom_fns: &[Arc<CustomFunction>],
    native_list: &BTreeSet<String>,
    bodies: BTreeMap<String, NodeExpr>,
    registry: &mut UserFunctionRegistry,
) -> Result<Arc<crate::shapes::ModelOwner>, ShapesProductError> {
    let owner = link_definitions(node_shapes, shape_index, custom_fns, bodies)?;
    register_model_functions(&owner, native_list, registry);
    Ok(owner)
}

/// Install and audit one complete original domain before exposing its functions.
pub(super) fn link_definitions(
    node_shapes: Vec<Shape>,
    shape_index: &ShapeIndex,
    custom_fns: &[Arc<CustomFunction>],
    bodies: BTreeMap<String, NodeExpr>,
) -> Result<Arc<crate::shapes::ModelOwner>, ShapesProductError> {
    install_bodies(custom_fns, bodies)?;
    let owner =
        crate::shapes::ModelOwner::from_declarations(Arc::clone(shape_index), custom_fns.to_vec())?;
    install_shape_index(node_shapes, shape_index, custom_fns)?;
    Ok(owner)
}

/// Install only this model's original declaration domain in its query registry.
pub(crate) fn register_model_functions(
    owner: &Arc<crate::shapes::ModelOwner>,
    native_list: &BTreeSet<String>,
    registry: &mut UserFunctionRegistry,
) {
    register_native_list_functions(native_list, owner, registry);
    register_expression_bodied_functions(&owner.functions, owner, registry);
}

/// Whether §7.3's "already registered" rule leaves `iri` alone: SHACL 1.2 SPARQL
/// Extensions §7.3 — "If a function with the same IRI is already registered, SHACL
/// engines MUST ignore the attempt to redefine it unless the function was
/// previously added as a custom SPARQL function."
///
/// A native (host) or expression-bodied registration is kept and the new one
/// ignored. A SPARQL-bodied one — a custom SPARQL function — is the exception the
/// sentence names, and is replaced.
fn keep_existing_registration(iri: &str, registry: &mut UserFunctionRegistry) -> bool {
    if registry.resolve_native(iri).is_some() || registry.resolve_expr(iri).is_some() {
        return true;
    }
    if registry.resolve(iri).is_some() {
        registry.remove_sparql_bodied(iri);
    }
    false
}

/// Register every built-in LIST-parameter function the shapes graph declares as a
/// callable SPARQL function with its NATIVE implementation (SHACL 1.2 SPARQL
/// Extensions §7.3: "SPARQL engines SHOULD register a function for any SHACL
/// instance of sh:ListParameterExpressionFunction from any provided shapes
/// graph.").
///
/// `shnex:conformsToShape` resolves its shape argument against `shape_index`; a
/// `sparql:<NAME>` renders its SPARQL form for the call's arity. Only DECLARED
/// built-ins are registered, so a shapes graph that does not merge the vocabulary
/// registers nothing and pays nothing.
pub(crate) fn register_native_list_functions(
    native_list: &BTreeSet<String>,
    owner: &Arc<crate::shapes::ModelOwner>,
    registry: &mut UserFunctionRegistry,
) {
    for iri in native_list {
        if keep_existing_registration(iri, registry) {
            continue;
        }
        let arity = if iri == crate::model::shnex::CONFORMS_TO_SHAPE {
            Arity::Exact(2)
        } else {
            Arity::AtLeast(0)
        };
        let name = iri.clone();
        // Resolved here, at load, so a call pays only the arity-dependent render.
        let callee = NativeListCallee::resolve(iri);
        let owner = Arc::clone(owner);
        registry.register_expr(
            iri.clone(),
            arity,
            Arc::new(move |call: &ExprFnCall<'_>| {
                invoke_native_list_function(&name, &callee, &owner, call)
            }),
        );
    }
}

/// Step 1: fill each declaration's body cell, then prove every one of them is full.
///
/// The proof is separate from the filling because the two callers fill at different
/// moments: the parser supplies every body in `bodies`, while the decoder has
/// already installed them from the byte stream and supplies none. Only the
/// after-the-fact check covers both.
pub(crate) fn install_bodies(
    custom_fns: &[Arc<CustomFunction>],
    mut bodies: BTreeMap<String, NodeExpr>,
) -> Result<(), ShapesProductError> {
    for func in custom_fns {
        let Some(body) = bodies.remove(func.iri.as_str()) else {
            continue;
        };
        func.body.set(body).map_err(|_| {
            malformed(format!(
                "the custom node-expression function <{}> was given a second body during linking; \
                 supply exactly one body per declaration, because a declaration's call sites all \
                 share one handle and the second body would reach none of them",
                func.iri.as_str()
            ))
        })?;
    }
    if let Some((iri, _)) = bodies.into_iter().next() {
        return Err(malformed(format!(
            "linking was given a body for <{iri}>, which this shapes graph does not declare as a \
             custom node-expression function; declare the function or drop the body, because a \
             body with no declaration has no call site to reach"
        )));
    }
    for func in custom_fns {
        if func.body.get().is_none() {
            return Err(malformed(format!(
                "the custom node-expression function <{}> reached the end of linking with no \
                 installed sh:bodyExpression; install every declared body before linking, because \
                 a call site that resolves to a body-less declaration has nothing to evaluate and \
                 would contribute the empty list at every call instead of failing",
                func.iri.as_str()
            )));
        }
    }
    Ok(())
}

/// Step 2: prove the shape-index topology, then fill the one cell.
///
/// The walk runs before moving the bare bodies into the index. A foreign handle
/// is refused even when no currently exported root reads the definition.
fn install_shape_index(
    node_shapes: Vec<Shape>,
    shape_index: &ShapeIndex,
    custom_fns: &[Arc<CustomFunction>],
) -> Result<(), ShapesProductError> {
    let mut walk = ShapeIndexWalk {
        index: shape_index,
        depth: 0,
        seen_fns: Vec::new(),
        seen_catalogs: Vec::new(),
    };
    for shape in &node_shapes {
        walk.shape(shape)?;
    }
    // Bodies are reachable from a shape only through a call site; a declaration
    // nothing calls yet still carries handles that must be the shared one.
    for func in custom_fns {
        walk.declare(func)?;
    }

    crate::shapes::ModelOwner::install_definitions(shape_index, node_shapes)?;
    Ok(())
}

/// Step 3: register every custom LIST parameter function as a callable SPARQL
/// function, into the SAME [`UserFunctionRegistry`] the `sh:select`/`sh:ask` bodies
/// go into.
///
/// SHACL 1.2 SPARQL Extensions §7.3: "SPARQL engines SHOULD register a function for
/// any SHACL instance of `sh:ListParameterExpressionFunction` from any provided
/// shapes graph." Only that class is registered — a
/// `sh:NamedParameterExpressionFunction` keys its arguments by parameter IRI and has
/// no positional call form, so §7.3 does not name it and there is no well-defined
/// `ex:f(?x, ?y)` for it to answer.
///
/// Each registration is a dataset-aware closure
/// ([`purrdf_sparql_eval::ExprFnBody`]): the body is a node expression, so it is
/// evaluated over the graph the CALLING QUERY supplies rather than over anything
/// captured here. That is what makes a call inside a rules fixpoint read the
/// current round's facts.
///
/// This step cannot fail: [`install_bodies`] has already proved that every
/// declaration — not merely every list-parameter one — carries a body, so no
/// registration can put a call site in reach of a function that answers nothing.
///
/// `pub(crate)` for the prepared-product restore
/// (`crate::product::certified`), which has to assemble the SAME registry from a
/// re-derived shapes graph and a host's injected table. Reusing this registration
/// rather than transcribing it is what keeps the restored call form identical to
/// the parsed one — a second copy would drift in exactly the arity the caller
/// never re-tests.
pub(crate) fn register_expression_bodied_functions(
    custom_fns: &[Arc<CustomFunction>],
    owner: &Arc<crate::shapes::ModelOwner>,
    registry: &mut UserFunctionRegistry,
) {
    for func in custom_fns {
        if !matches!(func.kind, CustomFnKind::ListParameter) {
            continue;
        }
        if keep_existing_registration(func.iri.as_str(), registry) {
            continue;
        }
        let arity = if func.required == func.params.len() {
            Arity::Exact(func.required)
        } else {
            Arity::Range {
                min: func.required,
                max: func.params.len(),
            }
        };
        let declared = Arc::clone(func);
        let owner = Arc::clone(owner);
        let iri = declared.iri.as_str().to_owned();
        registry.register_expr(
            iri,
            arity,
            Arc::new(move |call: &ExprFnCall<'_>| {
                invoke_expression_function(&owner, &declared, call)
            }),
        );
    }
}

// ---------------------------------------------------------------------------
// The model walk
// ---------------------------------------------------------------------------

/// The one traversal of a shapes model's positions that can carry a shape-index
/// handle or reach a custom-function declaration.
///
/// Three passes share this traversal: linking proves every shape-index handle is
/// the graph's one handle; the product codec collects reachable custom-function
/// declarations; native XPath admission inventories declared pattern cells. Each
/// pass supplies hooks at handles, function declarations or patterns. The traversal
/// itself exists once, so every pass visits the same model positions.
///
/// Every `match` over the model is WILDCARD-FREE, so a new `Constraint`, `Target`
/// or `NodeExpr` arm that could reach a handle or a declaration cannot be added
/// without deciding whether the walk has to see it. Each shape, constraint and node
/// expression opens one nesting level, bounded by [`MAX_DEPTH`], because the walk
/// recurses on the native stack.
/// Catalog and declaration-domain callbacks retain their distinct carrier types;
/// their implementations only admit identity, while this trait owns traversal.
/// Visited state belongs to each pass: product header discovery visits a catalog
/// once overall, whereas strong-ownership analysis revisits it for each parent
/// whose outgoing edges must be recorded. Sharing that mutable state between
/// passes would omit declarations or ownership edges.
pub(crate) trait ModelWalk<'model, E = ShapesProductError>
where
    E: From<ShapesProductError>,
{
    /// The live nesting depth the walk bounds.
    fn depth(&mut self) -> &mut u32;

    /// Visit the complete retained model domain once.
    fn first_owner(&mut self, owner: &'model Arc<crate::shapes::ModelOwner>) -> Result<bool, E>;

    fn model(&mut self, owner: &'model Arc<crate::shapes::ModelOwner>) -> Result<(), E> {
        if self.first_owner(owner)? {
            for function in &owner.functions {
                self.declare(function)?;
            }
            if let Some(definitions) = owner.index.get() {
                let mut ordered: Vec<_> = definitions.values().collect();
                ordered.sort_unstable_by(|left, right| {
                    crate::term::canonical_cmp(&left.id, &right.id)
                });
                for definition in ordered {
                    self.shape(definition)?;
                }
            }
        }
        Ok(())
    }

    /// Record `func`; `true` when its body has not been walked yet and must be now.
    /// Recording happens BEFORE the body is walked: a self-recursive body is legal,
    /// and this is what makes walking it terminate.
    ///
    /// # Errors
    ///
    /// Whatever the pass refuses about the declaration.
    fn first_declaration(&mut self, func: &'model Arc<CustomFunction>) -> Result<bool, E>;

    /// Inspect one shape-index handle.
    ///
    /// # Errors
    ///
    /// Whatever the pass refuses about the handle.
    fn shape_index(&mut self, found: &'model ShapeIndexRef) -> Result<(), E>;

    /// Inspect a declared pattern at its original model position.
    ///
    /// # Errors
    /// Returns the collecting pass's actual refusal.
    fn pattern(&mut self, _declaration: &'model Constraint) -> Result<(), E> {
        Ok(())
    }

    /// Open one nesting level, refusing past [`MAX_DEPTH`].
    ///
    /// # Errors
    ///
    /// [`ProductDimension::DepthLimit`] past [`MAX_DEPTH`].
    fn enter(&mut self) -> Result<(), E> {
        crate::product::ast::open_level(self.depth(), depth_limit).map_err(E::from)
    }

    /// Close one nesting level.
    fn leave(&mut self) {
        *self.depth() -= 1;
    }

    /// Walk a declaration and, the first time it is seen, its body.
    ///
    /// # Errors
    ///
    /// A hook's refusal, or the depth ceiling.
    fn declare(&mut self, func: &'model Arc<CustomFunction>) -> Result<(), E> {
        if self.first_declaration(func)?
            && let Some(body) = func.body.get()
        {
            self.node_expr(body)?;
        }
        Ok(())
    }

    /// Walk the target declarations that can reach a node expression: a
    /// structured `sh:targetNode` is one, and a `sh:targetWhere` is a shape.
    ///
    /// # Errors
    ///
    /// A hook's refusal, or the depth ceiling.
    fn targets(&mut self, targets: &'model [Target]) -> Result<(), E> {
        self.walk_targets(targets)
    }

    fn walk_targets(&mut self, targets: &'model [Target]) -> Result<(), E> {
        for target in targets {
            match target {
                Target::Class(_)
                | Target::SubjectsOf(_)
                | Target::ObjectsOf(_)
                | Target::Node(_)
                | Target::ImplicitClass(_)
                | Target::Sparql { .. } => {}
                Target::NodeExpression(expr) => self.node_expr(expr)?,
                Target::Where(shape) => self.shape(shape)?,
            }
        }
        Ok(())
    }

    /// Walk a node shape.
    ///
    /// # Errors
    ///
    /// A hook's refusal, or the depth ceiling.
    fn shape(&mut self, shape: &'model Shape) -> Result<(), E> {
        self.walk_shape(shape)
    }

    /// Walk a shape's original operands through the visitor hooks.
    fn walk_shape(&mut self, shape: &'model Shape) -> Result<(), E> {
        self.enter()?;
        if let Some(owner) = &shape.owner {
            self.model(owner)?;
        }
        self.targets(&shape.targets)?;
        self.deactivations(&shape.deactivated_when, &shape.constraint_annotations)?;
        for constraint in &shape.constraints {
            self.constraint(constraint)?;
        }
        for property in &shape.property_shapes {
            self.property_shape(property)?;
        }
        for rule in &shape.rules {
            self.rule(rule)?;
        }
        self.leave();
        Ok(())
    }

    /// Walk a rule: its node expressions and its condition shapes.
    ///
    /// # Errors
    ///
    /// A hook's refusal, or the depth ceiling.
    fn rule(&mut self, rule: &'model Rule) -> Result<(), E> {
        self.walk_rule(rule)
    }

    fn walk_rule(&mut self, rule: &'model Rule) -> Result<(), E> {
        match &rule.body {
            RuleBody::Triple {
                subject,
                predicate,
                object,
            } => {
                for expr in [subject, predicate, object].into_iter().flatten() {
                    self.node_expr(expr)?;
                }
            }
            RuleBody::Sparql {
                construct: _,
                parameters: _,
            } => {}
        }
        for condition in &rule.conditions {
            self.shape(condition)?;
        }
        Ok(())
    }

    /// Walk a property shape.
    ///
    /// # Errors
    ///
    /// A hook's refusal, or the depth ceiling.
    fn property_shape(&mut self, shape: &'model PropertyShape) -> Result<(), E> {
        self.walk_property_shape(shape)
    }

    fn walk_property_shape(&mut self, shape: &'model PropertyShape) -> Result<(), E> {
        self.enter()?;
        self.deactivations(&shape.deactivated_when, &shape.constraint_annotations)?;
        for expr in shape.values.iter().chain(&shape.default_value) {
            self.node_expr(expr)?;
        }
        for constraint in &shape.constraints {
            self.constraint(constraint)?;
        }
        for nested in &shape.property_shapes {
            self.property_shape(nested)?;
        }
        for reifier in &shape.reifier_shapes {
            self.shape(reifier)?;
        }
        self.leave();
        Ok(())
    }

    /// Walk focus-local and annotation deactivation expressions in their model.
    fn deactivations(
        &mut self,
        expressions: &'model [NodeExpr],
        annotations: &'model [crate::shapes::ConstraintAnnotation],
    ) -> Result<(), E> {
        self.walk_deactivations(expressions, annotations)
    }

    fn walk_deactivations(
        &mut self,
        expressions: &'model [NodeExpr],
        annotations: &'model [crate::shapes::ConstraintAnnotation],
    ) -> Result<(), E> {
        for expression in expressions.iter().chain(
            annotations
                .iter()
                .flat_map(|annotation| &annotation.deactivated_when),
        ) {
            self.node_expr(expression)?;
        }
        Ok(())
    }

    /// Walk a constraint. `Constraint::NodeByExpression` is one of the two sites
    /// that carry a shape-index handle.
    ///
    /// # Errors
    ///
    /// A hook's refusal, or the depth ceiling.
    fn constraint(&mut self, constraint: &'model Constraint) -> Result<(), E> {
        self.walk_constraint(constraint)
    }

    /// Walk a constraint's original operands through the visitor hooks.
    fn walk_constraint(&mut self, constraint: &'model Constraint) -> Result<(), E> {
        self.enter()?;
        match constraint {
            Constraint::PropertyReference(edge) => {
                for expression in &edge.deactivated_when {
                    self.node_expr(expression)?;
                }
            }
            Constraint::Reference(_)
            | Constraint::Class(_)
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
            | Constraint::Component { .. } => {}
            Constraint::Pattern { .. } => self.pattern(constraint)?,
            Constraint::UniqueValuesFor { targets, .. } => self.targets(targets)?,
            Constraint::Not(shape)
            | Constraint::Node(shape)
            | Constraint::MemberShape(shape)
            | Constraint::SomeValue(shape) => {
                self.shape(shape)?;
            }
            Constraint::And(shapes) | Constraint::Or(shapes) | Constraint::Xone(shapes) => {
                for shape in shapes {
                    self.shape(shape)?;
                }
            }
            Constraint::QualifiedValueShape {
                shape, siblings, ..
            } => {
                self.shape(shape)?;
                for sibling in siblings {
                    self.shape(sibling)?;
                }
            }
            Constraint::Expression { expr, .. } => self.node_expr(expr)?,
            Constraint::NodeByExpression { expr, shapes, .. } => {
                self.shape_index(shapes)?;
                self.node_expr(expr)?;
            }
        }
        self.leave();
        Ok(())
    }

    /// Walk the shape argument of `shnex:conformsToShape`, the second site that
    /// carries a shape-index handle.
    ///
    /// # Errors
    ///
    /// A hook's refusal, or the depth ceiling.
    fn shape_arg(&mut self, arg: &'model ShapeArg) -> Result<(), E> {
        self.walk_shape_arg(arg)
    }

    /// Walk a conformance shape argument through the visitor hooks.
    fn walk_shape_arg(&mut self, arg: &'model ShapeArg) -> Result<(), E> {
        match arg {
            ShapeArg::Named(shape) => self.shape(shape),
            ShapeArg::Computed { expr, shapes } => {
                self.shape_index(shapes)?;
                self.node_expr(expr)
            }
        }
    }

    /// Walk a node expression.
    ///
    /// # Errors
    ///
    /// A hook's refusal, or the depth ceiling.
    fn node_expr(&mut self, expr: &'model NodeExpr) -> Result<(), E> {
        self.walk_node_expr(expr)
    }

    /// Walk an expression's operands through the visitor hooks. A collecting
    /// pass can inspect an expression before delegating its structural walk.
    fn walk_node_expr(&mut self, expr: &'model NodeExpr) -> Result<(), E> {
        self.enter()?;
        match expr {
            NodeExpr::Constant(_)
            | NodeExpr::This
            | NodeExpr::Path(_)
            | NodeExpr::Arg(_)
            | NodeExpr::Empty
            | NodeExpr::Var(_)
            | NodeExpr::List(_)
            | NodeExpr::Select { .. } => {}
            NodeExpr::Filter { nodes, shape }
            | NodeExpr::FindFirst { nodes, shape }
            | NodeExpr::MatchAll { nodes, shape } => {
                self.node_expr(nodes)?;
                self.shape(shape)?;
            }
            NodeExpr::Union(operands)
            | NodeExpr::Intersection(operands)
            | NodeExpr::Concat(operands) => {
                for operand in operands {
                    self.node_expr(operand)?;
                }
            }
            NodeExpr::If { cond, then, els } => {
                self.node_expr(cond)?;
                self.node_expr(then)?;
                self.node_expr(els)?;
            }
            NodeExpr::Count { of, .. }
            | NodeExpr::Distinct(of)
            | NodeExpr::Min(of)
            | NodeExpr::Max(of)
            | NodeExpr::Sum(of)
            | NodeExpr::Limit { of, .. }
            | NodeExpr::Offset { of, .. }
            | NodeExpr::InstancesOf(of)
            | NodeExpr::Exists(of) => self.node_expr(of)?,
            NodeExpr::OrderBy { of, key, .. } => {
                self.node_expr(of)?;
                self.node_expr(key)?;
            }
            NodeExpr::Call(
                FnCall::Builtin { args, .. }
                | FnCall::UserDefined { args, .. }
                | FnCall::Sparql { args, .. },
            ) => {
                for arg in args {
                    self.node_expr(arg)?;
                }
            }
            NodeExpr::CustomCall { args, .. } => {
                for (_, arg) in args {
                    self.node_expr(arg)?;
                }
            }
            NodeExpr::Owned { expr, owner } => {
                self.model(owner)?;
                self.node_expr(expr)?;
            }
            NodeExpr::PathValues { focus, .. } => self.node_expr(focus)?,
            NodeExpr::Remove { nodes, remove } => {
                self.node_expr(nodes)?;
                self.node_expr(remove)?;
            }
            NodeExpr::FlatMap { nodes, map } => {
                self.node_expr(nodes)?;
                self.node_expr(map)?;
            }
            NodeExpr::NodesMatching(shape) => self.shape(shape)?,
            NodeExpr::ConformsToShape { node, shape } => {
                self.node_expr(node)?;
                self.shape_arg(shape)?;
            }
        }
        self.leave();
        Ok(())
    }
}

/// The linking pass's [`ModelWalk`]: proves every shape-index handle it reaches is
/// the ONE handle the graph shares.
struct ShapeIndexWalk<'a> {
    /// The single handle every reachable site must be [`Arc::ptr_eq`] to.
    index: &'a ShapeIndex,
    /// The live walk depth.
    depth: u32,
    /// Declarations already walked, by handle identity. A body may call its own
    /// function, so this is what makes the cyclic value graph terminate.
    seen_fns: Vec<Arc<CustomFunction>>,
    seen_catalogs: Vec<usize>,
}

impl ModelWalk<'_> for ShapeIndexWalk<'_> {
    fn first_owner(
        &mut self,
        owner: &Arc<crate::shapes::ModelOwner>,
    ) -> Result<bool, ShapesProductError> {
        let address = Arc::as_ptr(owner).addr();
        if self.seen_catalogs.contains(&address) {
            return Ok(false);
        }
        self.seen_catalogs.push(address);
        Ok(true)
    }
    fn depth(&mut self) -> &mut u32 {
        &mut self.depth
    }

    fn first_declaration(
        &mut self,
        func: &Arc<CustomFunction>,
    ) -> Result<bool, ShapesProductError> {
        if self.seen_fns.iter().any(|seen| Arc::ptr_eq(seen, func)) {
            return Ok(false);
        }
        self.seen_fns.push(Arc::clone(func));
        Ok(true)
    }

    fn shape_index(&mut self, found: &ShapeIndexRef) -> Result<(), ShapesProductError> {
        if found.ptr_eq(&Arc::downgrade(self.index)) {
            return Ok(());
        }
        Err(malformed(
            "this shapes model carries a sh:nodeByExpression shape index that is NOT the one \
             handle the graph shares; build the model so every sh:nodeByExpression constraint and \
             every computed shape argument clones the SAME Arc, because a per-site handle \
             type-checks, allocates one index per constraint instead of one per graph, and leaves \
             every clone permanently empty — the constraint would then resolve no shape at all \
             while the shapes graph loaded green and reported nothing",
        ))
    }
}

#[cfg(test)]
mod tests;
