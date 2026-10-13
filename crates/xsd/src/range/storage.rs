// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Before-birth ownership of the range algebra's actual native products.

use purrdf_lex::walk::VecReserve;
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;

use crate::bigint::{LimbScratchError, Mag, scratch::Allocate};
use crate::exact::{self, Cost};
use crate::value::XsdValue;

/// A native range product could not be admitted or physically allocated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageError {
    /// The caller refused the next work or storage operation.
    Refused,
    /// A checked layout or the physical allocator refused a destination.
    Allocation,
}

impl core::fmt::Display for StorageError {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        out.write_str(match self {
            Self::Refused => "range storage admission refused",
            Self::Allocation => "range allocation failed",
        })
    }
}
impl std::error::Error for StorageError {}

/// Original range construction admission. `resize` is the live total of this
/// range operation, excluding the caller's already-owned inputs. No default cap.
pub trait Storage {
    /// Admit work before the next operation.
    fn work(&mut self, operations: u64) -> Result<(), StorageError>;
    /// Admit the actual checked live allocation total before growth.
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError>;
    /// Record a successful native allocation, when observations are requested.
    fn allocated(&mut self) {}
}

pub(super) struct Resident;
impl Storage for Resident {
    fn work(&mut self, _: u64) -> Result<(), StorageError> {
        Ok(())
    }
    fn resize(&mut self, _: usize) -> Result<(), StorageError> {
        Ok(())
    }
}

/// This census releases original before-allocation grants after native scratch
/// dies. It never acquires admission for a resident result.
pub(super) trait OwnedBytes {
    fn owned_bytes(&self) -> usize;
}

impl<T: OwnedBytes> OwnedBytes for Vec<T> {
    fn owned_bytes(&self) -> usize {
        self.capacity() * size_of::<T>() + self.iter().map(OwnedBytes::owned_bytes).sum::<usize>()
    }
}
impl OwnedBytes for String {
    fn owned_bytes(&self) -> usize {
        self.capacity()
    }
}
impl OwnedBytes for exact::Integer {
    fn owned_bytes(&self) -> usize {
        usize::try_from(self.heap_bytes()).expect("native limb capacity is usize")
    }
}
impl OwnedBytes for exact::Decimal {
    fn owned_bytes(&self) -> usize {
        usize::try_from(self.heap_bytes()).expect("native limb capacity is usize")
    }
}
impl OwnedBytes for XsdValue {
    fn owned_bytes(&self) -> usize {
        match self {
            Self::String(text) => text.capacity(),
            Self::Binary { bytes, .. } => bytes.capacity(),
            Self::BigInteger { value, .. } => value.owned_bytes(),
            Self::BigDecimal(value) => value.owned_bytes(),
            _ => 0,
        }
    }
}
macro_rules! inline_bytes { ($($ty:ty),+ $(,)?) => {$(impl OwnedBytes for $ty { fn owned_bytes(&self) -> usize { 0 } })+}; }
inline_bytes!(
    u8,
    u32,
    u64,
    bool,
    Ordering,
    super::Cardinality,
    super::Satisfiability,
    super::Known
);
impl<T: OwnedBytes> OwnedBytes for Option<T> {
    fn owned_bytes(&self) -> usize {
        self.as_ref().map_or(0, OwnedBytes::owned_bytes)
    }
}
impl<A: OwnedBytes, B: OwnedBytes> OwnedBytes for (A, B) {
    fn owned_bytes(&self) -> usize {
        self.0.owned_bytes() + self.1.owned_bytes()
    }
}
impl<T: OwnedBytes, const N: usize> OwnedBytes for [T; N] {
    fn owned_bytes(&self) -> usize {
        self.iter().map(OwnedBytes::owned_bytes).sum()
    }
}

