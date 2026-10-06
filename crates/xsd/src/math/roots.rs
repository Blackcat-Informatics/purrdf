// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete two-dimensional interval root isolation, independently derived from
//! the mean value theorem and the contraction mapping theorem. For an exact,
//! nonsingular dyadic matrix C and centre c, every zero in X belongs to
//! K = c - C F(c) + (I - C DF(X))(X-c). A disjoint K excludes X. An inclusive
//! self-map together with a proved row-norm contraction proves one root, even
//! on a closed boundary. No local iterate establishes existence or exclusion.

use purrdf_lex::walk::WorkList;

use super::{CoordinateMath, FixedInterval, MathError};
use crate::BigInt;

/// A closed box, with both coordinates on the context's exact dyadic grid.
pub type RootBox2 = [FixedInterval; 2];
/// Image-coordinate rows and source-coordinate columns of a whole-box derivative.
pub type RootJacobian2 = [[FixedInterval; 2]; 2];

/// Caller declarations for deterministic root subdivision and contraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootIsolationLimits {
    /// Maximum geometrical bisections along a traversal path.
    pub max_depth: u32,
    /// Maximum certified contractions of one root/candidate enclosure.
    pub max_refinements: u32,
}

/// A continuous differentiable system over the complete admitted box.
/// Values enclose F(X), not a sample; derivatives enclose DF(X). The callback
/// may refuse singular boundaries, which forces subdivision or a typed refusal.
pub trait RootSystem2 {
    /// Enclose the residual image of the complete box.
    ///
    /// # Errors
    /// Propagates numerical admission and unresolved image decisions.
    fn image(
        &mut self,
        domain: &RootBox2,
        math: &mut CoordinateMath,
    ) -> Result<RootBox2, MathError>;
    /// Enclose every derivative on the complete box after image exclusion.
    ///
    /// # Errors
    /// Propagates singularities and numerical admission refusals.
    fn jacobian(
        &mut self,
        domain: &RootBox2,
        math: &mut CoordinateMath,
    ) -> Result<RootJacobian2, MathError>;
    /// Supply an independently proved exact dyadic zero in the complete box.
    /// Each returned interval must be a point, and its exact residual must be
    /// zero by the caller's original source law. An interval residual merely
    /// containing zero is not a witness. The solver separately proves strict
    /// contraction on the complete box before using this existence evidence.
    ///
    /// # Errors
    /// Propagates proof admission and cancellation refusals.
    fn exact_root(
        &mut self,
        _domain: &RootBox2,
        _math: &mut CoordinateMath,
    ) -> Result<Option<RootBox2>, MathError> {
        Ok(None)
    }
    /// Whether this proved root enclosure meets the caller's frozen output law.
    ///
    /// # Errors
    /// Refuses unresolved rounding or insufficient precision.
    fn complete(
        &mut self,
        enclosure: &RootBox2,
        math: &mut CoordinateMath,
    ) -> Result<bool, MathError>;
    /// Bounded cancellation/work poll between numerical chunks.
    ///
    /// # Errors
    /// A refusal stops before the next complete-box evaluation.
    fn poll(&mut self, math: &CoordinateMath) -> Result<(), MathError>;
}

/// One proved existing root, with a separate domain proving its uniqueness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootIsolation2 {
    enclosure: RootBox2,
    uniqueness_box: RootBox2,
}

impl RootIsolation2 {
    /// Inclusive bounds containing the root.
    #[must_use]
    pub const fn enclosure(&self) -> &RootBox2 {
        &self.enclosure
    }
    /// A complete box containing exactly this root.
    #[must_use]
    pub const fn uniqueness_box(&self) -> &RootBox2 {
        &self.uniqueness_box
    }
    /// Consume the proof record without copying either endpoint allocation.
    #[must_use]
    pub fn into_enclosure(self) -> RootBox2 {
        self.enclosure
    }
}

