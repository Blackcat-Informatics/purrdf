// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::collections::BTreeMap;
use std::fmt::{self, Write as _};

use sha2::{Digest, Sha256};

use super::{ProjectionError, ProjectionLimits};
use purrdf_lex::json::record::ToJson;

/// `value` as compact JSON, refused past the artifact byte limit.
///
/// The writer is [`purrdf_lex::json::write_compact`]: members in the order the
/// value holds them, numbers as their lexemes, strings in the one short-form
/// spelling.
pub(crate) fn canonical_json_bounded<T: ToJson + ?Sized>(
    value: &T,
    limits: ProjectionLimits,
    description: &str,
) -> Result<Vec<u8>, ProjectionError> {
    let bytes = purrdf_lex::json::write_compact(&value.to_json()).into_bytes();
    if bytes.len() > limits.max_artifact_bytes() {
        return Err(ProjectionError::limit(format!(
            "{description} exceeds the {}-byte artifact limit",
            limits.max_artifact_bytes()
        )));
    }
    Ok(bytes)
}

/// Validate a complete, unambiguous role map: every role of `roles` is bound,
/// no other role is, each binding passes `check`, and no two roles share a
/// binding.
///
/// Every caller-owned vocabulary of the projections (IRIs or JSON-LD compact
/// terms keyed by a closed role enum) is this one law; `vocabulary` names the
/// map in a refusal.
///
/// # Errors
///
/// Returns a configuration error for a missing or unsupported role, a binding
/// `check` refuses, or a binding shared by two roles.
pub(crate) fn validate_role_map<R: Copy + Ord + fmt::Debug>(
    map: &BTreeMap<R, String>,
    roles: &[R],
    vocabulary: &str,
    check: impl Fn(R, &str) -> Result<(), ProjectionError>,
) -> Result<(), ProjectionError> {
    for &role in roles {
        let binding = map.get(&role).ok_or_else(|| {
            ProjectionError::configuration(format!("{vocabulary} is missing role `{role:?}`"))
        })?;
        check(role, binding)?;
    }
    if map.len() != roles.len() {
        return Err(ProjectionError::configuration(format!(
            "{vocabulary} contains an unsupported role"
        )));
    }
    let mut inverse = BTreeMap::<&str, R>::new();
    for (&role, binding) in map {
        if let Some(previous) = inverse.insert(binding, role) {
            return Err(ProjectionError::configuration(format!(
                "{vocabulary} binds roles `{previous:?}` and `{role:?}` both to `{binding}`"
            )));
        }
    }
    Ok(())
}

/// Refuse a JSON-LD compact term that is empty, a keyword (`@`-prefixed), or
/// carries whitespace: none can name a term in a JSON-LD 1.1 context.
///
/// # Errors
///
/// Returns a configuration error naming `role` of `vocabulary`.
pub(crate) fn validate_compact_term(
    vocabulary: &str,
    role: impl fmt::Debug,
    term: &str,
) -> Result<(), ProjectionError> {
    if term.is_empty() || term.starts_with('@') || term.chars().any(char::is_whitespace) {
        return Err(ProjectionError::configuration(format!(
            "{vocabulary} role `{role:?}` has invalid compact term `{term}`"
        )));
    }
    Ok(())
}

/// Refuse a sorted slice in which two adjacent items have equal keys, with
/// the integrity error `refusal` builds.
///
/// A dataset view and a model hand their rows over sorted, so a repeat is
/// adjacent; this is the one scan every projection checks that with.
///
/// # Errors
///
/// Returns `refusal()` as an integrity error at the first adjacent repeat.
pub(crate) fn reject_duplicate_keys<T, K: PartialEq + ?Sized>(
    sorted: &[T],
    key: impl Fn(&T) -> &K,
    refusal: impl FnOnce() -> String,
) -> Result<(), ProjectionError> {
    if sorted.windows(2).any(|pair| key(&pair[0]) == key(&pair[1])) {
        return Err(ProjectionError::integrity(refusal()));
    }
    Ok(())
}

