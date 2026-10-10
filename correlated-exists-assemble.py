# Why not Rust: Ignored Stage-only patch assembly preserves shipping source untouched.
from pathlib import Path
import re
import difflib

WT = Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
STAGE = WT / '.stage/sparql-eval-complete-bounded-workspace'
OUT = STAGE / 'correlated-exists-owner-postimages'
OUT.mkdir(exist_ok=True)
base = {}
post = {}

def read(path):
    if path not in post:
        base[path] = (WT/path).read_text()
        post[path] = base[path]
    return post[path]

def balanced(s, at, left='{', right='}'):
    depth = 0
    i = at
    while i < len(s):
        if s.startswith('//', i):
            j = s.find('\n', i)
            i = len(s) if j < 0 else j
            continue
        if s.startswith('/*', i):
            j = s.find('*/', i+2)
            if j < 0: raise ValueError('comment')
            i = j+2
            continue
        if s[i] == '"':
            i += 1
            while i < len(s):
                if s[i] == '\\': i += 2
                elif s[i] == '"': i += 1; break
                else: i += 1
            continue
        if s[i] == "'" and re.match(r"'(?:\\.|[^'\\])'", s[i:]):
            i += len(re.match(r"'(?:\\.|[^'\\])'", s[i:])[0]); continue
        if s[i] == left: depth += 1
        if s[i] == right:
            depth -= 1
            if depth == 0: return i+1
        i += 1
    raise ValueError(('unbalanced', at, s[at:at+80]))

def span(s, name):
    m = re.search(r'(?m)^[ \t]*(?:(?:pub(?:\([^\n]*?\))?|const) )*fn '+re.escape(name)+r'\b', s)
    if not m: raise ValueError(('missing fn', name))
    a = s.find('{', m.end())
    return m.start(), a, balanced(s, a)

def function(s, name):
    a,b,c=span(s,name)
    return s[a:c]

def replace(path,name,new):
    s=read(path); a,b,c=span(s,name)
    post[path]=s[:a]+new+s[c:]

def body_of(s,name):
    a,b,c=span(s,name); return s[b+1:c-1]

def calls(body, names, memory='memory', fallible=True):
    # Calls are transformed from right to left; balanced arguments retain actual
    # closures and nested expression structure. Definitions live outside this body.
    for old,new in sorted(names.items(), key=lambda p:-len(p[0])):
        matches=list(re.finditer(r'\b'+re.escape(old)+r'\s*\(',body))
        for m in reversed(matches):
            line=body[body.rfind('\n',0,m.start())+1:m.start()]
            if line.lstrip().startswith('//'): continue
            p=body.find('(',m.start(),m.end()); end=balanced(body,p,'(',')')
            args=body[p+1:end-1].rstrip()
            if args.endswith(','): args=args[:-1].rstrip()
            add=(', ' if args else '')+memory
            q='?' if fallible else ''
            body=body[:m.start()]+new+'('+args+add+')'+q+body[end:]
    return body

def fallible_body(body):
    body=re.sub(r'\breturn\s*;', 'return Ok(());', body)
    # Return expressions may terminate a match arm with a comma rather than a
    # statement with a semicolon. Respect nested expression delimiters.
    for m in reversed(list(re.finditer(r'\breturn\s+(?!Ok\()',body))):
        start=m.end(); i=start; depth=0
        while i<len(body):
            ch=body[i]
            if ch in '([{': depth+=1
            elif ch in ')]}':
                if depth==0: break
                depth-=1
            if depth==0 and ch in ',;': break
            i+=1
        body=body[:start]+'Ok('+body[start:i].rstrip()+')'+body[i:]
    return body

P='crates/sparql-eval/src/expr.rs'
s=read(P)
a=s.index('#[derive(Clone, Default)]\npub(crate) struct SubstitutionRow')
b=s.index('/// Substitute outer-bound variables into a graph pattern',a)
s=s[:a]+(STAGE/'correlated-owner-support.rs').read_text().split('\n',2)[2]+'\n'+s[b:]
post[P]=s

# Original deferred bookkeeping retains native row and metadata owners.
DRAFT=(STAGE/'correlated-deferral-owner.rs').read_text()
for name in ['into_placeholders','defer','defer_lateral','expr_vars','expr_vars_of_body']:
    s=read(P); start=s.index("impl<'a> Deferral<'a>"); tail=s[start:]
    a,b,c=span(tail,name)
    post[P]=s[:start+a]+function(DRAFT,name)+s[start+c:]
s=read(P)
a=s.index('#[derive(Default)]\nstruct FreeVars')
b=s.index('/// One [`SubstitutionSourceMap`] entry:',a)
s=s[:a]+DRAFT[DRAFT.index('struct FreeVars'):]+ '\n'+s[b:]
s=s.replace('&Arc<crate::deferred_exists::ExistsSite>','&crate::workspace::SharedWorkspace<crate::deferred_exists::ExistsSite>')
post[P]=s
outer=(STAGE/'correlated-outer-row-owner.rs').read_text()
replace(P,'outer_bindings_for_substitution',function(outer,'outer_bindings_for_substitution'))
replace(P,'ground_term_from_term_value',function(outer,'ground_term_from_term_value'))
s=read(P)+'\n'+function(outer,'ground_term_from_term_value_with_memory')+'\n'
s=s.replace('pending.push(Step::Convert(value))?;','let baseline=memory.live_bytes();\n    pending.push(Step::Convert(value))?;')
s=s.replace('let TermValue::Iri(predicate)=p.as_ref() else { return Ok(None); };','''let TermValue::Iri(predicate)=p.as_ref() else {
                    drop(built); drop(pending);
                    let released=memory.live_bytes().checked_sub(baseline).ok_or(EvalError::WorkspaceBoundOverflow)?;
                    memory.release_bytes(released).map_err(|e|correlated_storage_error(memory,e))?;
                    return Ok(None);
                };''')
post[P]=s

# Thread exact native storage through the ORIGINAL exhaustive rewrite arms.
original=function(s,'substitute_pattern_impl')
body=body_of(s,'substitute_pattern_impl')
a=body.index('    // One level of a copy')
b=body.index('    #[cfg(test)]',a)
body=body[:a]+'    crate::stack::check("correlated substitution")?;\n'+body[b:]
a=body.index('        GraphPattern::Bgp { patterns } => {')
b=body.index('        // A property function',a)
body=body[:a]+'''        GraphPattern::Bgp { .. } | GraphPattern::Path { .. } => {
            let workspace = memory.admission_mut().workspace().clone();
            let vars = crate::eval::syntactic_schema_admitted(pattern, &workspace)?;
            let copy = pattern.clone_with_memory(memory)
                .map_err(|e| correlated_storage_error(memory,e))?;
            let leaf = boxed_and_mapped(copy,pattern,map,memory)?;
            join_leaf_with_values(leaf,&vars,&row.term,pattern,map,memory)?
        }
'''+body[b:]
# Replace complete modifier payload producers, retaining original freevar gates.
a=body.index('                    expression: expression\n                        .iter()')
b=body.index('\n                },\n                pattern,\n                map,',a)
body=body[:a]+'''                    expression: substitute_order_list::<PRESERVE>(expression,row,map,defer,memory)?,'''+body[b:]
a=body.index('                    aggregates: aggregates\n')
b=body.index('\n                },\n                pattern,\n                map,',a)
body=body[:a]+'''                    aggregates: substitute_aggregates::<PRESERVE>(aggregates,row,map,defer,memory)?,'''+body[b:]
a=body.index('            let declared: DetHashSet<_>')
b=body.index('            boxed_and_mapped(',a)
body=body[:a]+'''            let workspace = memory.admission_mut().workspace().clone();
            let outer = row.filtered(&workspace, |name| !policy.inputs.iter().any(|(input,_)| input == name))?;
            let right = substitute_pattern_impl::<PRESERVE>(right,&outer,map,defer,memory)?;
            let policy = policy.clone_box_with_memory(memory).map_err(|e| correlated_storage_error(memory,e))?;
'''+body[b:]
a=body.index('                    policy: Box::new(')
b=balanced(body,body.index('(',a),'(',')')
body=body[:a]+'                    policy'+body[b:]
body=body.replace('let mut arms_sub = Vec::with_capacity(arms.len());','''let mut arms_sub = Vec::new();
            memory.reserve(&mut arms_sub,arms.len()).map_err(|e| correlated_storage_error(memory,e))?;''')
