<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Diagnostic Code Reference

Every failure PurRDF reports carries a stable, machine-readable `code` on its
`RdfDiagnostic` (`severity`, `code`, `message`, `detail`, `location`). The
code is the contract: tests, the SARIF emitter, the CLI, the Python
`ValueError` text and downstream matchers key on it, while the `message` is
free prose that may change wording. This page lists the codes by family, with
the failure each one names and what a caller can do about it. The set was
enumerated from the constructor sites in the source tree, not written from
memory; if a code you see is missing here, the source is authoritative and the
page is stale.

Codes are kebab-case, prefixed by the family that owns them. They are never
translated, and a caller should compare the whole string rather than a prefix.

## `iri-*` — IRI parsing and base resolution (`purrdf-iri`)

`IriError::diagnostic_code` is the single owner of these strings for the
whole workspace; every codec, SPARQL, ShEx and SHACL route an IRI failure
through it. The two base-related codes are distinct because their remedies
are: one is fixed by supplying a base, the other is not.

| Code | Meaning | Remedy |
| --- | --- | --- |
| `iri-empty` | The string is empty where a non-empty IRI/URI was required. | Supply a non-empty IRI. |
| `iri-missing-scheme` | The string has no scheme, so it cannot be an absolute IRI. | Write the IRI in absolute form, with a scheme. |
| `iri-bad-scheme` | The scheme is malformed (it must start with a letter and contain only letters, digits, `+`, `-` and `.`). | Correct the scheme. |
| `iri-bad-percent-encoding` | A `%` at the reported byte offset is not followed by two hexadecimal digits. | Percent-encode the `%` itself, or complete the escape. |
| `iri-disallowed-char` | A character the IRI grammar does not admit occurs at the reported byte offset. | Percent-encode or remove the character. |
| `iri-bad-authority` | The authority component (`//host:port`) is malformed. | Correct the host or port. |
| `iri-non-absolute-base` | The base IRI supplied for resolution is not itself absolute. | Supply a base IRI that has a scheme, e.g. `http://example.org/dir/`. |
| `iri-relative-no-base` | A relative IRI reference was met with no base in scope: no in-document directive, no caller-supplied base, no retrieval IRI. | Add a base to the document (`@base`/`BASE` in Turtle-family syntaxes, `xml:base` in RDF/XML, `@context.@base` in JSON-LD) or pass a base IRI to the API. |
| `iri-not-absolute-by-grammar` | The reference is not absolute and the syntax admits no relative reference at all (N-Triples, N-Quads), so no base could ever apply. | Write the IRI in absolute form; supplying a base will not help. |

## `native-codec-*` — the text and XML codecs (`purrdf-rdf`)

| Code | Meaning | Remedy |
| --- | --- | --- |
| `native-codec-parse` | The codec (Turtle family, RDF/XML, TriX, HexTuples) could not parse the input; the location names the line and column. Term nesting past the parser's depth limit is reported under this code too. | Fix the document at the reported position. |
| `native-codec-utf8` | The input bytes are not valid UTF-8. | Re-encode the document as UTF-8. |
| `native-codec-panic` | The codec panicked while parsing and the panic guard caught it. This is a defect in PurRDF, never in the input. | Report it with the input that triggered it. |
| `native-codec-read` | Reading the RDF source through the streaming reader failed with an IO error. | Check the source stream or file. |
| `native-codec-write` | Writing the serialized output failed with an IO error. | Check the destination. |
| `native-codec-serialize` | The target format cannot represent the dataset — for example a named graph in a single-graph format — or the writer failed. | Choose a format that carries the construct, or serialize through the loss-ledger lane. |
| `native-codec-replay` | Replaying a parsed dataset into an event sink failed because the sink returned an error. | The sink's own error is carried in the message. |
| `native-codec-unsupported-format` | The media type or format identifier names no codec. | Use one of the supported media types or format ids. |
| `native-codec-datatype-not-iri` | A literal's datatype term is not an IRI. | Give the literal an IRI datatype. |
| `native-codec-direction-without-language` | A base direction was given on a literal that has no language tag. | Add a language tag, or drop the direction. |
| `native-codec-invalid-direction` | A literal base direction other than `ltr` or `rtl` was given. | Use `ltr` or `rtl`. |
| `native-codec-iri-missing-value` | An IRI term event carried an empty value. | Supply the IRI. |
| `native-codec-missing-reifier-binding` | A triple term refers to a reifier that has no binding. | Bind the reifier before referring to it. |
| `native-codec-predicate-not-iri` | A term in an IRI-only position (the predicate) is not an IRI. | Use an IRI predicate. |
| `native-codec-reifier-not-triple` | A reifier binds something other than a triple term. | Bind the reifier to a triple term. |
| `native-codec-term-out-of-range` | A term id in the event stream is outside the range the stream introduced. | The producing stream is inconsistent; regenerate it. |
| `native-codec-unbound-triple-term` | A triple term names neither its components nor a reifier. | Give the triple term its components or a reifier. |

