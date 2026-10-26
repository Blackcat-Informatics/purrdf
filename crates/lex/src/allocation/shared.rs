// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fallible immutable shared ownership at the common native allocation home.
//! A sized, immutable shared owner with one fallible allocation, no weak owners,
//! raw extraction, unsizing or mutable payload access. Clone never allocates.

use core::alloc::Layout;
use core::fmt;
use core::marker::PhantomData;
use core::ops::Deref;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering, fence};

mod buffers;
pub use buffers::{SharedBuffer, SharedBufferMut};

fn clone_count(strong: &AtomicUsize) -> Result<(), SharedCloneError> {
    let mut count = strong.load(Ordering::Relaxed);
    loop {
        if count >= usize::MAX / 2 {
            return Err(SharedCloneError);
        }
        match strong.compare_exchange_weak(count, count + 1, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return Ok(()),
            Err(current) => count = current,
        }
    }
}

struct Header<T> {
    strong: AtomicUsize,
    value: T,
}

// Only the uninitialized allocation is owned. The initialized Header<T>
// transfers directly to Shared; unwinding a factory cannot leak this block.
struct UninitializedAllocation {
    pointer: NonNull<u8>,
    layout: Layout,
}
impl Drop for UninitializedAllocation {
    fn drop(&mut self) {
        // SAFETY: this guard owns fresh global-allocator storage for this layout.
        unsafe { std::alloc::dealloc(self.pointer.as_ptr(), self.layout) };
    }
}

/// Allocation refusal while constructing an immutable shared owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SharedAllocError;

impl fmt::Display for SharedAllocError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("shared owner allocation failed")
    }
}
impl std::error::Error for SharedAllocError {}

impl purrdf_hash::error::StructuredError for SharedAllocError {
    purrdf_hash::error_visit! { @record self {} => "shared-allocation-failed" {} }
}

/// A checked shared-owner reference count cannot accept another reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SharedCloneError;

impl fmt::Display for SharedCloneError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("shared owner reference count overflow")
    }
}
impl std::error::Error for SharedCloneError {}

impl purrdf_hash::error::StructuredError for SharedCloneError {
    purrdf_hash::error_visit! { @record self {} => "shared-reference-count-overflow" {} }
}

/// Immutable, sized shared ownership. Construction can report allocator refusal.
pub struct Shared<T> {
    pointer: NonNull<Header<T>>,
    // The allocation owns a T; auto-trait and drop-check obligations follow T.
    _owner: PhantomData<Header<T>>,
}

/// An immutable descendant of one retained shared allocation. Projection is
/// available only from a live owner or another view of that same allocation.
/// The projected value has no independent allocation or destruction path.
pub struct SharedView<T, U> {
    owner: Shared<T>,
    value: NonNull<U>,
}

// SAFETY: the immutable descendant stays inside its retained Shared owner. Both
// the owner's payload and the projected shared reference may cross threads.
unsafe impl<T: Send + Sync, U: Sync> Send for SharedView<T, U> {}
// SAFETY: projections provide shared access only; the original atomic owner
// keeps the entire immutable descendant alive for every retained projection.
unsafe impl<T: Send + Sync, U: Sync> Sync for SharedView<T, U> {}

impl<T, U: fmt::Debug> fmt::Debug for SharedView<T, U> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, formatter)
    }
}

impl<T, U> SharedView<T, U> {
    /// Retain the original immutable allocation without copying its payload.
    #[must_use]
    pub fn owner(&self) -> Shared<T> {
        self.owner.clone()
    }

    /// Select an optional descendant without cloning the owner on a miss.
    #[must_use]
    pub fn try_project<V>(
        &self,
        select: impl for<'a> FnOnce(&'a U) -> Option<&'a V>,
    ) -> Option<SharedView<T, V>> {
        let value = NonNull::from(select(self)?);
        Some(SharedView {
            owner: self.owner.clone(),
            value,
        })
    }
}

impl<T, U> Clone for SharedView<T, U> {
    fn clone(&self) -> Self {
        Self {
            owner: self.owner.clone(),
            value: self.value,
        }
    }
}

