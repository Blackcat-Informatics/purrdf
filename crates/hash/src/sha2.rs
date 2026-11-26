// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Byte-oriented SHA-224, SHA-256, SHA-384 and SHA-512 (FIPS 180-4).

use crate::block::{BlockBuffer, pad_md, pad_md_length_block};
use crate::digest::Digest;
use crate::{Backend, SecretArray};

pub(crate) mod batch;
pub use batch::{Sha256Batch, Sha256BatchBackend};

pub(crate) type Sha256Blocks = fn(&mut [u32; 8], &[u8], &[u8]);
pub(crate) type Sha512Blocks = fn(&mut [u64; 8], &[u8], &[u8]);

// FIPS 180-4 §4.2.2–3 and §5.3.2–5.
pub(crate) const K32: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

pub(crate) const K64: [u64; 80] = [
    0x428a_2f98_d728_ae22,
    0x7137_4491_23ef_65cd,
    0xb5c0_fbcf_ec4d_3b2f,
    0xe9b5_dba5_8189_dbbc,
    0x3956_c25b_f348_b538,
    0x59f1_11f1_b605_d019,
    0x923f_82a4_af19_4f9b,
    0xab1c_5ed5_da6d_8118,
    0xd807_aa98_a303_0242,
    0x1283_5b01_4570_6fbe,
    0x2431_85be_4ee4_b28c,
    0x550c_7dc3_d5ff_b4e2,
    0x72be_5d74_f27b_896f,
    0x80de_b1fe_3b16_96b1,
    0x9bdc_06a7_25c7_1235,
    0xc19b_f174_cf69_2694,
    0xe49b_69c1_9ef1_4ad2,
    0xefbe_4786_384f_25e3,
    0x0fc1_9dc6_8b8c_d5b5,
    0x240c_a1cc_77ac_9c65,
    0x2de9_2c6f_592b_0275,
    0x4a74_84aa_6ea6_e483,
    0x5cb0_a9dc_bd41_fbd4,
    0x76f9_88da_8311_53b5,
    0x983e_5152_ee66_dfab,
    0xa831_c66d_2db4_3210,
    0xb003_27c8_98fb_213f,
    0xbf59_7fc7_beef_0ee4,
    0xc6e0_0bf3_3da8_8fc2,
    0xd5a7_9147_930a_a725,
    0x06ca_6351_e003_826f,
    0x1429_2967_0a0e_6e70,
    0x27b7_0a85_46d2_2ffc,
    0x2e1b_2138_5c26_c926,
    0x4d2c_6dfc_5ac4_2aed,
    0x5338_0d13_9d95_b3df,
    0x650a_7354_8baf_63de,
    0x766a_0abb_3c77_b2a8,
    0x81c2_c92e_47ed_aee6,
    0x9272_2c85_1482_353b,
    0xa2bf_e8a1_4cf1_0364,
    0xa81a_664b_bc42_3001,
    0xc24b_8b70_d0f8_9791,
    0xc76c_51a3_0654_be30,
    0xd192_e819_d6ef_5218,
    0xd699_0624_5565_a910,
    0xf40e_3585_5771_202a,
    0x106a_a070_32bb_d1b8,
    0x19a4_c116_b8d2_d0c8,
    0x1e37_6c08_5141_ab53,
    0x2748_774c_df8e_eb99,
    0x34b0_bcb5_e19b_48a8,
    0x391c_0cb3_c5c9_5a63,
    0x4ed8_aa4a_e341_8acb,
    0x5b9c_ca4f_7763_e373,
    0x682e_6ff3_d6b2_b8a3,
    0x748f_82ee_5def_b2fc,
    0x78a5_636f_4317_2f60,
    0x84c8_7814_a1f0_ab72,
    0x8cc7_0208_1a64_39ec,
    0x90be_fffa_2363_1e28,
    0xa450_6ceb_de82_bde9,
    0xbef9_a3f7_b2c6_7915,
    0xc671_78f2_e372_532b,
    0xca27_3ece_ea26_619c,
    0xd186_b8c7_21c0_c207,
    0xeada_7dd6_cde0_eb1e,
    0xf57d_4f7f_ee6e_d178,
    0x06f0_67aa_7217_6fba,
    0x0a63_7dc5_a2c8_98a6,
    0x113f_9804_bef9_0dae,
    0x1b71_0b35_131c_471b,
    0x28db_77f5_2304_7d84,
    0x32ca_ab7b_40c7_2493,
    0x3c9e_be0a_15c9_bebc,
    0x431d_67c4_9c10_0d4c,
    0x4cc5_d4be_cb3e_42b6,
    0x597f_299c_fc65_7e2a,
    0x5fcb_6fab_3ad6_faec,
    0x6c44_198c_4a47_5817,
];

