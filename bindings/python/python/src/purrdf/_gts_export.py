# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Relational exports of a GTS container: SQLite, DuckDB and Parquet.

These are the three writers that sit on top of the dictionary-encoded relational
projection the native extension already produces
(:func:`gts_relational_rows_from_bytes`). They are deliberately **pure Python**.

# Why Python and not Rust

The projection — reading the container, folding it, and dictionary-encoding
every term, quad, statement-layer row and blob — is the part that has to be fast
and has to agree byte for byte with the rest of PurRDF, and it is already in
Rust. What remains is schema definition and row insertion against three
third-party file formats. Doing that in Rust would pull `rusqlite`, `duckdb` and
the Arrow/Parquet stack into the workspace: a large new supply-chain surface,
three C/C++ build dependencies, and a materially slower build, all to write rows
somebody else's library already knows how to write. `sqlite3` is in the Python
standard library, and DuckDB and PyArrow are one `pip install` away for the
callers who want them.

# The shape of the data

Five tables, one per component of the projection, keyed on the dictionary term
ids the projection assigns:

* ``terms`` — the term dictionary. ``kind`` is ``0`` IRI, ``1`` literal, ``2``
  blank node, ``3`` triple term. A quoted triple's own ``(s, p, o)`` component
  ids are flattened into ``triple_s``/``triple_p``/``triple_o``; they are NOT
  derivable from ``reifier_id``, because one reifier id may bind several
  different triples.
* ``quads`` — ``(subject, predicate, object, graph)``, ``graph`` NULL in the
  default graph.
* ``reifiers`` / ``annotations`` — the RDF 1.2 statement layer, each row
  carrying the graph it was asserted in, because that layer is keyed per graph.
* ``blobs`` — decoded payloads keyed by content digest.

Row ORDER is the projection's own order in every writer, so exporting the same
container twice produces the same file content. That is the same determinism
contract the rest of PurRDF holds itself to, and it is why nothing here sorts,
dedupes or re-keys on the way out.
"""

from __future__ import annotations

import sqlite3
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:  # pragma: no cover - typing only
    from collections.abc import Iterator, Sequence

__all__ = ["gts_to_duckdb", "gts_to_parquet", "gts_to_sqlite"]

# ── Schema ───────────────────────────────────────────────────────────────────
#
# One definition, shared by the SQLite and DuckDB writers. The types are spelled
# in the SQL subset both engines accept, so a caller can join a table from one
# against the equivalent table from the other without a translation step.

_TABLES: tuple[tuple[str, str, tuple[str, ...]], ...] = (
    (
        "terms",
        """CREATE TABLE terms (
            term_id     BIGINT NOT NULL,
            kind        SMALLINT NOT NULL,
            value       VARCHAR,
            datatype_id BIGINT,
            lang        VARCHAR,
            reifier_id  BIGINT,
            triple_s    BIGINT,
            triple_p    BIGINT,
            triple_o    BIGINT
        )""",
        (
            "term_id",
            "kind",
            "value",
            "datatype_id",
            "lang",
            "reifier_id",
            "triple_s",
            "triple_p",
            "triple_o",
        ),
    ),
    (
        "quads",
        """CREATE TABLE quads (
            subject   BIGINT NOT NULL,
            predicate BIGINT NOT NULL,
            object    BIGINT NOT NULL,
            graph     BIGINT
        )""",
        ("subject", "predicate", "object", "graph"),
    ),
    (
        "reifiers",
        """CREATE TABLE reifiers (
            reifier_id BIGINT NOT NULL,
            subject    BIGINT NOT NULL,
            predicate  BIGINT NOT NULL,
            object     BIGINT NOT NULL,
            graph      BIGINT
        )""",
        ("reifier_id", "subject", "predicate", "object", "graph"),
    ),
    (
        "annotations",
        """CREATE TABLE annotations (
            reifier_id BIGINT NOT NULL,
            predicate  BIGINT NOT NULL,
            value      BIGINT NOT NULL,
            graph      BIGINT
        )""",
        ("reifier_id", "predicate", "value", "graph"),
    ),
    (
        "blobs",
        """CREATE TABLE blobs (
            digest  VARCHAR NOT NULL,
            payload BLOB
        )""",
        ("digest", "payload"),
    ),
)


def _rows(data: bytes) -> dict[str, Any]:
    """The native relational projection of a GTS container's bytes."""
    # Imported here rather than at module scope: this module is loaded from the
    # `purrdf` package `__init__`, which is mid-way through swapping itself for
    # the native module when it runs, so a top-level `from purrdf import …`
    # would import a half-built package.
    from .purrdf_native import rdf as _native

    return _native.gts_relational_rows_from_bytes(data)


