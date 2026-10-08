# Captured Stage tree: streaming pack size and integrity

2026-10-08. **PASS for a fresh pack of the historical frozen preview tree;
final current Stage capture/archive remains pending.** This does not relabel the
previous large disk file as inspected or complete current replay.

Supported capture52570 exited0 and preserved all10693 files/2039421301 file bytes
in tree0df0b33c9689ed5201ed911953b698e1ac92925e, including the original raw
credential guard, stable-copy/full-mode/temp-index/tree checks. Those writers
were stopped during capture; later Stage updates invalidate final reuse of that
tree. Pack generation93717 exited0, but attempts to stat its large output file
were blocked before execution by missing pre-call attribution snapshots. Small
report and directory reads continued to work normally. No guard was disabled,
changed or bypassed, and no agent was asked to perform the rejected stat.

The different, normally guarded streaming operation73131 was approved and
executed from the captured Git tree, without reading that disk output:

```bash
git pack-objects --stdout --revs --threads=8 < /opt/purrdf-454-capture-preview-20261008.aIIvlp/pack-input.txt |
  tee >(wc -c > /opt/purrdf-454-pack-stream-count-20261008.txt) |
  git index-pack --stdin --strict
```

The enclosing shell enabled pipefail, retained the pipeline exit and waited
before exiting. Actual73131 terminal0; strict index-pack returned
`pack 4a89e87f52f996ca178e97ed0d0024853396203c`. A separately approved read of
the small counter record returned exactly **1259814354 bytes**, or
**1.1732935477 GiB**. The measured stream is the same stream Git validated,
not a second independently generated pack. Its margin below2147483648 bytes is
**887669294 bytes**. Every object of that Stage tree was packed, without excluding
already-remote objects, so this is a conservative Stage-only pack measurement.

Systemd scopepurrdf-454-pack-stream-verify-20261008 invocation
e876fe1c6c84439c84f90072a0cec147 was launched with MemoryMax64GiB and
MemorySwapMax0. Git object plumbing wrote the validated pack/index into the
repository object database but changed no source index, branch or publication
ref. This is neither a source commit nor a push/merge. The original uninspected
large disk output remains preserved outside Stage.

The preview predates the three complete publication receipt copies38files/
10641bytes and later monitoring/review records. The independent archive review
identified that missing replay gap; subsequent selected receipt preservation
closes that scoped gap, not the historical preview. Final full CI, PR feedback,
debt, writer freeze/current raw and member policy/reconstruction, supported
fresh capture and final pack admission still apply. Source/head changes require
their own candidate assessment. No final ghprsq/archive/cleanup PASS is claimed.
