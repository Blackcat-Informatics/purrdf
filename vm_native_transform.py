from pathlib import Path
p = Path('crates/sparql-eval/src/vm/compile.rs')
s = p.read_text()
Path('.stage/sparql-eval-complete-bounded-workspace/vm-compiler-before-native.rs').write_text(s)
s=s.replace('use crate::DetHashMap;', 'use purrdf_core::FastMap;\nuse purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};')
s=s.replace('    pub(super) exists: u32,\n}', '    pub(super) exists: u32,\n    owner: Option<crate::workspace::LexicalFrame>,\n}',1)
s=s.replace('struct Compiler {', "struct Compiler<'m, 's, S: Admission + ?Sized> {\n    memory: &'m mut Memory<'s, S>,").replace('slots: DetHashMap<Variable, u32>', 'slots: FastMap<Variable, u32>')
a=s.index('    /// Compile `expr`, evaluated as a term.')
b=s.index('    /// Whether the program reaches an `EXISTS`.',a)
s=s[:a]+'''    /// Compile through the original native instruction body.
    pub(crate) fn compile(expr: &Expression) -> Self {
        let mut resident = Resident;
        Self::default().compile_with_memory(expr, &mut Memory::new(&mut resident)).expect("resident expression compilation")
    }
    pub(crate) fn compile_admitted(expr: &Expression, workspace: &crate::WorkspaceCapability) -> Result<Self, crate::EvalError> {
        let mut frame = crate::workspace::LexicalFrame::new(workspace);
        let result = Memory::new(&mut frame).scope(|memory| Self::default().compile_with_memory(expr, memory));
        let mut program = result.map_err(|error| frame.storage_error(error, "expression compilation"))?;
        program.owner = Some(frame);
        Ok(program)
    }
    pub(crate) fn recompile(&mut self, expr: &Expression) {
        let mut resident = Resident;
        *self = std::mem::take(self).compile_with_memory(expr, &mut Memory::new(&mut resident)).expect("resident expression recompilation");
    }
    fn compile_with_memory<S: Admission + ?Sized>(mut self, expr: &Expression, memory: &mut Memory<'_, S>) -> Result<Self, StorageError> {
        self.ops.clear(); self.vars.clear(); self.consts.clear(); self.strs.clear(); self.calls.clear(); self.regexes.clear(); self.exists=0;
        let mut compiler = Compiler { program: self, slots: FastMap::default(), labels: purrdf_core::SmallVec::new(), memory };
        let mut work: Tasks<'_> = purrdf_core::SmallVec::new();
        work.push_with_memory(Task::Compile(expr, Mode::Term), compiler.memory)?;
        let mut next: Tasks<'_> = purrdf_core::SmallVec::new();
        while let Some(task)=work.pop() {
            match task {
                Task::Compile(expr, mode) => { compiler.expand(expr,mode,&mut next)?; while let Some(task)=next.pop() { work.push_with_memory(task,compiler.memory)?; } }
                Task::AbsentStringArg => compiler.program.ops.push_with_memory(Op::StrNone,compiler.memory)?,
                Task::Emit(op) => compiler.program.ops.push_with_memory(op,compiler.memory)?,
                Task::Label(label) => compiler.labels[label as usize]=compiler.program.ops.len() as u32,
                Task::Skip(args) => compiler.program.exists+=count_exists(args,compiler.memory)?,
            }
        }
        compiler.resolve_labels();
        work.release_with_memory(compiler.memory)?;
        next.release_with_memory(compiler.memory)?;
        compiler.labels.release_with_memory(compiler.memory)?;
        let bytes=purrdf_core::hash::hash_table_allocation_bound::<(Variable,u32)>(compiler.slots.capacity()).ok_or(StorageError::SizeOverflow)?;
        drop(compiler.slots);
        compiler.memory.release_bytes(bytes)?;
        Ok(compiler.program)
    }
'''+s[b:]
s=s.replace('impl Compiler {', "impl<S: Admission + ?Sized> Compiler<'_, '_, S> {")
s=s.replace('fn label(&mut self) -> u32 {','fn label(&mut self) -> Result<u32, StorageError> {').replace('(self.labels.len() - 1) as u32\n','Ok((self.labels.len() - 1) as u32)\n')
s=s.replace('fn slot(&mut self, var: &Variable) -> u32 {','fn slot(&mut self, var: &Variable) -> Result<u32, StorageError> {').replace('return slot;','return Ok(slot);').replace('        slot\n    }','        Ok(slot)\n    }',1)
s=s.replace('fn constant(&mut self, value: Constant) -> Op {','fn constant(&mut self, value: Constant) -> Result<Op, StorageError> {').replace('Op::Const((self.program.consts.len() - 1) as u32)','Ok(Op::Const((self.program.consts.len() - 1) as u32))')
s=s.replace('fn string(&mut self, lexical: String, language: Option<String>) -> Op {','fn string(&mut self, lexical: String, language: Option<String>) -> Result<Op, StorageError> {').replace('self.program.strs.push((lexical, language, None));','self.memory.push(&mut self.program.strs, (lexical, language, None))?;').replace('Op::StrConst((self.program.strs.len() - 1) as u32)','Ok(Op::StrConst((self.program.strs.len() - 1) as u32))')
s=s.replace('fn call(&mut self, function: &Function) -> u32 {','fn call(&mut self, function: &Function) -> Result<u32, StorageError> {').replace('self.program.calls.push(function.clone());','let function = function.clone_with_memory(self.memory)?;\n        self.program.calls.push_with_memory(function, self.memory)?;').replace('(self.program.calls.len() - 1) as u32\n','Ok((self.program.calls.len() - 1) as u32)\n')
s=s.replace('fn regex(&mut self, constant: Option<(String, String)>) -> u32 {','fn regex(&mut self, constant: Option<(String, String)>) -> Result<u32, StorageError> {').replace('self.program.regexes.push(constant);','self.memory.push(&mut self.program.regexes, constant)?;').replace('(self.program.regexes.len() - 1) as u32\n','Ok((self.program.regexes.len() - 1) as u32)\n')
s=s.replace("out: &mut Tasks<'x>) {", "out: &mut Tasks<'x>) -> Result<(), StorageError> {").replace("        out: &mut Tasks<'x>,\n    ) {", "        out: &mut Tasks<'x>,\n    ) -> Result<(), StorageError> {")
a=s.index('impl<S: Admission + ?Sized> Compiler');b=s.index('/// The string a [`Mode::StringArg`]',a)
part=s[a:b]
for target in ['self.labels','self.program.vars','self.program.consts','out']:
    pos=0
    while True:
        at=part.find(target+'.push(',pos)
        if at<0: break
        begin=at+len(target)+6;depth=1;i=begin
        while depth:
            if part[i]=='(':depth+=1
            elif part[i]==')':depth-=1
            i+=1
        replacement=target+'.push_with_memory('+part[begin:i-1]+', self.memory)?'
        part=part[:at]+replacement+part[i:];pos=at+len(replacement)