impl<T, U> Deref for SharedView<T, U> {
    type Target = U;
    fn deref(&self) -> &U {
        // SAFETY: construction projects from the live Shared<T> payload and
        // subsequent projections use only descendants of an existing view.
        // Shared<T> is immutable after publication and its allocation cannot
        // move while this view retains a strong owner.
        unsafe { self.value.as_ref() }
    }
}

// Moving the owner moves only its pointer, never the allocated payload. Like
// Arc, this pointer is Unpin even when the pointee is not; pinning the owner
// does not structurally pin T. Avoid deriving this property through T's entire
// recursive ownership graph.
impl<T> Unpin for Shared<T> {}

// SAFETY: References expose only &T. Ownership may cross threads only when T
// permits both transfer and shared access. The strong count is atomic.
unsafe impl<T: Send + Sync> Send for Shared<T> {}
// SAFETY: Same immutable access and atomic lifetime protocol as Send above.
unsafe impl<T: Send + Sync> Sync for Shared<T> {}

impl<T> Shared<T> {
    /// Retain one immutable allocation while selecting a descendant of it.
    #[must_use]
    pub fn into_view<U>(self, select: impl for<'a> FnOnce(&'a T) -> &'a U) -> SharedView<T, U> {
        let value = NonNull::from(select(&self));
        SharedView { owner: self, value }
    }

    /// Observe the current strong-owner count without allocating or cloning.
    /// A live reference keeps the header valid; concurrent owners may change
    /// the returned snapshot immediately after the atomic load.
    #[must_use]
    pub fn strong_count(&self) -> usize {
        // SAFETY: this reference owns one strong count in the live header.
        unsafe { self.pointer.as_ref() }
            .strong
            .load(Ordering::Acquire)
    }

    /// Borrow the payload mutably only while this is the sole strong owner.
    /// There are no weak owners or raw exported pointers in this protocol.
    pub fn get_mut(&mut self) -> Option<&mut T> {
        // SAFETY: this live reference owns the header. Acquire synchronizes
        // with prior owner releases; exclusivity plus count one excludes any
        // simultaneous payload borrow or reference-count increment.
        let unique = unsafe { self.pointer.as_ref() }
            .strong
            .load(Ordering::Acquire)
            == 1;
        if unique {
            // SAFETY: uniqueness was established before creating any mutable
            // header reference; no weak owner can acquire a new strong owner.
            Some(unsafe { &mut self.pointer.as_mut().value })
        } else {
            None
        }
    }

    /// The complete, actual single allocation, including alignment and count.
    #[must_use]
    pub const fn allocation_layout() -> Layout {
        Layout::new::<Header<T>>()
    }

    /// Allocate one owner after its caller has admitted this concrete layout.
    ///
    /// # Errors
    /// Returns allocation refusal without constructing diagnostic text.
    pub fn try_new(value: T) -> Result<Self, SharedAllocError> {
        Self::try_new_with(|| value)
    }

    /// Allocate the actual control storage before invoking a payload factory.
    /// The factory is never called on allocation refusal. Its final ownership
    /// transfer must be infallible; a panic releases uninitialized storage.
    ///
    /// # Errors
    /// Returns allocator refusal while captured original payloads remain owned
    /// by the factory. No payload is removed before its destination exists.
    pub fn try_new_with(factory: impl FnOnce() -> T) -> Result<Self, SharedAllocError> {
        // SAFETY: alloc supplies null or global-allocator storage for this layout.
        unsafe { Self::try_new_with_allocator(factory, |layout| std::alloc::alloc(layout)) }
    }

    /// Internal allocation boundary, also exercised by null-allocator tests.
    ///
    /// # Safety
    /// The allocator returns null, or fresh global-allocator storage for exactly
    /// the supplied nonzero layout. It must not return an alias or other storage.
    unsafe fn try_new_with_allocator(
        factory: impl FnOnce() -> T,
        allocate: impl FnOnce(Layout) -> *mut u8,
    ) -> Result<Self, SharedAllocError> {
        let layout = Self::allocation_layout();
        let pointer = NonNull::new(allocate(layout).cast::<Header<T>>()).ok_or(SharedAllocError)?;
        // Keep the still-uninitialized destination under a deallocation guard
        // while arbitrary caller code runs. No T drop can run on this storage.
        let pending = UninitializedAllocation {
            pointer: pointer.cast(),
            layout,
        };
        let value = factory();
        // SAFETY: the allocation contract provides aligned, fresh storage;
        // no reference is exposed before this initialization.
        unsafe {
            pointer.as_ptr().write(Header {
                strong: AtomicUsize::new(1),
                value,
            });
        };
        core::mem::forget(pending);
        Ok(Self {
            pointer,
            _owner: PhantomData,
        })
    }

    /// Retain the existing allocation without allocating or copying its value.
    ///
    /// # Errors
    /// Refuses a count above isize::MAX without changing the existing count.
    pub fn try_clone(&self) -> Result<Self, SharedCloneError> {
        // SAFETY: self holds a strong reference, so the header is live.
        let header = unsafe { self.pointer.as_ref() };
        clone_count(&header.strong)?;
        Ok(Self {
            pointer: self.pointer,
            _owner: PhantomData,
        })
    }
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        self.try_clone()
            .expect("shared owner reference count overflow")
    }
}

