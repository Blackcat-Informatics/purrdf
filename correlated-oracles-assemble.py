# Why not Rust: Ignored Stage-only text assembly preserves the original Rust reference tests and shipping source.
P='crates/sparql-eval/src/expr.rs'
s=read(P)

# Preserve every original test payload when changing only its resident carrier.
for m in reversed(list(re.finditer(r'\bSubstitutionRow\s*\{',s))):
    line=s[s.rfind('\n',0,m.start())+1:m.start()]
    if 'struct ' in line: continue
    p=s.find('{',m.start());end=balanced(s,p)
    contents=s[p+1:end-1]
    if not re.match(r'\s*expr:',contents): continue
    marker=re.search(r'(?m)^\s*term:\s*',contents)
    if not marker: raise ValueError('resident row term field')
    expr=contents[contents.index(':')+1:marker.start()].rstrip().rstrip(',')
    term=contents[marker.end():].rstrip().rstrip(',')
    s=s[:m.start()]+'SubstitutionRow::resident('+expr+','+term+')'+s[end:]

# Native and recursive deferral readers deliberately retain independent output
# containers. The recursive oracle remains the original unordered-set law.
a=s.index('mod walk_tests {')
b=balanced(s,s.index('{',a))
t=s[a:b]
t=t.replace('    use std::sync::Arc;\n','    use crate::workspace::SharedWorkspace;\n')
t=t.replace('    const EX: &str =', '''    #[derive(Default)]
    struct ReferenceFreeVars {
        direct: DetHashSet<Variable>,
        sites: Vec<SharedWorkspace<crate::deferred_exists::ExistsSite>>,
    }

    const EX: &str =''',1)
t=t.replace('out: &mut FreeVars','out: &mut ReferenceFreeVars')
t=t.replace('out.sites.push(Arc::clone(site))','out.sites.push(site.clone())')
t=t.replace('fn sorted(vars: &DetHashSet<Variable>)','fn sorted<\'a>(vars: impl IntoIterator<Item=&\'a Variable>)')
t=t.replace('vars.iter().map(|v| v.as_str().to_owned()).collect()','vars.into_iter().map(|v| v.as_str().to_owned()).collect()')
t=re.sub(r'sorted\(&(\w+)\)',r'sorted(\1.iter())',t)
t=t.replace('sorted(&ours.direct)','sorted(ours.direct.iter().map(|(variable,_)|variable))')
t=t.replace('sorted(&expected.direct)','sorted(expected.direct.iter())')
t=t.replace('sorted(&deferred.direct)','sorted(deferred.direct.iter().map(|(variable,_)|variable))')
t=t.replace('sorted(&direct.direct)','sorted(direct.direct.iter().map(|(variable,_)|variable))')
post[P]=s[:a]+t+s[b:]

# The original positive singleton producer uses one native schema builder.
# Publication freezes only after the full selected row layout is known.
old=function(read(P),'exists_positive_seeded')
old=old.replace('let mut seed_schema = VarSchema::default();','let mut seed_schema = crate::solution::SchemaBuilder::new(&ctx.growth);')
old=old.replace('seed_schema.push_admitted(variable.clone(), &ctx.growth)?;','seed_schema.push(variable.clone())?;')
old=old.replace('    let mut seed_rows = RowsBuilder::new(&ctx.growth);','    let seed_schema=seed_schema.finish()?;\n    let mut seed_rows = RowsBuilder::new(&ctx.growth);')
replace(P,'exists_positive_seeded',old)

old=function(read(P),'exists')
old=old.replace('return Err(EvalError::internal(','return Err(crate::NativeDiagnostic::error(crate::NativeDiagnosticKind::Internal,')
old=old.replace('left it",\n        ));','left it",\n            &ctx.growth,\n        ));')
replace(P,'exists',old)
replace(P,'site_for', '''    fn site_for(body:&GraphPattern)->SharedWorkspace<crate::deferred_exists::ExistsSite> {
        let workspace=crate::WorkspaceCapability::resident();
        let vars=super::pattern_all_vars_admitted(body,&workspace).expect("resident variable owner");
        SharedWorkspace::new_admitted(crate::deferred_exists::ExistsSite {
            prepared:SharedWorkspace::new_admitted(
                crate::eval::PreparedExists::build(body),&workspace).expect("resident prepared site"),
            vars,parallel_unsafe:false,
            service_uses:crate::AdmittedVec::new(&workspace),
            plan_map:SharedWorkspace::new_admitted(super::SubstitutionSourceMap::default(),&workspace)
                .expect("resident source-map control"),
        },&workspace).expect("resident site control")
    }''')
