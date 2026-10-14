/* tslint:disable */
/* eslint-disable */

/**
 * One effect the host has taken, as JavaScript sees it.
 */
export class AsyncEffect {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * `SERVICE` and `LOAD`: the extra request headers as flattened `[name, value, name,
     * value, …]` pairs — the catalog profile's headers, then its credential header — in
     * the order they must be sent. Append each pair; a repeated name is legal and must
     * not be merged. Empty when no catalog is configured.
     */
    headers(): string[];
    /**
     * When the host abandons the effect if it is still unanswered, in milliseconds from
     * now: the request's own timeout or the job's remaining deadline, whichever is
     * sooner. The host calls [`AsyncJob::expire_effect`] when it passes. `undefined` for
     * a yield.
     */
    readonly abandonAfterMs: number | undefined;
    /**
     * `SERVICE`: the `Accept` header value (`application/sparql-results+json`); `LOAD`:
     * the media type of every RDF syntax a `LOAD` parses.
     */
    readonly accept: string | undefined;
    /**
     * `SERVICE`: whether a shared cache may answer the request or keep its answer —
     * `false` when it carries a credential, whose answer belongs to the credential's
     * holder.
     */
    readonly cacheable: boolean;
    /**
     * `SERVICE`: the `Content-Type` header value (`application/sparql-query`).
     */
    readonly contentType: string | undefined;
    /**
     * `SERVICE`: the endpoint IRI exactly as the query wrote it.
     */
    readonly endpoint: string | undefined;
    /**
     * `SERVICE` and a wait on an exchange: the shared exchange the answer is delivered
     * to.
     */
    readonly exchangeId: bigint | undefined;
    /**
     * `LOAD`: the IRI of this hop — the source, or the location a redirect named.
     */
    readonly iri: string | undefined;
    /**
     * What the effect asks for.
     */
    readonly kind: EffectKind;
    /**
     * `SERVICE`: the query's inclusive intermediate-cell ceiling, when the caller
     * actually configured one. `undefined` on an ungoverned query and on a governed one
     * that set no cell ceiling — never the metering bookkeeping sentinel a caller never
     * asked for.
     */
    readonly maxIntermediateCells: bigint | undefined;
    /**
     * `SERVICE`: the complete forwarded `SELECT * WHERE { … }` text — the request body.
     */
    readonly queryText: string | undefined;
    /**
     * `SERVICE`: milliseconds left before the job's deadline, when one is set.
     */
    readonly remainingDeadlineMs: number | undefined;
    /**
     * The sequence number every delivery for this effect must name.
     */
    readonly seq: number;
    /**
     * `SERVICE`: whether the clause was written `SERVICE SILENT`. Informational only:
     * an empty answer is never the host's to invent because of it.
     */
    readonly silent: boolean;
    /**
     * `SERVICE` and `LOAD`: the per-request timeout the catalog profile names, or the
     * default, in milliseconds.
     */
    readonly timeoutMs: number | undefined;
    /**
     * `SERVICE`: the `User-Agent` the service's profile names, or the engine default;
     * `LOAD`: the one the source's catalog profile names, if it names one.
     */
    readonly userAgent: string | undefined;
}

/**
 * What one asynchronous job cost: counts and phase times, all measured in Rust.
 *
 * `evaluateMs` is the wall time of the evaluation phase and so *includes* the waits the
 * three `…WaitMs` fields report separately; `freezeMs` and `serializeMs` are the phases
 * that run to completion without yielding.
 */
export class AsyncEvidence {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Milliseconds of the evaluation phase, waits included.
     */
    readonly evaluateMs: number;
    /**
     * Every XPath F&O numeric error the job's evaluations absorbed into an unbound
     * value, summed per code and positional by `expressionErrorCodes()`. All zero when
     * no expression failed. A number, as every other count of the job's evidence is, so
     * the job's evidence stays JSON-serializable.
     */
    readonly expressionErrors: Float64Array;
    /**
     * Milliseconds freezing the dataset snapshot before the job started.
     */
    readonly freezeMs: number;
    /**
     * `LOAD` effects the host answered or abandoned.
     */
    readonly loadEffects: number;
    /**
     * Milliseconds spent suspended on `LOAD` effects.
     */
    readonly loadWaitMs: number;
    /**
     * Stop-signal polls the evaluator made.
     */
    readonly polls: number;
    /**
     * Milliseconds serializing a raw result.
     */
    readonly serializeMs: number;
    /**
     * `SERVICE` effects — requests the job issued, and waits on another job's
     * identical request — the host answered or abandoned. A request the job repeats is
     * answered from its own memo and issues no effect.
     */
    readonly serviceEffects: number;
    /**
     * Milliseconds spent suspended on `SERVICE` effects.
     */
    readonly serviceWaitMs: number;
    /**
     * Every invocation a `SERVICE SILENT` or `LOAD SILENT` absorbed during the job, in
     * the order its evaluations recorded them. Empty when nothing failed.
     */
    readonly silenced: SilencedInvocation[];
    /**
     * The deepest the job's stack reached at a poll, in bytes below its region's top.
     * `0` when the job never ran on a region (the native build).
     */
    readonly stackHighWaterBytes: number;
    /**
     * Milliseconds spent suspended on yields.
     */
    readonly yieldWaitMs: number;
    /**
     * Times the job gave the event loop back.
     */
    readonly yields: number;
}

/**
 * One asynchronous operation, driven by the package root's JSPI scheduler.
 *
 * Every method is safe to call while the job's frames are suspended, and none of them
 * throws for a protocol condition: deliveries return a status and latch a fault instead.
 * The `take*` methods and `commitUpdate` are ordinary synchronous calls made after the
 * job has finished; they throw for a caller bug (the wrong kind, a job not yet finished)
 * and for the job's own error.
 *
 * A job stays registered — and its stack region allocated — until [`Self::finish`] is
 * called, whether or not this handle has been freed: the host calls it once the run
 * has settled.
 */
export class AsyncJob {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Start an asynchronous SHACL operation: run `request` — built by the
     * `ShaclJobRequest` constructor named for the synchronous entry it twins, over
     * exactly that entry's positional arguments — under `options` (from
     * `AsyncJobOptions.fromJs` for the `shacl` kind). The job runs the synchronous
     * entry's own body, with the dated XPath law `options` names (`xpathRegex`) as the
     * trailing argument the synchronous entry takes it as.
     *
     * The documents are parsed when the job runs, so a parse error — like a product or
     * import refusal — is the job's error, with the synchronous twin's words.
     *
     * A static constructor rather than a free function: it is the package root's
     * plumbing, reached through its `shacl…Async` twins, never a consumer entry point.
     *
     * # Errors
     *
     * Options validated for another kind.
     */
    static beginShacl(options: AsyncJobOptions, request: ShaclJobRequest): AsyncJob;
    /**
     * Cancel the job. It observes the cancellation at its next poll, yield or effect.
     */
    cancel(): DeliveryStatus;
    /**
     * Apply a finished update's result to `dataset` — refused, with the dataset left
     * untouched, when `dataset` is not the one the update read or has been mutated since
     * it started.
     */
    commitUpdate(dataset: Dataset): void;
    /**
     * Answer every job waiting on shared exchange `exchange` with SPARQL Results JSON
     * bytes, and close the exchange. `Finished` when no such exchange is open — its
     * last waiter left, or it was already answered.
     */
    static deliverExchangeBindings(exchange: bigint, bytes: Uint8Array): DeliveryStatus;
    /**
     * Fail every job waiting on shared exchange `exchange`: `kind` is `"transport"`,
     * `"denied"` or `"fault"`; any other kind is delivered as a fault naming it. Each
     * waiting job's effect fails as its own invocation's failure. Closes the exchange.
     */
    static deliverExchangeFailure(exchange: bigint, kind: string, message: string): DeliveryStatus;
    /**
     * Fail effect `seq`: `kind` is `"transport"` (unreachable, or unreadable), `"denied"`
     * (the host's policy refused it) or `"fault"` (the host's handler threw, rejected or
     * returned something the protocol does not define). Any other kind is delivered as a
     * fault naming it. Each is the effect's own failure, answered by the clause that
     * issued it; none is the job's fault.
     */
    deliverFailure(seq: number, kind: string, message: string): DeliveryStatus;
    /**
     * Record that the host abandoned effect `seq` on the job's stop signal. When that
     * signal is not already latched, this cancels the job. A job waiting on a shared
     * exchange leaves it; see [`Self::exchange_is_open`].
     */
    deliverGoverned(seq: number): DeliveryStatus;
    /**
     * Answer `LOAD` effect `seq` with a document. It is parsed here, by media type (or
     * any format name `Dataset.parse` accepts); a document that cannot be parsed is the
     * `LOAD`'s decode failure, never a fault.
     */
    deliverGraph(seq: number, bytes: Uint8Array, media_type: string, base?: string | null): DeliveryStatus;
    /**
     * Answer `LOAD` effect `seq` with a snapshot of an existing dataset.
     */
    deliverGraphDataset(seq: number, dataset: Dataset): DeliveryStatus;
    /**
     * Answer `LOAD` effect `seq` with a redirect to `location`, the response's
     * `Location` as sent. The job resolves it against the effect's IRI, authorizes the
     * target against its catalog and fetches it as a fresh effect, up to its redirect
     * limit.
     */
    deliverRedirect(seq: number, location: string): DeliveryStatus;
    /**
     * Whether shared exchange `exchange` is still open: some job waits on its answer.
     * Once it closes without an answer, the host aborts its call.
     */
    static exchangeIsOpen(exchange: bigint): boolean;
    /**
     * Effect `seq`'s abandonment instant ([`AsyncEffect::abandon_after_ms`]) has
     * passed with no answer. When it was the job's deadline, the deadline is latched and
     * the effect abandoned (`Abandoned`); when it was the request's own timeout, the
     * effect fails as a transport failure the `SILENT` forms absorb (`Answered`). A job
     * waiting on a shared exchange leaves it. An effect already answered is left as it
     * is (`Answered`), and a fault is reported as one (`Fault`).
     */
    expireEffect(seq: number): SuspendStatus;
    /**
     * Latch `message` as the job's fault: a host bug the job cannot continue past. The
     * job's signal fires, it winds down, and the fault becomes its error.
     */
    fault(message: string): DeliveryStatus;
    /**
     * Fail every job waiting on shared exchange `exchange` with the host handler's fault
     * `message` — it threw, rejected or returned something the protocol does not define
     * — and close the exchange. Each waiter's effect fails as its own invocation's
     * failure; no job's fault is latched.
     */
    static faultExchange(exchange: bigint, message: string): DeliveryStatus;
    /**
     * Remove the job from the registry once it has finished, and release an update's
     * claim on its dataset: returns 0, or 1 (and does nothing) while it is still
     * running, because the run is standing on the job's region.
     */
    finish(): number;
    /**
     * The status to resume effect `seq` with once the shared exchange it waited on has
     * settled: `Answered` when the exchange's answer was delivered to it, `Fault` when a
     * fault is latched (a settled exchange that delivered nothing to a job still waiting
     * on it latches one).
     */
    settledStatus(seq: number): SuspendStatus;
    /**
     * The effect the job is suspended on, once; `undefined` when there is none.
     */
    takeEffect(): AsyncEffect | undefined;
    /**
     * A governed entailment job's outcome.
     */
    takeEntailmentOutcome(): EntailmentQueryOutcome;
    /**
     * A snapshot of the job's evidence. Callable at any time, as often as wanted.
     */
    takeEvidence(): AsyncEvidence;
    /**
     * A negotiated query job's outcome: the serialized complete answer, or the trip.
     */
    takeNegotiatedOutcome(): NegotiatedOutcome;
    /**
     * A governed query job's outcome.
     */
    takeQueryOutcome(): QueryOutcome;
    /**
     * A `query` job's typed result. `expect` — `"select"`, `"ask"`, `"construct"`,
     * `"describe"` (or `"graph"`) — refuses any other result kind with the synchronous
     * twin's message; `undefined` accepts any.
     */
    takeQueryResult(expect?: string | null): QueryResult;
    /**
     * A raw job's serialized bytes, an explain job's rendered ledger, or a SHACL job's
     * SARIF log or entailed N-Triples (UTF-8 text).
     */
    takeRawBytes(): Uint8Array;
    /**
     * A SHACL change-validation job's outcome: the SARIF log and the scope it
     * describes, exactly as `shaclValidateChangesToSarif` returns them.
     */
    takeShaclChangeValidation(): ShaclChangeValidation;
    /**
     * A SHACL-AF entailment job's outcome: the materialized dataset and its
     * diagnostics, exactly as `shaclEntail` returns them.
     */
    takeShaclEntailment(): ShaclEntailment;
    /**
     * A shapes-graph job's import refusal, once — the `ShaclImportError` the synchronous
     * twin throws — or `undefined` when the job was not refused that way.
     */
    takeShaclImportError(): ShaclImportError | undefined;
    /**
     * A node-expression job's `ShaclNodeExprOutcome` — its output nodes and the shapes
     * graph's mandatory diagnostics — exactly as `shaclEvalNodeExpr` returns it.
     */
    takeShaclNodeExprOutcome(): ShaclNodeExprOutcome;
    /**
     * A SHACL product job's refusal, once — the `ShaclProductRefusal` the synchronous
     * twin throws — or `undefined` when the job was not refused that way.
     */
    takeShaclRefusal(): ShaclProductRefusal | undefined;
    /**
     * A rules job's outcome: the inference graph, proof and diagnostics, exactly as
     * `shaclApplyRules` returns them.
     */
    takeShaclRulesInference(): ShaclRulesInference;
    /**
     * A governed update job's outcome. Whether it applied is on the outcome; applying it
     * to the dataset is [`Self::commit_update`].
     */
    takeUpdateOutcome(): UpdateOutcome;
    /**
     * The stable code the job's failure is reported under — an engine diagnostic's own
     * code, or the code of a stop, a fault or a refusal — or `undefined` when it did not
     * fail (or has not finished).
     */
    readonly errorCode: string | undefined;
    /**
     * How the job failed — `"error"`, `"cancelled"`, `"deadline"`, `"fault"` or
     * `"not-acceptable"` — or `undefined` when it did not (or has not finished).
     */
    readonly errorKind: string | undefined;
    /**
     * The job's failure in words, or `undefined` when it did not fail (or has not
     * finished).
     */
    readonly errorMessage: string | undefined;
    /**
     * The job's failure's typed presentation as its JSON record — a SPARQL parse
     * failure's `sparql-parse-*` identity and typed parameters — or `undefined` when the
     * failure carries none (or the job did not fail, or has not finished).
     */
    readonly errorPresentation: string | undefined;
    /**
     * The id `purrdf_jspi_run` and `purrdf_jspi_suspend` name this job by.
     */
    readonly id: number;
    /**
     * Whether the run has returned.
     */
    readonly isFinished: boolean;
    /**
     * The operation this job runs.
     */
    readonly kind: AsyncOperationKind;
    /**
     * The region's top: the stack pointer the linked run wrapper installs for the run,
     * passed as the last argument of `purrdf_jspi_run`.
     */
    readonly stackTop: number;
}

/**
 * The configuration of one asynchronous job: an options object read and validated
 * against its operation's `OperationSpec` ([`Self::from_js`]).
 *
 * Every option a kind would ignore is refused by name rather than dropped: an option a
 * caller believes applies and that nothing enforces is the silent hole this surface
 * exists to close. The ceilings are 64-bit integers exactly as the synchronous governed
 * entries take them; `yieldEveryPolls` is refused unless it is an integer in range, so a
 * negative never wraps into a huge value. The job's stack region is not an option: every
 * job runs on a region the size of the module's own shadow stack, so the two lanes run
 * out of stack at the same depth.
 */
export class AsyncJobOptions {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Read and validate the options object of an operation of `kind`.
     *
     * `options` is the twin's options object without the keys the package root reads
     * itself (`resolveService`, `resolveLoad`, `signal`); `serviceHandler` identifies
     * the host's `resolveService` handler when it supplied one (only requests to the
     * same handler share a host call), and `loadHandler` says whether it supplied
     * `resolveLoad`. `argument` is the twin's positional argument — the regime of
     * `entailmentGoverned`, the format of `rawWithContext` — and must be absent for
     * every other kind.
     *
     * # Errors
     *
     * A `TypeError` for a key the operation does not take or a value of the wrong type,
     * and an `Error` for a value it cannot honor; both carry the code
     * `purrdf-wasm-options`.
     */
    static fromJs(kind: AsyncOperationKind, options: any, service_handler: number | null | undefined, load_handler: boolean, argument?: string | null): AsyncJobOptions;
}

/**
 * Which operation a job runs. Each is the asynchronous twin of a synchronous entry.
 */
export enum AsyncOperationKind {
    /**
     * `query`/`select`/`ask`/`construct`/`describe`: a typed result.
     */
    Query = 0,
    /**
     * `queryRaw`/`queryRawConfigured`/`Dataset.query`: serialized bytes.
     */
    Raw = 1,
    /**
     * `queryRawWithContext`: bytes serialized under a compiled JSON-LD context.
     */
    RawWithContext = 2,
    /**
     * `queryGoverned`: a governed outcome.
     */
    Governed = 3,
    /**
     * `queryEntailmentGoverned`: a two-phase governed outcome.
     */
    EntailmentGoverned = 4,
    /**
     * `update`: applied through [`AsyncJob::commit_update`].
     */
    Update = 5,
    /**
     * `updateGoverned`: a governed outcome, applied through [`AsyncJob::commit_update`].
     */
    UpdateGoverned = 6,
    /**
     * `queryGovernedNegotiatedAsync`: a governed outcome whose complete answer is
     * serialized in the format negotiated from an `Accept` header, once the result's
     * shape is known. The asynchronous lane's own: a SPARQL Protocol endpoint's query.
     */
    Negotiated = 7,
    /**
     * `explainQuery`: the rendered charge ledger, taken through
     * [`AsyncJob::take_raw_bytes`]. EXPLAIN evaluates the query to measure it, so its
     * measuring run suspends on `SERVICE`, yields and stops exactly as a query does.
     */
    Explain = 8,
    /**
     * A SHACL entry that can evaluate SPARQL — validation to SARIF, change validation,
     * SHACL-AF entailment, a rules run, a node expression, or validation with a
     * prepared product (`ShaclRequest` names which). Its `sh:SPARQLTarget` queries,
     * SHACL-SPARQL constraints, node expressions and rules run under the job's signal,
     * and the signal is polled between focus nodes too, so a validation with no SPARQL
     * in it still yields and stops. Started through `AsyncJob.beginShacl`.
     */
    Shacl = 9,
}

/**
 * A cancellation bit a page can flip, shared with every governed call it is handed to.
 *
 * Latching is by construction: the bit only ever moves from clear to set, and nothing
 * clears it. Build a fresh token per query rather than reusing one.
 *
 * # Cancelling a *running* wasm call
 *
 * A synchronous governed call holds the thread for its whole duration, so a token
 * flipped on that same thread cannot run until the call returns and is never observed
 * by it. For a synchronous call the token is therefore for the two shapes that
 * genuinely work: cancelling *before* the call (a queued query the user has since
 * navigated away from), and cancelling a worker's query from another thread when the
 * token is shared through a `SharedArrayBuffer`-backed worker split. Both observe the
 * same latching bit, and both report the same `"cancelled"` trip.
 *
 * The asynchronous lane (`crate::async_query`) is different: its jobs give the event
 * loop back at every yield and every host effect, so the page's own code does run
 * mid-query. It is cancelled through the job itself (the package root wires an
 * `AbortSignal` to it) and observes the cancellation at the next yield or effect — a
 * token is neither needed nor accepted there.
 */
export class CancellationToken {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Cancel every governed call holding this token. Idempotent, and never reversible.
     */
    cancel(): void;
    /**
     * A fresh, uncancelled token.
     */
    constructor();
    /**
     * A second handle onto the **same** bit — not a copy of it.
     *
     * Cancelling either handle cancels both, and neither can un-cancel. This exists
     * because a governed call **consumes** the token handle it is given: wasm-bindgen
     * moves an owned exported value across the boundary, which invalidates the JavaScript
     * object. The package root therefore hands the engine a share and keeps the caller's
     * own token alive, so one token governs a whole sequence of calls. The alternative —
     * a token that silently stops working after its first use — is a cancellation the
     * caller believes they still hold.
     */
    share(): CancellationToken;
    /**
     * Whether this token has been cancelled.
     */
    readonly isCancelled: boolean;
}

/**
 * An immutable JSON-LD 1.1 context compiled once and reusable across datasets.
 */
export class CompiledJsonLdContext {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Return the recursively canonical context document as JSON.
     */
    canonicalContextJson(): string;
    /**
     * Compile the context branch of a versioned JSON-LD options document.
     */
    constructor(options_json: string);
}

/**
 * An RDF/JS `DataFactory`. Stateless except for the auto-generated blank-node
 * counter (`blankNode()` with no argument mints a fresh label).
 */
export class DataFactory {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * `blankNode(value?)` → a `BlankNode` term; a fresh label is minted when omitted.
     */
    blankNode(value?: string | null): Term;
    /**
     * `defaultGraph()` → the `DefaultGraph` term.
     */
    defaultGraph(): Term;
    /**
     * `directionalLiteral(value, language, direction)` → an RDF-1.2 base-direction
     * literal (`direction` is `"ltr"` or `"rtl"`). The deliberate extension to stock
     * RDF/JS — no incumbent library carries base direction.
     *
     * # Errors
     *
     * Throws when `direction` is neither `"ltr"` nor `"rtl"`, and — as of the
     * language-tag parity fix — when `language` is not a language tag. A base
     * direction does not make a non-tag into a tag: `"x"@en us--ltr` is as
     * unwritable as `"x"@en us`, so both halves of a directional literal are
     * judged, not just the half that already had a checker.
     */
    directionalLiteral(value: string, language: string, direction: string): Term;
    /**
     * `fromQuad(original)` → a copy of `original`.
     */
    fromQuad(original: Quad): Quad;
    /**
     * `fromTerm(original)` → a copy of `original` (RDF/JS structural clone).
     */
    fromTerm(original: Term): Term;
    /**
     * `literal(value, language?)` → a plain (`xsd:string`) or language-tagged literal,
     * **judged**. This is the JS door: `DataFactory.prototype.literal`.
     *
     * The RDF/JS spec's unified `literal(value, languageOrDatatype)` — where the second
     * argument may be a string *or* a `NamedNode` — is presented by the TypeScript
     * wrapper, which dispatches the `NamedNode` case to [`DataFactory::typed_literal`].
     * (A `#[wasm_bindgen]`-exported type cannot be recovered from an untyped `JsValue`
     * in Rust, so the polymorphism lives one layer out, in JS.) For base-direction
     * literals (RDF 1.2) use [`DataFactory::directional_literal`].
     *
     * Its Rust name is not `literal`, and that is the point: `literal` is the
     * published infallible Rust signature (see [`DataFactory::literal`]), which
     * `purrdf-wasm` shipped in `rust-v2.0.2` and may not change. JS never sees a
     * Rust name — `js_name = literal` is the whole binding — so the refusal can
     * be added to the JS surface, which is where the parity gap was, without
     * touching the Rust surface, which is where the semver guarantee is.
     *
     * # Errors
     *
     * Throws when `language` is not a language tag the RDF concrete syntaxes
     * would have lexed — the same [`RdfLiteral::validate_components`] judgement
     * the C ABI and the Python binding make at their own literal constructors,
     * so all three language bindings admit one set of tags rather than two.
     * Raising it *here* is the whole point: the JS stack trace then names the
     * `literal()` call that chose the tag. Without it the tag rides along until
     * a `Dataset` freeze or a serializer refuses it, at a call site that never
     * saw the string.
     */
    literal(value: string, language?: string | null): Term;
    /**
     * `namedNode(value)` → a `NamedNode` term.
     */
    namedNode(value: string): Term;
    /**
     * `new DataFactory()` — a fresh factory with its blank-node counter at zero.
     *
     * A written-out function rather than the type's `Default`: `#[wasm_bindgen(constructor)]`
     * exports only a function of the `#[wasm_bindgen]` impl block, as `#[new]` does in a
     * `#[pymethods]` block, so every exported constructor has this one-line body.
     */
    constructor();
    /**
     * `quad(subject, predicate, object, graph?)` → a `Quad`. The graph defaults to the
     * default graph. A quoted-triple term (from [`DataFactory::quoted_triple`]) may be
     * passed as `subject` or `object` (the RDF-1.2 wedge).
     */
    quad(subject: Term, predicate: Term, object: Term, graph?: Term | null): Quad;
    /**
     * `quotedTriple(subject, predicate, object)` → a quoted-triple `Term`
     * (`termType: "Quad"`) — the RDF-1.2 wedge. Embed it by passing it as the
     * `subject`/`object` of another quad.
     */
    quotedTriple(subject: Term, predicate: Term, object: Term): Term;
    /**
     * `typedLiteral(value, datatype)` → a datatyped literal. `datatype` must be a
     * `NamedNode`. (The RDF/JS `literal(value, datatype)` form, surfaced by the TS
     * wrapper.)
     */
    typedLiteral(value: string, datatype: Term): Term;
    /**
     * `variable(value)` → a `Variable` term.
     */
    variable(value: string): Term;
}

