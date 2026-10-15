# Fixed-work native block-cache comparison

The unchanged Rust cache_control case constructs one 16384-term image, preloads
64 and 4096 real dictionary blocks in separate sessions, and executes exactly
262144 cache hits per session. It then verifies LRU eviction, pinned payloads,
reloads and final-owner release. Both versions pass the same complete case.

Both executables use the standard installed compiler and optimized test profile.
The baseline replaces only session.rs with main plus registration of this same
test module; the keyed view is not called by this control. Candidate source is
restored. No target/compiler/cache machinery is changed.

perf stat -r 5 counts instructions:u. No-match controls isolate harness setup;
fixture creation and preload remain inside the compared complete case. This is
not a pure-hit microkernel or an end-to-end SPARQL claim. No timing is used.

| Source | Whole instructions | No-match | Incremental | Repeat variation |
|---|---:|---:|---:|---|
| baseline | 3500763255 | 1027140 | 3499736115 | 0.00% (setup 0.02%) |
| candidate | 1074015597 | 1019373 | 1072996224 | 0.00% (setup 0.02%) |

Incremental change: -69.341 percent. The rounded repeat
variation is not a claim of zero uncertainty. Each operation process reports
exactly one passing test; each no-match process reports zero. Raw .perf, .stdout
and .stderr receipts remain beside the captured Cargo build JSON and logs.
