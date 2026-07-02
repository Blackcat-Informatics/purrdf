# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

# Type stub for the purrdf PyO3 extension. The signatures are transcribed
# verbatim from bindings/python/src/rdf.rs (the statement codec) and
# bindings/python/src/py_store.rs (the native Store / SPARQL / parse /
# canonicalize surface, #667) — keep them in lockstep with those files (they are
# the ABI source of truth). This stub describes the native `purrdf` term /
# result / store surface — the in-repo binding that replaced the external RDF
# package removed in #667.

from __future__ import annotations

import builtins
from typing import IO, Any, TypedDict, overload

# ── Statement codec (bindings/python/src/rdf.rs) ────────────────────────────────

def project_statements_rdf12(owl_ttl: str) -> str: ...
def normalize_rdf12_to_owl(rdf12_ttl: str) -> str: ...
def loss_matrix_json() -> str: ...
def canonicalize_turtle(
    turtle_bytes: bytes, extra_prefixes: list[tuple[str, str]] = ...
) -> bytes: ...

# ── Serialization / canonicalization enums ──────────────────────────────────────

class RdfFormat:
    TURTLE: RdfFormat
    N_TRIPLES: RdfFormat
    N_QUADS: RdfFormat
    TRIG: RdfFormat

class CanonicalizationAlgorithm:
    RDFC_1_0: CanonicalizationAlgorithm
    UNSTABLE: CanonicalizationAlgorithm

# ── Term model ──────────────────────────────────────────────────────────────────

class NamedNode:
    def __init__(self, value: str) -> None: ...
    @property
    def value(self) -> str: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...

class BlankNode:
    def __init__(self, value: str) -> None: ...
    @property
    def value(self) -> str: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...

class Literal:
    def __init__(
        self,
        value: str,
        *,
        datatype: NamedNode | None = ...,
        language: str | None = ...,
        direction: str | None = ...,
    ) -> None: ...
    @property
    def value(self) -> str: ...
    @property
    def language(self) -> str | None: ...
    @property
    def direction(self) -> str | None: ...
    @property
    def datatype(self) -> NamedNode: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...

class Triple:
    def __init__(
        self, subject: _Subject, predicate: NamedNode, object: _Term
    ) -> None: ...
    @property
    def subject(self) -> _Subject: ...
    @property
    def predicate(self) -> NamedNode: ...
    @property
    def object(self) -> _Term: ...
    def __hash__(self) -> int: ...
    # `object` (the property above) shadows the builtin in class scope, so the
    # annotation must qualify it — otherwise mypy reads it as `Triple.object`.
    def __eq__(self, other: builtins.object) -> bool: ...

class DefaultGraph:
    def __init__(self) -> None: ...

class Quad:
    def __init__(
        self,
        subject: _Subject,
        predicate: NamedNode,
        object: _Term,
        graph_name: NamedNode | BlankNode | DefaultGraph | None = ...,
    ) -> None: ...
    @property
    def subject(self) -> _Subject: ...
    @property
    def predicate(self) -> NamedNode: ...
    @property
    def object(self) -> _Term: ...
    @property
    def graph_name(self) -> NamedNode | BlankNode | DefaultGraph: ...
    def __hash__(self) -> int: ...
    # `object` (the property above) shadows the builtin in class scope, so the
    # annotation must qualify it — otherwise mypy reads it as `Quad.object`.
    def __eq__(self, other: builtins.object) -> bool: ...

class Variable:
    def __init__(self, value: str) -> None: ...
    @property
    def value(self) -> str: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...

# RDF 1.2 (unlike the obsolete RDF-star) permits triple terms in the OBJECT
# position only: a subject is an IRI or blank node, never a quoted triple. This
# mirrors oxigraph's `NamedOrBlankNode` subject type — see `extract_subject` in
# bindings/python/src/py_store.rs.
_Subject = NamedNode | BlankNode
_Term = NamedNode | BlankNode | Literal | Triple

# ── Query results ───────────────────────────────────────────────────────────────

class QuerySolution:
    def __getitem__(self, key: str | Variable | int) -> _Term | None: ...

class QuerySolutions:
    @property
    def variables(self) -> list[Variable]: ...
    def __iter__(self) -> QuerySolutions: ...
    def __next__(self) -> QuerySolution: ...
    def __len__(self) -> int: ...

