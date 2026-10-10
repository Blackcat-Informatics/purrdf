# Stage-only proposal assembler; shipping sources are read-only.
from pathlib import Path
import difflib

root = Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
stage = root / '.stage/sparql-eval-complete-bounded-workspace'
post = stage / 'certainty-owner-postimages'
path = 'crates/sparql-eval/src/property_fn_plan.rs'
before = (root / path).read_text()
after = before

def change(old, new, count=1):
    global after
    assert after.count(old) == count, (old[:90], after.count(old), count)
    after = after.replace(old, new)

def fallible_pushes(body, names):
    # Match full Rust method arguments rather than a line heuristic; none of these
    # fragments contain literal parentheses inside the selected push expressions.
    for name in names:
        needle = name + '.push('
        start = 0
        while True:
            start = body.find(needle, start)
            if start < 0:
                break
            opening = start + len(needle) - 1
            nesting, end = 1, opening + 1
            while nesting:
                if body[end] == '(':
                    nesting += 1
                elif body[end] == ')':
                    nesting -= 1
                end += 1
            if body[end:end+1] != '?':
                body = body[:end] + '?' + body[end:]
                end += 1
            start = end
    return body

imports = '''use crate::solution::{SchemaBuilder, VarSchema};
use crate::workspace::AdmittedVec;
use crate::WorkspaceCapability;
'''
change('use crate::DetHashSet;\n', 'use crate::DetHashSet;\n' + imports)

anchor = '/// [`collect_certainly_bound`] for a pattern evaluated with `context` already bound:'
seams = '''/// The same certainty law with every engine-created owner admitted before growth.
/// The returned immutable schema keeps its original payload admission alive.
pub(crate) fn collect_certainly_bound_admitted(
    pattern: &GraphPattern,
    workspace: &WorkspaceCapability,
) -> Result<VarSchema, EvalError> {
    collect_certainly_bound_in_admitted(pattern, &VarSchema::default(), workspace)
}

/// Contextual certainty, without adding the caller's context to the result.
pub(crate) fn collect_certainly_bound_in_admitted(
    pattern: &GraphPattern,
    context: &VarSchema,
    workspace: &WorkspaceCapability,
) -> Result<VarSchema, EvalError> {
    collect_bound_admitted(pattern, context, None, workspace)
}

'''
change(anchor, seams + anchor)

start = after.index('fn collect_bound(\n')
end = after.index('/// Whether an aggregate\'s output is bound', start)
body = after[start:end]
signature_end = body.index('    /// The context a node is evaluated under:')
body = '''pub(crate) fn collect_bound_admitted(
    pattern: &GraphPattern,
    context: &VarSchema,
    given_written: Option<&VarSchema>,
    workspace: &WorkspaceCapability,
) -> Result<VarSchema, EvalError> {
''' + body[signature_end:]
body = '/// Contextual certainty with optional promised bindings and retained schema owners.\n' + body
body = body.replace('DetHashSet<Variable>', 'VarSchema')
body = body.replace("given: Written<'s>", "given: Option<&'s VarSchema>")
body = body.replace("given: Written<'_>", "given: Option<&VarSchema>")
body = body.replace('Self::Given => given.everywhere,', 'Self::Given => given,')
body = body.replace('let mut arena: Vec<VarSchema> = Vec::new();', 'let mut arena = AdmittedVec::<VarSchema>::new(workspace);')
body = body.replace('let mut values: Vec<VarSchema> = Vec::new();', 'let mut values = AdmittedVec::<VarSchema>::new(workspace);')
body = body.replace('let mut steps = vec![Step::Enter(pattern, Context::Given, Writes::Given)];',
    'let mut steps = AdmittedVec::new(workspace);\n    steps.push(Step::Enter(pattern, Context::Given, Writes::Given))?;')
