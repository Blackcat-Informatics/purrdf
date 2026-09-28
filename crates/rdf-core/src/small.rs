// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared small-vector primitives for the workspace's hot, short-lived id rows.
//!
//! [`SmallVec`] stores its first `N` elements inline (no heap allocation) and
//! spills to a heap buffer only when it grows past that inline capacity, which
//! suits the many tiny, transient id vectors and solution rows on the IR and
//! evaluator paths. The type parameter is spelled as the inline array,
//! `SmallVec<[T; N]>`, and the crate-root macro invoked as `smallvec![a, b]`
//! builds one the way `vec!` builds a `Vec`.
//!
//! Only the generic [`IdVec`] alias lives here. Domain-named aliases (solution
//! rows, path frontiers, …) belong to the downstream crate that owns the
//! concept, not to this shared kernel module.
//!
//! # Representation and invariants
//!
//! A `SmallVec<[T; N]>` is one word, `word`, and a union of the inline array
//! (`MaybeUninit<[T; N]>`) with a `(pointer, length)` pair for a heap buffer.
//! Let `C` be the inline capacity: `N` for sized `T`, `usize::MAX - 1` for
//! zero-sized `T` (which never needs heap storage).
//!
//! * **Biased storage.** `word` and the heap length are stored plus one, in a
//!   `NonZeroUsize` and a `usize` respectively. Every stored value is therefore
//!   non-zero, which gives the type a niche: an enum wrapping a `SmallVec` is no
//!   larger than it. The bias never overflows, because no recorded value reaches
//!   `usize::MAX` (sized capacities are bounded by `isize::MAX`, zero-sized
//!   lengths by `C`).
//! * **Spilled ⇔ `word > C`.** The state is decided by that one comparison.
//! * **Inline** (`word <= C`): `word` is the length, the first `word` inline
//!   slots are initialised and the rest are not.
//! * **Spilled** (`word > C`): the pointer, the length and `word` (the
//!   capacity) are exactly the parts of a `Vec<T>` this value owns.
//! * **No small heap buffers.** A heap buffer whose capacity is `<= C` is never
//!   kept: every path that would adopt one moves its elements inline and frees
//!   it, which is what keeps the one-comparison state test unambiguous. A
//!   zero-sized `T` is always adopted inline; more than `C` of them is refused
//!   as a capacity overflow.
//!
//! Every mutation takes the data pointer and the length pointer from one
//! `&mut self` in one step and then works through those raw pointers only, and
//! the recorded length never covers a slot that is not initialised: an element
//! leaves the recorded length before it is moved out or dropped. A panic in a
//! caller's `Clone`, `Iterator`, predicate or `Drop` can therefore leak
//! elements but never drop one twice or expose an uninitialised slot.

use core::borrow::{Borrow, BorrowMut};
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::iter::{self, FusedIterator};
use core::mem::{ManuallyDrop, MaybeUninit};
use core::num::NonZeroUsize;
use core::ops::{Deref, DerefMut, Index, IndexMut};
use core::ptr::{self, NonNull};
use core::slice::{self, SliceIndex};

mod sealed {
    /// Restricts [`Array`](super::Array) to real `[T; N]` arrays, whose layout
    /// (`N` contiguous `T`s at `T`'s alignment) the inline storage relies on.
    pub trait Sealed {}

    impl<T, const N: usize> Sealed for [T; N] {}
}

/// The inline storage shape of a [`SmallVec`]: implemented for every `[T; N]`
/// and nothing else.
pub trait Array: sealed::Sealed {
    /// The element type `T`.
    type Item;
    /// The inline capacity `N`.
    const CAPACITY: usize;
}

impl<T, const N: usize> Array for [T; N] {
    type Item = T;
    const CAPACITY: usize = N;
}

/// The two storage states sharing one allocation-free slot.
union Data<A> {
    /// Active when `word <= N`: the inline slots.
    inline: ManuallyDrop<MaybeUninit<A>>,
    /// Active when `word > N`: the heap buffer (typed `A` so the container stays
    /// covariant; cast to the element type at every use) and the length.
    heap: (NonNull<A>, usize),
}

/// A vector that keeps up to `N` elements inline before spilling to the heap.
///
/// Written `SmallVec<[T; N]>`. It dereferences to `[T]`, so the whole slice API
/// applies, and its `Hash`, `Eq` and `Ord` are exactly those of the slice.
///
/// ```
/// use purrdf_core::SmallVec;
///
/// let mut row: SmallVec<[u32; 2]> = purrdf_core::smallvec![1, 2];
/// assert!(!row.spilled());
/// row.push(3);
/// assert!(row.spilled());
/// assert_eq!(&row[..], &[1, 2, 3]);
/// ```
pub struct SmallVec<A: Array> {
    /// The biased word, `word + 1`: inline, `word` is the length; spilled, the
    /// heap capacity. Storing it biased gives the type a niche (the zero bit
    /// pattern) at no size cost, so an enum wrapping a `SmallVec` keeps its
    /// discriminant here and is no larger than the vector.
    tag: NonZeroUsize,
    data: Data<A>,
}

// SAFETY: a `SmallVec` owns its elements exactly as a `Vec` does (inline or
// through a uniquely owned heap buffer) and shares no state, so sending it or
// sharing a reference to it is sound exactly when the same is true of `T`.
#[allow(
    clippy::non_send_fields_in_send_ty,
    reason = "the raw heap pointer in `data` is uniquely owned storage for `A::Item`s, which the bound covers"
)]
unsafe impl<A: Array> Send for SmallVec<A> where A::Item: Send {}
// SAFETY: as above; `&SmallVec` only hands out `&T`.
unsafe impl<A: Array> Sync for SmallVec<A> where A::Item: Sync {}

