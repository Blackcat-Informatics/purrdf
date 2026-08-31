# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""A quad-template CONSTRUCT keeps its graph names all the way into Python.

SPARQL 1.2 lets a CONSTRUCT template name a graph per statement, so one result may
write several named graphs and may mix them with default-graph triples. `Triple` has
no graph slot, so the Python egress splits by what the result CARRIES:

* every statement in the default graph → `QueryTriples`, unchanged (this is every
  SPARQL 1.1 CONSTRUCT, and every DESCRIBE over default-graph data — the
  backward-compatibility pin below);
* any statement in a named graph → `QueryQuads`, whose members carry `graph_name` and
  whose `serialize` round-trips through every quad-capable syntax.

A DESCRIBE reaches the same split from the other direction: it has no template to name
a graph with, but the Symmetric CBD keeps every layer — base quad, reifier declaration
and annotation — in the graph that asserted it, so a description over graph-scoped data
carries graph names too.

Asking a `QueryQuads` for a single-graph syntax raises rather than dropping the
graphs, mirroring the `purrdf query` CLI refusal: silently emitting a well-formed
document that is missing exactly what the query asked for is the failure this closes.
"""

from __future__ import annotations

import io

import purrdf
import pytest

EX = "https://example.org/"

# One default-graph triple to build every fixture query's WHERE clause from.
_SEED = (f"<{EX}s> <{EX}p> <{EX}o> .\n<{EX}s2> <{EX}p> <{EX}o2> .\n").encode()

# Every RdfFormat that can carry a graph name, and the two that cannot.
_QUAD_CAPABLE = [
    purrdf.RdfFormat.N_QUADS,
    purrdf.RdfFormat.TRIG,
    purrdf.RdfFormat.TRIX,
    purrdf.RdfFormat.HEXTUPLES,
    purrdf.RdfFormat.JSON_LD,
    purrdf.RdfFormat.YAML_LD,
]
_SINGLE_GRAPH = [purrdf.RdfFormat.TURTLE, purrdf.RdfFormat.N_TRIPLES]


def _store() -> purrdf.Store:
    """A store holding the two seed triples in the default graph."""
    store = purrdf.Store()
    store.load(_SEED, format=purrdf.RdfFormat.N_TRIPLES)
    return store


def _mutable() -> purrdf.MutableDataset:
    """The same seed, in the other query-bearing entry point."""
    dataset = purrdf.MutableDataset()
    dataset.load(_SEED, format=purrdf.RdfFormat.N_TRIPLES)
    return dataset


def _nquad_lines(payload: bytes) -> set[str]:
    """The N-Quads document's statements as a set of normalized lines."""
    return {line.strip() for line in payload.decode().splitlines() if line.strip()}


# ── the graph term survives into a quad-capable serialization ─────────────────────


def test_construct_graph_result_is_query_quads() -> None:
    """A `CONSTRUCT GRAPH` result is a `QueryQuads`, not a `QueryTriples`."""
    result = _store().query(
        f"CONSTRUCT {{ GRAPH <{EX}g> {{ ?s ?p ?o }} }} WHERE {{ ?s ?p ?o }}"
    )
    assert isinstance(result, purrdf.QueryQuads)
    assert not isinstance(result, purrdf.QueryTriples)
    assert len(result) == 2


def test_construct_graph_term_survives_into_nquads() -> None:
    """The graph the query named is present in the N-Quads output."""
    result = _store().query(
        f"CONSTRUCT {{ GRAPH <{EX}g> {{ ?s ?p ?o }} }} WHERE {{ ?s ?p ?o }}"
    )
    assert _nquad_lines(result.serialize(purrdf.RdfFormat.N_QUADS)) == {
        f"<{EX}s> <{EX}p> <{EX}o> <{EX}g> .",
        f"<{EX}s2> <{EX}p> <{EX}o2> <{EX}g> .",
    }


def test_construct_graph_quads_carry_graph_name() -> None:
    """Iterating the result yields `Quad`s whose `graph_name` is the named graph."""
    result = _store().query(
        f"CONSTRUCT {{ GRAPH <{EX}g> {{ ?s ?p ?o }} }} WHERE {{ ?s ?p ?o }}"
    )
    quads = list(result)
    assert len(quads) == 2
    assert {str(quad.graph_name) for quad in quads} == {f"<{EX}g>"}


