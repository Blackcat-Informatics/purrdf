# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""GTS exports through the Rust-owned native v1 five-table projection.

SQLite and DuckDB insert canonical rows with the native schema. Parquet bytes
come from the first-party codec. Python owns host file/database I/O only.
Term ids follow canonical RDF value order; they differ from folded GTS ids.
The schema carries blank scopes, graph declarations, base direction and the
RDF 1.2 statement layer. Blob contents are verified on import and keyed by the
native SHA-256 identity; their GTS BLAKE3 identity is recoverable from the bytes.
"""

from __future__ import annotations

import sqlite3
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from collections.abc import Iterator

__all__ = ["gts_to_duckdb", "gts_to_parquet", "gts_to_sqlite"]


def _rows(data: bytes) -> dict[str, Any]:
    """Canonical native v1 rows and their Rust-owned schema."""
    from .purrdf_native import rdf as _native

    return _native.gts_columnar_rows_from_bytes(data)


def _tables(rows: dict[str, Any]) -> Iterator[tuple[str, str, int]]:
    """SQL DDL from the trusted native schema, independent of container values."""
    sql_types = {"integer": "BIGINT", "text": "VARCHAR", "bytes": "BLOB"}
    for table, columns in rows["schema"].items():
        definitions = [
            f"{name} {sql_types[kind]}" + ("" if nullable else " NOT NULL")
            for name, kind, nullable in columns
        ]
        yield table, f"CREATE TABLE {table} ({', '.join(definitions)})", len(columns)


def gts_to_sqlite(data: bytes, path: str) -> str:
    """Write canonical native v1 rows to SQLite; return ``path``.

    Existing projection tables are replaced in one transaction. SQLite is part
    of Python's standard library and needs no optional dependency.
    """
    rows = _rows(data)
    connection = sqlite3.connect(path)
    try:
        with connection:
            for table, ddl, count in _tables(rows):
                connection.execute(f"DROP TABLE IF EXISTS {table}")
                connection.execute(ddl)
                placeholders = ", ".join("?" * count)
                connection.executemany(
                    f"INSERT INTO {table} VALUES ({placeholders})", rows[table]
                )
    finally:
        connection.close()
    return path


def gts_to_duckdb(data: bytes, path: str) -> str:
    """Write canonical native v1 rows to DuckDB; return ``path``.

    Requires ``pip install 'purrdf[duckdb]'``. Existing tables are replaced
    together; a failed insertion rolls the transaction back.
    """
    duckdb = _require("duckdb", "duckdb")
    rows = _rows(data)
    connection = duckdb.connect(path)
    try:
        connection.execute("BEGIN TRANSACTION")
        try:
            for table, ddl, count in _tables(rows):
                connection.execute(f"DROP TABLE IF EXISTS {table}")
                connection.execute(ddl)
                if rows[table]:
                    placeholders = ", ".join("?" * count)
                    connection.executemany(
                        f"INSERT INTO {table} VALUES ({placeholders})", rows[table]
                    )
            connection.execute("COMMIT")
        except BaseException:
            connection.execute("ROLLBACK")
            raise
    finally:
        connection.close()
    return path


def gts_to_parquet(data: bytes, out_dir: str) -> list[str]:
    """Write first-party native v1 Parquet files in canonical table order.

    No optional encoder dependency is required. The output includes all five
    files even for an empty dataset, with complete schemas and native metadata.
    """
    from pathlib import Path
    from .purrdf_native import rdf as _native

    files = _native.gts_columnar_parquet_from_bytes(data)
    directory = Path(out_dir)
    directory.mkdir(parents=True, exist_ok=True)
    written = []
    for name, content in files.items():
        target = directory / name
        target.write_bytes(content)
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
