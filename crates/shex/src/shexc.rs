// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! ShExC — the compact syntax serializer for ShEx schemas (spec §6).
//!
//! Renders a [`Schema`] AST back to Shape Expressions Compact Syntax, the
//! inverse of [`crate::parser::parse_shexc`]. Because the AST carries every
//! IRI in absolute form (labels, predicates, datatypes, value-set members),
//! this serializer emits every IRI as a full `<…>` reference and declares no
//! prefixes — so its output re-parses with no `@prefix`/`BASE` context and
//! `parse_shexc(to_shexc(schema), None)` reproduces `schema` exactly. This is
//! what `tests/shexc_roundtrip.rs` asserts over the entire shexTest corpus.
//!
//! # Scope
//!
//! ShExC is a proper subset of ShExJ's expressible schemas. The AST can hold a
//! handful of shapes that only ShExJ can spell — e.g. a [`ShapeExpr::External`]
//! nested inside another expression, a `closed: false` shape, or a stem *range*
//! with an empty exclusion list. Those never arise from a ShExC parse, so the
//! corpus round-trip is exact; when serializing a ShExJ-only schema, this
//! emitter renders the nearest faithful ShExC (documented at each site) rather
//! than failing.

use core::fmt::Write as _;

use crate::ast::{
    Annotation, IriExclusion, LanguageExclusion, LiteralExclusion, NodeConstraint, NodeKind,
    NumericLiteral, ObjectLiteral, ObjectValue, Schema, SemAct, Shape, ShapeDecl, ShapeExpr,
    StemValue, TripleConstraint, TripleExpr, TripleExprGroup, ValueSetValue,
};

/// Serialize a [`Schema`] to compact syntax (ShExC).
///
/// The output declares no prefixes and uses absolute `<…>` IRIs throughout, so
/// it re-parses with `base = None`.
#[must_use]
pub fn to_shexc(schema: &Schema) -> String {
    let mut out = String::new();
    for import in &schema.imports {
        out.push_str("IMPORT ");
        iri(&mut out, import);
        out.push('\n');
    }
    for act in &schema.start_acts {
        sem_act(&mut out, act);
        out.push('\n');
    }
    if let Some(start) = &schema.start {
        out.push_str("start = ");
        shape_expr(&mut out, start, false);
        out.push('\n');
    }
    for decl in &schema.shapes {
        shape_decl(&mut out, decl);
        out.push('\n');
    }
    out
}

// ── declarations ────────────────────────────────────────────────────────────

fn shape_decl(out: &mut String, decl: &ShapeDecl) {
    label(out, &decl.id);
    out.push(' ');
    if matches!(decl.expr, ShapeExpr::External) {
        // `EXTERNAL` is only a shapeExprDecl right-hand side in ShExC.
        out.push_str("EXTERNAL");
    } else {
        shape_expr(out, &decl.expr, false);
    }
}

// ── shape expressions ─────────────────────────────────────────────────────────

/// Emit a shape expression. `nested` requests parentheses around a composite
/// (`AND`/`OR`/`NOT`) so structure survives a re-parse; atoms ignore it.
fn shape_expr(out: &mut String, expr: &ShapeExpr, nested: bool) {
    match expr {
        ShapeExpr::And(parts) => composite(out, parts, " AND ", nested),
        ShapeExpr::Or(parts) => composite(out, parts, " OR ", nested),
        ShapeExpr::Not(inner) => {
            // `NOT` binds an atom, so a nested `NOT`/`AND`/`OR` needs its own
            // parentheses to remain a distinct atom (`NOT (NOT x)`), and the
            // operand is likewise emitted `nested`.
            if nested {
                out.push('(');
            }
            out.push_str("NOT ");
            shape_expr(out, inner, true);
            if nested {
                out.push(')');
            }
        }
        ShapeExpr::External => out.push_str("EXTERNAL"),
        ShapeExpr::Node(nc) => node_constraint(out, nc),
        ShapeExpr::Shape(s) => emit_shape(out, s),
        ShapeExpr::Ref(l) => {
            out.push('@');
            label(out, l);
        }
    }
}

