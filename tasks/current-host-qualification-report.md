# Current402 host qualification — settled boundary PASS

On2026-10-08 the admitted current-source host batch settled successfully.
The initial controller57794 exited1 after successful wheel build/install because
the Stage identity probe used `import purrdf.purrdf_native as n`; the existing
shim swaps the package object for its native RDF submodule, which does not carry
that attribute. The corrected Stage probe uses `importlib.import_module`.
Original traceback, command and terminal remain retained; production source was
not changed. Resume40394 exited0, without repeating wheel build/install.

Owned immutable receipts/artifacts are under
`/opt/purrdf-shacl-402-host-20261008.NWG9V4QQ`. Exact argv, complete output and
exit are each command's `.command`, `.log`, `.exit`; aggregate is
`host-settled-summary.json`. `current-host-qualification.sh` and
`current-host-resume.sh` are the attributable Stage controllers.

|Actual current-artifact boundary|Terminal|Executed result|
|---|---|---|
|Production PEP517 wheel, locked/jobs8; exact-wheel worktree-venv install|0/0|One fresh cp313 abi3 wheel; no dependencies/global installation|
|Corrected isolated module identity|0|Installed native bytes equal the wheel's exact native entry|
|Existing selected Python report/product/annotation/digest-refusal callers|0|13 passed;0 failed/skipped|
|Production private-output WASM package route|0|Release compile, Binaryen130 Oz, JSPI postlink and146295 SIMD opcode proof|
|Actual npm pack/extract|0/0|Current optimized module and shipped index/glue match unpacked runtime byte for byte|
|JSPI runtime admission|0|Nodev26.11.1 exposes WebAssembly.Suspending/promising|
|Selected public report/marshalling cases|0|5 passed;0 failed/skipped/cancelled/todo|
|Existing product byte-buffer/typed-refusal/export cases|0|7 passed;0 failed/skipped/cancelled/todo|
|Existing async SHACL cases|0|All8 executed/passed;0 failed/skipped/cancelled/todo|

The eight async cases exercise actual promises, yields, signals/timeouts/already
aborted calls, refusal attributes, suspended concurrent ambient state and sync
interleaving. They are host-boundary evidence; already qualified Rust semantics
and corpus counts are not requalified by host agreement. No broad host/vendor
semantic matrix or new host semantic mirrors were introduced. package.json has
no prepack/prepare lifecycle scripts, so `npm pack --ignore-scripts` skips no
required prepack gate.

Compiler is rustc1.100.0-nightly4b6d04e706108ccfeafe2547fbe857dfe8972bad,
LLVM23.1.1. Both controllers used eight Cargo/build/test jobs and owned disk
TMPDIR. Outer scopes enforce64GiB/Swap0; Python's normal Stage Cargo children
inherit the separately captured shared48GiB/8GiB swap slice. The scoped same-SDK
private WASM route honors the production helper's private target and stays in
the resumed outer scope: observed live1.776GB/current2.120GBpeak/zero swap at
that snapshot. Slice-wide historical peaks are not this batch's peaks.
`resume-scope-terminal.txt` shows the resumed scope inactive. No compiler
zero-swap claim is made for the normal Stage Python build.

Source remains HEAD4faeb36ce487eacbe48db2fc3e8bbcbd0c5a1df6 and resolved staged
MERGE_HEAD5384882d65750ee22bddfeeac473095ab9c3db03. No unmerged index entries or
shipping source/index changes were made. Final source/index fingerprints and
artifact SHA256/size readbacks are in the aggregate. Installed native SHA256
is16e324c5643897073594118d47fc8fa1084a54d8e7203140521ca91eba7395ce;
optimized packed WASM isfca1d0101936ef7c21e9f419c78f20e11c689e380789cd1c43f4a3ec1c9d7376.
Compiler registered-output JSON and capture receipt remain in
`tmp/tmp.l9Mezdl6A3/`; wheel, npm tarball and extracted tested runtime are retained.

Sole local build lane is FREE after actual40394 terminal0. Native qualification
remains separately PASS. Owning PO/native glossary/render/doc-count work,
matched report timing (held while308 profiling runs), current-source hosted
gates, main475 integration assessment and independent completion remain required.
This host batch is not whole402 completion or publication acceptance.
