// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The module's own shadow stack, as `wasm-ld` laid it out.
//!
//! Every `wasm32` target rustc ships links with `--stack-first`: the shadow stack is the
//! first region of linear memory, `[__stack_low, __stack_high)`; the stack pointer starts
//! at `__stack_high` and grows down; the static data, closed by `__data_end`, and the
//! heap, opened by `__heap_base`, lie above it. The linker records the four boundaries as
//! data symbols whose *addresses* are the values. Only the addresses are ever taken here;
//! the bytes at them are never read.
//!
//! The asynchronous lane sizes every job's stack region to the shadow stack — the two
//! lanes then run out of stack at the same depth, and a stack refusal on either is the
//! evaluator's own typed error — so it reads the size here ([`region_bytes`]) and refuses
//! to run at all on a module whose layout is not this one: a stack that is empty,
//! misaligned or not below the static data is a module this runtime has no region for.

/// The alignment the shadow-stack pointer keeps: every frame's top and bottom are
/// multiples of it, so a region's base and top must be too.
pub(crate) const STACK_ALIGN: usize = 16;

/// The four boundaries of the module's memory layout, as addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShadowStack {
    /// `__stack_low`: the lowest address a frame may reach.
    pub(crate) low: usize,
    /// `__stack_high`: the stack pointer's initial value, one past the highest byte.
    pub(crate) high: usize,
    /// `__data_end`: one past the last static datum.
    pub(crate) data_end: usize,
    /// `__heap_base`: the first address the allocator may hand out.
    pub(crate) heap_base: usize,
}

impl ShadowStack {
    /// The stack's size, once the layout is the stack-first one this runtime assumes;
    /// otherwise why it is not, as the words of the refusal.
    pub(crate) fn region_bytes(self) -> Result<usize, String> {
        let Self {
            low,
            high,
            data_end,
            heap_base,
        } = self;
        let reason = if low >= high {
            "the shadow stack is empty"
        } else if !low.is_multiple_of(STACK_ALIGN) || !high.is_multiple_of(STACK_ALIGN) {
            "the shadow stack is not 16-byte aligned"
        } else if high > data_end {
            "the shadow stack is not below the static data (the layout is not stack-first)"
        } else if data_end > heap_base {
            "the static data is not below the heap"
        } else {
            return Ok(high - low);
        };
        Err(format!(
            "the module's shadow stack is not laid out as the asynchronous lane assumes \
             (stack [{low:#x}, {high:#x}), data end {data_end:#x}, heap base {heap_base:#x}): \
             {reason}"
        ))
    }
}

#[cfg(target_arch = "wasm32")]
unsafe extern "C" {
    /// The low end of this module's shadow stack, where `wasm-ld` placed it. Only its
    /// address is ever taken; the byte is never read.
    safe static __stack_low: u8;
    /// The high end of the shadow stack: the stack pointer's initial value.
    safe static __stack_high: u8;
    /// One past the last static datum.
    safe static __data_end: u8;
    /// The first address of the heap.
    safe static __heap_base: u8;
}

/// The layout the linker gave this module.
///
/// `black_box`: the low end may be address 0, and nothing may be inferred from an
/// address the compiler assumes is not null.
#[cfg(target_arch = "wasm32")]
pub(crate) fn read() -> ShadowStack {
    ShadowStack {
        low: core::hint::black_box(&raw const __stack_low) as usize,
        high: core::hint::black_box(&raw const __stack_high) as usize,
        data_end: core::hint::black_box(&raw const __data_end) as usize,
        heap_base: core::hint::black_box(&raw const __heap_base) as usize,
    }
}

/// The size of the shadow stack rustc's `wasm32` targets link, which the native build
/// stands in for: it runs no job on a region (nothing switches its stack pointer), so a
/// region there only fixes the top the job's evidence is measured from.
#[cfg(not(target_arch = "wasm32"))]
const NATIVE_STAND_IN_BYTES: usize = 1024 * 1024;

/// The layout the native build stands in for: the stack-first one, with the static data
/// and the heap immediately above the stack.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn read() -> ShadowStack {
    ShadowStack {
        low: 0,
        high: NATIVE_STAND_IN_BYTES,
        data_end: NATIVE_STAND_IN_BYTES,
        heap_base: NATIVE_STAND_IN_BYTES,
    }
}

/// The size of every job's stack region: the module's own shadow stack's, in bytes, or
/// the refusal of a module laid out any other way.
pub(crate) fn region_bytes() -> Result<usize, String> {
    read().region_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The stack-first layout is accepted, its size read off the two stack symbols; and
    /// each way the layout can differ is refused with its reason, the neighbour that
    /// differs by one alignment unit included.
    #[test]
    fn the_stack_first_layout_is_accepted_and_every_other_is_refused() {
        let linked = ShadowStack {
            low: 0,
            high: 1_048_576,
            data_end: 2_112_256,
            heap_base: 2_113_484,
        };
        assert_eq!(linked.region_bytes(), Ok(1_048_576));
        // A stack the linker placed after the data (`--no-stack-first`) is a valid
        // module, and its size reads the same; the data end is then below the stack.
        let stack_last = ShadowStack {
            low: 1_048_576,
            high: 2_097_152,
            data_end: 1_048_576,
            heap_base: 2_097_152,
        };
        let refusal = stack_last.region_bytes().expect_err("stack above the data");
        assert!(refusal.contains("not stack-first"), "{refusal}");
        assert!(refusal.contains("0x100000"), "{refusal}");
        // The layout touching its bounds exactly is the accepted neighbour: the data may
        // start at the stack's top, and the heap at the data's end.
        let tight = ShadowStack {
            low: 0,
            high: 1_048_576,
            data_end: 1_048_576,
            heap_base: 1_048_576,
        };
        assert_eq!(tight.region_bytes(), Ok(1_048_576));
        for (case, layout) in [
            (
                "empty",
                ShadowStack {
                    low: 1_048_576,
                    ..linked
                },
            ),
            (
                "inverted",
                ShadowStack {
                    low: 1_048_577,
                    ..linked
                },
            ),
            ("misaligned low", ShadowStack { low: 8, ..linked }),
            (
                "misaligned high",
                ShadowStack {
                    high: 1_048_576 + 8,
                    ..linked
                },
            ),
            (
                "data below the stack top",
                ShadowStack {
                    data_end: 1_048_576 - 16,
                    ..linked
                },
            ),
            (
                "heap below the data",
                ShadowStack {
                    heap_base: 2_112_256 - 1,
                    ..linked
                },
            ),
        ] {
            let Err(refusal) = layout.region_bytes() else {
                panic!("{case} must be refused")
            };
            assert!(
                refusal.starts_with("the module's shadow stack is not laid out"),
                "{case}: {refusal}"
            );
        }
    }

    /// The build this test runs on has a layout the lane accepts, whatever the target.
    #[test]
    fn this_build_yields_a_region() {
        let bytes = region_bytes().expect("the layout is the stack-first one");
        assert!(bytes > 0 && bytes.is_multiple_of(STACK_ALIGN), "{bytes}");
    }
}