The last nine arise when the codec lane consumes term events rather than
text — a GTS graph being resolved through the codec surface — and mirror the
`gts-*` and `rdf-ir-*` codes below.

## `native-jsonld-*` and `jsonld-*` — JSON-LD and YAML-LD

| Code | Meaning | Remedy |
| --- | --- | --- |
| `native-jsonld-decode` | The JSON-LD or YAML-LD input is malformed at the surface: invalid JSON/YAML, an encoding error, or a materialized-carrier byte budget exceeded. | Fix the surface syntax or reduce the document. |
| `native-jsonld-parse` | The input is well-formed JSON-LD that does not map to RDF. | Correct the JSON-LD structure. |
| `jsonld-json-input` | The strict JSON reader rejected the input, for example a duplicate object member. | Remove the duplicate or correct the JSON. |
| `jsonld-context-invalid` | A context document, or the strict versioned options document, is invalid. | Correct the context or options document. |
| `jsonld-context-limit` | A context-processing ceiling was exceeded: loaded bytes, work count, definition complexity, or the offline context registry size. | Reduce the context, or raise the limit the options document declares. |
| `jsonld-derived-invalid` | The deterministic dataset-IRI `derived` mode could not derive a prefix (an invalid IRI, or a null mapping). | Correct the IRI the prefix would be derived from. |
| `jsonld-derived-limit` | A derived-context work or byte ceiling was exceeded. | Reduce the dataset's IRI vocabulary or raise the declared limit. |
| `jsonld-options-unused` | JSON-LD serialization options were supplied for a format that is not JSON-LD or YAML-LD. | Drop the options, or serialize to JSON-LD/YAML-LD. |

## `cdt-*` — SEP-0009 composite literals

A `cdt:List` or `cdt:Map` lexical form denotes blank nodes in the enclosing
document's scope, so a form that does not parse leaves that scope undefined and
the whole document is refused rather than the literal being kept opaque.

| Code | Meaning | Remedy |
| --- | --- | --- |
| `cdt-literal-malformed` | A composite literal's lexical form does not parse. | Correct the literal. |
| `cdt-literal-scan-disagreement` | The bounded lexical scanner and the full parser disagree about the literal. | This is a defect in PurRDF; report it with the literal. |

## `rdf-ir-*` — dataset structure (`purrdf-core` freeze and GTS import)

`RdfDatasetBuilder::freeze()` validates the structure of the dataset it is
about to freeze; the GTS import sink reports the same family for a container
whose terms do not form a well-formed dataset.

| Code | Meaning | Remedy |
| --- | --- | --- |
| `rdf-ir-term-out-of-range` | A quad references a `TermId` the builder never interned. | Intern the term before pushing the quad. |
| `rdf-ir-predicate-not-iri` | A quad's predicate is not an IRI. | Use an IRI predicate. |
| `rdf-ir-literal-subject` | A literal occupies subject position. | RDF admits no literal subject; restructure the statement. |
| `rdf-ir-triple-subject` | A triple term occupies subject position. | RDF 1.2 admits triple terms in object position only; use a reifier. |
| `rdf-ir-graph-name-invalid` | A graph name is a literal or a triple term. | A graph name must be an IRI or a blank node. |
| `rdf-ir-reifier-not-triple` | A reifier binding points at something other than a triple term. | Bind the reifier to a triple term. |
| `rdf-ir-triple-cycle` | A triple term contains itself, directly or through nesting. | Remove the cycle. |
| `rdf-ir-triple-nesting-limit` | Triple-term nesting exceeds the builder's depth limit. | Flatten the nesting. |
| `rdf-ir-dangling-term-ref` | A GTS role references a term id that no term event introduced. | The container is inconsistent; regenerate it. |
| `rdf-ir-gts-fold-diagnostic` | The GTS fold reported a diagnostic, surfaced through the import. | The fold diagnostic's own code and detail are in the message. |
| `rdf-ir-iri-missing-value` | An imported IRI term has an empty value. | Supply the IRI. |
| `rdf-ir-literal-datatype-not-iri` | An imported literal's datatype resolves to a non-IRI. | Give the literal an IRI datatype. |
| `rdf-ir-missing-reifier-binding` | An imported triple term references a reifier with no recorded binding. | Bind the reifier in the container. |
| `rdf-ir-term-nesting-limit` | Imported triple-term nesting exceeds the depth limit. | Flatten the nesting. |
| `rdf-ir-unbound-triple-term` | An imported triple term names neither its components nor a reifier. | Give the triple term its components or a reifier. |