pub(super) struct Memory<'a> {
    storage: &'a mut dyn Storage,
    live: usize,
}
impl<'a> Memory<'a> {
    pub(super) fn new(storage: &'a mut dyn Storage) -> Self {
        Self { storage, live: 0 }
    }
    pub(super) fn with_resident_input(storage: &'a mut Resident, live: usize) -> Self {
        Self { storage, live }
    }
    pub(super) fn step(&mut self) -> Result<(), StorageError> {
        self.storage.work(1)
    }
    pub(super) fn drop_value<T: OwnedBytes>(&mut self, value: T) -> Result<(), StorageError> {
        let bytes = value.owned_bytes();
        drop(value);
        self.resize(
            self.live
                .checked_sub(bytes)
                .ok_or(StorageError::Allocation)?,
        )
    }
    pub(super) fn array<T, const N: usize>(
        &mut self,
        mut item: impl FnMut(usize, &mut Self) -> Result<T, StorageError>,
    ) -> Result<[T; N], StorageError> {
        let mut values: [Option<T>; N] = std::array::from_fn(|_| None);
        for (index, slot) in values.iter_mut().enumerate() {
            *slot = Some(item(index, self)?);
        }
        Ok(values.map(|value| value.expect("every fixed array slot initialized")))
    }
    /// In-place unstable heapsort permits a typed comparator refusal without a
    /// partially ordered comparator, another buffer, or a recursive stack.
    pub(super) fn sort<T>(
        &mut self,
        values: &mut [T],
        mut compare: impl FnMut(&T, &T, &mut Self) -> Result<Ordering, StorageError>,
    ) -> Result<(), StorageError> {
        fn sift<'storage, T>(
            values: &mut [T],
            mut root: usize,
            end: usize,
            memory: &mut Memory<'storage>,
            compare: &mut impl FnMut(&T, &T, &mut Memory<'storage>) -> Result<Ordering, StorageError>,
        ) -> Result<(), StorageError> {
            loop {
                let Some(mut child) = root
                    .checked_mul(2)
                    .and_then(|n| n.checked_add(1))
                    .filter(|&n| n < end)
                else {
                    return Ok(());
                };
                if child + 1 < end
                    && compare(&values[child], &values[child + 1], memory)? == Ordering::Less
                {
                    child += 1;
                }
                if compare(&values[root], &values[child], memory)? != Ordering::Less {
                    return Ok(());
                }
                values.swap(root, child);
                root = child;
            }
        }
        for root in (0..values.len() / 2).rev() {
            sift(values, root, values.len(), self, &mut compare)?;
        }
        for end in (1..values.len()).rev() {
            values.swap(0, end);
            sift(values, 0, end, self, &mut compare)?;
        }
        Ok(())
    }
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        self.storage.resize(bytes)?;
        self.live = bytes;
        Ok(())
    }
    pub(super) fn scope<T: OwnedBytes>(
        &mut self,
        consumed: usize,
        body: impl FnOnce(&mut Self) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        let baseline = self
            .live
            .checked_sub(consumed)
            .ok_or(StorageError::Allocation)?;
        let result = body(self);
        match result {
            Ok(value) => {
                let target = baseline
                    .checked_add(value.owned_bytes())
                    .ok_or(StorageError::Allocation)?;
                if let Err(error) = self.resize(target) {
                    drop(value);
                    let _ = self.resize(baseline);
                    return Err(error);
                }
                Ok(value)
            }
            Err(error) => {
                let _ = self.resize(baseline);
                Err(error)
            }
        }
    }
    pub(super) fn one<T>(&mut self, value: T) -> Result<Vec<T>, StorageError> {
        let mut values = Vec::new();
        self.push(&mut values, value)?;
        Ok(values)
    }
    pub(super) fn clone_values<T: CloneOwned>(
        &mut self,
        input: &[T],
    ) -> Result<Vec<T>, StorageError> {
        let mut values = Vec::new();
        self.reserve(&mut values, input.len())?;
        for value in input {
            let value = value.clone_owned(self)?;
            values.push(value);
        }
        Ok(values)
    }
    fn numeric<T: OwnedBytes>(
        &mut self,
        cost: Cost,
        body: impl FnOnce(&Native<'_, '_>) -> Result<T, LimbScratchError>,
    ) -> Result<T, StorageError> {
        self.storage.work(cost.work())?;
        self.scope(0, |memory| {
            let native = Native {
                memory: RefCell::new(memory),
                failure: Cell::new(None),
            };
            let value = body(&native);
            value.map_err(|_| native.failure.get().unwrap_or(StorageError::Allocation))
        })
    }
    pub(super) fn integer_add(
        &mut self,
        left: &exact::Integer,
        right: &exact::Integer,
    ) -> Result<exact::Integer, StorageError> {
        self.numeric(left.add_cost(right), |native| left.add_using(right, native))
    }
    pub(super) fn integer_sub(
        &mut self,
        left: &exact::Integer,
        right: &exact::Integer,
    ) -> Result<exact::Integer, StorageError> {
        self.numeric(left.add_cost(right), |native| left.sub_using(right, native))
    }
    pub(super) fn decimal_cmp(
        &mut self,
        left: &exact::Decimal,
        right: &exact::Decimal,
    ) -> Result<Ordering, StorageError> {
        self.numeric(left.cmp_cost(right), |native| left.cmp_using(right, native))
    }
    pub(super) fn round(
        &mut self,
        value: &exact::Decimal,
        rounding: exact::Rounding,
    ) -> Result<exact::Integer, StorageError> {
        self.numeric(value.round_cost(0), |native| {
            value.round_to_integer_using(rounding, native)
        })
    }
    pub(super) fn decimal_of(
        &mut self,
        value: &XsdValue,
    ) -> Result<Option<exact::Decimal>, StorageError> {
        match value {
            XsdValue::Integer { value, .. } => Ok(Some(exact::Decimal::from(*value))),
            XsdValue::Decimal(value) => Ok(Some(exact::Decimal::from_bounded(value))),
            XsdValue::BigInteger { value, .. } => {
                Ok(Some(exact::Decimal::from_integer(value.clone_owned(self)?)))
            }
            XsdValue::BigDecimal(value) => Ok(Some(value.clone_owned(self)?)),
            _ => Ok(None),
        }
    }
}