@pytest.mark.parametrize("format", _QUAD_CAPABLE)
def test_every_quad_capable_format_round_trips_the_graph(
    format: purrdf.RdfFormat,
) -> None:
    """Each quad-capable syntax carries the graph name back out through `parse`.

    Re-parsed rather than string-matched: TriX, HexTuples, JSON-LD and YAML-LD all
    spell a graph name differently, and the claim under test is that the graph
    SURVIVES, not that any particular byte sequence appears.
    """
    result = _store().query(
        f"CONSTRUCT {{ GRAPH <{EX}g> {{ ?s ?p ?o }} }} WHERE {{ ?s ?p ?o }}"
    )
    reparsed = purrdf.parse(result.serialize(format), format)
    assert {str(quad.graph_name) for quad in reparsed} == {f"<{EX}g>"}


def test_module_level_serialize_accepts_query_quads() -> None:
    """`purrdf.serialize` takes a `QueryQuads` and writes the graph out."""
    result = _store().query(
        f"CONSTRUCT {{ GRAPH <{EX}g> {{ ?s ?p ?o }} }} WHERE {{ ?s ?p ?o }}"
    )
    sink = io.BytesIO()
    assert (
        purrdf.serialize(result, sink, purrdf.RdfFormat.N_QUADS)  # type: ignore[func-returns-value]
        is None
    )
    assert f"<{EX}g>" in sink.getvalue().decode()


# ── several graphs, and mixed default + named ────────────────────────────────────


def test_multi_graph_construct_keeps_every_graph() -> None:
    """A template writing two graphs yields both, each with its own statements."""
    result = _store().query(
        f"CONSTRUCT {{ GRAPH <{EX}g1> {{ ?s ?p ?o }} GRAPH <{EX}g2> {{ ?o ?p ?s }} }} "
        f"WHERE {{ ?s ?p ?o }}"
    )
    assert isinstance(result, purrdf.QueryQuads)
    assert result.graph_names == [f"<{EX}g1>", f"<{EX}g2>"]
    assert _nquad_lines(result.serialize(purrdf.RdfFormat.N_QUADS)) == {
        f"<{EX}s> <{EX}p> <{EX}o> <{EX}g1> .",
        f"<{EX}s2> <{EX}p> <{EX}o2> <{EX}g1> .",
        f"<{EX}o> <{EX}p> <{EX}s> <{EX}g2> .",
        f"<{EX}o2> <{EX}p> <{EX}s2> <{EX}g2> .",
    }


def test_mixed_default_and_named_construct_keeps_both_halves() -> None:
    """Default-graph triples and named-graph quads survive in one result.

    The case that makes refusal (rather than folding) the only honest answer for a
    single-graph syntax: flattening would emit the default-graph half and drop the
    rest, reporting a partial answer as a complete one.
    """
    result = _store().query(
        f"CONSTRUCT {{ ?s <{EX}q> ?o . GRAPH <{EX}g> {{ ?s ?p ?o }} }} "
        f"WHERE {{ ?s ?p ?o }}"
    )
    assert isinstance(result, purrdf.QueryQuads)
    assert result.graph_names == [f"<{EX}g>"]
    assert _nquad_lines(result.serialize(purrdf.RdfFormat.N_QUADS)) == {
        f"<{EX}s> <{EX}q> <{EX}o> .",
        f"<{EX}s2> <{EX}q> <{EX}o2> .",
        f"<{EX}s> <{EX}p> <{EX}o> <{EX}g> .",
        f"<{EX}s2> <{EX}p> <{EX}o2> <{EX}g> .",
    }


def test_graph_variable_binds_one_graph_per_solution() -> None:
    """A graph VARIABLE writes as many graphs as the WHERE has distinct bindings."""
    store = purrdf.Store()
    store.load(
        (
            f"<{EX}s> <{EX}p> <{EX}o> <{EX}g1> .\n"
            f"<{EX}s2> <{EX}p> <{EX}o2> <{EX}g2> .\n"
        ).encode(),
        format=purrdf.RdfFormat.N_QUADS,
    )
    result = store.query(
        "CONSTRUCT { GRAPH ?g { ?s ?p ?o } } WHERE { GRAPH ?g { ?s ?p ?o } }"
    )
    assert isinstance(result, purrdf.QueryQuads)
    assert result.graph_names == [f"<{EX}g1>", f"<{EX}g2>"]


def test_unbound_graph_variable_yields_query_triples() -> None:
    """A graph slot that never resolves writes only the default graph.

    The discriminator is what the result CARRIES, not what the query's syntax looked
    like: an unbindable `GRAPH ?g` skips its statements per SPARQL §16.2, so the
    remaining result is default-graph-only and stays a `QueryTriples`.
    """
    result = _store().query(
        f"CONSTRUCT {{ ?s <{EX}q> ?o . GRAPH ?g {{ ?s ?p ?o }} }} WHERE {{ ?s ?p ?o }}"
    )
    assert isinstance(result, purrdf.QueryTriples)
    assert len(result) == 2


