// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The **build shape**: the compile-time facts that decide what the
//! [`Reassociated`](super::Reassociated) body compiles to.
//!
//! The layout is documented on [`BuildShape`], and what the build identity records on
//! [`BuildIdentity`].

use core::fmt;

use super::build_identity::{self, BuildInputs};

/// One architecture's row of the layout table.
struct Row {
    /// The `target_arch` value.
    arch: &'static str,
    /// The architecture code.
    code: u8,
    /// The recorded features, in bit order.
    features: &'static [&'static str],
}

/// The code an architecture the layout does not name records.
const UNNAMED: u8 = 0xFF;

/// The first feature bit.
const FEATURE_SHIFT: u32 = 16;

/// The number of feature bits a shape holds.
const FEATURE_BITS: usize = 48;

/// The `x86` and `x86_64` features.
macro_rules! x86_features {
    ($then:ident) => {
        $then!(
            "x87",
            "sse",
            "sse2",
            "sse3",
            "ssse3",
            "sse4.1",
            "sse4.2",
            "avx",
            "avx2",
            "fma",
            "avx512f",
            "avx512cd",
            "avx512dq",
            "avx512bw",
            "avx512vl",
            "soft-float"
        )
    };
}

/// The `aarch64` and `arm64ec` features.
macro_rules! aarch64_features {
    ($then:ident) => {
        $then!(
            "neon", "fp16", "fcma", "rdm", "dotprod", "f64mm", "sve", "sve2"
        )
    };
}

/// The `arm` features.
macro_rules! arm_features {
    ($then:ident) => {
        $then!("vfp2", "vfp3", "vfp4", "d32", "fp-armv8", "neon")
    };
}

/// The wasm features.
macro_rules! wasm_features {
    ($then:ident) => {
        $then!("simd128", "relaxed-simd")
    };
}

/// The RISC-V features.
macro_rules! riscv_features {
    ($then:ident) => {
        $then!("f", "d", "zfh", "v", "zve64d")
    };
}

/// The PowerPC features.
macro_rules! powerpc_features {
    ($then:ident) => {
        $then!("altivec", "vsx", "power8-vector", "power9-vector")
    };
}

/// The `s390x` features.
macro_rules! s390x_features {
    ($then:ident) => {
        $then!("vector", "vector-enhancements-1", "vector-enhancements-2")
    };
}

/// The LoongArch features.
macro_rules! loongarch_features {
    ($then:ident) => {
        $then!("f", "d", "lsx", "lasx")
    };
}

/// The MIPS features.
macro_rules! mips_features {
    ($then:ident) => {
        $then!("fp64", "msa")
    };
}

/// The Hexagon features.
macro_rules! hexagon_features {
    ($then:ident) => {
        $then!("hvx")
    };
}

/// An architecture with no recorded feature.
macro_rules! no_features {
    ($then:ident) => {
        $then!()
    };
}

/// A feature list as the table's names.
macro_rules! names {
    ($($feature:literal),*) => {
        &[$($feature),*]
    };
}

/// A feature list as this build's `cfg(target_feature)` answers, in the same order.
macro_rules! enabled {
    ($($feature:literal),*) => {
        &[$(cfg!(target_feature = $feature)),*]
    };
}

/// The layout table and this build's row of it, from one list so the two cannot drift.
macro_rules! layout {
    ($($arch:literal => $code:literal, $features:ident;)*) => {
        /// The layout table, in code order.
        const ROWS: &[Row] = &[$(Row {
            arch: $arch,
            code: $code,
            features: $features!(names),
        }),*];

        /// This build's architecture code and its row's `cfg(target_feature)` answers.
        const fn compiled() -> (u8, &'static [bool]) {
            $(
                if cfg!(target_arch = $arch) {
                    return ($code, $features!(enabled));
                }
            )*
            (UNNAMED, &[])
        }
    };
}

layout! {
    "x86_64" => 1, x86_features;
    "x86" => 2, x86_features;
    "aarch64" => 3, aarch64_features;
    "arm64ec" => 4, aarch64_features;
    "arm" => 5, arm_features;
    "wasm32" => 6, wasm_features;
    "wasm64" => 7, wasm_features;
    "riscv64" => 8, riscv_features;
    "riscv32" => 9, riscv_features;
    "powerpc64" => 10, powerpc_features;
    "powerpc" => 11, powerpc_features;
    "s390x" => 12, s390x_features;
    "loongarch64" => 13, loongarch_features;
    "loongarch32" => 14, loongarch_features;
    "mips" => 15, mips_features;
    "mips64" => 16, mips_features;
    "mips32r6" => 17, mips_features;
    "mips64r6" => 18, mips_features;
    "sparc" => 19, no_features;
    "sparc64" => 20, no_features;
    "m68k" => 21, no_features;
    "csky" => 22, no_features;
    "hexagon" => 23, hexagon_features;
    "bpf" => 24, no_features;
    "avr" => 25, no_features;
    "msp430" => 26, no_features;
    "nvptx64" => 27, no_features;
    "amdgpu" => 28, no_features;
    "xtensa" => 29, no_features;
}

// Every row's features fit the bits a shape holds for them.
const _: () = {
    let mut row = 0;
    while row < ROWS.len() {
        assert!(ROWS[row].features.len() <= FEATURE_BITS);
        row += 1;
    }
};

/// The compile-time shape of a build's reassociated arithmetic: its target architecture,
/// the target features that decide what the reassociated body compiles to, and the
/// [`BuildIdentity`] of the compilation -- compiler, target CPU, optimisation level and
/// codegen flags. Two shapes are the same shape exactly when their bits and their identity
/// are equal.
///
/// A dispatch [`Path`](super::Path) names *which* compilation of the reassociated body ran,
/// but not what that compilation became. Each compilation is built with the consumer
/// build's own `-C target-cpu`/`-C target-feature` in addition to whatever
/// `#[target_feature]` the path enables, and by the consumer's compiler at the consumer's
/// optimisation level, so the same path compiles to different instructions in different
/// builds: the baseline `x86_64` path contracts to FMA in a build whose global features
/// include `fma`, the AVX-512F path runs `ymm` rather than `zmm` registers under
/// `x86-64-v4`, the NEON path becomes SVE under a Neoverse target, and one compiler
/// release may vectorize the body differently from the next. Two builds recording the same
/// path can therefore compute different bits. A [`BuildShape`] records what decides them,
/// in two parts:
///
/// * its **bits**: the target architecture and the `cfg(target_feature)` set relevant to
///   that architecture's vector and fused-multiply-add code generation, as the crate
///   holding the body was compiled -- what the compiler exposes to the source, laid out
///   below;
/// * its **identity** ([`BuildShape::identity`]): what the source cannot see -- CPU tuning
///   (`x86-64-v4`'s preference for 256-bit vectors, a Neoverse core's scheduling model),
///   the compiler release and its LLVM, the optimisation level and the codegen flags --
///   read by the crate's build script from what `cargo` hands it, and recorded as a digest.
///
/// Two builds of equal shape compile the body to the same code, so they compute the same
/// bits: a consumer that recomputes a recorded result in a build of the recorded shape, on
/// the recorded path, and gets other bits has been handed other inputs, not another
/// compilation. What neither part can see is named on [`BuildIdentity`].
///
/// # The layout, version 2
///
/// A shape is stored as two `u64`s, each little-endian: its bits, then its identity's
/// digest ([`BuildIdentity::digest`]). The bits are:
///
/// | bits | field |
/// |---:|---|
/// | 0..8 | the layout version, [`BuildShape::LAYOUT`] (2) |
/// | 8..16 | the architecture code, from the table below; `0xFF` for an architecture this layout does not name |
/// | 16..64 | one bit per feature of that architecture's row, bit `16 + i` for the row's `i`-th feature; set when the build compiled with it |
///
/// | code | `target_arch` | features, in bit order |
/// |---:|---|---|
/// | 1 | `x86_64` | `x87`, `sse`, `sse2`, `sse3`, `ssse3`, `sse4.1`, `sse4.2`, `avx`, `avx2`, `fma`, `avx512f`, `avx512cd`, `avx512dq`, `avx512bw`, `avx512vl`, `soft-float` |
/// | 2 | `x86` | as `x86_64` |
/// | 3 | `aarch64` | `neon`, `fp16`, `fcma`, `rdm`, `dotprod`, `f64mm`, `sve`, `sve2` |
/// | 4 | `arm64ec` | as `aarch64` |
/// | 5 | `arm` | `vfp2`, `vfp3`, `vfp4`, `d32`, `fp-armv8`, `neon` |
/// | 6 | `wasm32` | `simd128`, `relaxed-simd` |
/// | 7 | `wasm64` | as `wasm32` |
/// | 8 | `riscv64` | `f`, `d`, `zfh`, `v`, `zve64d` |
/// | 9 | `riscv32` | as `riscv64` |
/// | 10 | `powerpc64` | `altivec`, `vsx`, `power8-vector`, `power9-vector` |
/// | 11 | `powerpc` | as `powerpc64` |
/// | 12 | `s390x` | `vector`, `vector-enhancements-1`, `vector-enhancements-2` |
/// | 13 | `loongarch64` | `f`, `d`, `lsx`, `lasx` |
/// | 14 | `loongarch32` | as `loongarch64` |
/// | 15 | `mips` | `fp64`, `msa` |
/// | 16 | `mips64` | as `mips` |
/// | 17 | `mips32r6` | as `mips` |
/// | 18 | `mips64r6` | as `mips` |
/// | 19 | `sparc` | none |
/// | 20 | `sparc64` | none |
/// | 21 | `m68k` | none |
/// | 22 | `csky` | none |
/// | 23 | `hexagon` | `hvx` |
/// | 24 | `bpf` | none |
/// | 25 | `avr` | none |
/// | 26 | `msp430` | none |
/// | 27 | `nvptx64` | none |
/// | 28 | `amdgpu` | none |
/// | 29 | `xtensa` | none |
///
/// The table is append-only: a row's code and its features' bit positions never move, a
/// new architecture takes the next code, and a new feature of an existing row takes the
/// next bit. Anything else is a new layout version. Layout 1 was the bits alone, with no
/// identity; its table is this one. Two builds of architectures this layout does not name
/// share the code `0xFF`; such a build still records its dispatch path and its identity.
///
/// On wasm, `relaxed-simd` is recorded because a build that enables it globally lets the
/// compiler contract the reassociated body into relaxed fused multiply-adds, whose results
/// the engine chooses. PurRDF never enables it; a build that does records the bit, so its
/// images are never accepted by a build without it, and vice versa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BuildShape {
    /// The layout version, architecture code and feature bits.
    bits: u64,
    /// The compilation's identity.
    identity: BuildIdentity,
}

