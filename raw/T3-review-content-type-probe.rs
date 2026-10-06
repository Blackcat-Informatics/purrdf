// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
use purrdf_gts::{cose::{parse_sign1, verify_signatures}, model::Signature};
use std::cell::Cell;
fn main() {
    for text in ["text/plain", " text/plain", "text/plain ", "invalid", "/plain", "text/", "text/plain/extra", "text/🙂"] {
        // Untagged Sign1 with detached null, protected alg -8 plus content type.
        let mut protected = vec![0xa2, 1, 0x27, 3];
        assert!(text.len() < 24);
        protected.push(0x60 + text.len() as u8);
        protected.extend_from_slice(text.as_bytes());
        assert!(protected.len() < 24);
        let mut envelope = vec![0x84, 0x40 + protected.len() as u8];
        envelope.extend_from_slice(&protected);
        envelope.extend_from_slice(&[0xa1, 4, 0x42, b'i', b'd', 0xf6, 0x58, 64]);
        envelope.extend_from_slice(&[0; 64]);
        let accepted = parse_sign1(&envelope).is_ok();
        let calls = Cell::new(0);
        let mut rows = [Signature { frame_id: b"frame".to_vec(), kid: None, status: String::new(), cose: Some(envelope) }];
        verify_signatures(&mut rows, |_| { calls.set(calls.get()+1); None });
        println!("{text:?}: parse accepted = {accepted}, status = {}, lookup calls = {}", rows[0].status, calls.get());
    }
}
