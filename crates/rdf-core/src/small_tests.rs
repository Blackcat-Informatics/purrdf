// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Tests for [`SmallVec`]: a model-based property test against `Vec` at inline
//! capacities 0, 1, 4 and 8, an ownership-counting variant over a non-`Copy`
//! element, and targeted drop-accounting, panic-safety, zero-sized,
//! over-aligned, variance, auto-trait and overflow cases.

use std::cell::{Cell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use purrdf_testkit::prop::prelude::*;

use super::{Array, IdVec, IntoIter, SmallVec};
use crate::hash::hash_of;

/// Cases per property; fewer under Miri, whose interpreter is slow.
const CASES: u32 = if cfg!(miri) { 6 } else { 256 };

/// One generated operation: a selector, two operands and a short run of values.
type Op = (u8, u16, u16, Vec<u16>);

fn op_sequences() -> impl Strategy<Value = Vec<Op>> {
    let max_ops = if cfg!(miri) { 24 } else { 64 };
    prop::collection::vec(
        (
            any::<u8>(),
            any::<u16>(),
            any::<u16>(),
            prop::collection::vec(any::<u16>(), 0..12),
        ),
        0..max_ops,
    )
}

/// Every observable property of `sv` agrees with the model.
fn check<A: Array<Item = u16>>(sv: &SmallVec<A>, model: &[u16]) {
    assert_eq!(&sv[..], model);
    assert_eq!(sv.as_slice(), model);
    assert_eq!(sv.len(), model.len());
    assert_eq!(sv.is_empty(), model.is_empty());
    assert!(sv.capacity() >= sv.len());
    assert_eq!(sv.inline_size(), A::CAPACITY);
    assert_eq!(sv.spilled(), sv.capacity() > A::CAPACITY);
    assert_eq!(hash_of(sv), hash_of(model));
    assert_eq!(format!("{sv:?}"), format!("{model:?}"));
    let probe: &[u16] = &[3, 1, 4];
    let probe_sv = SmallVec::<A>::from_slice(probe);
    assert_eq!(sv.cmp(&probe_sv), model.cmp(probe));
    assert_eq!(sv.partial_cmp(&probe_sv), model.partial_cmp(probe));
    assert_eq!(*sv == probe_sv, model == probe);
    let as_ref: &[u16] = sv.as_ref();
    assert_eq!(as_ref, model);
    let borrowed: &[u16] = std::borrow::Borrow::borrow(sv);
    assert_eq!(borrowed, model);
    assert_eq!(sv.iter().count(), model.len());
    assert_eq!(sv.into_iter().copied().collect::<Vec<_>>(), model);
}

/// Applies `ops` to a `SmallVec<A>` and to a `Vec`, checking agreement after
/// every step.
fn run_model<A: Array<Item = u16>>(ops: &[Op]) {
    let mut sv = SmallVec::<A>::new();
    let mut model: Vec<u16> = Vec::new();
    for (selector, a, b, values) in ops {
        let (a, b) = (*a, *b);
        let len = model.len();
        match selector % 38 {
            0..=2 => {
                sv.push(a);
                model.push(a);
            }
            3 => assert_eq!(sv.pop(), model.pop()),
            4 => {
                let at = usize::from(a) % (len + 1);
                sv.insert(at, b);
                model.insert(at, b);
            }
            5 if len > 0 => {
                let at = usize::from(a) % len;
                assert_eq!(sv.remove(at), model.remove(at));
            }
            6 if len > 0 => {
                let at = usize::from(a) % len;
                assert_eq!(sv.swap_remove(at), model.swap_remove(at));
            }
            7 => {
                let to = usize::from(a) % (len + 2);
                sv.truncate(to);
                model.truncate(to);
            }
            8 if a % 4 == 0 => {
                sv.clear();
                model.clear();
            }
            9 => {
                let extra = usize::from(a) % 20;
                sv.reserve(extra);
                assert!(sv.capacity() >= len + extra);
            }
            10 => {
                let extra = usize::from(a) % 20;
                sv.reserve_exact(extra);
                assert!(sv.capacity() >= len + extra);
            }
            11 => {
                sv.shrink_to_fit();
                if len <= A::CAPACITY {
                    assert!(!sv.spilled());
                }
            }
            12 => {
                sv.extend_from_slice(values);
                model.extend_from_slice(values);
            }
            13 => {
                sv.extend(values.iter().copied());
                model.extend(values.iter().copied());
            }
            14 => {
                // An iterator with no useful size hint takes the push path.
                let odd = values.iter().copied().filter(|v| v % 2 == 1);
                sv.extend(odd.clone());
                model.extend(odd);
            }
            15 => {
                let to = usize::from(a) % 20;
                sv.resize(to, b);
                model.resize(to, b);
            }
            16 => {
                let to = usize::from(a) % 20;
                let mut next = b;
                sv.resize_with(to, || {
                    next = next.wrapping_add(1);
                    next
                });
                let mut next = b;
                model.resize_with(to, || {
                    next = next.wrapping_add(1);
                    next
                });
            }
            17 => {
                let m = a % 3;
                sv.retain(|v| *v % 3 != m);
                model.retain(|v| *v % 3 != m);
            }
            18 => {
                sv.retain_mut(|v| {
                    *v = v.wrapping_add(1);
                    *v % 2 == 0
                });
                model.retain_mut(|v| {
                    *v = v.wrapping_add(1);
                    *v % 2 == 0
                });
            }
            19 => {
                sv.dedup();
                model.dedup();
            }
            20 => {
                sv.dedup_by_key(|v| *v / 4);
                model.dedup_by_key(|v| *v / 4);
            }
            21 => {
                sv.dedup_by(|x, y| x.abs_diff(*y) < 1000);
                model.dedup_by(|x, y| x.abs_diff(*y) < 1000);
            }
            22 => {
                let mut other = SmallVec::<[u16; 2]>::from_slice(values);
                sv.append(&mut other);
                model.extend_from_slice(values);
                assert!(other.is_empty());
            }
            23 => {
                let copy = sv.clone();
                check(&copy, &model);
                sv = copy;
            }
            24 => {
                let mut target = SmallVec::<A>::from_slice(values);
                target.clone_from(&sv);
                sv = target;
            }
            25 => sv = sv.into_iter().collect(),
            26 => {
                sv = sv.into_iter().rev().collect();
                model.reverse();
            }
            27 => {
                let mut rest = sv.into_iter();
                assert_eq!(rest.len(), model.len());
                assert_eq!(rest.next(), (!model.is_empty()).then(|| model.remove(0)));
                assert_eq!(rest.next_back(), model.pop());
                assert_eq!(rest.as_slice(), &model[..]);
                assert_eq!(rest.as_mut_slice(), &mut model[..]);
                let cloned = rest.clone();
                assert_eq!(cloned.as_slice(), &model[..]);
                assert_eq!(format!("{cloned:?}"), format!("IntoIter({model:?})"));
                sv = rest.collect();
            }
            28 => sv = SmallVec::from_vec(sv.into_vec()),
            29 => sv = SmallVec::from(sv.into_vec()),
            30 => sv = SmallVec::from_vec(sv.into_boxed_slice().into_vec()),
            31 => {
                sv = SmallVec::from_slice(values);
                model.clone_from(values);
            }
            32 => {
                let at = usize::from(a) % (len + 1);
                sv.insert_from_slice(at, values);
                model.splice(at..at, values.iter().copied());
            }
            33 => {
                sv.sort_unstable();
                model.sort_unstable();
                if let Some(first) = sv.first_mut() {
                    *first = b;
                    model[0] = b;
                }
                if len > 1 {
                    sv[len - 1] = a;
                    model[len - 1] = a;
                }
            }
            34 => {
                let n = usize::from(a) % 20;
                sv = SmallVec::from_elem(b, n);
                model = vec![b; n];
            }
            35 => {
                sv = crate::smallvec![a, b];
                model = vec![a, b];
            }
            36 => match sv.into_inner() {
                Ok(array) => {
                    assert_eq!(model.len(), A::CAPACITY);
                    sv = SmallVec::from_buf(array);
                }
                Err(back) => {
                    assert!(back.spilled() || back.len() != A::CAPACITY);
                    sv = back;
                }
            },
            37 => {
                sv = SmallVec::from(&values[..]);
                model.clone_from(values);
            }
            _ => {}
        }
        check(&sv, &model);
    }
}

prop_test! {
    #![prop_config(Config::with_cases(CASES))]

    /// Every operation agrees with `Vec` at every inline capacity.
    #[test]
    fn model_matches_vec(ops in op_sequences()) {
        run_model::<[u16; 0]>(&ops);
        run_model::<[u16; 1]>(&ops);
        run_model::<[u16; 4]>(&ops);
        run_model::<[u16; 8]>(&ops);
    }

    /// Ownership never goes wrong for a non-`Copy` element: every `Rc` created
    /// is held exactly by the model and the vector, and is released at the end.
    #[test]
    fn ownership_matches_vec(ops in op_sequences()) {
        run_rc_model::<[Rc<u16>; 0]>(&ops);
        run_rc_model::<[Rc<u16>; 1]>(&ops);
        run_rc_model::<[Rc<u16>; 4]>(&ops);
        run_rc_model::<[Rc<u16>; 8]>(&ops);
    }
}

/// The ownership-counting model: the non-`Copy` subset of the operations over
/// `Rc<u16>`, where each value's strong count must equal one (the registry) plus
/// its occurrences in the vector and in the model.
fn run_rc_model<A: Array<Item = Rc<u16>>>(ops: &[Op]) {
    fn fresh(v: u16, registry: &mut Vec<Rc<u16>>) -> Rc<u16> {
        let rc = Rc::new(v);
        registry.push(Rc::clone(&rc));
        rc
    }
    let mut registry: Vec<Rc<u16>> = Vec::new();
    {
        let mut sv = SmallVec::<A>::new();
        let mut model: Vec<Rc<u16>> = Vec::new();
        for (selector, a, b, values) in ops {
            let (a, b) = (*a, *b);
            let len = model.len();
            match selector % 16 {
                0 | 1 => {
                    let rc = fresh(a, &mut registry);
                    model.push(Rc::clone(&rc));
                    sv.push(rc);
                }
                2 => assert_eq!(sv.pop(), model.pop()),
                3 => {
                    let at = usize::from(a) % (len + 1);
                    let rc = fresh(b, &mut registry);
                    model.insert(at, Rc::clone(&rc));
                    sv.insert(at, rc);
                }
                4 if len > 0 => {
                    let at = usize::from(a) % len;
                    assert_eq!(sv.remove(at), model.remove(at));
                }
                5 if len > 0 => {
                    let at = usize::from(a) % len;
                    assert_eq!(sv.swap_remove(at), model.swap_remove(at));
                }
                6 => {
                    let to = usize::from(a) % (len + 2);
                    sv.truncate(to);
                    model.truncate(to);
                }
                7 => {
                    let new: Vec<Rc<u16>> =
                        values.iter().map(|v| fresh(*v, &mut registry)).collect();
                    model.extend(new.iter().cloned());
                    sv.extend(new);
                }
                8 => {
                    let to = usize::from(a) % 20;
                    let rc = fresh(b, &mut registry);
                    model.resize(to, Rc::clone(&rc));
                    sv.resize(to, rc);
                }
                9 => {
                    let m = a % 3;
                    sv.retain(|v| **v % 3 != m);
                    model.retain(|v| **v % 3 != m);
                }
                10 => {
                    sv.dedup_by_key(|v| **v / 8);
                    model.dedup_by_key(|v| **v / 8);
                }
                11 => sv = sv.clone(),
                12 => {
                    let mut rest = sv.into_iter();
                    if rest.next().is_some() {
                        model.remove(0);
                    }
                    sv = rest.collect();
                }
                13 => sv = SmallVec::from_vec(sv.into_vec()),
                14 => sv.shrink_to_fit(),
                15 => {
                    let n = usize::from(a) % 12;
                    let rc = fresh(b, &mut registry);
                    model = vec![Rc::clone(&rc); n];
                    sv = crate::smallvec![rc; n];
                }
                _ => {}
            }
            assert_eq!(&sv[..], &model[..]);
            for rc in &registry {
                let occurrences = |xs: &[Rc<u16>]| xs.iter().filter(|x| Rc::ptr_eq(x, rc)).count();
                assert_eq!(
                    Rc::strong_count(rc),
                    1 + occurrences(&sv) + occurrences(&model)
                );
            }
        }
    }
    for rc in &registry {
        assert_eq!(
            Rc::strong_count(rc),
            1,
            "a value was leaked or dropped twice"
        );
    }
}

// ---------------------------------------------------------------------------
// Drop accounting.

/// A shared log of element lifecycles.
#[derive(Default)]
struct Log {
    /// Drop count per element id.
    drops: RefCell<Vec<usize>>,
    /// The id whose `Drop` panics (after being counted), if any.
    panic_on_drop: Cell<Option<usize>>,
    /// Clones still allowed before `Clone` panics.
    clones_left: Cell<usize>,
}

impl Log {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            clones_left: Cell::new(usize::MAX),
            ..Self::default()
        })
    }

    fn make(self: &Rc<Self>) -> Tracked {
        let mut drops = self.drops.borrow_mut();
        drops.push(0);
        Tracked {
            id: drops.len() - 1,
            log: Rc::clone(self),
        }
    }

    fn created(&self) -> usize {
        self.drops.borrow().len()
    }

    fn drops_of(&self, id: usize) -> usize {
        self.drops.borrow()[id]
    }

    fn assert_each_dropped_once(&self) {
        for (id, &n) in self.drops.borrow().iter().enumerate() {
            assert_eq!(n, 1, "element {id} dropped {n} times");
        }
    }
}

