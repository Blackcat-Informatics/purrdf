#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Generate ECMA-262 Unicode 17 executable ranges from the vendored UCD."""
from __future__ import annotations
import re
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2] / "iri/unicode/17.0.0"
ALIASES = Path(__file__).resolve().parents[1] / "src/ecma/property_tables.rs"


def records(file):
    for raw in (ROOT / file).read_text().splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        fields = [part.strip() for part in line.split(";")]
        span = fields[0].split("..")
        yield int(span[0], 16), int(span[-1], 16), fields[1:]


def merge(spans):
    result = []
    for lo, hi in sorted(spans):
        if result and lo <= result[-1][1] + 1:
            result[-1] = (result[-1][0], max(result[-1][1], hi))
        else:
            result.append((lo, hi))
    return result


def subtract(spans, cuts):
    result = []
    for lo, hi in spans:
        current = lo
        for left, right in cuts:
            if right < current:
                continue
            if left > hi:
                break
            if left > current:
                result.append((current, left - 1))
            current = max(current, right + 1)
            if current > hi:
                break
        if current <= hi:
            result.append((current, hi))
    return result


def complement(spans):
    return subtract([(0, 0x10FFFF)], merge(spans))


def aliases(section):
    text = ALIASES.read_text()
    start = text.index(f"pub(super) const {section}:")
    next_section = text.find("pub(super) const ", start + 1)
    body = text[start:next_section if next_section >= 0 else None]
    return {value for _, value in re.findall(r'\(\s*"([^"]+)",\s*"([^"]+)"\s*\)', body)}


def rust_hex(number):
    digits = f"{number:X}"
    groups = []
    while len(digits) > 4:
        groups.append(digits[-4:])
        digits = digits[:-4]
    groups.append(digits)
    return "0x" + "_".join(reversed(groups))


def unicode_categories():
    categories = defaultdict(list)
    first = None
    for line in (ROOT / "UnicodeData.txt").read_text().splitlines():
        fields = line.split(";")
        point = int(fields[0], 16)
        name, category = fields[1:3]
        if name.endswith(", First>"):
            first = (point, category)
        elif name.endswith(", Last>"):
            assert first and first[1] == category
            categories[category].append((first[0], point))
            first = None
        else:
            categories[category].append((point, point))
    assert first is None
    assigned = merge(span for spans in categories.values() for span in spans)
    categories["Cn"] = complement(assigned)
    return {name: merge(spans) for name, spans in categories.items()}


def main():
    category_ranges = unicode_categories()
    properties = {}
    for name in aliases("GENERAL_CATEGORY"):
        if name in category_ranges:
            properties[f"gc={name}"] = category_ranges[name]
        elif name == "LC":
            properties[f"gc={name}"] = merge(span for kind in ("Lu", "Ll", "Lt") for span in category_ranges[kind])
        else:
            properties[f"gc={name}"] = merge(span for kind, spans in category_ranges.items() if kind.startswith(name) for span in spans)
    scripts = defaultdict(list)
    for lo, hi, fields in records("Scripts.txt"):
        scripts[fields[0]].append((lo, hi))
    assigned_script = merge(span for spans in scripts.values() for span in spans)
    scripts["Unknown"] = complement(assigned_script)
    scripts = {name: merge(spans) for name, spans in scripts.items()}
    for name in aliases("SCRIPT"):
        properties[f"Script={name}"] = scripts.get(name, [])
    # Every listed extension overrides Script for its range. Unlisted points
    # retain Script, per the UCD @missing declaration.
    short_to_long = {}
    for raw in (ROOT / "PropertyValueAliases.txt").read_text().splitlines():
        fields = [part.strip() for part in raw.split("#", 1)[0].split(";")]
        if len(fields) >= 3 and fields[0] == "sc":
            short_to_long[fields[1]] = fields[2]
    extension_rows = list(records("ScriptExtensions.txt"))
    overrides = merge((lo, hi) for lo, hi, _ in extension_rows)
    for name in aliases("SCRIPT"):
        base = subtract(scripts.get(name, []), overrides)
        added = [(lo, hi) for lo, hi, fields in extension_rows if name in (short_to_long[alias] for alias in fields[0].split())]
        properties[f"Script_Extensions={name}"] = merge(base + added)
    binaries = defaultdict(list)
    for file in ["PropList.txt", "DerivedCoreProperties.txt", "DerivedNormalizationProps.txt", "emoji-data.txt"]:
        for lo, hi, fields in records(file):
            binaries[fields[0]].append((lo, hi))
    # Bidi_Mirrored is stored in UnicodeData's ninth field rather than a
    # stand-alone property file.
    for line in (ROOT / "UnicodeData.txt").read_text().splitlines():
        fields = line.split(";")
        if fields[9] == "Y":
            point = int(fields[0], 16)
            binaries["Bidi_Mirrored"].append((point, point))
    binaries["Any"] = [(0, 0x10FFFF)]
    binaries["ASCII"] = [(0, 0x7F)]
    binaries["Assigned"] = complement(category_ranges["Cn"])
    missing = aliases("BINARY") - binaries.keys()
    assert not missing, f"missing binary UCD properties: {sorted(missing)}"
    for name in aliases("BINARY"):
        properties[name] = merge(binaries[name])
    for special in ("ID_Start", "ID_Continue"):
        assert properties[special]
    print("// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>")
    print("// SPDX-FileCopyrightText: Unicode, Inc. <https://www.unicode.org>")
    print("// SPDX-License-Identifier: (MIT OR Apache-2.0 OR MulanPSL-2.0) AND Unicode-3.0")
    print("//! @generated by `python3 crates/jsonschema/examples/gen_ecma_unicode_ranges.py`.")
    print("//! Exact Unicode 17.0.0 executable property ranges for ECMA-262 /u.")
    print("pub(super) const RANGES: &[(&str, &[(u32, u32)])] = &[")
    for name, spans in sorted(properties.items()):
        print(f'    ("{name}", &[')
        for lo, hi in spans:
            print(f"        ({rust_hex(lo)}, {rust_hex(hi)}),")
        print("    ]),")
    print("];")
    folding = {}
    for raw in (ROOT / "CaseFolding.txt").read_text().splitlines():
        fields = [part.strip() for part in raw.split("#", 1)[0].split(";")]
        if len(fields) >= 3 and fields[1] in ("C", "S"):
            mapping = fields[2].split()
            if len(mapping) == 1:
                folding[int(fields[0], 16)] = int(mapping[0], 16)
    print("pub(super) const CASE_FOLD: &[(u32, u32)] = &[")
    for source, target in sorted(folding.items()):
        print(f"    ({rust_hex(source)}, {rust_hex(target)}),")
    print("];")
    print("pub(super) const CASE_FOLD_REVERSE: &[(u32, u32)] = &[")
    for target, source in sorted((target, source) for source, target in folding.items()):
        print(f"    ({rust_hex(target)}, {rust_hex(source)}),")
    print("];")


if __name__ == "__main__":
    main()
