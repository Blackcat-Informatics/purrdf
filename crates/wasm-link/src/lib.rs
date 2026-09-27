// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The post-link rewriter of the PurRDF wasm package.
//!
//! `make wasm-pkg` runs this tool last, after `wasm-opt`, on `purrdf_wasm_bg.wasm`, and
//! rewrites the module so that two obligations of the asynchronous lane are properties
//! of the module's own code rather than of the JavaScript around it: a resumed job runs
//! on its own shadow-stack pointer, and an instance that trapped refuses every later
//! call. Nothing that exists is renumbered; the rewrite appends functions, globals and
//! (when needed) one type, redirects the references it must, and rewrites the export
//! section. The output is validated before it is written, and `--check` proves an
//! already-linked module still has every property below.
//!
//! # The stack pointer
//!
//! Rust's shadow stack lives in linear memory behind one mutable `i32` global. The
//! module does not export it, so the linker finds it through the function wasm-bindgen
//! exports as `__wbindgen_add_to_stack_pointer`, whose body must be exactly
//! `local.get 0; global.get G; i32.add; global.set G; global.get G` (in either operand
//! order, since `wasm-opt` swaps the commutative operands) with no declared locals; any
//! other body is a hard error naming what was found. `G` is then exported as
//! [`STACK_POINTER_EXPORT`], so the JavaScript runtime can read it as a
//! `WebAssembly.Global`.
//!
//! # `$suspend`
//!
//! Every job suspends through one import, `./purrdf_jspi.mjs`.`purrdf_jspi_suspend`.
//! While it is suspended the host runs other work on the shadow stack, and by the time
//! the job resumes the stack pointer holds whatever that work left. The linker injects
//! `$suspend`, a function of the import's type that saves the stack pointer in a local,
//! sets it to the idle context's pointer (the `$idle` global), counts itself in
//! `$parked`, calls the import, and on return counts itself out, records the resumer's
//! stack pointer as the new `$idle`, restores its saved pointer and returns the
//! import's result. Every `call` and `return_call` of the import anywhere in the module
//! is redirected to `$suspend`, so the restore is structural: no compiler decision about
//! inlining or frame layout can remove it. A `ref.func`, table element or start-section
//! reference to the import is refused, because an indirect call would bypass `$suspend`;
//! a module in which nothing calls the import is refused too, since the lane it serves
//! was compiled out.
//!
//! # Import trampolines
//!
//! Every other import is reached through its own trampoline of the same type. A
//! trampoline saves `$idle`, counts the call in `$outbound`, calls the import, counts it
//! out and puts `$idle` back. The count lets the poison gate tell a legitimate re-entry
//! (the wasm-bindgen glue calling one of the module's allocator exports from inside an
//! import it is serving, or a synchronous sink callback calling back into the engine)
//! from a call that never returned; the `$idle` save lets a job started from inside such
//! a callback record the callback's context as its outer one without that value
//! outliving the callback.
//!
//! # The run wrapper
//!
//! The export named [`RUN_EXPORT`] runs a job on a region of the heap. The linker wraps
//! it in a function whose parameters are the run's followed by one more `i32`, the
//! region's top, which the JavaScript runtime now passes as the last argument of
//! `WebAssembly.promising(exports.purrdf_jspi_run)(job, top)`. The wrapper records the
//! current stack pointer as `$idle`, sets the stack pointer to the top, calls the
//! original, and restores `$idle` before returning. A run that finishes after a
//! resumption therefore returns to JavaScript with the pointer already back in the
//! resumer's context, closing the microtask gap during which a synchronous call could
//! otherwise have run on a finished job's region.
//!
//! # The poison gate
//!
//! Four mutable `i32` globals are appended and exported beside `$idle`:
//!
//! | Export | Meaning |
//! |---|---|
//! | [`POISONED_EXPORT`] | non-zero once the instance may not be entered again |
//! | [`ACTIVE_EXPORT`] | gated export entries that have not returned |
//! | [`PARKED_EXPORT`] | `$suspend` calls waiting for the import to return |
//! | [`OUTBOUND_EXPORT`] | trampoline calls waiting for their import to return |
//! | [`IDLE_EXPORT`] | the stack pointer of the JavaScript context that runs while wasm is suspended or returned |
//!
//! Every exported function is replaced by a gate wrapper that, on entry, traps when
//! `$poisoned` is non-zero; sets `$poisoned` and traps when
//! `$active − $parked ≠ $outbound`; and otherwise increments `$active`, calls the
//! original, decrements `$active` on the one exit and returns the result.
//!
//! The invariant behind the second check: whenever JavaScript is running and no wasm
//! frame has been unwound, every gated entry that has not returned is either parked in
//! `$suspend` or waiting inside an import call, so `$active − $parked − $outbound = 0`.
//! A trap, or a JavaScript exception thrown through wasm frames, unwinds an entry
//! without its exit — `$active` stays incremented — so the very next entry sees the
//! mismatch, marks the instance poisoned and refuses. A re-entrant call the glue makes
//! from inside an import it is serving sees `$active − $parked = $outbound` and passes.
//!
//! # What the JavaScript runtime does with this
//!
//! It passes the region top to the run export; it reads the exported globals rather
//! than tracking the stack pointer itself; it sets `purrdf_poisoned` to `1` when it
//! learns of a fault first (a Rust panic's hook, a run whose promise rejected); it
//! treats any trap out of an export while `purrdf_poisoned` is non-zero, or while
//! `purrdf_active − purrdf_parked ≠ purrdf_outbound` at the point the exception is
//! observed, as the instance's poisoning; and it never moves the stack pointer.
//!
//! # `--check`
//!
//! The check reads the exported globals, decodes every function body, and regenerates
//! each injected body from the indices the module names: every exported function must
//! be a gate around a defined function, the run export's gate must guard a run wrapper
//! around a function of the run's type, `__wbindgen_add_to_stack_pointer`'s guarded
//! function must set the exported stack-pointer global, every import must be reached
//! from exactly one function whose body is its trampoline (or `$suspend`), and no table
//! may reference an import. The module must also validate.