/**
 * An RDF/JS `DatasetCore` backed by the engine's COW mutable dataset.
 *
 * # Identity and generation
 *
 * Every dataset carries an `id`, unique within the wasm instance and fixed for its
 * lifetime, and a `generation` that advances on every mutation of its content. An
 * asynchronous UPDATE evaluates against a snapshot and commits later; the pair is how
 * that commit proves it is landing on the dataset it read, in the state it read it,
 * rather than overwriting a mutation made while it was in flight. The quads are
 * therefore private to this module, and `Dataset::mutate` is the one door every writer
 * in the crate goes through — a writer that bypassed it would be a mutation no commit
 * could detect.
 *
 * An asynchronous UPDATE also *claims* the dataset while it is in flight
 * (`Dataset::claim_update`): a second asynchronous update of the same dataset is
 * refused until the first is finished, rather than evaluated against a snapshot its
 * commit would then be refused over.
 */
export class Dataset {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * `add(quad)` → insert a quad. Returns `true` if the effective set changed.
     *
     * Throws if the quad carries a relative IRI in any position: this surface is
     * handed terms, never a document, so there is no base in scope to resolve one
     * against and none is invented. The thrown message carries the workspace's
     * shared `iri-relative-no-base` code.
     */
    add(quad: Quad): boolean;
    /**
     * `canonicalize()` → the dataset as canonical, flat N-Quads under RDFC-1.0
     * (SHA-256).
     *
     * The deterministic identity string for the graph: two datasets denote the same
     * RDF graph (under blank-node relabeling) iff their canonical forms are
     * byte-identical. This is the same RDFC-1.0 output the conformance gate pins.
     *
     * This surface reads caller-supplied (untrusted) documents, so a refusal —
     * reserved vocabulary, or n-degree search budget exhaustion — comes back as a
     * thrown `JsError` via the typed [`try_canonicalize_flat_view`] path, never as a
     * wasm trap/process abort.
     */
    canonicalize(): string;
    /**
     * `delete(quad)` → remove a quad. Returns `true` if the effective set changed.
     */
    delete(quad: Quad): boolean;
    /**
     * `has(quad)` → whether the quad is in the dataset.
     */
    has(quad: Quad): boolean;
    /**
     * `isomorphic(other)` → whether this dataset and `other` are the same RDF graph
     * under blank-node relabeling.
     *
     * The formal RDF graph-identity check, backed by full RDFC-1.0 canonicalization:
     * an exact oracle with no false positives or false negatives. Equivalent to
     * comparing the two [`canonicalize`](Self::canonicalize) strings, but avoids
     * materializing them for obviously-different inputs.
     */
    isomorphic(other: Dataset): boolean;
    /**
     * `match(subject?, predicate?, object?, graph?)` → a new dataset of the matching
     * quads. An omitted (`undefined`) position is a wildcard; `defaultGraph()` matches
     * only the default graph, a named node matches that graph.
     */
    match(subject?: Term | null, predicate?: Term | null, object?: Term | null, graph?: Term | null): Dataset;
    /**
     * An empty dataset.
     */
    constructor();
    /**
     * `parse(input, format, base?)` → a dataset of the parsed quads.
     *
     * `format` is a media type or short name
     * (turtle/ntriples/nquads/trig/rdfxml/jsonld/yamlld).
     * Ill-typed literals are preserved verbatim (RDFLib parity), not rejected.
     */
    static parse(input: string, format: string, base?: string | null): Dataset;
    /**
     * Project this dataset into a deterministic graph, tabular, or research-object USTAR package.
     */
    project(profile: string, config_json: string): ProjectionPackage;
    /**
     * Project this dataset plus a canonical payload-only USTAR into an attached RO-Crate.
     */
    projectWithAssets(profile: string, config_json: string, assets_archive: Uint8Array): ProjectionPackage;
    /**
     * `quads()` → every effective quad, as a JS array.
     */
    quads(): Quad[];
    /**
     * `query(sparql, base?, xpathRegex?)` → run a SPARQL query against this dataset,
     * offline.
     *
     * `xpathRegex` selects the dated native XPath law `REGEX`/`REPLACE` evaluate under,
     * exactly as every `QueryEngine` entry takes it (see the module header).
     *
     * Returns **SPARQL Results JSON** for SELECT / ASK, and for CONSTRUCT / DESCRIBE
     * **Turtle** — widened to **TriG** when the result carries a named graph, because
     * Turtle has no `GRAPH` construct and would render such a result as an empty
     * document. TriG is Turtle's dataset superset, so a default-graph-only result is
     * byte-identical Turtle exactly as before; see `default_graph_format`.
     *
     * A parse error, an evaluation error, or a `SERVICE` / `LOAD` clause (unresolvable
     * on this lane) throws a JsError — never a silent empty result. The `SILENT` forms
     * are the caller's own opt-out and succeed with nothing fetched, as SPARQL 1.1
     * requires; see this module's federation note.
     *
     * It runs on a fresh `QueryEngine`, so under the default division policy
     * (`"exact-or-18:half-even"`); set `divisionPolicy` on a `QueryEngine` for another.
     */
    query(sparql: string, base?: string | null, xpath_regex?: string | null): string;
    /**
     * `serialize(format, base?)` → the dataset rendered in `format` (a UTF-8 string).
     *
     * Formats: `turtle` / `ntriples` / `nquads` / `trig` / `rdfxml` / `trix` /
     * `hextuples` / `jsonld` (JSON-LD-star) / `yamlld` (YAML-LD-star), and their media
     * types — all resolved through the one core registry.
     *
     * This is the WRITER-NATIVE lane: it emits everything the target's writer has a
     * surface for, and REFUSES what it does not. Object-position quoted-triple terms
     * and the RDF-1.2 statement layer therefore survive Turtle, N-Triples, N-Quads
     * and TriG (as `<<( … )>>`), RDF/XML (as `rdf:parseType="Triple"`), and JSON-LD /
     * YAML-LD (as `@triple`); TriX and HexTuples, which have no triple-term surface at
     * all, throw rather than drop the layer silently, because this lane has no count
     * to report a drop through.
     *
     * Named graphs are the one thing it does drop without saying so: a single-graph
     * target (Turtle, N-Triples, RDF/XML) emits the default graph alone.
     *
     * # The document base
     *
     * `base` is the egress mirror of [`parse`](Self::parse)'s: a syntax that can
     * express a base (Turtle, TriG, RDF/XML, JSON-LD, YAML-LD) writes it and
     * relativizes its IRIs against it; one that cannot (N-Triples, N-Quads, TriX,
     * HexTuples) emits absolute IRIs, which is the only spelling those grammars admit.
     * A base that is not an absolute IRI throws whatever the target format is; omitting
     * it emits absolute IRIs. No base is ever fabricated.
     *
     * [`serialize_with_loss`](Self::serialize_with_loss) is the TRANSCODE lane
     * instead: it applies the declared format contract, projecting the statement layer
     * to base quads for every format the loss matrix calls star-incapable (RDF/XML,
     * TriX, HexTuples) and COUNTING what it projected, so it never throws for a layer
     * it cannot carry. For a star-capable target the two lanes are byte-identical; for
     * those three they are not, and the difference is exactly the counted loss.
     */
    serialize(format: string, base?: string | null): string;
    /**
     * Serialize JSON-LD/YAML-LD using the shared versioned options decoder.
     *
     * `base` is the document base the output is written under — the egress mirror of
     * [`parse`](Self::parse)'s. Both JSON-LD and YAML-LD can express a base, so it
     * reaches the emitted `@context` as `@base` and document-position `@id`s are
     * compacted against it. A base the caller's own context already declares wins,
     * matching the ingress precedence. A base that is not an absolute IRI throws.
     */
    serializeConfigured(format: string, options_json: string, base?: string | null): string;
    /**
     * Serialize INCREMENTALLY, handing each window to `sink.write`.
     *
     * The streaming twin of [`serialize`](Self::serialize), and the reason the sink's
     * destination is a trait rather than a Rust writer: a JS callback is not
     * `io::Write`, and a browser host otherwise had to take delivery of a whole
     * document — as one JS string, with the UTF-16 copy that implies — before it
     * could write a byte. Peak memory here tracks the staging window instead.
     *
     * `sink` is duck-typed: anything with a `write(chunk: Uint8Array)` method works,
     * which is the shape a `WritableStreamDefaultWriter`, a Node `Writable` and a
     * two-line array collector all already have. Nothing about it is PurRDF's type.
     *
     * A `write` that THROWS aborts the serialization and the throw reaches the
     * caller, rather than being swallowed into a truncated document. That is the
     * failure worth being loud about here: a host that silently received half an
     * answer has no way to discover it.
     *
     * The bytes are identical to [`serialize`](Self::serialize); only the delivery
     * differs.
     */
    serializeToSink(format: string, base: string | null | undefined, sink: { write(chunk: Uint8Array): void }): void;
    /**
     * Serialize JSON-LD/YAML-LD using a reusable compiled context.
     *
     * `base` is the document base, honored exactly as in
     * [`serializeConfigured`](Self::serialize_configured).
     */
    serializeWithContext(format: string, context: CompiledJsonLdContext, yaml_schema_url?: string | null, base?: string | null): string;
    /**
     * `serializeWithLoss(format, base?)` → the document the declared transcode contract
     * produces for `format`, plus the WHOLE realized loss of producing it.
     *
     * Byte-identical to [`serialize`](Self::serialize) for every star-capable target
     * (Turtle, N-Triples, N-Quads, TriG, JSON-LD, YAML-LD). For the three the loss
     * matrix calls star-incapable it deliberately differs: this lane projects the
     * RDF-1.2 statement layer to base quads and counts it, where `serialize` writes
     * the layer natively (RDF/XML) or throws for want of a surface (TriX, HexTuples).
     *
     * The transcode lane cannot refuse the way the CONSTRUCT lane does — a caller
     * asking a TriG document for N-Triples wants its default graph, and that is a
     * legitimate request — so the only honest alternative to refusing is COUNTING. The
     * counts partition the loss by cause, so their sum is the total and no row is
     * charged twice; reading one alone cannot distinguish "nothing was lost" from "the
     * loss was charged to a cause I am not reading".
     *
     * This is the JS twin of the C ABI's `purrdf_serialize` count out-params and of
     * Python's `Store.dump_with_loss`, so the same serialization reports the same
     * three numbers on every host.
     *
     * `base` is the egress document base, carrying exactly the meaning it does on
     * [`serialize`](Self::serialize): the transcode lane is still an EGRESS surface, so
     * leaving it base-free would make it the one writer in the workspace that cannot
     * emit `@base`, and a caller would silently get absolute IRIs from the lane that
     * counts its losses while the lane beside it relativized. A base that is not an
     * absolute IRI throws whatever the target format is; omitting it emits absolute
     * IRIs. No base is ever fabricated.
     */
    serializeWithLoss(format: string, base?: string | null): SerializeLoss;
    /**
     * `snapshot()` → an independent dataset holding this one's current content, with an
     * identity of its own at generation zero. Later changes to either leave the other as
     * it is; the content is shared until one of them changes.
     *
     * # Errors
     *
     * A dataset whose pending changes cannot be frozen.
     */
    snapshot(): Dataset;
    /**
     * `visualExportJson(optionsJson?)` -> model, scene, geometry, and index as JSON.
     */
    visualExportJson(options_json?: string | null): string;
    /**
     * `visualModelJson(optionsJson?)` -> the renderer-neutral RDF 1.2 model as JSON.
     */
    visualModelJson(options_json?: string | null): string;
    /**
     * `visualSvgJson(optionsJson?)` -> deterministic SVG and its complete export.
     */
    visualSvgJson(options_json?: string | null): string;
    /**
     * `generation` — how many mutations this dataset's content has seen.
     *
     * Advances on every `add` or `delete` that changed the effective set and on every
     * applied UPDATE; reading, querying and serializing never move it. An asynchronous
     * UPDATE captures it when it starts and refuses to commit if it has moved since.
     */
    readonly generation: bigint;
    /**
     * `id` — this dataset's identity, unique within the wasm instance and never reused.
     *
     * An exact JavaScript `bigint`, including identities above `2^53`.
     */
    readonly id: bigint;
    /**
     * `size` — the number of effective quads.
     */
    readonly size: number;
}

/**
 * What a delivery (and `fault`, `cancel`) reports.
 */
export enum DeliveryStatus {
    /**
     * The delivery answers the outstanding effect.
     */
    Accepted = 0,
    /**
     * The delivery named an effect that was already answered or abandoned — typically a
     * host promise settling after the job's signal won the race. Ignored; not a fault.
     */
    Stale = 1,
    /**
     * The delivery was malformed (a sequence number never issued, a payload the
     * outstanding effect cannot take). The job's fault is latched.
     */
    Fault = 2,
    /**
     * The job (or the shared exchange) has already finished; the delivery changes
     * nothing.
     */
    Finished = 3,
}

/**
 * What an effect asks the host to do. The numbering is the protocol's; 4 is reserved
 * for paged datasets.
 */
export enum EffectKind {
    /**
     * Answer a forwarded `SERVICE` query: the host calls its `resolveService` once and
     * delivers the answer to the effect's shared exchange ([`AsyncJob::deliver_exchange_bindings`]
     * and siblings), which every job waiting on it receives.
     */
    Service = 1,
    /**
     * Fetch a `LOAD` document.
     */
    Load = 2,
    /**
     * Turn the event loop once and resume.
     */
    Yield = 3,
    /**
     * Wait for a `SERVICE` exchange another job opened, whose answer this job receives
     * when that exchange is delivered.
     */
    AwaitExchange = 5,
}

/**
 * The two-phase outcome of a governed entailment-aware query.
 *
 * `takeOutcome()` and `report` are present together only after closure completed. A
 * closure-phase stop carries neither, preventing a host from treating an incomplete
 * closure as queryable data or as a reasoning certificate.
 */
export class EntailmentQueryOutcome {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Move phase two's governed query outcome out, or `undefined` if closure stopped.
     */
    takeOutcome(): QueryOutcome | undefined;
    /**
     * Move the governor that stopped either phase out, or `undefined` on completion.
     */
    takeTripped(): TrippedGovernor | undefined;
    /**
     * Whether the stop happened before a closure existed.
     */
    readonly closureStopped: boolean;
    /**
     * Whether both closure and query completed under every governor.
     */
    readonly isComplete: boolean;
    /**
     * Byte-stable reasoning report, absent when closure stopped.
     */
    readonly report: string | undefined;
}

/**
 * The HTTP problem a failed operation is answered with: build it with
 * [`SparqlProtocolRequest::problem_for`], read its `status` and whether it is
 * `internal`, and render its `body`.
 */
export class FailureProblem {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The RFC 9457 problem document: `type`, `title`, `status`, `detail` and `code`,
     * plus `correlationId` for an internal failure and `offered` for a `406`.
     *
     * # Errors
     *
     * An internal failure without a `correlationId`.
     */
    body(correlation_id?: string | null): string;
    /**
     * The response's `Content-Type`: `application/problem+json`.
     */
    readonly contentType: string;
    /**
     * Whether the failure is the host's own: its real error belongs in the host's log
     * under a correlation id, which `body` must be given and the response carries.
     */
    readonly internal: boolean;
    /**
     * The response status.
     */
    readonly status: number;
}

/**
 * Reusable immutable native point-cell index with strict search requests.
 */
export class GeoPointIndex {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Search stored buckets using the exact selected public comparison law.
     * # Errors
     * Preserves operational refusal before allocating a response envelope.
     */
    call(request: string): string;
}

/**
 * Immutable geographic profile and scalar/batch request session.
 */
export class GeoSession {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Execute a strict version-one scalar/batch request and return its response.
     * Semantic refusals remain JSON records alongside the same identities.
     * # Errors
     * Operational refusal returns through the host error channel without
     * constructing an error envelope after the original account refuses.
     */
    call(request: string): string;
    /**
     * Prepare an explicit version-one profile, or the standard CRS84 profile.
     *
     * # Errors
     *
     * Refuses malformed records, conflicting references and invalid limits.
     */
    constructor(profile?: string | null);
    /**
     * Prepare immutable native ordered point buckets from a strict request.
     *
     * # Errors
     * Refuses invalid source records, reference mismatch and incomplete admission.
     */
    pointIndex(request: string): GeoPointIndex;
    /**
     * Return the canonical strict profile record.
     *
     * # Errors
     *
     * Refuses a native custom parameter without an exact decimal representation.
     */
    profile(): string;
}

/**
 * One governed execution's receipt: what it was allowed and what it spent.
 *
 * Returned on the complete path as well as the exhausted one — "completed, cost N fuel,
 * peak M cells" is how a caller sizes the next call's budget in the first place.
 *
 * [`Self::consumed`] and [`Self::limits`] are positional vectors indexed by
 * [`governor_dimensions`], not maps, because the wasm boundary has no exact map of 64-bit
 * integers to hand over: a JSON object would round `2**64 - 2` — the ceiling a metered but
 * unbounded dimension actually carries — to something it is not.
 */
export class GovernorEvidence {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Consumption charged per dimension, positionally by [`governor_dimensions`].
     *
     * A peak-tracked dimension (`intermediate-cells`, `udf-depth`) reports the largest
     * single observation; every other dimension reports the running sum.
     */
    readonly consumed: BigUint64Array;
    /**
     * Every XPath F&O numeric error the execution absorbed into an unbound value (an
     * expression error is not a query error, SPARQL 1.1 §17.2), counted positionally by
     * [`expression_error_codes`]: `1/0` counts under `err:FOAR0001`. All zero when no
     * expression failed.
     */
    readonly expressionErrors: BigUint64Array;
    /**
     * Whether the execution completed with every governor intact.
     *
     * The governor itself is on the outcome rather than duplicated here: one trip, one
     * object, so a consumer cannot read two and wonder which is authoritative.
     */
    readonly isComplete: boolean;
    /**
     * The inclusive ceilings in force, positionally by [`governor_dimensions`].
     *
     * A governed call meters every caller-settable dimension, so a dimension the caller
     * declined reads `2**64 - 2` — engaged, at a ceiling no execution can reach.
     */
    readonly limits: BigUint64Array;
    /**
     * Every invocation a `SERVICE SILENT` or `LOAD SILENT` absorbed: the answer the
     * specification requires for it is indistinguishable from an endpoint with nothing
     * to add, and this is where the difference is kept. Empty when nothing failed.
     */
    readonly silenced: SilencedInvocation[];
}

/**
 * The outcome of one governed query whose complete answer was serialized in the format
 * negotiated from an `Accept` header (`queryGovernedNegotiatedAsync`).
 *
 * The same two shapes as [`QueryOutcome`], with the complete answer as a document rather
 * than a typed result: `takeBody()` with its `format` and `mediaType` when every governor
 * held, `takePartial()` with `takeTripped()` when one did not. The partial rows stay
 * typed — they are a certificate to inspect, not a document to send.
 */
export class NegotiatedOutcome {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Move the complete answer's UTF-8 bytes out, or `undefined` when a governor stopped
     * the execution.
     */
    takeBody(): Uint8Array | undefined;
    /**
     * Move this execution's receipt out. Present on both paths.
     */
    takeEvidence(): GovernorEvidence | undefined;
    /**
     * Move the partial-answer certificate out, or `undefined` when the query completed.
     */
    takePartial(): PartialAnswers | undefined;
    /**
     * Move the tripped governor out, or `undefined` when the query completed.
     */
    takeTripped(): TrippedGovernor | undefined;
    /**
     * The negotiated format token (`json`, `xml`, `csv`, `tsv`, `turtle`, `trig`,
     * `ntriples`, `nquads` or `jsonld`), or `undefined` when a governor stopped the
     * execution.
     */
    readonly format: string | undefined;
    /**
     * Whether every governor stayed intact and the body is the query's complete answer.
     */
    readonly isComplete: boolean;
    /**
     * The body's media type, for the response's `Content-Type`, or `undefined` when a
     * governor stopped the execution.
     */
    readonly mediaType: string | undefined;
}

/**
 * What the rows a truncated execution reached bound, relative to the query's true answer.
 *
 * A three-way interval, not a yes/no: `"certain"` rows are a certified **lower** bound and
 * are safe to admit as answers; `"at-most"` rows are a certified **upper** bound and are
 * sound only for the negative reading (a row absent from them is definitively not an
 * answer); `"unknown"` means neither bound survived, so **no row is handed over at all**
 * and [`Self::barrier`] names the operator that withheld them instead.
 */
export class PartialAnswers {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Move the rows in hand out of this certificate, or `undefined` on the `"unknown"`
     * class, where rows that bound the answer on neither side offer no sound use and one
     * unsound one.
     */
    takeResult(): QueryResult | undefined;
    /**
     * The algebra operator that withheld the rows, on the `"unknown"` class — which is
     * what says whether a larger budget or a different query is the way forward.
     */
    readonly barrier: string | undefined;
    /**
     * What these rows certify: `"certain"`, `"at-most"`, or `"unknown"`.
     */
    readonly certainty: string;
    /**
     * Whether these rows are certified answers — i.e. whether they may be admitted.
     */
    readonly isCertain: boolean;
    /**
     * Whether these rows are the true answer's **first** rows, in order. This licenses
     * resumption by raising a deterministic ceiling; a wall-deadline rerun is fresh and
     * may stop sooner. `undefined` on the `"unknown"` class.
     */
    readonly isPositionalPrefix: boolean | undefined;
}

/**
 * Result of lifting a strict carrier package into an in-memory RDF dataset.
 */
export class ProjectionLift {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Move the lifted dataset out of this result. The dataset can be taken once.
     */
    takeDataset(): Dataset | undefined;
    /**
     * Canonical, versioned runtime loss-ledger JSON.
     */
    readonly lossLedgerJson: string;
}

/**
 * A deterministic USTAR projection package and its canonical runtime ledger.
 */
