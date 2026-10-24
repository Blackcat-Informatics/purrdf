# Complete native CDT failure preservation

Keep the existing public parser, comparison and fallible-reader paths. Transfer
only the four inspected CDT tracked changes and the unique structured-error test
from the preserved donor; its checkout stays untouched. The patch uses the
repository's configured no-prefix diff format and is applied with -p0.

Expose every native variant and semantic field through the root borrowed visitor.
Keep original UCHAR and language-tag refusals inline; static grammar refusals have
explicit absent causes. Connect the same native errors through standard source()
using the existing macro. UcharError needs Display/Error at its lexical home for
that chain; do not add a replacement decoder or dependency. Diagnostic accounting
still owns only the existing IRI/key strings; the added causes have no heap data.

Qualify public failures after input destruction, real lexical/storage wrappers,
all comparison classifications, accepted neighbours, zero-allocation inspection,
existing grammar/output/resource controls and actual workspace/binding callers.
General semantics stay native. Use strict lint, normal hooks, current-head CI and
independent review; land only through ghprsq and verify archive/cleanup. Parent
558 stays open. No upstream work or external repository edits are authorized.

Repository memory was recalled; its donor observations were checked against
current production source. The existing architecture snapshot is stale/unusable;
current source establishes these local lexical/error callers. No relevant ADR
file or nested AGENTS.md was found under the affected paths.