/// Where a vector's length lives: a `usize` holding the length plus one (the
/// inline arm's biased word, or the heap arm's biased length), so both arms are
/// read and written the same way.
#[derive(Clone, Copy)]
struct LenSlot(*mut usize);

impl LenSlot {
    /// The length.
    ///
    /// # Safety
    ///
    /// The slot must belong to a live vector with no access to it since the
    /// slot was taken.
    #[inline]
    unsafe fn get(self) -> usize {
        // SAFETY: caller; the stored value is at least 1.
        unsafe { *self.0 - 1 }
    }

    /// Records `len`, which never exceeds the capacity (`< usize::MAX`), so the
    /// stored `len + 1` neither overflows nor is zero.
    ///
    /// # Safety
    ///
    /// As [`Self::get`], and the first `len` slots must be initialised.
    #[inline]
    unsafe fn set(self, len: usize) {
        // SAFETY: caller.
        unsafe { *self.0 = len + 1 };
    }
}

/// Stores a locally tracked length back when dropped, including on unwind.
struct LenOnDrop {
    slot: LenSlot,
    len: usize,
}

impl Drop for LenOnDrop {
    #[inline]
    fn drop(&mut self) {
        // SAFETY: `slot` is the length slot of a live `SmallVec`, obtained
        // from `raw_mut` with no other access to that `SmallVec` since, and the
        // first `len` slots were initialised by the owner of this guard.
        unsafe { self.slot.set(self.len) };
    }
}

/// Compacts a buffer in place: elements `[0, write)` are kept, `[write, read)`
/// are vacated, `[read, orig)` are not yet visited. On drop (normal or unwind)
/// the unvisited tail is moved down and the kept length recorded.
struct Compactor<T> {
    ptr: *mut T,
    slot: LenSlot,
    orig: usize,
    read: usize,
    write: usize,
}

impl<T> Drop for Compactor<T> {
    fn drop(&mut self) {
        let tail = self.orig - self.read;
        // SAFETY: `[read, orig)` are initialised and `write <= read`, so the
        // (possibly overlapping) move stays inside the buffer; afterwards
        // exactly `[0, write + tail)` are initialised.
        unsafe {
            if tail > 0 && self.read != self.write {
                ptr::copy(self.ptr.add(self.read), self.ptr.add(self.write), tail);
            }
            self.slot.set(self.write + tail);
        }
    }
}

/// Holds a spilled `SmallVec`'s buffer as a `Vec` while a `Vec` operation runs,
/// and re-adopts it (in whatever state it ends) when dropped, including on
/// unwind, so the `SmallVec` never keeps a stale pointer.
struct HeapGuard<'a, A: Array> {
    owner: &'a mut SmallVec<A>,
    vec: ManuallyDrop<Vec<A::Item>>,
}

impl<A: Array> Drop for HeapGuard<'_, A> {
    fn drop(&mut self) {
        // SAFETY: the `Vec` was taken out of `owner`, whose recorded state is
        // therefore owned by the `Vec` alone; it is taken exactly once, here.
        unsafe {
            let vec = ManuallyDrop::take(&mut self.vec);
            self.owner.adopt_vec(vec);
        }
    }
}

impl<A: Array> SmallVec<A> {
    /// `true` for zero-sized elements, which never need heap storage.
    const IS_ZST: bool = size_of::<A::Item>() == 0;

    /// The capacity without spilling: `N`, or for zero-sized elements every
    /// length the biased word can record.
    const INLINE: usize = if Self::IS_ZST {
        usize::MAX - 1
    } else {
        A::CAPACITY
    };

    /// The unbiased word.
    #[inline]
    const fn word(&self) -> usize {
        self.tag.get() - 1
    }

    /// Stores `word`, which is at most `usize::MAX - 1` (an inline length, or
    /// a non-zero-sized heap capacity, bounded by `isize::MAX`).
    #[inline]
    const fn set_word(&mut self, word: usize) {
        self.tag = NonZeroUsize::MIN.saturating_add(word);
    }

    /// An empty vector with inline storage; allocates nothing.
    #[inline]
    pub const fn new() -> Self {
        Self {
            tag: NonZeroUsize::MIN,
            data: Data {
                inline: ManuallyDrop::new(MaybeUninit::uninit()),
            },
        }
    }

