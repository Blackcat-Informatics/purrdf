# Why not Rust: Ignored Stage-only complete native caller text assembly preserves shipping files.
P='crates/sparql-eval/src/expr.rs'
prepared=(STAGE/'correlated-prepared-owner.rs').read_text()
post[P]=read(P)+'\n'+prepared+'\n'
post[P]=read(P).replace('fn collect_term_vars_admitted(','pub(crate) fn collect_term_vars_admitted(',1)
V='crates/sparql-eval/src/eval.rs'
s=read(V)
s=re.sub(r'(?m)^(\s*)(?:pub\(crate\) )?static PREPARED_EXISTS_BUILD_COUNT:',r'\1pub(crate) static PREPARED_EXISTS_BUILD_COUNT:',s,count=1)
s=s.replace('normalized: Arc<GraphPattern>','normalized: crate::expr::PreparedPattern')
s=s.replace('witness_wrapped: Arc<GraphPattern>','witness_wrapped: crate::expr::PreparedPattern')
s=s.replace('analysis: Arc<crate::governor::soundness::NodeAnalysisTable>','analysis: crate::workspace::SharedWorkspace<crate::governor::soundness::NodeAnalysisTable>')
s=s.replace('free_vars: Arc<crate::DetHashSet<Variable>>','free_vars: VarSchema')
s=s.replace('free_vars: Arc<DetHashSet<Variable>>','free_vars: VarSchema')
s=s.replace('Arc<crate::expr::SubstitutionSourceMap>','crate::workspace::SharedWorkspace<crate::expr::SubstitutionSourceMap>')
s=s.replace('Arc<crate::deferred_exists::NestedSites>','crate::workspace::SharedWorkspace<crate::deferred_exists::NestedSites>')
s=s.replace('Arc<crate::deferred_exists::DeferredMap>','crate::workspace::SharedWorkspace<crate::deferred_exists::DeferredMap>')
s=s.replace('Arc<crate::deferred_exists::PlanSites>','crate::workspace::SharedWorkspace<crate::deferred_exists::PlanSites>')
s=s.replace('Vec<crate::workspace::SharedWorkspace<crate::expr::SubstitutionSourceMap>>','crate::expr::CorrelatedMaps')
s=s.replace('correlated_node_maps: Vec::new()','correlated_node_maps: crate::expr::CorrelatedMaps::default()')
s=s.replace('self.correlated_node_maps.iter().rev()','self.correlated_node_maps.iter()')
s=s.replace('pub inner: Arc<SolutionSeq<I>>','pub inner: crate::workspace::SharedWorkspace<SolutionSeq<I>>')
s=s.replace('DetHashMap<ExistsCacheKey<D::Id>, Arc<ExistsInner<D::Id>>>','crate::AdmittedMap<ExistsCacheKey<D::Id>,crate::workspace::SharedWorkspace<ExistsInner<D::Id>>>')
s=s.replace('DetHashMap<usize, Arc<PreparedExists>>','crate::AdmittedMap<usize,crate::workspace::SharedWorkspace<PreparedExists>>')
s=s.replace('DetHashMap<ExistsDefinitionMemoKey<D::Id>, bool>','crate::AdmittedMap<ExistsDefinitionMemoKey<D::Id>,bool>')
for field in ['exists_inner_cache','exists_prepared_cache','exists_definition_memo']:
    s=s.replace(field+': DetHashMap::default()',field+': crate::AdmittedMap::default()')
s=s.replace('(usize, (u8, Option<I>), u64, Vec<Option<SolutionTerm<I>>>)','(usize, (u8, Option<I>), u64, crate::solution::RetainedRow<I>)')
s=s.replace('Option<Arc<CapPushdown>>','Option<crate::workspace::SharedWorkspace<CapPushdown>>')
post[V]=s
replace(V,'build','''    #[cfg(test)]
    pub(crate) fn build(pattern:&GraphPattern)->Self {
        Self::build_admitted(pattern,&crate::WorkspaceCapability::resident()).expect("resident EXISTS preparation failed")
    }''')
