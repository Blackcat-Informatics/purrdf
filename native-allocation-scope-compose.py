from pathlib import Path
import difflib
root = Path(__file__).resolve().parents[2]
stage = Path(__file__).resolve().parent
patches = []
def add(relative, before, after):
    patches.append(''.join(difflib.unified_diff(before.splitlines(True), after.splitlines(True), fromfile='a/'+relative, tofile='b/'+relative)))

relative = 'crates/lex/src/allocation.rs'
before = root.joinpath(relative).read_text()
point = before.index('    /// Borrow the original admission')
addition = '''    /// Run a native child body under this same original admission. On physical
    /// failure, the body's owned locals die before their original grant delta is
    /// released. Successful returned payloads keep their original grant unchanged.
    /// This never observes or adopts a resident factory's returned payload.
    ///
    /// # Errors
    /// Returns the body's original storage refusal or a cleanup invariant failure.
    pub fn scope<T>(
        &mut self,
        body: impl FnOnce(&mut Self) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        let baseline = self.live;
        let result = body(self);
        if result.is_err() {
            let released = self.live.checked_sub(baseline).ok_or(StorageError::SizeOverflow)?;
            self.release_bytes(released)?;
        }
        result
    }

'''
add(relative, before, before[:point]+addition+before[point:])

relative = 'crates/cdt/src/ops.rs'
before = root.joinpath(relative).read_text()
after = before.replace('    drop(admit);\n', '')
start = after.index('fn relation_scope<T>')
end = after.index('// ── Equality', start)
after = after[:start] + after[end:]
after = after.replace('relation_scope(memory, |memory| term_equal_kernel(a, b, memory))', 'memory.scope(|memory| term_equal_kernel(a, b, memory))')
after = after.replace('relation_scope(memory, |memory| sequence_less_than_kernel(left, right, memory))', 'memory.scope(|memory| sequence_less_than_kernel(left, right, memory))')
after = after.replace('Self::Owned(CdtTerm::Composite(value)) => {\n                let parts = (*value).into_parts();\n                memory.release_bytes', 'Self::Owned(CdtTerm::Composite(mut value)) => {\n                let inner = mem::replace(&mut *value, CdtValue::empty_list());\n                drop(value);\n                memory.release_bytes')
after = after.replace('match parts { CdtParts::List(items)', 'match inner.into_parts() { CdtParts::List(items)')
after = after.replace('CdtTerm::Composite(value) => { let value = *value; memory.release_bytes(Layout::new::<CdtValue>().size())?; Ok(Self::parsed(value)) },', 'CdtTerm::Composite(mut value) => {\n                let inner = mem::replace(&mut *value, CdtValue::empty_list());\n                drop(value);\n                memory.release_bytes(Layout::new::<CdtValue>().size())?;\n                Ok(Self::parsed(inner))\n            },')
add(relative, before, after)

relative = 'crates/cdt/src/render.rs'
before = root.joinpath(relative).read_text()
start = before.index('pub fn try_canonical_lexical(')
end = before.index('\n/// The immutable value', start)
replacement = '''pub fn try_canonical_lexical(value: &CdtValue, storage: &mut impl Storage) -> Result<(String, usize), StorageError> {
    let mut memory = Memory::new(storage);
    memory.scope(|memory| {
        let expected = canonical_lexical_len(value);
        let mut out = FixedText { text: String::new(), failed: false };
        memory.reserve_string(&mut out.text, expected)?;
        let mut jobs = Vec::new();
        push_value(&mut jobs, value, memory)?;
        run(&mut out, jobs, memory)?;
        if out.failed || out.text.len() != expected { return Err(StorageError::FormattingFailed); }
        Ok((out.text, memory.live_bytes()))
    })
}
'''
after = before[:start] + replacement + before[end:]
start = after.index('pub fn canonical_key_lexical(')
end = after.index('\n/// The byte length one element', start)
replacement = '''pub fn canonical_key_lexical(key: &CdtKey) -> String {
    try_canonical_key_lexical(key, &mut Memory::new(&mut Resident)).expect("resident canonical key storage")
}

pub(crate) fn try_canonical_key_lexical(key: &CdtKey, memory: &mut Memory<'_>) -> Result<String, StorageError> {
    memory.scope(|memory| {
        let length = key_lexical_len(key);
        let mut output = FixedText { text: String::new(), failed: false };
        memory.reserve_string(&mut output.text, length)?;
        write_key(&mut output, key);
        if output.failed || output.text.len() != length { return Err(StorageError::FormattingFailed); }
        Ok(output.text)
    })
}
'''
after = after[:start] + replacement + after[end:]
start = after.index('impl Sink for String {')
end = after.index('/// A [`Sink`] that materialises nothing', start)
after = after[:start] + after[end:]
after = after.replace('implementations — [`String`], which materialises the form,', 'implementations — [`FixedText`], which materialises the form,')
add(relative, before, after)
stage.joinpath('native-allocation-original-scope.patch').write_text(''.join(patches))
print('original allocation failure scope and renderer packet ready')
