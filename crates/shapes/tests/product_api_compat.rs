// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Compile-time proof of the public-API compatibility guarantee: existing
//! constructors, public fields, traits and execution APIs stay usable, and no
//! required public struct field or breaking exhaustive-enum change slips in.
//!
//! This file runs no fuzzing and needs no assertions to do its real job — the
//! job is that it COMPILES. Two techniques carry the whole file, and each turns
//! a specific kind of breaking change into a build failure rather than a runtime
//! surprise a downstream crate discovers later:
//!
//! 1. **Bare struct literals.** A literal that names every field, with no
//!    `..Default::default()` and no `..` rest pattern, stops compiling the
//!    moment a NEW field is added to that struct — because the literal no
//!    longer names all of them. That is exactly the shape of the forbidden
//!    change ("no required public struct fields"): a field can be *added* only
//!    if a caller who never learns about it keeps compiling, and a bare literal
//!    is the caller who has the least room to keep compiling of any of them. If
//!    a struct cannot be built this way at all — because it already carries a
//!    private field, which also rules out `..Default::default()` struct-update
//!    syntax, since that syntax still has to name the skipped field's type at
//!    the use site — the guarantee is instead upheld by [`Default`] plus plain
//!    field ASSIGNMENT, and this file proves THAT path compiles rather than
//!    pretending the bare-literal or struct-update path exists where neither
//!    ever did (see [`Shapes`] below).
//! 2. **Wildcard-free exhaustive matches.** A `match` with no `_` arm stops
//!    compiling the moment a NEW variant is added to that enum — because the
//!    match no longer covers every case. That is exactly the shape of the other
//!    forbidden change ("no breaking exhaustive enum changes"): a variant can be
//!    added only if code that matched on the enum before the addition still
//!    compiles, and a wildcard-free match is the code with the least room to
//!    keep compiling.
//!
//! **The compile error IS the point.** A future change that adds a required
//! public field or a new variant to one of the types named here is exactly the
//! change this file exists to catch, and the fix is never to edit this file to
//! make it compile again — it is to make the change non-breaking (give the new
//! field a default and add it to the relevant `Default` impl instead of here;
//! reconsider whether the enum needed a new variant, or accept that the
//! addition is a genuine major-version break and version it as one).
//!
//! Every construction below also feeds a tiny runtime assertion about the value
//! it built. That is not incidental: a compile-only check can be optimised away
//! by a refactor that stops actually using the constructed value, and would then
//! silently stop proving anything while still reporting green. Asserting a fact
//! about the value forces every construction to remain live.

use std::sync::Arc;

use purrdf_core::artifact::Identity;
use purrdf_shapes::engine::PreparedShapes;
use purrdf_shapes::expression::NodeExpr;
use purrdf_shapes::product::{HostBindings, ProductDimension, ShapesProductError, ShapesProfile};
use purrdf_shapes::provenance::{ProductRestore, ValidatorProvenance};
use purrdf_shapes::report::Severity;
use purrdf_shapes::shapes::{Constraint, Path, PropertyShape, Shape, Shapes, Target};
use purrdf_shapes::term::NamedNode;
use purrdf_sparql_eval::{AggregateRegistry, PropertyFunctionRegistry, UserFunctionRegistry};

fn iri(s: &str) -> NamedNode {
    NamedNode::new_unchecked(s)
}

// ═══════════════════════════════════════════════════════════════════════════════
// 1. Bare struct literals — no required public field can appear unannounced
// ═══════════════════════════════════════════════════════════════════════════════

/// [`Shape`] has no [`Default`] impl and every field is public, so the ONLY way a
/// downstream caller builds one by hand is a literal naming every field. That
/// literal is written here with no `..` of any kind: a new required field on
/// `Shape` stops this from compiling.
#[test]
fn shape_is_still_a_bare_literal() {
    let shape = Shape {
        id: iri("https://example.org/PersonShape").into_term(),
        targets: vec![Target::Class(iri("https://example.org/Person"))],
        constraints: vec![Constraint::MinCount(1)],
        property_shapes: Vec::<PropertyShape>::new(),
        severity: Severity::Violation,
        message: None,
        deactivated: false,
        box_roles: Vec::new(),
        rules: Vec::new(),
    };
    assert_eq!(
        shape.constraints.len(),
        1,
        "the literal above must have built a real shape"
    );
}

