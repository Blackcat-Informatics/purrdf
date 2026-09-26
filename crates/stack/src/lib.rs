// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf-stack` — how much stack the running thread has left.
//!
//! Exhausting the stack is not an error anywhere: natively the process aborts, and on
//! `wasm32-unknown-unknown` the shadow stack runs below its floor and traps with the
//! instance's memory in an unknown state. A bound on how deeply a request may nest does
//! not bound the stack handling it needs — a level of one construct can cost a hundred
//! times a level of another, and the thread it runs on may be small — so the SPARQL
//! parser (`purrdf-sparql-algebra`) and evaluator (`purrdf-sparql-eval`) measure the
//! stack actually left at every recursive entry and refuse, typed, when less than
//! [`MARGIN_BYTES`] remain. This crate is that measurement, in one place: one floor per
//! thread, one margin, the walk scopes that let a recursion with no error channel refuse
//! too ([`walk`], [`walk_is_low`]), and the functions hosts call to install the floor of a
//! stack they switch onto ([`replace_floor`]) or to switch whole contexts
//! ([`replace_context`]).
//!
//! # The floor
//!
//! "Stack left" is the current stack pointer minus the lowest address the stack may
//! reach — the *floor* — for the stack the thread is running on. Every target PurRDF
//! ships on has a downward-growing stack.
//!
//! * **`wasm32`**: the stack is the shadow stack in linear memory, and its pointer is the
//!   address of any address-taken local ([`stack_pointer`]). The floor is a thread-local
//!   whose default is the low end of the module's own shadow stack. Every `wasm32` target
//!   rustc ships links with `--stack-first`, which makes the shadow stack the first region
//!   of linear memory, so that low end is address 0 — running below it wraps the pointer
//!   and traps. The linker records it as the symbol `__stack_low`; this crate has no
//!   `unsafe` code on `wasm32` and cannot name a linker symbol, so a host that can (the
//!   PurRDF wasm package, when its instance starts) installs the symbol's value with
//!   [`replace_floor`], which also covers a module linked with the stack elsewhere.
//!   Neither `__data_end` nor `__heap_base` says where the stack *ends*, and the first
//!   pointer observed is the stack's top, not its floor. A host that runs work on a
//!   stack of its own — the wasm package's asynchronous lane runs each job on a
//!   heap-allocated region — installs that stack's context with [`replace_context`]
//!   whenever it switches onto it and puts the previous context back whenever it switches
//!   away (see [Switching contexts](#switching-contexts)).
//! * **native**: the floor is the current thread's stack limit, as the operating system
//!   reports it — `platform::stack_floor` reads `pthread_getattr_np`,
//!   `pthread_attr_get_np`, `pthread_stackseg_np`, `pthread_get_stackaddr_np` /
//!   `pthread_get_stacksize_np`, or `GetCurrentThreadStackLimits`, per platform; see that
//!   module for which targets read which call, and which read nothing. It is read once per
//!   thread and cached in the same thread-local, so a check costs one thread-local load
//!   and one comparison. A pointer found *below* the cached floor means the thread is
//!   running on a different stack than the one the floor was read for (a host that switched
//!   onto a stack of its own, say), so the floor is read again rather than refusing: a
//!   stack switch never turns into a refusal. (A stack a host switched to *above* the
//!   cached floor is measured against the old floor, which overstates what is left: such a
//!   host installs its own floor with [`replace_floor`], as the wasm asynchronous lane
//!   does.) On a platform whose limit cannot be read at all the measurement is inactive —
//!   there is nothing to measure against — and [`remaining`] reports the whole address
//!   range below the frame, so nothing refuses.
//!
//! # The margin
//!
//! A check refuses when less than [`MARGIN_BYTES`] remain, so the margin must hold the
//! deepest chain of frames any guarded path can push *between two checks*, plus the
//! non-recursive work that runs after the last one. See [`MARGIN_BYTES`] for the measured
//! figures, for the evaluator and for the parser.
//!
//! # Walk scopes
//!
//! A recursion that has an error channel refuses by returning an error when [`is_low`]
//! says so. One that has none — an infallible analysis, substitution or copy over a tree
//! whose height nothing but the stack bounds — runs inside a [`walk`] scope instead: each
//! level asks [`walk_is_low`], the first that finds the margin gone latches the refusal in
//! the scope and installs [`EXHAUSTED`] so every later check refuses at once, and the
//! scope discards whatever the walk built and reports the refusal. Which scope is open,
//! and what it latched, is per-context state kept beside the floor.
//!
//! # Switching contexts
//!
//! The floor and the walk-scope state together are a [`Context`]: everything this crate
//! keeps for the computation running on the thread. A host that runs several
//! computations on one thread and switches between them *before any has returned* — the
//! PurRDF wasm package's asynchronous lane suspends a job in the middle of an evaluation
//! and runs the synchronous lane, or another job, until it resumes — swaps the whole
//! context with [`replace_context`] at every switch. Swapping the floor alone is not
//! enough: a job suspended inside a walk scope would leave that scope open for whatever
//! ran next, whose own walks would latch their refusals in it — handing a placeholder to a
//! caller with no scope to discard it, and leaving [`EXHAUSTED`] installed on a context
//! that never refused — and two jobs closing their scopes in the order they happen to be
//! resumed, rather than the reverse of the order they opened them, would leave each
//! other's scope state behind.

