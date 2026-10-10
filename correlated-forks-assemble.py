# Why not Rust: Ignored Stage-only assembly preserves shipping source and the one build lane.
W='crates/sparql-eval/src/workspace.rs'
s=read(W)
at=s.index('    /// Borrow each existing key and value without an iterator allocation.')
s=s[:at]+'''    /// Copy table metadata through the same admitted insert home. The callback
    /// carries each original payload owner or admits its actual independent copy.
    pub(crate) fn clone_entries_admitted(
        &self,workspace:&WorkspaceCapability,
        mut copy:impl FnMut(&K,&V)->Result<(K,V),EvalError>,
    )->Result<Self,EvalError> {
        let mut output=Self::default();
        for (key,value) in self.iter() {
            let (key,value)=copy(key,value)?;
            output.insert_admitted(key,value,workspace)?;
        }
        Ok(output)
    }

'''+s[at:]
post[W]=s
V='crates/sparql-eval/src/eval.rs'
s=read(V)
for name in ['fork_for_worker','fork_for_worker_over','fork_for_loop_worker']:
    old=function(s,name)
    post[V]=s
    replace(V,name,old.replace('-> Self','-> Result<Self,EvalError>'))
    s=read(V)
old=function(s,'fork_with_scratch');body=body_of(old,'fork_with_scratch')
for field in ['exists_inner_cache','exists_prepared_cache']:
    body=body.replace(field+': self.'+field+'.clone()',field+': self.'+field+'.clone_entries_admitted(&self.growth,|key,value|Ok((*key,value.clone())))?')
body=body.replace('exists_definition_memo: self.exists_definition_memo.clone()',
'''exists_definition_memo:self.exists_definition_memo.clone_entries_admitted(&self.growth,|key,value| {
                Ok(((key.0,key.1,key.2,key.3.clone_admitted(&self.growth)?),*value))
            })?''')
body=body.replace('        Self {','        Ok(Self {',1)
end=body.rfind('        }');body=body[:end]+'        })'+body[end+9:]
replace(V,'fork_with_scratch',old[:old.index('{')].replace('-> Self','-> Result<Self,EvalError>')+'{'+body+'}')
s=read(V)
# Every actual bounded entry selects the original direct executor. This
# prevents raw resident scratch snapshots in fork-only producers; query law,
# bag ordering and all supported operators remain unchanged.
s=s.replace('        !self.governors_are_engaged()\n    }','        !self.growth.is_bounded() && !self.governors_are_engaged()\n    }',1)
s=s.replace('(self.governors.is_some()\n            && crate::parallel::should_parallelize','(!self.growth.is_bounded() && self.governors.is_some()\n            && crate::parallel::should_parallelize',1)
post[V]=s

# The one original parallel init body receives a fallible constructor. Old
# unrelated callers retain their infallible constructor convenience door.
R='crates/sparql-eval/src/parallel.rs'
for name in ['par_chunk_try_map_init','par_loop_try_map_init','par_blocks_try_map_init']:
    old=function(read(R),name)
    native=old.replace('fn '+name+'<','fn '+name+'_fallible<',1).replace('init: impl Fn() -> S + Sync','init: impl Fn() -> Result<S,EvalError> + Sync')
    native=native.replace('let mut state = init();','let mut state = init()?;')
    if name=='par_loop_try_map_init':
        native=native.replace('par_blocks_try_map_init(','par_blocks_try_map_init_fallible(').replace('par_chunk_try_map_init(','par_chunk_try_map_init_fallible(')
    if name=='par_blocks_try_map_init':
        native=native.replace('par_chunk_try_map_init(true,','par_chunk_try_map_init_fallible(true,')
        a=native.index('                let mut state = init()?;')
        b=native.index('                let failed = outcome.is_err();',a)
        original=native[a:b]
        original=original.replace('                let outcome = block','                block')
        original=original.replace('.map(|()| (acc, harvest(&mut state)));','.map(|()| (acc, harvest(&mut state)))')
        native=native[:a]+'''                let outcome=(|| {
'''+original+'''                })();
'''+native[b:]
    header=old[:old.index('{')]
    args='bounded,sequential,items,||Ok(init()),push,harvest' if name=='par_loop_try_map_init' else 'sequential,items,||Ok(init()),push,harvest'
    replace(R,name,header+'{ '+name+'_fallible('+args+') }')
    post[R]=read(R)+'\n/// Original ordered worker law with a fallible initial owner.\n'+native+'\n'

# Actual fork callers use native Result propagation; independent test helpers
# whose construction is explicitly resident keep their old state shape.
for path in ['crates/sparql-eval/src/expr.rs','crates/sparql-eval/src/binop.rs','crates/sparql-eval/src/modifier.rs']:
    s=read(path)
    s=s.replace('let mut child = ctx_ref.fork_for_worker();','let mut child=ctx_ref.fork_for_worker()?;')
    s=s.replace('let mut child = ctx.fork_for_loop_worker(snapshot.as_ref());','let mut child=ctx.fork_for_loop_worker(snapshot.as_ref())?;')
    if path.endswith('binop.rs'):
        s=s.replace('|| (ctx.fork_for_worker(), None),','|| Ok((ctx.fork_for_worker()?,None)),')
        # This occurrence is the original parallel BGP optional driver.
        a=s.index('fn parallel_seeded_optional')
        b=s.index('\n/// Admit a completed seeded row',a)
        s=s[:a]+s[a:b].replace('par_chunk_try_map_init(','par_chunk_try_map_init_fallible(')+s[b:]
    # Fork-loop constructor closures already contain ?; return their original
    # state tuple in Ok rather than installing any failed partial state.
    s=s.replace('crate::parallel::par_loop_try_map_init(','crate::parallel::par_loop_try_map_init_fallible(')
    for ending in ['(child, checkpoint, linked.fresh())','(child, fresh, ledger)','(child, linked.fresh(), ledger)']:
        s=s.replace('                '+ending+'\n','                Ok('+ending+')\n')
    post[path]=s

for path in ['crates/sparql-eval/src/parallel.rs','crates/sparql-eval/src/xpath_regex.rs',
             'crates/sparql-eval/src/row_checkpoint.rs','crates/sparql-eval/src/governor/charge_points.rs',
             'crates/sparql-eval/src/user_fn.rs']:
    s=read(path)
    # Only these actual occurrences belong to resident test/oracle setup.
    for name in ['fork_for_worker','fork_for_loop_worker','fork_for_worker_over']:
        for m in reversed(list(re.finditer(r'\.'+name+r'\(',s))):
            p=s.find('(',m.start());e=balanced(s,p,'(',')')
            if s[e:e+1] not in ['?','.']:
                s=s[:e]+'.expect("resident worker owner admission")'+s[e:]
    post[path]=s
