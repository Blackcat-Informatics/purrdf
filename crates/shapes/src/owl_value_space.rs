// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The value spaces of the OWL 2 datatype map, as an ontology's developer
//! schema judges them.
//!
//! OWL reads a datatype by its value space (OWL 2 Structural Specification
//! §4): `"7"^^xsd:nonNegativeInteger` is an `xsd:decimal`, and
//! `"a"^^xsd:token` an `xsd:string`, although neither is tagged so. SHACL's
//! `sh:datatype` compares the tag instead, which is right for a shape and wrong
//! for an OWL range or filler. This module says, for one datatype, which
//! tagged literals have their values in its value space: every datatype of the
//! map whose value space meets it, each with the bounds or lexical pattern
//! that keeps its literals inside. The schema compiler states each of those as
//! a value schema; the class-expression manifest reads whether that statement
//! is exact.

use purrdf_xsd::datatype::XSD_NS;

/// The four whitespace code points XSD's `collapse` facet trims.
const WS: &str = "[\\t\\n\\r ]*";

/// The datatypes with lexical forms whose values are real numbers, as
/// `(local name, least value, greatest value, integral)`: `xsd:decimal`,
/// `xsd:integer` and the twelve datatypes XSD derives from it (OWL 2 §4.1).
const NUMERIC: [(&str, Option<i128>, Option<i128>, bool); 14] = [
    ("decimal", None, None, false),
    ("integer", None, None, true),
    ("nonPositiveInteger", None, Some(0), true),
    ("negativeInteger", None, Some(-1), true),
    (
        "long",
        Some(-9_223_372_036_854_775_808),
        Some(9_223_372_036_854_775_807),
        true,
    ),
    ("int", Some(-2_147_483_648), Some(2_147_483_647), true),
    ("short", Some(-32_768), Some(32_767), true),
    ("byte", Some(-128), Some(127), true),
    ("nonNegativeInteger", Some(0), None, true),
    (
        "unsignedLong",
        Some(0),
        Some(18_446_744_073_709_551_615),
        true,
    ),
    ("unsignedInt", Some(0), Some(4_294_967_295), true),
    ("unsignedShort", Some(0), Some(65_535), true),
    ("unsignedByte", Some(0), Some(255), true),
    ("positiveInteger", Some(1), None, true),
];

/// The string datatypes of the OWL 2 datatype map (OWL 2 §4.3), each with
/// the datatypes whose value spaces lie within its own. An RDF literal's
/// lexical form must lie in its datatype's lexical space, which for each of
/// these is its value space (XSD 1.1 Part 2 §3.4.2–§3.4.8: an
/// `xsd:normalizedString` holds no tab or line break, an `xsd:token` no stray
/// space), so a literal's value is its lexical form.
const STRINGS: [(&str, &[&str]); 7] = [
    (
        "string",
        &[
            "string",
            "normalizedString",
            "token",
            "language",
            "Name",
            "NCName",
            "NMTOKEN",
        ],
    ),
    (
        "normalizedString",
        &[
            "normalizedString",
            "token",
            "language",
            "Name",
            "NCName",
            "NMTOKEN",
        ],
    ),
    ("token", &["token", "language", "Name", "NCName", "NMTOKEN"]),
    ("language", &["language"]),
    ("Name", &["Name", "NCName"]),
    ("NCName", &["NCName"]),
    ("NMTOKEN", &["NMTOKEN"]),
];

/// XML 1.0 (Fifth Edition) `NameStartChar`, without `:`.
const NC_NAME_START: &str = "A-Z_a-z\u{C0}-\u{D6}\u{D8}-\u{F6}\u{F8}-\u{2FF}\u{370}-\u{37D}\
\u{37F}-\u{1FFF}\u{200C}-\u{200D}\u{2070}-\u{218F}\u{2C00}-\u{2FEF}\u{3001}-\u{D7FF}\
\u{F900}-\u{FDCF}\u{FDF0}-\u{FFFD}\u{10000}-\u{EFFFF}";
/// The characters XML 1.0 `NameChar` adds to `NameStartChar`.
const NAME_REST: &str = "\\-.0-9\u{B7}\u{300}-\u{36F}\u{203F}-\u{2040}";

/// A pattern over a whole value of the string datatype `local`, or `None`
/// where every string value of it is (`xsd:string`).
fn string_value_pattern(local: &str) -> Option<String> {
    Some(match local {
        "normalizedString" => "[^\\t\\n\\r]*".to_owned(),
        "token" => "([^\\t\\n\\r ]+( [^\\t\\n\\r ]+)*)?".to_owned(),
        "language" => "[a-zA-Z]{1,8}(-[a-zA-Z0-9]{1,8})*".to_owned(),
        "Name" => format!("[:{NC_NAME_START}][:{NC_NAME_START}{NAME_REST}]*"),
        "NCName" => format!("[{NC_NAME_START}][{NC_NAME_START}{NAME_REST}]*"),
        "NMTOKEN" => format!("[:{NC_NAME_START}{NAME_REST}]+"),
        _ => return None,
    })
}

/// One datatype whose literals may have their values in a target datatype's
/// value space, with what keeps them there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Member {
    /// The member datatype's IRI.
    pub(crate) datatype: String,
    /// A least value to require, where the target's is above the member's.
    pub(crate) min: Option<i128>,
    /// A greatest value to require, where the target's is below the member's.
    pub(crate) max: Option<i128>,
    /// A pattern the member's lexical form must match for its value to be in
    /// the target's value space.
    pub(crate) pattern: Option<String>,
}

impl Member {
    fn of(local: &str) -> Self {
        Self {
            datatype: format!("{XSD_NS}{local}"),
            min: None,
            max: None,
            pattern: None,
        }
    }
}

/// The tagged literals whose values lie in one datatype's value space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValueSpace {
    pub(crate) members: Vec<Member>,
    /// Whether literals typed `owl:rational` are admitted without judging
    /// their value: a rational's lexical form `n/d` is in an integer or
    /// decimal value space exactly when `d` divides `n` (or `n/d` has a finite
    /// decimal expansion), which no pattern decides.
    pub(crate) unjudged_rationals: bool,
}

/// The value space of `datatype` as the tagged literals that have their
/// values in it, or `None` where its own tag is the only one (a datatype no
/// other datatype of the map shares values with, a declared datatype, or
/// `owl:real`/`owl:rational`, which the compiler projects apart).
pub(crate) fn value_space(datatype: &str) -> Option<ValueSpace> {
    let local = datatype.strip_prefix(XSD_NS)?;
    if let Some(&(_, low, high, integral)) = NUMERIC.iter().find(|(name, ..)| *name == local) {
        let mut members = Vec::new();
        for &(member, member_low, member_high, member_integral) in &NUMERIC {
            let least = max_bound(low, member_low);
            let greatest = min_bound(high, member_high);
            if least
                .zip(greatest)
                .is_some_and(|(least, greatest)| least > greatest)
            {
                continue;
            }
            let mut entry = Member::of(member);
            entry.min = low.filter(|low| member_low.is_none_or(|member_low| member_low < *low));
            entry.max =
                high.filter(|high| member_high.is_none_or(|member_high| member_high > *high));
            if integral && !member_integral {
                // A decimal is integral when its fraction is all zeros.
                entry.pattern = Some(format!("^{WS}[+\\-]?([0-9]+(\\.0*)?|\\.0+){WS}$"));
            }
            members.push(entry);
        }
        return Some(ValueSpace {
            members,
            unjudged_rationals: true,
        });
    }
    if let Some(&(_, within)) = STRINGS.iter().find(|(name, _)| *name == local) {
        let value = string_value_pattern(local);
        let mut members = Vec::new();
        for &(member, _) in &STRINGS {
            let mut entry = Member::of(member);
            if !within.contains(&member) {
                // A wider member's literal is in the value space when its
                // lexical form, which is its value, matches the target's.
                entry.pattern = Some(format!("^{}$", value.as_deref()?));
            }
            members.push(entry);
        }
        return Some(ValueSpace {
            members,
            unjudged_rationals: false,
        });
    }
    match local {
        "dateTime" => Some(ValueSpace {
            members: vec![Member::of("dateTime"), Member::of("dateTimeStamp")],
            unjudged_rationals: false,
        }),
        "dateTimeStamp" => {
            let mut stamped = Member::of("dateTime");
            stamped.pattern = Some(format!("^.*(Z|[+\\-][0-9]{{2}}:[0-9]{{2}}){WS}$"));
            Some(ValueSpace {
                members: vec![Member::of("dateTimeStamp"), stamped],
                unjudged_rationals: false,
            })
        }
        _ => None,
    }
}