# ── the loud failure for a single-graph syntax ───────────────────────────────────


@pytest.mark.parametrize("format", _SINGLE_GRAPH)
def test_single_graph_format_refuses_and_names_graph_and_format(
    format: purrdf.RdfFormat,
) -> None:
    """Turtle / N-Triples raise, naming the graph, the format, and the alternatives."""
    result = _store().query(
        f"CONSTRUCT {{ GRAPH <{EX}g> {{ ?s ?p ?o }} }} WHERE {{ ?s ?p ?o }}"
    )
    with pytest.raises(ValueError) as excinfo:
        result.serialize(format)
    message = str(excinfo.value)
    assert f"<{EX}g>" in message
    assert format.__class__.__name__ == "RdfFormat"
    assert ("TURTLE" if format == purrdf.RdfFormat.TURTLE else "N_TRIPLES") in message
    assert "DROPPED" in message
    assert "RdfFormat.N_QUADS/TRIG/TRIX/HEXTUPLES/JSON_LD/YAML_LD" in message


def test_refusal_lists_every_graph_in_lexicographic_order() -> None:
    """A multi-graph refusal names all of them, sorted, with an exact count."""
    result = _store().query(
        f"CONSTRUCT {{ GRAPH <{EX}zeta> {{ ?s ?p ?o }} GRAPH <{EX}alpha> {{ ?o ?p ?s }} }} "
        f"WHERE {{ ?s ?p ?o }}"
    )
    with pytest.raises(ValueError) as excinfo:
        result.serialize(purrdf.RdfFormat.TURTLE)
    message = str(excinfo.value)
    assert "carrying 2 named graphs" in message
    assert f"(<{EX}alpha>, <{EX}zeta>)" in message


def test_refusal_message_is_deterministic_across_runs() -> None:
    """The graph list is a function of the result, not of evaluation order."""
    query = (
        f"CONSTRUCT {{ GRAPH <{EX}g3> {{ ?s ?p ?o }} GRAPH <{EX}g1> {{ ?o ?p ?s }} "
        f"GRAPH <{EX}g2> {{ ?s <{EX}q> ?o }} }} WHERE {{ ?s ?p ?o }}"
    )
    messages = set()
    for _ in range(5):
        result = _store().query(query)
        with pytest.raises(ValueError) as excinfo:
            result.serialize(purrdf.RdfFormat.N_TRIPLES)
        messages.add(str(excinfo.value))
    assert len(messages) == 1


def test_mixed_result_refuses_rather_than_emitting_its_default_half() -> None:
    """A mixed result never yields the default-graph half alone."""
    result = _store().query(
        f"CONSTRUCT {{ ?s <{EX}q> ?o . GRAPH <{EX}g> {{ ?s ?p ?o }} }} "
        f"WHERE {{ ?s ?p ?o }}"
    )
    with pytest.raises(ValueError, match="would be DROPPED"):
        result.serialize(purrdf.RdfFormat.TURTLE)


def test_module_level_serialize_refuses_a_single_graph_format() -> None:
    """`purrdf.serialize` shares the refusal — it is not a way around the method."""
    result = _store().query(
        f"CONSTRUCT {{ GRAPH <{EX}g> {{ ?s ?p ?o }} }} WHERE {{ ?s ?p ?o }}"
    )
    with pytest.raises(ValueError, match="would be DROPPED"):
        purrdf.serialize(result, None, purrdf.RdfFormat.TURTLE)


def test_serialize_rejects_a_non_result_input() -> None:
    """The widened `serialize` input still refuses anything that is not a result."""
    with pytest.raises(TypeError, match="QueryTriples or a QueryQuads"):
        purrdf.serialize("not a result", None, purrdf.RdfFormat.N_TRIPLES)  # type: ignore[arg-type]


# ── the backward-compatibility pin: default-graph-only is untouched ──────────────


def test_default_graph_construct_is_still_query_triples() -> None:
    """A plain CONSTRUCT keeps its exact type — the pin the split must not move."""
    result = _store().query("CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }")
    assert type(result) is purrdf.QueryTriples
    assert len(result) == 2
    assert all(isinstance(triple, purrdf.Triple) for triple in result)


def test_describe_is_still_query_triples() -> None:
    """A DESCRIBE over default-graph data is unaffected by the split."""
    result = _store().query(f"DESCRIBE <{EX}s>")
    assert type(result) is purrdf.QueryTriples