## `gts-*` and `rdf-*` — GTS graph resolution, verification and writing

| Code | Meaning | Remedy |
| --- | --- | --- |
| `gts-term-out-of-range` | A GTS term id is out of range for the graph. | The container is inconsistent; regenerate it. |
| `gts-iri-missing-value` | A GTS IRI term has an empty value. | Supply the IRI. |
| `gts-predicate-not-iri` | A GTS predicate term is not an IRI. | Use an IRI predicate. |
| `gts-literal-datatype-not-iri` | A GTS literal datatype does not resolve to an IRI. | Give the literal an IRI datatype. |
| `gts-direction-without-language` | A GTS literal carries a base direction but no language tag. | Add a language tag, or drop the direction. |
| `gts-invalid-direction` | A GTS literal base direction is neither `ltr` nor `rtl`. | Use `ltr` or `rtl`. |
| `gts-missing-reifier-binding` | A GTS triple term references a reifier the graph does not bind. | Bind the reifier in the container. |
| `gts-unbound-triple-term` | A GTS triple term names neither its own components nor a reifier. | Give the triple term its components or a reifier. |
| `gts-self-reaching-term` | A GTS term resolves through itself, so no walk of its components can terminate. | Remove the cycle. |
| `gts-term-nesting-limit` | GTS term nesting exceeds the depth limit. | Flatten the nesting. |
| `gts-fold-diagnostic` | The GTS fold reported one or more diagnostics. | Inspect the fold diagnostics listed in the detail. |
| `gts-verify-digest-inclusion` | Content-addressed terms are not included in the verified chain. | The container's chain does not cover its content; do not trust it. |
| `gts-verify-signature` | COSE signature verification failed. | Check the signing key and the container's integrity. |
| `gts-writer-codec` | The GTS writer's codec reported an error while writing. | The codec's own error is in the message. |
| `rdf-graph-name-not-node` | While building a GTS graph, a named-graph name is not an IRI or blank node. | Use an IRI or blank node graph name. |
| `rdf-reifier-not-node` | While building a GTS graph, an RDF 1.2 reifier is not an IRI or blank node. | Use an IRI or blank node reifier. |
| `rdf-term-nesting-limit` | RDF term nesting exceeded the depth limit while building a GTS graph. | Flatten the nesting. |

## `native-sparql-*` — the SPARQL engine boundary (`purrdf-sparql-eval`)

