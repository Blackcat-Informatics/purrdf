// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::*;
use std::cell::RefCell;
use std::rc::Rc;

struct State {
    bytes: Vec<u8>,
    binding: AccessBinding,
    identity_calls: u128,
    fail_read: bool,
    fail_identity: bool,
}
struct TestSource {
    state: Rc<RefCell<State>>,
    position: u64,
}
#[derive(Debug)]
struct OriginalFailure;
impl fmt::Display for OriginalFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("original indexed source failure")
    }
}
impl std::error::Error for OriginalFailure {}
impl Read for TestSource {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let mut state = self.state.borrow_mut();
        if state.fail_read {
            state.fail_identity = true;
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                OriginalFailure,
            ));
        }
        if self.position >= state.bytes.len() as u64 {
            return Ok(0);
        }
        let start = usize::try_from(self.position).expect("position within the native byte buffer");
        let count = output.len().min(state.bytes.len() - start);
        output[..count].copy_from_slice(&state.bytes[start..start + count]);
        self.position += count as u64;
        Ok(count)
    }
}
impl Seek for TestSource {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.position = match position {
            SeekFrom::Start(position) => Some(position),
            SeekFrom::Current(delta) => self.position.checked_add_signed(delta),
            SeekFrom::End(delta) => {
                (self.state.borrow().bytes.len() as u64).checked_add_signed(delta)
            }
        }
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "seek overflow"))?;
        Ok(self.position)
    }
}
fn current(source: &TestSource) -> io::Result<AccessBinding> {
    let mut state = source.state.borrow_mut();
    state.identity_calls += 1;
    if state.fail_identity {
        return Err(io::Error::other("later identity failure"));
    }
    let mut binding = state.binding.clone();
    binding.length = state.bytes.len() as u64;
    Ok(binding)
}
type Input = IndexedInput<TestSource, fn(&TestSource) -> io::Result<AccessBinding>>;
fn input() -> (Input, Rc<RefCell<State>>) {
    let state = Rc::new(RefCell::new(State {
        bytes: vec![0x81, 7],
        binding: AccessBinding {
            source: AccessOwner::from_owner(Arc::new(())),
            generation: 0,
            length: 2,
            capabilities: AccessOwner::from_owner(Arc::new(())),
            capability_generation: 0,
            policy: AccessOwner::from_owner(Arc::new(())),
        },
        identity_calls: 0,
        fail_read: false,
        fail_identity: false,
    }));
    let input = IndexedInput::new(
        TestSource {
            state: Rc::clone(&state),
            position: 0,
        },
        current as fn(&TestSource) -> io::Result<AccessBinding>,
        DictionaryLimits {
            encoded_frame_bytes: 1024,
            decoded_frame_bytes: 1024,
            metadata_entries: 128,
        },
    )
    .expect("bound source");
    (input, state)
}

#[test]
fn source_and_capability_changes_are_refused_before_cached_range_use() {
    for change in 0..6 {
        let (mut input, state) = input();
        let range = input.scan(0).expect("cold scan").expect("array");
        let before = input.work();
        {
            let mut state = state.borrow_mut();
            match change {
                0 => state.binding.generation += 1,
                1 => state.bytes.push(0),
                2 => state.binding.source = AccessOwner::from_owner(Arc::new(())),
                3 => state.binding.capabilities = AccessOwner::from_owner(Arc::new(())),
                4 => state.binding.capability_generation += 1,
                5 => state.binding.policy = AccessOwner::from_owner(Arc::new(())),
                _ => unreachable!("six concrete identity changes"),
            }
        }
        assert!(matches!(
            input.decode(&range),
            Err(IndexedError::BindingChanged(_))
        ));
        assert_eq!(input.work().read_calls, before.read_calls);
        assert_eq!(input.work().read_bytes, before.read_bytes);
        let checks = input.work().identity_checks;
        assert!(matches!(
            input.decode(&range),
            Err(IndexedError::BindingChanged(_))
        ));
        assert_eq!(
            input.work().identity_checks,
            checks,
            "a failed input remains failed"
        );
    }
}

#[test]
fn foreign_ranges_are_refused_and_matching_cached_children_remain_valid() {
    let (mut original, state) = input();
    let range = original.scan(0).expect("scan").expect("array");
    assert_eq!(
        original.work().read_bytes,
        2,
        "the underlying prefetch is counted"
    );
    let mut children = original.array(&range).expect("array head").expect("array");
    let child = original
        .next(&mut children)
        .expect("child extent")
        .expect("integer");
    assert_eq!(
        original.decode(&child).expect("selected value"),
        Value::from(7_u8)
    );
    assert!(original.next(&mut children).expect("array end").is_none());
    assert_eq!(
        original.work().read_bytes,
        2,
        "cached rewinds do not hide new reads"
    );
    assert_eq!(
        original.work().identity_checks,
        state.borrow().identity_calls
    );

    let (mut foreign, _) = input();
    let before = foreign.work();
    assert!(matches!(
        foreign.decode(&range),
        Err(IndexedError::BindingChanged(_))
    ));
    assert_eq!(foreign.work().read_calls, before.read_calls);
    assert_eq!(foreign.work().read_bytes, before.read_bytes);
}

#[test]
fn original_source_failure_precedes_later_identity_failure_and_stays_typed() {
    let (mut input, state) = input();
    state.borrow_mut().fail_read = true;
    let error = input.scan(0).expect_err("native source refusal");
    assert!(matches!(&error, IndexedError::Structure(cause)
        if matches!(cause.kind(), purrdf_lex::cbor::DecodeErrorKind::Io(cause)
            if cause.kind() == io::ErrorKind::PermissionDenied)));
    let IndexedError::Structure(cause) = &error else {
        panic!("original structural IO refusal")
    };
    let purrdf_lex::cbor::DecodeErrorKind::Io(cause) = cause.kind() else {
        panic!("original IO owner")
    };
    assert!(
        cause
            .original()
            .get_ref()
            .is_some_and(<dyn std::error::Error + Send + Sync>::is::<OriginalFailure>),
        "the original opaque payload remains in its native IO owner"
    );
    let checks = input.work().identity_checks;
    assert!(matches!(
        input.check_current(),
        Err(IndexedError::Structure(_))
    ));
    assert_eq!(input.work().identity_checks, checks);
    let policy = Arc::new(LookupPolicy {
        allow_segments: false,
        expected_head: Some(vec![0; 32]),
        limits: input.limits(),
    });
    let capabilities = DecodeCapabilities::new(Arc::new(|_: &str| None));
    let failure = crate::reader::indexed::IndexedCatalog::new(input, capabilities, policy)
        .expect_err("original source failure precedes later concrete capability/policy refusals");
    assert!(matches!(failure, IndexedError::Structure(_)));
}
