// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Integer-only carrier admission before geometry and rational allocations.

use crate::{ExecutionPolicy, GeoError, MetricWorkObserver, Rat};

/// Admit the source before reader creation and preserve the original operational
/// refusal when a scanner reports an interrupted read in its place.
pub(crate) fn parse_in_policy<T>(
    source_bytes: usize,
    policy: ExecutionPolicy,
    parse: impl FnOnce(&std::cell::RefCell<ParseAdmission<'_, '_, '_>>) -> Result<T, GeoError>,
) -> Result<T, GeoError> {
    let admission = std::cell::RefCell::new(ParseAdmission::new(source_bytes, policy, None)?);
    let result = parse(&admission);
    if let Some(error) = admission.borrow_mut().take_error() {
        return Err(error);
    }
    result
}

/// One original physical account for a native carrier's containers and text.
///
/// The caller keeps this account alive until every produced payload is destroyed
/// or published with its original grant. It is separate from input/work polling:
/// the JSON scanner can poll the latter while this account prices its buffers.
pub struct CarrierStorage<'a> {
    memory: std::cell::RefCell<
        purrdf_lex::allocation::Memory<'a, dyn purrdf_lex::allocation::Admission + 'a>,
    >,
}
purrdf_hash::debug_non_exhaustive!(CarrierStorage<'_> {});

impl<'a> CarrierStorage<'a> {
    /// Begin under a fresh original account, before any carrier allocation.
    #[must_use]
    pub fn new(storage: &'a mut dyn purrdf_lex::allocation::Admission) -> Self {
        Self {
            memory: std::cell::RefCell::new(purrdf_lex::allocation::Memory::new(storage)),
        }
    }

    /// Resume the same account whose existing payloads own `admitted_bytes`.
    /// This allocates and resizes nothing; destruction still precedes refund.
    #[must_use]
    pub fn resume(
        storage: &'a mut dyn purrdf_lex::allocation::Admission,
        admitted_bytes: usize,
    ) -> Self {
        Self {
            memory: std::cell::RefCell::new(purrdf_lex::allocation::Memory::resume(
                storage,
                admitted_bytes,
            )),
        }
    }

    /// Original requested layouts still covered by this account.
    #[must_use]
    pub fn admitted_bytes(&self) -> usize {
        self.memory.borrow().admitted_bytes()
    }

    /// Lend the original memory view to a typed record producer. The producer
    /// borrows it only for each native growth, preserving nested callback order.
    #[must_use]
    pub fn record_memory(
        &self,
    ) -> purrdf_lex::json::record::RecordMemory<'_, 'a, dyn purrdf_lex::allocation::Admission + 'a>
    {
        purrdf_lex::json::record::RecordMemory::new(&self.memory)
    }

    /// The original account for short native record-builder borrows. Child
    /// producers must finish before a member borrows this cell; no new grant
    /// or allocation is made by exposing the immutable cell reference.
    #[must_use]
    pub fn memory(
        &self,
    ) -> &std::cell::RefCell<
        purrdf_lex::allocation::Memory<'a, dyn purrdf_lex::allocation::Admission + 'a>,
    > {
        &self.memory
    }

    /// Decode original ordered array elements under this same physical account.
    /// # Errors
    /// Returns unchanged lexical/element or original storage refusal.
    pub fn items<T, E: purrdf_lex::json::record::Within>(
        &self,
        value: &purrdf_lex::json::Value,
        read: impl FnMut(&purrdf_lex::json::Value) -> Result<T, E>,
    ) -> Result<Vec<T>, E> {
        let allocation = self.record_memory();
        purrdf_lex::json::record::items_with_memory(value, &self.memory, Some(&allocation), read)
    }

    /// Borrow the original physical account for one native producer. Any
    /// returned payload must remain enclosed by this account until destruction
    /// or ownership-carrying publication. Transient producers destroy their
    /// buffers before refunding their exact original layouts.
    pub fn native<T>(
        &self,
        body: impl FnOnce(
            &mut purrdf_lex::allocation::Memory<'_, dyn purrdf_lex::allocation::Admission + '_>,
        ) -> T,
    ) -> T {
        body(&mut self.memory.borrow_mut())
    }
}

/// Execute the same native body with either its original bounded storage or
/// the existing caller-owned resident account. Work polling remains separate.
pub(crate) fn with_storage<T>(
    storage: Option<&CarrierStorage<'_>>,
    body: impl FnOnce(
        &mut purrdf_lex::allocation::Memory<'_, dyn purrdf_lex::allocation::Admission + '_>,
    ) -> T,
) -> T {
    match storage {
        Some(storage) => storage.native(body),
        None => {
            let mut resident = purrdf_lex::allocation::Resident;
            body(&mut purrdf_lex::allocation::Memory::new(&mut resident))
        }
    }
}

/// Actual cumulative parser work and conservative retained source allowance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseReceipt {
    work: u64,
    workspace: u64,
}
impl ParseReceipt {
    /// Logical admitted parse work, including exact decimal construction.
    #[must_use]
    pub const fn work_items(self) -> u64 {
        self.work
    }
    /// Conservative complete retained source/scratch allowance.
    #[must_use]
    pub const fn workspace_bytes(self) -> u64 {
        self.workspace
    }
}