impl<T> Deref for Shared<T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: this strong owner keeps the initialized allocation alive;
        // mutation and raw pointer extraction are not exposed.
        unsafe { &self.pointer.as_ref().value }
    }
}

impl<T> AsRef<T> for Shared<T> {
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T: fmt::Debug> fmt::Debug for Shared<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, formatter)
    }
}

impl<T> Drop for Shared<T> {
    fn drop(&mut self) {
        // SAFETY: this owner has not yet released its strong reference.
        let header = unsafe { self.pointer.as_ref() };
        if header.strong.fetch_sub(1, Ordering::Release) != 1 {
            return;
        }
        // Synchronize with earlier Release decrements before destroying T.
        fence(Ordering::Acquire);
        // SAFETY: this was the last owner. Move its unpinned T onto the stack
        // before freeing the concrete control block. A payload-held workspace
        // grant therefore remains live through the original header deallocation.
        let value = unsafe { core::ptr::addr_of!((*self.pointer.as_ptr()).value).read() };
        // SAFETY: this header came from the global allocator for this layout,
        // and no live owner or payload remains in it. Deallocation precedes any
        // T destructor, including one that releases admission or unwinds.
        unsafe { std::alloc::dealloc(self.pointer.as_ptr().cast(), Self::allocation_layout()) };
        drop(value);
    }
}

struct OwnedPayload<V, O> {
    value: V,
    // The actual payload dies before its original admission owner.
    _owner: O,
}

struct OwnedVtable<V> {
    borrow: unsafe fn(NonNull<()>) -> *const V,
    clone: unsafe fn(NonNull<()>) -> Result<(), SharedCloneError>,
    drop: unsafe fn(NonNull<()>),
}

/// An immutable value and its original owner in one fallible shared allocation.
/// The private vtable erases only the owner type. Borrowing cannot detach the
/// payload, and shallow cloning follows the single Shared lifetime protocol.
pub struct SharedOwned<V> {
    pointer: NonNull<()>,
    vtable: OwnedVtable<V>,
    _value: PhantomData<V>,
}

// SAFETY: factories require both payload and original owner to be Send + Sync.
unsafe impl<V: Send + Sync> Send for SharedOwned<V> {}
// SAFETY: only immutable borrowed payloads are exposed; Shared is atomic.
unsafe impl<V: Send + Sync> Sync for SharedOwned<V> {}