impl BuildShape {
    /// The layout version this build writes, in the low byte of every shape's bits.
    pub const LAYOUT: u8 = 2;

    /// The shape of this build: the architecture and features the crate holding the
    /// reassociated body was compiled for, and that compilation's [`BuildIdentity`].
    #[must_use]
    pub const fn here() -> Self {
        let (code, enabled) = compiled();
        let mut features = 0_u64;
        let mut bit = 0;
        while bit < enabled.len() {
            if enabled[bit] {
                features |= 1 << bit;
            }
            bit += 1;
        }
        Self::assemble(code, features, BuildIdentity::here())
    }

    /// The shape a build of `arch` compiled with exactly the features `enabled`, under
    /// `identity`, has.
    ///
    /// The pure encoder behind [`BuildShape::here`], for a caller that names a build other
    /// than this one. `arch` is a `target_arch` value; one this layout does not name is
    /// encoded as the unnamed architecture with no features. A name in `enabled` that is
    /// not one of `arch`'s recorded features does not change the shape: the layout records
    /// only the features that decide the reassociated body's code.
    #[must_use]
    pub fn encode(arch: &str, enabled: &[&str], identity: BuildIdentity) -> Self {
        let Some(row) = ROWS.iter().find(|row| row.arch == arch) else {
            return Self::assemble(UNNAMED, 0, identity);
        };
        let features = row
            .features
            .iter()
            .enumerate()
            .filter(|(_, feature)| enabled.contains(feature))
            .fold(0_u64, |bits, (bit, _)| bits | (1 << bit));
        Self::assemble(row.code, features, identity)
    }

    /// The shape whose stored parts are `bits` and `identity`, as an image records them.
    /// Any values are a shape; one of another layout version is simply never equal to
    /// this build's.
    #[must_use]
    pub const fn from_parts(bits: u64, identity: BuildIdentity) -> Self {
        Self { bits, identity }
    }

    /// The shape's bits, as an image stores them.
    #[must_use]
    pub const fn bits(self) -> u64 {
        self.bits
    }

    /// The identity of the compilation the shape records.
    #[must_use]
    pub const fn identity(self) -> BuildIdentity {
        self.identity
    }

    /// The layout version the shape was written under.
    #[must_use]
    pub const fn layout(self) -> u8 {
        self.bits.to_le_bytes()[0]
    }

    /// The `target_arch` the shape names, or `None` for an architecture its layout does
    /// not name, or a layout this build does not read.
    #[must_use]
    pub fn architecture(self) -> Option<&'static str> {
        self.row().map(|row| row.arch)
    }

    /// The recorded features the shape's build compiled with, in bit order; empty when
    /// [`BuildShape::architecture`] is `None`.
    #[must_use]
    pub fn features(self) -> Vec<&'static str> {
        self.row().map_or_else(Vec::new, |row| {
            row.features
                .iter()
                .enumerate()
                .filter(|(bit, _)| self.feature_bits() & (1 << *bit) != 0)
                .map(|(_, feature)| *feature)
                .collect()
        })
    }

    /// The shape of this layout's architecture code and feature bits, under `identity`.
    const fn assemble(code: u8, features: u64, identity: BuildIdentity) -> Self {
        Self {
            bits: Self::LAYOUT as u64 | (code as u64) << 8 | features << FEATURE_SHIFT,
            identity,
        }
    }

    /// The architecture code.
    const fn code(self) -> u8 {
        self.bits.to_le_bytes()[1]
    }

    /// The feature bits, shifted down to bit 0.
    const fn feature_bits(self) -> u64 {
        self.bits >> FEATURE_SHIFT
    }

    /// The table row the shape names, under this layout.
    fn row(self) -> Option<&'static Row> {
        if self.layout() != Self::LAYOUT {
            return None;
        }
        ROWS.iter().find(|row| row.code == self.code())
    }

    /// The bits' half of the display.
    fn fmt_bits(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.layout() != Self::LAYOUT {
            return write!(
                f,
                "build shape {:#018x}, of layout {}, which this build does not read",
                self.bits,
                self.layout()
            );
        }
        let Some(row) = self.row() else {
            return write!(
                f,
                "build shape {:#018x}: an architecture layout {} does not name",
                self.bits,
                Self::LAYOUT
            );
        };
        write!(f, "build shape {:#018x}: {}", self.bits, row.arch)?;
        let features = self.features();
        if features.is_empty() {
            f.write_str(" with none of its recorded features")?;
        } else {
            write!(f, " with {}", features.join(", "))?;
        }
        let unassigned = self.feature_bits() & !((1_u64 << row.features.len()) - 1);
        if unassigned != 0 {
            write!(
                f,
                " and feature bits {unassigned:#x} layout {} does not assign",
                Self::LAYOUT
            )?;
        }
        Ok(())
    }
}

impl fmt::Display for BuildShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_bits(f)?;
        write!(f, ", under {}", self.identity)
    }
}

/// This build's identity text: the build script's, then the debug-assertion state this
/// crate was compiled with, read from its own `cfg`.
#[cfg(debug_assertions)]
const HERE_TEXT: &str = concat!(env!("PURRDF_BUILD_IDENTITY"), "; debug-assertions=on");
/// This build's identity text: the build script's, then the debug-assertion state this
/// crate was compiled with, read from its own `cfg`.
#[cfg(not(debug_assertions))]
const HERE_TEXT: &str = concat!(env!("PURRDF_BUILD_IDENTITY"), "; debug-assertions=off");

/// The 64-bit FNV-1a digest of `bytes`: fixed-key, deterministic on every target, and
/// computable at compile time.
const fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut at = 0;
    while at < bytes.len() {
        hash ^= bytes[at] as u64;
        hash = hash.wrapping_mul(0x0100_0000_01b3);
        at += 1;
    }
    hash
}

