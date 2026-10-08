// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The community SHACL corpus, run through the public dated-profile doors.
//!
//! Acquisition admits the catalog, the inventory, the RDF manifest closure and
//! the review registry before anything executes. Every applicable (case,
//! profile) pair then runs through the fresh, prepared and restored-product
//! complete-report doors, and the shared conformance kit grades each route's
//! observation: complete report graphs by contextual graph isomorphism under
//! the selected test-format policy, refusals by their stable typed reason.
//!
//! A catalog profile that names no built-in dated SHACL law is reported as
//! [`OutcomeKind::Unsupported`]. It is counted on its own, never as a pass and
//! never as a mismatch.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use purrdf_conformance_kit::graph::Reader;
use purrdf_conformance_kit::outcome::{self, Observation, OutcomeKind, Record};
use purrdf_conformance_kit::report::{
    BlankCorrespondence, BlankIdentity, ReportContext, ReportFormat, ReportPolicy, SourceContext,
    SourceRole,
};
use purrdf_conformance_kit::{GradeError, inventory, manifest, report, reviews};
use purrdf_core::{GraphMatch, RdfDataset, TermId, TermValue};
use purrdf_iri::vocab::{mf, rdf, sh, sht};
use purrdf_lex::json::Value;
use purrdf_shapes::ShaclProfile;
use purrdf_shapes::engine::{PreparedShapes, ValidationOptions};
use purrdf_shapes::profile::AdmissionReason;
use purrdf_shapes::report::{CompleteValidationError, CompleteValidationReport};
use purrdf_validate::CompleteValidationStatus;

/// The stable base every corpus artifact IRI is minted under: the corpus root's
/// relative path appended to it. Copying the corpus keeps every identity.
pub const BASE: &str = "http://example.org/community/";

/// The built-in dated laws a catalog profile can select.
const DATED: [ShaclProfile; 2] = [ShaclProfile::REC_20170720, ShaclProfile::WD_20260918];

/// Data and shapes identity domains. The two documents are acquired separately,
/// so their blanks never share identity.
const DATA_SOURCE: &str = "input-data";
const SHAPES_SOURCE: &str = "input-shapes";

/// A parsed shapes document: its dataset, artifact base and document prefixes.
type ShapesSource = (Arc<RdfDataset>, String, Vec<(String, String)>);

/// One parsed manifest document of the acquired closure.
#[derive(Debug)]
struct ManifestDocument {
    dataset: Arc<RdfDataset>,
    entries: Vec<TermId>,
}

/// The admitted corpus: catalog, inventories and the parsed manifest closure.
#[derive(Debug)]
pub struct Corpus {
    root: PathBuf,
    catalog: inventory::Catalog,
    inventories: Vec<inventory::Inventory>,
    documents: BTreeMap<String, ManifestDocument>,
}

/// One inventory case bound to its unique manifest entry.
#[derive(Debug, Clone, Copy)]
pub struct Case<'a> {
    /// The inventory record.
    pub case: &'a inventory::Case,
    directory: &'a str,
    document: &'a ManifestDocument,
    entry: TermId,
}

impl Case<'_> {
    /// The case IRI the manifest entry and every record carry.
    #[must_use]
    pub fn iri(&self) -> &str {
        self.case.iri.as_deref().unwrap_or(&self.case.id)
    }
}

/// How a catalog profile resolves against the built-in dated laws.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    /// The catalog profile names this built-in dated law.
    Supported(ShaclProfile),
    /// No built-in dated law implements this catalog profile.
    Unsupported(String),
}

/// One applicable (case, profile) execution and its per-route records.
#[derive(Debug, Clone)]
pub struct Execution {
    /// Case IRI.
    pub case: String,
    /// Catalog profile identity.
    pub profile: String,
    /// One record per executed public route.
    pub records: Vec<Record>,
}

impl Execution {
    /// True when the profile was unsupported, so nothing was graded.
    #[must_use]
    pub fn unsupported(&self) -> bool {
        self.records
            .iter()
            .any(|record| record.observed.kind == OutcomeKind::Unsupported)
    }

    /// True exactly when every route established the declared outcome.
    #[must_use]
    pub fn passed(&self) -> bool {
        !self.records.is_empty() && self.records.iter().all(|record| record.passed)
    }
}

/// Aggregate counts, with unsupported executions kept apart from failures.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Totals {
    /// Inventory cases.
    pub cases: usize,
    /// Applicable (case, profile) executions.
    pub executions: usize,
    /// Executions whose every route passed.
    pub passed: usize,
    /// Supported executions with at least one failing route.
    pub failed: usize,
    /// Executions under a profile no built-in law implements.
    pub unsupported: usize,
    /// Graded route observations.
    pub observations: usize,
    /// Passing route observations.
    pub passed_observations: usize,
    /// Per catalog profile: (executions, passed, failed, unsupported).
    pub profiles: BTreeMap<String, [usize; 4]>,
}

