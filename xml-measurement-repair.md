# XML measurement repair

All five native assembly jobs in CI run 38706966700 fail at the same selector:
xml_escape_text<TextSink> is inlined away. Their actual logs are retained as
asm-{x86,x86-v3,x86-v4,arm,neoverse}.log. This is a missing emitted-symbol failure,
not a numerical count mismatch or a failed cache/query test.

The existing write_term<TextSink> is explicitly out of line and contains those
escape calls. The WASM selector already uses it. The bounded correction shares
that same writer selector across all seven configurations, retaining every scan
floor, mnemonic requirement, FMA bound and relaxed-operation refusal.

This exact selector correction and its generator-produced XML document row are
transferred from the qualified carrier publication. The XML writer, TextSink and
core XML escape sources are unchanged between the branches. Other dependency
sources do differ, so the carrier reports are not called current-head evidence
for this branch. The copied row is a candidate pending new current-head assembly
qualification. No report identity is rewritten and no source/compiler guard is
disabled. No other carrier measurement cells are imported.

Production Rust is unchanged by this correction. Existing query/cache/native/
portable qualification remains applicable. The live final local gate is retained;
new current-head hosted reports must independently establish complete assembly
parity before integration.
