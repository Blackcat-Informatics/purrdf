// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Reads openEHR Operational Template (OPT) XML directly and lowers a
//! `C_DV_QUANTITY` magnitude interval constraint to SHACL Turtle.
//!
//! An OPT is the flattened, fully-expressed form of an ADL archetype: every
//! `ELEMENT` node carries a `node_id` (an at-code, e.g. `at0004`) and, when its
//! value is constrained to a `DV_QUANTITY`, a `magnitude` interval with four
//! boundary fields (`lower`, `upper`, `lower_included`, `upper_included`) plus
//! a sibling `units` string. [`read_magnitude_interval`] walks the OPT DOM to
//! find that interval for a given `node_id`, and [`lower_magnitude_to_shacl_ttl`]
//! turns the parsed interval into a SHACL property shape, choosing
//! `sh:minInclusive`/`sh:minExclusive` and `sh:maxInclusive`/`sh:maxExclusive`
//! according to the interval's own inclusivity flags rather than a hardcoded
//! assumption. The two responsibilities are kept separate: parsing never
//! decides SHACL vocabulary, and lowering never touches XML.
//!
//! OPT files reuse `lower_included`/`upper_included` field names inside many
//! unrelated interval blocks (`occurrences`, `existence`, `precision`, and
//! even a `DV_CODED_TEXT`-valued `ELEMENT` that happens to share an at-code
//! with a `DV_QUANTITY`-valued one elsewhere in the template). The reader
//! therefore does not grab the first interval it finds; it descends a fixed
//! structural path — `ELEMENT[node_id] → attributes[rm_attribute_name=value]
//! → children[xsi:type=C_DV_QUANTITY] → list → magnitude` — and hard-fails if
//! any step of that path is absent for the requested `node_id`.

use std::fmt;
use std::fmt::Write as _;

/// A parsed `C_DV_QUANTITY` magnitude interval, read verbatim from an OPT.
#[derive(Debug, Clone, PartialEq)]
pub struct MagnitudeInterval {
    /// The interval's lower bound.
    pub lower: f64,
    /// The interval's upper bound.
    pub upper: f64,
    /// Whether `lower` itself is admitted by the interval.
    pub lower_included: bool,
    /// Whether `upper` itself is admitted by the interval.
    pub upper_included: bool,
    /// The magnitude's unit string (e.g. `mm[Hg]`).
    pub units: String,
}

/// Failure reading or navigating an OPT document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptError(String);

impl fmt::Display for OptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "openEHR OPT read error: {}", self.0)
    }
}

impl std::error::Error for OptError {}

fn opt_err(msg: impl Into<String>) -> OptError {
    OptError(msg.into())
}

/// Finds the first element child named `name` (namespace-agnostic on the
/// local name, matching how the OPT's default+xsi namespaces are declared).
fn child_element<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
    name: &str,
) -> Option<roxmltree::Node<'a, 'input>> {
    node.children()
        .find(|c| c.is_element() && c.tag_name().name() == name)
}

fn xsi_type<'a>(node: roxmltree::Node<'a, '_>) -> Option<&'a str> {
    node.attributes()
        .find(|a| a.name() == "type")
        .map(|a| a.value())
}

fn element_text_f64(node: roxmltree::Node<'_, '_>, name: &str) -> Result<f64, OptError> {
    let child = child_element(node, name).ok_or_else(|| {
        opt_err(format!(
            "missing <{name}> under <{}>",
            node.tag_name().name()
        ))
    })?;
    let text = child
        .text()
        .ok_or_else(|| opt_err(format!("<{name}> has no text content")))?;
    text.trim()
        .parse::<f64>()
        .map_err(|e| opt_err(format!("<{name}> value {text:?} is not a number: {e}")))
}

fn element_text_bool(node: roxmltree::Node<'_, '_>, name: &str) -> Result<bool, OptError> {
    let child = child_element(node, name).ok_or_else(|| {
        opt_err(format!(
            "missing <{name}> under <{}>",
            node.tag_name().name()
        ))
    })?;
    let text = child
        .text()
        .ok_or_else(|| opt_err(format!("<{name}> has no text content")))?;
    match text.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(opt_err(format!(
            "<{name}> value {other:?} is not a boolean"
        ))),
    }
}

