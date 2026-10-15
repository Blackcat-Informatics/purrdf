# Complete projection application

Producer job 116264778471 in run 38687235725 completed successfully. The run's
overall failure includes the earlier stale-document shard failures; it is not
called a successful complete workflow. Its asm-projection artifact 11958379220
contains the generated patch, exported document, compiler/source receipts and
successful full qualification/aggregate logs.

The existing generator measured all 112 sites on all seven configurations,
qualified the generated document again, and replayed the complete report through
the unchanged identity and acceptance guards. Compiler 7df81381d (October 14,
LLVM 23.1.3) remained unchanged. The producer source is 49295bd27. The subsequent
025daaed5 delta changes only the optimized-package JS test, not measured Rust,
manifest or compiler inputs. Original whole-tree report identity is retained;
no report is relabeled as a final-head measurement.

The generated a/b patch applies cleanly. The applied document is byte-identical
to the artifact export. Inspection confirms only nine measured cells in five
rows change: regex pair on Neoverse, BGP on both Arm variants, chase on x86 v3/v4,
saturation on both Arm variants, and the XML writer on x86 v3/v4. All manual
columns and vector/FMA/relaxed requirements remain unchanged.

The final publication still needs its own current-head hosted checks and review
disposition. No local rebuild with a different compiler substitutes for these
producer measurements, and no source-identity guard is bypassed. Existing full
native/package/portable qualification is reused with its recorded scope because
this publication changes measured documentation cells only.
