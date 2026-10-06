// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Strict portable inventory/catalog records. Callers acquire referenced bytes.

use std::collections::BTreeSet;

use purrdf_lex::json::record::{DecodeError, FromJson, ToJson};
use purrdf_lex::json::{self, Value};

/// One parsed input artifact and its optional graph/media-type routing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    /// Path relative to the inventory's declared base directory.
    pub path: String,
    /// Explicit media type when supplied.
    pub media_type: Option<String>,
    /// Named graph IRI, absent for a default-graph input.
    pub graph: Option<String>,
}
purrdf_lex::json_record!(Input as "inventory input" {
    "path" => path: required,
    "mediaType" => media_type: optional,
    "graph" => graph: optional,
});

/// One declared remote endpoint and its independently acquired source document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceInput {
    /// Absolute endpoint IRI; this is a semantic endpoint, not a local HTTP port.
    pub endpoint: String,
    /// Source document parsed independently of local data and other endpoints.
    pub data: Input,
}
purrdf_lex::json_record!(ServiceInput as "inventory service input" {
    "endpoint" => endpoint: required,
    "data" => data: required,
});

/// A full report/inference or intended failure expectation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedSpec {
    /// `report`, `inferences`, `admission-rejection`, or `semantic-failure`.
    pub kind: String,
    /// Distinguished report root or failure identifier.
    pub node: Option<String>,
    /// Stable semantic/profile reason identity for a negative expectation.
    pub reason: Option<String>,
    /// Exact conformance flag when the expectation is a report.
    pub conforms: Option<bool>,
    /// Independently authored inference artifact.
    pub path: Option<String>,
}
purrdf_lex::json_record!(ExpectedSpec as "inventory expected outcome" {
    "kind" => kind: required,
    "node" => node: optional,
    "reason" => reason: optional,
    "conforms" => conforms: optional,
    "path" => path: optional,
});

/// The original query-file schema and the contextual SHACL schema share one reader.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Expected {
    /// A SPARQL results or RDF graph artifact.
    File(String),
    /// Full report/inference or typed negative expectation.
    Outcome(ExpectedSpec),
    /// A negative syntax case has no result artifact.
    #[default]
    None,
}
impl FromJson for Expected {
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        match value {
            Value::Null => Ok(Self::None),
            Value::String(path) => Ok(Self::File(path.clone())),
            _ => ExpectedSpec::from_json(value).map(Self::Outcome),
        }
    }
}
impl ToJson for Expected {
    fn to_json(&self) -> Value {
        match self {
            Self::File(path) => path.to_json(),
            Self::Outcome(spec) => spec.to_json(),
            Self::None => Value::Null,
        }
    }
}

/// One declared case; metadata is read once and carried to every execution surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Case {
    /// Stable local case identifier.
    pub id: String,
    /// Explicit case IRI when the inventory supplies it.
    pub iri: Option<String>,
    /// Declared operation kind.
    pub kind: String,
    /// Declaring RDF manifest path.
    pub manifest: Option<String>,
    /// Shapes artifact path.
    pub shapes: Option<String>,
    /// Query artifact path.
    pub query: Option<String>,
    /// Parsed input acquisition records.
    pub data: Vec<Input>,
    /// Explicit endpoint datasets; absent for ordinary non-federated cases.
    pub services: Vec<ServiceInput>,
    /// Independently authored expectation.
    pub expected: Expected,
    /// Original single-profile query schema.
    pub profile: Option<String>,
    /// Multi-profile validation schema.
    pub profiles: Vec<String>,
    /// Declared semantic coverage tags.
    pub categories: Vec<String>,
    /// Primary specification references.
    pub specifications: Vec<String>,
    /// Independent oracle derivation.
    pub rationale: String,
    /// Applicable capabilities, separate from normative outcomes.
    pub requires: Vec<String>,
    /// Explicit caller options, decoded by the shipping option authority.
    pub options: Value,
    /// Related-control family identifier.
    pub family: Option<String>,
    /// Independently supplied oracle review record, retained verbatim.
    pub oracle_review: Option<Value>,
    /// Explicit non-normative extension metadata.
    pub extensions: Option<Value>,
}
purrdf_lex::json_record!(Case as "inventory case" {
    "id" => id: required,
    "iri" => iri: optional,
    "kind" => kind: required,
    "manifest" => manifest: optional,
    "shapes" => shapes: optional,
    "query" => query: optional,
    "data" => data: defaulted,
    "services" => services: defaulted,
    "expected" => expected: defaulted,
    "profile" => profile: optional,
    "profiles" => profiles: defaulted,
    "categories" => categories: required,
    "specifications" => specifications: required,
    "rationale" => rationale: required,
    "requires" => requires: defaulted,
    "options" => options: defaulted,
    "family" => family: optional,
    "oracleReview" => oracle_review: optional,
    "extensions" => extensions: optional,
});