class QueryTriples:
    def __iter__(self) -> QueryTriples: ...
    def __next__(self) -> Triple: ...
    def __len__(self) -> int: ...
    def serialize(self, format: RdfFormat) -> bytes: ...

class QueryBoolean:
    def __bool__(self) -> bool: ...

# ── Store / Dataset ─────────────────────────────────────────────────────────────

class QuadIter:
    def __iter__(self) -> QuadIter: ...
    def __next__(self) -> Quad: ...

class Store:
    def __init__(self) -> None: ...
    def __iter__(self) -> QuadIter: ...
    def load(
        self,
        input: bytes | str | None = ...,
        format: RdfFormat | None = ...,
        *,
        path: str | None = ...,
    ) -> None: ...
    def bulk_load(
        self,
        input: bytes | str | None = ...,
        format: RdfFormat | None = ...,
        *,
        path: str | None = ...,
    ) -> None: ...
    def add(self, quad: Quad) -> None: ...
    def remove(self, quad: Quad) -> None: ...
    # Engine configuration kwargs (unset = engine defaults): `extension_namespaces`
    # enables the closed extension-function set under the caller's namespaces (OFF
    # by default), `standpoint_predicates` is the `(according_to, sharpens)`
    # predicate table the `heldIn` extension requires.
    def query(
        self,
        query: str,
        *,
        substitutions: dict[Variable, _Term] | None = ...,
        extension_namespaces: list[str] | None = ...,
        standpoint_predicates: tuple[str, str] | None = ...,
    ) -> QuerySolutions | QueryTriples | QueryBoolean: ...
    def update(
        self,
        update: str,
        *,
        extension_namespaces: list[str] | None = ...,
        standpoint_predicates: tuple[str, str] | None = ...,
    ) -> None: ...
    @overload
    def dump(
        self,
        output: IO[bytes],
        format: RdfFormat,
        *,
        from_graph: NamedNode | BlankNode | DefaultGraph | None = ...,
    ) -> None: ...
    @overload
    def dump(
        self,
        output: None = ...,
        *,
        format: RdfFormat,
        from_graph: NamedNode | BlankNode | DefaultGraph | None = ...,
    ) -> bytes: ...
    def __len__(self) -> int: ...

class MutableDataset:
    def __init__(self) -> None: ...
    def __iter__(self) -> QuadIter: ...
    def load(
        self,
        input: bytes | str | None = ...,
        format: RdfFormat | None = ...,
        *,
        path: str | None = ...,
    ) -> None: ...
    def add(self, quad: Quad) -> bool: ...
    def remove(self, quad: Quad) -> bool: ...
    def contains(self, quad: Quad) -> bool: ...
    def quads_for_pattern(
        self,
        subject: _Subject | None = ...,
        predicate: NamedNode | None = ...,
        object: _Term | None = ...,
        graph_name: NamedNode | BlankNode | DefaultGraph | None = ...,
        *,
        any_graph: bool = ...,
    ) -> list[Quad]: ...
    @overload
    def dump(
        self,
        output: IO[bytes],
        format: RdfFormat,
        *,
        from_graph: NamedNode | BlankNode | DefaultGraph | None = ...,
    ) -> None: ...
    @overload
    def dump(
        self,
        output: None = ...,
        *,
        format: RdfFormat,
        from_graph: NamedNode | BlankNode | DefaultGraph | None = ...,
    ) -> bytes: ...
    # Engine configuration kwargs: as on `Store.query` / `Store.update`.
    def query(
        self,
        query: str,
        *,
        substitutions: dict[Variable, _Term] | None = ...,
        extension_namespaces: list[str] | None = ...,
        standpoint_predicates: tuple[str, str] | None = ...,
    ) -> QuerySolutions | QueryTriples | QueryBoolean: ...
    def update(
        self,
        update: str,
        *,
        extension_namespaces: list[str] | None = ...,
        standpoint_predicates: tuple[str, str] | None = ...,
    ) -> None: ...
    def compact(self) -> None: ...
    def __len__(self) -> int: ...

class Dataset:
    def __init__(self, quads: object | None = ...) -> None: ...
    def add(self, quad: Quad) -> None: ...
    def canonicalize(self, algorithm: CanonicalizationAlgorithm) -> None: ...
    def __iter__(self) -> QuadIter: ...
    def __len__(self) -> int: ...

# ── Module functions ────────────────────────────────────────────────────────────

