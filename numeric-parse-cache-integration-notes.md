# Numeric parse/cache draft integration

The sibling `numeric-parse-cache-draft.patch` is proposed code only. Production source is untouched. No build, test, Git, forge or patch-application check was run by this support agent. The sole writer must integrate and qualify the complete delivery.

## Supplied unit and final API

Ten proposed source files: bigint.rs, bigint/scratch.rs; exact/error.rs, mod.rs, integer.rs, decimal.rs, cost.rs; numeric.rs, value.rs; and new evaluator parsed_value.rs.

* Existing native bounded readers expose crate-private `numeric::read_integer_typed` and `numeric::read_decimal`. Shared classification is `numeric::NumericReadError::{Invalid(&'static str), OutOfRange(&'static str)}`; owned resident materialization is `error.owned(datatype, lexical)`. There is no DecimalReadError alias. Root's temporal proposal must use NumericReadError and `.owned(XsdDatatype::Decimal, lexical)`.
* Public resident parsers call those same readers and retain their owned diagnostics. Exact fallback avoids first constructing a discarded overflow lexical String.
* The existing exact decimal parser checks original fractional scale before trimming fractional trailing zeros on borrowed slices. It calls the existing bigint decimal-chunk kernel, consumes the sign, and constructs canonical parts after the trimming proof. No concatenated coefficient String, negative magnitude clone or second parser remains.
* Bigint's `Fallible: Allocate` uses `Vec::try_reserve_exact` before heap destination resize/push and inline spill. Existing `mag_mul_small_add` calls `storage.push`; the existing digit-chunk body takes that storage interface. Resident parsing uses the same body with Unbounded.
* `BigInt::try_from_decimal_bytes(digits, negative)`, `exact::Integer::try_from_lexical` and `exact::Decimal::try_from_lexical` provide fallible physical destinations. Allocation refusal is LimbScratchError::AllocationFailed, never invalid lexical or unbound.
* New `exact::ExactParseError::{InvalidLexical, ScaleOverflow, Storage}` separates value-space errors from physical refusal. `code() -> Option<ErrorCode>` returns F&O only for semantic errors. Existing ExactError and its nonoptional code API remain unchanged; no fabricated F&O code labels OOM.
* `exact::cost::numeric_parse_layout(lexical, datatype, xsd10)` returns checked peak/retained capacities and allocation-free invalid classification for the new native entry. Three-limb magnitudes remain inline; larger decimal-chunk conversion accounts for both live requested limb destinations, bounded by significant-digit limbs plus one.
* `value::try_parse_numeric -> Result<ParsedNumeric, LimbScratchError>` returns Value, Invalid(Option<ErrorCode>) or OtherDatatype. Invalid results do not own lexical text; physical errors remain Err. Float/double preserve XSD 1.0/1.1 rules. XsdValue::owned_heap_bytes reads actual capacities exhaustively without rendering.
* Private ParsedValue stores heap-free values inline and heap values in immutable Arc<ParsedPayload>. Shared clones are shallow; the value is dropped before its retained WorkspaceAllocation. No raw extraction or replaceable lease is exposed.
* ParsedCache<K> uses the existing admitted HashTable layout home. Growth admits a replacement before reserve; old and new table charges coexist while entries are shallow-copied. Old table storage is dropped before its charge. Acquired values retain their own admission after cache drop.

The numeric factory charges peak parse storage and its shared header before native parse, checks the native result against its certificate, then settles to actual surviving payload plus header. Zero-heap parse avoids a workspace lease-box allocation. Stored term bytes are not used as a multiplier.

## Mandatory integration

The draft intentionally supplies no lib.rs/EvalCtx/expr.rs migration: this unit cannot replace the full supported dispatcher until other datatype owners are integrated. OtherDatatype requires that native owner; it is no refusal or permission for unpriced fallback.