impl<V: Send + Sync + 'static> SharedOwned<V> {
    /// Exact single allocation, including payload storage and owner storage.
    #[must_use]
    pub const fn allocation_layout<O>() -> Layout {
        Shared::<OwnedPayload<V, O>>::allocation_layout()
    }

    /// Publish a value whose original owner already covers its storage and
    /// this concrete control layout. Failure destroys value before its owner.
    ///
    /// # Errors
    /// Returns actual allocator refusal before constructing the shared payload.
    pub fn try_from_admitted<O: Send + Sync + 'static>(
        value: V,
        owner: O,
    ) -> Result<Self, SharedAllocError> {
        Self::try_from_admitted_with(|| (value, owner))
    }

    /// Allocate shared control before moving an admitted value and original owner.
    /// The infallible factory is not invoked on refusal, leaving the caller's
    /// payload and grant available for its original failure cleanup.
    ///
    /// # Errors
    /// Returns allocator refusal before ownership transfer. The returned payload
    /// must already have admission for this concrete control layout.
    pub fn try_from_admitted_with<O: Send + Sync + 'static>(
        factory: impl FnOnce() -> (V, O),
    ) -> Result<Self, SharedAllocError> {
        let shared = Shared::try_new_with(|| {
            let (value, owner) = factory();
            OwnedPayload {
                value,
                _owner: owner,
            }
        })?;
        let pointer = shared.pointer.cast();
        core::mem::forget(shared);
        Ok(Self {
            pointer,
            vtable: OwnedVtable {
                borrow: owned_borrow::<V, O>,
                clone: owned_clone::<V, O>,
                drop: owned_drop::<V, O>,
            },
            _value: PhantomData,
        })
    }
}

impl SharedOwned<()> {
    /// Retain a producer's existing account without copying its live payloads.
    /// Both shared controls are admitted before allocating this owner or moving
    /// the original account. `replacement` starts a separate empty account;
    /// its construction must not detach any of the existing payload's grants.
    ///
    /// # Errors
    /// Preserves checked layout, original admission and physical refusal. On
    /// failure the original account remains in `storage` for payload cleanup.
    pub fn retain_current<S: super::Admission + Send + Sync + 'static>(
        storage: &mut S,
        replacement: S,
        admitted: usize,
        control: Layout,
    ) -> Result<(Self, usize), super::StorageError> {
        let total = admitted
            .checked_add(control.size())
            .and_then(|bytes| bytes.checked_add(Self::allocation_layout::<S>().size()))
            .ok_or(super::StorageError::SizeOverflow)?;
        storage.resize(total)?;
        Self::try_from_admitted_with(|| ((), core::mem::replace(storage, replacement)))
            .map(|owner| (owner, total))
            .map_err(|_| super::StorageError::AllocationFailed)
    }
}

impl<V> SharedOwned<V> {
    /// Share the original allocation without copying either payload or owner.
    ///
    /// # Errors
    /// Returns checked strong-reference overflow.
    pub fn try_clone(&self) -> Result<Self, SharedCloneError> {
        // SAFETY: self owns a live reference installed by the matching factory.
        unsafe {
            (self.vtable.clone)(self.pointer)?;
        }
        Ok(Self {
            pointer: self.pointer,
            vtable: OwnedVtable {
                borrow: self.vtable.borrow,
                clone: self.vtable.clone,
                drop: self.vtable.drop,
            },
            _value: PhantomData,
        })
    }
}

unsafe fn owned_borrow<V, O>(pointer: NonNull<()>) -> *const V {
    // SAFETY: only the corresponding OwnedPayload<V,O> factory installs this.
    let header = unsafe { pointer.cast::<Header<OwnedPayload<V, O>>>().as_ref() };
    &raw const header.value.value
}
unsafe fn owned_clone<V, O>(pointer: NonNull<()>) -> Result<(), SharedCloneError> {
    let shared = core::mem::ManuallyDrop::new(Shared::<OwnedPayload<V, O>> {
        pointer: pointer.cast(),
        _owner: PhantomData,
    });
    let clone = shared.try_clone()?;
    core::mem::forget(clone);
    Ok(())
}
unsafe fn owned_drop<V, O>(pointer: NonNull<()>) {
    // Shared frees the concrete header before payload and original owner.
    drop(Shared::<OwnedPayload<V, O>> {
        pointer: pointer.cast(),
        _owner: PhantomData,
    });
}
impl<V> Clone for SharedOwned<V> {
    fn clone(&self) -> Self {
        self.try_clone()
            .expect("shared owner reference count overflow")
    }
}
impl<V> Drop for SharedOwned<V> {
    fn drop(&mut self) {
        // SAFETY: each carrier owns one reference of its factory's type.
        unsafe {
            (self.vtable.drop)(self.pointer);
        }
    }
}
impl<V> Deref for SharedOwned<V> {
    type Target = V;
    fn deref(&self) -> &V {
        // SAFETY: matching factory and a live reference; borrow tied to self.
        unsafe { &*(self.vtable.borrow)(self.pointer) }
    }
}
impl<V> AsRef<V> for SharedOwned<V> {
    fn as_ref(&self) -> &V {
        self
    }
}
impl<V: fmt::Debug> fmt::Debug for SharedOwned<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

/// Immutable shared text retaining its original allocation owner.
#[derive(Clone)]
pub struct SharedText(TextStorage);
#[derive(Clone)]
enum TextStorage {
    Owned(SharedOwned<String>),
    Static(&'static str),
}
impl SharedText {
    /// Borrow immutable metadata known before an invocation, without allocating.
    #[must_use]
    pub const fn from_static(text: &'static str) -> Self {
        Self(TextStorage::Static(text))
    }

    /// Exact single allocation in addition to the String capacity.
    #[must_use]
    pub const fn allocation_layout<O>() -> Layout {
        SharedOwned::<String>::allocation_layout::<O>()
    }
    /// Publish admitted text and its original owner without another allocation.
    ///
    /// # Errors
    /// Returns actual allocator refusal, destroying text before its owner.
    pub fn try_from_admitted<O: Send + Sync + 'static>(
        text: String,
        owner: O,
    ) -> Result<Self, SharedAllocError> {
        SharedOwned::try_from_admitted(text, owner).map(|value| Self(TextStorage::Owned(value)))
    }
    /// Borrow the spelling under this owner's lifetime.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match &self.0 {
            TextStorage::Owned(text) => text.as_str(),
            TextStorage::Static(text) => text,
        }
    }
    /// Original owned UTF-8 capacity; static text owns no allocation.
    /// This describes surviving storage, rather than proving a future allocation.
    #[must_use]
    pub fn owned_capacity(&self) -> usize {
        match &self.0 {
            TextStorage::Owned(text) => text.capacity(),
            TextStorage::Static(_) => 0,
        }
    }

    /// Clone the original shared owner without allocating.
    ///
    /// # Errors
    /// Returns checked strong-reference overflow.
    pub fn try_clone(&self) -> Result<Self, SharedCloneError> {
        match &self.0 {
            TextStorage::Owned(text) => text.try_clone().map(|text| Self(TextStorage::Owned(text))),
            TextStorage::Static(text) => Ok(Self::from_static(text)),
        }
    }
}
impl Deref for SharedText {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}
impl AsRef<str> for SharedText {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl fmt::Debug for SharedText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl core::borrow::Borrow<str> for SharedText {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}
impl PartialEq for SharedText {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}
impl Eq for SharedText {}
impl PartialOrd for SharedText {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for SharedText {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl core::hash::Hash for SharedText {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::hash::Hash::hash(self.as_str(), state);
    }
}

impl fmt::Display for SharedText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn moving_an_unpin_owner_preserves_a_non_unpin_payload_address() {
        let mut owner = Shared::try_new(core::marker::PhantomPinned).unwrap();
        let original = core::ptr::from_ref(&*owner);
        let pinned_owner = core::pin::Pin::new(&mut owner);
        assert_eq!(core::ptr::from_ref(&**pinned_owner), original);
        let moved = [owner];
        assert_eq!(core::ptr::from_ref(&*moved[0]), original);
    }

    use super::*;
    use std::sync::Arc;

