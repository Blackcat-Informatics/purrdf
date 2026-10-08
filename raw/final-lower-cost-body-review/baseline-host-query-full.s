core::ptr::drop_glue::<<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}::{closure#0}>:
.Lfunc_begin463:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception463
	pushq	%r14
	.cfi_def_cfa_offset 16
	pushq	%rbx
	.cfi_def_cfa_offset 24
	pushq	%rax
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -24
	.cfi_offset %r14, -16
	movq	%rdi, %rbx
.Ltmp6557:
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>
.Ltmp6558:
	movq	48(%rbx), %rsi
	cmpq	$-1, %rsi
	je	.LBB898_4
	testq	%rsi, %rsi
	je	.LBB898_4
	movq	56(%rbx), %rdi
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBB898_4:
	leaq	72(%rbx), %rdi
	callq	core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
	addq	$24, %rbx
	movq	%rbx, %rdi
	addq	$8, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmp	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.LBB898_5:
	.cfi_def_cfa_offset 32
.Ltmp6559:
	movq	48(%rbx), %rsi
	movq	%rax, %r14
	cmpq	$-1, %rsi
	je	.LBB898_8
	testq	%rsi, %rsi
	je	.LBB898_8
	movq	56(%rbx), %rdi
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBB898_8:
	leaq	72(%rbx), %rdi
	callq	core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
	addq	$24, %rbx
.Ltmp6560:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp6561:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB898_10:
.Ltmp6562:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end898:
