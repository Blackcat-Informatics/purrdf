from pathlib import Path
import difflib
root=Path.cwd(); stage=root/'.stage/sparql-eval-complete-bounded-workspace'; path=Path('crates/sparql-eval/src/property_fn_eval.rs'); old=(root/path).read_text(); s=old
s=s.replace('Triple(Box<[Self; 3]>),','Triple([purrdf_lex::walk::Nested<Self>; 3]),',1)
a=s.index('/// Dropping an argument'); b=s.index('/// The per-node compilation',a)
s=s[:a]+'''// Vacant child edges carry ancestor links; destruction never allocates.
impl purrdf_lex::walk::Dismantle for Arg {
    fn dismantle(node: Box<Self>) { purrdf_lex::walk::dismantle_tree(node); }
}
impl purrdf_lex::walk::DismantleTree for Arg {
    fn next_child(&mut self) -> Option<&mut purrdf_lex::walk::Nested<Self>> {
        match self { Self::Triple(parts) => parts.iter_mut().find(|part| !part.is_taken()), _ => None }
    }
    fn last_child(&mut self) -> Option<&mut purrdf_lex::walk::Nested<Self>> {
        match self { Self::Triple(parts) => parts.iter_mut().rev().find(|part| part.is_taken()), _ => None }
    }
}

'''+s[b:]
s=s.replace('    unobserved: Vec<bool>,\n}', '''    unobserved: Vec<bool>,
    // Payloads die before their original term and buffer grants.
    _term_owners: Vec<crate::WorkspaceAllocation>,
    _frame: crate::workspace::LexicalFrame,
}''',1)
# All Memory errors pass through a single static conversion; the account retains
# the original typed provider failure for terminal ingress selection.
helper='''
fn call_storage(error: purrdf_lex::allocation::StorageError) -> EvalError {
    match error {
        purrdf_lex::allocation::StorageError::SizeOverflow => EvalError::WorkspaceBoundOverflow,
        purrdf_lex::allocation::StorageError::AllocationFailed => EvalError::AllocationFailed { construct: "property-function producer" },
        purrdf_lex::allocation::StorageError::AdmissionFailed => EvalError::WorkspaceStopped,
        purrdf_lex::allocation::StorageError::FormattingFailed => EvalError::UnstableNativeDiagnostic,
    }
}
fn call_push<T,S: purrdf_lex::allocation::Admission + ?Sized>(values: &mut Vec<T>, value: T,
    memory: &mut purrdf_lex::allocation::Memory<'_,S>) -> Result<(),EvalError> {
    memory.push(values,value).map_err(call_storage)
}
fn constant_native<S: purrdf_lex::allocation::Admission + ?Sized>(term: &TermPattern,
    workspace: &crate::WorkspaceCapability, owners: &mut Vec<crate::WorkspaceAllocation>,
    memory: &mut purrdf_lex::allocation::Memory<'_, S>) -> Result<TermValue, EvalError> {
    memory.reserve(owners, owners.len().checked_add(1).ok_or(EvalError::WorkspaceBoundOverflow)?).map_err(call_storage)?;
    let term = crate::convert::ground_term_pattern_to_workspace_value(term, "a property-function call", workspace)?;
    let (value, owner) = term.into_parts(); owners.push(owner); Ok(value)
}
fn named_constant_native<S: purrdf_lex::allocation::Admission + ?Sized>(node: &NamedNode,
    workspace: &crate::WorkspaceCapability, owners: &mut Vec<crate::WorkspaceAllocation>,
    memory: &mut purrdf_lex::allocation::Memory<'_, S>) -> Result<TermValue, EvalError> {
    memory.reserve(owners, owners.len().checked_add(1).ok_or(EvalError::WorkspaceBoundOverflow)?).map_err(call_storage)?;
    let term = crate::convert::named_node_to_workspace_value(node, workspace)?;
    let (value, owner) = term.into_parts(); owners.push(owner); Ok(value)
}
'''
a=s.index('    fn compile(call:'); b=s.index('    /// The number of distinct',a)
s=s[:a]+'''    fn compile(call: &PropertyFunctionCall, input: &VarSchema) -> Result<Self, EvalError> {
        Self::compile_admitted(call, input, &crate::WorkspaceCapability::resident())
    }
    fn compile_admitted(call: &PropertyFunctionCall, input: &VarSchema,
        workspace: &crate::WorkspaceCapability) -> Result<Self, EvalError> {
        let mut frame = crate::workspace::LexicalFrame::new(workspace);
        let mut memory = purrdf_lex::allocation::Memory::new(&mut frame);
        let mut slots = DetHashMap::default();
        let mut schema = input.clone();
        let mut slot_cols = Vec::new();
        let mut slot_seed = Vec::new();
        let mut args = Vec::new();
        let mut owners = Vec::new();
        for term in call.subject_args.iter().chain(&call.object_args) {
            let arg = compile_arg_native(term, &mut slots, &mut schema,
                &mut slot_cols, &mut slot_seed, input, workspace, &mut owners, &mut memory)?;
            call_push(&mut args, arg, &mut memory)?;
        }
        let mut bound_cols = Vec::new();
        for (slot, col) in slot_cols.iter().enumerate() {
            if let Some(col) = col { call_push(&mut bound_cols, (slot, *col), &mut memory)?; }
        }
        let ceiling_is_offerable = args_are_admission_transparent_native(&args, slot_cols.len(), &mut memory)?;
        let unobserved = unobserved_positions_native(&args, &slot_cols, &slot_seed, &mut memory)?;
        let table_bytes = purrdf_core::hash::hash_table_allocation_bound::<(Variable,usize)>(slots.capacity())
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        drop(slots);
        memory.release_bytes(table_bytes).map_err(call_storage)?;
        drop(memory);
        let schema = schema.shared_admitted(workspace)?;
        Ok(Self { args, subject_len: call.subject_args.len(), slot_cols, schema,
            bound_cols, slot_seed, ceiling_is_offerable, unobserved, _term_owners: owners, _frame: frame })
    }

'''+s[b:]
a=s.index('fn args_are_admission_transparent('); b=s.index('\n/// Which flattened',a); block=s[a:b]
block=block.replace('fn args_are_admission_transparent(args: &[Arg], slot_count: usize) -> bool {','''fn args_are_admission_transparent_native<S: purrdf_lex::allocation::Admission + ?Sized>(
    args: &[Arg], slot_count: usize, memory: &mut purrdf_lex::allocation::Memory<'_, S>
) -> Result<bool, EvalError> {''').replace('    let mut seen = vec![false; slot_count];\n    args.iter().all(|arg| walk(arg, &mut seen))','''    let mut seen = Vec::new();
    for _ in 0..slot_count { call_push(&mut seen, false, memory)?; }
    let transparent = args.iter().all(|arg| walk(arg, &mut seen));
    memory.release_vec(seen).map_err(call_storage)?;
    Ok(transparent)''')