| Code | Meaning | Remedy |
| --- | --- | --- |
| `native-sparql-query-parse` | The query text does not parse under the SPARQL 1.1/1.2 grammar (including the enforced `VERSION` declaration). | Fix the query at the reported position. |
| `native-sparql-update-parse` | The update request does not parse. | Fix the update at the reported position. |
| `native-sparql-query-explain` | Evaluation under `--explain` failed; the evaluator's error is in the message. | Address the underlying evaluation error. |
| `native-sparql-property-function` | The property-function seam refused the query: a predicate under a declared namespace has no registration, a call site's arity does not match the relation, no total order can serve a chain, or a prepared plan is being evaluated under a different registry than it was prepared with. | Register the relation, correct the arity, or prepare and evaluate under the same registry. |
| `native-sparql-aggregate-function` | The custom-aggregate seam refused the query: an `AGG(<iri>, …)` names no registered aggregate, or a prepared plan is being evaluated under a different aggregate registry. | Register the aggregate, or prepare and evaluate under the same registry. |
| `native-sparql-custom-function` | A function or aggregate IRI resolved to no registered custom function, native function, or XSD constructor. | Register the function under that IRI, or use a native one. |
| `native-sparql-quoted-triple-term-variable` | A variable occupies a component of a quoted-triple term in a basic graph pattern or property path; structural triple-term matching is out of scope. | Bind the triple term as a whole, or match its components through the reifier surface. |
| `native-sparql-heldin-unconfigured` | `heldIn` was called with no caller-supplied standpoint-predicate configuration. | Configure the standpoint predicates before using `heldIn`. |
| `native-sparql-graph-pattern-depth-exceeded` | A manually constructed graph pattern nests deeper than the parser's safety bound. | Flatten the pattern. |
| `native-sparql-bnode-mint-prefix` | The blank-node mint prefix supplied in the options is invalid. | Supply a valid prefix. |
| `native-sparql-load-no-resolver` | `LOAD <iri>` was requested but no `GraphResolver` host seam was provided. | Inject a resolver, or remove the `LOAD`. |
| `native-sparql-update-bad-destination` | An `ADD`/`MOVE`/`COPY`/`LOAD` destination is `NAMED` or `ALL`; it must be `DEFAULT` or a single named `GRAPH`. | Name a single destination graph. |
| `native-sparql-subst-iri` | A substitution value is not a valid IRI. | Supply a valid IRI. |
| `native-sparql-subst-triple-predicate` | A substituted quoted triple has a predicate that is not an IRI. | Use an IRI predicate. |

## `reasoning-*` — SPARQL under an entailment regime (`purrdf`)

| Code | Meaning | Remedy |
| --- | --- | --- |
| `reasoning-closure-relation-witness` | Property-function relations derived from the closure cannot be combined with an OWL Direct-Semantics run whose restricted chase minted existential witnesses: a relation walking the closure could return a minted blank node the regime's scoping graph does not contain. | Query under a regime that mints no witnesses (`rdf`, `rdfs`, `owl-rl`, `d`, `rif`, `simple`), or drop the dataset-derived relations from the call. |
| `reasoning-closure-relation-rebuild` | Rebuilding the property-function relations over the closure failed. | The relation builder's own error is in the message. |

## `statements-*` — statement-metadata ingestion (`purrdf-rdf`)

These arise when a document's `rdf:reifies` and `owl:Axiom` statements are
read into the statement layer.

| Code | Meaning | Remedy |
| --- | --- | --- |
| `statements-turtle-parse` | The statement-metadata Turtle failed to parse. | Fix the Turtle. |
| `statements-non-iri` | A term that must be an IRI in this context is not one. | Use an IRI. |
| `statements-reifies-non-triple` | The object of `rdf:reifies` is not a triple term. | Reify a triple term. |
| `statements-malformed-axiom` | An `owl:Axiom` lacks its source, property or target. | Complete the axiom. |
| `statements-conflicting-structural` | One subject carries two different values for a structural field. | Keep one value. |

## Other single-code families

| Code | Meaning | Remedy |
| --- | --- | --- |
| `sssom-tsv-parse` | An SSSOM TSV document is malformed: a missing or unreadable header row, a malformed `curie_map` entry or set comment, a malformed row, or a non-numeric confidence; the location names the line. | Fix the TSV at the reported line. |
| `content-id-scheme` | A content-id scheme prefix is invalid: empty, non-ASCII, or ending in a hexadecimal digit (which would make it ambiguous with the 64-hex-character tail). | Choose a prefix that is non-empty ASCII and does not end in `0-9`, `a-f` or `A-F`. |

## Where the code reaches you

- **Rust** — `RdfDiagnostic::code` on the returned error; its `Display`
  form is `<severity> <code>: <message>`.
- **Python** — the `ValueError` message is that `Display` form (for example
  `error native-codec-parse: …`); an IRI failure is rendered as
  `<code>: <message>` (for example `iri-relative-no-base: …`).
- **JavaScript** — the thrown `Error` message carries the same text.
- **C** — the error string returned through the C ABI carries the same text.
- **SARIF** — the `ruleId` of each result is the code, with a
  `reportingDescriptor` in the run's rule table
  ([`purrdf-validate`](https://docs.rs/purrdf-validate)).
