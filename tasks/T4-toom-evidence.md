# Toom-3 partial evidence (uncommitted)

Scope: production `crates/xsd/src/bigint/compat.rs` and
`crates/xsd/src/exact/cost.rs`; no benchmark source changes. This is not
qualification of either numeric issue. The very-large transform, divide,
radix, architecture, host and full gates remain.

## Fixed-work native controls

The unchanged `xsd_exact_karatsuba` benchmark uses `label * 9` decimal
digits, not `label` binary limbs. Decimal digit bounds give exact binary
limb counts 8, 23, 60, 120, 240, 479 and 957 for labels 16, 48, 128, 256,
512, 1024 and 2048. The existing release bench was built with standard
`cargo bench -j 8 -p purrdf-xsd --bench exact --no-run`; each `perf stat`
used three repeats of the bench's `--test --exact` one-operation path. These
were nonexclusive and are instruction diagnostics, not timing claims.

| Toom cutoff | 240 limbs | 479 limbs | 957 limbs | 40k-digit growth product |
|---:|---:|---:|---:|---:|
| 128 | 596,575,615 | 598,783,371 | 605,354,437 | unrun |
| 512 | 596,414,977 | 598,640,032 | 607,031,815 | 627,405,110 |
| 1024 | 596,415,289 | 598,639,978 | 605,864,974 | 634,131,769 |
| 512, single-destination reconstruction | 596,415,043 | 598,640,044 | 606,690,453 | 625,423,827 |
| 512, plus direct positive point evaluation | 596,414,821 | 598,639,421 | 606,623,087 | 624,988,760 |
| 128, same final algorithm (diagnostic build) | 596,469,407 | 598,212,383 | 604,246,383 | 627,951,283 |
| 256, same algorithm (diagnostic build) | 596,415,550 | 598,925,873 | 604,246,228 | 624,988,649 |
| 512, plus checked exact right shift for interpolation /2 | 596,415,064 | 598,639,708 | 606,581,367 | 624,752,281 |

The empty harness was about 595.356 million instructions in each build.
Schoolbook controls on the 128-cutoff build were 596,204,133 at 240,
598,616,277 at 479 and 608,267,972 at 957. The 512 build's temporary
test-only trace showed 957 x 957 enters Toom once, then has five 319-320
limb Karatsuba children, each with three 159-160 limb children. The
nonmonotonic result therefore depends on recursive dispatch. The test-only
trace was removed.

The final two rows use the same unchanged benchmark and native instruction
event. The shared checked carry-add reconstructs directly into one destination
and evaluates positive points 1 and 2 into one destination each. At 957 limbs
this reduced the first 512-cutoff computation by 408,728 instructions, but it
still exceeds the existing Karatsuba route at the 1024 cutoff by 758,113.
The current 512-limb dispatch has no qualifying threshold claim.
The 128-limb diagnostic was built from the same optimized algorithm, then
the source cutoff was restored to 512. Relative to that final-source 512
row, 128 costs 54,586 more instructions at 240 limbs, saves 427,038 at
479, saves 2,376,704 at 957, and costs 2,962,523 more on the 40k-digit
growth case. This is a recursive-dispatch diagnostic, not a selected policy.
It must not be conflated with the historical 128 row above, which used an
earlier reconstruction and point evaluator.
The 256-limb diagnostic passed the native differential, governed peak and
native/actual Node-WASM large-product oracle. Relative to 512 it keeps the
240-limb and 40k-digit cases even, saves 2,376,859 instructions at 957,
but costs 286,452 more at 479. Source was restored to 512 afterward. No
single cutoff is selected by these four fixtures, and neither 128 nor 256
is a qualifying performance policy.
The current source keeps 512 and routes exact interpolation division by two
through the existing admitted signed right-shift, after asserting evenness.
This preserves the native/actual Node-WASM large-product oracle and governed
peak controls. It saves about 41,720 instructions at 957 limbs and 236,479 on
the 40k-digit case against the preceding 512 build. It still loses the
957-limb existing Karatsuba control by 716,393 instructions. The optimized
algorithm and threshold remain provisional.

## Physical controls and bound

The behavioral differential passed 1/1 at cutoff neighbors, asymmetric
operands, 1024 limbs and all four sign pairs on the current source. The
admitted physical peak/cost traffic control passed 1/1 on current source at
511/512/513, 1023/1024/1025 and asymmetric neighbors through 40k decimal
digits. An independent 325,271-byte large-product transcript matched frozen
digest `601791f97aa1152f` in native and actual Node/wasm32 runs, 4/4 each.
A temporary allocator probe on the earlier reconstruction at the 512 cutoff
reported 511/512/513-limb allocations 1681/960/960, requested bytes
713,920/560,704/560,992, peaks 49,056/42,352/42,408 and admitted bytes
65,408/65,536/65,664. At 1023/1024/1025 limbs allocations were 2850 each,
with peaks 84,576/84,728/84,784 and admitted bytes
130,944/131,072/131,200. The prints were removed.

For a balanced Toom node let `n=max(la,lb)`, `s=ceil(n/3)`, and
`m=min(la,lb)>2s`. The six copied chunks hold at most `la+lb<=2n` words.
The largest retained parent set during a recursive point multiplication is
at most `2n+12s+16` words (split chunks, two endpoint products, already
interpolated values and the two current point operands). Each child has at
most `s+3` limbs per side, so its eight-result reserve is at most `16s+48`.
The sum `2n+28s+64` is below `8(n+m)` for `n>=512`. Scoped `p1`, `pm1` and
`p2` owners are gone before the final single-destination offset carry-add;
its retained coefficients and one `n+m` destination fit under the envelope.

The provisional work charge uses `la*lb + 16(la+lb)+1` when the shorter
operand reaches the Toom cutoff. Its five child products have lengths at
most `s+3`. Copies, point evaluation, exact one-limb interpolation divisions
and offset carry-add reconstruction account for at most `128s+128` limb passes at that
node. For `s>=171` the five child charges plus that local bound fit the
charge above; if `s+3` reaches the three-way cutoff, the child charge's
linear coefficient also rises to 16, but then `s>=509` and the quadratic
slack is larger. This symbolic bound and the cutoff choice need independent
review; no issue closure or performance win is claimed.