    /// An empty vector able to hold `capacity` elements without reallocating.
    /// Allocates only when `capacity > N`.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        let mut v = Self::new();
        v.reserve_exact(capacity);
        v
    }

    /// Takes over a `Vec`'s buffer; if its capacity is at most `N` the elements
    /// move inline and the buffer is freed.
    pub fn from_vec(vec: Vec<A::Item>) -> Self {
        let mut v = Self::new();
        // SAFETY: a new `SmallVec` owns nothing that could be overwritten.
        unsafe { v.adopt_vec(vec) };
        v
    }

    /// A full inline vector holding the array's `N` elements.
    #[inline]
    pub const fn from_buf(buf: A) -> Self {
        assert!(A::CAPACITY <= Self::INLINE, "capacity overflow");
        Self {
            tag: NonZeroUsize::MIN.saturating_add(A::CAPACITY),
            data: Data {
                inline: ManuallyDrop::new(MaybeUninit::new(buf)),
            },
        }
    }

    /// A vector holding the `M` elements of `items`, in order: inline when
    /// `M <= N`, otherwise in one exactly sized heap buffer.
    pub fn from_array<const M: usize>(items: [A::Item; M]) -> Self {
        let items = ManuallyDrop::new(items);
        if M <= Self::INLINE {
            let mut v = Self::new();
            // SAFETY: `v` is inline with room for `N >= M` elements; the `M`
            // values are moved (the source is never dropped) and the length
            // is recorded afterwards.
            unsafe {
                ptr::copy_nonoverlapping(items.as_ptr(), v.inline_mut_ptr(), M);
            }
            v.set_word(M);
            v
        } else {
            Self::from_vec(Vec::from(ManuallyDrop::into_inner(items)))
        }
    }

    /// `n` clones of `elem` (the last slot receives `elem` itself).
    pub fn from_elem(elem: A::Item, n: usize) -> Self
    where
        A::Item: Clone,
    {
        let mut v = Self::with_capacity(n);
        v.extend(iter::repeat_n(elem, n));
        v
    }

    /// A copy of `items`, moved in with one bulk copy.
    pub fn from_slice(items: &[A::Item]) -> Self
    where
        A::Item: Copy,
    {
        let n = items.len();
        if n <= Self::INLINE {
            let mut v = Self::new();
            // SAFETY: `v` is inline with room for `N >= n` elements; `Copy`
            // values may be duplicated bitwise; the length follows the write.
            unsafe {
                ptr::copy_nonoverlapping(items.as_ptr(), v.inline_mut_ptr(), n);
            }
            v.set_word(n);
            v
        } else {
            Self::from_vec(items.to_vec())
        }
    }

    /// The number of elements.
    #[inline]
    pub fn len(&self) -> usize {
        if self.spilled() {
            // SAFETY: spilled, so `heap` is the active field.
            unsafe { self.data.heap.1 - 1 }
        } else {
            self.word()
        }
    }

    /// `true` when there are no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The number of elements that fit without reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        if self.spilled() {
            self.word()
        } else {
            Self::INLINE
        }
    }

    /// The inline capacity `N`.
    #[inline]
    pub const fn inline_size(&self) -> usize {
        A::CAPACITY
    }

    /// `true` when the elements live in a heap buffer.
    #[inline]
    pub const fn spilled(&self) -> bool {
        self.word() > Self::INLINE
    }

    /// The inline slots, read-only. Meaningful only while inline.
    #[inline]
    fn inline_ptr(&self) -> *const A::Item {
        (&raw const self.data.inline).cast()
    }

    /// The inline slots, writable. Meaningful only while inline.
    #[inline]
    fn inline_mut_ptr(&mut self) -> *mut A::Item {
        (&raw mut self.data.inline).cast()
    }

    /// A pointer to the first element (dangling but aligned when empty and
    /// zero-capacity).
    #[inline]
    pub fn as_ptr(&self) -> *const A::Item {
        if self.spilled() {
            // SAFETY: spilled, so `heap` is the active field.
            unsafe { self.data.heap.0.as_ptr().cast_const().cast() }
        } else {
            self.inline_ptr()
        }
    }

    /// A mutable pointer to the first element.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut A::Item {
        self.raw_mut().0
    }

    /// The data pointer, a pointer to the length slot, and the capacity, all
    /// taken from one borrow of `self` and derived from disjoint fields, so
    /// both pointers stay usable together until `self` is next used.
    #[inline]
    fn raw_mut(&mut self) -> (*mut A::Item, LenSlot, usize) {
        if self.spilled() {
            // SAFETY: spilled, so `heap` is the active field.
            let ptr = unsafe { self.data.heap.0 }.as_ptr().cast::<A::Item>();
            // SAFETY: a raw borrow of the active field's length reads nothing.
            let len_ptr = unsafe { &raw mut self.data.heap.1 };
            (ptr, LenSlot(len_ptr), self.word())
        } else {
            // `NonZeroUsize` is `repr(transparent)` over `usize`, and the slot
            // only ever stores `len + 1 >= 1`, so the tag stays valid.
            let len_ptr = (&raw mut self.tag).cast::<usize>();
            (self.inline_mut_ptr(), LenSlot(len_ptr), Self::INLINE)
        }
    }

    /// The heap buffer as the `Vec` it came from.
    ///
    /// # Safety
    ///
    /// `self` must be spilled, and the returned `Vec` then owns the elements
    /// and buffer: `self` must be re-adopted over or never used again.
    #[inline]
    unsafe fn take_heap_vec(&mut self) -> Vec<A::Item> {
        // SAFETY: spilled (caller), so `heap` is active and `(ptr, len, word)`
        // are the unchanged parts of a `Vec<A::Item>`.
        unsafe {
            let (ptr, biased_len) = self.data.heap;
            Vec::from_raw_parts(ptr.as_ptr().cast(), biased_len - 1, self.word())
        }
    }

    /// Makes `vec` this vector's contents, keeping its buffer when its
    /// capacity exceeds `N` and moving its elements inline otherwise.
    ///
    /// # Safety
    ///
    /// Whatever `self` currently records is overwritten without being dropped:
    /// it must already be owned elsewhere (or be empty and inline).
    unsafe fn adopt_vec(&mut self, vec: Vec<A::Item>) {
        // Zero-sized elements are always held inline, which records at most
        // `INLINE` of them; checked before anything moves, so a refusal drops
        // the `Vec` normally.
        assert!(
            vec.len() <= Self::INLINE || !Self::IS_ZST,
            "capacity overflow"
        );
        let mut vec = ManuallyDrop::new(vec);
        let len = vec.len();
        let cap = vec.capacity();
        if !Self::IS_ZST && cap > Self::INLINE {
            // SAFETY: a `Vec`'s pointer is never null.
            let ptr = unsafe { NonNull::new_unchecked(vec.as_mut_ptr()) };
            // A non-zero-sized `Vec`'s capacity is at most `isize::MAX`, so
            // the biased length and word cannot overflow.
            self.data.heap = (ptr.cast(), len + 1);
            self.set_word(cap);
        } else {
            let dst = self.inline_mut_ptr();
            // SAFETY: `len <= INLINE` fits inline (for sized elements
            // `len <= cap <= N`; zero-sized ones occupy no bytes and were
            // bounded above); the elements move out of
            // the `Vec` (its length is zeroed before it is freed, so they are
            // not dropped there), and the buffers are distinct allocations.
            unsafe {
                ptr::copy_nonoverlapping(vec.as_ptr(), dst, len);
                vec.set_len(0);
                ManuallyDrop::drop(&mut vec);
            }
            self.set_word(len);
        }
    }

    /// Runs `f` on the spilled buffer as a `Vec`, re-adopting it afterwards
    /// even if `f` panics.
    fn with_heap<R>(&mut self, f: impl FnOnce(&mut Vec<A::Item>) -> R) -> R {
        debug_assert!(self.spilled());
        // SAFETY: spilled (callers check); the guard re-adopts the `Vec`.
        let vec = unsafe { self.take_heap_vec() };
        let mut guard = HeapGuard {
            owner: self,
            vec: ManuallyDrop::new(vec),
        };
        f(&mut guard.vec)
    }

    /// Ensures room for `additional` more elements, growing geometrically.
    ///
    /// # Panics
    ///
    /// On capacity overflow.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        let len = self.len();
        if self.capacity() - len < additional {
            self.grow(len, additional, true);
        }
    }

    /// Ensures room for exactly `additional` more elements.
    ///
    /// # Panics
    ///
    /// On capacity overflow.
    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        let len = self.len();
        if self.capacity() - len < additional {
            self.grow(len, additional, false);
        }
    }

    #[cold]
    #[inline(never)]
    fn grow(&mut self, len: usize, additional: usize, amortized: bool) {
        let required = len.checked_add(additional).expect("capacity overflow");
        // Zero-sized elements are inline with capacity `INLINE`, and growth is
        // only asked for beyond the current capacity.
        assert!(!Self::IS_ZST, "capacity overflow");
        // Amortized growth rounds up to a power of two, so the capacity
        // sequence is the same whether it grows by pushes or by reserves and
        // does not depend on the heap `Vec`'s own growth policy.
        let cap = if amortized {
            required.checked_next_power_of_two().unwrap_or(required)
        } else {
            required
        };
        if self.spilled() {
            self.with_heap(|vec| vec.reserve_exact(cap - len));
        } else {
            // Inline and `required > N`: move to a heap buffer larger than `N`.
            let mut vec = Vec::with_capacity(cap);
            // SAFETY: inline, so the first `len` slots are initialised; they
            // move into the new buffer (capacity >= required > len) and `self`
            // is then overwritten, never dropping the moved-from slots.
            unsafe {
                ptr::copy_nonoverlapping(self.inline_ptr(), vec.as_mut_ptr(), len);
                vec.set_len(len);
                self.adopt_vec(vec);
            }
        }
    }

    /// Frees unused heap capacity; moves the elements back inline when they
    /// fit.
    pub fn shrink_to_fit(&mut self) {
        if !self.spilled() {
            return;
        }
        let len = self.len();
        if len <= Self::INLINE {
            // SAFETY: spilled, so the `Vec` takes sole ownership; its `len <= N`
            // elements move inline (a distinct allocation), `self` records them
            // as inline, and the emptied buffer is freed without dropping them.
            unsafe {
                let mut vec = self.take_heap_vec();
                ptr::copy_nonoverlapping(vec.as_ptr(), self.inline_mut_ptr(), len);
                vec.set_len(0);
                self.set_word(len);
                drop(vec);
            }
        } else {
            self.with_heap(Vec::shrink_to_fit);
        }
    }

    /// Appends `value`.
    #[inline]
    pub fn push(&mut self, value: A::Item) {
        let (mut ptr, mut len_ptr, mut cap) = self.raw_mut();
        // SAFETY: `len_ptr` is live (from `raw_mut`, nothing used since).
        let len = unsafe { len_ptr.get() };
        if len == cap {
            self.grow(len, 1, true);
            (ptr, len_ptr, cap) = self.raw_mut();
        }
        debug_assert!(len < cap);
        // SAFETY: `len < cap`, so slot `len` is in bounds and uninitialised;
        // it is written before the length covers it.
        unsafe {
            ptr.add(len).write(value);
            len_ptr.set(len + 1);
        }
    }

    /// Removes and returns the last element.
    #[inline]
    pub fn pop(&mut self) -> Option<A::Item> {
        let (ptr, len_ptr, _) = self.raw_mut();
        // SAFETY: the length is shortened before the last element is read out.
        unsafe {
            let len = len_ptr.get();
            if len == 0 {
                return None;
            }
            len_ptr.set(len - 1);
            Some(ptr.add(len - 1).read())
        }
    }

    /// Inserts `value` at `index`, shifting later elements right.
    ///
    /// # Panics
    ///
    /// When `index > len`.
    pub fn insert(&mut self, index: usize, value: A::Item) {
        let len = self.len();
        assert!(
            index <= len,
            "insertion index (is {index}) should be <= len (is {len})"
        );
        self.reserve(1);
        let (ptr, len_ptr, _) = self.raw_mut();
        // SAFETY: capacity >= len + 1, so shifting `[index, len)` up by one
        // stays in bounds; slot `index` is then overwritten without a drop.
        unsafe {
            let at = ptr.add(index);
            ptr::copy(at, at.add(1), len - index);
            at.write(value);
            len_ptr.set(len + 1);
        }
    }

    /// Inserts `items` at `index` with bulk copies.
    ///
    /// # Panics
    ///
    /// When `index > len`, or on capacity overflow.
    pub fn insert_from_slice(&mut self, index: usize, items: &[A::Item])
    where
        A::Item: Copy,
    {
        let len = self.len();
        assert!(
            index <= len,
            "insertion index (is {index}) should be <= len (is {len})"
        );
        let n = items.len();
        self.reserve(n);
        let (ptr, len_ptr, _) = self.raw_mut();
        // SAFETY: capacity >= len + n; the tail shifts up by `n` (overlap
        // allowed), then the `Copy` items fill the gap; `items` cannot alias
        // `self` (it is a shared borrow alongside `&mut self`).
        unsafe {
            let at = ptr.add(index);
            ptr::copy(at, at.add(n), len - index);
            ptr::copy_nonoverlapping(items.as_ptr(), at, n);
            len_ptr.set(len + n);
        }
    }

    /// Removes and returns the element at `index`, shifting later elements
    /// left.
    ///
    /// # Panics
    ///
    /// When `index >= len`.
    pub fn remove(&mut self, index: usize) -> A::Item {
        let len = self.len();
        assert!(
            index < len,
            "removal index (is {index}) should be < len (is {len})"
        );
        let (ptr, len_ptr, _) = self.raw_mut();
        // SAFETY: `index < len`; the element is read out, the tail closes the
        // gap, and the length drops by one.
        unsafe {
            len_ptr.set(len - 1);
            let at = ptr.add(index);
            let value = at.read();
            ptr::copy(at.add(1), at, len - index - 1);
            value
        }
    }

    /// Removes and returns the element at `index`, moving the last element
    /// into its place.
    ///
    /// # Panics
    ///
    /// When `index >= len`.
    pub fn swap_remove(&mut self, index: usize) -> A::Item {
        let len = self.len();
        assert!(
            index < len,
            "swap_remove index (is {index}) should be < len (is {len})"
        );
        let (ptr, len_ptr, _) = self.raw_mut();
        // SAFETY: `index < len`; the element is read out and the last element
        // (a distinct slot unless it is the same one) moves into it.
        unsafe {
            len_ptr.set(len - 1);
            let at = ptr.add(index);
            let value = at.read();
            if index != len - 1 {
                ptr::copy_nonoverlapping(ptr.add(len - 1), at, 1);
            }
            value
        }
    }

    /// Drops the elements from `new_len` on; no effect when `new_len >= len`.
    pub fn truncate(&mut self, new_len: usize) {
        let (ptr, len_ptr, _) = self.raw_mut();
        // SAFETY: the length is shortened first, then the tail — initialised
        // and now unrecorded — is dropped in place exactly once.
        unsafe {
            let len = len_ptr.get();
            if new_len >= len {
                return;
            }
            len_ptr.set(new_len);
            ptr::drop_in_place(ptr::slice_from_raw_parts_mut(
                ptr.add(new_len),
                len - new_len,
            ));
        }
    }

    /// Drops every element, keeping the storage.
    #[inline]
    pub fn clear(&mut self) {
        self.truncate(0);
    }

    /// Appends a copy of `items` with one bulk copy.
    pub fn extend_from_slice(&mut self, items: &[A::Item])
    where
        A::Item: Copy,
    {
        let n = items.len();
        self.reserve(n);
        let (ptr, len_ptr, _) = self.raw_mut();
        // SAFETY: capacity >= len + n; `Copy` items are duplicated bitwise into
        // the uninitialised tail; `items` cannot alias `self`.
        unsafe {
            let len = len_ptr.get();
            ptr::copy_nonoverlapping(items.as_ptr(), ptr.add(len), n);
            len_ptr.set(len + n);
        }
    }

    /// Moves every element of `other` onto the end of `self`, leaving `other`
    /// empty (its storage is kept).
    pub fn append<B: Array<Item = A::Item>>(&mut self, other: &mut SmallVec<B>) {
        let n = other.len();
        self.reserve(n);
        let (src, src_len, _) = other.raw_mut();
        let (dst, dst_len, _) = self.raw_mut();
        // SAFETY: two distinct vectors; capacity >= len + n; the elements move
        // (`other` forgets them by recording length 0).
        unsafe {
            let len = dst_len.get();
            ptr::copy_nonoverlapping(src, dst.add(len), n);
            src_len.set(0);
            dst_len.set(len + n);
        }
    }

    /// Resizes to `new_len`, cloning `value` into new slots or dropping the
    /// excess.
    pub fn resize(&mut self, new_len: usize, value: A::Item)
    where
        A::Item: Clone,
    {
        let len = self.len();
        if new_len > len {
            self.extend(iter::repeat_n(value, new_len - len));
        } else {
            self.truncate(new_len);
        }
    }

    /// Resizes to `new_len`, filling new slots from `f` or dropping the excess.
    pub fn resize_with<F: FnMut() -> A::Item>(&mut self, new_len: usize, f: F) {
        let len = self.len();
        if new_len > len {
            self.extend(iter::repeat_with(f).take(new_len - len));
        } else {
            self.truncate(new_len);
        }
    }

    /// Keeps only the elements for which `keep` returns `true`, in order.
    #[inline]
    pub fn retain<F: FnMut(&mut A::Item) -> bool>(&mut self, keep: F) {
        self.retain_mut(keep);
    }

    /// Keeps only the elements for which `keep` returns `true`, in order.
    pub fn retain_mut<F: FnMut(&mut A::Item) -> bool>(&mut self, mut keep: F) {
        let (ptr, len_ptr, _) = self.raw_mut();
        // SAFETY: `len_ptr` is live.
        let orig = unsafe { len_ptr.get() };
        let mut c = Compactor {
            ptr,
            slot: len_ptr,
            orig,
            read: 0,
            write: 0,
        };
        while c.read < c.orig {
            // SAFETY: slot `read` is initialised and not aliased; a kept
            // element moves down to `write < read` (or stays); a rejected one
            // is counted as visited before it is dropped, so an unwind from its
            // `Drop` (or from `keep`) is repaired by the compactor without
            // touching it again.
            unsafe {
                let cur = c.ptr.add(c.read);
                if keep(&mut *cur) {
                    if c.read != c.write {
                        ptr::copy_nonoverlapping(cur, c.ptr.add(c.write), 1);
                    }
                    c.write += 1;
                    c.read += 1;
                } else {
                    c.read += 1;
                    ptr::drop_in_place(cur);
                }
            }
        }
    }

    /// Removes consecutive elements for which `same(current, previous_kept)`
    /// returns `true`.
    pub fn dedup_by<F: FnMut(&mut A::Item, &mut A::Item) -> bool>(&mut self, mut same: F) {
        let (ptr, len_ptr, _) = self.raw_mut();
        // SAFETY: `len_ptr` is live.
        let orig = unsafe { len_ptr.get() };
        if orig < 2 {
            return;
        }
        let mut c = Compactor {
            ptr,
            slot: len_ptr,
            orig,
            read: 1,
            write: 1,
        };
        while c.read < c.orig {
            // SAFETY: as in `retain_mut`; `write - 1 < read`, so the two
            // `&mut` point at distinct initialised elements.
            unsafe {
                let cur = c.ptr.add(c.read);
                let prev = c.ptr.add(c.write - 1);
                if same(&mut *cur, &mut *prev) {
                    c.read += 1;
                    ptr::drop_in_place(cur);
                } else {
                    if c.read != c.write {
                        ptr::copy_nonoverlapping(cur, c.ptr.add(c.write), 1);
                    }
                    c.write += 1;
                    c.read += 1;
                }
            }
        }
    }

    /// Removes consecutive elements that map to equal keys.
    pub fn dedup_by_key<K: PartialEq, F: FnMut(&mut A::Item) -> K>(&mut self, mut key: F) {
        self.dedup_by(|a, b| key(a) == key(b));
    }

    /// Removes consecutive equal elements.
    pub fn dedup(&mut self)
    where
        A::Item: PartialEq,
    {
        self.dedup_by(|a, b| a == b);
    }

    /// The elements as a slice.
    #[inline]
    pub fn as_slice(&self) -> &[A::Item] {
        self
    }

    /// The elements as a mutable slice.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [A::Item] {
        self
    }

    /// The elements as a `Vec`, reusing the heap buffer when spilled.
    pub fn into_vec(mut self) -> Vec<A::Item> {
        if self.spilled() {
            let mut this = ManuallyDrop::new(self);
            // SAFETY: spilled; `this` is never used or dropped again.
            unsafe { this.take_heap_vec() }
        } else {
            let len = self.word();
            let mut vec = Vec::with_capacity(len);
            // SAFETY: inline, so the first `len` slots are initialised; they
            // move into the new buffer and `self` then records none of them.
            unsafe {
                ptr::copy_nonoverlapping(self.inline_ptr(), vec.as_mut_ptr(), len);
                vec.set_len(len);
            }
            self.set_word(0);
            vec
        }
    }

    /// The elements as a boxed slice.
    pub fn into_boxed_slice(self) -> Box<[A::Item]> {
        self.into_vec().into_boxed_slice()
    }

    /// The inline array, when the vector is inline and exactly full; `self`
    /// otherwise.
    pub fn into_inner(self) -> Result<A, Self> {
        if self.spilled() || self.word() != A::CAPACITY {
            return Err(self);
        }
        let this = ManuallyDrop::new(self);
        // SAFETY: inline and all `N` slots are initialised, so the array is
        // initialised; `this` is never dropped, so each element moves once.
        Ok(unsafe {
            ManuallyDrop::into_inner(ptr::read(&raw const this.data.inline)).assume_init()
        })
    }
}