/// Caller-owned root records; storage changes preserve every complete proof
/// record in insertion order. Arithmetic and canonical root ordering remain in
/// the shared solver. A push is admitted before its allocation is requested.
pub trait RootStorage2 {
    /// Complete currently held root records.
    fn roots(&self) -> &[RootIsolation2];
    /// Complete mutable root records for common uniqueness and canonical sorting.
    fn roots_mut(&mut self) -> &mut [RootIsolation2];
    /// Drop all records while retaining reusable container capacity.
    fn clear(&mut self);
    /// Add one already admitted record without changing earlier records.
    /// # Errors
    /// Refuses allocator/capacity failure without publishing a partial result.
    fn try_push(&mut self, root: RootIsolation2) -> Result<(), MathError>;
}
impl RootStorage2 for Vec<RootIsolation2> {
    fn roots(&self) -> &[RootIsolation2] {
        self
    }
    fn roots_mut(&mut self) -> &mut [RootIsolation2] {
        self
    }
    fn clear(&mut self) {
        self.clear();
    }
    fn try_push(&mut self, root: RootIsolation2) -> Result<(), MathError> {
        self.try_reserve_exact(1)
            .map_err(|_| MathError::WorkspaceExhausted)?;
        self.push(root);
        Ok(())
    }
}

#[derive(Debug)]
struct Candidate {
    domain: RootBox2,
    depth: u32,
    refinements: u32,
}

/// Isolate every root in the original closed box, or return a refusal.
/// A successful empty vector proves there is no root. Roots sharing subdivision
/// boundaries coalesce only after a common uniqueness proof. Neither precision
/// nor resource limits permit a successful partial list.
///
/// # Errors
/// Refuses exhausted depth, convergence, work, workspace, or an unresolved
/// multiplicity/alias decision. The callback must supply valid whole-box bounds.
pub fn isolate_roots2(
    domain: RootBox2,
    limits: RootIsolationLimits,
    system: &mut impl RootSystem2,
    math: &mut CoordinateMath,
) -> Result<Vec<RootIsolation2>, MathError> {
    let mut roots = Vec::new();
    isolate_roots2_into(domain, limits, system, math, &mut roots)?;
    Ok(roots)
}

/// Isolate every root into admitted caller storage using the same original
/// traversal, uniqueness, refinement and canonicalization body.
/// # Errors
/// Carries complete isolation refusals. Every refusal clears all output records.
pub fn isolate_roots2_into(
    domain: RootBox2,
    limits: RootIsolationLimits,
    system: &mut impl RootSystem2,
    math: &mut CoordinateMath,
    roots: &mut impl RootStorage2,
) -> Result<(), MathError> {
    roots.clear();
    if limits.max_depth == 0 || limits.max_refinements == 0 {
        return Err(MathError::PrecisionExhausted);
    }
    let mut reserved = 0;
    let result = isolate_inner(domain, limits, system, math, &mut reserved, roots);
    let result = math.release_workspace(reserved).and(result);
    if result.is_err() {
        roots.clear();
    }
    result
}

