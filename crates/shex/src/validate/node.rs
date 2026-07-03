// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Node-constraint satisfaction (ShEx 2.1 spec §5.4): node kind, datatype
//! with lexical-validity checking, string/numeric XML-Schema facets, and
//! value sets with stems, ranges and exclusions.

use purrdf_xsd::{value_cmp, XsdDatatype, XsdValue};

use super::pattern::compile_pattern;
use crate::ast::{
    IriExclusion, LanguageExclusion, LiteralExclusion, NodeConstraint, NodeKind, NumericLiteral,
    ObjectLiteral, StemValue, ValueSetValue,
};

/// The `xsd:string` datatype IRI (a plain literal's expanded datatype).
pub(crate) const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
/// The `rdf:langString` datatype IRI (a language-tagged literal's datatype).
pub(crate) const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// What sort of RDF term a focus/value node is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FactKind {
    /// An IRI.
    Iri,
    /// A blank node.
    Blank,
    /// A literal.
    Literal,
    /// An RDF 1.2 triple term (quoted triple).
    Triple,
}

/// The dataset-independent facts a node constraint inspects, extracted once
/// from either an interned [`purrdf_core::TermRef`] or a detached
/// [`purrdf_core::TermValue`].
#[derive(Clone, Copy, Debug)]
pub(crate) struct NodeFacts<'a> {
    /// The term's kind.
    pub kind: FactKind,
    /// The facet-relevant lexical form: the IRI string, the blank-node
    /// label, or the literal's lexical form (empty for triple terms).
    pub lexical: &'a str,
    /// The expanded datatype IRI, for literals.
    pub datatype: Option<&'a str>,
    /// The (lowercased) language tag, for language-tagged literals.
    pub language: Option<&'a str>,
}

impl NodeFacts<'_> {
    /// A short human description of the node, for failure reasons.
    fn describe(&self) -> String {
        match self.kind {
            FactKind::Iri => format!("IRI <{}>", self.lexical),
            FactKind::Blank => format!("blank node _:{}", self.lexical),
            FactKind::Literal => format!("literal {:?}", self.lexical),
            FactKind::Triple => "triple term".to_owned(),
        }
    }
}

/// Check every property of a [`NodeConstraint`] against a node (spec §5.4:
/// a node constraint is satisfied when ALL of its parts are).
pub(crate) fn check_node_constraint(
    nc: &NodeConstraint,
    facts: &NodeFacts<'_>,
) -> Result<(), String> {
    if let Some(kind) = nc.node_kind {
        check_node_kind(kind, facts)?;
    }
    if let Some(datatype) = &nc.datatype {
        check_datatype(datatype, facts)?;
    }
    check_string_facets(nc, facts)?;
    check_numeric_facets(nc, facts)?;
    if let Some(values) = &nc.values {
        check_value_set(values, facts)?;
    }
    Ok(())
}

// ── node kind ───────────────────────────────────────────────────────────────

fn check_node_kind(kind: NodeKind, facts: &NodeFacts<'_>) -> Result<(), String> {
    let ok = match kind {
        NodeKind::Iri => facts.kind == FactKind::Iri,
        NodeKind::BNode => facts.kind == FactKind::Blank,
        NodeKind::Literal => facts.kind == FactKind::Literal,
        NodeKind::NonLiteral => facts.kind != FactKind::Literal,
    };
    if ok {
        Ok(())
    } else {
        Err(format!(
            "node kind {} required, got {}",
            kind.as_str(),
            facts.describe()
        ))
    }
}

// ── datatype ────────────────────────────────────────────────────────────────

/// `true` iff `datatype` is one of the SPARQL operand datatypes whose
/// lexical forms the spec (§5.4.3) requires to be valid.
fn is_checked_datatype(datatype: XsdDatatype) -> bool {
    use XsdDatatype as D;
    matches!(
        datatype,
        D::Integer
            | D::Long
            | D::Int
            | D::Short
            | D::Byte
            | D::UnsignedLong
            | D::UnsignedInt
            | D::UnsignedShort
            | D::UnsignedByte
            | D::NonNegativeInteger
            | D::PositiveInteger
            | D::NonPositiveInteger
            | D::NegativeInteger
            | D::Decimal
            | D::Float
            | D::Double
            | D::Boolean
            | D::String
            | D::DateTime
    )
}

