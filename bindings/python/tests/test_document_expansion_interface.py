# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: This smoke must load the shipped CPython extension to verify its actual export, keyword signature, bytes transport and native exception attributes.
"""The Python interface only; document semantics are covered in native Rust."""

import inspect
import json

import pytest
import purrdf


def test_document_expansion_export_signature_and_native_error_transport():
    function = purrdf.expand_document
    signature = inspect.signature(function)
    assert signature.parameters["request_json"].kind is inspect.Parameter.KEYWORD_ONLY
    source = b'{"urn:p":"python"}'
    result = function(source, request_json=b'{"version":1,"format":"jsonld"}')
    assert isinstance(result, str)
    assert result == '[{"urn:p":[{"@value":"python"}]}]'
    with pytest.raises(TypeError):
        function(bytearray(source), request_json=b'{"version":1,"format":"jsonld"}')
    with pytest.raises(ValueError) as caught:
        function(source, request_json=b'{"version":1,"format":"jsonld","output":{"max_bytes":0}}')
    error = caught.value
    assert error.code == "document-expansion-output"
    assert json.loads(error.diagnostic_json)["code"] == error.code
    assert isinstance(error.native_cause, dict)
    assert "json-output-bytes" in error.native_cause_json
