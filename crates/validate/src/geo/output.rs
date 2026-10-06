// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete response allocation is admitted before result record construction.

use purrdf_geo_kernel::{GeoError, MetricContext, PreparationBudget, Rat};
use purrdf_lex::json::OutputLayout;
use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation};

type Result<T> = std::result::Result<T, GeoError>;

fn checked<T>(value: Option<T>) -> Result<T> {
    value.ok_or(GeoError::ArithmeticOverflow("geographic output layout"))
}

/// Every version-one response field name is at most 64 UTF-8 bytes. This bound
/// includes numbers as text, so their lexemes and String capacity are covered.
pub(super) fn record(members: u64, payload: u64) -> Result<OutputLayout> {
    checked(OutputLayout::record(
        members,
        checked(members.checked_mul(64))?
            .checked_add(payload)
            .ok_or(GeoError::ArithmeticOverflow("geographic output text"))?,
    ))
}

pub(super) fn array(count: usize, element: OutputLayout) -> Result<OutputLayout> {
    checked(OutputLayout::array(count as u64, element))
}

pub(super) fn batch<T>(
    answers: &[Option<T>],
    element: impl Fn(&T) -> Result<OutputLayout>,
) -> Result<OutputLayout> {
    let mut layout = array(answers.len(), OutputLayout::scalar())?;
    for answer in answers {
        layout = checked(
            layout.with_child(element(
                answer
                    .as_ref()
                    .expect("successful batch fills every result"),
            )?),
        )?;
    }
    Ok(layout)
}

pub(super) fn identity() -> Result<OutputLayout> {
    let fields = super::encode::IDENTITY_MEMBER_COUNT;
    record(fields, fields * 64)
}

pub(super) fn refusal(error: &super::GeoCallError) -> Result<OutputLayout> {
    let mut fields = Fields::default();
    // The error schema has at most fourteen fixed scalar payloads (each no
    // larger than a 24-byte IEEE/integer token) and 176 static message/code
    // bytes. Original text and arbitrary exact integer spelling are added below;
    // record() separately covers every field name and nested container.
    let mut fixed = 512_u64;
    let mut rational = |value: &Rat| -> Result<()> {
        // exact_rational proves the original denominator has a finite decimal,
        // then emits that spelling or both integers if it is nonterminating. The Display message
        // independently renders those integers. Admit the complete branch bound.
        fields.exact(value)?;
        fields.integer(value.numerator(), 2)?;
        fields.integer(value.denominator(), 2)
    };
    match error {
        super::GeoCallError::Decode(error) => {
            fixed = checked(fixed.checked_add(error.message().len() as u64))?;
            fixed = checked(fixed.checked_add(2 * error.pointer().len() as u64))?;
        }
        super::GeoCallError::Engine(error) => {
            use purrdf_geo_kernel::GeoError;
            match error {
                GeoError::CoordinateOutOfRange { axis, value } => {
                    rational(value)?;
                    fixed = checked(fixed.checked_add(2 * axis.len() as u64))?;
                }
                GeoError::NonFiniteCoordinate { axis, .. } => {
                    fixed = checked(fixed.checked_add(2 * axis.len() as u64))?;
                }
                GeoError::NonPositiveEdgeLength(value)
                | GeoError::NegativePhysicalRadius(value) => rational(value)?,
                GeoError::UnattainableEdgeLength { target, minimum } => {
                    rational(target)?;
                    rational(minimum)?;
                }
                GeoError::MissingOperation { source, target } => {
                    fixed = checked(fixed.checked_add(2 * source.len() as u64))?;
                    fixed = checked(fixed.checked_add(2 * target.len() as u64))?;
                }
                GeoError::FloatEnvironment(error) => {
                    use purrdf_xsd::ieee::environment::{
                        FloatEnvironmentError, FloatEnvironmentEvidence,
                    };
                    let evidence = match error {
                        FloatEnvironmentError::TrapsEnabled { evidence }
                        | FloatEnvironmentError::FlushToZero { evidence }
                        | FloatEnvironmentError::RoundingMode { evidence }
                        | FloatEnvironmentError::DoubleRounding { evidence } => Some(evidence),
                        _ => None,
                    };
                    let text = match evidence {
                        Some(FloatEnvironmentEvidence::Register { name, .. }) => name.len(),
                        Some(FloatEnvironmentEvidence::Probe { operation, .. }) => operation.len(),
                        _ => 0,
                    };
                    fixed = checked(fixed.checked_add(2 * text as u64))?;
                }
                _ => {}
            }
            fixed = checked(fixed.checked_add(2 * error.detail().len() as u64))?;
        }
    }
    checked(
        fields
            .finish(14, fixed, false)?
            .with_child(record(4, 0)?)
            .and_then(|layout| layout.with_child(record(4, 0).ok()?)),
    )
}

