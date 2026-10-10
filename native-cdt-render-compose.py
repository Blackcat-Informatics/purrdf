from pathlib import Path
import difflib
root=Path(__file__).resolve().parents[2];stage=Path(__file__).resolve().parent
p='crates/cdt/src/render.rs';s=(root/p).read_text();new=s
new=new.replace('use crate::memory::{Memory, Resident, StorageError};','use crate::memory::{Memory, Resident, Storage, StorageError};')
a=new.index('pub fn canonical_lexical(value:');b=new.index('/// The **byte length**',a)
new=new[:a]+'''pub fn canonical_lexical(value: &CdtValue) -> String {
    try_canonical_lexical(value, &mut Resident).expect("resident renderer capacity").0
}

/// Render the original canonical spelling under caller-owned admission.
/// The returned byte count belongs to the caller's original storage grant.
/// # Errors
/// Returns original admission, allocator or checked-layout refusal.
pub fn try_canonical_lexical(value: &CdtValue, storage: &mut impl Storage) -> Result<(String, usize), StorageError> {
    let mut memory = Memory::new(storage);
    let expected = canonical_lexical_len(value);
    let mut out = FixedText { text: String::new(), failed: false };
    memory.reserve_string(&mut out.text, expected)?;
    let mut jobs = Vec::new();
    push_value(&mut jobs, value, &mut memory)?;
    run(&mut out, jobs, &mut memory)?;
    if out.failed || out.text.len() != expected {
        return Err(StorageError::FormattingFailed);
    }
    Ok((out.text, memory.live_bytes()))
}

/// The immutable value's exact canonical extent is admitted before rendering.
/// The same emitter writes into this checked destination; a broken extent can
/// never trigger an unpriced String reallocation or publish partial output.
struct FixedText { text: String, failed: bool }
impl core::fmt::Write for FixedText {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        if self.failed || self.text.capacity().saturating_sub(self.text.len()) < text.len() {
            self.failed = true;
            return Err(core::fmt::Error);
        }
        self.text.push_str(text);
        Ok(())
    }
}
impl TextOut for FixedText {
    fn push_str(&mut self, text: &str) { let _ = core::fmt::Write::write_str(self, text); }
    fn push(&mut self, ch: char) {
        let mut bytes = [0; 4];
        TextOut::push_str(self, ch.encode_utf8(&mut bytes));
    }
    fn failed(&self) -> bool { self.failed }
}
impl Sink for FixedText {
    fn composite<'a>(&mut self, jobs: &mut Vec<Job<'a>>, value: &'a CdtValue, memory: &mut Memory<'_>) -> Result<(), StorageError> {
        push_value(jobs, value, memory)
    }
}

'''+new[b:]
(stage/'native-cdt-render-owner-draft.patch').write_text(''.join(difflib.unified_diff(s.splitlines(True),new.splitlines(True),fromfile='a/'+p,tofile='b/'+p)))
(stage/'render-native-cdt-postimage.rs').write_text(new)
