// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Certified numerical arithmetic over controlled binary64 and exact dyadic grids.
//!
//! Every fixed-grid endpoint is an integer multiple of `2^-precision_bits`.
//! Arithmetic rounds outward with exact integer quotients. Transcendental series
//! carry mathematical remainder bounds separately from their outward evaluation.
//! No platform transcendental library, fused operation, or reassociated sum is used.

mod taylor;
pub use taylor::{
    SymmetricTaylorPanel, TaylorJet, TaylorScratch, TaylorScratchError, TaylorWorkspace,
    integrate_taylor_panel, integrate_taylor_panel_observed, with_taylor_workspace,
    with_taylor_workspace_observed,
};

mod fixed;
mod matrix;
pub use matrix::{Matrix3, invert_matrix3, matrix_vector3};
mod floating;
mod solver;
pub use solver::{CertifiedInterval, FloatBound, FloatEnclosure, IntervalBound, IntervalContext};
mod roots;
pub use roots::{
    RootBox2, RootIsolation2, RootIsolationLimits, RootJacobian2, RootStorage2, RootSystem2,
    compose_jacobians2, inverse_coordinate_complete, inverse_coordinate_complete_with_error,
    invert_jacobian2, isolate_roots2, isolate_roots2_into, root_enclosure_complete,
};

#[cfg(test)]
mod tests;

use core::fmt;
use core::marker::PhantomData;
use purrdf_hash::Backend as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::BigInt;
use crate::ieee::{Binary64Scope, environment};

pub use fixed::{FixedInterval, FixedIntervalSum};
pub use floating::{
    CheckedBinary64, DoubleDouble, FloatInterval, FloatProductBackend, PreparedBinary64, Word,
    WordInterval,
};

// Inline interval/integer objects, vector headers, carry words and the context
// itself are counted in addition to the bit payload. This also bounds the zero
// and low-precision cases, whose inline storage dominates their payload.
const SCRATCH_OVERHEAD: usize = 8192;

/// Deterministic counts of numerical execution paths, without clocks or globals.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MathExecutionStats {
    /// Binary64 transcendental requests, including requests needing fallback.
    pub binary64_requests: u64,
    /// Requests evaluated through the exact fixed-grid fallback.
    pub fixed_fallbacks: u64,
    /// Generated immutable binary64 coefficient tables.
    pub binary64_preparations: u64,
}

/// Explicit admission limits for one numerical context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MathLimits {
    /// Inclusive dyadic endpoint precision, from 16 through 8192 bits.
    pub precision_bits: u32,
    /// Maximum number of charged numerical arithmetic and series work items.
    pub max_work: u64,
    /// Conservative upper bound for live numerical workspace in bytes.
    pub max_workspace_bytes: usize,
}

impl MathLimits {
    /// The ordinary 128-bit, 262144-item, 64-MiB numerical admission.
    pub const DEFAULT: Self = Self {
        precision_bits: 128,
        max_work: 262_144,
        max_workspace_bytes: 64 * 1024 * 1024,
    };
}

impl Default for MathLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A mathematical or operational failure; no unfinished estimate is returned.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MathError {
    /// The requested exact operation has no value on its input domain.
    Domain(&'static str),
    /// The requested precision or an unresolved interval decision is inadmissible.
    PrecisionExhausted,
    /// The admitted numerical work has been consumed.
    WorkExhausted,
    /// An intermediate would exceed the admitted workspace.
    WorkspaceExhausted,
    /// The explicitly admitted reusable integer arena has no suitable destination.
    LimbScratch(crate::integer::LimbScratchError),
    /// Another branded workspace owns the installed Taylor coefficient buffers.
    TaylorScratch(TaylorScratchError),
    /// A finite binary64 result cannot represent the requested enclosure.
    Binary64Range,
    /// Explicit cancellation was observed at a numerical work boundary.
    Cancelled,
    /// An admitted iterative solver could not isolate its certified result.
    ConvergenceExhausted {
        /// Number of completed solver iterations admitted by the policy.
        iterations: u32,
    },
    /// Separately certified roots remain in one explicitly admitted inverse domain.
    AmbiguousRoots {
        /// Number of distinct roots proved to exist.
        roots: usize,
    },
    /// The calling thread does not admit the controlled floating operations.
    Environment(environment::FloatEnvironmentError),
}

impl fmt::Display for MathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(message) => write!(f, "numerical domain: {message}"),
            Self::PrecisionExhausted => f.write_str("numerical precision exhausted"),
            Self::WorkExhausted => f.write_str("numerical work exhausted"),
            Self::WorkspaceExhausted => f.write_str("numerical workspace exhausted"),
            Self::LimbScratch(error) => error.fmt(f),
            Self::TaylorScratch(error) => error.fmt(f),
            Self::Binary64Range => f.write_str("numerical enclosure exceeds finite binary64"),
            Self::Cancelled => f.write_str("numerical evaluation cancelled"),
            Self::ConvergenceExhausted { iterations } => write!(
                f,
                "numerical convergence exhausted after {iterations} iterations"
            ),
            Self::AmbiguousRoots { roots } => write!(f, "{roots} certified admissible roots"),
            Self::Environment(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for MathError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Environment(error) => Some(error),
            Self::LimbScratch(error) => Some(error),
            Self::TaylorScratch(error) => Some(error),
            _ => None,
        }
    }
}

/// Per-worker scratch, generated constants, and resource accounting.
/// A context holds no floating-point control guard between calls.
#[derive(Debug)]
pub struct CoordinateMath {
    limits: MathLimits,
    scale: BigInt,
    limb_scratch: Option<crate::integer::LimbScratch>,
    taylor_scratch: Option<TaylorScratch>,
    taylor_scratch_owned_bytes: usize,
    work: u64,
    workspace_peak: usize,
    workspace_reserved: usize,
    cancelled: Option<Arc<AtomicBool>>,
    pi: Option<FixedInterval>,
    ln2: Option<FixedInterval>,
    binary64: Option<PreparedBinary64>,
    binary64_backend: FloatProductBackend,
    execution_stats: MathExecutionStats,
    thread: PhantomData<*const ()>,
}

impl CoordinateMath {
    /// Admit one context and validate the current thread's floating environment.
    ///
    /// # Errors
    ///
    /// Returns a typed precision, workspace, work, or environment refusal.
    pub fn new(limits: MathLimits) -> Result<Self, MathError> {
        Self::new_using(limits, None)
    }

    /// Admit the same context using an enclosing owner's already admitted
    /// arena. Even its high-precision grid constant uses bounded reusable
    /// storage, so construction has no temporary integer heap allocation.
    /// The enclosing owner retains the complete arena receipt for live values.
    /// # Errors
    /// Adds destination capacity/exhaustion to [`Self::new`]'s refusals.
    pub fn new_with_borrowed_limb_scratch(
        limits: MathLimits,
        scratch: crate::integer::LimbScratch,
    ) -> Result<Self, MathError> {
        Self::new_using(limits, Some(scratch))
    }

    fn new_using(
        limits: MathLimits,
        scratch: Option<crate::integer::LimbScratch>,
    ) -> Result<Self, MathError> {
        if !(16..=8192).contains(&limits.precision_bits) {
            return Err(MathError::PrecisionExhausted);
        }
        if limits.max_work == 0 {
            return Err(MathError::WorkExhausted);
        }
        let bytes = (limits.precision_bits as usize)
            .div_ceil(8)
            .saturating_mul(32)
            .saturating_add(SCRATCH_OVERHEAD);
        if bytes > limits.max_workspace_bytes {
            return Err(MathError::WorkspaceExhausted);
        }
        environment::check().map_err(MathError::Environment)?;
        let one = BigInt::from_i128(1);
        let scale = match &scratch {
            Some(scratch) => one
                .mul_pow2_in(limits.precision_bits, scratch)
                .map_err(MathError::LimbScratch)?,
            None => one.mul_pow2(limits.precision_bits),
        };
        Ok(Self {
            limits,
            scale,
            limb_scratch: scratch,
            taylor_scratch: None,
            taylor_scratch_owned_bytes: 0,
            work: 0,
            workspace_peak: bytes,
            workspace_reserved: 0,
            cancelled: None,
            pi: None,
            ln2: None,
            binary64: None,
            binary64_backend: FloatProductBackend::selected(),
            execution_stats: MathExecutionStats::default(),
            thread: PhantomData,
        })
    }

    /// Install reusable limb storage and admit its complete retained heap.
    /// Arithmetic using it returns workspace exhaustion instead of allocating
    /// a replacement buffer when the admitted arena is full.
    /// # Errors
    /// Refuses an already installed arena or an excessive workspace reservation.
    pub fn set_limb_scratch(
        &mut self,
        scratch: crate::integer::LimbScratch,
    ) -> Result<(), MathError> {
        self.install_limb_scratch(scratch, true)
    }

    /// Borrow a worker arena whose whole retained heap is already admitted by
    /// the enclosing owner. This context accounts only additional numerical
    /// storage, so nested workers do not count the same arena twice. The owner
    /// must keep its retained receipt for every live value from this arena.
    /// # Errors
    /// Refuses replacing a previously installed arena.
    pub fn set_borrowed_limb_scratch(
        &mut self,
        scratch: crate::integer::LimbScratch,
    ) -> Result<(), MathError> {
        self.install_limb_scratch(scratch, false)
    }

    fn install_limb_scratch(
        &mut self,
        scratch: crate::integer::LimbScratch,
        owns_receipt: bool,
    ) -> Result<(), MathError> {
        if self.limb_scratch.is_some() {
            return Err(MathError::Domain("integer scratch already installed"));
        }
        let retained = if owns_receipt {
            scratch.retained_bytes()
        } else {
            0
        };
        self.reserve_workspace(retained)?;
        // Constants participate in the same bounded arithmetic as endpoints.
        // In particular, clipping to +/-scale must clone an immutable arena
        // handle rather than allocate a fresh high-precision power of two.
        let scale = match self.scale.copy_in(&scratch) {
            Ok(scale) => scale,
            Err(error) => {
                self.release_workspace(retained)?;
                return Err(MathError::LimbScratch(error));
            }
        };
        self.scale = scale;
        self.limb_scratch = Some(scratch);
        Ok(())
    }

    /// Borrow the explicitly installed arena without changing resource counters.
    #[must_use]
    pub fn limb_scratch(&self) -> Option<&crate::integer::LimbScratch> {
        self.limb_scratch.as_ref()
    }

    /// Retain the pool actually prepared by Taylor evaluation, if any.
    #[must_use]
    pub fn taylor_scratch(&self) -> Option<TaylorScratch> {
        self.taylor_scratch.clone()
    }

    /// Borrow coefficient buffers whose complete heap is admitted by an
    /// enclosing worker. A larger requested shape prepares a separate pool;
    /// it never resizes this borrowed allocation.
    /// # Errors
    /// Refuses replacing an already installed pool.
    pub fn set_borrowed_taylor_scratch(&mut self, scratch: TaylorScratch) -> Result<(), MathError> {
        if self.taylor_scratch.is_some() {
            return Err(MathError::Domain("Taylor scratch already installed"));
        }
        self.taylor_scratch = Some(scratch);
        Ok(())
    }

    pub(crate) fn integer_copy(&self, value: &BigInt) -> Result<BigInt, MathError> {
        match &self.limb_scratch {
            Some(scratch) => value.copy_in(scratch).map_err(MathError::LimbScratch),
            None => Ok(value.clone()),
        }
    }
    pub(crate) fn integer_add(&self, a: &BigInt, b: &BigInt) -> Result<BigInt, MathError> {
        match &self.limb_scratch {
            Some(scratch) => a.add_in(b, scratch).map_err(MathError::LimbScratch),
            None => Ok(a.add(b)),
        }
    }
    pub(crate) fn integer_sub(&self, a: &BigInt, b: &BigInt) -> Result<BigInt, MathError> {
        match &self.limb_scratch {
            Some(scratch) => a.sub_in(b, scratch).map_err(MathError::LimbScratch),
            None => Ok(a.sub(b)),
        }
    }
    pub(crate) fn integer_mul(&self, a: &BigInt, b: &BigInt) -> Result<BigInt, MathError> {
        match &self.limb_scratch {
            Some(scratch) => a.mul_in(b, scratch).map_err(MathError::LimbScratch),
            None => Ok(a.mul(b)),
        }
    }
    pub(crate) fn integer_div_rem(
        &self,
        a: &BigInt,
        b: &BigInt,
    ) -> Result<Option<(BigInt, BigInt)>, MathError> {
        match &self.limb_scratch {
            Some(scratch) => a.div_rem_in(b, scratch).map_err(MathError::LimbScratch),
            None => Ok(a.div_rem(b)),
        }
    }
    pub(crate) fn integer_pow2(&self, a: &BigInt, exponent: u32) -> Result<BigInt, MathError> {
        match &self.limb_scratch {
            Some(scratch) => a
                .mul_pow2_in(exponent, scratch)
                .map_err(MathError::LimbScratch),
            None => Ok(a.mul_pow2(exponent)),
        }
    }
    pub(crate) fn integer_pow10(&self, a: &BigInt, exponent: u32) -> Result<BigInt, MathError> {
        match &self.limb_scratch {
            Some(scratch) => a
                .mul_pow10_in(exponent, scratch)
                .map_err(MathError::LimbScratch),
            None => Ok(a.mul_pow10(exponent)),
        }
    }
    pub(crate) fn integer_sqrt_product(
        &self,
        a: &BigInt,
        b: &BigInt,
    ) -> Result<Option<BigInt>, MathError> {
        match &self.limb_scratch {
            Some(scratch) => a
                .sqrt_product_floor_in(b, scratch)
                .map_err(MathError::LimbScratch),
            None => Ok(a.sqrt_product_floor(b)),
        }
    }
    pub(crate) fn integer_product_div_rem(
        &self,
        a: &BigInt,
        b: &BigInt,
        d: &BigInt,
    ) -> Result<Option<(BigInt, BigInt)>, MathError> {
        match &self.limb_scratch {
            Some(scratch) => a
                .mul_div_rem_in(b, d, scratch)
                .map_err(MathError::LimbScratch),
            None => Ok(a.mul_div_rem(b, d)),
        }
    }
    pub(crate) fn compare_products(
        &self,
        a: &BigInt,
        b: &BigInt,
        c: &BigInt,
        d: &BigInt,
    ) -> Result<core::cmp::Ordering, MathError> {
        use crate::integer::Int;
        match &self.limb_scratch {
            Some(scratch) => Int::compare_products_in(
                a.as_integer(),
                b.as_integer(),
                c.as_integer(),
                d.as_integer(),
                scratch,
            )
            .map_err(MathError::LimbScratch),
            None => Ok(Int::compare_products(
                a.as_integer(),
                b.as_integer(),
                c.as_integer(),
                d.as_integer(),
            )),
        }
    }

    /// The admitted limits.
    #[must_use]
    pub const fn limits(&self) -> MathLimits {
        self.limits
    }

    /// Work charged since admission, including generated constants.
    #[must_use]
    pub const fn work_used(&self) -> u64 {
        self.work
    }

    /// Conservative peak numerical workspace admitted by this context.
    #[must_use]
    pub const fn workspace_peak(&self) -> usize {
        self.workspace_peak
    }

    /// Bytes explicitly reserved for caller-retained coefficients and caches.
    #[must_use]
    pub const fn workspace_reserved(&self) -> usize {
        self.workspace_reserved
    }

    /// Execution counts since context admission.
    #[must_use]
    pub const fn execution_stats(&self) -> MathExecutionStats {
        self.execution_stats
    }

    /// Admit a bounded exact-arithmetic proof chunk before computing it.
    /// The caller supplies a proved upper bound for operand and temporary
    /// growth and polls external cancellation between admitted chunks. The
    /// existing numerical work, memory and cancellation rules apply unchanged.
    ///
    /// # Errors
    /// Refuses cancelled or exhausted work/workspace before exact operations.
    pub fn admit_exact(
        &mut self,
        arithmetic_items: u64,
        max_operand_bits: usize,
    ) -> Result<(), MathError> {
        self.charge(arithmetic_items, max_operand_bits)
    }

    /// Admit one complete original integer/rational body with shared checked
    /// limb-work, result-width and scratch bounds before doing arithmetic.
    /// The scratch allowance is temporary and does not retain a reservation.
    /// # Errors
    /// Refuses cancellation or exhausted/overflowing work and workspace atomically.
    pub fn admit_exact_cost(
        &mut self,
        cost: crate::integer::ExactArithmeticCost,
    ) -> Result<(), MathError> {
        let bits = usize::try_from(cost.output_bits).map_err(|_| MathError::WorkspaceExhausted)?;
        let bytes =
            usize::try_from(cost.workspace_bytes).map_err(|_| MathError::WorkspaceExhausted)?;
        let total = bytes
            .checked_add(self.workspace_reserved)
            .ok_or(MathError::WorkspaceExhausted)?;
        if total > self.limits.max_workspace_bytes {
            return Err(MathError::WorkspaceExhausted);
        }
        self.charge(cost.work_items, bits)?;
        self.workspace_peak = self.workspace_peak.max(total);
        Ok(())
    }

    /// Reserve retained numerical storage before allocating it. Scratch admission
    /// adds this reservation to each intermediate bound.
    ///
    /// # Errors
    /// Refuses an overflowing or excessive workspace reservation.
    pub fn reserve_workspace(&mut self, bytes: usize) -> Result<(), MathError> {
        let reserved = self
            .workspace_reserved
            .checked_add(bytes)
            .ok_or(MathError::WorkspaceExhausted)?;
        let scratch = (self.limits.precision_bits as usize)
            .div_ceil(8)
            .saturating_mul(32)
            .saturating_add(SCRATCH_OVERHEAD);
        let total = reserved
            .checked_add(scratch)
            .ok_or(MathError::WorkspaceExhausted)?;
        if total > self.limits.max_workspace_bytes {
            return Err(MathError::WorkspaceExhausted);
        }
        self.workspace_reserved = reserved;
        self.workspace_peak = self.workspace_peak.max(total);
        Ok(())
    }

    /// Release a completed retained-storage reservation.
    ///
    /// # Errors
    /// Refuses releasing more than the currently reserved storage.
    pub fn release_workspace(&mut self, bytes: usize) -> Result<(), MathError> {
        self.workspace_reserved = self
            .workspace_reserved
            .checked_sub(bytes)
            .ok_or(MathError::Domain("unbalanced workspace release"))?;
        Ok(())
    }

    /// Attach an explicit cancellation flag; no ambient process state is read.
    pub fn set_cancellation(&mut self, flag: Arc<AtomicBool>) {
        self.cancelled = Some(flag);
    }

    /// Force one available equivalent product path for this worker. The choice
    /// persists through chunk pauses, with no ambient override or held guard.
    ///
    /// # Errors
    /// Refuses an implementation unavailable on this build or processor.
    pub fn set_binary64_backend(&mut self, backend: FloatProductBackend) -> Result<(), MathError> {
        if !backend.is_available() {
            return Err(MathError::Domain("unavailable binary64 product backend"));
        }
        self.binary64_backend = backend;
        Ok(())
    }

    /// The worker's admitted equivalent product path.
    #[must_use]
    pub const fn binary64_backend(&self) -> FloatProductBackend {
        self.binary64_backend
    }

    /// Revalidate and enter a bounded floating chunk. Drop it before callbacks,
    /// suspension, or migration to another worker.
    ///
    /// # Errors
    ///
    /// Returns cancellation, work, or structured floating-environment evidence.
    pub fn enter_chunk(&mut self) -> Result<CheckedBinary64, MathError> {
        self.charge(1, self.limits.precision_bits as usize)?;
        environment::check().map_err(MathError::Environment)?;
        if self.binary64_backend != FloatProductBackend::Portable {
            environment::check_vector().map_err(MathError::Environment)?;
        }
        Ok(CheckedBinary64 {
            scope: Binary64Scope::enter(),
            backend: self.binary64_backend,
        })
    }

    fn charge(&mut self, work: u64, bits: usize) -> Result<(), MathError> {
        if self
            .cancelled
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            return Err(MathError::Cancelled);
        }
        let next = self
            .work
            .checked_add(work)
            .ok_or(MathError::WorkExhausted)?;
        if next > self.limits.max_work {
            return Err(MathError::WorkExhausted);
        }
        // A product/division keeps endpoint operands, four products, quotient,
        // remainder and carry buffers. 32 times the bit payload bounds those
        // shared binary-limb buffers and retained enclosures conservatively.
        let bytes = bits
            .div_ceil(8)
            .checked_mul(32)
            .and_then(|bytes| bytes.checked_add(SCRATCH_OVERHEAD))
            .and_then(|bytes| bytes.checked_add(self.workspace_reserved))
            .ok_or(MathError::WorkspaceExhausted)?;
        if bytes > self.limits.max_workspace_bytes {
            return Err(MathError::WorkspaceExhausted);
        }
        self.work = next;
        self.workspace_peak = self.workspace_peak.max(bytes);
        Ok(())
    }
}
