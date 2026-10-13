// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Allocation-free cleanup of directly owned secret storage.
//!
//! Observed default-value stores and a compiler fence protect the explicit
//! storage supplied here. Historical compiler copies, registers and spills,
//! external hash implementations and caller-owned copies remain separate.

use core::hint::black_box;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{Ordering, compiler_fence};

/// Overwrite every slot, observe the stores and fence before deallocation.
///
/// This safe-Rust operation does not promise elimination of historical copies
/// or physical memory erasure. Primitive integer defaults are zero.
pub fn wipe_secret<T: Copy + Default>(slots: &mut [T]) {
    slots.fill(T::default());
    black_box(&mut *slots);
    compiler_fence(Ordering::SeqCst);
}

/// An allocation-free owner whose complete array is overwritten on drop.
///
/// Cloning creates another guarded owner. Dereferencing exposes only borrowed
/// storage; any copies a caller makes remain that caller's responsibility.
pub struct SecretArray<T: Copy + Default, const N: usize> {
    values: [T; N],
    #[cfg(test)]
    observer: Option<std::sync::Arc<Observer<T, N>>>,
}

#[cfg(test)]
pub(crate) type Observer<T, const N: usize> = dyn Fn(&[T; N]) + Send + Sync;

impl<T: Copy + Default, const N: usize> SecretArray<T, N> {
    /// Take ownership of an array without allocating.
    pub const fn new(values: [T; N]) -> Self {
        Self {
            values,
            #[cfg(test)]
            observer: None,
        }
    }

    /// Overwrite the complete live array at its last use.
    pub fn clear(&mut self) {
        wipe_secret(&mut self.values);
    }

    #[cfg(test)]
    pub(crate) fn observe_cleared_drop(&mut self)
    where
        T: PartialEq + core::fmt::Debug + 'static,
    {
        self.observe_drop(std::sync::Arc::new(|live| {
            assert_eq!(live, &[T::default(); N]);
        }));
    }

    #[cfg(test)]
    pub(crate) fn observe_drop(&mut self, observer: std::sync::Arc<Observer<T, N>>) {
        self.observer = Some(observer);
    }
}

impl<T: Copy + Default, const N: usize> Clone for SecretArray<T, N> {
    fn clone(&self) -> Self {
        Self {
            values: self.values,
            #[cfg(test)]
            observer: self.observer.clone(),
        }
    }
}

impl<T: Copy + Default, const N: usize> Deref for SecretArray<T, N> {
    type Target = [T; N];

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl<T: Copy + Default, const N: usize> DerefMut for SecretArray<T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
    }
}

impl<T: Copy + Default, const N: usize> Drop for SecretArray<T, N> {
    fn drop(&mut self) {
        self.clear();
        // Observe the actual array while its owner and storage are still live.
        #[cfg(test)]
        if let Some(observer) = &self.observer {
            observer(&self.values);
        }
    }
}

impl<T: Copy + Default, const N: usize> core::fmt::Debug for SecretArray<T, N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SecretArray")
            .field("length", &N)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;

    pub(crate) fn observe<T: Copy + Default + PartialEq + core::fmt::Debug, const N: usize>(
        storage: &mut SecretArray<T, N>,
        seen: &Arc<AtomicUsize>,
    ) {
        let seen = seen.clone();
        storage.observe_drop(Arc::new(move |live| {
            assert_eq!(live, &[T::default(); N]);
            seen.fetch_add(1, Ordering::SeqCst);
        }));
    }

    #[test]
    fn complete_live_storage_clears_on_last_use_clone_return_error_and_unwind() {
        let seen = Arc::new(AtomicUsize::new(0));
        let mut original = SecretArray::new([0xa5u8; 168]);
        observe(&mut original, &seen);
        let mut cloned = original.clone();
        original.clear();
        assert_eq!(*original, [0; 168]);
        assert_eq!(*cloned, [0xa5; 168]);
        cloned.clear();
        assert_eq!(*cloned, [0; 168]);
        drop(original);
        drop(cloned);
        assert_eq!(seen.load(Ordering::SeqCst), 2);
        let error = || -> Result<(), ()> {
            let mut scratch = SecretArray::new([u64::MAX; 25]);
            observe(&mut scratch, &seen);
            Err(())
        };
        assert_eq!(error(), Err(()));
        assert_eq!(seen.load(Ordering::SeqCst), 3);
        let result = std::panic::catch_unwind(|| {
            let mut scratch = SecretArray::new([-3i32; 256]);
            observe(&mut scratch, &seen);
            panic!("exercise guarded unwind");
        });
        assert!(result.is_err());
        assert_eq!(seen.load(Ordering::SeqCst), 4);
    }
}
