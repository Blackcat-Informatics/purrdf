// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original-parameter noding of native oriented region boundaries.
//! Every cut denotes an original-law contact. Rational proof endpoints never
//! become replacement vertices, and changing their width never changes a node.

use super::{ChartLine, CurveIntersection};
use crate::context::WorkProgress;
use crate::numerical::{ExactAdmission, exact_rational, geo_math_error};
use crate::{
    GeoError, MetricContext, MetricWorkObserver, PreparedCurve, PreparedEdge, PreparedGeometry,
    PreparedPolygon, PreparedRegion, Rat,
};
use purrdf_core::SmallVec;
use purrdf_hash::{Domain, hex::Digest32};
use purrdf_xsd::integer::ExactOperation::{
    Linear, RationalAdd, RationalCompare, RationalDivide, RationalMultiply,
};
use purrdf_xsd::math::FixedInterval;
use std::sync::Arc;

const NODE_DOMAIN: Domain = Domain::new(b"purrdf-geo-kernel/native-arrangement-node/v1");
const FRAGMENT_DOMAIN: Domain = Domain::new(b"purrdf-geo-kernel/native-arrangement-fragment/v1");

mod relation;
pub(crate) use relation::relation_matrix;

type Bounds = (Rat, Rat);
type BoundRefs<'a> = [&'a Rat; 2];
type ContactPoint<'a> = [BoundRefs<'a>; 2];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Address {
    ring: usize,
    edge: usize,
    polygon: usize,
    local_ring: usize,
}

#[derive(Clone, Debug)]
enum Definition {
    Exact(Rat),
    Contact {
        pair: [Address; 2],
        occurrence: usize,
        endpoint: usize,
        side: usize,
        reversed: bool,
        isolated: bool,
    },
}

/// An original-curve parameter defined by an exact value or a unique contact.
/// Its bounds are numerical receipts. The node identity excludes those bounds,
/// compiler/backend choices, and resource admission.
#[derive(Clone, Debug)]
pub struct SourceParameter {
    source: Arc<PreparedGeometry>,
    definition: Definition,
    bounds: Bounds,
    node: Digest32,
}

impl SourceParameter {
    /// Stable identity of the physical arrangement node.
    #[must_use]
    pub const fn endpoint_identity(&self) -> Digest32 {
        self.node
    }

    /// Complete inclusive original-parameter enclosure.
    #[must_use]
    pub const fn bounds(&self) -> (&Rat, &Rat) {
        (&self.bounds.0, &self.bounds.1)
    }

    /// An exact value belongs to the original definition. A singleton contact
    /// enclosure remains a contact receipt and never becomes an exact tag.
    #[must_use]
    pub fn exact_value(&self) -> Option<&Rat> {
        match &self.definition {
            Definition::Exact(value) => Some(value),
            Definition::Contact { .. } => None,
        }
    }

    /// Storage owned by this cut receipt, excluding its shared original source.
    /// This admission evidence does not participate in the node identity.
    #[must_use]
    pub fn retained_workspace_bytes(&self) -> u64 {
        (size_of::<Self>() as u64)
            .saturating_add(self.bounds.0.allocated_bytes() as u64)
            .saturating_add(self.bounds.1.allocated_bytes() as u64)
            .saturating_add(match &self.definition {
                Definition::Exact(value) => value.allocated_bytes() as u64,
                Definition::Contact { .. } => 0,
            })
    }

    /// Preadmit both final bound integers before replaying the original contact.
    /// Numerical roots are dyadic at the requested precision. Exact linear
    /// contacts use coordinate differences, determinant products, determinant
    /// sums and a quotient; the shared arithmetic home bounds each width.
    pub(crate) fn refinement_storage_bound(&self, bits: u32) -> Result<u64, GeoError> {
        use purrdf_xsd::integer::ExactArithmeticCost;
        let source_bits = match &self.definition {
            Definition::Exact(value) => {
                return Ok(self.retained_workspace_bytes().saturating_add(
                    crate::numerical::rational_cost(Linear, &[value], 3)
                        .ok_or(GeoError::ArithmeticOverflow("exact cut storage"))?
                        .workspace_bytes,
                ));
            }
            Definition::Contact { pair, .. } => pair
                .iter()
                .map(|address| edge(&self.source, *address).max_original_operand_bits())
                .max()
                .unwrap_or(0),
        };
        let mut width = source_bits.max(u64::from(bits));
        for operation in [RationalAdd, RationalMultiply, RationalAdd, RationalDivide] {
            width = ExactArithmeticCost::for_operation(operation, width, 1)
                .ok_or(GeoError::ArithmeticOverflow("contact cut storage"))?
                .output_bits;
        }
        let bound = ExactArithmeticCost::for_operation(Linear, width, 3)
            .ok_or(GeoError::ArithmeticOverflow("contact cut storage"))?;
        (size_of::<Self>() as u64)
            .checked_add(bound.workspace_bytes)
            .ok_or(GeoError::ArithmeticOverflow("contact cut storage"))
    }

    /// Refine the same source-defined cut, without choosing a quantized vertex.
    ///
    /// # Errors
    /// Refuses incomplete original-law contact ordering or resource admission.
    pub fn refined(&self, bits: u32, context: &mut MetricContext) -> Result<Self, GeoError> {
        self.refine(context, bits, None)
    }

    /// Refine a cut while charging the shared bounded governor seam.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::refined`].
    pub fn refined_metered(
        &self,
        bits: u32,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        self.refine(context, bits, Some(observer))
    }

    fn refine(
        &self,
        context: &mut MetricContext,
        bits: u32,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        context.begin(1)?;
        let mut progress = WorkProgress::new(observer);
        progress.initial()?;
        self.refined_in(bits, context, &mut progress)
    }

    /// Refine the same implicit contact under the active cumulative invocation.
    pub(crate) fn refined_in(
        &self,
        bits: u32,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Self, GeoError> {
        if bits == 0 || bits > context.policy().limits().max_precision_bits {
            return Err(GeoError::PrecisionExhausted {
                bits: context.policy().limits().max_precision_bits,
            });
        }
        context.charge_work(1)?;
        progress.context_poll(context)?;
        // The immutable source was validated before this private-field cut was
        // published. Its binding identity proves every retained original edge;
        // refinement need not repeat an unmetered whole-source metadata walk.
        if !crate::numerical::reference_matches(self.source.reference(), context, progress)? {
            return Err(crate::numerical::missing_operation(
                self.source.reference(),
                context,
                progress,
            )?);
        }
        let bounds = match &self.definition {
            Definition::Exact(value) => {
                ExactAdmission::new(context, progress)
                    .rational(Linear, &[value], 2, || Ok((value.clone(), value.clone())))?
            }
            Definition::Contact {
                pair,
                occurrence,
                endpoint,
                side,
                reversed,
                isolated: _,
            } => with_pair_contacts(
                &self.source,
                *pair,
                bits,
                context,
                progress,
                |contacts, context, progress| {
                    let contact = contacts
                        .get(*occurrence)
                        .ok_or_else(|| GeoError::domain("original contact occurrence changed"))?;
                    let constants = [Rat::zero(), Rat::one()];
                    let values = contact_bounds(contact, &constants);
                    let point = values
                        .get(*endpoint)
                        .ok_or_else(|| GeoError::domain("original overlap endpoint changed"))?;
                    let bounds = copy_bounds(point[*side], context, progress)?;
                    if *reversed {
                        reverse_bounds(&bounds, context, progress)
                    } else {
                        Ok(bounds)
                    }
                },
            )?,
        };
        Ok(Self {
            source: self.source.clone(),
            definition: copy_definition(&self.definition, context, progress)?,
            bounds,
            node: self.node,
        })
    }
}

/// Actual dimension of a selected open fragment on the physical surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectedFragmentStratum {
    /// A selected one-dimensional component with exterior on both sides.
    CurveInterior,
    /// A boundary separating selected and unselected two-dimensional faces.
    ArealBoundary,
}

/// Dimension-specific contact with the complete selected physical closure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectedContactStratum {
    /// Contact with a boundary separating two-dimensional selected faces.
    ArealBoundary,
    /// Contact only with a selected one-dimensional component.
    CurveInterior,
    /// Contact only with a selected isolated point.
    IsolatedPoint,
}

/// A complete selected original subcurve, including lower-dimensional closure.
#[derive(Clone, Debug)]
pub struct NativeBoundaryFragment {
    source: Arc<PreparedGeometry>,
    address: Address,
    parameters: [SourceParameter; 2],
    reversed: bool,
    stratum: SelectedFragmentStratum,
    id: Digest32,
}

impl NativeBoundaryFragment {
    /// Actual selected-set stratum proved by the complete contact graph.
    #[must_use]
    pub const fn stratum(&self) -> SelectedFragmentStratum {
        self.stratum
    }

    /// Borrow the original edge law, including its selected arc branch.
    #[must_use]
    pub fn original_edge(&self) -> &PreparedEdge {
        edge(&self.source, self.address)
    }

    /// Source-order endpoints. Reversal changes orientation, not these inputs.
    #[must_use]
    pub const fn parameters(&self) -> &[SourceParameter; 2] {
        &self.parameters
    }

