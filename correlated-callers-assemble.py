# Why not Rust: Ignored Stage-only diff assembly preserves shipping production sources.
P='crates/sparql-eval/src/expr.rs'
s=read(P)
# Preserve the original strategy override and short-circuit law, but admit the
# actual native contextual proof before selecting the probe.
old=function(s,'exists_use_probe')
body=body_of(old,'exists_use_probe')
body=body.replace('return forced == ForcedExistsStrategy::Probe;','return Ok(forced == ForcedExistsStrategy::Probe);')
body=body.replace('    (!correlated && !stateful)\n        || crate::governor::soundness::probe_admissible(normalized, analysis, outer_schema)',
'''    if !correlated && !stateful { return Ok(true); }
    crate::governor::soundness::probe_admissible_admitted(normalized,analysis,outer_schema,workspace)''')
header=old[:old.index('{')].replace(') -> bool',', workspace:&crate::WorkspaceCapability,\n) -> Result<bool,EvalError>')
header=header.replace('outer_schema: &VarSchema,\n,','outer_schema: &VarSchema,')
replace(P,'exists_use_probe',header+'{'+body+'}')
s=read(P).replace('if exists_use_probe(correlated, stateful, normalized, analysis, schema) {',
                  'if exists_use_probe(correlated,stateful,normalized,analysis,schema,&ctx.growth)? {')
a=s.index('        let outer_bound: DetHashSet<Variable> = schema')
b=s.index('        // A walk over the whole inner pattern',a)
s=s[:a]+'''        let outer_bound=VarSchema::from_vars_admitted(
            schema.vars().iter().enumerate().filter(|(i,_)|row[*i].is_some()).map(|(_,v)|v.clone()),
            &ctx.growth)?;
'''+s[b:]
s=s.replace('crate::governor::soundness::exists_row_collision(normalized, &outer_bound)',
            'crate::governor::soundness::exists_row_collision_admitted(normalized,&outer_bound,&ctx.growth)?')
s=s.replace('var.as_str().to_owned(),','ctx.growth.authored_text(var.as_str())?,',1)
s=s.replace('''return Err(EvalError::exists_scope_collision(
                ctx.growth.authored_text(var.as_str())?,
                intro.as_str(),
            ));''','''return Err(EvalError::ExistsScopeCollision {
                variable:ctx.growth.authored_text(var.as_str())?,
                intro:intro.as_str(),
            });''',1)
# The selected pushdown is priced before installing a ledger window: a failed
# plan cannot leave a map pushed on the context's evidence cursor.
start=s.index('fn exists_positive_seeded')
a=s.index('    let track_ledger =',start)
b=s.index('    let evaluated =',a)
part=s[a:b]
x=part.index('    let next_pushdown=')
y=part.index('    let previous_pushdown=',x)
preflight=part[x:y]
part=part[:x]+part[y:]
s=s[:a]+preflight+part+s[b:]
post[P]=s

# Keep the exact structured collision variant and its message. Native fields
# carry their intrinsic text grant; the resident helper remains caller-owned.
E='crates/sparql-eval/src/error.rs'
s=read(E)
a=s.index('    ExistsScopeCollision {')
b=s.index('    },',a)
s=s[:a]+s[a:b].replace('variable: String','variable: purrdf_lex::allocation::SharedText')+s[b:]
a=s.index('        /// Construct an [')
a=s.index('        /// Construct an [',s.index('pub(crate) fn composite_bound',a))
b=s.index("        pub(crate) fn exists_scope_collision(variable, intro: &'static str) -> Self::ExistsScopeCollision { .. };",a)
b+=len("        pub(crate) fn exists_scope_collision(variable, intro: &'static str) -> Self::ExistsScopeCollision { .. };")
s=s[:a]+s[b:]
post[E]=s
s=read(E)
s=s.replace('"{intro} ?{variable} inside EXISTS is already in scope on the row being \\\n                 filtered: the substitution semantics define no answer for a rebinding"',
'''\"{intro} ?{} inside EXISTS is already in scope on the row being \\
                 filtered: the substitution semantics define no answer for a rebinding\",variable.as_str()''')
post[E]=s

B='crates/sparql-eval/src/binop.rs'
s=read(B)
# Full correlated caller migration: all actual doors retain the copied tree.
names={'crate::expr::substitute_pattern':'crate::expr::substitute_pattern_admitted',
       'crate::expr::substitute_pattern_tracked::<DECLARED>':'crate::expr::substitute_pattern_tracked_admitted::<DECLARED>',
       'crate::expr::substitute_pattern_deferring::<DECLARED>':'crate::expr::substitute_pattern_deferring_admitted::<DECLARED>',
       'crate::expr::substitute_pattern_deferring::<false>':'crate::expr::substitute_pattern_deferring_admitted::<false>',
       'crate::deferred_exists::nested_sites':'crate::deferred_exists::nested_sites'}
for old,new in names.items():
    for m in reversed(list(re.finditer(r'\b'+re.escape(old)+r'\s*\(',s))):
        p=s.find('(',m.start());end=balanced(s,p,'(',')');args=s[p+1:end-1].rstrip().rstrip(',')
        if old.endswith('nested_sites'):
            s=s[:end]+'?'+s[end:]
        else:
            s=s[:m.start()]+new+'('+args+',&ctx.growth)'+s[end:]
