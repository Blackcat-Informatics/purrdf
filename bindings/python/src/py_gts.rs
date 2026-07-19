// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The native `RDF → GTS` producer surface for the `purrdf` Python extension
//! (Task 8 / C7).
//!
//! This module moves the byte-emitting core of `src/purrdf_tools/gts_producer.py`
//! into Rust. The Python `_Builder` interns terms, content-sorts them, and emits
//! a SINGLE `dist`-profile `snapshot` frame (preceded by blob frames, and — when
//! signing — a transport-key `meta` frame). It does **not** use
//! [`purrdf_gts::writer::Writer::deterministic`] (which emits separate
//! `terms`/`quads`/`reifies`/`annot` frames); it authors the snapshot frame
//! directly via `Writer::add_frame("snapshot", …)`.
//!
//! To preserve **byte-identity** with the existing producer — and, crucially, the
//! `snapshot_content_id()` self-attestation that `feedback_bundle.py` relies on
//! — this module replicates `_Builder` exactly:
//!
//! * the same interning order (append-order, scope-aware blank nodes);
//! * the same content sort (`(kind, value, datatype-IRI, lang)`, IRIs first);
//! * the same snapshot payload map (`terms` + `quads`, plus `reifies`/`annot`
//!   when non-empty);
//! * the same blob ordering (`(rep, decoded-bytes)`);
//! * the same per-payload `zstd-rsyncable` selection above the threshold;
//! * the same transport-key `meta` frame on the signed path.
//!
//! All CBOR encoding, canonicalization, frame-id chaining, and signing is
//! delegated to `purrdf-gts` — never hand-rolled.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};

use crate::bundle::{RdfBundle, UnitMetadata};
// The byte-emitting compose core now lives in the pyo3-free `gts_compose` module
// (P6); this surface is the thin pyo3 wrapper that delegates to it.
use crate::gts_compose::{BlobRow, DEFAULT_RSYNCABLE_THRESHOLD, SnapshotBuilder, emit_gts};
use crate::ir::RdfDataset;
use crate::provenance::{DatasetProvenance, OriginKind};
use crate::py_jsonld::{PyCompiledJsonLdContext, options_from_inputs};
use crate::py_store::{PyRdfFormat, parse_quads};
use crate::{NativeRdfFormat, RdfQuad, flat_dataset_from_quads};

/// The `rep`-label prefix every S3 slice-artifact blob carries (S3). A blob
/// authored from the slice catalog rides ahead of the snapshot with
/// `rep == "slice-artifact:{role}:{logical_path}"`, so a repo-free consumer can
/// recover each ontology artifact by role + logical path + content digest. This
/// is the SAME content-addressed blob channel `doc_blobs` use — never a parallel
/// one (greenfield, one embedding).
const SLICE_ARTIFACT_REP_PREFIX: &str = "slice-artifact:";
/// One slice artifact row passed from Python (`gts_gen.py` via `purrdf_slice`):
/// `(slice_iri, slice_name, role, logical_path, content)`. `logical_path` is the
/// repo-relative path (e.g. `slices/core/epistemics/module.ttl`) and is the
/// bundle's normalized artifact path. Only the small ontology text artifacts
/// (module / shapes / docs / manifest) are passed here; the large external DATA
/// blobs (`graph.blobs`) STAY by-reference and never travel this channel
/// (blob-by-reference doctrine).
struct SliceArtifactRow {
    slice_iri: String,
    slice_name: String,
    role: String,
    logical_path: String,
    content: Vec<u8>,
}

/// Build a frozen [`RdfDataset`] from a flat native quad list (verbatim, no RDF 1.2
/// statement-layer fold). Used so the production [`RdfBundle`] carries the actual hot
/// graph (not a placeholder) while it gates the artifact index.
fn dataset_from_quads(quads: &[RdfQuad]) -> Result<std::sync::Arc<RdfDataset>, String> {
    flat_dataset_from_quads(quads)
}

