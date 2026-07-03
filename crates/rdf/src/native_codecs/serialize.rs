// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Frozen [`RdfDataset`] IR → native RDF text egress.
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

use super::media_type::{classify, NativeRdfFormat};
use super::ser_model::{self, SerAnnotationRow, SerGraph, SerReifierRow, SerTerm, SerTermKind};
use crate::ir::TermRef;
use crate::{RdfDataset, RdfDiagnostic, RdfTextDirection, SerializeGraph, TermId, TermValue};

/// The `xsd:string` datatype IRI: a literal of this datatype with no language is a
/// plain literal and is emitted WITHOUT an explicit `^^<…>`, so it round-trips back to
/// the same plain form (matching the purrdf-gts native projection).
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// Serialize a frozen [`RdfDataset`] to RDF text of `media_type`, honoring the
/// [`SerializeGraph`] selection. Returns the serialized bytes.
///
/// The full RDF 1.2 statement layer (reifier bindings + annotations) is emitted for
/// every star-capable format. To serialize the base quads ONLY — for a star-incapable
/// projection target where the statement layer is declared loss (RDF/XML, JSON-LD in
/// the #671 transcode contract) — use [`serialize_dataset_base_only`].
pub fn serialize_dataset(
    dataset: &RdfDataset,
    media_type: &str,
    selection: SerializeGraph<'_>,
) -> Result<Vec<u8>, RdfDiagnostic> {
    serialize_dataset_inner(dataset, media_type, selection, true)
}

/// Serialize a frozen [`RdfDataset`] to RDF text of `media_type`, emitting ONLY the
/// base quads and DROPPING the RDF 1.2 statement layer (reifier bindings +
/// annotations).
///
/// This is the projection egress for star-incapable targets in the #671 transcode
/// loss contract: the dropped statement-row count is the caller's to record as
/// declared loss (`rdf12-star-unrepresentable` / `rdf12-star-jsonld-rejected`) — the
/// drop here is never silent (CONSTITUTION P7), it is the realized count the caller
/// attaches to the loss ledger.
pub fn serialize_dataset_base_only(
    dataset: &RdfDataset,
    media_type: &str,
    selection: SerializeGraph<'_>,
) -> Result<Vec<u8>, RdfDiagnostic> {
    serialize_dataset_inner(dataset, media_type, selection, false)
}

fn serialize_dataset_inner(
    dataset: &RdfDataset,
    media_type: &str,
    selection: SerializeGraph<'_>,
    include_statement_layer: bool,
) -> Result<Vec<u8>, RdfDiagnostic> {
    let format = classify(media_type)?;
    let graph = build_ser_graph(dataset, format, selection, include_statement_layer)?;
    let text = match format {
        NativeRdfFormat::Turtle => ser_model::to_turtle(&graph)?,
        NativeRdfFormat::TriG => ser_model::to_trig(&graph),
        NativeRdfFormat::NTriples => ser_model::to_ntriples(&graph)?,
        NativeRdfFormat::NQuads => ser_model::to_nquads(&graph),
        // RDF/XML serializes FIRST-PARTY from the same `SerGraph`, walking its base
        // quads (the star layer is declared loss for the star-incapable target).
        NativeRdfFormat::RdfXml => super::rdfxml::serialize_ser_graph_to_rdfxml(&graph)?,
        // TriX / HexTuples serialize FIRST-PARTY from the same `SerGraph`, walking its
        // quads (with named-graph slots) through their in-repo emitters.
        NativeRdfFormat::TriX => super::trix::serialize_ser_graph_to_trix(&graph)?,
        NativeRdfFormat::HexTuples => super::hextuples::serialize_ser_graph_to_hextuples(&graph)?,
    };
    Ok(text.into_bytes())
}

/// Serialize a frozen [`RdfDataset`] into the given writer.
pub(crate) fn serialize_into<W: Write>(
    dataset: &RdfDataset,
    media_type: &str,
    selection: SerializeGraph<'_>,
    mut output: W,
) -> Result<(), RdfDiagnostic> {
    let bytes = serialize_dataset(dataset, media_type, selection)?;
    output
        .write_all(&bytes)
        .map_err(|e| RdfDiagnostic::error("native-codec-write", e.to_string()))
}

/// Outcome of serializing an [`RdfDataset`] to a concrete RDF format through the
/// native codecs (#671 universal transcoder helper, ported onto the native path in
/// #909).
#[derive(Debug, Clone)]
pub struct SerializeOutcome {
    /// The serialized document bytes.
    pub bytes: Vec<u8>,
    /// The number of RDF-1.2 statement-layer rows (reifier bindings + annotation
    /// triples) dropped because the target format does not carry the star layer in
    /// the #671 transcode contract. Zero for star-capable formats.
    pub statement_rows_dropped: usize,
}

