# Sentence-boundary source review

Static source verdict: PASS. Runtime acceptance: NOT MET.

Reviewed the complete #513 contract, the sentence iterator, shared borrowed cursor, sole Unicode generator changes, pinned source provenance, official corpus harness, allocation fixture and native/WASM registration. The implementation follows the specified default rule order, retains raw paragraph context separately from ignored characters, and uses a monotone SB8 decisive-offset cache. It returns slices into verbatim UTF-8 input, with byte offsets and the existing pinned alphanumeric filter for sentence_indices. Frozen official corpus whitespace remains verbatim.

The existing word iterator uses the same borrowed cursor; normalization, text-data identity and word-boundary behavior retain their existing owners. Sentence data has its own recorded digest and rule-law identifier. No locale, windowing policy, ontology namespace or runtime dependency was introduced.

Generated tables must be regenerated through gen_unicode_tables text before compilation. Required runtime evidence remains the complete official SentenceBreakTest corpus, original word/normalization corpus, borrowing and zero-allocation assertions, monotone lookahead regression, named actual WASM probe and the required repository gate. These checks have not run on the applied source. No issue is complete and no PR is qualified by this source review.
