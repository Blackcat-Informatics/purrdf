from pathlib import Path
import difflib
stage=Path('.stage/sparql-eval-complete-bounded-workspace'); p=stage/'native-call-runtime-postimage.rs'; s=p.read_text()
s=s.replace('use purrdf_core::TermBox;\n','',1)
# Native slots use the shared admitted map, rather than pricing std's opaque table.
start=s.index('impl CallPlan {'); end=s.index('// The driver',start)
part=s[start:end].replace('let mut slots = DetHashMap::default();','let mut slots = crate::AdmittedMap::default();',1)
a=part.index('        let table_bytes ='); b=part.index('        drop(memory);',a)
part=part[:a]+'        drop(slots);\n'+part[b:]
part=part.replace('slots: &mut DetHashMap<Variable, usize>,','slots: &mut crate::AdmittedMap<Variable, usize>,',2)
a=part.index('    let required = slots'); b=part.index('    Ok(slot)',a)
part=part[:a]+'    slots.insert_admitted(variable, slot, workspace)?;\n'+part[b:]
# Existing resident fixtures still invoke these exact laws with their caller maps.
a=part.index('#[cfg(test)]\nfn compile_arg('); b=part.index('#[cfg(test)]\nfn triple_is_ground(',a)
part=part[:a]+'''#[cfg(test)]
fn compile_arg(term: &TermPattern, slots: &mut DetHashMap<Variable,usize>, schema: &mut VarSchema,
    slot_cols: &mut Vec<Option<usize>>, slot_seed: &mut Vec<Option<usize>>, input: &VarSchema) -> Result<Arg,EvalError> {
    let workspace = crate::WorkspaceCapability::resident();
    let mut frame = crate::workspace::LexicalFrame::new(&workspace);
    let mut memory = purrdf_lex::allocation::Memory::new(&mut frame);
    let old = std::alloc::Layout::array::<Option<usize>>(slot_cols.capacity()).map_err(|_|EvalError::WorkspaceBoundOverflow)?.size()
        .checked_add(std::alloc::Layout::array::<Option<usize>>(slot_seed.capacity()).map_err(|_|EvalError::WorkspaceBoundOverflow)?.size())
        .ok_or(EvalError::WorkspaceBoundOverflow)?;
    memory.add_bytes(old).map_err(call_storage)?;
    let mut native_slots = crate::AdmittedMap::default();
    for (variable,slot) in slots.iter() { native_slots.insert_admitted(variable.clone(),*slot,&workspace)?; }
    let mut owners = Vec::new();
    let result = compile_arg_native(term,&mut native_slots,schema,slot_cols,slot_seed,input,&workspace,&mut owners,&mut memory);
    slots.clear(); slots.extend(native_slots.iter().map(|(v,n)|(v.clone(),*n)));
    result
}
#[cfg(test)]
fn slot_for(variable: Variable, projected: bool, slots: &mut DetHashMap<Variable,usize>,
    schema: &mut VarSchema, slot_cols: &mut Vec<Option<usize>>, slot_seed: &mut Vec<Option<usize>>, input: &VarSchema) -> usize {
    let workspace = crate::WorkspaceCapability::resident();
    let mut frame = crate::workspace::LexicalFrame::new(&workspace);
    let mut memory = purrdf_lex::allocation::Memory::new(&mut frame);
    let old = slot_cols.capacity() * size_of::<Option<usize>>() + slot_seed.capacity() * size_of::<Option<usize>>();
    memory.add_bytes(old).expect("resident admission");
    let mut native_slots = crate::AdmittedMap::default();
    for (variable,slot) in slots.iter() { native_slots.insert_admitted(variable.clone(),*slot,&workspace).expect("resident slot table"); }
    let result = slot_for_native(variable,projected,&mut native_slots,schema,slot_cols,slot_seed,input,&workspace,&mut memory).expect("resident slot allocation");
    slots.clear(); slots.extend(native_slots.iter().map(|(v,n)|(v.clone(),*n)));
    result
}
'''+part[b:]
s=s[:start]+part+s[end:]
s=s.replace('return Err(EvalError::relation_incomplete(&call.iri, reason));','return Err(EvalError::relation_incomplete_admitted(&call.iri, reason, &ctx.growth));',1)
p.write_text(s)
ep=Path('crates/sparql-eval/src/error.rs'); e=ep.read_text(); a=e.index('    RelationIncomplete {'); b=e.index('\n    },',a)
part=e[a:b].replace('iri: String','iri: purrdf_lex::allocation::SharedText').replace('reason: String','reason: purrdf_lex::allocation::SharedText'); e=e[:a]+part+e[b:]
a=e.index('    pub(crate) fn relation_incomplete('); b=e.index('\n}\n',a)
e=e[:a]+'''    pub(crate) fn relation_incomplete(iri: impl AsRef<str>, reason: impl AsRef<str>) -> Self {
        Self::relation_incomplete_admitted(iri.as_ref(),reason.as_ref(),&crate::WorkspaceCapability::resident())
    }
    pub(crate) fn relation_incomplete_admitted(iri: &str, reason: &str, workspace: &crate::WorkspaceCapability) -> Self {
        let result = (|| {
            let iri = workspace.authored_text(&iri)?;
            let reason = workspace.authored_text(&reason)?;
            Ok::<_,Self>(Self::RelationIncomplete { iri, reason })
        })();
        result.unwrap_or_else(|error| error)
    }
'''+e[b:]
(stage/'native-call-runtime-error-postimage.rs').write_text(e)
