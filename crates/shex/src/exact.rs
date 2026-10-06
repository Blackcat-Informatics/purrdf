// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! [`ExactSchema`]: a parsed [`Schema`] with its numeric facet bounds kept exactly as
//! written.
//!
//! [`NumericLiteral`](crate::NumericLiteral), the AST's facet value, follows ShExJ's
//! JSON numbers: an `i64` or an `f64`. That is lossy. `MININCLUSIVE
//! 100000000000000000001` becomes the double `1e20`, `MAXINCLUSIVE
//! 5.0000000000000000001` becomes the integer `5`, and `MINEXCLUSIVE 0.1` becomes the
//! double just above one tenth, so a facet compared through it gives wrong verdicts at
//! the edges. ShEx 2.1 compares a node's value with the facet's numeric literal in
//! the XSD value space, which needs the bound's own digits.
//!
//! Distinct bounds can share one `i64`/`f64` (`5` and `5.0000000000000000001`, or
//! `0.30000000000000001` and `0.30000000000000002`), so a bound is never looked up by
//! that value: it belongs to the node constraint it was written on, and validation
//! finds it by that constraint's identity.
//!
//! An [`ExactSchema`] keeps them. Its parsers record each numeric facet bound's
//! lexical form and datatype (`xsd:integer`, `xsd:decimal` or `xsd:double`, from the
//! ShExC token or the ShExJ number's spelling) beside the AST, and
//! [`validate_exact`] / [`validate_shape_map_exact`] compare every node against those
//! bounds exactly, for numbers of any size ([`purrdf_xsd::value_cmp`]). The
//! [`Schema`] itself is unchanged, so every existing consumer of the AST keeps
//! working; a schema built in code, with no source text, holds its `i64`/`f64` bounds
//! exactly as the values they are ([`ExactSchema::from_schema`]).

use std::cell::RefCell;

use purrdf_core::{FastMap, RdfDataset, TermValue};
use purrdf_xsd::{XsdDatatype, XsdValue};

use purrdf_lex::json::{Number, Value};

use crate::ast::{NodeConstraint, NumericLiteral, Schema, ShapeExpr, TripleExpr};
use crate::error::{Result, ShexError};
use crate::validate::{ResultShapeMap, ShapeSelector, ValidationOptions};

/// One numeric facet bound as written: its lexical form and the XSD datatype the
/// syntax gives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExactBound {
    lexical: String,
    datatype: XsdDatatype,
}

impl ExactBound {
    /// A ShExC `INTEGER` / `DECIMAL` / `DOUBLE` token, or a ShExJ number's lexeme
    /// (an exponent makes it a double, a point a decimal, neither an integer).
    pub(crate) fn from_lexical(lexical: &str) -> Self {
        let datatype = if lexical.contains(['e', 'E']) {
            XsdDatatype::Double
        } else if lexical.contains('.') {
            XsdDatatype::Decimal
        } else {
            XsdDatatype::Integer
        };
        Self {
            lexical: lexical.to_owned(),
            datatype,
        }
    }

    /// The value the bound denotes.
    fn value(&self) -> Option<XsdValue> {
        purrdf_xsd::parse(&self.lexical, self.datatype).ok()
    }

    /// The JSON number to write for this bound in place of `ast`'s ShExJ spelling, or
    /// `None` when that spelling already reads back as this bound.
    pub(crate) fn spelling_over(&self, ast: NumericLiteral) -> Option<Number> {
        let plain = crate::shexj::numeric_to_value(ast);
        if let Value::Number(number) = &plain
            && Self::from_lexical(number.lexeme()).same_bound(self)
        {
            return None;
        }
        let lexical = match self.value()? {
            XsdValue::Double(d) => purrdf_xsd::numeric::canonical_double(d),
            value @ (XsdValue::Decimal(_) | XsdValue::BigDecimal(_)) => {
                let digits = value.canonical_lexical();
                if digits.contains('.') {
                    digits
                } else {
                    format!("{digits}.0")
                }
            }
            value => value.canonical_lexical(),
        };
        Number::from_lexeme(lexical).ok()
    }

