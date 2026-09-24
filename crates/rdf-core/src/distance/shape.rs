// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The **build shape**: the compile-time facts that decide what the
//! [`Reassociated`](super::Reassociated) body compiles to.
//!
//! The layout and what a shape does not capture are documented on [`BuildShape`].

use core::fmt;

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

/// The number of feature bits a layout-1 shape holds.
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
        /// The layout-1 table, in code order.
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

// Every row's features fit the bits a layout-1 shape holds for them.
const _: () = {
    let mut row = 0;
    while row < ROWS.len() {
        assert!(ROWS[row].features.len() <= FEATURE_BITS);
        row += 1;
    }
};

/// The compile-time shape of a build's reassociated arithmetic: its target architecture
/// and the target features that decide what the reassociated body compiles to. Two shapes
/// are the same shape exactly when their bits are equal.
///
/// A dispatch [`Path`](super::Path) names *which* compilation of the reassociated body ran,
/// but not what that compilation became. Each compilation is built with the consumer
/// build's own `-C target-cpu`/`-C target-feature` in addition to whatever
/// `#[target_feature]` the path enables, so the same path compiles to different
/// instructions in different builds: the baseline `x86_64` path contracts to FMA in a build
/// whose global features include `fma`, the AVX-512F path runs `ymm` rather than `zmm`
/// registers under `x86-64-v4`, and the NEON path becomes SVE under a Neoverse target. Two
/// builds recording the same path can therefore compute different bits. A [`BuildShape`] is
/// the part of that difference the compiler exposes to the source: the target architecture
/// and the `cfg(target_feature)` set relevant to that architecture's vector and
/// fused-multiply-add code generation, as the crate holding the body was compiled.
///
/// # What it does not capture
///
/// `rustc` exposes no `cfg` for `-C target-cpu`, and the CPU's *tuning* is not a target
/// feature: `x86-64-v4`'s preference for 256-bit vectors and a Neoverse core's scheduling
/// model change the emitted code without changing a single `cfg(target_feature)`. Neither
/// is the compiler version, whose optimizer may vectorize one source differently from the
/// next. So two builds of equal shape are not guaranteed to compute the same bits; only
/// the same compiled artifact is. Equal shape is a necessary condition, checked; the rest
/// is refused by name where it is observed (a rebuild that diverges under an equal shape),
/// never answered as a tamper-looking `false`.
///
/// # The layout, version 1
///
/// A shape is one `u64`, little-endian wherever it is stored:
///
/// | bits | field |
/// |---:|---|
/// | 0..8 | the layout version, [`BuildShape::LAYOUT`] (1) |
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
/// next bit. Anything else is a new layout version. Two builds of architectures this layout
/// does not name share the code `0xFF`; such a build still records its dispatch path, and
/// its residual divergence is still refused by name.
///
/// On wasm, `relaxed-simd` is recorded because a build that enables it globally lets the
/// compiler contract the reassociated body into relaxed fused multiply-adds, whose results
/// the engine chooses. PurRDF never enables it; a build that does records the bit, so its
/// images are never accepted by a build without it, and vice versa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BuildShape(u64);

impl BuildShape {
    /// The layout version this build writes, in the low byte of every shape.
    pub const LAYOUT: u8 = 1;

    /// The shape of this build: the architecture and features the crate holding the
    /// reassociated body was compiled for.
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
        Self::assemble(code, features)
    }

    /// The shape a build of `arch` compiled with exactly the features `enabled` has.
    ///
    /// The pure encoder behind [`BuildShape::here`], for a caller that names a build other
    /// than this one. `arch` is a `target_arch` value; one this layout does not name is
    /// encoded as the unnamed architecture with no features. A name in `enabled` that is
    /// not one of `arch`'s recorded features does not change the shape: the layout records
    /// only the features that decide the reassociated body's code.
    #[must_use]
    pub fn encode(arch: &str, enabled: &[&str]) -> Self {
        let Some(row) = ROWS.iter().find(|row| row.arch == arch) else {
            return Self::assemble(UNNAMED, 0);
        };
        let features = row
            .features
            .iter()
            .enumerate()
            .filter(|(_, feature)| enabled.contains(feature))
            .fold(0_u64, |bits, (bit, _)| bits | (1 << bit));
        Self::assemble(row.code, features)
    }

    /// The shape whose stored bits are `bits`, as an image records it. Any value is a
    /// shape; one of another layout version is simply never equal to this build's.
    #[must_use]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// The shape's bits, as an image stores them.
    #[must_use]
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// The layout version the shape was written under.
    #[must_use]
    pub const fn layout(self) -> u8 {
        self.0.to_le_bytes()[0]
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

    /// The shape's layout-1 architecture code and feature bits.
    const fn assemble(code: u8, features: u64) -> Self {
        Self(Self::LAYOUT as u64 | (code as u64) << 8 | features << FEATURE_SHIFT)
    }

    /// The architecture code.
    const fn code(self) -> u8 {
        self.0.to_le_bytes()[1]
    }

    /// The feature bits, shifted down to bit 0.
    const fn feature_bits(self) -> u64 {
        self.0 >> FEATURE_SHIFT
    }

    /// The table row a layout-1 shape names.
    fn row(self) -> Option<&'static Row> {
        if self.layout() != Self::LAYOUT {
            return None;
        }
        ROWS.iter().find(|row| row.code == self.code())
    }
}

impl fmt::Display for BuildShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.layout() != Self::LAYOUT {
            return write!(
                f,
                "build shape {:#018x}, of layout {}, which this build does not read",
                self.0,
                self.layout()
            );
        }
        let Some(row) = self.row() else {
            return write!(
                f,
                "build shape {:#018x}: an architecture layout {} does not name",
                self.0,
                Self::LAYOUT
            );
        };
        write!(f, "build shape {:#018x}: {}", self.0, row.arch)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_fits_the_layout() {
        let mut codes: Vec<u8> = ROWS.iter().map(|row| row.code).collect();
        let count = codes.len();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), count, "no two architectures share a code");
        assert!(!codes.contains(&UNNAMED) && !codes.contains(&0));
        // Append-only: the pinned rows keep their codes and bit order.
        assert_eq!(BuildShape::encode("x86_64", &[]).bits(), 0x0101);
        assert_eq!(BuildShape::encode("aarch64", &["neon"]).bits(), 0x1_0301);
        assert_eq!(
            BuildShape::encode("x86_64", &["sse2", "fma"]).bits(),
            0x0101 | (1 << (16 + 2)) | (1 << (16 + 9))
        );
    }

    #[test]
    fn the_encoder_is_deterministic_and_order_free() {
        let one = BuildShape::encode("x86_64", &["sse", "sse2", "avx2", "fma"]);
        let two = BuildShape::encode("x86_64", &["fma", "avx2", "sse2", "sse"]);
        assert_eq!(one, two);
        assert_eq!(
            one,
            BuildShape::encode("x86_64", &["sse", "sse2", "avx2", "fma"])
        );
        assert_eq!(BuildShape::from_bits(one.bits()), one);
        assert_eq!(one.architecture(), Some("x86_64"));
        assert_eq!(one.features(), ["sse", "sse2", "avx2", "fma"]);
    }

    #[test]
    fn a_relevant_feature_changes_the_shape_and_an_irrelevant_one_does_not() {
        let baseline = ["x87", "sse", "sse2"];
        let base = BuildShape::encode("x86_64", &baseline);
        // Each relevant feature, added alone, is a different shape.
        for feature in [
            "sse3", "ssse3", "sse4.1", "sse4.2", "avx", "avx2", "fma", "avx512f", "avx512vl",
            "avx512dq", "avx512bw", "avx512cd",
        ] {
            let mut enabled = baseline.to_vec();
            enabled.push(feature);
            let shape = BuildShape::encode("x86_64", &enabled);
            assert_ne!(shape, base, "{feature} must change the shape");
            assert!(shape.features().contains(&feature));
        }
        // The neighbour: a feature that does not decide the body's code is not recorded.
        let mut enabled = baseline.to_vec();
        enabled.extend(["popcnt", "bmi2", "neon"]);
        assert_eq!(BuildShape::encode("x86_64", &enabled), base);
        // The same features on another architecture are another shape.
        assert_ne!(BuildShape::encode("x86", &baseline), base);
        let neon = BuildShape::encode("aarch64", &["neon"]);
        assert_ne!(neon, BuildShape::encode("aarch64", &["neon", "sve"]));
        assert_ne!(
            neon,
            BuildShape::encode("aarch64", &["neon", "sve", "sve2"])
        );
        assert_ne!(
            BuildShape::encode("wasm32", &["simd128"]),
            BuildShape::encode("wasm32", &["simd128", "relaxed-simd"])
        );
        assert_ne!(
            BuildShape::encode("wasm32", &["simd128"]),
            BuildShape::encode("wasm32", &[])
        );
    }

    #[test]
    fn this_build_encodes_as_the_encoder_would() {
        let here = BuildShape::here();
        assert_eq!(here.layout(), BuildShape::LAYOUT);
        let arch = here.architecture().expect("every CI target is a named one");
        assert_eq!(BuildShape::encode(arch, &here.features()), here);
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
        let shape = BuildShape::encode("x86_64", &["sse", "sse2", "fma"]);
        assert_eq!(
            shape.to_string(),
            format!(
                "build shape {:#018x}: x86_64 with sse, sse2, fma",
                shape.bits()
            )
        );
        let bare = BuildShape::encode("sparc64", &[]);
        assert!(
            bare.to_string()
                .ends_with("sparc64 with none of its recorded features")
        );
        let unnamed = BuildShape::encode("an-architecture-of-tomorrow", &["x"]);
        assert_eq!(unnamed.architecture(), None);
        assert_eq!(unnamed.features(), Vec::<&str>::new());
        assert!(
            unnamed
                .to_string()
                .contains("an architecture layout 1 does not name")
        );
        let later = BuildShape::from_bits(0x0102);
        assert_eq!(later.architecture(), None);
        assert!(
            later
                .to_string()
                .contains("of layout 2, which this build does not read")
        );
        let stray = BuildShape::from_bits(shape.bits() | 1 << 63);
        assert!(stray.to_string().contains("layout 1 does not assign"));
        assert_ne!(stray, shape);
    }
}