/// An element that records its drops and can be told to panic.
#[derive(Debug)]
struct Tracked {
    id: usize,
    log: Rc<Log>,
}

impl std::fmt::Debug for Log {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Log")
    }
}

impl Clone for Tracked {
    fn clone(&self) -> Self {
        let left = self.log.clones_left.get();
        assert!(left > 0, "clone budget exhausted");
        self.log.clones_left.set(left - 1);
        self.log.make()
    }
}

impl Drop for Tracked {
    fn drop(&mut self) {
        self.log.drops.borrow_mut()[self.id] += 1;
        if self.log.panic_on_drop.get() == Some(self.id) {
            self.log.panic_on_drop.set(None);
            panic!("element {} panics on drop", self.id);
        }
    }
}

fn tracked<A: Array<Item = Tracked>>(log: &Rc<Log>, n: usize) -> SmallVec<A> {
    (0..n).map(|_| log.make()).collect()
}

#[test]
fn every_ownership_path_drops_each_element_once() {
    for n in [0, 2, 4, 9] {
        let log = Log::new();
        // Plain drop, inline and spilled.
        drop(tracked::<[Tracked; 4]>(&log, n));
        // Pop / remove / swap_remove / truncate / clear.
        let mut v = tracked::<[Tracked; 4]>(&log, n);
        drop(v.pop());
        if v.len() > 1 {
            drop(v.remove(1));
            drop(v.swap_remove(0));
        }
        v.truncate(1);
        v.clear();
        drop(v);
        // Owning iteration, fully and partially consumed, from both ends.
        let v = tracked::<[Tracked; 4]>(&log, n);
        for item in v {
            drop(item);
        }
        let mut it = tracked::<[Tracked; 4]>(&log, n).into_iter();
        drop(it.next());
        drop(it.next_back());
        drop(it);
        // Conversions.
        drop(tracked::<[Tracked; 4]>(&log, n).into_vec());
        drop(SmallVec::<[Tracked; 4]>::from_vec(
            tracked::<[Tracked; 4]>(&log, n).into_vec(),
        ));
        drop(tracked::<[Tracked; 4]>(&log, n).into_boxed_slice());
        // Clones and in-place edits.
        let v = tracked::<[Tracked; 4]>(&log, n);
        let mut w = v.clone();
        w.clone_from(&v);
        w.retain(|t| t.id % 2 == 0);
        w.dedup_by(|a, b| a.id / 3 == b.id / 3);
        w.resize_with(n + 3, || log.make());
        w.resize(1, log.make());
        let mut x = tracked::<[Tracked; 2]>(&log, n);
        w.append(&mut x);
        w.shrink_to_fit();
        drop((v, w, x));
        log.assert_each_dropped_once();
    }
}

