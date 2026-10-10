from pathlib import Path
import difflib
root = Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
stage = root/'.stage/sparql-eval-complete-bounded-workspace'
base = (root/'crates/sparql-eval/src/modifier.rs').read_text()
assert 'AggregateScalarvals' in base, 'apply the complete custom source packet first'
text = base
start = text.index('pub(crate) trait GroupDomain: Copy + Sync')
end = text.index('// Out of line by design:', start)
text = text[:start] + '''pub(crate) trait GroupDomain: Copy + Sync {
    const CONTEXTUAL: bool;
    fn visible(
        &self,
        schema: &VarSchema,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<Option<AdmittedVec<usize>>, EvalError>;
}

impl GroupDomain for () {
    const CONTEXTUAL: bool = false;
    fn visible(&self, schema: &VarSchema, workspace: &crate::WorkspaceCapability)
        -> Result<Option<AdmittedVec<usize>>, EvalError> {
        crate::blank_scope::visible_columns_admitted(schema, workspace)
    }
}

impl GroupDomain for &[Variable] {
    const CONTEXTUAL: bool = true;
    fn visible(&self, schema: &VarSchema, workspace: &crate::WorkspaceCapability)
        -> Result<Option<AdmittedVec<usize>>, EvalError> {
        let mut columns = AdmittedVec::new(workspace);
        for name in self.iter() {
            if let Some(column) = schema.index_of(name) { columns.push(column)?; }
        }
        Ok(Some(columns))
    }
}

''' + text[end:]
assert text.count('let visible = domain.visible(&in_schema);') == 1
text = text.replace('let visible = domain.visible(&in_schema);', 'let visible = domain.visible(&in_schema, &ctx.growth)?;')
assert text.count('visible.as_ref().map_or(row.len(), Vec::len)') == 1
text = text.replace('visible.as_ref().map_or(row.len(), Vec::len)', 'visible.as_ref().map_or(row.len(), |columns| columns.len())')
assert text.count('let visible = agg.distinct.then(|| domain.visible(schema)).flatten();') == 1
text = text.replace('let visible = agg.distinct.then(|| domain.visible(schema)).flatten();',
    'let visible = if agg.distinct { domain.visible(schema, &ctx.growth)? } else { None };')
text += '\n' + (stage/'custom-contextual-owner-fixture.rs').read_text()
(stage/'modifier-visibility-native-baseline.rs').write_text(base)
(stage/'modifier-visibility-native-postimage.rs').write_text(text)

target = root/'crates/sparql-eval/src/blank_scope.rs'
base = target.read_text()
text = base
start = text.index('pub(crate) fn visible_columns(schema: &VarSchema)')
end = text.index('/// `seq` without', start)
text = text[:start] + '''pub(crate) fn visible_columns(schema: &VarSchema) -> Option<Vec<usize>> {
    visible_columns_admitted(schema, &crate::WorkspaceCapability::resident())
        .expect("resident visible-column allocation")
        .map(|columns| columns.try_into_resident().expect("resident visible-column owner"))
}

/// The same hidden-column identity law, with actual admitted array growth.
pub(crate) fn visible_columns_admitted(
    schema: &VarSchema,
    workspace: &crate::WorkspaceCapability,
) -> Result<Option<AdmittedVec<usize>>, crate::EvalError> {
    if !schema.vars().iter().any(is_joined_blank) { return Ok(None); }
    let mut columns = AdmittedVec::new(workspace);
    for (column, variable) in schema.vars().iter().enumerate() {
        if !is_joined_blank(variable) { columns.push(column)?; }
    }
    Ok(Some(columns))
}

''' + text[end:]
(stage/'blank-scope-visibility-native-baseline.rs').write_text(base)
(stage/'blank-scope-visibility-native-postimage.rs').write_text(text)
