from pathlib import Path
import difflib

stage=Path(__file__).parent
root=stage.parents[2]
changes=[]
for path in sorted((stage/'crates').rglob('*.rs')):
    relative=path.relative_to(stage)
    before=(root/relative).read_text().splitlines(keepends=True)
    after=path.read_text().splitlines(keepends=True)
    changes.extend(difflib.unified_diff(before,after,fromfile='a/'+str(relative),tofile='b/'+str(relative)))
(stage.parent/'native-prepare-cache-callers-owner-draft.patch').write_text(''.join(changes))
