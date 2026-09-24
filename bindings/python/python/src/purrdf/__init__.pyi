# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
from types import CapsuleType
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
# Any of the three spellings may carry ONE extra trailing position: what the host
# knows about the index the rows came from, which is the one part of a relation the
# rows themselves cannot express. A table read out of a search index mid-rebuild is
# the same tuple of rows as a table read out of a whole one, and no query text,
# dataset snapshot or registry fingerprint differs between the two runs — so if the
# host does not say, nothing can.
#
# `(generation, incompleteness)`, each `str` or `None`, both recorded verbatim and
# never parsed:
#
# * `generation` is the host's own name for the index version that produced the
#   rows. `None` is SILENCE — it is never a claim that the index was current.
# * `incompleteness` is the host's own reason the index was NOT whole ("shard 3 of 4
#   is still rebuilding"). `None` says nothing, which is likewise never a
#   certificate of wholeness: there is no seam at which wholeness can be certified,
#   so neither this binding nor the engine mints such a claim.
#
# An attested incompleteness is WITNESSED OR FATAL, decided by the entry point's own
# return type rather than by any keyword. `query_governed` and
# `query_entailment_governed` report it on `QueryOutcome.relation_witness` beside
# their rows; `query` and `update` have nowhere to put it, so they raise
# `ValueError` carrying `native-sparql-relation-incomplete` rather than hand back a
# short answer that is indistinguishable from a complete one.
#
#     outcome = store.query_governed(
#         "SELECT ?team WHERE { <http://example.org/ann> "
#         "<http://example.org/rel/memberOf> ?team }",
#         relations={
#             "http://example.org/rel/memberOf": (
#                 1, 1, rows, ("members-index-7", "shard 3 of 4 is still rebuilding"),
#             )
#         },
#     )
#     outcome.relation_witness["http://example.org/rel/memberOf"]["incompleteness"]
_Attestation = tuple[str | None, str | None]
_Relation = (
    tuple[int, int, Sequence[Sequence[_Term]]]
    | tuple[int, int, Sequence[Sequence[_Term]], _Attestation]
)
_RelationFromGraph = tuple[_Term, int, int] | tuple[_Term, int, int, _Attestation]

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
_PathRelation = (
    tuple[Sequence[_PathStep], int, int, int, int, str]
    | tuple[Sequence[_PathStep], int, int, int, int, str, _Attestation]
)

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

# What ONE relation attested across every invocation it served in one governed
# execution. Three facts, none derivable from the others: how hard the query leaned on
# the relation, which index versions answered, and whether any of them admitted to
# being short.
class RelationAttestations(TypedDict):
    #: Invocations of this relation that entered host code — the same executions the
    #: `property-function-invocation` charge point prices, so the receipt and the meter
    #: describe the same run.
    #:
    #: A fact about the SCHEDULE, not about the index, and therefore NOT comparable
    #: across runs — unlike the two declaration lists beside it. Under a `FILTER EXISTS`
    #: the engine evaluates each chunk of driving rows on a worker whose `EXISTS` memo
    #: starts cold, so the relation inside it is re-entered once per chunk and the chunk
    #: count comes from the runtime's thread count; the same query over the same data can
    #: report a different number while every declaration beside it is identical, including
    #: between two runs that differ only in the budget they were given. Read it as "did
    #: this relation run at all" (`0` versus non-zero) or as a rough magnitude for a log
    #: line, never as a value to compare between two receipts.
    invocations: int
    #: Every DISTINCT index version this relation declared, sorted and de-duplicated.
    #: `None` is a member like any other and means those invocations declared NOTHING:
    #: an absent generation is silence, and silence is NOT a claim that the index was
    #: whole or current.
    #:
    #: A list rather than a single value because the engine's record is a set: a
    #: long-running query CAN straddle a rebuild, and a relation that pinned one version
    #: for some invocations and another for the rest is the one thing that would
    #: otherwise be invisible. A relation registered from Python declares one
    #: attestation for the whole call, so through this surface the list holds exactly one
    #: entry per relation that ran.
    generations: list[str | None]
    #: The producer's OWN reasons, verbatim, for invocations that declared the index
    #: not whole — sorted and de-duplicated. Empty means nobody declared an
    #: incompleteness, which again is never a certificate of wholeness. Verbatim rather
    #: than a boolean because "shard 3 of 4 is still rebuilding" tells an operator what
    #: to do and `True` does not.
    incompleteness: list[str]

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
    # What each relation this execution INVOKED attested about the index behind it,
    # keyed by the IRI it was registered under and ordered by that IRI on every
    # machine and every run.
    #
    # ALWAYS PRESENT, possibly empty — never `None` and never absent. An empty mapping
    # is the true statement that no relation attested anything, usually because the
    # query invoked none; it is emphatically NOT a claim that an index was whole. A
    # relation that ran and declared nothing is listed, with its invocation count and a
    # single `None` generation, because "it ran and said nothing" and "it never ran" are
    # different facts.
    #
    # This is why the governed lane can answer where the ungoverned one raises: rows
    # whose receipt names the relation that was short, and quotes its reason, are
    # labelled rather than silently short.
    @property
    def relation_witness(self) -> dict[str, RelationAttestations]: ...

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

# A SPARQL query parsed and admitted once, run many times with different bindings.
# Built by `Store.prepare`. What is prepared is the PLAN, not the data: the object
# holds a reference to the store it was prepared against and re-reads its CURRENT
# contents on every `run`, so a mutation made after `prepare` — including one made
# between two `run` calls on the same handle — is visible to the next run. That is
# what a rule fixpoint or an incremental SHACL revalidation needs, since both mutate
# their store every round.
#
# Not thread-safe, and deliberately so: a run borrows this object uniquely, because a
# query body can re-enter the evaluator and a handle reachable a second time while a
# run is in flight is a handle two evaluations could disagree about.
class PreparedQuery:
    # The declared parameter names, in declaration order.
    @property
    def parameters(self) -> list[str]: ...
    # Bind every declared parameter and run, returning the results exactly as
    # `Store.query` does. Each keyword names a declared parameter; an unknown keyword
    # raises, and so does a parameter left unbound — an unbound focus would answer
    # over every subject, which is a silently wider answer rather than a visible
    # mistake.
    def run(
        self, **bindings: _Term
    ) -> QuerySolutions | QueryTriples | QueryQuads | QueryBoolean: ...

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
    # Fold everything mutated so far into this store's BASE, leaving the copy-on-write
    # delta empty. The store's CONTENTS are unchanged; what changes is what counts as a
    # "change". A freshly constructed `Store` has an EMPTY base, so without this the
    # delta of a store a million triples were loaded into IS those million triples and
    # `shapes.PreparedShapes.validate_store_changes` re-validates the whole graph.
    # Checkpoint after loading, mutate, and the delta is exactly the mutation.
    #
    # A real compaction (the base is rebuilt), so it is cheap once after a bulk load and
    # expensive in a tight mutation loop — which is why it is an explicit act rather than
    # something `add` does behind your back. Raises `ValueError` if the store cannot be
    # frozen.
    def checkpoint(self) -> None: ...
    # `(added, removed)`: how many quads this store has added since the last
    # `checkpoint`, and how many it has removed. The size of the change
    # `shapes.PreparedShapes.validate_store_changes` expands, readable without
    # validating anything.
    def change_size(self) -> tuple[int, int]: ...
    # Prepare a SPARQL query once, to be bound and run many times: `query` parses and
    # admits its text on every call, so a caller running one query per row pays that
    # cost per row for the same plan. `parameters` names the variables `run` will
    # bind, without the `?`/`$` sigil — each behaves exactly as a `query`
    # `substitutions` entry, so a parameter reaches inside `OPTIONAL`, `MINUS`,
    # `EXISTS` and sub-`SELECT`s by ordinary correlation.
    #
    # The returned `PreparedQuery` holds a reference to THIS store and re-reads its
    # current contents on every `run` rather than freezing a snapshot now, so a later
    # mutation is visible to the next run. It is not thread-safe — see
    # `PreparedQuery`.
    #
    # Engine configuration and relation/aggregate registration behave exactly as on
    # `query` below — `extension_namespaces`, `property_fn_namespaces`,
    # `standpoint_predicates`, `relations`, `relations_from_graph`, `path_relations`
    # and `aggregate_namespace` all admit the plan and are then CARRIED by the
    # returned `PreparedQuery`, so `PreparedQuery.run` evaluates under the SAME
    # registries the plan was admitted under. The two axes that read the store's own
    # graph — `relations_from_graph`, which reads a table written in it, and
    # `path_relations`, which traverses its edges — are REBUILT from each run's
    # dataset and the plan re-admitted under them, so a relation's rows are as fresh
    # as an ordinary triple pattern's and one answer is never assembled from two
    # points in time. `substitutions` has no seat here: a prepared query's whole point
    # is that the values that change between runs arrive per-run through `parameters`
    # / `PreparedQuery.run`'s bindings.
    def prepare(
        self,
        query: str,
        *,
        parameters: list[str] | None = ...,
        extension_namespaces: list[str] | None = ...,
        property_fn_namespaces: list[str] | None = ...,
        standpoint_predicates: tuple[str, str] | None = ...,
        relations: dict[str, _Relation] | None = ...,
        relations_from_graph: dict[str, _RelationFromGraph] | None = ...,
        path_relations: dict[str, _PathRelation] | None = ...,
        aggregate_namespace: str | None = ...,
    ) -> PreparedQuery: ...
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
    # INTERNAL cross-package protocol, not a caller surface: a capsule exposing a
    # frozen snapshot of this store by address, which `purrdf.shapes.Shapes`
    # (`purrdf_shapes`) calls BY STRING so the SHACL engine validates natively with no
    # N-Triples round-trip. Declared because it is live — the underscore is the whole
    # of its "do not call this" — and because a member the stub omits is a member a
    # checked caller cannot see at all, including to see that it is private. The
    # snapshot is immutable: a later `add`/`remove`/`update` leaves a capsule already
    # handed out untouched.
    def _store_capsule(self) -> CapsuleType: ...

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
    # The same INTERNAL cross-package protocol `Store._store_capsule` is, under the
    # same name and with the same pointee type: a capsule carrying a frozen snapshot
    # of this dataset by address, which `purrdf.shapes.Shapes.validate_store` reaches
    # BY STRING. Declared for the reason `Store`'s is — a member the stub omits is a
    # member a checked caller cannot see at all, including to see that it is private.
    # The snapshot is taken at the call and is immutable: a later `add`/`remove`/
    # `update` on this dataset leaves a capsule already handed out untouched.
    def _store_capsule(self) -> CapsuleType: ...

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