impl<A: Array> Drop for SmallVec<A> {
    fn drop(&mut self) {
        if self.spilled() {
            // SAFETY: spilled; `self` is not used after this.
            drop(unsafe { self.take_heap_vec() });
        } else {
            let (ptr, len_ptr, _) = self.raw_mut();
            // SAFETY: inline, so the first `len` slots are initialised and are
            // dropped exactly once here.
            unsafe { ptr::drop_in_place(ptr::slice_from_raw_parts_mut(ptr, len_ptr.get())) };
        }
    }
}

impl<A: Array> Deref for SmallVec<A> {
    type Target = [A::Item];

    #[inline]
    fn deref(&self) -> &[A::Item] {
        // Read the storage tag once. `as_ptr()` followed by `len()` would
        // independently dispatch on it twice for every slice coercion in a
        // solution-row compatibility probe.
        if self.spilled() {
            // SAFETY: spilled, so the heap pair is active; its first `len`
            // elements are initialized and the pointer is non-null/aligned.
            let (ptr, biased_len) = unsafe { self.data.heap };
            unsafe { slice::from_raw_parts(ptr.as_ptr().cast(), biased_len - 1) }
        } else {
            // SAFETY: inline, so the first `word` slots are initialized.
            unsafe { slice::from_raw_parts(self.inline_ptr(), self.word()) }
        }
    }
}

