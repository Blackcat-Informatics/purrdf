// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! ShExJ — the JSON(-LD) wire format for ShEx schemas (spec Appendix A).
//!
//! Maps the [`Schema`] AST to and from the exact ShExJ object model used by
//! the conformance suite's `schemas/*.json` ground truth:
//!
//! * type-discriminated objects (`"type": "Shape"`, `"ShapeAnd"`, …);
//! * shape/triple-expression *references* as bare strings;
//! * labeled `shapes` entries as shape-expression objects with an inlined
//!   `"id"`;
//! * `"min"`/`"max"` as JSON numbers with `-1` for unbounded;
//! * numeric facets as bare JSON numbers (integral values as integers);
//! * value-set members as bare IRI strings, `ObjectLiteral`s, or the
//!   `IriStem`/`IriStemRange`/`LiteralStem`/…/`Language`/`Wildcard` family.
//!
//! Deserialization is strict: unknown keys, malformed discriminators and
//! type-mismatched values are typed [`ShexError::Shexj`] failures, never
//! silently ignored — this is what lets the conformance harness catch
//! ShEx 2.next constructs in a 2.1 corpus.
//!
//! The document is read by the workspace's one JSON reader
//! ([`purrdf_lex::json`]) with at most 128 open arrays and
//! objects, and no object may repeat a member name. RFC 8259 §4 leaves the
//! meaning of a repeated name open, so a strict reader cannot pick one
//! occurrence without silently discarding the other (RFC 7493 §2.3).
//!
//! # Every IRI-valued member is document-relative, because ShExJ is JSON-LD
//!
//! ShEx 2.1 §2 (Notation) states that "ShExJ is a dialect of JSON-LD and the
//! member id is used as a node identifier"; §4 (Conformance) that "A ShExJ
//! document complies with this specification if it is a valid JSON-LD document,
//! and conforms to the ShExJ syntax, as described in § A"; and §2.2 (References)
//! points at JSON-LD §4.8 "Interpreting JSON as JSON-LD" for how a bare JSON
//! document is read as one. The ShExJ `@context` ([`SHEX_CONTEXT`]) is therefore
//! what says which members hold IRIs, and it types every one of them
//! `"@type": "@id"`: `imports`, `predicate`, `datatype`, `extra`, `name`,
//! `object`, `values`, `exclusions`, `start`, `startActs`, `shapes`,
//! `shapeExpr`/`shapeExprs`, `expression`/`expressions`, `semActs`,
//! `annotations` and `valueExpr`. A JSON-LD `@id` value is an IRI **reference**
//! resolved against the document base, exactly like Turtle's `<foo>`.
//!
//! So every IRI-valued position here goes through [`BaseScope::resolve`] — the
//! same layer, and the same call, the ShExC parser's `iri` production takes. That
//! is what makes the two syntaxes denote the same schema: `{"predicate": "p1"}`
//! and `<p1>` resolve alike under one base, and with no base in scope both are
//! the same hard `iri-relative-no-base` failure rather than a predicate string no
//! data term can ever match. A schema whose constraints cannot match anything is
//! not a stricter schema, it is a vacuous one, so it is refused.
//!
//! Positions that are **not** IRIs keep their bytes: a `_:`-prefixed label is a
//! JSON-LD blank node identifier rather than a reference, and literal values,
//! language tags, `LiteralStem`/`LanguageStem` stems, `pattern`/`flags` and a
//! `SemAct`'s `code` are plain strings the context leaves untyped.
//!
//! # A complete language tag is still held to the grammar
//!
//! "Untyped by the context" is not "unjudged". The three positions that hold a
//! WHOLE language tag — an `ObjectLiteral`'s `"language"`, a `Language` value-set
//! member's `"languageTag"`, and a string exclusion of a `LanguageStemRange` —
//! go through `check_language_tag` on `crate::lexer`'s `LANGTAG_PROFILE`, the
//! very constant the ShExC lexer holds `@tag` to. Without that the crate would
//! refuse its own output, because [`crate::shexc`] writes each of them back out
//! verbatim after an `@`. A *stem* is deliberately exempt: it is a prefix, and
//! the empty stem is the spec's "any language" wildcard.

use purrdf_iri::{BaseIri, BaseOrigin, BaseScope, langtag};
use purrdf_lex::json::{self, Limits, Number, Object, Value};
use purrdf_xsd::ieee::Binary64Scope;

use crate::ast::{
    Annotation, IriExclusion, LanguageExclusion, LiteralExclusion, NodeConstraint, NodeKind,
    NumericLiteral, ObjectLiteral, ObjectValue, Schema, SemAct, Shape, ShapeDecl, ShapeExpr,
    StemValue, TripleConstraint, TripleExpr, TripleExprGroup, ValueSetValue,
};
use crate::error::{Result, ShexError};

/// The canonical `@context` IRI emitted on serialized schemas.
pub const SHEX_CONTEXT: &str = "http://www.w3.org/ns/shex.jsonld";

/// The bounds a ShExJ document is read under: 128 open arrays and objects,
/// and no repeated member name.
const SHEXJ_LIMITS: Limits = Limits {
    unique_members: true,
    ..Limits::with_depth(128)
};

/// Parse a ShExJ document into a [`Schema`], resolving every IRI-valued member
/// against `base`.
///
/// `base` is the document's own base IRI — a caller-supplied base (RFC-3986
/// §5.1.2) or the retrieval IRI of the document the JSON came from (§5.1.3).
/// ShExJ is a JSON-LD dialect, so each `"@type": "@id"` member of the ShExJ
/// context is a document-relative IRI reference (see the module documentation);
/// with `base = None` a relative reference anywhere in the document is a hard
/// [`ShexError::Iri`] carrying the shared `iri-relative-no-base` diagnostic code,
/// never a reference interned verbatim.
///
/// # Examples
///
/// ```
/// use purrdf_shex::parse_shexj;
///
/// let doc = concat!(
///     r#"{"type":"Schema","shapes":[{"type":"Shape","id":"S1","#,
///     r#""expression":{"type":"TripleConstraint","predicate":"p1"}}]}"#,
/// );
///
/// // With a base in scope the relative references denote real IRIs.
/// let schema = parse_shexj(doc, Some("http://example.org/")).expect("resolves");
/// assert_eq!(schema.shapes[0].id, "http://example.org/S1");
///
/// // Without one they are refused, never interned as unmatchable strings.
/// let err = parse_shexj(doc, None).unwrap_err();
/// assert!(err.to_string().contains("iri-relative-no-base"));
/// ```
pub fn parse_shexj(input: &str, base: Option<&str>) -> Result<Schema> {
    parse_shexj_exact(input, base, false).map(|(schema, _)| schema)
}

/// [`parse_shexj`], with each numeric facet bound's lexeme recorded beside the AST
/// (see [`crate::ExactSchema`]). With `saturate`, a facet number without an exponent
/// past the `f64` range is accepted, its AST value the saturated stand-in
/// [`crate::exact::saturated`]; without it, the bound is refused, as the AST cannot
/// hold it.
pub(crate) fn parse_shexj_exact(
    input: &str,
    base: Option<&str>,
    saturate: bool,
) -> Result<(Schema, crate::exact::ExactTable)> {
    let value = json::read_with(input, SHEXJ_LIMITS)
        .map_err(|e| ShexError::shexj(format!("invalid JSON: {e}")))?;
    // A numeric facet's value is compared with every datum validated against it: its
    // lexeme becomes a binary64 inside a binary64 scope so the x87 rounds it once, like
    // every other unit.
    let _binary64 = Binary64Scope::enter();
    let mut reader = Reader::new(base)?;
    reader.saturate = saturate;
    let schema = reader.schema(&value)?;
    Ok((schema, reader.table.into_inner()))
}

/// Serialize a [`Schema`] to pretty-printed ShExJ.
///
/// # Examples
///
/// The two syntaxes describe the same [`Schema`], so a ShExC-parsed schema
/// round-trips through the JSON wire format:
///
/// ```
/// use purrdf_shex::{parse_shexc, parse_shexj, to_shexj};
///
/// let schema = parse_shexc(
///     "<http://example.org/UserShape> { <http://example.org/name> LITERAL }",
///     None,
/// )
/// .expect("a well-formed schema parses");
///
/// let json = to_shexj(&schema);
/// // The emitted document holds only absolute IRIs, so it needs no base.
/// assert_eq!(
///     parse_shexj(&json, None).expect("emitted ShExJ parses back"),
///     schema
/// );
/// ```
#[must_use]
pub fn to_shexj(schema: &Schema) -> String {
    to_shexj_with(schema, None)
}

/// The JSON spelling of a node constraint's numeric facets that replaces the AST's
/// `i64`/`f64`, by node-constraint address, in the order `mininclusive`,
/// `minexclusive`, `maxinclusive`, `maxexclusive` (see [`crate::ExactSchema`]).
pub(crate) type FacetSpellings = purrdf_core::FastMap<usize, [Option<Number>; 4]>;

/// [`to_shexj`], writing each numeric facet with a spelling in `spellings` as that
/// spelling, and every other byte as [`to_shexj`] does.
pub(crate) fn to_shexj_with(schema: &Schema, spellings: Option<&FacetSpellings>) -> String {
    let mut value = schema_to_value(schema, spellings);
    // Members are written in name order: the byte layout every ShExJ golden pins.
    value.sort_keys();
    json::write_pretty(&value)
}

// ── serialization (AST → Value) ─────────────────────────────────────────────

fn schema_to_value(schema: &Schema, sp: Option<&FacetSpellings>) -> Value {
    let mut obj = Object::new();
    obj.push("@context", SHEX_CONTEXT);
    obj.push("type", "Schema");
    if !schema.imports.is_empty() {
        obj.push("imports", schema.imports.as_slice());
    }
    if !schema.start_acts.is_empty() {
        let acts: Vec<Value> = schema.start_acts.iter().map(sem_act_to_value).collect();
        obj.push("startActs", Value::Array(acts));
    }
    if let Some(start) = &schema.start {
        obj.push("start", shape_expr_to_value(start, sp));
    }
    if !schema.shapes.is_empty() {
        let shapes: Vec<Value> = schema
            .shapes
            .iter()
            .map(|decl| shape_decl_to_value(decl, sp))
            .collect();
        obj.push("shapes", Value::Array(shapes));
    }
    Value::Object(obj)
}

fn shape_decl_to_value(decl: &ShapeDecl, sp: Option<&FacetSpellings>) -> Value {
    // ShExJ 2.1 inlines the declaration id on the shape-expression object. A
    // bare reference cannot carry an id, so it is wrapped in a singleton
    // `ShapeAnd` (the only faithful 2.1 encoding).
    let mut body = match &decl.expr {
        ShapeExpr::Ref(_) => shape_expr_to_value(&ShapeExpr::And(vec![decl.expr.clone()]), sp),
        other => shape_expr_to_value(other, sp),
    };
    if let Some(obj) = body.as_object_mut() {
        obj.push("id", &decl.id);
    }
    body
}

fn shape_expr_to_value(expr: &ShapeExpr, sp: Option<&FacetSpellings>) -> Value {
    match expr {
        ShapeExpr::Ref(label) => Value::from(label),
        ShapeExpr::And(parts) => {
            let parts: Vec<Value> = parts.iter().map(|p| shape_expr_to_value(p, sp)).collect();
            Value::object([
                ("type", Value::from("ShapeAnd")),
                ("shapeExprs", Value::from(parts)),
            ])
        }
        ShapeExpr::Or(parts) => {
            let parts: Vec<Value> = parts.iter().map(|p| shape_expr_to_value(p, sp)).collect();
            Value::object([
                ("type", Value::from("ShapeOr")),
                ("shapeExprs", Value::from(parts)),
            ])
        }
        ShapeExpr::Not(inner) => Value::object([
            ("type", Value::from("ShapeNot")),
            ("shapeExpr", shape_expr_to_value(inner, sp)),
        ]),
        ShapeExpr::External => Value::object([("type", Value::from("ShapeExternal"))]),
        ShapeExpr::Node(nc) => node_constraint_to_value(nc, sp),
        ShapeExpr::Shape(shape) => shape_to_value(shape, sp),
    }
}

fn node_constraint_to_value(nc: &NodeConstraint, sp: Option<&FacetSpellings>) -> Value {
    let spelled = sp.and_then(|sp| sp.get(&(std::ptr::from_ref(nc) as usize)));
    let mut obj = Object::new();
    obj.push("type", "NodeConstraint");
    if let Some(kind) = nc.node_kind {
        obj.push("nodeKind", kind.as_str());
    }
    if let Some(dt) = &nc.datatype {
        obj.push("datatype", dt);
    }
    for (key, slot) in [
        ("length", nc.length),
        ("minlength", nc.minlength),
        ("maxlength", nc.maxlength),
        ("totaldigits", nc.totaldigits),
        ("fractiondigits", nc.fractiondigits),
    ] {
        if let Some(n) = slot {
            obj.push(key, Value::from(n));
        }
    }
    if let Some(p) = &nc.pattern {
        obj.push("pattern", p);
    }
    if let Some(f) = &nc.flags {
        obj.push("flags", f);
    }
    for (index, (key, slot)) in [
        ("mininclusive", nc.mininclusive),
        ("minexclusive", nc.minexclusive),
        ("maxinclusive", nc.maxinclusive),
        ("maxexclusive", nc.maxexclusive),
    ]
    .into_iter()
    .enumerate()
    {
        if let Some(n) = slot {
            match spelled.and_then(|spelled| spelled[index].clone()) {
                Some(number) => obj.push(key, Value::Number(number)),
                None => obj.push(key, numeric_to_value(n)),
            }
        }
    }
    if let Some(values) = &nc.values {
        let values: Vec<Value> = values.iter().map(value_set_value_to_value).collect();
        obj.push("values", Value::Array(values));
    }
    Value::Object(obj)
}

/// The ShExJ number [`to_shexj`] writes for a facet value.
pub(crate) fn numeric_to_value(n: NumericLiteral) -> Value {
    match n {
        NumericLiteral::Integer(i) => Value::from(i),
        NumericLiteral::Fractional(f) => {
            Number::from_f64(f).map_or_else(|| Value::from(0_u8), Value::Number)
        }
    }
}

fn stem_to_value(stem: &StemValue) -> Value {
    match stem {
        StemValue::Str(s) => Value::from(s),
        StemValue::Wildcard => Value::object([("type", Value::from("Wildcard"))]),
    }
}

fn value_set_value_to_value(v: &ValueSetValue) -> Value {
    match v {
        ValueSetValue::Iri(iri) => Value::from(iri),
        ValueSetValue::Literal(lit) => object_literal_to_value(lit),
        ValueSetValue::IriStem { stem } => Value::object([
            ("type", Value::from("IriStem")),
            ("stem", Value::from(stem)),
        ]),
        ValueSetValue::IriStemRange { stem, exclusions } => {
            let exclusions: Vec<Value> = exclusions
                .iter()
                .map(|e| match e {
                    IriExclusion::Iri(iri) => Value::from(iri),
                    IriExclusion::Stem(stem) => Value::object([
                        ("type", Value::from("IriStem")),
                        ("stem", Value::from(stem)),
                    ]),
                })
                .collect();
            Value::object([
                ("type", Value::from("IriStemRange")),
                ("stem", stem_to_value(stem)),
                ("exclusions", Value::from(exclusions)),
            ])
        }
        ValueSetValue::LiteralStem { stem } => Value::object([
            ("type", Value::from("LiteralStem")),
            ("stem", Value::from(stem)),
        ]),
        ValueSetValue::LiteralStemRange { stem, exclusions } => {
            let exclusions: Vec<Value> = exclusions
                .iter()
                .map(|e| match e {
                    LiteralExclusion::Literal(v) => Value::from(v),
                    LiteralExclusion::Stem(stem) => Value::object([
                        ("type", Value::from("LiteralStem")),
                        ("stem", Value::from(stem)),
                    ]),
                })
                .collect();
            Value::object([
                ("type", Value::from("LiteralStemRange")),
                ("stem", stem_to_value(stem)),
                ("exclusions", Value::from(exclusions)),
            ])
        }
        ValueSetValue::Language { language_tag } => Value::object([
            ("type", Value::from("Language")),
            ("languageTag", Value::from(language_tag)),
        ]),
        ValueSetValue::LanguageStem { stem } => Value::object([
            ("type", Value::from("LanguageStem")),
            ("stem", Value::from(stem)),
        ]),
        ValueSetValue::LanguageStemRange { stem, exclusions } => {
            let exclusions: Vec<Value> = exclusions
                .iter()
                .map(|e| match e {
                    LanguageExclusion::Language(tag) => Value::from(tag),
                    LanguageExclusion::Stem(stem) => Value::object([
                        ("type", Value::from("LanguageStem")),
                        ("stem", Value::from(stem)),
                    ]),
                })
                .collect();
            Value::object([
                ("type", Value::from("LanguageStemRange")),
                ("stem", stem_to_value(stem)),
                ("exclusions", Value::from(exclusions)),
            ])
        }
    }
}

