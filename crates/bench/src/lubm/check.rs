// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Production acceptance of actual native corpus bytes and query answers.
use super::{PROFILE, Receipt, Spec};
use purrdf_iri::vocab::{owl, rdf};
use purrdf_rdf::{
    FastSet, NativeRdfFormat, RdfDataset, RdfDatasetBuilder, RdfTerm, RdfTriple, TermValue,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

/// The external ontology identity stays separate from any namespace projection.
pub const EXTERNAL_ONTOLOGY: &str = "http://swat.cse.lehigh.edu/onto/univ-bench.owl";

/// Independently observed full-corpus answer sets and byte identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Acceptance {
    /// Named generator profile.
    pub profile: String,
    /// Actual generation receipt digest.
    pub receipt_blake3: String,
    /// Actual aggregate converted content digest.
    pub converted_blake3: String,
    /// Number of complete department payloads.
    pub files: u64,
    /// Number of graph statements.
    pub statements: u64,
    /// Q1's graduate students taking the original fixed course.
    pub q1: Vec<String>,
    /// Q14's explicitly typed undergraduate students.
    pub q14: Vec<String>,
}
purrdf_lex::json_record!(Acceptance as "native university graph acceptance" {
    "profile" => profile: required, "receipt_blake3" => receipt_blake3: required,
    "converted_blake3" => converted_blake3: required, "files" => files: required,
    "statements" => statements: required, "q1" => q1: required, "q14" => q14: required,
});

fn digest(bytes: &[u8]) -> String {
    purrdf_hash::hex::Lower(purrdf_hash::blake3::hash(bytes).as_bytes()).to_string()
}
fn read(path: &Path) -> Result<Vec<u8>, String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if !metadata.is_file() || metadata.len() == 0 {
        return Err(format!(
            "{} must be a nonempty regular file, never a symlink",
            path.display()
        ));
    }
    fs::read(path).map_err(|error| format!("{}: {error}", path.display()))
}
fn parse(bytes: &[u8], mime: &str) -> Result<Arc<RdfDataset>, String> {
    purrdf_rdf::parse_dataset(bytes, mime, None).map_err(|error| error.to_string())
}
type Statements = FastSet<(TermValue, TermValue, TermValue)>;
fn statements(dataset: &RdfDataset) -> Result<Statements, String> {
    let mut result = Statements::default();
    for row in dataset.quads() {
        if row.g.is_some() {
            return Err(
                "native university documents must have only default-graph statements".into(),
            );
        }
        let s = dataset.term_value(row.s);
        let p = dataset.term_value(row.p);
        let o = dataset.term_value(row.o);
        if !matches!(s, TermValue::Iri(_)) || !matches!(p, TermValue::Iri(_)) {
            return Err("native university subjects and predicates must be named IRIs".into());
        }
        if !matches!(o, TermValue::Iri(_) | TermValue::Literal { .. }) {
            return Err("native university objects must be named IRIs or literals".into());
        }
        result.insert((s, p, o));
    }
    if result.is_empty() {
        return Err("empty native university graph".into());
    }
    Ok(result)
}
fn members(graph: &Statements, predicate: &str, object: &str) -> BTreeSet<String> {
    graph
        .iter()
        .filter_map(|(s, p, o)| match (s, p, o) {
            (TermValue::Iri(s), TermValue::Iri(p), TermValue::Iri(o))
                if p == predicate && o == object =>
            {
                Some(s.clone())
            }
            _ => None,
        })
        .collect()
}
fn line_count(bytes: &[u8]) -> Result<usize, String> {
    let text = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
    if !text.ends_with('\n') || text.lines().any(|line| line.trim().is_empty()) {
        return Err("native payload must have one nonempty complete statement per line".into());
    }
    Ok(text.lines().count())
}