fn composite(out: &mut String, parts: &[ShapeExpr], sep: &str, nested: bool) {
    if nested {
        out.push('(');
    }
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            out.push_str(sep);
        }
        // Operands are emitted `nested` so an inner composite gets its own
        // parentheses (and thus stays a distinct sub-expression).
        shape_expr(out, part, true);
    }
    if nested {
        out.push(')');
    }
}

// ── node constraints ──────────────────────────────────────────────────────────

fn node_constraint(out: &mut String, nc: &NodeConstraint) {
    let mut parts: Vec<String> = Vec::new();
    let has_primary = nc.node_kind.is_some() || nc.datatype.is_some() || nc.values.is_some();

    if let Some(kind) = nc.node_kind {
        parts.push(node_kind_kw(kind).to_owned());
    } else if let Some(dt) = &nc.datatype {
        parts.push(iri_string(dt));
    } else if let Some(values) = &nc.values {
        parts.push(value_set(values));
    }

    let has_numeric = nc.mininclusive.is_some()
        || nc.minexclusive.is_some()
        || nc.maxinclusive.is_some()
        || nc.maxexclusive.is_some()
        || nc.totaldigits.is_some()
        || nc.fractiondigits.is_some();

    // With no primary and a numeric facet, the FIRST token must be a numeric
    // facet so the parser enters its litNodeConstraint branch (a leading string
    // facet would route to the non-literal branch, which forbids numeric ones).
    if !has_primary && has_numeric {
        numeric_facets(&mut parts, nc);
        string_facets(&mut parts, nc);
    } else {
        string_facets(&mut parts, nc);
        numeric_facets(&mut parts, nc);
    }

    out.push_str(&parts.join(" "));
}

const fn node_kind_kw(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Iri => "IRI",
        NodeKind::BNode => "BNODE",
        NodeKind::NonLiteral => "NONLITERAL",
        NodeKind::Literal => "LITERAL",
    }
}

fn string_facets(parts: &mut Vec<String>, nc: &NodeConstraint) {
    if let Some(n) = nc.length {
        parts.push(format!("LENGTH {n}"));
    }
    if let Some(n) = nc.minlength {
        parts.push(format!("MINLENGTH {n}"));
    }
    if let Some(n) = nc.maxlength {
        parts.push(format!("MAXLENGTH {n}"));
    }
    if let Some(p) = &nc.pattern {
        parts.push(pattern(p, nc.flags.as_deref()));
    }
}

fn numeric_facets(parts: &mut Vec<String>, nc: &NodeConstraint) {
    for (kw, slot) in [
        ("MININCLUSIVE", nc.mininclusive),
        ("MINEXCLUSIVE", nc.minexclusive),
        ("MAXINCLUSIVE", nc.maxinclusive),
        ("MAXEXCLUSIVE", nc.maxexclusive),
    ] {
        if let Some(n) = slot {
            parts.push(format!("{kw} {}", numeric(n)));
        }
    }
    if let Some(n) = nc.totaldigits {
        parts.push(format!("TOTALDIGITS {n}"));
    }
    if let Some(n) = nc.fractiondigits {
        parts.push(format!("FRACTIONDIGITS {n}"));
    }
}

fn numeric(n: NumericLiteral) -> String {
    match n {
        NumericLiteral::Integer(i) => i.to_string(),
        NumericLiteral::Fractional(f) => {
            let s = format!("{f}");
            // A `Fractional` is non-integral by construction, so `{f}` always
            // carries a `.`/`e`; guard the invariant so the value can never
            // re-parse as an integer.
            if s.contains(['.', 'e', 'E']) {
                s
            } else {
                format!("{s}.0")
            }
        }
    }
}

/// `/pattern/flags`, with `/` escaped and any raw newline UCHAR-escaped so the
/// regex re-lexes to the identical stored source.
fn pattern(source: &str, flags: Option<&str>) -> String {
    let mut s = String::from('/');
    for c in source.chars() {
        match c {
            '/' => s.push_str("\\/"),
            '\n' => s.push_str("\\u000A"),
            '\r' => s.push_str("\\u000D"),
            _ => s.push(c),
        }
    }
    s.push('/');
    if let Some(flags) = flags {
        s.push_str(flags);
    }
    s
}

// ── value sets ────────────────────────────────────────────────────────────────

fn value_set(values: &[ValueSetValue]) -> String {
    let mut s = String::from('[');
    for (i, v) in values.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        value_set_value(&mut s, v);
    }
    s.push(']');
    s
}

fn value_set_value(out: &mut String, value: &ValueSetValue) {
    match value {
        ValueSetValue::Iri(i) => iri(out, i),
        ValueSetValue::Literal(lit) => literal(out, lit),
        ValueSetValue::IriStem { stem } => {
            iri(out, stem);
            out.push('~');
        }
        ValueSetValue::IriStemRange { stem, exclusions } => {
            stem_value(out, stem, iri);
            for ex in exclusions {
                out.push_str(" - ");
                match ex {
                    IriExclusion::Iri(i) => iri(out, i),
                    IriExclusion::Stem(i) => {
                        iri(out, i);
                        out.push('~');
                    }
                }
            }
        }
        ValueSetValue::LiteralStem { stem } => {
            string(out, stem);
            out.push('~');
        }
        ValueSetValue::LiteralStemRange { stem, exclusions } => {
            stem_value(out, stem, string);
            for ex in exclusions {
                out.push_str(" - ");
                match ex {
                    LiteralExclusion::Literal(l) => string(out, l),
                    LiteralExclusion::Stem(l) => {
                        string(out, l);
                        out.push('~');
                    }
                }
            }
        }
        ValueSetValue::Language { language_tag } => {
            out.push('@');
            out.push_str(language_tag);
        }
        ValueSetValue::LanguageStem { stem } => {
            out.push('@');
            out.push_str(stem);
            out.push('~');
        }
        ValueSetValue::LanguageStemRange { stem, exclusions } => {
            stem_value(out, stem, |o, s| {
                o.push('@');
                o.push_str(s);
            });
            for ex in exclusions {
                out.push_str(" - ");
                match ex {
                    LanguageExclusion::Language(t) => {
                        out.push('@');
                        out.push_str(t);
                    }
                    LanguageExclusion::Stem(t) => {
                        out.push('@');
                        out.push_str(t);
                        out.push('~');
                    }
                }
            }
        }
    }
}

/// Emit a stem-range head: a concrete stem `x~`, or the `.` wildcard.
fn stem_value(out: &mut String, stem: &StemValue, emit: impl Fn(&mut String, &str)) {
    match stem {
        StemValue::Str(s) => {
            emit(out, s);
            out.push('~');
        }
        StemValue::Wildcard => out.push('.'),
    }
}

// ── shapes & triple expressions ───────────────────────────────────────────────

fn emit_shape(out: &mut String, shape: &Shape) {
    if shape.closed == Some(true) {
        out.push_str("CLOSED ");
    }
    if !shape.extra.is_empty() {
        out.push_str("EXTRA");
        for p in &shape.extra {
            out.push(' ');
            iri(out, p);
        }
        out.push(' ');
    }
    out.push_str("{ ");
    if let Some(expr) = &shape.expression {
        triple_expr(out, expr, false);
        out.push(' ');
    }
    out.push('}');
    // Trailing annotations / semantic actions only ever attach to a
    // non-inline shape definition; an inline shape carries none, so emitting
    // the (empty) lists here is a no-op in that case.
    annotations(out, &shape.annotations);
    sem_acts(out, &shape.sem_acts);
}

/// Emit a triple expression. `member` = true when it sits inside a `;`/`|`
/// sequence, where a nested group must be parenthesized to keep its bounds.
fn triple_expr(out: &mut String, expr: &TripleExpr, member: bool) {
    match expr {
        TripleExpr::Ref(l) => {
            out.push('&');
            label(out, l);
        }
        TripleExpr::TripleConstraint(tc) => triple_constraint(out, tc),
        TripleExpr::EachOf(group) => group_expr(out, group, " ; ", member),
        TripleExpr::OneOf(group) => group_expr(out, group, " | ", member),
    }
}