/// Whether a [`NativeRdfFormat`] carries the RDF-1.2 statement layer (quoted-triple
/// reifiers + annotations) under the #671 transcode loss contract.
///
/// Turtle, N-Triples, N-Quads and TriG are star-capable. RDF/XML is treated as
/// star-INcapable here even though the native serializer *can* emit classic
/// `rdf:Statement` reification: the #671 loss ledger
/// (`crates/rdf-core/src/loss.rs`) declares `*→rdfxml` as `rdf12-star-unrepresentable`
/// because classic reification is a lossy projection, not faithful RDF-1.2 star.
/// Keeping the predicate aligned with the ledger keeps the realized-loss accounting
/// honest.
fn is_star_capable(format: NativeRdfFormat) -> bool {
    matches!(
        format,
        NativeRdfFormat::Turtle
            | NativeRdfFormat::NTriples
            | NativeRdfFormat::NQuads
            | NativeRdfFormat::TriG
    )
}

/// Serialize the frozen IR to a concrete [`NativeRdfFormat`], returning the bytes and
/// the count of RDF-1.2 statement-layer rows dropped because the target format does
/// not carry the star layer (the #671 projection doctrine).
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
pub fn serialize_dataset_to_format(
    dataset: &RdfDataset,
    format: NativeRdfFormat,
    _base_iri: Option<&str>,
) -> Result<SerializeOutcome, RdfDiagnostic> {
    let media_type = format.media_type();
    if is_star_capable(format) {
        let bytes = serialize_dataset(dataset, media_type, SerializeGraph::Dataset)?;
        Ok(SerializeOutcome {
            bytes,
            statement_rows_dropped: 0,
        })
    } else {
        let bytes = serialize_dataset_base_only(dataset, media_type, SerializeGraph::Dataset)?;
        let statement_rows_dropped = dataset.reifiers().count() + dataset.annotations().count();
        Ok(SerializeOutcome {
            bytes,
            statement_rows_dropped,
        })
    }
}

/// Build the first-party [`SerGraph`] from the frozen IR, applying the
/// [`SerializeGraph`] filter while populating the quad and statement-row tables.
///
/// `pub(crate)` so the JSON-LD / YAML-LD codec ([`super::jsonld`]) can build the same
/// first-party graph shape it walks (a dataset-capable `format` such as
/// [`NativeRdfFormat::NQuads`] preserves named graphs).
pub(crate) fn build_ser_graph(
    dataset: &RdfDataset,
    format: NativeRdfFormat,
    selection: SerializeGraph<'_>,
    include_statement_layer: bool,
) -> Result<SerGraph, RdfDiagnostic> {
    let mut interner = SerGraphInterner::with_capacity(dataset.term_count());

    // Which quad rows to emit, and whether the statement layer (reifiers/annotations)
    // participates — matching the oxigraph backend's filter exactly.
    let mut graph = SerGraph {
        terms: Vec::new(),
        quads: Vec::with_capacity(dataset.quad_count()),
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
                push_statement_rows(&mut interner, dataset, &mut graph)?;
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
            if include_statement_layer {
                push_statement_rows(&mut interner, dataset, &mut graph)?;
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
    Ok(graph)
}

/// Push the RDF 1.2 statement layer (reifier bindings + annotations) onto the graph,
/// interning their terms. The reifier bindings land in `interner.reifiers`; the
/// annotation triples in `interner.annotations`.
fn push_statement_rows(
    interner: &mut SerGraphInterner,
    dataset: &RdfDataset,
    _graph: &mut SerGraph,
) -> Result<(), RdfDiagnostic> {
    for (reifier, triple, graph) in dataset.reifiers_with_graph() {
        let reifier_id = interner.intern(dataset, reifier)?;
        let (s, p, o) = interner.intern_triple_components(dataset, triple)?;
        let g = graph.map(|g| interner.intern(dataset, g)).transpose()?;
        interner.reifiers.push((reifier_id, (s, p, o), g));
    }
    for (reifier, predicate, object, graph) in dataset.annotations_with_graph() {
        let r = interner.intern(dataset, reifier)?;
        let p = interner.intern(dataset, predicate)?;
        let o = interner.intern(dataset, object)?;
        let g = graph.map(|g| interner.intern(dataset, g)).transpose()?;
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
    fn intern(&mut self, dataset: &RdfDataset, id: TermId) -> Result<usize, RdfDiagnostic> {
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
    fn intern_triple_components(
        &mut self,
        dataset: &RdfDataset,
        triple: TermId,
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
fn term_value(dataset: &RdfDataset, id: TermId) -> TermValue {
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
fn iri_of(dataset: &RdfDataset, id: TermId) -> Result<String, RdfDiagnostic> {
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
    //! Coverage for the #671 universal-transcoder helper
    //! [`serialize_dataset_to_format`], ported onto the native codecs in #909.
    //! (JSON-LD is no longer routed through this helper — it has no [`NativeRdfFormat`]
    //! variant — so the JSON-LD drop accounting is exercised in
    //! `crates/pipeline/src/transcode.rs` via the native `yaml_ld` serializer.)
    use super::*;
    use crate::{parse_dataset, RdfDatasetBuilder, TermFactory};
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
        // 1 reifier + 1 annotation = 2 statement rows declared dropped (the #671
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
        assert!(text_of(&out).contains("https://example.org/s"));
    }
}