part=part.replace('let on_false = self.label();','let on_false = self.label()?;').replace('let end = self.label();','let end = self.label()?;').replace('let slot = self.slot(var);','let slot = self.slot(var)?;').replace('let call = self.call(function);','let call = self.call(function)?;')
part=part.replace('self.constant(Constant::Iri(node.clone()));','self.constant(Constant::Iri(node.clone()))?;').replace('self.constant(Constant::Literal(lit.clone()));','self.constant(Constant::Literal(lit.clone()))?;')
part=part.replace('self.slots.insert(var.clone(), slot);','let required=self.slots.len()+1;\n            purrdf_core::hash::reserve_map_with_memory(&mut self.slots,required,self.memory)?;\n            self.slots.insert(var.clone(), slot);').replace('self.slots.insert(known.clone(), index as u32);','let required=self.slots.len()+1;\n                purrdf_core::hash::reserve_map_with_memory(&mut self.slots,required,self.memory)?;\n                self.slots.insert(known.clone(), index as u32);')
part=part.replace('Mode::Term => self.expand_term(expr, out),','Mode::Term => self.expand_term(expr, out)?,').replace('Expression::FunctionCall(function, args) => self.expand_call(function, args, out),','Expression::FunctionCall(function, args) => self.expand_call(function, args, out)?,').replace('                    return;','                    return Ok(());')
part=part.replace('triple_operands(args, out);','triple_operands(args, out, self.memory)?;')
idx=part.index('fn triple_operands')
part=part[:idx]+part[idx:].replace('self.memory','memory').replace("fn triple_operands<'x>(args: &'x [Expression], out: &mut Tasks<'x>) -> Result<(), StorageError> {", "fn triple_operands<'x, S: Admission + ?Sized>(args: &'x [Expression], out: &mut Tasks<'x>, memory: &mut Memory<'_, S>) -> Result<(), StorageError> {")
part=part.replace("let string_args = |count: usize, out: &mut Tasks<'x>| {", "let string_args = |count: usize, out: &mut Tasks<'x>, memory: &mut Memory<'_, S>| -> Result<(), StorageError> {")
idx=part.index('let string_args =');end=part.index('        match function',idx)
part=part[:idx]+part[idx:end].replace('self.memory','memory').replace('        };','            Ok(())\n        };')+part[end:]
part=part.replace('string_args(2, out);','string_args(2, out, self.memory)?;').replace('string_args(if flags { 3 } else { 2 }, out);','string_args(if flags { 3 } else { 2 }, out, self.memory)?;')
for needle in ['    fn expand<','    fn expand_term<','    fn expand_call<','fn triple_operands<']:
    idx=part.index(needle);brace=part.index('{',idx);depth=1;i=brace+1
    while depth:
        if part[i]=='{':depth+=1
        elif part[i]=='}':depth-=1
        i+=1
    part=part[:i-1]+'    Ok(())\n'+part[i-1:]
s=s[:a]+part+s[b:]
p.write_text(s)
