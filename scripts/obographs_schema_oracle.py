#!/usr/bin/env -S uv run --script
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
# /// script
# requires-python = ">=3.11"
# dependencies = ["jsonschema==4.26.0"]
# ///

"""Validate production OBO Graphs output against the pinned official schema."""

from __future__ import annotations

import copy
import hashlib
import json
import pathlib
import sys

from jsonschema import Draft3Validator


SCHEMA_HASHES = {
    "obographs-schema.json": (
        "a54a564ff47014b626003e54e6a61392b65be7301939d878cc658d4186e88165"
    ),
    "subschemas/obographs-graph-schema.json": (
        "60668223ff53ff9fca59dfc62cb7abada60ba977da8f1ddaf2772631efec528d"
    ),
    "subschemas/obographs-meta-schema.json": (
        "0a1c6e1b0b446fc6a4515eddc4bf4767156097caf9d7b4c5f1d1ca733e3696a6"
    ),
}


def verify_schema_closure(root: pathlib.Path) -> None:
    for relative, expected in SCHEMA_HASHES.items():
        actual = hashlib.sha256((root / relative).read_bytes()).hexdigest()
        if actual != expected:
            raise AssertionError(
                f"pinned OBO Graphs schema drift for {relative}: {actual} != {expected}"
            )


def validation_errors(validator: Draft3Validator, document: object) -> list[str]:
    return [
        error.message
        for error in sorted(validator.iter_errors(document), key=lambda item: list(item.path))
    ]


def main() -> int:
    if len(sys.argv) != 2:
        raise SystemExit("usage: obographs_schema_oracle.py PRODUCTION_JSON")
    document_path = pathlib.Path(sys.argv[1])
    schema_root = (
        pathlib.Path(__file__).resolve().parents[1]
        / "crates/rdf/tests/fixtures/obographs-0.3.2"
    )
    verify_schema_closure(schema_root)
    schema = json.loads((schema_root / "obographs-schema.json").read_text("utf-8"))
    Draft3Validator.check_schema(schema)
    validator = Draft3Validator(schema)

    document = json.loads(document_path.read_text("utf-8"))
    errors = validation_errors(validator, document)
    if errors:
        raise AssertionError(f"official OBO Graphs 0.3.2 schema rejected output: {errors}")

    graphs = document.get("graphs")
    if not isinstance(graphs, list) or len(graphs) != 1:
        raise AssertionError("production fixture must contain exactly one graph")
    graph = graphs[0]
    for field in (
        "nodes",
        "edges",
        "equivalentNodesSets",
        "logicalDefinitionAxioms",
        "domainRangeAxioms",
        "propertyChainAxioms",
    ):
        if not graph.get(field):
            raise AssertionError(f"production fixture did not exercise {field}")

    bad_node_type = copy.deepcopy(document)
    bad_node_type["graphs"][0]["nodes"][0]["type"] = "NOT_A_NODE_TYPE"
    if not validation_errors(validator, bad_node_type):
        raise AssertionError("official schema accepted a deliberately invalid node type")

    bad_chain = copy.deepcopy(document)
    bad_chain["graphs"][0]["propertyChainAxioms"][0]["chainPredicateIds"][0] = 7
    if not validation_errors(validator, bad_chain):
        raise AssertionError("official schema accepted a deliberately invalid property chain")

    print(
        "OK: jsonschema==4.26.0 validated deterministic PurRDF output against "
        "the pinned OBO Graphs 0.3.2 schema and rejected corruptions"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