/// Refuse a sorted run of dataset-view rows that repeats a row.
///
/// # Errors
///
/// Returns an integrity error naming `description`.
pub(crate) fn reject_duplicates<T: PartialEq>(
    sorted: &[T],
    description: &str,
) -> Result<(), ProjectionError> {
    reject_duplicate_keys(
        sorted,
        |row| row,
        || format!("dataset view exposed duplicate {description}"),
    )
}

/// A running count of records against a configured maximum.
///
/// Every bounded projection reader and mapper counts the records it admits
/// through one of these, so the count cannot overflow silently and the
/// refusal names the bound the same way everywhere.
#[derive(Debug)]
pub(crate) struct RecordBudget {
    used: usize,
    maximum: usize,
    domain: &'static str,
}

impl RecordBudget {
    /// A budget of `maximum` records; `domain` names the bound (`"LPG"`).
    pub(crate) const fn new(maximum: usize, domain: &'static str) -> Self {
        Self {
            used: 0,
            maximum,
            domain,
        }
    }

    /// Records admitted so far.
    pub(crate) const fn used(&self) -> usize {
        self.used
    }

    /// Admit `amounts` more records, summed, for `description`.
    ///
    /// # Errors
    ///
    /// Returns a limit error when the sum overflows or the running count
    /// passes the maximum.
    pub(crate) fn consume(
        &mut self,
        amounts: &[usize],
        description: &str,
    ) -> Result<(), ProjectionError> {
        self.used = amounts
            .iter()
            .try_fold(self.used, |used, amount| used.checked_add(*amount))
            .ok_or_else(|| {
                ProjectionError::limit(format!("{} record count overflow", self.domain))
            })?;
        if self.used > self.maximum {
            return Err(ProjectionError::limit(format!(
                "{description} exceeds the {}-record {} limit",
                self.maximum, self.domain
            )));
        }
        Ok(())
    }
}

/// Field `index` of a CSV record read from the artifact at `path`.
///
/// # Errors
///
/// Returns a syntax error located at `path` when the record is shorter.
pub(crate) fn csv_field<'a>(
    record: &'a purrdf_core::csv::StringRecord,
    index: usize,
    path: &str,
) -> Result<&'a str, ProjectionError> {
    record.get(index).ok_or_else(|| {
        ProjectionError::syntax(format!("CSV row is missing field {index}")).at_path(path)
    })
}

/// The `path:line` location of the zero-based data record `record` of a CSV
/// artifact whose first line is its header: data record 0 is line 2.
pub(crate) fn csv_row_path(path: &str, record: usize) -> String {
    format!("{path}:{}", record + 2)
}