/// The mirror check for [`PropertyShape`], reached via `sh:property` — same
/// reasoning, same no-Default, all-public-fields shape.
#[test]
fn property_shape_is_still_a_bare_literal() {
    let property_shape = PropertyShape {
        id: iri("https://example.org/nameProp").into_term(),
        path: Path::Predicate(iri("https://example.org/name")),
        constraints: vec![Constraint::MinCount(1)],
        property_shapes: Vec::new(),
        reifier_shapes: Vec::new(),
        reification_required: false,
        severity: Severity::Violation,
        message: None,
        deactivated: false,
        box_roles: Vec::new(),
    };
    assert!(
        matches!(property_shape.path, Path::Predicate(_)),
        "the literal above must have built a real property shape",
    );
}

/// [`Shapes`] is the opposite case from [`Shape`]: it carries `pub(crate)`
/// fields (`shapes_dataset`, `parse_provenance`), and that visibility means
/// NEITHER a bare literal NOR `..Default::default()` struct-update syntax
/// compiles from outside this crate — functional update still has to name every
/// skipped field's TYPE at the use site to copy it, and a private field's type
/// is exactly what an external caller cannot name. (This is not new: the
/// pre-existing `shapes_dataset` field already had this effect before
/// `parse_provenance` existed.) The only construction path a downstream caller
/// has ever had is [`Shapes::default()`] as a value, followed by plain field
/// ASSIGNMENT into the individual public fields it wants to set — which is
/// exactly what this test does, and exactly why adding `parse_provenance` cost
/// nothing: [`Default`] absorbs a new field as long as the field has a sensible
/// default, and no external assignment syntax ever has to name it. A change
/// that broke this guarantee would be turning an existing PUBLIC field private,
/// or removing [`Default`] from `Shapes` altogether — either stops the
/// assignment below from compiling.
#[test]
fn shapes_still_builds_via_default_then_field_assignment() {
    let shape = Shape {
        id: iri("https://example.org/PersonShape").into_term(),
        targets: Vec::new(),
        constraints: Vec::new(),
        property_shapes: Vec::new(),
        severity: Severity::Violation,
        message: None,
        deactivated: false,
        box_roles: Vec::new(),
        rules: Vec::new(),
    };
    let mut shapes = Shapes::default();
    shapes.node_shapes = vec![shape];
    assert_eq!(shapes.node_shapes.len(), 1);
    // The accessor surface `Shapes::provenance` added stays reachable on a
    // value nobody constructed through the parser, and reports the "nothing
    // supplied" defaults rather than panicking.
    assert!(shapes.provenance().base().is_none());
}

// ═══════════════════════════════════════════════════════════════════════════════
// 2. Wildcard-free exhaustive matches — no variant can appear unannounced
// ═══════════════════════════════════════════════════════════════════════════════

/// Every [`Constraint`] variant, named with NO `_ =>` arm. Adding a new
/// constraint variant to the SHACL model stops this function compiling.
///
/// Struct-variant arms use `{ .. }` to skip their fields — that skips FIELDS of
/// a named variant, which is unrelated to skipping a whole VARIANT with a
/// top-level wildcard; a new variant still has no arm to match here.
fn constraint_name(constraint: &Constraint) -> &'static str {
    match constraint {
        Constraint::Class(_) => "class",
        Constraint::Datatype(_) => "datatype",
        Constraint::NodeKind(_) => "node-kind",
        Constraint::MinCount(_) => "min-count",
        Constraint::MaxCount(_) => "max-count",
        Constraint::In(_) => "in",
        Constraint::HasValue(_) => "has-value",
        Constraint::Pattern { .. } => "pattern",
        Constraint::MinLength(_) => "min-length",
        Constraint::MaxLength(_) => "max-length",
        Constraint::UniqueLang(_) => "unique-lang",
        Constraint::LanguageIn(_) => "language-in",
        Constraint::Not(_) => "not",
        Constraint::Closed { .. } => "closed",
        Constraint::MinInclusive(_) => "min-inclusive",
        Constraint::MaxInclusive(_) => "max-inclusive",
        Constraint::MinExclusive(_) => "min-exclusive",
        Constraint::MaxExclusive(_) => "max-exclusive",
        Constraint::And(_) => "and",
        Constraint::Or(_) => "or",
        Constraint::Xone(_) => "xone",
        Constraint::Node(_) => "node",
        Constraint::Sparql { .. } => "sparql",
        Constraint::Equals(_) => "equals",
        Constraint::Disjoint(_) => "disjoint",
        Constraint::LessThan(_) => "less-than",
        Constraint::LessThanOrEquals(_) => "less-than-or-equals",
        Constraint::QualifiedValueShape { .. } => "qualified-value-shape",
        Constraint::Expression { .. } => "expression",
        Constraint::NodeByExpression { .. } => "node-by-expression",
        Constraint::Component { .. } => "component",
    }
}

