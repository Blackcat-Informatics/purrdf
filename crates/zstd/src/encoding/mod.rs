#![allow(
    clippy::unreadable_literal,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Zstandard encoder — frame compression, streaming, dictionary support.
//!
//! Four entry points cover the common use cases:
//!
//! * [`compress`] — one-shot helper that builds a self-contained
//!   Zstandard frame from a `Read` source to a `Write` sink. The
//!   input is consumed incrementally from `Read`, so input buffering
//!   stays bounded; however, the compressed output is buffered in
//!   memory until the frame is complete so the Frame Content Size
//!   field can be filled in the header — peak memory is
//!   `O(compressed_size)` (worst-case `O(input_size)` for
//!   incompressible payloads, plus a small frame overhead). The
//!   savings vs [`compress_to_vec`] come from not materialising the
//!   input alongside the output.
//! * [`compress_to_vec`] — same one-shot path as [`compress`] but
//!   the input is eagerly drained into an internal `Vec` first
//!   (`read_to_end`) so the encoder can be handed a `&[u8]` and a
//!   precise source-size hint. Peak memory is therefore ≈
//!   `input_size + output_size`; prefer [`compress`] or
//!   [`StreamingEncoder`] when the input is large or unbounded.
//! * [`StreamingEncoder`] — implements [`crate::io::Write`], which
//!   re-exports [`std::io::Write`] under the `std` feature and falls
//!   back to a `no_std`-friendly trait otherwise. Accepts bytes
//!   incrementally and flushes compressed output as blocks fill.
//!   Requires `set_pledged_content_size` before the first write if
//!   the Frame Content Size field is to be populated.
//! * [`FrameCompressor`] — lower-level builder that owns the matcher and
//!   the per-frame configuration; the streaming and one-shot helpers are
//!   thin wrappers over it. Reach for it when you need to swap in a custom
//!   [`Matcher`] implementation or share the matcher across frames.
//!
//! Compression intensity is selected via [`CompressionLevel`], which
//! provides both named presets (`Fastest`, `Default`, `Better`, `Best`) and
//! numeric levels (`from_level(n)`) that mirror C zstd's level numbering
//! (negative for ultra-fast, `0` = default, `1..=22` for the standard
//! range).
//!
//! All produced frames are valid RFC 8878 Zstandard streams and decode
//! through both this crate's [`crate::decoding`] module and upstream C zstd.
//!
//! For memory budgeting, [`estimated_compression_workspace_bytes`] reports
//! the approximate steady-state heap footprint of a one-shot compression at
//! a given level (window + match-finder tables + block staging).

pub(crate) mod block_header;
pub(crate) mod blocks;
pub(crate) mod cparams;
pub(crate) mod dict_attach;
pub(crate) mod fastpath;
pub(crate) mod frame_header;
pub(crate) mod incompressible;
pub(crate) mod match_generator;
pub(crate) mod util;

// `#111` encoder architecture rewrite. `cost_model`, `opt`,
// `strategy`, `dfast`, `row`, and `simple` host the relocated
// cost-model types, the optimal-parser plain-data types, the
// const-generic [`strategy::Strategy`] trait + per-level [`strategy::
// StrategyTag`] dispatcher, and the Dfast / Row / Simple matchers
// respectively. `match_table::helpers` hosts the shared match-finder
// primitives. The rewrite plan is tracked in
// <https://github.com/structured-world/structured-zstd/issues/111>;
// per-phase boundaries are `perf/post-pr-110-baseline` (start),
// `perf/post-pr-121-baseline` (post-Phase-2).
pub(crate) mod bt;
pub(crate) mod cost_model;
pub(crate) mod dfast;
pub(crate) mod hc;
// LDM uses `twox_hash::XxHash64` (per-window XXH64 over the
// `min_match_length` byte slice, upstream zstd `zstd_ldm.c:315`). The
// `twox-hash` dependency is gated behind the `hash` feature so
pub(crate) mod lazy_parse;
// `default-features = false` builds (no_std, embedded) don't pull
// it in. `BtMatcher::ldm_producer` and the `cfg(all())`
// blocks inside `BtMatcher::prepare_ldm_candidates` /
// `BtMatcher::reset` carry the same gate; the call site in
// `match_generator.rs::start_matching_optimal` invokes
// `prepare_ldm_candidates` unconditionally because the
// gating is internal to the method body (under
// `not(all())` the method shrinks to the legacy
// `ldm_sequences.clear()` stub).