fn object_literal_to_value(lit: &ObjectLiteral) -> Value {
    let mut obj = Object::new();
    obj.push("value", &lit.value);
    if let Some(lang) = &lit.language {
        obj.push("language", lang);
    }
    if let Some(dt) = &lit.datatype {
        obj.push("type", dt);
    }
    Value::Object(obj)
}

fn shape_to_value(shape: &Shape, sp: Option<&FacetSpellings>) -> Value {
    let mut obj = Object::new();
    obj.push("type", "Shape");
    if let Some(closed) = shape.closed {
        obj.push("closed", closed);
    }
    if !shape.extra.is_empty() {
        obj.push("extra", shape.extra.as_slice());
    }
    if let Some(expr) = &shape.expression {
        obj.push("expression", triple_expr_to_value(expr, sp));
    }
    insert_acts_annots(&mut obj, &shape.sem_acts, &shape.annotations);
    Value::Object(obj)
}

fn insert_acts_annots(obj: &mut Object, sem_acts: &[SemAct], annots: &[Annotation]) {
    if !sem_acts.is_empty() {
        let acts: Vec<Value> = sem_acts.iter().map(sem_act_to_value).collect();
        obj.push("semActs", Value::Array(acts));
    }
    if !annots.is_empty() {
        let annots: Vec<Value> = annots.iter().map(annotation_to_value).collect();
        obj.push("annotations", Value::Array(annots));
    }
}

fn triple_expr_to_value(expr: &TripleExpr, sp: Option<&FacetSpellings>) -> Value {
    match expr {
        TripleExpr::Ref(label) => Value::from(label),
        TripleExpr::EachOf(group) => group_to_value("EachOf", group, sp),
        TripleExpr::OneOf(group) => group_to_value("OneOf", group, sp),
        TripleExpr::TripleConstraint(tc) => {
            let mut obj = Object::new();
            obj.push("type", "TripleConstraint");
            if let Some(id) = &tc.id {
                obj.push("id", id);
            }
            if let Some(inverse) = tc.inverse {
                obj.push("inverse", inverse);
            }
            obj.push("predicate", &tc.predicate);
            if let Some(ve) = &tc.value_expr {
                obj.push("valueExpr", shape_expr_to_value(ve, sp));
            }
            insert_min_max(&mut obj, tc.min, tc.max);
            insert_acts_annots(&mut obj, &tc.sem_acts, &tc.annotations);
            Value::Object(obj)
        }
    }
}

fn group_to_value(kind: &str, group: &TripleExprGroup, sp: Option<&FacetSpellings>) -> Value {
    let mut obj = Object::new();
    obj.push("type", kind);
    if let Some(id) = &group.id {
        obj.push("id", id);
    }
    let members: Vec<Value> = group
        .expressions
        .iter()
        .map(|member| triple_expr_to_value(member, sp))
        .collect();
    obj.push("expressions", Value::Array(members));
    insert_min_max(&mut obj, group.min, group.max);
    insert_acts_annots(&mut obj, &group.sem_acts, &group.annotations);
    Value::Object(obj)
}

fn insert_min_max(obj: &mut Object, min: Option<i64>, max: Option<i64>) {
    if let Some(min) = min {
        obj.push("min", min);
    }
    if let Some(max) = max {
        obj.push("max", max);
    }
}

fn sem_act_to_value(act: &SemAct) -> Value {
    let mut obj = Object::new();
    obj.push("type", "SemAct");
    obj.push("name", &act.name);
    if let Some(code) = &act.code {
        obj.push("code", code);
    }
    Value::Object(obj)
}

fn annotation_to_value(annotation: &Annotation) -> Value {
    let object = match &annotation.object {
        ObjectValue::Iri(iri) => Value::from(iri),
        ObjectValue::Literal(lit) => object_literal_to_value(lit),
    };
    Value::object([
        ("type", Value::from("Annotation")),
        ("predicate", Value::from(&annotation.predicate)),
        ("object", object),
    ])
}

// ── deserialization (Value → AST) ────────────────────────────────────────────

/// A strict object reader: every key must be consumed, or the read fails.
struct Obj<'a> {
    map: &'a Object,
    taken: Vec<&'a str>,
    what: &'static str,
}

impl<'a> Obj<'a> {
    fn new(value: &'a Value, what: &'static str) -> Result<Self> {
        let map = value
            .as_object()
            .ok_or_else(|| ShexError::shexj(format!("{what}: expected a JSON object")))?;
        Ok(Self {
            map,
            taken: Vec::new(),
            what,
        })
    }

    fn typed(value: &'a Value, expected: &str, what: &'static str) -> Result<Self> {
        let mut obj = Self::new(value, what)?;
        let ty = obj.take_str("type")?;
        if ty != expected {
            return Err(ShexError::shexj(format!(
                "{what}: expected type {expected:?}, found {ty:?}"
            )));
        }
        Ok(obj)
    }