def parse(input: bytes | str, format: RdfFormat) -> list[Quad]: ...
@overload
def serialize(input: QueryTriples, output: IO[bytes], format: RdfFormat) -> None: ...
@overload
def serialize(
    input: QueryTriples, output: None = ..., *, format: RdfFormat
) -> bytes: ...
def xsd_value_compare(
    left_lexical: str,
    left_datatype: str,
    right_lexical: str,
    right_datatype: str,
) -> int | None: ...
def xsd_canonical_lexical(lexical: str, datatype: str) -> str | None: ...
def xsd_decode_binary(lexical: str, datatype: str) -> bytes | None: ...
def xsd_normalize_whitespace(lexical: str, datatype: str) -> str | None: ...

# ── SPARQL Results serialization / parsing (bindings/python/src/py_store/results.rs) ──
#
# The four W3C SPARQL Results formats are keyed by the short id `"json"` / `"xml"`
# / `"csv"` / `"tsv"`. Serialization is byte-deterministic; parsing supports
# JSON and XML only (CSV/TSV have no native reader).

#: A SELECT row: one cell per projected variable, `None` for an unbound binding.
_ResultRow = list[_Term | None]

def serialize_sparql_solutions(
    format: str, variables: list[str], rows: list[_ResultRow]
) -> bytes: ...
def serialize_sparql_boolean(format: str, value: bool) -> bytes: ...

# A parsed SELECT is `("SELECT", variables, rows)`; a parsed ASK is `("ASK", bool)`
# — a heterogeneous tuple discriminated by its first element.
def parse_sparql_results(format: str, data: bytes) -> tuple[Any, ...]: ...

# ── RDF → GTS producer (bindings/python/src/py_gts.rs, #819 Task 8) ──────────────

#: A `(data, media_type, rep)` content-addressed blob row.
_BlobRow = tuple[bytes, str, str]
#: A `(slice_iri, slice_name, role, logical_path, content)` row (#820 S3).
_SliceArtifactRow = tuple[str, str, str, str, bytes]
#: A `(data, format, graph_name, scope)` named-graph ingest row.
_NamedGraphRow = tuple[bytes, RdfFormat, str | None, str | None]

def gts_from_quads(
    data: bytes,
    *,
    format: RdfFormat,
    profile: str = ...,
    transform: list[str] | None = ...,
) -> bytes: ...
def gts_from_rdf12_bytes(
    data: bytes,
    *,
    format: RdfFormat,
    profile: str = ...,
    transform: list[str] | None = ...,
) -> bytes: ...
def compile_gts_native(
    base_data: bytes,
    base_format: RdfFormat,
    *,
    base_scope: str | None = ...,
    rdf12_data: bytes | None = ...,
    rdf12_format: RdfFormat | None = ...,
    rdf12_graph_name: str | None = ...,
    rdf12_scope: str | None = ...,
    named_graphs: list[_NamedGraphRow] | None = ...,
    transform: list[str] | None = ...,
    doc_blobs: list[_BlobRow] | None = ...,
    report_blobs: list[_BlobRow] | None = ...,
    slice_artifacts: list[_SliceArtifactRow] | None = ...,
    signer_secret: bytes | None = ...,
    signer_kid: str | None = ...,
    public_key_armor: str | None = ...,
    rsyncable_threshold: int = ...,
) -> bytes: ...
def snapshot_content_id_native(data: bytes, *, format: RdfFormat) -> str: ...

# ── Text-format codecs via purrdf-gts (JSON-LD-star + RDF/XML, #834) ──────────────
# RDF bytes ↔ JSON-LD-star / RDF/XML through the purrdf-gts codec set. The compat
# `Graph.serialize`/`parse` route these formats here; serialize takes RDF bytes in
# `format` and returns the text form, parse takes the text and returns N-Quads bytes.
def to_json_ld(data: bytes, *, format: RdfFormat) -> str: ...

# `statement_vocab` is the caller-supplied statement-metadata vocabulary
# (keys: class/subject/predicate/object/objectLiteral, each an absolute IRI).
# When given, RDF-1.2 star features are downcast to flat statement-metadata
# cells in that vocabulary; PurRDF mints no default vocabulary of its own.
def from_json_ld(
    text: str, *, statement_vocab: dict[str, str] | None = ...
) -> bytes: ...
def to_rdf_xml(data: bytes, *, format: RdfFormat) -> str: ...
def from_rdf_xml(text: str) -> bytes: ...
def feedback_bundle_native(
    data: bytes,
    *,
    format: RdfFormat,
    report_blobs: list[_BlobRow] | None = ...,
) -> bytes: ...

