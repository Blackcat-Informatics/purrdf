# Ground ingress and original quoted-child ownership

`substitution-ground-owner-draft.patch` is a standard unified proposal against
the current source postimage. Only Stage files were written. A read-only
`patch --batch --dry-run -p1` passed for all five homes, and rustfmt parsed the
five Stage postimages with `--emit stdout --config skip_children=true`.
No compiler, test, full gate, source application, or public-query acceptance was
performed by this helper. Formatting and warning-free compilation remain the
shipping writer's coherent source-batch checks.

Apply with root's `substitute-request-seed-owner-draft.patch`, which supersedes
the earlier pattern-only packet. The paired helper signatures agree; the hunks
are in separate portions of substitute.rs. The source writer alone applies both.

## Actual API and ownership

At the existing substitute.rs ingress home:

```rust
ground_term_from_value_with_memory(
    value: &TermValue,
    memory: &mut Memory<'_, LexicalFrame>,
) -> Result<GroundTerm, GroundFailure>;

ground_term_from_id_with_memory<D: DatasetView>(
    dataset: &D, id: D::Id,
    memory: &mut Memory<'_, LexicalFrame>,
    source_error: impl FnMut(D::ReadError) -> GroundFailure,
) -> Result<GroundTerm, GroundFailure>;

clone_ground_with_memory(
    ground: &GroundTerm,
    memory: &mut Memory<'_, LexicalFrame>,
) -> Result<GroundTerm, GroundFailure>;
```

All three feed the existing subject/predicate/object postorder grounding body.
The clone door uses borrowed GroundTerm/NamedNode components, shallow immutable
leaf clones, the same native pending-stage walk and the same admitted box factory.
There is no second cloning traversal. It allocates fresh child boxes while
retaining each lexical leaf's original immutable text owner.

Memory admits actual `Layout::array::<OpenGroundTriple<C>>` replacement buffers
before fallible growth, and `size_of::<GroundTriple>()` before the existing
fallible box factory. Work buffers die before release. On success only work
capacity is released; the fresh boxes remain in the enclosing original query
grant. On failure local and pending GroundTerm children die first, then the delta
to the entry live-byte checkpoint is released, including anticipated replacement
layouts for an allocator that did not produce storage. A failed shrink remains an
operational failure; the actual first generic backing cause stays in the original
QueryWorkspace account. Never reuse a failed native computation as a success.

For a separately owned prepared grounding, the same body has standalone wrappers
`ground_term_from_{value,id}_admitted` returning the private ordered carrier:

```rust
pub(crate) struct AdmittedGroundTerm {
    pub(crate) term: GroundTerm,
    pub(crate) allocation: Option<WorkspaceAllocation>,
}
pub(crate) enum GroundFailure {
    Diagnostic(RetainedDiagnostic),
    Operational(EvalError),
}
```

The carrier is not Clone and has no public/raw extraction API. The term field
must die before its original child grant. Trusted query publication must move
both fields together or use the original aggregate Memory door; it cannot move
the raw term into a query and discard its child grant. Intrinsic SharedText leaf
grants remain attached when leaves are shallow-cloned or the carrier is moved.
The resident adapters retain the old RdfDiagnostic surface and native semantics;
actual physical allocation refusal follows their existing infallible allocator
contract. Bounded callers never use that adapter. `GroundFailure::into_resident_diagnostic`
is private same-module visible, as root's resident wrappers require.

After root's request/seed migration, its aggregate doors replace production uses
of the old private value-door wrapper. During compiler normalization, keep that
resident reference wrapper and any now test-only standalone value wrapper under
cfg(test) or remove them if the final production caller map makes them obsolete;
update their existing rustdoc links in the same source batch. Do not add a dead-code
allow or retain an unused parallel ingress route. The prepared id caller currently
at execution.rs `bind_id` still needs its final original-owner migration; root and
writer own that caller and final AdmittedQuery publication.

## Lexical and failure law

Blank qualification calls the existing core
`encode_blank_label_with_memory(..., LabelAlphabet::Unconstrained, memory)`.
Borrowed/default-scope spellings publish through authored_text; an owned encoded
String publishes without copying through the new narrow LexicalFrame::finish_text.
That factory checks the exact surviving String capacity, admits
SharedText::allocation_layout::<WorkspaceAllocation>() before taking the payload,
and retains the original String-before-grant owner through actual allocator refusal.
It assumes construction scratch has already died. Its two invariant diagnostics
use the existing retained NativeDiagnostic Internal home, never an unpriced String.

IRI recognition uses root's already shared native
`is_absolute_with_memory -> IriReadError`. Success creates no parsed copy.
Its original lexical error remains owned under its original frame through retained
rendering. A public borrowed IriTermDisplay at the existing algebra error home
preserves NamedNode's exact `invalid IRI ... in term position: ...` spelling;
ParseError::Iri uses that same body. Relative IRI failure keeps the original
`relative IRI reference in term position (no scheme)` reason. Storage failures
map through the original frame and remain Operational.

The existing LANGTAG_PROFILE parser borrows input and allocates no storage.
Its native reason/code and original spelling render through RetainedDiagnostic.
The literal branch preserves the old choice exactly: a present language selects
the standard language datatype and retains direction, ignoring the supplied
datatype; an absent language validates the supplied datatype and discards a stray
direction. Datatype resolution on the id door occurs in the original order.
The first non-IRI quoted predicate is refused before entering its object.

The id source callback must move the original D::ReadError into its outer typed
account, e.g. `GroundFailure::Operational(ctx.workspace.source_error(error))`.
It must not clone or render a generic source error before the final boundary.
Lexical diagnostics never become UNDEF/unbound cells, and physical refusal while
rendering a lexical diagnostic remains the operational outcome.

## Destruction and proposed evidence

tree.rs migrates GroundTriple and TriplePattern Child destruction to the existing
allocation-free DismantleTree/dismantle_tree home. Ordered subject/object selectors
use the original child slots as continuations, including both siblings. General
graph/expression/path drop remains outside this narrow hunk. No new dismantler or
drop work-list implementation is introduced. This closes GroundTerm cleanup and
root's SeedOnly TriplePattern cleanup after admission/allocator refusal.

Eight new fixtures are proposed and UNRUN:

- Original ground boxes and qualified lexical leaves survive input, capability
  and workspace destruction, then release the full original account.
- Fresh grounding and paired cloning join an already allocated enclosing query
  String's original Memory; only exact fresh GroundTriple boxes survive scratch.
- Early/middle native admission refusals destroy partial children and move the
  first typed cause once, with generic error clone count zero.
- SharedText publication preserves the original String pointer, shallow-clone
  lease, and account lifetime.
- Shared control admission refusal precedes String extraction and preserves the
  original payload and exact typed cause.
- Non-IRI predicate refusal precedes an unvisited malformed object.
- A non-latching dataset failure during literal datatype resolution releases a
  previously completed quoted subject and preserves the exact typed source cause.
- Ground and pattern triples with both subject and object children, 100000 levels
  deep on the existing 128 KiB thread, drop with zero allocator calls.

Existing 600-value generated grounding/reference cases, borrowed-id fixtures,
first-refusal cases and 100000-depth value tests now reach the same native body.
Their recursive reference independently checks the traversal/shape/order law;
the independent exact IRI scanner/error evidence is in root's IRI companion,
since reference leaf helpers intentionally share native lexical admission.

Final real-entry acceptance is still required in the sole qualification lane:
owned/borrowed/prepared value/id prebindings, focus injection, all four query
forms, substitutions inside correlated/nested scopes, blank identities and bags,
partial/source/admission failure precedence, retained prepared/query ownership,
typed errors under low capacity and physical allocator refusal. The request/probe/
query rewrite must keep all enclosing arrays, boxes and final original grant;
this packet does not certify those other production paths by itself.
