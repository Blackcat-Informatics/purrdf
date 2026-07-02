// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
//! [`Schema`] also implements [`serde::Serialize`]/[`serde::Deserialize`]
//! by delegation to this module, so it can be embedded in larger serde
//! documents.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Map, Number, Value};

use crate::ast::{
    Annotation, IriExclusion, LanguageExclusion, LiteralExclusion, NodeConstraint, NodeKind,
    NumericLiteral, ObjectLiteral, ObjectValue, Schema, SemAct, Shape, ShapeDecl, ShapeExpr,
    StemValue, TripleConstraint, TripleExpr, TripleExprGroup, ValueSetValue,
};
use crate::error::{Result, ShexError};

/// The canonical `@context` IRI emitted on serialized schemas.
pub const SHEX_CONTEXT: &str = "http://www.w3.org/ns/shex.jsonld";

/// Parse a ShExJ document into a [`Schema`].
pub fn parse_shexj(input: &str) -> Result<Schema> {
    let value: Value =
        serde_json::from_str(input).map_err(|e| ShexError::shexj(format!("invalid JSON: {e}")))?;
    schema_from_value(&value).map_err(ShexError::shexj)
}

/// Serialize a [`Schema`] to pretty-printed ShExJ.
#[must_use]
pub fn to_shexj(schema: &Schema) -> String {
    let value = schema_to_value(schema);
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| String::from("{}"))
}