# ── DESCRIBE reaches the same egress, because a description carries graphs ────────

# A graph-scoped RDF 1.2 statement layer: base quad, reifier declaration and
# annotation all asserted in `ex:g`.
_GRAPH_STAR_TRIG = (
    f"@prefix ex: <{EX}> .\n"
    'GRAPH ex:g { ex:s ex:p ex:o ~ex:r {| ex:note "n" |} . }\n'
).encode()

_REIFIES = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"


def _graph_star_store() -> purrdf.Store:
    """A store whose every statement — base, reifier and annotation — is in `ex:g`."""
    store = purrdf.Store()
    store.load(_GRAPH_STAR_TRIG, format=purrdf.RdfFormat.TRIG)
    return store


def test_describe_over_named_graphs_is_query_quads() -> None:
    """A DESCRIBE result carrying named graphs takes the SAME widening a CONSTRUCT does.

    No DESCRIBE names a graph — there is no template to name one in — but the
    description is graph-faithful at every layer, so it can carry graphs the source
    asserted. `Triple` has no graph slot, so the result must materialize as
    `QueryQuads` or the graphs are lost on the way into Python.
    """
    result = _graph_star_store().query(f"DESCRIBE <{EX}s>")
    assert isinstance(result, purrdf.QueryQuads)
    assert result.graph_names == [f"<{EX}g>"]
    assert _nquad_lines(result.serialize(purrdf.RdfFormat.N_QUADS)) == {
        f"<{EX}s> <{EX}p> <{EX}o> <{EX}g> .",
        f"<{EX}r> <{_REIFIES}> <<( <{EX}s> <{EX}p> <{EX}o> )>> <{EX}g> .",
        f'<{EX}r> <{EX}note> "n" <{EX}g> .',
    }


@pytest.mark.parametrize("format", _SINGLE_GRAPH)
def test_describe_over_named_graphs_refuses_a_single_graph_format(
    format: purrdf.RdfFormat,
) -> None:
    """The refusal covers a DESCRIBE result too, naming the graph it would drop."""
    result = _graph_star_store().query(f"DESCRIBE <{EX}s>")
    with pytest.raises(ValueError) as excinfo:
        result.serialize(format)
    message = str(excinfo.value)
    assert f"<{EX}g>" in message
    assert "DROPPED" in message


@pytest.mark.parametrize("format", _SINGLE_GRAPH + _QUAD_CAPABLE)
def test_default_graph_construct_serializes_on_every_format(
    format: purrdf.RdfFormat,
) -> None:
    """Every format still serializes a default-graph result, including Turtle."""
    result = _store().query("CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }")
    payload = result.serialize(format)
    assert payload
    reparsed = purrdf.parse(payload, format)
    assert {str(quad.subject) for quad in reparsed} == {f"<{EX}s>", f"<{EX}s2>"}
    assert all(quad.graph_name == purrdf.DefaultGraph() for quad in reparsed)


def test_default_graph_construct_bytes_match_a_hand_built_baseline() -> None:
    """The default-graph Turtle bytes are exactly what a direct dump produces.

    The byte-level half of the pin: not merely "Turtle still works" but "Turtle
    produces the same document it always did", proven against a dataset holding the
    same triples rather than against a copied literal.
    """
    result = _store().query("CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }")
    baseline = purrdf.MutableDataset()
    baseline.load(_SEED, format=purrdf.RdfFormat.N_TRIPLES)
    assert result.serialize(purrdf.RdfFormat.TURTLE) == baseline.dump(
        format=purrdf.RdfFormat.TURTLE
    )


# ── every entry point that shares `materialize_results` ──────────────────────────


def test_mutable_dataset_query_returns_query_quads() -> None:
    """`MutableDataset.query` routes through the same adapter as `Store.query`."""
    result = _mutable().query(
        f"CONSTRUCT {{ GRAPH <{EX}g> {{ ?s ?p ?o }} }} WHERE {{ ?s ?p ?o }}"
    )
    assert isinstance(result, purrdf.QueryQuads)
    assert result.graph_names == [f"<{EX}g>"]


def test_mutable_dataset_default_graph_query_is_query_triples() -> None:
    """…and keeps the default-graph result type there too."""
    result = _mutable().query("CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }")
    assert type(result) is purrdf.QueryTriples


def test_store_query_governed_result_is_query_quads() -> None:
    """The governed lane's complete result carries the graphs as well."""
    outcome = _store().query_governed(
        f"CONSTRUCT {{ GRAPH <{EX}g> {{ ?s ?p ?o }} }} WHERE {{ ?s ?p ?o }}"
    )
    assert outcome.is_complete
    assert isinstance(outcome.result, purrdf.QueryQuads)
    assert f"<{EX}g>" in outcome.result.serialize(purrdf.RdfFormat.N_QUADS).decode()