/// Declare a caller-owned role group: a record of mandatory absolute IRIs, one
/// per named semantic role, with no defaults.
///
/// The one declaration yields the struct, a constructor that refuses any
/// non-absolute IRI (naming the role and `vocabulary`), a documented accessor
/// per role, and the JSON codec — an object of the roles under their field
/// names, read back through the same constructor. Every projection vocabulary
/// of this shape is declared through here, so none can admit an IRI another
/// refuses.
macro_rules! iri_role_group {
    (
        $(#[$type_meta:meta])*
        $name:ident as $expecting:literal in $vocabulary:literal {
            $( $(#[$field_meta:meta])* $field:ident ),+ $(,)?
        }
    ) => {
        $(#[$type_meta])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name {
            $($field: String,)+
        }

        impl $name {
            /// Construct the role group from one absolute IRI per role.
            ///
            /// # Errors
            ///
            /// Returns a configuration error naming the first role whose IRI
            /// is not absolute.
            #[allow(
                clippy::too_many_arguments,
                reason = "named mandatory roles keep omission visible and forbid fabricated defaults"
            )]
            pub fn new($($field: impl Into<String>),+) -> Result<Self, $crate::projections::ProjectionError> {
                let roles = Self {
                    $($field: $field.into(),)+
                };
                $(
                    $crate::projections::validate_absolute_iri(
                        &roles.$field,
                        concat!($vocabulary, " vocabulary role `", stringify!($field), "`"),
                    )?;
                )+
                Ok(roles)
            }

            $(
                $(#[$field_meta])*
                pub fn $field(&self) -> &str {
                    &self.$field
                }
            )+

            /// Each role's name and IRI, in declaration order.
            pub(crate) fn named_iris(&self) -> impl Iterator<Item = (&'static str, &str)> {
                [$((stringify!($field), self.$field.as_str())),+].into_iter()
            }
        }

        purrdf_lex::json_record!($name as $expecting {
            $(stringify!($field) => $field: required::<String>,)+
        } => $name::new);
    };
}

pub(crate) use iri_role_group;

/// Refuse a vocabulary in which two semantic roles share one IRI: every role
/// must name a distinct term, or a projection could not tell them apart.
///
/// # Errors
///
/// Returns a configuration error naming the second role and the shared IRI.
pub(crate) fn validate_distinct_roles<'a>(
    vocabulary: &str,
    roles: impl IntoIterator<Item = (&'static str, &'a str)>,
) -> Result<(), ProjectionError> {
    let mut seen = std::collections::BTreeSet::new();
    for (role, iri) in roles {
        if !seen.insert(iri) {
            return Err(ProjectionError::configuration(format!(
                "{vocabulary} role `{role}` reuses `{iri}`; semantic roles must be distinct"
            )));
        }
    }
    Ok(())
}

/// Refuse a configured count bound of zero or one past the portable `u32`
/// ceiling every binding surface can carry.
///
/// # Errors
///
/// Returns a configuration error naming `field`.
pub(crate) fn validate_portable_bound(value: usize, field: &str) -> Result<(), ProjectionError> {
    if value == 0 {
        return Err(ProjectionError::configuration(format!(
            "{field} must be greater than zero"
        )));
    }
    if u32::try_from(value).is_err() {
        return Err(ProjectionError::configuration(format!(
            "{field} exceeds the portable u32 ceiling"
        )));
    }
    Ok(())
}

/// `value` as an owned absolute IRI.
///
/// # Errors
///
/// Returns [`validate_absolute_iri`]'s refusal naming `field`.
pub(crate) fn absolute_iri(
    value: impl Into<String>,
    field: &str,
) -> Result<String, ProjectionError> {
    let value = value.into();
    validate_absolute_iri(&value, field)?;
    Ok(value)
}

/// Refuse a slice that is not strictly ascending — out of order or holding a
/// repeat — with the integrity error `refusal` builds.
///
/// # Errors
///
/// Returns `refusal()` as an integrity error at the first pair out of order.
pub(crate) fn require_strictly_sorted<T: Ord>(
    values: &[T],
    refusal: impl FnOnce() -> String,
) -> Result<(), ProjectionError> {
    if values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ProjectionError::integrity(refusal()));
    }
    Ok(())
}

/// Push the all-IRI default-graph triple `subject predicate object`.
pub(crate) fn push_iri_triple(
    builder: &mut purrdf_core::RdfDatasetBuilder,
    subject: &str,
    predicate: &str,
    object: &str,
) {
    let subject = builder.intern_iri(subject);
    let predicate = builder.intern_iri(predicate);
    let object = builder.intern_iri(object);
    builder.push_quad(subject, predicate, object, None);
}

/// The entries of a path- or IRI-keyed byte map, borrowed, in key order.
pub(crate) fn byte_entries(
    map: &BTreeMap<String, Vec<u8>>,
) -> impl ExactSizeIterator<Item = (&str, &[u8])> {
    map.iter()
        .map(|(key, bytes)| (key.as_str(), bytes.as_slice()))
}

/// `value` after `validate` accepts it.
///
/// The one validate-then-return of every validating projection-config constructor;
/// each constructor keeps only its own field list and its own `validate`.
///
/// # Errors
///
/// Returns `validate`'s refusal.
pub(crate) fn validated<T>(
    value: T,
    validate: impl FnOnce(&T) -> Result<(), ProjectionError>,
) -> Result<T, ProjectionError> {
    validate(&value)?;
    Ok(value)
}

