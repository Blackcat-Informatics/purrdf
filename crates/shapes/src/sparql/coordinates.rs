// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Caller inputs for one serial rule execution and its nested SPARQL requests.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use purrdf_xsd::temporal::DateTime;

/// Effective supplied coordinates at the start of one successful rule inference.
///
/// This records inputs, not a complete execution or snapshot identity. An absent
/// value makes no claim about a host clock or entropy sample. Empty request options
/// can inherit an outer supplied scope; a partially supplied request replaces it.
/// The exact time shares the original caller-input owner, extending its lifetime
/// without copying its coefficient or allocating another owner. This retention is
/// not a new execution allocation.
#[derive(Debug, Clone, Default)]
pub struct RuleCoordinates {
    now: Option<Arc<DateTime>>,
    entropy_seed: Option<u64>,
}

impl RuleCoordinates {
    /// The effective supplied time, including exact seconds and declared timezone.
    #[must_use]
    pub fn now(&self) -> Option<&DateTime> {
        self.now.as_deref()
    }

    /// Starting state of this inference's `SplitMix64` query-seed sequence.
    /// This is not an individual query's seed or its evolved random state.
    /// Capturing the receipt consumes no draw.
    #[must_use]
    pub const fn entropy_seed(&self) -> Option<u64> {
        self.entropy_seed
    }
}

impl PartialEq for RuleCoordinates {
    fn eq(&self, other: &Self) -> bool {
        self.entropy_seed == other.entropy_seed
            && match (&self.now, &other.now) {
                (None, None) => true,
                (Some(left), Some(right)) => {
                    // Equal instants with different offsets can emit different
                    // STR(NOW()) values. Compare the retained representation,
                    // whose Decimal seconds already normalize lexical zeros.
                    left.year() == right.year()
                        && left.month() == right.month()
                        && left.day() == right.day()
                        && left.hour() == right.hour()
                        && left.minute() == right.minute()
                        && left.second_ref() == right.second_ref()
                        && left.timezone_minutes() == right.timezone_minutes()
                }
                _ => false,
            }
    }
}

impl Eq for RuleCoordinates {}

/// Borrow the effective runtime before any query advances its seed sequence.
pub(crate) fn snapshot() -> RuleCoordinates {
    CURRENT.with(|slot| {
        slot.borrow()
            .as_ref()
            .map_or_else(RuleCoordinates::default, |runtime| RuleCoordinates {
                now: runtime.now.clone(),
                entropy_seed: runtime.entropy,
            })
    })
}

#[derive(Debug)]
pub(crate) struct Runtime {
    now: Option<Arc<DateTime>>,
    entropy: Option<u64>,
}

thread_local! {
    static CURRENT: RefCell<Option<Runtime>> = const { RefCell::new(None) };
}

#[must_use]
pub(crate) struct Scope {
    previous: Option<Runtime>,
    _not_send: PhantomData<Rc<()>>,
}

impl Drop for Scope {
    fn drop(&mut self) {
        let _ = replace(self.previous.take());
    }
}

pub(crate) fn enter(now: Option<Arc<DateTime>>, entropy: Option<u64>) -> Option<Scope> {
    if now.is_none() && entropy.is_none() {
        return None;
    }
    Some(Scope {
        previous: replace(Some(Runtime { now, entropy })),
        _not_send: PhantomData,
    })
}

pub(super) fn replace(next: Option<Runtime>) -> Option<Runtime> {
    CURRENT.with(|slot| slot.replace(next))
}

pub(super) fn now() -> Option<Arc<DateTime>> {
    CURRENT.with(|slot| slot.borrow().as_ref()?.now.clone())
}

