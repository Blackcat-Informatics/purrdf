# Integration evidence replay

Published source is signed cee1c41b2faa370b7daa60cc7536e784a4756d0e,
tree0ab9fd797b15e945c6884eeac293cf83bb8930b9. The paired main input is
dfc0c21adabe557e5d11f027aaf2576bddbe1dd6,
tree8564b8bd94da23bb073c5dc4e4768fa733d55ad6.

The original source, manifest, verbose compiler command, effective flags,
emitted directory, original artifact hashes and selected full LLVM/ASM bodies
remain in the corresponding t3-base, t3-candidate, t3-inline-cost and
t3-static-tail raw receipts. Earlier failed emissions are historical evidence;
the static-tail candidate is the qualified final source.

Ten large emitted .ll/.s files were losslessly compressed after independent
plaintext reading completed. Their logical original paths, byte lengths,
original SHA-256, compressed SHA-256 and actual decompression byte-cmp success
are in t3-lossless-evidence-packaging.log. For an archived logical path P,
`gzip -dc P.gz` yields exactly the original P bytes and hash. The original
artifact hash manifests describe those logical uncompressed bytes.

The exact installed editable ABI3 module is retained in
t3-final-installed-native-module.abi3.so.gz. Its original/compressed hashes and
byte lengths are in the adjacent archive receipts. Decompression yields the
exact module tested by the final fifteen Python-only boundary cases.

The standalone Rust probe sources and identical locks are retained in
native-cost-t3-{base,candidate}-probe. The owned detached base checkout was
removed after completed reviews. To compile them again, recreate that clean
detached checkout at the manifest's qualified path and input commit, or adjust
both manifests to equivalent isolated qualified paths. Use the recorded normal
managed compiler commands with CARGO_BUILD_JOBS=2 and locked dependencies.
Matching effective flags/stages matter: native probe emission used ThinLTO,
the host release emitted-stage comparison did not assert final-link LTO, and
the installed binding was the actual dev optimized+debuginfo artifact.

Only the two owned probe target symlinks and inventory executable were removed;
shared cache contents, source/proof binaries and all sibling worktrees were
preserved. The inventory Rust source is retained for rebuilding the read tool.
No timing measurements, wasm gates or broad semantic Python suites were run.
