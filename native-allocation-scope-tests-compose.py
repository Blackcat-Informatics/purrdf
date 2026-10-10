from pathlib import Path
import difflib
root = Path(__file__).resolve().parents[2]
stage = Path(__file__).resolve().parent
relative = 'crates/lex/src/allocation.rs'
before = root.joinpath(relative).read_text()
after = before + stage.joinpath('native-allocation-scope-fixtures.rs').read_text()
stage.joinpath('native-allocation-original-scope-tests.patch').write_text(''.join(difflib.unified_diff(before.splitlines(True), after.splitlines(True), fromfile='a/'+relative, tofile='b/'+relative)))
print('original allocation scope fixtures ready')