replace(V,'enter_substituted_exists','''    pub(crate) fn enter_substituted_exists(
        &mut self,ledger_map:Option<crate::expr::SubstitutionSourceMap>,
        deferred:Option<crate::workspace::SharedWorkspace<crate::deferred_exists::DeferredMap>>,
    )->Result<EvalScopeGuard<'_,'d,D>,EvalError> {
        // All fallible growth precedes installation of this substitution window.
        let pushed_map=ledger_map.is_some();
        if let Some(map)=ledger_map {
            let map=crate::workspace::SharedWorkspace::new_admitted(map,&self.growth)?;
            self.correlated_node_maps.push(map,&self.growth)?;
        }
        let prev=self.in_substituted_exists;
        self.in_substituted_exists=true;
        let previous=std::mem::replace(&mut self.deferred_exists,deferred);
        Ok(EvalScopeGuard { ctx:self,restore:ScopeRestore::Substitution { prev,pushed_map,deferred:previous } })
    }''')
replace(V,'prepared_exists','''    pub(crate) fn prepared_exists(&mut self,pattern:&GraphPattern)
        ->Result<crate::workspace::SharedWorkspace<PreparedExists>,EvalError> {
        if self.in_substituted_exists {
            return crate::workspace::SharedWorkspace::new_admitted(PreparedExists::build_admitted(pattern,&self.growth)?,&self.growth);
        }
        // A freshly bounded execution must not mutate a caller-owned plan's
        // persistent site cache with its query account. Its per-run memo still
        // preserves one preparation per body, and dies on every failed run.
        if !self.growth.is_bounded() && let Some(plan)=self.plan.as_ref()
            && let Some(site)=plan.node_of(pattern).and_then(|body|plan.shape().site_of(body)) {
            return plan.shape().prepared_exists(site,&self.growth,||PreparedExists::build_admitted(pattern,&self.growth));
        }
        let key=std::ptr::from_ref(pattern) as usize;
        if let Some(existing)=self.exists_prepared_cache.get(&key) { return Ok(existing.clone()); }
        let built=crate::workspace::SharedWorkspace::new_admitted(PreparedExists::build_admitted(pattern,&self.growth)?,&self.growth)?;
        self.exists_prepared_cache.insert_admitted(key,built.clone(),&self.growth)?;
        Ok(built)
    }''')
PM='crates/sparql-eval/src/plan/mod.rs'
post[PM]=read(PM).replace('prepared: OnceLock<Arc<PreparedExists>>','prepared: OnceLock<crate::workspace::SharedWorkspace<PreparedExists>>')
replace(PM,'prepared_exists','''    pub(crate) fn prepared_exists(
        &self,site:SiteId,workspace:&crate::WorkspaceCapability,
        build:impl FnOnce()->Result<PreparedExists,crate::EvalError>,
    )->Result<crate::workspace::SharedWorkspace<PreparedExists>,crate::EvalError> {
        let slot=&self.sites[site.0 as usize].prepared;
        let transient=self.volatile_sites.get(site.0 as usize).copied()==Some(true) || workspace.is_bounded();
        if !transient && let Some(existing)=slot.get() { return Ok(existing.clone()); }
        let built=crate::workspace::SharedWorkspace::new_admitted(build()?,workspace)?;
        if transient { return Ok(built); }
        Ok(slot.get_or_init(||built).clone())
    }''')