export class ProjectionPackage {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Canonical deterministic USTAR bytes.
     */
    readonly archive: Uint8Array;
    /**
     * Canonical, versioned runtime loss-ledger JSON.
     */
    readonly lossLedgerJson: string;
    /**
     * Stable carrier profile name.
     */
    readonly profile: string;
}

/**
 * The `queryHash`/`engine` halves of a decoded provenance extension — the inverse of
 * `queryRaw`'s `provenanceNamespace` option. `undefined` in either slot means the
 * document carried no member under `prefix`, or the member omitted that field.
 *
 * Per-solution source provenance (`ResultProvenance::solutions`) is not exposed here:
 * no writer on this surface (nor `purrdf_validate::query::provenance`, which every host
 * emits through) populates it — it is the evaluator/S11 derivation graph's progressive fill
 * (see `purrdf_sparql_results::ResultProvenance`'s module docs) — so there is nothing
 * yet for this binding to round-trip beyond `queryHash`/`engine`.
 */
export class ProvenanceInfo {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The decoded `engine` label, or `undefined` if absent.
     */
    readonly engine: string | undefined;
    /**
     * The decoded `queryHash`, or `undefined` if absent.
     */
    readonly queryHash: string | undefined;
}

/**
 * An RDF/JS [Quad](https://rdf.js.org/data-model-spec/#quad-interface) — a statement
 * `(subject, predicate, object, graph)` with `termType: "Quad"`.
 */
export class Quad {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * A quoted-triple [`Term`] (`termType: "Quad"`) viewing this quad's `(s, p, o)` —
     * the RDF-1.2 wedge: pass the result as a subject/object to embed it.
     */
    asTerm(): Term;
    /**
     * Structural RDF/JS quad equality.
     */
    equals(other: Quad): boolean;
    /**
     * The graph [`Term`] of the quad (`DefaultGraph` when unnamed).
     */
    readonly graph: Term;
    /**
     * The object [`Term`] of the quad.
     */
    readonly object: Term;
    /**
     * The predicate [`Term`] of the quad.
     */
    readonly predicate: Term;
    /**
     * The subject [`Term`] of the quad.
     */
    readonly subject: Term;
    /**
     * Always `"Quad"` (a Quad is itself an RDF/JS term).
     */
    readonly termType: string;
    /**
     * Empty for a Quad (per RDF/JS).
     */
    readonly value: string;
}

/**
 * A reusable SPARQL engine that keeps the native plan cache alive across calls.
 *
 * The engine is reference-counted so an asynchronous job can keep evaluating on it
 * after the JavaScript handle that started the job has been freed, and so the
 * synchronous methods can run on it while a job is suspended.
 */
export class QueryEngine {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Run an ASK query and return the boolean result.
     */
    ask(dataset: Dataset, sparql: string, base?: string | null, xpath_regex?: string | null): boolean;
    /**
     * Start an asynchronous operation of `kind` over a snapshot of `dataset`, under
     * `options` (from `AsyncJobOptions.fromJs` for the same kind).
     *
     * The dataset is frozen now: a query sees this snapshot whatever happens to the
     * dataset afterwards, and an update captures its identity and generation so
     * `commitUpdate` can refuse to overwrite a mutation made while it ran. An update
     * also claims the dataset until the job is finished: a second asynchronous update of
     * it is refused (`native-sparql-update-in-flight`) rather than queued. The SPARQL
     * text is parsed when the job runs, so a parse error is the job's error, with the
     * synchronous twin's words. A `rawWithContext` operation starts through
     * `beginAsyncWithContext` instead.
     *
     * # Errors
     *
     * Options validated for another kind, a malformed JSON-LD options document or
     * provenance namespace, an update already in flight on `dataset`, or a dataset that
     * cannot be frozen.
     */
    beginAsync(dataset: Dataset, kind: AsyncOperationKind, sparql: string, options: AsyncJobOptions): AsyncJob;
    /**
     * Start a `rawWithContext` operation: a CONSTRUCT/DESCRIBE serialized under a
     * compiled JSON-LD context (and, for YAML-LD, the options' `yamlSchemaUrl`), in the
     * format the options were validated with.
     *
     * # Errors
     *
     * As [`Self::begin_async`], plus a kind other than `rawWithContext` and an invalid
     * `yamlSchemaUrl`.
     */
    beginAsyncWithContext(dataset: Dataset, kind: AsyncOperationKind, sparql: string, options: AsyncJobOptions, context: CompiledJsonLdContext): AsyncJob;
    /**
     * Run a CONSTRUCT query and return its result dataset.
     */
    construct(dataset: Dataset, sparql: string, base?: string | null, xpath_regex?: string | null): Dataset;
    /**
     * Run a DESCRIBE query and return its result dataset.
     */
    describe(dataset: Dataset, sparql: string, base?: string | null, xpath_regex?: string | null): Dataset;
    /**
     * The engine's charge ledger for a query: the join orders it chose, the plan
     * estimates it surveyed, and what each algebra node actually cost, rendered as text.
     *
     * This is how a caller sizes a budget before setting one. The query is evaluated
     * under the metering configuration — every dimension counted, no dimension bounded —
     * so the numbers are measurements of this query over this data, not predictions.
     *
     * # Errors
     *
     * A parse or evaluation failure.
     */
    explainQuery(dataset: Dataset, sparql: string, base?: string | null, xpath_regex?: string | null): string;
    /**
     * Create a reusable offline SPARQL engine.
     */
    constructor();
    /**
     * Run any SPARQL query and return a typed raw wasm result wrapper.
     */
    query(dataset: Dataset, sparql: string, base?: string | null, xpath_regex?: string | null): QueryResult;
    /**
     * Run a governed SPARQL query over a closure produced by `regime`, carrying the
     * closure report and query outcome together.
     *
     * `aggregate_namespace` behaves exactly as on [`Self::query_governed`]: it registers
     * PurRDF's first-party statistical aggregate set under that IRI namespace for the
     * closure query's PARSE and its evaluation, so `AGG(<{NAMESPACE}NAME>, args…)` reaches
     * the entailment-aware lane exactly as it reaches the ordinary one. `undefined` (the
     * default) leaves every one of the ten names an ordinary unregistered custom-aggregate
     * IRI.
     *
     * `import_iris`, `import_documents` and `premise_iris` are `entailCertainAnswers`'s, and
     * apply to `dataset`: OWL 2 defines an ontology's imports closure to BE the ontology, so
     * the closure the query runs over is materialized over the dataset merged with every
     * N-Quads document the table supplies, and the report then states
     * `ontology-import-resolved`. An `owl:imports` the table does not resolve, and the
     * dataset does not already hold, throws by name — never a closure of a smaller premise —
     * and so does a table entry the closure never reaches. Empty arrays are the ordinary
     * "imports nothing" case, and all three are required rather than defaulted.
     *
     * `max_stored_facts` and `max_join_steps` (each a `bigint`, or `undefined` for this
     * target's default of 131072 facts and 1048576 join steps) are the closure's evaluation
     * limits for the `rdf`, `rdfs`, `owl-rl` and `d` regimes, exactly as `entailMaterialize`
     * takes them. A closure past one throws naming the limit, the numbers and this method's
     * argument (`queryEntailmentGoverned's maxStoredFacts`, …); the governors price the
     * evaluation over the closure and never become a limit on it.
     *
     * # Errors
     *
     * An invalid regime/program, an import-table refusal, query parse/evaluation failure,
     * entailment failure, or malformed ceiling. A governor trip is returned in
     * [`EntailmentQueryOutcome`].
     */
    queryEntailmentGoverned(dataset: Dataset, sparql: string, base: string | null | undefined, regime: string, program: string | null | undefined, import_iris: string[], import_documents: string[], premise_iris: string[], max_stored_facts?: bigint | null, max_join_steps?: bigint | null, aggregate_namespace?: string | null, fuel?: bigint | null, deadline_ms?: bigint | null, max_answers?: bigint | null, max_intermediate_cells?: bigint | null, max_scratch_bytes?: bigint | null, max_remote_requests?: bigint | null, cancel?: CancellationToken | null, no_ceiling?: boolean | null, xpath_regex?: string | null): EntailmentQueryOutcome;
    /**
     * Run a SPARQL query under caller-supplied execution governors, returning a
     * [`QueryOutcome`] rather than the answers directly.
     *
     * Every ceiling is optional; `undefined` leaves that dimension metered without a
     * reachable cap. `no_ceiling` removes resource caps and accounting explicitly,
     * refuses a simultaneous resource cap, and retains deadline/cancellation signals:
     * `fuel` bounds abstract execution steps, `deadline_ms` a wall-clock budget in
     * milliseconds, `max_answers` the answer sequence (solution rows for SELECT, output
     * statements for CONSTRUCT/DESCRIBE — including RDF 1.2 reifier and annotation
     * statements — and nothing for ASK), `max_intermediate_cells` the largest intermediate
     * bag in `rows * columns`, `max_scratch_bytes` the per-query scratch arena, and
     * `max_remote_requests` federated requests. Every ceiling is **inclusive**:
     * consumption equal to it is admitted, and zero is a valid ceiling that trips on the
     * first charged unit of work.
     *
     * **A tripped governor is an outcome, not a thrown error** — see the module header.
     *
     * `aggregate_namespace` registers PurRDF's first-party statistical aggregate set
     * (`MEDIAN`, `PERCENTILE`, `STDDEV`, `STDDEV_POP`, `VARIANCE`, `VAR_POP`, `MODE`,
     * `FIRST`, `LAST`, `TOPK`) under that IRI namespace, so the query text can call
     * `AGG(<{NAMESPACE}NAME>, args…)` (see `purrdf_validate::query::statistical_aggregates`). `None` (the default)
     * leaves every one of the ten names an ordinary unregistered custom-aggregate IRI.
     *
     * # Errors
     *
     * A parse or evaluation failure, and a negative ceiling. A governor trip is neither.
     */
    queryGoverned(dataset: Dataset, sparql: string, base?: string | null, aggregate_namespace?: string | null, fuel?: bigint | null, deadline_ms?: bigint | null, max_answers?: bigint | null, max_intermediate_cells?: bigint | null, max_scratch_bytes?: bigint | null, max_remote_requests?: bigint | null, cancel?: CancellationToken | null, no_ceiling?: boolean | null, xpath_regex?: string | null): QueryOutcome;
    /**
     * Run any SPARQL query and serialize its raw result.
     *
     * `format` is optional: omitted, a SELECT/ASK result is SPARQL-Results JSON and a
     * CONSTRUCT/DESCRIBE result is Turtle, or TriG when it carries a named graph
     * (`default_graph_format`). Naming a single-graph RDF syntax for a
     * graph-carrying result throws rather than emitting a document missing exactly
     * what the query asked for (`refuse_uncarriable_named_graphs`).
     *
     * `provenance_prefix`/`provenance_iri` (both `undefined`, or both a string) anchor
     * the additive `purrdf` provenance extension on a SELECT/ASK result serialized to
     * SPARQL-results JSON/XML, under that `PREFIX`/`IRI`. `undefined` (the default)
     * leaves the output pure W3C; a CONSTRUCT/DESCRIBE graph result and CSV/TSV never
     * carry the extension. Read it back with `provenanceFromJson`/`provenanceFromXml`
     * under the SAME namespace.
     *
     * # Errors
     *
     * A parse/evaluation failure, an unsupported format, or exactly one of
     * `provenance_prefix`/`provenance_iri` supplied without the other.
     */
    queryRaw(dataset: Dataset, sparql: string, base?: string | null, format?: string | null, provenance_prefix?: string | null, provenance_iri?: string | null, xpath_regex?: string | null): string;
    /**
     * Serialize a CONSTRUCT/DESCRIBE result with configured JSON-LD/YAML-LD.
     */
    queryRawConfigured(dataset: Dataset, sparql: string, base: string | null | undefined, format: string, options_json: string, xpath_regex?: string | null): string;
    /**
     * Serialize a CONSTRUCT/DESCRIBE result with a reusable compiled context.
     */
    queryRawWithContext(dataset: Dataset, sparql: string, base: string | null | undefined, format: string, context: CompiledJsonLdContext, yaml_schema_url?: string | null, xpath_regex?: string | null): string;
    /**
     * Run a SELECT query and return typed rows.
     */
    select(dataset: Dataset, sparql: string, base?: string | null, xpath_regex?: string | null): SelectResult;
    /**
     * Select the same strict version-one profile used by every native host.
     *
     * # Errors
     * Malformed records or invalid configuration leave the current selection intact.
     */
    setGeoProfile(profile: string): void;
    /**
     * Select the immutable native geographic profile for subsequent operations.
     * An already started asynchronous operation retains its original selection.
     */
    setGeoSession(session: GeoSession): void;
    /**
     * Apply a SPARQL UPDATE atomically to the supplied dataset.
     */
    update(dataset: Dataset, sparql: string, base?: string | null, xpath_regex?: string | null): void;
    /**
     * Apply a SPARQL UPDATE under caller-supplied execution governors, returning an
     * [`UpdateOutcome`] rather than mutating unconditionally.
     *
     * The ceilings are those of [`Self::query_governed`] minus `max_answers`, which bounds
     * an answer sequence an UPDATE does not have — passing it is refused rather than
     * ignored, because a governor a caller believes they set and that nothing enforces is
     * worse than no governor at all. A request's size is bounded by the ceilings on the
     * work that computes it.
     *
     * **A tripped request applies nothing.** Not "not all of it": the dataset is left
     * exactly as it was found, whichever operation the governor stopped and however much
     * work the earlier operations of the same request had already done.
     *
     * `aggregate_namespace` behaves exactly as on [`Self::query_governed`], reachable
     * from a `DELETE`/`INSERT … WHERE` clause through a nested `SELECT … GROUP BY` —
     * the only place SPARQL UPDATE's grammar admits an aggregate.
     *
     * # Errors
     *
     * A parse or evaluation failure, a negative ceiling, and `max_answers`. A governor
     * trip is none of these.
     */
    updateGoverned(dataset: Dataset, sparql: string, base?: string | null, aggregate_namespace?: string | null, fuel?: bigint | null, deadline_ms?: bigint | null, max_answers?: bigint | null, max_intermediate_cells?: bigint | null, max_scratch_bytes?: bigint | null, max_remote_requests?: bigint | null, cancel?: CancellationToken | null, no_ceiling?: boolean | null, xpath_regex?: string | null): UpdateOutcome;
    /**
     * How a blank node's scope crosses to JS in the typed results this engine returns:
     * `"keep"` (the default) or `"merge"`.
     *
     * The engine holds a blank node as a label and a scope, so two nodes that share a
     * label in different scopes are two nodes. Under `"keep"` a scoped blank crosses as
     * its scope envelope and they stay two nodes in JS; an unscoped blank keeps its bare
     * label. Under `"merge"` the scope is dropped and every blank crosses as its bare
     * label, so nodes that share a label in different scopes are ONE node in JS. Set it
     * before running a query: it applies to `query`, `select`, `queryGoverned` and
     * `queryEntailmentGoverned`, and an asynchronous job takes the value in force when
     * it begins. The serialized (`queryRaw`) results are unaffected.
     */
    blankScope: string;
    /**
     * The precision every `xsd:integer`/`xsd:decimal` quotient (`/` and `AVG`) this
     * engine forms is computed at, in its one text form: `"exact"`, or `"N:ROUNDING"`
     * — `N` fractional digits rounded in the named direction. The default is
     * `"exact-or-18:half-even"`: exact when terminating, otherwise rounded.
     *
     * Under `"exact"` a quotient with no finite decimal expansion (`1/3`) is not
     * rounded: it is a SPARQL expression error, unbound like `1/0` and counted as
     * `err:FOAR0002` in a governed outcome's evidence, while a terminating one (`1/8`)
     * answers exactly. The policy applies to every query,
     * update, explain, governed, entailment, negotiated and serialized entry of this
     * engine, and an asynchronous job takes the policy in force when it begins.
     * `Dataset.query` and `Dataset.update` run on a fresh engine, so under the default.
     */
    divisionPolicy: string;
}

/**
 * The outcome of one governed query: a complete answer, or an exhausted budget carrying
 * the partial answers the execution actually reached.
 *
 * Exactly two shapes, and **neither is a thrown error**. Check `isComplete`, take
 * `takeResult()` when it holds, and take `takeTripped()` with `takePartial()` when it
 * does not. Every accessor that hands over a wasm-owned object moves it, so each can be
 * taken once.
 */
export class QueryOutcome {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Move this execution's receipt out. Present on both paths.
     */
    takeEvidence(): GovernorEvidence | undefined;
    /**
     * Move the partial-answer certificate out, or `undefined` when the query completed.
     */
    takePartial(): PartialAnswers | undefined;
    /**
     * Move the **complete** result out, or `undefined` when a governor stopped the
     * execution.
     *
     * Deliberately never the partial rows: a caller that stopped reading the outcome one
     * level too early receives nothing rather than a truncated answer wearing a complete
     * answer's type. The rows a trip reached are behind `takePartial()`, with the
     * certificate that says what they bound.
     */
    takeResult(): QueryResult | undefined;
    /**
     * Move the tripped governor out, or `undefined` when the query completed.
     */
    takeTripped(): TrippedGovernor | undefined;
    /**
     * Whether every governor stayed intact and this is the query's complete answer.
     */
    readonly isComplete: boolean;
}

/**
 * A typed SPARQL result returned by the raw wasm binding.
 */
export class QueryResult {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Move the graph dataset out of this wrapper.
     */
    takeDataset(): Dataset | undefined;
    /**
     * Move the SELECT result out of this wrapper.
     */
    takeSelect(): SelectResult | undefined;
    /**
     * The ASK boolean when `kind === "ask"`, otherwise `undefined`.
     */
    readonly boolean: boolean | undefined;
    /**
     * The result discriminator: `"select"`, `"ask"`, or `"graph"`.
     */
    readonly kind: string;
}

/**
 * A reasoning session over one ontology — `new Reasoner(document, stepCap)`.
 *
 * Every `entail*` function above takes the document as a string and rebuilds
 * everything it needs, so asking three questions parses and reverse-maps the ontology
 * three times. This class holds the parsed document instead: constructing it parses
 * once, the first question needing a knowledge base reverse-maps once, and later
 * questions reuse both. That matters more here than on any other host — the browser
 * pays this cost on the main thread.
 *
 * The methods answer exactly what the same-named functions answer: they are the
 * `purrdf_validate::regime` session those functions now wrap.
 *
 * ```js
 * const r = new Reasoner(document, 0);
 * const consistent = r.consistency();
 * const hierarchy = r.classify();   // no second parse
 * r.free();                         // wasm objects are not garbage-collected for you
 * ```
 */
export class Reasoner {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The entailed subsumption hierarchy. See `entailClassify`.
     */
    classify(): ReasoningAnswer;
    /**
     * Is the knowledge base consistent? See `entailConsistency`.
     */
    consistency(): ReasoningAnswer;
    /**
     * Does the ontology entail `axiom`? See `entailEntails`.
     */
    entails(axiom: string): ReasoningAnswer;
    /**
     * Why `conclusion` holds under `regime`. See `entailExplainConclusion`.
     */
    explainConclusion(regime: string, conclusion: string): ReasoningAnswer;
    /**
     * A module for `signature`. See `entailExtractModule`.
     */
    extractModule(signature: string, method: string): ReasoningAnswer;
    /**
     * The individuals entailed to be instances of `class`. See `entailInstances`.
     */
    instances(_class: string): ReasoningAnswer;
    /**
     * A justification for `axiom`. See `entailJustify`.
     */
    justify(axiom: string): ReasoningAnswer;
    /**
     * Parse `document` and open a session over it.
     *
     * `stepCap` narrows the per-decision tableau step cap for every question asked
     * through this session; **0 means the knowledge base's own cap**, not a cap of zero
     * steps, and it can only NARROW. `workCap` narrows the per-decision WORK cap on the
     * same rule — the cap on the matcher, scan, closure and clone work done INSIDE a
     * round, which a round cap cannot see.
     *
     * Nothing is reverse-mapped here, so an ontology whose knowledge base cannot be
     * built still constructs — and throws on the first question that needs one. That is
     * deliberate: `profile`, `extractModule`, `justify` and `explainConclusion` never
     * reason, and `profile` answers for any parseable document.
     *
     * Throws if `document` fails to parse.
     */
    constructor(document: string, step_cap: number, work_cap: number);
    /**
     * Which OWL 2 profiles the ontology is provably in. See `entailProfile`.
     *
     * Purely syntactic: never builds a knowledge base, so it answers even for an
     * ontology whose other services would throw.
     */
    profile(): ReasoningAnswer;
    /**
     * Answer `service` about `argument`, with its proof. See `entailProve`.
     *
     * Throws on a session that records nothing: open it with `Reasoner.withProofs`.
     */
    prove(service: string, argument: string): ReasoningAnswer;
    /**
     * The entailed types of the named individuals. See `entailRealize`.
     */
    realize(): ReasoningAnswer;
    /**
     * Whether this session records proof terms — `true` only for `Reasoner.withProofs`.
     */
    recordsProofs(): boolean;
    /**
     * `Reasoner.withProofs(document, stepCap, workCap)` → a session that RECORDS a proof
     * term for every service that has one.
     *
     * The session-level opt-in. `new Reasoner(...)` is unchanged and records nothing; this
     * records, so every answer it hands back carries a real `.proof` document instead of the
     * `availability not-recorded` one, and `prove` becomes callable.
     *
     * It changes nothing a service DECIDES. Every `.answer` and `.certificate` is
     * byte-identical to the same question asked through `new Reasoner(...)`.
     *
     * Throws if `document` fails to parse.
     */
    static withProofs(document: string, step_cap: number, work_cap: number): Reasoner;
}

/**
 * One reasoning service's answer and the certificate of the run that produced it.
 *
 * A class with two named getters rather than a positional pair, exactly as
 * [`RegimeClosure`] is and for the same reason: both strings cross the host
 * boundary and a two-string tuple is the wrong thing for a JS caller to get
 * backwards. There is no certificate-free getter — a caller that ignores the
 * evidence must at least have been handed it.
 */
export class ReasoningAnswer {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The service's answer.
     *
     * Line-oriented and byte-stable. A dataset-valued answer
     * (`entailExtractModule`, `entailJustify`) is canonical (RDFC-1.0) N-Quads
     * instead.
     */
    readonly answer: string;
    /**
     * How completely the service decided, in one of THREE certificate grammars —
     * which one is a property of the service, so a caller reads it off the call
     * rather than sniffing the string:
     *
     * - `purrdf-dl-certificate 1` — the tableau services (`entailConsistency`,
     *   `entailClassify`, `entailRealize`, `entailInstances`, `entailEntails`).
     * - `purrdf-reasoning-report 4` — the entailment-regime services
     *   (`entailCertainAnswers`, `entailGraphEntails`, `entailVerifyEntailment`),
     *   which decide by the regime's rule table rather than by a tableau.
     * - that service's own grammar — the three that run neither (`entailProfile`,
     *   `entailExtractModule`, `entailExplainConclusion`).
     *
     * The DL lane's completeness is `decided` / `decided-within-boundaries` /
     * `budget-exhausted` — NOT the chase's `exact` / `sound-incomplete`, which is
     * a difference of two rule tables and would report "exact" for a tableau that
     * ran out of budget.
     */
    readonly certificate: string;
    /**
     * The PROOF TERM of the run that produced this answer, as a `purrdf-dl-proof 1`
     * document — the thing `entailCheckProof` takes.
     *
     * Never the empty string, and that is the point. Recording is OPT-IN: an answer from an
     * `entail*` function or from a `new Reasoner(...)` session records nothing, and this
     * getter then reads
     *
     * ```text
     * purrdf-dl-proof 1
     * availability not-recorded
     * ```
     *
     * which says NOTHING WAS MEASURED. A recorded proof says `availability recorded`, and a
     * recorded proof with `runs 0` in it says something different again: that the service is
     * syntactic (`extractModule` is), so there was no search to check and the checker
     * verified exactly that. Three states, three documents, and `entailCheckProof` refuses
     * the first by name rather than reporting a verification of it.
     *
     * Ask for a proof with `entailProve(...)` or `Reasoner.withProofs(...)`.
     */
    readonly proof: string;
}