# The ingestion receipt for the same sources a snapshot build consumes. The producer
# entry points return bare `bytes` (or a bare `str`), which cannot carry an extra
# field, so the receipt rides a companion accessor — the same relationship
# `snapshot_content_id_native` has to `gts_from_quads`. Its arguments are
# `compile_gts_native`'s ingest half, so one accessor answers for both producer
# shapes: pass `base_data` alone for the single-dataset entry points, or add
# `rdf12_data` / `named_graphs` to mirror a `compile_gts_native` call.
#
# `declarations_omitted` is the load-bearing key: a named graph holding no row has
# no slot in the frozen `dist` snapshot payload, and interning its IRI would shift
# `snapshot_content_id`, so ingestion omits it — and names it here rather than
# dropping it in silence.
class GtsIngestReport(TypedDict):
    #: Rows read across all three tables (ordinary, reifier, annotation).
    rows_consumed: int
    #: Term rows this ingestion added to the snapshot dictionary.
    terms_interned: int
    #: The declaration-only graph IRIs that were NOT interned, sorted and deduplicated.
    declarations_omitted: list[str]
    #: Peak scratch bytes held by the intern indexes and sort buffers.
    scratch_bytes: int

def gts_ingest_report(
    base_data: bytes,
    base_format: RdfFormat,
    *,
    base_scope: str | None = ...,
    rdf12_data: bytes | None = ...,
    rdf12_format: RdfFormat | None = ...,
    rdf12_graph_name: str | None = ...,
    rdf12_scope: str | None = ...,
    named_graphs: list[_NamedGraphRow] | None = ...,
    base: str | None = ...,
) -> GtsIngestReport: ...

# `compile_gts_native` with BOTH answers of one build. `compile_gts_with_report`
# takes exactly the compiler's arguments and emits exactly its bytes, returning them
# beside the receipt for the very ingestion that minted them — ONE parse, ONE intern.
# It is the one-pass surface; `gts_ingest_report` remains the accessor for a caller
# who wants the receipt and no bytes. The receipt is the same `GtsIngestReport`, not
# a parallel shape.
class GtsCompileResult(TypedDict):
    #: The GTS container bytes, identical to `compile_gts_native`'s for these sources.
    snapshot_bytes: bytes
    #: The receipt for the ingestion that produced `snapshot_bytes`.
    ingest_report: GtsIngestReport

def compile_gts_with_report(
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
) -> GtsCompileResult: ...

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

# (term_id, kind, value, datatype_id, lang, reifier_id, triple).
#
# RDF 1.2 base direction is NOT a field here — it is the parallel `directions`
# column on `GtsRelationalRows`, index-aligned with `terms`. Callers unpack this
# row positionally, so widening it would break them at runtime with no type
# boundary to catch it.
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
    # RDF 1.2 base direction ("ltr"/"rtl"), positionally parallel to `terms`.
    directions: list[str | None]
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

# The relational EXPORT writers, layered in pure Python over
# `gts_relational_rows_from_bytes` (python/src/purrdf/_gts_export.py). Five tables
# — terms, quads, reifiers, annotations, blobs — written in the projection's own
# row order, so exporting the same container twice produces the same content.
#
# `gts_to_sqlite` uses the standard library and needs nothing extra. The other two
# need an optional dependency and raise `ModuleNotFoundError` naming the extra
# when it is absent: `pip install 'purrdf[duckdb]'` / `'purrdf[parquet]'`.
def gts_to_sqlite(data: bytes, path: str) -> str: ...
def gts_to_duckdb(data: bytes, path: str) -> str: ...

# Returns one path per table, in the fixed table order rather than directory order.
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
    # Either quad container, validated through the native snapshot seam: both hold
    # a frozen dataset behind their copy-on-write overlay, so neither is serialized
    # to N-Triples and parsed back to be validated. The report is a statement about
    # the data as it was — a later mutation moves the next report, not this one.
    # Anything that cannot hand over such a snapshot raises `TypeError` naming the
    # type that arrived and what is accepted; text belongs in `validate_nt`.
    def validate_store(self, data: Store | MutableDataset) -> _ValidationReport: ...
    # Analyze the shape tree once; the step a prepared PRODUCT is written from.
    def prepare(self) -> _PreparedShapes: ...
    # What the environment these declarations describe would make of every SPARQL
    # text the graph carries, asked BEFORE any validation runs. A relation IRI the
    # environment does not recognize becomes an ordinary triple pattern, matches
    # nothing, and conforms — which is also what a resolved relation over no rows
    # does, so the report cannot tell the two apart afterwards.
    #
    # Declarations rather than a registry: whether a predicate is a call is settled
    # at parse time by the IRI set and the declared namespaces alone, never by what
    # a relation would return, so the question can be asked before anything is
    # wired. An IRI you believe you registered appearing under `data` is the answer
    # to why it never ran.
    #
    # Returns `{"sites": {site: {"calls": [iri], "data": [iri]}}, "unreadable":
    # {site: reason}, "complete": bool}`. A non-empty `unreadable` means the graph
    # and these declarations disagree and validating under them fails at those sites.
    def extension_usage(
        self,
        *,
        relation_iris: Sequence[str] | None = None,
        relation_namespaces: Sequence[str] | None = None,
        extension_namespaces: Sequence[str] | None = None,
    ) -> dict[str, Any]: ...

class _ShapesProductError(ValueError):
    """A refusal from the prepared-shapes-product admission boundary.

    `dimension` is one of the codec's pinned kebab-case labels, or `None` when the
    failure happened before any product existed (a shapes or data document that did
    not parse was never admitted). Branch on it, never on `str(exc)`.
    """

    dimension: str | None

class _PreparedShapes:
    """An immutable shape preparation, reusable across data graphs and writable as
    a prepared product."""

    # Byte-deterministic: equal preparations produce identical bytes.
    def to_product(self) -> bytes: ...
    # Where this preparation came from, as one deterministic token: `parsed`,
    # `restored-admitted <identity_digest>` or `restored-rebuilt <identity_digest>`.
    # Total — there is always an answer, and none of them means "unknown".
    def provenance(self) -> str: ...
    def validate_nt(self, data_nt: str) -> _ValidationReport: ...
    # THE INCREMENTAL LANE: validate only what `store`'s PENDING CHANGE can move,
    # rather than the whole graph. A `Store` records its mutations as a copy-on-write
    # delta over a frozen base, so a mutated store already holds the one thing
    # incremental validation needs — a description of what moved.
    #
    #     store.load(base_ttl, RdfFormat.TURTLE)
    #     store.checkpoint()      # everything loaded so far is now the BASE
    #     store.add(quad)         # ... and this is the change
    #     outcome = prepared.validate_store_changes(store)
    #
    # Call `Store.checkpoint()` first or the "change" is the whole store, and read
    # `ChangeValidation.bounded` before reading `conforms`: the two arms answer
    # different questions. Raises `ValueError` when the store cannot be snapshotted,
    # when the snapshot exceeds the view's retention limits, or when constraint
    # evaluation hard-fails.
    def validate_store_changes(self, store: Store) -> _ChangeValidation: ...

class _ChangeValidation:
    """The outcome of `PreparedShapes.validate_store_changes`: the report, plus the
    SCOPE the report describes.

    Two facts rather than one, because a `ValidationReport` alone cannot say which
    question it answered. Read `bounded` first.
    """

    # The SHACL validation report. See `bounded` for what it describes.
    @property
    def report(self) -> _ValidationReport: ...
    # `True`: the change's footprint was bounded, the report describes the AFFECTED
    # focus nodes only, and `conforms` means THIS CHANGE introduced no violation (it
    # says nothing about a pre-existing violation the change cannot reach). `False`:
    # no bounded footprint exists for this shapes graph, the run fell back to a FULL
    # validation of the mutated graph, and `conforms` means the whole graph conforms.
    @property
    def bounded(self) -> bool: ...
    # How many focus nodes the change expanded into, or `None` on the unbounded arm.
    # `None` rather than the graph's node count: "every focus node in the graph" and a
    # number are different statements, and collapsing them would make a fallback
    # indistinguishable from a large bounded expansion.
    @property
    def focus_nodes(self) -> int | None: ...
    # Which construct made this shapes graph's change footprint unbounded (SPARQL query
    # text: `sh:sparql`, a SPARQL target, a component's `sh:ask`/`sh:select` validator,
    # a `sh:SPARQLFunction` call, a SPARQL node expression), or `None` when it was
    # bounded. Actionable rather than decorative: it names what to change to get
    # incremental validation back.
    @property
    def reason(self) -> str | None: ...

class _ShapesProduct:
    """A prepared product whose envelope is verified and whose self-description is
    decoded — but which has NOT been admitted."""

    @staticmethod
    def open(data: bytes) -> _ShapesProduct: ...
    def to_bytes(self) -> bytes: ...
    # Deterministic `key value` lines describing what the product was compiled from.
    def explain(self) -> str: ...
    def format_version(self) -> int: ...
    def stage_id(self) -> str: ...
    # False means `admit()` will refuse and `rebuild()` is the path that restores.
    def stage_known(self) -> bool: ...
    def identity_digest(self) -> str: ...
    # Ordered `(label, rendered_value)` pairs: the ORDER is the identity.
    def identity_components(self) -> list[tuple[str, str]]: ...
    def admit(self) -> _PreparedShapes: ...
    # Admit only the product whose `identity_digest()` is `expected_identity`; a
    # different binding raises with `.dimension == "shapes-graph"`.
    def admit_expecting(self, expected_identity: str) -> _PreparedShapes: ...
    def rebuild(self) -> _PreparedShapes: ...
    # Rebuild only the product whose `identity_digest()` is `expected_identity`; a
    # different binding raises with `.dimension == "shapes-graph"`.
    def rebuild_expecting(self, expected_identity: str) -> _PreparedShapes: ...
    def certify(self) -> None: ...

class shapes:
    # Every re-export below is spelled with an explicit `TypeAlias` — as
    # `purrdf.entail.Regime` is, and for the same reason — because each is a type a
    # caller ANNOTATES with: a function that takes a prepared shapes graph or returns
    # the outcome of a change validation writes `purrdf.shapes.PreparedShapes` or
    # `purrdf.shapes.ChangeValidation` in the signature, and a plain `X = X` reads to a
    # type checker as a variable, which is then rejected in annotation position. There
    # is no other public spelling of these names — `purrdf.PreparedShapes` does not
    # exist and `__all__` carries neither — so a plain assignment here makes the type
    # unwritable rather than merely awkward.
    ValidationReport: TypeAlias = _ValidationReport
    Shapes: TypeAlias = _Shapes
    PreparedShapes: TypeAlias = _PreparedShapes
    ChangeValidation: TypeAlias = _ChangeValidation
    ShapesProduct: TypeAlias = _ShapesProduct
    ShapesProductError: TypeAlias = _ShapesProductError
    # Compile a Turtle shapes graph into a prepared product in one call — the
    # composition of `Shapes(...).prepare().to_product()`.
    @staticmethod
    def pack_product(shapes_ttl: str, *, shapes_base: str | None = None) -> bytes: ...
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
    # Spelled with an explicit `TypeAlias`, as every type re-exported by a namespace
    # class here is, because `purrdf.entail.Regime` is a *type* every call site
    # annotates with; a plain assignment reads to mypy as a variable and is then
    # rejected in annotation position.
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
    # The two TYPES of this namespace, so spelled with an explicit `TypeAlias`: the
    # function re-exports above are values a caller CALLS, but these are written in
    # annotation position, where a plain assignment reads to a type checker as a
    # variable and is rejected.
    RdfDataset: TypeAlias = _RdfDataset
    GtsFoldViewNative: TypeAlias = _GtsFoldViewNative

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
    # The analysis borrows the catalog, so it runs EAGERLY here and the owned report is
    # retained: constructing one is the work, and `analyze()` hands back what it found.
    def __init__(self, catalog: _SliceCatalog) -> None: ...
    def analyze(self) -> _OwnershipReport: ...
    def analysis_graph_turtle(self) -> str: ...

class slice:
    # Every one an explicit `TypeAlias`, for the reason spelled out on `class shapes:`
    # above: these are the types a caller writes in a signature, and a plain assignment
    # reads to a type checker as a variable that is rejected in annotation position.
    ArtifactRecord: TypeAlias = _ArtifactRecord
    ManifestView: TypeAlias = _ManifestView
    SliceRecord: TypeAlias = _SliceRecord
    DependencyEdge: TypeAlias = _DependencyEdge
    ManifestPatch: TypeAlias = _ManifestPatch
    OwnershipReport: TypeAlias = _OwnershipReport
    SliceCatalog: TypeAlias = _SliceCatalog
    OwnershipAnalyzer: TypeAlias = _OwnershipAnalyzer

# ── Ranked retrieval (bindings/python/src/py_retrieval.rs, purrdf_native.retrieval) ──
# The composition layer over the ranked property-function producers, surfaced as
# `purrdf.retrieval`: one request planned, admitted, executed and fused into one
# ordered answer. Producers, strata and weights are caller-supplied; nothing here
# has a default, because PurRDF mints no vocabulary.
#
# Every weight crosses as an `int` of raw fixed-point units (`SCALE` is one whole
# unit) and every score comes back as its exact decimal `str`, never a float.

# One ranked text producer's declaration: `(stratum, predicate, graph)`, the same
# three followed by the producer's candidate domains, or those four followed by
# what the host attests about the index behind the producer.
#
# `domains` is `None` — which is also what omitting the fourth element declares —
# or a list of domain-tag IRI strings. `None` is `Unrestricted`: "this producer
# may name anything", which licenses a consumer to skip nothing and is exactly
# what every answer this binding produced before the element existed. A list
# promises every candidate this producer names lies in one of those blocks, which
# is what lets a fused top-k stop reading a stream that provably cannot name the
# candidate it is deciding about. It buys a shorter READ, never a different
# answer: the rows, the scores and the provenance are identical either way.
#
# The tags are the HOST's, because which entities an index names is a fact about
# the corpus that neither this layer nor the relation can see. A tag derived per
# stratum would hand two producers over one entity space a pair a consumer reads
# as disjoint, and that mistake is not conservative in either direction — it
# refuses a valid query where both producers name one entity, and certifies a
# score missing the other's contribution where they do not.
#
# An EMPTY list raises `ValueError` naming the producer. A promise to name
# nothing is not a narrow producer but one that should not be registered: a
# consumer holds a producer to its declaration row by row, so every row it
# emitted would contradict it. Pass `None` to restrict nothing.
#
# A list naming MORE THAN ONE distinct block raises `ValueError` naming the
# producer too, and it is refused at registration rather than at the first row.
# One block is entailed by the declaration, so a consumer reads it off the
# declaration and no row repeats it; several blocks say only that the rows lie
# somewhere in the set, which obliges the producer to name each row's own block —
# and the ranked relations this surface builds project a candidate and a score
# and declare no such column, because a tag describes how a host's corpora
# partition and only the host knows that. The refusal quotes the blocks in
# canonical order and names three exits: exactly one tag (the block this
# producer's rows really lie in), one producer per block with its own stratum, or
# `None`. A list that repeats one tag names one block and is accepted.
#
# A declaration is a promise, and `search` checks it against the rows it pulls.
# Two producers whose declarations place one candidate in disjoint blocks cannot
# both be telling the truth about it, so the fusion raises `ValueError` —
# "stream for stratum S named item I, which its declared candidate domains
# cannot reach; stratum T already named it" — naming the stratum that broke its
# promise, the stratum whose already-applied declaration it collided with, and
# the candidate. It is not silently widened instead: rows have already been
# certified on the strength of that declaration, so merging the late
# contribution would hand back a score its own provenance contradicts. The fix
# is to declare the tag the two producers share, or `None`; producers that
# really do rank the same entities fuse into one row carrying both
# contributions under either. The sibling refusal on the same seam — "stream for
# stratum S emitted item I more than once" — is a producer that declared unique
# candidates and repeated one, and names its stratum for the same reason.
#
# The FIFTH position is the attestation: `(generation, incompleteness)`, each a
# `str` or `None`, recorded verbatim and never parsed — the same declaration
# `_Relation` carries on the SPARQL lane, and the same one part of a producer that
# nothing else crossing this boundary can express. Which version of the host's
# index answered, and whether it was NOT whole, change an answer while every input
# the engine can see stays identical, so if the host does not say, nothing can.
#
# Declaring nothing is SILENCE, and silence is not a claim that the index was
# whole: there is no seam at which wholeness can be certified — a producer stopped
# at a row ceiling never looked at the rows it skipped — so neither this binding
# nor the engine mints such a claim. A spec written without the position declares
# exactly that silence and behaves as it always has.
#
# * `incompleteness` is the host's own reason the index was not whole ("shard 3 of
#   4 is still rebuilding"). `search` reports it verbatim under
#   `["attestations"][stratum]["incomplete"]` and names that stratum under BOTH
#   `["exactness"]["deficit"]` and `["exactness"]["inflation"]`: every score in
#   that answer is an ESTIMATE rather than a value, and the error runs in both
#   directions, because fusion scores by rank and a row the short index never
#   named is summed too low while every row behind it moved up a rank and is
#   summed too high. The rows are still real rows in the fusion's own certified
#   order, and what does not follow is that a row absent from the answer would
#   have stayed absent. It is reported rather than refused because this answer
#   has a slot to say it in.
# * `generation` is the host's own name for the index version that answered. It
#   appears under `["attestations"][stratum]["generation"]` and is NOT a shortfall
#   — an answer whose producers named only generations is still exact. It REPLACES
#   the content digest the shipped text relation would otherwise attest, because
#   one generation is pinned per invocation; a spelling that does not move when
#   the host's corpus does makes two answers from two index states carry one
#   `"evidence_id"`. Declaring an incompleteness alone leaves the digest in place.
#
# `plan` and `compile` read the same value — one producer declaration serves all
# three entry points — and neither reports it: they execute nothing, so no index
# has answered yet and there is nothing to attest about. It reaches no plan, no
# compiled unit and no identity either returns, the registry's content
# fingerprint included: that is a function of what a producer declares to the
# PLANNER, and an attestation declares nothing there.
#
# The position is FIFTH rather than fourth-or-fifth, and the `domains` position is
# written explicitly (as `None` to restrict nothing) to reach it. A four-element
# value's tail is always `domains`: `("a", "b")` is a well-formed two-tag
# restriction and a well-formed attestation at once, and guessing which the host
# meant would report a domain tag back to an operator as an index generation, or
# register a producer whose rows cannot back a restriction it never made. A value
# of any other width, or a fifth position that is not a two-member sequence (a
# bare string included — a `str` is a sequence of its own characters, and reading
# `"ab"` as `generation="a"` would put a claim in the host's mouth), raises
# `TypeError` naming all three accepted widths. A fifth position that IS a
# two-member sequence whose member is neither `str` nor `None` raises `TypeError`
# naming that member.
#
#     retrieval.search(
#         data,
#         [("lexical", "quick fox", None, "https://example.org/note")],
#         text_producers={
#             "https://example.org/pf/search": (
#                 "https://example.org/stratum/lexical",
#                 "https://example.org/note",
#                 "any",
#                 None,
#                 ("notes-index-7", "shard 3 of 4 is still rebuilding"),
#             )
#         },
#         ...,
#     )["exactness"]  # {"exact": False, "deficit": [".../stratum/lexical"],
#                      #  "inflation": [".../stratum/lexical"], "unbounded": []}
# What a producer's own SEARCH promises about the rows it can name, on two
# independent axes: `(completeness, order)`. Shaped like `_Attestation` beside it
# and read on the same terms — each member is a `str` or `None`, `None` is
# SILENCE on that axis, and a string is that axis declared degraded with the
# host's own words carried verbatim. Nothing parses either string; the member's
# POSITION is what says which axis it is about, so there is no tag to spell and
# no whitespace to lose.
#
# The two say different things about different objects. An attestation is about
# the INDEX — which version answered, and whether that version was whole. A
# fidelity is about the SEARCH over it — whether it names every row that was due
# (`completeness`), and whether a row it names arrives at a rank no better than
# it earned (`order`). Only the second breaks a score bound, which is why a
# perturbed order is what makes an interval `{"bounded": False}`.
#
# Silence on both is the top of the lattice, and it is a FACT here rather than a
# fabricated default: this binding builds the index in the same call, out of the
# document it was handed, and BM25 over it scores every document carrying a query
# term with no pruning and compares nothing in an approximated space. What it
# cannot see is whether that document is the whole of what the host means — a
# sample, one partition, a snapshot that has fallen behind, or text that was
# transliterated or machine-translated before it arrived. That is what a member
# is for, and the host is the only party who knows it.
#
#     purrdf.retrieval.search(
#         ...,
#         text_producers={
#             "https://example.org/pf/search": (
#                 "https://example.org/stratum/lexical",
#                 "https://example.org/note",
#                 "any",
#                 None,
#                 (None, None),
#                 ("a 10% sample of the corpus", None),
#             )
#         },
#         ...,
#     )["fidelities"]["https://example.org/stratum/lexical"]
#     # {"completeness": "lossy",
#     #  "completeness_evidence": "a 10% sample of the corpus",
#     #  "order": "faithful"}
_Fidelity: TypeAlias = tuple[str | None, str | None]
_TextProducerSpec: TypeAlias = (
    tuple[str, str, str]
    | tuple[str, str, str, list[str] | None]
    | tuple[str, str, str, list[str] | None, _Attestation]
    | tuple[str, str, str, list[str] | None, _Attestation, _Fidelity]
)
# One vector space, as the rows a vector producer ranks: `(iri, vector)` pairs
# in the host's own row order, which is kept because it is what ranks two
# neighbours at exactly equal distance. The IRI is the entity the row stands for
# — the spelling a fused row's `"entity"` names, without the angle brackets — and
# every vector carries the same number of finite components. A seed is an
# `("entity", "<iri>")` request term naming a row the space holds: a vector
# producer searches FROM a term it already has a vector for.
#
# An empty row list, a row whose width differs from the first row's, a row IRI
# the IRI parser refuses, a non-finite component, one IRI on two rows, and more
# rows than the guard's `max_candidates` each raise `ValueError` naming the
# producer, before any row is read. Under `"cosine"` a zero vector is refused
# too: it has no direction, so its distance to anything is undefined.
_VectorRows: TypeAlias = list[tuple[str, list[float]]]
# The distance a space ranks by: "cosine", "negative_dot" or
# "squared_euclidean". Any other spelling raises `ValueError` naming the three.
_VectorMetric: TypeAlias = str
# `(max_candidates, max_neighbours)`: the largest space the producer admits, and
# the largest neighbour count one invocation may ask for. Both are the host's
# statement of what it will spend and neither has a default; a zero in either
# admits nothing and raises `ValueError`.
_VectorGuard: TypeAlias = tuple[int, int]
# One HNSW producer: `(stratum, rows, metric, guard, (m, m0, ef_construction,
# ef_search), domains, attestation, order)`. Every position is written — `None`,
# `(None, None)` and `None` where the host states nothing — so a value is never
# read by guessing which optional position its tail meant.
#
# The graph is built in the call, deterministically, from the rows and the four
# parameters; an invalid parameter set (`m < 2`, `m0 < m`, `ef_construction <
# m0`, `ef_search < 1`) raises `ValueError`. `domains` and the attestation read
# exactly as `_TextProducerSpec`'s do: a silent attestation axis delegates to the
# relation's own, which attests the content digest of the graph and its terms.
#
# `order` is `None` — the vectors are the values the host meant — or the host's
# own words for how they were approximated before they arrived (a non-empty
# `str`, carried verbatim into `"fidelities"`). There is no completeness
# position: a beam search offers the candidates it reached and never certifies
# that nothing else matched, so this producer's stratum is always reported
# `"lossy"`, with the relation's own evidence, and the answer's `"exactness"`
# names it.
_HnswProducerSpec: TypeAlias = tuple[
    str,
    _VectorRows,
    _VectorMetric,
    _VectorGuard,
    tuple[int, int, int, int],
    list[str] | None,
    _Attestation,
    str | None,
]
# One exact nearest-neighbour producer: `(stratum, rows, metric, guard, domains,
# attestation, fidelity)`, every position written. The scan compares every row's
# exact distance, so over the vectors it holds it names every neighbour that was
# due and orders them truly; whether those vectors are the whole of the host's
# corpus is the host's to say, on the same `(completeness, order)` fidelity a
# text producer carries. `(None, None)` states nothing on either axis.
_KnnProducerSpec: TypeAlias = tuple[
    str,
    _VectorRows,
    _VectorMetric,
    _VectorGuard,
    list[str] | None,
    _Attestation,
    _Fidelity,
]
# Every relation the three producer maps build declares its own exclusion basis,
# `"membership"`: asked whether it will ever name a candidate, it answers out of
# its own index — a posting for the needle, or a row for the term. There is no
# position that asserts or withdraws a basis, because what an exclusion answer
# is a fact about is a property of the index rather than something a host can say
# on the producer's behalf.

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

