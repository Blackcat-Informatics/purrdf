from pathlib import Path
stage=Path('.stage/sparql-eval-complete-bounded-workspace'); p=stage/'native-call-runtime-postimage.rs'; s=p.read_text()
s=s.replace('GraphPattern, NamedNodePattern, PropertyFunctionCall,','GraphPattern, NamedNode, NamedNodePattern, PropertyFunctionCall,',1)
s=s.replace('        let mut args = Vec::new();\n        let mut owners = Vec::new();','        let mut owners = Vec::new();\n        let mut args = Vec::new();',1)
# Preserve fixture/resident entry signatures through the original native bodies.
wrappers='''
#[cfg(test)]
fn compile_arg(term: &TermPattern, slots: &mut DetHashMap<Variable,usize>, schema: &mut VarSchema,
    slot_cols: &mut Vec<Option<usize>>, slot_seed: &mut Vec<Option<usize>>, input: &VarSchema) -> Result<Arg,EvalError> {
    let workspace = crate::WorkspaceCapability::resident();
    let mut frame = crate::workspace::LexicalFrame::new(&workspace);
    let mut memory = purrdf_lex::allocation::Memory::new(&mut frame);
    let old = std::alloc::Layout::array::<Option<usize>>(slot_cols.capacity()).map_err(|_|EvalError::WorkspaceBoundOverflow)?.size()
        .checked_add(std::alloc::Layout::array::<Option<usize>>(slot_seed.capacity()).map_err(|_|EvalError::WorkspaceBoundOverflow)?.size())
        .and_then(|n| n.checked_add(purrdf_core::hash::hash_table_allocation_bound::<(Variable,usize)>(slots.capacity())?))
        .ok_or(EvalError::WorkspaceBoundOverflow)?;
    memory.add_bytes(old).map_err(call_storage)?;
    let mut owners = Vec::new();
    compile_arg_native(term,slots,schema,slot_cols,slot_seed,input,&workspace,&mut owners,&mut memory)
}
#[cfg(test)]
fn slot_for(variable: Variable, projected: bool, slots: &mut DetHashMap<Variable,usize>,
    schema: &mut VarSchema, slot_cols: &mut Vec<Option<usize>>, slot_seed: &mut Vec<Option<usize>>, input: &VarSchema) -> usize {
    let workspace = crate::WorkspaceCapability::resident();
    let mut frame = crate::workspace::LexicalFrame::new(&workspace);
    let mut memory = purrdf_lex::allocation::Memory::new(&mut frame);
    let old = slot_cols.capacity() * size_of::<Option<usize>>() + slot_seed.capacity() * size_of::<Option<usize>>()
        + purrdf_core::hash::hash_table_allocation_bound::<(Variable,usize)>(slots.capacity()).expect("resident table layout");
    memory.add_bytes(old).expect("resident admission");
    slot_for_native(variable,projected,slots,schema,slot_cols,slot_seed,input,&workspace,&mut memory).expect("resident slot allocation")
}
#[cfg(test)]
fn triple_is_ground(triple: &TriplePattern) -> bool {
    let workspace = crate::WorkspaceCapability::resident();
    let mut frame = crate::workspace::LexicalFrame::new(&workspace);
    triple_is_ground_native(triple, &mut purrdf_lex::allocation::Memory::new(&mut frame)).expect("resident ground walk")
}
#[cfg(test)]
fn unobserved_positions(args: &[Arg], cols: &[Option<usize>], seed: &[Option<usize>]) -> Vec<bool> {
    let workspace = crate::WorkspaceCapability::resident();
    let mut frame = crate::workspace::LexicalFrame::new(&workspace);
    unobserved_positions_native(args, cols, seed, &mut purrdf_lex::allocation::Memory::new(&mut frame)).expect("resident occurrence walk")
}
'''
at=s.index('\n// ---------------------------------------------------------------------------\n// The driver'); s=s[:at]+wrappers+s[at:]
# Runtime term work lists and values carry independent original owners.
a=s.index('fn arg_value('); b=s.index('\n/// Match one emitted row',a); block=s[a:b]
block=block.replace('fn arg_value(arg: &Arg, seed: &[Option<TermValue>]) -> Option<TermValue> {','''fn arg_value_admitted(arg: &Arg, seed: &[Option<crate::WorkspaceTerm>],
    workspace: &crate::WorkspaceCapability) -> Result<Option<crate::WorkspaceTerm>, EvalError> {''')
block=block.replace('    let mut built: Vec<TermValue> = Vec::new();\n    let mut pending = vec![Step::Value(arg)];','''    let mut built = crate::AdmittedVec::new(workspace);
    let mut pending = crate::AdmittedVec::new(workspace);
    pending.push(Step::Value(arg))?;''')
block=block.replace('Step::Value(Arg::Constant(value)) => value.clone(),','Step::Value(Arg::Constant(value)) => workspace.clone_term(value)?,')
block=block.replace('Step::Value(Arg::Slot(slot)) => seed[*slot].clone()?,','''Step::Value(Arg::Slot(slot)) => {
                let Some(value) = &seed[*slot] else { return Ok(None); };
                workspace.clone_term(value)?
            },''')