#[test]
fn panic_during_extend_drops_each_element_once() {
    for (already, yielded) in [(0, 0), (1, 2), (0, 6), (3, 8), (6, 3)] {
        let log = Log::new();
        let mut v = tracked::<[Tracked; 4]>(&log, already);
        let source = Rc::clone(&log);
        let mut count = 0;
        let items = std::iter::from_fn(move || {
            assert!(count < yielded, "iterator panics after {yielded} items");
            count += 1;
            Some(source.make())
        });
        let outcome = catch_unwind(AssertUnwindSafe(|| v.extend(items)));
        assert!(outcome.is_err());
        assert_eq!(v.len(), already + yielded);
        drop(v);
        log.assert_each_dropped_once();
    }
}

#[test]
fn panic_during_extend_with_exact_hint_drops_each_element_once() {
    // A size hint that promises more than it yields fills the reserved slots
    // through the guarded fast path before the panic.
    struct Liar {
        log: Rc<Log>,
        left: usize,
    }
    impl Iterator for Liar {
        type Item = Tracked;
        fn next(&mut self) -> Option<Tracked> {
            assert!(self.left > 0, "liar runs dry");
            self.left -= 1;
            Some(self.log.make())
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            (self.left + 10, None)
        }
    }
    for left in [0, 3, 12] {
        let log = Log::new();
        let mut v = SmallVec::<[Tracked; 4]>::new();
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            v.extend(Liar {
                log: Rc::clone(&log),
                left,
            });
        }));
        assert!(outcome.is_err());
        assert_eq!(v.len(), left);
        drop(v);
        log.assert_each_dropped_once();
    }
}