fn check_datatype(datatype: &str, facts: &NodeFacts<'_>) -> Result<(), String> {
    if facts.kind != FactKind::Literal {
        return Err(format!(
            "datatype <{datatype}> requires a literal, got {}",
            facts.describe()
        ));
    }
    let actual = facts.datatype.unwrap_or(XSD_STRING);
    if actual != datatype {
        return Err(format!("expected datatype <{datatype}>, got <{actual}>"));
    }
    if let Some(xsd) = XsdDatatype::from_iri(datatype) {
        if is_checked_datatype(xsd) {
            // shexTest v2.1.0 pins the XSD 1.0 float/double lexical space
            // (INF/-INF, not the XSD 1.1 "+INF" spelling), so validate with the
            // XSD-1.0-restricted parser rather than the 1.1 kernel default.
            purrdf_xsd::parse_xsd10(facts.lexical, xsd)
                .map_err(|e| format!("ill-formed <{datatype}> literal {:?}: {e}", facts.lexical))?;
        }
    }
    Ok(())
}

// ── string facets ───────────────────────────────────────────────────────────

fn check_string_facets(nc: &NodeConstraint, facts: &NodeFacts<'_>) -> Result<(), String> {
    let needs_lexical = nc.length.is_some()
        || nc.minlength.is_some()
        || nc.maxlength.is_some()
        || nc.pattern.is_some();
    if !needs_lexical {
        return Ok(());
    }
    if facts.kind == FactKind::Triple {
        return Err("string facets cannot apply to a triple term".to_owned());
    }
    // Facets count Unicode scalar values, not bytes (spec §5.4.5).
    let len = facts.lexical.chars().count() as u64;
    if let Some(length) = nc.length {
        if len != length {
            return Err(format!(
                "LENGTH {length} violated: {} has length {len}",
                facts.describe()
            ));
        }
    }
    if let Some(minlength) = nc.minlength {
        if len < minlength {
            return Err(format!(
                "MINLENGTH {minlength} violated: {} has length {len}",
                facts.describe()
            ));
        }
    }
    if let Some(maxlength) = nc.maxlength {
        if len > maxlength {
            return Err(format!(
                "MAXLENGTH {maxlength} violated: {} has length {len}",
                facts.describe()
            ));
        }
    }
    if let Some(pattern) = &nc.pattern {
        let re = compile_pattern(pattern, nc.flags.as_deref())?;
        if !re.is_match(facts.lexical) {
            return Err(format!(
                "pattern /{pattern}/{} does not match {}",
                nc.flags.as_deref().unwrap_or(""),
                facts.describe()
            ));
        }
    }
    Ok(())
}

// ── numeric facets ──────────────────────────────────────────────────────────

/// Parse the node into the XSD numeric value space, or explain why not.
fn numeric_value(facts: &NodeFacts<'_>) -> Result<XsdValue, String> {
    if facts.kind != FactKind::Literal {
        return Err(format!(
            "numeric facet requires a numeric literal, got {}",
            facts.describe()
        ));
    }
    let datatype = facts.datatype.unwrap_or(XSD_STRING);
    let Some(xsd) = XsdDatatype::from_iri(datatype) else {
        return Err(format!(
            "numeric facet requires a numeric datatype, got <{datatype}>"
        ));
    };
    use XsdDatatype as D;
    let numeric = matches!(
        xsd,
        D::Integer
            | D::Long
            | D::Int
            | D::Short
            | D::Byte
            | D::UnsignedLong
            | D::UnsignedInt
            | D::UnsignedShort
            | D::UnsignedByte
            | D::NonNegativeInteger
            | D::PositiveInteger
            | D::NonPositiveInteger
            | D::NegativeInteger
            | D::Decimal
            | D::Float
            | D::Double
    );
    if !numeric {
        return Err(format!(
            "numeric facet requires a numeric datatype, got <{datatype}>"
        ));
    }
    purrdf_xsd::parse(facts.lexical, xsd)
        .map_err(|e| format!("ill-formed numeric literal {:?}: {e}", facts.lexical))
}

/// The facet bound as an XSD value (SPARQL numeric promotion applies in
/// `numeric_cmp`).
fn facet_value(bound: NumericLiteral) -> XsdValue {
    match bound {
        NumericLiteral::Integer(i) => XsdValue::Integer {
            value: i128::from(i),
            datatype: XsdDatatype::Integer,
        },
        NumericLiteral::Fractional(f) => XsdValue::Double(f),
    }
}

