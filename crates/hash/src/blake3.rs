// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Unkeyed BLAKE3-256, from the BLAKE3 specification §2.1–2.5.
//!
//! PurRDF's content identities use this mode and its first 32 output bytes.
//! Input is limited by the specification to fewer than 2^64 bytes. Tree
//! storage is bounded; neither streaming nor one-shot hashing allocates.

use crate::dispatch::Backend as _;

pub(crate) const IV: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];
/// Execution paths used by conformance tests and performance measurements.
/// Every path implements the same unkeyed BLAKE3-256 function.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    /// Scalar 32-bit arithmetic, including 32-bit and baseline wasm targets.
    Portable,
    /// Four chunks and four quarter rounds on baseline x86-64 SSE2.
    Sse2,
    /// SSE2 chunk batching and SSSE3 byte rotations for single blocks.
    Ssse3,
    /// Eight chunks in AVX2 lanes, without requiring AVX-512.
    Avx2,
    /// Sixteen chunks with AVX-512F, and single blocks with AVX-512F/VL.
    Avx512,
    /// Four chunks on little-endian AArch64 NEON.
    Neon,
    /// Four chunks on wasm SIMD128.
    Wasm128,
}
impl crate::dispatch::Backend for Backend {
    const ALL: &'static [Self] = &[
        Self::Avx512,
        Self::Avx2,
        Self::Ssse3,
        Self::Sse2,
        Self::Neon,
        Self::Wasm128,
        Self::Portable,
    ];

    fn is_available(self) -> bool {
        match self {
            Self::Portable => true,
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            Self::Sse2 => std::is_x86_feature_detected!("sse2"),
            #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
            Self::Sse2 => false,
            Self::Neon => cfg!(all(
                target_arch = "aarch64",
                target_feature = "neon",
                target_endian = "little"
            )),
            Self::Wasm128 => cfg!(all(target_arch = "wasm32", target_feature = "simd128")),
            #[cfg(target_arch = "x86_64")]
            Self::Ssse3 => std::is_x86_feature_detected!("ssse3"),
            #[cfg(target_arch = "x86_64")]
            Self::Avx2 => std::is_x86_feature_detected!("avx2"),
            #[cfg(target_arch = "x86_64")]
            Self::Avx512 => {
                std::is_x86_feature_detected!("avx512f")
                    && std::is_x86_feature_detected!("avx512vl")
            }
            #[cfg(not(target_arch = "x86_64"))]
            Self::Ssse3 | Self::Avx2 | Self::Avx512 => false,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Portable => "portable",
            Self::Sse2 => "sse2",
            Self::Ssse3 => "ssse3",
            Self::Avx2 => "avx2",
            Self::Avx512 => "avx512",
            Self::Neon => "neon",
            Self::Wasm128 => "simd128",
        }
    }
}

impl Backend {
    /// Hash on this path, or return `None` if the path cannot execute here.
    #[inline]
    pub fn hash(self, data: &[u8]) -> Option<Hash> {
        self.is_available().then(|| hash_on(data, self))
    }
    /// A stream pinned to this backend, if available.
    pub fn hasher(self) -> Option<Hasher> {
        self.is_available().then(|| Streaming::on(self))
    }
    /// A compact record stream pinned to this backend, if available.
    pub fn record_hasher(self) -> Option<RecordHasher> {
        self.is_available().then(|| Streaming::on(self))
    }
    /// A stream with the specified buffer size, pinned to this backend.
    pub fn streaming<const BUFFER: usize>(self) -> Option<Streaming<BUFFER>> {
        self.is_available().then(|| Streaming::on(self))
    }
    #[inline]
    fn compress(
        self,
        cv: [u32; 8],
        words: &[u32; 16],
        counter: u64,
        length: u32,
        flags: u32,
    ) -> [u32; 8] {
        if self == Self::Portable {
            compress_portable(cv, words, counter, length, flags)
        } else {
            crate::arch::blake3_compress_on(self, cv, words, counter, length, flags)
                .expect("backend availability was checked at construction")
        }
    }
}

const START: u32 = 1;
const END: u32 = 2;
const PARENT: u32 = 4;
const ROOT: u32 = 8;
const CHUNK: usize = 1024;

