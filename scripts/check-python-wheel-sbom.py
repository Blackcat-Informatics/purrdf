# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: Audits Python wheel metadata through the standard-library ZIP reader.
"""Extract and validate the CycloneDX SBOM carried by a PurRDF wheel."""

import argparse
import json
from pathlib import Path
from zipfile import ZipFile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("wheel", type=Path)
    parser.add_argument("version")
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    member = f"purrdf-{args.version}.dist-info/sboms/purrdf-python.cyclonedx.json"
    with ZipFile(args.wheel) as wheel:
        if wheel.namelist().count(member) != 1:
            parser.error(f"{args.wheel.name} must contain exactly one {member}")
        raw = wheel.read(member)
    document = json.loads(raw)
    if document.get("bomFormat") != "CycloneDX":
        parser.error("the native wheel SBOM must be CycloneDX")
    root = document["metadata"]["component"]
    if root["name"] != "purrdf-python" or root["version"] != args.version:
        parser.error("the SBOM describes another distribution or version")
    if not any(
        target["name"] == "purrdf_native" and target["version"] == args.version
        for target in root["components"]
    ):
        parser.error("the SBOM does not describe the native Python extension")
    components = {component["name"] for component in document["components"]}
    if not {"purrdf", "pyo3"} <= components:
        parser.error("the SBOM is missing required native dependencies")
    args.output.write_bytes(raw)


if __name__ == "__main__":
    main()
