from pathlib import Path
import difflib
stage=Path(__file__).parent
root=stage.parents[2]
parts=[]
for path in sorted((stage/'crates').rglob('*.rs')):
 rel=path.relative_to(stage).as_posix()
 source=root/rel
 old=source.read_text() if source.exists() else ''
 new=path.read_text()
 if old==new: continue
 parts.append(f'diff --git a/{rel} b/{rel}\n')
 if not source.exists(): parts.append('new file mode 100644\n')
 parts.extend(difflib.unified_diff(old.splitlines(True),new.splitlines(True),
     fromfile='a/'+rel if source.exists() else '/dev/null',tofile='b/'+rel))
(stage.parent/'general-algebra-drop-owner-draft.patch').write_text(''.join(parts))
print('Wrote complete four-home cleanup/constructor packet.')