const fn schedules() -> [[usize; 16]; 7] {
    let permutation = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8];
    let mut out = [[0; 16]; 7];
    let mut i = 0;
    while i < 16 {
        out[0][i] = i;
        i += 1;
    }
    let mut round = 1;
    while round < 7 {
        i = 0;
        while i < 16 {
            out[round][i] = out[round - 1][permutation[i]];
            i += 1;
        }
        round += 1;
    }
    out
}
pub(crate) const SCHEDULE: [[usize; 16]; 7] = schedules();

#[inline]
fn rotate<const N: u32>(value: u32) -> u32 {
    #[cfg(all(target_arch = "x86_64", not(miri)))]
    {
        crate::arch::blake3_x86::rotate::<N>(value)
    }
    #[cfg(not(all(target_arch = "x86_64", not(miri))))]
    {
        value.rotate_right(N)
    }
}

pub(crate) fn compress_portable(
    cv: [u32; 8],
    words: &[u32; 16],
    counter: u64,
    length: u32,
    flags: u32,
) -> [u32; 8] {
    let mut v = [0; 16];
    v[..8].copy_from_slice(&cv);
    v[8..12].copy_from_slice(&IV[..4]);
    v[12] = counter as u32;
    v[13] = (counter >> 32) as u32;
    v[14] = length;
    v[15] = flags;
    macro_rules! mix {
        ($a:expr,$b:expr,$c:expr,$d:expr,$x:expr,$y:expr) => {{
            v[$a] = v[$a].wrapping_add(v[$b]).wrapping_add($x);
            v[$d] = rotate::<16>(v[$d] ^ v[$a]);
            v[$c] = v[$c].wrapping_add(v[$d]);
            v[$b] = rotate::<12>(v[$b] ^ v[$c]);
            v[$a] = v[$a].wrapping_add(v[$b]).wrapping_add($y);
            v[$d] = rotate::<8>(v[$d] ^ v[$a]);
            v[$c] = v[$c].wrapping_add(v[$d]);
            v[$b] = rotate::<7>(v[$b] ^ v[$c]);
        }};
    }
    // Expand the schedule here: outlining round functions prevents constant
    // message indices and causes repeated array traffic on smaller CPUs.
    macro_rules! round {
        ($r:expr) => {{
            let s = SCHEDULE[$r];
            mix!(0, 4, 8, 12, words[s[0]], words[s[1]]);
            mix!(1, 5, 9, 13, words[s[2]], words[s[3]]);
            mix!(2, 6, 10, 14, words[s[4]], words[s[5]]);
            mix!(3, 7, 11, 15, words[s[6]], words[s[7]]);
            mix!(0, 5, 10, 15, words[s[8]], words[s[9]]);
            mix!(1, 6, 11, 12, words[s[10]], words[s[11]]);
            mix!(2, 7, 8, 13, words[s[12]], words[s[13]]);
            mix!(3, 4, 9, 14, words[s[14]], words[s[15]]);
        }};
    }
    round!(0);
    round!(1);
    round!(2);
    round!(3);
    round!(4);
    round!(5);
    round!(6);
    core::array::from_fn(|i| v[i] ^ v[i + 8])
}

fn words(block: &[u8]) -> [u32; 16] {
    let mut padded = [0; 64];
    padded[..block.len()].copy_from_slice(block);
    core::array::from_fn(|i| {
        u32::from_le_bytes(*padded[i * 4..].first_chunk().expect("four bytes"))
    })
}

