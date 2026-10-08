purrdf_sparql_eval::expr::eval_extend::<purrdf_core::ir::dataset::RdfDataset>:
.Lfunc_begin239:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception159
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
	movq	%rdi, 104(%rsp)
	leaq	1816(%rsp), %rdi
	movq	%r9, %r12
	movq	%r8, %r14
	movq	%rcx, %rbp
	movq	%rdx, %rbx
	movq	%rsi, %r15
	callq	*%rax
.Ltmp6930:
	leaq	2528(%rsp), %rdi
	movq	%rbx, %rsi
	movq	%r12, 16(%rsp)
	movq	%r12, %rdx
	movq	%rbx, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp6931:
	cmpl	$1, 2528(%rsp)
	jne	.LBB239_25
	vmovdqu64	2544(%rsp), %zmm0
	vmovdqu64	2576(%rsp), %zmm1
	movq	104(%rsp), %r15
	movq	1888(%rsp), %rax
	vmovdqu64	%zmm1, 48(%r15)
	vmovdqu64	%zmm0, 16(%r15)
	movq	$1, (%r15)
	cmpq	$6, %rax
	jb	.LBB239_12
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
	jge	.LBB239_5
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB239_5:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB239_11
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB239_5
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
.LBB239_8:
	cmpq	%rax, %rsi
	jge	.LBB239_10
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB239_8
.LBB239_10:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB239_11:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB239_12:
	movq	1816(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB239_22
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
	jge	.LBB239_15
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB239_15:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB239_21
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB239_15
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
.LBB239_18:
	cmpq	%rax, %rsi
	jge	.LBB239_20
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB239_18
.LBB239_20:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB239_21:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB239_22:
	movq	1912(%rsp), %rax
	testq	%rax, %rax
	je	.LBB239_626
	lock		decq	(%rax)
	jne	.LBB239_626
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1912(%rsp), %rdi
	#MEMBARRIER
	jmp	.LBB239_625
.LBB239_25:
	vmovdqu64	2568(%rsp), %zmm1
	vmovdqu64	2536(%rsp), %zmm0
	vmovdqu64	%zmm1, 528(%rsp)
	vmovdqu64	%zmm0, 496(%rsp)
.Ltmp6932:
	leaq	944(%rsp), %rdi
	leaq	1816(%rsp), %rsi
	leaq	496(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp6933:
	cmpq	$-1, 944(%rsp)
	je	.LBB239_55
	vmovdqu	944(%rsp), %ymm0
	vmovdqu64	1856(%rsp), %zmm1
	vmovdqu64	1816(%rsp), %zmm2
	movb	$1, %r12b
	movq	%r15, 920(%rsp)
	vmovdqu64	%zmm1, 1992(%rsp)
	vmovdqu	%ymm0, 384(%rsp)
	vmovdqu64	%zmm2, 1952(%rsp)
.Ltmp6934:
	movq	16(%rsp), %rbx
	movq	%r14, %rsi
	movq	%rbx, %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop
.Ltmp6935:
	cmpb	$2, 472(%rbx)
	sete	%cl
	andb	%cl, %al
	cmpb	$1, %al
	jne	.LBB239_34
	movq	616(%rbx), %rax
	testq	%rax, %rax
	je	.LBB239_33
	cmpq	$-2, 24(%rax)
	jb	.LBB239_34
	cmpq	$-2, 32(%rax)
	jb	.LBB239_34
	cmpq	$-3, 48(%rax)
	jbe	.LBB239_34
.LBB239_33:
	movq	16(%rsp), %rax
	cmpq	$1025, 400(%rsp)
	movzbl	1234(%rax), %ebx
	setae	%al
	notb	%bl
	andb	%al, %bl
	jmp	.LBB239_35
.LBB239_34:
	xorl	%ebx, %ebx
.LBB239_35:
	movq	400(%rsp), %r15
.Ltmp6936:
	movq	16(%rsp), %rsi
	movzbl	%bl, %edx
	leaq	2328(%rsp), %rdi
	movq	%r15, %rcx
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp6937:
	movq	408(%rsp), %rsi
	addq	$16, %rsi
.Ltmp6938:
	leaq	2056(%rsp), %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.4261325137610144415)
.Ltmp6939:
	movq	(%rbp), %rsi
	lock		incq	(%rsi)
	jle	.LBB239_721
	movq	8(%rbp), %rdx
.Ltmp6941:
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	2056(%rsp), %rdi
	callq	*%rax
.Ltmp6942:
	vmovdqu	2080(%rsp), %ymm0
	vmovdqu	2056(%rsp), %ymm1
	movq	%rax, 928(%rsp)
	movq	2072(%rsp), %rax
	movq	malloc@GOTPCREL(%rip), %r12
	movl	$72, %edi
	movq	%rax, 776(%rsp)
	vmovdqu	%ymm0, 536(%rsp)
	vmovdqu	%ymm1, 512(%rsp)
	movq	$1, 496(%rsp)
	movq	$1, 504(%rsp)
	vzeroupper
	callq	*%r12
	testq	%rax, %rax
	je	.LBB239_717
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
	jle	.LBB239_42
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB239_42:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB239_48
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB239_42
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
.LBB239_45:
	cmpq	%rax, %rdx
	jle	.LBB239_47
	lock		cmpxchgq	%rdx, (%rsi)
	jne	.LBB239_45
.LBB239_47:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB239_48:
	vmovdqu64	496(%rsp), %zmm0
	movq	560(%rsp), %rax
	movq	%rcx, 128(%rsp)
	movq	%rax, 64(%rcx)
	movb	$1, %al
	movl	%eax, 8(%rsp)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp6946:
	movq	16(%rsp), %r13
	movq	920(%rsp), %rsi
	movq	%r14, %rdx
	movq	%r13, %rdi
	vzeroupper
	callq	purrdf_sparql_eval::vm::program_at::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp6947:
	movq	128(%rsp), %rcx
	addq	$16, %rcx
.Ltmp6948:
	leaq	2112(%rsp), %rbp
	movq	%rax, %rsi
	movq	%r14, %rdx
	movq	%r13, %r8
	movq	%rbp, %rdi
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::link::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp6949:
	testb	%bl, %bl
	je	.LBB239_60
	movq	2512(%rsp), %rax
	movq	16(%rsp), %rdi
	movq	392(%rsp), %rbx
	cmpq	%r15, %rax
	movq	1040(%rdi), %rcx
	cmovbq	%rax, %r15
	addq	904(%rdi), %rcx
	movq	%rcx, 1720(%rsp)
.Ltmp6965:
	movq	%r15, %rsi
	callq	<purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot
.Ltmp6966:
	movq	16(%rsp), %r14
	leaq	2328(%rsp), %r12
	movq	%rax, 352(%rsp)
	movq	616(%r14), %rcx
	testq	%rcx, %rcx
	je	.LBB239_125
	cmpq	$-2, 16(%rcx)
	movb	$1, %al
	jb	.LBB239_126
	cmpq	$-2, 40(%rcx)
	setb	%al
	jmp	.LBB239_126
.LBB239_55:
	movq	1912(%rsp), %r15
	testq	%r15, %r15
	je	.LBB239_70
	lock		incq	(%r15)
	jle	.LBB239_721
	movq	%r15, 496(%rsp)
	leaq	16(%r15), %rsi
.Ltmp7272:
	leaq	2112(%rsp), %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.4261325137610144415)
.Ltmp7273:
	lock		decq	(%r15)
	jne	.LBB239_74
	#MEMBARRIER
.Ltmp7278:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	496(%rsp), %rdi
	callq	*%rax
.Ltmp7279:
	jmp	.LBB239_74
.LBB239_60:
	leaq	(,%r15,8), %rax
	leaq	(%rax,%rax,4), %rbx
	testq	%r15, %r15
	je	.LBB239_101
	movq	%rbx, %rdi
	callq	*%r12
	testq	%rax, %rax
	je	.LBB239_719
	movq	%rax, %r14
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rsi
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdi
	movabsq	$-9223372036854775808, %rcx
	movq	$-1, %r8
	leaq	(%rbx,%rax), %rdx
	sarq	$63, %rdx
	xorq	%rcx, %rdx
	addq	%rbx, %rax
	cmovoq	%rdx, %rax
	incq	%rsi
	cmoveq	%r8, %rsi
	addq	%rbx, %rdi
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmovbq	%r8, %rdi
	movq	%rsi, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%rdi, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB239_64
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB239_64:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB239_102
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB239_64
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
.LBB239_67:
	cmpq	%rax, %rdx
	jle	.LBB239_69
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB239_67
.LBB239_69:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jmp	.LBB239_102
.LBB239_70:
.Ltmp7280:
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp7281:
	movq	%rax, %rsi
	movq	%rax, %rbx
	movq	%rax, 496(%rsp)
	addq	$16, %rsi
.Ltmp7282:
	leaq	2112(%rsp), %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.4261325137610144415)
.Ltmp7283:
	lock		decq	(%rbx)
	jne	.LBB239_74
	#MEMBARRIER
.Ltmp7288:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	496(%rsp), %rdi
	callq	*%rax
.Ltmp7289:
.LBB239_74:
	movq	(%rbp), %rsi
	lock		incq	(%rsi)
	jle	.LBB239_721
	movq	8(%rbp), %rdx
.Ltmp7291:
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	2112(%rsp), %rdi
	callq	*%rax
.Ltmp7292:
	vmovdqu64	1856(%rsp), %zmm1
	vmovups	1816(%rsp), %zmm0
	vmovdqu	2136(%rsp), %ymm2
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$72, %edi
	vmovdqu64	%zmm1, 536(%rsp)
	vmovups	%zmm0, 496(%rsp)
	vmovdqu	2112(%rsp), %ymm0
	vmovdqu	%ymm2, 984(%rsp)
	vmovdqu	%ymm0, 960(%rsp)
	movq	$1, 944(%rsp)
	movq	$1, 952(%rsp)
	vzeroupper
	callq	*%rax
	movq	104(%rsp), %r15
	testq	%rax, %rax
	je	.LBB239_718
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
	jle	.LBB239_79
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB239_79:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB239_85
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB239_79
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
.LBB239_82:
	cmpq	%rax, %rdx
	jle	.LBB239_84
	lock		cmpxchgq	%rdx, (%rsi)
	jne	.LBB239_82