1. Register `mod parsed_value;`. Change EvalCtx::xsd_parse_cache to ParsedCache<D::Id>; its three constructor/fork assignments become ParsedCache::default(). Forks keep fresh caches.
2. Make expr::xsd_of_term return Result<Option<ParsedValue>, EvalError>. Lookup is `if let Some(cached) = ctx.xsd_parse_cache.get(id) { return Ok(cached); }`; outer Some distinguishes a hit from cached opaque/invalid None.
3. Keep dataset lexical/datatype guards borrowed. Inside them invoke one fallible admitted native dispatcher with lexical, datatype/version and &ctx.growth. Numeric uses ParsedValue::parse_numeric. Unknown datatype/nonliteral remain None; known nonnumeric uses its real native owner and the same carrier.
4. Insert with `ctx.xsd_parse_cache.insert_admitted(id, parsed.clone(), &ctx.growth)?;`. Computed scratch literals borrow their existing text and use the same dispatcher.
5. Source callbacks now contain an EvalError leaf inside source Results. Before source_read allocates diagnostic text, consult the writer's sticky-failure helper. Preserve first WorkspaceStopped and its backend cause, then flatten the leaf unchanged. No .ok().flatten().
6. Preserve Invalid(code) until a cast's established F&O branch consumes it; EBV/comparison cache may keep its former invalid None.
7. Root owns separate temporal/binary/nonnumeric proposals. The final borrowed temporal dependency is NumericReadError as above, not DecimalReadError.

## Actual remaining owner work

Known nonnumeric datatypes remain supported and required:

* Boolean: existing borrowed classification, inline result, no owned diagnostic for operational invalid values.
* String: actual copied lexical buffer admitted before allocation and retained with carrier.
* Hex/base64: root's fallible native destination/layout proposal. Count whitespace-filter and decoded buffers when both coexist; growing collect cannot remain unpriced.
* Temporal/Gregorian/duration: root's one-grammar borrowed dispatcher including timezone/seconds/body and deferred static errors. Inline final values do not cover previous parser temporaries.
* XSD 1.0: plan and parse the same version. +INF is invalid there and must not run an owned-error parser under a valid zero-heap plan.

Arc::new(ParsedPayload) has its header priced but follows the current shared-owner convention; it does **not** make physical header allocation fallible. Wire the complete delivery's actual fallible shared-header factory here if typed allocator refusal is required for that header too. Precharging is not proof of physical refusal. This patch resolves the native limb Vec destination seam, not every shared-header/workspace-provider allocation seam.

## Production caller map

| Caller | Required migration |
|---|---|
| ebv_term | `parsed.as_deref().and_then(effective_boolean_value)`. |
| equal_terms | Match ax.as_deref(), bx.as_deref(); preserve NaN/type/language laws. |
| compare_terms | Hold both carriers through comparison and admitted promotion; &carrier dereferences to &XsdValue. |
| term_holds_nan | `as_deref().is_some_and(is_xsd_nan)`. |
| arithmetic_step, unary_numeric_term | Borrow carriers through native calls; operation/promotion/render admission remains required separately. |
| unary_numeric_fn, IsNumeric, casts | Replace operational xsd_of(TermValue) with the admitted dispatcher; keep carriers local; borrow filters with as_deref(); preserve Invalid code. |
| value_cast, numeric_xsd, lexical parse_xsd10 | Thread that same native admitted dispatcher; owned input bytes do not cover new parse work. |
| in_candidate, rdf_equal_in, value_holds_nan, nested triples | Pass immutable capability through the same semantic body and propagate Result. Mutable EvalCtx is needed only for cache insertion. |
| modifier::project_shallow, ValueClass, SortKey | Hold carriers in keys, or admit an explicit new key copy before building it. Dropped cache admission cannot cover surviving raw magnitudes. |
| Aggregate xsd_of / NumericScratch::push | Hold carriers through folds or store them in scratch. Keep SUM/AVG left-fold law; kernels and render keep separate actual owners. |
| Prepare-time constant_ebv | Actual bounded preparation requires admission; resident-only calls may stay resident after call-path verification. |
| Existing xsd_of_term tests | Inspect borrowed XsdValue; no raw into_inner. Preserve opaque/invalid None and source-error expectations. |

These dispatcher/caller changes remain blocked acceptance until implemented, not waived scope.

## Meaningful qualification, not run here

* Large numeric parse occurs once; cache hits share payload. Acquired carriers survive cache/context drop with live admission until the last owner drops.
* Primitive, zero and long leading-zero values keep actual inline behavior without a lease box.
* Workspace refusal precedes native magnitude allocation; enough budget returns resident parity. Invalid lexical and XSD-1.0 +INF remain semantic failures.
* Inject physical destination refusal at existing Allocate seam: ExactParseError::Storage/native Err propagates operationally and never becomes Invalid, None or an owned diagnostic.
* Replacement-table refusal precedes allocation and preserves old cache ownership; successful replacement never deep-clones magnitudes.
* Decimal parity covers signs/i128::MIN, leading/trailing/no dot, zero trimming, >38 digits, large scale with coefficient one, malformed lexical and original-scale overflow. Existing conformance is still required.

These are complete-delivery acceptance obligations. No executed qualification is claimed.
