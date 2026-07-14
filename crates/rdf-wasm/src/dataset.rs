// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The RDF/JS [DatasetCore](https://rdf.js.org/dataset-spec/#datasetcore-interface) —
//! an in-memory, mutable quad collection.
//!
//! Wraps the engine's COW [`MutableDataset`](purrdf::ir::MutableDataset): a shared
//! frozen base plus an append/suppress delta. `parse` builds a frozen base from text
//! and wraps it; `serialize` compacts the effective set (`freeze`) and emits it;
//! `add`/`delete`/`has`/`match`/`quads` are the RDF/JS `DatasetCore` mutation + query
//! surface over the COW delta.

use purrdf::dataset_view::{DatasetMut, GraphMatchValue};
use purrdf::ir::MutableDataset;
use purrdf::{
    RdfDatasetBuilder, RdfDiagnostic, SerializeGraph, TermValue, canonical_flat_nquads,
    datasets_isomorphic, parse_dataset, serialize_dataset,
};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

use purrdf::viz::{
    VizGraphPolicy, VizLabelPolicy, VizLayoutOptions, VizMode, VizRenderOptions, VizRole,
    VizRoleRule, VizSpec, VizSvgOptions, VizTableField, VizVocabularyMapping, export_json,
    project_dataset, project_dataset_export, render_dataset_svg,
};

use crate::codec::resolve_media_type;
use crate::convert::{quad_to_quad_values, quad_values_to_quad, rdf_term_to_term_value};
use crate::term::{Quad, Term, TermInner};

