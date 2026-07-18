# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Execute emitted Pydantic models and compare their live JSON Schema surface."""

from __future__ import annotations

import hashlib
import importlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

from packaging.version import InvalidVersion, Version
from pydantic import ValidationError


REPO = Path(__file__).resolve().parents[3]
FLAT_BASELINE_SHA256 = "a89d78ba9be04d5d10be8709ea81da938176e5c678920c09007c3b94d60d3625"
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


def _assert_version_oracle(cases: list[dict[str, Any]]) -> None:
    for case in cases:
        raw = case["raw"]
        try:
            parsed = Version(raw)
            packaging_accepts = True
        except InvalidVersion:
            parsed = None
            packaging_accepts = False
        over_limit = len(raw.encode("utf-8")) > 512
        expected = packaging_accepts and not over_limit
        if case["accepted"] != expected:
            raise AssertionError(
                f"PEP 440 differential mismatch for {raw!r}: "
                f"packaging={packaging_accepts}, purrdf={case}"
            )
        if case["resource_error"] != (packaging_accepts and over_limit):
            raise AssertionError(f"version resource classification mismatch: {case}")
        if expected:
            assert parsed is not None
            if case["is_local"] != (parsed.local is not None):
                raise AssertionError(f"local-version classification mismatch: {case}")


def _assert_reverse(reverse: dict[str, Any], expected_shape_ids: list[str]) -> None:
    if reverse["shape_ids"] != expected_shape_ids:
        raise AssertionError(f"unexpected reverse SHACL shapes: {reverse['shape_ids']!r}")
    reverse_losses = reverse["losses"]["losses"]
    if not all(
        entry["from"] == "pydantic-v2"
        and entry["to"] == "shacl"
        and entry["intentional"]
        and " subject=#/" in entry["location"]
        for entry in reverse_losses
    ):
        raise AssertionError("Pydantic reverse package has an unsound or unlocated loss")


def _load_models(model_paths: dict[str, str]) -> dict[str, type[Any]]:
    models: dict[str, type[Any]] = {}
    for source_key, import_path in model_paths.items():
        module_name, class_name = import_path.rsplit(".", 1)
        module = importlib.import_module(module_name)
        model = getattr(module, class_name)
        if not isinstance(model, type):
            raise AssertionError(f"{import_path} is not a generated model type")
        models[source_key] = model
    return models


