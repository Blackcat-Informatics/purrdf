// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The field scanner: the offset of the first byte of a slice that belongs to
//! a small set of bytes chosen at run time.
//!
//! Every hot loop of the reader and the writer asks this one question. The row
//! splitter stops at the quote character, the escape character and the first
//! byte of each line terminator; the cell parser stops at the delimiter, the
//! quote character and the escape character; the writer's quoting decision
//! stops at the delimiter, the quote character, CR and LF. Those bytes come
//! from the [`Dialect`](super::Dialect), so the class is a run-time value and
//! cannot be a compile-time [`ByteClass`](crate::terminals::ByteClass).
//!
//! A [`StopSet`] holds up to [`LANES`] distinct bytes (padded by repeating the
//! first) and a 256-entry membership table. [`StopSet::find`] answers with the
//! widest kernel the target offers — see [`super::arch`] — and with the
//! portable kernel [`find_portable`] everywhere else, for sets wider than
//! [`LANES`] bytes, and for slices shorter than one vector.

use super::arch;

/// How many distinct bytes the vector kernels compare each chunk against.
pub(crate) const LANES: usize = 8;

/// Bytes per portable chunk.
const CHUNK: usize = 16;

/// A run-time set of bytes a scan stops at.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct StopSet {
    /// The distinct members, in first-seen order, padded to [`LANES`] by
    /// repeating the first member (a repeated needle cannot change an answer).
    /// Meaningful only when `count <= LANES`.
    needles: [u8; LANES],
    /// How many distinct members the set has (0..=256).
    count: usize,
    /// Entry `b` is `true` when byte `b` is a member.
    table: [bool; 256],
}

impl std::fmt::Debug for StopSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let members: Vec<u8> = (0..=u8::MAX).filter(|&b| self.contains(b)).collect();
        f.debug_struct("StopSet")
            .field("members", &members)
            .finish()
    }
}

impl StopSet {
    /// The set whose members are the bytes of `bytes` (repeats ignored).
    pub(crate) const fn new(bytes: &[u8]) -> Self {
        let mut table = [false; 256];
        let mut needles = [0_u8; LANES];
        let mut count = 0;
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if !table[b as usize] {
                table[b as usize] = true;
                if count < LANES {
                    needles[count] = b;
                }
                count += 1;
            }
            i += 1;
        }
        if count > 0 {
            let mut lane = if count < LANES { count } else { LANES };
            while lane < LANES {
                needles[lane] = needles[0];
                lane += 1;
            }
        }
        Self {
            needles,
            count,
            table,
        }
    }

    /// Whether `b` is a member.
    #[inline]
    pub(crate) const fn contains(&self, b: u8) -> bool {
        self.table[b as usize]
    }

    /// Whether the vector kernels answer this set: it has between one and
    /// [`LANES`] members.
    #[inline]
    pub(crate) const fn vectorizable(&self) -> bool {
        self.count > 0 && self.count <= LANES
    }

    /// The members as vector needles, padded to [`LANES`].
    #[inline]
    pub(crate) const fn needles(&self) -> &[u8; LANES] {
        &self.needles
    }

    /// The offset of the first member byte of `haystack`, or `None`.
    #[inline]
    pub(crate) fn find(&self, haystack: &[u8]) -> Option<usize> {
        if !self.vectorizable() || haystack.len() < CHUNK {
            return self.find_in_tail(haystack);
        }
        arch::find(self, haystack)
    }

    /// The first member of a short slice, one table load per byte.
    #[inline]
    pub(crate) fn find_in_tail(&self, haystack: &[u8]) -> Option<usize> {
        haystack.iter().position(|&b| self.table[usize::from(b)])
    }
}