/// Assemble the self-describing S3 [`RdfBundle`] from the slice-artifact rows and
/// the parsed base graph, hard-fail `validate()` it, and return the artifact bytes
/// as content-addressed [`BlobRow`]s to embed (S3, gap G4).
///
/// One [`UnitId`] per slice (metadata = slice IRI + name), one content-addressed
/// `ArtifactRecord` per ontology artifact, every blob inserted into the bundle's
/// `ContentStore`. The producer emits a SINGLE `snapshot` frame, so every unit is
/// associated with that one snapshot segment (segment 0) — set-valued and never
/// assuming one-segment == one-slice. The blob rows ride the SAME channel
/// `doc_blobs` use; the bundle's `dataset` carries the real hot graph.
fn assemble_slice_bundle(
    base_quads: &[RdfQuad],
    rows: &[SliceArtifactRow],
) -> Result<Vec<BlobRow>, String> {
    const SNAPSHOT_SEGMENT: usize = 0;

    let dataset = dataset_from_quads(base_quads)?;
    let provenance = DatasetProvenance::new();
    let mut bundle = RdfBundle::new(dataset, provenance);

    let mut blob_rows: Vec<BlobRow> = Vec::with_capacity(rows.len());
    for row in rows {
        // One UnitId per slice (idempotent intern); metadata = IRI + name.
        let unit = bundle
            .provenance
            .register_unit(row.slice_iri.clone(), OriginKind::Source);
        bundle.add_unit(
            unit,
            UnitMetadata::new(row.slice_iri.clone(), row.slice_name.clone()),
        );
        // One content-addressed artifact per ontology file (bytes → ContentStore).
        let artifact = bundle
            .provenance
            .register_artifact(row.logical_path.clone());
        bundle.add_artifact(
            artifact,
            unit,
            row.logical_path.clone(),
            row.role.clone(),
            row.content.clone(),
        );
        // Every unit lives in the single snapshot segment (set-valued S0.7).
        bundle.associate_segment(SNAPSHOT_SEGMENT, unit);

        // The SAME content-addressed blob channel doc_blobs ride: rep encodes
        // role + logical path so a repo-free consumer recovers each artifact.
        blob_rows.push(BlobRow {
            data: row.content.clone(),
            media_type: media_type_for(&row.logical_path),
            rep: format!(
                "{SLICE_ARTIFACT_REP_PREFIX}{}:{}",
                row.role, row.logical_path
            ),
        });
    }

    // HARD-fail on any structural violation BEFORE serialization (no-optionality).
    bundle.validate().map_err(|e| e.to_string())?;
    Ok(blob_rows)
}

/// Infer a stable MIME type for a slice artifact path (mirrors the slice catalog's
/// `infer_media_type`, kept local to avoid a kernel→slice dependency edge).
fn media_type_for(path: &str) -> String {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "ttl" => "text/turtle",
        "nt" => "application/n-triples",
        "nq" => "application/n-quads",
        "sparql" | "rq" => "application/sparql-query",
        "md" => "text/markdown",
        "yaml" | "yml" | "cff" => "application/yaml",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
    .to_string()
}

// ── Python helpers ────────────────────────────────────────────────────────────

fn blob_rows_from_py(blobs: Option<&Bound<'_, PyList>>) -> PyResult<Vec<BlobRow>> {
    let Some(blobs) = blobs else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(blobs.len());
    for item in blobs.iter() {
        let (data, media_type, rep): (Vec<u8>, String, String) = item
            .extract()
            .map_err(|_| PyValueError::new_err("blob rows must be (bytes, media_type, rep)"))?;
        out.push(BlobRow {
            data,
            media_type,
            rep,
        });
    }
    Ok(out)
}

/// Parse the slice-artifact rows passed from Python: each is the tuple
/// `(slice_iri, slice_name, role, logical_path, content)`.
fn slice_artifact_rows_from_py(
    rows: Option<&Bound<'_, PyList>>,
) -> PyResult<Vec<SliceArtifactRow>> {
    let Some(rows) = rows else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(rows.len());
    for item in rows.iter() {
        let (slice_iri, slice_name, role, logical_path, content): (
            String,
            String,
            String,
            String,
            Vec<u8>,
        ) = item.extract().map_err(|_| {
            PyValueError::new_err(
                "slice artifact rows must be (slice_iri, slice_name, role, logical_path, content)",
            )
        })?;
        out.push(SliceArtifactRow {
            slice_iri,
            slice_name,
            role,
            logical_path,
            content,
        });
    }
    Ok(out)
}

