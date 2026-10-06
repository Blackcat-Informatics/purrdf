// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Checked upper bounds for the original exact integer and rational operations.
//!
//! One work unit admits a bounded binary-limb scan/product/division operation.
//! School products visit at most n² pairs. Normalized division has at most n
//! quotient digits, two trial corrections and one add-back per digit. Stein GCD
//! removes at least one bit from the sum of the odd operand lengths per iteration;
//! each iteration scans at most n limbs for comparison, subtraction and shifting.
//! The formulas include the reduction and quotient bodies, not just products.

/// Exact arithmetic body admitted before constructing its operands/results.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExactOperation {
    /// Addition, subtraction, shifts or signed comparison with one carry limb.
    Linear,
    /// One full school product.
    Multiply,
    /// One normalized quotient and remainder.
    Divide,
    /// One greatest common divisor, including all binary reductions.
    Gcd,
    /// Two rational cross products followed by exact signed comparison.
    RationalCompare,
    /// A rational reduction, including the checked-profile output GCD.
    RationalReduce,
    /// Two crosswise GCDs, four quotients, two products and the output GCD.
    RationalMultiply,
    /// Reciprocal operand normalization followed by rational multiplication.
    RationalDivide,
    /// Denominator/numerator GCDs, four quotients, three products and output GCD.
    RationalAdd,
    /// Original base-10^19 integer rendering, including quotient/group storage.
    DecimalRender,
}

/// A checked conservative bound independent of clocks, allocators and floats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExactArithmeticCost {
    /// Bounded complete binary-limb work, including canonical reduction.
    pub work_items: u64,
    /// Simultaneous result and scratch allowance, including inline storage.
    pub workspace_bytes: u64,
    /// Maximum result/intermediate magnitude width before reduction.
    pub output_bits: u64,
}

impl ExactArithmeticCost {
    /// Bound the original finite binary64-to-rational factory.
    /// The shared bit decoder supplies the normalized significand and binary
    /// exponent without allocating. Four linear allowances cover integer
    /// construction, the original shift, optional sign normalization and scans;
    /// the canonical rational reduction then uses its actual numerator and
    /// denominator widths, including the checked-profile coprimality proof.
    /// Returns `None` for a nonfinite encoding or complete-bound overflow.
    #[must_use]
    pub fn binary64_rational(bits: u64) -> Option<Self> {
        let decoded = crate::ieee::dyadic::Binary64Dyadic::decode(f64::from_bits(bits))?;
        let significand = u64::from(decoded.significand().bit_width());
        let exponent = decoded.exponent();
        let numerator = if exponent < 0 {
            significand
        } else {
            significand.checked_add(u64::from(exponent.unsigned_abs()))?
        };
        let denominator = if exponent < 0 {
            u64::from(exponent.unsigned_abs()).checked_add(1)?
        } else {
            1
        };
        Self::for_operation(ExactOperation::Linear, numerator.max(denominator), 4)?.followed_by(
            Self::for_rational_operands(
                ExactOperation::RationalReduce,
                [(numerator, denominator)],
                1,
            )?,
        )
    }

    /// Admit the original rational ordering's sign and denominator checks.
    ///
    /// Reading each sign and canonical magnitude length is constant work;
    /// structural denominator equality visits at most the larger denominator's
    /// limbs. One shared linear allowance includes those metadata reads and
    /// that complete scan. No magnitude is copied or constructed, so this
    /// phase requires no owned scratch. Only after this phase executes may the
    /// comparison choose signed numerator ordering or the original two cross
    /// products, which are admitted separately before their bodies execute.
    /// Returns `None` on complete-bound overflow.
    #[must_use]
    pub fn rational_comparison_scan(denominator_bits: u64) -> Option<Self> {
        let mut cost = Self::for_operation(ExactOperation::Linear, denominator_bits.max(1), 1)?;
        cost.workspace_bytes = 0;
        Some(cost)
    }

    /// Bound the original exact `numerator / 2^fractional_bits` factory.
    /// Trailing-zero scans and one right shift cancel binary factors; one
    /// left shift builds the remaining denominator. The checked canonical
    /// invariant scans the power-of-two denominator and tests numerator parity,
    /// without general GCD or division. Eight linear allowances enclose those
    /// scans, shifted result copies, sign/zero checks and trimming, including
    /// both live output magnitudes. Returns `None` on complete-bound overflow.
    #[must_use]
    pub fn dyadic_rational(numerator_bits: u64, fractional_bits: u32) -> Option<Self> {
        if numerator_bits == 0 {
            let mut cost = Self::for_operation(ExactOperation::Linear, 1, 1)?;
            cost.output_bits = 1;
            return Some(cost);
        }
        let bits = numerator_bits.max(u64::from(fractional_bits).checked_add(1)?);
        let mut cost = Self::for_operation(ExactOperation::Linear, bits, 8)?;
        cost.output_bits = bits;
        Some(cost)
    }

    /// Bound the original integer floor-square-root body.
    ///
    /// For a nonzero B-bit input, its initial Newton value is
    /// `2^ceil(B/2)`, at most twice the real root. Its relative error r is
    /// initially at most one; a descending Newton step has relative error
    /// `r²/(2(1+r)) <= r²/2`. Integer truncation only lowers this value.
    /// After k steps the relative error is at most `2^(1-2^k)` until the
    /// integer root is reached. Since the real root is below `2^(B/2)`,
    /// `k=ceil(log2(B+2))` makes the absolute error less than one. Three
    /// further steps cover integer-root descent and the final nondecreasing
    /// termination check. Each iteration uses
    /// one original quotient/remainder and four linear allowances for the
    /// addition, shift, comparison and retained-value scans. The initial
    /// shift and input checks fit two further linear allowances.
    ///
    /// All live magnitudes have at most B+1 bits. The original division
    /// workspace allowance also encloses the simultaneous Newton values,
    /// quotient/remainder and normalization buffers. The inline u128 body
    /// uses the same Newton equation and fits this conservative bound.
    /// Returns `None` before arithmetic if any complete bound overflows.
    #[must_use]
    pub fn integer_square_root(bits: u64) -> Option<Self> {
        let working = bits.max(1).checked_add(1)?;
        let iterations = u64::from((bits.checked_add(2)? - 1).bit_width()).checked_add(3)?;
        let mut cost = Self::for_operation(ExactOperation::Divide, working, iterations)?
            .followed_by(Self::for_operation(
                ExactOperation::Linear,
                working,
                iterations.checked_mul(4)?.checked_add(2)?,
            )?)?;
        cost.output_bits = bits.div_ceil(2);
        Some(cost)
    }

    /// Bound the original shifted rational floor-square-root equation.
    ///
    /// A positive B-bit divisor is at least `2^(B-1)`. The shifted dividend
    /// has at most A+2F bits, so its quotient has at most
    /// `max(0,A+2F-B+1)` bits. Shift/division use their full simultaneous
    /// operand width; only Newton root work uses the smaller quotient width.
    /// The original division allowance encloses the retained shifted dividend,
    /// quotient/remainder and root workspace, so composed scratch uses a maximum.
    /// The already owned destination arena is admitted separately. Invalid
    /// shifts and overflowing complete bounds return `None` before execution.
    #[must_use]
    pub fn integer_ratio_square_root(
        numerator_bits: u64,
        denominator_bits: u64,
        fractional_bits: u32,
    ) -> Option<Self> {
        let shift = fractional_bits.checked_mul(2)?;
        let shifted = numerator_bits.checked_add(u64::from(shift))?;
        let denominator = denominator_bits.max(1);
        let working = shifted.max(denominator).max(1);
        let quotient = shifted.checked_add(1)?.saturating_sub(denominator);
        let mut cost = Self::for_operation(ExactOperation::Linear, working, 3)?
            .followed_by(Self::for_operation(ExactOperation::Divide, working, 1)?)?
            .followed_by(Self::integer_square_root(quotient)?)?;
        cost.output_bits = quotient.div_ceil(2);
        Some(cost)
    }

    /// Bound the original half-open rational-difference/integer-interval test.
    ///
    /// Each operand pair supplies its actual numerator and positive denominator
    /// widths. `integer_bound_bits` bounds the magnitude of both signed i64
    /// interval endpoints. Three products form the two cross numerators and
    /// common positive denominator; two further products form the integer
    /// bounds. Eight linear allowances cover signed subtraction, operand/result
    /// scans, comparisons and trimming. All estimates compose the original
    /// multiply/linear homes; no rational reduction or GCD executes.
    ///
    /// The six integer outputs coexist. At maximum admitted output length m,
    /// their magnitudes require at most 6*(m+1)*8 bytes, including a carry limb
    /// per product. The shared 192*m+512 allowance also encloses their inline
    /// values, two one-limb bound constants and the eight-limb product scratch.
    /// Taking the maximum composed workspace therefore bounds the simultaneous
    /// set, rather than treating each retained product as sequential scratch.
    /// An already owned reusable destination arena is admitted separately.
    /// Returns `None` before arithmetic if a complete bound overflows.
    #[must_use]
    pub fn rational_difference_integer_interval(
        left: (u64, u64),
        right: (u64, u64),
        integer_bound_bits: u64,
    ) -> Option<Self> {
        let original_bits = left.0.max(left.1).max(right.0).max(right.1).max(1);
        let denominator_bits = left.1.checked_add(right.1)?;
        let difference_bits = left
            .0
            .checked_add(right.1)?
            .max(right.0.checked_add(left.1)?)
            .checked_add(1)?;
        let intermediate_bits = difference_bits
            .max(denominator_bits.checked_add(integer_bound_bits)?)
            .max(1);
        Self::for_operation(ExactOperation::Multiply, original_bits, 3)?
            .followed_by(Self::for_operation(
                ExactOperation::Multiply,
                denominator_bits.max(integer_bound_bits).max(1),
                2,
            )?)?
            .followed_by(Self::for_operation(
                ExactOperation::Linear,
                intermediate_bits,
                8,
            )?)
    }

    /// Bound the original denominator-only finite-decimal scale decision.
    ///
    /// A B-bit denominator has at most (B-1)/2 factors of five because
    /// 5^b > 2^(2b). The shared body uses 27-factor one-limb quotients, then
    /// at most 26 single-five quotients. One extra block covers the first
    /// failed attempt. Two linear-work allowances cover each small quotient,
    /// including its initialization, division, remainder and trimming; four
    /// further allowances cover valuation, shift, strip-copy and residue tests.
    /// All magnitudes shrink, so no numerator, power, product or GCD admission
    /// is included. Returns `None` before execution if a complete bound overflows.
    #[must_use]
    pub fn finite_decimal_scale(denominator_bits: u64) -> Option<Self> {
        let bits = denominator_bits.max(1);
        if bits == 1 {
            // Canonical denominators are positive: width one proves the
            // integer/zero branch before any copied destination or quotient.
            let mut cost = Self::for_operation(ExactOperation::Linear, 1, 1)?;
            cost.output_bits = 1;
            return Some(cost);
        }
        let five_limit = (bits - 1) / 2;
        let blocks = (five_limit / 27).checked_add(1)?;
        let single = five_limit.min(26);
        let passes = blocks.checked_add(single)?.checked_add(4)?.checked_mul(2)?;
        let mut cost = Self::for_operation(ExactOperation::Linear, bits, passes)?;
        cost.output_bits = bits;
        Some(cost)
    }

    /// Bound the original canonical decimal `mantissa / 10^places`.
    /// Binary valuation and shifts cancel powers of two. At most one failed
    /// 5^27 quotient follows complete chunks, followed by at most 26 quotients
    /// by five. The denominator uses limb-sized powers of five and one shift;
    /// the checked-profile coprimality proof uses one further quotient by five.
    /// Returns `None` before arithmetic if any complete bound overflows.
    #[must_use]
    pub fn decimal_rational(mantissa_bits: u64, places: u32) -> Option<Self> {
        if mantissa_bits == 0 {
            return Some(Self {
                work_items: 16,
                workspace_bytes: 512,
                output_bits: 1,
            });
        }
        let places = u64::from(places);
        // 5^27 > 2^62 and 5 > 2^2: successful quotients strictly decrease
        // magnitude width, so both scale and original width bound attempts.
        let block_divisions = (places / 27).min(mantissa_bits.div_ceil(62));
        let single_divisions = places.min(26).min(mantissa_bits.div_ceil(2));
        let divisions = block_divisions
            .checked_add(single_divisions)?
            .checked_add(1)?;
        let numerator_limbs = mantissa_bits.div_ceil(64);
        // log2(5)<3; the final binary shift contributes at most places bits.
        let power_bits = places.checked_mul(3)?.checked_add(1)?;
        let denominator_bits = power_bits.checked_add(places)?;
        let power_limbs = power_bits.div_ceil(64);
        let output_bits = mantissa_bits.max(denominator_bits);
        let output_limbs = output_bits.div_ceil(64);
        // Sixteen bounded limb operations cover each small quotient/product,
        // its initialization and trimming, and the scalar chunk multiplier.
        // Four linear passes include the original clone, valuation, right
        // shift, strip-factor clone and checked sign/prime invariants.
        let linear = numerator_limbs.checked_add(1)?.checked_mul(16)?;
        let work_items = linear
            .checked_mul(divisions.checked_add(4)?)?
            .checked_add(
                places
                    .div_ceil(27)
                    .checked_mul(power_limbs.checked_add(1)?)?
                    .checked_mul(16)?,
            )?
            .checked_add(output_limbs.checked_add(1)?.checked_mul(16)?)?;
        Some(Self {
            work_items,
            // Original, shifted and stripped numerators, the current power,
            // old/new quotient/product storage and both final values coexist.
            workspace_bytes: output_limbs.checked_mul(192)?.checked_add(512)?,
            output_bits,
        })
    }

    /// Bound exact half-even rational rounding to an integer decimal mantissa.
    /// The original power, fused product/quotient and remainder tie comparison
    /// are included; no decimal string or digit-group rendering is performed.
    /// Returns `None` before arithmetic if a complete bound overflows.
    #[must_use]
    pub fn rational_round(numerator_bits: u64, denominator_bits: u64, places: u32) -> Option<Self> {
        let power_bits = u64::from(places).checked_mul(4)?.checked_add(1)?;
        let scaled_bits = numerator_bits.max(1).checked_add(power_bits)?;
        let maximum = scaled_bits.max(denominator_bits.max(1));
        let pow10 = Self::for_operation(
            ExactOperation::Multiply,
            power_bits,
            u64::from(places).div_ceil(19).checked_add(1)?,
        )?;
        Some(Self {
            work_items: pow10
                .work_items
                .checked_add(product_rect(numerator_bits.max(1), power_bits)?)?
                .checked_add(division(maximum.div_ceil(64))?)?
                .checked_add(maximum.div_ceil(64).checked_add(1)?.checked_mul(64)?)?,
            workspace_bytes: pow10
                .workspace_bytes
                .checked_add(maximum.div_ceil(64).checked_mul(64)?)?,
            output_bits: scaled_bits,
        })
    }

    /// Bound the original exact rational-to-binary64 rounding body.
    /// The binade comparison shifts one input to the other's magnitude width.
    /// Finite normal operands after scaling have at most max(widths)+53 bits;
    /// subnormal scaling has at most denominator_width+52 bits. One extra bit
    /// admits the rounding carry. Original absolute copies, shifted operands,
    /// quotient/remainder and tie subtraction coexist with the shared division
    /// allowance, which includes normalization and its guard limb.
    /// No GCD, power-of-ten construction or decimal rendering is performed.
    #[must_use]
    pub fn rational_binary64(numerator_bits: u64, denominator_bits: u64) -> Option<Self> {
        let working = numerator_bits
            .max(denominator_bits)
            .max(1)
            .checked_add(54)?;
        let division = Self::for_operation(ExactOperation::Divide, working, 1)?;
        let scans = Self::for_operation(ExactOperation::Linear, working, 12)?;
        Some(Self {
            work_items: division.work_items.checked_add(scans.work_items)?,
            workspace_bytes: division
                .workspace_bytes
                .checked_add(scans.workspace_bytes)?,
            output_bits: working,
        })
    }

    /// Bound exact half-even rational decimal rendering at a declared scale.
    /// The original body constructs `10^places`, multiplies the numerator,
    /// divides by the denominator, rounds the remainder and renders the integer
    /// in base-10^19 groups before decimal padding. No output is constructed by
    /// this estimator. `None` refuses bound overflow before any arithmetic.
    #[must_use]
    pub fn rational_decimal(
        numerator_bits: u64,
        denominator_bits: u64,
        places: u32,
    ) -> Option<Self> {
        let rounding = Self::rational_round(numerator_bits, denominator_bits, places)?;
        let scaled_bits = rounding.output_bits;
        let maximum = scaled_bits.max(denominator_bits.max(1));
        let render = Self::for_operation(ExactOperation::DecimalRender, scaled_bits, 1)?;
        let work_items = rounding.work_items.checked_add(render.work_items)?;
        // Log10(2)<1/3 bounds integer digits. The fractional spelling may
        // require places+1 padded digits. Sign, decimal point, integer groups,
        // both original/scaled magnitudes and simultaneous output strings are
        // included before the admitted original body allocates any of them.
        let text_bytes = scaled_bits
            .div_ceil(3)
            .checked_add(u64::from(places))?
            .checked_add(8)?;
        let workspace_bytes = rounding
            .workspace_bytes
            .max(
                render
                    .workspace_bytes
                    .checked_add(maximum.div_ceil(64).checked_mul(64)?)?,
            )
            .checked_add(text_bytes.checked_mul(4)?)?;
        Some(Self {
            work_items,
            workspace_bytes,
            output_bits: scaled_bits,
        })
    }

    /// Compose sequential admitted bodies, reusing scratch while accumulating
    /// work. Callers account separately for outputs retained between bodies.
    /// Returns `None` before execution if the combined work bound overflows.
    #[must_use]
    pub fn followed_by(self, next: Self) -> Option<Self> {
        Some(Self {
            work_items: self.work_items.checked_add(next.work_items)?,
            workspace_bytes: self.workspace_bytes.max(next.workspace_bytes),
            output_bits: self.output_bits.max(next.output_bits),
        })
    }