/**
 * One closure of one document under one regime: the canonical N-Quads and the
 * rendered reasoning report.
 *
 * A class with two named getters rather than a positional pair, because a
 * two-string tuple is the wrong thing for a JS caller to get backwards.
 */
export class RegimeClosure {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The materialized dataset — every input quad plus every inferred triple —
     * as canonical (RDFC-1.0) N-Quads.
     */
    readonly nquads: string;
    /**
     * What the run actually did: which rules fired and how often, which
     * specification rules did NOT fire, which constructs were left at a
     * boundary, the evaluation budget, and the calculus's contract hash.
     */
    readonly report: string;
}

/**
 * What `purrdf_jspi_run` reports.
 */
export enum RunStatus {
    /**
     * The job's outcome is stored.
     */
    Outcome = 0,
    /**
     * The job's error is stored.
     */
    Error = 1,
    /**
     * No job has this id.
     */
    UnknownJob = 2,
    /**
     * The job had already been started.
     */
    AlreadyStarted = 3,
}

/**
 * A typed SELECT result returned by the raw wasm binding.
 */
export class SelectResult {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Move the next unconsumed row out of the result.
     */
    nextRow(): SelectRow | undefined;
    /**
     * Move a row out by result index. Each row can be consumed once.
     */
    takeRow(index: number): SelectRow | undefined;
    /**
     * The result discriminator.
     */
    readonly kind: string;
    /**
     * Number of rows that have not yet been consumed.
     */
    readonly remaining: number;
    /**
     * Total number of SELECT rows, including rows already consumed.
     */
    readonly rowCount: number;
    /**
     * Projected variables, in SELECT projection order.
     */
    readonly variables: string[];
}

/**
 * One SELECT binding row.
 */
export class SelectRow {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Return the bound term for a variable name, or `undefined` for unbound/absent.
     */
    get(variable: string): Term | undefined;
    /**
     * Move one value out by projection index, or return `undefined` when the
     * cell is unbound, absent, or was already consumed.
     */
    takeValue(index: number): Term | undefined;
    /**
     * Variables projected by this row, in SELECT projection order.
     */
    readonly variables: string[];
}

/**
 * One serialized document plus the realized loss of producing it, partitioned by
 * CAUSE — the return of [`Dataset::serialize_with_loss`].
 *
 * Three counts rather than one flag because the causes are independent: a
 * star-capable single-graph target (Turtle, N-Triples) loses named graphs and no
 * statement rows; a dataset-capable star-incapable one (TriX, HexTuples) loses
 * statement rows and no named graphs, and also every base direction; RDF/XML loses
 * statement rows AND named graphs while carrying direction. A single "lossy" boolean
 * would answer none of the questions a caller actually has.
 *
 * Every count is REALIZED — what this document actually discarded — not the static
 * pair contract of `lossMatrixJson`, which says what a format PAIR can lose in
 * principle.
 */
export class SerializeLoss {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Object literals whose RDF-1.2 base direction the target has no surface for
     * (TriX / HexTuples keep the language tag but cannot carry the direction). `0`
     * for every direction-capable format.
     */
    readonly directionalLiteralsDropped: number;
    /**
     * Declared named graphs holding no row that the target has no spelling for
     * (N-Quads, HexTuples and every single-graph syntax), so the document omits them.
     * They own no row, so no other count sees them. `0` for TriG, TriX, JSON-LD and
     * YAML-LD, which write an empty graph, and for a dataset that declares none.
     */
    readonly emptyNamedGraphsDropped: number;
    /**
     * Rows the single-graph flattening dropped because the target has no named-graph
     * construct: base quads asserted in a named graph plus the statement-layer rows
     * scoped to one. `0` for every dataset-capable format. The rows are DROPPED, not
     * folded into the default graph.
     */
    readonly namedGraphRowsDropped: number;
    /**
     * RDF-1.2 statement-layer rows (reifier bindings + annotation triples) dropped
     * because the target cannot represent quoted triples. `0` for star-capable
     * formats. Rows dropped because they were scoped to a named graph are counted by
     * [`named_graph_rows_dropped`](Self::named_graph_rows_dropped) instead, never
     * here and never twice.
     */
    readonly statementRowsDropped: number;
    /**
     * The serialized document. Identical to what `serialize(format)` returns for a
     * star-capable target; for RDF/XML, TriX and HexTuples it is the contract's
     * projected document instead — the statement layer reduced to base quads and
     * charged to [`statement_rows_dropped`](Self::statement_rows_dropped).
     */
    readonly text: string;
}

/**
 * The per-service policy host-resolved `SERVICE` requests and `LOAD` fetches are
 * authorized against, before the host is ever called: deny by default, one profile per
 * endpoint (or `LOAD` source), an optional fallback. Each profile is JSON:
 *
 * ```json
 * {
 *   "capabilities": ["query", "network", "credentials"],
 *   "headers": [["X-Tenant", "a"], ["Accept-Language", "en"]],
 *   "credential": { "header": "Authorization", "value": "Bearer …" },
 *   "userAgent": "my-worker/1.0",
 *   "timeoutMs": 5000
 * }
 * ```
 *
 * `capabilities` is required; a host-resolved `SERVICE` request needs `query` and
 * `network`, a `LOAD` fetch `network`, and a `credential` needs `credentials` too (the
 * native rule: a credential that may not be sent refuses the request rather than sending
 * it without). `timeoutMs` bounds each request to the service: the host abandons one
 * unanswered after it, or at the job's deadline when that falls first. `headers` is an array of
 * `[name, value]` pairs because order and repeated names are significant. Unknown keys
 * and capability names are refused.
 */
export class ServiceCatalog {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Register the profile `profileJson` for the service IRI `endpoint`, replacing any
     * earlier one.
     */
    addService(endpoint: string, profile_json: string): void;
    /**
     * An independent copy of this catalog: later changes to either leave the other as
     * it is. An asynchronous operation's options take a copy of the catalog they name,
     * so the caller's own stays usable for later operations.
     */
    copy(): ServiceCatalog;
    /**
     * An empty catalog: every service is denied.
     */
    constructor();
    /**
     * Apply the profile `profileJson` to every service with no entry of its own — the
     * explicit opt-out of deny-by-default.
     */
    setFallback(profile_json: string): void;
}

/**
 * The outcome of `shaclValidateChangesToSarif`: the SARIF log, and the SCOPE that
 * log describes.
 *
 * Two facts rather than one, because a report alone cannot say which question it
 * answered. `bounded === true` means the log covers the focus nodes the change
 * could move — for those nodes it is identical, results and ordering alike, to a
 * full validation of the mutated graph — and is silent about a pre-existing
 * violation the change cannot reach, so an empty log means *this change
 * introduced no violation*. `bounded === false` means the shapes graph reads
 * through SPARQL query text, no bounded footprint exists for it, the call fell
 * back to a FULL validation of the mutated graph, and an empty log means *the
 * graph conforms*. The weaker reading is the dangerous one, so it is stated
 * rather than left to be assumed.
 *
 * Like every other wasm-bindgen class in this package, this owns wasm memory and
 * is released with `.free()`.
 */
export class ShaclChangeValidation {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Whether the change's footprint could be bounded.
     */
    readonly bounded: boolean;
    /**
     * How many focus nodes the change was expanded into, or `undefined` when the
     * footprint could not be bounded and the whole graph was validated.
     *
     * `undefined` rather than the graph's node count on the fallback path: "every
     * focus node in the graph" and a number are different statements, and
     * collapsing them would make a fallback indistinguishable from a large
     * bounded expansion.
     */
    readonly focusNodes: number | undefined;
    /**
     * Which construct made this shapes graph's change footprint unbounded, or
     * `undefined` when it was bounded.
     *
     * Actionable rather than decorative: it names what to change to get
     * incremental validation back.
     */
    readonly reason: string | undefined;
    /**
     * The SARIF 2.1.0 JSON log. See `bounded` for what it describes.
     */
    readonly sarif: string;
}

/**
 * One mandatory diagnostic of a shapes graph — a shape whose `sh:in` or `sh:xone` list is
 * empty — as the structured value every host carries: the rule, then the shape.
 *
 * Like every other wasm-bindgen class in this package, this owns wasm memory and is
 * released with `.free()`.
 */
export class ShaclDiagnostic {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The syntax rule's id: `in-minListLength` or `xone-minListLength` (SHACL 1.2 Core,
     * Appendix A).
     */
    readonly rule: string;
    /**
     * The shape whose list is empty, as an N-Triples term (`<iri>` or `_:label`).
     */
    readonly shape: string;
}

/**
 * The outcome of `shaclEntail`: the materialized dataset and the shapes graph's
 * mandatory diagnostics.
 *
 * Like every other wasm-bindgen class in this package, this owns wasm memory and is
 * released with `.free()`.
 */
export class ShaclEntailment {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The shapes graph's mandatory diagnostics, one `{ rule, shape }` per shape with an
     * empty `sh:in` or `sh:xone` list, which every run reports.
     */
    readonly diagnostics: ShaclDiagnostic[];
    /**
     * The MATERIALIZED dataset — the base graph plus every inferred triple — as canonical
     * N-Triples.
     */
    readonly ntriples: string;
}

/**
 * A shapes graph's `owl:imports` closure is not in hand, or the import table cannot be
 * used — the one refusal every shapes-graph entry point raises, on every PurRDF host
 * alike.
 *
 * Every function that takes a Turtle shapes graph takes the caller's import table as two
 * trailing parallel arrays, `importIris` and `importDocuments` — entry `i` declares that
 * the ontology IRI `importIris[i]` names the Turtle document `importDocuments[i]`, parsed
 * with that IRI as its base; the same convention `entailCertainAnswers` uses. Omitted,
 * the table is empty, and the rule still applies. An `owl:imports` is an import only on
 * the shapes graph's own IRI (`shapesBase`, or the document's own `@base`), on an
 * `owl:Ontology` header, on a `sh:ShapesGraph` (`sh:RulesGraph` and subclasses included),
 * or on a node naming one of those as its `owl:versionIRI`; on any other node — one that is
 * only a `sh:DataGraph` among them — it is data. An import is resolved by a table entry, by
 * `shapesBase` (or the document's own `@base`) naming the imported document, or by the
 * closure declaring it (`<X> a owl:Ontology`, `<X> a sh:ShapesGraph`, or an ontology whose
 * `owl:versionIRI` is `<X>`). Anything
 * else rejects with this class rather than validating a smaller shapes graph than the one
 * named. PurRDF fetches nothing.
 *
 * `kind` is the matchable half — `unresolved-import`, `unreached-import` (a table entry
 * neither an import nor a data-graph link names), `incompatible-import-versions` (the
 * closure holds two versions of one series, or a graph another declares
 * `owl:incompatibleWith`, SHACL 1.2 Core sections 1.3 and 6.1), `invalid-import` (a key that is not an
 * absolute IRI, a key named twice, or a document that is not Turtle),
 * `unresolved-shapes-graph-link` (the data graph links a graph with `sh:shapesGraph`,
 * SHACL 1.2 Core section 6.4, that nothing in hand resolves — pass it in the import
 * table), `unheld-shapes-graph-link` (a prepared product does not hold a graph the data
 * graph links) or `invalid-shapes-graph-link` (a data-graph `sh:shapesGraph` value that
 * is not an IRI) — and `iris` the IRIs (or values) it names. `message` is
 * prose; do not match on it. Like every other wasm-bindgen class in this package, a
 * caught instance owns wasm memory and is released with `e.free()`.
 */
export class ShaclImportError {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The engine's own rendering, which leads with the kind label.
     */
    toString(): string;
    /**
     * The IRIs the refusal names.
     */
    readonly iris: string[];
    /**
     * `unresolved-import`, `unreached-import`, `incompatible-import-versions`,
     * `invalid-import`, `unresolved-shapes-graph-link`, `unheld-shapes-graph-link` or
     * `invalid-shapes-graph-link`.
     */
    readonly kind: string;
    /**
     * The engine's explanation, naming the remedy.
     */
    readonly message: string;
}

/**
 * A SHACL request an asynchronous twin hands `AsyncJob.beginShacl`: built by the
 * constructor named for its synchronous entry, over exactly that entry's
 * positional arguments, and consumed by the job that runs it.
 */
export class ShaclJobRequest {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    static applyRules(data_nt: string, shapes_ttl?: string | null, srl?: string | null, shapes_base?: string | null, srl_base?: string | null, explain?: boolean | null, import_iris?: string[] | null, import_documents?: string[] | null, shapes_graph?: string | null, max_term_generating_rounds?: bigint | null, max_generated_terms?: bigint | null, max_stored_facts?: bigint | null, max_join_steps?: bigint | null): ShaclJobRequest;
    static entail(shapes_ttl: string, data_nt: string, shapes_base?: string | null, import_iris?: string[] | null, import_documents?: string[] | null, shapes_graph?: string | null, max_term_generating_rounds?: bigint | null, max_generated_terms?: bigint | null, max_stored_facts?: bigint | null, max_join_steps?: bigint | null): ShaclJobRequest;
    static evalNodeExpr(shapes_ttl: string, data_nt: string, expr: string | null | undefined, focus: string, scope?: string[] | null, shapes_base?: string | null, import_iris?: string[] | null, import_documents?: string[] | null, expr_at?: string | null, expr_via?: string[] | null, expr_turtle?: string | null): ShaclJobRequest;
    static productValidateToSarif(product: Uint8Array, data_nt: string): ShaclJobRequest;
    static productValidateToSarifExpecting(product: Uint8Array, data_nt: string, expect_identity: string): ShaclJobRequest;
    static productValidateToSarifRebuild(product: Uint8Array, data_nt: string): ShaclJobRequest;
    static productValidateToSarifRebuildExpecting(product: Uint8Array, data_nt: string, expect_identity: string): ShaclJobRequest;
    static validateChangesToSarif(shapes_ttl: string, data_nt: string, added_nt?: string | null, removed_nt?: string | null, shapes_base?: string | null, import_iris?: string[] | null, import_documents?: string[] | null, shapes_graph?: string | null): ShaclJobRequest;
    static validateToSarif(shapes_ttl: string, data_nt: string, shapes_base?: string | null, conformance_disallows?: string[] | null, import_iris?: string[] | null, import_documents?: string[] | null, shapes_graph?: string | null, subclass_of_in_shapes_graph?: boolean | null): ShaclJobRequest;
}

/**
 * The outcome of `shaclLintShapes`: the cold-certify report of a shapes graph.
 *
 * Like every other wasm-bindgen class in this package, this owns wasm memory and is
 * released with `.free()`.
 */
export class ShaclLintReport {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Whether the report carries no finding: the loader accepted the graph, every
     * `shacl-shacl.ttl` result is superseded (flagged there, well-formed SHACL 1.2 Core),
     * no unexecuted query violates a pre-binding restriction and no mandatory diagnostic
     * applies.
     */
    readonly clean: boolean;
    /**
     * The finding count: one for a load refusal, plus every `shacl-shacl.ttl` result no
     * supersession covers, plus every unexecuted query that violates a pre-binding
     * restriction, plus every mandatory diagnostic (an empty `sh:in` or `sh:xone` list).
     */
    readonly findings: number;
    /**
     * The loader's refusal, or `undefined` when it accepted the graph.
     */
    readonly loadError: string | undefined;
    /**
     * The whole report as the deterministic text every PurRDF host prints: the `load`,
     * `shacl-shacl` (`result …` lines, `superseded NAME` where SHACL 1.2 Core makes the
     * flagged graph well-formed), `functions` (`call BINDING <IRI> in OWNER`),
     * `validators` (`alternative <COMPONENT> <ATTACHMENT> VALIDATOR LANGUAGE
     * superseded-by-native`, one per validator declared for a built-in component) and
     * `unexecuted` (`violation DECLARATION`, one per query that violates a pre-binding
     * restriction and that nothing executes), `diagnostics` (`diagnostic RULE SHAPE`, one
     * per shape whose `sh:in` or `sh:xone` list is empty, `RULE` being
     * `in-minListLength` or `xone-minListLength`; each a finding, and the
     * `shacl-shacl` warning on the same list is marked `diagnosed RULE` and not counted) and
     * `unanchored-imports` (`unanchored
     * SUBJECT OBJECT document -|<IRI>`, one per `owl:imports` triple of the closure whose
     * subject is no anchor of its document, so it is data and imported nothing; never a
     * finding) sections, then `findings N` and `clean true|false`.
     */
    readonly report: string;
}

/**
 * The outcome of `shaclEvalNodeExpr`: the expression's output nodes and the shapes graph's
 * mandatory diagnostics.
 *
 * Like every other wasm-bindgen class in this package, this owns wasm memory and is
 * released with `.free()`.
 */
export class ShaclNodeExprOutcome {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The shapes graph's mandatory diagnostics — one `{ rule, shape }` per shape of its
     * `owl:imports` closure with an empty `sh:in` or `sh:xone` list — which every run
     * reports. They change no output.
     */
    readonly diagnostics: ShaclDiagnostic[];
    /**
     * The output nodes, as N-Triples 1.2 terms in the order the expression's sequence
     * semantics define.
     */
    readonly outputs: string[];
}

/**
 * A refusal from the prepared-shapes-product admission boundary, thrown by the four
 * `shaclProduct*` functions.
 *
 * `dimension` is the stable, matchable half: one of the codec's pinned kebab-case
 * labels (`magic`, `format-version`, `stage-id`, `profile`, `truncated`, `trailer`,
 * `section-digest`, `container-digest`, `dataset-identity`, `shapes-graph`,
 * `prefixes`, `base`, `vocabulary`, `function-registry`, `aggregate-registry`,
 * `property-function-registry`, `class-catalog`, `parse-configuration`,
 * `included-graphs`, `unsupported-capability`, `depth-limit`, `malformed`), or
 * `undefined` when the failure happened before any
 * product existed — a shapes or data DOCUMENT that did not parse was never admitted,
 * and naming a dimension for it would claim a product was inspected when none was.
 *
 * `code` is the stable code of a refusal that is not about the product at all: a
 * native XPath resource refusal of the selected `xpathRegex` law carries its resource's
 * own code (`xpath-pattern-bytes`, `xpath-match-steps`, ...), exactly as a SPARQL
 * entry's `Error` does; every other refusal carries `undefined`.
 *
 * `message` is prose for a human: it names the action that resolves the refusal,
 * not only the condition that caused it. Do not match on it.
 */
export class ShaclProductRefusal {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * `<dimension>: <message>`, or the message alone when there is no dimension —
     * the codec's own rendering, so a host that only logs still sees the label.
     */
    toString(): string;
    /**
     * A native XPath resource refusal's own code (`xpath-pattern-bytes`, ...), or
     * `undefined`.
     */
    readonly code: string | undefined;
    /**
     * The admission dimension that refused, or `undefined`.
     */
    readonly dimension: string | undefined;
    /**
     * The prescriptive explanation, without the dimension label.
     */
    readonly message: string;
}

/**
 * The outcome of `shaclCheckRules`: a SPARQL 1.2 RL rule set that passed every check
 * its level asks for.
 *
 * Like every other wasm-bindgen class in this package, this owns wasm memory and is
 * released with `.free()`.
 */
export class ShaclRulesCheck {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The number of data-block triples of the combined rule set.
     */
    readonly dataTriples: number;
    /**
     * The IRIs of the imported rule sets, each once, in the order they were read.
     */
    readonly imported: string[];
    /**
     * The level the rule set passed: `"syntax"`, `"well-formed"` or `"stratified"`.
     */
    readonly level: string;
    /**
     * Every rule of the combined rule set — the rule set's own, then every imported
     * rule — described by its IRI or by where it is written.
     */
    readonly rules: string[];
    /**
     * The number of stratification layers, or `undefined` below `"stratified"`, which
     * does not stratify.
     */
    readonly strata: number | undefined;
    /**
     * The one-line summary every PurRDF host reports.
     */
    readonly summary: string;
    /**
     * The `VERSION` labels, in document order.
     */
    readonly versions: string[];
}

/**
 * The outcome of `shaclApplyRules`: the inference graph, and its proof when one was
 * asked for.
 *
 * Like every other wasm-bindgen class in this package, this owns wasm memory and is
 * released with `.free()`.
 */
export class ShaclRulesInference {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The shapes graph's mandatory diagnostics, one `{ rule, shape }` per shape with an
     * empty `sh:in` or `sh:xone` list, which every run reports; empty for an `srl` rule
     * set, which has no shapes graph.
     */
    readonly diagnostics: ShaclDiagnostic[];
    /**
     * The INFERENCE GRAPH — the inferred triples only, never the data graph — as
     * N-Triples 1.2, one triple per line, in canonical order.
     */
    readonly inferred: string;
    /**
     * The proof of every inferred triple, or `undefined` when `explain` was not set:
     * `derived S P O .`, then `  rule R` and one `  premise S P O .` per fact the rule's
     * body matched, or `  data-block` for a SPARQL 1.2 RL data-block triple.
     */
    readonly proof: string | undefined;
}

/**
 * One invocation a `SILENT` clause absorbed: a `SERVICE SILENT` that answered the single
 * empty solution, or a `LOAD SILENT` that succeeded with nothing loaded.
 */
export class SilencedInvocation {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The `SERVICE` endpoint, or `undefined` for a `LOAD`. A variable endpoint bound to
     * a term that is not an IRI reports that term in N-Triples form.
     */
    readonly endpoint: string | undefined;
    /**
     * The `LOAD` source IRI, or `undefined` for a `SERVICE`.
     */
    readonly iri: string | undefined;
    /**
     * Why the invocation failed: `"transport"`, `"decode"`, `"disabled"`,
     * `"unconfigured"`, `"denied"`, `"host-denied"`, `"not-an-iri"` or `"fault"`.
     */
    readonly kind: string;
    /**
     * The failure's message, as the error would have read without `SILENT`.
     */
    readonly message: string;
    /**
     * `"service"` or `"load"`.
     */
    readonly target: string;
}

/**
 * An RDF/JS `Sink` — a streaming consumer that interns pushed quads through the
 * `purrdf-events` protocol and freezes them at `finish()`.
 */
export class Sink {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * `finish()` — run the protocol's forward-reference resolution and return the
     * resulting dataset. The sink is consumed; further `push`/`finish` is an error.
     */
    finish(): Dataset;
    /**
     * `new Sink()` — an empty sink ready to accept quads via `push`.
     */
    constructor();
    /**
     * `push(quad)` — stream one quad into the sink (interned via the event protocol).
     */
    push(quad: Quad): void;
}

/**
 * One SPARQL 1.1 Protocol operation read from an HTTP request.
 *
 * `SparqlProtocolRequest.parse(method, contentType, queryString, body)` reads `GET
 * ?query=`, `POST application/sparql-query`, `POST application/sparql-update` and `POST
 * application/x-www-form-urlencoded` (`query=` / `update=`), with the dataset parameters
 * `default-graph-uri` / `named-graph-uri` (queries) and `using-graph-uri` /
 * `using-named-graph-uri` (updates). A malformed request throws the typed `Error` the
 * module documentation describes.
 */