fn max_bound(left: Option<i128>, right: Option<i128>) -> Option<i128> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (bound, None) | (None, bound) => bound,
    }
}

fn min_bound(left: Option<i128>, right: Option<i128>) -> Option<i128> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (bound, None) | (None, bound) => bound,
    }
}

/// Whether `local` names one of the map's string datatypes.
pub(crate) fn is_string_datatype(local: &str) -> bool {
    STRINGS.iter().any(|(name, _)| *name == local)
}

/// The string datatypes of the map, by local name.
pub(crate) fn string_datatypes() -> impl Iterator<Item = &'static str> {
    STRINGS.iter().map(|(name, _)| *name)
}

/// The numeric datatypes of the map with lexical forms, by local name, with
/// their least and greatest values and whether they are integral.
pub(crate) fn numeric_datatypes()
-> impl Iterator<Item = (&'static str, Option<i128>, Option<i128>, bool)> {
    NUMERIC.iter().copied()
}

/// Whether a literal value equal to one of `datatype`'s can be recognised in
/// every literal that denotes it: true for strings and booleans, whose
/// lexical forms a pattern compares, and for `xsd:double`/`xsd:float`, whose
/// values the compiler compares; false for the real numbers, which a literal
/// typed `owl:rational` may also denote, and for every other datatype, whose
/// equal values (a `dateTime` in another time zone, say) no pattern finds.
pub(crate) fn equality_is_exact(datatype: &str) -> bool {
    let Some(local) = datatype.strip_prefix(XSD_NS) else {
        return false;
    };
    is_string_datatype(local) || matches!(local, "boolean" | "double" | "float")
}

/// Whether `datatype` is one of the map's real-number datatypes with lexical
/// forms (`xsd:decimal` and the integers).
pub(crate) fn is_numeric(datatype: &str) -> bool {
    datatype
        .strip_prefix(XSD_NS)
        .is_some_and(|local| NUMERIC.iter().any(|(name, ..)| *name == local))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_admits_every_integer_datatype_and_integer_an_integral_decimal() {
        let decimal = value_space(&format!("{XSD_NS}decimal")).expect("decimal");
        assert_eq!(decimal.members.len(), NUMERIC.len());
        assert!(
            decimal
                .members
                .iter()
                .all(|member| member.pattern.is_none())
        );
        let non_negative = value_space(&format!("{XSD_NS}nonNegativeInteger")).expect("nni");
        let int = non_negative
            .members
            .iter()
            .find(|member| member.datatype.ends_with("#int"))
            .expect("int overlaps the non-negative integers");
        assert_eq!((int.min, int.max), (Some(0), None));
        assert!(
            !non_negative
                .members
                .iter()
                .any(|member| member.datatype.ends_with("#negativeInteger")),
            "no negative integer is non-negative"
        );
        let decimal_member = non_negative
            .members
            .iter()
            .find(|member| member.datatype.ends_with("#decimal"))
            .expect("an integral decimal is an integer");
        assert!(decimal_member.pattern.is_some());
    }

    #[test]
    fn a_wider_string_literal_is_in_a_narrower_value_space_by_its_lexical_form() {
        let token = value_space(&format!("{XSD_NS}token")).expect("token");
        let string = token
            .members
            .iter()
            .find(|member| member.datatype.ends_with("#string"))
            .expect("a string without stray spaces is a token");
        assert!(string.pattern.is_some());
        assert!(
            token
                .members
                .iter()
                .find(|member| member.datatype.ends_with("#language"))
                .is_some_and(|member| member.pattern.is_none())
        );
    }
}