body = body.replace('let mut bound = DetHashSet::default();', 'let mut bound = SchemaBuilder::new(workspace);', 3)
body = body.replace('collect_triple_vars(triple, &mut bound);', 'collect_triple_vars_admitted(triple, &mut bound, workspace)?;')
body = body.replace('collect_term_vars(subject, &mut bound);', 'collect_term_vars_admitted(subject, &mut bound, workspace)?;')
body = body.replace('collect_term_vars(object, &mut bound);', 'collect_term_vars_admitted(object, &mut bound, workspace)?;')
body = body.replace('collect_term_vars(term, &mut bound);', 'collect_term_vars_admitted(term, &mut bound, workspace)?;')
body = body.replace('values.push(bound);', 'values.push(bound.finish()?);', 3)
old = '''                        let bound = variables
                            .iter()
                            .enumerate()
                            .filter(|(column, _)| {
                                bindings
                                    .iter()
                                    .all(|row| row.get(*column).is_some_and(Option::is_some))
                            })
                            .map(|(_, variable)| variable.clone())
                            .collect();'''
new = '''                        let bound = VarSchema::from_vars_admitted(
                            variables.iter().enumerate().filter(|(column, _)| {
                                bindings.iter().all(|row| row.get(*column).is_some_and(Option::is_some))
                            }).map(|(_, variable)| variable.clone()),
                            workspace,
                        )?;'''
assert old in body
body = body.replace(old, new)
body = body.replace('values.push(DetHashSet::default());', 'values.push(VarSchema::default());')
body = body.replace('let narrowed = narrowed_to(resolve(ctx, context, &arena), variables);',
    'let narrowed = narrowed_to_admitted(resolve(ctx, context, &arena), variables, workspace)?;')
old = '''                let mut right_context = outer.clone();
                let mut supplied = written
                    .set(given_written, &arena)
                    .cloned()
                    .unwrap_or_default();
                for (input, _) in &policy.inputs {
                    right_context.remove(input);
                    supplied.remove(input);
                }'''
new = '''                let shadowed = VarSchema::from_vars_admitted(
                    policy.inputs.iter().map(|(input, _)| input.clone()), workspace,
                )?;
                let mut right_context = SchemaBuilder::new(workspace);
                for variable in outer.vars().iter().filter(|variable| !shadowed.contains(variable)) {
                    right_context.push(variable.clone())?;
                }
                let written_set = written.set(given_written, &arena);
                let mut supplied = SchemaBuilder::new(workspace);
                for variable in written_set.into_iter().flat_map(|set| set.vars().iter())
                    .filter(|variable| !shadowed.contains(variable)) {
                    supplied.push(variable.clone())?;
                }'''
assert old in body
body = body.replace(old, new)
body = body.replace('right_context.insert(input.clone());', 'right_context.push(input.clone())?;')
body = body.replace('supplied.insert(input.clone());', 'supplied.push(input.clone())?;')
body = body.replace('arena.push(right_context);\n                arena.push(supplied);',
    'arena.push(right_context.finish()?);\n                arena.push(supplied.finish()?);')
body = body.replace('mark.checked_add(2)\n                                .expect("an allocated arena index is representable")',
    'mark.checked_add(2).ok_or(EvalError::WorkspaceBoundOverflow)?')
body = body.replace('''                let mut right_context = resolve(ctx, context, &arena).clone();
                right_context.extend(left_bound.iter().cloned());''',
    '''                let right_context = resolve(ctx, context, &arena)
                    .union_admitted(&left_bound, workspace)?;''')
body = body.replace('bound.extend(left_bound);', 'bound = bound.union_admitted(&left_bound, workspace)?;')
body = body.replace('left.extend(right);', 'left = left.union_admitted(&right, workspace)?;')
body = body.replace('bound.extend(right);', 'bound = bound.union_admitted(&right, workspace)?;')
old = '''                        let at = values.len() - arms.len();
                        let mut arms_bound = values.drain(at..);
                        let first = arms_bound.next().unwrap_or_default();
                        arms_bound.fold(first, |common, bound| {
                            common.intersection(&bound).cloned().collect()
                        })'''
new = '''                        let at = values.len().checked_sub(arms.len())
                            .expect("each UNION arm produced its certainty value");
                        let mut arms_bound = values.drain_from(at);
                        let mut common = arms_bound.next().unwrap_or_default();
                        for bound in arms_bound {
                            common = intersection_admitted(&common, &bound, workspace)?;
                        }
                        common'''
assert old in body
body = body.replace(old, new)
old = '''                        bound.extend(
                            requires(expr, Outcome::Truth)
                                .unwrap_or_default()
                                .into_iter()
                                .filter(|variable| {
                                    !written.writes(variable, given_written, &arena)
                                }),
                        );'''