/// Verify exact retained inventory, file bytes, per-file conversion and full graph union.
/// No expected answer is obtained from generator counters or a SPARQL engine.
///
/// # Errors
/// Refuses altered receipts, payloads, inventory, configurations, dropped/duplicated statements,
/// non-equivalent conversions and empty undergraduate answer sets.
pub fn verify(
    directory: &Path,
    converted: &Path,
    aggregate: &Path,
    spec: &Spec,
) -> Result<Acceptance, String> {
    Spec::new(
        spec.seed,
        spec.index,
        spec.universities,
        spec.ontology.clone(),
        spec.document_base.clone(),
    )?;
    let receipt_bytes = read(&directory.join("receipt.json"))?;
    let receipt: Receipt =
        purrdf_lex::json::record::from_slice(&receipt_bytes).map_err(|e| e.to_string())?;
    if receipt.profile != PROFILE
        || receipt.seed != spec.seed
        || receipt.index != spec.index
        || receipt.universities != spec.universities
        || receipt.ontology != spec.ontology
        || receipt.document_base != spec.document_base
    {
        return Err(
            "generation receipt does not match the admitted native profile and configuration"
                .into(),
        );
    }
    let mut expected = BTreeSet::from(["receipt.json".to_string()]);
    let mut expected_converted = BTreeSet::new();
    let mut union = Statements::default();
    let mut concatenated = purrdf_hash::blake3::Hasher::new();
    let mut departments = BTreeMap::<u64, BTreeSet<usize>>::new();
    let schema = format!("{}#", spec.ontology);
    let mut previous = "";
    for entry in &receipt.files {
        if entry.name.as_str() <= previous {
            return Err("receipt payload names must be unique and bytewise sorted".into());
        }
        previous = &entry.name;
        let tail = entry
            .name
            .strip_prefix("University")
            .and_then(|s| s.strip_suffix(".nt"))
            .ok_or("invalid receipt filename")?;
        let (university, department) = tail.split_once('_').ok_or("invalid department filename")?;
        let u: u64 = university
            .parse()
            .map_err(|_| "invalid university filename index")?;
        let d: usize = department
            .parse()
            .map_err(|_| "invalid department filename index")?;
        if entry.name != format!("University{u}_{d}.nt")
            || !(spec.index..spec.index + spec.universities).contains(&u)
        {
            return Err(
                "receipt filename outside admitted university range or not canonical".into(),
            );
        }
        departments.entry(u).or_default().insert(d);
        expected.insert(entry.name.clone());
        let bytes = read(&directory.join(&entry.name))?;
        if bytes.len() as u64 != entry.bytes || digest(&bytes) != entry.blake3 {
            return Err(format!("receipt byte identity mismatch for {}", entry.name));
        }
        let original = statements(parse(&bytes, "application/n-triples")?.as_ref())?;
        if line_count(&bytes)? != original.len() {
            return Err(format!(
                "duplicate or dropped native statement in {}",
                entry.name
            ));
        }
        let department_iri = format!("http://www.Department{d}.University{u}.edu");
        let document = format!("{}{}", spec.document_base, entry.name);
        for (s, p, o) in [
            (
                department_iri.clone(),
                rdf::TYPE.to_string(),
                format!("{schema}Department"),
            ),
            (
                department_iri,
                format!("{schema}subOrganizationOf"),
                format!("http://www.University{u}.edu"),
            ),
            (
                document.clone(),
                rdf::TYPE.to_string(),
                owl::ONTOLOGY.to_string(),
            ),
            (document, owl::IMPORTS.to_string(), spec.ontology.clone()),
        ] {
            if !original.contains(&(TermValue::Iri(s), TermValue::Iri(p), TermValue::Iri(o))) {
                return Err(format!(
                    "missing native department/document identity in {}",
                    entry.name
                ));
            }
        }
        let name = entry.name.replace(".nt", ".nq");
        expected_converted.insert(name.clone());
        let bytes = read(&converted.join(&name))?;
        concatenated.update(&bytes);
        let actual = statements(parse(&bytes, "application/n-quads")?.as_ref())?;
        if actual != original || line_count(&bytes)? != actual.len() {
            return Err(format!(
                "converted department graph differs from {}",
                entry.name
            ));
        }
        union.extend(original);
    }
    if departments.len() as u64 != spec.universities {
        return Err("native receipt omits an admitted university".into());
    }
    for actual in departments.into_values() {
        if !(15..=25).contains(&actual.len()) || actual != (0..actual.len()).collect() {
            return Err(
                "native university department inventory is incomplete or outside profile bounds"
                    .into(),
            );
        }
    }
    inventory(directory, &expected)?;
    inventory(converted, &expected_converted)?;
    let bytes = read(aggregate)?;
    let graph = statements(parse(&bytes, "application/n-quads")?.as_ref())?;
    if graph != union || purrdf_hash::blake3::hash(&bytes) != concatenated.finalize() {
        return Err(
            "aggregate converted bytes or graph differ from complete sorted department union"
                .into(),
        );
    }
    let q14 = members(&graph, rdf::TYPE, &format!("{schema}UndergraduateStudent"));
    if q14.is_empty() {
        return Err("native Q14 undergraduate graph oracle is vacuous".into());
    }
    let graduates = members(&graph, rdf::TYPE, &format!("{schema}GraduateStudent"));
    let takes_course = members(
        &graph,
        &format!("{schema}takesCourse"),
        "http://www.Department0.University0.edu/GraduateCourse0",
    );
    let q1 = graduates.intersection(&takes_course).cloned().collect();
    Ok(Acceptance {
        profile: PROFILE.into(),
        receipt_blake3: digest(&receipt_bytes),
        converted_blake3: digest(&bytes),
        files: receipt.files.len() as u64,
        statements: graph.len() as u64,
        q1,
        q14: q14.into_iter().collect(),
    })
}
fn inventory(directory: &Path, expected: &BTreeSet<String>) -> Result<(), String> {
    let actual = fs::read_dir(directory)
        .map_err(|e| e.to_string())?
        .map(|entry| {
            entry.map_err(|e| e.to_string()).and_then(|entry| {
                entry
                    .file_name()
                    .into_string()
                    .map_err(|_| "non-UTF8 inventory entry".to_string())
            })
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if &actual != expected {
        return Err(format!(
            "{} inventory differs from exact native receipt",
            directory.display()
        ));
    }
    Ok(())
}
/// Persist a fresh acceptance document only after all graph checks succeed.
///
/// # Errors
/// Refuses an existing output and failed writes, flushes or synchronization.
pub fn write_acceptance(path: &Path, acceptance: &Acceptance) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(&purrdf_lex::json::record::to_vec(acceptance))
        .and_then(|()| file.write_all(b"\n"))
        .and_then(|()| file.flush())
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())
}
/// Validate actual SPARQL JSON and, for Q1/Q14, compare exact URI answer sets.
///
/// # Errors
/// Refuses malformed results, duplicate/unbound/non-URI oracle cells and wrong answers.
pub fn results(bytes: &[u8], oracle: Option<(&Acceptance, &str)>) -> Result<usize, String> {
    let results = purrdf_sparql_results::from_json(bytes).map_err(|e| e.to_string())?;
    if let Some((acceptance, id)) = oracle {
        let expected = match id {
            "Q1" => &acceptance.q1,
            "Q14" => &acceptance.q14,
            _ => return Err("unknown native graph oracle".into()),
        };
        if results.variables != ["X"] {
            return Err("oracle result projection must be exactly X".into());
        }
        let mut actual = BTreeSet::new();
        for row in &results.rows {
            let [Some(TermValue::Iri(iri))] = row.as_slice() else {
                return Err("oracle answer must bind X to one URI".into());
            };
            if !actual.insert(iri.clone()) {
                return Err("duplicate native oracle answer".into());
            }
        }
        if actual != expected.iter().cloned().collect() {
            return Err(format!(
                "{id} actual URI answer set differs from independent full-graph oracle"
            ));
        }
    }
    Ok(results.rows.len())
}
fn project_iri(iri: &str, ontology: &str) -> String {
    if iri == EXTERNAL_ONTOLOGY {
        ontology.into()
    } else if let Some(local) = iri.strip_prefix(&format!("{EXTERNAL_ONTOLOGY}#")) {
        format!("{ontology}#{local}")
    } else {
        iri.into()
    }
}
fn project_term(term: &RdfTerm, ontology: &str) -> Result<RdfTerm, String> {
    term.try_fold(
        |leaf| {
            Ok(match leaf {
                RdfTerm::Iri(iri) => RdfTerm::iri(project_iri(iri, ontology)),
                RdfTerm::Literal(literal) => {
                    let mut literal = literal.clone();
                    literal.datatype = literal.datatype.map(|iri| project_iri(&iri, ontology));
                    RdfTerm::literal(literal)
                }
                RdfTerm::BlankNode(_) => leaf.clone(),
                RdfTerm::Triple(_) => {
                    return Err("triple term unexpectedly reached projection leaf".into());
                }
            })
        },
        |iri| Ok(RdfTerm::iri(project_iri(iri, ontology))),
        |s, p, o| {
            let RdfTerm::Iri(predicate) = p else {
                return Err("projected predicate is not an IRI".into());
            };
            Ok(RdfTerm::triple(RdfTriple::new(s, predicate, o)))
        },
    )
}
/// Mechanically project all ontology RDF positions through the actual model.
/// External input is never edited; W3C/foreign IRIs and literal lexical bytes stay intact.
///
/// # Errors
/// Refuses invalid RDF and failed model construction/serialization.
pub fn project(bytes: &[u8], ontology: &str) -> Result<Vec<u8>, String> {
    Spec::ontology_identity(ontology)?;
    let original = parse(bytes, "application/n-quads")?;
    if original.quads().count() == 0 {
        return Err("empty external ontology".into());
    }
    let mut builder = RdfDatasetBuilder::new();
    for mut quad in original.owned_quads() {
        quad.subject = project_term(&quad.subject, ontology)?;
        quad.predicate = project_iri(&quad.predicate, ontology);
        quad.object = project_term(&quad.object, ontology)?;
        quad.graph_name = quad
            .graph_name
            .as_ref()
            .map(|term| project_term(term, ontology))
            .transpose()?;
        builder.push_owned_quad(&quad);
    }
    let projected = builder.freeze().map_err(|e| e.to_string())?;
    purrdf_rdf::serialize_dataset_to_format(projected.as_ref(), NativeRdfFormat::NQuads, None)
        .map(|outcome| outcome.bytes)
        .map_err(|e| e.to_string())
}

