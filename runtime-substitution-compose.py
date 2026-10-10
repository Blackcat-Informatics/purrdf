from pathlib import Path
import difflib
stage=Path(__file__).parent
root=stage.parents[1]
relative=Path('crates/sparql-eval/src/engine.rs')
before=(root/relative).read_text()
s=before.replace('fn evaluate_with_substitutions<', (stage/'runtime-substitution-owner.rs').read_text()+'\nfn evaluate_with_substitutions<',1)
old='''    let substituted =
        crate::substitute::apply_shacl_prebinding(prepared.query.clone(), substitutions)?;'''
assert s.count(old)==2
s=s.replace(old,'''    let substituted = runtime_substitution(&prepared.query, substitutions, &ctx.workspace.capability())?;''')
s=s.replace('evaluate_query_over(&substituted, None, ctx)', 'evaluate_query_over(&substituted.query, None, ctx)')
s=s.replace('evaluate_query_evaluated(&substituted, ctx)', 'evaluate_query_evaluated(&substituted.query, ctx)')
post=stage/'runtime-substitution-postimages'/relative
post.parent.mkdir(parents=True,exist_ok=True)
post.write_text(s)
(stage/'runtime-substitution-owner-draft.patch').write_text(''.join(difflib.unified_diff(before.splitlines(keepends=True),s.splitlines(keepends=True),fromfile='a/'+str(relative),tofile='b/'+str(relative))))
