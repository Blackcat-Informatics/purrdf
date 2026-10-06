// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SIMD evaluation of the BLAKE3 specification's independent quarter rounds
//! (single block) and independent chunks (sixteen chunks). No external code.
use crate::blake3::{IV, SCHEDULE};

/// Keep scalar ARX rotations in the same register. With BMI2 enabled, LLVM's
/// three-operand RORX selection increased register pressure and spills across
/// the four independent chains. Paired baseline/native builds favor ROR here.
#[cfg(not(miri))]
#[inline]
pub(crate) fn rotate<const N: u32>(mut value: u32) -> u32 {
    // SAFETY: ROR is baseline x86-64 and touches only its register and flags.
    // The compiler is told flags are clobbered (no `preserves_flags` option).
    // There is no memory access, stack adjustment or dependency on flags.
    unsafe {
        core::arch::asm!(
            "ror {value:e}, {amount}",
            value = inout(reg) value,
            amount = const N,
            options(pure, nomem, nostack),
        );
    }
    value
}

#[cfg(all(test, not(miri)))]
mod scalar_rotation_tests {
    use super::rotate;

    #[test]
    fn rotations_match_integer_arithmetic() {
        let mut value = 0_u32;
        for _ in 0..65536 {
            // An odd increment visits every u32 if allowed to complete the
            // cycle. Include one-hot values separately to pin every bit move.
            for word in [value, !value, 1_u32 << (value & 31)] {
                assert_eq!(rotate::<7>(word), word.rotate_right(7));
                assert_eq!(rotate::<8>(word), word.rotate_right(8));
                assert_eq!(rotate::<12>(word), word.rotate_right(12));
                assert_eq!(rotate::<16>(word), word.rotate_right(16));
            }
            value = value.wrapping_add(0x9e37_79b9);
        }
    }
}
use core::arch::x86_64::{
    _mm_add_epi32, _mm_cvtsi128_si32, _mm_or_si128, _mm_ror_epi32, _mm_setr_epi8, _mm_setr_epi32,
    _mm_shuffle_epi8, _mm_shuffle_epi32, _mm_slli_epi32, _mm_srli_epi32, _mm_storeu_si128,
    _mm_xor_si128, _mm512_add_epi32, _mm512_castsi512_si128, _mm512_loadu_si512,
    _mm512_mask_mov_epi32, _mm512_mask_or_epi32, _mm512_permutexvar_epi32, _mm512_ror_epi32,
    _mm512_set1_epi32, _mm512_setr_epi32, _mm512_setzero_si512, _mm512_shuffle_i32x4,
    _mm512_unpackhi_epi32, _mm512_unpackhi_epi64, _mm512_unpacklo_epi32, _mm512_unpacklo_epi64,
    _mm512_xor_si512,
};

