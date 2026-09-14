# Enhancement Audit — XML Writer Literal Fidelity

## Basis examined

I read the supplied plan and `.goals`, then inspected the XML 1.0 `Char`
production in `crates/iri/src/terminals.rs`, the codec egress paths in
`crates/rdf/src/native_codecs/{rdfxml,trix}.rs`, the codec append contract in
`native_codecs/codec.rs`, existing XML egress consumers (`projections/util.rs`,
`viz/svg.rs`, and SPARQL Results XML), and the present XML-character tests.
The worktree already contains `purrdf_core::xml_escape` with `Context`,
`InvalidXmlChar`, `escape` (zero-copy clean path) and atomic `push`; both codecs,
the projections, and SVG use it. This makes several proposed plan locations and
the claimed absence of a dedicated module stale rather than merely stylistic.

## Findings

1. **TRANSFORMATION — make XML egress a specified, lossless contextual encoding boundary, not a literal-only patch.** **Cost: M; payoff: decisive.**
   The underlying problem is an injective transducer from strings in the XML 1.0
   `Char` value space to lexical XML in a particular context, such that parsing
   recovers the exact original scalar sequence. XML's end-of-line and attribute
   value normalization mean that syntactic well-formedness is insufficient: raw
   CR in character data, and raw tab/LF/CR in an attribute, are distinct source
   strings that collapse in the parsed infoset. The definitive contract is
   `parse_xml_context(encode_xml(value, context)) == value` for every XML `Char`,
   and a hard error for every Unicode scalar outside `Char`; escaping markup is
   the separate requirement that encoded output remains lexically valid.

   Codify that contract in the shared API documentation and test it as a
   context-by-context encoder law, then make all hand-written XML emitters use
   it. This subsumes the next predictable bugs in SVG, projections, SPARQL
   results, XML literal markup, namespace/IRI attributes, and a future streaming
   sink rather than rediscovering XML normalization per format. The current
   `xml_escape.rs` is already the right architectural seed: its `Context`,
   `Cow` clean path, `push`, and use of the sole `is_xml_char` production embody
   this structure. The plan's proposed placement in `iri_escape.rs` is a
   category error: XML lexical egress is not IRIREF escaping and that module
   would turn a reusable grammar boundary into a misleading grab bag.

2. **TRANSFORMATION — correct the domain from “bad C0 controls” to the complete XML 1.0 `Char` complement.** **Cost: S; payoff: high.**
   The plan explicitly declines U+FFFE/U+FFFF while saying the shared escape
   validates the XML `Char` production. Those statements cannot both be true.
   XML 1.0 Fifth Edition `[2]` accepts `#x9 | #xA | #xD | #x20-#xD7FF |
   #xE000-#xFFFD | #x10000-#x10FFFF`; Rust already rules out surrogates, but
   U+FFFE and U+FFFF remain constructible `char`s and cannot be represented by
   either raw text or a character reference. Permitting them makes output
   non-well-formed—the very defect being fixed. The existing canonical predicate
   and test correctly refuse both. Amend the plan to require *all* excluded
   scalars, use the shared predicate as the sole oracle, and remove the decline.
   Confidence: high.

3. **LEVERAGE — retain and extend the existing `purrdf_core::xml_escape`, rather than adding the planned functions to `iri_escape.rs`.** **Cost: S; payoff: high.**
   `xml_escape.rs` already avoids allocations with `Cow` where no substitution
   is needed, has `push` for callers that already own an output buffer, reports
   the first invalid scalar plus byte offset, and rolls back `push` output on an
   error. `rdfxml.rs` and `trix.rs` already map its error to their established
   `RdfDiagnostic`; `projections/util.rs` maps the same failure to
   `ProjectionError`; `svg.rs` uses the append form. A plan that moves the
   implementation into `iri_escape.rs` risks duplication, churning established
   callers, and losing this general surface. The task should instead inventory
   every XML writer and eliminate any residual local encoder by delegation to
   `xml_escape::{escape,push}` with `Context::{Text,Attribute}`.

4. **LEVERAGE — use the existing production round-trip and atomic-document seams.** **Cost: S; payoff: medium.**
   `rdfxml_predicate_names.rs` provides the existing write/read round-trip
   precedent and `proptest_roundtrip.rs` already exercises RDF/XML. More
   importantly, both XML codecs build a complete document via
   `serialize_ser_graph_to_{rdfxml,trix}` and only then append it in
   `RdfCodec::serialize_into`. This is the mechanism that fulfils “no partial
   output” at the public serialization boundary, even when an error occurs after
   a prefix was assembled. State it and test `serialize_into` with a nonempty
   caller buffer, because merely testing an error returned by
   `serialize_dataset_to_format` does not prove its append contract.