fn check_numeric_facets(nc: &NodeConstraint, facts: &NodeFacts<'_>) -> Result<(), String> {
    use core::cmp::Ordering;
    let comparisons: [(&str, Option<NumericLiteral>, &[Ordering]); 4] = [
        (
            "MININCLUSIVE",
            nc.mininclusive,
            &[Ordering::Greater, Ordering::Equal],
        ),
        ("MINEXCLUSIVE", nc.minexclusive, &[Ordering::Greater]),
        (
            "MAXINCLUSIVE",
            nc.maxinclusive,
            &[Ordering::Less, Ordering::Equal],
        ),
        ("MAXEXCLUSIVE", nc.maxexclusive, &[Ordering::Less]),
    ];
    let any_range = comparisons.iter().any(|(_, bound, _)| bound.is_some());
    let any_digits = nc.totaldigits.is_some() || nc.fractiondigits.is_some();
    if !any_range && !any_digits {
        return Ok(());
    }
    let value = numeric_value(facts)?;
    for (name, bound, allowed) in comparisons {
        if let Some(bound) = bound {
            let facet = facet_value(bound);
            let Some(ordering) = value_cmp(&value, &facet) else {
                return Err(format!(
                    "{name} comparison with {} failed",
                    facts.describe()
                ));
            };
            if !allowed.contains(&ordering) {
                return Err(format!("{name} violated by {}", facts.describe()));
            }
        }
    }
    if any_digits {
        let (total, fraction) = decimal_digits(&value).ok_or_else(|| {
            format!(
                "TOTALDIGITS/FRACTIONDIGITS require a decimal-derived value, got {}",
                facts.describe()
            )
        })?;
        if let Some(limit) = nc.totaldigits {
            if total > limit {
                return Err(format!(
                    "TOTALDIGITS {limit} violated: {} has {total} digits",
                    facts.describe()
                ));
            }
        }
        if let Some(limit) = nc.fractiondigits {
            if fraction > limit {
                return Err(format!(
                    "FRACTIONDIGITS {limit} violated: {} has {fraction} fraction digits",
                    facts.describe()
                ));
            }
        }
    }
    Ok(())
}

/// `(total, fraction)` digit counts of the canonical decimal representation,
/// or `None` when the value is not decimal-derived (spec §5.4.5: the digit
/// facets fail on float/double).
fn decimal_digits(value: &XsdValue) -> Option<(u64, u64)> {
    let canonical = match value {
        XsdValue::Integer { value, .. } => value.unsigned_abs().to_string(),
        XsdValue::Decimal(d) => d.canonical_lexical(),
        _ => return None,
    };
    let unsigned = canonical.trim_start_matches('-');
    let (int_part, frac_part) = match unsigned.split_once('.') {
        Some((i, f)) => (i, f.trim_end_matches('0')),
        None => (unsigned, ""),
    };
    let int_digits = int_part.trim_start_matches('0').len() as u64;
    let frac_digits = frac_part.len() as u64;
    Some((int_digits + frac_digits, frac_digits))
}

// ── value sets ──────────────────────────────────────────────────────────────

fn check_value_set(values: &[ValueSetValue], facts: &NodeFacts<'_>) -> Result<(), String> {
    if values.iter().any(|v| value_matches(v, facts)) {
        Ok(())
    } else {
        Err(format!("{} is not in the value set", facts.describe()))
    }
}

fn value_matches(value: &ValueSetValue, facts: &NodeFacts<'_>) -> bool {
    match value {
        ValueSetValue::Iri(iri) => facts.kind == FactKind::Iri && facts.lexical == iri,
        ValueSetValue::Literal(literal) => literal_matches(literal, facts),
        ValueSetValue::IriStem { stem } => {
            facts.kind == FactKind::Iri && facts.lexical.starts_with(stem)
        }
        ValueSetValue::IriStemRange { stem, exclusions } => {
            facts.kind == FactKind::Iri
                && stem_matches(stem, facts.lexical)
                && !exclusions.iter().any(|e| match e {
                    IriExclusion::Iri(iri) => facts.lexical == iri,
                    IriExclusion::Stem(prefix) => facts.lexical.starts_with(prefix),
                })
        }
        ValueSetValue::LiteralStem { stem } => {
            facts.kind == FactKind::Literal && facts.lexical.starts_with(stem)
        }
        ValueSetValue::LiteralStemRange { stem, exclusions } => {
            facts.kind == FactKind::Literal
                && stem_matches(stem, facts.lexical)
                && !exclusions.iter().any(|e| match e {
                    LiteralExclusion::Literal(lit) => facts.lexical == lit,
                    LiteralExclusion::Stem(prefix) => facts.lexical.starts_with(prefix),
                })
        }
        ValueSetValue::Language { language_tag } => facts
            .language
            .is_some_and(|tag| tag.eq_ignore_ascii_case(language_tag)),
        ValueSetValue::LanguageStem { stem } => facts
            .language
            .is_some_and(|tag| language_stem_matches(tag, stem)),
        ValueSetValue::LanguageStemRange { stem, exclusions } => {
            facts.language.is_some_and(|tag| {
                let base = match stem {
                    StemValue::Str(prefix) => language_stem_matches(tag, prefix),
                    StemValue::Wildcard => true,
                };
                base && !exclusions.iter().any(|e| match e {
                    LanguageExclusion::Language(l) => tag.eq_ignore_ascii_case(l),
                    LanguageExclusion::Stem(prefix) => language_stem_matches(tag, prefix),
                })
            })
        }
    }
}