#[test]
fn panic_during_clone_drops_each_element_once() {
    for (n, budget) in [(3, 0), (3, 2), (9, 0), (9, 5), (9, 8)] {
        let log = Log::new();
        let v = tracked::<[Tracked; 4]>(&log, n);
        log.clones_left.set(budget);
        let outcome = catch_unwind(AssertUnwindSafe(|| v.clone()));
        assert!(outcome.is_err());
        // The clones made before the panic have already been dropped.
        assert_eq!(log.created(), n + budget);
        for id in n..n + budget {
            assert_eq!(log.drops_of(id), 1);
        }
        log.clones_left.set(budget);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            SmallVec::<[Tracked; 4]>::from_elem(log.make(), n + budget + 1)
        }));
        assert!(outcome.is_err());
        drop(v);
        log.assert_each_dropped_once();
    }
}

#[test]
fn panic_in_retain_and_dedup_keeps_a_consistent_vector() {
    for n in [4, 9] {
        let log = Log::new();
        let mut v = tracked::<[Tracked; 4]>(&log, n);
        let mut seen = 0;
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            v.retain(|t| {
                seen += 1;
                assert!(seen < 3, "predicate panics");
                t.id % 2 == 1
            });
        }));
        assert!(outcome.is_err());
        // Visited: ids 0 (dropped), 1 (kept); unvisited 2.. keep their order.
        let ids: Vec<usize> = v.iter().map(|t| t.id).collect();
        assert_eq!(ids, std::iter::once(1).chain(2..n).collect::<Vec<_>>());
        let mut seen = 0;
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            v.dedup_by(|_, _| {
                seen += 1;
                assert!(seen < 2, "comparison panics");
                true
            });
        }));
        assert!(outcome.is_err());
        let ids: Vec<usize> = v.iter().map(|t| t.id).collect();
        assert_eq!(ids, std::iter::once(1).chain(3..n).collect::<Vec<_>>());
        drop(v);
        log.assert_each_dropped_once();
    }
}