impl Totals {
    /// Tally a complete run.
    #[must_use]
    pub fn of(cases: usize, executions: &[Execution]) -> Self {
        let mut totals = Self {
            cases,
            executions: executions.len(),
            ..Self::default()
        };
        for execution in executions {
            let row = totals
                .profiles
                .entry(execution.profile.clone())
                .or_default();
            row[0] += 1;
            if execution.unsupported() {
                totals.unsupported += 1;
                row[3] += 1;
                continue;
            }
            totals.observations += execution.records.len();
            totals.passed_observations += execution
                .records
                .iter()
                .filter(|record| record.passed)
                .count();
            if execution.passed() {
                totals.passed += 1;
                row[1] += 1;
            } else {
                totals.failed += 1;
                row[2] += 1;
            }
        }
        totals
    }

    /// The scoreboard lines the conformance matrix reads.
    #[must_use]
    pub fn scoreboard(&self) -> String {
        use std::fmt::Write as _;
        let mut lines = String::new();
        for (profile, [executions, passed, failed, unsupported]) in &self.profiles {
            let _ = writeln!(
                lines,
                "COMMUNITY SHACL PROFILE {profile}: executions {executions} passed {passed} \
                 failed {failed} unsupported {unsupported}"
            );
        }
        let _ = writeln!(
            lines,
            "COMMUNITY SHACL TOTAL: cases {} executions {} passed {} failed {} unsupported {} \
             observations {} passedObservations {}",
            self.cases,
            self.executions,
            self.passed,
            self.failed,
            self.unsupported,
            self.observations,
            self.passed_observations,
        );
        let _ = writeln!(
            lines,
            "COMMUNITY-SHACL: passed {} total {}",
            self.passed, self.executions
        );
        lines
    }
}

fn malformed(message: impl Into<String>) -> GradeError {
    GradeError::Malformed(message.into())
}

/// Read one artifact, naming it in the diagnostic.
///
/// # Errors
/// Returns the read failure with its path.
pub fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))
}

impl Corpus {
    /// Acquire and admit the catalog, inventories, manifest closure and reviews.
    ///
    /// # Errors
    /// Refuses malformed metadata, an unreadable or escaping artifact, a suite
    /// kind this runner does not execute, an include cycle, an incomplete
    /// one-to-one case closure, missing coverage, or reviewed bytes that changed.
    pub fn acquire(root: &Path) -> Result<Self, String> {
        let root = root
            .canonicalize()
            .map_err(|error| format!("{}: {error}", root.display()))?;
        let catalog = inventory::decode_catalog(&read(&root.join("catalog.json"))?)
            .map_err(|error| error.to_string())?;
        let mut inventories = Vec::new();
        for suite in &catalog.suites {
            if suite.kind != "shacl" {
                return Err(format!(
                    "{}: suite kind {} has no community SHACL runner",
                    suite.inventory, suite.kind
                ));
            }
            let inventory =
                inventory::decode_inventory(&read(&Self::file(&root, ".", &suite.inventory)?)?)
                    .map_err(|error| error.to_string())?;
            if inventory.cases.len() != suite.case_count {
                return Err(format!(
                    "{}: inventory count disagrees with the catalog",
                    suite.inventory
                ));
            }
            inventories.push(inventory);
        }
        let mut corpus = Self {
            root,
            catalog,
            inventories,
            documents: BTreeMap::new(),
        };
        corpus.acquire_manifests()?;
        corpus.admit_reviews()?;
        let mut coverage = BTreeSet::new();
        for inventory in &corpus.inventories {
            for case in &inventory.cases {
                coverage.extend(case.categories.iter().map(String::as_str));
            }
        }
        for required in &corpus.catalog.required_coverage {
            if !coverage.contains(required.as_str()) {
                return Err(format!("required coverage tag {required} is absent"));
            }
        }
        Ok(corpus)
    }

    /// Resolve a corpus artifact, refusing an escape from the corpus root.
    fn file(root: &Path, directory: &str, relative: &str) -> Result<PathBuf, String> {
        let path = root
            .join(directory)
            .join(relative)
            .canonicalize()
            .map_err(|error| format!("{relative}: {error}"))?;
        if path.starts_with(root) {
            Ok(path)
        } else {
            Err(format!("{relative}: artifact escapes the corpus root"))
        }
    }

    /// The root-relative path of an admitted artifact.
    fn relative(&self, path: &Path) -> Result<String, String> {
        path.strip_prefix(&self.root)
            .map_err(|_| format!("{}: outside the corpus root", path.display()))?
            .to_str()
            .map(|path| path.replace('\\', "/"))
            .ok_or_else(|| format!("{}: path is not UTF-8", path.display()))
    }