    /// Whether the selected region interior is on the original right side.
    #[must_use]
    pub const fn reversed(&self) -> bool {
        self.reversed
    }

    /// Stable complete original-region identity.
    #[must_use]
    pub fn source_id(&self) -> Digest32 {
        self.source.id()
    }

    /// Stable original subcurve and orientation identity.
    #[must_use]
    pub const fn id(&self) -> Digest32 {
        self.id
    }
}

/// A selected physical point with no selected positive-dimensional incidence.
/// Its position remains an original-law parameter, including irrational cuts.
#[derive(Clone, Debug)]
pub struct NativeSelectedPoint {
    source: Arc<PreparedGeometry>,
    address: Address,
    parameter: SourceParameter,
}

impl NativeSelectedPoint {
    /// Original source edge defining the retained point.
    #[must_use]
    pub fn original_edge(&self) -> &PreparedEdge {
        edge(&self.source, self.address)
    }

    /// Refinable original-law point parameter.
    #[must_use]
    pub const fn parameter(&self) -> &SourceParameter {
        &self.parameter
    }

    /// Stable physical node identity, independent of proof tightness.
    #[must_use]
    pub const fn node_id(&self) -> Digest32 {
        self.parameter.endpoint_identity()
    }

    /// Owned result storage, excluding the shared complete original source.
    #[must_use]
    pub fn retained_workspace_bytes(&self) -> u64 {
        (size_of::<Self>() as u64)
            .saturating_add(self.parameter.retained_workspace_bytes())
            .saturating_sub(size_of::<SourceParameter>() as u64)
    }
}

/// Fully noded selected physical strata, with original curves retained once.
#[derive(Clone, Debug)]
pub struct NativeBoundaryArrangement {
    source: Arc<PreparedGeometry>,
    fragments: Vec<NativeBoundaryFragment>,
    isolated_points: Vec<NativeSelectedPoint>,
    has_areal_faces: bool,
    retained: u64,
}

impl NativeBoundaryArrangement {
    /// Complete selected physical boundary fragments in source hierarchy order.
    #[must_use]
    pub fn fragments(&self) -> &[NativeBoundaryFragment] {
        &self.fragments
    }

    /// Complete selected isolated zero-dimensional components.
    #[must_use]
    pub fn isolated_points(&self) -> &[NativeSelectedPoint] {
        &self.isolated_points
    }

    /// Whether an actual selected open two-dimensional face was proved.
    #[must_use]
    pub const fn has_areal_faces(&self) -> bool {
        self.has_areal_faces
    }

    /// Original region whose exact Boolean law selected these fragments.
    #[must_use]
    pub fn region(&self) -> &PreparedRegion {
        self.source.region()
    }

    /// Source identity independent of proof tightness and admitted limits.
    #[must_use]
    pub fn source_id(&self) -> Digest32 {
        self.source.id()
    }

    /// Retained original source and result storage, counted once.
    #[must_use]
    pub const fn retained_workspace_bytes(&self) -> u64 {
        self.retained
    }
}

/// Decide contact with the complete selected physical boundary. Original
/// parameters, including every repeated phase visit, are compared with the
/// source-defined cuts; numerical cut tails are never discarded.
pub(crate) fn selected_boundary_contact(
    point: &crate::LonLat,
    boundary: &NativeBoundaryArrangement,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    selected_contact_stratum(point, boundary, context, progress).map(|value| value.is_some())
}