#![deny(unsafe_code)]

use core::cell::Cell;

#[cfg(not(target_arch = "wasm32"))]
#[allow(
    unsafe_code,
    reason = "reading the current thread's stack bounds from the operating system; each \
              block carries its own safety argument, and none of it compiles on wasm32, \
              where the crate stays unsafe-free"
)]
mod platform;

/// The stack a check requires to be left, in bytes: 128 KiB natively.
///
/// # Derivation
///
/// The margin has to hold the deepest chain of frames any guarded path pushes between
/// two checks, together with the work that runs past the last check before a leaf
/// returns.
///
/// **The evaluator** checks at every recursive entry, so what is left between two checks
/// is one level of one recursion plus the leaf under it. It was measured by lowering the
/// margin and sweeping the size of the thread a request runs on (x86_64, the workspace's
/// opt-level-3 profile), over 26 request shapes that include every nesting and sibling
/// form the parser admits at its limits:
///
/// * with a 4 or 8 KiB margin (threads of 96 to 704 KiB in 4 KiB steps) some thread
///   sizes abort — the widest interval is a basic graph pattern's first use of a lazily
///   built index, whose sort needs between 8 and 16 KiB;
/// * with a 16 KiB margin (threads of 32 to 736 KiB in 8 KiB steps, 2 314 runs) every
///   request answers or is the refusal, except on a 32 KiB thread, too small for the
///   walks that run once before a plan's first operator. The same sweep over whole
///   requests, governed ones, updates and in-process `SERVICE` calls found no other abort
///   in the evaluator.
///
/// **The parser** checks at its one recursion guard, which every production that can
/// reach itself again enters through, so what is left between two checks is one written
/// level of one construct (at most a sub-`SELECT`'s 8.5 KiB), its leaves, and the walks
/// over a finished operator or property-path chain — which a loop builds at no stack
/// cost, and which the parser builds only where a walk of its whole height fits the
/// stack left past this margin. Swept the same way — the margin lowered, the stack left
/// for a parse stepped by 2 KiB from 20 KiB, over every recursive production nested 127
/// levels deep and 380- to 511-operator expression and path chains, when chains were
/// still binary trees that recursed once per operator, at the top and 120 levels
/// down — a 48 KiB margin aborted on a 510-step property-path chain and a 64 KiB one
/// answered or refused every run; at 128 KiB, a 4 KiB-step sweep from 32 KiB to 828 KiB
/// left found no abort. The parser's own tests sweep forms nested hundreds of levels
/// deep the same way, and walk every tree they parse from the frame that parsed it.
///
/// 128 KiB is eight times the evaluator's widest interval and twice the parser's. The
/// rest is room for what the sweeps cannot see: host code a leaf calls (a registered
/// function, relation or aggregate, a `SERVICE` transport), and the derived copy and
/// comparison of terms, which nest no deeper than the parser's triple-term limit. It
/// costs a thread little it could have used: a default 2 MiB thread keeps 94% of its
/// stack, and a request refused here needed nearly all of it anyway.
#[cfg(not(target_arch = "wasm32"))]
pub const MARGIN_BYTES: usize = 128 * 1024;

/// The stack a check requires to be left, in bytes: 64 KiB on `wasm32`.
///
/// # Derivation
///
/// As for the native margin. **The evaluator's** was measured with the margin lowered to
/// 16 KiB, on the shipped npm artifact's synchronous lane with its shadow stack painted:
/// 565 requests mixing two nesting forms at a time (`NOT EXISTS`, `EXISTS`, `OPTIONAL`,
/// `LATERAL`, `MINUS`, sub-`SELECT`) over four innermost shapes (a triple, a 200-operator
/// chain, a 121-step path, a nested `EXISTS` over a 100-term conjunction), so the refusal
/// falls at every phase of every level. None trapped; the deepest frame any of them
/// reached was 11 064 bytes above the floor — 5 320 bytes past the point the check
/// refuses at. Only address-taken locals live on the shadow stack, so its levels are
/// smaller than their native twins, and host code (a JavaScript function or resolver)
/// runs on the engine's own stack, not on this one. **The parser's** frames cost 0.37 to
/// 0.45 of their native size on the shadow stack — at most 3.2 KiB for a written
/// sub-`SELECT` level, 1.8 KiB for a built-in call — so its widest native interval, the
/// walks over a 510-step path chain when chains were binary (between 48 and 64 KiB),
/// comes to under 29 KiB at those ratios.
///
/// 64 KiB is twelve times the measured interval. It leaves the synchronous lane 960 KiB
/// of its 1 MiB — 63 nested `FILTER EXISTS`, which reached 96% of the stack before the
/// evaluator's guard existed, is refused, where 63 nested `FILTER NOT EXISTS` used to
/// trap and corrupt the instance — and it sits below an asynchronous job's poll-time
/// guard band, so a job whose frames poll is stopped there first and a check stops the
/// frames that never poll. The parser checks against the same margin, so a parse deeper
/// than a lane's shadow stack is refused there too; the host engine's own call stack,
/// which this crate cannot read, the SPARQL parser bounds with a budget of its own.
#[cfg(target_arch = "wasm32")]
pub const MARGIN_BYTES: usize = 64 * 1024;

/// A floor that leaves no stack at all: while it is installed, [`remaining`] reports `0`
/// and [`is_low`] is `true` on this thread, whatever the stack pointer.
///
/// For a guard that must stop every further check on the thread at once — a [`walk`]
/// scope latches a refusal this way so the rest of the walk unwinds without computing
/// over the placeholder it left — installed with [`replace_floor`], and replaced by the
/// floor [`replace_floor`] returned once the refusal has been reported.
pub const EXHAUSTED: usize = usize::MAX - 1;

/// The floor's value before anything has been read or installed on this thread.
const UNKNOWN: usize = usize::MAX;

/// Where the running context stands with respect to [`walk`] scopes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WalkState {
    /// No scope is open: a walk that runs low has nobody to hand a refusal to, so it
    /// carries on exactly as it would with no guard at all.
    Unscoped,
    /// A scope is open and nothing has refused yet.
    Scoped,
    /// A walk inside the open scope refused at `construct`; the floor it replaced with
    /// [`EXHAUSTED`] is `floor`.
    Refused {
        /// What refused.
        construct: &'static str,
        /// The floor to put back when the scope closes.
        floor: usize,
    },
}

thread_local! {
    /// The floor of the stack this thread is running on; [`UNKNOWN`] until the first
    /// measurement reads it (or a host installs one with [`replace_floor`]).
    static FLOOR: Cell<usize> = const { Cell::new(UNKNOWN) };
    /// The running context's [`WalkState`].
    static WALK: Cell<WalkState> = const { Cell::new(WalkState::Unscoped) };
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
/// frame is already below it or [`EXHAUSTED`] is installed.
#[must_use]
pub fn remaining() -> usize {
    let sp = stack_pointer();
    match FLOOR.with(Cell::get) {
        EXHAUSTED => 0,
        floor => sp.checked_sub(floor).unwrap_or_else(|| refresh(sp)),
    }
}

/// Install `floor` as the floor of the stack this thread now runs on, returning the
/// floor it replaces (which the caller puts back when it switches away again).
///
/// Only a host that switches the stack pointer onto a stack of its own needs this, or a
/// guard latching a refusal with [`EXHAUSTED`]; see the [crate documentation](crate).
/// The value returned may be the "not yet read" marker, which is a valid value to put
/// back: the next measurement reads the floor again.
pub fn replace_floor(floor: usize) -> usize {
    FLOOR.with(|cell| cell.replace(floor))
}

/// Everything this crate keeps for the computation running on the thread: the floor of
/// its stack and the state of its [`walk`] scopes.
///
/// A host that switches between computations before they return swaps the whole of it
/// with [`replace_context`]; see [Switching contexts](crate#switching-contexts). The
/// value is opaque: a host only ever puts back what [`replace_context`] handed it, or
/// installs a fresh one with [`Context::on_floor`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Context {
    /// The floor [`replace_floor`] would install.
    floor: usize,
    /// The walk-scope state.
    walk: WalkState,
}

impl Context {
    /// The context of a stack whose floor is `floor`, with no walk scope open — what a
    /// host installs when it switches onto a stack of its own for a computation that has
    /// not started yet.
    #[must_use]
    pub const fn on_floor(floor: usize) -> Self {
        Self {
            floor,
            walk: WalkState::Unscoped,
        }
    }
}

/// Install `context` as the running computation's, returning the context it replaces
/// (which the caller puts back when it switches away again).
///
/// The floor and the walk-scope state are swapped together, so a walk scope a suspended
/// computation left open — and a refusal it latched there, with [`EXHAUSTED`] installed —
/// stays with that computation, and whatever runs while it waits sees only its own. See
/// [Switching contexts](crate#switching-contexts).
pub fn replace_context(context: Context) -> Context {
    Context {
        floor: replace_floor(context.floor),
        walk: WALK.with(|state| state.replace(context.walk)),
    }
}

/// Run `body` — an infallible recursive walk over a tree whose height nothing but the
/// stack bounds (an analysis, a substitution, a copy) — so that it can refuse when the stack
/// runs low.
///
/// Such a walk has no error channel, so a level of it that finds less than
/// [`MARGIN_BYTES`] left calls [`walk_is_low`], which latches the refusal in this scope
/// and tells the walk to stop descending and return a placeholder. The scope then
/// discards whatever `body` produced and returns `Err` with the construct that refused:
/// a placeholder never escapes, because the only way to reach `body`'s value is through
/// the `Ok` this returns only when nothing refused. Anything the walk wrote through a
/// reference must be discarded with it, which is why a caller hands its walk state it owns
/// (a fresh table, a fresh map) and drops it on the error.
///
/// Once a level has refused, the rest of the walk must not go on computing over the
/// placeholder it left — a later level could trip an internal consistency assertion on a
/// tree it half-built — so the refusal also replaces the running context's floor with
/// [`EXHAUSTED`]: every check in that context, in every walk, in the evaluator and in the
/// parser alike, then refuses at once, and the walk unwinds to this scope level by level
/// without doing any more work. The real floor is put back when the scope closes.
///
/// Scopes nest: the enclosing scope's state (and, after a refusal, the floor) is put back
/// when this one closes, even when `body` unwinds. They nest *per context*: a host that
/// suspends a computation inside a scope swaps its [`Context`] out with
/// [`replace_context`], so the scope is closed in the context that opened it.
///
/// # Errors
///
/// The `construct` a level of the walk named when it refused.
pub fn walk<T>(body: impl FnOnce() -> T) -> Result<T, &'static str> {
    /// Closes the scope when the walk returns or unwinds: puts the real floor back if a
    /// level refused, then the enclosing scope's state.
    struct Close(WalkState);
    impl Drop for Close {
        fn drop(&mut self) {
            let inner = WALK.with(|state| state.replace(self.0));
            if let WalkState::Refused { floor, .. } = inner {
                replace_floor(floor);
            }
        }
    }
    let close = Close(WALK.with(|state| state.replace(WalkState::Scoped)));
    let value = body();
    let outcome = WALK.with(Cell::get);
    drop(close);
    match outcome {
        WalkState::Refused { construct, .. } => Err(construct),
        WalkState::Unscoped | WalkState::Scoped => Ok(value),
    }
}

