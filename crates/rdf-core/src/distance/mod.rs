// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **Distance arithmetic as a type**: the binary64 folds every ranked-retrieval surface
//! in the workspace computes its distances with, and the law each one follows.
//!
//! A float kernel has two properties a caller can depend on, and they are different
//! properties. One is the *metric*: what is being measured ([`Measure`], and PURREMB's
//! `DistanceMetric` above it). The other is the *arithmetic*: the exact order of the
//! correctly rounded operations that produce the number. Two builds that agree on the
//! metric and disagree on the arithmetic return different last bits, two near-tied
//! candidates swap, and the same query ranks differently on two targets. So the
//! arithmetic is named here, as a type, rather than left to whatever order a loop
//! happened to be written in.
//!
//! [`Arithmetic`] is a sealed trait. Each implementation is one law with one stable
//! identifier ([`Arithmetic::ID`]) and the codes images record it under
//! ([`Arithmetic::IMAGE_CODES`], one per dispatch path whose bits it distinguishes).
//! Every consumer is generic over it, so two arithmetics are two monomorphized
//! functions rather than one function with a mode, and mixing them is a type error.
//!
//! # [`Exact`]: the fixed-lane law
//!
//! `Exact` is the one law whose bits are the same on every target. It is *defined* as
//! sixteen independent binary64 accumulators over consecutive sixteen-element chunks,
//! a fixed pairwise tree, and a sequential tail:
//!
//! 1. Lane `l` (`0 ≤ l < 16`) sums the terms at indices `16·c + l`, for every whole
//!    chunk `c`, in ascending `c`, starting from `+0.0`.
//! 2. The sixteen lanes are combined pairwise: `s8[l] = lane[l] + lane[l+8]`, then
//!    `s4[l] = s8[l] + s8[l+4]`, then `s2[l] = s4[l] + s4[l+2]`, then `s2[0] + s2[1]`.
//! 3. The remaining `len % 16` terms are added to that sum one at a time, in ascending
//!    index.
//!
//! A term is `x[i] · y[i]` for a dot product and `(x[i] - y[i])²` for squared
//! Euclidean distance. Every product is bound to a named local before it is added, so
//! no multiply is ever fused into an add; Rust never contracts a plain `*` and `+`
//! into a fused multiply-add, so no target flag can change that either. Each operand
//! is widened to binary64 per component ([`Scalar`]), which is exact, so `f32`, `f64`
//! and mixed operands give identical bits.
//!
//! Because the order is the definition rather than an accident of compilation, it
//! needs no permission from the compiler to vectorize: the sixteen lanes are sixteen
//! independent add chains, and LLVM packs them into whatever vector width the target
//! has. On a sequence shorter than one chunk the law is exactly the ascending
//! sequential fold, since the tree of sixteen `+0.0` lanes is `+0.0`.
//!
//! # [`Reassociated`]: the fast law, whose bits depend on the target
//!
//! `Reassociated` gives up bit-identity across targets for the freedom the exact law
//! withholds. Inside each 64-element block the terms are summed with the
//! `algebraic_*` operations, which license the compiler to reassociate the sum and to
//! contract a multiply into an add; the block sums are then combined with plain `+`
//! in ascending block order. So the result may differ in its last bits from `Exact`,
//! and between dispatch paths and builds, and the sign of a zero result is
//! unspecified; [`Arithmetic::evidence`] says so in words every consumer records.
//! Within one build and one dispatch path it is still a function: each path's body is
//! compiled exactly once, out of line, so every caller that scores a pair on that path
//! gets the same bits. And because the blocks combine in order, every bounded
//! checkpoint is a true prefix of the full value, so the bounded form abandons only
//! where the full form would have met the bound, exactly as in `Exact`. A non-finite
//! result is refused as it is in `Exact`.
//!
//! # Dispatch happens once, inside one contract
//!
//! [`Arithmetic::resolve`] is called once per scan, relation or index and returns a
//! [`Resolved`] handle naming the dispatch [`Path`] this process will run. On
//! `x86_64`, `Exact` has two: a portable compilation of the generic body and an
//! AVX2 compilation of the *same* body. They are two compilations of one source order,
//! so they return the same bits, and a test holds every path the host can execute to
//! a scalar reference model. `aarch64` NEON and wasm `simd128` are compile-time
//! features of the one portable path. There is no exact AVX-512 path: at sixteen
//! binary64 lanes AVX2 already holds the fold in four registers.
//!
//! `Reassociated` compiles its own body and never runs a compilation of `Exact`'s: on
//! `x86_64` an AVX-512F, an AVX2+FMA and a baseline SSE2 compilation, chosen in that
//! order by what the processor reports; on `aarch64` NEON, and on wasm the `simd128` or
//! scalar compilation the build was made with, both fixed at compile time; and on every
//! other target its portable compilation, the body built once for the target's baseline
//! features. So it runs on every target, as `Exact` does. Its portable path shares the
//! name [`Path::Portable`] with `Exact`'s, because it is the same kind of compilation,
//! but not its code, and its image code (8) is its own. Relaxed wasm SIMD is never used:
//! its results are left to the engine.
//!
//! # A path is not the whole of a compilation: the build shape
//!
//! Each reassociated compilation is built with the consumer build's own
//! `-C target-cpu`/`-C target-feature` as well as the path's `#[target_feature]`, so one
//! path compiles to different instructions in different builds -- the baseline `x86_64`
//! path contracts to FMA in a build compiled with `fma`, and the NEON path becomes SVE under
//! a Neoverse target. [`Arithmetic::build_shape`] names the part of that the compiler
//! exposes: the target architecture and the target features relevant to the body's vector
//! and FMA code generation ([`BuildShape`]). A consumer that records a reassociated result
//! records the shape beside the path, and refuses a result whose shape is not its own.
//! Equal shape is necessary, not sufficient: CPU tuning and the compiler version are not
//! visible to the source, so a reassociated result is reproducible only by the same
//! compiled artifact. [`Exact`] has no shape; its bits are the same in every build.
//!
//! A result recorded on one path is recomputed on that path, not on the widest:
//! [`Arithmetic::resolve_recorded`] resolves the path an image code names whenever this
//! process can run it -- its compilation is in this build and the processor reports every
//! feature that compilation was built with -- so a reassociated image recorded on
//! `x86_64`'s SSE2 or AVX2+FMA path runs on an AVX-512F processor, whose binary holds all
//! three compilations. Only a path this process cannot run is refused, with a named
//! [`RecordedPathError`].
//!
//! The batch kernels ([`Resolved::distances`], [`Resolved::distances_indexed`]) are the
//! unit of dispatch. A per-pair call ([`Resolved::distance`]) runs the same body.
//!
//! # The float environment is a precondition, checked
//!
//! Every arithmetic here assumes IEEE-754 round-to-nearest, ties-to-even, with
//! subnormals preserved. A thread that has set flush-to-zero or denormals-are-zero, or
//! another rounding direction, would compute different bits from the same code.
//! [`Arithmetic::resolve`] proves the environment by behaviour, on every target: it runs
//! eight binary64 operations whose IEEE-754 results are known constants, each chosen so
//! that flushing a subnormal or rounding by any other rule changes the bits. Where the
//! control register can be read (MXCSR on `x86_64`, FPCR on `aarch64`) it is read first,
//! so the refusal can name it. A departure is refused with a named
//! [`FloatEnvironmentError`] whose [`FloatEnvironmentEvidence`] says what was observed.
//!
//! The check cannot be skipped, because nothing public computes a distance without the
//! handle it produces: every distance entry point in this module is a method of
//! [`Resolved`], and the per-pair and batch kernels built on it elsewhere in the
//! workspace take a [`Resolved`] too. The same holds for the L2 norm a cosine kernel
//! divides by: [`Resolved::<Exact>::norm`] is the only public form of PURREMB's
//! normative norm fold, and a [`Reassociated`] consumer reaches it through
//! [`Resolved::exact`], which carries the environment its handle already proved. A
//! flushing thread is therefore refused before any distance or norm, per-pair or batch,
//! is computed on it, rather than handed different bits by whichever entry point did
//! not ask. The handle is resolved once per scan or call site and is `Copy`, so the
//! check is never repeated per pair.