    fn take(&mut self, key: &'static str) -> Option<&'a Value> {
        let v = self.map.get(key)?;
        self.taken.push(key);
        Some(v)
    }

    fn take_str(&mut self, key: &'static str) -> Result<String> {
        self.take_str_opt(key)?
            .ok_or_else(|| ShexError::shexj(format!("{}: missing {key:?}", self.what)))
    }

    fn take_str_opt(&mut self, key: &'static str) -> Result<Option<String>> {
        match self.take(key) {
            None => Ok(None),
            Some(Value::String(s)) => Ok(Some(s.clone())),
            Some(_) => Err(ShexError::shexj(format!(
                "{}: {key:?} must be a string",
                self.what
            ))),
        }
    }

    fn take_u64_opt(&mut self, key: &'static str) -> Result<Option<u64>> {
        match self.take(key) {
            None => Ok(None),
            Some(Value::Number(n)) => n.as_u64().map(Some).ok_or_else(|| {
                ShexError::shexj(format!(
                    "{}: {key:?} must be a non-negative integer",
                    self.what
                ))
            }),
            Some(_) => Err(ShexError::shexj(format!(
                "{}: {key:?} must be a number",
                self.what
            ))),
        }
    }

    fn take_i64_opt(&mut self, key: &'static str) -> Result<Option<i64>> {
        match self.take(key) {
            None => Ok(None),
            Some(Value::Number(n)) => n.as_i64().map(Some).ok_or_else(|| {
                ShexError::shexj(format!("{}: {key:?} must be an integer", self.what))
            }),
            Some(_) => Err(ShexError::shexj(format!(
                "{}: {key:?} must be a number",
                self.what
            ))),
        }
    }

    fn take_bool_opt(&mut self, key: &'static str) -> Result<Option<bool>> {
        match self.take(key) {
            None => Ok(None),
            Some(Value::Bool(b)) => Ok(Some(*b)),
            Some(_) => Err(ShexError::shexj(format!(
                "{}: {key:?} must be a boolean",
                self.what
            ))),
        }
    }

    fn take_numeric_opt(&mut self, key: &'static str) -> Result<Option<NumericLiteral>> {
        match self.take(key) {
            None => Ok(None),
            Some(Value::Number(n)) => {
                if let Some(i) = n.as_i64() {
                    Ok(Some(NumericLiteral::Integer(i)))
                } else if let Some(f) = Some(n.as_f64()).filter(|f| f.is_finite()) {
                    Ok(Some(NumericLiteral::Fractional(f)))
                } else {
                    Err(ShexError::shexj(format!(
                        "{}: {key:?} out of range",
                        self.what
                    )))
                }
            }
            Some(_) => Err(ShexError::shexj(format!(
                "{}: {key:?} must be a number",
                self.what
            ))),
        }
    }

    fn take_array(&mut self, key: &'static str) -> Result<Option<&'a [Value]>> {
        match self.take(key) {
            None => Ok(None),
            Some(Value::Array(items)) => Ok(Some(items)),
            Some(_) => Err(ShexError::shexj(format!(
                "{}: {key:?} must be an array",
                self.what
            ))),
        }
    }

    /// Fail on any key that was never consumed (strict ShExJ 2.1 reading).
    fn finish(self) -> Result<()> {
        for key in self.map.keys() {
            if !self.taken.contains(&key.as_str()) {
                return Err(ShexError::shexj(format!(
                    "{}: unknown key {key:?}",
                    self.what
                )));
            }
        }
        Ok(())
    }
}

/// A ShExJ document reader bound to that document's base.
///
/// Every IRI-valued member goes through [`Reader::iri`] and every node
/// identifier through [`Reader::label`], so there is exactly one place where a
/// ShExJ string becomes an IRI and exactly one base it can resolve against.
struct Reader {
    /// The base the document's IRI references resolve against — empty when the
    /// caller supplied none, which makes a relative reference a hard failure.
    base: BaseScope,
    /// The exact bounds of each numeric node constraint read so far in the current
    /// owner, in creation order (see [`crate::ExactSchema`]).
    exact: std::cell::RefCell<Vec<crate::exact::ExactBounds>>,
    /// The exact bounds per owner, filled as each owner completes.
    table: std::cell::RefCell<crate::exact::ExactTable>,
    /// Whether a facet number without an exponent past the `f64` range is accepted
    /// with a saturated AST value (an [`crate::ExactSchema`] keeps its digits).
    saturate: bool,
}

impl Reader {
    /// Build a reader rooted at `base` (RFC-3986 §5.1.2/§5.1.3), or with an empty
    /// scope when the document has no base at all (§5.1.4).
    fn new(base: Option<&str>) -> Result<Self> {
        let base = match base {
            Some(iri) => BaseScope::rooted(
                BaseIri::parse(iri).map_err(|e| ShexError::iri(iri, &e))?,
                BaseOrigin::Caller,
            ),
            None => BaseScope::empty(),
        };
        Ok(Self {
            base,
            exact: std::cell::RefCell::default(),
            table: std::cell::RefCell::default(),
            saturate: false,
        })
    }

    /// Resolve one document-relative IRI reference (a `"@type": "@id"` member).
    ///
    /// There is deliberately no "keep the reference verbatim when no base is in
    /// scope" fallthrough, for the reason the ShExC parser has none: an
    /// unresolved reference in a schema denotes nothing an absolute data term can
    /// match, so it turns every constraint written with it into a vacuous one.
    fn iri(&self, reference: &str) -> Result<String> {
        crate::parser::resolve_iri(&self.base, reference)
    }

    /// Read a `shapeExprLabel` / `tripleExprLabel`.
    ///
    /// ShEx 2.1 §2 says the `id` member is a JSON-LD **node identifier**, which is
    /// either a blank node identifier or an IRI. A `_:`-prefixed label is the
    /// former and is not a reference at all, so it keeps its bytes exactly as the
    /// ShExC parser's `parse_label` keeps a `BNODE` token's.
    fn label(&self, reference: &str) -> Result<String> {
        if reference.starts_with("_:") {
            return Ok(reference.to_owned());
        }
        self.iri(reference)
    }

    fn schema(&self, value: &Value) -> Result<Schema> {
        let mut obj = Obj::typed(value, "Schema", "Schema")?;
        let _context = obj.take("@context");
        let mut schema = Schema::default();
        if let Some(items) = obj.take_array("imports")? {
            schema.imports.reserve(items.len());
            for item in items {
                let Value::String(iri) = item else {
                    return Err(ShexError::shexj("Schema: imports entries must be strings"));
                };
                schema.imports.push(self.iri(iri)?);
            }
        }
        if let Some(items) = obj.take_array("startActs")? {
            schema.start_acts.reserve(items.len());
            for item in items {
                schema.start_acts.push(self.sem_act(item)?);
            }
        }
        if let Some(start) = obj.take("start") {
            let mark = self.exact.borrow().len();
            schema.start = Some(Box::new(self.shape_expr(start)?));
            self.table.borrow_mut().start = self.exact.borrow_mut().split_off(mark);
        }
        if let Some(items) = obj.take_array("shapes")? {
            schema.shapes.reserve(items.len());
            for item in items {
                let mark = self.exact.borrow().len();
                schema.shapes.push(self.shape_decl(item)?);
                let groups = self.exact.borrow_mut().split_off(mark);
                self.table.borrow_mut().shapes.push(groups);
            }
        }
        obj.finish()?;
        Ok(schema)
    }

    fn shape_decl(&self, value: &Value) -> Result<ShapeDecl> {
        // A `shapes` entry is a shape-expression object with an inlined `id`;
        // strip the id and re-read the remainder as a plain shape expression.
        let map = value
            .as_object()
            .ok_or_else(|| ShexError::shexj("shapes entry: expected a JSON object"))?;
        let Some(Value::String(id)) = map.get("id") else {
            return Err(ShexError::shexj("shapes entry: missing string \"id\""));
        };
        let id = self.label(id)?;
        let mut rest = map.clone();
        rest.remove("id");
        let expr = self.shape_expr(&Value::Object(rest))?;
        Ok(ShapeDecl { id, expr })
    }

    fn shape_expr(&self, value: &Value) -> Result<ShapeExpr> {
        if let Value::String(label) = value {
            return Ok(ShapeExpr::Ref(self.label(label)?));
        }
        let ty = value
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| ShexError::shexj("shapeExpr: missing \"type\" discriminator"))?;
        match ty {
            "ShapeAnd" | "ShapeOr" => {
                let mut obj = Obj::typed(value, ty, "ShapeAnd/ShapeOr")?;
                let items = obj
                    .take_array("shapeExprs")?
                    .ok_or_else(|| ShexError::shexj(format!("{ty}: missing \"shapeExprs\"")))?;
                let parts = items
                    .iter()
                    .map(|item| self.shape_expr(item))
                    .collect::<Result<Vec<_>>>()?;
                obj.finish()?;
                if ty == "ShapeAnd" {
                    Ok(ShapeExpr::And(parts))
                } else {
                    Ok(ShapeExpr::Or(parts))
                }
            }
            "ShapeNot" => {
                let mut obj = Obj::typed(value, "ShapeNot", "ShapeNot")?;
                let inner = obj
                    .take("shapeExpr")
                    .ok_or_else(|| ShexError::shexj("ShapeNot: missing \"shapeExpr\""))?;
                let inner = self.shape_expr(inner)?;
                obj.finish()?;
                Ok(ShapeExpr::Not(Box::new(inner)))
            }
            "ShapeExternal" => {
                let obj = Obj::typed(value, "ShapeExternal", "ShapeExternal")?;
                obj.finish()?;
                Ok(ShapeExpr::External)
            }
            "NodeConstraint" => Ok(ShapeExpr::Node(self.node_constraint(value)?)),
            "Shape" => Ok(ShapeExpr::Shape(self.shape(value)?)),
            other => Err(ShexError::shexj(format!(
                "shapeExpr: unknown type {other:?}"
            ))),
        }
    }

    /// A numeric facet's bound, recording the JSON number's lexeme in `exact`: an
    /// exponent makes it a double, a point a decimal, neither an integer (see
    /// [`crate::ExactSchema`]).
    fn numeric_facet(
        obj: &mut Obj<'_>,
        key: &'static str,
        exact: &mut Option<crate::exact::ExactBound>,
        saturate: bool,
    ) -> Result<Option<NumericLiteral>> {
        let lexeme = match obj.map.get(key) {
            Some(Value::Number(number)) => Some(number.lexeme().to_owned()),
            _ => None,
        };
        let past_f64 = lexeme.as_deref().is_some_and(|lexeme| {
            !lexeme.contains(['e', 'E'])
                && Number::from_lexeme(lexeme).is_ok_and(|n| !n.as_f64().is_finite())
        });
        let bound = match &lexeme {
            Some(lexeme) if saturate && past_f64 => {
                obj.take(key);
                Some(crate::exact::saturated(lexeme))
            }
            _ => obj.take_numeric_opt(key)?,
        };
        if let (Some(_), Some(lexeme)) = (bound, lexeme) {
            *exact = Some(crate::exact::ExactBound::from_lexical(&lexeme));
        }
        Ok(bound)
    }

    fn node_constraint(&self, value: &Value) -> Result<NodeConstraint> {
        let mut exact = crate::exact::ExactBounds::default();
        let [min_in, min_ex, max_in, max_ex] = &mut exact.slots;
        let mut obj = Obj::typed(value, "NodeConstraint", "NodeConstraint")?;
        let mut nc = NodeConstraint {
            node_kind: obj
                .take_str_opt("nodeKind")?
                .map(|s| node_kind_from_str(&s))
                .transpose()?,
            datatype: obj
                .take_str_opt("datatype")?
                .map(|dt| self.iri(&dt))
                .transpose()?,
            length: obj.take_u64_opt("length")?,
            minlength: obj.take_u64_opt("minlength")?,
            maxlength: obj.take_u64_opt("maxlength")?,
            pattern: obj.take_str_opt("pattern")?,
            flags: obj.take_str_opt("flags")?,
            mininclusive: Self::numeric_facet(&mut obj, "mininclusive", min_in, self.saturate)?,
            minexclusive: Self::numeric_facet(&mut obj, "minexclusive", min_ex, self.saturate)?,
            maxinclusive: Self::numeric_facet(&mut obj, "maxinclusive", max_in, self.saturate)?,
            maxexclusive: Self::numeric_facet(&mut obj, "maxexclusive", max_ex, self.saturate)?,
            totaldigits: obj.take_u64_opt("totaldigits")?,
            fractiondigits: obj.take_u64_opt("fractiondigits")?,
            values: None,
        };
        if let Some(items) = obj.take_array("values")? {
            let values = items
                .iter()
                .map(|item| self.value_set_value(item))
                .collect::<Result<Vec<_>>>()?;
            nc.values = Some(values);
        }
        obj.finish()?;
        if !exact.is_empty() {
            self.exact.borrow_mut().push(exact);
        }
        Ok(nc)
    }

    /// An `IriStemRange` stem: an IRI reference, or the `Wildcard` object.
    fn iri_stem(&self, value: &Value, what: &'static str) -> Result<StemValue> {
        match value {
            Value::String(s) => Ok(StemValue::Str(self.iri(s)?)),
            Value::Object(_) => wildcard_stem(value, what),
            _ => Err(ShexError::shexj(format!(
                "{what}: stem must be a string or Wildcard"
            ))),
        }
    }

    fn value_set_value(&self, value: &Value) -> Result<ValueSetValue> {
        if let Value::String(iri) = value {
            return Ok(ValueSetValue::Iri(self.iri(iri)?));
        }
        // NB: an ObjectLiteral's "type" key is its datatype IRI, so only the known
        // stem/range discriminators select a non-literal variant.
        let ty = value.get("type").and_then(Value::as_str);
        match ty {
            None => Ok(ValueSetValue::Literal(self.object_literal(value)?)),
            Some("IriStem") => {
                let mut obj = Obj::typed(value, "IriStem", "IriStem")?;
                let stem = self.iri(&obj.take_str("stem")?)?;
                obj.finish()?;
                Ok(ValueSetValue::IriStem { stem })
            }
            Some("LiteralStem") => {
                let mut obj = Obj::typed(value, "LiteralStem", "LiteralStem")?;
                let stem = obj.take_str("stem")?;
                obj.finish()?;
                Ok(ValueSetValue::LiteralStem { stem })
            }
            Some("LanguageStem") => {
                let mut obj = Obj::typed(value, "LanguageStem", "LanguageStem")?;
                let stem = obj.take_str("stem")?;
                check_language_stem(&stem, "LanguageStem: \"stem\"")?;
                obj.finish()?;
                Ok(ValueSetValue::LanguageStem { stem })
            }
            Some("Language") => {
                let mut obj = Obj::typed(value, "Language", "Language")?;
                let language_tag = obj.take_str("languageTag")?;
                check_language_tag(&language_tag, "Language: \"languageTag\"")?;
                obj.finish()?;
                Ok(ValueSetValue::Language { language_tag })
            }
            Some("IriStemRange") => self.iri_stem_range(value),
            Some("LiteralStemRange") => self.literal_stem_range(value),
            Some("LanguageStemRange") => self.language_stem_range(value),
            Some(_) => Ok(ValueSetValue::Literal(self.object_literal(value)?)),
        }
    }

    fn iri_stem_range(&self, value: &Value) -> Result<ValueSetValue> {
        let mut obj = Obj::typed(value, "IriStemRange", "IriStemRange")?;
        let stem = obj
            .take("stem")
            .ok_or_else(|| ShexError::shexj("IriStemRange: missing \"stem\""))
            .and_then(|v| self.iri_stem(v, "IriStemRange"))?;
        let exclusions = obj
            .take_array("exclusions")?
            .unwrap_or_default()
            .iter()
            .map(|e| match e {
                Value::String(iri) => Ok(IriExclusion::Iri(self.iri(iri)?)),
                other => {
                    let mut obj = Obj::typed(other, "IriStem", "IriStemRange exclusion")?;
                    let stem = self.iri(&obj.take_str("stem")?)?;
                    obj.finish()?;
                    Ok(IriExclusion::Stem(stem))
                }
            })
            .collect::<Result<Vec<_>>>()?;
        obj.finish()?;
        Ok(ValueSetValue::IriStemRange { stem, exclusions })
    }

    fn literal_stem_range(&self, value: &Value) -> Result<ValueSetValue> {
        let mut obj = Obj::typed(value, "LiteralStemRange", "LiteralStemRange")?;
        let stem = obj
            .take("stem")
            .ok_or_else(|| ShexError::shexj("LiteralStemRange: missing \"stem\""))
            .and_then(|v| plain_stem(v, "LiteralStemRange"))?;
        let exclusions = obj
            .take_array("exclusions")?
            .unwrap_or_default()
            .iter()
            .map(|e| match e {
                Value::String(v) => Ok(LiteralExclusion::Literal(v.clone())),
                other => {
                    let mut obj = Obj::typed(other, "LiteralStem", "LiteralStemRange exclusion")?;
                    let stem = obj.take_str("stem")?;
                    obj.finish()?;
                    Ok(LiteralExclusion::Stem(stem))
                }
            })
            .collect::<Result<Vec<_>>>()?;
        obj.finish()?;
        Ok(ValueSetValue::LiteralStemRange { stem, exclusions })
    }

    fn language_stem_range(&self, value: &Value) -> Result<ValueSetValue> {
        let mut obj = Obj::typed(value, "LanguageStemRange", "LanguageStemRange")?;
        let stem = obj
            .take("stem")
            .ok_or_else(|| ShexError::shexj("LanguageStemRange: missing \"stem\""))
            .and_then(|v| plain_stem(v, "LanguageStemRange"))?;
        if let StemValue::Str(stem) = &stem {
            check_language_stem(stem, "LanguageStemRange: \"stem\"")?;
        }
        let exclusions = obj
            .take_array("exclusions")?
            .unwrap_or_default()
            .iter()
            .map(|e| match e {
                Value::String(tag) => {
                    check_language_tag(tag, "LanguageStemRange exclusion")?;
                    Ok(LanguageExclusion::Language(tag.clone()))
                }
                other => {
                    let mut obj = Obj::typed(other, "LanguageStem", "LanguageStemRange exclusion")?;
                    let stem = obj.take_str("stem")?;
                    check_language_stem(&stem, "LanguageStemRange exclusion: \"stem\"")?;
                    obj.finish()?;
                    Ok(LanguageExclusion::Stem(stem))
                }
            })
            .collect::<Result<Vec<_>>>()?;
        obj.finish()?;
        Ok(ValueSetValue::LanguageStemRange { stem, exclusions })
    }

    fn object_literal(&self, value: &Value) -> Result<ObjectLiteral> {
        let mut obj = Obj::new(value, "ObjectLiteral")?;
        let lit = ObjectLiteral {
            value: obj.take_str("value")?,
            language: obj
                .take_str_opt("language")?
                .map(|tag| check_language_tag(&tag, "ObjectLiteral: \"language\"").map(|()| tag))
                .transpose()?,
            datatype: obj
                .take_str_opt("type")?
                .map(|dt| self.iri(&dt))
                .transpose()?,
        };
        obj.finish()?;
        Ok(lit)
    }

    fn shape(&self, value: &Value) -> Result<Shape> {
        let mut obj = Obj::typed(value, "Shape", "Shape")?;
        let mut shape = Shape {
            closed: obj.take_bool_opt("closed")?,
            ..Shape::default()
        };
        if let Some(items) = obj.take_array("extra")? {
            shape.extra.reserve(items.len());
            for item in items {
                let Value::String(iri) = item else {
                    return Err(ShexError::shexj("Shape: extra entries must be strings"));
                };
                shape.extra.push(self.iri(iri)?);
            }
        }
        if let Some(expr) = obj.take("expression") {
            shape.expression = Some(self.triple_expr(expr)?);
        }
        shape.sem_acts = self.take_sem_acts(&mut obj)?;
        shape.annotations = self.take_annotations(&mut obj)?;
        obj.finish()?;
        Ok(shape)
    }

    fn take_sem_acts(&self, obj: &mut Obj<'_>) -> Result<Vec<SemAct>> {
        obj.take_array("semActs")?
            .unwrap_or_default()
            .iter()
            .map(|item| self.sem_act(item))
            .collect()
    }

    fn take_annotations(&self, obj: &mut Obj<'_>) -> Result<Vec<Annotation>> {
        obj.take_array("annotations")?
            .unwrap_or_default()
            .iter()
            .map(|item| self.annotation(item))
            .collect()
    }

    fn triple_expr(&self, value: &Value) -> Result<TripleExpr> {
        if let Value::String(label) = value {
            return Ok(TripleExpr::Ref(self.label(label)?));
        }
        let ty = value
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| ShexError::shexj("tripleExpr: missing \"type\" discriminator"))?;
        match ty {
            "EachOf" | "OneOf" => {
                let mut obj = Obj::typed(value, ty, "EachOf/OneOf")?;
                let mut group = TripleExprGroup {
                    id: obj
                        .take_str_opt("id")?
                        .map(|id| self.label(&id))
                        .transpose()?,
                    min: obj.take_i64_opt("min")?,
                    max: obj.take_i64_opt("max")?,
                    ..TripleExprGroup::default()
                };
                let items = obj
                    .take_array("expressions")?
                    .ok_or_else(|| ShexError::shexj(format!("{ty}: missing \"expressions\"")))?;
                group.expressions = items
                    .iter()
                    .map(|item| self.triple_expr(item))
                    .collect::<Result<Vec<_>>>()?;
                group.sem_acts = self.take_sem_acts(&mut obj)?;
                group.annotations = self.take_annotations(&mut obj)?;
                obj.finish()?;
                if ty == "EachOf" {
                    Ok(TripleExpr::EachOf(group))
                } else {
                    Ok(TripleExpr::OneOf(group))
                }
            }
            "TripleConstraint" => {
                let mut obj = Obj::typed(value, "TripleConstraint", "TripleConstraint")?;
                let mut tc = TripleConstraint {
                    id: obj
                        .take_str_opt("id")?
                        .map(|id| self.label(&id))
                        .transpose()?,
                    inverse: obj.take_bool_opt("inverse")?,
                    predicate: self.iri(&obj.take_str("predicate")?)?,
                    min: obj.take_i64_opt("min")?,
                    max: obj.take_i64_opt("max")?,
                    ..TripleConstraint::default()
                };
                if let Some(ve) = obj.take("valueExpr") {
                    tc.value_expr = Some(Box::new(self.shape_expr(ve)?));
                }
                tc.sem_acts = self.take_sem_acts(&mut obj)?;
                tc.annotations = self.take_annotations(&mut obj)?;
                obj.finish()?;
                Ok(TripleExpr::TripleConstraint(tc))
            }
            other => Err(ShexError::shexj(format!(
                "tripleExpr: unknown type {other:?}"
            ))),
        }
    }

