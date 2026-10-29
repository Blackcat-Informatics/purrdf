# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: These cases exercise Python keyword decoding, bytes extraction and returned dictionaries through the installed extension.
"""Caller ordering crosses the installed Python boundary with its evidence."""

import pytest
from purrdf import retrieval

EX = "https://example.org/"
STRATUM = f"{EX}stratum/text"
DATA = f'<{EX}a> <{EX}note> "alpha alpha alpha" .\n<{EX}b> <{EX}note> "alpha beta beta beta" .'
REQUEST = [("lexical", "alpha", None, f"{EX}note")]
KWARGS = {
    "text_producers": {f"{EX}pf/search": (STRATUM, f"{EX}note", "any", {"lexicon": "empty"})},
    "statistics": {"source": "caller", "revision": "r1"},
    "weights": {STRATUM: 10**12},
    "k": 60,
    "decay": "reciprocal_rank",
    "scale_digits": 12,
    "top_k": 1,
}


def test_ordering_keywords_preserve_bytes_and_snapshot_evidence():
    ordering = {
        "semantics": f"{EX}order/tier-activation",
        "leading": True,
        "keys": {f"<{EX}a>": (b"\x02", b"a"), f"<{EX}b>": (b"\x01", b"b")},
    }
    answer = retrieval.search(DATA, REQUEST, ordering=ordering, **KWARGS)
    row = answer["rows"][0]
    assert row["entity"] == f"<{EX}b>"
    assert row["contributions"][0]["rank"] == 2
    assert row["order_witness"]["leading"] == b"\x01"
    assert row["order_witness"]["tie"] == b"b"
    assert answer["ordering"][STRATUM]["semantics"] == ordering["semantics"]
    assert len(answer["ordering"][STRATUM]["snapshot"]) == 32
    assert "unemitted_order" in answer
    repeated = retrieval.search(DATA, REQUEST, ordering=ordering, **KWARGS)
    assert repeated["evidence_id"] == answer["evidence_id"]
    changed = dict(ordering, keys={f"<{EX}a>": (b"\x02", b"a"), f"<{EX}b>": (b"\x01", b"changed")})
    assert retrieval.search(DATA, REQUEST, ordering=changed, **KWARGS)["evidence_id"] != answer["evidence_id"]
    compiled = retrieval.compile(DATA, REQUEST, ordering=ordering, **KWARGS)
    assert next(iter(compiled["planned_resolution"].values()))["requested_depth"] == 2
    planned = retrieval.plan(
        DATA, REQUEST, text_producers=KWARGS["text_producers"],
        statistics=KWARGS["statistics"], top_k=1, caller_ordered=True,
    )
    assert planned["stratum_depths"][STRATUM] == answer["planned_resolution"][STRATUM]["requested_depth"] == 2
    assert retrieval.certify_plan(planned["canonical_bytes"])["plan_id"] == planned["plan_id"]
    with pytest.raises(ValueError, match="requires keys"):
        retrieval.search(DATA, REQUEST, ordering={"semantics": ordering["semantics"], "leading": True}, **KWARGS)
    with pytest.raises(ValueError, match="no keys"):
        retrieval.search(DATA, REQUEST, ordering=dict(ordering, keys={}), **KWARGS)
    with pytest.raises((TypeError, ValueError)):
        retrieval.search(DATA, REQUEST, ordering=dict(ordering, keys={f"<{EX}a>": ("text", b"a")}), **KWARGS)