#[target_feature(enable = "ssse3")]
pub(super) unsafe fn single(
    cv: [u32; 8],
    words: &[u32; 16],
    counter: u64,
    length: u32,
    flags: u32,
) -> [u32; 8] {
    let mut a = _mm_setr_epi32(cv[0] as i32, cv[1] as i32, cv[2] as i32, cv[3] as i32);
    let mut b = _mm_setr_epi32(cv[4] as i32, cv[5] as i32, cv[6] as i32, cv[7] as i32);
    let mut c = _mm_setr_epi32(IV[0] as i32, IV[1] as i32, IV[2] as i32, IV[3] as i32);
    let mut d = _mm_setr_epi32(
        counter as i32,
        (counter >> 32) as i32,
        length as i32,
        flags as i32,
    );
    let rotate16 = _mm_setr_epi8(2, 3, 0, 1, 6, 7, 4, 5, 10, 11, 8, 9, 14, 15, 12, 13);
    let rotate8 = _mm_setr_epi8(1, 2, 3, 0, 5, 6, 7, 4, 9, 10, 11, 8, 13, 14, 15, 12);
    macro_rules! g {
        ($x:expr,$y:expr) => {{
            a = _mm_add_epi32(_mm_add_epi32(a, b), $x);
            d = _mm_shuffle_epi8(_mm_xor_si128(d, a), rotate16);
            c = _mm_add_epi32(c, d);
            b = _mm_xor_si128(b, c);
            b = _mm_or_si128(_mm_srli_epi32::<12>(b), _mm_slli_epi32::<20>(b));
            a = _mm_add_epi32(_mm_add_epi32(a, b), $y);
            d = _mm_shuffle_epi8(_mm_xor_si128(d, a), rotate8);
            c = _mm_add_epi32(c, d);
            b = _mm_xor_si128(b, c);
            b = _mm_or_si128(_mm_srli_epi32::<7>(b), _mm_slli_epi32::<25>(b));
        }};
    }
    macro_rules! round {
        ($r:expr) => {{
            let s = SCHEDULE[$r];
            g!(
                _mm_setr_epi32(
                    words[s[0]] as i32,
                    words[s[2]] as i32,
                    words[s[4]] as i32,
                    words[s[6]] as i32
                ),
                _mm_setr_epi32(
                    words[s[1]] as i32,
                    words[s[3]] as i32,
                    words[s[5]] as i32,
                    words[s[7]] as i32
                )
            );
            b = _mm_shuffle_epi32::<0x39>(b);
            c = _mm_shuffle_epi32::<0x4e>(c);
            d = _mm_shuffle_epi32::<0x93>(d);
            g!(
                _mm_setr_epi32(
                    words[s[8]] as i32,
                    words[s[10]] as i32,
                    words[s[12]] as i32,
                    words[s[14]] as i32
                ),
                _mm_setr_epi32(
                    words[s[9]] as i32,
                    words[s[11]] as i32,
                    words[s[13]] as i32,
                    words[s[15]] as i32
                )
            );
            b = _mm_shuffle_epi32::<0x93>(b);
            c = _mm_shuffle_epi32::<0x4e>(c);
            d = _mm_shuffle_epi32::<0x39>(d);
        }};
    }
    round!(0);
    round!(1);
    round!(2);
    round!(3);
    round!(4);
    round!(5);
    round!(6);
    let mut out = [0; 8];
    // SAFETY: each unaligned store writes four words into an eight-word array.
    unsafe {
        _mm_storeu_si128(out.as_mut_ptr().cast(), _mm_xor_si128(a, c));
        _mm_storeu_si128(out.as_mut_ptr().add(4).cast(), _mm_xor_si128(b, d));
    }
    out
}

#[target_feature(enable = "avx512f,avx512vl")]
pub(super) unsafe fn single_avx512(
    cv: [u32; 8],
    words: &[u32; 16],
    counter: u64,
    length: u32,
    flags: u32,
) -> [u32; 8] {
    let mut a = _mm_setr_epi32(cv[0] as i32, cv[1] as i32, cv[2] as i32, cv[3] as i32);
    let mut b = _mm_setr_epi32(cv[4] as i32, cv[5] as i32, cv[6] as i32, cv[7] as i32);
    let mut c = _mm_setr_epi32(IV[0] as i32, IV[1] as i32, IV[2] as i32, IV[3] as i32);
    let mut d = _mm_setr_epi32(
        counter as i32,
        (counter >> 32) as i32,
        length as i32,
        flags as i32,
    );
    // SAFETY: `words` is a full 64-byte block.
    let message = unsafe { _mm512_loadu_si512(words.as_ptr().cast()) };
    macro_rules! message4 {
        ($a:expr,$b:expr,$c:expr,$d:expr) => {
            _mm512_castsi512_si128(_mm512_permutexvar_epi32(
                _mm512_setr_epi32(
                    $a as i32, $b as i32, $c as i32, $d as i32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                ),
                message,
            ))
        };
    }
    macro_rules! g {
        ($x:expr,$y:expr) => {{
            a = _mm_add_epi32(_mm_add_epi32(a, b), $x);
            d = _mm_ror_epi32::<16>(_mm_xor_si128(d, a));
            c = _mm_add_epi32(c, d);
            b = _mm_xor_si128(b, c);
            b = _mm_ror_epi32::<12>(b);
            a = _mm_add_epi32(_mm_add_epi32(a, b), $y);
            d = _mm_ror_epi32::<8>(_mm_xor_si128(d, a));
            c = _mm_add_epi32(c, d);
            b = _mm_xor_si128(b, c);
            b = _mm_ror_epi32::<7>(b);
        }};
    }
    macro_rules! round {
        ($r:expr) => {{
            let s = SCHEDULE[$r];
            g!(
                message4!(s[0], s[2], s[4], s[6]),
                message4!(s[1], s[3], s[5], s[7])
            );
            b = _mm_shuffle_epi32::<0x39>(b);
            c = _mm_shuffle_epi32::<0x4e>(c);
            d = _mm_shuffle_epi32::<0x93>(d);
            g!(
                message4!(s[8], s[10], s[12], s[14]),
                message4!(s[9], s[11], s[13], s[15])
            );
            b = _mm_shuffle_epi32::<0x93>(b);
            c = _mm_shuffle_epi32::<0x4e>(c);
            d = _mm_shuffle_epi32::<0x39>(d);
        }};
    }
    round!(0);
    round!(1);
    round!(2);
    round!(3);
    round!(4);
    round!(5);
    round!(6);
    let mut out = [0; 8];
    // SAFETY: each unaligned store writes four words into an eight-word array.
    unsafe {
        _mm_storeu_si128(out.as_mut_ptr().cast(), _mm_xor_si128(a, c));
        _mm_storeu_si128(out.as_mut_ptr().add(4).cast(), _mm_xor_si128(b, d));
    }
    out
}