# The original permanent-address remap happens BEFORE any later allocation.
body=body.replace('remap_moved(map, before, after);','''remap_moved(map,before,after,memory)?;
                memory.release_bytes(std::alloc::Layout::new::<GraphPattern>().size())
                    .map_err(|e| correlated_storage_error(memory,e))?;''')
body=body.replace('let mut vars = DetHashSet::default();\n                vars.insert(v.clone());','let vars = VarSchema::from_vars_admitted([v.clone()],memory.admission_mut().workspace())?;')
body=body.replace('let mut vars = DetHashSet::default();\n                    vars.insert(v);','let vars = VarSchema::from_vars_admitted([v],memory.admission_mut().workspace())?;')
body=body.replace('let mut free = FreeVars::default();','let mut free = FreeVars::new(memory.admission_mut().workspace());')
body=body.replace('variables: variables.clone(),','variables: memory.collect(variables.iter().cloned()).map_err(|e| correlated_storage_error(memory,e))?,')
body=body.replace('row.narrow_to(variables)','row.narrow_to(variables,memory.admission_mut().workspace())?')
body=body.replace('boxed_and_mapped(pattern.clone(), pattern, map)','boxed_and_mapped(pattern.clone_with_memory(memory).map_err(|e| correlated_storage_error(memory,e))?, pattern, map)')
body=body.replace('let mut bound = call.clone();','let mut bound = call.clone_with_memory(memory).map_err(|e| correlated_storage_error(memory,e))?;')
body=body.replace('crate::substitute::bind_call_arguments(&mut bound, &row.term, Some(&left_sub))','crate::substitute::bind_call_arguments_with_memory(&mut bound,&row.term,Some(&left_sub),memory).map_err(crate::substitute::GroundFailure::into_eval_error)?')
# Transform only the remaining old calls; explicitly authored new ones already carry memory.
for old in ['substitute_pattern_impl::<PRESERVE>', 'substitute_expr::<PRESERVE>', 'boxed_and_mapped','join_leaf_with_values','wrap_with_expr_term_only_values','bind_row_into_call','plant_mapped_driver','defer.expr_vars','defer.defer_lateral']:
    matches=list(re.finditer(re.escape(old)+r'\s*\(',body))
    for m in reversed(matches):
        p=body.find('(',m.start(),m.end()); e=balanced(body,p,'(',')'); args=body[p+1:e-1].rstrip().rstrip(',')
        if re.search(r',\s*memory\s*$',args): continue
        body=body[:m.start()]+old+'('+args+', memory)?'+body[e:]
# Returning Optional rewritten condition is genuinely fallible.
body=body.replace('.map(|e| substitute_expr::<PRESERVE>(e, row, map, defer, memory)?),','.map(|e| substitute_expr::<PRESERVE>(e,row,map,defer,memory)).transpose()?,')
body=body.replace('defer.eager_depth += 1;','defer.eager_depth = defer.eager_depth.checked_add(1).ok_or(EvalError::WorkspaceBoundOverflow)?;')
body=body.replace('let inner_sub = substitute_pattern_impl::<PRESERVE>(inner, row, map, defer, memory)?;\n            defer.eager_depth -= 1;','let inner_result = substitute_pattern_impl::<PRESERVE>(inner,row,map,defer,memory);\n            defer.eager_depth -= 1;\n            let inner_sub = inner_result?;')
# Every arm produces the unwrapped Box value; exactly one Result envelope.
body=body.replace('    match pattern {','    Ok(match pattern {',1)
k=body.rfind('    }'); body=body[:k]+'    })'+body[k+5:]
header=original[:original.index('{')].replace(') -> Box<GraphPattern>',",\n    memory: &mut CorrelatedMemory<'_>,\n) -> Result<Box<GraphPattern>,EvalError>")
header=header.replace("defer: &mut Deferral<'_>,\n,","defer: &mut Deferral<'_>,")
replace(P,'substitute_pattern_impl',header+'{'+body+'}')

