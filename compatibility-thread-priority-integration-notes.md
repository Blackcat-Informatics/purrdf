# Compatibility capture priority correction

Writer's actual qualification found the unchanged independent differential in
native_regex_storage.rs: anchored `^(a*?)*$` on `aa` produced native group 1
`1..2` while the original compatibility implementation produced `0..2`.
This was a semantic failure, not a capacity waiver. The oracle/fixture is unchanged.

`compatibility-thread-priority-draft.patch` changes one existing native home,
xpath/pike.rs. Only Stage files were written. The correction is UNCOMPILED and
UNRUN here; writer owns qualification and integration.

The actual production route is Searcher::linear_find -> Sets::first_start ->
Pike::walk. Its existing Mode::Walk parks only one locally viable continuation,
using capture-free future information and cached decided moves. That proof says
which path can finish; it does not preserve the visited epsilon controls of every
earlier ordered thread. Compatibility's first-tag merging is global to the input
position. Discarding those other closures permits a subsequent nullable outer
iteration to overwrite the first capture start, matching the observed `1..2`.

For the explicit compatibility law, walk now delegates to the same Pike's existing
find_from/Mode::Threads/search body. It retains the complete ordered thread set
and its existing compatibility visited-control equivalence, using the same Nodes,
captures, sets, Budget, reserve_match, storage callback and fallible buffers.
The set-certified leftmost start remains the lower search boundary. Earlier starts
have already been excluded; later injected starts remain lower priority and cannot
replace a completing earlier start. No second AST, NFA encoding, matcher, buffer
body, allocator path, or semantic feature is introduced.

The dated laws retain their existing optimized one-thread walk and decided-move
cache. Compatibility find/replacement use the real ordered thread population;
each vector/state/capture allocation is admitted by the existing shared native
Budget paths. Physical refusal remains a typed original-owned error, and caller
selected limits remain authoritative. This changes the work population and may
change compatibility performance; no benchmark or speed claim is made. A future
single-thread optimization would need to preserve this exact first-tag priority
law rather than treating a capture-free reachability proof as sufficient.

Required acceptance in the sole managed lane:

- Re-run unchanged compatibility_preserves_nullable_repeat_capture_priority,
  including every group/span for greedy/lazy star/plus/optional/finite counts,
  empty alternatives, nested nullable captures, anchors and UTF-8 inputs.
- Re-run unchanged compatibility replacement differential (empty-match
  suppression, captures/template references and UTF-8 progression).
- Retain Unicode16/category/name/set and terminal-newline anchor differentials.
- Retain native compiler/matcher/replacement physical peak and original-owned
  refusal cases. The ordered population must fit its actual callback admission,
  rather than the previous one-thread population or output LIMIT.
- Retain all dated-law native machine/reference acceptance; the conditional must
  not change their grammar, captures, priority or arbitrary count law.

A successful dry-run or Rust syntax parse only establishes applicability/syntax;
neither establishes the semantic correction. Any further real failure must be
fixed against the unchanged differential before this producer is qualified.