replace(P,'site_ids', '''    fn site_ids(sites:&[SharedWorkspace<crate::deferred_exists::ExistsSite>])->Vec<usize> {
        sites.iter().map(|site|std::ptr::from_ref(site.as_ref()) as usize).collect()
    }''')
s=read(P)
a=s.index('mod walk_tests {');b=balanced(s,s.index('{',a));t=s[a:b]
t=t.replace('let mut expected = FreeVars::default();','let mut expected = ReferenceFreeVars::default();')
for name in ['ours','deferred','direct']:
    t=t.replace('let mut '+name+' = FreeVars::default();','let mut '+name+' = FreeVars::new(&crate::WorkspaceCapability::resident());')
t=re.sub(r'\bsite_ids\(&(ours|expected)\)',r'site_ids(&\1.sites)',t)
for m in reversed(list(re.finditer(r'\b(?:eager|deferring)\.expr_vars\(',t))):
    p=t.find('(',m.start());end=balanced(t,p,'(',')')
    args=t[p+1:end-1]
    t=t[:m.start()]+'''{
                let mut frame=crate::workspace::LexicalFrame::new(&crate::WorkspaceCapability::resident());
                '''+t[m.start():p]+'('+args+''',&mut purrdf_lex::allocation::Memory::new(&mut frame))
                    .expect("resident native deferral reader");
            }'''+t[end:]
post[P]=s[:a]+t+s[b:]

# These raw helpers are reference/test doors only. Production now always uses
# the same admitted bodies and cannot enter the resident adapter.
N='crates/sparql-eval/src/enf.rs'
s=read(N)
for name in ['left_join_erasable','order_by_erasable','normalize','ledger_source_map','map_clone']:
    a,_,_=span(s,name)
    s=s[:a]+'#[cfg(test)]\n'+s[a:]
for name in ['normalize','ledger_source_map','map_clone']:
    target=name+'_with_memory' if name=='normalize' else name+'_admitted'
    s=s.replace('['+chr(96)+name+chr(96)+']','['+chr(96)+target+chr(96)+']')
post[N]=s

B='crates/sparql-eval/src/binop.rs'
s=read(B).replace(
    'for layer in slot.env.then(&current,&site.vars,&ctx.growth)?.layers(&ctx.growth)? {',
    'let layered=slot.env.then(&current,&site.vars,&ctx.growth)?;\n    for layer in layered.layers(&ctx.growth)? {')
post[B]=s

G='crates/sparql-eval/src/exists_admission_gate.rs'
post[G]=read(G).replace('assert_eq!(variable, expected_variable,','assert_eq!(variable.as_str(), expected_variable,')

# Preserve the native RDF failure classification while moving its original
# diagnostic owner; this does not render/copy a diagnostic.
R='crates/sparql-eval/src/protocol.rs'
s=read(R)
s=s.replace('            EvalError::Parse(_) => Self::QueryParse,',
'''            EvalError::Parse(_) => Self::QueryParse,
            EvalError::RetainedDiagnostic(diagnostic) =>
                Self::from_diagnostic_code(&diagnostic.diagnostic().code),''',1)
post[R]=s

# The actual original workspace test fixture supplies a typed, non-latching
# reservation. Do not duplicate that owner implementation for this batch.
W='crates/sparql-eval/src/workspace.rs'
s=read(W)
a=s.index('mod owned_row_regressions {')
b=balanced(s,s.index('{',a))
s=s[:b-1]+(STAGE/'correlated-owner-fixtures.rs').read_text()+'\n'+s[b-1:]
post[W]=s
F='crates/sparql-eval/tests/correlated_owned_admission.rs'
base[F]=''
post[F]=(STAGE/'correlated-public-fixtures.rs').read_text()
