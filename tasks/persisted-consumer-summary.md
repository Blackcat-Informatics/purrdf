# Persisted query and block-layout repair

Published `47725646e` stops statement-layer traversal through the existing shared
ControlFlow consumer, preserves original source-local addresses for same-source
probes, and uses indexed reifier/annotation membership. The native file provider
is consolidated with the old example into one implementation. Resident image
Debug output is bounded. No dependency or Cargo feature is added.

Actual file-backed tests pass at 2000 and 200,000 claims, under a 256 MiB query
budget. The large source has 600,260 terms. LIMIT 10 touches 21 representatives;
COUNT independently returns 400,000 rows. Three fitting layouts preserve answers:

- 4096 bytes / 32 records: image 171,057,152 bytes; COUNT peak 193,296,356 bytes.
- 4096 / 64: image 92,127,232 bytes; COUNT peak 193,296,356 bytes.
- 8192 / 128: image 90,906,624 bytes; COUNT peak 168,261,604 bytes.

The 4096 / 128 neighbor explicitly refuses both populations because its encoded
record block does not fit. The measured byte-kind breakdown identifies the two
dictionary indexes as about 90% of the original image; authentication accounts
for 4,194,304 bytes. The sizing guide describes these measured inputs and their
limits without a universal ratio or timing claim.

All fourteen actual portable segmented-stack controls, strict affected all-target
compilation, helper hygiene, metadata generation and normal commit/push hooks
pass. The mandatory final whole gate is running. No whole-gate PASS, complete PR
qualification, merge or issue closure is claimed. Original failures remain
retained, including the losslessly compressed oversized Debug receipt.