#[test]
fn panicking_drop_no_double_free() {
    for n in [3, 9] {
        let victim = 1;
        // Dropping the whole vector.
        let log = Log::new();
        let v = tracked::<[Tracked; 4]>(&log, n);
        log.panic_on_drop.set(Some(victim));
        assert!(catch_unwind(AssertUnwindSafe(|| drop(v))).is_err());
        log.assert_each_dropped_once();
        // Truncating.
        let log = Log::new();
        let mut v = tracked::<[Tracked; 4]>(&log, n);
        log.panic_on_drop.set(Some(victim));
        assert!(catch_unwind(AssertUnwindSafe(|| v.truncate(0))).is_err());
        assert!(v.is_empty());
        drop(v);
        log.assert_each_dropped_once();
        // Retaining (the rejected element panics as it is dropped).
        let log = Log::new();
        let mut v = tracked::<[Tracked; 4]>(&log, n);
        log.panic_on_drop.set(Some(victim));
        assert!(catch_unwind(AssertUnwindSafe(|| v.retain(|t| t.id != victim))).is_err());
        assert_eq!(v.len(), n - 1);
        drop(v);
        log.assert_each_dropped_once();
        // Dropping a partially consumed owning iterator.
        let log = Log::new();
        let mut it = tracked::<[Tracked; 4]>(&log, n).into_iter();
        drop(it.next());
        log.panic_on_drop.set(Some(victim));
        assert!(catch_unwind(AssertUnwindSafe(|| drop(it))).is_err());
        log.assert_each_dropped_once();
    }
}