    /// The stable artifact IRI of a root-relative path.
    fn artifact_iri(relative: &str) -> String {
        format!("{BASE}{relative}")
    }

    /// Follow `mf:include` structurally from the root manifest.
    fn acquire_manifests(&mut self) -> Result<(), String> {
        let mut pending =
            vec![self.relative(&Self::file(&self.root, ".", &self.catalog.root_manifest)?)?];
        while let Some(relative) = pending.pop() {
            if self.documents.contains_key(&relative) {
                return Err(format!("{relative}: manifest is included more than once"));
            }
            let dataset = purrdf::parse_dataset(
                &read(&self.root.join(&relative))?,
                "text/turtle",
                Some(&Self::artifact_iri(&relative)),
            )
            .map_err(|error| format!("{relative}: {error}"))?;
            let metadata = manifest::decode(Reader {
                dataset: &dataset,
                graph: GraphMatch::Default,
            })
            .map_err(|error| format!("{relative}: {error}"))?;
            for include in &metadata.includes {
                let target = include.strip_prefix(BASE).ok_or_else(|| {
                    format!("{relative}: include {include} is outside the corpus")
                })?;
                pending.push(self.relative(&Self::file(&self.root, ".", target)?)?);
            }
            self.documents.insert(
                relative,
                ManifestDocument {
                    dataset,
                    entries: metadata.entries,
                },
            );
        }
        Ok(())
    }

    /// Admit every reviewed payload byte against the review registry.
    fn admit_reviews(&self) -> Result<(), String> {
        let index_path = self
            .catalog
            .review_index
            .as_deref()
            .ok_or_else(|| "the community corpus requires a review registry".to_owned())?;
        let index = reviews::decode(&read(&Self::file(&self.root, ".", index_path)?)?)
            .map_err(|error| error.to_string())?;
        let paths: BTreeSet<_> = index
            .cases
            .iter()
            .flat_map(|case| case.payloads.iter().map(|payload| payload.path.clone()))
            .collect();
        let bytes = paths
            .into_iter()
            .map(|path| Ok((read(&Self::file(&self.root, ".", &path)?)?, path)))
            .collect::<Result<Vec<_>, String>>()?;
        let payloads: Vec<_> = bytes
            .iter()
            .map(|(bytes, path)| reviews::Payload { path, bytes })
            .collect();
        reviews::admit(&self.catalog, &self.inventories, &index, &payloads)
            .map_err(|error| error.to_string())
    }

    /// The admitted catalog.
    #[must_use]
    pub const fn catalog(&self) -> &inventory::Catalog {
        &self.catalog
    }