pub(crate) mod ldm;
pub(crate) mod match_table;
pub(crate) mod opt;
pub(crate) mod row;
pub(crate) mod simple;
pub(crate) mod strategy;

pub(crate) mod frame_compressor;

mod levels;
pub(crate) mod parameters;

mod streaming_encoder;
pub use frame_compressor::{EncoderDictionary, FrameCompressor};

pub use levels::config::{
    estimated_bt_strategy_extra_bytes, estimated_compression_workspace_bytes,
};
pub use match_generator::MatchGeneratorDriver;
pub use parameters::{
    Bounds, CParameter, CompressionParameters, CompressionParametersBuilder, ParameterError,
    Strategy,
};
pub use streaming_encoder::StreamingEncoder;

use crate::io::{Read, Write};
use alloc::vec::Vec;

/// Convenience function to compress some source into a target without reusing any resources of the compressor
/// ```rust
/// use purrdf_zstd::encoding::{compress, CompressionLevel};
/// let data: &[u8] = &[0,0,0,0,0,0,0,0,0,0,0,0];
/// let mut target = Vec::new();
/// compress(data, &mut target, CompressionLevel::Fastest);
/// ```
pub fn compress<R: Read, W: Write>(source: R, target: W, level: CompressionLevel) {
    let mut frame_enc = FrameCompressor::new(level);
    frame_enc.set_source(source);
    frame_enc.set_drain(target);
    frame_enc.compress();
}

/// Convenience function to compress some source into a Vec without reusing any resources of the compressor.
///
/// This helper eagerly buffers the full input (`Read`) before compression so it
/// can provide a source-size hint to the one-shot encoder path. Peak memory can
/// therefore be roughly `input_size + output_size`. For very large payloads or
/// tighter memory budgets, prefer streaming APIs such as [`StreamingEncoder`].
///
/// **This is NOT a streaming API.** The source is fully buffered
/// into a `Vec<u8>` before any compression work begins, so peak input
/// memory is bounded by `source.len()` (not "constant regardless of
/// payload size" as a stream-shaped encoder would offer). If the
/// source is large enough that holding it in memory is not acceptable,
/// use [`StreamingEncoder`] which consumes chunks incrementally
/// without the up-front Vec build.
///
/// This helper drives `read_to_end` to materialize the full source
/// into a `Vec<u8>` before forwarding the slice to
/// [`compress_slice_to_vec`]. For a `Read` whose size is unknown ahead
/// of time, `read_to_end` grows that input `Vec` via power-of-two
/// doubling: peak input allocation can be up to 2× the final source
/// length transiently. The live working set on this entry point is
/// roughly `input.capacity()` plus the block-accumulation buffer and
/// per-block scratch carried by [`compress_slice_to_vec`], plus the
/// exactly-sized output `Vec`. [`StreamingEncoder`] avoids the input
/// materialization step entirely and is the right entry point when
/// the source is large or unbounded.
///
/// ```rust
/// use purrdf_zstd::encoding::{compress_to_vec, CompressionLevel};
/// let data: &[u8] = &[0,0,0,0,0,0,0,0,0,0,0,0];
/// let compressed = compress_to_vec(data, CompressionLevel::Fastest);
/// ```
pub fn compress_to_vec<R: Read>(source: R, level: CompressionLevel) -> Vec<u8> {
    let mut source = source;
    let mut input = Vec::new();
    source.read_to_end(&mut input).unwrap();
    compress_slice_to_vec(input.as_slice(), level)
}

/// Compress a contiguous byte slice into a fresh `Vec<u8>` without the
/// input-buffering step that [`compress_to_vec`] performs to adapt a
/// `Read` source.
///
/// One-shot wrapper over
/// [`FrameCompressor::compress_independent_frame`]: the input is read by
/// reference (the eligible Fast path scans it in place, no per-block
/// history copy), and the returned `Vec` is allocated exactly once at the
/// final frame size after compression. Peak transient memory is the
/// block-accumulation buffer (grown via amortized doubling, ≈ 2× current
/// compressed size at the last realloc) plus the exactly-sized output. The
/// worst-case compressed-size bound is never pinned upfront, so a highly
/// compressible 100 MiB input does not charge ~100 MiB of worst-case
/// expansion against peak.
///
/// To compress many slices, construct one [`FrameCompressor`] and call
/// [`compress_independent_frame_into`](FrameCompressor::compress_independent_frame_into)
/// in a loop instead, which reuses the matcher tables, scratch, and output
/// buffer across frames (this function allocates and primes from scratch
/// each call).
///
/// # Panics
///
/// Panics on encoder error (matches the failure surface of
/// [`compress_to_vec`], which this function backs). Out-of-memory during
/// the output / per-block scratch allocations is handled by the global
/// allocator's abort policy. The slice/Vec entry points mirror the upstream zstd
/// `ZSTD_compress` shape (no error return on the bulk path).
///
/// ```rust
/// use purrdf_zstd::encoding::{compress_slice_to_vec, CompressionLevel};
/// let data: &[u8] = &[0,0,0,0,0,0,0,0,0,0,0,0];
/// let compressed = compress_slice_to_vec(data, CompressionLevel::Fastest);
/// ```
pub fn compress_slice_to_vec(source: &[u8], level: CompressionLevel) -> Vec<u8> {
    // Bare `FrameCompressor` resolves all three type params to their
    // defaults (`&'static [u8]` reader, `Vec<u8>` drain, MatchGeneratorDriver);
    // neither the reader nor the drain is used by the in-place
    // `compress_independent_frame` path.
    let mut enc: FrameCompressor = FrameCompressor::new(level);
    enc.compress_independent_frame(source)
}

/// Worst-case compressed-frame size for an input of `src_size` bytes.
///
/// A destination buffer of this size is always large enough to hold the
/// output of [`compress_slice_to_vec`] (or any single-frame compression) for
/// an input of `src_size` bytes, so a caller sizing a fixed buffer once (the
/// shape the C `ZSTD_compress` entry point needs) never has to grow it.
///
/// Mirrors the upstream `ZSTD_COMPRESSBOUND` formula exactly:
/// `src_size + (src_size >> 8) + margin`, where `margin` is
/// `(128 KiB - src_size) >> 11` for inputs below 128 KiB and `0` otherwise.
/// The margin guarantees `bound(a) + bound(b) <= bound(a + b)` for blocks of
/// at least 128 KiB, which keeps multi-frame concatenation sizing sound.
///
/// Saturates at [`usize::MAX`] if the formula would overflow on a
/// pathologically large `src_size` — no allocation that large can exist, so
/// the saturated value is the correct "cannot fit" sentinel rather than a
/// masked wrap.
///
/// ```rust
/// use purrdf_zstd::encoding::{compress_bound, compress_slice_to_vec, CompressionLevel};
/// let data = [7u8; 4096];
/// assert!(compress_slice_to_vec(&data, CompressionLevel::Default).len() <= compress_bound(data.len()));
/// ```
pub const fn compress_bound(src_size: usize) -> usize {
    const LOWER: usize = 128 * 1024;
    let margin = if src_size < LOWER {
        (LOWER - src_size) >> 11
    } else {
        0
    };
    // Saturating is the correct UPPER-BOUND semantic here, not a masked bug:
    // this is a public API over an arbitrary `usize`, and the largest meaningful
    // bound is `usize::MAX`. A real slice is at most `isize::MAX` bytes, so the
    // `* 1.004 + margin` cannot overflow for genuine inputs; the saturation only
    // caps a pathological caller-supplied size at the representable ceiling.
    src_size
        .saturating_add(src_size >> 8)
        .saturating_add(margin)
}

/// Compress a byte slice into a fresh `Vec<u8>` using fine-grained
/// [`CompressionParameters`] (#27) instead of a bare
/// [`CompressionLevel`].
///
/// One-shot wrapper over [`FrameCompressor::set_parameters`] +
/// [`FrameCompressor::compress_independent_frame`]. The produced frame is
/// a valid RFC 8878 stream regardless of the knobs chosen.
///
/// ```rust
/// use purrdf_zstd::encoding::{
///     compress_with_parameters, CompressionLevel, CompressionParameters, Strategy,
/// };
/// let data: &[u8] = b"the quick brown fox jumps over the lazy dog";
/// let params = CompressionParameters::builder(CompressionLevel::Level(5))
///     .strategy(Strategy::Greedy)
///     .build()
///     .unwrap();
/// let compressed = compress_with_parameters(data, &params);
/// assert!(!compressed.is_empty());
/// ```
pub fn compress_with_parameters(source: &[u8], params: &CompressionParameters) -> Vec<u8> {
    let mut enc: FrameCompressor = FrameCompressor::new(params.level());
    enc.set_parameters(params);
    enc.compress_independent_frame(source)
}

/// The compression mode used impacts the speed of compression,
/// and resulting compression ratios. Faster compression will result
/// in worse compression ratios, and vice versa.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CompressionLevel {
    /// This level does not compress the data at all, and simply wraps
    /// it in a Zstandard frame.
    Uncompressed,
    /// This level is roughly equivalent to Zstd compression level 1
    Fastest,
    /// This level uses the crate's dedicated `dfast`-style matcher to
    /// target a better speed/ratio tradeoff than [`CompressionLevel::Fastest`].
    ///
    /// It represents this crate's "default" compression setting and may
    /// evolve in future versions as the implementation moves closer to
    /// reference zstd level 3 behavior.
    Default,
    /// This level is roughly equivalent to Zstd level 7.
    ///
    /// Uses the hash-chain matcher with a lazy2 matching strategy: the encoder
    /// evaluates up to two positions ahead before committing to a match,
    /// trading speed for a better compression ratio than [`CompressionLevel::Default`].
    Better,
    /// This level is roughly equivalent to Zstd level 11.
    ///
    /// Uses the hash-chain matcher with a deep lazy2 matching strategy and
    /// a 16 MiB window. Compared to [`CompressionLevel::Better`], this level
    /// uses larger hash and chain tables (2 M / 1 M entries vs 1 M / 512 K),
    /// a deeper search (32 candidates vs 16), and a higher target match
    /// length (128 vs 48), trading speed for the best compression ratio
    /// available in this crate.
    Best,
    /// Numeric compression level.
    ///
    /// Levels 1–22 correspond to the C zstd level numbering.  Higher values
    /// produce smaller output at the cost of more CPU time.  Negative values
    /// select ultra-fast modes that trade ratio for speed.  Level 0 is
    /// treated as [`DEFAULT_LEVEL`](Self::DEFAULT_LEVEL), matching C zstd
    /// semantics.
    ///
    /// Named variants map to specific numeric levels:
    /// [`Fastest`](Self::Fastest) = 1, [`Default`](Self::Default) = 3,
    /// [`Better`](Self::Better) = 7, [`Best`](Self::Best) = 11.
    /// [`Best`](Self::Best) remains the highest-ratio named preset, but
    /// [`Level`](Self::Level) values above 11 can target stronger (slower)
    /// tuning than the named hierarchy.
    ///
    /// Levels above 11 use progressively larger windows and deeper search.
    /// Levels 16–17 use a `btopt`-style price parser, 18–19 use `btultra`,
    /// and 20–22 use a `btultra2`-style two-pass selection profile.
    ///
    /// Semver note: this variant was added after the initial enum shape and
    /// is a breaking API change for downstream crates that exhaustively
    /// `match` on [`CompressionLevel`] without a wildcard arm.
    Level(i32),
}

impl CompressionLevel {
    /// The minimum supported numeric compression level (ultra-fast mode).
    pub const MIN_LEVEL: i32 = -131072;
    /// The maximum supported numeric compression level.
    pub const MAX_LEVEL: i32 = 22;
    /// The default numeric compression level (equivalent to [`Default`](Self::Default)).
    pub const DEFAULT_LEVEL: i32 = 3;

