// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Immutable owned-or-borrowed storage without a cloning or allocation contract.

/// Store an owned value inline or borrow the caller's exact immutable value.
///
/// Unlike copy-on-write storage this requires neither Clone nor ToOwned and
/// exposes no mutable dereference. Moving/dropping an owned carrier has normal
/// Rust ownership semantics; dropping a borrowed carrier leaves its owner alone.
#[derive(Debug)]
pub enum OwnedOrBorrowed<'a, T> {
    /// Own the original value inline, without a new box or clone.
    Owned(T),
    /// Borrow the original value for its admitted lifetime.
    Borrowed(&'a T),
}

impl<T> std::ops::Deref for OwnedOrBorrowed<'_, T> {
    type Target = T;

    /// Read the original immutable payload through either storage choice.
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(value) => value,
            Self::Borrowed(value) => value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OwnedOrBorrowed;
    use std::cell::Cell;

    /// Intentionally non-Clone; the storage must never demand a copy operation.
    #[derive(Debug)]
    struct Payload<'a> {
        drops: &'a Cell<usize>,
        value: usize,
    }
    impl Drop for Payload<'_> {
        /// Count destruction of the original payload independently of its carrier.
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }

    /// Moving the inline non-Clone payload transfers its single drop obligation.
    #[test]
    fn owned_nonclone_payload_moves_and_drops_once() {
        let drops = Cell::new(0);
        let first = OwnedOrBorrowed::Owned(Payload {
            drops: &drops,
            value: 7,
        });
        let moved = first;
        assert_eq!(moved.value, 7);
        assert_eq!(drops.get(), 0);
        drop(moved);
        assert_eq!(drops.get(), 1);
    }

    /// Borrowing preserves pointer identity and leaves destruction to the owner.
    #[test]
    fn borrowed_nonclone_payload_is_the_original_and_outlives_the_carrier() {
        let drops = Cell::new(0);
        let value = Payload {
            drops: &drops,
            value: 11,
        };
        let borrowed = OwnedOrBorrowed::Borrowed(&value);
        assert!(std::ptr::eq(&raw const *borrowed, &raw const value));
        assert_eq!(borrowed.value, 11);
        drop(borrowed);
        assert_eq!(drops.get(), 0);
        assert_eq!(value.value, 11);
        drop(value);
        assert_eq!(drops.get(), 1);
    }
}