new = '''                        let required = requires_admitted(expr, Outcome::Truth, workspace)?
                            .unwrap_or_default();
                        let constrained = VarSchema::from_vars_admitted(
                            required.vars().iter().filter(|variable| {
                                !written.writes(variable, given_written, &arena)
                            }).cloned(), workspace,
                        )?;
                        bound = bound.union_admitted(&constrained, workspace)?;'''
assert old in body
body = body.replace(old, new)
body = body.replace('if expression_reads_only_bound(expression, &|read| {', 'if expression_reads_only_bound_admitted(expression, &|read| {')
body = body.replace('''                                || written.writes(read, given_written, &arena)
                        }) {''', '''                                || written.writes(read, given_written, &arena)
                        }, workspace)? {''')
body = body.replace('bound.insert(variable.clone());', 'bound.push_admitted(variable.clone(), workspace)?;')
old = '''                        variables
                            .iter()
                            .filter(|variable| inner_bound.contains(*variable))
                            .cloned()
                            .collect()'''
new = '''                        VarSchema::from_vars_admitted(
                            variables.iter().filter(|variable| inner_bound.contains(variable)).cloned(),
                            workspace,
                        )?'''
assert old in body
body = body.replace(old, new)
old = '''                        let mut bound: VarSchema = variables
                            .iter()
                            .filter(|key| row_binds(key))
                            .cloned()
                            .collect();'''
new = '''                        let mut bound = SchemaBuilder::new(workspace);
                        for variable in variables.iter().filter(|key| row_binds(key)) {
                            bound.push(variable.clone())?;
                        }'''
assert old in body
body = body.replace(old, new)
body = body.replace('if aggregate_certainly_binds(aggregate, grouped, &|read| {',
    'if aggregate_certainly_binds_admitted(aggregate, grouped, &|read| {')
body = body.replace('''                                row_binds(read) || written.writes(read, given_written, &arena)
                            }) {''', '''                                row_binds(read) || written.writes(read, given_written, &arena)
                            }, workspace)? {''')
group_at = body.index('                    GraphPattern::Group {', body.index('            Step::Exit'))
group_end = body.index('                    GraphPattern::Bgp {', group_at)
group = body[group_at:group_end].replace('bound.push_admitted(variable.clone(), workspace)?;', 'bound.push(variable.clone())?;')
group = group.replace('                        bound\n', '                        bound.finish()?\n')
body = body[:group_at] + group + body[group_end:]
body = body.replace('''    out.extend(values.pop().expect("the root's set is computed last"));
}''', '''    Ok(values.pop().expect("the root's set is computed last"))
}''')
body = fallible_pushes(body, ['steps', 'values', 'arena'])
wrapper = '''fn collect_bound(
    pattern: &GraphPattern,
    context: &DetHashSet<Variable>,
    given_written: Written<'_>,
    out: &mut DetHashSet<Variable>,
) {
    // These sets are caller-owned resident planner inputs and output. Bounded
    // callers keep the VarSchema returned by collect_bound_admitted instead.
    let workspace = WorkspaceCapability::default();
    let context = VarSchema::from_vars_admitted(context.iter().cloned(), &workspace)
        .expect("resident certainty context");
    let written = given_written.everywhere.map(|set| {
        VarSchema::from_vars_admitted(set.iter().cloned(), &workspace)
            .expect("resident certainty writes")
    });
    let bound = collect_bound_admitted(pattern, &context, written.as_ref(), &workspace)
        .expect("resident certainly-bound allocation");
    out.extend(bound.vars().iter().cloned());
}

'''
after = after[:start] + wrapper + body + after[end:]

