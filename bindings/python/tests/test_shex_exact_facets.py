# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: asserts the verdict dicts and the ShExJ text the installed CPython extension's shex.parse and shex.validate hand Python callers
"""ShEx numeric facets through the Python surface, decided by each bound's digits.

The facet semantics are Rust-owned (``purrdf_shex::ExactSchema``). These tests pin
what a Python caller sees: ``shex.validate`` compares every value against the bound
exactly as written, in either schema syntax, and ``shex.parse`` writes those bounds
exactly, so its ShExJ decides every case as the ShExC does.
"""

from __future__ import annotations

from purrdf import shex

EX = "https://example.org/"


def _verdict(schema: str, data: str, node: str, shape: str) -> dict[str, object]:
    """The single verdict for a one-association shape map."""
    entries = shex.validate(schema, data, [(node, shape)])
    assert len(entries) == 1, entries
    return entries[0]


def test_numeric_facets_compare_against_each_bound_as_written() -> None:
    """Each numeric facet decides by its own digits, in either schema syntax.

    ``MAXINCLUSIVE 5.0000000000000000001`` and ``MAXINCLUSIVE 5`` are one ``5`` in
    the schema AST, ``100000000000000000001`` is the double ``1e20``, and a
    400-digit bound is past the double range altogether. The binding validates
    through the exact bounds, and ``parse`` writes them exactly, so its ShExJ
    decides every case as the ShExC does and writes back to the same bytes.
    """
    huge = "1" + "0" * 398
    shexc = (
        f"PREFIX ex: <{EX}>\n"
        "ex:Fine { ex:n MAXINCLUSIVE 5.0000000000000000001 }\n"
        "ex:Five { ex:n MAXINCLUSIVE 5 }\n"
        "ex:Big { ex:n MININCLUSIVE 100000000000000000001 }\n"
        f"ex:Huge {{ ex:n MININCLUSIVE {huge}1 }}\n"
    )
    shexj = shex.parse(shexc)
    assert "5.0000000000000000001" in shexj, shexj
    assert f"{huge}1" in shexj, shexj
    assert shex.parse(shexj, format="shexj") == shexj, "the exact ShExJ is canonical"
    data = (
        f"<{EX}between> <{EX}n> 5.00000000000000000005 .\n"
        f"<{EX}five> <{EX}n> 5 .\n"
        f"<{EX}a> <{EX}n> 100000000000000000000 .\n"
        f"<{EX}b> <{EX}n> 100000000000000000001 .\n"
        f"<{EX}under> <{EX}n> {huge}0 .\n"
        f"<{EX}at> <{EX}n> {huge}1 .\n"
    )
    cases = [
        ("between", "Fine", True),
        ("between", "Five", False),
        ("five", "Fine", True),
        ("five", "Five", True),
        ("a", "Big", False),
        ("b", "Big", True),
        ("under", "Huge", False),
        ("at", "Huge", True),
    ]
    for node, shape, conformant in cases:
        verdict = _verdict(shexc, data, f"{EX}{node}", f"{EX}{shape}")
        assert verdict["conformant"] is conformant, (node, shape, verdict)
        [entry] = shex.validate(
            shexj, data, [(f"{EX}{node}", f"{EX}{shape}")], schema_format="shexj"
        )
        assert entry["conformant"] is conformant, ("shexj", node, shape, entry)
