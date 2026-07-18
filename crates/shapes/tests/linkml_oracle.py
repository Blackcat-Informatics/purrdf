# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Validate emitted LinkML through the official locked 1.11 toolchain."""

from __future__ import annotations

import importlib.metadata
import json
import subprocess
from pathlib import Path
from typing import Any

from jsonschema.validators import validator_for
from linkml.generators.jsonschemagen import JsonSchemaGenerator
from linkml_runtime.linkml_model.meta import SchemaDefinition
from linkml_runtime.loaders import yaml_loader
from linkml_runtime.utils.schemaview import SchemaView


REPO = Path(__file__).resolve().parents[3]
LINKML_PACKAGE_VERSION = "1.11.1"
LINKML_METAMODEL_VERSION = "1.11.0"


def _fixture() -> dict[str, Any]:
    completed = subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "purrdf-shapes",
            "--example",
            "linkml_oracle_fixture",
            "--locked",
            "--quiet",
        ],
        cwd=REPO,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(completed.stdout)


def _load(text: str) -> SchemaDefinition:
    schema = yaml_loader.loads(text, target_class=SchemaDefinition)
    if schema.metamodel_version != LINKML_METAMODEL_VERSION:
        raise AssertionError(
            f"unexpected metamodel version: {schema.metamodel_version!r}"
        )
    return schema


def _generate(schema: SchemaDefinition) -> dict[str, Any]:
    generated = JsonSchemaGenerator(
        schema,
        not_closed=True,
        preserve_names=True,
        useuris=False,
        include_null=False,
        materialize_patterns=True,
    ).serialize()
    value = json.loads(generated)
    if not isinstance(value, dict):
        raise AssertionError("official LinkML JSON Schema generator returned a non-object")
    return value


def _rewrite_refs(value: Any, element_names: dict[str, str]) -> Any:
    if isinstance(value, list):
        return [_rewrite_refs(child, element_names) for child in value]
    if not isinstance(value, dict):
        return value
    rewritten = {
        key: _rewrite_refs(child, element_names) for key, child in value.items()
    }
    reference = value.get("$ref")
    if isinstance(reference, str) and reference.startswith("#/$defs/"):
        key = reference.removeprefix("#/$defs/").replace("~1", "/").replace(
            "~0", "~"
        )
        rewritten["$ref"] = f"#/$defs/{element_names[key]}"
    return rewritten


def _definition_wrapper(document: dict[str, Any], name: str) -> dict[str, Any]:
    return {
        "$schema": document["$schema"],
        "$defs": document["$defs"],
        "$ref": f"#/$defs/{name}",
    }


def _is_valid(document: dict[str, Any], name: str, instance: Any) -> bool:
    wrapper = _definition_wrapper(document, name)
    validator_class = validator_for(wrapper)
    validator_class.check_schema(wrapper)
    return validator_class(wrapper).is_valid(instance)


def _assert_acceptance_agrees(
    source: dict[str, Any],
    generated: dict[str, Any],
    probes: dict[str, list[Any]],
) -> None:
    for name, instances in probes.items():
        for instance in instances:
            expected = _is_valid(source, name, instance)
            actual = _is_valid(generated, name, instance)
            if actual != expected:
                raise AssertionError(
                    f"{name} acceptance differs for {instance!r}: "
                    f"source={expected}, generated={actual}"
                )


def _assert_reference_closure(document: dict[str, Any]) -> None:
    definitions = document.get("$defs")
    if not isinstance(definitions, dict):
        raise AssertionError("generated JSON Schema has no $defs object")

    def walk(value: Any) -> None:
        if isinstance(value, list):
            for child in value:
                walk(child)
            return
        if not isinstance(value, dict):
            return
        reference = value.get("$ref")
        if isinstance(reference, str) and reference.startswith("#/$defs/"):
            target = reference.removeprefix("#/$defs/")
            if target not in definitions:
                raise AssertionError(f"dangling generated reference: {reference!r}")
        for child in value.values():
            walk(child)

    walk(definitions)


def _assert_reverse(payload: dict[str, Any], expected_shape_ids: list[str]) -> None:
    reverse = payload["reverse"]
    if reverse["shape_ids"] != expected_shape_ids:
        raise AssertionError(
            f"unexpected reverse SHACL shapes: {reverse['shape_ids']!r}"
        )
    _assert_reference_closure(reverse["schema"])
    losses = reverse["losses"]["losses"]
    if not all(entry["from"] == "linkml-1.11" for entry in losses):
        raise AssertionError("reverse ledger contains a foreign source format")
    if not all(entry["to"] == "shacl" for entry in losses):
        raise AssertionError("reverse ledger contains a foreign target format")
    if not all(entry["intentional"] for entry in losses):
        raise AssertionError("reverse fixture contains an unregistered loss")
    if not all(" subject=#/" in entry["location"] for entry in losses):
        raise AssertionError("reverse loss was not remapped to a native LinkML path")


def _assert_exact(payload: dict[str, Any]) -> None:
    source = payload["schema"]
    element_names = payload["element_names"]
    schema = _load(payload["yaml"])
    view = SchemaView(schema)
    if sorted(view.all_classes()) != ["Address", "Person"]:
        raise AssertionError(f"unexpected classes: {sorted(view.all_classes())}")
    if sorted(view.all_enums()) != ["Color"]:
        raise AssertionError(f"unexpected enums: {sorted(view.all_enums())}")

    age = view.induced_slot("ex:age", "Person")
    tags = view.induced_slot("ex:tags", "Person")
    address = view.induced_slot("ex:address", "Person")
    if str(age.range) != "integer" or not age.required:
        raise AssertionError(f"age slot drifted: {age!r}")
    if age.minimum_value != 0 or age.maximum_value != 130:
        raise AssertionError(f"age bounds drifted: {age!r}")
    if not tags.multivalued or tags.minimum_cardinality != 1:
        raise AssertionError(f"tags list contract drifted: {tags!r}")
    if str(address.range) != "Address" or not address.inlined:
        raise AssertionError(f"address reference drifted: {address!r}")
    if schema.classes["Person"].extra_slots.allowed:
        raise AssertionError("closed Person class became open")

    generated = _generate(schema)
    expected_defs = {
        element_names[key]: _rewrite_refs(definition, element_names)
        for key, definition in source["$defs"].items()
    }
    if generated.get("$defs") != expected_defs:
        raise AssertionError(
            "official LinkML generator disagrees with the exact CompiledSchema:\n"
            f"expected={json.dumps(expected_defs, indent=2, sort_keys=True)}\n"
            f"actual={json.dumps(generated.get('$defs'), indent=2, sort_keys=True)}"
        )
    _assert_reference_closure(generated)

    valid_person = {
        "@id": "ex:alice",
        "ex:active": True,
        "ex:address": {"ex:city": "Edmonton", "ex:postalCode": "A1"},
        "ex:age": 42,
        "ex:color": "ex:red",
        "ex:name": "Alice",
        "ex:score": 0.75,
        "ex:tags": ["one", "two"],
        "ex:value": "text",
    }
    probes = {
        "Address": [
            {"ex:city": "Edmonton"},
            {},
            {"ex:city": "edmonton"},
            {"ex:city": "Edmonton", "ex:unexpected": True},
        ],
        "Color": ["ex:red", "ex:blue", "ex:green", 7],
        "Person": [
            valid_person,
            {"ex:age": 42},
            {"ex:age": -1, "ex:name": "Alice"},
            {"ex:age": 42, "ex:name": "alice"},
            {"ex:age": 42, "ex:name": "Alice", "ex:color": "ex:green"},
            {"ex:age": 42, "ex:name": "Alice", "ex:tags": []},
            {"ex:age": 42, "ex:name": "Alice", "ex:value": False},
            {"ex:age": 42, "ex:name": "Alice", "ex:unexpected": True},
        ],
    }
    _assert_acceptance_agrees(
        {
            "$schema": source["$schema"],
            "$defs": expected_defs,
        },
        generated,
        probes,
    )
    _assert_reverse(
        payload,
        ["<https://example.org/Address>", "<https://example.org/Person>"],
    )


def _assert_lossy(payload: dict[str, Any]) -> None:
    losses = payload["losses"]["losses"]
    if len(losses) != 18:
        raise AssertionError(f"unexpected lossy-ledger size: {len(losses)}")
    codes = {entry["code"] for entry in losses}
    expected_codes = {
        "array-contains-validation-dropped",
        "conditional-validation-dropped",
        "dependency-validation-dropped",
        "exclusive-bound-validation-widened",
        "format-validation-widened",
        "keyword-validation-dropped",
        "multiple-of-validation-dropped",
        "non-scalar-enum-validation-widened",
        "property-count-validation-dropped",
        "string-length-validation-dropped",
        "tuple-array-validation-widened",
        "unevaluated-validation-dropped",
    }
    if codes != expected_codes:
        raise AssertionError(f"loss code drift: {sorted(codes)}")
    if not all(entry["intentional"] for entry in losses):
        raise AssertionError("lossy fixture contains an unregistered loss")

    schema = _load(payload["yaml"])
    view = SchemaView(schema)
    if sorted(view.all_classes()) != ["Lossy"]:
        raise AssertionError(f"unexpected lossy classes: {sorted(view.all_classes())}")
    if sorted(view.all_enums()) != ["InlineDefsLossyPropertiesExChoiceEnum"]:
        raise AssertionError(f"unexpected lossy enums: {sorted(view.all_enums())}")
    extra = schema.classes["Lossy"].extra_slots
    if not extra.allowed or str(extra.range_expression.range) != "integer":
        raise AssertionError(f"typed extra-slot contract drifted: {extra!r}")

    number = view.induced_slot("ex:number", "Lossy")
    array = view.induced_slot("ex:array", "Lossy")
    choice = view.induced_slot("ex:choice", "Lossy")
    if number.minimum_value != 0 or number.maximum_value != 10:
        raise AssertionError(f"inclusive bound widening drifted: {number!r}")
    if not array.multivalued or len(array.any_of) != 2:
        raise AssertionError(f"tuple widening drifted: {array!r}")
    if str(choice.range) != "InlineDefsLossyPropertiesExChoiceEnum":
        raise AssertionError(f"enum carrier drifted: {choice!r}")

    generated = _generate(schema)
    _assert_reference_closure(generated)
    if not _is_valid(generated, "Lossy", {"ex:number": 5}):
        raise AssertionError("representable numeric interior was rejected")
    if _is_valid(generated, "Lossy", {"ex:number": "five"}):
        raise AssertionError("representable numeric carrier was widened unexpectedly")
    if not _is_valid(generated, "Lossy", {"ex:array": ["text", 7]}):
        raise AssertionError("representable homogeneous tuple union was rejected")
    if _is_valid(generated, "Lossy", {"ex:array": [{"bad": True}]}):
        raise AssertionError("representable tuple item union accepted an object")
    if not _is_valid(generated, "Lossy", {"ex:choice": "ex:open"}):
        raise AssertionError("projected permissible value was rejected")
    if _is_valid(generated, "Lossy", {"ex:choice": "ex:unknown"}):
        raise AssertionError("projected permissible value set widened unexpectedly")

    source = payload["schema"]
    if _is_valid(source, "Lossy", {"ex:number": 0}):
        raise AssertionError("source exclusive minimum unexpectedly accepted its boundary")
    if not _is_valid(generated, "Lossy", {"ex:number": 0}):
        raise AssertionError("recorded inclusive-bound widening is not observable")
    if _is_valid(source, "Lossy", {"ex:label": "x"}):
        raise AssertionError("source string length unexpectedly accepted a short label")
    if not _is_valid(generated, "Lossy", {"ex:label": "x"}):
        raise AssertionError("recorded string-length drop is not observable")
    _assert_reverse(payload, ["<https://example.org/Lossy>"])


def _assert_renamed(payload: dict[str, Any]) -> None:
    schema = _load(payload["yaml"])
    view = SchemaView(schema)
    if sorted(view.all_classes()) != ["Carrier"]:
        raise AssertionError(f"unexpected renamed classes: {sorted(view.all_classes())}")

    reports = payload["slot_renames"]
    if len(reports) != 7:
        raise AssertionError(f"unexpected rename report size: {len(reports)}")
    ordered = sorted(
        reports,
        key=lambda row: (row["source_class"], row["source_path"], row["source_name"]),
    )
    if reports != ordered:
        raise AssertionError("rename report is not in canonical source order")
    if payload["slot_diagnostics"]:
        raise AssertionError("default rename fixture unexpectedly returned skip diagnostics")

    by_source = {row["source_name"]: row for row in reports}
    for source, row in by_source.items():
        slot = view.induced_slot(row["new_slot_name"], "Carrier")
        if str(slot.slot_uri) != row["emitted_slot_uri"]:
            raise AssertionError(
                f"{source!r} slot_uri drifted: {slot.slot_uri!r} != "
                f"{row['emitted_slot_uri']!r}"
            )
        if row["disposition"] == "identity-preserved":
            if row["old_slot_uri"] != row["emitted_slot_uri"]:
                raise AssertionError(f"{source!r} lost its original semantic identity")
        elif row["disposition"] == "identity-rehomed":
            if row["old_slot_uri"] is not None:
                raise AssertionError(f"{source!r} falsely claims a source semantic identity")
            if row["new_slot_name"] != row["emitted_slot_uri"]:
                raise AssertionError(f"{source!r} re-home is not explicit")
        else:
            raise AssertionError(f"unexpected rename disposition: {row!r}")

    expected_uris = {
        "custom:alpha/beta": "custom:alpha/beta",
        "did:example:123": "did:example:123",
        "ex:a/b": "ex:a/b",
        "https://outside.example/path/external": (
            "https://outside.example/path/external"
        ),
        "mailto:cat@example.org": "mailto:cat@example.org",
        "skos:definition": "ex:definition",
        "urn:example:part": "urn:example:part",
    }
    observed_uris = {
        source: row["emitted_slot_uri"] for source, row in by_source.items()
    }
    if observed_uris != expected_uris:
        raise AssertionError(f"renamed identity map drifted: {observed_uris!r}")

    generated = _generate(schema)
    _assert_reference_closure(generated)
    properties = generated["$defs"]["Carrier"]["properties"]
    expected_names = set(by_source)
    if set(properties) != expected_names:
        raise AssertionError(
            f"official generator slot names drifted: {sorted(properties)!r}"
        )
    if generated["$defs"]["Carrier"].get("required") != ["ex:a/b"]:
        raise AssertionError("renamed required slot drifted")

    losses = payload["losses"]["losses"]
    if len(losses) != 1 or losses[0]["code"] != "slot-identity-rehomed":
        raise AssertionError(f"unexpected renamed losses: {losses!r}")
    if not losses[0]["intentional"]:
        raise AssertionError("identity re-home is not registered")

    reverse_properties = payload["reverse"]["schema"]["$defs"]["Carrier"][
        "properties"
    ]
    expected_reverse = {
        "@annotation",
        "@id",
        "@type",
        "custom:alpha/beta",
        "did:example:123",
        "ex:a/b",
        "ex:definition",
        "https://outside.example/path/external",
        "mailto:cat@example.org",
        "urn:example:part",
    }
    if set(reverse_properties) != expected_reverse:
        raise AssertionError(
            f"reverse predicate identities drifted: {sorted(reverse_properties)!r}"
        )
    _assert_reverse(payload, ["<https://example.org/Carrier>"])


def main() -> None:
    if importlib.metadata.version("linkml") != LINKML_PACKAGE_VERSION:
        raise AssertionError("linkml package version is not locked to 1.11.1")
    if importlib.metadata.version("linkml-runtime") != LINKML_PACKAGE_VERSION:
        raise AssertionError("linkml-runtime package version is not locked to 1.11.1")

    payload = _fixture()
    _assert_exact(payload["exact"])
    _assert_lossy(payload["lossy"])
    _assert_renamed(payload["renamed"])
    print(
        "LinkML oracle: exact $defs and 16 instance probes agree; "
        "18 located losses, 7 verified slot renames, reverse SHACL imports, "
        "and representable widening probes pass"
    )


if __name__ == "__main__":
    main()