    /// Whether `self` and `other` decide every comparison alike: both doubles, or
    /// both exact (`xsd:integer`/`xsd:decimal`), with one value — so `5`, `05` and
    /// `5.0` are one bound, while `5` and `5.0000000000000000001`, or `1.5` and
    /// `1.5E0`, are not.
    fn same_bound(&self, other: &Self) -> bool {
        match (self.value(), other.value()) {
            (Some(a), Some(b)) => {
                matches!(a, XsdValue::Double(_)) == matches!(b, XsdValue::Double(_))
                    && purrdf_xsd::value_cmp(&a, &b) == Some(core::cmp::Ordering::Equal)
            }
            _ => self == other,
        }
    }
}

/// The AST stand-in for an `INTEGER`/`DECIMAL` bound past the `f64` range: the
/// finite double of largest magnitude, with the bound's sign (see
/// [`ExactSchema::schema`]).
pub(crate) fn saturated(lexical: &str) -> NumericLiteral {
    if lexical.starts_with('-') {
        NumericLiteral::Fractional(-f64::MAX)
    } else {
        NumericLiteral::Fractional(f64::MAX)
    }
}

/// The exact bounds of one node constraint's numeric range facets, in the order
/// `MININCLUSIVE`, `MINEXCLUSIVE`, `MAXINCLUSIVE`, `MAXEXCLUSIVE`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ExactBounds {
    pub(crate) slots: [Option<ExactBound>; 4],
}

impl ExactBounds {
    /// Whether no facet bound was recorded.
    pub(crate) fn is_empty(&self) -> bool {
        self.slots.iter().all(Option::is_none)
    }

    /// Whether every slot of `self` holds the same bound as `other`'s
    /// ([`ExactBound::same_bound`]).
    fn same_bounds(&self, other: &Self) -> bool {
        self.slots.iter().zip(&other.slots).all(|pair| match pair {
            (None, None) => true,
            (Some(a), Some(b)) => a.same_bound(b),
            _ => false,
        })
    }

    /// Whether the recorded slots are exactly the facets `nc` carries.
    fn matches(&self, nc: &NodeConstraint) -> bool {
        let present = [
            nc.mininclusive.is_some(),
            nc.minexclusive.is_some(),
            nc.maxinclusive.is_some(),
            nc.maxexclusive.is_some(),
        ];
        self.slots
            .iter()
            .zip(present)
            .all(|(slot, present)| slot.is_some() == present)
    }
}

/// The exact bounds of every node constraint that carries a numeric range facet, per
/// owner of a shape expression: the `start` expression, and each declaration in
/// `Schema::shapes` order. Within an owner the bounds are in pre-order, which is the
/// order the parsers create the constraints in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ExactTable {
    pub(crate) start: Vec<ExactBounds>,
    pub(crate) shapes: Vec<Vec<ExactBounds>>,
}

