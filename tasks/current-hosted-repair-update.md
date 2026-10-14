Published `bd4d219bf` removes the two obsolete bounded AVG exports, corrects
public rustdoc for exact decimals and temporal comparison, and fixes the three
needless closure borrows reported by current hosted Clippy. The original AVG
consumers already use exact `numeric_div_with_policy`; the old implementations
remain private test controls. Breaking guidance names the removed helpers.

Affected all-target Clippy, warning-denied public rustdoc, metadata/license
regeneration and normal commit hooks pass. The original temporal peak admission,
refusal and release test passes after naming its synthetic counters accurately
as an allocation budget. Its assertions and the CodeQL policy are unchanged.

The repaired full local gate and 458-test installed WASM package passed on the
previous published source, with 42 current installed Python controls. The new
hosted run is in progress. The baseline assembly floor passed but its generated
document cell was stale (43 versus 42); the existing complete seven-configuration
projection producer is running at
https://github.com/Blackcat-Informatics/purrdf/actions/runs/38592881046.
Its patch, ordinary hosted parity, current reviews and final integration still
need acceptance. No merge or issue closure is claimed.
