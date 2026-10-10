from pathlib import Path
import difflib
import hashlib
import json

root = Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
stage = root / '.stage/sparql-eval-complete-bounded-workspace'
post = stage / 'evaluator-clippy-correction-5-postimages'
identities = json.loads((stage / 'evaluator-clippy-correction-5-source-identity.json').read_text())
patch = []
for record in identities:
    path = record['path']
    original = (root / path).read_bytes()
    assert hashlib.sha256(original).hexdigest() == record['source_sha256'], f'changed source: {path}'
    proposed = (post / path).read_bytes()
    record['postimage_sha256'] = hashlib.sha256(proposed).hexdigest()
    patch.extend(difflib.unified_diff(original.decode().splitlines(True), proposed.decode().splitlines(True), fromfile='a/' + path, tofile='b/' + path))
(stage / 'evaluator-clippy-correction-5.patch').write_text(''.join(patch))
(stage / 'evaluator-clippy-correction-5-source-identity.json').write_text(json.dumps(identities, indent=2) + '\n')
findings = json.loads((stage / 'evaluator-clippy-correction-5-findings.json').read_text())
for record in identities:
    name = Path(record['path']).name
    owned = [f for f in findings if f['file'] == name]
    lints = sorted({f['lint'] for f in owned})
    print('{}: {} diagnostics; {}'.format(name, len(owned), ', '.join(lints)))
print('{} files, {} diagnostic records, {} patch lines'.format(len(identities), len(findings), len(patch)))
