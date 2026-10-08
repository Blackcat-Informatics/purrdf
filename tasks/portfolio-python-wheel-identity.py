# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: This boundary probe validates Python isolated imports and the interpreter-selected native module against the actual installed wheel.
"""Stage-only isolated interpreter/wheel identity probe; not a shipping gate."""

import hashlib
import importlib
import json
from pathlib import Path
import sys
import zipfile

wheel = Path(sys.argv[1]).resolve(strict=True)
venv = Path(sys.argv[2]).resolve(strict=True)
native = importlib.import_module("purrdf.purrdf_native")
native_path = Path(native.__file__).resolve(strict=True)
assert Path(sys.prefix).resolve() == venv, (sys.prefix, venv)
assert native_path.is_relative_to(venv), native_path
assert sys.flags.isolated == 1
installed_root = native_path.parent.parent
receipts = []
with zipfile.ZipFile(wheel) as archive:
    names = [name for name in archive.namelist() if name.startswith("purrdf/") and not name.endswith("/")]
    native_names = [name for name in names if "purrdf_native" in name and name.endswith(".so")]
    assert len(native_names) == 1, native_names
    assert (installed_root / native_names[0]).resolve() == native_path
    for name in names:
        packaged = archive.read(name)
        installed = (installed_root / name).read_bytes()
        assert packaged == installed, name
        receipts.append({"path": name, "sha256": hashlib.sha256(installed).hexdigest(), "bytes": len(installed)})
assert receipts
print(json.dumps({"interpreter": sys.executable, "venv": str(venv), "isolated": sys.flags.isolated, "native": str(native_path), "wheel": str(wheel), "wheel_sha256": hashlib.sha256(wheel.read_bytes()).hexdigest(), "package_files": receipts}, sort_keys=True, indent=2))