fn group_expr(out: &mut String, group: &TripleExprGroup, sep: &str, member: bool) {
    let card = cardinality(group.min, group.max);
    let has_mods = group.id.is_some()
        || card.is_some()
        || !group.sem_acts.is_empty()
        || !group.annotations.is_empty();
    // Parenthesize when nested in a sequence, or when carrying modifiers that
    // must bind to the whole group.
    let paren = member || has_mods;

    if let Some(id) = &group.id {
        out.push('$');
        label(out, id);
        out.push(' ');
    }
    if paren {
        out.push('(');
    }
    for (i, member_expr) in group.expressions.iter().enumerate() {
        if i > 0 {
            out.push_str(sep);
        }
        triple_expr(out, member_expr, true);
    }
    if paren {
        out.push(')');
    }
    if let Some(card) = card {
        out.push_str(&card);
    }
    annotations(out, &group.annotations);
    sem_acts(out, &group.sem_acts);
}

fn triple_constraint(out: &mut String, tc: &TripleConstraint) {
    if let Some(id) = &tc.id {
        out.push('$');
        label(out, id);
        out.push(' ');
    }
    if tc.inverse == Some(true) {
        out.push('^');
    }
    iri(out, &tc.predicate);
    out.push(' ');
    match &tc.value_expr {
        Some(ve) => shape_expr(out, ve, false),
        // The absent value expression is the `.` wildcard.
        None => out.push('.'),
    }
    if let Some(card) = cardinality(tc.min, tc.max) {
        out.push_str(&card);
    }
    annotations(out, &tc.annotations);
    sem_acts(out, &tc.sem_acts);
}

fn cardinality(min: Option<i64>, max: Option<i64>) -> Option<String> {
    match (min, max) {
        (Some(0), Some(-1)) => Some("*".to_owned()),
        (Some(1), Some(-1)) => Some("+".to_owned()),
        (Some(0), Some(1)) => Some("?".to_owned()),
        (Some(m), Some(-1)) => Some(format!("{{{m},}}")),
        (Some(m), Some(n)) if m == n => Some(format!("{{{m}}}")),
        (Some(m), Some(n)) => Some(format!("{{{m},{n}}}")),
        _ => None,
    }
}

// ── annotations & semantic actions ────────────────────────────────────────────

fn annotations(out: &mut String, annots: &[Annotation]) {
    for a in annots {
        out.push_str(" // ");
        iri(out, &a.predicate);
        out.push(' ');
        match &a.object {
            ObjectValue::Iri(i) => iri(out, i),
            ObjectValue::Literal(lit) => literal(out, lit),
        }
    }
}

fn sem_acts(out: &mut String, acts: &[SemAct]) {
    for a in acts {
        out.push(' ');
        sem_act(out, a);
    }
}

fn sem_act(out: &mut String, act: &SemAct) {
    out.push('%');
    iri(out, &act.name);
    match &act.code {
        Some(code) => {
            out.push('{');
            for c in code.chars() {
                match c {
                    '\\' => out.push_str("\\\\"),
                    '%' => out.push_str("\\%"),
                    _ => out.push(c),
                }
            }
            out.push_str("%}");
        }
        None => out.push('%'),
    }
}

// ── literals, IRIs, labels ─────────────────────────────────────────────────────

fn literal(out: &mut String, lit: &ObjectLiteral) {
    string(out, &lit.value);
    if let Some(lang) = &lit.language {
        out.push('@');
        out.push_str(lang);
    } else if let Some(dt) = &lit.datatype {
        out.push_str("^^");
        iri(out, dt);
    }
}

/// A shape/triple-expression label: a blank node (`_:x`) verbatim, else an IRI.
fn label(out: &mut String, l: &str) {
    if l.starts_with("_:") {
        out.push_str(l);
    } else {
        iri(out, l);
    }
}

fn iri_string(i: &str) -> String {
    let mut s = String::new();
    iri(&mut s, i);
    s
}

/// `<…>` with every character an IRIREF forbids UCHAR-escaped.
fn iri(out: &mut String, i: &str) {
    out.push('<');
    for c in i.chars() {
        match c {
            '\u{0}'..='\u{20}' | '<' | '>' | '"' | '{' | '}' | '|' | '^' | '`' | '\\' => {
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            _ => out.push(c),
        }
    }
    out.push('>');
}

/// A `"…"` string literal with the four control escapes and `"`/`\` escaped.
fn string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            _ => out.push(c),
        }
    }
    out.push('"');
}
