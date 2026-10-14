# Hosted conformance projection review — 2026-10-13

**Verdict: the documentation-only projection is supported by the four retained hosted conformance shards, within that scope.** This is not a full CI or issue #542 completion verdict.

Reviewed published source `8360fff43` plus the sole working-tree source diff, `docs/CONFORMANCE.md`: its only changed value is the Python binding suite pass count, `4218` to `4223`. The retained `python.json` row at index 31 reports `passed=4223`, `xskip=4`, `failed=0`, `ok=true`, `scoreboard_missing=false`; its captured test output ends `4223 passed, 1 skipped, 4 xfailed`. The other Python row reports 81 passed and five strict xfails.

The retained run `38372155868` inputs under `hosted-conformance-8360/` contain exactly the four expected format `purrdf-conformance-shard/1` files: core (12 rows), python (2), shapes (9), and sparql (9). Their disjoint indices cover 0–31, all rows report `ok=true`, `scoreboard_missing=false`, and zero failed; combined they report 16,358 passes and 16 xfail/skip. `scripts/conformance-matrix.py::merge_shards` requires exactly these named shards, their owned indices and full row fields. The retained `conformance-projection-8360.log` records the existing `--from-results ... --write-doc` projection as terminal zero, totals 16,358/16/0 and `wrote matrix block to docs/CONFORMANCE.md`; the resulting source diff is only the count above. No suite was rerun for this review.

This supports refreshing the matrix from the measured hosted shard artifacts. It does not establish the unrelated hosted wasm-package check, which failed on shared #531, nor a global CI pass, final source integration, or issue closure.