start = after.index('fn aggregate_certainly_binds(\n')
end = after.index('/// The part of `context` a sub-`SELECT`', start)
after = after[:start] + '''fn aggregate_certainly_binds_admitted(
    aggregate: &AggregateExpression,
    grouped: bool,
    row_binds: &dyn Fn(&Variable) -> bool,
    workspace: &WorkspaceCapability,
) -> Result<bool, EvalError> {
    match aggregate.function() {
        AggregateFunction::Count
        | AggregateFunction::Sum
        | AggregateFunction::Avg
        | AggregateFunction::GroupConcat
        | AggregateFunction::Fold => Ok(true),
        AggregateFunction::Sample
        | AggregateFunction::Min
        | AggregateFunction::Max
        | AggregateFunction::Custom(_) => {
            if !grouped { return Ok(false); }
            for arg in aggregate.args() {
                if !expression_reads_only_bound_admitted(arg, row_binds, workspace)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
    }
}

''' + after[end:]

start = after.index('fn narrowed_to(context:')
end = after.index('/// Whether `expr` can be left without a value', start)
after = after[:start] + '''fn narrowed_to(context: &DetHashSet<Variable>, variables: &[Variable]) -> DetHashSet<Variable> {
    let workspace = WorkspaceCapability::default();
    let context = VarSchema::from_vars_admitted(context.iter().cloned(), &workspace)
        .expect("resident projected certainty context");
    narrowed_to_admitted(&context, variables, &workspace)
        .expect("resident projected certainty allocation").vars().iter().cloned().collect()
}

fn narrowed_to_admitted(
    context: &VarSchema,
    variables: &[Variable],
    workspace: &WorkspaceCapability,
) -> Result<VarSchema, EvalError> {
    VarSchema::from_vars_admitted(
        context.vars().iter().filter(|variable| variables.contains(variable)).cloned(), workspace,
    )
}

fn intersection_admitted(
    left: &VarSchema, right: &VarSchema, workspace: &WorkspaceCapability,
) -> Result<VarSchema, EvalError> {
    VarSchema::from_vars_admitted(
        left.vars().iter().filter(|variable| right.contains(variable)).cloned(), workspace,
    )
}

''' + after[end:]

start = after.index('fn expression_reads_only_bound(expr:')
end = after.index('/// Whether `function` is a type test:', start)
body = after[start:end]
body = body.replace('fn expression_reads_only_bound(expr: &Expression, is_bound: &dyn Fn(&Variable) -> bool) -> bool {',
    '''fn expression_reads_only_bound_admitted(
    expr: &Expression, is_bound: &dyn Fn(&Variable) -> bool,
    workspace: &WorkspaceCapability,
) -> Result<bool, EvalError> {''')
body = body.replace('let mut steps = vec![Step::Enter(expr)];',
    'let mut steps = AdmittedVec::new(workspace);\n    steps.push(Step::Enter(expr))?;')
body = body.replace('let mut values: Vec<bool> = Vec::new();', 'let mut values = AdmittedVec::<bool>::new(workspace);')
body = body.replace('1 + rest.len()', 'rest.len().checked_add(1).ok_or(EvalError::WorkspaceBoundOverflow)?')
body = body.replace('1 + haystack.len()', 'haystack.len().checked_add(1).ok_or(EvalError::WorkspaceBoundOverflow)?')
body = body.replace('let at = values.len() - count;', 'let at = values.len().checked_sub(count).expect("every operand answered");')
body = body.replace('values.drain(at..)', 'values.drain_from(at)')
body = body.replace('''    values
        .pop()
        .expect("the root's answer is the last one computed")''',
    '''    Ok(values.pop().expect("the root's answer is the last one computed"))''')
body = fallible_pushes(body, ['steps', 'values'])
wrapper = '''#[cfg(test)]
fn expression_reads_only_bound(expr: &Expression, is_bound: &dyn Fn(&Variable) -> bool) -> bool {
    expression_reads_only_bound_admitted(expr, is_bound, &WorkspaceCapability::default())
        .expect("resident expression certainty allocation")
}

'''
after = after[:start] + wrapper + body + after[end:]

# The raw set combiners remain only in the pre-existing recursive test oracle.
for signature in ['type Requires =', 'fn needs_nothing()', 'fn needs(variable:',
                  'fn all_of(left:', 'fn one_of(left:']:
    change(signature, '#[cfg(test)]\n' + signature)

anchor = '/// The outcome an expression is asked what it must have bound for'
native = '''/// A reachable requirement owns its original schema; None means impossible.
type AdmittedRequires = Option<VarSchema>;

fn needs_admitted(variable: &Variable, workspace: &WorkspaceCapability) -> Result<AdmittedRequires, EvalError> {
    Ok(Some(VarSchema::from_vars_admitted(std::iter::once(variable.clone()), workspace)?))
}

fn all_of_admitted(
    left: AdmittedRequires, right: AdmittedRequires, workspace: &WorkspaceCapability,
) -> Result<AdmittedRequires, EvalError> {
    let (Some(left), Some(right)) = (left, right) else { return Ok(None); };
    Ok(Some(left.union_admitted(&right, workspace)?))
}

fn one_of_admitted(
    left: AdmittedRequires, right: AdmittedRequires, workspace: &WorkspaceCapability,
) -> Result<AdmittedRequires, EvalError> {
    match (left, right) {
        (None, other) | (other, None) => Ok(other),
        (Some(left), Some(right)) => Ok(Some(intersection_admitted(&left, &right, workspace)?)),
    }
}

'''
change(anchor, native + anchor)

start = after.index('fn requires(expr:')
end = after.index('/// Ask `expr` what it needs to reach `outcome`:', start)
after = after[:start] + '''#[cfg(test)]
fn requires(expr: &Expression, outcome: Outcome) -> Requires {
    requires_admitted(expr, outcome, &WorkspaceCapability::default())
        .expect("resident expression requirement allocation")
        .map(|bound| bound.vars().iter().cloned().collect())
}

fn requires_admitted(
    expr: &Expression, outcome: Outcome, workspace: &WorkspaceCapability,
) -> Result<AdmittedRequires, EvalError> {
    let mut steps = AdmittedVec::new(workspace);
    steps.push(RequirementStep::Enter(expr, outcome))?;
    let mut values = AdmittedVec::<AdmittedRequires>::new(workspace);
    while let Some(step) = steps.pop() {
        match step {
            RequirementStep::Enter(expr, outcome) => {
                enter_requirement(expr, outcome, &mut steps, &mut values, workspace)?;
            }
            RequirementStep::Exit(combine, count) => {
                let at = values.len().checked_sub(count)
                    .expect("each requested outcome produced one requirement");
                let combined = {
                    let mut operands = values.drain_from(at);
                    match combine {
                        Combine::All => {
                            let mut required = SchemaBuilder::new(workspace);
                            let mut possible = true;
                            for operand in operands {
                                match operand {
                                    Some(bound) if possible => {
                                        for variable in bound.vars() {
                                            required.push(variable.clone())?;
                                        }
                                    }
                                    None => possible = false,
                                    Some(_) => {}
                                }
                            }
                            if possible { Some(required.finish()?) } else { None }
                        }
                        Combine::One => {
                            let mut combined = None;
                            for operand in operands {
                                combined = one_of_admitted(combined, operand, workspace)?;
                            }
                            combined
                        }
                        Combine::OneOrNothing => {
                            let mut combined = operands.next().unwrap_or_else(|| Some(VarSchema::default()));
                            for operand in operands {
                                combined = one_of_admitted(combined, operand, workspace)?;
                            }
                            combined
                        }
                        Combine::Conditional => {
                            let mut next = || operands.next().expect("an IF asks five operand requirements");
                            let condition_value = next();
                            let condition_true = next();
                            let then = next();
                            let condition_false = next();
                            let otherwise = next();
                            let then = all_of_admitted(condition_true, then, workspace)?;
                            let otherwise = all_of_admitted(condition_false, otherwise, workspace)?;
                            let branch = one_of_admitted(then, otherwise, workspace)?;
                            all_of_admitted(condition_value, branch, workspace)?
                        }
                    }
                };
                values.push(combined)?;
            }
        }
    }
    Ok(values.pop().expect("the root's requirement is the last one combined"))
}

''' + after[end:]

start = after.index("fn enter_requirement<'e>(")
end = after.index('/// Push `operands` to be asked', start)
body = after[start:end]
body = body.replace('steps: &mut Vec<RequirementStep<\'e>>,', "steps: &mut AdmittedVec<RequirementStep<'e>>,")
body = body.replace('values: &mut Vec<Requires>,\n) {',
    'values: &mut AdmittedVec<AdmittedRequires>,\n    workspace: &WorkspaceCapability,\n) -> Result<(), EvalError> {')