/// Admit an output directory before retaining externally sourced schema/query bytes.
/// Outside Git worktrees it is owned output; inside any worktree its whole directory
/// must be ignored by a tracked .gitignore, never a personal exclusion rule.
///
/// # Errors
/// Refuses inaccessible paths, broken Git state and repo-visible output directories.
pub fn admit_output(directory: &Path) -> Result<(), String> {
    let directory = fs::canonicalize(directory).map_err(|e| e.to_string())?;
    if !directory.is_dir() {
        return Err("native output parent must be a directory".into());
    }
    let mut repository = None;
    for ancestor in directory.ancestors() {
        if ancestor
            .join(".git")
            .try_exists()
            .map_err(|e| e.to_string())?
        {
            repository = Some(ancestor);
            break;
        }
    }
    let Some(repository) = repository else {
        return Ok(());
    };
    let relative = directory
        .strip_prefix(repository)
        .map_err(|e| e.to_string())?;
    if relative.as_os_str().is_empty() {
        return Err("external artifacts cannot be written into a repository root".into());
    }
    let path = format!(
        "{}/",
        relative
            .to_str()
            .ok_or("native output path must be UTF-8")?
            .replace(std::path::MAIN_SEPARATOR, "/")
    );
    let mut child = std::process::Command::new("git")
        .arg("-C")
        .arg(repository)
        .args([
            "-c",
            "core.excludesFile=/dev/null",
            "check-ignore",
            "--verbose",
            "--no-index",
            "-z",
            "--stdin",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut input = child.stdin.take().ok_or("Git ignore check has no stdin")?;
    input
        .write_all(path.as_bytes())
        .and_then(|()| input.write_all(&[0]))
        .map_err(|e| e.to_string())?;
    drop(input);
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "LUBM_OUT is not ignored by source-controlled rules: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let fields = output.stdout.split(|byte| *byte == 0).collect::<Vec<_>>();
    let [source, _, pattern, actual, empty] = fields.as_slice() else {
        return Err("invalid Git ignore provenance record".into());
    };
    if pattern.starts_with(b"!") {
        return Err("the output directory is re-included by its matching ignore rule".into());
    }
    if !empty.is_empty() || *actual != path.as_bytes() {
        return Err("Git ignore provenance does not name the output directory".into());
    }
    let source = std::str::from_utf8(source).map_err(|e| e.to_string())?;
    if Path::new(source).file_name() != Some(std::ffi::OsStr::new(".gitignore")) {
        return Err("personal Git exclusions do not admit external benchmark output".into());
    }
    let tracked = std::process::Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(["ls-files", "--error-unmatch", "--", source])
        .output()
        .map_err(|e| e.to_string())?;
    if !tracked.status.success() {
        return Err("the output ignore rule must come from a tracked .gitignore".into());
    }
    Ok(())
}
