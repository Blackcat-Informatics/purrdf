# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Report-only benchmark: ``purrdf.compat.rdflib`` vs. the real ``rdflib``.

This harness times the core RDF operations for BOTH engines in a single
process and prints a side-by-side table (with the purrdf/rdflib ratio) plus an
optional machine-readable JSON dump. It is **report-only**: per AGENTS.md §4
("measure, never assert") it makes no claim of a speedup and is never wired into
``make check`` or ``pytest``. Run it with ``make bench-python``.

Both engines are imported directly:

* the oracle  — ``import rdflib`` (the genuine library, dev-group dependency)
* the shim    — ``import purrdf.compat.rdflib`` (native-backed compat surface)

The corpus is generated **deterministically** from the triple index (no
``random`` and no wall-clock in the data itself) so every run over a given size
is byte-for-byte identical and the two engines see exactly the same input. Only
``time.perf_counter`` observes wall-clock, and we report the best-of / median of
several repetitions. Fixtures use ``example.org`` (never a minted vocabulary).

Numbers are host-dependent illustrations, not a guarantee — see
``docs/BENCHMARKS.md``.
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
import time
from collections.abc import Callable, Iterable
from dataclasses import asdict, dataclass
from types import ModuleType
from typing import Any

# ── deterministic corpus ────────────────────────────────────────────────────────

EX = "http://example.org/"
XSD_INT = "http://www.w3.org/2001/XMLSchema#integer"
NUM_PREDICATES = 8
NUM_TYPES = 5


def _triple_line(i: int) -> str:
    """Return the ``i``-th N-Triples line, derived purely from ``i``.

    The value is a closed-form function of the index — no RNG, no clock — so the
    corpus is identical on every run and both engines parse the same bytes. We
    interleave three shapes (a typed literal, an ``rdf:type``, and an object
    reference to a neighbouring subject) so BGP joins, filters, and aggregates
    all have something to chew on.
    """
    subj = f"<{EX}s{i}>"
    shape = i % 3
    if shape == 0:
        pred = f"<{EX}p{i % NUM_PREDICATES}>"
        obj = f'"{i}"^^<{XSD_INT}>'
    elif shape == 1:
        pred = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"
        obj = f"<{EX}Type{i % NUM_TYPES}>"
    else:
        pred = f"<{EX}ref>"
        obj = f"<{EX}s{(i * 2654435761) % (i + 1)}>"
    return f"{subj} {pred} {obj} ."


def build_ntriples(n: int) -> str:
    """Return a deterministic N-Triples document with ``n`` triples."""
    return "\n".join(_triple_line(i) for i in range(n)) + "\n"


# ── timing core ─────────────────────────────────────────────────────────────────


@dataclass(frozen=True)
class Measurement:
    """Timing summary for one (engine, operation, size) cell, in milliseconds."""

    engine: str
    operation: str
    size: int
    best_ms: float
    median_ms: float
    repetitions: int


def _time_call(fn: Callable[[], object], repetitions: int) -> tuple[float, float]:
    """Return ``(best_ms, median_ms)`` for ``fn`` over ``repetitions`` runs."""
    samples: list[float] = []
    for _ in range(repetitions):
        start = time.perf_counter()
        fn()
        samples.append((time.perf_counter() - start) * 1000.0)
    return min(samples), statistics.median(samples)


# ── per-engine operation closures ───────────────────────────────────────────────

# A BGP + join query: subjects that both carry a typed value and an rdf:type.
QUERY_BGP = (
    "PREFIX ex: <http://example.org/> "
    "PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> "
    "SELECT ?s ?v ?t WHERE { ?s ex:p0 ?v . ?s rdf:type ?t } LIMIT 100"
)

# A filter + aggregate query: count typed integer values above a threshold.
QUERY_AGG = (
    "PREFIX ex: <http://example.org/> "
    "SELECT (COUNT(?v) AS ?n) (AVG(?v) AS ?avg) WHERE { "
    "?s ex:p0 ?v . FILTER(?v > 100) }"
)


def _operations(
    engine: ModuleType, nt_data: str, ttl_data: str
) -> dict[str, Callable[[], object]]:
    """Return the named operation closures for one engine module.

    Each closure is self-contained (builds its own graph where a fresh one is
    needed) so repetitions do not accumulate state across runs.
    """
    # ``engine`` is a module, so ``Graph`` and the graph instance are untyped
    # (``Any``) across the two engines; that is expected for a cross-library harness.
    graph_cls: Any = engine.Graph

    def parse_nt() -> object:
        g = graph_cls()
        g.parse(data=nt_data, format="nt")
        return g

    def parse_ttl() -> object:
        g = graph_cls()
        g.parse(data=ttl_data, format="turtle")
        return g

    # A pre-parsed graph reused by the read-only operations (serialize/query/iter).
    loaded: Any = parse_nt()

    def serialize_nt() -> object:
        return loaded.serialize(format="nt")

    def serialize_ttl() -> object:
        return loaded.serialize(format="turtle")

    def query_bgp() -> object:
        return list(loaded.query(QUERY_BGP))

    def query_agg() -> object:
        return list(loaded.query(QUERY_AGG))

    def triples_scan() -> object:
        return sum(1 for _ in loaded.triples((None, None, None)))

    return {
        "parse_nt": parse_nt,
        "parse_ttl": parse_ttl,
        "serialize_nt": serialize_nt,
        "serialize_ttl": serialize_ttl,
        "query_bgp": query_bgp,
        "query_agg": query_agg,
        "triples_scan": triples_scan,
    }


