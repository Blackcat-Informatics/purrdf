# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

# Type stub for the purrdf PyO3 extension. The signatures are transcribed
# verbatim from bindings/python/src/rdf.rs (the statement codec) and
# bindings/python/src/py_store.rs (the native Store / SPARQL / parse /
# canonicalize surface) — keep them in lockstep with those files (they are
# the ABI source of truth). This stub describes the native `purrdf` term /
# result / store surface — the in-repo binding that replaced the external RDF
# package that no longer exists.

from __future__ import annotations

import builtins
from collections.abc import Sequence
from typing import IO, Any, Callable, TypeAlias, TypedDict, overload

# `Literal` is aliased because this package DEFINES an RDF `Literal` class below.
# Importing typing's under its own name shadows it, and mypy then resolves the RDF
# class in `_Term` to `typing.Literal` and demands type parameters for it. Same
# reason `builtins` is imported qualified above.
from typing import Literal as TypingLiteral

# ── Statement codec (bindings/python/src/rdf.rs) ────────────────────────────────

def project_statements_rdf12(owl_ttl: str) -> str: ...
def normalize_rdf12_to_owl(rdf12_ttl: str) -> str: ...
def loss_matrix_json() -> str: ...
def rdf_gts_loss_matrix_json() -> str: ...
def canonicalize_turtle(
    turtle_bytes: bytes, extra_prefixes: list[tuple[str, str]] = ...
) -> bytes: ...

# ── Deterministic graph/tabular/research-object projection carriers ────────────

type ProjectionProfile = TypingLiteral[
    "lpg-csv",
    "neo4j-csv",
    "open-cypher",
    "graphml",
    "csvw-exact",
    "csvw-terms",
    "okf-terms",
    "obo-graphs",
    "skos",
    "croissant-1.1",
    "ro-crate-1.3",
    "datacite-4.6",
    "dcat-3",
    "dcat-rdf",
    "void",
    "frictionless-data-package-1",
]
type LiftProfile = TypingLiteral[
    "lpg-csv",
    "neo4j-csv",
    "open-cypher",
    "graphml",
    "csvw-exact",
    "croissant-1.1",
    "ro-crate-1.3",
    "datacite-4.6",
    "dcat-3",
    "frictionless-data-package-1",
]
type ArtifactEvent = TypingLiteral[
    "begin-package",
    "begin-artifact",
    "chunk",
    "finish-artifact",
    "commit-package",
    "abort-package",
]

class ProjectionLoss:
    @property
    def code(self) -> str: ...
    @property
    def source(self) -> str: ...
    @property
    def target(self) -> str: ...
    @property
    def note(self) -> str: ...
    @property
    def location(self) -> str | None: ...

class ProjectionPackage:
    @property
    def profile(self) -> str: ...
    @property
    def archive(self) -> bytes: ...
    @property
    def losses(self) -> list[ProjectionLoss]: ...

class ProjectionProgress:
    @property
    def phase(self) -> str: ...
    @property
    def input_records(self) -> int: ...
    @property
    def model_records(self) -> int: ...
    @property
    def nodes(self) -> int: ...
    @property
    def edges(self) -> int: ...
    @property
    def artifacts(self) -> int: ...
    @property
    def bytes(self) -> int: ...
    @property
    def path(self) -> str | None: ...

class ProjectionStream:
    @property
    def profile(self) -> str: ...
    @property
    def losses(self) -> list[ProjectionLoss]: ...
    @property
    def input_records(self) -> int: ...
    @property
    def model_records(self) -> int: ...
    @property
    def nodes(self) -> int: ...
    @property
    def edges(self) -> int: ...

class ProjectionLift:
    @property
    def dataset(self) -> RdfDataset: ...
    @property
    def losses(self) -> list[ProjectionLoss]: ...

def project(
    data: bytes | str,
    *,
    format: RdfFormat,
    profile: ProjectionProfile,
    config: bytes | str,
    assets: bytes | None = ...,
) -> ProjectionPackage: ...

def project_artifacts(
    data: bytes | str,
    *,
    format: RdfFormat,
    profile: TypingLiteral["lpg-csv", "neo4j-csv", "open-cypher", "graphml"],
    config: bytes | str,
    artifact_callback: Callable[[ArtifactEvent, str | None, bytes], None],
    progress_callback: Callable[[ProjectionProgress], None] | None = ...,
) -> ProjectionStream: ...