    /// Bound `count` sequential operations with all original operands at most
    /// `operand_bits` wide. Scratch is reused; work is accumulated.
    /// Returns `None` before arithmetic if any bound overflows.
    #[must_use]
    pub fn for_operation(operation: ExactOperation, operand_bits: u64, count: u64) -> Option<Self> {
        if count == 0 {
            return Some(Self {
                work_items: 0,
                workspace_bytes: 0,
                output_bits: 0,
            });
        }
        let bits = operand_bits.max(1);
        let output_bits = match operation {
            ExactOperation::Linear => bits.checked_add(1)?,
            ExactOperation::Multiply
            | ExactOperation::RationalCompare
            | ExactOperation::RationalMultiply
            | ExactOperation::RationalDivide => bits.checked_mul(2)?,
            ExactOperation::RationalAdd => bits.checked_mul(2)?.checked_add(1)?,
            ExactOperation::Divide
            | ExactOperation::Gcd
            | ExactOperation::RationalReduce
            | ExactOperation::DecimalRender => bits,
        };
        let n = bits.div_ceil(64);
        let m = output_bits.div_ceil(64);
        let linear = n.checked_add(1)?.checked_mul(8)?;
        let product = product_rect(bits, bits)?;
        let division = division(m)?;
        let gcd = gcd_pair(output_bits, output_bits)?;
        // Canonical Rat::new also validates coprimality in the checked profile.
        let reduction = gcd.checked_mul(2)?.checked_add(division.checked_mul(2)?)?;
        // Cross cancellation and denominator-based addition each perform up to
        // two input/intermediate GCDs and four quotients before the final
        // checked-profile coprimality GCD. Bound all three at the output width.
        let rational_reduction = gcd.checked_mul(3)?.checked_add(division.checked_mul(4)?)?;
        let work = match operation {
            ExactOperation::Linear => linear,
            ExactOperation::Multiply => product,
            ExactOperation::Divide => division,
            ExactOperation::Gcd => gcd,
            ExactOperation::RationalCompare => {
                product.checked_mul(2)?.checked_add(m.checked_mul(8)?)?
            }
            ExactOperation::RationalReduce => reduction,
            ExactOperation::RationalMultiply => {
                product.checked_mul(2)?.checked_add(rational_reduction)?
            }
            ExactOperation::RationalDivide => product
                .checked_mul(2)?
                .checked_add(rational_reduction)?
                // Reciprocal clones both original integers, with optional sign
                // normalization, before entering the original multiply body.
                .checked_add(linear.checked_mul(2)?)?,
            ExactOperation::RationalAdd => product
                .checked_mul(3)?
                .checked_add(m.checked_mul(8)?)?
                .checked_add(rational_reduction)?,
            ExactOperation::DecimalRender => {
                let groups = bits.div_ceil(63);
                n.checked_mul(groups)?
                    .checked_mul(8)?
                    .checked_add(groups.checked_mul(32)?)?
                    .checked_add(n.checked_mul(8)?)?
            }
        };
        let workspace_bytes = if operation == ExactOperation::DecimalRender {
            let groups = bits.div_ceil(63);
            // Both current magnitude and quotient, u64 groups, original clone,
            // sign and at most nineteen decimal bytes per group coexist.
            n.checked_mul(24)?
                .checked_add(groups.checked_mul(27)?)?
                .checked_add(512)?
        } else {
            m.checked_mul(192)?.checked_add(512)?
        };
        Some(Self {
            work_items: work.checked_mul(count)?,
            workspace_bytes,
            output_bits,
        })
    }

    /// Bound canonical rational operands from their exact numerator and positive
    /// denominator widths. Denominator width one proves the value is an integer.
    /// Every operand used by the body, including constants, must be supplied;
    /// With exactly two operands, `count` repeats that original binary pair;
    /// division supplies dividend then divisor. Larger groups admit sequential
    /// uses of any pair under their shared numerator/denominator width bounds.
    ///
    /// Integer addition takes Rat's denominator-GCD-equals-one branch: three
    /// products, one sum, no quotients, and the checked-profile output GCD exits
    /// on denominator one. Integer multiplication's two crosswise GCDs also
    /// exit on one before any quotient. Comparison uses equal denominators and
    /// performs only signed numerator comparison. Division has a distinct
    /// descriptor because reciprocation can create a nonunit denominator.
    /// Other rational shapes bound every original crosswise/intermediate GCD
    /// from the admitted numerator and denominator widths separately. Neither
    /// output reduction nor checked-profile canonicality scans are omitted.
    #[must_use]
    pub fn for_rational_operands(
        operation: ExactOperation,
        operands: impl IntoIterator<Item = (u64, u64)>,
        count: u64,
    ) -> Option<Self> {
        let mut bits = 1_u64;
        let mut numerator_bits = 0_u64;
        let mut denominator_bits = 1_u64;
        let mut integer = true;
        let mut unit_numerators = true;
        let mut operand_count = 0_usize;
        let mut pair = [(0_u64, 1_u64); 2];
        for (numerator, denominator) in operands {
            if let Some(slot) = pair.get_mut(operand_count) {
                *slot = (numerator, denominator);
            }
            operand_count = operand_count.checked_add(1)?;
            bits = bits.max(numerator).max(denominator);
            numerator_bits = numerator_bits.max(numerator);
            denominator_bits = denominator_bits.max(denominator);
            integer &= denominator == 1;
            unit_numerators &= numerator <= 1;
        }
        let generic = Self::for_operation(operation, bits, count)?;
        if count == 0 || operand_count == 0 {
            return Some(generic);
        }
        if operand_count == 2
            && matches!(
                operation,
                ExactOperation::RationalMultiply | ExactOperation::RationalDivide
            )
        {
            let (output_bits, work) = if operation == ExactOperation::RationalDivide {
                rational_division_pair(pair[0], pair[1])?
            } else {
                rational_multiply_pair(pair[0], pair[1])?
            };
            return Some(Self {
                work_items: work.checked_mul(count)?,
                workspace_bytes: generic.workspace_bytes,
                output_bits,
            });
        }
        if !integer
            && operand_count == 2
            && matches!(
                operation,
                ExactOperation::RationalCompare | ExactOperation::RationalAdd
            )
        {
            let (output_bits, work) = if operation == ExactOperation::RationalCompare {
                rational_compare_pair(pair[0], pair[1])?
            } else {
                rational_add_pair(pair[0], pair[1])?
            };
            return Some(Self {
                work_items: work.checked_mul(count)?,
                workspace_bytes: generic.workspace_bytes,
                output_bits,
            });
        }
        if !integer {
            if !matches!(
                operation,
                ExactOperation::RationalCompare
                    | ExactOperation::RationalReduce
                    | ExactOperation::RationalMultiply
                    | ExactOperation::RationalDivide
                    | ExactOperation::RationalAdd
            ) {
                return Some(generic);
            }
            let (output_bits, work) = rational_shape(operation, numerator_bits, denominator_bits)?;
            return Some(Self {
                work_items: work.checked_mul(count)?,
                workspace_bytes: generic.workspace_bytes,
                output_bits,
            });
        }
        let n = bits.div_ceil(64);
        let linear = n.checked_add(1)?.checked_mul(8)?;
        let (output_bits, work) = match operation {
            ExactOperation::RationalCompare => (bits, linear.checked_mul(2)?),
            ExactOperation::RationalAdd => {
                let output = bits.checked_add(1)?;
                let m = output.div_ceil(64);
                // Each numerator-times-one product visits n limb pairs; the
                // denominator product is one pair. The remaining scans include
                // signed addition/subtraction, optional negation, abs, canonical
                // assertions and both constant-time gcd(...,1) exits.
                let single_product = n.checked_mul(16)?.checked_add(16)?;
                let work = single_product
                    .checked_mul(2)?
                    .checked_add(m.checked_add(1)?.checked_mul(8)?)?
                    .checked_add(n.checked_add(1)?.checked_mul(32)?)?
                    .checked_add(64)?;
                (output, work)
            }
            ExactOperation::RationalMultiply => {
                let output = bits.checked_mul(2)?;
                let product = Self::for_operation(ExactOperation::Multiply, bits, 1)?;
                // Four original limb copies, the output abs scan, denominator
                // one multiplication, and three gcd(...,1) early exits coexist.
                let work = product
                    .work_items
                    .checked_mul(2)?
                    .checked_add(linear.checked_mul(6)?)?
                    .checked_add(48)?;
                (output, work)
            }
            ExactOperation::RationalReduce | ExactOperation::RationalDivide if unit_numerators => {
                (bits, linear.checked_mul(8)?.checked_add(48)?)
            }
            ExactOperation::RationalReduce => (bits, linear.checked_mul(4)?.checked_add(32)?),
            _ => return Some(generic),
        };
        Some(Self {
            work_items: work.checked_mul(count)?,
            // Retain the former complete scratch bound. Shape refinement reduces
            // admitted CPU work and output growth, never live memory protection.
            workspace_bytes: generic.workspace_bytes,
            output_bits,
        })
    }

    /// Bound original decimal accumulation, linear chunked power-of-ten
    /// construction, the final product and canonical rational normalization.
    /// `power` is the absolute decimal scale after applying the written exponent.
    /// Returns `None` if the complete pre-construction bound overflows.
    #[must_use]
    pub fn decimal(digits: usize, power: u64) -> Option<Self> {
        let digits = u64::try_from(digits).ok()?;
        // log2(10)<4 bounds both original mantissa and exact power magnitudes.
        let bits = digits.checked_add(power)?.checked_mul(4)?.max(1);
        let n = bits.div_ceil(64);
        // The original constructor processes 19 digits per u64-sized chunk.
        let chunks = digits
            .div_ceil(super::DECIMAL_CHUNK as u64)
            .checked_add(power.div_ceil(super::DECIMAL_CHUNK as u64))?;
        let construction = chunks.checked_mul(n)?.checked_mul(8)?.checked_add(digits)?;
        let product = Self::for_operation(ExactOperation::Multiply, bits, 1)?;
        let reduction = Self::for_operation(ExactOperation::RationalReduce, bits, 1)?;
        Some(Self {
            work_items: construction
                .checked_add(product.work_items)?
                .checked_add(reduction.work_items)?,
            workspace_bytes: product.workspace_bytes.max(reduction.workspace_bytes),
            output_bits: bits,
        })
    }
}

// Shape bounds describe the same original canonical rational bodies. Distinct
// numerator/denominator widths avoid charging a tiny denominator as though it
// were an unreduced full-width numerator. Checked-profile abs/copy/sign scans
// are included in the common linear allowance.
fn rational_shape(operation: ExactOperation, n: u64, d: u64) -> Option<(u64, u64)> {
    let bits = n.max(d).max(1);
    let linear = bits.div_ceil(64).checked_add(1)?.checked_mul(8)?;
    let sum = n.checked_add(d)?;
    let result = match operation {
        ExactOperation::RationalCompare => {
            let work = product_rect(n, d)?
                .checked_mul(2)?
                .checked_add(sum.div_ceil(64).checked_add(1)?.checked_mul(8)?)?;
            (sum, work)
        }
        ExactOperation::RationalReduce => {
            let work = gcd_pair(n, d)?
                .checked_mul(2)?
                .checked_add(division(bits.div_ceil(64))?.checked_mul(2)?)?
                .checked_add(linear.checked_mul(4)?)?;
            (bits, work)
        }
        ExactOperation::RationalMultiply => {
            let output_n = n.checked_mul(2)?;
            let output_d = d.checked_mul(2)?;
            let work = gcd_pair(n, d)?
                .checked_mul(2)?
                .checked_add(division(n.div_ceil(64))?.checked_mul(2)?)?
                .checked_add(division(d.div_ceil(64))?.checked_mul(2)?)?
                .checked_add(product_rect(n, n)?)?
                .checked_add(product_rect(d, d)?)?
                .checked_add(gcd_pair(output_n, output_d)?)?
                .checked_add(linear.checked_mul(8)?)?;
            (output_n.max(output_d), work)
        }
        ExactOperation::RationalDivide => {
            // Reciprocal swaps the second operand widths. The two input GCDs
            // are numerator/numerator and denominator/denominator respectively.
            let work = gcd_pair(n, n)?
                .checked_add(gcd_pair(d, d)?)?
                .checked_add(division(n.div_ceil(64))?.checked_mul(2)?)?
                .checked_add(division(d.div_ceil(64))?.checked_mul(2)?)?
                .checked_add(product_rect(n, d)?.checked_mul(2)?)?
                .checked_add(gcd_pair(sum, sum)?)?
                .checked_add(linear.checked_mul(10)?)?;
            (sum, work)
        }
        ExactOperation::RationalAdd => {
            let output_n = sum.checked_add(1)?;
            let output_d = d.checked_mul(2)?;
            let work = gcd_pair(d, d)?
                .checked_add(division(d.div_ceil(64))?.checked_mul(3)?)?
                .checked_add(gcd_pair(output_n, d)?)?
                .checked_add(division(output_n.div_ceil(64))?)?
                .checked_add(product_rect(n, d)?.checked_mul(2)?)?
                .checked_add(product_rect(d, d)?)?
                .checked_add(gcd_pair(output_n, output_d)?)?
                .checked_add(output_n.div_ceil(64).checked_add(1)?.checked_mul(8)?)?
                .checked_add(linear.checked_mul(8)?)?;
            (output_n.max(output_d), work)
        }
        _ => return None,
    };
    Some(result)
}

fn rational_compare_pair(left: (u64, u64), right: (u64, u64)) -> Option<(u64, u64)> {
    let (nl, dl) = left;
    let (nr, dr) = right;
    let output = nl.checked_add(dr)?.max(nr.checked_add(dl)?);
    let work = product_rect(nl, dr)?
        .checked_add(product_rect(nr, dl)?)?
        .checked_add(output.div_ceil(64).checked_add(1)?.checked_mul(8)?)?;
    Some((output, work))
}

fn rational_add_pair(left: (u64, u64), right: (u64, u64)) -> Option<(u64, u64)> {
    let (nl, dl) = left;
    let (nr, dr) = right;
    let output_n = nl
        .checked_add(dr)?
        .max(nr.checked_add(dl)?)
        .checked_add(1)?;
    let output_d = dl.checked_add(dr)?;
    let linear = nl
        .max(dl)
        .max(nr)
        .max(dr)
        .max(1)
        .div_ceil(64)
        .checked_add(1)?
        .checked_mul(8)?;
    // g=gcd(dl,dr), then g2=gcd(t,g). The exact original body divides both
    // denominators by g, the sum numerator by g2, and the right denominator by
    // g2. The latter GCD's second operand is bounded by min(dl,dr), not either
    // cross-product width. Output canonicality still includes the full bound.
    let work = gcd_pair(dl, dr)?
        .checked_add(division(dl.div_ceil(64))?)?
        .checked_add(division(dr.div_ceil(64))?.checked_mul(2)?)?
        .checked_add(gcd_pair(output_n, dl.min(dr))?)?
        .checked_add(division(output_n.div_ceil(64))?)?
        .checked_add(product_rect(nl, dr)?)?
        .checked_add(product_rect(nr, dl)?)?
        .checked_add(product_rect(dl, dr)?)?
        .checked_add(gcd_pair(output_n, output_d)?)?
        .checked_add(linear.checked_mul(10)?)?;
    Some((output_n.max(output_d), work))
}

fn rational_division_pair(left: (u64, u64), right: (u64, u64)) -> Option<(u64, u64)> {
    let (output, work) = rational_multiply_pair(left, (right.1, right.0))?;
    let bits = right.0.max(right.1).max(1);
    let reciprocal = bits.div_ceil(64).checked_add(1)?.checked_mul(16)?;
    Some((output, work.checked_add(reciprocal)?))
}

fn rational_multiply_pair(left: (u64, u64), right: (u64, u64)) -> Option<(u64, u64)> {
    let (nl, dl) = left;
    let (nr, dr) = right;
    let output_n = nl.checked_add(nr)?;
    let output_d = dl.checked_add(dr)?;
    let bits = nl.max(dl).max(nr).max(dr).max(1);
    let linear = bits.div_ceil(64).checked_add(1)?.checked_mul(8)?;
    // A one-bit nonzero numerator or positive denominator is exactly one,
    // proving that cross-GCD's two quotient branches cannot execute. Zero is
    // not a one-bit magnitude: its width is zero, so it retains the full path.
    let first_quotients = if nl == 1 || dr == 1 {
        0
    } else {
        division(nl.div_ceil(64))?.checked_add(division(dr.div_ceil(64))?)?
    };
    let second_quotients = if nr == 1 || dl == 1 {
        0
    } else {
        division(nr.div_ceil(64))?.checked_add(division(dl.div_ceil(64))?)?
    };
    let work = gcd_pair(nl, dr)?
        .checked_add(gcd_pair(nr, dl)?)?
        .checked_add(first_quotients)?
        .checked_add(second_quotients)?
        .checked_add(product_rect(nl, nr)?)?
        .checked_add(product_rect(dl, dr)?)?
        .checked_add(gcd_pair(output_n, output_d)?)?
        .checked_add(linear.checked_mul(8)?)?;
    Some((output_n.max(output_d), work))
}

fn product_rect(left_bits: u64, right_bits: u64) -> Option<u64> {
    let left = left_bits.div_ceil(64);
    let right = right_bits.div_ceil(64);
    left.checked_mul(right)?
        .checked_mul(8)?
        .checked_add(left.checked_add(right)?.checked_add(1)?.checked_mul(8)?)
}

fn gcd_pair(left_bits: u64, right_bits: u64) -> Option<u64> {
    let bits = left_bits.max(right_bits).max(1);
    let limbs = bits.div_ceil(64);
    if left_bits <= 1 || right_bits <= 1 {
        // Zero clones the other magnitude; one exits before either algorithm.
        return limbs.checked_add(1)?.checked_mul(8);
    }
    if bits <= 128 {
        // After the initial larger/smaller remainder, every pair of Euclidean
        // remainders halves the preceding smaller operand. The actual u128
        // body therefore uses at most 2*min(widths)+1 fixed-width divisions.
        left_bits.min(right_bits).checked_mul(8)?.checked_add(24)
    } else {
        // Each odd Stein subtraction/shift reduces the sum of operand lengths.
        left_bits
            .checked_add(right_bits)?
            .checked_mul(limbs)?
            .checked_mul(12)?
            .checked_add(limbs.checked_mul(16)?)
    }
}

fn division(limbs: u64) -> Option<u64> {
    limbs
        .checked_mul(limbs)?
        .checked_mul(24)?
        .checked_add(limbs.checked_mul(16)?)
}
#[cfg(test)]
mod tests {
    use super::{ExactArithmeticCost, ExactOperation};

    #[test]
    fn binary64_factory_cost_bounds_extreme_original_owners() {
        for value in [
            0.0,
            -0.0,
            f64::from_bits(1),
            -f64::from_bits(1),
            f64::MIN_POSITIVE,
            1.5,
            f64::MAX,
            -f64::MAX,
        ] {
            let cost = ExactArithmeticCost::binary64_rational(value.to_bits()).unwrap();
            let exact = crate::rational::Rat::from_binary64(value).unwrap();
            assert!(cost.work_items > 0);
            assert!(cost.output_bits >= exact.numerator().bit_len());
            assert!(cost.output_bits >= exact.denominator().bit_len());
            assert!(
                cost.workspace_bytes
                    >= exact.allocated_bytes() as u64 + size_of::<crate::rational::Rat>() as u64
            );
        }
        for value in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            assert_eq!(
                ExactArithmeticCost::binary64_rational(value.to_bits()),
                None
            );
        }
    }

    #[test]
    fn ratio_root_cost_uses_exact_quotient_width_and_bounds_live_scratch() {
        use crate::integer::Int;
        for (numerator_bits, denominator_bits, fractional_bits) in [
            (1_u32, 1_u32, 0_u32),
            (2, 3, 1),
            (63, 129, 96),
            (194, 511, 256),
            (511, 194, 96),
        ] {
            let numerator = Int::one().shl(numerator_bits).sub(&Int::one());
            let denominator = Int::one().shl(denominator_bits - 1);
            let root =
                super::super::sqrt_ratio_floor(&numerator, &denominator, fractional_bits).unwrap();
            let cost = ExactArithmeticCost::integer_ratio_square_root(
                u64::from(numerator_bits),
                u64::from(denominator_bits),
                fractional_bits,
            )
            .unwrap();
            assert!(root.bit_len() <= cost.output_bits);
            let shifted = u64::from(numerator_bits) + 2 * u64::from(fractional_bits);
            let limbs = shifted.max(u64::from(denominator_bits)).div_ceil(64);
            // Shifted dividend, divisor, quotient, remainder, two Newton values,
            // successor/product scratch and eight inline control objects.
            assert!(cost.workspace_bytes >= 8 * (limbs + 1) * 8 + 8 * size_of::<Int>() as u64);
        }
        assert!(ExactArithmeticCost::integer_ratio_square_root(u64::MAX, 1, 1).is_none());
        assert!(ExactArithmeticCost::integer_ratio_square_root(1, 1, u32::MAX).is_none());
    }

    #[test]
    fn square_root_admission_encloses_original_result_width_and_live_values() {
        use crate::integer::Int;

        for bits in [0u32, 1, 2, 63, 64, 127, 128, 129, 194, 512] {
            let input = if bits == 0 {
                Int::zero()
            } else {
                Int::one().shl(bits).sub(&Int::one())
            };
            let cost = ExactArithmeticCost::integer_square_root(u64::from(bits)).unwrap();
            let root = input.sqrt_floor().unwrap();
            assert!(root.bit_len() <= cost.output_bits);
            assert!(root.mul(&root) <= input);
            let successor = root.add(&Int::one());
            assert!(successor.mul(&successor) > input);
            let limbs = (u64::from(bits) + 1).div_ceil(64);
            let live_values = 6 * (limbs + 1) * size_of::<u64>() as u64;
            let inline_values = 6 * size_of::<Int>() as u64;
            assert!(cost.workspace_bytes >= live_values + inline_values);
        }
        assert!(ExactArithmeticCost::integer_square_root(u64::MAX).is_none());
    }

    #[test]
    fn dyadic_factory_cost_encloses_both_shifted_outputs_without_gcd_work() {
        for (numerator, fractional) in [(1, 96), (449, 448), (32_768, 32_769)] {
            let cost = ExactArithmeticCost::dyadic_rational(numerator, fractional).unwrap();
            let bits = numerator.max(u64::from(fractional) + 1);
            assert_eq!(cost.output_bits, bits);
            let live = 2 * bits.div_ceil(64) * size_of::<u64>() as u64
                + 2 * size_of::<crate::integer::Int>() as u64;
            assert!(cost.workspace_bytes >= live);
            assert!(
                cost.work_items
                    < ExactArithmeticCost::for_operation(ExactOperation::RationalReduce, bits, 1)
                        .unwrap()
                        .work_items
            );
        }
        let zero = ExactArithmeticCost::dyadic_rational(0, u32::MAX).unwrap();
        assert_eq!(zero.output_bits, 1);
        assert_eq!(zero.work_items, 16);
        assert!(ExactArithmeticCost::dyadic_rational(u64::MAX, 96).is_none());
    }

    #[test]
    fn difference_interval_cost_encloses_all_live_original_products() {
        for (left, right, bounds) in [
            ((0, 1), (0, 1), 1),
            ((1, 1), (1, 1), 64),
            ((65, 64), (64, 65), 8),
            ((449, 450), (441, 448), 8),
            ((32_768, 32_769), (32_767, 32_768), 64),
        ] {
            let cost =
                ExactArithmeticCost::rational_difference_integer_interval(left, right, bounds)
                    .unwrap();
            let limbs = cost.output_bits.div_ceil(64);
            let live_magnitudes = 6 * (limbs + 1) * size_of::<u64>() as u64;
            let inline_values = 8 * size_of::<crate::integer::Int>() as u64;
            let product_scratch = 8 * size_of::<u64>() as u64;
            assert!(
                cost.workspace_bytes >= live_magnitudes + inline_values + product_scratch,
                "complete simultaneous storage must fit its original shared allowance"
            );
        }
    }

    #[test]
    fn integer_shapes_admit_only_proved_early_exit_branches() {
        for operation in [
            ExactOperation::RationalCompare,
            ExactOperation::RationalAdd,
            ExactOperation::RationalMultiply,
            ExactOperation::RationalReduce,
        ] {
            let generic = ExactArithmeticCost::for_operation(operation, 65, 1).unwrap();
            let integer =
                ExactArithmeticCost::for_rational_operands(operation, [(65, 1), (64, 1)], 1)
                    .unwrap();
            assert!(integer.work_items < generic.work_items);
            assert_eq!(integer.workspace_bytes, generic.workspace_bytes);
            let repeated =
                ExactArithmeticCost::for_rational_operands(operation, [(65, 1), (64, 1)], 3)
                    .unwrap();
            assert_eq!(repeated.work_items, integer.work_items * 3);
            assert_eq!(repeated.workspace_bytes, integer.workspace_bytes);
        }
        let add = ExactArithmeticCost::for_rational_operands(
            ExactOperation::RationalAdd,
            [(65, 1), (65, 1)],
            1,
        )
        .unwrap();
        assert_eq!(add.output_bits, 66);
        let compare = ExactArithmeticCost::for_rational_operands(
            ExactOperation::RationalCompare,
            [(65, 1), (65, 1)],
            1,
        )
        .unwrap();
        assert_eq!(compare.output_bits, 65);
    }

    #[test]
    fn division_and_nonunit_denominators_retain_complete_normalization() {
        for operation in [
            ExactOperation::RationalCompare,
            ExactOperation::RationalAdd,
            ExactOperation::RationalMultiply,
            ExactOperation::RationalDivide,
            ExactOperation::RationalReduce,
        ] {
            let generic = ExactArithmeticCost::for_operation(operation, 129, 1).unwrap();
            let nonunit =
                ExactArithmeticCost::for_rational_operands(operation, [(129, 1), (64, 2)], 1)
                    .unwrap();
            assert_eq!(nonunit.workspace_bytes, generic.workspace_bytes);
            assert!(nonunit.output_bits <= generic.output_bits);
            assert!(nonunit.work_items > 0);
        }
        let division = ExactArithmeticCost::for_rational_operands(
            ExactOperation::RationalDivide,
            [(129, 1), (128, 1)],
            1,
        )
        .unwrap();
        assert_eq!(division.output_bits, 130);
        let small_divisor = ExactArithmeticCost::for_rational_operands(
            ExactOperation::RationalDivide,
            [(129, 1), (2, 1)],
            1,
        )
        .unwrap();
        assert!(small_divisor.work_items < division.work_items);
        assert!(
            ExactArithmeticCost::for_rational_operands(
                ExactOperation::RationalMultiply,
                [(32768, 1), (32768, 1)],
                1,
            )
            .unwrap()
            .work_items
                > 262_144
        );
        assert!(
            ExactArithmeticCost::for_rational_operands(
                ExactOperation::RationalAdd,
                [(u64::MAX, 1)],
                1,
            )
            .is_none()
        );
    }

    #[test]
    fn asymmetric_rational_widths_bound_each_original_gcd() {
        let operation = ExactOperation::RationalMultiply;
        let asymmetric =
            ExactArithmeticCost::for_rational_operands(operation, [(120, 3), (120, 3)], 1).unwrap();
        let generic = ExactArithmeticCost::for_operation(operation, 120, 1).unwrap();
        assert!(asymmetric.work_items < generic.work_items);
        assert_eq!(asymmetric.output_bits, 240);
        assert_eq!(asymmetric.workspace_bytes, generic.workspace_bytes);
        let balanced =
            ExactArithmeticCost::for_rational_operands(operation, [(120, 119), (120, 119)], 1)
                .unwrap();
        assert!(balanced.work_items > asymmetric.work_items);
        let division = ExactArithmeticCost::for_rational_operands(
            ExactOperation::RationalDivide,
            [(120, 3), (120, 3)],
            1,
        )
        .unwrap();
        assert_eq!(division.output_bits, 123);
        // Numerator/numerator cancellation cannot use the tiny-denominator
        // early-exit proof admitted by multiplication.
        assert!(division.work_items > 2 * super::gcd_pair(120, 120).unwrap());
    }

    #[test]
    fn individual_pairs_do_not_promote_the_smaller_rational_to_the_other_width() {
        for operation in [ExactOperation::RationalCompare, ExactOperation::RationalAdd] {
            let pair = ExactArithmeticCost::for_rational_operands(
                operation,
                [(1075, 1095), (150, 160)],
                2,
            )
            .unwrap();
            let promoted = ExactArithmeticCost::for_rational_operands(
                operation,
                [(1075, 1095), (1075, 1095), (150, 160)],
                2,
            )
            .unwrap();
            assert!(pair.work_items < promoted.work_items);
            assert!(pair.output_bits < promoted.output_bits);
            assert_eq!(pair.workspace_bytes, promoted.workspace_bytes);
        }
    }

    #[test]
    fn complete_reduction_and_decimal_power_costs_grow_before_construction() {
        let small =
            ExactArithmeticCost::for_operation(ExactOperation::RationalReduce, 128, 1).unwrap();
        let spilled =
            ExactArithmeticCost::for_operation(ExactOperation::RationalReduce, 129, 1).unwrap();
        assert!(spilled.work_items > small.work_items);
        assert!(spilled.workspace_bytes >= small.workspace_bytes);
        let short = ExactArithmeticCost::decimal(1, 0).unwrap();
        let exponent = ExactArithmeticCost::decimal(1, 100_000).unwrap();
        assert!(exponent.work_items > 262_144);
        assert!(exponent.workspace_bytes > short.workspace_bytes);
        assert!(
            ExactArithmeticCost::for_operation(ExactOperation::RationalAdd, u64::MAX, 1).is_none()
        );
        assert!(ExactArithmeticCost::decimal(usize::MAX, u64::MAX).is_none());
    }

    #[test]
    fn scaled_integer_admission_bounds_prime_cancellation_and_checked_storage() {
        let reduction = ExactArithmeticCost::for_rational_operands(
            ExactOperation::RationalReduce,
            [(256, 73)],
            1,
        )
        .unwrap();
        let decimal = ExactArithmeticCost::decimal_rational(256, 18).unwrap();
        assert!(decimal.work_items < reduction.work_items);
        assert!(decimal.workspace_bytes >= reduction.workspace_bytes);
        assert!(decimal.output_bits >= 256);
        assert!(ExactArithmeticCost::decimal_rational(u64::MAX, 18).is_none());
        let huge = ExactArithmeticCost::decimal_rational(1, u32::MAX).unwrap();
        assert!(huge.work_items > 262_144);
        assert!(huge.workspace_bytes > 64 * 1024 * 1024);
        assert_eq!(
            ExactArithmeticCost::decimal_rational(0, u32::MAX)
                .unwrap()
                .work_items,
            16
        );
    }

    #[test]
    fn mantissa_rounding_admits_arithmetic_without_unperformed_text_rendering() {
        for (numerator, denominator, places) in [(1, 1, 6), (112, 224, 15), (512, 256, 30)] {
            let rounded =
                ExactArithmeticCost::rational_round(numerator, denominator, places).unwrap();
            let text =
                ExactArithmeticCost::rational_decimal(numerator, denominator, places).unwrap();
            assert_eq!(rounded.output_bits, text.output_bits);
            assert!(rounded.work_items < text.work_items);
            assert!(rounded.workspace_bytes < text.workspace_bytes);
        }
        assert!(ExactArithmeticCost::rational_round(u64::MAX, 1, 6).is_none());
        assert!(ExactArithmeticCost::rational_round(1, u64::MAX, 6).is_none());
    }
}
#[test]
fn rational_scan_is_linear_borrowed_and_checked_before_branch_selection() {
    let small = ExactArithmeticCost::rational_comparison_scan(1).unwrap();
    let long = ExactArithmeticCost::rational_comparison_scan(32_768).unwrap();
    assert_eq!(small.workspace_bytes, 0);
    assert_eq!(long.workspace_bytes, 0);
    assert_eq!(long.work_items, (32_768 / 64 + 1) * 8);
    assert!(long.work_items > small.work_items);
    assert!(ExactArithmeticCost::rational_comparison_scan(u64::MAX).is_none());
}