s=s[:a]+block+s[b:]
a=s.index('fn unobserved_positions('); b=s.index('\n/// Compile one argument',a); block=s[a:b]
block=block.replace('fn unobserved_positions(', 'fn unobserved_positions_native<S: purrdf_lex::allocation::Admission + ?Sized>(').replace(') -> Vec<bool> {', '''    memory: &mut purrdf_lex::allocation::Memory<'_, S>,
) -> Result<Vec<bool>, EvalError> {''')
block=block.replace('    let mut occurrences = vec![0_usize; slot_cols.len()];\n    let mut pending: Vec<&Arg> = args.iter().collect();', '''    let mut occurrences = Vec::new();
    for _ in 0..slot_cols.len() { call_push(&mut occurrences, 0_usize, memory)?; }
    let mut pending: Vec<&Arg> = Vec::new();
    for arg in args { call_push(&mut pending, arg, memory)?; }''').replace('Arg::Triple(parts) => pending.extend(parts.iter()),', 'Arg::Triple(parts) => { for part in parts { call_push(&mut pending, &**part, memory)?; } },')
at=block.index('    args.iter()'); block=block[:at]+'''    let mut output = Vec::new();
    for arg in args {
        let unobserved = match arg {
            Arg::Slot(slot) => slot_cols[*slot].is_none() && slot_seed[*slot].is_none() && occurrences[*slot] == 1,
            Arg::Constant(_) | Arg::Triple(_) => false,
        };
        call_push(&mut output, unobserved, memory)?;
    }
    memory.release_vec(pending).map_err(call_storage)?;
    memory.release_vec(occurrences).map_err(call_storage)?;
    Ok(output)
}
'''
s=s[:a]+block+s[b:]
a=s.index('fn compile_arg('); b=s.index('\n/// The slot of',a); block=s[a:b]
block=block.replace('fn compile_arg(', 'fn compile_arg_native<S: purrdf_lex::allocation::Admission + ?Sized>(').replace(') -> Result<Arg, EvalError> {', '''    workspace: &crate::WorkspaceCapability,
    owners: &mut Vec<crate::WorkspaceAllocation>,
    memory: &mut purrdf_lex::allocation::Memory<'_, S>,
) -> Result<Arg, EvalError> {''')
block=block.replace('    let mut pending = vec![Step::Compile(term)];','    let mut pending = Vec::new();\n    call_push(&mut pending, Step::Compile(term), memory)?;')
block=block.replace('crate::convert::ground_term_pattern_to_value(\n                    term,\n                    "a property-function call",\n                )?', 'constant_native(term, workspace, owners, memory)?')
block=block.replace('slot_for(', 'slot_for_native(').replace('crate::bgp::blank_var(blank.as_str())','crate::bgp::blank_var_admitted(blank.as_str(), workspace)?')
block=block.replace('                input,\n            ))','                input, workspace, memory,\n+            )?)').replace('                            input,\n                        ))','                            input, workspace, memory,\n+                        )?)')
block=block.replace('if triple_is_ground(triple) {','if triple_is_ground_native(triple, memory)? {').replace('crate::convert::ground_triple_pattern_to_value(\n                        triple,\n                        "a property-function call",\n                    )?', 'constant_native(term, workspace, owners, memory)?').replace('crate::convert::named_node_to_value(node)', 'named_constant_native(node, workspace, owners, memory)?')
block=block.replace('pending.push(Step::Assemble(predicate));','call_push(&mut pending, Step::Assemble(predicate), memory)?;').replace('pending.push(Step::Compile(&triple.object));','call_push(&mut pending, Step::Compile(&triple.object), memory)?;').replace('pending.push(Step::Compile(&triple.subject));','call_push(&mut pending, Step::Compile(&triple.subject), memory)?;')
block=block.replace('Arg::Triple(Box::new([subject, predicate, object]))', '''Arg::Triple([
                    purrdf_lex::walk::Nested::try_new(subject, memory).map_err(call_storage)?,
                    purrdf_lex::walk::Nested::try_new(predicate, memory).map_err(call_storage)?,
                    purrdf_lex::walk::Nested::try_new(object, memory).map_err(call_storage)?,
                ])''').replace('        built.push(arg);','        call_push(&mut built, arg, memory)?;')