/// Whether a level of an infallible walk must stop descending: inside a [`walk`] scope,
/// less than [`MARGIN_BYTES`] of stack are left (the refusal is latched for the scope to
/// report), or a level of the same walk already refused. Outside any scope this is always
/// `false`. `construct` names the walk, for the scope's error.
///
/// The hot path is [`is_low`]'s; the scope is consulted only once the margin is gone.
#[inline]
#[must_use]
pub fn walk_is_low(construct: &'static str) -> bool {
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
            let floor = replace_floor(EXHAUSTED);
            state.set(WalkState::Refused { construct, floor });
            true
        }
        WalkState::Refused { .. } => true,
    })
}

/// Whether less than [`MARGIN_BYTES`] of stack are left.
///
/// The hot path is one thread-local load, one subtraction and one comparison; the floor
/// is read (or re-read) only on the cold path.
#[inline]
#[must_use]
pub fn is_low() -> bool {
    let sp = stack_pointer();
    sp.checked_sub(FLOOR.with(Cell::get))
        .is_none_or(|left| left < MARGIN_BYTES)
        && is_low_cold(sp)
}

/// The cold half of [`is_low`]: the floor was never read on this thread, the frame is
/// below it, [`EXHAUSTED`] is installed, or the margin really is gone.
#[cold]
#[inline(never)]
fn is_low_cold(sp: usize) -> bool {
    let left = match FLOOR.with(Cell::get) {
        EXHAUSTED => return true,
        floor => sp.checked_sub(floor).unwrap_or_else(|| refresh(sp)),
    };
    left < MARGIN_BYTES
}

/// Read the floor of the stack `sp` is on, cache it, and return the bytes left above it.
#[cfg(not(target_arch = "wasm32"))]
fn refresh(sp: usize) -> usize {
    // `platform::stack_floor` reads the thread's bounds directly from the operating
    // system rather than from any particular frame, so — unlike a floor derived from a
    // "bytes left" figure measured a few frames down — this is the thread's exact floor,
    // not an approximation of it.
    let floor = platform::stack_floor().unwrap_or(0);
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
/// Only the first measurement on an instance with no installed floor reads anything: it
/// takes [`DEFAULT_FLOOR`]. A frame below an installed floor has already run past the
/// stack that floor belongs to, so nothing is left — the one case a native thread
/// re-reads (a host that switched stacks under it) cannot arise here, where every stack
/// switch installs its floor.
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

    /// `levels` frames of 4 KiB below the caller, then `at_bottom`.
    fn deeper<T>(levels: usize, at_bottom: &dyn Fn() -> T) -> T {
        if levels == 0 {
            return at_bottom();
        }
        let frame = core::hint::black_box([0u8; 4096]);
        let value = deeper(levels - 1, at_bottom);
        core::hint::black_box(&frame);
        value
    }

    /// The first measurement on a thread reads its floor, and what it reports is that
    /// thread's own stack, and tracks the stack pointer: sixteen 4 KiB frames deeper, it
    /// reports at least 64 KiB less.
    #[test]
    fn remaining_measures_the_thread_s_own_stack() {
        const BYTES: usize = 4 * 1024 * 1024;
        let (top, below) = on_thread(BYTES, || (remaining(), deeper(16, &remaining)));
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

    /// On a small thread, `remaining` falls as frames are pushed, `is_low` turns true
    /// once the margin is eaten into — and not before — and both recover as the frames
    /// unwind.
    #[test]
    fn a_small_thread_runs_low_as_its_stack_is_consumed() {
        let (top, low_at_top, reports) = on_thread(512 * 1024, || {
            let top = remaining();
            let low_at_top = is_low();
            let mut reports = Vec::new();
            let mut levels = 0;
            loop {
                let (left, low) = deeper(levels, &|| (remaining(), is_low()));
                reports.push((left, low));
                if low {
                    break;
                }
                levels += 4;
            }
            (top, low_at_top, reports)
        });
        assert!(
            !low_at_top,
            "{top} bytes left at the top of a 512 KiB thread"
        );
        assert!(
            reports.windows(2).all(|pair| pair[1].0 < pair[0].0),
            "remaining falls with every four frames: {reports:?}"
        );
        // `remaining` and `is_low` read the stack pointer in two different frames, a few
        // hundred bytes apart at most; the kilobyte of slack below is that, not the step.
        const FRAME_SLACK: usize = 1024;
        let (last_left, last_low) = *reports.last().expect("at least one report");
        assert!(
            last_low && last_left < MARGIN_BYTES + FRAME_SLACK,
            "{reports:?}"
        );
        assert!(
            reports[..reports.len() - 1]
                .iter()
                .all(|&(left, low)| !low && left + FRAME_SLACK > MARGIN_BYTES),
            "nothing is low while the margin is whole: {reports:?}"
        );
        assert!(
            reports.len() > 2,
            "the thread had room for several steps before the margin: {reports:?}"
        );
        assert!(
            on_thread(512 * 1024, || !is_low()),
            "a fresh thread is not low"
        );
    }

    /// A floor installed above the running frame (a host that switched away without
    /// putting the previous floor back) is read again rather than reporting nothing left.
    #[test]
    fn a_floor_above_the_frame_is_read_again() {
        let (low, left) = on_thread(1024 * 1024, || {
            let previous = replace_floor(stack_pointer() + 4096);
            let low = is_low();
            let left = remaining();
            replace_floor(previous);
            (low, left)
        });
        assert!(!low);
        assert!(left > MARGIN_BYTES, "{left} bytes left");
    }

    /// `replace_floor` hands back what it replaced, so a host can nest switches, and an
    /// installed floor is the one measured against.
    #[test]
    fn replace_floor_sets_and_returns_the_floor() {
        on_thread(256 * 1024, || {
            let original = replace_floor(1);
            assert_eq!(replace_floor(2), 1);
            let sp = stack_pointer();
            replace_floor(sp - 8192);
            let left = remaining();
            assert!(
                (8192 - 1024..8192 + 1024).contains(&left),
                "{left} bytes left above a floor 8 KiB down"
            );
            assert!(is_low(), "8 KiB is inside the margin");
            assert_eq!(replace_floor(original), sp - 8192);
            assert!(!is_low(), "the thread's own floor is back");
        });
    }

    /// While [`EXHAUSTED`] is installed nothing is left, whatever the stack pointer, and
    /// putting the previous floor back restores the measurement.
    #[test]
    fn the_exhausted_floor_leaves_nothing() {
        on_thread(1024 * 1024, || {
            assert!(!is_low());
            let previous = replace_floor(EXHAUSTED);
            assert_eq!(remaining(), 0);
            assert!(is_low());
            assert_eq!(replace_floor(previous), EXHAUSTED);
            assert!(!is_low());
            assert!(remaining() > MARGIN_BYTES);
        });
    }

    /// A floor 1 KiB below the calling frame: inside the margin, so [`is_low`] is `true`.
    fn low_floor() -> usize {
        stack_pointer() - 1024
    }

    /// A walk that runs low inside a scope is refused with the construct it named, the
    /// rest of the walk sees [`EXHAUSTED`], and closing the scope puts the real floor back.
    /// Outside any scope the same low stack is not a refusal, and nothing is latched.
    #[test]
    fn a_scope_latches_a_low_walk_and_an_unscoped_walk_carries_on() {
        on_thread(1024 * 1024, || {
            assert!(!is_low(), "the thread's own floor, read");
            let real = FLOOR.with(Cell::get);
            let low = low_floor();
            replace_floor(low);
            assert!(!walk_is_low("unscoped walk"), "no scope, no refusal");
            assert_eq!(replace_floor(real), low, "nothing latched");

            let mut observed = None;
            let refused = walk(|| {
                replace_floor(low_floor());
                let first = walk_is_low("scoped walk");
                // The low floor is swapped for EXHAUSTED, so every later level stops too.
                let latched = FLOOR.with(Cell::get);
                observed = Some((first, latched, walk_is_low("a later level")));
            });
            assert_eq!(observed, Some((true, EXHAUSTED, true)));
            assert_eq!(refused, Err("scoped walk"), "the first refusal is reported");
            // The scope put back the floor it replaced — the low one this test installed.
            assert!(is_low());
            replace_floor(real);
            assert!(!is_low(), "the thread's own floor is back");
            assert_eq!(walk(|| walk_is_low("roomy walk")), Ok(false));
        });
    }

    /// A computation suspended inside a scope takes the scope with it: the context that
    /// runs while it waits has none, so a walk there that runs low carries on and latches
    /// nothing, and the suspended scope is still open — and still able to refuse — when
    /// the computation resumes. The neighbouring control swaps the floor alone, as a host
    /// that knew nothing of walk scopes would: the waiting context's walk then latches its
    /// refusal in the suspended scope, leaves [`EXHAUSTED`] as the waiting context's floor,
    /// and the suspended computation is refused for a walk it never ran.
    #[test]
    fn a_suspended_scope_stays_with_its_context() {
        on_thread(1024 * 1024, || {
            assert!(!is_low(), "the thread's own floor, read");
            let real = FLOOR.with(Cell::get);
            let mut observed = None;
            let suspended = walk(|| {
                // Suspend: the waiting context runs on its own, nearly exhausted, stack.
                let waiting_floor = low_floor();
                let job = replace_context(Context::on_floor(waiting_floor));
                let waiting_low = walk_is_low("waiting walk");
                let waiting = replace_context(job);
                // Resume: the job's scope is open, and refuses a walk that runs low.
                replace_floor(low_floor());
                let job_low = walk_is_low("resumed walk");
                observed = Some((
                    waiting_low,
                    waiting == Context::on_floor(waiting_floor),
                    job_low,
                ));
            });
            assert_eq!(observed, Some((false, true, true)));
            assert_eq!(
                suspended,
                Err("resumed walk"),
                "the job's own refusal, no other"
            );
            // Closing the refusing scope put back the floor the job's walk replaced.
            assert!(is_low());
            replace_floor(real);

            let waiting = walk(|| {
                let waiting_floor = low_floor();
                let job = replace_context(Context::on_floor(waiting_floor));
                let low = walk_is_low("waiting walk");
                let after = replace_context(job);
                (low, after == Context::on_floor(waiting_floor))
            });
            assert_eq!(
                waiting,
                Ok((false, true)),
                "the waiting walk had no scope and latched nothing; the job was not refused"
            );
            assert!(!is_low());

            let mut observed = None;
            let floor_only = walk(|| {
                let job_floor = replace_floor(low_floor());
                let low = walk_is_low("waiting walk");
                observed = Some((low, replace_floor(job_floor)));
            });
            assert_eq!(
                observed,
                Some((true, EXHAUSTED)),
                "swapping the floor alone latches the waiting walk in the job's scope, and \
                 leaves the waiting context exhausted"
            );
            assert_eq!(
                floor_only,
                Err("waiting walk"),
                "and refuses the job for the waiting context's walk"
            );
            replace_floor(real);
        });
    }
}