#[test]
fn constraint_match_is_exhaustive_and_reachable() {
    let constraint = Constraint::MinCount(1);
    assert_eq!(constraint_name(&constraint), "min-count");
}

/// Every [`NodeExpr`] variant (SHACL-AF plus the SHACL 1.2 Node Expressions and
/// SPARQL-extension arms), named with NO `_ =>` arm. Adding a new node
/// expression form stops this function compiling.
fn node_expr_name(expr: &NodeExpr) -> &'static str {
    match expr {
        NodeExpr::Constant(_) => "constant",
        NodeExpr::This => "this",
        NodeExpr::Path(_) => "path",
        NodeExpr::Filter { .. } => "filter",
        NodeExpr::Union(_) => "union",
        NodeExpr::Intersection(_) => "intersection",
        NodeExpr::If { .. } => "if",
        NodeExpr::Count { .. } => "count",
        NodeExpr::Distinct(_) => "distinct",
        NodeExpr::Min(_) => "min",
        NodeExpr::Max(_) => "max",
        NodeExpr::Sum(_) => "sum",
        NodeExpr::Limit { .. } => "limit",
        NodeExpr::Offset { .. } => "offset",
        NodeExpr::OrderBy { .. } => "order-by",
        NodeExpr::Exists(_) => "exists",
        NodeExpr::Call(_) => "call",
        NodeExpr::Arg(_) => "arg",
        NodeExpr::CustomCall { .. } => "custom-call",
        NodeExpr::Empty => "empty",
        NodeExpr::Var(_) => "var",
        NodeExpr::List(_) => "list",
        NodeExpr::PathValues { .. } => "path-values",
        NodeExpr::Concat(_) => "concat",
        NodeExpr::Remove { .. } => "remove",
        NodeExpr::FlatMap { .. } => "flat-map",
        NodeExpr::FindFirst { .. } => "find-first",
        NodeExpr::MatchAll { .. } => "match-all",
        NodeExpr::InstancesOf(_) => "instances-of",
        NodeExpr::NodesMatching(_) => "nodes-matching",
        NodeExpr::ConformsToShape { .. } => "conforms-to-shape",
        NodeExpr::Select { .. } => "select",
    }
}

#[test]
fn node_expr_match_is_exhaustive_and_reachable() {
    assert_eq!(node_expr_name(&NodeExpr::This), "this");
}

