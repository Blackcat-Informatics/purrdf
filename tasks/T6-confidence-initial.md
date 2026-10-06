What am I least confident about, and why?

Compiler, JIT and hardware side-channel behavior. The original native
implementation has independently reviewed fixed-schedule arithmetic and
full-scan comparisons, and it reproduces complete official NIST and IETF
answers natively and in wasm. Those checks establish functional behavior,
not a timing or fault-resistance proof. FIPS rejection sampling is
variable-time; no complete-operation constant-time or formal FIPS
certification claim is made.

What should the user know?

The construction is deliberately pinned to JOSE/COSE draft-04 and LAMPS
draft-19. Its requested COSE identifier -58 remains unassigned in the
official IANA refresh. The captured authoritative GTS upstream still
publishes only two EdDSA Sign1 vectors, so independent primary composite
answers do not establish shared-engine GTS composite interoperability.
Both components authenticate one Sign1; neither may survive independently.

Production callers must supply dedicated independent component keys and a
fresh, full cryptographic randomizer for each signature. Provider failure
refuses atomically; the portable library cannot detect external key reuse
or certify a successful provider's entropy quality. Controlled owned
secrets are overwritten on drop, but historical compiler copies, registers,
spills and hash states are outside that clearing guarantee.

The attached approved plan preserves its original planning snapshot,
including its historical progress footer. Current implementation and
qualification are recorded in the signed task checkpoints and final
qualification evidence. PR publication, hosted checks and Stage 2/3
acceptance are separately verified workflow states.
