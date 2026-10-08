purrdf_sparql_eval::expr::eval_extend::<purrdf_core::ir::dataset::RdfDataset>:
.Lfunc_begin241:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception161
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
	subq	$2648, %rsp
	.cfi_def_cfa_offset 2704
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	%rdi, 136(%rsp)
	leaq	1816(%rsp), %rdi
	movq	%r9, %r15
	movq	%r8, %r14
	movq	%rcx, %r13
	movq	%rdx, %rbx
	movq	%rsi, %r12
	callq	*%rax
.Ltmp6880:
	leaq	2528(%rsp), %rdi
	movq	%rbx, %rsi
	movq	%r15, 16(%rsp)
	movq	%r15, %rdx
	movq	%rbx, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp6881:
	cmpl	$1, 2528(%rsp)
	jne	.LBB241_25
	vmovdqu64	2544(%rsp), %zmm0
	vmovdqu64	2576(%rsp), %zmm1
	movq	136(%rsp), %rax
	vmovdqu64	%zmm1, 48(%rax)
	vmovdqu64	%zmm0, 16(%rax)
	movq	$1, (%rax)
	movq	1888(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB241_12
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	1896(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB241_5
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB241_5:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB241_11
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB241_5
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
.LBB241_8:
	cmpq	%rax, %rsi
	jge	.LBB241_10
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB241_8
.LBB241_10:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB241_11:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB241_12:
	movq	1816(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB241_22
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	1824(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB241_15
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB241_15:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB241_21
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB241_15
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
.LBB241_18:
	cmpq	%rax, %rsi
	jge	.LBB241_20
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB241_18
.LBB241_20:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB241_21:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB241_22:
	movq	1912(%rsp), %rax
	testq	%rax, %rax
	je	.LBB241_626
	lock		decq	(%rax)
	jne	.LBB241_626
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1912(%rsp), %rdi
	#MEMBARRIER
	jmp	.LBB241_625
.LBB241_25:
	vmovdqu64	2568(%rsp), %zmm1
	vmovdqu64	2536(%rsp), %zmm0
	vmovdqu64	%zmm1, 528(%rsp)
	vmovdqu64	%zmm0, 496(%rsp)
.Ltmp6882:
	leaq	928(%rsp), %rdi
	leaq	1816(%rsp), %rsi
	leaq	496(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp6883:
	cmpq	$-1, 928(%rsp)
	je	.LBB241_55
	vmovdqu	928(%rsp), %ymm0
	vmovdqu64	1856(%rsp), %zmm1
	vmovdqu64	1816(%rsp), %zmm2
	movb	$1, %r15b
	vmovdqu64	%zmm1, 1992(%rsp)
	vmovdqu	%ymm0, 384(%rsp)
	vmovdqu64	%zmm2, 1952(%rsp)
.Ltmp6884:
	movq	16(%rsp), %rbx
	movq	%r14, %rsi
	movq	%rbx, %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop
.Ltmp6885:
	cmpb	$2, 472(%rbx)
	sete	%cl
	andb	%cl, %al
	cmpb	$1, %al
	jne	.LBB241_34
	movq	616(%rbx), %rax
	testq	%rax, %rax
	je	.LBB241_33
	cmpq	$-2, 24(%rax)
	jb	.LBB241_34
	cmpq	$-2, 32(%rax)
	jb	.LBB241_34
	cmpq	$-3, 48(%rax)
	jbe	.LBB241_34
.LBB241_33:
	movq	16(%rsp), %rax
	cmpq	$1025, 400(%rsp)
	movzbl	1234(%rax), %ebx
	setae	%al
	notb	%bl
	andb	%al, %bl
	jmp	.LBB241_35
.LBB241_34:
	xorl	%ebx, %ebx
.LBB241_35:
	movq	400(%rsp), %rbp
.Ltmp6886:
	movq	16(%rsp), %rsi
	movzbl	%bl, %edx
	leaq	2328(%rsp), %rdi
	movq	%rbp, %rcx
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp6887:
	movq	408(%rsp), %rsi
	addq	$16, %rsi
.Ltmp6888:
	leaq	2056(%rsp), %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.18159039729619107857)
.Ltmp6889:
	movq	(%r13), %rsi
	lock		incq	(%rsi)
	jle	.LBB241_715
	movq	8(%r13), %rdx
.Ltmp6891:
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	2056(%rsp), %rdi
	callq	*%rax
.Ltmp6892:
	vmovdqu	2080(%rsp), %ymm0
	vmovdqu	2056(%rsp), %ymm1
	movq	%rax, 912(%rsp)
	movq	2072(%rsp), %rax
	movq	malloc@GOTPCREL(%rip), %r13
	movl	$72, %edi
	movq	%rax, 776(%rsp)
	vmovdqu	%ymm0, 536(%rsp)
	vmovdqu	%ymm1, 512(%rsp)
	movq	$1, 496(%rsp)
	movq	$1, 504(%rsp)
	vzeroupper
	callq	*%r13
	testq	%rax, %rax
	je	.LBB241_711
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rdx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rdx, %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB241_42
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB241_42:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB241_48
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB241_42
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	$72, (%rdx)
	movl	$72, %edx
	lock		xaddq	%rdx, (%rsi)
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rsi
	addq	$72, %rdx
	cmovoq	%rax, %rdx
	movq	(%rsi), %rax
	.p2align	4
.LBB241_45:
	cmpq	%rax, %rdx
	jle	.LBB241_47
	lock		cmpxchgq	%rdx, (%rsi)
	jne	.LBB241_45
.LBB241_47:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB241_48:
	vmovdqu64	496(%rsp), %zmm0
	movq	560(%rsp), %rax
	movq	%rcx, 120(%rsp)
	movq	%rax, 64(%rcx)
	movb	$1, %al
	movl	%eax, 8(%rsp)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp6896:
	movq	16(%rsp), %r15
	movq	%r12, 1400(%rsp)
	movq	%r12, %rsi
	movq	%r14, %rdx
	movq	%r15, %rdi
	vzeroupper
	callq	purrdf_sparql_eval::vm::program_at::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp6897:
	movq	120(%rsp), %rcx
	addq	$16, %rcx
.Ltmp6898:
	leaq	2112(%rsp), %r12
	movq	%rax, %rsi
	movq	%r14, %rdx
	movq	%r15, %r8
	movq	%r12, %rdi
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::link::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp6899:
	testb	%bl, %bl
	je	.LBB241_60
	movq	2512(%rsp), %rax
	movq	16(%rsp), %rdi
	movq	392(%rsp), %rbx
	cmpq	%rbp, %rax
	movq	1040(%rdi), %rcx
	cmovbq	%rax, %rbp
	addq	904(%rdi), %rcx
	movq	%rcx, 1720(%rsp)
.Ltmp6915:
	movq	%rbp, %rsi
	callq	<purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot
.Ltmp6916:
	movq	16(%rsp), %rcx
	leaq	2328(%rsp), %rdi
	movq	%rax, 352(%rsp)
	movq	616(%rcx), %rcx
	testq	%rcx, %rcx
	je	.LBB241_122
	cmpq	$-2, 16(%rcx)
	movb	$1, %al
	jb	.LBB241_123
	cmpq	$-2, 40(%rcx)
	setb	%al
	jmp	.LBB241_123
.LBB241_55:
	movq	1912(%rsp), %r14
	testq	%r14, %r14
	je	.LBB241_70
	lock		incq	(%r14)
	jle	.LBB241_715
	movq	%r14, 496(%rsp)
	leaq	16(%r14), %rsi
.Ltmp7222:
	leaq	2112(%rsp), %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.18159039729619107857)
.Ltmp7223:
	lock		decq	(%r14)
	jne	.LBB241_74
	#MEMBARRIER
.Ltmp7228:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	496(%rsp), %rdi
	callq	*%rax
.Ltmp7229:
	jmp	.LBB241_74
.LBB241_60:
	leaq	(,%rbp,8), %rax
	leaq	(%rax,%rax,4), %r14
	testq	%rbp, %rbp
	je	.LBB241_101
	movq	%r14, %rdi
	callq	*%r13
	testq	%rax, %rax
	je	.LBB241_713
	movq	%rax, %rdi
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rsi
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %r8
	movabsq	$-9223372036854775808, %rcx
	movq	$-1, %r9
	leaq	(%r14,%rax), %rdx
	sarq	$63, %rdx
	xorq	%rcx, %rdx
	addq	%r14, %rax
	cmovoq	%rdx, %rax
	incq	%rsi
	cmoveq	%r9, %rsi
	addq	%r14, %r8
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmovbq	%r9, %r8
	movq	%rsi, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%r8, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB241_64
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB241_64:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB241_102
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB241_64
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	%r14, (%rdx)
	movq	%r14, %rdx
	lock		xaddq	%rdx, (%rsi)
	leaq	(%rdx,%r14), %rax
	sarq	$63, %rax
	xorq	%rcx, %rax
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	addq	%r14, %rdx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB241_67:
	cmpq	%rax, %rdx
	jle	.LBB241_69
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB241_67
.LBB241_69:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jmp	.LBB241_102
.LBB241_70:
.Ltmp7230:
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp7231:
	movq	%rax, %rsi
	movq	%rax, %r14
	movq	%rax, 496(%rsp)
	addq	$16, %rsi
.Ltmp7232:
	leaq	2112(%rsp), %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.18159039729619107857)
.Ltmp7233:
	lock		decq	(%r14)
	jne	.LBB241_74
	#MEMBARRIER
.Ltmp7238:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	496(%rsp), %rdi
	callq	*%rax
.Ltmp7239:
.LBB241_74:
	movq	(%r13), %rsi
	lock		incq	(%rsi)
	jle	.LBB241_715
	movq	8(%r13), %rdx
.Ltmp7241:
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	2112(%rsp), %rdi
	callq	*%rax
.Ltmp7242:
	vmovdqu64	1856(%rsp), %zmm1
	vmovups	1816(%rsp), %zmm0
	vmovdqu	2136(%rsp), %ymm2
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$72, %edi
	vmovdqu64	%zmm1, 536(%rsp)
	vmovups	%zmm0, 496(%rsp)
	vmovdqu	2112(%rsp), %ymm0
	vmovdqu	%ymm2, 968(%rsp)
	vmovdqu	%ymm0, 944(%rsp)
	movq	$1, 928(%rsp)
	movq	$1, 936(%rsp)
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB241_712
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rdx
	movabsq	$9223372036854775807, %rbx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rbx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB241_79
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB241_79:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB241_85
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB241_79
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	$72, (%rdx)
	movl	$72, %edx
	lock		xaddq	%rdx, (%rsi)
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rsi
	movq	(%rsi), %rax
	addq	$72, %rdx
	cmovoq	%rbx, %rdx
	.p2align	4
.LBB241_82:
	cmpq	%rax, %rdx
	jle	.LBB241_84
	lock		cmpxchgq	%rdx, (%rsi)
	jne	.LBB241_82
.LBB241_84:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB241_85:
	vmovups	928(%rsp), %zmm0
	movq	992(%rsp), %rax
	cmpq	$-1, 496(%rsp)
	movq	%rcx, 1480(%rsp)
	movq	$0, 1456(%rsp)
	movq	$8, 1464(%rsp)
	movq	$0, 1472(%rsp)
	movq	%rax, 64(%rcx)
	vmovups	%zmm0, (%rcx)
	je	.LBB241_87
	leaq	928(%rsp), %rdi
	leaq	1456(%rsp), %rsi
	leaq	1816(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	568(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB241_88
	jmp	.LBB241_97
.LBB241_87:
	movq	1464(%rsp), %rcx
	movq	1456(%rsp), %rax
	movq	1472(%rsp), %rdx
	movq	%rcx, 944(%rsp)
	movq	1480(%rsp), %rcx
	movq	%rax, 936(%rsp)
	movq	%rdx, 952(%rsp)
	movq	%rcx, 960(%rsp)
	movq	$-1, 928(%rsp)
	movq	568(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB241_97
.LBB241_88:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	576(%rsp), %rdi
	cmpq	%rbx, %rcx
	cmovaeq	%rbx, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rbx, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB241_90
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB241_90:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB241_96
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB241_90
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rbx, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB241_93:
	cmpq	%rax, %rdx
	jge	.LBB241_95
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB241_93
.LBB241_95:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB241_96:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB241_97:
	movq	592(%rsp), %rax
	testq	%rax, %rax
	je	.LBB241_100
	lock		decq	(%rax)
	jne	.LBB241_100
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	592(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB241_100:
	vmovdqu64	928(%rsp), %zmm0
	vmovdqu64	960(%rsp), %zmm1
	movq	136(%rsp), %rax
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	jmp	.LBB241_626
.LBB241_101:
	movl	$8, %edi
.LBB241_102:
	movq	392(%rsp), %rax
	movq	384(%rsp), %rcx
	movq	%rbp, 160(%rsp)
	movq	%rdi, 168(%rsp)
	movq	$0, 176(%rsp)
	addq	%rax, %r14
	movq	%rax, 1456(%rsp)
	movq	%rcx, 1472(%rsp)
	movq	%rcx, 48(%rsp)
	movq	%rax, 80(%rsp)
	movq	%r14, 1480(%rsp)
	testq	%rbp, %rbp
	je	.LBB241_121
	leaq	936(%rsp), %rbp
	xorl	%ebx, %ebx
	movq	%rdi, 8(%rsp)
	jmp	.LBB241_105
	.p2align	4
.LBB241_104:
	movq	40(%rsp), %rcx
	leaq	(%rbx,%rbx,4), %rax
	movq	%rdx, 8(%rsp)
	movq	%r13, %rbx
	movq	%rbp, (%rdx,%rax,8)
	movq	%r12, %rbp
	movq	%rcx, 8(%rdx,%rax,8)
	vmovdqa	496(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rdx,%rax,8)
	movq	512(%rsp), %rcx
	movq	%rcx, 32(%rdx,%rax,8)
	movq	%r13, 176(%rsp)
	movq	%r15, %rax
	cmpq	%r14, %r15
	je	.LBB241_180
.LBB241_105:
	movq	(%rax), %rcx
	leaq	40(%rax), %r15
	testq	%rcx, %rcx
	je	.LBB241_181
	vmovdqu	8(%rax), %ymm0
	leaq	1(%rbx), %r13
	movq	%rbx, %r12
	movq	%rcx, 928(%rsp)
	vmovdqu	%ymm0, 1168(%rsp)
	vmovdqu	%ymm0, (%rbp)
.Ltmp6902:
	movq	16(%rsp), %rdx
	leaq	496(%rsp), %rdi
	leaq	2328(%rsp), %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp6903:
	cmpb	$-1, 496(%rsp)
	jne	.LBB241_145
	movq	776(%rsp), %rsi
.Ltmp6904:
	leaq	928(%rsp), %rdi
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::resize
.Ltmp6905:
	movq	928(%rsp), %rcx
	movq	16(%rsp), %rax
	movq	%rbp, %rdx
	decq	%rcx
	movq	%r12, 568(%rax)
	cmpq	$5, %rcx
	jb	.LBB241_111
	movq	944(%rsp), %rcx
	movq	936(%rsp), %rdx
	decq	%rcx
.LBB241_111:
	movq	120(%rsp), %r8
	addq	$16, %r8
.Ltmp6906:
	movq	16(%rsp), %r9
	leaq	496(%rsp), %rdi
	leaq	2112(%rsp), %rsi
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp6907:
	vmovq	504(%rsp), %xmm0
	movq	496(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB241_147
	movq	928(%rsp), %rax
	movq	%rax, %rsi
	cmpq	$6, %rax
	jb	.LBB241_115
	movq	944(%rsp), %rsi
.LBB241_115:
	movq	912(%rsp), %rdi
	decq	%rsi
	cmpq	%rsi, %rdi
	jae	.LBB241_710
	movq	8(%rsp), %rdx
	movq	%rbp, %r12
	movq	%rbp, %rcx
	cmpq	$6, %rax
	jb	.LBB241_118
	movq	936(%rsp), %rcx
.LBB241_118:
	vmovq	%xmm0, (%rcx,%rdi,8)
	vmovdqu	8(%r12), %xmm0
	movq	936(%rsp), %rax
	movq	24(%r12), %rcx
	movq	928(%rsp), %rbp
	movq	%rax, 40(%rsp)
	movq	%rcx, 512(%rsp)
	vmovdqa	%xmm0, 496(%rsp)
	cmpq	160(%rsp), %rbx
	jne	.LBB241_104
.Ltmp6912:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	callq	*%rax
.Ltmp6913:
	movq	168(%rsp), %rdx
	jmp	.LBB241_104
.LBB241_121:
	xorl	%ebx, %ebx
	movq	%rax, %r15
	jmp	.LBB241_181
.LBB241_122:
	xorl	%eax, %eax
.LBB241_123:
	movq	16(%rsp), %rdx
	leaq	352(%rsp), %r8
	leaq	776(%rsp), %r9
	movq	%rbx, 1456(%rsp)
	movq	%rbp, 1464(%rsp)
	movq	%r9, 1472(%rsp)
	leaq	912(%rsp), %r9
	movzbl	1234(%rdx), %ecx
	movq	%rdx, 1168(%rsp)
	movq	%r8, 1176(%rsp)
	leaq	120(%rsp), %r8
	movq	%rdi, 1184(%rsp)
	movq	%r12, 1192(%rsp)
	movq	%r8, 1480(%rsp)
	leaq	1720(%rsp), %r8
	movq	%r9, 1488(%rsp)
	movq	%r8, 1496(%rsp)
	movzbl	%cl, %esi
	testb	%al, %al
	je	.LBB241_125
.Ltmp6919:
	movq	%rdi, (%rsp)
	leaq	496(%rsp), %rdi
	leaq	1168(%rsp), %r8
	leaq	1456(%rsp), %r9
	movq	%rbx, %rdx
	movq	%rbp, %rcx
	callq	purrdf_sparql_eval::parallel::par_blocks_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>
.Ltmp6920:
	jmp	.LBB241_126
.LBB241_125:
.Ltmp6917:
	movq	%rdi, (%rsp)
	leaq	496(%rsp), %rdi
	leaq	1168(%rsp), %r8
	leaq	1456(%rsp), %r9
	movq	%rbx, %rdx
	movq	%rbp, %rcx
	callq	purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>
.Ltmp6918:
.LBB241_126:
	movq	496(%rsp), %rbx
	cmpq	$-1, %rbx
	je	.LBB241_134
	vmovups	688(%rsp), %zmm0
	vmovups	672(%rsp), %zmm1
	movq	520(%rsp), %rax
	movq	536(%rsp), %rdx
	movq	504(%rsp), %rbp
	movq	512(%rsp), %r13
	movq	528(%rsp), %rcx
	movl	$1, %edi
	movq	%rbx, 864(%rsp)
	leaq	-3(%rax), %rsi
	movq	%rbp, 872(%rsp)
	movq	%r13, 880(%rsp)
	cmpq	$-2, %rsi
	movl	$1, %esi
	cmovbq	%rax, %rdi
	cmovbq	%rdx, %rax
	cmovaeq	%rdx, %rsi
	vmovups	%zmm0, 1072(%rsp)
	vmovups	%zmm1, 1056(%rsp)
	vmovdqu64	544(%rsp), %zmm0
	vmovdqu64	608(%rsp), %zmm1
	decq	%rax
	vmovdqu64	1072(%rsp), %zmm3
	vmovdqu64	1056(%rsp), %zmm2
	vmovdqu64	%zmm0, 928(%rsp)
	vmovdqu64	%zmm1, 992(%rsp)
	vmovdqu64	%zmm1, 584(%rsp)
	vmovdqu64	%zmm0, 520(%rsp)
	vmovdqu64	%zmm3, 664(%rsp)
	vmovdqu64	%zmm2, 648(%rsp)
	movq	%rdi, 496(%rsp)
	movq	%rcx, 504(%rsp)
	movq	%rsi, 512(%rsp)
	movq	$0, 728(%rsp)
	movq	%rax, 736(%rsp)
.Ltmp6922:
	leaq	928(%rsp), %rdi
	leaq	496(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp6923:
	vmovups	1104(%rsp), %zmm2
	vmovups	1088(%rsp), %zmm1
	movq	16(%rsp), %rax
	vmovups	928(%rsp), %ymm0
	movq	616(%rax), %r14
	leaq	888(%rax), %rcx
	movq	%rcx, 48(%rsp)
	vmovups	%zmm2, 1600(%rsp)
	vmovups	%zmm1, 1584(%rsp)
	vmovups	1024(%rsp), %zmm2
	vmovups	960(%rsp), %zmm1
	vmovups	%ymm0, 1728(%rsp)
	vmovups	%zmm2, 1520(%rsp)
	vmovups	%zmm1, 1456(%rsp)
	testq	%r14, %r14
	je	.LBB241_135
	vmovdqu	864(%rsp), %xmm0
	vmovdqu64	1456(%rsp), %zmm4
	vmovdqu64	1520(%rsp), %zmm1
	vmovdqu64	1600(%rsp), %zmm3
	vmovdqu64	1584(%rsp), %zmm2
	movq	880(%rsp), %rax
	cmpb	$2, 2522(%rsp)
	movq	%rax, 1936(%rsp)
	vmovdqu64	%zmm3, 1072(%rsp)
	vmovdqa	%xmm0, 1920(%rsp)
	vmovdqu64	%zmm2, 1056(%rsp)
	vmovdqu64	%zmm1, 992(%rsp)
	vmovdqu64	%zmm4, 928(%rsp)
	jne	.LBB241_141
	movq	928(%rsp), %rax
	vmovdqu64	1480(%rsp), %zmm0
	vmovdqu64	1600(%rsp), %zmm2
	vmovdqu64	1544(%rsp), %zmm1
	movq	944(%rsp), %rdx
	movq	936(%rsp), %rcx
	movl	$1, %edi
	movl	$1, %esi
	cmpq	$3, %rax
	cmovaeq	%rax, %rdi
	cmovaeq	%rdx, %rax
	cmovaeq	%rsi, %rdx
	decq	%rax
	vmovdqu64	%zmm2, 640(%rsp)
	vmovdqu64	%zmm1, 584(%rsp)
	vmovdqu64	%zmm0, 520(%rsp)
	movq	%rdi, 496(%rsp)
	movq	%rcx, 504(%rsp)
	movq	%rdx, 512(%rsp)
	movq	$0, 704(%rsp)
	movq	%rax, 712(%rsp)
.Ltmp6954:
	leaq	1760(%rsp), %rdi
	leaq	496(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp6955:
	movq	1776(%rsp), %rcx
	movq	1768(%rsp), %r12
	imulq	$200, %rcx, %rax
	addq	%r12, %rax
	movq	%rax, 8(%rsp)
	testq	%rcx, %rcx
	je	.LBB241_253
	movl	%ecx, %esi
	andl	$3, %esi
	cmpq	$4, %rcx
	jae	.LBB241_227
	xorl	%edi, %edi
	xorl	%r10d, %r10d
	jmp	.LBB241_246
.LBB241_134:
	vmovdqu64	544(%rsp), %zmm0
	vmovdqu	512(%rsp), %ymm1
	movq	136(%rsp), %rax
	vmovdqu64	%zmm0, 48(%rax)
	vmovdqu	%ymm1, 16(%rax)
	vmovdqu64	%zmm0, 928(%rsp)
	movq	$1, (%rax)
	movq	352(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB241_317
	jmp	.LBB241_319
.LBB241_135:
	vmovups	1520(%rsp), %zmm1
	movq	880(%rsp), %rax
	vmovdqu	864(%rsp), %xmm0
	vmovdqu64	1456(%rsp), %zmm4
	vmovdqu64	1600(%rsp), %zmm3
	vmovdqu64	1584(%rsp), %zmm2
	movq	%rax, 848(%rsp)
	movq	400(%rsp), %rax
	vmovups	%zmm1, 560(%rsp)
	vmovdqa	384(%rsp), %xmm1
	vmovdqu64	%zmm3, 640(%rsp)
	movq	$0, 384(%rsp)
	movq	$8, 392(%rsp)
	vmovdqa	%xmm0, 832(%rsp)
	vmovdqu64	%zmm2, 624(%rsp)
	vmovdqu64	%zmm4, 496(%rsp)
	movq	$0, 400(%rsp)
	movq	%rax, 1376(%rsp)
	movzbl	2522(%rsp), %eax
	movb	%al, 56(%rsp)
	vmovdqa	%xmm1, 1360(%rsp)
	testb	%al, %al
	jne	.LBB241_707
	vmovdqa	1360(%rsp), %xmm0
	movq	1376(%rsp), %rax
	movq	%rbx, 80(%rsp)
	movq	%rax, 320(%rsp)
	vmovdqa	%xmm0, 304(%rsp)
.Ltmp7093:
	leaq	928(%rsp), %rdi
	leaq	304(%rsp), %rsi
	movq	%r13, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp7094:
	movq	928(%rsp), %r12
	movq	936(%rsp), %rbx
	movq	944(%rsp), %r14
	movq	952(%rsp), %r15
	cmpq	$-1, %r12
	je	.LBB241_207
	vmovdqa	960(%rsp), %xmm0
	vmovdqu	976(%rsp), %ymm2
	vmovdqu	992(%rsp), %ymm1
	vmovdqu	%ymm2, 160(%rsp)
	vmovdqa	%xmm0, 16(%rsp)
	vmovdqu	%ymm1, 176(%rsp)
.Ltmp7098:
	leaq	864(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7099:
	movq	%r15, %rbp
.LBB241_139:
	vmovups	160(%rsp), %ymm0
	vmovups	176(%rsp), %ymm1
	vmovups	%ymm0, 416(%rsp)
	vmovups	%ymm1, 432(%rsp)
.Ltmp7106:
	leaq	496(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp7107:
	vmovups	416(%rsp), %ymm0
	vmovdqu	432(%rsp), %ymm1
	movq	136(%rsp), %rcx
	vmovdqu	%ymm1, 80(%rcx)
	vmovups	%ymm0, 64(%rcx)
	vmovdqa	16(%rsp), %xmm0
	jmp	.LBB241_315
.LBB241_141:
	movq	$0, 304(%rsp)
	movq	$8, 312(%rsp)
	movq	$0, 320(%rsp)
.Ltmp6927:
	leaq	496(%rsp), %rdi
	leaq	304(%rsp), %rsi
	movq	%r13, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp6928:
	movq	496(%rsp), %r12
	movq	504(%rsp), %rax
	movq	512(%rsp), %r14
	movq	520(%rsp), %r15
	cmpq	$-1, %r12
	je	.LBB241_217
	vmovdqu	544(%rsp), %ymm0
	vmovdqu	560(%rsp), %ymm1
	movq	528(%rsp), %rbx
	movq	536(%rsp), %r13
	movq	%rax, 48(%rsp)
	vmovdqu	%ymm0, 160(%rsp)
	vmovdqu	%ymm1, 176(%rsp)
.Ltmp6932:
	leaq	864(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp6933:
	movq	%r15, %rbp
.LBB241_144:
	vmovups	160(%rsp), %ymm0
	vmovups	176(%rsp), %ymm1
	movq	%rbx, %r15
	shrq	$8, %r15
	vmovups	%ymm0, 1408(%rsp)
	vmovups	%ymm1, 1424(%rsp)
	jmp	.LBB241_312
.LBB241_145:
	movq	928(%rsp), %rax
	movq	%r15, 1464(%rsp)
	movq	%r13, 1488(%rsp)
	cmpq	$6, %rax
	jb	.LBB241_182
	movq	936(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB241_182
.LBB241_147:
	vmovdqu64	512(%rsp), %zmm1
	vmovdqu64	528(%rsp), %zmm2
	movq	136(%rsp), %rcx
	vmovdqu64	%zmm2, 48(%rcx)
	vmovdqu64	%zmm1, 32(%rcx)
	movq	%rax, 16(%rcx)
	movq	928(%rsp), %rax
	vmovq	%xmm0, 24(%rcx)
	movq	$1, (%rcx)
	cmpq	$6, %rax
	jb	.LBB241_149
	movq	936(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB241_149:
	movq	8(%rsp), %r13
	subq	%r15, %r14
	je	.LBB241_162
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	shrq	$3, %r14
	movabsq	$-3689348814741910323, %rbx
	imulq	%r14, %rbx
	xorl	%r14d, %r14d
	jmp	.LBB241_154
.LBB241_151:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB241_152:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB241_153:
	incq	%r14
	cmpq	%rbx, %r14
	je	.LBB241_162
.LBB241_154:
	leaq	(%r14,%r14,4), %rcx
	movq	(%r15,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB241_153
	leaq	(%r15,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB241_157
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB241_157:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB241_152
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB241_157
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rbp), %rax
	.p2align	4
.LBB241_160:
	cmpq	%rax, %rdx
	jge	.LBB241_151
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB241_160
	jmp	.LBB241_151
.LBB241_162:
	movq	48(%rsp), %rax
	testq	%rax, %rax
	je	.LBB241_164
	movq	80(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB241_164:
	testq	%r12, %r12
	je	.LBB241_177
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r14
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r15
	movq	free@GOTPCREL(%rip), %rbp
	xorl	%ebx, %ebx
	jmp	.LBB241_169
.LBB241_166:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB241_167:
	vzeroupper
	callq	*%rbp
.LBB241_168:
	incq	%rbx
	cmpq	%rbx, %r12
	je	.LBB241_177
.LBB241_169:
	leaq	(%rbx,%rbx,4), %rcx
	movq	(%r13,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB241_168
	leaq	(%r13,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB241_172
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB241_172:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB241_167
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB241_172
	movq	%rcx, %rdx
	negq	%rdx
	xorl	%eax, %eax
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%r14)
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r15), %rax
	.p2align	4
.LBB241_175:
	cmpq	%rax, %rdx
	jge	.LBB241_166
	lock		cmpxchgq	%rdx, (%r15)
	jne	.LBB241_175
	jmp	.LBB241_166
.LBB241_177:
	movq	160(%rsp), %rax
	testq	%rax, %rax
	je	.LBB241_179
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r13, %rdi
	leaq	(%rax,%rax,4), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB241_179:
	movl	$0, 8(%rsp)
	jmp	.LBB241_320
.LBB241_180:
	movq	%r13, %rbx
	movq	%r14, %r15
.LBB241_181:
	movq	%r15, 1464(%rsp)
	movq	%rbx, 1488(%rsp)
.LBB241_182:
	subq	%r15, %r14
	je	.LBB241_195
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r12
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	shrq	$3, %r14
	movabsq	$-3689348814741910323, %rbx
	imulq	%r14, %rbx
	xorl	%r14d, %r14d
	jmp	.LBB241_187
	.p2align	4
.LBB241_184:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB241_185:
	callq	*%rbp
.LBB241_186:
	incq	%r14
	cmpq	%rbx, %r14
	je	.LBB241_195
.LBB241_187:
	leaq	(%r14,%r14,4), %rcx
	movq	(%r15,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB241_186
	leaq	(%r15,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB241_190
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB241_190:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB241_185
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB241_190
	movq	%rcx, %rdx
	negq	%rdx
	xorl	%eax, %eax
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%r12)
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB241_193:
	cmpq	%rax, %rdx
	jge	.LBB241_184
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB241_193
	jmp	.LBB241_184
.LBB241_195:
	movq	48(%rsp), %rax
	movq	1400(%rsp), %rbx
	testq	%rax, %rax
	je	.LBB241_206
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB241_198
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB241_198:
	movq	80(%rsp), %rdi
	.p2align	4
.LBB241_199:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB241_205
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB241_199
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB241_202:
	cmpq	%rax, %rdx
	jge	.LBB241_204
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB241_202
.LBB241_204:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB241_205:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB241_206:
	vmovdqu	160(%rsp), %xmm0
	movq	176(%rsp), %rax
	xorl	%r14d, %r14d
	movq	%rax, 1344(%rsp)
	vmovdqa	%xmm0, 1328(%rsp)
	jmp	.LBB241_565
.LBB241_207:
	leaq	(,%r13,8), %rax
	movq	80(%rsp), %rsi
	movq	%r13, %rcx
	movq	%rbx, 1408(%rsp)
	movq	%r14, 1416(%rsp)
	movq	%rbp, 784(%rsp)
	movq	%r15, 1424(%rsp)
	leaq	(%rax,%rax,4), %r13
	leaq	(%rbp,%r13), %rdx
	movq	%rsi, 800(%rsp)
	movq	%rdx, 64(%rsp)
	movq	%rdx, 808(%rsp)
	testq	%rcx, %rcx
	je	.LBB241_276
	leaq	(,%r15,8), %rax
	movq	%r14, 8(%rsp)
	movq	%r15, %r14
	addq	$40, %rbp
	leaq	(%rax,%rax,4), %r15
	jmp	.LBB241_211
.LBB241_209:
	movq	1416(%rsp), %rax
	movq	%rax, 8(%rsp)
.LBB241_210:
	vmovdqa	80(%rsp), %xmm0
	movq	8(%rsp), %rax
	movq	%rbx, (%rax,%r15)
	movq	%r12, 8(%rax,%r15)
	vmovdqu	%xmm0, 16(%rax,%r15)
	movq	%rbp, 32(%rax,%r15)
	movq	40(%rsp), %rbp
	movq	%r14, %rax
	incq	%rax
	addq	$40, %r15
	movq	%rax, %r14
	movq	%rax, 1424(%rsp)
	addq	$40, %rbp
	addq	$-40, %r13
	je	.LBB241_274
.LBB241_211:
	movq	-8(%rbp), %rax
	leaq	1176(%rsp), %rcx
	movq	%rbp, 40(%rsp)
	movq	%rax, 32(%rcx)
	movq	16(%rsp), %rax
	vmovdqu	-40(%rbp), %ymm0
	vmovdqu	%ymm0, (%rcx)
	movq	%rax, 1168(%rsp)
	cmpq	$0, 1176(%rsp)
	je	.LBB241_213
	leaq	-40(%rbp), %rax
	leaq	936(%rsp), %rdx
	movq	32(%rax), %rcx
	movq	%rcx, 32(%rdx)
	vmovdqu	(%rax), %ymm0
	vmovdqu	%ymm0, (%rdx)
	jmp	.LBB241_215
.LBB241_213:
	movq	664(%rax), %rdx
.Ltmp7101:
	movq	48(%rsp), %rsi
	leaq	928(%rsp), %rdi
	leaq	1184(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7102:
	movq	928(%rsp), %r12
	cmpq	$-1, %r12
	jne	.LBB241_373
.LBB241_215:
	vmovdqu	952(%rsp), %xmm0
	movq	936(%rsp), %rbx
	movq	944(%rsp), %r12
	movq	968(%rsp), %rbp
	vmovdqa	%xmm0, 80(%rsp)
	cmpq	1408(%rsp), %r14
	jne	.LBB241_210
.Ltmp7111:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	1408(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp7112:
	jmp	.LBB241_209
.LBB241_217:
	movq	%rax, 784(%rsp)
	leaq	(,%r13,8), %rax
	movq	%r13, %rcx
	movq	%r14, 792(%rsp)
	movq	%rbp, 416(%rsp)
	movq	%r15, 800(%rsp)
	movq	%rbx, 432(%rsp)
	leaq	(%rax,%rax,4), %r13
	leaq	(%rbp,%r13), %rax
	movq	%rax, 440(%rsp)
	testq	%rcx, %rcx
	je	.LBB241_309
	movq	%rax, 56(%rsp)
	leaq	(,%r15,8), %rax
	movq	%r14, 8(%rsp)
	movq	%r15, %r14
	addq	$40, %rbp
	leaq	(%rax,%rax,4), %r15
	jmp	.LBB241_221
.LBB241_219:
	movq	792(%rsp), %rax
	movq	%rax, 8(%rsp)
.LBB241_220:
	vmovdqa	80(%rsp), %xmm0
	movq	8(%rsp), %rax
	movq	%rbx, (%rax,%r15)
	movq	%r12, 8(%rax,%r15)
	vmovdqu	%xmm0, 16(%rax,%r15)
	movq	%rbp, 32(%rax,%r15)
	movq	40(%rsp), %rbp
	movq	%r14, %rax
	incq	%rax
	addq	$40, %r15
	movq	%rax, %r14
	movq	%rax, 800(%rsp)
	addq	$40, %rbp
	addq	$-40, %r13
	je	.LBB241_275
.LBB241_221:
	movq	16(%rsp), %rcx
	leaq	1176(%rsp), %rdx
	movq	%rbp, 40(%rsp)
	movq	%rcx, 1168(%rsp)
	movq	-8(%rbp), %rax
	movq	%rax, 32(%rdx)
	vmovdqu	-40(%rbp), %ymm0
	vmovdqu	%ymm0, (%rdx)
	cmpq	$0, 1176(%rsp)
	je	.LBB241_223
	leaq	-40(%rbp), %rax
	leaq	504(%rsp), %rdx
	movq	32(%rax), %rcx
	movq	%rcx, 32(%rdx)
	vmovdqu	(%rax), %ymm0
	vmovdqu	%ymm0, (%rdx)
	jmp	.LBB241_225
.LBB241_223:
	movq	664(%rcx), %rdx
.Ltmp6935:
	movq	48(%rsp), %rsi
	leaq	496(%rsp), %rdi
	leaq	1184(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp6936:
	movq	496(%rsp), %r12
	cmpq	$-1, %r12
	jne	.LBB241_375
.LBB241_225:
	vmovdqu	520(%rsp), %xmm0
	movq	504(%rsp), %rbx
	movq	512(%rsp), %r12
	movq	536(%rsp), %rbp
	vmovdqa	%xmm0, 80(%rsp)
	cmpq	784(%rsp), %r14
	jne	.LBB241_220
.Ltmp6940:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	784(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp6941:
	jmp	.LBB241_219
.LBB241_227:
	movq	%rcx, %r8
	andq	$-4, %r8
	leaq	776(%r12), %r9
	xorl	%edi, %edi
	xorl	%r10d, %r10d
	jmp	.LBB241_229
	.p2align	4
.LBB241_228:
	addq	$4, %rdi
	addq	$800, %r9
	cmpq	%rdi, %r8
	je	.LBB241_245
.LBB241_229:
	movq	-600(%r9), %rax
	mulq	-608(%r9)
	jo	.LBB241_238
	addq	%rax, %r10
	movq	$-1, %r11
	jb	.LBB241_231
.LBB241_239:
	movq	%r10, %r11
	movq	-400(%r9), %rax
	mulq	-408(%r9)
	jno	.LBB241_232
.LBB241_240:
	movq	$-1, %rax
	addq	%rax, %r11
	movq	$-1, %r10
	jae	.LBB241_241
	.p2align	4
.LBB241_233:
	movq	-200(%r9), %rax
	mulq	-208(%r9)
	jo	.LBB241_242
.LBB241_234:
	addq	%rax, %r10
	movq	$-1, %r11
	jb	.LBB241_235
.LBB241_243:
	movq	%r10, %r11
	movq	(%r9), %rax
	mulq	-8(%r9)
	jno	.LBB241_236
.LBB241_244:
	movq	$-1, %rax
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB241_228
	jmp	.LBB241_237
.LBB241_238:
	movq	$-1, %rax
	addq	%rax, %r10
	movq	$-1, %r11
	jae	.LBB241_239
	.p2align	4
.LBB241_231:
	movq	-400(%r9), %rax
	mulq	-408(%r9)
	jo	.LBB241_240
.LBB241_232:
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB241_233
.LBB241_241:
	movq	%r11, %r10
	movq	-200(%r9), %rax
	mulq	-208(%r9)
	jno	.LBB241_234
.LBB241_242:
	movq	$-1, %rax
	addq	%rax, %r10
	movq	$-1, %r11
	jae	.LBB241_243
	.p2align	4
.LBB241_235:
	movq	(%r9), %rax
	mulq	-8(%r9)
	jo	.LBB241_244
.LBB241_236:
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB241_228
.LBB241_237:
	movq	%r11, %r10
	jmp	.LBB241_228
.LBB241_245:
	testq	%rsi, %rsi
	je	.LBB241_250
.LBB241_246:
	imulq	$200, %rdi, %rax
	imulq	$200, %rsi, %rsi
	movq	$-1, %r9
	xorl	%r8d, %r8d
	leaq	176(%rax,%r12), %rdi
	.p2align	4
.LBB241_247:
	movq	(%rdi,%r8), %rax
	mulq	-8(%rdi,%r8)
	jo	.LBB241_249
.LBB241_248:
	addq	%rax, %r10
	cmovbq	%r9, %r10
	addq	$200, %r8
	cmpq	%r8, %rsi
	jne	.LBB241_247
	jmp	.LBB241_250
.LBB241_249:
	movq	$-1, %rax
	jmp	.LBB241_248
.LBB241_250:
	testq	%r10, %r10
	je	.LBB241_253
	cmpq	$0, 336(%r14)
	je	.LBB241_253
	lock		addq	%r10, 352(%r14)
.LBB241_253:
	movq	1760(%rsp), %rax
	movq	8(%rsp), %rdx
	movq	%r12, 1168(%rsp)
	movq	$0, 160(%rsp)
	movq	$8, 168(%rsp)
	movq	%rbx, 80(%rsp)
	movq	$0, 176(%rsp)
	movq	%rax, 1184(%rsp)
	movq	%rdx, 1192(%rsp)
	testq	%rcx, %rcx
	je	.LBB241_262
	movq	%rbp, 40(%rsp)
	leaq	504(%rsp), %rbp
	addq	$200, %r12
	movl	$8, %ecx
	movq	%r13, 56(%rsp)
	xorl	%r13d, %r13d
	xorl	%r15d, %r15d
	.p2align	4
.LBB241_255:
	movq	-200(%r12), %rax
	cmpq	$-1, %rax
	je	.LBB241_263
	leaq	-200(%r12), %r14
	vmovups	8(%r14), %zmm0
	vmovups	72(%r14), %zmm1
	vmovups	96(%r14), %zmm2
	vmovups	%zmm2, 88(%rbp)
	vmovups	%zmm1, 64(%rbp)
	vmovups	%zmm0, (%rbp)
	movq	%rax, 496(%rsp)
	movzbl	648(%rsp), %ebx
	cmpq	160(%rsp), %r13
	jne	.LBB241_259
.Ltmp6957:
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp6958:
	movq	168(%rsp), %rcx
.LBB241_259:
	vmovdqu64	496(%rsp), %zmm0
	vmovdqu64	560(%rsp), %zmm1
	vmovdqu64	592(%rsp), %zmm2
	leaq	1(%r13), %rax
	vmovdqu64	%zmm2, 96(%rcx,%r15)
	vmovdqu64	%zmm1, 64(%rcx,%r15)
	vmovdqu64	%zmm0, (%rcx,%r15)
	movq	%rax, 176(%rsp)
	testb	%bl, %bl
	jne	.LBB241_266
	addq	$160, %r15
	addq	$200, %r12
	addq	$200, %r14
	movq	%rax, %r13
	cmpq	8(%rsp), %r14
	jne	.LBB241_255
	movq	8(%rsp), %r12
	movq	%rcx, %rbx
	jmp	.LBB241_264
.LBB241_262:
	movl	$8, %ebx
	xorl	%eax, %eax
	jmp	.LBB241_265
.LBB241_263:
	movq	%rcx, %rbx
	movq	%r13, %rax
.LBB241_264:
	movq	40(%rsp), %rbp
	movq	56(%rsp), %r13
.LBB241_265:
	movq	%r12, 1176(%rsp)
	movl	$0, 156(%rsp)
	movq	%rax, %r12
	jmp	.LBB241_267
.LBB241_266:
	incq	%r13
	movq	%r12, 1176(%rsp)
	movq	40(%rsp), %rbp
	movb	$1, %al
	movq	%rcx, %rbx
	movq	%r13, %r12
	movq	56(%rsp), %r13
	movl	%eax, 156(%rsp)
.LBB241_267:
.Ltmp6965:
	leaq	1168(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp6966:
	movq	160(%rsp), %rax
	movq	%rax, 344(%rsp)
	testq	%r12, %r12
	je	.LBB241_271
	cmpq	$8, %r12
	jae	.LBB241_272
	movq	%r13, %r14
	xorl	%eax, %eax
	xorl	%edx, %edx
	jmp	.LBB241_287
.LBB241_271:
	movq	%r13, %r14
	xorl	%edx, %edx
	jmp	.LBB241_289
.LBB241_272:
	cmpq	$32, %r12
	jae	.LBB241_280
	movq	%r13, %r14
	xorl	%eax, %eax
	xorl	%edx, %edx
	jmp	.LBB241_284
.LBB241_274:
	movq	64(%rsp), %rax
	movq	%r14, %rbp
	jmp	.LBB241_277
.LBB241_275:
	movq	56(%rsp), %rax
	movq	%r14, %rbp
	jmp	.LBB241_310
.LBB241_276:
	movq	%rbp, %rax
	movq	%r15, %rbp
.LBB241_277:
	movq	%rax, 792(%rsp)
.Ltmp7117:
	leaq	784(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7118:
	movq	1408(%rsp), %rbx
	movq	1416(%rsp), %r14
.Ltmp7125:
	leaq	496(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp7126:
	movq	$0, 80(%rsp)
	jmp	.LBB241_551
.LBB241_280:
	vmovdqa64	.LCPI241_0(%rip), %zmm1
	vpbroadcastq	.LCPI241_1(%rip), %zmm2
	vpbroadcastq	.LCPI241_2(%rip), %zmm3
	movq	%r12, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
.LBB241_281:
	vpmullq	%zmm2, %zmm1, %zmm7
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm8, %xmm8, %xmm8
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$-32, %rcx
	vpgatherqq	64(%rbx,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm0, %zmm8, %zmm0
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	1344(%rbx,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm4, %zmm8, %zmm4
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	2624(%rbx,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm5, %zmm8, %zmm5
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	3904(%rbx,%zmm7), %zmm8 {%k1}
	vpaddq	%zmm6, %zmm8, %zmm6
	jne	.LBB241_281
	vpaddq	%zmm0, %zmm4, %zmm0
	movq	%r13, %r14
	vpaddq	%zmm0, %zmm5, %zmm0
	vpaddq	%zmm0, %zmm6, %zmm0
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rdx
	cmpq	%rax, %r12
	je	.LBB241_289
	testb	$24, %r12b
	je	.LBB241_287
.LBB241_284:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI241_0(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI241_1(%rip), %zmm2
	vpbroadcastq	.LCPI241_3(%rip), %zmm3
	movq	%r12, %rax
	andq	$-8, %rax
	vmovq	%rdx, %xmm0
	subq	%rax, %rcx
	.p2align	4
.LBB241_285:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	64(%rbx,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB241_285
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rdx
	cmpq	%rax, %r12
	je	.LBB241_289
.LBB241_287:
	movq	%r12, %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	64(%rax,%rbx), %rax
	.p2align	4
.LBB241_288:
	addq	(%rax), %rdx
	addq	$160, %rax
	decq	%rcx
	jne	.LBB241_288
.LBB241_289:
	movq	344(%rsp), %rax
	movzbl	2521(%rsp), %r15d
	movzbl	2523(%rsp), %r13d
	movq	$0, 1168(%rsp)
	movq	$8, 1176(%rsp)
	movq	%rdx, 1704(%rsp)
	movq	$0, 1184(%rsp)
	movq	%rax, 1784(%rsp)
	movl	156(%rsp), %eax
	movq	%rbx, 1792(%rsp)
	movq	%r12, 1800(%rsp)
	movb	%al, 1808(%rsp)
.Ltmp6974:
	leaq	496(%rsp), %rdi
	leaq	1168(%rsp), %rsi
	movq	%r14, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp6975:
	movq	%r12, %rdi
	movq	496(%rsp), %r12
	movq	504(%rsp), %rax
	movq	512(%rsp), %rdx
	movq	520(%rsp), %rcx
	movq	%rbx, 760(%rsp)
	movq	%rdi, 896(%rsp)
	cmpq	$-1, %r12
	je	.LBB241_302
	movq	%rax, 48(%rsp)
	movzbl	528(%rsp), %eax
	vmovdqu	544(%rsp), %ymm0
	vmovdqu	560(%rsp), %ymm1
	movzbl	535(%rsp), %ebp
	movzwl	533(%rsp), %r14d
	movl	529(%rsp), %ebx
	movq	%rdx, 8(%rsp)
	movq	%rcx, 40(%rsp)
	movq	%rax, 80(%rsp)
	movq	536(%rsp), %rax
	vmovdqu	%ymm0, 784(%rsp)
	vmovdqu	%ymm1, 800(%rsp)
	movq	%rax, 56(%rsp)
.Ltmp6979:
	leaq	864(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp6980:
	shll	$16, %ebp
	orl	%ebp, %r14d
	shlq	$32, %r14
	orq	%r14, %rbx
	movq	%rbx, 64(%rsp)
.LBB241_293:
	movq	760(%rsp), %r14
	movq	896(%rsp), %r15
	movq	344(%rsp), %rbp
.LBB241_294:
	testq	%r15, %r15
	je	.LBB241_298
	movl	$1, %r13d
	subq	%r15, %r13
	movq	%r14, %r15
	.p2align	4
.LBB241_296:
.Ltmp7081:
	movq	%r15, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7082:
	incq	%r13
	addq	$160, %r15
	cmpq	$1, %r13
	jne	.LBB241_296
.LBB241_298:
	testq	%rbp, %rbp
	je	.LBB241_300
	shlq	$5, %rbp
	movl	$8, %edx
	movq	%r14, %rdi
	leaq	(%rbp,%rbp,4), %rsi
	callq	__rustc::__rust_dealloc
.LBB241_300:
	cmpq	$-1, %r12
	je	.LBB241_308
	vmovdqu	800(%rsp), %ymm1
	vmovdqu	784(%rsp), %ymm0
	movq	40(%rsp), %rbp
	movq	8(%rsp), %r14
	movq	80(%rsp), %rbx
	movq	56(%rsp), %r13
	movq	64(%rsp), %r15
	vmovdqu	%ymm1, 1424(%rsp)
	vmovdqu	%ymm0, 1408(%rsp)
	jmp	.LBB241_314
.LBB241_302:
	movq	80(%rsp), %rsi
	movq	%rbp, 832(%rsp)
	movq	%rax, 256(%rsp)
	leaq	(%r14,%r14,4), %rax
	movq	%rdx, 264(%rsp)
	movq	%rcx, 272(%rsp)
	leaq	(%rbp,%rax,8), %rax
	movq	%rsi, 848(%rsp)
	movq	16(%rsp), %rsi
	movq	%rbp, 840(%rsp)
	movq	%rax, 856(%rsp)
	movq	616(%rsi), %rax
	movq	%rax, 1392(%rsp)
	testq	%rax, %rax
	je	.LBB241_363
	movq	%r13, 1672(%rsp)
	movb	%r15b, 79(%rsp)
	lock		incq	(%rax)
	jle	.LBB241_715
	movq	16(%rsp), %rax
	movq	%rdi, %r14
	movq	616(%rax), %rax
	movq	%rax, 920(%rsp)
	movq	%rax, 128(%rsp)
	movq	16(%rax), %rcx
	movq	40(%rax), %rax
	movq	%rcx, 360(%rsp)
	movq	%rax, 288(%rsp)
	cmpq	$-1, %rax
	je	.LBB241_395
	testq	%r14, %r14
	je	.LBB241_377
	cmpq	$8, %r14
	jae	.LBB241_381
	xorl	%eax, %eax
	xorl	%esi, %esi
	jmp	.LBB241_392
.LBB241_308:
	movq	40(%rsp), %rbp
	movq	8(%rsp), %r14
	movq	80(%rsp), %rcx
	jmp	.LBB241_548
.LBB241_309:
	movq	%rbp, %rax
	movq	%r15, %rbp
.LBB241_310:
	movq	%rax, 424(%rsp)
.Ltmp6946:
	leaq	416(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp6947:
	movq	784(%rsp), %rax
	movq	792(%rsp), %r14
	movq	$-1, %r12
	xorl	%r15d, %r15d
	xorl	%ebx, %ebx
	movq	%rax, 48(%rsp)
.LBB241_312:
.Ltmp6951:
	leaq	928(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp6952:
	cmpq	$-1, %r12
	je	.LBB241_550
.LBB241_314:
	vmovups	1408(%rsp), %ymm0
	vmovdqu	1424(%rsp), %ymm1
	movq	136(%rsp), %rcx
	movzbl	%bl, %eax
	movq	48(%rsp), %rbx
	shlq	$8, %r15
	vmovq	%r13, %xmm2
	orq	%r15, %rax
	vmovdqu	%ymm1, 80(%rcx)
	vmovups	%ymm0, 64(%rcx)
	vmovq	%rax, %xmm0
	vpunpcklqdq	%xmm2, %xmm0, %xmm0
.LBB241_315:
	movq	%r12, 16(%rcx)
	movq	%rbx, 24(%rcx)
	movq	%r14, 32(%rcx)
	movq	%rbp, 40(%rcx)
	vmovdqa	%xmm0, 48(%rcx)
	movq	$1, (%rcx)
.Ltmp7108:
	leaq	1728(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7109:
	movq	352(%rsp), %rax
	testq	%rax, %rax
	je	.LBB241_319
.LBB241_317:
	lock		decq	(%rax)
	jne	.LBB241_319
	#MEMBARRIER
.Ltmp7187:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	352(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp7188:
.LBB241_319:
	movb	$1, %al
	movl	%eax, 8(%rsp)
.LBB241_320:
.Ltmp7192:
	leaq	2112(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp7193:
	movq	120(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB241_323
	#MEMBARRIER
.Ltmp7197:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	120(%rsp), %rdi
	callq	*%rax
.Ltmp7198:
.LBB241_323:
.Ltmp7200:
	movl	8(%rsp), %r15d
	leaq	2328(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7201:
	movq	2024(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB241_334
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	2032(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB241_327
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB241_327:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB241_333
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB241_327
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB241_330:
	cmpq	%rax, %rdx
	jge	.LBB241_332
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB241_330
.LBB241_332:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB241_333:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB241_334:
	movq	1952(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB241_344
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	1960(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB241_337
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB241_337:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB241_343
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB241_337
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB241_340:
	cmpq	%rax, %rdx
	jge	.LBB241_342
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB241_340
.LBB241_342:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB241_343:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB241_344:
	movq	2048(%rsp), %rax
	testq	%rax, %rax
	je	.LBB241_347
	lock		decq	(%rax)
	jne	.LBB241_347
	leaq	2048(%rsp), %rdi
	#MEMBARRIER
.Ltmp7203:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp7204:
.LBB241_347:
	movq	408(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB241_349
	#MEMBARRIER
.Ltmp7206:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	408(%rsp), %rdi
	callq	*%rax
.Ltmp7207:
.LBB241_349:
	testb	%r15b, %r15b
	je	.LBB241_626
	movq	392(%rsp), %r14
	movq	400(%rsp), %rbx
	testq	%rbx, %rbx
	je	.LBB241_615
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r12
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	xorl	%r15d, %r15d
	jmp	.LBB241_355
	.p2align	4
.LBB241_352:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB241_353:
	callq	*%rbp
.LBB241_354:
	incq	%r15
	cmpq	%rbx, %r15
	je	.LBB241_615
.LBB241_355:
	leaq	(%r15,%r15,4), %rcx
	movq	(%r14,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB241_354
	leaq	(%r14,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB241_358
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB241_358:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB241_353
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB241_358
	movq	%rcx, %rdx
	negq	%rdx
	xorl	%eax, %eax
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%r12)
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB241_361:
	cmpq	%rax, %rdx
	jge	.LBB241_352
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB241_361
	jmp	.LBB241_352
.LBB241_363:
	movq	%rdx, 8(%rsp)
	movq	840(%rsp), %rdx
	movq	832(%rsp), %rax
	movq	848(%rsp), %rsi
	movq	%rdx, 168(%rsp)
	movq	856(%rsp), %rdx
	movq	%rax, 160(%rsp)
	movq	%rsi, 176(%rsp)
	movq	%rdx, 184(%rsp)
	movq	184(%rsp), %rax
	movq	168(%rsp), %r13
	movq	%rax, 368(%rsp)
	cmpq	%rax, %r13
	je	.LBB241_378
	leaq	(,%rcx,8), %rax
	movq	%rcx, %r14
	leaq	(%rax,%rax,4), %rbx
	jmp	.LBB241_367
.LBB241_365:
	movq	264(%rsp), %rax
	movq	%rax, 8(%rsp)
.LBB241_366:
	movq	8(%rsp), %rcx
	movq	80(%rsp), %rax
	shll	$16, %ebp
	movq	64(%rsp), %rdx
	addq	$40, %r13
	orl	%ebp, %r15d
	shlq	$32, %r15
	orq	%r15, %r14
	movq	%rax, (%rcx,%rbx)
	movq	56(%rsp), %rax
	movq	%rax, 8(%rcx,%rbx)
	movzbl	376(%rsp), %eax
	movq	%r12, 16(%rcx,%rbx)
	movb	%al, 24(%rcx,%rbx)
	movq	%r14, %rax
	movl	%r14d, 25(%rcx,%rbx)
	shrq	$32, %r14
	shrq	$48, %rax
	movw	%r14w, 29(%rcx,%rbx)
	movq	40(%rsp), %r14
	movb	%al, 31(%rcx,%rbx)
	movq	%rdx, 32(%rcx,%rbx)
	addq	$40, %rbx
	incq	%r14
	movq	%r14, 272(%rsp)
	cmpq	368(%rsp), %r13
	je	.LBB241_379
.LBB241_367:
	movq	32(%r13), %rax
	leaq	1176(%rsp), %rcx
	movq	%rax, 32(%rcx)
	movq	16(%rsp), %rax
	vmovdqu	(%r13), %ymm0
	vmovdqu	%ymm0, (%rcx)
	movq	%rax, 1168(%rsp)
	cmpq	$0, 1176(%rsp)
	je	.LBB241_369
	movq	32(%r13), %rax
	leaq	504(%rsp), %rcx
	movq	%rax, 32(%rcx)
	vmovdqu	(%r13), %ymm0
	vmovdqu	%ymm0, (%rcx)
	jmp	.LBB241_371
.LBB241_369:
	movq	664(%rax), %rdx
.Ltmp7060:
	movq	48(%rsp), %rsi
	leaq	496(%rsp), %rdi
	leaq	1184(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7061:
	movq	496(%rsp), %r12
	cmpq	$-1, %r12
	jne	.LBB241_383
.LBB241_371:
	movq	512(%rsp), %rcx
	movq	%r14, %rdx
	movq	504(%rsp), %rax
	movzbl	528(%rsp), %esi
	movq	520(%rsp), %r12
	movzbl	535(%rsp), %ebp
	movzwl	533(%rsp), %r15d
	movl	529(%rsp), %r14d
	movq	%rdx, 40(%rsp)
	movq	%rcx, 56(%rsp)
	movq	536(%rsp), %rcx
	movq	%rax, 80(%rsp)
	movb	%sil, 376(%rsp)
	movq	%rcx, 64(%rsp)
	cmpq	256(%rsp), %rdx
	jne	.LBB241_366
.Ltmp7068:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	256(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp7069:
	jmp	.LBB241_365
.LBB241_373:
	vmovdqa	960(%rsp), %xmm0
	vmovdqu	976(%rsp), %ymm2
	vmovdqu	992(%rsp), %ymm1
	movq	40(%rsp), %rax
	movq	936(%rsp), %rbx
	movq	944(%rsp), %r14
	movq	952(%rsp), %rbp
	movq	%rax, 792(%rsp)
	vmovdqu	%ymm2, 160(%rsp)
	vmovdqa	%xmm0, 16(%rsp)
	vmovdqu	%ymm1, 176(%rsp)
.Ltmp7104:
	leaq	784(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7105:
	leaq	1408(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB241_139
.LBB241_375:
	vmovdqu	544(%rsp), %ymm0
	vmovdqu	560(%rsp), %ymm1
	movq	40(%rsp), %rax
	movq	504(%rsp), %r15
	movq	512(%rsp), %r14
	movq	520(%rsp), %rbp
	movq	528(%rsp), %rbx
	movq	536(%rsp), %r13
	movq	%rax, 424(%rsp)
	vmovdqu	%ymm0, 160(%rsp)
	vmovdqu	%ymm1, 176(%rsp)
.Ltmp6938:
	leaq	416(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp6939:
	leaq	784(%rsp), %rdi
	movq	%r15, 48(%rsp)
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB241_144
.LBB241_377:
	xorl	%esi, %esi
	jmp	.LBB241_394
.LBB241_378:
	movq	%rcx, %r14
.LBB241_379:
	movb	$1, %r15b
	movq	%r14, 40(%rsp)
	movq	%r13, 168(%rsp)
.Ltmp7073:
	leaq	160(%rsp), %rdi
	movb	$1, %r13b
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7074:
	movq	256(%rsp), %rax
	movq	264(%rsp), %rcx
	movq	$-1, %r12
	movq	%rax, 48(%rsp)
	movb	$2, %al
	movq	%rcx, 8(%rsp)
	movq	%rax, 80(%rsp)
	jmp	.LBB241_293
.LBB241_381:
	cmpq	$32, %r14
	jae	.LBB241_385
	xorl	%eax, %eax
	xorl	%esi, %esi
	jmp	.LBB241_389
.LBB241_383:
	vmovups	544(%rsp), %ymm0
	movq	504(%rsp), %rax
	movq	512(%rsp), %rcx
	movq	520(%rsp), %rdx
	movzbl	535(%rsp), %ebp
	movzwl	533(%rsp), %r14d
	addq	$40, %r13
	movb	$1, %r15b
	movq	%r13, 168(%rsp)
	movq	%rax, 48(%rsp)
	movzbl	528(%rsp), %eax
	movq	%rcx, 8(%rsp)
	movq	536(%rsp), %rcx
	movq	%rdx, 40(%rsp)
	vmovups	%ymm0, 784(%rsp)
	vmovdqu	560(%rsp), %ymm0
	movq	%rax, 80(%rsp)
	movl	529(%rsp), %eax
	movq	%rcx, 56(%rsp)
	movq	%rax, 64(%rsp)
	vmovdqu	%ymm0, 800(%rsp)
.Ltmp7063:
	leaq	160(%rsp), %rdi
	movb	$1, %r13b
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7064:
	shll	$16, %ebp
	movb	$1, %r15b
	movb	$1, %r13b
	orl	%ebp, %r14d
	shlq	$32, %r14
	addq	%r14, 64(%rsp)
	jmp	.LBB241_700
.LBB241_385:
	vmovdqa64	.LCPI241_0(%rip), %zmm1
	vpbroadcastq	.LCPI241_1(%rip), %zmm2
	vpbroadcastq	.LCPI241_2(%rip), %zmm3
	movq	%r14, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
.LBB241_386:
	vpmullq	%zmm2, %zmm1, %zmm7
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm8, %xmm8, %xmm8
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$-32, %rcx
	vpgatherqq	16(%rbx,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm0, %zmm8, %zmm0
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	1296(%rbx,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm4, %zmm8, %zmm4
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	2576(%rbx,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm5, %zmm8, %zmm5
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	3856(%rbx,%zmm7), %zmm8 {%k1}
	vpaddq	%zmm6, %zmm8, %zmm6
	jne	.LBB241_386
	vpaddq	%zmm0, %zmm4, %zmm0
	vpaddq	%zmm0, %zmm5, %zmm0
	vpaddq	%zmm0, %zmm6, %zmm0
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rsi
	cmpq	%rax, %r14
	je	.LBB241_394
	testb	$24, %r14b
	je	.LBB241_392
.LBB241_389:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI241_0(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI241_1(%rip), %zmm2
	vpbroadcastq	.LCPI241_3(%rip), %zmm3
	movq	%r14, %rax
	andq	$-8, %rax
	vmovq	%rsi, %xmm0
	subq	%rax, %rcx
.LBB241_390:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	16(%rbx,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB241_390
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rsi
	cmpq	%rax, %r14
	je	.LBB241_394
.LBB241_392:
	movq	%r14, %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	16(%rax,%rbx), %rax
.LBB241_393:
	addq	(%rax), %rsi
	addq	$160, %rax
	decq	%rcx
	jne	.LBB241_393
.LBB241_394:
	movb	$1, %r15b
.Ltmp6982:
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts@GOTPCREL(%rip), %rax
	movq	48(%rsp), %rdi
	movb	$1, %r13b
	vzeroupper
	callq	*%rax
.Ltmp6983:
.LBB241_395:
	movq	128(%rsp), %rax
	movq	344(%rsp), %rdx
	leaq	(%r14,%r14,4), %rcx
	movq	%rbx, 1360(%rsp)
	movq	%rbx, 1368(%rsp)
	shlq	$5, %rcx
	addq	%rbx, %rcx
	movq	%rcx, 1680(%rsp)
	addq	$16, %rax
	movq	%rdx, 1376(%rsp)
	movq	%rcx, 1384(%rsp)
	movq	%rax, 280(%rsp)
	testq	%r14, %r14
	je	.LBB241_541
	leaq	504(%rsp), %r14
.LBB241_397:
	leaq	160(%rbx), %rdx
	movq	%rdx, 1368(%rsp)
	vmovups	96(%rbx), %zmm0
	movq	(%rbx), %rax
	vmovups	%zmm0, 1256(%rsp)
	vmovups	72(%rbx), %zmm0
	vmovups	%zmm0, 1232(%rsp)
	vmovups	8(%rbx), %zmm0
	vmovups	%zmm0, 1168(%rsp)
	cmpq	$-1, %rax
	je	.LBB241_541
	vmovdqu64	1168(%rsp), %zmm0
	vmovdqu64	1232(%rsp), %zmm1
	vmovdqu64	1256(%rsp), %zmm2
	movq	%rax, 496(%rsp)
	movq	%rdx, 1688(%rsp)
	vmovdqu64	%zmm2, 88(%r14)
	vmovdqu64	%zmm1, 64(%r14)
	vmovdqu64	%zmm0, (%r14)
	movq	504(%rsp), %rdx
	movq	536(%rsp), %rdi
	imulq	$88, 512(%rsp), %rsi
	movq	520(%rsp), %rbx
	movq	528(%rsp), %rcx
	movq	544(%rsp), %r13
	movq	%rdi, 888(%rsp)
	movq	%rdx, 304(%rsp)
	movq	%rax, 320(%rsp)
	movq	552(%rsp), %rdi
	movq	560(%rsp), %rax
	movq	%rcx, 144(%rsp)
	movq	%rdx, 8(%rsp)
	movq	%rdx, 312(%rsp)
	movq	%rbx, 768(%rsp)
	addq	%rdx, %rsi
	movq	%rsi, 112(%rsp)
	movq	%rsi, 328(%rsp)
	testq	%rax, %rax
	je	.LBB241_529
	movq	592(%rsp), %rcx
	shlq	$5, %rax
	leaq	160(%rsp), %r14
	xorl	%r15d, %r15d
	movq	%r13, 104(%rsp)
	movq	%rdi, 488(%rsp)
	addq	%rdi, %rax
	movq	%rax, 1696(%rsp)
	movq	%rdi, %rax
	movq	%rcx, 40(%rsp)
	jmp	.LBB241_402
.LBB241_400:
	movq	488(%rsp), %rdi
	movq	904(%rsp), %rax
	movq	296(%rsp), %r15
.LBB241_401:
	addq	$32, %rax
	movq	%rbp, 840(%rsp)
	cmpq	1696(%rsp), %rax
	je	.LBB241_529
.LBB241_402:
	movq	16(%rax), %rcx
	movq	(%rax), %r12
	movq	8(%rax), %r13
	movq	%rax, 904(%rsp)
	movq	24(%rax), %rax
	movq	%rcx, 64(%rsp)
	movq	%rax, 80(%rsp)
	testq	%r12, %r12
	je	.LBB241_409
	cmpq	$-1, 360(%rsp)
	je	.LBB241_409
	movq	%r12, 416(%rsp)
	movq	$0, 424(%rsp)
.Ltmp6984:
	movq	280(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	leaq	416(%rsp), %rdx
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp6985:
	cmpb	$-1, 176(%rsp)
	jne	.LBB241_705
	movq	16(%rsp), %rax
	movq	632(%rax), %rax
	testq	%rax, %rax
	je	.LBB241_409
	movq	16(%rsp), %rcx
	movl	1228(%rcx), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB241_409
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	movq	1672(%rsp), %rax
	lock		addq	%r12, (%rcx,%rax,8)
.LBB241_409:
.Ltmp6986:
	movq	888(%rsp), %rdx
	movq	%r13, %rdi
	movq	%r15, %rsi
	vzeroupper
	callq	<usize as core::cmp::Ord>::clamp
	movq	%rax, 296(%rsp)
.Ltmp6987:
	movq	296(%rsp), %rsi
	cmpq	%r15, %rsi
	jb	.LBB241_708
	cmpq	888(%rsp), %rsi
	ja	.LBB241_708
	movq	144(%rsp), %rcx
	leaq	(%r15,%r15,2), %rax
	leaq	416(%rsp), %rdi
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqa	%xmm0, 160(%rsp)
	movq	$0, 176(%rsp)
	leaq	(%rcx,%rax,8), %r13
	leaq	(%rsi,%rsi,2), %rax
	leaq	(%rcx,%rax,8), %rdx
	movq	%r13, %rsi
	movq	%r14, %rcx
	movq	%rdx, 56(%rsp)
	callq	<core::slice::iter::Iter<purrdf_sparql_eval::row_checkpoint::Deferred> as core::iter::traits::iterator::Iterator>::fold::<(u64, u64, u64), purrdf_sparql_eval::row_checkpoint::commit_items<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>::{closure#2}>
	movq	424(%rsp), %rax
	movq	432(%rsp), %rcx
	movq	416(%rsp), %r12
	cmpq	$-1, 288(%rsp)
	movq	%rax, 376(%rsp)
	movq	%rcx, 368(%rsp)
	je	.LBB241_414
	movq	64(%rsp), %rax
	movq	8(%rsp), %rsi
	movq	112(%rsp), %rdx
	movl	$0, %ecx
	subq	40(%rsp), %rax
	movq	%rsi, 160(%rsp)
	movq	%rdx, 168(%rsp)
	cmovaeq	%rax, %rcx
	movq	%rcx, 176(%rsp)
.Ltmp6988:
	movq	%r14, %rdi
	callq	<core::iter::adapters::take::Take<core::slice::iter::Iter<(u64, purrdf_core::ir::term::TermValue)>> as core::iter::adapters::take::SpecTake>::spec_fold::<u64, purrdf_sparql_eval::row_checkpoint::commit_items<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>::{closure#3}>
.Ltmp6989:
	jmp	.LBB241_415
.LBB241_414:
	xorl	%eax, %eax
.LBB241_415:
	movq	128(%rsp), %rcx
	movl	296(%rcx), %ecx
	testl	%ecx, %ecx
	je	.LBB241_429
.LBB241_416:
	cmpq	$-1, 360(%rsp)
	je	.LBB241_418
	movq	128(%rsp), %rdx
	movq	$-1, %rsi
	movq	80(%rdx), %rcx
	addq	%r12, %rcx
	cmovbq	%rsi, %rcx
	cmpq	16(%rdx), %rcx
	ja	.LBB241_430
.LBB241_418:
	cmpq	$-1, 288(%rsp)
	je	.LBB241_420
	movq	16(%rsp), %rdx
	movq	$-1, %rsi
	movq	1048(%rdx), %rcx
	movq	1056(%rdx), %rdx
	subq	%rdx, %rcx
	movl	$0, %edx
	cmovaeq	%rcx, %rdx
	movq	128(%rsp), %rcx
	addq	%rax, %rdx
	cmovbq	%rsi, %rdx
	addq	376(%rsp), %rdx
	cmovbq	%rsi, %rdx
	addq	368(%rsp), %rdx
	movq	104(%rcx), %rax
	cmovbq	%rsi, %rdx
	addq	%rdx, %rax
	cmovbq	%rsi, %rax
	cmpq	40(%rcx), %rax
	ja	.LBB241_430
.LBB241_420:
	cmpq	$-1, 360(%rsp)
	je	.LBB241_470
	cmpq	296(%rsp), %r15
	je	.LBB241_470
	movq	296(%rsp), %rcx
	leaq	(,%r15,8), %rax
	leaq	(%rax,%rax,2), %rax
	leaq	(,%rcx,8), %rcx
	leaq	(%rcx,%rcx,2), %rcx
	jmp	.LBB241_424
.LBB241_423:
	addq	$24, %rax
	cmpq	%rax, %rcx
	je	.LBB241_470
.LBB241_424:
	movq	144(%rsp), %rdx
	cmpb	$0, (%rdx,%rax)
	jne	.LBB241_423
	movq	144(%rsp), %rdx
	movzbl	1(%rdx,%rax), %edx
	cmpq	$255, %rdx
	je	.LBB241_423
	movq	16(%rsp), %rsi
	movq	632(%rsi), %rsi
	testq	%rsi, %rsi
	je	.LBB241_423
	movq	16(%rsp), %rdi
	movl	1228(%rdi), %edi
	cmpq	%rdi, 56(%rsi)
	jbe	.LBB241_423
	movq	144(%rsp), %r8
	movq	%rdi, %r9
	shlq	$7, %r9
	leaq	(%r9,%rdi,8), %rdi
	addq	48(%rsi), %rdi
	movq	8(%r8,%rax), %r8
	lock		addq	%r8, (%rdi,%rdx,8)
	jmp	.LBB241_423
.LBB241_429:
	movq	128(%rsp), %rcx
	cmpb	$-1, 272(%rcx)
	je	.LBB241_416
.LBB241_430:
	cmpq	296(%rsp), %r15
	jne	.LBB241_432
.LBB241_462:
	cmpq	$-1, 288(%rsp)
	je	.LBB241_474
	movq	64(%rsp), %rax
	movq	104(%rsp), %r13
	cmpq	%rax, 40(%rsp)
	jae	.LBB241_519
	movq	8(%rsp), %rdx
	cmpq	112(%rsp), %rdx
	je	.LBB241_483
	movq	64(%rsp), %rax
	addq	$88, %rdx
	leaq	-1(%rax), %r15
	movq	%rdx, %rax
.LBB241_466:
	movq	%rax, %rdx
	movq	-8(%rdx), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rdx, 8(%rsp)
	movq	%rcx, 480(%rsp)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, 416(%rsp)
	cmpq	$-1, %rax
	je	.LBB241_482
	vmovdqu64	416(%rsp), %zmm0
	movq	%rax, 160(%rsp)
	movq	480(%rsp), %rax
	leaq	168(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7023:
	movq	48(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%r14, %rdx
	vzeroupper
	callq	*%rax
.Ltmp7024:
	movq	40(%rsp), %rax
	cmpq	%rax, %r15
	je	.LBB241_484
	movq	8(%rsp), %rdx
	incq	%rax
	movq	%rax, 40(%rsp)
	leaq	-88(%rdx), %rcx
	leaq	88(%rdx), %rax
	addq	$88, %rcx
	cmpq	112(%rsp), %rcx
	jne	.LBB241_466
	jmp	.LBB241_483
.LBB241_431:
	cmpb	$-1, 160(%rsp)
	leaq	160(%rsp), %r14
	jne	.LBB241_684
.LBB241_461:
	addq	$24, %r13
	cmpq	56(%rsp), %r13
	je	.LBB241_462
.LBB241_432:
	movzbl	(%r13), %eax
	leaq	.LJTI241_0(%rip), %rcx
	movslq	(%rcx,%rax,4), %rax
	addq	%rcx, %rax
	jmpq	*%rax
.LBB241_433:
	cmpq	$-1, 360(%rsp)
	je	.LBB241_461
	movq	%r14, %rdi
	movzbl	1(%r13), %r15d
	movq	8(%r13), %r12
	movq	16(%r13), %r14
	movq	%r12, 416(%rsp)
	movq	$0, 424(%rsp)
.Ltmp7017:
	movq	280(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	leaq	416(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp7018:
	movzbl	176(%rsp), %ecx
	cmpb	$-1, %cl
	setne	%al
	testq	%r14, %r14
	setne	%dl
	testb	%al, %dl
	jne	.LBB241_668
	cmpb	$-1, %cl
	setne	%cl
	cmpl	$255, %r15d
	sete	%dl
	orb	%cl, %dl
	jne	.LBB241_453
	movq	16(%rsp), %rcx
	movq	632(%rcx), %rax
	testq	%rax, %rax
	je	.LBB241_454
	movl	1228(%rcx), %ecx
	leaq	160(%rsp), %r14
	cmpq	%rcx, 56(%rax)
	jbe	.LBB241_461
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	lock		addq	%r12, (%rcx,%r15,8)
	jmp	.LBB241_461
.LBB241_440:
	movq	8(%r13), %r15
	cmpq	%r15, 40(%rsp)
	jae	.LBB241_458
	movq	8(%rsp), %rdx
	cmpq	112(%rsp), %rdx
	je	.LBB241_456
	addq	$88, %rdx
	leaq	-1(%r15), %r12
	movq	%rdx, %rax
.LBB241_443:
	movq	%rax, %rdx
	movq	-8(%rdx), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rdx, 8(%rsp)
	movq	%rcx, 480(%rsp)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, 416(%rsp)
	cmpq	$-1, %rax
	je	.LBB241_455
	vmovdqu64	416(%rsp), %zmm0
	movq	%rax, 160(%rsp)
	movq	480(%rsp), %rax
	leaq	168(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7007:
	movq	48(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%r14, %rdx
	vzeroupper
	callq	*%rax
.Ltmp7008:
	movq	40(%rsp), %rsi
	cmpq	%rsi, %r12
	je	.LBB241_457
	movq	8(%rsp), %rdx
	incq	%rsi
	movq	%rsi, 40(%rsp)
	leaq	-88(%rdx), %rcx
	leaq	88(%rdx), %rax
	addq	$88, %rcx
	cmpq	112(%rsp), %rcx
	jne	.LBB241_443
	jmp	.LBB241_456
.LBB241_447:
	cmpq	$-1, 288(%rsp)
	je	.LBB241_461
	movq	8(%r13), %rcx
.Ltmp7005:
	movq	280(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::admit_transient@GOTPCREL(%rip), %rax
	movl	$3, %edx
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp7006:
	jmp	.LBB241_431
.LBB241_449:
	cmpq	$-1, 288(%rsp)
	je	.LBB241_461
	movq	128(%rsp), %rax
	cmpq	$-1, 40(%rax)
	je	.LBB241_461
.Ltmp7012:
	movq	8(%r13), %rcx
	movq	280(%rsp), %rsi
	movq	%r14, %rdi
	movq	16(%r13), %r14
	movl	$3, %edx
	xorl	%r8d, %r8d
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.18159039729619107857)
.Ltmp7013:
	cmpb	$-1, 160(%rsp)
	setne	%al
	sete	%cl
	testq	%r14, %r14
	sete	%dl
	orb	%cl, %dl
	je	.LBB241_675
.LBB241_453:
	testb	%al, %al
	leaq	160(%rsp), %r14
	je	.LBB241_461
	jmp	.LBB241_684
.LBB241_454:
	leaq	160(%rsp), %r14
	jmp	.LBB241_461
.LBB241_455:
	movq	8(%rsp), %rdx
.LBB241_456:
	movq	%rdx, 8(%rsp)
	movq	%rdx, 312(%rsp)
	jmp	.LBB241_459
.LBB241_457:
	movq	%r15, 40(%rsp)
.LBB241_458:
	movq	8(%rsp), %rax
	movq	%rax, 312(%rsp)
.LBB241_459:
	cmpq	$-1, 288(%rsp)
	je	.LBB241_461
.Ltmp7010:
	movq	16(%rsp), %rsi
	movq	%r14, %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp7011:
	jmp	.LBB241_431
.LBB241_470:
	movq	296(%rsp), %rax
	movb	$1, %r13b
	cmpq	%rax, %r15
	je	.LBB241_489
	shlq	$3, %rax
	shlq	$3, %r15
	leaq	(%rax,%rax,2), %rax
	leaq	(%r15,%r15,2), %rcx
.LBB241_472:
	movq	144(%rsp), %rdx
	cmpb	$2, -24(%rdx,%rax)
	je	.LBB241_475
	addq	$-24, %rax
	cmpq	%rax, %rcx
	jne	.LBB241_472
	jmp	.LBB241_489
.LBB241_475:
	movq	144(%rsp), %rcx
	movq	-16(%rcx,%rax), %r15
	cmpq	%r15, 40(%rsp)
	jae	.LBB241_485
	movq	8(%rsp), %rax
	cmpq	112(%rsp), %rax
	je	.LBB241_486
	addq	$88, %rax
	leaq	-1(%r15), %r13
.LBB241_478:
	movq	%rax, %rdx
	movq	-8(%rdx), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rdx, %r14
	movq	%rcx, 480(%rsp)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, 416(%rsp)
	cmpq	$-1, %rax
	je	.LBB241_488
	vmovdqu64	416(%rsp), %zmm0
	movq	%rax, 160(%rsp)
	movq	480(%rsp), %rax
	leaq	168(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp6990:
	movq	48(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp6991:
	movq	40(%rsp), %rdx
	cmpq	%rdx, %r13
	je	.LBB241_487
	leaq	-88(%r14), %rcx
	incq	%rdx
	leaq	88(%r14), %rax
	addq	$88, %rcx
	movq	%rdx, 40(%rsp)
	cmpq	112(%rsp), %rcx
	jne	.LBB241_478
	jmp	.LBB241_488
.LBB241_482:
	movq	8(%rsp), %rdx
.LBB241_483:
	movq	%rdx, 8(%rsp)
	movq	%rdx, 312(%rsp)
	jmp	.LBB241_519
.LBB241_484:
	movq	8(%rsp), %rax
	movq	64(%rsp), %rcx
	movq	%rax, 312(%rsp)
	movq	%rcx, 40(%rsp)
	jmp	.LBB241_519
.LBB241_485:
	movq	8(%rsp), %r14
	jmp	.LBB241_488
.LBB241_486:
	movq	%rax, %r14
	jmp	.LBB241_488
.LBB241_487:
	movq	%r15, 40(%rsp)
.LBB241_488:
	movq	%r14, 8(%rsp)
	movq	%r14, 312(%rsp)
	leaq	160(%rsp), %r14
	xorl	%r13d, %r13d
.LBB241_489:
	cmpq	$-1, 360(%rsp)
	je	.LBB241_494
	testq	%r12, %r12
	je	.LBB241_494
	movq	%r12, 416(%rsp)
	movq	$0, 424(%rsp)
.Ltmp6993:
	movq	280(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	leaq	416(%rsp), %rdx
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp6994:
	cmpb	$-1, 176(%rsp)
	je	.LBB241_494
	cmpq	$-1, 288(%rsp)
	movq	104(%rsp), %r13
	movb	$1, %r15b
	jne	.LBB241_507
	jmp	.LBB241_706
.LBB241_494:
	cmpq	$-1, 288(%rsp)
	je	.LBB241_474
	movq	128(%rsp), %rax
	cmpq	$-1, 40(%rax)
	je	.LBB241_499
.Ltmp6995:
	movq	280(%rsp), %rsi
	movq	376(%rsp), %rcx
	movl	$3, %edx
	movq	%r14, %rdi
	xorl	%r8d, %r8d
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.18159039729619107857)
.Ltmp6996:
	cmpb	$-1, 160(%rsp)
	je	.LBB241_499
	movq	104(%rsp), %r13
	movb	$1, %r15b
	jmp	.LBB241_507
.LBB241_474:
	movq	104(%rsp), %r13
	jmp	.LBB241_519
.LBB241_499:
	testb	%r13b, %r13b
	movq	104(%rsp), %r13
	jne	.LBB241_502
.Ltmp6997:
	movq	16(%rsp), %rsi
	movq	%r14, %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp6998:
	cmpb	$-1, 160(%rsp)
	movq	104(%rsp), %r13
	movb	$1, %r15b
	jne	.LBB241_507
.LBB241_502:
	movq	368(%rsp), %rcx
	testq	%rcx, %rcx
	je	.LBB241_506
.Ltmp6999:
	movq	280(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::admit_transient@GOTPCREL(%rip), %rax
	movl	$3, %edx
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp7000:
	cmpb	$-1, 160(%rsp)
	movq	104(%rsp), %r13
	setne	%r15b
	jmp	.LBB241_507
.LBB241_506:
	xorl	%r15d, %r15d
.LBB241_507:
	movq	64(%rsp), %rax
	cmpq	%rax, 40(%rsp)
	jae	.LBB241_517
	movq	8(%rsp), %rdx
	cmpq	112(%rsp), %rdx
	je	.LBB241_515
	movq	64(%rsp), %rax
	addq	$88, %rdx
	leaq	-1(%rax), %r12
	movq	%rdx, %rax
.LBB241_510:
	movq	%rax, %rdx
	movq	-8(%rdx), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rdx, 8(%rsp)
	movq	%rcx, 480(%rsp)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, 416(%rsp)
	cmpq	$-1, %rax
	je	.LBB241_514
	vmovdqu64	416(%rsp), %zmm0
	movq	%rax, 160(%rsp)
	movq	480(%rsp), %rax
	leaq	168(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7002:
	movq	48(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%r14, %rdx
	vzeroupper
	callq	*%rax
.Ltmp7003:
	movq	40(%rsp), %rsi
	cmpq	%rsi, %r12
	je	.LBB241_516
	movq	8(%rsp), %rdx
	incq	%rsi
	movq	%rsi, 40(%rsp)
	leaq	-88(%rdx), %rcx
	leaq	88(%rdx), %rax
	addq	$88, %rcx
	cmpq	112(%rsp), %rcx
	jne	.LBB241_510
	jmp	.LBB241_515
.LBB241_514:
	movq	8(%rsp), %rdx
.LBB241_515:
	movq	%rdx, 8(%rsp)
	movq	%rdx, 312(%rsp)
	jmp	.LBB241_518
.LBB241_516:
	movq	64(%rsp), %rax
	movq	%rax, 40(%rsp)
.LBB241_517:
	movq	8(%rsp), %rax
	movq	%rax, 312(%rsp)
.LBB241_518:
	testb	%r15b, %r15b
	jne	.LBB241_706
.LBB241_519:
	cmpq	$0, 80(%rsp)
	je	.LBB241_400
	movq	856(%rsp), %rax
	movq	488(%rsp), %rdi
	movq	296(%rsp), %r15
	movq	%rax, 1712(%rsp)
	movq	904(%rsp), %rax
	jmp	.LBB241_522
.LBB241_521:
	movq	264(%rsp), %rax
	movq	56(%rsp), %rdx
	leaq	(%rbp,%rbp,4), %rcx
	shll	$16, %r13d
	movq	376(%rsp), %rdi
	movq	80(%rsp), %rsi
	addq	$40, %rbx
	incq	%rbp
	orl	%r13d, %r12d
	movq	104(%rsp), %r13
	shlq	$32, %r12
	orq	%r12, %r14
	movq	%rdx, (%rax,%rcx,8)
	movq	64(%rsp), %rdx
	decq	%rsi
	movq	%rsi, 80(%rsp)
	movq	%rdx, 8(%rax,%rcx,8)
	movq	368(%rsp), %rdx
	movq	%rdx, 16(%rax,%rcx,8)
	movb	%r15b, 24(%rax,%rcx,8)
	movq	%r14, %rdx
	shrq	$48, %rdx
	movl	%r14d, 25(%rax,%rcx,8)
	shrq	$32, %r14
	movq	296(%rsp), %r15
	movw	%r14w, 29(%rax,%rcx,8)
	movb	%dl, 31(%rax,%rcx,8)
	movq	%rdi, 32(%rax,%rcx,8)
	movq	%rbp, 272(%rsp)
	movq	%rbx, %rbp
	movq	768(%rsp), %rbx
	movq	488(%rsp), %rdi
	movq	904(%rsp), %rax
	leaq	160(%rsp), %r14
	testq	%rsi, %rsi
	je	.LBB241_401
.LBB241_522:
	cmpq	1712(%rsp), %rbp
	je	.LBB241_401
	movq	32(%rbp), %rax
	leaq	424(%rsp), %rcx
	movq	%rax, 32(%rcx)
	movq	16(%rsp), %rax
	vmovdqu	(%rbp), %ymm0
	vmovdqu	%ymm0, (%rcx)
	movq	%rax, 416(%rsp)
	cmpq	$0, 424(%rsp)
	je	.LBB241_525
	movq	32(%rbp), %rax
	leaq	168(%rsp), %rcx
	movq	%rbp, %rbx
	movq	%rax, 32(%rcx)
	vmovdqu	(%rbp), %ymm0
	vmovdqu	%ymm0, (%rcx)
	jmp	.LBB241_527
.LBB241_525:
	movq	664(%rax), %rdx
	movq	%rbp, %rbx
.Ltmp7026:
	movq	48(%rsp), %rsi
	leaq	432(%rsp), %rcx
	movq	%r14, %rdi
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7027:
	movq	160(%rsp), %r12
	cmpq	$-1, %r12
	jne	.LBB241_667
.LBB241_527:
	movq	176(%rsp), %rcx
	movq	168(%rsp), %rax
	movq	184(%rsp), %rdx
	movzbl	192(%rsp), %r15d
	movzbl	199(%rsp), %r13d
	movzwl	197(%rsp), %r12d
	movl	193(%rsp), %r14d
	movq	272(%rsp), %rbp
	movq	%rcx, 64(%rsp)
	movq	200(%rsp), %rcx
	movq	%rax, 56(%rsp)
	movq	%rdx, 368(%rsp)
	movq	%rcx, 376(%rsp)
	cmpq	256(%rsp), %rbp
	jne	.LBB241_521
.Ltmp7036:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	256(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp7037:
	jmp	.LBB241_521
.LBB241_529:
	testq	%r13, %r13
	je	.LBB241_531
	shlq	$5, %r13
	movl	$8, %edx
	movq	%r13, %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB241_531:
.Ltmp7044:
	leaq	304(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp7045:
	leaq	504(%rsp), %r14
	testq	%rbx, %rbx
	je	.LBB241_534
	movq	144(%rsp), %rdi
	shlq	$3, %rbx
	movl	$8, %edx
	leaq	(%rbx,%rbx,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB241_534:
	movq	584(%rsp), %rax
	testq	%rax, %rax
	je	.LBB241_537
	lock		decq	(%rax)
	jne	.LBB241_537
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	584(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB241_537:
	movq	616(%rsp), %rax
	movq	1688(%rsp), %rbx
	testq	%rax, %rax
	je	.LBB241_540
	lock		decq	(%rax)
	jne	.LBB241_540
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	616(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB241_540:
	cmpq	1680(%rsp), %rbx
	jne	.LBB241_397
.LBB241_541:
	movb	$1, %r15b
	xorl	%r13d, %r13d
.Ltmp7049:
	leaq	1360(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp7050:
	cmpq	$-1, 360(%rsp)
	movzbl	79(%rsp), %ecx
	sete	%al
	xorb	$1, %cl
	orb	156(%rsp), %cl
	orb	%al, %cl
	cmpb	$1, %cl
	je	.LBB241_544
	movq	$1, 1168(%rsp)
	xorl	%r13d, %r13d
	movq	$0, 1176(%rsp)
.Ltmp7051:
	movq	<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items@GOTPCREL(%rip), %rax
	movq	280(%rsp), %rsi
	leaq	496(%rsp), %rdi
	leaq	1168(%rsp), %rdx
	movl	$1, %ecx
	callq	*%rax
.Ltmp7052:
.LBB241_544:
	movq	256(%rsp), %rax
	movq	128(%rsp), %rcx
	movq	264(%rsp), %r14
	movq	272(%rsp), %rbx
	movq	%rax, 48(%rsp)
	lock		decq	(%rcx)
	jne	.LBB241_546
	xorl	%r15d, %r15d
	#MEMBARRIER
.Ltmp7056:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	920(%rsp), %rdi
	xorl	%r13d, %r13d
	callq	*%rax
.Ltmp7057:
.LBB241_546:
	xorl	%r15d, %r15d
.Ltmp7058:
	leaq	832(%rsp), %rdi
	xorl	%r13d, %r13d
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7059:
	movb	$2, %cl
	movq	%rbx, %rbp
.LBB241_548:
	movq	48(%rsp), %rax
	movq	%rax, 496(%rsp)
	movq	%r14, 504(%rsp)
	movq	%rbp, 512(%rsp)
.Ltmp7087:
	movzbl	%cl, %esi
	movzbl	156(%rsp), %edx
	movq	16(%rsp), %rdi
	movq	1704(%rsp), %rcx
	callq	purrdf_sparql_eval::row_checkpoint::settle_commit::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7088:
	movq	%rax, %rbx
	movq	%rdx, %r13
.LBB241_550:
	movq	%rbx, 80(%rsp)
	movq	48(%rsp), %rbx
	movq	%r13, 56(%rsp)
.LBB241_551:
	movq	1728(%rsp), %rcx
	movq	1744(%rsp), %rdx
	movq	1736(%rsp), %rax
	movq	1752(%rsp), %rsi
	movl	$1, %r12d
	movl	$1, %edi
	movq	%rbx, 160(%rsp)
	movq	%r14, 168(%rsp)
	movq	%r14, 8(%rsp)
	movq	%rbp, 40(%rsp)
	movq	%rbp, 176(%rsp)
	cmpq	$3, %rcx
	movq	%rcx, %r15
	cmovaeq	%rdx, %r15
	cmovaeq	%rcx, %rdi
	cmovaeq	%r12, %rdx
	movq	%rdi, 496(%rsp)
	movq	%rax, 504(%rsp)
	movq	%rdx, 512(%rsp)
	movq	%rsi, 520(%rsp)
	movq	%r15, %rsi
	decq	%rsi
	movq	$0, 528(%rsp)
	movq	%rsi, 536(%rsp)
	je	.LBB241_555
	cmpq	$3, %rcx
	movq	16(%rsp), %rcx
	movq	<purrdf_sparql_eval::witness::RelationWitness>::merge@GOTPCREL(%rip), %rbp
	leaq	504(%rsp), %r13
	leaq	928(%rsp), %r14
	cmovaeq	%rax, %r13
	leaq	640(%rcx), %rbx
	.p2align	4
.LBB241_553:
	movq	%r12, 528(%rsp)
	movq	16(%r13), %rax
	movq	%rax, 944(%rsp)
	vmovdqu	(%r13), %xmm0
	vmovdqa	%xmm0, 928(%rsp)
.Ltmp7130:
	movq	%rbx, %rdi
	movq	%r14, %rsi
	callq	*%rbp
.Ltmp7131:
	addq	$24, %r13
	incq	%r12
	cmpq	%r12, %r15
	jne	.LBB241_553
.LBB241_555:
.Ltmp7136:
	leaq	496(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7137:
	movq	56(%rsp), %r12
	testb	$1, 80(%rsp)
	je	.LBB241_561
	movq	400(%rsp), %rbx
	movq	%rbx, %rcx
	subq	%r12, %rcx
	jb	.LBB241_709
	movq	392(%rsp), %r15
.Ltmp7138:
	movq	16(%rsp), %rsi
	leaq	496(%rsp), %rdi
	xorl	%edx, %edx
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7139:
	cmpq	%r12, %rbx
	jne	.LBB241_637
.LBB241_560:
.Ltmp7163:
	leaq	496(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7164:
.LBB241_561:
	movq	176(%rsp), %rax
	vmovdqu	160(%rsp), %xmm0
	movq	%rax, 1344(%rsp)
	movq	352(%rsp), %rax
	vmovdqa	%xmm0, 1328(%rsp)
	testq	%rax, %rax
	je	.LBB241_564
	lock		decq	(%rax)
	jne	.LBB241_564
	#MEMBARRIER
.Ltmp7165:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	352(%rsp), %rdi
	callq	*%rax
.Ltmp7166:
.LBB241_564:
	movq	1400(%rsp), %rbx
	movb	$1, %r14b
.LBB241_565:
	movq	16(%rsp), %rax
	movq	696(%rax), %rax
	movl	40(%rax), %ecx
	testl	%ecx, %ecx
	je	.LBB241_569
.LBB241_566:
	vmovups	1992(%rsp), %zmm1
	vmovdqu64	1952(%rsp), %zmm0
	movq	1344(%rsp), %rcx
	movq	120(%rsp), %rax
	movq	%rcx, 1472(%rsp)
	vmovups	%zmm1, 536(%rsp)
	vmovdqa	1328(%rsp), %xmm1
	vmovdqu64	%zmm0, 496(%rsp)
	cmpq	$-1, 496(%rsp)
	vmovdqa	%xmm1, 1456(%rsp)
	movq	%rax, 1480(%rsp)
	je	.LBB241_585
	leaq	928(%rsp), %rdi
	leaq	1456(%rsp), %rsi
	leaq	1952(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	568(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB241_568
.LBB241_586:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	576(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB241_588
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB241_588:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB241_594
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB241_588
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB241_591:
	cmpq	%rax, %rdx
	jge	.LBB241_593
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB241_591
.LBB241_593:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB241_594:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	592(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB241_595
	jmp	.LBB241_597
.LBB241_569:
	movzbl	16(%rax), %ecx
	cmpb	$-1, %cl
	je	.LBB241_566
	movb	%cl, 928(%rsp)
	movq	120(%rsp), %rcx
	vmovdqu	17(%rax), %xmm0
	vmovdqu	%xmm0, 929(%rsp)
	movq	32(%rax), %rax
	movq	%rax, 944(%rsp)
	lock		incq	(%rcx)
	jle	.LBB241_715
	movq	120(%rsp), %rcx
	movl	%r14d, 8(%rsp)
.Ltmp7167:
	leaq	496(%rsp), %rdi
	leaq	928(%rsp), %rdx
	movq	%rbx, %rsi
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
.Ltmp7168:
	vmovdqu64	496(%rsp), %zmm0
	vmovdqu64	528(%rsp), %zmm1
	movq	136(%rsp), %rax
	movq	1336(%rsp), %r14
	movq	1344(%rsp), %rbx
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	testq	%rbx, %rbx
	je	.LBB241_627
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r12
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	xorl	%r15d, %r15d
	jmp	.LBB241_577
.LBB241_574:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB241_575:
	vzeroupper
	callq	*%rbp
.LBB241_576:
	incq	%r15
	cmpq	%rbx, %r15
	je	.LBB241_627
.LBB241_577:
	leaq	(%r15,%r15,4), %rcx
	movq	(%r14,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB241_576
	leaq	(%r14,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB241_580
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB241_580:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB241_575
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB241_580
	movq	%rcx, %rdx
	negq	%rdx
	xorl	%eax, %eax
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%r12)
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB241_583:
	cmpq	%rax, %rdx
	jge	.LBB241_574
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB241_583
	jmp	.LBB241_574
.LBB241_585:
	vmovdqu	1456(%rsp), %xmm0
	movq	1472(%rsp), %rax
	movq	1480(%rsp), %rcx
	movq	%rax, 952(%rsp)
	movq	%rcx, 960(%rsp)
	vmovdqu	%xmm0, 936(%rsp)
	movq	$-1, 928(%rsp)
	movq	568(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB241_586
.LBB241_568:
	movq	592(%rsp), %rax
	testq	%rax, %rax
	je	.LBB241_597
.LBB241_595:
	lock		decq	(%rax)
	jne	.LBB241_597
	leaq	592(%rsp), %rdi
	#MEMBARRIER
.Ltmp7170:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp7171:
.LBB241_597:
	vmovdqu64	928(%rsp), %zmm0
	vmovdqu64	960(%rsp), %zmm1
	movq	136(%rsp), %rax
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
.Ltmp7173:
	leaq	2112(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp7174:
.Ltmp7176:
	leaq	2328(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7177:
	movq	408(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB241_601
	#MEMBARRIER
.Ltmp7179:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	408(%rsp), %rdi
	callq	*%rax
.Ltmp7180:
.LBB241_601:
	testb	%r14b, %r14b
	je	.LBB241_626
	movq	392(%rsp), %r14
	movq	400(%rsp), %rbx
	testq	%rbx, %rbx
	je	.LBB241_615
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r12
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	xorl	%r15d, %r15d
	jmp	.LBB241_607
	.p2align	4
.LBB241_604:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB241_605:
	callq	*%rbp
.LBB241_606:
	incq	%r15
	cmpq	%rbx, %r15
	je	.LBB241_615
.LBB241_607:
	leaq	(%r15,%r15,4), %rcx
	movq	(%r14,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB241_606
	leaq	(%r14,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB241_610
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB241_610:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB241_605
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB241_610
	movq	%rcx, %rdx
	negq	%rdx
	xorl	%eax, %eax
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%r12)
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB241_613:
	cmpq	%rax, %rdx
	jge	.LBB241_604
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB241_613
	jmp	.LBB241_604
.LBB241_615:
	movq	384(%rsp), %rax
	testq	%rax, %rax
	je	.LBB241_626
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB241_618
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB241_618:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB241_624
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB241_618
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB241_621:
	cmpq	%rax, %rdx
	jge	.LBB241_623
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB241_621
.LBB241_623:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB241_624:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
.LBB241_625:
	vzeroupper
	callq	*%rax
.LBB241_626:
	movq	136(%rsp), %rax
	addq	$2648, %rsp
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
.LBB241_627:
	.cfi_def_cfa_offset 2704
	movq	1328(%rsp), %rax
	testq	%rax, %rax
	je	.LBB241_320
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB241_630
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB241_630:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB241_636
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB241_630
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
.LBB241_633:
	cmpq	%rax, %rdx
	jge	.LBB241_635
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB241_633
.LBB241_635:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB241_636:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
	jmp	.LBB241_320
.LBB241_637:
	movq	40(%rsp), %rax
	shlq	$3, %r12
	shlq	$3, %rbx
	leaq	1176(%rsp), %r13
	leaq	928(%rsp), %rbp
	leaq	(,%rax,8), %rax
	leaq	(%rax,%rax,4), %r14
	leaq	(%r12,%r12,4), %rax
	leaq	(%rbx,%rbx,4), %r12
	leaq	8(%r15,%rax), %r15
	subq	%rax, %r12
	jmp	.LBB241_639
.LBB241_638:
	movq	8(%rsp), %rcx
	addq	$40, %r15
	movq	%rbx, (%rcx,%r14)
	movq	%r12, 8(%rcx,%r14)
	vmovdqa	928(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rcx,%r14)
	movq	944(%rsp), %rax
	movq	%rax, 32(%rcx,%r14)
	movq	40(%rsp), %rax
	addq	$40, %r14
	incq	%rax
	addq	$-40, %rdx
	movq	%rdx, %r12
	movq	%rax, 40(%rsp)
	movq	%rax, 176(%rsp)
	je	.LBB241_560
.LBB241_639:
.Ltmp7140:
	movq	16(%rsp), %rdx
	leaq	496(%rsp), %rsi
	movq	%rbp, %rdi
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7141:
	cmpb	$-1, 928(%rsp)
	jne	.LBB241_560
	movq	776(%rsp), %rdx
	movq	$1, 928(%rsp)
	cmpq	$5, %rdx
	jae	.LBB241_660
.LBB241_642:
	vmovdqu	936(%rsp), %xmm0
	movq	960(%rsp), %rax
	movq	928(%rsp), %rdx
	movq	952(%rsp), %rcx
	movq	%rax, 1200(%rsp)
	movq	%rdx, 1168(%rsp)
	movq	%rcx, 1192(%rsp)
	movq	%r15, %rax
	vmovdqu	%xmm0, 1176(%rsp)
	movq	-8(%r15), %rbx
	decq	%rbx
	cmpq	$5, %rbx
	jb	.LBB241_644
	movq	8(%r15), %rbx
	movq	(%r15), %rax
	decq	%rbx
.LBB241_644:
	movq	%rax, 80(%rsp)
	movq	1168(%rsp), %rax
	movq	1184(%rsp), %rsi
	movl	$4, %edx
	movq	%r12, 48(%rsp)
	leaq	-1(%rax), %rcx
	decq	%rsi
	cmpq	$5, %rcx
	cmovbq	%rcx, %rsi
	cmovbq	%rdx, %rcx
	subq	%rsi, %rcx
	cmpq	%rbx, %rcx
	jb	.LBB241_661
.LBB241_645:
	xorl	%ebp, %ebp
	movq	%r13, %r12
	movq	%r13, %rcx
	cmpq	$6, %rax
	setae	%al
	jb	.LBB241_647
	movq	1176(%rsp), %rcx
.LBB241_647:
	movb	%al, %bpl
	movq	80(%rsp), %rsi
	movq	memcpy@GOTPCREL(%rip), %rax
	leaq	(,%rbx,8), %rdx
	shll	$4, %ebp
	movq	1168(%rsp,%rbp), %r13
	leaq	-8(%rcx,%r13,8), %rdi
	callq	*%rax
	movq	776(%rsp), %rsi
	addq	%rbx, %r13
	movq	%r13, 1168(%rsp,%rbp)
.Ltmp7148:
	leaq	1168(%rsp), %rdi
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::resize
.Ltmp7149:
	leaq	928(%rsp), %rbp
	movq	1168(%rsp), %rcx
	movq	%r12, %r13
	movq	%r12, %rdx
	decq	%rcx
	cmpq	$5, %rcx
	jb	.LBB241_650
	movq	1184(%rsp), %rcx
	movq	1176(%rsp), %rdx
	decq	%rcx
.LBB241_650:
	movq	120(%rsp), %r8
	addq	$16, %r8
.Ltmp7150:
	movq	16(%rsp), %r9
	leaq	2112(%rsp), %rsi
	movq	%rbp, %rdi
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7151:
	vmovq	936(%rsp), %xmm0
	movq	928(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB241_663
	movq	1168(%rsp), %rax
	movq	%rax, %rsi
	cmpq	$6, %rax
	jb	.LBB241_654
	movq	1184(%rsp), %rsi
.LBB241_654:
	movq	912(%rsp), %rdi
	decq	%rsi
	cmpq	%rsi, %rdi
	jae	.LBB241_714
	movq	%r13, %rcx
	cmpq	$6, %rax
	jb	.LBB241_657
	movq	1176(%rsp), %rcx
.LBB241_657:
	vmovq	%xmm0, (%rcx,%rdi,8)
	leaq	1184(%rsp), %rax
	movq	48(%rsp), %rdx
	movq	40(%rsp), %rcx
	vmovdqu	(%rax), %xmm0
	movq	16(%rax), %rax
	movq	1168(%rsp), %rbx
	movq	1176(%rsp), %r12
	movq	%rax, 944(%rsp)
	vmovdqa	%xmm0, 928(%rsp)
	cmpq	160(%rsp), %rcx
	jne	.LBB241_638
.Ltmp7158:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	callq	*%rax
.Ltmp7159:
	movq	168(%rsp), %rax
	movq	48(%rsp), %rdx
	movq	%rax, 8(%rsp)
	jmp	.LBB241_638
.LBB241_660:
.Ltmp7143:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	movq	%rbp, %rdi
	xorl	%esi, %esi
	xorl	%ecx, %ecx
	callq	*%rax
.Ltmp7144:
	jmp	.LBB241_642
.LBB241_661:
.Ltmp7146:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	1168(%rsp), %rdi
	movl	$1, %ecx
	movq	%rbx, %rdx
	callq	*%rax
.Ltmp7147:
	movq	1168(%rsp), %rax
	jmp	.LBB241_645
.LBB241_663:
	vmovups	944(%rsp), %zmm1
	vmovups	960(%rsp), %zmm2
	movq	136(%rsp), %rcx
	vmovups	%zmm2, 48(%rcx)
	vmovups	%zmm1, 32(%rcx)
	movq	%rax, 16(%rcx)
	movq	1168(%rsp), %rax
	vmovq	%xmm0, 24(%rcx)
	movq	$1, (%rcx)
	cmpq	$6, %rax
	jb	.LBB241_665
	movq	1176(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB241_665:
.Ltmp7153:
	leaq	496(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7154:
	leaq	160(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	352(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB241_317
	jmp	.LBB241_319
.LBB241_667:
	vmovups	208(%rsp), %ymm0
	movq	168(%rsp), %rax
	movzbl	199(%rsp), %edx
	movzwl	197(%rsp), %ecx
	movq	176(%rsp), %rsi
	movq	184(%rsp), %r14
	movzbl	192(%rsp), %ebp
	addq	$40, %rbx
	movb	$1, %r15b
	movq	%rbx, 840(%rsp)
	movq	%rax, 48(%rsp)
	movl	193(%rsp), %eax
	shll	$16, %edx
	movq	%rsi, 8(%rsp)
	orl	%edx, %ecx
	shlq	$32, %rcx
	vmovups	%ymm0, 784(%rsp)
	vmovdqu	224(%rsp), %ymm0
	orq	%rcx, %rax
	movq	200(%rsp), %rcx
	movq	%rax, 64(%rsp)
	movq	%rcx, 56(%rsp)
	vmovdqu	%ymm0, 800(%rsp)
	movq	104(%rsp), %r13
	testq	%r13, %r13
	jne	.LBB241_686
	jmp	.LBB241_687
.LBB241_668:
	movq	40(%rsp), %r12
	leaq	-1(%r14), %rax
	cmpq	%rax, %r12
	jae	.LBB241_682
	movq	8(%rsp), %r15
	cmpq	112(%rsp), %r15
	je	.LBB241_683
	notq	%r12
	addq	$88, %r15
	leaq	160(%rsp), %rbx
	addq	%r14, %r12
	movq	%r15, %rax
.LBB241_671:
	movq	%rax, %r15
	movq	-8(%r15), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rcx, 480(%rsp)
	vmovdqu64	-72(%r15), %zmm0
	vmovdqu64	%zmm0, 416(%rsp)
	cmpq	$-1, %rax
	je	.LBB241_683
	vmovdqu64	416(%rsp), %zmm0
	movq	%rax, 160(%rsp)
	movq	480(%rsp), %rax
	leaq	168(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7020:
	movq	48(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%rbx, %rdx
	vzeroupper
	callq	*%rax
.Ltmp7021:
	decq	%r12
	je	.LBB241_683
	leaq	-88(%r15), %rcx
	leaq	88(%r15), %rax
	addq	$88, %rcx
	cmpq	112(%rsp), %rcx
	jne	.LBB241_671
	jmp	.LBB241_683
.LBB241_675:
	movq	40(%rsp), %r12
	leaq	-1(%r14), %rax
	cmpq	%rax, %r12
	jae	.LBB241_682
	movq	8(%rsp), %r15
	cmpq	112(%rsp), %r15
	je	.LBB241_683
	notq	%r12
	addq	$88, %r15
	leaq	160(%rsp), %rbx
	addq	%r14, %r12
	movq	%r15, %rax
.LBB241_678:
	movq	%rax, %r15
	movq	-8(%r15), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rcx, 480(%rsp)
	vmovdqu64	-72(%r15), %zmm0
	vmovdqu64	%zmm0, 416(%rsp)
	cmpq	$-1, %rax
	je	.LBB241_683
	vmovdqu64	416(%rsp), %zmm0
	movq	%rax, 160(%rsp)
	movq	480(%rsp), %rax
	leaq	168(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7014:
	movq	48(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%rbx, %rdx
	vzeroupper
	callq	*%rax
.Ltmp7015:
	decq	%r12
	je	.LBB241_683
	leaq	-88(%r15), %rcx
	leaq	88(%r15), %rax
	addq	$88, %rcx
	cmpq	112(%rsp), %rcx
	jne	.LBB241_678
	jmp	.LBB241_683
.LBB241_682:
	movq	8(%rsp), %r15
.LBB241_683:
	movq	%r15, 312(%rsp)
.LBB241_684:
	movq	256(%rsp), %rax
	movq	264(%rsp), %rcx
	movq	272(%rsp), %r14
	movq	$-1, %r12
	movb	$1, %bpl
.LBB241_685:
	xorl	%r15d, %r15d
	movq	%rax, 48(%rsp)
	movq	%rcx, 8(%rsp)
	movq	104(%rsp), %r13
	testq	%r13, %r13
	je	.LBB241_687
.LBB241_686:
	movq	488(%rsp), %rdi
	shlq	$5, %r13
	movl	$8, %edx
	movq	%r13, %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB241_687:
.Ltmp7029:
	leaq	304(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp7030:
	movq	768(%rsp), %rax
	testq	%rax, %rax
	je	.LBB241_690
	movq	144(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB241_690:
	movq	584(%rsp), %rax
	testq	%rax, %rax
	je	.LBB241_693
	lock		decq	(%rax)
	jne	.LBB241_693
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	584(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB241_693:
	movq	616(%rsp), %rax
	testq	%rax, %rax
	je	.LBB241_696
	lock		decq	(%rax)
	jne	.LBB241_696
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	616(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB241_696:
	xorl	%r13d, %r13d
	movq	%rbp, 80(%rsp)
	movq	%r14, 40(%rsp)
.Ltmp7032:
	leaq	1360(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp7033:
	movq	128(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB241_699
	xorl	%r13d, %r13d
	#MEMBARRIER
.Ltmp7034:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	920(%rsp), %rdi
	callq	*%rax
.Ltmp7035:
.LBB241_699:
	xorl	%r13d, %r13d
.LBB241_700:
	cmpq	$0, 1392(%rsp)
	je	.LBB241_702
.Ltmp7065:
	leaq	832(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7066:
.LBB241_702:
	movq	760(%rsp), %r14
	movq	344(%rsp), %rbp
	testb	%r15b, %r15b
	movq	896(%rsp), %r15
	je	.LBB241_704
	leaq	256(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB241_704:
	testb	%r13b, %r13b
	jne	.LBB241_294
	jmp	.LBB241_300
.LBB241_705:
	movq	256(%rsp), %rax
	movq	264(%rsp), %rcx
	movq	272(%rsp), %r14
	movq	$-1, %r12
	xorl	%ebp, %ebp
	jmp	.LBB241_685
.LBB241_706:
	movq	256(%rsp), %rax
	movq	264(%rsp), %rcx
	movq	272(%rsp), %r14
	movq	$-1, %r12
	movb	$1, %bpl
	xorl	%r15d, %r15d
	movq	%rax, 48(%rsp)
	movq	%rcx, 8(%rsp)
	testq	%r13, %r13
	jne	.LBB241_686
	jmp	.LBB241_687
.LBB241_707:
.Ltmp7090:
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.152(%rip), %rdi
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.154(%rip), %rdx
	movl	$83, %esi
	vzeroupper
	callq	*%rax
.Ltmp7091:
	jmp	.LBB241_715
.LBB241_708:
.Ltmp7039:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	movq	888(%rsp), %rdx
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.290(%rip), %rcx
	movq	%r15, %rdi
	callq	*%rax
.Ltmp7040:
	jmp	.LBB241_715
.LBB241_709:
.Ltmp7182:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.404(%rip), %rcx
	movq	%r12, %rdi
	movq	%rbx, %rsi
	movq	%rbx, %rdx
	callq	*%rax
.Ltmp7183:
	jmp	.LBB241_715
.LBB241_710:
	movq	%r15, 1464(%rsp)
	movq	%r13, 1488(%rsp)
.Ltmp6909:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.402(%rip), %rdx
	callq	*%rax
.Ltmp6910:
	jmp	.LBB241_715
.LBB241_711:
.Ltmp7209:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	leaq	512(%rsp), %r14
	callq	*%rax
.Ltmp7210:
	jmp	.LBB241_715
.LBB241_712:
.Ltmp7248:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	leaq	944(%rsp), %r14
	callq	*%rax
.Ltmp7249:
	jmp	.LBB241_715
.LBB241_713:
.Ltmp6900:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp6901:
	jmp	.LBB241_715
.LBB241_714:
.Ltmp7155:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.403(%rip), %rdx
	callq	*%rax
.Ltmp7156:
.LBB241_715:
	ud2
.LBB241_716:
.Ltmp7145:
	movq	%rax, 16(%rsp)
	movq	928(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB241_805
	movq	936(%rsp), %rdi
	jmp	.LBB241_803
.LBB241_718:
.Ltmp7016:
	jmp	.LBB241_720
.LBB241_719:
.Ltmp7022:
.LBB241_720:
	movq	%rax, 16(%rsp)
	movq	%r15, 312(%rsp)
	jmp	.LBB241_838
.LBB241_721:
.Ltmp7046:
	movq	%rax, 16(%rsp)
	movb	$1, %r15b
	jmp	.LBB241_841
.LBB241_722:
.Ltmp7031:
	movq	%rax, 16(%rsp)
	jmp	.LBB241_841
.LBB241_723:
.Ltmp6992:
	movq	%rax, 16(%rsp)
	movq	%r14, 312(%rsp)
	jmp	.LBB241_838
.LBB241_724:
.Ltmp7067:
	movq	%rax, 16(%rsp)
	jmp	.LBB241_854
.LBB241_725:
.Ltmp7075:
	cmpq	$0, 1392(%rsp)
	movq	%rax, 16(%rsp)
	jne	.LBB241_853
	jmp	.LBB241_854
.LBB241_726:
.Ltmp6948:
	movq	%rax, 16(%rsp)
	jmp	.LBB241_768
.LBB241_727:
.Ltmp7053:
	movq	%rax, 16(%rsp)
	jmp	.LBB241_851
.LBB241_728:
.Ltmp7089:
	leaq	496(%rsp), %rdi
	movq	%rax, 16(%rsp)
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB241_864
.LBB241_729:
.Ltmp7119:
	movq	%rax, 16(%rsp)
	jmp	.LBB241_779
.LBB241_730:
.Ltmp7004:
	jmp	.LBB241_764
.LBB241_731:
.Ltmp6981:
	movq	%rax, 16(%rsp)
	jmp	.LBB241_857
.LBB241_732:
.Ltmp6934:
	movq	%rax, 16(%rsp)
	jmp	.LBB241_769
.LBB241_733:
.Ltmp7100:
	movq	%rax, 16(%rsp)
	jmp	.LBB241_861
.LBB241_734:
.Ltmp7062:
	addq	$40, %r13
	movq	%rax, 16(%rsp)
	movq	%r13, 168(%rsp)
	jmp	.LBB241_743
.LBB241_735:
.Ltmp7025:
	jmp	.LBB241_764
.LBB241_736:
.Ltmp6953:
	movq	%rax, 16(%rsp)
	jmp	.LBB241_864
.LBB241_737:
.Ltmp6976:
	movq	%rax, 16(%rsp)
.Ltmp6977:
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp6978:
	jmp	.LBB241_857
.LBB241_738:
.Ltmp6967:
	movq	%rax, 16(%rsp)
	jmp	.LBB241_783
.LBB241_739:
.Ltmp6929:
	movq	%rax, 16(%rsp)
.Ltmp6930:
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp6931:
	jmp	.LBB241_769
.LBB241_740:
.Ltmp6956:
	movq	%rax, 16(%rsp)
	jmp	.LBB241_784
.LBB241_741:
.Ltmp7070:
	addq	$40, %r13
	cmpq	$6, 80(%rsp)
	movq	%rax, 16(%rsp)
	movq	%r13, 168(%rsp)
	jb	.LBB241_743
	movq	80(%rsp), %rax
	movq	56(%rsp), %rdi
	movl	$4, %edx
	leaq	-8(,%rax,8), %rsi
	callq	__rustc::__rust_dealloc
.LBB241_743:
	movb	$1, %r13b
.Ltmp7071:
	leaq	160(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7072:
	jmp	.LBB241_855
.LBB241_744:
.Ltmp7127:
	movq	%rax, 16(%rsp)
	jmp	.LBB241_864
.LBB241_745:
.Ltmp7095:
	movq	%rax, 16(%rsp)
.Ltmp7096:
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7097:
	jmp	.LBB241_861
.LBB241_746:
.Ltmp7169:
	leaq	1328(%rsp), %rdi
	movq	%rax, 16(%rsp)
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB241_869
.LBB241_747:
.Ltmp7172:
	movl	%r14d, 8(%rsp)
	movq	%rax, 16(%rsp)
	xorl	%ebx, %ebx
	jmp	.LBB241_870
.LBB241_748:
.Ltmp7001:
	jmp	.LBB241_837
.LBB241_749:
.Ltmp7028:
	addq	$40, %rbx
	movq	%rax, 16(%rsp)
	movq	%rbx, 840(%rsp)
	jmp	.LBB241_838
.LBB241_750:
.Ltmp7160:
	movq	%rax, 16(%rsp)
	cmpq	$6, %rbx
	jb	.LBB241_805
	leaq	-8(,%rbx,8), %rsi
	movl	$4, %edx
	movq	%r12, %rdi
	jmp	.LBB241_804
.LBB241_752:
.Ltmp7019:
	jmp	.LBB241_837
.LBB241_753:
.Ltmp7205:
	jmp	.LBB241_792
.LBB241_754:
.Ltmp7038:
	addq	$40, %rbx
	cmpq	$6, 56(%rsp)
	movq	%rax, 16(%rsp)
	movq	%rbx, 840(%rsp)
	jb	.LBB241_838
	movq	56(%rsp), %rax
	movq	64(%rsp), %rdi
	movl	$4, %edx
	leaq	-8(,%rax,8), %rsi
	callq	__rustc::__rust_dealloc
	jmp	.LBB241_838
.LBB241_756:
.Ltmp7110:
	jmp	.LBB241_795
.LBB241_757:
.Ltmp6937:
	movq	40(%rsp), %rcx
	movq	%rax, 16(%rsp)
	movq	%rcx, 424(%rsp)
	jmp	.LBB241_767
.LBB241_758:
.Ltmp7181:
	movq	%rax, 16(%rsp)
	testb	%r14b, %r14b
	jne	.LBB241_881
	jmp	.LBB241_882
.LBB241_759:
.Ltmp7234:
	movq	%rax, 16(%rsp)
	lock		decq	(%r14)
	jne	.LBB241_827
	#MEMBARRIER
.Ltmp7235:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	496(%rsp), %rdi
	callq	*%rax
.Ltmp7236:
	jmp	.LBB241_827
.LBB241_761:
.Ltmp7237:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB241_762:
.Ltmp7103:
	movq	40(%rsp), %rcx
	movq	%rax, 16(%rsp)
	movq	%rcx, 792(%rsp)
	jmp	.LBB241_778
.LBB241_763:
.Ltmp7009:
.LBB241_764:
	movq	8(%rsp), %rcx
	movq	%rax, 16(%rsp)
	movq	%rcx, 312(%rsp)
	jmp	.LBB241_838
.LBB241_765:
.Ltmp6942:
	movq	40(%rsp), %rcx
	movq	%rax, 16(%rsp)
	movq	%rcx, 424(%rsp)
	cmpq	$6, %rbx
	jb	.LBB241_767
	leaq	-8(,%rbx,8), %rsi
	movl	$4, %edx
	movq	%r12, %rdi
	callq	__rustc::__rust_dealloc
.LBB241_767:
.Ltmp6943:
	leaq	416(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp6944:
.LBB241_768:
	leaq	784(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB241_769:
.Ltmp6949:
	leaq	928(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp6950:
	jmp	.LBB241_864
.LBB241_770:
.Ltmp6945:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB241_771:
.Ltmp7208:
	movq	%rax, 16(%rsp)
	testb	%r15b, %r15b
	jne	.LBB241_881
	jmp	.LBB241_882
.LBB241_772:
.Ltmp7199:
	movq	%rax, 16(%rsp)
	jmp	.LBB241_812
.LBB241_773:
.Ltmp7157:
	jmp	.LBB241_801
.LBB241_774:
.Ltmp6924:
	movq	%rax, 16(%rsp)
.Ltmp6925:
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp6926:
	jmp	.LBB241_865
.LBB241_775:
.Ltmp7142:
	movq	%rax, 16(%rsp)
	jmp	.LBB241_805
.LBB241_776:
.Ltmp7113:
	movq	40(%rsp), %rcx
	movq	%rax, 16(%rsp)
	movq	%rcx, 792(%rsp)
	cmpq	$6, %rbx
	jb	.LBB241_778
	leaq	-8(,%rbx,8), %rsi
	movl	$4, %edx
	movq	%r12, %rdi
	callq	__rustc::__rust_dealloc
.LBB241_778:
.Ltmp7114:
	leaq	784(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7115:
.LBB241_779:
	leaq	1408(%rsp), %rdi
	jmp	.LBB241_860
.LBB241_780:
.Ltmp7116:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB241_781:
.Ltmp6959:
	movq	%rax, 16(%rsp)
	movq	%r12, 1176(%rsp)
.Ltmp6960:
	leaq	496(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp6961:
.Ltmp6963:
	leaq	1168(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp6964:
.LBB241_783:
.Ltmp6968:
	leaq	160(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp6969:
.LBB241_784:
.Ltmp6971:
	leaq	1920(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp6972:
	jmp	.LBB241_864
.LBB241_785:
.Ltmp6962:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB241_786:
.Ltmp6970:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB241_787:
.Ltmp6973:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB241_788:
.Ltmp7224:
	movq	%rax, 16(%rsp)
	lock		decq	(%r14)
	jne	.LBB241_827
	#MEMBARRIER
.Ltmp7225:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	496(%rsp), %rdi
	callq	*%rax
.Ltmp7226:
	jmp	.LBB241_827
.LBB241_790:
.Ltmp7227:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB241_791:
.Ltmp7178:
	movl	%r14d, 8(%rsp)
.LBB241_792:
	movq	%rax, 16(%rsp)
	jmp	.LBB241_878
.LBB241_793:
.Ltmp7175:
	movl	%r14d, 8(%rsp)
	movq	%rax, 16(%rsp)
	jmp	.LBB241_874
.LBB241_794:
.Ltmp6921:
.LBB241_795:
	movq	%rax, 16(%rsp)
	jmp	.LBB241_865
.LBB241_796:
.Ltmp7250:
	movq	%rax, 16(%rsp)
.Ltmp7251:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.18159039729619107857)
.Ltmp7252:
.Ltmp7254:
	leaq	1816(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp7255:
	jmp	.LBB241_882
.LBB241_798:
.Ltmp7253:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB241_799:
.Ltmp7243:
	movq	%rax, 16(%rsp)
.Ltmp7244:
	leaq	2112(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.18159039729619107857)
.Ltmp7245:
	jmp	.LBB241_827
.LBB241_800:
.Ltmp7152:
.LBB241_801:
	movq	%rax, 16(%rsp)
	movq	1168(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB241_805
	movq	1176(%rsp), %rdi
.LBB241_803:
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
.LBB241_804:
	callq	__rustc::__rust_dealloc
.LBB241_805:
.Ltmp7161:
	leaq	496(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7162:
	jmp	.LBB241_835
.LBB241_806:
.Ltmp7189:
	movq	%rax, 16(%rsp)
	movb	$1, %al
	movb	$1, %bl
	movl	%eax, 8(%rsp)
	jmp	.LBB241_870
.LBB241_807:
.Ltmp7211:
	movq	%rax, 16(%rsp)
.Ltmp7212:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.18159039729619107857)
.Ltmp7213:
	jmp	.LBB241_811
.LBB241_808:
.Ltmp7214:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB241_809:
.Ltmp6893:
	movq	%rax, 16(%rsp)
.Ltmp6894:
	leaq	2056(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.18159039729619107857)
.Ltmp6895:
	jmp	.LBB241_811
.LBB241_810:
.Ltmp6890:
	movq	%rax, 16(%rsp)
.LBB241_811:
	movb	$1, %al
	movl	%eax, 8(%rsp)
.LBB241_812:
	movb	$1, %bl
	jmp	.LBB241_875
.LBB241_813:
.Ltmp6914:
	movq	%r15, 1464(%rsp)
	movq	%rax, 16(%rsp)
	movq	%r13, 1488(%rsp)
	cmpq	$5, %rbp
	ja	.LBB241_832
	jmp	.LBB241_833
.LBB241_814:
.Ltmp7083:
	movq	%rax, 16(%rsp)
	testq	%r13, %r13
	je	.LBB241_818
	negq	%r13
	addq	$160, %r15
.LBB241_816:
.Ltmp7084:
	movq	%r15, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7085:
	addq	$160, %r15
	decq	%r13
	jne	.LBB241_816
.LBB241_818:
	cmpq	$0, 344(%rsp)
	je	.LBB241_864
	movq	344(%rsp), %rax
	movq	760(%rsp), %rdi
	movl	$8, %edx
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
	jmp	.LBB241_864
.LBB241_820:
.Ltmp7086:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB241_821:
.Ltmp7132:
	movq	%rax, 16(%rsp)
.Ltmp7133:
	leaq	496(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7134:
	jmp	.LBB241_835
.LBB241_822:
.Ltmp7135:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB241_823:
.Ltmp6911:
	movq	%rax, 16(%rsp)
	jmp	.LBB241_830
.LBB241_824:
.Ltmp7194:
	movq	%rax, 16(%rsp)
	jmp	.LBB241_872
.LBB241_825:
.Ltmp7202:
	movl	%r15d, 8(%rsp)
	movq	%rax, 16(%rsp)
	jmp	.LBB241_877
.LBB241_826:
.Ltmp7240:
	movq	%rax, 16(%rsp)
.LBB241_827:
.Ltmp7246:
	leaq	1816(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp7247:
	jmp	.LBB241_882
.LBB241_828:
.Ltmp7256:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB241_829:
.Ltmp6908:
	movq	%r15, 1464(%rsp)
	movq	%rax, 16(%rsp)
	movq	%r13, 1488(%rsp)
.LBB241_830:
	movq	928(%rsp), %rbp
	cmpq	$6, %rbp
	jb	.LBB241_833
	movq	936(%rsp), %rax
	movq	%rax, 40(%rsp)
.LBB241_832:
	movq	40(%rsp), %rdi
	leaq	-8(,%rbp,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB241_833:
	leaq	1456(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, <purrdf_sparql_eval::bgp::BgpProjection>::apply<purrdf_core::ir::term::TermId>::{closure#0}>>
	leaq	160(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movb	$1, %bl
	movl	$0, 8(%rsp)
	jmp	.LBB241_870
.LBB241_834:
.Ltmp7184:
	movq	%rax, 16(%rsp)
.LBB241_835:
	leaq	160(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB241_865
.LBB241_836:
.Ltmp7041:
.LBB241_837:
	movq	%rax, 16(%rsp)
.LBB241_838:
	cmpq	$0, 104(%rsp)
	je	.LBB241_840
	movq	104(%rsp), %rsi
	movq	488(%rsp), %rdi
	movl	$8, %edx
	shlq	$5, %rsi
	callq	__rustc::__rust_dealloc
.LBB241_840:
	movb	$1, %r15b
.Ltmp7042:
	leaq	304(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp7043:
.LBB241_841:
	cmpq	$0, 768(%rsp)
	je	.LBB241_843
	movq	768(%rsp), %rax
	movq	144(%rsp), %rdi
	movl	$8, %edx
	shlq	$3, %rax
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB241_843:
	movq	584(%rsp), %rax
	testq	%rax, %rax
	je	.LBB241_846
	lock		decq	(%rax)
	jne	.LBB241_846
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	584(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB241_846:
	movq	616(%rsp), %rax
	testq	%rax, %rax
	je	.LBB241_849
	lock		decq	(%rax)
	jne	.LBB241_849
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	616(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB241_849:
.Ltmp7047:
	leaq	1360(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp7048:
	xorl	%r13d, %r13d
.LBB241_851:
	movq	128(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB241_853
	#MEMBARRIER
.Ltmp7054:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	920(%rsp), %rdi
	callq	*%rax
.Ltmp7055:
.LBB241_853:
.Ltmp7076:
	leaq	832(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7077:
.LBB241_854:
	testb	%r15b, %r15b
	je	.LBB241_856
.LBB241_855:
	leaq	256(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB241_856:
	testb	%r13b, %r13b
	je	.LBB241_864
.LBB241_857:
.Ltmp7078:
	leaq	1784(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp7079:
	jmp	.LBB241_864
.LBB241_858:
.Ltmp7080:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB241_859:
.Ltmp7092:
	leaq	1360(%rsp), %rdi
	movq	%rax, 16(%rsp)
.LBB241_860:
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB241_861:
.Ltmp7120:
	leaq	496(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp7121:
	cmpb	$0, 56(%rsp)
	je	.LBB241_864
.Ltmp7122:
	leaq	832(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7123:
.LBB241_864:
.Ltmp7128:
	leaq	1728(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7129:
.LBB241_865:
	movq	352(%rsp), %rax
	movb	$1, %cl
	movl	%ecx, 8(%rsp)
	testq	%rax, %rax
	je	.LBB241_869
	lock		decq	(%rax)
	movb	$1, %al
	movl	%eax, 8(%rsp)
	jne	.LBB241_869
	movb	$1, %al
	#MEMBARRIER
	movl	%eax, 8(%rsp)
.Ltmp7185:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	352(%rsp), %rdi
	callq	*%rax
.Ltmp7186:
	movb	$1, %bl
	jmp	.LBB241_870
.LBB241_869:
	movb	$1, %bl
.LBB241_870:
.Ltmp7190:
	leaq	2112(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp7191:
	testb	%bl, %bl
	je	.LBB241_874
.LBB241_872:
	movq	120(%rsp), %rax
	movb	$1, %bl
	lock		decq	(%rax)
	jne	.LBB241_875
	#MEMBARRIER
.Ltmp7195:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	120(%rsp), %rdi
	callq	*%rax
.Ltmp7196:
	jmp	.LBB241_875
.LBB241_874:
	xorl	%ebx, %ebx
.LBB241_875:
.Ltmp7215:
	leaq	2328(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7216:
	testb	%bl, %bl
	je	.LBB241_878
.LBB241_877:
.Ltmp7217:
	leaq	1952(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp7218:
.LBB241_878:
	movq	408(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB241_880
	leaq	408(%rsp), %rdi
	#MEMBARRIER
.Ltmp7219:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp7220:
.LBB241_880:
	cmpb	$0, 8(%rsp)
	je	.LBB241_882
.LBB241_881:
	leaq	384(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB241_882:
	movq	16(%rsp), %rdi
	callq	_Unwind_Resume@PLT
.LBB241_883:
.Ltmp7124:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB241_884:
.Ltmp7221:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end241:
