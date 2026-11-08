// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Refreshing an identity hashes its retained payloads without copying them.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_core::artifact::Identity;
use sha2::{Digest, Sha256};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[test]
fn appending_a_component_does_not_allocate_for_existing_payloads() {
    let mut baseline = None;
    for payload in [1024, 1024 * 1024] {
        let original = vec![0xa5; payload];
        let mut identity = Identity::new().with("retained", &original);
        let window = CurrentThreadWindow::open();
        identity.push("new", b"value");
        let measured = window.close();
        assert_eq!(measured.allocations, 2, "{measured:?}");
        assert_eq!(measured.requested_bytes, 8, "{measured:?}");
        assert_eq!(measured.retained_bytes, 8, "{measured:?}");
        assert_eq!(measured.peak_working_bytes, 8, "{measured:?}");
        if let Some(previous) = baseline {
            assert_eq!(measured, previous);
        }
        baseline = Some(measured);
        assert_eq!(identity.component("retained"), Some(original.as_slice()));
        assert_eq!(identity.component("new"), Some(b"value".as_slice()));
        let bytes = identity.to_bytes();
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        assert_eq!(*identity.digest(), digest);
        assert_eq!(Identity::from_bytes(&bytes).unwrap(), identity);
    }
}
