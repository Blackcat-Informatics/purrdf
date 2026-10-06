// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared BLAKE3 lane arithmetic. Each instantiation supplies ISA operations;
//! the message schedule, flags and tree shape have one definition.

macro_rules! blake3_lanes {
    ($feature:literal, $lanes:literal) => {
        use crate::blake3::{IV, SCHEDULE};
        const LANES: usize = $lanes;
        macro_rules! lane_compress {
            ($cv:expr,$m:expr,$low:expr,$high:expr,$length:expr,$flags:expr) => {{
                let cv = $cv;
                let m = $m;
                let low = $low;
                let high = $high;
                let length = $length;
                let flags = $flags;
                let mut v = [V::splat(0); 16];
                v[..8].copy_from_slice(&cv);
                for i in 0..4 {
                    v[8 + i] = V::splat(IV[i]);
                }
                v[12] = low;
                v[13] = high;
                v[14] = length;
                v[15] = flags;
                macro_rules! mix {
                    ($a:expr,$b:expr,$c:expr,$d:expr,$x:expr,$y:expr) => {{
                        v[$a] = v[$a].add(v[$b]).add($x);
                        v[$d] = v[$d].xor(v[$a]).rotr::<16, 16>();
                        v[$c] = v[$c].add(v[$d]);
                        v[$b] = v[$b].xor(v[$c]).rotr::<12, 20>();
                        v[$a] = v[$a].add(v[$b]).add($y);
                        v[$d] = v[$d].xor(v[$a]).rotr::<8, 24>();
                        v[$c] = v[$c].add(v[$d]);
                        v[$b] = v[$b].xor(v[$c]).rotr::<7, 25>();
                    }};
                }
                macro_rules! round {
                    ($r:expr) => {{
                        let s = SCHEDULE[$r];
                        mix!(0, 4, 8, 12, m[s[0]], m[s[1]]);
                        mix!(1, 5, 9, 13, m[s[2]], m[s[3]]);
                        mix!(2, 6, 10, 14, m[s[4]], m[s[5]]);
                        mix!(3, 7, 11, 15, m[s[6]], m[s[7]]);
                        mix!(0, 5, 10, 15, m[s[8]], m[s[9]]);
                        mix!(1, 6, 11, 12, m[s[10]], m[s[11]]);
                        mix!(2, 7, 8, 13, m[s[12]], m[s[13]]);
                        mix!(3, 4, 9, 14, m[s[14]], m[s[15]]);
                    }};
                }
                round!(0);
                round!(1);
                round!(2);
                round!(3);
                round!(4);
                round!(5);
                round!(6);
                let mut out = [V::splat(0); 8];
                for i in 0..8 {
                    out[i] = v[i].xor(v[i + 8]);
                }
                out
            }};
        }

        /// A full SIMD batch of parent nodes. Each lane hashes two child CVs.
        #[target_feature(enable=$feature)]
        pub(crate) fn parents(children: &[[u32; 8]; 2 * LANES]) -> [[u32; 8]; LANES] {
            let mut cv = [V::splat(0); 8];
            for i in 0..8 {
                cv[i] = V::splat(IV[i]);
            }
            let mut m = [V::splat(0); 16];
            for i in 0..8 {
                m[i] = V::load(core::array::from_fn(|lane| children[2 * lane][i]));
                m[i + 8] = V::load(core::array::from_fn(|lane| children[2 * lane + 1][i]));
            }
            let cv = lane_compress!(cv, m, V::splat(0), V::splat(0), V::splat(64), V::splat(4));
            let mut out = [[0; 8]; LANES];
            for (word, column) in cv.into_iter().enumerate() {
                for (lane, value) in column.array().into_iter().enumerate() {
                    out[lane][word] = value;
                }
            }
            out
        }

        // Full batches need neither a padded copy nor per-lane tail masks.
        // Keep this separate so the compiler sees uniform lengths and flags.
        #[target_feature(enable=$feature)]
        fn full_chunk_cvs(bytes: &[u8; 1024 * LANES], counter: u64) -> [[u32; 8]; LANES] {
            let mut cv = [V::splat(0); 8];
            for i in 0..8 {
                cv[i] = V::splat(IV[i]);
            }
            let low = V::load(core::array::from_fn(|i| (counter + i as u64) as u32));
            let high = V::load(core::array::from_fn(|i| {
                ((counter + i as u64) >> 32) as u32
            }));
            for block in 0..16 {
                let m = load_block(bytes, block);
                let flags = V::splat(u32::from(block == 0) | (u32::from(block == 15) << 1));
                cv = lane_compress!(cv, m, low, high, V::splat(64), flags);
            }
            let mut result = [[0; 8]; LANES];
            for (word, column) in cv.into_iter().enumerate() {
                for (lane, value) in column.array().into_iter().enumerate() {
                    result[lane][word] = value;
                }
            }
            result
        }

        /// Chaining values for the live chunks. Inactive lanes duplicate the
        /// first valid chunk; all pointer loads stay within the padded buffer.
        #[target_feature(enable=$feature)]
        pub(crate) fn chunk_cvs(bytes: &[u8], counter: u64) -> [[u32; 8]; LANES] {
            assert!(!bytes.is_empty() && bytes.len() <= 1024 * LANES);
            if let Ok(full) = <&[u8; 1024 * LANES]>::try_from(bytes) {
                return full_chunk_cvs(full, counter);
            }
            let mut padded = [0u8; 1024 * LANES];
            padded[..bytes.len()].copy_from_slice(bytes);
            let count = bytes.len().div_ceil(1024);
            let last_length = (bytes.len() - 1) % 1024 + 1;
            let last_block = (last_length - 1) / 64;
            let mut cv = [V::splat(0); 8];
            for i in 0..8 {
                cv[i] = V::splat(IV[i]);
            }
            let low = V::load(core::array::from_fn(|i| (counter + i as u64) as u32));
            let high = V::load(core::array::from_fn(|i| {
                ((counter + i as u64) >> 32) as u32
            }));
            for block in 0..16 {
                let m = load_block(&padded, block);
                let flags = V::load(core::array::from_fn(|i| {
                    let end = if i + 1 == count { last_block } else { 15 };
                    u32::from(block == 0) | (u32::from(block == end) << 1)
                }));
                let length = V::load(core::array::from_fn(|i| {
                    if i + 1 == count && block == last_block {
                        ((last_length - 1) % 64 + 1) as u32
                    } else {
                        64
                    }
                }));
                let mut next = lane_compress!(cv, m, low, high, length, flags);
                if block > last_block {
                    // Keep the completed final chunk; all earlier chunks
                    // have sixteen blocks. This mask depends only on length.
                    let keep = V::load(core::array::from_fn(|i| {
                        if i + 1 == count { u32::MAX } else { 0 }
                    }));
                    for i in 0..8 {
                        next[i] = next[i].select(keep, cv[i]);
                    }
                }
                cv = next;
            }
            let mut result = [[0; 8]; LANES];
            for (word, column) in cv.into_iter().enumerate() {
                for (lane, value) in column.array().into_iter().enumerate() {
                    result[lane][word] = value;
                }
            }
            result
        }
    };
}
pub(crate) use blake3_lanes;

macro_rules! blake3_single4 {
    ($feature:literal) => {
        /// One block: four independent quarter rounds occupy four SIMD lanes.
        // Keep the kernel out of the small dispatch wrapper so that wrapper
        // can inline into chunk loops without duplicating seven rounds.
        #[inline(never)]
        #[target_feature(enable=$feature)]
        pub(super) fn single(
            cv: [u32; 8],
            words: &[u32; 16],
            counter: u64,
            length: u32,
            flags: u32,
        ) -> [u32; 8] {
            let mut a = V::load([cv[0], cv[1], cv[2], cv[3]]);
            let mut b = V::load([cv[4], cv[5], cv[6], cv[7]]);
            let mut c = V::load([IV[0], IV[1], IV[2], IV[3]]);
            let mut d = V::load([counter as u32, (counter >> 32) as u32, length, flags]);
            macro_rules! g {
                ($x:expr,$y:expr) => {{
                    a = a.add(b).add($x);
                    d = d.xor(a).rotr::<16, 16>();
                    c = c.add(d);
                    b = b.xor(c).rotr::<12, 20>();
                    a = a.add(b).add($y);
                    d = d.xor(a).rotr::<8, 24>();
                    c = c.add(d);
                    b = b.xor(c).rotr::<7, 25>();
                }};
            }
            macro_rules! round {
                ($r:expr) => {{
                    let s = SCHEDULE[$r];
                    g!(
                        V::load([words[s[0]], words[s[2]], words[s[4]], words[s[6]]]),
                        V::load([words[s[1]], words[s[3]], words[s[5]], words[s[7]]])
                    );
                    b = b.cycle1();
                    c = c.cycle2();
                    d = d.cycle3();
                    g!(
                        V::load([words[s[8]], words[s[10]], words[s[12]], words[s[14]]]),
                        V::load([words[s[9]], words[s[11]], words[s[13]], words[s[15]]])
                    );
                    b = b.cycle3();
                    c = c.cycle2();
                    d = d.cycle1();
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
            out[..4].copy_from_slice(&a.xor(c).array());
            out[4..].copy_from_slice(&b.xor(d).array());
            out
        }
    };
}
pub(crate) use blake3_single4;
