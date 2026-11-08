#![allow(
    clippy::inline_always,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Shared byte-histogram helpers used by entropy-table builders.
//!
//! Follows the upstream zstd strategy from `zstd/lib/compress/hist.c`:
//! a scalar path for small inputs and a striped counting path for larger inputs.

const PARALLEL_COUNT_THRESHOLD: usize = 1500;
type CountBytesFn = fn(&[u8], &mut [usize; 256]) -> (usize, usize);

#[inline]
fn count_bytes_scalar(data: &[u8], counts: &mut [usize; 256]) -> (usize, usize) {
    counts.fill(0);
    if data.is_empty() {
        return (0, 0);
    }

    let mut max_symbol = 0usize;
    let mut largest_count = 0usize;
    for &byte in data {
        let symbol = byte as usize;
        let next = counts[symbol] + 1;
        counts[symbol] = next;
        max_symbol = max_symbol.max(symbol);
        largest_count = largest_count.max(next);
    }

    (max_symbol, largest_count)
}

#[inline(always)]
fn count_bytes_parallel(data: &[u8], counts: &mut [usize; 256]) -> (usize, usize) {
    if data.len() > u32::MAX as usize {
        // The striped counters are u32-based; preserve correctness for
        // oversized inputs by using the scalar usize accumulator.
        return count_bytes_scalar(data, counts);
    }

    let mut counting1 = [0u32; 256];
    let mut counting2 = [0u32; 256];
    let mut counting3 = [0u32; 256];
    let mut counting4 = [0u32; 256];
    let mut index = 0usize;

    if data.len() >= 16 {
        let end = data.len() - 16;
        while index <= end {
            // SAFETY: loop condition guarantees we can read 16 bytes starting at
            // `index`; read_unaligned matches upstream zstd-style lane loads.
            let ptr = unsafe { data.as_ptr().add(index) };
            let lane0 = u32::from_le(unsafe { core::ptr::read_unaligned(ptr.cast::<u32>()) });
            let lane1 =
                u32::from_le(unsafe { core::ptr::read_unaligned(ptr.add(4).cast::<u32>()) });
            let lane2 =
                u32::from_le(unsafe { core::ptr::read_unaligned(ptr.add(8).cast::<u32>()) });
            let lane3 =
                u32::from_le(unsafe { core::ptr::read_unaligned(ptr.add(12).cast::<u32>()) });
            index += 16;

            counting1[(lane0 & 0xFF) as usize] += 1;
            counting2[((lane0 >> 8) & 0xFF) as usize] += 1;
            counting3[((lane0 >> 16) & 0xFF) as usize] += 1;
            counting4[(lane0 >> 24) as usize] += 1;

            counting1[(lane1 & 0xFF) as usize] += 1;
            counting2[((lane1 >> 8) & 0xFF) as usize] += 1;
            counting3[((lane1 >> 16) & 0xFF) as usize] += 1;
            counting4[(lane1 >> 24) as usize] += 1;

            counting1[(lane2 & 0xFF) as usize] += 1;
            counting2[((lane2 >> 8) & 0xFF) as usize] += 1;
            counting3[((lane2 >> 16) & 0xFF) as usize] += 1;
            counting4[(lane2 >> 24) as usize] += 1;

            counting1[(lane3 & 0xFF) as usize] += 1;
            counting2[((lane3 >> 8) & 0xFF) as usize] += 1;
            counting3[((lane3 >> 16) & 0xFF) as usize] += 1;
            counting4[(lane3 >> 24) as usize] += 1;
        }
    }

    while index < data.len() {
        counting1[data[index] as usize] += 1;
        index += 1;
    }

    let mut max_symbol = 0usize;
    let mut largest_count = 0usize;
    for symbol in 0..256 {
        let value = merge_lane_counts(
            counting1[symbol],
            counting2[symbol],
            counting3[symbol],
            counting4[symbol],
        );
        counts[symbol] = value;
        if value > 0 {
            max_symbol = symbol;
            largest_count = largest_count.max(value);
        }
    }

    (max_symbol, largest_count)
}

#[inline(always)]
fn merge_lane_counts(c1: u32, c2: u32, c3: u32, c4: u32) -> usize {
    #[cfg(target_pointer_width = "64")]
    {
        (c1 as usize) + (c2 as usize) + (c3 as usize) + (c4 as usize)
    }

    #[cfg(target_pointer_width = "32")]
    {
        let sum = u64::from(c1) + u64::from(c2) + u64::from(c3) + u64::from(c4);
        sum as usize
    }
}

/// Counts byte frequencies in `data` and writes them into `counts`.
///
/// Returns `(max_symbol, largest_count)` where:
/// - `max_symbol` is the highest symbol index with non-zero count
/// - `largest_count` is the highest observed frequency
///
/// Uses a scalar path for small buffers and a striped-count path for larger
/// buffers. On AArch64 + `std`, dispatches through a cached SVE2-gated variant when
/// the runtime reports `sve2` support.
pub(crate) fn count_bytes(data: &[u8], counts: &mut [usize; 256]) -> (usize, usize) {
    if data.len() < PARALLEL_COUNT_THRESHOLD {
        return count_bytes_scalar(data, counts);
    }

    count_bytes_dispatch()(data, counts)
}

#[cfg(target_arch = "aarch64")]
#[inline]
fn count_bytes_dispatch() -> CountBytesFn {
    static DISPATCH: std::sync::OnceLock<CountBytesFn> = std::sync::OnceLock::new();

    *DISPATCH.get_or_init(|| {
        if std::arch::is_aarch64_feature_detected!("sve2") {
            count_bytes_sve2_wrapper
        } else {
            count_bytes_parallel
        }
    })
}

#[cfg(not(target_arch = "aarch64"))]
#[inline]
fn count_bytes_dispatch() -> CountBytesFn {
    count_bytes_parallel
}

#[cfg(target_arch = "aarch64")]
#[inline]
fn count_bytes_sve2_wrapper(data: &[u8], counts: &mut [usize; 256]) -> (usize, usize) {
    // SAFETY: dispatch cache selects this only when runtime detection reports
    // SVE2 support for the current process.
    unsafe { count_bytes_sve2(data, counts) }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "sve2")]
unsafe fn count_bytes_sve2(data: &[u8], counts: &mut [usize; 256]) -> (usize, usize) {
    // Rust stable does not expose HISTCNT intrinsics yet, so we keep the same
    // striped algorithm while compiling this variant with SVE2 enabled.
    count_bytes_parallel(data, counts)
}
