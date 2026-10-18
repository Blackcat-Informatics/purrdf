// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact radix-2^16 convolution over 2^64 - 2^32 + 1.

use super::{Allocate, LimbScratchError, Mag, mag_trim};

pub(crate) const THRESHOLD: u64 = 32_768;
const MODULUS: u64 = 0xffff_ffff_0000_0001;
const FOLD: u64 = 0xffff_ffff;
const GENERATOR: u64 = 7;
const MAX_LENGTH: u64 = 1 << 32;

// For any prime q dividing MODULUS, this equality forces the order of 7
// modulo q to contain 2^32. Thus q >= 2^32+1 > sqrt(MODULUS): the modulus
// is prime, and roots derived from 7 have the required power-of-two orders.
const _: () = assert!(power(GENERATOR, (MODULUS - 1) / 2) == MODULUS - 1);

/// The same checked balanced dispatch shape serves arithmetic and admission.
/// Asymmetry or a length beyond the field's roots retains recursive products.
pub(crate) const fn selected_length(a: u64, b: u64) -> Option<u64> {
    if a < THRESHOLD || b < THRESHOLD {
        return None;
    }
    let (long, short) = if a > b { (a, b) } else { (b, a) };
    if short <= long.div_ceil(3).saturating_mul(2) {
        return None;
    }
    let Some(words) = a.checked_add(b) else {
        return None;
    };
    let Some(digits) = words.checked_mul(4) else {
        return None;
    };
    let Some(length) = digits.checked_next_power_of_two() else {
        return None;
    };
    if length > MAX_LENGTH {
        None
    } else {
        Some(length)
    }
}

/// Add with a sum below 2p. Carry folding cannot overflow again because
/// 2p < 2^65 - 2^33 + 2; subtracting p once makes the result canonical.
const fn add(a: u64, b: u64) -> u64 {
    let (sum, carry) = a.overflowing_add(b);
    let sum = if carry { sum + FOLD } else { sum };
    if sum >= MODULUS { sum - MODULUS } else { sum }
}

const fn subtract(a: u64, b: u64) -> u64 {
    let (difference, borrow) = a.overflowing_sub(b);
    if borrow {
        difference - FOLD
    } else {
        difference
    }
}

/// Since 2^64 = 2^32-1 mod p, the high product word contributes
/// low32(high)*(2^32-1)-high32(high). Both folded additions stay below 2p.
const fn multiply(a: u64, b: u64) -> u64 {
    let product = a as u128 * b as u128;
    let high = (product >> 64) as u64;
    let (low, borrow) = (product as u64).overflowing_sub(high >> 32);
    let low = if borrow { low - FOLD } else { low };
    add(low, (high & FOLD) * FOLD)
}

const fn power(mut value: u64, mut exponent: u64) -> u64 {
    let mut result = 1;
    while exponent != 0 {
        if exponent & 1 != 0 {
            result = multiply(result, value);
        }
        exponent >>= 1;
        if exponent != 0 {
            value = multiply(value, value);
        }
    }
    result
}

/// One iterative butterfly body for both directions, separately emitted for
/// the architecture manifest. No transform-sized stack or twiddle table exists.
#[inline(never)]
fn transform(words: &mut [u64], inverse: bool) {
    let length = words.len();
    debug_assert!(length.is_power_of_two());
    let mut reversed = 0;
    for index in 1..length {
        let mut bit = length / 2;
        while reversed & bit != 0 {
            reversed ^= bit;
            bit /= 2;
        }
        reversed ^= bit;
        if index < reversed {
            words.swap(index, reversed);
        }
    }
    let generator = if inverse {
        power(GENERATOR, MODULUS - 2)
    } else {
        GENERATOR
    };
    for stage in 1..=length.trailing_zeros() {
        let width = 1_usize << stage;
        let root = power(generator, (MODULUS - 1) / width as u64);
        for block in words.chunks_exact_mut(width) {
            let (left, right) = block.split_at_mut(width / 2);
            let mut factor = 1;
            for (even, odd) in left.iter_mut().zip(right) {
                let value = multiply(*odd, factor);
                *odd = subtract(*even, value);
                *even = add(*even, value);
                factor = multiply(factor, root);
            }
        }
    }
    if inverse {
        let reciprocal = power(length as u64, MODULUS - 2);
        for word in words {
            *word = multiply(*word, reciprocal);
        }
    }
}

/// Two admitted arrays, then one array beside the minimal ordinary result.
/// Each coefficient is < min(4a,4b)*2^32 <= 2^63, so its field residue is
/// the actual integer coefficient. Carry reconstruction therefore stays exact.
pub(super) fn product(
    a: &[u64],
    b: &[u64],
    length: usize,
    storage: &impl Allocate,
) -> Result<Mag, LimbScratchError> {
    let result_length = a
        .len()
        .checked_add(b.len())
        .ok_or(LimbScratchError::SizeOverflow)?;
    debug_assert!(length.is_power_of_two() && length / 4 >= result_length);
    let mut left = storage.destination(length, length)?;
    let mut right = storage.destination(length, length)?;
    for (input, output) in [(a, &mut left), (b, &mut right)] {
        for (&word, digits) in input.iter().zip(output.as_chunks_mut::<4>().0) {
            for (part, digit) in digits.iter_mut().enumerate() {
                *digit = (word >> (part * 16)) & 0xffff;
            }
        }
        transform(output, false);
    }
    for (slot, &value) in left.iter_mut().zip(right.iter()) {
        *slot = multiply(*slot, value);
    }
    drop(right);
    transform(&mut left, true);
    let mut result = storage.destination(result_length, result_length)?;
    let mut carry = 0_u64;
    for (word, digits) in result.iter_mut().zip(left.as_chunks::<4>().0) {
        for (part, &coefficient) in digits.iter().enumerate() {
            let value = coefficient
                .checked_add(carry)
                .ok_or(LimbScratchError::SizeOverflow)?;
            *word |= (value & 0xffff) << (part * 16);
            carry = value >> 16;
        }
    }
    if carry != 0 {
        return Err(LimbScratchError::SizeOverflow);
    }
    mag_trim(&mut result);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bigint::{BigInt, Unbounded};

    #[test]
    fn field_folding_and_roots_match_integer_remainders() {
        let mut state = 0x004e_5454_4649_454c;
        let edges = [
            0,
            1,
            2,
            FOLD - 1,
            FOLD,
            FOLD + 1,
            MODULUS / 2,
            MODULUS - 2,
            MODULUS - 1,
        ];
        for a in edges
            .into_iter()
            .chain((0..4096).map(|_| purrdf_hash::mix::splitmix64_next(&mut state) % MODULUS))
        {
            for b in edges {
                assert_eq!(
                    multiply(a, b),
                    ((u128::from(a) * u128::from(b)) % u128::from(MODULUS)) as u64
                );
                assert_eq!(
                    add(a, b),
                    ((u128::from(a) + u128::from(b)) % u128::from(MODULUS)) as u64
                );
                assert_eq!(
                    subtract(a, b),
                    ((u128::from(a) + u128::from(MODULUS) - u128::from(b)) % u128::from(MODULUS))
                        as u64
                );
            }
        }
        for bits in 1..=32 {
            let length = 1_u64 << bits;
            let root = power(GENERATOR, (MODULUS - 1) / length);
            assert_eq!(power(root, length), 1);
            assert_eq!(power(root, length / 2), MODULUS - 1);
        }
    }

    #[test]
    fn convolution_matches_schoolbook_with_full_carries_and_zero_padding() {
        // (B²-1)² = B⁴-2B²+1, B=2^64: a frozen full-carry golden.
        assert_eq!(
            &*product(&[u64::MAX; 2], &[u64::MAX; 2], 16, &Unbounded).unwrap(),
            &[1, 0, u64::MAX - 1, u64::MAX],
        );
        let mut state = 0x004e_5454_5052_4f44;
        for (la, lb) in [
            (1_usize, 1),
            (2, 3),
            (31, 32),
            (32, 33),
            (127, 129),
            (1024, 1023),
        ] {
            for dense in [false, true] {
                let mut words = |length| {
                    (0..length)
                        .map(|_| {
                            if dense {
                                u64::MAX
                            } else {
                                purrdf_hash::mix::splitmix64_next(&mut state)
                            }
                        })
                        .collect::<Vec<_>>()
                };
                let a = BigInt::from_parts(false, Mag::Heap(words(la)));
                let b = BigInt::from_parts(false, Mag::Heap(words(lb)));
                let length = ((la + lb) * 4).next_power_of_two();
                let actual = BigInt::from_parts(
                    false,
                    product(&a.magnitude, &b.magnitude, length, &Unbounded).unwrap(),
                );
                assert_eq!(actual, a.mul(&b), "{la}/{lb}, dense={dense}");
            }
        }
    }

    #[test]
    fn selected_products_fit_admission_and_return_supplied_storage() {
        use crate::bigint::LimbScratch;
        use crate::exact::cost::{IntegerOperation, integer_operation_layout};
        use purrdf_alloc_probe::CurrentThreadWindow;

        for limbs in [THRESHOLD - 1, THRESHOLD, THRESHOLD + 1] {
            let bits = u32::try_from(limbs * 64).unwrap();
            let power = BigInt::one().shl(bits);
            let input = power.sub(&BigInt::one());
            let expected = BigInt::one()
                .shl(2 * bits)
                .sub(&power.shl(1))
                .add(&BigInt::one())
                .negated();
            let negative = input.negated();
            let layout =
                integer_operation_layout(limbs, limbs, false, true, IntegerOperation::Multiply)
                    .unwrap();
            let cost = crate::exact::cost::mul(limbs, limbs);
            let window = CurrentThreadWindow::open();
            let result = input.mul_fast(&negative);
            let measured = window.close();
            let peak = u64::try_from(measured.peak_working_bytes).unwrap();
            assert_eq!(result, expected, "{limbs} limbs");
            assert!(
                peak <= layout.required_bytes() as u64,
                "{limbs}: {measured:?}, {layout:?}"
            );
            assert!(peak <= cost.bytes());
            assert!(measured.retained_bytes <= layout.retained_bytes() as i64);
            if let Some(length) = selected_length(limbs, limbs) {
                assert_eq!(measured.allocations, 3);
                assert_eq!(peak, 2 * length * 8);
                assert_eq!(measured.retained_bytes, (2 * limbs * 8) as i64);
            }
        }

        let limbs = THRESHOLD as usize;
        let input = BigInt::one()
            .shl(u32::try_from(THRESHOLD * 64).unwrap())
            .sub(&BigInt::one());
        let length = selected_length(THRESHOLD, THRESHOLD).unwrap() as usize;
        let scratch = LimbScratch::new(2, length).unwrap();
        let window = CurrentThreadWindow::open();
        let result = input.mul_fast_using(&input, &scratch).unwrap();
        let shared = result.clone();
        drop(result);
        assert_eq!(scratch.available(), 1);
        drop(shared);
        assert_eq!(scratch.available(), 2);
        assert_eq!(window.close().allocations, 0);

        let short = LimbScratch::new(1, length).unwrap();
        let window = CurrentThreadWindow::open();
        assert!(matches!(
            input.mul_fast_using(&input, &short),
            Err(LimbScratchError::Exhausted { .. })
        ));
        assert_eq!(short.available(), 1);
        let neighbor = BigInt::one().shl_in(400, &short).unwrap();
        drop(neighbor);
        assert_eq!(short.available(), 1);
        assert_eq!(window.close().allocations, 0);
        assert!(selected_length((1 << 30) + 1, limbs as u64).is_none());
        assert!(selected_length(u64::MAX, THRESHOLD).is_none());
    }
}
