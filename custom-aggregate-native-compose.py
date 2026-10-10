from pathlib import Path
import difflib

root = Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
stage = root / '.stage/sparql-eval-complete-bounded-workspace'
source = root / 'crates/sparql-eval/src/modifier.rs'
base = source.read_text()
text = base

def replace(old, new, count=1):
    global text
    assert text.count(old) == count, (old[:100], text.count(old), count)
    text = text.replace(old, new)

replace('use owners::{DedupState, order_permutation};', 'use owners::DedupState;\npub(crate) use owners::order_permutation;')
replace('use crate::DetHashSet;\n', '')
replace('ground_term_to_workspace_value, literal_to_value, named_node_to_value', 'ground_term_to_workspace_value, named_node_to_value')
if "pub(crate) struct OwnedSortKey<'a> {\n    key:" in text:
    replace("pub(crate) struct OwnedSortKey<'a> {\n    key:", "pub(crate) struct OwnedSortKey<'a> {\n    pub(crate) key:")
replace('fn numeric_fold_binary(', 'pub(crate) fn numeric_fold_binary(')
replace('purrdf_xsd::numeric::numeric_binary_admitted(a, b, op, division, &mut |layout| {', 'purrdf_xsd::ops::value_binary_admitted(a, b, op, division, &mut |layout| {')
replace('    Custom(Box<ContextualCustomFold>),', '''    Custom {
        value: Box<ContextualCustomFold>,
        // The actual box dies before its external control grant.
        allocation: crate::WorkspaceAllocation,
    },''')
replace('''enum AggregateFinished {
    Resident(TermValue),
    Admitted(crate::WorkspaceTerm),
}

''', '')
replace('''struct ContextualCustomFold {
    accumulator: Box<dyn crate::agg_fn::AggregateAccumulator>,
    aggregate: Arc<dyn crate::agg_fn::CustomAggregate>,
    iri: String,
    scalarvals: Vec<(String, TermValue)>,''', '''struct ContextualCustomFold {
    accumulator: crate::agg_fn::WorkspaceAccumulator,
    aggregate: Arc<dyn crate::agg_fn::CustomAggregate>,
    iri: purrdf_lex::allocation::SharedText,
    scalarvals: AggregateScalarvals,''')
start = text.index('            AggregateFunction::Custom(iri) => {', text.index('impl ContextualFold'))
end = text.index('            AggregateFunction::Fold =>', start)
text = text[:start] + '''            AggregateFunction::Custom(iri) => {
                let iri = iri.as_str();
                let custom = ctx.aggregates.resolve(iri).cloned().ok_or_else(|| {
                    missing_custom_aggregate(iri, &ctx.growth)
                })?;
                let bound = crate::agg_fn::state_bound_contained_admitted(
                    custom.as_ref(), iri, &ctx.growth,
                )?;
                if let Err(tripped) =
                    ctx.charge_amount(purrdf_core::ResourceDimension::ScratchBytes, bound)
                {
                    record_contextual_trip(ctx, local_trip, tripped);
                    return Ok(None);
                }
                let scalarvals = AggregateScalarvals::resolve(agg, &ctx.growth)?;
                let initial = crate::agg_fn::exact_numeric_cost_contained_admitted(
                    custom.as_ref(), iri, &[], &scalarvals, ctx.division, &ctx.growth,
                )?;
                if let Err(tripped) = ctx.charge_exact_numeric(initial) {
                    record_contextual_trip(ctx, local_trip, tripped);
                    return Ok(None);
                }
                let allocation = ctx.growth.charge(
                    u64::try_from(size_of::<ContextualCustomFold>())
                        .map_err(|_| EvalError::WorkspaceBoundOverflow)?,
                )?;
                let iri = ctx.growth.authored_text(iri)?;
                let accumulator = crate::agg_fn::init_contained_admitted(
                    custom.as_ref(), &iri, &scalarvals, ctx.division, &ctx.growth,
                )?;
                let value = purrdf_core::small::try_boxed(ContextualCustomFold {
                    accumulator,
                    aggregate: custom,
                    iri,
                    scalarvals,
                    survivors: AdmittedVec::new(&ctx.growth),
                    survivor_owners: AdmittedVec::new(&ctx.growth),
                    admitted_work: initial.work(),
                }).map_err(|_| EvalError::AllocationFailed {
                    construct: "contextual custom aggregate control",
                })?;
                Self::Custom { value, allocation }
            }
''' + text[end:]
replace('            Self::Custom(custom) => {\n                // Price', '            Self::Custom { value: custom, .. } => {\n                // Price')
replace('''let cost = crate::agg_fn::exact_numeric_cost_contained(
                    custom.aggregate.as_ref(),
                    &custom.iri,
                    &custom.survivors,
                    &custom.scalarvals,
                    ctx.division,
                )?;''', '''let cost = crate::agg_fn::exact_numeric_cost_contained_admitted(
                    custom.aggregate.as_ref(),
                    &custom.iri,
                    &custom.survivors,
                    &custom.scalarvals,
                    ctx.division,
                    &ctx.growth,
                )?;''')
replace('''crate::agg_fn::step_contained(
                    custom.accumulator.as_mut(),
                    &custom.iri,
                    custom.survivors.last().expect("accepted tuple"),
                )?;''', '''crate::agg_fn::step_contained_admitted(
                    &mut custom.accumulator,
                    &custom.iri,
                    custom.survivors.last().expect("accepted tuple"),
                    &ctx.growth,
                )?;''')
replace(') -> Result<Option<AggregateFinished>, EvalError> {', ') -> Result<Option<crate::WorkspaceTerm>, EvalError> {')
replace('''let result = accumulator
                    .finish_admitted(workspace)
                    .map(|value| value.map(AggregateFinished::Admitted));''', '''let result = accumulator.finish_admitted(workspace);''')
replace('''            Self::Custom(custom) => {
                let ContextualCustomFold {
                    accumulator, iri, ..
                } = *custom;
                crate::agg_fn::finish_contained(accumulator, &iri, absorb)
                    .map(|value| value.map(AggregateFinished::Resident))
            }''', '''            Self::Custom { value, allocation } => {
                // The consuming native body has destroyed the original box
                // before its outside control admission is released here.
                let result = value.finish(absorb, workspace);
                drop(allocation);
                result
            }''')
replace('''                }
                .map(|value| value.map(AggregateFinished::Admitted)),''', '''                },''')
replace('''                    .map(|value| match value {
                        AggregateFinished::Admitted(value) => ctx.intern_workspace_term(value),
                        AggregateFinished::Resident(value) => ctx
                            .scratch
                            .try_intern_checked_admitted(ctx.dataset, value, &ctx.growth),
                    })''', '''                    .map(|value| ctx.intern_workspace_term(value))''')
replace('''        .map(|value| match value {
            AggregateFinished::Admitted(value) => ctx.intern_workspace_term(value),
            AggregateFinished::Resident(value) => {
                ctx.scratch
                    .try_intern_checked_admitted(ctx.dataset, value, &ctx.growth)
            }
        })''', '''        .map(|value| ctx.intern_workspace_term(value))''')
text = text.replace('\n        .map(AggregateFinished::Admitted)', '')
text = text.replace('\n            .map(AggregateFinished::Admitted)', '')

position = text.index('\nstruct ContextualLane')
text = text[:position] + '''
impl ContextualCustomFold {
    fn finish(
        self: Box<Self>,
        absorb: &mut dyn FnMut(purrdf_xsd::ErrorCode),
        workspace: &crate::WorkspaceCapability,
    ) -> Result<Option<crate::WorkspaceTerm>, EvalError> {
        let Self { accumulator, iri, .. } = *self;
        crate::agg_fn::finish_contained_admitted(accumulator, &iri, absorb, workspace)
    }
}
''' + text[position:]
position = text.index('\nfn eval_aggregate')
helper = (stage/'custom-aggregate-native-owners.rs').read_text().split('\n', 3)[3]
text = text[:position] + '\n' + helper + text[position:]
start = text.index('pub(crate) fn eval_custom_aggregate')
end = text.index('\n/// The source-order `SUM`/`AVG`', start)
direct = (stage/'custom-aggregate-native-direct.rs').read_text().split('\n', 3)[3]
text = text[:start] + direct + text[end:]

base_path = stage/'modifier-custom-native-baseline.rs'
base_path.write_text(base)
(stage/'modifier-custom-native-postimage.rs').write_text(text)
owners_path = root/'crates/sparql-eval/src/modifier/owners.rs'
owners_base = owners_path.read_text()
assert owners_base.count('pub(super) fn order_permutation(') + owners_base.count('pub(crate) fn order_permutation(') == 1
owners_text = owners_base.replace('pub(super) fn order_permutation(', 'pub(crate) fn order_permutation(')
(stage/'modifier-custom-owners-baseline.rs').write_text(owners_base)
(stage/'modifier-custom-owners-postimage.rs').write_text(owners_text)