    fn sem_act(&self, value: &Value) -> Result<SemAct> {
        let mut obj = Obj::typed(value, "SemAct", "SemAct")?;
        let act = SemAct {
            name: self.iri(&obj.take_str("name")?)?,
            code: obj.take_str_opt("code")?,
        };
        obj.finish()?;
        Ok(act)
    }

    fn annotation(&self, value: &Value) -> Result<Annotation> {
        let mut obj = Obj::typed(value, "Annotation", "Annotation")?;
        let predicate = self.iri(&obj.take_str("predicate")?)?;
        let object = obj
            .take("object")
            .ok_or_else(|| ShexError::shexj("Annotation: missing \"object\""))?;
        let object = match object {
            Value::String(iri) => ObjectValue::Iri(self.iri(iri)?),
            other => ObjectValue::Literal(self.object_literal(other)?),
        };
        obj.finish()?;
        Ok(Annotation { predicate, object })
    }
}

fn node_kind_from_str(s: &str) -> Result<NodeKind> {
    match s {
        "iri" => Ok(NodeKind::Iri),
        "bnode" => Ok(NodeKind::BNode),
        "nonliteral" => Ok(NodeKind::NonLiteral),
        "literal" => Ok(NodeKind::Literal),
        other => Err(ShexError::shexj(format!(
            "NodeConstraint: unknown nodeKind {other:?}"
        ))),
    }
}

/// A `LiteralStemRange` / `LanguageStemRange` stem: a plain string (a literal
/// prefix or a language-tag prefix — neither is an IRI), or `Wildcard`.
fn plain_stem(value: &Value, what: &'static str) -> Result<StemValue> {
    match value {
        Value::String(s) => Ok(StemValue::Str(s.clone())),
        Value::Object(_) => wildcard_stem(value, what),
        _ => Err(ShexError::shexj(format!(
            "{what}: stem must be a string or Wildcard"
        ))),
    }
}

/// Admit a complete language tag out of an untrusted ShExJ document, on the one
/// grammar this crate owns.
///
/// ShExJ is JSON, so nothing between the file and the AST lexes anything: a
/// `"language"`, a `"languageTag"` and a `LanguageStemRange` string exclusion are
/// each a bare `String` lifted straight out of the JSON value. The ShExC lexer holds
/// the identical positions to [`crate::lexer::LANGTAG_PROFILE`] (`@tag`), and
/// [`crate::shexc`] writes every one of them back out **verbatim** after `@`. So
/// without this call the crate refuses its own output: `parse_shexj` on
/// `{"language": "en us"}` → `to_shexc` writing `"v"@en us` → `parse_shexc`
/// rejecting it as a lex error. Refusing at ingress keeps the two syntaxes
/// denoting the same set of schemas, which is the whole contract between them.
///
/// The judgement is on the whole tag, so a `LanguageStem`/`LanguageStemRange`
/// *stem* goes through [`check_language_stem`] instead, which admits the empty
/// stem on top of this. It is **not** left ungated: see that function for why the
/// "a stem is only a prefix" reading was an escape rather than a carve-out.
///
/// # Errors
/// [`ShexError::Shexj`] naming the member, the tag, and `purrdf-iri`'s own
/// `langtag-*` diagnostic code — the same code `crate::lexer` quotes for the same
/// condition, because ShEx does not get its own vocabulary for a grammar the
/// workspace shares.
fn check_language_tag(tag: &str, what: &str) -> Result<()> {
    match langtag::parse_with(tag, crate::lexer::LANGTAG_PROFILE) {
        Ok(_) => Ok(()),
        Err(error) => Err(ShexError::shexj(format!(
            "{what}: malformed language tag {tag:?}: {error} [{code}]",
            code = error.diagnostic_code()
        ))),
    }
}