impl<A: Array> DerefMut for SmallVec<A> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [A::Item] {
        let (ptr, len_ptr, _) = self.raw_mut();
        // SAFETY: as in `deref`, with unique access through `&mut self`.
        unsafe { slice::from_raw_parts_mut(ptr, len_ptr.get()) }
    }
}

impl<A: Array> Extend<A::Item> for SmallVec<A> {
    fn extend<I: IntoIterator<Item = A::Item>>(&mut self, iterable: I) {
        let mut iter = iterable.into_iter();
        self.reserve(iter.size_hint().0);
        {
            let (ptr, len_ptr, cap) = self.raw_mut();
            let mut guard = LenOnDrop {
                slot: len_ptr,
                // SAFETY: `len_ptr` is live.
                len: unsafe { len_ptr.get() },
            };
            // Fill the reserved slots without a per-element capacity check; the
            // guard records every written element even if `next` panics.
            while guard.len < cap {
                match iter.next() {
                    // SAFETY: `len < cap`, so the slot is in bounds and
                    // uninitialised.
                    Some(value) => unsafe {
                        ptr.add(guard.len).write(value);
                        guard.len += 1;
                    },
                    None => return,
                }
            }
        }
        for value in iter {
            self.push(value);
        }
    }
}

impl<A: Array> FromIterator<A::Item> for SmallVec<A> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = A::Item>>(iterable: I) -> Self {
        let mut v = Self::new();
        v.extend(iterable);
        v
    }
}