impl Case {
    /// Exact declared profile list, preserving the original schema's singleton.
    ///
    /// # Errors
    /// Refuses ambiguous, empty or repeated profile declarations.
    pub fn selected_profiles(&self) -> Result<Vec<&str>, DecodeError> {
        let profiles: Vec<_> = match (&self.profile, self.profiles.is_empty()) {
            (Some(profile), true) => vec![profile.as_str()],
            (None, false) => self.profiles.iter().map(String::as_str).collect(),
            _ => {
                return Err(DecodeError::custom(
                    "case must declare exactly one profile representation",
                ));
            }
        };
        let distinct: BTreeSet<_> = profiles.iter().copied().collect();
        if profiles.iter().any(|profile| profile.is_empty()) || distinct.len() != profiles.len() {
            return Err(DecodeError::custom("empty or duplicate selected profile"));
        }
        Ok(profiles)
    }
}

/// One acquired inventory document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inventory {
    /// Declared schema revision, absent in the preserved query inventory.
    pub contract_version: Option<u32>,
    /// Fixture ownership.
    pub copyright: String,
    /// Fixture license.
    pub license: String,
    /// Profile descriptions from the preserved query schema.
    pub profiles: Option<Value>,
    /// Every declared case in source order.
    pub cases: Vec<Case>,
}
purrdf_lex::json_record!(Inventory as "conformance inventory" {
    "contractVersion" => contract_version: optional,
    "copyright" => copyright: required,
    "license" => license: required,
    "profiles" => profiles: optional,
    "cases" => cases: required,
});

/// A catalog's single authoritative inventory reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suite {
    /// The child inventory artifact.
    pub inventory: String,
    /// Relative base for paths in that child inventory.
    pub directory: String,
    /// Language/validation suite kind.
    pub kind: String,
    /// Exact expected number of independently identified cases.
    pub case_count: usize,
    /// RDF manifest roots used for reachability qualification.
    pub manifests: Vec<String>,
}
purrdf_lex::json_record!(Suite as "catalog suite" {
    "inventory" => inventory: required,
    "directory" => directory: required,
    "kind" => kind: required,
    "caseCount" => case_count: required,
    "manifests" => manifests: required,
});

/// A catalog references inventories without a competing copy of case metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalog {
    /// Catalog schema revision.
    pub contract_version: u32,
    /// Ownership notice.
    pub copyright: String,
    /// Fixture license.
    pub license: String,
    /// Approval/proposal category, never inferred from a passing engine.
    pub status: String,
    /// Aggregate manifest root.
    pub root_manifest: String,
    /// Unique inventory references.
    pub suites: Vec<Suite>,
    /// Exact total of all child case counts.
    pub case_count: usize,
    /// Required coverage metadata.
    pub required_coverage: Vec<String>,
    /// Public profile descriptions supplied by the corpus.
    pub profiles: Value,
    /// Root-relative independent review registry, when declared by the corpus.
    pub review_index: Option<String>,
}
purrdf_lex::json_record!(Catalog as "conformance catalog" {
    "contractVersion" => contract_version: required,
    "copyright" => copyright: required,
    "license" => license: required,
    "status" => status: required,
    "rootManifest" => root_manifest: required,
    "suites" => suites: required,
    "caseCount" => case_count: required,
    "requiredCoverage" => required_coverage: required,
    "profiles" => profiles: required,
    "reviewIndex" => review_index: optional,
});