# The raw exported doors remain only the resident test reference; actual callers
# take the corresponding ownership-carrying native doors below.
for name in ['substitute_pattern','substitute_pattern_deferring','substitute_pattern_tracked']:
    old=function(read(P),name)
    header=old[:old.index('{')]
    if name=='substitute_pattern':
        newbody='substitute_pattern_admitted(pattern,row,&crate::WorkspaceCapability::resident()).map(CorrelatedPattern::into_resident)'
    elif name=='substitute_pattern_deferring':
        newbody='substitute_pattern_deferring_admitted::<PRESERVE>(pattern,row,defer,&crate::WorkspaceCapability::resident()).map(CorrelatedPattern::into_resident)'
    else:
        newbody='substitute_pattern_tracked_admitted::<PRESERVE>(pattern,row,tracked,enclosing,defer,&crate::WorkspaceCapability::resident()).map(CorrelatedPattern::into_resident)'
    replace(P,name,'#[cfg(test)]\n'+header+'{ '+newbody+' }')
s=read(P)
a=s.index('/// The substitution walk itself.')
s=s[:a]+'''pub(crate) fn substitute_pattern_admitted(pattern:&GraphPattern,row:&SubstitutionRow,workspace:&crate::WorkspaceCapability)
    -> Result<CorrelatedPattern,EvalError> {
    substitute_pattern_deferring_admitted::<false>(pattern,row,&mut Deferral::eager(),workspace)
}
pub(crate) fn substitute_pattern_deferring_admitted<const PRESERVE:bool>(
    pattern:&GraphPattern,row:&SubstitutionRow,defer:&mut Deferral<'_>,workspace:&crate::WorkspaceCapability,
) -> Result<CorrelatedPattern,EvalError> {
    CorrelatedPattern::build(workspace,|memory| {
        let mut tracking=None;
        substitute_pattern_impl::<PRESERVE>(pattern,row,&mut tracking,defer,memory)
    })
}
pub(crate) fn substitute_pattern_tracked_admitted<const PRESERVE:bool>(
    pattern:&GraphPattern,row:&SubstitutionRow,tracked:&mut SubstitutionSourceMap,
    enclosing:Option<&SubstitutionSourceMap>,defer:&mut Deferral<'_>,workspace:&crate::WorkspaceCapability,
) -> Result<CorrelatedPattern,EvalError> {
    CorrelatedPattern::build(workspace,|memory| {
        let mut claimed=crate::AdmittedMap::default();
        let mut tracking=SubstitutionTracking { map:tracked,enclosing,claimed:&mut claimed };
        substitute_pattern_impl::<PRESERVE>(pattern,row,&mut Some(&mut tracking),defer,memory)
    })
}

'''+s[a:]
s=s.replace('pub(crate) type SubstitutionSourceMap = crate::DetHashMap<usize, SubstitutionSource>;','pub(crate) type SubstitutionSourceMap = crate::AdmittedMap<usize, SubstitutionSource>;')
s=s.replace("claimed: &'a mut DetHashSet<usize>","claimed: &'a mut crate::AdmittedMap<usize,()>")
post[P]=s

for name in ['substitute_expr','join_leaf_with_values','wrap_with_expr_term_only_values','bind_row_into_call','plant_mapped_driver']:
    f=(STAGE/('correlated-rewrite-expressions.rs' if name=='substitute_expr' else 'correlated-rewrite-values.rs')).read_text()
    replace(P,name,function(f,name))
s=read(P)
s+='\n'+''.join(function((STAGE/'correlated-rewrite-expressions.rs').read_text(),n)+'\n' for n in ['substitute_expression_list','substitute_order_list','substitute_aggregates'])
s+='\n'+function((STAGE/'correlated-rewrite-values.rs').read_text(),'join_selected_values')+'\n'
post[P]=s