def _flatten_terms(terms: Sequence[Any]) -> Iterator[tuple[Any, ...]]:
    """Flatten each term row's optional ``(s, p, o)`` triple into three columns."""
    for term_id, kind, value, datatype_id, lang, reifier_id, triple in terms:
        s, p, o = triple if triple is not None else (None, None, None)
        yield (term_id, kind, value, datatype_id, lang, reifier_id, s, p, o)


def _table_rows(rows: dict[str, Any], table: str) -> list[tuple[Any, ...]]:
    """The row tuples for one table, in the projection's own order."""
    if table == "terms":
        return list(_flatten_terms(rows["terms"]))
    if table == "blobs":
        return [(digest, payload) for digest, payload in rows["blobs"]]
    return [tuple(row) for row in rows[table]]


def gts_to_sqlite(data: bytes, path: str) -> str:
    """Export a GTS container's relational projection to a SQLite database.

    Returns `path`. Uses the standard library's :mod:`sqlite3`, so this writer
    has no third-party dependency at all.

    An existing table of the same name is dropped first, so re-exporting over an
    existing file replaces the projection rather than appending a second copy of
    it — appending would silently double every row, which no caller wants and
    which a `CREATE TABLE` alone would not prevent.
    """
    rows = _rows(data)
    connection = sqlite3.connect(path)
    try:
        with connection:
            for table, ddl, columns in _TABLES:
                connection.execute(f"DROP TABLE IF EXISTS {table}")
                connection.execute(ddl)
                placeholders = ", ".join("?" * len(columns))
                connection.executemany(
                    f"INSERT INTO {table} VALUES ({placeholders})",
                    _table_rows(rows, table),
                )
    finally:
        connection.close()
    return path


def gts_to_duckdb(data: bytes, path: str) -> str:
    """Export a GTS container's relational projection to a DuckDB database.

    Returns `path`. Requires the optional `duckdb` dependency
    (``pip install 'purrdf[duckdb]'``).
    """
    duckdb = _require("duckdb", "duckdb")
    rows = _rows(data)
    connection = duckdb.connect(path)
    try:
        for table, ddl, columns in _TABLES:
            connection.execute(f"DROP TABLE IF EXISTS {table}")
            connection.execute(ddl)
            payload = _table_rows(rows, table)
            if payload:
                placeholders = ", ".join("?" * len(columns))
                connection.executemany(
                    f"INSERT INTO {table} VALUES ({placeholders})", payload
                )
    finally:
        connection.close()
    return path


def gts_to_parquet(data: bytes, out_dir: str) -> list[str]:
    """Export a GTS container's relational projection to Parquet files.

    One file per table, written into `out_dir` (created if absent), returned in
    the fixed table order rather than in directory order — a caller that zips the
    result against a schema list must not depend on the filesystem.

    Requires the optional `pyarrow` dependency (``pip install 'purrdf[parquet]'``).
    """
    pa = _require("pyarrow", "parquet")
    pq = _require("pyarrow.parquet", "parquet")

    from pathlib import Path

    directory = Path(out_dir)
    directory.mkdir(parents=True, exist_ok=True)

    rows = _rows(data)
    written: list[str] = []
    for table, _ddl, columns in _TABLES:
        payload = _table_rows(rows, table)
        # Columnar, so transpose — and give an EMPTY table its columns explicitly,
        # because `pa.table({})` on no rows would otherwise write a file with no
        # schema at all and a reader could not tell it from a corrupt one.
        column_data = (
            {name: [row[i] for row in payload] for i, name in enumerate(columns)}
            if payload
            else {name: [] for name in columns}
        )
        target = directory / f"{table}.parquet"
        pq.write_table(pa.table(column_data), target)
        written.append(str(target))
    return written


def _require(module: str, extra: str) -> Any:
    """Import `module`, or raise naming the extra that supplies it.

    A bare `ModuleNotFoundError: No module named 'duckdb'` tells a caller what is
    missing but not how PurRDF expects it to be installed; the extra's name is
    the actionable half.
    """
    import importlib

    try:
        return importlib.import_module(module)
    except ModuleNotFoundError as error:  # pragma: no cover - env dependent
        raise ModuleNotFoundError(
            f"{module} is required for this export and is not installed. "
            f"Install it with: pip install 'purrdf[{extra}]'"
        ) from error