#![forbid(unsafe_code)]

mod error;
mod rewrite;
mod scan;
mod template;
mod verify;

#[cfg(test)]
mod tests;

use wasmparser::{Validator, WasmFeatures};

pub use error::LinkError;

/// The module string of the suspending import.
pub const SUSPEND_MODULE: &str = "./purrdf_jspi.mjs";
/// The field name of the suspending import.
pub const SUSPEND_NAME: &str = "purrdf_jspi_suspend";
/// The export whose body locates the shadow-stack pointer.
pub const ADD_TO_STACK_POINTER_EXPORT: &str = "__wbindgen_add_to_stack_pointer";
/// The export that runs an asynchronous job on a region.
pub const RUN_EXPORT: &str = "purrdf_jspi_run";
/// The shadow-stack pointer global, exported by the linker.
pub const STACK_POINTER_EXPORT: &str = "purrdf_stack_pointer";
/// The idle-context stack pointer global, exported by the linker.
pub const IDLE_EXPORT: &str = "purrdf_idle";
/// The poisoned flag, exported by the linker.
pub const POISONED_EXPORT: &str = "purrdf_poisoned";
/// The active-entry counter, exported by the linker.
pub const ACTIVE_EXPORT: &str = "purrdf_active";
/// The parked-suspension counter, exported by the linker.
pub const PARKED_EXPORT: &str = "purrdf_parked";
/// The outbound-import-call counter, exported by the linker.
pub const OUTBOUND_EXPORT: &str = "purrdf_outbound";
/// Every export the linker adds; their presence marks a module as already linked.
pub const RESERVED_EXPORTS: [&str; 6] = [
    STACK_POINTER_EXPORT,
    IDLE_EXPORT,
    POISONED_EXPORT,
    ACTIVE_EXPORT,
    PARKED_EXPORT,
    OUTBOUND_EXPORT,
];

/// The indices of the injected globals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateGlobals {
    /// `$idle`.
    pub idle: u32,
    /// `$poisoned`.
    pub poisoned: u32,
    /// `$active`.
    pub active: u32,
    /// `$parked`.
    pub parked: u32,
    /// `$outbound`.
    pub outbound: u32,
}

/// What a link or a check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// The shadow-stack pointer's global index.
    pub stack_pointer_global: u32,
    /// The suspending import's function index.
    pub suspend_import: u32,
    /// `$suspend`'s function index.
    pub suspend_function: u32,
    /// Call sites of `$suspend` outside itself: the calls of the import the link
    /// redirected.
    pub suspend_call_sites: usize,
    /// Import trampolines (every import but the suspending one).
    pub trampolines: usize,
    /// References to trampolines from outside them: the import references the link
    /// routed.
    pub trampoline_references: usize,
    /// The run wrapper's function index.
    pub run_wrapper: u32,
    /// Every function export, in export order; each now stands behind a gate.
    pub wrapped_exports: Vec<String>,
    /// The injected globals.
    pub gate_globals: GateGlobals,
    /// The feature set the module was validated under.
    pub features: String,
}

impl Report {
    /// One line naming everything that was found.
    pub fn describe(&self) -> String {
        let GateGlobals {
            idle,
            poisoned,
            active,
            parked,
            outbound,
        } = self.gate_globals;
        format!(
            "stack pointer is global {} (exported as {STACK_POINTER_EXPORT}); {SUSPEND_NAME} is function {}, \
             called only from $suspend (function {}), which {} call site(s) reach; {} other import(s) behind \
             trampolines with {} reference(s) routed; {RUN_EXPORT} wrapped by function {} (region top appended \
             as its last parameter); {} exported function(s) behind the poison gate (globals idle={idle}, \
             poisoned={poisoned}, active={active}, parked={parked}, outbound={outbound}); validated under {}",
            self.stack_pointer_global,
            self.suspend_import,
            self.suspend_function,
            self.suspend_call_sites,
            self.trampolines,
            self.trampoline_references,
            self.run_wrapper,
            self.wrapped_exports.len(),
            self.features
        )
    }
}

/// Link `bytes`: the rewritten module and what was found.
pub fn link(bytes: &[u8]) -> Result<(Vec<u8>, Report), LinkError> {
    rewrite::link(bytes)
}

/// Check an already-linked module.
pub fn check(bytes: &[u8]) -> Result<Report, LinkError> {
    verify::check(bytes)
}

/// Validate `bytes` under `features`, naming `stage` in the error.
pub(crate) fn validate(
    bytes: &[u8],
    features: WasmFeatures,
    stage: &'static str,
) -> Result<(), LinkError> {
    Validator::new_with_features(features)
        .validate_all(bytes)
        .map(|_| ())
        .map_err(|error| LinkError::Invalid {
            stage,
            message: error.to_string(),
        })
}