for name in ['boxed_and_mapped','boxed_and_mapped_scaffolding','boxed_and_mapped_as']:
    old=function(read(P),name); body=body_of(old,name)
    body=body.replace('let boxed = Box::new(built);','let boxed = correlated_box(built,memory)?;')
    body=body.replace('let counts_rows = counts_rows && tracking.claimed.insert(real_source);','let counts_rows = counts_rows && tracking.claimed.insert_admitted(real_source,(),memory.admission_mut().workspace())?.is_none();')
    body=calls(body,{'tracking.map.insert':'tracking.map.insert_admitted'},'memory.admission_mut().workspace()')
    body=calls(body,{'boxed_and_mapped_as':'boxed_and_mapped_as'})
    body=body.replace('    boxed\n','    Ok(boxed)\n')
    if name!='boxed_and_mapped_as': body=body.replace(')?',')')
    header=old[:old.index('{')].replace(') -> Box<GraphPattern>',"    memory: &mut CorrelatedMemory<'_>,\n) -> Result<Box<GraphPattern>,EvalError>")
    replace(P,name,header+'{'+body+'}')
replace(P,'remap_moved','''fn remap_moved(map:&mut Option<&mut SubstitutionTracking<'_>>,before:usize,after:usize,memory:&mut CorrelatedMemory<'_>) -> Result<(),EvalError> {
    if let Some(tracking)=map.as_deref_mut() && let Some(entry)=tracking.map.remove(&before) {
        tracking.map.insert_admitted(after,entry,memory.admission_mut().workspace())?;
    }
    Ok(())
}''')

# The existing native algebra clone homes supply all payload-producing copies.
A='crates/sparql-algebra/src/owned.rs'
a=read(A)
a+='''
impl PropertyFunctionCall {
    /// Clone argument trees and IRI under the original native memory account.
    /// # Errors
    /// Returns physical layout, allocator or admission refusal.
    pub fn clone_with_memory<S: Admission + ?Sized>(&self,memory:&mut Memory<'_,S>) -> Result<Self,StorageError> {
        Ok(Self { iri:memory.string(&self.iri)?,
            subject_args:clone_vec_with_memory(&self.subject_args,memory,|term,memory| term.clone_with_memory(memory))?,
            object_args:clone_vec_with_memory(&self.object_args,memory,|term,memory| term.clone_with_memory(memory))? })
    }
}
impl AggregateExpression {
    /// Rebuild rewritten expressions without changing checked aggregate shape.
    /// The scalar-values array is copied by the original native clone home.
    /// # Errors
    /// Returns physical layout, allocator or admission refusal.
    pub fn rebuild_with_memory<S: Admission + ?Sized>(&self,args:Vec<Expression>,order_by:Vec<OrderExpression>,memory:&mut Memory<'_,S>) -> Result<Self,StorageError> {
        assert_eq!(args.len(),self.args.len(),"rewrite preserves checked argument count");
        assert_eq!(order_by.len(),self.order_by.len(),"rewrite preserves checked sort-key count");
        Ok(Self { function:self.function.clone(),args,
            scalarvals:self.clone_scalarvals_with_memory(memory)?,
            order_by,distinct:self.distinct })
    }
}
'''
post[A]=a
old=function(read(P),'substitute_expr')
replace(P,'substitute_expr',old.replace('Expression::FunctionCall(function.clone(),','Expression::FunctionCall(function.clone_with_memory(memory).map_err(|e| correlated_storage_error(memory,e))?,'))

for companion in ['correlated-deferred-assemble.py','correlated-enf-assemble.py',
                  'correlated-eval-assemble.py','correlated-callers-assemble.py',
                  'correlated-forks-assemble.py','correlated-shared-home-assemble.py',
                  'correlated-oracles-assemble.py']:
    exec((STAGE/companion).read_text(),globals())

patch=[]
for path,source in post.items():
    target=OUT/path
    target.parent.mkdir(parents=True,exist_ok=True)
    target.write_text(source)
    patch.extend(difflib.unified_diff(base[path].splitlines(keepends=True),source.splitlines(keepends=True),
                                    fromfile='a/'+path if base[path] else '/dev/null',tofile='b/'+path))
(STAGE/'correlated-exists-owner-draft.patch').write_text(''.join(patch))
print('Stage postimages and ordinary diff written:',len(post),'homes')
