from pathlib import Path
import difflib
stage=Path('.stage/sparql-eval-complete-bounded-workspace'); p=stage/'native-call-runtime-postimage.rs'; s=p.read_text()
s=s.replace('use crate::DetHashMap;','#[cfg(test)]\nuse crate::DetHashMap;',1)
s=s.replace('Arg::Slot(slot) => occurrences[*slot] += 1,','Arg::Slot(slot) => occurrences[*slot] = occurrences[*slot].checked_add(1).ok_or(EvalError::WorkspaceBoundOverflow)?,',1)
old='''memory
        .reserve(
            owners,
            owners
                .len()
                .checked_add(1)
                .ok_or(EvalError::WorkspaceBoundOverflow)?,
        )
        .map_err(call_storage)?;'''
new='''let required = owners.len().checked_add(1).ok_or(EvalError::WorkspaceBoundOverflow)?;
    if required > owners.capacity() {
        let capacity = owners.capacity().checked_mul(2).ok_or(EvalError::WorkspaceBoundOverflow)?.max(required);
        memory.reserve(owners,capacity).map_err(call_storage)?;
    }'''
assert s.count(old)==2, s.count(old)
s=s.replace(old,new)
p.write_text(s)
# Freeze ordinary source diffs only; archival postimages must never overwrite source.
entries=[('crates/sparql-eval/src/property_fn_eval.rs','native-call-runtime-postimage.rs'),('crates/sparql-eval/src/error.rs','native-call-runtime-error-postimage.rs')]
patch=''
for source,draft in entries:
    before=Path(source).read_text(); after=(stage/draft).read_text()
    patch+=''.join(difflib.unified_diff(before.splitlines(True),after.splitlines(True),fromfile='a/'+source,tofile='b/'+source))
(stage/'native-call-runtime-owner-draft.patch').write_text(patch)
