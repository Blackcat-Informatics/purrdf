// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! ShEx 2.1 validation conformance over the vendored shexTest corpus
//! (`vectors/shexTest/validation/manifest.ttl`, upstream tag v2.1.0).
//!
//! The manifest is parsed as Turtle (via the `purrdf-rdf` dev-dependency);
//! each `sht:ValidationTest` / `sht:ValidationFailure` entry loads its
//! schema (ShExC, with the schema URL as base), its data graph (Turtle,
//! with the data URL as base) and its focus/shape (or shape-map JSON), runs
//! [`purrdf_shex::validate`], and compares the verdict.
//!
//! * **SKIP** (a counted category): entries whose traits demand machinery
//!   this engine deliberately does not ship — `SemanticAction`, `Extends`,
//!   `ExtendsDiamond`. Nothing else is skipped; `Greedy`, `Exhaustive` and
//!   `OutsideBMP` entries are attempted, and `Import` is now resolved.
//! * **XFAIL**: genuine engine gaps, listed exactly (name + reason). A
//!   passing xfail fails the harness (a stale ledger is a test error).

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use purrdf_rdf::{parse_dataset, DatasetView, GraphMatch, RdfDataset, TermId, TermValue};
use purrdf_shex::{
    parse_shexc, parse_shexj, resolve_imports, validate, ConformanceStatus, Schema, ShapeSelector,
};

/// The corpus is byte-frozen; drift in the entry count means the vectors
/// were touched, which this harness must notice.
const ENTRY_COUNT: usize = 1105;

/// The URL prefix the vendored tree mirrors.
const CORPUS_URL: &str = "https://raw.githubusercontent.com/shexSpec/shexTest/master/";

/// Traits this engine deliberately does not implement (skipped, counted).
const SKIP_TRAITS: &[&str] = &["SemanticAction", "Extends", "ExtendsDiamond"];

/// Genuine engine gaps: entries expected to produce the WRONG verdict, each
/// with a reason. A passing xfail fails the harness.
const XFAIL: &[(&str, &str)] = &[];

// ── vocabulary ──────────────────────────────────────────────────────────────

const MF: &str = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#";
const SHT: &str = "http://www.w3.org/ns/shacl/test-suite#";
const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vectors/shexTest")
}

fn url_to_path(url: &str) -> PathBuf {
    let rest = url
        .strip_prefix(CORPUS_URL)
        .unwrap_or_else(|| panic!("URL outside the vendored corpus: {url}"));
    corpus_dir().join(rest)
}

// ── manifest access ─────────────────────────────────────────────────────────

struct Manifest {
    ds: Arc<RdfDataset>,
}

impl Manifest {
    fn load() -> Self {
        let path = corpus_dir().join("validation/manifest.ttl");
        let text = fs::read_to_string(&path).expect("read validation manifest");
        let ds = parse_dataset(
            text.as_bytes(),
            "text/turtle",
            Some(&format!("{CORPUS_URL}validation/manifest")),
        )
        .expect("parse validation manifest");
        Self { ds }
    }

    fn named(&self, iri: &str) -> Option<TermId> {
        self.ds.term_id_by_value(&TermValue::iri(iri))
    }

    fn objects(&self, s: TermId, p: &str) -> Vec<TermId> {
        let Some(pid) = self.named(p) else {
            return Vec::new();
        };
        self.ds
            .quads_for_pattern(Some(s), Some(pid), None, GraphMatch::Any)
            .map(|q| q.o)
            .collect()
    }

    fn object(&self, s: TermId, p: &str) -> Option<TermId> {
        self.objects(s, p).into_iter().next()
    }

    fn iri(&self, id: TermId) -> Option<String> {
        match self.ds.term_value(id) {
            TermValue::Iri(iri) => Some(iri),
            _ => None,
        }
    }

    fn lexical(&self, id: TermId) -> Option<String> {
        match self.ds.term_value(id) {
            TermValue::Literal { lexical_form, .. } => Some(lexical_form),
            _ => None,
        }
    }

