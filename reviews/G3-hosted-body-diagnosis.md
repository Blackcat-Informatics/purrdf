# G3 actual hosted body and provenance diagnosis

Read-only specialist result, 2026-10-06: the existing v3/v4 selector names the actual production eager seal and includes its scalar page-to-global gathers. No selector defect was found in these two hosted configurations. Both selected parent bodies legitimately count `v0·f0·r0` under the unchanged reader law. This resolves the earlier exact-body uncertainty; it is not a full G3, matrix, CI, issue-completion or merge PASS.

No source/index, Cargo, compiler/toolchain, Stage configuration, sibling worktree, driver measurement/probe, or forge mutation occurred. Existing reader homes analyzed the downloaded public `.s` bytes directly. Only the assigned Stage diagnosis and supporting analysis/body receipts were written.

## Bound hosted inputs

Normal PR CI run: `37532186299`. v3 job `112504322483`; v4 job `112504323089`.

- Signed/pushed branch head: `8364b29c9fb70195f123e2bc084160d83f78850a`.
- Hosted checkout: synthetic merge `e32f3cfd0681ab438f58d924b5d426c836c724c0`.
- Both checkout and branch source tree: `cb77b4d6beb3a5b7f17e3979570e651dbebe0564`.
- Actual compiler: `rustc 1.101.0-nightly (ea137335b 2026-10-05)`, full commit `ea137335b78829b4514bf1b4c16302f74fab8581`, LLVM `23.1.3`, host `x86_64-unknown-linux-gnu`.
- Manifest SHA-256: `c96ab1574d88cae967d67bcb642a7a4b7716555aaf9ca66e1b2e87fac36f760a`.
- Source content identity: `5cdc94d05519536689165b4fad734f27a4a23825a69df2d6071072c823aa1fec`.
- Reader SHA-256: `3295116bcdc67581a301f53a19b3fadabb63f1ee11a83dcd0caa00f0b24596c5`.
- Runtime SHA-256: `8e8f406574e9c7e31a73b8d2b266546cf702707b9c9208e0b6d899f6204cbd05`.

I independently recomputed both retained ZIP hashes, matching `raw/G3a-normal-run-artifacts.json`:

| Configuration | Artifact ID | ZIP SHA-256 |
|---|---|---|
| v3 | 11445197096 | `9ec0031de20cc98f074c19dc851b56cb92eeb5f7ae5c0e99d5a4ef45c7659db2` |
| v4 | 11444917866 | `049ea49dbaf47248007f41059d767dc02a5a960d90dc25f0a08e3ab6b2edd48c` |

The artifact metadata binds each diagnostic to this exact normal CI run and branch head. Root previously verified safe extraction. Each extracted `asm-diagnostics/compiler.txt` equals its incomplete report's compiler identity byte-for-byte; checkout receipts name the source above. Manifest bytes hash to the report's manifest. I independently called the existing `runtime.source_identity` in root's clean replay `/home/paudley/Active/purrdf/.worktrees/qual-asm-37532305791`; it returned exactly the hosted source identity above. This was byte-hash reconstruction only, not local compiler or report qualification. The active issue worktree's untracked/unignored `.stage/` was neither moved nor used to substitute a local source identity.

Both actual job logs show exactly one document-parity refusal: v3's old cell `v14·f0·r0` and v4's old cell `v10·f0·r0` disagree with `v0·f0·r0`. Their new failure diagnostic uploads succeeded, establishing real hosted behavior of that lasting mechanism. Their reports remain `status: incomplete`, `cells: {}`; they do not qualify any successful shard or full matrix.

## Assembly and compiler-command provenance

| Configuration | Context key | Core `.s` SHA-256 |
|---|---|---|
| v3 | `d75186906acf6aa57f7af1854ce87d03683eddd78ee35759a18ac497d3856047` | `29d938338b5bc639ce1b09c47b4e405cc283ccc836ba707bf1c8c04e32205754` |
| v4 | `5651b23f3fee308586b8d2c433cc1a00a2510901d807755d363291dd544aff6b` | `6e3ab72d4572540b4133ae5e4ef3157a18e7830550de559327101644ffa9eccf` |