/// Integer-only admission for original source bytes and exact decimal construction.
/// It can serve a strict record reader before any typed coordinates are built.
pub struct ParseAdmission<'a, 'storage, 'account> {
    policy: ExecutionPolicy,
    observer: Option<&'a mut dyn MetricWorkObserver>,
    work: u64,
    workspace: u64,
    observed: usize,
    remainder: usize,
    coordinates: u64,
    error: Option<GeoError>,
    integer_scratch: Option<purrdf_xsd::integer::LimbScratch>,
    storage: Option<&'storage CarrierStorage<'account>>,
}
purrdf_hash::debug_non_exhaustive!(ParseAdmission<'_, '_, '_> { policy, work, workspace, coordinates });

impl<'a, 'storage, 'account> ParseAdmission<'a, 'storage, 'account> {
    /// Logical byte work and storage for borrowed source text before interpreting it.
    /// `retained_bytes` includes actual owned capacity. Sixty-three additional
    /// bytes per text byte retain the original carrier's conservative parser,
    /// interpretation and error-copy envelope, without counting spare capacity
    /// as byte work. The caller admits this allowance; no work is charged here.
    ///
    /// # Errors
    /// Refuses checked source-storage overflow.
    pub fn text_allowance(bytes: u64, retained_bytes: u64) -> Result<(u64, u64), GeoError> {
        let storage = bytes
            .checked_mul(63)
            .and_then(|copies| copies.checked_add(retained_bytes))
            .ok_or(GeoError::ArithmeticOverflow(
                "carrier source text allowance",
            ))?;
        Ok((bytes.div_ceil(16), storage))
    }

    /// Reserve complete source storage before parsing and charge the entry chunk.
    /// # Errors
    /// Refuses checked work/storage exhaustion or the observer's cancellation.
    pub fn new(
        bytes: usize,
        policy: ExecutionPolicy,
        observer: Option<&'a mut dyn MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        let bytes = u64::try_from(bytes)
            .map_err(|_| GeoError::ArithmeticOverflow("carrier source admission"))?;
        let workspace = Self::text_allowance(bytes, bytes)?
            .1
            .checked_add(1024)
            .ok_or(GeoError::ArithmeticOverflow("carrier source admission"))?;
        if workspace > policy.limits().max_workspace_bytes {
            return Err(GeoError::MemoryExhausted {
                limit: policy.limits().max_workspace_bytes,
            });
        }
        let mut admission = Self {
            policy,
            observer,
            work: 0,
            workspace,
            observed: 0,
            remainder: 0,
            coordinates: 0,
            error: None,
            integer_scratch: None,
            storage: None,
        };
        admission.charge(1, workspace)?;
        Ok(admission)
    }

    /// Actual already charged input and decimal-construction receipt.
    #[must_use]
    pub fn receipt(&self) -> ParseReceipt {
        ParseReceipt {
            work: self.work,
            workspace: self.workspace,
        }
    }

    /// Original caller-selected policy, without a replacement preset.
    #[must_use]
    pub const fn policy(&self) -> ExecutionPolicy {
        self.policy
    }

    /// Take the original latched operational cause after a reader was interrupted.
    pub fn take_error(&mut self) -> Option<GeoError> {
        self.error.take()
    }

    /// Attach the original physical account before any parser allocation.
    #[must_use]
    pub fn with_storage(mut self, storage: &'storage CarrierStorage<'account>) -> Self {
        self.storage = Some(storage);
        self
    }

    pub(crate) const fn storage(&self) -> Option<&'storage CarrierStorage<'account>> {
        self.storage
    }

    /// Validate and retain an original reference name under this source account.
    /// # Errors
    /// Preserves native IRI, physical allocation and original observer refusal.
    pub fn crs(&mut self, iri: &str) -> Result<crate::Crs, GeoError> {
        crate::Crs::validate(iri, self.storage)?;
        let text = match self.observer.as_deref_mut() {
            Some(observer) => observer.retain_text(iri)?,
            None => crate::carrier::shared_text_with_storage(iri, self.storage)?,
        };
        Ok(crate::Crs::from_admitted(text))
    }

    fn charge(&mut self, work: u64, growth: u64) -> Result<(), GeoError> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        let result = (|| {
            let limit = self.policy.limits().max_work_items;
            let total = self
                .work
                .checked_add(work)
                .ok_or(GeoError::WorkExhausted { limit })?;
            if total > limit {
                return Err(GeoError::WorkExhausted { limit });
            }
            self.work = total;
            if let Some(observer) = self.observer.as_deref_mut() {
                observer.charge_chunk(work, growth)?;
            }
            Ok(())
        })();
        if let Err(error) = &result {
            self.error = Some(error.clone());
        }
        result
    }

    pub(crate) fn advance_to(&mut self, offset: usize) -> bool {
        let bytes = offset.saturating_sub(self.observed);
        self.observed = self.observed.max(offset);
        self.advance(bytes)
    }
    pub(crate) fn advance(&mut self, bytes: usize) -> bool {
        let Some(total) = self.remainder.checked_add(bytes) else {
            self.error = Some(GeoError::ArithmeticOverflow("carrier byte work"));
            return false;
        };
        self.remainder = total % 16;
        self.charge((total / 16) as u64, 0).is_ok()
    }

    pub(crate) fn coordinate(&mut self) -> Result<(), GeoError> {
        let limit = self.policy.limits().max_output_elements;
        self.coordinates = self
            .coordinates
            .checked_add(1)
            .ok_or(GeoError::OutputExhausted { limit })?;
        if self.coordinates > limit {
            let error = GeoError::OutputExhausted { limit };
            self.error = Some(error.clone());
            return Err(error);
        }
        self.charge(1, 0)
    }

    /// Admit a bounded exact source or record walk before processing its items.
    /// # Errors
    /// Refuses exhausted work or the original observer's cancellation.
    pub fn admit_items(&mut self, items: u64) -> Result<(), GeoError> {
        self.charge(items, 0)
    }

    /// Admit additional live source ownership before copying or inspecting it.
    /// The existing byte parser allowance can already cover part of the source.
    pub(crate) fn retain_source(&mut self, bytes: u64) -> Result<(), GeoError> {
        let limit = self.policy.limits().max_workspace_bytes;
        let workspace = self
            .workspace
            .checked_add(bytes)
            .filter(|workspace| *workspace <= limit)
            .ok_or(GeoError::MemoryExhausted { limit })?;
        self.charge(0, bytes)?;
        self.workspace = workspace;
        Ok(())
    }

    fn diagnostic_number(
        &mut self,
        value: &Rat,
    ) -> Result<crate::context::Retained<Rat>, GeoError> {
        self.retained_diagnostic(
            value.detached_required_bytes().map_err(GeoError::Storage)?,
            |progress| GeoError::diagnostic_number(value, progress),
        )
    }

    /// Quote a native diagnostic's actual immutable control and payload through
    /// this parser's original policy/observer before the common factory runs.
    fn retained_diagnostic<T>(
        &mut self,
        payload: usize,
        make: impl FnOnce(
            &mut crate::context::WorkProgress<'_>,
        ) -> Result<crate::context::Retained<T>, GeoError>,
    ) -> Result<crate::context::Retained<T>, GeoError> {
        let control = crate::context::Retained::<T>::allocation_layout().size();
        let owner = match self.observer.as_ref() {
            Some(observer) => observer.retained_owner_layout(),
            None => crate::context::WorkProgress::integer(None).retained_owner_layout(),
        };
        let bytes = control
            .checked_add(owner.size())
            .and_then(|bytes| bytes.checked_add(payload))
            .and_then(|bytes| u64::try_from(bytes).ok())
            .ok_or(GeoError::Storage(
                purrdf_lex::allocation::StorageError::SizeOverflow,
            ))?;
        let previous = self.workspace;
        self.retain_source(bytes)?;
        let mut progress = crate::context::WorkProgress::integer(match &mut self.observer {
            Some(observer) => Some(*observer),
            None => None,
        });
        let result = make(&mut progress);
        if result.is_err() {
            self.workspace = previous;
        }
        result
    }

    /// Admit an original exact decimal spelling without constructing its value.
    /// Non-decimal strings do not perform numerical construction.
    /// # Errors
    /// Refuses work/storage exhaustion before integer or power allocation.
    pub fn admit_decimal_lexeme(&mut self, text: &str) -> Result<(), GeoError> {
        if let Some((digits, power)) = Rat::decimal_construction(text) {
            self.admit_construction(digits, power)?;
        }
        Ok(())
    }

    fn admit_construction(&mut self, digits: usize, power: u64) -> Result<(), GeoError> {
        let cost = purrdf_xsd::integer::ExactArithmeticCost::decimal(digits, power).ok_or(
            GeoError::ArithmeticOverflow("decimal construction admission"),
        )?;
        let work = cost.work_items;
        let growth = cost.workspace_bytes;
        let workspace =
            self.workspace
                .checked_add(growth)
                .ok_or_else(|| GeoError::MemoryExhausted {
                    limit: self.policy.limits().max_workspace_bytes,
                })?;
        if workspace > self.policy.limits().max_workspace_bytes {
            let error = GeoError::MemoryExhausted {
                limit: self.policy.limits().max_workspace_bytes,
            };
            self.error = Some(error.clone());
            return Err(error);
        }
        self.workspace = workspace;
        self.charge(work, growth)
    }

    /// Construct one exact decimal through the original admitted integer arena.
    /// # Errors
    /// Returns work, numerical or physical refusal; malformed decimals return None.
    pub fn decimal(&mut self, text: &str) -> Result<Option<Rat>, GeoError> {
        let result = self.decimal_construct(text);
        if let Err(error) = &result
            && self.error.is_none()
        {
            self.error = Some(error.clone());
        }
        result
    }

    /// Render the original rational on an explicit decimal grid. An absent
    /// grid requires a terminating exact spelling, without a scale preset.
    /// The shared prime-removal, rational rounding and integer text bodies are
    /// used directly; their actual destination arena and UTF-8 buffers are
    /// admitted before birth under this original account.
    /// # Errors
    /// Preserves work, native storage and nonterminating-value refusals.
    pub fn decimal_text(
        &mut self,
        value: &Rat,
        scale: Option<u32>,
        rounding: purrdf_xsd::ieee::ratio::Rounding,
    ) -> Result<String, GeoError> {
        use purrdf_xsd::integer::ExactArithmeticCost;
        let denominator_bits = value.denominator().bit_len();
        let scale = match scale {
            Some(scale) => scale,
            None => {
                let cost = ExactArithmeticCost::finite_decimal_scale(denominator_bits)
                    .ok_or(GeoError::ArithmeticOverflow("exact decimal scale"))?;
                self.charge(cost.work_items, 0)?;
                let limbs = usize::try_from(cost.output_bits.div_ceil(64))
                    .map_err(|_| GeoError::ArithmeticOverflow("exact decimal scale limbs"))?;
                // Prime removal retains the shifted odd denominator, its
                // current copy and one next quotient at the same time.
                self.prepare_destinations(if limbs > 3 { 3 } else { 0 }, limbs)?;
                let scale = value
                    .finite_decimal_scale_in(self.integer_scratch.as_ref().expect("scale arena"))
                    .map_err(GeoError::NumericalScratch)?;
                let Some(scale) = scale else {
                    let previous = self.workspace;
                    let captured = (|| {
                        let value = self.diagnostic_number(value)?;
                        self.retained_diagnostic(0, |progress| {
                            crate::context::Retained::produce(
                                progress,
                                |_| {
                                    Ok(crate::GeometryDomainError::NonDecimalParameter {
                                        value,
                                        rounding,
                                    })
                                },
                                |_, _| Ok(()),
                            )
                        })
                    })();
                    return Err(match captured {
                        Ok(cause) => GeoError::GeometryDomain(cause),
                        Err(error) => {
                            self.workspace = previous;
                            error
                        }
                    });
                };
                u32::try_from(scale)
                    .map_err(|_| GeoError::ArithmeticOverflow("exact decimal scale"))?
            }
        };
        let cost = ExactArithmeticCost::rational_decimal(
            value.numerator().bit_len(),
            denominator_bits,
            scale,
        )
        .ok_or(GeoError::ArithmeticOverflow("rational decimal rendering"))?;
        self.charge(cost.work_items, 0)?;
        let width = cost
            .output_bits
            .max(denominator_bits)
            .checked_add(1)
            .ok_or(GeoError::ArithmeticOverflow("decimal division guard limb"))?;
        let limbs = usize::try_from(width.div_ceil(64))
            .map_err(|_| GeoError::ArithmeticOverflow("decimal destination limbs"))?;
        // At normalized division's peak: power, absolute divisor, absolute
        // numerator and factor, product, and its three division destinations.
        // Power construction and the subsequent rounding each use fewer slots.
        self.prepare_destinations(if limbs > 3 { 8 } else { 0 }, limbs)?;
        let coefficient = value
            .round_to_scale_with_in(
                scale,
                rounding,
                self.integer_scratch.as_ref().expect("decimal arena"),
            )
            .map_err(GeoError::NumericalScratch)?;
        let rendered = with_storage(self.storage, |memory| {
            let mut output = String::new();
            purrdf_xsd::exact::append_scaled_integer_with_memory(
                &coefficient,
                scale,
                &mut output,
                memory,
            )
            .map(|()| output)
            .map_err(|error| match error {
                purrdf_xsd::exact::DecimalTextError::Storage(error) => GeoError::Storage(error),
                purrdf_xsd::exact::DecimalTextError::Numeric(error) => {
                    GeoError::NumericalScratch(error)
                }
            })
        });
        drop(coefficient);
        rendered
    }

    /// Render a borrowed integer through the sole admitted group/text body.
    /// # Errors
    /// Refuses original work, physical storage or native numerical capacity.
    pub fn integer_text(&mut self, value: &crate::Int) -> Result<String, GeoError> {
        let cost = purrdf_xsd::integer::ExactArithmeticCost::for_operation(
            purrdf_xsd::integer::ExactOperation::DecimalRender,
            value.bit_len(),
            1,
        )
        .ok_or(GeoError::ArithmeticOverflow(
            "certificate integer rendering",
        ))?;
        self.charge(cost.work_items, 0)?;
        self.scaled_integer_text(value, 0)
    }

    pub(crate) fn scaled_integer_text(
        &self,
        value: &crate::Int,
        scale: u32,
    ) -> Result<String, GeoError> {
        with_storage(self.storage, |memory| {
            let mut output = String::new();
            purrdf_xsd::exact::append_scaled_integer_with_memory(value, scale, &mut output, memory)
                .map(|()| output)
                .map_err(|error| match error {
                    purrdf_xsd::exact::DecimalTextError::Storage(error) => GeoError::Storage(error),
                    purrdf_xsd::exact::DecimalTextError::Numeric(error) => {
                        GeoError::NumericalScratch(error)
                    }
                })
        })
    }

    fn decimal_construct(&mut self, text: &str) -> Result<Option<Rat>, GeoError> {
        let Some((digits, power)) = Rat::decimal_construction(text) else {
            return Ok(None);
        };
        // This allocation-free metadata scan precedes the original constructor;
        // its own input work is charged in addition to the constructor receipt.
        self.charge(
            u64::try_from(text.len().div_ceil(16))
                .map_err(|_| GeoError::ArithmeticOverflow("decimal metadata scan"))?,
            0,
        )?;
        self.admit_construction(digits, power)?;
        let (required, limbs) =
            Rat::decimal_scratch_layout(digits, power).map_err(GeoError::NumericalScratch)?;
        self.prepare_destinations(required, limbs)?;
        let scratch = self
            .integer_scratch
            .as_ref()
            .expect("original decimal arena prepared before construction");
        match Rat::parse_decimal_in(text, &mut |_, _| Ok::<(), GeoError>(()), scratch) {
            Ok(value) => Ok(value),
            Err(purrdf_xsd::integer::DecimalConstructionError::Admission(error)) => Err(error),
            Err(purrdf_xsd::integer::DecimalConstructionError::Storage(error)) => {
                Err(GeoError::NumericalScratch(error))
            }
        }
    }
    fn prepare_destinations(&mut self, required: usize, limbs: usize) -> Result<(), GeoError> {
        let reusable = self.integer_scratch.as_ref().is_some_and(|scratch| {
            required == 0 || (scratch.limb_capacity() >= limbs && scratch.available() >= required)
        });
        if !reusable {
            let maximum = self.policy.limits().max_scratch_destinations as usize;
            if required > maximum {
                return Err(GeoError::NumericalScratch(
                    purrdf_xsd::integer::LimbScratchError::Exhausted {
                        destinations: maximum,
                    },
                ));
            }
            // Geometric growth amortizes the actual slot/control construction.
            // Old immutable coordinates retain their own original arena grants;
            // replacing this handle cannot detach either their limbs or owners.
            let buffers = if required == 0 {
                0
            } else {
                self.integer_scratch.as_ref().map_or(Ok(required), |old| {
                    old.destination_capacity()
                        .checked_mul(2)
                        .map(|count| count.max(required).min(maximum))
                        .ok_or(GeoError::ArithmeticOverflow("source integer arena growth"))
                })?
            };
            let bytes = match self.observer.as_ref() {
                Some(observer) => observer.integer_scratch_layout(buffers, limbs)?,
                None => purrdf_xsd::integer::LimbScratch::required_bytes(buffers, limbs)
                    .map_err(GeoError::NumericalScratch)?,
            };
            let bytes = u64::try_from(bytes)
                .map_err(|_| GeoError::ArithmeticOverflow("decimal arena layout"))?;
            self.retain_source(bytes)?;
            self.charge(
                u64::try_from(buffers)
                    .ok()
                    .and_then(|count| count.checked_add(1))
                    .ok_or(GeoError::ArithmeticOverflow("decimal arena initialization"))?,
                0,
            )?;
            let scratch = match self.observer.as_deref_mut() {
                Some(observer) => observer.prepare_integer_scratch(buffers, limbs)?,
                None => {
                    if let Some(storage) = self.storage {
                        storage
                            .native(|memory| {
                                memory.add_bytes(usize::try_from(bytes).map_err(|_| {
                                    purrdf_lex::allocation::StorageError::SizeOverflow
                                })?)
                            })
                            .map_err(GeoError::Storage)?;
                    }
                    purrdf_xsd::integer::LimbScratch::new(buffers, limbs)
                        .map_err(GeoError::NumericalScratch)?
                }
            };
            self.integer_scratch = Some(scratch);
        }
        Ok(())
    }

    /// Decode a finite original binary64 ordinate through the shared dyadic home.
    /// # Errors
    /// Returns typed nonfinite, native capacity and original physical refusal.
    pub fn binary64(&mut self, axis: &'static str, value: f64) -> Result<Rat, GeoError> {
        let decoded = purrdf_xsd::ieee::dyadic::Binary64Dyadic::decode(value).ok_or(
            GeoError::NonFiniteCoordinate {
                axis,
                bits: value.to_bits(),
            },
        )?;
        let power = decoded.exponent().unsigned_abs();
        let cost = purrdf_xsd::integer::ExactArithmeticCost::binary64_rational(value.to_bits())
            .ok_or(GeoError::ArithmeticOverflow("binary64 source construction"))?;
        self.charge(cost.work_items, 0)?;
        let numerator_bits = u64::from(decoded.significand().bit_width())
            .checked_add(if decoded.exponent() > 0 {
                u64::from(power)
            } else {
                0
            })
            .ok_or(GeoError::ArithmeticOverflow("binary64 numerator layout"))?;
        let denominator_bits = if decoded.exponent() < 0 {
            u64::from(power) + 1
        } else {
            1
        };
        let width = numerator_bits.max(denominator_bits);
        // The normalized significand is inline. Exactly one shift destination
        // can exceed the inline representation: numerator or denominator.
        let limbs = usize::try_from(width.div_ceil(64))
            .map_err(|_| GeoError::ArithmeticOverflow("binary64 source limbs"))?;
        self.prepare_destinations(usize::from(limbs > 3), limbs)?;
        Rat::from_binary64_in(value, self.integer_scratch.as_ref().expect("source arena"))
            .map_err(GeoError::NumericalScratch)?
            .ok_or(GeoError::NonFiniteCoordinate {
                axis,
                bits: value.to_bits(),
            })
    }

    /// Validate the original exact coordinate range under admitted comparisons.
    /// # Errors
    /// Preserves the same original axis/range failure and native refusal.
    pub fn location(&mut self, longitude: Rat, latitude: Rat) -> Result<crate::LonLat, GeoError> {
        let cost = purrdf_xsd::integer::ExactArithmeticCost::for_rational_operands(
            purrdf_xsd::integer::ExactOperation::RationalCompare,
            [
                (
                    longitude.numerator().bit_len(),
                    longitude.denominator().bit_len(),
                ),
                (
                    latitude.numerator().bit_len(),
                    latitude.denominator().bit_len(),
                ),
                (8, 1),
            ],
            4,
        )
        .ok_or(GeoError::ArithmeticOverflow(
            "source coordinate range comparison",
        ))?;
        self.charge(cost.work_items, 0)?;
        let limbs = usize::try_from(cost.output_bits.div_ceil(64))
            .ok()
            .and_then(|n| n.checked_add(1))
            .ok_or(GeoError::ArithmeticOverflow("source comparison limbs"))?;
        // The native comparison holds at most its two cross products. On a
        // range error they die before the original numerator/denominator copy.
        self.prepare_destinations(if limbs > 3 { 2 } else { 0 }, limbs)?;
        let scratch = self.integer_scratch.as_ref().expect("source arena").clone();
        let admission = std::cell::RefCell::new(self);
        let diagnostic = |value: &Rat| admission.borrow_mut().diagnostic_number(value);
        crate::LonLat::new_using(
            longitude,
            latitude,
            &crate::numerical::ExactValues::with_diagnostics(Some(&scratch), &diagnostic),
        )
    }

    /// Validate the original closed geographic box under exact native products.
    /// # Errors
    /// Preserves longitude/latitude range order, inversion and native refusal.
    pub fn closed_box(
        &mut self,
        west: Rat,
        south: Rat,
        east: Rat,
        north: Rat,
    ) -> Result<crate::cells::ClosedBox, GeoError> {
        let cost = purrdf_xsd::integer::ExactArithmeticCost::for_rational_operands(
            purrdf_xsd::integer::ExactOperation::RationalCompare,
            [&west, &south, &east, &north]
                .into_iter()
                .map(|value| (value.numerator().bit_len(), value.denominator().bit_len()))
                .chain([(8, 1)]),
            9,
        )
        .ok_or(GeoError::ArithmeticOverflow("source box range comparison"))?;
        self.charge(cost.work_items, 0)?;
        let limbs = usize::try_from(cost.output_bits.div_ceil(64))
            .ok()
            .and_then(|count| count.checked_add(1))
            .ok_or(GeoError::ArithmeticOverflow("source box comparison limbs"))?;
        // Each of the eight angular comparisons and the interval comparison
        // retains at most its two cross products; subsequent comparisons reuse
        // those original destinations after the previous products die.
        self.prepare_destinations(if limbs > 3 { 2 } else { 0 }, limbs)?;
        let scratch = self.integer_scratch.as_ref().expect("source arena").clone();
        let admission = std::cell::RefCell::new(self);
        let diagnostic = |value: &Rat| admission.borrow_mut().diagnostic_number(value);
        crate::cells::ClosedBox::new_using(
            west,
            south,
            east,
            north,
            crate::numerical::ExactValues::with_diagnostics(Some(&scratch), &diagnostic),
        )
    }
}