# ── GTS fold view and relational exports (bindings/python/src/py_gts_view.rs) ───

_TermRow = tuple[int, int, str | None, int | None, str | None, int | None]
_QuadRow = tuple[int, int, int, int | None]
_ReifierRow = tuple[int, int, int, int]
_AnnotationRow = tuple[int, int, int]
_BlobExportRow = tuple[str, bytes]
_InputTermRow = tuple[int, str | None, int | None, str | None, str | None, int | None]

class GtsRelationalRows(TypedDict):
    terms: list[_TermRow]
    quads: list[_QuadRow]
    reifiers: list[_ReifierRow]
    annotations: list[_AnnotationRow]
    blobs: list[_BlobExportRow]

class GtsFoldViewNative:
    @staticmethod
    def from_bytes(data: bytes) -> GtsFoldViewNative: ...
    @staticmethod
    def from_parts(
        terms: list[_InputTermRow],
        quads: list[_QuadRow],
        reifiers: list[tuple[int, tuple[int, int, int]]],
        annotations: list[_AnnotationRow],
    ) -> GtsFoldViewNative: ...
    def term_count(self) -> int: ...
    def quad_count(self) -> int: ...
    def reifier_count(self) -> int: ...
    def annotation_count(self) -> int: ...
    def term_tuple(self, tid: int) -> _InputTermRow: ...
    def is_iri(self, tid: int) -> bool: ...
    def is_bnode(self, tid: int) -> bool: ...
    def is_literal(self, tid: int) -> bool: ...
    def iri(self, tid: int) -> str | None: ...
    def lex(self, tid: int) -> str: ...
    def lang(self, tid: int) -> str | None: ...
    def datatype(self, tid: int) -> str: ...
    def nq_token(self, tid: int) -> str: ...
    def python_value(self, tid: int) -> object: ...
    def tid_of_iri(self, iri: str) -> int | None: ...
    def curie(self, iri: str) -> str: ...
    def quads(self, scope: str | None = ...) -> list[_QuadRow]: ...
    def subjects_by_type(
        self, class_iri: str, scope: str | None = ...
    ) -> list[int]: ...
    def objects(self, s_tid: int, p_iri: str, scope: str | None = ...) -> list[int]: ...
    def value(self, s_tid: int, p_iri: str, scope: str | None = ...) -> int | None: ...
    def predicate_objects(
        self, s_tid: int, scope: str | None = ...
    ) -> list[tuple[int, int]]: ...
    def has(
        self, s_tid: int, p_iri: str, o_tid: int, scope: str | None = ...
    ) -> bool: ...
    def rdf_list(self, head_tid: int, scope: str | None = ...) -> list[int]: ...
    def reifiers(self) -> list[tuple[int, tuple[int, int, int]]]: ...
    def annotations(self) -> list[_AnnotationRow]: ...
    def tag_map(self) -> dict[str, str]: ...
    def available_languages(self) -> list[str]: ...
    def public_text(self, s_tid: int, p_iri: str, scope: str | None = ...) -> str: ...
    def public_literal(
        self, s_tid: int, p_iri: str, scope: str | None = ...
    ) -> tuple[str, str | None]: ...
    def public_literal_with_fallback(
        self,
        s_tid: int,
        p_iri: str,
        requested: list[str],
        scope: str | None = ...,
    ) -> tuple[str, str | None, bool]: ...
    def public_text_with_fallback(
        self,
        s_tid: int,
        p_iri: str,
        requested: list[str],
        scope: str | None = ...,
    ) -> tuple[str, bool]: ...
    def public_texts(
        self,
        s_tid: int,
        p_iri: str,
        requested: list[str],
        scope: str | None = ...,
    ) -> list[tuple[str, str | None, bool]]: ...
    def relational_rows(self) -> GtsRelationalRows: ...

def gts_relational_rows_from_bytes(data: bytes) -> GtsRelationalRows: ...
def gts_to_sqlite(data: bytes, path: str) -> str: ...
def gts_to_duckdb(data: bytes, path: str) -> str: ...
def gts_to_parquet(data: bytes, out_dir: str) -> list[str]: ...

