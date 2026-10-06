// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Bounded interval Taylor arithmetic on a caller-admitted reusable coefficient pool.
//!
//! Coefficient n encloses the nth derivative divided by n!, at every point
//! represented by the argument's constant interval. This is a Taylor-theorem
//! remainder bound, not an estimate from differences between quadrature rules.
//! Brands prevent a jet from escaping its workspace; generations refuse a stale
//! handle after scratch rewind. Operations allocate no coefficient vectors.

use core::marker::PhantomData;

use super::{CoordinateMath, FixedInterval, MathError};

mod scratch;
use scratch::ScratchBuffers;
pub use scratch::{TaylorScratch, TaylorScratchError};

/// A branded handle to coefficients retained in one bounded workspace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaylorJet<'scope> {
    slot: usize,
    generation: u64,
    brand: PhantomData<&'scope mut &'scope ()>,
}

/// Fixed-order coefficient storage admitted before allocation.
#[derive(Debug)]
pub struct TaylorWorkspace<'scope> {
    order: usize,
    capacity: usize,
    used: usize,
    sequence: u64,
    high_water: usize,
    coefficients: Box<[FixedInterval]>,
    generations: Box<[u64]>,
    zero: FixedInterval,
    scratch: TaylorScratch,
    brand: PhantomData<&'scope mut &'scope ()>,
}

struct TaylorReservation<'math> {
    math: &'math mut CoordinateMath,
    bytes: usize,
}

impl TaylorReservation<'_> {
    fn release(mut self) -> Result<(), MathError> {
        let bytes = core::mem::take(&mut self.bytes);
        self.math.release_workspace(bytes)
    }
}

impl Drop for TaylorReservation<'_> {
    fn drop(&mut self) {
        // Normal completion reports a corrupted reservation as a typed error.
        // On unwinding, release our own temporary admission without masking the
        // original panic if a callback has altered its parent's counters.
        let _ = self.math.release_workspace(self.bytes);
    }
}

/// Run bounded Taylor arithmetic and release all scratch on either success or refusal.
///
/// # Errors
/// Refuses count overflow, insufficient work/workspace, invalid arithmetic and
/// a closure that exceeds its admitted live-jet capacity. No jet can escape.
pub fn with_taylor_workspace<T>(
    order: usize,
    live_jets: usize,
    math: &mut CoordinateMath,
    evaluate: impl for<'scope> FnOnce(
        &mut TaylorWorkspace<'scope>,
        &mut CoordinateMath,
    ) -> Result<T, MathError>,
) -> Result<T, MathError> {
    with_taylor_workspace_observed(
        order,
        live_jets,
        math,
        &mut (),
        |_, ()| Ok(()),
        |workspace, math, ()| evaluate(workspace, math),
    )
}

/// Prepare/check out the same bounded pool after exposing its admission to an
/// explicit observer. The observation precedes allocation and holds no lock.
/// State is borrowed sequentially by the observer and branded evaluator.
///
/// # Errors
/// Adds the observer's original refusal and simultaneous pool checkout to
/// [`with_taylor_workspace`]'s arithmetic and admission errors.
pub fn with_taylor_workspace_observed<T, S>(
    order: usize,
    live_jets: usize,
    math: &mut CoordinateMath,
    state: &mut S,
    mut observe: impl FnMut(&CoordinateMath, &mut S) -> Result<(), MathError>,
    evaluate: impl for<'scope> FnOnce(
        &mut TaylorWorkspace<'scope>,
        &mut CoordinateMath,
        &mut S,
    ) -> Result<T, MathError>,
) -> Result<T, MathError> {
    let width = order.checked_add(1).ok_or(MathError::WorkspaceExhausted)?;
    let count = width
        .checked_mul(live_jets)
        .ok_or(MathError::WorkspaceExhausted)?;
    let endpoint_storage = (math.limits().precision_bits as usize)
        .div_ceil(8)
        .checked_mul(8)
        .ok_or(MathError::WorkspaceExhausted)?;
    let coefficient_bytes = size_of::<FixedInterval>()
        .checked_add(endpoint_storage)
        .and_then(|value| value.checked_add(128))
        .ok_or(MathError::WorkspaceExhausted)?;
    let bytes = count
        .checked_mul(coefficient_bytes)
        .and_then(|value| value.checked_add(live_jets.checked_mul(size_of::<u64>())?))
        .and_then(|value| value.checked_add(size_of::<TaylorWorkspace<'_>>() + 128))
        .ok_or(MathError::WorkspaceExhausted)?;
    math.reserve_workspace(bytes)?;
    let reservation = TaylorReservation { math, bytes };
    let math = &mut *reservation.math;
    let result = (|| {
        // Initialization and unconditional cleanup each visit at most the
        // requested count. Admit those exact linear copies before
        // entering a closure that may hold many live interval coefficients.
        let copies = count
            .checked_mul(2)
            .and_then(|value| value.checked_add(live_jets))
            .and_then(|value| u64::try_from(value).ok())
            .ok_or(MathError::WorkExhausted)?;
        math.admit_exact(copies, 1)?;
        let zero = FixedInterval::from_i64(0, math)?;
        let existing = math.taylor_scratch.clone();
        if let Some(scratch) = &existing {
            scratch.available()?;
        }
        let scratch =
            if let Some(scratch) = existing.filter(|value| value.supports(count, live_jets)) {
                observe(math, state)?;
                scratch
            } else {
                let retained = TaylorScratch::required_bytes(count, live_jets)
                    .ok_or(MathError::WorkspaceExhausted)?;
                math.reserve_workspace(retained)?;
                let prepared = observe(math, state)
                    .and_then(|()| TaylorScratch::new(count, live_jets, zero.clone()));
                let prepared = match prepared {
                    Ok(prepared) => prepared,
                    Err(error) => {
                        math.release_workspace(retained)?;
                        return Err(error);
                    }
                };
                math.release_workspace(math.taylor_scratch_owned_bytes)?;
                math.taylor_scratch_owned_bytes = retained;
                math.taylor_scratch = Some(prepared.clone());
                prepared
            };
        let buffers = scratch.checkout()?;
        let mut workspace = TaylorWorkspace {
            order,
            capacity: live_jets,
            used: 0,
            sequence: 0,
            high_water: 0,
            coefficients: buffers.coefficients,
            generations: buffers.generations,
            zero,
            scratch,
            brand: PhantomData,
        };
        evaluate(&mut workspace, math, state)
    })();
    reservation.release()?;
    result
}

impl Drop for TaylorWorkspace<'_> {
    fn drop(&mut self) {
        // No arithmetic, callbacks or allocation occur while unwinding. Slots
        // above used may have been rewound; high_water includes them as well.
        let count = self.high_water * (self.order + 1);
        self.coefficients[..count].fill(self.zero.clone());
        self.generations[..self.high_water].fill(0);
        self.scratch.return_buffers(ScratchBuffers {
            coefficients: core::mem::take(&mut self.coefficients),
            generations: core::mem::take(&mut self.generations),
        });
    }
}

impl<'scope> TaylorWorkspace<'scope> {
    /// The greatest represented derivative order.
    #[must_use]
    pub const fn order(&self) -> usize {
        self.order
    }

    /// Current live-jet count; retain this mark to rewind temporary expressions.
    #[must_use]
    pub const fn checkpoint(&self) -> usize {
        self.used
    }

    /// Discard jets allocated at or after a valid earlier checkpoint.
    ///
    /// # Errors
    /// Refuses a checkpoint beyond current allocation. Existing discarded handles
    /// become stale, including when their slots are subsequently reused.
    pub fn rewind(&mut self, checkpoint: usize) -> Result<(), MathError> {
        if checkpoint > self.used {
            return Err(MathError::Domain("invalid Taylor scratch checkpoint"));
        }
        for generation in &mut self.generations[checkpoint..self.used] {
            *generation = 0;
        }
        self.used = checkpoint;
        Ok(())
    }

    fn allocate(&mut self, math: &mut CoordinateMath) -> Result<TaylorJet<'scope>, MathError> {
        if self.used == self.capacity {
            return Err(MathError::WorkspaceExhausted);
        }
        // Rewinding permits arbitrarily many sequential slot reuses. Charge
        // each reset, including its generation entry, before touching storage;
        // the one setup reservation cannot bound repeated callback arithmetic.
        math.admit_exact(
            u64::try_from(self.order)
                .ok()
                .and_then(|order| order.checked_add(2))
                .ok_or(MathError::WorkExhausted)?,
            1,
        )?;
        self.sequence = self
            .sequence
            .checked_add(1)
            .ok_or(MathError::WorkExhausted)?;
        let slot = self.used;
        self.used += 1;
        self.high_water = self.high_water.max(self.used);
        self.generations[slot] = self.sequence;
        let zero = FixedInterval::from_i64(0, math)?;
        let start = slot * (self.order + 1);
        self.coefficients[start..=start + self.order].fill(zero);
        Ok(TaylorJet {
            slot,
            generation: self.sequence,
            brand: PhantomData,
        })
    }

    fn validate(&self, jet: TaylorJet<'scope>) -> Result<usize, MathError> {
        if jet.slot >= self.used || self.generations[jet.slot] != jet.generation {
            return Err(MathError::Domain("stale Taylor coefficient handle"));
        }
        Ok(jet.slot * (self.order + 1))
    }

    /// One coefficient enclosing derivative/order-factorial.
    ///
    /// # Errors
    /// Refuses a stale handle or an order beyond the workspace's declared order.
    pub fn coefficient(
        &self,
        jet: TaylorJet<'scope>,
        order: usize,
    ) -> Result<&FixedInterval, MathError> {
        if order > self.order {
            return Err(MathError::Domain("Taylor derivative beyond declared order"));
        }
        Ok(&self.coefficients[self.validate(jet)? + order])
    }

    /// Store a separately proved normalized derivative coefficient in a live
    /// jet. The caller's recurrence supplies the mathematical proof; this
    /// method enforces the original grid, brand and declared order.
    ///
    /// # Errors
    /// Refuses stale handles, an excessive order or a different precision grid.
    pub fn set_coefficient(
        &mut self,
        jet: TaylorJet<'scope>,
        order: usize,
        value: FixedInterval,
        math: &mut CoordinateMath,
    ) -> Result<(), MathError> {
        let start = self.validate(jet)?;
        if order > self.order {
            return Err(MathError::Domain("Taylor derivative beyond declared order"));
        }
        if value.precision_bits() != math.limits().precision_bits {
            return Err(MathError::Domain("Taylor coefficient precision differs"));
        }
        math.admit_exact(1, 1)?;
        self.coefficients[start + order] = value;
        Ok(())
    }

    fn store(&mut self, jet: TaylorJet<'scope>, order: usize, value: FixedInterval) {
        self.coefficients[jet.slot * (self.order + 1) + order] = value;
    }

    /// Constant interval with every higher derivative zero.
    ///
    /// # Errors
    /// Refuses exhausted scratch/work admission.
    pub fn constant(
        &mut self,
        value: FixedInterval,
        math: &mut CoordinateMath,
    ) -> Result<TaylorJet<'scope>, MathError> {
        let jet = self.allocate(math)?;
        self.store(jet, 0, value);
        Ok(jet)
    }

    /// Affine argument with an interval constant and exact/enclosed first derivative.
    ///
    /// # Errors
    /// Refuses a first derivative in an order-zero pool and exhausted admission.
    pub fn argument(
        &mut self,
        value: FixedInterval,
        derivative: FixedInterval,
        math: &mut CoordinateMath,
    ) -> Result<TaylorJet<'scope>, MathError> {
        if self.order == 0 {
            return Err(MathError::Domain(
                "Taylor affine argument requires positive order",
            ));
        }
        let jet = self.constant(value, math)?;
        self.store(jet, 1, derivative);
        Ok(jet)
    }

    /// Truncated antiderivative with a separately proved constant coefficient.
    /// Coefficient n is `value[n-1]/n`; the highest input coefficient is not
    /// represented because its integral has order `self.order()+1`.
    ///
    /// # Errors
    /// Refuses stale handles and arithmetic/admission failures. A caller using
    /// this in a differential recurrence must prove the supplied input jet's
    /// lower coefficients before interpreting the result as derivatives.
    pub fn integral(
        &mut self,
        value: TaylorJet<'scope>,
        constant: FixedInterval,
        math: &mut CoordinateMath,
    ) -> Result<TaylorJet<'scope>, MathError> {
        self.validate(value)?;
        let result = self.constant(constant, math)?;
        for n in 1..=self.order {
            let divisor = FixedInterval::from_i64(
                i64::try_from(n).map_err(|_| MathError::WorkExhausted)?,
                math,
            )?;
            let coefficient = self.coefficient(value, n - 1)?.div(&divisor, math)?;
            self.store(result, n, coefficient);
        }
        Ok(result)
    }

    /// Copy a proved jet into an existing live slot before rewinding scratch.
    /// Both handles retain their brand and generation; no handle is allocated.
    ///
    /// # Errors
    /// Refuses either stale handle and exhausted arithmetic admission.
    pub fn assign(
        &mut self,
        target: TaylorJet<'scope>,
        source: TaylorJet<'scope>,
        math: &mut CoordinateMath,
    ) -> Result<(), MathError> {
        let target_start = self.validate(target)?;
        let source_start = self.validate(source)?;
        if target_start == source_start {
            return Ok(());
        }
        let bits = self.coefficients[source_start..=source_start + self.order]
            .iter()
            .map(|coefficient| {
                coefficient
                    .lower()
                    .bits_upper_bound()
                    .max(coefficient.upper().bits_upper_bound())
            })
            .max()
            .unwrap_or(0);
        math.admit_exact(
            u64::try_from(self.order + 1).map_err(|_| MathError::WorkExhausted)?,
            bits,
        )?;
        for n in 0..=self.order {
            self.coefficients[target_start + n] = self.coefficients[source_start + n].clone();
        }
        Ok(())
    }

    /// Coefficient-wise sum of two jets.
    ///
    /// # Errors
    /// Refuses stale handles and arithmetic/admission failures.
    pub fn add(
        &mut self,
        left: TaylorJet<'scope>,
        right: TaylorJet<'scope>,
        math: &mut CoordinateMath,
    ) -> Result<TaylorJet<'scope>, MathError> {
        self.validate(left)?;
        self.validate(right)?;
        let result = self.allocate(math)?;
        for n in 0..=self.order {
            let value = self
                .coefficient(left, n)?
                .add(self.coefficient(right, n)?, math)?;
            self.store(result, n, value);
        }
        Ok(result)
    }

    /// Coefficient-wise difference of two jets.
    ///
    /// # Errors
    /// Refuses stale handles and arithmetic/admission failures.
    pub fn sub(
        &mut self,
        left: TaylorJet<'scope>,
        right: TaylorJet<'scope>,
        math: &mut CoordinateMath,
    ) -> Result<TaylorJet<'scope>, MathError> {
        self.validate(left)?;
        self.validate(right)?;
        let result = self.allocate(math)?;
        for n in 0..=self.order {
            let value = self
                .coefficient(left, n)?
                .sub(self.coefficient(right, n)?, math)?;
            self.store(result, n, value);
        }
        Ok(result)
    }

    /// Multiply by a constant enclosure without allocating a constant jet.
    ///
    /// # Errors
    /// Refuses stale handles and arithmetic/admission failures.
    pub fn scale(
        &mut self,
        value: TaylorJet<'scope>,
        factor: &FixedInterval,
        math: &mut CoordinateMath,
    ) -> Result<TaylorJet<'scope>, MathError> {
        self.validate(value)?;
        let result = self.allocate(math)?;
        for n in 0..=self.order {
            let coefficient = self.coefficient(value, n)?.mul(factor, math)?;
            self.store(result, n, coefficient);
        }
        Ok(result)
    }

    /// Cauchy product; an identical handle preserves the zeroth coefficient's square.
    ///
    /// # Errors
    /// Refuses stale handles and arithmetic/admission failures.
    pub fn mul(
        &mut self,
        left: TaylorJet<'scope>,
        right: TaylorJet<'scope>,
        math: &mut CoordinateMath,
    ) -> Result<TaylorJet<'scope>, MathError> {
        self.validate(left)?;
        self.validate(right)?;
        let result = self.allocate(math)?;
        for n in 0..=self.order {
            let value = if n == 0 && left == right {
                self.coefficient(left, 0)?.square(math)?
            } else {
                let mut sum = FixedInterval::from_i64(0, math)?;
                for j in 0..=n {
                    sum = sum.add(
                        &self
                            .coefficient(left, j)?
                            .mul(self.coefficient(right, n - j)?, math)?,
                        math,
                    )?;
                }
                sum
            };
            self.store(result, n, value);
        }
        Ok(result)
    }

    /// Quotient recurrence from `left = right * result`.
    ///
    /// # Errors
    /// Refuses an unresolved/zero divisor, stale handles and admission failures.
    pub fn div(
        &mut self,
        left: TaylorJet<'scope>,
        right: TaylorJet<'scope>,
        math: &mut CoordinateMath,
    ) -> Result<TaylorJet<'scope>, MathError> {
        self.validate(left)?;
        self.validate(right)?;
        let result = self.allocate(math)?;
        for n in 0..=self.order {
            let mut value = self.coefficient(left, n)?.clone();
            for j in 1..=n {
                value = value.sub(
                    &self
                        .coefficient(right, j)?
                        .mul(self.coefficient(result, n - j)?, math)?,
                    math,
                )?;
            }
            value = value.div(self.coefficient(right, 0)?, math)?;
            self.store(result, n, value);
        }
        Ok(result)
    }

    /// Positive square-root recurrence from `result² = value`.
    ///
    /// # Errors
    /// Refuses unresolved zero/domain singularities, stale handles and admission failures.
    pub fn sqrt(
        &mut self,
        value: TaylorJet<'scope>,
        math: &mut CoordinateMath,
    ) -> Result<TaylorJet<'scope>, MathError> {
        self.validate(value)?;
        let result = self.allocate(math)?;
        let first = self.coefficient(value, 0)?.sqrt(math)?;
        self.store(result, 0, first.clone());
        let denominator = first.mul(&FixedInterval::from_i64(2, math)?, math)?;
        for n in 1..=self.order {
            let mut coefficient = self.coefficient(value, n)?.clone();
            for j in 1..n {
                coefficient = coefficient.sub(
                    &self
                        .coefficient(result, j)?
                        .mul(self.coefficient(result, n - j)?, math)?,
                    math,
                )?;
            }
            self.store(result, n, coefficient.div(&denominator, math)?);
        }
        Ok(result)
    }

    /// Coupled sine/cosine recurrences from their exact derivatives.
    ///
    /// # Errors
    /// Refuses stale handles and arithmetic/admission failures.
    pub fn sin_cos(
        &mut self,
        value: TaylorJet<'scope>,
        math: &mut CoordinateMath,
    ) -> Result<(TaylorJet<'scope>, TaylorJet<'scope>), MathError> {
        self.validate(value)?;
        let sine = self.allocate(math)?;
        let cosine = self.allocate(math)?;
        let (s0, c0) = self.coefficient(value, 0)?.sin_cos_range(math)?;
        self.store(sine, 0, s0);
        self.store(cosine, 0, c0);
        for n in 1..=self.order {
            let mut ss = FixedInterval::from_i64(0, math)?;
            let mut cc = FixedInterval::from_i64(0, math)?;
            for j in 1..=n {
                let derivative = self
                    .coefficient(value, j)?
                    .mul(&FixedInterval::from_i64(j as i64, math)?, math)?;
                ss = ss.add(
                    &derivative.mul(self.coefficient(cosine, n - j)?, math)?,
                    math,
                )?;
                cc = cc.sub(&derivative.mul(self.coefficient(sine, n - j)?, math)?, math)?;
            }
            let count = FixedInterval::from_i64(n as i64, math)?;
            self.store(sine, n, ss.div(&count, math)?);
            self.store(cosine, n, cc.div(&count, math)?);
        }
        Ok((sine, cosine))
    }

    /// Natural logarithm via `(log value)' = value'/value`.
    ///
    /// # Errors
    /// Refuses a nonpositive/singular interval, stale handles and admission failures.
    pub fn log(
        &mut self,
        value: TaylorJet<'scope>,
        math: &mut CoordinateMath,
    ) -> Result<TaylorJet<'scope>, MathError> {
        self.validate(value)?;
        let result = self.allocate(math)?;
        let first = self.coefficient(value, 0)?.log(math)?;
        self.store(result, 0, first);
        let derivative = self.allocate(math)?;
        for n in 0..self.order {
            let coefficient = self
                .coefficient(value, n + 1)?
                .mul(&FixedInterval::from_i64((n + 1) as i64, math)?, math)?;
            self.store(derivative, n, coefficient);
        }
        let quotient = self.div(derivative, value, math)?;
        for n in 1..=self.order {
            let coefficient = self
                .coefficient(quotient, n - 1)?
                .div(&FixedInterval::from_i64(n as i64, math)?, math)?;
            self.store(result, n, coefficient);
        }
        Ok(result)
    }

    /// Exponential via `(exp value)' = value' * exp value`.
    ///
    /// # Errors
    /// Refuses stale handles and arithmetic/admission failures.
    pub fn exp(
        &mut self,
        value: TaylorJet<'scope>,
        math: &mut CoordinateMath,
    ) -> Result<TaylorJet<'scope>, MathError> {
        self.validate(value)?;
        let result = self.allocate(math)?;
        let first = self.coefficient(value, 0)?.exp(math)?;
        self.store(result, 0, first);
        for n in 1..=self.order {
            let mut coefficient = FixedInterval::from_i64(0, math)?;
            for j in 1..=n {
                coefficient = coefficient.add(
                    &self
                        .coefficient(value, j)?
                        .mul(&FixedInterval::from_i64(j as i64, math)?, math)?
                        .mul(self.coefficient(result, n - j)?, math)?,
                    math,
                )?;
            }
            self.store(
                result,
                n,
                coefficient.div(&FixedInterval::from_i64(n as i64, math)?, math)?,
            );
        }
        Ok(result)
    }
}

/// A complete closed symmetric quadrature panel on one exact fixed grid.
/// The center and half width are point intervals; construction checks that they
/// reconstruct the complete panel exactly, including both outward endpoints.
#[derive(Clone, Copy, Debug)]
pub struct SymmetricTaylorPanel<'a> {
    center: &'a FixedInterval,
    interval: &'a FixedInterval,
    half_width: &'a FixedInterval,
}
impl<'a> SymmetricTaylorPanel<'a> {
    /// Validate a complete panel before applying symmetric Taylor moments.
    ///
    /// # Errors
    /// Refuses different grids, nonpoint center/width, negative width, an
    /// asymmetric panel, or exhausted arithmetic admission.
    pub fn new(
        center: &'a FixedInterval,
        interval: &'a FixedInterval,
        half_width: &'a FixedInterval,
        math: &mut CoordinateMath,
    ) -> Result<Self, MathError> {
        if center.lower() != center.upper()
            || half_width.lower() != half_width.upper()
            || half_width.lower().is_negative()
        {
            return Err(MathError::Domain(
                "Taylor panel requires exact center and half width",
            ));
        }
        let lower = center.sub(half_width, math)?;
        let upper = center.add(half_width, math)?;
        if lower.lower() != interval.lower()
            || upper.upper() != interval.upper()
            || interval.precision_bits() != math.limits().precision_bits
        {
            return Err(MathError::Domain("asymmetric Taylor quadrature panel"));
        }
        Ok(Self {
            center,
            interval,
            half_width,
        })
    }
}

/// Integrate a positive even-order Taylor jet over a complete symmetric panel.
///
/// The callback returns normalized derivative coefficients at the exact center
/// and over the whole panel. Center coefficients of orders below `order` provide
/// the even moments; the complete-panel coefficient of order `order` bounds the
/// Taylor remainder. Their computation must follow the original mathematical
/// law; successive heuristic quadrature differences are never certificates.
///
/// # Errors
/// Refuses invalid order, arithmetic, scratch or callback admission. No jet
/// escapes the bounded coefficient pool or survives its scratch rewind.
pub fn integrate_taylor_panel(
    order: usize,
    live_jets: usize,
    panel: SymmetricTaylorPanel<'_>,
    math: &mut CoordinateMath,
    mut evaluate: impl for<'scope> FnMut(
        &FixedInterval,
        &mut TaylorWorkspace<'scope>,
        &mut CoordinateMath,
    ) -> Result<TaylorJet<'scope>, MathError>,
) -> Result<(FixedInterval, FixedInterval), MathError> {
    integrate_taylor_panel_observed(
        order,
        live_jets,
        panel,
        math,
        &mut (),
        |_, ()| Ok(()),
        |parameter, workspace, math, ()| evaluate(parameter, workspace, math),
    )
}

/// Integrate the same complete Taylor panel after observing coefficient-pool
/// admission before setup allocation. No lock survives either callback.
///
/// # Errors
/// Adds the observer's original refusal to [`integrate_taylor_panel`].
pub fn integrate_taylor_panel_observed<S>(
    order: usize,
    live_jets: usize,
    panel: SymmetricTaylorPanel<'_>,
    math: &mut CoordinateMath,
    state: &mut S,
    observe: impl FnMut(&CoordinateMath, &mut S) -> Result<(), MathError>,
    mut evaluate: impl for<'scope> FnMut(
        &FixedInterval,
        &mut TaylorWorkspace<'scope>,
        &mut CoordinateMath,
        &mut S,
    ) -> Result<TaylorJet<'scope>, MathError>,
) -> Result<(FixedInterval, FixedInterval), MathError> {
    if order == 0 || !order.is_multiple_of(2) {
        return Err(MathError::Domain(
            "Taylor quadrature order must be positive and even",
        ));
    }
    with_taylor_workspace_observed(
        order,
        live_jets,
        math,
        state,
        observe,
        |workspace, math, state| {
            let centre = evaluate(panel.center, workspace, math, state)?;
            let mut integral = FixedInterval::from_i64(0, math)?;
            let square_half = panel.half_width.square(math)?;
            let mut power = panel.half_width.clone();
            for n in (0..order).step_by(2) {
                let coefficient = workspace.coefficient(centre, n)?;
                integral = integral.add(
                    &coefficient
                        .mul(&power, math)?
                        .mul(&FixedInterval::from_i64(2, math)?, math)?
                        .div(
                            &FixedInterval::from_i64(
                                i64::try_from(n + 1).map_err(|_| MathError::WorkspaceExhausted)?,
                                math,
                            )?,
                            math,
                        )?,
                    math,
                )?;
                power = power.mul(&square_half, math)?;
            }
            workspace.rewind(0)?;
            let enclosure = evaluate(panel.interval, workspace, math, state)?;
            let derivative = workspace.coefficient(enclosure, order)?.abs(math)?;
            let remainder = derivative
                .mul(&power, math)?
                .mul(&FixedInterval::from_i64(2, math)?, math)?
                .div(
                    &FixedInterval::from_i64(
                        i64::try_from(order + 1).map_err(|_| MathError::WorkspaceExhausted)?,
                        math,
                    )?,
                    math,
                )?;
            Ok((integral, remainder))
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BigInt;
    use crate::math::MathLimits;

    fn context() -> CoordinateMath {
        CoordinateMath::new(MathLimits {
            precision_bits: 96,
            max_work: 100_000,
            max_workspace_bytes: 1_048_576,
        })
        .unwrap()
    }

    #[test]
    fn exact_sine_coefficients_and_derivative_enclosures() {
        let mut math = context();
        FixedInterval::pi(&mut math).unwrap();
        FixedInterval::from_i64(1, &mut math)
            .unwrap()
            .exp(&mut math)
            .unwrap();
        let persistent = math.workspace_reserved();
        with_taylor_workspace(8, 12, &mut math, |workspace, math| {
            let zero = FixedInterval::from_i64(0, math)?;
            let one = FixedInterval::from_i64(1, math)?;
            let variable = workspace.argument(zero, one, math)?;
            let (sine, cosine) = workspace.sin_cos(variable, math)?;
            for (jet, order, numerator, denominator) in [
                (sine, 1, 1, 1),
                (sine, 3, -1, 6),
                (sine, 5, 1, 120),
                (cosine, 2, -1, 2),
                (cosine, 4, 1, 24),
                (cosine, 6, -1, 720),
            ] {
                let expected = FixedInterval::from_ratio(
                    &BigInt::from_i128(numerator),
                    &BigInt::from_i128(denominator),
                    math,
                )?;
                let actual = workspace.coefficient(jet, order)?;
                assert!(actual.lower() <= expected.lower() && actual.upper() >= expected.upper());
            }
            let mark = workspace.checkpoint();
            let interval = FixedInterval::from_bounds(
                BigInt::from_i128(-1).mul_pow2(96),
                BigInt::from_i128(1).mul_pow2(96),
                math,
            )?;
            let variable = workspace.argument(interval, FixedInterval::from_i64(1, math)?, math)?;
            let exponential = workspace.exp(variable, math)?;
            let sixth = workspace.coefficient(exponential, 6)?;
            let lower = FixedInterval::from_i64(-1, math)?
                .exp(math)?
                .div(&FixedInterval::from_i64(720, math)?, math)?;
            let upper = FixedInterval::from_i64(1, math)?
                .exp(math)?
                .div(&FixedInterval::from_i64(720, math)?, math)?;
            assert!(sixth.lower() <= lower.lower() && sixth.upper() >= upper.upper());
            workspace.rewind(mark)?;
            assert!(workspace.coefficient(exponential, 0).is_err());
            Ok(())
        })
        .unwrap();
        assert_eq!(
            math.workspace_reserved(),
            persistent + math.taylor_scratch().unwrap().retained_bytes()
        );
    }

    #[test]
    fn stale_handles_and_failure_release_admitted_pool() {
        let mut math = context();
        let result = with_taylor_workspace(4, 1, &mut math, |workspace, math| {
            let mark = workspace.checkpoint();
            let original = workspace.constant(FixedInterval::from_i64(7, math)?, math)?;
            workspace.rewind(mark)?;
            let replacement = workspace.constant(FixedInterval::from_i64(11, math)?, math)?;
            assert!(workspace.coefficient(original, 0).is_err());
            assert_eq!(
                workspace.coefficient(replacement, 0)?.lower(),
                &BigInt::from_i128(11).mul_pow2(96)
            );
            workspace.constant(FixedInterval::from_i64(13, math)?, math)?;
            Ok(())
        });
        assert_eq!(result, Err(MathError::WorkspaceExhausted));
        let retained = math.taylor_scratch().unwrap().retained_bytes();
        assert_eq!(math.workspace_reserved(), retained);
        with_taylor_workspace(4, 1, &mut math, |workspace, math| {
            let zero = workspace.constant(FixedInterval::from_i64(0, math)?, math)?;
            assert!(workspace.coefficient(zero, 0)?.is_exact_zero());
            Ok(())
        })
        .unwrap();
        let peak = math.workspace_peak();
        assert_eq!(
            with_taylor_workspace(usize::MAX, 2, &mut math, |_, _| Ok(())),
            Err(MathError::WorkspaceExhausted)
        );
        assert_eq!(math.workspace_peak(), peak);
    }

    #[test]
    fn pool_observation_precedes_allocation_and_nested_checkout_refuses() {
        let mut math = context();
        let refusal = with_taylor_workspace_observed(
            8,
            16,
            &mut math,
            &mut (),
            |_, ()| Err(MathError::Cancelled),
            |_, _, ()| Ok(()),
        );
        assert_eq!(refusal, Err(MathError::Cancelled));
        assert!(math.taylor_scratch().is_none());
        assert_eq!(math.workspace_reserved(), 0);
        with_taylor_workspace(4, 2, &mut math, |_, math| {
            let mut child = context();
            child.set_borrowed_taylor_scratch(math.taylor_scratch().unwrap())?;
            assert_eq!(
                with_taylor_workspace(4, 2, &mut child, |_, _| Ok(())),
                Err(MathError::TaylorScratch(TaylorScratchError::InUse))
            );
            assert_eq!(child.workspace_reserved(), 0);
            Ok(())
        })
        .unwrap();
        with_taylor_workspace(4, 2, &mut math, |_, _| Ok(())).unwrap();
    }

    #[test]
    fn a_larger_child_pool_preserves_the_borrowed_parent_receipt() {
        let mut parent = context();
        with_taylor_workspace(4, 2, &mut parent, |_, _| Ok(())).unwrap();
        let original = parent.taylor_scratch().unwrap();
        let original_bytes = parent.workspace_reserved();
        let mut child = context();
        child.set_borrowed_taylor_scratch(original.clone()).unwrap();
        with_taylor_workspace(8, 4, &mut child, |_, _| Ok(())).unwrap();
        let replacement = child.taylor_scratch().unwrap();
        assert!(!original.shares_storage(&replacement));
        assert_eq!(child.workspace_reserved(), replacement.retained_bytes());
        assert_eq!(parent.workspace_reserved(), original_bytes);
        with_taylor_workspace(4, 2, &mut parent, |_, _| Ok(())).unwrap();
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn unwinding_returns_and_clears_the_complete_checkout() {
        let mut math = context();
        let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = with_taylor_workspace::<()>(4, 2, &mut math, |workspace, math| {
                workspace.constant(FixedInterval::from_i64(7, math)?, math)?;
                panic!("coefficient callback failed");
            });
        }));
        assert!(refused.is_err());
        with_taylor_workspace(4, 2, &mut math, |workspace, math| {
            let jet = workspace.constant(FixedInterval::from_i64(3, math)?, math)?;
            for n in 1..=4 {
                assert!(workspace.coefficient(jet, n)?.is_exact_zero());
            }
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn differential_recurrence_retains_state_and_invalidates_rewound_sources() {
        let mut math = context();
        with_taylor_workspace(8, 3, &mut math, |workspace, math| {
            let one = FixedInterval::from_i64(1, math)?;
            let state = workspace.constant(one.clone(), math)?;
            let mark = workspace.checkpoint();
            for _ in 0..8 {
                let next = workspace.integral(state, one.clone(), math)?;
                workspace.assign(state, next, math)?;
                workspace.rewind(mark)?;
                assert_eq!(
                    workspace.assign(state, next, math),
                    Err(MathError::Domain("stale Taylor coefficient handle"))
                );
            }
            let mut factorial = 1_i128;
            for order in 0..=8 {
                if order != 0 {
                    factorial *= order as i128;
                }
                let expected = FixedInterval::from_ratio(
                    &BigInt::from_i128(1),
                    &BigInt::from_i128(factorial),
                    math,
                )?;
                let actual = workspace.coefficient(state, order)?;
                assert!(actual.lower() <= expected.lower() && actual.upper() >= expected.upper());
            }
            let integral = workspace.integral(state, one, math)?;
            assert!(workspace.coefficient(integral, 9).is_err());
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn correlated_square_and_composite_derivatives_cover_singularity_neighbors() {
        let mut math = context();
        with_taylor_workspace(6, 16, &mut math, |workspace, math| {
            let x0 = FixedInterval::from_bounds(
                BigInt::from_i128(-1).mul_pow2(95),
                BigInt::from_i128(1).mul_pow2(95),
                math,
            )?;
            let x = workspace.argument(x0, FixedInterval::from_i64(1, math)?, math)?;
            let square = workspace.mul(x, x, math)?;
            assert!(workspace.coefficient(square, 0)?.lower() >= &BigInt::zero());
            let one = workspace.constant(FixedInterval::from_i64(1, math)?, math)?;
            let positive = workspace.add(one, square, math)?;
            let root = workspace.sqrt(positive, math)?;
            let logarithm = workspace.log(positive, math)?;
            assert!(workspace.coefficient(root, 0)?.lower() >= &BigInt::from_i128(1).mul_pow2(96));
            assert!(workspace.coefficient(logarithm, 0)?.lower() <= &BigInt::zero());
            assert!(workspace.coefficient(logarithm, 0)?.upper() >= &BigInt::zero());
            Ok(())
        })
        .unwrap();
    }
    #[test]
    fn complete_symmetric_panel_integrates_exact_polynomial_and_bounds_remainder() {
        let mut math = context();
        let zero = FixedInterval::from_i64(0, &mut math).unwrap();
        let half =
            FixedInterval::from_ratio(&BigInt::from_i128(1), &BigInt::from_i128(2), &mut math)
                .unwrap();
        let lower = half.neg(&mut math).unwrap();
        let panel = lower.hull(&half, &mut math).unwrap();
        let geometry = SymmetricTaylorPanel::new(&zero, &panel, &half, &mut math).unwrap();
        let (integral, remainder) =
            integrate_taylor_panel(8, 8, geometry, &mut math, |argument, workspace, math| {
                let x = workspace.argument(
                    argument.clone(),
                    FixedInterval::from_i64(1, math)?,
                    math,
                )?;
                let square = workspace.mul(x, x, math)?;
                workspace.mul(square, square, math)
            })
            .unwrap();
        let exact =
            FixedInterval::from_ratio(&BigInt::from_i128(1), &BigInt::from_i128(80), &mut math)
                .unwrap();
        assert!(integral.lower() <= exact.lower() && exact.upper() <= integral.upper());
        assert_eq!(remainder.lower(), &BigInt::zero());
        assert_eq!(remainder.upper(), &BigInt::zero());
        let (center, remainder) =
            integrate_taylor_panel(8, 8, geometry, &mut math, |argument, workspace, math| {
                let x = workspace.argument(
                    argument.clone(),
                    FixedInterval::from_i64(1, math)?,
                    math,
                )?;
                let square = workspace.mul(x, x, math)?;
                let fourth = workspace.mul(square, square, math)?;
                workspace.mul(fourth, fourth, math)
            })
            .unwrap();
        assert_eq!(center.lower(), &BigInt::zero());
        assert_eq!(center.upper(), &BigInt::zero());
        let exact =
            FixedInterval::from_ratio(&BigInt::from_i128(1), &BigInt::from_i128(2304), &mut math)
                .unwrap();
        assert!(remainder.upper() >= exact.upper());
        let asymmetric = zero.hull(&half, &mut math).unwrap();
        assert!(SymmetricTaylorPanel::new(&zero, &asymmetric, &half, &mut math).is_err());
        assert_eq!(
            math.workspace_reserved(),
            math.taylor_scratch().unwrap().retained_bytes()
        );
    }
}
