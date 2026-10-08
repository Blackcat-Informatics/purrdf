<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::FromIterator<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::from_iter::<core::iter::adapters::map::Map<core::slice::iter::Iter<core::option::Option<usize>>, purrdf_sparql_eval::modifier::eval_project_sequence<purrdf_core::ir::term::TermId>::{closure#1}>>:
.Lfunc_begin1246:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception829
	pushq	%rbp
	.cfi_def_cfa_offset 16
	pushq	%r15
	.cfi_def_cfa_offset 24
	pushq	%r14
	.cfi_def_cfa_offset 32
	pushq	%r13
	.cfi_def_cfa_offset 40
	pushq	%r12
	.cfi_def_cfa_offset 48
	pushq	%rbx
	.cfi_def_cfa_offset 56
	subq	$88, %rsp
	.cfi_def_cfa_offset 144
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	8(%rsi), %r15
	movq	(%rsi), %rbx
	movq	16(%rsi), %r12
	movq	$1, (%rsp)
	movq	%rdi, 40(%rsp)
	movq	%r15, %rdx
	subq	%rbx, %rdx
	cmpq	$64, %rdx
	ja	.LBB1246_2
	leaq	8(%rsp), %rcx
	movq	%rsp, %r14
	movl	$4, %eax
	movl	$1, %r13d
	movq	%rcx, %rbp
	leaq	-1(%r13), %rdx
	cmpq	%rax, %rdx
	jae	.LBB1246_5
.LBB1246_14:
	movl	$2, %esi
	leaq	8(%r12), %rdx
	incq	%rax
	vmovd	%esi, %xmm0
	jmp	.LBB1246_15
	.p2align	4
.LBB1246_19:
	movq	8(%r8), %rdi
	cmpq	%rsi, %rdi
	jae	.LBB1246_20
	vmovq	(%r9,%rdi,8), %xmm1
.LBB1246_28:
	vmovq	%xmm1, -8(%rcx,%r13,8)
	incq	%r13
	leaq	16(%r8), %rbx
	cmpq	%r13, %rax
	je	.LBB1246_6
.LBB1246_15:
	cmpq	%r15, %rbx
	je	.LBB1246_36
	cmpl	$1, (%rbx)
	movq	%rbx, %r8
	vmovdqa	%xmm0, %xmm1
	jne	.LBB1246_28
	movq	(%r12), %rsi
	movq	%rdx, %r9
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB1246_19
	movq	16(%r12), %rsi
	movq	8(%r12), %r9
	decq	%rsi
	jmp	.LBB1246_19
.LBB1246_36:
	movq	%r13, (%r14)
	jmp	.LBB1246_37
.LBB1246_6:
	addq	$16, %r8
	movq	%r8, %rbx
	movq	%rax, (%r14)
	cmpq	%r15, %rbx
	jne	.LBB1246_8
	jmp	.LBB1246_37
.LBB1246_2:
	shrq	$4, %rdx
.Ltmp23310:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	movq	%rsp, %r14
	movq	%r14, %rdi
	xorl	%esi, %esi
	callq	*%rax
.Ltmp23311:
	movq	(%rsp), %rcx
	xorl	%edx, %edx
	movl	$4, %eax
	leaq	8(%rsp), %rbp
	leaq	16(%rsp), %rsi
	decq	%rcx
	cmpq	$5, %rcx
	cmovaeq	%rcx, %rax
	movq	8(%rsp), %rcx
	setae	%dl
	cmovaeq	%rsi, %r14
	cmovbq	%rbp, %rcx
	shll	$4, %edx
	movq	(%rsp,%rdx), %r13
	leaq	-1(%r13), %rdx
	cmpq	%rax, %rdx
	jb	.LBB1246_14
.LBB1246_5:
	movq	%r13, %rax
	movq	%rax, (%r14)
	cmpq	%r15, %rbx
	je	.LBB1246_37
.LBB1246_8:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %r8
	movl	$2, %eax
	movq	%rsp, %r14
	vmovd	%eax, %xmm0
	vmovdqa	%xmm0, 64(%rsp)
	.p2align	4
.LBB1246_9:
	vmovdqa	64(%rsp), %xmm0
	cmpl	$1, (%rbx)
	vmovdqa	%xmm0, 48(%rsp)
	jne	.LBB1246_23
	movq	(%r12), %rsi
	leaq	8(%r12), %rax
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB1246_12
	movq	16(%r12), %rsi
	movq	8(%r12), %rax
	decq	%rsi
.LBB1246_12:
	movq	8(%rbx), %rdi
	cmpq	%rsi, %rdi
	jae	.LBB1246_13
	vmovq	(%rax,%rdi,8), %xmm0
	vmovdqa	%xmm0, 48(%rsp)
.LBB1246_23:
	movq	(%rsp), %rsi
	movq	8(%rsp), %rax
	xorl	%edx, %edx
	leaq	16(%rsp), %rdi
	movq	%r14, %rcx
	decq	%rsi
	cmpq	$5, %rsi
	cmovaeq	%rdi, %rcx
	movl	$4, %edi
	setae	%dl
	cmovbq	%rbp, %rax
	cmovbq	%rdi, %rsi
	shll	$4, %edx
	movq	(%rsp,%rdx), %r13
	leaq	-1(%r13), %rdx
	cmpq	%rsi, %rdx
	je	.LBB1246_24
.LBB1246_26:
	vmovdqa	48(%rsp), %xmm0
	addq	$16, %rbx
	vmovq	%xmm0, -8(%rax,%r13,8)
	incq	%r13
	movq	%r13, (%rcx)
	cmpq	%r15, %rbx
	jne	.LBB1246_9
	jmp	.LBB1246_37
.LBB1246_24:
.Ltmp23318:
	movl	$1, %edx
	movl	$1, %ecx
	movq	%r14, %rdi
	callq	*%r8
.Ltmp23319:
	cmpq	$6, (%rsp)
	movq	8(%rsp), %rax
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %r8
	leaq	16(%rsp), %rdx
	movq	%r14, %rcx
	cmovbq	%rbp, %rax
	cmovaeq	%rdx, %rcx
	jmp	.LBB1246_26
.LBB1246_37:
	vmovups	(%rsp), %ymm0
	movq	40(%rsp), %rcx
	movq	32(%rsp), %rax
	movq	%rax, 32(%rcx)
	vmovups	%ymm0, (%rcx)
	addq	$88, %rsp
	.cfi_def_cfa_offset 56
	popq	%rbx
	.cfi_def_cfa_offset 48
	popq	%r12
	.cfi_def_cfa_offset 40
	popq	%r13
	.cfi_def_cfa_offset 32
	popq	%r14
	.cfi_def_cfa_offset 24
	popq	%r15
	.cfi_def_cfa_offset 16
	popq	%rbp
	.cfi_def_cfa_offset 8
	vzeroupper
	retq
.LBB1246_13:
	.cfi_def_cfa_offset 144
.Ltmp23315:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.791(%rip), %rdx
	callq	*%rax
.Ltmp23316:
	jmp	.LBB1246_21
.LBB1246_20:
.Ltmp23312:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.791(%rip), %rdx
	callq	*%rax
.Ltmp23313:
.LBB1246_21:
	ud2
.LBB1246_30:
.Ltmp23320:
	jmp	.LBB1246_32
.LBB1246_29:
.Ltmp23314:
	movq	%rax, %rbx
	movq	%r13, (%r14)
	jmp	.LBB1246_33
.LBB1246_31:
.Ltmp23317:
.LBB1246_32:
	movq	%rax, %rbx
.LBB1246_33:
	movq	(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB1246_35
	movq	8(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB1246_35:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end1246:
