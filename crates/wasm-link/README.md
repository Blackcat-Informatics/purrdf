<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# wasm-link

The host build tool that establishes PurRDF's asynchronous WebAssembly stack and
poison guarantees after `wasm-opt`. Its library documentation specifies the
wrappers, counters, refusal variants and verification rules.

## Dependency and validation boundary

The first-party binary reader follows the [WebAssembly binary grammar](https://webassembly.github.io/spec/core/binary/index.html).
It decodes instruction boundaries and function-index immediates, preserves the
remaining original bytes, and appends the new functions, globals and types.
Constants, SIMD lane payloads, LEB integers and memory arguments are consumed as
whole immediates; their bytes cannot be confused with calls. Unknown constructs
fail closed. The artifact uses plain function types; GC object type declarations
are refused. `wasm-encoder`, with default features disabled, emits only the new
sections and templates; its only dependency is `leb128fmt`.

**Binaryen 130's `wasm-opt` must be on PATH**, including for native unit tests.
This is the existing pinned wasm package optimizer. The tool validates input and
output against the package baseline plus recognized LLVM `target_features`
declarations. It receives bytes through stdin and sends its output to the null
sink; it neither writes a temporary module nor replaces the emitted artifact.
Input is written concurrently while diagnostics are drained, avoiding pipe
backpressure deadlocks. Missing tools and all validation failures are errors.

The checker regenerates every injected body from its signature and indices,
compares the decoded instructions, and rejects raw imported function references
from bodies, table elements, global/table initializers and the start section.
The latter references cannot bypass the import trampolines through an exported
function-reference global.

## Tests

`cargo test -p wasm-link` retains the wrapper, export, poison-variant and feature
refusal cases, and adds a valid-module attack exposing a raw import through a
global initializer, hostile opcode bytes inside floating-point/SIMD immediates,
and truncated/overflowing LEB integers. `cargo clippy -p wasm-link --all-targets`
checks the native host tool. The wasm package lane checks the actual optimized
artifact and exercises its JavaScript runtime behavior.
