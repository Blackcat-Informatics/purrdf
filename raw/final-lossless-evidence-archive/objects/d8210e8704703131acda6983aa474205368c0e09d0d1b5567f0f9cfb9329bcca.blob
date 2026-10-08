purrdf_sparql_eval::modifier::eval_project::<purrdf_core::ir::dataset::RdfDataset>:
.Lfunc_begin1003:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception663
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
	subq	$680, %rsp
	.cfi_def_cfa_offset 736
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	%rdi, %rbx
	leaq	456(%rsp), %rdi
	movq	%r9, %r14
	movq	%r8, %r15
	movq	%rcx, %r13
	movq	%rdx, %rbp
	callq	*%rax
	movq	1136(%r14), %rax
	movq	%rbx, 232(%rsp)
	testq	%rax, %rax
	je	.LBB1003_1
	movq	1128(%r14), %rcx
	movq	%rax, 72(%rsp)
	leaq	(%rax,%rax,2), %rax
	movq	%r15, %r12
	shlq	$4, %r12
	movq	%r14, 8(%rsp)
	movq	%r15, 48(%rsp)
	movq	%r13, 16(%rsp)
	movq	%rbp, 64(%rsp)
	addq	%r13, %r12
	shlq	$4, %rax
	movq	%rax, 240(%rsp)
	addq	%rcx, %rax
	movq	%rax, 24(%rsp)
	testq	%r15, %r15
	je	.LBB1003_41
	movq	bcmp@GOTPCREL(%rip), %rbx
	movq	%rcx, %rax
.LBB1003_4:
	movq	24(%rax), %rbp
	movq	32(%rax), %r14
	addq	$48, %rax
	movq	%rax, 32(%rsp)
	leaq	16(%rbp), %r15
	jmp	.LBB1003_5
	.p2align	4
.LBB1003_38:
	xorq	%r14, %rax
	xorq	%rbp, %rdi
	orq	%rax, %rdi
	je	.LBB1003_7
.LBB1003_39:
	addq	$16, %r13
	cmpq	%r12, %r13
	je	.LBB1003_40
.LBB1003_5:
	movq	(%r13), %rdi
	movq	8(%r13), %rax
	cmpq	%rbp, %rdi
	sete	%cl
	cmpq	%r14, %rax
	setne	%dl
	orb	%cl, %dl
	jne	.LBB1003_38
	addq	$16, %rdi
	movq	%r15, %rsi
	movq	%r14, %rdx
	callq	*%rbx
	testl	%eax, %eax
	jne	.LBB1003_39
.LBB1003_7:
	movq	32(%rsp), %rax
	movq	16(%rsp), %r13
	cmpq	24(%rsp), %rax
	jne	.LBB1003_4
	movq	8(%rsp), %rbp
	movl	$8, %r14d
	movq	$8, 272(%rsp)
	movq	$0, 288(%rsp)
	movq	$8, 296(%rsp)
	jmp	.LBB1003_9
.LBB1003_40:
	movq	48(%rsp), %r15
	lock		incq	(%rbp)
	jg	.LBB1003_46
	jmp	.LBB1003_43
.LBB1003_1:
.Ltmp16605:
	leaq	560(%rsp), %rdi
	movq	%rbp, %rsi
	movq	%r14, %rdx
	movq	%rbp, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp16606:
	cmpl	$1, 560(%rsp)
	jne	.LBB1003_96
.LBB1003_15:
	vmovups	608(%rsp), %zmm1
	vmovups	576(%rsp), %zmm0
	movq	528(%rsp), %rax
	vmovups	%zmm1, 48(%rbx)
	vmovups	%zmm0, 16(%rbx)
	movq	$1, (%rbx)
	cmpq	$6, %rax
	jb	.LBB1003_25
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	536(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1003_18
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1003_18:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1003_24
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1003_18
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB1003_21:
	cmpq	%rax, %rsi
	jge	.LBB1003_23
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB1003_21
.LBB1003_23:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1003_24:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB1003_25:
	movq	456(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB1003_35
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	464(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1003_28
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1003_28:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1003_34
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1003_28
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB1003_31:
	cmpq	%rax, %rsi
	jge	.LBB1003_33
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB1003_31
.LBB1003_33:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1003_34:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB1003_35:
	movq	552(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1003_202
	lock		decq	(%rax)
	jne	.LBB1003_202
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	552(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
	jmp	.LBB1003_202
.LBB1003_41:
	movq	24(%rcx), %rbp
	movq	32(%rcx), %r14
	leaq	48(%rcx), %rax
	movq	%rax, 32(%rsp)
	lock		incq	(%rbp)
	jle	.LBB1003_43
.LBB1003_46:
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$64, %edi
	movq	%rbp, 272(%rsp)
	movq	%r14, 280(%rsp)
	callq	*%rax
	testq	%rax, %rax
	je	.LBB1003_58
	movq	%rax, %rsi
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$64, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$64, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB1003_49
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
.LBB1003_49:
	movq	8(%rsp), %rax
	addq	$1120, %rax
	movq	%rax, 256(%rsp)
	.p2align	4
.LBB1003_50:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1003_56
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1003_50
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdi
	lock		incq	(%rax)
	lock		addq	$64, (%rdx)
	movl	$64, %edx
	lock		xaddq	%rdx, (%rdi)
	addq	$64, %rdx
	cmovoq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
	.p2align	4
.LBB1003_53:
	cmpq	%rax, %rdx
	jle	.LBB1003_55
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1003_53
.LBB1003_55:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1003_56:
	movq	32(%rsp), %rax
	movq	%rbp, (%rsi)
	movq	$4, 112(%rsp)
	movq	%rsi, 120(%rsp)
	movq	%r14, 8(%rsi)
	movq	$1, 128(%rsp)
	cmpq	24(%rsp), %rax
	je	.LBB1003_57
	movl	$1, %r14d
	testq	%r15, %r15
	je	.LBB1003_66
	movq	32(%rsp), %rbx
	jmp	.LBB1003_61
	.p2align	4
.LBB1003_88:
	movq	120(%rsp), %rsi
.LBB1003_77:
	movq	%r14, %rax
	shlq	$4, %rax
	incq	%r14
	movq	%r13, (%rsi,%rax)
	movq	%rbp, 8(%rsi,%rax)
	movq	%r14, 128(%rsp)
	cmpq	24(%rsp), %rbx
	je	.LBB1003_78
.LBB1003_61:
	movq	%rsi, 248(%rsp)
.LBB1003_62:
	movq	24(%rbx), %r13
	leaq	48(%rbx), %rax
	movq	32(%rbx), %rbp
	movq	16(%rsp), %rbx
	movq	%rax, 32(%rsp)
	leaq	16(%r13), %r15
	jmp	.LBB1003_63
	.p2align	4
.LBB1003_73:
	xorq	%rbp, %rax
	xorq	%r13, %rdi
	orq	%rax, %rdi
	je	.LBB1003_65
.LBB1003_74:
	addq	$16, %rbx
	cmpq	%r12, %rbx
	je	.LBB1003_75
.LBB1003_63:
	movq	(%rbx), %rdi
	movq	8(%rbx), %rax
	cmpq	%r13, %rdi
	sete	%cl
	cmpq	%rbp, %rax
	setne	%dl
	orb	%cl, %dl
	jne	.LBB1003_73
	movq	bcmp@GOTPCREL(%rip), %rax
	addq	$16, %rdi
	movq	%r15, %rsi
	movq	%rbp, %rdx
	callq	*%rax
	testl	%eax, %eax
	jne	.LBB1003_74
.LBB1003_65:
	movq	32(%rsp), %rbx
	cmpq	24(%rsp), %rbx
	jne	.LBB1003_62
	jmp	.LBB1003_78
	.p2align	4
.LBB1003_75:
	lock		incq	(%r13)
	movq	248(%rsp), %rsi
	jle	.LBB1003_43
	movq	32(%rsp), %rbx
	movq	%r13, 272(%rsp)
	movq	%rbp, 280(%rsp)
	cmpq	112(%rsp), %r14
	jne	.LBB1003_77
.Ltmp16570:
	movl	$1, %edx
	movl	$8, %ecx
	movl	$16, %r8d
	leaq	112(%rsp), %rdi
	movq	%r14, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.13412714042204560522)
.Ltmp16571:
	jmp	.LBB1003_88
.LBB1003_57:
	movl	$1, %r14d
.LBB1003_78:
	movq	8(%rsp), %rbp
	movq	120(%rsp), %rbx
	movq	112(%rsp), %rax
	movq	<alloc::raw_vec::RawVec<core::option::Option<(alloc::string::String, alloc::string::String)>>>::grow_one@GOTPCREL(%rip), %r13
	shlq	$4, %r14
	movq	1136(%rbp), %r15
	movq	%rbx, 272(%rsp)
	movq	%rax, 288(%rsp)
	addq	%rbx, %r14
	addq	$16, %rbx
	movq	%r14, 296(%rsp)
	movq	%r15, %rax
	shlq	$4, %rax
	leaq	(%rax,%rax,2), %r12
	jmp	.LBB1003_79
	.p2align	4
.LBB1003_81:
	vmovdqu	112(%rsp), %ymm0
	vmovups	128(%rsp), %ymm1
	movq	1128(%rbp), %rcx
	leaq	-16(%rbx), %rax
	incq	%r15
	addq	$16, %rbx
	addq	$16, %rax
	vmovups	%ymm1, 16(%rcx,%r12)
	vmovdqu	%ymm0, (%rcx,%r12)
	addq	$48, %r12
	movq	%r15, 1136(%rbp)
	cmpq	%r14, %rax
	je	.LBB1003_82
.LBB1003_79:
	vmovups	-16(%rbx), %xmm0
	movq	256(%rsp), %rax
	vmovups	%xmm0, 136(%rsp)
	movq	$2, 112(%rsp)
	movb	$0, 152(%rsp)
	cmpq	(%rax), %r15
	jne	.LBB1003_81
.Ltmp16581:
	movq	256(%rsp), %rdi
	vzeroupper
	callq	*%r13
.Ltmp16582:
	jmp	.LBB1003_81
.LBB1003_82:
	movq	16(%rsp), %r13
.LBB1003_9:
	movq	%r14, 280(%rsp)
.Ltmp16589:
	leaq	272(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
.Ltmp16590:
	movq	232(%rsp), %rbx
	movq	48(%rsp), %r15
	movq	72(%rsp), %r14
.Ltmp16591:
	movq	64(%rsp), %rcx
	leaq	112(%rsp), %rdi
	movq	%rbp, %rdx
	movq	%rcx, %rsi
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp16592:
	movq	1136(%rbp), %rsi
	subq	%r14, %rsi
	jb	.LBB1003_13
	movq	240(%rsp), %rdi
	addq	1128(%rbp), %rdi
	movq	%r14, 1136(%rbp)
.Ltmp16593:
	callq	core::ptr::drop_glue::<[purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>]>
.Ltmp16594:
.LBB1003_13:
	vmovups	160(%rsp), %zmm1
	vmovdqu64	112(%rsp), %zmm0
	vmovups	%zmm1, 608(%rsp)
	vmovdqu64	%zmm0, 560(%rsp)
	cmpl	$1, 560(%rsp)
	je	.LBB1003_15
.LBB1003_96:
	leaq	568(%rsp), %rcx
.Ltmp16607:
	leaq	112(%rsp), %rdi
	leaq	456(%rsp), %rsi
	xorl	%edx, %edx
	movq	%rdi, %rbp
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp16608:
	cmpq	$-1, 112(%rsp)
	je	.LBB1003_207
	vmovdqu	112(%rsp), %ymm0
	vmovdqu	%ymm0, 416(%rsp)
.Ltmp16609:
	movq	<purrdf_sparql_eval::solution::VarSchema>::interned@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r15, %rsi
	vzeroupper
	callq	*%rax
.Ltmp16610:
	movq	%rax, 408(%rsp)
	movq	%rax, 376(%rsp)
	movq	32(%rax), %rcx
	movq	%rcx, %rbx
	shlq	$4, %rbx
	movq	%rcx, 8(%rsp)
	movq	%rbx, 56(%rsp)
	testq	%rcx, %rcx
	je	.LBB1003_100
	movq	24(%rax), %r14
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB1003_121
	movq	%rax, %r13
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	$-1, %rcx
	movabsq	$9223372036854775807, %rdx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	%rbx, %rax
	cmovbq	%rcx, %rax
	cmpq	%rdx, %rbx
	movq	%rdx, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmovbq	%rbx, %rcx
	addq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB1003_112
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1003_112:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1003_118
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1003_112
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	%rbx, (%rdx)
	movq	%rcx, %rdx
	lock		xaddq	%rdx, (%rsi)
	movabsq	$-9223372036854775808, %rsi
	leaq	(%rdx,%rcx), %rax
	sarq	$63, %rax
	xorq	%rax, %rsi
	addq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rsi, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB1003_115:
	cmpq	%rax, %rdx
	jle	.LBB1003_117
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1003_115
.LBB1003_117:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1003_118:
	movq	440(%rsp), %rbx
	movq	8(%rsp), %r12
	xorl	%r15d, %r15d
	addq	$16, %rbx
	.p2align	4
.LBB1003_119:
	leaq	(%r14,%r15), %rsi
.Ltmp16612:
	movq	%rbx, %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.13412714042204560522)
.Ltmp16613:
	movq	%rax, (%r13,%r15)
	movq	%rdx, 8(%r13,%r15)
	addq	$16, %r15
	decq	%r12
	jne	.LBB1003_119
	movq	432(%rsp), %rax
	movq	%r13, 64(%rsp)
	testq	%rax, %rax
	je	.LBB1003_102
.LBB1003_123:
	movq	424(%rsp), %rcx
	movq	malloc@GOTPCREL(%rip), %r14
	movq	%rax, 48(%rsp)
	leaq	(,%rax,8), %rax
	leaq	(%rax,%rax,4), %rbx
	movq	%rbx, %rdi
	movq	%rcx, 240(%rsp)
	callq	*%r14
	movq	%rax, 72(%rsp)
	testq	%rax, %rax
	je	.LBB1003_138
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rsi
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rdi
	movabsq	$-9223372036854775808, %rcx
	movq	$-1, %r8
	movq	%rbp, %r10
	leaq	(%rbx,%rax), %rdx
	sarq	$63, %rdx
	xorq	%rcx, %rdx
	addq	%rbx, %rax
	cmovoq	%rdx, %rax
	incq	%rsi
	cmoveq	%r8, %rsi
	addq	%rbx, %rdi
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmovbq	%r8, %rdi
	movq	%rsi, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%rdi, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB1003_126
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1003_126:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1003_132
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1003_126
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	%rbx, (%rdx)
	movq	%rbx, %rdx
	lock		xaddq	%rdx, (%rsi)
	leaq	(%rdx,%rbx), %rax
	sarq	$63, %rax
	xorq	%rcx, %rax
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	addq	%rbx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB1003_129:
	cmpq	%rax, %rdx
	jle	.LBB1003_131
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1003_129
.LBB1003_131:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1003_132:
	movq	56(%rsp), %rbp
	movq	48(%rsp), %rax
	movq	72(%rsp), %rsi
	movq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::grow@GOTPCREL(%rip), %r11
	movl	$2, %ecx
	leaq	120(%rsp), %r14
	movq	$0, 16(%rsp)
	vmovd	%ecx, %xmm0
	vmovdqa	%xmm0, 32(%rsp)
	leaq	(%r13,%rbp), %rdx
	negq	%rbp
	movq	%rax, 384(%rsp)
	movq	%rsi, 392(%rsp)
	movq	%rdx, 24(%rsp)
	jmp	.LBB1003_133
	.p2align	4
.LBB1003_170:
	movq	%rbx, (%r12)
.LBB1003_171:
	vmovdqu	112(%rsp), %ymm0
	movq	144(%rsp), %rax
	movq	72(%rsp), %rcx
	movq	248(%rsp), %rdx
	movq	64(%rsp), %r13
	movq	%rax, 32(%rcx,%rdx,8)
	movq	%rax, 304(%rsp)
	movq	48(%rsp), %rax
	vmovdqu	%ymm0, (%rcx,%rdx,8)
	movq	16(%rsp), %rcx
	vmovdqu	%ymm0, 272(%rsp)
	incq	%rcx
	movq	%rcx, 16(%rsp)
	cmpq	%rax, %rcx
	je	.LBB1003_172
.LBB1003_133:
	cmpq	$5, 8(%rsp)
	movl	$1, %ebx
	movl	$4, %eax
	movq	$1, 112(%rsp)
	movq	%r14, %rcx
	movq	%r10, %r12
	jae	.LBB1003_134
.LBB1003_136:
	movq	16(%rsp), %rdx
	movq	240(%rsp), %rdi
	leaq	(%rdx,%rdx,4), %rsi
	leaq	-1(%rbx), %rdx
	leaq	(%rdi,%rsi,8), %r15
	movq	%rsi, 248(%rsp)
	cmpq	%rax, %rdx
	jae	.LBB1003_137
	movq	(%r15), %rsi
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB1003_140
	movq	16(%r15), %rsi
	movq	8(%r15), %rdx
	decq	%rsi
	jmp	.LBB1003_142
	.p2align	4
.LBB1003_137:
	movq	%rbx, %rax
	movq	%rax, (%r12)
	cmpq	24(%rsp), %r13
	jne	.LBB1003_149
	jmp	.LBB1003_171
	.p2align	4
.LBB1003_140:
	leaq	8(%r15), %rdx
.LBB1003_142:
	incq	%rax
	xorl	%r8d, %r8d
	movq	%r13, %r9
	jmp	.LBB1003_143
	.p2align	4
.LBB1003_160:
	vmovq	(%rdx,%rdi,8), %xmm0
.LBB1003_161:
	vmovq	%xmm0, -8(%rcx,%rbx,8)
	addq	$16, %r9
	incq	%rbx
	addq	$-16, %r8
	cmpq	%rbx, %rax
	je	.LBB1003_147
.LBB1003_143:
	cmpq	%r8, %rbp
	je	.LBB1003_170
	vmovdqa	32(%rsp), %xmm0
	cmpl	$1, (%r9)
	jne	.LBB1003_161
	movq	8(%r9), %rdi
	cmpq	%rsi, %rdi
	jb	.LBB1003_160
	jmp	.LBB1003_146
	.p2align	4
.LBB1003_147:
	subq	%r8, %r13
	movq	%rax, (%r12)
	cmpq	24(%rsp), %r13
	je	.LBB1003_171
.LBB1003_149:
	movq	(%r15), %rbx
	decq	%rbx
	cmpq	$5, %rbx
	jb	.LBB1003_150
	movq	16(%r15), %rbx
	movq	8(%r15), %r15
	decq	%rbx
	jmp	.LBB1003_152
	.p2align	4
.LBB1003_150:
	addq	$8, %r15
	jmp	.LBB1003_152
.LBB1003_157:
.Ltmp16627:
	movl	$1, %edx
	movl	$1, %ecx
	movq	%r10, %rdi
	vzeroupper
	callq	*%r11
.Ltmp16628:
	cmpq	$6, 112(%rsp)
	movq	120(%rsp), %rax
	movq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::grow@GOTPCREL(%rip), %r11
	leaq	112(%rsp), %r10
	leaq	128(%rsp), %rdx
	movq	%r10, %rcx
	cmovbq	%r14, %rax
	cmovaeq	%rdx, %rcx
	jmp	.LBB1003_159
	.p2align	4
.LBB1003_152:
	vmovdqa	32(%rsp), %xmm0
	cmpl	$1, (%r13)
	vmovdqa	%xmm0, 256(%rsp)
	jne	.LBB1003_156
	movq	8(%r13), %rdi
	cmpq	%rbx, %rdi
	jae	.LBB1003_154
	vmovq	(%r15,%rdi,8), %xmm0
	vmovdqa	%xmm0, 256(%rsp)
.LBB1003_156:
	movq	112(%rsp), %rsi
	movq	120(%rsp), %rax
	xorl	%edx, %edx
	leaq	128(%rsp), %rdi
	movq	%r10, %rcx
	decq	%rsi
	cmpq	$5, %rsi
	cmovaeq	%rdi, %rcx
	movl	$4, %edi
	setae	%dl
	cmovbq	%r14, %rax
	cmovbq	%rdi, %rsi
	shll	$4, %edx
	movq	112(%rsp,%rdx), %r12
	leaq	-1(%r12), %rdx
	cmpq	%rsi, %rdx
	je	.LBB1003_157
.LBB1003_159:
	vmovdqa	256(%rsp), %xmm0
	addq	$16, %r13
	vmovq	%xmm0, -8(%rax,%r12,8)
	incq	%r12
	movq	%r12, (%rcx)
	cmpq	24(%rsp), %r13
	jne	.LBB1003_152
	jmp	.LBB1003_171
.LBB1003_134:
.Ltmp16618:
	movq	8(%rsp), %rdx
	movl	$1, %ecx
	movq	%r10, %rdi
	xorl	%esi, %esi
	movq	%r10, %r15
	movq	%r11, %r12
	vzeroupper
	callq	*%r11
.Ltmp16619:
	movq	112(%rsp), %rax
	xorl	%edx, %edx
	movl	$4, %ecx
	leaq	128(%rsp), %rsi
	movq	%r15, %rdi
	movq	%r15, %r10
	movq	%r12, %r11
	decq	%rax
	cmpq	$5, %rax
	cmovbq	%rcx, %rax
	movq	120(%rsp), %rcx
	setae	%dl
	cmovaeq	%rsi, %rdi
	movq	%rdi, %r12
	cmovbq	%r14, %rcx
	shll	$4, %edx
	movq	112(%rsp,%rdx), %rbx
	jmp	.LBB1003_136
.LBB1003_207:
.Ltmp16639:
	movq	<purrdf_sparql_eval::solution::VarSchema>::interned@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r15, %rsi
	callq	*%rax
.Ltmp16640:
	vmovups	496(%rsp), %zmm1
	vmovups	456(%rsp), %zmm0
	movq	%rax, 104(%rsp)
	movq	$0, 80(%rsp)
	movq	$8, 88(%rsp)
	movq	$0, 96(%rsp)
	vmovups	%zmm1, 152(%rsp)
	vmovups	%zmm0, 112(%rsp)
	cmpq	$-1, 112(%rsp)
	je	.LBB1003_210
	leaq	272(%rsp), %rdi
	leaq	80(%rsp), %rsi
	leaq	456(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	184(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB1003_212
	jmp	.LBB1003_221
.LBB1003_66:
	movq	32(%rsp), %rax
	movl	$24, %ebx
	leaq	112(%rsp), %rbp
	jmp	.LBB1003_67
	.p2align	4
.LBB1003_71:
	addq	$48, %r12
	movq	%r13, -8(%rsi,%rbx)
	movq	%r15, (%rsi,%rbx)
	incq	%r14
	addq	$16, %rbx
	movq	%r12, %rax
	movq	%r14, 128(%rsp)
	cmpq	24(%rsp), %r12
	je	.LBB1003_78
.LBB1003_67:
	movq	24(%rax), %r13
	movq	32(%rax), %r15
	lock		incq	(%r13)
	jle	.LBB1003_43
	movq	%r13, 272(%rsp)
	movq	%rax, %r12
	movq	%r15, 280(%rsp)
	cmpq	112(%rsp), %r14
	jne	.LBB1003_71
.Ltmp16573:
	movl	$1, %edx
	movl	$8, %ecx
	movl	$16, %r8d
	movq	%rbp, %rdi
	movq	%r14, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.13412714042204560522)
.Ltmp16574:
	movq	120(%rsp), %rsi
	jmp	.LBB1003_71
.LBB1003_100:
	movl	$8, %r13d
	movq	432(%rsp), %rax
	movq	%r13, 64(%rsp)
	testq	%rax, %rax
	jne	.LBB1003_123
.LBB1003_102:
	movq	$0, 384(%rsp)
	movq	$8, 392(%rsp)
.LBB1003_172:
	vmovdqu64	456(%rsp), %zmm0
	vmovups	496(%rsp), %zmm1
	movq	%rax, 400(%rsp)
	movq	%rax, 96(%rsp)
	movq	376(%rsp), %rcx
	movq	384(%rsp), %rax
	movq	392(%rsp), %rdx
	movq	%rax, 80(%rsp)
	movq	%rdx, 88(%rsp)
	movq	%rcx, 104(%rsp)
	vmovdqu64	%zmm0, 112(%rsp)
	vmovups	%zmm1, 152(%rsp)
	cmpq	$-1, 112(%rsp)
	je	.LBB1003_177
	leaq	272(%rsp), %rdi
	leaq	80(%rsp), %rsi
	leaq	456(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	232(%rsp), %rbx
	movq	184(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB1003_179
.LBB1003_188:
	movq	208(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB1003_189
	jmp	.LBB1003_191
.LBB1003_177:
	movq	88(%rsp), %rcx
	movq	80(%rsp), %rax
	movq	96(%rsp), %rdx
	movq	%rcx, 288(%rsp)
	movq	104(%rsp), %rcx
	movq	%rax, 280(%rsp)
	movq	%rdx, 296(%rsp)
	movq	%rcx, 304(%rsp)
	movq	$-1, 272(%rsp)
	movq	232(%rsp), %rbx
	movq	184(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB1003_188
.LBB1003_179:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	192(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1003_181
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1003_181:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1003_187
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1003_181
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB1003_184:
	cmpq	%rax, %rsi
	jge	.LBB1003_186
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB1003_184
.LBB1003_186:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1003_187:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	208(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1003_191
.LBB1003_189:
	lock		decq	(%rax)
	jne	.LBB1003_191
	xorl	%ebp, %ebp
	leaq	208(%rsp), %rdi
	#MEMBARRIER
.Ltmp16632:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp16633:
.LBB1003_191:
	vmovups	304(%rsp), %zmm1
	vmovups	272(%rsp), %zmm0
	cmpq	$0, 8(%rsp)
	vmovups	%zmm1, 40(%rbx)
	vmovups	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
	je	.LBB1003_201
	movq	56(%rsp), %rdi
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rcx
	cmpq	%rcx, %rdi
	cmovaeq	%rcx, %rdi
	xorl	%edx, %edx
	cmpq	%rdi, %rax
	movq	%rdi, %rsi
	setns	%dl
	addq	%rcx, %rdx
	subq	%rdi, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1003_194
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1003_194:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1003_200
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1003_194
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rsi, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rsi, %rdx
	setns	%al
	addq	%rcx, %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	subq	%rsi, %rdx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB1003_197:
	cmpq	%rax, %rdx
	jge	.LBB1003_199
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1003_197
.LBB1003_199:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1003_200:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	vzeroupper
	callq	*%rax
.LBB1003_201:
	leaq	416(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
	jmp	.LBB1003_202
.LBB1003_210:
	movq	88(%rsp), %rcx
	movq	80(%rsp), %rax
	movq	96(%rsp), %rdx
	movq	%rcx, 288(%rsp)
	movq	104(%rsp), %rcx
	movq	%rax, 280(%rsp)
	movq	%rdx, 296(%rsp)
	movq	%rcx, 304(%rsp)
	movq	$-1, 272(%rsp)
	movq	184(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB1003_221
.LBB1003_212:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	192(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1003_214
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1003_214:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1003_220
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1003_214
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB1003_217:
	cmpq	%rax, %rsi
	jge	.LBB1003_219
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB1003_217
.LBB1003_219:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1003_220:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB1003_221:
	movq	208(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1003_224
	lock		decq	(%rax)
	jne	.LBB1003_224
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB1003_224:
	vmovups	304(%rsp), %zmm1
	vmovups	272(%rsp), %zmm0
	vmovups	%zmm1, 40(%rbx)
	vmovups	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
.LBB1003_202:
	movq	%rbx, %rax
	addq	$680, %rsp
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
.LBB1003_154:
	.cfi_def_cfa_offset 736
.Ltmp16624:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.717(%rip), %rdx
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
.Ltmp16625:
	jmp	.LBB1003_43
.LBB1003_146:
.Ltmp16621:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.717(%rip), %rdx
	vzeroupper
	callq	*%rax
.Ltmp16622:
	jmp	.LBB1003_43
.LBB1003_58:
.Ltmp16599:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$64, %esi
	callq	*%rax
.Ltmp16600:
	jmp	.LBB1003_43
.LBB1003_121:
.Ltmp16615:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp16616:
	jmp	.LBB1003_43
.LBB1003_138:
	movb	$1, %bpl
.Ltmp16630:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp16631:
.LBB1003_43:
	ud2
.LBB1003_164:
.Ltmp16620:
	jmp	.LBB1003_166
.LBB1003_163:
.Ltmp16629:
	jmp	.LBB1003_166
.LBB1003_204:
.Ltmp16617:
	movq	%rax, %r14
	jmp	.LBB1003_205
.LBB1003_174:
.Ltmp16634:
	movq	%rax, %r14
	jmp	.LBB1003_175
.LBB1003_94:
.Ltmp16595:
	movq	%rax, %r14
.Ltmp16596:
	leaq	112(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::result::Result<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>, purrdf_sparql_eval::error::EvalError>>
.Ltmp16597:
	jmp	.LBB1003_226
.LBB1003_103:
.Ltmp16611:
	movb	$1, %bl
	movq	%rax, %r14
	jmp	.LBB1003_104
.LBB1003_44:
.Ltmp16601:
	lock		decq	(%rbp)
	movq	%rax, %r14
	jne	.LBB1003_226
	#MEMBARRIER
.Ltmp16602:
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	272(%rsp), %rdi
	callq	*%rax
.Ltmp16603:
	jmp	.LBB1003_226
.LBB1003_83:
.Ltmp16572:
	jmp	.LBB1003_84
.LBB1003_72:
.Ltmp16575:
.LBB1003_84:
	lock		decq	(%r13)
	movq	%rax, %r14
	jne	.LBB1003_86
	#MEMBARRIER
.Ltmp16576:
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	272(%rsp), %rdi
	callq	*%rax
.Ltmp16577:
.LBB1003_86:
.Ltmp16579:
	leaq	112(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_algebra::ast::Variable>>
.Ltmp16580:
	jmp	.LBB1003_226
.LBB1003_89:
.Ltmp16578:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1003_90:
.Ltmp16604:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1003_225:
.Ltmp16641:
	movq	%rax, %r14
	jmp	.LBB1003_226
.LBB1003_122:
.Ltmp16614:
	movq	56(%rsp), %rsi
	movl	$8, %edx
	movq	%r13, %rdi
	movq	%rax, %r14
	callq	__rustc::__rust_dealloc
	jmp	.LBB1003_205
.LBB1003_91:
.Ltmp16583:
	movq	%rax, %r14
	movq	%rbx, 280(%rsp)
.Ltmp16584:
	leaq	112(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>
.Ltmp16585:
.Ltmp16587:
	leaq	272(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
.Ltmp16588:
	jmp	.LBB1003_226
.LBB1003_95:
.Ltmp16598:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1003_93:
.Ltmp16586:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1003_162:
.Ltmp16623:
	movq	%rax, %r14
	movq	%rbx, (%r12)
	jmp	.LBB1003_167
.LBB1003_165:
.Ltmp16626:
.LBB1003_166:
	movq	%rax, %r14
.LBB1003_167:
	movq	112(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB1003_169
	movq	120(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB1003_169:
	movq	16(%rsp), %rax
	leaq	384(%rsp), %rdi
	movq	%rax, 400(%rsp)
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movb	$1, %bpl
.LBB1003_175:
	cmpq	$0, 8(%rsp)
	je	.LBB1003_107
	movq	64(%rsp), %rdi
	movq	56(%rsp), %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB1003_107:
	testb	%bpl, %bpl
	je	.LBB1003_108
.LBB1003_205:
	movq	376(%rsp), %rax
	movb	$1, %bl
	lock		decq	(%rax)
	jne	.LBB1003_104
	#MEMBARRIER
.Ltmp16635:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	408(%rsp), %rdi
	callq	*%rax
.Ltmp16636:
	jmp	.LBB1003_104
.LBB1003_108:
	xorl	%ebx, %ebx
.LBB1003_104:
.Ltmp16637:
	leaq	416(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp16638:
	testb	%bl, %bl
	je	.LBB1003_106
.LBB1003_226:
.Ltmp16642:
	leaq	456(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp16643:
.LBB1003_106:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB1003_203:
.Ltmp16644:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end1003:
