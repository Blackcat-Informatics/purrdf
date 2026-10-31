#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Optimal-parser DP used by `BtOpt` / `BtUltra` / `BtUltra2`.
//!
//! Will host:
//! - Forward DP loop (`build_optimal_plan_impl`, upstream zstd
//!   `ZSTD_compressBlock_opt_generic`)
//! - Reverse traceback + sequence emit
//! - `init_stats_ultra` BtUltra2 first-pass (upstream zstd
//!   `ZSTD_initStats_ultra`)
//! - LDM integration (`optLdm_processMatchCandidate` parity hooks);
//!   the actual LDM matcher (gear hash + bucket table) lands as a
//!   dedicated `super::ldm` module during #111 Phase 5 (implements #18).
//!
//! Phase 1 of #111 extracts the plain-data types into [`types`] and the
//! LDM integration data types into [`ldm`]. DP method bodies stay on
//! `HcMatchGenerator` for now — they migrate when the per-strategy split
//! lands.

#![allow(dead_code)]

pub(crate) mod ldm;
pub(crate) mod types;