fn secret_array(secret: Option<&Bound<'_, PyBytes>>) -> PyResult<Option<[u8; 32]>> {
    match secret {
        None => Ok(None),
        Some(bytes) => {
            let raw = bytes.as_bytes();
            let arr: [u8; 32] = raw
                .try_into()
                .map_err(|_| PyValueError::new_err("signer secret must be 32 raw Ed25519 bytes"))?;
            Ok(Some(arr))
        }
    }
}

/// Parse RDF bytes leniently into native quads. The lenient parser accepts
/// private-use language tags (`@x-purrdf-*`) that the strict `purrdf.Literal`
/// constructor would reject — the producer therefore lowers rdflib sources to
/// N-Quads/Turtle bytes and parses HERE, never building `Quad` objects.
///
/// Takes plain `&[u8]` so callers can run it inside [`Python::detach`] (GIL
/// released); the error is a lazily-materialized `ValueError`.
fn parse_rdf(data: &[u8], format: PyRdfFormat) -> PyResult<Vec<RdfQuad>> {
    parse_quads(data, rdf_format(format), None)
        .map_err(|e| PyValueError::new_err(format!("parse error: {e}")))
}

/// Parse RDF bytes into a frozen native [`RdfDataset`] for `SnapshotBuilder`
/// ingestion (the native carrier path). The native parse folds the RDF 1.2
/// statement layer into the dataset's reifier/annotation side-tables and preserves
/// named graphs, so `add_dataset_scoped` reproduces the legacy oxigraph ingestion
/// byte-for-byte. The blank-node `scope` is applied at INGESTION (by
/// `add_dataset_scoped`), not here — `parse_dataset`'s third argument is the base
/// IRI, never a blank scope. Private-use language tags (`@x-purrdf-*`) survive.
fn parse_rdf_dataset(data: &[u8], format: PyRdfFormat) -> PyResult<std::sync::Arc<RdfDataset>> {
    crate::parse_dataset(data, rdf_format(format).media_type(), None)
        .map_err(|e| PyValueError::new_err(format!("parse error: {e}")))
}

// ── Module-level functions ────────────────────────────────────────────────────

/// The pure-Rust parse → snapshot-build → emit core `gts_from_quads` /
/// `gts_from_rdf12_bytes` share; runs inside [`Python::detach`] (GIL released).
fn snapshot_gts_bytes(
    data: &[u8],
    format: PyRdfFormat,
    profile: &str,
    transform: Option<Vec<String>>,
) -> PyResult<Vec<u8>> {
    let dataset = parse_rdf_dataset(data, format)?;
    let mut builder = SnapshotBuilder::default();
    builder
        .add_dataset(&dataset)
        .map_err(PyValueError::new_err)?;
    emit_gts(
        &builder,
        profile,
        transform,
        Vec::new(),
        Vec::new(),
        None,
        None,
        None,
        DEFAULT_RSYNCABLE_THRESHOLD,
    )
    .map_err(PyValueError::new_err)
}

/// Produce a GTS snapshot from a serialized RDF 1.1 base graph (Turtle/N-Quads
/// bytes, parsed leniently). Mirrors `gts_producer.gts_from_graph`. `transform`
/// defaults to `["zstd"]` when `None`.
#[pyfunction]
#[pyo3(signature = (data, *, format, profile="dist", transform=None))]
fn gts_from_quads(
    py: Python<'_>,
    data: &Bound<'_, PyBytes>,
    format: PyRdfFormat,
    profile: &str,
    transform: Option<Vec<String>>,
) -> PyResult<Py<PyBytes>> {
    let raw = data.as_bytes();
    let bytes = py.detach(|| snapshot_gts_bytes(raw, format, profile, transform))?;
    Ok(PyBytes::new(py, &bytes).unbind())
}