export class SparqlProtocolRequest {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The operation text to evaluate: the request's dataset parameters applied (the
     * query's own `FROM`/`FROM NAMED` replaced; `USING`/`USING NAMED` written into each
     * update operation with a `WHERE`), and the whole text parsed under the engine's own
     * reading, so a malformed operation is refused here as the client error it is.
     *
     * # Errors
     *
     * The typed protocol refusal (`MalformedOperation`, `UsingConflictsWithClause`).
     */
    effectiveText(): string;
    /**
     * The media type of a format token `negotiate` returns, or `undefined` for any other
     * string.
     */
    static formatMediaType(token: string): string | undefined;
    /**
     * The format token to answer this query in, negotiated from an `Accept` header
     * against the query's result kind (RFC 9110 §12.5.1; absent or empty means the
     * protocol's default), or `undefined` when no offered format is acceptable — the
     * host's `406`.
     *
     * # Errors
     *
     * `MalformedOperation` as [`Self::result_kind`]; and an update, which has no result
     * to negotiate.
     */
    negotiate(accept?: string | null): string | undefined;
    /**
     * Why a result of `kind` cannot be sent to a client whose `Accept` header allows none
     * of the formats that carry it — the `detail` of a `406`, naming those formats.
     *
     * # Errors
     *
     * Any `kind` other than `"solutions"`, `"boolean"`, `"graph"` and `"dataset"`.
     */
    static notAcceptableDetail(kind: string): string;
    /**
     * The media types a result of `kind` (`"solutions"`, `"boolean"`, `"graph"`, or
     * `"dataset"` — a graph carrying named graphs) is offered in, in server preference
     * order.
     *
     * # Errors
     *
     * Any other `kind`.
     */
    static offeredMediaTypes(kind: string): string[];
    /**
     * Read one protocol operation from an HTTP request's method, `Content-Type` header,
     * query string (without the leading `?`) and body bytes.
     *
     * # Errors
     *
     * The typed protocol refusal: `name` is the `ProtocolError` variant, `status` the
     * HTTP status, `parameter` the parameter at fault when there is one.
     */
    static parse(method: string, content_type: string | null | undefined, query_string: string | null | undefined, body: Uint8Array): SparqlProtocolRequest;
    /**
     * The HTTP problem a failed operation is answered with: `error` is what it rejected
     * with, read for its `code` and `message`; `cancelled` says the request's own signal
     * stopped it, whatever it rejected with. An error without a string `code` is an
     * exception nothing classified — the host's own fault.
     */
    static problemFor(error: any, cancelled: boolean): FailureProblem;
    /**
     * The HTTP problem a governed operation a governor stopped is answered with:
     * `tripped` is its outcome's `tripped` record. A ceiling reached or refused at
     * admission is a `422` carrying its `dimension`, `limit` and `consumed` or
     * `estimate`; a stop signal is the `503` of its `cause`. The problem's `code` is the
     * governor's label.
     *
     * # Errors
     *
     * A record that describes no governor this build names.
     */
    static problemForTrip(tripped: any): FailureProblem;
    /**
     * The reason phrase of an HTTP `status` (RFC 9110 §15) — an `about:blank`
     * problem's `title` — or `undefined` for a status no problem is answered with here.
     */
    static statusTitle(status: number): string | undefined;
    /**
     * Every `default-graph-uri`, in request order.
     */
    readonly defaultGraphUris: string[];
    /**
     * Every parameter the protocol does not define, as flattened `[name, value, name,
     * value, …]` pairs in request order (query string first, then a form body).
     */
    readonly extraParameters: string[];
    /**
     * Whether the request carries dataset parameters.
     */
    readonly hasDatasetParameters: boolean;
    /**
     * `"query"` or `"update"`.
     */
    readonly kind: string;
    /**
     * Every `named-graph-uri`, in request order.
     */
    readonly namedGraphUris: string[];
    /**
     * The query's result kind — `"solutions"` (`SELECT`), `"boolean"` (`ASK`) or `"graph"`
     * (`CONSTRUCT`/`DESCRIBE`) — read from its query form, or `undefined` for an update.
     *
     * # Errors
     *
     * The typed protocol refusal `MalformedOperation` when no query form follows the
     * prologue.
     */
    readonly resultKind: string | undefined;
    /**
     * The operation text as the request carried it, before any dataset parameter is
     * applied.
     */
    readonly text: string;
    /**
     * Every `using-graph-uri`, in request order.
     */
    readonly usingGraphUris: string[];
    /**
     * Every `using-named-graph-uri`, in request order.
     */
    readonly usingNamedGraphUris: string[];
}

/**
 * What `purrdf_jspi_suspend` reports when the host resumes the job.
 */
export enum SuspendStatus {
    /**
     * The effect was answered.
     */
    Answered = 0,
    /**
     * The host abandoned the effect on the job's stop signal.
     */
    Abandoned = 1,
    /**
     * A fault is latched on the job.
     */
    Fault = 2,
}

/**
 * An RDF/JS [Term](https://rdf.js.org/data-model-spec/#term-interface).
 */
export class Term {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Structural RDF/JS term equality.
     */
    equals(other: Term): boolean;
    /**
     * `datatype` — the literal's datatype as a `NamedNode`, or `undefined` for a
     * non-literal.
     */
    readonly datatype: Term | undefined;
    /**
     * `direction` — the RDF-1.2 base direction (`"ltr"`/`"rtl"`), or `""` when absent.
     * The deliberate extension to stock RDF/JS (`.goals`: overcome, don't inherit).
     */
    readonly direction: string;
    /**
     * `graph` of a quoted-triple term — always the default graph (a quoted triple has
     * no graph slot), else `undefined`.
     */
    readonly graph: Term | undefined;
    /**
     * `language` — the literal's language tag, or `""` for a non-language-tagged term
     * (RDF/JS uses the empty string, not `undefined`).
     */
    readonly language: string;
    /**
     * `object` of a quoted-triple term, else `undefined`.
     */
    readonly object: Term | undefined;
    /**
     * `predicate` of a quoted-triple term as a `NamedNode`, else `undefined`.
     */
    readonly predicate: Term | undefined;
    /**
     * `subject` of a quoted-triple term (`termType: "Quad"`), else `undefined`.
     */
    readonly subject: Term | undefined;
    /**
     * `termType` — the RDF/JS discriminator.
     */
    readonly termType: string;
    /**
     * `value` — the IRI, blank label, lexical form, or variable name. Empty for a
     * quoted triple and the default graph (per RDF/JS).
     */
    readonly value: string;
}

/**
 * The governor that stopped one execution: which one, on which dimension, against which
 * ceiling.
 */
export class TrippedGovernor {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Which stop signal fired — `"cancelled"` or `"deadline-exceeded"` — or `undefined`
     * when a ceiling rather than a signal stopped the execution.
     */
    readonly cause: string | undefined;
    /**
     * Consumption charged before the refused work — a **measurement**, and present only
     * on the `"budget"` kind.
     */
    readonly consumed: bigint | undefined;
    /**
     * The governed dimension, e.g. `"fuel"` — `undefined` when a stop signal fired, which
     * belongs to no dimension.
     */
    readonly dimension: string | undefined;
    /**
     * The planner's estimate that exceeded the ceiling — **not** a measurement, and
     * present only on the `"refused"` kind, where nothing ran to measure.
     */
    readonly estimate: bigint | undefined;
    /**
     * Which kind of governor stopped the execution: `"budget"` (a ceiling was reached),
     * `"stopped"` (a stop signal fired), or `"refused"` (the planner's estimate already
     * exceeded a ceiling, so nothing ran).
     *
     * # The wildcard arm, here and on every accessor below
     *
     * The kernel's `TrippedGovernor` is `#[non_exhaustive]`, so this crate — foreign to
     * the one that defines it — must carry a wildcard even though the enum is exhaustive
     * today. A governor a future kernel adds and this build cannot name therefore reads
     * `"unknown"` here and `undefined` on every field accessor, rather than being
     * silently folded into a kind it is not. `label` and `message` still describe it
     * exactly, because both come from the kernel rather than from this match.
     */
    readonly kind: string;
    /**
     * The stable kebab-case discriminant, e.g. `"deadline-exceeded"`. A pinned contract:
     * match on this rather than on the prose of `message`.
     */
    readonly label: string;
    /**
     * The inclusive ceiling in force, or `undefined` when a stop signal fired.
     */
    readonly limit: bigint | undefined;
    /**
     * A human-readable rendering. Prose, not a contract — match on `label`.
     */
    readonly message: string;
}

/**
 * The outcome of one governed SPARQL UPDATE.
 *
 * Deliberately not a [`QueryOutcome`] and deliberately without a partial arm: a query's
 * partial answer is a certifiable thing, a partial *mutation* is not. A tripped request
 * applied **nothing** — not "not all of it" — and left the dataset exactly as it found it.
 */
export class UpdateOutcome {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Move this request's receipt out. Present on both paths.
     */
    takeEvidence(): GovernorEvidence | undefined;
    /**
     * Move the tripped governor out, or `undefined` when the request applied.
     */
    takeTripped(): TrippedGovernor | undefined;
    /**
     * Whether every operation of the request applied.
     *
     * `false` means **nothing** applied, never "not all of it applied".
     */
    readonly isApplied: boolean;
}

/**
 * Seed the terminal counter once, then exercise its real typed refusal through JS.
 */
export function __purrdf_test_exchange_terminal(seed: boolean): bigint;

/**
 * Unit-test-module fixture only: seed the real exported getters beyond binary64's
 * exact integer range without adding a seed operation to the release package.
 */
export function __purrdf_test_large_dataset_identity(): Dataset;

/**
 * Actual ABI fixtures belong only to the unit-test module, never the package.
 * Seed the real allocator temporarily so adjacent large IDs can be settled from JS.
 */
export function __purrdf_test_open_exchange(id: bigint): AsyncEffect;

/**
 * The size of every job's stack region, in bytes: the module's own shadow stack's.
 *
 * The package root reads it once, when it installs the asynchronous lane, and refuses to
 * install the lane when this throws.
 *
 * # Errors
 *
 * The module's layout is not the stack-first one the lane assumes; the error carries the
 * host-fault code and names the layout that was found.
 */
export function asyncStackRegionBytes(): number;

/**
 * `entailCertainAnswers(regime, document, pattern, importIris, importDocuments, premiseIris,
 * maxStoredFacts?, maxJoinSteps?)` → the substitutions the knowledge
 * base ENTAILS the pattern under, as `var` and `row` lines.
 *
 * A certain answer is true in every model, not merely present in one closure, which is
 * what SPARQL's entailment regimes define the answers to a basic graph pattern to be.
 *
 * `pattern` is N-Triples with `?name` in any position, the PREDICATE included. A blank
 * node in it is a NON-DISTINGUISHED variable — constrained by the match, not projected, and
 * not a column — which is what SPARQL says a query blank node is. A variable inside an RDF
 * 1.2 triple term is an ordinary variable: it binds, it is a column, and one NAME is one
 * VARIABLE wherever it was written, so a pattern using it above and below the triple-term
 * boundary is joined rather than split into two. A predicate variable is
 * projected like any other, and under `owl-rl` it also renders a `limit`: it ranges over the
 * whole predicate vocabulary, including the schema predicates and the constructs the
 * mechanisms beyond the rule table decide, and the closure holds neither.
 *
 * A `limit` line says the row set may not be EXHAUSTIVE. Every row is sound
 * unconditionally; what needs a precondition is the claim about a row that is NOT there,
 * so no `limit` lines is the claim that the row set is complete.
 *
 * The answer opens `mechanism <name>`. A pattern with a projected variable is
 * `strict-table`: the five mechanisms beyond the rule table are not run for one, because
 * a projected variable over what any of them decides is a different question — and that
 * one of them WOULD have been needed arrives as a `limit` line naming the lane, never as
 * an exhaustive empty answer. A pattern with NO projected variable is a conclusion graph,
 * is answered by the same fold `entailGraphEntails` runs, and names whichever of the seven
 * reached it; such an answer is the relation with no columns, so a `yes` is one bare `row`
 * line and a `no` is none.
 *
 * `importIris` and `importDocuments` are the caller's `owl:imports` table, as two PARALLEL
 * arrays of the same length: entry `i` declares that the ontology IRI `importIris[i]`
 * denotes the N-Quads document `importDocuments[i]`. A premise carrying an `owl:imports`
 * states that its axioms are its own PLUS those of the documents it names, so this is where
 * those documents arrive and the `owl:imports` triple stays exactly where the caller wrote
 * it. **PurRDF fetches nothing**: an ontology IRI the table does not resolve, and the premise
 * does not already hold (`<X> a owl:Ontology`, `<X> a sh:ShapesGraph`, or an
 * `owl:versionIRI` naming it), throws by name, never a network access and never a silently empty import; an entry the
 * premise's import closure never names throws too, since it would be read and never used.
 * Two empty arrays are the
 * ordinary "imports nothing" case, and both are required rather than defaulted.
 *
 * `premiseIris` are the IRIs the premise DOCUMENT was read from — its retrieval IRI, or the
 * base it was parsed under, when the host knows one. Each is the premise's own IRI: an
 * `owl:imports` on one of these, like one on the premise's `owl:Ontology` header or on a
 * `sh:ShapesGraph` it declares (`sh:RulesGraph` and subclasses included), is an import,
 * while an `owl:imports` on any other node — one that is only a `sh:DataGraph` among them —
 * is a premise triple and names no document. An `owl:imports` of one of these names the premise itself, so it is resolved
 * in place rather than thrown as missing. A host
 * handed bare text has no such IRI, and the empty array is that ordinary case; like the
 * import table it is required rather than defaulted, in the same position on every host.
 *
 * Throws on an unknown regime, on `owl-direct` or `rif` (each defined by an input this
 * signature does not carry), on a malformed document, pattern or import document, on import
 * arrays of different lengths, on a duplicate or empty import IRI, on a pattern that names
 * a graph, on a pattern that writes a variable in a literal's DATATYPE — a slot RDF reserves
 * for an IRI, and one a basic graph pattern has no binding to project — on an `owl:imports`
 * the table does not resolve and the premise does not hold, and on an inconsistent premise — whose refusal carries the
 * full report.
 */
export function entailCertainAnswers(regime: string, document: string, pattern: string, import_iris: string[], import_documents: string[], premise_iris: string[], max_stored_facts?: bigint | null, max_join_steps?: bigint | null): ReasoningAnswer;

/**
 * `entailCheckAbsentProof()` — prove that an answer nobody asked to record is NOT
 * presentable as a verified one, and throw if it is.
 *
 * The companion to [`entail_check_proof_golden_vectors`], covering the path an artifact
 * cannot: there is no proof to pair an input with, and the property under test is that the
 * ABSENCE says so. An unrecorded answer must carry `availability not-recorded`,
 * `entailCheckProof` must refuse that document by name, and the same question asked WITH a
 * proof must check — so the refusal is the absence rather than a broken checker.
 *
 * Throws naming which of the three failed.
 */
export function entailCheckAbsentProof(): void;

/**
 * `entailCheckGoldenVectors()` — run the committed tri-host golden vector
 * artifact through this build and throw on the first byte that differs.
 *
 * This is the wasm half of the load-bearing assertion. The artifact
 * (`crates/validate/tests/fixtures/regime-boundary.vectors`) is compiled into
 * the module by `purrdf-validate`, and the C-ABI crate's Rust test, the
 * `purrdf-validate` Rust test and this entry point all call the SAME checker
 * over the SAME bytes. Running it here executes the entailment chase, the
 * RDFC-1.0 canonical serializer and the report renderer on `wasm32` — different
 * pointer width, different `usize`, different map iteration — and compares the
 * result byte for byte against what the native build produced. A host that
 * diverges fails here in the same words it fails natively.
 *
 * It ships in the released module deliberately: a consumer can prove the wasm
 * they actually loaded agrees with the reference implementation, without
 * trusting this repository's CI.
 *
 * Throws with the case name and a diff of the two strings on any mismatch.
 */
export function entailCheckGoldenVectors(): void;

/**
 * `entailCheckInconsistentRefusal()` — prove that an INCONSISTENT input is refused
 * with its certificate and its witness triples, and throw if it is not.
 *
 * The companion to [`entail_check_golden_vectors`], and the path that artifact cannot
 * cover: an inconsistent knowledge base has no closure, so there is nothing to pair an
 * input with, and the only channel the evidence has is the thrown error's message. That
 * makes it exactly the path on which a host quietly drops the report — as every host but
 * the command line once did, leaving `inconsistency` a constant `none`.
 *
 * The C-ABI crate's Rust test and the `purrdf-validate` Rust test call the SAME checker.
 *
 * Throws naming the fragment the refusal failed to carry, with the refusal verbatim.
 */
export function entailCheckInconsistentRefusal(): void;

/**
 * `entailCheckProof(document, service, argument, answer, certificate, proof)` → the
 * `purrdf-dl-proof-check 1` report, or a throw naming the rejection.
 *
 * THE CHECKER, and nothing in it trusts the producer: the ontology is parsed from
 * `document`, the question is re-derived from `service` and `argument`, the claims are read
 * back out of `answer`'s own grammar, and the checking context comes from a reverse mapping
 * this call performs itself. The proof supplies the runs and nothing else. An `entails`
 * proof for a different axiom, a proof for a different document, or a genuine proof of some
 * OTHER answer are each refused.
 *
 * `answer` and `certificate` may be empty, and each empty one is a WEAKER check that says so
 * rather than one that quietly passed: with no answer the report reads `answer not-checked`,
 * and with no certificate a proof carrying a stopping receipt is refused, because there is
 * nothing for the receipt to be a receipt of.
 *
 * Throws for a proof document that says `availability not-recorded` — an answer nobody asked
 * to record is never presented as a verified one — and for every other rejection.
 */
export function entailCheckProof(document: string, service: string, argument: string, answer: string, certificate: string, proof: string): string;

/**
 * `entailCheckProofGoldenVectors()` — run the committed proof golden-vector artifact through
 * this build and throw on the first byte that differs.
 *
 * The wasm half of the CROSS-HOST byte-stability assertion for the proof surface. The
 * artifact (`crates/validate/tests/fixtures/dl-proof.vectors`) is compiled into the module by
 * `purrdf-validate`, and the C-ABI crate's Rust test, the PyO3 crate's Rust test, the
 * `purrdf-validate` Rust test and this entry point all call the SAME checker over the SAME
 * bytes. Running it here produces and verifies real proof terms on `wasm32` — different
 * pointer width, different `usize` — and compares the rendered proof byte for byte against
 * what the native build produced. Since the rendered proof carries `ServiceProof::encode`'s
 * canonical bytes, a divergence here is a divergence in the PROOF TERM, not in a rendering.
 *
 * Throws with the case name and a diff of the two strings on any mismatch.
 */
export function entailCheckProofGoldenVectors(): void;

/**
 * `entailClassify(document, stepCap, workCap)` → the entailed subsumption hierarchy over
 * the ontology's named classes.
 *
 * The answer carries `equivalent`, `subclass` (the full transitively-closed
 * relation), `direct` (its transitive reduction) and `unsatisfiable` lines, in
 * that block order. Both subsumption blocks are emitted because they are different
 * facts: `direct` is "direct as far as this run decided", which weakens under a
 * `budget-exhausted` certificate while every listed pair stays a genuine
 * subsumption.
 *
 * Throws on a malformed document, or on an ontology with no model — every class
 * then subsumes every other and the hierarchy would be a complete graph.
 */
export function entailClassify(document: string, step_cap: number, work_cap: number): ReasoningAnswer;

/**
 * `entailConsistency(document, importIris, importDocuments, premiseIris, stepCap, workCap)`
 * → is the knowledge base consistent?
 *
 * `importIris`, `importDocuments` and `premiseIris` are [`entail_certain_answers`]'s: OWL 2
 * defines an ontology's imports closure to BE the ontology, so consistency is decided for
 * the ontology merged with every document the table supplies, and the certificate then
 * names `ontology-import-resolved`. An import the table does not resolve, and the ontology
 * does not already hold, throws by name — never a verdict over a smaller ontology — and so
 * does a table entry the closure never reaches.
 *
 * `stepCap` narrows the per-decision tableau step cap; **0 means the knowledge
 * base's own cap**, not a cap of zero steps. It can only NARROW, so it cannot be
 * used to make a hard instance answerable — only to make the `budget-exhausted`
 * certificate reachable from a test.
 *
 * `workCap` narrows the per-decision WORK cap on the same `0`-means-the-knowledge-base's-own-cap
 * rule and narrows only, and it bounds what `stepCap` cannot: a round is a PASS over the
 * completion graph rather than a unit of cost, so an ontology can make each round enormously
 * more expensive without taking more rounds. A run that reaches it answers `unknown` with
 * `work` equal to `work-budget` in its certificate.
 *
 * The one DL service that answers for an unsatisfiable ontology, because it is the
 * one that detects one; every other throws rather than returning the vacuous
 * answer an ontology with no model gives.
 *
 * Throws if `document` fails to parse, on an import-table refusal, or if the reverse
 * mapping fails.
 */
export function entailConsistency(document: string, import_iris: string[], import_documents: string[], premise_iris: string[], step_cap: number, work_cap: number): ReasoningAnswer;

/**
 * `entailEntails(document, axiom, stepCap, workCap)` → does the ontology entail `axiom`?
 *
 * `axiom` is ONE triple of the OWL 2 RDF mapping, in N-Triples syntax. Seven
 * reserved predicates select the seven named axiom kinds — `rdfs:subClassOf`,
 * `owl:equivalentClass`, `owl:disjointWith`, `rdf:type`, `owl:sameAs`,
 * `owl:differentFrom`, `rdfs:subPropertyOf` — and any other predicate is an
 * object-property assertion. No encoding is invented: this is the mapping the
 * reasoner's own reverse mapping reads.
 *
 * The answer is `entails true|false|unknown` followed by the axiom as it was READ,
 * so a caller can see which axiom its predicate selected. `unknown` is never
 * collapsed to `false`: it means the search reached its step cap or its work cap, which
 * the certificate says in its own words.
 *
 * Throws on a malformed document, an `axiom` that is not one triple, or an
 * ontology with no model.
 */
export function entailEntails(document: string, axiom: string, step_cap: number, work_cap: number): ReasoningAnswer;

/**
 * `entailExplainConclusion(document, regime, conclusion)` → WHY one triple of a
 * chase closure holds: which rules, from which premises.
 *
 * `conclusion` is ONE N-Quads statement; its graph, if it names one, selects the
 * closure to explain.
 *
 * The certificate's `derived-*` lines are what the CHECKER re-derived from the
 * proof term and the clause program — not what the proof claims — so a proof whose
 * stated conclusion its own premises do not license shows up as differing lines
 * rather than a silent `checked true`.
 *
 * Throws for `rdf` and `rdfs`, four of whose rules conclude about a FRESH blank
 * node: an existential head has no Datalog semantics, so a "proof" of such a step
 * could only be believed. Also throws for a conclusion that is neither asserted nor
 * derived — a hard error, because there is nothing to explain and an empty answer
 * would read as though there were.
 */
export function entailExplainConclusion(document: string, regime: string, conclusion: string): ReasoningAnswer;

