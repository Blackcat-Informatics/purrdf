// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! C-ABI carriers for SPARQL execution governors, receipts, and cancellation.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use purrdf_sparql_eval::{
    CancellationFlag, GovernorEvidence, QueryGovernors, ResourceDimension, StopCause, StopSignal,
    TrippedGovernor, WallDeadline,
};

use crate::error::PurrdfError;
use crate::status::PurrdfStatus;

/// Bit values for [`PurrdfQueryGovernors::enabled`].
///
/// The struct stores a plain `uint32_t`, not this enum, so C may combine values safely.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PurrdfGovernorFlag {
    /// Enforce `fuel`.
    Fuel = 1 << 0,
    /// Enforce `max_answers` (query only; UPDATE rejects this flag).
    MaxAnswers = 1 << 1,
    /// Enforce `max_intermediate_cells`.
    MaxIntermediateCells = 1 << 2,
    /// Enforce `max_scratch_bytes`.
    MaxScratchBytes = 1 << 3,
    /// Enforce `max_remote_requests`.
    MaxRemoteRequests = 1 << 4,
    /// Enforce `deadline_millis` from the start of the call.
    DeadlineMillis = 1 << 5,
    /// Poll `cancellation`.
    Cancellation = 1 << 6,
}

const KNOWN_FLAGS: u32 = PurrdfGovernorFlag::Fuel as u32
    | PurrdfGovernorFlag::MaxAnswers as u32
    | PurrdfGovernorFlag::MaxIntermediateCells as u32
    | PurrdfGovernorFlag::MaxScratchBytes as u32
    | PurrdfGovernorFlag::MaxRemoteRequests as u32
    | PurrdfGovernorFlag::DeadlineMillis as u32
    | PurrdfGovernorFlag::Cancellation as u32;

/// Caller-supplied governors for one C-ABI query or UPDATE call.
///
/// Set the corresponding [`PurrdfGovernorFlag`] bit in `enabled` for every field that is
/// meaningful. Zero is a valid inclusive ceiling. Unset numeric fields are ignored, and
/// every governed call still meters them at the native `METERED` ceiling so its evidence
/// is useful. `reserved` must be zero. `cancellation`, when enabled, must remain a live
/// handle until the synchronous call returns.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PurrdfQueryGovernors {
    /// OR-ed [`PurrdfGovernorFlag`] values.
    pub enabled: u32,
    /// Reserved for ABI-compatible extension; must be zero.
    pub reserved: u32,
    /// Inclusive fuel ceiling.
    pub fuel: u64,
    /// Inclusive query-answer ceiling.
    pub max_answers: u64,
    /// Inclusive peak intermediate-cell ceiling.
    pub max_intermediate_cells: u64,
    /// Inclusive scratch-byte ceiling.
    pub max_scratch_bytes: u64,
    /// Inclusive remote-request ceiling.
    pub max_remote_requests: u64,
    /// Relative wall-clock budget in milliseconds.
    pub deadline_millis: u64,
    /// Optional shareable cancellation handle.
    pub cancellation: *const PurrdfCancellation,
}

impl PurrdfQueryGovernors {
    const METERED: Self = Self {
        enabled: 0,
        reserved: 0,
        fuel: 0,
        max_answers: 0,
        max_intermediate_cells: 0,
        max_scratch_bytes: 0,
        max_remote_requests: 0,
        deadline_millis: 0,
        cancellation: std::ptr::null(),
    };

    pub(crate) const fn flag(&self, flag: PurrdfGovernorFlag) -> bool {
        self.enabled & flag as u32 != 0
    }
}

/// Initialize `*out` as a metered governed call with no finite caller ceiling.
///
/// # Safety
/// `out` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_query_governors_init(out: *mut PurrdfQueryGovernors) -> i32 {
    unsafe {
        ffi_guard!(PurrdfStatus::Panic as i32, {
            if out.is_null() {
                return PurrdfStatus::NullPointer as i32;
            }
            *out = PurrdfQueryGovernors::METERED;
            PurrdfStatus::Ok as i32
        })
    }
}

/// A shareable, monotone cancellation handle. Create one per cancellation lifetime.
#[derive(Debug)]
pub struct PurrdfCancellation(CancellationFlag);

/// Allocate a fresh, uncancelled handle in `*out`.
///
/// # Safety
/// `out` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_cancellation_new(out: *mut *mut PurrdfCancellation) -> i32 {
    unsafe {
        ffi_guard!(PurrdfStatus::Panic as i32, {
            if out.is_null() {
                return PurrdfStatus::NullPointer as i32;
            }
            *out = Box::into_raw(Box::new(PurrdfCancellation(CancellationFlag::new())));
            PurrdfStatus::Ok as i32
        })
    }
}

/// Latch `cancellation`. Idempotent and safe to call from another thread.
///
/// # Safety
/// `cancellation` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_cancellation_cancel(
    cancellation: *const PurrdfCancellation,
) -> i32 {
    unsafe {
        ffi_guard!(PurrdfStatus::Panic as i32, {
            if cancellation.is_null() {
                return PurrdfStatus::NullPointer as i32;
            }
            (*cancellation).0.cancel();
            PurrdfStatus::Ok as i32
        })
    }
}

/// Write `1` when `cancellation` has latched, otherwise `0`.
///
/// # Safety
/// `cancellation` must be live and `out` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_cancellation_is_cancelled(
    cancellation: *const PurrdfCancellation,
    out: *mut u8,
) -> i32 {
    unsafe {
        ffi_guard!(PurrdfStatus::Panic as i32, {
            if cancellation.is_null() || out.is_null() {
                return PurrdfStatus::NullPointer as i32;
            }
            *out = u8::from((*cancellation).0.is_cancelled());
            PurrdfStatus::Ok as i32
        })
    }
}

/// Release a cancellation handle. No-op on null.
///
/// # Safety
/// `cancellation` must be null or a live handle not already freed. A handle referenced by
/// an active governed call must remain live until that call returns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_cancellation_free(cancellation: *mut PurrdfCancellation) {
    unsafe {
        ffi_guard!((), {
            if !cancellation.is_null() {
                drop(Box::from_raw(cancellation));
            }
        });
    }
}

/// Stable C discriminants for governed resource dimensions.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PurrdfResourceDimension {
    /// Abstract execution fuel.
    Fuel = 0,
    /// Final answer sequence units.
    AnswerRows = 1,
    /// Peak intermediate cells.
    IntermediateCells = 2,
    /// Scratch-arena bytes.
    ScratchBytes = 3,
    /// Remote requests.
    RemoteRequests = 4,
    /// UDF recursion depth.
    UdfDepth = 5,
    /// Demand-paged pages.
    Pages = 6,
    /// Demand-paged bytes.
    Bytes = 7,
}

/// Stable C discriminants for the shape of a tripped governor.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PurrdfGovernorTripKind {
    /// No governor tripped.
    None = 0,
    /// An observed resource ceiling was exceeded.
    Budget = 1,
    /// A cancellation or deadline fired.
    Stopped = 2,
    /// Admission rejected a plan estimate before evaluation.
    Refused = 3,
    /// A future kernel trip this ABI version cannot decode.
    Unknown = 4,
}

/// Stable C discriminants for stop causes.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PurrdfStopCause {
    /// No stop signal fired.
    None = 0,
    /// Explicit cancellation.
    Cancelled = 1,
    /// Wall deadline expired.
    Deadline = 2,
}

/// A fixed, named C representation of the kernel's eight-dimensional resource vector.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PurrdfResourceVector {
    /// Fuel units.
    pub fuel: u64,
    /// Final answer units.
    pub answer_rows: u64,
    /// Peak intermediate cells.
    pub intermediate_cells: u64,
    /// Scratch bytes.
    pub scratch_bytes: u64,
    /// Remote requests.
    pub remote_requests: u64,
    /// UDF recursion depth.
    pub udf_depth: u64,
    /// Demand-paged pages.
    pub pages: u64,
    /// Demand-paged bytes.
    pub bytes: u64,
}

/// Typed details of the governor that stopped an execution.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PurrdfGovernorTrip {
    /// [`PurrdfGovernorTripKind`] discriminant.
    pub kind: i32,
    /// [`PurrdfResourceDimension`] discriminant, or `-1` for a stop/none/unknown.
    pub dimension: i32,
    /// [`PurrdfStopCause`] discriminant.
    pub stop_cause: i32,
    /// Inclusive ceiling for a budget/refusal; zero otherwise.
    pub limit: u64,
    /// Observed consumption for a budget trip; zero otherwise.
    pub consumed: u64,
    /// Planner estimate for an admission refusal; zero otherwise.
    pub estimate: u64,
}

/// One governed execution's deterministic accounting receipt.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PurrdfGovernorEvidence {
    /// Consumption charged per dimension.
    pub consumed: PurrdfResourceVector,
    /// Ceilings in force per dimension.
    pub limits: PurrdfResourceVector,
    /// The terminal trip, or `kind == NONE` on completion.
    pub trip: PurrdfGovernorTrip,
}

/// Build native governors from a validated C carrier.
///
/// # Safety
/// `config` and any enabled cancellation handle must remain live for this call.
pub(crate) unsafe fn decode_governors(
    config: *const PurrdfQueryGovernors,
) -> Result<QueryGovernors, PurrdfError> {
    unsafe {
        if config.is_null() {
            return Err(PurrdfError::new(
                PurrdfStatus::NullPointer,
                "governors is null",
            ));
        }
        let config = &*config;
        if config.reserved != 0 {
            return Err(PurrdfError::new(
                PurrdfStatus::InvalidArgument,
                "PurrdfQueryGovernors.reserved must be zero",
            ));
        }
        if config.enabled & !KNOWN_FLAGS != 0 {
            return Err(PurrdfError::new(
                PurrdfStatus::InvalidArgument,
                format!(
                    "unknown governor flag bits: 0x{:08x}",
                    config.enabled & !KNOWN_FLAGS
                ),
            ));
        }

        let mut governors = QueryGovernors::METERED;
        if config.flag(PurrdfGovernorFlag::Fuel) {
            governors = governors.with_fuel(config.fuel);
        }
        if config.flag(PurrdfGovernorFlag::MaxAnswers) {
            governors = governors.with_max_answers(config.max_answers);
        }
        if config.flag(PurrdfGovernorFlag::MaxIntermediateCells) {
            governors = governors.with_max_intermediate_cells(config.max_intermediate_cells);
        }
        if config.flag(PurrdfGovernorFlag::MaxScratchBytes) {
            governors = governors.with_max_scratch_bytes(config.max_scratch_bytes);
        }
        if config.flag(PurrdfGovernorFlag::MaxRemoteRequests) {
            governors = governors.with_max_remote_requests(config.max_remote_requests);
        }

        let cancellation = if config.flag(PurrdfGovernorFlag::Cancellation) {
            if config.cancellation.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "cancellation flag is enabled but its handle is null",
                ));
            }
            Some((*config.cancellation).0.clone())
        } else {
            None
        };
        let deadline = config
            .flag(PurrdfGovernorFlag::DeadlineMillis)
            .then(|| WallDeadline::after(Duration::from_millis(config.deadline_millis)));
        let watch = CStopWatch::new(cancellation, deadline);
        if watch.is_armed() {
            let signal: Arc<dyn StopSignal> = Arc::new(watch);
            governors = governors.with_stop_signal(signal);
        }
        Ok(governors)
    }
}

/// Reject an answer cap on UPDATE, whose answer sequence is empty by definition.
pub(crate) unsafe fn validate_update_governors(
    config: *const PurrdfQueryGovernors,
) -> Result<(), PurrdfError> {
    unsafe {
        if config.is_null() {
            return Err(PurrdfError::new(
                PurrdfStatus::NullPointer,
                "governors is null",
            ));
        }
        if (*config).flag(PurrdfGovernorFlag::MaxAnswers) {
            return Err(PurrdfError::new(
                PurrdfStatus::InvalidArgument,
                "max_answers is not accepted by governed UPDATE: UPDATE has no answer sequence",
            ));
        }
        Ok(())
    }
}

/// Convert kernel evidence into its ABI-stable C carrier.
pub(crate) fn encode_evidence(evidence: &GovernorEvidence) -> PurrdfGovernorEvidence {
    PurrdfGovernorEvidence {
        consumed: encode_vector(evidence.consumed()),
        limits: encode_vector(evidence.limits()),
        trip: encode_trip(evidence.tripped()),
    }
}

fn encode_vector(vector: purrdf_core::ResourceVector) -> PurrdfResourceVector {
    PurrdfResourceVector {
        fuel: vector.get(ResourceDimension::Fuel),
        answer_rows: vector.get(ResourceDimension::AnswerRows),
        intermediate_cells: vector.get(ResourceDimension::IntermediateCells),
        scratch_bytes: vector.get(ResourceDimension::ScratchBytes),
        remote_requests: vector.get(ResourceDimension::RemoteRequests),
        udf_depth: vector.get(ResourceDimension::UdfDepth),
        pages: vector.get(ResourceDimension::Pages),
        bytes: vector.get(ResourceDimension::Bytes),
    }
}