#[derive(Clone, Copy)]
struct Output {
    cv: [u32; 8],
    words: [u32; 16],
    counter: u64,
    length: u32,
    flags: u32,
}
impl Output {
    #[inline]
    fn chaining(self, backend: Backend) -> [u32; 8] {
        backend.compress(self.cv, &self.words, self.counter, self.length, self.flags)
    }
    #[inline]
    fn root(self, backend: Backend) -> Hash {
        let cv = backend.compress(self.cv, &self.words, 0, self.length, self.flags | ROOT);
        let mut bytes = [0; 32];
        for (dest, word) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(cv) {
            dest.copy_from_slice(&word.to_le_bytes());
        }
        Hash(bytes)
    }
}
#[inline]
fn parent(left: [u32; 8], right: [u32; 8]) -> Output {
    let mut words = [0; 16];
    words[..8].copy_from_slice(&left);
    words[8..].copy_from_slice(&right);
    Output {
        cv: IV,
        words,
        counter: 0,
        length: 64,
        flags: PARENT,
    }
}
fn chunk(bytes: &[u8], counter: u64, backend: Backend) -> Output {
    debug_assert!(bytes.len() <= CHUNK);
    let mut cv = IV;
    let mut offset = 0;
    while bytes.len() - offset > 64 {
        cv = backend.compress(
            cv,
            &words(&bytes[offset..offset + 64]),
            counter,
            64,
            if offset == 0 { START } else { 0 },
        );
        offset += 64;
    }
    Output {
        cv,
        words: words(&bytes[offset..]),
        counter,
        length: (bytes.len() - offset) as u32,
        flags: END | if offset == 0 { START } else { 0 },
    }
}

/// A 256-bit content digest, in BLAKE3's little-endian output order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Hash([u8; 32]);
impl Hash {
    /// The digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Streaming BLAKE3 optimized for file, codec and index streams. Its 16 KiB
/// buffer batches tiny writes across SIMD lanes. Reuse it with `reset` when
/// hashing successive messages to amortize state initialization.
pub type Hasher = Streaming<16384>;

/// Compact streaming state for short, framed record identities. Uses a 1 KiB
/// buffer; long byte streams should use [`Hasher`] to retain SIMD batching.
/// The buffer size never changes the digest.
pub type RecordHasher = Streaming<1024>;

/// The shared bounded streaming engine. Prefer [`Hasher`] or [`RecordHasher`].
/// The two supported buffer sizes are 1024 and 16384 bytes.
#[derive(Clone)]
pub struct Streaming<const BUFFER: usize> {
    backend: Backend,
    short_backend: Backend,
    buffer: [u8; BUFFER],
    filled: usize,
    pending: Option<(Output, u32)>,
    chunks: u64,
    // At most 54 complete subtree roots precede the final chunk in a
    // message shorter than 2^64 bytes (2^54 chunks).
    stack: [[u32; 8]; 54],
    depth: usize,
    length: u64,
}
impl<const BUFFER: usize> Streaming<BUFFER> {
    /// An empty message.
    #[must_use]
    pub fn new() -> Self {
        let mut stream = Self::on(Backend::selected());
        // Scalar quarter rounds expose four independent dependency chains to
        // x86's scheduler. Within-block SIMD serializes them into one vector
        // chain; measured short roots are faster with scalar ARX. Explicitly
        // forced streams keep the requested kernel for differential coverage.
        if cfg!(any(target_arch = "x86", target_arch = "x86_64")) {
            stream.short_backend = Backend::Portable;
        }
        stream
    }

