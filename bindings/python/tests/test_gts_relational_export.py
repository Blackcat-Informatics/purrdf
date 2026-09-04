# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""The three GTS relational export writers: SQLite, DuckDB and Parquet.

Each writer is checked against the SAME oracle — `gts_relational_rows_from_bytes`,
the native projection the writers are layered on — rather than against a
hand-written expected table. A hand-written expectation would only restate this
file's own idea of the schema; comparing to the projection asserts the thing that
can actually break, which is a writer losing, reordering or mistyping rows on the
way out.

These functions previously existed and raised `ValueError("... is pending
reimplementation on purrdf primitives")` unconditionally. The names are
deliberately unchanged, so a caller that already imported them keeps working —
which is what makes implementing them a 1.x addition rather than a break.
"""

from __future__ import annotations

import sqlite3
from typing import TYPE_CHECKING, Any

import pytest

import purrdf
from purrdf import RdfFormat

if TYPE_CHECKING:  # pragma: no cover - typing only
    from pathlib import Path

# A dataset with something in every table the projection produces: plain triples,
# a typed literal, a language-tagged literal, a named graph (so `quads.graph` is
# non-NULL somewhere), and an RDF 1.2 reifier with an annotation (so the
# statement-layer tables are non-empty).
SOURCE = """
PREFIX ex: <http://example.org/>
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>

