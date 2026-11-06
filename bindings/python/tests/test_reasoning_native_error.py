# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: Python exception attributes and native-field snapshots must be
# observed through the installed PyO3 extension and public Python entry points.

"""Native reasoning records survive Python request and receipt presentation."""

import json

import pytest

from purrdf import Store, entail


def test_reasoning_request_fields_cross_python_boundaries():
    store = Store()
    for operation in (
        lambda: entail.rules("RDFS"),
        lambda: store.query_entailment_governed("ASK {}", "RDFS"),
    ):
        with pytest.raises(ValueError) as raised:
            operation()
        fields = raised.value.native_cause
        assert fields["code"] == "reasoning-request"
        assert fields["fields"]["cause"]["value"]["fields"]["offered"]["value"] == "RDFS"
        assert json.loads(raised.value.native_cause_json)["code"] == fields["code"]
    assert entail.rules("rdfs")


def test_reasoning_proof_fields_survive_receipt_attachment():
    answer, certificate = entail.consistency("", [], [], canonical_search="1000000").value
    with pytest.raises(ValueError) as raised:
        entail.check_proof(
            "", "consistency", "", answer, certificate,
            "purrdf-dl-proof 1\navailability not-recorded\n",
            canonical_search="1000000",
        )
    fields = raised.value.native_cause
    assert fields["code"] == "reasoning-proof-document"
    assert fields["fields"]["cause"]["value"]["code"] == "reasoning-proof-not-recorded"
    assert json.loads(raised.value.native_cause_json)["code"] == fields["code"]
    assert isinstance(json.loads(raised.value.canonicalization_json), list)