/// Produce a GTS snapshot from an RDF 1.2 statement-layer artifact's bytes
/// (parsed natively as Turtle/N-Quads). Mirrors `gts_producer.gts_from_rdf12`.
#[pyfunction]
#[pyo3(signature = (data, *, format, profile="dist", transform=None))]
fn gts_from_rdf12_bytes(
    py: Python<'_>,
    data: &Bound<'_, PyBytes>,
    format: PyRdfFormat,
    profile: &str,
    transform: Option<Vec<String>>,
) -> PyResult<Py<PyBytes>> {
    let raw = data.as_bytes();
    let bytes = py.detach(|| snapshot_gts_bytes(raw, format, profile, transform))?;
    Ok(PyBytes::new(py, &bytes).unbind())
}

/// Serialize RDF bytes to **JSON-LD-star** (RDF-1.2-faithful) via the FIRST-PARTY native
/// codec: parse the input RDF bytes into the frozen IR, then emit JSON-LD-star through the
/// in-repo `native_codecs::jsonld` serializer — no longer the external purrdf-gts JSON-LD
/// codec. This is the RDF-1.2-first JSON-LD form the published `*.jsonld` artifacts emit.
#[pyfunction]
#[pyo3(signature = (data, *, format, options_json=None, context=None))]
fn to_json_ld(
    py: Python<'_>,
    data: &Bound<'_, PyBytes>,
    format: PyRdfFormat,
    options_json: Option<&str>,
    context: Option<&PyCompiledJsonLdContext>,
) -> PyResult<String> {
    let raw = data.as_bytes();
    let configured = if options_json.is_some() || context.is_some() {
        Some(options_from_inputs(options_json, context, None)?)
    } else {
        None
    };
    py.detach(|| {
        let dataset = parse_rdf_dataset(raw, format)?;
        if let Some(options) = &configured {
            crate::native_codecs::jsonld::serialize_dataset_to_jsonld_with_options(
                &dataset, options,
            )
            .map_err(|e| PyValueError::new_err(format!("json-ld-star serialization error: {e}")))
        } else {
            crate::native_codecs::jsonld::serialize_dataset_to_jsonld(&dataset).map_err(|e| {
                PyValueError::new_err(format!("json-ld-star serialization error: {e}"))
            })
        }
    })
}

/// The caller-supplied statement-metadata vocabulary passed from Python as a
/// dict with the keys `class` / `subject` / `predicate` / `object` /
/// `objectLiteral` (each an absolute IRI). PurRDF mints no vocabulary of its
/// own, so every key is REQUIRED — there is no fabricated default.
fn statement_vocab_from_dict(vocab: &Bound<'_, PyDict>) -> PyResult<[String; 5]> {
    let field = |key: &str| -> PyResult<String> {
        vocab
            .get_item(key)?
            .ok_or_else(|| {
                PyValueError::new_err(format!(
                    "statement_vocab is missing the required {key:?} key \
                     (keys: class/subject/predicate/object/objectLiteral)"
                ))
            })?
            .extract::<String>()
    };
    Ok([
        field("class")?,
        field("subject")?,
        field("predicate")?,
        field("object")?,
        field("objectLiteral")?,
    ])
}