pub(super) fn envelope() -> Result<OutputLayout> {
    checked(record(3, 8)?.with_child(identity()?))
}

pub(super) fn admit(budget: PreparationBudget, layout: OutputLayout) -> Result<()> {
    let mut complete = budget;
    reserve(&mut complete, layout)
}

pub(super) fn reserve(budget: &mut PreparationBudget, layout: OutputLayout) -> Result<()> {
    let layout = checked(envelope()?.with_child(layout))?;
    budget.retain(layout.work_items(), checked(layout.workspace_bytes())?)
}

pub(super) fn admit_context(context: &mut MetricContext, layout: OutputLayout) -> Result<()> {
    let layout = checked(envelope()?.with_child(layout))?;
    // Complete encoding lives alongside the native answer/worker caches. The
    // reservation covers the following writer even after native answers drop.
    context.admit_workspace(checked(layout.workspace_bytes())?)?;
    context.charge_work(layout.work_items())
}

pub(super) fn cell() -> Result<OutputLayout> {
    record(7, 64 + 5 * 16 + 3)
}
pub(super) fn range() -> Result<OutputLayout> {
    record(6, 64 + 3 * 16 + 22)
}

#[derive(Default)]
struct Fields {
    text: u64,
    work: u64,
    workspace: u64,
}
impl Fields {
    fn cost(&mut self, cost: ExactArithmeticCost, copies: u64) -> Result<()> {
        self.work = checked(
            self.work
                .checked_add(checked(cost.work_items.checked_mul(copies))?),
        )?;
        self.workspace = self.workspace.max(cost.workspace_bytes);
        Ok(())
    }
    fn integer(&mut self, value: &purrdf_geo_kernel::Int, copies: u64) -> Result<()> {
        let bits = value.bit_len();
        let bytes = checked(bits.div_ceil(3).checked_add(2))?;
        self.text = checked(self.text.checked_add(bytes))?;
        self.cost(
            checked(ExactArithmeticCost::for_operation(
                ExactOperation::DecimalRender,
                bits,
                1,
            ))?,
            copies,
        )
    }
    fn decimal(&mut self, value: &Rat, places: u32, copies: u64) -> Result<()> {
        self.decimal_shape(
            value.numerator().bit_len(),
            value.denominator().bit_len(),
            places,
            copies,
        )
    }
    fn decimal_shape(
        &mut self,
        numerator: u64,
        denominator: u64,
        places: u32,
        copies: u64,
    ) -> Result<()> {
        let cost = checked(ExactArithmeticCost::rational_decimal(
            numerator,
            denominator,
            places,
        ))?;
        let bytes = checked(
            cost.output_bits
                .div_ceil(3)
                .checked_add(u64::from(places))
                .and_then(|n| n.checked_add(8)),
        )?;
        self.text = checked(self.text.checked_add(bytes))?;
        self.cost(cost, copies)
    }
    fn exact(&mut self, value: &Rat) -> Result<()> {
        self.cost(
            checked(ExactArithmeticCost::finite_decimal_scale(
                value.denominator().bit_len(),
            ))?,
            1,
        )?;
        // A finite decimal's minimal scale is bounded by the denominator's bit
        // width. The original finite-scale proof decides whether formatting is
        // needed; no decimal reparsing or rational comparison is executed.
        let scale = u32::try_from(value.denominator().bit_len())
            .unwrap_or(100_000)
            .min(100_000);
        self.decimal(value, scale, 1)
    }

