// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A panic poisons the instance before its trap unwinds.
//!
//! On `wasm32-unknown-unknown` a panic aborts: once the panic hook returns, the module
//! executes `unreachable`, and the trap unwinds every wasm frame of the call to the
//! JavaScript that made it. Nothing on the way restores anything. A `RefCell` a frame
//! had borrowed stays borrowed, a mutation the panic interrupted stays half-applied, and
//! the shadow-stack pointer stays wherever the panicking frame left it. A call that
//! followed would run on that state — whether the panic came out of an asynchronous job
//! or out of an ordinary synchronous export.
//!
//! So the hook [`install`] sets reports the panic to the asynchronous runtime
//! (`purrdf_jspi_panicked` in `js/src/purrdf_jspi.mjs`) before anything else, and the
//! runtime poisons the instance exactly as it does for a trapped job: every entry point
//! of the package refuses from then on, with an error that names the panic. The hook
//! chains the one it replaces, so a host-side panic hook keeps working.
//!
//! The hook runs in the middle of a failure, so it does as little as it can: it formats
//! the panic's location and message into a fixed-size buffer on the stack (truncating a
//! longer message on a character boundary), and makes one call into JavaScript that
//! reads that buffer and never calls back into the instance.
//!
//! A trap that is not a panic is not seen here. Allocation failure aborts without
//! running the panic hook (only an unstable toolchain flag turns it into a panic), and an
//! explicit `unreachable` raises its trap directly. The JavaScript runtime catches those
//! instead: every exported function the glue calls goes through a thin guard
//! (`purrdf_jspi_bind_glue`) that poisons the instance when a `WebAssembly.RuntimeError`
//! escapes the export, naming the trap. A panic reaches that guard too, as the trap that
//! follows its hook; the instance is already poisoned by then, and the guard keeps the
//! panic's reason.

#![cfg_attr(
    not(target_arch = "wasm32"),
    allow(
        dead_code,
        reason = "the hook is installed only on wasm32; the native build unit-tests its formatting"
    )
)]

use core::fmt::{self, Write as _};
use core::panic::Location;

/// The JavaScript global that arms `__purrdf_test_panic`: the test export panics only
/// while this property of `globalThis` is `true`.
pub(crate) const TEST_PANIC_FLAG: &str = "__purrdfArmTestPanic";

/// The JavaScript global that arms `__purrdf_test_trap`: the test export executes
/// `unreachable` only while this property of `globalThis` is `true`.
pub(crate) const TEST_TRAP_FLAG: &str = "__purrdfArmTestTrap";

/// The most bytes of a panic's description the poison error carries.
const REASON_CAPACITY: usize = 1024;

/// Appended to a description cut short at [`REASON_CAPACITY`].
const TRUNCATED: &str = "…";

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "./purrdf_jspi.mjs")]
unsafe extern "C" {
    /// Poison the instance, naming the panic: `reason` points at `len` bytes of UTF-8.
    /// Reads the bytes, never calls back into the instance, and never throws.
    fn purrdf_jspi_panicked(reason: *const u8, len: usize);
}

/// Install the poisoning panic hook, chaining the hook it replaces. Called once, from the
/// module's start function.
#[cfg(target_arch = "wasm32")]
pub(crate) fn install() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let mut reason = PanicReason::new();
        describe(info.location(), info.payload_as_str(), &mut reason);
        let text = reason.as_str();
        // SAFETY: `text` is initialized UTF-8 that stays live for the whole call; the
        // import only reads it.
        unsafe { purrdf_jspi_panicked(text.as_ptr(), text.len()) };
        previous(info);
    }));
}

/// Write a panic's description into `reason`: `panicked at <file>:<line>:<column>:
/// <message>`, with either part omitted when the panic does not carry it.
fn describe(location: Option<&Location<'_>>, message: Option<&str>, reason: &mut PanicReason) {
    // `PanicReason` never fails a write: it truncates instead.
    let _ = match (location, message) {
        (Some(location), Some(message)) => write!(reason, "panicked at {location}: {message}"),
        (Some(location), None) => {
            write!(reason, "panicked at {location} with a non-string payload")
        }
        (None, Some(message)) => write!(reason, "panicked: {message}"),
        (None, None) => write!(reason, "panicked with a non-string payload"),
    };
}

/// A panic's description, held in a fixed-size buffer: writing never allocates, and text
/// past the capacity is dropped on a character boundary and marked with [`TRUNCATED`].
struct PanicReason {
    bytes: [u8; REASON_CAPACITY],
    len: usize,
    truncated: bool,
}

impl PanicReason {
    const fn new() -> Self {
        Self {
            bytes: [0; REASON_CAPACITY],
            len: 0,
            truncated: false,
        }
    }

    /// The description written so far, with the truncation mark when text was dropped.
    fn as_str(&mut self) -> &str {
        if self.truncated {
            // Room for the mark was held back from every write.
            let end = self.len + TRUNCATED.len();
            self.bytes[self.len..end].copy_from_slice(TRUNCATED.as_bytes());
            self.len = end;
            self.truncated = false;
        }
        // Every write stopped on a character boundary, so the bytes are UTF-8.
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or_default()
    }
}

impl fmt::Write for PanicReason {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if self.truncated {
            return Ok(());
        }
        let room = REASON_CAPACITY - TRUNCATED.len() - self.len;
        let mut take = text.len().min(room);
        while !text.is_char_boundary(take) {
            take -= 1;
        }
        self.bytes[self.len..self.len + take].copy_from_slice(&text.as_bytes()[..take]);
        self.len += take;
        self.truncated = take < text.len();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn described(location: Option<&Location<'_>>, message: Option<&str>) -> String {
        let mut reason = PanicReason::new();
        describe(location, message, &mut reason);
        reason.as_str().to_owned()
    }

    #[test]
    fn a_panic_is_described_by_its_location_and_message() {
        let here = Location::caller();
        assert_eq!(
            described(Some(here), Some("the example.org fixture broke")),
            format!("panicked at {here}: the example.org fixture broke")
        );
        assert_eq!(
            described(Some(here), None),
            format!("panicked at {here} with a non-string payload")
        );
        assert_eq!(described(None, Some("boom")), "panicked: boom");
        assert_eq!(described(None, None), "panicked with a non-string payload");
    }

    #[test]
    fn a_description_that_fits_is_kept_whole() {
        // The longest message that fits exactly: nothing dropped, no mark.
        let prefix = "panicked: ";
        let fits = "a".repeat(REASON_CAPACITY - TRUNCATED.len() - prefix.len());
        assert_eq!(described(None, Some(&fits)), format!("{prefix}{fits}"));
    }

    #[test]
    fn a_long_description_is_cut_on_a_character_boundary_and_marked() {
        // One byte more than fits, as a two-byte character straddling the limit: the
        // whole character is dropped, never half of it.
        let prefix = "panicked: ";
        let room = REASON_CAPACITY - TRUNCATED.len() - prefix.len();
        let message = format!("{}é", "a".repeat(room - 1));
        let text = described(None, Some(&message));
        assert_eq!(text, format!("{prefix}{}{TRUNCATED}", "a".repeat(room - 1)));
        assert!(text.len() <= REASON_CAPACITY);
        // Much longer text is cut to the same bound, and later writes add nothing.
        let text = described(
            Some(Location::caller()),
            Some(&"é".repeat(4 * REASON_CAPACITY)),
        );
        assert!(text.ends_with(TRUNCATED));
        assert!(text.len() <= REASON_CAPACITY);
        assert!(text.len() > REASON_CAPACITY - TRUNCATED.len() - 2);
    }
}