mod dispatch;
mod env;
mod exact;
mod reassociated;
mod shape;

#[cfg(test)]
mod reassociated_tests;
#[cfg(test)]
mod tests;

use core::fmt;
use core::marker::PhantomData;

pub use env::{FloatEnvironmentError, FloatEnvironmentEvidence};
pub use shape::BuildShape;

/// The number of independent accumulators in the [`Exact`] law.
pub const EXACT_LANES: usize = exact::LANES;

/// A stored scalar that widens to binary64 **exactly**.
///
/// PURREMB stores embedding matrices as either `binary32` or `binary64` (§12), and
/// widening an `f32` to an `f64` is lossless -- every `f32` is representable. So a kernel
/// can accept either width and widen per component inside its fold, and get bit for bit
/// the answer it would have got from a matrix widened up front.
///
/// That distinction is worth the trait. Widening at LOAD doubles the resident size of an
/// `f32` corpus and changes no arithmetic; widening in the FOLD costs nothing and changes
/// no arithmetic either. At a million rows of 4,096 components that is sixteen gigabytes
/// of difference for an identical answer, and on `wasm32` -- whose address space stops at
/// four gigabytes -- it is the difference between a corpus loading and being refused.
///
/// Narrowing is NOT offered and must not be added: `f64` to `f32` loses bits, so a
/// genuine `binary64` artifact has to stay `binary64`.
///
/// Sealed: `f32` and `f64` are PURREMB's two stored widths and the only implementations.
/// The [`Reassociated`] arithmetic compiles one out-of-line body per dispatch path for
/// each pair of these widths, so that every caller on a path gets the same bits, and
/// that needs to know the concrete width behind a generic operand.
pub trait Scalar: Copy + sealed::Stored {
    /// This value as an `f64`, exactly.
    fn widen(self) -> f64;
}

