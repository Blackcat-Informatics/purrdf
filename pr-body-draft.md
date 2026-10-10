Opaque blobs and MIME messages now share the existing exact byte-cover law.
The core emitter preserves original octets, declared continuations and
digest-bound references; its delimited-record consumer retains empty records.
The MIME codec preserves ordered duplicate headers, nested messages and multipart
ranges in queryable RDF 1.2, with reproducible attachment bytes and typed defects
for malformed input. Decoding reconstructs, reparses and verifies the complete
selected projection before releasing bytes.

Profiles, vocabularies and resource bounds are supplied by callers. The parser
and owned model use iterative flat state. New runtime dependency edges are
first-party only, and the umbrella exposes the same implementation.

Validation already observed: original UTF-8 cover, JSON and Markdown controls;
native and actual WASM core byte-cover controls; native and actual WASM MIME
round-trip, graph refusal, caller-limit and original-octet controls. The frozen
six-message transcript exercises actual Turtle, N-Triples and JSON-LD bytes,
including two unchanged public CPython originals and a15-part digest.

Strict affected lint, native16/actual WASM16 MIME acceptance, metadata and the
complete original local gate pass, including all32 release-WASM crates.
Independent full-contract and behavior-task review passes. Original rendered
translation gate and English/Chinese book builds pass with zero template drift.
Hosted acceptance and actual integration assessment remain pending.

Closes #521
Closes #520
