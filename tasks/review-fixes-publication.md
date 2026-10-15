The carrier review findings are corrected in the production paths:

- Local graph admission stops at SERVICE. Its resolver admits the remote body;
  neighboring local GRAPH and FROM NAMED still refuse. The actual pre-fix runtime
  failure and subsequent native/WASM raw/prepared controls are retained.
- Recursive rules whose Base reads are only negative keep their semi-naive
  model delta. Negative atoms and negative conjunctions both produce the same
  66 path facts as the quad reference within its exact 691-step budget, on native
  and WASM. The old code exceeds that limit.
- Healthy TripleRead checks use the shared acquire flag; the original first
  cause is published under the mutex before the release store. Native
  sibling-thread and WASM first-fault/fresh-session controls pass.
- The assembly reader holds one shared artifact lease through both Cargo builds,
  artifact reads and the verdict. The actual regression fails before the repair;
  all seventeen reader controls pass afterward. Cargo uses the standard dirs.
- The backend contract names both carriers' frozen term owner and the actual
  Inference<TripleDataset> return type.

Affected warning-denied all-target Clippy and the actual helper gate pass.
The complete compact-carrier default-query corpus replay also passes with its
original grading and inventory (590 default cases and 45 typed named refusals).
The optional Base dictionary proposal is separately disposed in comment
6184209352; both independent arenas remain charged. It is not used to excuse
the negative-Base algorithm defect.

@coderabbitai Please review the published delta against the previous complete
review, including the SERVICE, shared-failure publication, recursive negative
Base and shared artifact-lease corrections. Recheck the two corrected contract
descriptions and report any remaining material findings. The complete hosted
assembly projection follows on the stable source; it is not yet claimed passed.
