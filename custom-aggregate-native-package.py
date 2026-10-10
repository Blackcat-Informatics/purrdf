from pathlib import Path
import difflib

stage = Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace/.stage/sparql-eval-complete-bounded-workspace')
patch = []
for baseline, postimage, target in [
    ('modifier-custom-native-baseline.rs', 'modifier-custom-native-postimage.rs', 'crates/sparql-eval/src/modifier.rs'),
    ('modifier-custom-owners-baseline.rs', 'modifier-custom-owners-postimage.rs', 'crates/sparql-eval/src/modifier/owners.rs'),
]:
    old = (stage/baseline).read_text()
    new = (stage/postimage).read_text()
    patch.extend(difflib.unified_diff(old.splitlines(True), new.splitlines(True), 'a/'+target, 'b/'+target))
fixture = (stage/'custom-aggregate-owned-entry-fixtures.rs').read_text()
patch.extend(difflib.unified_diff([], fixture.splitlines(True), '/dev/null', 'b/crates/sparql-eval/tests/custom_aggregate_ownership.rs'))
(stage/'custom-aggregate-native-owner-draft.patch').write_text(''.join(patch))
