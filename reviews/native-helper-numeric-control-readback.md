# Post-helper numeric-control artifact readback

Read-only inspection of source, ELF symbols, disassembly and relocations; no
benchmark execution, builds, tests, timing or process changes. The five normalized
current disassemblies are retained as `native-helper-*-current.asm` in this Stage.

The actual candidate named by `native-helper-bench-identities.sha256` and
`recovery-row-append-measure.sh` still hashes to
`820f626632a242651636a7b557099bec610058aaecd46310ac144df2f08aa33a`.
The original baseline still hashes to
`7bb8ab6e42fbe8cf1a842d47bae2f1f34877921eeebd7c87c52d1758f6089d72`.
Both are the named `query_eval-8e4ffe4ed9e13b7f` binaries in their respective
`purrdf-evidence-default-{current,baseline}/build/release/build/purrdf-sparql-eval/8e4ffe4ed9e13b7f/out/`
directories.

| Inspected body | Fresh start | Bytes / instructions | Fresh / baseline start modulo 64 |
| --- | --- | --- | --- |
| `value_total_cmp` | `0x867b90` | 30 / 10 | 16 / 16 |
| `value_cmp` | `0x868470` | 1359 / 349 | 48 / 48 |
| `numeric_total_cmp` | `0x86cb70` | 1240 / 317 | 48 / 48 |
| Actual total timed closure #0 | `0x8fc450` | 346 / 87 | 16 / 48 |
| Actual promoted timed closure #1 | `0x8fc5b0` | 321 / 77 | 48 / 16 |

All five instruction bodies match both the retained `native-layout-*-baseline.asm`
and `native-append-*-current.asm` exactly under their existing normalization:
instruction addresses and RIP-relative displacements/comments removed, symbolic
branch targets retained. This is normalized instruction identity, not whole-binary
or arbitrary transitive-callee identity.

The fresh actual `FnMut` entries are bound by ELF relative relocations at
`0x9a2570 -> 0x8fc450` and `0x9a2598 -> 0x8fc5b0`; these are the timed bodies,
not their separately emitted `FnOnce` shims. Testkit `bench/mod.rs:885–936` still
samples `&mut dyn FnMut`. `query_eval.rs:658–718` parses operands before timing
and registers each of the six pairs through these same two closures.

The total closure still inlines only the value-discriminant routing and calls
through `0x9a87f0` or `0x9a87f8`. ELF relative relocations resolve those slots to
the inspected `value_cmp` at `0x868470` and `numeric_total_cmp` at `0x86cb70`.
The promoted closure loads `0x9a87f0` into its retained register and calls that
same original comparator each iteration. There is no wrong comparator selection
or added operation in these five bodies.

Concrete new difference from the append artifact: each of the five starts moved
back by `0x20`. Comparator entry alignment now matches baseline; timed-closure
alignment differs by 32 bytes. Total hot-loop start is `0x8fc4c8` (mod64 = 8,
baseline 40); its six-byte numeric call is `0x8fc4ee` (mod64 = 46, baseline 14),
which does **not** cross a 64-byte boundary. Promoted hot-loop start is
`0x8fc610` (mod64 = 16, baseline 48). These are placement observations only;
they do not establish a causal performance explanation.

The four stable-cap rejects described by `numeric-control-current-assessment.md`
remain unresolved performance evidence. Different attained frequencies do not
prove their cause; matching instruction bodies do not qualify throughput.
The new candidate still needs the original admitted 36-lane comparison with
unchanged acceptance. Any remaining rejection must be diagnosed at its current
computation and measured execution conditions; this readback supplies the exact
current loop/callee binding for that diagnosis, not a waiver or proposed
benchmark-only alignment change.