/// The portable kernel: sixteen-byte chunks whose lanes are answered by one
/// comparison per needle with no data-dependent branch, OR-folded as the
/// clean-chunk test (the formulation of
/// [`ByteClass`](crate::terminals::ByteClass), over run-time needles), and a
/// table-driven tail.
///
/// This is the answer on every target without an explicit kernel, and the
/// oracle every explicit kernel is tested against — by this crate's own unit
/// tests, and by [`super::backend::kernels`] for the wasm32 differential
/// integration test, so it is exercised on every target regardless of which
/// explicit kernel that target also compiles.
#[inline(never)]
pub(crate) fn find_portable(set: &StopSet, haystack: &[u8]) -> Option<usize> {
    if set.count == 0 {
        return None;
    }
    if set.count > LANES {
        return set.find_in_tail(haystack);
    }
    let needles = set.needles();
    let (chunks, tail) = haystack.as_chunks::<CHUNK>();
    for (k, chunk) in chunks.iter().enumerate() {
        let mut lanes = [0_u8; CHUNK];
        for (lane, &b) in lanes.iter_mut().zip(chunk) {
            let mut hit = false;
            for &needle in needles {
                hit |= b == needle;
            }
            *lane = u8::from(hit);
        }
        if lanes.iter().fold(0, |any, &lane| any | lane) != 0 {
            return lanes
                .iter()
                .position(|&lane| lane != 0)
                .map(|first| k * CHUNK + first);
        }
    }
    set.find_in_tail(tail)
        .map(|offset| chunks.len() * CHUNK + offset)
}

/// A kernel's signature: what [`super::arch::kernel`] resolves a
/// [`super::backend::Backend`] to, for the differential tests (this crate's own
/// unit tests below, and [`super::backend`], which adapts it to a
/// `StopSet`-free signature an integration test outside this crate can call).
pub(crate) type Kernel = fn(&StopSet, &[u8]) -> Option<usize>;

#[cfg(test)]
mod tests {
    use purrdf_hash::Backend as _;
    use purrdf_hash::dispatch::{assert_required_available, host_advertises};
    use purrdf_testkit::rng::SplitMix64;

    use super::super::backend::Backend;
    use super::{Kernel, LANES, StopSet};

    /// The per-byte answer every kernel must equal.
    fn reference(set: &StopSet, haystack: &[u8]) -> Option<usize> {
        haystack.iter().position(|&b| set.contains(b))
    }