/// Parse **JSON-LD-star** text into N-Quads bytes, via the FIRST-PARTY native codec:
/// `native_codecs::jsonld::parse_jsonld` into the frozen IR, then serialize to N-Quads —
/// no longer the external purrdf-gts JSON-LD codec.
///
/// With `statement_vocab` (a dict with the `class` / `subject` / `predicate` /
/// `object` / `objectLiteral` IRI keys) the RDF-1.2 statement layer is DOWNCAST
/// to flat statement-metadata cells in the caller's vocabulary (rdflib-safe: no
/// quoted triples in the output). Without it, star features round-trip as
/// RDF 1.2 N-Quads (`rdf:reifies` + quoted-triple terms) — PurRDF mints no
/// default vocabulary, so no vocabulary terms are ever fabricated.
#[pyfunction]
#[pyo3(signature = (text, *, statement_vocab=None))]
fn from_json_ld(
    py: Python<'_>,
    text: &str,
    statement_vocab: Option<&Bound<'_, PyDict>>,
) -> PyResult<Py<PyBytes>> {
    // Extract the vocab dict to owned Rust data BEFORE releasing the GIL.
    let vocab_fields: Option<[String; 5]> =
        statement_vocab.map(statement_vocab_from_dict).transpose()?;
    let nquads: Vec<u8> = py.detach(|| {
        if let Some([class, subject, predicate, object, object_literal]) = &vocab_fields {
            let vocab = crate::native_codecs::jsonld::StatementMetadataVocab {
                statement_metadata: class,
                q_subject: subject,
                q_predicate: predicate,
                q_object: object,
                q_object_literal: object_literal,
            };
            return crate::native_codecs::jsonld::jsonld_to_statement_metadata_nquads(
                text.as_bytes(),
                Some(&vocab),
            )
            .map(String::into_bytes)
            .map_err(|e| PyValueError::new_err(format!("json-ld-star downcast error: {e}")));
        }
        let dataset = crate::native_codecs::jsonld::parse_jsonld(text.as_bytes())
            .map_err(|e| PyValueError::new_err(format!("json-ld-star parse error: {e}")))?;
        crate::serialize_dataset(
            &dataset,
            NativeRdfFormat::NQuads.media_type(),
            crate::SerializeGraph::Dataset,
        )
        .map_err(|e| {
            PyValueError::new_err(format!("json-ld-star→n-quads serialization error: {e}"))
        })
    })?;
    Ok(PyBytes::new(py, &nquads).unbind())
}

/// Serialize RDF bytes to **RDF/XML** via the FIRST-PARTY native codec:
/// parse the input RDF bytes into the frozen IR, then emit RDF/XML through the in-repo
/// `native_codecs::rdfxml` serializer — no longer the external purrdf-gts RDF/XML codec.
#[pyfunction]
#[pyo3(signature = (data, *, format))]
fn to_rdf_xml(py: Python<'_>, data: &Bound<'_, PyBytes>, format: PyRdfFormat) -> PyResult<String> {
    let raw = data.as_bytes();
    py.detach(|| {
        let dataset = parse_rdf_dataset(raw, format)?;
        let bytes = crate::serialize_dataset(
            &dataset,
            NativeRdfFormat::RdfXml.media_type(),
            crate::SerializeGraph::Dataset,
        )
        .map_err(|e| PyValueError::new_err(format!("rdf/xml serialization error: {e}")))?;
        String::from_utf8(bytes).map_err(|e| {
            PyValueError::new_err(format!("rdf/xml serialization produced non-utf8: {e}"))
        })
    })
}

/// Parse **RDF/XML** text into N-Quads bytes, via the FIRST-PARTY native codec
/// parse RDF/XML into the frozen IR, then serialize to N-Quads — no longer
/// the external purrdf-gts RDF/XML codec.
#[pyfunction]
fn from_rdf_xml(py: Python<'_>, text: &str) -> PyResult<Py<PyBytes>> {
    let nquads: Vec<u8> = py.detach(|| {
        let dataset =
            crate::parse_dataset(text.as_bytes(), NativeRdfFormat::RdfXml.media_type(), None)
                .map_err(|e| PyValueError::new_err(format!("rdf/xml parse error: {e}")))?;
        crate::serialize_dataset(
            &dataset,
            NativeRdfFormat::NQuads.media_type(),
            crate::SerializeGraph::Dataset,
        )
        .map_err(|e| PyValueError::new_err(format!("rdf/xml→n-quads serialization error: {e}")))
    })?;
    Ok(PyBytes::new(py, &nquads).unbind())
}

/// One named-graph ingest row passed from Python: `(data, format, graph_name, scope)`.
/// `graph_name`/`scope` may be `None` (the default graph / un-scoped blank nodes).
type NamedGraphRow<'py> = (
    Bound<'py, PyBytes>,
    PyRdfFormat,
    Option<String>,
    Option<String>,
);

/// A [`NamedGraphRow`] with its Python-side values lowered to plain borrows
/// (`&[u8]` bytes, `&str` names) — the GIL-free shape the detached compile
/// closure consumes.
type BorrowedNamedGraphRow<'a> = (&'a [u8], PyRdfFormat, Option<&'a str>, Option<&'a str>);

