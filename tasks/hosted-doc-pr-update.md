The hosted documentation failure was traced to two stale GlobalDictionary
error links. Their documentation now names the actual IriReadError lexical and
storage variants. The full warning-denied documentation build then exposed one
Datalog TermId link whose import was test-only; it now links to its existing
production type.

The corrected standard make doc passes all workspace documentation targets.
No runtime body changed. The census intentionally detected the changed source
and shifted line; after independent source review, only those exact proof
identities and the one location were refreshed. Final index/helper results and
normal publication are recorded in the Stage validation index. Existing full
make check and runtime qualification remain scoped to unaffected behavior.

The original hosted failure remains a failure on its original head. Current-head
hosted checks and final integration assessment remain pending; no merge or issue
closure is claimed.