def _assert_package_runtime(
    schema: dict[str, Any],
    model_paths: dict[str, str],
    metadata: dict[str, dict[str, Any]] | None,
) -> None:
    source_defs = schema["$defs"]
    models = _load_models(model_paths)
    expected_defs = {
        import_path.rsplit(".", 1)[1]: _rewrite_refs(source_defs[source_key], model_paths)
        for source_key, import_path in model_paths.items()
    }
    for source_key, model in models.items():
        expected_module = model_paths[source_key].rsplit(".", 1)[0]
        if model.__module__ != expected_module:
            raise AssertionError(
                f"{source_key} module drifted: {model.__module__!r} != {expected_module!r}"
            )
        native_schema = super(model, model).model_json_schema(by_alias=True)
        if not isinstance(native_schema, dict):
            raise AssertionError(
                f"{source_key} cannot produce a native Pydantic JSON Schema"
            )
        if metadata is not None:
            if model.__doc__ != f"Caller documentation for {source_key}.":
                raise AssertionError(f"{source_key} lost its caller class documentation")
            if model.model_config.get("json_schema_extra") != metadata[source_key]:
                raise AssertionError(f"{source_key} lost caller metadata linkage")
            native_metadata = native_schema.get("$defs", {}).get(
                model.__name__, native_schema
            )
            for key, value in metadata[source_key].items():
                if native_metadata.get(key) != value:
                    raise AssertionError(
                        f"{source_key} native schema lost metadata key {key!r}"
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

    color = models["Color"].model_validate("ex:red")
    assert color.model_dump(mode="json") == "ex:red"
    _assert_rejects(models["Color"], "ex:green")
    _assert_rejects(models["Empty"], "anything")

    state = models["State"].model_validate({"@id": "ex:open"})
    assert state.model_dump(mode="json") == {"@id": "ex:open"}
    _assert_rejects(models["State"], {"@id": "ex:unknown"})

    if "CycleLeft" in models:
        cycle = models["CycleLeft"].model_validate({"ex:right": {"ex:left": {}}})
        assert cycle.model_dump(mode="json", by_alias=True, exclude_none=True) == {
            "ex:right": {"ex:left": {}}
        }

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
    person = models["Person"].model_validate(person_payload)
    dumped = person.model_dump(mode="json", by_alias=True, exclude_none=True)
    assert dumped == person_payload
    alias = models["PersonAlias"].model_validate(person_payload)
    assert alias.model_dump(mode="json", by_alias=True, exclude_none=True) == person_payload

    nullable_payload = dict(person_payload)
    nullable_payload.update(
        {
            "ex:nullableCount": None,
            "ex:nullableName": None,
            "ex:nullableTags": None,
        }
    )
    nullable_person = models["Person"].model_validate(nullable_payload)
    nullable_dump = nullable_person.model_dump(mode="json", by_alias=True)
    assert nullable_dump["ex:nullableCount"] is None
    assert nullable_dump["ex:nullableName"] is None
    assert nullable_dump["ex:nullableTags"] is None

    _assert_rejects(models["Person"], {"ex:age": -1, "ex:name": "Alice"})
    _assert_rejects(models["Person"], {"ex:age": 1, "ex:name": ""})
    _assert_rejects(models["Person"], {"ex:age": 1, "name": "Alice"})
    for non_json_number in [float("nan"), float("inf"), float("-inf")]:
        _assert_rejects(
            models["Person"],
            {"ex:age": 1, "ex:name": "Alice", "ex:score": non_json_number},
        )
    _assert_rejects(
        models["Person"],
        {"ex:age": 1, "ex:name": "Alice", "ex:nullableCount": -1},
    )
    _assert_rejects(
        models["Person"],
        {"ex:age": 1, "ex:name": "Alice", "ex:nullableName": "n"},
    )
    _assert_rejects(
        models["Person"],
        {"ex:age": 1, "ex:name": "Alice", "ex:nullableTags": []},
    )
    _assert_rejects(
        models["Person"], {"ex:age": 1, "ex:name": "Alice", "ex:when": 0}
    )
    _assert_rejects(
        models["Person"], {"ex:age": 1, "ex:name": "Alice", "ex:path": "wrong"}
    )
    _assert_rejects(
        models["Person"], {"ex:age": 1, "ex:name": "Alice", "ex:label": "A"}
    )
    typed_label = models["Person"].model_validate(
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
    assert typed_label.model_dump(mode="json", by_alias=True, exclude_none=True)[
        "ex:label"
    ] == {
        "@value": "A",
        "@type": "xsd:string",
        "@language": "en",
    }
    _assert_rejects(
        models["Person"],
        {"ex:age": 1, "ex:name": "Alice", "ex:unexpected": True},
    )
    _assert_rejects(
        models["Person"],
        {
            "ex:age": 1,
            "ex:name": "Alice",
            "ex:address": {"ex:postalCode": "bad"},
        },
    )


def _assert_strict_routed_types(root: Path) -> None:
    consumer = root / "routed_consumer.py"
    consumer.write_text(
        """# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
from routed_oracle_models.catalog.enums import Color, Empty, State
from routed_oracle_models.common.paths import PathWithToken
from routed_oracle_models.cycles.left import CycleLeft
from routed_oracle_models.cycles.right import CycleRight
from routed_oracle_models.domain.people import Person, PersonAlias

def compose(
    person: Person,
    alias: PersonAlias,
    color: Color,
    empty: Empty,
    state: State,
    path: PathWithToken,
    cycle_left: CycleLeft,
    cycle_right: CycleRight,
) -> tuple[Person, PersonAlias, Color, Empty, State, PathWithToken, CycleLeft, CycleRight]:
    return person, alias, color, empty, state, path, cycle_left, cycle_right
""",
        encoding="utf-8",
        newline="\n",
    )
    env = dict(os.environ)
    env["MYPYPATH"] = str(root)
    completed = subprocess.run(
        [
            sys.executable,
            "-m",
            "mypy",
            "--strict",
            "--no-incremental",
            "routed_oracle_models",
            "routed_consumer.py",
        ],
        cwd=root,
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        raise AssertionError(
            "strict mypy rejected the routed generated package:\n"
            f"{completed.stdout}{completed.stderr}"
        )


def main() -> None:
    payload = _fixture()
    flat_baseline = json.dumps(
        [payload["artifacts"], payload["model_paths"], payload["reverse"]["losses"]],
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ) + "\n"
    flat_digest = hashlib.sha256(flat_baseline.encode()).hexdigest()
    if flat_digest != FLAT_BASELINE_SHA256:
        raise AssertionError(
            f"flat Pydantic package baseline drifted: {flat_digest} != "
            f"{FLAT_BASELINE_SHA256}"
        )
    _assert_version_oracle(payload["version_oracle"])
    _assert_reverse(payload["reverse"], ["<https://example.org/Person>"])
    _assert_reverse(
        payload["routed"]["reverse"],
        [
            "<https://example.org/CycleLeft>",
            "<https://example.org/CycleRight>",
            "<https://example.org/Person>",
        ],
    )
    with tempfile.TemporaryDirectory(prefix="purrdf-pydantic-oracle-") as directory:
        root = Path(directory)
        for artifacts in [payload["artifacts"], payload["routed"]["artifacts"]]:
            for relative, text in artifacts.items():
                destination = root / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_text(text, encoding="utf-8", newline="\n")

        _assert_strict_routed_types(root)

        sys.path.insert(0, str(root))
        try:
            _assert_package_runtime(payload["schema"], payload["model_paths"], None)
            _assert_package_runtime(
                payload["routed"]["schema"],
                payload["routed"]["model_paths"],
                payload["routed"]["metadata"],
            )
            routed_root = importlib.import_module("routed_oracle_models")
            if routed_root.__version__ != payload["routed"]["version"]:
                raise AssertionError("routed package version export drifted")
        finally:
            sys.path.remove(str(root))

    print(
        f"Pydantic oracle: {len(payload['version_oracle'])} PEP 440 differential cases "
        "agree; flat 6-model and routed 8-model packages pass strict typing, live schemas, "
        "validation/alias probes, metadata/version linkage, and verified reverse SHACL import"
    )


if __name__ == "__main__":
    main()