    fn on(backend: Backend) -> Self {
        const {
            assert!(
                BUFFER == 1024 || BUFFER == 16384,
                "BLAKE3 stream buffer must be 1024 or 16384 bytes"
            );
        }
        Self {
            backend,
            short_backend: backend,
            buffer: [0; BUFFER],
            filled: 0,
            pending: None,
            chunks: 0,
            stack: [[0; 8]; 54],
            depth: 0,
            length: 0,
        }
    }
    fn push_subtree(&mut self, mut cv: [u32; 8], height: u32) {
        self.chunks += 1 << height;
        let mut count = self.chunks >> height;
        while count & 1 == 0 {
            self.depth -= 1;
            cv = parent(self.stack[self.depth], cv).chaining(self.short_backend);
            count >>= 1;
        }
        self.stack[self.depth] = cv;
        self.depth += 1;
    }
    /// Absorb bytes, retaining final compression inputs for root domain separation.
    ///
    /// # Panics
    /// If the total input length exceeds BLAKE3's limit of 2^64 - 1 bytes.
    pub fn update(&mut self, mut data: &[u8]) -> &mut Self {
        self.length = self
            .length
            .checked_add(u64::try_from(data.len()).expect("input length fits u64"))
            .expect("BLAKE3 input exceeds 2^64 - 1 bytes");
        if data.is_empty() {
            return self;
        }
        if let Some((output, height)) = self.pending.take() {
            self.push_subtree(output.chaining(self.short_backend), height);
        }
        if self.filled != 0 {
            let count = data.len().min(BUFFER - self.filled);
            self.buffer[self.filled..self.filled + count].copy_from_slice(&data[..count]);
            self.filled += count;
            data = &data[count..];
            if data.is_empty() {
                return self;
            }
            self.push_subtree(
                tree_cv(&self.buffer, self.chunks, self.backend),
                (BUFFER / CHUNK).ilog2(),
            );
            self.filled = 0;
        }
        while data.len() >= BUFFER {
            // Consume the largest aligned complete subtree directly from the
            // caller. Retain its Output when it ends the update: that keeps
            // ROOT information without copying a full SIMD batch into storage.
            let available_height = (data.len() / CHUNK).ilog2();
            let height = available_height.min(self.chunks.trailing_zeros());
            let bytes = CHUNK << height;
            if bytes == data.len() {
                self.pending = Some((tree_output(data, self.chunks, self.backend), height));
                return self;
            }
            self.push_subtree(tree_cv(&data[..bytes], self.chunks, self.backend), height);
            data = &data[bytes..];
        }
        self.buffer[..data.len()].copy_from_slice(data);
        self.filled = data.len();
        self
    }
    /// Start a new message, reusing the buffer and tree storage.
    /// Previously written bytes are inaccessible to the new message; this
    /// method is not a secure erasure operation.
    pub const fn reset(&mut self) {
        self.filled = 0;
        self.pending = None;
        self.chunks = 0;
        self.depth = 0;
        self.length = 0;
    }

    /// The digest so far, without consuming or altering the state.
    #[must_use]
    pub fn finalize(&self) -> Hash {
        if self.pending.is_none() && self.chunks == 0 {
            let backend = if self.filled <= CHUNK {
                self.short_backend
            } else {
                self.backend
            };
            return hash_on(&self.buffer[..self.filled], backend);
        }
        let tail_backend = if self.filled <= CHUNK {
            self.short_backend
        } else {
            self.backend
        };
        let mut output = self.pending.map_or_else(
            || tree_output(&self.buffer[..self.filled], self.chunks, tail_backend),
            |(output, _)| output,
        );
        for left in self.stack[..self.depth].iter().rev() {
            output = parent(*left, output.chaining(self.short_backend));
        }
        output.root(self.short_backend)
    }
}
impl<const BUFFER: usize> Default for Streaming<BUFFER> {
    fn default() -> Self {
        Self::new()
    }
}
impl<const BUFFER: usize> core::fmt::Debug for Streaming<BUFFER> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Blake3")
            .field("length", &self.length)
            .finish_non_exhaustive()
    }
}
impl<const BUFFER: usize> std::io::Write for Streaming<BUFFER> {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        let length = u64::try_from(data.len()).map_err(|_| std::io::ErrorKind::InvalidInput)?;
        if self.length.checked_add(length).is_none() {
            return Err(std::io::ErrorKind::InvalidInput.into());
        }
        self.update(data);
        Ok(data.len())
    }
    // All accepted bytes are already part of the digest state. Flushing must
    // neither force a partial chunk nor finalize/reset the message.
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<const BUFFER: usize> crate::Digest for Streaming<BUFFER> {
    fn output_len(&self) -> usize {
        32
    }
    fn update(&mut self, data: &[u8]) {
        Self::update(self, data);
    }
    fn finalize_reset(&mut self, out: &mut [u8]) -> usize {
        out[..32].copy_from_slice(self.finalize().as_bytes());
        self.reset();
        32
    }
    fn reset(&mut self) {
        Self::reset(self);
    }
}

/// Hash a contiguous message with the unkeyed BLAKE3-256 function.
#[must_use]
#[inline]
pub fn hash(data: &[u8]) -> Hash {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if data.len() <= CHUNK {
        return hash_on(data, Backend::Portable);
    }
    let backend = Backend::selected();
    let short = if cfg!(any(target_arch = "x86", target_arch = "x86_64")) {
        Backend::Portable
    } else {
        backend
    };
    if data.len() <= CHUNK {
        hash_on(data, short)
    } else {
        tree_output(data, 0, backend).root(short)
    }
}