body = body.replace('needs(variable)', 'needs_admitted(variable, workspace)?')
body = body.replace('needs_nothing()', 'Some(VarSchema::default())')
body = body.replace('crate::expr::constant_ebv(literal)', 'crate::expr::constant_ebv_admitted(literal, workspace)?')
body = body.replace('.then(DetHashSet::default)', '.then(VarSchema::default)')
body = body.replace('values\n                .push(', 'values.push(')
# Every push_operands/conditional is fallible and shares the caller workspace via
# the already-owned AdmittedVec; no admission or capacity estimator lives here.
for name in ['push_operands', 'push_conditional']:
    needle = name + '('
    pos = 0
    while True:
        pos = body.find(needle, pos)
        if pos < 0: break
        i, depth = pos + len(needle), 1
        while depth:
            if body[i] == '(': depth += 1
            elif body[i] == ')': depth -= 1
            i += 1
        body = body[:i] + '?' + body[i:]
        pos = i + 1
body = body.replace('        return;\n', '        return Ok(());\n')
body = fallible_pushes(body, ['steps', 'values'])
after = after[:start] + body + after[end:]

start = after.index("fn push_operands<'e>(")
end = after.index('/// Push an `IF`\'s five operand requirements', start)
body = after[start:end]
body = body.replace("steps: &mut Vec<RequirementStep<'e>>,", "steps: &mut AdmittedVec<RequirementStep<'e>>,")
body = body.replace(') {\n    let exit = steps.len();', ') -> Result<(), EvalError> {\n    let exit = steps.len();')
body = body.replace('steps[exit] =', 'steps.as_mut_slice()[exit] =')
body = body.replace('steps[exit + 1..].reverse();', 'steps.as_mut_slice()[exit + 1..].reverse();\n    Ok(())')
body = fallible_pushes(body, ['steps'])
after = after[:start] + body + after[end:]

start = after.index("fn push_conditional<'e>(")
end = after.index('/// Add a triple pattern\'s variables', start)
body = after[start:end]
body = body.replace("steps: &mut Vec<RequirementStep<'e>>,", "steps: &mut AdmittedVec<RequirementStep<'e>>,")
body = body.replace(') {\n    push_operands(', ') -> Result<(), EvalError> {\n    push_operands(')
body = body.replace('        .into_iter(),\n    );', '        .into_iter(),\n    )')
after = after[:start] + body + after[end:]

start = after.index('fn collect_triple_vars(triple:')
end = after.index('// ---------------------------------------------------------------------------\n// The registry fingerprint', start)
body = after[start:end]
body = body.replace('fn collect_triple_vars(triple: &TriplePattern, out: &mut DetHashSet<Variable>) {',
    '''fn collect_triple_vars_admitted(
    triple: &TriplePattern, out: &mut SchemaBuilder, workspace: &WorkspaceCapability,
) -> Result<(), EvalError> {''')
body = body.replace('let mut pending = vec![triple];',
    'let mut pending = AdmittedVec::new(workspace);\n    pending.push(triple)?;')
body = body.replace('out.insert(variable.clone());', 'out.push(variable.clone())?;')
body = body.replace('''    }
}

/// Add a term position''', '''    }
    Ok(())
}

/// Add a term position''', 1)
body = body.replace('/// Add a term position\'s variables, a quoted triple\'s through [`collect_triple_vars`].',
    '/// Add a term position\'s variables through the same admitted nested-triple walk.')
body = body.replace('fn collect_term_vars(term: &TermPattern, out: &mut DetHashSet<Variable>) {',
    '''fn collect_term_vars_admitted(
    term: &TermPattern, out: &mut SchemaBuilder, workspace: &WorkspaceCapability,
) -> Result<(), EvalError> {''')
body = body.replace('TermPattern::Triple(triple) => collect_triple_vars(triple, out),',
    'TermPattern::Triple(triple) => collect_triple_vars_admitted(triple, out, workspace)?,')
