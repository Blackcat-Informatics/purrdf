# Enhancement audit — XML writer character fidelity

## Basis and method

I read the supplied plan and `.goals`, then examined the XML `Char` authority in
`crates/iri/src/terminals.rs`, the RDF/XML and TriX serializers and their parser/
serialization seam, existing XML projections/SVG output, and SPARQL Results XML.
The worktree already contains an uncommitted `purrdf_core::xml_escape` authority
and migrations of RDF/XML, TriX, projections, SVG, and SRX to it. That is useful
evidence of the correct leverage and of surfaces the plan currently fails to name;
it is not treated as a substitute for making the plan complete.

The central criterion is not merely "well-formed XML": each XML egress context
must be an injective encoding with respect to the receiving XML parser's
normalization. For an XML context `c`, the desired law is
`read_c(escape_c(s)) = s` for every XML-1.0-representable Unicode scalar string,
and a named error for every other input. Raw CR violates that law in text; raw tab,
LF, and CR violate it in attributes. XML `Char` is the representability domain,
not just a C0 special case.

## Findings

1. **[TRANSFORMATION] Make XML egress a single, total contract over all XML-producing surfaces, rather than an RDF/XML/TriX literal patch.** **Cost: M. Payoff: decisive.**
   The feature is a *normalization-aware, partial serialization isomorphism*:
   context-indexed escaping plus a single representability predicate. The plan
   recognizes `Text` versus double-quoted `Attribute`, but limits the shared
   authority and migration to two RDF codecs. The repository also has user-derived
   XML in GraphML/DataCite/RO-Crate projection utilities, SVG, and SPARQL Results
   XML. These are not cosmetic siblings: each can presently emit CR or an excluded
   scalar, and several consume the same literal/IRI-derived values. The definitive
   implementation should establish `purrdf_core::xml_escape::{Context, escape,
   push, InvalidXmlChar}` as the sole XML 1.0 character-value boundary, then audit
   every XML output call site for a context-specific delegation. It should state
   the atomicity contract at public egress APIs: an error returns no artifact,
   even if a lower layer accumulated a private buffer. This subsumes the next
   likely issues (SVG, SPARQL XML, and projection corruption) at marginal cost and
   satisfies .goals' one-path/maximal-utility requirements.

2. **[TRANSFORMATION] Specify and test the complete XML 1.0 Fifth Edition domain, not the issue's illustrative C0 subset.** **Cost: M. Payoff: high.**
   The plan explicitly declines U+FFFE/U+FFFF despite also directing use of
   `is_xml_char`. This is internally contradictory: XML `Char` excludes those two
   noncharacters just as definitively as it excludes U+0000, and a raw writer
   would again claim success while producing unreadable XML. The existing terminal
   authority correctly defines `[ #x9-#xA | #xD-#xD | #x20-#x7F |
   #x80-#xD7FF | #xE000-#xFFFD | #x10000-#x10FFFF ]`; Rust has no surrogate
   `char`, so this is an exact, finite range decision. Amend R3 to refuse *every*
   scalar outside that predicate, including U+FFFE/U+FFFF, while deliberately
   accepting C1 controls and supplementary-plane noncharacters. Add boundary and
   exhaustive-range/property tests at the shared primitive, plus writer→reader
   tests for representative boundary values. This prevents future hand-copied
   ranges and validates both sides of every hole.

3. **[LEVERAGE] Use the already-existing semantic home `purrdf_core::xml_escape`, not `iri_escape.rs`, and expose a context enum rather than paired ambiguous APIs.** **Cost: S. Payoff: high.**
   `iri_escape.rs` is explicitly the transcription of the RDF `IRIREF` writer
   terminal; XML character-data escaping is governed by XML productions and has a
   distinct error/normalization law. Conflating them hides the abstraction and
   creates a misleading dependency between IRI lexical escaping and XML output.
   The worktree's `xml_escape.rs` already uses `Context::{Text, Attribute}`, returns
   borrowed output when safe, and has an append form that rolls back on error.
   Make that the planned module and API. Thin format-local functions may only map
   `InvalidXmlChar` into the format's diagnostic type. Keep a `push` variant for
   hot buffer-building writers to avoid an intermediate `Cow`/allocation, and an
   `escape` variant for convenient callers. This is portable pure Rust and preserves
   the common no-escape fast path.