impl sealed::Stored for f32 {
    #[inline]
    fn width(values: &[Self]) -> sealed::Width<'_> {
        sealed::Width::F32(values)
    }
}

impl Scalar for f32 {
    #[inline]
    fn widen(self) -> f64 {
        f64::from(self)
    }
}

impl sealed::Stored for f64 {
    #[inline]
    fn width(values: &[Self]) -> sealed::Width<'_> {
        sealed::Width::F64(values)
    }
}

impl Scalar for f64 {
    #[inline]
    fn widen(self) -> f64 {
        self
    }
}

/// The threshold a bounded distance is measured against.
///
/// The two forms exist because the callers' comparisons differ, and picking the wrong one
/// is a silent wrong answer rather than a slow one. A caller ranking by distance first and
/// then by row may only abandon on a **strictly** greater partial sum, because at an equal
/// distance the row still decides the comparison. A caller testing a bare `<` on the
/// distance alone may abandon on equality, since equality already falsifies it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Bound {
    /// Abandon once the partial sum is greater than or equal to this value.
    AtOrAbove(f64),
    /// Abandon only once the partial sum is strictly greater than this value.
    Above(f64),
}

impl Bound {
    /// Whether `sum` has reached this bound.
    #[must_use]
    pub fn is_met_by(self, sum: f64) -> bool {
        match self {
            Self::AtOrAbove(limit) => sum >= limit,
            Self::Above(limit) => sum > limit,
        }
    }
}

/// The outcome of a bounded distance evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Bounded {
    /// The distance, on the near side of the bound, computed in full.
    Below(f64),
    /// The bound was reached; the distance's value was never completed.
    Beyond,
    /// A partial sum left the finite range, exactly as a full distance reports `None`.
    NonFinite,
}

/// Which distance a kernel computes, in PURREMB's sense: smaller ranks first.
///
/// These are the three built-in PURREMB metrics and only those. A caller-defined
/// extension metric has opaque parameters no kernel here can interpret, so it has no
/// `Measure`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Measure {
    /// `1 - dot(x, y) / (L2(x) · L2(y))`, written as one product, one quotient and one
    /// subtraction, each rounded on its own. Undefined for a zero-norm operand.
    Cosine,
    /// `-dot(x, y)`.
    NegativeDot,
    /// `sum((x[i] - y[i])²)`.
    SquaredEuclidean,
}

/// The dispatch path a [`Resolved`] arithmetic runs on this process.
///
/// A path is a compilation of an arithmetic's body, never a different arithmetic: every
/// path of one arithmetic honours that arithmetic's whole contract. For [`Exact`] every
/// path returns the same bits. [`Exact`] runs `Portable` and `Avx2`, and
/// [`Reassociated`] runs `Portable` and the other six. `Portable` names the same kind of
/// compilation for both, each arithmetic's own body compiled for the target's baseline
/// features, and an image code is always read against its arithmetic, so the shared
/// name never makes one arithmetic's results read as the other's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Path {
    /// An arithmetic's body compiled for the target's baseline features.
    ///
    /// For [`Exact`] this is the path on every target without AVX2: on `aarch64` that is
    /// NEON, and on a `wasm32` build with `+simd128` it is wasm SIMD; both are
    /// compile-time features, so this is the only exact path those targets have. For
    /// [`Reassociated`] it is the path of every target other than `x86_64`, `aarch64` and
    /// wasm, which have no named reassociated path, fixed at compile time.
    Portable,
    /// [`Exact`]: the same generic body compiled with
    /// `#[target_feature(enable = "avx2")]`, selected on `x86_64` when the processor
    /// reports AVX2.
    Avx2,
    /// [`Reassociated`] on `x86_64`: the body compiled for the baseline, SSE2.
    Sse2,
    /// [`Reassociated`] on `x86_64`: the body compiled with AVX2 and FMA enabled,
    /// selected when the processor reports both.
    Avx2Fma,
    /// [`Reassociated`] on `x86_64`: the body compiled with AVX-512F enabled, selected
    /// when the processor reports it along with AVX2 and FMA.
    Avx512f,
    /// [`Reassociated`] on `aarch64`: the body compiled for NEON, the baseline.
    Neon,
    /// [`Reassociated`] on wasm, in a build with `simd128` enabled.
    WasmSimd128,
    /// [`Reassociated`] on wasm, in a build without `simd128`.
    WasmScalar,
}

impl Path {
    /// The stable name of this path, as evidence text and test output print it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Portable => "portable",
            Self::Avx2 => "avx2",
            Self::Sse2 => "sse2",
            Self::Avx2Fma => "avx2+fma",
            Self::Avx512f => "avx512f",
            Self::Neon => "neon",
            Self::WasmSimd128 => "wasm-simd128",
            Self::WasmScalar => "wasm-scalar",
        }
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Why this process cannot run a dispatch path an image recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathUnavailable {
    /// This build holds no compilation of the path: it belongs to another target
    /// architecture, or to a wasm build made with the other `simd128` setting.
    NotCompiled,
    /// The path's compilation is in this build, but the processor does not report a
    /// feature it was built with; the value is the first such feature's name, as
    /// `#[target_feature]` spells it.
    MissingFeature(&'static str),
}

impl fmt::Display for PathUnavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotCompiled => f.write_str("this build holds no compilation of it"),
            Self::MissingFeature(feature) => {
                write!(f, "the processor does not report `{feature}`")
            }
        }
    }
}

/// A refusal of [`Arithmetic::resolve_recorded`].
///
/// Exhaustive, so a caller mapping it into its own refusals names every case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordedPathError {
    /// The calling thread's float environment is not the IEEE one; see
    /// [`Arithmetic::resolve`].
    FloatEnvironment(FloatEnvironmentError),
    /// The code is not one of the arithmetic's [`Arithmetic::IMAGE_CODES`].
    UnknownCode {
        /// The identifier of the arithmetic asked to resolve it.
        arithmetic: &'static str,
        /// The code.
        code: u32,
    },
    /// The code names a path of the arithmetic this process cannot run.
    Unavailable {
        /// The path the code names.
        path: Path,
        /// Why this process cannot run it.
        reason: PathUnavailable,
    },
}

impl fmt::Display for RecordedPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FloatEnvironment(error) => error.fmt(f),
            Self::UnknownCode { arithmetic, code } => write!(
                f,
                "{code} is not an image code of the {arithmetic} distance arithmetic"
            ),
            Self::Unavailable { path, reason } => write!(
                f,
                "the {path} dispatch path cannot run in this process: {reason}"
            ),
        }
    }
}

impl std::error::Error for RecordedPathError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::FloatEnvironment(error) => Some(error),
            Self::UnknownCode { .. } | Self::Unavailable { .. } => None,
        }
    }
}

impl From<FloatEnvironmentError> for RecordedPathError {
    fn from(error: FloatEnvironmentError) -> Self {
        Self::FloatEnvironment(error)
    }
}

/// A flat, row-major matrix of stored vectors, with the per-row norms a cosine kernel
/// divides by.
///
/// Row `r` occupies `data[r * dims .. (r + 1) * dims]`. `norms` is either empty (a
/// kernel that does not divide by a norm reads `0.0`) or holds exactly one norm per row.
#[derive(Debug, Clone, Copy)]
pub struct RowsRef<'a, T> {
    data: &'a [T],
    rows: usize,
    dims: usize,
    norms: &'a [f64],
}

impl<'a, T: Scalar> RowsRef<'a, T> {
    /// A view of `rows` rows of `dims` components each over `data`.
    ///
    /// Returns `None` unless `data` holds exactly `rows * dims` values and `norms` is
    /// either empty or holds exactly `rows` values. A matrix whose shape disagrees with
    /// its buffer has no rows a kernel could score honestly.
    #[must_use]
    pub fn new(data: &'a [T], rows: usize, dims: usize, norms: &'a [f64]) -> Option<Self> {
        let expected = rows.checked_mul(dims)?;
        if data.len() != expected || !(norms.is_empty() || norms.len() == rows) {
            return None;
        }
        Some(Self {
            data,
            rows,
            dims,
            norms,
        })
    }

    /// How many rows the view holds.
    #[must_use]
    pub const fn rows(&self) -> usize {
        self.rows
    }

    /// The number of components in every row.
    #[must_use]
    pub const fn dims(&self) -> usize {
        self.dims
    }

    /// Row `row`'s components.
    ///
    /// # Panics
    ///
    /// Panics if `row` is not a row of the view.
    #[must_use]
    pub fn row(&self, row: usize) -> &'a [T] {
        assert!(
            row < self.rows,
            "row {row} is not one of the {} rows",
            self.rows
        );
        let start = row * self.dims;
        &self.data[start..start + self.dims]
    }

    /// Row `row`'s norm, or `0.0` when the view carries none.
    #[must_use]
    pub fn norm(&self, row: usize) -> f64 {
        self.norms.get(row).copied().unwrap_or(0.0)
    }
}

mod sealed {
    /// Only this module's arithmetics implement [`super::Arithmetic`].
    pub trait Sealed {}

    /// Only PURREMB's two stored widths implement [`super::Scalar`].
    pub trait Stored: Sized {
        /// `values` at its concrete width.
        fn width(values: &[Self]) -> Width<'_>;
    }

    /// A stored operand at its concrete width.
    #[derive(Debug, Clone, Copy)]
    pub enum Width<'a> {
        /// `binary32` components.
        F32(&'a [f32]),
        /// `binary64` components.
        F64(&'a [f64]),
    }
}

/// An arithmetic contract: one law for the order of every rounded operation in a
/// distance, with a stable identity.
///
/// Sealed. Every implementation is defined in this module, because an arithmetic is a
/// promise about bits that every consumer records and a third-party law could not be
/// held to it.
///
/// The kernel functions take a [`Resolved`] handle rather than a bare [`Path`]: a path
/// is a claim about the processor, and the only ways to obtain one are
/// [`Arithmetic::resolve`] and [`Arithmetic::resolve_recorded`], which checked it. Callers normally use the methods on
/// [`Resolved`].
pub trait Arithmetic: sealed::Sealed + Copy + fmt::Debug + Send + Sync + 'static {
    /// The stable identifier of this law. A plain identifier, not an IRI: PurRDF mints
    /// no vocabulary.
    const ID: &'static str;

    /// Every code an image may record this arithmetic under, ascending.
    ///
    /// An arithmetic whose bits are the same on every path has one code; one whose bits
    /// depend on the path has one code per path, so an image records the compilation
    /// its distances came from. No two arithmetics share a code, and zero is never a
    /// code, so a field that was reserved and zero in an older format can never be read
    /// as naming an arithmetic.
    const IMAGE_CODES: &'static [u32];

    /// The code an image records for results computed along `path`, or `None` when
    /// `path` is not one of this arithmetic's.
    fn image_code(path: Path) -> Option<u32>;

    /// The divergence this arithmetic's results carry along `path`, or `None` for an
    /// arithmetic whose bits are the same on every path and target.
    fn evidence(path: Path) -> Option<&'static str>;

    /// The compile shape of this build's compilations of the arithmetic, or `None` for an
    /// arithmetic whose bits are the same in every build.
    ///
    /// For an arithmetic whose bits depend on the compilation, a recorded result is
    /// reproducible only by a build of the same shape (and, beyond what the shape can
    /// see, only by the same compiled artifact), so a consumer records it beside the image
    /// code and refuses a mismatch. See [`BuildShape`].
    fn build_shape() -> Option<BuildShape>;

    /// Check the float environment and select this process's dispatch path.
    ///
    /// Called once per scan, relation or index. The environment is a per-thread
    /// property, so a caller that moves work to another thread resolves there too.
    ///
    /// # Errors
    ///
    /// [`FloatEnvironmentError`] when the current thread's floating-point environment
    /// flushes subnormals or rounds other than to nearest, ties to even — shown by its
    /// control register where one is read, and by the behavioural probe on every target.
    /// Every arithmetic has a compilation for every target, so nothing else refuses.
    fn resolve() -> Result<Resolved<Self>, FloatEnvironmentError>;

    /// Check the float environment and select the dispatch path an image recorded as
    /// `code`, when this process can run it.
    ///
    /// [`Arithmetic::resolve`] selects the path a *new* result is computed on, the widest
    /// the processor reports. A result already recorded on a path must be recomputed on
    /// that path, and a processor runs every compilation its binary holds whose features
    /// it reports, not only the widest: so this resolves the recorded path itself, and
    /// refuses only a path this process cannot run. An arithmetic whose bits are the same
    /// on every path has one code and resolves it to the path [`Arithmetic::resolve`]
    /// selects.
    ///
    /// # Errors
    ///
    /// [`RecordedPathError::FloatEnvironment`] as for [`Arithmetic::resolve`], checked
    /// first; [`RecordedPathError::UnknownCode`] for a code that is not one of
    /// [`Arithmetic::IMAGE_CODES`]; and [`RecordedPathError::Unavailable`] for a path
    /// whose compilation is not in this build or whose features the processor does not
    /// report.
    fn resolve_recorded(code: u32) -> Result<Resolved<Self>, RecordedPathError>;

    /// Score every row of `rows` against `query` into `out`, in row order.
    ///
    /// `out[r]` is `None` when row `r`'s distance left the finite range.
    ///
    /// # Panics
    ///
    /// Panics if `query.len()` differs from `rows.dims()` or `out.len()` from
    /// `rows.rows()`.
    fn distances<Q: Scalar, T: Scalar>(
        resolved: Resolved<Self>,
        measure: Measure,
        query: &[Q],
        query_norm: f64,
        rows: RowsRef<'_, T>,
        out: &mut [Option<f64>],
    );

    /// Score the rows named by `ids` against `query` into `out`, in `ids` order.
    ///
    /// # Panics
    ///
    /// Panics if `query.len()` differs from `rows.dims()`, `out.len()` from
    /// `ids.len()`, or an id is not a row of `rows`.
    fn distances_indexed<Q: Scalar, T: Scalar>(
        resolved: Resolved<Self>,
        measure: Measure,
        query: &[Q],
        query_norm: f64,
        rows: RowsRef<'_, T>,
        ids: &[usize],
        out: &mut [Option<f64>],
    );

    /// The distance from `a` to `b`, or `None` when it left the finite range.
    ///
    /// The operands are folded over their common prefix.
    fn distance_on<Q: Scalar, T: Scalar>(
        resolved: Resolved<Self>,
        measure: Measure,
        a: &[Q],
        a_norm: f64,
        b: &[T],
        b_norm: f64,
    ) -> Option<f64>;

    /// [`Arithmetic::distance_on`], permitted to stop once the answer cannot clear
    /// `bound`.
    fn distance_bounded_on<Q: Scalar, T: Scalar>(
        resolved: Resolved<Self>,
        measure: Measure,
        a: &[Q],
        a_norm: f64,
        b: &[T],
        b_norm: f64,
        bound: Bound,
    ) -> Bounded;
}

/// An arithmetic whose float environment was checked and whose dispatch path was
/// selected, by [`Arithmetic::resolve`] or [`Arithmetic::resolve_recorded`].
///
/// `Copy` and cheap: it is the path and nothing else. It cannot be constructed any other
/// way, so a path that names a processor feature is always one the processor reported.
#[derive(Clone, Copy)]
pub struct Resolved<A: Arithmetic> {
    path: Path,
    arithmetic: PhantomData<fn() -> A>,
}

impl<A: Arithmetic> Resolved<A> {
    /// A handle on `path`. Callers must have established that the processor runs it.
    const fn on(path: Path) -> Self {
        Self {
            path,
            arithmetic: PhantomData,
        }
    }

    /// The dispatch path this handle runs.
    #[must_use]
    pub const fn path(self) -> Path {
        self.path
    }

    /// The divergence evidence of this arithmetic along this path; see
    /// [`Arithmetic::evidence`].
    #[must_use]
    pub fn evidence(self) -> Option<&'static str> {
        A::evidence(self.path)
    }

    /// The code an image records for results computed by this handle; see
    /// [`Arithmetic::image_code`].
    #[must_use]
    pub fn image_code(self) -> u32 {
        A::image_code(self.path).unwrap_or_else(|| {
            unreachable!(
                "{} resolved to {}, which is not one of its paths",
                A::ID,
                self.path
            )
        })
    }

    /// The compile shape of this build's compilations of the arithmetic; see
    /// [`Arithmetic::build_shape`].
    #[must_use]
    pub fn build_shape(self) -> Option<BuildShape> {
        A::build_shape()
    }

    /// See [`Arithmetic::distances`].
    pub fn distances<Q: Scalar, T: Scalar>(
        self,
        measure: Measure,
        query: &[Q],
        query_norm: f64,
        rows: RowsRef<'_, T>,
        out: &mut [Option<f64>],
    ) {
        A::distances(self, measure, query, query_norm, rows, out);
    }

    /// See [`Arithmetic::distances_indexed`].
    pub fn distances_indexed<Q: Scalar, T: Scalar>(
        self,
        measure: Measure,
        query: &[Q],
        query_norm: f64,
        rows: RowsRef<'_, T>,
        ids: &[usize],
        out: &mut [Option<f64>],
    ) {
        A::distances_indexed(self, measure, query, query_norm, rows, ids, out);
    }

    /// See [`Arithmetic::distance_on`].
    #[must_use]
    pub fn distance<Q: Scalar, T: Scalar>(
        self,
        measure: Measure,
        a: &[Q],
        a_norm: f64,
        b: &[T],
        b_norm: f64,
    ) -> Option<f64> {
        A::distance_on(self, measure, a, a_norm, b, b_norm)
    }

    /// See [`Arithmetic::distance_bounded_on`].
    #[must_use]
    pub fn distance_bounded<Q: Scalar, T: Scalar>(
        self,
        measure: Measure,
        a: &[Q],
        a_norm: f64,
        b: &[T],
        b_norm: f64,
        bound: Bound,
    ) -> Bounded {
        A::distance_bounded_on(self, measure, a, a_norm, b, b_norm, bound)
    }
}