/// Decode and validate a catalog without acquiring anything it references.
///
/// # Errors
/// Refuses strict JSON shape errors, unknown revisions and inconsistent totals.
pub fn decode_catalog(bytes: &[u8]) -> Result<Catalog, DecodeError> {
    let catalog = Catalog::from_json(
        &json::read_slice(bytes, json::Limits::DEFAULT).map_err(DecodeError::custom)?,
    )?;
    let distinct: BTreeSet<_> = catalog
        .suites
        .iter()
        .map(|suite| &suite.inventory)
        .collect();
    if catalog.contract_version != 1
        || catalog.suites.is_empty()
        || distinct.len() != catalog.suites.len()
        || catalog
            .suites
            .iter()
            .try_fold(0_usize, |total, suite| total.checked_add(suite.case_count))
            != Some(catalog.case_count)
    {
        return Err(DecodeError::custom(
            "catalog revision, unique suite references or case totals disagree",
        ));
    }
    Ok(catalog)
}

/// Decode and validate all cases, without an engine or filesystem.
///
/// # Errors
/// Refuses unknown revisions, duplicate identities and incomplete expectations.
pub fn decode_inventory(bytes: &[u8]) -> Result<Inventory, DecodeError> {
    let inventory = Inventory::from_json(
        &json::read_slice(bytes, json::Limits::DEFAULT).map_err(DecodeError::custom)?,
    )?;
    if inventory
        .contract_version
        .is_some_and(|version| version != 1)
        || inventory.cases.is_empty()
    {
        return Err(DecodeError::custom(
            "unknown inventory revision or empty case list",
        ));
    }
    let mut ids = BTreeSet::new();
    let mut iris = BTreeSet::new();
    for case in &inventory.cases {
        if case.id.is_empty()
            || !ids.insert(&case.id)
            || case
                .iri
                .as_ref()
                .is_some_and(|iri| iri.is_empty() || !iris.insert(iri))
            || case.rationale.is_empty()
            || case.specifications.is_empty()
        {
            return Err(DecodeError::custom(
                "case identity or independent rationale is absent or duplicated",
            ));
        }
        case.selected_profiles()?;
        let mut endpoints = BTreeSet::new();
        for service in &case.services {
            if case.kind != "query-evaluation"
                || service.data.path.is_empty()
                || service.data.graph.is_some()
                || !endpoints.insert(&service.endpoint)
                || purrdf_iri::BaseIri::parse(&service.endpoint).is_err()
            {
                return Err(DecodeError::custom(
                    "SERVICE inputs require distinct absolute endpoints and default-graph query source documents",
                ));
            }
        }
        let query_input =
            case.query.as_deref().is_some_and(|path| !path.is_empty()) && case.shapes.is_none();
        let shapes_input = case.shapes.as_deref().is_some_and(|path| !path.is_empty())
            && case.query.is_none()
            && !case.data.is_empty();
        let compatible = match (case.kind.as_str(), &case.expected) {
            ("query-evaluation", Expected::File(_)) | ("negative-syntax", Expected::None) => {
                query_input
            }
            ("validation", Expected::Outcome(spec)) => {
                shapes_input
                    && matches!(
                        spec.kind.as_str(),
                        "report" | "admission-rejection" | "semantic-failure"
                    )
            }
            ("rules", Expected::Outcome(spec)) => {
                shapes_input
                    && matches!(
                        spec.kind.as_str(),
                        "inferences" | "admission-rejection" | "semantic-failure"
                    )
            }
            _ => false,
        };
        if !compatible
            || case.data.iter().any(|input| {
                input.path.is_empty() || input.graph.as_deref().is_some_and(str::is_empty)
            })
        {
            return Err(DecodeError::custom(
                "operation, inputs and expected carrier are incompatible",
            ));
        }
        match &case.expected {
            Expected::None if case.kind == "negative-syntax" => {}
            Expected::File(path) if !path.is_empty() && case.query.is_some() => {}
            Expected::Outcome(spec) => match spec.kind.as_str() {
                "report"
                    if spec.conforms.is_some()
                        && spec.node.as_deref().is_some_and(|node| !node.is_empty()) => {}
                "inferences" if spec.path.as_deref().is_some_and(|path| !path.is_empty()) => {}
                "admission-rejection" | "semantic-failure"
                    if spec
                        .reason
                        .as_deref()
                        .is_some_and(|reason| !reason.is_empty()) => {}
                _ => {
                    return Err(DecodeError::custom(
                        "incomplete or unknown expected outcome",
                    ));
                }
            },
            _ => {
                return Err(DecodeError::custom(
                    "case kind and expected carrier disagree",
                ));
            }
        }
    }
    Ok(inventory)
}
