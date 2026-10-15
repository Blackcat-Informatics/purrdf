Commit `da57b2500` fixes the cross-link failure in the existing assembly
consumer command. Standard `cargo check` emits the original benchmark consumer
without linking or running it. The actual AArch64 lookup probe and all 112
registered sites pass their original criteria on that configuration. Existing
command/refusal checks and normal commit/push hooks pass. No vector floor,
arithmetic restriction, compiler or host tool was changed.

The complete seven-configuration producer is running as CI `38614052111`.
Single-configuration evidence does not establish that full acceptance; generated
parity and final hosted checks/review remain required.