/// Admit a language **stem** out of an untrusted ShExJ document: the empty stem,
/// or a complete tag on the one grammar this crate owns.
///
/// A stem is an RFC 4647 basic-filtering PREFIX — `@en~` matches `en-US` — and
/// that reading is why this position was originally left ungated. The reading is
/// true and the conclusion does not follow, because the stem is not only matched,
/// it is also **written**: [`crate::shexc`] emits it verbatim between `@` and `~`,
/// and [`crate::lexer`] then lexes that text with the `LANGTAG` terminal. So an
/// unjudged stem is the founding symptom of this whole gate, one field over —
/// `{"type":"LanguageStem","stem":"en-"}` parses, `to_shexc` writes `[@en-~]`, and
/// `parse_shexc` refuses it as `langtag-subtag-length-zero`. The crate refuses its
/// own output. Same for `"en us"`, `"abcdefghi"`, and every other string the
/// terminal cannot spell.
///
/// Gating it here costs a legitimate schema **nothing**, because the admitted set
/// is exactly the set ShExC can produce. [`crate::parser`] builds a language stem
/// from a `Token::LangTag` — which is a complete tag, already on
/// [`crate::lexer::LANGTAG_PROFILE`] — or from the empty stem `@~`, the legal "any
/// language" wildcard, and from nothing else. There is no third production. So
/// `is_empty() ||` a whole-tag judgement admits every ShExC-writable stem and
/// refuses only stems no ShExC schema could have contained; the vendored shexTest
/// corpus, whose stems are `""`, `fr`, `fr-be`, `fr-cd` and `fr-ch`, is untouched.
///
/// A prefix that is not itself a tag (`en-`) is therefore not a schema this crate
/// ever has to represent — ShExC cannot write one, and ShExJ that contains one is
/// a document ShExJ and ShExC disagree about, which is the precise condition the
/// two-syntax contract exists to prevent.
///
/// # Errors
/// [`ShexError::Shexj`] from [`check_language_tag`], naming the member, the stem
/// and `purrdf-iri`'s `langtag-*` diagnostic code.
fn check_language_stem(stem: &str, what: &str) -> Result<()> {
    if stem.is_empty() {
        return Ok(());
    }
    check_language_tag(stem, what)
}

