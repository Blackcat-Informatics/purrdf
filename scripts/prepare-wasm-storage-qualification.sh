#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Prepare an unpublished browser workload with an enforced linear-memory cap.
# The receipt pin must come independently from successful full certification.
set -euo pipefail
if [ "$#" -lt 4 ] || [ "$#" -gt 5 ]; then
    echo "usage: $0 SOURCE RECEIPT TRUSTED_RECEIPT_PIN NEW_OUTPUT_DIRECTORY [LINEAR_MEMORY_MAX_BYTES]" >&2
    exit 2
fi
source_file="$(realpath "$1")"
receipt_file="$(realpath "$2")"
trusted_pin="$3"
output_dir="$4"
linear_max="${5:-268435456}"
python3 - "$linear_max" <<'PY'
import sys
try:
    limit=int(sys.argv[1])
except ValueError:
    raise SystemExit("linear memory cap must be an integer byte count")
if limit < 1<<20 or limit > 256<<20 or limit & (limit-1):
    raise SystemExit("linear memory cap must be a power of two from 1 MiB through 256 MiB")
PY
[[ "$trusted_pin" =~ ^[0-9a-f]{64}$ ]] || { echo "invalid canonical receipt pin" >&2; exit 2; }
[ "$(wc -c < "$receipt_file")" -eq 104 ] || { echo "receipt must be exactly 104 bytes" >&2; exit 2; }
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$root"
command -v wasm-bindgen >/dev/null
command -v wasm-dis >/dev/null
mkdir "$output_dir"
output_dir="$(realpath "$output_dir")"
# Cargo's published target path may be shared across concurrent flag variants.
# The shared capture home requests a private Cargo target and validates the
# selected JSON artifact inside it before copying the bytes to this directory.
module="$output_dir/compiler-module.wasm"
qualification_rustflags="${RUSTFLAGS:-} -D warnings -C link-arg=--max-memory=$linear_max"
RUSTFLAGS="$qualification_rustflags" \
    python3 scripts/build-private-wasm.py "$module" wasm_storage_qualification \
    --locked --offline --release --target wasm32-unknown-unknown \
    -p purrdf-envelope-probe --example wasm_storage_qualification
wasm-bindgen --target web --out-name qualification --out-dir "$output_dir" "$module"
cp "$source_file" "$output_dir/source.bin"
cp "$receipt_file" "$output_dir/source.receipt"
cp scripts/fixtures/wasm-storage-qualification.html "$output_dir/index.html"
cp scripts/fixtures/wasm-storage-qualification.mjs "$output_dir/run.mjs"
python3 - "$output_dir" "$trusted_pin" "$linear_max" "$qualification_rustflags" <<'PY'
import hashlib, json, pathlib, re, subprocess, sys
output=pathlib.Path(sys.argv[1])
module=output / "qualification_bg.wasm"
maximum=int(sys.argv[3])
signature=(output/"qualification.d.ts").read_text()
if not re.search(r"qualify_resident_envelope\(actual_hundred_thousand: boolean\)", signature):
    raise SystemExit("qualification module lacks the current supplementary resident ABI")
text=subprocess.run(["wasm-dis", str(module)], check=True, capture_output=True, text=True).stdout
memories=re.findall(r"\(memory(?: \$[^\s()]+)? (\d+) (\d+)\)", text)
if len(memories)!=1 or int(memories[0][1])*65536!=maximum:
    raise SystemExit("qualification module must declare exactly the requested memory maximum")
manifest={
    "schema":"purrdf-wasm-storage-preparation-v1",
    "trusted_receipt_pin":sys.argv[2],
    "linear_memory_maximum_bytes":maximum,
    "initial_linear_memory_bytes":int(memories[0][0])*65536,
    "wasm_sha256":hashlib.sha256(module.read_bytes()).hexdigest(),
    "compiler_output_sha256":hashlib.sha256((output/"compiler-module.wasm").read_bytes()).hexdigest(),
    "source_sha256":hashlib.sha256((output/"source.bin").read_bytes()).hexdigest(),
    "source_bytes":(output/"source.bin").stat().st_size,
    "rustflags":sys.argv[4],
    "rustc":subprocess.run(["rustc","--version"],check=True,capture_output=True,text=True).stdout.strip(),
}
(output/"prepared.json").write_text(json.dumps(manifest,indent=2)+"\n")
print("Prepared browser workload:",output)
print("Serve that directory over HTTP, open index.html in the target browser, and retain its complete result record.")
PY
