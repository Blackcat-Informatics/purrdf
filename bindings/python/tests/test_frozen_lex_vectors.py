# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Frozen lexical answers through the exposed results reader and SPARQL builtin.

The binding exposes JSON string decoding via results parsing, unreserved percent
encoding via ENCODE_FOR_URI, and UCHAR/ECHAR decoding via the SPARQL parser
(replayed in test_frozen_escape_vectors.py). Other lexical files specify raw primitives
(pointers, normalization, byte search and other percent sets) for which there is no exported Python function. Raw Ed25519 key/sign/verify
functions are likewise absent: the Python signer constructs and signs GTS frames,
whose signing preimages differ from the arbitrary-message frozen vectors.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re

import purrdf
import pytest

VECTORS = Path(__file__).resolve().parents[3] / "crates" / "lex" / "tests" / "vectors"
BATCH = 2048


def test_frozen_lexical_capability_inventory() -> None:
    exposed = {"json_string_vectors.txt", "json_unit_vectors.txt", "percent_unreserved_vectors.txt",
               "escape_vectors.txt", "uchar_scalar_vectors.txt"}
    unexposed = {"find_byte_vectors.txt", "json_pointer_vectors.txt",
                 "unicode_normalization_vectors.txt", "normalization_differential_vectors.txt",
                 "percent_csvw_name_vectors.txt", "percent_decode_form_vectors.txt",
                 "percent_decode_vectors.txt", "percent_fragment_vectors.txt",
                 "percent_non_ascii_vectors.txt", "percent_normalize_vectors.txt",
                 "percent_path_vectors.txt", "percent_sub_delims_vectors.txt",
                 "percent_unreserved_slash_vectors.txt", "percent_uri_template_reserved_vectors.txt"}
    assert exposed.isdisjoint(unexposed)
    assert {path.name for path in VECTORS.glob("*.txt")} == exposed | unexposed


def _field(text: str) -> str:
    if text == r"\0":
        return ""
    return re.sub(r"\\\\|\\x([0-9a-f]{2})", lambda m: chr(int(m[1], 16)) if m[1] else "\\", text)


def _encode_field(text: str) -> str:
    return "".join("\\\\" if ch == "\\" else f"\\x{ord(ch):02x}" if ord(ch) < 0x21 or ch == "#" or ord(ch) == 0x7F else ch for ch in text)


def _records(name: str) -> list[list[str]]:
    lines = (VECTORS / name).read_text().split("\n")
    header = dict(line[2:].split(": ", 1) for line in lines if line.startswith(("# vector-count: ", "# body-sha256: ")))
    body = [line for line in lines if line and not line.startswith("#")]
    assert len(body) == int(header["vector-count"])
    assert hashlib.sha256(("\n".join(body) + "\n").encode()).hexdigest() == header["body-sha256"]
    return [[_field(field) for field in line.split("\t")] for line in body]


def _json_values(bodies: list[str]) -> list[str]:
    rows = ",".join('{"x":{"type":"literal","value":"' + body + '"}}' for body in bodies)
    document = '{"head":{"vars":["x"]},"results":{"bindings":[' + rows + ']}}'
    parsed = purrdf.parse_sparql_results("json", document.encode())
    assert parsed[0] == "SELECT" and parsed[1] == ["x"]
    assert len(parsed[2]) == len(bodies)
    return [row[0].value for row in parsed[2]]


@pytest.mark.parametrize("body,answer", _records("json_string_vectors.txt"))
def test_frozen_json_string_decoding(body: str, answer: str) -> None:
    if answer == "-":
        with pytest.raises(ValueError):
            _json_values([body])
    else:
        assert _json_values([body]) == [answer[1:]]


@pytest.mark.parametrize("form,case,first,last,expected", _records("json_unit_vectors.txt"))
def test_frozen_json_unit_and_pair_digests(form, case, first, last, expected) -> None:
    digest = hashlib.sha256()
    digits = "04X" if case == "upper" else "04x"
    def body(cp: int) -> str:
        units = [cp] if form == "unit" else [0xD800 + ((cp - 0x10000) >> 10), 0xDC00 + ((cp - 0x10000) & 1023)]
        return "".join("\\u" + format(unit, digits) for unit in units)
    start, stop = int(first, 16), int(last, 16) + 1
    for offset in range(start, stop, BATCH):
        points = list(range(offset, min(offset + BATCH, stop)))
        if form == "unit" and 0xD800 <= offset < 0xE000:
            for cp in points:
                with pytest.raises(ValueError):
                    _json_values([body(cp)])
                digest.update(b"-\n")
        else:
            for value in _json_values([body(cp) for cp in points]):
                digest.update((_encode_field("=" + value) + "\n").encode())
    assert digest.hexdigest() == expected


def _percent_values(store, texts: list[str]) -> list[str]:
    bindings = " ".join(f"({i} {json.dumps(text, ensure_ascii=False)})" for i, text in enumerate(texts))
    rows = store.query("SELECT (ENCODE_FOR_URI(?s) AS ?encoded) WHERE { VALUES (?i ?s) { " + bindings + " } } ORDER BY ?i")
    return [row[0].value for row in rows]


def test_frozen_unreserved_percent_vectors() -> None:
    store = purrdf.Store()
    for kind, value, answer in _records("percent_unreserved_vectors.txt"):
        if kind == "text":
            assert _percent_values(store, [value]) == [answer]
        else:
            assert kind == "plane"
            first = int(value, 16) << 16
            digest = hashlib.sha256()
            for offset in range(first, first + 65536, BATCH):
                texts = [chr(cp) for cp in range(offset, offset + BATCH) if not 0xD800 <= cp <= 0xDFFF]
                for encoded in _percent_values(store, texts) if texts else []:
                    digest.update((encoded + "\n").encode())
            assert digest.hexdigest() == answer
