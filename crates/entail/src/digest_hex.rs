// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! One first-party renderer for "a 32-byte digest as 64 lowercase hex characters".
//!
//! Before this module existed, [`crate::explain`], [`crate::reasoner::proof`] and
//! [`crate::owl_dl::proof`] each carried their own private `fn hex(digest: [u8; 32]) ->
//! String`, byte-for-byte identical in shape (a `char::from_digit` nibble loop) and used
//! for the same purpose in all three: rendering a proof or contract digest for display and
//! for the golden-tested `digest_hex()` accessors. Three copies of one operation is the
//! DUPLICATE shape the workspace's standing goals forbid, so it is consolidated here.
//!
//! The rendering itself is [`purrdf_core::hex::lower`], the workspace's one
//! `&[u8]` → lowercase-hex helper. This module keeps only the digest-shaped signature its
//! three callers want; consolidating three copies into a fourth private implementation
//! would have traded one duplicate for another. This is a one-shot operation (called at
//! most a handful of times per proof, never inside a fixpoint's inner loop), so there is no
//! hot-path reason to prefer a lookup table here the way `purrdf_datalog::chase`'s
//! witness-label renderer does for its own, much hotter, call site.

/// Render a 32-byte digest as 64 lowercase hex characters.
pub(crate) fn hex(digest: [u8; 32]) -> String {
    purrdf_core::hex::lower(&digest)
}

#[cfg(test)]
mod tests {
    use super::hex;

    /// Agrees with a manual per-byte nibble rendering over every byte value, not just the
    /// extremes — the two `u128` halves must not, say, swap order or drop leading zeros.
    #[test]
    fn matches_manual_nibble_rendering() {
        let digest: [u8; 32] = core::array::from_fn(|i| (i * 7) as u8);
        let mut expected = String::with_capacity(64);
        for byte in digest {
            use std::fmt::Write as _;
            write!(expected, "{byte:02x}").expect("writing to a String never fails");
        }
        assert_eq!(hex(digest), expected);
    }

    #[test]
    fn all_zero_bytes_render_as_64_zeros() {
        assert_eq!(hex([0u8; 32]), "0".repeat(64));
    }

    #[test]
    fn all_max_bytes_render_as_64_fs() {
        assert_eq!(hex([0xffu8; 32]), "f".repeat(64));
    }

    /// A leading zero byte must not be dropped — the classic "treat it as an integer and
    /// lose the leading zero" bug this whole module exists to avoid reintroducing.
    #[test]
    fn leading_zero_byte_is_not_dropped() {
        let mut digest = [0xabu8; 32];
        digest[0] = 0x00;
        let rendered = hex(digest);
        assert_eq!(rendered.len(), 64);
        assert!(rendered.starts_with("00"));
    }
}