    /// Bind every inventory case to exactly one reachable manifest entry.
    ///
    /// # Errors
    /// Refuses a case with zero or several matching entries, an entry no case
    /// claims, or a manifest entry that disagrees with its inventory record.
    pub fn cases(&self) -> Result<Vec<Case<'_>>, String> {
        let mut unclaimed: BTreeSet<(&str, TermId)> = self
            .documents
            .iter()
            .flat_map(|(path, document)| {
                document
                    .entries
                    .iter()
                    .map(move |entry| (path.as_str(), *entry))
            })
            .collect();
        let mut cases = Vec::new();
        for (suite, inventory) in self.catalog.suites.iter().zip(&self.inventories) {
            for case in &inventory.cases {
                let iri = case
                    .iri
                    .as_deref()
                    .ok_or_else(|| format!("{}: a SHACL case must declare its IRI", case.id))?;
                let mut matches = Vec::new();
                for (path, document) in &self.documents {
                    let reader = Reader {
                        dataset: &document.dataset,
                        graph: GraphMatch::Default,
                    };
                    for &entry in &document.entries {
                        if reader.iri(entry).map_err(|error| error.to_string())? == iri {
                            matches.push((path.as_str(), document, entry));
                        }
                    }
                }
                let [(path, document, entry)] = matches.as_slice() else {
                    return Err(format!(
                        "{}: {} reachable manifest entries carry its IRI",
                        case.id,
                        matches.len()
                    ));
                };
                let declared = case
                    .manifest
                    .as_deref()
                    .ok_or_else(|| format!("{}: the declaring manifest is absent", case.id))?;
                if self.relative(&Self::file(&self.root, &suite.directory, declared)?)? != *path {
                    return Err(format!(
                        "{}: manifest ownership differs from the inventory",
                        case.id
                    ));
                }
                unclaimed.remove(&(*path, *entry));
                let bound = Case {
                    case,
                    directory: &suite.directory,
                    document,
                    entry: *entry,
                };
                self.admit_entry(&suite.directory, &bound)
                    .map_err(|error| format!("{}: {error}", case.id))?;
                cases.push(bound);
            }
        }
        if !unclaimed.is_empty() || cases.len() != self.catalog.case_count {
            return Err("manifest entries and inventory cases are not one-to-one".to_owned());
        }
        Ok(cases)
    }

    /// Check one manifest entry against its inventory record before execution.
    fn admit_entry(&self, directory: &str, case: &Case<'_>) -> Result<(), String> {
        let reader = Reader {
            dataset: &case.document.dataset,
            graph: GraphMatch::Default,
        };
        let iri_of = |relative: &str| -> Result<String, String> {
            Ok(Self::artifact_iri(
                &self.relative(&Self::file(&self.root, directory, relative)?)?,
            ))
        };
        let error = |error: GradeError| error.to_string();
        let types: BTreeSet<_> = reader
            .objects(case.entry, rdf::TYPE)
            .into_iter()
            .map(|id| reader.iri(id))
            .collect::<Result<_, _>>()
            .map_err(error)?;
        let required_type = match case.case.kind.as_str() {
            "validation" => sht::VALIDATE,
            "rules" => sht::INFER,
            kind => return Err(format!("operation {kind} is not a SHACL case")),
        };
        if !types.contains(required_type) {
            return Err(format!("the manifest entry is not typed {required_type}"));
        }
        let action = reader.one(case.entry, mf::ACTION).map_err(error)?;
        let shapes = reader
            .iri(reader.one(action, sht::SHAPES_GRAPH).map_err(error)?)
            .map_err(error)?;
        let data = reader
            .iri(reader.one(action, sht::DATA_GRAPH).map_err(error)?)
            .map_err(error)?;
        let declared_shapes = case
            .case
            .shapes
            .as_deref()
            .ok_or_else(|| "the shapes graph is absent".to_owned())?;
        let [declared_data] = case.case.data.as_slice() else {
            return Err("a SHACL test action names exactly one data graph".to_owned());
        };
        if declared_data.graph.is_some()
            || declared_data
                .media_type
                .as_deref()
                .is_some_and(|media| media != "text/turtle")
        {
            return Err("the data graph must be a default-graph Turtle document".to_owned());
        }
        if shapes != iri_of(declared_shapes)? || data != iri_of(&declared_data.path)? {
            return Err("the manifest action disagrees with the inventory inputs".to_owned());
        }
        let result = reader.one(case.entry, mf::RESULT).map_err(error)?;
        let failure = matches!(
            case.document.dataset.term_value(result),
            TermValue::Iri(ref iri) if iri == sht::FAILURE
        );
        let inventory::Expected::Outcome(spec) = &case.case.expected else {
            return Err("a SHACL case declares a typed expected outcome".to_owned());
        };
        let consistent = match spec.kind.as_str() {
            "admission-rejection" | "semantic-failure" => failure,
            "report" => !failure && report_root_in(&case.document.dataset, result).is_ok(),
            "inferences" => !failure && manifest::inference_triples(reader, result).is_ok(),
            _ => false,
        };
        if consistent {
            Ok(())
        } else {
            Err(format!(
                "mf:result disagrees with the {} expectation",
                spec.kind
            ))
        }
    }

    /// Resolve a catalog profile against the built-in dated laws by the exact
    /// specification it names.
    ///
    /// # Errors
    /// Refuses a profile the case selects but the catalog does not declare.
    pub fn select(&self, profile: &str) -> Result<Selection, String> {
        select(&self.catalog.profiles, profile)
    }

    fn path(&self, case: &Case<'_>, relative: &str) -> Result<PathBuf, String> {
        Self::file(&self.root, case.directory, relative)
    }

    /// The data graph: the parsed manifest itself when the action names it,
    /// so the inline expected report and the validated data share identity.
    fn data(&self, case: &Case<'_>) -> Result<Arc<RdfDataset>, String> {
        let [input] = case.case.data.as_slice() else {
            return Err("a SHACL test action names exactly one data graph".to_owned());
        };
        let relative = self.relative(&self.path(case, &input.path)?)?;
        if let Some(document) = self.documents.get(&relative) {
            return Ok(Arc::clone(&document.dataset));
        }
        purrdf::parse_dataset(
            &read(&self.root.join(&relative))?,
            "text/turtle",
            Some(&Self::artifact_iri(&relative)),
        )
        .map_err(|error| format!("{relative}: {error}"))
    }

    /// Parse the shapes document once; the oracle and the engine read this one
    /// acquisition.
    fn shapes_source(&self, case: &Case<'_>) -> Result<ShapesSource, String> {
        let declared = case
            .case
            .shapes
            .as_deref()
            .ok_or_else(|| "the shapes graph is absent".to_owned())?;
        let relative = self.relative(&self.path(case, declared)?)?;
        let base = Self::artifact_iri(&relative);
        let parsed = purrdf::parse_dataset_with(
            &read(&self.root.join(&relative))?,
            "text/turtle",
            Some(&base),
            &purrdf::ParseOptions::default(),
        )
        .map_err(|error| format!("{relative}: {error}"))?;
        Ok((parsed.dataset, base, parsed.document_prefixes))
    }

    /// Execute one case under one catalog profile through every public route.
    ///
    /// # Errors
    /// Refuses an unreadable or unparseable input artifact; engine outcomes,
    /// including failures, are records rather than errors.
    pub fn execute(&self, case: &Case<'_>, profile: &str) -> Result<Execution, String> {
        let selection = self.select(profile)?;
        self.execute_selected(case, profile, &selection)
    }

    /// Execute one case under an already resolved selection.
    ///
    /// # Errors
    /// As [`Self::execute`].
    pub fn execute_selected(
        &self,
        case: &Case<'_>,
        profile: &str,
        selection: &Selection,
    ) -> Result<Execution, String> {
        let expected = expected_observation(case.case);
        let record = |surface: &str, observed: Observation| {
            let passed = outcome::grade(&expected, &observed).is_ok();
            Record {
                case: case.iri().to_owned(),
                profile: profile.to_owned(),
                surface: surface.to_owned(),
                expected: expected.clone(),
                observed,
                passed,
                artifacts: Vec::new(),
            }
        };
        let law = match selection {
            Selection::Supported(law) => *law,
            Selection::Unsupported(reason) => {
                return Ok(Execution {
                    case: case.iri().to_owned(),
                    profile: profile.to_owned(),
                    records: vec![record(
                        "native",
                        Observation {
                            kind: OutcomeKind::Unsupported,
                            reason: None,
                            message: reason.clone(),
                        },
                    )],
                });
            }
        };
        let data = self.data(case)?;
        let (shapes_dataset, base, prefixes) = self.shapes_source(case)?;
        let options = ValidationOptions::default().with_profile(law);
        let mut records = Vec::new();
        let shapes = match purrdf_shapes::shapes::from_dataset_with_options(
            &shapes_dataset,
            Some(&base),
            &prefixes,
            None,
            None,
            &purrdf_shapes::ShapesImports::default(),
            &options,
        ) {
            Ok(shapes) => Arc::new(shapes),
            Err(error) => {
                records.push(record("native/parse", refusal(law, &error)));
                return Ok(Execution {
                    case: case.iri().to_owned(),
                    profile: profile.to_owned(),
                    records,
                });
            }
        };
        if case.case.kind == "rules" {
            let observed = match purrdf_shapes::rules::run_rules(
                purrdf_shapes::rules::RuleSource::Shapes(&shapes),
                &data,
                &purrdf_shapes::rules::RuleLimits::default(),
                purrdf_shapes::rules::LimitKnobs::new("rounds", "terms", "facts", "joins"),
            ) {
                Ok(inference) => self.grade_inferences(case, &inference),
                Err(message) => Observation {
                    kind: OutcomeKind::Crash,
                    reason: None,
                    message,
                },
            };
            records.push(record("native/rules", observed));
        } else {
            let grade = |outcome: Result<CompleteValidationReport, CompleteValidationError>| {
                self.observe(case, law, &shapes_dataset, outcome)
            };
            records.push(record(
                "native/fresh",
                grade(purrdf_validate::validate_complete_sources(
                    Arc::clone(&data),
                    Arc::clone(&shapes),
                    &options,
                )),
            ));
            let prepared =
                PreparedShapes::new(Arc::clone(&shapes)).with_validation_options(options.clone());
            match prepared.bind_complete_shared_dataset(Arc::clone(&data)) {
                Ok(validator) => {
                    for repetition in 0..2 {
                        records.push(record(
                            &format!("native/prepared/{repetition}"),
                            grade(validator.validate()),
                        ));
                    }
                }
                Err(error) => records.push(record("native/prepared/bind", grade(Err(error)))),
            }
            match purrdf_validate::prepared_to_product(&prepared) {
                Ok(product) => {
                    for (surface, rebuild) in [
                        ("native/product-admitted", false),
                        ("native/product-rebuilt", true),
                    ] {
                        let outcome = purrdf_validate::restore_shapes_product_with_options(
                            &product, &options, None, rebuild,
                        )
                        .and_then(|restored| {
                            restored.bind_complete_shared_dataset(Arc::clone(&data))
                        })
                        .and_then(|validator| validator.validate());
                        records.push(record(surface, grade(outcome)));
                    }
                }
                Err(error) => records.push(record(
                    "native/product",
                    Observation {
                        kind: OutcomeKind::Crash,
                        reason: None,
                        message: error.to_string(),
                    },
                )),
            }
        }
        Ok(Execution {
            case: case.iri().to_owned(),
            profile: profile.to_owned(),
            records,
        })
    }

    /// Classify one complete-report outcome; a produced report is graded.
    fn observe(
        &self,
        case: &Case<'_>,
        law: ShaclProfile,
        shapes: &RdfDataset,
        outcome: Result<CompleteValidationReport, CompleteValidationError>,
    ) -> Observation {
        match outcome {
            Ok(report) => match compare_report(case, law, shapes, &report) {
                Ok(()) => Observation {
                    kind: OutcomeKind::Compared,
                    reason: None,
                    message: "the complete report equals the reviewed expectation".to_owned(),
                },
                Err(GradeError::Mismatch(message)) => Observation {
                    kind: OutcomeKind::Mismatch,
                    reason: None,
                    message,
                },
                Err(GradeError::Canonicalization(message)) => Observation {
                    kind: OutcomeKind::ResourceExhausted,
                    reason: None,
                    message,
                },
                Err(GradeError::Malformed(message)) => Observation {
                    kind: OutcomeKind::Malformed,
                    reason: None,
                    message,
                },
            },
            Err(error) => refusal(law, &error),
        }
    }

    /// Compare an inference delta with both the reviewed expected graph and the
    /// manifest's inline triple list.
    fn grade_inferences(
        &self,
        case: &Case<'_>,
        inference: &purrdf_shapes::srl::Inference,
    ) -> Observation {
        let graded = (|| {
            let inventory::Expected::Outcome(spec) = &case.case.expected else {
                return Err(malformed("inference expectation absent"));
            };
            let relative = spec
                .path
                .as_deref()
                .ok_or_else(|| malformed("inference expectation absent"))?;
            let path = self.path(case, relative).map_err(malformed)?;
            let relative = self.relative(&path).map_err(malformed)?;
            let expected = purrdf::parse_dataset(
                &read(&path).map_err(malformed)?,
                "text/turtle",
                Some(&Self::artifact_iri(&relative)),
            )
            .map_err(|error| malformed(error.to_string()))?;
            let inline = {
                let reader = Reader {
                    dataset: &case.document.dataset,
                    graph: GraphMatch::Default,
                };
                let triples =
                    manifest::inference_triples(reader, reader.one(case.entry, mf::RESULT)?)?;
                triples_dataset(&triples)?
            };
            report::compare_graphs(&expected, &inline).map_err(|_| {
                malformed("the inline mf:result list and the expected graph disagree")
            })?;
            let actual = inference.inferred_dataset().map_err(malformed)?;
            report::compare_graphs(&expected, &actual)
        })();
        match graded {
            Ok(()) => Observation {
                kind: OutcomeKind::Compared,
                reason: None,
                message: "the inference delta equals the reviewed expectation".to_owned(),
            },
            Err(GradeError::Mismatch(message)) => Observation {
                kind: OutcomeKind::Mismatch,
                reason: None,
                message,
            },
            Err(GradeError::Canonicalization(message)) => Observation {
                kind: OutcomeKind::ResourceExhausted,
                reason: None,
                message,
            },
            Err(GradeError::Malformed(message)) => Observation {
                kind: OutcomeKind::Malformed,
                reason: None,
                message,
            },
        }
    }

    /// Execute every applicable (case, profile) pair of the admitted corpus.
    ///
    /// # Errors
    /// As [`Self::cases`] and [`Self::execute`].
    pub fn run(&self) -> Result<(Totals, Vec<Execution>), String> {
        let cases = self.cases()?;
        let mut executions = Vec::new();
        for case in &cases {
            for profile in case
                .case
                .selected_profiles()
                .map_err(|error| error.to_string())?
            {
                executions.push(self.execute(case, profile)?);
            }
        }
        Ok((Totals::of(cases.len(), &executions), executions))
    }
}

