Task 3 is independently reviewed, committed and pushed as signed `31498279c83c7fb9c3c6d97faffc1628a82452e1`; normal hooks passed and remote branch readback matches.

The native combiner implements the pinned ML-DSA-65/Ed25519 construction, with both components mandatory. One strict detached COSE Sign1 parser now honors the protected algorithm, preserves exact protected bytes, refuses malformed/ambiguous envelopes before key lookup, and supports optional opaque key IDs. Existing EdDSA convenience APIs use that same core and retain the frozen output bytes. Composite `-58` remains the pinned draft's provisional request, with no final registration or shared GTS interoperability claim.

Independent extraction matched all eight published IETF fixture fields, including all 3373 signature bytes. Review found malformed textual content-type acceptance; the shared validator was corrected to RFC 9052/RFC 6838 grammar. The independent original public reproducer now rejects every malformed neighbor before lookup. The initial failed review and its correction evidence are preserved.

Qualification: 263 affected GTS cases including nine doctests; eleven public groups execute successfully in native and wasm/Node, covering external answers, both-component attacks, optional/binary IDs, and 20 valid plus 162 malformed content-type cases. GTS all-target clippy with warnings denied, formatting, helper census, declaration gates, all new-file whitespace and exact-source readback pass. RDF compilation and separate wasm library evidence remain applicable across the parser-only correction; the report distinguishes those earlier checks from current execution.

Final reviewed manifest SHA256: `54d8e8204b39b971126d9398f0a3d216175edc1b9ee0ca309356af01ce67c0a6`. Complete Task 3 patch SHA256: `cccbf183ac6de2283855cf38065511e904743fdbec92ea42bda836e5f6f75022`.

The next approved Task 4 integrates the actual Writer's fallible hedging provider and algorithm-aware key resolution. Task 5 covers compaction/RDF certification; Task 6 and Stages 2–3 cover final qualification, PR and merge. Whole-issue completion, hosted CI, hardware timing and formal certification are not claimed by this checkpoint.