4. **[LEVERAGE] Expand the audit to embedded XML literals and all actual attribute-bearing writer paths.** **Cost: M. Payoff: high.**
   RDF/XML has two distinct XML-producing paths: ordinary RDF/XML serialization
   and `rdf:parseType="Literal"` canonicalization (`serialize_children_as_xml`),
   whose text, namespace URI, and arbitrary embedded element attributes need the
   same lossless rules. TriX has text nodes plus constrained `xml:lang`/datatype
   attributes; it cannot by itself demonstrate arbitrary attribute whitespace.
   The plan's R6 says tab/LF/CR attributes round-trip but does not identify a
   production path capable of carrying them. Add a constructed RDF/XML XMLLiteral
   fixture containing an embedded attribute and text content, and use the actual
   relevant reader for each producer. This makes the claimed coverage executable
   rather than relying on an impossible ordinary `xml:lang` example.

5. **[UTILITY] Turn the round-trip suite into a reusable XML egress conformance matrix.** **Cost: M. Payoff: high.**
   Parameterize producers (RDF/XML, TriX, SRX, projection helpers where a reader
   exists) over contexts and scalar classes: mandatory markup characters; text CR;
   attribute tab/LF/CR; valid range endpoints; C0 holes; FFFE/FFFF; and a non-BMP
   scalar. Assert (a) XML parser well-formedness, (b) exact semantic round-trip
   through the production reader where one exists, (c) the required numeric
   reference spelling for normalization-sensitive characters, and (d) diagnostic
   scalar and byte offset on refusal. A small table-driven harness prevents each
   future XML emitter from relearning the same XML 1.0 rules. Graph isomorphism is
   necessary for blank nodes, but for fixed input literals compare canonical
   N-Triples bytes/term values too, so an isomorphic graph cannot mask lexical
   corruption.

6. **[ROBUSTNESS] Correct the plan's verification command and make the frozen-artifact audit evidence-based.** **Cost: S. Payoff: medium.**
   R8 claims `make check`, but Task 5 runs a hand-assembled subset and omits the
   repository's authoritative `make check` command (which includes hygiene and
   build-profile checks). Use `make check` as the final gate, with targeted crate
   tests during iteration. Before changing fixtures, identify whether any
   byte-exact RDF/XML/TriX golden actually contains a raw CR; list only the files
   that changed, their before/after hashes, and their semantic reason. Frozen GTS
   vectors must not be regenerated. This is stronger than a prospective claim to
   "run the full test suite" and avoids manufacturing vector churn merely because
   serializer code changed.

7. **[ROBUSTNESS] Preserve error provenance and atomic output explicitly at all call boundaries.** **Cost: S. Payoff: medium.**
   The plan says "add `InvalidXmlChar` in diagnostics (or appropriate location)"
   but leaves error ownership and conversion unspecified. Keep an error carrying
   the Unicode scalar and its byte offset in the shared XML module; format layers
   must convert it into their established serialize error family without dropping
   either datum. Test an invalid scalar after already-escaped content, and in every
   value position that can be user derived (literal lexical form, XMLLiteral child
   text/attribute, IRI/base/namespace attribute where representable by the IR).
   At the top-level `Result<Vec<u8>, _>` interfaces, assert error rather than a
   partial artifact; at append-level interfaces assert rollback. That makes the
   hard-failure promise observable without adding needless double buffering.

## Items deliberately not flagged

- **REJECTED — XML 1.1 support.** It conflicts with the stated RDF 1.2/XML 1.0
  contract and low-optionality rule; a second character law would weaken the one
  canonical egress path.
- Escaping apostrophes in the current double-quoted attribute context is neither
  necessary nor useful; XML permits it and changing deterministic bytes would add
  churn without improving the round-trip law.
- Rejecting U+007F, C1 controls, or supplementary-plane noncharacters is wrong
  for XML 1.0 Fifth Edition. `is_xml_char` deliberately accepts them, and the
  plan's proposed valid-neighbour tests correctly guard against over-refusal.
- A streaming serializer redesign is not required for this issue: current public
  APIs construct and return owned bytes only on success. The required enhancement
  is to document/test that boundary and make lower-level append rollback reliable,
  not introduce speculative I/O abstraction.

## Confidence

High for findings 1–4 and 6–7: they follow directly from the XML 1.0 production,
the supplied plan's contradiction, and the enumerated repository call sites.
Medium for the breadth of the reusable matrix because projection readers differ;
where no production reader exists, well-formedness and DOM-level infoset checks
are the appropriate oracle rather than inventing a parallel parser.
