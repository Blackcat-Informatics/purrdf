# Shared byte covers and faithful MIME

Complete scope:521 and520, in one isolated successor based on merged foundation
d608b0584. Both complete live bodies and comment surfaces are intake evidence;
there are no comments. This plan does not narrow either contract. Main and all
dirty donors are preserved. Required commits use normal hooks; final integration
uses ghprsq. Each issue counts only when its complete contract is merged+closed.

## Requirements and governing decisions

The core has one exact cover decoder, but its public span payload and return
value assume UTF-8. Generalize its existing coverage walk to arbitrary bytes,
retain the UTF-8 API as a caller of that same walk, and expose one emitter.
Identity covers include empty input and every octet. A reference binds a byte
range to original content identity, proves that identity before returning bytes,
and refuses malformed ranges and mismatching digests. The raw delimited-record
codec uses the actual emitter and preserves empty occurrence multiplicity.

The MIME codec must produce ordered, queryable RDF1.2 occurrences and reconstruct
all original bytes, including malformed input. Preserve duplicate/folded/8-bit
headers, part nesting, nested message/rfc822 content, boundaries, preambles,
epilogues, broken encodings and conflicting headers. Describe defects as typed
occurrences; never guess a boundary, charset or repair. Attachment decoding is
a deterministic function of covered bytes with typed decoding refusals. Explicit
profile and vocabulary; no compiled header/depth/line/size ceilings. Parsing and
owned destruction use flat iterative state, not recursive host-stack depth.

Read root AGENTS.md, .baseline and .goals. Cited ADRs are in sibling Katamari,
not missing decisions: docs/adr/0022-normalization-and-repair.md retains raw bytes
and makes repairs attributed additional representations; 0029-retrieval-coverage-
and-the-search-family-contract.md29.22 keys windows by literal digest/span/space;
0045-programs-are-sparql-rules-and-ops.md keeps content and judgments distinct;
0046-katamari-owns-its-derivations.md makes identities semantic and excludes
execution details. These motivate content-bound spans without adding Katamari
namespaces or a repair engine to PurRDF.

Primary wire specifications: RFC5322 and RFC2045–2049. RFC2046 assigns the CRLF
before a boundary to the delimiter; body range/attachment decoding must preserve
that distinction. Transfer decoding never normalizes decoded attachment bytes.
The XSD binary home already supplies strict base64; reuse its actual decoder
where the MIME lexical policy agrees instead of introducing another algorithm.

## Prior art and selection

stagectl prior-art.md records forge search and recurrence, including19 none,
1semantic-identity-loss and1performance trailer. Its unresolved ADR crawl is
resolved by the explicit sibling paths above. Existing JSON/Markdown codecs prove
the ordered occurrence + reparse contract and IR production seams. Reuse those
laws and existing lexical/hash/IR homes, not their private parsers or bounded
profiles. New runtime dependency edges are first-party only; no new external
package, semantic Cargo feature, alternate cover decoder or normalized fallback.

## Task 1: One arbitrary-byte cover law and emit/reference APIs

Generalize the original cover walk once. Implement ByteSpan, ByteCover identity,
CoverBuilder declarations/finalization, digest-bound SpanReference and the raw
DelimitedCover production consumer. Preserve all legacy UTF-8 refusal vectors.
Acceptance: every-octet and empty identities, borrowed original payload, lawful
continuations beside gaps/self/undeclared overlaps, digest mismatch, out-of-range
and empty references, delimiter/empty record multiplicity, frozen SHA256 and
input-order independence. Native and real WASM execution use one testkit target.

## Task 2: Complete iterative MIME model and deterministic transfer decoding

Add purrdf-mime with caller-selected vocabulary/profile/bounds, flat part/header/
structure/defect occurrences, complete iterative message+multipart traversal and
canonical reproducible attachment decoding. Preserve source ranges and original
physical line endings. Duplicate/conflicting structural headers produce explicit
ambiguity rather than an inferred first-header structure. Missing/mismatched
boundary, unfinished multipart, broken transfer and malformed header conditions
remain covered and queryable. All payload spans emit through Task1's builder.

Acceptance: real and adversarial corpus includes folds/repeats, empty bodies,
every octet, bare LF, malformed fields, all transfer forms and refusals,
multipart nesting with distinct parent IDs, preamble/epilogue/delimiters,
message/rfc822, missing/ambiguous boundaries and repeated conflicting fields.
Scale beyond old JSON/Markdown depth, line, occurrence and size defaults proves
absence of hidden ceilings; caller-declared refusal neighbors remain intact.

## Task 3: RDF projection and exact graph decode/reparse

Project directly to shared IR, unique occurrence IDs retain ordered duplicates,
explicit document/source/profile/digest/offsets. Encode arbitrary bytes using the
existing canonical binary lexical home. Decode the selected document, reconstruct
through core, reparse and compare every asserted occurrence against canonical
production projection before releasing bytes. Unknown/conflicting/invented/
missing occurrences or metadata refuse. Other document graphs do not authorize
unverified fields in the selected document. Expose through umbrella mime module.

Acceptance: real SPARQL sees duplicate/header/part order; source reconstruction
through actual RDF serialization/parsing; adversarial graph mutations refuse;
decoded attachment identity repeats under different message filenames; exact
golden RDF bytes and content identities on native+WASM and repeated runs.

## Task 4: Grouped qualification, independent review and protected integration

Write complete behavior units before affected checks. One cheap complete hygiene
pass catches all ledger/layer/export/generated/document drift before the settled
required makecheck. Run meaningful affected native/portable and legacy codec
controls, then full required gate once; failures remain failures and are corrected
without exemptions/hook bypass. Generate projections through existing writers.
Independent intake/prior-art/implementation/contract review when a reviewer slot
is available; no self-review is represented as independent. Publish signed normal
hooked source and grouped PR closing both contracts, satisfy final-head CI/review
and actual candidate acceptance, merge only via ghprsq, verify issue closure and
archive/owned cleanup, then update root REWORK_LIST.md. No source/test/PR stage is
counted as completion. Runtime, independent review, full gate and merge are NOT MET.