impl<A: Arithmetic> Resolved<A> {
    /// The [`Exact`] arithmetic's handle, under the float environment this handle already
    /// proved.
    ///
    /// The environment check is the same for every arithmetic, so a thread that resolved
    /// one has proved what the other needs; only the dispatch path is selected afresh, as
    /// [`Exact`]'s resolve would select it. This is how a consumer running under
    /// [`Reassociated`] obtains the norm its cosine kernel divides by: the norm is
    /// PURREMB's normative fold and has no reassociated form, so it is computed by
    /// [`Resolved::<Exact>::norm`] whatever arithmetic ranks the distances.
    #[must_use]
    pub fn exact(self) -> Resolved<Exact> {
        Resolved::on(dispatch::exact_path())
    }
}

impl Resolved<Exact> {
    /// The Euclidean (L2) norm of `vector`, by PURREMB's normative scaled fold.
    ///
    /// PURREMB §13.2 fixes this fold's written order (no fused multiply-add, sequential
    /// over ascending index), and a space stored with deterministic L2 postprocessing was
    /// normalized by it, so it is never reordered and has no second arithmetic: it is a
    /// method of the exact handle only, and a [`Reassociated`] consumer reaches it through
    /// [`Resolved::exact`]. It carries a running maximum magnitude and a sum of squared
    /// *ratios*, which keeps it finite for vectors whose squares would overflow or
    /// underflow.
    ///
    /// Taking the handle is what makes that finiteness a fact rather than a hope: the
    /// ratios of a vector with components near `1e-160` square into the subnormal range,
    /// and a thread that flushes subnormals would fold zeros there and return different
    /// bits, which a cosine kernel would then divide by. Such a thread cannot obtain the
    /// handle ([`Arithmetic::resolve`] refuses it by name), so no norm is computed on it.
    ///
    /// A zero-length vector and a vector of zeros both norm to `0.0`; refusing a zero norm
    /// is the job of a caller whose metric divides by it.
    #[must_use]
    pub fn norm<T: Scalar>(self, vector: &[T]) -> f64 {
        l2_norm(vector)
    }
}

impl<A: Arithmetic> PartialEq for Resolved<A> {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl<A: Arithmetic> Eq for Resolved<A> {}

impl<A: Arithmetic> fmt::Debug for Resolved<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Resolved")
            .field("arithmetic", &A::ID)
            .field("path", &self.path)
            .finish()
    }
}

/// The fixed-lane exact arithmetic: sixteen binary64 lanes, the pairwise tree
/// `(l, l+8)`, `(l, l+4)`, `(l, l+2)`, `(0, 1)`, then a sequential ascending tail.
///
/// The same bits on every target and every dispatch path. See the
/// [module documentation](self) for the law.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Exact;

impl sealed::Sealed for Exact {}

impl Exact {
    /// The one code an image records the exact arithmetic under: every path computes
    /// the same bits, so the path is not recorded.
    pub const IMAGE_CODE: u32 = 1;
}

impl Arithmetic for Exact {
    const ID: &'static str = "binary64-lane16-tree-v1";
    const IMAGE_CODES: &'static [u32] = &[Self::IMAGE_CODE];

    fn image_code(path: Path) -> Option<u32> {
        matches!(path, Path::Portable | Path::Avx2).then_some(Self::IMAGE_CODE)
    }

    fn evidence(_path: Path) -> Option<&'static str> {
        None
    }

    fn build_shape() -> Option<BuildShape> {
        None
    }

    fn resolve() -> Result<Resolved<Self>, FloatEnvironmentError> {
        env::check()?;
        Ok(Resolved::on(dispatch::exact_path()))
    }

    fn resolve_recorded(code: u32) -> Result<Resolved<Self>, RecordedPathError> {
        env::check()?;
        if code != Self::IMAGE_CODE {
            return Err(RecordedPathError::UnknownCode {
                arithmetic: Self::ID,
                code,
            });
        }
        // Every exact path computes the same bits, so the one code is honoured on
        // whichever path this process selects.
        Ok(Resolved::on(dispatch::exact_path()))
    }

    fn distances<Q: Scalar, T: Scalar>(
        resolved: Resolved<Self>,
        measure: Measure,
        query: &[Q],
        query_norm: f64,
        rows: RowsRef<'_, T>,
        out: &mut [Option<f64>],
    ) {
        dispatch::distances(resolved.path, measure, query, query_norm, rows, out);
    }

    fn distances_indexed<Q: Scalar, T: Scalar>(
        resolved: Resolved<Self>,
        measure: Measure,
        query: &[Q],
        query_norm: f64,
        rows: RowsRef<'_, T>,
        ids: &[usize],
        out: &mut [Option<f64>],
    ) {
        dispatch::distances_indexed(resolved.path, measure, query, query_norm, rows, ids, out);
    }

    fn distance_on<Q: Scalar, T: Scalar>(
        resolved: Resolved<Self>,
        measure: Measure,
        a: &[Q],
        a_norm: f64,
        b: &[T],
        b_norm: f64,
    ) -> Option<f64> {
        dispatch::distance(resolved.path, measure, a, a_norm, b, b_norm)
    }

    fn distance_bounded_on<Q: Scalar, T: Scalar>(
        resolved: Resolved<Self>,
        measure: Measure,
        a: &[Q],
        a_norm: f64,
        b: &[T],
        b_norm: f64,
        bound: Bound,
    ) -> Bounded {
        dispatch::distance_bounded(resolved.path, measure, a, a_norm, b, b_norm, bound)
    }
}

