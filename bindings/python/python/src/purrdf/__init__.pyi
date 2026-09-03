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

# A host-supplied PROPERTY FUNCTION: a relation invoked from predicate position,
# which — unlike a function — may emit zero, one, or many rows per call. Both
# spellings declare a positional arity, `subject_arity` values written on the
# subject side of the predicate and `object_arity` on the object side, and both
# name the relation by the IRI a query spells in predicate position.
#
# `_Relation` carries the table as Python data: `(subject_arity, object_arity,
# rows)`, where `rows` is a sequence of rows and each row is a sequence of terms
# in flattened order (subject-side values first, then object-side). Rows are
# emitted in the order given.
#
# `_RelationFromGraph` carries the table as RDF instead: `(head, subject_arity,
# object_arity)`, where `head` names an `rdf:List` of `rdf:List`s — one inner list
# per row — in the store's own DEFAULT graph. Row order is list order.
#
#     store.query(
#         "SELECT ?team WHERE { <http://example.org/ann> "
#         "<http://example.org/rel/memberOf> ?team }",
#         relations={
#             "http://example.org/rel/memberOf": (
#                 1,
#                 1,
#                 [
#                     [NamedNode("http://example.org/ann"), NamedNode("http://example.org/blue")],
#                     [NamedNode("http://example.org/bob"), NamedNode("http://example.org/red")],
#                 ],
#             )
#         },
#     )
_Relation = tuple[int, int, Sequence[Sequence[_Term]]]
_RelationFromGraph = tuple[_Term, int, int]

# `_PathRelation` is the third spelling, and the one that is not a table at all: it
# declares a TRAVERSAL over the store's own edges, and the relation binds the walk it
# finds. A call reads
#
#     ?start <iri> ( ?end ?pathId ?len ?step ?node ?edge )
#
# and emits ONE ROW PER HOP: row `i` of a `k`-hop walk binds `?len = k`, `?step = i`,
# `?node` to the node that hop arrived at, and `?edge` to the STATEMENT it traversed —
# an RDF 1.2 triple term, which joins straight back into the dataset by an ordinary
# basic graph pattern. `GROUP BY ?pathId` reassembles one walk from its hop rows and
# `ORDER BY ?step` puts them back in traversal order (`?step` and `?len` are
# `xsd:integer` literals precisely so that ordering is numeric).
#
# It crosses the boundary as pure DATA, exactly as the two table spellings do: a
# specification of which edges a hop may follow, never a Python callable the traversal
# would call back into. That is what keeps the whole evaluation GIL-free.
#
# Every field is MANDATORY and none has a default. `PathLimits` deliberately has no
# `Default`: a zero-hop path has no witness, and an unbounded traversal depth is a stack
# overflow — an abort, which escapes the engine's panic containment entirely — so a limit
# this binding invented would be one the caller never read. `min_hops == 0`, an empty
# `min_hops..max_hops` interval, a `max_hops` past the engine's hard cap, a zero guard, an
# empty or duplicated `steps`, and a non-IRI predicate all raise `ValueError` carrying the
# engine's own diagnostic. A step ALTERNATIVE the store has no edges for is not among
# them: it contributes zero edges, exactly as `p|q` does not fail when `q` matches
# nothing.
#
#     store.query(
#         "SELECT ?end ?step ?node WHERE { <http://example.org/a> "
#         "<http://example.org/pf#walk> ( ?end ?pathId ?len ?step ?node ?edge ) } "
#         "ORDER BY ?len ?step",
#         path_relations={
#             "http://example.org/pf#walk": (
#                 [(NamedNode("http://example.org/p"), "forward")],
#                 1, 4, 1024, 100000, "walk",
#             )
#         },
#     )
#
# (steps, min_hops, max_hops, max_paths_per_seed, max_expansions_per_invocation, mode)
# steps: each (predicate_term, "forward" | "inverse"); at least one, no duplicates
# mode: "walk" (every simple-prefix witness) | "shortest" (one shortest witness per pair)
_PathStep = tuple[_Term, str]
_PathRelation = tuple[Sequence[_PathStep], int, int, int, int, str]

# ── Query results ───────────────────────────────────────────────────────────────

class QuerySolution:
    def __getitem__(self, key: str | Variable | int) -> _Term | None: ...

class QuerySolutions:
    @property
    def variables(self) -> list[Variable]: ...
    def __iter__(self) -> QuerySolutions: ...
    def __next__(self) -> QuerySolution: ...
    def __len__(self) -> int: ...

# A CONSTRUCT/DESCRIBE result whose statements all land in the DEFAULT graph — every
# SPARQL 1.1 CONSTRUCT, and every DESCRIBE over default-graph data. A result carrying a
# named graph yields `QueryQuads` instead, because a `Triple` has no slot to carry the
# graph name in: a template that names a graph, or a DESCRIBE whose description is
# graph-scoped in the source (an SCBD keeps every layer — base quad, reifier declaration
# and annotation — in the graph that asserted it).
class QueryTriples:
    def __iter__(self) -> QueryTriples: ...
    def __next__(self) -> Triple: ...
    def __len__(self) -> int: ...
    # `base` is the document base the output is written under, exactly as on the
    # module-level `serialize`.
    def serialize(
        self, format: RdfFormat, *, base: str | None = ...
    ) -> bytes: ...