/// The full statement-complete compiler, mirroring `gts_producer.compile_gts`.
///
/// `base_data` is the canonicalized RDF 1.1 base graph as RDF bytes (the caller
/// canonicalizes blank-node labels with RDFC-1.0 before serializing, exactly as
/// the Python `compile_gts` does via `to_canonical_graph`). It is parsed leniently
/// HERE so private-use language tags survive. `rdf12_data` is the RDF 1.2 statement
/// layer's bytes. `named_graphs` carries the alignment graph and any extra named
/// graphs as `(data, format, graph_name, scope)` rows.
#[pyfunction]
#[pyo3(signature = (
    base_data,
    base_format,
    *,
    base_scope=None,
    rdf12_data=None,
    rdf12_format=None,
    rdf12_graph_name=None,
    rdf12_scope=None,
    named_graphs=None,
    transform=None,
    doc_blobs=None,
    report_blobs=None,
    slice_artifacts=None,
    signer_secret=None,
    signer_kid=None,
    public_key_armor=None,
    rsyncable_threshold=DEFAULT_RSYNCABLE_THRESHOLD,
))]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
fn compile_gts_native(
    py: Python<'_>,
    base_data: &Bound<'_, PyBytes>,
    base_format: PyRdfFormat,
    base_scope: Option<String>,
    rdf12_data: Option<&Bound<'_, PyBytes>>,
    rdf12_format: Option<PyRdfFormat>,
    rdf12_graph_name: Option<String>,
    rdf12_scope: Option<String>,
    named_graphs: Option<Vec<NamedGraphRow<'_>>>,
    transform: Option<Vec<String>>,
    doc_blobs: Option<&Bound<'_, PyList>>,
    report_blobs: Option<&Bound<'_, PyList>>,
    slice_artifacts: Option<&Bound<'_, PyList>>,
    signer_secret: Option<&Bound<'_, PyBytes>>,
    signer_kid: Option<String>,
    public_key_armor: Option<String>,
    rsyncable_threshold: usize,
) -> PyResult<Py<PyBytes>> {
    // Convert EVERY Python-side argument to plain/owned Rust data BEFORE
    // releasing the GIL; the parse + snapshot-build + emit core runs detached.
    let base_bytes = base_data.as_bytes();
    let rdf12_bytes: Option<(&[u8], PyRdfFormat)> = match rdf12_data {
        Some(data) => {
            let format = rdf12_format
                .ok_or_else(|| PyValueError::new_err("rdf12_data requires rdf12_format"))?;
            Some((data.as_bytes(), format))
        }
        None => None,
    };
    let named_graphs = named_graphs.unwrap_or_default();
    let named_graph_rows: Vec<BorrowedNamedGraphRow<'_>> = named_graphs
        .iter()
        .map(|(data, format, graph_name, scope)| {
            (
                data.as_bytes(),
                *format,
                graph_name.as_deref(),
                scope.as_deref(),
            )
        })
        .collect();
    let doc_blob_rows = blob_rows_from_py(doc_blobs)?;
    let report_blob_rows = blob_rows_from_py(report_blobs)?;
    let slice_rows = slice_artifact_rows_from_py(slice_artifacts)?;
    let secret = secret_array(signer_secret)?;

    let bytes: Vec<u8> = py.detach(move || {
        let mut builder = SnapshotBuilder::default();

        let base_dataset = parse_rdf_dataset(base_bytes, base_format)?;
        builder
            .add_dataset_scoped(&base_dataset, None, base_scope.as_deref())
            .map_err(PyValueError::new_err)?;

        if let Some((data, format)) = rdf12_bytes {
            let dataset = parse_rdf_dataset(data, format)?;
            builder
                .add_dataset_scoped(
                    &dataset,
                    rdf12_graph_name.as_deref(),
                    rdf12_scope.as_deref(),
                )
                .map_err(PyValueError::new_err)?;
        }

        for (data, format, graph_name, scope) in named_graph_rows {
            let dataset = parse_rdf_dataset(data, format)?;
            builder
                .add_dataset_scoped(&dataset, graph_name, scope)
                .map_err(PyValueError::new_err)?;
        }

        // S3 (gap G4): assemble the self-describing RdfBundle from the slice
        // catalog rows, hard-fail `validate()`, and fold each ontology artifact in as
        // a content-addressed blob through the SAME channel doc_blobs ride. The base
        // graph is the bundle's hot dataset. Large external DATA blobs (graph.blobs)
        // are NOT passed here and STAY by-reference (blob-by-reference doctrine).
        let mut all_doc_blobs = doc_blob_rows;
        if !slice_rows.is_empty() {
            // The bundle assembler still consumes a flat oxigraph quad list for its hot
            // dataset; re-parse the base here (only when slice artifacts are present).
            let base = parse_rdf(base_bytes, base_format)?;
            let bundle_blobs =
                assemble_slice_bundle(&base, &slice_rows).map_err(PyValueError::new_err)?;
            all_doc_blobs.extend(bundle_blobs);
        }

        emit_gts(
            &builder,
            "dist",
            transform,
            all_doc_blobs,
            report_blob_rows,
            secret,
            signer_kid,
            public_key_armor,
            rsyncable_threshold,
        )
        .map_err(PyValueError::new_err)
    })?;
    Ok(PyBytes::new(py, &bytes).unbind())
}

