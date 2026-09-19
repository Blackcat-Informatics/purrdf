#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""The LUBM comparison lane (`make bench-lubm`): fetch, generate, run.

Everything upstream is fetched **by digest and never vendored**: the UBA 1.7
generator is GPL-2.0-or-later (we run it, we do not ship it), and the
univ-bench ontology carries no license text at all, so redistribution is not
admitted. Upstream publishes no checksums; the pins below are this project's
own provenance record, taken 2026-09-18.

The lane is a REPORT-ONLY comparison workload, never a gate: timings are
recorded evidence. Generation is reproducible by construction (UBA reseeds
per university over Java's spec-fixed LCG) once the mandatory Linux path fix
is applied — which requires recompiling one class, hence a JDK.

Query normalization is mechanical and recorded: the 14 queries predate final
SPARQL (comma-separated projection lists) and reference the pre-final
ontology IRI; both rewrites are pure text substitutions whose output digest
is printed so a capture can name exactly what it executed. Each query carries
its entailment regime from the canonical LUBM paper; engines compare only
within matching regimes.
"""

import argparse
import hashlib
import re
import shutil
import subprocess
import sys
import urllib.request
import zipfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CACHE = REPO_ROOT / "target" / "bench" / "lubm"

BASE = "https://swat.cse.lehigh.edu"
# (relative url, filename, sha256) — our pins; upstream publishes none.
ARTIFACTS = [
    ("/projects/lubm/uba1.7.zip", "uba1.7.zip",
     "3d44f468e36b7f3cd532f0f5693a019d020877846397b1fa657367cbd53f380a"),
    ("/projects/lubm/GeneratorLinuxFix.zip", "GeneratorLinuxFix.zip",
     "cc92e7a8373306086c593b519b15a5991f859d40e220e04cebb994d0cdc44be4"),
    ("/projects/lubm/queries-sparql.txt", "queries-sparql.txt",
     "c34fd26ecb6fb9f0a2f185d73b72505ef0abf25705d831e6eed3c896557cd104"),
    ("/onto/univ-bench.owl", "univ-bench.owl",
     "e6eca926fcb7d6c7925ea0c48f7c5d79abc2818f89e7432fd16561aafeb3f67a"),
]

# The ontology IRI the queries were written against, and the one the data is
# generated under (the live ontology location, passed to UBA via -onto).
QUERY_ONTO = "http://www.lehigh.edu/~zhp2/2004/0401/univ-bench.owl"
DATA_ONTO = "http://swat.cse.lehigh.edu/onto/univ-bench.owl"

# Entailment regime per query, from the canonical JWS 2005 paper (App. 1).
# Engines are compared only within matching regimes.
REGIMES = {
    1: "none", 2: "none", 14: "none",
    3: "rdfs-subclass", 4: "rdfs-subclass",
    5: "rdfs-subclass+subproperty",
    6: "owl-implicit-subsumption", 7: "owl-implicit-subsumption",
    8: "owl-implicit-subsumption", 9: "owl-implicit-subsumption",
    10: "owl-implicit-subsumption",
    11: "owl-transitive",
    12: "owl-realization",
    13: "owl-inverse+subproperty",
}


def sha256_of(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def fetch() -> None:
    CACHE.mkdir(parents=True, exist_ok=True)
    for relative, name, expected in ARTIFACTS:
        target = CACHE / name
        if not target.exists():
            print(f"fetching {BASE}{relative}")
            with urllib.request.urlopen(BASE + relative) as response:  # noqa: S310
                target.write_bytes(response.read())
        actual = sha256_of(target)
        if actual != expected:
            sys.exit(f"FAIL: {name} digest mismatch\n  expected {expected}\n  actual   {actual}")
        print(f"OK: {name} {expected[:16]}…")


def prepare() -> Path:
    """Unpack UBA, apply the Linux path fix, recompile (requires a JDK)."""
    if shutil.which("javac") is None:
        sys.exit(
            "FAIL: `javac` not found — the mandatory Linux path fix replaces "
            "Generator.java and must be recompiled; install a JDK (a JRE is "
            "not enough) and re-run"
        )
    work = CACHE / "uba"
    if work.exists():
        shutil.rmtree(work)
    with zipfile.ZipFile(CACHE / "uba1.7.zip") as archive:
        archive.extractall(work)
    with zipfile.ZipFile(CACHE / "GeneratorLinuxFix.zip") as archive:
        archive.extractall(CACHE / "linuxfix")
    fixed = next((CACHE / "linuxfix").rglob("Generator.java"), None)
    original = next(work.rglob("Generator.java"), None)
    if fixed is None or original is None:
        sys.exit("FAIL: expected Generator.java in both archives")
    shutil.copyfile(fixed, original)
    sources = [str(p) for p in work.rglob("*.java")]
    subprocess.run(["javac", *sources], check=True, cwd=work)
    print(f"OK: UBA prepared (Linux fix applied, {len(sources)} sources compiled)")
    return work


def generate(universities: int, seed: int) -> Path:
    work = prepare()
    out = CACHE / f"data-univ{universities}-seed{seed}"
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)
    class_root = next(work.rglob("Generator.class"), None)
    if class_root is None:
        sys.exit("FAIL: compiled Generator.class not found")
    # UBA's package is edu.lehigh.swat.bench.uba; classpath is the tree root.
    depth = len("edu/lehigh/swat/bench/uba".split("/"))
    classpath = class_root.parents[depth - 1]
    subprocess.run(
        [
            "java", "-cp", str(classpath), "edu.lehigh.swat.bench.uba.Generator",
            "-univ", str(universities), "-index", "0", "-seed", str(seed),
            "-onto", DATA_ONTO,
        ],
        check=True,
        cwd=out,
    )
    files = sorted(out.glob("*.owl"))
    print(f"OK: generated {len(files)} RDF/XML files under {out.name}/")
    return out


def convert(data_dir: Path) -> Path:
    """RDF/XML department files → one N-Quads corpus, through the purrdf CLI."""
    merged = data_dir.with_suffix(".nq")
    inputs: list[str] = []
    for path in sorted(data_dir.glob("*.owl")):
        inputs.extend(["--input", str(path)])
    if not inputs:
        sys.exit(f"FAIL: no RDF/XML files under {data_dir}")
    first = inputs.pop(1)
    inputs.pop(0)
    subprocess.run(
        ["cargo", "run", "--quiet", "--release", "-p", "purrdf-cli", "--",
         "convert", "--from", "rdfxml", "--to", "nquads",
         *inputs, first, str(merged)],
        check=True,
        cwd=REPO_ROOT,
    )
    print(f"OK: corpus {merged.name} sha256 {sha256_of(merged)}")
    return merged


def normalized_queries() -> list[tuple[int, str]]:
    text = (CACHE / "queries-sparql.txt").read_text(encoding="utf-8")
    text = text.replace(QUERY_ONTO, DATA_ONTO)
    # Pre-final SPARQL wrote `SELECT ?X, ?Y` — the commas must go.
    text = re.sub(r"(\?\w+)\s*,", r"\1 ", text)
    digest = hashlib.sha256(text.encode()).hexdigest()
    print(f"OK: queries normalized (ontology IRI + projection commas); sha256 {digest}")
    queries: list[tuple[int, str]] = []
    # A delimiter is `# QueryN` with nothing after the number — the file also
    # contains prose comments beginning `# Query 11, 12 and 13 …` that must
    # not split.
    for block in re.split(r"^#[ \t]*Query(?=\d+[ \t]*\r?$)", text, flags=re.MULTILINE)[1:]:
        numbered = re.match(r"(\d+)", block)
        if numbered is None:
            sys.exit("FAIL: query block without a number — normalization drifted")
        body = block.split("\n", 1)[1].strip()
        queries.append((int(numbered.group(1)), body))
    if len(queries) != 14:
        sys.exit(f"FAIL: expected 14 queries, parsed {len(queries)}")
    return queries


def run(corpus: Path) -> None:
    import json
    import time

    rows = []
    for number, body in normalized_queries():
        query_file = CACHE / f"q{number:02}.rq"
        query_file.write_text(body, encoding="utf-8")
        started = time.monotonic()
        completed = subprocess.run(
            ["cargo", "run", "--quiet", "--release", "-p", "purrdf-cli", "--",
             "query", str(corpus), str(query_file),
             "--results-format", "csv"],
            capture_output=True,
            text=True,
            cwd=REPO_ROOT,
            check=False,
        )
        elapsed_ms = int((time.monotonic() - started) * 1000)
        answer_rows = max(completed.stdout.count("\n") - 1, 0)
        rows.append({
            "query": number,
            "regime": REGIMES[number],
            "ok": completed.returncode == 0,
            "rows": answer_rows,
            "wall_ms_report_only": elapsed_ms,
        })
        if completed.returncode != 0:
            print(f"q{number:02}: FAILED\n{completed.stderr.strip()[:500]}", file=sys.stderr)
    print(json.dumps({"suite": "lubm", "corpus_sha256": sha256_of(corpus), "results": rows}))


def self_test() -> int:
    CACHE.mkdir(parents=True, exist_ok=True)
    probe = CACHE / "self-test.bin"
    probe.write_bytes(b"not an artifact")
    if sha256_of(probe) == ARTIFACTS[0][2]:
        print("SELF-TEST FAIL: wrong bytes matched a pin")
        return 1
    if len(REGIMES) != 14:
        print("SELF-TEST FAIL: regime table must cover all 14 queries")
        return 1
    print("OK: bench-lubm self-test (digest mismatch detectable, regimes complete)")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--fetch", action="store_true")
    parser.add_argument("--generate", type=int, metavar="UNIVERSITIES")
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--run", type=Path, metavar="CORPUS_NQ")
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    if args.fetch:
        fetch()
    if args.generate is not None:
        fetch()
        corpus = convert(generate(args.generate, args.seed))
        print(f"corpus ready: {corpus}")
    if args.run is not None:
        fetch()
        run(args.run)
    return 0


if __name__ == "__main__":
    sys.exit(main())
