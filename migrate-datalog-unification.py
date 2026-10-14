from pathlib import Path
import re
root=Path(__file__).parents[2]
helpers=(root/".stage/typed-capacity-errors-instead-of-u32/migrate-fixture-results.py").read_text().split("files =")[0]
ns={"__file__":__file__}; exec(helpers,ns)
masked, close=ns["masked"],ns["close"]
file=root/"crates/datalog/src/unify.rs"
source=file.read_text()
def change(name, transform):
    global source
    mask=masked(source)
    m=re.search(r"\bfn "+name+r"\s*\(",mask)
    assert m,name
    brace=mask.find("{",m.end()); end=close(mask,brace,"{","}")
    old=source[m.start():end]
    new=transform(old)
    assert old!=new,name
    source=source[:m.start()]+new+source[end:]
def core(s):
    s=s.replace("-> Unified", "-> Result<Unified, StorageError>")
    s=s.replace("let checkpoint = s.clone();","let checkpoint = s.try_clone()?;")
    s=s.replace("!matches!(result, Unified::Ok)", "!matches!(result, Ok(Unified::Ok))")
    return s
change("unify",core);change("unify_sorted",core)
def normal(s):
    s=s.replace("-> Unified", "-> Result<Unified, StorageError>")
    s=re.sub(r"Unified::(Clash|Occurs)\s*\{[^{}]*\}",lambda m:"Ok("+m[0]+")",s)
    s=re.sub(r"\bUnified::Ok\b","Ok(Unified::Ok)",s)
    # Unwrap only actual recursive calls; the return type carries physical failure.
    s=re.sub(r"(let (?:op_result|result) = unify_at\([^;]+\));",r"\1?;",s)
    s=s.replace("return op_result;","return Ok(op_result);").replace("return result;","return Ok(result);")
    # Existing matches inspect the already successful normal result.
    s=s.replace("matches!(op_result, Ok(Unified::Ok))", "matches!(op_result, Unified::Ok)")
    s=s.replace("matches!(result, Ok(Unified::Ok))", "matches!(result, Unified::Ok)")
    return s
def unify_at(s):
    s=normal(s)
    s=s.replace("whnf(dag, s, a, depth);", "whnf(dag, s, a, depth)?;")
    s=s.replace("whnf(dag, s, b, depth);", "whnf(dag, s, b, depth)?;")
    return s
change("unify_at",unify_at)
change("whnf",lambda s:s.replace("-> NodeId","-> Result<NodeId, StorageError>").replace("return resolved;","return Ok(resolved);"))
def bind(s):
    s=normal(s).replace("shift_down(dag, t, depth) else","shift_down(dag, t, depth)? else")
    s=s.replace("s.bind(m, lowered);","s.bind(m, lowered)?;")
    s=s.replace("s.declare_meta_sort(m2, glb);","s.declare_meta_sort(m2, glb)?;")
    return s
change("bind_meta",bind)
change("apply",lambda s:s.replace("-> NodeId","-> Result<NodeId, StorageError>"))
def operation(s,name):
    s=s.replace("-> NodeId","-> Result<NodeId, StorageError>")
    s=s.replace("return cached;","return Ok(cached);")
    # All operation expressions produce NodeId only after typed propagation.
    for call in ("apply_at","shift_at","shift","dag.intern_app","dag.intern_binder","dag.intern_bound"):
        mask=masked(s); additions=[]
        for m in re.finditer(re.escape(call)+r"\s*\(",mask):
            if mask[max(0,m.start()-3):m.start()]=="fn ": continue
            end=close(mask,m.end()-1,"(",")")
            if s[end:end+1]!="?": additions.append(end)
        for pos in reversed(additions):s=s[:pos]+"?"+s[pos:]
    # Iterator closures return the native result, collected before publication.
    s=s.replace("memo)?))","memo)))")
    s=s.replace(".collect();",".collect::<Result<Vec<_>, StorageError>>()?;")
    # Restore fallible closure operands: ? belongs to collect, not its closure.
    s=re.sub(r"(\.map\(\|[^|]+\| (?:apply_at|shift_at)\([^;\n]*\))\?\)",r"\1)",s)
    s=s.replace("debruijn + by", "debruijn.checked_add(by).ok_or(StorageError::SizeOverflow)?")
    s=s.replace("\n    result\n}", "\n    Ok(result)\n}")
    return s
change("apply_at",lambda s:operation(s,"apply_at"))
change("shift",lambda s:s.replace("-> NodeId","-> Result<NodeId, StorageError>").replace("return node;","return Ok(node);"))
change("shift_at",lambda s:operation(s,"shift_at"))
change("shift_down",lambda _:'''fn shift_down(dag: &mut TermDag, node: NodeId, by: u32) -> Result<Option<NodeId>, StorageError> {
    if by == 0 { return Ok(Some(node)); }
    let mut memo = BTreeMap::new();
    shift_down_at(dag, node, by, 0, &mut memo)
}''')
change("shift_down_at",lambda _:'''fn shift_down_at(
    dag: &mut TermDag,
    node: NodeId,
    by: u32,
    cutoff: u32,
    memo: &mut BTreeMap<(NodeId, u32), Option<NodeId>>,
) -> Result<Option<NodeId>, StorageError> {
    if let Some(&cached) = memo.get(&(node, cutoff)) { return Ok(cached); }
    let result = match dag.data(node).clone() {
        NodeData::Leaf(_) | NodeData::Free(_) | NodeData::Meta(_) => Some(node),
        NodeData::Bound { debruijn, slot } => {
            if debruijn < cutoff { Some(node) }
            else if debruijn < cutoff.checked_add(by).ok_or(StorageError::SizeOverflow)? { None }
            else { Some(dag.intern_bound(debruijn - by, slot)?) }
        }
        NodeData::App { op, args } => {
            let Some(new_op) = shift_down_at(dag, op, by, cutoff, memo)? else { return Ok(None); };
            let mut new_args = Vec::new();
            new_args.try_reserve_exact(args.len()).map_err(|_| StorageError::AllocationFailed)?;
            for arg in args {
                let Some(arg) = shift_down_at(dag, arg, by, cutoff, memo)? else { return Ok(None); };
                new_args.push(arg);
            }
            Some(dag.intern_app(new_op, new_args)?)
        }
        NodeData::Binder { op, sorts, body } => {
            let Some(new_op) = shift_down_at(dag, op, by, cutoff, memo)? else { return Ok(None); };
            let mut new_sorts = Vec::new();
            new_sorts.try_reserve_exact(sorts.len()).map_err(|_| StorageError::AllocationFailed)?;
            for sort in sorts {
                let Some(sort) = shift_down_at(dag, sort, by, cutoff, memo)? else { return Ok(None); };
                new_sorts.push(sort);
            }
            let Some(new_body) = shift_down_at(dag, body, by,
                cutoff.checked_add(1).ok_or(StorageError::SizeOverflow)?, memo)? else { return Ok(None); };
            Some(dag.intern_binder(new_op, new_sorts, new_body)?)
        }
    };
    memo.insert((node, cutoff), result);
    Ok(result)
}''')
file.write_text(source)
print("Migrated original unification/memo/shift bodies to typed storage propagation.")