impl Serialize for Schema {
    fn serialize<S: Serializer>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error> {
        schema_to_value(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Schema {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> core::result::Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        schema_from_value(&value).map_err(D::Error::custom)
    }
}

// ── serialization (AST → Value) ─────────────────────────────────────────────

fn schema_to_value(schema: &Schema) -> Value {
    let mut obj = Map::new();
    obj.insert("@context".into(), json!(SHEX_CONTEXT));
    obj.insert("type".into(), json!("Schema"));
    if !schema.imports.is_empty() {
        obj.insert("imports".into(), json!(schema.imports));
    }
    if !schema.start_acts.is_empty() {
        let acts: Vec<Value> = schema.start_acts.iter().map(sem_act_to_value).collect();
        obj.insert("startActs".into(), Value::Array(acts));
    }
    if let Some(start) = &schema.start {
        obj.insert("start".into(), shape_expr_to_value(start));
    }
    if !schema.shapes.is_empty() {
        let shapes: Vec<Value> = schema.shapes.iter().map(shape_decl_to_value).collect();
        obj.insert("shapes".into(), Value::Array(shapes));
    }
    Value::Object(obj)
}

fn shape_decl_to_value(decl: &ShapeDecl) -> Value {
    // ShExJ 2.1 inlines the declaration id on the shape-expression object. A
    // bare reference cannot carry an id, so it is wrapped in a singleton
    // `ShapeAnd` (the only faithful 2.1 encoding).
    let body = match &decl.expr {
        ShapeExpr::Ref(_) => shape_expr_to_value(&ShapeExpr::And(vec![decl.expr.clone()])),
        other => shape_expr_to_value(other),
    };
    match body {
        Value::Object(mut obj) => {
            obj.insert("id".into(), json!(decl.id));
            Value::Object(obj)
        }
        other => other,
    }
}

fn shape_expr_to_value(expr: &ShapeExpr) -> Value {
    match expr {
        ShapeExpr::Ref(label) => json!(label),
        ShapeExpr::And(parts) => {
            let parts: Vec<Value> = parts.iter().map(shape_expr_to_value).collect();
            json!({"type": "ShapeAnd", "shapeExprs": parts})
        }
        ShapeExpr::Or(parts) => {
            let parts: Vec<Value> = parts.iter().map(shape_expr_to_value).collect();
            json!({"type": "ShapeOr", "shapeExprs": parts})
        }
        ShapeExpr::Not(inner) => {
            json!({"type": "ShapeNot", "shapeExpr": shape_expr_to_value(inner)})
        }
        ShapeExpr::External => json!({"type": "ShapeExternal"}),
        ShapeExpr::Node(nc) => node_constraint_to_value(nc),
        ShapeExpr::Shape(shape) => shape_to_value(shape),
    }
}

fn node_constraint_to_value(nc: &NodeConstraint) -> Value {
    let mut obj = Map::new();
    obj.insert("type".into(), json!("NodeConstraint"));
    if let Some(kind) = nc.node_kind {
        obj.insert("nodeKind".into(), json!(kind.as_str()));
    }
    if let Some(dt) = &nc.datatype {
        obj.insert("datatype".into(), json!(dt));
    }
    for (key, slot) in [
        ("length", nc.length),
        ("minlength", nc.minlength),
        ("maxlength", nc.maxlength),
        ("totaldigits", nc.totaldigits),
        ("fractiondigits", nc.fractiondigits),
    ] {
        if let Some(n) = slot {
            obj.insert(key.into(), json!(n));
        }
    }
    if let Some(p) = &nc.pattern {
        obj.insert("pattern".into(), json!(p));
    }
    if let Some(f) = &nc.flags {
        obj.insert("flags".into(), json!(f));
    }
    for (key, slot) in [
        ("mininclusive", nc.mininclusive),
        ("minexclusive", nc.minexclusive),
        ("maxinclusive", nc.maxinclusive),
        ("maxexclusive", nc.maxexclusive),
    ] {
        if let Some(n) = slot {
            obj.insert(key.into(), numeric_to_value(n));
        }
    }
    if let Some(values) = &nc.values {
        let values: Vec<Value> = values.iter().map(value_set_value_to_value).collect();
        obj.insert("values".into(), Value::Array(values));
    }
    Value::Object(obj)
}

fn numeric_to_value(n: NumericLiteral) -> Value {
    match n {
        NumericLiteral::Integer(i) => json!(i),
        NumericLiteral::Fractional(f) => {
            Number::from_f64(f).map_or_else(|| json!(0), Value::Number)
        }
    }
}

fn stem_to_value(stem: &StemValue) -> Value {
    match stem {
        StemValue::Str(s) => json!(s),
        StemValue::Wildcard => json!({"type": "Wildcard"}),
    }
}

fn value_set_value_to_value(v: &ValueSetValue) -> Value {
    match v {
        ValueSetValue::Iri(iri) => json!(iri),
        ValueSetValue::Literal(lit) => object_literal_to_value(lit),
        ValueSetValue::IriStem { stem } => json!({"type": "IriStem", "stem": stem}),
        ValueSetValue::IriStemRange { stem, exclusions } => {
            let exclusions: Vec<Value> = exclusions
                .iter()
                .map(|e| match e {
                    IriExclusion::Iri(iri) => json!(iri),
                    IriExclusion::Stem(stem) => json!({"type": "IriStem", "stem": stem}),
                })
                .collect();
            json!({"type": "IriStemRange", "stem": stem_to_value(stem), "exclusions": exclusions})
        }
        ValueSetValue::LiteralStem { stem } => json!({"type": "LiteralStem", "stem": stem}),
        ValueSetValue::LiteralStemRange { stem, exclusions } => {
            let exclusions: Vec<Value> = exclusions
                .iter()
                .map(|e| match e {
                    LiteralExclusion::Literal(v) => json!(v),
                    LiteralExclusion::Stem(stem) => {
                        json!({"type": "LiteralStem", "stem": stem})
                    }
                })
                .collect();
            json!({
                "type": "LiteralStemRange",
                "stem": stem_to_value(stem),
                "exclusions": exclusions,
            })
        }
        ValueSetValue::Language { language_tag } => {
            json!({"type": "Language", "languageTag": language_tag})
        }
        ValueSetValue::LanguageStem { stem } => json!({"type": "LanguageStem", "stem": stem}),
        ValueSetValue::LanguageStemRange { stem, exclusions } => {
            let exclusions: Vec<Value> = exclusions
                .iter()
                .map(|e| match e {
                    LanguageExclusion::Language(tag) => json!(tag),
                    LanguageExclusion::Stem(stem) => {
                        json!({"type": "LanguageStem", "stem": stem})
                    }
                })
                .collect();
            json!({
                "type": "LanguageStemRange",
                "stem": stem_to_value(stem),
                "exclusions": exclusions,
            })
        }
    }
}

fn object_literal_to_value(lit: &ObjectLiteral) -> Value {
    let mut obj = Map::new();
    obj.insert("value".into(), json!(lit.value));
    if let Some(lang) = &lit.language {
        obj.insert("language".into(), json!(lang));
    }
    if let Some(dt) = &lit.datatype {
        obj.insert("type".into(), json!(dt));
    }
    Value::Object(obj)
}

fn shape_to_value(shape: &Shape) -> Value {
    let mut obj = Map::new();
    obj.insert("type".into(), json!("Shape"));
    if let Some(closed) = shape.closed {
        obj.insert("closed".into(), json!(closed));
    }
    if !shape.extra.is_empty() {
        obj.insert("extra".into(), json!(shape.extra));
    }
    if let Some(expr) = &shape.expression {
        obj.insert("expression".into(), triple_expr_to_value(expr));
    }
    insert_acts_annots(&mut obj, &shape.sem_acts, &shape.annotations);
    Value::Object(obj)
}

fn insert_acts_annots(obj: &mut Map<String, Value>, sem_acts: &[SemAct], annots: &[Annotation]) {
    if !sem_acts.is_empty() {
        let acts: Vec<Value> = sem_acts.iter().map(sem_act_to_value).collect();
        obj.insert("semActs".into(), Value::Array(acts));
    }
    if !annots.is_empty() {
        let annots: Vec<Value> = annots.iter().map(annotation_to_value).collect();
        obj.insert("annotations".into(), Value::Array(annots));
    }
}

fn triple_expr_to_value(expr: &TripleExpr) -> Value {
    match expr {
        TripleExpr::Ref(label) => json!(label),
        TripleExpr::EachOf(group) => group_to_value("EachOf", group),
        TripleExpr::OneOf(group) => group_to_value("OneOf", group),
        TripleExpr::TripleConstraint(tc) => {
            let mut obj = Map::new();
            obj.insert("type".into(), json!("TripleConstraint"));
            if let Some(id) = &tc.id {
                obj.insert("id".into(), json!(id));
            }
            if let Some(inverse) = tc.inverse {
                obj.insert("inverse".into(), json!(inverse));
            }
            obj.insert("predicate".into(), json!(tc.predicate));
            if let Some(ve) = &tc.value_expr {
                obj.insert("valueExpr".into(), shape_expr_to_value(ve));
            }
            insert_min_max(&mut obj, tc.min, tc.max);
            insert_acts_annots(&mut obj, &tc.sem_acts, &tc.annotations);
            Value::Object(obj)
        }
    }
}

fn group_to_value(kind: &str, group: &TripleExprGroup) -> Value {
    let mut obj = Map::new();
    obj.insert("type".into(), json!(kind));
    if let Some(id) = &group.id {
        obj.insert("id".into(), json!(id));
    }
    let members: Vec<Value> = group.expressions.iter().map(triple_expr_to_value).collect();
    obj.insert("expressions".into(), Value::Array(members));
    insert_min_max(&mut obj, group.min, group.max);
    insert_acts_annots(&mut obj, &group.sem_acts, &group.annotations);
    Value::Object(obj)
}

fn insert_min_max(obj: &mut Map<String, Value>, min: Option<i64>, max: Option<i64>) {
    if let Some(min) = min {
        obj.insert("min".into(), json!(min));
    }
    if let Some(max) = max {
        obj.insert("max".into(), json!(max));
    }
}

fn sem_act_to_value(act: &SemAct) -> Value {
    let mut obj = Map::new();
    obj.insert("type".into(), json!("SemAct"));
    obj.insert("name".into(), json!(act.name));
    if let Some(code) = &act.code {
        obj.insert("code".into(), json!(code));
    }
    Value::Object(obj)
}

fn annotation_to_value(annotation: &Annotation) -> Value {
    let object = match &annotation.object {
        ObjectValue::Iri(iri) => json!(iri),
        ObjectValue::Literal(lit) => object_literal_to_value(lit),
    };
    json!({
        "type": "Annotation",
        "predicate": annotation.predicate,
        "object": object,
    })
}

// ── deserialization (Value → AST) ────────────────────────────────────────────

type ParseResult<T> = core::result::Result<T, String>;

/// A strict object reader: every key must be consumed, or the read fails.
struct Obj<'a> {
    map: &'a Map<String, Value>,
    taken: Vec<&'a str>,
    what: &'static str,
}

impl<'a> Obj<'a> {
    fn new(value: &'a Value, what: &'static str) -> ParseResult<Self> {
        let map = value
            .as_object()
            .ok_or_else(|| format!("{what}: expected a JSON object"))?;
        Ok(Self {
            map,
            taken: Vec::new(),
            what,
        })
    }

