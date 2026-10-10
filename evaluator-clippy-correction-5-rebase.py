from pathlib import Path
import hashlib
import json

root = Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
stage = root / '.stage/sparql-eval-complete-bounded-workspace'
path = 'crates/sparql-eval/src/expr.rs'
target = stage / 'evaluator-clippy-correction-5-postimages' / path
before = '''            if let Some(cached) = ctx.xsd_parse_cache.get(id) {
                return Ok(cached);
            }'''
after = '''            if let Some(cached) = ctx.xsd_parse_cache.get(id) {
                return Ok(cached.into_value());
            }'''
text = target.read_text()
assert text.count(before) == 1
assert (root / path).read_text().count(after) == 1
target.write_text(text.replace(before, after))
identities = json.loads((stage / 'evaluator-clippy-correction-5-source-identity.json').read_text())
for record in identities:
    if record['path'] == path:
        record['source_sha256'] = hashlib.sha256((root / path).read_bytes()).hexdigest()
(stage / 'evaluator-clippy-correction-5-source-identity.json').write_text(json.dumps(identities, indent=2) + '\n')
print('preserved the integrated numeric cache-hit carrier')