block=block.replace('    Ok(built\n        .pop()\n        .expect("the position compiles to exactly one argument"))', '''    let output = built.pop().expect("the position compiles to exactly one argument");
    memory.release_vec(built).map_err(call_storage)?;
    memory.release_vec(pending).map_err(call_storage)?;
    Ok(output)''')
s=s[:a]+block+s[b:]
a=s.index('fn slot_for('); b=s.index('\n/// Whether a quoted',a); original_slot=s[a:b]; block=original_slot
block=block.replace('fn slot_for(', 'fn slot_for_native<S: purrdf_lex::allocation::Admission + ?Sized>(').replace(') -> usize {','''    workspace: &crate::WorkspaceCapability,
    memory: &mut purrdf_lex::allocation::Memory<'_, S>,
) -> Result<usize, EvalError> {''').replace('return slot;', 'return Ok(slot);')
block=block.replace('    slot_cols.push(projected.then(|| schema.push(variable.clone())));\n    slot_seed.push(projected.then(|| input.index_of(&variable)).flatten());\n    slots.insert(variable, slot);\n    slot', '''    let col = if projected { Some(schema.push_admitted(variable.clone(), workspace)?) } else { None };
    call_push(slot_cols, col, memory)?;
    call_push(slot_seed, projected.then(|| input.index_of(&variable)).flatten(), memory)?;
    let required = slots.len().checked_add(1).ok_or(EvalError::WorkspaceBoundOverflow)?;
    purrdf_core::hash::reserve_map_with_memory(slots, required, memory).map_err(call_storage)?;
    slots.insert(variable, slot);
    Ok(slot)''')
s=s[:a]+block+s[b:]
a=s.index('fn triple_is_ground('); b=s.index('\n// ---------------------------------------------------------------------------\n// The driver',a); block=s[a:b]
block=block.replace('fn triple_is_ground(triple: &TriplePattern) -> bool {','''fn triple_is_ground_native<S: purrdf_lex::allocation::Admission + ?Sized>(
    triple: &TriplePattern, memory: &mut purrdf_lex::allocation::Memory<'_, S>
) -> Result<bool, EvalError> {''').replace('let mut pending = vec![triple];','let mut pending = Vec::new();\n    call_push(&mut pending, triple, memory)?;')
block=block.replace('            return false;', '            memory.release_vec(pending).map_err(call_storage)?;\n            return Ok(false);').replace('TermPattern::Variable(_) | TermPattern::BlankNode(_) => return false,', 'TermPattern::Variable(_) | TermPattern::BlankNode(_) => { memory.release_vec(pending).map_err(call_storage)?; return Ok(false); },').replace('TermPattern::Triple(inner) => pending.push(inner),','TermPattern::Triple(inner) => call_push(&mut pending, &**inner, memory)?,').replace('    true\n}', '    memory.release_vec(pending).map_err(call_storage)?;\n    Ok(true)\n}')
s=s[:a]+block+s[b:]
at=s.index('\n// ---------------------------------------------------------------------------\n// The driver'); s=s[:at]+helper+s[at:]
(stage/'native-call-runtime-postimage.rs').write_text(s)
(stage/'native-call-slot-resident-reference.rs').write_text(original_slot)