    /// Walk the `mf:entries` rdf:List.
    fn entries(&self) -> Vec<TermId> {
        let entries_pred = self
            .named(&format!("{MF}entries"))
            .expect("mf:entries predicate");
        let mut head = self
            .ds
            .quads_for_pattern(None, Some(entries_pred), None, GraphMatch::Any)
            .map(|q| q.o)
            .next()
            .expect("manifest entry list");
        let nil = self.named(&format!("{RDF}nil"));
        let mut out = Vec::new();
        loop {
            if Some(head) == nil {
                break;
            }
            let Some(first) = self.object(head, &format!("{RDF}first")) else {
                break;
            };
            out.push(first);
            let Some(rest) = self.object(head, &format!("{RDF}rest")) else {
                break;
            };
            head = rest;
        }
        out
    }
}

// ── entry model ─────────────────────────────────────────────────────────────

struct Entry {
    name: String,
    /// `true` for `sht:ValidationTest`, `false` for `sht:ValidationFailure`.
    expect_conformant: bool,
    /// Trait local names (`Import`, `Stem`, …).
    traits: Vec<String>,
    schema_url: String,
    data_url: String,
    /// `None` = the schema's START shape.
    shape: Option<String>,
    focus: Option<TermValue>,
    map_url: Option<String>,
    result_url: Option<String>,
}

fn read_entry(m: &Manifest, id: TermId) -> Entry {
    let name = m
        .object(id, &format!("{MF}name"))
        .and_then(|o| m.lexical(o))
        .expect("mf:name");
    let types: Vec<String> = m
        .objects(id, &format!("{RDF}type"))
        .into_iter()
        .filter_map(|t| m.iri(t))
        .collect();
    let expect_conformant = if types.iter().any(|t| t == &format!("{SHT}ValidationTest")) {
        true
    } else {
        assert!(
            types
                .iter()
                .any(|t| t == &format!("{SHT}ValidationFailure")),
            "{name}: unexpected entry type {types:?}"
        );
        false
    };
    let traits: Vec<String> = m
        .objects(id, &format!("{SHT}trait"))
        .into_iter()
        .filter_map(|t| m.iri(t))
        .filter_map(|iri| iri.strip_prefix(SHT).map(str::to_owned))
        .collect();
    let action = m.object(id, &format!("{MF}action")).expect("mf:action");
    let schema_url = m
        .object(action, &format!("{SHT}schema"))
        .and_then(|o| m.iri(o))
        .expect("sht:schema");
    let data_url = m
        .object(action, &format!("{SHT}data"))
        .and_then(|o| m.iri(o))
        .expect("sht:data");
    let shape = m
        .object(action, &format!("{SHT}shape"))
        .map(|o| match m.ds.term_value(o) {
            TermValue::Iri(iri) => iri,
            TermValue::Blank { label, .. } => format!("_:{label}"),
            other => panic!("{name}: unsupported sht:shape term {other:?}"),
        });
    let focus = m
        .object(action, &format!("{SHT}focus"))
        .map(|o| m.ds.term_value(o));
    let map_url = m
        .object(action, &format!("{SHT}map"))
        .and_then(|o| m.iri(o));
    let result_url = m.object(id, &format!("{MF}result")).and_then(|o| m.iri(o));
    Entry {
        name,
        expect_conformant,
        traits,
        schema_url,
        data_url,
        shape,
        focus,
        map_url,
        result_url,
    }
}

// ── caches ──────────────────────────────────────────────────────────────────

#[derive(Default)]
struct Caches {
    schemas: HashMap<String, Result<Arc<Schema>, String>>,
    data: HashMap<String, Result<Arc<RdfDataset>, String>>,
}

/// Read one schema document, choosing ShExC/ShExJ by the on-disk extension
/// and parsing with `url` as base. An import IRI carries no extension, so
/// `.shex` then `.json` are tried; a schema URL names the file directly.
fn read_schema(url: &str) -> Result<Schema, String> {
    let base = url_to_path(url);
    let candidates: Vec<PathBuf> = if base.extension().is_some() {
        vec![base]
    } else {
        vec![base.with_extension("shex"), base.with_extension("json")]
    };
    for path in candidates {
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        return if path.extension().is_some_and(|x| x == "json") {
            parse_shexj(&source).map_err(|e| e.to_string())
        } else {
            parse_shexc(&source, Some(url)).map_err(|e| e.to_string())
        };
    }
    Err(format!("no schema document for {url}"))
}