const IV224: [u32; 8] = [
    0xc105_9ed8,
    0x367c_d507,
    0x3070_dd17,
    0xf70e_5939,
    0xffc0_0b31,
    0x6858_1511,
    0x64f9_8fa7,
    0xbefa_4fa4,
];

pub(crate) const IV256: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

const IV384: [u64; 8] = [
    0xcbbb_9d5d_c105_9ed8,
    0x629a_292a_367c_d507,
    0x9159_015a_3070_dd17,
    0x152f_ecd8_f70e_5939,
    0x6733_2667_ffc0_0b31,
    0x8eb4_4a87_6858_1511,
    0xdb0c_2e0d_64f9_8fa7,
    0x47b5_481d_befa_4fa4,
];

const IV512: [u64; 8] = [
    0x6a09_e667_f3bc_c908,
    0xbb67_ae85_84ca_a73b,
    0x3c6e_f372_fe94_f82b,
    0xa54f_f53a_5f1d_36f1,
    0x510e_527f_ade6_82d1,
    0x9b05_688c_2b3e_6c1f,
    0x1f83_d9ab_fb41_bd6b,
    0x5be0_cd19_137e_2179,
];

// FIPS 180-4 §6.2.2 and §6.4.2 use the same round and schedule laws
// with different word widths, rotation distances and round constants.
macro_rules! portable {
    ($name:ident, $word:ty, $bytes:literal, $block:literal, $k:ident,
     ($s0a:literal, $s0b:literal, $s0c:literal),
     ($s1a:literal, $s1b:literal, $s1c:literal),
     ($b0a:literal, $b0b:literal, $b0c:literal),
     ($b1a:literal, $b1b:literal, $b1c:literal)) => {
        fn $name(state: &mut [$word; 8], blocks: &[u8]) {
            for block in blocks.as_chunks::<$block>().0 {
                let mut schedule = [0 as $word; 16];
                for (word, bytes) in schedule.iter_mut().zip(block.as_chunks::<$bytes>().0) {
                    *word = <$word>::from_be_bytes(*bytes);
                }
                let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
                for (t, constant) in $k.into_iter().enumerate() {
                    let slot = t & 15;
                    if t >= 16 {
                        let x = schedule[(slot + 1) & 15];
                        let y = schedule[(slot + 14) & 15];
                        let low = x.rotate_right($s0a) ^ x.rotate_right($s0b) ^ (x >> $s0c);
                        let high = y.rotate_right($s1a) ^ y.rotate_right($s1b) ^ (y >> $s1c);
                        schedule[slot] = schedule[slot]
                            .wrapping_add(low)
                            .wrapping_add(schedule[(slot + 9) & 15])
                            .wrapping_add(high);
                    }
                    let choose = g ^ (e & (f ^ g));
                    let majority = (a & b) | ((a | b) & c);
                    let low = a.rotate_right($b0a) ^ a.rotate_right($b0b) ^ a.rotate_right($b0c);
                    let high = e.rotate_right($b1a) ^ e.rotate_right($b1b) ^ e.rotate_right($b1c);
                    let first = h
                        .wrapping_add(high)
                        .wrapping_add(choose)
                        .wrapping_add(constant)
                        .wrapping_add(schedule[slot]);
                    let second = low.wrapping_add(majority);
                    [a, b, c, d, e, f, g, h] = [
                        first.wrapping_add(second),
                        a,
                        b,
                        c,
                        d.wrapping_add(first),
                        e,
                        f,
                        g,
                    ];
                }
                for (word, add) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
                    *word = word.wrapping_add(add);
                }
            }
        }
    };
}