/// Reads the `C_DV_QUANTITY` magnitude interval for the `ELEMENT` whose
/// `<node_id>` text equals `node_id`.
///
/// Navigates: the `ELEMENT` (`C_COMPLEX_OBJECT`) with a direct `<node_id>`
/// child matching `node_id` → its `<attributes xsi:type="C_SINGLE_ATTRIBUTE">`
/// with `<rm_attribute_name>value</rm_attribute_name>` → that attribute's
/// `<children xsi:type="C_DV_QUANTITY">` → `<list>` → `<magnitude>`, reading
/// `lower_included`, `upper_included`, `lower`, `upper`, and the sibling
/// `<units>` under `<list>`.
///
/// An OPT may contain multiple `ELEMENT`s sharing the same `node_id` at
/// different archetype-slot expansions, and only some of those are
/// `DV_QUANTITY`-valued (others may be `DV_CODED_TEXT` or other RM types).
/// This function scans every `ELEMENT` with the requested `node_id` and
/// returns the first one whose value is a `C_DV_QUANTITY`; if none match,
/// it hard-fails rather than defaulting.
pub fn read_magnitude_interval(
    opt_xml: &str,
    node_id: &str,
) -> Result<MagnitudeInterval, OptError> {
    let doc = roxmltree::Document::parse(opt_xml)
        .map_err(|e| opt_err(format!("XML parse failure: {e}")))?;

    let candidate_elements = doc.descendants().filter(|n| {
        n.is_element()
            && n.tag_name().name() == "children"
            && xsi_type(*n) == Some("C_COMPLEX_OBJECT")
            && child_element(*n, "node_id").and_then(|nid| nid.text()) == Some(node_id)
    });

    for element in candidate_elements {
        let attributes = match child_element(element, "attributes") {
            Some(a) if xsi_type(a) == Some("C_SINGLE_ATTRIBUTE") => a,
            _ => continue,
        };
        let rm_attribute_name =
            child_element(attributes, "rm_attribute_name").and_then(|n| n.text());
        if rm_attribute_name != Some("value") {
            continue;
        }
        let value_children = match child_element(attributes, "children") {
            Some(c) if xsi_type(c) == Some("C_DV_QUANTITY") => c,
            _ => continue,
        };
        let list = child_element(value_children, "list").ok_or_else(|| {
            opt_err(format!(
                "node_id {node_id:?}: C_DV_QUANTITY has no <list> child"
            ))
        })?;
        let magnitude = child_element(list, "magnitude").ok_or_else(|| {
            opt_err(format!(
                "node_id {node_id:?}: <list> has no <magnitude> child"
            ))
        })?;
        let units_node = child_element(list, "units")
            .ok_or_else(|| opt_err(format!("node_id {node_id:?}: <list> has no <units> child")))?;
        let units = units_node
            .text()
            .ok_or_else(|| opt_err(format!("node_id {node_id:?}: <units> has no text content")))?
            .to_string();

        return Ok(MagnitudeInterval {
            lower: element_text_f64(magnitude, "lower")?,
            upper: element_text_f64(magnitude, "upper")?,
            lower_included: element_text_bool(magnitude, "lower_included")?,
            upper_included: element_text_bool(magnitude, "upper_included")?,
            units,
        });
    }

    Err(opt_err(format!(
        "no C_DV_QUANTITY-valued ELEMENT with node_id {node_id:?} found in OPT"
    )))
}

