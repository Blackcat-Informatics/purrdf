# Handwritten conformance count correction

Published20ede055f corrects the separate handwritten Python binding scoreboard
from4218 to the actual4223. Both the generated matrix and prose now agree with
all four retained hosted shards; Python reports4223 passes/four strict xfails,
not merely the compatibility differential. No tooling or test expectation changes.

Existing `python3 scripts/check-doc-claims.py` terminal0:154 claims agree.
Independent scoped source review PASS in reviews/restated-conformance-count-review.md.
Normal commit7536 and push68515 terminal0, retained logs adjacent.

Predecessor CI38423608191 is terminal FAILURE:37 jobs pass, including conformance,
all measured assembly configurations and architecture execution. Workspace failed
only on the now-corrected handwritten count. Optimized package446/449 still fails
the shared synchronous/LATERAL/NOT EXISTS floors135/236/134 versus181/281/180.
That production repair remains incomplete. Corrected-head CI and final integration
are pending; no issue closure or whole-delivery pass is claimed.
