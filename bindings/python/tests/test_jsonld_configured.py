# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Configured JSON-LD/YAML-LD Python surfaces use the shared Rust context lens."""

from __future__ import annotations

import json
from types import ModuleType

import pytest

import purrdf


SOURCE = b'<https://example.org/alice> <https://schema.org/name> "Alice" .\n'
PREFIX_OPTIONS = json.dumps(
    {
        "version": 1,
        "mode": "context",
        "prefixes": {
            "ex": "https://example.org/",
            "schema": "https://schema.org/",
        },
    },
    sort_keys=True,
    separators=(",", ":"),
)


def _assert_compacted(text: str) -> None:
    document = json.loads(text)
    assert document["@context"] == {
        "ex": {"@id": "https://example.org/", "@prefix": True},
        "schema": {"@id": "https://schema.org/", "@prefix": True},
    }
    assert document["@graph"][0]["@id"] == "ex:alice"
    assert document["@graph"][0]["schema:name"] == {"@value": "Alice"}


def test_direct_serializer_options_and_compiled_context_are_byte_identical() -> None:
    """The versioned decoder and reusable handle reach one deterministic engine."""
    direct = purrdf.serialize_jsonld(
        SOURCE,
        format=purrdf.RdfFormat.N_QUADS,
        output_format="jsonld",
        options_json=PREFIX_OPTIONS,
    )
    context = purrdf.CompiledJsonLdContext(PREFIX_OPTIONS)
    reused = purrdf.serialize_jsonld(
        SOURCE,
        format=purrdf.RdfFormat.N_QUADS,
        output_format="application/ld+json",
        context=context,
    )
    assert direct == reused
    _assert_compacted(direct)
    assert json.loads(context.canonical_context_json()) == json.loads(direct)["@context"]


def test_yaml_schema_and_immutable_dataset_surface() -> None:
    """YAML-LD keeps the configured schema header and compiled-context bytes."""
    context = purrdf.CompiledJsonLdContext.from_prefixes(
        {"ex": "https://example.org/", "schema": "https://schema.org/"}
    )
    dataset = purrdf.RdfDataset(SOURCE, purrdf.RdfFormat.N_QUADS)
    rendered = dataset.serialize_jsonld(
        "yamlld",
        context=context,
        yaml_schema_url="https://example.org/purrdf.schema.json",
    )
    assert rendered.startswith(
        "# yaml-language-server: $schema=https://example.org/purrdf.schema.json\n"
    )
    assert "ex:alice" in rendered
    assert "schema:name" in rendered


@pytest.mark.parametrize("store_type", [purrdf.Store, purrdf.MutableDataset])
def test_store_surfaces_accept_reusable_context(store_type: type[object]) -> None:
    """Both mutable store implementations accept the same immutable handle."""
    store = store_type()
    store.load(SOURCE, format=purrdf.RdfFormat.N_QUADS)
    context = purrdf.CompiledJsonLdContext(PREFIX_OPTIONS)
    rendered = store.dump(
        format=purrdf.RdfFormat.JSON_LD,
        jsonld_context=context,
    )
    assert isinstance(rendered, bytes)
    _assert_compacted(rendered.decode())


def test_rdflib_graph_bound_prefixes_are_an_explicit_context(
    compat: ModuleType,
) -> None:
    """Caller-bound namespaces compact output without changing default bytes."""
    graph = compat.Graph()
    graph.parse(data=SOURCE, format="nquads")
    graph.bind("ex", compat.Namespace("https://example.org/"))
    graph.bind("schema", compat.Namespace("https://schema.org/"))
    _assert_compacted(graph.serialize(format="json-ld"))


def test_configured_inputs_are_closed_and_mutually_exclusive() -> None:
    """Malformed, unknown, and conflicting inputs hard-fail before output."""
    context = purrdf.CompiledJsonLdContext(PREFIX_OPTIONS)
    with pytest.raises(ValueError, match="exactly one"):
        purrdf.serialize_jsonld(
            SOURCE,
            format=purrdf.RdfFormat.N_QUADS,
            output_format="jsonld",
            options_json=PREFIX_OPTIONS,
            context=context,
        )
    with pytest.raises(ValueError, match="unknown"):
        purrdf.serialize_jsonld(
            SOURCE,
            format=purrdf.RdfFormat.N_QUADS,
            output_format="jsonld",
            options_json='{"version":1,"mode":"expanded","unknown":true}',
        )
    with pytest.raises(ValueError, match="requires JSON-LD options"):
        purrdf.CompiledJsonLdContext('{"version":1,"mode":"derived"}')
