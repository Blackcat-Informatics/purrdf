"""Replay the canonical document writer over the authenticated CI measurement.

This does not build, probe the host, alter reports, or declare failed CI passed.
Current-source CI still owns final seven-configuration qualification.
"""

import hashlib
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys
import tomllib

root = Path(__file__).resolve().parents[2]
stage = Path(__file__).resolve().parent
sys.path.insert(0, str(root / "scripts"))
spec = importlib.util.spec_from_file_location("ci_asm_projection_owner", root / "scripts/check-simd-asm.py")
gate = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = gate
spec.loader.exec_module(gate)

manifest = gate.load_manifest(tomllib.loads(gate.MANIFEST.read_text()))
text = subprocess.check_output(["git", "show", "HEAD:docs/design/purrdf-simd.md"], cwd=root, text=True)
header, rows, _ = gate.parse_table(text, gate.SITES_BEGIN, gate.SITES_END)
cells = {}
changed = []
artifacts = []
for config, log_name in [("x86_64-v4", "current-ci-x86-v4.log"), ("aarch64", "current-ci-aarch64.log")]:
    artifact = stage / ("asm-diagnostics-" + config)
    report = json.loads((artifact / "asm-reports" / (config + ".json")).read_text())
    identity = report["identity"]
    assert report["status"] == "incomplete", "retain the original failed report"
    assert identity["manifest"] == hashlib.sha256(gate.MANIFEST.read_bytes()).hexdigest()
    assert identity["compiler"] == (artifact / "asm-diagnostics/compiler.txt").read_text()
    assert (artifact / "asm-diagnostics/source-status.txt").read_text() == ""
    checkout = (artifact / "asm-diagnostics/checkout.txt").read_text().splitlines()
    assert checkout[0] == subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    for line in (artifact / "asm-diagnostics/inputs.sha256").read_text().splitlines():
        expected, name = line.split(maxsplit=1)
        data = text.encode() if name == "docs/design/purrdf-simd.md" else (root / name).read_bytes()
        assert hashlib.sha256(data).hexdigest() == expected, name
    artifacts.append(identity)
    log = re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", (stage / "raw" / log_name).read_text())
    measured = {}
    for line in log.splitlines():
        line = re.sub(r"^\d{4}-\d\d-\d\dT\S+Z\s?", "", line)
        match = re.fullmatch(r"([a-z0-9_.-]+)\s{2,}(.+)", line)
        if match and match[1] in manifest.ids():
            assert match[1] not in measured, "ambiguous repeated measurement row"
            measured[match[1]] = match[2].strip()
    assert set(measured) == manifest.ids(), "CI did not print the complete measured shard"
    mismatch = []
    for row in rows:
        site = gate._plain(row[header.index("id")])
        if site not in manifest.ids():
            continue
        recorded = row[header.index(config)]
        actual = measured[site]
        if recorded.replace("<br>", " / ") == actual:
            cells[(site, config)] = recorded
        else:
            assert site == "eval.bgp-join-probe", "unexpected additional projection drift"
            assert re.fullmatch(r"v\d+·f\d+·r\d+", actual), "not a scalar measured cell"
            cells[(site, config)] = actual
            mismatch.append(site)
    assert mismatch == ["eval.bgp-join-probe"]
    assert "FAIL: 1 problem(s):" in log
    assert "measured '" + measured[mismatch[0]] + "'" in log
    changed.append((mismatch[0], config))
assert artifacts[0] == artifacts[1], "configuration measurements have different identities"
updated = gate.write_doc(text, cells)
problems = gate.doc_checks(updated, manifest, cells, gate.workspace_world(), configs=("x86_64-v4", "aarch64"))
assert not problems, problems

# Recreate the immutable measured input only after proving any existing edit is
# exactly this task's earlier canonical projection; never discard another edit.
previous = gate.DOC.read_text()
earlier = gate.write_doc(text, {key: value for key, value in cells.items() if key[1] == "x86_64-v4"})
assert previous in (text, earlier), "unexpected document edit; preserve it"
assert subprocess.check_output(["git", "diff", "--name-only"], cwd=root, text=True).splitlines() in ([], ["docs/design/purrdf-simd.md"])
try:
    gate.DOC.write_text(text)
    assert artifacts[0]["source"] == gate.runtime.source_identity(root), "CI and original source differ"
except BaseException:
    gate.DOC.write_text(previous)
    raise
gate.DOC.write_text(updated)
print(f"Verified matching CI source, compiler and manifest; regenerated {len(changed)} cells from two complete {len(measured)}-site shards using the canonical owner.")