#[target_feature(enable = "avx2")]
pub(super) unsafe fn single_avx2(
    cv: [u32; 8],
    words: &[u32; 16],
    counter: u64,
    length: u32,
    flags: u32,
) -> [u32; 8] {
    let mut a = _mm_setr_epi32(cv[0] as i32, cv[1] as i32, cv[2] as i32, cv[3] as i32);
    let mut b = _mm_setr_epi32(cv[4] as i32, cv[5] as i32, cv[6] as i32, cv[7] as i32);
    let mut c = _mm_setr_epi32(IV[0] as i32, IV[1] as i32, IV[2] as i32, IV[3] as i32);
    let mut d = _mm_setr_epi32(
        counter as i32,
        (counter >> 32) as i32,
        length as i32,
        flags as i32,
    );
    // SAFETY: the two loads read the two halves of the 16-word input array.
    let (message0, message1) = unsafe {
        (
            _mm256_loadu_si256(words.as_ptr().cast()),
            _mm256_loadu_si256(words.as_ptr().add(8).cast()),
        )
    };
    let rotate16 = _mm_setr_epi8(2, 3, 0, 1, 6, 7, 4, 5, 10, 11, 8, 9, 14, 15, 12, 13);
    let rotate8 = _mm_setr_epi8(1, 2, 3, 0, 5, 6, 7, 4, 9, 10, 11, 8, 13, 14, 15, 12);
    macro_rules! message4 {
        ($a:expr,$b:expr,$c:expr,$d:expr) => {{
            const MASK: i32 = (($a >= 8) as i32)
                | ((($b >= 8) as i32) << 1)
                | ((($c >= 8) as i32) << 2)
                | ((($d >= 8) as i32) << 3);
            let indices = _mm256_setr_epi32(
                ($a % 8) as i32,
                ($b % 8) as i32,
                ($c % 8) as i32,
                ($d % 8) as i32,
                0,
                0,
                0,
                0,
            );
            let selected = if MASK == 0 {
                _mm256_permutevar8x32_epi32(message0, indices)
            } else if MASK == 15 {
                _mm256_permutevar8x32_epi32(message1, indices)
            } else {
                _mm256_blend_epi32::<MASK>(
                    _mm256_permutevar8x32_epi32(message0, indices),
                    _mm256_permutevar8x32_epi32(message1, indices),
                )
            };
            _mm256_castsi256_si128(selected)
        }};
    }
    macro_rules! g {
        ($x:expr,$y:expr) => {{
            a = _mm_add_epi32(_mm_add_epi32(a, b), $x);
            d = _mm_shuffle_epi8(_mm_xor_si128(d, a), rotate16);
            c = _mm_add_epi32(c, d);
            b = _mm_xor_si128(b, c);
            b = _mm_or_si128(_mm_srli_epi32::<12>(b), _mm_slli_epi32::<20>(b));
            a = _mm_add_epi32(_mm_add_epi32(a, b), $y);
            d = _mm_shuffle_epi8(_mm_xor_si128(d, a), rotate8);
            c = _mm_add_epi32(c, d);
            b = _mm_xor_si128(b, c);
            b = _mm_or_si128(_mm_srli_epi32::<7>(b), _mm_slli_epi32::<25>(b));
        }};
    }
    macro_rules! round {
        ($r:expr) => {{
            const S: [usize; 16] = SCHEDULE[$r];
            g!(
                message4!(S[0], S[2], S[4], S[6]),
                message4!(S[1], S[3], S[5], S[7])
            );
            b = _mm_shuffle_epi32::<0x39>(b);
            c = _mm_shuffle_epi32::<0x4e>(c);
            d = _mm_shuffle_epi32::<0x93>(d);
            g!(
                message4!(S[8], S[10], S[12], S[14]),
                message4!(S[9], S[11], S[13], S[15])
            );
            b = _mm_shuffle_epi32::<0x93>(b);
            c = _mm_shuffle_epi32::<0x4e>(c);
            d = _mm_shuffle_epi32::<0x39>(d);
        }};
    }
    round!(0);
    round!(1);
    round!(2);
    round!(3);
    round!(4);
    round!(5);
    round!(6);
    let mut out = [0; 8];
    // SAFETY: each unaligned store writes four words into an eight-word array.
    unsafe {
        _mm_storeu_si128(out.as_mut_ptr().cast(), _mm_xor_si128(a, c));
        _mm_storeu_si128(out.as_mut_ptr().add(4).cast(), _mm_xor_si128(b, d));
    }
    out
}

pub(crate) fn subtree(bytes: &[u8], counter: u64, height: u32) -> Option<[u32; 8]> {
    if std::is_x86_feature_detected!("avx512f")
        && (1..=4).contains(&height)
        && bytes.len() >= (1024 << height)
    {
        // SAFETY: feature checked; the first 1024 << height bytes contain every live chunk.
        Some(unsafe { subtree_16(bytes, counter, height) })
    } else {
        None
    }
}

macro_rules! wide {
    ($cv:expr, $m:expr, $low:expr, $high:expr, $length:expr, $flags:expr $(,)?) => {{
        let cv = $cv;
        let m = $m;
        let low = $low;
        let high = $high;
        let flags = $flags;
        let length = $length;
        let mut v = [_mm512_setzero_si512(); 16];
        v[..8].copy_from_slice(&cv);
        for i in 0..4 {
            v[8 + i] = _mm512_set1_epi32(IV[i] as i32);
        }
        v[12] = low;
        v[13] = high;
        v[14] = length;
        v[15] = flags;
        macro_rules! g {
            ($a:expr,$b:expr,$c:expr,$d:expr,$x:expr,$y:expr) => {{
                v[$a] = _mm512_add_epi32(_mm512_add_epi32(v[$a], v[$b]), $x);
                v[$d] = _mm512_ror_epi32::<16>(_mm512_xor_si512(v[$d], v[$a]));
                v[$c] = _mm512_add_epi32(v[$c], v[$d]);
                v[$b] = _mm512_ror_epi32::<12>(_mm512_xor_si512(v[$b], v[$c]));
                v[$a] = _mm512_add_epi32(_mm512_add_epi32(v[$a], v[$b]), $y);
                v[$d] = _mm512_ror_epi32::<8>(_mm512_xor_si512(v[$d], v[$a]));
                v[$c] = _mm512_add_epi32(v[$c], v[$d]);
                v[$b] = _mm512_ror_epi32::<7>(_mm512_xor_si512(v[$b], v[$c]));
            }};
        }
        macro_rules! round {
            ($r:expr) => {{
                let s = SCHEDULE[$r];
                g!(0, 4, 8, 12, m[s[0]], m[s[1]]);
                g!(1, 5, 9, 13, m[s[2]], m[s[3]]);
                g!(2, 6, 10, 14, m[s[4]], m[s[5]]);
                g!(3, 7, 11, 15, m[s[6]], m[s[7]]);
                g!(0, 5, 10, 15, m[s[8]], m[s[9]]);
                g!(1, 6, 11, 12, m[s[10]], m[s[11]]);
                g!(2, 7, 8, 13, m[s[12]], m[s[13]]);
                g!(3, 4, 9, 14, m[s[14]], m[s[15]]);
            }};
        }
        round!(0);
        round!(1);
        round!(2);
        round!(3);
        round!(4);
        round!(5);
        round!(6);
        let mut out = [_mm512_setzero_si512(); 8];
        for i in 0..8 {
            out[i] = _mm512_xor_si512(v[i], v[i + 8]);
        }
        out
    }};
}

#[inline]
#[target_feature(enable = "avx512f")]
unsafe fn chunk_columns16(
    bytes: &[u8],
    counter: u64,
    height: u32,
) -> [core::arch::x86_64::__m512i; 8] {
    let mut initial = [_mm512_setzero_si512(); 8];
    for i in 0..8 {
        initial[i] = _mm512_set1_epi32(IV[i] as i32);
    }
    let lows: [u32; 16] = core::array::from_fn(|i| (counter + i as u64) as u32);
    let highs: [u32; 16] = core::array::from_fn(|i| ((counter + i as u64) >> 32) as u32);
    // SAFETY: each load reads a full 16-word local array.
    let (low, high) = unsafe {
        (
            _mm512_loadu_si512(lows.as_ptr().cast()),
            _mm512_loadu_si512(highs.as_ptr().cast()),
        )
    };
    let mut cv = initial;
    for block in 0..16 {
        let mut rows = [_mm512_setzero_si512(); 16];
        for (i, row) in rows.iter_mut().enumerate() {
            // SAFETY: each load is one full block inside a live chunk; inactive lanes repeat live input.
            *row = unsafe {
                _mm512_loadu_si512(
                    bytes
                        .as_ptr()
                        .add((i % (1 << height)) * 1024 + block * 64)
                        .cast(),
                )
            };
        }
        // Transpose sixteen contiguous block loads into word-parallel lanes.
        let mut pairs = [_mm512_setzero_si512(); 16];
        for i in (0..16).step_by(2) {
            pairs[i] = _mm512_unpacklo_epi32(rows[i], rows[i + 1]);
            pairs[i + 1] = _mm512_unpackhi_epi32(rows[i], rows[i + 1]);
        }
        let mut groups = [_mm512_setzero_si512(); 16];
        for i in (0..16).step_by(4) {
            groups[i] = _mm512_unpacklo_epi64(pairs[i], pairs[i + 2]);
            groups[i + 1] = _mm512_unpackhi_epi64(pairs[i], pairs[i + 2]);
            groups[i + 2] = _mm512_unpacklo_epi64(pairs[i + 1], pairs[i + 3]);
            groups[i + 3] = _mm512_unpackhi_epi64(pairs[i + 1], pairs[i + 3]);
        }
        let mut m = [_mm512_setzero_si512(); 16];
        for col in 0..4 {
            let a = _mm512_shuffle_i32x4::<0x44>(groups[col], groups[col + 4]);
            let b = _mm512_shuffle_i32x4::<0x44>(groups[col + 8], groups[col + 12]);
            let c = _mm512_shuffle_i32x4::<0xee>(groups[col], groups[col + 4]);
            let d = _mm512_shuffle_i32x4::<0xee>(groups[col + 8], groups[col + 12]);
            m[col] = _mm512_shuffle_i32x4::<0x88>(a, b);
            m[col + 4] = _mm512_shuffle_i32x4::<0xdd>(a, b);
            m[col + 8] = _mm512_shuffle_i32x4::<0x88>(c, d);
            m[col + 12] = _mm512_shuffle_i32x4::<0xdd>(c, d);
        }
        cv = wide!(
            cv,
            m,
            low,
            high,
            _mm512_set1_epi32(64),
            _mm512_set1_epi32(if block == 0 {
                1
            } else if block == 15 {
                2
            } else {
                0
            }),
        );
    }
    cv
}

