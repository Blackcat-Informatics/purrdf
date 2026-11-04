// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Frozen upstream rendering law, canonical byte identity and native admission.
use purrdf_hash::hex::{self, Digest32};
use purrdf_lex::{
    allocation::{Admission, Resident, StorageError},
    json,
    visual_fingerprint::{self as visual, Error, Request},
};
use std::{cell::RefCell, rc::Rc};

#[test]
fn frozen_renderings_and_digest_spellings_use_original_bytes() {
    for source in [
        include_str!("../../../vectors/emojihash/empty.json"),
        include_str!("../../../vectors/emojihash/ascii-gts.json"),
        include_str!("../../../vectors/emojihash/zeros32.json"),
        include_str!("../../../vectors/emojihash/seq32.json"),
    ] {
        let vector = json::read(source).unwrap();
        let spelling = vector["data"].as_str().unwrap();
        let bytes = hex::decode(spelling).unwrap();
        let count = usize::try_from(vector["length"].as_u64().unwrap()).unwrap();
        let request = Request::new(count, "").unwrap();
        let rendered = visual::render_bytes(&bytes, request, Resident).unwrap();
        assert_eq!(rendered.emoji(), vector["emoji"].as_str().unwrap());
        assert_eq!(rendered.spoken(), vector["labels"].as_str().unwrap());
        let indices: Vec<_> = rendered
            .emoji()
            .split(' ')
            .map(|emoji| {
                visual::EMOJI
                    .iter()
                    .position(|entry| *entry == emoji)
                    .unwrap()
            })
            .collect();
        let frozen: Vec<_> = vector["indices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|index| usize::try_from(index.as_u64().unwrap()).unwrap())
            .collect();
        assert_eq!(indices, frozen);
        if bytes.len() == Digest32::LEN {
            for text in [spelling.to_owned(), spelling.to_uppercase()] {
                let digest = visual::parse_digest(&text, &mut Resident).unwrap();
                let presented = visual::present(digest, request, Resident).unwrap();
                assert_eq!(presented.rendering().emoji(), rendered.emoji());
                assert_eq!(presented.rendering().spoken(), rendered.spoken());
                assert_eq!(presented.rendering().randomart(), rendered.randomart());
            }
        }
    }
    for source in [
        include_str!("../../../vectors/randomart/empty.json"),
        include_str!("../../../vectors/randomart/ascii-gts.json"),
        include_str!("../../../vectors/randomart/zeros32.json"),
        include_str!("../../../vectors/randomart/seq32.json"),
        include_str!("../../../vectors/randomart/labeled-ed25519.json"),
    ] {
        let vector = json::read(source).unwrap();
        let bytes = hex::decode(vector["data"].as_str().unwrap()).unwrap();
        let request = Request::new(11, vector["label"].as_str().unwrap()).unwrap();
        let rendered = visual::render_bytes(&bytes, request, Resident).unwrap();
        assert_eq!(rendered.randomart(), vector["art"].as_str().unwrap());
    }
}

#[derive(Default)]
struct Observation {
    live: usize,
    peak: usize,
    work: u64,
    calls: usize,
}
struct Account {
    state: Rc<RefCell<Observation>>,
    bytes: usize,
    work: u64,
    refuse_cleanup: bool,
}
impl Admission for Account {
    fn progress_enabled(&self) -> bool {
        true
    }
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        if bytes > self.bytes || bytes == 0 && self.refuse_cleanup {
            return Err(StorageError::AdmissionFailed);
        }
        let mut observed = self.state.borrow_mut();
        observed.live = bytes;
        observed.peak = observed.peak.max(bytes);
        Ok(())
    }
    fn checkpoint(&mut self, units: u64) -> Result<(), StorageError> {
        let mut observed = self.state.borrow_mut();
        observed.calls += 1;
        observed.work = observed
            .work
            .checked_add(units)
            .ok_or(StorageError::SizeOverflow)?;
        if observed.work > self.work {
            Err(StorageError::AdmissionFailed)
        } else {
            Ok(())
        }
    }
}

#[test]
fn exact_admission_retains_output_and_refuses_atomically() {
    let digest = Digest32::new([0; 32]);
    let request = Request::new(11, "BLAKE3 256").unwrap();
    let resident = visual::present(digest, request, Resident).unwrap();
    let expected = resident.rendering();
    let bytes = expected.emoji().len() + expected.spoken().len() + expected.randomart().len();
    let state = Rc::new(RefCell::new(Observation::default()));
    let make = |bytes, work, refuse_cleanup| Account {
        state: Rc::clone(&state),
        bytes,
        work,
        refuse_cleanup,
    };
    let output = visual::present(digest, request, make(bytes, u64::MAX, false)).unwrap();
    assert_eq!(output.rendering().emoji(), expected.emoji());
    assert_eq!(output.rendering().spoken(), expected.spoken());
    assert_eq!(
        state.borrow().live,
        bytes,
        "exact shorter labels fit without worst-case over-reservation"
    );
    let work = state.borrow().work;
    output.release().unwrap();
    assert_eq!(
        state.borrow().live,
        0,
        "output dies before its original grant is refunded"
    );
    *state.borrow_mut() = Observation::default();
    assert!(matches!(
        visual::present(digest, request, make(bytes - 1, u64::MAX, false)),
        Err(Error::Storage {
            cause: StorageError::AdmissionFailed,
            ..
        })
    ));
    assert_eq!(
        state.borrow().live,
        0,
        "refusal leaves no partial output allocation"
    );
    *state.borrow_mut() = Observation::default();
    let output = visual::present(digest, request, make(bytes, work, false)).unwrap();
    output.release().unwrap();
    *state.borrow_mut() = Observation::default();
    assert!(matches!(
        visual::present(digest, request, make(bytes, work - 1, false)),
        Err(Error::Storage {
            cause: StorageError::AdmissionFailed,
            ..
        })
    ));
    assert_eq!(state.borrow().live, 0);
    *state.borrow_mut() = Observation::default();
    let huge = Request::new(1_000_000_000, "").unwrap();
    assert!(visual::present(digest, huge, make(usize::MAX, 1, false)).is_err());
    assert!(
        state.borrow().calls <= 2,
        "work refusal precedes the huge count scan"
    );
    assert_eq!(state.borrow().peak, 0);
    assert!(matches!(Request::new(0, ""), Err(Error::Length { .. })));
    assert!(matches!(
        Request::new(usize::MAX, ""),
        Err(Error::Length { .. })
    ));
    assert!(matches!(
        visual::parse_digest("ff", &mut Resident),
        Err(Error::DigestLength { .. })
    ));
    assert!(matches!(
        visual::parse_digest(&"z".repeat(64), &mut Resident),
        Err(Error::Hex(_))
    ));
    *state.borrow_mut() = Observation::default();
    assert!(matches!(
        visual::present(digest, request, make(bytes, 0, true)),
        Err(Error::Storage {
            cause: StorageError::AdmissionFailed,
            cleanup_refusal: Some(StorageError::AdmissionFailed)
        })
    ));
    *state.borrow_mut() = Observation::default();
    let output = visual::present(digest, request, make(bytes, u64::MAX, true)).unwrap();
    assert_eq!(output.release(), Err(StorageError::AdmissionFailed));
    assert_eq!(
        state.borrow().live,
        bytes,
        "refused cleanup retains the conservative original grant"
    );
}
