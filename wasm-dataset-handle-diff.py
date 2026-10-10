from pathlib import Path
import difflib
import hashlib
import json

stage = Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace/.stage/sparql-eval-complete-bounded-workspace')
identities = json.loads((stage / 'wasm-dataset-handle-source-identities.json').read_text())
diff = []
for path, identity in identities.items():
    before = (stage / 'wasm-dataset-handle-preimages' / path).read_text()
    after = (stage / 'wasm-dataset-handle-postimages' / path).read_text()
    identity['postimage_sha256'] = hashlib.sha256(after.encode()).hexdigest()
    diff.extend(difflib.unified_diff(before.splitlines(True), after.splitlines(True),
                 fromfile='a/' + path, tofile='b/' + path))
(stage / 'wasm-dataset-handle-owner-draft.patch').write_text(''.join(diff))
(stage / 'wasm-dataset-handle-source-identities.json').write_text(json.dumps(identities, indent=2) + '\n')