/**
 * `entailExtensions(regime)` → the rules this build fires BEYOND `regime`'s
 * specification table, one canonical rule name per array entry.
 *
 * Disjoint from both `entailRules(regime)` and `entailImplementedRules(regime)`:
 * the normative table is a statement about the specification and does not move
 * because this build fires a sound rule the table happens not to list. A rendered
 * report names the same rules on its `extension` line — this answers the question
 * without materializing a dataset first.
 *
 * `[]` for a lane with nothing added to it, which is every lane but `owl-rl`.
 *
 * Throws for an unknown regime spelling, naming the accepted set.
 */
export function entailExtensions(regime: string): string[];

/**
 * `entailExtractModule(document, signature, method)` → the locality module of the
 * ontology for a seed signature.
 *
 * `signature` is one N-Triples term per line (blank lines ignored). `method` is
 * `bot`, `top` or `star`; an unknown spelling throws with the accepted set named.
 *
 * The answer is the module as canonical (RDFC-1.0) N-Quads — the same serializer
 * `entailMaterialize` uses. The certificate's `conservative` line says whether the
 * module is the minimal one or a sound SUPERSET, which is what a caller sizing a
 * module needs to know.
 *
 * Throws on a malformed document, a signature line that is not one N-Triples term,
 * an unknown method, or a module that cannot be frozen.
 */
export function entailExtractModule(document: string, signature: string, method: string): ReasoningAnswer;

/**
 * `entailGraphEntails(regime, premise, conclusion, importIris, importDocuments, premiseIris,
 * maxStoredFacts?, maxJoinSteps?)` → does the premise entail the
 * conclusion GRAPH under the regime's rule table?
 *
 * NOT [`entail_entails`], which asks the OWL 2 Direct-Semantics TABLEAU about one AXIOM
 * and renders a `purrdf-dl-certificate 1`. This asks the regime's RULE TABLE about a
 * conclusion GRAPH and renders a `purrdf-reasoning-report 4`. Different question,
 * different calculus, different certificate — and the banners differ so neither can be
 * parsed as the other.
 *
 * The answer opens `mechanism <name>`: WHICH of the seven mechanisms reached the verdict.
 * `strict-table` is the regime's own rule table, run once; five more exist because no head
 * in that table has the conclusion's shape at all; and `composite` is two or more of those
 * five folded over one conclusion.
 *
 * THREE verdicts, never two. `not-entailed` is a PROOF — the procedure was complete for
 * this premise — and `undecided` is what an incomplete procedure is entitled to say
 * instead. Collapsing the second into the first would turn a limitation of this library
 * into a false statement about the caller's data.
 *
 * `importIris`/`importDocuments`/`premiseIris` are [`entail_certain_answers`]'s, and apply to the
 * PREMISE: the conclusion is a graph to match rather than an ontology to close, so an
 * `owl:imports` in it names nothing this service resolves.
 *
 * Throws as [`entail_certain_answers`].
 */
export function entailGraphEntails(regime: string, premise: string, conclusion: string, import_iris: string[], import_documents: string[], premise_iris: string[], max_stored_facts?: bigint | null, max_join_steps?: bigint | null): ReasoningAnswer;

/**
 * `entailImplementedRules(regime)` → the subset of `entailRules(regime)` this
 * build's chase actually fires today.
 *
 * `entailRules(r)` minus `entailImplementedRules(r)` is the regime's measurable
 * gap — the same set the rendered report's `missing` lines name.
 *
 * Throws for an unknown regime spelling, naming the accepted set.
 */
export function entailImplementedRules(regime: string): string[];

/**
 * `entailInstances(document, class, stepCap, workCap)` → the named individuals entailed to
 * be instances of `class`, as `instance <term>` lines.
 *
 * `class` is ONE N-Triples term — `"<http://example.org/Cat>"`, angle brackets
 * included. A class the ontology never mentions is not an error: nothing
 * constrains it, so the empty answer for it is a real answer.
 *
 * Throws on a malformed document, a `class` that is not one N-Triples term, or an
 * ontology with no model.
 */
export function entailInstances(document: string, _class: string, step_cap: number, work_cap: number): ReasoningAnswer;

/**
 * `entailJustify(document, axiom)` → WHY a Description-Logic axiom is entailed: a
 * minimal subset of the ontology that still entails it.
 *
 * A tableau performs no derivation steps, so this is a JUSTIFICATION and
 * deliberately not called a proof; `entailExplainConclusion` is the chase lane's
 * genuinely derivational explanation, and the two are different kinds of thing
 * rather than two spellings of one.
 *
 * The answer is the justification's axioms as canonical N-Quads. The certificate's
 * `sufficient` and `minimal` lines are **re-decided** over the justification alone
 * and over each of its one-axiom-smaller subsets, so they check the answer rather
 * than restate it.
 *
 * Throws if the ontology does not entail the axiom — the empty set reads as
 * "nothing is needed" and means the opposite — or if the tableau could not decide
 * it, leaving no answer to shrink against.
 */
export function entailJustify(document: string, axiom: string): ReasoningAnswer;

/**
 * `entailMaterialize(document, regime, program, importIris, importDocuments, premiseIris,
 * maxStoredFacts?, maxJoinSteps?)` → a `RegimeClosure` carrying the canonical N-Quads
 * closure and the rendered reasoning report.
 *
 * `document` is parsed as N-Quads, which accepts an N-Triples document
 * unchanged, so a document that names a graph keeps naming it. `regime` is one
 * of `simple`, `rdf`, `rdfs`, `owl-rl`, `owl-direct`, `rif`, `d` — the same
 * spellings the CLI, the C ABI and the Python surface accept — and ALL SEVEN
 * materialize.
 *
 * `program` is the regime's own rule document. `rif` entails under the CALLER's
 * rules, so for that spelling `program` is a normative RIF-in-XML document; every
 * other regime's rule table is the specification's, so its `program` is `""`, and a
 * non-empty one throws rather than being silently discarded.
 *
 * `maxStoredFacts` (a `bigint`) bounds the facts each evaluation store may hold and
 * `maxJoinSteps` (a `bigint`) the candidate solutions the rules may enumerate, for the
 * `rdf`, `rdfs`, `owl-rl` and `d` regimes. Omitted, each is this target's default:
 * 131072 facts and 1048576 join steps, sized for one WebAssembly linear memory — a
 * native build's defaults are 4194304 and 1048576. A run past either throws naming
 * the limit, the numbers and the argument that raises it; a run inside them returns
 * exactly the closure larger limits would, and the report's `contract-hash` names the
 * calculus under the limits in force.
 *
 * `importIris`, `importDocuments` and `premiseIris` are [`entail_certain_answers`]'s: OWL 2
 * defines an ontology's imports closure to BE the ontology, so a document carrying an
 * `owl:imports` is closed over the merge of itself and every N-Quads document the table
 * supplies, and the report then states `ontology-import-resolved`. An import the table does
 * not resolve, and the document does not already hold, throws by name — never a closure of
 * a smaller ontology — and so does a table entry the closure never reaches. Empty arrays are
 * the ordinary "imports nothing" case, and all three are required rather than defaulted.
 *
 * Throws if `document` fails to parse, if `regime` is not one of those spellings
 * (the message names the accepted set), if `program` is wrong for the regime, on an
 * import-table refusal, or if the run passes an evaluation limit.
 */
export function entailMaterialize(document: string, regime: string, program: string, import_iris: string[], import_documents: string[], premise_iris: string[], max_stored_facts?: bigint | null, max_join_steps?: bigint | null): RegimeClosure;

/**
 * `entailProfile(document)` → which OWL 2 profiles the ontology is provably in,
 * and what blocked the others.
 *
 * The answer is `certified <profile>` lines, most restrictive first (EL, QL, RL,
 * DL, Full). Purely syntactic — no tableau, no closure, no budget — so the
 * certificate is a `purrdf-owl-profile-certificate 1` block rather than a DL one:
 * there is no search whose completeness could be reported, and rendering a
 * fabricated `decided` would be exactly the overclaim the certificates prevent.
 *
 * The certificate ends `one-directional true`: a certification PROVES membership,
 * a violation does NOT prove non-membership.
 *
 * Throws on a malformed document.
 */
export function entailProfile(document: string): ReasoningAnswer;

/**
 * `entailProve(document, service, argument, stepCap, workCap)` → a `ReasoningAnswer` whose
 * `.proof` is the run's `purrdf-dl-proof 1` document.
 *
 * THE OPT-IN. Every other `entail*` function is unchanged and records nothing, so a caller
 * who does not want a proof runs exactly the search they ran before and pays exactly what
 * they paid before. This one records — which costs the completion graph of every tableau run
 * it keeps — and hands back a document `entailCheckProof` can verify.
 *
 * `service` is one of `consistency`, `class-satisfiability`, `classify`, `realize`,
 * `instances`, `entails`, `extract-module`; an unknown spelling throws with the accepted set
 * named. `argument` is the question's own input in that service's grammar: empty for
 * `consistency`/`classify`/`realize` (a non-empty one throws rather than being discarded),
 * ONE N-Triples term for `class-satisfiability`/`instances`, ONE triple for `entails`, and a
 * `method <bot|top|star>` line followed by one term per line for `extract-module`.
 *
 * `.answer` and `.certificate` are byte-identical to the same question asked without a
 * proof: recording is an observation the reasoner makes of itself, never a lever it reads.
 *
 * `stepCap` and `workCap` are `entailConsistency`'s.
 *
 * Throws on a malformed document, an unknown service, an argument wrong for the service, or
 * whatever that service itself refuses.
 */
export function entailProve(document: string, service: string, argument: string, step_cap: number, work_cap: number): ReasoningAnswer;

/**
 * `entailRealize(document, stepCap, workCap)` → the entailed types of the ontology's named
 * individuals, and the most specific of them (`type` then `direct-type` lines).
 *
 * Throws on a malformed document or an ontology with no model.
 */
export function entailRealize(document: string, step_cap: number, work_cap: number): ReasoningAnswer;

/**
 * `entailRules(regime)` → the rule table the specification *defines* the regime
 * by, one canonical rule name per array entry, in specification table order.
 *
 * `[]` for a regime with no rule table of its own (`simple`, plus `owl-direct`, which
 * decides through the tableau, and `rif`, which entails under the caller's rules —
 * all three still MATERIALIZE). `owl-rl` returns all 78 rules of OWL 2 Profiles §4.3
 * Tables 4–9 whether or not this build fires them — that is the point: compare
 * it with [`entail_implemented_rules`] to measure the gap.
 *
 * Throws for an unknown regime spelling, naming the accepted set.
 */
export function entailRules(regime: string): string[];

/**
 * `entailVerifyEntailment(regime, premise, conclusion, importIris, importDocuments, premiseIris,
 * maxStoredFacts?, maxJoinSteps?)` → [`entail_graph_entails`] with
 * the warrant re-decided, without running a reasoner.
 *
 * The re-check re-derives nothing: "the closure follows from the premise" is the chase's
 * claim and `entailExplainConclusion` is its checker, while "the conclusion follows from
 * the closure" is this one and is finite and purely combinatorial.
 *
 * `warrant absent` / `verified not-applicable` is a `not-entailed` or an `undecided`:
 * there is no evidence to re-decide, and a `false` there would read as a failed check
 * rather than as an absent one.
 *
 * `importIris`/`importDocuments`/`premiseIris` are [`entail_certain_answers`]'s. The re-check runs
 * against the premise AS WRITTEN rather than against its imports closure: a warrant
 * re-decidable from the caller's own document is a stronger check than one only
 * re-decidable against a graph the library assembled.
 *
 * Throws as [`entail_certain_answers`].
 */
export function entailVerifyEntailment(regime: string, premise: string, conclusion: string, import_iris: string[], import_documents: string[], premise_iris: string[], max_stored_facts?: bigint | null, max_join_steps?: bigint | null): ReasoningAnswer;

/**
 * The XPath F&O error codes, `err:`-prefixed (`"err:FOAR0001"`), in the order the
 * evidence's `expressionErrors` vectors are indexed by.
 */
export function expressionErrorCodes(): string[];

/**
 * The kernel's governed resource dimensions, by their stable kebab-case labels, in the
 * kernel's own declaration order.
 *
 * This order is the index order of the [`GovernorEvidence`] consumption and ceiling
 * vectors, so a consumer zips the three together rather than hard-coding a vocabulary the
 * engine owns. Exposed as a function rather than restated in JavaScript for exactly that
 * reason: a dimension added to the kernel appears here without the package root being
 * edited, and cannot silently drop out of a caller's evidence map.
 */
export function governorDimensions(): string[];

/**
 * Runs once, when the instance starts: installs the synchronous shadow stack's floor
 * in [`purrdf_stack`], the measurement the SPARQL evaluator's stack guards refuse
 * against, and the panic hook that poisons the instance (the `panic_poison` module).
 *
 * The floor's default is address 0, the floor rustc's `--stack-first` layout gives the
 * stack; this reads the linker's own record of it instead (`shadow_stack`), so the
 * guards measure against wherever the stack really ends however this module was linked.
 * The asynchronous lane switches away from this floor onto each job's region and back
 * (`async_query`), and puts this value back whenever a job suspends or returns.
 */
export function install_stack_floor(): void;

/**
 * Lift a strict bidirectional USTAR package into an in-memory RDF dataset.
 */
export function liftProjection(archive: Uint8Array, profile: string, config_json: string): ProjectionLift;

/**
 * Decode the additive `purrdf` provenance extension a SPARQL-results JSON document
 * carries under the namespace `prefix`/`iri`, the inverse of `queryRaw`'s
 * `provenanceNamespace` option. A document with no member under `prefix` (never
 * written, or written under a different namespace) decodes to an empty
 * [`ProvenanceInfo`] rather than erroring.
 *
 * # Errors
 *
 * An invalid `prefix`/`iri`, malformed JSON, or a member under `prefix` whose shape
 * does not match the writer's.
 */
export function provenanceFromJson(json: string, prefix: string, iri: string): ProvenanceInfo;

/**
 * Decode the additive `purrdf` provenance extension a SPARQL-results XML document
 * carries under the namespace `prefix`/`iri` — the XML twin of [`provenance_from_json`].
 *
 * # Errors
 *
 * An invalid `prefix`/`iri`, malformed XML, or a non-`<sparql>` root.
 */
export function provenanceFromXml(xml: string, prefix: string, iri: string): ProvenanceInfo;

/**
 * r" `shaclApplyRules(dataNt, shapesTtl?, srl?, shapesBase?, srlBase?, explain?, importIris?,
 * r" importDocuments?, shapesGraph?, maxTermGeneratingRounds?, maxGeneratedTerms?,
 * r" maxStoredFacts?, maxJoinSteps?)` → a `ShaclRulesInference`.
 * r"
 * r" The import table, then `shapesGraph`, then the four rule-evaluation limits in the order
 * r" `shaclEntail` takes them: the two SHACL rule entry points spell their shared parameters
 * r" in one order, so a caller moving between them moves no argument.
 * r"
 * r" Runs exactly one rule source over the N-Triples data graph: the SHACL 1.2 rules of the
 * r" Turtle shapes graph `shapesTtl` (its default rule set), or the SPARQL 1.2 RL rule set
 * r" `srl`. Naming neither or both throws. `shapesBase` / `srlBase` are the documents' base
 * r" IRIs — a guest has no retrieval IRI to derive one from.
 * r"
 * r" `maxTermGeneratingRounds` (a `bigint`) bounds the evaluation rounds that infer a term
 * r" the graph did not hold (default 16384), and `maxGeneratedTerms` (a `bigint`) the terms
 * r" inferred beyond the input's (default max(65536, 4 × N) for N distinct input terms). A
 * r" run past either throws naming the limit, the numbers, the rules that inferred a new
 * r" term last, and the argument that raises it.
 * r"
 * r" `maxStoredFacts` (a `bigint`) bounds the facts the evaluation store may hold — the data
 * r" graph, a rule set's data and every inferred triple — and `maxJoinSteps` (a `bigint`)
 * r" the candidate solutions the rule bodies may enumerate. Omitted, each is this target's
 * r" default: 131072 facts and 1048576 join steps, sized for one WebAssembly linear memory
 * r" (a native build's defaults are 4194304 and 1048576). A run past either throws naming
 * r" the limit, the numbers and the argument that raises it.
 * r"
 * r" `importIris` / `importDocuments` are the rule source's import table: the shapes graph's
 * r" `owl:imports` table (Turtle documents, see [`ShaclImportError`]) beside `shapesTtl`, the
 * r" rule set's `IMPORTS` table (SPARQL 1.2 RL texts) beside `srl`, followed transitively. An
 * r" imported document's rules run. An import no entry supplies, and an entry the import
 * r" closure never names, throw.
 * r"
 * r" `shapesGraph` is the shapes-graph IRI the SHACL rules see the shapes graph under, as
 * r" `purrdf rules --shapes-graph` names it: a `sh:SPARQLRule`'s `$shapesGraph` is pre-bound
 * r" to it and `GRAPH $shapesGraph { … }` reads the shapes graph. A relative one resolves
 * r" against `shapesBase`; omitted, `$shapesGraph` is an ordinary variable. Naming one beside
 * r" `srl` throws: a SPARQL 1.2 RL rule set has no shapes graph.
 * r"
 * r" Throws on a document that does not parse, an ill-formed or unstratifiable rule set, a
 * r" rule failing during execution, and a passed evaluation limit; rejects with a
 * r" [`ShaclImportError`] when the shapes graph's `owl:imports` closure is not in hand. Call
 * r" `.free()` on the result.
 */
export function shaclApplyRules(data_nt: string, shapes_ttl?: string | null, srl?: string | null, shapes_base?: string | null, srl_base?: string | null, explain?: boolean | null, import_iris?: string[] | null, import_documents?: string[] | null, shapes_graph?: string | null, max_term_generating_rounds?: bigint | null, max_generated_terms?: bigint | null, max_stored_facts?: bigint | null, max_join_steps?: bigint | null, xpath_regex?: string | null): ShaclRulesInference;

/**
 * `shaclCheckRules(srl, srlBase?, importIris?, importDocuments?, level?)` → a
 * `ShaclRulesCheck`.
 *
 * Checks the SPARQL 1.2 RL rule set `srl` WITHOUT evaluating it: the grammar, the
 * `IMPORTS` closure resolved from `importIris` / `importDocuments` (rule-set texts, as
 * `shaclApplyRules` takes them), well-formedness and stratification — every static check
 * `shaclApplyRules` applies before it runs — with no data graph read and no rule run.
 * `srlBase` is the rule set's base IRI.
 *
 * `level` is how far the check goes, each level including the ones before it: `"syntax"`
 * (the grammar, for the rule set and every document its imports read), `"well-formed"`
 * (every rule, imported ones included, is well formed) or `"stratified"` (the combined
 * rule set can be stratified); omitted, `"stratified"`. Any other name throws.
 *
 * Throws, naming the stage, for a rule set a check refuses: a syntax error, an import the
 * table does not supply or an entry its closure never names, an ill-formed rule, a rule
 * set that cannot be stratified. Call `.free()` on the result.
 */
export function shaclCheckRules(srl: string, srl_base?: string | null, import_iris?: string[] | null, import_documents?: string[] | null, level?: string | null): ShaclRulesCheck;

/**
 * r" `shaclEntail(shapesTtl, dataNt, shapesBase?, importIris?, importDocuments?,
 * r" shapesGraph?, maxTermGeneratingRounds?, maxGeneratedTerms?, maxStoredFacts?,
 * r" maxJoinSteps?)` → a `ShaclEntailment`: `ntriples`, the materialized dataset as an
 * r" N-Triples string (the base graph plus every inferred triple), and `diagnostics`, the
 * r" shapes graph's mandatory diagnostics (a `ShaclDiagnostic` — `rule`, `shape` — per shape
 * r" with an empty `sh:in` or `sh:xone` list). Call `.free()` on the result.
 * r"
 * r" Its parameters after `dataNt` are `shaclApplyRules`' in the same order: the shapes
 * r" document's base, the import table, `shapesGraph`, then the four rule-evaluation
 * r" limits.
 * r"
 * r" `shapesTtl` is a Turtle shapes graph; `dataNt` is an N-Triples data graph.
 * r" Throws (rejects) if either graph fails to parse or if rule application fails.
 * r"
 * r" `shapesBase` carries the same meaning it does on
 * r" [`shacl_validate_to_sarif`] — the shapes document's own base IRI, supplied by the
 * r" host because a wasm guest has no retrieval IRI to derive one from.
 * r"
 * r" Nothing is dropped on the way out: the underlying writer is the graph-carrying
 * r" canonical N-Quads serializer, and the output is N-Triples because BOTH inputs
 * r" are single-graph syntaxes, not because a graph slot was discarded.
 * r"
 * r" `importIris` / `importDocuments` are the shapes graph's `owl:imports` table (see
 * r" [`ShaclImportError`]): an imported document's rules run.
 * r"
 * r" `shapesGraph` is the shapes-graph IRI the SHACL rules see the shapes graph under, as
 * r" [`shacl_apply_rules`] takes it: a `sh:SPARQLRule`'s `$shapesGraph` is pre-bound to it.
 * r" A relative one resolves against `shapesBase`; omitted, `$shapesGraph` is an ordinary
 * r" variable.
 * r"
 * r" `maxTermGeneratingRounds`, `maxGeneratedTerms`, `maxStoredFacts` and `maxJoinSteps`
 * r" (each a `bigint`) are the four rule-evaluation limits, exactly as
 * r" [`shacl_apply_rules`] takes them and with the same defaults: the evaluation rounds that
 * r" infer a term the graph did not hold (16384), the terms inferred beyond the input's
 * r" (max(65536, 4 × N) for N distinct input terms), the facts the evaluation store may hold
 * r" (131072 on this target) and the candidate solutions the rule bodies may enumerate
 * r" (1048576). A run past one throws naming the limit, the numbers and the argument that
 * r" raises it.
 */
export function shaclEntail(shapes_ttl: string, data_nt: string, shapes_base?: string | null, import_iris?: string[] | null, import_documents?: string[] | null, shapes_graph?: string | null, max_term_generating_rounds?: bigint | null, max_generated_terms?: bigint | null, max_stored_facts?: bigint | null, max_join_steps?: bigint | null, xpath_regex?: string | null): ShaclEntailment;

/**
 * r" `shaclEvalNodeExpr(shapesTtl, dataNt, expr, focus, scope?, shapesBase?, importIris?,
 * r" importDocuments?, exprAt?, exprVia?, exprTurtle?)` → a `ShaclNodeExprOutcome`:
 * r" `outputs`, the output nodes as an array of N-Triples 1.2 terms in the order the
 * r" expression's sequence semantics define, and `diagnostics`, the shapes graph's mandatory
 * r" diagnostics (`{ rule, shape }` per shape with an empty `sh:in` or `sh:xone` list). Call
 * r" `.free()` on the result.
 * r"
 * r" Evaluates ONE node expression of the Turtle shapes graph — SHACL 1.2 Node Expressions'
 * r" `evalExpr(expr, focusGraph, focusNode, scope)` — against a focus node of the
 * r" N-Triples data graph. The expression is named exactly one way: `expr` is an absolute
 * r#" IRI or `"_:label"` for a blank node the shapes document labels so; or `expr` is"#
 * r" `undefined` and `exprAt` names a node and `exprVia` the predicate IRIs a walk from it
 * r" follows, each step reaching exactly one value (how an anonymous `[ … ]` expression is
 * r" named); or `expr` is `undefined` and `exprTurtle` is the expression as a Turtle
 * r" document, read under the shapes document's prefixes and base, whose one root blank node
 * r" is the expression. None or several selectors, a walk step reaching no value or several,
 * r" and an inline document without exactly one root throw. `focus` is an absolute IRI or
 * r" any N-Triples term; `scope` is
 * r#" an array of `"NAME=TERM"` bindings read by `shnex:var "NAME"`, the term spelled as"#
 * r" `focus` is. Throws on a label the shapes document never wrote, a binding named
 * r" `focusNode` or bound twice (neither could ever be read), and any parse or evaluation
 * r" failure. `importIris` / `importDocuments` are the shapes graph's `owl:imports` table
 * r" (see [`ShaclImportError`]); an imported document's functions and shapes are in scope.
 */
