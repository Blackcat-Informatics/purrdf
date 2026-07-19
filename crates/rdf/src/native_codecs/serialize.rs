// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Frozen [`RdfDataset`](crate::RdfDataset) IR → native RDF text egress.
//!
//! Builds a first-party [`SerGraph`](super::ser_model::SerGraph) from the frozen IR —
//! interning every IR term into the graph's term table, materializing literal datatypes
//! and quoted-triple reifier bindings — then dispatches to the matching first-party
//! serializer: the Turtle / TriG / N-Triples / N-Quads serializers in
//! [`ser_model`](super::ser_model), and the in-repo [`rdfxml`](super::rdfxml) codec for
//! RDF/XML. The graph layout mirrors exactly what the parser produces, so parse and
//! serialize are inverses.
//!
//! The [`SerializeGraph`] filter matches `oxigraph/backend.rs:333-391` exactly:
//! `DefaultGraph` emits the default-graph quads plus ALL statement rows
//! (reifiers/annotations); `Named(g)` emits only that graph's quads as triples and NO
//! statement rows; `Dataset` keeps graph names for TriG/N-Quads but falls back to the
//! default graph for Turtle/N-Triples/RDF-XML.

use std::collections::HashMap;
use std::io::Write;

use super::jsonld::JsonLdSerializeOptions;
use super::media_type::{NativeRdfFormat, classify};
use super::ser_model::{SerAnnotationRow, SerGraph, SerReifierRow, SerTerm, SerTermKind};
use crate::ir::TermRef;
use crate::{DatasetView, RdfDiagnostic, RdfTextDirection, SerializeGraph, TermValue};

/// The `xsd:string` datatype IRI: a literal of this datatype with no language is a
/// plain literal and is emitted WITHOUT an explicit `^^<…>`, so it round-trips back to
/// the same plain form (matching the purrdf-gts native projection).
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// Serialize a frozen [`RdfDataset`](crate::RdfDataset) to RDF text of `media_type`, honoring the
/// [`SerializeGraph`] selection. Returns the serialized bytes.
///
/// The full RDF 1.2 statement layer (reifier bindings + annotations) is emitted for
/// every star-capable format. To serialize the base quads ONLY — for a star-incapable
/// projection target where the statement layer is declared loss (RDF/XML, JSON-LD in
/// the transcode contract) — use [`serialize_dataset_base_only`].
pub fn serialize_dataset<D: DatasetView>(
    dataset: &D,
    media_type: &str,
    selection: SerializeGraph<'_>,
) -> Result<Vec<u8>, RdfDiagnostic> {
    serialize_dataset_inner(dataset, media_type, selection, true)
}

/// Serialize JSON-LD or YAML-LD through the generic media-type surface under an
/// explicit configured mode.
///
/// Supplying JSON-LD options for another syntax is a hard error instead of silently
/// ignoring caller policy. Existing [`serialize_dataset`] calls retain their frozen
/// expanded compatibility behavior.
pub fn serialize_dataset_with_jsonld_options<D: DatasetView>(
    dataset: &D,
    media_type: &str,
    selection: SerializeGraph<'_>,
    options: &JsonLdSerializeOptions,
) -> Result<Vec<u8>, RdfDiagnostic> {
    let format = classify(media_type)?;
    if !matches!(format, NativeRdfFormat::JsonLd | NativeRdfFormat::YamlLd) {
        return Err(jsonld_options_unused(format));
    }
    let graph = build_ser_graph(dataset, format, selection, true)?;
    let text = match format {
        NativeRdfFormat::JsonLd => {
            super::jsonld::serialize_ser_graph_with_options(&graph, options)?
        }
        NativeRdfFormat::YamlLd => {
            super::jsonld::serialize_ser_graph_to_yamlld_with_options(&graph, options)?
        }
        _ => unreachable!("format was restricted to JSON-LD/YAML-LD"),
    };
    Ok(text.into_bytes())
}

/// Serialize a frozen [`RdfDataset`](crate::RdfDataset) to RDF text of `media_type`, emitting ONLY the
/// base quads and DROPPING the RDF 1.2 statement layer (reifier bindings +
/// annotations).
///
/// This is the projection egress for star-incapable targets in the transcode
/// loss contract: the dropped statement-row count is the caller's to record as
/// declared loss (`rdf12-star-unrepresentable` / `rdf12-star-jsonld-rejected`) — the
/// drop here is never silent (CONSTITUTION P7), it is the realized count the caller
/// attaches to the loss ledger.
pub fn serialize_dataset_base_only<D: DatasetView>(
    dataset: &D,
    media_type: &str,
    selection: SerializeGraph<'_>,
) -> Result<Vec<u8>, RdfDiagnostic> {
    serialize_dataset_inner(dataset, media_type, selection, false)
}

