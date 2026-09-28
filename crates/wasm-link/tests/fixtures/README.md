<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Frozen instruction-boundary oracle

`instruction-spans.tsv` contains 2,283 frozen vectors generated on 2026-09-28
using **wasmparser 0.259.0** as an independent binary grammar oracle. No oracle
source was copied. The crate's regression test reads only these frozen bytes;
wasmparser is not a workspace dependency.

The format is `hex<TAB>expected_span<TAB>function_index_or_dash`; `#` lines are
comments. The span and optional function index describe the first instruction.
Every input ends with `10 00 01 0b` (`call 0; nop; end`) after that instruction,
so the test also checks that an immediate did not absorb a following real call.
The oracle is the [WebAssembly binary instruction grammar](https://webassembly.github.io/spec/core/binary/instructions.html).

The initial grid covered every single-byte opcode and subopcodes 0–300 under
`fc`, `fd`, `fe`, and `fb`, with immediate bytes filled by `00`, `01`, `10`, or
`7f`. It retained cases decoded by both readers and froze **wasmparser's** span
and reference result. Cases whose irrelevant trailing fill differs can become
identical vectors after truncation; row count is not a claim of distinct
instructions. Unsupported experimental instructions were refused by the native
reader. This corpus checks accepted instruction boundaries; it does not claim
complete WebAssembly proposal coverage or replace semantic validation.

## Audit identities

SHA-256:

| Input | Digest |
|---|---|
| Frozen TSV | `80adf1dff70f48b10a3cd39051db260fc6325d83a6aa45130b53abb4a704e528` |
| Audited native `src/binary.rs` | `980d81bc465b67d3b152e27a46a20347e9904f78c27f1eeb56cff6752f7b62fa` |
| Existing local package artifact A | `c9e7c59205da0510509f44e40d04ba02fcc11b74e3e209c100a3400357eb438e` |
| Existing isolated package artifact B | `c0957344d26c7cd111455bfa12adf28bedb7878efabd03e6e32d89a6ab256481` |

Separately, every instruction start/end and function reference in artifact A
matched the oracle: 10,573 bodies, 6,059,115 instructions, 161,614 references.
Artifact B matched over 12,638 bodies, 7,342,852 instructions, 191,233 references.
These are existing artifact identities used for the reader audit, not a claim
that either artifact is the final integrated build. The artifacts are not vendored.

## Reproduce the frozen expected values

From the repository root, the following creates an isolated temporary oracle
project. Its dependency never enters the workspace. It independently decodes
every frozen input and compares the expected columns byte for byte. Use
`--offline` when wasmparser 0.259.0 and its dependencies are already cached.

```bash
oracle_dir=$(mktemp -d)
mkdir -p "$oracle_dir/src"
cat > "$oracle_dir/Cargo.toml" <<'TOML'
[package]
name = "instruction-span-oracle"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
wasmparser = "=0.259.0"
TOML
cat > "$oracle_dir/src/main.rs" <<'RS'
use wasmparser::{BinaryReader, Operator, OperatorsReader};
fn main() {
    let input = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    for row in input.lines().filter(|line| !line.starts_with('#')) {
        let hex = row.split('\t').next().unwrap();
        let bytes: Vec<u8> = hex.as_bytes().chunks_exact(2).map(|pair| {
            u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap()
        }).collect();
        let mut reader = OperatorsReader::new(BinaryReader::new(&bytes, 0));
        let index = match reader.read().unwrap() {
            Operator::Call { function_index }
            | Operator::ReturnCall { function_index }
            | Operator::RefFunc { function_index } => function_index.to_string(),
            _ => "-".into(),
        };
        println!("{hex}\t{}\t{index}", reader.original_position());
    }
}
RS
cargo run --quiet --manifest-path "$oracle_dir/Cargo.toml" -- \
  crates/wasm-link/tests/fixtures/instruction-spans.tsv > "$oracle_dir/actual.tsv"
rg -v '^#' crates/wasm-link/tests/fixtures/instruction-spans.tsv > "$oracle_dir/expected.tsv"
diff -u "$oracle_dir/expected.tsv" "$oracle_dir/actual.tsv"
```

The normal native regression is `cargo test -p wasm-link --lib --locked` and
requires the pinned Binaryen installation used by the crate's other tests.