/// Prove the actual selected contact stratum, giving areal incidence priority.
/// Area consumers can discard lower-dimensional contacts without copying the
/// original point/parameter proof or changing complete-set membership.
pub(crate) fn selected_contact_stratum(
    point: &crate::LonLat,
    boundary: &NativeBoundaryArrangement,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<SelectedContactStratum>, GeoError> {
    for stratum in [
        SelectedFragmentStratum::ArealBoundary,
        SelectedFragmentStratum::CurveInterior,
    ] {
        for fragment in boundary.fragments() {
            context.charge_work(1)?;
            progress.context_poll(context)?;
            if fragment.stratum() == stratum
                && source_contact(
                    point,
                    fragment.original_edge(),
                    [&fragment.parameters[0], &fragment.parameters[1]],
                    context,
                    progress,
                )?
            {
                return Ok(Some(match stratum {
                    SelectedFragmentStratum::ArealBoundary => SelectedContactStratum::ArealBoundary,
                    SelectedFragmentStratum::CurveInterior => SelectedContactStratum::CurveInterior,
                }));
            }
        }
    }
    for selected in boundary.isolated_points() {
        context.charge_work(1)?;
        progress.context_poll(context)?;
        if source_contact(
            point,
            selected.original_edge(),
            [selected.parameter(); 2],
            context,
            progress,
        )? {
            return Ok(Some(SelectedContactStratum::IsolatedPoint));
        }
    }
    Ok(None)
}

fn source_contact(
    point: &crate::LonLat,
    edge: &PreparedEdge,
    original_parameters: [&SourceParameter; 2],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    if let Some(constant) = super::constant::constant_point(edge, context, progress)? {
        return ExactAdmission::new(context, progress).rational(
            Linear,
            &[
                constant.longitude(),
                constant.latitude(),
                point.longitude(),
                point.latitude(),
            ],
            4,
            || Ok(constant.same_location(point)),
        );
    }
    let scratch = super::source_workspace_bound(core::iter::once(edge))
        .saturating_add(
            (point.longitude().allocated_bytes() as u64)
                .saturating_add(point.latitude().allocated_bytes() as u64)
                .saturating_mul(32),
        )
        .saturating_add(65_536);
    context.admit_workspace(scratch)?;
    let mut retained = 0u64;
    let result = (|| {
        let maximum = context.policy().limits().max_precision_bits;
        let mut bits = 80.min(maximum);
        let mut refined: [Option<SourceParameter>; 2] = [None, None];
        loop {
            let parameters =
                crate::atlas::point::parameter_enclosures(edge, point, bits, context, progress)?;
            let cuts = [0, 1].map(|index| {
                refined[index]
                    .as_ref()
                    .unwrap_or_else(|| original_parameters[index])
                    .bounds()
            });
            let mut decisive = true;
            for (lower, upper) in &parameters {
                let (inside, outside) = ExactAdmission::new(context, progress).rational(
                    RationalCompare,
                    &[lower, upper, cuts[0].0, cuts[0].1, cuts[1].0, cuts[1].1],
                    4,
                    || {
                        Ok((
                            lower >= cuts[0].1 && upper <= cuts[1].0,
                            upper < cuts[0].0 || lower > cuts[1].1,
                        ))
                    },
                )?;
                if inside {
                    return Ok(true);
                }
                decisive &= outside;
            }
            if decisive {
                return Ok(false);
            }
            if bits == maximum {
                return Err(GeoError::PrecisionExhausted { bits });
            }
            bits = bits.saturating_mul(2).min(maximum);
            for (index, slot) in refined.iter_mut().enumerate() {
                let original = original_parameters[index];
                let reservation = original.refinement_storage_bound(bits)?;
                context.admit_workspace(reservation)?;
                let value = match original.refined_in(bits, context, progress) {
                    Ok(value) => value,
                    Err(error) => {
                        context.release_workspace(reservation)?;
                        return Err(error);
                    }
                };
                let owned = value.retained_workspace_bytes();
                if owned > reservation {
                    drop(value);
                    context.release_workspace(reservation)?;
                    return Err(GeoError::ArithmeticOverflow(
                        "selected cut refinement storage",
                    ));
                }
                context.release_workspace(reservation - owned)?;
                let previous = slot
                    .as_ref()
                    .map_or(0, SourceParameter::retained_workspace_bytes);
                retained = retained
                    .checked_add(owned)
                    .and_then(|bytes| bytes.checked_sub(previous))
                    .ok_or(GeoError::ArithmeticOverflow("selected cut retention"))?;
                let previous_value = slot.replace(value);
                drop(previous_value);
                context.release_workspace(previous)?;
            }
        }
    })();
    context.release_workspace(retained)?;
    context.release_workspace(scratch)?;
    result
}

/// Node every original ring contact and retain the region's selected strata.
///
/// # Errors
/// Refuses unresolved contact equality/order, singular side classification,
/// reference mismatch, and incomplete work/memory/output/precision admission.
/// Both written chart rings and explicitly oriented native rings are admitted.
pub fn native_boundary(
    region: &PreparedRegion,
    context: &mut MetricContext,
) -> Result<NativeBoundaryArrangement, GeoError> {
    arrange(region, context, None)
}

/// Construct the same complete boundary with bounded governor charging.
///
/// # Errors
/// Adds observer refusal to [`native_boundary`].
pub fn native_boundary_metered(
    region: &PreparedRegion,
    context: &mut MetricContext,
    observer: &mut dyn MetricWorkObserver,
) -> Result<NativeBoundaryArrangement, GeoError> {
    arrange(region, context, Some(observer))
}

#[derive(Clone)]
struct Cut {
    bounds: Bounds,
    definition: Definition,
    node: usize,
}

struct Overlap {
    pair: [Address; 2],
    bounds: [[Bounds; 2]; 2],
    reversed: bool,
}

struct Inventory {
    source: Arc<PreparedGeometry>,
    addresses: Vec<Address>,
    cuts: Vec<Vec<Cut>>,
    nodes: Vec<Digest32>,
    parents: Vec<usize>,
    ranks: Vec<u8>,
    canonical: Vec<Digest32>,
    overlaps: Vec<Overlap>,
    retained: u64,
}

fn arrange(
    region: &PreparedRegion,
    context: &mut MetricContext,
    observer: Option<&mut dyn MetricWorkObserver>,
) -> Result<NativeBoundaryArrangement, GeoError> {
    context.begin(1)?;
    let mut progress = WorkProgress::new(observer);
    progress.initial()?;
    native_boundary_in(region, context, &mut progress)
}

/// Borrow the active invocation without resetting its cumulative admission.
pub(crate) fn native_boundary_in(
    region: &PreparedRegion,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<NativeBoundaryArrangement, GeoError> {
    let binding = crate::numerical::reference_identity(context, progress, None)?;
    region.check_binding_admitted(binding, context, progress)?;
    let reference = crate::numerical::reference_clone(context, progress)?;
    let polygons = polygons(region)?;
    let mut count = 0usize;
    for polygon in polygons {
        context.charge_work(1)?;
        progress.context_poll(context)?;
        for ring in polygon.rings() {
            context.charge_work(1)?;
            progress.context_poll(context)?;
            count = count
                .checked_add(ring.edges().len())
                .ok_or(GeoError::ArithmeticOverflow("native edge inventory"))?;
        }
    }
    let pairs = count
        .checked_mul(count.saturating_sub(1))
        .map(|v| v / 2)
        .ok_or(GeoError::ArithmeticOverflow("native pair inventory"))?;
    context.charge_work(count as u64 + pairs as u64 + 1)?;
    progress.context_poll(context)?;
    let scratch = super::source_workspace_bound(
        polygons
            .iter()
            .flat_map(PreparedPolygon::rings)
            .flat_map(PreparedCurve::edges),
    )
    .checked_add(
        (count as u64)
            .checked_mul(2048)
            .ok_or(GeoError::ArithmeticOverflow("native noding storage"))?,
    )
    .ok_or(GeoError::ArithmeticOverflow("native noding storage"))?;
    context.admit_workspace(scratch)?;
    let result = arrange_admitted(region, reference, context, progress);
    context.release_workspace(scratch)?;
    result
}

fn arrange_admitted(
    region: &PreparedRegion,
    reference: crate::GeographicReference,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<NativeBoundaryArrangement, GeoError> {
    let mut temporary = 0;
    let result = (|| {
        let region = relation::selected_region(region, context, progress, &mut temporary)?;
        let policy = context.remaining_child()?.policy();
        let result = {
            let mut observer = progress.nested(
                context.work_items(),
                context
                    .policy()
                    .limits()
                    .max_workspace_bytes
                    .saturating_sub(context.remaining_workspace()),
                context.workspace_peak(),
            );
            PreparedGeometry::from_parts_metered(
                reference,
                Vec::new(),
                Vec::new(),
                region,
                policy,
                &mut observer,
            )
        };
        let source = progress.absorb_nested(context, result)?;
        context.admit_workspace(source.retained_workspace_bytes())?;
        Ok::<_, GeoError>(source)
    })();
    context.release_workspace(temporary)?;
    let source = result?;
    progress.context_poll(context)?;
    let retained = source.retained_workspace_bytes();
    let mut inventory = Inventory {
        source: Arc::new(source),
        addresses: Vec::new(),
        cuts: Vec::new(),
        nodes: Vec::new(),
        parents: Vec::new(),
        ranks: Vec::new(),
        canonical: Vec::new(),
        overlaps: Vec::new(),
        retained,
    };
    let result = (|| {
        initialize(&mut inventory, context, progress)?;
        let mut goal = 32;
        loop {
            match node_pairs(&mut inventory, goal, context, progress) {
                Err(GeoError::PrecisionExhausted { .. })
                    if goal < context.policy().limits().max_precision_bits =>
                {
                    reset_inventory(&mut inventory, retained, context, progress)?;
                    initialize(&mut inventory, context, progress)?;
                    goal = goal
                        .saturating_mul(2)
                        .min(context.policy().limits().max_precision_bits);
                }
                result => {
                    result?;
                    break;
                }
            }
        }
        relation::selected_strata(&inventory, context, progress)
    })();
    let retained = inventory.retained;
    drop(inventory);
    context.release_workspace(retained)?;
    result
}

fn reset_inventory(
    inventory: &mut Inventory,
    base: u64,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    // Drop every old contact, limb and grown container before its allowance is
    // released. The enclosing inventory reservation admits the fresh metadata.
    let old = (
        core::mem::take(&mut inventory.addresses),
        core::mem::take(&mut inventory.cuts),
        core::mem::take(&mut inventory.nodes),
        core::mem::take(&mut inventory.parents),
        core::mem::take(&mut inventory.ranks),
        core::mem::take(&mut inventory.canonical),
        core::mem::take(&mut inventory.overlaps),
    );
    drop(old);
    context.release_workspace(inventory.retained.saturating_sub(base))?;
    inventory.retained = base;
    progress.context_poll(context)
}

fn polygons(region: &PreparedRegion) -> Result<&[PreparedPolygon], GeoError> {
    match region {
        PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons) => {
            Ok(polygons)
        }
        PreparedRegion::Empty | PreparedRegion::Whole => Ok(&[]),
    }
}

fn rings(source: &PreparedGeometry) -> impl Iterator<Item = &PreparedCurve> {
    polygons(source.region())
        .expect("prepared finite region")
        .iter()
        .flat_map(PreparedPolygon::rings)
}

fn edge(source: &PreparedGeometry, address: Address) -> &PreparedEdge {
    &curve(source, address).edges()[address.edge]
}

fn curve(source: &PreparedGeometry, address: Address) -> &PreparedCurve {
    if address.polygon == usize::MAX {
        &source.curves()[address.local_ring]
    } else {
        &polygons(source.region()).expect("prepared finite region")[address.polygon].rings()
            [address.local_ring]
    }
}

fn node_id(source: Digest32, fields: &[u64]) -> Digest32 {
    let fields = fields
        .iter()
        .copied()
        .map(u64::to_be_bytes)
        .collect::<SmallVec<[[u8; 8]; 8]>>();
    crate::profile::hash_fields(
        NODE_DOMAIN,
        core::iter::once(source.as_bytes().as_slice())
            .chain(fields.iter().map(<[u8; 8]>::as_slice)),
    )
}

pub(super) use crate::carrier::reserve_metadata;

fn initialize(
    inventory: &mut Inventory,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    let mut count = 0usize;
    for curve in inventory
        .source
        .curves()
        .iter()
        .chain(rings(&inventory.source))
    {
        context.charge_work(1)?;
        progress.context_poll(context)?;
        count = count
            .checked_add(curve.edges().len())
            .ok_or(GeoError::ArithmeticOverflow("contact edge metadata"))?;
    }
    let maximum_nodes = count
        .checked_mul(2)
        .ok_or(GeoError::ArithmeticOverflow("contact node metadata"))?;
    let storage = count
        .checked_mul(size_of::<Address>() + size_of::<Vec<Cut>>() + 2 * size_of::<Cut>())
        .and_then(|bytes| {
            maximum_nodes
                .checked_mul(2 * size_of::<Digest32>() + size_of::<usize>() + size_of::<u8>())
                .and_then(|extra| bytes.checked_add(extra))
        })
        .and_then(|bytes| u64::try_from(bytes).ok())
        .ok_or(GeoError::ArithmeticOverflow(
            "contact initial metadata storage",
        ))?;
    let retained = inventory
        .retained
        .checked_add(storage)
        .ok_or(GeoError::ArithmeticOverflow(
            "contact initial metadata retention",
        ))?;
    context.admit_workspace(storage)?;
    inventory.retained = retained;
    reserve_metadata(&mut inventory.addresses, count, context, progress)?;
    reserve_metadata(&mut inventory.cuts, count, context, progress)?;
    reserve_metadata(&mut inventory.nodes, maximum_nodes, context, progress)?;
    reserve_metadata(&mut inventory.parents, maximum_nodes, context, progress)?;
    reserve_metadata(&mut inventory.ranks, maximum_nodes, context, progress)?;
    reserve_metadata(&mut inventory.canonical, maximum_nodes, context, progress)?;
    let region_curves = polygons(inventory.source.region())?
        .iter()
        .enumerate()
        .flat_map(|(polygon, value)| {
            value
                .rings()
                .iter()
                .enumerate()
                .map(move |(local_ring, curve)| (polygon, local_ring, curve))
        });
    let curves = region_curves.chain(
        inventory
            .source
            .curves()
            .iter()
            .enumerate()
            .map(|(index, curve)| (usize::MAX, index, curve)),
    );
    for (ring, (polygon, local_ring, curve)) in curves.enumerate() {
        let first_node = inventory.nodes.len();
        // A rank-lost areal image is a union of selected curve/point supports,
        // not a Jordan traversal. Its original edges need independent endpoint
        // nodes until complete original-law contacts identify them.
        let closed = polygon != usize::MAX
            && !polygons(inventory.source.region())?[polygon].closed_support();
        let nodes = if closed {
            curve.edges().len()
        } else {
            curve
                .edges()
                .len()
                .checked_mul(2)
                .ok_or(GeoError::ArithmeticOverflow("open curve nodes"))?
        };
        for index in 0..nodes {
            context.charge_work(1)?;
            progress.context_poll(context)?;
            inventory.nodes.push(node_id(
                inventory.source.id(),
                &[0, ring as u64, index as u64],
            ));
            inventory.parents.push(inventory.parents.len());
            inventory.ranks.push(0);
        }
        for index in 0..curve.edges().len() {
            inventory.addresses.push(Address {
                ring,
                edge: index,
                polygon,
                local_ring,
            });
            let mut cuts = Vec::new();
            reserve_metadata(&mut cuts, 2, context, progress)?;
            cuts.extend([
                Cut {
                    bounds: (Rat::zero(), Rat::zero()),
                    definition: Definition::Exact(Rat::zero()),
                    node: first_node + if closed { index } else { index * 2 },
                },
                Cut {
                    bounds: (Rat::one(), Rat::one()),
                    definition: Definition::Exact(Rat::one()),
                    node: first_node
                        + if closed {
                            (index + 1) % curve.edges().len()
                        } else {
                            index * 2 + 1
                        },
                },
            ]);
            if !closed && relation::closed_endpoints(&curve.edges()[index], context, progress)? {
                unite(
                    &mut inventory.parents,
                    &mut inventory.ranks,
                    cuts[0].node,
                    cuts[1].node,
                );
            }
            inventory.cuts.push(cuts);
        }
    }
    Ok(())
}

fn with_pair_contacts<T>(
    source: &PreparedGeometry,
    pair: [Address; 2],
    goal: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    evaluate: impl FnOnce(
        &[CurveIntersection],
        &mut MetricContext,
        &mut WorkProgress<'_>,
    ) -> Result<T, GeoError>,
) -> Result<T, GeoError> {
    let mut child = context.remaining_child()?;
    let result = (|| {
        child.begin(1)?;
        let mut observer = progress.nested(
            context.work_items(),
            context
                .policy()
                .limits()
                .max_workspace_bytes
                .saturating_sub(context.remaining_workspace()),
            context.workspace_peak(),
        );
        let mut child_progress = WorkProgress::new(Some(&mut observer));
        child_progress.initial()?;
        if pair[0] == pair[1] {
            super::self_intersections_refined_in(
                edge(source, pair[0]),
                goal,
                &mut child,
                &mut child_progress,
            )
        } else {
            super::intersections_refined_in(
                edge(source, pair[0]),
                edge(source, pair[1]),
                goal,
                &mut child,
                &mut child_progress,
            )
        }
    })();
    let mut contacts = progress.absorb_child_result(context, &child, result)?;
    let constants = [Rat::zero(), Rat::one()];
    context.charge_work((contacts.len() as u64).saturating_mul(16))?;
    progress.context_poll(context)?;
    let storage = contacts.iter().fold(
        (contacts.capacity() as u64).saturating_mul(size_of::<CurveIntersection>() as u64),
        |bytes, contact| {
            let bounds = contact_bounds(contact, &constants);
            let bytes = bounds
                .iter()
                .flatten()
                .flatten()
                .fold(bytes, |bytes, value| {
                    bytes.saturating_add(value.allocated_bytes() as u64)
                });
            let point_bytes = |point: &crate::LonLat| {
                (point.longitude().allocated_bytes() as u64)
                    .saturating_add(point.latitude().allocated_bytes() as u64)
            };
            bytes.saturating_add(match contact {
                CurveIntersection::Exact { point, .. }
                | CurveIntersection::Endpoint { point, .. }
                | CurveIntersection::ConstantContact { point, .. } => point_bytes(point),
                CurveIntersection::Overlap { start, end, .. }
                | CurveIntersection::GeodesicOverlap { start, end, .. } => {
                    point_bytes(start).saturating_add(point_bytes(end))
                }
                CurveIntersection::Isolated {
                    longitude_radians: (a, b),
                    latitude_radians: (c, d),
                    ..
                } => [a, b, c, d].into_iter().fold(0u64, |bytes, v| {
                    bytes.saturating_add(v.allocated_bytes() as u64)
                }),
                _ => 0,
            })
        },
    );
    context.admit_workspace(storage)?;
    let result = (|| {
        purrdf_lex::walk::try_sort_unstable_by(&mut contacts, |a, b| {
            let a = contact_bounds(a, &constants);
            let b = contact_bounds(b, &constants);
            let first = bounds_order(a[0][0], b[0][0], context, progress)?;
            if first.is_eq() {
                bounds_order(a[0][1], b[0][1], context, progress)
            } else {
                Ok(first)
            }
        })?;
        evaluate(&contacts, context, progress)
    })();
    drop(contacts);
    context.release_workspace(storage)?;
    result
}

fn contact_bounds<'a>(
    contact: &'a CurveIntersection,
    constants: &'a [Rat; 2],
) -> SmallVec<[ContactPoint<'a>; 2]> {
    let mut points = SmallVec::new();
    match contact {
        CurveIntersection::Exact { left, right, .. }
        | CurveIntersection::Endpoint { left, right, .. }
        | CurveIntersection::SymbolicEndpoint { left, right }
        | CurveIntersection::SymbolicSourceContact { left, right } => {
            points.push([[left, left], [right, right]]);
        }
        CurveIntersection::Isolated { left, right, .. }
        | CurveIntersection::ConstantContact { left, right, .. } => {
            points.push([[&left.0, &left.1], [&right.0, &right.1]]);
        }
        CurveIntersection::Coincident { reversed } => {
            let second = if *reversed {
                [&constants[1], &constants[0]]
            } else {
                [&constants[0], &constants[1]]
            };
            points.push([[&constants[0], &constants[0]], [second[0], second[0]]]);
            points.push([[&constants[1], &constants[1]], [second[1], second[1]]]);
        }
        CurveIntersection::BranchOverlap { left, right }
        | CurveIntersection::Overlap { left, right, .. } => {
            points.push([[&left.0, &left.0], [&right.0, &right.0]]);
            points.push([[&left.1, &left.1], [&right.1, &right.1]]);
        }
        CurveIntersection::GeodesicOverlap { left, right, .. } => {
            for index in 0..2 {
                points.push([
                    [&left[index].0, &left[index].1],
                    [&right[index].0, &right[index].1],
                ]);
            }
        }
    }
    points
}

fn bounds_order(
    a: BoundRefs<'_>,
    b: BoundRefs<'_>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<core::cmp::Ordering, GeoError> {
    ExactAdmission::new(context, progress).rational(
        RationalCompare,
        &[a[0], a[1], b[0], b[1]],
        4,
        || {
            if a[1] < b[0] {
                Ok(core::cmp::Ordering::Less)
            } else if b[1] < a[0] {
                Ok(core::cmp::Ordering::Greater)
            } else if a[0] == a[1] && b[0] == b[1] && a[0] == b[0] {
                Ok(core::cmp::Ordering::Equal)
            } else {
                Err(GeoError::PrecisionExhausted { bits: 0 })
            }
        },
    )
}

fn copy_bounds(
    bounds: BoundRefs<'_>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Bounds, GeoError> {
    ExactAdmission::new(context, progress).rational(Linear, &bounds, 2, || {
        Ok((bounds[0].clone(), bounds[1].clone()))
    })
}

fn copy_definition(
    definition: &Definition,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Definition, GeoError> {
    match definition {
        Definition::Exact(value) => {
            ExactAdmission::new(context, progress)
                .rational(Linear, &[value], 1, || Ok(Definition::Exact(value.clone())))
        }
        Definition::Contact {
            pair,
            occurrence,
            endpoint,
            side,
            reversed,
            isolated,
        } => Ok(Definition::Contact {
            pair: *pair,
            occurrence: *occurrence,
            endpoint: *endpoint,
            side: *side,
            reversed: *reversed,
            isolated: *isolated,
        }),
    }
}

fn node_pairs(
    inventory: &mut Inventory,
    goal: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    let count = inventory.addresses.len();
    let bytes = (count as u64)
        .checked_mul(size_of::<(usize, bool)>() as u64)
        .ok_or(GeoError::ArithmeticOverflow("coincident source aliases"))?;
    context.admit_workspace(bytes)?;
    let mut aliases: Vec<(usize, bool)> = Vec::new();
    let result = (|| {
        reserve_metadata(&mut aliases, count, context, progress)?;
        context.charge_work(count as u64)?;
        progress.context_poll(context)?;
        let curved = inventory.addresses.iter().any(|address| {
            !matches!(
                edge(&inventory.source, *address),
                PreparedEdge::SourceLinear(_)
            )
        });
        for index in 0..count {
            let mut alias = (index, false);
            for representative in 0..if curved { index } else { 0 } {
                context.charge_work(1)?;
                progress.context_poll(context)?;
                if aliases[representative].0 != representative {
                    continue;
                }
                if let Some(reversed) = super::coincident_admitted(
                    edge(&inventory.source, inventory.addresses[representative]),
                    edge(&inventory.source, inventory.addresses[index]),
                    context,
                    progress,
                )? {
                    alias = (representative, reversed);
                    break;
                }
            }
            aliases.push(alias);
        }
        node_pairs_inner(inventory, goal, &aliases, context, progress)
    })();
    drop(aliases);
    context.release_workspace(bytes)?;
    result
}

fn reverse_bounds(
    bounds: &Bounds,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Bounds, GeoError> {
    let one = Rat::one();
    let mut admission = ExactAdmission::new(context, progress);
    Ok((
        exact_rational(
            Some(&mut admission),
            RationalAdd,
            &[&one, &bounds.1],
            || one.sub(&bounds.1),
        )?,
        exact_rational(
            Some(&mut admission),
            RationalAdd,
            &[&one, &bounds.0],
            || one.sub(&bounds.0),
        )?,
    ))
}

fn copy_alias_cuts(
    inventory: &mut Inventory,
    aliases: &[(usize, bool)],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    for (index, &(original, reversed)) in aliases.iter().enumerate() {
        context.charge_work(1)?;
        progress.context_poll(context)?;
        if original == index {
            continue;
        }
        let count = inventory.cuts[original].len();
        let one = Rat::one();
        for position in 0..count {
            let cut = &inventory.cuts[original][position];
            let operation = if reversed { RationalAdd } else { Linear };
            let cost = crate::numerical::rational_cost(
                operation,
                &[&cut.bounds.0, &cut.bounds.1, &one],
                4,
            )
            .ok_or(GeoError::ArithmeticOverflow("coincident cut copies"))?;
            let bytes = cost
                .workspace_bytes
                .checked_add(size_of::<Cut>() as u64)
                .ok_or(GeoError::ArithmeticOverflow("coincident cut storage"))?;
            context.retain_workspace(bytes, &mut inventory.retained)?;
            let bounds = if reversed {
                reverse_bounds(&cut.bounds, context, progress)?
            } else {
                copy_bounds([&cut.bounds.0, &cut.bounds.1], context, progress)?
            };
            let mut definition = copy_definition(&cut.definition, context, progress)?;
            if reversed {
                match &mut definition {
                    Definition::Exact(value) => {
                        *value = exact_rational(
                            Some(&mut ExactAdmission::new(context, progress)),
                            RationalAdd,
                            &[&one, value],
                            || one.sub(value),
                        )?;
                    }
                    Definition::Contact { reversed, .. } => *reversed = !*reversed,
                }
            }
            let copied = Cut {
                bounds,
                definition,
                node: cut.node,
            };
            reserve_metadata(&mut inventory.cuts[index], 1, context, progress)?;
            inventory.cuts[index].push(copied);
        }
    }
    Ok(())
}

/// A previously proved exact node supplies independent existence inside an
/// isolated contact's complete parameter box. That contact's uniqueness proof
/// then identifies the two nodes. Bounds overlap alone is never equality.
fn complete_exact_contacts(
    inventory: &mut Inventory,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    for cuts in &inventory.cuts {
        for first in 0..cuts.len() {
            context.charge_work(1)?;
            progress.context_poll(context)?;
            let Definition::Exact(a) = &cuts[first].definition else {
                continue;
            };
            for second in first + 1..cuts.len() {
                context.charge_work(1)?;
                progress.context_poll(context)?;
                let Definition::Exact(b) = &cuts[second].definition else {
                    continue;
                };
                if ExactAdmission::new(context, progress)
                    .compare(a, b)?
                    .is_eq()
                {
                    context.charge_work(2 * node_depth_bound(inventory.parents.len()) + 2)?;
                    unite(
                        &mut inventory.parents,
                        &mut inventory.ranks,
                        cuts[first].node,
                        cuts[second].node,
                    );
                }
            }
        }
    }
    for row in 0..inventory.cuts.len() {
        for position in 0..inventory.cuts[row].len() {
            context.charge_work(1)?;
            progress.context_poll(context)?;
            let Definition::Contact {
                pair,
                occurrence,
                endpoint,
                side: 0,
                reversed: false,
                isolated: true,
            } = inventory.cuts[row][position].definition
            else {
                continue;
            };
            let node = inventory.cuts[row][position].node;
            let Some(ExactContactWitness { rows, positions }) =
                exact_contact_witness(inventory, pair, node, context, progress)?
            else {
                continue;
            };
            context.charge_work(2 * node_depth_bound(inventory.parents.len()) + 2)?;
            unite(
                &mut inventory.parents,
                &mut inventory.ranks,
                node,
                inventory.cuts[rows[0]][positions[0]].node,
            );
            for output_row in 0..inventory.cuts.len() {
                for output_position in 0..inventory.cuts[output_row].len() {
                    context.charge_work(1)?;
                    progress.context_poll(context)?;
                    let Definition::Contact {
                        pair: original,
                        occurrence: original_occurrence,
                        endpoint: original_endpoint,
                        side,
                        reversed,
                        isolated: true,
                    } = inventory.cuts[output_row][output_position].definition
                    else {
                        continue;
                    };
                    if original != pair
                        || original_occurrence != occurrence
                        || original_endpoint != endpoint
                    {
                        continue;
                    }
                    let Definition::Exact(value) =
                        &inventory.cuts[rows[side]][positions[side]].definition
                    else {
                        unreachable!("independent exact source witness");
                    };
                    let one = Rat::one();
                    let mut admission = ExactAdmission::new(context, progress);
                    let exact = if reversed {
                        admission.rational_owner(
                            RationalAdd,
                            &[&one, value],
                            &mut inventory.retained,
                            || one.sub(value),
                        )?
                    } else {
                        admission.rational_owner(
                            Linear,
                            &[value],
                            &mut inventory.retained,
                            || value.clone(),
                        )?
                    };
                    let lower = admission.rational_owner(
                        Linear,
                        &[&exact],
                        &mut inventory.retained,
                        || exact.clone(),
                    )?;
                    let upper = admission.rational_owner(
                        Linear,
                        &[&exact],
                        &mut inventory.retained,
                        || exact.clone(),
                    )?;
                    let cut = &mut inventory.cuts[output_row][output_position];
                    cut.bounds = (lower, upper);
                    cut.definition = Definition::Exact(exact);
                }
            }
        }
    }
    Ok(())
}

struct ExactContactWitness {
    rows: [usize; 2],
    positions: [usize; 2],
}

fn bounds_contain_exact(
    value: &Rat,
    bounds: &Bounds,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    let mut admission = ExactAdmission::new(context, progress);
    Ok(!admission.compare(value, &bounds.0)?.is_lt()
        && !admission.compare(value, &bounds.1)?.is_gt())
}

fn exact_contact_witness(
    inventory: &Inventory,
    pair: [Address; 2],
    node: usize,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<ExactContactWitness>, GeoError> {
    let scan = (inventory.addresses.len() as u64)
        .checked_mul(2)
        .ok_or(GeoError::ArithmeticOverflow("isolated contact source walk"))?;
    context.charge_work(scan)?;
    progress.context_poll(context)?;
    let rows = pair.map(|address| {
        inventory
            .addresses
            .iter()
            .position(|value| *value == address)
            .expect("original contact address")
    });
    let cuts = rows.map(|row| &inventory.cuts[row]);
    let mut bounds = [None, None];
    for side in 0..2 {
        for cut in cuts[side] {
            context.charge_work(1)?;
            progress.context_poll(context)?;
            if cut.node == node
                && matches!(cut.definition, Definition::Contact { side: original, reversed: false, .. } if original == side)
            {
                bounds[side] = Some(&cut.bounds);
                break;
            }
        }
    }
    let [Some(a), Some(b)] = bounds else {
        return Ok(None);
    };
    for (first, cut) in cuts[0].iter().enumerate() {
        context.charge_work(node_depth_bound(inventory.parents.len()) + 2)?;
        progress.context_poll(context)?;
        let Definition::Exact(value) = &cut.definition else {
            continue;
        };
        if !bounds_contain_exact(value, a, context, progress)? {
            continue;
        }
        let original = root(&inventory.parents, cut.node);
        for (second, other) in cuts[1].iter().enumerate() {
            context.charge_work(node_depth_bound(inventory.parents.len()) + 2)?;
            progress.context_poll(context)?;
            // Root walks are bounded by union by rank, and are charged before
            // the candidate's original exact parameter is inspected.
            // Every shared node already has an original existence witness.
            let Definition::Exact(value) = &other.definition else {
                continue;
            };
            if original == root(&inventory.parents, other.node)
                && bounds_contain_exact(value, b, context, progress)?
            {
                return Ok(Some(ExactContactWitness {
                    rows,
                    positions: [first, second],
                }));
            }
        }
    }
    Ok(None)
}

fn node_pairs_inner(
    inventory: &mut Inventory,
    goal: u32,
    aliases: &[(usize, bool)],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    for first in 0..inventory.addresses.len() {
        for second in first..inventory.addresses.len() {
            if aliases[first].0 != first
                || (aliases[second].0 != second && aliases[second].0 != first)
            {
                continue;
            }
            let pair = [inventory.addresses[first], inventory.addresses[second]];
            if first == second
                && (!matches!(
                    edge(&inventory.source, pair[0]),
                    PreparedEdge::AzimuthLength(_)
                ) || relation::is_constant(
                    edge(&inventory.source, pair[0]),
                    context,
                    progress,
                )?)
            {
                continue;
            }
            if pair[0].ring == pair[1].ring
                && pair[0].polygon != usize::MAX
                && polygons(inventory.source.region())?[pair[0].polygon]
                    .oriented_interior()
                    .is_some()
            {
                continue;
            }
            let source = inventory.source.clone();
            with_pair_contacts(
                &source,
                pair,
                goal,
                context,
                progress,
                |contacts, context, progress| {
                    for (occurrence, contact) in contacts.iter().enumerate() {
                        if matches!(contact, CurveIntersection::ConstantContact { .. })
                            && pair.iter().all(|address| {
                                address.polygon != usize::MAX
                                    && polygons(source.region()).expect("prepared polygon source")
                                        [address.polygon]
                                        .oriented_interior()
                                        .is_some()
                            })
                        {
                            return Err(GeoError::domain("constant edge in validated Jordan ring"));
                        }
                        let constants = [Rat::zero(), Rat::one()];
                        let mut points = contact_bounds(contact, &constants);
                        if matches!(contact, CurveIntersection::ConstantContact { .. }) {
                            for point in &mut points {
                                for side in 0..2 {
                                    if relation::is_constant(
                                        edge(&source, pair[side]),
                                        context,
                                        progress,
                                    )? {
                                        point[side] = [&constants[0], &constants[0]];
                                    }
                                }
                            }
                        }
                        let mut original_endpoints = [0, 1];
                        if points.len() == 2
                            && bounds_order(points[0][0], points[1][0], context, progress)?.is_gt()
                        {
                            points.swap(0, 1);
                            original_endpoints.swap(0, 1);
                        }
                        let bytes = points
                            .iter()
                            .flatten()
                            .fold(1024u64, |bytes, [a, b]| {
                                bytes
                                    .saturating_add(a.allocated_bytes() as u64)
                                    .saturating_add(b.allocated_bytes() as u64)
                            })
                            .saturating_mul(4);
                        context.admit_workspace(bytes)?;
                        inventory.retained = inventory
                            .retained
                            .checked_add(bytes)
                            .ok_or(GeoError::ArithmeticOverflow("native contact storage"))?;
                        reserve_metadata(&mut inventory.nodes, points.len(), context, progress)?;
                        reserve_metadata(&mut inventory.parents, points.len(), context, progress)?;
                        reserve_metadata(&mut inventory.ranks, points.len(), context, progress)?;
                        reserve_metadata(
                            &mut inventory.canonical,
                            points.len(),
                            context,
                            progress,
                        )?;
                        for index in [first, second] {
                            reserve_metadata(
                                &mut inventory.cuts[index],
                                points.len(),
                                context,
                                progress,
                            )?;
                        }
                        if points.len() == 2 {
                            reserve_metadata(&mut inventory.overlaps, 1, context, progress)?;
                        }
                        for (endpoint, point) in points.iter().enumerate() {
                            let node = inventory.nodes.len();
                            inventory.nodes.push(node_id(
                                inventory.source.id(),
                                &[
                                    1,
                                    first as u64,
                                    second as u64,
                                    occurrence as u64,
                                    endpoint as u64,
                                ],
                            ));
                            inventory.parents.push(node);
                            inventory.ranks.push(0);
                            for (side, index) in [first, second].into_iter().enumerate() {
                                let bounds = copy_bounds(point[side], context, progress)?;
                                let exact = ExactAdmission::new(context, progress).rational(
                                    RationalCompare,
                                    &[&bounds.0, &bounds.1],
                                    1,
                                    || Ok(bounds.0 == bounds.1),
                                )?;
                                let definition = if exact {
                                    ExactAdmission::new(context, progress).rational(
                                        Linear,
                                        &[&bounds.0],
                                        1,
                                        || Ok(Definition::Exact(bounds.0.clone())),
                                    )?
                                } else {
                                    Definition::Contact {
                                        pair,
                                        occurrence,
                                        endpoint: original_endpoints[endpoint],
                                        side,
                                        reversed: false,
                                        isolated: matches!(
                                            contact,
                                            CurveIntersection::Isolated { .. }
                                        ),
                                    }
                                };
                                inventory.cuts[index].push(Cut {
                                    bounds,
                                    definition,
                                    node,
                                });
                            }
                        }
                        if points.len() == 2 {
                            let reversed =
                                bounds_order(points[0][1], points[1][1], context, progress)?
                                    .is_gt();
                            inventory.overlaps.push(Overlap {
                                pair,
                                bounds: [
                                    [
                                        copy_bounds(points[0][0], context, progress)?,
                                        copy_bounds(points[1][0], context, progress)?,
                                    ],
                                    [
                                        copy_bounds(points[0][1], context, progress)?,
                                        copy_bounds(points[1][1], context, progress)?,
                                    ],
                                ],
                                reversed,
                            });
                        }
                    }
                    Ok(())
                },
            )?;
        }
    }
    copy_alias_cuts(inventory, aliases, context, progress)?;
    context.charge_work(inventory.addresses.len() as u64)?;
    progress.context_poll(context)?;
    if inventory.addresses.iter().any(|address| {
        !matches!(
            edge(&inventory.source, *address),
            PreparedEdge::SourceLinear(_)
        )
    }) {
        complete_exact_contacts(inventory, context, progress)?;
    }
    for cuts in &mut inventory.cuts {
        purrdf_lex::walk::try_sort_unstable_by(cuts, |a, b| {
            bounds_order(
                [&a.bounds.0, &a.bounds.1],
                [&b.bounds.0, &b.bounds.1],
                context,
                progress,
            )
        })?;
        let mut retained = usize::from(!cuts.is_empty());
        for next in 1..cuts.len() {
            if bounds_order(
                [&cuts[retained - 1].bounds.0, &cuts[retained - 1].bounds.1],
                [&cuts[next].bounds.0, &cuts[next].bounds.1],
                context,
                progress,
            )?
            .is_eq()
            {
                context.charge_work(2 * node_depth_bound(inventory.parents.len()) + 2)?;
                progress.context_poll(context)?;
                unite(
                    &mut inventory.parents,
                    &mut inventory.ranks,
                    cuts[retained - 1].node,
                    cuts[next].node,
                );
            } else {
                cuts.swap(retained, next);
                retained += 1;
            }
        }
        cuts.truncate(retained);
    }
    context.charge_work(
        (inventory.nodes.len() as u64).saturating_mul(node_depth_bound(inventory.nodes.len()) + 1),
    )?;
    progress.context_poll(context)?;
    inventory.canonical.clone_from(&inventory.nodes);
    for (index, id) in inventory.nodes.iter().copied().enumerate() {
        let representative = root(&inventory.parents, index);
        inventory.canonical[representative] = inventory.canonical[representative].min(id);
    }
    Ok(())
}

fn node_depth_bound(nodes: usize) -> u64 {
    u64::from(nodes.bit_width())
}

fn root(parents: &[usize], mut node: usize) -> usize {
    while parents[node] != node {
        node = parents[node];
    }
    node
}
fn unite(parents: &mut [usize], ranks: &mut [u8], a: usize, b: usize) {
    let mut a = root(parents, a);
    let mut b = root(parents, b);
    if a != b {
        if ranks[a] < ranks[b] {
            core::mem::swap(&mut a, &mut b);
        }
        parents[b] = a;
        if ranks[a] == ranks[b] {
            ranks[a] += 1;
        }
    }
}

fn physical_node(inventory: &Inventory, node: usize) -> Digest32 {
    inventory.canonical[root(&inventory.parents, node)]
}

fn midpoint(
    first: &Cut,
    second: &Cut,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Rat, GeoError> {
    let half = Rat::one().div(&Rat::from_i64(2)).expect("half");
    let mut admission = ExactAdmission::new(context, progress);
    let sum = exact_rational(
        Some(&mut admission),
        RationalAdd,
        &[&first.bounds.1, &second.bounds.0],
        || first.bounds.1.add(&second.bounds.0),
    )?;
    exact_rational(
        Some(&mut admission),
        RationalMultiply,
        &[&sum, &half],
        || sum.mul(&half),
    )
}

fn exact_fragment_point(
    edge: &PreparedEdge,
    parameter: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<crate::LonLat>, GeoError> {
    let endpoint =
        ExactAdmission::new(context, progress).rational(Linear, &[parameter], 2, || {
            Ok(if parameter.is_zero() {
                edge.start().map(crate::PreparedCoordinate::point)
            } else if parameter == &Rat::one() {
                match edge {
                    PreparedEdge::SourceLinear(line) => Some(line.end().point()),
                    PreparedEdge::ShortestGeodesic(arc) => Some(arc.end().point()),
                    PreparedEdge::AzimuthLength(arc) if arc.length().exact().is_zero() => {
                        Some(arc.start().point())
                    }
                    PreparedEdge::AzimuthLength(_) | PreparedEdge::Transformed(_) => None,
                }
            } else {
                None
            })
        })?;
    if let Some(point) = endpoint {
        return ExactAdmission::new(context, progress).rational(
            Linear,
            &[point.longitude(), point.latitude()],
            2,
            || Ok(Some(point.clone())),
        );
    }
    if let PreparedEdge::ShortestGeodesic(arc) = edge {
        return super::axis_overlap::equator_point(arc, parameter, context, progress);
    }
    let PreparedEdge::SourceLinear(line) = edge else {
        return Ok(None);
    };
    let mut admission = ExactAdmission::new(context, progress);
    let start = line.start().point();
    let end = line.end().point();
    let longitude = crate::SourceLinearEdge::interpolate_ordinate_admitted(
        start.longitude(),
        end.longitude(),
        parameter,
        &mut admission,
    )?;
    let latitude = crate::SourceLinearEdge::interpolate_ordinate_admitted(
        start.latitude(),
        end.latitude(),
        parameter,
        &mut admission,
    )?;
    let cost = crate::numerical::rational_cost(RationalCompare, &[&longitude, &latitude], 4)
        .ok_or(GeoError::ArithmeticOverflow(
            "original representative range proof",
        ))?;
    progress
        .exact(context, cost, || crate::LonLat::new(longitude, latitude))
        .map(Some)
}

fn fragment_point(
    edge: &PreparedEdge,
    parameter: &Rat,
    bits: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<[FixedInterval; 2], GeoError> {
    fragment_enclosure(edge, [parameter, parameter], bits, context, progress)
}

fn fragment_enclosure(
    edge: &PreparedEdge,
    bounds: [&Rat; 2],
    bits: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<[FixedInterval; 2], GeoError> {
    let policy = context.policy();
    super::with_line_refs_with_precision(
        &[edge],
        bits,
        true,
        context,
        progress,
        |lines, math, progress| {
            (|| {
                let parameter = crate::numerical::fixed_from_bounds(bounds[0], bounds[1], math)?;
                let (image, _) = ChartLine::image(
                    &lines[0],
                    &parameter,
                    policy.limits().max_iterations,
                    math,
                    progress,
                )?;
                let degrees =
                    FixedInterval::from_i64(180, math)?.div(&FixedInterval::pi(math)?, math)?;
                Ok([image[0].mul(&degrees, math)?, image[1].mul(&degrees, math)?])
            })()
            .map_err(|error| geo_math_error(&error, policy))
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Coord, ExecutionLimits, ExecutionPolicy, GeographicReference, OrientedInterior};

    fn worker(work: u64) -> MetricContext {
        let mut limits = ExecutionLimits::GEOMETRY;
        limits.max_work_items = work;
        MetricContext::new(
            GeographicReference::wgs84(),
            ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap()
    }

    fn rectangle(bounds: [i64; 4], context: &mut MetricContext) -> PreparedPolygon {
        let [west, east, south, north] = bounds;
        let coordinates = [
            (west, south),
            (east, south),
            (east, north),
            (west, north),
            (west, south),
        ]
        .map(|(x, y)| Coord::xy(Rat::from_i64(x), Rat::from_i64(y)));
        let curve = PreparedCurve::from_source(&coordinates, context.reference()).unwrap();
        PreparedPolygon::from_curves(vec![curve], OrientedInterior::Left, context).unwrap()
    }

    fn source_union(context: &mut MetricContext) -> PreparedRegion {
        PreparedRegion::polygons(vec![
            rectangle([0, 4, 0, 4], context),
            rectangle([2, 6, -1, 3], context),
        ])
    }

    fn geodesic_rectangle(bounds: [i64; 4], context: &mut MetricContext) -> PreparedPolygon {
        let [west, east, south, north] = bounds;
        let coordinates = [
            (west, south),
            (east, south),
            (east, north),
            (west, north),
            (west, south),
        ];
        let edges = coordinates
            .windows(2)
            .map(|pair| super::super::tests::shortest(pair[0], pair[1], context))
            .collect();
        PreparedPolygon::from_curves(
            vec![PreparedCurve::new(edges)],
            OrientedInterior::Left,
            context,
        )
        .unwrap()
    }

    fn degrees_two(arrangement: &NativeBoundaryArrangement) {
        let mut nodes = arrangement
            .fragments()
            .iter()
            .flat_map(|fragment| {
                fragment
                    .parameters()
                    .iter()
                    .map(SourceParameter::endpoint_identity)
            })
            .collect::<Vec<_>>();
        nodes.sort_unstable();
        for node in &nodes {
            assert_eq!(
                nodes.iter().filter(|other| *other == node).count(),
                2,
                "one complete physical boundary cycle"
            );
        }
    }

    #[test]
    fn exact_native_crossings_select_only_the_complete_union_boundary() {
        let mut context = worker(8_000_000);
        let region = source_union(&mut context);
        let boundary =
            native_boundary(&region, &mut context).expect("complete native crossing arrangement");
        assert_eq!(boundary.fragments().len(), 8);
        degrees_two(&boundary);
        let complement = native_boundary(&region.clone().complement(), &mut context).unwrap();
        assert_eq!(complement.fragments().len(), boundary.fragments().len());
        for (first, second) in boundary.fragments().iter().zip(complement.fragments()) {
            assert_eq!(first.original_edge(), second.original_edge());
            assert_ne!(first.reversed(), second.reversed());
            for (first, second) in first.parameters().iter().zip(second.parameters()) {
                assert_eq!(first.bounds(), second.bounds());
            }
        }
        let mut raised = worker(16_000_000);
        let raised = native_boundary(&region, &mut raised).unwrap();
        assert_eq!(boundary.source_id(), raised.source_id());
        assert_eq!(
            boundary
                .fragments()
                .iter()
                .map(NativeBoundaryFragment::id)
                .collect::<Vec<_>>(),
            raised
                .fragments()
                .iter()
                .map(NativeBoundaryFragment::id)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn coincident_native_fragments_toggle_every_incident_ring_once() {
        let mut context = worker(8_000_000);
        let first = rectangle([0, 4, 0, 4], &mut context);
        let region =
            PreparedRegion::polygons(vec![first.clone(), rectangle([2, 6, 0, 4], &mut context)]);
        let boundary =
            native_boundary(&region, &mut context).expect("complete overlap arrangement");
        assert_eq!(boundary.fragments().len(), 8);
        degrees_two(&boundary);
        let duplicate = native_boundary(
            &PreparedRegion::polygons(vec![first.clone(), first]),
            &mut context,
        )
        .expect("identical source rings coalesce");
        assert_eq!(duplicate.fragments().len(), 4);
        degrees_two(&duplicate);
    }

    #[test]
    fn southern_pole_aliases_node_the_complete_original_union() {
        let mut context = MetricContext::wgs84().unwrap();
        let mut polygons = Vec::new();
        for [west, east] in [[0, 60], [30, 90]] {
            let coordinates = [(east, -90), (east, 0), (west, 0), (west, -90)]
                .map(|(x, y)| Coord::xy(Rat::from_i64(x), Rat::from_i64(y)));
            let ring = PreparedCurve::from_source(&coordinates, context.reference()).unwrap();
            polygons.push(
                PreparedPolygon::from_curves(vec![ring], OrientedInterior::Left, &mut context)
                    .unwrap(),
            );
        }
        let region = PreparedRegion::polygons(polygons);
        let boundary = native_boundary(&region, &mut context)
            .expect("complete physical pole contact arrangement under ordinary admission");
        assert_eq!(boundary.fragments().len(), 5);
        degrees_two(&boundary);
        let complement = native_boundary(&region.complement(), &mut context).unwrap();
        assert_eq!(boundary.fragments().len(), complement.fragments().len());
        for (a, b) in boundary.fragments().iter().zip(complement.fragments()) {
            assert_eq!(a.original_edge(), b.original_edge());
            assert_ne!(a.reversed(), b.reversed());
            assert_eq!(a.parameters()[0].bounds(), b.parameters()[0].bounds());
            assert_eq!(a.parameters()[1].bounds(), b.parameters()[1].bounds());
        }
    }

    #[test]
    fn curved_crossings_retain_original_branches_and_refinable_implicit_nodes() {
        let mut context = worker(80_000_000);
        let region = PreparedRegion::polygons(vec![
            geodesic_rectangle([-4, 4, -4, 4], &mut context),
            geodesic_rectangle([0, 8, 0, 8], &mut context),
        ]);
        let boundary = native_boundary(&region, &mut context).unwrap();
        assert_eq!(boundary.fragments().len(), 8);
        degrees_two(&boundary);
        let mut refined_contacts = 0;
        let quantum = Rat::new(crate::Int::one(), crate::Int::one().shl(64)).unwrap();
        for fragment in boundary.fragments() {
            assert!(matches!(
                fragment.original_edge(),
                PreparedEdge::ShortestGeodesic(_)
            ));
            for parameter in fragment.parameters() {
                if !matches!(parameter.definition, Definition::Contact { .. }) {
                    continue;
                }
                let refined = parameter.refined(64, &mut context).unwrap();
                assert_eq!(refined.endpoint_identity(), parameter.endpoint_identity());
                assert!(refined.bounds().0 <= parameter.bounds().1);
                assert!(parameter.bounds().0 <= refined.bounds().1);
                assert!(refined.bounds().1.sub(refined.bounds().0) <= quantum);
                refined_contacts += 1;
            }
        }
        assert_eq!(refined_contacts, 4);
    }

    fn closed_rectangles(bounds: [[i64; 4]; 2], context: &mut MetricContext) -> PreparedRegion {
        let curves = bounds
            .into_iter()
            .map(|bounds| rectangle(bounds, context).rings()[0].clone())
            .collect();
        PreparedRegion::polygons(vec![
            PreparedPolygon::from_curves(curves, OrientedInterior::Left, context).unwrap(),
        ])
    }

    #[test]
    fn selected_curve_support_joins_ordinary_one_dimensional_length_totals() {
        let mut context = worker(100_000_000);
        // The closed support of two touching rectangle rings is the shared
        // meridian wall, lon 1 between lat 0 and lat 1: a curve, not an area.
        let wall = closed_rectangles([[0, 1, 0, 1], [1, 2, 0, 1]], &mut context);
        let equator = PreparedCurve::from_source(
            &[
                Coord::xy(Rat::from_i64(3), Rat::zero()),
                Coord::xy(Rat::from_i64(4), Rat::zero()),
            ],
            context.reference(),
        )
        .unwrap();
        let mixed = PreparedGeometry::from_parts(
            context.reference().clone(),
            Vec::new(),
            vec![equator.clone()],
            wall,
            context.policy(),
        )
        .unwrap();
        let meridian = PreparedCurve::from_source(
            &[
                Coord::xy(Rat::one(), Rat::zero()),
                Coord::xy(Rat::one(), Rat::one()),
            ],
            context.reference(),
        )
        .unwrap();
        let standalone = PreparedGeometry::from_parts(
            context.reference().clone(),
            Vec::new(),
            vec![meridian, equator],
            PreparedRegion::Empty,
            context.policy(),
        )
        .unwrap();
        let mixed = crate::ellipsoidal::length(&mixed, &mut context).unwrap();
        let standalone = crate::ellipsoidal::length(&standalone, &mut context).unwrap();
        assert_eq!(
            standalone.exact(),
            &Rat::parse_decimal("221893.879351").unwrap()
        );
        assert_eq!(mixed.exact(), standalone.exact());
    }

    #[test]
    fn selected_closure_retains_curve_and_isolated_point_strata() {
        let mut context = worker(2_000_000);
        let segment = closed_rectangles([[0, 1, 0, 1], [1, 2, 0, 1]], &mut context);
        let selected = native_boundary(&segment, &mut context).unwrap();
        assert!(!selected.has_areal_faces());
        assert_eq!(selected.fragments().len(), 1);
        assert_eq!(
            selected.fragments()[0].stratum(),
            SelectedFragmentStratum::CurveInterior
        );
        assert!(selected.isolated_points().is_empty());
        let isolated = closed_rectangles([[0, 1, 0, 1], [1, 2, 1, 2]], &mut context);
        let selected = native_boundary(&isolated, &mut context).unwrap();
        assert!(!selected.has_areal_faces());
        assert!(selected.fragments().is_empty());
        assert_eq!(selected.isolated_points().len(), 1);
        let point = &selected.isolated_points()[0];
        assert_eq!(point.node_id(), point.parameter().endpoint_identity());
        assert!(point.retained_workspace_bytes() >= size_of::<NativeSelectedPoint>() as u64);
        context.begin(1).unwrap();
        let mut progress = WorkProgress::new(None);
        progress.initial().unwrap();
        assert!(
            selected_boundary_contact(
                &crate::LonLat::new(Rat::one(), Rat::one()).unwrap(),
                &selected,
                &mut context,
                &mut progress
            )
            .unwrap()
        );
        let complement = native_boundary(&isolated.complement(), &mut context).unwrap();
        assert!(complement.has_areal_faces());
        assert!(complement.fragments().is_empty());
        assert!(complement.isolated_points().is_empty());
    }

    #[test]
    fn exact_contact_completion_preserves_distinct_and_nonisolated_receipts() {
        let mut context = worker(2_000_000);
        let half = Rat::parse_decimal("0.5").unwrap();
        let coordinate = |x: Rat, y| Coord::xy(x, Rat::from_i64(y));
        let endpoints = [
            [coordinate(Rat::from_i64(-1), 0), coordinate(Rat::one(), 0)],
            [coordinate(Rat::zero(), -1), coordinate(Rat::zero(), 1)],
            [coordinate(half.clone(), -1), coordinate(half.clone(), 1)],
            [coordinate(Rat::zero(), 0), coordinate(Rat::one(), 0)],
        ];
        let curves = endpoints
            .iter()
            .map(|endpoints| PreparedCurve::from_source(endpoints, context.reference()).unwrap())
            .collect();
        let source = PreparedGeometry::from_parts(
            context.reference().clone(),
            Vec::new(),
            curves,
            PreparedRegion::Empty,
            context.policy(),
        )
        .unwrap();
        let mut inventory = Inventory {
            source: Arc::new(source),
            addresses: Vec::new(),
            cuts: Vec::new(),
            nodes: Vec::new(),
            parents: Vec::new(),
            ranks: Vec::new(),
            canonical: Vec::new(),
            overlaps: Vec::new(),
            retained: 0,
        };
        context.begin(1).unwrap();
        let mut progress = WorkProgress::new(None);
        initialize(&mut inventory, &mut context, &mut progress).unwrap();
        let exact_node = inventory.nodes.len();
        for index in 0..4 {
            inventory
                .nodes
                .push(node_id(inventory.source.id(), &[index as u64]));
            inventory.parents.push(inventory.parents.len());
            inventory.ranks.push(0);
        }
        // The horizontal/vertical origin supplies independent exact existence.
        // The fourth source also begins at that same original physical point.
        for row in [0, 1] {
            inventory.cuts[row].push(Cut {
                bounds: (half.clone(), half.clone()),
                definition: Definition::Exact(half.clone()),
                node: exact_node,
            });
        }
        unite(
            &mut inventory.parents,
            &mut inventory.ranks,
            exact_node,
            inventory.cuts[3][0].node,
        );
        for (other, first_bounds, second_bounds, isolated) in [
            (1, ["0.49", "0.51"], ["0.49", "0.51"], true),
            (2, ["0.74", "0.76"], ["0.49", "0.51"], true),
            (3, ["0.49", "0.51"], ["0", "0.01"], false),
        ] {
            let pair = [inventory.addresses[0], inventory.addresses[other]];
            for (side, (row, bounds)) in [(0, first_bounds), (other, second_bounds)]
                .into_iter()
                .enumerate()
            {
                let bounds = bounds
                    .map(|value| Rat::parse_decimal(value).unwrap())
                    .into();
                inventory.cuts[row].push(Cut {
                    bounds,
                    definition: Definition::Contact {
                        pair,
                        occurrence: 0,
                        endpoint: 0,
                        side,
                        reversed: false,
                        isolated,
                    },
                    node: exact_node + other,
                });
            }
        }
        complete_exact_contacts(&mut inventory, &mut context, &mut progress).unwrap();
        let cuts = &inventory.cuts[0];
        assert!(matches!(&cuts[3].definition, Definition::Exact(value) if value == &half));
        assert_eq!(
            root(&inventory.parents, exact_node + 1),
            root(&inventory.parents, exact_node)
        );
        assert!(matches!(
            cuts[4].definition,
            Definition::Contact { isolated: true, .. }
        ));
        assert_ne!(
            root(&inventory.parents, exact_node + 2),
            root(&inventory.parents, exact_node)
        );
        assert!(matches!(
            cuts[5].definition,
            Definition::Contact {
                isolated: false,
                ..
            }
        ));
        assert_ne!(
            root(&inventory.parents, exact_node + 3),
            root(&inventory.parents, exact_node)
        );
    }

    #[test]
    fn native_arrangement_refuses_before_returning_partial_output() {
        let mut context = worker(8_000_000);
        let region = source_union(&mut context);
        let mut limits = ExecutionLimits::GEOMETRY;
        limits.max_work_items = 8_000_000;
        limits.max_output_elements = 7;
        let mut limited = MetricContext::new(
            GeographicReference::wgs84(),
            ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            native_boundary(&region, &mut limited),
            Err(GeoError::OutputExhausted { limit: 7 })
        ));
        struct Cancel;
        impl MetricWorkObserver for Cancel {
            fn charge_chunk(&mut self, _work: u64, _workspace: u64) -> Result<(), GeoError> {
                Err(GeoError::Cancelled)
            }
        }
        assert_eq!(
            native_boundary_metered(&region, &mut context, &mut Cancel).unwrap_err(),
            GeoError::Cancelled
        );
    }
}