ex:alice ex:name "Alice"@en ; ex:age 42 .
ex:bob   ex:name "Bob" .
ex:alice ex:knows ex:bob ~ ex:r1 .
ex:r1 ex:certainty "0.9"^^xsd:decimal .
"""

# The five tables, and the column count each writer must round-trip.
TABLES = ("terms", "quads", "reifiers", "annotations", "blobs")


@pytest.fixture(scope="module")
def container() -> bytes:
    """A GTS container built from `SOURCE`."""
    return purrdf.gts_from_quads(SOURCE.encode(), format=RdfFormat.TURTLE)


@pytest.fixture(scope="module")
def rows(container: bytes) -> dict[str, Any]:
    """The native relational projection — the oracle every writer is checked against."""
    return purrdf.gts_relational_rows_from_bytes(container)


def test_the_projection_is_non_trivial(rows: dict[str, Any]) -> None:
    # Guard the guard: if the fixture ever degenerated to an empty projection,
    # every "writer round-trips the projection" assertion below would pass
    # vacuously. The statement-layer tables are the ones most easily lost.
    assert len(rows["terms"]) > 0
    assert len(rows["quads"]) > 0
    assert len(rows["reifiers"]) > 0, "the fixture must exercise the statement layer"
    assert len(rows["annotations"]) > 0


def test_sqlite_export_round_trips_every_table(
    container: bytes, rows: dict[str, Any], tmp_path: Path
) -> None:
    target = tmp_path / "out.sqlite"
    assert purrdf.gts_to_sqlite(container, str(target)) == str(target)
    assert target.exists()

    connection = sqlite3.connect(target)
    try:
        for table in TABLES:
            stored = connection.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0]
            assert stored == len(rows[table]), f"{table} row count must match"

        # Values, not just counts — and in the projection's own order, which is the
        # determinism contract. `terms` is the one with a flattened column.
        quads = connection.execute("SELECT * FROM quads").fetchall()
        assert quads == [tuple(row) for row in rows["quads"]]

        terms = connection.execute(
            "SELECT term_id, kind, value, datatype_id, lang, direction, reifier_id,"
            " triple_s, triple_p, triple_o FROM terms"
        ).fetchall()
        expected = [
            (tid, kind, value, dt, lang, direction, rid, *(triple or (None, None, None)))
            for tid, kind, value, dt, lang, rid, triple, direction in rows["terms"]
        ]
        assert terms == expected
    finally:
        connection.close()


def test_sqlite_export_replaces_rather_than_appends(
    container: bytes, rows: dict[str, Any], tmp_path: Path
) -> None:
    # Exporting twice to one path must not double every row. `CREATE TABLE` alone
    # would leave the old rows in place and silently append a second copy.
    target = tmp_path / "twice.sqlite"
    purrdf.gts_to_sqlite(container, str(target))
    purrdf.gts_to_sqlite(container, str(target))

    connection = sqlite3.connect(target)
    try:
        stored = connection.execute("SELECT COUNT(*) FROM quads").fetchone()[0]
    finally:
        connection.close()
    assert stored == len(rows["quads"])


def test_duckdb_export_round_trips_every_table(
    container: bytes, rows: dict[str, Any], tmp_path: Path
) -> None:
    duckdb = pytest.importorskip("duckdb", reason="the [duckdb] extra is not installed")

    target = tmp_path / "out.duckdb"
    assert purrdf.gts_to_duckdb(container, str(target)) == str(target)

    connection = duckdb.connect(str(target))
    try:
        for table in TABLES:
            stored = connection.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0]
            assert stored == len(rows[table]), f"{table} row count must match"
        quads = connection.execute("SELECT * FROM quads").fetchall()
    finally:
        connection.close()
    assert quads == [tuple(row) for row in rows["quads"]]


def test_parquet_export_writes_one_file_per_table(
    container: bytes, rows: dict[str, Any], tmp_path: Path
) -> None:
    pq = pytest.importorskip(
        "pyarrow.parquet", reason="the [parquet] extra is not installed"
    )

    target = tmp_path / "parquet"
    written = purrdf.gts_to_parquet(container, str(target))

    # Fixed table order, not directory order: a caller zipping this against a
    # schema list must not depend on the filesystem's enumeration.
    assert [p.rsplit("/", 1)[-1] for p in written] == [f"{t}.parquet" for t in TABLES]

    for table, path in zip(TABLES, written, strict=True):
        parquet_table = pq.read_table(path)
        assert parquet_table.num_rows == len(rows[table]), f"{table} row count"

    quads = pq.read_table(written[TABLES.index("quads")]).to_pydict()
    assert list(
        zip(
            quads["subject"],
            quads["predicate"],
            quads["object"],
            quads["graph"],
            strict=True,
        )
    ) == [tuple(row) for row in rows["quads"]]


def test_parquet_export_creates_a_missing_directory(
    container: bytes, tmp_path: Path
) -> None:
    pytest.importorskip("pyarrow.parquet", reason="the [parquet] extra is not installed")
    target = tmp_path / "does" / "not" / "exist"
    written = purrdf.gts_to_parquet(container, str(target))
    assert len(written) == len(TABLES)
    assert target.is_dir()


def test_an_empty_container_exports_empty_tables_not_a_failure(tmp_path: Path) -> None:
    # THE NEIGHBOURING VALID CASE. An empty dataset is valid input, and a writer
    # that special-cases "no rows" into an error — or into a Parquet file with no
    # schema, indistinguishable from a corrupt one — would refuse work that is fine.
    empty = purrdf.gts_from_quads(b"", format=RdfFormat.TURTLE)
    target = tmp_path / "empty.sqlite"
    purrdf.gts_to_sqlite(empty, str(target))

    connection = sqlite3.connect(target)
    try:
        for table in TABLES:
            # The table EXISTS and is queryable; it simply has no rows.
            assert connection.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0] == 0
    finally:
        connection.close()

    pq = pytest.importorskip(
        "pyarrow.parquet", reason="the [parquet] extra is not installed"
    )
    written = purrdf.gts_to_parquet(empty, str(tmp_path / "empty-parquet"))
    for table, path in zip(TABLES, written, strict=True):
        parquet_table = pq.read_table(path)
        assert parquet_table.num_rows == 0
        # The schema survives even with no rows, so a reader can still tell what
        # the columns are.
        assert parquet_table.num_columns > 0, f"{table} must carry its schema"


def test_the_writers_are_reachable_under_every_documented_name(
    container: bytes, tmp_path: Path
) -> None:
    # They used to be native functions registered onto the root module; the
    # implementation moved to Python, and the NAMES must not have moved with it.
    for name in ("gts_to_sqlite", "gts_to_duckdb", "gts_to_parquet"):
        assert hasattr(purrdf, name), f"purrdf.{name}"
        assert hasattr(purrdf.gts, name), f"purrdf.gts.{name}"
        assert getattr(purrdf, name) is getattr(purrdf.gts, name)

    # And they no longer raise: the whole point of the change.
    target = tmp_path / "reachable.sqlite"
    assert purrdf.gts_to_sqlite(container, str(target)) == str(target)


def test_base_direction_survives_the_export(tmp_path: Path) -> None:
    """RDF 1.2 literal base direction reaches the exported `direction` column.

    The term model stores language and direction SEPARATELY — `lang` is bare
    (`en`) and the two are recombined into `@en--ltr` only for display — so the
    relational projection dropped direction entirely until it got a column of its
    own. Nothing pinned it, which is how it went unnoticed.

    This is a conflation, not a cosmetic loss: `@en--ltr` and `@en--rtl` are
    different terms under the canonicalization profile, so two literals differing
    only in direction exported as one indistinguishable pair of rows.
    """
    source = (
        "@prefix ex: <http://example.org/> .\n"
        'ex:cat ex:label "Cat"@en--ltr .\n'
        'ex:cat ex:label "Cat"@en--rtl .\n'
        'ex:cat ex:plain "Cat"@en .\n'
    )
    container = purrdf.gts_from_quads(source.encode(), format=RdfFormat.TURTLE)
    rows = purrdf.gts_relational_rows_from_bytes(container)

    target = tmp_path / "direction.sqlite"
    purrdf.gts_to_sqlite(container, str(target))
    connection = sqlite3.connect(target)
    try:
        stored = connection.execute(
            "SELECT value, lang, direction FROM terms WHERE kind = 1 ORDER BY term_id"
        ).fetchall()
    finally:
        connection.close()

    directions = sorted((d for _v, _l, d in stored), key=lambda d: (d is not None, d))
    assert directions == [None, "ltr", "rtl"], (
        f"each base direction must reach its own column, not be dropped: {stored}"
    )
    # The lang column stays BARE — direction is a separate axis, not a suffix.
    assert {lang for _v, lang, _d in stored} == {"en"}, stored
    # And the two directional literals remain distinguishable, which is the
    # property that was actually lost.
    assert len({(v, lang, d) for v, lang, d in stored}) == 3, stored
    # The export agrees with the projection it is derived from.
    assert sorted(t[7] or "" for t in rows["terms"] if t[1] == 1) == ["", "ltr", "rtl"]


def test_one_reifier_binding_two_triples_exports_both(tmp_path: Path) -> None:
    """A multi-valued reifier must export every binding.

    `rdf:reifies` is NOT a functional property: one reifier id may legitimately
    bind several distinct triples, and the statement layer is multi-valued. An
    export that keyed, joined or de-duplicated on `reifier_id` would keep one
    binding and silently discard the rest — the exact defect the wire format's
    self-describing triple terms exist to prevent.

    Nothing here keys on `reifier_id`: rows are written positionally in the
    projection's order and no table carries a PRIMARY KEY or UNIQUE constraint.
    That is a deliberate property, so it is pinned rather than assumed.
    """
    source = (
        "@prefix ex: <http://example.org/> .\n"
        'ex:cat ex:label "Cat"@en ~ ex:r1 .\n'
        'ex:cat ex:label "Chat"@fr ~ ex:r1 .\n'
    )
    container = purrdf.gts_from_quads(source.encode(), format=RdfFormat.TURTLE)
    rows = purrdf.gts_relational_rows_from_bytes(container)

    assert len(rows["reifiers"]) == 2, (
        f"the projection itself must keep both bindings: {rows['reifiers']}"
    )
    bound = {r[0] for r in rows["reifiers"]}
    assert len(bound) == 1, f"both rows share one reifier id: {rows['reifiers']}"

    target = tmp_path / "multi.sqlite"
    purrdf.gts_to_sqlite(container, str(target))
    connection = sqlite3.connect(target)
    try:
        stored = connection.execute(
            "SELECT reifier_id, subject, predicate, object FROM reifiers"
        ).fetchall()
    finally:
        connection.close()

    assert len(stored) == 2, f"both bindings must survive the export: {stored}"
    assert len({row[3] for row in stored}) == 2, (
        f"and they must remain DISTINCT triples, not one row twice: {stored}"
    )