/// The node constraints of `expr` that carry a numeric range facet, in pre-order.
fn numeric_constraints(expr: &ShapeExpr) -> Vec<&NodeConstraint> {
    enum Item<'a> {
        Shape(&'a ShapeExpr),
        Triple(&'a TripleExpr),
    }
    let mut out = Vec::new();
    let mut pending = vec![Item::Shape(expr)];
    while let Some(item) = pending.pop() {
        match item {
            Item::Shape(ShapeExpr::And(parts) | ShapeExpr::Or(parts)) => {
                pending.extend(parts.iter().rev().map(Item::Shape));
            }
            Item::Shape(ShapeExpr::Not(inner)) => pending.push(Item::Shape(inner)),
            Item::Shape(ShapeExpr::Node(nc)) => {
                if nc.mininclusive.is_some()
                    || nc.minexclusive.is_some()
                    || nc.maxinclusive.is_some()
                    || nc.maxexclusive.is_some()
                {
                    out.push(nc);
                }
            }
            Item::Shape(ShapeExpr::Shape(shape)) => {
                if let Some(triple) = &shape.expression {
                    pending.push(Item::Triple(triple));
                }
            }
            Item::Shape(ShapeExpr::External | ShapeExpr::Ref(_)) => {}
            Item::Triple(TripleExpr::EachOf(group) | TripleExpr::OneOf(group)) => {
                pending.extend(group.expressions.iter().rev().map(Item::Triple));
            }
            Item::Triple(TripleExpr::TripleConstraint(tc)) => {
                if let Some(value) = &tc.value_expr {
                    pending.push(Item::Shape(value));
                }
            }
            Item::Triple(TripleExpr::Ref(_)) => {}
        }
    }
    out
}

/// The bounds a code-built constraint's `i64`/`f64` facet values are exactly.
fn bounds_of(nc: &NodeConstraint) -> ExactBounds {
    let bound = |value: Option<NumericLiteral>| {
        value.map(|value| match value {
            NumericLiteral::Integer(i) => ExactBound {
                lexical: i.to_string(),
                datatype: XsdDatatype::Integer,
            },
            NumericLiteral::Fractional(f) => ExactBound {
                lexical: purrdf_xsd::numeric::canonical_double(f),
                datatype: XsdDatatype::Double,
            },
        })
    };
    ExactBounds {
        slots: [
            bound(nc.mininclusive),
            bound(nc.minexclusive),
            bound(nc.maxinclusive),
            bound(nc.maxexclusive),
        ],
    }
}

/// Check that `groups` are, in order, the bounds of `expr`'s numeric constraints.
fn aligned(expr: &ShapeExpr, groups: &[ExactBounds]) -> bool {
    let constraints = numeric_constraints(expr);
    constraints.len() == groups.len()
        && constraints
            .iter()
            .zip(groups)
            .all(|(nc, bounds)| bounds.matches(nc))
}

/// A ShEx schema whose numeric facet bounds are kept exactly as written: its parsers
/// record each bound's digits beside the AST, and [`validate_exact`] compares every
/// node's value against them in the XSD value space.
///
/// ```
/// use purrdf_core::TermValue;
/// use purrdf_rdf::parse_dataset;
/// use purrdf_shex::{ConformanceStatus, ExactSchema, ShapeSelector, validate_exact};
///
/// // As a double, 100000000000000000001 is 1e20, which the value 1e20 would meet.
/// let schema = ExactSchema::parse_shexc(
///     "<http://example.org/S> { <http://example.org/n> MININCLUSIVE 100000000000000000001 }",
///     None,
/// )?;
/// let data = parse_dataset(
///     b"<http://example.org/x> <http://example.org/n> 100000000000000000000 .",
///     "text/turtle",
///     None,
/// )
/// .expect("data parses");
/// let map = vec![(
///     TermValue::Iri("http://example.org/x".to_owned()),
///     ShapeSelector::Label("http://example.org/S".to_owned()),
/// )];
/// let result = validate_exact(&schema, &data, &map, &Default::default());
/// assert_eq!(result.entries[0].status, ConformanceStatus::Nonconformant);
/// # Ok::<(), purrdf_shex::ShexError>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct ExactSchema {
    schema: Schema,
    table: ExactTable,
}

impl ExactSchema {
    /// Parse ShExC, as [`crate::parse_shexc`], keeping each numeric facet bound's
    /// digits.
    ///
    /// # Errors
    ///
    /// As [`crate::parse_shexc`].
    pub fn parse_shexc(input: &str, base: Option<&str>) -> Result<Self> {
        let (schema, table) = crate::parser::parse_shexc_exact(input, base, true)?;
        Self::checked(schema, table)
    }

    /// Parse ShExJ, as [`crate::parse_shexj`], keeping each numeric facet bound's
    /// lexeme.
    ///
    /// # Errors
    ///
    /// As [`crate::parse_shexj`].
    pub fn parse_shexj(input: &str, base: Option<&str>) -> Result<Self> {
        let (schema, table) = crate::shexj::parse_shexj_exact(input, base, true)?;
        Self::checked(schema, table)
    }

    /// A schema built in code, whose `i64`/`f64` facet bounds are exactly the
    /// values they hold.
    #[must_use]
    pub fn from_schema(schema: Schema) -> Self {
        let owner = |expr: &ShapeExpr| -> Vec<ExactBounds> {
            numeric_constraints(expr)
                .into_iter()
                .map(bounds_of)
                .collect()
        };
        let table = ExactTable {
            start: schema.start.as_deref().map_or_else(Vec::new, owner),
            shapes: schema.shapes.iter().map(|decl| owner(&decl.expr)).collect(),
        };
        Self { schema, table }
    }