portable!(
    compress32,
    u32,
    4,
    64,
    K32,
    (7, 18, 3),
    (17, 19, 10),
    (2, 13, 22),
    (6, 11, 25)
);
portable!(
    compress64,
    u64,
    8,
    128,
    K64,
    (1, 8, 7),
    (19, 61, 6),
    (28, 34, 39),
    (14, 18, 41)
);

fn portable32(state: &mut [u32; 8], whole: &[u8], tail: &[u8]) {
    compress32(state, whole);
    compress32(state, tail);
}

fn portable64(state: &mut [u64; 8], whole: &[u8], tail: &[u8]) {
    compress64(state, whole);
    compress64(state, tail);
}

macro_rules! paths {
    ($(#[$family:meta])* $backend:ident, $function:ident, $portable:ident,
     [$(($(#[$path:meta])* $variant:ident, $name:literal, $x86:ident)),+ $(,)?]
     $(, $(#[$arm_path:meta])* $arm_variant:ident, $arm_name:literal, $arm:ident)?) => {
        $(#[$family])*
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        pub enum $backend {
            $($(#[$path])* $variant,)+
            $($(#[$arm_path])* $arm_variant,)?
            /// Portable scalar compression.
            Portable,
        }

        impl Backend for $backend {
            const ALL: &'static [Self] = &[$(Self::$variant,)+ $(Self::$arm_variant,)? Self::Portable];

            fn is_available(self) -> bool {
                self.blocks().is_some()
            }

            fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $name,)+
                    $(Self::$arm_variant => $arm_name,)?
                    Self::Portable => "portable",
                }
            }
        }

        impl $backend {
            fn blocks(self) -> Option<$function> {
                match self {
                    Self::Portable => Some($portable),
                    $(Self::$variant => {
                        #[cfg(target_arch = "x86_64")]
                        { crate::arch::sha2_x86::$x86() }
                        #[cfg(not(target_arch = "x86_64"))]
                        { None }
                    })+
                    $(
                        Self::$arm_variant => {
                            #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
                            { crate::arch::sha2_aarch64::$arm() }
                            #[cfg(not(all(target_arch = "aarch64", target_endian = "little")))]
                            { None }
                        }
                    )?
                }
            }

            fn selected_blocks() -> $function {
                static SELECTED: std::sync::OnceLock<$function> = std::sync::OnceLock::new();
                *SELECTED.get_or_init(|| {
                    Self::ALL.iter().find_map(|path| path.blocks()).unwrap_or($portable)
                })
            }
        }
    };
}

paths!(
    /// Execution path for SHA-224 and SHA-256.
    Sha256Backend,
    Sha256Blocks,
    portable32,
    [(
        /// Intel SHA instructions with AVX2 input loads.
        X86ShaAvx2,
        "x86-sha-avx2",
        sha256_blocks
    )],
    /// Armv8 SHA2 instructions.
    Aarch64Sha2,
    "aarch64-sha2",
    sha256_blocks
);

paths!(
    /// Execution path for SHA-384 and SHA-512.
    Sha512Backend,
    Sha512Blocks,
    portable64,
    [
        (
            /// Vector SHA-512 round pairs and message schedule.
            X86Sha512,
            "x86-sha512",
            sha512_instruction_blocks
        ),
        (
            /// AVX-512VL paired message schedule with scalar rounds.
            X86Avx512Vl,
            "x86-avx512vl",
            sha512_blocks
        ),
    ],
    /// Paired Arm SHA-512 instructions.
    Aarch64Sha512,
    "aarch64-sha512",
    sha512_blocks
);

macro_rules! hasher {
    ($name:ident, $label:literal, $word:ty, $word_len:literal, $count:ty,
     $block:literal, $out:literal, $iv:ident, $backend:ident, $compress:ident) => {
        #[doc = concat!("A streaming ", $label, " hasher.")]
        #[derive(Clone)]
        pub struct $name {
            state: [$word; 8],
            buffer: BlockBuffer<$block>,
            bytes: $count,
            blocks: $compress,
        }

        impl $name {
            /// A hasher with nothing absorbed.
            #[must_use]
            pub fn new() -> Self {
                Self::on($backend::selected_blocks())
            }

            /// Pin the hasher to an available execution path.
            #[must_use]
            pub fn with_backend(backend: $backend) -> Option<Self> {
                backend.blocks().map(Self::on)
            }

            const fn on(blocks: $compress) -> Self {
                Self {
                    state: $iv,
                    buffer: BlockBuffer::new(),
                    bytes: 0,
                    blocks,
                }
            }

            /// Compute the digest of one byte slice.
            #[must_use]
            pub fn digest(data: &[u8]) -> [u8; $out] {
                let whole = data.len() / $block * $block;
                let rest = &data[whole..];
                let length = (data.len() as $count).wrapping_mul(8).to_be_bytes();
                let mut state = $iv;
                let compress = $backend::selected_blocks();
                if rest.len() < $block - length.len() {
                    let mut tail = SecretArray::<u8, $block>::new([0; $block]);
                    if rest.is_empty() {
                        pad_md(&mut tail[..], 0, &length);
                    } else {
                        tail[..rest.len()].copy_from_slice(rest);
                        pad_md(&mut tail[..], rest.len(), &length);
                    }
                    compress(&mut state, &data[..whole], &tail[..]);
                } else {
                    let mut tail = SecretArray::<u8, { 2 * $block }>::new([0; 2 * $block]);
                    tail[..rest.len()].copy_from_slice(rest);
                    pad_md(&mut tail[..$block], rest.len(), &length);
                    pad_md_length_block(&mut tail[$block..], &length);
                    compress(&mut state, &data[..whole], &tail[..]);
                }
                Self::output(state)
            }

            /// Absorb the next bytes of the message.
            pub fn update(&mut self, data: &[u8]) {
                self.bytes = self.bytes.wrapping_add(data.len() as $count);
                let state = &mut self.state;
                let compress = self.blocks;
                self.buffer
                    .absorb($block, data, |blocks| compress(state, blocks, &[]));
            }

            /// Return the digest of the whole message.
            #[must_use]
            pub fn finalize(mut self) -> [u8; $out] {
                self.finish()
            }

            fn finish(&mut self) -> [u8; $out] {
                let state = &mut self.state;
                let compress = self.blocks;
                // FIPS 180-4 §5.1 encodes the bit length in big-endian order.
                let length = self.bytes.wrapping_mul(8).to_be_bytes();
                self.buffer
                    .finish_md(length, |blocks| compress(state, blocks, &[]));
                Self::output(*state)
            }

            fn output(state: [$word; 8]) -> [u8; $out] {
                // SHA-224 and SHA-384 take only the leading whole words.
                let mut out = [0; $out];
                for (bytes, word) in out.as_chunks_mut::<$word_len>().0.iter_mut().zip(state) {
                    *bytes = word.to_be_bytes();
                }
                out
            }
        }

        crate::default_from_new!($name);

        impl core::fmt::Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_struct(stringify!($name)).finish_non_exhaustive()
            }
        }

        impl Digest for $name {
            fn output_len(&self) -> usize {
                $out
            }

            fn update(&mut self, data: &[u8]) {
                Self::update(self, data);
            }

            fn finalize_reset(&mut self, out: &mut [u8]) -> usize {
                out[..$out].copy_from_slice(&self.finish());
                self.reset();
                $out
            }

            fn reset(&mut self) {
                self.state = $iv;
                self.buffer.clear();
                self.bytes = 0;
            }
        }
    };
}

hasher!(
    Sha224,
    "SHA-224",
    u32,
    4,
    u64,
    64,
    28,
    IV224,
    Sha256Backend,
    Sha256Blocks
);
hasher!(
    Sha256,
    "SHA-256",
    u32,
    4,
    u64,
    64,
    32,
    IV256,
    Sha256Backend,
    Sha256Blocks
);
hasher!(
    Sha384,
    "SHA-384",
    u64,
    8,
    u128,
    128,
    48,
    IV384,
    Sha512Backend,
    Sha512Blocks
);
hasher!(
    Sha512,
    "SHA-512",
    u64,
    8,
    u128,
    128,
    64,
    IV512,
    Sha512Backend,
    Sha512Blocks
);