/// Formats a bound as an integer literal when it is a whole number, else as
/// a plain decimal — matching how SHACL numeric literals are conventionally
/// written in the ontology's Turtle sources.
fn format_bound(value: f64) -> String {
    if value.fract() == 0.0 && value.is_finite() {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

/// Lowers a parsed [`MagnitudeInterval`] to a SHACL `sh:NodeShape` in Turtle,
/// targeting `target_class` and constraining `path_predicate` under the
/// property shape identified by `shape_iri`.
///
/// `target_class` / `path_predicate` / `shape_iri` may be prefixed names; the
/// caller supplies every `(prefix, namespace)` binding they rely on via
/// `prefixes` — PurRDF mints no vocabulary namespace of its own, so nothing
/// beyond the W3C `sh:` prefix is declared by default.
///
/// Inclusivity maps directly onto the corresponding SHACL constraint
/// component: `lower_included` selects `sh:minInclusive` (true) or
/// `sh:minExclusive` (false); `upper_included` selects `sh:maxInclusive`
/// (true) or `sh:maxExclusive` (false).
pub fn lower_magnitude_to_shacl_ttl(
    interval: &MagnitudeInterval,
    target_class: &str,
    path_predicate: &str,
    shape_iri: &str,
    prefixes: &[(&str, &str)],
) -> String {
    let min_predicate = if interval.lower_included {
        "sh:minInclusive"
    } else {
        "sh:minExclusive"
    };
    let max_predicate = if interval.upper_included {
        "sh:maxInclusive"
    } else {
        "sh:maxExclusive"
    };

    let mut prefix_block = String::from("@prefix sh:    <http://www.w3.org/ns/shacl#> .\n");
    for (prefix, ns) in prefixes {
        let _ = writeln!(prefix_block, "@prefix {prefix}: <{ns}> .");
    }

    format!(
        "{prefix_block}\
         \n\
         {shape_iri} a sh:NodeShape ;\n\
         \x20   sh:targetClass {target_class} ;\n\
         \x20   sh:property [\n\
         \x20       sh:path {path_predicate} ;\n\
         \x20       {min_predicate} {lower} ;\n\
         \x20       {max_predicate} {upper} ;\n\
         \x20   ] .\n",
        lower = format_bound(interval.lower),
        upper = format_bound(interval.upper),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Caller-supplied prefix bindings for the CURIEs the tests pass in
    /// (PurRDF mints no vocabulary namespace of its own).
    const TEST_PREFIXES: &[(&str, &str)] = &[
        ("meta", "https://example.org/meta/"),
        ("ex", "https://purrdf.example/openehr/bp/"),
    ];

    fn interval(lower_included: bool, upper_included: bool) -> MagnitudeInterval {
        MagnitudeInterval {
            lower: 0.0,
            upper: 1000.0,
            lower_included,
            upper_included,
            units: "mm[Hg]".to_string(),
        }
    }

    #[test]
    fn lower_included_true_emits_min_inclusive() {
        let ttl = lower_magnitude_to_shacl_ttl(
            &interval(true, false),
            "meta:SystolicMeasurement",
            "meta:quantityValue",
            "ex:SystolicMeasurementShape",
            TEST_PREFIXES,
        );
        assert!(ttl.contains("sh:minInclusive 0"));
        assert!(!ttl.contains("sh:minExclusive"));
    }

    #[test]
    fn lower_included_false_emits_min_exclusive() {
        let ttl = lower_magnitude_to_shacl_ttl(
            &interval(false, false),
            "meta:SystolicMeasurement",
            "meta:quantityValue",
            "ex:SystolicMeasurementShape",
            TEST_PREFIXES,
        );
        assert!(ttl.contains("sh:minExclusive 0"));
        assert!(!ttl.contains("sh:minInclusive"));
    }

    #[test]
    fn upper_included_true_emits_max_inclusive() {
        let ttl = lower_magnitude_to_shacl_ttl(
            &interval(true, true),
            "meta:SystolicMeasurement",
            "meta:quantityValue",
            "ex:SystolicMeasurementShape",
            TEST_PREFIXES,
        );
        assert!(ttl.contains("sh:maxInclusive 1000"));
        assert!(!ttl.contains("sh:maxExclusive"));
    }

    #[test]
    fn upper_included_false_emits_max_exclusive() {
        let ttl = lower_magnitude_to_shacl_ttl(
            &interval(true, false),
            "meta:SystolicMeasurement",
            "meta:quantityValue",
            "ex:SystolicMeasurementShape",
            TEST_PREFIXES,
        );
        assert!(ttl.contains("sh:maxExclusive 1000"));
        assert!(!ttl.contains("sh:maxInclusive"));
    }

    #[test]
    fn missing_node_id_hard_fails() {
        let err = read_magnitude_interval("<template></template>", "at9999").unwrap_err();
        assert!(err.to_string().contains("at9999"));
    }
}
