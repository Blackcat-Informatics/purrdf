// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Dedicated composite fixture keys shared by public integration tests.

use purrdf_gts::cose::composite;

pub(crate) fn composite_key(seed: u8) -> composite::SigningKey {
    let mut seeds = [seed; 64];
    // Distinct component seed inputs are fixture data, not entropy claims.
    seeds[32..].fill(seed + 1);
    let key = composite::SigningKey::from_bytes(&seeds).unwrap();
    purrdf_ed25519::wipe_secret(&mut seeds);
    key
}
