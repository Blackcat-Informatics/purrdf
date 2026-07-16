# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Execute emitted Pydantic models and compare their live JSON Schema surface."""

from __future__ import annotations

import importlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

from pydantic import ValidationError


REPO = Path(__file__).resolve().parents[3]
SCHEMA_MAP_KEYWORDS = (
    "$defs",
    "properties",
    "patternProperties",
    "dependentSchemas",
)
SCHEMA_ARRAY_KEYWORDS = ("allOf", "anyOf", "oneOf", "prefixItems")
SCHEMA_SINGLE_KEYWORDS = (
    "items",
    "additionalItems",
    "unevaluatedItems",
    "additionalProperties",
    "unevaluatedProperties",
    "propertyNames",
    "contains",
    "not",
    "if",
    "then",
    "else",
    "contentSchema",
)


def _fixture() -> dict[str, Any]:
    completed = subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "purrdf-shapes",
            "--example",
            "pydantic_oracle_fixture",
            "--locked",
            "--quiet",
        ],
        cwd=REPO,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(completed.stdout)


def _rewrite_refs(value: Any, model_paths: dict[str, str]) -> Any:
    if not isinstance(value, dict):
        return value
    rewritten = dict(value)
    if "$ref" in value:
        child = value["$ref"]
        prefix = "#/$defs/"
        if not isinstance(child, str) or not child.startswith(prefix):
            raise AssertionError(f"oracle fixture contains unsupported ref: {child!r}")
        source_key = child.removeprefix(prefix).replace("~1", "/").replace("~0", "~")
        class_name = model_paths[source_key].rsplit(".", 1)[1]
        rewritten["$ref"] = f"#/$defs/{class_name}"
    for keyword in SCHEMA_MAP_KEYWORDS:
        children = value.get(keyword)
        if isinstance(children, dict):
            rewritten[keyword] = {
                key: _rewrite_refs(child, model_paths)
                for key, child in children.items()
            }
    for keyword in SCHEMA_ARRAY_KEYWORDS:
        children = value.get(keyword)
        if isinstance(children, list):
            rewritten[keyword] = [
                _rewrite_refs(child, model_paths) for child in children
            ]
    for keyword in SCHEMA_SINGLE_KEYWORDS:
        child = value.get(keyword)
        if isinstance(child, (dict, bool)):
            rewritten[keyword] = _rewrite_refs(child, model_paths)
    return rewritten


def _definition_surface(schema: dict[str, Any]) -> dict[str, Any]:
    surface = dict(schema)
    surface.pop("$defs", None)
    return surface


def _assert_rejects(model: type[Any], payload: Any) -> None:
    try:
        model.model_validate(payload)
    except ValidationError:
        return
    raise AssertionError(f"{model.__name__} unexpectedly accepted {payload!r}")


def _normalize_inferred_types(actual: Any, expected: Any) -> Any:
    """Drop only a Pydantic type assertion already implied by a source enum."""
    if isinstance(actual, list) and isinstance(expected, list):
        return [
            _normalize_inferred_types(item, expected[index])
            for index, item in enumerate(actual)
        ]
    if not isinstance(actual, dict) or not isinstance(expected, dict):
        return actual
    normalized = dict(actual)
    if "enum" in expected and "type" not in expected:
        normalized.pop("type", None)
    for key in normalized.keys() & expected.keys():
        normalized[key] = _normalize_inferred_types(normalized[key], expected[key])
    return normalized


def main() -> None:
    payload = _fixture()
    with tempfile.TemporaryDirectory(prefix="purrdf-pydantic-oracle-") as directory:
        root = Path(directory)
        for relative, text in payload["artifacts"].items():
            destination = root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_text(text, encoding="utf-8", newline="\n")

        sys.path.insert(0, str(root))
        try:
            models = importlib.import_module("oracle_models.models")
            source_defs = payload["schema"]["$defs"]
            model_paths = payload["model_paths"]
            expected_defs = {
                import_path.rsplit(".", 1)[1]: _rewrite_refs(
                    source_defs[source_key], model_paths
                )
                for source_key, import_path in model_paths.items()
            }
            for source_key, import_path in model_paths.items():
                class_name = import_path.rsplit(".", 1)[1]
                model = getattr(models, class_name)
                native_schema = super(model, model).model_json_schema(by_alias=True)
                if not isinstance(native_schema, dict):
                    raise AssertionError(
                        f"{source_key} cannot produce a native Pydantic JSON Schema"
                    )
                live_schema = model.model_json_schema(by_alias=True)
                if live_schema.get("$defs") != expected_defs:
                    raise AssertionError(
                        f"{source_key} model_json_schema() has a divergent $defs table:\n"
                        f"expected={json.dumps(expected_defs, indent=2, sort_keys=True)}\n"
                        f"actual={json.dumps(live_schema.get('$defs'), indent=2, sort_keys=True)}"
                    )
                actual = _definition_surface(live_schema)
                expected = _rewrite_refs(source_defs[source_key], model_paths)
                actual = _normalize_inferred_types(actual, expected)
                if actual != expected:
                    raise AssertionError(
                        f"{source_key} model_json_schema() disagrees with CompiledSchema:\n"
                        f"expected={json.dumps(expected, indent=2, sort_keys=True)}\n"
                        f"actual={json.dumps(actual, indent=2, sort_keys=True)}"
                    )

            color = models.Color.model_validate("ex:red")
            assert color.model_dump(mode="json") == "ex:red"
            _assert_rejects(models.Color, "ex:green")
            _assert_rejects(models.Empty, "anything")

            state = models.State.model_validate({"@id": "ex:open"})
            assert state.model_dump(mode="json") == {"@id": "ex:open"}
            _assert_rejects(models.State, {"@id": "ex:unknown"})

            person_payload = {
                "@id": "ex:alice",
                "ex:active": True,
                "ex:address": {"ex:city": "E", "ex:postalCode": "A1"},
                "ex:age": 42,
                "ex:color": "ex:red",
                "ex:friend": {"ex:age": 39, "ex:name": "Bob"},
                "ex:label": "Al",
                "ex:lookahead": "A",
                "ex:name": "Alice",
                "ex:nullableCount": 2,
                "ex:nullableName": "Name",
                "ex:nullableTags": ["one"],
                "ex:path": "mapped:value",
                "ex:score": 0.75,
                "ex:tags": ["one", "two"],
                "ex:when": "2026-07-15T12:00:00Z",
            }
            person = models.Person.model_validate(person_payload)
            dumped = person.model_dump(mode="json", by_alias=True, exclude_none=True)
            assert dumped == person_payload
            alias = models.PersonAlias.model_validate(person_payload)
            assert alias.model_dump(mode="json", by_alias=True, exclude_none=True) == person_payload

            nullable_payload = dict(person_payload)
            nullable_payload.update(
                {
                    "ex:nullableCount": None,
                    "ex:nullableName": None,
                    "ex:nullableTags": None,
                }
            )
            nullable_person = models.Person.model_validate(nullable_payload)
            nullable_dump = nullable_person.model_dump(mode="json", by_alias=True)
            assert nullable_dump["ex:nullableCount"] is None
            assert nullable_dump["ex:nullableName"] is None
            assert nullable_dump["ex:nullableTags"] is None

            _assert_rejects(models.Person, {"ex:age": -1, "ex:name": "Alice"})
            _assert_rejects(models.Person, {"ex:age": 1, "ex:name": ""})
            _assert_rejects(models.Person, {"ex:age": 1, "name": "Alice"})
            for non_json_number in [float("nan"), float("inf"), float("-inf")]:
                _assert_rejects(
                    models.Person,
                    {
                        "ex:age": 1,
                        "ex:name": "Alice",
                        "ex:score": non_json_number,
                    },
                )
            _assert_rejects(
                models.Person,
                {"ex:age": 1, "ex:name": "Alice", "ex:nullableCount": -1},
            )
            _assert_rejects(
                models.Person,
                {"ex:age": 1, "ex:name": "Alice", "ex:nullableName": "n"},
            )
            _assert_rejects(
                models.Person,
                {"ex:age": 1, "ex:name": "Alice", "ex:nullableTags": []},
            )
            _assert_rejects(
                models.Person,
                {"ex:age": 1, "ex:name": "Alice", "ex:when": 0},
            )
            _assert_rejects(
                models.Person,
                {"ex:age": 1, "ex:name": "Alice", "ex:path": "wrong"},
            )
            _assert_rejects(
                models.Person,
                {"ex:age": 1, "ex:name": "Alice", "ex:label": "A"},
            )
            typed_label = models.Person.model_validate(
                {
                    "ex:age": 1,
                    "ex:name": "Alice",
                    "ex:label": {
                        "@value": "A",
                        "@type": "xsd:string",
                        "@language": "en",
                    },
                }
            )
            assert typed_label.model_dump(
                mode="json", by_alias=True, exclude_none=True
            )["ex:label"] == {
                "@value": "A",
                "@type": "xsd:string",
                "@language": "en",
            }
            _assert_rejects(
                models.Person,
                {"ex:age": 1, "ex:name": "Alice", "ex:unexpected": True},
            )
            _assert_rejects(
                models.Person,
                {
                    "ex:age": 1,
                    "ex:name": "Alice",
                    "ex:address": {"ex:postalCode": "bad"},
                },
            )
        finally:
            sys.path.remove(str(root))

    print("Pydantic oracle: 6 live model schemas agree; validation/alias probes pass")


if __name__ == "__main__":
    main()
