from pathlib import Path
import re,difflib
wt=Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
st=wt/'.stage/sparql-eval-complete-bounded-workspace'
src=(wt/'crates/sparql-eval/src/stat_agg.rs').read_text()
def block(text,start):
 p=text.index(start); a=text.index('{',p); depth=1; i=a+1
 while depth:
  if text[i]=='{': depth+=1
  elif text[i]=='}': depth-=1
  i+=1
 return text[p:i]
def fn(name): return block(src,'fn '+name+'(')
head=src[:src.index('use std::cmp::Ordering;')]
head=head.replace('[`numeric_floor`]','[`purrdf_xsd::numeric_floor`]').replace('[`value_sub`]','[`purrdf_xsd::value_sub`]').replace('[`value_add`]','[`purrdf_xsd::value_add`]').replace('[`value_mul`]','[`purrdf_xsd::value_mul`]')
imports='''use std::cmp::Ordering;
use std::mem;
use std::sync::Arc;
use purrdf_core::TermValue;
use purrdf_xsd::{XsdDatatype, XsdValue};
#[cfg(test)]
use purrdf_xsd::value_total_cmp;
use purrdf_xsd::datatype::XSD_STRING;
use purrdf_xsd::exact::Cost;
use purrdf_xsd::exact::cost::{Shape, compare_chain, sort_rounds, sum_chain};
use crate::agg_fn::{AggregateAccumulator, AggregateRegistry, AlgebraicClass, CustomAggregate, ScalarvalKind, ScalarvalSpec, WorkspaceAccumulator, downcast_combine_partial_admitted};
use crate::error::EvalError;
use crate::modifier::{ValueClass, project_admitted, total_order_admitted};
use crate::user_fn::{Arity, Volatility};
'''
constants=src[src.index('const MEDIAN:'):src.index('/// Look up a named scalarval')]
constants=constants.replace('use purrdf_xsd::datatype::XSD_DOUBLE;\n','').replace('use purrdf_xsd::datatype::XSD_STRING;\n','')
constants+='\n'+src[src.index('const MOMENTS_STATE_BOUND'):src.index('/// The exact decimal')]
shared='\n\n'.join(fn(n) for n in ['is_numeric_or_duration_xsd','same_value_family','half','machine_word_operands','tower_step','largest','count_shape'])
shapes='''fn operand_shapes(survivors: &[Vec<TermValue>], workspace: &crate::WorkspaceCapability) -> Result<crate::AdmittedVec<Shape>, EvalError> {
    let mut shapes = crate::AdmittedVec::new(workspace);
    for tuple in survivors {
        if let Some(shape) = tuple.first().and_then(crate::expr::literal_shape) { shapes.push(shape)?; }
    }
    Ok(shapes)
}
'''
p=fn('percentile_cost').replace('p: Option<&XsdValue>) -> Cost','p: Option<&XsdValue>, workspace: &crate::WorkspaceCapability) -> Result<Cost, EvalError>').replace('return Cost::ZERO;','return Ok(Cost::ZERO);').replace('operand_shapes(survivors);','operand_shapes(survivors, workspace)?;').replace('sort.then(tower_step(finish, &[m, p, rank, diff, scaled, result]))','Ok(sort.then(tower_step(finish, &[m, p, rank, diff, scaled, result])))')
m=fn('moments_cost').replace(') -> Cost {', 'workspace: &crate::WorkspaceCapability,\n) -> Result<Cost, EvalError> {').replace('return Cost::ZERO;','return Ok(Cost::ZERO);').replace('operand_shapes(survivors);','operand_shapes(survivors, workspace)?;').replace('let squares: Vec<Shape> = shapes.iter().map(|x| x.product(*x)).collect();','let mut squares = crate::AdmittedVec::with_capacity(shapes.len(), workspace)?;\n    for x in shapes.iter() { squares.push(x.product(*x))?; }').replace('.zip(&squares)','.zip(squares.iter())').replace('sum_chain(squares)','sum_chain(squares.iter().copied())').replace('return squaring.then(sum_cost).then(sumsq_cost);','return Ok(squaring.then(sum_cost).then(sumsq_cost));')
m=m.replace('    squaring.then(sum_cost).then(sumsq_cost).then(tower_step(','    Ok(squaring.then(sum_cost).then(sumsq_cost).then(tower_step(')
m=m[:-2].rstrip()+')\n}'
# Preserve the registered metadata declarations; replace only factory/pricing doors.
aggregates=[]
for name in ['MedianAggregate','PercentileAggregate','MomentsAggregate','ModeAggregate','FirstAggregate','LastAggregate','TopKAggregate']:
 decl=block(src,'struct '+name) if name=='MomentsAggregate' else 'struct '+name+';'
 body=block(src,'impl CustomAggregate for '+name)
 for method in ['exact_numeric_cost_under','exact_numeric_cost','init_under','init']:
  try: old=block(body,'    fn '+method+'(')
  except ValueError: continue
  body=body.replace(old,'')
 native_cost={
 'MedianAggregate':'percentile_cost(survivors, Some(&half()), workspace)',
 'PercentileAggregate':'{ let p = scalarval_native(scalarvals, PERCENTILE_P, workspace)?; percentile_cost(survivors, p.as_deref(), workspace) }',
 'MomentsAggregate':'moments_cost(survivors, matches!(self.kind, MomentsKind::Stddev | MomentsKind::StddevPop), division, workspace)',
 'ModeAggregate':'{ if machine_word_operands(survivors) { return Ok(Cost::ZERO); } let shapes = operand_shapes(survivors, workspace)?; Ok(compare_chain(&shapes, 1)) }',
 'FirstAggregate':'Ok(Cost::ZERO)', 'LastAggregate':'Ok(Cost::ZERO)',
 'TopKAggregate':'''{ let k = scalarval_native(scalarvals, TOPK_K, workspace)?.as_deref().and_then(|value| match value { XsdValue::Integer { value, .. } if *value > 0 => u64::try_from(*value).ok(), _ => None }).unwrap_or(1); if machine_word_operands(survivors) { return Ok(Cost::ZERO); } let shapes = operand_shapes(survivors, workspace)?; let rounds = k.min(shapes.len() as u64).saturating_add(1).saturating_add(sort_rounds(shapes.len())); Ok(compare_chain(&shapes, rounds)) }'''}[name]
 init={
 'MedianAggregate':'PercentileAccumulator { p: Some(inline_stat_value(half())?), state: ValueSeries::Empty, workspace: workspace.clone() }',
 'PercentileAggregate':'PercentileAccumulator { p: scalarval_native(scalarvals, PERCENTILE_P, workspace)?, state: ValueSeries::Empty, workspace: workspace.clone() }',
 'MomentsAggregate':'MomentsAccumulator { kind: self.kind, state: MomentsState::Empty, division, workspace: workspace.clone() }',
 'ModeAggregate':'ModeAccumulator { values: crate::AdmittedVec::new(workspace), workspace: workspace.clone() }',
 'FirstAggregate':'EdgeAccumulator { value: None, last: false, workspace: workspace.clone() }',
 'LastAggregate':'EdgeAccumulator { value: None, last: true, workspace: workspace.clone() }',
 'TopKAggregate':'''TopKAccumulator { state: match scalarval_native(scalarvals, TOPK_K, workspace)?.as_deref().and_then(|value| match value { XsdValue::Integer { value, .. } if *value > 0 => usize::try_from(*value).ok(), _ => None }) { Some(k) => TopKState::Valid { k, values: crate::AdmittedVec::new(workspace) }, None => TopKState::Poisoned }, workspace: workspace.clone() }'''}[name]
 # Parameters are explicitly named only where consumed, avoiding unused-variable lints.
 costparams=('scalarvals' if name in ['PercentileAggregate','TopKAggregate'] else '_scalarvals', 'division' if name=='MomentsAggregate' else '_division','survivors' if name not in ['FirstAggregate','LastAggregate'] else '_survivors','workspace' if name not in ['FirstAggregate','LastAggregate'] else '_workspace')
 initparams=('scalarvals' if name in ['PercentileAggregate','TopKAggregate'] else '_scalarvals','division' if name=='MomentsAggregate' else '_division')
 methods=f'''
    fn exact_numeric_cost(&self, survivors: &[Vec<TermValue>], scalarvals: &[(String, TermValue)]) -> Cost {{
        self.exact_numeric_cost_under(survivors, scalarvals, purrdf_xsd::exact::DivisionPolicy::xsd_default())
    }}
    fn exact_numeric_cost_under(&self, survivors: &[Vec<TermValue>], scalarvals: &[(String, TermValue)], division: purrdf_xsd::exact::DivisionPolicy) -> Cost {{
        self.exact_numeric_cost_admitted(survivors, scalarvals, division, &crate::WorkspaceCapability::resident()).expect("resident statistical cost")
    }}
    fn exact_numeric_cost_admitted(&self, {costparams[2]}: &[Vec<TermValue>], {costparams[0]}: &[(String, TermValue)], {costparams[1]}: purrdf_xsd::exact::DivisionPolicy, {costparams[3]}: &crate::WorkspaceCapability) -> Result<Cost, EvalError> {{ {native_cost} }}
    fn init(&self, scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {{
        self.init_under(scalarvals, purrdf_xsd::exact::DivisionPolicy::xsd_default())
    }}
    fn init_under(&self, scalarvals: &[(String, TermValue)], division: purrdf_xsd::exact::DivisionPolicy) -> Box<dyn AggregateAccumulator> {{
        self.init_admitted(scalarvals, division, &crate::WorkspaceCapability::resident()).expect("resident statistical factory").into_resident().unwrap_or_else(|_| unreachable!("resident statistical owner"))
    }}
    fn init_admitted(&self, {initparams[0]}: &[(String, TermValue)], {initparams[1]}: purrdf_xsd::exact::DivisionPolicy, workspace: &crate::WorkspaceCapability) -> Result<WorkspaceAccumulator, EvalError> {{
        WorkspaceAccumulator::new({init}, workspace)
    }}
'''
 body=body[:-1]+methods+'}'
 aggregates.append(decl+'\n'+body)
acc=(st/'stat-native-accumulators.rs').read_text().replace('downcast_combine_partial::<Self>(other)?','downcast_combine_partial_admitted::<Self>(other, &self.workspace)?')
registration=src[src.index('impl AggregateRegistry {'):]
post=head+imports+constants+'\n'+shared+'\n'+shapes+'\n'+p+'\n'+m+'\n'+(st/'stat-native-kernels.rs').read_text()+'\n'+acc+'\n'+'\n\n'.join(aggregates)+'\n'+registration
(st/'stat_agg-native-postimage.rs').write_text(post)
