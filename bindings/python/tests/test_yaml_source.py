# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: CPython object lifetime and exception attributes require the installed Python extension.
"""The paired source operations carry native ownership, bytes and error fields."""

import gc
import json

import pytest

import purrdf


def test_yaml_source_pair_transports_bytes_ownership_and_native_errors() -> None:
    profile = b'{"version":1,"name":"python-source","profile_version":1,"vocabulary":"https://example.org/yaml#"}'
    source = b"\xef\xbb\xbf# original\r\nkey: 'value'\r\n"
    exported = purrdf.yaml_source_export(
        source, source_iri="https://example.org/source.yaml", profile_json=profile
    )
    assert isinstance(exported, purrdf.YamlSourceExport)
    dataset, root = exported.dataset, exported.root
    assert isinstance(dataset, purrdf.RdfDataset) and dataset.quad_count() > 0
    del exported
    gc.collect()
    assert purrdf.yaml_source_reconstruct(dataset, root=root, profile_json=profile) == source

    limited = profile[:-1] + b',"bounds":{"syntax":{"max_input_bytes":0}}}'
    with pytest.raises(ValueError) as refused:
        purrdf.yaml_source_export(source, source_iri="https://example.org/source.yaml", profile_json=limited)
    assert refused.value.native_cause["code"] == "yaml-source-limit"
    assert refused.value.native_cause["fields"]["resource"]["value"] == "source bytes"

    with pytest.raises(ValueError) as refused:
        purrdf.yaml_source_export(b"[unterminated", source_iri="https://example.org/source.yaml", profile_json=profile)
    error = refused.value
    assert error.code == "yaml-source-native"
    assert json.loads(error.diagnostic_json)["code"] == error.code
    assert error.message_id is None and error.presentation is None
    assert json.loads(error.native_cause_json)["code"] == error.native_cause["code"] == "yaml-source-syntax"
    position = error.native_cause["fields"]["cause"]["value"]["fields"]
    assert position["line"]["value"] > 0 and position["column"]["value"] > 0
    assert position["offset"]["value"] >= 0

    with pytest.raises(ValueError) as refused:
        purrdf.yaml_source_reconstruct(dataset, root=root, profile_json=b"{")
    assert refused.value.native_cause["code"] == "json-document"
    del dataset
    gc.collect()
    assert error.native_cause["code"] == "yaml-source-syntax"