class retrieval:
    # The decimal exponent of one whole fixed-point unit.
    SCALE_DIGITS: int
    # One whole fixed-point unit, in raw units: the weight `Fixed::ONE`.
    SCALE: int
    # Spelled with an explicit `TypeAlias`, exactly as `purrdf.shapes` spells its
    # own re-exports and for the same reason: this is a type a caller ANNOTATES
    # with, and a plain `X = X` reads to a type checker as a variable.
    PlanDocumentError: TypeAlias = _PlanDocumentError

    # Every entry point that takes a smoothing constant `k` also takes the
    # `decay` rule it belongs to, as one of two spellings, with no default:
    #
    # * "reciprocal_rank" truncates the reciprocal to the declared scale BEFORE
    #   the weight is applied. The inner truncation is a ceiling no weight can
    #   lift, so this rule stops separating adjacent ranks at the same depth —
    #   just past a million — for every weight at or above one.
    # * "weighted_reciprocal_rank" folds the weight into the numerator as one
    #   exactly-rounded division. Its value is never below the other's, and its
    #   reachable depth grows with the weight, so a heavier stratum is readable
    #   deeper.
    #
    # The two compute different contributions from the same weights and name
    # different content-addressed laws, so neither is a default and an omitted
    # rule is refused. An unknown spelling raises `ValueError` naming both.

    # Plan one request against the declared ranked producers, executing nothing.
    #
    # Each `request` entry is a tuple whose first element names its kind:
    # ("lexical", text, language | None, predicate | None),
    # ("vector", [component, ...], "cosine" | "dot" | "euclidean", hint | None),
    # ("spatial", geometry, predicate, max_distance_raw | None),
    # ("temporal", predicate, lower | None, upper | None),
    # ("numeric", predicate, lower_raw | None, upper_raw | None), or
    # ("entity", term).
    #
    # `text_producers` maps a producer IRI to (stratum, predicate, graph), to
    # (stratum, predicate, graph, domains), or to those four followed by one
    # (generation, incompleteness) attestation and then one (completeness, order)
    # fidelity, where `graph` is "any", "default",
    # or a named-graph IRI and `domains` is the producer's candidate-domain
    # declaration (see `_TextProducerSpec`: `None` or an omitted fourth element
    # promises nothing and restricts nothing, a list of tag IRIs restricts the
    # producer to those blocks, and an empty list is refused by name; the fifth
    # position is what the host attests about the index behind the producer, and
    # declaring nothing there is silence rather than a claim the index was whole).
    #
    # `hnsw_producers` and `knn_producers` map a producer IRI to a
    # `_HnswProducerSpec` or a `_KnnProducerSpec`: an approximate or an exact
    # nearest-neighbour search over the host's own vectors, seeded by an
    # ("entity", "<iri>") request term. Omitting either map registers no producer
    # of that kind. All three maps are one registry: two producers of any kinds
    # that claim one stratum, or one producer IRI written into two maps, raise
    # `ValueError` naming both.
    # `statistics` must name
    # its "source" and "revision", and may carry "cardinality" (stratum IRI to
    # row count) and "selectivity" ((stratum IRI, request-term index) to an
    # integer of parts per million in [0, 1000000] — never a float, because the
    # value reaches the plan's canonical identity). A reported selectivity
    # lowers that stratum's planned depth; it never raises one.
    #
    # The returned plan states what it was built against, in two places that
    # answer two questions. `"stratum_derivations"` maps each stratum to
    # everything its depth was derived from — "declared", "cardinality",
    # "selectivity_ppm", "selectivity_terms" and "licensed_prefix", plus a
    # "cause" naming which of them bound the number ("declaration",
    # "cardinality", "selectivity", "licensed_prefix", "floor", "unbounded" or
    # "read_ceiling"). Those seven are the whole vocabulary: the classification
    # is a closed enum rendered by an exhaustive match, so a cause this list does
    # not name is a compile error in the binding rather than a word handed to a
    # caller. The depth is recomputable from those inputs, which is
    # what makes it a checkable claim rather than an asserted one — and
    # `certify_plan` is where it is recomputed, over a document that came from
    # somewhere else. `plan` does NOT recompute it: it just derived every one of
    # those depths, so checking them here would charge every caller on every
    # call, once per stratum, for an answer this build had a moment earlier.
    # `"statistics"["entries"]` names EVERY subject planning consulted —
    # each of those strata, and each predicate the request named — so a caller
    # asking one question ("which statistics was this planned against?") reads
    # one list. A stratum's entry is a projection of its derivation rather than a
    # second consultation of the host, and `certify_plan` refuses a plan whose
    # two records of one stratum disagree; a subject that is both a stratum and a
    # request predicate is named once, carrying the derivation's own values.
    #
    # In both, "cardinality" and "selectivity_ppm" are `None` when the provider
    # reported none, never 0: zero is a measurement and absence is not, and a
    # subject the provider was silent about is still named so that a replay
    # against moved statistics is detectable. A subject planning never consulted
    # is absent from both rather than recorded as empty.
    #
    # `top_k` is required here, on the stage that executes nothing, because the
    # row bound is a PLANNING input: it decides how deep each stratum is read, so
    # a top-five request and a top-five-hundred request are different plans with
    # different `"stratum_depths"` and different `"plan_id"` values. Whether it
    # actually narrows a depth is decided by the producers' own `domains`: over
    # strata whose declared blocks do not overlap each stratum is planned to `k`
    # rows and no deeper, and over anything else the declared-or-measured bound
    # stands. It never widens a depth, and it never changes an answer.
    #
    # `"canonical_bytes"` is the plan's canonical, length-framed encoding as
    # `bytes` — what a host stores, logs, or sends somewhere else. `"plan_id"` is
    # its digest, so the two are one fact: a plan can leave this process and be
    # handed back to `certify_plan`, which is where the recorded depths are
    # checked against the recorded inputs.
    @staticmethod
    def plan(
        data: str,
        request: list[tuple[builtins.object, ...]],
        *,
        text_producers: dict[str, _TextProducerSpec],
        statistics: dict[str, builtins.object],
        top_k: int,
        hnsw_producers: dict[str, _HnswProducerSpec] | None = None,
        knn_producers: dict[str, _KnnProducerSpec] | None = None,
        data_format: str = "turtle",
        base: str | None = None,
    ) -> dict[str, builtins.object]: ...
    # Decode a plan document and check every depth it records against the inputs
    # it records beside them.
    #
    # `plan_bytes` is exactly what `plan(...)["canonical_bytes"]` handed out, and
    # the answer is the DECODED plan rendered as `plan` renders it — so a host
    # reads back the document it received rather than the one it believes it
    # sent. The round trip is exact: an untampered document renders equal to the
    # dict it came from, `"plan_id"` included.
    #
    # A plan is untrusted input. It can be edited and it can be forged, and its
    # depths decide how deep each stratum is actually read. The plan records
    # every input those depths were derived from, so this recomputes each with
    # the engine's own arithmetic and refuses a plan the two disagree about.
    #
    # It is the COLD path and is on no hot one: `plan`, `compile` and `search`
    # do not run it, and neither does admission. What it answers — is this
    # document internally coherent at all — is a property of the bytes alone,
    # unmoved by the registry or the statistics in force now, so it is asked once
    # by the party that received them rather than on every call by the party that
    # produced them.
    #
    # Every refusal raises `retrieval.PlanDocumentError` with a pinned `.refusal`
    # name; branch on that, never on the message. A layout this build does not
    # write is `version`; a keyed section out of order is `non-ascending-keys`,
    # and a record's run of contributing request-term indices out of order is
    # `non-ascending-selectivity-terms`; one stratum recorded twice is
    # `duplicate-stratum-depth` or `duplicate-stratum-derivation`, and one term
    # counted twice into one selectivity is `duplicate-selectivity-term`; a depth
    # that does not follow from its own inputs is `depth-not-derivable`; a
    # stratum the snapshot names nowhere is
    # `derivation-without-statistics-entry`, and a snapshot row for a subject
    # nothing consulted is `unconsulted-statistics-subject`; a snapshot row
    # saying something other than the derivation beside it is
    # `statistics-entry-contradicts-derivation`; and a recorded selectivity
    # domain indexing a term the plan's own request does not carry is
    # `selectivity-term-out-of-range`. `PlanDocumentError` documents every one.
    @staticmethod
    def certify_plan(plan_bytes: bytes) -> dict[str, builtins.object]: ...
    # Which recorded input bound `stratum`'s depth in the plan document
    # `plan_bytes`, or `None` when that plan records no derivation for it.
    #
    # One of `"declaration"`, `"cardinality"`, `"selectivity"`,
    # `"licensed_prefix"`, `"floor"`, `"unbounded"` or `"read_ceiling"` — the
    # same closed vocabulary `plan` renders under each derivation's `"cause"`,
    # from the same engine call. A depth of one is the motivating case: it
    # arrives by four different roads with four different remedies, and the
    # number alone does not say which.
    #
    # It reads the derivation the document records and does NOT certify it. "Which
    # leg bound this number" and "does this number follow from those legs" are
    # different questions, and `certify_plan` answers the second over the whole
    # plan at once. Raises `retrieval.PlanDocumentError` for every way the
    # document itself is refused, plus `invalid-iri` when `stratum` is not an IRI.
    @staticmethod
    def explain_depth(plan_bytes: bytes, stratum: str) -> str | None: ...
    # Plan, admit, and emit the per-stratum SPARQL the request compiles to.
    #
    # Each entry under `"units"` is `{"stratum": str, "sparql": str, "depth": int,
    # "declared_rows": int | None}`. `"depth"` is the REPORTABLE bound — the most
    # rows that stratum may contribute to an answer — and it is deliberately NOT
    # the `LIMIT` in
    # `"sparql"`. The text is emitted exactly `depth + 1` rows deep, and that last
    # row is a probe: it exists only so a reader can tell a producer that ran out
    # of rows from a read the planned depth cut short, two endings a text bounded
    # at exactly `depth` cannot distinguish. The probe row is a READ and never a
    # value. So a host that runs the text itself keeps at most `"depth"` rows and
    # reports nothing past them; `search`, which runs the units for you, already
    # does.
    #
    # The probe is emitted even where the producer's declared row bound already
    # equals the depth, and that case is the one it exists for: a read stopping
    # exactly at the declaration cannot tell a producer that ran out from one the
    # bound cut, so reporting exhaustion there would rest on a number nobody
    # checked. The unit asks for one row more instead. If that row arrives the
    # producer contradicted its own registration and the read is refused by name;
    # if it does not, the exhaustion is verified rather than believed. It is
    # emitted at a declared row bound of ZERO too: that declaration is read rather
    # than obeyed, so the depth is floored at one row and the text still reaches
    # for a second.
    #
    # One shape cannot be probed, and its answer says so rather than guessing. A
    # relation that takes the depth as an ARGUMENT bounds itself by the number it
    # is handed, and that number is never raised past the row count the relation
    # registered — asking for more asks it to contradict its own registration. So
    # where the depth already sits on that registration the relation is asked for
    # exactly `depth` rows and no row past them can arrive, however many its index
    # holds. That stratum's status is `"row_bound_reached"`, which names the
    # declared bound as the stopper and claims nothing about what lies below it.
    # No relation THIS module registers has that shape — the text producers it
    # wires place no depth argument — so the shape is described for a host driving
    # the Rust surface, and `"declared_rows"` equalling `"depth"` here does not put
    # a Python caller in it.
    #
    # The extra row is in the `LIMIT` only — a ceiling the evaluator applies to a
    # cursor the producer never hears about, so probing costs nothing and a
    # producer that reads a depth argument is never asked to exceed what it
    # registered. That is why `depth + 1` holds with no exception: what a
    # self-bounding producer is asked for is capped at its declaration, but the
    # `LIMIT` the text carries is not, and those are two different numbers.
    #
    # `"declared_rows"` is the row count the registry declared for that stratum's
    # one producer — the number the depth was checked against — and it is `None`
    # for a producer that declared no access mode and therefore declared no row
    # count at all. "Declared nothing" and "declared zero" are different facts and
    # do not share a representation: an absent declaration can refuse nothing,
    # while a zero is a measurement of the producer's data. It is here because the
    # depth alone cannot say which situation a host is in. A depth BELOW
    # `"declared_rows"` leaves rows underneath the read; a depth EQUAL to it means
    # the producer has already promised there is nothing further, and the probe row
    # is what checks that promise rather than taking it. The distinction is not
    # recoverable from `"depth"`, from the text, or from the plan — and a host
    # reading `"depth"` to know how many rows it may report has the same claim on
    # it that `search` does.
    #
    # Read the bound off `"depth"`, never off the text's `LIMIT`, which is always
    # the larger of the two. `"planned_resolution"` is not a fallback
    # source for it: that map is empty unless the call names a fusion law.
    #
    # `weights`, `k` and `decay` are the fusion law the caller means to fuse
    # under, and naming it is what makes `"planned_resolution"` answerable: per
    # weighted stratum, the `"separates_to"` depth this law still tells adjacent
    # ranks apart at (`None` when it never stops inside a depth a plan can
    # express), the `"requested_depth"` the plan recorded, and
    # `"fully_separated"`. That is what the plan will cost in rank resolution,
    # known without executing a single unit. Omit all three and the map is empty
    # — no law is invented to measure against — and naming some of them and not
    # the rest raises `ValueError` saying which part is missing, because the
    # three are one law between them. The rule matters most here: a stratum the
    # truncated rule reports as coarse may be fully separated under the folded
    # one at the same weight.
    #
    # `top_k` is required, as it is on `plan` and for the same reason: the depths
    # this stage emits a `LIMIT` for were derived from it. This stage narrows
    # nothing on its own — a `LIMIT` smaller than `"depth"` would make `"depth"`
    # and `"planned_resolution"` describe a read nobody took.
    @staticmethod
    def compile(
        data: str,
        request: list[tuple[builtins.object, ...]],
        *,
        text_producers: dict[str, _TextProducerSpec],
        statistics: dict[str, builtins.object],
        top_k: int,
        hnsw_producers: dict[str, _HnswProducerSpec] | None = None,
        knn_producers: dict[str, _KnnProducerSpec] | None = None,
        weights: dict[str, int] | None = None,
        k: int | None = None,
        decay: str | None = None,
        data_format: str = "turtle",
        base: str | None = None,
    ) -> dict[str, builtins.object]: ...
    # Run the whole ladder and return one fused answer.
    #
    # `weights` maps a stratum IRI to its weight in raw fixed-point units, where
    # `SCALE` (that is, `10 ** SCALE_DIGITS`) is one whole unit. A weight of one
    # is `SCALE`, never the literal `1`, which is one raw unit. Weights are read
    # only as ratios, so a dict mixing the two spellings is a silent
    # factor-of-`SCALE` error: it runs, refuses nothing, and ranks as though the
    # smaller stratum were absent. Write every weight the same way.
    #
    # `k`, `decay` and `top_k` are required: the fusion law is the caller's and
    # fused enumeration is top-k by construction. How many contributions a
    # candidate may receive is not a parameter — it is the number of weighted
    # strata, because a candidate surfaces at most once in each.
    #
    # `top_k` is also a planning input, which is why `plan` and `compile` take it
    # too: over strata whose declared `domains` do not overlap it is what each
    # stratum's depth — and therefore its emitted `LIMIT` — is derived from, so the
    # work a bounded search does is bounded in rows READ and not only in rows
    # returned. The answer is identical either way; see `plan`. `decay` reaches
    # every number in the answer, not only the law's identity: the two rules
    # produce different contributions, different resolution maps and different
    # `"profile_id"` values from the same weights.
    #
    # The answer reports rank resolution under two distinct keys.
    # `"planned_resolution"` is the admission waist's map, identical to what
    # `compile` reports for the same request under the same law: what the plan's
    # depths were going to cost, knowable before any row was read.
    # `"observed_resolution"` is what the rows this run actually pulled did cost,
    # with an entry per weighted stratum a stream was fused for — including one
    # that yielded no rows, whose `"ranks_pulled"` is zero rather than absent:
    # `"separates_to"`, the `"ranks_pulled"` reached, the
    # `"collisions_observed"`, the `"exclusion_lookups"` spent and the
    # `"rows_materialised"` the reads behind that stratum returned. The two
    # resolutions legitimately disagree — a top-k that certified early never
    # reaches its planned depth — and neither is a correction of the other.
    #
    # The last two are COST, and they are on the answer rather than behind a
    # diagnostics switch. `"ranks_pulled"` is what the fusion consumed, and on
    # its own it cannot tell an expensive answer from a cheap one: a five-row
    # answer whose producers were drained four hundred rows deep reports a
    # perfectly truthful `"exhausted"` status and nothing else here would say
    # what it cost. `"exclusion_lookups"` counts the point queries the fusion
    # spent settling finality — a different read of a different question, never
    # added into the rank — and `"rows_materialised"` counts the rows the
    # stratum's one read produced. Every stratum is read on demand, one
    # invocation read a row per pull, so this is the ranks the fusion pulled plus
    # the probe row where it read past the planned depth — never a row read
    # twice. It is `None` only for a stream with no read behind it, which is
    # never one this module builds; the absence means "there is no read to
    # count", never "the read was free".
    #
    # `"statuses"` maps a stratum to its producer's own terminal status, and the
    # `"status"` string has exactly seven spellings. `"exhausted"` (with
    # `"rows_emitted"`: int) is the only one that names no stopper — that producer
    # emitted every row ITS SEARCH PRODUCED. On its own that is not a claim that
    # everything matching was returned, which is why it is read beside the
    # stratum's `"fidelities"` entry and never instead of it. The other six each
    # name who stopped the read and where, and none may be read as "that was all
    # of it":
    # `"depth_reached"` (with `"rank"`: int) is the producer stopping at the depth
    # the plan gave it, verified against the rows fusion pulled, so ranks one
    # through `"rank"` were read and nothing below was looked at;
    # `"row_bound_reached"` (with `"rank"`: int) is the producer stopping at the row
    # count IT declared it can serve per invocation — it takes its depth as an
    # argument, the depth already sat on that declaration, so the row past it could
    # not be asked for and whether one exists was NOT observable, which is why this
    # is not `"exhausted"`;
    # `"ceiling_reached"` (with `"bound"`: an exact decimal `str`) is a
    # contribution bound, every row at or above it read and the rows below not —
    # what a fusion the caller's `top_k` stopped writes over the streams it
    # stopped; `"supplied_query_ended"` (with `"rank"`: int) is a unit running a
    # query text the host wrote rather than one this layer rendered — the layer
    # bounds only the outside of such a text, so what that text bounded inside
    # itself, and therefore what it left unread, was not observable either;
    # `"execution_failed"` (with `"reason"`: str) is a producer that
    # could not run at all; and `"terms_rejected"` is one that declined the
    # request terms it was handed. "Answered with nothing" and "could not answer"
    # stay distinguishable, because none of the seven is reduced to a flag.
    #
    # Four of the seven can come out of THIS surface: `"exhausted"`,
    # `"depth_reached"`, `"ceiling_reached"` and `"row_bound_reached"` — the last
    # from a vector producer, which takes its depth as its own neighbour count, so
    # a read whose depth sits on the count it declared ends there unobserved. The
    # other three belong to bundles or producers this module does not build —
    # `"supplied_query_ended"` needs a unit carrying a query text a caller wrote
    # and this surface compiles every unit it runs, `"terms_rejected"` is a
    # receipt a producer writes for itself, and `"execution_failed"` needs a unit
    # whose text could not be prepared or run — so they are reachable for a host
    # driving the Rust surface with a bundle of its own. They are spelled and mapped here
    # regardless: the mapping is what makes a status a host DOES receive readable,
    # and the seven-way vocabulary is the engine's, not this binding's.
    #
    # `"domains"` maps a stratum to the candidate-domain declaration its stream
    # fused under — `None` for the unrestricted promise, a list of tag IRIs for a
    # restriction — and `"exclusion_bases"` maps it to what that stream declared
    # its exclusion answers would be a fact about: `"unavailable"` or
    # `"membership"`. The two answer one question by two means: a declaration says
    # a stream will never name a candidate, a lookup observes it. Zero
    # `"exclusion_lookups"` means two different things in general — nothing COULD
    # be asked, or nothing NEEDED asking, since the frontier asks only about
    # candidates a verdict could change the fate of — and `"exclusion_bases"` is
    # what tells them apart. Every relation this surface builds declares
    # `"membership"`, so on an answer from here a zero is always the second.
    #
    # `"attestations"` maps a stratum to what the index behind its stream
    # attested, as `{"generation": str | None, "incomplete": str | None}`, read
    # at the instant that stream was opened — so a stratum a bounded read later
    # stopped still reports both facts. The two axes are independent and each is
    # independently absent, and an absence is an ABSENCE: `None` under
    # `"generation"` is "this producer declared no generation", and `None` under
    # `"incomplete"` is "this producer said nothing about whether its index was
    # whole". The second is specifically NOT a claim that the index WAS whole.
    # There is no value here that could carry such a claim: a producer stopped at
    # the engine's row ceiling never looked at the rows it was licensed to skip,
    # so it could not certify wholeness even if it were asked, and the seam
    # therefore asks only the narrower question that has an honest answer on every
    # path — was your index NOT whole? Reading the silence as certification would
    # put a claim in the mouth of every producer that never spoke. Only the
    # streams fusion was handed are keyed; a stratum that never became a stream is
    # absent rather than reported as having declined to answer.
    #
    # Either axis may be the HOST's word rather than the relation's, through the
    # fifth position of that producer's `_TextProducerSpec` or the attestation
    # position of a `_HnswProducerSpec` / `_KnnProducerSpec`. That is the only way
    # an incompleteness reaches this map at all: every shipped relation indexes
    # what it was handed in this call and has no way to know what was missing from
    # it, so a host whose corpus was assembled from a partial index is the only
    # party who can say so. A declared generation replaces the content digest the
    # relation would otherwise attest; a declared incompleteness is added beside it
    # and leaves it alone.
    #
    # `"fidelities"` maps a stratum to what its producer declared about the rows
    # it can name, on two independent axes. `"completeness"` is `"complete"` or
    # `"lossy"`; `"order"` is `"faithful"` or `"perturbed"`. Where an axis is
    # degraded, `"completeness_evidence"` / `"order_evidence"` carries that
    # producer's OWN words for it, verbatim — never parsed here, never re-worded.
    # The evidence key is absent, not `None`, when the axis is not degraded:
    # silence is the thing this surface exists to stop a caller interpreting.
    #
    # It is read WITH `"statuses"`, never instead of it. A status says how the
    # read ENDED; a fidelity says whether the rows that ended it were all the
    # rows that were DUE. `"exhausted"` beside a `"lossy"` declaration is neither
    # a contradiction nor a completeness claim: the producer emitted every row
    # its search produced, and the declaration says the search does not produce
    # every row there was.
    #
    # `"exactness"` is `{"exact": bool, "deficit": list[str],
    # "inflation": list[str], "unbounded": list[str]}`, derived from those
    # declarations and attestations alone and therefore unmoved by how deep this
    # call read. `True` says no stratum in this fusion declared itself degraded —
    # the narrow true thing, not a certificate that every index was whole and
    # every search exhaustive.
    #
    # When `"exact"` is `False`, every score is an ESTIMATE rather than a value,
    # and the error runs in BOTH directions — which is why there is no
    # `"lower_bounds_for"` key. Fusion scores by RANK and nothing else, so a
    # stratum that fails to name a row does not merely withhold that row's
    # contribution: every row behind the missing one moves up a rank and collects
    # a larger one than it earned. A candidate the degraded stratum missed is
    # summed too LOW; one it named is summed too HIGH. `"deficit"` and
    # `"inflation"` name the strata responsible on each side, and `"unbounded"`
    # names strata whose declared ORDER is perturbed, for which no finite bound
    # exists at all. The rows are still real rows in this fusion's own certified
    # order; what does NOT follow is that a row absent from the answer would have
    # stayed absent.
    #
    # Each row's `"interval"` carries the size of its own error: `{"bounded":
    # True, "deficit": str, "inflation": str}` in the same fixed-point lexical as
    # `"score"`, or `{"bounded": False, "perturbed": list[str]}` where no finite
    # bound exists. `"certain_prefix"` is how many LEADING rows keep their places
    # whatever the degraded strata did or did not find — the answer a caller with
    # a completeness obligation actually has, since without it the only safe move
    # is to downgrade the whole answer. It claims membership and never absence: a
    # row PAST the prefix is possible rather than excluded.
    #
    # `"unemitted_ceiling"` is the evidence that verdict rests on, in the same
    # fixed-point lexical as `"score"`: the most any candidate outside the answer
    # could be worth, counting both the candidates no stream ever named and the
    # ones a bounded read abandoned. A leading row is certain exactly when its own
    # floor clears it, which is what lets the prefix speak about candidates no
    # producer named -- a bound over only the rows in hand would be a claim about
    # the ranking rather than about the answer. It is `None` where a stratum
    # declared a perturbed order, because that breaks the one inequality every
    # bound here rests on and no finite ceiling exists; `"certain_prefix"` is then
    # `0`.
    #
    # `"domains"` maps a stratum to the candidate-domain declaration its stream
    # fused under: `None` where the producer promised only that it may name
    # anything, and a sorted list of tag IRIs where it restricted itself. It is on
    # the answer because it is an input the answer cannot otherwise be audited
    # against — these declarations decide which streams fusion was allowed to skip
    # when it certified a row, so a reader asking why a stratum stopped at a bound
    # instead of being read to its end is asking about this map. An answer whose
    # every entry is `None` was certified with no licence to skip anything.
    #
    # `"evidence_id"` is the content identity of `"attestations"` and of which
    # strata answered the exclusion lookups the rows were certified on, rendered
    # exactly like `"plan_id"` and `"profile_id"`: 64 lowercase hex characters. It
    # is the third of the three identities an answer carries — the plan pins the
    # question, the profile pins the law, and this pins the index generations that
    # answered — so one comparison over the triple decides whether two answers are
    # comparable at all. Nothing else on the answer can show the difference: a
    # rebuilt index moves neither the dataset passed in, nor the request, nor the
    # registry fingerprint.
    @staticmethod
    def search(
        data: str,
        request: list[tuple[builtins.object, ...]],
        *,
        text_producers: dict[str, _TextProducerSpec],
        weights: dict[str, int],
        statistics: dict[str, builtins.object],
        k: int,
        decay: str,
        top_k: int,
        hnsw_producers: dict[str, _HnswProducerSpec] | None = None,
        knn_producers: dict[str, _KnnProducerSpec] | None = None,
        data_format: str = "turtle",
        base: str | None = None,
    ) -> dict[str, builtins.object]: ...
    # The smallest stratum weight, in raw fixed-point units, that still separates
    # every adjacent pair of ranks up to `depth` under the rule `decay` names.
    #
    # The profile-design calculus read in the direction an author needs: name the
    # depth you must read to, get the weight that buys it. The answer is the true
    # minimum rather than a safe over-estimate, because weights are read as
    # ratios and an over-estimate would silently re-scale that stratum's share of
    # every fused score.
    #
    # Raises `ValueError` for five distinct reasons and the message says which:
    # an unknown `decay` spelling; a `k` of zero, which describes no law and is
    # refused before `depth` is read, so a `depth` of one is refused rather than
    # priced; a `depth` of zero, which names no rank to separate; a `depth` past
    # `2 ** 32 - 1`, which is deeper than a plan can record and is the only one
    # of the two walls "weighted_reciprocal_rank" ever meets; and a depth no
    # weight reaches, which only "reciprocal_rank" raises because it rounds the
    # reciprocal before the weight lands. That last message names the exact
    # depth it does reach — the deepest any weight reaches, not the depth of one
    # particular weight — and its remedy is reachable from this same call: ask
    # again under "weighted_reciprocal_rank". A `k` of zero is deliberately not
    # told that way, because switching rules does not make it usable.
    @staticmethod
    def weight_for_depth(depth: int, k: int, *, decay: str) -> int: ...
    # How many consecutive ranks around `rank` a weight of `weight_raw` cannot
    # tell apart under the rule `decay` names.
    #
    # One means the rank is still separated from both neighbours by score alone;
    # `w` means `w` consecutive ranks share a contribution and their order falls
    # through to the declared tie-break. This is the resolution curve, of which
    # `weight_for_depth` prices a single point, and the curve belongs to the
    # rule — the two answer differently at the same weight and rank.
    #
    # `None` means the class is still running at the deepest rank a plan can
    # express, exactly as `deepest_rank_within_width` renders its own saturation.
    # A plan records a per-stratum depth as a 32-bit rank, so there is no end
    # inside its reach to count to, and `2 ** 32 - 1` would be the search's
    # ceiling wearing a width's shape: under `"reciprocal_rank"` a raw weight of
    # one truncates every contribution to zero, so rank one's class is the whole
    # expressible range, and at a raw weight of fifty — fifty times heavier — it
    # still is. An `int` there would say those two classes are the same size.
    #
    # No stratum is taken, because a width is a property of the rule, its
    # smoothing constant, the weight and the rank and of nothing else.
    # `weight_raw` is in raw fixed-point units, where `SCALE` is one whole unit.
    #
    # An operand the law cannot evaluate raises `ValueError` rather than
    # returning a width: a rank of zero, a smoothing constant of zero, an unknown
    # `decay` spelling, or a weight that is not strictly positive.
    @staticmethod
    def class_width(
        weight_raw: int, k: int, rank: int, *, decay: str
    ) -> int | None: ...
    # The deepest depth that can be read with every rank in it sitting in a
    # class no wider than `max_width`, for a weight of `weight_raw` under the
    # rule `decay` names.
    #
    # This inverts `class_width`: name the tolerance you can live with, get the
    # depth it buys. `max_width` of one is the separating depth itself — the
    # deepest depth a read can stop at with every rank it *actually read*
    # separated from both of its neighbours within that read. It is a depth, not
    # a rank property: `class_width` at that rank never reports the one a "still
    # separated from both neighbours" reading would predict, because the
    # unbounded curve it walks also looks at the one rank the bounded read never
    # reaches. Where it counts a width at all that width is at least
    # `max_width + 1`, and it is exactly `max_width + 1` only where the run that
    # ends the walk is one rank longer than the tolerance — the smooth case, not
    # the rule. A light weight is where the difference shows: under
    # `"reciprocal_rank"` with `k` of 60 and a raw weight of `10 ** 2`, a
    # tolerance of one lands on depth one, whose class is forty ranks wide.
    # Where that run reaches the end of the expressible range `class_width` is
    # `None` there, having no width to compare. The two agree by answering a
    # depth question and a rank question. The curve belongs to the rule — the
    # two answer differently at the same weight and tolerance.
    #
    # `None` means no depth a plan can express ever exceeds the tolerance,
    # exactly as `"separates_to"` is `None` on a `search` answer for a law that
    # never stops separating. A plan records a per-stratum depth as a 32-bit
    # rank, so there is no bound inside its reach to report, and `2 ** 32 - 1`
    # would be a saturation point wearing a measurement's shape: two weights
    # fifty times apart both land there.
    #
    # No stratum is taken, because the answer is a property of the rule, its
    # smoothing constant, the weight and the tolerance and of nothing else.
    # `weight_raw` is in raw fixed-point units, where `SCALE` is one whole unit.
    #
    # An operand the law cannot evaluate raises `ValueError` rather than
    # returning a depth: a smoothing constant of zero, a `max_width` of zero —
    # a class always contains its own rank, so a tolerance of zero is not a
    # tolerance — an unknown `decay` spelling, or a weight that is not strictly
    # positive. The constant and the tolerance are both checked before anything
    # is measured, so neither refusal depends on the other argument; a tolerance
    # of one does no walking, and letting it answer where a larger tolerance
    # refuses would make one unusable rule usable or not according to the
    # question asked of it.
    #
    # The answer is walked rank by rank — the class width is not monotone in the
    # rank, so bisecting it would silently over-report — from the separating
    # depth `weight_for_depth` prices, not from rank one. It lands near
    # `sqrt(max_width)` times that depth, so the walk is about
    # `sqrt(max_width) - 1` times it: a `max_width` of one does not walk at all
    # and a small tolerance is cheap, while a large tolerance at a heavy weight
    # under `"weighted_reciprocal_rank"` walks very far. The walk stops at the
    # deepest depth a plan can record rather than running on — reporting `None`
    # there — so it is bounded at fewer than `2**32` steps and always
    # terminates, and the GIL is held throughout. It is a design-time query, not
    # a hot-loop one.
    @staticmethod
    def deepest_rank_within_width(
        weight_raw: int, k: int, max_width: int, *, decay: str
    ) -> int | None: ...
    # The head rank at which a candidate that has collected `naming_raw`'s
    # contributions, at rank `at_rank` in each of them, first beats the threshold
    # `sharing_raw`'s strata impose, under the rule `decay` names.
    #
    # The second half of the fused emission gate: while any stream is open a
    # candidate is emittable only when its lower bound is STRICTLY above the
    # threshold. `naming_raw` are the weights of the strata that named it;
    # `sharing_raw` are the weights of the strata whose declarations admit its
    # block, which is exactly what `"sharing_weights"` on a `compile` answer
    # reports per stratum. A candidate every sharer named crosses at rank one; a
    # candidate one of two equal sharers named is measured against twice its own
    # weight, so under "reciprocal_rank" the head must outlast the smoothing
    # constant.
    #
    # This is the plan-time question: it opens no store and reads no row, so a
    # host learns what depth its OWN declarations have committed it to before it
    # pays for anything. It bounds the threshold gate only — a candidate is also
    # withheld while a stream that could still name it is open, and a
    # configuration where that never resolves reads past this rank regardless.
    #
    # `None` when no rank a plan can express brings the threshold below the
    # bound, rendered as a wall and never as an enormous rank. Weights are raw
    # fixed-point units (`SCALE` is one whole unit); a non-positive weight, a `k`
    # of zero, a rank of zero or an unknown `decay` raise `ValueError`.
    @staticmethod
    def crossing_rank_at(
        naming_raw: list[int],
        at_rank: int,
        sharing_raw: list[int],
        k: int,
        *,
        decay: str,
    ) -> int | None: ...