# A CONSTRUCT/DESCRIBE result carrying at least one NAMED graph — a quad template
# (`CONSTRUCT { GRAPH ?g { ... } }`, a first-party extension, NOT defined by SPARQL 1.2),
# or a DESCRIBE whose description is graph-scoped in the source. One result may span
# several graphs and may mix
# them with default-graph statements, so the members are `Quad`s with a live
# `graph_name`. `serialize` raises `ValueError` for a single-graph syntax
# (`RdfFormat.TURTLE` / `RdfFormat.N_TRIPLES`) rather than dropping the graphs.
class QueryQuads:
    def __iter__(self) -> QueryQuads: ...
    def __next__(self) -> Quad: ...
    def __len__(self) -> int: ...
    # Every distinct named graph the result carries, in N-Triples term syntax, sorted.
    @property
    def graph_names(self) -> list[str]: ...
    def serialize(self, format: RdfFormat) -> bytes: ...

# One serialized document plus the WHOLE realized loss of producing it, partitioned by
# CAUSE — the return of `Store.dump_with_loss` / `MutableDataset.dump_with_loss`.
#
# `dump` answers with bytes alone: a multi-graph store dumped to `RdfFormat.TURTLE`
# comes back well-formed with every graph-scoped statement missing and no signal at
# all. These counts are that signal. They partition the loss, so their sum is the total
# and no row is charged twice; reading one alone cannot distinguish "nothing was lost"
# from "the loss was charged to a cause I am not reading". Every count is REALIZED —
# what this document actually discarded — not the static pair contract
# `loss_matrix_json()` describes. The same three numbers are the C ABI's
# `purrdf_serialize` out-params and the wasm `Dataset.serializeWithLoss` getters.
class SerializeLoss:
    @property
    def bytes(self) -> bytes: ...
    @property
    def statement_rows_dropped(self) -> int: ...
    @property
    def directional_literals_dropped(self) -> int: ...
    @property
    def named_graph_rows_dropped(self) -> int: ...

class QueryBoolean:
    def __bool__(self) -> bool: ...

# ── Execution governors ─────────────────────────────────────────────────────────
#
# The governed query/update surface (bindings/python/src/py_store/query.rs). A
# tripped governor is an OUTCOME, never an exception: `query_governed` returns a
# `QueryOutcome` on both paths so the rows a budget already paid for survive, with
# the certificate that says what they bound. The one stop cause that raises is a
# `KeyboardInterrupt`, which the governed call polls for while the GIL is released.

# Which kind of governor stopped an execution. `"unknown"` is reachable only if a
# future kernel adds a governor kind this build cannot name; `label` still names it.
type GovernorKind = TypingLiteral["budget", "stopped", "refused", "unknown"]
# What a truncated execution's rows certify about the query's true answer.
type PartialCertainty = TypingLiteral["certain", "at-most", "unknown"]

class CancellationToken:
    def __init__(self) -> None: ...
    def cancel(self) -> None: ...
    @property
    def cancelled(self) -> bool: ...

class TrippedGovernor:
    @property
    def kind(self) -> GovernorKind: ...
    @property
    def label(self) -> str: ...
    @property
    def dimension(self) -> str | None: ...
    @property
    def limit(self) -> int | None: ...
    @property
    def consumed(self) -> int | None: ...
    @property
    def estimate(self) -> int | None: ...
    @property
    def cause(self) -> str | None: ...
    def __str__(self) -> str: ...

class GovernorEvidence:
    @property
    def consumed(self) -> dict[str, int]: ...
    @property
    def limits(self) -> dict[str, int]: ...
    @property
    def tripped(self) -> TrippedGovernor | None: ...
    @property
    def is_complete(self) -> bool: ...
    def consumed_in(self, dimension: str) -> int: ...
    def limit_for(self, dimension: str) -> int: ...

class PartialAnswers:
    @property
    def certainty(self) -> PartialCertainty: ...
    @property
    def is_certain(self) -> bool: ...
    @property
    def result(self) -> QuerySolutions | QueryTriples | QueryQuads | QueryBoolean | None: ...
    @property
    def is_positional_prefix(self) -> bool | None: ...
    @property
    def barrier(self) -> str | None: ...

class QueryOutcome:
    @property
    def is_complete(self) -> bool: ...
    # The COMPLETE result only; `None` when a governor tripped. The rows a trip
    # reached are on `partial`, behind the certificate that says what they bound.
    @property
    def result(self) -> QuerySolutions | QueryTriples | QueryQuads | QueryBoolean | None: ...
    @property
    def partial(self) -> PartialAnswers | None: ...
    @property
    def tripped(self) -> TrippedGovernor | None: ...
    @property
    def evidence(self) -> GovernorEvidence: ...

class EntailmentQueryOutcome:
    @property
    def phase(self) -> TypingLiteral["answered", "closure-stopped"]: ...
    @property
    def is_complete(self) -> bool: ...
    @property
    def outcome(self) -> QueryOutcome | None: ...
    @property
    def report(self) -> str | None: ...
    @property
    def tripped(self) -> TrippedGovernor | None: ...

