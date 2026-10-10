from pathlib import Path
import difflib
stage=Path(__file__).parent
root=stage.parents[1]
relative=Path('crates/sparql-eval/src/engine.rs')
before=(root/relative).read_text()
s=before
a=s.index('fn relation_identity(')
b=s.index('\n/// Apply the [`QueryOptions`]',a)
s=s[:a]+(stage/'governed-metadata-owner.rs').read_text()+s[b:]
# Every original producer invocation receives the actual execution account.
s=s.replace('let identity = relation_identity(prepared, options.property_functions())?;', 'let mut identity_owner = relation_identity(prepared, options.property_functions(), &workspace.capability())?;')
s=s.replace('let identity = relation_identity(&prepared, options.property_functions())?;', 'let mut identity_owner = relation_identity(&prepared, options.property_functions(), &workspace.capability())?;')
# The admission refusal moves the sole identity, rather than cloning an
# unpriced second IRI array. A non-refused admission leaves it intact.
s=s.replace('identity: &RelationIdentity,\n', 'identity: &mut RelationIdentity,\n')
s=s.replace('relations: identity.clone(),', 'relations: core::mem::replace(identity, RelationIdentity::EMPTY),')
s=s.replace('                &identity,\n', '                &mut identity_owner.value,\n')
s=s.replace('            &identity,\n', '            &mut identity_owner.value,\n')
s=s.replace('resolve_governed(evaluated, &mut ctx, state, identity)', 'resolve_governed(evaluated, &mut ctx, state, core::mem::replace(&mut identity_owner.value, RelationIdentity::EMPTY))')
a=s.index('    fn query_governed_prepared_admitted')
b=s.index('\n    ///',a)
p=s[a:b]
p=p.replace('Result<GovernedOutcome, EvaluationFailure>', 'Result<AdmittedGovernedOutcome, EvaluationFailure>')
p=p.replace('return refused.map(GovernedOutcome::BudgetExhausted);', 'return refused.map(|exhausted| AdmittedGovernedOutcome { outcome: GovernedOutcome::BudgetExhausted(exhausted), _identity: identity_owner._frame });')
p=p.replace('materialize_governed(evaluated, &mut ctx, state, identity)', 'materialize_governed(evaluated, &mut ctx, state, core::mem::replace(&mut identity_owner.value, RelationIdentity::EMPTY)).map(|outcome| AdmittedGovernedOutcome { outcome, _identity: identity_owner._frame })')
s=s[:a]+p+s[b:]
# Legacy public doors explicitly transfer already-built owned metadata to the
# caller. The operational fallible door retains its private owner to finishing.
for name in ['fn query_governed_view','pub fn query_prepared_governed_view','fn query_governed_prepared_in_state','pub fn query_governed_in_operation']:
    a=s.index('    '+name)
    b=s.index('\n    ///',a)
    p=s[a:b]
    call=p.index('self.query_governed_prepared_admitted(')
    close=p.index('\n            )',call)+len('\n            )')
    p=p[:close]+'.map(AdmittedGovernedOutcome::into_caller)'+p[close:]
    s=s[:a]+p+s[b:]
a=s.index('fn finish_governed_fallible_query')
b=s.index('\n///',a)
p=s[a:b].replace('evaluation: Result<GovernedOutcome, EvaluationFailure>', 'evaluation: Result<AdmittedGovernedOutcome, EvaluationFailure>')
p=p.replace('Ok(GovernedOutcome::Complete { result, .. }) => {', '''Ok(AdmittedGovernedOutcome { outcome: GovernedOutcome::Complete { result, relations, .. }, _identity }) => {
                    drop(relations);
                    drop(_identity);''')
p=p.replace('Ok(GovernedOutcome::BudgetExhausted(exhausted)) => {', '''Ok(AdmittedGovernedOutcome { outcome: GovernedOutcome::BudgetExhausted(exhausted), _identity }) => {
                    let BudgetExhausted { tripped, evidence, relations, partial } = exhausted;
                    drop(relations);
                    drop(_identity);
                    let exhausted = BudgetExhausted { tripped, evidence, relations: RelationIdentity::EMPTY, partial };''')
s=s[:a]+p+s[b:]
post=stage/'governed-metadata-postimages'/relative
post.parent.mkdir(parents=True,exist_ok=True)
post.write_text(s)
(stage/'governed-metadata-owner-draft.patch').write_text(''.join(difflib.unified_diff(before.splitlines(keepends=True),s.splitlines(keepends=True),fromfile='a/'+str(relative),tofile='b/'+str(relative))))