impl<A: Array> Default for SmallVec<A> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<A: Array> Clone for SmallVec<A>
where
    A::Item: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        let len = self.len();
        if len > Self::INLINE {
            // `Vec::extend_from_slice` specializes its clone path for Copy
            // cells, and `from_vec` adopts this one exactly sized buffer.
            // Most solution-row clones update existing positions, so spare
            // capacity would add allocator traffic to that hot path.
            let mut values = Vec::with_capacity(len);
            values.extend_from_slice(self.as_slice());
            return Self::from_vec(values);
        }
        let mut out = Self::new();
        let source = self.as_ptr();
        let (destination, slot, _) = out.raw_mut();
        let mut guard = LenOnDrop { slot, len: 0 };
        for index in 0..len {
            // SAFETY: the source's first `len` elements are initialized; the
            // destination is inline with capacity at least `len`. The guard
            // publishes each clone before the next may panic.
            unsafe { destination.add(index).write((*source.add(index)).clone()) };
            guard.len += 1;
        }
        drop(guard);
        out
    }

    fn clone_from(&mut self, source: &Self) {
        self.truncate(source.len());
        let (head, tail) = source.split_at(self.len());
        self.clone_from_slice(head);
        self.extend(tail.iter().cloned());
    }
}

impl<A: Array> fmt::Debug for SmallVec<A>
where
    A::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<A: Array, B: Array> PartialEq<SmallVec<B>> for SmallVec<A>