    fn typed(value: &'a Value, expected: &str, what: &'static str) -> ParseResult<Self> {
        let mut obj = Self::new(value, what)?;
        let ty = obj.take_str("type")?;
        if ty != expected {
            return Err(format!("{what}: expected type {expected:?}, found {ty:?}"));
        }
        Ok(obj)
    }

    fn take(&mut self, key: &'static str) -> Option<&'a Value> {
        let v = self.map.get(key)?;
        self.taken.push(key);
        Some(v)
    }

    fn take_str(&mut self, key: &'static str) -> ParseResult<String> {
        self.take_str_opt(key)?
            .ok_or_else(|| format!("{}: missing {key:?}", self.what))
    }

    fn take_str_opt(&mut self, key: &'static str) -> ParseResult<Option<String>> {
        match self.take(key) {
            None => Ok(None),
            Some(Value::String(s)) => Ok(Some(s.clone())),
            Some(_) => Err(format!("{}: {key:?} must be a string", self.what)),
        }
    }

    fn take_u64_opt(&mut self, key: &'static str) -> ParseResult<Option<u64>> {
        match self.take(key) {
            None => Ok(None),
            Some(Value::Number(n)) => n
                .as_u64()
                .map(Some)
                .ok_or_else(|| format!("{}: {key:?} must be a non-negative integer", self.what)),
            Some(_) => Err(format!("{}: {key:?} must be a number", self.what)),
        }
    }

    fn take_i64_opt(&mut self, key: &'static str) -> ParseResult<Option<i64>> {
        match self.take(key) {
            None => Ok(None),
            Some(Value::Number(n)) => n
                .as_i64()
                .map(Some)
                .ok_or_else(|| format!("{}: {key:?} must be an integer", self.what)),
            Some(_) => Err(format!("{}: {key:?} must be a number", self.what)),
        }
    }

    fn take_bool_opt(&mut self, key: &'static str) -> ParseResult<Option<bool>> {
        match self.take(key) {
            None => Ok(None),
            Some(Value::Bool(b)) => Ok(Some(*b)),
            Some(_) => Err(format!("{}: {key:?} must be a boolean", self.what)),
        }
    }

    fn take_numeric_opt(&mut self, key: &'static str) -> ParseResult<Option<NumericLiteral>> {
        match self.take(key) {
            None => Ok(None),
            Some(Value::Number(n)) => {
                if let Some(i) = n.as_i64() {
                    Ok(Some(NumericLiteral::Integer(i)))
                } else if let Some(f) = n.as_f64() {
                    Ok(Some(NumericLiteral::Fractional(f)))
                } else {
                    Err(format!("{}: {key:?} out of range", self.what))
                }
            }
            Some(_) => Err(format!("{}: {key:?} must be a number", self.what)),
        }
    }

    fn take_array(&mut self, key: &'static str) -> ParseResult<Option<&'a [Value]>> {
        match self.take(key) {
            None => Ok(None),
            Some(Value::Array(items)) => Ok(Some(items)),
            Some(_) => Err(format!("{}: {key:?} must be an array", self.what)),
        }
    }

    /// Fail on any key that was never consumed (strict ShExJ 2.1 reading).
    fn finish(self) -> ParseResult<()> {
        for key in self.map.keys() {
            if !self.taken.contains(&key.as_str()) {
                return Err(format!("{}: unknown key {key:?}", self.what));
            }
        }
        Ok(())
    }
}

fn schema_from_value(value: &Value) -> ParseResult<Schema> {
    let mut obj = Obj::typed(value, "Schema", "Schema")?;
    let _context = obj.take("@context");
    let mut schema = Schema::default();
    if let Some(items) = obj.take_array("imports")? {
        for item in items {
            let Value::String(iri) = item else {
                return Err("Schema: imports entries must be strings".into());
            };
            schema.imports.push(iri.clone());
        }
    }
    if let Some(items) = obj.take_array("startActs")? {
        for item in items {
            schema.start_acts.push(sem_act_from_value(item)?);
        }
    }
    if let Some(start) = obj.take("start") {
        schema.start = Some(Box::new(shape_expr_from_value(start)?));
    }
    if let Some(items) = obj.take_array("shapes")? {
        for item in items {
            schema.shapes.push(shape_decl_from_value(item)?);
        }
    }
    obj.finish()?;
    Ok(schema)
}

fn shape_decl_from_value(value: &Value) -> ParseResult<ShapeDecl> {
    // A `shapes` entry is a shape-expression object with an inlined `id`;
    // strip the id and re-read the remainder as a plain shape expression.
    let map = value
        .as_object()
        .ok_or_else(|| "shapes entry: expected a JSON object".to_owned())?;
    let Some(Value::String(id)) = map.get("id") else {
        return Err("shapes entry: missing string \"id\"".into());
    };
    let mut rest = map.clone();
    rest.remove("id");
    let expr = shape_expr_from_value(&Value::Object(rest))?;
    Ok(ShapeDecl {
        id: id.clone(),
        expr,
    })
}

fn shape_expr_from_value(value: &Value) -> ParseResult<ShapeExpr> {
    if let Value::String(label) = value {
        return Ok(ShapeExpr::Ref(label.clone()));
    }
    let ty = value
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| "shapeExpr: missing \"type\" discriminator".to_owned())?;
    match ty {
        "ShapeAnd" | "ShapeOr" => {
            let mut obj = Obj::typed(value, ty, "ShapeAnd/ShapeOr")?;
            let items = obj
                .take_array("shapeExprs")?
                .ok_or_else(|| format!("{ty}: missing \"shapeExprs\""))?;
            let parts = items
                .iter()
                .map(shape_expr_from_value)
                .collect::<ParseResult<Vec<_>>>()?;
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
                .ok_or_else(|| "ShapeNot: missing \"shapeExpr\"".to_owned())?;
            let inner = shape_expr_from_value(inner)?;
            obj.finish()?;
            Ok(ShapeExpr::Not(Box::new(inner)))
        }
        "ShapeExternal" => {
            let obj = Obj::typed(value, "ShapeExternal", "ShapeExternal")?;
            obj.finish()?;
            Ok(ShapeExpr::External)
        }
        "NodeConstraint" => Ok(ShapeExpr::Node(node_constraint_from_value(value)?)),
        "Shape" => Ok(ShapeExpr::Shape(shape_from_value(value)?)),
        other => Err(format!("shapeExpr: unknown type {other:?}")),
    }
}

fn node_kind_from_str(s: &str) -> ParseResult<NodeKind> {
    match s {
        "iri" => Ok(NodeKind::Iri),
        "bnode" => Ok(NodeKind::BNode),
        "nonliteral" => Ok(NodeKind::NonLiteral),
        "literal" => Ok(NodeKind::Literal),
        other => Err(format!("NodeConstraint: unknown nodeKind {other:?}")),
    }
}

fn node_constraint_from_value(value: &Value) -> ParseResult<NodeConstraint> {
    let mut obj = Obj::typed(value, "NodeConstraint", "NodeConstraint")?;
    let mut nc = NodeConstraint {
        node_kind: obj
            .take_str_opt("nodeKind")?
            .map(|s| node_kind_from_str(&s))
            .transpose()?,
        datatype: obj.take_str_opt("datatype")?,
        length: obj.take_u64_opt("length")?,
        minlength: obj.take_u64_opt("minlength")?,
        maxlength: obj.take_u64_opt("maxlength")?,
        pattern: obj.take_str_opt("pattern")?,
        flags: obj.take_str_opt("flags")?,
        mininclusive: obj.take_numeric_opt("mininclusive")?,
        minexclusive: obj.take_numeric_opt("minexclusive")?,
        maxinclusive: obj.take_numeric_opt("maxinclusive")?,
        maxexclusive: obj.take_numeric_opt("maxexclusive")?,
        totaldigits: obj.take_u64_opt("totaldigits")?,
        fractiondigits: obj.take_u64_opt("fractiondigits")?,
        values: None,
    };
    if let Some(items) = obj.take_array("values")? {
        let values = items
            .iter()
            .map(value_set_value_from_value)
            .collect::<ParseResult<Vec<_>>>()?;
        nc.values = Some(values);
    }
    obj.finish()?;
    Ok(nc)
}

fn stem_from_value(value: &Value, what: &'static str) -> ParseResult<StemValue> {
    match value {
        Value::String(s) => Ok(StemValue::Str(s.clone())),
        Value::Object(_) => {
            let obj = Obj::typed(value, "Wildcard", what)?;
            obj.finish()?;
            Ok(StemValue::Wildcard)
        }
        _ => Err(format!("{what}: stem must be a string or Wildcard")),
    }
}

fn value_set_value_from_value(value: &Value) -> ParseResult<ValueSetValue> {
    if let Value::String(iri) = value {
        return Ok(ValueSetValue::Iri(iri.clone()));
    }
    // NB: an ObjectLiteral's "type" key is its datatype IRI, so only the known
    // stem/range discriminators select a non-literal variant.
    let ty = value.get("type").and_then(Value::as_str);
    match ty {
        None => Ok(ValueSetValue::Literal(object_literal_from_value(value)?)),
        Some("IriStem") => {
            let mut obj = Obj::typed(value, "IriStem", "IriStem")?;
            let stem = obj.take_str("stem")?;
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
            obj.finish()?;
            Ok(ValueSetValue::LanguageStem { stem })
        }
        Some("Language") => {
            let mut obj = Obj::typed(value, "Language", "Language")?;
            let language_tag = obj.take_str("languageTag")?;
            obj.finish()?;
            Ok(ValueSetValue::Language { language_tag })
        }
        Some("IriStemRange") => {
            let mut obj = Obj::typed(value, "IriStemRange", "IriStemRange")?;
            let stem = obj
                .take("stem")
                .ok_or_else(|| "IriStemRange: missing \"stem\"".to_owned())
                .and_then(|v| stem_from_value(v, "IriStemRange"))?;
            let exclusions = obj
                .take_array("exclusions")?
                .unwrap_or_default()
                .iter()
                .map(|e| match e {
                    Value::String(iri) => Ok(IriExclusion::Iri(iri.clone())),
                    other => {
                        let mut obj = Obj::typed(other, "IriStem", "IriStemRange exclusion")?;
                        let stem = obj.take_str("stem")?;
                        obj.finish()?;
                        Ok(IriExclusion::Stem(stem))
                    }
                })
                .collect::<ParseResult<Vec<_>>>()?;
            obj.finish()?;
            Ok(ValueSetValue::IriStemRange { stem, exclusions })
        }
        Some("LiteralStemRange") => {
            let mut obj = Obj::typed(value, "LiteralStemRange", "LiteralStemRange")?;
            let stem = obj
                .take("stem")
                .ok_or_else(|| "LiteralStemRange: missing \"stem\"".to_owned())
                .and_then(|v| stem_from_value(v, "LiteralStemRange"))?;
            let exclusions = obj
                .take_array("exclusions")?
                .unwrap_or_default()
                .iter()
                .map(|e| match e {
                    Value::String(v) => Ok(LiteralExclusion::Literal(v.clone())),
                    other => {
                        let mut obj =
                            Obj::typed(other, "LiteralStem", "LiteralStemRange exclusion")?;
                        let stem = obj.take_str("stem")?;
                        obj.finish()?;
                        Ok(LiteralExclusion::Stem(stem))
                    }
                })
                .collect::<ParseResult<Vec<_>>>()?;
            obj.finish()?;
            Ok(ValueSetValue::LiteralStemRange { stem, exclusions })
        }
        Some("LanguageStemRange") => {
            let mut obj = Obj::typed(value, "LanguageStemRange", "LanguageStemRange")?;
            let stem = obj
                .take("stem")
                .ok_or_else(|| "LanguageStemRange: missing \"stem\"".to_owned())
                .and_then(|v| stem_from_value(v, "LanguageStemRange"))?;
            let exclusions = obj
                .take_array("exclusions")?
                .unwrap_or_default()
                .iter()
                .map(|e| match e {
                    Value::String(tag) => Ok(LanguageExclusion::Language(tag.clone())),
                    other => {
                        let mut obj =
                            Obj::typed(other, "LanguageStem", "LanguageStemRange exclusion")?;
                        let stem = obj.take_str("stem")?;
                        obj.finish()?;
                        Ok(LanguageExclusion::Stem(stem))
                    }
                })
                .collect::<ParseResult<Vec<_>>>()?;
            obj.finish()?;
            Ok(ValueSetValue::LanguageStemRange { stem, exclusions })
        }
        Some(_) => Ok(ValueSetValue::Literal(object_literal_from_value(value)?)),
    }
}

fn object_literal_from_value(value: &Value) -> ParseResult<ObjectLiteral> {
    let mut obj = Obj::new(value, "ObjectLiteral")?;
    let lit = ObjectLiteral {
        value: obj.take_str("value")?,
        language: obj.take_str_opt("language")?,
        datatype: obj.take_str_opt("type")?,
    };
    obj.finish()?;
    Ok(lit)
}

fn shape_from_value(value: &Value) -> ParseResult<Shape> {
    let mut obj = Obj::typed(value, "Shape", "Shape")?;
    let mut shape = Shape {
        closed: obj.take_bool_opt("closed")?,
        ..Shape::default()
    };
    if let Some(items) = obj.take_array("extra")? {
        for item in items {
            let Value::String(iri) = item else {
                return Err("Shape: extra entries must be strings".into());
            };
            shape.extra.push(iri.clone());
        }
    }
    if let Some(expr) = obj.take("expression") {
        shape.expression = Some(triple_expr_from_value(expr)?);
    }
    shape.sem_acts = take_sem_acts(&mut obj)?;
    shape.annotations = take_annotations(&mut obj)?;
    obj.finish()?;
    Ok(shape)
}

fn take_sem_acts(obj: &mut Obj<'_>) -> ParseResult<Vec<SemAct>> {
    obj.take_array("semActs")?
        .unwrap_or_default()
        .iter()
        .map(sem_act_from_value)
        .collect()
}

fn take_annotations(obj: &mut Obj<'_>) -> ParseResult<Vec<Annotation>> {
    obj.take_array("annotations")?
        .unwrap_or_default()
        .iter()
        .map(annotation_from_value)
        .collect()
}

fn triple_expr_from_value(value: &Value) -> ParseResult<TripleExpr> {
    if let Value::String(label) = value {
        return Ok(TripleExpr::Ref(label.clone()));
    }
    let ty = value
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| "tripleExpr: missing \"type\" discriminator".to_owned())?;
    match ty {
        "EachOf" | "OneOf" => {
            let mut obj = Obj::typed(value, ty, "EachOf/OneOf")?;
            let mut group = TripleExprGroup {
                id: obj.take_str_opt("id")?,
                min: obj.take_i64_opt("min")?,
                max: obj.take_i64_opt("max")?,
                ..TripleExprGroup::default()
            };
            let items = obj
                .take_array("expressions")?
                .ok_or_else(|| format!("{ty}: missing \"expressions\""))?;
            group.expressions = items
                .iter()
                .map(triple_expr_from_value)
                .collect::<ParseResult<Vec<_>>>()?;
            group.sem_acts = take_sem_acts(&mut obj)?;
            group.annotations = take_annotations(&mut obj)?;
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
                id: obj.take_str_opt("id")?,
                inverse: obj.take_bool_opt("inverse")?,
                predicate: obj.take_str("predicate")?,
                min: obj.take_i64_opt("min")?,
                max: obj.take_i64_opt("max")?,
                ..TripleConstraint::default()
            };
            if let Some(ve) = obj.take("valueExpr") {
                tc.value_expr = Some(Box::new(shape_expr_from_value(ve)?));
            }
            tc.sem_acts = take_sem_acts(&mut obj)?;
            tc.annotations = take_annotations(&mut obj)?;
            obj.finish()?;
            Ok(TripleExpr::TripleConstraint(tc))
        }
        other => Err(format!("tripleExpr: unknown type {other:?}")),
    }
}

fn sem_act_from_value(value: &Value) -> ParseResult<SemAct> {
    let mut obj = Obj::typed(value, "SemAct", "SemAct")?;
    let act = SemAct {
        name: obj.take_str("name")?,
        code: obj.take_str_opt("code")?,
    };
    obj.finish()?;
    Ok(act)
}

fn annotation_from_value(value: &Value) -> ParseResult<Annotation> {
    let mut obj = Obj::typed(value, "Annotation", "Annotation")?;
    let predicate = obj.take_str("predicate")?;
    let object = obj
        .take("object")
        .ok_or_else(|| "Annotation: missing \"object\"".to_owned())?;
    let object = match object {
        Value::String(iri) => ObjectValue::Iri(iri.clone()),
        other => ObjectValue::Literal(object_literal_from_value(other)?),
    };
    obj.finish()?;
    Ok(Annotation { predicate, object })
}