/// Read the `{"type": "Wildcard"}` stem object.
fn wildcard_stem(value: &Value, what: &'static str) -> Result<StemValue> {
    let obj = Obj::typed(value, "Wildcard", what)?;
    obj.finish()?;
    Ok(StemValue::Wildcard)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "http://example.org/dir/doc.shexj";

    /// A ShExJ document exercising one relative reference in every IRI-valued
    /// position the ShExJ context types `"@type": "@id"`, plus the non-IRI
    /// positions that must survive verbatim beside them.
    const EVERY_POSITION: &str = concat!(
        r#"{"type":"Schema","imports":["imported"],"#,
        r#""startActs":[{"type":"SemAct","name":"ext"}],"#,
        r#""start":"Started","#,
        r#""shapes":[{"type":"Shape","id":"S1","extra":["extra1"],"#,
        r#""expression":{"type":"EachOf","id":"_:group","expressions":["#,
        r#"{"type":"TripleConstraint","id":"_:tc","predicate":"p1","#,
        r#""valueExpr":{"type":"NodeConstraint","datatype":"dt","values":["#,
        r#""v1",{"type":"IriStem","stem":"stem1"},"#,
        r#"{"type":"IriStemRange","stem":"stem2","exclusions":["x1",{"type":"IriStem","stem":"x2"}]},"#,
        r#"{"type":"LiteralStem","stem":"lit"},"#,
        r#"{"type":"LanguageStem","stem":"en"},"#,
        r#"{"value":"7","type":"ldt"}]},"#,
        r#""annotations":[{"type":"Annotation","predicate":"ap","object":"ao"}]}]},"#,
        r#""semActs":[{"type":"SemAct","name":"shapeExt","code":"relative"}]},"#,
        r#"{"type":"Shape","id":"_:bnodeShape"}]}"#,
    );

    fn parse(base: Option<&str>) -> Result<Schema> {
        parse_shexj(EVERY_POSITION, base)
    }

    #[test]
    fn a_repeated_member_name_is_refused() {
        let doc = r#"{"type":"Schema","shapes":[{"type":"Shape","id":"http://example.org/S","closed":true,"closed":false}]}"#;
        let err = parse_shexj(doc, None).expect_err("a repeated member name is ambiguous");
        assert!(
            err.to_string().contains("repeats a member name"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn distinct_member_names_beside_a_repeated_value_are_accepted() {
        // The neighbour of the refusal: the same value twice under distinct names,
        // and the same name in two sibling objects, are not repeats.
        let doc = concat!(
            r#"{"type":"Schema","shapes":["#,
            r#"{"type":"Shape","id":"http://example.org/S","closed":true},"#,
            r#"{"type":"Shape","id":"http://example.org/T","closed":true}]}"#,
        );
        let schema = parse_shexj(doc, None).expect("no object repeats a member name");
        assert_eq!(schema.shapes.len(), 2);
    }

    #[test]
    fn nesting_is_bounded_at_128_open_containers() {
        let nested = |depth: usize| {
            // `Schema` and `shapes` are two containers; the rest is ShapeNot nesting.
            let mut doc = String::from(r#"{"type":"Schema","shapes":["#);
            let inner = depth - 3;
            doc.push_str(r#"{"type":"ShapeNot","id":"http://example.org/S","shapeExpr":"#);
            for _ in 1..inner {
                doc.push_str(r#"{"type":"ShapeNot","shapeExpr":"#);
            }
            doc.push_str(r#"{"type":"ShapeExternal"}"#);
            for _ in 0..inner {
                doc.push('}');
            }
            doc.push_str("]}");
            doc
        };
        let err = parse_shexj(&nested(129), None).expect_err("129 open containers");
        assert!(
            err.to_string().contains("nested"),
            "unexpected error: {err}"
        );
        parse_shexj(&nested(128), None).expect("128 open containers are within the bound");
    }

    /// A numeric facet holds the value its decimal spells: the witnesses are numbers a
    /// reader that is not correctly rounded (or the x87's double-rounded fast path) reads
    /// as a neighbour, which then admits or refuses the data next to the bound.
    #[test]
    fn numeric_facets_are_correctly_rounded() {
        use purrdf_xsd::ieee::reference as soft;

        let mut lexicals = soft::misread_decimals(30);
        lexicals.extend(soft::x87_fast_path_decimals(30));
        let shapes = lexicals
            .iter()
            .enumerate()
            .map(|(index, lexical)| {
                format!(r#"{{"type":"NodeConstraint","id":"N{index}","mininclusive":{lexical}}}"#)
            })
            .collect::<Vec<_>>()
            .join(",");
        let document = format!(r#"{{"type":"Schema","shapes":[{shapes}]}}"#);
        let schema = parse_shexj(&document, Some(BASE)).expect("schema");
        for (declaration, lexical) in schema.shapes.iter().zip(&lexicals) {
            let ShapeExpr::Node(constraint) = &declaration.expr else {
                panic!("expected a node constraint, found {:?}", declaration.expr);
            };
            let correct: f64 = lexical.parse().expect("decimal");
            assert_eq!(
                constraint.mininclusive,
                Some(NumericLiteral::Fractional(correct)),
                "{lexical}"
            );
        }
    }

    fn shape_of(schema: &Schema, index: usize) -> &Shape {
        match &schema.shapes[index].expr {
            ShapeExpr::Shape(shape) => shape,
            other => panic!("expected a Shape, found {other:?}"),
        }
    }

    #[test]
    fn every_iri_position_resolves_against_the_document_base() {
        let schema = parse(Some(BASE)).expect("every reference resolves");
        assert_eq!(schema.imports, vec!["http://example.org/dir/imported"]);
        assert_eq!(schema.start_acts[0].name, "http://example.org/dir/ext");
        assert_eq!(
            *schema.start.as_deref().expect("a start expression"),
            ShapeExpr::Ref("http://example.org/dir/Started".to_owned())
        );
        assert_eq!(schema.shapes[0].id, "http://example.org/dir/S1");

        let shape = shape_of(&schema, 0);
        assert_eq!(shape.extra, vec!["http://example.org/dir/extra1"]);
        assert_eq!(
            shape.sem_acts[0].name, "http://example.org/dir/shapeExt",
            "a semantic action's extension IRI is an IRI position"
        );

        let TripleExpr::EachOf(group) = shape.expression.as_ref().expect("an expression") else {
            panic!("expected an EachOf");
        };
        let TripleExpr::TripleConstraint(tc) = &group.expressions[0] else {
            panic!("expected a TripleConstraint");
        };
        assert_eq!(tc.predicate, "http://example.org/dir/p1");
        assert_eq!(
            tc.annotations[0].predicate, "http://example.org/dir/ap",
            "an annotation predicate is an IRI position"
        );
        assert_eq!(
            tc.annotations[0].object,
            ObjectValue::Iri("http://example.org/dir/ao".to_owned()),
            "an IRI-valued annotation object is an IRI position"
        );

        let ShapeExpr::Node(nc) = tc.value_expr.as_deref().expect("a value expression") else {
            panic!("expected a NodeConstraint");
        };
        assert_eq!(nc.datatype.as_deref(), Some("http://example.org/dir/dt"));
        let values = nc.values.as_ref().expect("a value set");
        assert_eq!(
            values[0],
            ValueSetValue::Iri("http://example.org/dir/v1".to_owned())
        );
        assert_eq!(
            values[1],
            ValueSetValue::IriStem {
                stem: "http://example.org/dir/stem1".to_owned()
            }
        );
        assert_eq!(
            values[2],
            ValueSetValue::IriStemRange {
                stem: StemValue::Str("http://example.org/dir/stem2".to_owned()),
                exclusions: vec![
                    IriExclusion::Iri("http://example.org/dir/x1".to_owned()),
                    IriExclusion::Stem("http://example.org/dir/x2".to_owned()),
                ],
            }
        );
        assert_eq!(
            values[5],
            ValueSetValue::Literal(ObjectLiteral {
                value: "7".to_owned(),
                language: None,
                datatype: Some("http://example.org/dir/ldt".to_owned()),
            }),
            "an ObjectLiteral's `type` key is its datatype IRI"
        );
    }

    #[test]
    fn non_iri_positions_keep_their_bytes() {
        let schema = parse(Some(BASE)).expect("every reference resolves");
        assert_eq!(
            schema.shapes[1].id, "_:bnodeShape",
            "a `_:` label is a JSON-LD blank node identifier, not a reference"
        );

        let shape = shape_of(&schema, 0);
        assert_eq!(
            shape.sem_acts[0].code.as_deref(),
            Some("relative"),
            "a semantic action's code is opaque text"
        );

        let TripleExpr::EachOf(group) = shape.expression.as_ref().expect("an expression") else {
            panic!("expected an EachOf");
        };
        assert_eq!(group.id.as_deref(), Some("_:group"));
        let TripleExpr::TripleConstraint(tc) = &group.expressions[0] else {
            panic!("expected a TripleConstraint");
        };
        assert_eq!(tc.id.as_deref(), Some("_:tc"));

        let ShapeExpr::Node(nc) = tc.value_expr.as_deref().expect("a value expression") else {
            panic!("expected a NodeConstraint");
        };
        let values = nc.values.as_ref().expect("a value set");
        assert_eq!(
            values[3],
            ValueSetValue::LiteralStem {
                stem: "lit".to_owned()
            },
            "a literal stem is a string prefix, never an IRI"
        );
        assert_eq!(
            values[4],
            ValueSetValue::LanguageStem {
                stem: "en".to_owned()
            },
            "a language stem is a language-tag prefix, never an IRI"
        );
    }

    #[test]
    fn a_relative_reference_with_no_base_is_refused_not_interned() {
        let err = parse(None).expect_err("no base, so nothing resolves");
        assert!(
            err.to_string().contains("iri-relative-no-base"),
            "the failure carries the shared diagnostic code: {err}"
        );
        assert!(
            matches!(err, ShexError::Iri { .. }),
            "typed as an IRI failure"
        );
    }

    /// Each IRI position is refused ON ITS OWN, so none of them is silently
    /// admitted because an earlier one happened to fail first.
    #[test]
    fn every_iri_position_is_refused_on_its_own_with_no_base() {
        for document in [
            r#"{"type":"Schema","imports":["i"],"shapes":[{"type":"Shape","id":"_:s"}]}"#,
            r#"{"type":"Schema","shapes":[{"type":"Shape","id":"S"}]}"#,
            r#"{"type":"Schema","start":"S","shapes":[{"type":"Shape","id":"_:s"}]}"#,
            r#"{"type":"Schema","startActs":[{"type":"SemAct","name":"e"}]}"#,
            r#"{"type":"Schema","shapes":[{"type":"Shape","id":"_:s","extra":["p"]}]}"#,
            concat!(
                r#"{"type":"Schema","shapes":[{"type":"Shape","id":"_:s","#,
                r#""expression":{"type":"TripleConstraint","predicate":"p"}}]}"#,
            ),
            concat!(
                r#"{"type":"Schema","shapes":[{"type":"Shape","id":"_:s","#,
                r#""expression":{"type":"TripleConstraint","predicate":"http://e.example/p","#,
                r#""valueExpr":{"type":"NodeConstraint","datatype":"d"}}}]}"#,
            ),
            concat!(
                r#"{"type":"Schema","shapes":[{"type":"Shape","id":"_:s","#,
                r#""expression":{"type":"TripleConstraint","predicate":"http://e.example/p","#,
                r#""valueExpr":{"type":"NodeConstraint","values":["v"]}}}]}"#,
            ),
            concat!(
                r#"{"type":"Schema","shapes":[{"type":"Shape","id":"_:s","#,
                r#""expression":{"type":"TripleConstraint","predicate":"http://e.example/p","#,
                r#""annotations":[{"type":"Annotation","predicate":"a","object":"o"}]}}]}"#,
            ),
        ] {
            let err = parse_shexj(document, None)
                .expect_err("a relative reference with no base must be refused");
            assert!(
                err.to_string().contains("iri-relative-no-base"),
                "{document}: {err}"
            );
            // And the SAME document resolves once a base is in scope.
            parse_shexj(document, Some(BASE)).expect("a base resolves it");
        }
    }

    #[test]
    fn a_malformed_base_is_reported_against_the_base_itself() {
        let err = parse_shexj(r#"{"type":"Schema"}"#, Some("not a base"))
            .expect_err("a relative base is not a base");
        assert!(err.to_string().contains("not a base"), "{err}");
    }

    /// The wire format emits only resolved absolute IRIs, so a re-read needs no
    /// base at all — which is what makes `to_shexj` a self-contained document.
    #[test]
    fn the_serialized_form_needs_no_base() {
        let schema = parse(Some(BASE)).expect("every reference resolves");
        let reparsed = parse_shexj(&to_shexj(&schema), None).expect("absolute output re-reads");
        assert_eq!(reparsed, schema);
    }
}