fn serialize_dataset_inner<D: DatasetView>(
    dataset: &D,
    media_type: &str,
    selection: SerializeGraph<'_>,
    include_statement_layer: bool,
) -> Result<Vec<u8>, RdfDiagnostic> {
    let format = classify(media_type)?;
    let graph = build_ser_graph(dataset, format, selection, include_statement_layer)?;
    // Dispatch to the format's codec (the single `codec_for` chokepoint): the line/Turtle
    // family walks the shared `ser_model` writers, and RDF/XML, TriX and HexTuples walk
    // the SAME `SerGraph` through their in-repo emitters (the star layer is declared loss
    // for the star-incapable XML/NDJSON targets).
    let text = super::codec::codec_for(format).serialize(&graph)?;
    Ok(text.into_bytes())
}

/// Serialize a frozen [`RdfDataset`](crate::RdfDataset) into the given writer.
pub(crate) fn serialize_into<D: DatasetView, W: Write>(
    dataset: &D,
    media_type: &str,
    selection: SerializeGraph<'_>,
    mut output: W,
) -> Result<(), RdfDiagnostic> {
    let bytes = serialize_dataset(dataset, media_type, selection)?;
    output
        .write_all(&bytes)
        .map_err(|e| RdfDiagnostic::error("native-codec-write", e.to_string()))
}

/// Outcome of serializing an [`RdfDataset`](crate::RdfDataset) to a concrete RDF format through the
/// native codecs (universal transcoder helper, ported onto the native path).
#[derive(Debug, Clone)]
pub struct SerializeOutcome {
    /// The serialized document bytes.
    pub bytes: Vec<u8>,
    /// The number of RDF-1.2 statement-layer rows (reifier bindings + annotation
    /// triples) dropped because the target format does not carry the star layer in
    /// the transcode contract. Zero for star-capable formats.
    pub statement_rows_dropped: usize,
    /// The number of base-quad object literals whose RDF-1.2 base direction was
    /// dropped because the target format has no direction surface (TriX / HexTuples
    /// keep the language tag but cannot carry `--ltr` / `--rtl`). Zero for every
    /// direction-capable format. Recorded as declared loss — never a silent drop
    /// (CONSTITUTION P7).
    pub directional_literals_dropped: usize,
}

/// Serialize the frozen IR to a concrete [`NativeRdfFormat`], returning the bytes and
/// the count of RDF-1.2 statement-layer rows dropped because the target format does
/// not carry the star layer (the projection doctrine).
///
/// Star-capable formats (Turtle, N-Triples, N-Quads, TriG) emit the full RDF-1.2
/// statement layer and report `statement_rows_dropped = 0`. Star-incapable formats
/// (RDF/XML) emit only the base quads and report the dropped statement-row count —
/// the caller records this as declared loss against the loss ledger.
///
/// `base_iri` is accepted for call-site compatibility with the former oxigraph
/// serializer; the native codecs emit absolute IRIs, so it is currently unused.
///
/// Graph selection follows [`SerializeGraph::Dataset`]: dataset-capable formats
/// (N-Quads, TriG) emit all named graphs; the single-graph syntaxes (Turtle,
/// N-Triples, RDF/XML) flatten to the default graph.
pub fn serialize_dataset_to_format<D: DatasetView>(
    dataset: &D,
    format: NativeRdfFormat,
    _base_iri: Option<&str>,
) -> Result<SerializeOutcome, RdfDiagnostic> {
    let media_type = format.media_type();
    // A base direction is dropped independently of the star layer: RDF/XML is
    // star-incapable yet carries direction, while TriX / HexTuples carry neither. Count
    // the affected object literals up front for whichever branch emits.
    let directional_literals_dropped = if format.carries_direction() {
        0
    } else {
        count_directional_object_literals(dataset)
    };
    if format.carries_star() {
        let bytes = serialize_dataset(dataset, media_type, SerializeGraph::Dataset)?;
        Ok(SerializeOutcome {
            bytes,
            statement_rows_dropped: 0,
            directional_literals_dropped,
        })
    } else {
        let bytes = serialize_dataset_base_only(dataset, media_type, SerializeGraph::Dataset)?;
        let statement_rows_dropped =
            dataset.reifier_quads().count() + dataset.annotation_quads().count();
        Ok(SerializeOutcome {
            bytes,
            statement_rows_dropped,
            directional_literals_dropped,
        })
    }
}