def lift(
    archive: bytes,
    *,
    profile: LiftProfile,
    config: bytes | str,
) -> ProjectionLift: ...

# ── Serialization / canonicalization enums ──────────────────────────────────────

class RdfFormat:
    TURTLE: RdfFormat
    N_TRIPLES: RdfFormat
    N_QUADS: RdfFormat
    TRIG: RdfFormat
    TRIX: RdfFormat
    HEXTUPLES: RdfFormat
    JSON_LD: RdfFormat
    YAML_LD: RdfFormat

class CompiledJsonLdContext:
    def __init__(self, options_json: str) -> None: ...
    @staticmethod
    def from_prefixes(prefixes: dict[str, str]) -> CompiledJsonLdContext: ...
    def canonical_context_json(self) -> str: ...

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
        base: str | None = ...,
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
        jsonld_options: str | None = ...,
        jsonld_context: CompiledJsonLdContext | None = ...,
        yaml_schema_url: str | None = ...,
    ) -> None: ...
    @overload
    def dump(
        self,
        output: None = ...,
        *,
        format: RdfFormat,
        from_graph: NamedNode | BlankNode | DefaultGraph | None = ...,
        jsonld_options: str | None = ...,
        jsonld_context: CompiledJsonLdContext | None = ...,
        yaml_schema_url: str | None = ...,
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
        base: str | None = ...,
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
        jsonld_options: str | None = ...,
        jsonld_context: CompiledJsonLdContext | None = ...,
        yaml_schema_url: str | None = ...,
    ) -> None: ...
    @overload
    def dump(
        self,
        output: None = ...,
        *,
        format: RdfFormat,
        from_graph: NamedNode | BlankNode | DefaultGraph | None = ...,
        jsonld_options: str | None = ...,
        jsonld_context: CompiledJsonLdContext | None = ...,
        yaml_schema_url: str | None = ...,
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

# ── RDF → GTS producer (bindings/python/src/py_gts.rs) ──────────────────────────

#: A `(data, media_type, rep)` content-addressed blob row.
_BlobRow = tuple[bytes, str, str]
#: A `(slice_iri, slice_name, role, logical_path, content)` row.
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

# ── Text-format codecs via purrdf-gts (JSON-LD-star + RDF/XML) ─────────────────
# RDF bytes ↔ JSON-LD-star / RDF/XML through the purrdf-gts codec set. The compat
# `Graph.serialize`/`parse` route these formats here; serialize takes RDF bytes in
# `format` and returns the text form, parse takes the text and returns N-Quads bytes.
def to_json_ld(
    data: bytes,
    *,
    format: RdfFormat,
    options_json: str | None = ...,
    context: CompiledJsonLdContext | None = ...,
) -> str: ...

def serialize_jsonld(
    data: bytes,
    *,
    format: RdfFormat,
    output_format: str,
    options_json: str | None = ...,
    context: CompiledJsonLdContext | None = ...,
    yaml_schema_url: str | None = ...,
) -> str: ...

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

# A Python handle to a frozen, immutable RDF 1.2 dataset.
class RdfDataset:
    def __init__(self, data: bytes | str, format: RdfFormat) -> None: ...
    def quad_count(self) -> int: ...
    def term_count(self) -> int: ...
    def __len__(self) -> int: ...
    # Canonical (RDFC-1.0) flat N-Quads — the readable surface of a frozen
    # dataset, and the same serializer the shared string boundary uses. N-Triples
    # is a syntactic subset, so a default-graph-only dataset serializes to a valid
    # N-Triples document; one that names graphs keeps the graph term. There is
    # deliberately no `to_ntriples` alias: one serializer, one name.
    def to_nquads(self) -> str: ...
    def serialize_jsonld(
        self,
        output_format: str,
        *,
        options_json: str | None = ...,
        context: CompiledJsonLdContext | None = ...,
        yaml_schema_url: str | None = ...,
    ) -> str: ...
    def to_gts(self, profile: str = ...) -> bytes: ...

# ── Native SSSOM codec (bindings/python/src/py_sssom.rs) ───────────────────────
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
    def to_sarif(self) -> str: ...

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
    # Entail a data graph (N-Triples) under a shapes graph (Turtle): apply every
    # SHACL-AF sh:rule to a fixpoint, returning the materialized dataset (base
    # graph plus every inferred triple) as a canonical N-Triples string.
    @staticmethod
    def entail(shapes_ttl: str, data_nt: str) -> str: ...

# Back-compat alias for the native submodule's own name.
shacl = shapes

# ── Entailment regimes (bindings/python/src/py_entail.rs, purrdf_native.entail) ──
# SPARQL entailment-regime materialization, surfaced as `purrdf.entail`. NOT the
# same mechanism as `shapes.entail` above: that one applies the SHACL-AF sh:rules
# a shapes graph declares, this one closes a document under a regime's own
# specification rule table (no shapes involved).

class _Regime:
    """A SPARQL entailment regime (`purrdf.entail.Regime.OWL_RL`)."""

    SIMPLE: _Regime
    RDF: _Regime
    RDFS: _Regime
    OWL_RL: _Regime
    OWL_DIRECT: _Regime
    RIF: _Regime
    D: _Regime

# Every entry point accepts either a `Regime` member or the regime's CLI spelling
# ("simple", "rdf", "rdfs", "owl-rl", "owl-direct", "rif", "d"); anything else
# raises ValueError naming the accepted set.
type RegimeLike = _Regime | str

class entail:
    # Spelled with an explicit `TypeAlias` (rather than the bare `X = _X` the
    # namespaces above use) because `purrdf.entail.Regime` is a *type* every call
    # site annotates with; a plain assignment reads to mypy as a variable and is
    # then rejected in annotation position.
    Regime: TypeAlias = _Regime
    # Close a frozen RdfDataset under `regime`, returning (closure, report). The
    # report is never optional: it names which rules fired, which specification
    # rules did not, and the calculus's contract hash. Read the closure with
    # `closure.to_nquads()`. Raises ValueError for an unknown regime spelling
    # and for a `program` that is wrong for the regime. EVERY regime
    # materializes, including `owl-direct` and `rif`; `program` is the rule
    # document `rif` entails under and must be `""` for every other regime,
    # because a caller who passed rules to `rdfs` believes they ran.
    @staticmethod
    def materialize(
        dataset: RdfDataset, regime: RegimeLike, program: str
    ) -> tuple[RdfDataset, str]: ...
    # The text-in/text-out twin of `materialize`: an N-Quads (or N-Triples)
    # document in, canonical (RDFC-1.0) N-Quads plus the rendered report out.
    @staticmethod
    def materialize_nt(
        data: str, regime: RegimeLike, program: str
    ) -> tuple[str, str]: ...
    # The rule table the specification DEFINES the regime by, in table order
    # (78 rules for OWL-RL, 18 for RDFS, 5 for D, 3 for RDF; `simple`, `owl-direct`
    # and `rif` have no specification table of their own, so `[]`).
    @staticmethod
    def rules(regime: RegimeLike) -> list[str]: ...
    # The subset of `rules(regime)` this workspace's chase actually fires. The
    # difference between the two is the regime's measurable gap.
    @staticmethod
    def implemented_rules(regime: RegimeLike) -> list[str]: ...
    # The rules this build fires BEYOND the specification table, disjoint from
    # both lists above. Non-normative and named, so a caller can tell what this
    # build adds without materializing a dataset to read a report line.
    @staticmethod
    def extensions(regime: RegimeLike) -> list[str]: ...

    # ── The OWL 2 Direct-Semantics reasoning services ────────────────────────
    # A different LANE from the four above. `materialize*` is the chase, whose
    # report reads `completeness exact | sound-incomplete <n>` — a difference of
    # two rule tables. Everything below is the tableau, whose certificate reads
    # `completeness decided | decided-within-boundaries | budget-exhausted`. The
    # DL lane has no rule table to subtract, so reusing the chase's notion would
    # report "exact" for a search that ran out of budget; the two renderings carry
    # different banners so neither can be parsed as the other.
    #
    # Every service returns `(answer, certificate)`. The pair is a tuple, so a
    # caller must UNPACK the evidence rather than being able to not ask for it.
    #
    # `step_cap` narrows the per-decision tableau step cap; 0 (the default) means
    # the knowledge base's own cap, NOT a cap of zero steps. It can only narrow,
    # so it cannot make a hard instance answerable — only make the
    # `budget-exhausted` certificate reachable.

    # Does the knowledge base have a model at all? The answer is one line,
    # `consistency true|false|unknown`. The only DL service that answers for an
    # unsatisfiable ontology, because it is the one that detects one.
    @staticmethod
    def consistency(data: str, step_cap: int = ...) -> tuple[str, str]: ...
    # The entailed subsumption hierarchy over the named classes: `equivalent`,
    # `subclass` (the full transitive closure), `direct` (its reduction) and
    # `unsatisfiable` lines, in that block order. Raises ValueError for an
    # ontology with no model, where every class subsumes every other.
    @staticmethod
    def classify(data: str, step_cap: int = ...) -> tuple[str, str]: ...
    # The entailed types of the named individuals (`type` lines) and the most
    # specific of them (`direct-type` lines).
    @staticmethod
    def realize(data: str, step_cap: int = ...) -> tuple[str, str]: ...
    # The named individuals entailed to be instances of `class_`, as
    # `instance <term>` lines. `class_` is ONE N-Triples term, angle brackets
    # included. A class the ontology never mentions yields an empty answer, which
    # is a real answer rather than an error.
    @staticmethod
    def instances(data: str, class_: str, step_cap: int = ...) -> tuple[str, str]: ...
    # Does the ontology entail `axiom`? `axiom` is ONE triple of the OWL 2 RDF
    # mapping: rdfs:subClassOf, owl:equivalentClass, owl:disjointWith, rdf:type,
    # owl:sameAs, owl:differentFrom and rdfs:subPropertyOf select the seven named
    # axiom kinds, and any other predicate is an object-property assertion. The
    # answer is `entails true|false|unknown` followed by the axiom AS READ.
    @staticmethod
    def entails(data: str, axiom: str, step_cap: int = ...) -> tuple[str, str]: ...
    # Which OWL 2 profiles the ontology is provably in (`certified <profile>`
    # lines, most restrictive first: EL, QL, RL, DL, Full) and what blocked the
    # others. Purely syntactic, so the certificate is an OWL profile certificate
    # rather than a DL one — there is no search whose completeness to report.
    @staticmethod
    def profile(data: str) -> tuple[str, str]: ...
    # The locality module for a seed signature (one N-Triples term per line) under
    # `method` ("bot", "top" or "star"). The answer is the module as canonical
    # N-Quads; the certificate's `conservative` line says whether it is the
    # minimal module or a sound superset.
    @staticmethod
    def extract_module(data: str, signature: str, method: str) -> tuple[str, str]: ...
    # WHY a DL axiom is entailed: a minimal subset of the ontology that still
    # entails it, as canonical N-Quads. A tableau performs no derivation steps, so
    # this is a JUSTIFICATION and deliberately not called a proof. The
    # certificate's `sufficient` and `minimal` lines are RE-DECIDED here, so they
    # check the answer rather than restate it.
    @staticmethod
    def justify(data: str, axiom: str) -> tuple[str, str]: ...
    # WHY one triple of a chase closure holds: which rules, from which premises.
    # `conclusion` is ONE N-Quads statement. The certificate's `derived-*` lines
    # are what the CHECKER re-derived from the proof term and the clause program,
    # not what the proof claims. Raises ValueError for RDF and RDFS, four of whose
    # rules have existential heads with no checkable proof term.
    @staticmethod
    def explain_conclusion(
        data: str, regime: RegimeLike, conclusion: str
    ) -> tuple[str, str]: ...

    # ── Conclusion-directed entailment (the CHASE lane, not the tableau) ─────
    # The CERTAIN ANSWERS of a basic graph pattern: the substitutions the
    # knowledge base ENTAILS the pattern under — true in every model, not merely
    # present in one closure, which is what SPARQL's entailment regimes define
    # the answers to a basic graph pattern to be. `pattern` is N-Triples with
    # `?name` in any position; a blank node in it is a NON-DISTINGUISHED
    # variable, constrained by the match and not projected. The answer opens
    # `mechanism <name>`, then `var` and `row` lines, then a `limit` line
    # per reason the row set may not be EXHAUSTIVE — no `limit` lines is the
    # claim that it is. A pattern with a projected variable is `strict-table`,
    # and a lane that would have been needed for it names itself in a `limit`;
    # a pattern with NO projected variable is a conclusion graph, is answered by
    # the same fold `graph_entails` runs, and names whichever of the seven
    # reached it. Raises ValueError for OWL_DIRECT and RIF, each defined by
    # an input this signature does not carry.
    #
    # `imports` is the caller's `owl:imports` table: an ORDERED sequence of
    # `(ontology_iri, document)` pairs, `document` being N-Quads text exactly
    # like `data`. A premise carrying an `owl:imports` states that its axioms
    # are its own PLUS those of the documents it names, so this is where those
    # documents arrive. PurRDF FETCHES NOTHING: an ontology IRI the sequence
    # does not resolve raises ValueError naming the document, never a network
    # access and never a silently empty import. `[]` is the ordinary "imports
    # nothing" case; the argument is required, not defaulted, and sits in the
    # same position on all four hosts.
    @staticmethod
    def certain_answers(
        regime: RegimeLike,
        data: str,
        pattern: str,
        imports: Sequence[tuple[str, str]],
    ) -> tuple[str, str]: ...
    # Does `premise` entail the conclusion GRAPH under the regime's rule table?
    # NOT `entails`, which asks the OWL 2 Direct-Semantics TABLEAU about one
    # AXIOM and renders a DL certificate; this asks the RULE TABLE about a
    # conclusion GRAPH and renders a reasoning report. The answer opens
    # `mechanism <name>` — which of the six mechanisms reached the verdict — and
    # then gives THREE verdicts, never two: `not-entailed` is a PROOF, and
    # `undecided` is what an incomplete procedure is entitled to say instead.
    # `imports` is `certain_answers`'s, and applies to the PREMISE: the
    # conclusion is a graph to match, not an ontology to close.
    @staticmethod
    def graph_entails(
        regime: RegimeLike,
        premise: str,
        conclusion: str,
        imports: Sequence[tuple[str, str]],
    ) -> tuple[str, str]: ...
    # `graph_entails` with the warrant RE-DECIDED, without running a reasoner.
    # Adds `warrant present|absent` and `verified true|false|not-applicable`;
    # `warrant absent` is a not-entailed or an undecided, where there is no
    # evidence to re-decide and a `false` would read as a failed check rather
    # than an absent one. `imports` is `certain_answers`'s; the re-check runs
    # against the premise AS WRITTEN, which is a stronger check than one only
    # re-decidable against a graph the library assembled.
    @staticmethod
    def verify_entailment(
        regime: RegimeLike,
        premise: str,
        conclusion: str,
        imports: Sequence[tuple[str, str]],
    ) -> tuple[str, str]: ...

    # ── The session ──────────────────────────────────────────────────────────
    # Every service above takes the document as a string and rebuilds everything
    # it needs, so asking three questions parses and reverse-maps the ontology
    # three times. `Reasoner` holds the parsed document: constructing it parses
    # once, the first question needing a knowledge base reverse-maps once, and
    # later questions reuse both. The methods answer exactly what the same-named
    # functions answer — they ARE the session those functions now wrap — so
    # moving between the two cannot change an answer or a certificate.
    #
    # The knowledge base is built lazily and NOT by the constructor: `profile`,
    # `extract_module`, `justify` and `explain_conclusion` never reason, and
    # `profile` answers for any parseable document — including one whose
    # `owl:hasKey` axioms would exhaust the tableau while it was reverse-mapped.
    class Reasoner:
        def __init__(self, data: str, step_cap: int = ...) -> None: ...
        def consistency(self) -> tuple[str, str]: ...
        def classify(self) -> tuple[str, str]: ...
        def realize(self) -> tuple[str, str]: ...
        def instances(self, class_: str) -> tuple[str, str]: ...
        def entails(self, axiom: str) -> tuple[str, str]: ...
        def profile(self) -> tuple[str, str]: ...
        def extract_module(self, signature: str, method: str) -> tuple[str, str]: ...
        def justify(self, axiom: str) -> tuple[str, str]: ...
        def explain_conclusion(
            self, regime: RegimeLike, conclusion: str
        ) -> tuple[str, str]: ...
        def __repr__(self) -> str: ...

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