/// Load a schema and fold in its transitive imports. The import resolver reads
/// each imported IRI from the vendored corpus, parsing it with its own IRI as
/// base (per-document base resolution).
fn load_schema(url: &str) -> Result<Schema, String> {
    let root = read_schema(url)?;
    resolve_imports(root, &|iri| read_schema(iri).ok()).map_err(|e| e.to_string())
}

impl Caches {
    fn schema(&mut self, url: &str) -> Result<Arc<Schema>, String> {
        self.schemas
            .entry(url.to_owned())
            .or_insert_with(|| load_schema(url).map(Arc::new))
            .clone()
    }

    fn dataset(&mut self, url: &str) -> Result<Arc<RdfDataset>, String> {
        self.data
            .entry(url.to_owned())
            .or_insert_with(|| {
                let path = url_to_path(url);
                let bytes = fs::read(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
                parse_dataset(&bytes, "text/turtle", Some(url)).map_err(|e| e.to_string())
            })
            .clone()
    }
}

// ── shape-map JSON entries ──────────────────────────────────────────────────

fn json_node_to_value(node: &str) -> TermValue {
    if let Some(label) = node.strip_prefix("_:") {
        TermValue::blank(label)
    } else {
        TermValue::iri(node)
    }
}

/// Run a `sht:map`-driven entry: validate every association in the map JSON
/// and compare each verdict to the results JSON.
fn run_shape_map_entry(entry: &Entry, caches: &mut Caches) -> Result<(), String> {
    let schema = caches.schema(&entry.schema_url)?;
    let data = caches.dataset(&entry.data_url)?;
    let map_url = entry.map_url.as_deref().expect("map url");
    let map_text =
        fs::read_to_string(url_to_path(map_url)).map_err(|e| format!("read map: {e}"))?;
    let map_json: serde_json::Value =
        serde_json::from_str(&map_text).map_err(|e| format!("map JSON: {e}"))?;
    let mut associations = Vec::new();
    for pair in map_json.as_array().ok_or("map JSON is not an array")? {
        let node = pair["node"].as_str().ok_or("map node")?;
        let shape = pair["shape"].as_str().ok_or("map shape")?;
        associations.push((
            json_node_to_value(node),
            ShapeSelector::Label(shape.to_owned()),
        ));
    }
    let outcome = validate(&schema, &data, &associations);

    let result_url = entry
        .result_url
        .as_deref()
        .ok_or("shape-map entry without mf:result")?;
    let result_text =
        fs::read_to_string(url_to_path(result_url)).map_err(|e| format!("read result: {e}"))?;
    let result_json: serde_json::Value =
        serde_json::from_str(&result_text).map_err(|e| format!("result JSON: {e}"))?;
    for (index, (node, selector)) in associations.iter().enumerate() {
        let node_key = match node {
            TermValue::Iri(iri) => iri.clone(),
            TermValue::Blank { label, .. } => format!("_:{label}"),
            other => return Err(format!("unsupported map node {other:?}")),
        };
        let ShapeSelector::Label(shape_key) = selector else {
            return Err("START in a map entry".to_owned());
        };
        let expected = result_json[&node_key]
            .as_array()
            .and_then(|rows| {
                rows.iter()
                    .find(|row| row["shape"].as_str() == Some(shape_key))
            })
            .and_then(|row| row["result"].as_bool())
            .ok_or_else(|| format!("no expected result for {node_key} / {shape_key}"))?;
        let got = outcome.entries[index].status == ConformanceStatus::Conformant;
        if got != expected {
            return Err(format!(
                "{node_key} @ {shape_key}: expected {expected}, got {got} ({})",
                outcome.entries[index]
                    .reason
                    .as_deref()
                    .unwrap_or("conformant")
            ));
        }
    }
    Ok(())
}

/// Run a focus/shape entry; `Ok(())` when the verdict matches expectation.
fn run_focus_entry(entry: &Entry, caches: &mut Caches) -> Result<(), String> {
    let schema = caches.schema(&entry.schema_url)?;
    let data = caches.dataset(&entry.data_url)?;
    let focus = entry.focus.clone().ok_or("entry without sht:focus")?;
    let selector = entry
        .shape
        .clone()
        .map_or(ShapeSelector::Start, ShapeSelector::Label);
    let outcome = validate(&schema, &data, &[(focus, selector)]);
    let conformant = outcome.all_conformant();
    if conformant == entry.expect_conformant {
        Ok(())
    } else if entry.expect_conformant {
        Err(format!(
            "expected conformant, got: {}",
            outcome.entries[0].reason.as_deref().unwrap_or("?")
        ))
    } else {
        Err("expected nonconformant, but the node conformed".to_owned())
    }
}

// ── the harness ─────────────────────────────────────────────────────────────

#[test]
fn validation_conformance() {
    let manifest = Manifest::load();
    let ids = manifest.entries();
    assert_eq!(ids.len(), ENTRY_COUNT, "corpus drift in the manifest");

    let xfail: BTreeMap<&str, &str> = XFAIL.iter().copied().collect();
    let mut caches = Caches::default();

    let mut passed = 0usize;
    let mut xfailed: Vec<&str> = Vec::new();
    let mut stale_xfails: Vec<String> = Vec::new();
    let mut unexpected: Vec<String> = Vec::new();
    let mut skipped: BTreeMap<String, usize> = BTreeMap::new();
    let mut trait_totals: BTreeMap<String, (usize, usize)> = BTreeMap::new();

    for id in ids {
        let entry = read_entry(&manifest, id);
        if let Some(skip) = entry
            .traits
            .iter()
            .find(|t| SKIP_TRAITS.contains(&t.as_str()))
        {
            *skipped.entry(skip.clone()).or_default() += 1;
            continue;
        }
        let outcome = if entry.map_url.is_some() {
            run_shape_map_entry(&entry, &mut caches)
        } else {
            run_focus_entry(&entry, &mut caches)
        };
        let ok = outcome.is_ok();
        for t in &entry.traits {
            let slot = trait_totals.entry(t.clone()).or_default();
            slot.0 += 1;
            if ok {
                slot.1 += 1;
            }
        }
        match (ok, xfail.contains_key(entry.name.as_str())) {
            (true, false) => passed += 1,
            (false, true) => {
                xfailed.push(xfail.get_key_value(entry.name.as_str()).expect("xfail").0);
            }
            (true, true) => stale_xfails.push(entry.name.clone()),
            (false, false) => unexpected.push(format!(
                "{}: {}",
                entry.name,
                outcome.expect_err("failed outcome")
            )),
        }
    }

    // Scoreboard.
    let attempted = passed + xfailed.len() + stale_xfails.len() + unexpected.len();
    let skipped_total: usize = skipped.values().sum();
    let mut board = String::new();
    let _ = writeln!(board, "shexTest validation scoreboard:");
    let _ = writeln!(
        board,
        "  entries {ENTRY_COUNT} | attempted {attempted} | pass {passed} | xfail {} | fail {} | skipped {skipped_total}",
        xfailed.len(),
        unexpected.len(),
    );
    for (trait_name, count) in &skipped {
        let _ = writeln!(board, "  skip[{trait_name}] = {count}");
    }
    for (trait_name, (total, ok)) in &trait_totals {
        let _ = writeln!(board, "  trait[{trait_name}] = {ok}/{total}");
    }
    println!("{board}");

    assert!(
        stale_xfails.is_empty(),
        "XFAIL entries now pass (remove them from the ledger): {stale_xfails:?}\n{board}"
    );
    assert!(
        unexpected.is_empty(),
        "{} entries produced the wrong verdict and are not in the XFAIL ledger:\n{}\n{board}",
        unexpected.len(),
        unexpected.join("\n"),
    );
    assert_eq!(
        xfailed.len(),
        XFAIL.len(),
        "some XFAIL ledger entries were skipped instead of failing\n{board}"
    );
}
