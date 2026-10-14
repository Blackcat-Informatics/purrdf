from pathlib import Path
import re
root=Path(__file__).parents[2]
ns={"__file__":__file__};exec((root/".stage/typed-capacity-errors-instead-of-u32/migrate-fixture-results.py").read_text().split("files =")[0],ns)
masked,close=ns["masked"],ns["close"]
file=root/"crates/datalog/src/resolve_fol.rs";source=file.read_text()
source=source.replace("use purrdf_hash::sha1::Sha1;","use purrdf_hash::sha1::Sha1;\nuse purrdf_lex::allocation::StorageError;")
source=source.replace("pub enum FolProofError {","pub enum FolProofError {\n    /// Native refusal, distinct from rejecting an invalid proof.\n    Storage(StorageError),")
source=source.replace("// ── The independent checker", "purrdf_lex::variant_from!(FolProofError { Storage(StorageError) });\n\n// ── The independent checker")
# Normal clause refusal and operational failure share no invented verdict.
source=source.replace("// ── The DlClause lowering adapter",'''/// Refusal while lowering or executing a flat backward query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolError {
    /// The original clause has no flat backward-resolution semantics.
    Clause(NonDatalogClause),
    /// Original native storage or logical capacity refusal.
    Storage(StorageError),
}
impl core::fmt::Display for FolError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self { Self::Clause(e) => e.fmt(f), Self::Storage(e) => e.fmt(f) }
    }
}
impl std::error::Error for FolError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self { Self::Clause(e) => Some(e), Self::Storage(e) => Some(e) }
    }
}
purrdf_lex::variant_from!(FolError { Clause(NonDatalogClause), Storage(StorageError) });

// ── The DlClause lowering adapter''')
def change(name,transform):
    global source
    mask=masked(source);m=re.search(r"\bfn "+name+r"\s*\(",mask);assert m,name
    brace=mask.find("{",m.end());end=close(mask,brace,"{","}")
    old=source[m.start():end];new=transform(old);assert new!=old,name
    source=source[:m.start()]+new+source[end:]
calls=("dag.fresh_meta","dag.intern_leaf","dag.intern_app","dag.intern_binder","dag.intern_bound",
       "unify::apply","unify::unify_sorted","renaming.bind_renaming","subst.declare_meta_sort",
       "engine.declare_known_sorts","freshen_clause","is_ground","solve_body","expand_round","ground","project")
def propagate(s,extra=()):
    for name in calls+tuple(extra):
        mask=masked(s);additions=[]
        for m in re.finditer(re.escape(name)+r"\s*\(",mask):
            if mask[max(0,m.start()-3):m.start()]=="fn ":continue
            end=close(mask,m.end()-1,"(",")")
            if s[end:end+1]!="?":additions.append(end)
        for pos in reversed(additions):s=s[:pos]+"?"+s[pos:]
    return s
change("declare_known_sorts",lambda s:s.replace(") {",") -> Result<(), StorageError> {",1)
       .replace("return;","return Ok(());").replace("subst.declare_meta_sort(meta, sort);","subst.declare_meta_sort(meta, sort)?;")
       .replace("\n    }","\n        Ok(())\n    }"))
change("freshen_clause",lambda _:'''fn freshen_clause(
    dag: &mut TermDag, clause: &FolClause, meta_sorts: &BTreeMap<MetaId, NodeId>,
) -> Result<(NodeId, Vec<FolLit>, Vec<(MetaId, NodeId)>), StorageError> {
    let mut nodes: Vec<NodeId> = vec![clause.head];
    nodes.extend(clause.body.iter().map(FolLit::atom));
    let metas = distinct_metas(dag, &nodes);
    let mut renaming = Subst::default();
    let mut sort_decls = Vec::new();
    for old in metas {
        let (fresh_id, fresh) = dag.fresh_meta()?;
        renaming.bind_renaming(old, fresh)?;
        if let Some(&sort) = meta_sorts.get(&old) { sort_decls.push((fresh_id, sort)); }
    }
    let head = unify::apply(dag, &renaming, clause.head)?;
    let mut body = Vec::new();
    body.try_reserve_exact(clause.body.len()).map_err(|_| StorageError::AllocationFailed)?;
    for lit in &clause.body {
        body.push(match lit {
            FolLit::Pos(atom) => FolLit::Pos(unify::apply(dag, &renaming, *atom)?),
            FolLit::Neg(atom) => FolLit::Neg(unify::apply(dag, &renaming, *atom)?),
        });
    }
    Ok((head, body, sort_decls))
}''')
change("is_ground",lambda s:propagate(s.replace("-> bool","-> Result<bool, StorageError>")
            .replace("dag.free_meta(applied).is_empty()","Ok(dag.free_meta(applied).is_empty())")))
def unit(s):
    s=s.replace(") {",") -> Result<(), StorageError> {",1).replace("return;","return Ok(());")
    s=propagate(s)
    return s[:-1]+"    Ok(())\n}"
change("solve_body",unit)
change("expand_round",lambda s:propagate(s.replace("-> Vec<Produced>","-> Result<Vec<Produced>, StorageError>")
                                      .replace("\n    produced\n}","\n    Ok(produced)\n}")))
change("ground",lambda s:propagate(s.replace("-> bool","-> Result<bool, StorageError>")
                 .replace("return false;","return Ok(false);").replace("return true;","return Ok(true);")
                 .replace("\n    false\n}","\n    Ok(false)\n}")))
change("project",lambda s:propagate(s.replace("-> Vec<FolBinding>","-> Result<Vec<FolBinding>, StorageError>")
                                    .replace("\n    answers\n}","\n    Ok(answers)\n}")))
def resolve(s):
    s=s.replace("-> FolControl","-> Result<FolControl, StorageError>")
    s=s.replace("return FolControl::Unsupported(FolUnsupported::ClauseBodyTooWide {",
                "return Ok(FolControl::Unsupported(FolUnsupported::ClauseBodyTooWide {")
    s=s.replace("literals: clause.body.len(),\n            });","literals: clause.body.len(),\n            }));")
    s=s.replace("return FolControl::Unsupported(FolUnsupported::Floundering);",
                "return Ok(FolControl::Unsupported(FolUnsupported::Floundering));")
    s=s.replace("FolControl::Decided(Box::new(FolOutcome {","Ok(FolControl::Decided(Box::new(FolOutcome {")
    s=s.replace("\n    }))","\n    })))")
    return propagate(s)
change("resolve_fol",resolve)
change("check_fol_proof",propagate)
change("lower_term",lambda _:'''fn lower_term(
    dag: &mut TermDag, scope: &mut BTreeMap<String, NodeId>, term: &ClauseTerm,
) -> Result<NodeId, StorageError> {
    match term {
        ClauseTerm::Var(name) => {
            if let Some(&node) = scope.get(name) { return Ok(node); }
            let (_, node) = dag.fresh_meta()?;
            scope.insert(name.clone(), node);
            Ok(node)
        }
        other => {
            let surface = other.surface().expect("a non-variable clause term has a lexical surface");
            dag.intern_leaf(&surface)
        }
    }
}''')
change("lower_atom",lambda s:s.replace("-> NodeId","-> Result<NodeId, StorageError>")
       .replace(".collect();",".collect::<Result<Vec<_>, StorageError>>()?;"))
def clauses(s):
    s=s.replace("NonDatalogClause>","FolError>").replace("return Err(NonDatalogClause::new(index, clause.head_form()));",
        "return Err(NonDatalogClause::new(index, clause.head_form()).into());")
    s=s.replace("return Err(NonDatalogClause::guarded(index, clause.head_form()));",
        "return Err(NonDatalogClause::guarded(index, clause.head_form()).into());")
    s=propagate(s,("lower_atom",))
    s=s.replace("FolLit::Neg(lowered)","Ok(FolLit::Neg(lowered))").replace("FolLit::Pos(lowered)","Ok(FolLit::Pos(lowered))")
    s=s.replace(".collect();",".collect::<Result<Vec<_>, StorageError>>()?;")
    return s
change("lower_datalog_clauses",clauses)
change("lower_goal",lambda _:'''fn lower_goal(
    dag: &mut TermDag, triple_op: NodeId, goal: &ClauseAtom,
) -> Result<(NodeId, Vec<(NodeId, String)>), StorageError> {
    let mut scope = BTreeMap::new();
    let mut order = Vec::new();
    let mut args = Vec::new();
    for term in goal.terms() {
        let node = match term {
            ClauseTerm::Var(name) => {
                if let Some(&node) = scope.get(name) { node }
                else {
                    let (_, node) = dag.fresh_meta()?;
                    scope.insert(name.clone(), node);
                    order.push((node, name.clone()));
                    node
                }
            }
            other => {
                let surface = other.surface().expect("a non-variable clause term has a lexical surface");
                dag.intern_leaf(&surface)?
            }
        };
        args.push(node);
    }
    Ok((dag.intern_app(triple_op, args)?, order))
}''')
change("solve_datalog_goal",lambda s:propagate(s.replace("NonDatalogClause>","FolError>"),("lower_goal","resolve_fol")))
file.write_text(source)
print("Migrated original FOL freshening, unification, projection and public error propagation.")