    /// Create a compression level from a numeric value.
    ///
    /// Returns named variants for canonical levels (`0`/`3`, `1`, `7`, `11`)
    /// and [`Level`](Self::Level) for all other values.
    ///
    /// With the default matcher backend (`MatchGeneratorDriver`), values
    /// outside [`MIN_LEVEL`](Self::MIN_LEVEL)..=[`MAX_LEVEL`](Self::MAX_LEVEL)
    /// are silently clamped during built-in level parameter resolution.
    pub const fn from_level(level: i32) -> Self {
        match level {
            0 | Self::DEFAULT_LEVEL => Self::Default,
            1 => Self::Fastest,
            7 => Self::Better,
            11 => Self::Best,
            _ => Self::Level(level),
        }
    }
}

/// Trait used by the encoder that users can use to extend the matching facilities with their own algorithm
/// making their own tradeoffs between runtime, memory usage and compression ratio
///
/// This trait operates on buffers that represent the chunks of data the matching algorithm wants to work on.
/// Each one of these buffers is referred to as a *space*. One or more of these buffers represent the window
/// the decoder will need to decode the data again.
///
/// This library asks the Matcher for a new buffer using `get_next_space` to allow reusing of allocated buffers when they are no longer part of the
/// window of data that is being used for matching.
///
/// The library fills the buffer with data that is to be compressed and commits them back to the matcher using `commit_space`.
///
/// Then it will either call `start_matching` or, if the space is deemed not worth compressing, `skip_matching` is called.
///
/// This is repeated until no more data is left to be compressed.
pub trait Matcher {
    /// Get a space where we can put data to be matched on. Will be encoded as one block. The maximum allowed size is 128 kB.
    fn get_next_space(&mut self) -> Vec<u8>;
    /// Get a reference to the last committed space
    fn get_last_space(&mut self) -> &[u8];
    /// Commit a space to the matcher so it can be matched against
    fn commit_space(&mut self, space: Vec<u8>);
    /// Just process the data in the last committed space for future matching.
    fn skip_matching(&mut self);
    /// Hint-aware skip path used internally to thread a precomputed block
    /// incompressibility verdict to matcher backends.
    ///
    /// Default implementation preserves backwards compatibility for external
    /// custom matchers by delegating to [`skip_matching`](Self::skip_matching).
    fn skip_matching_with_hint(&mut self, _incompressible_hint: Option<bool>) {
        self.skip_matching();
    }
    /// Process the data in the last committed space for future matching AND generate matches for the data
    fn start_matching(&mut self, handle_sequence: impl for<'a> FnMut(Sequence<'a>));
    /// Reset this matcher so it can be used for the next new frame
    fn reset(&mut self, level: CompressionLevel);
    /// Provide a hint about the total uncompressed size for the next frame.
    ///
    /// Implementations may use this to select smaller hash tables and windows
    /// for small inputs, matching the C zstd source-size-class behavior.
    /// Called before [`reset`](Self::reset) when the caller knows the input
    /// size (e.g. from pledged content size or file metadata).
    ///
    /// The default implementation is a no-op for custom matchers and
    /// test stubs. The built-in runtime matcher (`MatchGeneratorDriver`)
    /// overrides this hook and applies the hint during level resolution.
    fn set_source_size_hint(&mut self, _size: u64) {}
    /// Hint the byte size of the dictionary that will be primed into the next
    /// frame. The built-in runtime matcher uses it to size the binary-tree /
    /// hash-chain match-finder tables from the dictionary's cParams tier rather
    /// than the source window (upstream zstd CDict economics), while keeping the
    /// eviction window source-sized. Default no-op for custom matchers and test
    /// stubs; consumed at the next [`reset`](Self::reset).
    fn set_dictionary_size_hint(&mut self, _size: usize) {}
    /// Drop any per-frame fine-grained parameter overrides installed via
    /// the public parameter API, reverting to plain level-based geometry
    /// at the next [`reset`](Self::reset). Called by
    /// [`FrameCompressor::set_compression_level`](crate::encoding::FrameCompressor::set_compression_level)
    /// so switching back to a bare level after a customized frame does not
    /// keep the old overrides sticky. Default no-op for custom matchers.
    fn clear_param_overrides(&mut self) {}
    /// Prime matcher state with dictionary history before compressing the next frame.
    /// Default implementation is a no-op for custom matchers that do not support this.
    fn prime_with_dictionary(&mut self, _dict_content: &[u8], _offset_hist: [u32; 3]) {}
    /// Whether the most recent [`reset`](Self::reset) re-borrowed a resident
    /// attach-mode dictionary (kept the dict bytes + cached index in place).
    /// When `true` the caller MUST skip [`Self::prime_with_dictionary`] and only
    /// reapply the offset history via [`Self::reapply_resident_dictionary`].
    fn dictionary_is_resident(&self) -> bool {
        false
    }
    /// Reapply the dictionary's offset history to a re-borrowed frame — the cheap
    /// tail of priming, without the dict commit / re-index. Default no-op.
    fn reapply_resident_dictionary(&mut self, _offset_hist: [u32; 3]) {}
    /// CDict-equivalent fast path for repeated frames sharing one dictionary.
    /// Restore the matcher state captured by [`Self::capture_primed_dictionary`]
    /// at the SAME level (a table copy) instead of re-running
    /// [`Self::prime_with_dictionary`] (which re-hashes every dictionary
    /// position). Returns `true` when a matching snapshot was restored;
    /// `false` (the default) means the caller must prime then capture.
    fn restore_primed_dictionary(&mut self, _level: CompressionLevel) -> bool {
        false
    }
    /// Snapshot the post-prime matcher state for the given level so later
    /// frames can [`Self::restore_primed_dictionary`] it. Default no-op.
    fn capture_primed_dictionary(&mut self, _level: CompressionLevel) {}
    /// Drop any captured prime snapshot (dictionary or level changed).
    /// Default no-op.
    fn invalidate_primed_dictionary(&mut self) {}
    /// Seed matcher cost model with dictionary entropy tables before the next frame.
    /// Default implementation is a no-op for custom matchers.
    fn seed_dictionary_entropy(
        &mut self,
        _huff: Option<&crate::huff0::huff0_encoder::HuffmanTable>,
        _ll: Option<&crate::fse::fse_encoder::FSETable>,
        _ml: Option<&crate::fse::fse_encoder::FSETable>,
        _of: Option<&crate::fse::fse_encoder::FSETable>,
    ) {
    }
    /// Returns whether this matcher can consume dictionary priming state and produce
    /// dictionary-dependent sequences. Defaults to `false` for custom matchers.
    fn supports_dictionary_priming(&self) -> bool {
        false
    }
    /// Whether a sample of `block` hashes to a match in an attached dictionary.
    /// The raw-fast-path uses this to avoid skipping the scan on a block that
    /// looks incompressible but compresses against the dictionary (an external
    /// match the block's own content cannot reveal). Defaults to `false` for
    /// custom matchers (and the no-dict case), leaving the content-only verdict.
    fn block_samples_match_dict(&self, _block: &[u8]) -> bool {
        false
    }
    /// Heap bytes this matcher's allocations hold (tables, history, scratch),
    /// excluding the inline struct itself. Lets a context report its true
    /// footprint via `ZSTD_sizeof_CCtx`. Defaults to `0` for custom matchers.
    fn heap_size(&self) -> usize {
        0
    }
    /// The size of the window the decoder will need to execute all sequences produced by this matcher.
    ///
    /// Must return a positive (non-zero) value; returning 0 causes
    /// [`StreamingEncoder`] to reject the first write with an invalid-input error
    /// (`InvalidInput` with `std`, `Other` with `no_std`).
    ///
    /// Must remain stable for the lifetime of a frame.
    /// It may change only after `reset()` is called for the next frame
    /// (for example because the compression level changed).
    fn window_size(&self) -> u64;
}

#[derive(PartialEq, Eq, Debug)]
/// Sequences that a [`Matcher`] can produce
pub enum Sequence<'data> {
    /// Is encoded as a sequence for the decoder sequence execution.
    ///
    /// First the literals will be copied to the decoded data,
    /// then `match_len` bytes are copied from `offset` bytes back in the decoded data
    Triple {
        literals: &'data [u8],
        offset: usize,
        match_len: usize,
    },
    /// This is returned as the last sequence in a block
    ///
    /// These literals will just be copied at the end of the sequence execution by the decoder
    Literals { literals: &'data [u8] },
}
