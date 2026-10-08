<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>> as alloc::vec::spec_extend::SpecExtend<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, core::iter::adapters::map::Map<core::slice::iter::Iter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, purrdf_sparql_eval::modifier::eval_project_sequence<purrdf_core::ir::term::TermId>::{closure#1}>>>::spec_extend:
.Lfunc_begin2210:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception1432
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
	subq	$168, %rsp
	.cfi_def_cfa_offset 224
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	8(%rsi), %rbx
	movq	(%rsi), %r12
	movq	(%rdi), %r8
	movq	16(%rdi), %rdx
	movabsq	$-3689348814741910323, %rcx
	movq	%rbx, %rax
	subq	%r12, %rax
	subq	%rdx, %r8
	shrq	$3, %rax
	imulq	%rax, %rcx
	movq	%rcx, 56(%rsp)
	cmpq	%r8, %rcx
	ja	.LBB2210_1
	movq	%rdi, (%rsp)
	cmpq	%rbx, %r12
	je	.LBB2210_41
.LBB2210_3:
	movq	8(%rdi), %rax
	movq	16(%rsi), %rsi
	movl	$2, %ecx
	leaq	32(%rsp), %rbp
	leaq	16(%rsp), %r11
	xorl	%ebx, %ebx
	movq	%r12, 72(%rsp)
	vmovd	%ecx, %xmm0
	vmovdqa	%xmm0, 96(%rsp)
	movq	%rax, 64(%rsp)
	movq	%rsi, 80(%rsp)
	jmp	.LBB2210_4
	.p2align	4
.LBB2210_39:
	movq	%rbp, 16(%rsp,%r14)
	movq	%r9, %rbp
.LBB2210_40:
	vmovdqu	16(%rsp), %ymm0
	movq	8(%rsp), %rdx
	movq	88(%rsp), %rbx
	movq	48(%rsp), %rax
	movq	64(%rsp), %rsi
	movq	72(%rsp), %r12
	leaq	(%rdx,%rdx,4), %rcx
	incq	%rdx
	incq	%rbx
	movq	%rax, 160(%rsp)
	vmovdqu	%ymm0, 128(%rsp)
	movq	%rax, 32(%rsi,%rcx,8)
	vmovdqu	%ymm0, (%rsi,%rcx,8)
	cmpq	56(%rsp), %rbx
	je	.LBB2210_41
.LBB2210_4:
	leaq	(%rbx,%rbx,4), %rax
	movq	%rdx, 8(%rsp)
	movq	(%r12,%rax,8), %r13
	decq	%r13
	cmpq	$5, %r13
	jb	.LBB2210_5
	movq	16(%r12,%rax,8), %r13
	movq	8(%r12,%rax,8), %r12
	decq	%r13
	jmp	.LBB2210_7
	.p2align	4
.LBB2210_5:
	leaq	8(%r12,%rax,8), %r12
.LBB2210_7:
	movq	80(%rsp), %rax
	movq	$1, 16(%rsp)
	movq	8(%rax), %r15
	movq	16(%rax), %r8
	cmpq	$5, %r8
	jae	.LBB2210_9
	movq	%rbp, %r9
	xorl	%eax, %eax
.LBB2210_11:
	shlq	$4, %r8
	xorl	%r14d, %r14d
	cmpq	$5, %rax
	movl	$4, %edx
	leaq	24(%rsp), %rsi
	movq	%rbx, 88(%rsp)
	setae	%r14b
	cmovbq	%rdx, %rax
	cmovbq	%rsi, %rcx
	shll	$4, %r14d
	movq	16(%rsp,%r14), %rbp
	leaq	-1(%rbp), %rdx
	cmpq	%rax, %rdx
	jae	.LBB2210_12
	movq	%r8, %rdx
	incq	%rax
	negq	%rdx
	movq	%r8, %r10
	xorl	%esi, %esi
	movq	%r15, %r8
	jmp	.LBB2210_23
	.p2align	4
.LBB2210_26:
	vmovq	(%r12,%rdi,8), %xmm0
.LBB2210_27:
	vmovq	%xmm0, -8(%rcx,%rbp,8)
	addq	$16, %r8
	incq	%rbp
	addq	$-16, %rsi
	cmpq	%rbp, %rax
	je	.LBB2210_13
.LBB2210_23:
	cmpq	%rsi, %rdx
	je	.LBB2210_39
	vmovdqa	96(%rsp), %xmm0
	cmpl	$1, (%r8)
	jne	.LBB2210_27
	movq	8(%r8), %rdi
	cmpq	%r13, %rdi
	jb	.LBB2210_26
	jmp	.LBB2210_28
	.p2align	4
.LBB2210_12:
	movq	%r15, %rbx
	movq	%rbp, %rax
	jmp	.LBB2210_14
	.p2align	4
.LBB2210_13:
	movq	%r15, %rbx
	subq	%rsi, %rbx
	movq	%r10, %r8
.LBB2210_14:
	addq	%r8, %r15
	movq	%rax, 16(%rsp,%r14)
	movq	%r9, %rbp
	cmpq	%r15, %rbx
	je	.LBB2210_40
	.p2align	4
.LBB2210_15:
	vmovdqa	96(%rsp), %xmm0
	cmpl	$1, (%rbx)
	vmovdqa	%xmm0, 112(%rsp)
	jne	.LBB2210_18
	movq	8(%rbx), %rdi
	cmpq	%r13, %rdi
	jae	.LBB2210_30
	vmovq	(%r12,%rdi,8), %xmm0
	vmovdqa	%xmm0, 112(%rsp)
.LBB2210_18:
	movq	16(%rsp), %rsi
	movq	24(%rsp), %rax
	xorl	%edx, %edx
	leaq	24(%rsp), %rcx
	movl	$4, %edi
	decq	%rsi
	cmpq	$5, %rsi
	setae	%dl
	cmovbq	%rcx, %rax
	movq	%r11, %rcx
	cmovaeq	%rbp, %rcx
	cmovbq	%rdi, %rsi
	shll	$4, %edx
	movq	16(%rsp,%rdx), %r14
	leaq	-1(%r14), %rdx
	cmpq	%rsi, %rdx
	je	.LBB2210_19
.LBB2210_21:
	vmovdqa	112(%rsp), %xmm0
	addq	$16, %rbx
	vmovq	%xmm0, -8(%rax,%r14,8)
	incq	%r14
	movq	%r14, (%rcx)
	cmpq	%r15, %rbx
	jne	.LBB2210_15
	jmp	.LBB2210_40
.LBB2210_19:
.Ltmp43562:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	movl	$1, %edx
	movl	$1, %ecx
	movq	%r11, %rdi
	vzeroupper
	callq	*%rax
.Ltmp43563:
	cmpq	$6, 16(%rsp)
	movq	24(%rsp), %rax
	leaq	24(%rsp), %rcx
	leaq	16(%rsp), %r11
	cmovbq	%rcx, %rax
	movq	%r11, %rcx
	cmovaeq	%rbp, %rcx
	jmp	.LBB2210_21
.LBB2210_9:
.Ltmp43553:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %r10
	movl	$1, %ecx
	movq	%r11, %rdi
	xorl	%esi, %esi
	movq	%r8, %rdx
	movq	%r8, %r14
	vzeroupper
	callq	*%r10
.Ltmp43554:
	movq	16(%rsp), %rax
	movq	24(%rsp), %rcx
	leaq	16(%rsp), %r11
	movq	%rbp, %r9
	movq	%r14, %r8
	decq	%rax
	jmp	.LBB2210_11
.LBB2210_1:
	movq	%rsi, %r14
	movq	%rdx, %rsi
	movq	56(%rsp), %rdx
	movl	$8, %ecx
	movl	$40, %r8d
	movq	%rdi, %r15
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.8174518965507137190)
	movq	16(%r15), %rdx
	movq	%r15, %rdi
	movq	%r14, %rsi
	movq	%rdi, (%rsp)
	cmpq	%rbx, %r12
	jne	.LBB2210_3
.LBB2210_41:
	movq	(%rsp), %rax
	movq	%rdx, 16(%rax)
	addq	$168, %rsp
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
.LBB2210_30:
	.cfi_def_cfa_offset 224
.Ltmp43559:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.791(%rip), %rdx
	movq	%r13, %rsi
	vzeroupper
	callq	*%rax
.Ltmp43560:
	jmp	.LBB2210_29
.LBB2210_28:
.Ltmp43556:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.791(%rip), %rdx
	movq	%r13, %rsi
	vzeroupper
	callq	*%rax
.Ltmp43557:
.LBB2210_29:
	ud2
.LBB2210_33:
.Ltmp43555:
	jmp	.LBB2210_35
.LBB2210_32:
.Ltmp43564:
	jmp	.LBB2210_35
.LBB2210_31:
.Ltmp43558:
	movq	%rax, %r15
	movq	%rbp, 16(%rsp,%r14)
	jmp	.LBB2210_36
.LBB2210_34:
.Ltmp43561:
.LBB2210_35:
	movq	%rax, %r15
.LBB2210_36:
	movq	(%rsp), %rbx
	movq	8(%rsp), %r14
	movq	16(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB2210_38
	movq	24(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB2210_38:
	movq	%r15, %rdi
	movq	%r14, 16(%rbx)
	callq	_Unwind_Resume@PLT
.Lfunc_end2210:
