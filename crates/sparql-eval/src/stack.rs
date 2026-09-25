// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! How much stack the running evaluation has left, and the guard every recursive
//! evaluator entry passes through.
//!
//! The parser bounds how deeply a request may nest
//! ([`purrdf_sparql_algebra::MAX_NESTING_DEPTH`]), but a level-count bound cannot bound
//! the stack *evaluating* that request needs: one written level of `FILTER NOT EXISTS`
//! costs the evaluator about 16.7 KB of wasm32 shadow stack, one of `LATERAL` about
//! 9.5 KB, and any other algebra level (`OPTIONAL`, `MINUS`, `BIND`, a sibling spine)
//! about 4.8 KB, so the deepest admitted requests need more than the synchronous wasm
//! lane's whole 1 MiB stack, while the flat and shallow ones a lower count would refuse
//! need a fraction of it. Exhausting the stack is not an error anywhere: natively the
//! process aborts, and on `wasm32-unknown-unknown` the shadow stack runs below its floor
//! and traps with the instance's memory in an unknown state.
//!
//! So every evaluator entry that can deepen the stack measures the stack actually left
//! and refuses with [`EvalError::StackExhausted`] — naming the construct — when it is
//! below [`MARGIN_BYTES`]. Nothing traps, and the thread (or the wasm instance) goes on to
//! answer the next request. There are three kinds of entry:
//!
//! * **Evaluation proper** checks and returns the error: every algebra node
//!   (`eval::eval_evaluated`), every expression node (`expr::eval_expr`), every `EXISTS`,
//!   every correlated evaluation (a `LATERAL` right side or a correlated `EXISTS`, per
//!   outer row) and every user-defined function call — and so do the fallible walks a
//!   plan passes through before its first operator: a governed evaluation's plan survey
//!   and the `1.2-basic` profile's admission.
//! * **Walks with no error channel** that run while evaluating — over a subtree whose
//!   height only the parser's budgets bound — run inside a `walk` scope. Each level asks
//!   `walk_is_low`; the first that finds the margin gone latches the refusal and returns a
//!   placeholder, and the scope discards whatever the walk built and returns the error.
//!   These are an `EXISTS` site's preparation (normalization, its source map, its
//!   structural analysis) and its scope-collision check, a correlated evaluation's
//!   per-row substitution copy, a property path's traversal, a `SERVICE` body's analysis,
//!   copy and serialization, an in-process `SERVICE`'s blank-node rewrite, a function
//!   body's copy and pre-binding rewrite, and template instantiation (`CONSTRUCT`, and
//!   an update's `DELETE`/`INSERT` and `DATA` templates). Their copies of algebra trees
//!   go through the `clone` submodule rather than the derived `Clone`, so they can refuse too.
//! * **Walks whose answer has a safe side** answer it when the stack is low, with no
//!   scope: the parallel-safety classification answers "unsafe" (the loop runs
//!   sequentially), `EXISTS` probe admissibility answers "not admissible" (the per-row
//!   definition runs), and the `GRAPH` emptiness proofs answer "not proven" (the graph
//!   is evaluated). Each is always correct, and the evaluation that follows refuses at
//!   its own next check.
//!
//! What remains unchecked is bounded by the parser's budgets (128 levels of graph
//! pattern, 512 of expression), at tens to a few hundred bytes a level: the derived copy
//! and drop of terms and of the per-row copies, the algebra serializer, the one-level
//! visitors — all of which the margin holds wherever they run — and the infallible walks
//! that run once over the whole plan before its first operator (planning, blank-node
//! scoping, the parallel and admission analyses). Those start from the top of the stack
//! the evaluation starts on; a thread too small even for them would need far more to
//! evaluate the plan's first levels, and is outside what this guard can make safe.
//!
//! # The floor
//!
//! "Stack left" is the current stack pointer minus the lowest address the stack may
//! reach — the *floor* — for the stack the evaluation is running on. Every target PurRDF
//! ships on has a downward-growing stack.
//!
//! * **`wasm32`**: the stack is the shadow stack in linear memory, and its pointer is the
//!   address of any address-taken local ([`stack_pointer`]). The floor is a thread-local
//!   whose default is the low end of the module's own shadow stack. Every `wasm32` target
//!   rustc ships links with `--stack-first`, which makes the shadow stack the first region
//!   of linear memory, so that low end is address 0 — running below it wraps the pointer
//!   and traps. The linker records it as the symbol `__stack_low`; this crate forbids
//!   `unsafe` code and cannot name a linker symbol, so a host that can (the PurRDF wasm
//!   package, when its instance starts) installs the symbol's value with
//!   [`replace_floor`], which also covers a module linked with the stack elsewhere.
//!   Neither `__data_end` nor `__heap_base` says where the stack *ends*, and the first
//!   pointer observed is the stack's top, not its floor. A host that runs evaluation on a
//!   stack of its own — the asynchronous lane runs each job on a heap-allocated region —
//!   installs that stack's floor with [`replace_floor`] whenever it switches onto it and
//!   puts the previous floor back whenever it switches away.
//! * **native**: the floor is the current thread's stack limit, as the operating system
//!   reports it (the `stacker` crate reads `pthread_getattr_np`,
//!   `pthread_get_stackaddr_np`, or `GetCurrentThreadStackLimits`, per platform). It is
//!   read once per thread and cached in the same thread-local, so the guard costs one
//!   thread-local load and one comparison. A pointer found *below* the cached floor means
//!   the thread is running on a different stack than the one the floor was read for (a
//!   host that grew its stack with `stacker::grow`, say), so the floor is read again
//!   rather than refusing: a stack switch never turns into a refusal. (A stack a host
//!   switched to *above* the cached floor is measured against the old floor, which
//!   overstates what is left: such a host installs its own floor with [`replace_floor`],
//!   as the wasm asynchronous lane does.) On a platform whose
//!   limit cannot be read at all the guard is inactive — there is nothing to measure
//!   against — and evaluation behaves as it did before the guard existed.
//!
//! # The margin
//!
//! A check refuses when less than [`MARGIN_BYTES`] remain, so the margin must hold the
//! deepest chain of frames any evaluator path can push *between two checks*, plus the
//! non-recursive work that runs after the last one (a basic graph pattern's join, a
//! sort, a regular-expression compilation, a result serialization of a `SERVICE`
//! request). See [`MARGIN_BYTES`] for the measured figures.

use core::cell::Cell;

use crate::error::EvalError;

pub(crate) mod clone;

/// The stack a check requires to be left, in bytes: 128 KiB natively.
///
/// # Derivation
///
/// The margin has to hold the deepest chain of frames any evaluator path pushes between
/// two checks, together with the work that runs past the last check before a leaf
/// returns. With a check at every recursive entry (see the [module documentation](self)),
/// what is left between two of them is one level of one recursion plus the leaf under it,
/// and it was measured by lowering the margin and sweeping the size of the thread a
/// request runs on (x86_64, the workspace's opt-level-3 profile), over 26 request shapes
/// that include every nesting and sibling form the parser admits at its limits:
///
/// * with a 4 or 8 KiB margin (threads of 96 to 704 KiB in 4 KiB steps) some thread
///   sizes abort — the widest interval is a basic graph pattern's first use of a lazily
///   built index, whose sort needs between 8 and 16 KiB;
/// * with a 16 KiB margin (threads of 32 to 736 KiB in 8 KiB steps, 2 314 runs) every
///   request answers or is this refusal, except on a 32 KiB thread, too small for the
///   walks that run once before a plan's first operator (see the module documentation).
///   The same sweep over whole requests, governed ones, updates and in-process `SERVICE`
///   calls found no other abort in the evaluator; the ones it found were in the parser,
///   which bounds its own recursion by level count and is not guarded here.
///
/// 128 KiB is eight times the widest interval. The rest is room for what the sweep cannot
/// see: host code a leaf calls (a registered function, relation or aggregate, a `SERVICE`
/// transport), and the recursive drop of a correlated evaluation's per-row copy, whose
/// height only the parser's expression budget bounds. It costs a thread little it could
/// have used: a default 2 MiB thread keeps 94% of its stack for evaluation, and a request
/// refused here needed nearly all of it anyway.
#[cfg(not(target_arch = "wasm32"))]
pub const MARGIN_BYTES: usize = 128 * 1024;

/// The stack a check requires to be left, in bytes: 64 KiB on `wasm32`.
///
/// # Derivation
///
/// As for the native margin, measured with the margin lowered to 16 KiB, on the shipped
/// npm artifact's synchronous lane with its shadow stack painted: 565 requests mixing two
/// nesting forms at a time (`NOT EXISTS`, `EXISTS`, `OPTIONAL`, `LATERAL`, `MINUS`,
/// sub-`SELECT`) over four innermost shapes (a triple, a 200-operator chain, a
/// 121-step path, a nested `EXISTS` over a 100-term conjunction), so the refusal falls at
/// every phase of every level. None trapped; the deepest frame any of them reached was
/// 11 064 bytes above the floor — 5 320 bytes past the point the check refuses at. Only
/// address-taken locals live on the shadow stack, so its levels are smaller than their
/// native twins, and host code (a JavaScript function or resolver) runs on the engine's
/// own stack, not on this one.
///
/// 64 KiB is twelve times the measured interval. It leaves the synchronous lane 960 KiB
/// of its 1 MiB for evaluation — 63 nested `FILTER EXISTS`, which reached 96% of the
/// stack before this guard existed, is refused, where 63 nested `FILTER NOT EXISTS` used
/// to trap and corrupt the instance — and it sits below an asynchronous job's poll-time
/// guard band, so a job whose frames poll is stopped there first and this check stops
/// the frames that never poll.
#[cfg(target_arch = "wasm32")]
pub const MARGIN_BYTES: usize = 64 * 1024;

/// The floor's value before anything has been read or installed on this thread.
const UNKNOWN: usize = usize::MAX;

/// The floor's value while a [`walk`] scope that has refused is still open: every check
/// on the thread refuses until the scope closes and puts the real floor back. See
/// [`walk`].
const REFUSED: usize = usize::MAX - 1;

thread_local! {
    /// The floor of the stack this thread is evaluating on; [`UNKNOWN`] until the first
    /// check reads it (or a host installs one with [`replace_floor`]).
    static FLOOR: Cell<usize> = const { Cell::new(UNKNOWN) };
}

/// The current stack pointer, as the address of a local of the calling frame.
///
/// On `wasm32` an address-taken local lives on the shadow stack, so this is the shadow
/// stack's depth, not the (inaccessible) value stack's; natively it is within one frame
/// of the machine stack pointer. `black_box` keeps the local a real, addressed slot.
/// Whether or not the call is inlined, the address is within this function's own small
/// frame of the caller's, which the margin absorbs.
#[inline]
#[must_use]
pub fn stack_pointer() -> usize {
    let marker = 0u8;
    core::hint::black_box(&raw const marker) as usize
}

/// The bytes of stack left below the calling frame before the floor, or `0` when the
/// frame is already below it.
#[must_use]
pub fn remaining() -> usize {
    let sp = stack_pointer();
    match FLOOR.with(Cell::get) {
        REFUSED => 0,
        floor => sp.checked_sub(floor).unwrap_or_else(|| refresh(sp)),
    }
}

/// Install `floor` as the floor of the stack this thread now evaluates on, returning the
/// floor it replaces (which the caller puts back when it switches away again).
///
/// Only a host that switches the stack pointer onto a stack of its own needs this; see
/// the [module documentation](self). The value returned may be the "not yet read" marker,
/// which is a valid value to put back: the next check reads the floor again.
pub fn replace_floor(floor: usize) -> usize {
    FLOOR.with(|cell| cell.replace(floor))
}

/// Refuse to go deeper when less than [`MARGIN_BYTES`] of stack are left.
///
/// `construct` names what was about to be evaluated, for the error.
///
/// # Errors
///
/// [`EvalError::StackExhausted`] naming `construct` when the stack left is below the
/// margin.
#[inline]
pub(crate) fn check(construct: &'static str) -> Result<(), EvalError> {
    if is_low() {
        return Err(EvalError::StackExhausted { construct });
    }
    Ok(())
}

/// Whether less than [`MARGIN_BYTES`] of stack are left.
///
/// The hot path is one thread-local load, one subtraction and one comparison; the floor
/// is read (or re-read) only on the cold path. [`check`] is this with the error attached;
/// a caller that names its construct only once it knows it is refusing (so the name is
/// never computed on the hot path), or a walk that cannot return an [`EvalError`] itself
/// and latches the refusal for its caller, calls this directly.
#[inline]
pub(crate) fn is_low() -> bool {
    let sp = stack_pointer();
    sp.checked_sub(FLOOR.with(Cell::get))
        .is_none_or(|left| left < MARGIN_BYTES)
        && is_low_cold(sp)
}

/// The cold half of [`is_low`]: the floor was never read on this thread, the frame is
/// below it, or the margin really is gone.
#[cold]
#[inline(never)]
fn is_low_cold(sp: usize) -> bool {
    let left = match FLOOR.with(Cell::get) {
        REFUSED => return true,
        floor => sp.checked_sub(floor).unwrap_or_else(|| refresh(sp)),
    };
    left < MARGIN_BYTES
}