/// Resolve a catalog profile against the built-in dated laws.
///
/// A profile resolves to the law whose dated Core or SPARQL specification is
/// exactly the one the catalog names; no nearby date and no legacy fallback is
/// used.
///
/// # Errors
/// Refuses a profile absent from the catalog or without a specification.
pub fn select(profiles: &Value, profile: &str) -> Result<Selection, String> {
    let specification = profiles
        .get(profile)
        .ok_or_else(|| format!("profile {profile} is absent from the catalog"))?
        .get("specification")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("profile {profile} names no specification"))?;
    Ok(DATED
        .into_iter()
        .find(|law| {
            law.core_specification() == Some(specification)
                || law.sparql_specification() == Some(specification)
        })
        .map_or_else(
            || {
                Selection::Unsupported(format!(
                    "no built-in dated SHACL law implements {specification}"
                ))
            },
            Selection::Supported,
        ))
}

/// The declared outcome, read from the inventory alone.
#[must_use]
pub fn expected_observation(case: &inventory::Case) -> Observation {
    let (kind, reason) = match &case.expected {
        inventory::Expected::Outcome(spec) if spec.kind == "admission-rejection" => {
            (OutcomeKind::IntendedRejection, spec.reason.clone())
        }
        inventory::Expected::Outcome(spec) if spec.kind == "semantic-failure" => {
            (OutcomeKind::SemanticFailure, spec.reason.clone())
        }
        _ => (OutcomeKind::Compared, None),
    };
    Observation {
        kind,
        reason,
        message: "reviewed corpus expectation".to_owned(),
    }
}

/// The corpus's stable reason token for a typed admission refusal.
///
/// The Recommendation forbids every explicit VALUES clause and every SERVICE
/// pattern outright. The dated draft forbids only a VALUES clause that names a
/// potentially pre-bound variable, so its refusal carries that variable, and
/// its SERVICE refusal is a declared processor policy.
#[must_use]
pub fn admission_reason(
    law: ShaclProfile,
    reason: AdmissionReason,
    variable: Option<&str>,
) -> &'static str {
    match reason {
        AdmissionReason::Minus => "minus",
        AdmissionReason::Service if law == ShaclProfile::REC_20170720 => "service",
        AdmissionReason::Service => "service-policy",
        AdmissionReason::Values if variable.is_some() => "prebound-values",
        AdmissionReason::Values => "explicit-values",
        AdmissionReason::Assignment => "prebound-as",
        AdmissionReason::SubqueryProjection => "subquery-projection",
        AdmissionReason::QueryForm => "query-form",
        _ => "unclassified-admission",
    }
}

/// Classify a typed refusal through the shared status classifier.
fn refusal(law: ShaclProfile, error: &CompleteValidationError) -> Observation {
    let message = error.to_string();
    let status = purrdf_validate::complete_validation_status(&Err(match error {
        CompleteValidationError::Admission(refusal) => {
            CompleteValidationError::Admission(refusal.clone())
        }
        CompleteValidationError::Semantic(failure) => {
            CompleteValidationError::Semantic(failure.clone())
        }
        CompleteValidationError::Resource(refusal) => {
            CompleteValidationError::Resource(refusal.clone())
        }
        other => CompleteValidationError::Execution(other.to_string()),
    }));
    let (kind, reason) = match (status, error) {
        (
            CompleteValidationStatus::AdmissionRefused,
            CompleteValidationError::Admission(refusal),
        ) => (
            OutcomeKind::IntendedRejection,
            Some(admission_reason(law, refusal.reason(), refusal.variable()).to_owned()),
        ),
        (CompleteValidationStatus::SemanticFailure, _) => (
            OutcomeKind::SemanticFailure,
            Some("solution-failure".to_owned()),
        ),
        (CompleteValidationStatus::ResourceRefused, _) => (OutcomeKind::ResourceExhausted, None),
        (CompleteValidationStatus::UnsupportedProfile, _) => (OutcomeKind::Unsupported, None),
        _ => (OutcomeKind::Crash, None),
    };
    Observation {
        kind,
        reason,
        message,
    }
}