Each diagnostic contains exactly one actual core assembly file. Each recomputed assembly hash identifies exactly one matching `evidence/*.json` receipt carrying that configuration's context key and actual `--crate-name purrdf_core` command. Existing `verify_command_lines` returns no problem for either receipt.

The actual core rustc commands specify `--crate-type rlib`, target `x86_64-unknown-linux-gnu`, `-C opt-level=3`, `-C codegen-units=1`, `-C embed-bitcode=no`, `--emit=asm`, `-D warnings`, and exactly `-C target-cpu=x86-64-v3` or `x86-64-v4`. They do not name a replacement CPU/feature. The existing `build_commands` home constructs the release/locked/lib Cargo commands with `profile.release.lto=false` and `profile.release.codegen-units=1`, followed by the wasm rlib graph. Its output is preserved in the detail receipt; constructing these argv lists executed no Cargo.

The actual `cargo.graph-0.stdout.jsonl` artifact message binds the core unit to graph 0, `fresh: false`. Graph 1 reports the same unit/path with `fresh: true`. Both have the same production `purrdf-core@3.0.1` package identity and `features: []`. Existing `asm_paths` derives the same specific uploaded assembly path from each graph's actual artifact message. Thus the reader sees the same core bytes in both build graphs, not a second differently numbered generic kernel or an unrelated artifact. The actual recorded commands, matching assembly receipts and graph membership are all retained in `raw/G3-hosted-body-analysis.json`.

## What actually matched and counted

The manifest selector remains `<purrdf_core::ir::paged::PagedDataset>::from_provider`. Existing `collect_functions`, anchored `symbol_names`, `measured`, `count`, and `evaluate` applied to the hosted bytes yield:

| Configuration | Actual matched parent copies | Instructions per copy | Vector work | FMA | Relaxed | Measure problems |
|---|---|---|---|---|---|---|
| v3 | one per graph 0 and 1, same assembly | 944 | 0 | 0 | 0 | none |
| v4 | one per graph 0 and 1, same assembly | 944 | 0 | 0 | 0 | none |

The selected raw symbol is `_RNvMs3_NtNtCskgWIz7JHH9v_11purrdf_core2ir5pagedNtB5_12PagedDataset13from_provider` in both configurations. The reader's minimum-over-matched-copies therefore is not concealing a low-count unrelated copy: each selected graph membership points to the same full production body and has the same zero counts. A nested `from_provider::{closure#2}` exists with 116 instructions and zero counts; the normal parent-preference rule correctly excludes that closure from the measure. There are no differently instantiated `map_ids`/`to_global` bodies hidden among these selected matches.

The 944-instruction body contains 43 `vmovups`, 10 `vmovaps`, and one same-register `vxorps`. Those are moves and the zeroing idiom, excluded by the existing vector-work law. They are not evidence of packed gather arithmetic. Scalar bounds checks, scalar id translation loads, ordinary control flow, allocation/result copies and direct/indirect calls compose the body; no packed work, fused arithmetic or relaxed instruction was counted. “Zero vector work” does not mean “no vector-register instruction.”

## Actual page mapping and delegation

The raw parent extracts preserve labels and source instruction ordering. In each body, the primary mapping loop at `.LBB1634_18` reads the local subject/predicate/object and graph columns, adjusts the nonzero local ids, performs bounds checks, and issues scalar table loads such as:

```asm
movq (%r15,%rdi,8), %r13
movq (%r15,%rax,8), %rbp
movq (%r15,%rdx,8), %r12
testl %ecx, %ecx
je .LBB1634_24
...
movq (%r15,%rcx,8), %r14
```

The optional graph branch supplies zero when absent; the three mandatory values and optional graph are written into the overlap key and passed to the ordinary fixed-state `hashbrown::map::HashMap::insert` call. Analogous scalar loops for the reifier and annotation tables are present at `.LBB1634_41` and `.LBB1634_52`. These match the actual source's three `q.map_ids(|id| translation.to_global(id))` loops. The source mapping has not disappeared behind an unrelated method matched by a broad substring: this actual parent performs the expected three mandatory scalar gathers and optional fourth.