#[inline]
fn hash_on(data: &[u8], backend: Backend) -> Hash {
    if data.len() <= 64 {
        let cv = backend.compress(IV, &words(data), 0, data.len() as u32, START | END | ROOT);
        let mut bytes = [0; 32];
        for (dest, word) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(cv) {
            dest.copy_from_slice(&word.to_le_bytes());
        }
        return Hash(bytes);
    }
    tree_output(data, 0, backend).root(backend)
}

/// Caller-owned scheduling for independent BLAKE3 subtrees.
///
/// Implementations may run both closures inline or concurrently. The hashing
/// crate owns counters, canonical tree shape and root domain separation; the
/// caller owns the pool and scheduling policy. No threads are created here.
pub trait Join: Sync {
    /// Execute both closures and return their results in left/right order.
    fn join<A: Send, B: Send>(
        &self,
        left: impl FnOnce() -> A + Send,
        right: impl FnOnce() -> B + Send,
    ) -> (A, B);
}

/// Hash using a caller-supplied scheduler for inputs at least
/// `minimum_parallel_bytes` long. Smaller subtrees use ordinary SIMD hashing.
/// Values below two chunks are clamped to two chunks. Scheduling never changes
/// bytes, and the caller may implement [`Join`] sequentially on threadless hosts.
#[must_use]
pub fn hash_with_join(data: &[u8], minimum_parallel_bytes: usize, join: &impl Join) -> Hash {
    if data.len() <= CHUNK {
        return hash(data);
    }
    let backend = Backend::selected();
    joined_output(
        data,
        0,
        backend,
        minimum_parallel_bytes.max(2 * CHUNK),
        join,
    )
    .root(backend)
}

fn joined_output(
    data: &[u8],
    counter: u64,
    backend: Backend,
    minimum: usize,
    join: &impl Join,
) -> Output {
    if data.len() < minimum {
        return tree_output(data, counter, backend);
    }
    let left_chunks = 1usize << ((data.len() - 1) / CHUNK).ilog2();
    let split = left_chunks * CHUNK;
    let (left, right) = join.join(
        || joined_output(&data[..split], counter, backend, minimum, join).chaining(backend),
        || {
            joined_output(
                &data[split..],
                counter + left_chunks as u64,
                backend,
                minimum,
                join,
            )
            .chaining(backend)
        },
    );
    parent(left, right)
}

fn tree_output(data: &[u8], counter: u64, backend: Backend) -> Output {
    #[cfg(target_arch = "x86_64")]
    if backend == Backend::Avx512 && data.len() == 4096 {
        let parents = crate::arch::blake3_subtree_four(
            data.try_into().expect("four complete chunks"),
            counter,
        )
        .expect("available four-chunk backend");
        return parent(parents[0], parents[1]);
    }
    if data.len() <= CHUNK {
        return chunk(data, counter, backend);
    }
    if backend == Backend::Avx512 && data.len() <= 4096 {
        return narrow_output::<4>(data, counter, backend);
    }
    #[cfg(target_arch = "x86_64")]
    if backend == Backend::Avx512
        && let Some(words) = crate::arch::blake3_x86::small_output(data, counter)
    {
        return Output {
            cv: IV,
            words,
            counter: 0,
            length: 64,
            flags: PARENT,
        };
    }
    // Reducing 256 chunk CVs together fills sixteen parent lanes across
    // neighbouring chunk batches instead of repeatedly leaving most idle.
    if backend == Backend::Avx512 && data.len() <= 262_144 && data.len().is_multiple_of(16_384) {
        return narrow_output::<256>(data, counter, backend);
    }
    if data.len() <= 65536 && backend != Backend::Portable && backend != Backend::Avx512 {
        return narrow_output::<64>(data, counter, backend);
    }
    let left_chunks = 1usize << ((data.len() - 1) / CHUNK).ilog2();
    let split = left_chunks * CHUNK;
    let left = tree_cv(&data[..split], counter, backend);
    let right = tree_cv(&data[split..], counter + left_chunks as u64, backend);
    parent(left, right)
}
fn tree_cv(data: &[u8], counter: u64, backend: Backend) -> [u32; 8] {
    #[cfg(target_arch = "x86_64")]
    if backend == Backend::Avx512
        && data.len() == 16 * CHUNK
        && let Some(cv) = crate::arch::blake3_x86::subtree(data, counter, 4)
    {
        return cv;
    }
    tree_output(data, counter, backend).chaining(backend)
}

