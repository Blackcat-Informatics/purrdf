// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Scoped assignment, exact restoration, structured resolution and admission boundaries.

use purrdf_core::content_alias::{Alias, AliasScope, Error, ScopeId, ScopedAlias};
use purrdf_hash::hex::Digest32;
use purrdf_lex::allocation::{Admission, StorageError};
use std::{cell::RefCell, rc::Rc};

// The documented `PURRAL01` fixed header, before its assignment digests.
const HEADER: usize = 80;

struct State {
    bytes: usize,
    work: u64,
    max_bytes: usize,
    max_work: u64,
    refuse_refund_at_least: Option<usize>,
}
#[derive(Clone)]
struct Authority(Rc<RefCell<State>>);
impl Authority {
    fn fresh() -> Self {
        Self(Rc::new(RefCell::new(State {
            bytes: 0,
            work: 0,
            max_bytes: usize::MAX,
            max_work: u64::MAX,
            refuse_refund_at_least: None,
        })))
    }
}
impl Admission for Authority {
    fn progress_enabled(&self) -> bool {
        true
    }
    fn checkpoint(&mut self, units: u64) -> Result<(), StorageError> {
        let mut state = self.0.borrow_mut();
        let next = state
            .work
            .checked_add(units)
            .ok_or(StorageError::SizeOverflow)?;
        if next > state.max_work {
            return Err(StorageError::AdmissionFailed);
        }
        state.work = next;
        Ok(())
    }
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        let mut state = self.0.borrow_mut();
        let refused_refund = bytes < state.bytes
            && state
                .refuse_refund_at_least
                .is_some_and(|minimum| state.bytes - bytes >= minimum);
        if bytes > state.max_bytes || refused_refund {
            return Err(StorageError::AdmissionFailed);
        }
        state.bytes = bytes;
        Ok(())
    }
}
fn scope(byte: u8) -> ScopeId {
    ScopeId::new(Digest32::new([byte; 32]))
}
fn digest(byte: u8) -> Digest32 {
    Digest32::new([byte; 32])
}

