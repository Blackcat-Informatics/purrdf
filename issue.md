# Issue #1365: Strict lints: deny levels in the workspace table and a stricter clippy line in make check

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Problem

The workspace lints warn where they should refuse. `unreachable_pub` and the clippy
`all` group are at `warn`. `unused`, `dead_code`, `allow_attributes` and the
other named lints are not set at all. No gate builds the library and binary
targets with the stricter set below.

A first run of the clippy command below on `main` stopped at `purrdf-hash` with
`clippy::allow_attributes` on 8 `#[allow]` sites: `crates/rdf-core/build.rs:18`,
`crates/hash/src/lib.rs:103`, `crates/hash/src/fixed/fold.rs:38`,
`crates/hash/src/fixed/keys.rs:97,112,127,137` and `crates/hash/src/hex.rs:424`.
Crates after `purrdf-hash` were not checked, so the full count is unknown.

## Change

1. Add this to `make check`:

   ```
   cargo clippy --workspace --lib --bins -- \
     -D warnings \
     -D unused \
     -D unreachable-pub \
     -D clippy::allow_attributes \
     -D clippy::used_underscore_binding \
     -D clippy::needless_pass_by_value \
     -D clippy::trivially_copy_pass_by_ref \
     -D clippy::redundant_clone
   ```

2. Raise the root `[workspace.lints]` tables to these levels:

   ```toml
   [workspace.lints.rust]
   unused = "deny"
   dead_code = "deny"
   unreachable_pub = "deny"
   unused_crate_dependencies = "warn"

   [workspace.lints.clippy]
   all = { level = "deny", priority = -1 }
   pedantic = { level = "warn", priority = -1 }
   allow_attributes = "deny"
   allow_attributes_without_reason = "deny"
   used_underscore_binding = "deny"
   dbg_macro = "deny"
   todo = "deny"
   unimplemented = "deny"
   ```

   Keep the existing entries this list does not name: `nursery`, `missing_docs` and the
   other rust lints, and the reasoned opt-out list. Every member already inherits the
   workspace table. The three manifests without a `[lints]` table (the GTS fuzz
   crate, the preserve-order consumer and the clean-room oracle template) are decided
   one by one.

3. Fix every violation in the code. Do not add `allow` entries. With
   `allow_attributes` denied, each remaining justified suppression becomes
   `#[expect(..., reason = "...")]`. Update the `#[allow]` guidance in `AGENTS.md`
   and `docs/agents/RUST_GUIDELINES.md` to match.

## Acceptance

- The clippy command above exits 0 on the branch head, run with the gate's flags
  exactly, and is in `make check`.
- The raised workspace levels are in the root `Cargo.toml`, and the existing
  `make check` clippy line (`--all-targets -D warnings`) still passes.
- No new `#[allow]` attributes, and no new opt-outs in the lint tables.


## Comments (0)

