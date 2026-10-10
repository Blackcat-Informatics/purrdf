# Why not Rust: Ignored Stage-only native ownership text assembly never edits shipping source.
D='crates/sparql-eval/src/deferred_exists.rs'
s=read(D)
s=s.replace('use crate::DetHashSet;','use crate::{AdmittedMap,AdmittedVec,WorkspaceCapability};\nuse crate::workspace::SharedWorkspace;\nuse crate::solution::VarSchema;\nuse crate::error::EvalError;')
s=s.replace('Arc<','SharedWorkspace<').replace('DetHashSet<Variable>','VarSchema')
s=s.replace('merged: Option<SharedWorkspace<SubstitutionRow>>','merged: Option<SubstitutionRow>')
s=s.replace('crate::DetHashMap<usize, SharedWorkspace<ExistsSite>>','AdmittedMap<usize,SharedWorkspace<ExistsSite>>')
s=s.replace('crate::DetHashMap<usize, SharedWorkspace<LateralSite>>','AdmittedMap<usize,SharedWorkspace<LateralSite>>')
s=s.replace('crate::DetHashMap<usize, SharedWorkspace<Self>>','AdmittedMap<usize,SharedWorkspace<Self>>')
s=s.replace('crate::DetHashMap<usize, Deferred>','AdmittedMap<usize,Deferred>')
s=s.replace('crate::DetHashMap<usize, SharedWorkspace<NestedSites>>','AdmittedMap<usize,SharedWorkspace<NestedSites>>')
s=s.replace('pub(crate) schema: Vec<Variable>','pub(crate) schema: VarSchema')
s=s.replace('pub(crate) service_uses: Vec<Variable>','pub(crate) service_uses: AdmittedVec<Variable>')
s=s.replace('use std::sync::{Arc, OnceLock};','use std::sync::OnceLock;')
# Each cut body retains the original single tree admission after all scratch dies.
at=s.index('    sites: OnceLock<SharedWorkspace<NestedSites>>,',s.index('pub(crate) struct LateralSite'))
s=s[:at]+s[at:].replace('    sites: OnceLock<SharedWorkspace<NestedSites>>,','    sites: OnceLock<SharedWorkspace<NestedSites>>,\n    _body_owner: SharedWorkspace<std::sync::Mutex<crate::workspace::LexicalFrame>>,',1)
post[D]=s
env=(STAGE/'correlated-environment-owner.rs').read_text()
for name in ['then','layers','restrict','merge','with_row']:
    replace(D,name,function(env,name))
for name in ['placeholder','lateral_placeholder']:
    old=function(read(D),name);header=old[:old.index('{')]
    args='memory' if name=='placeholder' else 'schema,memory'
    replace(D,name,'#[cfg(test)]\n'+header+'{\n    let mut frame=crate::workspace::LexicalFrame::new(&WorkspaceCapability::resident());\n    '+name+'_with_memory('+args.replace('memory','&mut purrdf_lex::allocation::Memory::new(&mut frame)')+').expect("resident placeholder allocation failed")\n}')
s=read(D)+'\n'+function(env,'placeholder_with_memory')+'\n'+function(env,'lateral_placeholder_with_memory')+'\n'
post[D]=s
sites=(STAGE/'correlated-sites-owner.rs').read_text().replace('body_owner:owner.clone()','_body_owner:owner.clone()')
sites=sites.replace('let mut built=AdmittedVec::with_capacity(pieces.len(),workspace)?;','let mut built:AdmittedVec<Option<SharedWorkspace<LateralSite>>>=AdmittedVec::with_capacity(pieces.len(),workspace)?;')
sites=sites.replace('let mut children=AdmittedMap::default();','let mut children:AdmittedMap<usize,SharedWorkspace<LateralSite>>=AdmittedMap::default();')
for name in ['nested_sites','prepare_sites','nested_bodies','visit_lateral_operands_mut','build_lateral_site','build_site','to_plan']:
    replace(D,name,function(sites,name))
post[D]=read(D)+'\n'+function(sites,'remember_failure')+'\n'
replace(D,'operand','''    fn operand(self,operand:&GraphPattern,workspace:&WorkspaceCapability)->Result<bool,EvalError> {
        Ok(!matches!(operand,GraphPattern::PropertyFunction(_)) && !is_lateral_placeholder(operand)
            && (self.no_variable_endpoint || !crate::service_endpoints::mentions_variable_endpoint_admitted(operand,workspace)?))
    }''')