#[target_feature(enable = "avx512f")]
unsafe fn subtree_16(bytes: &[u8], counter: u64, height: u32) -> [u32; 8] {
    // SAFETY: caller checked feature support and the complete live chunks.
    let mut cv = unsafe { chunk_columns16(bytes, counter, height) };
    let initial: [_; 8] = core::array::from_fn(|i| _mm512_set1_epi32(IV[i] as i32));
    // Four levels of binary parents. Only the leading 8,4,2,1 lanes are
    // live; permuting unused lanes is harmless and does not access memory.
    let even = _mm512_setr_epi32(0, 2, 4, 6, 8, 10, 12, 14, 0, 0, 0, 0, 0, 0, 0, 0);
    let odd = _mm512_setr_epi32(1, 3, 5, 7, 9, 11, 13, 15, 0, 0, 0, 0, 0, 0, 0, 0);
    for _ in 0..height {
        let mut m = [_mm512_setzero_si512(); 16];
        for i in 0..8 {
            m[i] = _mm512_permutexvar_epi32(even, cv[i]);
            m[i + 8] = _mm512_permutexvar_epi32(odd, cv[i]);
        }
        cv = wide!(
            initial,
            m,
            _mm512_setzero_si512(),
            _mm512_setzero_si512(),
            _mm512_set1_epi32(64),
            _mm512_set1_epi32(4),
        );
    }
    let mut out = [0; 8];
    for i in 0..8 {
        out[i] = _mm_cvtsi128_si32(_mm512_castsi512_si128(cv[i])) as u32;
    }
    out
}

