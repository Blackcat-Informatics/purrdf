// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

/// Host-supplied, per-query non-deterministic inputs: the wall-clock value `NOW()`
/// returns and the seed for `RAND()`/`UUID()`/`STRUUID()`. The engine never samples
/// a clock or entropy source itself (a wasm build has neither), so a host injects an
/// implementation via [`NativeSparqlEngine::with_query_env`](crate::NativeSparqlEngine::with_query_env).
/// Without one the engine stays fully deterministic: epoch `NOW()`, seed 0.
pub trait QueryEnv: Send + Sync {
    /// The `xsd:dateTime` `NOW()` returns, sampled once per query (all `NOW()`
    /// calls in one query see the same instant).
    fn now(&self) -> purrdf_xsd::temporal::DateTime;
    /// The SplitMix64 seed for `RAND()`/`UUID()`/`STRUUID()`, sampled once per query.
    fn rng_seed(&self) -> u64;
}