fn isolate_inner(
    domain: RootBox2,
    limits: RootIsolationLimits,
    system: &mut impl RootSystem2,
    math: &mut CoordinateMath,
    reserved: &mut usize,
    roots: &mut impl RootStorage2,
) -> Result<(), MathError> {
    // A binary depth-first traversal holds at most depth+1 pending boxes.
    // Source magnitudes can be much larger than the fractional precision, so
    // admission includes the actual endpoint storage, not only precision bits.
    let interval_bytes = domain
        .iter()
        .map(FixedInterval::workspace_bytes)
        .max()
        .ok_or(MathError::WorkspaceExhausted)?
        .checked_mul(4)
        .and_then(|bytes| bytes.checked_add(64))
        .ok_or(MathError::WorkspaceExhausted)?;
    reserve_records(
        usize::try_from(limits.max_depth)
            .map_err(|_| MathError::WorkspaceExhausted)?
            .checked_add(2)
            .ok_or(MathError::WorkspaceExhausted)?,
        interval_bytes,
        math,
        reserved,
    )?;
    let original = domain.clone();
    let mut pending = WorkList::<Candidate, 8>::with(Candidate {
        domain,
        depth: 0,
        refinements: 0,
    });
    while let Some(mut candidate) = pending.pop() {
        system.poll(math)?;
        math.charge(1, math.limits.precision_bits as usize)?;
        if roots
            .roots()
            .iter()
            .any(|root| subset(&candidate.domain, &root.uniqueness_box))
        {
            continue;
        }
        let image = match system.image(&candidate.domain, math) {
            Ok(image) => image,
            Err(MathError::PrecisionExhausted) => {
                subdivide(candidate, limits, &mut pending, math)?;
                continue;
            }
            Err(error) => return Err(error),
        };
        if image.iter().any(|value| !contains_zero(value)) {
            continue;
        }
        let attempt = krawczyk(&candidate.domain, system, math);
        let Some((proposal, contraction)) = (match attempt {
            Ok(attempt) => attempt,
            Err(MathError::PrecisionExhausted) => None,
            Err(error) => return Err(error),
        }) else {
            subdivide(candidate, limits, &mut pending, math)?;
            continue;
        };
        let witness = if contraction {
            system.exact_root(&candidate.domain, math)?
        } else {
            None
        };
        if let Some(witness) = witness {
            if witness.iter().any(|value| value.lower() != value.upper())
                || !subset(&witness, &candidate.domain)
            {
                return Err(MathError::Domain(
                    "exact root witness must be a point inside its original box",
                ));
            }
            if !subset(&witness, &proposal) {
                return Err(MathError::PrecisionExhausted);
            }
            let mut root = RootIsolation2 {
                enclosure: witness,
                uniqueness_box: candidate.domain,
            };
            finish_root(&mut root, limits.max_refinements, system, math)?;
            insert_root(root, roots, limits, system, math, reserved, interval_bytes)?;
            continue;
        }
        let Some(intersection) = intersect(&candidate.domain, &proposal, math)? else {
            continue;
        };
        let proof_box = if contraction && subset(&proposal, &candidate.domain) {
            Some(candidate.domain.clone())
        } else if contraction {
            // A root on an internal subdivision face can defeat a centred
            // self-map despite a valid contraction. Prove it on an enlarged
            // box still wholly inside the original admitted domain. Every root
            // of the candidate is already in the intersection and hence in
            // this proof box; no uncovered candidate is discarded.
            let expanded = enlarge(&intersection, &original, math)?;
            match krawczyk(&expanded, system, math) {
                Ok(Some((proposal, true))) if subset(&proposal, &expanded) => Some(expanded),
                Ok(_) | Err(MathError::PrecisionExhausted) => None,
                Err(error) => return Err(error),
            }
        } else {
            None
        };
        if let Some(proof_box) = proof_box {
            let Some((proposal, _)) = krawczyk(&proof_box, system, math)? else {
                return Err(MathError::PrecisionExhausted);
            };
            let mut root = RootIsolation2 {
                enclosure: intersect(&proof_box, &proposal, math)?
                    .ok_or(MathError::PrecisionExhausted)?,
                uniqueness_box: proof_box,
            };
            finish_root(&mut root, limits.max_refinements, system, math)?;
            insert_root(root, roots, limits, system, math, reserved, interval_bytes)?;
        } else if contracted(&candidate.domain, &intersection) {
            candidate.refinements =
                candidate
                    .refinements
                    .checked_add(1)
                    .ok_or(MathError::ConvergenceExhausted {
                        iterations: limits.max_refinements,
                    })?;
            if candidate.refinements > limits.max_refinements {
                return Err(MathError::ConvergenceExhausted {
                    iterations: limits.max_refinements,
                });
            }
            candidate.domain = intersection;
            pending.push(candidate);
        } else {
            subdivide(candidate, limits, &mut pending, math)?;
        }
    }
    roots.roots_mut().sort_by(|left, right| {
        left.enclosure[0]
            .lower()
            .cmp(right.enclosure[0].lower())
            .then_with(|| left.enclosure[1].lower().cmp(right.enclosure[1].lower()))
    });
    Ok(())
}

fn reserve_records(
    count: usize,
    interval_bytes: usize,
    math: &mut CoordinateMath,
    reserved: &mut usize,
) -> Result<(), MathError> {
    let bytes = count
        .checked_mul(interval_bytes)
        .and_then(|bytes| bytes.checked_mul(12))
        .ok_or(MathError::WorkspaceExhausted)?;
    math.reserve_workspace(bytes)?;
    *reserved = reserved
        .checked_add(bytes)
        .ok_or(MathError::WorkspaceExhausted)?;
    Ok(())
}

fn insert_root(
    mut root: RootIsolation2,
    roots: &mut impl RootStorage2,
    limits: RootIsolationLimits,
    system: &mut impl RootSystem2,
    math: &mut CoordinateMath,
    reserved: &mut usize,
    interval_bytes: usize,
) -> Result<(), MathError> {
    for old in roots.roots_mut().iter_mut() {
        for iteration in 0..=limits.max_refinements {
            if disjoint(&root.enclosure, &old.enclosure) {
                break;
            }
            if subset(&root.enclosure, &old.uniqueness_box)
                || subset(&old.enclosure, &root.uniqueness_box)
                || common_uniqueness(old, &root, system, math)?
            {
                old.enclosure = intersect(&old.enclosure, &root.enclosure, math)?
                    .ok_or(MathError::PrecisionExhausted)?;
                return Ok(());
            }
            if iteration == limits.max_refinements {
                return Err(MathError::PrecisionExhausted);
            }
            refine_root(old, system, math)?;
            refine_root(&mut root, system, math)?;
        }
    }
    // reserve_exact avoids geometric capacity growth escaping the admission.
    reserve_records(1, interval_bytes, math, reserved)?;
    roots.try_push(root)
}

fn common_uniqueness(
    a: &RootIsolation2,
    b: &RootIsolation2,
    system: &mut impl RootSystem2,
    math: &mut CoordinateMath,
) -> Result<bool, MathError> {
    let union = hull(&a.enclosure, &b.enclosure, math)?;
    match krawczyk(&union, system, math) {
        Ok(Some((_, contraction))) => Ok(contraction),
        Ok(None) | Err(MathError::PrecisionExhausted) => Ok(false),
        Err(error) => Err(error),
    }
}

fn finish_root(
    root: &mut RootIsolation2,
    iterations: u32,
    system: &mut impl RootSystem2,
    math: &mut CoordinateMath,
) -> Result<(), MathError> {
    for _ in 0..iterations {
        system.poll(math)?;
        if system.complete(&root.enclosure, math)? {
            return Ok(());
        }
        refine_root(root, system, math)?;
    }
    Err(MathError::ConvergenceExhausted { iterations })
}

fn refine_root(
    root: &mut RootIsolation2,
    system: &mut impl RootSystem2,
    math: &mut CoordinateMath,
) -> Result<(), MathError> {
    system.poll(math)?;
    let Some((proposal, _)) = krawczyk(&root.enclosure, system, math)? else {
        return Err(MathError::PrecisionExhausted);
    };
    let tighter =
        intersect(&root.enclosure, &proposal, math)?.ok_or(MathError::PrecisionExhausted)?;
    if tighter == root.enclosure {
        return Err(MathError::PrecisionExhausted);
    }
    root.enclosure = tighter;
    Ok(())
}