export function shaclEvalNodeExpr(shapes_ttl: string, data_nt: string, expr: string | null | undefined, focus: string, scope?: string[] | null, shapes_base?: string | null, import_iris?: string[] | null, import_documents?: string[] | null, expr_at?: string | null, expr_via?: string[] | null, expr_turtle?: string | null, xpath_regex?: string | null): ShaclNodeExprOutcome;

/**
 * `shaclLintShapes(shapesTtl, shapesBase?, importIris?, importDocuments?, shapesGraph?)`
 * → a `ShaclLintReport`.
 *
 * Certifies a Turtle shapes graph COLD — its whole `owl:imports` closure, resolved
 * against `importIris` / `importDocuments` (see [`ShaclImportError`]): the loader's
 * verdict, every result of validating it against the W3C `shacl-shacl.ttl`, and which
 * implementation every node-expression function call binds to (`native`, `custom`,
 * `sparql-registered`, `host-extension`). Rejects with a [`ShaclImportError`] when the
 * closure is not in hand — never a report about the importing document alone. Otherwise
 * throws only when the document is not Turtle; a malformed shapes graph is a report with
 * findings, not an exception. `shapesGraph` configures the loader as `purrdf shapes lint
 * --shapes-graph` does (see [`shacl_validate_to_sarif`]). Call `.free()` on the result.
 */
export function shaclLintShapes(shapes_ttl: string, shapes_base?: string | null, import_iris?: string[] | null, import_documents?: string[] | null, shapes_graph?: string | null): ShaclLintReport;

/**
 * `shaclPackProduct(shapesTtl, shapesBase?, importIris?, importDocuments?, shapesGraph?)`
 * → the prepared product as a `Uint8Array`.
 *
 * Compile once, restore many times: the product carries the compiled SHACL model AND
 * the shapes dataset it came from, under a per-section SHA-256 and a whole-container
 * digest, plus the binding of every input it was compiled against.
 *
 * Byte-deterministic — no clock, no randomness and no hash-iteration order reach the
 * writer — so two calls over the same shapes graph and base produce identical bytes
 * and a content-addressed cache key over them is stable.
 *
 * `shapesBase` carries the same meaning it does on
 * [`shacl_validate_to_sarif`] — the shapes document's own base IRI, supplied by the
 * host because a wasm guest has no retrieval IRI to derive one from. It is RECORDED
 * in the product, so a restore resolves the same relative references without the
 * document.
 *
 * `importIris` / `importDocuments` are the shapes graph's `owl:imports` table (see
 * [`ShaclImportError`]); the product carries the merged closure, so a restore needs no
 * documents. `shapesGraph` (see [`shacl_validate_to_sarif`]) is recorded in the product
 * and bound by its identity, as `purrdf shacl pack --shapes-graph` records it.
 *
 * Rejects with a [`ShaclImportError`] — the same refusal `shaclValidateToSarif` raises —
 * when the shapes graph's `owl:imports` closure is not in hand, and with a
 * [`ShaclProductRefusal`] otherwise; call `.free()` on either when done.
 */
export function shaclPackProduct(shapes_ttl: string, shapes_base?: string | null, import_iris?: string[] | null, import_documents?: string[] | null, shapes_graph?: string | null): Uint8Array;

/**
 * `shaclProductCertify(product)` → resolves when the product's shapes dataset
 * canonicalizes to the digest its own identity binding claims.
 *
 * The codec's COLD path, and deliberately unreachable from a restore:
 * canonicalization is a graph-isomorphism computation over the shapes graph's blank
 * nodes and can cost more than the shapes parse a product exists to eliminate. Call
 * it from a build step or a test, not before every validation —
 * `shaclProductValidateToSarif` already verifies every section digest and the whole
 * container.
 *
 * Rejects with a [`ShaclProductRefusal`]; call `.free()` on it when done.
 */
export function shaclProductCertify(product: Uint8Array): void;

/**
 * `shaclProductExplain(product)` → what the product says it was compiled from, as
 * deterministic `key value` lines, WITHOUT admitting it.
 *
 * The container format version, the preparation stage id and whether this build knows
 * it, the identity digest and every labelled identity component, then the recorded
 * parse inputs (base, `sh:shapesGraph` IRI, prefix map). This is what makes a named
 * refusal actionable: a restore rejected with `dimension === "prefixes"` is answered
 * by reading which prefix map the product actually carries, not by guessing.
 *
 * Rejects with a [`ShaclProductRefusal`]; call `.free()` on it when done.
 */
export function shaclProductExplain(product: Uint8Array): string;

/**
 * r" `shaclProductValidateToSarif(product, dataNt, xpathRegex?)` → a SARIF 2.1.0 JSON
 * r" string.
 * r"
 * r" The point of a product: restore the preparation instead of re-parsing the shapes
 * r" graph, then validate. The verdict is the identical one
 * r" [`shacl_validate_to_sarif`] reaches over the shapes document the product was packed
 * r" from — the same engine entry point runs, over the same restored `Shapes`.
 * r"
 * r" Admission runs first and in full: the container's framing, every section digest,
 * r" the whole-container digest, then the product's stage id, profile and complete input
 * r" binding, all before a single focus node is resolved. A product prepared under a
 * r" different prefix map, base, vocabulary or registry is REFUSED rather than validated
 * r" into a report about a shapes graph nobody asked for.
 * r"
 * r" `xpathRegex` selects the dated pattern law exactly as [`shacl_validate_to_sarif`]
 * r" takes it; the product itself is unchanged by the selection. The same holds for the
 * r" three other product entries.
 * r"
 * r" Rejects with a [`ShaclProductRefusal`]; call `.free()` on it when done. A malformed
 * r" `dataNt`, a name that selects no law and a native resource refusal reject with
 * r" `dimension === undefined`: none of them is an admission dimension of the product.
 */
export function shaclProductValidateToSarif(product: Uint8Array, data_nt: string, xpath_regex?: string | null): string;

/**
 * r" `shaclProductValidateToSarifExpecting(product, dataNt, expectIdentity, xpathRegex?)` → a SARIF
 * r" 2.1.0 JSON string, but only from the product whose input binding is
 * r" `expectIdentity`.
 * r"
 * r" Everything [`shacl_product_validate_to_sarif`] checks is a question about the
 * r" executing guest — its build, its registries, its class analysis. None of them asks
 * r" whether these are the bytes the host meant, because nothing in a product states
 * r" which product was wanted. A host that fetches a product over the network, reads one
 * r" out of a cache, or builds its path from a configuration string has no other way to
 * r" say so, and admitting the wrong one produces a decided, well-formed SARIF log about
 * r" a shapes graph nobody asked about.
 * r"
 * r" `expectIdentity` is the 64 hexadecimal digits `shaclProductExplain` prints on its
 * r" `identity-digest` line — one spelling, readable off the artifact, so the selector
 * r" can be pinned in a manifest beside the product it names.
 * r"
 * r" Rejects with a [`ShaclProductRefusal`]; call `.free()` on it when done. A product
 * r#" carrying a different binding rejects with `dimension === "shapes-graph"`; an"#
 * r" `expectIdentity` that is not 64 hexadecimal digits rejects with
 * r" `dimension === undefined`, because no product was ever inspected.
 */
export function shaclProductValidateToSarifExpecting(product: Uint8Array, data_nt: string, expect_identity: string, xpath_regex?: string | null): string;

/**
 * r" `shaclProductValidateToSarifRebuild(product, dataNt, xpathRegex?)` → a SARIF 2.1.0 JSON
 * r" string, restoring the preparation by RE-DERIVING it from the shapes dataset the
 * r" product carries rather than admitting its memo.
 * r"
 * r" The forward-compatibility path: [`shacl_product_validate_to_sarif`] refuses a
 * r" product whose stage id this guest does not know with `dimension ===
 * r#" "stage-id"`, and this is the remedy it names. No RDF text is parsed and no"#
 * r" file is read — the dataset travels inside the product under the envelope's
 * r" own digests, and this re-derives the shapes graph from it.
 * r"
 * r" Also correct, and does the identical work, over a CURRENT product whose stage
 * r" id this guest already knows: rebuilding re-derives from the SAME carried
 * r" dataset [`shacl_product_validate_to_sarif`] restores a memo of, so the two
 * r" reach the byte-identical report. This function is a second DOOR onto one
 * r" product, never a second, divergent answer.
 * r"
 * r" Rejects with a [`ShaclProductRefusal`]; call `.free()` on it when done. A
 * r" malformed `dataNt` rejects with `dimension === undefined`, for the same
 * r" reason [`shacl_product_validate_to_sarif`] does.
 */
export function shaclProductValidateToSarifRebuild(product: Uint8Array, data_nt: string, xpath_regex?: string | null): string;

/**
 * r" `shaclProductValidateToSarifRebuildExpecting(product, dataNt, expectIdentity,
 * r" xpathRegex?)` →
 * r" a SARIF 2.1.0 JSON string, restoring the preparation by RE-DERIVING it from
 * r" the shapes dataset the product carries rather than admitting its memo, but
 * r" only from the product whose input binding is `expectIdentity`.
 * r"
 * r" The bound twin of [`shacl_product_validate_to_sarif_rebuild`], for the same
 * r" reason [`shacl_product_validate_to_sarif_expecting`] exists beside
 * r" [`shacl_product_validate_to_sarif`]: the forward-compatibility rescue is not
 * r" a reason to stop asking *is this the product the host meant?* — a product
 * r" fetched over the network or read out of a cache under a stage id this guest
 * r" does not recognize is still just bytes that could be the wrong ones. The
 * r" 32-byte comparison runs FIRST, ahead of the re-derivation, exactly as it does
 * r" on [`shacl_product_validate_to_sarif_expecting`].
 * r"
 * r" `expectIdentity` carries the same meaning it does on
 * r" [`shacl_product_validate_to_sarif_expecting`] — the 64 hexadecimal digits
 * r" `shaclProductExplain` prints on its `identity-digest` line.
 * r"
 * r" Rejects with a [`ShaclProductRefusal`]; call `.free()` on it when done. A
 * r" product carrying a different binding rejects with `dimension ===
 * r#" "shapes-graph"`; an `expectIdentity` that is not 64 hexadecimal digits"#
 * r" rejects with `dimension === undefined`, because no product was ever
 * r" inspected.
 */
export function shaclProductValidateToSarifRebuildExpecting(product: Uint8Array, data_nt: string, expect_identity: string, xpath_regex?: string | null): string;

/**
 * r" `shaclValidateChangesToSarif(shapesTtl, dataNt, addedNt?, removedNt?, shapesBase?,
 * r" importIris?, importDocuments?, shapesGraph?, xpathRegex?)` → a
 * r" `ShaclChangeValidation` carrying a SARIF 2.1.0 JSON string and its scope.
 * r"
 * r" The incremental twin of [`shacl_validate_to_sarif`]: instead of re-validating
 * r" the whole graph after an edit, hand it both halves of the delta and the engine
 * r" expands the change into the focus nodes it can move and re-validates exactly
 * r" those. A host that edits a graph and asks *what did my last change break?* pays
 * r" for the change rather than for the graph.
 * r"
 * r" `addedNt` is the rows joining `dataNt` and `removedNt` the rows leaving it,
 * r" each an N-Triples string or omitted. Both halves, because a verdict moves when
 * r" a row leaves the graph as readily as when one joins, and one parameter would be
 * r" half a delta. Additions apply before removals, so a change naming the same row
 * r" on both halves settles on *removed*; a removal naming a row `dataNt` does not
 * r" carry retracts nothing rather than throwing, because a change set describes
 * r" what moved and does not assert what the base contained.
 * r"
 * r" `shapesBase` carries the same meaning it does on [`shacl_validate_to_sarif`] —
 * r" the shapes document's own base IRI, supplied by the host because a wasm guest
 * r" has no retrieval IRI to derive one from.
 * r"
 * r" **Read `bounded` before the log.** It decides what the log MEANS; see
 * r" [`ShaclChangeValidation`]. The unbounded fallback is not optional — a short
 * r" expansion and a clean bill of health are indistinguishable in a report.
 * r"
 * r" `importIris` / `importDocuments` are the shapes graph's `owl:imports` table (see
 * r" [`ShaclImportError`]), and `shapesGraph` the IRI SHACL-SPARQL sees the shapes graph
 * r" under, exactly as [`shacl_validate_to_sarif`] takes them; so is `xpathRegex`, the
 * r" dated pattern law both the expansion and the re-validation run under.
 * r"
 * r" Throws (rejects) if the shapes graph or any of the three N-Triples documents
 * r" fails to parse, and with a [`ShaclImportError`] when the shapes graph's
 * r" `owl:imports` closure is not in hand. Call `.free()` on the returned object when
 * r" done.
 */
export function shaclValidateChangesToSarif(shapes_ttl: string, data_nt: string, added_nt?: string | null, removed_nt?: string | null, shapes_base?: string | null, import_iris?: string[] | null, import_documents?: string[] | null, shapes_graph?: string | null, xpath_regex?: string | null): ShaclChangeValidation;

/**
 * r" `shaclValidateToSarif(shapesTtl, dataNt, shapesBase?, conformanceDisallows?,
 * r" importIris?, importDocuments?, shapesGraph?, subClassOfInShapesGraph?, xpathRegex?)`
 * r" → a SARIF 2.1.0 JSON string.
 * r"
 * r" `shapesTtl` is a Turtle shapes graph; `dataNt` is an N-Triples data graph.
 * r" Throws (rejects) if either graph fails to parse.
 * r"
 * r" `conformanceDisallows` is the conformance-disallow set: severity IRIs whose results make
 * r" the data non-conforming. Omitted, it is SHACL's default set (`sh:Violation`,
 * r" `sh:Warning`, `sh:Info`); an empty array or a non-IRI throws. The log's run carries
 * r" `properties.shaclConforms` and `properties.shaclConformanceDisallows` (the set the
 * r" report was judged against), because the results alone cannot say whether the data
 * r" conforms: an `sh:Debug` / `sh:Trace` result — SARIF `kind` `informational`, `level`
 * r" `none` — appears in the log of a conforming report.
 * r"
 * r" `shapesBase` is the base IRI the SHAPES document's relative IRI references resolve
 * r" against. A browser or Node host has no retrieval IRI of its own — it was handed a
 * r" string — so PurRDF will not invent one; omit it and a relative reference is a hard
 * r" `iri-relative-no-base` naming the remedy. Passing the document's own URL is what makes
 * r" `<PersonShape>` in a fetched shapes graph mean what its author wrote. `dataNt` needs
 * r" no such parameter: N-Triples admits no relative IRI by grammar.
 * r"
 * r" `importIris` / `importDocuments` are the shapes graph's `owl:imports` table (see
 * r" [`ShaclImportError`]); an incomplete closure rejects with that class.
 * r"
 * r" `shapesGraph` is the IRI SHACL-SPARQL sees the shapes graph under, as `purrdf validate
 * r" --shapes-graph` names it: `$shapesGraph` is pre-bound to it and `GRAPH $shapesGraph {
 * r" … }` reads the shapes graph — SHACL 1.0's pre-binding, which SHACL 1.2 removed.
 * r" Omitted, no graph is named and `$shapesGraph` is an ordinary variable. A relative IRI
 * r" resolves against `shapesBase`; one with no base throws (`iri-relative-no-base`).
 * r"
 * r" `subClassOfInShapesGraph` is SHACL 1.2 Core §6.3's parameter of that name: `true`
 * r" reads the shapes graph's `rdfs:subClassOf` triples, in addition to the data graph's,
 * r" wherever SHACL type decides class membership (`sh:targetClass`, implicit class targets,
 * r" `sh:class`, `sh:rootClass`, `shnex:instancesOf`). Omitted or `false`, the
 * r" specification's default: the data graph alone.
 * r"
 * r" `xpathRegex` is the stable name of the dated native XPath law `sh:pattern`, and the
 * r" `REGEX`/`REPLACE` of SPARQL-based targets, constraints and SHACL-AF functions,
 * r#" evaluate under: `"xpath-2.0-2010-12-14"` or `"xpath-3.1-2017-03-21"`, matched"#
 * r" exactly. Omitted, the compatibility pattern behaviour is unchanged. A name that
 * r" selects no law throws, listing the accepted names. A pattern the law's grammar
 * r" refuses stays an ordinary validation result; a native resource refusal throws with
 * r" its resource code and no report.
 */
export function shaclValidateToSarif(shapes_ttl: string, data_nt: string, shapes_base?: string | null, conformance_disallows?: string[] | null, import_iris?: string[] | null, import_documents?: string[] | null, shapes_graph?: string | null, subclass_of_in_shapes_graph?: boolean | null, xpath_regex?: string | null): string;

/**
 * The purrdf engine version (the crate's SemVer), exposed to JS as `version()`.
 *
 * A liveness probe for the wasm build + the npm package: importing `purrdf` and
 * calling `version()` proves the module instantiated and the engine linked.
 */
export function version(): string;

/**
 * The XSD canonical lexical form of `lexical` read as the datatype IRI `datatype`
 * (`"+007"` as `xsd:integer` is `"7"`, `"1.50"` as `xsd:decimal` is `"1.5"`).
 *
 * Integers and decimals are exact at any length. Returns `undefined` when `datatype`
 * is not an XSD datatype the engine maps, or when `lexical` is not in its lexical
 * space.
 */
export function xsdCanonicalLexical(lexical: string, datatype: string): string | undefined;

/**
 * Compare two typed values in the XSD value space: `-1` when the left value is the
 * smaller, `0` when they are equal, `1` when it is the larger.
 *
 * Numeric datatypes compare across each other (`xsd:integer` against `xsd:decimal`),
 * exactly at any length. Returns `undefined` when either datatype is not an XSD
 * datatype the engine maps, either lexical form is not in its datatype's lexical
 * space, or the two values are incomparable (a `NaN`, or two value-space families
 * such as a number and a string).
 *
 * The order is an `f64` because a JavaScript number is one. An optional `f64` also
 * returns through the call's result slot, so the generated glue enters the instance
 * before it reads any argument, as every other entry point's does: on a poisoned
 * instance the call refuses at the poison gate rather than with a glue `TypeError`.
 */
