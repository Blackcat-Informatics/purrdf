from pathlib import Path
import difflib
root=Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
stage=root/'.stage/sparql-eval-complete-bounded-workspace'
changes={}
p='crates/rdf-core/src/xsd_regex/xpath/pike.rs';a=(root/p).read_text();b=a
start=b.index('        let (empty, nullable) = match nodes[node] {');end=b.index('\n        links[node].empty = empty;',start)
old=b[start:end]
body=old[old.index('match nodes[node] {'):]
body=body.replace('match nodes[node]', 'match node').rstrip(';')
import re
body=re.sub(r'links\[(left|right|body)\]\.empty',r'child(\1).0',body)
body=re.sub(r'links\[(left|right|body)\]\.nullable',r'child(\1).1',body)
helper='''/// The one empty/nullable property law shared by construction and link analysis.
/// Children are supplied from their original already-built arena metadata.
pub(super) fn node_properties(node: Node, child: impl Fn(usize) -> (bool, bool)) -> (bool, bool) {
    '''+body+'''
}
'''
b=b[:start]+'''        let (empty, nullable) = node_properties(nodes[node], |child| (links[child].empty, links[child].nullable));'''+b[end:]
pos=b.index('pub(super) fn analyze(')
b=b[:pos]+helper+'\n'+b[pos:]
changes[p]=(a,b)
p='crates/rdf-core/src/xsd_regex/xpath/compile.rs';a=(root/p).read_text();b=a
# Exactly one transient property pair per original arena node; all physical
# growth is native and the table dies before the compiled artifact survives.
b=b.replace('        nodes: Vec::new(),','        nodes: Vec::new(),\n        properties: Vec::new(),',1)
b=b.replace('        frames,\n        ..\n    } = parser;','        frames,\n        properties,\n        ..\n    } = parser;',1)
b=b.replace('    budget.release_vec(frames)?;','    budget.release_vec(frames)?;\n    budget.release_vec(properties)?;',1)
marker="struct Parser<'a, 'storage> {"
pos=b.index(marker);before=b[:pos];after=b[pos:]
after=after.replace('    nodes: Vec<Node>,','    nodes: Vec<Node>,\n    properties: Vec<(bool, bool)>,',1)
b=before+after
b=b.replace('''        let id = self.nodes.len();
        self.nodes.push(node);
        Ok(id)''','''        let properties = super::pike::node_properties(node, |child| self.properties[child]);
        grow(&mut self.budget, &mut self.properties, 1)?;
        let id = self.nodes.len();
        self.nodes.push(node);
        self.properties.push(properties);
        Ok(id)''',1)
old='&& min == Count::Finite(0) && max.is_none() {'
assert old in b
b=b.replace(old,'''&& min == Count::Finite(0) && max.is_none() && self.properties[body].1 {
            // Only a nullable body needs the optional-plus lowering. Giving a
            // nonnullable star two split points changes nested reluctant capture
            // priority even though it accepts the same whole-match strings.''',1)
changes[p]=(a,b)
patch=''.join(''.join(difflib.unified_diff(a.splitlines(True),b.splitlines(True),fromfile='a/'+p,tofile='b/'+p)) for p,(a,b) in changes.items())
(stage/'native-regex-star-priority-owner-draft.patch').write_text(patch)
for p,(a,b) in changes.items():
    dest=stage/'native-regex-star-priority-postimages'/p;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(b)
print(len(patch.splitlines()),'patch lines')