fn stem_matches(stem: &StemValue, lexical: &str) -> bool {
    match stem {
        StemValue::Str(prefix) => lexical.starts_with(prefix),
        StemValue::Wildcard => true,
    }
}

/// RFC 4647 basic filtering: `fr` matches `fr` and `fr-BE` but not `frc`.
/// The empty stem (`@~`) matches every language-tagged literal.
fn language_stem_matches(tag: &str, stem: &str) -> bool {
    if stem.is_empty() {
        return true;
    }
    if tag.len() < stem.len() || !tag[..stem.len()].eq_ignore_ascii_case(stem) {
        return false;
    }
    tag.len() == stem.len() || tag.as_bytes()[stem.len()] == b'-'
}

/// ShEx value-set literal matching is by **term identity**: lexical form,
/// datatype IRI and (case-insensitive) language tag (spec §5.4.6).
fn literal_matches(literal: &ObjectLiteral, facts: &NodeFacts<'_>) -> bool {
    if facts.kind != FactKind::Literal || facts.lexical != literal.value {
        return false;
    }
    match (&literal.language, facts.language) {
        (Some(want), Some(have)) => have.eq_ignore_ascii_case(want),
        (None, None) => {
            let want = literal.datatype.as_deref().unwrap_or(XSD_STRING);
            facts.datatype.unwrap_or(XSD_STRING) == want
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn literal_facts<'a>(
        lexical: &'a str,
        datatype: &'a str,
        language: Option<&'a str>,
    ) -> NodeFacts<'a> {
        NodeFacts {
            kind: FactKind::Literal,
            lexical,
            datatype: Some(datatype),
            language,
        }
    }

    #[test]
    fn digit_counting_matches_the_suite() {
        let d = |s: &str| {
            purrdf_xsd::parse(s, XsdDatatype::Decimal)
                .map(|v| decimal_digits(&v).expect("decimal"))
                .expect("parse")
        };
        assert_eq!(d("1.2345"), (5, 4));
        assert_eq!(d("01.23450"), (5, 4));
        assert_eq!(d("1.234560"), (6, 5));
        assert_eq!(d("0.05"), (2, 2));
        let i = purrdf_xsd::parse("12345", XsdDatatype::Integer).expect("parse");
        assert_eq!(decimal_digits(&i), Some((5, 0)));
        assert_eq!(decimal_digits(&XsdValue::Double(1.5)), None);
    }

    #[test]
    fn language_stem_is_rfc4647_basic() {
        assert!(language_stem_matches("fr", "fr"));
        assert!(language_stem_matches("fr-be", "fr"));
        assert!(language_stem_matches("fr-BE", "FR"));
        assert!(!language_stem_matches("frc", "fr"));
        assert!(language_stem_matches("anything", ""));
    }

    #[test]
    fn value_set_literal_matching_is_lexical() {
        let set = vec![ValueSetValue::Literal(ObjectLiteral {
            value: "1".to_owned(),
            language: None,
            datatype: Some("http://www.w3.org/2001/XMLSchema#integer".to_owned()),
        })];
        let hit = literal_facts("1", "http://www.w3.org/2001/XMLSchema#integer", None);
        let miss = literal_facts("01", "http://www.w3.org/2001/XMLSchema#integer", None);
        assert!(check_value_set(&set, &hit).is_ok());
        assert!(check_value_set(&set, &miss).is_err());
    }

    #[test]
    fn ill_formed_checked_datatype_fails() {
        let facts = literal_facts("abc", "http://www.w3.org/2001/XMLSchema#integer", None);
        assert!(check_datatype("http://www.w3.org/2001/XMLSchema#integer", &facts).is_err());
        let ok = literal_facts("42", "http://www.w3.org/2001/XMLSchema#integer", None);
        assert!(check_datatype("http://www.w3.org/2001/XMLSchema#integer", &ok).is_ok());
    }

    #[test]
    fn float_double_pin_xsd_1_0_positive_infinity() {
        // shexTest v2.1.0 pins XSD 1.0: `INF` is a valid positive infinity but
        // the XSD 1.1 `+INF` spelling is not.
        for dt in [
            "http://www.w3.org/2001/XMLSchema#double",
            "http://www.w3.org/2001/XMLSchema#float",
        ] {
            let inf = literal_facts("INF", dt, None);
            assert!(check_datatype(dt, &inf).is_ok(), "INF valid for <{dt}>");
            let plus_inf = literal_facts("+INF", dt, None);
            assert!(
                check_datatype(dt, &plus_inf).is_err(),
                "+INF must be rejected for <{dt}>"
            );
        }
    }
}
