from pathlib import Path
import difflib

root = Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
stage = root / '.stage/sparql-eval-complete-bounded-workspace'
path = 'crates/rdf-core/tests/native_regex_storage.rs'
before = (root / path).read_text()
after = before.replace('// Append to native_regex_storage.rs, reusing its counting allocator and Account.\n\n', '')
after = after.replace('    drop(memory);\n', '')
patch = 'diff --git a/' + path + ' b/' + path + '\n' + ''.join(difflib.unified_diff(
    before.splitlines(True), after.splitlines(True), fromfile='a/' + path, tofile='b/' + path))
(stage / 'expand-uchars-fixture-cleanup.patch').write_text(patch)
