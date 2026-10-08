<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::adapters::map::Map<core::slice::iter::Iter<core::option::Option<usize>>, purrdf_sparql_eval::modifier::eval_project_sequence<purrdf_core::ir::term::TermId>::{closure#1}>>:
.Lfunc_begin1241:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception824
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
	subq	$56, %rsp
	.cfi_def_cfa_offset 112
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	(%rdi), %rax
	movq	8(%rsi), %r14
	movq	(%rsi), %r13
	movq	16(%rsi), %r15
	movq	16(%rdi), %rsi
	movl	$4, %ecx
	leaq	16(%rdi), %r11
	movl	$4, %r12d
	movq	%r14, %rdx
	subq	%r13, %rdx
	decq	%rax
	decq	%rsi
	shrq	$4, %rdx
	cmpq	$5, %rax
	cmovaeq	%rax, %rcx
	cmovbq	%rax, %rsi
	movq	%rcx, %r8
	subq	%rsi, %r8
	cmpq	%rdx, %r8
	jb	.LBB1241_1
.LBB1241_2:
	movq	8(%rdi), %rdx
	xorl	%esi, %esi
	cmpq	$5, %rax
	leaq	8(%rdi), %rbp
	movq	%rdi, %rbx
	setae	%sil
	cmovaeq	%r11, %rbx
	cmovbq	%rbp, %rdx
	shll	$4, %esi
	movq	(%rdi,%rsi), %r12
	leaq	-1(%r12), %rax
	cmpq	%rcx, %rax
	jae	.LBB1241_3
	movl	$2, %eax
	leaq	8(%r15), %r8
	incq	%rcx
	vmovd	%eax, %xmm0
	jmp	.LBB1241_13
	.p2align	4
.LBB1241_28:
	vmovq	%xmm1, -8(%rdx,%r12,8)
	incq	%r12
	leaq	16(%r9), %r13
	cmpq	%r12, %rcx
	je	.LBB1241_4
.LBB1241_13:
	cmpq	%r14, %r13
	je	.LBB1241_29
	cmpl	$1, (%r13)
	movq	%r13, %r9
	vmovdqa	%xmm0, %xmm1
	jne	.LBB1241_28
	movq	(%r15), %rsi
	movq	%r8, %r10
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB1241_17
	movq	16(%r15), %rsi
	movq	8(%r15), %r10
	decq	%rsi
.LBB1241_17:
	movq	8(%r9), %rax
	cmpq	%rsi, %rax
	jae	.LBB1241_18
	vmovq	(%r10,%rax,8), %xmm1
	jmp	.LBB1241_28
.LBB1241_3:
	movq	%r12, %rcx
	movq	%rcx, (%rbx)
	cmpq	%r14, %r13
	jne	.LBB1241_6
	jmp	.LBB1241_26
.LBB1241_29:
	movq	%r12, (%rbx)
	jmp	.LBB1241_26
.LBB1241_4:
	addq	$16, %r9
	movq	%r9, %r13
	movq	%rcx, (%rbx)
	cmpq	%r14, %r13
	je	.LBB1241_26
.LBB1241_6:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %r9
	movl	$2, %eax
	leaq	8(%r15), %r8
	movl	$4, %ebx
	vmovd	%eax, %xmm0
	.p2align	4
.LBB1241_7:
	cmpl	$1, (%r13)
	vmovdqa	%xmm0, %xmm1
	jne	.LBB1241_21
	movq	(%r15), %rsi
	movq	%r8, %rcx
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB1241_10
	movq	16(%r15), %rsi
	movq	8(%r15), %rcx
	decq	%rsi
.LBB1241_10:
	movq	8(%r13), %rax
	cmpq	%rsi, %rax
	jae	.LBB1241_11
	vmovq	(%rcx,%rax,8), %xmm1
.LBB1241_21:
	movq	(%rdi), %rsi
	movq	8(%rdi), %rax
	xorl	%edx, %edx
	movq	%rdi, %rcx
	decq	%rsi
	cmpq	$5, %rsi
	setae	%dl
	cmovbq	%rbp, %rax
	cmovaeq	%r11, %rcx
	cmovbq	%rbx, %rsi
	shll	$4, %edx
	movq	(%rdi,%rdx), %r12
	leaq	-1(%r12), %rdx
	cmpq	%rsi, %rdx
	je	.LBB1241_22
.LBB1241_25:
	vmovq	%xmm1, -8(%rax,%r12,8)
	incq	%r12
	addq	$16, %r13
	movq	%r12, (%rcx)
	cmpq	%r14, %r13
	jne	.LBB1241_7
	jmp	.LBB1241_26
.LBB1241_22:
	movq	%rdi, (%rsp)
	movl	$1, %edx
	movl	$1, %ecx
	vmovdqa	%xmm1, 16(%rsp)
	vmovdqa	%xmm0, 32(%rsp)
	movq	%r11, 8(%rsp)
	movq	(%rsp), %rdi
	callq	*%r9
	movq	(%rsp), %rdi
	movq	8(%rsp), %r11
	movq	%rbp, %rax
	cmpq	$6, (%rdi)
	movq	%rdi, %rcx
	jb	.LBB1241_24
	movq	8(%rdi), %rax
	movq	%r11, %rcx
.LBB1241_24:
	vmovdqa	32(%rsp), %xmm0
	vmovdqa	16(%rsp), %xmm1
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %r9
	leaq	8(%r15), %r8
	jmp	.LBB1241_25
.LBB1241_26:
	addq	$56, %rsp
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
	retq
.LBB1241_1:
	.cfi_def_cfa_offset 112
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	movq	%rdi, %rbx
	movq	%r11, %rbp
	callq	*%rax
	movq	(%rbx), %rax
	movq	%rbp, %r11
	movq	%rbx, %rdi
	decq	%rax
	cmpq	$5, %rax
	cmovaeq	%rax, %r12
	movq	%r12, %rcx
	jmp	.LBB1241_2
.LBB1241_11:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rcx
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.791(%rip), %rdx
	movq	%rax, %rdi
	callq	*%rcx
.LBB1241_18:
.Ltmp23313:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rcx
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.791(%rip), %rdx
	movq	%rax, %rdi
	callq	*%rcx
.Ltmp23314:
	ud2
.LBB1241_30:
.Ltmp23315:
	movq	%rax, %rdi
	movq	%r12, (%rbx)
	callq	_Unwind_Resume@PLT
.Lfunc_end1241:
