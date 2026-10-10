# Why not Rust: This evidence replay invokes the shipped Python assembly document generator verbatim.
"""Replay seven authoritative hosted measurement columns; do not qualify changed source."""
import importlib.util
import json
from pathlib import Path
import re
import sys
import tomllib

root = Path.cwd()
stage = root / '.stage/owl-propertychainaxiom-is-a-property'
sys.path.insert(0, str(root / 'scripts'))
spec = importlib.util.spec_from_file_location('simd_gate', root / 'scripts/check-simd-asm.py')
gate = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = gate
spec.loader.exec_module(gate)
manifest = gate.load_manifest(tomllib.loads(gate.MANIFEST.read_text()))
cells = {}
identity = None
for name in gate.CONFIG_NAMES:
    candidates = list((stage / 'hosted-asm-original').rglob(name + '.json'))
    assert len(candidates) == 1, (name, candidates)
    report = json.loads(candidates[0].read_text())
    identity = report['identity'] if identity is None else identity
    assert report['identity'] == identity, name
    if name in ('x86_64-v3', 'x86_64-v4'):
        assert report['status'] == 'incomplete' and report['cells'] == {}, name
        log = (stage / ('review-debt-simd-' + name.removeprefix('x86_64-') + '.log')).read_text(encoding='utf-8-sig')
        column = {}
        for line in log.splitlines():
            match = re.match(r'^\S+Z (\S+)\s{2,}(.+)$', line)
            if match and match[1] in manifest.ids():
                assert match[1] not in column, match[1]
                column[match[1]] = match[2].replace(' / ', '<br>')
    else:
        assert report['status'] == 'passed', name
        assert set(report['cells']) == {name}
        column = report['cells'][name]
    assert set(column) == manifest.ids(), (name, set(column) ^ manifest.ids())
    cells.update({(site, name): value for site, value in column.items()})
assert '32dba69d6 2026-10-09' in identity['compiler']
original = gate.DOC.read_text()
updated = gate.write_doc(original, cells)
gate.DOC.write_text(updated)
print('Replayed original write_doc over all seven complete hosted columns.')
print('Original measured identity:', json.dumps(identity, sort_keys=True))
print('These original-head measurements do not qualify changed remediation source.')
for name in gate.CONFIG_NAMES:
    print(name, 'entail.classify', cells['entail.classify', name])
