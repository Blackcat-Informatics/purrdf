from pathlib import Path
import difflib
stage = Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace/.stage/sparql-eval-complete-bounded-workspace')
patch = []
for baseline, postimage, target in [
    ('modifier-visibility-native-baseline.rs', 'modifier-visibility-native-postimage.rs', 'crates/sparql-eval/src/modifier.rs'),
    ('blank-scope-visibility-native-baseline.rs', 'blank-scope-visibility-native-postimage.rs', 'crates/sparql-eval/src/blank_scope.rs'),
]:
    patch.extend(difflib.unified_diff((stage/baseline).read_text().splitlines(True), (stage/postimage).read_text().splitlines(True), 'a/'+target, 'b/'+target))
(stage/'custom-aggregate-visibility-owner-supplement.patch').write_text(''.join(patch))
