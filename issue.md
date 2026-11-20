# Issue #1397: v5.0: migrate SPARQL SERVICE and LOAD to the shared HTTP exchange

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Problem

The published `purrdf_sparql_eval::HttpTransport` requires `post(HttpRequest)`. Its request contains a SPARQL query and its result is a body. The JavaScript/Wasm SERVICE host implements it. #1292 requires retiring this public trait in favour of a method-aware exchange that also serves LOAD. Removing it breaks Rust host implementations.

## Required before 5.0

- Ship one caller-supplied exchange for the existing remote SERVICE and LOAD paths. Carry the method, target, ordered headers, body, response status and response headers. Migrate the Rust exports and actual host adapters off `HttpTransport` and its POST-only request. Keep catalog-backed graph resolution available.
- Preserve request admission, cancellation, typed host failures, SERVICE SILENT, and LOAD policy. Keep existing JavaScript host behaviour where the adapter can preserve it. Document the Rust migration and any host change that callers must make.
- Exercise the existing SERVICE and LOAD paths through the shared exchange with a real host adapter, including denied and neighbouring allowed requests. Build the owner for Wasm.

Solid client behaviour, Solid-specific headers, clock and entropy integration, and live CLI transport stay in #1292 and #1300.


## Comments (1)

### paudley — 2026-11-20T02:10:35Z

## Plan

**Surface that breaks.** `purrdf_sparql_eval::remote_http::HttpTransport` (`remote_http.rs:126`) has one method, `post(HttpRequest)`. It is replaced by a method-aware exchange: method, target, ordered headers, body in; status, response headers and body out. The one exchange serves SERVICE and LOAD. The POST-only `HttpRequest` is replaced too.

**Who changes.**
- Rust host implementers: implement the new exchange instead of `post`. The migration note gives the mapping from the old request fields.
- The wasm host (`crates/rdf-wasm/src/async_query.rs`) changes its adapter to the exchange, keeping its JavaScript-visible behaviour.
- The conformance harness (`crates/sparql-conformance/tests/governor_corpus.rs`) and the sparql-eval tests and workspace fixtures move to the exchange.
- Catalog-backed graph resolution stays as it is.

**Frozen identities.** None move. The transport carries bytes to and from a remote endpoint. It does not enter any digest, label, or canonical form.

**Verification.** The existing SERVICE and LOAD paths run through the exchange with a real host adapter, including a denied request and an adjacent allowed request, with SERVICE SILENT and the typed host failures preserved. Wasm build of the owner crate. Cancellation and request admission tests carried over unchanged.

**Open point.** The issue says the JavaScript host keeps its behaviour "where the adapter can preserve it". I need the owner to say which JavaScript behaviour is required to be unchanged, so the adapter is not changed silently.

**Lane.** Touches `sparql-eval/src/remote_http.rs`, `workspace.rs` and tests, `rdf-wasm`, and `sparql-conformance`. It shares `sparql-eval` with #1399, but not the same files, so the two can run as parallel lanes.