/// The unique validation report root of a report graph.
fn report_root_in(dataset: &RdfDataset, candidate: TermId) -> Result<TermId, GradeError> {
    let reader = Reader {
        dataset,
        graph: GraphMatch::Default,
    };
    let class = dataset
        .term_id_by_iri(sh::VALIDATION_REPORT)
        .ok_or_else(|| malformed("the report class is absent"))?;
    if reader.objects(candidate, rdf::TYPE).contains(&class) {
        Ok(candidate)
    } else {
        Err(malformed("mf:result is not a validation report"))
    }
}

fn report_root(dataset: &RdfDataset) -> Result<TermId, GradeError> {
    let class = dataset
        .term_id_by_iri(sh::VALIDATION_REPORT)
        .ok_or_else(|| malformed("the report class is absent"))?;
    let roots = Reader {
        dataset,
        graph: GraphMatch::Default,
    }
    .subjects(rdf::TYPE, class);
    match roots.as_slice() {
        [root] => Ok(*root),
        _ => Err(malformed(format!("a report has {} roots", roots.len()))),
    }
}

/// Every blank identity in a report graph, keyed by its label.
fn blank_labels(dataset: &RdfDataset) -> BTreeMap<String, BlankIdentity> {
    let mut labels = BTreeMap::new();
    for quad in dataset.quads() {
        for id in [Some(quad.s), Some(quad.p), Some(quad.o), quad.g]
            .into_iter()
            .flatten()
        {
            let _ = dataset
                .term_value(id)
                .visit_blank_identities(|label, scope| {
                    labels.insert(
                        label.to_owned(),
                        BlankIdentity {
                            label: label.to_owned(),
                            scope,
                        },
                    );
                    ControlFlow::<()>::Continue(())
                });
        }
    }
    labels
}