.LBB239_84:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB239_85:
	vmovups	944(%rsp), %zmm0
	movq	1008(%rsp), %rax
	cmpq	$-1, 496(%rsp)
	movq	%rcx, 1496(%rsp)
	movq	$0, 1472(%rsp)
	movq	$8, 1480(%rsp)
	movq	$0, 1488(%rsp)
	movq	%rax, 64(%rcx)
	vmovups	%zmm0, (%rcx)
	je	.LBB239_87
	leaq	944(%rsp), %rdi
	leaq	1472(%rsp), %rsi
	leaq	1816(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	568(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB239_88
	jmp	.LBB239_97
.LBB239_87:
	movq	1480(%rsp), %rcx
	movq	1472(%rsp), %rax
	movq	1488(%rsp), %rdx
	movq	%rcx, 960(%rsp)
	movq	1496(%rsp), %rcx
	movq	%rax, 952(%rsp)
	movq	%rdx, 968(%rsp)
	movq	%rcx, 976(%rsp)
	movq	$-1, 944(%rsp)
	movq	568(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB239_97
.LBB239_88:
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
	jge	.LBB239_90
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB239_90:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB239_96
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB239_90
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
.LBB239_93:
	cmpq	%rax, %rdx
	jge	.LBB239_95
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB239_93
.LBB239_95:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB239_96:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB239_97:
	movq	592(%rsp), %rax
	testq	%rax, %rax
	je	.LBB239_100
	lock		decq	(%rax)
	jne	.LBB239_100
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	592(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB239_100:
	vmovdqu64	976(%rsp), %zmm1
	vmovdqu64	944(%rsp), %zmm0
	vmovdqu64	%zmm1, 40(%r15)
	vmovdqu64	%zmm0, 8(%r15)
	movq	$0, (%r15)
	jmp	.LBB239_626
.LBB239_101:
	movl	$8, %r14d
.LBB239_102:
	movq	392(%rsp), %rax
	movq	384(%rsp), %rcx
	movq	%r15, 144(%rsp)
	movq	%r14, 152(%rsp)
	movq	$0, 160(%rsp)
	addq	%rax, %rbx
	movq	%rax, 1472(%rsp)
	movq	%rcx, 1488(%rsp)
	movq	%rcx, 48(%rsp)
	movq	%rax, 304(%rsp)
	movq	%rbx, 1496(%rsp)
	testq	%r15, %r15
	je	.LBB239_124
	leaq	952(%rsp), %r13
	leaq	496(%rsp), %r12
	xorl	%edx, %edx
	movq	%rbx, 40(%rsp)
	jmp	.LBB239_106
	.p2align	4
.LBB239_104:
	movq	152(%rsp), %rdx
.LBB239_105:
	movq	8(%rsp), %rax
	leaq	(%rax,%rax,4), %rax
	movq	%r13, (%rdx,%rax,8)
	movq	%r14, 8(%rdx,%rax,8)
	movq	%rdx, %r14
	movq	%r15, %r13
	vmovdqa	496(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rdx,%rax,8)
	movq	512(%rsp), %rcx
	movq	%rcx, 32(%rdx,%rax,8)
	movq	%rbx, 160(%rsp)
	movq	%rbx, %rdx
	movq	%rbp, %rax
	cmpq	40(%rsp), %rbp
	je	.LBB239_185
.LBB239_106:
	movq	(%rax), %rcx
	leaq	40(%rax), %rbp
	testq	%rcx, %rcx
	je	.LBB239_148
	vmovdqu	8(%rax), %ymm0
	leaq	1(%rdx), %rbx
	movq	%rcx, 944(%rsp)
	movq	%rdx, 8(%rsp)
	vmovdqu	%ymm0, 1184(%rsp)
	vmovdqu	%ymm0, (%r13)
.Ltmp6952:
	movq	16(%rsp), %rdx
	leaq	2328(%rsp), %rsi
	movq	%r12, %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp6953:
	cmpb	$-1, 496(%rsp)
	jne	.LBB239_149
	movq	944(%rsp), %rcx
	movq	960(%rsp), %rdx
	movq	776(%rsp), %rax
	leaq	-1(%rcx), %rsi
	leaq	-1(%rdx), %rdi
	cmpq	$5, %rsi
	cmovbq	%rsi, %rdi
	movq	%rax, %rsi
	subq	%rdi, %rsi
	jbe	.LBB239_111
	movl	$2, 496(%rsp)
	movq	%rsi, 504(%rsp)
.Ltmp6954:
	leaq	944(%rsp), %rdi
	movq	%r12, %rsi
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp6955:
	jmp	.LBB239_113
	.p2align	4
.LBB239_111:
	cmpq	$6, %rcx
	cmovbq	%rcx, %rdx
	decq	%rdx
	cmpq	%rdx, %rax
	jae	.LBB239_113
	xorl	%edx, %edx
	cmpq	$6, %rcx
	setae	%dl
	incq	%rax
	shll	$4, %edx
	movq	%rax, 944(%rsp,%rdx)
.LBB239_113:
	movq	16(%rsp), %rax
	movq	8(%rsp), %rcx
	movq	%r13, %rdx
	movq	%rcx, 568(%rax)
	movq	944(%rsp), %rcx
	decq	%rcx
	cmpq	$5, %rcx
	jb	.LBB239_115
	movq	960(%rsp), %rcx
	movq	952(%rsp), %rdx
	decq	%rcx
.LBB239_115:
	movq	128(%rsp), %r8
	addq	$16, %r8
.Ltmp6956:
	movq	16(%rsp), %r9
	leaq	2112(%rsp), %rsi
	movq	%r12, %rdi
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp6957:
	vmovq	504(%rsp), %xmm0
	movq	496(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB239_152
	movq	944(%rsp), %rax
	movq	%rax, %rsi
	cmpq	$6, %rax
	jb	.LBB239_119
	movq	960(%rsp), %rsi
.LBB239_119:
	movq	928(%rsp), %rdi
	decq	%rsi
	cmpq	%rsi, %rdi
	jae	.LBB239_716
	movq	%r14, %rdx
	movq	%r13, %r15
	cmpq	$6, %rax
	jb	.LBB239_122
	movq	952(%rsp), %r13
.LBB239_122:
	vmovq	%xmm0, (%r13,%rdi,8)
	movq	8(%rsp), %rcx
	vmovdqu	8(%r15), %xmm0
	movq	24(%r15), %rax
	movq	944(%rsp), %r13
	movq	952(%rsp), %r14
	movq	%rax, 512(%rsp)
	vmovdqa	%xmm0, 496(%rsp)
	cmpq	144(%rsp), %rcx
	jne	.LBB239_105
.Ltmp6962:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	144(%rsp), %rdi
	callq	*%rax
.Ltmp6963:
	jmp	.LBB239_104
.LBB239_124:
	xorl	%edx, %edx
	movq	%rax, %rbp
	jmp	.LBB239_186
.LBB239_125:
	xorl	%eax, %eax
.LBB239_126:
	movzbl	1234(%r14), %ecx
	leaq	776(%rsp), %rdi
	leaq	128(%rsp), %rsi
	movq	%rbx, 1472(%rsp)
	movq	%r15, 1480(%rsp)
	leaq	352(%rsp), %rdx
	movq	%r14, 1184(%rsp)
	movq	%rdi, 1488(%rsp)
	movq	%rsi, 1496(%rsp)
	leaq	928(%rsp), %rdi
	leaq	1720(%rsp), %rsi
	movq	%rdx, 1192(%rsp)
	movq	%r12, 1200(%rsp)
	movq	%rbp, 1208(%rsp)
	movq	%rdi, 1504(%rsp)
	movq	%rsi, 1512(%rsp)
	testb	%al, %al
	je	.LBB239_128
.Ltmp6969:
	movzbl	%cl, %esi
	leaq	496(%rsp), %rdi
	leaq	1184(%rsp), %r8
	leaq	1472(%rsp), %r9
	movq	%r12, (%rsp)
	movq	%rbx, %rdx
	movq	%r15, %rcx
	callq	purrdf_sparql_eval::parallel::par_blocks_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>
.Ltmp6970:
	movq	104(%rsp), %r12
	jmp	.LBB239_129
.LBB239_128:
.Ltmp6967:
	movzbl	%cl, %esi
	leaq	496(%rsp), %rdi
	leaq	1184(%rsp), %r8
	leaq	1472(%rsp), %r9
	movq	%r12, (%rsp)
	movq	%rbx, %rdx
	movq	%r15, %rcx
	callq	purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>
.Ltmp6968:
	movq	104(%rsp), %r12
.LBB239_129:
	movq	496(%rsp), %r13
	cmpq	$-1, %r13
	je	.LBB239_137
	vmovups	688(%rsp), %zmm0
	vmovups	672(%rsp), %zmm1
	movq	520(%rsp), %rax
	movq	536(%rsp), %rdx
	movq	504(%rsp), %r15
	movq	512(%rsp), %rbp
	movq	528(%rsp), %rcx
	movl	$1, %edi
	movq	%r13, 864(%rsp)
	leaq	-3(%rax), %rsi
	movq	%r15, 872(%rsp)
	movq	%rbp, 880(%rsp)
	cmpq	$-2, %rsi
	movl	$1, %esi
	cmovbq	%rax, %rdi
	cmovbq	%rdx, %rax
	cmovaeq	%rdx, %rsi
	vmovups	%zmm0, 1088(%rsp)
	vmovups	%zmm1, 1072(%rsp)
	vmovdqu64	544(%rsp), %zmm0
	vmovdqu64	608(%rsp), %zmm1
	decq	%rax
	vmovdqu64	1088(%rsp), %zmm3
	vmovdqu64	1072(%rsp), %zmm2
	vmovdqu64	%zmm0, 944(%rsp)
	vmovdqu64	%zmm1, 1008(%rsp)
	vmovdqu64	%zmm1, 584(%rsp)
	vmovdqu64	%zmm0, 520(%rsp)
	vmovdqu64	%zmm3, 664(%rsp)
	vmovdqu64	%zmm2, 648(%rsp)
	movq	%rdi, 496(%rsp)
	movq	%rcx, 504(%rsp)
	movq	%rsi, 512(%rsp)
	movq	$0, 728(%rsp)
	movq	%rax, 736(%rsp)
.Ltmp6972:
	leaq	944(%rsp), %rdi
	leaq	496(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp6973:
	vmovups	1120(%rsp), %zmm2
	vmovups	1104(%rsp), %zmm1
	vmovups	944(%rsp), %ymm0
	movq	616(%r14), %rbx
	leaq	888(%r14), %rax
	movq	%rbp, 80(%rsp)
	movq	%rax, 48(%rsp)
	vmovups	%zmm2, 1616(%rsp)
	vmovups	%zmm1, 1600(%rsp)
	vmovups	1040(%rsp), %zmm2
	vmovups	976(%rsp), %zmm1
	vmovups	%ymm0, 1728(%rsp)
	vmovups	%zmm2, 1536(%rsp)
	vmovups	%zmm1, 1472(%rsp)
	testq	%rbx, %rbx
	je	.LBB239_138
	vmovdqu	864(%rsp), %xmm0
	vmovdqu64	1472(%rsp), %zmm4
	vmovdqu64	1536(%rsp), %zmm1
	vmovdqu64	1616(%rsp), %zmm3
	vmovdqu64	1600(%rsp), %zmm2
	movq	880(%rsp), %rax
	cmpb	$2, 2522(%rsp)
	movq	%rax, 1936(%rsp)
	vmovdqu64	%zmm3, 1088(%rsp)
	vmovdqa	%xmm0, 1920(%rsp)
	vmovdqu64	%zmm2, 1072(%rsp)
	vmovdqu64	%zmm1, 1008(%rsp)
	vmovdqu64	%zmm4, 944(%rsp)
	jne	.LBB239_144
	movq	944(%rsp), %rax
	vmovdqu64	1496(%rsp), %zmm0
	vmovdqu64	1616(%rsp), %zmm2
	vmovdqu64	1560(%rsp), %zmm1
	movq	960(%rsp), %rdx
	movq	952(%rsp), %rcx
	movl	$1, %edi
	movl	$1, %esi
	movq	%r15, 40(%rsp)
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
.Ltmp7004:
	leaq	1760(%rsp), %rdi
	leaq	496(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp7005:
	movq	1776(%rsp), %rcx
	movq	1768(%rsp), %rbp
	imulq	$200, %rcx, %rax
	addq	%rbp, %rax
	movq	%rax, 8(%rsp)
	testq	%rcx, %rcx
	je	.LBB239_258
	movl	%ecx, %esi
	andl	$3, %esi
	cmpq	$4, %rcx
	jae	.LBB239_232
	xorl	%edi, %edi
	xorl	%r10d, %r10d
	jmp	.LBB239_251
.LBB239_137:
	vmovdqu64	544(%rsp), %zmm0
	vmovdqu	512(%rsp), %ymm1
	vmovdqu64	%zmm0, 48(%r12)
	vmovdqu	%ymm1, 16(%r12)
	vmovdqu64	%zmm0, 944(%rsp)
	movq	$1, (%r12)
	movq	352(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB239_312
	jmp	.LBB239_314
.LBB239_138:
	vmovups	1536(%rsp), %zmm1
	movq	880(%rsp), %rax
	vmovdqu	864(%rsp), %xmm0
	vmovdqu64	1472(%rsp), %zmm4
	vmovdqu64	1616(%rsp), %zmm3
	vmovdqu64	1600(%rsp), %zmm2
	movq	%r13, 56(%rsp)
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
	movq	%rax, 1392(%rsp)
	movzbl	2522(%rsp), %eax
	movb	%al, 96(%rsp)
	vmovdqa	%xmm1, 1376(%rsp)
	testb	%al, %al
	jne	.LBB239_713
	vmovdqa	1376(%rsp), %xmm0
	movq	1392(%rsp), %rax
	movq	%rax, 336(%rsp)
	vmovdqa	%xmm0, 320(%rsp)
.Ltmp7143:
	leaq	944(%rsp), %rdi
	leaq	320(%rsp), %rsi
	movq	%rbp, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp7144:
	movq	944(%rsp), %rax
	movq	952(%rsp), %r14
	movq	960(%rsp), %rbx
	movq	968(%rsp), %rbp
	cmpq	$-1, %rax
	je	.LBB239_212
	vmovdqa	976(%rsp), %xmm0
	vmovdqu	992(%rsp), %ymm2
	vmovdqu	1008(%rsp), %ymm1
	movq	%rax, %r13
	vmovdqu	%ymm2, 144(%rsp)
	vmovdqa	%xmm0, 16(%rsp)
	vmovdqu	%ymm1, 160(%rsp)
.Ltmp7148:
	leaq	864(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7149:
	movq	%r13, %r15
.LBB239_142:
	vmovups	144(%rsp), %ymm0
	vmovups	160(%rsp), %ymm1
	vmovups	%ymm0, 416(%rsp)
	vmovups	%ymm1, 432(%rsp)
.Ltmp7156:
	leaq	496(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp7157:
	vmovdqu	432(%rsp), %ymm1
	vmovups	416(%rsp), %ymm0
	vmovdqu	%ymm1, 80(%r12)
	vmovups	%ymm0, 64(%r12)
	vmovdqa	16(%rsp), %xmm0
	jmp	.LBB239_310
.LBB239_144:
	movq	$0, 320(%rsp)
	movq	$8, 328(%rsp)
	movq	%r13, 56(%rsp)
	movq	$0, 336(%rsp)
.Ltmp6977:
	leaq	496(%rsp), %rdi
	leaq	320(%rsp), %rsi
	movq	%rbp, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp6978:
	movq	496(%rsp), %rax
	movq	504(%rsp), %r14
	movq	512(%rsp), %rbx
	movq	520(%rsp), %rbp
	cmpq	$-1, %rax
	je	.LBB239_222
	movq	%rax, %r13
	movq	528(%rsp), %rax
	vmovdqu	544(%rsp), %ymm0
	vmovdqu	560(%rsp), %ymm1
	movq	%rax, 48(%rsp)
	movq	536(%rsp), %rax
	vmovdqu	%ymm0, 144(%rsp)
	vmovdqu	%ymm1, 160(%rsp)
	movq	%rax, 80(%rsp)
.Ltmp6982:
	leaq	864(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp6983:
	movq	%r13, %r15
	movq	48(%rsp), %r13
.LBB239_147:
	vmovups	144(%rsp), %ymm0
	vmovups	160(%rsp), %ymm1
	movq	%r13, 48(%rsp)
	shrq	$8, %r13
	vmovups	%ymm0, 1424(%rsp)
	vmovups	%ymm1, 1440(%rsp)
	jmp	.LBB239_285
.LBB239_148:
	movq	40(%rsp), %rbx
	jmp	.LBB239_186
.LBB239_149:
	movq	944(%rsp), %rax
	movq	%rbp, 1480(%rsp)
	movq	%rbx, 1504(%rsp)
	cmpq	$6, %rax
	jb	.LBB239_151
	movq	952(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB239_151:
	movq	40(%rsp), %rbx
	jmp	.LBB239_187
.LBB239_152:
	vmovdqu64	512(%rsp), %zmm1
	vmovdqu64	528(%rsp), %zmm2
	movq	104(%rsp), %rcx
	vmovdqu64	%zmm2, 48(%rcx)
	vmovdqu64	%zmm1, 32(%rcx)
	movq	%rax, 16(%rcx)
	movq	944(%rsp), %rax
	vmovq	%xmm0, 24(%rcx)
	movq	$1, (%rcx)
	cmpq	$6, %rax
	jb	.LBB239_154
	movq	952(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB239_154:
	movq	40(%rsp), %rax
	movq	8(%rsp), %rdx
	subq	%rbp, %rax
	je	.LBB239_167
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r13
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r12
	shrq	$3, %rax
	movabsq	$-3689348814741910323, %rbx
	xorl	%r15d, %r15d
	imulq	%rax, %rbx
	jmp	.LBB239_159
.LBB239_156:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB239_157:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	8(%rsp), %rdx
.LBB239_158:
	incq	%r15
	cmpq	%rbx, %r15
	je	.LBB239_167
.LBB239_159:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbp,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB239_158
	leaq	(%rbp,%rcx,8), %rdx
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
	jge	.LBB239_162
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB239_162:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB239_157
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB239_162
	movq	%rcx, %rdx
	negq	%rdx
	xorl	%eax, %eax
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%r13)
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r12), %rax
	.p2align	4
.LBB239_165:
	cmpq	%rax, %rdx
	jge	.LBB239_156
	lock		cmpxchgq	%rdx, (%r12)
	jne	.LBB239_165
	jmp	.LBB239_156
.LBB239_167:
	movq	48(%rsp), %rax
	testq	%rax, %rax
	je	.LBB239_169
	movq	304(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
	movq	8(%rsp), %rdx
.LBB239_169:
	testq	%rdx, %rdx
	je	.LBB239_182
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r15
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r12
	movq	free@GOTPCREL(%rip), %rbp
	xorl	%ebx, %ebx
	jmp	.LBB239_174
.LBB239_171:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB239_172:
	vzeroupper
	callq	*%rbp
	movq	8(%rsp), %rdx
.LBB239_173:
	incq	%rbx
	cmpq	%rbx, %rdx
	je	.LBB239_182
.LBB239_174:
	leaq	(%rbx,%rbx,4), %rcx
	movq	(%r14,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB239_173
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
	jge	.LBB239_177
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB239_177:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB239_172
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB239_177
	movq	%rcx, %rdx
	negq	%rdx
	xorl	%eax, %eax
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%r15)
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r12), %rax
	.p2align	4
.LBB239_180:
	cmpq	%rax, %rdx
	jge	.LBB239_171
	lock		cmpxchgq	%rdx, (%r12)
	jne	.LBB239_180
	jmp	.LBB239_171
.LBB239_182:
	movq	144(%rsp), %rax
	testq	%rax, %rax
	je	.LBB239_184
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r14, %rdi
	leaq	(%rax,%rax,4), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB239_184:
	movl	$0, 8(%rsp)
	jmp	.LBB239_315
.LBB239_185:
	movq	%rbx, %rdx
	movq	40(%rsp), %rbx
	movq	%rbx, %rbp
.LBB239_186:
	movq	%rbp, 1480(%rsp)
	movq	%rdx, 1504(%rsp)
.LBB239_187:
	subq	%rbp, %rbx
	je	.LBB239_200
	shrq	$3, %rbx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r15
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r12
	movq	free@GOTPCREL(%rip), %r13
	xorl	%r14d, %r14d
	movq	%rbx, %rax
	movabsq	$-3689348814741910323, %rbx
	imulq	%rax, %rbx
	jmp	.LBB239_192
	.p2align	4
.LBB239_189:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB239_190:
	callq	*%r13
.LBB239_191:
	incq	%r14
	cmpq	%rbx, %r14
	je	.LBB239_200
.LBB239_192:
	leaq	(%r14,%r14,4), %rcx
	movq	(%rbp,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB239_191
	leaq	(%rbp,%rcx,8), %rdx
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
	jge	.LBB239_195
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB239_195:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB239_190
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB239_195
	movq	%rcx, %rdx
	negq	%rdx
	xorl	%eax, %eax
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%r15)
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r12), %rax
	.p2align	4
.LBB239_198:
	cmpq	%rax, %rdx
	jge	.LBB239_189
	lock		cmpxchgq	%rdx, (%r12)
	jne	.LBB239_198
	jmp	.LBB239_189
.LBB239_200:
	movq	48(%rsp), %rax
	movq	920(%rsp), %r14
	testq	%rax, %rax
	je	.LBB239_211
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
	jge	.LBB239_203
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB239_203:
	movq	304(%rsp), %rdi
	.p2align	4
.LBB239_204:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB239_210
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB239_204
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
.LBB239_207:
	cmpq	%rax, %rdx
	jge	.LBB239_209
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB239_207
.LBB239_209:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB239_210:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB239_211:
	vmovdqu	144(%rsp), %xmm0
	movq	160(%rsp), %rax
	xorl	%ebp, %ebp
	movq	%rax, 1360(%rsp)
	vmovdqa	%xmm0, 1344(%rsp)
	jmp	.LBB239_565
.LBB239_212:
	movq	80(%rsp), %r13
	movq	56(%rsp), %rdx
	movq	%r14, 1424(%rsp)
	movq	%rbx, 1432(%rsp)
	movq	%r15, 784(%rsp)
	movq	%rbx, 8(%rsp)
	movq	%rbp, 1440(%rsp)
	leaq	(,%r13,8), %rax
	movq	%rdx, 800(%rsp)
	leaq	(%rax,%rax,4), %r14
	leaq	(%r15,%r14), %rcx
	movq	%rcx, 808(%rsp)
	testq	%r13, %r13
	je	.LBB239_279
	leaq	(,%rbp,8), %rax
	addq	$40, %r15
	movq	%rcx, 80(%rsp)
	leaq	(%rax,%rax,4), %rbx
	jmp	.LBB239_216
.LBB239_214:
	movq	1432(%rsp), %rax
	movq	%rax, 8(%rsp)
.LBB239_215:
	movq	8(%rsp), %rax
	vmovdqa	304(%rsp), %xmm0
	movq	%r15, (%rax,%rbx)
	movq	40(%rsp), %r15
	movq	%r12, 8(%rax,%rbx)
	vmovdqu	%xmm0, 16(%rax,%rbx)
	movq	%rbp, 32(%rax,%rbx)
	movq	%r13, %rbp
	incq	%rbp
	addq	$40, %rbx
	movq	%rbp, 1440(%rsp)
	addq	$40, %r15
	addq	$-40, %r14
	je	.LBB239_278
.LBB239_216:
	movq	-8(%r15), %rax
	leaq	1192(%rsp), %rcx
	movq	%r15, 40(%rsp)
	movq	%rax, 32(%rcx)
	movq	16(%rsp), %rax
	vmovdqu	-40(%r15), %ymm0
	vmovdqu	%ymm0, (%rcx)
	movq	%rax, 1184(%rsp)
	cmpq	$0, 1192(%rsp)
	je	.LBB239_218
	leaq	-40(%r15), %rax
	leaq	952(%rsp), %rdx
	movq	32(%rax), %rcx
	movq	%rcx, 32(%rdx)
	vmovdqu	(%rax), %ymm0
	vmovdqu	%ymm0, (%rdx)
	jmp	.LBB239_220
.LBB239_218:
	movq	664(%rax), %rdx
.Ltmp7151:
	movq	48(%rsp), %rsi
	leaq	944(%rsp), %rdi
	leaq	1200(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7152:
	movq	944(%rsp), %r15
	cmpq	$-1, %r15
	jne	.LBB239_375
.LBB239_220:
	vmovdqu	968(%rsp), %xmm0
	movq	%rbp, %rax
	movq	952(%rsp), %r15
	movq	960(%rsp), %r12
	movq	984(%rsp), %rbp
	movq	%rax, %r13
	vmovdqa	%xmm0, 304(%rsp)
	cmpq	1424(%rsp), %rax
	jne	.LBB239_215
.Ltmp7161:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	1424(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp7162:
	jmp	.LBB239_214
.LBB239_222:
	movq	80(%rsp), %r13
	movq	56(%rsp), %rdx
	movq	%r14, 784(%rsp)
	movq	%rbx, 792(%rsp)
	movq	%r15, 416(%rsp)
	movq	%rbx, 8(%rsp)
	movq	%rbp, 800(%rsp)
	leaq	(,%r13,8), %rax
	movq	%rdx, 432(%rsp)
	leaq	(%rax,%rax,4), %r14
	leaq	(%r15,%r14), %rcx
	movq	%rcx, 56(%rsp)
	movq	%rcx, 440(%rsp)
	testq	%r13, %r13
	je	.LBB239_283
	leaq	(,%rbp,8), %rax
	addq	$40, %r15
	leaq	(%rax,%rax,4), %r13
	jmp	.LBB239_225
.LBB239_224:
	movq	8(%rsp), %rax
	vmovdqa	304(%rsp), %xmm0
	movq	%r15, (%rax,%r13)
	movq	40(%rsp), %r15
	movq	%r12, 8(%rax,%r13)
	vmovdqu	%xmm0, 16(%rax,%r13)
	movq	%rbp, 32(%rax,%r13)
	movq	%rbx, %rbp
	incq	%rbp
	addq	$40, %r13
	movq	%rbp, 800(%rsp)
	addq	$40, %r15
	addq	$-40, %r14
	je	.LBB239_282
.LBB239_225:
	movq	16(%rsp), %rcx
	leaq	1192(%rsp), %rdx
	movq	%r15, 40(%rsp)
	movq	%rcx, 1184(%rsp)
	movq	-8(%r15), %rax
	movq	%rax, 32(%rdx)
	vmovdqu	-40(%r15), %ymm0
	vmovdqu	%ymm0, (%rdx)
	cmpq	$0, 1192(%rsp)
	je	.LBB239_227
	leaq	-40(%r15), %rax
	leaq	504(%rsp), %rdx
	movq	32(%rax), %rcx
	movq	%rcx, 32(%rdx)
	vmovdqu	(%rax), %ymm0
	vmovdqu	%ymm0, (%rdx)
	jmp	.LBB239_229
.LBB239_227:
	movq	664(%rcx), %rdx
.Ltmp6985:
	movq	48(%rsp), %rsi
	leaq	496(%rsp), %rdi
	leaq	1200(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp6986:
	movq	496(%rsp), %r15
	cmpq	$-1, %r15
	jne	.LBB239_377
.LBB239_229:
	vmovdqu	520(%rsp), %xmm0
	movq	%rbp, %rax
	movq	504(%rsp), %r15
	movq	512(%rsp), %r12
	movq	536(%rsp), %rbp
	movq	%rax, %rbx
	vmovdqa	%xmm0, 304(%rsp)
	cmpq	784(%rsp), %rax
	jne	.LBB239_224
.Ltmp6990:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	784(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp6991:
	movq	792(%rsp), %rax
	movq	%rax, 8(%rsp)
	jmp	.LBB239_224
.LBB239_232:
	movq	%rcx, %r8
	andq	$-4, %r8
	leaq	776(%rbp), %r9
	xorl	%edi, %edi
	xorl	%r10d, %r10d
	jmp	.LBB239_234
	.p2align	4
.LBB239_233:
	addq	$4, %rdi
	addq	$800, %r9
	cmpq	%rdi, %r8
	je	.LBB239_250
.LBB239_234:
	movq	-600(%r9), %rax
	mulq	-608(%r9)
	jo	.LBB239_243
	addq	%rax, %r10
	movq	$-1, %r11
	jb	.LBB239_236
.LBB239_244:
	movq	%r10, %r11
	movq	-400(%r9), %rax
	mulq	-408(%r9)
	jno	.LBB239_237
.LBB239_245:
	movq	$-1, %rax
	addq	%rax, %r11
	movq	$-1, %r10
	jae	.LBB239_246
	.p2align	4
.LBB239_238:
	movq	-200(%r9), %rax
	mulq	-208(%r9)
	jo	.LBB239_247
.LBB239_239:
	addq	%rax, %r10
	movq	$-1, %r11
	jb	.LBB239_240
.LBB239_248:
	movq	%r10, %r11
	movq	(%r9), %rax
	mulq	-8(%r9)
	jno	.LBB239_241
.LBB239_249:
	movq	$-1, %rax
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB239_233
	jmp	.LBB239_242
.LBB239_243:
	movq	$-1, %rax
	addq	%rax, %r10
	movq	$-1, %r11
	jae	.LBB239_244
	.p2align	4
.LBB239_236:
	movq	-400(%r9), %rax
	mulq	-408(%r9)
	jo	.LBB239_245
.LBB239_237:
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB239_238
.LBB239_246:
	movq	%r11, %r10
	movq	-200(%r9), %rax
	mulq	-208(%r9)
	jno	.LBB239_239
.LBB239_247:
	movq	$-1, %rax
	addq	%rax, %r10
	movq	$-1, %r11
	jae	.LBB239_248
	.p2align	4
.LBB239_240:
	movq	(%r9), %rax
	mulq	-8(%r9)
	jo	.LBB239_249
.LBB239_241:
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB239_233
.LBB239_242:
	movq	%r11, %r10
	jmp	.LBB239_233
.LBB239_250:
	testq	%rsi, %rsi
	je	.LBB239_255
.LBB239_251:
	imulq	$200, %rdi, %rax
	imulq	$200, %rsi, %rsi
	movq	$-1, %r9
	xorl	%r8d, %r8d
	leaq	176(%rax,%rbp), %rdi
	.p2align	4
.LBB239_252:
	movq	(%rdi,%r8), %rax
	mulq	-8(%rdi,%r8)
	jo	.LBB239_254
.LBB239_253:
	addq	%rax, %r10
	cmovbq	%r9, %r10
	addq	$200, %r8
	cmpq	%r8, %rsi
	jne	.LBB239_252
	jmp	.LBB239_255
.LBB239_254:
	movq	$-1, %rax
	jmp	.LBB239_253
.LBB239_255:
	testq	%r10, %r10
	je	.LBB239_258
	cmpq	$0, 336(%rbx)
	je	.LBB239_258
	lock		addq	%r10, 352(%rbx)
.LBB239_258:
	movq	1760(%rsp), %rax
	movq	8(%rsp), %rdx
	movq	%rbp, 1184(%rsp)
	movq	$0, 144(%rsp)
	movq	$8, 152(%rsp)
	movq	$0, 160(%rsp)
	movq	%rax, 1200(%rsp)
	movq	%rdx, 1208(%rsp)
	testq	%rcx, %rcx
	je	.LBB239_267
	movq	%r13, 56(%rsp)
	movl	$8, %eax
	leaq	504(%rsp), %r13
	addq	$200, %rbp
	xorl	%r15d, %r15d
	xorl	%r12d, %r12d
	movq	%rax, 304(%rsp)
	.p2align	4
.LBB239_260:
	movq	-200(%rbp), %rax
	cmpq	$-1, %rax
	je	.LBB239_268
	leaq	-200(%rbp), %rbx
	vmovups	8(%rbx), %zmm0
	vmovups	72(%rbx), %zmm1
	vmovups	96(%rbx), %zmm2
	vmovups	%zmm2, 88(%r13)
	vmovups	%zmm1, 64(%r13)
	vmovups	%zmm0, (%r13)
	movq	%rax, 496(%rsp)
	movq	%r15, %rax
	movq	%rax, %r14
	movzbl	648(%rsp), %r15d
	cmpq	144(%rsp), %rax
	jne	.LBB239_264
.Ltmp7007:
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one@GOTPCREL(%rip), %rax
	leaq	144(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp7008:
	movq	152(%rsp), %rax
	movq	%rax, 304(%rsp)
.LBB239_264:
	vmovdqu64	496(%rsp), %zmm0
	vmovdqu64	560(%rsp), %zmm1
	vmovdqu64	592(%rsp), %zmm2
	movq	304(%rsp), %rax
	vmovdqu64	%zmm2, 96(%rax,%r12)
	vmovdqu64	%zmm1, 64(%rax,%r12)
	vmovdqu64	%zmm0, (%rax,%r12)
	leaq	1(%r14), %rax
	movq	%rax, 160(%rsp)
	testb	%r15b, %r15b
	jne	.LBB239_270
	addq	$160, %r12
	addq	$200, %rbp
	addq	$200, %rbx
	movq	%rax, %r15
	cmpq	8(%rsp), %rbx
	jne	.LBB239_260
	movq	8(%rsp), %rbp
	movq	56(%rsp), %r13
	movq	%rax, %r15
	jmp	.LBB239_269
.LBB239_267:
	movl	$8, %eax
	xorl	%r15d, %r15d
	movq	%rax, 304(%rsp)
	jmp	.LBB239_269
.LBB239_268:
	movq	56(%rsp), %r13
.LBB239_269:
	movq	16(%rsp), %r14
	movq	%rbp, 1192(%rsp)
	movl	$0, 260(%rsp)
	jmp	.LBB239_271
.LBB239_270:
	movq	%r14, %rcx
	movq	16(%rsp), %r14
	movq	56(%rsp), %r13
	incq	%rcx
	movb	$1, %al
	movq	%rbp, 1192(%rsp)
	movq	%rcx, %r15
	movl	%eax, 260(%rsp)
.LBB239_271:
.Ltmp7015:
	leaq	1184(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp7016:
	movq	304(%rsp), %rbx
	movq	40(%rsp), %rbp
	movq	144(%rsp), %r12
	testq	%r15, %r15
	je	.LBB239_275
	cmpq	$8, %r15
	jae	.LBB239_276
	xorl	%eax, %eax
	xorl	%edx, %edx
	jmp	.LBB239_294
.LBB239_275:
	xorl	%edx, %edx
	jmp	.LBB239_296
.LBB239_276:
	cmpq	$32, %r15
	jae	.LBB239_287
	xorl	%eax, %eax
	xorl	%edx, %edx
	jmp	.LBB239_291
.LBB239_278:
	movq	80(%rsp), %r15
.LBB239_279:
	movq	%r15, 792(%rsp)
.Ltmp7167:
	leaq	784(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7168:
	movq	1424(%rsp), %r14
	movq	1432(%rsp), %rbx
.Ltmp7175:
	leaq	496(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp7176:
	movq	$0, 48(%rsp)
	jmp	.LBB239_551
.LBB239_282:
	movq	104(%rsp), %r12
	movq	56(%rsp), %r15
.LBB239_283:
	movq	%r15, 424(%rsp)
.Ltmp6996:
	leaq	416(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp6997:
	movq	784(%rsp), %r14
	movq	792(%rsp), %rbx
	movq	$-1, %r15
	xorl	%r13d, %r13d
	movq	$0, 48(%rsp)
.LBB239_285:
.Ltmp7001:
	leaq	944(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp7002:
	cmpq	$-1, %r15
	jne	.LBB239_309
	jmp	.LBB239_551
.LBB239_287:
	vmovdqa64	.LCPI239_0(%rip), %zmm1
	vpbroadcastq	.LCPI239_1(%rip), %zmm2
	vpbroadcastq	.LCPI239_2(%rip), %zmm3
	movq	%r15, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
.LBB239_288:
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
	jne	.LBB239_288
	vpaddq	%zmm0, %zmm4, %zmm0
	vpaddq	%zmm0, %zmm5, %zmm0
	vpaddq	%zmm0, %zmm6, %zmm0
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rdx
	cmpq	%rax, %r15
	je	.LBB239_296
	testb	$24, %r15b
	je	.LBB239_294
.LBB239_291:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI239_0(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI239_1(%rip), %zmm2
	vpbroadcastq	.LCPI239_3(%rip), %zmm3
	movq	%r15, %rax
	andq	$-8, %rax
	vmovq	%rdx, %xmm0
	subq	%rax, %rcx
	.p2align	4
.LBB239_292:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	64(%rbx,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB239_292
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rdx
	cmpq	%rax, %r15
	je	.LBB239_296
.LBB239_294:
	movq	%r15, %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	64(%rax,%rbx), %rax
	.p2align	4
.LBB239_295:
	addq	(%rax), %rdx
	addq	$160, %rax
	decq	%rcx
	jne	.LBB239_295
.LBB239_296:
	movzbl	2521(%rsp), %eax
	movq	%r12, 1784(%rsp)
	movq	%rbx, 1792(%rsp)
	movq	$0, 1184(%rsp)
	movq	$8, 1192(%rsp)
	movq	%rdx, 1712(%rsp)
	movq	%r15, 1800(%rsp)
	movq	$0, 1200(%rsp)
	movb	%al, 119(%rsp)
	movzbl	2523(%rsp), %eax
	movq	%rax, 1704(%rsp)
	movl	260(%rsp), %eax
	movb	%al, 1808(%rsp)
.Ltmp7024:
	movq	80(%rsp), %rdx
	leaq	496(%rsp), %rdi
	leaq	1184(%rsp), %rsi
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp7025:
	movq	496(%rsp), %rsi
	movq	504(%rsp), %rax
	movq	512(%rsp), %rdx
	movq	520(%rsp), %rcx
	movq	%r12, 896(%rsp)
	movq	%rsi, 96(%rsp)
	cmpq	$-1, %rsi
	je	.LBB239_358
	movq	%rax, 56(%rsp)
	movzbl	528(%rsp), %eax
	vmovdqu	544(%rsp), %ymm0
	vmovdqu	560(%rsp), %ymm1
	movzbl	535(%rsp), %ebp
	movzwl	533(%rsp), %ebx
	movl	529(%rsp), %r13d
	movq	%rdx, 8(%rsp)
	movq	%rcx, 40(%rsp)
	movq	%rax, 48(%rsp)
	movq	536(%rsp), %rax
	vmovdqu	%ymm0, 784(%rsp)
	vmovdqu	%ymm1, 800(%rsp)
	movq	%rax, 80(%rsp)
.Ltmp7029:
	leaq	864(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7030:
	shll	$16, %ebp
	orl	%ebp, %ebx
	shlq	$32, %rbx
	orq	%rbx, %r13
.LBB239_300:
	movq	104(%rsp), %r12
	movq	304(%rsp), %rbp
.LBB239_301:
	testq	%r15, %r15
	je	.LBB239_305
	movl	$1, %ebx
	subq	%r15, %rbx
	movq	%rbp, %r15
	.p2align	4
.LBB239_303:
.Ltmp7131:
	movq	%r15, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7132:
	incq	%rbx
	addq	$160, %r15
	cmpq	$1, %rbx
	jne	.LBB239_303
.LBB239_305:
	movq	896(%rsp), %rax
	testq	%rax, %rax
	je	.LBB239_307
	shlq	$5, %rax
	movl	$8, %edx
	movq	%rbp, %rdi
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
.LBB239_307:
	movq	96(%rsp), %r15
	cmpq	$-1, %r15
	je	.LBB239_364
	vmovdqu	800(%rsp), %ymm1
	vmovdqu	784(%rsp), %ymm0
	movq	40(%rsp), %rbp
	movq	8(%rsp), %rbx
	movq	56(%rsp), %r14
	vmovdqu	%ymm1, 1440(%rsp)
	vmovdqu	%ymm0, 1424(%rsp)
.LBB239_309:
	vmovdqu	1440(%rsp), %ymm1
	vmovups	1424(%rsp), %ymm0
	vmovq	80(%rsp), %xmm2
	movzbl	48(%rsp), %eax
	shlq	$8, %r13
	orq	%r13, %rax
	vmovdqu	%ymm1, 80(%r12)
	vmovups	%ymm0, 64(%r12)
	vmovq	%rax, %xmm0
	vpunpcklqdq	%xmm2, %xmm0, %xmm0
.LBB239_310:
	movq	%r15, 16(%r12)
	movq	%r14, 24(%r12)
	movq	%rbx, 32(%r12)
	movq	%rbp, 40(%r12)
	vmovdqa	%xmm0, 48(%r12)
	movq	$1, (%r12)
.Ltmp7158:
	leaq	1728(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7159:
	movq	352(%rsp), %rax
	testq	%rax, %rax
	je	.LBB239_314
.LBB239_312:
	lock		decq	(%rax)
	jne	.LBB239_314
	#MEMBARRIER
.Ltmp7237:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	352(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp7238:
.LBB239_314:
	movb	$1, %al
	movl	%eax, 8(%rsp)
.LBB239_315:
.Ltmp7242:
	leaq	2112(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp7243:
	movq	128(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB239_318
	#MEMBARRIER
.Ltmp7247:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	128(%rsp), %rdi
	callq	*%rax
.Ltmp7248:
.LBB239_318:
.Ltmp7250:
	movl	8(%rsp), %r12d
	leaq	2328(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7251:
	movq	2024(%rsp), %rax
	movq	104(%rsp), %r15
	cmpq	$6, %rax
	jb	.LBB239_329
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
	jge	.LBB239_322
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB239_322:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB239_328
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB239_322
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
.LBB239_325:
	cmpq	%rax, %rdx
	jge	.LBB239_327
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB239_325
.LBB239_327:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB239_328:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB239_329:
	movq	1952(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB239_339
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
	jge	.LBB239_332
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB239_332:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB239_338
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB239_332
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
.LBB239_335:
	cmpq	%rax, %rdx
	jge	.LBB239_337
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB239_335
.LBB239_337:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB239_338:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB239_339:
	movq	2048(%rsp), %rax
	testq	%rax, %rax
	je	.LBB239_342
	lock		decq	(%rax)
	jne	.LBB239_342
	leaq	2048(%rsp), %rdi
	#MEMBARRIER
.Ltmp7253:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp7254:
.LBB239_342:
	movq	408(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB239_344
	#MEMBARRIER
.Ltmp7256:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	408(%rsp), %rdi
	callq	*%rax
.Ltmp7257:
.LBB239_344:
	testb	%r12b, %r12b
	je	.LBB239_626
	movq	392(%rsp), %r14
	movq	400(%rsp), %rbx
	testq	%rbx, %rbx
	je	.LBB239_615
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r12
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	xorl	%r15d, %r15d
	jmp	.LBB239_350
	.p2align	4
.LBB239_347:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB239_348:
	callq	*%rbp
.LBB239_349:
	incq	%r15
	cmpq	%rbx, %r15
	je	.LBB239_615
.LBB239_350:
	leaq	(%r15,%r15,4), %rcx
	movq	(%r14,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB239_349
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
	jge	.LBB239_353
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB239_353:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB239_348
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB239_353
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
.LBB239_356:
	cmpq	%rax, %rdx
	jge	.LBB239_347
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB239_356
	jmp	.LBB239_347
.LBB239_358:
	movq	%rax, 264(%rsp)
	movq	80(%rsp), %rax
	movq	%rbp, 832(%rsp)
	movq	%r13, 848(%rsp)
	movq	%rbp, 840(%rsp)
	movq	%rdx, 272(%rsp)
	movq	%rcx, 280(%rsp)
	movq	%r15, 904(%rsp)
	leaq	(%rax,%rax,4), %rax
	leaq	(%rbp,%rax,8), %rax
	movq	%rax, 856(%rsp)
	movq	616(%r14), %rax
	movq	%rax, 1416(%rsp)
	testq	%rax, %rax
	je	.LBB239_365
	lock		incq	(%rax)
	jle	.LBB239_721
	movq	616(%r14), %rax
	movq	%rax, 936(%rsp)
	movq	%rax, 136(%rsp)
	movq	16(%rax), %rcx
	movq	40(%rax), %rax
	movq	%rcx, 368(%rsp)
	movq	%rax, 296(%rsp)
	cmpq	$-1, %rax
	je	.LBB239_397
	testq	%r15, %r15
	je	.LBB239_379
	cmpq	$8, %r15
	jae	.LBB239_383
	xorl	%eax, %eax
	xorl	%esi, %esi
	jmp	.LBB239_394
.LBB239_364:
	movq	40(%rsp), %rbp
	movq	48(%rsp), %rcx
	jmp	.LBB239_549
.LBB239_365:
	movq	%rdx, 8(%rsp)
	movq	840(%rsp), %rdx
	movq	832(%rsp), %rax
	movq	848(%rsp), %rsi
	movq	%rdx, 152(%rsp)
	movq	856(%rsp), %rdx
	movq	%rax, 144(%rsp)
	movq	%rsi, 160(%rsp)
	movq	%rdx, 168(%rsp)
	movq	168(%rsp), %rax
	movq	152(%rsp), %r12
	movq	%rax, 88(%rsp)
	cmpq	%rax, %r12
	je	.LBB239_380
	leaq	(,%rcx,8), %rax
	movq	%rcx, %rbx
	leaq	(%rax,%rax,4), %r13
	jmp	.LBB239_369
.LBB239_367:
	movq	272(%rsp), %rax
	movq	%rax, 8(%rsp)
.LBB239_368:
	movq	8(%rsp), %rcx
	movq	80(%rsp), %rax
	shll	$16, %ebp
	movq	96(%rsp), %rdx
	addq	$40, %r12
	orl	%ebp, %ebx
	shlq	$32, %rbx
	orq	%rbx, %r14
	movq	40(%rsp), %rbx
	movq	%rax, (%rcx,%r13)
	movq	56(%rsp), %rax
	incq	%rbx
	movq	%rax, 8(%rcx,%r13)
	movzbl	72(%rsp), %eax
	movq	%r15, 16(%rcx,%r13)
	movb	%al, 24(%rcx,%r13)
	movq	%r14, %rax
	movl	%r14d, 25(%rcx,%r13)
	shrq	$32, %r14
	shrq	$48, %rax
	movw	%r14w, 29(%rcx,%r13)
	movq	16(%rsp), %r14
	movb	%al, 31(%rcx,%r13)
	movq	%rdx, 32(%rcx,%r13)
	addq	$40, %r13
	movq	%rbx, 280(%rsp)
	cmpq	88(%rsp), %r12
	je	.LBB239_381
.LBB239_369:
	movq	32(%r12), %rax
	leaq	1192(%rsp), %rcx
	movq	%rax, 32(%rcx)
	vmovdqu	(%r12), %ymm0
	vmovdqu	%ymm0, (%rcx)
	movq	%r14, 1184(%rsp)
	cmpq	$0, 1192(%rsp)
	je	.LBB239_371
	movq	32(%r12), %rax
	leaq	504(%rsp), %rcx
	movq	%rax, 32(%rcx)
	vmovdqu	(%r12), %ymm0
	vmovdqu	%ymm0, (%rcx)
	jmp	.LBB239_373
.LBB239_371:
	movq	664(%r14), %rdx
.Ltmp7110:
	movq	48(%rsp), %rsi
	leaq	496(%rsp), %rdi
	leaq	1200(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7111:
	movq	496(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB239_385
.LBB239_373:
	movq	512(%rsp), %rcx
	movq	%rbx, %rdx
	movq	504(%rsp), %rax
	movzbl	528(%rsp), %esi
	movq	520(%rsp), %r15
	movzbl	535(%rsp), %ebp
	movzwl	533(%rsp), %ebx
	movl	529(%rsp), %r14d
	movq	%rdx, 40(%rsp)
	movq	%rcx, 56(%rsp)
	movq	536(%rsp), %rcx
	movq	%rax, 80(%rsp)
	movb	%sil, 72(%rsp)
	movq	%rcx, 96(%rsp)
	cmpq	264(%rsp), %rdx
	jne	.LBB239_368
.Ltmp7118:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	264(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp7119:
	jmp	.LBB239_367
.LBB239_375:
	vmovdqa	976(%rsp), %xmm0
	vmovdqu	992(%rsp), %ymm2
	vmovdqu	1008(%rsp), %ymm1
	movq	40(%rsp), %rax
	movq	952(%rsp), %r14
	movq	960(%rsp), %rbx
	movq	968(%rsp), %rbp
	movq	%rax, 792(%rsp)
	vmovdqu	%ymm2, 144(%rsp)
	vmovdqa	%xmm0, 16(%rsp)
	vmovdqu	%ymm1, 160(%rsp)
.Ltmp7154:
	leaq	784(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7155:
	movq	104(%rsp), %r12
	leaq	1424(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB239_142
.LBB239_377:
	movq	40(%rsp), %rax
	vmovdqu	544(%rsp), %ymm0
	vmovdqu	560(%rsp), %ymm1
	movq	504(%rsp), %r14
	movq	512(%rsp), %rbx
	movq	520(%rsp), %rbp
	movq	528(%rsp), %r13
	movq	%rax, 424(%rsp)
	movq	536(%rsp), %rax
	vmovdqu	%ymm0, 144(%rsp)
	vmovdqu	%ymm1, 160(%rsp)
	movq	%rax, 80(%rsp)
.Ltmp6988:
	leaq	416(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp6989:
	movq	104(%rsp), %r12
	leaq	784(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB239_147
.LBB239_379:
	xorl	%esi, %esi
	jmp	.LBB239_396
.LBB239_380:
	movq	%rcx, %rbx
.LBB239_381:
	movb	$1, %al
	movq	%rbx, 40(%rsp)
	movq	%r12, 152(%rsp)
	movl	%eax, 88(%rsp)
.Ltmp7123:
	leaq	144(%rsp), %rdi
	movb	$1, %r12b
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7124:
	movq	904(%rsp), %r15
	movq	264(%rsp), %rax
	movq	272(%rsp), %rcx
	movq	$-1, 96(%rsp)
	movq	%rax, 56(%rsp)
	movb	$2, %al
	movq	%rcx, 8(%rsp)
	movq	%rax, 48(%rsp)
	jmp	.LBB239_300
.LBB239_383:
	cmpq	$32, %r15
	jae	.LBB239_387
	xorl	%eax, %eax
	xorl	%esi, %esi
	jmp	.LBB239_391
.LBB239_385:
	vmovups	544(%rsp), %ymm0
	movq	%rax, 96(%rsp)
	movzbl	528(%rsp), %eax
	movq	504(%rsp), %rcx
	movq	512(%rsp), %rdx
	movzbl	535(%rsp), %ebp
	movzwl	533(%rsp), %ebx
	movl	529(%rsp), %r13d
	addq	$40, %r12
	movq	%r12, 152(%rsp)
	movq	%rax, 48(%rsp)
	movq	536(%rsp), %rax
	movq	%rcx, 56(%rsp)
	movq	520(%rsp), %rcx
	movq	%rdx, 8(%rsp)
	vmovups	%ymm0, 784(%rsp)
	vmovdqu	560(%rsp), %ymm0
	movq	%rax, 80(%rsp)
	movb	$1, %al
	movq	%rcx, 40(%rsp)
	movl	%eax, 88(%rsp)
	vmovdqu	%ymm0, 800(%rsp)
.Ltmp7113:
	leaq	144(%rsp), %rdi
	movb	$1, %r12b
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7114:
	movq	904(%rsp), %r15
	shll	$16, %ebp
	movb	$1, %al
	orl	%ebp, %ebx
	movl	%eax, 72(%rsp)
	shlq	$32, %rbx
	orq	%rbx, %r13
	movb	$1, %bl
	jmp	.LBB239_707
.LBB239_387:
	vmovdqa64	.LCPI239_0(%rip), %zmm1
	vpbroadcastq	.LCPI239_1(%rip), %zmm2
	vpbroadcastq	.LCPI239_2(%rip), %zmm3
	movq	%r15, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
.LBB239_388:
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
	jne	.LBB239_388
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
	cmpq	%rax, %r15
	je	.LBB239_396
	testb	$24, %r15b
	je	.LBB239_394
.LBB239_391:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI239_0(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI239_1(%rip), %zmm2
	vpbroadcastq	.LCPI239_3(%rip), %zmm3
	movq	%r15, %rax
	andq	$-8, %rax
	vmovq	%rsi, %xmm0
	subq	%rax, %rcx
.LBB239_392:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	16(%rbx,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB239_392
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rsi
	cmpq	%rax, %r15
	je	.LBB239_396
.LBB239_394:
	movq	%r15, %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	16(%rax,%rbx), %rax
.LBB239_395:
	addq	(%rax), %rsi
	addq	$160, %rax
	decq	%rcx
	jne	.LBB239_395
.LBB239_396:
	movb	$1, %al
	movl	%eax, 88(%rsp)
.Ltmp7032:
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts@GOTPCREL(%rip), %rax
	movq	48(%rsp), %rdi
	movb	$1, %cl
	movl	%ecx, 72(%rsp)
	vzeroupper
	callq	*%rax
.Ltmp7033:
.LBB239_397:
	movq	136(%rsp), %rax
	leaq	(%r15,%r15,4), %rcx
	movq	%rbx, 1376(%rsp)
	movq	%rbx, 1384(%rsp)
	movq	%r12, 1392(%rsp)
	shlq	$5, %rcx
	addq	%rbx, %rcx
	movq	%rcx, 1680(%rsp)
	movq	%rcx, 1400(%rsp)
	addq	$16, %rax
	movq	%rax, 288(%rsp)
	testq	%r15, %r15
	je	.LBB239_542
	leaq	504(%rsp), %r12
.LBB239_399:
	movq	%rbx, %rcx
	addq	$160, %rbx
	movq	%rbx, 1384(%rsp)
	vmovups	96(%rcx), %zmm0
	movq	(%rcx), %rax
	vmovups	%zmm0, 1272(%rsp)
	vmovups	72(%rcx), %zmm0
	vmovups	%zmm0, 1248(%rsp)
	vmovups	8(%rcx), %zmm0
	vmovups	%zmm0, 1184(%rsp)
	cmpq	$-1, %rax
	je	.LBB239_542
	vmovdqu64	1184(%rsp), %zmm0
	vmovdqu64	1248(%rsp), %zmm1
	vmovdqu64	1272(%rsp), %zmm2
	movq	%rax, 496(%rsp)
	vmovdqu64	%zmm2, 88(%r12)
	vmovdqu64	%zmm1, 64(%r12)
	vmovdqu64	%zmm0, (%r12)
	imulq	$88, 512(%rsp), %rsi
	movq	504(%rsp), %rdx
	movq	536(%rsp), %rdi
	movq	520(%rsp), %r12
	movq	528(%rsp), %rcx
	movq	%rdi, 888(%rsp)
	movq	%rdx, 320(%rsp)
	movq	%rax, 336(%rsp)
	movq	%rdx, 328(%rsp)
	movq	552(%rsp), %rdi
	movq	560(%rsp), %rax
	movq	%rcx, 248(%rsp)
	movq	%rdx, 8(%rsp)
	movq	%r12, 64(%rsp)
	addq	%rdx, %rsi
	movq	%rsi, 120(%rsp)
	movq	%rsi, 344(%rsp)
	movq	544(%rsp), %rsi
	testq	%rax, %rax
	je	.LBB239_530
	movq	592(%rsp), %rcx
	shlq	$5, %rax
	movq	$0, 360(%rsp)
	movq	%rsi, 376(%rsp)
	movq	%rdi, 760(%rsp)
	movq	%rbx, 912(%rsp)
	addq	%rdi, %rax
	movq	%rax, 1688(%rsp)
	movq	%rdi, %rax
	movq	%rcx, 40(%rsp)
	jmp	.LBB239_404
.LBB239_402:
	movq	376(%rsp), %rsi
	movq	760(%rsp), %rdi
	movq	912(%rsp), %rbx
.LBB239_403:
	movq	1696(%rsp), %rax
	movq	%rbp, 840(%rsp)
	addq	$32, %rax
	cmpq	1688(%rsp), %rax
	je	.LBB239_530
.LBB239_404:
	movq	360(%rsp), %rsi
	movq	16(%rax), %rcx
	movq	(%rax), %r12
	movq	8(%rax), %rbx
	movq	%rax, 1696(%rsp)
	movq	24(%rax), %rax
	movq	%rcx, 72(%rsp)
	movq	%rax, 80(%rsp)
	movq	%rsi, 96(%rsp)
	testq	%r12, %r12
	je	.LBB239_411
	cmpq	$-1, 368(%rsp)
	je	.LBB239_411
	movq	%r12, 416(%rsp)
	movq	$0, 424(%rsp)
.Ltmp7034:
	movq	288(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	leaq	144(%rsp), %rdi
	leaq	416(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp7035:
	cmpb	$-1, 160(%rsp)
	jne	.LBB239_687
	movq	632(%r14), %rax
	movq	96(%rsp), %rsi
	testq	%rax, %rax
	je	.LBB239_411
	movl	1228(%r14), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB239_411
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	movq	1704(%rsp), %rax
	lock		addq	%r12, (%rcx,%rax,8)
.LBB239_411:
.Ltmp7036:
	movq	888(%rsp), %rdx
	movq	%rbx, %rdi
	vzeroupper
	callq	<usize as core::cmp::Ord>::clamp
.Ltmp7037:
	movq	96(%rsp), %rdi
	movq	%rax, %rsi
	cmpq	%rdi, %rax
	jb	.LBB239_714
	cmpq	888(%rsp), %rsi
	ja	.LBB239_714
	movq	248(%rsp), %rcx
	leaq	(%rdi,%rdi,2), %rax
	leaq	416(%rsp), %rdi
	leaq	144(%rsp), %r12
	vpxor	%xmm0, %xmm0, %xmm0
	movq	%rsi, 360(%rsp)
	vmovdqa	%xmm0, 144(%rsp)
	movq	$0, 160(%rsp)
	leaq	(%rcx,%rax,8), %r13
	leaq	(%rsi,%rsi,2), %rax
	leaq	(%rcx,%rax,8), %rdx
	movq	%r13, %rsi
	movq	%r12, %rcx
	movq	%rdx, 56(%rsp)
	callq	<core::slice::iter::Iter<purrdf_sparql_eval::row_checkpoint::Deferred> as core::iter::traits::iterator::Iterator>::fold::<(u64, u64, u64), purrdf_sparql_eval::row_checkpoint::commit_items<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>::{closure#2}>
	movq	424(%rsp), %rax
	movq	432(%rsp), %rcx
	movq	416(%rsp), %rbx
	cmpq	$-1, 296(%rsp)
	movq	%rax, 88(%rsp)
	movq	%rcx, 768(%rsp)
	je	.LBB239_416
	movq	72(%rsp), %rax
	movq	8(%rsp), %rsi
	movq	120(%rsp), %rdx
	movl	$0, %ecx
	subq	40(%rsp), %rax
	movq	%rsi, 144(%rsp)
	movq	%rdx, 152(%rsp)
	cmovaeq	%rax, %rcx
	movq	%rcx, 160(%rsp)
.Ltmp7038:
	movq	%r12, %rdi
	callq	<core::iter::adapters::take::Take<core::slice::iter::Iter<(u64, purrdf_core::ir::term::TermValue)>> as core::iter::adapters::take::SpecTake>::spec_fold::<u64, purrdf_sparql_eval::row_checkpoint::commit_items<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>::{closure#3}>
.Ltmp7039:
	jmp	.LBB239_417
.LBB239_416:
	xorl	%eax, %eax
.LBB239_417:
	movq	136(%rsp), %rcx
	movl	296(%rcx), %ecx
	movq	64(%rsp), %r12
	movq	96(%rsp), %r10
	testl	%ecx, %ecx
	je	.LBB239_431
.LBB239_418:
	cmpq	$-1, 368(%rsp)
	je	.LBB239_420
	movq	136(%rsp), %rdx
	movq	$-1, %rsi
	movq	80(%rdx), %rcx
	addq	%rbx, %rcx
	cmovbq	%rsi, %rcx
	cmpq	16(%rdx), %rcx
	ja	.LBB239_432
.LBB239_420:
	cmpq	$-1, 296(%rsp)
	je	.LBB239_422
	movq	1048(%r14), %rcx
	movq	1056(%r14), %rdx
	movq	$-1, %rsi
	subq	%rdx, %rcx
	movl	$0, %edx
	cmovaeq	%rcx, %rdx
	movq	136(%rsp), %rcx
	addq	%rax, %rdx
	cmovbq	%rsi, %rdx
	addq	88(%rsp), %rdx
	cmovbq	%rsi, %rdx
	addq	768(%rsp), %rdx
	movq	104(%rcx), %rax
	cmovbq	%rsi, %rdx
	addq	%rdx, %rax
	cmovbq	%rsi, %rax
	cmpq	40(%rcx), %rax
	ja	.LBB239_432
.LBB239_422:
	cmpq	$-1, 368(%rsp)
	je	.LBB239_473
	cmpq	360(%rsp), %r10
	je	.LBB239_473
	movq	360(%rsp), %rcx
	leaq	(,%r10,8), %rax
	leaq	(%rax,%rax,2), %rax
	leaq	(,%rcx,8), %rcx
	leaq	(%rcx,%rcx,2), %rcx
	jmp	.LBB239_426
.LBB239_425:
	addq	$24, %rax
	cmpq	%rax, %rcx
	je	.LBB239_473
.LBB239_426:
	movq	248(%rsp), %rdx
	cmpb	$0, (%rdx,%rax)
	jne	.LBB239_425
	movq	248(%rsp), %rdx
	movzbl	1(%rdx,%rax), %edx
	cmpq	$255, %rdx
	je	.LBB239_425
	movq	632(%r14), %rsi
	testq	%rsi, %rsi
	je	.LBB239_425
	movl	1228(%r14), %edi
	cmpq	%rdi, 56(%rsi)
	jbe	.LBB239_425
	movq	248(%rsp), %r8
	movq	%rdi, %r9
	shlq	$7, %r9
	leaq	(%r9,%rdi,8), %rdi
	addq	48(%rsi), %rdi
	movq	8(%r8,%rax), %r8
	lock		addq	%r8, (%rdi,%rdx,8)
	jmp	.LBB239_425
.LBB239_431:
	movq	136(%rsp), %rcx
	cmpb	$-1, 272(%rcx)
	je	.LBB239_418
.LBB239_432:
	cmpq	360(%rsp), %r10
	jne	.LBB239_443
.LBB239_433:
	cmpq	$-1, 296(%rsp)
	je	.LBB239_520
	movq	72(%rsp), %rax
	cmpq	%rax, 40(%rsp)
	jae	.LBB239_520
	movq	8(%rsp), %rdx
	cmpq	120(%rsp), %rdx
	je	.LBB239_485
	movq	72(%rsp), %rax
	addq	$88, %rdx
	leaq	-1(%rax), %rbx
	movq	%rdx, %rax
.LBB239_437:
	movq	%rax, %rdx
	movq	-8(%rdx), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rdx, 8(%rsp)
	movq	%rcx, 480(%rsp)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, 416(%rsp)
	cmpq	$-1, %rax
	je	.LBB239_484
	vmovdqu64	416(%rsp), %zmm0
	movq	%rax, 144(%rsp)
	movq	480(%rsp), %rax
	leaq	152(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7073:
	movq	48(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	144(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp7074:
	movq	40(%rsp), %rax
	cmpq	%rax, %rbx
	je	.LBB239_486
	movq	8(%rsp), %rdx
	incq	%rax
	movq	%rax, 40(%rsp)
	leaq	-88(%rdx), %rcx
	leaq	88(%rdx), %rax
	addq	$88, %rcx
	cmpq	120(%rsp), %rcx
	jne	.LBB239_437
	jmp	.LBB239_485
.LBB239_441:
	movq	64(%rsp), %r12
.LBB239_442:
	addq	$24, %r13
	cmpq	56(%rsp), %r13
	je	.LBB239_433
.LBB239_443:
	movzbl	(%r13), %eax
	leaq	.LJTI239_0(%rip), %rcx
	movslq	(%rcx,%rax,4), %rax
	addq	%rcx, %rax
	jmpq	*%rax
.LBB239_444:
	cmpq	$-1, 368(%rsp)
	je	.LBB239_442
	movzbl	1(%r13), %ebx
	movq	8(%r13), %r12
	movq	16(%r13), %r14
	movq	%r12, 416(%rsp)
	movq	$0, 424(%rsp)
.Ltmp7067:
	movq	288(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	leaq	144(%rsp), %rdi
	leaq	416(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp7068:
	movzbl	160(%rsp), %ecx
	cmpb	$-1, %cl
	setne	%al
	testq	%r14, %r14
	setne	%dl
	testb	%al, %dl
	jne	.LBB239_672
	cmpb	$-1, %cl
	setne	%cl
	cmpl	$255, %ebx
	sete	%dl
	orb	%cl, %dl
	jne	.LBB239_464
	movq	16(%rsp), %r14
	movq	632(%r14), %rax
	testq	%rax, %rax
	je	.LBB239_441
	movl	1228(%r14), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB239_441
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	lock		addq	%r12, (%rcx,%rbx,8)
	jmp	.LBB239_441
.LBB239_451:
	movq	8(%r13), %rbx
	cmpq	%rbx, 40(%rsp)
	jae	.LBB239_468
	movq	8(%rsp), %rdx
	cmpq	120(%rsp), %rdx
	je	.LBB239_466
	addq	$88, %rdx
	leaq	-1(%rbx), %r12
	movq	%rdx, %rax
.LBB239_454:
	movq	%rax, %rdx
	movq	-8(%rdx), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rdx, 8(%rsp)
	movq	%rcx, 480(%rsp)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, 416(%rsp)
	cmpq	$-1, %rax
	je	.LBB239_465
	vmovdqu64	416(%rsp), %zmm0
	movq	%rax, 144(%rsp)
	movq	480(%rsp), %rax
	leaq	152(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7057:
	movq	48(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	144(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp7058:
	movq	40(%rsp), %rsi
	cmpq	%rsi, %r12
	je	.LBB239_467
	movq	8(%rsp), %rdx
	incq	%rsi
	movq	%rsi, 40(%rsp)
	leaq	-88(%rdx), %rcx
	leaq	88(%rdx), %rax
	addq	$88, %rcx
	cmpq	120(%rsp), %rcx
	jne	.LBB239_454
	jmp	.LBB239_466
.LBB239_458:
	cmpq	$-1, 296(%rsp)
	je	.LBB239_442
	movq	8(%r13), %rcx
.Ltmp7055:
	movq	288(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::admit_transient@GOTPCREL(%rip), %rax
	leaq	144(%rsp), %rdi
	movl	$3, %edx
	vzeroupper
	callq	*%rax
.Ltmp7056:
	jmp	.LBB239_471
.LBB239_460:
	cmpq	$-1, 296(%rsp)
	je	.LBB239_442
	movq	136(%rsp), %rax
	cmpq	$-1, 40(%rax)
	je	.LBB239_442
.Ltmp7062:
	movq	8(%r13), %rcx
	movq	288(%rsp), %rsi
	movq	16(%r13), %rbx
	leaq	144(%rsp), %rdi
	movl	$3, %edx
	xorl	%r8d, %r8d
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.4261325137610144415)
.Ltmp7063:
	cmpb	$-1, 144(%rsp)
	setne	%al
	sete	%cl
	testq	%rbx, %rbx
	sete	%dl
	orb	%cl, %dl
	je	.LBB239_680
.LBB239_464:
	testb	%al, %al
	jmp	.LBB239_472
.LBB239_465:
	movq	8(%rsp), %rdx
.LBB239_466:
	movq	%rdx, 8(%rsp)
	movq	%rdx, 328(%rsp)
	jmp	.LBB239_469
.LBB239_467:
	movq	%rbx, 40(%rsp)
.LBB239_468:
	movq	8(%rsp), %rax
	movq	%rax, 328(%rsp)
.LBB239_469:
	cmpq	$-1, 296(%rsp)
	leaq	144(%rsp), %rdi
	je	.LBB239_441
.Ltmp7060:
	movq	%r14, %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp7061:
.LBB239_471:
	cmpb	$-1, 144(%rsp)
.LBB239_472:
	movq	16(%rsp), %r14
	movq	64(%rsp), %r12
	je	.LBB239_442
	jmp	.LBB239_691
.LBB239_473:
	movq	360(%rsp), %rax
	movb	$1, %r13b
	cmpq	%rax, %r10
	je	.LBB239_491
	shlq	$3, %rax
	shlq	$3, %r10
	leaq	(%rax,%rax,2), %rax
	leaq	(%r10,%r10,2), %rcx
.LBB239_475:
	movq	248(%rsp), %rdx
	cmpb	$2, -24(%rdx,%rax)
	je	.LBB239_477
	addq	$-24, %rax
	cmpq	%rax, %rcx
	jne	.LBB239_475
	jmp	.LBB239_491
.LBB239_477:
	movq	248(%rsp), %rcx
	movq	-16(%rcx,%rax), %r13
	cmpq	%r13, 40(%rsp)
	jae	.LBB239_490
	movq	8(%rsp), %rdx
	cmpq	120(%rsp), %rdx
	je	.LBB239_488
	addq	$88, %rdx
	leaq	-1(%r13), %r12
	movq	%rdx, %rax
.LBB239_480:
	movq	%rax, %rdx
	movq	-8(%rdx), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rdx, 8(%rsp)
	movq	%rcx, 480(%rsp)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, 416(%rsp)
	cmpq	$-1, %rax
	je	.LBB239_487
	vmovdqu64	416(%rsp), %zmm0
	movq	%rax, 144(%rsp)
	movq	480(%rsp), %rax
	leaq	152(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7040:
	movq	48(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	144(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp7041:
	movq	40(%rsp), %rsi
	cmpq	%rsi, %r12
	je	.LBB239_489
	movq	8(%rsp), %rdx
	incq	%rsi
	movq	%rsi, 40(%rsp)
	leaq	-88(%rdx), %rcx
	leaq	88(%rdx), %rax
	addq	$88, %rcx
	cmpq	120(%rsp), %rcx
	jne	.LBB239_480
	jmp	.LBB239_488
.LBB239_484:
	movq	8(%rsp), %rdx
.LBB239_485:
	movq	%rdx, 8(%rsp)
	movq	%rdx, 328(%rsp)
	jmp	.LBB239_520
.LBB239_486:
	movq	8(%rsp), %rax
	movq	72(%rsp), %rcx
	movq	%rax, 328(%rsp)
	movq	%rcx, 40(%rsp)
	jmp	.LBB239_520
.LBB239_487:
	movq	8(%rsp), %rdx
.LBB239_488:
	movq	64(%rsp), %r12
	movq	%rdx, 8(%rsp)
	movq	%rdx, 328(%rsp)
	xorl	%r13d, %r13d
	jmp	.LBB239_491
.LBB239_489:
	movq	64(%rsp), %r12
	movq	%r13, 40(%rsp)
.LBB239_490:
	movq	8(%rsp), %rax
	xorl	%r13d, %r13d
	movq	%rax, 328(%rsp)
.LBB239_491:
	cmpq	$-1, 368(%rsp)
	je	.LBB239_496
	testq	%rbx, %rbx
	je	.LBB239_496
	movq	%rbx, 416(%rsp)
	movq	$0, 424(%rsp)
.Ltmp7043:
	movq	288(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	leaq	144(%rsp), %rdi
	leaq	416(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp7044:
	cmpb	$-1, 160(%rsp)
	movq	64(%rsp), %r12
	je	.LBB239_496
	cmpq	$-1, 296(%rsp)
	movb	$1, %bl
	jne	.LBB239_508
	jmp	.LBB239_691
.LBB239_496:
	cmpq	$-1, 296(%rsp)
	je	.LBB239_520
	movq	136(%rsp), %rax
	cmpq	$-1, 40(%rax)
	je	.LBB239_501
.Ltmp7045:
	movq	288(%rsp), %rsi
	movq	88(%rsp), %rcx
	leaq	144(%rsp), %rdi
	movl	$3, %edx
	xorl	%r8d, %r8d
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.4261325137610144415)
.Ltmp7046:
	cmpb	$-1, 144(%rsp)
	movq	64(%rsp), %r12
	je	.LBB239_501
	movb	$1, %bl
	jmp	.LBB239_508
.LBB239_501:
	testb	%r13b, %r13b
	jne	.LBB239_504
.Ltmp7047:
	leaq	144(%rsp), %rdi
	movq	%r14, %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp7048:
	cmpb	$-1, 144(%rsp)
	movq	64(%rsp), %r12
	movb	$1, %bl
	jne	.LBB239_508
.LBB239_504:
	movq	768(%rsp), %rcx
	testq	%rcx, %rcx
	je	.LBB239_507
.Ltmp7049:
	movq	288(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::admit_transient@GOTPCREL(%rip), %rax
	leaq	144(%rsp), %rdi
	movl	$3, %edx
	vzeroupper
	callq	*%rax
.Ltmp7050:
	cmpb	$-1, 144(%rsp)
	movq	64(%rsp), %r12
	setne	%bl
	jmp	.LBB239_508
.LBB239_507:
	xorl	%ebx, %ebx
.LBB239_508:
	movq	72(%rsp), %rax
	cmpq	%rax, 40(%rsp)
	jae	.LBB239_518
	movq	8(%rsp), %rdx
	cmpq	120(%rsp), %rdx
	je	.LBB239_516
	movq	72(%rsp), %rax
	addq	$88, %rdx
	leaq	-1(%rax), %r12
	movq	%rdx, %rax
.LBB239_511:
	movq	%rax, %rdx
	movq	-8(%rdx), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rdx, 8(%rsp)
	movq	%rcx, 480(%rsp)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, 416(%rsp)
	cmpq	$-1, %rax
	je	.LBB239_515
	vmovdqu64	416(%rsp), %zmm0
	movq	%rax, 144(%rsp)
	movq	480(%rsp), %rax
	leaq	152(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7052:
	movq	48(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	144(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp7053:
	movq	40(%rsp), %rsi
	cmpq	%rsi, %r12
	je	.LBB239_517
	movq	8(%rsp), %rdx
	incq	%rsi
	movq	%rsi, 40(%rsp)
	leaq	-88(%rdx), %rcx
	leaq	88(%rdx), %rax
	addq	$88, %rcx
	cmpq	120(%rsp), %rcx
	jne	.LBB239_511
	jmp	.LBB239_516
.LBB239_515:
	movq	8(%rsp), %rdx
.LBB239_516:
	movq	64(%rsp), %r12
	movq	%rdx, 8(%rsp)
	movq	%rdx, 328(%rsp)
	jmp	.LBB239_519
.LBB239_517:
	movq	72(%rsp), %rax
	movq	64(%rsp), %r12
	movq	%rax, 40(%rsp)
.LBB239_518:
	movq	8(%rsp), %rax
	movq	%rax, 328(%rsp)
.LBB239_519:
	testb	%bl, %bl
	jne	.LBB239_691
.LBB239_520:
	cmpq	$0, 80(%rsp)
	je	.LBB239_402
	movq	856(%rsp), %rax
	movq	376(%rsp), %rsi
	movq	760(%rsp), %rdi
	movq	912(%rsp), %rbx
	movq	%rax, 768(%rsp)
	jmp	.LBB239_523
.LBB239_522:
	movq	272(%rsp), %rax
	movq	56(%rsp), %rdx
	leaq	(%rbp,%rbp,4), %rcx
	movq	80(%rsp), %rsi
	shll	$16, %r13d
	movq	72(%rsp), %rdi
	addq	$40, %rbx
	incq	%rbp
	orl	%r13d, %r12d
	shlq	$32, %r12
	orq	%r12, %r14
	movq	64(%rsp), %r12
	movq	%rdx, (%rax,%rcx,8)
	movq	96(%rsp), %rdx
	decq	%rsi
	movq	%rsi, 80(%rsp)
	movq	%rdx, 8(%rax,%rcx,8)
	movzbl	88(%rsp), %edx
	movq	%r15, 16(%rax,%rcx,8)
	movq	904(%rsp), %r15
	movb	%dl, 24(%rax,%rcx,8)
	movq	%r14, %rdx
	shrq	$48, %rdx
	movl	%r14d, 25(%rax,%rcx,8)
	shrq	$32, %r14
	movw	%r14w, 29(%rax,%rcx,8)
	movb	%dl, 31(%rax,%rcx,8)
	movq	%rdi, 32(%rax,%rcx,8)
	movq	%rbp, 280(%rsp)
	movq	%rbx, %rbp
	movq	16(%rsp), %r14
	movq	760(%rsp), %rdi
	movq	912(%rsp), %rbx
	testq	%rsi, %rsi
	movq	376(%rsp), %rsi
	je	.LBB239_403
.LBB239_523:
	cmpq	768(%rsp), %rbp
	je	.LBB239_403
	movq	32(%rbp), %rax
	leaq	424(%rsp), %rcx
	movq	%rax, 32(%rcx)
	vmovdqu	(%rbp), %ymm0
	vmovdqu	%ymm0, (%rcx)
	movq	%r14, 416(%rsp)
	cmpq	$0, 424(%rsp)
	je	.LBB239_526
	movq	32(%rbp), %rax
	leaq	152(%rsp), %rcx
	movq	%rbp, %rbx
	movq	%rax, 32(%rcx)
	vmovdqu	(%rbp), %ymm0
	vmovdqu	%ymm0, (%rcx)
	jmp	.LBB239_528
.LBB239_526:
	movq	664(%r14), %rdx
	movq	%rbp, %rbx
.Ltmp7076:
	movq	48(%rsp), %rsi
	leaq	144(%rsp), %rdi
	leaq	432(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7077:
	movq	144(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB239_670
.LBB239_528:
	movq	152(%rsp), %rax
	movq	160(%rsp), %rsi
	movzbl	176(%rsp), %edx
	movq	184(%rsp), %rcx
	movq	168(%rsp), %r15
	movzbl	183(%rsp), %r13d
	movzwl	181(%rsp), %r12d
	movl	177(%rsp), %r14d
	movq	280(%rsp), %rbp
	movq	%rax, 56(%rsp)
	movq	%rsi, 96(%rsp)
	movb	%dl, 88(%rsp)
	movq	%rcx, 72(%rsp)
	cmpq	264(%rsp), %rbp
	jne	.LBB239_522
.Ltmp7086:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	264(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp7087:
	jmp	.LBB239_522
.LBB239_530:
	testq	%rsi, %rsi
	je	.LBB239_532
	shlq	$5, %rsi
	movl	$8, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB239_532:
.Ltmp7094:
	leaq	320(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp7095:
	testq	%r12, %r12
	je	.LBB239_535
	movq	248(%rsp), %rdi
	shlq	$3, %r12
	movl	$8, %edx
	leaq	(%r12,%r12,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB239_535:
	movq	584(%rsp), %rax
	testq	%rax, %rax
	je	.LBB239_538
	lock		decq	(%rax)
	jne	.LBB239_538
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	584(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB239_538:
	movq	616(%rsp), %rax
	leaq	504(%rsp), %r12
	testq	%rax, %rax
	je	.LBB239_541
	lock		decq	(%rax)
	jne	.LBB239_541
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	616(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB239_541:
	cmpq	1680(%rsp), %rbx
	jne	.LBB239_399
.LBB239_542:
	movb	$1, %al
	movl	$0, 72(%rsp)
	movl	%eax, 88(%rsp)
.Ltmp7099:
	leaq	1376(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp7100:
	cmpq	$-1, 368(%rsp)
	movzbl	119(%rsp), %ecx
	sete	%al
	xorb	$1, %cl
	orb	260(%rsp), %cl
	orb	%al, %cl
	cmpb	$1, %cl
	je	.LBB239_545
	movq	$1, 1184(%rsp)
	movq	$0, 1192(%rsp)
	movl	$0, 72(%rsp)
.Ltmp7101:
	movq	<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items@GOTPCREL(%rip), %rax
	movq	288(%rsp), %rsi
	leaq	496(%rsp), %rdi
	leaq	1184(%rsp), %rdx
	movl	$1, %ecx
	callq	*%rax
.Ltmp7102:
.LBB239_545:
	movq	264(%rsp), %rax
	movq	272(%rsp), %rdx
	movq	136(%rsp), %rcx
	movq	280(%rsp), %rbp
	movq	%rax, 56(%rsp)
	movq	%rdx, 8(%rsp)
	lock		decq	(%rcx)
	jne	.LBB239_547
	#MEMBARRIER
	movl	$0, 88(%rsp)
.Ltmp7106:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	936(%rsp), %rdi
	xorl	%r12d, %r12d
	callq	*%rax
.Ltmp7107:
.LBB239_547:
	xorl	%ebx, %ebx
.Ltmp7108:
	leaq	832(%rsp), %rdi
	movl	$0, 72(%rsp)
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7109:
	movb	$2, %cl
.LBB239_549:
	movq	56(%rsp), %rax
	movq	8(%rsp), %r8
	movq	%rax, 496(%rsp)
	movq	%r8, 504(%rsp)
	movq	%rbp, 512(%rsp)
.Ltmp7137:
	movzbl	%cl, %esi
	movzbl	260(%rsp), %edx
	movq	1712(%rsp), %rcx
	movq	%r14, %rdi
	callq	purrdf_sparql_eval::row_checkpoint::settle_commit::<purrdf_core::ir::dataset::RdfDataset>
	movq	%rax, 48(%rsp)
	movq	%rdx, 80(%rsp)
.Ltmp7138:
	movq	8(%rsp), %rbx
	movq	56(%rsp), %r14
.LBB239_551:
	movq	1728(%rsp), %rcx
	movq	1744(%rsp), %rdx
	movq	1736(%rsp), %rax
	movq	1752(%rsp), %rsi
	movl	$1, %r13d
	movl	$1, %edi
	movq	%r14, 144(%rsp)
	movq	%rbx, 152(%rsp)
	movq	%rbx, 8(%rsp)
	movq	%rbp, 40(%rsp)
	movq	%rbp, 160(%rsp)
	cmpq	$3, %rcx
	movq	%rcx, %r15
	cmovaeq	%rdx, %r15
	cmovaeq	%rcx, %rdi
	cmovaeq	%r13, %rdx
	movq	%rdi, 496(%rsp)
	movq	%rax, 504(%rsp)
	movq	%rdx, 512(%rsp)
	movq	%rsi, 520(%rsp)
	movq	%r15, %rsi
	decq	%rsi
	movq	$0, 528(%rsp)
	movq	%rsi, 536(%rsp)
	je	.LBB239_555
	cmpq	$3, %rcx
	movq	16(%rsp), %rcx
	movq	<purrdf_sparql_eval::witness::RelationWitness>::merge@GOTPCREL(%rip), %rbp
	leaq	504(%rsp), %r12
	leaq	944(%rsp), %r14
	cmovaeq	%rax, %r12
	leaq	640(%rcx), %rbx
	.p2align	4
.LBB239_553:
	movq	%r13, 528(%rsp)
	movq	16(%r12), %rax
	movq	%rax, 960(%rsp)
	vmovdqu	(%r12), %xmm0
	vmovdqa	%xmm0, 944(%rsp)
.Ltmp7180:
	movq	%rbx, %rdi
	movq	%r14, %rsi
	callq	*%rbp
.Ltmp7181:
	addq	$24, %r12
	incq	%r13
	cmpq	%r13, %r15
	jne	.LBB239_553
.LBB239_555:
.Ltmp7186:
	leaq	496(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7187:
	movq	80(%rsp), %r13
	testb	$1, 48(%rsp)
	je	.LBB239_561
	movq	400(%rsp), %rbx
	movq	%rbx, %rcx
	subq	%r13, %rcx
	jb	.LBB239_715
	movq	392(%rsp), %r14
.Ltmp7188:
	movq	16(%rsp), %rsi
	leaq	496(%rsp), %rdi
	xorl	%edx, %edx
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7189:
	movq	40(%rsp), %r15
	cmpq	%r13, %rbx
	jne	.LBB239_637
.LBB239_560:
.Ltmp7213:
	leaq	496(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7214:
.LBB239_561:
	movq	160(%rsp), %rax
	vmovdqu	144(%rsp), %xmm0
	movq	%rax, 1360(%rsp)
	movq	352(%rsp), %rax
	vmovdqa	%xmm0, 1344(%rsp)
	testq	%rax, %rax
	je	.LBB239_564
	lock		decq	(%rax)
	jne	.LBB239_564
	#MEMBARRIER
.Ltmp7215:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	352(%rsp), %rdi
	callq	*%rax
.Ltmp7216:
.LBB239_564:
	movq	920(%rsp), %r14
	movb	$1, %bpl
.LBB239_565:
	movq	16(%rsp), %rax
	movq	696(%rax), %rax
	movl	40(%rax), %ecx
	testl	%ecx, %ecx
	je	.LBB239_569
.LBB239_566:
	vmovups	1992(%rsp), %zmm1
	vmovdqu64	1952(%rsp), %zmm0
	movq	1360(%rsp), %rcx
	movq	128(%rsp), %rax
	movq	104(%rsp), %r15
	movq	%rcx, 1488(%rsp)
	vmovups	%zmm1, 536(%rsp)
	vmovdqa	1344(%rsp), %xmm1
	vmovdqu64	%zmm0, 496(%rsp)
	cmpq	$-1, 496(%rsp)
	vmovdqa	%xmm1, 1472(%rsp)
	movq	%rax, 1496(%rsp)
	je	.LBB239_585
	leaq	944(%rsp), %rdi
	leaq	1472(%rsp), %rsi
	leaq	1952(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	568(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB239_568
.LBB239_586:
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
	jge	.LBB239_588
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB239_588:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB239_594
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB239_588
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
.LBB239_591:
	cmpq	%rax, %rdx
	jge	.LBB239_593
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB239_591
.LBB239_593:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB239_594:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	592(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB239_595
	jmp	.LBB239_597
.LBB239_569:
	movzbl	16(%rax), %ecx
	cmpb	$-1, %cl
	je	.LBB239_566
	movb	%cl, 944(%rsp)
	movq	128(%rsp), %rcx
	vmovdqu	17(%rax), %xmm0
	vmovdqu	%xmm0, 945(%rsp)
	movq	32(%rax), %rax
	movq	%rax, 960(%rsp)
	lock		incq	(%rcx)
	movq	104(%rsp), %rbx
	jle	.LBB239_721
	movq	128(%rsp), %rcx
	movl	%ebp, 8(%rsp)
.Ltmp7217:
	leaq	496(%rsp), %rdi
	leaq	944(%rsp), %rdx
	movq	%r14, %rsi
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
.Ltmp7218:
	vmovdqu64	528(%rsp), %zmm1
	vmovdqu64	496(%rsp), %zmm0
	movq	1360(%rsp), %r14
	vmovdqu64	%zmm1, 40(%rbx)
	vmovdqu64	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
	movq	1352(%rsp), %rbx
	testq	%r14, %r14
	je	.LBB239_627
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r12
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	xorl	%r15d, %r15d
	jmp	.LBB239_577
.LBB239_574:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB239_575:
	vzeroupper
	callq	*%rbp
.LBB239_576:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB239_627
.LBB239_577:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB239_576
	leaq	(%rbx,%rcx,8), %rdx
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
	jge	.LBB239_580
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB239_580:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB239_575
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB239_580
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
.LBB239_583:
	cmpq	%rax, %rdx
	jge	.LBB239_574
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB239_583
	jmp	.LBB239_574
.LBB239_585:
	vmovdqu	1472(%rsp), %xmm0
	movq	1488(%rsp), %rax
	movq	1496(%rsp), %rcx
	movq	%rax, 968(%rsp)
	movq	%rcx, 976(%rsp)
	vmovdqu	%xmm0, 952(%rsp)
	movq	$-1, 944(%rsp)
	movq	568(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB239_586
.LBB239_568:
	movq	592(%rsp), %rax
	testq	%rax, %rax
	je	.LBB239_597
.LBB239_595:
	lock		decq	(%rax)
	jne	.LBB239_597
	leaq	592(%rsp), %rdi
	#MEMBARRIER
.Ltmp7220:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp7221:
.LBB239_597:
	vmovdqu64	976(%rsp), %zmm1
	vmovdqu64	944(%rsp), %zmm0
	vmovdqu64	%zmm1, 40(%r15)
	vmovdqu64	%zmm0, 8(%r15)
	movq	$0, (%r15)
.Ltmp7223:
	leaq	2112(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp7224:
.Ltmp7226:
	leaq	2328(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7227:
	movq	408(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB239_601
	#MEMBARRIER
.Ltmp7229:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	408(%rsp), %rdi
	callq	*%rax
.Ltmp7230:
.LBB239_601:
	testb	%bpl, %bpl
	je	.LBB239_626
	movq	392(%rsp), %r14
	movq	400(%rsp), %rbx
	testq	%rbx, %rbx
	je	.LBB239_615
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r12
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	xorl	%r15d, %r15d
	jmp	.LBB239_607
	.p2align	4
.LBB239_604:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB239_605:
	callq	*%rbp
.LBB239_606:
	incq	%r15
	cmpq	%rbx, %r15
	je	.LBB239_615
.LBB239_607:
	leaq	(%r15,%r15,4), %rcx
	movq	(%r14,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB239_606
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
	jge	.LBB239_610
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB239_610:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB239_605
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB239_610
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
.LBB239_613:
	cmpq	%rax, %rdx
	jge	.LBB239_604
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB239_613
	jmp	.LBB239_604
.LBB239_615:
	movq	384(%rsp), %rax
	movq	104(%rsp), %r15
	testq	%rax, %rax
	je	.LBB239_626
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
	jge	.LBB239_618
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB239_618:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB239_624
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB239_618
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
.LBB239_621:
	cmpq	%rax, %rdx
	jge	.LBB239_623
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB239_621
.LBB239_623:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB239_624:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
.LBB239_625:
	vzeroupper
	callq	*%rax
.LBB239_626:
	movq	%r15, %rax
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
.LBB239_627:
	.cfi_def_cfa_offset 2704
	movq	1344(%rsp), %rax
	testq	%rax, %rax
	je	.LBB239_315
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
	jge	.LBB239_630
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB239_630:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB239_636
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB239_630
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
.LBB239_633:
	cmpq	%rax, %rdx
	jge	.LBB239_635
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB239_633
.LBB239_635:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB239_636:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
	jmp	.LBB239_315
.LBB239_637:
	leaq	(,%r15,8), %rax
	shlq	$3, %r13
	shlq	$3, %rbx
	leaq	944(%rsp), %rbp
	leaq	(%rbx,%rbx,4), %rbx
	leaq	(%rax,%rax,4), %r12
	leaq	(%r13,%r13,4), %rax
	leaq	8(%r14,%rax), %r14
	subq	%rax, %rbx
	jmp	.LBB239_640
.LBB239_638:
	movq	152(%rsp), %rax
	movq	48(%rsp), %rdx
	movq	%rax, 8(%rsp)
.LBB239_639:
	movq	8(%rsp), %rcx
	incq	%r15
	addq	$40, %r14
	movq	%r13, (%rcx,%r12)
	movq	%rbx, 8(%rcx,%r12)
	vmovdqa	944(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rcx,%r12)
	movq	960(%rsp), %rax
	movq	%rax, 32(%rcx,%r12)
	addq	$40, %r12
	addq	$-40, %rdx
	movq	%r15, 160(%rsp)
	movq	%rdx, %rbx
	je	.LBB239_560
.LBB239_640:
.Ltmp7190:
	movq	16(%rsp), %rdx
	leaq	496(%rsp), %rsi
	movq	%rbp, %rdi
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7191:
	cmpb	$-1, 944(%rsp)
	jne	.LBB239_560
	movq	776(%rsp), %rdx
	movq	%rbx, 48(%rsp)
	movq	$1, 944(%rsp)
	cmpq	$5, %rdx
	jae	.LBB239_663
.LBB239_643:
	vmovdqu	952(%rsp), %xmm0
	movq	976(%rsp), %rax
	movq	944(%rsp), %rdx
	movq	968(%rsp), %rcx
	movq	%r14, %r13
	movq	%rax, 1216(%rsp)
	movq	%rdx, 1184(%rsp)
	movq	%rcx, 1208(%rsp)
	vmovdqu	%xmm0, 1192(%rsp)
	movq	-8(%r14), %rbx
	decq	%rbx
	cmpq	$5, %rbx
	jb	.LBB239_645
	movq	8(%r14), %rbx
	movq	(%r14), %r13
	decq	%rbx
.LBB239_645:
	movq	1184(%rsp), %rax
	movq	1200(%rsp), %rsi
	movl	$4, %edx
	movq	%r15, 40(%rsp)
	leaq	-1(%rax), %rcx
	decq	%rsi
	cmpq	$5, %rcx
	cmovbq	%rcx, %rsi
	cmovbq	%rdx, %rcx
	subq	%rsi, %rcx
	cmpq	%rbx, %rcx
	jb	.LBB239_664
.LBB239_646:
	xorl	%ebp, %ebp
	leaq	1192(%rsp), %rcx
	cmpq	$6, %rax
	setae	%al
	jb	.LBB239_648
	movq	1192(%rsp), %rcx
.LBB239_648:
	movb	%al, %bpl
	movq	memcpy@GOTPCREL(%rip), %rax
	leaq	(,%rbx,8), %rdx
	movq	%r13, %rsi
	shll	$4, %ebp
	movq	1184(%rsp,%rbp), %r15
	leaq	-8(%rcx,%r15,8), %rdi
	callq	*%rax
	addq	%rbx, %r15
	movq	776(%rsp), %rax
	movq	%r15, 1184(%rsp,%rbp)
	movq	1184(%rsp), %rcx
	movq	1200(%rsp), %rdx
	leaq	-1(%rcx), %rsi
	leaq	-1(%rdx), %rdi
	cmpq	$5, %rsi
	cmovbq	%rsi, %rdi
	movq	%rax, %rsi
	subq	%rdi, %rsi
	jbe	.LBB239_650
	movl	$2, 944(%rsp)
	movq	%rsi, 952(%rsp)
.Ltmp7198:
	leaq	944(%rsp), %rbp
	leaq	1184(%rsp), %rdi
	movq	%rbp, %rsi
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp7199:
	movq	104(%rsp), %rbx
	movq	40(%rsp), %r15
	jmp	.LBB239_652
.LBB239_650:
	movq	104(%rsp), %rbx
	movq	40(%rsp), %r15
	cmpq	$6, %rcx
	leaq	944(%rsp), %rbp
	cmovbq	%rcx, %rdx
	decq	%rdx
	cmpq	%rdx, %rax
	jae	.LBB239_652
	xorl	%edx, %edx
	cmpq	$6, %rcx
	setae	%dl
	incq	%rax
	shll	$4, %edx
	movq	%rax, 1184(%rsp,%rdx)
.LBB239_652:
	movq	1184(%rsp), %rcx
	leaq	1192(%rsp), %rdx
	decq	%rcx
	cmpq	$5, %rcx
	jb	.LBB239_654
	movq	1200(%rsp), %rcx
	movq	1192(%rsp), %rdx
	decq	%rcx
.LBB239_654:
	movq	128(%rsp), %r8
	addq	$16, %r8
.Ltmp7200:
	movq	16(%rsp), %r9
	leaq	2112(%rsp), %rsi
	movq	%rbp, %rdi
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7201:
	vmovq	952(%rsp), %xmm0
	movq	944(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB239_666
	movq	1184(%rsp), %rax
	movq	%rax, %rsi
	cmpq	$6, %rax
	jb	.LBB239_658
	movq	1200(%rsp), %rsi
.LBB239_658:
	movq	928(%rsp), %rdi
	decq	%rsi
	cmpq	%rsi, %rdi
	jae	.LBB239_720
	leaq	1192(%rsp), %rcx
	cmpq	$6, %rax
	jb	.LBB239_661
	movq	1192(%rsp), %rcx
.LBB239_661:
	vmovq	%xmm0, (%rcx,%rdi,8)
	leaq	1200(%rsp), %rax
	movq	48(%rsp), %rdx
	vmovdqu	(%rax), %xmm0
	movq	16(%rax), %rax
	movq	1184(%rsp), %r13
	movq	1192(%rsp), %rbx
	movq	%rax, 960(%rsp)
	vmovdqa	%xmm0, 944(%rsp)
	cmpq	144(%rsp), %r15
	jne	.LBB239_639
.Ltmp7208:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	144(%rsp), %rdi
	callq	*%rax
.Ltmp7209:
	jmp	.LBB239_638
.LBB239_663:
.Ltmp7193:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	movq	%rbp, %rdi
	xorl	%esi, %esi
	xorl	%ecx, %ecx
	callq	*%rax
.Ltmp7194:
	jmp	.LBB239_643
.LBB239_664:
.Ltmp7196:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	1184(%rsp), %rdi
	movl	$1, %ecx
	movq	%rbx, %rdx
	callq	*%rax
.Ltmp7197:
	movq	1184(%rsp), %rax
	jmp	.LBB239_646
.LBB239_666:
	vmovups	976(%rsp), %zmm2
	vmovups	960(%rsp), %zmm1
	vmovups	%zmm2, 48(%rbx)
	vmovups	%zmm1, 32(%rbx)
	movq	%rax, 16(%rbx)
	movq	1184(%rsp), %rax
	vmovq	%xmm0, 24(%rbx)
	movq	$1, (%rbx)
	cmpq	$6, %rax
	jb	.LBB239_668
	movq	1192(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB239_668:
.Ltmp7203:
	leaq	496(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7204:
	leaq	144(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	352(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB239_312
	jmp	.LBB239_314
.LBB239_670:
	vmovups	192(%rsp), %ymm0
	movq	160(%rsp), %rcx
	movq	%rax, 96(%rsp)
	movzbl	176(%rsp), %eax
	movzbl	183(%rsp), %edx
	addq	$40, %rbx
	movq	152(%rsp), %rsi
	movl	177(%rsp), %r13d
	movq	%rbx, 840(%rsp)
	movq	168(%rsp), %rbx
	movq	%rcx, 8(%rsp)
	movzwl	181(%rsp), %ecx
	movq	%rax, 48(%rsp)
	movq	184(%rsp), %rax
	shll	$16, %edx
	movq	%rsi, 56(%rsp)
	vmovups	%ymm0, 784(%rsp)
	vmovdqu	208(%rsp), %ymm0
	orl	%edx, %ecx
	movq	%rax, 80(%rsp)
	movb	$1, %al
	shlq	$32, %rcx
	movl	%eax, 88(%rsp)
	orq	%rcx, %r13
	vmovdqu	%ymm0, 800(%rsp)
.LBB239_671:
	movq	64(%rsp), %r12
	jmp	.LBB239_692
.LBB239_672:
	movq	40(%rsp), %rbp
	leaq	-1(%r14), %rax
	cmpq	%rax, %rbp
	jae	.LBB239_688
	movq	8(%rsp), %r13
	movq	64(%rsp), %r12
	cmpq	120(%rsp), %r13
	je	.LBB239_712
	notq	%rbp
	addq	$88, %r13
	leaq	144(%rsp), %rbx
	addq	%r14, %rbp
	movq	%r13, %rax
.LBB239_675:
	movq	%rax, %r13
	movq	-8(%r13), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rcx, 480(%rsp)
	vmovdqu64	-72(%r13), %zmm0
	vmovdqu64	%zmm0, 416(%rsp)
	cmpq	$-1, %rax
	je	.LBB239_712
	vmovdqu64	416(%rsp), %zmm0
	movq	%rax, 144(%rsp)
	movq	480(%rsp), %rax
	leaq	152(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7070:
	movq	48(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%rbx, %rdx
	vzeroupper
	callq	*%rax
.Ltmp7071:
	decq	%rbp
	je	.LBB239_712
	leaq	-88(%r13), %rcx
	leaq	88(%r13), %rax
	addq	$88, %rcx
	cmpq	120(%rsp), %rcx
	jne	.LBB239_675
.LBB239_712:
	movq	16(%rsp), %r14
	movq	%r13, 328(%rsp)
	jmp	.LBB239_691
.LBB239_680:
	movq	40(%rsp), %r14
	leaq	-1(%rbx), %rax
	cmpq	%rax, %r14
	jae	.LBB239_689
	movq	8(%rsp), %r12
	cmpq	120(%rsp), %r12
	je	.LBB239_690
	notq	%r14
	addq	$88, %r12
	addq	%rbx, %r14
	leaq	144(%rsp), %rbx
	movq	%r12, %rax
.LBB239_683:
	movq	%rax, %r12
	movq	-8(%r12), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rcx, 480(%rsp)
	vmovdqu64	-72(%r12), %zmm0
	vmovdqu64	%zmm0, 416(%rsp)
	cmpq	$-1, %rax
	je	.LBB239_690
	vmovdqu64	416(%rsp), %zmm0
	movq	%rax, 144(%rsp)
	movq	480(%rsp), %rax
	leaq	152(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7064:
	movq	48(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%rbx, %rdx
	vzeroupper
	callq	*%rax
.Ltmp7065:
	decq	%r14
	je	.LBB239_690
	leaq	-88(%r12), %rcx
	leaq	88(%r12), %rax
	addq	$88, %rcx
	cmpq	120(%rsp), %rcx
	jne	.LBB239_683
	jmp	.LBB239_690
.LBB239_687:
	movq	264(%rsp), %rax
	movq	272(%rsp), %rcx
	movq	280(%rsp), %rbx
	movq	$-1, 96(%rsp)
	movq	$0, 48(%rsp)
	movl	$0, 88(%rsp)
	movq	%rax, 56(%rsp)
	movq	%rcx, 8(%rsp)
	jmp	.LBB239_671
.LBB239_688:
	movq	8(%rsp), %r13
	movq	16(%rsp), %r14
	movq	64(%rsp), %r12
	movq	%r13, 328(%rsp)
	jmp	.LBB239_691
.LBB239_689:
	movq	8(%rsp), %r12
.LBB239_690:
	movq	%r12, 328(%rsp)
	movq	16(%rsp), %r14
	movq	64(%rsp), %r12
.LBB239_691:
	movq	264(%rsp), %rax
	movq	272(%rsp), %rcx
	movq	280(%rsp), %rbx
	movq	$-1, 96(%rsp)
	movl	$0, 88(%rsp)
	movq	%rax, 56(%rsp)
	movb	$1, %al
	movq	%rcx, 8(%rsp)
	movq	%rax, 48(%rsp)
.LBB239_692:
	movq	376(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB239_694
	movq	760(%rsp), %rdi
	shlq	$5, %rsi
	movl	$8, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB239_694:
.Ltmp7079:
	leaq	320(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp7080:
	testq	%r12, %r12
	je	.LBB239_697
	movq	248(%rsp), %rdi
	shlq	$3, %r12
	movl	$8, %edx
	leaq	(%r12,%r12,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB239_697:
	movq	584(%rsp), %rax
	testq	%rax, %rax
	je	.LBB239_700
	lock		decq	(%rax)
	jne	.LBB239_700
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	584(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB239_700:
	movq	616(%rsp), %rax
	testq	%rax, %rax
	je	.LBB239_703
	lock		decq	(%rax)
	jne	.LBB239_703
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	616(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB239_703:
	movq	%rbx, 40(%rsp)
	movl	$0, 72(%rsp)
.Ltmp7082:
	leaq	1376(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp7083:
	movq	136(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB239_706
	xorl	%r12d, %r12d
	#MEMBARRIER
.Ltmp7084:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	936(%rsp), %rdi
	callq	*%rax
.Ltmp7085:
.LBB239_706:
	movl	88(%rsp), %ebx
	movl	$0, 72(%rsp)
.LBB239_707:
	cmpq	$0, 1416(%rsp)
	movq	104(%rsp), %r12
	je	.LBB239_709
.Ltmp7115:
	leaq	832(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7116:
.LBB239_709:
	movq	304(%rsp), %rbp
	testb	%bl, %bl
	je	.LBB239_711
	leaq	264(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB239_711:
	cmpb	$0, 72(%rsp)
	jne	.LBB239_301
	jmp	.LBB239_307
.LBB239_713:
.Ltmp7140:
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.152(%rip), %rdi
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.154(%rip), %rdx
	movl	$83, %esi
	vzeroupper
	callq	*%rax
.Ltmp7141:
	jmp	.LBB239_721
.LBB239_714:
.Ltmp7089:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	movq	888(%rsp), %rdx
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.290(%rip), %rcx
	callq	*%rax
.Ltmp7090:
	jmp	.LBB239_721
.LBB239_715:
.Ltmp7232:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.404(%rip), %rcx
	movq	%r13, %rdi
	movq	%rbx, %rsi
	movq	%rbx, %rdx
	callq	*%rax
.Ltmp7233:
	jmp	.LBB239_721
.LBB239_716:
	movq	%rbp, 1480(%rsp)
	movq	%rbx, 1504(%rsp)
.Ltmp6959:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.402(%rip), %rdx
	callq	*%rax
.Ltmp6960:
	jmp	.LBB239_721
.LBB239_717:
.Ltmp7259:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	leaq	512(%rsp), %rbx
	callq	*%rax
.Ltmp7260:
	jmp	.LBB239_721
.LBB239_718:
.Ltmp7298:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	leaq	960(%rsp), %rbx
	callq	*%rax
.Ltmp7299:
	jmp	.LBB239_721
.LBB239_719:
.Ltmp6950:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp6951:
	jmp	.LBB239_721
.LBB239_720:
.Ltmp7205:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.403(%rip), %rdx
	callq	*%rax
.Ltmp7206:
.LBB239_721:
	ud2
.LBB239_722:
.Ltmp7195:
	movq	%rax, 16(%rsp)
	movq	944(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB239_804
	movq	952(%rsp), %rdi
	jmp	.LBB239_802
.LBB239_724:
.Ltmp7066:
	movq	%rax, 16(%rsp)
	movq	%r12, 328(%rsp)
	jmp	.LBB239_843
.LBB239_725:
.Ltmp7072:
	movq	%rax, 16(%rsp)
	movq	%r13, 328(%rsp)
	jmp	.LBB239_843
.LBB239_726:
.Ltmp7096:
	movq	%rax, 16(%rsp)
	movb	$1, %al
	movl	%eax, 88(%rsp)
	jmp	.LBB239_846
.LBB239_727:
.Ltmp7081:
	movq	%rax, 16(%rsp)
	jmp	.LBB239_846
.LBB239_728:
.Ltmp7042:
	jmp	.LBB239_769
.LBB239_729:
.Ltmp7117:
	movl	%ebx, 88(%rsp)
	movq	%rax, 16(%rsp)
	jmp	.LBB239_859
.LBB239_730:
.Ltmp7125:
	cmpq	$0, 1416(%rsp)
	movl	%r12d, 72(%rsp)
	movq	%rax, 16(%rsp)
	jne	.LBB239_858
	jmp	.LBB239_859
.LBB239_731:
.Ltmp6998:
	movq	%rax, 16(%rsp)
	jmp	.LBB239_773
.LBB239_732:
.Ltmp7103:
	movq	%rax, 16(%rsp)
	jmp	.LBB239_856
.LBB239_733:
.Ltmp7139:
	leaq	496(%rsp), %rdi
	movq	%rax, 16(%rsp)
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB239_869
.LBB239_734:
.Ltmp7169:
	movq	%rax, 16(%rsp)
	jmp	.LBB239_784
.LBB239_735:
.Ltmp7054:
	jmp	.LBB239_769
.LBB239_736:
.Ltmp7031:
	movq	%rax, 16(%rsp)
	jmp	.LBB239_862
.LBB239_737:
.Ltmp6984:
	movq	%rax, 16(%rsp)
	jmp	.LBB239_774
.LBB239_738:
.Ltmp7150:
	movq	%rax, 16(%rsp)
	jmp	.LBB239_866
.LBB239_739:
.Ltmp7112:
	addq	$40, %r12
	movq	%rax, 16(%rsp)
	movq	%r12, 152(%rsp)
	jmp	.LBB239_748
.LBB239_740:
.Ltmp7075:
	jmp	.LBB239_769
.LBB239_741:
.Ltmp7003:
	movq	%rax, 16(%rsp)
	jmp	.LBB239_869
.LBB239_742:
.Ltmp7026:
	movq	%rax, 16(%rsp)
.Ltmp7027:
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7028:
	jmp	.LBB239_862
.LBB239_743:
.Ltmp7017:
	movq	%rax, 16(%rsp)
	jmp	.LBB239_788
.LBB239_744:
.Ltmp6979:
	movq	%rax, 16(%rsp)
.Ltmp6980:
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp6981:
	jmp	.LBB239_774
.LBB239_745:
.Ltmp7006:
	movq	%rax, 16(%rsp)
	jmp	.LBB239_789
.LBB239_746:
.Ltmp7120:
	addq	$40, %r12
	cmpq	$6, 80(%rsp)
	movq	%rax, 16(%rsp)
	movq	%r12, 152(%rsp)
	jb	.LBB239_748
	movq	80(%rsp), %rax
	movq	56(%rsp), %rdi
	movl	$4, %edx
	leaq	-8(,%rax,8), %rsi
	callq	__rustc::__rust_dealloc
.LBB239_748:
	movb	$1, %al
	movl	%eax, 72(%rsp)
.Ltmp7121:
	leaq	144(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7122:
	jmp	.LBB239_860
.LBB239_749:
.Ltmp7177:
	movq	%rax, 16(%rsp)
	jmp	.LBB239_869
.LBB239_750:
.Ltmp7145:
	movq	%rax, 16(%rsp)
.Ltmp7146:
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7147:
	jmp	.LBB239_866
.LBB239_751:
.Ltmp7219:
	leaq	1344(%rsp), %rdi
	movq	%rax, 16(%rsp)
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB239_874
.LBB239_752:
.Ltmp7222:
	movl	%ebp, 8(%rsp)
	movq	%rax, 16(%rsp)
	xorl	%ebx, %ebx
	jmp	.LBB239_875
.LBB239_753:
.Ltmp7051:
	jmp	.LBB239_842
.LBB239_754:
.Ltmp7078:
	addq	$40, %rbx
	movq	%rax, 16(%rsp)
	movq	%rbx, 840(%rsp)
	jmp	.LBB239_843
.LBB239_755:
.Ltmp7210:
	movq	%rax, 16(%rsp)
	cmpq	$6, %r13
	jb	.LBB239_804
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
	movq	%rbx, %rdi
	jmp	.LBB239_803
.LBB239_757:
.Ltmp7069:
	jmp	.LBB239_842
.LBB239_758:
.Ltmp7255:
	jmp	.LBB239_797
.LBB239_759:
.Ltmp7088:
	addq	$40, %rbx
	cmpq	$6, 56(%rsp)
	movq	%rax, 16(%rsp)
	movq	%rbx, 840(%rsp)
	jb	.LBB239_843
	movq	56(%rsp), %rax
	movq	96(%rsp), %rdi
	movl	$4, %edx
	leaq	-8(,%rax,8), %rsi
	callq	__rustc::__rust_dealloc
	jmp	.LBB239_843
.LBB239_761:
.Ltmp7160:
	jmp	.LBB239_806
.LBB239_762:
.Ltmp6987:
	movq	40(%rsp), %rcx
	movq	%rax, 16(%rsp)
	movq	%rcx, 424(%rsp)
	jmp	.LBB239_772
.LBB239_763:
.Ltmp7231:
	movq	%rax, 16(%rsp)
	testb	%bpl, %bpl
	jne	.LBB239_886
	jmp	.LBB239_887
.LBB239_764:
.Ltmp7284:
	movq	%rax, 16(%rsp)
	lock		decq	(%rbx)
	jne	.LBB239_832
	#MEMBARRIER
.Ltmp7285:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	496(%rsp), %rdi
	callq	*%rax
.Ltmp7286:
	jmp	.LBB239_832
.LBB239_766:
.Ltmp7287:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB239_767:
.Ltmp7153:
	movq	40(%rsp), %rcx
	movq	%rax, 16(%rsp)
	movq	%rcx, 792(%rsp)
	jmp	.LBB239_783
.LBB239_768:
.Ltmp7059:
.LBB239_769:
	movq	8(%rsp), %rcx
	movq	%rax, 16(%rsp)
	movq	%rcx, 328(%rsp)
	jmp	.LBB239_843
.LBB239_770:
.Ltmp6992:
	movq	40(%rsp), %rcx
	movq	%rax, 16(%rsp)
	movq	%rcx, 424(%rsp)
	cmpq	$6, %r15
	jb	.LBB239_772
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	movq	%r12, %rdi
	callq	__rustc::__rust_dealloc
.LBB239_772:
.Ltmp6993:
	leaq	416(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp6994:
.LBB239_773:
	leaq	784(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB239_774:
.Ltmp6999:
	leaq	944(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp7000:
	jmp	.LBB239_869
.LBB239_775:
.Ltmp6995:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB239_776:
.Ltmp7258:
	movq	%rax, 16(%rsp)
	testb	%r12b, %r12b
	jne	.LBB239_886
	jmp	.LBB239_887
.LBB239_777:
.Ltmp7249:
	movq	%rax, 16(%rsp)
	jmp	.LBB239_817
.LBB239_778:
.Ltmp7207:
	jmp	.LBB239_800
.LBB239_779:
.Ltmp6974:
	movq	%rax, 16(%rsp)
.Ltmp6975:
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp6976:
	jmp	.LBB239_870
.LBB239_780:
.Ltmp7192:
	movq	%rax, 16(%rsp)
	jmp	.LBB239_804
.LBB239_781:
.Ltmp7163:
	movq	40(%rsp), %rcx
	movq	%rax, 16(%rsp)
	movq	%rcx, 792(%rsp)
	cmpq	$6, %r15
	jb	.LBB239_783
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	movq	%r12, %rdi
	callq	__rustc::__rust_dealloc
.LBB239_783:
.Ltmp7164:
	leaq	784(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7165:
.LBB239_784:
	leaq	1424(%rsp), %rdi
	jmp	.LBB239_865
.LBB239_785:
.Ltmp7166:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB239_786:
.Ltmp7009:
	movq	%rax, 16(%rsp)
	movq	%rbp, 1192(%rsp)
.Ltmp7010:
	leaq	496(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7011:
.Ltmp7013:
	leaq	1184(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp7014:
.LBB239_788:
.Ltmp7018:
	leaq	144(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp7019:
.LBB239_789:
.Ltmp7021:
	leaq	1920(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7022:
	jmp	.LBB239_869
.LBB239_790:
.Ltmp7012:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB239_791:
.Ltmp7020:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB239_792:
.Ltmp7023:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB239_793:
.Ltmp7274:
	movq	%rax, 16(%rsp)
	lock		decq	(%r15)
	jne	.LBB239_832
	#MEMBARRIER
.Ltmp7275:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	496(%rsp), %rdi
	callq	*%rax
.Ltmp7276:
	jmp	.LBB239_832
.LBB239_795:
.Ltmp7277:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB239_796:
.Ltmp7228:
	movl	%ebp, 8(%rsp)
.LBB239_797:
	movq	%rax, 16(%rsp)
	jmp	.LBB239_883
.LBB239_798:
.Ltmp7225:
	movl	%ebp, 8(%rsp)
	movq	%rax, 16(%rsp)
	jmp	.LBB239_879
.LBB239_799:
.Ltmp7202:
.LBB239_800:
	movq	%rax, 16(%rsp)
	movq	1184(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB239_804
	movq	1192(%rsp), %rdi
.LBB239_802:
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
.LBB239_803:
	callq	__rustc::__rust_dealloc
.LBB239_804:
.Ltmp7211:
	leaq	496(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7212:
	jmp	.LBB239_840
.LBB239_805:
.Ltmp6971:
.LBB239_806:
	movq	%rax, 16(%rsp)
	jmp	.LBB239_870
.LBB239_807:
.Ltmp7300:
	movq	%rax, 16(%rsp)
.Ltmp7301:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.4261325137610144415)
.Ltmp7302:
.Ltmp7304:
	leaq	1816(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp7305:
	jmp	.LBB239_887
.LBB239_809:
.Ltmp7303:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB239_810:
.Ltmp7293:
	movq	%rax, 16(%rsp)
.Ltmp7294:
	leaq	2112(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.4261325137610144415)
.Ltmp7295:
	jmp	.LBB239_832
.LBB239_811:
.Ltmp7239:
	movq	%rax, 16(%rsp)
	movb	$1, %al
	movb	$1, %bl
	movl	%eax, 8(%rsp)
	jmp	.LBB239_875
.LBB239_812:
.Ltmp7261:
	movq	%rax, 16(%rsp)
.Ltmp7262:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.4261325137610144415)
.Ltmp7263:
	jmp	.LBB239_816
.LBB239_813:
.Ltmp7264:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB239_814:
.Ltmp6943:
	movq	%rax, 16(%rsp)
.Ltmp6944:
	leaq	2056(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.4261325137610144415)
.Ltmp6945:
	jmp	.LBB239_816
.LBB239_815:
.Ltmp6940:
	movq	%rax, 16(%rsp)
.LBB239_816:
	movb	$1, %al
	movl	%eax, 8(%rsp)
.LBB239_817:
	movb	$1, %bl
	jmp	.LBB239_880
.LBB239_818:
.Ltmp6964:
	movq	%rbp, 1480(%rsp)
	movq	%rax, 16(%rsp)
	movq	%rbx, 1504(%rsp)
	cmpq	$5, %r13
	ja	.LBB239_837
	jmp	.LBB239_838
.LBB239_819:
.Ltmp7133:
	movq	%rax, 16(%rsp)
	testq	%rbx, %rbx
	je	.LBB239_823
	negq	%rbx
	addq	$160, %r15
.LBB239_821:
.Ltmp7134:
	movq	%r15, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7135:
	addq	$160, %r15
	decq	%rbx
	jne	.LBB239_821
.LBB239_823:
	cmpq	$0, 896(%rsp)
	je	.LBB239_869
	movq	896(%rsp), %rax
	movq	304(%rsp), %rdi
	movl	$8, %edx
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
	jmp	.LBB239_869
.LBB239_825:
.Ltmp7136:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB239_826:
.Ltmp7182:
	movq	%rax, 16(%rsp)
.Ltmp7183:
	leaq	496(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7184:
	jmp	.LBB239_840
.LBB239_827:
.Ltmp7185:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB239_828:
.Ltmp6961:
	movq	%rax, 16(%rsp)
	jmp	.LBB239_835
.LBB239_829:
.Ltmp7244:
	movq	%rax, 16(%rsp)
	jmp	.LBB239_877
.LBB239_830:
.Ltmp7252:
	movl	%r12d, 8(%rsp)
	movq	%rax, 16(%rsp)
	jmp	.LBB239_882
.LBB239_831:
.Ltmp7290:
	movq	%rax, 16(%rsp)
.LBB239_832:
.Ltmp7296:
	leaq	1816(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp7297:
	jmp	.LBB239_887
.LBB239_833:
.Ltmp7306:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB239_834:
.Ltmp6958:
	movq	%rbp, 1480(%rsp)
	movq	%rax, 16(%rsp)
	movq	%rbx, 1504(%rsp)
.LBB239_835:
	movq	944(%rsp), %r13
	cmpq	$6, %r13
	jb	.LBB239_838
	movq	952(%rsp), %r14
.LBB239_837:
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
	movq	%r14, %rdi
	callq	__rustc::__rust_dealloc
.LBB239_838:
	leaq	1472(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, <purrdf_sparql_eval::bgp::BgpProjection>::apply<purrdf_core::ir::term::TermId>::{closure#0}>>
	leaq	144(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movb	$1, %bl
	movl	$0, 8(%rsp)
	jmp	.LBB239_875
.LBB239_839:
.Ltmp7234:
	movq	%rax, 16(%rsp)
.LBB239_840:
	leaq	144(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB239_870
.LBB239_841:
.Ltmp7091:
.LBB239_842:
	movq	%rax, 16(%rsp)
.LBB239_843:
	cmpq	$0, 376(%rsp)
	je	.LBB239_845
	movq	376(%rsp), %rsi
	movq	760(%rsp), %rdi
	movl	$8, %edx
	shlq	$5, %rsi
	callq	__rustc::__rust_dealloc
.LBB239_845:
	movb	$1, %al
	movl	%eax, 88(%rsp)
.Ltmp7092:
	leaq	320(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp7093:
.LBB239_846:
	cmpq	$0, 64(%rsp)
	je	.LBB239_848
	movq	64(%rsp), %rax
	movq	248(%rsp), %rdi
	movl	$8, %edx
	shlq	$3, %rax
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB239_848:
	movq	584(%rsp), %rax
	testq	%rax, %rax
	je	.LBB239_851
	lock		decq	(%rax)
	jne	.LBB239_851
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	584(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB239_851:
	movq	616(%rsp), %rax
	testq	%rax, %rax
	je	.LBB239_854
	lock		decq	(%rax)
	jne	.LBB239_854
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	616(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB239_854:
.Ltmp7097:
	leaq	1376(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp7098:
	movl	$0, 72(%rsp)
.LBB239_856:
	movq	136(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB239_858
	#MEMBARRIER
.Ltmp7104:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	936(%rsp), %rdi
	callq	*%rax
.Ltmp7105:
.LBB239_858:
.Ltmp7126:
	leaq	832(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7127:
.LBB239_859:
	cmpb	$0, 88(%rsp)
	je	.LBB239_861
.LBB239_860:
	leaq	264(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB239_861:
	cmpb	$0, 72(%rsp)
	je	.LBB239_869
.LBB239_862:
.Ltmp7128:
	leaq	1784(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp7129:
	jmp	.LBB239_869
.LBB239_863:
.Ltmp7130:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB239_864:
.Ltmp7142:
	leaq	1376(%rsp), %rdi
	movq	%rax, 16(%rsp)
.LBB239_865:
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB239_866:
.Ltmp7170:
	leaq	496(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp7171:
	cmpb	$0, 96(%rsp)
	je	.LBB239_869
.Ltmp7172:
	leaq	832(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7173:
.LBB239_869:
.Ltmp7178:
	leaq	1728(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7179:
.LBB239_870:
	movq	352(%rsp), %rax
	movb	$1, %cl
	movl	%ecx, 8(%rsp)
	testq	%rax, %rax
	je	.LBB239_874
	lock		decq	(%rax)
	movb	$1, %al
	movl	%eax, 8(%rsp)
	jne	.LBB239_874
	movb	$1, %al
	#MEMBARRIER
	movl	%eax, 8(%rsp)
.Ltmp7235:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	352(%rsp), %rdi
	callq	*%rax
.Ltmp7236:
	movb	$1, %bl
	jmp	.LBB239_875
.LBB239_874:
	movb	$1, %bl
.LBB239_875:
.Ltmp7240:
	leaq	2112(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp7241:
	testb	%bl, %bl
	je	.LBB239_879
.LBB239_877:
	movq	128(%rsp), %rax
	movb	$1, %bl
	lock		decq	(%rax)
	jne	.LBB239_880
	#MEMBARRIER
.Ltmp7245:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	128(%rsp), %rdi
	callq	*%rax
.Ltmp7246:
	jmp	.LBB239_880
.LBB239_879:
	xorl	%ebx, %ebx
.LBB239_880:
.Ltmp7265:
	leaq	2328(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7266:
	testb	%bl, %bl
	je	.LBB239_883
.LBB239_882:
.Ltmp7267:
	leaq	1952(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp7268:
.LBB239_883:
	movq	408(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB239_885
	leaq	408(%rsp), %rdi
	#MEMBARRIER
.Ltmp7269:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp7270:
.LBB239_885:
	cmpb	$0, 8(%rsp)
	je	.LBB239_887
.LBB239_886:
	leaq	384(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB239_887:
	movq	16(%rsp), %rdi
	callq	_Unwind_Resume@PLT
.LBB239_888:
.Ltmp7174:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB239_889:
.Ltmp7271:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end239:
