# Stage-only proposal assembler. No shipping edits, builds or tests.
from pathlib import Path
import difflib

root = Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
stage = root / '.stage/sparql-eval-complete-bounded-workspace'
post = stage / 'expand-uchars-owner-postimages'
items = []

path = 'crates/lex/src/allocation.rs'
before = (root / path).read_text()
anchor = '/// Exact live-buffer accounting shared by native scanners, compilers and walks.'
assert before.count(anchor) == 1
resident = '''/// A zero-size resident callback with no application capacity preset.
///
/// Accepts each exact native live layout; Memory still checks the layout and
/// performs the actual allocation fallibly. This supplies no borrowed external
/// grant, so resident callers own the returned payload themselves.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Resident;

impl Admission for Resident {
    fn resize(&mut self, _live_bytes: usize) -> Result<(), StorageError> { Ok(()) }
}

'''
after = before.replace(anchor, resident + anchor)
anchor = '    /// Append one scalar after admitting any physical UTF-8 buffer growth.'
assert after.count(anchor) == 1
append = '''    /// Append borrowed UTF-8 after admitting any amortized buffer growth.
    /// The original text survives admission or allocator refusal unchanged.
    /// Its nonempty buffer must already belong to this memory's live total.
    ///
    /// # Errors
    /// Returns checked layout overflow, admission refusal or allocator refusal.
    pub fn push_str(&mut self, text: &mut String, suffix: &str) -> Result<(), StorageError> {
        let required = text.len().checked_add(suffix.len()).ok_or(StorageError::SizeOverflow)?;
        if required > text.capacity() {
            let capacity = text.capacity().checked_mul(2)
                .ok_or(StorageError::SizeOverflow)?.max(required);
            self.reserve_string(text, capacity)?;
        }
        text.push_str(suffix);
        Ok(())
    }

'''
after = after.replace(anchor, append + anchor)
start = after.index('    pub fn push_char(')
end = after.index('    /// Render borrowed native fields once', start)
after = after[:start] + '''    pub fn push_char(&mut self, text: &mut String, ch: char) -> Result<(), StorageError> {
        let mut utf8 = [0; 4];
        self.push_str(text, ch.encode_utf8(&mut utf8))
    }

''' + after[end:]
start = after.index('        let result = (|| {', after.index("impl<S: Admission + ?Sized> fmt::Write for FormatSink"))
end = after.index('        result.map_err', start)
after = after[:start] + '        let result = self.memory.push_str(self.text, text);\n' + after[end:]
items.append((path, before, after))

path = 'crates/lex/src/terminals.rs'
before = (root / path).read_text()
start = before.index('pub fn expand_uchars(text: &str)')
end = before.index('/// The scalar an XML character reference', start)
wrapper = '''pub fn expand_uchars(text: &str) -> Cow<'_, str> {
    let mut resident = crate::allocation::Resident;
    let mut memory = crate::allocation::Memory::new(&mut resident);
    expand_uchars_with_memory(text, &mut memory)
        .expect("resident UCHAR expansion allocation failed")
}

'''
after = before[:start] + wrapper + (stage / 'expand-uchars-owner-draft.rs').read_text() + '\n\n' + before[end:]
items.append((path, before, after))

path = 'crates/rdf-core/tests/native_regex_storage.rs'
before = (root / path).read_text()
after = before + '\n' + (stage / 'expand-uchars-owner-fixtures.rs').read_text()
items.append((path, before, after))

out = []
for path, before, after in items:
    target = post / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(after)
    out.append('diff --git a/' + path + ' b/' + path + '\n')
    out.extend(difflib.unified_diff(before.splitlines(True), after.splitlines(True),
        fromfile='a/' + path, tofile='b/' + path))
(stage / 'expand-uchars-owner-draft.patch').write_text(''.join(out))
print('Wrote same-body decoder/Resident/actual-allocation fixture proposal.')