    struct DropProbe(Arc<AtomicUsize>);
    impl Drop for DropProbe {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn null_allocator_refusal_drops_input_once() {
        let drops = Arc::new(AtomicUsize::new(0));
        let original = DropProbe(drops.clone());
        // SAFETY: null satisfies the allocation boundary's refusal contract.
        let result =
            unsafe { Shared::try_new_with_allocator(|| original, |_| core::ptr::null_mut()) };
        assert!(matches!(result, Err(SharedAllocError)));
        assert_eq!(drops.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn allocation_refusal_never_invokes_payload_factory() {
        let invoked = core::cell::Cell::new(false);
        // SAFETY: null satisfies the exact allocation-refusal contract.
        let result = unsafe {
            Shared::try_new_with_allocator(
                || {
                    invoked.set(true);
                    7u8
                },
                |_| core::ptr::null_mut(),
            )
        };
        assert!(matches!(result, Err(SharedAllocError)));
        assert!(!invoked.get());
    }

    #[test]
    fn factory_constructs_once_and_shared_clones_do_not_repeat_it() {
        let invoked = core::cell::Cell::new(0usize);
        let value = Shared::try_new_with(|| {
            invoked.set(invoked.get() + 1);
            7u8
        })
        .unwrap();
        let clone = value.clone();
        assert_eq!(invoked.get(), 1);
        assert!(core::ptr::eq(&raw const *value, &raw const *clone));
    }

    #[test]
    fn shallow_clone_keeps_original_payload_live() {
        let drops = Arc::new(AtomicUsize::new(0));
        let original = Shared::try_new(DropProbe(drops.clone())).expect("test allocation");
        let survivor = original.clone();
        assert!(core::ptr::eq(&raw const *original, &raw const *survivor));
        drop(original);
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        drop(survivor);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn projected_view_retains_the_original_owner_through_move_and_last_drop() {
        struct Payload {
            values: [u32; 2],
            _drops: DropProbe,
        }
        let drops = Arc::new(AtomicUsize::new(0));
        let mut owner = Shared::try_new(Payload {
            values: [7, 11],
            _drops: DropProbe(drops.clone()),
        })
        .expect("test allocation");
        let projected = owner.clone().into_view(|payload| &payload.values);
        assert!(owner.get_mut().is_none());
        let element = projected
            .try_project(|values| values.get(1))
            .expect("second element");
        let moved = [element.clone()];
        drop(projected);
        drop(owner);
        drop(element);
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        assert_eq!(**moved.first().expect("retained projected element"), 11);
        drop(moved);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn last_owner_drops_payload_once_across_threads() {
        let drops = Arc::new(AtomicUsize::new(0));
        let owner = Shared::try_new(DropProbe(drops.clone())).expect("test allocation");
        std::thread::scope(|scope| {
            for _ in 0..8 {
                let worker = owner.clone();
                scope.spawn(move || {
                    for _ in 0..1024 {
                        drop(worker.clone());
                    }
                });
            }
        });
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        drop(owner);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn zero_sized_and_overaligned_payloads_keep_valid_layouts() {
        #[repr(align(256))]
        struct Aligned(u8);
        let empty = Shared::try_new(()).expect("zero-sized payload");
        assert_eq!(*empty, ());
        assert!(Shared::<()>::allocation_layout().size() >= size_of::<AtomicUsize>());
        let aligned = Shared::try_new(Aligned(7)).expect("aligned payload");
        assert_eq!(aligned.0, 7);
        assert_eq!((&raw const *aligned).addr() % 256, 0);
        assert_eq!(Shared::<Aligned>::allocation_layout().align(), 256);
    }

    #[test]
    fn checked_clone_refuses_without_wrapping_count() {
        let owner = Shared::try_new(7u8).expect("test allocation");
        // SAFETY: test owns the only strong reference; no other thread can use
        // it. Restore the real count before its destructor executes.
        let count = unsafe { &owner.pointer.as_ref().strong };
        count.store(usize::MAX / 2, Ordering::Relaxed);
        let refused = owner.try_clone();
        assert!(matches!(refused, Err(SharedCloneError)));
        assert_eq!(count.load(Ordering::Relaxed), usize::MAX / 2);
        count.store(1, Ordering::Relaxed);
    }
}
