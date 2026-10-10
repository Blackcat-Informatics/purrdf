from pathlib import Path
import difflib
root = Path(__file__).resolve().parents[2]
stage = Path(__file__).resolve().parent
patches = []
for relative in ['crates/cdt/tests/value_relations.rs', 'crates/cdt/Cargo.toml']:
    before = root.joinpath(relative).read_text()
    if relative.endswith('.rs'):
        after = before + stage.joinpath('native-cdt-relations-fixtures.rs').read_text()
    else:
        after = before + '\n# Real allocator evidence for original native storage admission.\npurrdf-alloc-probe = { workspace = true }\n'
    patches.append(''.join(difflib.unified_diff(before.splitlines(True), after.splitlines(True), fromfile='a/'+relative, tofile='b/'+relative)))
stage.joinpath('native-cdt-relations-original-grant-tests.patch').write_text(''.join(patches))
print('native CDT relation allocator packet ready')