/// Grade a produced complete report against the case's inline expected report.
///
/// The expected source correspondence is frozen from the reviewed inputs; the
/// actual one is the producer's own record of which source blank each emitted
/// blank is. Report-only isomorphism cannot repair a wrong source identity.
fn compare_report(
    case: &Case<'_>,
    law: ShaclProfile,
    shapes: &RdfDataset,
    actual: &CompleteValidationReport,
) -> Result<(), GradeError> {
    let manifest_dataset = &case.document.dataset;
    let reader = Reader {
        dataset: manifest_dataset,
        graph: GraphMatch::Default,
    };
    let expected_root = report_root_in(manifest_dataset, reader.one(case.entry, mf::RESULT)?)?;
    let expected = report::extract_report(manifest_dataset, expected_root)?;
    let expected_root = report_root(&expected)?;
    let inventory::Expected::Outcome(spec) = &case.case.expected else {
        return Err(malformed("the report expectation is absent"));
    };
    let expected_reader = Reader {
        dataset: &expected,
        graph: GraphMatch::Default,
    };
    let conforms = expected_reader.lexical(expected_reader.one(expected_root, sh::CONFORMS)?)?;
    if !matches!(conforms.as_str(), "true" | "false") || spec.conforms != Some(conforms == "true") {
        return Err(malformed(
            "the inline report and the inventory disagree on conformance",
        ));
    }
    let expected_sources = [
        SourceContext {
            dataset: manifest_dataset,
            role: SourceRole::DataGraph,
            domain: DATA_SOURCE,
        },
        SourceContext {
            dataset: shapes,
            role: SourceRole::ShapesGraph,
            domain: SHAPES_SOURCE,
        },
    ];
    let expected_links = report::expected_source_correspondence(&expected, &expected_sources)?;

    let graph = actual.to_graph();
    let actual_dataset = graph.dataset();
    let actual_root = report_root(actual_dataset)?;
    let context = graph.source_context();
    if context.sources_share_identity() {
        return Err(GradeError::Mismatch(
            "independently acquired data and shapes were reported as one source".to_owned(),
        ));
    }
    let actual_sources = [
        SourceContext {
            dataset: context.data().dataset(),
            role: SourceRole::DataGraph,
            domain: DATA_SOURCE,
        },
        SourceContext {
            dataset: context.shapes().dataset(),
            role: SourceRole::ShapesGraph,
            domain: SHAPES_SOURCE,
        },
    ];
    let emitted = blank_labels(actual_dataset);
    let actual_links = graph
        .blank_labels()
        .sources()
        .map(|(label, source)| {
            Ok(BlankCorrespondence {
                report: emitted
                    .get(label)
                    .cloned()
                    .ok_or_else(|| malformed("a correspondence names an absent report blank"))?,
                source_role: if source.source() == context.data() {
                    SourceRole::DataGraph
                } else {
                    SourceRole::ShapesGraph
                },
                source: BlankIdentity {
                    label: source.label().to_owned(),
                    scope: source.scope(),
                },
            })
        })
        .collect::<Result<Vec<_>, GradeError>>()?;
    let policy = ReportPolicy {
        format: if law == ShaclProfile::REC_20170720 {
            ReportFormat::Recommendation2017
        } else {
            ReportFormat::Draft20260918
        },
        allow_unstated_source_constraint: false,
    };
    report::compare_with_policy(
        ReportContext {
            report: &expected,
            root: Some(expected_root),
            sources: &expected_sources,
            correspondence: &expected_links,
        },
        ReportContext {
            report: actual_dataset,
            root: Some(actual_root),
            sources: &actual_sources,
            correspondence: &actual_links,
        },
        policy,
    )
}

/// A default-graph dataset of the manifest's inline expected triples.
fn triples_dataset(triples: &[[TermValue; 3]]) -> Result<Arc<RdfDataset>, GradeError> {
    let mut builder = purrdf_core::RdfDatasetBuilder::new();
    for [s, p, o] in triples {
        let s = purrdf_core::TermFactory::intern_value(&mut builder, s);
        let p = purrdf_core::TermFactory::intern_value(&mut builder, p);
        let o = purrdf_core::TermFactory::intern_value(&mut builder, o);
        builder.push_quad(s, p, o, None);
    }
    builder
        .freeze()
        .map_err(|error| malformed(error.to_string()))
}
