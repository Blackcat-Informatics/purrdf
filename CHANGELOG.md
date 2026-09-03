# Changelog

All notable changes to the PurRDF crate suite are recorded here. The suite
ships one lockstep version across crates.io, PyPI, and npm; from 1.0.0 a
breaking change bumps the major version, a minor bump is additive, and a patch
bump is bugfix-only. The C ABI (`purrdf.h`) is versioned separately and remains
0.x.

## [Unreleased]

## [1.0.0] - 2026-09-02

The first release under full semantic versioning. The tree is the 0.13.0 tree:
0.13.0 was published only to create the crates.io records for `purrdf-cdt`,
`purrdf-geo` and `purrdf-text` — Trusted Publishing cannot create a crate that
does not exist — and 1.0.0 is the same code republished through Trusted
Publishing across all 21 crates, PyPI and npm. There is no functional change
between the two.

### Documentation

- **release:** From 1.0.0 a breaking change bumps the major version, a minor
  bump is additive, and a patch bump is bugfix-only; the changelog's
  **BREAKING** markers name each one. The C ABI (`purrdf.h`,
  `PURRDF_ABI_MAJOR.PURRDF_ABI_MINOR` = 0.7) is versioned separately and
  remains 0.x. The changelog header, its `cliff.toml` template and the release
  docs now state that policy instead of the pre-1.0 one.
- **release:** `make bump` now regenerates `purrdf.h`, because cbindgen derives
  `PURRDF_MINOR` from the crate version and the 0.13.0 bump left the committed
  header one integer behind, failing `capi-check` in CI.

## [0.13.0] - 2026-09-02

This release re-founds the aggregate algebra on the SPARQL specification's own shape, adds a
caller-extensible aggregate registry (with a first-party statistical set), retains and enforces
the `VERSION` declaration, adds RDF 1.2's `ADJUST` and the underlying F&O temporal operation
table, corrects several results-format spellings, adds `LATERAL` (SEP-0006) surface syntax
alongside a corrected correlated-evaluation substitution, and rebuilds `EXISTS`/`NOT EXISTS` on
SEP-0007's defensible substitution semantics (one definition, two proven-equivalent strategies,
and the Part 3 assignment restriction). It carries a large number of breaking surfaces; each is
called out below with what a consumer must do.

### Bug Fixes
- **BREAKING** **core:** `check_provenance` now measures the sidecar against the dataset in BOTH
  directions, and an empty quad set means "the dataset is empty", not "check nothing". Rule 3
  (every dataset quad has at least one occurrence) previously ran only when the handle slice was
  non-empty, so a caller that forgot to collect its handles — or passed `&[]` to mean "skip" —
  got an `Ok(())` indistinguishable from a real pass. The parameter is now the dataset's
  complete quad-handle set, rule 3 runs unconditionally, and a new rule 4 refuses every
  occurrence whose quad is not in that set with
  `ProvenanceError::DanglingQuad { occurrence_index, quad_index }` — the quad-axis twin of
  `UnknownUnit` / `UnknownArtifact`. An empty sidecar for an empty dataset still passes (every
  rule ran over zero elements, which is a checked pass); a non-empty sidecar checked against
  `&[]` now fails, as does an occurrence for a quad outside a non-empty set. A caller that passed
  a subset of its quads, or `&[]` to run rules 1–2 alone, must now pass the full set. The
  signature is unchanged and `ProvenanceError` is `#[non_exhaustive]`, so the new variant is
  not itself a source break; the accepted-input set is what changed.
