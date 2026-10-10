from pathlib import Path
import difflib
root = Path(__file__).resolve().parents[2]
stage = Path(__file__).resolve().parent
path = root / 'crates/cdt/src/render.rs'
before = path.read_text()
point = before.index('/// The byte length one map key occupies')
addition = '''/// Measure the original triple renderer before its checked box is born.
pub(crate) fn try_triple_lexical_len(
    triple: &CdtTripleTerm,
    memory: &mut Memory<'_>,
) -> Result<usize, StorageError> {
    let mut output = Measure(0);
    let mut jobs = Vec::new();
    push_triple(&mut jobs, triple, memory)?;
    run(&mut output, jobs, memory)?;
    Ok(output.0)
}

'''
after = before[:point] + addition + before[point:]
stage.joinpath('native-cdt-triple-measure.patch').write_text(''.join(difflib.unified_diff(before.splitlines(True), after.splitlines(True), fromfile='a/crates/cdt/src/render.rs', tofile='b/crates/cdt/src/render.rs')))
print('native triple measure packet ready')
