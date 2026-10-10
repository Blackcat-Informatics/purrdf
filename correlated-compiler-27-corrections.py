from pathlib import Path
import difflib
root=Path.cwd()
stage=root/'.stage/sparql-eval-complete-bounded-workspace'
post=stage/'correlated-compiler-27-postimages'
changes={}
def edit(path, f):
    before=(root/path).read_text(); after=f(before)
    assert before!=after,path
    changes[path]=(before,after)
    p=post/path;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(after)
def expr(s):
    s=s.replace('use purrdf_sparql_algebra::{Chain, Child};','use purrdf_sparql_algebra::{Chain, Child, NonEmpty};',1)
    s=s.replace('use std::sync::Arc;','#[cfg(test)]\nuse std::sync::Arc;',1)
    s=s.replace('crate::solution::VarSchema','VarSchema')
    s=s.replace('Expression::Arithmetic(first.into(), Chain::try_from(copied)','Expression::Arithmetic(first.into(), NonEmpty::try_from(copied)')
    s=s.replace('Expression::In(needle,items)','Expression::In(needle,items.into())')
    s=s.replace('Expression::Coalesce(substitute_expression_list::<PRESERVE>(items,row,map,defer,memory)?)','Expression::Coalesce(substitute_expression_list::<PRESERVE>(items,row,map,defer,memory)?.into())')
    s=s.replace('substitute_expression_list::<PRESERVE>(args,row,map,defer,memory)?)','substitute_expression_list::<PRESERVE>(args,row,map,defer,memory)?.into())')
    s=s.replace('purrdf_iri::vocab::rdf::language_datatype_iri','purrdf_iri::vocab::language_datatype_iri')
    return s
edit('crates/sparql-eval/src/expr.rs',expr)
def sub(s):
    s=s.replace('fn rewrite_child<T: purrdf_sparql_algebra::Subtree>(', 'fn rewrite_child<T: purrdf_sparql_algebra::Subtree + purrdf_lex::walk::Dismantle>(')
    a=s.index('fn carry_past_group_with_memory');b=s.index('\nfn ',a+5)
    t=s[a:b].replace('let mut indices = crate::AdmittedVec::new','let mut indices: crate::AdmittedVec<usize> = crate::AdmittedVec::new')
    s=s[:a]+t+s[b:]
    a=s.index('fn descend_into_inner_with_memory');b=s.index('\nfn ',a+5)
    t=s[a:b].replace('    memory: &mut RewriteMemory','    _memory: &mut RewriteMemory')
    s=s[:a]+t+s[b:]
    a=s.index('fn seed_reaches_every_read_with_memory');b=s.index('\nfn ',a+5)
    t=s[a:b].replace('return Ok(Flow::Descend);','return Flow::Descend;')
    return s[:a]+t+s[b:]
edit('crates/sparql-eval/src/substitute.rs',sub)
def enf(s):
    s=s.replace('use purrdf_sparql_algebra::{Chain, Child};','use purrdf_sparql_algebra::Chain;',1)
    p=s.index('    /// [`normalize_with_memory`], written as the recursion')
    s=s[:p]+'''    /// The independent recursive oracle copies its caller-owned fixture tree.
    fn copied(pattern: &GraphPattern) -> GraphPattern {
        crate::op_count::count_copied(purrdf_sparql_algebra::NodeRef::Pattern(pattern));
        pattern.clone()
    }

'''+s[p:]
    s=s.replace('use super::{Enf, copied, ledger_source_map','use super::{Enf, ledger_source_map')
    return s
edit('crates/sparql-eval/src/enf.rs',enf)
edit('crates/sparql-eval/src/service_endpoints.rs',lambda s:s.replace('let substituted = crate::expr::substitute_pattern(node, &row)?;', 'let substituted = crate::expr::substitute_pattern_admitted(node, &row, &ctx.growth)?;'))
edit('crates/sparql-eval/src/parallel.rs',lambda s:s.replace('->Result<bool,crate::EvalError>','->Result<bool,EvalError>'))
edit('crates/sparql-eval/src/binop.rs',lambda s:s.replace('use std::sync::Arc;','#[cfg(test)]\nuse std::sync::Arc;',1))
edit('crates/sparql-eval/src/workspace.rs',lambda s:s.replace('label: "focus".to_owned(),\n                    scope: None,','label: "focus".to_owned(),\n                    scope: purrdf_core::BlankScope::DEFAULT,',1))
patch=''.join(''.join(difflib.unified_diff(before.splitlines(True),after.splitlines(True),fromfile='a/'+path,tofile='b/'+path)) for path,(before,after) in changes.items())
(stage/'correlated-compiler-27-corrections.patch').write_text(patch)
print(f'{len(changes)} homes; {len(patch)} bytes; Stage-only')
