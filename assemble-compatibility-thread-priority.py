# Stage-only concrete correction over the shared native Pike body.
from pathlib import Path
import difflib

stage = Path(__file__).resolve().parent
root = stage.parents[1]
path = "crates/rdf-core/src/xsd_regex/xpath/pike.rs"
before = (root / path).read_text()
old = '''    /// The match that starts exactly at `start`, which the set machine found
    /// a match start: one thread follows the first control state, in priority
    /// order, that can still complete a match.
    pub(super) fn walk(&mut self, start: usize) -> Result<Option<Captures>, Error> {
        self.mode = Mode::Walk;
        self.run(start, Self::anchored)
    }'''
new = '''    /// Complete the leftmost start certified by the capture-free set machine.
    /// Dated matching follows one viable control state. Compatibility retains
    /// every earlier thread's epsilon closure: its first capture tags must merge
    /// later nullable-loop paths even when those earlier paths are not the one
    /// the capture-free future would select to consume the next character.
    pub(super) fn walk(&mut self, start: usize) -> Result<Option<Captures>, Error> {
        if self.ctx.program.law.is_compatibility() {
            return self.find_from(start);
        }
        self.mode = Mode::Walk;
        self.run(start, Self::anchored)
    }'''
assert before.count(old) == 1
after = before.replace(old, new, 1)
target = stage / "compatibility-thread-priority-postimages" / path
target.parent.mkdir(parents=True, exist_ok=True)
target.write_text(after)
patch = "".join(difflib.unified_diff(
    before.splitlines(keepends=True), after.splitlines(keepends=True),
    fromfile="a/" + path, tofile="b/" + path,
))
(stage / "compatibility-thread-priority-draft.patch").write_text(patch)
print("Wrote one-home Stage-only standard priority correction; oracle unchanged.")