# Native row-restriction keys retain exactly the cell/control owners they use.
replace(P,'definition_restriction_key','''fn definition_restriction_key<I:ViewTermId>(
    row:&[Option<SolutionTerm<I>>],schema:&VarSchema,free_vars:&VarSchema,workspace:&crate::WorkspaceCapability,
)->Result<RetainedRow<I>,EvalError> {
    let width=schema.vars().iter().filter(|v|free_vars.contains(v)).count();
    RetainedRow::from_cells(width,schema.vars().iter().enumerate().filter(|(_,v)|free_vars.contains(v)).map(|(index,_)|row[index]),workspace)
}''')
s=read(P)
s=s.replace('&Arc<SubstitutionSourceMap>','&crate::workspace::SharedWorkspace<SubstitutionSourceMap>')
s=s.replace("&'s Arc<SubstitutionSourceMap>","&'s crate::workspace::SharedWorkspace<SubstitutionSourceMap>")
s=s.replace('free_vars: &DetHashSet<Variable>','free_vars: &VarSchema')
s=s.replace('ctx.prepared_exists(pattern);','ctx.prepared_exists(pattern)?;')
s=s.replace('free_vars\n        .iter()','free_vars\n        .vars().iter()')
s=s.replace('let prepared = crate::eval::PreparedExists::build(body);','let prepared = crate::eval::PreparedExists::build_admitted(body,&ctx.growth)?;')
s=s.replace('let mut copy: Option<Box<GraphPattern>> = None;','let mut copy: Option<CorrelatedPattern> = None;')
s=s.replace('for layer in slot.env.layers() {','for layer in slot.env.layers(&ctx.growth)? {')
s=s.replace('copy = Some(substitute_pattern(from, layer)?);','copy = Some(substitute_pattern_admitted(from,layer,&ctx.growth)?);')
s=s.replace('crate::deferred_exists::with_row(env, &current, &site.vars)','crate::deferred_exists::with_row(env,&current,&site.vars,&ctx.growth)?')
for m in reversed(list(re.finditer(r'ctx\.correlated_node_maps\.push\(',s))):
    p=s.find('(',m.start());e=balanced(s,p,'(',')');args=s[p+1:e-1]
    args=re.sub(r'Arc::clone\(&?(.*?)\)',r'\1.clone()',args)
    s=s[:m.start()]+'ctx.correlated_node_maps.push('+args+',&ctx.growth)?'+s[e:]
s=s.replace('let inner = Arc::new(evaluated.into_complete()','let inner = crate::workspace::SharedWorkspace::new_admitted(evaluated.into_complete()')
s=s.replace('unreachable!("a non-truncated result is complete by construction")\n                }));','unreachable!("a non-truncated result is complete by construction")\n                }),&ctx.growth)?;')
s=s.replace('let entry = Arc::new(crate::eval::ExistsInner {','let entry = crate::workspace::SharedWorkspace::new_admitted(crate::eval::ExistsInner {')
s=s.replace('                    wild,\n                });','                    wild,\n                },&ctx.growth)?;')
for cache in ['exists_inner_cache','exists_definition_memo']:
    s=calls(s,{'ctx.'+cache+'.insert':'ctx.'+cache+'.insert_admitted'},'&ctx.growth')
s=s.replace('let memo_key = use_memo.then(|| {\n            let restriction = definition_restriction_key(row, schema, free_vars);','let memo_key = if use_memo {\n            let restriction = definition_restriction_key(row,schema,free_vars,&ctx.growth)?;\n            Some(')
# End of the definition key's original tuple block.
needle='''                restriction,
            )
        });'''
s=s.replace(needle,'''                restriction,
            ))
        } else { None };''',1)
a=s.index('    let memo_key = (ctx.options.exists_memo && !ctx.in_substituted_exists).then(|| {')
b=s.index('    if let Some(key) = &memo_key',a)
s=s[:a]+'''    let memo_key=if ctx.options.exists_memo && !ctx.in_substituted_exists {
        Some((site,ctx.graph_key(),crate::eval::schema_fingerprint(schema),
            definition_restriction_key(row,schema,free_vars,&ctx.growth)?))
    } else { None };
'''+s[b:]
# Compute fallible pushdown BEFORE touching the active evidence cursor/window.
a=s.index('    let previous_pushdown = ctx.cap_pushdown.take();',s.index('fn exists_positive_seeded'))
b=s.index('    let evaluated =',a)
s=s[:a]+'''    let next_pushdown=crate::workspace::SharedWorkspace::new_admitted(
        crate::governor::soundness::plan_cap_pushdown_admitted(normalized,Some(1),&ctx.growth)?,&ctx.growth)?;
    let previous_pushdown=ctx.cap_pushdown.replace(next_pushdown);
    let previous_cursor=ctx.cap_at;
'''+s[b:]
post[P]=s
