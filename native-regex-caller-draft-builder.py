# Stage-only source proposal assembler. Never writes shipping files.
from pathlib import Path
import difflib

root = Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
stage = root / '.stage/sparql-eval-complete-bounded-workspace'
post = stage / 'native-regex-caller-postimages'
post.mkdir(exist_ok=True)
images = {}

def read(name):
    return (root / name).read_text()

def change(text, old, new):
    assert text.count(old) == 1, (old[:100], text.count(old))
    return text.replace(old, new, 1)

name = 'crates/rdf-core/src/small/shared.rs'
source = read(name)
start = source.index('    /// Allocate one owner after')
end = source.index('    /// Retain the existing allocation', start)
body = '''    /// Allocate one owner after its caller has admitted this concrete layout.
    ///
    /// # Errors
    /// Returns allocation refusal without constructing diagnostic text.
    pub fn try_new(value: T) -> Result<Self, SharedAllocError> {
        Self::try_new_with(|| value)
    }

    /// Allocate the actual control storage before invoking a payload factory.
    /// The factory is never called on allocation refusal. Its final ownership
    /// transfer must be infallible; a panic releases uninitialized storage.
    ///
    /// # Errors
    /// Returns allocator refusal while captured original payloads remain owned
    /// by the factory. No payload is removed before its destination exists.
    pub fn try_new_with(factory: impl FnOnce() -> T) -> Result<Self, SharedAllocError> {
        // SAFETY: alloc supplies null or global-allocator storage for this layout.
        unsafe { Self::try_new_with_allocator(factory, |layout| std::alloc::alloc(layout)) }
    }

    /// Internal allocation boundary, also exercised by null-allocator tests.
    ///
    /// # Safety
    /// The allocator returns null, or fresh global-allocator storage for exactly
    /// the supplied nonzero layout. It must not return an alias or other storage.
    unsafe fn try_new_with_allocator(
        factory: impl FnOnce() -> T,
        allocate: impl FnOnce(Layout) -> *mut u8,
    ) -> Result<Self, SharedAllocError> {
        let layout = Self::allocation_layout();
        let pointer = NonNull::new(allocate(layout).cast::<Header<T>>()).ok_or(SharedAllocError)?;
        // Keep the still-uninitialized destination under a deallocation guard
        // while arbitrary caller code runs. No T drop can run on this storage.
        let pending = UninitializedAllocation { pointer: pointer.cast(), layout };
        let value = factory();
        // SAFETY: the allocation contract provides aligned, fresh storage;
        // no reference is exposed before this initialization.
        unsafe { pointer.as_ptr().write(Header { strong: AtomicUsize::new(1), value }) };
        core::mem::forget(pending);
        Ok(Self { pointer, _owner: PhantomData })
    }

'''
source = source[:start] + body + source[end:]
marker = '/// Allocation refusal while constructing an immutable shared owner.'
guard = '''// Only the uninitialized allocation is owned. The initialized Header<T>
// transfers directly to Shared; unwinding a factory cannot leak this block.
struct UninitializedAllocation {
    pointer: NonNull<u8>,
    layout: Layout,
}
impl Drop for UninitializedAllocation {
    fn drop(&mut self) {
        // SAFETY: this guard owns fresh global-allocator storage for this layout.
        unsafe { std::alloc::dealloc(self.pointer.as_ptr(), self.layout) };
    }
}

'''
source = change(source, marker, guard + marker)
source = change(source,
    '        // SAFETY: this was the last owner. The initialized allocation came from\n        // the global allocator with Layout::new::<Header<T>>. Box drops T and\n        // deallocates it, including if T\'s destructor unwinds.\n        unsafe { drop(Box::from_raw(self.pointer.as_ptr())) };',
    '        // SAFETY: this was the last owner. Move its unpinned T onto the stack\n        // before freeing the concrete control block. A payload-held workspace\n        // grant therefore remains live through the original header deallocation.\n        let value = unsafe { core::ptr::addr_of!((*self.pointer.as_ptr()).value).read() };\n        // SAFETY: this header came from the global allocator for this layout,\n        // and no live owner or payload remains in it. Deallocation precedes any\n        // T destructor, including one that releases admission or unwinds.\n        unsafe { std::alloc::dealloc(self.pointer.as_ptr().cast(), Self::allocation_layout()) };\n        drop(value);')
source = change(source,
    'unsafe { Shared::try_new_with(DropProbe(drops.clone()), |_| core::ptr::null_mut()) }',
    'unsafe { Shared::try_new_with_allocator(|| DropProbe(drops.clone()), |_| core::ptr::null_mut()) }')
# The old test's captured value must exist before invoking the factory, so refusal
# drops that original value once without constructing a new one.
source = change(source,
    '        // SAFETY: null satisfies the allocation boundary\'s refusal contract.\n        let result =\n            unsafe { Shared::try_new_with_allocator(|| DropProbe(drops.clone()), |_| core::ptr::null_mut()) };',
    '        let original = DropProbe(drops.clone());\n        // SAFETY: null satisfies the allocation boundary\'s refusal contract.\n        let result =\n            unsafe { Shared::try_new_with_allocator(|| original, |_| core::ptr::null_mut()) };')
tests = '''    #[test]
    fn allocation_refusal_never_invokes_payload_factory() {
        let invoked = core::cell::Cell::new(false);
        // SAFETY: null satisfies the exact allocation-refusal contract.
        let result = unsafe { Shared::try_new_with_allocator(|| {
            invoked.set(true);
            7u8
        }, |_| core::ptr::null_mut()) };
        assert!(matches!(result, Err(SharedAllocError)));
        assert!(!invoked.get());
    }

    #[test]
    fn factory_constructs_once_and_shared_clones_do_not_repeat_it() {
        let invoked = core::cell::Cell::new(0usize);
        let value = Shared::try_new_with(|| { invoked.set(invoked.get() + 1); 7u8 }).unwrap();
        let clone = value.clone();
        assert_eq!(invoked.get(), 1);
        assert!(core::ptr::eq(&*value, &*clone));
    }

'''
source = change(source, '    #[test]\n    fn shallow_clone_keeps_original_payload_live()', tests + '    #[test]\n    fn shallow_clone_keeps_original_payload_live()')
images[name] = source

name = 'crates/sparql-eval/src/workspace.rs'
source = read(name)
start = source.index('    /// Borrow a value without constructing or copying an owned key.', source.index('impl<K: Eq + std::hash::Hash, V> AdmittedMap'))
end = source.index('    /// Insert or replace after admitting', start)
source = source[:start] + (stage / 'native-regex-map-access-draft.rs').read_text() + '\n' + source[end:]
images[name] = source

name = 'crates/sparql-eval/src/plan_cache.rs'
source = read(name)
source = source.replace('use std::collections::BTreeMap;\n', '').replace('use crate::DetHashMap;\n', '')
start = source.index('#[derive(Debug)]\nstruct Entry')
end = source.index('#[cfg(test)]', start)
source = source[:start] + (stage / 'native-regex-cache-body-draft.rs').read_text() + '\n' + source[end:]
images[name] = source

name = 'crates/sparql-eval/src/xpath_regex.rs'
original = read(name)
tests = original[original.index('#[cfg(all(test, not(target_arch = "wasm32")))]'):]
import re
tests = re.sub(r'\.is_match\(("[^"\\]*(?:\\.[^"\\]*)*")\)', r'.is_match(\1, &WorkspaceCapability::resident())', tests)
tests = change(tests,
    '        let different = Arc::new(xpath::compile(Profile::Xpath20, "z", "", Limits::new()).unwrap());',
    '        let different = compile_owned(Profile::Xpath20, "z", "", Limits::new(), &ctx.growth).unwrap().unwrap();')
tests = change(tests,
    '            .insert(key, Arc::clone(&different), different.storage_bytes());',
    '            .insert_admitted(key, different.clone(), different.storage.live_bytes(), &ctx.growth).unwrap();')
source = (stage / 'native-regex-program-owner-draft.rs').read_text()
source = source.replace('// Integration body for the existing evaluator xpath_regex home. Not compiled.', '//! Selected pattern laws at the SPARQL expression boundary.')
images[name] = source + '\n' + tests

name = 'crates/sparql-eval/src/expr.rs'
source = read(name)
source = change(source,
    '    let Some(replaced) = compiled.replace_all(&s, replacement)? else {\n        return Ok(None);\n    };\n    let replaced = replaced.into_owned();\n    make_string_dir(ctx, replaced, lang.map(str::to_owned), dir)',
    '    let Some(replaced) = compiled.replace_literal(s, replacement, lang, dir, &ctx.growth)? else {\n        return Ok(None);\n    };\n    ctx.intern_workspace_term(replaced)')
images[name] = source

name = 'crates/sparql-eval/src/vm/mod.rs'
source = read(name)
source = change(source, '.is_match(text.parts().0)?', '.is_match(text.parts().0, &ctx.growth)?')
images[name] = source

patches = []
for name, after in images.items():
    before = read(name)
    destination = post / name
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(after)
    patches.append(''.join(difflib.unified_diff(before.splitlines(True), after.splitlines(True),
        fromfile='a/' + name, tofile='b/' + name)))
(stage / 'native-regex-caller-owner-draft.patch').write_text(''.join(patches))
print('Wrote Stage-only six-home standard proposal:', stage / 'native-regex-caller-owner-draft.patch')