- **sparql-algebra:** A language tag's base direction is exactly `ltr` or `rtl`, lower case, and
  anything else is a syntax error. SPARQL 1.2 §2.3.1: "The base direction is restricted to
  either `ltr` or `rtl`. Unlike a language tag, it is always lower case." The parser SILENTLY
  DROPPED an unrecognised `--` suffix — `"x"@en--foo` parsed as `"x"@en`, and `"x"@en--LTR` as
  `"x"@en--ltr` — where the W3C rdf-tests pin both spellings as negative syntax ("undefined base
  direction", "upper case LTR"), and the pre-release sweep had folded the comparison to
  `eq_ignore_ascii_case`, which the specification forbids. The language half is now held to the
  `LANG_DIR` production `[a-zA-Z]+ ('-' [a-zA-Z0-9]+)*` at the same site, so `@en-`, `@en--`,
  `@1en` and `@en--x--ltr` are refused too, while `@en`, `@en-US`, `@zh-Hant-TW`, `@x-klingon`,
  `@en--ltr`, `@ar--rtl`, `@en-US--rtl` and `@EN--ltr` (only the DIRECTION is case-restricted)
  all still parse. The vendored `lang-basedir/langdir-literal-invalid.rq` could not catch this:
  it spells its projection `AS v`, which is a syntax error on its own, so the harness refused it
  for the wrong reason.
- **core:** The `pack_query` dictionary benchmark measures again. Its literal-heavy fixture
  (added by the pre-release sweep) asserted a quoted triple term as a quad's SUBJECT, which
  RDF 1.2 does not admit and the freeze gate refuses (`rdf-ir-triple-subject`), so the fixture
  panicked and the report-only `benchmarks` CI job had been red on `main` since `b4f99093`. The
  triple term is now asserted in the object position (`s q <<s p plain>>`), the one place the
  model puts it, and the dictionary closure still resolves one triple term per row.
- **core:** `MutableDataset` no longer fabricates a literal datatype. When a base literal's
  datatype id resolved to something other than an IRI, `base_value_of` rendered that term's
  `Debug` form and used it as the datatype IRI, where `RdfDataset::term_value` states the same
  invariant with `unreachable!`. The fabricated value could not fail at the site; it would
  surface later as an `IriError` about a string nobody wrote. The path is unreachable from the
  public API (the builder interns every datatype through `intern_iri`, and the pack decoder
  refuses a non-IRI datatype entry before a pack becomes a dataset), so the two sites now agree
  on `unreachable!` with the invariant stated, and a test pins that `base_value_of` and
  `term_value` agree on every literal shape.
- **BREAKING** **sparql-eval,geo:** A host function registered through
  `UserFunctionRegistry::register_native` can now raise a SPARQL **expression error** for one
  solution instead of failing the whole query. `NativeFnBody` returns
  `Result<Option<TermValue>, EvalError>` rather than `Result<TermValue, EvalError>` — the same
  three-exit channel `register_expr`'s `ExprFnBody` already carried — so `Ok(None)` means "this
  call has no value for this solution" and `Err` stays reserved for a query-fatal condition. The
  evaluator, not the closure, then applies the outcome the calling context requires: a `FILTER`
  eliminates that solution (SPARQL 1.1 §17), while a `BIND` or `SELECT` expression leaves the
  variable unbound and continues (§10; algebra §18.5 `Extend`). Previously the seam had no
  `Ok(None)` exit at all, so every domain refusal — a malformed literal, an out-of-range index, a
  type mismatch — had to be spelled `Err` and aborted the query; one bad geometry anywhere in a
  dataset failed every query that scanned past it. This was found while merging `purrdf-geo` and
  affected every function on the seam, not just the `geof:` family. Callers with a
  `register_native` body must wrap their success value in `Some` and should move argument-level
  refusals from `Err` to `Ok(None)`.
- **BREAKING** **geo:** `functions::evaluate` returns `Result<Option<TermValue>, EvalError>` (the
  seam shape above) and maps `GeoError::Literal`/`GeoError::Domain` onto the per-solution
  `Ok(None)`; `GeoError::Unsupported`, `GeoError::Config` and the new `GeoError::Arity` stay
  query-fatal, because each holds for every solution alike and answering "no value" would empty a
  result set and present that as the answer. `GeoError` gains an `Arity` variant (a wrong argument
  count is nobody else's mistake — not the data's, not the wiring's) and a
  `GeoError::is_expression_error` predicate that is the single site deciding how far a refusal
  travels. A caller that needs the refusal itself — with its message and kind intact, which a
  SPARQL expression error by construction cannot carry — calls the new `functions::compute`,
  which answers in `Result<TermValue, GeoError>`.
- **shapes:** A SHACL validation report no longer FUSES a blank node it mints with one it
  carries. The report invents `_:report`, `_:r0`, `_:r1`, … and the interior nodes of a
  complex `sh:path`; blank-node labels arriving from the data or shapes graph are opaque
  strings that pass through the IR verbatim, so a data graph containing `_:r0` produced
  `_:r0 a sh:ValidationResult ; sh:focusNode _:r0` — the validation result and the node it
  reports on silently became ONE node, with the report asserting that a
  `sh:ValidationResult` was an instance of the data's own class. Nothing was dropped and no
  error was raised. The minted nodes now take a reserved label prefix whenever, and only
  whenever, the report actually carries a colliding label, so a report with no collision is
  byte-identical to before (the byte-frozen first-party corpus reports are unchanged).
- **entail:** OWL-Direct now DECIDES the `SHOIQ` nominal / inverse-role / qualified-number-restriction
  corner. Both decision cores implement the nominal-introduction rule — Horrocks & Sattler's `NN`-rule
  in the `cfg(test)` concept-tree reference and Motik–Shearer–Horrocks' Table 5 `NI`-rule in the
  production hypertableau — so an ontology with an at-most over an inverse role (an
  `owl:InverseFunctionalProperty`, or the vendored W3C spy-point `webont-description-logic-035`) is
  decided rather than answered `consistent-within-boundaries`; `webont-description-logic-035` now
  decides **inconsistent**, matching its published verdict, and is graded on every conformance run.
- **entail:** **Removed** the `counting-on-inverse` completeness boundary
  (`report::Construct::CountingOnInverse` and its `"counting-on-inverse"` certificate line, plus the
  `decided-within-boundaries` completeness it forced there). The corner is now decided, so a
  reasoning report over it no longer carries that boundary; a consumer that matched the
  `"counting-on-inverse"` boundary name or expected `decided-within-boundaries` there will instead
  see a plain `decided` verdict (or, past the counting ceiling, an honest `budget-exhausted`).
- **BREAKING** **sparql:** The `VERSION "1.2-basic"` declaration is now actually enforced as the
  SPARQL 1.2 Basic profile (full 1.2 syntax minus RDF 1.2 triple terms), not merely retained and
  round-tripped. Evaluation admission now refuses a triple term in any triple/quad pattern or path
  endpoint, a ground triple term in `VALUES`, and the "Functions on Triple Terms" builtins, naming
  the offending construct — for an update, with no mutation applied. Callers that declared
  `1.2-basic` and relied on it silently running the full engine must drop the declaration or
  remove those constructs from the request.
- **BREAKING** **sparql-eval:** An `UPDATE` prologue declaring an unrecognized `VERSION` (anything
  other than `"1.2"`/`"1.2-basic"`) now refuses at admission instead of mutating the store — the
  byte-identical read-only query form was already refused. Callers issuing such requests must
  stop, declare a recognized version, or omit the declaration.
- **BREAKING** **sparql-algebra:** `'*'` is refused for every aggregate but `COUNT` (it was
  previously accepted everywhere and silently answered the group's row count for `SUM(*)`,
  `AVG(*)`, `MIN(*)`, `MAX(*)`, `SAMPLE(*)`, and `GROUP_CONCAT(*)`). `AggregateExpression::new` is
  now the sole, checked constructor — it returns a `Result` and refuses `*`/an empty argument list
  for any aggregate but `COUNT` — and `args` is no longer a public field; callers that built the
  node with struct-literal syntax or read `args` directly must switch to `AggregateExpression::new`
  and the `args()`/`into_parts()` accessors.
- **BREAKING** **sparql-eval:** The aggregate accumulator trait gained a required `into_any`
  downcast (used to merge partial fold states by concrete type instead of recovering only the
  finished term) and its `combine` method now returns `Result<(), EvalError>` instead of `()`, so
  a host contract violation (a partial state of the wrong concrete type) is a typed refusal rather
  than a panic — which matters on `wasm32-unknown-unknown`, where a panic aborts the whole
  instance. Every implementor of `AggregateAccumulator`, including any host-registered
  `CustomAggregate`, must add `into_any` and update `combine`'s return type and propagate the
  downcast helper's error.
- **BREAKING** **sparql-eval:** A prepared plan's aggregate/property-function admission now keys
  on registry INSTANCE identity (a process-monotonic `RegistryId`), not declared metadata alone —
  two independently built registries that declare identically for a shared IRI but resolve it to
  different implementations previously produced the same plan fingerprint, so a plan prepared
  against one registry could execute unrefused against the other. A caller relying on two
  distinctly constructed registries with identical declarations being interchangeable at execution
  time now gets a typed refusal instead of a silently wrong answer; cloning a registry still
  shares its source's identity.
- **BREAKING** **sparql-eval:** Planner admission failures are now attributed to the extension
  seam that actually raised them. An unregistered custom aggregate or an aggregate arity violation
  previously reported the property-function diagnostic code; callers matching on the documented
  aggregate diagnostic code must now handle those failures there instead of under the
  property-function code.
- **BREAKING** **sparql-eval:** The within-group parallel aggregate fold now sizes its chunks from
  the group's row count alone instead of the live host's thread count, so governed outcomes for
  large aggregate folds no longer vary with worker-pool size. Governed outcomes for large
  within-group aggregate folds change on any host whose worker count differs from the new fixed
  reference parallelism; a consumer pinning `GOVERNOR_CORPUS_DIGEST` or `GOVERNOR_PROFILE_DIGEST`
  must re-pin both.
- **BREAKING** **xsd,sparql-eval:** Integer `SUM`/`AVG` now accumulate at arbitrary precision
  instead of a fixed-width accumulator, so a running total that used to overflow into an unbound
  answer now answers exactly (for example, a large positive, a one, and the matching large
  negative now sum to `1` instead of unbound). This changes only the answering direction — a query
  that previously received `unbound` for an overflowing integer `SUM`/`AVG` now receives a value —
  and needs no caller action beyond expecting an answer where one is now owed.
- **BREAKING** **shapes:** `Shapes` gains a public `aggregates: Arc<AggregateRegistry>` field,
  installed at every SHACL-SPARQL validation entry point (including the parallel focus-node chunk
  fork, which previously dropped the aggregate scope across the thread boundary, making a
  registered aggregate's resolution depend on the number of focus nodes). Callers constructing
  `Shapes` via struct literal must now supply `aggregates`, or use `Shapes::default()` / the
  parser's constructor, both of which populate it with an empty registry.
- **BREAKING** **cli,python,purrdf:** A dataset-derived property function combined with an
  entailment regime returned a SHORT answer, reported complete, at a success exit and with no
  diagnostic. The registry is built by the caller, before the call, and therefore before the
  closure exists — so a `--path-relation` walk (or a `path_relations` / `relations_from_graph`
  registration) read the SOURCE data while every other pattern in the same query read the
  closure. Over `ex:sub rdfs:subPropertyOf ex:p . ex:a ex:p ex:b . ex:b ex:sub ex:c .` under
  `rdfs`, `SELECT ?end WHERE { ex:a ex:p+ ?end }` answered `ex:b, ex:c` and the equivalent walk
  answered `ex:b` alone. Both entry points now materialize the closure FIRST and register the
  relations against it, so the two halves of one query read one dataset: `query_with_entailment`
  and `query_with_entailment_governed` take a new `relations: &ClosureRelations<'_>` argument
  (after `options`, before `governors`). Rust callers whose relations are dataset-independent —
  an in-memory table, an empty registry — pass `ClosureRelations::NONE` and keep their present
  behaviour byte for byte; a caller with a dataset-derived relation passes
  `ClosureRelations::rebuilt_by(&f)`, where `f` is handed the materialized closure and returns
  the registry to answer with. The CLI and Python surfaces are unchanged in shape and now supply
  the rebuilder themselves; the C ABI and WebAssembly surfaces register no relation and pass
  `NONE`. One pairing is refused rather than answered: a rebuilder combined with an OWL
  Direct-Semantics run whose restricted chase MINTED existential witnesses, because a walk over
  that closure could return a minted blank node as an observable binding and the regime's witness
  filtration cannot reach a property function's output. It carries the stable code
  `reasoning-closure-relation-witness` (exit 2 from the CLI, `ValueError` from Python) and names
  the regimes that accept the pairing; an `owl-direct` run that mints no witness is not refused.
- **BREAKING** **iri,rdf,cli:** A relative IRI reference with no base IRI in scope is now a hard
  error instead of being interned verbatim. Documents that previously "worked" this way were
  emitting N-Triples no conformant parser accepts, so the failure surfaces an existing defect
  rather than introducing one. Reference resolution is now a single layer, `purrdf-iri`, shared by
  every codec: the RFC 3986 §5.1 precedence chain is an in-document directive
  (`@base`/`BASE`/`xml:base`/`@context.@base`), else a caller-supplied base, else the document's
  retrieval IRI, else the §5.1.4 failure. The stable codes are `iri-relative-no-base` (fixable by
  supplying a base), `iri-not-absolute-by-grammar` (N-Triples, N-Quads, TriX and HexTuples admit no
  relative reference at all, so a base cannot help) and `iri-non-absolute-base` (the supplied base
  has no scheme). Three surfaces have a retrieval IRI, all of them ones that opened the file
  themselves: `purrdf-slice` derives it (the workspace's single RFC 8089 `file://` derivation),
  and `purrdf-shapes`' shape-union loader and `purrdf-cli` consume that derivation rather than
  repeating it — so a file input needs no flag on any of the three. Every surface handed BYTES —
  the `purrdf-rdf`/`purrdf-iri` library APIs, wasm, the C ABI, Python and CLI stdin — has no
  retrieval IRI and hard-fails as §5.1.4 specifies. Callers on
  those surfaces must give the document a base directive or pass one to the API. On the way out,
  a syntax that can express a base (Turtle, TriG, RDF/XML, JSON-LD, YAML-LD) now emits it and
  relativizes against a supplied base; one that cannot (N-Triples, N-Quads, TriX, HexTuples) keeps
  writing absolute IRIs. See "Base IRIs & Relative References" in The PurRDF Book.
- **BREAKING** **rdf:** `serialize_dataset_base_only` is **removed**, and `serialize_dataset_with`
  is added as the one serialization seam the rest of the family is now expressed through. The
  split family could not state a document base together with a graph selection or the RDF 1.2
  statement layer: `serialize_dataset` took the selection and the layer but no base, while
  `serialize_dataset_to_format` took a base but forced `SerializeGraph::Dataset` and the transcode
  projection — so asking for a base on RDF/XML silently traded away reifier and annotation rows
  the RDF/XML emitter can in fact render. `serialize_dataset_with(dataset, format, base_iri,
  &SerializeOptions { selection, statement_layer, jsonld_options })` states all four axes, and the
  new `StatementLayer` enum makes the third an explicit choice — `Emit` (render it, or fail closed
  where there is no surface for it), `Project` (drop it and REPORT the count), or
  `PerFormatCapability` (the registry's `carries_star()` decision, which is what every
  `*_to_format` spelling applies). `serialize_dataset`, `serialize_dataset_with_jsonld_options`,
  `serialize_dataset_to_format` and `serialize_dataset_to_format_with_jsonld_options` keep their
  signatures and behaviour and are one-expression delegations, so there is no second code path.
  Replace `serialize_dataset_base_only(d, media_type, selection)` with `serialize_dataset_with`
  under `StatementLayer::Project`, which additionally hands back the dropped-row count instead of
  leaving the caller to recompute it.
- **BREAKING** **capi:** The C ABI moves `0.6.0` → `0.7.0`, an **incompatible** bump.
  `purrdf_shacl_validate_to_sarif` and `purrdf_shacl_entail_to_ntriples` each gained a
  `shapes_base_iri` parameter **in the middle** of the existing list, between `shapes_ttl` and
  `data_nt` — a host compiled against `0.6.x` and run against `0.7.0` without recompiling passes
  `data_nt` into the `shapes_base_iri` slot and its `PurrdfBuffer **` out-pointer into `data_nt`,
  which the boundary then reads as a NUL-terminated C string. That silent, unguardable misread is
  the whole reason the version moved; the parameter is positional rather than appended because it
  belongs beside the document it qualifies. `purrdf_serialize_jsonld_configured` likewise gained
  `base_iri` after `media_type`, the slot it holds on `purrdf_serialize`. Both breaks ride the one
  bump because `0.7.0` is unreleased, so a consumer recompiles once rather than twice for one
  reason. Every C host must recompile against the new `purrdf.h`. Signature drift is now caught at
  test time: `crates/rdf-capi/tests/abi_signatures.rs` pins the complete exported prototype list
  against a committed snapshot and against the version triple, so a future incompatible change
  cannot reach a release without an author deliberately moving the version.
- **BREAKING** **shex,cli:** A shape map naming a shape label the schema does not declare — and
  `START` against a schema that declares no start shape — is now a hard refusal
  (`ShexError::UnknownShape`; CLI exit **1**) instead of a `"status":"nonconformant"` result at
  exit **0**.
  Scripts that parsed the JSON result and ignored the exit code will now see a failure where they
  previously saw a definite negative about the data; that is the point, since the old answer spent
  the format's one word for a finding about the DATA on a mistake the data had no part in — a typo
  in a shell's own argument read back as a validation verdict. Labels reachable through the import
  closure count as declared. The refusal happens before selector expansion, so a selector matching
  no node is refused identically to one matching many. The ShEx specification's ShapeMap status
  vocabulary has no value meaning "not evaluated", so this was resolved on the project's
  hard-fail doctrine rather than on anything the specification requires.
- **BREAKING** **cli:** `--base` is now refused by name when NEITHER leg of the operation can
  spend it, instead of being accepted and silently never read. A base is spent on parse (the
  source syntax admits a relative reference) or on serialize (the target syntax can write a base
  directive); `convert --from ntriples --to ntriples --base http://example.org/` satisfies
  neither and previously exited 0 having done nothing and said nothing. It is now a usage error
  (exit **2**) naming each leg and why it cannot take the value. A base ANY leg can spend is still
  honoured, so `--base X --to ntriples` continues to resolve the input. The same refusal covers a
  pack `--from`/`--to`, which carries no document base at all. Scripts passing `--base`
  unconditionally across format pairs must drop it on the pairs that cannot use it.
- **BREAKING** **rdf:** `GtsFoldView::new` and `GtsFoldView::with_config` now return
  `Result<Self, RdfDiagnostic>` instead of `Self`. They refuse a graph whose term table lets a
  term resolve through itself, with the code `gts-self-reaching-term`. The view's accessors —
  `nq_token`, `public_value` and everything built on them — walk a quoted triple's resolved
  components down to the leaves, so a self-reaching term recursed without bound and aborted the
  process; the view now refuses to EXIST rather than hand back an object whose every renderer is
  a process kill (the fold-time refusal GTS-SPEC §7.3 permits, applied once at construction
  instead of as a guard inside every walk). A graph read off the wire cannot contain one — the
  reader already refuses the row that would close the loop — so this reaches only callers who
  assemble a term table themselves. Rust callers must handle or propagate the `Result`; `?` is
  usually the whole change.
- **BREAKING** **python:** `GtsFoldViewNative.from_bytes` and `GtsFoldViewNative.from_parts` now
  raise `ValueError` carrying `gts-self-reaching-term`, for the reason above. `from_parts` is the
  reachable one: it is handed a caller-assembled term table, which `from_bytes`' reader validates
  on its own. Python callers constructing a fold view from parts must handle `ValueError`.
- **BREAKING** **rdf:** The byte-reproducibility classifier for `CONSTRUCT` dataset-description
  views now refuses a custom aggregate call, a custom scalar-function call, or any `SERVICE`
  clause (including `SERVICE SILENT`), matching the registry-dependency doctrine the
  property-function arm already applied — these depend on a registry or endpoint the classifier
  cannot inspect, so a view using them is not byte-reproducible. Callers who relied on such views
  being accepted must configure them without those constructs, or accept the resulting refusal.
  Built-in aggregates and built-in functions are unaffected.
- **rdf:** The byte-reproducibility refusal above, and the planner's two admission seams, now name
  the CONSTRUCT rejection's actual cause (a custom aggregate call, a custom scalar-function call,
  or a `SERVICE` clause) instead of funnelling every rejection through one message that named only
  the nondeterministic-builtin/blank-minting causes, which none of the three later-added causes
  appears in.
- **xsd,sparql-eval:** Integer `AVG`'s arbitrary-precision quotient is now taken on the same
  arbitrary-precision path its sum already uses, so an average that used to answer nothing whenever
  the exact running total escaped `i128` — even when the quotient itself was ordinary — now
  answers exactly (for example, `AVG` of two copies of the largest representable integer now
  answers that integer, rather than unbound).
- **sparql-eval:** `SERVICE` bodies forward custom scalar-function calls again unless `SILENT` is
  present. A prior fix closed a real silent-wrong-answer hazard for `Function::Custom` calls
  inside `SERVICE`, but the refusal it added was unconditional rather than scoped to the
  hazard it described — so a plain (non-`SILENT`) `SERVICE` body containing a call to the
  endpoint's own extension function (e.g. `FILTER(<http://example.org/fn>(?x) > 0)`) now
  hard-failed even though a non-silent `SERVICE` already turns any endpoint-side failure
  into an honest error. Callers who worked around the regression by dropping `SILENT` need
  no further change; callers who still need `SILENT` and hit the refusal should read the
  error message, which now names the same workaround (drop `SILENT`) directly.
- **sparql-eval:** The `SERVICE`-forwarding `LATERAL` guard is rebuilt to close a bypass and
  correct an over-broad refusal. The bypass: the guard's `Lateral` arm recursed into a
  variable-endpoint `SERVICE ?g { … }` auto-wrap's `left` operand only, never its wrapped
  `Service::inner` — so a written `LATERAL` nested inside `SERVICE ?g { … }` (itself nested
  inside a forwarded body) reached a remote endpoint's text unrefused. The guard now walks the
  full forwardable body, including every `Service::inner` (fixed-IRI or variable-endpoint), so a
  written `LATERAL` is found no matter how deeply it is nested. The over-broad refusal: a
  `LATERAL` clause inside a forwarded body was refused unconditionally, even though the hazard —
  a `SILENT` clause swallowing the endpoint's rejection into the join identity, a result that
  looks complete and is wrong — exists only under `SERVICE SILENT`. The refusal is now scoped to
  `SILENT`, naming it as the reason; a plain, non-silent `SERVICE` body forwards its `LATERAL {
  … }` text and surfaces the endpoint's actual verdict — an answer from a `LATERAL`-capable
  endpoint (`LATERAL` is Apache Jena's own extension) or an honest `EvalError::Remote` from one
  that rejects it. Callers who need `LATERAL` federated to a `LATERAL`-capable endpoint should
  drop `SILENT`; callers relying on the previous unconditional refusal under `SILENT` see no
  change, since `SILENT` still refuses.
- **sparql:** Let aggregate partial states merge by their concrete type (see the `into_any`
  entry above for the resulting trait break).
- **BREAKING** **sparql-eval:** The three extension registries (custom aggregates, property
  functions, SHACL-AF functions) move from `Option<&Registry>` to plain `&Registry`, with a
  canonical `Registry::EMPTY` constant replacing the `None` case. `QueryOptions`'s three registry
  fields and `PlanCache::prepare_with_relations`'s registry parameters change type accordingly
  across the core engine, all four host surfaces (CLI, C ABI, wasm, Python), the shapes crate, and
  the conformance harness; callers pass `&Registry::EMPTY` where they previously passed `None`.
  Decimal division's zero-divisor guard was corrected in the same change: it now returns the
  crate's typed error in release builds instead of a debug-only assertion that could panic.
- **BREAKING** **results:** The JSON/XML results writers now spell RDF 1.2 base direction as
  `its:dir` (were `dir`/`purrdf:dir` under a namespace this crate minted for itself), and the
  always-on purrdf-branded provenance extension is gone: `to_json`/`to_xml`/`serialize` take an
  `Option<&ProvenanceNamespace>`, and emission requires a caller-supplied prefix and IRI —
  omitting one emits no extension member and reports the drop, matching the CSV/TSV contract.
  `ProvenanceNamespace` separately moves to private fields and a fallible constructor,
  `ProvenanceNamespace::new(prefix, iri)`: the prior public fields let an unvalidated `prefix`
  splice unescaped into XML element/attribute names (a markup-injection hole), so `prefix` is now
  validated as an XML Namespaces `NCName` and `iri` as an absolute IRI. Callers constructing
  `ProvenanceNamespace` via struct literal must switch to the constructor and handle the `Result`.
  The TSV writer now hard-fails (`Error::Format`) on a variable name containing a tab or line
  break instead of silently corrupting the column/record structure.
- **BREAKING** **results:** The XML writer now declares the `its:` namespace once on the document
  root (with an `its:version` attribute) when any directional literal appears anywhere in the
  result set, instead of inline on every directional literal, matching the spec's worked examples;
  the JSON writer no longer emits an explicit `"datatype"` member on a plain (simple) literal, per
  the results-format's own encoding table. Both are byte-level output changes: a consumer pinning
  writer bytes must re-pin.
- **BREAKING** **sparql-results:** `ProvenanceNamespace` gained the validated, fallible
  constructor described above; unvalidated construction is no longer possible (see the results
  writer-spelling entry).
- **BREAKING** **rdf-core:** Replay delta-added quads in insertion order, not hash order. The
  copy-on-write delta layer kept its added/suppressed keys in standard hash sets (a fresh random
  seed per process), so freezing and pattern scanning walked them in per-process hash-iteration
  order rather than a reproducible one — the same query over the same data mutated the same way
  could return rows in a different order from one run to the next, which broke the documented
  ordering guarantee for `GROUP_CONCAT` and the first/last aggregates. Those sets now use the
  crate's fixed-key hasher and carry each key's insertion ordinal, so a delta replays in call
  order. The CLI was never affected (it parses straight into a builder and never uses the delta
  layer); the Python, WebAssembly, and C interfaces all wrap the delta layer directly and were all
  fixed by this one kernel change. Callers that happened to rely on the previous arbitrary order
  may observe a different, now-stable order.
- **BREAKING** **xsd:** `parse_duration` now enforces the `yearMonthDuration`/`dayTimeDuration`
  subtype pattern facets (XSD 1.1 Part 2 §3.4.26/§3.4.27) at parse time instead of accepting any
  lexical form its caller's declared tag claims: a `D`/`T` component under a `yearMonthDuration`
  tag, or a `Y`/`M` date component under a `dayTimeDuration` tag, is now a typed `InvalidLexical`
  rather than a silently accepted value that violated its own declared subtype. A caller lexical
  that relied on this laxity (e.g. `"P1D"^^xsd:yearMonthDuration`) now fails to parse instead of
  succeeding with a value its own datatype's pattern facet forbids.
- **BREAKING** **xsd:** A `Duration` whose months and seconds components carry opposing signs
  (e.g. `+12` months against `-1` day) is no longer constructible — XSD 1.1 Part 2 §3.3.6 puts
  mixed-sign values outside the lexical mapping's range, so there is no correct string to emit for
  one. The guard now sits in the single smart constructor every construction site — parsing and
  arithmetic alike — routes through, so it cannot be reached through one door (subtraction) while
  missed through another (addition of an already-negated operand). A caller combining a
  `yearMonthDuration` and a `dayTimeDuration` of opposing sign now gets a typed `OutOfRange`
  instead of a value that could never round-trip through its own canonical lexical form.
- **BREAKING** **xsd:** `duration =` (`op:duration-equal`) is now total, matching F&O: two
  durations with equal months and equal seconds compare equal regardless of declared subtype,
  where a cross-subtype comparison previously fell through to an incomparable/type-mismatched
  result — the reading `<`/`>` correctly keeps, since F&O defines ordering only for the
  `yearMonthDuration`/`dayTimeDuration` subtypes, not for the general type. A caller that treated
  duration equality as raising an error must now expect a plain `bool`.
- **xsd:** Two further `Duration` defects, both reachable only through extreme inputs, are closed
  alongside the arithmetic surface below: month accumulation during duration parsing used
  unchecked `i64` arithmetic (`"P9223372036854775807Y"` silently wrapped) and now reports a typed
  `OutOfRange`; and a zero-valued `yearMonthDuration`'s canonical lexical form is now the
  subtype-correct `"P0M"` rather than `"PT0S"`.
- **xsd:** An unreachable branch in exact decimal division — a scale-down path that could fire
  only if the crate-wide `scale <= 18` invariant were already broken, and whose own comment
  incorrectly called that "unusual but possible" — is now a build-surviving `unreachable!()` in
  place of a debug-only assertion, so a future violation of that invariant cannot ship a silent
  truncation in a release build. `xsd:duration ÷ xsd:duration` still reports a typed `OutOfRange`
  for the case that actually is reachable: scaling one operand's mantissa past `i128::MAX`.
- **BREAKING** **sparql-algebra,rdf:** The serializer rendered a join's right operand bare
  whenever it began with its own bare left, so a re-parse of the emitted text re-associated an
  `OPTIONAL`/`MINUS`/`FILTER`/`BIND`/`LATERAL` right operand into a semantically different tree —
  and this text is the `SERVICE` federation wire format, so a remote could receive a different
  query than the plan it was chosen for. Every re-absorbable right operand is now braced, decided
  by an exhaustive predicate; a plain join's right operand stays bare, since join associativity
  makes the re-association semantics-preserving. A variable-endpoint `SERVICE` under `LATERAL` is
  no longer double-wrapped in the keyword its own re-parse would re-wrap. The corpus round-trip
  sweep that caught this also surfaced and fixed five further serializer defects, none of them
  `LATERAL`-dependent: a multi-condition `HAVING` chain silently dropped its aggregate
  reconstruction, a projection-less aggregate emitted an illegal `SELECT *` over `GROUP BY`, a
  `FILTER` flattened as a left operand lost its group, property paths failed to parenthesize by
  precedence (alternation under sequence), and a trailing `VALUES` failed to absorb into its body
  group. A caller comparing serialized query bytes against a previous release should expect these
  five constructs, and the right-operand-braced ones above, to serialize differently — and
  correctly.
- **BREAKING** **sparql-eval:** Correlated evaluation (`LATERAL`, and `EXISTS`/`NOT EXISTS`
  correlated through an expression) substituted outer bindings by rewriting terms in place, which
  could not place a literal or blank-node binding into a triple position, silently skipped path
  and predicate positions, flipped `MINUS` into its disjoint-domain case by erasing the shared
  variables that make the two sides comparable, and crossed sub-select projection boundaries —
  correlating a variable the SEP-0006 scope example says is explicitly NOT correlated.
  Substitution now joins each pattern leaf with a one-row `VALUES` table carrying that leaf's own
  bindings, narrowed at every projection boundary to the variables it actually projects; an
  expression position keeps direct value substitution for an IRI or literal binding, and a
  blank-node or quoted-triple binding referenced ONLY in an expression position (no leaf
  occurrence for the leaf join above to carry it) is now carried the same way — by joining the
  expression's own owning pattern node against a one-row `VALUES` table — since no SPARQL
  expression syntax can spell either term kind as a rewritten constant; `BOUND` in particular now
  answers correctly for a bound variable of ANY term kind rather than only IRI/literal ones. This
  is a strictly corrective behavior change: `MINUS` inside a correlated `LATERAL`/`EXISTS` now
  answers correctly instead of a domain-flipped wrong answer, a `LATERAL`-bearing pattern is now
  refused rather than silently forwarded to a `SERVICE` endpoint (a remote rejecting the extension
  under `SILENT` would otherwise have contributed the identity table as a silent wrong answer),
  and a blank-node or quoted-triple outer binding reachable ONLY through an expression inside a
  `LATERAL` right-hand side no longer silently drops the row.
- **BREAKING** **sparql-algebra:** SEP-0007 Part 3's assignment restriction is now enforced at
  parse time: a `BIND`/a sub-`SELECT`'s `(expr AS ?v)` projection target/a `GROUP BY (expr AS ?v)`
  grouping target, or a `VALUES` column, inside an `EXISTS`/`NOT EXISTS` body — both polarities
  share the same grammar production, so both are covered identically, including a nested
  `EXISTS`'s own body checked against its immediately enclosing `EXISTS`'s scope — that rebinds a
  variable already in scope on the row being filtered is now a typed `ParseError` naming the
  variable and the introducing construct, instead of being silently accepted and evaluated. A
  rebinding confined to a `MINUS` right operand inside the body stays legal at any depth, since
  such an introduction never escapes it to become observable. A caller whose query relied on the
  previously accepted (and ambiguous) rebinding must rename the colliding variable.
- **BREAKING** **sparql-eval:** Three `EXISTS`/`NOT EXISTS` answer defects are corrected. A
  `GRAPH ?g { … }` body correlated through the row being filtered left the graph name unresolved
  against that row — the substitution walk skipped it because only one of its two callers ran the
  compatibility merge the name needed — so an existence filter could accept rows bound to a graph
  that does not actually hold the pattern; the name now resolves to the row's bound IRI for
  indexed selection. A correlation reaching a nested `EXISTS`'s body only through a triple position
  (never that inner's own expression positions) went undetected by the variable walk feeding the
  correlation decision, so that inner ran unconstrained instead of per-row; the walk now agrees
  with the one substitution itself uses. And a bare `OPTIONAL` at the top of a correlated body was
  evaluated as though its right side and join condition mattered to the existence test, when
  `OPTIONAL` pads every left row unconditionally and never removes one — `FILTER EXISTS { OPTIONAL
  { P } }` is always `true` (and its `NOT EXISTS` twin always `false`) regardless of whether `P`
  matches, which Existential Normal Form's spine-top `LeftJoin` erasure now decides directly. A
  query that depended on any of the three previous wrong answers now gets the correct one.

### Features
- **shapes:** New `ValidationReport::to_dataset()` returns the report graph as a frozen
  `Arc<RdfDataset>` — the report's PRIMAL RDF form. Rendering a report in any syntax other
  than N-Triples previously forced a `to_ntriples()` → `parse_dataset()` round-trip; that
  parse was pure waste, because the report was already being materialized as IR quads and
  the N-Triples text was only ever a serialization of them. `to_ntriples()` is now defined
  as "serialize `to_dataset()`", so the two are the same graph by construction rather than
  by coincidence, and the direct path carries every RDF 1.2 term the report holds (a
  triple-term focus node or `sh:value`, and blank-node labels the text grammar can only
  carry escaped) instead of whatever survives a text grammar and a parser's relabelling.
  Equivalence is proven canonically (RDFC-1.0) over a report spanning all four severity
  kinds, IRI/blank/triple-term focus nodes, a complex `sh:path` shared by two results, and
  typed/language-tagged/blank/triple-term values. Rust surface only: no Python, JS/wasm, or
  C ABI equivalent is added.
- **cli:** `purrdf validate --format <rdf-syntax>` no longer serializes the report to
  N-Triples and re-parses that text; it hands the report's own dataset to the shared sink.
  Identical output, one fewer full parse of the report per invocation.
- **cdt:** New crate `purrdf-cdt` implementing SEP-0009 SPARQL Composite Datatypes
  (`cdt:List` / `cdt:Map`) as a closed leaf over `purrdf-iri` + `purrdf-xsd` only. The
  function library is the spec's fifteen — `cdt:List`, `concat`, `contains`, `get`, `head`,
  `tail`, `reverse`, `size`, `subseq`, `cdt:Map`, `containsKey`, `keys`, `merge`, `put`,
  `remove` — and the set is CLOSED: there is no registry to configure and no way for a
  caller to shadow a spec function, so the same query means the same thing on every host.
  Nesting is fully ITERATIVE under three bounds (depth 64, 2²⁰ elements, 64 MiB); a
  million-deep input exits cleanly rather than overflowing the stack, which would abort the
  process rather than raise. PurRDF mints no vocabulary here: the SEP-0009 namespace is the
  spec's own fixed string, recognized and never invented.
- **sparql:** Evaluate SEP-0009 end to end. The fifteen functions are recognized at parse
  time (with arity enforced there) and dispatched by the evaluator; `FOLD` rides the
  aggregate ACCUMULATOR seam as a keyword aggregate carrying its own `ORDER BY`; `UNFOLD` is
  its own `GraphPatternNotTriples` alternative over an expression — structurally where
  `LATERAL` sits — rather than a property function, because the property-function registry
  keys on a predicate IRI and no `UNFOLD` predicate IRI exists to key on. `ORDER BY`, `MIN`
  and `MAX` order composite literals by the value they denote. Blank-node labels written
  inside a composite lexical form bind through the SAME ingress rule as bare `_:` tokens, on
  every codec path, and participate in canonicalization and skolemization — so `_:b` written
  as a subject and `_:b` written inside a `cdt:List` in the same document are one node.
- **BREAKING** **sparql-algebra:** `GraphPattern` gains an `Unfold` variant and
  `Function` gains a `Cdt` variant. Both enums are matched exhaustively across the
  workspace; a downstream consumer matching on either without a wildcard arm must add the
  new arm. There is no Cargo feature to opt out — CDT is unconditional, like every other
  part of the engine.
- **conformance:** The vendored SEP-0009 suite (`vectors/sparql-cdt/`, `awslabs/SPARQL-CDTs`
  at commit `e0a7465`, 658 cases across six groups) is run and reports its own scoreboard
  row: **658 / 658, 0 ledgered**. Note the divergence that number cannot express, stated in
  full in [`docs/CONFORMANCE.md`](docs/CONFORMANCE.md): PurRDF's reader accepts two element
  forms the published lexical space does not — an RDF 1.2 triple term and a directional
  language-tagged literal — because refusing an RDF 1.2 term type is not an admissible
  outcome for this toolkit. **A conformant SEP-0009 reader handed one of those literals will
  call it ill-formed.** The mitigation is executed rather than argued: a scan grades every
  composite literal in every corpus this workspace ships and proves not one of them needs
  either form, with its counts pinned as equalities so it cannot pass vacuously.
- **BREAKING** **sparql-eval:** Generalize the `SERVICE` seam into a per-service-context
  `ServiceResolver`. The trait `RemoteQuerySource` is renamed `ServiceResolver` and its method
  is renamed `resolve`, taking the whole request as one `ServiceRequest` value (endpoint,
  forwarded query text, the `SILENT` flag, stop signal, intermediate-cell ceiling) instead of
  four positional arguments; `LocalRemoteQuerySource` is renamed `InProcessServiceResolver` and
  moves to the new `purrdf_sparql_eval::service` module (re-exported at the crate root).
  Implementors must rename the trait and method and destructure `ServiceRequest`; callers must
  rename the two types. `HttpRequest` gains a `headers` field carrying the per-service headers
  and credential — a transport built with struct-literal syntax must add it, and a transport
  that receives but ignores it will issue an *unauthenticated* request for a service configured
  as credentialed, whose rejection `SERVICE SILENT` is entitled to swallow. `HttpTransport`
  itself is unchanged, and a source with no catalog sends the same bytes it always did.
- **sparql-eval:** Add per-service policy for `SERVICE` federation: a `ServiceCatalog` maps a
  service IRI to a `ServiceProfile` carrying extra headers, a redacting `ServiceCredential`
  (bearer / RFC 7617 basic / arbitrary header), timeout and User-Agent overrides, and an
  explicit `ServiceCapabilities` grant set (`Query`, `Network`, `Credentials`). Context lives on
  the resolver keyed by endpoint, never in the service IRI — which would put credentials into
  the query text, plans, and receipts. Catalogs deny by default and are opt-in: a resolver with
  no catalog behaves exactly as before, and gating a service adds no header its profile does not
  carry. Withholding `Network` makes an in-process façade provable rather than promised —
  `InProcessServiceResolver` holds a dataset map and no transport of any kind — and the new
  `ServiceRouter` composes in-process and network resolvers with the routing table, not the
  query text, deciding which answers what. The catalog is consulted on every resolution,
  including one nested inside a forwarded body. The `SILENT` contract is now stated in full:
  `SILENT` swallows an unreachable or undecodable endpoint to the join identity, and never
  swallows a capability denial or a governor trip, both of which are decisions taken on this
  side of the seam. There is deliberately no knob softening that — a host wanting a blocked
  service to read as unreachable returns a transport error from its own resolver. The
  denial holds at every nesting depth: `EvalError` gains a structured `ServiceDenied`
  variant (the enum is `#[non_exhaustive]`, so this is additive) so that a denial raised by
  a `SERVICE` nested inside a forwarded body is re-raised as a denial rather than decaying
  into a silenceable endpoint failure that an enclosing `SERVICE SILENT` would swallow to
  the join identity. A credential is also validated when it is attached rather than when it
  is rendered — CR/LF/NUL in a bearer token or an arbitrary credential header is refused at
  configuration time, with the credential withheld from the message, while a Basic password
  may still contain any byte because it is base64-encoded before it reaches the wire.
- **sparql-eval:** Add path-WITNESS property functions, which answer the derivation question the
  core grammar's property paths cannot: `?s ex:p+ ?o` reports that some route exists and binds only
  the endpoint pair, while a call to `?start <caller-iri> ( ?end ?pathId ?len ?step ?node ?edge )`
  binds the route itself — one row per hop, with `?edge` the traversed STATEMENT as a first-class
  RDF 1.2 term that joins straight back into the dataset by an ordinary basic graph pattern.
  `GROUP BY ?pathId` with `ORDER BY ?step` reassembles a whole walk inside the query language, so a
  caller can weight, filter, or re-join a route without host code and without a list term. Two
  relations, not one with a mode flag, because the planner reads cardinality off the registration:
  `PathWitnessRelation` enumerates every simple-prefix walk (exponential in the worst case) and
  `ShortestPathWitnessRelation` yields one shortest witness per reachable pair (polynomial).
  Enumeration terminates structurally on cyclic input, and its endpoint projection equals `p+`.
- **cli:** Add `purrdf query --path-relation` and `purrdf update --path-relation`, the binary's
  first property-function registration surface — before it, `QueryOptions::property_functions`
  stayed empty on every call. Repeatable; the value is semicolon-separated `key=value` pairs
  (`iri`, repeatable `forward`/`inverse`, `min-hops`, `max-hops`, `max-paths-per-seed`,
  `max-expansions`, `mode=walk|shortest`). Every key is mandatory and none has a default: PurRDF
  mints no vocabulary IRIs, so the relation IRI is caller-supplied with no default namespace, and
  a traversal envelope the binary invented would be a limit the operator never read. Each
  malformed spelling names the offending token. The flag reaches the ungoverned, governed,
  `--explain`, and `--entailment` lanes of `query` and the `WHERE` clause of `update`; the
  `--explain` receipt's `relations` block now names what was registered.
- **python:** Add the `path_relations` keyword beside `relations` / `relations_from_graph` on
  every `Store` and `MutableDataset` query and update entry, registering a path-witness relation
  over the store's own edges as `{iri: (steps, min_hops, max_hops, max_paths_per_seed,
  max_expansions_per_invocation, mode)}`. It crosses the boundary as pure data — a specification
  of which directed predicates a hop may follow, never a Python callable — so the evaluation still
  runs with the GIL released. Every envelope field is mandatory; an unknown direction or mode
  string, an empty or duplicated step alternation, a non-IRI predicate, and an unbuildable
  envelope each raise `ValueError` carrying the engine's own diagnostic.
- **xsd:** Implement the XPath Functions & Operators section 9 temporal operation table:
  timezone adjustment for `dateTime`/`date`/`time`, `yearMonthDuration`/`dayTimeDuration`
  arithmetic with month-end clamping, instant subtraction, and duration add/subtract/
  multiply/divide computed in exact `Decimal`. The existing partial order is unchanged; parsing
  and one canonical form are, per the `xsd` Bug Fixes entries above — subtype pattern facets are
  now enforced at parse time, and a zero-valued `yearMonthDuration` now canonicalizes to `"P0M"`
  instead of `"PT0S"`.
- **sparql:** Add the `ADJUST` builtin over `dateTime`, `date`, and `time` (SEP-0002's
  two-argument signature over the new F&O adjust-to-timezone family): shifts a timezoned value,
  annotates an untimezoned one, and treats the empty simple literal as timezone removal.
- **BREAKING** **sparql:** Retain the `VERSION` declaration as a typed value. `Query` and
  `Update` gain a `version` field (`SparqlVersion`) on every variant, exposed via a new
  `version()` accessor beside `dataset()`/`base_iri()`; construction sites using struct-literal
  syntax without struct-update (`..`) must add the field. Evaluation admits only `"1.2"` and
  `"1.2-basic"`; anything else is refused at the evaluation chokepoint before any work is spent,
  naming the declared string, while parsing itself stays syntax-only.
- **BREAKING** **sparql:** Re-found the aggregate algebra on the specification's own shape:
  `AggregateExpression` is a struct (`function`, `args`, `scalarvals`, `distinct`) rather than a
  lossy simplification, `CountStar` is gone (`COUNT(*)` is the empty argument list), and
  `GroupConcat` carries no separator payload (the separator is the `"separator"` scalarval). See
  the matching `sparql-algebra` bug-fix entry above for the resulting constructor break.
- **sparql:** Evaluate custom aggregates through a fold-algebra registry: a caller registers an
  `init`/`step`/`combine`/`finish` accumulator under an IRI, reached from query text as
  `AGG(<iri>, [DISTINCT] arg, arg, …)`; an unregistered IRI or wrong-arity call is refused at
  prepare time, before any budget is spent, and the prepared plan carries the registry's
  fingerprint (see the registry-instance-identity bug-fix entry above).
- **BREAKING** **sparql:** Price the aggregate fold in the governor profile. Profile v6 adds two
  charge points — `aggregate-invocation` (once per group per aggregate expression) and
  `aggregate-accumulation` (once per value inspected) — shared by built-in and custom aggregates.
  `GOVERNOR_PROFILE_VERSION` is 6 and the profile/corpus digests a consumer pins have moved.
- **BREAKING** **sparql-eval,xsd:** Charge the aggregate fold's retained per-group row buffer to
  the scratch-bytes governor as it is buffered, on both the built-in and registered-aggregate
  paths; that real, group-size-proportional memory was previously unpriced by any resource
  dimension. Only the scratch-bytes figures move — fuel is byte-identical, so
  `GOVERNOR_PROFILE_VERSION` and its digest correctly stay put — but a governed query near a
  scratch-bytes ceiling may now be refused where it previously completed, and a consumer pinning
  the frozen governor corpus's byte-frozen expectations must re-pin against the new figures.
- **sparql:** Fold single large groups in parallel: rows fold in chunks whose partial states
  combine strictly in chunk order, byte-identical to the sequential fold for every algebraic
  class. `GROUP_CONCAT`'s row order is now pinned by exact strings (see the SPARQL querying book
  chapter's "GROUP_CONCAT ordering" section for the guaranteed reading), and a blank node or
  triple term in its input now poisons the fold to unbound — the same reading `SUM`/`AVG` already
  use for a non-numeric running total — where it was previously silently dropped. This is a
  behavior change to `GROUP_CONCAT` over non-literal input, not an API break: a query that used to
  get a partial concatenation silently omitting a blank-node/triple-term value now gets unbound.
- **sparql:** Ship a statistical aggregate set under caller configuration: ten exact statistical
  aggregates (`MEDIAN`, `PERCENTILE`, `STDDEV`, `STDDEV_POP`, `VARIANCE`, `VAR_POP`, `MODE`,
  `FIRST`, `LAST`, `TOPK`) register as fold instances under a namespace the caller supplies (no
  default), reached one call away via `AggregateRegistry::register_statistical_aggregates`.
- **BREAKING** **sparql:** Thread an aggregate namespace through every host surface — a keyword
  argument on the Python query/update entry points, a flag on the CLI query/update subcommands, a
  parameter on the governed wasm entry points, and a nullable string on the governed C ABI entry
  points — so the statistical aggregate set is reachable from every binding, not only the embedded
  Rust engine. The C ABI's version moves `0.3.0` -> `0.4.0` to reflect the additive surface, and
  its header is regenerated; at this point in the series, the entailment and explain CLI lanes,
  and the ungoverned/entailment wasm calls, refuse the parameter outright since they structurally
  cannot honour it — closed by the entry below, which carries the C ABI to `0.6.0`.
- **BREAKING** **sparql-eval,purrdf:** The entailment-aware query lanes
  (`purrdf::query_with_entailment`/`query_with_entailment_governed`) hardcoded empty query
  options, so no scalar-function, property-function, or aggregate registry could reach a query
  evaluated over an entailed closure — `AGG(<{NS}MEDIAN>, …)` over an inferred closure was simply
  refused. Both lanes now take a `QueryOptions` and thread it through closure parsing, the
  witness-restriction rewrite, and evaluation. `QueryExplanation` gains an `aggregates()` accessor
  and a rendered block naming each resolved aggregate with its arity, volatility, algebraic class,
  state bound, and scalar parameters, mirroring how resolved relations are already reported. The
  CLI's prior refusal of `--aggregate-namespace` beside `--entailment` is removed. This closes the
  gap at the Rust engine layer; the C ABI, wasm, and Python bindings did not yet expose the
  parameter on their entailment-aware entries until the entry below.
- **sparql,capi,wasm:** Accept the aggregate namespace on the entailment-aware governed query lane
  everywhere it previously took no `QueryOptions` seam at all: the C interface, the WebAssembly
  binding, and the Python binding gain the same namespace parameter their ordinary governed
  entries already had, so `AGG(<{NS}MEDIAN>, ?x)` resolves over an entailed closure exactly as it
  resolves over a raw view. The C ABI's version moves `0.4.0` -> `0.6.0` (`0.5.0` is skipped —
  one commit carries both the host-surface change and the version bump) and its header is
  regenerated.
- **BREAKING** **capi:** `purrdf_serialize` gains `out_directional_literals_dropped` and
  `out_named_graph_rows_dropped` beside the existing `out_statement_rows_dropped`, so the whole
  realized loss of one serialization is partitioned by cause and no row is charged twice: a
  star-capable single-graph target (Turtle, N-Triples) previously reported `0` while silently
  discarding every named graph it was handed. Each count stays independently nullable. This
  transcode lane flattens and counts rather than refusing the way the query lane does, so a
  dataset whose every row is graph-scoped serializes to a single-graph syntax as a well-formed
  EMPTY document with status `OK` and the whole loss in `out_named_graph_rows_dropped` — pinned
  from C in the smoke test. The C ABI's version moves `0.6.0` -> `0.7.0` and its header is
  regenerated; the minor number tracks the exported signatures, so an additive parameter bumps it
  too, and every C caller must widen the call.
- **BREAKING** **sparql-eval,cli:** Give the engine one options-carrying explain entry,
  `NativeSparqlEngine::explain_query_with_options`/`_view`, that carries the SHACL-AF function,
  property-function, and aggregate registries together, and remove the narrower single-registry
  explain and query entries (`explain_query_with_aggregates`, `explain_query_with_property_functions`,
  `query_with_property_functions`, `query_with_user_functions`, and their `_view` twins). A query
  needing a registered relation and a registered aggregate together previously got a receipt (or
  an answer) for a different, narrower query through a narrower entry — a silent wrong report
  rather than a refusal — because an entry that could not be handed every registry in scope has no
  registry-independent way to know a call it did not recognize was ever meant to resolve against
  one it was not given. A caller of any removed entry now calls
  `explain_query_with_options`/`_view` or `query_with_options_view` with the same registry set on
  `QueryOptions`; the CLI's `--explain`, its ordinary query/update lanes, and every other in-tree
  caller already went through the options-carrying entries and need no change. `--aggregate-namespace`
  with `--entailment` or `--explain` is no longer refused (see the two entries above); `--explain`
  with `--aggregate-namespace` now prints the plan with the registered aggregates named in the
  receipt's `aggregates` block.
- **sparql-conformance:** Respell the conformance harness's manifest-extension predicate namespace
  under `example.org`; it was minted under this suite's own project-branded namespace, the same
  mistake a prior change fixed for the SPARQL-XML results writer. A new hygiene test
  (`no_purrdf_dev_iri_is_minted_outside_the_closed_exemption_list`) fails on any occurrence of
  that branded namespace found in the tracked tree outside a closed, stale-checked allowlist of
  two reader-tolerance fixtures. A sibling manifest fixture proves the guard's negative case is
  triggered by the missing description specifically, not by some other structural defect.
- **BREAKING** **sparql:** Accept named scalar-value parameters on aggregate calls: `AGG(<iri>,
  …)` now admits trailing `; NAME=value` clauses, generalizing `GROUP_CONCAT`'s own
  `; SEPARATOR="…"` clause to any custom aggregate (e.g. `AGG(<{NS}PERCENTILE>, ?x; P=0.95)`).
  `CustomAggregate::init` now takes the call site's resolved named scalar-value parameters
  (`&[(String, TermValue)]`) alongside `self`; every implementor of the trait must add the
  parameter (an aggregate declaring no scalar parameters via the new default
  `CustomAggregate::scalarvals` may ignore it). The first-party `PERCENTILE` and `TOPK`
  aggregates now take their fraction/bound through the named clause instead of as trailing
  positional arguments; callers passing them positionally must move them to named form.
- **sparql-algebra:** `parse_literal` — the grammar behind an `AGG(<iri>, …; NAME=value)`
  scalarval, a `VALUES` ground term, and a triple pattern's literal object — now accepts the
  full SPARQL literal grammar it always claimed to: the signed halves of the numeric tower
  (`Q=-1`, `P=+0.5`) and the boolean literals (`B=true`), not only unsigned numerals and
  strings. `VALUES ?x { -1 true }` and `?s :p -3` parse for the same reason: both routed
  through the same unsigned-only `parse_literal`, so the gap was one production, not one call
  site.
- **cli:** `purrdf query --explain` refuses `--results-format`, `--loss-ledger`, and
  `--jsonld-options` by name instead of accepting and silently ignoring them — `--explain`
  returns before any serializer or loss-ledger surface runs, so a named `--results-format`
  never selected a serialization, a named `--loss-ledger` never had a transcode to report, and
  a configured `--jsonld-options` document never reached a serializer. `--results-format` is
  now `Option<QueryFormat>` (defaulting to `json` only when the flag is genuinely absent) so
  "not named" and "named `json`" are distinguishable, which is what the refusal needs.
- **cli:** `purrdf convert --canonical` refuses `--to` by name instead of silently overriding
  it — canonical output is always RDFC-1.0 N-Quads, so a `--to` naming a different target
  format was accepted and never read. `--to` may still be omitted under `--canonical`, exactly
  as before.
- **BREAKING** **xsd:** Add a value-space arithmetic operator surface — `value_add`/`value_sub`/
  `value_mul`/`value_div`/`value_unary_minus` — dispatching, in one call, over the full numeric
  tower plus `xsd:dateTime`/`xsd:date`/`xsd:time`/`xsd:duration` (and its two subtypes) and the
  five Gregorian partial-date types (`xsd:gYearMonth`/`xsd:gYear`/`xsd:gMonth`/`xsd:gMonthDay`/
  `xsd:gDay`) ± duration arithmetic, covering SEP-0002's full temporal operator table plus
  Gregorian ± duration, which has no SEP-0002 row of its own but matches the permissive reading
  of another SPARQL engine's own duration handling everywhere the answer does not depend on a
  fabricated calendar field — for `xsd:gMonthDay` specifically, "does not depend on a fabricated
  field" is decided by anchoring the complete months-then-days computation at every year in one
  full 400-year Gregorian period (the calendar's leap rule is exactly periodic at that length) and
  requiring every anchor to agree on the finished `(month, day)`, not on either component alone
  (the book's SPARQL querying chapter carries the full table and that divergence's record).
  `numeric_add`/`numeric_sub`/`numeric_mul`/`numeric_div` remain public but
  are now documented as the narrower numeric-tier operators the value-space surface delegates to
  for numeric operands, not as the SPARQL-facing entry point — a caller that depended on them
  accepting a temporal operand and returning a type error there should switch to the `value_*`
  surface, which accepts it and returns a value.
- **BREAKING** **sparql-eval:** `+`, `-`, `*`, `/`, and unary `-` in `FILTER`/`BIND` now evaluate
  over `xsd:dateTime`/`xsd:date`/`xsd:time`/`xsd:duration` (and its subtypes) and the five
  Gregorian partial-date types through the new value-space operator surface, where they previously
  fell through a numeric-only dispatch and folded every such operand pair to unbound. A query that
  relied on a temporal arithmetic expression silently answering unbound now receives the computed
  value instead; the result's datatype tag follows the operands' own declared tags (documented in
  the book's SPARQL querying chapter), never the computed component values.
- **sparql-eval:** `SUM`/`AVG` accept a group of `xsd:duration` values (any subtype) alongside
  their existing numeric acceptance, summing componentwise in the duration group and rounding
  `AVG`'s months component to the nearest whole month (ties toward positive infinity) — a PurRDF
  extension, since SPARQL 1.1 §18.5.1.3 defines `SUM` over the numeric tower only. A group mixing
  numeric and duration values still folds to unbound; this widens acceptance, it does not narrow
  the existing numeric one.
- **xsd:** Add unary minus on `xsd:duration` — `-(?duration)` negates both components together,
  so it can never produce the mixed-sign value the type cannot represent — a PurRDF extension,
  since F&O's unary minus (§4.2.8) is numeric-only and defines no duration form. Unary plus
  deliberately stays numeric-only, so `+(?duration)` remains a type error while `-(?duration)`
  is not.
- **xsd:** `xsd:duration ÷ xsd:duration` now also accepts the general `xsd:duration` type on
  either or both sides, not only the matching-subtype pairs F&O defines
  (`op:divide-yearMonthDuration-by-yearMonthDuration`,
  `op:divide-dayTimeDuration-by-dayTimeDuration`), dispatching on the operands' VALUE
  commensurability — both purely months, or both purely seconds — rather than their declared
  tags, so a `dayTimeDuration` and a general `xsd:duration` that happens to be purely day-shaped
  still divide. Two values whose components are not commensurable, even under matching declared
  tags, are a typed error rather than an arbitrary answer.
- **sparql-algebra:** Add `LATERAL { GroupGraphPattern }` surface syntax (SEP-0006, implemented in
  Apache Jena 4.7.0 — the SPARQL 1.2 Query specification's own text defines no `LATERAL`
  production). The parser enforces the SEP's scope restriction: no variable introduced by a
  `BIND`, a sub-`SELECT` projection expression, a `GROUP BY` aggregate output, or `VALUES` at the
  right-hand side's own scope level may collide with a variable already visible on the left;
  correlated USE of a left-hand variable, and a sub-`SELECT`'s legitimate shadowing, both stay
  legal. `LATERAL` is also now legal (and scope-checked) inside `INSERT`/`DELETE … WHERE` and
  `WITH … WHERE`, while a `DELETE WHERE` quad template refuses it by name instead of misparsing it
  as a subject term.
- **BREAKING** **sparql-eval:** `EXISTS`/`NOT EXISTS` now rests on one stated semantics — SEP-0007's
  `Replace`/`PrjMap` substitution (`exists(X, μ) ⟺ eval(D(G), Replace(PrjMap(X), μ))` is non-empty)
  — served by exactly two implementations chosen through a proven boundary instead of two
  heuristics with a guessed one: a memoized existence probe (evaluate the inner once, index it,
  existence-probe each row) where a prepare-time admissibility proof shows it equivalent to
  per-row substitution for every row the site can see, and the per-row definition itself (backed
  by a restriction-keyed memo and a first-witness `Slice{0, Some(1)}` stop) everywhere else.
  `--explain`'s per-algebra-node charge ledger now reports the chosen strategy and its cost
  through three new evidence counters, `exists-probe-answered`, `exists-definition-answered`, and
  `exists-inner-solutions-consumed`. `GOVERNOR_PROFILE_VERSION` moves `6` -> `7` to carry the
  three new charge points, and the frozen governor corpus is regenerated; a consumer pinning
  `GOVERNOR_PROFILE_VERSION`, `GOVERNOR_PROFILE_DIGEST`, or `GOVERNOR_CORPUS_DIGEST` must re-pin
  all three. A query without a correlated `EXISTS`/`NOT EXISTS` charges none of the three new
  points, so its fuel is unchanged in value even though the schedule and its digest moved.

### Performance
- **core:** The interner no longer mints a `String` for the datatype IRI of every untyped or
  language-tagged literal; `rdf:reifies` is interned once per builder rather than by string on
  every reifier push. RDFC-1.0 canonicalization renders blank labels and predicate IRIs by
  borrow inside the n-degree search, and issues ids with one tree descent instead of three.
  `MutableDataset::freeze` memoizes each base term's builder id (dense table, one slot per
  base id) instead of rebuilding an owned value for every one of a quad's four term
  occurrences; `quads_for_pattern` on the mutable view no longer materializes a base-sized key
  vector. No output, ordering or hashing changes.
- **iri, xsd:** RFC 3986 dot-segment removal walks a borrowed cursor (zero allocations per
  resolve, was O(segments) buffer rewrites); `XsdDatatype::from_iri` compares the namespace once;
  the `whiteSpace` facets copy clean runs in bulk; `Decimal::canonical_lexical` builds its
  output in one buffer. Each rewrite is pinned byte-for-byte against its previous
  implementation by a test.
- **sparql-eval:** `OPTIONAL { … FILTER }` is hash-indexed like the unfiltered join whenever the
  right side is fully bound on the shared columns (the same candidate rows in the same order,
  so results, padding and governor charges are unchanged); a `MINUS` whose arms share no
  variable returns the left bag by move instead of scanning every pair; `EXISTS` no longer
  rebuilds a bound-variable set per outer row; `LANGMATCHES` compares bytes without lowercasing
  three strings per row; `^path` shares its inner memo instead of deep-cloning the reach set;
  `CONSTRUCT` reuses its per-row blank-label map.
- **sparql-algebra:** The GROUP BY / HAVING / ORDER BY continuation checks and language-direction
  split no longer allocate a case-folded copy per token; ASCII lookaheads in the lexer compare
  bytes instead of decoding a `char`.
- **shapes:** `sh:nodeKind`, `sh:minLength`, `sh:maxLength` and `sh:languageIn` read the value
  node's kind, length and language from the dataset arena and materialize an owned term only
  for a violation; `sh:closed` probes borrowed predicate names.
- **rdf:** Quoted-triple terms resolve their reifier binding through a one-per-document index
  (the table scan made star-heavy documents quadratic in their statement layer, including inside
  the canonical sort's comparators); the serializer memoizes term ids so a repeated predicate
  never rebuilds its value; the RDF/XML and TriX escapers make a single pass and borrow
  untriggered input; JSON-LD expansion borrows the parent context when an object declares no
  `@context`. Every emitted byte is unchanged.
- **gts, sparql-results:** The canonical GTS writer caches its CBOR sort keys once per row
  instead of re-encoding per comparison and computes the MMR root from peaks alone (no boxed
  tree); the SRJ/SRX escapers and the SRJ string reader copy plain runs in bulk. Frozen vectors
  are byte-identical.
- **sparql-algebra:** Parsing no longer re-walks the whole accumulated pattern for every
  `BIND`/`SELECT *`/`LATERAL` scope check. The group-pattern loop maintains its in-scope variable
  set incrementally (an ordered set, not a hash, to stay a zero-dependency wasm-clean leaf),
  removing a quadratic parse-time cost on adversarial input; debug builds assert the incremental
  set against a fresh walk at every consultation.

### Refactor
- **sparql-eval:** Drive built-in aggregates through the same `AggregateAccumulator` fold trait
  as caller-registered ones — each built-in is now a concrete accumulator monomorphized through
  one generic driver, rather than a hand-rolled counter plus a per-function enum. No behavior
  change; governor charging and the frozen corpus are untouched.

### Documentation
- **sparql:** Document the aggregate seam and the SPARQL 1.2 remainder: the book's query chapter
  gains `ADJUST`, the `VERSION` declaration, the custom-aggregate seam with a worked registration
  example, and the ten statistical aggregates; the results chapter documents the `its:dir`
  spelling and the caller-named provenance extension.
- **sparql:** Correct the book's query chapter, which still described the entailment-aware query
  lane as refusing `aggregate_namespace` on every host — a paragraph an earlier fix removed from
  the Python stub and one book location but left standing in a second. It now shows the
  combination working, with a worked `--entailment`/`--aggregate-namespace` CLI example.
- **sparql:** Document `LATERAL` (SEP-0006) in the book's query chapter: the production, a
  top-1-per-group worked example, the scope restriction with the SEP's own legal/illegal pair,
  the two deliberate divergences from Jena, the `UPDATE WHERE` status, and the `SERVICE`-forwarding
  refusal. The front-end surface enumeration now names it as a SEP extension.
- **sparql:** Document `EXISTS`/`NOT EXISTS` under SEP-0007 in the book's query chapter: the
  precise points SEP-0007 repairs against SPARQL 1.1/1.2 §18.6's literal `substitute`/`evalExists`
  reading (variable-only positions, the `MINUS` domain flip, blank nodes as variables, disconnected
  variables, and the Part 3 assignment restriction), the one `Replace`/`PrjMap` definition,
  Existential Normal Form's rewrite laws, the two evaluation strategies and their `--explain`
  evidence counters, the performance characteristic naming the shapes the memoized probe cannot
  serve, and the Part 3 restriction with the SEP's own legal/illegal example pair. The front-end
  surface enumeration's prior "EXISTS decorrelation" claim — never accurate, since a correlated
  filter is answered either by proof-admitted probing or by genuine per-row substitution, never by
  a blanket decorrelation — is corrected to point at this section. The README's shipped-surface
  bullet and Direction list move `EXISTS`'s SEP-0007 semantics out of "near-term direction" and
  into what SPARQL 1.1/1.2 already ships.

- **conformance:** The embedding kNN lane (PURREMB nearest-neighbour retrieval) and the
  `purrdf-geo` GeoSPARQL 1.1 lane now have conformance-matrix rows, ratchet budgets and
  per-engine scoreboard entries. Both shipped with no matrix representation at all — the only
  trace of either was a forward reference inside the governor row — so the umbrella gate could
  not see a regression in either lane. The kNN row counts test functions rather than fixtures,
  because it grades a seam rather than a document format and has no corpus; the document says so
  rather than inventing a fixture count.

  The GeoSPARQL row counts **corpus geometries**, and the reason is recorded because the first
  version of it did not. Written as a `cargo test` tally over `purrdf-geo`'s five integration
  binaries it measured **33 on one machine and 37 on the CI runner from byte-identical source**,
  turning the doc drift-guard red for a reason unrelated to GeoSPARQL. A number that moves with
  the build environment is not a measurement — the same principle the matrix's `_no_scoreboard`
  path already states from the other direction — so the row now reports the 20 geometries of
  `purrdf_geo::determinism::CORPUS` whose serialized bytes fold into one `u64`, compared against
  the `GOLDEN_DIGEST` pinned in the test source. That comparison is an oracle rather than a
  self-report, and it is stable everywhere. The row additionally records what it does **not**
  measure: **no OGC conformance suite is vendored and none is claimed**, the crate's SHACL shapes
  being first-party `example.org` mirrors of the shipped OGC 22-047r1 validator (PurRDF mints no
  vocabulary IRIs), so the lane has an independent oracle for its determinism but none for its
  semantics, and a misreading of OGC 22-047r1 would pass.
- **release:** Correct `docs/RELEASE.md`'s outstanding-bootstrap section, which named **four**
  crates as having no crates.io record. `purrdf-datalog` has had one since 2026-07-31 and
  answers `0.12.0`; the genuinely unpublished set is `purrdf-cdt`, `purrdf-text` and
  `purrdf-geo`, and the heading, the body, the publish-order ordinals and the in-page anchor
  all said otherwise. That set is now `PURRDF_UNBOOTSTRAPPED_CRATES` in
  `scripts/release-crates.sh`, a ledger `scripts/check-crates-io-records.sh` holds to the
  registry in **both** directions — an unlisted missing crate and a listed present crate each
  fail the preflight on their own — and the prose restates the ledger under a gate. The
  bootstrap examples drop their pinned version literal in favour of the argumentless form,
  which reads the workspace version from `cargo metadata` and cannot rot. The `make doc` / CI
  "N publishable crates" comments are corrected 20 → 21.
- **BREAKING** **release:** Both publish lanes — `scripts/bootstrap-crates-io.sh` and the
  tag-driven `release-cargo.yaml` loop — now **verify** every `cargo publish` (cargo builds the
  packaged crate against the registry before the upload that cannot be undone), and
  `purrdf-geo` moves from 13th to 17th in the publish order to make that possible. The loop's `--no-verify` was load-bearing, not incidental: `purrdf-geo`
  dev-depends on `purrdf-rdf` and `purrdf-shapes`, verification resolves the packaged crate's
  whole graph including dev-dependencies, and while `purrdf-geo` was ordered before both, a
  verifying publish of the set would have failed at crate 13 after twelve irreversible
  uploads. Moving one crate removes the last forward dev-edge. The new
  `scripts/check-publish-order.py` proves on every `make check` that the order is a
  topological order of normal **and** dev-dependencies, that the release set is exactly the
  publishable members, and that the bootstrap ledger is in-set and in order; its
  `--self-test` perturbs each check and requires the refusal, and the release workflow runs it
  again in its verify step at the point of no return. `PUBLISH_NO_VERIFY=true` restores
  the old behaviour for one run. Anyone with a checked-out publish order, a pinned release
  script, or a Trusted Publisher configured by ordinal must re-read
  `scripts/release-crates.sh`.
- **release:** `PUBLISH_COOLDOWN_SECONDS` defaults to `0` (was `620`). crates.io's new-crate
  rate limit is enforced at the publish: a limited `cargo publish` exits non-zero, `set -e`
  stops the run before the next crate, and a re-run resumes because published versions are
  skipped — a visible, resumable refusal, not a corrupted release. The old default modelled the
  limit's ten-minute refill unconditionally and added about half an hour of dead time to a
  three-record run. The environment override is kept.

### Testing
- **results:** Pin its:dir precedence over legacy spellings in SRX
- **sparql,conformance:** Pin the newly-added SPARQL evaluation surface in the conformance
  corpus: the `VERSION` declaration evaluated (not merely parsed), the `AGG` call form's
  grammar, and `GROUP_CONCAT`'s row-order concatenation pinned to an exact string.
- **sparql-algebra:** Pin the whole serializer with a corpus round-trip sweep: parse, serialize,
  re-parse, and compare (modulo left-linearized join spines) over every vendored W3C, first-party,
  and doc-example query text — the empty exception ledger is the point; every disagreement it
  found is fixed above, not ledgered.
- **rdf:** Reduce the golden-capture deferred-construct classifier to the engine's actual typed
  residue — `LATERAL`, `SERVICE`, `DESCRIBE`, and property paths no longer misroute into expected
  deferral now that they are implemented.
- **sparql-conformance:** Add the `purrdf-extend` `LATERAL` manifest cases: the SEP-0006 worked
  examples (including the scoping oracle, proving Project-boundary narrowing rather than mere
  textual substitution), a shared-variable-injection case, and the SEP's own legal/illegal syntax
  pair.
- **sparql-conformance:** Add eight `purrdf-extend` SEP-0007 `EXISTS`/`NOT EXISTS` manifest cases
  through the shipped stack — the correlated graph-variable body, the `OPTIONAL`-padding tautology
  in both polarities, nested negation, a per-row `LIMIT 1` sub-select a one-shot probe would
  truncate wrongly, the `MINUS` shape whose right-operand correlation a one-shot probe would flip,
  and the Part 3 assignment restriction's own colliding-`BIND`/colliding-`VALUES` negative-syntax
  pair — bringing the suite to fifty-five cases (forty-eight evaluation, five negative-syntax).
  Each expected evaluation result was hand-derived from the `Replace`/`PrjMap` substitution
  definition before being pinned against the release binary, so a still-wrong engine could not
  have pinned itself correct.
- **sparql-eval:** Pin the probe/definition strategy boundary from both sides with a test-only
  forced-strategy seam: agreement tests run every admissible shape through both strategies and
  assert row-for-row equality, and divergence witnesses force the probe onto each refused shape
  and assert the specific wrong answer it would give. A bounded-exhaustive generator sweeps
  hundreds of inner shapes at depth two, checking memo equivalence throughout and cross-strategy
  agreement on every admitted one. Twenty-four `FILTER EXISTS`/`FILTER NOT EXISTS` shapes also run
  as real query text through the public engine end to end, including the substitution document's
  own worked examples, every solution modifier inside the body, the `HAVING`-position scope pin,
  and quoted-triple/blank-node outer bindings.


- **conformance:** Bring `scripts/conformance-baseline.json`'s free-text `note:` prose under the
  same gate as its `ledgered` integer. Only the integer was ever machine-checked, and the OWL 2
  DL note rotted a full generation behind it — claiming 261 vendored cases, 30 non-terminating
  and 12 withheld exclusions against a measured 262, 0 and 25, with every gate green the whole
  time. `scripts/check-doc-claims.py` gains `baseline_note_claim`, which checks that the
  baseline's suite names and the generated matrix block's suite names are the same set, that
  every `ledgered` budget equals its matrix row's XFail/Skip column, and that every
  matrix-derivable integer any note restates equals the column that measures it — with the DL
  note's subset/exclusion tally sourced from the same frozen `census.tsv` the three
  `docs/CONFORMANCE.md` restatements already are. A reworded note that stops matching its
  pattern fails as loudly as a wrong number, so the gate cannot be silenced by rewriting prose.
- **release:** `scripts/check-doc-claims.py` gains `outstanding_bootstrap_claim` and
  `publishable_crate_count_claim`, holding `docs/RELEASE.md`'s bootstrap heading, body crate
  count, per-crate publish-order ordinals and in-page anchor — and the `Makefile` / CI
  publishable-crate counts — to `scripts/release-crates.sh`. Membership is deliberately left to
  `scripts/check-crates-io-records.sh`, since whether a crate record exists is a fact about
  crates.io rather than about this tree.

## [0.12.0] - 2026-08-02

### Bug Fixes

- **BREAKING** **canon:** Reserve the overlay's namespace by refusal, not by assertion

### Features

- **BREAKING** Let a new enum variant stop being a breaking change
- **errors:** Let the standard chain reach the failure underneath
- **canon:** Name and version the canonicalization profile, with a frozen vector corpus

### Performance

- **datalog:** Keep the join's binding frame off the heap
- **sparql-eval:** Reject a candidate before paying to copy the row
- **rdf:** Write the text serializers into one buffer instead of many
- **rdf:** Write TriG in place, blocks and all
- **rdf:** Sort the canonical order through a comparator, not through keys
- **rdf:** Cut allocations on the hot paths; fix(canon)!: collision-safe RDF 1.2 canonicalization profile

### Refactor

- **BREAKING** **rdf:** The codec's serializer takes the caller's buffer

### Testing

- **datalog:** Give the semi-naive join a bench of its own
- **conformance:** Put the canonicalization profile on the scoreboard

## [0.11.0] - 2026-08-02

### Bug Fixes

- **entail:** Make the survey's three-way early exit reachable
- **docs:** Hold the xfail sentence to the count the matrix generates
- **wasm:** Assert an error names a term, do not compile the term into a pattern
- **BREAKING** **entail:** `?name` in any position, including the one it was refused in
- **entail:** A boundary that meant two things, and two claims about rule heads that were false
- **python:** Give each newline one way to match, not two
- **wasm:** Assert a refusal names a term exactly, by neither of the two wrong ways
- **wasm:** Pin the whole refusal, so the IRI reaches no matcher at all
- **BREAKING** **entail:** A variable is not a datatype IRI, and a withheld predicate is not a smaller question
- **BREAKING** **entail:** One IRI is one answer, and one variable name is one variable
- **gates:** Two gates that could not fail, and two guards that could not see
- **docs:** Derive which documents the overclaim ban sweeps, from the ban itself
- **docs:** Make the derived sweep total, rather than saying it is
- **docs:** Let the gate's own guards be mutated, and refuse the mutations
- **docs:** Compose the ban patterns around their markers, rather than checking they contain one
- **BREAKING** **entail:** A deep input must be an error, not a dead process
- **BREAKING** **rdf:** Bound term nesting where it is parsed, and survey every position the merge writes
- **entail:** Five entailment-diagnostic fixes — a lossy diagnostic, a split precedence, and three claims nothing checked

### Documentation

- **entail:** The doc gate is not `make check`, and it found six broken links
- **rdf:** The XML-literal walk adds no bound; something in front of it does

### Features

- **entail:** A conclusion-directed entailment service over the RL chase
- **entail:** Close the negative-conclusion lane by refutation over the chase
- **entail:** Decide a schema axiom by freezing its body and chasing the head
- **entail:** Comprehend the anonymous class expressions a conclusion names
- **entail:** Read a reflexive property's self-loops off the conclusion, not the closure
- **BREAKING** **entail:** Decide an rdfs:range axiom by datatype containment
- **entail:** Vendor the document the last unreached premise names
- **BREAKING** **entail:** Return the run that answered, not the verdict alone
- **entail:** The conclusion-directed surface reaches all four hosts
- **conformance:** Print the split an empty ledger makes trivially true
- **BREAKING** **entail:** A conclusion graph is a conjunction, and entailment is monotone over one
- **BREAKING** **entail:** A lane not run is a limit, not a silence
- **BREAKING** **entail:** The import map is the caller's, on every host that has the service
- **cli:** The binary can ask the question, not only compute the closure
- **conformance:** Print what the twenty-three negative agreements are made of
- **BREAKING** **entail:** Close the 16 ledgered W3C OWL 2 RL entailment-corpus gaps

## [0.10.0] - 2026-07-31

### Bug Fixes

- **BREAKING** **datalog:** Carry the predicate as data so meta-rules are expressible
- **entail:** Stop fabricating rdfs:Resource for a derived triple term
- **entail:** Drop the tableau's unique name assumption for nominals
- Accept D from the CLI, close the umbrella gap, correct shipped strings
- **release,docs:** Refuse a half-publish, and make documented numbers checkable
- **entail:** Make the certificate real, and grade the rules against W3C
- **entail:** Make the overclaim state unrepresentable, and surface the certificate everywhere
- **entail:** Derive the DL certificate's completeness instead of storing it
- **wasm:** Reach every reasoner service from the npm package root
- **docs:** Unbreak the rustdoc gate and name the Python suite row for what it runs
- **bindings:** Make the shipped type stub match the extension, and gate what published numbers claim
- **docs:** Gate the numbers that recurred, and stop promising a component this workspace does not have
- Make three gates inspect what they claimed to, and cover a tableau clash nothing reached
- **docs:** Correct eight published figures and gate the surfaces that carried them
- **docs:** Close three ways this pass's own gates could be satisfied without checking anything
- **entail:** Refuse explain-conclusion per conclusion, not per regime
- **BREAKING** **docs:** Name the DL fragment SHOIQ(D), date the exclusion emitters, and correct three provenance claims
- **docs:** A concept ledger over repo-wide facts, and twenty corrected figures
- **BREAKING** **entail:** Refuse the unrepresentable cardinality, bound the counting search, and state what the blocking evidence shows
- **BREAKING** **entail:** Make the combined approach reachable, sound on every result form, and honest about its fragment
- **docs:** The figure sweep — thirty corrected claims, three gate holes closed
- **hygiene:** The issue-reference ban could not see string literals or SPARQL
- **playground:** The console offered seven codecs while the engine registers nine
- **entail:** The counting/inverse limit keyed on spelling, not on meaning
- **python:** `uv run mypy` checked nothing and exited on a usage error
- **wasm:** The session handle needs Debug and Self, as the lint table requires
- **hygiene:** The wasm export gate read the export block and not the imports
- **datalog:** Make the SLG budget bound the work, not just the output
- **datalog:** Keep freshen_clause's doc on freshen_clause, and collapse the filter's ifs
- **build:** Make the wasm artifact's size independent of who built it

### CI & Build

- **wasm:** Record the session's 8,882 bytes
- **wasm:** Raise the ceiling 25% and stop failing on the exact byte count
- **npm:** Raise the package ceilings 25% and stop failing on the exact byte count

### Documentation

- **entail:** Generate the rule inventory and correct every stale claim
- Fix two intra-doc links that failed the rustdoc gate
- **conformance:** Grade the rules against W3C, and gate every number
- Date the DL exclusion tally, which is a recorded measurement rather than a live one
- **provenance:** Record the cutover as it stands, the slme port, and the revision reachability
- Published text carried internal program codes
- The session's three Python tests and its bytes reach the recorded figures
- **conformance:** The prose scoreboard row lagged the generated block
- **provenance:** Say which generated projections this repo can actually regenerate
- Say why the backward resolver has no caller, and stop claiming it has one
- State the backward check's boundary as the cost it is, with numbers that reproduce
- **validate:** The skip test's own doc still told the story the code disproved

### Features

- **BREAKING** **datalog:** Add the purrdf-datalog crate and wire every release gate
- **datalog:** Port the physical primitives — branded ids, arena, bitset, binding patterns
- **datalog:** Port the relation store, cursors, and index-selection planner
- **datalog:** Port the semi-naive evaluator with analytic goldens and hard budgets
- **datalog:** Replace the provisional rule IR with the DL-clause IR
- **entail:** Add the machine-readable rule inventory
- **datalog:** Add checkable proof terms and a contract hash
- **BREAKING** **entail:** Return a reasoning certificate from every materialize call
- **validate:** Add the shared entailment-regime string boundary
- **entail:** Seed the finite axiomatic triples and add four RDFS rules
- **python:** Expose entailment regimes as purrdf.entail
- **entail:** Add RDF-list materialization and the prp, cax and scm rules
- **wasm,capi:** Expose entailment regimes to WebAssembly and the C ABI
- **entail:** Complete OWL 2 RL — all 78 rules, and make D materializable
- **entail:** Stop dropping OWL axioms silently, and add the existential chase
- **entail:** Expose the DL reasoner services behind a certified facade
- **entail:** Dataset semantics, reifier interactions, and explanations
- **entail:** Reach every reasoner service from every host
- **entail:** Close the last W3C entailment gap, wire the plan cache, report termination
- **entail:** Bind the extension inventory on every host and gate its disclosure
- **BREAKING** **entail:** Decide OWL 2 data ranges, and check the tableau against a model-enumeration oracle
- **xsd:** Decide the rational-decimal identity exactly, and write the gmeow cutover guide
- **BREAKING** **entail:** Certain answers by the combined approach, over a ported SLG-WFS resolver
- **BREAKING** **entail:** A clause-based hypertableau is the OWL-Direct decision core
- **entail:** The nominal/inverse/counting limit is a named boundary, not buried prose
- **validate:** A reasoning session, so asking twice costs one parse
- **python:** Expose the reasoning session as `entail.Reasoner`
- **wasm:** Expose the reasoning session as `Reasoner`
- **capi:** Expose the reasoning session as `PurrdfReasoner`
- **purrdf:** Surface the reasoning session on the Rust facade, and gate all four hosts
- **entail:** Cross-check every chase explanation against backward resolution
- **entail:** Re-derive every chase explanation backward, and report the outcome
- **BREAKING** **entail:** Complete the entailment surface — 78/78 OWL 2 RL, certified runs, four language hosts

### Other

- Revert "feat(entail): cross-check every chase explanation against backward resolution"

### Performance

- **BREAKING** **entail:** Classify by one saturation instead of a tableau run per class pair
- **datalog:** Reject impossible clause/call pairs before freshening them

### Refactor

- **BREAKING** **entail:** Run the declared clause program instead of a hand-written chase
- **entail:** Split the calculus into one module per rule family
- **BREAKING** **entail:** Make materialization total over every regime

### Testing

- **entail:** Capture the chase's behaviour as a golden oracle
- **conformance:** Vendor the W3C OWL 2 suite and give entailment its own row
- **entail:** Check the OWL 2 RL closure against an independent second implementation
- **entail:** Assert the tableau is not over-permissive where that is decidable
- **entail:** Pin the owlrl divergence triples, and cover the value class that separates two language-tagged values
- **entail:** Pin the divergence triple count so a regeneration cannot absorb a regression
- Carry the per-conclusion explain contract to the remaining two hosts

## [0.9.0] - 2026-07-28

### Bug Fixes

- **gts:** Use neutral fixtures, drop an unreachable guard, surface breaking changes

### CI & Build

- **changelog:** Mark a breaking release when the squash duplicates a subject

### Documentation

- **gts:** Name the real undicted plan in the frozen-vector test

### Features

- **BREAKING** **gts:** Pin caller-supplied in-band dictionary bytes in a compaction plan

### Testing

- **gts:** Say what the mixed-plan header assertion can actually observe

## [0.8.5] - 2026-07-26

### Bug Fixes

- **slice:** Scope ownership by declared term namespaces, not the framework ns
- **slice:** Never mistake a Turtle comment for the sliceDependsOn block
- **docs:** Repair public GTS links
- **gts:** Harden dictionary append invariants

### Documentation

- **gts:** Register zstd-rsyncable level?/dct? and the dict-vector corpus

### Features

- **gts:** Multi-dictionary packs, rsyncable dict priming, and a declared zstd level
- **gts:** Support multi-dictionary rsyncable packs

### Testing

- **gts:** Freeze dictionary vector fold oracles

## [0.8.3] - 2026-07-23

### Bug Fixes

- **rdf:** Make JSON-LD byte budgets target independent
- **rdf:** Harden JSON-LD size arithmetic
- **capi:** Refresh generated package version
- **rdf:** Complete JSON-LD portability audit
- **rdf:** Make JSON-LD byte budgets portable

### CI & Build

- **release:** Gate coordinated tags on full checks
- **release:** Validate every published surface

### Other

- **shapes:** Restore canonical formatting

### Testing

- **rdf:** Use inclusive multiplicity range

## [0.8.2] - 2026-07-22

### Bug Fixes

- **shapes:** Make SHACL-AF function scope re-entrancy-safe under parallel validation
- **shapes:** Disambiguate ontology classes named after reserved JSON-Schema $def keys
- **rdf:** Raise JSON-LD carrier/document row ceilings to 2^23 for large bundles
- **rdf:** Size the JSON-LD carrier envelope for a whole-ontology bundle
- **rdf:** Raise JSON-LD output/document byte ceilings for whole-ontology bundles

## [0.8.1] - 2026-07-21

### Bug Fixes

- **wasm:** Align npm package size budgets

### CI & Build

- **wasm:** Budget shared SHACL membership view

### Documentation

- **wasm:** Record optimized membership artifact
- **conformance:** Record subclass corpus case

### Features

- **shapes:** Add subclass membership view
- **shapes:** Unify subclass membership semantics

### Other

- Unify SHACL subclass membership across native and SPARQL validation

### Performance

- **shapes:** Benchmark subclass membership hot path
- **shapes:** Prune inactive membership rows

### Testing

- **shapes:** Freeze subclass membership semantics
- **shapes:** Freeze subclass corpus hashes

## [0.8.0] - 2026-07-20

### Bug Fixes

- Harden purremb contracts and lookups
- **shapes:** Standardize schema input blanks apart
- **shapes:** Return typed schema key errors
- **shapes:** Admit identifier-only ontology ranges
- **shapes:** Retain custom datatype ranges
- **shapes:** Bound propagated schema facts
- **shapes:** Lower unsafe LinkML slots deterministically
- **shapes:** Bound schema parsing during construction
- **shapes:** Reject initializer module components
- **jsonld:** Preserve active-context option semantics
- **jsonld:** Preserve compact carrier round trips
- **jsonld:** Bound derived context validation
- **cli:** Bound JSON-LD options input
- **python:** Isolate generated namespace bindings
- **playground:** Route configured JSON-LD formats
- **capi:** Update projection example scope
- **wasm:** Update projection scope fixtures
- **csvw:** Preserve W3C RDF conversion
- **capi:** Refresh generated projection header
- **build:** Measure OKF wasm package growth
- **capi:** Close smoke fixture on seek failure
- **build:** Make Cargo target fallback safe
- **capi:** Honor active smoke-test profile
- **build:** Harden Cargo target discovery
- **release:** Deduplicate generated changelog entries
- **capi:** Regenerate header for 0.8.0

### CI & Build

- **wasm:** Rebaseline scoped projection budgets

### Documentation

- **rdf-core:** Specify the PURREMB v1 format
- **shapes:** Expose ontology schema workflow
- **shapes:** Publish LinkML slot migration contract
- **shapes:** Document rich Pydantic package emission
- **shapes:** Qualify flat byte compatibility
- **jsonld:** Document deterministic context compaction
- **conformance:** Refresh compatibility count
- **conformance:** Refresh Python parity count
- **csvw:** Guide curated terms projection
- **cli:** Enumerate liftable profiles
- **wasm:** Refresh optimized size measurement
- **cli:** Explain OKF lift rejection
- **conformance:** Account for attached parity test

### Features

- **rdf-core:** Add deterministic PURREMB writing
- **rdf-core:** Add borrowed PURREMB reading
- **rdf-core:** Add PURREMB binding verification
- **rdf-core:** Add deterministic .purremb companion format
- **sssom:** Model set-level document comments
- **sssom:** Retain parsed document envelopes
- **sssom:** Serialize typed document envelopes
- **shapes:** Define ontology schema compilation contract
- **shapes:** Derive deterministic ontology schema surface
- **shapes:** Emit ontology-complete schema carriers
- **shapes:** Define LinkML slot naming contract
- **shapes:** Verify LinkML slot reports on import
- **shapes:** Define deterministic Pydantic package topology
- **shapes:** Emit routed rich Pydantic packages
- **rdf:** Compile JSON-LD active contexts
- **rdf:** Compact JSON-LD through a typed carrier
- **rdf:** Derive deterministic JSON-LD contexts
- **cli:** Expose configured JSON-LD serialization
- **bindings:** Expose compiled JSON-LD contexts
- **lpg:** Require explicit projection scope
- **lpg:** Stream projection artifacts
- **lpg:** Expose scoped streaming hosts
- **csvw:** Define curated terms profile
- **csvw:** Project scoped curated term tables
- **projection:** Expose curated CSVW terms
- **rdf:** Define OKF terms projection contract
- **rdf:** Generate deterministic OKF term bundles
- **rdf:** Expose OKF terms across hosts
- **rdf:** Add attached RO-Crate assets
- **bindings:** Expose attached RO-Crate packaging

### Other

- Compile ontology-complete developer schema surfaces
- Make LinkML slot lowering deterministic and reversible
- Emit deterministic rich Pydantic packages
- **jsonld:** Document scoped lint exceptions
- Add deterministic context compaction
- Preserve typed SSSOM set comments
- Add scoped streaming LPG projections
- Add caller-configured curated CSVW projections
- Add deterministic caller-configured OKF term bundles
- Add deterministic attached RO-Crate payload packages
- Add native DCAT and VoID dataset descriptions
- Benchmark whole-bundle SHACL focus execution
- Optimize SHACL focus validation invariants
- Parallelize SHACL focus evaluation deterministically
- Prepare bounded SHACL validation for realtime use
- Eliminate SHACL canonical sort key allocations
- Preserve interned IDs through recursive SHACL checks
- Prove deterministic SHACL parallel execution
- Document realtime SHACL validation operations
- Optimize realtime and whole-bundle SHACL validation

### Performance

- **rdf-core:** Benchmark PURREMB access paths
- **sssom:** Streamline column selection
- **shapes:** Cache ontology row sort keys
- **shapes:** Borrow unchanged LinkML slot locals
- **shapes:** Reuse Pydantic path buffers
- **shapes:** Avoid duplicate schema limit traversal
- **shapes:** Index routed definition owners
- **jsonld:** Remove carrier hot-path allocation
- **lpg:** Measure scoped streaming carriers
- **projection:** Coalesce artifact sink writes
- **csvw:** Reuse URI expansion allocations
- **csvw:** Remove curated selection temporaries
- **csvw:** Borrow curated table memberships
- **rdf:** Remove OKF classifier scratch allocations
- **rdf:** Remove attached crate loop allocations

### Refactor

- **shapes:** Clarify routed name guard
- **jsonld:** Unify context processing

### Testing

- **rdf-core:** Harden PURREMB conformance coverage
- **shapes:** Prove ontology surface across emitters
- **shapes:** Keep namespace fixture vocabulary neutral
- **shapes:** Prove emitter-specific ownership
- **shapes:** Harden LinkML slot lowering
- **shapes:** Exercise routed Pydantic packages
- **jsonld:** Freeze expanded codec baseline
- **rdf:** Pin OKF terms cross-host parity
- **rdf:** Freeze attached crate host parity

## [0.7.0] - 2026-07-17

### Benchmarks

- **core:** Measure wavelet indexes against pack FoQ
- **shapes:** Measure LinkML imports

### Bug Fixes

- **shapes:** Record losses for non-class shape targets instead of dropping silently
- **capi:** Regenerate the ABI header for the 0.6 version bump
- **rdf:** Skip triple-term self-reifier sentinels in the JSON-LD reifier index
- **rdf:** Graph-scope reifier/annotation identity end-to-end
- **rdf:** Record TriX/HexTuples base-direction drop in the loss ledger
- **cli:** Reach reason stdin/stdout via --from/--to + fail-fast format resolve
- **cli:** Treat stdout BrokenPipe as a clean exit, not a runtime error
- **wasm,playground:** Honest size re-baseline + JSON-LD bidirectional round-trip
- **rdf-core:** Keep page planning metadata-only
- **rdf-core:** Make paged view debug inert
- **columnar:** Support published loss ledger API
- **columnar:** Bound untrusted decode allocations
- **rdf:** Scan escaped OKF link destinations
- **rdf:** Normalize exponent-form OKF decimals
- **shapes:** Enforce constrained schema unions
- **shapes:** Align Pydantic loss contracts
- **shapes:** Enforce JSON carrier fidelity
- **shapes:** Harden TypeScript reference closure
- **shapes:** Close TypeScript loss-audit gaps
- **shapes:** Bound TypeScript alias-cycle scans
- **shapes:** Describe GraphQL key validation bidirectionally
- **capi:** Restrict projection output permissions
- **rdf:** Honor CSVW record dialects
- **rdf:** Validate CSVW BCP 47 tags

### CI & Build

- **wasm:** Raise size budget for packed-dataset restoration
- **wasm:** Raise npm tarball size ceiling for packed-dataset restoration

### Documentation

- **core:** Drop stale issue-number token from loss_matrix_json doc comment
- **core:** Unlink private registry_entries from public loss_matrix_json doc
- **rdf:** Unlink classify from the private FORMATS table for rustdoc
- **cli:** Add the purrdf CLI README; fix a private intra-doc link
- **query:** Define paged completeness contract
- **shapes:** Document Pydantic projection
- **shapes:** Document LinkML projection
- **shapes:** Define TypeScript projection contract
- **shapes:** Define the GraphQL carrier boundary
- **rdf:** Complete projection adoption surface
- **conformance:** Refresh projection parity count
- Refresh conformance parity count
- **shapes:** Complete schema reverse surface
- Keep issue tracking out of provenance

### Features

- **core:** Unify the runtime loss ledger and add an enumerable codec-pair registry
- **shapes:** Record schema-projection losses on the unified LossLedger
- **core:** Add reusable ledger soundness + completeness verification helpers
- **core:** Enumerate the codec-pair registry and pin the runtime-ledger schema
- **core:** Make loss_matrix_json the enumerable codec-pair registry
- **core:** Unified loss-ledger surface for all codecs
- **rdf:** Register JSON-LD/YAML-LD as first-class native format variants
- **rdf:** Lossless JSON-LD-star triple-term encoding + orphan-reifier fix
- **rdf:** Lossless nested triple terms + reject annotations inside a @triple
- **rdf:** Unify JSON-LD/YAML-LD into the native format registry
- **rdf:** Make the native serializer generic over DatasetView
- **rdf-core:** Public DatasetView-generic pack reconstructor
- **cli:** Add the purrdf CLI crate (convert/query/reason core)
- **cli:** Convert --base/--entailment/--canonical + full matrix tests
- **cli:** Query --base/--entailment + CONSTRUCT/DESCRIBE RDF sink
- **cli:** Reason --base + per-regime boundary diagnostics + tests
- **cli:** The purrdf CLI — convert / query / reason
- **rdf:** Expose packed dataset restoration
- **rdf-core:** Certify paged provider snapshots
- **rdf-core:** Add fallible paged query views
- **sparql:** Certify complete fallible query results
- Certify fallible paged SPARQL execution
- **columnar:** Define five-table Parquet contract
- **columnar:** Implement deterministic Parquet kernel
- **columnar:** Project RDF datasets to five tables
- **columnar:** Reconstruct RDF from five tables
- **columnar:** Add deterministic bidirectional Parquet codec
- **rdf:** Define OKF loss contracts
- **rdf:** Lift OKF bundles into event sinks
- **rdf:** Write deterministic OKF bundles
- **rdf:** Add native bidirectional OKF codec
- **core:** Register Pydantic projection losses
- **shapes:** Emit Pydantic v2 packages
- **core:** Register LinkML loss profile
- **shapes:** Add canonical LinkML codec
- **shapes:** Project schemas to LinkML
- **shapes:** Add canonical LinkML 1.11 projection
- **core:** Register TypeScript projection losses
- **shapes:** Emit TypeScript declarations
- **shapes:** Emit deterministic TypeScript declarations
- **core:** Register GraphQL loss profile
- **shapes:** Emit deterministic GraphQL SDL
- **rdf:** Add projection carrier foundations
- **rdf:** Add canonical LPG mapping
- **rdf:** Add LPG CSV adapters
- **rdf:** Add LPG graph carriers
- **rdf:** Add bidirectional CSVW projections
- **rdf:** Add OBO Graphs projection
- **rdf:** Add deterministic SKOS projection
- **projections:** Expose deterministic carrier surfaces
- **rdf:** Add graph and tabular projections
- **rdf:** Add research-object semantic pivot
- **rdf:** Add Croissant 1.1 codec
- **rdf:** Add RO-Crate 1.3 codec
- **rdf:** Add DataCite 4.6 codec
- **rdf:** Add DCAT 3 codec
- **rdf:** Add Frictionless Data Package codec
- **rdf:** Expose research-object carrier surfaces
- **rdf:** Complete research-object carrier integration
- **rdf:** Add bidirectional research-object codecs
- **core:** Register schema to SHACL loss profiles
- **shapes:** Import JSON Schema as SHACL
- **shapes:** Import LinkML as SHACL
- **shapes:** Import generated schema packages
- **shapes:** Import schemas as SHACL

### Other

- Expose packed dataset restoration
- Emit caller-configured Pydantic v2 packages

### Performance

- **core:** Memoize the codec-pair loss registry behind OnceLock
- **cli:** Mmap-borrow disk pack→pack passthrough instead of heap-buffering
- **shapes:** Avoid duplicate Pydantic schema work
- **shapes:** Avoid LinkML projection allocations
- **shapes:** Avoid TypeScript render copies
- **shapes:** Avoid redundant GraphQL oracle escaping
- **rdf:** Avoid clean JSON pointer allocation
- **rdf:** Avoid DataCite XML uppercase copy
- **shapes:** Reduce schema import allocations

### Refactor

- **core:** Remove the dead RdfLoss diagnostic type
- **core:** Derive loss-entry intentional from profile membership
- **rdf:** Single FormatDescriptor table as the format metadata source of truth
- **wasm:** Route the wasm format resolver through the one core registry
- **shapes:** Share compiled schema catalog

### Testing

- **rdf:** Assert bnode-scope-flatten is an in-profile loss
- **rdf:** Production-surface tests for JSON-LD/YAML-LD + named-graph reifier fix
- **rdf:** Pin YAML-LD adversarial-scalar literals against the Norway problem
- **wasm:** Assert isomorphism on the JSON-LD/YAML-LD round-trip + fix stale docs
- **cli:** Pin --loss-ledger surfacing tri-state + universal-sink invariant
- **cli,rdf:** Pin query --entailment exit-3 boundary + dedup pack test fixture
- **sparql:** Cover fallible query guarantees
- **columnar:** Prove backend and DuckDB interoperability
- **columnar:** Cover empty files in DuckDB oracle
- **shapes:** Execute Pydantic schema oracle
- **shapes:** Exercise recursive Pydantic refs
- **shapes:** Add official LinkML oracle
- **shapes:** Add TypeScript compiler oracle
- **shapes:** Verify GraphQL coercion with GraphQL.js
- **cli:** Tolerate early stdin closure

## [0.6.0] - 2026-07-14

### Bug Fixes

- **gts:** Keep original authorship signatures bound across repacks
- **rdf:** Fail closed instead of panicking when verifying poison packs
- **rdf:** Anchor compaction projection to the provenance predicate vocabulary
- **gts:** Make the packaging signature a required compaction parameter
- **paged:** Enforce G3 quad-disjointness across the side tables
- **core:** Unify the pack dictionary to one id per term value for DatasetView compatibility
- **core:** Reject a literal datatype id that does not reference an IRI in the pack decoder
- **core:** Fail closed in verify_pack when a reconstructed dataset is structurally invalid
- **core:** Fail closed on FoQ index count-sum overflow in the pack decoder
- **shapes:** Sh:in enum members match the instance projector encoding
- **sparql-eval:** Make the fork-join parallel-safety gate registry-aware
- **gts:** Hard-fail the event bridge on a dangling term reference
- **gts:** Fire StreamingSink::frame before a frame's rows

### Documentation

- **design:** Author the PurRDF backend contract (C-clauses + paged G-clauses)
- **paged:** Fix rustdoc intra-doc link errors denied by the doc gate
- **core:** Document the pack backend + add the pack_query criterion bench
- **gts:** Fix rustdoc private-intra-doc-link errors denied by the doc gate
- **shapes:** Document the value-vocabulary enum projection
- **gts,rdf:** Demote private intra-doc links to code spans for the doc gate
- **sparql-eval:** Fix private intra-doc link denied by the doc gate
- **sparql-eval:** Broaden user_fn module doc to both function kinds
- **gts,rdf:** Demote private intra-doc links denied by the workspace doc gate
- **sparql-eval:** Drop plan-id process-flow refs from native-fn tests
- **core:** Describe the pack module by behavior, dropping plan task refs
- **core:** Reword residual plan-task phrasing in pack module docs
- **gts:** Add the §7.7 streaming-fold cross-reference and repair the rustdoc doc gate
- **gts:** Correct GtsEventSink provenance-ordering doc after the frame fix
- **gts:** Drop public-to-private intra-doc links tripping the doc gate

### Features

- **gts:** Deterministic in-band pack dictionaries for the zstd dct codec
- **gts:** In-band zstd dct codec with finalized pack dictionaries
- **gts:** Train and pin an in-band pack dictionary in streamable compaction
- **gts:** Bind detached signatures under an MMR root with a packaging head sig
- **rdf:** CompactionCertificate and verify_compaction refold-equivalence API
- **rdf:** Witness the suppression-compaction commuting square on both digests
- **gts:** Compress compaction blobs against the pinned in-band dict
- **core:** Make DatasetView an unsealed, id-agnostic read seam
- **core:** Add GlobalTermId + GlobalDictionary u64 identity layer
- **core:** Add PagedDataset — id-agnostic demand-paged DatasetView over a u64 dictionary
- **sparql-eval:** Generify the evaluator over D: DatasetView
- **sparql-eval:** Make the binding layer id-generic over D::Id
- **core:** Add freeze-refusal + deterministic compaction to PagedDataset
- **paged:** Add from_parts warm-restart constructor for PagedDataset
- **core:** Id-agnostic DatasetView seam + paged u64 backend served by the evaluator
- **gts:** Certified signature-preserving compaction with in-band dict codec
- **core:** Succinct rank/select + bit-packed IntVector primitives for the pack codec
- **core:** Four-section PFC value dictionary for the pack codec
- **core:** Graph-partitioned succinct bitmap-triples with FoQ all-pattern indexes
- **core:** RDF 1.2 reifier + annotation side-tables for the pack codec
- **core:** Deterministic pack container framing + zero-copy PackView reader
- **core:** Implement DatasetView for PackView (PackId, all-pattern query seam)
- **core:** Certified read-only projection — verify_pack recomputes the RDFC-1.0 digest
- **shapes:** Project value vocabularies to enum $defs (projection-only)
- **shapes:** Resolve sh:class / rdfs:range value-vocab refs to enum $defs
- **sparql-eval:** Native Rust-closure user-function registry + dispatch
- **sparql-eval:** Native Rust-closure user-function registry
- **gts:** Two-inventory replication diff, splice reconstruction, and diff_json
- **gts:** Surface per-frame provenance (content-id + byte range) on the streaming read path
- **gts:** Stream GTS frames into an RdfEventSink with per-frame provenance
- **gts:** Two-inventory diff/splice fetch-list + streaming RdfEventSink bridge on a shared decode core
- **core:** Read-only succinct HDTQ-style pack codec as a DatasetView backend

### Other

- **shapes:** Apply rustfmt to json_schema.rs
- **sparql-eval:** Rustfmt the native-scorer test helper

### Performance

- **paged:** Fold the G3 seal probe into a single map insert
- **paged:** Stream the paged read path instead of collecting a Vec
- **shapes:** Scan rdfs:range once per dataset for value-vocab $ref mapping
- **shapes:** Compute first_literal via single-pass running minimum
- **sparql-eval:** Lend native-fn args as &[&TermValue], drop per-call deep clone
- **gts:** Drop the streaming bridge's iri_map at each segment close
- **gts:** Hash the streaming decode maps with the fixed-key ahash policy

### Refactor

- **shapes:** Panic! directly for value-vocab enum-key twin guard
- **gts:** Hoist ByteRange to a shared model type and record FrameInventory.prev
- **gts:** Extract shared per-segment decode core and subsume the GTS import sink onto it
- **gts:** Bound the streaming decode core to one segment's memory

### Testing

- **gts:** Freeze in-band dict-compaction vectors with drift guards and docs
- **rdf:** Exercise the pack/tail seam chain across the boundary
- **gts:** Re-freeze the streamable-compacted vector to the current format
- **gts:** Assert the streamable-compacted pack pins no dct header entry
- **sparql-eval:** Prove SPARQL served directly over a multi-page PagedDataset
- **paged:** Cover property paths, aggregates, and subqueries on the paged backend
- **core:** Demonstrate mmap-able zero-copy PackView over a memory-mapped file
- **sparql-eval:** SPARQL-over-pack end-to-end parity with RdfDataset
- **shapes:** Prove value-vocab enum round-trip and open-validator decoupling
- **shapes:** Cover value-vocab class-key clash guard and multi-range tiebreak
- **sparql-eval:** Native scorer push-down + determinism (FILTER/ORDER BY/NaN/parallel)
- **gts:** Assert the streaming decode core's memory is bounded per segment
- **rdf:** Actually exercise GTS base-direction preservation on the sink path

## [0.5.0] - 2026-07-12

### Bug Fixes

- **shapes:** Key JSON-Schema $defs resolution set by def_key so cross-namespace local-name twins never dangle a $ref
- **wasm:** Free empty SELECT result handle without iteration
- **serialize:** Emit RDF 1.2 triple terms as non-asserting <<>>

### Documentation

- **conformance:** Record expanded Python parity suite
- **shapes:** Correct resolve_id doc to match variant-specific lookup

### Other

- Optimize Rust ownership and hot paths for 0.5.0

### Performance

- **core:** Remove owned lookup and freeze overhead
- **query:** Move parser tokens and stream results
- **reasoning:** Store terms once and index RIF joins
- **validation:** Cache sort keys and compile ShEx once
- **gts:** Stream deterministic CBOR without value clones
- **viz:** Linearize projection and ownership analysis
- **bindings:** Stream result ownership across FFI
- **reasoning:** Reuse RIF chase frontier index across fixpoint iterations
- **query:** Bound parser reparse fork to the braced block

### Testing

- **bench:** Cover SPARQL graph serialization and DISTINCT allocation paths
- **bench:** Cover SHACL canonical sort and ShEx prepared-shape paths

## [0.4.3] - 2026-07-10

### Bug Fixes

- **ci:** Serialize SPARQL conformance tallies

### Other

- Add semantic RDF 1.2 visualization exports

## [0.4.2] - 2026-07-10

### Other

- Harden npm wasm RDF 1.2 toolkit
- Expose entailed SPARQL and RIF parsing

## [0.4.1] - 2026-07-09

### Bug Fixes

- **shapes:** Project external object-class values to a node-ref, not a string, in JSON Schema
- **npm:** Align package-root RDFJS typings
- **capi:** Refresh generated ABI header
- **npm:** Accept null dataset inputs
- **npm:** Correct ecosystem probe evidence

### Documentation

- **npm:** Add ecosystem probe evidence

### Features

- **npm:** Add reusable SPARQL query engine

### Performance

- **wasm:** Benchmark query engine reuse

### Testing

- **npm:** Gate packed wasm package
- **npm:** Pin package gate toolchain

## [0.4.0] - 2026-07-07

### Bug Fixes

- **hygiene:** Exclude rustdoc inline-code spans from the issue-ref lint
- **makefile:** Use POSIX sed for wasm-bindgen pin extraction (macOS grep -oP)
- **makefile:** Use awk not tr to parse wc byte counts in wasm-pkg-size
- **hygiene:** Restrict issue-ref inline-code exclusion to Rust doc comments
- **playground:** Clear CodeQL alerts — structural entailment assertion + worker same-origin guard
- **shapes:** Polarity-sound sh:not projection in json_schema emitter
- **shapes:** Negate sh:not inner as a whole conjunction (De Morgan sound)
- **shapes:** Route sh:not maxCount property inner to a loss (no vacuous not)
- **shapes:** Route array-unsafe value-restriction sh:not inners to a loss
- **shapes:** Route existential sh:hasValue sh:not inner to a loss
- **shapes:** Restrict sh:not negand to exact-complement projections
- **shapes:** Polarity-sound sh:not projection (kill vacuous class negation)

### CI & Build

- **capi:** Gate the purrdf.h C-ABI header against drift
- **wasm:** Gate optimized artifact size via a pinned wasm-toolchain composite action
- **release:** Share the pinned wasm-toolchain action and enforce the size budget on release
- **wasm:** Drop unpinned twiggy source-build diagnostics step
- **docs:** Deploy the RDF-1.2 console at /playground in the Pages artifact

### Documentation

- Uplift product docs to top-tier Rust project standard
- **agents:** Document the wasm size-budget gate and deliberate-raise procedure
- Link the RDF-1.2 playground from the root and package READMEs
- **wasm:** List the shacl module in the lib.rs surface doc comment
- **shapes:** Strip issue-ref tokens from emitted schema descriptions
- Reconcile README and docs for 0.4.0

### Features

- **capi:** Make purrdf.h reproducible via cargo-c `capi` marker + regenerate canonically
- **capi:** Make purrdf.h reproducible via cargo-c `capi` marker + gate it in CI
- **wasm:** Add reproducible wasm-pkg-size budget gate (binaryen pinned)
- **wasm:** CI-gated wasm artifact size budget
- **wasm:** Expose SHACL + RDFC-1.0 canonicalize/isomorphic on the package surface
- **playground:** Standalone client-side RDF-1.2 console (engine in a Web Worker)
- **playground:** Drop the post-load wasm-size probe so the console makes zero network requests after assets load
- **playground:** Assert the SARIF 2.1.0 contract in the SHACL pane instead of echoing the engine version
- **playground:** Standalone deployed RDF-1.2 console over purrdf-wasm

### Other

- **shapes:** Satisfy fmt, clippy docs, and issue-ref hygiene gates

### Performance

- **shapes:** Cache the sort key for sh:not negand ordering

### Testing

- **shapes:** Add trusted external JSON-Schema validator harness
- **shapes:** Behavioral accept/reject tests for polarity-sound sh:not

## [0.3.3] - 2026-07-05

### Documentation

- **capi:** Regenerate purrdf.h with SHACL validate + entail declarations

### Performance

- **rdf:** Memoize the line index so parser diagnostics stay linear
- **rdf:** Memoize parser line index — fix quadratic diagnostics scan

## [0.3.2] - 2026-07-05

### Bug Fixes

- **shapes:** Reconcile SHACL-AF work with the merged 0.3.1 baseline

### Features

- **shapes:** SHACL Rules — 100% SHACL-AF coverage

## [0.3.1] - 2026-07-05

### Bug Fixes

- **shapes:** Pre-bind $shapesGraph in sh:SPARQLRule CONSTRUCT execution
- **build:** Optimize parse-hot workspace crates in dev/test profile to remove ~300x regression

### CI & Build

- **release:** Edition 2024, publish purrdf-entail, expose entail+validate, bump 0.3.1

### Documentation

- **conformance:** Add SHACL Rules scoreboard row; SHACL-AF is 100% complete
- **release:** Changelog for 0.3.1

### Features

- **shapes:** SHACL Rules engine — sh:TripleRule, sh:SPARQLRule, fixpoint entailment
- **shapes:** Cartesian-product multi-valued function-call node-expression args
- **bindings:** Expose SHACL rule entailment on Python, wasm, and C-API surfaces

### Performance

- **shapes:** Key the rules fixpoint divergence universe on Term, not String
- **shapes:** Reuse bindings buffer and hoist arg keys in function-call cartesian product

### Testing

- **shapes:** SHACL Rules conformance corpus + inferred-graph harness
- **shapes:** Audit every node-expression kind in sh:TripleRule subject/predicate/object positions
- **shapes:** Cover blank-focus blank minting and multi-round fixpoint convergence

## [0.3.0] - 2026-07-05

### Benchmarks

- **sparql-eval:** Isolate Solution row construction and join
- **rdf:** Add report-only span-tracking arm for the NoSpans zero-cost claim

### Bug Fixes

- **sparql-eval:** Correct correlated-EXISTS over address-keyed cache reuse
- **rdf:** Verify content-chain inclusion via blob/segment-head/MMR-leaf union
- **shex:** Route numeric_value through the XSD-1.0 float/double restriction
- **rdf:** Accept empty predicateObjectList item in Turtle parser
- **shex:** Fire group semantic actions only for participating groups
- **shex:** Make result-shape-map JSON fully round-trip through parse_shape_map
- **shex:** Distinguish unresolved IMPORT from conflicting redefinition
- **sparql-algebra:** Accept trailing top-level VALUES clause
- **sparql-conformance:** Dedupe named graphs by IRI + case-insensitive media-type
- **sparql-conformance:** Compare SELECT solutions up to W3C whole-set bnode isomorphism
- **sparql-eval:** XSD constructor casts emit canonical value forms
- **sparql-eval:** Correct built-in value spaces and XSD 1.1 canonical decimal
- **sparql-eval:** Scope UPDATE template blank nodes per request, not per operation
- **sparql-algebra:** Rewind trailing dot after blank-node label
- **sparql:** Exclude MINUS-right-only variables from in-scope set
- **shex:** Surface the concrete import parse error instead of swallowing it
- **shex:** Propagate concrete import cause through ImportResolver
- **conformance:** Make the byte-freeze manifest cross-platform deterministic
- **conformance:** Normalize CRLF in the matrix doc drift-check
- **conformance:** Honest matrix reporting for compile errors and first-party corpus
- **shacl-af:** Record sh:expression as a lossy JSON Schema projection
- **shacl-af:** SPARQL-value orderby, canonical set outputs, value-true is_true
- **shacl-af:** Keep sh:desc and described constant IRIs out of function-call parsing
- **conformance:** Repair --group dev gate, lock compat ratchet, refresh matrix
- **shacl-af:** Unbound mandatory sh:SPARQLFunction parameter yields no result
- **shacl-af:** Treat sh:returnType as informational, not an enforced datatype
- **shacl-af:** Reject empty or SHACL-reserved sh:SPARQLFunction parameter names
- **shacl-af:** Hard-fail malformed sh:order/sh:optional and multi-projection bodies
- **shacl-af:** Merge sh:SPARQLFunction body state back into the caller
- **shapes:** Repair broken merge — stray conflict markers and missed run_select rename
- **shapes:** Repair non-compiling SHACL-SPARQL component merge
- **shapes:** Make the validation-report sort total for byte determinism
- **entail:** Reject non-range-restricted RIF rules instead of panicking
- **entail:** Deterministic RDFS/OWL inferred-triple emission order
- **rdf:** Report located diagnostics at the offending token, not the next one
- **rdf:** Report Turtle/TriG located diagnostics at the offending token
- **rdf,validate:** Emit real byteOffset for N-Triples/N-Quads source spans
- **validate:** Wire SarifOptions::source_root_uri into the emitted SARIF
- **rdf:** Standardize blanks apart on native quad merge to stop cross-source collapse
- **release:** Stamp the pending version in the generated changelog
- **release:** Treat registry-restricted crates as publishable
- **release:** Anchor the package.json version bump to the top-level key
- **release:** Ignore commented-out crates in the publish-list parser
- **release:** Guard release-tags against a missing CHANGELOG section before tagging
- **release:** Assert per-crate version coherence in check-versions.py
- **hygiene:** Purge issue refs from python shim docstring and lint docstrings
- **rdf:** Pass the span collector in the empty-namespace turtle test
- **rdf-core:** Reject rdf:_0 and leading-zero container membership ordinals

### CI & Build

- **release-npm:** Guard binaryen --enable-simd + document SIMD baseline
- **wasm-pkg:** Hard-fail the build if the artifact carries no SIMD opcodes
- **wasm-pkg:** Append to RUSTFLAGS instead of overwriting it
- **release:** Enforce cross-registry version coherence and complete the publish list
- **release:** Pin the git-cliff version in the changelog target
- **release:** Make set-version.py rewrite every version location
- **release:** Gate internal dependency pins, at commit AND publish time

### Documentation

- **deps:** Correct memchr comment; drop issue refs from sparql-algebra
- **wasm-pkg:** Align the SIMD Node floor with the package engine (18)
- **wasm-pkg:** Describe the parse bench as report-only, not a regression gate
- **sparql-eval:** Correct fork_for_worker doc to the portable-row merge mechanism
- Describe behavior, not process, in content-addressing comments
- **rdf:** Document trust-on-first-use semantics of verify_content_chain
- **xsd:** Describe the i128/scale bound instead of a "deferred enhancement"
- Scrub GitHub issue-number references from shapes and sparql-eval comments
- **shex:** Clarify IMPORT conflict and inert-extension doctrine
- **conformance:** Reconcile rdflib LSP gate ledger scoreboard to live 62/24
- **iri:** Harden conformance-vector provenance for W3C IRI gate
- **conformance:** Finalize unified matrix at full-corpus SPARQL numbers
- **sparql-conformance:** Frame SPARQL 1.2 as a complete first-class spec
- **sparql-conformance:** Document that entailment simple1-8 are OWL-Direct, not simple-entailment
- **conformance:** Refresh SPARQL matrix counts to live harness (614 pass / 36 xfail)
- **conformance:** Correct SPARQL 1.2 provenance to zero ledgered residuals
- **sparql:** Finalize W3C SPARQL 1.1 syntax-suite provenance
- **conformance:** Reconcile the ledger and drift-guard the published matrix
- **conformance:** Distinguish normative SHACL-AF node expressions from owned extensions
- **conformance:** Correct stale SHACL first-party corpus count (64 to 69)
- **entail:** Fix misleading comment in the RIF emit path
- **validate:** Fix stale intra-doc link to a non-existent locate module
- **release:** Document MSRV and pre-1.0 semver policy
- **release:** Docs.rs metadata, front-page example, and workspace doc gate
- **ci:** Reconcile doc-target crate count to 16 (15 publishable + purrdf-entail)
- **rdf,shapes:** Fix private/broken intra-doc links failing the doc gate
- **conformance:** Regenerate matrix block for the added SPARQL fixtures
- **release:** Changelog for 0.3.0 and correct the published-crate count

### Features

- **rdf:** Memchr the parallel chunker newline split
- **rdf:** Scan-first serializer escape fast path
- **21:** Memchr/SWAR scan sweep — codec scans + IRI char-class LUT
- **sparql-eval:** Add rayon dep and deterministic two-phase parallel scaffold
- **rdf-core:** Add Blake3ContentId newtype with shared hex decode
- **rdf-core:** Add caller-supplied content-addressing config surface
- **rdf-core:** Recognize content-addressed IRIs at intern time
- **rdf-core:** Carry the content-id side table into the frozen dataset
- **rdf-core:** Add suppression-target and derivation-link traversal helpers
- **rdf-core:** Add a derived predecessor index over derivation annotations
- **rdf:** Add verify_content_chain GTS bridge over content-addressed terms
- **rdf-core:** Content-addressed term support in the IR (GTS-aligned)
- **xsd:** Add opt-in XSD-1.0 float/double lexical restriction
- **sparql-eval:** Pin xsd:float/double cast to XSD 1.0 lexicals
- **xsd:** Shared XSD-1.0 float/double lexical restriction
- **shex:** Resolve transitive cycle-tolerant IMPORT
- **shex:** Dispatch semantic actions via a Test extension registry
- **shex:** Query shape maps with FOCUS triple-pattern selectors
- **shex:** Serialize result shape maps to deterministic JSON
- **shex:** Populate SemActContext value and predicate per matched triple
- **shex:** Add validate_shape_map end-to-end entry point
- **sparql-conformance:** Harden conformance gate — no silent skips + license hygiene
- **sparql-conformance:** Support W3C UpdateEvaluationTest cases
- **sparql-eval:** LATERAL evaluation seam for variable-endpoint and nested SERVICE
- **entail:** Native wasm-clean RDFS + OWL-RL materialization reasoner
- **sparql-conformance:** Wire entailment regime into conformance harness
- **sparql:** Implement RDF-1.2 base-direction functions
- **sparql:** Parse RDF 1.2 triple terms, reifiers, and annotation blocks
- **sparql:** Complete RDF 1.2 triple-term/reifier support across parser, codec, and evaluator
- **sparql-eval:** Evaluate negated-inverse and set-repetition property paths
- **sparql:** Group-by projection check, EXISTS graph scope, GRAPH ?g over empty graphs
- **rdf:** Reifier-consistent CONSTRUCT/UPDATE emission and triple-term equality
- **rdf:** Give the RDF 1.2 reifier/annotation model a graph dimension
- **conformance:** Full W3C SPARQL 1.1/1.2 eval + native entailment + lateral SERVICE
- **shex:** Imports, Test-extension semantic actions, query shape maps
- **sparql:** Vendor W3C SPARQL 1.1 syntax-query suite
- **sparql:** Vendor W3C SPARQL 1.1 syntax-update-1/2 suites
- **sparql:** Vendor W3C SPARQL 1.1 syntax-fed conformance suite
- **sparql:** Vendor W3C SPARQL 1.1 syntax suite as parser conformance fixtures
- **conformance:** Enforce a monotone-shrink ledger ratchet in the gate
- **conformance:** SHA-256 byte-freeze the vendored conformance corpora
- **conformance:** Monotone ledger ratchet, drift-proof published matrix, and byte-freeze verification for the SHACL/shexTest gates
- **shacl-af:** Add node-expression IR skeleton and AF vocabulary
- **shacl-af:** Parse node expressions from the shapes graph
- **shacl-af:** Wire sh:ExpressionConstraintComponent end-to-end
- **shacl-af:** Evaluate built-in function-call node expressions
- **shacl-af:** Evaluate aggregation, paging, and ordering node expressions
- **shacl-af:** Evaluate filterShape and exists with cycle-safe re-entry
- **shacl-af:** Wire vectors/shacl/af seam and refresh conformance matrix
- **shacl-af:** Authority-grounded sh:orderby with sort-key expr and sh:desc
- **shacl-af:** Dispatch XPath-namespace keyword builtins in function calls
- **python:** Complete rdflib plugin entry-point discovery and acceptance matrix
- **shacl-af:** Sh:expression node constraints + node-expression evaluator
- **sparql-eval:** Dynamic SHACL-AF SPARQL function registry seam
- **shapes:** Parse sh:SPARQLFunction declarations into a function registry
- **shapes:** Resolve sh:SPARQLFunction calls in validation, remove the stub
- **shapes:** Implement SHACL-SPARQL custom constraint components
- **shacl-sparql:** Pre-binding substitution semantics and shapes-graph variables
- **shacl-af:** Sh:SPARQLFunction user-defined SPARQL functions
- **shapes:** Complete SHACL-AF validation coverage
- **core:** Expose shared FastHasher/FastMap/FastSet + smallvec primitives
- **shapes:** Id-native SHACL engine over interned TermIds
- **sparql-algebra:** Zero-copy lexer tokens borrowing the source
- **entail:** Bare-RDF axiomatic predicate-typing entailment (rdf01)
- **entail:** SHOIQ(D) OWL-Direct tableau reasoner core (concept, parser, tableau)
- **entail:** Query-directed OWL-Direct DL materialization clears 25 conformance cases
- **entail:** RIF-Core rule engine clears rif01/03/04/06 (zero entailment xfails)
- **entail:** Native OWL-DL tableau + RIF-Core engine + bare-RDF axiomatic entailment
- Close public-maturity epic
- **iri:** Add shared source-position primitive (LineIndex/Position)
- **diagnostics:** Resolve lexer byte offsets to line/column
- **codec:** Attach line/column locations to RDF text parse errors
- **codec:** Opt-in triple->source span table
- **validate:** Scaffold purrdf-validate SARIF boundary crate
- **validate:** Hand-rolled deterministic SARIF 2.1.0 model
- **validate:** Map reports and diagnostics to SARIF results
- **validate:** Source-traced SARIF physical and logical locations
- **validate:** To_sarif surface + Python binding + schema validation
- **bindings:** SARIF surfaces for WASM and C-ABI
- **validate:** SARIF rule metadata with W3C SHACL help links
- **validate:** SARIF 2.1.0 source-traced reporting
- **perf:** Id-native SHACL engine, zero-copy lexer tokens, workspace small-vec + hasher sweep
- **rdf-core:** Graph-scoped rdf:first/rest/nil + container traversal on DatasetView
- **slice:** Graph-scoped nav cursor, RDF-1.2 triple-term interiors, list/container materializer
- **release:** Generate the changelog and GitHub Release notes with git-cliff
- **release:** Single-command version bump and coherent tag cut
- **hygiene:** Extend issue-ref lint to workflow yaml and python comments
- **python:** Complete rdflib drop-in epic
- **release:** Docs.rs polish, MSRV/semver docs, version-coherence gate + changelog
- **rdf-query:** Graph-scoped nav cursor, list/container materializer, one-path blank-safe merge, FILTER/UNION regressions

### Other

- **iri,rdf:** Rustfmt the LUT/escape hot paths
- Remove old docs
- **rdf:** Fmt import order in ser_model tests
- Ignore more
- **sparql-eval:** Apply cargo fmt to expr.rs
- **shapes:** Apply cargo fmt to the pinned-lexical test
- Integrate main (parallel eval, content-addressed terms, XSD-1.0 float/double) into the W3C conformance branch
- Strip stale gmeow-ontology issue refs from Rust comments + lint against regression
- **conformance:** Integrate origin/main; fix issue-ref lint to scan only tracked source
- **shacl-af:** Rustfmt wrapping and register new corpus fixtures
- Rustfmt normalization across the SARIF work

### Performance

- **iri:** Const char-class LUT for ASCII validation
- **sparql:** Byte-cursor + memchr tokenizer
- **rdf:** Hex-LUT UCHAR escape, drop write! from the hot path
- **rdf:** Borrow clean input in escape_scan via Cow
- **sparql-eval:** Memoize constant expression atoms per query
- **sparql-eval:** Memoize dataset literal XSD parses per query
- **sparql-eval:** Allocation-free single-column hash-join keys + pre-sized build map
- **sparql-eval:** O(1) visited-set for property-path transitive closure
- **sparql-eval:** Hoist loop-invariant BGP probe permutation selection
- **sparql-eval:** Pre-parse quoted-triple ORDER BY sort keys
- **sparql-eval:** Single-threaded optimization backlog (items 3–7 + ORDER BY triple residual)
- **wasm-pkg:** Add Node parse-throughput benchmark
- **wasm-pkg:** Build the npm artifact with +simd128
- **sparql-eval:** Parallelize BGP inner loop and read-only join probes
- **sparql-eval:** Parallelize FILTER/filtered-left-join with forked per-worker contexts
- **sparql-eval:** Parallelize UNION, BIND, and per-group aggregates with deterministic scratch merge
- **sparql-eval:** Chunk-based parallel collects to cut per-row allocation
- **sparql-eval:** Reintern minted rows by value to drop the per-cell TermValue clone
- **sparql-eval:** Deterministic parallel evaluation (UNION, joins, BGP, FILTER, aggregates)
- **rdf-core:** Add report-only bench for intern-time content-id overhead
- **rdf-core:** Hash predecessor_chain visited set with ahash
- **entail:** Genuine semi-naive delta chase with new-vertex reflexive derivation
- **sparql:** Reuse blank-label set across update-operation iterations
- **shacl-af:** Hoist recursion guard, reuse intersection set, cache sort keys
- **sparql-eval,shapes:** Adopt small-vectors for hot per-row/per-node collections
- **entail:** Reuse frontier buffers in the RDFS chase loop
- **entail:** Reuse frontier buffers in the RIF chase loop
- **shapes:** Pre-resolve rdf:type id once per Class constraint
- **shapes:** Carry id-native value nodes through the constraint layer
- **shapes:** Adopt fixed-key ahash for the remaining membership sets
- **shapes:** Cache report sort key and borrow the sparql dataset
- **rdf:** Validate UTF-8 lazily in the text-format span-tracking arm
- **rdf-core:** Resolve container type once in is_typed_container

### Refactor

- **sparql-eval:** Rc→Arc on SolutionSeq/ExistsInner for Send+Sync
- **sparql-eval:** Make EvalCtx Send+Sync (Arc caches, RwLock order cache, Sync remote)
- **rdf:** Reuse purrdf_gts::wire::hex in the verify bridge
- **shex:** Route datatype checks through parse_xsd10
- **shapes:** Fold double/float lexical check into purrdf-xsd
- **entail:** Split reasoner into vocab/interner/rdfs modules + Regime::Rif scaffolding
- **shapes:** Collapse the non-interned path walker to reflexive inclusion
- **validate:** Hoist shared validate-to-SARIF helper into purrdf-validate
- **rdf:** Centralize native-codec format dispatch behind an RdfCodec trait

### Testing

- **rdf:** Bench the serializer escape boundary path
- **iri:** First parse criterion bench (dev-dep only)
- **sparql-eval:** Forced-parallel byte-identity determinism gate over a query corpus
- **rdf:** Prove content addressing does not perturb serialized bytes
- **shapes:** Cover xsd:float +INF at the SHACL layer; clarify pinned accept-set test
- **shex:** Drop Turtle-parser workaround in validation conformance harness
- **rdf:** Lock leading-empty rejection in nested callers and drain-to-pipe run
- **shex:** Empty the validation trait-skip list and assert zero skips
- **shex:** Lock Import and SemanticAction trait coverage with exact counts
- **sparql-conformance:** Vendor full W3C SPARQL 1.1 QUERY suite with typed non-pass ledger
- **sparql-conformance:** Vendor W3C SPARQL 1.1 UPDATE evaluation suite
- **sparql-conformance:** Vendor W3C SPARQL 1.2 DRAFT suite + classify
- **rdflib-gate:** Ledger test_group_by — purrdf stricter than rdflib on GROUP BY projection
- **sparql:** Cover blank-node reuse inside RDF-1.2 quoted triples
- **shacl-af:** End-to-end goldens for sh:min/max/distinct/offset
- **shapes:** First-party sh:SPARQLFunction conformance corpus cases
- **shacl-af:** Negative-path coverage for sh:SPARQLFunction
- **bench:** Add SHACL pattern-lookup and value-token lexer micro-benches
- **entail:** Lock in deterministic inferred-triple emission order
- **validate:** Cover attribution UnitId -> slice IRI resolution (S0.5)
- **rdf:** Lock parallel line numbering for a newline-less final line
- **validate:** Lock SHACL helpUri anchors against the live spec format
- **validate:** Avoid a literal #N token in the S0.5 test
- **sparql:** Regression-cover FILTER-NOT-EXISTS arithmetic and all-FILTER UNION branch

## [0.2.1] - 2026-07-02

### Benchmarks

- **python:** Published rdflib-vs-shim benchmark harness + docs

### Bug Fixes

- **python/compat:** Reject bare-form xsd duration instead of zeroing it
- **python/compat:** Honor base/initBindings/native kwargs in SPARQL processors
- **python/compat:** Unwrap Resource-typed predicate and index arguments
- **python:** Sample the host wall clock for NOW and RAND/UUID
- **capi:** Sample the host wall clock for NOW and RAND/UUID
- **sparql:** NOW is the wall clock and RAND is real entropy — by default, everywhere
- **sparql:** Thread standpoint + order cache into the UPDATE WHERE context
- **sparql:** NOW/RAND sample the host wall clock, not the epoch

### CI & Build

- **python:** Run the acceptance matrix under the acceptance dep group

### Documentation

- **python/compat:** Strip in-repo tracker references from the compat shim

### Features

- **python:** Python test harness + xfail-ledger gate for the rdflib drop-in
- **python:** Top-level engine exports mirroring the Rust umbrella crate
- **python:** Term-model completeness — value coercion, RDF 1.2 direction, from_n3
- **python:** NamespaceManager + Namespace parity with rdflib
- **python:** Graph/Dataset facade parity with rdflib
- **python:** Rdflib plugin registry + entry-point discovery
- **python:** SPARQL result serialization, native substitutions, property paths
- **python:** Opt-in top-level `rdflib` shadow (import rdflib -> purrdf)
- **build:** Single conformance matrix — native W3C suites + rdflib gate
- **xsd:** Native binary decode + whitespace facets for the compat value map
- **rdf:** Native TriX and HexTuples codecs for the compat plugin registry
- **sparql-eval:** Add wasm-clean QueryEnv seam for NOW/RNG injection
- **11:** Purrdf as an RDF-1.2-first drop-in rdflib replacement

### Other

- **python:** Rustfmt the native SPARQL-results + term-direction bindings
- Release 0.2.1: rdflib drop-in hardening — native xsd coercion + TriX/HexTuples codecs

### Testing

- **python:** Gate — rdflib's own test suite against the compat shim
- **python:** Downstream acceptance matrix (pyshacl / SPARQLWrapper / sssom)

## [0.2.0] - 2026-07-02

### Other

- Release 0.2.0: complete umbrella facade, OntologyProfile, ShExC serializer, drop openEHR OPT

## [0.1.5] - 2026-07-02

### Bug Fixes

- Fix release lanes: wasm-opt post-MVP features, workspace-inherited version

### Documentation

- Add the PurRDF DOI (10.67342/pkg8gpp4no/v1) to CITATION.cff and the README badge row

### Other

- Package README + metadata, js package 0.1.4
- Shex in flight
- Full ShEx 2.1 + complete SHACL Core + de-gmeow the library namespaces
- Purge the invented namespace: purrdf is a toolkit, not an ontology
- Parallel parse + parallel GTS verification (deterministic by construction)
- Python bindings: shex module, engine configuration, GIL release
- Release 0.1.5: full SHACL/ShEx, de-gmeow'd namespaces, SPARQL eval speedups

## [0.1.3] - 2026-07-02

### Bug Fixes

- Include Python sdist toolchain
- Build PyPI manylinux wheels

### Other

- Parameterize jsonld prefix
- Release 0.1.3: brand, first-class docs, strict lints, perf, npm lane
- Stabilize the toolchain: stable-Rust-clean workspace, real MSRV

## [0.1.1] - 2026-07-01

### Bug Fixes

- Set crates.io release user agent
- Pace crates.io bootstrap publishes
- Set crates.io workflow user agent
- Make purrdf the umbrella crate
- Pace only new crate publishes

### Other

- First commit


