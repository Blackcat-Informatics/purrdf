#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Vendor the reviewed offline RDF-carrier slice of the JSON-LD 1.1 REC suite."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

REVISION = "3e7fa5377b2b3c5176eacf8bde8e01fdb7c4a062"
TAG = "REC-2020-07-16"

# All positive, local, default-option core to-RDF cases that do not require an
# implicit document URL (0001-0036 and 0113-0132), followed by the reviewed
# JSON-LD 1.1 construct cases used by PurRDF's RDF-carrier contract. 0016-0018
# are document-URL-relative, 0021 is absent upstream, 0118 requests generalized
# RDF, 0124 is the RFC3986 algorithm stress matrix rather than a carrier case,
# and 0125 requires a manifest-supplied document base.
TO_RDF_IDS = (
    "0001 0002 0003 0004 0005 0006 0007 0008 0009 0010 0011 0012 0013 0014 0015 "
    "0019 0020 0022 0023 0024 0025 0026 0027 0028 0029 0030 0031 0032 0033 0034 "
    "0035 0036 0113 0114 0115 0116 0117 0119 0120 0121 0122 0123 0126 0127 0128 "
    "0129 0130 0131 0132 c001 c002 c004 c005 c035 c036 di07 e030 e034 e037 e079 "
    "e080 e085 e086 e099 e100 js06 js07 m001 m006 m009 n001 pi11 pr10"
).split()

# Exact official compaction oracles whose expanded inputs are bijective RDF
# carrier documents. 0024 and la01 specifically exercise common list
# language/type selection; RDF 1.2 and graph-index extensions live in the
# first-party lens suite because the REC predates RDF 1.2 and standard @index
# metadata is deliberately non-RDF.
COMPACTION_IDS = (
    "0005 0006 0013 0022 0024 0025 0073 di04 di05 di06 la01 m011 p002"
).split()


def digest(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


def cases_by_id(manifest: dict[str, object]) -> dict[str, dict[str, object]]:
    cases: dict[str, dict[str, object]] = {}
    for case in manifest["sequence"]:  # type: ignore[index]
        identifier = case["@id"].removeprefix("#t")
        cases.setdefault(identifier, case)
    return cases


def read_text(tests: Path, relative: object) -> str:
    return (tests / str(relative)).read_text(encoding="utf-8")


def write_json(output: Path, document: object) -> None:
    with output.open("w", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(document, indent=2, ensure_ascii=False) + "\n")


def vendor_to_rdf(tests: Path, output: Path) -> None:
    manifest = json.loads((tests / "toRdf-manifest.jsonld").read_text(encoding="utf-8"))
    cases = cases_by_id(manifest)
    vectors = []
    for identifier in TO_RDF_IDS:
        case = cases[identifier]
        input_text = read_text(tests, case["input"])
        expected = read_text(tests, case["expect"])
        vectors.append(
            {
                "id": identifier,
                "name": case["name"],
                "purpose": case["purpose"],
                "input_sha256": digest(input_text),
                "expected_sha256": digest(expected),
                "input": input_text,
                "expected_nquads": expected,
            }
        )
    document = {
        "schema_version": 1,
        "upstream_revision": REVISION,
        "upstream_tag": TAG,
        "expected_vector_count": len(vectors),
        "vectors": vectors,
    }
    write_json(output, document)


def vendor_compaction(tests: Path, output: Path) -> None:
    manifest = json.loads((tests / "compact-manifest.jsonld").read_text(encoding="utf-8"))
    cases = cases_by_id(manifest)
    vectors = []
    for identifier in COMPACTION_IDS:
        case = cases[identifier]
        input_text = read_text(tests, case["input"])
        context = read_text(tests, case["context"])
        expected = read_text(tests, case["expect"])
        vectors.append(
            {
                "id": identifier,
                "name": case["name"],
                "purpose": case["purpose"],
                "input_sha256": digest(input_text),
                "context_sha256": digest(context),
                "expected_sha256": digest(expected),
                "input": input_text,
                "context": context,
                "expected": expected,
            }
        )
    document = {
        "schema_version": 1,
        "upstream_revision": REVISION,
        "upstream_tag": TAG,
        "expected_vector_count": len(vectors),
        "vectors": vectors,
    }
    write_json(output, document)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("upstream", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    tests = args.upstream / "tests"
    args.output.mkdir(parents=True, exist_ok=True)
    vendor_to_rdf(tests, args.output / "vectors.json")
    vendor_compaction(tests, args.output / "compaction_vectors.json")


if __name__ == "__main__":
    main()
