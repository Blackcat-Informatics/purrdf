# Package instantiation refusal under an added virtual limit

The current release artifact built successfully and the first package invocation
passed 456 of 458 tests. Two small linker-shape tests failed while instantiating
WASM memories, before reaching the tested runtime behavior. This invocation had
an added 16 GiB virtual address-space limit inherited by Node.

Both exact failing tests pass with the same artifact and standard Node defaults.
A diagnostic retaining two one-page WASM memories reports virtual sizes of
9,807,440 and 18,196,560 KiB, while resident use is 48,716 and 50,540 KiB.
The second reservation exceeds the added address-space limit without approaching
16 GiB resident use. This is a restriction introduced by the qualification
invocation, not evidence of a library allocation regression.

No source, tool, test, assertion, package option or host configuration was changed.
The unchanged complete npm gate now runs with the provided host defaults, then
executes the previously unreached identity-transport suffix. The initial failed
result remains in `capacity-integrated-package.log`; the two-test and reservation
receipts are `instantiation-standard-node-direct.log` and
`node-wasm-address-space-diagnostic.log`. An earlier attempted RSS wrapper did not
execute because `/usr/bin/time` is absent; it is not a test result.