/// Build a stable collision-resistant identifier from a caller-owned ASCII prefix
/// and arbitrary key bytes.
///
/// The full SHA-256 digest is retained, so the helper never depends on iteration
/// order, random seeds, process identity, time, or a truncation collision policy.
///
/// # Errors
///
/// Returns a configuration error unless `prefix` starts with an ASCII letter and
/// otherwise contains only ASCII alphanumerics or `_`.
pub fn stable_identifier(prefix: &str, key: &[u8]) -> Result<String, ProjectionError> {
    let mut chars = prefix.chars();
    if !chars.next().is_some_and(|ch| ch.is_ascii_alphabetic())
        || !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return Err(ProjectionError::configuration(
            "identifier prefix must start with an ASCII letter and contain only ASCII alphanumerics or `_`",
        ));
    }
    let digest = Sha256::digest(key);
    let mut output = String::with_capacity(prefix.len() + 1 + digest.len() * 2);
    output.push_str(prefix);
    output.push('_');
    // Infallible: `String`'s `fmt::Write` never returns an error.
    let _ = write!(output, "{}", purrdf_hash::hex::Lower(&digest));
    Ok(output)
}

/// Validate a mandatory absolute IRI configuration field.
///
/// This is the single gate every IRI-valued projection configuration field passes through —
/// dataset identities, generated-resource bases, scheme IRIs, entity bases, document bases,
/// graph names, vocabulary role tables, predicates and datatypes alike — so what "absolute
/// IRI" means in a projection configuration is decided once rather than per profile.
///
/// It is [`purrdf_iri::BaseIri::parse`], the workspace's shared "a valid RFC-3987 IRI that
/// has a scheme" primitive, and the failure carries
/// [`purrdf_iri::IriError::diagnostic_code`] — the workspace's single owner of those
/// spellings. It used to be `purrdf_sparql_algebra::NamedNode::new`, which reaches the same
/// grammar but wraps it in a private reason of its own ("relative IRI reference in term
/// position"), so a configuration field failed with a SPARQL term-position sentence and no
/// shared code for a consumer to switch on.
///
/// The relative case gets a remedy written for the surface the value came from. The shared
/// `iri-non-absolute-base` remedy names a BASE ("supply a base IRI that has a scheme"), and
/// most fields checked here are not bases; and a configuration document is not an RDF
/// document, so the `@base`/`xml:base` remedy could not be applied to it either. Naming a fix
/// the caller cannot apply is worse than naming none.
///
/// # Errors
///
/// Returns a configuration error naming `field` when `value` is not an absolute IRI.
pub fn validate_absolute_iri(value: &str, field: &str) -> Result<(), ProjectionError> {
    let Err(error) = purrdf_iri::BaseIri::parse(value) else {
        return Ok(());
    };
    let code = error.diagnostic_code();
    if code == "iri-non-absolute-base" {
        return Err(ProjectionError::configuration(format!(
            "{field} must be an absolute IRI: {code}: `{value}` is a relative IRI reference. A \
             projection configuration is not an RDF document and has no base of its own, so \
             there is nothing to resolve it against: write the value in absolute form, with a \
             scheme"
        )));
    }
    Err(ProjectionError::configuration(format!(
        "{field} must be an absolute IRI: {code}: {error}"
    )))
}

/// Escape an openCypher backtick-delimited identifier body.
pub fn escape_cypher_identifier(value: &str) -> String {
    value.replace('`', "``")
}

/// Escape an openCypher single-quoted string body.
pub fn escape_cypher_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => output.push_str("\\\\"),
            '\'' => output.push_str("\\'"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            control if control.is_control() => {
                let _ = write!(output, "\\u{:04x}", control as u32);
            }
            other => output.push(other),
        }
    }
    output
}

/// Escape XML 1.0 character-data text.
///
/// # Errors
///
/// Returns a term error when `value` contains a character forbidden by XML 1.0.
pub fn escape_xml_text(value: &str) -> Result<String, ProjectionError> {
    escape_xml(value, false)
}

/// Escape a double-quoted XML 1.0 attribute value.
///
/// # Errors
///
/// Returns a term error when `value` contains a character forbidden by XML 1.0.
pub fn escape_xml_attribute(value: &str) -> Result<String, ProjectionError> {
    escape_xml(value, true)
}

