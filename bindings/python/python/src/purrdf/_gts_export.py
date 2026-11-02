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
from contextlib import ExitStack, contextmanager
from dataclasses import dataclass
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import TYPE_CHECKING, Any
from types import SimpleNamespace

if TYPE_CHECKING:
    from collections.abc import Iterator

__all__ = ["GtsExportResult", "gts_to_duckdb", "gts_to_parquet", "gts_to_sqlite"]


@dataclass(frozen=True)
class GtsExportResult:
    """Written paths and the always-present native admission report."""

    paths: tuple[str, ...]
    report: Any


def _tables(rows: dict[str, Any]) -> Iterator[tuple[str, str, int]]:
    """SQL DDL from the trusted native schema, independent of container values."""
    sql_types = {"integer": "BIGINT", "text": "VARCHAR", "bytes": "BLOB"}
    for table, columns in rows["schema"].items():
        definitions = [
            f"{name} {sql_types[kind]}" + ("" if nullable else " NOT NULL")
            for name, kind, nullable in columns
        ]
        yield table, f"CREATE TABLE {table} ({', '.join(definitions)})", len(columns)


def _write_chunk(output: Any, chunk: bytes) -> None:
    if output.write(chunk) != len(chunk):
        raise OSError("GTS export destination accepted only part of a chunk")


@contextmanager
def _staging() -> Iterator[Any]:
    """Host-owned provisional storage; Rust owns decoding and admission."""
    with TemporaryDirectory(prefix="purrdf-gts-") as directory, ExitStack() as stack:
        files = []
        reader = None
        reader_handle = None

        def begin() -> int:
            if files:
                files[-1].close()
            handle = len(files)
            files.append(stack.enter_context((Path(directory) / str(handle)).open("w+b")))
            return handle

        def write(handle: int, chunk: bytes) -> None:
            _write_chunk(files[handle], chunk)
            files[handle].flush()

        def read(handle: int, offset: int, length: int) -> bytes:
            nonlocal reader, reader_handle
            if files and handle == len(files) - 1:
                files[-1].close()
            if reader_handle != handle:
                if reader is not None:
                    reader.close()
                reader = stack.enter_context(Path(files[handle].name).open("rb"))
                reader_handle = handle
            reader.seek(offset)
            return reader.read(length)

        def discard(handle: int) -> None:
            nonlocal reader, reader_handle
            files[handle].close()
            if reader_handle == handle:
                reader.close()
                reader = None
                reader_handle = None
            Path(files[handle].name).unlink()

        yield SimpleNamespace(begin=begin, write=write, read=read, discard=discard)


def gts_to_sqlite(data: bytes, path: str, *, blob_limits: Any) -> GtsExportResult:
    """Write canonical native v1 rows to SQLite; return paths and admission report.

    Existing projection tables are replaced in one transaction. SQLite is part
    of Python's standard library and needs no optional dependency.
    """
    from .purrdf_native import rdf as _native

    with _staging() as staging:
        rows, blobs, report = _native.gts_columnar_staged_from_bytes(
            data, blob_limits=blob_limits, staging=staging
        )
        connection = sqlite3.connect(path)
        try:
            with connection:
                for table, ddl, count in _tables(rows):
                    connection.execute(f"DROP TABLE IF EXISTS {table}")
                    connection.execute(ddl)
                    if table == "blobs":
                        for digest, length, handle in blobs:
                            cursor = connection.execute(
                                "INSERT INTO blobs VALUES (?, zeroblob(?))", (digest, length)
                            )
                            with connection.blobopen("blobs", "bytes", cursor.lastrowid) as target:
                                offset = 0
                                while offset < length:
                                    chunk = staging.read(handle, offset, min(32768, length - offset))
                                    if not chunk:
                                        raise OSError("staged GTS blob ended before its admitted length")
                                    target.write(chunk)
                                    offset += len(chunk)
                    else:
                        placeholders = ", ".join("?" * count)
                        connection.executemany(
                            f"INSERT INTO {table} VALUES ({placeholders})", rows[table]
                        )
        finally:
            connection.close()
    return GtsExportResult((path,), report)


def gts_to_duckdb(data: bytes, path: str, *, blob_limits: Any) -> GtsExportResult:
    """Write canonical native v1 rows to DuckDB; return paths and admission report.

    Requires ``pip install 'purrdf[duckdb]'``. Existing tables are replaced
    together; a failed insertion rolls the transaction back.
    """
    duckdb = _require("duckdb", "duckdb")
    with TemporaryDirectory(prefix="purrdf-duckdb-") as directory:
        projected = gts_to_parquet(data, directory, blob_limits=blob_limits)
        connection = duckdb.connect(path)
        try:
            connection.execute("BEGIN TRANSACTION")
            try:
                for source in projected.paths:
                    table = Path(source).stem
                    connection.execute(f"DROP TABLE IF EXISTS {table}")
                    connection.execute(
                        f"CREATE TABLE {table} AS SELECT * FROM read_parquet(?)", [source]
                    )
                connection.execute("COMMIT")
            except BaseException:
                connection.execute("ROLLBACK")
                raise
        finally:
            connection.close()
    return GtsExportResult((path,), projected.report)


def gts_to_parquet(data: bytes, out_dir: str, *, blob_limits: Any) -> GtsExportResult:
    """Write first-party native v1 Parquet files in canonical table order.

    No optional encoder dependency is required. The output includes all five
    files even for an empty dataset, with complete schemas and native metadata.
    """
    from .purrdf_native import rdf as _native

    directory = Path(out_dir)
    directory.mkdir(parents=True, exist_ok=True)
    with _staging() as staging, TemporaryDirectory(
        prefix=".purrdf-gts-", dir=directory
    ) as provisional:
        with ExitStack() as stack:
            files = {}

            def emit(name: str, chunk: bytes) -> None:
                if name not in files:
                    files[name] = stack.enter_context((Path(provisional) / name).open("wb"))
                _write_chunk(files[name], chunk)

            report = _native.gts_columnar_stream_to(
                data, blob_limits=blob_limits, staging=staging, emit=emit,
            )
        written = []
        for name in files:
            target = directory / name
            (Path(provisional) / name).replace(target)
            written.append(str(target))
    return GtsExportResult(tuple(written), report)


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