fn encode_dimension(dimension: ResourceDimension) -> i32 {
    match dimension {
        ResourceDimension::Fuel => PurrdfResourceDimension::Fuel as i32,
        ResourceDimension::AnswerRows => PurrdfResourceDimension::AnswerRows as i32,
        ResourceDimension::IntermediateCells => PurrdfResourceDimension::IntermediateCells as i32,
        ResourceDimension::ScratchBytes => PurrdfResourceDimension::ScratchBytes as i32,
        ResourceDimension::RemoteRequests => PurrdfResourceDimension::RemoteRequests as i32,
        ResourceDimension::UdfDepth => PurrdfResourceDimension::UdfDepth as i32,
        ResourceDimension::Pages => PurrdfResourceDimension::Pages as i32,
        ResourceDimension::Bytes => PurrdfResourceDimension::Bytes as i32,
    }
}

fn encode_trip(tripped: Option<TrippedGovernor>) -> PurrdfGovernorTrip {
    let mut out = PurrdfGovernorTrip {
        kind: PurrdfGovernorTripKind::None as i32,
        dimension: -1,
        stop_cause: PurrdfStopCause::None as i32,
        limit: 0,
        consumed: 0,
        estimate: 0,
    };
    match tripped {
        None => {}
        Some(TrippedGovernor::Budget {
            dimension,
            limit,
            consumed,
        }) => {
            out.kind = PurrdfGovernorTripKind::Budget as i32;
            out.dimension = encode_dimension(dimension);
            out.limit = limit;
            out.consumed = consumed;
        }
        Some(TrippedGovernor::Stopped { cause }) => {
            out.kind = PurrdfGovernorTripKind::Stopped as i32;
            out.stop_cause = match cause {
                StopCause::Cancelled => PurrdfStopCause::Cancelled as i32,
                StopCause::Deadline => PurrdfStopCause::Deadline as i32,
            };
        }
        Some(TrippedGovernor::Refused {
            dimension,
            limit,
            estimate,
        }) => {
            out.kind = PurrdfGovernorTripKind::Refused as i32;
            out.dimension = encode_dimension(dimension);
            out.limit = limit;
            out.estimate = estimate;
        }
        Some(_) => out.kind = PurrdfGovernorTripKind::Unknown as i32,
    }
    out
}

/// One latched stop signal composed from the C cancellation and deadline sources.
#[derive(Debug)]
struct CStopWatch {
    latched: OnceLock<StopCause>,
    cancellation: Option<CancellationFlag>,
    deadline: Option<WallDeadline>,
}

impl CStopWatch {
    fn new(cancellation: Option<CancellationFlag>, deadline: Option<WallDeadline>) -> Self {
        Self {
            latched: OnceLock::new(),
            cancellation,
            deadline,
        }
    }

    const fn is_armed(&self) -> bool {
        self.cancellation.is_some() || self.deadline.is_some()
    }

    fn observe(&self) -> Option<StopCause> {
        if self
            .cancellation
            .as_ref()
            .is_some_and(CancellationFlag::is_cancelled)
        {
            return Some(StopCause::Cancelled);
        }
        self.deadline.as_ref().and_then(StopSignal::poll)
    }
}

impl StopSignal for CStopWatch {
    fn poll(&self) -> Option<StopCause> {
        if let Some(&cause) = self.latched.get() {
            return Some(cause);
        }
        let cause = self.observe()?;
        Some(*self.latched.get_or_init(|| cause))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initializer_is_metered_without_a_finite_caller_ceiling() {
        let mut config = PurrdfQueryGovernors::METERED;
        assert_eq!(unsafe { purrdf_query_governors_init(&raw mut config) }, 0);
        let governors = unsafe { decode_governors(&raw const config) }.expect("decode");
        assert_eq!(config.enabled, 0);
        assert_eq!(config.reserved, 0);
        assert!(governors.is_engaged());
        assert_eq!(
            governors.limits().get(ResourceDimension::Fuel),
            u64::MAX - 1
        );
    }

    #[test]
    fn cancellation_handle_is_shared_and_latched() {
        let mut cancellation = std::ptr::null_mut();
        assert_eq!(unsafe { purrdf_cancellation_new(&raw mut cancellation) }, 0);
        let mut observed = 1;
        assert_eq!(
            unsafe { purrdf_cancellation_is_cancelled(cancellation, &raw mut observed) },
            0
        );
        assert_eq!(observed, 0);
        assert_eq!(unsafe { purrdf_cancellation_cancel(cancellation) }, 0);
        assert_eq!(
            unsafe { purrdf_cancellation_is_cancelled(cancellation, &raw mut observed) },
            0
        );
        assert_eq!(observed, 1);
        unsafe { purrdf_cancellation_free(cancellation) };
    }
}
