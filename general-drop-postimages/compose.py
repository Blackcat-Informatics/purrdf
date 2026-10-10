from pathlib import Path
stage=Path(__file__).parent
p=stage/'crates/sparql-algebra/src/owned.rs'
s=p.read_text()
a=s.index('/// How many dismantled nodes')
b=s.index('/// An owned node',a)
s=s[:a]+s[b:]
a=s.index('/// Dismantle `work`')
b=s.index('/// The finished copies',a)
s=s[:a]+(stage/'drop_body.rs').read_text()+'\n'+s[b:]
s=s.replace('//! Owned nodes on a work list: the iterative drop, and the iterative clone that\n//! builds a copy bottom-up.','//! Original owners for allocation-free drop and bottom-up iterative cloning.')
p.write_text(s)