/// Lower an optional pattern [`Term`] to an optional [`TermValue`] (None = wildcard).
///
/// A `Variable` term is the RDF/JS idiom for a wildcard position in `match()` (the same
/// role an omitted/`undefined` argument plays), so it lowers to `None` rather than
/// erroring — only a concrete RDF term constrains the position.
fn pattern_value(term: Option<&Term>) -> Result<Option<TermValue>, JsError> {
    match term {
        None => Ok(None),
        Some(t) if matches!(t.inner, TermInner::Variable(_)) => Ok(None),
        Some(t) => {
            let rdf = t.to_rdf_term().map_err(|e| JsError::new(&e))?;
            Ok(Some(rdf_term_to_term_value(&rdf)))
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
struct VisualOptions {
    mode: Option<VizMode>,
    focus: Option<String>,
    role_rules: Vec<VisualRoleRule>,
    vocabulary: Vec<VizVocabularyMapping>,
    graph: Option<String>,
    graphs: Vec<String>,
    label_policy: Option<VizLabelPolicy>,
    max_statements: Option<usize>,
    max_terms: Option<usize>,
    table_fields: Option<Vec<VizTableField>>,
    layout: VisualLayoutOptions,
    svg: VisualSvgOptions,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct VisualRoleRule {
    predicate_iri: String,
    role: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
struct VisualLayoutOptions {
    margin: Option<i32>,
    rank_spacing: Option<i32>,
    node_spacing: Option<i32>,
    component_spacing: Option<i32>,
    component_wrap_width: Option<i32>,
    crossing_sweeps: Option<u32>,
    max_node_width: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
struct VisualSvgOptions {
    embed_metadata: Option<bool>,
    include_styles: Option<bool>,
    title: Option<String>,
}

impl VisualOptions {
    fn parse(json: Option<String>) -> Result<Self, JsError> {
        json.filter(|value| !value.trim().is_empty()).map_or_else(
            || Ok(Self::default()),
            |value| serde_json::from_str(&value).map_err(|error| JsError::new(&error.to_string())),
        )
    }

    fn into_engine_options(self) -> Result<(VizSpec, VizRenderOptions), JsError> {
        if self.graph.is_some() && !self.graphs.is_empty() {
            return Err(JsError::new(
                "visualization options cannot set both graph and graphs",
            ));
        }
        let mut spec = VizSpec::default();
        if let Some(mode) = self.mode {
            spec.mode = mode;
        }
        spec.focus = self.focus;
        spec.role_rules = self
            .role_rules
            .into_iter()
            .map(|rule| VizRoleRule {
                predicate_iri: rule.predicate_iri,
                role: VizRole::Custom(rule.role),
            })
            .collect();
        spec.vocabulary = self.vocabulary;
        let graph_selectors = self
            .graph
            .into_iter()
            .chain(self.graphs)
            .collect::<Vec<_>>();
        if !graph_selectors.is_empty() {
            spec.graph_policy = VizGraphPolicy::Include(graph_selectors);
        }
        if let Some(label_policy) = self.label_policy {
            spec.label_policy = label_policy;
        }
        if let Some(max_statements) = self.max_statements {
            spec.max_statements = max_statements;
        }
        if let Some(max_terms) = self.max_terms {
            spec.max_terms = max_terms;
        }
        if let Some(table_fields) = self.table_fields {
            spec.table_fields = table_fields;
        }

        let mut layout = VizLayoutOptions::default();
        macro_rules! apply_layout {
            ($field:ident) => {
                if let Some(value) = self.layout.$field {
                    layout.$field = value;
                }
            };
        }
        apply_layout!(margin);
        apply_layout!(rank_spacing);
        apply_layout!(node_spacing);
        apply_layout!(component_spacing);
        apply_layout!(component_wrap_width);
        apply_layout!(crossing_sweeps);
        apply_layout!(max_node_width);

        let mut svg = VizSvgOptions::default();
        if let Some(embed_metadata) = self.svg.embed_metadata {
            svg.embed_metadata = embed_metadata;
        }
        if let Some(include_styles) = self.svg.include_styles {
            svg.include_styles = include_styles;
        }
        if let Some(title) = self.svg.title {
            svg.title = title;
        }
        Ok((spec, VizRenderOptions { layout, svg }))
    }
}

/// Map an engine diagnostic to a JS error.
pub(crate) fn diag_to_err(diag: &RdfDiagnostic) -> JsError {
    JsError::new(&diag.to_string())
}

/// An RDF/JS `DatasetCore` backed by the engine's COW mutable dataset.
#[wasm_bindgen]
#[derive(Debug)]
pub struct Dataset {
    pub(crate) inner: MutableDataset,
}

impl Dataset {
    /// An empty frozen base — the COW root for a dataset with no parsed content.
    fn empty_base() -> Result<MutableDataset, JsError> {
        let base = RdfDatasetBuilder::new()
            .freeze()
            .map_err(|e| diag_to_err(&e))?;
        Ok(MutableDataset::new(base))
    }
}

#[wasm_bindgen]
impl Dataset {
    /// An empty dataset.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Self, JsError> {
        Ok(Self {
            inner: Self::empty_base()?,
        })
    }

    /// `parse(input, format, base?)` → a dataset of the parsed quads.
    ///
    /// `format` is a media type or short name (turtle/ntriples/nquads/trig/rdfxml).
    /// Ill-typed literals are preserved verbatim (RDFLib parity), not rejected.
    #[wasm_bindgen(js_name = parse)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn parse(input: &str, format: &str, base: Option<String>) -> Result<Self, JsError> {
        let media_type = resolve_media_type(format).map_err(|e| JsError::new(&e))?;
        let dataset = parse_dataset(input.as_bytes(), media_type, base.as_deref())
            .map_err(|e| diag_to_err(&e))?;
        Ok(Self {
            inner: MutableDataset::new(dataset),
        })
    }

    /// `serialize(format)` → the dataset rendered in `format` (a UTF-8 string).
    ///
    /// Formats: `turtle` / `ntriples` / `nquads` / `trig` / `rdfxml` / `jsonld`
    /// (JSON-LD-star) / `yamlld` (YAML-LD-star), and their media types — all resolved
    /// through the one core registry.
    #[wasm_bindgen(js_name = serialize)]
    pub fn serialize(&self, format: &str) -> Result<String, JsError> {
        let frozen = self.inner.freeze().map_err(|e| diag_to_err(&e))?;
        let media_type = resolve_media_type(format).map_err(|e| JsError::new(&e))?;
        let bytes = serialize_dataset(&frozen, media_type, SerializeGraph::Dataset)
            .map_err(|e| diag_to_err(&e))?;
        String::from_utf8(bytes)
            .map_err(|e| JsError::new(&format!("serialization produced non-UTF-8 bytes: {e}")))
    }

    /// `canonicalize()` → the dataset as canonical, flat N-Quads under RDFC-1.0
    /// (SHA-256).
    ///
    /// The deterministic identity string for the graph: two datasets denote the same
    /// RDF graph (under blank-node relabeling) iff their canonical forms are
    /// byte-identical. This is the same RDFC-1.0 output the conformance gate pins.
    #[wasm_bindgen(js_name = canonicalize)]
    pub fn canonicalize(&self) -> Result<String, JsError> {
        let frozen = self.inner.freeze().map_err(|e| diag_to_err(&e))?;
        canonical_flat_nquads(&frozen).map_err(|e| JsError::new(&e))
    }

    /// `isomorphic(other)` → whether this dataset and `other` are the same RDF graph
    /// under blank-node relabeling.
    ///
    /// The formal RDF graph-identity check, backed by full RDFC-1.0 canonicalization:
    /// an exact oracle with no false positives or false negatives. Equivalent to
    /// comparing the two [`canonicalize`](Self::canonicalize) strings, but avoids
    /// materializing them for obviously-different inputs.
    #[wasm_bindgen(js_name = isomorphic)]
    pub fn isomorphic(&self, other: &Self) -> Result<bool, JsError> {
        let a = self.inner.freeze().map_err(|e| diag_to_err(&e))?;
        let b = other.inner.freeze().map_err(|e| diag_to_err(&e))?;
        Ok(datasets_isomorphic(&a, &b))
    }

    /// `size` — the number of effective quads.
    #[wasm_bindgen(getter)]
    pub fn size(&self) -> usize {
        self.inner.effective_count()
    }

    /// `add(quad)` → insert a quad. Returns `true` if the effective set changed.
    #[wasm_bindgen(js_name = add)]
    pub fn add(&mut self, quad: &Quad) -> Result<bool, JsError> {
        let values = quad_to_quad_values(quad).map_err(|e| JsError::new(&e))?;
        Ok(self.inner.insert(values))
    }

    /// `delete(quad)` → remove a quad. Returns `true` if the effective set changed.
    #[wasm_bindgen(js_name = delete)]
    pub fn delete(&mut self, quad: &Quad) -> Result<bool, JsError> {
        let values = quad_to_quad_values(quad).map_err(|e| JsError::new(&e))?;
        Ok(self.inner.remove(&values))
    }

    /// `has(quad)` → whether the quad is in the dataset.
    #[wasm_bindgen(js_name = has)]
    pub fn has(&self, quad: &Quad) -> Result<bool, JsError> {
        let values = quad_to_quad_values(quad).map_err(|e| JsError::new(&e))?;
        Ok(self.inner.contains(&values))
    }

    /// `quads()` → every effective quad, as a JS array.
    #[wasm_bindgen(js_name = quads)]
    pub fn quads(&self) -> Result<Vec<Quad>, JsError> {
        self.inner
            .quads_for_pattern(None, None, None, GraphMatchValue::Any)
            .iter()
            .map(|qv| quad_values_to_quad(qv).map_err(|e| JsError::new(&e)))
            .collect()
    }

    /// `visualModelJson(optionsJson?)` -> the renderer-neutral RDF 1.2 model as JSON.
    #[wasm_bindgen(js_name = visualModelJson)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn visual_model_json(&self, options_json: Option<String>) -> Result<String, JsError> {
        let (spec, _) = VisualOptions::parse(options_json)?.into_engine_options()?;
        let frozen = self.inner.freeze().map_err(|error| diag_to_err(&error))?;
        let model =
            project_dataset(&frozen, &spec).map_err(|error| JsError::new(&error.to_string()))?;
        serde_json::to_string(&model).map_err(|error| JsError::new(&error.to_string()))
    }

    /// `visualExportJson(optionsJson?)` -> model, scene, geometry, and index as JSON.
    #[wasm_bindgen(js_name = visualExportJson)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn visual_export_json(&self, options_json: Option<String>) -> Result<String, JsError> {
        let (spec, options) = VisualOptions::parse(options_json)?.into_engine_options()?;
        let frozen = self.inner.freeze().map_err(|error| diag_to_err(&error))?;
        let export = project_dataset_export(&frozen, &spec, &options.layout)
            .map_err(|error| JsError::new(&error.to_string()))?;
        export_json(&export).map_err(|error| JsError::new(&error.to_string()))
    }

    /// `visualSvgJson(optionsJson?)` -> deterministic SVG and its complete export.
    #[wasm_bindgen(js_name = visualSvgJson)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn visual_svg_json(&self, options_json: Option<String>) -> Result<String, JsError> {
        let (spec, options) = VisualOptions::parse(options_json)?.into_engine_options()?;
        let frozen = self.inner.freeze().map_err(|error| diag_to_err(&error))?;
        let document = render_dataset_svg(&frozen, &spec, &options)
            .map_err(|error| JsError::new(&error.to_string()))?;
        serde_json::to_string(&document).map_err(|error| JsError::new(&error.to_string()))
    }

    /// `match(subject?, predicate?, object?, graph?)` → a new dataset of the matching
    /// quads. An omitted (`undefined`) position is a wildcard; `defaultGraph()` matches
    /// only the default graph, a named node matches that graph.
    #[wasm_bindgen(js_name = match)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn match_pattern(
        &self,
        subject: Option<Term>,
        predicate: Option<Term>,
        object: Option<Term>,
        graph: Option<Term>,
    ) -> Result<Self, JsError> {
        let s = pattern_value(subject.as_ref())?;
        let p = pattern_value(predicate.as_ref())?;
        let o = pattern_value(object.as_ref())?;
        // The graph slot needs the three-way Any / Default / Named distinction that a
        // bare Option<TermValue> cannot express. A `Variable` graph term is a wildcard
        // (`Any`), like an omitted argument — never resolved as a named graph.
        let named_graph = match &graph {
            Some(t) if !matches!(t.inner, TermInner::DefaultGraph | TermInner::Variable(_)) => {
                Some(rdf_term_to_term_value(
                    &t.to_rdf_term().map_err(|e| JsError::new(&e))?,
                ))
            }
            _ => None,
        };
        let graph_match = match &graph {
            None => GraphMatchValue::Any,
            Some(t) if matches!(t.inner, TermInner::DefaultGraph) => GraphMatchValue::Default,
            Some(t) if matches!(t.inner, TermInner::Variable(_)) => GraphMatchValue::Any,
            Some(_) => GraphMatchValue::Named(named_graph.as_ref().expect(
                "a named-graph value is computed for a non-default, non-variable graph term",
            )),
        };
        let matched = self
            .inner
            .quads_for_pattern(s.as_ref(), p.as_ref(), o.as_ref(), graph_match);
        let mut out = Self::empty_base()?;
        for qv in &matched {
            out.insert(qv.clone());
        }
        Ok(Self { inner: out })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(iri: &str) -> Term {
        Term::from_inner(TermInner::Named(iri.to_owned()))
    }

    fn variable(name: &str) -> Term {
        Term::from_inner(TermInner::Variable(name.to_owned()))
    }

    fn triple(s: &str, p: &str, o: &str) -> Quad {
        Quad::from_parts(
            named(s),
            named(p),
            named(o),
            Term::from_inner(TermInner::DefaultGraph),
        )
    }

    #[test]
    fn empty_dataset_has_zero_size() {
        let ds = Dataset::new().unwrap();
        assert_eq!(ds.size(), 0);
    }

    #[test]
    fn jsonld_and_yamlld_round_trip_through_the_wasm_surface() {
        // The wasm parse + serialize surface reaches JSON-LD and YAML-LD through the one
        // unified resolver — no side-door, no special-case guard. (JsError is only
        // constructed on the error path, which panics off-wasm, so a passing test never
        // builds one.)
        let nt = "<https://e/s> <https://e/p> <https://e/o> .\n";
        let Ok(ds) = Dataset::parse(nt, "ntriples", None) else {
            panic!("parse n-triples failed");
        };
        for fmt in [
            "jsonld",
            "application/ld+json",
            "yamlld",
            "application/ld+yaml",
        ] {
            let Ok(text) = ds.serialize(fmt) else {
                panic!("serialize {fmt} failed");
            };
            let Ok(reparsed) = Dataset::parse(&text, fmt, None) else {
                panic!("re-parse {fmt} failed");
            };
            assert_eq!(
                reparsed.size(),
                ds.size(),
                "wasm round-trip via {fmt} preserves quad count"
            );
        }
    }

    #[test]
    fn add_has_delete_are_consistent() {
        let mut ds = Dataset::new().unwrap();
        let q = triple("https://e/s", "https://e/p", "https://e/o");
        assert!(!ds.has(&q).unwrap());
        assert!(ds.add(&q).unwrap());
        assert_eq!(ds.size(), 1);
        assert!(ds.has(&q).unwrap());
        // Re-adding is a no-op (the effective set is unchanged).
        assert!(!ds.add(&q).unwrap());
        assert!(ds.delete(&q).unwrap());
        assert_eq!(ds.size(), 0);
        assert!(!ds.has(&q).unwrap());
    }

    #[test]
    fn add_then_has_a_language_literal() {
        // Exercises the canonicalization seam: a tag added as "EN" is found as the
        // canonical lowercased rdf:langString literal.
        use purrdf::RdfLiteral;
        let mut ds = Dataset::new().unwrap();
        let q = Quad::from_parts(
            named("https://e/s"),
            named("https://e/p"),
            Term::literal(RdfLiteral::language_tagged("Hello", "EN")),
            Term::from_inner(TermInner::DefaultGraph),
        );
        assert!(ds.add(&q).unwrap());
        assert!(ds.has(&q).unwrap());
    }

    #[test]
    fn match_filters_by_pattern() {
        let mut ds = Dataset::new().unwrap();
        ds.add(&triple("https://e/s1", "https://e/p", "https://e/o1"))
            .unwrap();
        ds.add(&triple("https://e/s2", "https://e/p", "https://e/o2"))
            .unwrap();

        let by_subject = ds
            .match_pattern(Some(named("https://e/s1")), None, None, None)
            .unwrap();
        assert_eq!(by_subject.size(), 1);

        let all = ds.match_pattern(None, None, None, None).unwrap();
        assert_eq!(all.size(), 2);

        // Both quads are in the default graph.
        let default_only = ds
            .match_pattern(
                None,
                None,
                None,
                Some(Term::from_inner(TermInner::DefaultGraph)),
            )
            .unwrap();
        assert_eq!(default_only.size(), 2);

        let no_match = ds
            .match_pattern(Some(named("https://e/absent")), None, None, None)
            .unwrap();
        assert_eq!(no_match.size(), 0);
    }

    #[test]
    fn match_treats_variable_as_wildcard() {
        // RDF/JS idiom: a Variable in a match() slot is a wildcard, equivalent to an
        // omitted (None) argument — it must NOT error, and must NOT constrain the slot.
        let mut ds = Dataset::new().unwrap();
        ds.add(&triple("https://e/s1", "https://e/p", "https://e/o1"))
            .unwrap();
        ds.add(&triple("https://e/s2", "https://e/p", "https://e/o2"))
            .unwrap();

        // A Variable in every term slot matches everything, exactly like all-None.
        let all_vars = ds
            .match_pattern(
                Some(variable("s")),
                Some(variable("p")),
                Some(variable("o")),
                Some(variable("g")),
            )
            .unwrap();
        assert_eq!(all_vars.size(), 2);

        // A Variable wildcard composes with a concrete constraint in another slot.
        let by_predicate = ds
            .match_pattern(
                Some(variable("s")),
                Some(named("https://e/p")),
                None,
                Some(variable("g")),
            )
            .unwrap();
        assert_eq!(by_predicate.size(), 2);

        // A Variable graph term is a wildcard (Any), not a named-graph lookup that throws.
        let any_graph = ds
            .match_pattern(Some(named("https://e/s1")), None, None, Some(variable("g")))
            .unwrap();
        assert_eq!(any_graph.size(), 1);
    }

    #[test]
    fn quads_returns_inserted_quads() {
        let mut ds = Dataset::new().unwrap();
        let q = triple("https://e/s", "https://e/p", "https://e/o");
        ds.add(&q).unwrap();
        let quads = ds.quads().unwrap();
        assert_eq!(quads.len(), 1);
        assert!(quads[0].equals(&q));
    }

    #[test]
    fn parse_then_iterate_quads() {
        let ds = Dataset::parse(
            "<https://e/s> <https://e/p> <https://e/o> .\n",
            "ntriples",
            None,
        )
        .unwrap();
        let quads = ds.quads().unwrap();
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0].subject().value(), "https://e/s");
        assert_eq!(quads[0].graph().term_type(), "DefaultGraph");
    }

    #[test]
    fn visualization_json_methods_share_one_semantic_projection() {
        let ds = Dataset::parse(
            "<https://e/alice> <https://e/knows> <https://e/bob> .\n",
            "ntriples",
            None,
        )
        .unwrap();
        let options = Some(
            r#"{"mode":"compact","vocabulary":[{"prefix":"ex","namespace":"https://e/"}],"svg":{"title":"Example graph"}}"#
                .to_owned(),
        );
        let model: serde_json::Value =
            serde_json::from_str(&ds.visual_model_json(options.clone()).expect("model JSON"))
                .expect("model value");
        let export: serde_json::Value =
            serde_json::from_str(&ds.visual_export_json(options.clone()).expect("export JSON"))
                .expect("export value");
        let document: serde_json::Value =
            serde_json::from_str(&ds.visual_svg_json(options).expect("SVG JSON"))
                .expect("document value");
        assert_eq!(model["statements"].as_array().map(Vec::len), Some(1));
        assert_eq!(export["schema_version"], "purrdf-viz-export-1");
        assert_eq!(export["model"], model);
        assert_eq!(document["export"], export);
        assert!(
            document["svg"]
                .as_str()
                .is_some_and(|svg| svg.contains("<metadata id=\"purrdf-viz-export\""))
        );
    }

    #[test]
    fn parse_then_serialize_round_trips_ntriples() {
        let input = "<https://e/s> <https://e/p> <https://e/o> .\n";
        let ds = Dataset::parse(input, "ntriples", None).unwrap();
        assert_eq!(ds.size(), 1);
        let out = ds.serialize("ntriples").unwrap();
        assert!(out.contains("https://e/s"));
        assert!(out.contains("https://e/p"));
        assert!(out.contains("https://e/o"));
        // Re-parsing the output yields the same single quad.
        let reparsed = Dataset::parse(&out, "ntriples", None).unwrap();
        assert_eq!(reparsed.size(), 1);
    }

    #[test]
    fn parse_turtle_with_base_resolves_relative_iris() {
        let input = "<rel> <https://e/p> <https://e/o> .\n";
        let ds = Dataset::parse(input, "turtle", Some("https://example.org/".to_owned())).unwrap();
        let out = ds.serialize("ntriples").unwrap();
        assert!(out.contains("https://example.org/rel"));
    }

    /// CROSS-PATH regression (the adversarial case): a directional literal PARSED
    /// from text (the engine interns it as `rdf:langString` + a separate `direction`)
    /// must be found by a `has` whose query literal is built via the SAME path a
    /// `DataFactory` literal would take — `rdf_term_to_term_value` →
    /// `canonicalize_literal`. The whole point of `canonicalize_literal` is byte
    /// identity with how the engine stores/interns the literal after a parse: if the
    /// canonical datatype diverges from what the engine interned, this `has` MISSES.
    #[test]
    fn parsed_directional_literal_is_found_by_factory_built_has() {
        use purrdf::{RdfLiteral, RdfTextDirection};

        // Parse a directional language-tagged literal from N-Triples text. The native
        // codec interns it with `direction = Some(Rtl)` and datatype `rdf:langString`
        // (see crates/rdf/src/native_codecs/parse.rs: a language tag forces
        // rdf:langString at intern time, direction kept separately).
        let input =
            "<https://e/s> <https://e/p> \"\u{0645}\u{0631}\u{062d}\u{0628}\u{0627}\"@ar--rtl .\n";
        let ds = Dataset::parse(input, "ntriples", None).unwrap();
        assert_eq!(ds.size(), 1, "the directional literal parsed into one quad");

        // Build the IDENTICAL directional literal the way a DataFactory would (which
        // routes through `Term::literal` → `canonicalize_literal`).
        let factory_literal = Term::literal(RdfLiteral {
            lexical_form: "\u{0645}\u{0631}\u{062d}\u{0628}\u{0627}".to_owned(),
            datatype: None,
            language: Some("ar".to_owned()),
            direction: Some(RdfTextDirection::Rtl),
        });
        let query = Quad::from_parts(
            named("https://e/s"),
            named("https://e/p"),
            factory_literal,
            Term::from_inner(TermInner::DefaultGraph),
        );

        // The decisive assertion: the parse-interned literal must be `has`-equal to the
        // factory-built one, even though the engine stored `rdf:langString` while the
        // RDF-1.2 effective datatype is `rdf:dirLangString`.
        assert!(
            ds.has(&query).unwrap(),
            "a factory-built directional literal must match the parse-interned one (cross-path)"
        );
    }

    /// RDF-1.2 inequality: a directional literal must NOT be `has`-equal to a plain
    /// (non-directional) langString literal with the same lexical form + language tag.
    /// The base direction participates in identity (engine C0.1), so the two are
    /// distinct terms.
    #[test]
    fn directional_literal_is_not_equal_to_plain_lang_literal() {
        use purrdf::{RdfLiteral, RdfTextDirection};

        // Parse the plain (no-direction) language-tagged literal into the base.
        let input =
            "<https://e/s> <https://e/p> \"\u{0645}\u{0631}\u{062d}\u{0628}\u{0627}\"@ar .\n";
        let ds = Dataset::parse(input, "ntriples", None).unwrap();
        assert_eq!(ds.size(), 1);

        // A directional query literal of the same text + language must NOT match it.
        let directional = Term::literal(RdfLiteral {
            lexical_form: "\u{0645}\u{0631}\u{062d}\u{0628}\u{0627}".to_owned(),
            datatype: None,
            language: Some("ar".to_owned()),
            direction: Some(RdfTextDirection::Rtl),
        });
        let query = Quad::from_parts(
            named("https://e/s"),
            named("https://e/p"),
            directional,
            Term::from_inner(TermInner::DefaultGraph),
        );
        assert!(
            !ds.has(&query).unwrap(),
            "a directional literal must NOT match a plain langString literal (RDF-1.2 distinguishes them)"
        );
    }

    // The unsupported-format error path builds a JsError (wasm-only); the pure
    // resolver is unit-tested in `codec`, and the node test in Task 5 exercises the
    // JS-boundary error.
}