/// The identity of the compilation that built the reassociated body, beyond what its
/// `cfg` exposes: the part of a [`BuildShape`] the source cannot see, as a digest.
///
/// The crate's build script reads what `cargo` hands every build script and writes it as
/// one line of text ([`BuildIdentity::describe`]); the identity is that text's 64-bit
/// FNV-1a digest. The text records, in this order:
///
/// * `rustc`, `commit` and `llvm`: the compiler's release, commit hash and LLVM version,
///   from `rustc -vV` run on the compiler `cargo` names in `RUSTC` (never through a
///   `RUSTC_WRAPPER`, which would be the wrapper's version). The host the compiler runs on
///   is not recorded: one compiler release emits the same code for a target wherever it
///   runs;
/// * `target-cpu`: the CPU the code is tuned and selected for. A `-C target-cpu` in the
///   flags is recorded as named, `native` resolved to the CPU `rustc` reports for the
///   building host, and the target's default CPU, from `rustc --print target-cpus`, when
///   none is named;
/// * `target-feature`: every `-C target-feature` in the flags, as `rustc` applies them (the
///   last sign of each name wins), sorted by name. Tuning features such as
///   `prefer-256-bit` are not target-feature `cfg`s, so the shape's bits cannot see them.
///   `crt-static` is a link choice and is left out;
/// * `llvm-args`, `codegen-units`, `lto` and `overflow-checks`, where the flags set them;
/// * `opt-level`: the flags' `-C opt-level` (or `-O`), else the profile's;
/// * `debug-assertions`: whether this crate was compiled with them, from its own `cfg`.
///   They enable the standard library's precondition checks inside the iterators the body
///   inlines.
///
/// The flags are `CARGO_ENCODED_RUSTFLAGS`: `RUSTFLAGS`, `build.rustflags` and every other
/// source `cargo` merges. Any other flag, `-D warnings` among them, does not change code
/// and is not recorded, so builds that differ only in lints share an identity.
///
/// # What no identity sees
///
/// A build script sees the profile's optimisation level and debug assertions, and every
/// flag in the rustflags, but `cargo` passes the rest of a `[profile]` table straight to
/// `rustc`: a profile's `lto`, `codegen-units` and `overflow-checks` settings, and the
/// flags a `cargo rustc -- …` invocation appends, are invisible to it and so to this
/// identity, as is a compiler whose `rustc -vV` does not tell it apart from another (a
/// locally modified build with an unchanged version). Builds that differ only there record
/// one identity; they select and tune the body's code identically and differ only in which
/// optimisation pipeline runs it, and a result one of them recomputes differently from the
/// other's is answered as a result that differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BuildIdentity(u64);

impl BuildIdentity {
    /// This build's identity.
    #[must_use]
    pub const fn here() -> Self {
        Self(fnv1a64(HERE_TEXT.as_bytes()))
    }

    /// The identity of a build whose build script observed `inputs`, compiled with debug
    /// assertions exactly when `debug_assertions`.
    ///
    /// The pure encoder behind [`BuildIdentity::here`]: the digest of
    /// [`BuildIdentity::describe`].
    #[must_use]
    pub fn encode(inputs: &BuildInputs<'_>, debug_assertions: bool) -> Self {
        Self(fnv1a64(Self::describe(inputs, debug_assertions).as_bytes()))
    }

    /// The identity text of a build whose build script observed `inputs`, compiled with
    /// debug assertions exactly when `debug_assertions`: the text whose digest
    /// [`BuildIdentity::encode`] is.
    #[must_use]
    pub fn describe(inputs: &BuildInputs<'_>, debug_assertions: bool) -> String {
        let assertions = if debug_assertions { "on" } else { "off" };
        format!(
            "{}; debug-assertions={assertions}",
            build_identity::compiler_text(inputs)
        )
    }

    /// The identity whose digest is `digest`, as an image records it.
    #[must_use]
    pub const fn from_digest(digest: u64) -> Self {
        Self(digest)
    }

    /// The identity's digest, as an image stores it.
    #[must_use]
    pub const fn digest(self) -> u64 {
        self.0
    }

    /// The identity's text, known for this build's identity only: an image records the
    /// digest, and a digest cannot be read back into the build it names.
    #[must_use]
    pub fn text(self) -> Option<&'static str> {
        (self == Self::here()).then_some(HERE_TEXT)
    }
}