class UpdateOutcome:
    # `False` means NOTHING applied, never "not all of it applied".
    @property
    def is_applied(self) -> bool: ...
    @property
    def tripped(self) -> TrippedGovernor | None: ...
    @property
    def evidence(self) -> GovernorEvidence: ...

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
    # by default), `property_fn_namespaces` does the same for property-function
    # PREFIX recognition, `standpoint_predicates` is the `(according_to, sharpens)`
    # predicate table the `heldIn` extension requires.
    #
    # `relations` / `relations_from_graph` / `path_relations` register host relations
    # for THIS call (see `_Relation` / `_RelationFromGraph` / `_PathRelation`). A
    # registered IRI is recognized in predicate position EXACTLY, so reaching one needs
    # no namespace declaration; declaring `property_fn_namespaces` is how a caller asks
    # for the stricter reading, in which an UNREGISTERED IRI under the namespace is a
    # hard error instead of a triple pattern that matches nothing. A duplicate IRI —
    # including one named in two of the three dicts — a ragged table, a torn `rdf:List`,
    # an empty or duplicated step alternation, and an unbuildable traversal envelope all
    # raise `ValueError`.
    #
    # All three cross the boundary as pure DATA and never as a Python callable, which is
    # what lets the whole evaluation run with the GIL released.
    #
    # SPARQL 1.2's ADJUST(value, timezone) and the VERSION prologue declaration
    # need no kwarg here: both are ordinary grammar the parser and evaluator
    # handle unconditionally, unlike the extension seams above.
    #
    # `aggregate_namespace` registers purrdf's first-party statistical aggregate set
    # (`MEDIAN`, `PERCENTILE`, `STDDEV`, `STDDEV_POP`, `VARIANCE`, `VAR_POP`, `MODE`,
    # `FIRST`, `LAST`, `TOPK` — `AggregateRegistry::register_statistical_aggregates`)
    # under that IRI, so the query text can call `AGG(<{NAMESPACE}NAME>, args…)`, e.g.:
    #
    #   store.query(
    #       "PREFIX ex: <https://ex.example/> "
    #       "SELECT (AGG(<https://ex.example/agg#MEDIAN>, ?v) AS ?m) "
    #       "WHERE { ?s ex:value ?v }",
    #       aggregate_namespace="https://ex.example/agg#",
    #   )
    #
    # Unset (the default) leaves every one of the ten names an ordinary unregistered
    # custom-aggregate IRI, refused at prepare time exactly as any other unregistered
    # `AGG(<iri>, …)` call. `AggregateRegistry::register_statistical_aggregates` takes
    # only a namespace string — no host Rust closure to marshal — which is what makes
    # this kwarg possible: it crosses the Python boundary exactly the way
    # `property_fn_namespaces` does. The GENERAL custom-aggregate seam
    # (`purrdf_sparql_eval::agg_fn::AggregateRegistry::register`, an arbitrary
    # `init`/`step`/`combine`/`finish` closure) remains Rust-host-only — a fold has no
    # data-only reduction the way a property-function relation does — and this binding
    # exposes no surface for it, not even a namespace-only one.
    def query(
        self,
        query: str,
        *,
        substitutions: dict[Variable, _Term] | None = ...,
        extension_namespaces: list[str] | None = ...,
        property_fn_namespaces: list[str] | None = ...,
        standpoint_predicates: tuple[str, str] | None = ...,
        relations: dict[str, _Relation] | None = ...,
        relations_from_graph: dict[str, _RelationFromGraph] | None = ...,
        path_relations: dict[str, _PathRelation] | None = ...,
        aggregate_namespace: str | None = ...,
    ) -> QuerySolutions | QueryTriples | QueryQuads | QueryBoolean: ...
    # Governed sibling of `query`: every ceiling is inclusive; an omitted dimension
    # remains metered at an effectively unreachable ceiling. `deadline_ms` is a
    # wall-clock budget in milliseconds. A trip is returned in the `QueryOutcome`,
    # never raised.
    def query_governed(
        self,
        query: str,
        *,
        substitutions: dict[Variable, _Term] | None = ...,
        extension_namespaces: list[str] | None = ...,
        property_fn_namespaces: list[str] | None = ...,
        standpoint_predicates: tuple[str, str] | None = ...,
        relations: dict[str, _Relation] | None = ...,
        relations_from_graph: dict[str, _RelationFromGraph] | None = ...,
        path_relations: dict[str, _PathRelation] | None = ...,
        aggregate_namespace: str | None = ...,
        fuel: int | None = ...,
        deadline_ms: int | None = ...,
        max_answers: int | None = ...,
        max_intermediate_cells: int | None = ...,
        max_scratch_bytes: int | None = ...,
        max_remote_requests: int | None = ...,
        cancel: CancellationToken | None = ...,
    ) -> QueryOutcome: ...
    # Governed two-phase entailment query. `outcome` and `report` are absent only
    # when the closure phase itself was stopped. `aggregate_namespace` behaves
    # exactly as on `query_governed` above. `property_fn_namespaces` / `relations` /
    # `relations_from_graph` / `path_relations` behave exactly as on `query_governed`,
    # too: a registered relation is reachable from the closure query exactly as it is
    # from an ordinary one, so registering one here and omitting it there cannot silently
    # change which rows the SAME predicate position yields. `relations_from_graph` reads
    # its table — and `path_relations` snapshots its edges — from the CLOSURE the regime
    # materializes, which is the dataset the query is answered over; a regime that DERIVES
    # a quad under a step's predicate therefore widens the walk exactly as it widens a
    # `p+` in the same query. The one refused pairing is `entailment="owl-direct"` on an
    # ontology whose restricted chase mints existential witnesses: a walk over that
    # closure could return a minted blank node, so it raises `ValueError` carrying the
    # code `reasoning-closure-relation-witness` rather than answering.
    def query_entailment_governed(
        self,
        query: str,
        entailment: str,
        *,
        program: str = ...,
        substitutions: dict[Variable, _Term] | None = ...,
        extension_namespaces: list[str] | None = ...,
        property_fn_namespaces: list[str] | None = ...,
        standpoint_predicates: tuple[str, str] | None = ...,
        relations: dict[str, _Relation] | None = ...,
        relations_from_graph: dict[str, _RelationFromGraph] | None = ...,
        path_relations: dict[str, _PathRelation] | None = ...,
        aggregate_namespace: str | None = ...,
        fuel: int | None = ...,
        deadline_ms: int | None = ...,
        max_answers: int | None = ...,
        max_intermediate_cells: int | None = ...,
        max_scratch_bytes: int | None = ...,
        max_remote_requests: int | None = ...,
        cancel: CancellationToken | None = ...,
    ) -> EntailmentQueryOutcome: ...
    # `aggregate_namespace` behaves exactly as on `query` above, and is reachable
    # from a `DELETE`/`INSERT … WHERE` clause through a nested `SELECT … GROUP BY` —
    # the only place SPARQL UPDATE's grammar admits an aggregate.
    def update(
        self,
        update: str,
        *,
        extension_namespaces: list[str] | None = ...,
        property_fn_namespaces: list[str] | None = ...,
        standpoint_predicates: tuple[str, str] | None = ...,
        relations: dict[str, _Relation] | None = ...,
        relations_from_graph: dict[str, _RelationFromGraph] | None = ...,
        path_relations: dict[str, _PathRelation] | None = ...,
        aggregate_namespace: str | None = ...,
    ) -> None: ...
    # Governed sibling of `update`. No `max_answers`: it bounds an answer sequence
    # an UPDATE does not have.
    def update_governed(
        self,
        update: str,
        *,
        extension_namespaces: list[str] | None = ...,
        property_fn_namespaces: list[str] | None = ...,
        standpoint_predicates: tuple[str, str] | None = ...,
        relations: dict[str, _Relation] | None = ...,
        relations_from_graph: dict[str, _RelationFromGraph] | None = ...,
        path_relations: dict[str, _PathRelation] | None = ...,
        aggregate_namespace: str | None = ...,
        fuel: int | None = ...,
        deadline_ms: int | None = ...,
        max_intermediate_cells: int | None = ...,
        max_scratch_bytes: int | None = ...,
        max_remote_requests: int | None = ...,
        cancel: CancellationToken | None = ...,
    ) -> UpdateOutcome: ...
    # `base` is the document base the dump is WRITTEN under — the egress mirror of
    # `load(base=...)`, which this surface previously lacked. A syntax that can express
    # a base writes it and relativizes against it; one that cannot emits absolute IRIs.
    # A base that is not an absolute IRI raises whatever the format. It composes with
    # `from_graph`: a base and a non-default graph selection apply together, and the
    # RDF 1.2 statement layer is emitted rather than projected away, so a dump does not
    # silently thin the store on the way out.
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
        base: str | None = ...,
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
        base: str | None = ...,
    ) -> bytes: ...
    # The counting twin of `dump`: same bytes, plus the realized loss of producing
    # them. No `from_graph` and no JSON-LD configuration — a graph selection would make
    # the named-graph count meaningless, and the JSON-LD family loses nothing.
    def dump_with_loss(self, format: RdfFormat) -> SerializeLoss: ...
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
    # `base` is the document base the dump is WRITTEN under — the egress mirror of
    # `load(base=...)`, honored exactly as on `Store.dump`, including alongside a
    # `from_graph` selection. The RDF 1.2 statement layer is emitted, not projected.
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
        base: str | None = ...,
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
        base: str | None = ...,
    ) -> bytes: ...
    # The counting twin of `dump`; see `Store.dump_with_loss`.
    def dump_with_loss(self, format: RdfFormat) -> SerializeLoss: ...
    # Engine configuration kwargs: as on `Store.query` / `Store.update`, including
    # `aggregate_namespace` (see `Store.query`).
    def query(
        self,
        query: str,
        *,
        substitutions: dict[Variable, _Term] | None = ...,
        extension_namespaces: list[str] | None = ...,
        property_fn_namespaces: list[str] | None = ...,
        standpoint_predicates: tuple[str, str] | None = ...,
        relations: dict[str, _Relation] | None = ...,
        relations_from_graph: dict[str, _RelationFromGraph] | None = ...,
        path_relations: dict[str, _PathRelation] | None = ...,
        aggregate_namespace: str | None = ...,
    ) -> QuerySolutions | QueryTriples | QueryQuads | QueryBoolean: ...
    # Governed siblings: keywords, outcome, and Ctrl-C interaction exactly as on
    # `Store.query_governed` / `Store.update_governed`.
    def query_governed(
        self,
        query: str,
        *,
        substitutions: dict[Variable, _Term] | None = ...,
        extension_namespaces: list[str] | None = ...,
        property_fn_namespaces: list[str] | None = ...,
        standpoint_predicates: tuple[str, str] | None = ...,
        relations: dict[str, _Relation] | None = ...,
        relations_from_graph: dict[str, _RelationFromGraph] | None = ...,
        path_relations: dict[str, _PathRelation] | None = ...,
        aggregate_namespace: str | None = ...,
        fuel: int | None = ...,
        deadline_ms: int | None = ...,
        max_answers: int | None = ...,
        max_intermediate_cells: int | None = ...,
        max_scratch_bytes: int | None = ...,
        max_remote_requests: int | None = ...,
        cancel: CancellationToken | None = ...,
    ) -> QueryOutcome: ...
    # `property_fn_namespaces` / `relations` / `relations_from_graph` / `path_relations`
    # behave exactly as on `query_governed` above: a registered relation is reachable
    # from the closure query exactly as it is from an ordinary one.
    # `relations_from_graph` reads its table — and `path_relations` snapshots its edges —
    # from the CLOSURE the regime materializes, exactly as `Store.query_entailment_governed`
    # does, including its one refused `owl-direct` pairing.
    def query_entailment_governed(
        self,
        query: str,
        entailment: str,
        *,
        program: str = ...,
        substitutions: dict[Variable, _Term] | None = ...,
        extension_namespaces: list[str] | None = ...,
        property_fn_namespaces: list[str] | None = ...,
        standpoint_predicates: tuple[str, str] | None = ...,
        relations: dict[str, _Relation] | None = ...,
        relations_from_graph: dict[str, _RelationFromGraph] | None = ...,
        path_relations: dict[str, _PathRelation] | None = ...,
        aggregate_namespace: str | None = ...,
        fuel: int | None = ...,
        deadline_ms: int | None = ...,
        max_answers: int | None = ...,
        max_intermediate_cells: int | None = ...,
        max_scratch_bytes: int | None = ...,
        max_remote_requests: int | None = ...,
        cancel: CancellationToken | None = ...,
    ) -> EntailmentQueryOutcome: ...
    def update(
        self,
        update: str,
        *,
        extension_namespaces: list[str] | None = ...,
        property_fn_namespaces: list[str] | None = ...,
        standpoint_predicates: tuple[str, str] | None = ...,
        relations: dict[str, _Relation] | None = ...,
        relations_from_graph: dict[str, _RelationFromGraph] | None = ...,
        path_relations: dict[str, _PathRelation] | None = ...,
        aggregate_namespace: str | None = ...,
    ) -> None: ...
    def update_governed(
        self,
        update: str,
        *,
        extension_namespaces: list[str] | None = ...,
        property_fn_namespaces: list[str] | None = ...,
        standpoint_predicates: tuple[str, str] | None = ...,
        relations: dict[str, _Relation] | None = ...,
        relations_from_graph: dict[str, _RelationFromGraph] | None = ...,
        path_relations: dict[str, _PathRelation] | None = ...,
        aggregate_namespace: str | None = ...,
        fuel: int | None = ...,
        deadline_ms: int | None = ...,
        max_intermediate_cells: int | None = ...,
        max_scratch_bytes: int | None = ...,
        max_remote_requests: int | None = ...,
        cancel: CancellationToken | None = ...,
    ) -> UpdateOutcome: ...
    def compact(self) -> None: ...
    def __len__(self) -> int: ...