#[test]
fn word_prefix_plans_are_minimal_and_retain_the_exact_alias_snapshot() {
    use purrdf_core::content_prefix::{Length, PrefixPlan, ScopedReference};
    use purrdf_hash::word_id::{self, Reference};
    use purrdf_lex::allocation::Resident;
    let id = scope(11);
    let mut close = [0; 32];
    close[31] = 1;
    let close = Digest32::new(close);
    let values = [digest(32), close, digest(0), digest(16), close];
    let mut aliases = AliasScope::new(id, Resident).unwrap();
    aliases.extend(id, &values).unwrap();
    let mut plan = PrefixPlan::from_aliases(&mut aliases, Resident).unwrap();
    let binding = plan.presentation();
    let summary = plan.summary();
    assert_eq!(summary.identities, 4);
    assert_eq!(summary.uniform_payload_words, 22);
    assert_eq!(summary.per_entry_lengths[0], 2);
    assert_eq!(summary.per_entry_lengths[21], 2);
    for value in values {
        for length in [Length::Uniform, Length::PerEntry] {
            let reference = plan.shorten(binding, value, length).unwrap();
            assert_eq!(plan.resolve(reference).unwrap(), value);
            let text = reference.reference.to_text().unwrap();
            assert_eq!(plan.resolve_text(binding, text.as_bytes()).unwrap(), value);
            for words in 1..reference.reference.payload_words() {
                // Only per-entry plans promise individual minimality. The
                // uniform minimum is established by the close pair below.
                if length == Length::PerEntry {
                    assert!(matches!(
                        plan.resolve(ScopedReference {
                            presentation: binding,
                            reference: Reference::from_digest(value, words).unwrap()
                        }),
                        Err(purrdf_core::content_prefix::Error::Ambiguous { .. })
                    ));
                }
            }
        }
    }
    assert!(matches!(
        plan.shorten(binding, digest(99), Length::Uniform),
        Err(purrdf_core::content_prefix::Error::Unknown { .. })
    ));
    assert!(matches!(
        plan.resolve(ScopedReference {
            presentation: binding,
            reference: Reference::Full(digest(99))
        }),
        Err(purrdf_core::content_prefix::Error::Unknown { .. })
    ));
    let old = plan.shorten(binding, digest(16), Length::PerEntry).unwrap();
    let selector = Reference::from_digest(digest(0), 1).unwrap();
    let ambiguous = plan
        .resolve(ScopedReference {
            presentation: binding,
            reference: selector,
        })
        .unwrap_err();
    let captured = purrdf_testkit::error_capture::Capture::read(&ambiguous);
    let prefix = captured.child("reference").child("prefix");
    assert!(matches!(
        prefix.field("payload_words"),
        purrdf_hash::error::ErrorValue::Unsigned(1)
    ));
    assert_eq!(prefix.child("lower").code, "digest32");
    assert!(
        matches!(ambiguous, purrdf_core::content_prefix::Error::Ambiguous { reference, .. } if reference == selector)
    );
    aliases.extend(id, &[digest(17)]).unwrap();
    let mut grown = PrefixPlan::from_aliases(&mut aliases, Resident).unwrap();
    assert!(matches!(
        grown.resolve(old),
        Err(purrdf_core::content_prefix::Error::Binding(
            Error::Revision { .. }
        ))
    ));
    assert_eq!(plan.resolve(old).unwrap(), digest(16));
    let wrong = purrdf_core::content_alias::Presentation {
        scope: scope(12),
        ..binding
    };
    assert!(matches!(
        plan.shorten(wrong, digest(0), Length::Uniform),
        Err(purrdf_core::content_prefix::Error::Binding(
            Error::Scope { .. }
        ))
    ));
    // Lexical validation precedes even an invalid snapshot binding.
    assert!(matches!(
        plan.resolve_text(wrong, b"not a checked word reference"),
        Err(purrdf_core::content_prefix::Error::Codec(_))
    ));
    let mut empty_aliases = AliasScope::new(scope(15), Resident).unwrap();
    let mut empty = PrefixPlan::from_aliases(&mut empty_aliases, Resident).unwrap();
    let empty_binding = empty.presentation();
    assert_eq!(empty.summary().uniform_payload_words, 0);
    assert!(matches!(
        empty.shorten(empty_binding, digest(0), Length::Uniform),
        Err(purrdf_core::content_prefix::Error::Unknown { .. })
    ));
    let mut singleton_aliases = AliasScope::new(scope(16), Resident).unwrap();
    singleton_aliases
        .extend(scope(16), &[digest(0), digest(0)])
        .unwrap();
    let mut singleton = PrefixPlan::from_aliases(&mut singleton_aliases, Resident).unwrap();
    let singleton_binding = singleton.presentation();
    assert_eq!(singleton.summary().identities, 1);
    let one = singleton
        .shorten(singleton_binding, digest(0), Length::Uniform)
        .unwrap();
    assert_eq!(one.reference.payload_words(), 1);
    let mut buffer = [0; word_id::MAX_ENCODED_LEN];
    let len = one.reference.encode(&mut buffer).unwrap();
    assert_eq!(
        singleton
            .resolve_text(singleton_binding, &buffer[..len])
            .unwrap(),
        digest(0)
    );
}