    /// Pair a parsed schema with the bounds its parser recorded, refusing a pairing
    /// that does not line up (an internal invariant: the parsers record one group per
    /// numeric constraint, in pre-order).
    fn checked(schema: Schema, table: ExactTable) -> Result<Self> {
        let start_ok = match schema.start.as_deref() {
            Some(start) => aligned(start, &table.start),
            None => table.start.is_empty(),
        };
        let shapes_ok = schema.shapes.len() == table.shapes.len()
            && schema
                .shapes
                .iter()
                .zip(&table.shapes)
                .all(|(decl, groups)| aligned(&decl.expr, groups));
        if start_ok && shapes_ok {
            Ok(Self { schema, table })
        } else {
            Err(ShexError::syntax(
                "numeric facet bounds do not line up with the parsed schema",
                0,
            ))
        }
    }

    /// The schema.
    ///
    /// Its numeric facets are the AST's `i64`/`f64` approximations of the bounds this
    /// schema keeps exactly. An `INTEGER`/`DECIMAL` bound past the `f64` range, which
    /// [`ExactSchema::parse_shexc`] / [`ExactSchema::parse_shexj`] accept and
    /// [`crate::parse_shexc`] / [`crate::parse_shexj`] refuse, stands there as the
    /// finite `NumericLiteral::Fractional(f64::MAX)` (or `-f64::MAX` for a negative
    /// bound): the nearest value the AST holds, not the bound. [`validate_exact`] and
    /// [`ExactSchema::to_shexj`] use the bound itself; [`crate::validate()`] and
    /// [`crate::to_shexj`] over this AST see only the stand-in.
    #[must_use]
    pub const fn schema(&self) -> &Schema {
        &self.schema
    }

    /// The schema, without its exact bounds.
    #[must_use]
    pub fn into_schema(self) -> Schema {
        self.schema
    }

    /// Flatten this schema and its transitive imports, as [`crate::resolve_imports`],
    /// keeping every declaration's exact bounds. `resolver` supplies each imported
    /// schema, itself parsed exactly.
    ///
    /// # Errors
    ///
    /// As [`crate::resolve_imports`]; [`ShexError::ImportConflict`] also when two
    /// schemas declare one label with the same AST but exact bounds of different
    /// values (`MAXINCLUSIVE 5` and `MAXINCLUSIVE 5.0000000000000000001`, one `5` in
    /// the AST). Bounds spelled differently with one value (`5` and `5.0`) merge.
    pub fn resolve_imports(self, resolver: &dyn Fn(&str) -> Result<Self>) -> Result<Self> {
        // Every source's declarations with their bounds, root first, then imports in
        // the order `resolve_imports` asks for them.
        let sources: RefCell<Vec<(Schema, ExactTable)>> =
            RefCell::new(vec![(self.schema.clone(), self.table.clone())]);
        let merged = crate::imports::resolve_imports(self.schema, &|iri| {
            let imported = resolver(iri)?;
            sources
                .borrow_mut()
                .push((imported.schema.clone(), imported.table));
            Ok(imported.schema)
        })?;
        let sources = sources.into_inner();
        let mut shapes = Vec::with_capacity(merged.shapes.len());
        for decl in &merged.shapes {
            let mut found: Option<&Vec<ExactBounds>> = None;
            for (schema, table) in &sources {
                for (candidate, groups) in schema.shapes.iter().zip(&table.shapes) {
                    if candidate != decl {
                        continue;
                    }
                    match found {
                        None => found = Some(groups),
                        Some(first)
                            if first.len() == groups.len()
                                && first.iter().zip(groups).all(|(a, b)| a.same_bounds(b)) => {}
                        Some(_) => return Err(ShexError::import_conflict(decl.id.clone())),
                    }
                }
            }
            shapes.push(found.cloned().unwrap_or_default());
        }
        let table = ExactTable {
            start: self.table.start,
            shapes,
        };
        Self::checked(merged, table)
    }

