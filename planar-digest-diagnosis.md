# Planar digest diagnosis

The actual diagnostic run in `planar-byte-differential-3.log` compiled and
executed the unchanged accepted implementation and the native successor using
the original twenty-member corpus. The original produced the frozen value
`9667c2ee2cd3ad4b`; the successor produced `cae63e406267a3d7`. Both folded exactly
1,057 inputs. The diagnostic stopped on the ten real byte differences, so its
terminal status is intentionally failure, not qualification.

Exactly five planar `length` decimals and their five double bit encodings
changed. All WKT, GeoJSON, DE-9IM, area, perimeter, distance, constructor and
other double inputs matched byte for byte.

| Corpus geometry | Old length | Native length |
| --- | --- | --- |
| Unit square | 0 | 4 |
| Reversed unit square | 0 | 4 |
| Square with hole | 0 | 20 |
| Nested point/line/polygon collection | 1.414213562373095048 | 3.414213562373095048 |
| Two-triangle multipolygon | 0 | 6.828427124746190096 |

The old `crates/geo/src/measure.rs::length` sums curve components only. The
successor's sole `measure::length` sums curve and polygon-boundary lengths
separately, then returns the greater dimension-total. This implements
[OGC 22-047r1 sections 10.9.24 and 10.9.25](https://docs.ogc.org/is/22-047r1/22-047r1.html):
the length functions include polygon boundaries and select the longest length
from one dimension. The official specification was read for this diagnosis.

No shipping semantic change, corpus change, expected-value change, limit
increase or receipt regeneration was made to obtain this diagnostic result.
The root adjudicated the ten differences as the required standards correction:
the first-party aggregate pin may change after preserving the old digest and
all five independent calculations. Reverting the native length law would
restore the observed standards defect.

The shipping pin in `crates/geo/tests/determinism.rs` and its existing
conformance-board specimen in `scripts/conformance-matrix.py` now name
`cae63e406267a3d7`. The original geometry corpus remains unchanged. The native
`measure` fixture independently calculates all five lengths and pins their
double encodings; the separate mixed-dimensional fixture also retains its
independent expected totals. No official/vendor/wire corpus was changed.
Shipping runtime qualification of this update remains pending the consolidated
matrix. The negative diagnostic terminal itself is not qualification.

The Stage-only diagnostic borrows all original geometry operation modules.
Its captured digest files only replace the fold import with a recorder, and
the captured original geometry file only supplies its original child-module
path. These diagnostic sources are never linked into a shipping library and
do not introduce a second production implementation.