s=s.replace('ctx.correlated_node_maps.last().map(Arc::as_ref)','ctx.correlated_node_maps.last().map(|map|map.as_ref())')
s=s.replace('deferral.into_placeholders()','deferral.into_placeholders(&ctx.growth)?')
s=s.replace('ctx.enter_substituted_exists(None, None);','ctx.enter_substituted_exists(None,None)?;')
s=s.replace('ctx.enter_substituted_exists(ledger_map, placeholders);','ctx.enter_substituted_exists(ledger_map,placeholders)?;')
s=s.replace('ctx.enter_substituted_exists(None, placeholders);','ctx.enter_substituted_exists(None,placeholders)?;')
s=s.replace('let mut copy: Option<Box<GraphPattern>> = None;','let mut copy: Option<crate::expr::CorrelatedPattern> = None;')
s=s.replace('Option<Arc<crate::deferred_exists::DeferredMap>>','Option<crate::workspace::SharedWorkspace<crate::deferred_exists::DeferredMap>>')
s=s.replace('slot.env.then(&current, &site.vars).layers()','slot.env.then(&current,&site.vars,&ctx.growth)?.layers(&ctx.growth)?')
s=calls(s,{'crate::deferred_exists::with_row':'crate::deferred_exists::with_row'},'&ctx.growth')
for m in reversed(list(re.finditer(r'ctx\.correlated_node_maps\.push\(',s))):
    p=s.find('(',m.start());end=balanced(s,p,'(',')');args=s[p+1:end-1]
    args=re.sub(r'Arc::clone\(&?(.*?)\)',r'\1.clone()',args)
    s=s[:m.start()]+'ctx.correlated_node_maps.push('+args+',&ctx.growth)?'+s[end:]
# The declared optional retry wrap moves the original owned root; there is no
# raw child extraction, and preflight occurs before installing the scope guard.
a=s.index('    let mut guard = ctx.enter_substituted_exists(ledger_map,placeholders)?;')
s=s[:a]+'''    let substituted=if FIRST { substituted.first()? } else { substituted };
    let first_pushdown=if FIRST { Some(crate::workspace::SharedWorkspace::new_admitted(
        crate::governor::soundness::plan_cap_pushdown_admitted(&substituted,Some(1),&ctx.growth)?,&ctx.growth)?) } else { None };
'''+s[a:]
a=s.index('        let witness = GraphPattern::Slice {',a)
b=s.index('        let evaluated = eval_evaluated(&witness, &mut guard);',a)
s=s[:a]+'''        let previous=std::mem::replace(&mut guard.cap_pushdown,first_pushdown);
        let cursor=guard.cap_at;
'''+s[b:]
s=s.replace('let evaluated = eval_evaluated(&witness, &mut guard);','let evaluated=eval_evaluated(&substituted,&mut guard);',1)
# Declared APPLY inputs and marker literals use the same native immutable row
# producer, preserving exact mapping order and driver placement.
a=s.index('        let mut row = crate::expr::SubstitutionRow {')
b=s.index('        let source = crate::deferred_exists::CorrelatedSource {',a)
s=s[:a]+'''        let marker=policy.optional.as_ref().map(|optional|(&optional.forget_marker,first));
        let row=outer.application_inputs(inputs,marker,&ctx.growth)?;
'''+s[b:]
post[B]=s

V='crates/sparql-eval/src/eval.rs'
old=function(read(V),'install_local_slice_pushdown')
body=body_of(old,'install_local_slice_pushdown')
body=body.replace('return false;','return Ok(false);')
body=body.replace('CapPushdown::over_plan(plan, pattern, u64::MAX)','CapPushdown::over_plan_admitted(plan,pattern,u64::MAX,&ctx.growth)?')
body=body.replace('crate::governor::soundness::plan_cap_pushdown(\n            pattern,\n            Some(u64::MAX),\n        )','crate::governor::soundness::plan_cap_pushdown_admitted(pattern,Some(u64::MAX),&ctx.growth)?')
body=body.replace('ctx.cap_pushdown = Some(Arc::new(pushdown));','ctx.cap_pushdown=Some(crate::workspace::SharedWorkspace::new_admitted(pushdown,&ctx.growth)?);')
body=body.replace('            true\n','            Ok(true)\n').replace('        None => false,','        None => Ok(false),')
replace(V,'install_local_slice_pushdown',old[:old.index('{')].replace(') -> bool',') -> Result<bool,EvalError>')+'{'+body+'}')
old=function(read(V),'install_answer_cap_pushdown')
body=body_of(old,'install_answer_cap_pushdown').replace('_ => return,','_ => return Ok(()),')
body=body.replace('CapPushdown::over_plan(plan, query_pattern(query), cap)','CapPushdown::over_plan_admitted(plan,query_pattern(query),cap,&ctx.growth)?')
body=body.replace('ctx.cap_pushdown = pushdown.map(Arc::new);','ctx.cap_pushdown=pushdown.map(|value|crate::workspace::SharedWorkspace::new_admitted(value,&ctx.growth)).transpose()?;\n    Ok(())')
replace(V,'install_answer_cap_pushdown',old[:old.index('{')].rstrip()+'->Result<(),EvalError> { '+body+'}')
s=read(V)
for name in ['install_local_slice_pushdown','install_answer_cap_pushdown']:
    for m in reversed(list(re.finditer(r'\b'+name+r'\s*\(',s))):
        line=s[s.rfind('\n',0,m.start())+1:m.start()]
        if 'fn ' in line or line.lstrip().startswith('//'):continue
        p=s.find('(',m.start());end=balanced(s,p,'(',')')
        if s[end:end+1]!='?':s=s[:end]+'?'+s[end:]
post[V]=s