    fn outward(&mut self, bound: &Rat) -> Result<()> {
        self.outward_shape(bound.numerator().bit_len(), bound.denominator().bit_len())
    }
    fn outward_shape(&mut self, numerator: u64, denominator: u64) -> Result<()> {
        // The shared formatter rounds directly Down/Up once, then emits the
        // same canonical mantissa spelling. No intermediate Rat is reduced.
        self.decimal_shape(numerator, denominator, 36, 1)
    }
    fn finish(self, members: u64, fixed: u64, certificate: bool) -> Result<OutputLayout> {
        // All certificate formats have at most 512 fixed/tag/framing bytes.
        // Numerical fields are rendered again into their framed certificate.
        let certificate_bytes = if certificate {
            checked(
                512_u64
                    .checked_add(self.text)
                    .and_then(|n| n.checked_mul(2)),
            )?
        } else {
            0
        };
        let text = checked(
            self.text
                .checked_add(fixed)
                .and_then(|n| n.checked_add(certificate_bytes)),
        )?;
        checked(record(members, text)?.with_formatting(
            self.work,
            checked(self.workspace.checked_add(certificate_bytes))?,
        ))
    }
}

pub(super) fn metric(answer: &purrdf_geo_kernel::MetricEstimate) -> Result<OutputLayout> {
    let mut fields = Fields::default();
    fields.decimal(answer.value().exact(), 6, 1)?;
    fields.integer(answer.quantized_micrometres(), 2)?;
    let bound = answer.host_conversion_bound().exact();
    fields.integer(bound.numerator(), 1)?;
    fields.integer(bound.denominator(), 1)?;
    fields.outward(bound)?;
    fields.finish(9, 64 * 2 + 16 + 24, true)
}

pub(super) fn direct(answer: &purrdf_geo_kernel::geodesic::DirectResult) -> Result<OutputLayout> {
    let mut fields = Fields::default();
    let places = answer.output_grid().decimal_places();
    for value in [
        answer.endpoint().longitude(),
        answer.endpoint().latitude(),
        answer.final_azimuth(),
    ] {
        fields.decimal(value, places, 2)?;
    }
    // Direct's certificate also renders the explicitly declared degree quantum.
    let quantum_bits = checked(
        u64::from(places)
            .checked_mul(4)
            .and_then(|n| n.checked_add(1)),
    )?;
    fields.cost(
        checked(ExactArithmeticCost::decimal_rational(1, places))?,
        1,
    )?;
    fields.cost(
        checked(ExactArithmeticCost::rational_decimal(
            1,
            quantum_bits,
            places,
        ))?,
        1,
    )?;
    fields.text = checked(
        fields
            .text
            .checked_add(u64::from(places))
            .and_then(|n| n.checked_add(8)),
    )?;
    fields.finish(7, 64 * 2 + 16, true)
}

pub(super) fn inverse(answer: &purrdf_geo_kernel::geodesic::InverseResult) -> Result<OutputLayout> {
    let mut fields = Fields::default();
    for value in answer
        .forward_azimuth()
        .into_iter()
        .chain(answer.final_azimuth())
        .chain([answer.arc_degrees(), answer.scale12(), answer.scale21()])
    {
        fields.decimal(value, 15, 2)?;
    }
    fields.decimal(answer.reduced_length().exact(), 6, 2)?;
    fields.decimal(answer.geodesic_quadrilateral_area().exact(), 2, 2)?;
    for (latitude, from, to) in answer.pole_cuts() {
        for value in [latitude, from, to] {
            fields.decimal(value, 15, 2)?;
        }
    }
    let child = checked(
        metric(answer.distance())?.with_child(array(answer.pole_cuts().len(), record(3, 3 * 32)?)?),
    )?;
    checked(
        checked(fields.finish(13, 64 * 2 + 64, true)?.with_child(child))?
            .with_child(metric(answer.distance())?),
    )
}