for text in ['Step::Assemble','Step::Value(&parts[2])','Step::Value(&parts[1])','Step::Value(&parts[0])']:
    block=block.replace('pending.push('+text+');','pending.push('+text+')?;')
block=block.replace('''TermValue::Triple {
                    s: TermBox::new(s),
                    p: TermBox::new(p),
                    o: TermBox::new(o),
                }''','crate::WorkspaceTerm::triple(s, p, o, workspace)?')
block=block.replace('        built.push(value);','        built.push(value)?;').replace('    built.pop()','    Ok(built.pop())')
wrapper='''
fn arg_value(arg: &Arg, seed: &[Option<TermValue>]) -> Option<TermValue> {
    let workspace = crate::WorkspaceCapability::resident();
    let mut owned = crate::AdmittedVec::new(&workspace);
    for value in seed { owned.push(value.as_ref().map(|v|workspace.clone_term(v).expect("resident term copy"))).expect("resident seed buffer"); }
    arg_value_admitted(arg, &owned, &workspace).expect("resident argument build")
        .map(|value| value.into_resident().expect("resident argument owner"))
}
'''
s=s[:a]+block+wrapper+s[b:]
a=s.index('fn unify_row('); b=s.index('\n// ---------------------------------------------------------------------------\n// One call',a); block=s[a:b]
block=block.replace('fn unify_row(args: &[Arg], emitted: &[TermValue], values: &mut [Option<TermValue>]) -> bool {\n    args.iter()\n        .zip(emitted)\n        .all(|(arg, value)| unify_term(arg, value, values))\n}', '''fn unify_row_admitted(args: &[Arg], emitted: &[TermValue], values: &mut [Option<crate::WorkspaceTerm>],
    workspace: &crate::WorkspaceCapability) -> Result<bool, EvalError> {
    for (arg, value) in args.iter().zip(emitted) {
        if !unify_term_admitted(arg, value, values, workspace)? { return Ok(false); }
    }
    Ok(true)
}''')
block=block.replace('fn unify_term(arg: &Arg, value: &TermValue, values: &mut [Option<TermValue>]) -> bool {\n    let mut pending = vec![(arg, value)];','''fn unify_term_admitted(arg: &Arg, value: &TermValue, values: &mut [Option<crate::WorkspaceTerm>],
    workspace: &crate::WorkspaceCapability) -> Result<bool, EvalError> {
    let mut pending = crate::AdmittedVec::new(workspace);
    pending.push((arg, value))?;''')
block=block.replace('return false;', 'return Ok(false);').replace('_ => return false,','_ => return Ok(false),').replace('if existing != value {','if &**existing != value {').replace('None => values[*slot] = Some(value.clone()),','None => values[*slot] = Some(workspace.clone_term(value)?),')
for text in ['(&parts[2], o)','(&parts[1], p)','(&parts[0], s)']:
    block=block.replace('pending.push('+text+');','pending.push('+text+')?;')
block=block.replace('    true\n}', '    Ok(true)\n}')
wrapper='''
fn unify_row(args: &[Arg], emitted: &[TermValue], values: &mut [Option<TermValue>]) -> bool {
    let workspace = crate::WorkspaceCapability::resident();
    let mut owned = crate::AdmittedVec::new(&workspace);
    for value in values.iter_mut() { owned.push(value.take().map(crate::WorkspaceTerm::resident)).expect("resident unification buffer"); }
    let result = unify_row_admitted(args,emitted,owned.as_mut_slice(),&workspace).expect("resident unification");
    for (dst,value) in values.iter_mut().zip(owned) { *dst = value.map(|v|v.into_resident().expect("resident unified term")); }
    result
}
#[cfg(test)]
fn unify_term(arg: &Arg, value: &TermValue, values: &mut [Option<TermValue>]) -> bool {
    unify_row(std::slice::from_ref(arg),std::slice::from_ref(value),values)
}
'''
s=s[:a]+block+wrapper+s[b:]
# Actual governed driver uses only original native arrays and term owners.
a=s.index('fn eval_call_over<'); b=s.index('\n/// Resolve a call',a); block=s[a:b]
block=block.replace('CallPlan::compile(call, &input.schema)?','CallPlan::compile_admitted(call, &input.schema, &ctx.growth)?')
block=block.replace('''let modes = crate::property_fn::declaration_contained(&call.iri, "declared modes", || {
        relation.modes().to_vec()
    })?;''','''let modes = crate::property_fn::declaration_contained(&call.iri, "declared modes", || {
        let mut modes = crate::AdmittedVec::new(&ctx.growth);
        for mode in relation.modes() { modes.push(*mode)?; }
        Ok::<_,EvalError>(modes)
    })??;''')
block=block.replace('    let mut seed: Vec<Option<TermValue>> = vec![None; plan.slot_count()];\n    let mut values: Vec<Option<TermValue>> = vec![None; plan.slot_count()];', '''    let mut seed = crate::AdmittedVec::with_capacity(plan.slot_count(), &ctx.growth)?;
    let mut values = crate::AdmittedVec::with_capacity(plan.slot_count(), &ctx.growth)?;
    for _ in 0..plan.slot_count() { seed.push(None)?; values.push(None)?; }''')
