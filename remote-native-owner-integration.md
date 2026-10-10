# Native SERVICE producer closure

`remote-native-owner-draft.patch` is the complete ordinary source proposal for
this support assignment. It changes 17 existing source/test homes. All writes
were confined to selected Stage artifacts. Ordinary `patch --batch --dry-run`
passed against the current shipping worktree, and every changed postimage parses
with `rustfmt --emit stdout --config skip_children=true`. These checks establish
application and syntax only. No compiler, build, test, hook, full gate, source,
index, ref, forge or process lifecycle action was run here.

The shipping writer owns integration and qualification. Do not rerun the Stage
assembly scripts after integration: they are authoring tools over the original
preimage, not a second production path.

## Production interfaces and original owners

- `ServiceResolver::resolve_admitted(request, WorkspaceCapability)` is the
  bounded/native door. Native in-process, HTTP and routing resolvers override it
  with their real original producers. An opaque implementation can supply
  `workspace_certificate(request)` with the full peak requested layouts of its
  working storage, output and dynamic errors. Its original grant is acquired
  before dispatch. A missing opaque certificate refuses the exchange before
  entering host code. Row/cell ceilings are never treated as byte certificates.
- `AdmittedResolvedBindings` exposes only borrowed immutable raw bindings and
  `try_into_resident`, which returns the intact carrier if any original grant is
  bounded. Native `try_build` supplies the shared lexical Memory. In-process
  egress separately retains every original copied cell grant in an admitted
  array. The carrier destroys raw values, then cell grants, then its original
  producer frame. Its raw resident counterpart remains available to callers.
- `HttpTransport::post_admitted` carries immutable `AdmittedHttpBody`. The same
  opaque-certificate rule applies before an opaque transport is touched. A
  native transport uses `AdmittedHttpBody::try_build` and the actual neutral
  Memory buffers; its body remains owned while the JSON producer decodes it.
- `AdmittedRemoteError` is an immutable shared original error payload plus its
  native/certified producer owner and exact fallible Shared control. Ordinary
  clones share that owner. `RetainedServiceFailure` preserves denial/host/error
  classification, stable codes and original Display across federation nesting.
  Its `EvalError`/protocol/export seams are included. Source/workspace/stack and
  exchange failures retain their original hard precedence over SILENT and
  post-return cancellation. Resident adapters keep the existing raw public error
  variants and wording for existing consumers and reference cases.
- The existing `WorkspaceCapability::authored_text` is made public, without a
  second formatting body, so admitted host builders can publish immutable native
  variable/leaf text. The existing parser preparation door becomes `pub(crate)`
  for the in-process resolver. No other engine preparation body changes here.

## Original implementation migrations

`serialize.rs` now uses the existing parser ScopeTable AVL arena for scope/name
lookups and its original deterministic naming order. Actual walk spills, arena
replacement, aliases, subselect projection scratch and output strings use the
same Memory before allocation. Grammar rendering still runs the original item
engine and TextOut escape homes. Existing resident render APIs delegate this
body. The scope diagnostic uses admitted formatting rather than raw conversion.

The existing forwarding sanitizer remains the original iterative child-order
rebuild. Every frame, copied row, keep mask, native AST clone, child box and
filter-sinking condition buffer is admitted before allocation. Blank columns,
zero-column bag multiplicity, filter-read dependencies and identical-block
collapse retain their existing laws. Resident test doors call this native body;
the existing independent sanitizer oracle remains independent.

Ingestion borrows the response under its original owner, checks the original
governed row frontier before copying/remapping cells, copies through clone_term,
and uses the same checked scratch intern door with original WorkspaceTerm grants.
Surplus cells do not mint identities and ragged rows stay unbound. Original blank
scope maps now use admitted maps and immutable intrinsic label owners. Bare,
triple and composite blanks share the same mapping and native mint body.

The core owned-term fold now exposes its original native Memory traversal; the
resident API delegates that body. Composite remapping exposes the existing CDT
scanner/splicer through Memory rather than adding another decoder. Every new
label, decoded token, replacement splice, triple box and fold work buffer is
priced before construction. The original copied cell and the new remap grant
overlap until trusted publication settles their original live totals. This can
retain a conservative charge for old copied lexical/box storage that has already
died; it never approves an allocation by a post-construction census.

In-process execution uses the same admitted parser and native preparation planner
as the engine. A query-before-frame carrier keeps the original AST grant alive
through caches, evaluation and response publication. Replaced AST destruction
precedes release of its original certified live bytes. The same native evaluator
and existing governor/cell-prefix law produce the response. Its egress resolves
cells through try_owned_value_of and retains their grants beside the raw carrier.

HTTP policy, header order, credentials, stop observation and post-completion
positional-prefix withdrawal remain the existing production laws. Header strings
and Basic credentials run the same canonical Base64 Display body through Memory;
the transient secret is destroyed before release. The root-integrated
`results::from_json_with_memory(bytes, Option<u64>, memory)` is the actual JSON
producer. Decode errors, variable conversion and output rows retain the original
decoder frame; no separate decode or fee-after-decode adapter remains.

## Acceptance wiring, not a claim of completion

Five unit fixtures reuse the existing original-account ledger and cover response
extraction refusal/lifetime, shared native error lifetime, opaque dispatch
prevention, native in-process/HTTP bag production and first-growth refusal cleanup.
Their ledger assertions do not pretend to measure the allocator: libtest does
not install the counting allocator for these unit cases.

Two production integration fixtures are added to the existing counting-allocator
`correlated_owned_admission` target. They cover all four query forms through
direct, prepared, governed and prepared-governed entries, native in-process
service bags, retained output clone allocation/lifetime, HTTP decode and shared
response blank identities with duplicate rows, and a physical layout refusal
outranking SERVICE SILENT. Every measured native peak must be covered by the
source's real original admission evidence. All new fixtures are UNRUN.

The writer must remove any remaining engine-wide blanket refusal of
`QueryOptions::remote`; actual producer ownership now decides whether a particular
resolver is admitted. The existing query_completion test still expecting that
blanket refusal is stale and must be reconciled with the complete issue contract.
This is an engine admission integration obligation, not an authorized semantic
descope of native SERVICE. Root and writer were notified explicitly.

Required paired source already integrated: root JSON SELECT/lex Reader producer,
native AST clone and preparation planner, mint/blank visitor, neutral Memory,
fallible Shared controls and row/scratch original-owner carriers. Compilation and
full required acceptance remain the writer's qualification lane.