def test_mutable_dataset_query_governed_result_is_query_quads() -> None:
    """…and so does the mutable-dataset governed lane."""
    outcome = _mutable().query_governed(
        f"CONSTRUCT {{ GRAPH <{EX}g> {{ ?s ?p ?o }} }} WHERE {{ ?s ?p ?o }}"
    )
    assert outcome.is_complete
    assert isinstance(outcome.result, purrdf.QueryQuads)


def test_governed_partial_answers_carry_the_graphs() -> None:
    """A tripped governor's partial rows keep their graph names too.

    The partial-answer certificate re-enters the same adapter, so a truncated
    graph-carrying CONSTRUCT must not degrade into a graph-less triple stream — that
    would make the truncated answer wrong in a second, undeclared way.
    """
    outcome = _store().query_governed(
        f"CONSTRUCT {{ GRAPH <{EX}g> {{ ?s ?p ?o }} }} WHERE {{ ?s ?p ?o }}",
        max_answers=1,
    )
    assert not outcome.is_complete
    partial = outcome.partial
    assert partial is not None
    assert isinstance(partial.result, purrdf.QueryQuads)
    assert partial.result.graph_names == [f"<{EX}g>"]


# ── the RDF 1.2 statement layer follows its quad into the named graph ────────────


def test_statement_layer_rows_keep_their_graph() -> None:
    """A reifier declared inside a named graph is re-materialized into that graph.

    The statement layer is keyed per graph, and the flat quad stream must carry that
    key: an `rdf:reifies` edge that came back in the default graph would silently
    unscope the annotation and re-fold into the wrong graph on the next parse.
    """
    store = purrdf.Store()
    store.load(
        (
            f"<{EX}g> {{ <{EX}r> "
            f"<http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> "
            f"<<( <{EX}s> <{EX}p> <{EX}o> )>> . "
            f'<{EX}r> <{EX}certainty> "0.9" . }}\n'
        ).encode(),
        format=purrdf.RdfFormat.TRIG,
    )
    result = store.query(
        "CONSTRUCT { GRAPH ?g { ?s ?p ?o } } WHERE { GRAPH ?g { ?s ?p ?o } }"
    )
    assert isinstance(result, purrdf.QueryQuads)
    assert result.graph_names == [f"<{EX}g>"]
    assert all(str(quad.graph_name) == f"<{EX}g>" for quad in result)


def test_flat_parse_keeps_the_statement_layer_graph() -> None:
    """The same invariant on the `parse` lane, which shares the flattening."""
    quads = purrdf.parse(
        (
            f"<{EX}g> {{ <{EX}r> "
            f"<http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> "
            f"<<( <{EX}s> <{EX}p> <{EX}o> )>> . }}\n"
        ),
        purrdf.RdfFormat.TRIG,
    )
    assert quads
    assert all(str(quad.graph_name) == f"<{EX}g>" for quad in quads)


# ── the rdflib compat shim hands back a Dataset, not a flattened Graph ───────────


def test_compat_graph_query_returns_a_dataset_for_a_quad_template() -> None:
    """`purrdf.compat.rdflib`'s `Graph.query` keeps the graphs in a `Dataset`."""
    from purrdf.compat import rdflib as compat

    graph = compat.Graph()
    graph.add(
        (
            compat.URIRef(f"{EX}s"),
            compat.URIRef(f"{EX}p"),
            compat.URIRef(f"{EX}o"),
        )
    )
    result = graph.query(
        f"CONSTRUCT {{ GRAPH <{EX}g> {{ ?s ?p ?o }} }} WHERE {{ ?s ?p ?o }}"
    )
    assert isinstance(result.graph, compat.Dataset)
    names = {str(context.identifier) for context in result.graph.contexts()}
    assert f"{EX}g" in names


def test_compat_graph_query_still_returns_a_graph_for_a_plain_construct() -> None:
    """The default-graph compat path is unchanged."""
    from purrdf.compat import rdflib as compat

    graph = compat.Graph()
    graph.add(
        (
            compat.URIRef(f"{EX}s"),
            compat.URIRef(f"{EX}p"),
            compat.URIRef(f"{EX}o"),
        )
    )
    result = graph.query("CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }")
    assert isinstance(result.graph, compat.Graph)
    assert not isinstance(result.graph, compat.Dataset)
    assert len(result.graph) == 1
