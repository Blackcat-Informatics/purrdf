# Native composite blank-label admission integration

Status: implementation proposal; not qualified or complete.

Apply in order:
- retained-cdt-binding-owner-draft.patch
- retained-cdt-binding-growth-cleanup.patch
- helper's common allocation Resident/push_str and terminals expand_uchars_with_memory companion (before compilation).

Ordinary main-packet git apply --check passed against unchanged original core homes. Stage postimages passed rustfmt syntax. No shipping source edit or build was performed by the proposal author.

Native API:
- blank_label::encode_blank_label_with_memory(label, scope, alphabet, &mut Memory)
- blank_label::decode_blank_label_with_memory(token, alphabet, &mut Memory)
- BlankBinding::rebind_with_memory(token, &mut Memory)
- cdt_blank::bind_cdt_blank_labels_unchecked_with_memory(lexical, datatype, binding, &mut Memory)

Encoding uses one allocation-free envelope Display through the original Memory. Decoding keeps canonical scope, uppercase six-digit scalar syntax and byte-exact alphabet image check. Malformed/image-mismatched tokens retain original borrowed identity; physical storage refusal propagates separately. Image scratch dies before publication; only a successful decoded label survives.

Scanner and splicer keep their original native recognition bodies, datatype guard, embedded-literal handling, root-offset mapping, token ordering/overlap removal and byte-preserving output. Every actual region/queue/span/string/map growth is admitted before its fallible factory. Removed span payloads are destroyed before their admission is released. Construction scratch dies before returning the output. The existing unchecked malformed-text behavior and MAX_ELEMENTS scan budget are preserved, not replaced with a new supported subset. Shared Memory::push_str avoids quadratic exact reallocation per fragment.

An owned Cow remains included in the caller's original Memory/lexical frame. Ground conversion must transfer that original grant through the final literal facets/control owner; it must not drop the frame then measure/re-admit already allocated output. Borrowed lexical output needs original admission before its final copy. On physical error the computation must stop and discard its original frame after scratch destruction; the memory total is not a recovery certificate.

Five new core cdt_blank_storage tests measure real allocator behavior:
1. nested literals/escaped datatypes/Unicode spans, ambient and decoded binding parity, peak and exact surviving destination, release;
2. plain/non-composite/malformed lexical borrowed behavior and no surviving scratch;
3. physical refusals at initial and later actual growth thresholds;
4. direct ground-label decoding peak, original surviving label and typed image/rendering refusal;
5. malformed/non-image envelopes retain identity and release all scratch.

All five are UNRUN. Existing core/cdt blank-label tests must qualify resident spelling preservation; existing public ground/VALUES/path/BGP/expression matrix must qualify production wiring and original grant lifetime. Full make check remains 0/3; no commit/PR/completion.

Separate SharedText review: inspected concrete Shared<TextPayload<T>> control layout, whole ordered payload capture on allocation refusal, private matching borrow/clone/drop vtable, checked atomic cloning, header-deallocation-before-payload law and constructor Send+Sync bound. No source ownership/safety defect found; source review alone does not prove required runtime allocator/null/lifetime tests.
