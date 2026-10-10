# Native temporal parser proposal

`temporal-parser-draft.patch` is a standard unified proposal against the current
native temporal home. `git apply --check` passes. Shipping source is untouched;
no compiler, tests or full gate ran. Its one parser core replaces the old core;
resident public APIs wrap it, rather than retaining a second grammar.

It covers dateTime, date, time, all three duration types and all five Gregorian
types. `read_temporal(lexical, datatype)` returns None for another native parser's
datatype, or a supported Result with an inline XsdValue / static classification.
Timezone splitting borrows its body. Significant seconds retry borrows the
trailing-zero-normalized slice and uses the numeric helper's final
`read_decimal`/`NumericReadError`, including its original precision semantics.
Deferred year, numeral and seconds range failures hold no owned lexical text;
later lexical validation still outranks them. Component arithmetic is checked.

Public parse_datetime/date/time/duration/gregorian retain the original XsdError
datatype, lexical and reason by materializing only at the resident boundary.
Duration::new already constructs only range errors with an empty String; that
error path has no heap allocation and is converted explicitly, preserving its
empty lexical. A violated constructor/error invariant hard-fails. The existing
private two-digit-field test materializes its static error so its historical
owned diagnostic assertions retain their meaning.

Connect all supported temporal cases to the same ParsedValue dispatcher used
for numeric, Boolean, String and binary parsing. Successful temporal values own
no heap and use the inline parsed carrier; invalid/native range classifications
retain the old value-space None. None from read_temporal is a dispatch obligation,
never an unsupported refusal or an unpriced resident parse fallback. All actual
cache, cast, comparison and aggregate caller paths still require migration.

`temporal-parser-regressions.rs` is proposed for the existing exact_governance
allocator test crate. It asserts zero native allocation for every temporal
datatype, accepted timezone bodies, malformed inputs, 8192-digit years and
fractions, significant-scale refusals, and later lexical failures overriding
deferred range errors. Authored Strings are constructed before measurement;
canonical rendering and assertions happen after it. Expected outcomes and
canonical values are frozen. Existing temporal lexical/error/conformance tests
remain required. This fixture was formatted for syntax only, not compiled/run.

This removes parse temporaries; it does not price temporal arithmetic, duration
operations, casts' output rendering or produced lexical Strings. Those actual
production owners remain part of the complete evaluator batch.