/// The reassociated arithmetic: within each 64-element block the terms are summed with
/// the `algebraic_*` operations, so the compiler may reassociate the sum and contract
/// multiplies into adds; block sums are combined with plain `+` in ascending order.
///
/// Its bits depend on the target, the build and the dispatch path; they are a function
/// of the inputs only within one compiled build on one path. [`BuildShape`] names the
/// part of the build that decides them which the source can see. See the
/// [module documentation](self) and [`Arithmetic::evidence`] for what it gives up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Reassociated;

impl sealed::Sealed for Reassociated {}

impl Arithmetic for Reassociated {
    const ID: &'static str = "binary64-reassociated-v1";
    const IMAGE_CODES: &'static [u32] = &[2, 3, 4, 5, 6, 7, 8];

    fn image_code(path: Path) -> Option<u32> {
        match path {
            Path::Sse2 => Some(2),
            Path::Avx2Fma => Some(3),
            Path::Avx512f => Some(4),
            Path::Neon => Some(5),
            Path::WasmSimd128 => Some(6),
            Path::WasmScalar => Some(7),
            Path::Portable => Some(8),
            Path::Avx2 => None,
        }
    }

    fn evidence(path: Path) -> Option<&'static str> {
        Some(reassociated::evidence(path))
    }

    fn build_shape() -> Option<BuildShape> {
        Some(BuildShape::here())
    }

    fn resolve() -> Result<Resolved<Self>, FloatEnvironmentError> {
        env::check()?;
        Ok(Resolved::on(reassociated::path()))
    }

    fn resolve_recorded(code: u32) -> Result<Resolved<Self>, RecordedPathError> {
        env::check()?;
        let path = reassociated::path_of(code).ok_or(RecordedPathError::UnknownCode {
            arithmetic: Self::ID,
            code,
        })?;
        let path = reassociated::recorded(path)
            .map_err(|reason| RecordedPathError::Unavailable { path, reason })?;
        Ok(Resolved::on(path))
    }

    fn distances<Q: Scalar, T: Scalar>(
        resolved: Resolved<Self>,
        measure: Measure,
        query: &[Q],
        query_norm: f64,
        rows: RowsRef<'_, T>,
        out: &mut [Option<f64>],
    ) {
        reassociated::distances(resolved.path, measure, query, query_norm, rows, out);
    }

    fn distances_indexed<Q: Scalar, T: Scalar>(
        resolved: Resolved<Self>,
        measure: Measure,
        query: &[Q],
        query_norm: f64,
        rows: RowsRef<'_, T>,
        ids: &[usize],
        out: &mut [Option<f64>],
    ) {
        reassociated::distances_indexed(resolved.path, measure, query, query_norm, rows, ids, out);
    }

    fn distance_on<Q: Scalar, T: Scalar>(
        resolved: Resolved<Self>,
        measure: Measure,
        a: &[Q],
        a_norm: f64,
        b: &[T],
        b_norm: f64,
    ) -> Option<f64> {
        reassociated::distance(resolved.path, measure, a, a_norm, b, b_norm)
    }

    fn distance_bounded_on<Q: Scalar, T: Scalar>(
        resolved: Resolved<Self>,
        measure: Measure,
        a: &[Q],
        a_norm: f64,
        b: &[T],
        b_norm: f64,
        bound: Bound,
    ) -> Bounded {
        reassociated::distance_bounded(resolved.path, measure, a, a_norm, b, b_norm, bound)
    }
}

/// The PURREMB §13.2 fold itself, for the crate's own callers, each of which holds a
/// [`Resolved`] handle proving the float environment before it reaches here.
pub(crate) fn l2_norm<T: Scalar>(vector: &[T]) -> f64 {
    let mut scale = 0.0_f64;
    let mut sum_of_squares = 1.0_f64;
    for value in vector {
        crate::ir::embedding::norm_fold(value.widen().abs(), &mut scale, &mut sum_of_squares);
    }
    scale * sum_of_squares.sqrt()
}
