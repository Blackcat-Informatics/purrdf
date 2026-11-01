#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Values and interfaces shared between the encoding side
//! and the decoding side.

// --- FRAMES ---
/// This magic number is included at the start of a single Zstandard frame
pub(crate) const MAGIC_NUM: u32 = 0xFD2F_B528;
/// Window size refers to the minimum amount of memory needed to decode any given frame.
///
/// The minimum window size is defined as 1 KB
pub(crate) const MIN_WINDOW_SIZE: u64 = 1024;
/// Window size refers to the minimum amount of memory needed to decode any given frame.
///
/// The maximum window size allowed by the spec is 3.75TB
pub(crate) const MAX_WINDOW_SIZE: u64 = (1 << 41) + 7 * (1 << 38);

// --- BLOCKS ---
/// While the spec limits block size to 128KB, the implementation uses
/// 128kibibytes
///
/// <https://github.com/facebook/zstd/blob/eca205fc7849a61ab287492931a04960ac58e031/doc/educational_decoder/zstd_decompress.c#L28-L29>
pub(crate) const MAX_BLOCK_SIZE: u32 = 128 * 1024;
/// Smallest accepted block-size target (upstream `ZSTD_TARGETCBLOCKSIZE_MIN`):
/// below this the per-block header overhead dominates any latency benefit.
/// Re-exported at the crate root as the single source of truth; the C ABI
/// parameter bounds import it from there.
pub const MIN_TARGET_BLOCK_SIZE: u32 = 1340;

/// Decoder window-size limit (128 MiB = `1 << 27`), matching upstream zstd's
/// default `ZSTD_d_windowLogMax` (`ZSTD_WINDOWLOG_LIMIT_DEFAULT = 27`). Frames
/// advertising a larger window are rejected to bound allocation on untrusted
/// input. The spec permits larger windows, but no standard zstd encoder emits
/// them by default, so matching the upstream limit keeps the drop-in contract:
/// every frame a stock zstd decoder accepts, we accept too — and crucially,
/// every frame OUR encoder emits (up to `window_log 27` at level 22) round-trips
/// through our own decoder. A non-power-of-two cap below `1 << 27` would reject
/// our own level-22 / streaming output (whose window header carries the full
/// 128 MiB). Decompression-bomb protection lives on the OUTPUT path, not here.
pub(crate) const MAXIMUM_ALLOWED_WINDOW_SIZE: u64 = 1 << 27;