5. **UTILITY — broaden the acceptance matrix from lexical literals to every dynamic XML position, with explicit ownership.** **Cost: M; payoff: high.**
   RDF/XML uses text and attributes not only for ordinary literal lexical forms,
   but also for `rdf:parseType="Literal"` XML-literal node text, XML-literal
   attributes, namespace declarations, `xml:base`, language, datatype, and
   resource values. TriX emits term values in text and language/datatype in
   attributes. A correct shared boundary protects all these positions, but the
   plan's test wording concentrates on literal text and a vague “attributes”.
   Add a table mapping every writer call site to its `Context`, and test at least
   the RDF/XML XML-literal attribute path end-to-end; the current
   `xml_literal_canonicalization_preserves_attribute_whitespace` is a useful
   model. This catches the next normalization loss without inventing an optional
   per-writer policy.

6. **ROBUSTNESS — test the encoder law exhaustively at scalar boundaries and assert exact lexical obligations where they matter.** **Cost: M; payoff: high.**
   The listed neighbour examples are good regression cases but are not an
   adequate proof of the production. Add a compact exhaustive unit/property
   sweep over all Unicode scalar values (and short strings combining each
   normalization-sensitive character with markup) against the independent XML
   reader: accepted scalars parse back exactly in both contexts; rejected ones
   return `InvalidXmlChar` with their byte offset; text encodes CR as `&#xD;`;
   attributes encode tab/LF/CR as `&#x9;`, `&#xA;`, `&#xD;`. Include boundaries
   `U+0008/U+0009`, `U+000A/U+000B/U+000C/U+000D/U+000E/U+001F/U+0020`,
   `U+D7FF/U+E000`, `U+FFFD/U+FFFE/U+FFFF`, and `U+10000/U+10FFFF`.
   Integration tests should continue to use the production writer and reader
   and compare datasets (isomorphism when blank identity can differ), while the
   lower-level law test establishes the exact escaping cause. This is portable,
   deterministic, and far stronger than checking only absence of a raw CR.

7. **ROBUSTNESS — make error atomicity and diagnostics contractual, not “if feasible”.** **Cost: S; payoff: medium.**
   A byte offset is already cheap and implemented by `char_indices`; it should
   be required, not conditional. Require the diagnostic category to remain the
   writer's normal serialization error while including Unicode scalar and offset,
   then test the first invalid scalar is reported in a multibyte prefix. For
   `push`, test pre-existing output is byte-for-byte unchanged after an error;
   for codecs, test the caller's pre-existing buffer is unchanged. This prevents
   a future streaming refactor from exposing a partial XML prologue despite the
   current whole-document construction.

8. **ROBUSTNESS — replace the proposed verification command list with the repository gate and a precise artifact audit.** **Cost: S; payoff: medium.**
   Task 5 promises `make check` but does not run it, listing fragments that omit
   repository hygiene and profile checks; the plan should name `make check` as
   the authoritative final command (plus any focused codec/test command for
   iteration). Task 4 must distinguish generated artifacts, ordinary checked-in
   goldens, W3C conformance rows, and immutable GTS vectors. It must not
   regenerate frozen GTS vectors at all; if output bytes change, enumerate the
   affected non-frozen artifact paths and before/after hashes, explain the XML
   character-reference change, and run `make metadata` only if generated
   projections are touched. “Move every frozen vector” is misleading under the
   repository rule that frozen GTS vectors are never regenerated.

## Considered but not flagged

- XML 1.1 support is correctly excluded: the code and `is_xml_char` contract
  target XML 1.0, and adding a selectable XML version would violate the
  low-optionality/hard-fail direction without satisfying this issue.
- Escaping apostrophes in the current double-quoted attribute context is neither
  necessary nor beneficial; it would change deterministic bytes without adding
  fidelity. `>` may be escaped conservatively as the present encoder does; this
  is safe, although it is not the source of the normalization defect.
- I did not recommend an XML dependency or a second `Char` table. The existing
  pure-Rust parser and canonical terminal predicate preserve wasm portability,
  performance, and the terminal ring-fence.

## Rejected enhancements

- **REJECTED — offer XML 1.0/1.1 mode selection.** It conflicts with `.goals`'
  low/no optionality and with the project's fixed RDF/XML/XML 1.0 egress
  contract; a mode would create two incompatible fidelity domains.
