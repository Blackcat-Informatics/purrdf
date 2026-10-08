#!/usr/bin/env python3
# Stage evidence adapter only; not shipping tooling or a new catalogue parser.
# Why not Rust: reuse the existing render gate's exact Python catalogue reader for this retained evidence comparison.
import json
from pathlib import Path
import sys

root, logs = map(Path, sys.argv[1:])
sys.path.insert(0, str(root / "scripts"))
import po_catalog

def entries(path):
    return [entry for entry in po_catalog.parse_po(path.read_text(encoding="utf-8")) if entry.msgid]

generated = entries(logs / "generated.po")
retained = {(entry.msgctxt, entry.msgid, entry.msgstr) for entry in generated}
report = {}
for name in ("main", "contextual", "merged"):
    original = entries(logs / f"{name}-preimage.po")
    lost = [(entry.msgctxt, entry.msgid, entry.msgstr) for entry in original
            if entry.msgstr and (entry.msgctxt, entry.msgid, entry.msgstr) not in retained]
    report[name] = {"entries": len(original), "lost_exact_translation_triples": lost}
active = [entry for entry in generated if not entry.obsolete]
report["generated"] = {"entries": len(generated), "active": len(active),
                       "obsolete": len(generated) - len(active),
                       "fuzzy_active": sum(entry.fuzzy for entry in active),
                       "empty_active": sum(not entry.msgstr for entry in active)}
(logs / "translation-preservation-audit.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
if any(report[name]["lost_exact_translation_triples"] for name in ("main", "contextual", "merged")):
    raise SystemExit("catalogue regeneration lost retained translation triples; inspect audit before staging")