/// Returns the two final child CVs so the caller can apply ROOT exactly once.
pub(crate) fn small_output(bytes: &[u8], counter: u64) -> Option<[u32; 16]> {
    if std::is_x86_feature_detected!("avx512f") && (1025..=16384).contains(&bytes.len()) {
        // SAFETY: feature and length checked; the kernel pads all SIMD loads.
        Some(unsafe { small_parent(bytes, counter) })
    } else {
        None
    }
}
#[target_feature(enable = "avx512f")]
unsafe fn small_parent(bytes: &[u8], counter: u64) -> [u32; 16] {
    let mut padded = [0u8; 16384];
    padded[..bytes.len()].copy_from_slice(bytes);
    let mut count = bytes.len().div_ceil(1024);
    let last_length = (bytes.len() - 1) % 1024 + 1;
    let last_block = (last_length - 1) / 64;
    let last_mask = 1u16 << (count - 1);
    let mut initial = [_mm512_setzero_si512(); 8];
    for i in 0..8 {
        initial[i] = _mm512_set1_epi32(IV[i] as i32);
    }
    let lows: [u32; 16] = core::array::from_fn(|i| (counter + i as u64) as u32);
    let highs: [u32; 16] = core::array::from_fn(|i| ((counter + i as u64) >> 32) as u32);
    // SAFETY: each load reads a full local array.
    let (low, high) = unsafe {
        (
            _mm512_loadu_si512(lows.as_ptr().cast()),
            _mm512_loadu_si512(highs.as_ptr().cast()),
        )
    };
    let mut cv = initial;
    for block in 0..16 {
        let mut rows = [_mm512_setzero_si512(); 16];
        for (i, row) in rows.iter_mut().enumerate() {
            // SAFETY: each load is one full block inside a live chunk; inactive lanes repeat live input.
            *row = unsafe { _mm512_loadu_si512(padded.as_ptr().add(i * 1024 + block * 64).cast()) };
        }
        // Transpose sixteen contiguous block loads into word-parallel lanes.
        let mut pairs = [_mm512_setzero_si512(); 16];
        for i in (0..16).step_by(2) {
            pairs[i] = _mm512_unpacklo_epi32(rows[i], rows[i + 1]);
            pairs[i + 1] = _mm512_unpackhi_epi32(rows[i], rows[i + 1]);
        }
        let mut groups = [_mm512_setzero_si512(); 16];
        for i in (0..16).step_by(4) {
            groups[i] = _mm512_unpacklo_epi64(pairs[i], pairs[i + 2]);
            groups[i + 1] = _mm512_unpackhi_epi64(pairs[i], pairs[i + 2]);
            groups[i + 2] = _mm512_unpacklo_epi64(pairs[i + 1], pairs[i + 3]);
            groups[i + 3] = _mm512_unpackhi_epi64(pairs[i + 1], pairs[i + 3]);
        }
        let mut m = [_mm512_setzero_si512(); 16];
        for col in 0..4 {
            let a = _mm512_shuffle_i32x4::<0x44>(groups[col], groups[col + 4]);
            let b = _mm512_shuffle_i32x4::<0x44>(groups[col + 8], groups[col + 12]);
            let c = _mm512_shuffle_i32x4::<0xee>(groups[col], groups[col + 4]);
            let d = _mm512_shuffle_i32x4::<0xee>(groups[col + 8], groups[col + 12]);
            m[col] = _mm512_shuffle_i32x4::<0x88>(a, b);
            m[col + 4] = _mm512_shuffle_i32x4::<0xdd>(a, b);
            m[col + 8] = _mm512_shuffle_i32x4::<0x88>(c, d);
            m[col + 12] = _mm512_shuffle_i32x4::<0xdd>(c, d);
        }

        let flags = _mm512_set1_epi32(if block == 0 {
            1
        } else if block == 15 {
            2
        } else {
            0
        });
        let flags = if block == last_block {
            _mm512_mask_or_epi32(flags, last_mask, flags, _mm512_set1_epi32(2))
        } else {
            flags
        };
        let length = if block == last_block {
            _mm512_mask_mov_epi32(
                _mm512_set1_epi32(64),
                last_mask,
                _mm512_set1_epi32(((last_length - 1) % 64 + 1) as i32),
            )
        } else {
            _mm512_set1_epi32(64)
        };
        let mut next = wide!(cv, m, low, high, length, flags);
        if block > last_block {
            for i in 0..8 {
                next[i] = _mm512_mask_mov_epi32(next[i], last_mask, cv[i]);
            }
        }
        cv = next;
    }
    let even = _mm512_setr_epi32(0, 2, 4, 6, 8, 10, 12, 14, 0, 0, 0, 0, 0, 0, 0, 0);
    let odd = _mm512_setr_epi32(1, 3, 5, 7, 9, 11, 13, 15, 0, 0, 0, 0, 0, 0, 0, 0);
    while count > 2 {
        let mut m = [_mm512_setzero_si512(); 16];
        for i in 0..8 {
            m[i] = _mm512_permutexvar_epi32(even, cv[i]);
            m[i + 8] = _mm512_permutexvar_epi32(odd, cv[i]);
        }
        let mut next = wide!(
            initial,
            m,
            _mm512_setzero_si512(),
            _mm512_setzero_si512(),
            _mm512_set1_epi32(64),
            _mm512_set1_epi32(4)
        );
        if !count.is_multiple_of(2) {
            let last = _mm512_set1_epi32((count - 1) as i32);
            for i in 0..8 {
                next[i] = _mm512_mask_mov_epi32(
                    next[i],
                    1 << (count / 2),
                    _mm512_permutexvar_epi32(last, cv[i]),
                );
            }
        }
        cv = next;
        count = count.div_ceil(2);
    }
    let mut out = [0; 16];
    for i in 0..8 {
        let low = _mm512_castsi512_si128(cv[i]);
        out[i] = _mm_cvtsi128_si32(low) as u32;
        out[i + 8] = _mm_cvtsi128_si32(_mm_shuffle_epi32::<0x55>(low)) as u32;
    }
    out
}

use core::arch::x86_64::{
    _mm256_blend_epi32, _mm256_castsi256_si128, _mm256_loadu_si256, _mm256_permutevar8x32_epi32,
    _mm256_setr_epi32,
};

pub(crate) fn chunks16(bytes: &[u8], counter: u64) -> Option<[[u32; 8]; 16]> {
    if std::is_x86_feature_detected!("avx512f") && bytes.len() == 16384 {
        // SAFETY: the feature and complete input batch were checked.
        Some(unsafe { chunk_cvs16(bytes, counter) })
    } else {
        None
    }
}

