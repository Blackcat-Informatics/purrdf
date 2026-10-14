/* @ts-self-types="./fixture.d.ts" */

/**
 * One effect the host has taken, as JavaScript sees it.
 */
class AsyncEffect {
    static __wrap(ptr) {
        const obj = Object.create(AsyncEffect.prototype);
        obj.__wbg_ptr = ptr;
        AsyncEffectFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        AsyncEffectFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_asynceffect_free(ptr, 0);
    }
    /**
     * When the host abandons the effect if it is still unanswered, in milliseconds from
     * now: the request's own timeout or the job's remaining deadline, whichever is
     * sooner. The host calls [`AsyncJob::expire_effect`] when it passes. `undefined` for
     * a yield.
     * @returns {number | undefined}
     */
    get abandonAfterMs() {
        const ret = wasm.asynceffect_abandonAfterMs(this.__wbg_ptr);
        return ret[0] === 0 ? undefined : ret[1];
    }
    /**
     * `SERVICE`: the `Accept` header value (`application/sparql-results+json`); `LOAD`:
     * the media type of every RDF syntax a `LOAD` parses.
     * @returns {string | undefined}
     */
    get accept() {
        const ret = wasm.asynceffect_accept(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * `SERVICE`: whether a shared cache may answer the request or keep its answer —
     * `false` when it carries a credential, whose answer belongs to the credential's
     * holder.
     * @returns {boolean}
     */
    get cacheable() {
        const ret = wasm.asynceffect_cacheable(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * `SERVICE`: the `Content-Type` header value (`application/sparql-query`).
     * @returns {string | undefined}
     */
    get contentType() {
        const ret = wasm.asynceffect_contentType(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * `SERVICE`: the endpoint IRI exactly as the query wrote it.
     * @returns {string | undefined}
     */
    get endpoint() {
        const ret = wasm.asynceffect_endpoint(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * `SERVICE` and a wait on an exchange: the shared exchange the answer is delivered
     * to.
     * @returns {bigint | undefined}
     */
    get exchangeId() {
        const ret = wasm.asynceffect_exchangeId(this.__wbg_ptr);
        return ret[0] === 0 ? undefined : BigInt.asUintN(64, ret[1]);
    }
    /**
     * `SERVICE` and `LOAD`: the extra request headers as flattened `[name, value, name,
     * value, …]` pairs — the catalog profile's headers, then its credential header — in
     * the order they must be sent. Append each pair; a repeated name is legal and must
     * not be merged. Empty when no catalog is configured.
     * @returns {string[]}
     */
    headers() {
        const ret = wasm.asynceffect_headers(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * `LOAD`: the IRI of this hop — the source, or the location a redirect named.
     * @returns {string | undefined}
     */
    get iri() {
        const ret = wasm.asynceffect_iri(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * What the effect asks for.
     * @returns {EffectKind}
     */
    get kind() {
        const ret = wasm.asynceffect_kind(this.__wbg_ptr);
        return ret;
    }
    /**
     * `SERVICE`: the query's inclusive intermediate-cell ceiling, when the caller
     * actually configured one. `undefined` on an ungoverned query and on a governed one
     * that set no cell ceiling — never the metering bookkeeping sentinel a caller never
     * asked for.
     * @returns {bigint | undefined}
     */
    get maxIntermediateCells() {
        const ret = wasm.asynceffect_maxIntermediateCells(this.__wbg_ptr);
        return ret[0] === 0 ? undefined : BigInt.asUintN(64, ret[1]);
    }
    /**
     * `SERVICE`: the complete forwarded `SELECT * WHERE { … }` text — the request body.
     * @returns {string | undefined}
     */
    get queryText() {
        const ret = wasm.asynceffect_queryText(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * `SERVICE`: milliseconds left before the job's deadline, when one is set.
     * @returns {number | undefined}
     */
    get remainingDeadlineMs() {
        const ret = wasm.asynceffect_remainingDeadlineMs(this.__wbg_ptr);
        return ret[0] === 0 ? undefined : ret[1];
    }
    /**
     * The sequence number every delivery for this effect must name.
     * @returns {number}
     */
    get seq() {
        const ret = wasm.asynceffect_seq(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * `SERVICE`: whether the clause was written `SERVICE SILENT`. Informational only:
     * an empty answer is never the host's to invent because of it.
     * @returns {boolean}
     */
    get silent() {
        const ret = wasm.asynceffect_silent(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * `SERVICE` and `LOAD`: the per-request timeout the catalog profile names, or the
     * default, in milliseconds.
     * @returns {number | undefined}
     */
    get timeoutMs() {
        const ret = wasm.asynceffect_timeoutMs(this.__wbg_ptr);
        return ret[0] === 0 ? undefined : ret[1];
    }
    /**
     * `SERVICE`: the `User-Agent` the service's profile names, or the engine default;
     * `LOAD`: the one the source's catalog profile names, if it names one.
     * @returns {string | undefined}
     */
    get userAgent() {
        const ret = wasm.asynceffect_userAgent(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
}
if (Symbol.dispose) AsyncEffect.prototype[Symbol.dispose] = AsyncEffect.prototype.free;
exports.AsyncEffect = AsyncEffect;

/**
 * What one asynchronous job cost: counts and phase times, all measured in Rust.
 *
 * `evaluateMs` is the wall time of the evaluation phase and so *includes* the waits the
 * three `…WaitMs` fields report separately; `freezeMs` and `serializeMs` are the phases
 * that run to completion without yielding.
 */
class AsyncEvidence {
    static __wrap(ptr) {
        const obj = Object.create(AsyncEvidence.prototype);
        obj.__wbg_ptr = ptr;
        AsyncEvidenceFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        AsyncEvidenceFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_asyncevidence_free(ptr, 0);
    }
    /**
     * Milliseconds of the evaluation phase, waits included.
     * @returns {number}
     */
    get evaluateMs() {
        const ret = wasm.asyncevidence_evaluateMs(this.__wbg_ptr);
        return ret;
    }
    /**
     * Every XPath F&O numeric error the job's evaluations absorbed into an unbound
     * value, summed per code and positional by `expressionErrorCodes()`. All zero when
     * no expression failed. A number, as every other count of the job's evidence is, so
     * the job's evidence stays JSON-serializable.
     * @returns {Float64Array}
     */
    get expressionErrors() {
        const ret = wasm.asyncevidence_expressionErrors(this.__wbg_ptr);
        var v1 = getArrayF64FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
        return v1;
    }
    /**
     * Milliseconds freezing the dataset snapshot before the job started.
     * @returns {number}
     */
    get freezeMs() {
        const ret = wasm.asyncevidence_freezeMs(this.__wbg_ptr);
        return ret;
    }
    /**
     * `LOAD` effects the host answered or abandoned.
     * @returns {number}
     */
    get loadEffects() {
        const ret = wasm.asyncevidence_loadEffects(this.__wbg_ptr);
        return ret;
    }
    /**
     * Milliseconds spent suspended on `LOAD` effects.
     * @returns {number}
     */
    get loadWaitMs() {
        const ret = wasm.asyncevidence_loadWaitMs(this.__wbg_ptr);
        return ret;
    }
    /**
     * Stop-signal polls the evaluator made.
     * @returns {number}
     */
    get polls() {
        const ret = wasm.asyncevidence_polls(this.__wbg_ptr);
        return ret;
    }
    /**
     * Milliseconds serializing a raw result.
     * @returns {number}
     */
    get serializeMs() {
        const ret = wasm.asyncevidence_serializeMs(this.__wbg_ptr);
        return ret;
    }
    /**
     * `SERVICE` effects — requests the job issued, and waits on another job's
     * identical request — the host answered or abandoned. A request the job repeats is
     * answered from its own memo and issues no effect.
     * @returns {number}
     */
    get serviceEffects() {
        const ret = wasm.asyncevidence_serviceEffects(this.__wbg_ptr);
        return ret;
    }
    /**
     * Milliseconds spent suspended on `SERVICE` effects.
     * @returns {number}
     */
    get serviceWaitMs() {
        const ret = wasm.asyncevidence_serviceWaitMs(this.__wbg_ptr);
        return ret;
    }
    /**
     * Every invocation a `SERVICE SILENT` or `LOAD SILENT` absorbed during the job, in
     * the order its evaluations recorded them. Empty when nothing failed.
     * @returns {SilencedInvocation[]}
     */
    get silenced() {
        const ret = wasm.asyncevidence_silenced(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * The deepest the job's stack reached at a poll, in bytes below its region's top.
     * `0` when the job never ran on a region (the native build).
     * @returns {number}
     */
    get stackHighWaterBytes() {
        const ret = wasm.asyncevidence_stackHighWaterBytes(this.__wbg_ptr);
        return ret;
    }
    /**
     * Milliseconds spent suspended on yields.
     * @returns {number}
     */
    get yieldWaitMs() {
        const ret = wasm.asyncevidence_yieldWaitMs(this.__wbg_ptr);
        return ret;
    }
    /**
     * Times the job gave the event loop back.
     * @returns {number}
     */
    get yields() {
        const ret = wasm.asyncevidence_yields(this.__wbg_ptr);
        return ret;
    }
}
if (Symbol.dispose) AsyncEvidence.prototype[Symbol.dispose] = AsyncEvidence.prototype.free;
exports.AsyncEvidence = AsyncEvidence;

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
class AsyncJob {
    static __wrap(ptr) {
        const obj = Object.create(AsyncJob.prototype);
        obj.__wbg_ptr = ptr;
        AsyncJobFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        AsyncJobFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_asyncjob_free(ptr, 0);
    }
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
     * @param {AsyncJobOptions} options
     * @param {ShaclJobRequest} request
     * @returns {AsyncJob}
     */
    static beginShacl(options, request) {
        _assertClass(options, AsyncJobOptions);
        _assertClass(request, ShaclJobRequest);
        var ptr0 = request.__destroy_into_raw();
        const ret = wasm.asyncjob_beginShacl(options.__wbg_ptr, ptr0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return AsyncJob.__wrap(ret[0]);
    }
    /**
     * Cancel the job. It observes the cancellation at its next poll, yield or effect.
     * @returns {DeliveryStatus}
     */
    cancel() {
        const ret = wasm.asyncjob_cancel(this.__wbg_ptr);
        return ret;
    }
    /**
     * Apply a finished update's result to `dataset` — refused, with the dataset left
     * untouched, when `dataset` is not the one the update read or has been mutated since
     * it started.
     * @param {Dataset} dataset
     */
    commitUpdate(dataset) {
        _assertClass(dataset, Dataset);
        const ret = wasm.asyncjob_commitUpdate(this.__wbg_ptr, dataset.__wbg_ptr);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Answer every job waiting on shared exchange `exchange` with SPARQL Results JSON
     * bytes, and close the exchange. `Finished` when no such exchange is open — its
     * last waiter left, or it was already answered.
     * @param {bigint} exchange
     * @param {Uint8Array} bytes
     * @returns {DeliveryStatus}
     */
    static deliverExchangeBindings(exchange, bytes) {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.asyncjob_deliverExchangeBindings(exchange, ptr0, len0);
        return ret;
    }
    /**
     * Fail every job waiting on shared exchange `exchange`: `kind` is `"transport"`,
     * `"denied"` or `"fault"`; any other kind is delivered as a fault naming it. Each
     * waiting job's effect fails as its own invocation's failure. Closes the exchange.
     * @param {bigint} exchange
     * @param {string} kind
     * @param {string} message
     * @returns {DeliveryStatus}
     */
    static deliverExchangeFailure(exchange, kind, message) {
        const ptr0 = passStringToWasm0(kind, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(message, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.asyncjob_deliverExchangeFailure(exchange, ptr0, len0, ptr1, len1);
        return ret;
    }
    /**
     * Fail effect `seq`: `kind` is `"transport"` (unreachable, or unreadable), `"denied"`
     * (the host's policy refused it) or `"fault"` (the host's handler threw, rejected or
     * returned something the protocol does not define). Any other kind is delivered as a
     * fault naming it. Each is the effect's own failure, answered by the clause that
     * issued it; none is the job's fault.
     * @param {number} seq
     * @param {string} kind
     * @param {string} message
     * @returns {DeliveryStatus}
     */
    deliverFailure(seq, kind, message) {
        const ptr0 = passStringToWasm0(kind, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(message, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.asyncjob_deliverFailure(this.__wbg_ptr, seq, ptr0, len0, ptr1, len1);
        return ret;
    }
    /**
     * Record that the host abandoned effect `seq` on the job's stop signal. When that
     * signal is not already latched, this cancels the job. A job waiting on a shared
     * exchange leaves it; see [`Self::exchange_is_open`].
     * @param {number} seq
     * @returns {DeliveryStatus}
     */
    deliverGoverned(seq) {
        const ret = wasm.asyncjob_deliverGoverned(this.__wbg_ptr, seq);
        return ret;
    }
    /**
     * Answer `LOAD` effect `seq` with a document. It is parsed here, by media type (or
     * any format name `Dataset.parse` accepts); a document that cannot be parsed is the
     * `LOAD`'s decode failure, never a fault.
     * @param {number} seq
     * @param {Uint8Array} bytes
     * @param {string} media_type
     * @param {string | null} [base]
     * @returns {DeliveryStatus}
     */
    deliverGraph(seq, bytes, media_type, base) {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(media_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(base) ? 0 : passStringToWasm0(base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.asyncjob_deliverGraph(this.__wbg_ptr, seq, ptr0, len0, ptr1, len1, ptr2, len2);
        return ret;
    }
    /**
     * Answer `LOAD` effect `seq` with a snapshot of an existing dataset.
     * @param {number} seq
     * @param {Dataset} dataset
     * @returns {DeliveryStatus}
     */
    deliverGraphDataset(seq, dataset) {
        _assertClass(dataset, Dataset);
        const ret = wasm.asyncjob_deliverGraphDataset(this.__wbg_ptr, seq, dataset.__wbg_ptr);
        return ret;
    }
    /**
     * Answer `LOAD` effect `seq` with a redirect to `location`, the response's
     * `Location` as sent. The job resolves it against the effect's IRI, authorizes the
     * target against its catalog and fetches it as a fresh effect, up to its redirect
     * limit.
     * @param {number} seq
     * @param {string} location
     * @returns {DeliveryStatus}
     */
    deliverRedirect(seq, location) {
        const ptr0 = passStringToWasm0(location, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.asyncjob_deliverRedirect(this.__wbg_ptr, seq, ptr0, len0);
        return ret;
    }
    /**
     * The stable code the job's failure is reported under — an engine diagnostic's own
     * code, or the code of a stop, a fault or a refusal — or `undefined` when it did not
     * fail (or has not finished).
     * @returns {string | undefined}
     */
    get errorCode() {
        const ret = wasm.asyncjob_errorCode(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * How the job failed — `"error"`, `"cancelled"`, `"deadline"`, `"fault"` or
     * `"not-acceptable"` — or `undefined` when it did not (or has not finished).
     * @returns {string | undefined}
     */
    get errorKind() {
        const ret = wasm.asyncjob_errorKind(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * The job's failure in words, or `undefined` when it did not fail (or has not
     * finished).
     * @returns {string | undefined}
     */
    get errorMessage() {
        const ret = wasm.asyncjob_errorMessage(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * The job's failure's typed presentation as its JSON record — a SPARQL parse
     * failure's `sparql-parse-*` identity and typed parameters — or `undefined` when the
     * failure carries none (or the job did not fail, or has not finished).
     * @returns {string | undefined}
     */
    get errorPresentation() {
        const ret = wasm.asyncjob_errorPresentation(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * Whether shared exchange `exchange` is still open: some job waits on its answer.
     * Once it closes without an answer, the host aborts its call.
     * @param {bigint} exchange
     * @returns {boolean}
     */
    static exchangeIsOpen(exchange) {
        const ret = wasm.asyncjob_exchangeIsOpen(exchange);
        return ret !== 0;
    }
    /**
     * Effect `seq`'s abandonment instant ([`AsyncEffect::abandon_after_ms`]) has
     * passed with no answer. When it was the job's deadline, the deadline is latched and
     * the effect abandoned (`Abandoned`); when it was the request's own timeout, the
     * effect fails as a transport failure the `SILENT` forms absorb (`Answered`). A job
     * waiting on a shared exchange leaves it. An effect already answered is left as it
     * is (`Answered`), and a fault is reported as one (`Fault`).
     * @param {number} seq
     * @returns {SuspendStatus}
     */
    expireEffect(seq) {
        const ret = wasm.asyncjob_expireEffect(this.__wbg_ptr, seq);
        return ret;
    }
    /**
     * Latch `message` as the job's fault: a host bug the job cannot continue past. The
     * job's signal fires, it winds down, and the fault becomes its error.
     * @param {string} message
     * @returns {DeliveryStatus}
     */
    fault(message) {
        const ptr0 = passStringToWasm0(message, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.asyncjob_fault(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Fail every job waiting on shared exchange `exchange` with the host handler's fault
     * `message` — it threw, rejected or returned something the protocol does not define
     * — and close the exchange. Each waiter's effect fails as its own invocation's
     * failure; no job's fault is latched.
     * @param {bigint} exchange
     * @param {string} message
     * @returns {DeliveryStatus}
     */
    static faultExchange(exchange, message) {
        const ptr0 = passStringToWasm0(message, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.asyncjob_faultExchange(exchange, ptr0, len0);
        return ret;
    }
    /**
     * Remove the job from the registry once it has finished, and release an update's
     * claim on its dataset: returns 0, or 1 (and does nothing) while it is still
     * running, because the run is standing on the job's region.
     * @returns {number}
     */
    finish() {
        const ret = wasm.asyncjob_finish(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * The id `purrdf_jspi_run` and `purrdf_jspi_suspend` name this job by.
     * @returns {number}
     */
    get id() {
        const ret = wasm.asyncjob_id(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Whether the run has returned.
     * @returns {boolean}
     */
    get isFinished() {
        const ret = wasm.asyncjob_isFinished(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * The operation this job runs.
     * @returns {AsyncOperationKind}
     */
    get kind() {
        const ret = wasm.asyncjob_kind(this.__wbg_ptr);
        return ret;
    }
    /**
     * The status to resume effect `seq` with once the shared exchange it waited on has
     * settled: `Answered` when the exchange's answer was delivered to it, `Fault` when a
     * fault is latched (a settled exchange that delivered nothing to a job still waiting
     * on it latches one).
     * @param {number} seq
     * @returns {SuspendStatus}
     */
    settledStatus(seq) {
        const ret = wasm.asyncjob_settledStatus(this.__wbg_ptr, seq);
        return ret;
    }
    /**
     * The region's top: the stack pointer the linked run wrapper installs for the run,
     * passed as the last argument of `purrdf_jspi_run`.
     * @returns {number}
     */
    get stackTop() {
        const ret = wasm.asyncjob_stackTop(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * The effect the job is suspended on, once; `undefined` when there is none.
     * @returns {AsyncEffect | undefined}
     */
    takeEffect() {
        const ret = wasm.asyncjob_takeEffect(this.__wbg_ptr);
        return ret === 0 ? undefined : AsyncEffect.__wrap(ret);
    }
    /**
     * A governed entailment job's outcome.
     * @returns {EntailmentQueryOutcome}
     */
    takeEntailmentOutcome() {
        const ret = wasm.asyncjob_takeEntailmentOutcome(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return EntailmentQueryOutcome.__wrap(ret[0]);
    }
    /**
     * A snapshot of the job's evidence. Callable at any time, as often as wanted.
     * @returns {AsyncEvidence}
     */
    takeEvidence() {
        const ret = wasm.asyncjob_takeEvidence(this.__wbg_ptr);
        return AsyncEvidence.__wrap(ret);
    }
    /**
     * A negotiated query job's outcome: the serialized complete answer, or the trip.
     * @returns {NegotiatedOutcome}
     */
    takeNegotiatedOutcome() {
        const ret = wasm.asyncjob_takeNegotiatedOutcome(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return NegotiatedOutcome.__wrap(ret[0]);
    }
    /**
     * A governed query job's outcome.
     * @returns {QueryOutcome}
     */
    takeQueryOutcome() {
        const ret = wasm.asyncjob_takeQueryOutcome(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return QueryOutcome.__wrap(ret[0]);
    }
    /**
     * A `query` job's typed result. `expect` — `"select"`, `"ask"`, `"construct"`,
     * `"describe"` (or `"graph"`) — refuses any other result kind with the synchronous
     * twin's message; `undefined` accepts any.
     * @param {string | null} [expect]
     * @returns {QueryResult}
     */
    takeQueryResult(expect) {
        var ptr0 = isLikeNone(expect) ? 0 : passStringToWasm0(expect, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        const ret = wasm.asyncjob_takeQueryResult(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return QueryResult.__wrap(ret[0]);
    }
    /**
     * A raw job's serialized bytes, an explain job's rendered ledger, or a SHACL job's
     * SARIF log or entailed N-Triples (UTF-8 text).
     * @returns {Uint8Array}
     */
    takeRawBytes() {
        const ret = wasm.asyncjob_takeRawBytes(this.__wbg_ptr);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * A SHACL change-validation job's outcome: the SARIF log and the scope it
     * describes, exactly as `shaclValidateChangesToSarif` returns them.
     * @returns {ShaclChangeValidation}
     */
    takeShaclChangeValidation() {
        const ret = wasm.asyncjob_takeShaclChangeValidation(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ShaclChangeValidation.__wrap(ret[0]);
    }
    /**
     * A SHACL-AF entailment job's outcome: the materialized dataset and its
     * diagnostics, exactly as `shaclEntail` returns them.
     * @returns {ShaclEntailment}
     */
    takeShaclEntailment() {
        const ret = wasm.asyncjob_takeShaclEntailment(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ShaclEntailment.__wrap(ret[0]);
    }
    /**
     * A shapes-graph job's import refusal, once — the `ShaclImportError` the synchronous
     * twin throws — or `undefined` when the job was not refused that way.
     * @returns {ShaclImportError | undefined}
     */
    takeShaclImportError() {
        const ret = wasm.asyncjob_takeShaclImportError(this.__wbg_ptr);
        return ret === 0 ? undefined : ShaclImportError.__wrap(ret);
    }
    /**
     * A node-expression job's `ShaclNodeExprOutcome` — its output nodes and the shapes
     * graph's mandatory diagnostics — exactly as `shaclEvalNodeExpr` returns it.
     * @returns {ShaclNodeExprOutcome}
     */
    takeShaclNodeExprOutcome() {
        const ret = wasm.asyncjob_takeShaclNodeExprOutcome(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ShaclNodeExprOutcome.__wrap(ret[0]);
    }
    /**
     * A SHACL product job's refusal, once — the `ShaclProductRefusal` the synchronous
     * twin throws — or `undefined` when the job was not refused that way.
     * @returns {ShaclProductRefusal | undefined}
     */
    takeShaclRefusal() {
        const ret = wasm.asyncjob_takeShaclRefusal(this.__wbg_ptr);
        return ret === 0 ? undefined : ShaclProductRefusal.__wrap(ret);
    }
    /**
     * A rules job's outcome: the inference graph, proof and diagnostics, exactly as
     * `shaclApplyRules` returns them.
     * @returns {ShaclRulesInference}
     */
    takeShaclRulesInference() {
        const ret = wasm.asyncjob_takeShaclRulesInference(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ShaclRulesInference.__wrap(ret[0]);
    }
    /**
     * A governed update job's outcome. Whether it applied is on the outcome; applying it
     * to the dataset is [`Self::commit_update`].
     * @returns {UpdateOutcome}
     */
    takeUpdateOutcome() {
        const ret = wasm.asyncjob_takeUpdateOutcome(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return UpdateOutcome.__wrap(ret[0]);
    }
}
if (Symbol.dispose) AsyncJob.prototype[Symbol.dispose] = AsyncJob.prototype.free;
exports.AsyncJob = AsyncJob;

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
class AsyncJobOptions {
    static __wrap(ptr) {
        const obj = Object.create(AsyncJobOptions.prototype);
        obj.__wbg_ptr = ptr;
        AsyncJobOptionsFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        AsyncJobOptionsFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_asyncjoboptions_free(ptr, 0);
    }
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
     * @param {AsyncOperationKind} kind
     * @param {any} options
     * @param {number | null | undefined} service_handler
     * @param {boolean} load_handler
     * @param {string | null} [argument]
     * @returns {AsyncJobOptions}
     */
    static fromJs(kind, options, service_handler, load_handler, argument) {
        var ptr0 = isLikeNone(argument) ? 0 : passStringToWasm0(argument, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        const ret = wasm.asyncjoboptions_fromJs(kind, options, isLikeNone(service_handler) ? Number.MAX_SAFE_INTEGER : (service_handler) >>> 0, load_handler, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return AsyncJobOptions.__wrap(ret[0]);
    }
}
if (Symbol.dispose) AsyncJobOptions.prototype[Symbol.dispose] = AsyncJobOptions.prototype.free;
exports.AsyncJobOptions = AsyncJobOptions;

/**
 * Which operation a job runs. Each is the asynchronous twin of a synchronous entry.
 * @enum {0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9}
 */
const AsyncOperationKind = Object.freeze({
    /**
     * `query`/`select`/`ask`/`construct`/`describe`: a typed result.
     */
    Query: 0, "0": "Query",
    /**
     * `queryRaw`/`queryRawConfigured`/`Dataset.query`: serialized bytes.
     */
    Raw: 1, "1": "Raw",
    /**
     * `queryRawWithContext`: bytes serialized under a compiled JSON-LD context.
     */
    RawWithContext: 2, "2": "RawWithContext",
    /**
     * `queryGoverned`: a governed outcome.
     */
    Governed: 3, "3": "Governed",
    /**
     * `queryEntailmentGoverned`: a two-phase governed outcome.
     */
    EntailmentGoverned: 4, "4": "EntailmentGoverned",
    /**
     * `update`: applied through [`AsyncJob::commit_update`].
     */
    Update: 5, "5": "Update",
    /**
     * `updateGoverned`: a governed outcome, applied through [`AsyncJob::commit_update`].
     */
    UpdateGoverned: 6, "6": "UpdateGoverned",
    /**
     * `queryGovernedNegotiatedAsync`: a governed outcome whose complete answer is
     * serialized in the format negotiated from an `Accept` header, once the result's
     * shape is known. The asynchronous lane's own: a SPARQL Protocol endpoint's query.
     */
    Negotiated: 7, "7": "Negotiated",
    /**
     * `explainQuery`: the rendered charge ledger, taken through
     * [`AsyncJob::take_raw_bytes`]. EXPLAIN evaluates the query to measure it, so its
     * measuring run suspends on `SERVICE`, yields and stops exactly as a query does.
     */
    Explain: 8, "8": "Explain",
    /**
     * A SHACL entry that can evaluate SPARQL — validation to SARIF, change validation,
     * SHACL-AF entailment, a rules run, a node expression, or validation with a
     * prepared product (`ShaclRequest` names which). Its `sh:SPARQLTarget` queries,
     * SHACL-SPARQL constraints, node expressions and rules run under the job's signal,
     * and the signal is polled between focus nodes too, so a validation with no SPARQL
     * in it still yields and stops. Started through `AsyncJob.beginShacl`.
     */
    Shacl: 9, "9": "Shacl",
});
exports.AsyncOperationKind = AsyncOperationKind;

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
class CancellationToken {
    static __wrap(ptr) {
        const obj = Object.create(CancellationToken.prototype);
        obj.__wbg_ptr = ptr;
        CancellationTokenFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        CancellationTokenFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_cancellationtoken_free(ptr, 0);
    }
    /**
     * Cancel every governed call holding this token. Idempotent, and never reversible.
     */
    cancel() {
        wasm.cancellationtoken_cancel(this.__wbg_ptr);
    }
    /**
     * Whether this token has been cancelled.
     * @returns {boolean}
     */
    get isCancelled() {
        const ret = wasm.cancellationtoken_isCancelled(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * A fresh, uncancelled token.
     */
    constructor() {
        const ret = wasm.cancellationtoken_new();
        this.__wbg_ptr = ret;
        CancellationTokenFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
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
     * @returns {CancellationToken}
     */
    share() {
        const ret = wasm.cancellationtoken_share(this.__wbg_ptr);
        return CancellationToken.__wrap(ret);
    }
}
if (Symbol.dispose) CancellationToken.prototype[Symbol.dispose] = CancellationToken.prototype.free;
exports.CancellationToken = CancellationToken;

/**
 * An immutable JSON-LD 1.1 context compiled once and reusable across datasets.
 */
class CompiledJsonLdContext {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        CompiledJsonLdContextFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_compiledjsonldcontext_free(ptr, 0);
    }
    /**
     * Return the recursively canonical context document as JSON.
     * @returns {string}
     */
    canonicalContextJson() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.compiledjsonldcontext_canonicalContextJson(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Compile the context branch of a versioned JSON-LD options document.
     * @param {string} options_json
     */
    constructor(options_json) {
        const ptr0 = passStringToWasm0(options_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.compiledjsonldcontext_new(ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        CompiledJsonLdContextFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
}
if (Symbol.dispose) CompiledJsonLdContext.prototype[Symbol.dispose] = CompiledJsonLdContext.prototype.free;
exports.CompiledJsonLdContext = CompiledJsonLdContext;

/**
 * An RDF/JS `DataFactory`. Stateless except for the auto-generated blank-node
 * counter (`blankNode()` with no argument mints a fresh label).
 */
class DataFactory {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        DataFactoryFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datafactory_free(ptr, 0);
    }
    /**
     * `blankNode(value?)` → a `BlankNode` term; a fresh label is minted when omitted.
     * @param {string | null} [value]
     * @returns {Term}
     */
    blankNode(value) {
        var ptr0 = isLikeNone(value) ? 0 : passStringToWasm0(value, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        const ret = wasm.datafactory_blankNode(this.__wbg_ptr, ptr0, len0);
        return Term.__wrap(ret);
    }
    /**
     * `defaultGraph()` → the `DefaultGraph` term.
     * @returns {Term}
     */
    defaultGraph() {
        const ret = wasm.datafactory_defaultGraph(this.__wbg_ptr);
        return Term.__wrap(ret);
    }
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
     * @param {string} value
     * @param {string} language
     * @param {string} direction
     * @returns {Term}
     */
    directionalLiteral(value, language, direction) {
        const ptr0 = passStringToWasm0(value, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(language, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(direction, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.datafactory_directionalLiteral(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return Term.__wrap(ret[0]);
    }
    /**
     * `fromQuad(original)` → a copy of `original`.
     * @param {Quad} original
     * @returns {Quad}
     */
    fromQuad(original) {
        _assertClass(original, Quad);
        const ret = wasm.datafactory_fromQuad(this.__wbg_ptr, original.__wbg_ptr);
        return Quad.__wrap(ret);
    }
    /**
     * `fromTerm(original)` → a copy of `original` (RDF/JS structural clone).
     * @param {Term} original
     * @returns {Term}
     */
    fromTerm(original) {
        _assertClass(original, Term);
        const ret = wasm.datafactory_fromTerm(this.__wbg_ptr, original.__wbg_ptr);
        return Term.__wrap(ret);
    }
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
     * @param {string} value
     * @param {string | null} [language]
     * @returns {Term}
     */
    literal(value, language) {
        const ptr0 = passStringToWasm0(value, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(language) ? 0 : passStringToWasm0(language, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        const ret = wasm.datafactory_literal(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return Term.__wrap(ret[0]);
    }
    /**
     * `namedNode(value)` → a `NamedNode` term.
     * @param {string} value
     * @returns {Term}
     */
    namedNode(value) {
        const ptr0 = passStringToWasm0(value, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.datafactory_namedNode(this.__wbg_ptr, ptr0, len0);
        return Term.__wrap(ret);
    }
    /**
     * `new DataFactory()` — a fresh factory with its blank-node counter at zero.
     *
     * A written-out function rather than the type's `Default`: `#[wasm_bindgen(constructor)]`
     * exports only a function of the `#[wasm_bindgen]` impl block, as `#[new]` does in a
     * `#[pymethods]` block, so every exported constructor has this one-line body.
     */
    constructor() {
        const ret = wasm.datafactory_new();
        this.__wbg_ptr = ret;
        DataFactoryFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * `quad(subject, predicate, object, graph?)` → a `Quad`. The graph defaults to the
     * default graph. A quoted-triple term (from [`DataFactory::quoted_triple`]) may be
     * passed as `subject` or `object` (the RDF-1.2 wedge).
     * @param {Term} subject
     * @param {Term} predicate
     * @param {Term} object
     * @param {Term | null} [graph]
     * @returns {Quad}
     */
    quad(subject, predicate, object, graph) {
        _assertClass(subject, Term);
        _assertClass(predicate, Term);
        _assertClass(object, Term);
        let ptr0 = 0;
        if (!isLikeNone(graph)) {
            _assertClass(graph, Term);
            ptr0 = graph.__destroy_into_raw();
        }
        const ret = wasm.datafactory_quad(this.__wbg_ptr, subject.__wbg_ptr, predicate.__wbg_ptr, object.__wbg_ptr, ptr0);
        return Quad.__wrap(ret);
    }
    /**
     * `quotedTriple(subject, predicate, object)` → a quoted-triple `Term`
     * (`termType: "Quad"`) — the RDF-1.2 wedge. Embed it by passing it as the
     * `subject`/`object` of another quad.
     * @param {Term} subject
     * @param {Term} predicate
     * @param {Term} object
     * @returns {Term}
     */
    quotedTriple(subject, predicate, object) {
        _assertClass(subject, Term);
        _assertClass(predicate, Term);
        _assertClass(object, Term);
        const ret = wasm.datafactory_quotedTriple(this.__wbg_ptr, subject.__wbg_ptr, predicate.__wbg_ptr, object.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return Term.__wrap(ret[0]);
    }
    /**
     * `typedLiteral(value, datatype)` → a datatyped literal. `datatype` must be a
     * `NamedNode`. (The RDF/JS `literal(value, datatype)` form, surfaced by the TS
     * wrapper.)
     * @param {string} value
     * @param {Term} datatype
     * @returns {Term}
     */
    typedLiteral(value, datatype) {
        const ptr0 = passStringToWasm0(value, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertClass(datatype, Term);
        const ret = wasm.datafactory_typedLiteral(this.__wbg_ptr, ptr0, len0, datatype.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return Term.__wrap(ret[0]);
    }
    /**
     * `variable(value)` → a `Variable` term.
     * @param {string} value
     * @returns {Term}
     */
    variable(value) {
        const ptr0 = passStringToWasm0(value, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.datafactory_variable(this.__wbg_ptr, ptr0, len0);
        return Term.__wrap(ret);
    }
}
if (Symbol.dispose) DataFactory.prototype[Symbol.dispose] = DataFactory.prototype.free;
exports.DataFactory = DataFactory;

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
class Dataset {
    static __wrap(ptr) {
        const obj = Object.create(Dataset.prototype);
        obj.__wbg_ptr = ptr;
        DatasetFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    static __unwrap(jsValue) {
        if (!(jsValue instanceof Dataset)) {
            return 0;
        }
        return jsValue.__destroy_into_raw();
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        DatasetFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_dataset_free(ptr, 0);
    }
    /**
     * `add(quad)` → insert a quad. Returns `true` if the effective set changed.
     *
     * Throws if the quad carries a relative IRI in any position: this surface is
     * handed terms, never a document, so there is no base in scope to resolve one
     * against and none is invented. The thrown message carries the workspace's
     * shared `iri-relative-no-base` code.
     * @param {Quad} quad
     * @returns {boolean}
     */
    add(quad) {
        _assertClass(quad, Quad);
        const ret = wasm.dataset_add(this.__wbg_ptr, quad.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] !== 0;
    }
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
     * @returns {string}
     */
    canonicalize() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.dataset_canonicalize(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * `delete(quad)` → remove a quad. Returns `true` if the effective set changed.
     * @param {Quad} quad
     * @returns {boolean}
     */
    delete(quad) {
        _assertClass(quad, Quad);
        const ret = wasm.dataset_delete(this.__wbg_ptr, quad.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] !== 0;
    }
    /**
     * `generation` — how many mutations this dataset's content has seen.
     *
     * Advances on every `add` or `delete` that changed the effective set and on every
     * applied UPDATE; reading, querying and serializing never move it. An asynchronous
     * UPDATE captures it when it starts and refuses to commit if it has moved since.
     * @returns {bigint}
     */
    get generation() {
        const ret = wasm.dataset_generation(this.__wbg_ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
     * `has(quad)` → whether the quad is in the dataset.
     * @param {Quad} quad
     * @returns {boolean}
     */
    has(quad) {
        _assertClass(quad, Quad);
        const ret = wasm.dataset_has(this.__wbg_ptr, quad.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] !== 0;
    }
    /**
     * `id` — this dataset's identity, unique within the wasm instance and never reused.
     *
     * An exact JavaScript `bigint`, including identities above `2^53`.
     * @returns {bigint}
     */
    get id() {
        const ret = wasm.dataset_id(this.__wbg_ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
     * `isomorphic(other)` → whether this dataset and `other` are the same RDF graph
     * under blank-node relabeling.
     *
     * The formal RDF graph-identity check, backed by full RDFC-1.0 canonicalization:
     * an exact oracle with no false positives or false negatives. Equivalent to
     * comparing the two [`canonicalize`](Self::canonicalize) strings, but avoids
     * materializing them for obviously-different inputs.
     * @param {Dataset} other
     * @returns {boolean}
     */
    isomorphic(other) {
        _assertClass(other, Dataset);
        const ret = wasm.dataset_isomorphic(this.__wbg_ptr, other.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] !== 0;
    }
    /**
     * `match(subject?, predicate?, object?, graph?)` → a new dataset of the matching
     * quads. An omitted (`undefined`) position is a wildcard; `defaultGraph()` matches
     * only the default graph, a named node matches that graph.
     * @param {Term | null} [subject]
     * @param {Term | null} [predicate]
     * @param {Term | null} [object]
     * @param {Term | null} [graph]
     * @returns {Dataset}
     */
    match(subject, predicate, object, graph) {
        let ptr0 = 0;
        if (!isLikeNone(subject)) {
            _assertClass(subject, Term);
            ptr0 = subject.__destroy_into_raw();
        }
        let ptr1 = 0;
        if (!isLikeNone(predicate)) {
            _assertClass(predicate, Term);
            ptr1 = predicate.__destroy_into_raw();
        }
        let ptr2 = 0;
        if (!isLikeNone(object)) {
            _assertClass(object, Term);
            ptr2 = object.__destroy_into_raw();
        }
        let ptr3 = 0;
        if (!isLikeNone(graph)) {
            _assertClass(graph, Term);
            ptr3 = graph.__destroy_into_raw();
        }
        const ret = wasm.dataset_match(this.__wbg_ptr, ptr0, ptr1, ptr2, ptr3);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return Dataset.__wrap(ret[0]);
    }
    /**
     * An empty dataset.
     */
    constructor() {
        const ret = wasm.dataset_new();
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        DatasetFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * `parse(input, format, base?)` → a dataset of the parsed quads.
     *
     * `format` is a media type or short name
     * (turtle/ntriples/nquads/trig/rdfxml/jsonld/yamlld).
     * Ill-typed literals are preserved verbatim (RDFLib parity), not rejected.
     * @param {string} input
     * @param {string} format
     * @param {string | null} [base]
     * @returns {Dataset}
     */
    static parse(input, format, base) {
        const ptr0 = passStringToWasm0(input, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(format, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(base) ? 0 : passStringToWasm0(base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.dataset_parse(ptr0, len0, ptr1, len1, ptr2, len2);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return Dataset.__wrap(ret[0]);
    }
    /**
     * Project this dataset into a deterministic graph, tabular, or research-object USTAR package.
     * @param {string} profile
     * @param {string} config_json
     * @returns {ProjectionPackage}
     */
    project(profile, config_json) {
        const ptr0 = passStringToWasm0(profile, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(config_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.dataset_project(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ProjectionPackage.__wrap(ret[0]);
    }
    /**
     * Project this dataset plus a canonical payload-only USTAR into an attached RO-Crate.
     * @param {string} profile
     * @param {string} config_json
     * @param {Uint8Array} assets_archive
     * @returns {ProjectionPackage}
     */
    projectWithAssets(profile, config_json, assets_archive) {
        const ptr0 = passStringToWasm0(profile, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(config_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passArray8ToWasm0(assets_archive, wasm.__wbindgen_malloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.dataset_projectWithAssets(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ProjectionPackage.__wrap(ret[0]);
    }
    /**
     * `quads()` → every effective quad, as a JS array.
     * @returns {Quad[]}
     */
    quads() {
        const ret = wasm.dataset_quads(this.__wbg_ptr);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
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
     * (`"18:toward-zero"`); set `divisionPolicy` on a `QueryEngine` for another.
     * @param {string} sparql
     * @param {string | null} [base]
     * @param {string | null} [xpath_regex]
     * @returns {string}
     */
    query(sparql, base, xpath_regex) {
        let deferred5_0;
        let deferred5_1;
        try {
            const ptr0 = passStringToWasm0(sparql, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            var ptr1 = isLikeNone(base) ? 0 : passStringToWasm0(base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            var ptr2 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len2 = WASM_VECTOR_LEN;
            const ret = wasm.dataset_query(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
            var ptr4 = ret[0];
            var len4 = ret[1];
            if (ret[3]) {
                ptr4 = 0; len4 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred5_0 = ptr4;
            deferred5_1 = len4;
            return getStringFromWasm0(ptr4, len4);
        } finally {
            wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
        }
    }
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
     * @param {string} format
     * @param {string | null} [base]
     * @returns {string}
     */
    serialize(format, base) {
        let deferred4_0;
        let deferred4_1;
        try {
            const ptr0 = passStringToWasm0(format, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            var ptr1 = isLikeNone(base) ? 0 : passStringToWasm0(base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            const ret = wasm.dataset_serialize(this.__wbg_ptr, ptr0, len0, ptr1, len1);
            var ptr3 = ret[0];
            var len3 = ret[1];
            if (ret[3]) {
                ptr3 = 0; len3 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred4_0 = ptr3;
            deferred4_1 = len3;
            return getStringFromWasm0(ptr3, len3);
        } finally {
            wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
        }
    }
    /**
     * Serialize JSON-LD/YAML-LD using the shared versioned options decoder.
     *
     * `base` is the document base the output is written under — the egress mirror of
     * [`parse`](Self::parse)'s. Both JSON-LD and YAML-LD can express a base, so it
     * reaches the emitted `@context` as `@base` and document-position `@id`s are
     * compacted against it. A base the caller's own context already declares wins,
     * matching the ingress precedence. A base that is not an absolute IRI throws.
     * @param {string} format
     * @param {string} options_json
     * @param {string | null} [base]
     * @returns {string}
     */
    serializeConfigured(format, options_json, base) {
        let deferred5_0;
        let deferred5_1;
        try {
            const ptr0 = passStringToWasm0(format, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(options_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            var ptr2 = isLikeNone(base) ? 0 : passStringToWasm0(base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len2 = WASM_VECTOR_LEN;
            const ret = wasm.dataset_serializeConfigured(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
            var ptr4 = ret[0];
            var len4 = ret[1];
            if (ret[3]) {
                ptr4 = 0; len4 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred5_0 = ptr4;
            deferred5_1 = len4;
            return getStringFromWasm0(ptr4, len4);
        } finally {
            wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
        }
    }
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
     * @param {string} format
     * @param {string | null | undefined} base
     * @param {{ write(chunk: Uint8Array): void }} sink
     */
    serializeToSink(format, base, sink) {
        const ptr0 = passStringToWasm0(format, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(base) ? 0 : passStringToWasm0(base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        const ret = wasm.dataset_serializeToSink(this.__wbg_ptr, ptr0, len0, ptr1, len1, sink);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Serialize JSON-LD/YAML-LD using a reusable compiled context.
     *
     * `base` is the document base, honored exactly as in
     * [`serializeConfigured`](Self::serialize_configured).
     * @param {string} format
     * @param {CompiledJsonLdContext} context
     * @param {string | null} [yaml_schema_url]
     * @param {string | null} [base]
     * @returns {string}
     */
    serializeWithContext(format, context, yaml_schema_url, base) {
        let deferred5_0;
        let deferred5_1;
        try {
            const ptr0 = passStringToWasm0(format, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            _assertClass(context, CompiledJsonLdContext);
            var ptr1 = isLikeNone(yaml_schema_url) ? 0 : passStringToWasm0(yaml_schema_url, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            var ptr2 = isLikeNone(base) ? 0 : passStringToWasm0(base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len2 = WASM_VECTOR_LEN;
            const ret = wasm.dataset_serializeWithContext(this.__wbg_ptr, ptr0, len0, context.__wbg_ptr, ptr1, len1, ptr2, len2);
            var ptr4 = ret[0];
            var len4 = ret[1];
            if (ret[3]) {
                ptr4 = 0; len4 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred5_0 = ptr4;
            deferred5_1 = len4;
            return getStringFromWasm0(ptr4, len4);
        } finally {
            wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
        }
    }
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
     * @param {string} format
     * @param {string | null} [base]
     * @returns {SerializeLoss}
     */
    serializeWithLoss(format, base) {
        const ptr0 = passStringToWasm0(format, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(base) ? 0 : passStringToWasm0(base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        const ret = wasm.dataset_serializeWithLoss(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return SerializeLoss.__wrap(ret[0]);
    }
    /**
     * `size` — the number of effective quads.
     * @returns {number}
     */
    get size() {
        const ret = wasm.dataset_size(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * `snapshot()` → an independent dataset holding this one's current content, with an
     * identity of its own at generation zero. Later changes to either leave the other as
     * it is; the content is shared until one of them changes.
     *
     * # Errors
     *
     * A dataset whose pending changes cannot be frozen.
     * @returns {Dataset}
     */
    snapshot() {
        const ret = wasm.dataset_snapshot(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return Dataset.__wrap(ret[0]);
    }
    /**
     * `visualExportJson(optionsJson?)` -> model, scene, geometry, and index as JSON.
     * @param {string | null} [options_json]
     * @returns {string}
     */
    visualExportJson(options_json) {
        let deferred3_0;
        let deferred3_1;
        try {
            var ptr0 = isLikeNone(options_json) ? 0 : passStringToWasm0(options_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len0 = WASM_VECTOR_LEN;
            const ret = wasm.dataset_visualExportJson(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * `visualModelJson(optionsJson?)` -> the renderer-neutral RDF 1.2 model as JSON.
     * @param {string | null} [options_json]
     * @returns {string}
     */
    visualModelJson(options_json) {
        let deferred3_0;
        let deferred3_1;
        try {
            var ptr0 = isLikeNone(options_json) ? 0 : passStringToWasm0(options_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len0 = WASM_VECTOR_LEN;
            const ret = wasm.dataset_visualModelJson(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * `visualSvgJson(optionsJson?)` -> deterministic SVG and its complete export.
     * @param {string | null} [options_json]
     * @returns {string}
     */
    visualSvgJson(options_json) {
        let deferred3_0;
        let deferred3_1;
        try {
            var ptr0 = isLikeNone(options_json) ? 0 : passStringToWasm0(options_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len0 = WASM_VECTOR_LEN;
            const ret = wasm.dataset_visualSvgJson(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
}
if (Symbol.dispose) Dataset.prototype[Symbol.dispose] = Dataset.prototype.free;
exports.Dataset = Dataset;

/**
 * What a delivery (and `fault`, `cancel`) reports.
 * @enum {0 | 1 | 2 | 3}
 */
const DeliveryStatus = Object.freeze({
    /**
     * The delivery answers the outstanding effect.
     */
    Accepted: 0, "0": "Accepted",
    /**
     * The delivery named an effect that was already answered or abandoned — typically a
     * host promise settling after the job's signal won the race. Ignored; not a fault.
     */
    Stale: 1, "1": "Stale",
    /**
     * The delivery was malformed (a sequence number never issued, a payload the
     * outstanding effect cannot take). The job's fault is latched.
     */
    Fault: 2, "2": "Fault",
    /**
     * The job (or the shared exchange) has already finished; the delivery changes
     * nothing.
     */
    Finished: 3, "3": "Finished",
});
exports.DeliveryStatus = DeliveryStatus;

/**
 * What an effect asks the host to do. The numbering is the protocol's; 4 is reserved
 * for paged datasets.
 * @enum {1 | 2 | 3 | 5}
 */
const EffectKind = Object.freeze({
    /**
     * Answer a forwarded `SERVICE` query: the host calls its `resolveService` once and
     * delivers the answer to the effect's shared exchange ([`AsyncJob::deliver_exchange_bindings`]
     * and siblings), which every job waiting on it receives.
     */
    Service: 1, "1": "Service",
    /**
     * Fetch a `LOAD` document.
     */
    Load: 2, "2": "Load",
    /**
     * Turn the event loop once and resume.
     */
    Yield: 3, "3": "Yield",
    /**
     * Wait for a `SERVICE` exchange another job opened, whose answer this job receives
     * when that exchange is delivered.
     */
    AwaitExchange: 5, "5": "AwaitExchange",
});
exports.EffectKind = EffectKind;

/**
 * The two-phase outcome of a governed entailment-aware query.
 *
 * `takeOutcome()` and `report` are present together only after closure completed. A
 * closure-phase stop carries neither, preventing a host from treating an incomplete
 * closure as queryable data or as a reasoning certificate.
 */
class EntailmentQueryOutcome {
    static __wrap(ptr) {
        const obj = Object.create(EntailmentQueryOutcome.prototype);
        obj.__wbg_ptr = ptr;
        EntailmentQueryOutcomeFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        EntailmentQueryOutcomeFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_entailmentqueryoutcome_free(ptr, 0);
    }
    /**
     * Whether the stop happened before a closure existed.
     * @returns {boolean}
     */
    get closureStopped() {
        const ret = wasm.entailmentqueryoutcome_closureStopped(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Whether both closure and query completed under every governor.
     * @returns {boolean}
     */
    get isComplete() {
        const ret = wasm.entailmentqueryoutcome_isComplete(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Byte-stable reasoning report, absent when closure stopped.
     * @returns {string | undefined}
     */
    get report() {
        const ret = wasm.entailmentqueryoutcome_report(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * Move phase two's governed query outcome out, or `undefined` if closure stopped.
     * @returns {QueryOutcome | undefined}
     */
    takeOutcome() {
        const ret = wasm.entailmentqueryoutcome_takeOutcome(this.__wbg_ptr);
        return ret === 0 ? undefined : QueryOutcome.__wrap(ret);
    }
    /**
     * Move the governor that stopped either phase out, or `undefined` on completion.
     * @returns {TrippedGovernor | undefined}
     */
    takeTripped() {
        const ret = wasm.entailmentqueryoutcome_takeTripped(this.__wbg_ptr);
        return ret === 0 ? undefined : TrippedGovernor.__wrap(ret);
    }
}
if (Symbol.dispose) EntailmentQueryOutcome.prototype[Symbol.dispose] = EntailmentQueryOutcome.prototype.free;
exports.EntailmentQueryOutcome = EntailmentQueryOutcome;

/**
 * The HTTP problem a failed operation is answered with: build it with
 * [`SparqlProtocolRequest::problem_for`], read its `status` and whether it is
 * `internal`, and render its `body`.
 */
class FailureProblem {
    static __wrap(ptr) {
        const obj = Object.create(FailureProblem.prototype);
        obj.__wbg_ptr = ptr;
        FailureProblemFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        FailureProblemFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_failureproblem_free(ptr, 0);
    }
    /**
     * The RFC 9457 problem document: `type`, `title`, `status`, `detail` and `code`,
     * plus `correlationId` for an internal failure and `offered` for a `406`.
     *
     * # Errors
     *
     * An internal failure without a `correlationId`.
     * @param {string | null} [correlation_id]
     * @returns {string}
     */
    body(correlation_id) {
        let deferred3_0;
        let deferred3_1;
        try {
            var ptr0 = isLikeNone(correlation_id) ? 0 : passStringToWasm0(correlation_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len0 = WASM_VECTOR_LEN;
            const ret = wasm.failureproblem_body(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * The response's `Content-Type`: `application/problem+json`.
     * @returns {string}
     */
    get contentType() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.failureproblem_contentType(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Whether the failure is the host's own: its real error belongs in the host's log
     * under a correlation id, which `body` must be given and the response carries.
     * @returns {boolean}
     */
    get internal() {
        const ret = wasm.failureproblem_internal(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * The response status.
     * @returns {number}
     */
    get status() {
        const ret = wasm.failureproblem_status(this.__wbg_ptr);
        return ret;
    }
}
if (Symbol.dispose) FailureProblem.prototype[Symbol.dispose] = FailureProblem.prototype.free;
exports.FailureProblem = FailureProblem;

/**
 * Reusable immutable native point-cell index with strict search requests.
 */
class GeoPointIndex {
    static __wrap(ptr) {
        const obj = Object.create(GeoPointIndex.prototype);
        obj.__wbg_ptr = ptr;
        GeoPointIndexFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        GeoPointIndexFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_geopointindex_free(ptr, 0);
    }
    /**
     * Search stored buckets using the exact selected public comparison law.
     * # Errors
     * Preserves operational refusal before allocating a response envelope.
     * @param {string} request
     * @returns {string}
     */
    call(request) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(request, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.geopointindex_call(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
}
if (Symbol.dispose) GeoPointIndex.prototype[Symbol.dispose] = GeoPointIndex.prototype.free;
exports.GeoPointIndex = GeoPointIndex;

/**
 * Immutable geographic profile and scalar/batch request session.
 */
class GeoSession {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        GeoSessionFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_geosession_free(ptr, 0);
    }
    /**
     * Execute a strict version-one scalar/batch request and return its response.
     * Semantic refusals remain JSON records alongside the same identities.
     * # Errors
     * Operational refusal returns through the host error channel without
     * constructing an error envelope after the original account refuses.
     * @param {string} request
     * @returns {string}
     */
    call(request) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(request, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.geosession_call(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Prepare an explicit version-one profile, or the standard CRS84 profile.
     *
     * # Errors
     *
     * Refuses malformed records, conflicting references and invalid limits.
     * @param {string | null} [profile]
     */
    constructor(profile) {
        var ptr0 = isLikeNone(profile) ? 0 : passStringToWasm0(profile, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        const ret = wasm.geosession_new(ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        GeoSessionFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Prepare immutable native ordered point buckets from a strict request.
     *
     * # Errors
     * Refuses invalid source records, reference mismatch and incomplete admission.
     * @param {string} request
     * @returns {GeoPointIndex}
     */
    pointIndex(request) {
        const ptr0 = passStringToWasm0(request, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.geosession_pointIndex(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return GeoPointIndex.__wrap(ret[0]);
    }
    /**
     * Return the canonical strict profile record.
     *
     * # Errors
     *
     * Refuses a native custom parameter without an exact decimal representation.
     * @returns {string}
     */
    profile() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.geosession_profile(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
}
if (Symbol.dispose) GeoSession.prototype[Symbol.dispose] = GeoSession.prototype.free;
exports.GeoSession = GeoSession;

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
class GovernorEvidence {
    static __wrap(ptr) {
        const obj = Object.create(GovernorEvidence.prototype);
        obj.__wbg_ptr = ptr;
        GovernorEvidenceFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        GovernorEvidenceFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_governorevidence_free(ptr, 0);
    }
    /**
     * Consumption charged per dimension, positionally by [`governor_dimensions`].
     *
     * A peak-tracked dimension (`intermediate-cells`, `udf-depth`) reports the largest
     * single observation; every other dimension reports the running sum.
     * @returns {BigUint64Array}
     */
    get consumed() {
        const ret = wasm.governorevidence_consumed(this.__wbg_ptr);
        var v1 = getArrayU64FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
        return v1;
    }
    /**
     * Every XPath F&O numeric error the execution absorbed into an unbound value (an
     * expression error is not a query error, SPARQL 1.1 §17.2), counted positionally by
     * [`expression_error_codes`]: `1/0` counts under `err:FOAR0001`. All zero when no
     * expression failed.
     * @returns {BigUint64Array}
     */
    get expressionErrors() {
        const ret = wasm.governorevidence_expressionErrors(this.__wbg_ptr);
        var v1 = getArrayU64FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
        return v1;
    }
    /**
     * Whether the execution completed with every governor intact.
     *
     * The governor itself is on the outcome rather than duplicated here: one trip, one
     * object, so a consumer cannot read two and wonder which is authoritative.
     * @returns {boolean}
     */
    get isComplete() {
        const ret = wasm.governorevidence_isComplete(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * The inclusive ceilings in force, positionally by [`governor_dimensions`].
     *
     * A governed call meters every caller-settable dimension, so a dimension the caller
     * declined reads `2**64 - 2` — engaged, at a ceiling no execution can reach.
     * @returns {BigUint64Array}
     */
    get limits() {
        const ret = wasm.governorevidence_limits(this.__wbg_ptr);
        var v1 = getArrayU64FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
        return v1;
    }
    /**
     * Every invocation a `SERVICE SILENT` or `LOAD SILENT` absorbed: the answer the
     * specification requires for it is indistinguishable from an endpoint with nothing
     * to add, and this is where the difference is kept. Empty when nothing failed.
     * @returns {SilencedInvocation[]}
     */
    get silenced() {
        const ret = wasm.governorevidence_silenced(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
}
if (Symbol.dispose) GovernorEvidence.prototype[Symbol.dispose] = GovernorEvidence.prototype.free;
exports.GovernorEvidence = GovernorEvidence;

/**
 * The outcome of one governed query whose complete answer was serialized in the format
 * negotiated from an `Accept` header (`queryGovernedNegotiatedAsync`).
 *
 * The same two shapes as [`QueryOutcome`], with the complete answer as a document rather
 * than a typed result: `takeBody()` with its `format` and `mediaType` when every governor
 * held, `takePartial()` with `takeTripped()` when one did not. The partial rows stay
 * typed — they are a certificate to inspect, not a document to send.
 */
class NegotiatedOutcome {
    static __wrap(ptr) {
        const obj = Object.create(NegotiatedOutcome.prototype);
        obj.__wbg_ptr = ptr;
        NegotiatedOutcomeFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        NegotiatedOutcomeFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_negotiatedoutcome_free(ptr, 0);
    }
    /**
     * The negotiated format token (`json`, `xml`, `csv`, `tsv`, `turtle`, `trig`,
     * `ntriples`, `nquads` or `jsonld`), or `undefined` when a governor stopped the
     * execution.
     * @returns {string | undefined}
     */
    get format() {
        const ret = wasm.negotiatedoutcome_format(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * Whether every governor stayed intact and the body is the query's complete answer.
     * @returns {boolean}
     */
    get isComplete() {
        const ret = wasm.negotiatedoutcome_isComplete(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * The body's media type, for the response's `Content-Type`, or `undefined` when a
     * governor stopped the execution.
     * @returns {string | undefined}
     */
    get mediaType() {
        const ret = wasm.negotiatedoutcome_mediaType(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * Move the complete answer's UTF-8 bytes out, or `undefined` when a governor stopped
     * the execution.
     * @returns {Uint8Array | undefined}
     */
    takeBody() {
        const ret = wasm.negotiatedoutcome_takeBody(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * Move this execution's receipt out. Present on both paths.
     * @returns {GovernorEvidence | undefined}
     */
    takeEvidence() {
        const ret = wasm.negotiatedoutcome_takeEvidence(this.__wbg_ptr);
        return ret === 0 ? undefined : GovernorEvidence.__wrap(ret);
    }
    /**
     * Move the partial-answer certificate out, or `undefined` when the query completed.
     * @returns {PartialAnswers | undefined}
     */
    takePartial() {
        const ret = wasm.negotiatedoutcome_takePartial(this.__wbg_ptr);
        return ret === 0 ? undefined : PartialAnswers.__wrap(ret);
    }
    /**
     * Move the tripped governor out, or `undefined` when the query completed.
     * @returns {TrippedGovernor | undefined}
     */
    takeTripped() {
        const ret = wasm.negotiatedoutcome_takeTripped(this.__wbg_ptr);
        return ret === 0 ? undefined : TrippedGovernor.__wrap(ret);
    }
}
if (Symbol.dispose) NegotiatedOutcome.prototype[Symbol.dispose] = NegotiatedOutcome.prototype.free;
exports.NegotiatedOutcome = NegotiatedOutcome;

/**
 * What the rows a truncated execution reached bound, relative to the query's true answer.
 *
 * A three-way interval, not a yes/no: `"certain"` rows are a certified **lower** bound and
 * are safe to admit as answers; `"at-most"` rows are a certified **upper** bound and are
 * sound only for the negative reading (a row absent from them is definitively not an
 * answer); `"unknown"` means neither bound survived, so **no row is handed over at all**
 * and [`Self::barrier`] names the operator that withheld them instead.
 */
class PartialAnswers {
    static __wrap(ptr) {
        const obj = Object.create(PartialAnswers.prototype);
        obj.__wbg_ptr = ptr;
        PartialAnswersFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        PartialAnswersFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_partialanswers_free(ptr, 0);
    }
    /**
     * The algebra operator that withheld the rows, on the `"unknown"` class — which is
     * what says whether a larger budget or a different query is the way forward.
     * @returns {string | undefined}
     */
    get barrier() {
        const ret = wasm.partialanswers_barrier(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * What these rows certify: `"certain"`, `"at-most"`, or `"unknown"`.
     * @returns {string}
     */
    get certainty() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.partialanswers_certainty(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Whether these rows are certified answers — i.e. whether they may be admitted.
     * @returns {boolean}
     */
    get isCertain() {
        const ret = wasm.partialanswers_isCertain(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Whether these rows are the true answer's **first** rows, in order. This licenses
     * resumption by raising a deterministic ceiling; a wall-deadline rerun is fresh and
     * may stop sooner. `undefined` on the `"unknown"` class.
     * @returns {boolean | undefined}
     */
    get isPositionalPrefix() {
        const ret = wasm.partialanswers_isPositionalPrefix(this.__wbg_ptr);
        return ret === 0xFFFFFF ? undefined : ret !== 0;
    }
    /**
     * Move the rows in hand out of this certificate, or `undefined` on the `"unknown"`
     * class, where rows that bound the answer on neither side offer no sound use and one
     * unsound one.
     * @returns {QueryResult | undefined}
     */
    takeResult() {
        const ret = wasm.partialanswers_takeResult(this.__wbg_ptr);
        return ret === 0 ? undefined : QueryResult.__wrap(ret);
    }
}
if (Symbol.dispose) PartialAnswers.prototype[Symbol.dispose] = PartialAnswers.prototype.free;
exports.PartialAnswers = PartialAnswers;

/**
 * Result of lifting a strict carrier package into an in-memory RDF dataset.
 */
class ProjectionLift {
    static __wrap(ptr) {
        const obj = Object.create(ProjectionLift.prototype);
        obj.__wbg_ptr = ptr;
        ProjectionLiftFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        ProjectionLiftFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_projectionlift_free(ptr, 0);
    }
    /**
     * Canonical, versioned runtime loss-ledger JSON.
     * @returns {string}
     */
    get lossLedgerJson() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.projectionlift_lossLedgerJson(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Move the lifted dataset out of this result. The dataset can be taken once.
     * @returns {Dataset | undefined}
     */
    takeDataset() {
        const ret = wasm.projectionlift_takeDataset(this.__wbg_ptr);
        return ret === 0 ? undefined : Dataset.__wrap(ret);
    }
}
if (Symbol.dispose) ProjectionLift.prototype[Symbol.dispose] = ProjectionLift.prototype.free;
exports.ProjectionLift = ProjectionLift;

/**
 * A deterministic USTAR projection package and its canonical runtime ledger.
 */
class ProjectionPackage {
    static __wrap(ptr) {
        const obj = Object.create(ProjectionPackage.prototype);
        obj.__wbg_ptr = ptr;
        ProjectionPackageFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        ProjectionPackageFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_projectionpackage_free(ptr, 0);
    }
    /**
     * Canonical deterministic USTAR bytes.
     * @returns {Uint8Array}
     */
    get archive() {
        const ret = wasm.projectionpackage_archive(this.__wbg_ptr);
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * Canonical, versioned runtime loss-ledger JSON.
     * @returns {string}
     */
    get lossLedgerJson() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.projectionpackage_lossLedgerJson(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Stable carrier profile name.
     * @returns {string}
     */
    get profile() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.projectionpackage_profile(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) ProjectionPackage.prototype[Symbol.dispose] = ProjectionPackage.prototype.free;
exports.ProjectionPackage = ProjectionPackage;

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
class ProvenanceInfo {
    static __wrap(ptr) {
        const obj = Object.create(ProvenanceInfo.prototype);
        obj.__wbg_ptr = ptr;
        ProvenanceInfoFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        ProvenanceInfoFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_provenanceinfo_free(ptr, 0);
    }
    /**
     * The decoded `engine` label, or `undefined` if absent.
     * @returns {string | undefined}
     */
    get engine() {
        const ret = wasm.provenanceinfo_engine(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * The decoded `queryHash`, or `undefined` if absent.
     * @returns {string | undefined}
     */
    get queryHash() {
        const ret = wasm.provenanceinfo_queryHash(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
}
if (Symbol.dispose) ProvenanceInfo.prototype[Symbol.dispose] = ProvenanceInfo.prototype.free;
exports.ProvenanceInfo = ProvenanceInfo;

/**
 * An RDF/JS [Quad](https://rdf.js.org/data-model-spec/#quad-interface) — a statement
 * `(subject, predicate, object, graph)` with `termType: "Quad"`.
 */
class Quad {
    static __wrap(ptr) {
        const obj = Object.create(Quad.prototype);
        obj.__wbg_ptr = ptr;
        QuadFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        QuadFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_quad_free(ptr, 0);
    }
    /**
     * A quoted-triple [`Term`] (`termType: "Quad"`) viewing this quad's `(s, p, o)` —
     * the RDF-1.2 wedge: pass the result as a subject/object to embed it.
     * @returns {Term}
     */
    asTerm() {
        const ret = wasm.quad_asTerm(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return Term.__wrap(ret[0]);
    }
    /**
     * Structural RDF/JS quad equality.
     * @param {Quad} other
     * @returns {boolean}
     */
    equals(other) {
        _assertClass(other, Quad);
        const ret = wasm.quad_equals(this.__wbg_ptr, other.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * The graph [`Term`] of the quad (`DefaultGraph` when unnamed).
     * @returns {Term}
     */
    get graph() {
        const ret = wasm.quad_graph(this.__wbg_ptr);
        return Term.__wrap(ret);
    }
    /**
     * The object [`Term`] of the quad.
     * @returns {Term}
     */
    get object() {
        const ret = wasm.quad_object(this.__wbg_ptr);
        return Term.__wrap(ret);
    }
    /**
     * The predicate [`Term`] of the quad.
     * @returns {Term}
     */
    get predicate() {
        const ret = wasm.quad_predicate(this.__wbg_ptr);
        return Term.__wrap(ret);
    }
    /**
     * The subject [`Term`] of the quad.
     * @returns {Term}
     */
    get subject() {
        const ret = wasm.quad_subject(this.__wbg_ptr);
        return Term.__wrap(ret);
    }
    /**
     * Always `"Quad"` (a Quad is itself an RDF/JS term).
     * @returns {string}
     */
    get termType() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.quad_term_type(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Empty for a Quad (per RDF/JS).
     * @returns {string}
     */
    get value() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.quad_value(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) Quad.prototype[Symbol.dispose] = Quad.prototype.free;
exports.Quad = Quad;

/**
 * A reusable SPARQL engine that keeps the native plan cache alive across calls.
 *
 * The engine is reference-counted so an asynchronous job can keep evaluating on it
 * after the JavaScript handle that started the job has been freed, and so the
 * synchronous methods can run on it while a job is suspended.
 */
class QueryEngine {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        QueryEngineFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_queryengine_free(ptr, 0);
    }
    /**
     * Run an ASK query and return the boolean result.
     * @param {Dataset} dataset
     * @param {string} sparql
     * @param {string | null} [base]
     * @param {string | null} [xpath_regex]
     * @returns {boolean}
     */
    ask(dataset, sparql, base, xpath_regex) {
        _assertClass(dataset, Dataset);
        const ptr0 = passStringToWasm0(sparql, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(base) ? 0 : passStringToWasm0(base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.queryengine_ask(this.__wbg_ptr, dataset.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] !== 0;
    }
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
     * @param {Dataset} dataset
     * @param {AsyncOperationKind} kind
     * @param {string} sparql
     * @param {AsyncJobOptions} options
     * @returns {AsyncJob}
     */
    beginAsync(dataset, kind, sparql, options) {
        _assertClass(dataset, Dataset);
        const ptr0 = passStringToWasm0(sparql, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertClass(options, AsyncJobOptions);
        const ret = wasm.queryengine_beginAsync(this.__wbg_ptr, dataset.__wbg_ptr, kind, ptr0, len0, options.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return AsyncJob.__wrap(ret[0]);
    }
    /**
     * Start a `rawWithContext` operation: a CONSTRUCT/DESCRIBE serialized under a
     * compiled JSON-LD context (and, for YAML-LD, the options' `yamlSchemaUrl`), in the
     * format the options were validated with.
     *
     * # Errors
     *
     * As [`Self::begin_async`], plus a kind other than `rawWithContext` and an invalid
     * `yamlSchemaUrl`.
     * @param {Dataset} dataset
     * @param {AsyncOperationKind} kind
     * @param {string} sparql
     * @param {AsyncJobOptions} options
     * @param {CompiledJsonLdContext} context
     * @returns {AsyncJob}
     */
    beginAsyncWithContext(dataset, kind, sparql, options, context) {
        _assertClass(dataset, Dataset);
        const ptr0 = passStringToWasm0(sparql, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertClass(options, AsyncJobOptions);
        _assertClass(context, CompiledJsonLdContext);
        const ret = wasm.queryengine_beginAsyncWithContext(this.__wbg_ptr, dataset.__wbg_ptr, kind, ptr0, len0, options.__wbg_ptr, context.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return AsyncJob.__wrap(ret[0]);
    }
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
     * @returns {string}
     */
    get blankScope() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.queryengine_blank_scope_name(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Run a CONSTRUCT query and return its result dataset.
     * @param {Dataset} dataset
     * @param {string} sparql
     * @param {string | null} [base]
     * @param {string | null} [xpath_regex]
     * @returns {Dataset}
     */
    construct(dataset, sparql, base, xpath_regex) {
        _assertClass(dataset, Dataset);
        const ptr0 = passStringToWasm0(sparql, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(base) ? 0 : passStringToWasm0(base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.queryengine_construct(this.__wbg_ptr, dataset.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return Dataset.__wrap(ret[0]);
    }
    /**
     * Run a DESCRIBE query and return its result dataset.
     * @param {Dataset} dataset
     * @param {string} sparql
     * @param {string | null} [base]
     * @param {string | null} [xpath_regex]
     * @returns {Dataset}
     */
    describe(dataset, sparql, base, xpath_regex) {
        _assertClass(dataset, Dataset);
        const ptr0 = passStringToWasm0(sparql, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(base) ? 0 : passStringToWasm0(base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.queryengine_describe(this.__wbg_ptr, dataset.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return Dataset.__wrap(ret[0]);
    }
    /**
     * The precision every `xsd:integer`/`xsd:decimal` quotient (`/` and `AVG`) this
     * engine forms is computed at, in its one text form: `"exact"`, or `"N:ROUNDING"`
     * — `N` fractional digits rounded in the named direction. The default is
     * `"18:toward-zero"`, XSD's eighteen fractional digits truncated toward zero.
     *
     * Under `"exact"` a quotient with no finite decimal expansion (`1/3`) is not
     * rounded: it is a SPARQL expression error, unbound like `1/0` and counted as
     * `err:FOAR0002` in a governed outcome's evidence, while a terminating one (`1/8`)
     * answers exactly. The policy applies to every query,
     * update, explain, governed, entailment, negotiated and serialized entry of this
     * engine, and an asynchronous job takes the policy in force when it begins.
     * `Dataset.query` and `Dataset.update` run on a fresh engine, so under the default.
     * @returns {string}
     */
    get divisionPolicy() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.queryengine_division_policy(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
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
     * @param {Dataset} dataset
     * @param {string} sparql
     * @param {string | null} [base]
     * @param {string | null} [xpath_regex]
     * @returns {string}
     */
    explainQuery(dataset, sparql, base, xpath_regex) {
        let deferred5_0;
        let deferred5_1;
        try {
            _assertClass(dataset, Dataset);
            const ptr0 = passStringToWasm0(sparql, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            var ptr1 = isLikeNone(base) ? 0 : passStringToWasm0(base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            var ptr2 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len2 = WASM_VECTOR_LEN;
            const ret = wasm.queryengine_explainQuery(this.__wbg_ptr, dataset.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
            var ptr4 = ret[0];
            var len4 = ret[1];
            if (ret[3]) {
                ptr4 = 0; len4 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred5_0 = ptr4;
            deferred5_1 = len4;
            return getStringFromWasm0(ptr4, len4);
        } finally {
            wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
        }
    }
    /**
     * Create a reusable offline SPARQL engine.
     */
    constructor() {
        const ret = wasm.queryengine_new();
        this.__wbg_ptr = ret;
        QueryEngineFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Run any SPARQL query and return a typed raw wasm result wrapper.
     * @param {Dataset} dataset
     * @param {string} sparql
     * @param {string | null} [base]
     * @param {string | null} [xpath_regex]
     * @returns {QueryResult}
     */
    query(dataset, sparql, base, xpath_regex) {
        _assertClass(dataset, Dataset);
        const ptr0 = passStringToWasm0(sparql, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(base) ? 0 : passStringToWasm0(base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.queryengine_query(this.__wbg_ptr, dataset.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return QueryResult.__wrap(ret[0]);
    }
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
     * @param {Dataset} dataset
     * @param {string} sparql
     * @param {string | null | undefined} base
     * @param {string} regime
     * @param {string | null | undefined} program
     * @param {string[]} import_iris
     * @param {string[]} import_documents
     * @param {string[]} premise_iris
     * @param {bigint | null} [max_stored_facts]
     * @param {bigint | null} [max_join_steps]
     * @param {string | null} [aggregate_namespace]
     * @param {bigint | null} [fuel]
     * @param {bigint | null} [deadline_ms]
     * @param {bigint | null} [max_answers]
     * @param {bigint | null} [max_intermediate_cells]
     * @param {bigint | null} [max_scratch_bytes]
     * @param {bigint | null} [max_remote_requests]
     * @param {CancellationToken | null} [cancel]
     * @param {boolean | null} [no_ceiling]
     * @param {string | null} [xpath_regex]
     * @returns {EntailmentQueryOutcome}
     */
    queryEntailmentGoverned(dataset, sparql, base, regime, program, import_iris, import_documents, premise_iris, max_stored_facts, max_join_steps, aggregate_namespace, fuel, deadline_ms, max_answers, max_intermediate_cells, max_scratch_bytes, max_remote_requests, cancel, no_ceiling, xpath_regex) {
        _assertClass(dataset, Dataset);
        const ptr0 = passStringToWasm0(sparql, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(base) ? 0 : passStringToWasm0(base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(regime, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        var ptr3 = isLikeNone(program) ? 0 : passStringToWasm0(program, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len3 = WASM_VECTOR_LEN;
        const ptr4 = passArrayJsValueToWasm0(import_iris, wasm.__wbindgen_malloc);
        const len4 = WASM_VECTOR_LEN;
        const ptr5 = passArrayJsValueToWasm0(import_documents, wasm.__wbindgen_malloc);
        const len5 = WASM_VECTOR_LEN;
        const ptr6 = passArrayJsValueToWasm0(premise_iris, wasm.__wbindgen_malloc);
        const len6 = WASM_VECTOR_LEN;
        var ptr7 = isLikeNone(aggregate_namespace) ? 0 : passStringToWasm0(aggregate_namespace, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len7 = WASM_VECTOR_LEN;
        let ptr8 = 0;
        if (!isLikeNone(cancel)) {
            _assertClass(cancel, CancellationToken);
            ptr8 = cancel.__destroy_into_raw();
        }
        var ptr9 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len9 = WASM_VECTOR_LEN;
        const ret = wasm.queryengine_queryEntailmentGoverned(this.__wbg_ptr, dataset.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5, ptr6, len6, !isLikeNone(max_stored_facts), isLikeNone(max_stored_facts) ? BigInt(0) : max_stored_facts, !isLikeNone(max_join_steps), isLikeNone(max_join_steps) ? BigInt(0) : max_join_steps, ptr7, len7, !isLikeNone(fuel), isLikeNone(fuel) ? BigInt(0) : fuel, !isLikeNone(deadline_ms), isLikeNone(deadline_ms) ? BigInt(0) : deadline_ms, !isLikeNone(max_answers), isLikeNone(max_answers) ? BigInt(0) : max_answers, !isLikeNone(max_intermediate_cells), isLikeNone(max_intermediate_cells) ? BigInt(0) : max_intermediate_cells, !isLikeNone(max_scratch_bytes), isLikeNone(max_scratch_bytes) ? BigInt(0) : max_scratch_bytes, !isLikeNone(max_remote_requests), isLikeNone(max_remote_requests) ? BigInt(0) : max_remote_requests, ptr8, isLikeNone(no_ceiling) ? 0xFFFFFF : no_ceiling ? 1 : 0, ptr9, len9);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return EntailmentQueryOutcome.__wrap(ret[0]);
    }
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
     * @param {Dataset} dataset
     * @param {string} sparql
     * @param {string | null} [base]
     * @param {string | null} [aggregate_namespace]
     * @param {bigint | null} [fuel]
     * @param {bigint | null} [deadline_ms]
     * @param {bigint | null} [max_answers]
     * @param {bigint | null} [max_intermediate_cells]
     * @param {bigint | null} [max_scratch_bytes]
     * @param {bigint | null} [max_remote_requests]
     * @param {CancellationToken | null} [cancel]
     * @param {boolean | null} [no_ceiling]
     * @param {string | null} [xpath_regex]
     * @returns {QueryOutcome}
     */
    queryGoverned(dataset, sparql, base, aggregate_namespace, fuel, deadline_ms, max_answers, max_intermediate_cells, max_scratch_bytes, max_remote_requests, cancel, no_ceiling, xpath_regex) {
        _assertClass(dataset, Dataset);
        const ptr0 = passStringToWasm0(sparql, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(base) ? 0 : passStringToWasm0(base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(aggregate_namespace) ? 0 : passStringToWasm0(aggregate_namespace, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        let ptr3 = 0;
        if (!isLikeNone(cancel)) {
            _assertClass(cancel, CancellationToken);
            ptr3 = cancel.__destroy_into_raw();
        }
        var ptr4 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len4 = WASM_VECTOR_LEN;
        const ret = wasm.queryengine_queryGoverned(this.__wbg_ptr, dataset.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, !isLikeNone(fuel), isLikeNone(fuel) ? BigInt(0) : fuel, !isLikeNone(deadline_ms), isLikeNone(deadline_ms) ? BigInt(0) : deadline_ms, !isLikeNone(max_answers), isLikeNone(max_answers) ? BigInt(0) : max_answers, !isLikeNone(max_intermediate_cells), isLikeNone(max_intermediate_cells) ? BigInt(0) : max_intermediate_cells, !isLikeNone(max_scratch_bytes), isLikeNone(max_scratch_bytes) ? BigInt(0) : max_scratch_bytes, !isLikeNone(max_remote_requests), isLikeNone(max_remote_requests) ? BigInt(0) : max_remote_requests, ptr3, isLikeNone(no_ceiling) ? 0xFFFFFF : no_ceiling ? 1 : 0, ptr4, len4);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return QueryOutcome.__wrap(ret[0]);
    }
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
     * @param {Dataset} dataset
     * @param {string} sparql
     * @param {string | null} [base]
     * @param {string | null} [format]
     * @param {string | null} [provenance_prefix]
     * @param {string | null} [provenance_iri]
     * @param {string | null} [xpath_regex]
     * @returns {string}
     */
    queryRaw(dataset, sparql, base, format, provenance_prefix, provenance_iri, xpath_regex) {
        let deferred8_0;
        let deferred8_1;
        try {
            _assertClass(dataset, Dataset);
            const ptr0 = passStringToWasm0(sparql, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            var ptr1 = isLikeNone(base) ? 0 : passStringToWasm0(base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            var ptr2 = isLikeNone(format) ? 0 : passStringToWasm0(format, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len2 = WASM_VECTOR_LEN;
            var ptr3 = isLikeNone(provenance_prefix) ? 0 : passStringToWasm0(provenance_prefix, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len3 = WASM_VECTOR_LEN;
            var ptr4 = isLikeNone(provenance_iri) ? 0 : passStringToWasm0(provenance_iri, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len4 = WASM_VECTOR_LEN;
            var ptr5 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len5 = WASM_VECTOR_LEN;
            const ret = wasm.queryengine_queryRaw(this.__wbg_ptr, dataset.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5);
            var ptr7 = ret[0];
            var len7 = ret[1];
            if (ret[3]) {
                ptr7 = 0; len7 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred8_0 = ptr7;
            deferred8_1 = len7;
            return getStringFromWasm0(ptr7, len7);
        } finally {
            wasm.__wbindgen_free(deferred8_0, deferred8_1, 1);
        }
    }
    /**
     * Serialize a CONSTRUCT/DESCRIBE result with configured JSON-LD/YAML-LD.
     * @param {Dataset} dataset
     * @param {string} sparql
     * @param {string | null | undefined} base
     * @param {string} format
     * @param {string} options_json
     * @param {string | null} [xpath_regex]
     * @returns {string}
     */
    queryRawConfigured(dataset, sparql, base, format, options_json, xpath_regex) {
        let deferred7_0;
        let deferred7_1;
        try {
            _assertClass(dataset, Dataset);
            const ptr0 = passStringToWasm0(sparql, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            var ptr1 = isLikeNone(base) ? 0 : passStringToWasm0(base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            const ptr2 = passStringToWasm0(format, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            const ptr3 = passStringToWasm0(options_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len3 = WASM_VECTOR_LEN;
            var ptr4 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len4 = WASM_VECTOR_LEN;
            const ret = wasm.queryengine_queryRawConfigured(this.__wbg_ptr, dataset.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4);
            var ptr6 = ret[0];
            var len6 = ret[1];
            if (ret[3]) {
                ptr6 = 0; len6 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred7_0 = ptr6;
            deferred7_1 = len6;
            return getStringFromWasm0(ptr6, len6);
        } finally {
            wasm.__wbindgen_free(deferred7_0, deferred7_1, 1);
        }
    }
    /**
     * Serialize a CONSTRUCT/DESCRIBE result with a reusable compiled context.
     * @param {Dataset} dataset
     * @param {string} sparql
     * @param {string | null | undefined} base
     * @param {string} format
     * @param {CompiledJsonLdContext} context
     * @param {string | null} [yaml_schema_url]
     * @param {string | null} [xpath_regex]
     * @returns {string}
     */
    queryRawWithContext(dataset, sparql, base, format, context, yaml_schema_url, xpath_regex) {
        let deferred7_0;
        let deferred7_1;
        try {
            _assertClass(dataset, Dataset);
            const ptr0 = passStringToWasm0(sparql, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            var ptr1 = isLikeNone(base) ? 0 : passStringToWasm0(base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            const ptr2 = passStringToWasm0(format, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            _assertClass(context, CompiledJsonLdContext);
            var ptr3 = isLikeNone(yaml_schema_url) ? 0 : passStringToWasm0(yaml_schema_url, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len3 = WASM_VECTOR_LEN;
            var ptr4 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len4 = WASM_VECTOR_LEN;
            const ret = wasm.queryengine_queryRawWithContext(this.__wbg_ptr, dataset.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, context.__wbg_ptr, ptr3, len3, ptr4, len4);
            var ptr6 = ret[0];
            var len6 = ret[1];
            if (ret[3]) {
                ptr6 = 0; len6 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred7_0 = ptr6;
            deferred7_1 = len6;
            return getStringFromWasm0(ptr6, len6);
        } finally {
            wasm.__wbindgen_free(deferred7_0, deferred7_1, 1);
        }
    }
    /**
     * Run a SELECT query and return typed rows.
     * @param {Dataset} dataset
     * @param {string} sparql
     * @param {string | null} [base]
     * @param {string | null} [xpath_regex]
     * @returns {SelectResult}
     */
    select(dataset, sparql, base, xpath_regex) {
        _assertClass(dataset, Dataset);
        const ptr0 = passStringToWasm0(sparql, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(base) ? 0 : passStringToWasm0(base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.queryengine_select(this.__wbg_ptr, dataset.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return SelectResult.__wrap(ret[0]);
    }
    /**
     * Select the same strict version-one profile used by every native host.
     *
     * # Errors
     * Malformed records or invalid configuration leave the current selection intact.
     * @param {string} profile
     */
    setGeoProfile(profile) {
        const ptr0 = passStringToWasm0(profile, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.queryengine_setGeoProfile(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Select the immutable native geographic profile for subsequent operations.
     * An already started asynchronous operation retains its original selection.
     * @param {GeoSession} session
     */
    setGeoSession(session) {
        _assertClass(session, GeoSession);
        wasm.queryengine_setGeoSession(this.__wbg_ptr, session.__wbg_ptr);
    }
    /**
     * Set [`Self::blank_scope_name`]. Throws on any value but `"keep"` or `"merge"`.
     *
     * # Errors
     *
     * A value that is neither `"keep"` nor `"merge"`; the mode is left as it was.
     * @param {string} mode
     */
    set blankScope(mode) {
        const ptr0 = passStringToWasm0(mode, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.queryengine_set_blank_scope(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set [`Self::division_policy`] from its text form: `"exact"`, `"N"` (`N`
     * fractional digits truncated toward zero) or `"N:ROUNDING"`, with `ROUNDING` one
     * of `toward-zero`, `away-from-zero`, `floor`, `ceiling`, `half-even`,
     * `half-away-from-zero`, `half-toward-zero`, `half-ceiling` or `half-floor`.
     *
     * # Errors
     *
     * Text in none of those forms; the policy is left as it was.
     * @param {string} policy
     */
    set divisionPolicy(policy) {
        const ptr0 = passStringToWasm0(policy, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.queryengine_set_division_policy(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Apply a SPARQL UPDATE atomically to the supplied dataset.
     * @param {Dataset} dataset
     * @param {string} sparql
     * @param {string | null} [base]
     * @param {string | null} [xpath_regex]
     */
    update(dataset, sparql, base, xpath_regex) {
        _assertClass(dataset, Dataset);
        const ptr0 = passStringToWasm0(sparql, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(base) ? 0 : passStringToWasm0(base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.queryengine_update(this.__wbg_ptr, dataset.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
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
     * @param {Dataset} dataset
     * @param {string} sparql
     * @param {string | null} [base]
     * @param {string | null} [aggregate_namespace]
     * @param {bigint | null} [fuel]
     * @param {bigint | null} [deadline_ms]
     * @param {bigint | null} [max_answers]
     * @param {bigint | null} [max_intermediate_cells]
     * @param {bigint | null} [max_scratch_bytes]
     * @param {bigint | null} [max_remote_requests]
     * @param {CancellationToken | null} [cancel]
     * @param {boolean | null} [no_ceiling]
     * @param {string | null} [xpath_regex]
     * @returns {UpdateOutcome}
     */
    updateGoverned(dataset, sparql, base, aggregate_namespace, fuel, deadline_ms, max_answers, max_intermediate_cells, max_scratch_bytes, max_remote_requests, cancel, no_ceiling, xpath_regex) {
        _assertClass(dataset, Dataset);
        const ptr0 = passStringToWasm0(sparql, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(base) ? 0 : passStringToWasm0(base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(aggregate_namespace) ? 0 : passStringToWasm0(aggregate_namespace, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        let ptr3 = 0;
        if (!isLikeNone(cancel)) {
            _assertClass(cancel, CancellationToken);
            ptr3 = cancel.__destroy_into_raw();
        }
        var ptr4 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len4 = WASM_VECTOR_LEN;
        const ret = wasm.queryengine_updateGoverned(this.__wbg_ptr, dataset.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, !isLikeNone(fuel), isLikeNone(fuel) ? BigInt(0) : fuel, !isLikeNone(deadline_ms), isLikeNone(deadline_ms) ? BigInt(0) : deadline_ms, !isLikeNone(max_answers), isLikeNone(max_answers) ? BigInt(0) : max_answers, !isLikeNone(max_intermediate_cells), isLikeNone(max_intermediate_cells) ? BigInt(0) : max_intermediate_cells, !isLikeNone(max_scratch_bytes), isLikeNone(max_scratch_bytes) ? BigInt(0) : max_scratch_bytes, !isLikeNone(max_remote_requests), isLikeNone(max_remote_requests) ? BigInt(0) : max_remote_requests, ptr3, isLikeNone(no_ceiling) ? 0xFFFFFF : no_ceiling ? 1 : 0, ptr4, len4);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return UpdateOutcome.__wrap(ret[0]);
    }
}
if (Symbol.dispose) QueryEngine.prototype[Symbol.dispose] = QueryEngine.prototype.free;
exports.QueryEngine = QueryEngine;

/**
 * The outcome of one governed query: a complete answer, or an exhausted budget carrying
 * the partial answers the execution actually reached.
 *
 * Exactly two shapes, and **neither is a thrown error**. Check `isComplete`, take
 * `takeResult()` when it holds, and take `takeTripped()` with `takePartial()` when it
 * does not. Every accessor that hands over a wasm-owned object moves it, so each can be
 * taken once.
 */
class QueryOutcome {
    static __wrap(ptr) {
        const obj = Object.create(QueryOutcome.prototype);
        obj.__wbg_ptr = ptr;
        QueryOutcomeFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        QueryOutcomeFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_queryoutcome_free(ptr, 0);
    }
    /**
     * Whether every governor stayed intact and this is the query's complete answer.
     * @returns {boolean}
     */
    get isComplete() {
        const ret = wasm.queryoutcome_isComplete(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Move this execution's receipt out. Present on both paths.
     * @returns {GovernorEvidence | undefined}
     */
    takeEvidence() {
        const ret = wasm.queryoutcome_takeEvidence(this.__wbg_ptr);
        return ret === 0 ? undefined : GovernorEvidence.__wrap(ret);
    }
    /**
     * Move the partial-answer certificate out, or `undefined` when the query completed.
     * @returns {PartialAnswers | undefined}
     */
    takePartial() {
        const ret = wasm.queryoutcome_takePartial(this.__wbg_ptr);
        return ret === 0 ? undefined : PartialAnswers.__wrap(ret);
    }
    /**
     * Move the **complete** result out, or `undefined` when a governor stopped the
     * execution.
     *
     * Deliberately never the partial rows: a caller that stopped reading the outcome one
     * level too early receives nothing rather than a truncated answer wearing a complete
     * answer's type. The rows a trip reached are behind `takePartial()`, with the
     * certificate that says what they bound.
     * @returns {QueryResult | undefined}
     */
    takeResult() {
        const ret = wasm.queryoutcome_takeResult(this.__wbg_ptr);
        return ret === 0 ? undefined : QueryResult.__wrap(ret);
    }
    /**
     * Move the tripped governor out, or `undefined` when the query completed.
     * @returns {TrippedGovernor | undefined}
     */
    takeTripped() {
        const ret = wasm.queryoutcome_takeTripped(this.__wbg_ptr);
        return ret === 0 ? undefined : TrippedGovernor.__wrap(ret);
    }
}
if (Symbol.dispose) QueryOutcome.prototype[Symbol.dispose] = QueryOutcome.prototype.free;
exports.QueryOutcome = QueryOutcome;

/**
 * A typed SPARQL result returned by the raw wasm binding.
 */
class QueryResult {
    static __wrap(ptr) {
        const obj = Object.create(QueryResult.prototype);
        obj.__wbg_ptr = ptr;
        QueryResultFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        QueryResultFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_queryresult_free(ptr, 0);
    }
    /**
     * The ASK boolean when `kind === "ask"`, otherwise `undefined`.
     * @returns {boolean | undefined}
     */
    get boolean() {
        const ret = wasm.queryresult_boolean(this.__wbg_ptr);
        return ret === 0xFFFFFF ? undefined : ret !== 0;
    }
    /**
     * The result discriminator: `"select"`, `"ask"`, or `"graph"`.
     * @returns {string}
     */
    get kind() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.queryresult_kind(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Move the graph dataset out of this wrapper.
     * @returns {Dataset | undefined}
     */
    takeDataset() {
        const ret = wasm.queryresult_takeDataset(this.__wbg_ptr);
        return ret === 0 ? undefined : Dataset.__wrap(ret);
    }
    /**
     * Move the SELECT result out of this wrapper.
     * @returns {SelectResult | undefined}
     */
    takeSelect() {
        const ret = wasm.queryresult_takeSelect(this.__wbg_ptr);
        return ret === 0 ? undefined : SelectResult.__wrap(ret);
    }
}
if (Symbol.dispose) QueryResult.prototype[Symbol.dispose] = QueryResult.prototype.free;
exports.QueryResult = QueryResult;

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
class Reasoner {
    static __wrap(ptr) {
        const obj = Object.create(Reasoner.prototype);
        obj.__wbg_ptr = ptr;
        ReasonerFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        ReasonerFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_reasoner_free(ptr, 0);
    }
    /**
     * The entailed subsumption hierarchy. See `entailClassify`.
     * @returns {ReasoningAnswer}
     */
    classify() {
        const ret = wasm.reasoner_classify(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ReasoningAnswer.__wrap(ret[0]);
    }
    /**
     * Is the knowledge base consistent? See `entailConsistency`.
     * @returns {ReasoningAnswer}
     */
    consistency() {
        const ret = wasm.reasoner_consistency(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ReasoningAnswer.__wrap(ret[0]);
    }
    /**
     * Does the ontology entail `axiom`? See `entailEntails`.
     * @param {string} axiom
     * @returns {ReasoningAnswer}
     */
    entails(axiom) {
        const ptr0 = passStringToWasm0(axiom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.reasoner_entails(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ReasoningAnswer.__wrap(ret[0]);
    }
    /**
     * Why `conclusion` holds under `regime`. See `entailExplainConclusion`.
     * @param {string} regime
     * @param {string} conclusion
     * @returns {ReasoningAnswer}
     */
    explainConclusion(regime, conclusion) {
        const ptr0 = passStringToWasm0(regime, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(conclusion, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.reasoner_explainConclusion(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ReasoningAnswer.__wrap(ret[0]);
    }
    /**
     * A module for `signature`. See `entailExtractModule`.
     * @param {string} signature
     * @param {string} method
     * @returns {ReasoningAnswer}
     */
    extractModule(signature, method) {
        const ptr0 = passStringToWasm0(signature, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(method, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.reasoner_extractModule(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ReasoningAnswer.__wrap(ret[0]);
    }
    /**
     * The individuals entailed to be instances of `class`. See `entailInstances`.
     * @param {string} _class
     * @returns {ReasoningAnswer}
     */
    instances(_class) {
        const ptr0 = passStringToWasm0(_class, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.reasoner_instances(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ReasoningAnswer.__wrap(ret[0]);
    }
    /**
     * A justification for `axiom`. See `entailJustify`.
     * @param {string} axiom
     * @returns {ReasoningAnswer}
     */
    justify(axiom) {
        const ptr0 = passStringToWasm0(axiom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.reasoner_justify(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ReasoningAnswer.__wrap(ret[0]);
    }
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
     * @param {string} document
     * @param {number} step_cap
     * @param {number} work_cap
     */
    constructor(document, step_cap, work_cap) {
        const ptr0 = passStringToWasm0(document, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.reasoner_new(ptr0, len0, step_cap, work_cap);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        ReasonerFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Which OWL 2 profiles the ontology is provably in. See `entailProfile`.
     *
     * Purely syntactic: never builds a knowledge base, so it answers even for an
     * ontology whose other services would throw.
     * @returns {ReasoningAnswer}
     */
    profile() {
        const ret = wasm.reasoner_profile(this.__wbg_ptr);
        return ReasoningAnswer.__wrap(ret);
    }
    /**
     * Answer `service` about `argument`, with its proof. See `entailProve`.
     *
     * Throws on a session that records nothing: open it with `Reasoner.withProofs`.
     * @param {string} service
     * @param {string} argument
     * @returns {ReasoningAnswer}
     */
    prove(service, argument) {
        const ptr0 = passStringToWasm0(service, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(argument, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.reasoner_prove(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ReasoningAnswer.__wrap(ret[0]);
    }
    /**
     * The entailed types of the named individuals. See `entailRealize`.
     * @returns {ReasoningAnswer}
     */
    realize() {
        const ret = wasm.reasoner_realize(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ReasoningAnswer.__wrap(ret[0]);
    }
    /**
     * Whether this session records proof terms — `true` only for `Reasoner.withProofs`.
     * @returns {boolean}
     */
    recordsProofs() {
        const ret = wasm.reasoner_recordsProofs(this.__wbg_ptr);
        return ret !== 0;
    }
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
     * @param {string} document
     * @param {number} step_cap
     * @param {number} work_cap
     * @returns {Reasoner}
     */
    static withProofs(document, step_cap, work_cap) {
        const ptr0 = passStringToWasm0(document, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.reasoner_withProofs(ptr0, len0, step_cap, work_cap);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return Reasoner.__wrap(ret[0]);
    }
}
if (Symbol.dispose) Reasoner.prototype[Symbol.dispose] = Reasoner.prototype.free;
exports.Reasoner = Reasoner;

/**
 * One reasoning service's answer and the certificate of the run that produced it.
 *
 * A class with two named getters rather than a positional pair, exactly as
 * [`RegimeClosure`] is and for the same reason: both strings cross the host
 * boundary and a two-string tuple is the wrong thing for a JS caller to get
 * backwards. There is no certificate-free getter — a caller that ignores the
 * evidence must at least have been handed it.
 */
class ReasoningAnswer {
    static __wrap(ptr) {
        const obj = Object.create(ReasoningAnswer.prototype);
        obj.__wbg_ptr = ptr;
        ReasoningAnswerFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        ReasoningAnswerFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_reasoninganswer_free(ptr, 0);
    }
    /**
     * The service's answer.
     *
     * Line-oriented and byte-stable. A dataset-valued answer
     * (`entailExtractModule`, `entailJustify`) is canonical (RDFC-1.0) N-Quads
     * instead.
     * @returns {string}
     */
    get answer() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.reasoninganswer_answer(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
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
     * @returns {string}
     */
    get certificate() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.reasoninganswer_certificate(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
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
     * @returns {string}
     */
    get proof() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.reasoninganswer_proof(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) ReasoningAnswer.prototype[Symbol.dispose] = ReasoningAnswer.prototype.free;
exports.ReasoningAnswer = ReasoningAnswer;

/**
 * One closure of one document under one regime: the canonical N-Quads and the
 * rendered reasoning report.
 *
 * A class with two named getters rather than a positional pair, because a
 * two-string tuple is the wrong thing for a JS caller to get backwards.
 */
class RegimeClosure {
    static __wrap(ptr) {
        const obj = Object.create(RegimeClosure.prototype);
        obj.__wbg_ptr = ptr;
        RegimeClosureFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        RegimeClosureFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_regimeclosure_free(ptr, 0);
    }
    /**
     * The materialized dataset — every input quad plus every inferred triple —
     * as canonical (RDFC-1.0) N-Quads.
     * @returns {string}
     */
    get nquads() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.regimeclosure_nquads(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * What the run actually did: which rules fired and how often, which
     * specification rules did NOT fire, which constructs were left at a
     * boundary, the evaluation budget, and the calculus's contract hash.
     * @returns {string}
     */
    get report() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.regimeclosure_report(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) RegimeClosure.prototype[Symbol.dispose] = RegimeClosure.prototype.free;
exports.RegimeClosure = RegimeClosure;

/**
 * What `purrdf_jspi_run` reports.
 * @enum {0 | 1 | 2 | 3}
 */
const RunStatus = Object.freeze({
    /**
     * The job's outcome is stored.
     */
    Outcome: 0, "0": "Outcome",
    /**
     * The job's error is stored.
     */
    Error: 1, "1": "Error",
    /**
     * No job has this id.
     */
    UnknownJob: 2, "2": "UnknownJob",
    /**
     * The job had already been started.
     */
    AlreadyStarted: 3, "3": "AlreadyStarted",
});
exports.RunStatus = RunStatus;

/**
 * A typed SELECT result returned by the raw wasm binding.
 */
class SelectResult {
    static __wrap(ptr) {
        const obj = Object.create(SelectResult.prototype);
        obj.__wbg_ptr = ptr;
        SelectResultFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        SelectResultFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_selectresult_free(ptr, 0);
    }
    /**
     * The result discriminator.
     * @returns {string}
     */
    get kind() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.selectresult_kind(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Move the next unconsumed row out of the result.
     * @returns {SelectRow | undefined}
     */
    nextRow() {
        const ret = wasm.selectresult_nextRow(this.__wbg_ptr);
        return ret === 0 ? undefined : SelectRow.__wrap(ret);
    }
    /**
     * Number of rows that have not yet been consumed.
     * @returns {number}
     */
    get remaining() {
        const ret = wasm.selectresult_remaining(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Total number of SELECT rows, including rows already consumed.
     * @returns {number}
     */
    get rowCount() {
        const ret = wasm.selectresult_row_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Move a row out by result index. Each row can be consumed once.
     * @param {number} index
     * @returns {SelectRow | undefined}
     */
    takeRow(index) {
        const ret = wasm.selectresult_takeRow(this.__wbg_ptr, index);
        return ret === 0 ? undefined : SelectRow.__wrap(ret);
    }
    /**
     * Projected variables, in SELECT projection order.
     * @returns {string[]}
     */
    get variables() {
        const ret = wasm.selectresult_variables(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
}
if (Symbol.dispose) SelectResult.prototype[Symbol.dispose] = SelectResult.prototype.free;
exports.SelectResult = SelectResult;

/**
 * One SELECT binding row.
 */
class SelectRow {
    static __wrap(ptr) {
        const obj = Object.create(SelectRow.prototype);
        obj.__wbg_ptr = ptr;
        SelectRowFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        SelectRowFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_selectrow_free(ptr, 0);
    }
    /**
     * Return the bound term for a variable name, or `undefined` for unbound/absent.
     * @param {string} variable
     * @returns {Term | undefined}
     */
    get(variable) {
        const ptr0 = passStringToWasm0(variable, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.selectrow_get(this.__wbg_ptr, ptr0, len0);
        return ret === 0 ? undefined : Term.__wrap(ret);
    }
    /**
     * Move one value out by projection index, or return `undefined` when the
     * cell is unbound, absent, or was already consumed.
     * @param {number} index
     * @returns {Term | undefined}
     */
    takeValue(index) {
        const ret = wasm.selectrow_takeValue(this.__wbg_ptr, index);
        return ret === 0 ? undefined : Term.__wrap(ret);
    }
    /**
     * Variables projected by this row, in SELECT projection order.
     * @returns {string[]}
     */
    get variables() {
        const ret = wasm.selectrow_variables(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
}
if (Symbol.dispose) SelectRow.prototype[Symbol.dispose] = SelectRow.prototype.free;
exports.SelectRow = SelectRow;

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
class SerializeLoss {
    static __wrap(ptr) {
        const obj = Object.create(SerializeLoss.prototype);
        obj.__wbg_ptr = ptr;
        SerializeLossFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        SerializeLossFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_serializeloss_free(ptr, 0);
    }
    /**
     * Object literals whose RDF-1.2 base direction the target has no surface for
     * (TriX / HexTuples keep the language tag but cannot carry the direction). `0`
     * for every direction-capable format.
     * @returns {number}
     */
    get directionalLiteralsDropped() {
        const ret = wasm.serializeloss_directionalLiteralsDropped(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Declared named graphs holding no row that the target has no spelling for
     * (N-Quads, HexTuples and every single-graph syntax), so the document omits them.
     * They own no row, so no other count sees them. `0` for TriG, TriX, JSON-LD and
     * YAML-LD, which write an empty graph, and for a dataset that declares none.
     * @returns {number}
     */
    get emptyNamedGraphsDropped() {
        const ret = wasm.serializeloss_emptyNamedGraphsDropped(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Rows the single-graph flattening dropped because the target has no named-graph
     * construct: base quads asserted in a named graph plus the statement-layer rows
     * scoped to one. `0` for every dataset-capable format. The rows are DROPPED, not
     * folded into the default graph.
     * @returns {number}
     */
    get namedGraphRowsDropped() {
        const ret = wasm.serializeloss_namedGraphRowsDropped(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * RDF-1.2 statement-layer rows (reifier bindings + annotation triples) dropped
     * because the target cannot represent quoted triples. `0` for star-capable
     * formats. Rows dropped because they were scoped to a named graph are counted by
     * [`named_graph_rows_dropped`](Self::named_graph_rows_dropped) instead, never
     * here and never twice.
     * @returns {number}
     */
    get statementRowsDropped() {
        const ret = wasm.serializeloss_statementRowsDropped(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * The serialized document. Identical to what `serialize(format)` returns for a
     * star-capable target; for RDF/XML, TriX and HexTuples it is the contract's
     * projected document instead — the statement layer reduced to base quads and
     * charged to [`statement_rows_dropped`](Self::statement_rows_dropped).
     * @returns {string}
     */
    get text() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.serializeloss_text(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) SerializeLoss.prototype[Symbol.dispose] = SerializeLoss.prototype.free;
exports.SerializeLoss = SerializeLoss;

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
class ServiceCatalog {
    static __wrap(ptr) {
        const obj = Object.create(ServiceCatalog.prototype);
        obj.__wbg_ptr = ptr;
        ServiceCatalogFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    static __unwrap(jsValue) {
        if (!(jsValue instanceof ServiceCatalog)) {
            return 0;
        }
        return jsValue.__destroy_into_raw();
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        ServiceCatalogFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_servicecatalog_free(ptr, 0);
    }
    /**
     * Register the profile `profileJson` for the service IRI `endpoint`, replacing any
     * earlier one.
     * @param {string} endpoint
     * @param {string} profile_json
     */
    addService(endpoint, profile_json) {
        const ptr0 = passStringToWasm0(endpoint, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(profile_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.servicecatalog_addService(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * An independent copy of this catalog: later changes to either leave the other as
     * it is. An asynchronous operation's options take a copy of the catalog they name,
     * so the caller's own stays usable for later operations.
     * @returns {ServiceCatalog}
     */
    copy() {
        const ret = wasm.servicecatalog_copy(this.__wbg_ptr);
        return ServiceCatalog.__wrap(ret);
    }
    /**
     * An empty catalog: every service is denied.
     */
    constructor() {
        const ret = wasm.servicecatalog_new();
        this.__wbg_ptr = ret;
        ServiceCatalogFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Apply the profile `profileJson` to every service with no entry of its own — the
     * explicit opt-out of deny-by-default.
     * @param {string} profile_json
     */
    setFallback(profile_json) {
        const ptr0 = passStringToWasm0(profile_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.servicecatalog_setFallback(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
}
if (Symbol.dispose) ServiceCatalog.prototype[Symbol.dispose] = ServiceCatalog.prototype.free;
exports.ServiceCatalog = ServiceCatalog;

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
class ShaclChangeValidation {
    static __wrap(ptr) {
        const obj = Object.create(ShaclChangeValidation.prototype);
        obj.__wbg_ptr = ptr;
        ShaclChangeValidationFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        ShaclChangeValidationFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_shaclchangevalidation_free(ptr, 0);
    }
    /**
     * Whether the change's footprint could be bounded.
     * @returns {boolean}
     */
    get bounded() {
        const ret = wasm.shaclchangevalidation_bounded(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * How many focus nodes the change was expanded into, or `undefined` when the
     * footprint could not be bounded and the whole graph was validated.
     *
     * `undefined` rather than the graph's node count on the fallback path: "every
     * focus node in the graph" and a number are different statements, and
     * collapsing them would make a fallback indistinguishable from a large
     * bounded expansion.
     * @returns {number | undefined}
     */
    get focusNodes() {
        const ret = wasm.shaclchangevalidation_focus_nodes(this.__wbg_ptr);
        return ret === Number.MAX_SAFE_INTEGER ? undefined : ret;
    }
    /**
     * Which construct made this shapes graph's change footprint unbounded, or
     * `undefined` when it was bounded.
     *
     * Actionable rather than decorative: it names what to change to get
     * incremental validation back.
     * @returns {string | undefined}
     */
    get reason() {
        const ret = wasm.shaclchangevalidation_reason(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * The SARIF 2.1.0 JSON log. See `bounded` for what it describes.
     * @returns {string}
     */
    get sarif() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.shaclchangevalidation_sarif(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) ShaclChangeValidation.prototype[Symbol.dispose] = ShaclChangeValidation.prototype.free;
exports.ShaclChangeValidation = ShaclChangeValidation;

/**
 * One mandatory diagnostic of a shapes graph — a shape whose `sh:in` or `sh:xone` list is
 * empty — as the structured value every host carries: the rule, then the shape.
 *
 * Like every other wasm-bindgen class in this package, this owns wasm memory and is
 * released with `.free()`.
 */
class ShaclDiagnostic {
    static __wrap(ptr) {
        const obj = Object.create(ShaclDiagnostic.prototype);
        obj.__wbg_ptr = ptr;
        ShaclDiagnosticFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        ShaclDiagnosticFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_shacldiagnostic_free(ptr, 0);
    }
    /**
     * The syntax rule's id: `in-minListLength` or `xone-minListLength` (SHACL 1.2 Core,
     * Appendix A).
     * @returns {string}
     */
    get rule() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.shacldiagnostic_rule(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * The shape whose list is empty, as an N-Triples term (`<iri>` or `_:label`).
     * @returns {string}
     */
    get shape() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.shacldiagnostic_shape(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) ShaclDiagnostic.prototype[Symbol.dispose] = ShaclDiagnostic.prototype.free;
exports.ShaclDiagnostic = ShaclDiagnostic;

/**
 * The outcome of `shaclEntail`: the materialized dataset and the shapes graph's
 * mandatory diagnostics.
 *
 * Like every other wasm-bindgen class in this package, this owns wasm memory and is
 * released with `.free()`.
 */
class ShaclEntailment {
    static __wrap(ptr) {
        const obj = Object.create(ShaclEntailment.prototype);
        obj.__wbg_ptr = ptr;
        ShaclEntailmentFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        ShaclEntailmentFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_shaclentailment_free(ptr, 0);
    }
    /**
     * The shapes graph's mandatory diagnostics, one `{ rule, shape }` per shape with an
     * empty `sh:in` or `sh:xone` list, which every run reports.
     * @returns {ShaclDiagnostic[]}
     */
    get diagnostics() {
        const ret = wasm.shaclentailment_diagnostics(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * The MATERIALIZED dataset — the base graph plus every inferred triple — as canonical
     * N-Triples.
     * @returns {string}
     */
    get ntriples() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.shaclentailment_ntriples(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) ShaclEntailment.prototype[Symbol.dispose] = ShaclEntailment.prototype.free;
exports.ShaclEntailment = ShaclEntailment;

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
class ShaclImportError {
    static __wrap(ptr) {
        const obj = Object.create(ShaclImportError.prototype);
        obj.__wbg_ptr = ptr;
        ShaclImportErrorFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        ShaclImportErrorFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_shaclimporterror_free(ptr, 0);
    }
    /**
     * The IRIs the refusal names.
     * @returns {string[]}
     */
    get iris() {
        const ret = wasm.shaclimporterror_iris(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * `unresolved-import`, `unreached-import`, `incompatible-import-versions`,
     * `invalid-import`, `unresolved-shapes-graph-link`, `unheld-shapes-graph-link` or
     * `invalid-shapes-graph-link`.
     * @returns {string}
     */
    get kind() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.shaclimporterror_kind(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * The engine's explanation, naming the remedy.
     * @returns {string}
     */
    get message() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.shaclimporterror_message(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * The engine's own rendering, which leads with the kind label.
     * @returns {string}
     */
    toString() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.shaclimporterror_toString(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) ShaclImportError.prototype[Symbol.dispose] = ShaclImportError.prototype.free;
exports.ShaclImportError = ShaclImportError;

/**
 * A SHACL request an asynchronous twin hands `AsyncJob.beginShacl`: built by the
 * constructor named for its synchronous entry, over exactly that entry's
 * positional arguments, and consumed by the job that runs it.
 */
class ShaclJobRequest {
    static __wrap(ptr) {
        const obj = Object.create(ShaclJobRequest.prototype);
        obj.__wbg_ptr = ptr;
        ShaclJobRequestFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        ShaclJobRequestFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_shacljobrequest_free(ptr, 0);
    }
    /**
     * @param {string} data_nt
     * @param {string | null} [shapes_ttl]
     * @param {string | null} [srl]
     * @param {string | null} [shapes_base]
     * @param {string | null} [srl_base]
     * @param {boolean | null} [explain]
     * @param {string[] | null} [import_iris]
     * @param {string[] | null} [import_documents]
     * @param {string | null} [shapes_graph]
     * @param {bigint | null} [max_term_generating_rounds]
     * @param {bigint | null} [max_generated_terms]
     * @param {bigint | null} [max_stored_facts]
     * @param {bigint | null} [max_join_steps]
     * @returns {ShaclJobRequest}
     */
    static applyRules(data_nt, shapes_ttl, srl, shapes_base, srl_base, explain, import_iris, import_documents, shapes_graph, max_term_generating_rounds, max_generated_terms, max_stored_facts, max_join_steps) {
        const ptr0 = passStringToWasm0(data_nt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(shapes_ttl) ? 0 : passStringToWasm0(shapes_ttl, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(srl) ? 0 : passStringToWasm0(srl, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        var ptr3 = isLikeNone(shapes_base) ? 0 : passStringToWasm0(shapes_base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len3 = WASM_VECTOR_LEN;
        var ptr4 = isLikeNone(srl_base) ? 0 : passStringToWasm0(srl_base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len4 = WASM_VECTOR_LEN;
        var ptr5 = isLikeNone(import_iris) ? 0 : passArrayJsValueToWasm0(import_iris, wasm.__wbindgen_malloc);
        var len5 = WASM_VECTOR_LEN;
        var ptr6 = isLikeNone(import_documents) ? 0 : passArrayJsValueToWasm0(import_documents, wasm.__wbindgen_malloc);
        var len6 = WASM_VECTOR_LEN;
        var ptr7 = isLikeNone(shapes_graph) ? 0 : passStringToWasm0(shapes_graph, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len7 = WASM_VECTOR_LEN;
        const ret = wasm.shacljobrequest_applyRules(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, isLikeNone(explain) ? 0xFFFFFF : explain ? 1 : 0, ptr5, len5, ptr6, len6, ptr7, len7, !isLikeNone(max_term_generating_rounds), isLikeNone(max_term_generating_rounds) ? BigInt(0) : max_term_generating_rounds, !isLikeNone(max_generated_terms), isLikeNone(max_generated_terms) ? BigInt(0) : max_generated_terms, !isLikeNone(max_stored_facts), isLikeNone(max_stored_facts) ? BigInt(0) : max_stored_facts, !isLikeNone(max_join_steps), isLikeNone(max_join_steps) ? BigInt(0) : max_join_steps);
        return ShaclJobRequest.__wrap(ret);
    }
    /**
     * @param {string} shapes_ttl
     * @param {string} data_nt
     * @param {string | null} [shapes_base]
     * @param {string[] | null} [import_iris]
     * @param {string[] | null} [import_documents]
     * @param {string | null} [shapes_graph]
     * @param {bigint | null} [max_term_generating_rounds]
     * @param {bigint | null} [max_generated_terms]
     * @param {bigint | null} [max_stored_facts]
     * @param {bigint | null} [max_join_steps]
     * @returns {ShaclJobRequest}
     */
    static entail(shapes_ttl, data_nt, shapes_base, import_iris, import_documents, shapes_graph, max_term_generating_rounds, max_generated_terms, max_stored_facts, max_join_steps) {
        const ptr0 = passStringToWasm0(shapes_ttl, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(data_nt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(shapes_base) ? 0 : passStringToWasm0(shapes_base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        var ptr3 = isLikeNone(import_iris) ? 0 : passArrayJsValueToWasm0(import_iris, wasm.__wbindgen_malloc);
        var len3 = WASM_VECTOR_LEN;
        var ptr4 = isLikeNone(import_documents) ? 0 : passArrayJsValueToWasm0(import_documents, wasm.__wbindgen_malloc);
        var len4 = WASM_VECTOR_LEN;
        var ptr5 = isLikeNone(shapes_graph) ? 0 : passStringToWasm0(shapes_graph, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len5 = WASM_VECTOR_LEN;
        const ret = wasm.shacljobrequest_entail(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5, !isLikeNone(max_term_generating_rounds), isLikeNone(max_term_generating_rounds) ? BigInt(0) : max_term_generating_rounds, !isLikeNone(max_generated_terms), isLikeNone(max_generated_terms) ? BigInt(0) : max_generated_terms, !isLikeNone(max_stored_facts), isLikeNone(max_stored_facts) ? BigInt(0) : max_stored_facts, !isLikeNone(max_join_steps), isLikeNone(max_join_steps) ? BigInt(0) : max_join_steps);
        return ShaclJobRequest.__wrap(ret);
    }
    /**
     * @param {string} shapes_ttl
     * @param {string} data_nt
     * @param {string | null | undefined} expr
     * @param {string} focus
     * @param {string[] | null} [scope]
     * @param {string | null} [shapes_base]
     * @param {string[] | null} [import_iris]
     * @param {string[] | null} [import_documents]
     * @param {string | null} [expr_at]
     * @param {string[] | null} [expr_via]
     * @param {string | null} [expr_turtle]
     * @returns {ShaclJobRequest}
     */
    static evalNodeExpr(shapes_ttl, data_nt, expr, focus, scope, shapes_base, import_iris, import_documents, expr_at, expr_via, expr_turtle) {
        const ptr0 = passStringToWasm0(shapes_ttl, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(data_nt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(expr) ? 0 : passStringToWasm0(expr, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(focus, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        var ptr4 = isLikeNone(scope) ? 0 : passArrayJsValueToWasm0(scope, wasm.__wbindgen_malloc);
        var len4 = WASM_VECTOR_LEN;
        var ptr5 = isLikeNone(shapes_base) ? 0 : passStringToWasm0(shapes_base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len5 = WASM_VECTOR_LEN;
        var ptr6 = isLikeNone(import_iris) ? 0 : passArrayJsValueToWasm0(import_iris, wasm.__wbindgen_malloc);
        var len6 = WASM_VECTOR_LEN;
        var ptr7 = isLikeNone(import_documents) ? 0 : passArrayJsValueToWasm0(import_documents, wasm.__wbindgen_malloc);
        var len7 = WASM_VECTOR_LEN;
        var ptr8 = isLikeNone(expr_at) ? 0 : passStringToWasm0(expr_at, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len8 = WASM_VECTOR_LEN;
        var ptr9 = isLikeNone(expr_via) ? 0 : passArrayJsValueToWasm0(expr_via, wasm.__wbindgen_malloc);
        var len9 = WASM_VECTOR_LEN;
        var ptr10 = isLikeNone(expr_turtle) ? 0 : passStringToWasm0(expr_turtle, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len10 = WASM_VECTOR_LEN;
        const ret = wasm.shacljobrequest_evalNodeExpr(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5, ptr6, len6, ptr7, len7, ptr8, len8, ptr9, len9, ptr10, len10);
        return ShaclJobRequest.__wrap(ret);
    }
    /**
     * @param {Uint8Array} product
     * @param {string} data_nt
     * @returns {ShaclJobRequest}
     */
    static productValidateToSarif(product, data_nt) {
        const ptr0 = passArray8ToWasm0(product, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(data_nt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.shacljobrequest_productValidateToSarif(ptr0, len0, ptr1, len1);
        return ShaclJobRequest.__wrap(ret);
    }
    /**
     * @param {Uint8Array} product
     * @param {string} data_nt
     * @param {string} expect_identity
     * @returns {ShaclJobRequest}
     */
    static productValidateToSarifExpecting(product, data_nt, expect_identity) {
        const ptr0 = passArray8ToWasm0(product, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(data_nt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(expect_identity, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.shacljobrequest_productValidateToSarifExpecting(ptr0, len0, ptr1, len1, ptr2, len2);
        return ShaclJobRequest.__wrap(ret);
    }
    /**
     * @param {Uint8Array} product
     * @param {string} data_nt
     * @returns {ShaclJobRequest}
     */
    static productValidateToSarifRebuild(product, data_nt) {
        const ptr0 = passArray8ToWasm0(product, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(data_nt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.shacljobrequest_productValidateToSarifRebuild(ptr0, len0, ptr1, len1);
        return ShaclJobRequest.__wrap(ret);
    }
    /**
     * @param {Uint8Array} product
     * @param {string} data_nt
     * @param {string} expect_identity
     * @returns {ShaclJobRequest}
     */
    static productValidateToSarifRebuildExpecting(product, data_nt, expect_identity) {
        const ptr0 = passArray8ToWasm0(product, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(data_nt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(expect_identity, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.shacljobrequest_productValidateToSarifRebuildExpecting(ptr0, len0, ptr1, len1, ptr2, len2);
        return ShaclJobRequest.__wrap(ret);
    }
    /**
     * @param {string} shapes_ttl
     * @param {string} data_nt
     * @param {string | null} [added_nt]
     * @param {string | null} [removed_nt]
     * @param {string | null} [shapes_base]
     * @param {string[] | null} [import_iris]
     * @param {string[] | null} [import_documents]
     * @param {string | null} [shapes_graph]
     * @returns {ShaclJobRequest}
     */
    static validateChangesToSarif(shapes_ttl, data_nt, added_nt, removed_nt, shapes_base, import_iris, import_documents, shapes_graph) {
        const ptr0 = passStringToWasm0(shapes_ttl, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(data_nt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(added_nt) ? 0 : passStringToWasm0(added_nt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        var ptr3 = isLikeNone(removed_nt) ? 0 : passStringToWasm0(removed_nt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len3 = WASM_VECTOR_LEN;
        var ptr4 = isLikeNone(shapes_base) ? 0 : passStringToWasm0(shapes_base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len4 = WASM_VECTOR_LEN;
        var ptr5 = isLikeNone(import_iris) ? 0 : passArrayJsValueToWasm0(import_iris, wasm.__wbindgen_malloc);
        var len5 = WASM_VECTOR_LEN;
        var ptr6 = isLikeNone(import_documents) ? 0 : passArrayJsValueToWasm0(import_documents, wasm.__wbindgen_malloc);
        var len6 = WASM_VECTOR_LEN;
        var ptr7 = isLikeNone(shapes_graph) ? 0 : passStringToWasm0(shapes_graph, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len7 = WASM_VECTOR_LEN;
        const ret = wasm.shacljobrequest_validateChangesToSarif(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5, ptr6, len6, ptr7, len7);
        return ShaclJobRequest.__wrap(ret);
    }
    /**
     * @param {string} shapes_ttl
     * @param {string} data_nt
     * @param {string | null} [shapes_base]
     * @param {string[] | null} [conformance_disallows]
     * @param {string[] | null} [import_iris]
     * @param {string[] | null} [import_documents]
     * @param {string | null} [shapes_graph]
     * @param {boolean | null} [subclass_of_in_shapes_graph]
     * @returns {ShaclJobRequest}
     */
    static validateToSarif(shapes_ttl, data_nt, shapes_base, conformance_disallows, import_iris, import_documents, shapes_graph, subclass_of_in_shapes_graph) {
        const ptr0 = passStringToWasm0(shapes_ttl, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(data_nt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(shapes_base) ? 0 : passStringToWasm0(shapes_base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        var ptr3 = isLikeNone(conformance_disallows) ? 0 : passArrayJsValueToWasm0(conformance_disallows, wasm.__wbindgen_malloc);
        var len3 = WASM_VECTOR_LEN;
        var ptr4 = isLikeNone(import_iris) ? 0 : passArrayJsValueToWasm0(import_iris, wasm.__wbindgen_malloc);
        var len4 = WASM_VECTOR_LEN;
        var ptr5 = isLikeNone(import_documents) ? 0 : passArrayJsValueToWasm0(import_documents, wasm.__wbindgen_malloc);
        var len5 = WASM_VECTOR_LEN;
        var ptr6 = isLikeNone(shapes_graph) ? 0 : passStringToWasm0(shapes_graph, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len6 = WASM_VECTOR_LEN;
        const ret = wasm.shacljobrequest_validateToSarif(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5, ptr6, len6, isLikeNone(subclass_of_in_shapes_graph) ? 0xFFFFFF : subclass_of_in_shapes_graph ? 1 : 0);
        return ShaclJobRequest.__wrap(ret);
    }
}
if (Symbol.dispose) ShaclJobRequest.prototype[Symbol.dispose] = ShaclJobRequest.prototype.free;
exports.ShaclJobRequest = ShaclJobRequest;

/**
 * The outcome of `shaclLintShapes`: the cold-certify report of a shapes graph.
 *
 * Like every other wasm-bindgen class in this package, this owns wasm memory and is
 * released with `.free()`.
 */
class ShaclLintReport {
    static __wrap(ptr) {
        const obj = Object.create(ShaclLintReport.prototype);
        obj.__wbg_ptr = ptr;
        ShaclLintReportFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        ShaclLintReportFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_shacllintreport_free(ptr, 0);
    }
    /**
     * Whether the report carries no finding: the loader accepted the graph, every
     * `shacl-shacl.ttl` result is superseded (flagged there, well-formed SHACL 1.2 Core),
     * no unexecuted query violates a pre-binding restriction and no mandatory diagnostic
     * applies.
     * @returns {boolean}
     */
    get clean() {
        const ret = wasm.shacllintreport_clean(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * The finding count: one for a load refusal, plus every `shacl-shacl.ttl` result no
     * supersession covers, plus every unexecuted query that violates a pre-binding
     * restriction, plus every mandatory diagnostic (an empty `sh:in` or `sh:xone` list).
     * @returns {number}
     */
    get findings() {
        const ret = wasm.shacllintreport_findings(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * The loader's refusal, or `undefined` when it accepted the graph.
     * @returns {string | undefined}
     */
    get loadError() {
        const ret = wasm.shacllintreport_load_error(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
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
     * @returns {string}
     */
    get report() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.shacllintreport_report(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) ShaclLintReport.prototype[Symbol.dispose] = ShaclLintReport.prototype.free;
exports.ShaclLintReport = ShaclLintReport;

/**
 * The outcome of `shaclEvalNodeExpr`: the expression's output nodes and the shapes graph's
 * mandatory diagnostics.
 *
 * Like every other wasm-bindgen class in this package, this owns wasm memory and is
 * released with `.free()`.
 */
class ShaclNodeExprOutcome {
    static __wrap(ptr) {
        const obj = Object.create(ShaclNodeExprOutcome.prototype);
        obj.__wbg_ptr = ptr;
        ShaclNodeExprOutcomeFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        ShaclNodeExprOutcomeFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_shaclnodeexproutcome_free(ptr, 0);
    }
    /**
     * The shapes graph's mandatory diagnostics — one `{ rule, shape }` per shape of its
     * `owl:imports` closure with an empty `sh:in` or `sh:xone` list — which every run
     * reports. They change no output.
     * @returns {ShaclDiagnostic[]}
     */
    get diagnostics() {
        const ret = wasm.shaclnodeexproutcome_diagnostics(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * The output nodes, as N-Triples 1.2 terms in the order the expression's sequence
     * semantics define.
     * @returns {string[]}
     */
    get outputs() {
        const ret = wasm.shaclnodeexproutcome_outputs(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
}
if (Symbol.dispose) ShaclNodeExprOutcome.prototype[Symbol.dispose] = ShaclNodeExprOutcome.prototype.free;
exports.ShaclNodeExprOutcome = ShaclNodeExprOutcome;

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
class ShaclProductRefusal {
    static __wrap(ptr) {
        const obj = Object.create(ShaclProductRefusal.prototype);
        obj.__wbg_ptr = ptr;
        ShaclProductRefusalFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        ShaclProductRefusalFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_shaclproductrefusal_free(ptr, 0);
    }
    /**
     * A native XPath resource refusal's own code (`xpath-pattern-bytes`, ...), or
     * `undefined`.
     * @returns {string | undefined}
     */
    get code() {
        const ret = wasm.shaclproductrefusal_code(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * The admission dimension that refused, or `undefined`.
     * @returns {string | undefined}
     */
    get dimension() {
        const ret = wasm.shaclproductrefusal_dimension(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * The prescriptive explanation, without the dimension label.
     * @returns {string}
     */
    get message() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.shaclproductrefusal_message(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * `<dimension>: <message>`, or the message alone when there is no dimension —
     * the codec's own rendering, so a host that only logs still sees the label.
     * @returns {string}
     */
    toString() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.shaclproductrefusal_toString(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) ShaclProductRefusal.prototype[Symbol.dispose] = ShaclProductRefusal.prototype.free;
exports.ShaclProductRefusal = ShaclProductRefusal;

/**
 * The outcome of `shaclCheckRules`: a SPARQL 1.2 RL rule set that passed every check
 * its level asks for.
 *
 * Like every other wasm-bindgen class in this package, this owns wasm memory and is
 * released with `.free()`.
 */
class ShaclRulesCheck {
    static __wrap(ptr) {
        const obj = Object.create(ShaclRulesCheck.prototype);
        obj.__wbg_ptr = ptr;
        ShaclRulesCheckFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        ShaclRulesCheckFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_shaclrulescheck_free(ptr, 0);
    }
    /**
     * The number of data-block triples of the combined rule set.
     * @returns {number}
     */
    get dataTriples() {
        const ret = wasm.shaclrulescheck_data_triples(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * The IRIs of the imported rule sets, each once, in the order they were read.
     * @returns {string[]}
     */
    get imported() {
        const ret = wasm.shaclrulescheck_imported(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * The level the rule set passed: `"syntax"`, `"well-formed"` or `"stratified"`.
     * @returns {string}
     */
    get level() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.shaclrulescheck_level(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Every rule of the combined rule set — the rule set's own, then every imported
     * rule — described by its IRI or by where it is written.
     * @returns {string[]}
     */
    get rules() {
        const ret = wasm.shaclrulescheck_rules(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * The number of stratification layers, or `undefined` below `"stratified"`, which
     * does not stratify.
     * @returns {number | undefined}
     */
    get strata() {
        const ret = wasm.shaclrulescheck_strata(this.__wbg_ptr);
        return ret === Number.MAX_SAFE_INTEGER ? undefined : ret;
    }
    /**
     * The one-line summary every PurRDF host reports.
     * @returns {string}
     */
    get summary() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.shaclrulescheck_summary(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * The `VERSION` labels, in document order.
     * @returns {string[]}
     */
    get versions() {
        const ret = wasm.shaclrulescheck_versions(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
}
if (Symbol.dispose) ShaclRulesCheck.prototype[Symbol.dispose] = ShaclRulesCheck.prototype.free;
exports.ShaclRulesCheck = ShaclRulesCheck;

/**
 * The outcome of `shaclApplyRules`: the inference graph, and its proof when one was
 * asked for.
 *
 * Like every other wasm-bindgen class in this package, this owns wasm memory and is
 * released with `.free()`.
 */
class ShaclRulesInference {
    static __wrap(ptr) {
        const obj = Object.create(ShaclRulesInference.prototype);
        obj.__wbg_ptr = ptr;
        ShaclRulesInferenceFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        ShaclRulesInferenceFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_shaclrulesinference_free(ptr, 0);
    }
    /**
     * The shapes graph's mandatory diagnostics, one `{ rule, shape }` per shape with an
     * empty `sh:in` or `sh:xone` list, which every run reports; empty for an `srl` rule
     * set, which has no shapes graph.
     * @returns {ShaclDiagnostic[]}
     */
    get diagnostics() {
        const ret = wasm.shaclrulesinference_diagnostics(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * The INFERENCE GRAPH — the inferred triples only, never the data graph — as
     * N-Triples 1.2, one triple per line, in canonical order.
     * @returns {string}
     */
    get inferred() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.shaclrulesinference_inferred(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * The proof of every inferred triple, or `undefined` when `explain` was not set:
     * `derived S P O .`, then `  rule R` and one `  premise S P O .` per fact the rule's
     * body matched, or `  data-block` for a SPARQL 1.2 RL data-block triple.
     * @returns {string | undefined}
     */
    get proof() {
        const ret = wasm.shaclrulesinference_proof(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
}
if (Symbol.dispose) ShaclRulesInference.prototype[Symbol.dispose] = ShaclRulesInference.prototype.free;
exports.ShaclRulesInference = ShaclRulesInference;

/**
 * One invocation a `SILENT` clause absorbed: a `SERVICE SILENT` that answered the single
 * empty solution, or a `LOAD SILENT` that succeeded with nothing loaded.
 */
class SilencedInvocation {
    static __wrap(ptr) {
        const obj = Object.create(SilencedInvocation.prototype);
        obj.__wbg_ptr = ptr;
        SilencedInvocationFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        SilencedInvocationFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_silencedinvocation_free(ptr, 0);
    }
    /**
     * The `SERVICE` endpoint, or `undefined` for a `LOAD`. A variable endpoint bound to
     * a term that is not an IRI reports that term in N-Triples form.
     * @returns {string | undefined}
     */
    get endpoint() {
        const ret = wasm.silencedinvocation_endpoint(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * The `LOAD` source IRI, or `undefined` for a `SERVICE`.
     * @returns {string | undefined}
     */
    get iri() {
        const ret = wasm.silencedinvocation_iri(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * Why the invocation failed: `"transport"`, `"decode"`, `"disabled"`,
     * `"unconfigured"`, `"denied"`, `"host-denied"`, `"not-an-iri"` or `"fault"`.
     * @returns {string}
     */
    get kind() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.silencedinvocation_kind(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * The failure's message, as the error would have read without `SILENT`.
     * @returns {string}
     */
    get message() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.silencedinvocation_message(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * `"service"` or `"load"`.
     * @returns {string}
     */
    get target() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.silencedinvocation_target(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) SilencedInvocation.prototype[Symbol.dispose] = SilencedInvocation.prototype.free;
exports.SilencedInvocation = SilencedInvocation;

/**
 * An RDF/JS `Sink` — a streaming consumer that interns pushed quads through the
 * `purrdf-events` protocol and freezes them at `finish()`.
 */
class Sink {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        SinkFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_sink_free(ptr, 0);
    }
    /**
     * `finish()` — run the protocol's forward-reference resolution and return the
     * resulting dataset. The sink is consumed; further `push`/`finish` is an error.
     * @returns {Dataset}
     */
    finish() {
        const ret = wasm.sink_finish(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return Dataset.__wrap(ret[0]);
    }
    /**
     * `new Sink()` — an empty sink ready to accept quads via `push`.
     */
    constructor() {
        const ret = wasm.sink_new();
        this.__wbg_ptr = ret;
        SinkFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * `push(quad)` — stream one quad into the sink (interned via the event protocol).
     * @param {Quad} quad
     */
    push(quad) {
        _assertClass(quad, Quad);
        const ret = wasm.sink_push(this.__wbg_ptr, quad.__wbg_ptr);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
}
if (Symbol.dispose) Sink.prototype[Symbol.dispose] = Sink.prototype.free;
exports.Sink = Sink;

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
class SparqlProtocolRequest {
    static __wrap(ptr) {
        const obj = Object.create(SparqlProtocolRequest.prototype);
        obj.__wbg_ptr = ptr;
        SparqlProtocolRequestFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        SparqlProtocolRequestFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_sparqlprotocolrequest_free(ptr, 0);
    }
    /**
     * Every `default-graph-uri`, in request order.
     * @returns {string[]}
     */
    get defaultGraphUris() {
        const ret = wasm.sparqlprotocolrequest_defaultGraphUris(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * The operation text to evaluate: the request's dataset parameters applied (the
     * query's own `FROM`/`FROM NAMED` replaced; `USING`/`USING NAMED` written into each
     * update operation with a `WHERE`), and the whole text parsed under the engine's own
     * reading, so a malformed operation is refused here as the client error it is.
     *
     * # Errors
     *
     * The typed protocol refusal (`MalformedOperation`, `UsingConflictsWithClause`).
     * @returns {string}
     */
    effectiveText() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.sparqlprotocolrequest_effectiveText(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Every parameter the protocol does not define, as flattened `[name, value, name,
     * value, …]` pairs in request order (query string first, then a form body).
     * @returns {string[]}
     */
    get extraParameters() {
        const ret = wasm.sparqlprotocolrequest_extraParameters(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * The media type of a format token `negotiate` returns, or `undefined` for any other
     * string.
     * @param {string} token
     * @returns {string | undefined}
     */
    static formatMediaType(token) {
        const ptr0 = passStringToWasm0(token, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.sparqlprotocolrequest_formatMediaType(ptr0, len0);
        let v2;
        if (ret[0] !== 0) {
            v2 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v2;
    }
    /**
     * Whether the request carries dataset parameters.
     * @returns {boolean}
     */
    get hasDatasetParameters() {
        const ret = wasm.sparqlprotocolrequest_hasDatasetParameters(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * `"query"` or `"update"`.
     * @returns {string}
     */
    get kind() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.sparqlprotocolrequest_kind(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Every `named-graph-uri`, in request order.
     * @returns {string[]}
     */
    get namedGraphUris() {
        const ret = wasm.sparqlprotocolrequest_namedGraphUris(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
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
     * @param {string | null} [accept]
     * @returns {string | undefined}
     */
    negotiate(accept) {
        var ptr0 = isLikeNone(accept) ? 0 : passStringToWasm0(accept, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        const ret = wasm.sparqlprotocolrequest_negotiate(this.__wbg_ptr, ptr0, len0);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        let v2;
        if (ret[0] !== 0) {
            v2 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v2;
    }
    /**
     * Why a result of `kind` cannot be sent to a client whose `Accept` header allows none
     * of the formats that carry it — the `detail` of a `406`, naming those formats.
     *
     * # Errors
     *
     * Any `kind` other than `"solutions"`, `"boolean"`, `"graph"` and `"dataset"`.
     * @param {string} kind
     * @returns {string}
     */
    static notAcceptableDetail(kind) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(kind, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.sparqlprotocolrequest_notAcceptableDetail(ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * The media types a result of `kind` (`"solutions"`, `"boolean"`, `"graph"`, or
     * `"dataset"` — a graph carrying named graphs) is offered in, in server preference
     * order.
     *
     * # Errors
     *
     * Any other `kind`.
     * @param {string} kind
     * @returns {string[]}
     */
    static offeredMediaTypes(kind) {
        const ptr0 = passStringToWasm0(kind, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.sparqlprotocolrequest_offeredMediaTypes(ptr0, len0);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v2 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v2;
    }
    /**
     * Read one protocol operation from an HTTP request's method, `Content-Type` header,
     * query string (without the leading `?`) and body bytes.
     *
     * # Errors
     *
     * The typed protocol refusal: `name` is the `ProtocolError` variant, `status` the
     * HTTP status, `parameter` the parameter at fault when there is one.
     * @param {string} method
     * @param {string | null | undefined} content_type
     * @param {string | null | undefined} query_string
     * @param {Uint8Array} body
     * @returns {SparqlProtocolRequest}
     */
    static parse(method, content_type, query_string, body) {
        const ptr0 = passStringToWasm0(method, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(content_type) ? 0 : passStringToWasm0(content_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(query_string) ? 0 : passStringToWasm0(query_string, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ptr3 = passArray8ToWasm0(body, wasm.__wbindgen_malloc);
        const len3 = WASM_VECTOR_LEN;
        const ret = wasm.sparqlprotocolrequest_parse(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return SparqlProtocolRequest.__wrap(ret[0]);
    }
    /**
     * The HTTP problem a failed operation is answered with: `error` is what it rejected
     * with, read for its `code` and `message`; `cancelled` says the request's own signal
     * stopped it, whatever it rejected with. An error without a string `code` is an
     * exception nothing classified — the host's own fault.
     * @param {any} error
     * @param {boolean} cancelled
     * @returns {FailureProblem}
     */
    static problemFor(error, cancelled) {
        const ret = wasm.sparqlprotocolrequest_problemFor(error, cancelled);
        return FailureProblem.__wrap(ret);
    }
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
     * @param {any} tripped
     * @returns {FailureProblem}
     */
    static problemForTrip(tripped) {
        const ret = wasm.sparqlprotocolrequest_problemForTrip(tripped);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return FailureProblem.__wrap(ret[0]);
    }
    /**
     * The query's result kind — `"solutions"` (`SELECT`), `"boolean"` (`ASK`) or `"graph"`
     * (`CONSTRUCT`/`DESCRIBE`) — read from its query form, or `undefined` for an update.
     *
     * # Errors
     *
     * The typed protocol refusal `MalformedOperation` when no query form follows the
     * prologue.
     * @returns {string | undefined}
     */
    get resultKind() {
        const ret = wasm.sparqlprotocolrequest_resultKind(this.__wbg_ptr);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * The reason phrase of an HTTP `status` (RFC 9110 §15) — an `about:blank`
     * problem's `title` — or `undefined` for a status no problem is answered with here.
     * @param {number} status
     * @returns {string | undefined}
     */
    static statusTitle(status) {
        const ret = wasm.sparqlprotocolrequest_statusTitle(status);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * The operation text as the request carried it, before any dataset parameter is
     * applied.
     * @returns {string}
     */
    get text() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.sparqlprotocolrequest_text(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Every `using-graph-uri`, in request order.
     * @returns {string[]}
     */
    get usingGraphUris() {
        const ret = wasm.sparqlprotocolrequest_usingGraphUris(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Every `using-named-graph-uri`, in request order.
     * @returns {string[]}
     */
    get usingNamedGraphUris() {
        const ret = wasm.sparqlprotocolrequest_usingNamedGraphUris(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
}
if (Symbol.dispose) SparqlProtocolRequest.prototype[Symbol.dispose] = SparqlProtocolRequest.prototype.free;
exports.SparqlProtocolRequest = SparqlProtocolRequest;

/**
 * What `purrdf_jspi_suspend` reports when the host resumes the job.
 * @enum {0 | 1 | 2}
 */
const SuspendStatus = Object.freeze({
    /**
     * The effect was answered.
     */
    Answered: 0, "0": "Answered",
    /**
     * The host abandoned the effect on the job's stop signal.
     */
    Abandoned: 1, "1": "Abandoned",
    /**
     * A fault is latched on the job.
     */
    Fault: 2, "2": "Fault",
});
exports.SuspendStatus = SuspendStatus;

/**
 * An RDF/JS [Term](https://rdf.js.org/data-model-spec/#term-interface).
 */
class Term {
    static __wrap(ptr) {
        const obj = Object.create(Term.prototype);
        obj.__wbg_ptr = ptr;
        TermFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        TermFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_term_free(ptr, 0);
    }
    /**
     * `datatype` — the literal's datatype as a `NamedNode`, or `undefined` for a
     * non-literal.
     * @returns {Term | undefined}
     */
    get datatype() {
        const ret = wasm.term_datatype(this.__wbg_ptr);
        return ret === 0 ? undefined : Term.__wrap(ret);
    }
    /**
     * `direction` — the RDF-1.2 base direction (`"ltr"`/`"rtl"`), or `""` when absent.
     * The deliberate extension to stock RDF/JS (`.goals`: overcome, don't inherit).
     * @returns {string}
     */
    get direction() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.term_direction(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Structural RDF/JS term equality.
     * @param {Term} other
     * @returns {boolean}
     */
    equals(other) {
        _assertClass(other, Term);
        const ret = wasm.term_equals(this.__wbg_ptr, other.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * `graph` of a quoted-triple term — always the default graph (a quoted triple has
     * no graph slot), else `undefined`.
     * @returns {Term | undefined}
     */
    get graph() {
        const ret = wasm.term_graph(this.__wbg_ptr);
        return ret === 0 ? undefined : Term.__wrap(ret);
    }
    /**
     * `language` — the literal's language tag, or `""` for a non-language-tagged term
     * (RDF/JS uses the empty string, not `undefined`).
     * @returns {string}
     */
    get language() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.term_language(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * `object` of a quoted-triple term, else `undefined`.
     * @returns {Term | undefined}
     */
    get object() {
        const ret = wasm.term_object(this.__wbg_ptr);
        return ret === 0 ? undefined : Term.__wrap(ret);
    }
    /**
     * `predicate` of a quoted-triple term as a `NamedNode`, else `undefined`.
     * @returns {Term | undefined}
     */
    get predicate() {
        const ret = wasm.term_predicate(this.__wbg_ptr);
        return ret === 0 ? undefined : Term.__wrap(ret);
    }
    /**
     * `subject` of a quoted-triple term (`termType: "Quad"`), else `undefined`.
     * @returns {Term | undefined}
     */
    get subject() {
        const ret = wasm.term_subject(this.__wbg_ptr);
        return ret === 0 ? undefined : Term.__wrap(ret);
    }
    /**
     * `termType` — the RDF/JS discriminator.
     * @returns {string}
     */
    get termType() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.term_term_type(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * `value` — the IRI, blank label, lexical form, or variable name. Empty for a
     * quoted triple and the default graph (per RDF/JS).
     * @returns {string}
     */
    get value() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.term_value(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) Term.prototype[Symbol.dispose] = Term.prototype.free;
exports.Term = Term;

/**
 * The governor that stopped one execution: which one, on which dimension, against which
 * ceiling.
 */
class TrippedGovernor {
    static __wrap(ptr) {
        const obj = Object.create(TrippedGovernor.prototype);
        obj.__wbg_ptr = ptr;
        TrippedGovernorFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        TrippedGovernorFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_trippedgovernor_free(ptr, 0);
    }
    /**
     * Which stop signal fired — `"cancelled"` or `"deadline-exceeded"` — or `undefined`
     * when a ceiling rather than a signal stopped the execution.
     * @returns {string | undefined}
     */
    get cause() {
        const ret = wasm.trippedgovernor_cause(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * Consumption charged before the refused work — a **measurement**, and present only
     * on the `"budget"` kind.
     * @returns {bigint | undefined}
     */
    get consumed() {
        const ret = wasm.trippedgovernor_consumed(this.__wbg_ptr);
        return ret[0] === 0 ? undefined : BigInt.asUintN(64, ret[1]);
    }
    /**
     * The governed dimension, e.g. `"fuel"` — `undefined` when a stop signal fired, which
     * belongs to no dimension.
     * @returns {string | undefined}
     */
    get dimension() {
        const ret = wasm.trippedgovernor_dimension(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * The planner's estimate that exceeded the ceiling — **not** a measurement, and
     * present only on the `"refused"` kind, where nothing ran to measure.
     * @returns {bigint | undefined}
     */
    get estimate() {
        const ret = wasm.trippedgovernor_estimate(this.__wbg_ptr);
        return ret[0] === 0 ? undefined : BigInt.asUintN(64, ret[1]);
    }
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
     * @returns {string}
     */
    get kind() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.trippedgovernor_kind(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * The stable kebab-case discriminant, e.g. `"deadline-exceeded"`. A pinned contract:
     * match on this rather than on the prose of `message`.
     * @returns {string}
     */
    get label() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.trippedgovernor_label(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * The inclusive ceiling in force, or `undefined` when a stop signal fired.
     * @returns {bigint | undefined}
     */
    get limit() {
        const ret = wasm.trippedgovernor_limit(this.__wbg_ptr);
        return ret[0] === 0 ? undefined : BigInt.asUintN(64, ret[1]);
    }
    /**
     * A human-readable rendering. Prose, not a contract — match on `label`.
     * @returns {string}
     */
    get message() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.trippedgovernor_message(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) TrippedGovernor.prototype[Symbol.dispose] = TrippedGovernor.prototype.free;
exports.TrippedGovernor = TrippedGovernor;

/**
 * The outcome of one governed SPARQL UPDATE.
 *
 * Deliberately not a [`QueryOutcome`] and deliberately without a partial arm: a query's
 * partial answer is a certifiable thing, a partial *mutation* is not. A tripped request
 * applied **nothing** — not "not all of it" — and left the dataset exactly as it found it.
 */
class UpdateOutcome {
    static __wrap(ptr) {
        const obj = Object.create(UpdateOutcome.prototype);
        obj.__wbg_ptr = ptr;
        UpdateOutcomeFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        UpdateOutcomeFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_updateoutcome_free(ptr, 0);
    }
    /**
     * Whether every operation of the request applied.
     *
     * `false` means **nothing** applied, never "not all of it applied".
     * @returns {boolean}
     */
    get isApplied() {
        const ret = wasm.updateoutcome_isApplied(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Move this request's receipt out. Present on both paths.
     * @returns {GovernorEvidence | undefined}
     */
    takeEvidence() {
        const ret = wasm.updateoutcome_takeEvidence(this.__wbg_ptr);
        return ret === 0 ? undefined : GovernorEvidence.__wrap(ret);
    }
    /**
     * Move the tripped governor out, or `undefined` when the request applied.
     * @returns {TrippedGovernor | undefined}
     */
    takeTripped() {
        const ret = wasm.updateoutcome_takeTripped(this.__wbg_ptr);
        return ret === 0 ? undefined : TrippedGovernor.__wrap(ret);
    }
}
if (Symbol.dispose) UpdateOutcome.prototype[Symbol.dispose] = UpdateOutcome.prototype.free;
exports.UpdateOutcome = UpdateOutcome;

/**
 * Seed the terminal counter once, then exercise its real typed refusal through JS.
 * @param {boolean} seed
 * @returns {bigint}
 */
function __purrdf_test_exchange_terminal(seed) {
    const ret = wasm.__purrdf_test_exchange_terminal(seed);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return BigInt.asUintN(64, ret[0]);
}
exports.__purrdf_test_exchange_terminal = __purrdf_test_exchange_terminal;

/**
 * Unit-test-module fixture only: seed the real exported getters beyond binary64's
 * exact integer range without adding a seed operation to the release package.
 * @returns {Dataset}
 */
function __purrdf_test_large_dataset_identity() {
    const ret = wasm.__purrdf_test_large_dataset_identity();
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Dataset.__wrap(ret[0]);
}
exports.__purrdf_test_large_dataset_identity = __purrdf_test_large_dataset_identity;

/**
 * Actual ABI fixtures belong only to the unit-test module, never the package.
 * Seed the real allocator temporarily so adjacent large IDs can be settled from JS.
 * @param {bigint} id
 * @returns {AsyncEffect}
 */
function __purrdf_test_open_exchange(id) {
    const ret = wasm.__purrdf_test_open_exchange(id);
    return AsyncEffect.__wrap(ret);
}
exports.__purrdf_test_open_exchange = __purrdf_test_open_exchange;

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
 * @returns {number}
 */
function asyncStackRegionBytes() {
    const ret = wasm.asyncStackRegionBytes();
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ret[0] >>> 0;
}
exports.asyncStackRegionBytes = asyncStackRegionBytes;

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
 * @param {string} regime
 * @param {string} document
 * @param {string} pattern
 * @param {string[]} import_iris
 * @param {string[]} import_documents
 * @param {string[]} premise_iris
 * @param {bigint | null} [max_stored_facts]
 * @param {bigint | null} [max_join_steps]
 * @returns {ReasoningAnswer}
 */
function entailCertainAnswers(regime, document, pattern, import_iris, import_documents, premise_iris, max_stored_facts, max_join_steps) {
    const ptr0 = passStringToWasm0(regime, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(document, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passStringToWasm0(pattern, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len2 = WASM_VECTOR_LEN;
    const ptr3 = passArrayJsValueToWasm0(import_iris, wasm.__wbindgen_malloc);
    const len3 = WASM_VECTOR_LEN;
    const ptr4 = passArrayJsValueToWasm0(import_documents, wasm.__wbindgen_malloc);
    const len4 = WASM_VECTOR_LEN;
    const ptr5 = passArrayJsValueToWasm0(premise_iris, wasm.__wbindgen_malloc);
    const len5 = WASM_VECTOR_LEN;
    const ret = wasm.entailCertainAnswers(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5, !isLikeNone(max_stored_facts), isLikeNone(max_stored_facts) ? BigInt(0) : max_stored_facts, !isLikeNone(max_join_steps), isLikeNone(max_join_steps) ? BigInt(0) : max_join_steps);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ReasoningAnswer.__wrap(ret[0]);
}
exports.entailCertainAnswers = entailCertainAnswers;

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
function entailCheckAbsentProof() {
    const ret = wasm.entailCheckAbsentProof();
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}
exports.entailCheckAbsentProof = entailCheckAbsentProof;

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
function entailCheckGoldenVectors() {
    const ret = wasm.entailCheckGoldenVectors();
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}
exports.entailCheckGoldenVectors = entailCheckGoldenVectors;

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
function entailCheckInconsistentRefusal() {
    const ret = wasm.entailCheckInconsistentRefusal();
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}
exports.entailCheckInconsistentRefusal = entailCheckInconsistentRefusal;

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
 * @param {string} document
 * @param {string} service
 * @param {string} argument
 * @param {string} answer
 * @param {string} certificate
 * @param {string} proof
 * @returns {string}
 */
function entailCheckProof(document, service, argument, answer, certificate, proof) {
    let deferred8_0;
    let deferred8_1;
    try {
        const ptr0 = passStringToWasm0(document, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(service, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(argument, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(answer, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        const ptr4 = passStringToWasm0(certificate, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len4 = WASM_VECTOR_LEN;
        const ptr5 = passStringToWasm0(proof, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len5 = WASM_VECTOR_LEN;
        const ret = wasm.entailCheckProof(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5);
        var ptr7 = ret[0];
        var len7 = ret[1];
        if (ret[3]) {
            ptr7 = 0; len7 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred8_0 = ptr7;
        deferred8_1 = len7;
        return getStringFromWasm0(ptr7, len7);
    } finally {
        wasm.__wbindgen_free(deferred8_0, deferred8_1, 1);
    }
}
exports.entailCheckProof = entailCheckProof;

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
function entailCheckProofGoldenVectors() {
    const ret = wasm.entailCheckProofGoldenVectors();
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}
exports.entailCheckProofGoldenVectors = entailCheckProofGoldenVectors;

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
 * @param {string} document
 * @param {number} step_cap
 * @param {number} work_cap
 * @returns {ReasoningAnswer}
 */
function entailClassify(document, step_cap, work_cap) {
    const ptr0 = passStringToWasm0(document, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.entailClassify(ptr0, len0, step_cap, work_cap);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ReasoningAnswer.__wrap(ret[0]);
}
exports.entailClassify = entailClassify;

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
 * @param {string} document
 * @param {string[]} import_iris
 * @param {string[]} import_documents
 * @param {string[]} premise_iris
 * @param {number} step_cap
 * @param {number} work_cap
 * @returns {ReasoningAnswer}
 */
function entailConsistency(document, import_iris, import_documents, premise_iris, step_cap, work_cap) {
    const ptr0 = passStringToWasm0(document, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passArrayJsValueToWasm0(import_iris, wasm.__wbindgen_malloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passArrayJsValueToWasm0(import_documents, wasm.__wbindgen_malloc);
    const len2 = WASM_VECTOR_LEN;
    const ptr3 = passArrayJsValueToWasm0(premise_iris, wasm.__wbindgen_malloc);
    const len3 = WASM_VECTOR_LEN;
    const ret = wasm.entailConsistency(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, step_cap, work_cap);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ReasoningAnswer.__wrap(ret[0]);
}
exports.entailConsistency = entailConsistency;

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
 * @param {string} document
 * @param {string} axiom
 * @param {number} step_cap
 * @param {number} work_cap
 * @returns {ReasoningAnswer}
 */
function entailEntails(document, axiom, step_cap, work_cap) {
    const ptr0 = passStringToWasm0(document, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(axiom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.entailEntails(ptr0, len0, ptr1, len1, step_cap, work_cap);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ReasoningAnswer.__wrap(ret[0]);
}
exports.entailEntails = entailEntails;

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
 * @param {string} document
 * @param {string} regime
 * @param {string} conclusion
 * @returns {ReasoningAnswer}
 */
function entailExplainConclusion(document, regime, conclusion) {
    const ptr0 = passStringToWasm0(document, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(regime, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passStringToWasm0(conclusion, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len2 = WASM_VECTOR_LEN;
    const ret = wasm.entailExplainConclusion(ptr0, len0, ptr1, len1, ptr2, len2);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ReasoningAnswer.__wrap(ret[0]);
}
exports.entailExplainConclusion = entailExplainConclusion;

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
 * @param {string} regime
 * @returns {string[]}
 */
function entailExtensions(regime) {
    const ptr0 = passStringToWasm0(regime, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.entailExtensions(ptr0, len0);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    var v2 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
    return v2;
}
exports.entailExtensions = entailExtensions;

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
 * @param {string} document
 * @param {string} signature
 * @param {string} method
 * @returns {ReasoningAnswer}
 */
function entailExtractModule(document, signature, method) {
    const ptr0 = passStringToWasm0(document, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(signature, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passStringToWasm0(method, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len2 = WASM_VECTOR_LEN;
    const ret = wasm.entailExtractModule(ptr0, len0, ptr1, len1, ptr2, len2);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ReasoningAnswer.__wrap(ret[0]);
}
exports.entailExtractModule = entailExtractModule;

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
 * @param {string} regime
 * @param {string} premise
 * @param {string} conclusion
 * @param {string[]} import_iris
 * @param {string[]} import_documents
 * @param {string[]} premise_iris
 * @param {bigint | null} [max_stored_facts]
 * @param {bigint | null} [max_join_steps]
 * @returns {ReasoningAnswer}
 */
function entailGraphEntails(regime, premise, conclusion, import_iris, import_documents, premise_iris, max_stored_facts, max_join_steps) {
    const ptr0 = passStringToWasm0(regime, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(premise, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passStringToWasm0(conclusion, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len2 = WASM_VECTOR_LEN;
    const ptr3 = passArrayJsValueToWasm0(import_iris, wasm.__wbindgen_malloc);
    const len3 = WASM_VECTOR_LEN;
    const ptr4 = passArrayJsValueToWasm0(import_documents, wasm.__wbindgen_malloc);
    const len4 = WASM_VECTOR_LEN;
    const ptr5 = passArrayJsValueToWasm0(premise_iris, wasm.__wbindgen_malloc);
    const len5 = WASM_VECTOR_LEN;
    const ret = wasm.entailGraphEntails(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5, !isLikeNone(max_stored_facts), isLikeNone(max_stored_facts) ? BigInt(0) : max_stored_facts, !isLikeNone(max_join_steps), isLikeNone(max_join_steps) ? BigInt(0) : max_join_steps);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ReasoningAnswer.__wrap(ret[0]);
}
exports.entailGraphEntails = entailGraphEntails;

/**
 * `entailImplementedRules(regime)` → the subset of `entailRules(regime)` this
 * build's chase actually fires today.
 *
 * `entailRules(r)` minus `entailImplementedRules(r)` is the regime's measurable
 * gap — the same set the rendered report's `missing` lines name.
 *
 * Throws for an unknown regime spelling, naming the accepted set.
 * @param {string} regime
 * @returns {string[]}
 */
function entailImplementedRules(regime) {
    const ptr0 = passStringToWasm0(regime, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.entailImplementedRules(ptr0, len0);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    var v2 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
    return v2;
}
exports.entailImplementedRules = entailImplementedRules;

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
 * @param {string} document
 * @param {string} _class
 * @param {number} step_cap
 * @param {number} work_cap
 * @returns {ReasoningAnswer}
 */
function entailInstances(document, _class, step_cap, work_cap) {
    const ptr0 = passStringToWasm0(document, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(_class, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.entailInstances(ptr0, len0, ptr1, len1, step_cap, work_cap);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ReasoningAnswer.__wrap(ret[0]);
}
exports.entailInstances = entailInstances;

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
 * @param {string} document
 * @param {string} axiom
 * @returns {ReasoningAnswer}
 */
function entailJustify(document, axiom) {
    const ptr0 = passStringToWasm0(document, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(axiom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.entailJustify(ptr0, len0, ptr1, len1);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ReasoningAnswer.__wrap(ret[0]);
}
exports.entailJustify = entailJustify;

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
 * @param {string} document
 * @param {string} regime
 * @param {string} program
 * @param {string[]} import_iris
 * @param {string[]} import_documents
 * @param {string[]} premise_iris
 * @param {bigint | null} [max_stored_facts]
 * @param {bigint | null} [max_join_steps]
 * @returns {RegimeClosure}
 */
function entailMaterialize(document, regime, program, import_iris, import_documents, premise_iris, max_stored_facts, max_join_steps) {
    const ptr0 = passStringToWasm0(document, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(regime, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passStringToWasm0(program, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len2 = WASM_VECTOR_LEN;
    const ptr3 = passArrayJsValueToWasm0(import_iris, wasm.__wbindgen_malloc);
    const len3 = WASM_VECTOR_LEN;
    const ptr4 = passArrayJsValueToWasm0(import_documents, wasm.__wbindgen_malloc);
    const len4 = WASM_VECTOR_LEN;
    const ptr5 = passArrayJsValueToWasm0(premise_iris, wasm.__wbindgen_malloc);
    const len5 = WASM_VECTOR_LEN;
    const ret = wasm.entailMaterialize(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5, !isLikeNone(max_stored_facts), isLikeNone(max_stored_facts) ? BigInt(0) : max_stored_facts, !isLikeNone(max_join_steps), isLikeNone(max_join_steps) ? BigInt(0) : max_join_steps);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return RegimeClosure.__wrap(ret[0]);
}
exports.entailMaterialize = entailMaterialize;

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
 * @param {string} document
 * @returns {ReasoningAnswer}
 */
function entailProfile(document) {
    const ptr0 = passStringToWasm0(document, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.entailProfile(ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ReasoningAnswer.__wrap(ret[0]);
}
exports.entailProfile = entailProfile;

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
 * @param {string} document
 * @param {string} service
 * @param {string} argument
 * @param {number} step_cap
 * @param {number} work_cap
 * @returns {ReasoningAnswer}
 */
function entailProve(document, service, argument, step_cap, work_cap) {
    const ptr0 = passStringToWasm0(document, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(service, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passStringToWasm0(argument, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len2 = WASM_VECTOR_LEN;
    const ret = wasm.entailProve(ptr0, len0, ptr1, len1, ptr2, len2, step_cap, work_cap);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ReasoningAnswer.__wrap(ret[0]);
}
exports.entailProve = entailProve;

/**
 * `entailRealize(document, stepCap, workCap)` → the entailed types of the ontology's named
 * individuals, and the most specific of them (`type` then `direct-type` lines).
 *
 * Throws on a malformed document or an ontology with no model.
 * @param {string} document
 * @param {number} step_cap
 * @param {number} work_cap
 * @returns {ReasoningAnswer}
 */
function entailRealize(document, step_cap, work_cap) {
    const ptr0 = passStringToWasm0(document, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.entailRealize(ptr0, len0, step_cap, work_cap);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ReasoningAnswer.__wrap(ret[0]);
}
exports.entailRealize = entailRealize;

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
 * @param {string} regime
 * @returns {string[]}
 */
function entailRules(regime) {
    const ptr0 = passStringToWasm0(regime, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.entailRules(ptr0, len0);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    var v2 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
    return v2;
}
exports.entailRules = entailRules;

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
 * @param {string} regime
 * @param {string} premise
 * @param {string} conclusion
 * @param {string[]} import_iris
 * @param {string[]} import_documents
 * @param {string[]} premise_iris
 * @param {bigint | null} [max_stored_facts]
 * @param {bigint | null} [max_join_steps]
 * @returns {ReasoningAnswer}
 */
function entailVerifyEntailment(regime, premise, conclusion, import_iris, import_documents, premise_iris, max_stored_facts, max_join_steps) {
    const ptr0 = passStringToWasm0(regime, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(premise, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passStringToWasm0(conclusion, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len2 = WASM_VECTOR_LEN;
    const ptr3 = passArrayJsValueToWasm0(import_iris, wasm.__wbindgen_malloc);
    const len3 = WASM_VECTOR_LEN;
    const ptr4 = passArrayJsValueToWasm0(import_documents, wasm.__wbindgen_malloc);
    const len4 = WASM_VECTOR_LEN;
    const ptr5 = passArrayJsValueToWasm0(premise_iris, wasm.__wbindgen_malloc);
    const len5 = WASM_VECTOR_LEN;
    const ret = wasm.entailVerifyEntailment(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5, !isLikeNone(max_stored_facts), isLikeNone(max_stored_facts) ? BigInt(0) : max_stored_facts, !isLikeNone(max_join_steps), isLikeNone(max_join_steps) ? BigInt(0) : max_join_steps);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ReasoningAnswer.__wrap(ret[0]);
}
exports.entailVerifyEntailment = entailVerifyEntailment;

/**
 * The XPath F&O error codes, `err:`-prefixed (`"err:FOAR0001"`), in the order the
 * evidence's `expressionErrors` vectors are indexed by.
 * @returns {string[]}
 */
function expressionErrorCodes() {
    const ret = wasm.expressionErrorCodes();
    var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
    return v1;
}
exports.expressionErrorCodes = expressionErrorCodes;

/**
 * The kernel's governed resource dimensions, by their stable kebab-case labels, in the
 * kernel's own declaration order.
 *
 * This order is the index order of the [`GovernorEvidence`] consumption and ceiling
 * vectors, so a consumer zips the three together rather than hard-coding a vocabulary the
 * engine owns. Exposed as a function rather than restated in JavaScript for exactly that
 * reason: a dimension added to the kernel appears here without the package root being
 * edited, and cannot silently drop out of a caller's evidence map.
 * @returns {string[]}
 */
function governorDimensions() {
    const ret = wasm.governorDimensions();
    var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
    return v1;
}
exports.governorDimensions = governorDimensions;

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
function install_stack_floor() {
    wasm.install_stack_floor();
}
exports.install_stack_floor = install_stack_floor;

/**
 * Lift a strict bidirectional USTAR package into an in-memory RDF dataset.
 * @param {Uint8Array} archive
 * @param {string} profile
 * @param {string} config_json
 * @returns {ProjectionLift}
 */
function liftProjection(archive, profile, config_json) {
    const ptr0 = passArray8ToWasm0(archive, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(profile, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passStringToWasm0(config_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len2 = WASM_VECTOR_LEN;
    const ret = wasm.liftProjection(ptr0, len0, ptr1, len1, ptr2, len2);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ProjectionLift.__wrap(ret[0]);
}
exports.liftProjection = liftProjection;

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
 * @param {string} json
 * @param {string} prefix
 * @param {string} iri
 * @returns {ProvenanceInfo}
 */
function provenanceFromJson(json, prefix, iri) {
    const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(prefix, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passStringToWasm0(iri, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len2 = WASM_VECTOR_LEN;
    const ret = wasm.provenanceFromJson(ptr0, len0, ptr1, len1, ptr2, len2);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ProvenanceInfo.__wrap(ret[0]);
}
exports.provenanceFromJson = provenanceFromJson;

/**
 * Decode the additive `purrdf` provenance extension a SPARQL-results XML document
 * carries under the namespace `prefix`/`iri` — the XML twin of [`provenance_from_json`].
 *
 * # Errors
 *
 * An invalid `prefix`/`iri`, malformed XML, or a non-`<sparql>` root.
 * @param {string} xml
 * @param {string} prefix
 * @param {string} iri
 * @returns {ProvenanceInfo}
 */
function provenanceFromXml(xml, prefix, iri) {
    const ptr0 = passStringToWasm0(xml, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(prefix, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passStringToWasm0(iri, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len2 = WASM_VECTOR_LEN;
    const ret = wasm.provenanceFromXml(ptr0, len0, ptr1, len1, ptr2, len2);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ProvenanceInfo.__wrap(ret[0]);
}
exports.provenanceFromXml = provenanceFromXml;

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
 * @param {string} data_nt
 * @param {string | null} [shapes_ttl]
 * @param {string | null} [srl]
 * @param {string | null} [shapes_base]
 * @param {string | null} [srl_base]
 * @param {boolean | null} [explain]
 * @param {string[] | null} [import_iris]
 * @param {string[] | null} [import_documents]
 * @param {string | null} [shapes_graph]
 * @param {bigint | null} [max_term_generating_rounds]
 * @param {bigint | null} [max_generated_terms]
 * @param {bigint | null} [max_stored_facts]
 * @param {bigint | null} [max_join_steps]
 * @param {string | null} [xpath_regex]
 * @returns {ShaclRulesInference}
 */
function shaclApplyRules(data_nt, shapes_ttl, srl, shapes_base, srl_base, explain, import_iris, import_documents, shapes_graph, max_term_generating_rounds, max_generated_terms, max_stored_facts, max_join_steps, xpath_regex) {
    const ptr0 = passStringToWasm0(data_nt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    var ptr1 = isLikeNone(shapes_ttl) ? 0 : passStringToWasm0(shapes_ttl, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len1 = WASM_VECTOR_LEN;
    var ptr2 = isLikeNone(srl) ? 0 : passStringToWasm0(srl, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len2 = WASM_VECTOR_LEN;
    var ptr3 = isLikeNone(shapes_base) ? 0 : passStringToWasm0(shapes_base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len3 = WASM_VECTOR_LEN;
    var ptr4 = isLikeNone(srl_base) ? 0 : passStringToWasm0(srl_base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len4 = WASM_VECTOR_LEN;
    var ptr5 = isLikeNone(import_iris) ? 0 : passArrayJsValueToWasm0(import_iris, wasm.__wbindgen_malloc);
    var len5 = WASM_VECTOR_LEN;
    var ptr6 = isLikeNone(import_documents) ? 0 : passArrayJsValueToWasm0(import_documents, wasm.__wbindgen_malloc);
    var len6 = WASM_VECTOR_LEN;
    var ptr7 = isLikeNone(shapes_graph) ? 0 : passStringToWasm0(shapes_graph, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len7 = WASM_VECTOR_LEN;
    var ptr8 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len8 = WASM_VECTOR_LEN;
    const ret = wasm.shaclApplyRules(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, isLikeNone(explain) ? 0xFFFFFF : explain ? 1 : 0, ptr5, len5, ptr6, len6, ptr7, len7, !isLikeNone(max_term_generating_rounds), isLikeNone(max_term_generating_rounds) ? BigInt(0) : max_term_generating_rounds, !isLikeNone(max_generated_terms), isLikeNone(max_generated_terms) ? BigInt(0) : max_generated_terms, !isLikeNone(max_stored_facts), isLikeNone(max_stored_facts) ? BigInt(0) : max_stored_facts, !isLikeNone(max_join_steps), isLikeNone(max_join_steps) ? BigInt(0) : max_join_steps, ptr8, len8);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ShaclRulesInference.__wrap(ret[0]);
}
exports.shaclApplyRules = shaclApplyRules;

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
 * @param {string} srl
 * @param {string | null} [srl_base]
 * @param {string[] | null} [import_iris]
 * @param {string[] | null} [import_documents]
 * @param {string | null} [level]
 * @returns {ShaclRulesCheck}
 */
function shaclCheckRules(srl, srl_base, import_iris, import_documents, level) {
    const ptr0 = passStringToWasm0(srl, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    var ptr1 = isLikeNone(srl_base) ? 0 : passStringToWasm0(srl_base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len1 = WASM_VECTOR_LEN;
    var ptr2 = isLikeNone(import_iris) ? 0 : passArrayJsValueToWasm0(import_iris, wasm.__wbindgen_malloc);
    var len2 = WASM_VECTOR_LEN;
    var ptr3 = isLikeNone(import_documents) ? 0 : passArrayJsValueToWasm0(import_documents, wasm.__wbindgen_malloc);
    var len3 = WASM_VECTOR_LEN;
    var ptr4 = isLikeNone(level) ? 0 : passStringToWasm0(level, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len4 = WASM_VECTOR_LEN;
    const ret = wasm.shaclCheckRules(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ShaclRulesCheck.__wrap(ret[0]);
}
exports.shaclCheckRules = shaclCheckRules;

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
 * @param {string} shapes_ttl
 * @param {string} data_nt
 * @param {string | null} [shapes_base]
 * @param {string[] | null} [import_iris]
 * @param {string[] | null} [import_documents]
 * @param {string | null} [shapes_graph]
 * @param {bigint | null} [max_term_generating_rounds]
 * @param {bigint | null} [max_generated_terms]
 * @param {bigint | null} [max_stored_facts]
 * @param {bigint | null} [max_join_steps]
 * @param {string | null} [xpath_regex]
 * @returns {ShaclEntailment}
 */
function shaclEntail(shapes_ttl, data_nt, shapes_base, import_iris, import_documents, shapes_graph, max_term_generating_rounds, max_generated_terms, max_stored_facts, max_join_steps, xpath_regex) {
    const ptr0 = passStringToWasm0(shapes_ttl, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(data_nt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    var ptr2 = isLikeNone(shapes_base) ? 0 : passStringToWasm0(shapes_base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len2 = WASM_VECTOR_LEN;
    var ptr3 = isLikeNone(import_iris) ? 0 : passArrayJsValueToWasm0(import_iris, wasm.__wbindgen_malloc);
    var len3 = WASM_VECTOR_LEN;
    var ptr4 = isLikeNone(import_documents) ? 0 : passArrayJsValueToWasm0(import_documents, wasm.__wbindgen_malloc);
    var len4 = WASM_VECTOR_LEN;
    var ptr5 = isLikeNone(shapes_graph) ? 0 : passStringToWasm0(shapes_graph, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len5 = WASM_VECTOR_LEN;
    var ptr6 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len6 = WASM_VECTOR_LEN;
    const ret = wasm.shaclEntail(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5, !isLikeNone(max_term_generating_rounds), isLikeNone(max_term_generating_rounds) ? BigInt(0) : max_term_generating_rounds, !isLikeNone(max_generated_terms), isLikeNone(max_generated_terms) ? BigInt(0) : max_generated_terms, !isLikeNone(max_stored_facts), isLikeNone(max_stored_facts) ? BigInt(0) : max_stored_facts, !isLikeNone(max_join_steps), isLikeNone(max_join_steps) ? BigInt(0) : max_join_steps, ptr6, len6);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ShaclEntailment.__wrap(ret[0]);
}
exports.shaclEntail = shaclEntail;

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
 * @param {string} shapes_ttl
 * @param {string} data_nt
 * @param {string | null | undefined} expr
 * @param {string} focus
 * @param {string[] | null} [scope]
 * @param {string | null} [shapes_base]
 * @param {string[] | null} [import_iris]
 * @param {string[] | null} [import_documents]
 * @param {string | null} [expr_at]
 * @param {string[] | null} [expr_via]
 * @param {string | null} [expr_turtle]
 * @param {string | null} [xpath_regex]
 * @returns {ShaclNodeExprOutcome}
 */
function shaclEvalNodeExpr(shapes_ttl, data_nt, expr, focus, scope, shapes_base, import_iris, import_documents, expr_at, expr_via, expr_turtle, xpath_regex) {
    const ptr0 = passStringToWasm0(shapes_ttl, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(data_nt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    var ptr2 = isLikeNone(expr) ? 0 : passStringToWasm0(expr, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len2 = WASM_VECTOR_LEN;
    const ptr3 = passStringToWasm0(focus, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len3 = WASM_VECTOR_LEN;
    var ptr4 = isLikeNone(scope) ? 0 : passArrayJsValueToWasm0(scope, wasm.__wbindgen_malloc);
    var len4 = WASM_VECTOR_LEN;
    var ptr5 = isLikeNone(shapes_base) ? 0 : passStringToWasm0(shapes_base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len5 = WASM_VECTOR_LEN;
    var ptr6 = isLikeNone(import_iris) ? 0 : passArrayJsValueToWasm0(import_iris, wasm.__wbindgen_malloc);
    var len6 = WASM_VECTOR_LEN;
    var ptr7 = isLikeNone(import_documents) ? 0 : passArrayJsValueToWasm0(import_documents, wasm.__wbindgen_malloc);
    var len7 = WASM_VECTOR_LEN;
    var ptr8 = isLikeNone(expr_at) ? 0 : passStringToWasm0(expr_at, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len8 = WASM_VECTOR_LEN;
    var ptr9 = isLikeNone(expr_via) ? 0 : passArrayJsValueToWasm0(expr_via, wasm.__wbindgen_malloc);
    var len9 = WASM_VECTOR_LEN;
    var ptr10 = isLikeNone(expr_turtle) ? 0 : passStringToWasm0(expr_turtle, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len10 = WASM_VECTOR_LEN;
    var ptr11 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len11 = WASM_VECTOR_LEN;
    const ret = wasm.shaclEvalNodeExpr(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5, ptr6, len6, ptr7, len7, ptr8, len8, ptr9, len9, ptr10, len10, ptr11, len11);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ShaclNodeExprOutcome.__wrap(ret[0]);
}
exports.shaclEvalNodeExpr = shaclEvalNodeExpr;

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
 * @param {string} shapes_ttl
 * @param {string | null} [shapes_base]
 * @param {string[] | null} [import_iris]
 * @param {string[] | null} [import_documents]
 * @param {string | null} [shapes_graph]
 * @returns {ShaclLintReport}
 */
function shaclLintShapes(shapes_ttl, shapes_base, import_iris, import_documents, shapes_graph) {
    const ptr0 = passStringToWasm0(shapes_ttl, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    var ptr1 = isLikeNone(shapes_base) ? 0 : passStringToWasm0(shapes_base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len1 = WASM_VECTOR_LEN;
    var ptr2 = isLikeNone(import_iris) ? 0 : passArrayJsValueToWasm0(import_iris, wasm.__wbindgen_malloc);
    var len2 = WASM_VECTOR_LEN;
    var ptr3 = isLikeNone(import_documents) ? 0 : passArrayJsValueToWasm0(import_documents, wasm.__wbindgen_malloc);
    var len3 = WASM_VECTOR_LEN;
    var ptr4 = isLikeNone(shapes_graph) ? 0 : passStringToWasm0(shapes_graph, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len4 = WASM_VECTOR_LEN;
    const ret = wasm.shaclLintShapes(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ShaclLintReport.__wrap(ret[0]);
}
exports.shaclLintShapes = shaclLintShapes;

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
 * @param {string} shapes_ttl
 * @param {string | null} [shapes_base]
 * @param {string[] | null} [import_iris]
 * @param {string[] | null} [import_documents]
 * @param {string | null} [shapes_graph]
 * @returns {Uint8Array}
 */
function shaclPackProduct(shapes_ttl, shapes_base, import_iris, import_documents, shapes_graph) {
    const ptr0 = passStringToWasm0(shapes_ttl, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    var ptr1 = isLikeNone(shapes_base) ? 0 : passStringToWasm0(shapes_base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len1 = WASM_VECTOR_LEN;
    var ptr2 = isLikeNone(import_iris) ? 0 : passArrayJsValueToWasm0(import_iris, wasm.__wbindgen_malloc);
    var len2 = WASM_VECTOR_LEN;
    var ptr3 = isLikeNone(import_documents) ? 0 : passArrayJsValueToWasm0(import_documents, wasm.__wbindgen_malloc);
    var len3 = WASM_VECTOR_LEN;
    var ptr4 = isLikeNone(shapes_graph) ? 0 : passStringToWasm0(shapes_graph, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len4 = WASM_VECTOR_LEN;
    const ret = wasm.shaclPackProduct(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    var v6 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v6;
}
exports.shaclPackProduct = shaclPackProduct;

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
 * @param {Uint8Array} product
 */
function shaclProductCertify(product) {
    const ptr0 = passArray8ToWasm0(product, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.shaclProductCertify(ptr0, len0);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}
exports.shaclProductCertify = shaclProductCertify;

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
 * @param {Uint8Array} product
 * @returns {string}
 */
function shaclProductExplain(product) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passArray8ToWasm0(product, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.shaclProductExplain(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}
exports.shaclProductExplain = shaclProductExplain;

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
 * @param {Uint8Array} product
 * @param {string} data_nt
 * @param {string | null} [xpath_regex]
 * @returns {string}
 */
function shaclProductValidateToSarif(product, data_nt, xpath_regex) {
    let deferred5_0;
    let deferred5_1;
    try {
        const ptr0 = passArray8ToWasm0(product, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(data_nt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.shaclProductValidateToSarif(ptr0, len0, ptr1, len1, ptr2, len2);
        var ptr4 = ret[0];
        var len4 = ret[1];
        if (ret[3]) {
            ptr4 = 0; len4 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred5_0 = ptr4;
        deferred5_1 = len4;
        return getStringFromWasm0(ptr4, len4);
    } finally {
        wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
    }
}
exports.shaclProductValidateToSarif = shaclProductValidateToSarif;

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
 * @param {Uint8Array} product
 * @param {string} data_nt
 * @param {string} expect_identity
 * @param {string | null} [xpath_regex]
 * @returns {string}
 */
function shaclProductValidateToSarifExpecting(product, data_nt, expect_identity, xpath_regex) {
    let deferred6_0;
    let deferred6_1;
    try {
        const ptr0 = passArray8ToWasm0(product, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(data_nt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(expect_identity, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        var ptr3 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len3 = WASM_VECTOR_LEN;
        const ret = wasm.shaclProductValidateToSarifExpecting(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3);
        var ptr5 = ret[0];
        var len5 = ret[1];
        if (ret[3]) {
            ptr5 = 0; len5 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred6_0 = ptr5;
        deferred6_1 = len5;
        return getStringFromWasm0(ptr5, len5);
    } finally {
        wasm.__wbindgen_free(deferred6_0, deferred6_1, 1);
    }
}
exports.shaclProductValidateToSarifExpecting = shaclProductValidateToSarifExpecting;

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
 * @param {Uint8Array} product
 * @param {string} data_nt
 * @param {string | null} [xpath_regex]
 * @returns {string}
 */
function shaclProductValidateToSarifRebuild(product, data_nt, xpath_regex) {
    let deferred5_0;
    let deferred5_1;
    try {
        const ptr0 = passArray8ToWasm0(product, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(data_nt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.shaclProductValidateToSarifRebuild(ptr0, len0, ptr1, len1, ptr2, len2);
        var ptr4 = ret[0];
        var len4 = ret[1];
        if (ret[3]) {
            ptr4 = 0; len4 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred5_0 = ptr4;
        deferred5_1 = len4;
        return getStringFromWasm0(ptr4, len4);
    } finally {
        wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
    }
}
exports.shaclProductValidateToSarifRebuild = shaclProductValidateToSarifRebuild;

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
 * @param {Uint8Array} product
 * @param {string} data_nt
 * @param {string} expect_identity
 * @param {string | null} [xpath_regex]
 * @returns {string}
 */
function shaclProductValidateToSarifRebuildExpecting(product, data_nt, expect_identity, xpath_regex) {
    let deferred6_0;
    let deferred6_1;
    try {
        const ptr0 = passArray8ToWasm0(product, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(data_nt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(expect_identity, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        var ptr3 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len3 = WASM_VECTOR_LEN;
        const ret = wasm.shaclProductValidateToSarifRebuildExpecting(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3);
        var ptr5 = ret[0];
        var len5 = ret[1];
        if (ret[3]) {
            ptr5 = 0; len5 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred6_0 = ptr5;
        deferred6_1 = len5;
        return getStringFromWasm0(ptr5, len5);
    } finally {
        wasm.__wbindgen_free(deferred6_0, deferred6_1, 1);
    }
}
exports.shaclProductValidateToSarifRebuildExpecting = shaclProductValidateToSarifRebuildExpecting;

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
 * @param {string} shapes_ttl
 * @param {string} data_nt
 * @param {string | null} [added_nt]
 * @param {string | null} [removed_nt]
 * @param {string | null} [shapes_base]
 * @param {string[] | null} [import_iris]
 * @param {string[] | null} [import_documents]
 * @param {string | null} [shapes_graph]
 * @param {string | null} [xpath_regex]
 * @returns {ShaclChangeValidation}
 */
function shaclValidateChangesToSarif(shapes_ttl, data_nt, added_nt, removed_nt, shapes_base, import_iris, import_documents, shapes_graph, xpath_regex) {
    const ptr0 = passStringToWasm0(shapes_ttl, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(data_nt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    var ptr2 = isLikeNone(added_nt) ? 0 : passStringToWasm0(added_nt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len2 = WASM_VECTOR_LEN;
    var ptr3 = isLikeNone(removed_nt) ? 0 : passStringToWasm0(removed_nt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len3 = WASM_VECTOR_LEN;
    var ptr4 = isLikeNone(shapes_base) ? 0 : passStringToWasm0(shapes_base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len4 = WASM_VECTOR_LEN;
    var ptr5 = isLikeNone(import_iris) ? 0 : passArrayJsValueToWasm0(import_iris, wasm.__wbindgen_malloc);
    var len5 = WASM_VECTOR_LEN;
    var ptr6 = isLikeNone(import_documents) ? 0 : passArrayJsValueToWasm0(import_documents, wasm.__wbindgen_malloc);
    var len6 = WASM_VECTOR_LEN;
    var ptr7 = isLikeNone(shapes_graph) ? 0 : passStringToWasm0(shapes_graph, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len7 = WASM_VECTOR_LEN;
    var ptr8 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len8 = WASM_VECTOR_LEN;
    const ret = wasm.shaclValidateChangesToSarif(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5, ptr6, len6, ptr7, len7, ptr8, len8);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ShaclChangeValidation.__wrap(ret[0]);
}
exports.shaclValidateChangesToSarif = shaclValidateChangesToSarif;

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
 * @param {string} shapes_ttl
 * @param {string} data_nt
 * @param {string | null} [shapes_base]
 * @param {string[] | null} [conformance_disallows]
 * @param {string[] | null} [import_iris]
 * @param {string[] | null} [import_documents]
 * @param {string | null} [shapes_graph]
 * @param {boolean | null} [subclass_of_in_shapes_graph]
 * @param {string | null} [xpath_regex]
 * @returns {string}
 */
function shaclValidateToSarif(shapes_ttl, data_nt, shapes_base, conformance_disallows, import_iris, import_documents, shapes_graph, subclass_of_in_shapes_graph, xpath_regex) {
    let deferred10_0;
    let deferred10_1;
    try {
        const ptr0 = passStringToWasm0(shapes_ttl, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(data_nt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(shapes_base) ? 0 : passStringToWasm0(shapes_base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        var ptr3 = isLikeNone(conformance_disallows) ? 0 : passArrayJsValueToWasm0(conformance_disallows, wasm.__wbindgen_malloc);
        var len3 = WASM_VECTOR_LEN;
        var ptr4 = isLikeNone(import_iris) ? 0 : passArrayJsValueToWasm0(import_iris, wasm.__wbindgen_malloc);
        var len4 = WASM_VECTOR_LEN;
        var ptr5 = isLikeNone(import_documents) ? 0 : passArrayJsValueToWasm0(import_documents, wasm.__wbindgen_malloc);
        var len5 = WASM_VECTOR_LEN;
        var ptr6 = isLikeNone(shapes_graph) ? 0 : passStringToWasm0(shapes_graph, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len6 = WASM_VECTOR_LEN;
        var ptr7 = isLikeNone(xpath_regex) ? 0 : passStringToWasm0(xpath_regex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len7 = WASM_VECTOR_LEN;
        const ret = wasm.shaclValidateToSarif(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5, ptr6, len6, isLikeNone(subclass_of_in_shapes_graph) ? 0xFFFFFF : subclass_of_in_shapes_graph ? 1 : 0, ptr7, len7);
        var ptr9 = ret[0];
        var len9 = ret[1];
        if (ret[3]) {
            ptr9 = 0; len9 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred10_0 = ptr9;
        deferred10_1 = len9;
        return getStringFromWasm0(ptr9, len9);
    } finally {
        wasm.__wbindgen_free(deferred10_0, deferred10_1, 1);
    }
}
exports.shaclValidateToSarif = shaclValidateToSarif;

/**
 * The purrdf engine version (the crate's SemVer), exposed to JS as `version()`.
 *
 * A liveness probe for the wasm build + the npm package: importing `purrdf` and
 * calling `version()` proves the module instantiated and the engine linked.
 * @returns {string}
 */
function version() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.version();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}
exports.version = version;

/**
 * The XSD canonical lexical form of `lexical` read as the datatype IRI `datatype`
 * (`"+007"` as `xsd:integer` is `"7"`, `"1.50"` as `xsd:decimal` is `"1.5"`).
 *
 * Integers and decimals are exact at any length. Returns `undefined` when `datatype`
 * is not an XSD datatype the engine maps, or when `lexical` is not in its lexical
 * space.
 * @param {string} lexical
 * @param {string} datatype
 * @returns {string | undefined}
 */
function xsdCanonicalLexical(lexical, datatype) {
    const ptr0 = passStringToWasm0(lexical, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(datatype, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.xsdCanonicalLexical(ptr0, len0, ptr1, len1);
    let v3;
    if (ret[0] !== 0) {
        v3 = getStringFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    }
    return v3;
}
exports.xsdCanonicalLexical = xsdCanonicalLexical;

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
 * @param {string} left_lexical
 * @param {string} left_datatype
 * @param {string} right_lexical
 * @param {string} right_datatype
 * @returns {number | undefined}
 */
function xsdValueCompare(left_lexical, left_datatype, right_lexical, right_datatype) {
    const ptr0 = passStringToWasm0(left_lexical, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(left_datatype, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passStringToWasm0(right_lexical, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len2 = WASM_VECTOR_LEN;
    const ptr3 = passStringToWasm0(right_datatype, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len3 = WASM_VECTOR_LEN;
    const ret = wasm.xsdValueCompare(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3);
    return ret[0] === 0 ? undefined : ret[1];
}
exports.xsdValueCompare = xsdValueCompare;
const import1 = require("./purrdf_jspi.mjs");
const import2 = require("./purrdf_jspi.mjs");

function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg_Error_fdd633d4bb5dd76a: function(arg0, arg1) {
            const ret = Error(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg___wbindgen_bigint_get_as_i64_d9e915702856f831: function(arg0, arg1) {
            const v = arg1;
            const ret = typeof(v) === 'bigint' ? v : undefined;
            getDataViewMemory0().setBigInt64(arg0 + 8 * 1, isLikeNone(ret) ? BigInt(0) : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_boolean_get_edaed31a367ce1bd: function(arg0) {
            const v = arg0;
            const ret = typeof(v) === 'boolean' ? v : undefined;
            return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
        },
        __wbg___wbindgen_debug_string_8a447059637473e2: function(arg0, arg1) {
            const ret = debugString(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_is_bigint_90b5ccfe67c78460: function(arg0) {
            const ret = typeof(arg0) === 'bigint';
            return ret;
        },
        __wbg___wbindgen_is_null_6d937fbfb6478470: function(arg0) {
            const ret = arg0 === null;
            return ret;
        },
        __wbg___wbindgen_is_object_0beba4a1980d3eea: function(arg0) {
            const val = arg0;
            const ret = typeof(val) === 'object' && val !== null;
            return ret;
        },
        __wbg___wbindgen_is_undefined_721f8decd50c87a3: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_jsval_eq_4e8c38722cb8ff51: function(arg0, arg1) {
            const ret = arg0 === arg1;
            return ret;
        },
        __wbg___wbindgen_number_get_1cc01dd708740256: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_string_get_71bb4348194e31f0: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_throw_ea4887a5f8f9a9db: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg_code_aa125d603cc54b40: function(arg0) {
            const ret = arg0.code;
            return ret;
        },
        __wbg_consumed_71cee05a6efa4def: function(arg0) {
            const ret = arg0.consumed;
            return ret;
        },
        __wbg_copy_8e64879acd056840: function() { return handleError(function (arg0) {
            const ret = arg0.copy();
            return ret;
        }, arguments); },
        __wbg_dataset_unwrap: function(arg0) {
            const ret = Dataset.__unwrap(arg0);
            return ret;
        },
        __wbg_dimension_bb9abfc2342a9489: function(arg0) {
            const ret = arg0.dimension;
            return ret;
        },
        __wbg_estimate_112e98228fd938f2: function(arg0) {
            const ret = arg0.estimate;
            return ret;
        },
        __wbg_get_9fd41b7ab6984e3a: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.get(arg0, getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments); },
        __wbg_isArray_a719743027ae3b3b: function(arg0) {
            const ret = Array.isArray(arg0);
            return ret;
        },
        __wbg_keys_62307c14cd1b96ac: function(arg0, arg1) {
            const ret = Object.keys(arg1);
            const ptr1 = passArrayJsValueToWasm0(ret, wasm.__wbindgen_malloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_kind_383e925cb873a8c1: function(arg0) {
            const ret = arg0.kind;
            return ret;
        },
        __wbg_label_0a060b22ebf48aa8: function(arg0) {
            const ret = arg0.label;
            return ret;
        },
        __wbg_limit_49ce10e253f534dc: function(arg0) {
            const ret = arg0.limit;
            return ret;
        },
        __wbg_message_4c9d93c12a41660b: function(arg0) {
            const ret = arg0.message;
            return ret;
        },
        __wbg_new_1171081c6e849937: function(arg0, arg1) {
            const ret = new Error(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_3c0f723dc788e1a6: function(arg0, arg1) {
            const ret = new TypeError(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_d706cf7d033e5e41: function(arg0, arg1) {
            const ret = new Error(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_now_7055c5bf7ae696bd: function() {
            const ret = Date.now();
            return ret;
        },
        __wbg_parse_998a7b86037695ab: function() { return handleError(function (arg0, arg1) {
            const ret = JSON.parse(getStringFromWasm0(arg0, arg1));
            return ret;
        }, arguments); },
        __wbg_quad_new: function(arg0) {
            const ret = Quad.__wrap(arg0);
            return ret;
        },
        __wbg_random_e238afeb2ce7773e: function() {
            const ret = Math.random();
            return ret;
        },
        __wbg_servicecatalog_unwrap: function(arg0) {
            const ret = ServiceCatalog.__unwrap(arg0);
            return ret;
        },
        __wbg_set_code_4c54e2d8bb33b572: function(arg0, arg1, arg2) {
            arg0.code = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_code_83408655a34d782e: function(arg0, arg1, arg2) {
            arg0.code = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_name_01c69096ae19b5cf: function(arg0, arg1, arg2) {
            arg0.name = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_parameter_af28cf554755455d: function(arg0, arg1, arg2) {
            arg0.parameter = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_presentation_1bacf593b4cbf0b2: function(arg0, arg1) {
            arg0.presentation = arg1;
        },
        __wbg_set_status_cc22422b2454aeed: function(arg0, arg1) {
            arg0.status = arg1;
        },
        __wbg_shacldiagnostic_new: function(arg0) {
            const ret = ShaclDiagnostic.__wrap(arg0);
            return ret;
        },
        __wbg_shaclimporterror_new: function(arg0) {
            const ret = ShaclImportError.__wrap(arg0);
            return ret;
        },
        __wbg_shaclproductrefusal_new: function(arg0) {
            const ret = ShaclProductRefusal.__wrap(arg0);
            return ret;
        },
        __wbg_silencedinvocation_new: function(arg0) {
            const ret = SilencedInvocation.__wrap(arg0);
            return ret;
        },
        __wbg_snapshot_9a2be9f469f51888: function() { return handleError(function (arg0) {
            const ret = arg0.snapshot();
            return ret;
        }, arguments); },
        __wbg_static_accessor_GLOBAL_THIS_a7eb9084bcbd58ae: function() {
            const ret = globalThis;
            return ret;
        },
        __wbg_write_c24e6d05dff48b9c: function() { return handleError(function (arg0, arg1, arg2) {
            var v0 = getArrayU8FromWasm0(arg1, arg2).slice();
            wasm.__wbindgen_free(arg1, arg2 * 1, 1);
            arg0.write(v0);
        }, arguments); },
        __wbindgen_cast_0000000000000001: function(arg0) {
            // Cast intrinsic for `I64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_cast_0000000000000003: function(arg0) {
            // Cast intrinsic for `U64 -> Externref`.
            const ret = BigInt.asUintN(64, arg0);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./fixture_bg.js": import0,
        "./purrdf_jspi.mjs": import1,
        "./purrdf_jspi.mjs": import2,
    };
}

const AsyncEffectFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_asynceffect_free(ptr, 1));
const AsyncEvidenceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_asyncevidence_free(ptr, 1));
const AsyncJobFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_asyncjob_free(ptr, 1));
const AsyncJobOptionsFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_asyncjoboptions_free(ptr, 1));
const CancellationTokenFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_cancellationtoken_free(ptr, 1));
const CompiledJsonLdContextFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_compiledjsonldcontext_free(ptr, 1));
const DataFactoryFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_datafactory_free(ptr, 1));
const DatasetFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_dataset_free(ptr, 1));
const EntailmentQueryOutcomeFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_entailmentqueryoutcome_free(ptr, 1));
const FailureProblemFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_failureproblem_free(ptr, 1));
const GeoPointIndexFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_geopointindex_free(ptr, 1));
const GeoSessionFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_geosession_free(ptr, 1));
const GovernorEvidenceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_governorevidence_free(ptr, 1));
const NegotiatedOutcomeFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_negotiatedoutcome_free(ptr, 1));
const PartialAnswersFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_partialanswers_free(ptr, 1));
const ProjectionLiftFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_projectionlift_free(ptr, 1));
const ProjectionPackageFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_projectionpackage_free(ptr, 1));
const ProvenanceInfoFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_provenanceinfo_free(ptr, 1));
const QuadFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_quad_free(ptr, 1));
const QueryEngineFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_queryengine_free(ptr, 1));
const QueryOutcomeFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_queryoutcome_free(ptr, 1));
const QueryResultFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_queryresult_free(ptr, 1));
const ReasonerFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_reasoner_free(ptr, 1));
const ReasoningAnswerFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_reasoninganswer_free(ptr, 1));
const RegimeClosureFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_regimeclosure_free(ptr, 1));
const SelectResultFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_selectresult_free(ptr, 1));
const SelectRowFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_selectrow_free(ptr, 1));
const SerializeLossFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_serializeloss_free(ptr, 1));
const ServiceCatalogFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_servicecatalog_free(ptr, 1));
const ShaclChangeValidationFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_shaclchangevalidation_free(ptr, 1));
const ShaclDiagnosticFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_shacldiagnostic_free(ptr, 1));
const ShaclEntailmentFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_shaclentailment_free(ptr, 1));
const ShaclImportErrorFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_shaclimporterror_free(ptr, 1));
const ShaclJobRequestFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_shacljobrequest_free(ptr, 1));
const ShaclLintReportFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_shacllintreport_free(ptr, 1));
const ShaclNodeExprOutcomeFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_shaclnodeexproutcome_free(ptr, 1));
const ShaclProductRefusalFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_shaclproductrefusal_free(ptr, 1));
const ShaclRulesCheckFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_shaclrulescheck_free(ptr, 1));
const ShaclRulesInferenceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_shaclrulesinference_free(ptr, 1));
const SilencedInvocationFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_silencedinvocation_free(ptr, 1));
const SinkFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_sink_free(ptr, 1));
const SparqlProtocolRequestFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_sparqlprotocolrequest_free(ptr, 1));
const TermFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_term_free(ptr, 1));
const TrippedGovernorFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_trippedgovernor_free(ptr, 1));
const UpdateOutcomeFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_updateoutcome_free(ptr, 1));

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
}

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function getArrayF64FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat64ArrayMemory0().subarray(ptr / 8, ptr / 8 + len);
}

function getArrayJsValueFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    const mem = getDataViewMemory0();
    const result = [];
    for (let i = ptr; i < ptr + 4 * len; i += 4) {
        result.push(wasm.__wbindgen_externrefs.get(mem.getUint32(i, true)));
    }
    wasm.__externref_drop_slice(ptr, len);
    return result;
}

function getArrayU64FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getBigUint64ArrayMemory0().subarray(ptr / 8, ptr / 8 + len);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedBigUint64ArrayMemory0 = null;
function getBigUint64ArrayMemory0() {
    if (cachedBigUint64ArrayMemory0 === null || cachedBigUint64ArrayMemory0.byteLength === 0) {
        cachedBigUint64ArrayMemory0 = new BigUint64Array(wasm.memory.buffer);
    }
    return cachedBigUint64ArrayMemory0;
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedFloat64ArrayMemory0 = null;
function getFloat64ArrayMemory0() {
    if (cachedFloat64ArrayMemory0 === null || cachedFloat64ArrayMemory0.byteLength === 0) {
        cachedFloat64ArrayMemory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachedFloat64ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passArrayJsValueToWasm0(array, malloc) {
    const ptr = malloc(array.length * 4, 4) >>> 0;
    for (let i = 0; i < array.length; i++) {
        const add = addToExternrefTable0(array[i]);
        getDataViewMemory0().setUint32(ptr + 4 * i, add, true);
    }
    WASM_VECTOR_LEN = array.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
function decodeText(ptr, len) {
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

const wasmPath = `${__dirname}/fixture_bg.wasm`;
const wasmBytes = require('fs').readFileSync(wasmPath);
const wasmModule = new WebAssembly.Module(wasmBytes);
let wasmInstance = new WebAssembly.Instance(wasmModule, __wbg_get_imports());
let wasm = wasmInstance.exports;
wasm.__wbindgen_start();