pub(super) fn transform(answer: &purrdf_geo_kernel::TransformResult) -> Result<OutputLayout> {
    let mut fields = Fields::default();
    let point = answer.point();
    for value in [&point.x, &point.y]
        .into_iter()
        .chain(point.z.iter())
        .chain(point.epoch.iter())
    {
        fields.exact(value)?;
        fields.integer(value.numerator(), 1)?;
        fields.integer(value.denominator(), 1)?;
    }
    fields.finish(10, 64 * 2 + 32, true)
}

pub(super) fn term(term: &purrdf_core::TermValue) -> Result<OutputLayout> {
    let bytes = match term {
        purrdf_core::TermValue::Iri(iri) => iri.as_str().len(),
        purrdf_core::TermValue::Literal {
            lexical_form,
            datatype,
            language,
            ..
        } => lexical_form
            .as_str()
            .len()
            .checked_add(datatype.as_str().len())
            .and_then(|n| n.checked_add(language.as_ref().map_or(0, String::len)))
            .ok_or(GeoError::ArithmeticOverflow("geometry term output"))?,
        _ => 0,
    };
    record(5, checked((bytes as u64).checked_add(32))?)
}

pub(super) fn proof(receipt: &purrdf_geo_kernel::MetricProofReceipt) -> Result<OutputLayout> {
    let mut fields = Fields::default();
    fields.outward(receipt.lower.exact())?;
    fields.outward(receipt.upper.exact())?;
    fields.finish(4, 40, false)
}

pub(super) fn inverse_proof(
    receipt: &purrdf_geo_kernel::geodesic::InverseProofReceipt,
) -> Result<OutputLayout> {
    let mut fields = Fields::default();
    for (lower, upper) in receipt
        .forward_azimuth
        .iter()
        .chain(receipt.final_azimuth.iter())
        .chain([&receipt.arc_degrees, &receipt.scale12, &receipt.scale21])
    {
        fields.outward(lower)?;
        fields.outward(upper)?;
    }
    for bound in [
        receipt.reduced_length.0.exact(),
        receipt.reduced_length.1.exact(),
        receipt.geodesic_quadrilateral_area.0.exact(),
        receipt.geodesic_quadrilateral_area.1.exact(),
    ] {
        fields.outward(bound)?;
    }
    let scalar = fields.finish(8, 64, false)?;
    checked(checked(scalar.with_child(record(14, 0)?))?.with_child(proof(&receipt.distance)?))
}

pub(super) fn geometry_metric(
    estimate: &purrdf_geo_kernel::ellipsoidal::GeometryMetricEstimate,
) -> Result<OutputLayout> {
    let mut fields = Fields::default();
    fields.decimal(estimate.exact(), if estimate.is_area() { 2 } else { 6 }, 2)?;
    let numerator = estimate.exact().numerator().bit_len();
    let denominator = estimate.exact().denominator().bit_len();
    // Both the response and its canonical certificate construct the same frozen
    // absolute/relative maximum. Admission uses operand metadata and never
    // computes the bound before its exact operations are charged.
    fields.cost(checked(ExactArithmeticCost::decimal_rational(47, 14))?, 4)?;
    fields.cost(
        checked(ExactArithmeticCost::for_rational_operands(
            ExactOperation::Linear,
            [(numerator, denominator)],
            1,
        ))?,
        2,
    )?;
    fields.cost(
        checked(ExactArithmeticCost::for_rational_operands(
            ExactOperation::RationalMultiply,
            [(numerator, denominator), (1, 47)],
            1,
        ))?,
        2,
    )?;
    let bound_numerator = checked(numerator.checked_add(47))?.max(20);
    let bound_denominator = checked(denominator.checked_add(47))?.max(20);
    fields.cost(
        checked(ExactArithmeticCost::for_rational_operands(
            ExactOperation::RationalCompare,
            [(bound_numerator, bound_denominator), (20, 20)],
            1,
        ))?,
        2,
    )?;
    fields.outward_shape(bound_numerator, bound_denominator)?;
    // The certificate additionally renders exact bound numerator/denominator.
    for bits in [bound_numerator, bound_denominator] {
        fields.cost(
            checked(ExactArithmeticCost::for_operation(
                ExactOperation::DecimalRender,
                bits,
                1,
            ))?,
            1,
        )?;
        fields.text = checked(
            fields
                .text
                .checked_add(checked(bits.div_ceil(3).checked_add(2))?),
        )?;
    }
    fields.finish(7, 64 * 2 + 32, true)
}

