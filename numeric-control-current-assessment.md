# Numerical controls: computation and timing admission

No current performance acceptance. This readback concerns the retained bag-summary benchmark artifact and original baseline; the new native-row layout artifact is not yet built.

The original assessor uses `Change::between` from the shipping testkit, with 10,000 seeded bootstrap resamples and 95% confidence. A lane is Regressed only when its whole relative-change interval lies above +1%. All 36 original lanes must avoid that verdict. No threshold, sample/body, baseline or assessor was changed.

Current benchmark source, XSD `ops.rs` and `numeric.rs` match the original baseline source exactly. Both retained benchmark and XSD library fingerprints agree on compiler, target, profile, compile_kind, features and rustflags (`-D warnings`). These control loops parse their operands outside timing, black-box each input, and call the original arithmetic/comparison functions. They instantiate no evaluator row.

`numeric-emitted-comparison.txt` and `numeric-timed-loop-comparison.txt` record identical relocation-normalized bodies for the original `value_cmp`, `value_total_cmp`, `numeric_total_cmp`, and both actual sort-order timed closures. Their code placement differs; in the bag-summary current total-comparison loop, the six-byte numeric call crosses a 64-byte boundary. This is an observed placement difference, not a causal proof or an acceptance waiver.

The new read-only projection `numeric-prior-frequency-readback.txt` shows CPU11 policy caps repeatedly switching between 5,187,500 and 3,000,000 kHz during the retained panel. The rejected decimal promoted, double total and double promoted controls have unequal/moving caps. The rejected integer/double promoted control also has moving caps. These runs do not isolate software cost.

Four rejected numeric lanes retain the same observed cap throughout both halves and still require explanation: integer total +9.03%, integer/double-window total +4.44%, decimal/double-u128 promoted +1.67%, decimal/double-bigint total +1.09%. Their observed frequencies are recorded separately; there is no attribution to row layout and no arithmetic source change justified by the existing evidence.

The read-only admission in `recovery-native-layout-measure.sh matrix` refuses a launch while live unrelated build/link/compression jobs are present, checks a stable policy cap before timing, preserves the unchanged original36 runner's scheduler/frequency observations, and samples newly arriving heavy work during the matrix. A cap change or competing job rejects the environment separately from the retained original performance verdict. Only specifically supplied root-owned geo scopes may be frozen, with automatic EXIT thaw. No governor or sibling-process changes, no automatic repeat after rejection.

Next: rebuild the changed candidate under the original profile, compare fixed-work DISTINCT instructions, then run one admitted original36 matrix. Any remaining numeric rejection needs actual current emitted-loop/callee and hardware-counter diagnosis; predecessor layout or unchanged-source observations alone do not discharge it.

## Refreshed native-layout artifact

The candidate is now rebuilt with the original bench profile; selected benchmark fingerprint fields still agree with baseline (compiler, target, profile, compile_kind, features and flags). Relocation-normalized bodies remain identical for value total/partial, numeric total and both actual timed comparison loops: `native-layout-{value_total,value_partial,numeric_total,timed_total,timed_partial}-{baseline,current}.asm`. All three comparator entry addresses now match baseline modulo64; timed-loop entries differ32 bytes. The formerly crossing numeric call no longer crosses the64-byte boundary. This is emitted-code evidence, not attribution or acceptance. No refreshed original36 timing has been admitted while unrelated heavy host work remains active. Numeric controls remain independently unqualified.

## Final append artifact readback

After the confirmed row-append correction, the five normalized numeric/timed-loop bodies still match the original baseline exactly (`native-append-*-current.asm` against `native-layout-*-baseline.asm`). Actual current starts/sizes are: value_total0x867bb0/30, value_cmp0x868490/1359, numeric_total0x86cb90/1240, total timed closure0x8fc470/346 and promoted timed closure0x8fc5d0/321. Both timed-closure starts now equal baseline modulo64 (48 and16 respectively), and their numeric call/loop alignment consequently matches baseline. Comparator entry placement differs by32 modulo64. ELF relative relocations resolve both actual timed loops' value_cmp slot0x9a8820 to0x868490 and numeric_total slot0x9a8828 to0x86cb90, the inspected original function bodies. `native-append-bench-identities.sha256` retains actual candidate query and physical benchmark identities. These readbacks rule out changed operations or wrong callee selection in these controls; they do not establish performance or attribute predecessor regressions to alignment.

## Post-helper artifact independent readback

The refreshed query executable matches native-helper-bench-identities.sha256.
Independent readback in reviews/native-helper-numeric-control-readback.md binds
both actual FnMut dispatch slots and comparator GOT slots to the five inspected
bodies, which remain identical to the normalized original baseline and append
artifact. Five native-helper-*-current.asm receipts retain exact disassembly.
Comparator starts match baseline modulo64; both timed loops differ by32 bytes,
and the numeric call does not cross a cache-line boundary. No changed operation
or wrong callee was found. This read-only observation does not explain the four
predecessor stable-cap rejects, establish a clock cause or qualify throughput.
The first admitted current original36 run and causal diagnosis of any resulting
reject remain mandatory. No additional timing or build ran for this review.