class Dataset:
    def __init__(self, quads: object | None = ...) -> None: ...
    def add(self, quad: Quad) -> None: ...
    def canonicalize(self, algorithm: CanonicalizationAlgorithm) -> None: ...
    def __iter__(self) -> QuadIter: ...
    def __len__(self) -> int: ...

# ── Module functions ────────────────────────────────────────────────────────────

# `base` is the document base relative IRI references resolve against on the parse
# leg, and the base the output is written under on the serialize leg — the same
# parameter `Store.load` carries and the same one the WebAssembly and C surfaces
# take. Omitting it means "no base in scope": PurRDF has no retrieval IRI to derive
# one from and fabricates none, so a relative reference then raises `ValueError`
# carrying the shared `iri-relative-no-base` code. A document's own base (Turtle
# `@base`, `xml:base`, JSON-LD `@context.@base`) wins over the supplied one. On the
# serialize leg a format that cannot express a base (N-Triples, N-Quads, TriX,
# HexTuples) emits absolute IRIs rather than raising; a base that is not an absolute
# IRI raises on either leg.
def parse(
    input: bytes | str, format: RdfFormat, *, base: str | None = ...
) -> list[Quad]: ...
@overload
def serialize(
    input: QueryTriples | QueryQuads,
    output: IO[bytes],
    format: RdfFormat,
    *,
    base: str | None = ...,
) -> None: ...
@overload
def serialize(
    input: QueryTriples | QueryQuads,
    output: None = ...,
    *,
    format: RdfFormat,
    base: str | None = ...,
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

#: `(prefix, iri)` anchoring the additive `purrdf` provenance extension. PurRDF
#: mints no vocabulary IRIs of its own — there is no default namespace.
_ProvenanceNamespace = tuple[str, str]

def serialize_sparql_solutions(
    format: str,
    variables: list[str],
    rows: list[_ResultRow],
    *,
    provenance_namespace: _ProvenanceNamespace | None = ...,
    query_hash: str | None = ...,
) -> bytes: ...
def serialize_sparql_boolean(
    format: str,
    value: bool,
    *,
    provenance_namespace: _ProvenanceNamespace | None = ...,
    query_hash: str | None = ...,
) -> bytes: ...

# A parsed SELECT is `("SELECT", variables, rows)`; a parsed ASK is `("ASK", bool)`
# — a heterogeneous tuple discriminated by its first element.
def parse_sparql_results(format: str, data: bytes) -> tuple[Any, ...]: ...

#: Decoded provenance: `{"query_hash": str | None, "engine": str | None}`.
_ProvenanceDict = dict[str, str | None]

# The inverse of `serialize_sparql_solutions`'s/`serialize_sparql_boolean`'s
# `provenance_namespace`: a document with no member under `prefix` decodes to
# both fields `None` rather than raising.
def provenance_from_json(data: bytes, prefix: str, iri: str) -> _ProvenanceDict: ...
def provenance_from_xml(data: bytes, prefix: str, iri: str) -> _ProvenanceDict: ...

# ── RDF → GTS producer (bindings/python/src/py_gts.rs) ──────────────────────────

#: A `(data, media_type, rep)` content-addressed blob row.
_BlobRow = tuple[bytes, str, str]
#: A `(slice_iri, slice_name, role, logical_path, content)` row.
_SliceArtifactRow = tuple[str, str, str, str, bytes]
#: A `(data, format, graph_name, scope)` named-graph ingest row.
_NamedGraphRow = tuple[bytes, RdfFormat, str | None, str | None]

# Every producer entry below takes the same optional `base`: the document base the
# source bytes' relative IRI references resolve against. Absent means "no base in
# scope" — never a fabricated one — so a relative reference raises `ValueError`
# carrying `iri-relative-no-base`, and an in-document base still wins.
def gts_from_quads(
    data: bytes,
    *,
    format: RdfFormat,
    profile: str = ...,
    transform: list[str] | None = ...,
    base: str | None = ...,
) -> bytes: ...
def gts_from_rdf12_bytes(
    data: bytes,
    *,
    format: RdfFormat,
    profile: str = ...,
    transform: list[str] | None = ...,
    base: str | None = ...,
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
    base: str | None = ...,
) -> bytes: ...
def snapshot_content_id_native(
    data: bytes, *, format: RdfFormat, base: str | None = ...
) -> str: ...

# ── Text-format codecs via purrdf-gts (JSON-LD-star + RDF/XML) ─────────────────
# RDF bytes ↔ JSON-LD-star / RDF/XML through the purrdf-gts codec set. The compat
# `Graph.serialize`/`parse` route these formats here; serialize takes RDF bytes in
# `format` and returns the text form, parse takes the text and returns N-Quads bytes.
# `base` here is BOTH legs: relative references in `data` resolve against it, and
# JSON-LD (whose grammar can express a base) carries it into the emitted context as
# `@base` with document-position `@id`s compacted against it. A base the caller's own
# context already declares wins.
def to_json_ld(
    data: bytes,
    *,
    format: RdfFormat,
    options_json: str | None = ...,
    context: CompiledJsonLdContext | None = ...,
    base: str | None = ...,
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
    text: str,
    *,
    statement_vocab: dict[str, str] | None = ...,
    base: str | None = ...,
) -> bytes: ...

# `to_rdf_xml`'s `base` applies to BOTH legs: relative references in `data` resolve
# against it, and the emitted RDF/XML declares it as `xml:base` with its `rdf:about` /
# `rdf:resource` references spelled against it. The RDF 1.2 statement layer is still
# emitted (RDF/XML renders a reifier binding as `rdf:parseType="Triple"`), so the base
# is not bought at the cost of reifier and annotation rows.
def to_rdf_xml(
    data: bytes, *, format: RdfFormat, base: str | None = ...
) -> str: ...
def from_rdf_xml(text: str, *, base: str | None = ...) -> bytes: ...
def feedback_bundle_native(
    data: bytes,
    *,
    format: RdfFormat,
    report_blobs: list[_BlobRow] | None = ...,
    base: str | None = ...,
) -> bytes: ...

# ── GTS fold view and relational exports (bindings/python/src/py_gts_view.rs) ───

_TermRow = tuple[
    int, int, str | None, int | None, str | None, int | None, tuple[int, int, int] | None
]
_QuadRow = tuple[int, int, int, int | None]
_ReifierRow = tuple[int, int, int, int, int | None]
_AnnotationRow = tuple[int, int, int, int | None]
_FoldReifierRow = tuple[int, tuple[int, int, int], int | None]
_BlobExportRow = tuple[str, bytes]
_InputTermRow = tuple[
    int, str | None, int | None, str | None, str | None, int | None, tuple[int, int, int] | None
]

class GtsRelationalRows(TypedDict):
    terms: list[_TermRow]
    quads: list[_QuadRow]
    reifiers: list[_ReifierRow]
    annotations: list[_AnnotationRow]
    blobs: list[_BlobExportRow]

class GtsFoldViewNative:
    # Both constructors raise ValueError carrying `gts-self-reaching-term` when the
    # term table lets a term resolve through itself. The view's accessors walk a
    # quoted triple's resolved components to the leaves, so such a term would recurse
    # without bound and abort the process; the view refuses to EXIST rather than hand
    # back an object whose every renderer is a process kill. `from_bytes` cannot
    # normally hit it (the GTS reader refuses the row that closes the loop), but a
    # term table assembled by the caller and handed to `from_parts` can.
    @staticmethod
    def from_bytes(data: bytes) -> GtsFoldViewNative: ...
    @staticmethod
    def from_parts(
        terms: list[_InputTermRow],
        quads: list[_QuadRow],
        reifiers: list[_FoldReifierRow],
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
    def reifiers(self) -> list[_FoldReifierRow]: ...
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

# Declared but NOT implemented: each of the three below raises `ValueError`
# unconditionally and writes nothing (bindings/python/src/py_gts_view.rs).
# `gts_relational_rows_from_bytes` is the working relational surface.
def gts_to_sqlite(data: bytes, path: str) -> str: ...
def gts_to_duckdb(data: bytes, path: str) -> str: ...
def gts_to_parquet(data: bytes, out_dir: str) -> list[str]: ...

# A Python handle to a frozen, immutable RDF 1.2 dataset.
class RdfDataset:
    # `base` is the document base relative IRI references resolve against, exactly
    # as on the module-level `parse` and `Store.load`.
    def __init__(
        self, data: bytes | str, format: RdfFormat, *, base: str | None = ...
    ) -> None: ...
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

    # `base` is the shapes document's own base IRI, resolving its relative IRI
    # references. Omitted, only an in-document `@base` can establish one and a
    # relative reference raises ValueError rather than being silently unresolved.
    def __init__(self, shapes_ttl: str, *, base: str | None = None) -> None: ...
    def validate_nt(self, data_nt: str) -> _ValidationReport: ...
    def validate_store(self, data: Store | MutableDataset) -> _ValidationReport: ...

class shapes:
    ValidationReport = _ValidationReport
    Shapes = _Shapes
    # Validate a data graph (N-Triples) against a shapes graph (Turtle).
    #
    # `shapes_base` is the base IRI the SHAPES document's relative IRI references
    # resolve against; `data_nt` needs no counterpart because N-Triples admits no
    # relative IRI by grammar.
    @staticmethod
    def validate(
        shapes_ttl: str, data_nt: str, *, shapes_base: str | None = None
    ) -> dict[str, builtins.object]: ...
    # Entail a data graph (N-Triples) under a shapes graph (Turtle): apply every
    # SHACL-AF sh:rule to a fixpoint, returning the materialized dataset (base
    # graph plus every inferred triple) as a canonical N-Triples string.
    @staticmethod
    def entail(
        shapes_ttl: str, data_nt: str, *, shapes_base: str | None = None
    ) -> str: ...

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
    #
    # `work_cap` narrows the per-decision WORK cap on the same rules. It bounds
    # what `step_cap` structurally cannot: a round is a PASS over the completion
    # graph rather than a unit of cost, so an ontology can make each round
    # enormously more expensive without making the search take more rounds. A run
    # that reaches it answers `unknown` with `work` equal to `work-budget` in its
    # certificate.
    #
    # The certificate's search-cost counters, one line each:
    #   * `steps` — rounds spent, against the per-decision round cap.
    #   * `budget` — the round cap the decision ran under (the knowledge base's own
    #     derived cap, or `step_cap` if that narrowed it).
    #   * `work` — matcher, scan, closure and clone work spent, against the work cap.
    #   * `work-budget` — the work cap the decision ran under (derived, or
    #     `work_cap` if that narrowed it).
    #   * `decisions` — how many sub-decisions the run made.
    #   * `peak-nodes` — the largest completion graph a decision built.
    #   * `disjunctions` — how many times the tableau's case-split rule fired.
    #   * `peak-depth` — how deep that rule's branch stack got.

    # Does the knowledge base have a model at all? The answer is one line,
    # `consistency true|false|unknown`. The only DL service that answers for an
    # unsatisfiable ontology, because it is the one that detects one.
    @staticmethod
    def consistency(
        data: str, step_cap: int = ..., work_cap: int = ...
    ) -> tuple[str, str]: ...
    # The entailed subsumption hierarchy over the named classes: `equivalent`,
    # `subclass` (the full transitive closure), `direct` (its reduction) and
    # `unsatisfiable` lines, in that block order. Raises ValueError for an
    # ontology with no model, where every class subsumes every other.
    @staticmethod
    def classify(
        data: str, step_cap: int = ..., work_cap: int = ...
    ) -> tuple[str, str]: ...
    # The entailed types of the named individuals (`type` lines) and the most
    # specific of them (`direct-type` lines).
    @staticmethod
    def realize(
        data: str, step_cap: int = ..., work_cap: int = ...
    ) -> tuple[str, str]: ...
    # The named individuals entailed to be instances of `class_`, as
    # `instance <term>` lines. `class_` is ONE N-Triples term, angle brackets
    # included. A class the ontology never mentions yields an empty answer, which
    # is a real answer rather than an error.
    @staticmethod
    def instances(
        data: str, class_: str, step_cap: int = ..., work_cap: int = ...
    ) -> tuple[str, str]: ...
    # Does the ontology entail `axiom`? `axiom` is ONE triple of the OWL 2 RDF
    # mapping: rdfs:subClassOf, owl:equivalentClass, owl:disjointWith, rdf:type,
    # owl:sameAs, owl:differentFrom and rdfs:subPropertyOf select the seven named
    # axiom kinds, and any other predicate is an object-property assertion. The
    # answer is `entails true|false|unknown` followed by the axiom AS READ.
    @staticmethod
    def entails(
        data: str, axiom: str, step_cap: int = ..., work_cap: int = ...
    ) -> tuple[str, str]: ...
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

    # ── Proof terms: opt-in to produce, and a checker to consume ─────────────
    # Everything above records NOTHING and returns a two-tuple. `prove` is the
    # opt-in: it records the tableau runs a service made — which costs the
    # completion graph of each one — and returns a THREE-tuple whose third
    # element is a `purrdf-dl-proof 1` document. `answer` and `certificate` are
    # byte-identical to the same question asked without a proof: recording is an
    # observation the reasoner makes of itself, never a lever it reads.
    #
    # `argument` is the question's own input in that service's grammar: "" for
    # `consistency`/`classify`/`realize` (a non-empty one raises rather than
    # being discarded), ONE N-Triples term for `class-satisfiability`/`instances`,
    # ONE triple for `entails`, and a `method <bot|top|star>` line followed by one
    # term per line for `extract-module`.
    @staticmethod
    def prove(
        data: str,
        service: str,
        argument: str = ...,
        step_cap: int = ...,
        work_cap: int = ...,
    ) -> tuple[str, str, str]: ...
    # CHECK a proof against the CALLER's own ontology, question and answer.
    # Nothing in it trusts the producer: the ontology is parsed from `data`, the
    # question is re-derived from `service` and `argument`, the claims are read
    # back out of `answer`'s own grammar, and the checking context comes from a
    # reverse mapping this call performs itself. Returns the
    # `purrdf-dl-proof-check 1` report. `answer` and `certificate` may each be ""
    # for a weaker check that SAYS so. Raises ValueError for a proof document
    # reading `availability not-recorded` — an answer nobody asked to record is
    # never presented as a verified one — and for every other rejection.
    @staticmethod
    def check_proof(
        data: str,
        service: str,
        argument: str,
        answer: str,
        certificate: str,
        proof: str,
    ) -> str: ...
    # The seven services `prove` and `check_proof` accept, so a caller can
    # MEASURE the set rather than trust a docstring.
    @staticmethod
    def proof_services() -> list[str]: ...

    # ── Conclusion-directed entailment (the CHASE lane, not the tableau) ─────
    # The CERTAIN ANSWERS of a basic graph pattern: the substitutions the
    # knowledge base ENTAILS the pattern under — true in every model, not merely
    # present in one closure, which is what SPARQL's entailment regimes define
    # the answers to a basic graph pattern to be. `pattern` is N-Triples with
    # `?name` in any position, the PREDICATE included; a blank node in it is a
    # NON-DISTINGUISHED variable, constrained by the match and not projected. A
    # variable inside an RDF 1.2 triple term is an ordinary variable — it binds
    # and it is a column — and one NAME is one VARIABLE wherever it was written,
    # so a pattern using it above and below the triple-term boundary is joined. A
    # predicate variable is projected like any other, and under OWL_RL it also
    # renders a `limit`: it ranges over the whole predicate vocabulary, including
    # the schema predicates and the constructs the mechanisms beyond the rule
    # table decide, and the closure holds neither. The answer opens
    # `mechanism <name>`, then `var` and `row` lines, then a `limit` line
    # per reason the row set may not be EXHAUSTIVE — no `limit` lines is the
    # claim that it is. A pattern with a projected variable is `strict-table`,
    # and a lane that would have been needed for it names itself in a `limit`;
    # a pattern with NO projected variable is a conclusion graph, is answered by
    # the same fold `graph_entails` runs, and names whichever of the seven
    # reached it. Raises ValueError for OWL_DIRECT and RIF, each defined by
    # an input this signature does not carry, and for a variable in a literal's
    # DATATYPE — a slot that holds an IRI rather than a term to bind, so
    # `"5"^^?d` is refused by name rather than answered.
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
    #
    # `step_cap` and `work_cap` are the same tighten-only round/work narrowings
    # the module-level functions above take, fixed once at construction and then
    # applied to EVERY decision the session goes on to make — not re-askable per
    # call — so every question asked through one `Reasoner` runs under the same
    # pair of caps.
    class Reasoner:
        def __init__(
            self,
            data: str,
            step_cap: int = ...,
            work_cap: int = ...,
            proofs: bool = ...,
        ) -> None: ...
        # Whether this session records proof terms. False unless the session was
        # constructed with `proofs=True`, which is the whole opt-in: a session
        # nobody asked to record keeps no traces and costs what it always cost.
        @property
        def records_proofs(self) -> bool: ...
        # Answer `service` about `argument`, with its proof — see
        # `purrdf.entail.prove`. Returns (answer, certificate, proof). Raises
        # ValueError on a session that records nothing.
        def prove(self, service: str, argument: str = ...) -> tuple[str, str, str]: ...
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