/// Serialize through the generic format surface with explicit JSON-LD/YAML-LD
/// configuration.
///
/// The function accepts only the two JSON-LD family formats and reports zero loss for
/// their RDF 1.2-capable carrier. Passing another format is a stable hard failure.
pub fn serialize_dataset_to_format_with_jsonld_options<D: DatasetView>(
    dataset: &D,
    format: NativeRdfFormat,
    _base_iri: Option<&str>,
    options: &JsonLdSerializeOptions,
) -> Result<SerializeOutcome, RdfDiagnostic> {
    if !matches!(format, NativeRdfFormat::JsonLd | NativeRdfFormat::YamlLd) {
        return Err(jsonld_options_unused(format));
    }
    let bytes = serialize_dataset_with_jsonld_options(
        dataset,
        format.media_type(),
        SerializeGraph::Dataset,
        options,
    )?;
    Ok(SerializeOutcome {
        bytes,
        statement_rows_dropped: 0,
        directional_literals_dropped: 0,
    })
}

fn jsonld_options_unused(format: NativeRdfFormat) -> RdfDiagnostic {
    RdfDiagnostic::error(
        "jsonld-options-unused",
        format!(
            "JSON-LD serialization options cannot be used with `{}`",
            format.media_type()
        ),
    )
}

/// Count the base-quad OBJECT literals whose resolved term carries an RDF-1.2 base
/// direction. Used to record declared loss when serializing to a format with no
/// direction surface (TriX / HexTuples) — the drop is the realized count the caller
/// attaches to the loss ledger, never a silent loss (CONSTITUTION P7).
fn count_directional_object_literals<D: DatasetView>(dataset: &D) -> usize {
    dataset
        .quads()
        .filter(|q| {
            matches!(
                dataset.resolve(q.o),
                TermRef::Literal {
                    direction: Some(_),
                    ..
                }
            )
        })
        .count()
}

/// Build the first-party [`SerGraph`] from the frozen IR, applying the
/// [`SerializeGraph`] filter while populating the quad and statement-row tables.
///
/// `pub(crate)` so the JSON-LD / YAML-LD codec ([`super::jsonld`]) can build the same
/// first-party graph shape it walks (a dataset-capable `format` such as
/// [`NativeRdfFormat::NQuads`] preserves named graphs).
pub(crate) fn build_ser_graph<D: DatasetView>(
    dataset: &D,
    format: NativeRdfFormat,
    selection: SerializeGraph<'_>,
    include_statement_layer: bool,
) -> Result<SerGraph, RdfDiagnostic> {
    let mut interner = SerGraphInterner::with_capacity(dataset.term_count());

    // Which quad rows to emit, and whether the statement layer (reifiers/annotations)
    // participates — matching the oxigraph backend's filter exactly.
    let mut graph = SerGraph {
        terms: Vec::new(),
        quads: Vec::with_capacity(dataset.len_hint().unwrap_or(0)),
        reifiers: Vec::new(),
        annotations: Vec::new(),
    };

    match selection {
        // TriG / N-Quads keep graph names; the single-graph syntaxes fall back to the
        // default graph (their `to_*` serializers reject named-graph quads).
        SerializeGraph::Dataset if format.supports_datasets() => {
            for quad in dataset.quads() {
                let s = interner.intern(dataset, quad.s)?;
                let p = interner.intern(dataset, quad.p)?;
                let o = interner.intern(dataset, quad.o)?;
                let g = match quad.g {
                    Some(g) => Some(interner.intern(dataset, g)?),
                    None => None,
                };
                graph.quads.push((s, p, o, g));
            }
            if include_statement_layer {
                push_statement_rows(&mut interner, dataset, &mut graph, false)?;
            }
        }
        SerializeGraph::Dataset | SerializeGraph::DefaultGraph => {
            for quad in dataset.quads() {
                if quad.g.is_some() {
                    continue;
                }
                let s = interner.intern(dataset, quad.s)?;
                let p = interner.intern(dataset, quad.p)?;
                let o = interner.intern(dataset, quad.o)?;
                graph.quads.push((s, p, o, None));
            }
            // A single-graph (flattened) projection drops named-graph QUADS above, so
            // it must likewise drop graph-scoped STATEMENT ROWS — otherwise the
            // single-graph serializers' `ensure_default_graph_projection` guard rejects
            // a graph-scoped reifier/annotation that has no home in the default graph.
            if include_statement_layer {
                push_statement_rows(&mut interner, dataset, &mut graph, true)?;
            }
        }
        SerializeGraph::Named(name) => {
            let target = dataset.term_id_by_value(name);
            for quad in dataset.quads() {
                if quad.g != target {
                    continue;
                }
                let s = interner.intern(dataset, quad.s)?;
                let p = interner.intern(dataset, quad.p)?;
                let o = interner.intern(dataset, quad.o)?;
                graph.quads.push((s, p, o, None));
            }
            // A named-graph selection emits NO statement rows (oxigraph parity).
        }
    }

    graph.terms = std::mem::take(&mut interner.terms);
    // The interner rows already carry the serialization row-array's graph slot (`None`
    // = default graph): a reifier/annotation declared inside a `GRAPH g { … }` block
    // keeps `g` so the emitted N-Quads/TriG round-trips it.
    graph.reifiers = std::mem::take(&mut interner.reifiers);
    // Annotations populated alongside the statement rows above.
    graph
        .annotations
        .extend(std::mem::take(&mut interner.annotations));
    // Impose a canonical, value-based row order so the emitted document is
    // byte-identical across `DatasetView` backends (whose term-table interning order —
    // and hence quad iteration order — differs) and independent of insertion order.
    graph.sort_canonical();
    Ok(graph)
}