OPERATION_ORDER = (
    "parse_nt",
    "parse_ttl",
    "serialize_nt",
    "serialize_ttl",
    "query_bgp",
    "query_agg",
    "triples_scan",
)


# ── run + report ────────────────────────────────────────────────────────────────


def _load_engines() -> dict[str, ModuleType]:
    """Import and return both engines: the purrdf shim and the real rdflib."""
    import rdflib as real_rdflib
    import purrdf.compat.rdflib as compat_rdflib

    return {"purrdf": compat_rdflib, "rdflib": real_rdflib}


def _turtle_from_nt(engine: ModuleType, nt_data: str) -> str:
    """Round-trip the N-Triples corpus into Turtle via the given engine."""
    g = engine.Graph()
    g.parse(data=nt_data, format="nt")
    out = g.serialize(format="turtle")
    return out if isinstance(out, str) else out.decode("utf-8")


def run(sizes: Iterable[int], repetitions: int) -> list[Measurement]:
    """Time every (engine, operation, size) cell and return the measurements."""
    engines = _load_engines()
    results: list[Measurement] = []
    for size in sizes:
        nt_data = build_ntriples(size)
        # Generate Turtle from the real rdflib so both engines parse identical
        # Turtle bytes (the shim's canonical Turtle differs syntactically).
        ttl_data = _turtle_from_nt(engines["rdflib"], nt_data)
        for engine_name, engine in engines.items():
            ops = _operations(engine, nt_data, ttl_data)
            for op_name in OPERATION_ORDER:
                best, median = _time_call(ops[op_name], repetitions)
                results.append(
                    Measurement(
                        engine=engine_name,
                        operation=op_name,
                        size=size,
                        best_ms=best,
                        median_ms=median,
                        repetitions=repetitions,
                    )
                )
    return results


def _table(results: list[Measurement]) -> str:
    """Render the measurements as a side-by-side text table (best-of ms)."""
    by_key: dict[tuple[int, str], dict[str, float]] = {}
    for m in results:
        by_key.setdefault((m.size, m.operation), {})[m.engine] = m.best_ms

    header = (
        f"{'size':>8}  {'operation':<15}  "
        f"{'purrdf ms':>12}  {'rdflib ms':>12}  {'ratio (p/r)':>12}"
    )
    lines = [header, "-" * len(header)]
    for size in sorted({s for s, _ in by_key}):
        for op in OPERATION_ORDER:
            cell = by_key.get((size, op))
            if cell is None:
                continue
            p = cell.get("purrdf")
            r = cell.get("rdflib")
            ratio = f"{p / r:>12.3f}" if p and r else f"{'n/a':>12}"
            p_s = f"{p:>12.3f}" if p is not None else f"{'n/a':>12}"
            r_s = f"{r:>12.3f}" if r is not None else f"{'n/a':>12}"
            lines.append(f"{size:>8}  {op:<15}  {p_s}  {r_s}  {ratio}")
    lines.append("")
    lines.append(
        "ratio < 1.0 => purrdf shim faster; > 1.0 => real rdflib faster. "
        "Host-dependent; report-only (AGENTS.md §4)."
    )
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    """CLI entry point: parse args, run the harness, print the report."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--sizes",
        type=int,
        nargs="+",
        default=[1_000, 10_000, 100_000],
        help="corpus sizes in triples (default: 1000 10000 100000)",
    )
    parser.add_argument(
        "--repetitions",
        type=int,
        default=5,
        help="timed repetitions per cell; best-of and median reported (default: 5)",
    )
    parser.add_argument(
        "--json",
        type=str,
        default=None,
        help="optional path to write machine-readable JSON results",
    )
    args = parser.parse_args(argv)

    results = run(args.sizes, args.repetitions)
    print(_table(results))

    if args.json is not None:
        payload = {
            "schema": "purrdf.bench_compat/1",
            "sizes": list(args.sizes),
            "repetitions": args.repetitions,
            "measurements": [asdict(m) for m in results],
        }
        with open(args.json, "w", encoding="utf-8") as fh:
            json.dump(payload, fh, indent=2, sort_keys=True)
            fh.write("\n")
        print(f"\nwrote JSON results to {args.json}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