#[test]
fn word_prefix_builds_preserve_source_order_and_original_admission_refusals() {
    use purrdf_core::content_prefix::{BuildError, Error as PrefixError, Length, PrefixPlan};
    use purrdf_lex::allocation::Resident;
    let mut aliases = AliasScope::new(scope(13), Resident).unwrap();
    aliases.extend(scope(13), &[digest(0), digest(1)]).unwrap();
    let binding = aliases.presentation(scope(13)).unwrap();
    let ordered = [digest(0), digest(0), digest(1)].map(Ok::<_, &'static str>);
    let good = PrefixPlan::from_sorted_source(binding, ordered, Resident).unwrap();
    assert_eq!(good.summary().identities, 2);
    let descending = [Ok::<_, &'static str>(digest(1)), Ok(digest(0))];
    assert!(matches!(
        PrefixPlan::from_sorted_source(binding, descending, Resident),
        Err(BuildError::Plan(PrefixError::Order { ordinal: 2, .. }))
    ));
    let authority = Authority::fresh();
    let original = [
        Ok(digest(0)),
        Err("original source unavailable"),
        Ok(digest(1)),
    ];
    assert!(matches!(
        PrefixPlan::from_source(binding, original, authority.clone()),
        Err(BuildError::Source {
            ordinal: 2,
            cause: "original source unavailable"
        })
    ));
    assert_eq!(authority.0.borrow().bytes, 0);
    let calls = std::cell::Cell::new(0);
    authority.0.borrow_mut().max_work = 0;
    let source = std::iter::from_fn(|| {
        calls.set(calls.get() + 1);
        Some(Ok::<_, &'static str>(digest(0)))
    });
    assert!(matches!(
        PrefixPlan::from_source(binding, source, authority.clone()),
        Err(BuildError::Plan(PrefixError::Storage(
            StorageError::AdmissionFailed
        )))
    ));
    assert_eq!(calls.get(), 0);
    authority.0.borrow_mut().max_work = u64::MAX;
    authority.0.borrow_mut().max_bytes = 0;
    assert!(matches!(
        PrefixPlan::from_digests(binding, &[digest(0), digest(1)], authority.clone()),
        Err(PrefixError::Storage(StorageError::AdmissionFailed))
    ));
    assert_eq!(authority.0.borrow().bytes, 0);
    authority.0.borrow_mut().max_bytes = usize::MAX;
    let mut plan =
        PrefixPlan::from_digests(binding, &[digest(0), digest(1)], authority.clone()).unwrap();
    assert!(authority.0.borrow().bytes > 0);
    let spent = authority.0.borrow().work;
    authority.0.borrow_mut().max_work = spent;
    assert!(matches!(
        plan.shorten(binding, digest(0), Length::Uniform),
        Err(PrefixError::Storage(StorageError::AdmissionFailed))
    ));
    authority.0.borrow_mut().max_work = u64::MAX;
    let reference = plan.shorten(binding, digest(0), Length::Uniform).unwrap();
    assert_eq!(plan.resolve(reference).unwrap(), digest(0));
    drop(plan);
    assert_eq!(authority.0.borrow().bytes, 0);
}

#[test]
fn first_seen_assignments_are_scoped_reproducible_and_append_only() {
    let id = scope(9);
    let mut map = AliasScope::new(id, Authority::fresh()).unwrap();
    let mut replay = AliasScope::new(id, Authority::fresh()).unwrap();
    let values = [digest(0), digest(255), digest(0), digest(17)];
    map.extend(id, &values).unwrap();
    replay.extend(id, &values).unwrap();
    for (digest, ordinal) in [(digest(0), 1), (digest(255), 2), (digest(17), 3)] {
        let reference = map.reference(id, digest).unwrap();
        assert_eq!(reference.alias.ordinal(), ordinal);
        assert_eq!(map.resolve(reference).unwrap(), digest);
        assert_eq!(replay.reference(id, digest).unwrap(), reference);
    }
    let before = map.reference(id, digest(255)).unwrap();
    map.extend(id, &[digest(17), digest(1)]).unwrap();
    assert_eq!(map.resolve(before).unwrap(), digest(255));
    assert_eq!(map.reference(id, digest(1)).unwrap().alias.ordinal(), 4);
    assert_eq!(
        map.references(id)
            .unwrap()
            .iter()
            .map(|item| item.alias.ordinal())
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
    assert!(matches!(
        map.reference(scope(8), digest(0)),
        Err(Error::Scope { .. })
    ));
    assert!(matches!(
        map.resolve(ScopedAlias {
            scope: scope(8),
            alias: Alias::new(1).unwrap()
        }),
        Err(Error::Scope { .. })
    ));
    assert!(matches!(map.references(scope(8)), Err(Error::Scope { .. })));
    assert!(matches!(map.export(scope(8)), Err(Error::Scope { .. })));
    assert!(matches!(
        map.presentation(scope(8)),
        Err(Error::Scope { .. })
    ));
    assert!(matches!(
        map.reference(id, digest(99)),
        Err(Error::UnknownDigest { .. })
    ));
    assert!(matches!(
        map.resolve(ScopedAlias {
            scope: id,
            alias: Alias::new(99).unwrap()
        }),
        Err(Error::UnknownReference { .. })
    ));
    assert!(matches!(
        Alias::new(0),
        Err(Error::InvalidAlias { ordinal: 0 })
    ));
}

#[test]
fn exported_bits_restore_exactly_and_conflicts_never_replace_assignments() {
    let id = scope(3);
    let mut map = AliasScope::new(id, Authority::fresh()).unwrap();
    map.extend(id, &[digest(0), digest(255), digest(37)])
        .unwrap();
    let bytes = map.export(id).unwrap().to_vec();
    let mut restored = AliasScope::new(id, Authority::fresh()).unwrap();
    restored.restore(id, &bytes).unwrap();
    assert_eq!(&*restored.export(id).unwrap(), bytes.as_slice());
    let maximal = restored.reference(id, digest(255)).unwrap();
    assert_eq!(restored.resolve(maximal).unwrap(), digest(255));
    let original = restored.presentation(id).unwrap();
    let mut duplicate = bytes.clone();
    let first = duplicate[HEADER..HEADER + Digest32::LEN].to_vec();
    duplicate[HEADER + Digest32::LEN..HEADER + Digest32::LEN * 2].copy_from_slice(&first);
    assert!(matches!(
        restored.restore(id, &duplicate),
        Err(Error::Prefix { .. } | Error::Duplicate { .. })
    ));
    let mut empty = AliasScope::new(id, Authority::fresh()).unwrap();
    assert!(matches!(
        empty.restore(id, &duplicate),
        Err(Error::Duplicate { .. })
    ));
    let mut corrupt = bytes.clone();
    corrupt[8 + Digest32::LEN] ^= 1;
    assert!(matches!(
        restored.restore(id, &corrupt),
        Err(Error::Revision { .. })
    ));
    assert!(matches!(
        restored.restore(scope(4), &bytes),
        Err(Error::Scope { .. })
    ));
    let mut wrong = AliasScope::new(scope(4), Authority::fresh()).unwrap();
    assert!(matches!(
        wrong.restore(scope(4), &bytes),
        Err(Error::Scope { .. })
    ));
    assert!(matches!(
        restored.restore(id, &bytes[..bytes.len() - 1]),
        Err(Error::Wire { .. })
    ));
    assert_eq!(restored.presentation(id).unwrap(), original);
    let mut conflicting = AliasScope::new(id, Authority::fresh()).unwrap();
    conflicting
        .extend(id, &[digest(255), digest(0), digest(37)])
        .unwrap();
    let conflict = conflicting.export(id).unwrap();
    assert!(matches!(
        restored.restore(id, &conflict),
        Err(Error::Prefix { ordinal: 1 })
    ));
    let mut short = AliasScope::new(id, Authority::fresh()).unwrap();
    short.extend(id, &[digest(0)]).unwrap();
    assert!(matches!(
        restored.restore(id, &short.export(id).unwrap()),
        Err(Error::Prefix { ordinal: 2 })
    ));
    assert_eq!(restored.presentation(id).unwrap(), original);
}

#[test]
fn structured_responses_use_exact_presentations_and_full_identities_across_datasets() {
    let (left_id, right_id) = (scope(1), scope(2));
    let mut left = AliasScope::new(left_id, Authority::fresh()).unwrap();
    let mut right = AliasScope::new(right_id, Authority::fresh()).unwrap();
    left.extend(left_id, &[digest(0), digest(255)]).unwrap();
    right.extend(right_id, &[digest(255), digest(0)]).unwrap();
    let presented_left = left.presentation(left_id).unwrap();
    let presented_right = right.presentation(right_id).unwrap();
    let response = purrdf_lex::json::read(r#"{"aliases":[1]}"#).unwrap();
    let mut fork = AliasScope::new(left_id, Authority::fresh()).unwrap();
    fork.extend(left_id, &[digest(255), digest(0)]).unwrap();
    let forked = fork.presentation(left_id).unwrap();
    assert!(matches!(
        left.resolve_response(forked, &response),
        Err(Error::Revision { .. })
    ));
    let left_resolved = left.resolve_response(presented_left, &response).unwrap();
    let right_resolved = right.resolve_response(presented_right, &response).unwrap();
    assert_ne!(left_resolved[0], right_resolved[0]);
    left_resolved.release().unwrap();
    right_resolved.release().unwrap();
    let reordered = purrdf_lex::json::read(r#"{"aliases":[2,1,2]}"#).unwrap();
    assert_eq!(
        &*left.resolve_response(presented_left, &reordered).unwrap(),
        &[digest(255), digest(0), digest(255)]
    );
    assert!(matches!(
        right.resolve_response(presented_left, &response),
        Err(Error::Scope { .. })
    ));
    for source in [
        r#"{"aliases":[1,99]}"#,
        r#"{"aliases":[0]}"#,
        r#"{"aliases":[1.0]}"#,
        r#"{"aliases":["1"]}"#,
        r#"{"aliases":[-1]}"#,
        r#"{"aliases":[1],"other":[]}"#,
        r#"{"alias":[1]}"#,
    ] {
        let invalid = purrdf_lex::json::read(source).unwrap();
        assert!(
            left.resolve_response(presented_left, &invalid).is_err(),
            "{source}"
        );
    }
    left.extend(left_id, &[digest(5)]).unwrap();
    assert!(matches!(
        left.resolve_response(presented_left, &response),
        Err(Error::Revision { .. })
    ));
    let latest = left.presentation(left_id).unwrap();
    assert_eq!(
        &*left.resolve_response(latest, &response).unwrap(),
        &[digest(0)]
    );
}

#[test]
fn original_work_and_storage_admission_protect_atomic_growth_restore_and_output() {
    let control = Authority::fresh();
    let id = scope(7);
    let mut map = AliasScope::new(id, control.clone()).unwrap();
    map.extend(id, &[digest(0), digest(255)]).unwrap();
    let original = map.presentation(id).unwrap();
    let admitted = control.0.borrow().bytes;
    control.0.borrow_mut().max_bytes = admitted;
    assert_eq!(map.extend(id, &[]).unwrap().revision, original.revision);
    assert_eq!(
        map.extend(id, &[digest(0), digest(255), digest(0)])
            .unwrap()
            .revision,
        original.revision
    );
    assert!(matches!(
        map.extend(id, &[digest(1)]),
        Err(Error::Storage(StorageError::AdmissionFailed))
    ));
    control.0.borrow_mut().max_bytes = usize::MAX;
    assert_eq!(map.presentation(id).unwrap(), original);
    assert!(matches!(
        map.reference(id, digest(1)),
        Err(Error::UnknownDigest { .. })
    ));
    let batch: Vec<_> = (1..=64).map(digest).collect();
    let spent = control.0.borrow().work;
    control.0.borrow_mut().max_work = spent + 64;
    assert!(matches!(
        map.extend(id, &batch),
        Err(Error::Storage(StorageError::AdmissionFailed))
    ));
    control.0.borrow_mut().max_work = u64::MAX;
    assert_eq!(map.presentation(id).unwrap(), original);
    assert!(matches!(
        map.reference(id, digest(1)),
        Err(Error::UnknownDigest { .. })
    ));
    map.extend(id, &batch).unwrap();
    let bytes = map.export(id).unwrap().to_vec();
    let other = Authority::fresh();
    let mut restored = AliasScope::new(id, other.clone()).unwrap();
    other.0.borrow_mut().max_bytes = 0;
    assert!(matches!(
        restored.restore(id, &bytes),
        Err(Error::Storage(StorageError::AdmissionFailed))
    ));
    other.0.borrow_mut().max_bytes = usize::MAX;
    assert!(matches!(
        restored.reference(id, digest(0)),
        Err(Error::UnknownDigest { .. })
    ));
    restored.restore(id, &bytes).unwrap();
    let spent = other.0.borrow().work;
    other.0.borrow_mut().max_work = spent;
    assert!(matches!(
        restored.reference(id, digest(0)),
        Err(Error::Storage(StorageError::AdmissionFailed))
    ));
    assert!(matches!(
        restored.references(id),
        Err(Error::Storage(StorageError::AdmissionFailed))
    ));
    other.0.borrow_mut().max_work = u64::MAX;

    let cleanup = Authority::fresh();
    let mut map = AliasScope::new(id, cleanup.clone()).unwrap();
    map.extend(id, &[digest(0)]).unwrap();
    let original_grant = cleanup.0.borrow().bytes;
    // Refuse release of the complete old map after commit, while allowing
    // smaller intermediate index-replacement refunds during construction.
    cleanup.0.borrow_mut().refuse_refund_at_least = Some(original_grant);
    let commit = map.extend(id, &[digest(255)]).unwrap();
    assert_eq!(commit.cleanup_refusal, Some(StorageError::AdmissionFailed));
    assert_eq!(map.reference(id, digest(255)).unwrap().alias.ordinal(), 2);
    cleanup.0.borrow_mut().refuse_refund_at_least = Some(1);
    assert_eq!(
        map.references(id).unwrap().release(),
        Err(StorageError::AdmissionFailed)
    );
    assert_eq!(
        map.take_cleanup_refusal(),
        Some(StorageError::AdmissionFailed)
    );
    cleanup.0.borrow_mut().refuse_refund_at_least = None;
    drop(map);
    assert_eq!(cleanup.0.borrow().bytes, 0);
}