# A Python handle to a frozen, immutable RDF 1.2 dataset (#819 C7 foundation).
class RdfDataset:
    def __init__(self, data: bytes | str, format: RdfFormat) -> None: ...
    def quad_count(self) -> int: ...
    def term_count(self) -> int: ...
    def __len__(self) -> int: ...
    def to_gts(self, profile: str = ...) -> bytes: ...

# ── Native SSSOM codec (bindings/python/src/py_sssom.rs, #848) ──────────────────
# Parse + validate + RDF serialize for PurRDF SSSOM TSV mapping artifacts — the
# in-repo replacement for the external `sssom` package. `validate_sssom` returns
# one `SssomDiagnostic` dict per diagnostic (a parse failure surfaces as a single
# `severity="FATAL"`, `check="parse"` dict); a clean file yields `[]`.
class SssomDiagnostic(TypedDict):
    severity: str
    code: str
    message: str
    check: str
    instance: str | None

def validate_sssom(text: str) -> list[SssomDiagnostic]: ...
def sssom_to_rdf(text: str) -> str: ...
def sssom_roundtrip_tsv(text: str) -> str: ...
def sssom_default_validation_types() -> list[str]: ...

# ── ShEx 2.1 engine (bindings/python/src/py_shex.rs, purrdf_native.shex) ─────────
# The native `purrdf_native.shex` submodule, re-attached as `purrdf.shex` by the
# `__init__.py` shim. Declared here as a class-namespace so the single-stub
# layout stays the one ABI source of truth.

class ShexResultEntry(TypedDict):
    """One fixed-shape-map verdict: the input `(node, shape)` echoed verbatim."""

    node: str
    shape: str
    conformant: bool
    reason: str | None

class shex:
    # Validate a fixed shape map: `map` pairs a focus node (IRI — bare or
    # `<…>`-wrapped —, `_:`-prefixed blank node, or Turtle literal token) with a
    # shape label, or the literal string "START" for the schema's start shape.
    # `schema_format` is "shexc" (default) or "shexj"; `data_format` is "turtle"
    # (default), "ntriples", or "nquads"; `base` resolves relative IRIs in the
    # schema and data. Typed engine errors raise ValueError.
    @staticmethod
    def validate(
        schema: str,
        data: str,
        map: list[tuple[str, str]],
        *,
        schema_format: str = ...,
        data_format: str = ...,
        base: str | None = ...,
    ) -> list[ShexResultEntry]: ...
    # Parse a ShEx schema ("shexc" or "shexj") and return its canonical ShExJ
    # JSON text, for schema tooling and cross-syntax round-trips.
    @staticmethod
    def parse(
        schema: str,
        *,
        format: str = ...,
        base: str | None = ...,
    ) -> str: ...

# ── Top-level engine submodules (attached by the __init__.py shim) ───────────────
# Mirroring the Rust `purrdf` umbrella crate, the SHACL / slice / GTS engines are
# reachable directly off `purrdf` — no caller touches `purrdf_native`. Declared
# here (the one ABI source of truth) as class-namespaces, exactly like `shex`.
# Engine classes carry an underscore-prefixed module-level name and are re-exported
# under their public name inside each namespace: a plain `X = X` in a class body
# reads as a self-referential type alias, so the indirection is deliberate.

# ── SHACL engine (bindings/python/src/shacl.rs, purrdf_native.shacl) ─────────────
# `purrdf.shapes` is the canonical (Rust-parity) name; `purrdf.shacl` is an alias.

class _ValidationReport:
    """A SHACL validation report."""

    @property
    def conforms(self) -> bool: ...
    @property
    def results(self) -> list[dict[str, builtins.object]]: ...
    def to_ntriples(self) -> str: ...

class _Shapes:
    """Parsed SHACL shapes, reusable across many data graphs."""

    def __init__(self, shapes_ttl: str) -> None: ...
    def validate_nt(self, data_nt: str) -> _ValidationReport: ...
    def validate_store(self, data: Store | MutableDataset) -> _ValidationReport: ...

class shapes:
    ValidationReport = _ValidationReport
    Shapes = _Shapes
    # Validate a data graph (N-Triples) against a shapes graph (Turtle).
    @staticmethod
    def validate(shapes_ttl: str, data_nt: str) -> dict[str, builtins.object]: ...

# Back-compat alias for the native submodule's own name.
shacl = shapes

