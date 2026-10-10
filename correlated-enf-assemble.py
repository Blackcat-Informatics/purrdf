# Why not Rust: Ignored Stage-only text assembly preserves the original production algorithm.
F='crates/sparql-eval/src/enf.rs'
s=read(F)
old=function(s,'normalize');body=body_of(old,'normalize')
body=body.replace('let mut steps: Vec<Step<\'_>> = vec![Step::Enter(pattern)];','let workspace=memory.admission_mut().workspace().clone();\n    let mut steps=crate::AdmittedVec::new(&workspace);\n    steps.push(Step::Enter(pattern))?;')
body=body.replace('let mut forms: Vec<Enf> = Vec::new();','let mut forms=crate::AdmittedVec::new(&workspace);')
body=calls(body,{'left_join_erasable':'left_join_erasable_admitted','order_by_erasable':'order_by_erasable_admitted','soundness::pattern_can_hard_error':'soundness::pattern_can_hard_error_admitted'},'&workspace')
body=calls(body,{'copied':'copied_with_memory'})
body=body.replace('inner: Child::new(p),','inner: crate::expr::correlated_box(p,memory)?.into(),')
body=body.replace('variables: variables.to_vec(),','variables: memory.collect(variables.iter().cloned()).map_err(|e|crate::expr::correlated_storage_error(memory,e))?,')
a=body.index('                let kept: Vec<GraphPattern> = forms')
b=body.index('                forms.push(match Chain::try_from(kept)',a)
body=body[:a]+'''                let count=forms[first..].iter().filter(|form|matches!(form,Enf::Pattern(_))).count();
                let mut kept=Vec::new();
                memory.reserve(&mut kept,count).map_err(|e|crate::expr::correlated_storage_error(memory,e))?;
                for form in forms.drain_from(first) { if let Enf::Pattern(pattern)=form { kept.push(pattern); } }
'''+body[b:]
body=body.replace('Err(mut kept) => kept.pop().map_or(Enf::FoldedEmpty, Enf::Pattern),','''Err(mut kept)=>{
                        let form=kept.pop().map_or(Enf::FoldedEmpty,Enf::Pattern);
                        memory.release_vec(kept).map_err(|e|crate::expr::correlated_storage_error(memory,e))?;
                        form
                    },''')
for push in ['steps.push','forms.push']:
    for m in reversed(list(re.finditer(re.escape(push)+r'\(',body))):
        p=body.find('(',m.start());e=balanced(body,p,'(',')')
        if body[e:e+1]!='?':body=body[:e]+'?'+body[e:]