    /// The live dispatch and every kernel this build and processor run, by
    /// name.
    fn kernels() -> Vec<(&'static str, Kernel)> {
        let mut kernels: Vec<(&'static str, Kernel)> =
            vec![("dispatch", |set, haystack| set.find(haystack))];
        kernels.extend(Backend::all_available().map(|backend| {
            let kernel = super::super::arch::kernel(backend).expect("an available path");
            (backend.name(), kernel)
        }));
        kernels
    }

    fn sets() -> Vec<StopSet> {
        vec![
            StopSet::new(b""),
            StopSet::new(b","),
            StopSet::new(b",\"\r\n"),
            StopSet::new(b"\t\"\\\r\n"),
            StopSet::new(b";\"\\\r\n\xe2"),
            StopSet::new(b"abcdefgh"),
            // Nine members: wider than the vector kernels, answered portably.
            StopSet::new(b"abcdefghi"),
            // Repeats collapse.
            StopSet::new(b",,,,"),
            // The byte values at both ends of the range.
            StopSet::new(b"\x00\xff"),
        ]
    }

    #[test]
    fn a_set_of_repeats_is_one_needle_and_padding_never_adds_a_member() {
        let set = StopSet::new(b",,,,");
        assert_eq!(set.count, 1);
        assert_eq!(set.needles(), &[b','; LANES]);
        assert!(set.contains(b',') && !set.contains(0));
        let empty = StopSet::new(b"");
        assert!(!empty.contains(0));
        assert_eq!(empty.find(&[0; 64]), None);
    }

    /// Every kernel against the per-byte answer at every start alignment
    /// 0..64 and every length 0..256, with the first member planted at every
    /// position (and nowhere).
    #[test]
    fn every_kernel_agrees_with_the_per_byte_scan_at_every_alignment_and_length() {
        let kernels = kernels();
        // The explicit kernel the target compiles is among those compared, so
        // a cfg slip cannot leave it untested while this passes.
        let names: Vec<&str> = kernels.iter().map(|(name, _)| *name).collect();
        let explicit = if cfg!(target_arch = "x86_64") {
            Some("sse2")
        } else if cfg!(target_arch = "aarch64") {
            Some("neon")
        } else if cfg!(all(target_arch = "wasm32", target_feature = "simd128")) {
            Some("simd128")
        } else {
            None
        };
        if let Some(explicit) = explicit {
            assert!(names.contains(&explicit), "{names:?}");
        }
        let mut backing = vec![b'.'; 64 + 256 + 64];
        for set in sets() {
            let member = (0..=u8::MAX).find(|&b| set.contains(b));
            let filler = (0..=u8::MAX)
                .find(|&b| !set.contains(b))
                .expect("no set covers every byte");
            for start in 0..64 {
                for len in 0..256 {
                    let plants = std::iter::once(None).chain((0..len).map(Some));
                    for plant in plants {
                        backing.fill(filler);
                        // A member beyond the slice must never be reported.
                        if let Some(m) = member {
                            backing[start + len] = m;
                        }
                        if let (Some(at), Some(m)) = (plant, member) {
                            backing[start + at] = m;
                        }
                        let haystack = &backing[start..start + len];
                        let expected = reference(&set, haystack);
                        for (name, kernel) in &kernels {
                            assert_eq!(
                                kernel(&set, haystack),
                                expected,
                                "{name} start {start} len {len} plant {plant:?} set {set:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    /// A uniform-enough draw below `n` (`n > 0`) for input generation.
    fn below(rng: &mut SplitMix64, n: usize) -> usize {
        (rng.next_u64() % n as u64) as usize
    }

    /// Seeded random haystacks over a small alphabet, so members are dense in
    /// some and absent from others.
    #[test]
    fn every_kernel_agrees_with_the_per_byte_scan_over_seeded_inputs() {
        let kernels = kernels();
        let mut rng = SplitMix64::new(0x00C5_F1E1_D5CA_2026);
        for _ in 0..4_000 {
            let members: Vec<u8> = (0..below(&mut rng, 10))
                .map(|_| rng.next_u64().to_le_bytes()[0])
                .collect();
            let set = StopSet::new(&members);
            let alphabet: Vec<u8> = (0..=below(&mut rng, 12))
                .map(|_| rng.next_u64().to_le_bytes()[0])
                .collect();
            let len = below(&mut rng, 700);
            let haystack: Vec<u8> = (0..len)
                .map(|_| alphabet[below(&mut rng, alphabet.len())])
                .collect();
            let start = below(&mut rng, len.max(1)).min(len);
            let haystack = &haystack[start..];
            let expected = reference(&set, haystack);
            for (name, kernel) in &kernels {
                assert_eq!(kernel(&set, haystack), expected, "{name} {set:?}");
            }
        }
    }

    /// Every available path is one of the family's, the selected one among
    /// them, and the architecture's baseline vector kernel is available where
    /// the target has one.
    #[test]
    fn the_selected_kernel_is_among_the_available_ones() {
        let available: Vec<Backend> = Backend::all_available().collect();
        assert!(available.contains(&Backend::selected()), "{available:?}");
        assert!(available.contains(&Backend::Portable), "{available:?}");
        #[cfg(target_arch = "x86_64")]
        assert!(available.contains(&Backend::Sse2), "{available:?}");
        #[cfg(target_arch = "aarch64")]
        assert_eq!(Backend::selected(), Backend::Neon);
    }

    /// Whether this host is expected to run a field-scanner path: its
    /// architecture and build, and the processor features it advertises
    /// independently of the detection under test.
    fn expected_here(backend: Backend) -> bool {
        match backend {
            Backend::Portable => true,
            Backend::Sse2 => cfg!(target_arch = "x86_64"),
            Backend::Avx2 => cfg!(target_arch = "x86_64") && host_advertises(&["avx2"]),
            Backend::Neon => cfg!(target_arch = "aarch64"),
            Backend::Simd128 => cfg!(all(target_arch = "wasm32", target_feature = "simd128")),
        }
    }

    /// Every `csv` path `PURRDF_REQUIRE_SIMD_PATHS` requires is available, so
    /// the differentials above ran it; under `1` that is the architecture's
    /// baseline kernel (`sse2`, `neon`) and AVX2 where the host advertises it.
    #[test]
    fn required_csv_paths_are_available() {
        let required = assert_required_available("csv", expected_here);
        println!(
            "csv paths required: {}",
            required
                .iter()
                .map(|backend| backend.name())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
}