    /// Every numeric constraint of the schema, by address, with its exact bounds as
    /// values — built per validation call, while the schema is borrowed and cannot
    /// move.
    pub(crate) fn bounds_by_constraint(&self) -> crate::validate::ExactBoundsMap {
        self.by_constraint(|_, bounds| {
            [0, 1, 2, 3].map(|i| bounds.slots[i].as_ref().and_then(ExactBound::value))
        })
    }

    /// Serialize to ShExJ, as [`crate::to_shexj`] over [`ExactSchema::schema`], but
    /// with each numeric facet written as the bound this schema keeps: an integer or
    /// decimal bound as its exact digits, a double bound in the canonical XSD double
    /// spelling (so it reads back as a double). A facet the AST's number already
    /// spells faithfully is written as [`crate::to_shexj`] writes it, so a schema
    /// whose bounds the AST holds exactly writes the same bytes.
    ///
    /// ```
    /// use purrdf_shex::ExactSchema;
    ///
    /// let schema = ExactSchema::parse_shexc(
    ///     "<http://example.org/S> { <http://example.org/n> MAXINCLUSIVE 5.0000000000000000001 }",
    ///     None,
    /// )?;
    /// assert!(schema.to_shexj().contains("5.0000000000000000001"));
    /// assert!(!purrdf_shex::to_shexj(schema.schema()).contains("5.0000000000000000001"));
    /// # Ok::<(), purrdf_shex::ShexError>(())
    /// ```
    #[must_use]
    pub fn to_shexj(&self) -> String {
        let spellings = self.by_constraint(|nc, bounds| {
            let ast = [
                nc.mininclusive,
                nc.minexclusive,
                nc.maxinclusive,
                nc.maxexclusive,
            ];
            [0, 1, 2, 3].map(|i| match (ast[i], &bounds.slots[i]) {
                (Some(value), Some(bound)) => bound.spelling_over(value),
                _ => None,
            })
        });
        crate::shexj::to_shexj_with(&self.schema, Some(&spellings))
    }

    /// `per` of every numeric constraint of the schema and its exact bounds, by the
    /// constraint's address — built per call, while the schema is borrowed and cannot
    /// move.
    fn by_constraint<T>(
        &self,
        per: impl Fn(&NodeConstraint, &ExactBounds) -> T,
    ) -> FastMap<usize, T> {
        let mut map = FastMap::default();
        let mut add = |expr: &ShapeExpr, groups: &[ExactBounds]| {
            for (nc, bounds) in numeric_constraints(expr).into_iter().zip(groups) {
                map.insert(std::ptr::from_ref(nc) as usize, per(nc, bounds));
            }
        };
        if let Some(start) = self.schema.start.as_deref() {
            add(start, &self.table.start);
        }
        for (decl, groups) in self.schema.shapes.iter().zip(&self.table.shapes) {
            add(&decl.expr, groups);
        }
        map
    }
}

/// [`crate::validate_with`] over an [`ExactSchema`]: every numeric facet compares the
/// node's value against the bound exactly as written.
#[must_use]
pub fn validate_exact(
    schema: &ExactSchema,
    data: &RdfDataset,
    map: &[(TermValue, ShapeSelector)],
    options: &ValidationOptions<'_>,
) -> ResultShapeMap {
    let bounds = schema.bounds_by_constraint();
    crate::validate::validate_with_bounds(&schema.schema, data, map, options, Some(&bounds))
}

/// [`crate::validate_shape_map`] over an [`ExactSchema`], as [`validate_exact`].
///
/// # Errors
///
/// As [`crate::validate_shape_map`].
pub fn validate_shape_map_exact(
    schema: &ExactSchema,
    data: &RdfDataset,
    map_src: &str,
    base: Option<&str>,
    options: &ValidationOptions<'_>,
) -> Result<ResultShapeMap> {
    let bounds = schema.bounds_by_constraint();
    crate::shapemap::validate_shape_map_with_bounds(
        &schema.schema,
        data,
        map_src,
        base,
        options,
        Some(&bounds),
    )
}