/// Where the current thread stands with respect to [`walk`] scopes.
#[derive(Clone, Copy)]
enum WalkState {
    /// No scope is open: a walk that runs low has nobody to hand a refusal to, so it
    /// carries on exactly as it did before the guard existed.
    Unscoped,
    /// A scope is open and nothing has refused yet.
    Scoped,
    /// A walk inside the open scope refused at `construct`; the floor it replaced with
    /// [`REFUSED`] is `floor`.
    Refused {
        /// What refused.
        construct: &'static str,
        /// The floor to put back when the scope closes.
        floor: usize,
    },
}

thread_local! {
    /// This thread's [`WalkState`].
    static WALK: Cell<WalkState> = const { Cell::new(WalkState::Unscoped) };
}

/// Run `body` — an infallible recursive walk over part of the plan (an analysis, a
/// substitution, a copy) — so that it can refuse when the stack runs low.
///
/// Such a walk has no error channel, so a level of it that finds less than
/// [`MARGIN_BYTES`] left calls [`walk_is_low`], which latches the refusal on this thread
/// and tells the walk to stop descending and return a placeholder. This scope then
/// discards whatever `body` produced and returns [`EvalError::StackExhausted`] naming the
/// construct that refused: a placeholder never escapes, because the only way to reach
/// `body`'s value is through the `Ok` this returns only when nothing refused. Anything
/// the walk wrote through a reference must be discarded with it, which is why every
/// caller hands its walk state it owns (a fresh table, a fresh map) and drops it on the
/// error.
///
/// Once a level has refused, the rest of the walk must not go on computing over the
/// placeholder it left — a later level could trip an internal consistency assertion on a
/// tree it half-built — so the refusal also replaces the thread's floor with [`REFUSED`]:
/// every check on the thread, in every walk and in the evaluator alike, then refuses at
/// once, and the walk unwinds to this scope level by level without doing any more work.
/// The real floor is put back when the scope closes.
///
/// Scopes nest: the enclosing scope's state (and, after a refusal, the floor) is put back
/// when this one closes, even when `body` unwinds.
///
/// # Errors
///
/// [`EvalError::StackExhausted`] when any level of the walk refused.
pub(crate) fn walk<T>(body: impl FnOnce() -> T) -> Result<T, EvalError> {
    /// Closes the scope when the walk returns or unwinds: puts the real floor back if a
    /// level refused, then the enclosing scope's state.
    struct Close(WalkState);
    impl Drop for Close {
        fn drop(&mut self) {
            let inner = WALK.with(|state| state.replace(self.0));
            if let WalkState::Refused { floor, .. } = inner {
                FLOOR.with(|cell| cell.set(floor));
            }
        }
    }
    let close = Close(WALK.with(|state| state.replace(WalkState::Scoped)));
    let value = body();
    let outcome = WALK.with(Cell::get);
    drop(close);
    match outcome {
        WalkState::Refused { construct, .. } => Err(EvalError::StackExhausted { construct }),
        WalkState::Unscoped | WalkState::Scoped => Ok(value),
    }
}

/// Whether a level of an infallible walk must stop descending: inside a [`walk`] scope,
/// less than [`MARGIN_BYTES`] of stack are left (the refusal is latched for the scope to
/// report), or a level of the same walk already refused. Outside any scope this is always
/// `false`. `construct` names the walk, for the error.
#[inline]
pub(crate) fn walk_is_low(construct: &'static str) -> bool {
    if !is_low() {
        return false;
    }
    walk_refuse(construct)
}

/// The cold half of [`walk_is_low`]: latch the refusal in the open scope, if any.
#[cold]
#[inline(never)]
fn walk_refuse(construct: &'static str) -> bool {
    WALK.with(|state| match state.get() {
        WalkState::Unscoped => false,
        WalkState::Scoped => {
            let floor = FLOOR.with(|cell| cell.replace(REFUSED));
            state.set(WalkState::Refused { construct, floor });
            true
        }
        WalkState::Refused { .. } => true,
    })
}

/// Read the floor of the stack `sp` is on, cache it, and return the bytes left above it.
#[cfg(not(target_arch = "wasm32"))]
fn refresh(sp: usize) -> usize {
    // `remaining_stack` measures from its own frame, a few bytes below `sp`; the floor
    // derived from it is therefore at most those few bytes too high, which errs toward
    // refusing a hair early rather than late.
    let floor = stacker::remaining_stack().map_or(0, |left| sp.saturating_sub(left));
    FLOOR.with(|cell| cell.set(floor));
    sp.saturating_sub(floor)
}

/// The floor of the synchronous shadow stack when no host has installed one: address 0.
///
/// Every `wasm32` target rustc ships links with `wasm-ld --stack-first`, which places the
/// shadow stack as the first region of linear memory: `[0, stack size)`, the stack
/// pointer starting at its top (1 MiB by default) and growing down toward address 0,
/// with static data and the heap above it. The linker records that low end as the symbol
/// `__stack_low`; this crate forbids `unsafe` code, so it cannot name a linker symbol
/// itself, and takes the value the layout fixes instead. A host that links otherwise
/// (`--no-stack-first` puts the stack above the static data) or that can read the
/// symbol installs the real floor with [`replace_floor`] — the PurRDF wasm package does
/// exactly that, from `__stack_low`, when its instance starts.
#[cfg(target_arch = "wasm32")]
const DEFAULT_FLOOR: usize = 0;

/// Read the floor of the stack `sp` is on, cache it, and return the bytes left above it.
///
/// Only the first check on an instance with no installed floor reads anything: it takes
/// [`DEFAULT_FLOOR`]. A frame below an installed floor has already run past the stack
/// that floor belongs to, so nothing is left — the one case a native thread re-reads
/// (a host that switched stacks under it) cannot arise here, where every stack switch
/// installs its floor.
#[cfg(target_arch = "wasm32")]
fn refresh(sp: usize) -> usize {
    if FLOOR.with(Cell::get) == UNKNOWN {
        FLOOR.with(|cell| cell.set(DEFAULT_FLOOR));
        return sp - DEFAULT_FLOOR;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run `body` on a fresh thread with `bytes` of stack, so the thread-local floor is
    /// read for that thread alone.
    fn on_thread<T: Send + 'static>(bytes: usize, body: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(bytes)
            .spawn(body)
            .expect("spawn")
            .join()
            .expect("join")
    }

    /// The first measurement on a thread reads its floor, and what it reports is that
    /// thread's own stack, and tracks the stack pointer: sixteen 4 KiB frames deeper, it
    /// reports at least 64 KiB less.
    #[test]
    fn remaining_measures_the_thread_s_own_stack() {
        const BYTES: usize = 4 * 1024 * 1024;
        fn deeper(levels: usize) -> usize {
            if levels == 0 {
                return remaining();
            }
            let frame = core::hint::black_box([0u8; 4096]);
            let left = deeper(levels - 1);
            core::hint::black_box(&frame);
            left
        }
        let (top, below) = on_thread(BYTES, || (remaining(), deeper(16)));
        // At least what was asked for, less the frames already live. (The C library may
        // hand a thread more than it asked for, never less.)
        assert!(
            top > BYTES - 64 * 1024,
            "{top} bytes left of at least {BYTES}"
        );
        let used = top - below;
        assert!(
            (16 * 4096..16 * 4096 + 32 * 1024).contains(&used),
            "sixteen 4 KiB frames used {used} bytes"
        );
    }

    /// A check passes on a roomy stack and refuses, typed, once a recursion has eaten
    /// into the margin — and the thread keeps running afterwards.
    #[test]
    fn check_refuses_inside_the_margin_and_not_before() {
        fn descend(level: usize) -> Result<usize, EvalError> {
            check("test recursion")?;
            // A frame big enough that the recursion reaches the margin quickly.
            let frame = core::hint::black_box([0u8; 4096]);
            descend(level + 1).map(|deepest| deepest.max(level + usize::from(frame[0])))
        }
        let refused = on_thread(1024 * 1024, || (descend(0), check("after")));
        assert_eq!(
            refused.0,
            Err(EvalError::StackExhausted {
                construct: "test recursion"
            })
        );
        assert_eq!(refused.1, Ok(()), "the unwound thread passes a check again");
    }

    /// A floor installed above the running frame (a host that switched away without
    /// putting the previous floor back) is read again rather than refusing every check.
    #[test]
    fn a_floor_above_the_frame_is_read_again() {
        let (checked, left) = on_thread(1024 * 1024, || {
            let previous = replace_floor(stack_pointer() + 4096);
            let checked = check("after a stale floor");
            let left = remaining();
            replace_floor(previous);
            (checked, left)
        });
        assert_eq!(checked, Ok(()));
        assert!(left > MARGIN_BYTES, "{left} bytes left");
    }

    /// `replace_floor` hands back what it replaced, so a host can nest switches.
    #[test]
    fn replace_floor_returns_the_previous_floor() {
        on_thread(256 * 1024, || {
            let original = replace_floor(1);
            assert_eq!(replace_floor(2), 1);
            assert_eq!(replace_floor(original), 2);
        });
    }
}
