from pathlib import Path
import difflib
stage=Path('.stage/sparql-eval-complete-bounded-workspace'); source=Path('crates/lex/src/walk.rs'); old=source.read_text()
before='''        let bytes = core::alloc::Layout::new::<T>().size();
        memory.add_bytes(bytes)?;
        match crate::allocation::try_boxed(value) {
            Ok(value) => Ok(Self::from(value)),
            Err(_) => {
                memory.release_bytes(bytes)?;
                Err(crate::allocation::StorageError::AllocationFailed)
            }
        }'''
assert old.count(before)==1
new=old.replace(before,'        memory.boxed(value).map(Self::from)',1)
(stage/'native-box-home-owner-draft.patch').write_text(''.join(difflib.unified_diff(old.splitlines(True),new.splitlines(True),fromfile='a/'+str(source),tofile='b/'+str(source))))