/// The `blake3:<hex>` snapshot content id of a base graph (RDF bytes), mirroring
/// `_Builder.snapshot_content_id` for the feedback-bundle self-attestation.
#[pyfunction]
#[pyo3(signature = (data, *, format))]
fn snapshot_content_id_native(
    py: Python<'_>,
    data: &Bound<'_, PyBytes>,
    format: PyRdfFormat,
) -> PyResult<String> {
    let raw = data.as_bytes();
    py.detach(|| {
        let dataset = parse_rdf_dataset(raw, format)?;
        let mut builder = SnapshotBuilder::default();
        builder
            .add_dataset(&dataset)
            .map_err(PyValueError::new_err)?;
        Ok(builder.snapshot_content_id())
    })
}

/// Build a feedback bundle: a base graph (RDF bytes) as the snapshot, report blobs
/// riding ahead. Mirrors `feedback_bundle.build_feedback_bundle`'s `_Builder.to_gts`.
#[pyfunction]
#[pyo3(signature = (data, *, format, report_blobs=None))]
fn feedback_bundle_native(
    py: Python<'_>,
    data: &Bound<'_, PyBytes>,
    format: PyRdfFormat,
    report_blobs: Option<&Bound<'_, PyList>>,
) -> PyResult<Py<PyBytes>> {
    let raw = data.as_bytes();
    let report_blob_rows = blob_rows_from_py(report_blobs)?;
    let bytes: Vec<u8> = py.detach(move || {
        let dataset = parse_rdf_dataset(raw, format)?;
        let mut builder = SnapshotBuilder::default();
        builder
            .add_dataset(&dataset)
            .map_err(PyValueError::new_err)?;
        emit_gts(
            &builder,
            "dist",
            None,
            Vec::new(),
            report_blob_rows,
            None,
            None,
            None,
            DEFAULT_RSYNCABLE_THRESHOLD,
        )
        .map_err(PyValueError::new_err)
    })?;
    Ok(PyBytes::new(py, &bytes).unbind())
}

fn rdf_format(format: PyRdfFormat) -> NativeRdfFormat {
    format.to_native()
}

/// Register the native GTS producer surface on the `purrdf` module.
pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(gts_from_quads, m)?)?;
    m.add_function(wrap_pyfunction!(gts_from_rdf12_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(compile_gts_native, m)?)?;
    m.add_function(wrap_pyfunction!(snapshot_content_id_native, m)?)?;
    m.add_function(wrap_pyfunction!(feedback_bundle_native, m)?)?;
    m.add_function(wrap_pyfunction!(to_json_ld, m)?)?;
    m.add_function(wrap_pyfunction!(from_json_ld, m)?)?;
    m.add_function(wrap_pyfunction!(to_rdf_xml, m)?)?;
    m.add_function(wrap_pyfunction!(from_rdf_xml, m)?)?;
    crate::py_gts_dataset::register(m)?;
    Ok(())
}