impl purrdf_lex::json::ReadObserver for ParseAdmission<'_, '_, '_> {
    fn advance(&mut self, bytes: usize) -> bool {
        self.advance(bytes)
    }
}

pub(crate) struct JsonProgress<'a, 'observer, 'storage, 'account>(
    pub(crate) &'a std::cell::RefCell<ParseAdmission<'observer, 'storage, 'account>>,
);
impl purrdf_lex::json::ReadObserver for JsonProgress<'_, '_, '_, '_> {
    fn advance(&mut self, bytes: usize) -> bool {
        self.0.borrow_mut().advance(bytes)
    }
}

pub(crate) fn whitespace_only(
    lexical: &str,
    admission: Option<&std::cell::RefCell<ParseAdmission<'_, '_, '_>>>,
) -> Result<bool, GeoError> {
    for (index, chunk) in lexical.as_bytes().chunks(256).enumerate() {
        if let Some(admission) = admission {
            let mut admission = admission.borrow_mut();
            if !admission.advance_to((index * 256 + chunk.len()).min(lexical.len())) {
                return Err(admission
                    .take_error()
                    .expect("refused carrier input has a cause"));
            }
        }
        if !chunk.iter().copied().all(purrdf_iri::terminals::is_ws) {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_xsd::ieee::ratio::Rounding;

    #[test]
    fn nonterminating_profile_value_retains_exact_operand_and_selected_rounding() {
        let value = Rat::new(crate::Int::one().shl(4096), crate::Int::from_i64(3)).unwrap();
        for rounding in [Rounding::NearestEven, Rounding::Down, Rounding::Up] {
            let mut admission = ParseAdmission::new(0, ExecutionPolicy::geometry(), None).unwrap();
            let error = admission.decimal_text(&value, None, rounding).unwrap_err();
            drop(admission);
            assert!(matches!(error.original(), GeoError::GeometryDomain(cause)
                if matches!(&**cause, crate::GeometryDomainError::NonDecimalParameter { value: original, rounding: actual }
                    if **original == value && *actual == rounding)));
            assert_eq!(
                error.detail(),
                "custom profile parameter has no exact decimal representation"
            );
            let capture =
                purrdf_testkit::error_capture::Capture::read(error.original()).child("cause");
            assert_eq!(capture.child("value").code, "geometric-rational");
            assert_eq!(
                capture.child("rounding").code,
                match rounding {
                    Rounding::NearestEven => "rounding-nearest-even",
                    Rounding::Down => "rounding-down",
                    Rounding::Up => "rounding-up",
                }
            );
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let last = error.clone();
            assert_eq!(window.close().allocations, 0);
            drop(error);
            assert!(last.is_expression_error());
        }
    }

    #[test]
    fn nondecimal_diagnostic_respects_original_parser_reservation_before_allocation() {
        let value = Rat::new(crate::Int::one(), crate::Int::from_i64(3)).unwrap();
        let mut admission = ParseAdmission::new(0, ExecutionPolicy::geometry(), None).unwrap();
        admission.prepare_destinations(0, 1).unwrap();
        let baseline = admission.workspace;
        admission.policy = ExecutionPolicy::new(crate::ExecutionLimits {
            max_workspace_bytes: baseline,
            ..crate::ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let error = admission
            .decimal_text(&value, None, Rounding::NearestEven)
            .unwrap_err();
        assert!(
            matches!(error.original(), GeoError::MemoryExhausted { limit } if *limit == baseline)
        );
        assert_eq!(admission.workspace, baseline);
        assert_eq!(window.close().allocations, 0);
    }
}