impl VecReserve for Memory<'_> {
    type Error = StorageError;

    fn reserve<T>(&mut self, values: &mut Vec<T>, additional: usize) -> Result<(), StorageError> {
        let needed = values
            .len()
            .checked_add(additional)
            .ok_or(StorageError::Allocation)?;
        if needed <= values.capacity() {
            return Ok(());
        }
        let capacity = needed.max(
            values
                .capacity()
                .checked_mul(2)
                .ok_or(StorageError::Allocation)?,
        );
        let bytes = capacity
            .checked_mul(size_of::<T>())
            .filter(|&n| isize::try_from(n).is_ok())
            .ok_or(StorageError::Allocation)?;
        let old_live = self.live;
        self.resize(
            old_live
                .checked_add(bytes)
                .ok_or(StorageError::Allocation)?,
        )?;
        let old = values.capacity() * size_of::<T>();
        if values.try_reserve_exact(capacity - values.len()).is_err() {
            let _ = self.resize(old_live);
            return Err(StorageError::Allocation);
        }
        self.storage.allocated();
        self.resize(old_live - old + values.capacity() * size_of::<T>())
    }
}

struct Native<'a, 's> {
    memory: RefCell<&'a mut Memory<'s>>,
    failure: Cell<Option<StorageError>>,
}
impl Allocate for Native<'_, '_> {
    fn destination(&self, capacity: usize, length: usize) -> Result<Mag, LimbScratchError> {
        if capacity <= 3 {
            return Mag::try_destination(capacity, length);
        }
        let mut memory = self.memory.borrow_mut();
        let bytes = capacity
            .checked_mul(size_of::<u64>())
            .filter(|&n| isize::try_from(n).is_ok());
        let result = (|| {
            let bytes = bytes.ok_or(StorageError::Allocation)?;
            let target = memory
                .live
                .checked_add(bytes)
                .ok_or(StorageError::Allocation)?;
            memory.resize(target)?;
            let value =
                Mag::try_destination(capacity, length).map_err(|_| StorageError::Allocation)?;
            memory.storage.allocated();
            Ok(value)
        })();
        result.map_err(|error| {
            self.failure.set(self.failure.get().or(Some(error)));
            LimbScratchError::SizeOverflow
        })
    }
}

pub(super) trait CloneOwned: OwnedBytes + Sized {
    fn clone_owned(&self, memory: &mut Memory<'_>) -> Result<Self, StorageError>;
}
macro_rules! numeric_clone_owned {
    ($($ty:ty),+ $(,)?) => {$(
        impl CloneOwned for $ty {
            fn clone_owned(&self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
                memory.numeric(self.unary_cost(), |native| self.copy_using(native))
            }
        }
    )+};
}
numeric_clone_owned!(exact::Integer, exact::Decimal);
impl CloneOwned for u32 {
    fn clone_owned(&self, _: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(*self)
    }
}
impl<T: CloneOwned> CloneOwned for Vec<T> {
    fn clone_owned(&self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        memory.clone_values(self)
    }
}
impl CloneOwned for XsdValue {
    fn clone_owned(&self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(match self {
            Self::String(text) => {
                let bytes = memory.copy(text.as_bytes())?;
                Self::String(String::from_utf8(bytes).expect("copied valid UTF-8"))
            }
            Self::Binary { bytes, datatype } => Self::Binary {
                bytes: memory.copy(bytes)?,
                datatype: *datatype,
            },
            Self::BigInteger { value, datatype } => Self::BigInteger {
                value: value.clone_owned(memory)?,
                datatype: *datatype,
            },
            Self::BigDecimal(value) => Self::BigDecimal(value.clone_owned(memory)?),
            value => value.clone(),
        })
    }
}
