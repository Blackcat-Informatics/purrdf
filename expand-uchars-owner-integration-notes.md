# Shared UCHAR expansion and UTF-8 append proposal

`expand-uchars-owner-draft.patch` is a standard three-home patch against the current shipping postimage. `patch --dry-run -p1` passes. No source was edited and no compiler, build, or test was run.

The lexical home exposes `expand_uchars_with_memory<'a, S: Admission + ?Sized>(text: &'a str, memory: &mut Memory<'_, S>) -> Result<Cow<'a, str>, StorageError>`. This moves the existing loop and `decode_uchar` call into the admitted body; the resident wrapper delegates to it. No backslash returns the original borrowed slice without allocation or admission. A backslash reserves exactly the original UTF-8 byte length before writing: decoded UCHAR source occupies six or ten ASCII bytes and produces at most four UTF-8 bytes, while malformed sequences preserve their bytes. This is a certified destination bound for this decoder, not a row or term multiplier.

`allocation::Resident` is the common zero-size callback. It accepts each exact live layout without adding a caller capacity preset; physical allocation and checked layouts remain fallible in `Memory`.

`Memory::push_str(&mut String, &str)` now owns the existing checked doubling and old-plus-new replacement law. Both `FormatSink::write_str` and `push_char` delegate to this body. Native CDT splice callers can append fragments amortized through `memory.push_str`; they must pass only buffers created or explicitly included in the same original memory grant.

For CDT binding scans, retain the original admission/grant while an owned Cow is live. A borrowed Cow requiring a retained owned string must be copied through `memory.string`; do not call unadmitted `Cow::into_owned`. A refused allocation remains a physical `StorageError`, separate from malformed UCHAR spelling, which the existing lexical semantics preserve verbatim.

Four proposed fixtures append to the existing core counting-allocator test home: real decoder peak/retained capacity and malformed-byte parity; admission refusal before allocation and allocation-free borrow; amortized UTF-8 append with actual old/new peak; and refusal preserving original pointer, text, and live capacity. These fixtures are uncompiled and unrun. The Stage assembler is historical evidence, not a command to rerun after integration.
