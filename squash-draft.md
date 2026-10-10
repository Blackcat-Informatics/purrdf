# Render deep Turtle blank chains through the native iterative work list

Draft only: final integration suffix and hosted checks are still pending. This
file is preparation, not a merge-readiness verdict or final squash-notes file.

Closes #472 after verified integration. The native structural renderer previously
recursed through property/object/list output and expanded indentation with depth.
It now uses the existing heap WorkList, retaining the forty-level structural-key
guard and saturating indentation after forty actual levels. Shallow output keeps
its original bytes; beyond that boundary only the documented indentation policy
changes. Cycles, shared/quoted references, strict collections, named graphs and
statement metadata remain intact. Equality follows the same authored bounded key
and identity law as ordering. Singleton objects do not build unused keys.

Production changes live in rdf-core's existing turtle_render home. The RDF target
supplies the shared deep corpus and first-party renderer benchmark; Makefile
registers its portable seven-case execution. Documentation records both that
portable contract and the benchmark's actual shared IRI escape census site.
No dependency, semantic feature, namespace default, larger stack or alternative
renderer was introduced. The preserved renderer donor remains unshipped.

Required local full make check passed in the original execution95104, including
strict compilation, static/hygiene/generated controls, native workspace tests and
doctests, standalone preserve-order consumer and optimized WASM release build.
Native and optimized portable corpora each pass7/7 at actual100k/1M sizes over
packed/default Turtle/named TriG routes, with all six exact frozen length/digest
receipts and metadata reparse/rewrite. Exact original-neighbor captures, guard
permutations, cycles/shared/quoted controls, a100k collection and continuation
indentation pass. Direct key coherence passes. Original benchmark3/3 is report
only; host medians4.645µs/236.3ms/2.928s are observations under concurrent load.

The actual base eb1fd88b introduced regular-role portable registrations beside
the Turtle list. The Makefile conflict was resolved by retaining both complete
lists through normal synchronization42ca16721. Independent review found no
renderer interaction from the base's shared lexical-scope/Tarjan changes. The
combined strict core/RDF all-target check passes; native/portable suffix results
must be finalized from integration-suffix.log before merge. Unchanged full-gate,
benchmark and original-capture evidence remain applicable to the unchanged
renderer/test inputs.

Published review's stale portable totals and exception list were corrected to
57 named cases/16 targets/62 executions/20 invocations. Hosted census failure was
corrected by registering the Turtle benchmark against core.canon-escape, which
the measured prefix-free fixture actually calls through the shared IRI writer.
The original document coverage validator passes; this is not an assembly-matrix
claim. The review reply was accepted and its thread resolved. Final-head hosted
CI/Docs/CodeQL still require terminal qualification before final notes.

Authoritative plan: selected Stage plan.md. Required evidence, complete feedback,
independent implementation/task/completion reviews, raw logs, benchmark estimates,
exact original captures and cleanup report are in the selected
.stage/turtle-writer-deep-blank-node-chains directory. Archive that complete
selected evidence and final notes through ghprsq before owned cleanup. Completion
still requires verified merged PR, closed issue and Stagectl-owned branch/worktree
cleanup; no prior donor or recovery archive is eligible for removal by this merge.