# Original endpoint visibility traversal, with each actual native work array owned.
old=function(read(D),'endpoint_uses');body=body_of(old,'endpoint_uses')
body=body.replace('let mut scopes: Vec<&[Variable]> = Vec::new();','let mut scopes=AdmittedVec::new(workspace);')
body=body.replace('let mut scope_of: Vec<usize> = Vec::new();','let mut scope_of=AdmittedVec::new(workspace);')
body=body.replace('let mut uses = Vec::new();','let mut uses=AdmittedVec::new(workspace);')
body=body.replace('let mut pending = vec![Node::Pattern(body, usize::MAX)];','let mut pending=AdmittedVec::new(workspace);\n    pending.push(Node::Pattern(body,usize::MAX))?;')
for visit in ['crate::governor::soundness::visit_pattern_parts','crate::governor::soundness::visit_expression_parts']:
    for m in reversed(list(re.finditer(re.escape(visit)+r'\s*\(',body))):
        p=body.find('(',m.start(),m.end());e=balanced(body,p,'(',')'); part=body[p+1:e-1]
        for push in reversed(list(re.finditer(r'pending\.push\(',part))):
            p2=part.find('(',push.start());e2=balanced(part,p2,'(',')')
            part=part[:push.start()]+'if remember_failure('+part[push.start():e2]+',&mut failure) { return true; }'+part[e2:]
        body=body[:m.start()]+'let mut failure=None;\n                    '+visit+'('+part+');\n                    if let Some(error)=failure { return Err(error); }'+body[e+1:]
for m in reversed(list(re.finditer(r'(?:uses|scopes|scope_of|pending)\.push\(',body))):
    line=body[body.rfind('\n',0,m.start())+1:m.start()]
    if 'remember_failure' in line: continue
    p=body.find('(',m.start());e=balanced(body,p,'(',')');body=body[:e]+'?'+body[e:]
body=body.replace('    uses\n','    Ok(uses)\n')
body=body.replace('??','?')
replace(D,'endpoint_uses','fn endpoint_uses(body:&GraphPattern,workspace:&WorkspaceCapability)->Result<AdmittedVec<Variable>,EvalError> {'+body+'}')

S='crates/sparql-eval/src/service_endpoints.rs'
post[S]=read(S).replace('fn mentions_variable_endpoint_admitted(','pub(crate) fn mentions_variable_endpoint_admitted(',1)

# The same fork-safety walk retains its inline worklist and original priority.
R='crates/sparql-eval/src/parallel.rs'
old=function(read(R),'reaches_unsafe_builtin');body=body_of(old,'reaches_unsafe_builtin')
body=body.replace('let mut pending: purrdf_core::SmallVec<[Reach<\'_>; 16]> = purrdf_core::smallvec![root];',
'''let mut frame=crate::workspace::LexicalFrame::new(workspace);
    let mut memory=purrdf_lex::allocation::Memory::new(&mut frame);
    let mut pending=purrdf_lex::walk::WorkList::<Reach<'_>,16>::new();
    pending.try_push_admitted(root,&mut memory).map_err(|e|memory.admission_mut().storage_error(e,"correlated fork-safety worklist"))?;''')
body=body.replace('pending[first..].reverse();','pending.reverse_top(pending.len()-first);')
for visit in ['visit_pattern_parts','visit_expression_parts']:
    for m in reversed(list(re.finditer(r'\b'+visit+r'\s*\(',body))):
        p=body.find('(',m.start(),m.end());e=balanced(body,p,'(',')');part=body[p+1:e-1]
        for push in reversed(list(re.finditer(r'pending\.push\(',part))):
            p2=part.find('(',push.start());e2=balanced(part,p2,'(',')');args=part[p2+1:e2-1]
            part=part[:push.start()]+'''if let Err(error)=pending.try_push_admitted('''+args+''',&mut memory) {
                        failure=Some(memory.admission_mut().storage_error(error,"correlated fork-safety worklist"));return true;
                    }'''+part[e2:]
        body=body[:m.start()]+'let mut failure=None;\n                '+visit+'('+part+');\n                if let Some(error)=failure { return Err(error); }'+body[e+1:]
body=body.replace('None => pending.push(Reach::Pattern(body)),','None => pending.try_push_admitted(Reach::Pattern(body),&mut memory).map_err(|e|memory.admission_mut().storage_error(e,"correlated fork-safety worklist"))?,')
body=body.replace('return true;','return Ok(true);')
body=body.replace('storage_error(error,"correlated fork-safety worklist"));return Ok(true);','storage_error(error,"correlated fork-safety worklist"));return true;')
body=body.replace('    false\n','    Ok(false)\n')
body=body.replace('                    Ok(false)','                    false')
body=body.replace('=> return true,','=> return Ok(true),')
new='fn reaches_unsafe_builtin_admitted(root:Reach<\'_>,registries:SafetyRegistries<\'_>,verdict:ExistsVerdict<\'_>,workspace:&crate::WorkspaceCapability)->Result<bool,crate::EvalError> {'+body+'}'
replace(R,'reaches_unsafe_builtin','''fn reaches_unsafe_builtin(root:Reach<'_>,registries:SafetyRegistries<'_>,verdict:ExistsVerdict<'_>)->bool {
    reaches_unsafe_builtin_admitted(root,registries,verdict,&crate::WorkspaceCapability::resident()).expect("resident safety scan allocation failed")
}''')
post[R]=read(R)+'\n'+new+'''\n
pub(crate) fn is_parallel_safe_pattern_with_admitted(pattern:&GraphPattern,registries:SafetyRegistries<'_>,verdict:ExistsVerdict<'_>,workspace:&crate::WorkspaceCapability)->Result<bool,crate::EvalError> {
    reaches_unsafe_builtin_admitted(Reach::Pattern(pattern),registries,verdict,workspace).map(|unsafe_|!unsafe_)
}
'''