#[test]
fn forget_into_iter_no_ub() {
    use std::sync::atomic::{AtomicPtr, Ordering};
    // The forgotten iterators stay reachable from here, so Miri's leak checker
    // reads the leak as the deliberate one it is.
    static FORGOTTEN: [AtomicPtr<u8>; 3] = [const { AtomicPtr::new(std::ptr::null_mut()) }; 3];
    for (slot, n) in [3, 9].into_iter().enumerate() {
        let log = Log::new();
        let mut it = tracked::<[Tracked; 4]>(&log, n).into_iter();
        drop(it.next());
        // `Box::leak` never runs the destructor: the iterator is forgotten
        // exactly as by `mem::forget`, with its window half consumed.
        let forgotten = Box::leak(Box::new(it));
        FORGOTTEN[slot].store(std::ptr::from_mut(forgotten).cast(), Ordering::SeqCst);
        // The yielded element was dropped once; the rest are leaked, not freed.
        assert_eq!(log.drops_of(0), 1);
        for id in 1..n {
            assert_eq!(log.drops_of(id), 0);
        }
    }
    // Forgetting an owning `SmallVec` leaks the same way.
    let log = Log::new();
    let v = tracked::<[Tracked; 2]>(&log, 5);
    let forgotten = Box::leak(Box::new(v));
    FORGOTTEN[2].store(std::ptr::from_mut(forgotten).cast(), Ordering::SeqCst);
    assert!((0..5).all(|id| log.drops_of(id) == 0));
}

#[test]
fn zst_elements() {
    let mut v: SmallVec<[(); 4]> = SmallVec::new();
    for _ in 0..10 {
        v.push(());
    }
    assert_eq!(v.len(), 10);
    // Zero-sized elements occupy no storage, so they never spill.
    assert!(!v.spilled());
    assert_eq!(v.capacity(), usize::MAX - 1);
    assert_eq!(v.pop(), Some(()));
    v.insert(3, ());
    v.remove(0);
    v.retain(|()| true);
    v.dedup();
    assert_eq!(v.len(), 1);
    v.extend(std::iter::repeat_n((), 20));
    v.truncate(2);
    v.shrink_to_fit();
    assert!(!v.spilled());
    assert_eq!(v.clone().into_iter().count(), 2);
    assert_eq!(v.into_vec().len(), 2);
    let zero: SmallVec<[(); 0]> = crate::smallvec![(); 5];
    assert_eq!(zero.len(), 5);
    let from_vec = SmallVec::<[(); 2]>::from_vec(vec![(); 9]);
    assert_eq!(from_vec.len(), 9);
    assert!(!from_vec.spilled());
    // The one refusal: more zero-sized elements than the biased word records.
    let mut full = SmallVec::<[(); 2]>::new();
    full.extend(std::iter::repeat_n((), 3));
    assert!(catch_unwind(AssertUnwindSafe(|| full.reserve(usize::MAX - 3))).is_err());
    full.reserve(usize::MAX - 4);
    assert_eq!(full.len(), 3);

    // Zero-sized elements with drop glue are still dropped once each.
    thread_local!(static DROPS: Cell<usize> = const { Cell::new(0) });
    struct Counted;
    impl Drop for Counted {
        fn drop(&mut self) {
            DROPS.with(|d| d.set(d.get() + 1));
        }
    }
    DROPS.with(|d| d.set(0));
    let mut v: SmallVec<[Counted; 2]> = (0..7).map(|_| Counted).collect();
    v.truncate(5);
    let mut it = v.into_iter();
    drop(it.next());
    drop(it);
    assert_eq!(DROPS.with(Cell::get), 7);
}