fn narrow_output<const CAPACITY: usize>(data: &[u8], counter: u64, backend: Backend) -> Output {
    let lanes = match backend {
        Backend::Avx512 if data.len() <= 4096 => 4,
        Backend::Avx512 => 16,
        Backend::Avx2 => 8,
        _ => 4,
    };
    let mut cvs = [[0; 8]; CAPACITY];
    for (group, bytes) in data.chunks(lanes * CHUNK).enumerate() {
        let start = group * lanes;
        let count = bytes.len().div_ceil(CHUNK);
        #[cfg(target_arch = "x86_64")]
        if lanes == 16 {
            let batch = crate::arch::blake3_x86::chunks16(bytes, counter + start as u64)
                .expect("complete supported batch");
            cvs[start..start + count].copy_from_slice(&batch[..count]);
            continue;
        }
        #[cfg(target_arch = "x86_64")]
        if backend == Backend::Avx512 {
            let batch = crate::arch::blake3_four_avx512(bytes, counter + start as u64)
                .expect("available four-lane backend");
            cvs[start..start + count].copy_from_slice(&batch[..count]);
            continue;
        }
        if lanes == 8 {
            let batch = crate::arch::blake3_eight(bytes, counter + start as u64)
                .expect("available backend");
            cvs[start..start + count].copy_from_slice(&batch[..count]);
        } else {
            let batch =
                crate::arch::blake3_four(bytes, counter + start as u64).expect("available backend");
            cvs[start..start + count].copy_from_slice(&batch[..count]);
        }
    }
    let lanes = lanes.min(8);
    let mut count = data.len().div_ceil(CHUNK);
    while count > 2 {
        let pairs = count / 2;
        let mut i = 0;
        #[cfg(target_arch = "x86_64")]
        if backend == Backend::Avx512 {
            while i + 16 <= pairs {
                let parents = crate::arch::blake3_x86::parents16(
                    cvs[2 * i..2 * i + 32].try_into().expect("32 child CVs"),
                )
                .expect("available backend");
                cvs[i..i + 16].copy_from_slice(&parents);
                i += 16;
            }
        }
        while i + lanes <= pairs {
            if lanes == 8 {
                let children = cvs[2 * i..2 * i + 16]
                    .try_into()
                    .expect("sixteen child CVs");
                let parents = crate::arch::blake3_parents8(children).expect("available backend");
                cvs[i..i + 8].copy_from_slice(&parents);
            } else {
                let children = cvs[2 * i..2 * i + 8].try_into().expect("eight child CVs");
                let parents = crate::arch::blake3_parents4(children).expect("available backend");
                cvs[i..i + 4].copy_from_slice(&parents);
            }
            i += lanes;
        }
        // The next tree level may fill four lanes after an eight-lane
        // reduction. Use the narrower batch before falling back to individual
        // parent compressions.
        while i + 4 <= pairs {
            let children = cvs[2 * i..2 * i + 8].try_into().expect("eight child CVs");
            #[cfg(target_arch = "x86_64")]
            let wide = if backend == Backend::Avx512 {
                crate::arch::blake3_parents4_avx512(children)
            } else {
                None
            };
            #[cfg(not(target_arch = "x86_64"))]
            let wide = None;
            let parents = wide
                .or_else(|| crate::arch::blake3_parents4(children))
                .expect("available backend");
            cvs[i..i + 4].copy_from_slice(&parents);
            i += 4;
        }
        #[cfg(target_arch = "x86_64")]
        if backend == Backend::Avx512 && i + 2 <= pairs {
            let children = cvs[2 * i..2 * i + 4].try_into().expect("four child CVs");
            let parents = crate::arch::blake3_parents2_avx512(children).expect("available backend");
            cvs[i..i + 2].copy_from_slice(&parents);
            i += 2;
        }
        while i < pairs {
            let short = if cfg!(any(target_arch = "x86", target_arch = "x86_64")) {
                Backend::Portable
            } else {
                backend
            };
            cvs[i] = parent(cvs[2 * i], cvs[2 * i + 1]).chaining(short);
            i += 1;
        }
        if !count.is_multiple_of(2) {
            cvs[pairs] = cvs[count - 1];
        }
        count = count.div_ceil(2);
    }
    parent(cvs[0], cvs[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writer_rejects_length_overflow_without_mutation() {
        use std::io::Write as _;
        let mut stream = Hasher::new();
        stream.update(b"prefix");
        stream.length = u64::MAX;
        assert_eq!(
            stream.write(b"x").expect_err("length overflow").kind(),
            std::io::ErrorKind::InvalidInput
        );
        assert_eq!(stream.filled, 6);
        assert_eq!(&stream.buffer[..6], b"prefix");
        assert_eq!(stream.length, u64::MAX);
        assert_eq!(stream.write(&[]).expect("empty update fits"), 0);
    }

    #[test]
    fn narrow_single_blocks_match_portable_arithmetic() {
        for counter in [0, 1, u64::from(u32::MAX), (1u64 << 54) - 1] {
            for len in 0..=64 {
                for flags in [
                    0,
                    START,
                    END,
                    START | END,
                    START | END | ROOT,
                    PARENT,
                    PARENT | ROOT,
                ] {
                    let cv = core::array::from_fn(|i| IV[i] ^ (len * 0x10203));
                    let words =
                        core::array::from_fn(|i| (i as u32).wrapping_mul(0x9e37_79b9) ^ len);
                    if let Some(actual) =
                        crate::arch::blake3_compress4(cv, &words, counter, len, flags)
                    {
                        assert_eq!(actual, compress_portable(cv, &words, counter, len, flags));
                    }
                }
            }
        }
    }

    #[test]
    fn narrow_simd_lanes_preserve_counters_and_tails() {
        let data: Vec<u8> = (0..8256).map(|i| ((i * 13 + i / 17) % 251) as u8).collect();
        for counter in [
            0,
            1,
            u64::from(u32::MAX) - 1,
            u64::from(u32::MAX),
            (1u64 << 54) - 16,
        ] {
            for offset in 0..32 {
                for len in [
                    1, 63, 64, 65, 1023, 1024, 1025, 2047, 2048, 2049, 3072, 4095, 4096, 4097,
                    8191, 8192,
                ] {
                    let bytes = &data[offset..offset + len];
                    let expected: Vec<_> = bytes
                        .chunks(CHUNK)
                        .enumerate()
                        .map(|(i, b)| {
                            chunk(b, counter + i as u64, Backend::Portable)
                                .chaining(Backend::Portable)
                        })
                        .collect();
                    if let Some(cvs) = crate::arch::blake3_four(bytes, counter) {
                        assert_eq!(
                            &cvs[..expected.len()],
                            expected.as_slice(),
                            "four lanes counter={counter}, offset={offset}, len={len}"
                        );
                    }
                    #[cfg(target_arch = "x86_64")]
                    if let Ok(full) = <&[u8; 4096]>::try_from(bytes)
                        && let Some(parents) = crate::arch::blake3_subtree_four(full, counter)
                    {
                        assert_eq!(
                            parents,
                            [
                                parent(expected[0], expected[1]).chaining(Backend::Portable),
                                parent(expected[2], expected[3]).chaining(Backend::Portable),
                            ],
                            "four-chunk parents counter={counter}, offset={offset}"
                        );
                    }
                    if let Some(cvs) = crate::arch::blake3_eight(bytes, counter) {
                        assert_eq!(
                            &cvs[..expected.len()],
                            expected.as_slice(),
                            "eight lanes counter={counter}, offset={offset}, len={len}"
                        );
                    }
                }
            }
        }
    }
}
