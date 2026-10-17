# Contract analysis

The full issue and zero comments require three modes at the official extended
output length, streaming keyed/derived input, no dependencies and the existing
backend dispatch. A 32-byte-only keyed implementation would fail acceptance.
Key and mode flags must enter every parent and batched chunk; extending only the
scalar path would silently produce wrong long messages. Root output uses a block
counter independent of the input chunk counter and includes all sixteen words.
Tests must force available kernels rather than only exercise default selection.
The motivation timing table is not acceptance and no timing will be measured.