/// Push the RDF 1.2 statement layer (reifier bindings + annotations) onto the graph,
/// interning their terms. The reifier bindings land in `interner.reifiers`; the
/// annotation triples in `interner.annotations`.
fn push_statement_rows<D: DatasetView>(
    interner: &mut SerGraphInterner,
    dataset: &D,
    _graph: &mut SerGraph,
    default_graph_only: bool,
) -> Result<(), RdfDiagnostic> {
    // `reifier_quads()` yields each side-table binding as a virtual quad
    // `(s = reifier, p = rdf:reifies, o = triple-term, g = graph)`. The `rdf:reifies`
    // predicate id (`q.p`) is the fixed virtual edge and is not materialized here — the
    // reifier row carries the resolved triple components directly.
    for q in dataset.reifier_quads() {
        // A flattened single-graph projection carries only the default graph, so a
        // graph-scoped binding is dropped exactly as its named-graph quads are.
        if default_graph_only && q.g.is_some() {
            continue;
        }
        let reifier_id = interner.intern(dataset, q.s)?;
        let (s, p, o) = interner.intern_triple_components(dataset, q.o)?;
        let g = q.g.map(|g| interner.intern(dataset, g)).transpose()?;
        interner.reifiers.push((reifier_id, (s, p, o), g));
    }
    // `annotation_quads()` yields each annotation as `(s = reifier, p = predicate,
    // o = object, g = graph)`.
    for q in dataset.annotation_quads() {
        if default_graph_only && q.g.is_some() {
            continue;
        }
        let r = interner.intern(dataset, q.s)?;
        let p = interner.intern(dataset, q.p)?;
        let o = interner.intern(dataset, q.o)?;
        let g = q.g.map(|g| interner.intern(dataset, g)).transpose()?;
        interner.annotations.push((r, p, o, g));
    }
    Ok(())
}

/// Builds the first-party term table from the frozen IR, deduplicating terms by value
/// and materializing literal datatypes + quoted-triple reifier bindings.
#[derive(Default)]
struct SerGraphInterner {
    terms: Vec<SerTerm>,
    /// Reifier-id → `(s, p, o)` bindings. Carries both the statement-layer reifiers
    /// (a resource reifying a statement) and the self-reifier sentinels of inline
    /// quoted-triple terms (skipped by the N-Quads serializer).
    reifiers: Vec<SerReifierRow>,
    annotations: Vec<SerAnnotationRow>,
    /// Value → term-id memo so equal terms collapse to one term, matching the fold the
    /// reader produces.
    memo: HashMap<TermValue, usize>,
}

