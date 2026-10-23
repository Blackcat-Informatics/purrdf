// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Caller inputs for one serial rule execution and its nested SPARQL requests.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use purrdf_xsd::temporal::DateTime;

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
}