pub(super) fn operation_chain(chain: &purrdf_geo_kernel::OperationChain) -> Result<OutputLayout> {
    let mut layout = array(chain.operations().len(), OutputLayout::scalar())?;
    for operation in chain.operations() {
        let mut fields = Fields::default();
        let mut count = 0_u64;
        let mut error = None;
        operation.model().visit_original_operands(&mut |value| {
            if error.is_none() {
                count = match count.checked_add(1) {
                    Some(count) => count,
                    None => {
                        error = Some(GeoError::ArithmeticOverflow(
                            "operation output operand count",
                        ));
                        return;
                    }
                };
                if let Err(refusal) = fields.exact(value) {
                    error = Some(refusal);
                }
            }
        });
        if let Some(error) = error {
            return Err(error);
        }
        use purrdf_geo_kernel::operation::OperationModel;
        let children = match operation.model() {
            OperationModel::BilinearGrid(grid) => grid.nodes().len(),
            OperationModel::BilinearGridInverse(parameters) => parameters.grid.nodes().len(),
            OperationModel::Polynomial2d(polynomial) => polynomial.terms().len(),
            OperationModel::Polynomial2dInverse(parameters) => parameters.polynomial.terms().len(),
            _ => 0,
        };
        let members = checked(count.checked_mul(4).and_then(|n| n.checked_add(32)))?;
        let element = fields.finish(members, 512, false)?;
        let element = checked(element.with_child(array(children, record(4, 64)?)?))?;
        layout = checked(layout.with_child(element))?;
    }
    Ok(layout)
}

pub(super) fn profile(profile: &purrdf_geo_kernel::GeoProfile) -> Result<OutputLayout> {
    let mut references = array(profile.references().len(), OutputLayout::scalar())?;
    for binding in profile.references() {
        let mut fields = Fields::default();
        fields.exact(binding.reference().ellipsoid().semimajor())?;
        fields.exact(binding.reference().ellipsoid().inverse_flattening())?;
        let item = fields.finish(5, (binding.crs().as_str().len() + 64 + 32) as u64, false)?;
        references = checked(references.with_child(item))?;
    }
    let mut operations = array(profile.operations().len(), OutputLayout::scalar())?;
    for binding in profile.operations() {
        let names = (binding.name().as_str().len()
            + binding.source().as_str().len()
            + binding.target().as_str().len()) as u64;
        let item = checked(record(4, names)?.with_child(operation_chain(binding.chain())?))?;
        operations = checked(operations.with_child(item))?;
    }
    let mut units = array(profile.linear_units().len(), OutputLayout::scalar())?;
    for binding in profile.linear_units() {
        let mut fields = Fields::default();
        fields.exact(binding.metres_per_unit())?;
        units = checked(units.with_child(fields.finish(
            2,
            binding.unit().as_str().len() as u64,
            false,
        )?))?;
    }
    checked(
        record(5, 8)?
            .with_child(references)
            .and_then(|layout| layout.with_child(operations))
            .and_then(|layout| layout.with_child(units))
            .and_then(|layout| layout.with_child(record(7, 7 * 20).ok()?)),
    )
}

pub(super) fn cell_scale(
    bounds: &purrdf_geo_kernel::cells::CellScaleBounds,
) -> Result<OutputLayout> {
    let mut fields = Fields::default();
    for value in [
        bounds.lower(),
        bounds.upper(),
        bounds.nominal_lower(),
        bounds.nominal_upper(),
        bounds.footprint_guard(),
    ] {
        fields.outward(value.exact())?;
    }
    fields.finish(8, 64 + 32, false)
}

pub(super) fn offset(radius: &Rat) -> Result<OutputLayout> {
    let mut fields = Fields::default();
    fields.exact(radius)?;
    fields.finish(4, 128 + 5, false)
}
