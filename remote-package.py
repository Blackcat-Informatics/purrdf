from pathlib import Path
import difflib
stage=Path(__file__).parent;root=stage.parent.parent;out=stage/'remote-native-owner-postimages'
p=out/'crates/sparql-algebra/src/serialize.rs';s=p.read_text()
s=s.replace('    let storage = RenderStorage::new(&mut memory);\n    emit(&mut Surface { text: s,', '    memory.add_bytes(s.capacity()).expect("resident caller-owned buffer layout");\n    let storage = RenderStorage::new(&mut memory);\n    emit(&mut Surface { text: s,')
p.write_text(s)
# Engine error construction widened only at the existing declaration; use the
# CURRENT source for these shared homes so no sibling writer body is reverted.
name='crates/sparql-eval/src/engine.rs';s=(root/name).read_text().replace('\nfn parse_admitted_query(','\npub(crate) fn parse_admitted_query(',1);(out/name).write_text(s)
# Prepare one ordinary, apply-checkable diff; do not touch the shipping index.
parts=[]
for path in sorted(out.rglob('*.rs')):
    relative=path.relative_to(out)
    before=(root/relative).read_text() if (root/relative).exists() else ''
    after=path.read_text()
    if before!=after:
        parts.extend(difflib.unified_diff(before.splitlines(True),after.splitlines(True),fromfile='a/'+str(relative),tofile='b/'+str(relative)))
(stage/'remote-native-owner-draft.patch').write_text(''.join(parts))
print('ordinary native SERVICE packet:',len(''.join(parts)),'bytes')