No direct out-of-line `QuadIds::map_ids` or `PageTranslation::to_global` call/body was emitted in the inspected core unit. The scalar mapping instructions are in the parent itself. This is an observation about these exact v3/v4 artifacts; it does not add a future inlining requirement or claim that all target compilers must do the same.

The parent has 69 call sites, including actual `PageTranslation::try_build`, fixed-state overlap-map insertion, reifier term lookup, `GraphPageIndex::derive`, overlap-error closure, allocation/copy/drop helpers, and indirect provider calls. Their transitive instruction work is **not** counted by this selected parent site. The adjacent translation `build`/`try_build`/clone bodies were parsed as supporting context (37/385/91 instructions, all zero counted work) but are not selected as this site's evidence. Direct call details and raw operands are preserved in `raw/G3-hosted-body-details.json`. This is exactly the corrected descriptive scope: enclosing emitted parent counts, not an isolated gather benchmark, whole-call-graph count or final application throughput claim.

## Method and attributable analysis receipts

Two ad-hoc `python3 -c` analyses loaded the existing reader via `importlib` under a non-main module name. The existing parser/count/selection/profile argv homes were called directly; the driver `run`/`main`, `Runner`, `measure_all`, `report_identity`, rustc and Cargo were never invoked. No new Python tooling file or duplicate gate algorithm was introduced.

The first analysis called `load_manifest`, `asm_paths` on the actual graph messages, `verify_command_lines` on the matching compiler-command receipt, `collect_functions` on each actual hosted `.s` with its graph membership, `symbol_names`/`measured` for the actual manifest selector, and `count`/`evaluate`. It asserted exact manifest/compiler/assembly/context bindings and explicitly incomplete status. Exit 0 is recorded in the tool receipt; `raw/G3-hosted-body-analysis.log` records both 944-instruction zero-count parents and no measure problems.

The second analysis used existing `collect_functions`, `demangle`, `count` and non-executing `build_commands` to preserve direct calls, excluded vector-register instruction categories, and complete raw parent assembly extracts. It returned exit 0; `raw/G3-hosted-body-details.log` records the observations. Non-Rust unwind operands remain raw calls rather than misidentified Rust symbols. A separate existing `runtime.source_identity(clean_replay)` call returned exit 0 and the exact expected source digest in `raw/G3-hosted-clean-source-replay.log`.

Supporting outputs:

- `raw/G3-hosted-body-analysis.json` and `.log`: input hashes, matching receipt/commands, actual graph artifacts, exact selected and excluded functions, counts and measure result.
- `raw/G3-hosted-body-details.json` and `.log`: actual call sites, profile argv construction, excluded move/zeroing categories, raw extract hashes and instruction digests.
- `raw/G3-hosted-v3-bodies.txt`, `raw/G3-hosted-v4-bodies.txt`: existing parser's supporting bodies and instruction sequence, with graph attribution.
- `raw/G3-hosted-v3-parent.s`, `raw/G3-hosted-v4-parent.s`: verbatim actual parent excerpts with original basic-block labels.
- `raw/G3-hosted-clean-source-replay.log`: independent exact source-hash reconstruction using the existing runtime home.

## Observations, inference limits and remaining qualification

The two actual selected production parents and their scalar gathers are proved. The unchanged law's zero count and corrected parent-scope description are supported. No selector or production vectorization defect is established here, and no floor weakening or production mapping rewrite is warranted by these artifacts.

The reason historical document counts were 14/10 is **not** proved: this diagnosis has not obtained the old positive-count assembly at the same source/compiler/profiles. Do not attribute the historical change to checked interning, generic outlining, compiler upgrades or a performance regression without a controlled comparison. Current scalar parent bodies and delegated call sites alone cannot identify that historical cause.

Full projection dispatch `37532305791` is still separately required to finish and publish its successful whole-matrix artifact. Its exact generated document, complete checked report/aggregate, compiler/source stability and clean replay must be reviewed before G3b applies the projection. Final normal seven independent shards and aggregate must then pass on the final source/candidate/compiler, followed by the remaining Stage completion/review-debt/integration gates. These failed normal jobs' successful diagnostic uploads and two valid body diagnoses do not satisfy those outstanding acceptance criteria.
