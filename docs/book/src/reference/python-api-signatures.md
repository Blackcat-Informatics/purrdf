<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# Python API signatures

This reference records every declaration in the canonical Python stubs, including
classes, properties, overloads, type aliases and namespace members. It describes
the public package; underscore names also declare supporting types used in the
public signatures. They do not promise separately importable constructors.

The installed-package signature test compares both complete inventories using
Python syntax trees. Parameter kinds, annotations, defaults, overloads and property
decorators therefore cannot drift silently. The accompanying native parameter
comparison covers the engine namespaces; inventory parity alone does not prove
that every native callable implements its declared signature.

Read [Getting Started: Python](../getting-started/python.md) for usage and the
linked engine chapters for their semantic contracts. This reference preserves
Python identifiers in every language.

## `__init__.pyi`

<!-- python-api-stub: __init__.pyi -->
```pyi
from __future__ import annotations
import builtins
from collections.abc import Iterator, Mapping, Sequence
from types import CapsuleType
from typing import IO, Any, Callable, Required, TypeAlias, TypedDict, overload
from typing import Literal as TypingLiteral
from . import geo as geo
type GeoConfiguration = geo.GeoProfile | geo.GeoSession | str | None

def project_statements_rdf12(owl_ttl: str) -> str:
    ...

def normalize_rdf12_to_owl(rdf12_ttl: str) -> str:
    ...

def loss_matrix_json() -> str:
    ...

def rdf_gts_loss_matrix_json() -> str:
    ...

def canonicalize_turtle(turtle_bytes: bytes, extra_prefixes: list[tuple[str, str]]=...) -> bytes:
    ...
XPATH_REGEX_PROFILES: tuple[str, ...]
type ProjectionProfile = TypingLiteral['lpg-csv', 'neo4j-csv', 'open-cypher', 'graphml', 'csvw-exact', 'csvw-terms', 'okf-terms', 'obo-graphs', 'skos', 'croissant-1.1', 'ro-crate-1.3', 'datacite-4.6', 'dcat-3', 'dcat-rdf', 'void', 'frictionless-data-package-1']
type LiftProfile = TypingLiteral['lpg-csv', 'neo4j-csv', 'open-cypher', 'graphml', 'csvw-exact', 'croissant-1.1', 'ro-crate-1.3', 'datacite-4.6', 'dcat-3', 'frictionless-data-package-1']
type ArtifactEvent = TypingLiteral['begin-package', 'begin-artifact', 'chunk', 'finish-artifact', 'commit-package', 'abort-package']

class ProjectionLoss:

    @property
    def code(self) -> str:
        ...

    @property
    def source(self) -> str:
        ...

    @property
    def target(self) -> str:
        ...

    @property
    def note(self) -> str:
        ...

    @property
    def location(self) -> str | None:
        ...

class ProjectionPackage:

    @property
    def profile(self) -> str:
        ...

    @property
    def archive(self) -> bytes:
        ...

    @property
    def losses(self) -> list[ProjectionLoss]:
        ...

class ProjectionProgress:

    @property
    def phase(self) -> str:
        ...

    @property
    def input_records(self) -> int:
        ...

    @property
    def model_records(self) -> int:
        ...

    @property
    def nodes(self) -> int:
        ...

    @property
    def edges(self) -> int:
        ...

    @property
    def artifacts(self) -> int:
        ...

    @property
    def bytes(self) -> int:
        ...

    @property
    def path(self) -> str | None:
        ...

class ProjectionStream:

    @property
    def profile(self) -> str:
        ...

    @property
    def losses(self) -> list[ProjectionLoss]:
        ...

    @property
    def input_records(self) -> int:
        ...

    @property
    def model_records(self) -> int:
        ...

    @property
    def nodes(self) -> int:
        ...

    @property
    def edges(self) -> int:
        ...

class ProjectionLift:

    @property
    def dataset(self) -> RdfDataset:
        ...

    @property
    def losses(self) -> list[ProjectionLoss]:
        ...

def project(data: bytes | str, *, format: RdfFormat, profile: ProjectionProfile, config: bytes | str, assets: bytes | None=...) -> ProjectionPackage:
    ...

def project_artifacts(data: bytes | str, *, format: RdfFormat, profile: TypingLiteral['lpg-csv', 'neo4j-csv', 'open-cypher', 'graphml'], config: bytes | str, artifact_callback: Callable[[ArtifactEvent, str | None, bytes], None], progress_callback: Callable[[ProjectionProgress], None] | None=...) -> ProjectionStream:
    ...

def lift(archive: bytes, *, profile: LiftProfile, config: bytes | str) -> ProjectionLift:
    ...

class RdfFormat:
    TURTLE: RdfFormat
    N_TRIPLES: RdfFormat
    N_QUADS: RdfFormat
    TRIG: RdfFormat
    TRIX: RdfFormat
    HEXTUPLES: RdfFormat
    JSON_LD: RdfFormat
    YAML_LD: RdfFormat
    RDF_XML: RdfFormat

class CompiledJsonLdContext:

    def __init__(self, options_json: str) -> None:
        ...

    @staticmethod
    def from_prefixes(prefixes: dict[str, str]) -> CompiledJsonLdContext:
        ...

    def canonical_context_json(self) -> str:
        ...

class CanonicalizationAlgorithm:
    RDFC_1_0: CanonicalizationAlgorithm
    UNSTABLE: CanonicalizationAlgorithm

class NamedNode:

    def __init__(self, value: str) -> None:
        ...

    @property
    def value(self) -> str:
        ...

    def __hash__(self) -> int:
        ...

    def __eq__(self, other: object) -> bool:
        ...

class BlankNode:

    def __init__(self, value: str) -> None:
        ...

    @property
    def value(self) -> str:
        ...

    def __hash__(self) -> int:
        ...

    def __eq__(self, other: object) -> bool:
        ...

class Literal:

    def __init__(self, value: str, *, datatype: NamedNode | None=..., language: str | None=..., direction: str | None=...) -> None:
        ...

    @property
    def value(self) -> str:
        ...

    @property
    def language(self) -> str | None:
        ...

    @property
    def direction(self) -> str | None:
        ...

    @property
    def datatype(self) -> NamedNode:
        ...

    def __hash__(self) -> int:
        ...

    def __eq__(self, other: object) -> bool:
        ...

class Triple:

    def __init__(self, subject: _Subject, predicate: NamedNode, object: _Term) -> None:
        ...

    @property
    def subject(self) -> _Subject:
        ...

    @property
    def predicate(self) -> NamedNode:
        ...

    @property
    def object(self) -> _Term:
        ...

    def __hash__(self) -> int:
        ...

    def __eq__(self, other: builtins.object) -> bool:
        ...

class DefaultGraph:

    def __init__(self) -> None:
        ...

class Quad:

    def __init__(self, subject: _Subject, predicate: NamedNode, object: _Term, graph_name: NamedNode | BlankNode | DefaultGraph | None=...) -> None:
        ...

    @property
    def subject(self) -> _Subject:
        ...

    @property
    def predicate(self) -> NamedNode:
        ...

    @property
    def object(self) -> _Term:
        ...

    @property
    def graph_name(self) -> NamedNode | BlankNode | DefaultGraph:
        ...

    def __hash__(self) -> int:
        ...

    def __eq__(self, other: builtins.object) -> bool:
        ...

class Variable:

    def __init__(self, value: str) -> None:
        ...

    @property
    def value(self) -> str:
        ...

    def __hash__(self) -> int:
        ...

    def __eq__(self, other: object) -> bool:
        ...
_Subject = NamedNode | BlankNode
_Term = NamedNode | BlankNode | Literal | Triple
_Attestation = tuple[str | None, str | None]
_Relation = tuple[int, int, Sequence[Sequence[_Term]]] | tuple[int, int, Sequence[Sequence[_Term]], _Attestation]
_RelationFromGraph = tuple[_Term, int, int] | tuple[_Term, int, int, _Attestation]
_PathStep = tuple[_Term, str]
_PathRelation = tuple[Sequence[_PathStep], int, int, int, int, str] | tuple[Sequence[_PathStep], int, int, int, int, str, _Attestation]

class QuerySolution:

    def __getitem__(self, key: str | Variable | int) -> _Term | None:
        ...

    def __len__(self) -> int:
        ...

    def __iter__(self) -> Iterator[_Term | None]:
        ...

class _QuerySolutionIterator:

    def __iter__(self) -> _QuerySolutionIterator:
        ...

    def __next__(self) -> _Term | None:
        ...

    def __length_hint__(self) -> int:
        ...

class QuerySolutions:

    @property
    def execution_receipt_json(self) -> str | None:
        ...

    @property
    def variables(self) -> list[Variable]:
        ...

    def __iter__(self) -> QuerySolutions:
        ...

    def __next__(self) -> QuerySolution:
        ...

    def __len__(self) -> int:
        ...

class QueryTriples:

    @property
    def execution_receipt_json(self) -> str | None:
        ...

    def __iter__(self) -> QueryTriples:
        ...

    def __next__(self) -> Triple:
        ...

    def __len__(self) -> int:
        ...

    def serialize(self, format: RdfFormat, *, base: str | None=...) -> bytes:
        ...

class QueryQuads:

    @property
    def execution_receipt_json(self) -> str | None:
        ...

    def __iter__(self) -> QueryQuads:
        ...

    def __next__(self) -> Quad:
        ...

    def __len__(self) -> int:
        ...

    @property
    def graph_names(self) -> list[str]:
        ...

    def serialize(self, format: RdfFormat) -> bytes:
        ...

class SerializeLoss:

    @property
    def bytes(self) -> bytes:
        ...

    @property
    def statement_rows_dropped(self) -> int:
        ...

    @property
    def directional_literals_dropped(self) -> int:
        ...

    @property
    def named_graph_rows_dropped(self) -> int:
        ...

    @property
    def empty_named_graphs_dropped(self) -> int:
        ...

class QueryBoolean:

    @property
    def execution_receipt_json(self) -> str | None:
        ...

    def __bool__(self) -> bool:
        ...
type GovernorKind = TypingLiteral['budget', 'stopped', 'refused', 'unknown']
type PartialCertainty = TypingLiteral['certain', 'at-most', 'unknown']

class CancellationToken:

    def __init__(self) -> None:
        ...

    def cancel(self) -> None:
        ...

    @property
    def cancelled(self) -> bool:
        ...

class TrippedGovernor:

    @property
    def kind(self) -> GovernorKind:
        ...

    @property
    def label(self) -> str:
        ...

    @property
    def dimension(self) -> str | None:
        ...

    @property
    def limit(self) -> int | None:
        ...

    @property
    def consumed(self) -> int | None:
        ...

    @property
    def estimate(self) -> int | None:
        ...

    @property
    def cause(self) -> str | None:
        ...

    def __str__(self) -> str:
        ...

class GovernorEvidence:

    @property
    def consumed(self) -> dict[str, int]:
        ...

    @property
    def limits(self) -> dict[str, int]:
        ...

    @property
    def tripped(self) -> TrippedGovernor | None:
        ...

    @property
    def is_complete(self) -> bool:
        ...

    @property
    def expression_errors(self) -> dict[str, int]:
        ...

    def consumed_in(self, dimension: str) -> int:
        ...

    def limit_for(self, dimension: str) -> int:
        ...

class PartialAnswers:

    @property
    def certainty(self) -> PartialCertainty:
        ...

    @property
    def is_certain(self) -> bool:
        ...

    @property
    def result(self) -> QuerySolutions | QueryTriples | QueryQuads | QueryBoolean | None:
        ...

    @property
    def is_positional_prefix(self) -> bool | None:
        ...

    @property
    def barrier(self) -> str | None:
        ...

class RelationAttestations(TypedDict):
    invocations: int
    generations: list[str | None]
    incompleteness: list[str]

class QueryOutcome:

    @property
    def execution_receipt_json(self) -> str:
        ...

    @property
    def is_complete(self) -> bool:
        ...

    @property
    def result(self) -> QuerySolutions | QueryTriples | QueryQuads | QueryBoolean | None:
        ...

    @property
    def partial(self) -> PartialAnswers | None:
        ...

    @property
    def tripped(self) -> TrippedGovernor | None:
        ...

    @property
    def evidence(self) -> GovernorEvidence:
        ...

    @property
    def relation_witness(self) -> dict[str, RelationAttestations]:
        ...

class EntailmentQueryOutcome:

    @property
    def phase(self) -> TypingLiteral['answered', 'closure-stopped']:
        ...

    @property
    def is_complete(self) -> bool:
        ...

    @property
    def outcome(self) -> QueryOutcome | None:
        ...

    @property
    def report(self) -> str | None:
        ...

    @property
    def tripped(self) -> TrippedGovernor | None:
        ...

class UpdateOutcome:

    @property
    def execution_receipt_json(self) -> str:
        ...

    @property
    def is_applied(self) -> bool:
        ...

    @property
    def tripped(self) -> TrippedGovernor | None:
        ...

    @property
    def evidence(self) -> GovernorEvidence | None:
        ...

class PreparedQuery:

    @property
    def parameters(self) -> list[str]:
        ...

    def run(self, bindings: dict[str, _Term] | None=..., *, now: str | None=..., entropy_seed: int | None=...) -> QuerySolutions | QueryTriples | QueryQuads | QueryBoolean:
        ...

class QuadIter:

    def __iter__(self) -> QuadIter:
        ...

    def __next__(self) -> Quad:
        ...

class _QuadStore:

    def __iter__(self) -> QuadIter:
        ...

    def add_graph(self, graph: NamedNode | BlankNode, *, silent: bool=False) -> None:
        ...

    def named_graphs(self) -> list[NamedNode | BlankNode]:
        ...

    def query(self, query: str, *, substitutions: dict[Variable, _Term] | None=..., extension_namespaces: list[str] | None=..., property_fn_namespaces: list[str] | None=..., standpoint_predicates: tuple[str, str] | None=..., relations: dict[str, _Relation] | None=..., relations_from_graph: dict[str, _RelationFromGraph] | None=..., path_relations: dict[str, _PathRelation] | None=..., aggregate_namespace: str | None=..., xpath_regex: str | None=..., geo: GeoConfiguration=..., division: str | None=..., now: str | None=..., entropy_seed: int | None=...) -> QuerySolutions | QueryTriples | QueryQuads | QueryBoolean:
        ...

    def query_rdflib(self, query: str, *, substitutions: dict[Variable, _Term] | None=..., extension_namespaces: list[str] | None=..., property_fn_namespaces: list[str] | None=..., standpoint_predicates: tuple[str, str] | None=..., relations: dict[str, _Relation] | None=..., relations_from_graph: dict[str, _RelationFromGraph] | None=..., path_relations: dict[str, _PathRelation] | None=..., aggregate_namespace: str | None=..., xpath_regex: str | None=..., geo: GeoConfiguration=..., division: str | None=..., now: str | None=..., entropy_seed: int | None=..., named_graphs: bool=..., default_graph: NamedNode | BlankNode | None=..., default_union: bool=...) -> QuerySolutions | QueryTriples | QueryQuads | QueryBoolean:
        ...

    def query_governed(self, query: str, *, substitutions: dict[Variable, _Term] | None=..., extension_namespaces: list[str] | None=..., property_fn_namespaces: list[str] | None=..., standpoint_predicates: tuple[str, str] | None=..., relations: dict[str, _Relation] | None=..., relations_from_graph: dict[str, _RelationFromGraph] | None=..., path_relations: dict[str, _PathRelation] | None=..., aggregate_namespace: str | None=..., xpath_regex: str | None=..., geo: GeoConfiguration=..., division: str | None=..., now: str | None=..., entropy_seed: int | None=..., fuel: int | None=..., deadline_ms: int | None=..., max_answers: int | None=..., max_intermediate_cells: int | None=..., max_scratch_bytes: int | None=..., max_remote_requests: int | None=..., no_ceiling: bool=False, cancel: CancellationToken | None=...) -> QueryOutcome:
        ...

    def query_entailment_governed(self, query: str, entailment: str, *, program: str=..., imports: Sequence[tuple[str, str]] | None=..., premise_iris: Sequence[str] | None=..., max_stored_facts: int | None=..., max_join_steps: int | None=..., max_term_arena_bytes: int | None=..., substitutions: dict[Variable, _Term] | None=..., extension_namespaces: list[str] | None=..., property_fn_namespaces: list[str] | None=..., standpoint_predicates: tuple[str, str] | None=..., relations: dict[str, _Relation] | None=..., relations_from_graph: dict[str, _RelationFromGraph] | None=..., path_relations: dict[str, _PathRelation] | None=..., aggregate_namespace: str | None=..., xpath_regex: str | None=..., geo: GeoConfiguration=..., division: str | None=..., now: str | None=..., entropy_seed: int | None=..., fuel: int | None=..., deadline_ms: int | None=..., max_answers: int | None=..., max_intermediate_cells: int | None=..., max_scratch_bytes: int | None=..., max_remote_requests: int | None=..., no_ceiling: bool=False, cancel: CancellationToken | None=...) -> EntailmentQueryOutcome:
        ...

    def update(self, update: str, *, extension_namespaces: list[str] | None=..., property_fn_namespaces: list[str] | None=..., standpoint_predicates: tuple[str, str] | None=..., relations: dict[str, _Relation] | None=..., relations_from_graph: dict[str, _RelationFromGraph] | None=..., path_relations: dict[str, _PathRelation] | None=..., aggregate_namespace: str | None=..., xpath_regex: str | None=..., geo: GeoConfiguration=..., division: str | None=..., now: str | None=..., entropy_seed: int | None=...) -> UpdateOutcome:
        ...

    def update_governed(self, update: str, *, extension_namespaces: list[str] | None=..., property_fn_namespaces: list[str] | None=..., standpoint_predicates: tuple[str, str] | None=..., relations: dict[str, _Relation] | None=..., relations_from_graph: dict[str, _RelationFromGraph] | None=..., path_relations: dict[str, _PathRelation] | None=..., aggregate_namespace: str | None=..., xpath_regex: str | None=..., geo: GeoConfiguration=..., division: str | None=..., now: str | None=..., entropy_seed: int | None=..., fuel: int | None=..., deadline_ms: int | None=..., max_intermediate_cells: int | None=..., max_scratch_bytes: int | None=..., max_remote_requests: int | None=..., no_ceiling: bool=False, cancel: CancellationToken | None=...) -> UpdateOutcome:
        ...

    def dump_with_loss(self, format: RdfFormat) -> SerializeLoss:
        ...

    def __len__(self) -> int:
        ...

    def _store_capsule(self) -> CapsuleType:
        ...

class Store(_QuadStore):

    def __init__(self, *, remember_empty_graphs: bool=True) -> None:
        ...

    def load(self, input: bytes | str | None=..., format: RdfFormat | None=..., *, path: str | None=..., base: str | None=...) -> list[tuple[str, str]]:
        ...

    def bulk_load(self, input: bytes | str | None=..., format: RdfFormat | None=..., *, path: str | None=..., base: str | None=...) -> list[tuple[str, str]]:
        ...

    def add(self, quad: Quad) -> None:
        ...

    def remove(self, quad: Quad) -> None:
        ...

    def checkpoint(self) -> None:
        ...

    def change_size(self) -> tuple[int, int]:
        ...

    def prepare(self, query: str, *, parameters: list[str] | None=..., extension_namespaces: list[str] | None=..., property_fn_namespaces: list[str] | None=..., standpoint_predicates: tuple[str, str] | None=..., relations: dict[str, _Relation] | None=..., relations_from_graph: dict[str, _RelationFromGraph] | None=..., path_relations: dict[str, _PathRelation] | None=..., aggregate_namespace: str | None=..., xpath_regex: str | None=..., geo: GeoConfiguration=..., division: str | None=...) -> PreparedQuery:
        ...

    @overload
    def dump(self, output: IO[bytes], format: RdfFormat, *, from_graph: NamedNode | BlankNode | DefaultGraph | None=..., jsonld_options: str | None=..., jsonld_context: CompiledJsonLdContext | None=..., yaml_schema_url: str | None=..., base: str | None=...) -> None:
        ...

    @overload
    def dump(self, output: None=..., *, format: RdfFormat, from_graph: NamedNode | BlankNode | DefaultGraph | None=..., jsonld_options: str | None=..., jsonld_context: CompiledJsonLdContext | None=..., yaml_schema_url: str | None=..., base: str | None=...) -> bytes:
        ...

class MutableDataset(_QuadStore):

    def __init__(self, *, remember_empty_graphs: bool=True) -> None:
        ...

    def load(self, input: bytes | str | None=..., format: RdfFormat | None=..., *, path: str | None=..., base: str | None=...) -> list[tuple[str, str]]:
        ...

    def add(self, quad: Quad) -> bool:
        ...

    def remove(self, quad: Quad) -> bool:
        ...

    def contains(self, quad: Quad) -> bool:
        ...

    def quads_for_pattern(self, subject: _Subject | None=..., predicate: NamedNode | None=..., object: _Term | None=..., graph_name: NamedNode | BlankNode | DefaultGraph | None=..., *, any_graph: bool=...) -> list[Quad]:
        ...

    @overload
    def dump(self, output: IO[bytes], format: RdfFormat, *, from_graph: NamedNode | BlankNode | DefaultGraph | None=..., jsonld_options: str | None=..., jsonld_context: CompiledJsonLdContext | None=..., yaml_schema_url: str | None=..., base: str | None=...) -> None:
        ...

    @overload
    def dump(self, output: None=..., *, format: RdfFormat, from_graph: NamedNode | BlankNode | DefaultGraph | None=..., jsonld_options: str | None=..., jsonld_context: CompiledJsonLdContext | None=..., yaml_schema_url: str | None=..., base: str | None=...) -> bytes:
        ...

    def compact(self) -> None:
        ...

class Dataset:

    def __init__(self, quads: object | None=...) -> None:
        ...

    def add(self, quad: Quad) -> None:
        ...

    def canonicalize(self, algorithm: CanonicalizationAlgorithm) -> None:
        ...

    def __iter__(self) -> QuadIter:
        ...

    def __len__(self) -> int:
        ...

def parse(input: bytes | str, format: RdfFormat, *, base: str | None=...) -> list[Quad]:
    ...

@overload
def serialize(input: QueryTriples | QueryQuads, output: IO[bytes], format: RdfFormat, *, base: str | None=...) -> None:
    ...

@overload
def serialize(input: QueryTriples | QueryQuads, output: None=..., *, format: RdfFormat, base: str | None=...) -> bytes:
    ...

def xsd_value_compare(left_lexical: str, left_datatype: str, right_lexical: str, right_datatype: str) -> int | None:
    ...

def xsd_canonical_lexical(lexical: str, datatype: str) -> str | None:
    ...

def xsd_decode_binary(lexical: str, datatype: str) -> bytes | None:
    ...

def xsd_normalize_whitespace(lexical: str, datatype: str) -> str | None:
    ...
_ResultRow = list[_Term | None]
_ProvenanceNamespace = tuple[str, str]
type SparqlResultsFormat = TypingLiteral['json', 'srj', 'sparql-json', 'application/sparql-results+json', 'xml', 'sparql-xml', 'application/sparql-results+xml', 'csv', 'text/csv', 'tsv', 'text/tab-separated-values']

def serialize_sparql_solutions(format: SparqlResultsFormat | str, variables: list[str], rows: list[_ResultRow], *, provenance_namespace: _ProvenanceNamespace | None=..., query_hash: str | None=...) -> bytes:
    ...

def serialize_sparql_boolean(format: SparqlResultsFormat | str, value: bool, *, provenance_namespace: _ProvenanceNamespace | None=..., query_hash: str | None=...) -> bytes:
    ...

def parse_sparql_results(format: SparqlResultsFormat | str, data: bytes) -> tuple[Any, ...]:
    ...
_ProvenanceDict = dict[str, str | None]

def provenance_from_json(data: bytes, prefix: str, iri: str) -> _ProvenanceDict:
    ...

def provenance_from_xml(data: bytes, prefix: str, iri: str) -> _ProvenanceDict:
    ...
_BlobRow = tuple[bytes, str, str]
_SliceArtifactRow = tuple[str, str, str, str, bytes]
_NamedGraphRow = tuple[bytes, RdfFormat, str | None, str | None]

def gts_from_quads(data: bytes, *, format: RdfFormat, profile: str=..., transform: list[str] | None=..., base: str | None=...) -> bytes:
    ...

def gts_from_rdf12_bytes(data: bytes, *, format: RdfFormat, profile: str=..., transform: list[str] | None=..., base: str | None=...) -> bytes:
    ...

def compile_gts_native(base_data: bytes, base_format: RdfFormat, *, base_scope: str | None=..., rdf12_data: bytes | None=..., rdf12_format: RdfFormat | None=..., rdf12_graph_name: str | None=..., rdf12_scope: str | None=..., named_graphs: list[_NamedGraphRow] | None=..., transform: list[str] | None=..., doc_blobs: list[_BlobRow] | None=..., report_blobs: list[_BlobRow] | None=..., slice_artifacts: list[_SliceArtifactRow] | None=..., signer_secret: bytes | None=..., signer_kid: str | None=..., public_key_armor: str | None=..., rsyncable_threshold: int=..., base: str | None=...) -> bytes:
    ...

def snapshot_content_id_native(data: bytes, *, format: RdfFormat, base: str | None=...) -> str:
    ...

class GtsIngestReport(TypedDict):
    rows_consumed: int
    terms_interned: int
    declarations_omitted: list[str]
    scratch_bytes: int

def gts_ingest_report(base_data: bytes, base_format: RdfFormat, *, base_scope: str | None=..., rdf12_data: bytes | None=..., rdf12_format: RdfFormat | None=..., rdf12_graph_name: str | None=..., rdf12_scope: str | None=..., named_graphs: list[_NamedGraphRow] | None=..., base: str | None=...) -> GtsIngestReport:
    ...

class GtsCompileResult(TypedDict):
    snapshot_bytes: bytes
    ingest_report: GtsIngestReport

def compile_gts_with_report(base_data: bytes, base_format: RdfFormat, *, base_scope: str | None=..., rdf12_data: bytes | None=..., rdf12_format: RdfFormat | None=..., rdf12_graph_name: str | None=..., rdf12_scope: str | None=..., named_graphs: list[_NamedGraphRow] | None=..., transform: list[str] | None=..., doc_blobs: list[_BlobRow] | None=..., report_blobs: list[_BlobRow] | None=..., slice_artifacts: list[_SliceArtifactRow] | None=..., signer_secret: bytes | None=..., signer_kid: str | None=..., public_key_armor: str | None=..., rsyncable_threshold: int=..., base: str | None=...) -> GtsCompileResult:
    ...

def to_json_ld(data: bytes, *, format: RdfFormat, options_json: str | None=..., context: CompiledJsonLdContext | None=..., base: str | None=...) -> str:
    ...

def serialize_jsonld(data: bytes, *, format: RdfFormat, output_format: str, options_json: str | None=..., context: CompiledJsonLdContext | None=..., yaml_schema_url: str | None=...) -> str:
    ...

def from_json_ld(text: str, *, statement_vocab: dict[str, str] | None=..., base: str | None=...) -> bytes:
    ...

def to_rdf_xml(data: bytes, *, format: RdfFormat, base: str | None=...) -> str:
    ...

def from_rdf_xml(text: str, *, base: str | None=...) -> bytes:
    ...

def feedback_bundle_native(data: bytes, *, format: RdfFormat, report_blobs: list[_BlobRow] | None=..., base: str | None=...) -> bytes:
    ...
_TermRow = tuple[int, int, str | None, int | None, str | None, int | None, tuple[int, int, int] | None]
_QuadRow = tuple[int, int, int, int | None]
_ReifierRow = tuple[int, int, int, int, int | None]
_AnnotationRow = tuple[int, int, int, int | None]
_FoldReifierRow = tuple[int, tuple[int, int, int], int | None]
_BlobExportRow = tuple[str, bytes]
_InputTermRow = tuple[int, str | None, int | None, str | None, str | None, int | None, tuple[int, int, int] | None]
GTS_ALL_SCOPE: str

class GtsRelationalRows(TypedDict):
    terms: list[_TermRow]
    directions: list[str | None]
    quads: list[_QuadRow]
    reifiers: list[_ReifierRow]
    annotations: list[_AnnotationRow]
    blobs: list[_BlobExportRow]

class GtsFoldViewNative:

    @staticmethod
    def from_bytes(data: bytes, curie_prefixes: list[tuple[str, str]]=...) -> GtsFoldViewNative:
        ...

    @staticmethod
    def from_parts(terms: list[_InputTermRow], quads: list[_QuadRow], reifiers: list[_FoldReifierRow], annotations: list[_AnnotationRow], curie_prefixes: list[tuple[str, str]]=...) -> GtsFoldViewNative:
        ...

    def term_count(self) -> int:
        ...

    def quad_count(self) -> int:
        ...

    def reifier_count(self) -> int:
        ...

    def annotation_count(self) -> int:
        ...

    def term_tuple(self, tid: int) -> _InputTermRow:
        ...

    def is_iri(self, tid: int) -> bool:
        ...

    def is_bnode(self, tid: int) -> bool:
        ...

    def is_literal(self, tid: int) -> bool:
        ...

    def iri(self, tid: int) -> str | None:
        ...

    def lex(self, tid: int) -> str:
        ...

    def lang(self, tid: int) -> str | None:
        ...

    def datatype(self, tid: int) -> str:
        ...

    def nq_token(self, tid: int) -> str:
        ...

    def python_value(self, tid: int) -> object:
        ...

    def tid_of_iri(self, iri: str) -> int | None:
        ...

    def curie(self, iri: str) -> str:
        ...

    def quads(self, scope: str | None=...) -> list[_QuadRow]:
        ...

    def subjects_by_type(self, class_iri: str, scope: str | None=...) -> list[int]:
        ...

    def objects(self, s_tid: int, p_iri: str, scope: str | None=...) -> list[int]:
        ...

    def value(self, s_tid: int, p_iri: str, scope: str | None=...) -> int | None:
        ...

    def predicate_objects(self, s_tid: int, scope: str | None=...) -> list[tuple[int, int]]:
        ...

    def has(self, s_tid: int, p_iri: str, o_tid: int, scope: str | None=...) -> bool:
        ...

    def rdf_list(self, head_tid: int, scope: str | None=...) -> list[int]:
        ...

    def reifiers(self) -> list[_FoldReifierRow]:
        ...

    def annotations(self) -> list[_AnnotationRow]:
        ...

    def tag_map(self) -> dict[str, str]:
        ...

    def available_languages(self) -> list[str]:
        ...

    def public_text(self, s_tid: int, p_iri: str, scope: str | None=...) -> str:
        ...

    def public_literal(self, s_tid: int, p_iri: str, scope: str | None=...) -> tuple[str, str | None]:
        ...

    def public_literal_with_fallback(self, s_tid: int, p_iri: str, requested: list[str], scope: str | None=...) -> tuple[str, str | None, bool]:
        ...

    def public_text_with_fallback(self, s_tid: int, p_iri: str, requested: list[str], scope: str | None=...) -> tuple[str, bool]:
        ...

    def public_texts(self, s_tid: int, p_iri: str, requested: list[str], scope: str | None=...) -> list[tuple[str, str | None, bool]]:
        ...

    def relational_rows(self) -> GtsRelationalRows:
        ...

def gts_relational_rows_from_bytes(data: bytes) -> GtsRelationalRows:
    ...

class ColumnarRows(TypedDict):
    schema: dict[str, list[tuple[str, str, bool]]]
    terms: list[tuple[int | str | bytes | None, ...]]
    quads: list[tuple[int | str | bytes | None, ...]]
    reifiers: list[tuple[int | str | bytes | None, ...]]
    annotations: list[tuple[int | str | bytes | None, ...]]
    blobs: list[tuple[int | str | bytes | None, ...]]

def gts_columnar_rows_from_bytes(data: bytes) -> ColumnarRows:
    ...

def gts_columnar_parquet_from_bytes(data: bytes) -> dict[str, bytes]:
    ...

def gts_to_sqlite(data: bytes, path: str) -> str:
    ...

def gts_to_duckdb(data: bytes, path: str) -> str:
    ...

def gts_to_parquet(data: bytes, out_dir: str) -> list[str]:
    ...

class RdfDataset:

    def __init__(self, data: bytes | str, format: RdfFormat, *, base: str | None=...) -> None:
        ...

    def quad_count(self) -> int:
        ...

    def term_count(self) -> int:
        ...

    def __len__(self) -> int:
        ...

    def to_nquads(self) -> str:
        ...

    def serialize_jsonld(self, output_format: str, *, options_json: str | None=..., context: CompiledJsonLdContext | None=..., yaml_schema_url: str | None=...) -> str:
        ...

    def to_gts(self, profile: str=...) -> bytes:
        ...

    def columnar_rows(self) -> ColumnarRows:
        ...

    def to_parquet_files(self) -> dict[str, bytes]:
        ...

class SssomDiagnostic(TypedDict):
    severity: str
    code: str
    message: str
    check: str
    instance: str | None

def validate_sssom(text: str) -> list[SssomDiagnostic]:
    ...

def sssom_to_rdf(text: str) -> str:
    ...

def sssom_roundtrip_tsv(text: str) -> str:
    ...

def sssom_default_validation_types() -> list[str]:
    ...

class ShexResultEntry(TypedDict):
    """One fixed-shape-map verdict: the input `(node, shape)` echoed verbatim."""
    node: str
    shape: str
    conformant: bool
    reason: str | None

class shex:

    @staticmethod
    def validate(schema: str, data: str, map: list[tuple[str, str]], *, schema_format: str=..., data_format: str=..., base: str | None=..., xpath_regex: str | None=...) -> list[ShexResultEntry]:
        ...

    @staticmethod
    def parse(schema: str, *, format: str=..., base: str | None=...) -> str:
        ...

class _ValidationReport:
    """A SHACL validation report."""

    @property
    def conforms(self) -> bool:
        ...

    @property
    def shapes_graph_well_formed(self) -> bool | None:
        ...

    @property
    def diagnostics(self) -> list[dict[str, str]]:
        ...

    @property
    def results(self) -> list[dict[str, builtins.object]]:
        ...

    def to_ntriples(self) -> str:
        ...

    def to_sarif(self) -> str:
        ...

class _Shapes:
    """Parsed SHACL shapes, reusable across many data graphs."""

    def __init__(self, shapes_ttl: str, *, base: str | None=None, imports: Sequence[tuple[str, str]]=..., shapes_graph: str | None=None, subclass_of_in_shapes_graph: bool=False, shacl_profile: str | None=None) -> None:
        ...

    def validate_nt(self, data_nt: str, *, xpath_regex: str | None=None) -> _ValidationReport:
        ...

    def validate_store(self, data: Store | MutableDataset, *, xpath_regex: str | None=None) -> _ValidationReport:
        ...

    def prepare(self) -> _PreparedShapes:
        ...

    def extension_usage(self, *, relation_iris: Sequence[str] | None=None, relation_namespaces: Sequence[str] | None=None, extension_namespaces: Sequence[str] | None=None) -> dict[str, Any]:
        ...

class _ShapesProductError(ValueError):
    """A refusal from the prepared-shapes-product admission boundary.

    `dimension` is one of the codec's pinned kebab-case labels, or `None` when the
    failure happened before any product existed (a shapes or data document that did
    not parse was never admitted). Branch on it, never on `str(exc)`.
    """
    dimension: str | None

class _ShapesImportError(ValueError):
    """A shapes graph's `owl:imports` closure is not in hand, or the `imports` table
    cannot be used — the one refusal every shapes-graph entry point raises, on every
    PurRDF host alike.

    `kind` is `unresolved-import` (pass the named documents in `imports`),
    `unreached-import` (a table entry neither an import nor a data-graph link names),
    `invalid-import` (a key that is not an absolute IRI, a key named twice, or a
    document that is not Turtle), `incompatible-import-versions` (the closure holds two
    versions of one series, or a graph another declares `owl:incompatibleWith`, SHACL
    1.2 Core sections 1.3 and 6.1 — `iris` are the conflicting documents),
    `unresolved-shapes-graph-link` (the data graph links a
    graph with `sh:shapesGraph`, SHACL 1.2 Core section 6.4, that nothing in hand
    resolves — pass it in `imports`) or `invalid-shapes-graph-link` (a data-graph
    `sh:shapesGraph` value that is not an IRI); `iris` are the IRIs (or values) it
    names. Branch on `kind`, never on `str(exc)`.

    A shapes graph prepared before the data graph was known (`Shapes`,
    `PreparedShapes`, a product) cannot take a linked graph in: validating a data graph
    that links one it does not hold — one that is not its base, a document its import
    closure absorbed, or a graph it declares — raises this class with kind
    `unheld-shapes-graph-link`.
    """
    kind: str
    iris: list[str]

class _PreparedShapes:
    """An immutable shape preparation, reusable across data graphs and writable as
    a prepared product."""

    def to_product(self) -> bytes:
        ...

    def provenance(self) -> str:
        ...

    def validate_nt(self, data_nt: str, *, xpath_regex: str | None=None) -> _ValidationReport:
        ...

    def validate_store_changes(self, store: Store, *, xpath_regex: str | None=None) -> _ChangeValidation:
        ...

class _ChangeValidation:
    """The outcome of `PreparedShapes.validate_store_changes`: the report, plus the
    SCOPE the report describes.

    Two facts rather than one, because a `ValidationReport` alone cannot say which
    question it answered. Read `bounded` first.
    """

    @property
    def report(self) -> _ValidationReport:
        ...

    @property
    def bounded(self) -> bool:
        ...

    @property
    def focus_nodes(self) -> int | None:
        ...

    @property
    def reason(self) -> str | None:
        ...

class _ShapesProduct:
    """A prepared product whose envelope is verified and whose self-description is
    decoded — but which has NOT been admitted."""

    @staticmethod
    def open(data: bytes) -> _ShapesProduct:
        ...

    def to_bytes(self) -> bytes:
        ...

    def explain(self) -> str:
        ...

    def format_version(self) -> int:
        ...

    def stage_id(self) -> str:
        ...

    def stage_known(self) -> bool:
        ...

    def identity_digest(self) -> str:
        ...

    def identity_components(self) -> list[tuple[str, str]]:
        ...

    def admit(self) -> _PreparedShapes:
        ...

    def admit_expecting(self, expected_identity: str) -> _PreparedShapes:
        ...

    def rebuild(self) -> _PreparedShapes:
        ...

    def rebuild_expecting(self, expected_identity: str) -> _PreparedShapes:
        ...

    def certify(self) -> None:
        ...

class shapes:
    ValidationReport: TypeAlias = _ValidationReport
    Shapes: TypeAlias = _Shapes
    PreparedShapes: TypeAlias = _PreparedShapes
    ChangeValidation: TypeAlias = _ChangeValidation
    ShapesProduct: TypeAlias = _ShapesProduct
    ShapesProductError: TypeAlias = _ShapesProductError
    ShapesImportError: TypeAlias = _ShapesImportError

    @staticmethod
    def pack_product(shapes_ttl: str, *, shapes_base: str | None=None, imports: Sequence[tuple[str, str]]=..., shapes_graph: str | None=None) -> bytes:
        ...

    @staticmethod
    def validate(shapes_ttl: str, data_nt: str, *, shapes_base: str | None=None, conformance_disallows: Sequence[str] | None=None, imports: Sequence[tuple[str, str]]=..., shapes_graph: str | None=None, subclass_of_in_shapes_graph: bool=False, xpath_regex: str | None=None, shacl_profile: str | None=None) -> dict[str, builtins.object]:
        ...

    @staticmethod
    def entail(shapes_ttl: str, data_nt: str, *, shapes_base: str | None=None, imports: Sequence[tuple[str, str]]=..., shapes_graph: str | None=None, max_term_generating_rounds: int | None=None, max_generated_terms: int | None=None, max_stored_facts: int | None=None, max_join_steps: int | None=None, xpath_regex: str | None=None, now: str | None=None, entropy_seed: int | None=None) -> dict[str, builtins.object]:
        ...

    @staticmethod
    def apply_rules(data_nt: str, shapes_ttl: str | None=None, *, srl: str | None=None, shapes_base: str | None=None, srl_base: str | None=None, explain: bool=False, max_term_generating_rounds: int | None=None, max_generated_terms: int | None=None, max_stored_facts: int | None=None, max_join_steps: int | None=None, imports: Sequence[tuple[str, str]]=..., shapes_graph: str | None=None, xpath_regex: str | None=None, now: str | None=None, entropy_seed: int | None=None) -> dict[str, builtins.object]:
        ...

    @staticmethod
    def check_rules(srl: str, *, srl_base: str | None=None, imports: Sequence[tuple[str, str]]=..., level: str | None=None) -> dict[str, builtins.object]:
        ...

    @staticmethod
    def eval_node_expr(shapes_ttl: str, data_nt: str, expr: str | None, focus: str, *, expr_at: str | None=None, expr_via: Sequence[str]=..., expr_turtle: str | None=None, scope: Mapping[str, str] | None=None, shapes_base: str | None=None, imports: Sequence[tuple[str, str]]=..., xpath_regex: str | None=None) -> dict[str, builtins.object]:
        ...

    @staticmethod
    def lint_shapes(shapes_ttl: str, *, shapes_base: str | None=None, imports: Sequence[tuple[str, str]]=..., shapes_graph: str | None=None) -> dict[str, builtins.object]:
        ...
shacl = shapes

class _Regime:
    """A SPARQL entailment regime (`purrdf.entail.Regime.OWL_RL`)."""
    SIMPLE: _Regime
    RDF: _Regime
    RDFS: _Regime
    OWL_RL: _Regime
    OWL_DIRECT: _Regime
    RIF: _Regime
    D: _Regime
type RegimeLike = _Regime | str

class entail:
    Regime: TypeAlias = _Regime

    @staticmethod
    def materialize(dataset: RdfDataset, regime: RegimeLike, program: str, imports: Sequence[tuple[str, str]], premise_iris: Sequence[str], *, max_stored_facts: int | None=None, max_join_steps: int | None=None, max_term_arena_bytes: int | None=None) -> tuple[RdfDataset, str]:
        ...

    @staticmethod
    def materialize_nt(data: str, regime: RegimeLike, program: str, imports: Sequence[tuple[str, str]], premise_iris: Sequence[str], *, max_stored_facts: int | None=None, max_join_steps: int | None=None, max_term_arena_bytes: int | None=None) -> tuple[str, str]:
        ...

    @staticmethod
    def rules(regime: RegimeLike) -> list[str]:
        ...

    @staticmethod
    def implemented_rules(regime: RegimeLike) -> list[str]:
        ...

    @staticmethod
    def extensions(regime: RegimeLike) -> list[str]:
        ...

    @staticmethod
    def consistency(data: str, imports: Sequence[tuple[str, str]], premise_iris: Sequence[str], step_cap: int=..., work_cap: int=...) -> tuple[str, str]:
        ...

    @staticmethod
    def classify(data: str, step_cap: int=..., work_cap: int=...) -> tuple[str, str]:
        ...

    @staticmethod
    def realize(data: str, step_cap: int=..., work_cap: int=...) -> tuple[str, str]:
        ...

    @staticmethod
    def instances(data: str, class_: str, step_cap: int=..., work_cap: int=...) -> tuple[str, str]:
        ...

    @staticmethod
    def entails(data: str, axiom: str, step_cap: int=..., work_cap: int=...) -> tuple[str, str]:
        ...

    @staticmethod
    def profile(data: str) -> tuple[str, str]:
        ...

    @staticmethod
    def extract_module(data: str, signature: str, method: str) -> tuple[str, str]:
        ...

    @staticmethod
    def justify(data: str, axiom: str) -> tuple[str, str]:
        ...

    @staticmethod
    def explain_conclusion(data: str, regime: RegimeLike, conclusion: str) -> tuple[str, str]:
        ...

    @staticmethod
    def prove(data: str, service: str, argument: str=..., step_cap: int=..., work_cap: int=...) -> tuple[str, str, str]:
        ...

    @staticmethod
    def check_proof(data: str, service: str, argument: str, answer: str, certificate: str, proof: str) -> str:
        ...

    @staticmethod
    def proof_services() -> list[str]:
        ...

    @staticmethod
    def certain_answers(regime: RegimeLike, data: str, pattern: str, imports: Sequence[tuple[str, str]], premise_iris: Sequence[str], *, max_stored_facts: int | None=None, max_join_steps: int | None=None, max_term_arena_bytes: int | None=None) -> tuple[str, str]:
        ...

    @staticmethod
    def graph_entails(regime: RegimeLike, premise: str, conclusion: str, imports: Sequence[tuple[str, str]], premise_iris: Sequence[str], *, max_stored_facts: int | None=None, max_join_steps: int | None=None, max_term_arena_bytes: int | None=None) -> tuple[str, str]:
        ...

    @staticmethod
    def verify_entailment(regime: RegimeLike, premise: str, conclusion: str, imports: Sequence[tuple[str, str]], premise_iris: Sequence[str], *, max_stored_facts: int | None=None, max_join_steps: int | None=None, max_term_arena_bytes: int | None=None) -> tuple[str, str]:
        ...

    class Reasoner:

        def __init__(self, data: str, step_cap: int=..., work_cap: int=..., proofs: bool=...) -> None:
            ...

        @property
        def records_proofs(self) -> bool:
            ...

        def prove(self, service: str, argument: str=...) -> tuple[str, str, str]:
            ...

        def consistency(self) -> tuple[str, str]:
            ...

        def classify(self) -> tuple[str, str]:
            ...

        def realize(self) -> tuple[str, str]:
            ...

        def instances(self, class_: str) -> tuple[str, str]:
            ...

        def entails(self, axiom: str) -> tuple[str, str]:
            ...

        def profile(self) -> tuple[str, str]:
            ...

        def extract_module(self, signature: str, method: str) -> tuple[str, str]:
            ...

        def justify(self, axiom: str) -> tuple[str, str]:
            ...

        def explain_conclusion(self, regime: RegimeLike, conclusion: str) -> tuple[str, str]:
            ...

        def __repr__(self) -> str:
            ...
_gts_from_quads = gts_from_quads
_gts_from_rdf12_bytes = gts_from_rdf12_bytes
_compile_gts_native = compile_gts_native
_compile_gts_with_report = compile_gts_with_report
_gts_ingest_report = gts_ingest_report
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
    compile_gts_with_report = _compile_gts_with_report
    gts_ingest_report = _gts_ingest_report
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
    RdfDataset: TypeAlias = _RdfDataset
    GtsFoldViewNative: TypeAlias = _GtsFoldViewNative

class _ArtifactRecord:

    @property
    def role(self) -> str:
        ...

    @property
    def logical_path(self) -> str:
        ...

    @property
    def media_type(self) -> str:
        ...

    @property
    def raw_digest(self) -> str:
        ...

    @property
    def semantic_digest(self) -> str:
        ...

    @property
    def content(self) -> builtins.bytes:
        ...

class _ManifestView:

    @property
    def identifier(self) -> str:
        ...

    @property
    def slice_iri(self) -> str:
        ...

    @property
    def label(self) -> str | None:
        ...

    @property
    def title(self) -> str | None:
        ...

    @property
    def tier(self) -> str | None:
        ...

    @property
    def creators(self) -> list[str]:
        ...

    @property
    def consumers(self) -> list[str]:
        ...

class _SliceRecord:

    @property
    def manifest(self) -> _ManifestView:
        ...

    @property
    def manifest_path(self) -> str:
        ...

    @property
    def slice_dir(self) -> str:
        ...

    @property
    def artifacts(self) -> list[_ArtifactRecord]:
        ...

class _DependencyEdge:

    @property
    def from_slice(self) -> str:
        ...

    @property
    def to_slice(self) -> str:
        ...

    @property
    def is_semantic(self) -> bool:
        ...

    @property
    def reconciliation(self) -> str:
        ...

class _ManifestPatch:

    @property
    def manifest_path(self) -> str:
        ...

    @property
    def original_text(self) -> str:
        ...

    @property
    def patched_text(self) -> str:
        ...

class _OwnershipReport:

    @property
    def edges(self) -> list[_DependencyEdge]:
        ...

    @property
    def has_ownership_defect(self) -> bool:
        ...

    @property
    def ownership_errors(self) -> list[str]:
        ...

class _SliceCatalog:

    @staticmethod
    def discover(root: str, namespace: str) -> _SliceCatalog:
        ...

    @property
    def records(self) -> list[_SliceRecord]:
        ...

    @property
    def core_slice_iris(self) -> list[str]:
        ...

    def fix_deps(self) -> list[_ManifestPatch]:
        ...

class _OwnershipAnalyzer:

    def __init__(self, catalog: _SliceCatalog) -> None:
        ...

    def analyze(self) -> _OwnershipReport:
        ...

    def analysis_graph_turtle(self) -> str:
        ...

class slice:
    ArtifactRecord: TypeAlias = _ArtifactRecord
    ManifestView: TypeAlias = _ManifestView
    SliceRecord: TypeAlias = _SliceRecord
    DependencyEdge: TypeAlias = _DependencyEdge
    ManifestPatch: TypeAlias = _ManifestPatch
    OwnershipReport: TypeAlias = _OwnershipReport
    SliceCatalog: TypeAlias = _SliceCatalog
    OwnershipAnalyzer: TypeAlias = _OwnershipAnalyzer
_Fidelity: TypeAlias = tuple[str | None, str | None]

class _TextAnalyzerSpec(TypedDict, total=False):
    lexicon: Required[TypingLiteral['empty', 'baseline', 'dictionary']]
    artifacts: list[bytes]
    entries: list[tuple[str, int]]
    max_token_scalars: int
    accent: TypingLiteral['preserve', 'latin-greek-cyrillic', 'latin-greek-cyrillic-arabic-hebrew'] | list[TypingLiteral['latin', 'greek', 'cyrillic', 'hebrew', 'arabic']]
    stemming: TypingLiteral['none', 'english']
    input_mode: TypingLiteral['plain', 'html-text', 'html-attribute']
    code_length: int
    edit_distance: int
    projection: TypingLiteral['lexical', 'han']
    substring_posting_operations: int
    substring_candidate_spans: int
    substring_verification_bytes: int
_TextProducerSpec: TypeAlias = tuple[str, str, str, _TextAnalyzerSpec] | tuple[str, str, str, _TextAnalyzerSpec, list[str] | None] | tuple[str, str, str, _TextAnalyzerSpec, list[str] | None, _Attestation] | tuple[str, str, str, _TextAnalyzerSpec, list[str] | None, _Attestation, _Fidelity]
_VectorRows: TypeAlias = list[tuple[str, list[float]]]
_VectorMetric: TypeAlias = str
_VectorGuard: TypeAlias = tuple[int, int]
_HnswProducerSpec: TypeAlias = tuple[str, _VectorRows, _VectorMetric, _VectorGuard, tuple[int, int, int, int], list[str] | None, _Attestation, str | None]
_KnnProducerSpec: TypeAlias = tuple[str, _VectorRows, _VectorMetric, _VectorGuard, list[str] | None, _Attestation, _Fidelity]

class _PlanDocumentError(ValueError):
    """A refusal from the plan-document boundary: `certify_plan`, `explain_depth`.

    `refusal` is one of the engine's pinned kebab-case names. Decode side:
    `version`, `truncated`, `trailing-bytes`, `invalid-tag`, `invalid-utf8`,
    `invalid-iri`, `non-ascending-keys`, `non-ascending-selectivity-terms`,
    `duplicate-stratum-depth`, `duplicate-stratum-derivation`,
    `duplicate-statistics-subject`, `duplicate-selectivity-term`. Certificate
    side: `depth-not-derivable`, `depth-without-derivation`,
    `derivation-without-depth`, `derivation-without-statistics-entry`,
    `statistics-entry-contradicts-derivation`, `selectivity-term-out-of-range`,
    `unconsulted-statistics-subject`. Branch on it, never on `str(exc)`.
    """
    refusal: str

class _RetrievalScore(TypedDict):
    raw: int
    scale_digits: int

class retrieval:
    PlanDocumentError: TypeAlias = _PlanDocumentError

    @staticmethod
    def plan(data: str, request: list[tuple[builtins.object, ...]], *, text_producers: dict[str, _TextProducerSpec], statistics: dict[str, builtins.object], top_k: int, hnsw_producers: dict[str, _HnswProducerSpec] | None=None, knn_producers: dict[str, _KnnProducerSpec] | None=None, data_format: str='turtle', base: str | None=None) -> dict[str, builtins.object]:
        ...

    @staticmethod
    def certify_plan(plan_bytes: bytes) -> dict[str, builtins.object]:
        ...

    @staticmethod
    def explain_depth(plan_bytes: bytes, stratum: str) -> str | None:
        ...

    @staticmethod
    def compile(data: str, request: list[tuple[builtins.object, ...]], *, text_producers: dict[str, _TextProducerSpec], statistics: dict[str, builtins.object], top_k: int, hnsw_producers: dict[str, _HnswProducerSpec] | None=None, knn_producers: dict[str, _KnnProducerSpec] | None=None, weights: dict[str, int] | None=None, k: int | None=None, decay: str | None=None, scale_digits: int | None=None, data_format: str='turtle', base: str | None=None) -> dict[str, builtins.object]:
        ...

    @staticmethod
    def search(data: str, request: list[tuple[builtins.object, ...]], *, text_producers: dict[str, _TextProducerSpec], weights: dict[str, int], statistics: dict[str, builtins.object], k: int, decay: str, scale_digits: int, top_k: int, hnsw_producers: dict[str, _HnswProducerSpec] | None=None, knn_producers: dict[str, _KnnProducerSpec] | None=None, data_format: str='turtle', base: str | None=None) -> dict[str, builtins.object]:
        ...

    @staticmethod
    def weight_for_depth(depth: int, k: int, *, decay: str, scale_digits: int) -> _RetrievalScore:
        ...

    @staticmethod
    def class_width(weight_raw: int, k: int, rank: int, *, decay: str, scale_digits: int) -> int | None:
        ...

    @staticmethod
    def deepest_rank_within_width(weight_raw: int, k: int, max_width: int, *, decay: str, scale_digits: int) -> int | None:
        ...

    @staticmethod
    def crossing_rank_at(naming_raw: list[int], at_rank: int, sharing_raw: list[int], k: int, *, decay: str, scale_digits: int) -> int | None:
        ...
```

## `geo.pyi`

<!-- python-api-stub: geo.pyi -->
```pyi
from __future__ import annotations

class GeoProfile:
    """Immutable strict native profile; omitted input selects standard CRS84."""

    def __init__(self, profile: str | None=...) -> None:
        ...

    def to_json(self) -> str:
        ...

class GeoSession:
    """One Rust session over the strict version-one geographic records."""

    def __init__(self, geo: GeoProfile | GeoSession | str | None=...) -> None:
        ...

    def call(self, request: str) -> str:
        """Return canonical semantic outcomes; raise ValueError on operational refusal."""
        ...

    def point_index(self, request: str) -> GeoPointIndex:
        ...

    def profile(self) -> GeoProfile:
        ...

class GeoPointIndex:
    """Immutable ordered point buckets, constructed by GeoSession.point_index."""

    def call(self, request: str) -> str:
        """Search through the same native comparison law and refusal channel."""
        ...
```