#[test]
fn overaligned_elements() {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[repr(align(64))]
    struct Wide(u8);
    fn aligned(v: &[Wide]) -> bool {
        v.iter().all(|w| std::ptr::from_ref(w).addr() % 64 == 0)
    }
    let mut v: SmallVec<[Wide; 3]> = SmallVec::new();
    for i in 0..3 {
        v.push(Wide(i));
        assert!(aligned(&v));
    }
    assert!(!v.spilled());
    v.push(Wide(3));
    assert!(v.spilled());
    assert!(aligned(&v));
    v.truncate(2);
    v.shrink_to_fit();
    assert!(!v.spilled());
    assert!(aligned(&v));
    assert_eq!(&v[..], &[Wide(0), Wide(1)]);
    let copy = SmallVec::<[Wide; 3]>::from_slice(&v);
    assert!(aligned(&copy));
    assert_eq!(copy.into_iter().collect::<Vec<_>>(), [Wide(0), Wide(1)]);
}

#[test]
fn covariance_compiles() {
    fn shorten<'a>(v: SmallVec<[&'static str; 2]>) -> SmallVec<[&'a str; 2]> {
        v
    }
    fn shorten_iter<'a>(it: IntoIter<[&'static str; 2]>) -> IntoIter<[&'a str; 2]> {
        it
    }
    fn shorten_ref<'s, 'a>(v: &'s SmallVec<[&'static str; 2]>) -> &'s SmallVec<[&'a str; 2]> {
        v
    }
    let v: SmallVec<[&'static str; 2]> = crate::smallvec!["a", "b", "c"];
    assert_eq!(shorten_ref(&v).len(), 3);
    let local = String::from("d");
    let mut w = shorten(v);
    w.push(&local);
    assert_eq!(&w[..], &["a", "b", "c", "d"]);
    let it = shorten_iter(SmallVec::from_slice(&["x"]).into_iter());
    assert_eq!(it.count(), 1);
}

#[test]
fn send_sync_bounds() {
    fn send<T: Send>() {}
    fn sync<T: Sync>() {}
    send::<SmallVec<[u32; 4]>>();
    sync::<SmallVec<[u32; 4]>>();
    send::<IdVec>();
    sync::<IdVec>();
    send::<IntoIter<[String; 2]>>();
    sync::<IntoIter<[String; 2]>>();
    send::<SmallVec<[Cell<u8>; 2]>>();
    // A vector must be sendable across threads in practice, too.
    let v: SmallVec<[String; 1]> = crate::smallvec![String::from("a"), String::from("b")];
    let joined = std::thread::spawn(move || v.concat()).join();
    assert_eq!(joined.ok().as_deref(), Some("ab"));
}

#[test]
fn wrapping_enums_stay_the_same_size() {
    // An enum around a `SmallVec` stores its discriminant in the niche, so a
    // `Vec` of the enum can be collected into a `Vec` of rows in place.
    enum Row {
        Direct(SmallVec<[Option<u64>; 4]>),
        Other(Vec<u64>),
    }
    assert_eq!(size_of::<Row>(), size_of::<SmallVec<[Option<u64>; 4]>>());
    assert_eq!(size_of::<Option<IdVec>>(), size_of::<IdVec>());
    let rows = vec![Row::Direct(crate::smallvec![Some(1)]), Row::Other(vec![2])];
    let before = rows.as_ptr().addr();
    let collected: Vec<SmallVec<[Option<u64>; 4]>> = rows
        .into_iter()
        .map(|row| match row {
            Row::Direct(row) => row,
            Row::Other(values) => values.into_iter().map(Some).collect(),
        })
        .collect();
    assert_eq!(collected.as_ptr().addr(), before, "the buffer was reused");
    assert_eq!(&collected[1][..], &[Some(2)]);
}

