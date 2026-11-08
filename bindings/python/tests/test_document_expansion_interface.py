# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: This smoke must load the shipped CPython extension to verify its actual export, keyword signature, bytes transport and native exception attributes.
"""Python interfaces only; document semantics are covered in native Rust."""
import inspect
import json
import pytest
import purrdf
def test_document_operations_export_signatures_and_native_error_transport():
    source = b'{"@id":"urn:s","urn:p":"python"}'
    for function, frames, request, expected, code in [
        (purrdf.expand_document, (), b'{"version":1,"format":"jsonld"}', '[{"@id":"urn:s","urn:p":[{"@value":"python"}]}]', "document-expansion-output"),
        (purrdf.compact_document, (), b'{"version":1,"format":"jsonld","output_context":{}}', '{"@context":{},"@id":"urn:s","urn:p":"python"}', "document-compaction-output"),
        (purrdf.flatten_document, (), b'{"version":1,"format":"jsonld","output_context":{}}', '{"@context":{},"@id":"urn:s","urn:p":"python"}', "document-flattening-output"),
        (purrdf.frame_document, (b'{}',), b'{"version":1,"format":"jsonld"}', '{"@context":{},"@id":"urn:s","urn:p":"python"}', "document-framing-output"),
    ]:
        assert inspect.signature(function).parameters["request_json"].kind is inspect.Parameter.KEYWORD_ONLY
        assert function(source, *frames, request_json=request) == expected
        with pytest.raises(TypeError):
            function(bytearray(source), *frames, request_json=request)
        controls = json.loads(request)
        controls["output"] = {"max_bytes": 0}
        with pytest.raises(ValueError) as caught:
            function(source, *frames, request_json=json.dumps(controls).encode())
        error = caught.value
        assert error.code == code
        assert json.loads(error.diagnostic_json)["code"] == error.code
        assert isinstance(error.native_cause, dict)
        assert "json-output-bytes" in error.native_cause_json