block=block.replace('    let mut args: Vec<Option<TermValue>> = vec![None; plan.args.len()];','''    let mut args = crate::AdmittedVec::with_capacity(plan.args.len(), &ctx.growth)?;
    for _ in 0..plan.args.len() { args.push(None)?; }''')
block=block.replace('seed[slot] = column','seed.as_mut_slice()[slot] = column').replace('.map(|term| ctx.scratch.try_value_of(ctx.dataset, term))\n                .transpose()\n                .map_err(EvalError::source_read)?;', '''.map(|term| ctx.scratch.try_owned_value_of(ctx.dataset, term, &ctx.growth,
                    |error| ctx.workspace.source_error(error)))
                .transpose()?;''')
block=block.replace('args.iter_mut().zip(&plan.args)', 'args.as_mut_slice().iter_mut().zip(&plan.args)').replace('*dst = arg_value(arg, &seed);','*dst = arg_value_admitted(arg, &seed, &ctx.growth)?;')
block=block.replace('''let refs: purrdf_core::SmallVec<[Option<&TermValue>; 4]> =
            args.iter().map(Option::as_ref).collect();''','''let mut refs = crate::AdmittedVec::with_capacity(args.len(), &ctx.growth)?;
        for value in args.iter() { refs.push(value.as_deref())?; }''')
block=block.replace('admit_mode(&modes, &call.iri, mode)?;', 'admit_mode_admitted(&modes, &call.iri, mode, &ctx.growth)?;')
block=block.replace('            values.clone_from(&seed);\n            if !unify_row(&plan.args, &emitted, &mut values) {', '''            for (dst, value) in values.as_mut_slice().iter_mut().zip(seed.iter()) {
                *dst = value.as_ref().map(|value| ctx.growth.clone_term(value)).transpose()?;
            }
            if !unify_row_admitted(&plan.args, &emitted, values.as_mut_slice(), &ctx.growth)? {''')
block=block.replace('unbound_slot_internal(&call.iri)','unbound_slot_admitted(&call.iri, &growth)')
block=block.replace('return Err(EvalError::function(format!(', 'return Err(crate::error::NativeDiagnostic::error(crate::error::NativeDiagnosticKind::Function, format_args!(')
block=block.replace('''            call.iri
        )));''', '''            call.iri
        ), &ctx.growth));''',1)
block=block.replace('return Err(EvalError::function_operational(format!(', 'return Err(crate::error::NativeDiagnostic::error(crate::error::NativeDiagnosticKind::FunctionOperational, format_args!(')
block=block.replace('''                    declared.total()
                )));''','''                    declared.total()
                ), &ctx.growth));''',1)
s=s[:a]+block+s[b:]
# Original resolver/failure presentation: stream directly into native storage.
a=s.index('fn resolve<'); b=s.index('\n/// Check that a relation',a); block=s[a:b]
block=block.replace('EvalError::function(format!(', 'crate::error::NativeDiagnostic::error(crate::error::NativeDiagnosticKind::Function, format_args!(').replace('''                call.iri
            ))''','''                call.iri
            ), &ctx.growth)''')
s=s[:a]+block+s[b:]
a=s.index('fn admit_mode('); b=s.index('\n/// The failure of the invariant',a)
s=s[:a]+'''fn admit_mode(modes: &[BindingPattern], iri: &str, mode: BindingPattern) -> Result<(), EvalError> {
    admit_mode_admitted(modes,iri,mode,&crate::WorkspaceCapability::resident())
}
fn admit_mode_admitted(modes: &[BindingPattern], iri: &str, mode: BindingPattern,
    workspace: &crate::WorkspaceCapability) -> Result<(), EvalError> {
    if modes.iter().any(|declared| declared.subsumes(mode)) { return Ok(()); }
    struct Modes<'a>(&'a [BindingPattern]);
    impl core::fmt::Display for Modes<'_> {
        fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            for (i,mode) in self.0.iter().enumerate() { if i != 0 { out.write_str(", ")?; } core::fmt::Display::fmt(mode,out)?; }
            Ok(())
        }
    }
    Err(crate::error::NativeDiagnostic::error(crate::error::NativeDiagnosticKind::Function,
        format_args!("property function <{iri}> cannot serve the invocation `{mode}`; it declares [{}] — a position the plan expected to be bound is unbound in this row",Modes(modes)), workspace))
}
fn unbound_slot_admitted(iri: &str, workspace: &crate::WorkspaceCapability) -> EvalError {
    crate::error::NativeDiagnostic::error(crate::error::NativeDiagnosticKind::Internal,
        format_args!("property function <{iri}>: a unified row left an argument slot unbound"), workspace)
}
'''+s[b:]
# Recursive oracles construct the same representation, preserving their independent laws.
s=s.replace('Arg::Triple(Box::new([subject, predicate, object]))','Arg::Triple([subject.into(), predicate.into(), object.into()])')
p.write_text(s)