where
    A::Item: PartialEq<B::Item>,
{
    #[inline]
    fn eq(&self, other: &SmallVec<B>) -> bool {
        self[..] == other[..]
    }
}

impl<A: Array> Eq for SmallVec<A> where A::Item: Eq {}

impl<A: Array> PartialOrd for SmallVec<A>
where
    A::Item: PartialOrd,
{
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self[..].partial_cmp(&other[..])
    }
}

impl<A: Array> Ord for SmallVec<A>
where
    A::Item: Ord,
{
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self[..].cmp(&other[..])
    }
}

impl<A: Array> Hash for SmallVec<A>
where
    A::Item: Hash,
{
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self[..].hash(state);
    }
}

impl<A: Array, I: SliceIndex<[A::Item]>> Index<I> for SmallVec<A> {
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &I::Output {
        &(**self)[index]
    }
}

impl<A: Array, I: SliceIndex<[A::Item]>> IndexMut<I> for SmallVec<A> {
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut I::Output {
        &mut (**self)[index]
    }
}

impl<A: Array> AsRef<[A::Item]> for SmallVec<A> {
    #[inline]
    fn as_ref(&self) -> &[A::Item] {
        self
    }
}

impl<A: Array> AsMut<[A::Item]> for SmallVec<A> {
    #[inline]
    fn as_mut(&mut self) -> &mut [A::Item] {
        self
    }
}

impl<A: Array> Borrow<[A::Item]> for SmallVec<A> {
    #[inline]
    fn borrow(&self) -> &[A::Item] {
        self
    }
}

impl<A: Array> BorrowMut<[A::Item]> for SmallVec<A> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut [A::Item] {
        self
    }
}

impl<A: Array> From<Vec<A::Item>> for SmallVec<A> {
    #[inline]
    fn from(vec: Vec<A::Item>) -> Self {
        Self::from_vec(vec)
    }
}

impl<A: Array> From<&[A::Item]> for SmallVec<A>
where
    A::Item: Clone,
{
    fn from(items: &[A::Item]) -> Self {
        items.iter().cloned().collect()
    }
}

impl<A: Array> From<A> for SmallVec<A> {
    #[inline]
    fn from(buf: A) -> Self {
        Self::from_buf(buf)
    }
}

impl<'a, A: Array> IntoIterator for &'a SmallVec<A> {
    type Item = &'a A::Item;
    type IntoIter = slice::Iter<'a, A::Item>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, A: Array> IntoIterator for &'a mut SmallVec<A> {
    type Item = &'a mut A::Item;
    type IntoIter = slice::IterMut<'a, A::Item>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<A: Array> IntoIterator for SmallVec<A> {
    type Item = A::Item;
    type IntoIter = IntoIter<A>;

    #[inline]
    fn into_iter(mut self) -> IntoIter<A> {
        let (_, len_ptr, _) = self.raw_mut();
        // SAFETY: the elements' ownership passes to the iterator's window; the
        // vector records none of them, so its own drop only frees storage.
        let len = unsafe {
            let len = len_ptr.get();
            len_ptr.set(0);
            len
        };
        IntoIter {
            data: self,
            front: 0,
            back: len,
        }
    }
}

/// An owning iterator over a [`SmallVec`]'s elements.
///
/// Owns the storage (with recorded length 0) and the window `[front, back)` of
/// elements not yet yielded; leaking it with `mem::forget` only leaks them.
pub struct IntoIter<A: Array> {
    data: SmallVec<A>,
    front: usize,
    back: usize,
}

impl<A: Array> IntoIter<A> {
    /// The elements not yet yielded.
    pub fn as_slice(&self) -> &[A::Item] {
        // SAFETY: `[front, back)` are initialised slots of the owned storage.
        unsafe { slice::from_raw_parts(self.data.as_ptr().add(self.front), self.back - self.front) }
    }

    /// The elements not yet yielded, mutably.
    pub fn as_mut_slice(&mut self) -> &mut [A::Item] {
        let (front, back) = (self.front, self.back);
        // SAFETY: as in `as_slice`, with unique access through `&mut self`.
        unsafe { slice::from_raw_parts_mut(self.data.as_mut_ptr().add(front), back - front) }
    }
}

impl<A: Array> Iterator for IntoIter<A> {
    type Item = A::Item;

    #[inline]
    fn next(&mut self) -> Option<A::Item> {
        if self.front == self.back {
            return None;
        }
        let at = self.front;
        self.front += 1;
        // SAFETY: slot `at` was in the window and has just left it, so it is
        // initialised and read out exactly once.
        Some(unsafe { self.data.as_ptr().add(at).read() })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.back - self.front;
        (n, Some(n))
    }
}

impl<A: Array> DoubleEndedIterator for IntoIter<A> {
    #[inline]
    fn next_back(&mut self) -> Option<A::Item> {
        if self.front == self.back {
            return None;
        }
        self.back -= 1;
        // SAFETY: as in `next`, for the slot that just left the window's end.
        Some(unsafe { self.data.as_ptr().add(self.back).read() })
    }
}

impl<A: Array> ExactSizeIterator for IntoIter<A> {}

impl<A: Array> FusedIterator for IntoIter<A> {}

impl<A: Array> Drop for IntoIter<A> {
    fn drop(&mut self) {
        let (front, back) = (self.front, self.back);
        self.front = back;
        let ptr = self.data.as_mut_ptr();
        // SAFETY: the window is emptied first, then its former elements —
        // initialised and owned only here — are dropped once; the storage is
        // freed afterwards by `data`'s own drop.
        unsafe { ptr::drop_in_place(ptr::slice_from_raw_parts_mut(ptr.add(front), back - front)) };
    }
}

impl<A: Array> Clone for IntoIter<A>
where
    A::Item: Clone,
{
    fn clone(&self) -> Self {
        SmallVec::<A>::from_iter(self.as_slice().iter().cloned()).into_iter()
    }
}

impl<A: Array> fmt::Debug for IntoIter<A>
where
    A::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IntoIter").field(&self.as_slice()).finish()
    }
}

/// Builds a [`SmallVec`] the way `vec!` builds a `Vec`.
///
/// * `smallvec![]` — empty.
/// * `smallvec![a, b, c]` — the listed elements (a trailing comma is allowed),
///   moved inline in one bulk copy when they fit.
/// * `smallvec![elem; n]` — `n` clones of `elem` (`n` may be a runtime value).
///
/// ```
/// use purrdf_core::SmallVec;
///
/// let empty: SmallVec<[u8; 4]> = purrdf_core::smallvec![];
/// let listed: SmallVec<[u8; 4]> = purrdf_core::smallvec![1, 2, 3,];
/// let width = 6;
/// let blank: SmallVec<[Option<u8>; 4]> = purrdf_core::smallvec![None; width];
/// assert!(empty.is_empty());
/// assert_eq!(&listed[..], &[1, 2, 3]);
/// assert_eq!(blank.len(), 6);
/// assert!(blank.spilled());
/// ```
#[macro_export]
macro_rules! smallvec {
    () => {
        $crate::SmallVec::new()
    };
    ($elem:expr; $n:expr) => {
        $crate::SmallVec::from_elem($elem, $n)
    };
    ($($item:expr),+ $(,)?) => {
        $crate::SmallVec::from_array([$($item),+])
    };
}

/// A small-vector of interned [`TermId`](crate::TermId)s, inline up to 4 ids.
///
/// The generic id-row primitive: most id sequences on hot paths (quad rows,
/// short frontiers) fit inline, avoiding a heap allocation.
pub type IdVec = SmallVec<[crate::TermId; 4]>;

#[cfg(test)]
#[path = "small_tests.rs"]
mod tests;