/// Test completed inverse enclosures against a fixed carrier grid or the
/// conditioned width of an unrounded input family. This never proves existence;
/// the complete root solver must already have established that separately.
///
/// # Errors
/// Refuses singular whole-box derivatives and numerical admission.
pub fn root_enclosure_complete(
    enclosure: &RootBox2,
    target: &RootBox2,
    jacobian: &RootJacobian2,
    decimal_places: u32,
    quantize: bool,
    math: &mut CoordinateMath,
) -> Result<bool, MathError> {
    let zero = FixedInterval::from_i64(0, math)?;
    if quantize {
        for value in enclosure {
            if !inverse_coordinate_complete(value, &zero, decimal_places, true, math)? {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    let inverse = invert_jacobian2(jacobian, math)?;
    let input = [target[0].width(math)?, target[1].width(math)?];
    for axis in 0..2 {
        let conditioned = inverse[axis][0]
            .abs(math)?
            .mul(&input[0], math)?
            .add(&inverse[axis][1].abs(math)?.mul(&input[1], math)?, math)?;
        if !inverse_coordinate_complete(
            &enclosure[axis],
            &conditioned,
            decimal_places,
            false,
            math,
        )? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Complete one inverse ordinate under a fixed output grid or an explicitly
/// proved conditioning bound for the whole unrounded input family.
///
/// # Errors
/// Propagates numerical admission and refuses a negative supplied width bound.
pub fn inverse_coordinate_complete(
    enclosure: &FixedInterval,
    conditioned_input_width: &FixedInterval,
    decimal_places: u32,
    quantize: bool,
    math: &mut CoordinateMath,
) -> Result<bool, MathError> {
    inverse_coordinate_complete_with_error(
        enclosure,
        conditioned_input_width,
        &FixedInterval::from_i64(0, math)?,
        decimal_places,
        quantize,
        math,
    )
}

/// Complete an inverse family including its separately proved arithmetic floor.
/// `conditioned_evaluation_width` bounds the width introduced by evaluating the
/// forward map at a single exact dyadic point, transported through a proved
/// inverse conditioning bound. It is not a difference between approximations.
///
/// # Errors
/// Propagates admission and refuses negative input or evaluation width bounds.
pub fn inverse_coordinate_complete_with_error(
    enclosure: &FixedInterval,
    conditioned_input_width: &FixedInterval,
    conditioned_evaluation_width: &FixedInterval,
    decimal_places: u32,
    quantize: bool,
    math: &mut CoordinateMath,
) -> Result<bool, MathError> {
    if conditioned_input_width.lower() < &BigInt::zero()
        || conditioned_evaluation_width.lower() < &BigInt::zero()
    {
        return Err(MathError::Domain("negative inverse conditioning width"));
    }
    if quantize {
        let (lower, upper) = enclosure.round_decimal(decimal_places, math)?;
        return Ok(lower == upper);
    }
    let guard = FixedInterval::from_bounds(BigInt::from_i128(128), BigInt::from_i128(128), math)?;
    let admitted = conditioned_input_width
        .add(conditioned_evaluation_width, math)?
        .mul(&FixedInterval::from_i64(2, math)?, math)?
        .add(&guard, math)?;
    Ok(enclosure.width(math)?.upper() <= admitted.upper())
}

/// Compose whole-box two-dimensional derivatives in declared operation order.
///
/// # Errors
/// Propagates controlled arithmetic and numerical admission refusals.
pub fn compose_jacobians2(
    left: &RootJacobian2,
    right: &RootJacobian2,
    math: &mut CoordinateMath,
) -> Result<RootJacobian2, MathError> {
    let entry = |row: usize, column: usize, math: &mut CoordinateMath| {
        left[row][0]
            .mul(&right[0][column], math)?
            .add(&left[row][1].mul(&right[1][column], math)?, math)
    };
    Ok([
        [entry(0, 0, math)?, entry(0, 1, math)?],
        [entry(1, 0, math)?, entry(1, 1, math)?],
    ])
}

/// Enclose the actual inverse of every nonsingular matrix in this derivative.
///
/// # Errors
/// Refuses a determinant enclosure containing zero and arithmetic admission.
pub fn invert_jacobian2(
    matrix: &RootJacobian2,
    math: &mut CoordinateMath,
) -> Result<RootJacobian2, MathError> {
    let determinant = matrix[0][0]
        .mul(&matrix[1][1], math)?
        .sub(&matrix[0][1].mul(&matrix[1][0], math)?, math)?;
    if contains_zero(&determinant) {
        return Err(MathError::PrecisionExhausted);
    }
    Ok([
        [
            matrix[1][1].div(&determinant, math)?,
            matrix[0][1].neg(math)?.div(&determinant, math)?,
        ],
        [
            matrix[1][0].neg(math)?.div(&determinant, math)?,
            matrix[0][0].div(&determinant, math)?,
        ],
    ])
}

fn krawczyk(
    domain: &RootBox2,
    system: &mut impl RootSystem2,
    math: &mut CoordinateMath,
) -> Result<Option<(RootBox2, bool)>, MathError> {
    let centre = [domain[0].midpoint(math)?, domain[1].midpoint(math)?];
    let jacobian = system.jacobian(domain, math)?;
    let Some(c) = preconditioner(&jacobian, math)? else {
        return Ok(None);
    };
    let value = system.image(&centre, math)?;
    let zero = FixedInterval::from_i64(0, math)?;
    let one = FixedInterval::from_i64(1, math)?;
    let product = compose_jacobians2(&c, &jacobian, math)?;
    let mut error = core::array::from_fn::<_, 2, _>(|_| [zero.clone(), zero.clone()]);
    for row in 0..2 {
        for column in 0..2 {
            error[row][column] = if row == column {
                one.clone()
            } else {
                zero.clone()
            }
            .sub(&product[row][column], math)?;
        }
    }
    let mut contraction = true;
    for row in &error {
        let bound = row[0].abs(math)?.add(&row[1].abs(math)?, math)?;
        contraction &= bound.upper() < one.lower();
    }
    let delta = [
        domain[0].sub(&centre[0], math)?,
        domain[1].sub(&centre[1], math)?,
    ];
    let mut proposal = centre.clone();
    for row in 0..2 {
        proposal[row] = centre[row]
            .sub(
                &c[row][0]
                    .mul(&value[0], math)?
                    .add(&c[row][1].mul(&value[1], math)?, math)?,
                math,
            )?
            .add(
                &error[row][0]
                    .mul(&delta[0], math)?
                    .add(&error[row][1].mul(&delta[1], math)?, math)?,
                math,
            )?;
    }
    Ok(Some((proposal, contraction)))
}

fn preconditioner(
    jacobian: &RootJacobian2,
    math: &mut CoordinateMath,
) -> Result<Option<RootJacobian2>, MathError> {
    let m = [
        [
            jacobian[0][0].midpoint(math)?,
            jacobian[0][1].midpoint(math)?,
        ],
        [
            jacobian[1][0].midpoint(math)?,
            jacobian[1][1].midpoint(math)?,
        ],
    ];
    let inverse = match invert_jacobian2(&m, math) {
        Ok(inverse) => inverse,
        Err(MathError::PrecisionExhausted) => return Ok(None),
        Err(error) => return Err(error),
    };
    // A chosen dyadic point, rather than an interval matrix, is the actual C.
    let c = [
        [inverse[0][0].midpoint(math)?, inverse[0][1].midpoint(math)?],
        [inverse[1][0].midpoint(math)?, inverse[1][1].midpoint(math)?],
    ];
    let determinant = c[0][0]
        .mul(&c[1][1], math)?
        .sub(&c[0][1].mul(&c[1][0], math)?, math)?;
    Ok((!contains_zero(&determinant)).then_some(c))
}

fn subdivide(
    candidate: Candidate,
    limits: RootIsolationLimits,
    pending: &mut WorkList<Candidate, 8>,
    math: &mut CoordinateMath,
) -> Result<(), MathError> {
    if candidate.depth >= limits.max_depth {
        return Err(MathError::PrecisionExhausted);
    }
    let axis = usize::from(width(&candidate.domain[1]) > width(&candidate.domain[0]));
    let middle = candidate.domain[axis].midpoint(math)?;
    if middle.lower() <= candidate.domain[axis].lower()
        || middle.upper() >= candidate.domain[axis].upper()
    {
        return Err(MathError::PrecisionExhausted);
    }
    let mut low = candidate.domain.clone();
    let mut high = candidate.domain;
    low[axis] =
        FixedInterval::from_bounds(low[axis].lower().clone(), middle.upper().clone(), math)?;
    high[axis] =
        FixedInterval::from_bounds(middle.lower().clone(), high[axis].upper().clone(), math)?;
    pending.push(Candidate {
        domain: high,
        depth: candidate.depth + 1,
        refinements: 0,
    });
    pending.push(Candidate {
        domain: low,
        depth: candidate.depth + 1,
        refinements: 0,
    });
    Ok(())
}

fn contains_zero(value: &FixedInterval) -> bool {
    value.lower() <= &BigInt::zero() && value.upper() >= &BigInt::zero()
}
fn width(value: &FixedInterval) -> BigInt {
    value.upper().sub(value.lower())
}
fn subset(a: &RootBox2, b: &RootBox2) -> bool {
    (0..2).all(|axis| a[axis].lower() >= b[axis].lower() && a[axis].upper() <= b[axis].upper())
}
fn disjoint(a: &RootBox2, b: &RootBox2) -> bool {
    (0..2).any(|axis| a[axis].upper() < b[axis].lower() || b[axis].upper() < a[axis].lower())
}
fn contracted(a: &RootBox2, b: &RootBox2) -> bool {
    width(&b[0]).add(&width(&b[1])).mul_small(4) < width(&a[0]).add(&width(&a[1])).mul_small(3)
}

fn intersect(
    a: &RootBox2,
    b: &RootBox2,
    math: &mut CoordinateMath,
) -> Result<Option<RootBox2>, MathError> {
    let Some(first) = a[0].intersection(&b[0], math)? else {
        return Ok(None);
    };
    let Some(second) = a[1].intersection(&b[1], math)? else {
        return Ok(None);
    };
    Ok(Some([first, second]))
}
fn hull(a: &RootBox2, b: &RootBox2, math: &mut CoordinateMath) -> Result<RootBox2, MathError> {
    Ok([a[0].hull(&b[0], math)?, a[1].hull(&b[1], math)?])
}

fn enlarge(
    domain: &RootBox2,
    original: &RootBox2,
    math: &mut CoordinateMath,
) -> Result<RootBox2, MathError> {
    let mut expanded = domain.clone();
    for axis in 0..2 {
        let margin = width(&domain[axis]).max(BigInt::from_i128(2));
        expanded[axis] = FixedInterval::from_bounds(
            domain[axis]
                .lower()
                .sub(&margin)
                .max(original[axis].lower().clone()),
            domain[axis]
                .upper()
                .add(&margin)
                .min(original[axis].upper().clone()),
            math,
        )?;
    }
    Ok(expanded)
}

#[cfg(test)]
mod tests;