/// Draw only at the common execution-options seam, never during preparation.
pub(super) fn next_seed() -> Option<u64> {
    CURRENT.with(|slot| {
        let mut runtime = slot.borrow_mut();
        let state = runtime.as_mut()?.entropy.as_mut()?;
        Some(purrdf_hash::mix::splitmix64_next(state))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suspended_and_nested_scopes_preserve_the_seed_counter() {
        let mut reference = 7;
        let first = purrdf_hash::mix::splitmix64_next(&mut reference);
        let second = purrdf_hash::mix::splitmix64_next(&mut reference);
        let outer = enter(None, Some(7)).unwrap();
        assert_eq!(snapshot().entropy_seed(), Some(7));
        assert_eq!(next_seed(), Some(first));
        let suspended = replace(None);
        assert_eq!(next_seed(), None);
        assert!(enter(None, None).is_none());
        let inner = enter(None, Some(100)).unwrap();
        let mut reference = 100;
        assert_eq!(
            next_seed(),
            Some(purrdf_hash::mix::splitmix64_next(&mut reference))
        );
        drop(inner);
        assert_eq!(next_seed(), None);
        assert!(replace(suspended).is_none());
        assert_eq!(next_seed(), Some(second));
        drop(outer);
        assert_eq!(next_seed(), None);
        let _fresh = enter(None, Some(7)).unwrap();
        assert_eq!(next_seed(), Some(first));
    }

    #[test]
    fn suspended_time_keeps_the_original_exact_input_owner() {
        let lexical = format!("2024-02-29T12:34:56.{}+05:30", "7".repeat(4096));
        let caller = Arc::new(purrdf_xsd::temporal::parse_datetime(&lexical).unwrap());
        let identity = Arc::as_ptr(&caller);
        let expected = caller.canonical_lexical();
        let scope = enter(Some(Arc::clone(&caller)), Some(7)).unwrap();
        drop(caller);
        let selected = now().unwrap();
        assert_eq!(Arc::as_ptr(&selected), identity);
        assert_eq!(selected.canonical_lexical(), expected);
        drop(selected);
        let suspended = replace(None);
        assert!(now().is_none());
        let nested = enter(
            Some(Arc::new(
                purrdf_xsd::temporal::parse_datetime("2025-01-01T00:00:00Z").unwrap(),
            )),
            Some(8),
        )
        .unwrap();
        assert_ne!(Arc::as_ptr(&now().unwrap()), identity);
        drop(nested);
        assert!(now().is_none());
        assert!(replace(suspended).is_none());
        let selected = now().unwrap();
        assert_eq!(Arc::as_ptr(&selected), identity);
        assert_eq!(selected.canonical_lexical(), expected);
        drop(scope);
        assert!(now().is_none());
        assert_eq!(selected.canonical_lexical(), expected);
    }

    #[test]
    fn inference_receipt_inherits_current_seed_and_releases_original_time_owner() {
        use crate::srl::{self, InferOptions};

        let document = srl::parse_and_check(
            "PREFIX : <http://example.org/> RULE { :s :time ?time } WHERE { SET(?time := NOW()) }",
            None,
        )
        .unwrap();
        let source = purrdf_core::RdfDatasetBuilder::new().freeze().unwrap();
        let caller = Arc::new(
            purrdf_xsd::temporal::parse_datetime(&format!(
                "2024-02-29T12:34:56.{}+05:30",
                "7".repeat(4096)
            ))
            .unwrap(),
        );
        let original = Arc::as_ptr(&caller);
        let weak = Arc::downgrade(&caller);
        let scope = enter(Some(caller), Some(7)).unwrap();
        let mut expected_seed = 7;
        assert_eq!(
            next_seed(),
            Some(purrdf_hash::mix::splitmix64_next(&mut expected_seed))
        );
        let result = purrdf_testkit::harness::without_host_clock_or_entropy(|| {
            srl::infer(&document, &source, &InferOptions::default())
        })
        .unwrap();
        assert_eq!(result.coordinates().entropy_seed(), Some(expected_seed));
        assert_eq!(
            std::ptr::from_ref(result.coordinates().now().unwrap()),
            original
        );

        // A partial request replaces the complete outer scope, rather than
        // silently inheriting its missing input. Returning restores the outer run.
        let time_only = InferOptions::default()
            .with_now(purrdf_xsd::temporal::parse_datetime("2025-01-01T00:00:00Z").unwrap());
        let replaced = srl::infer(&document, &source, &time_only).unwrap();
        assert!(replaced.coordinates().entropy_seed().is_none());
        assert_ne!(result.coordinates(), replaced.coordinates());
        let seed_only = srl::infer(
            &document,
            &source,
            &InferOptions::default().with_entropy_seed(11),
        )
        .unwrap();
        assert!(seed_only.coordinates().now().is_none());
        assert_eq!(seed_only.coordinates().entropy_seed(), Some(11));
        assert_eq!(Arc::as_ptr(&now().unwrap()), original);

        let retained = result.clone();
        let receipt = result.coordinates().clone();
        drop(result);
        drop(scope);
        drop(source);
        drop(document);
        assert!(now().is_none());
        assert!(weak.upgrade().is_some());
        assert_eq!(retained.coordinates(), &receipt);
        drop(retained);
        assert!(weak.upgrade().is_some());
        drop(receipt);
        assert!(weak.upgrade().is_none());
    }
}
