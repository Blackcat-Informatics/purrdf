// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Named execution paths, and the one requirement a test harness reads about
//! them.
//!
//! A kernel family with more than one compilation of the same function (a
//! portable body and processor-specific ones) names each compilation as a
//! path and implements [`Backend`] for it. Every path of a family computes the
//! same answer, so a test runs [`Backend::all_available`] against the portable
//! path and the frozen vectors, and a bench times each one.
//!
//! # `PURRDF_REQUIRE_SIMD_PATHS`
//!
//! A test that runs every available path proves nothing about a path the host
//! never offered: a detection that silently fails leaves only the portable
//! path, and every test still passes. [`REQUIRE_SIMD_PATHS`] turns "every path
//! the host runs" into a checked claim. Its value is one of:
//!
//! * `1`: every path this host is *expected* to run — its architecture's
//!   baseline paths, and each path whose processor features the host
//!   advertises independently of the detection under test (Linux
//!   `/proc/cpuinfo`, see [`host_advertises`]) — is required.
//! * a comma-separated list of `family:path` entries, such as
//!   `distance:sse2,deflate:avx2`: exactly the named paths are required, and
//!   a job on a known or emulated processor names what that processor must
//!   run.
//!
//! Unset, nothing is required. Any other value fails the run, as does an
//! entry naming a family outside [`FAMILIES`] or a path its family does not
//! have: a misspelt requirement must fail rather than require nothing.
//!
//! What "required" then asserts is the family's contract: that the path is
//! available ([`assert_required_available`]), and for some families that it
//! is also the selected path or that a test executed it.

/// The environment variable a test harness reads to name the execution paths
/// its host must run.
pub const REQUIRE_SIMD_PATHS: &str = "PURRDF_REQUIRE_SIMD_PATHS";

/// Every family of named paths [`REQUIRE_SIMD_PATHS`] can address, sorted: the
/// family is the part of an entry before its `:`.
///
/// | Family | Paths of |
/// |---|---|
/// | `blake3` | BLAKE3 chunk and block kernels (`purrdf_hash::blake3`) |
/// | `crc32` | the CRC-32 register update (`purrdf_hash::crc32`) |
/// | `csv` | the CSV field scanner (`purrdf_core::csv`) |
/// | `deflate` | DEFLATE match copy, compare and window hashing (`purrdf_deflate`) |
/// | `distance` | the vector distance arithmetic (`purrdf_core::distance`) |
/// | `hex` | base16 encoding (`purrdf_hash::hex`) |
/// | `sha1` | the SHA-1 block function (`purrdf_hash::sha1`) |
pub const FAMILIES: [&str; 7] = [
    "blake3", "crc32", "csv", "deflate", "distance", "hex", "sha1",
];

/// A family of named execution paths of one function.
///
/// Every path computes the same answer; they differ only in the instructions
/// they run. [`ALL`](Self::ALL) lists every path in order of preference, and
/// the last one is always available, so [`selected`](Self::selected) always
/// has an answer.
pub trait Backend: Copy + Eq + Sized + 'static {
    /// Every path, available or not, in order of preference; the last is
    /// available on every build and processor.
    const ALL: &'static [Self];

    /// The path the public API runs on this build and processor. By default
    /// the first available path of [`ALL`](Self::ALL).
    #[must_use]
    fn selected() -> Self {
        Self::all_available()
            .next()
            .expect("the last path of `Backend::ALL` is always available")
    }

    /// Whether this build and processor can run the path.
    fn is_available(self) -> bool;

    /// Every path this build and processor can run, in order of preference.
    fn all_available() -> impl Iterator<Item = Self> {
        Self::ALL
            .iter()
            .copied()
            .filter(|backend| backend.is_available())
    }

    /// The path's stable name, as test output prints it and
    /// [`REQUIRE_SIMD_PATHS`] spells it.
    fn name(self) -> &'static str;
}

/// Declare a [`Backend`] family on the vector-ISA ladder every byte kernel of
/// the workspace is compiled for: `Portable`, `Sse2`, `Avx2`, `Neon` and
/// `Simd128`, preferred widest first (`Avx2`, `Sse2`, `Neon`, `Simd128`, then
/// `Portable`), each named by its lowercase spelling.
///
/// The ladder, its order and its names are one thing, stated here; a family
/// supplies its documentation, each path's documentation, and which paths this
/// build and processor can run.
///
/// ```rust
/// use purrdf_hash::Backend as _;
///
/// purrdf_hash::vector_backend! {
///     /// A kernel path.
///     pub enum Path {
///         /// Always available.
///         Portable,
///         /// SSE2.
///         Sse2,
///         /// AVX2.
///         Avx2,
///         /// NEON.
///         Neon,
///         /// wasm `simd128`.
///         Simd128,
///     }
///     available: |path| path == Path::Portable;
/// }
///
/// assert_eq!(Path::selected(), Path::Portable);
/// assert_eq!(Path::Avx2.name(), "avx2");
/// ```
#[macro_export]
macro_rules! vector_backend {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $(#[$portable:meta])* Portable,
            $(#[$sse2:meta])* Sse2,
            $(#[$avx2:meta])* Avx2,
            $(#[$neon:meta])* Neon,
            $(#[$simd128:meta])* Simd128,
        }
        available: |$path:ident| $available:expr;
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        $vis enum $name {
            $(#[$portable])* Portable,
            $(#[$sse2])* Sse2,
            $(#[$avx2])* Avx2,
            $(#[$neon])* Neon,
            $(#[$simd128])* Simd128,
        }

        impl $crate::Backend for $name {
            const ALL: &'static [Self] = &[
                Self::Avx2,
                Self::Sse2,
                Self::Neon,
                Self::Simd128,
                Self::Portable,
            ];

            fn is_available(self) -> bool {
                let $path = self;
                $available
            }

            fn name(self) -> &'static str {
                match self {
                    Self::Portable => "portable",
                    Self::Sse2 => "sse2",
                    Self::Avx2 => "avx2",
                    Self::Neon => "neon",
                    Self::Simd128 => "simd128",
                }
            }
        }
    };
}

/// Whether this host advertises every processor feature in `flags`,
/// independently of the run-time detection a test is checking: the `flags`
/// (x86) or `Features` (Arm) line of Linux `/proc/cpuinfo`. An empty `flags`
/// is advertised everywhere; a host without that file advertises nothing
/// else, so only explicitly named paths are required there.
#[must_use]
pub fn host_advertises(flags: &[&str]) -> bool {
    if flags.is_empty() {
        return true;
    }
    let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") else {
        return false;
    };
    let Some(advertised) = cpuinfo
        .lines()
        .find(|line| line.starts_with("flags") || line.starts_with("Features"))
        .and_then(|line| line.split_once(':'))
        .map(|(_, advertised)| advertised)
    else {
        return false;
    };
    flags
        .iter()
        .all(|flag| advertised.split_whitespace().any(|have| have == *flag))
}

/// The names of `family`'s paths that [`REQUIRE_SIMD_PATHS`] requires, in the
/// order `known` lists them: none when it is unset; under `1`, each `known`
/// path for which `expected_here` holds; otherwise the paths it names for
/// `family`.
///
/// `known` is every path of the family, compiled here or not, and
/// `expected_here` is the family's host oracle: whether this host's
/// architecture and advertised features ([`host_advertises`]) mean it is
/// expected to run the path.
///
/// # Panics
///
/// When `family` is not in [`FAMILIES`], or the variable is set and is not
/// UTF-8, is neither `1` nor a list of `family:path` entries, or names an
/// unknown family, or a path `family` does not have.
#[must_use]
pub fn required_names(
    family: &str,
    known: &[&'static str],
    expected_here: impl Fn(&'static str) -> bool,
) -> Vec<&'static str> {
    let Some(value) = std::env::var_os(REQUIRE_SIMD_PATHS) else {
        return Vec::new();
    };
    let value = value
        .into_string()
        .unwrap_or_else(|raw| panic!("{REQUIRE_SIMD_PATHS} is not UTF-8: {raw:?}"));
    parse(&value, family, known, expected_here).unwrap_or_else(|refusal| panic!("{refusal}"))
}

/// [`required_names`] for a [`Backend`] family.
///
/// # Panics
///
/// As [`required_names`].
#[must_use]
pub fn required<B: Backend>(family: &str, expected_here: impl Fn(B) -> bool) -> Vec<B> {
    let known: Vec<&'static str> = B::ALL.iter().map(|backend| backend.name()).collect();
    let by_name = |name: &str| {
        B::ALL
            .iter()
            .copied()
            .find(|backend| backend.name() == name)
            .expect("a required name is one of the family's paths")
    };
    required_names(family, &known, |name| expected_here(by_name(name)))
        .into_iter()
        .map(by_name)
        .collect()
}

/// Asserts that every path of `family` that [`REQUIRE_SIMD_PATHS`] requires
/// ([`required`]) is available on this build and processor, and returns them.
///
/// # Panics
///
/// As [`required`], and when a required path is unavailable.
pub fn assert_required_available<B: Backend>(
    family: &str,
    expected_here: impl Fn(B) -> bool,
) -> Vec<B> {
    let required = required::<B>(family, expected_here);
    for backend in &required {
        assert!(
            backend.is_available(),
            "{REQUIRE_SIMD_PATHS} requires {family}:{}, which this build and processor cannot \
             run (available: {})",
            backend.name(),
            B::all_available()
                .map(Backend::name)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    required
}

/// The requirement `value` makes of `family`, or why `value` is refused.
fn parse(
    value: &str,
    family: &str,
    known: &[&'static str],
    expected_here: impl Fn(&'static str) -> bool,
) -> Result<Vec<&'static str>, String> {
    assert!(
        FAMILIES.contains(&family),
        "`{family}` is not a family of {REQUIRE_SIMD_PATHS}; the families are: {}",
        FAMILIES.join(", ")
    );
    if value.trim() == "1" {
        return Ok(known
            .iter()
            .copied()
            .filter(|&name| expected_here(name))
            .collect());
    }
    let mut names: Vec<&'static str> = Vec::new();
    let mut entries = 0_usize;
    for entry in value
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        entries += 1;
        let Some((entry_family, path)) = entry.split_once(':') else {
            return Err(format!(
                "{REQUIRE_SIMD_PATHS} entry `{entry}` is not `family:path`; set the variable \
                 to `1` or to a comma-separated list of `family:path` entries, or unset it to \
                 require nothing"
            ));
        };
        if !FAMILIES.contains(&entry_family) {
            return Err(format!(
                "{REQUIRE_SIMD_PATHS} entry `{entry}` names `{entry_family}`, which is not a \
                 family; the families are: {}",
                FAMILIES.join(", ")
            ));
        }
        if entry_family != family {
            continue;
        }
        let Some(&name) = known.iter().find(|&&name| name == path) else {
            return Err(format!(
                "{REQUIRE_SIMD_PATHS} entry `{entry}` names `{path}`, which is not a \
                 {family} path; the paths are: {}",
                known.join(", ")
            ));
        };
        if !names.contains(&name) {
            names.push(name);
        }
    }
    if entries == 0 {
        return Err(format!(
            "{REQUIRE_SIMD_PATHS} is set but names nothing ({value:?}); set it to `1` or to a \
             comma-separated list of `family:path` entries, or unset it to require nothing"
        ));
    }
    Ok(known
        .iter()
        .copied()
        .filter(|name| names.contains(name))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{Backend, FAMILIES, host_advertises, parse};
    use crate::backend::{Crc32Backend, HexBackend, Sha1Backend};
    use crate::blake3::Backend as Blake3Backend;

    const KNOWN: [&str; 3] = ["portable", "sse2", "avx2"];

    fn only_portable(name: &'static str) -> bool {
        name == "portable"
    }

    #[test]
    fn one_requires_every_path_the_host_is_expected_to_run() {
        assert_eq!(
            parse("1", "csv", &KNOWN, only_portable),
            Ok(vec!["portable"])
        );
        assert_eq!(parse(" 1 ", "csv", &KNOWN, |_| true), Ok(KNOWN.to_vec()));
    }

    #[test]
    fn a_list_requires_exactly_its_own_family_entries_in_known_order() {
        assert_eq!(
            parse(
                "csv:avx2, deflate:neon,csv:portable,csv:avx2",
                "csv",
                &KNOWN,
                only_portable
            ),
            Ok(vec!["portable", "avx2"])
        );
        assert_eq!(
            parse("deflate:neon", "csv", &KNOWN, only_portable),
            Ok(Vec::new())
        );
    }

    #[test]
    fn an_unknown_family_is_refused() {
        let refusal = parse("defalte:neon", "csv", &KNOWN, only_portable).unwrap_err();
        assert!(
            refusal.contains("`defalte`, which is not a family"),
            "{refusal}"
        );
        // The neighbour: the family spelt correctly is accepted.
        assert!(parse("deflate:neon", "csv", &KNOWN, only_portable).is_ok());
    }

    #[test]
    fn an_unknown_path_of_the_family_is_refused() {
        let refusal = parse("csv:avx512", "csv", &KNOWN, only_portable).unwrap_err();
        assert!(
            refusal.contains("`avx512`, which is not a csv path"),
            "{refusal}"
        );
        // The neighbour: a path the family has is accepted.
        assert_eq!(
            parse("csv:avx2", "csv", &KNOWN, only_portable),
            Ok(vec!["avx2"])
        );
    }

    #[test]
    fn an_entry_without_a_family_is_refused() {
        let refusal = parse("sse2", "csv", &KNOWN, only_portable).unwrap_err();
        assert!(refusal.contains("is not `family:path`"), "{refusal}");
        // `0` does not mean "off": unset means off, so `0` is refused too.
        assert!(parse("0", "csv", &KNOWN, only_portable).is_err());
        // The neighbour: `1` is the one bare value.
        assert!(parse("1", "csv", &KNOWN, only_portable).is_ok());
    }

    #[test]
    fn a_value_naming_nothing_is_refused() {
        for empty in ["", " ", ",", " , ,"] {
            let refusal = parse(empty, "csv", &KNOWN, only_portable).unwrap_err();
            assert!(refusal.contains("names nothing"), "{empty:?}: {refusal}");
        }
        // The neighbour: one entry, even of another family, names something.
        assert_eq!(
            parse(",hex:portable,", "csv", &KNOWN, only_portable),
            Ok(Vec::new())
        );
    }

    #[test]
    #[should_panic(expected = "is not a family of PURRDF_REQUIRE_SIMD_PATHS")]
    fn a_caller_family_outside_the_registry_panics() {
        let _ = parse("1", "csvw", &KNOWN, only_portable);
    }

    #[test]
    fn the_families_are_sorted_and_unique() {
        assert!(FAMILIES.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn nothing_but_the_empty_set_is_advertised_without_a_name() {
        assert!(host_advertises(&[]));
        assert!(!host_advertises(&["no-processor-has-this-feature"]));
    }

    /// Every path family of this crate selects one of its available paths,
    /// and its last path is available everywhere.
    #[test]
    fn every_family_selects_an_available_path() {
        fn check<B: Backend + core::fmt::Debug>() {
            let available: Vec<B> = B::all_available().collect();
            assert!(available.contains(&B::selected()), "{available:?}");
            assert!(B::ALL.last().is_some_and(|last| last.is_available()));
            let mut names: Vec<&str> = B::ALL.iter().map(|backend| backend.name()).collect();
            names.sort_unstable();
            names.dedup();
            assert_eq!(names.len(), B::ALL.len(), "path names are unique");
        }
        check::<Sha1Backend>();
        check::<Crc32Backend>();
        check::<HexBackend>();
        check::<Blake3Backend>();
    }
}