body=body.replace('(0, _) => steps.push(Step::Enter(inner))?,','(0, _) => { steps.push(Step::Enter(inner))?; },')
body=body.replace('(_, _) => forms.push(Enf::Pattern(copied_with_memory(pattern, memory)?))?,','(_, _) => { forms.push(Enf::Pattern(copied_with_memory(pattern, memory)?))?; },')
body=body.replace('other => forms.push(Enf::Pattern(copied_with_memory(other, memory)?))?,','other => { forms.push(Enf::Pattern(copied_with_memory(other, memory)?))?; },')
a=body.rfind('    forms\n');body=body[:a]+'    Ok('+body[a:].strip()+')\n'
new='pub(crate) fn normalize_with_memory(pattern:&GraphPattern,memory:&mut crate::expr::CorrelatedMemory<\'_>)->Result<Enf,crate::EvalError> {'+body+'}'
replace(F,'normalize','''pub(crate) fn normalize(pattern:&GraphPattern)->Enf {
    let mut frame=crate::workspace::LexicalFrame::new(&crate::WorkspaceCapability::resident());
    normalize_with_memory(pattern,&mut purrdf_lex::allocation::Memory::new(&mut frame)).expect("resident ENF allocation failed")
}''')
post[F]=read(F)+'\n'+new+'\n'
replace(F,'copied','''fn copied_with_memory(pattern:&GraphPattern,memory:&mut crate::expr::CorrelatedMemory<'_>)->Result<GraphPattern,crate::EvalError> {
    #[cfg(test)]
    crate::op_count::count_copied(purrdf_sparql_algebra::NodeRef::Pattern(pattern));
    pattern.clone_with_memory(memory).map_err(|e|crate::expr::correlated_storage_error(memory,e))
}''')
post[F]=read(F)+'''
fn left_join_erasable_admitted(right:&GraphPattern,expression:Option<&purrdf_sparql_algebra::Expression>,workspace:&crate::WorkspaceCapability)->Result<bool,crate::EvalError> {
    let right_clean=!soundness::pattern_can_hard_error_admitted(right,workspace)?;
    let condition_clean=match expression { None=>true,Some(expr)=>!soundness::expr_can_hard_error_admitted(expr,workspace)? };
    Ok(right_clean&&condition_clean)
}
fn order_by_erasable_admitted(expression:&[OrderExpression],workspace:&crate::WorkspaceCapability)->Result<bool,crate::EvalError> {
    for order in expression { if soundness::expr_can_hard_error_admitted(order.expression(),workspace)? { return Ok(false); } }
    Ok(true)
}
'''
# Erasure gates remain one implementation for resident references too.
replace(F,'left_join_erasable','''fn left_join_erasable(right:&GraphPattern,expression:Option<&purrdf_sparql_algebra::Expression>)->bool {
    left_join_erasable_admitted(right,expression,&crate::WorkspaceCapability::resident()).expect("resident ENF gate failed")
}''')
replace(F,'order_by_erasable','''fn order_by_erasable(expression:&[OrderExpression])->bool {
    order_by_erasable_admitted(expression,&crate::WorkspaceCapability::resident()).expect("resident ENF gate failed")
}''')
old=function(read(F),'map_spine');body=body_of(old,'map_spine')
body=calls(body,{'left_join_erasable':'left_join_erasable_admitted','order_by_erasable':'order_by_erasable_admitted'},'workspace')
body=body.replace('=> return,','=> return Ok(()),')
body=body.replace('return map_clone(original, normalized, map)','return map_clone_admitted(original,normalized,map,workspace)')
replace(F,'map_spine','''fn map_spine_admitted(original:&GraphPattern,normalized:&GraphPattern,map:&mut SubstitutionSourceMap,workspace:&crate::WorkspaceCapability)->Result<(),crate::EvalError> {'''+body+'}')
replace(F,'ledger_source_map','''pub(crate) fn ledger_source_map(original:&GraphPattern,normalized:&GraphPattern)->SubstitutionSourceMap {
    ledger_source_map_admitted(original,normalized,&crate::WorkspaceCapability::resident()).expect("resident source map allocation failed")
}''')
replace(F,'map_clone','''pub(crate) fn map_clone(original:&GraphPattern,copy:&GraphPattern,map:&mut SubstitutionSourceMap) {
    map_clone_admitted(original,copy,map,&crate::WorkspaceCapability::resident()).expect("resident clone mapping allocation failed")
}''')
post[F]=read(F)+'''
pub(crate) fn ledger_source_map_admitted(original:&GraphPattern,normalized:&GraphPattern,workspace:&crate::WorkspaceCapability)->Result<SubstitutionSourceMap,crate::EvalError> {
    let mut map=SubstitutionSourceMap::default();
    map_spine_admitted(original,normalized,&mut map,workspace)?;
    Ok(map)
}
pub(crate) fn map_clone_admitted(original:&GraphPattern,copy:&GraphPattern,map:&mut SubstitutionSourceMap,workspace:&crate::WorkspaceCapability)->Result<(),crate::EvalError> {
    let mut pending=crate::AdmittedVec::new(workspace);
    pending.push((original,copy))?;
    while let Some((original,copy))=pending.pop() {
        map.insert_admitted(std::ptr::from_ref(copy) as usize,SubstitutionSource { source:std::ptr::from_ref(original) as usize,counts_rows:true },workspace)?;
        let mut originals=crate::AdmittedVec::new(workspace);
        soundness::visit_classified_children_admitted(original,workspace,&mut |child,_| { originals.push(child)?;Ok(false) })?;
        let mut copies=crate::AdmittedVec::new(workspace);
        soundness::visit_classified_children_admitted(copy,workspace,&mut |child,_| { copies.push(child)?;Ok(false) })?;
        assert_eq!(originals.len(),copies.len(),"structural clone children pair 1:1");
        for (original,copy) in originals.iter().zip(copies.iter()).rev() { pending.push((*original,*copy))?; }
    }
    Ok(())
}
'''
