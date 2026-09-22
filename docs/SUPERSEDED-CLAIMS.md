<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Superseded claims

A claim this repository once made in its own source, and no longer makes.

## Why this file exists

A doc comment that states an invariant is a conclusion someone reached with the code
in front of them. When the code moves, the honest edit is to rewrite the comment —
and a rewritten comment leaves no trace that the old reading was ever the rule. The
next reader who finds the old claim quoted in a design note, a downstream project, or
a review comment has no way to tell whether it was wrong, whether it changed, or
whether they are looking at a bug.

So a reversal gets recorded here as well as fixed in place. Each entry names the claim
as it stood, why it was believed, what changed, and where the new rule lives. This is
not a changelog: it holds only claims that were once *stated as invariants* and are now
false.

It is **not** an authorization surface, and nothing here excuses undone work. An entry
means a claim was corrected, never that a defect is accepted.

## The claims

### A prepared shapes product cannot carry host relations

**Was stated in** `crates/shapes/src/product/mod.rs`, on `ShapesProfile::CORE`:

> What the profile genuinely excludes is the one binding no shapes graph can describe:
> a product of this profile is prepared against the EMPTY property-function registry,
> because `sh:sparql` bodies that call host relations depend on wiring the shapes graph
> cannot state and a product therefore cannot carry.

**Why it was believed.** A relation registry is host wiring. Nothing in a shapes graph
declares it, so a restoring process cannot rebuild it, and the profile took that to
mean a product could not be *bound* to one either.

**What changed.** The premise conflated two different things: what a shapes graph can
describe, and what a product can state as a requirement. The identity rows exist
precisely for the bindings a shapes graph cannot describe — that is what the
implementation-identity row already did for native functions. A product binds the
environment it was prepared against, and `PreparedShapes::to_product_for_host` is how a
host that wired a relation or declared a namespace says which.

**The rule now.** A product binds the environment it was prepared against.
`to_product` was prepared against nothing, so restoring it under a host that wired a
relation is still refused — a fact about which writer a caller used, not a capability
the profile lacks. Pinned by
`product::tests::a_product_written_for_a_host_restores_under_that_host_s_relations` and
its empty-registry sibling.

### A SPARQL-bodied function's body is deliberately not folded into its identity

**Was stated in** `crates/sparql-eval/src/user_fn.rs`, on `content_fingerprint`:

> a SPARQL-bodied function's parsed body is deliberately not folded either: the only
> stable byte form available for it is the algebra serializer's rendering, and making
> that load-bearing would turn a cosmetic wording change in an unrelated module into a
> silent invalidation of every persisted artifact.

**Why it was believed.** `UserFunction::body` held parsed algebra, and the only way to
get bytes out of algebra was the serializer — whose output is a rendering choice, not a
fact about the function.

**What changed.** The objection was sound and is answered rather than overruled.
`UserFunction::body` is now the declaration's own SPARQL **text**, which no serializer
can reword. Folding it closes a real hole: two registries whose functions declared
identical parameters, arities and return types but carried different bodies digested
identically, so a persisted artifact bound to one would open against the other.

**The rule now.** The `Declared` population folds the body text, canonicalized. It
still does **not** fold the environment a body was later bound against, because a
`Declared` entry must stay rebuildable at restore from the shapes graph alone — folding
host wiring in would refuse a valid restore. Pinned by
`the_declared_digest_separates_two_identical_declarations_with_different_bodies` and
`binding_does_not_move_the_declared_digest`.

### A custom function IRI is resolved dynamically, never at parse time

**Was stated in** `crates/sparql-eval/src/engine.rs`, justifying why the user-function
registry is excluded from `check_plan_matches_relations`:

> `Function::Custom` is resolved dynamically at evaluation time (never at parse time,
> unlike a property-function predicate or a `Custom` aggregate's admission).

**Why it was believed.** It was true of every function kind at the time: a call-position
IRI was looked up in the registry while an expression was being evaluated, so no plan
could depend on the table.

**What changed.** A SPARQL-bodied function's body is now parsed and feasibility-ordered
against an extension environment before evaluation begins, so that half of the registry
*is* parse-time-bound.

**The rule now.** The two resolutions are distinguished rather than merged:
expression-position `Function::Custom` dispatch is still dynamic, while a SPARQL body's
parse binding is static and carries the environment identity it was bound against.

### The SPARQL engine holds the parse-time configuration for every query it runs

**Was stated in** `crates/sparql-eval/src/engine.rs`, on `NativeSparqlEngine`'s
`parser_options` field and its `with_parser_options` setter:

> Parse-time configuration (the extension-function namespace set), applied to every
> query and update this engine parses.

**Why it was believed.** It was true when written, and it was the only home available:
a `ParserOptions` had to live somewhere the parse could reach, and the engine is what
performs the parse.

**What changed.** An environment gained base options of its own, and for a while both
existed. That is not a redundancy, it is a fork: `prepare_for` derived from the
engine's field, so every ordinary query — every `sh:sparql` body, target, rule and
node expression — read the engine's configuration, while the environment's reached
exactly one door, the function-body bind. A host could declare a relation namespace,
watch it hard-error an unregistered IRI inside a `sh:SPARQLFunction` body, and watch
the identical IRI in a `sh:sparql` body on the same host silently become an ordinary
triple pattern and conform green. Two parse configurations wearing one name, and the
symptom was this repository's own silent-reclassification bug at a sibling door.

Several hosts had already been bitten without anyone noticing: the conformance
harness, the text search tests and the geo rewrite tests each set engine options AND
built a separate environment, so the declaration they thought they had made was
already being dropped.

**The rule now.** The engine holds no parse configuration. `parser_options`,
`with_parser_options` and `parser_options_for` are gone, and an `ExtensionEnv` is the
only thing that says how a SPARQL text is read — on the query lane, the UPDATE lane,
the function-body bind and the product identity alike. Deleting the field rather than
leaving it unread is deliberate: a setter that silently stopped taking effect would be
the same class of defect one level further down. Pinned by
`function_body_relation::a_declared_namespace_reaches_a_sparql_constraint_body` with
its two valid neighbours, and by
`engine::tests::every_chunk_worker_sees_the_declared_parser_options`.

### A `sh:SPARQLFunction` body is opaque to the footprint walk because the walk does not read it

**Was stated in** `crates/shapes/src/footprint.rs`:

> A `sh:SPARQLFunction` body is a SELECT that may carry one, and this walk does not
> read it.

**Why it was believed.** It is a true description of the walk, and `OPAQUE_QUERY_TEXT`
is the conservative answer, so the reason looked sufficient.

**What changed.** Nothing in the code — this entry records a *strengthened* reason, not
a reversal of behaviour, because the weak reason invited a change that would have been
unsound. Reading the body would make the footprint environment-dependent, and a
footprint is derived at plan time from a `Shapes` value with no environment in scope,
cached on a `PreparedShapes`, and reused across validations that each install a
different registry. A precise footprint would therefore be consumed under an
environment it was not derived under.

**The rule now.** The body stays opaque because opaque is TOP, and TOP is the only
answer correct under every environment simultaneously. Pinned by
`plan::tests::a_function_body_s_footprint_does_not_depend_on_the_environment`.