export function xsdValueCompare(left_lexical: string, left_datatype: string, right_lexical: string, right_datatype: string): number | undefined;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __purrdf_test_exchange_terminal: (a: number) => [bigint, number, number];
    readonly __purrdf_test_large_dataset_identity: () => [number, number, number];
    readonly __purrdf_test_open_exchange: (a: bigint) => number;
    readonly __purrdf_test_panic: () => number;
    readonly __purrdf_test_trap: () => number;
    readonly __wbg_asynceffect_free: (a: number, b: number) => void;
    readonly __wbg_asyncevidence_free: (a: number, b: number) => void;
    readonly __wbg_asyncjob_free: (a: number, b: number) => void;
    readonly __wbg_asyncjoboptions_free: (a: number, b: number) => void;
    readonly __wbg_cancellationtoken_free: (a: number, b: number) => void;
    readonly __wbg_compiledjsonldcontext_free: (a: number, b: number) => void;
    readonly __wbg_datafactory_free: (a: number, b: number) => void;
    readonly __wbg_dataset_free: (a: number, b: number) => void;
    readonly __wbg_entailmentqueryoutcome_free: (a: number, b: number) => void;
    readonly __wbg_failureproblem_free: (a: number, b: number) => void;
    readonly __wbg_geopointindex_free: (a: number, b: number) => void;
    readonly __wbg_geosession_free: (a: number, b: number) => void;
    readonly __wbg_governorevidence_free: (a: number, b: number) => void;
    readonly __wbg_negotiatedoutcome_free: (a: number, b: number) => void;
    readonly __wbg_partialanswers_free: (a: number, b: number) => void;
    readonly __wbg_projectionlift_free: (a: number, b: number) => void;
    readonly __wbg_projectionpackage_free: (a: number, b: number) => void;
    readonly __wbg_provenanceinfo_free: (a: number, b: number) => void;
    readonly __wbg_quad_free: (a: number, b: number) => void;
    readonly __wbg_queryengine_free: (a: number, b: number) => void;
    readonly __wbg_queryoutcome_free: (a: number, b: number) => void;
    readonly __wbg_queryresult_free: (a: number, b: number) => void;
    readonly __wbg_reasoner_free: (a: number, b: number) => void;
    readonly __wbg_reasoninganswer_free: (a: number, b: number) => void;
    readonly __wbg_regimeclosure_free: (a: number, b: number) => void;
    readonly __wbg_selectresult_free: (a: number, b: number) => void;
    readonly __wbg_selectrow_free: (a: number, b: number) => void;
    readonly __wbg_serializeloss_free: (a: number, b: number) => void;
    readonly __wbg_servicecatalog_free: (a: number, b: number) => void;
    readonly __wbg_shaclchangevalidation_free: (a: number, b: number) => void;
    readonly __wbg_shaclentailment_free: (a: number, b: number) => void;
    readonly __wbg_shaclimporterror_free: (a: number, b: number) => void;
    readonly __wbg_shacljobrequest_free: (a: number, b: number) => void;
    readonly __wbg_shacllintreport_free: (a: number, b: number) => void;
    readonly __wbg_shaclnodeexproutcome_free: (a: number, b: number) => void;
    readonly __wbg_shaclproductrefusal_free: (a: number, b: number) => void;
    readonly __wbg_shaclrulescheck_free: (a: number, b: number) => void;
    readonly __wbg_shaclrulesinference_free: (a: number, b: number) => void;
    readonly __wbg_silencedinvocation_free: (a: number, b: number) => void;
    readonly __wbg_sink_free: (a: number, b: number) => void;
    readonly __wbg_sparqlprotocolrequest_free: (a: number, b: number) => void;
    readonly __wbg_term_free: (a: number, b: number) => void;
    readonly __wbg_trippedgovernor_free: (a: number, b: number) => void;
    readonly __wbg_updateoutcome_free: (a: number, b: number) => void;
    readonly asyncStackRegionBytes: () => [number, number, number];
    readonly asynceffect_abandonAfterMs: (a: number) => [number, number];
    readonly asynceffect_accept: (a: number) => [number, number];
    readonly asynceffect_cacheable: (a: number) => number;
    readonly asynceffect_contentType: (a: number) => [number, number];
    readonly asynceffect_endpoint: (a: number) => [number, number];
    readonly asynceffect_exchangeId: (a: number) => [number, bigint];
    readonly asynceffect_headers: (a: number) => [number, number];
    readonly asynceffect_iri: (a: number) => [number, number];
    readonly asynceffect_kind: (a: number) => number;
    readonly asynceffect_maxIntermediateCells: (a: number) => [number, bigint];
    readonly asynceffect_queryText: (a: number) => [number, number];
    readonly asynceffect_remainingDeadlineMs: (a: number) => [number, number];
    readonly asynceffect_seq: (a: number) => number;
    readonly asynceffect_silent: (a: number) => number;
    readonly asynceffect_timeoutMs: (a: number) => [number, number];
    readonly asynceffect_userAgent: (a: number) => [number, number];
    readonly asyncevidence_evaluateMs: (a: number) => number;
    readonly asyncevidence_expressionErrors: (a: number) => [number, number];
    readonly asyncevidence_freezeMs: (a: number) => number;
    readonly asyncevidence_loadEffects: (a: number) => number;
    readonly asyncevidence_loadWaitMs: (a: number) => number;
    readonly asyncevidence_polls: (a: number) => number;
    readonly asyncevidence_serializeMs: (a: number) => number;
    readonly asyncevidence_serviceEffects: (a: number) => number;
    readonly asyncevidence_serviceWaitMs: (a: number) => number;
    readonly asyncevidence_silenced: (a: number) => [number, number];
    readonly asyncevidence_stackHighWaterBytes: (a: number) => number;
    readonly asyncevidence_yieldWaitMs: (a: number) => number;
    readonly asyncevidence_yields: (a: number) => number;
    readonly asyncjob_beginShacl: (a: number, b: number) => [number, number, number];
    readonly asyncjob_cancel: (a: number) => number;
    readonly asyncjob_commitUpdate: (a: number, b: number) => [number, number];
    readonly asyncjob_deliverExchangeBindings: (a: any, b: number, c: number) => number;
    readonly asyncjob_deliverExchangeFailure: (a: any, b: number, c: number, d: number, e: number) => number;
    readonly asyncjob_deliverFailure: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
    readonly asyncjob_deliverGoverned: (a: number, b: number) => number;
    readonly asyncjob_deliverGraph: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => number;
    readonly asyncjob_deliverGraphDataset: (a: number, b: number, c: number) => number;
    readonly asyncjob_deliverRedirect: (a: number, b: number, c: number, d: number) => number;
    readonly asyncjob_errorCode: (a: number) => [number, number];
    readonly asyncjob_errorKind: (a: number) => [number, number];
    readonly asyncjob_errorMessage: (a: number) => [number, number];
    readonly asyncjob_errorPresentation: (a: number) => [number, number];
    readonly asyncjob_exchangeIsOpen: (a: any) => number;
    readonly asyncjob_expireEffect: (a: number, b: number) => number;
    readonly asyncjob_fault: (a: number, b: number, c: number) => number;
    readonly asyncjob_faultExchange: (a: any, b: number, c: number) => number;
    readonly asyncjob_finish: (a: number) => number;
    readonly asyncjob_id: (a: number) => number;
    readonly asyncjob_isFinished: (a: number) => number;
    readonly asyncjob_kind: (a: number) => number;
    readonly asyncjob_settledStatus: (a: number, b: number) => number;
    readonly asyncjob_stackTop: (a: number) => number;
    readonly asyncjob_takeEffect: (a: number) => number;
    readonly asyncjob_takeEntailmentOutcome: (a: number) => [number, number, number];
    readonly asyncjob_takeEvidence: (a: number) => number;
    readonly asyncjob_takeNegotiatedOutcome: (a: number) => [number, number, number];
    readonly asyncjob_takeQueryOutcome: (a: number) => [number, number, number];
    readonly asyncjob_takeQueryResult: (a: number, b: number, c: number) => [number, number, number];
    readonly asyncjob_takeRawBytes: (a: number) => [number, number, number, number];
    readonly asyncjob_takeShaclChangeValidation: (a: number) => [number, number, number];
    readonly asyncjob_takeShaclEntailment: (a: number) => [number, number, number];
    readonly asyncjob_takeShaclImportError: (a: number) => number;
    readonly asyncjob_takeShaclNodeExprOutcome: (a: number) => [number, number, number];
    readonly asyncjob_takeShaclRefusal: (a: number) => number;
    readonly asyncjob_takeShaclRulesInference: (a: number) => [number, number, number];
    readonly asyncjob_takeUpdateOutcome: (a: number) => [number, number, number];
    readonly asyncjoboptions_fromJs: (a: number, b: any, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly cancellationtoken_cancel: (a: number) => void;
    readonly cancellationtoken_isCancelled: (a: number) => number;
    readonly cancellationtoken_new: () => number;
    readonly cancellationtoken_share: (a: number) => number;
    readonly compiledjsonldcontext_canonicalContextJson: (a: number) => [number, number];
    readonly compiledjsonldcontext_new: (a: number, b: number) => [number, number, number];
    readonly datafactory_blankNode: (a: number, b: number, c: number) => number;
    readonly datafactory_defaultGraph: (a: number) => number;
    readonly datafactory_directionalLiteral: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number];
    readonly datafactory_fromQuad: (a: number, b: number) => number;
    readonly datafactory_fromTerm: (a: number, b: number) => number;
    readonly datafactory_literal: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly datafactory_namedNode: (a: number, b: number, c: number) => number;
    readonly datafactory_new: () => number;
    readonly datafactory_quad: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly datafactory_quotedTriple: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly datafactory_typedLiteral: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly datafactory_variable: (a: number, b: number, c: number) => number;
    readonly dataset_add: (a: number, b: number) => [number, number, number];
    readonly dataset_canonicalize: (a: number) => [number, number, number, number];
    readonly dataset_delete: (a: number, b: number) => [number, number, number];
    readonly dataset_generation: (a: number) => bigint;
    readonly dataset_has: (a: number, b: number) => [number, number, number];
    readonly dataset_id: (a: number) => bigint;
    readonly dataset_isomorphic: (a: number, b: number) => [number, number, number];
    readonly dataset_match: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly dataset_new: () => [number, number, number];
    readonly dataset_parse: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly dataset_project: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly dataset_projectWithAssets: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number];
    readonly dataset_quads: (a: number) => [number, number, number, number];
    readonly dataset_query: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number, number];
    readonly dataset_serialize: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly dataset_serializeConfigured: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number, number];
    readonly dataset_serializeToSink: (a: number, b: number, c: number, d: number, e: number, f: any) => [number, number];
    readonly dataset_serializeWithContext: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number, number];
    readonly dataset_serializeWithLoss: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly dataset_size: (a: number) => number;
    readonly dataset_snapshot: (a: number) => [number, number, number];
    readonly dataset_visualExportJson: (a: number, b: number, c: number) => [number, number, number, number];
    readonly dataset_visualModelJson: (a: number, b: number, c: number) => [number, number, number, number];
    readonly dataset_visualSvgJson: (a: number, b: number, c: number) => [number, number, number, number];
    readonly entailCertainAnswers: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: bigint, o: number, p: bigint) => [number, number, number];
    readonly entailCheckAbsentProof: () => [number, number];
    readonly entailCheckGoldenVectors: () => [number, number];
    readonly entailCheckInconsistentRefusal: () => [number, number];
    readonly entailCheckProof: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number) => [number, number, number, number];
    readonly entailCheckProofGoldenVectors: () => [number, number];
    readonly entailClassify: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly entailConsistency: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => [number, number, number];
    readonly entailEntails: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly entailExplainConclusion: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly entailExtensions: (a: number, b: number) => [number, number, number, number];
    readonly entailExtractModule: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly entailGraphEntails: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: bigint, o: number, p: bigint) => [number, number, number];
    readonly entailImplementedRules: (a: number, b: number) => [number, number, number, number];
    readonly entailInstances: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly entailJustify: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly entailMaterialize: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: bigint, o: number, p: bigint) => [number, number, number];
    readonly entailProfile: (a: number, b: number) => [number, number, number];
    readonly entailProve: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number];
    readonly entailRealize: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly entailRules: (a: number, b: number) => [number, number, number, number];
    readonly entailVerifyEntailment: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: bigint, o: number, p: bigint) => [number, number, number];
    readonly entailmentqueryoutcome_closureStopped: (a: number) => number;
    readonly entailmentqueryoutcome_isComplete: (a: number) => number;
    readonly entailmentqueryoutcome_report: (a: number) => [number, number];
    readonly entailmentqueryoutcome_takeOutcome: (a: number) => number;
    readonly entailmentqueryoutcome_takeTripped: (a: number) => number;
    readonly expressionErrorCodes: () => [number, number];
    readonly failureproblem_body: (a: number, b: number, c: number) => [number, number, number, number];
    readonly failureproblem_contentType: (a: number) => [number, number];
    readonly failureproblem_internal: (a: number) => number;
    readonly failureproblem_status: (a: number) => number;
    readonly geopointindex_call: (a: number, b: number, c: number) => [number, number, number, number];
    readonly geosession_call: (a: number, b: number, c: number) => [number, number, number, number];
    readonly geosession_new: (a: number, b: number) => [number, number, number];
    readonly geosession_pointIndex: (a: number, b: number, c: number) => [number, number, number];
    readonly geosession_profile: (a: number) => [number, number, number, number];
    readonly governorDimensions: () => [number, number];
    readonly governorevidence_consumed: (a: number) => [number, number];
    readonly governorevidence_expressionErrors: (a: number) => [number, number];
    readonly governorevidence_isComplete: (a: number) => number;
    readonly governorevidence_limits: (a: number) => [number, number];
    readonly governorevidence_silenced: (a: number) => [number, number];
    readonly install_stack_floor: () => void;
    readonly liftProjection: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly negotiatedoutcome_format: (a: number) => [number, number];
    readonly negotiatedoutcome_isComplete: (a: number) => number;
    readonly negotiatedoutcome_mediaType: (a: number) => [number, number];
    readonly negotiatedoutcome_takeBody: (a: number) => [number, number];
    readonly negotiatedoutcome_takeEvidence: (a: number) => number;
    readonly negotiatedoutcome_takePartial: (a: number) => number;
    readonly negotiatedoutcome_takeTripped: (a: number) => number;
    readonly partialanswers_barrier: (a: number) => [number, number];
    readonly partialanswers_certainty: (a: number) => [number, number];
    readonly partialanswers_isCertain: (a: number) => number;
    readonly partialanswers_isPositionalPrefix: (a: number) => number;
    readonly partialanswers_takeResult: (a: number) => number;
    readonly projectionlift_lossLedgerJson: (a: number) => [number, number];
    readonly projectionlift_takeDataset: (a: number) => number;
    readonly projectionpackage_archive: (a: number) => [number, number];
    readonly projectionpackage_lossLedgerJson: (a: number) => [number, number];
    readonly projectionpackage_profile: (a: number) => [number, number];
    readonly provenanceFromJson: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly provenanceFromXml: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly provenanceinfo_engine: (a: number) => [number, number];
    readonly provenanceinfo_queryHash: (a: number) => [number, number];
    readonly purrdf_jspi_run: (a: number) => number;
    readonly quad_asTerm: (a: number) => [number, number, number];
    readonly quad_equals: (a: number, b: number) => number;
    readonly quad_graph: (a: number) => number;
    readonly quad_object: (a: number) => number;
    readonly quad_predicate: (a: number) => number;
    readonly quad_subject: (a: number) => number;
    readonly quad_term_type: (a: number) => [number, number];
    readonly quad_value: (a: number) => [number, number];
    readonly queryengine_ask: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number];
    readonly queryengine_beginAsync: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly queryengine_beginAsyncWithContext: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number];
    readonly queryengine_blank_scope_name: (a: number) => [number, number];
    readonly queryengine_construct: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number];
    readonly queryengine_division_policy: (a: number) => [number, number];
    readonly queryengine_explainQuery: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number, number];
    readonly queryengine_new: () => number;
    readonly queryengine_query: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number];
    readonly queryengine_queryEntailmentGoverned: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number, o: number, p: number, q: number, r: bigint, s: number, t: bigint, u: number, v: number, w: number, x: bigint, y: number, z: bigint, a1: number, b1: bigint, c1: number, d1: bigint, e1: number, f1: bigint, g1: number, h1: bigint, i1: number, j1: number, k1: number, l1: number) => [number, number, number];
    readonly queryengine_queryGoverned: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: bigint, k: number, l: bigint, m: number, n: bigint, o: number, p: bigint, q: number, r: bigint, s: number, t: bigint, u: number, v: number, w: number, x: number) => [number, number, number];
    readonly queryengine_queryRaw: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number) => [number, number, number, number];
    readonly queryengine_queryRawConfigured: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number) => [number, number, number, number];
    readonly queryengine_queryRawWithContext: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number) => [number, number, number, number];
    readonly queryengine_select: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number];
    readonly queryengine_setGeoProfile: (a: number, b: number, c: number) => [number, number];
    readonly queryengine_setGeoSession: (a: number, b: number) => void;
    readonly queryengine_set_blank_scope: (a: number, b: number, c: number) => [number, number];
    readonly queryengine_set_division_policy: (a: number, b: number, c: number) => [number, number];
    readonly queryengine_update: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number];
    readonly queryengine_updateGoverned: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: bigint, k: number, l: bigint, m: number, n: bigint, o: number, p: bigint, q: number, r: bigint, s: number, t: bigint, u: number, v: number, w: number, x: number) => [number, number, number];
    readonly queryoutcome_isComplete: (a: number) => number;
    readonly queryoutcome_takeEvidence: (a: number) => number;
    readonly queryoutcome_takeResult: (a: number) => number;
    readonly queryoutcome_takeTripped: (a: number) => number;
    readonly queryresult_boolean: (a: number) => number;
    readonly queryresult_kind: (a: number) => [number, number];
    readonly queryresult_takeDataset: (a: number) => number;
    readonly queryresult_takeSelect: (a: number) => number;
    readonly reasoner_classify: (a: number) => [number, number, number];
    readonly reasoner_consistency: (a: number) => [number, number, number];
    readonly reasoner_entails: (a: number, b: number, c: number) => [number, number, number];
    readonly reasoner_explainConclusion: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly reasoner_extractModule: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly reasoner_instances: (a: number, b: number, c: number) => [number, number, number];
    readonly reasoner_justify: (a: number, b: number, c: number) => [number, number, number];
    readonly reasoner_new: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly reasoner_profile: (a: number) => number;
    readonly reasoner_prove: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly reasoner_realize: (a: number) => [number, number, number];
    readonly reasoner_recordsProofs: (a: number) => number;
    readonly reasoner_withProofs: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly reasoninganswer_answer: (a: number) => [number, number];
    readonly reasoninganswer_certificate: (a: number) => [number, number];
    readonly reasoninganswer_proof: (a: number) => [number, number];
    readonly regimeclosure_nquads: (a: number) => [number, number];
    readonly regimeclosure_report: (a: number) => [number, number];
    readonly selectresult_kind: (a: number) => [number, number];
    readonly selectresult_nextRow: (a: number) => number;
    readonly selectresult_remaining: (a: number) => number;
    readonly selectresult_row_count: (a: number) => number;
    readonly selectresult_takeRow: (a: number, b: number) => number;
    readonly selectresult_variables: (a: number) => [number, number];
    readonly selectrow_get: (a: number, b: number, c: number) => number;
    readonly selectrow_takeValue: (a: number, b: number) => number;
    readonly selectrow_variables: (a: number) => [number, number];
    readonly serializeloss_directionalLiteralsDropped: (a: number) => number;
    readonly serializeloss_namedGraphRowsDropped: (a: number) => number;
    readonly serializeloss_statementRowsDropped: (a: number) => number;
    readonly serializeloss_text: (a: number) => [number, number];
    readonly servicecatalog_addService: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly servicecatalog_copy: (a: number) => number;
    readonly servicecatalog_new: () => number;
    readonly servicecatalog_setFallback: (a: number, b: number, c: number) => [number, number];
    readonly shaclApplyRules: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number, o: number, p: number, q: number, r: number, s: bigint, t: number, u: bigint, v: number, w: bigint, x: number, y: bigint, z: number, a1: number) => [number, number, number];
    readonly shaclCheckRules: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => [number, number, number];
    readonly shaclEntail: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: bigint, o: number, p: bigint, q: number, r: bigint, s: number, t: bigint, u: number, v: number) => [number, number, number];
    readonly shaclEvalNodeExpr: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number, o: number, p: number, q: number, r: number, s: number, t: number, u: number, v: number, w: number, x: number) => [number, number, number];
    readonly shaclLintShapes: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => [number, number, number];
    readonly shaclPackProduct: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => [number, number, number, number];
    readonly shaclProductCertify: (a: number, b: number) => [number, number];
    readonly shaclProductExplain: (a: number, b: number) => [number, number, number, number];
    readonly shaclProductValidateToSarif: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly shaclProductValidateToSarifExpecting: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number, number];
    readonly shaclProductValidateToSarifRebuild: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly shaclProductValidateToSarifRebuildExpecting: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number, number];
    readonly shaclValidateChangesToSarif: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number, o: number, p: number, q: number, r: number) => [number, number, number];
    readonly shaclValidateToSarif: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number, o: number, p: number, q: number) => [number, number, number, number];
    readonly shaclchangevalidation_bounded: (a: number) => number;
    readonly shaclchangevalidation_focus_nodes: (a: number) => number;
    readonly shaclchangevalidation_reason: (a: number) => [number, number];
    readonly shaclchangevalidation_sarif: (a: number) => [number, number];
    readonly shaclentailment_diagnostics: (a: number) => [number, number];
    readonly shaclentailment_ntriples: (a: number) => [number, number];
    readonly shaclimporterror_iris: (a: number) => [number, number];
    readonly shaclimporterror_kind: (a: number) => [number, number];
    readonly shaclimporterror_message: (a: number) => [number, number];
    readonly shacljobrequest_applyRules: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number, o: number, p: number, q: number, r: number, s: bigint, t: number, u: bigint, v: number, w: bigint, x: number, y: bigint) => number;
    readonly shacljobrequest_entail: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: bigint, o: number, p: bigint, q: number, r: bigint, s: number, t: bigint) => number;
    readonly shacljobrequest_evalNodeExpr: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number, o: number, p: number, q: number, r: number, s: number, t: number, u: number, v: number) => number;
    readonly shacljobrequest_productValidateToSarif: (a: number, b: number, c: number, d: number) => number;
    readonly shacljobrequest_productValidateToSarifExpecting: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
    readonly shacljobrequest_productValidateToSarifRebuild: (a: number, b: number, c: number, d: number) => number;
    readonly shacljobrequest_productValidateToSarifRebuildExpecting: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
    readonly shacljobrequest_validateChangesToSarif: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number, o: number, p: number) => number;
    readonly shacljobrequest_validateToSarif: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number, o: number) => number;
    readonly shacllintreport_clean: (a: number) => number;
    readonly shacllintreport_findings: (a: number) => number;
    readonly shacllintreport_load_error: (a: number) => [number, number];
    readonly shacllintreport_report: (a: number) => [number, number];
    readonly shaclnodeexproutcome_diagnostics: (a: number) => [number, number];
    readonly shaclnodeexproutcome_outputs: (a: number) => [number, number];
    readonly shaclproductrefusal_code: (a: number) => [number, number];
    readonly shaclproductrefusal_dimension: (a: number) => [number, number];
    readonly shaclproductrefusal_message: (a: number) => [number, number];
    readonly shaclproductrefusal_toString: (a: number) => [number, number];
    readonly shaclrulescheck_imported: (a: number) => [number, number];
    readonly shaclrulescheck_level: (a: number) => [number, number];
    readonly shaclrulescheck_rules: (a: number) => [number, number];
    readonly shaclrulescheck_strata: (a: number) => number;
    readonly shaclrulescheck_summary: (a: number) => [number, number];
    readonly shaclrulescheck_versions: (a: number) => [number, number];
    readonly shaclrulesinference_diagnostics: (a: number) => [number, number];
    readonly shaclrulesinference_inferred: (a: number) => [number, number];
    readonly shaclrulesinference_proof: (a: number) => [number, number];
    readonly silencedinvocation_endpoint: (a: number) => [number, number];
    readonly silencedinvocation_iri: (a: number) => [number, number];
    readonly silencedinvocation_kind: (a: number) => [number, number];
    readonly silencedinvocation_message: (a: number) => [number, number];
    readonly silencedinvocation_target: (a: number) => [number, number];
    readonly sink_finish: (a: number) => [number, number, number];
    readonly sink_new: () => number;
    readonly sink_push: (a: number, b: number) => [number, number];
    readonly sparqlprotocolrequest_defaultGraphUris: (a: number) => [number, number];
    readonly sparqlprotocolrequest_effectiveText: (a: number) => [number, number, number, number];
    readonly sparqlprotocolrequest_extraParameters: (a: number) => [number, number];
    readonly sparqlprotocolrequest_formatMediaType: (a: number, b: number) => [number, number];
    readonly sparqlprotocolrequest_hasDatasetParameters: (a: number) => number;
    readonly sparqlprotocolrequest_kind: (a: number) => [number, number];
    readonly sparqlprotocolrequest_namedGraphUris: (a: number) => [number, number];
    readonly sparqlprotocolrequest_negotiate: (a: number, b: number, c: number) => [number, number, number, number];
    readonly sparqlprotocolrequest_notAcceptableDetail: (a: number, b: number) => [number, number, number, number];
    readonly sparqlprotocolrequest_offeredMediaTypes: (a: number, b: number) => [number, number, number, number];
    readonly sparqlprotocolrequest_parse: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number];
    readonly sparqlprotocolrequest_problemFor: (a: any, b: number) => number;
    readonly sparqlprotocolrequest_problemForTrip: (a: any) => [number, number, number];
    readonly sparqlprotocolrequest_resultKind: (a: number) => [number, number, number, number];
    readonly sparqlprotocolrequest_statusTitle: (a: number) => [number, number];
    readonly sparqlprotocolrequest_text: (a: number) => [number, number];
    readonly sparqlprotocolrequest_usingGraphUris: (a: number) => [number, number];
    readonly sparqlprotocolrequest_usingNamedGraphUris: (a: number) => [number, number];
    readonly term_datatype: (a: number) => number;
    readonly term_direction: (a: number) => [number, number];
    readonly term_equals: (a: number, b: number) => number;
    readonly term_graph: (a: number) => number;
    readonly term_language: (a: number) => [number, number];
    readonly term_object: (a: number) => number;
    readonly term_predicate: (a: number) => number;
    readonly term_subject: (a: number) => number;
    readonly term_term_type: (a: number) => [number, number];
    readonly term_value: (a: number) => [number, number];
    readonly trippedgovernor_cause: (a: number) => [number, number];
    readonly trippedgovernor_consumed: (a: number) => [number, bigint];
    readonly trippedgovernor_dimension: (a: number) => [number, number];
    readonly trippedgovernor_estimate: (a: number) => [number, bigint];
    readonly trippedgovernor_kind: (a: number) => [number, number];
    readonly trippedgovernor_label: (a: number) => [number, number];
    readonly trippedgovernor_limit: (a: number) => [number, bigint];
    readonly trippedgovernor_message: (a: number) => [number, number];
    readonly updateoutcome_isApplied: (a: number) => number;
    readonly updateoutcome_takeEvidence: (a: number) => number;
    readonly updateoutcome_takeTripped: (a: number) => number;
    readonly version: () => [number, number];
    readonly xsdCanonicalLexical: (a: number, b: number, c: number, d: number) => [number, number];
    readonly xsdValueCompare: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number];
    readonly main: (a: number, b: number) => number;
    readonly queryengine_describe: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number];
    readonly serializeloss_emptyNamedGraphsDropped: (a: number) => number;
    readonly shaclrulescheck_data_triples: (a: number) => number;
    readonly queryoutcome_takePartial: (a: number) => number;
    readonly shacldiagnostic_rule: (a: number) => [number, number];
    readonly shacldiagnostic_shape: (a: number) => [number, number];
    readonly shaclimporterror_toString: (a: number) => [number, number];
    readonly __wbg_shacldiagnostic_free: (a: number, b: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __externref_drop_slice: (a: number, b: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