# ── GTS surface grouping (purrdf.gts) ────────────────────────────────────────────
# The GTS entry points are also present at the purrdf root (declared above); the
# `gts` namespace groups them to mirror the Rust umbrella's `purrdf::gts` module.

_gts_from_quads = gts_from_quads
_gts_from_rdf12_bytes = gts_from_rdf12_bytes
_compile_gts_native = compile_gts_native
_snapshot_content_id_native = snapshot_content_id_native
_feedback_bundle_native = feedback_bundle_native
_to_json_ld = to_json_ld
_from_json_ld = from_json_ld
_to_rdf_xml = to_rdf_xml
_from_rdf_xml = from_rdf_xml
_gts_relational_rows_from_bytes = gts_relational_rows_from_bytes
_gts_to_sqlite = gts_to_sqlite
_gts_to_duckdb = gts_to_duckdb
_gts_to_parquet = gts_to_parquet
_RdfDataset = RdfDataset
_GtsFoldViewNative = GtsFoldViewNative

class gts:
    gts_from_quads = _gts_from_quads
    gts_from_rdf12_bytes = _gts_from_rdf12_bytes
    compile_gts_native = _compile_gts_native
    snapshot_content_id_native = _snapshot_content_id_native
    feedback_bundle_native = _feedback_bundle_native
    to_json_ld = _to_json_ld
    from_json_ld = _from_json_ld
    to_rdf_xml = _to_rdf_xml
    from_rdf_xml = _from_rdf_xml
    gts_relational_rows_from_bytes = _gts_relational_rows_from_bytes
    gts_to_sqlite = _gts_to_sqlite
    gts_to_duckdb = _gts_to_duckdb
    gts_to_parquet = _gts_to_parquet
    RdfDataset = _RdfDataset
    GtsFoldViewNative = _GtsFoldViewNative

# ── Slice tooling (bindings/python/src/py_slice.rs, purrdf_native.slice) ─────────
# Project artifact/dependency tooling, surfaced as `purrdf.slice`.

class _ArtifactRecord:
    @property
    def role(self) -> str: ...
    @property
    def logical_path(self) -> str: ...
    @property
    def media_type(self) -> str: ...
    @property
    def raw_digest(self) -> str: ...
    @property
    def semantic_digest(self) -> str: ...
    @property
    def content(self) -> builtins.bytes: ...

class _ManifestView:
    @property
    def identifier(self) -> str: ...
    @property
    def slice_iri(self) -> str: ...
    @property
    def label(self) -> str | None: ...
    @property
    def title(self) -> str | None: ...
    @property
    def tier(self) -> str | None: ...
    @property
    def creators(self) -> list[str]: ...
    @property
    def consumers(self) -> list[str]: ...

class _SliceRecord:
    @property
    def manifest(self) -> _ManifestView: ...
    @property
    def manifest_path(self) -> str: ...
    @property
    def slice_dir(self) -> str: ...
    @property
    def artifacts(self) -> list[_ArtifactRecord]: ...

class _DependencyEdge:
    @property
    def from_slice(self) -> str: ...
    @property
    def to_slice(self) -> str: ...
    @property
    def is_semantic(self) -> bool: ...
    @property
    def reconciliation(self) -> str: ...

class _ManifestPatch:
    @property
    def manifest_path(self) -> str: ...
    @property
    def original_text(self) -> str: ...
    @property
    def patched_text(self) -> str: ...

class _OwnershipReport:
    @property
    def edges(self) -> list[_DependencyEdge]: ...
    @property
    def has_ownership_defect(self) -> bool: ...
    @property
    def ownership_errors(self) -> list[str]: ...

class _SliceCatalog:
    @staticmethod
    def discover(root: str, namespace: str) -> _SliceCatalog: ...
    @property
    def records(self) -> list[_SliceRecord]: ...
    @property
    def core_slice_iris(self) -> list[str]: ...
    def fix_deps(self) -> list[_ManifestPatch]: ...

class _OwnershipAnalyzer:
    def analyze(self) -> _OwnershipReport: ...
    def analysis_graph_turtle(self) -> str: ...

class slice:
    ArtifactRecord = _ArtifactRecord
    ManifestView = _ManifestView
    SliceRecord = _SliceRecord
    DependencyEdge = _DependencyEdge
    ManifestPatch = _ManifestPatch
    OwnershipReport = _OwnershipReport
    SliceCatalog = _SliceCatalog
    OwnershipAnalyzer = _OwnershipAnalyzer