#[test]
fn no_larger_than_needed() {
    // One word plus the larger of the inline array and a (pointer, length)
    // pair; the niche lives inside the biased word, not in an extra field.
    let word = size_of::<usize>();
    assert_eq!(size_of::<SmallVec<[u64; 1]>>(), 3 * word);
    assert_eq!(size_of::<SmallVec<[(); 1]>>(), 3 * word);
    assert_eq!(size_of::<SmallVec<[u8; 1]>>(), 3 * word);
    assert_eq!(size_of::<Option<SmallVec<[u64; 1]>>>(), 3 * word);
    assert_eq!(size_of::<Option<SmallVec<[(); 1]>>>(), 3 * word);
    assert_eq!(size_of::<Option<IdVec>>(), size_of::<IdVec>());
    if word == 8 {
        assert_eq!(size_of::<SmallVec<[u64; 1]>>(), 24);
        assert_eq!(size_of::<SmallVec<[(); 1]>>(), 24);
        assert_eq!(size_of::<IdVec>(), 24);
        assert_eq!(size_of::<SmallVec<[Option<u64>; 4]>>(), 72);
    }
}

#[test]
fn capacity_overflow_panics() {
    // Inline: the required length overflows `usize`.
    let mut v: SmallVec<[u64; 4]> = crate::smallvec![1];
    assert!(catch_unwind(AssertUnwindSafe(|| v.reserve(usize::MAX))).is_err());
    assert_eq!(&v[..], &[1]);
    // Spilled: the heap buffer cannot grow that far.
    let mut v: SmallVec<[u64; 1]> = crate::smallvec![1, 2];
    assert!(catch_unwind(AssertUnwindSafe(|| v.reserve(usize::MAX - 8))).is_err());
    assert_eq!(&v[..], &[1, 2]);
    assert!(catch_unwind(|| SmallVec::<[u64; 1]>::with_capacity(usize::MAX)).is_err());
    // The neighbouring valid requests still succeed.
    v.reserve(100);
    assert!(v.capacity() >= 102);
    let mut w: SmallVec<[u64; 4]> = crate::smallvec![1];
    w.reserve(3);
    assert!(!w.spilled());
    w.reserve(4);
    assert!(w.spilled());
    assert_eq!(&w[..], &[1]);
}

#[test]
fn macro_forms() {
    let empty: SmallVec<[u8; 2]> = crate::smallvec![];
    assert!(empty.is_empty());
    let one: SmallVec<[u8; 2]> = crate::smallvec![7];
    assert_eq!(&one[..], &[7]);
    let trailing: SmallVec<[u8; 2]> = crate::smallvec![1, 2, 3,];
    assert_eq!(&trailing[..], &[1, 2, 3]);
    assert!(trailing.spilled());
    let width = 5;
    let rows: SmallVec<[Option<u32>; 4]> = crate::smallvec![None; width];
    assert_eq!(&rows[..], &[None; 5]);
    let zero: SmallVec<[String; 2]> = crate::smallvec![String::from("x"); 0];
    assert!(zero.is_empty());
}

#[test]
fn id_rows_behave_as_slices() {
    use crate::TermId;
    let ids: Vec<TermId> = (0..6).map(TermId::from_index).collect();
    let row = IdVec::from_slice(&ids[..3]);
    assert!(!row.spilled());
    assert_eq!(hash_of(&row), hash_of(&ids[..3]));
    let long: IdVec = ids.iter().copied().collect();
    assert!(long.spilled());
    assert!(row < long);
    let mut set = crate::FastSet::default();
    set.insert(row);
    assert!(set.contains(&IdVec::from_slice(&ids[..3])));
    assert_eq!(IdVec::default(), IdVec::new());
}

#[test]
fn fallible_reservation_preserves_inline_and_spilled_owned_values_on_refusal() {
    let mut values: SmallVec<[String; 2]> = SmallVec::from_array(["first".into(), "second".into()]);
    let inline_capacity = values.capacity();
    assert!(matches!(
        values.try_reserve_exact(usize::MAX),
        Err(super::SmallVecReserveError::CapacityOverflow)
    ));
    assert_eq!(values.capacity(), inline_capacity);
    assert_eq!(values.as_slice(), &["first", "second"]);
    values.try_reserve_exact(3).unwrap();
    values.push("third".into());
    let spilled_capacity = values.capacity();
    assert!(values.spilled());
    assert!(matches!(
        values.try_reserve_exact(usize::MAX),
        Err(super::SmallVecReserveError::CapacityOverflow)
    ));
    assert_eq!(values.capacity(), spilled_capacity);
    assert_eq!(values.as_slice(), &["first", "second", "third"]);
}
