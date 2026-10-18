// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Recursive block division over the same normalized limb representation.

use super::{
    Allocate, BigInt, LimbScratchError, Mag, divide_normalized, mag_add, mag_cmp, mag_shl,
    mag_shr_in_place, mag_sub, mag_sub_in_place, mag_trim,
};
use core::cmp::Ordering;

pub(crate) const DIVISION_BLOCK_THRESHOLD: usize = 2_048;
const BASE_DIVISOR_LIMBS: usize = 128;
// Every recursive split must remain even until this direct base is reached.
const BASE_DIVISION_CUTOFF: usize = 272;

/// A divisor width with enough factors of two for every recursive split.
/// The minimum alignment preserves ordinary block widths; larger inputs need
/// a larger alignment so the final child reaches the base before becoming odd.
pub(crate) const fn normalized_divisor_limbs(limbs: u64) -> Option<u64> {
    if limbs == 0 {
        return None;
    }
    let groups = limbs.div_ceil(BASE_DIVISION_CUTOFF as u64);
    let Some(alignment) = groups.checked_next_power_of_two() else {
        return None;
    };
    let alignment = if alignment < BASE_DIVISOR_LIMBS as u64 {
        BASE_DIVISOR_LIMBS as u64
    } else {
        alignment
    };
    limbs.div_ceil(alignment).checked_mul(alignment)
}

pub(super) fn use_block_division(numerator: usize, divisor: usize) -> bool {
    divisor >= DIVISION_BLOCK_THRESHOLD
        && numerator.saturating_sub(divisor).saturating_add(1) >= divisor
}

/// Normalize both operands once; process high-to-low divisor-sized blocks.
pub(super) fn divide_blocks(
    a: &[u64],
    b: &[u64],
    storage: &impl Allocate,
) -> Result<(Mag, Mag), LimbScratchError> {
    let limbs = u64::try_from(b.len()).map_err(|_| LimbScratchError::SizeOverflow)?;
    let n = normalized_divisor_limbs(limbs).ok_or(LimbScratchError::SizeOverflow)?;
    let n = usize::try_from(n).map_err(|_| LimbScratchError::SizeOverflow)?;
    let pad_bits = n
        .checked_sub(b.len())
        .and_then(|words| words.checked_mul(64))
        .ok_or(LimbScratchError::SizeOverflow)?;
    let shift = u64::try_from(pad_bits)
        .map_err(|_| LimbScratchError::SizeOverflow)?
        .checked_add(u64::from(b[b.len() - 1].leading_zeros()))
        .ok_or(LimbScratchError::SizeOverflow)?;
    let numerator = mag_shl(a, shift, storage)?;
    let divisor = mag_shl(b, shift, storage)?;
    if divisor.len() != n || divisor[n - 1] >> 63 != 1 {
        return Err(LimbScratchError::SizeOverflow);
    }
    let quotient_len = a
        .len()
        .checked_sub(b.len())
        .and_then(|length| length.checked_add(1))
        .ok_or(LimbScratchError::SizeOverflow)?;
    let mut quotient = storage.destination(quotient_len, quotient_len)?;
    let mut remainder = Mag::new();
    for (block_index, block) in numerator.chunks(n).enumerate().rev() {
        let length = remainder
            .len()
            .checked_add(n)
            .ok_or(LimbScratchError::SizeOverflow)?;
        let mut combined = storage.destination(length, length)?;
        combined[..block.len()].copy_from_slice(block);
        combined[n..n + remainder.len()].copy_from_slice(&remainder);
        mag_trim(&mut combined);
        drop(remainder);
        let (digits, next_remainder) = div_2n_1n(&combined, &divisor, n, storage)?;
        drop(combined);
        let offset = block_index
            .checked_mul(n)
            .ok_or(LimbScratchError::SizeOverflow)?;
        if !digits.is_empty() {
            let end = offset
                .checked_add(digits.len())
                .ok_or(LimbScratchError::SizeOverflow)?;
            let slots = quotient
                .get_mut(offset..end)
                .ok_or(LimbScratchError::SizeOverflow)?;
            slots.copy_from_slice(&digits);
        }
        remainder = next_remainder;
    }
    mag_trim(&mut quotient);
    mag_shr_in_place(&mut remainder, shift);
    Ok((quotient, remainder))
}

/// Divide at most 2n normalized limbs by n normalized limbs; q fits n limbs.
fn div_2n_1n(
    a: &[u64],
    b: &[u64],
    n: usize,
    storage: &impl Allocate,
) -> Result<(Mag, Mag), LimbScratchError> {
    if mag_cmp(a, b) == Ordering::Less {
        return Ok((Mag::new(), storage.copy(a)?));
    }
    if n <= BASE_DIVISION_CUTOFF {
        let length = n.checked_mul(2).ok_or(LimbScratchError::SizeOverflow)?;
        let mut dividend = storage.destination(length, length)?;
        dividend[..a.len()].copy_from_slice(a);
        let mut quotient = storage.destination(n, n)?;
        divide_normalized(&mut dividend, b, &mut quotient);
        dividend.resize(n, 0);
        mag_trim(&mut dividend);
        mag_trim(&mut quotient);
        return Ok((quotient, dividend));
    }
    let h = n / 2;
    let high = a.get(h..).unwrap_or(&[]);
    let (high_quotient, high_remainder) = div_3n_2n(high, b, n, storage)?;
    let length = n.checked_add(h).ok_or(LimbScratchError::SizeOverflow)?;
    let mut low_dividend = storage.destination(length, length)?;
    let low = &a[..a.len().min(h)];
    low_dividend[..low.len()].copy_from_slice(low);
    low_dividend[h..h + high_remainder.len()].copy_from_slice(&high_remainder);
    drop(high_remainder);
    mag_trim(&mut low_dividend);
    let (low_quotient, remainder) = div_3n_2n(&low_dividend, b, n, storage)?;
    if low_quotient.len() > h || high_quotient.len() > h {
        return Err(LimbScratchError::SizeOverflow);
    }
    let mut quotient = storage.destination(n, n)?;
    quotient[..low_quotient.len()].copy_from_slice(&low_quotient);
    quotient[h..h + high_quotient.len()].copy_from_slice(&high_quotient);
    mag_trim(&mut quotient);
    Ok((quotient, remainder))
}

/// Divide at most 3h limbs by 2h normalized limbs; q fits h limbs.
fn div_3n_2n(
    a: &[u64],
    b: &[u64],
    n: usize,
    storage: &impl Allocate,
) -> Result<(Mag, Mag), LimbScratchError> {
    if mag_cmp(a, b) == Ordering::Less {
        return Ok((Mag::new(), storage.copy(a)?));
    }
    let h = n / 2;
    let high_dividend = a.get(2 * h..).unwrap_or(&[]);
    let high_divisor = &b[h..];
    let (estimate, high_remainder) = match mag_cmp(high_dividend, high_divisor) {
        Ordering::Less => div_2n_1n(a.get(h..).unwrap_or(&[]), high_divisor, h, storage)?,
        Ordering::Equal => {
            let mut estimate = storage.destination(h, h)?;
            estimate.fill(u64::MAX);
            let middle = a.get(h..a.len().min(2 * h)).unwrap_or(&[]);
            (estimate, mag_add(middle, high_divisor, storage)?)
        }
        Ordering::Greater => return Err(LimbScratchError::SizeOverflow),
    };
    let length = h.checked_mul(3).ok_or(LimbScratchError::SizeOverflow)?;
    let mut assembled = storage.destination(length, length)?;
    let low = &a[..a.len().min(h)];
    assembled[..low.len()].copy_from_slice(low);
    assembled[h..h + high_remainder.len()].copy_from_slice(&high_remainder);
    drop(high_remainder);
    // Move the estimate into the existing multiplication home; only the
    // borrowed low divisor needs a new admitted owner. Recover both owned
    // magnitudes without cloning them after the product is constructed.
    let estimate_value = BigInt::from_parts(false, estimate);
    let low_divisor = BigInt::from_parts(false, storage.copy(&b[..h])?);
    let product = estimate_value.mul_fast_using(&low_divisor, storage)?;
    drop(low_divisor);
    let mut estimate = estimate_value.magnitude;
    let mut product = product.magnitude;
    mag_trim(&mut assembled);
    let mut remainder = if mag_cmp(&assembled, &product) == Ordering::Less {
        mag_sub_in_place(&mut product, &assembled);
        drop(assembled);
        // The normalized estimate overshoots by at most two. Retain the
        // signed deficit until each correction has been accounted for.
        if mag_cmp(&product, b) == Ordering::Greater {
            mag_sub_in_place(&mut product, b);
            mag_sub_in_place(&mut estimate, &[1]);
        }
        if mag_cmp(&product, b) == Ordering::Greater {
            return Err(LimbScratchError::SizeOverflow);
        }
        mag_sub_in_place(&mut estimate, &[1]);
        mag_sub(b, &product, storage)?
    } else {
        mag_sub_in_place(&mut assembled, &product);
        assembled
    };
    if mag_cmp(&remainder, b) != Ordering::Less || estimate.len() > h {
        return Err(LimbScratchError::SizeOverflow);
    }
    mag_trim(&mut remainder);
    Ok((estimate, remainder))
}
