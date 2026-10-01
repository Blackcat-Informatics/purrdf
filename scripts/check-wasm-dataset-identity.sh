#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Build the unit-test wasm module (whose cfg(test) fixture is absent from releases),
# and exercise exact Dataset/exchange identity ABIs above 2^53 in Node and TypeScript.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$root"
command -v node >/dev/null
command -v wasm-bindgen >/dev/null
pin="$(sed -n 's/^wasm-bindgen = "=\([0-9][0-9.]*\)"$/\1/p' Cargo.toml)"
found="$(wasm-bindgen --version | sed -n 's/^wasm-bindgen \([0-9][0-9.]*\).*$/\1/p')"
[ "$found" = "$pin" ] || {
    echo "wasm-bindgen CLI version $found does not match pinned $pin" >&2
    exit 1
}
scratch="$(mktemp -d "$root/target/identity-transport.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT
python3 - "$scratch" <<'CAPTURE'
import hashlib
import json
import os
import runpy
import shutil
import subprocess
import sys
from pathlib import Path

scratch = Path(sys.argv[1]).resolve()
private_target = scratch / "compiler-target"
log = scratch / "compiler.jsonl"
# Cargo stores test executables in build-dir, unlike final cdylib outputs.
# Keep both owned artifact directories private. A host wrapper that overrides
# either setting is refused by the common containment proof below.
child_env = os.environ.copy()
child_env["CARGO_TARGET_DIR"] = str(private_target)
child_env["CARGO_BUILD_BUILD_DIR"] = str(private_target / "build")
with log.open("x") as output:
    result = subprocess.run([
        "cargo", "test", "--locked", "--target", "wasm32-unknown-unknown",
        "-p", "purrdf-wasm", "--lib", "--no-run", "--message-format=json",
        "--target-dir", str(private_target),
    ], stdout=output, env=child_env)
records = [json.loads(line) for line in log.read_text().splitlines()]
if result.returncode:
    for record in records:
        if record.get("reason") == "compiler-message":
            diagnostic = record.get("message", {})
            if diagnostic.get("level") == "error":
                print(diagnostic.get("rendered") or diagnostic.get("message"), file=sys.stderr)
    raise SystemExit(result.returncode)
units = [record for record in records if record.get("profile", {}).get("test")]
validate = runpy.run_path("scripts/build-private-wasm.py")["validate_capture"]
registered, fresh = validate(units, "purrdf_wasm", private_target)
shutil.copyfile(registered, scratch / "fixture.wasm")
receipt = {
    "schema": "purrdf-wasm-identity-abi-v1",
    "registered_output": str(registered),
    "private_target_directory": str(private_target),
    "private_build_directory": child_env["CARGO_BUILD_BUILD_DIR"],
    "cargo_fresh": fresh,
    "wasm_sha256": hashlib.sha256((scratch / "fixture.wasm").read_bytes()).hexdigest(),
    "rustc": subprocess.run(["rustc", "--version"], check=True, capture_output=True, text=True).stdout.strip(),
}
(scratch / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
CAPTURE
module="$scratch/fixture.wasm"
wasm-bindgen --target nodejs --out-name fixture --out-dir "$scratch" "$module"
cp crates/rdf-wasm/js/src/purrdf_jspi.mjs "$scratch/purrdf_jspi.mjs"
cp scripts/fixtures/wasm-identity-transport.ts "$scratch/identity.ts"
"$root/crates/rdf-wasm/js/node_modules/.bin/tsc" --ignoreConfig --strict --target ES2020 --lib ES2020,DOM,ESNext.Disposable --module commonjs --noEmit "$scratch/identity.ts"
node scripts/fixtures/wasm-dataset-identity.cjs "$scratch/fixture.js"

python3 - "$scratch" <<'RECEIPT'
import hashlib
import json
import sys
from pathlib import Path

scratch = Path(sys.argv[1])
receipt = json.loads((scratch / "receipt.json").read_text())
receipt["typescript_identity_contract"] = "passed"
receipt["node_actual_identity_abi"] = "passed"
receipt["bindgen_wasm_sha256"] = hashlib.sha256((scratch / "fixture_bg.wasm").read_bytes()).hexdigest()
receipt["bindgen_js_sha256"] = hashlib.sha256((scratch / "fixture.js").read_bytes()).hexdigest()
receipt["bindgen_types_sha256"] = hashlib.sha256((scratch / "fixture.d.ts").read_bytes()).hexdigest()
output = Path("target/identity-transport-receipts")
output.mkdir(exist_ok=True)
record = output / (receipt["wasm_sha256"] + ".json")
record.write_text(json.dumps(receipt, indent=2) + "\n")
print("Identity ABI receipt:", record)
RECEIPT