idx = body.rfind('    }\n}\n')
assert idx >= 0
body = body[:idx] + body[idx:].replace('    }\n}\n', '    }\n    Ok(())\n}\n', 1)
body = fallible_pushes(body, ['pending'])
body += '''#[cfg(test)]
fn collect_term_vars(term: &TermPattern, out: &mut DetHashSet<Variable>) {
    let workspace = WorkspaceCapability::default();
    let mut bound = SchemaBuilder::new(&workspace);
    collect_term_vars_admitted(term, &mut bound, &workspace)
        .expect("resident term variable walk");
    out.extend(bound.finish().expect("resident term variable schema").vars().iter().cloned());
}

'''
after = after[:start] + body + after[end:]

# Preserve documentation links after the native aggregate seam's rename.
after = after.replace('[`aggregate_certainly_binds`]', '[`aggregate_certainly_binds_admitted`]')
after = after.replace('[`expression_reads_only_bound`]', '[`expression_reads_only_bound_admitted`]')
after = after.replace('[`requires`]', '[`requires_admitted`]')
after = after.replace('[`Requires`]', '[`AdmittedRequires`]')
after = after.replace('[`all_of`]', '[`all_of_admitted`]')
after = after.replace('[`one_of`]', '[`one_of_admitted`]')

# Public API concrete native projection, not a second literal implementation.
path2 = 'crates/sparql-eval/src/expr.rs'
before2 = (root / path2).read_text()
start = before2.index('pub(crate) fn constant_ebv(')
end = before2.index('/// The effective boolean value of a concrete term', start)
after2 = before2[:start] + '''#[cfg(test)]
pub(crate) fn constant_ebv(literal: &purrdf_sparql_algebra::Literal) -> Option<bool> {
    constant_ebv_admitted(literal, &crate::WorkspaceCapability::default())
        .expect("resident constant EBV allocation")
}

/// The same constant law over borrowed authored spelling and admitted parsed owners.
pub(crate) fn constant_ebv_admitted(
    literal: &purrdf_sparql_algebra::Literal, workspace: &crate::WorkspaceCapability,
) -> Result<Option<bool>, EvalError> {
    if literal.language().is_some() { return Ok(None); }
    let Some(datatype) = XsdDatatype::from_iri(literal.datatype().as_str()) else { return Ok(None); };
    let parsed = crate::parsed_value::ParsedValue::parse(literal.value(), datatype, false, workspace)?;
    Ok(parsed.as_deref().and_then(effective_boolean_value))
}

''' + before2[end:]

fixtures = stage / 'certainty-owner-fixtures.rs'
if fixtures.exists():
    after += '\n' + fixtures.read_text()

path3 = 'crates/sparql-eval/src/solution.rs'
before3 = (root / path3).read_text()
old_union = '''        let mut out = self.clone();
        for variable in other.vars() { out.push_admitted(variable.clone(), workspace)?; }
        Ok(out)'''
new_union = '''        if self.is_empty() { return Ok(other.clone()); }
        if other.vars().iter().all(|variable| self.contains(variable)) { return Ok(self.clone()); }
        Self::from_vars_admitted(self.vars().iter().chain(other.vars()).cloned(), workspace)'''
assert before3.count(old_union) == 1
after3 = before3.replace(old_union, new_union)

items = [(path, before, after), (path2, before2, after2), (path3, before3, after3)]
owner_fixtures = stage / 'certainty-owner-lifetime-fixtures.rs'
if owner_fixtures.exists():
    path4 = 'crates/sparql-eval/src/workspace.rs'
    before4 = (root / path4).read_text()
    # Append within the existing fixture home: one reservation law/account helper.
    closing = before4.rfind('\n}')
    assert closing >= before4.index('mod owned_row_regressions')
    after4 = before4[:closing] + '\n' + owner_fixtures.read_text() + before4[closing:]
    items.append((path4, before4, after4))
patch = []
for path, old, new in items:
    dest = post / path
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(new)
    patch.append('diff --git a/' + path + ' b/' + path + '\n')
    patch.extend(difflib.unified_diff(old.splitlines(True), new.splitlines(True),
        fromfile='a/' + path, tofile='b/' + path))
(stage / 'certainty-owner-draft.patch').write_text(''.join(patch))
(stage / 'certainty-owner-draft.rs').write_text(after[after.index('// Certainly-bound variables'):after.index('// The registry fingerprint')])
print('Wrote original-home certainty and borrowed constant EBV proposals.')
