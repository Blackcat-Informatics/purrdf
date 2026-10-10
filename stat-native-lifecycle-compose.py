from pathlib import Path
import difflib
root = Path(__file__).resolve().parents[2]
stage = Path(__file__).resolve().parent
relative = 'crates/sparql-eval/src/agg_fn.rs'
before = root.joinpath(relative).read_text()
after = before
point = after.index('    /// Fold one row\'s already-evaluated', after.index('pub trait AggregateAccumulator'))
after = after[:point] + '''    /// A first-party native accumulator retains the capability used by its
    /// factory. Opaque implementations keep the default and must override the
    /// admitted lifecycle themselves before serving bounded queries.
    fn native_workspace(&self) -> Option<&crate::WorkspaceCapability> { None }

''' + after[point:]
point = after.index('        if workspace.is_bounded()', after.index('    fn step_admitted('))
after = after[:point] + after[point:].replace('if workspace.is_bounded() {', 'if workspace.is_bounded() && !self.native_workspace().is_some_and(crate::WorkspaceCapability::is_bounded) {', 1)
point = after.index('    /// Recover this accumulator\'s original concrete type')
after = after[:point] + '''    /// Merge through the original native state owners.
    /// # Errors
    /// Refuses an opaque bounded merge before its body executes.
    fn combine_admitted(&mut self, other: Box<dyn AggregateAccumulator>, workspace: &crate::WorkspaceCapability) -> Result<(), EvalError> {
        if workspace.is_bounded() && !self.native_workspace().is_some_and(crate::WorkspaceCapability::is_bounded) {
            return Err(EvalError::WorkspaceUnpriced("an aggregate without admitted state merging"));
        }
        self.combine(other)
    }

''' + after[point:]
point = after.index('\n}\n\n/// A host-injected custom aggregate')
after = after[:point] + '''
    /// The native absorbing publication boundary retains original output grants.
    /// # Errors
    /// Returns physical refusal or the original aggregate's operational error.
    fn finish_absorbing_admitted(self: Box<Self>, absorb: &mut dyn FnMut(purrdf_xsd::ErrorCode), workspace: &crate::WorkspaceCapability) -> Result<Option<crate::WorkspaceTerm>, EvalError> {
        if workspace.is_bounded() { return self.finish_admitted(workspace); }
        self.finish_absorbing(absorb).map(|value| value.map(crate::WorkspaceTerm::resident))
    }
''' + after[point:]
point = after.rfind('\n}', after.index('pub trait CustomAggregate'), after.index('/// The fixed diagnostic label'))
after = after[:point] + '''
    /// Begin a native invocation whose box and state retain original admission.
    /// # Errors
    /// Refuses an opaque bounded factory before calling resident host code.
    fn init_admitted(&self, scalarvals: &[(String, TermValue)], division: purrdf_xsd::exact::DivisionPolicy, workspace: &crate::WorkspaceCapability) -> Result<WorkspaceAccumulator, EvalError> {
        if workspace.is_bounded() { return Err(EvalError::WorkspaceUnpriced("an aggregate without an admitted factory")); }
        Ok(WorkspaceAccumulator::resident(self.init_under(scalarvals, division)))
    }
    /// Price the original arithmetic while admitting any cost-analysis scratch.
    /// # Errors
    /// Refuses an opaque bounded cost producer before its body executes.
    fn exact_numeric_cost_admitted(&self, survivors: &[Vec<TermValue>], scalarvals: &[(String, TermValue)], division: purrdf_xsd::exact::DivisionPolicy, workspace: &crate::WorkspaceCapability) -> Result<purrdf_xsd::exact::Cost, EvalError> {
        if workspace.is_bounded() { return Err(EvalError::WorkspaceUnpriced("an aggregate without admitted cost analysis")); }
        Ok(self.exact_numeric_cost_under(survivors, scalarvals, division))
    }
''' + after[point:]
point = after.index('/// The fixed diagnostic label')
after = after[:point] + stage.joinpath('stat-native-lifecycle.rs').read_text() + '\n' + after[point:]
stage.joinpath('agg_fn-stat-native-postimage.rs').write_text(after)
stage.joinpath('stat-native-lifecycle-draft.patch').write_text(''.join(difflib.unified_diff(before.splitlines(True), after.splitlines(True), fromfile='a/'+relative, tofile='b/'+relative)))
print('native custom lifecycle postimage durable')
