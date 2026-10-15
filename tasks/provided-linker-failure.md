# Provided tool link failure

The standard metadata/book batch7318 terminated2. RDF/transcode/rule,
Unicode/lexicon and visualization generators ran;154 documented claims passed;
no generated projection or visualization drift appeared. The unchanged metadata
script then invoked its core lib-test suffix and failed at link:

```
error: the self-contained linker was requested, but it wasn't found in the target's sysroot, or in rustc's sysroot
error: could not compile `purrdf-core` (lib test) due to 1 previous error
make: *** [Makefile:82: metadata] Error 101
```

The original complete receipt is carrier-metadata-book.log. Metadata as a whole
FAILED; license-bundle, check-i18n, explicit doc-claim and English-book suffixes
were not reached. Earlier generator results are scoped observations only.

Read-only observations at2026-10-14 02:06:25UTC: ordinary rustc resolves through
/usr/bin/rustc to /opt/stage-rust/bin/rustc, reports1.100.0-nightly4b6d04e70 and
sysroot/opt/stage-rust. That sysroot contains native rust-lld. The provided Cargo
wrapper explicitly selects managed active-toolchain/bin/cargo. Its selected
compiler reports the managed nightly directory as sysroot, whose native
rustlib/bin was absent. Managed binaries were present with Oct13 20:05 timestamps.
This establishes different observed tool homes, not who changed them or why.
No compiler, linker, PATH, target/cache/config, installation or lifecycle change
was made by this task. Root owns the shared diagnosis.

Source freeze ended at the terminal. The existing identity control now has a
relabeled/scoped blank neighbor; clause/SRL docs distinguish the operational
form and carrier semantics. Focused native/WASM/rustdoc qualification is pending.
The installed rustfmt check passed before root requested no further broken-tool
execution; one subsequent prose-only clarification has only diff-check coverage.

Root subsequently observed restored ordinary tools and native managed rust-lld.
Direct standard readback now reports Rust1.101.0-nightly822182db4 (2026-10-13),
LLVM23.1.3; this is compiler drift from earlier1.100.0-nightly4b6d04e70. No tool
mutation was performed here. Root authorized affected checks to resume;27141 is
the new native/WASM relabeling and corrected-module rustdoc batch. It records
the actual compiler. Original7318 remains a failed gate, not retroactively green.