/// Every [`ProductDimension`] variant, named with NO `_ =>` arm, from OUTSIDE
/// the crate. `crates/shapes/src/product/error.rs` already carries an in-crate
/// exhaustive match over `ProductDimension::ALL`; this one exercises the same
/// guarantee from where a downstream caller actually sits, which is the
/// perspective the compatibility promise is about.
const fn dimension_ordinal(dimension: ProductDimension) -> u8 {
    match dimension {
        ProductDimension::Magic => 0,
        ProductDimension::FormatVersion => 1,
        ProductDimension::StageId => 2,
        ProductDimension::Profile => 3,
        ProductDimension::Truncated => 4,
        ProductDimension::Trailer => 5,
        ProductDimension::SectionDigest => 6,
        ProductDimension::ContainerDigest => 7,
        ProductDimension::DatasetIdentity => 8,
        ProductDimension::ShapesGraph => 9,
        ProductDimension::Prefixes => 10,
        ProductDimension::Base => 11,
        ProductDimension::Vocabulary => 12,
        ProductDimension::FunctionRegistry => 13,
        ProductDimension::AggregateRegistry => 14,
        ProductDimension::PropertyFunctionRegistry => 15,
        ProductDimension::ClassCatalog => 16,
        ProductDimension::ParseConfiguration => 17,
        ProductDimension::UnsupportedCapability => 18,
        ProductDimension::DepthLimit => 19,
        ProductDimension::Malformed => 20,
    }
}

#[test]
fn product_dimension_match_is_exhaustive_and_reachable() {
    for dimension in ProductDimension::ALL {
        // Round-tripping through the ordinal proves each arm above is reached
        // (a duplicate or transposed arm would fail this for some dimension),
        // not merely that the match compiles.
        assert_eq!(
            ProductDimension::ALL[dimension_ordinal(dimension) as usize],
            dimension,
        );
    }
}

/// Both [`ValidatorProvenance`] variants, named with NO `_ =>` arm. This is the
/// newest public enum on the prepared-product surface — adding a third way a
/// preparation could have come into existence stops this compiling.
fn validator_provenance_name(provenance: &ValidatorProvenance) -> &'static str {
    match provenance {
        ValidatorProvenance::Parsed => "parsed",
        ValidatorProvenance::Restored { .. } => "restored",
    }
}

#[test]
fn validator_provenance_match_is_exhaustive_and_reachable() {
    assert_eq!(
        validator_provenance_name(&ValidatorProvenance::Parsed),
        "parsed"
    );
    let restored = ValidatorProvenance::Restored {
        identity: Identity::default(),
        restore: ProductRestore::Admitted,
    };
    assert_eq!(validator_provenance_name(&restored), "restored");
    assert_eq!(restored.product_identity(), Some(&Identity::default()));
}

/// Both [`ProductRestore`] seams, named with NO `_ =>` arm.
fn product_restore_name(restore: ProductRestore) -> &'static str {
    match restore {
        ProductRestore::Admitted => "admitted",
        ProductRestore::Rebuilt => "rebuilt",
    }
}

#[test]
fn product_restore_match_is_exhaustive_and_reachable() {
    assert_eq!(product_restore_name(ProductRestore::Admitted), "admitted");
    assert_eq!(product_restore_name(ProductRestore::Rebuilt), "rebuilt");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 3. Public constructors, called with their original signatures
// ═══════════════════════════════════════════════════════════════════════════════

/// `Shapes::default()`, `PreparedShapes::new`, `HostBindings::new` /
/// `HostBindings::empty`, `ShapesProfile::CORE` and `ShapesProductError::new`
/// are the entry points a host actually holds onto across a restore. A
/// signature change to any of these — a reordered or added non-defaulted
/// parameter — stops this test compiling.
#[test]
fn public_constructors_keep_their_signatures() {
    let prepared = PreparedShapes::new(Arc::new(Shapes::default()));
    assert_eq!(prepared.shapes().node_shapes.len(), 0);

    let profile = ShapesProfile::CORE;
    assert_eq!(profile.id(), ShapesProfile::CORE.id());

    let functions = UserFunctionRegistry::new();
    let aggregates = AggregateRegistry::new();
    let relations = PropertyFunctionRegistry::new();
    let empty = HostBindings::empty();
    let bound =
        HostBindings::without_declarations(&functions, &aggregates, &relations, b"build-id");
    assert_eq!(empty.implementation_identity(), b"");
    assert_eq!(bound.implementation_identity(), b"build-id");

    let error = ShapesProductError::new(ProductDimension::Magic, "not a product");
    assert_eq!(error.dimension(), ProductDimension::Magic);
    assert_eq!(error.message(), "not a product");

    let identity = Identity::new().with("example", b"value");
    assert_eq!(identity.components().len(), 1);
}