impl SerGraphInterner {
    fn with_capacity(term_count: usize) -> Self {
        Self {
            terms: Vec::with_capacity(term_count),
            reifiers: Vec::new(),
            annotations: Vec::new(),
            memo: HashMap::with_capacity(term_count),
        }
    }

    /// Intern an IR term id into the first-party term table, returning its index.
    fn intern<D: DatasetView>(&mut self, dataset: &D, id: D::Id) -> Result<usize, RdfDiagnostic> {
        let value = term_value(dataset, id);
        if let Some(&idx) = self.memo.get(&value) {
            return Ok(idx);
        }
        let idx = match dataset.resolve(id) {
            TermRef::Iri(iri) => self.push_term(SerTerm {
                kind: SerTermKind::Iri,
                value: Some(iri.to_owned()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            }),
            TermRef::Blank { label, scope } => self.push_term(SerTerm {
                kind: SerTermKind::Bnode,
                value: Some(scope.qualify_label(label).into_owned()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            }),
            TermRef::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                let datatype_iri = iri_of(dataset, datatype)?;
                // A plain literal (xsd:string, no language) and a language-tagged
                // literal carry no explicit datatype term — the serializer defaults
                // them, so emitting one would change the round-trip text.
                let datatype_slot = if language.is_some() || datatype_iri == XSD_STRING {
                    None
                } else {
                    Some(self.intern_iri_string(&datatype_iri))
                };
                self.push_term(SerTerm {
                    kind: SerTermKind::Literal,
                    value: Some(lexical.to_owned()),
                    datatype: datatype_slot,
                    lang: language.map(str::to_owned),
                    direction: direction.map(direction_str),
                    reifier: None,
                })
            }
            TermRef::Triple { s, p, o } => {
                // A quoted-triple term is a `Triple` term whose `reifier` points at a
                // self-reifier binding holding `(s, p, o)`. This self-reifier sentinel
                // is what the N-Quads serializer skips.
                let s = self.intern(dataset, s)?;
                let p = self.intern(dataset, p)?;
                let o = self.intern(dataset, o)?;
                let triple_id = self.terms.len();
                self.terms.push(SerTerm {
                    kind: SerTermKind::Triple,
                    value: None,
                    datatype: None,
                    lang: None,
                    direction: None,
                    reifier: Some(triple_id),
                });
                // Self-reifier sentinel for an inline quoted-triple TERM — never a
                // graph-scoped statement-layer row, so its graph slot is `None`.
                self.reifiers.push((triple_id, (s, p, o), None));
                triple_id
            }
        };
        self.memo.insert(value, idx);
        Ok(idx)
    }

    /// Intern an IRI by value, deduplicating through the memo. Used for literal
    /// datatype terms, which the IR does not surface as standalone term ids.
    fn intern_iri_string(&mut self, iri: &str) -> usize {
        let value = TermValue::Iri(iri.to_owned());
        if let Some(&idx) = self.memo.get(&value) {
            return idx;
        }
        let idx = self.push_term(SerTerm {
            kind: SerTermKind::Iri,
            value: Some(iri.to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        self.memo.insert(value, idx);
        idx
    }

    /// Resolve a triple-term id to the `(s, p, o)` term indices of its components
    /// (interning each), for a statement-layer reifier binding.
    fn intern_triple_components<D: DatasetView>(
        &mut self,
        dataset: &D,
        triple: D::Id,
    ) -> Result<(usize, usize, usize), RdfDiagnostic> {
        match dataset.resolve(triple) {
            TermRef::Triple { s, p, o } => {
                let s = self.intern(dataset, s)?;
                let p = self.intern(dataset, p)?;
                let o = self.intern(dataset, o)?;
                Ok((s, p, o))
            }
            other => Err(RdfDiagnostic::error(
                "native-codec-reifier-not-triple",
                format!("a reifier must bind a triple term, got {other:?}"),
            )),
        }
    }

    fn push_term(&mut self, term: SerTerm) -> usize {
        let idx = self.terms.len();
        self.terms.push(term);
        idx
    }
}

/// The dataset-independent value of an IR term, for the interner memo.
fn term_value<D: DatasetView>(dataset: &D, id: D::Id) -> TermValue {
    match dataset.resolve(id) {
        TermRef::Iri(iri) => TermValue::Iri(iri.to_owned()),
        TermRef::Blank { label, scope } => TermValue::Blank {
            label: label.to_owned(),
            scope,
        },
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => TermValue::Literal {
            lexical_form: lexical.to_owned(),
            datatype: iri_of(dataset, datatype).unwrap_or_default(),
            language: language.map(str::to_owned),
            direction,
        },
        TermRef::Triple { s, p, o } => TermValue::Triple {
            s: Box::new(term_value(dataset, s)),
            p: Box::new(term_value(dataset, p)),
            o: Box::new(term_value(dataset, o)),
        },
    }
}

/// Resolve an IR term id known to be an IRI (a literal datatype) to its IRI string.
fn iri_of<D: DatasetView>(dataset: &D, id: D::Id) -> Result<String, RdfDiagnostic> {
    match dataset.resolve(id) {
        TermRef::Iri(iri) => Ok(iri.to_owned()),
        other => Err(RdfDiagnostic::error(
            "native-codec-datatype-not-iri",
            format!("a literal datatype must be an IRI, got {other:?}"),
        )),
    }
}

fn direction_str(direction: RdfTextDirection) -> String {
    match direction {
        RdfTextDirection::Ltr => "ltr".to_owned(),
        RdfTextDirection::Rtl => "rtl".to_owned(),
    }
}

#[cfg(test)]
mod serialize_to_format_tests {
    //! Coverage for the universal-transcoder helper
    //! [`serialize_dataset_to_format`], ported onto the native codecs. JSON-LD and
    //! YAML-LD are now first-class [`NativeRdfFormat`] variants routed through this
    //! helper, so their star-drop accounting is exercised alongside the others (they are
    //! star-capable, so the count is 0).
    use super::*;
    use crate::{RdfDataset, RdfDatasetBuilder, TermFactory, parse_dataset};
    use std::sync::Arc;

    /// A star-free dataset: 1 default-graph quad + 1 named-graph quad.
    fn star_free_dataset() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri_value("https://example.org/s");
        let p = b.intern_iri_value("https://example.org/p");
        let o = b.intern_iri_value("https://example.org/o");
        let g = b.intern_iri_value("https://example.org/g");
        let s2 = b.intern_iri_value("https://example.org/s2");
        let o2 = b.intern_iri_value("https://example.org/o2");
        b.push_quad(s, p, o, None);
        b.push_quad(s2, p, o2, Some(g));
        b.freeze().expect("star_free_dataset freeze")
    }

    /// A dataset WITH one reifier (`rdf:reifies` binding) + one annotation.
    fn reifier_dataset() -> Arc<RdfDataset> {
        let nq = concat!(
            "<https://e/s> <https://e/p> <https://e/o> .\n",
            "<https://e/r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ",
            "<<( <https://e/s> <https://e/p> <https://e/o> )>> .\n",
            "<https://e/r> <https://e/confidence> \"0.9\" .\n",
        );
        parse_dataset(nq.as_bytes(), "application/n-triples", None).expect("reifier_dataset parse")
    }

    fn text_of(outcome: &SerializeOutcome) -> String {
        String::from_utf8(outcome.bytes.clone()).expect("valid utf-8")
    }

    // ── star-capable formats: full statement layer, zero drops ────────────────────

    #[test]
    fn star_free_nquads_preserves_named_graph_drops_zero() {
        let ds = star_free_dataset();
        let out = serialize_dataset_to_format(&ds, NativeRdfFormat::NQuads, None)
            .expect("serialize to NQuads");
        assert_eq!(out.statement_rows_dropped, 0);
        let text = text_of(&out);
        assert!(text.contains("https://example.org/s"), "default-graph quad");
        assert!(text.contains("https://example.org/s2"), "named-graph quad");
        assert!(
            text.contains("https://example.org/g"),
            "named graph IRI preserved in NQuads"
        );
    }

    #[test]
    fn star_free_turtle_flattens_named_graph_drops_zero() {
        let ds = star_free_dataset();
        let out = serialize_dataset_to_format(&ds, NativeRdfFormat::Turtle, None)
            .expect("serialize to Turtle");
        assert_eq!(out.statement_rows_dropped, 0);
        let text = text_of(&out);
        assert!(text.contains("https://example.org/s"), "default-graph quad");
        // Turtle is default-graph-only: the named graph IRI must NOT appear.
        assert!(
            !text.contains("https://example.org/g"),
            "Turtle must not emit the named graph IRI"
        );
    }

    #[test]
    fn star_free_trig_preserves_named_graph_drops_zero() {
        let ds = star_free_dataset();
        let out = serialize_dataset_to_format(&ds, NativeRdfFormat::TriG, None)
            .expect("serialize to TriG");
        assert_eq!(out.statement_rows_dropped, 0);
        let text = text_of(&out);
        assert!(text.contains("https://example.org/s2"));
        assert!(
            text.contains("https://example.org/g"),
            "named graph preserved in a dataset-capable format"
        );
    }

    #[test]
    fn reifier_nquads_lossless() {
        let ds = reifier_dataset();
        assert_eq!(ds.reifiers().count(), 1);
        assert_eq!(ds.annotations().count(), 1);
        let out = serialize_dataset_to_format(&ds, NativeRdfFormat::NQuads, None)
            .expect("serialize to NQuads");
        assert_eq!(
            out.statement_rows_dropped, 0,
            "NQuads is star-capable: no rows dropped"
        );
        let text = text_of(&out);
        assert!(text.contains("22-rdf-syntax-ns#reifies"), "rdf:reifies row");
        assert!(text.contains("https://e/confidence"), "annotation row");
        assert!(text.contains("https://e/s"), "base quad");
    }

    // ── star-incapable format (RDF/XML): base quads only, rows reported dropped ────

    #[test]
    fn reifier_rdfxml_drops_statement_rows() {
        let ds = reifier_dataset();
        let out = serialize_dataset_to_format(&ds, NativeRdfFormat::RdfXml, None)
            .expect("serialize to RDF/XML");
        // 1 reifier + 1 annotation = 2 statement rows declared dropped (the
        // loss contract treats classic reification as non-faithful star).
        assert_eq!(out.statement_rows_dropped, 2);
        let text = text_of(&out);
        assert!(text.contains("https://e/s"), "base quad present in RDF/XML");
        assert!(
            !text.contains("22-rdf-syntax-ns#reifies"),
            "rdf:reifies must not appear in base-only RDF/XML output"
        );
    }

    #[test]
    fn star_free_rdfxml_drops_zero_when_no_statement_layer() {
        let ds = star_free_dataset();
        let out = serialize_dataset_to_format(&ds, NativeRdfFormat::RdfXml, None)
            .expect("serialize to RDF/XML");
        // No reifiers/annotations in the dataset → nothing to drop.
        assert_eq!(out.statement_rows_dropped, 0);
        // RDF/XML carries a base direction: nothing dropped there either.
        assert_eq!(out.directional_literals_dropped, 0);
        assert!(text_of(&out).contains("https://example.org/s"));
    }

    // ── base-direction drop accounting (TriX / HexTuples only) ─────────────────────

    /// A dataset with one base-direction object literal (`"hello"@en--ltr`).
    fn directional_literal_dataset() -> Arc<RdfDataset> {
        let nt = "<https://example.org/s> <https://example.org/greeting> \"hello\"@en--ltr .\n";
        parse_dataset(nt.as_bytes(), "application/n-triples", None)
            .expect("directional_literal_dataset parse")
    }

    #[test]
    fn directional_literal_dropped_by_trix_and_hextuples() {
        let ds = directional_literal_dataset();
        for format in [NativeRdfFormat::TriX, NativeRdfFormat::HexTuples] {
            let out = serialize_dataset_to_format(&ds, format, None)
                .expect("serialize to a direction-incapable format");
            assert_eq!(
                out.directional_literals_dropped, 1,
                "{format:?} must record the dropped base direction"
            );
            // The bytes still emit the language tag (only the direction is lost).
            let text = text_of(&out);
            assert!(text.contains("en"), "{format:?} keeps the language tag");
        }
    }

    #[test]
    fn directional_literal_preserved_by_direction_capable_formats() {
        let ds = directional_literal_dataset();
        for format in [
            NativeRdfFormat::Turtle,
            NativeRdfFormat::TriG,
            NativeRdfFormat::NTriples,
            NativeRdfFormat::NQuads,
            NativeRdfFormat::RdfXml,
            NativeRdfFormat::JsonLd,
            NativeRdfFormat::YamlLd,
        ] {
            let out = serialize_dataset_to_format(&ds, format, None)
                .expect("serialize to a direction-capable format");
            assert_eq!(
                out.directional_literals_dropped, 0,
                "{format:?} carries the base direction — nothing dropped"
            );
        }
    }
}