fn escape_xml(value: &str, attribute: bool) -> Result<String, ProjectionError> {
    let context = if attribute {
        purrdf_core::xml_escape::Context::Attribute
    } else {
        purrdf_core::xml_escape::Context::Text
    };
    purrdf_core::xml_escape::escape(value, context)
        .map(std::borrow::Cow::into_owned)
        .map_err(|error| ProjectionError::term(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_lex::json::record::{FromJson, ToJson};

    fn read_roles(
        text: &str,
    ) -> Result<crate::projections::SkosClassRoles, purrdf_lex::json::record::DecodeError> {
        crate::projections::SkosClassRoles::from_json(&purrdf_lex::json::read(text).expect("JSON"))
    }

    #[test]
    fn a_role_group_reads_through_its_constructor() {
        let text = r#"{"rdf_type":"https://example.org/type","concept":"https://example.org/C","concept_scheme":"https://example.org/S"}"#;
        let roles = read_roles(text).expect("absolute roles read");
        assert_eq!(roles.concept(), "https://example.org/C");
        assert_eq!(purrdf_lex::json::write_compact(&roles.to_json()), text);
        assert_eq!(
            roles.named_iris().map(|(role, _)| role).collect::<Vec<_>>(),
            ["rdf_type", "concept", "concept_scheme"]
        );
        let relative = read_roles(&text.replace("https://example.org/C", "C"))
            .expect_err("a relative role IRI");
        assert!(
            relative
                .to_string()
                .contains("SKOS vocabulary role `concept`"),
            "{relative}"
        );
        assert!(read_roles(r#"{"rdf_type":"https://example.org/type"}"#).is_err());
    }

    #[test]
    fn distinct_roles_refuse_a_shared_iri_and_admit_distinct_ones() {
        assert!(
            validate_distinct_roles(
                "V",
                [
                    ("a", "https://example.org/x"),
                    ("b", "https://example.org/y")
                ]
            )
            .is_ok()
        );
        let error = validate_distinct_roles(
            "V",
            [
                ("a", "https://example.org/x"),
                ("b", "https://example.org/x"),
            ],
        )
        .expect_err("a shared IRI");
        assert!(error.to_string().contains("V role `b` reuses"), "{error}");
    }

    #[test]
    fn a_role_map_is_complete_unambiguous_and_checked() {
        let roles = ["a", "b"];
        let map = |pairs: &[(&'static str, &str)]| -> BTreeMap<&'static str, String> {
            pairs
                .iter()
                .map(|(role, term)| (*role, (*term).to_owned()))
                .collect()
        };
        let check = |role: &str, term: &str| validate_compact_term("V", role, term);
        assert!(validate_role_map(&map(&[("a", "x"), ("b", "y")]), &roles, "V", check).is_ok());
        assert!(validate_role_map(&map(&[("a", "x")]), &roles, "V", check).is_err());
        assert!(validate_role_map(&map(&[("a", "x"), ("b", "x")]), &roles, "V", check).is_err());
        assert!(validate_role_map(&map(&[("a", "x"), ("b", "@y")]), &roles, "V", check).is_err());
        assert!(
            validate_role_map(
                &map(&[("a", "x"), ("b", "y"), ("c", "z")]),
                &roles,
                "V",
                check
            )
            .is_err()
        );
    }

    #[test]
    fn a_portable_bound_is_positive_and_fits_u32() {
        assert!(validate_portable_bound(1, "n").is_ok());
        assert!(validate_portable_bound(u32::MAX as usize, "n").is_ok());
        assert!(validate_portable_bound(0, "n").is_err());
        #[cfg(target_pointer_width = "64")]
        assert!(validate_portable_bound(u32::MAX as usize + 1, "n").is_err());
    }

    #[test]
    fn a_record_budget_refuses_past_its_maximum_and_admits_up_to_it() {
        let mut budget = RecordBudget::new(3, "Example");
        assert!(budget.consume(&[1, 2], "rows").is_ok());
        assert_eq!(budget.used(), 3);
        let error = budget.consume(&[1], "rows").expect_err("past the maximum");
        assert!(
            error
                .to_string()
                .contains("exceeds the 3-record Example limit"),
            "{error}"
        );
        let mut overflow = RecordBudget::new(usize::MAX, "Example");
        assert!(overflow.consume(&[usize::MAX, 1], "rows").is_err());
    }

    #[test]
    fn duplicate_and_order_scans_refuse_repeats_and_admit_strict_order() {
        assert!(reject_duplicates(&[1, 2, 3], "rows").is_ok());
        assert!(reject_duplicates(&[1, 2, 2], "rows").is_err());
        assert!(require_strictly_sorted(&[1, 2, 3], || "unordered".to_owned()).is_ok());
        assert!(require_strictly_sorted(&[1, 3, 2], || "unordered".to_owned()).is_err());
        assert!(require_strictly_sorted(&[1, 1], || "unordered".to_owned()).is_err());
        assert_eq!(
            absolute_iri("https://example.org/a", "f").as_deref(),
            Ok("https://example.org/a")
        );
        assert!(absolute_iri("a", "f").is_err());
    }

    #[test]
    fn stable_id_is_full_digest_and_repeatable() {
        let first = stable_identifier("node", b"http://example.org/a").expect("identifier");
        let second = stable_identifier("node", b"http://example.org/a").expect("identifier");
        assert_eq!(first, second);
        assert_eq!(first.len(), "node_".len() + 64);
        assert!(stable_identifier("bad-prefix", b"x").is_err());
    }

    #[test]
    fn cypher_escaping_is_injection_safe() {
        assert_eq!(escape_cypher_identifier("a`b"), "a``b");
        assert_eq!(escape_cypher_string("a'\\\nb"), "a\\'\\\\\\nb");
    }

    #[test]
    fn xml_text_and_attribute_escaping_are_distinct() {
        assert_eq!(escape_xml_text("<&>\"'").expect("text"), "&lt;&amp;&gt;\"'");
        assert_eq!(
            escape_xml_attribute("<&>\"'").expect("attribute"),
            "&lt;&amp;&gt;&quot;'"
        );
        assert!(escape_xml_text("bad\0value").is_err());
    }

    /// Every failure names the FIELD and carries the shared `purrdf_iri` diagnostic code, so
    /// a projection configuration failure groups with every other IRI failure in the
    /// workspace instead of spelling a private reason of its own.
    #[test]
    fn absolute_iri_validation_fails_closed() {
        // A fragment is part of an absolute IRI and must survive: nearly every vocabulary
        // role in these profiles is a `…#term`.
        assert!(validate_absolute_iri("http://example.org/p", "predicate").is_ok());
        assert!(
            validate_absolute_iri(
                "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                "predicate"
            )
            .is_ok()
        );

        // RELATIVE: the shared code, the field, the value, and a remedy that fits a
        // configuration document rather than an RDF one.
        let relative = validate_absolute_iri("relative", "predicate").expect_err("relative");
        let text = relative.to_string();
        assert!(text.contains("iri-non-absolute-base"), "{text}");
        assert!(
            text.contains("predicate") && text.contains("`relative`"),
            "{text}"
        );
        assert!(text.contains("write the value in absolute form"), "{text}");
        assert!(
            !text.contains("@base") && !text.contains("term position"),
            "no remedy the caller cannot apply, and no SPARQL term-position sentence: {text}"
        );

        // MALFORMED: the specific shared code for what is wrong with it.
        let malformed = validate_absolute_iri("ht tp://example.org/p", "predicate")
            .expect_err("malformed scheme");
        assert!(
            malformed.to_string().contains("iri-bad-scheme"),
            "{malformed}"
        );

        // The EMPTY string gets its own shared code rather than being folded into the
        // relative case: `iri-empty` says which of the two a caller actually wrote.
        let empty = validate_absolute_iri("", "predicate").expect_err("empty");
        assert!(empty.to_string().contains("iri-empty"), "{empty}");
    }
}