impl fmt::Display for BuildIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "build identity {:#018x}", self.0)?;
        match self.text() {
            Some(text) => write!(f, " ({text})"),
            None => f.write_str(" (not this build's)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One identity, for shapes whose bits are the subject.
    fn id() -> BuildIdentity {
        BuildIdentity::from_digest(0x5eed)
    }

    #[test]
    fn the_table_fits_the_layout() {
        let mut codes: Vec<u8> = ROWS.iter().map(|row| row.code).collect();
        let count = codes.len();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), count, "no two architectures share a code");
        assert!(!codes.contains(&UNNAMED) && !codes.contains(&0));
        // Append-only: the pinned rows keep their codes and bit order.
        assert_eq!(BuildShape::encode("x86_64", &[], id()).bits(), 0x0102);
        assert_eq!(
            BuildShape::encode("aarch64", &["neon"], id()).bits(),
            0x1_0302
        );
        assert_eq!(
            BuildShape::encode("x86_64", &["sse2", "fma"], id()).bits(),
            0x0102 | (1 << (16 + 2)) | (1 << (16 + 9))
        );
    }

    #[test]
    fn the_encoder_is_deterministic_and_order_free() {
        let one = BuildShape::encode("x86_64", &["sse", "sse2", "avx2", "fma"], id());
        let two = BuildShape::encode("x86_64", &["fma", "avx2", "sse2", "sse"], id());
        assert_eq!(one, two);
        assert_eq!(
            one,
            BuildShape::encode("x86_64", &["sse", "sse2", "avx2", "fma"], id())
        );
        assert_eq!(BuildShape::from_parts(one.bits(), one.identity()), one);
        assert_eq!(one.architecture(), Some("x86_64"));
        assert_eq!(one.features(), ["sse", "sse2", "avx2", "fma"]);
    }

    #[test]
    fn a_relevant_feature_changes_the_shape_and_an_irrelevant_one_does_not() {
        let baseline = ["x87", "sse", "sse2"];
        let base = BuildShape::encode("x86_64", &baseline, id());
        // Each relevant feature, added alone, is a different shape.
        for feature in [
            "sse3", "ssse3", "sse4.1", "sse4.2", "avx", "avx2", "fma", "avx512f", "avx512vl",
            "avx512dq", "avx512bw", "avx512cd",
        ] {
            let mut enabled = baseline.to_vec();
            enabled.push(feature);
            let shape = BuildShape::encode("x86_64", &enabled, id());
            assert_ne!(shape, base, "{feature} must change the shape");
            assert!(shape.features().contains(&feature));
        }
        // The neighbour: a feature that does not decide the body's code is not recorded.
        let mut enabled = baseline.to_vec();
        enabled.extend(["popcnt", "bmi2", "neon"]);
        assert_eq!(BuildShape::encode("x86_64", &enabled, id()), base);
        // The same features on another architecture are another shape.
        assert_ne!(BuildShape::encode("x86", &baseline, id()), base);
        let neon = BuildShape::encode("aarch64", &["neon"], id());
        assert_ne!(neon, BuildShape::encode("aarch64", &["neon", "sve"], id()));
        assert_ne!(
            neon,
            BuildShape::encode("aarch64", &["neon", "sve", "sve2"], id())
        );
        assert_ne!(
            BuildShape::encode("wasm32", &["simd128"], id()),
            BuildShape::encode("wasm32", &["simd128", "relaxed-simd"], id())
        );
        assert_ne!(
            BuildShape::encode("wasm32", &["simd128"], id()),
            BuildShape::encode("wasm32", &[], id())
        );
    }

    #[test]
    fn this_build_encodes_as_the_encoder_would() {
        let here = BuildShape::here();
        assert_eq!(here.layout(), BuildShape::LAYOUT);
        let arch = here.architecture().expect("every CI target is a named one");
        assert_eq!(
            BuildShape::encode(arch, &here.features(), BuildIdentity::here()),
            here
        );
        assert_eq!(here.identity(), BuildIdentity::here());
        assert_eq!(cfg!(target_arch = "x86_64"), arch == "x86_64");
        #[cfg(target_arch = "x86_64")]
        {
            assert!(here.features().contains(&"sse2"), "the x86_64 baseline");
            assert_eq!(
                here.features().contains(&"fma"),
                cfg!(target_feature = "fma")
            );
            assert_eq!(
                here.features().contains(&"avx512f"),
                cfg!(target_feature = "avx512f")
            );
        }
    }

    #[test]
    fn the_display_names_the_architecture_and_features() {
        let shape = BuildShape::encode("x86_64", &["sse", "sse2", "fma"], id());
        assert_eq!(
            shape.to_string(),
            format!(
                "build shape {:#018x}: x86_64 with sse, sse2, fma, under build identity \
                 0x0000000000005eed (not this build's)",
                shape.bits()
            )
        );
        let bare = BuildShape::encode("sparc64", &[], id());
        assert!(
            bare.to_string()
                .contains("sparc64 with none of its recorded features, under")
        );
        let unnamed = BuildShape::encode("an-architecture-of-tomorrow", &["x"], id());
        assert_eq!(unnamed.architecture(), None);
        assert_eq!(unnamed.features(), Vec::<&str>::new());
        assert!(
            unnamed
                .to_string()
                .contains("an architecture layout 2 does not name")
        );
        let later = BuildShape::from_parts(0x0103, id());
        assert_eq!(later.architecture(), None);
        assert!(
            later
                .to_string()
                .contains("of layout 3, which this build does not read")
        );
        // A layout-1 shape, from before the identity was recorded, is not read either.
        let earlier = BuildShape::from_parts(shape.bits() & !0xFF | 1, id());
        assert_eq!(earlier.architecture(), None);
        assert_ne!(earlier, shape);
        let stray = BuildShape::from_parts(shape.bits() | 1 << 63, id());
        assert!(stray.to_string().contains("layout 2 does not assign"));
        assert_ne!(stray, shape);
        // This build's shape names its identity's text.
        let here = BuildShape::here();
        let text = here.identity().text().expect("this build's text is known");
        assert!(here.to_string().ends_with(&format!("({text})")), "{here}");
    }

    /// The `rustc -vV` of one compiler.
    const RUSTC: &str = "rustc 1.100.0-nightly (4b6d04e70 2026-09-13)\nbinary: rustc\n\
        commit-hash: 4b6d04e706108ccfeafe2547fbe857dfe8972bad\ncommit-date: 2026-09-13\n\
        host: x86_64-unknown-linux-gnu\nrelease: 1.100.0-nightly\nLLVM version: 23.1.1\n";

    /// Its `--print target-cpus` for `x86_64-unknown-linux-gnu`, abridged.
    const CPUS: &str = "Available CPUs for this target:\n    \
        native                  - Select the CPU of the current host (currently znver5).\n    \
        alderlake\n    x86-64-v3\n    \
        x86-64                  - This is the default target CPU for the current build \
        target (currently x86_64-unknown-linux-gnu).\n";

    /// A build with no flags beyond `-D warnings`, at opt-level 3.
    fn inputs<'a>(rustflags: &'a [&'a str]) -> BuildInputs<'a> {
        BuildInputs {
            rustc_version: RUSTC,
            target_cpus: Some(CPUS),
            target: "x86_64-unknown-linux-gnu",
            rustflags,
            opt_level: "3",
        }
    }

    #[test]
    fn the_identity_text_names_the_compiler_cpu_and_level() {
        assert_eq!(
            BuildIdentity::describe(&inputs(&["-D", "warnings"]), true),
            "rustc=1.100.0-nightly; commit=4b6d04e706108ccfeafe2547fbe857dfe8972bad; \
             llvm=23.1.1; target-cpu=x86-64; opt-level=3; debug-assertions=on"
        );
        assert_eq!(
            BuildIdentity::describe(
                &inputs(&[
                    "-C",
                    "target-cpu=native",
                    "-Ctarget-feature=+fma,+prefer-256-bit,-fma,+crt-static",
                    "--codegen",
                    "target-feature=+avx2",
                    "--codegen=llvm-args=-x86-use-fsrm-for-memcpy",
                    "-Copt-level=2",
                    "-C",
                    "codegen-units=1",
                    "-Clto=fat",
                    "-Coverflow-checks=off",
                ]),
                false
            ),
            "rustc=1.100.0-nightly; commit=4b6d04e706108ccfeafe2547fbe857dfe8972bad; \
             llvm=23.1.1; target-cpu=znver5; target-feature=+avx2,-fma,+prefer-256-bit; \
             llvm-args=-x86-use-fsrm-for-memcpy; codegen-units=1; lto=fat; \
             overflow-checks=off; opt-level=2; debug-assertions=off"
        );
        // A listing that does not resolve the CPU leaves it unresolved, never guessed.
        let mut bare = inputs(&["-C", "target-cpu=native"]);
        bare.target_cpus = None;
        assert!(BuildIdentity::describe(&bare, true).contains("target-cpu=native(unresolved)"));
        bare.rustflags = &[];
        assert!(
            BuildIdentity::describe(&bare, true)
                .contains("target-cpu=default(x86_64-unknown-linux-gnu)")
        );
    }

    #[test]
    fn the_identity_is_deterministic_and_moves_with_every_input() {
        let base = BuildIdentity::encode(&inputs(&[]), true);
        assert_eq!(base, BuildIdentity::encode(&inputs(&[]), true));
        // The neighbours: flags that change no code are the same identity, and so is a
        // CPU named as the default it already was, and one feature set spelled twice.
        assert_eq!(
            base,
            BuildIdentity::encode(&inputs(&["-D", "warnings"]), true)
        );
        assert_eq!(
            base,
            BuildIdentity::encode(&inputs(&["-C", "target-cpu=x86-64"]), true)
        );
        assert_eq!(
            BuildIdentity::encode(&inputs(&["-Ctarget-feature=+avx2,+fma"]), true),
            BuildIdentity::encode(
                &inputs(&["-Ctarget-feature=+fma", "-Ctarget-feature=+avx2"]),
                true
            )
        );
        assert_eq!(
            base,
            BuildIdentity::encode(&inputs(&["-Ctarget-feature=+crt-static"]), true)
        );
        // Each input that decides the body's code is another identity.
        let other_compiler = RUSTC.replace("23.1.1", "23.1.2");
        let other_commit = RUSTC.replace("4b6d04e706108", "5b6d04e706108");
        let other_release = RUSTC.replace("release: 1.100.0", "release: 1.101.0");
        let mut others = Vec::new();
        for version in [&other_compiler, &other_commit, &other_release] {
            let mut changed = inputs(&[]);
            changed.rustc_version = version;
            others.push(BuildIdentity::encode(&changed, true));
        }
        for flags in [
            &["-C", "target-cpu=x86-64-v3"][..],
            &["-C", "target-cpu=native"],
            &["-Ctarget-feature=+prefer-256-bit"],
            &["-Cllvm-args=-fp-contract=fast"],
            &["-Copt-level=1"],
            &["-O"],
            &["-Ccodegen-units=1"],
            &["-Clto=fat"],
            &["-Coverflow-checks=off"],
        ] {
            others.push(BuildIdentity::encode(&inputs(flags), true));
        }
        let mut level = inputs(&[]);
        level.opt_level = "2";
        others.push(BuildIdentity::encode(&level, true));
        others.push(BuildIdentity::encode(&inputs(&[]), false));
        let count = others.len();
        others.push(base);
        others.sort_unstable_by_key(|identity| identity.digest());
        others.dedup();
        assert_eq!(others.len(), count + 1, "every input is its own identity");

        // The digest is FNV-1a over the text, pinned so it cannot drift.
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(
            base.digest(),
            fnv1a64(BuildIdentity::describe(&inputs(&[]), true).as_bytes())
        );
    }

    #[test]
    fn this_build_records_its_own_identity() {
        let here = BuildIdentity::here();
        let text = here.text().expect("this build's text is known");
        assert_eq!(here.digest(), fnv1a64(text.as_bytes()));
        assert!(text.starts_with("rustc="), "{text}");
        assert!(text.contains("; target-cpu="), "{text}");
        assert!(
            !text.contains("(unresolved)") && !text.contains("default("),
            "{text}"
        );
        assert!(
            text.ends_with(if cfg!(debug_assertions) {
                "; debug-assertions=on"
            } else {
                "; debug-assertions=off"
            }),
            "{text}"
        );
        assert_eq!(BuildIdentity::from_digest(here.digest()), here);
        assert_eq!(BuildIdentity::from_digest(here.digest() ^ 1).text(), None);
        assert!(
            BuildIdentity::from_digest(here.digest() ^ 1)
                .to_string()
                .ends_with("(not this build's)")
        );
        // Two shapes of equal bits and another identity are two shapes.
        let shape = BuildShape::here();
        let other =
            BuildShape::from_parts(shape.bits(), BuildIdentity::from_digest(!here.digest()));
        assert_ne!(other, shape);
        assert_eq!(other.features(), shape.features());
    }
}