#[target_feature(enable = "avx512f")]
unsafe fn chunk_cvs16(bytes: &[u8], counter: u64) -> [[u32; 8]; 16] {
    assert_eq!(bytes.len(), 16384);
    // SAFETY: complete sixteen-chunk batch; feature enabled by this function.
    let cv = unsafe { chunk_columns16(bytes, counter, 4) };
    let mut columns = [[0u32; 16]; 8];
    for (dest, column) in columns.iter_mut().zip(cv) {
        // SAFETY: the destination contains sixteen writable words.
        unsafe { core::arch::x86_64::_mm512_storeu_si512(dest.as_mut_ptr().cast(), column) };
    }
    core::array::from_fn(|lane| core::array::from_fn(|word| columns[word][lane]))
}

pub(crate) fn parents16(children: &[[u32; 8]; 32]) -> Option<[[u32; 8]; 16]> {
    if std::is_x86_feature_detected!("avx512f") {
        // SAFETY: feature checked; the array contains all 32 child CVs.
        Some(unsafe { parents16_inner(children) })
    } else {
        None
    }
}
#[target_feature(enable = "avx512f")]
unsafe fn parents16_inner(children: &[[u32; 8]; 32]) -> [[u32; 8]; 16] {
    // SAFETY: each row reads two consecutive eight-word CVs, inside the array.
    let rows: [_; 16] = core::array::from_fn(|i| unsafe {
        _mm512_loadu_si512(children.as_ptr().add(i * 2).cast())
    });
    // Transpose sixteen contiguous block loads into word-parallel lanes.
    let mut pairs = [_mm512_setzero_si512(); 16];
    for i in (0..16).step_by(2) {
        pairs[i] = _mm512_unpacklo_epi32(rows[i], rows[i + 1]);
        pairs[i + 1] = _mm512_unpackhi_epi32(rows[i], rows[i + 1]);
    }
    let mut groups = [_mm512_setzero_si512(); 16];
    for i in (0..16).step_by(4) {
        groups[i] = _mm512_unpacklo_epi64(pairs[i], pairs[i + 2]);
        groups[i + 1] = _mm512_unpackhi_epi64(pairs[i], pairs[i + 2]);
        groups[i + 2] = _mm512_unpacklo_epi64(pairs[i + 1], pairs[i + 3]);
        groups[i + 3] = _mm512_unpackhi_epi64(pairs[i + 1], pairs[i + 3]);
    }
    let mut m = [_mm512_setzero_si512(); 16];
    for col in 0..4 {
        let a = _mm512_shuffle_i32x4::<0x44>(groups[col], groups[col + 4]);
        let b = _mm512_shuffle_i32x4::<0x44>(groups[col + 8], groups[col + 12]);
        let c = _mm512_shuffle_i32x4::<0xee>(groups[col], groups[col + 4]);
        let d = _mm512_shuffle_i32x4::<0xee>(groups[col + 8], groups[col + 12]);
        m[col] = _mm512_shuffle_i32x4::<0x88>(a, b);
        m[col + 4] = _mm512_shuffle_i32x4::<0xdd>(a, b);
        m[col + 8] = _mm512_shuffle_i32x4::<0x88>(c, d);
        m[col + 12] = _mm512_shuffle_i32x4::<0xdd>(c, d);
    }

    let initial: [_; 8] = core::array::from_fn(|i| _mm512_set1_epi32(IV[i] as i32));
    let cv = wide!(
        initial,
        m,
        _mm512_setzero_si512(),
        _mm512_setzero_si512(),
        _mm512_set1_epi32(64),
        _mm512_set1_epi32(4)
    );
    let mut columns = [[0u32; 16]; 8];
    for (dest, column) in columns.iter_mut().zip(cv) {
        // SAFETY: the destination contains sixteen writable words.
        unsafe { core::arch::x86_64::_mm512_storeu_si512(dest.as_mut_ptr().cast(), column) };
    }
    core::array::from_fn(|lane| core::array::from_fn(|word| columns[word][lane]))
}
