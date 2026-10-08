purrdf_sparql_eval::expr::eval_filter::<purrdf_core::ir::dataset::RdfDataset>:
.Lfunc_begin242:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception162
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
	subq	$2360, %rsp
	.cfi_def_cfa_offset 2416
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	%rdi, %r12
	leaq	1720(%rsp), %rdi
	movq	%r8, %r15
	movq	%rcx, %r14
	movq	%rdx, %rbx
	movq	%rsi, %r13
	callq	*%rax
.Ltmp7257:
	leaq	1824(%rsp), %rdi
	movq	%r14, %rsi
	movq	%r15, 32(%rsp)
	movq	%r15, %rdx
	movq	%r14, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp7258:
	cmpl	$1, 1824(%rsp)
	jne	.LBB242_25
	vmovdqu64	1872(%rsp), %zmm1
	vmovdqu64	1840(%rsp), %zmm0
	movq	1792(%rsp), %rax
	vmovdqu64	%zmm1, 48(%r12)
	vmovdqu64	%zmm0, 16(%r12)
	movq	$1, (%r12)
	cmpq	$6, %rax
	jb	.LBB242_12
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	1800(%rsp), %rdi
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
	jge	.LBB242_5
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB242_5:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB242_11
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB242_5
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
.LBB242_8:
	cmpq	%rax, %rsi
	jge	.LBB242_10
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB242_8
.LBB242_10:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB242_11:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB242_12:
	movq	1720(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB242_22
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	1728(%rsp), %rdi
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
	jge	.LBB242_15
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB242_15:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB242_21
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB242_15
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
.LBB242_18:
	cmpq	%rax, %rsi
	jge	.LBB242_20
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB242_18
.LBB242_20:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB242_21:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB242_22:
	movq	1816(%rsp), %rax
	testq	%rax, %rax
	je	.LBB242_633
	lock		decq	(%rax)
	jne	.LBB242_633
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1816(%rsp), %rdi
	#MEMBARRIER
	jmp	.LBB242_632
.LBB242_25:
	vmovdqu64	1864(%rsp), %zmm1
	vmovdqu64	1832(%rsp), %zmm0
	vmovdqu64	%zmm1, 480(%rsp)
	vmovdqu64	%zmm0, 448(%rsp)
.Ltmp7259:
	leaq	880(%rsp), %rdi
	leaq	1720(%rsp), %rsi
	leaq	448(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp7260:
	cmpq	$-1, 880(%rsp)
	je	.LBB242_812
	vmovdqu	880(%rsp), %ymm0
	vmovdqu64	1760(%rsp), %zmm1
	vmovdqu64	1720(%rsp), %zmm2
	vmovdqu	%ymm0, 416(%rsp)
	vmovdqu64	%zmm1, 1656(%rsp)
	vmovdqu64	%zmm2, 1616(%rsp)
	movq	440(%rsp), %rax
	lock		incq	(%rax)
	jle	.LBB242_696
	movq	%rax, 120(%rsp)
	movb	$1, %al
	movl	%eax, 16(%rsp)
.Ltmp7265:
	movq	32(%rsp), %r14
	movq	%rbx, %rsi
	movq	%r14, %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop
.Ltmp7266:
	cmpb	$2, 472(%r14)
	sete	%cl
	andb	%cl, %al
	cmpb	$1, %al
	jne	.LBB242_32
	movq	616(%r14), %rax
	testq	%rax, %rax
	je	.LBB242_33
	cmpq	$-2, 24(%rax)
	jb	.LBB242_32
	cmpq	$-2, 32(%rax)
	jae	.LBB242_35
.LBB242_32:
	xorl	%ebp, %ebp
	jmp	.LBB242_36
.LBB242_812:
	leaq	8(%r12), %rdi
	leaq	1720(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
	movq	$0, (%r12)
	jmp	.LBB242_633
.LBB242_33:
	movb	$1, %bpl
	jmp	.LBB242_36
.LBB242_35:
	cmpq	$-2, 48(%rax)
	setae	%bpl
.LBB242_36:
	movq	432(%rsp), %r14
.Ltmp7267:
	movq	32(%rsp), %r15
	movzbl	%bpl, %edx
	leaq	1944(%rsp), %rdi
	movq	%r14, %rcx
	movq	%r15, %rsi
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7268:
	movb	$1, %al
	movl	%eax, 16(%rsp)
.Ltmp7269:
	movb	$1, %al
	movq	%r15, %rdi
	movq	%r13, 872(%rsp)
	movq	%r13, %rsi
	movq	%rbx, %rdx
	movl	%eax, 172(%rsp)
	callq	purrdf_sparql_eval::vm::program_at::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7270:
	movq	120(%rsp), %rcx
	movb	$1, %dl
	movl	%edx, 172(%rsp)
	addq	$16, %rcx
.Ltmp7271:
	leaq	2144(%rsp), %r13
	movb	$1, %dl
	movq	%rax, %rsi
	movq	%r15, %r8
	movl	%edx, 16(%rsp)
	movq	%r13, %rdi
	movq	%rbx, %rdx
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::link::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7272:
	movq	%r12, 112(%rsp)
	testb	%bpl, %bpl
	je	.LBB242_44
	movq	2128(%rsp), %rax
	movq	424(%rsp), %rbx
	cmpq	%r14, %rax
	cmovbq	%rax, %r14
.Ltmp7283:
	movq	32(%rsp), %rdi
	movq	%r14, %rsi
	callq	<purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot
.Ltmp7284:
	movq	32(%rsp), %rcx
	leaq	1944(%rsp), %rsi
	movq	%rax, 160(%rsp)
	movq	616(%rcx), %rcx
	testq	%rcx, %rcx
	je	.LBB242_70
	cmpq	$-2, 16(%rcx)
	movb	$1, %al
	jb	.LBB242_71
	cmpq	$-2, 40(%rcx)
	setb	%al
	jmp	.LBB242_71
.LBB242_44:
	movq	424(%rsp), %rbx
	movq	416(%rsp), %rcx
	leaq	(%r14,%r14,4), %rax
	movabsq	$9223372036854775807, %r12
	movq	$0, 1120(%rsp)
	movq	$8, 1128(%rsp)
	movq	$0, 1136(%rsp)
	leaq	(%rbx,%rax,8), %r15
	movq	%rbx, 304(%rsp)
	movq	%rcx, 320(%rsp)
	movq	%rcx, 752(%rsp)
	movq	%rbx, 24(%rsp)
	movq	%r15, 328(%rsp)
	testq	%r14, %r14
	je	.LBB242_88
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	24(%rsp), %rbx
	movl	$8, %eax
	movq	$0, 16(%rsp)
	movq	%rax, 40(%rsp)
	jmp	.LBB242_49
.LBB242_46:
	movq	1128(%rsp), %rax
	movq	%rax, 40(%rsp)
.LBB242_47:
	movq	16(%rsp), %rsi
	movq	40(%rsp), %rdx
	leaq	(%rsi,%rsi,4), %rax
	incq	%rsi
	movq	%rsi, 16(%rsp)
	movq	%r13, (%rdx,%rax,8)
	movq	%r14, 8(%rdx,%rax,8)
	vmovdqa	448(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rdx,%rax,8)
	movq	464(%rsp), %rcx
	movq	%rcx, 32(%rdx,%rax,8)
	movq	%rsi, 1136(%rsp)
.LBB242_48:
	cmpq	%r15, %rbx
	je	.LBB242_87
.LBB242_49:
	vmovups	8(%rbx), %ymm0
	movq	(%rbx), %r13
	addq	$40, %rbx
	vmovups	%ymm0, 176(%rsp)
	testq	%r13, %r13
	je	.LBB242_88
	vmovdqu	176(%rsp), %ymm0
	leaq	1416(%rsp), %rax
	movq	%r13, 1408(%rsp)
	vmovdqu	%ymm0, (%rax)
.Ltmp7273:
	movq	32(%rsp), %rdx
	leaq	448(%rsp), %rdi
	leaq	1944(%rsp), %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7274:
	cmpb	$-1, 448(%rsp)
	jne	.LBB242_84
	movq	1424(%rsp), %rcx
	movq	1416(%rsp), %r14
	movq	120(%rsp), %r8
	leaq	-1(%r13), %rax
	leaq	1416(%rsp), %rdx
	decq	%rcx
	cmpq	$5, %rax
	cmovaeq	%r14, %rdx
	cmovbq	%rax, %rcx
	addq	$16, %r8
.Ltmp7275:
	movq	32(%rsp), %r9
	leaq	448(%rsp), %rdi
	leaq	2144(%rsp), %rsi
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7276:
	movq	448(%rsp), %rax
	movl	456(%rsp), %edx
	movl	460(%rsp), %ecx
	cmpq	$-1, %rax
	jne	.LBB242_86
	cmpl	$2, %edx
	je	.LBB242_58
.Ltmp7277:
	movq	32(%rsp), %rsi
	leaq	448(%rsp), %rdi
	callq	purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7278:
	movq	448(%rsp), %rax
	movzbl	456(%rsp), %edx
	cmpq	$-1, %rax
	jne	.LBB242_152
	testb	$1, %dl
	jne	.LBB242_68
.LBB242_58:
	cmpq	$6, %r13
	jb	.LBB242_48
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	leaq	-8(,%r13,8), %rcx
	cmpq	%r12, %rcx
	cmovaeq	%r12, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%r12, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB242_61
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB242_61:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB242_67
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB242_61
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r12, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rbp), %rax
	.p2align	4
.LBB242_64:
	cmpq	%rax, %rdx
	jge	.LBB242_66
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB242_64
.LBB242_66:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB242_67:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
	jmp	.LBB242_48
.LBB242_68:
	leaq	184(%rsp), %rcx
	vmovdqu	(%rcx), %xmm0
	movq	16(%rcx), %rax
	movq	16(%rsp), %rcx
	movq	%rax, 464(%rsp)
	vmovdqa	%xmm0, 448(%rsp)
	cmpq	1120(%rsp), %rcx
	jne	.LBB242_47
.Ltmp7280:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	1120(%rsp), %rdi
	callq	*%rax
.Ltmp7281:
	jmp	.LBB242_46
.LBB242_70:
	xorl	%eax, %eax
.LBB242_71:
	movq	32(%rsp), %rdx
	movq	%rbx, 176(%rsp)
	movq	%r14, 184(%rsp)
	movzbl	1234(%rdx), %ecx
	movq	%rdx, 1408(%rsp)
	leaq	160(%rsp), %rdx
	movq	%rdx, 1416(%rsp)
	leaq	120(%rsp), %rdx
	movq	%rsi, 1424(%rsp)
	movq	%r13, 1432(%rsp)
	movq	%rdx, 192(%rsp)
	testb	%al, %al
	je	.LBB242_73
.Ltmp7287:
	movq	%rsi, (%rsp)
	movzbl	%cl, %esi
	leaq	448(%rsp), %rdi
	leaq	1408(%rsp), %r8
	leaq	176(%rsp), %r9
	movq	%rbx, %rdx
	movq	%r14, %rcx
	callq	purrdf_sparql_eval::parallel::par_blocks_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>
.Ltmp7288:
	jmp	.LBB242_74
.LBB242_73:
.Ltmp7285:
	movq	%rsi, (%rsp)
	movzbl	%cl, %esi
	leaq	448(%rsp), %rdi
	leaq	1408(%rsp), %r8
	leaq	176(%rsp), %r9
	movq	%rbx, %rdx
	movq	%r14, %rcx
	callq	purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>
.Ltmp7286:
.LBB242_74:
	movq	448(%rsp), %r14
	cmpq	$-1, %r14
	je	.LBB242_79
	vmovups	640(%rsp), %zmm0
	vmovups	624(%rsp), %zmm1
	movq	456(%rsp), %rsi
	movq	472(%rsp), %rax
	movq	488(%rsp), %rdx
	movq	464(%rsp), %r13
	movq	480(%rsp), %rcx
	movq	%r14, 1328(%rsp)
	movl	$1, %edi
	movq	%rsi, 1336(%rsp)
	movq	%rsi, 96(%rsp)
	leaq	-3(%rax), %rsi
	movq	%r13, 1344(%rsp)
	cmpq	$-2, %rsi
	movl	$1, %esi
	cmovbq	%rax, %rdi
	cmovbq	%rdx, %rax
	cmovaeq	%rdx, %rsi
	vmovups	%zmm0, 1024(%rsp)
	vmovups	%zmm1, 1008(%rsp)
	vmovdqu64	496(%rsp), %zmm0
	vmovdqu64	560(%rsp), %zmm1
	decq	%rax
	vmovdqu64	1024(%rsp), %zmm3
	vmovdqu64	1008(%rsp), %zmm2
	vmovdqu64	%zmm0, 880(%rsp)
	vmovdqu64	%zmm1, 944(%rsp)
	vmovdqu64	%zmm1, 536(%rsp)
	vmovdqu64	%zmm0, 472(%rsp)
	vmovdqu64	%zmm3, 616(%rsp)
	vmovdqu64	%zmm2, 600(%rsp)
	movq	%rdi, 448(%rsp)
	movq	%rcx, 456(%rsp)
	movq	%rsi, 464(%rsp)
	movq	$0, 680(%rsp)
	movq	%rax, 688(%rsp)
.Ltmp7290:
	leaq	880(%rsp), %rdi
	leaq	448(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp7291:
	vmovups	1056(%rsp), %zmm2
	vmovups	1040(%rsp), %zmm1
	movq	32(%rsp), %rcx
	vmovdqu	880(%rsp), %ymm0
	cmpb	$2, 2138(%rsp)
	movq	616(%rcx), %rbx
	setne	%al
	vmovups	%zmm2, 1552(%rsp)
	vmovups	%zmm1, 1536(%rsp)
	vmovdqu64	976(%rsp), %zmm2
	vmovdqu64	912(%rsp), %zmm1
	vmovdqu	%ymm0, 1264(%rsp)
	testq	%rbx, %rbx
	sete	%cl
	orb	%al, %cl
	vmovdqu64	%zmm2, 1472(%rsp)
	vmovdqu64	%zmm1, 1408(%rsp)
	cmpb	$1, %cl
	jne	.LBB242_80
.Ltmp7409:
	leaq	1408(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp7410:
	movq	96(%rsp), %r12
	movl	$0, 24(%rsp)
	jmp	.LBB242_499
.LBB242_79:
	vmovdqu64	496(%rsp), %zmm0
	vmovdqu	464(%rsp), %ymm1
	vmovdqu64	%zmm0, 48(%r12)
	vmovdqu	%ymm1, 16(%r12)
	vmovdqu64	%zmm0, 880(%rsp)
	movq	$1, (%r12)
	movq	160(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB242_237
	jmp	.LBB242_239
.LBB242_80:
	movq	96(%rsp), %rax
	movq	%r14, 1304(%rsp)
	vmovdqu64	1552(%rsp), %zmm0
	vmovdqu64	1432(%rsp), %zmm1
	vmovdqu64	1496(%rsp), %zmm2
	movq	1424(%rsp), %rdx
	movq	1416(%rsp), %rcx
	movl	$1, %edi
	movl	$1, %esi
	movq	%rax, 1312(%rsp)
	movq	1408(%rsp), %rax
	movq	%r13, 1320(%rsp)
	vmovdqu64	%zmm0, 592(%rsp)
	vmovdqu64	%zmm2, 536(%rsp)
	vmovdqu64	%zmm1, 472(%rsp)
	cmpq	$3, %rax
	cmovaeq	%rax, %rdi
	cmovaeq	%rdx, %rax
	cmovaeq	%rsi, %rdx
	decq	%rax
	movq	%rdi, 448(%rsp)
	movq	%rcx, 456(%rsp)
	movq	%rdx, 464(%rsp)
	movq	$0, 656(%rsp)
	movq	%rax, 664(%rsp)
.Ltmp7293:
	leaq	1352(%rsp), %rdi
	leaq	448(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp7294:
	movq	1368(%rsp), %rcx
	movq	1360(%rsp), %rbp
	imulq	$200, %rcx, %rax
	addq	%rbp, %rax
	movq	%rax, 40(%rsp)
	testq	%rcx, %rcx
	je	.LBB242_141
	movl	%ecx, %esi
	andl	$3, %esi
	cmpq	$4, %rcx
	jae	.LBB242_116
	xorl	%edi, %edi
	xorl	%r10d, %r10d
	jmp	.LBB242_135
.LBB242_84:
	movq	%rbx, 312(%rsp)
	cmpq	$6, %r13
	jb	.LBB242_89
	movq	1416(%rsp), %rdi
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB242_89
.LBB242_86:
	vmovups	480(%rsp), %zmm1
	vmovups	464(%rsp), %zmm0
	movl	%edx, %esi
	shrl	$8, %esi
	vmovups	%zmm1, 896(%rsp)
	vmovups	%zmm0, 880(%rsp)
	jmp	.LBB242_153
.LBB242_87:
	movq	%r15, %rbx
.LBB242_88:
	movq	%rbx, 312(%rsp)
.LBB242_89:
	subq	%rbx, %r15
	je	.LBB242_102
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	shrq	$3, %r15
	movabsq	$-3689348814741910323, %r14
	imulq	%r15, %r14
	xorl	%r15d, %r15d
	jmp	.LBB242_94
	.p2align	4
.LBB242_91:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB242_92:
	vzeroupper
	callq	*%r13
.LBB242_93:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB242_102
.LBB242_94:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB242_93
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%r12, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r12, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r12, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB242_97
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB242_97:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB242_92
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB242_97
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r12, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rbp), %rax
	.p2align	4
.LBB242_100:
	cmpq	%rax, %rdx
	jge	.LBB242_91
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB242_100
	jmp	.LBB242_91
.LBB242_102:
	movq	752(%rsp), %rax
	movq	872(%rsp), %rbx
	movq	32(%rsp), %r14
	testq	%rax, %rax
	je	.LBB242_112
	shlq	$3, %rax
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%r12, %rcx
	cmovaeq	%r12, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%r12, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB242_105
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB242_105:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB242_111
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB242_105
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r12, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB242_108:
	cmpq	%rax, %rdx
	jge	.LBB242_110
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB242_108
.LBB242_110:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB242_111:
	movq	free@GOTPCREL(%rip), %rax
	movq	24(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB242_112:
	vmovdqu	1120(%rsp), %xmm0
	movq	1136(%rsp), %rax
	movq	112(%rsp), %r12
	movl	$0, 16(%rsp)
	movq	%rax, 864(%rsp)
	vmovdqa	%xmm0, 848(%rsp)
	movq	696(%r14), %rax
	movl	40(%rax), %ecx
	testl	%ecx, %ecx
	jne	.LBB242_113
	jmp	.LBB242_520
.LBB242_116:
	movq	%rcx, %r8
	andq	$-4, %r8
	leaq	776(%rbp), %r9
	xorl	%edi, %edi
	xorl	%r10d, %r10d
	jmp	.LBB242_118
	.p2align	4
.LBB242_117:
	addq	$4, %rdi
	addq	$800, %r9
	cmpq	%rdi, %r8
	je	.LBB242_134
.LBB242_118:
	movq	-600(%r9), %rax
	mulq	-608(%r9)
	jo	.LBB242_127
	addq	%rax, %r10
	movq	$-1, %r11
	jb	.LBB242_120
.LBB242_128:
	movq	%r10, %r11
	movq	-400(%r9), %rax
	mulq	-408(%r9)
	jno	.LBB242_121
.LBB242_129:
	movq	$-1, %rax
	addq	%rax, %r11
	movq	$-1, %r10
	jae	.LBB242_130
	.p2align	4
.LBB242_122:
	movq	-200(%r9), %rax
	mulq	-208(%r9)
	jo	.LBB242_131
.LBB242_123:
	addq	%rax, %r10
	movq	$-1, %r11
	jb	.LBB242_124
.LBB242_132:
	movq	%r10, %r11
	movq	(%r9), %rax
	mulq	-8(%r9)
	jno	.LBB242_125
.LBB242_133:
	movq	$-1, %rax
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB242_117
	jmp	.LBB242_126
.LBB242_127:
	movq	$-1, %rax
	addq	%rax, %r10
	movq	$-1, %r11
	jae	.LBB242_128
	.p2align	4
.LBB242_120:
	movq	-400(%r9), %rax
	mulq	-408(%r9)
	jo	.LBB242_129
.LBB242_121:
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB242_122
.LBB242_130:
	movq	%r11, %r10
	movq	-200(%r9), %rax
	mulq	-208(%r9)
	jno	.LBB242_123
.LBB242_131:
	movq	$-1, %rax
	addq	%rax, %r10
	movq	$-1, %r11
	jae	.LBB242_132
	.p2align	4
.LBB242_124:
	movq	(%r9), %rax
	mulq	-8(%r9)
	jo	.LBB242_133
.LBB242_125:
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB242_117
.LBB242_126:
	movq	%r11, %r10
	jmp	.LBB242_117
.LBB242_134:
	testq	%rsi, %rsi
	je	.LBB242_138
.LBB242_135:
	imulq	$200, %rdi, %rax
	imulq	$200, %rsi, %rsi
	movq	$-1, %r9
	xorl	%r8d, %r8d
	leaq	176(%rax,%rbp), %rdi
	.p2align	4
.LBB242_136:
	movq	(%rdi,%r8), %rax
	mulq	-8(%rdi,%r8)
	jo	.LBB242_813
.LBB242_137:
	addq	%rax, %r10
	cmovbq	%r9, %r10
	addq	$200, %r8
	cmpq	%r8, %rsi
	jne	.LBB242_136
	jmp	.LBB242_138
.LBB242_813:
	movq	$-1, %rax
	jmp	.LBB242_137
.LBB242_138:
	testq	%r10, %r10
	je	.LBB242_141
	cmpq	$0, 336(%rbx)
	je	.LBB242_141
	lock		addq	%r10, 352(%rbx)
.LBB242_141:
	movq	1352(%rsp), %rax
	movq	40(%rsp), %rdx
	movq	%rbp, 880(%rsp)
	movq	$0, 176(%rsp)
	movq	$8, 184(%rsp)
	movq	%r13, 16(%rsp)
	movq	$0, 192(%rsp)
	movq	%r14, 704(%rsp)
	movq	%rax, 896(%rsp)
	movq	%rdx, 904(%rsp)
	testq	%rcx, %rcx
	je	.LBB242_814
	leaq	456(%rsp), %r14
	addq	$200, %rbp
	movl	$8, %ecx
	xorl	%r15d, %r15d
	xorl	%r12d, %r12d
	.p2align	4
.LBB242_143:
	movq	-200(%rbp), %rax
	cmpq	$-1, %rax
	je	.LBB242_150
	leaq	-200(%rbp), %rbx
	vmovups	8(%rbx), %zmm0
	vmovups	72(%rbx), %zmm1
	vmovups	96(%rbx), %zmm2
	vmovups	%zmm2, 88(%r14)
	vmovups	%zmm1, 64(%r14)
	vmovups	%zmm0, (%r14)
	movq	%rax, 448(%rsp)
	movq	%r15, %rax
	movq	%rax, %r13
	movzbl	600(%rsp), %r15d
	cmpq	176(%rsp), %rax
	jne	.LBB242_147
.Ltmp7296:
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp7297:
	movq	184(%rsp), %rcx
.LBB242_147:
	vmovdqu64	448(%rsp), %zmm0
	vmovdqu64	512(%rsp), %zmm1
	vmovdqu64	544(%rsp), %zmm2
	leaq	1(%r13), %rax
	vmovdqu64	%zmm2, 96(%rcx,%r12)
	vmovdqu64	%zmm1, 64(%rcx,%r12)
	vmovdqu64	%zmm0, (%rcx,%r12)
	movq	%rax, 192(%rsp)
	testb	%r15b, %r15b
	jne	.LBB242_186
	addq	$160, %r12
	addq	$200, %rbp
	addq	$200, %rbx
	movq	%rax, %r15
	cmpq	40(%rsp), %rbx
	jne	.LBB242_143
	movq	40(%rsp), %rbp
	movq	%rax, %r15
.LBB242_150:
	movq	%rcx, %r12
	jmp	.LBB242_151
.LBB242_814:
	movl	$8, %r12d
	xorl	%r15d, %r15d
.LBB242_151:
	movq	%rbp, 888(%rsp)
	xorl	%ebp, %ebp
	jmp	.LBB242_187
.LBB242_152:
	movzbl	459(%rsp), %ecx
	movzwl	457(%rsp), %esi
	vmovups	464(%rsp), %zmm0
	vmovups	480(%rsp), %zmm1
	shll	$16, %ecx
	orl	%ecx, %esi
	movl	460(%rsp), %ecx
	vmovups	%zmm0, 880(%rsp)
	vmovups	%zmm1, 896(%rsp)
.LBB242_153:
	vmovdqu64	880(%rsp), %zmm0
	vmovdqu64	896(%rsp), %zmm1
	movq	112(%rsp), %rdi
	movw	%si, 25(%rdi)
	shrl	$16, %esi
	movb	%sil, 27(%rdi)
	movl	%ecx, 28(%rdi)
	vmovdqu64	%zmm0, 32(%rdi)
	vmovdqu64	%zmm1, 48(%rdi)
	movq	%rax, 16(%rdi)
	movb	%dl, 24(%rdi)
	movq	$1, (%rdi)
	cmpq	$6, %r13
	jb	.LBB242_155
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
	movq	%r14, %rdi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB242_155:
	subq	%rbx, %r15
	je	.LBB242_168
	shrq	$3, %r15
	movabsq	$-3689348814741910323, %r14
	imulq	%r15, %r14
	xorl	%r15d, %r15d
	jmp	.LBB242_160
.LBB242_157:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB242_158:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB242_159:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB242_168
.LBB242_160:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB242_159
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%r12, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r12, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r12, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB242_163
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB242_163:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB242_158
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB242_163
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r12, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rbp), %rax
	.p2align	4
.LBB242_166:
	cmpq	%rax, %rdx
	jge	.LBB242_157
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB242_166
	jmp	.LBB242_157
.LBB242_168:
	movq	752(%rsp), %rax
	testq	%rax, %rax
	je	.LBB242_170
	movq	24(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB242_170:
	movq	16(%rsp), %r15
	movq	40(%rsp), %r14
	testq	%r15, %r15
	je	.LBB242_183
	xorl	%ebx, %ebx
	jmp	.LBB242_175
.LBB242_172:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB242_173:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB242_174:
	incq	%rbx
	cmpq	%r15, %rbx
	je	.LBB242_183
.LBB242_175:
	leaq	(%rbx,%rbx,4), %rcx
	movq	(%r14,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB242_174
	leaq	(%r14,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%r12, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r12, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r12, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB242_178
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB242_178:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB242_173
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB242_178
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r12, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rbp), %rax
	.p2align	4
.LBB242_181:
	cmpq	%rax, %rdx
	jge	.LBB242_172
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB242_181
	jmp	.LBB242_172
.LBB242_183:
	movq	1120(%rsp), %rax
	testq	%rax, %rax
	je	.LBB242_185
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r14, %rdi
	leaq	(%rax,%rax,4), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB242_185:
	movq	112(%rsp), %r12
	movl	$0, 16(%rsp)
	jmp	.LBB242_579
.LBB242_186:
	movq	%rbp, 888(%rsp)
	incq	%r13
	movb	$1, %bpl
	movq	%rcx, %r12
	movq	%r13, %r15
.LBB242_187:
.Ltmp7304:
	leaq	880(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp7305:
	movq	176(%rsp), %r13
	testq	%r15, %r15
	je	.LBB242_191
	cmpq	$8, %r15
	jae	.LBB242_192
	xorl	%eax, %eax
	xorl	%ebx, %ebx
	jmp	.LBB242_201
.LBB242_191:
	xorl	%ebx, %ebx
	jmp	.LBB242_203
.LBB242_192:
	cmpq	$32, %r15
	jae	.LBB242_194
	xorl	%eax, %eax
	xorl	%ebx, %ebx
	jmp	.LBB242_198
.LBB242_194:
	vmovdqa64	.LCPI242_0(%rip), %zmm1
	vpbroadcastq	.LCPI242_1(%rip), %zmm2
	vpbroadcastq	.LCPI242_2(%rip), %zmm3
	movq	%r15, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
	.p2align	4
.LBB242_195:
	vpmullq	%zmm2, %zmm1, %zmm7
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm8, %xmm8, %xmm8
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$-32, %rcx
	vpgatherqq	64(%r12,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm0, %zmm8, %zmm0
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	1344(%r12,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm4, %zmm8, %zmm4
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	2624(%r12,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm5, %zmm8, %zmm5
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	3904(%r12,%zmm7), %zmm8 {%k1}
	vpaddq	%zmm6, %zmm8, %zmm6
	jne	.LBB242_195
	vpaddq	%zmm0, %zmm4, %zmm0
	vpaddq	%zmm0, %zmm5, %zmm0
	vpaddq	%zmm0, %zmm6, %zmm0
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rbx
	cmpq	%rax, %r15
	je	.LBB242_203
	testb	$24, %r15b
	je	.LBB242_201
.LBB242_198:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI242_0(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI242_1(%rip), %zmm2
	vpbroadcastq	.LCPI242_3(%rip), %zmm3
	movq	%r15, %rax
	andq	$-8, %rax
	vmovq	%rbx, %xmm0
	subq	%rax, %rcx
	.p2align	4
.LBB242_199:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	64(%r12,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB242_199
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rbx
	cmpq	%rax, %r15
	je	.LBB242_203
.LBB242_201:
	movq	%r15, %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	64(%rax,%r12), %rax
	.p2align	4
.LBB242_202:
	addq	(%rax), %rbx
	addq	$160, %rax
	decq	%rcx
	jne	.LBB242_202
.LBB242_203:
	movzbl	2139(%rsp), %eax
	movzbl	2137(%rsp), %r14d
	movq	%r13, 1376(%rsp)
	movq	%r12, 1384(%rsp)
	movq	%r15, 1392(%rsp)
	movq	$0, 880(%rsp)
	movq	$8, 888(%rsp)
	movb	%bpl, 1400(%rsp)
	movq	$0, 896(%rsp)
	movq	%rax, 1192(%rsp)
.Ltmp7310:
	movq	16(%rsp), %rdx
	leaq	448(%rsp), %rdi
	leaq	880(%rsp), %rsi
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp7311:
	movq	448(%rsp), %rcx
	movq	456(%rsp), %rax
	movq	464(%rsp), %rdx
	movq	472(%rsp), %rdi
	movq	%r15, 752(%rsp)
	movq	%rbx, 1216(%rsp)
	movl	%ebp, 92(%rsp)
	movq	%r13, 48(%rsp)
	movq	%rcx, 104(%rsp)
	cmpq	$-1, %rcx
	je	.LBB242_240
	vmovups	496(%rsp), %ymm0
	movq	%rdx, 40(%rsp)
	movzbl	487(%rsp), %edx
	movzwl	485(%rsp), %ecx
	movq	%rax, 80(%rsp)
	movzbl	480(%rsp), %eax
	movl	481(%rsp), %r13d
	movq	96(%rsp), %rbx
	movq	%rdi, 24(%rsp)
	movq	%r12, 64(%rsp)
	shll	$16, %edx
	movl	%eax, 72(%rsp)
	orl	%edx, %ecx
	shlq	$32, %rcx
	cmpq	$0, 16(%rsp)
	vmovups	%ymm0, 1120(%rsp)
	vmovdqu	512(%rsp), %ymm0
	movq	%rcx, 152(%rsp)
	movq	488(%rsp), %rcx
	movq	%rcx, 136(%rsp)
	vmovdqu	%ymm0, 1136(%rsp)
	je	.LBB242_218
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r12
	movabsq	$9223372036854775807, %r15
	xorl	%r14d, %r14d
	jmp	.LBB242_210
.LBB242_207:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB242_208:
	vzeroupper
	callq	*%r12
.LBB242_209:
	incq	%r14
	cmpq	16(%rsp), %r14
	je	.LBB242_218
.LBB242_210:
	leaq	(%r14,%r14,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB242_209
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%r15, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r15, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r15, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB242_213
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB242_213:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB242_208
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB242_213
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r15, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rbp), %rax
	.p2align	4
.LBB242_216:
	cmpq	%rax, %rdx
	jge	.LBB242_207
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB242_216
	jmp	.LBB242_207
.LBB242_218:
	addq	152(%rsp), %r13
	movq	704(%rsp), %rax
	testq	%rax, %rax
	je	.LBB242_220
	shlq	$3, %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	leaq	(%rax,%rax,4), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB242_220:
	shlq	$8, %r13
	movq	112(%rsp), %r12
	movl	92(%rsp), %ebp
	movq	752(%rsp), %r15
	movq	104(%rsp), %rbx
	movq	%r13, 16(%rsp)
	movq	32(%rsp), %r13
	testq	%r15, %r15
	je	.LBB242_224
.LBB242_221:
	movq	64(%rsp), %r14
	movq	%r15, %rax
	movl	$1, %r15d
	subq	%rax, %r15
	.p2align	4
.LBB242_222:
.Ltmp7397:
	movq	%r14, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7398:
	incq	%r15
	addq	$160, %r14
	cmpq	$1, %r15
	jne	.LBB242_222
.LBB242_224:
	movq	48(%rsp), %rax
	testq	%rax, %rax
	je	.LBB242_234
	shlq	$5, %rax
	movabsq	$9223372036854775807, %rdx
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
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
	jge	.LBB242_227
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB242_227:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB242_233
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB242_227
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
.LBB242_230:
	cmpq	%rax, %rsi
	jge	.LBB242_232
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB242_230
.LBB242_232:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB242_233:
	movq	free@GOTPCREL(%rip), %rax
	movq	64(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB242_234:
	cmpq	$-1, %rbx
	je	.LBB242_246
	vmovdqu	1136(%rsp), %ymm1
	vmovdqu	1120(%rsp), %ymm0
	movq	80(%rsp), %rdx
	movzbl	72(%rsp), %eax
	movq	16(%rsp), %rcx
	movq	40(%rsp), %rsi
	orq	%rax, %rcx
	vmovdqu	%ymm1, 80(%r12)
	vmovdqu	%ymm0, 64(%r12)
	movq	%rbx, 16(%r12)
	movq	%rdx, 24(%r12)
	movq	24(%rsp), %rdx
	movq	%rsi, 32(%r12)
	movq	%rdx, 40(%r12)
	movq	%rcx, 48(%r12)
	movq	136(%rsp), %rcx
	movq	%rcx, 56(%r12)
	movq	$1, (%r12)
.Ltmp7403:
	leaq	1264(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7404:
	movq	160(%rsp), %rax
	testq	%rax, %rax
	je	.LBB242_239
.LBB242_237:
	lock		decq	(%rax)
	jne	.LBB242_239
	#MEMBARRIER
.Ltmp7462:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp7463:
.LBB242_239:
	movb	$1, %al
	movl	%eax, 16(%rsp)
	jmp	.LBB242_579
.LBB242_240:
	movq	%rax, 264(%rsp)
	movq	%rax, %r8
	movq	16(%rsp), %rax
	movq	96(%rsp), %rcx
	movq	704(%rsp), %rsi
	movq	32(%rsp), %r13
	movq	%rdx, 272(%rsp)
	movq	%rdi, 280(%rsp)
	leaq	(%rax,%rax,4), %rax
	movq	%rcx, 816(%rsp)
	movq	%rsi, 832(%rsp)
	movq	%rcx, 824(%rsp)
	leaq	(%rcx,%rax,8), %rax
	movq	%rax, 840(%rsp)
	movq	616(%r13), %rax
	testq	%rax, %rax
	je	.LBB242_247
	movb	%r14b, 91(%rsp)
	lock		incq	(%rax)
	jle	.LBB242_696
	movq	616(%r13), %r14
	movq	48(%rsp), %r13
	movq	%r14, 808(%rsp)
	movq	%r14, 24(%rsp)
	movq	16(%r14), %rax
	movq	40(%r14), %rcx
	movq	%rax, 288(%rsp)
	movq	%rcx, 136(%rsp)
	cmpq	$-1, %rcx
	je	.LBB242_270
	testq	%r15, %r15
	je	.LBB242_254
	movq	32(%rsp), %rdx
	cmpq	$8, %r15
	jae	.LBB242_255
	xorl	%eax, %eax
	xorl	%esi, %esi
	jmp	.LBB242_267
.LBB242_246:
	movq	%r13, %r15
	movq	24(%rsp), %r13
	movq	80(%rsp), %r14
	movq	40(%rsp), %r12
	jmp	.LBB242_488
.LBB242_247:
	movq	824(%rsp), %rcx
	movq	%rdx, 40(%rsp)
	movq	816(%rsp), %rax
	movq	832(%rsp), %rdx
	movq	%r12, 64(%rsp)
	movq	%r8, 80(%rsp)
	movq	%rdi, 24(%rsp)
	movq	%rcx, 456(%rsp)
	movq	840(%rsp), %rcx
	movq	%rax, 448(%rsp)
	movq	%rdx, 464(%rsp)
	movq	%rcx, 472(%rsp)
	movq	472(%rsp), %rax
	movq	456(%rsp), %r12
	movq	%rax, 96(%rsp)
	cmpq	%rax, %r12
	je	.LBB242_259
	movq	24(%rsp), %rax
	addq	$40, %r12
	movq	%r12, %rcx
	leaq	(,%rax,8), %rax
	leaq	(%rax,%rax,4), %r14
	leaq	264(%rsp), %rax
	movq	%rax, 80(%rsp)
	jmp	.LBB242_250
.LBB242_249:
	movq	40(%rsp), %rdx
	movq	16(%rsp), %rcx
	shll	$16, %r13d
	movq	704(%rsp), %rsi
	leaq	-40(%r12), %rax
	orl	%r13d, %r15d
	movq	32(%rsp), %r13
	addq	$40, %rax
	shlq	$32, %r15
	orq	%r15, %rbx
	movq	752(%rsp), %r15
	movq	%rbp, (%rdx,%r14)
	movq	%rcx, 8(%rdx,%r14)
	movq	72(%rsp), %rcx
	movq	%rcx, 16(%rdx,%r14)
	movzbl	104(%rsp), %ecx
	movb	%cl, 24(%rdx,%r14)
	movq	%rbx, %rcx
	shrq	$48, %rcx
	movl	%ebx, 25(%rdx,%r14)
	shrq	$32, %rbx
	movb	%cl, 31(%rdx,%r14)
	movq	24(%rsp), %rcx
	movw	%bx, 29(%rdx,%r14)
	movq	%rsi, 32(%rdx,%r14)
	addq	$40, %r14
	incq	%rcx
	movq	%rcx, 24(%rsp)
	movq	%rcx, 280(%rsp)
	leaq	40(%r12), %rcx
	cmpq	96(%rsp), %rax
	je	.LBB242_257
.LBB242_250:
	movq	-40(%rcx), %rbp
	movq	264(%rsp), %rax
	movq	%rcx, %r12
	testq	%rbp, %rbp
	je	.LBB242_258
	movq	-32(%r12), %rcx
	movq	-24(%r12), %rdi
	movzbl	-16(%r12), %esi
	movq	-8(%r12), %rdx
	movzwl	-11(%r12), %r15d
	movzbl	-9(%r12), %r13d
	movl	-15(%r12), %ebx
	movq	%rcx, 16(%rsp)
	movq	%rdi, 72(%rsp)
	movb	%sil, 104(%rsp)
	movq	%rdx, 704(%rsp)
	cmpq	%rax, 24(%rsp)
	jne	.LBB242_249
.Ltmp7391:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	264(%rsp), %rdi
	callq	*%rax
.Ltmp7392:
	movq	272(%rsp), %rax
	movq	%rax, 40(%rsp)
	jmp	.LBB242_249
.LBB242_254:
	movq	32(%rsp), %rdx
	xorl	%esi, %esi
	jmp	.LBB242_269
.LBB242_255:
	cmpq	$32, %r15
	jae	.LBB242_260
	xorl	%eax, %eax
	xorl	%esi, %esi
	jmp	.LBB242_264
.LBB242_257:
	movq	264(%rsp), %rax
.LBB242_258:
	movq	272(%rsp), %rcx
	movl	92(%rsp), %ebp
	movq	%rax, 80(%rsp)
	movq	%rcx, 40(%rsp)
.LBB242_259:
	leaq	448(%rsp), %rdi
	movq	%r12, 456(%rsp)
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	112(%rsp), %r12
	movb	$2, %al
	movq	$-1, %rbx
	movq	$0, 16(%rsp)
	movl	%eax, 72(%rsp)
	testq	%r15, %r15
	jne	.LBB242_221
	jmp	.LBB242_224
.LBB242_260:
	vmovdqa64	.LCPI242_0(%rip), %zmm1
	vpbroadcastq	.LCPI242_1(%rip), %zmm2
	vpbroadcastq	.LCPI242_2(%rip), %zmm3
	movq	%r15, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
.LBB242_261:
	vpmullq	%zmm2, %zmm1, %zmm7
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm8, %xmm8, %xmm8
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$-32, %rcx
	vpgatherqq	16(%r12,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm0, %zmm8, %zmm0
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	1296(%r12,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm4, %zmm8, %zmm4
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	2576(%r12,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm5, %zmm8, %zmm5
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	3856(%r12,%zmm7), %zmm8 {%k1}
	vpaddq	%zmm6, %zmm8, %zmm6
	jne	.LBB242_261
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
	je	.LBB242_269
	testb	$24, %r15b
	je	.LBB242_267
.LBB242_264:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI242_0(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI242_1(%rip), %zmm2
	vpbroadcastq	.LCPI242_3(%rip), %zmm3
	movq	%r15, %rax
	andq	$-8, %rax
	vmovq	%rsi, %xmm0
	subq	%rax, %rcx
.LBB242_265:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	16(%r12,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB242_265
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rsi
	cmpq	%rax, %r15
	je	.LBB242_269
.LBB242_267:
	movq	%r15, %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	16(%rax,%r12), %rax
.LBB242_268:
	addq	(%rax), %rsi
	addq	$160, %rax
	decq	%rcx
	jne	.LBB242_268
.LBB242_269:
	movb	$1, %bl
	leaq	888(%rdx), %rdi
.Ltmp7313:
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts@GOTPCREL(%rip), %rax
	movb	$1, %bpl
	vzeroupper
	callq	*%rax
.Ltmp7314:
.LBB242_270:
	movq	96(%rsp), %rbx
	leaq	(%r15,%r15,4), %rcx
	leaq	16(%r14), %rax
	movq	%r12, 1224(%rsp)
	movq	%r12, 1232(%rsp)
	movq	%r13, 1240(%rsp)
	movq	%rax, 128(%rsp)
	shlq	$5, %rcx
	addq	%r12, %rcx
	movq	%rcx, 1168(%rsp)
	movq	%rcx, 1248(%rsp)
	testq	%r15, %r15
	je	.LBB242_443
	movq	32(%rsp), %rcx
	leaq	272(%r14), %rax
	movq	$-1, %r15
	movq	%rax, 800(%rsp)
	leaq	888(%rcx), %rax
	movq	%rax, 80(%rsp)
.LBB242_272:
	movq	%r12, %rcx
	addq	$160, %r12
	movq	%r12, 1232(%rsp)
	vmovups	96(%rcx), %zmm0
	movq	(%rcx), %rax
	vmovups	%zmm0, 968(%rsp)
	vmovups	72(%rcx), %zmm0
	vmovups	%zmm0, 944(%rsp)
	vmovups	8(%rcx), %zmm0
	vmovups	%zmm0, 880(%rsp)
	cmpq	$-1, %rax
	je	.LBB242_443
	vmovdqu64	880(%rsp), %zmm0
	vmovdqu64	944(%rsp), %zmm1
	vmovdqu64	968(%rsp), %zmm2
	leaq	456(%rsp), %rcx
	movq	%rax, 448(%rsp)
	vmovdqu64	%zmm2, 88(%rcx)
	vmovdqu64	%zmm1, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
	movq	456(%rsp), %rdx
	imulq	$88, 464(%rsp), %rdi
	movq	472(%rsp), %rcx
	movq	488(%rsp), %rsi
	movq	%rdx, 384(%rsp)
	movq	%rax, 400(%rsp)
	movq	496(%rsp), %rax
	movq	%rcx, 736(%rsp)
	movq	480(%rsp), %rcx
	movq	%rdx, 16(%rsp)
	movq	%rdx, 392(%rsp)
	movq	%rsi, 1200(%rsp)
	addq	%rdx, %rdi
	movq	504(%rsp), %rdx
	movq	%rdi, 48(%rsp)
	movq	%rdi, 408(%rsp)
	movq	%rax, 56(%rsp)
	movq	512(%rsp), %rax
	movq	%rcx, 752(%rsp)
	movq	%rdx, 792(%rsp)
	testq	%rax, %rax
	je	.LBB242_431
	movq	544(%rsp), %r13
	shlq	$5, %rax
	movq	$0, 744(%rsp)
	movq	%r12, 64(%rsp)
	addq	%rdx, %rax
	movq	%rax, 1184(%rsp)
	leaq	8(%rcx), %rax
	movq	%rax, 1176(%rsp)
	jmp	.LBB242_277
.LBB242_275:
	movq	96(%rsp), %rbx
.LBB242_276:
	movq	1208(%rsp), %rdx
	movq	%rbx, 824(%rsp)
	addq	$32, %rdx
	cmpq	1184(%rsp), %rdx
	je	.LBB242_431
.LBB242_277:
	movq	%rbx, 96(%rsp)
	movq	744(%rsp), %rax
	movq	16(%rdx), %rsi
	movq	24(%rdx), %rcx
	movq	(%rdx), %r14
	movq	8(%rdx), %rbx
	movq	%rdx, 1208(%rsp)
	movq	%rax, 296(%rsp)
	movq	%rsi, 144(%rsp)
	movq	%rcx, 40(%rsp)
	testq	%r14, %r14
	je	.LBB242_287
	cmpq	$-1, 288(%rsp)
	je	.LBB242_287
	movq	24(%rsp), %rdx
	movq	80(%rdx), %rax
	.p2align	4
.LBB242_280:
	movq	%rax, %rcx
	addq	%r14, %rcx
	cmovbq	%r15, %rcx
	lock		cmpxchgq	%rcx, 80(%rdx)
	jne	.LBB242_280
	movq	128(%rsp), %rcx
	addq	%r14, %rax
	cmovbq	%r15, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB242_284
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$0, 176(%rsp)
.Ltmp7315:
	movq	128(%rsp), %rsi
	leaq	304(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.2910935600939035342)
.Ltmp7316:
	cmpb	$-1, 304(%rsp)
	jne	.LBB242_467
.LBB242_284:
	movq	32(%rsp), %rax
	movq	632(%rax), %rax
	testq	%rax, %rax
	je	.LBB242_287
	movq	32(%rsp), %rcx
	movl	1228(%rcx), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB242_287
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	movq	1192(%rsp), %rax
	lock		addq	%r14, (%rcx,%rax,8)
.LBB242_287:
	movq	1200(%rsp), %rdx
	movq	296(%rsp), %rdi
	cmpq	%rdx, %rdi
	ja	.LBB242_694
	cmpq	%rdx, %rbx
	movq	%rdx, %rsi
	cmovbq	%rbx, %rsi
	cmpq	%rdi, %rbx
	cmovbq	%rdi, %rsi
	cmpq	%rdi, %rsi
	jb	.LBB242_693
	leaq	(,%rdi,8), %rax
	movq	%rsi, 744(%rsp)
	leaq	(%rax,%rax,2), %r14
	leaq	(,%rsi,8), %rax
	leaq	(%rax,%rax,2), %rbx
	cmpq	%rsi, %rdi
	jne	.LBB242_296
	movq	$0, 72(%rsp)
	movq	$0, 704(%rsp)
	movq	$0, 104(%rsp)
.LBB242_291:
	cmpq	$-1, 136(%rsp)
	movq	%r13, 152(%rsp)
	movq	$-1, %r13
	je	.LBB242_302
	movq	48(%rsp), %r15
	movq	144(%rsp), %rax
	movl	$0, %ecx
	movabsq	$3353953467947191203, %rdx
	movl	$0, %r12d
	subq	16(%rsp), %r15
	subq	152(%rsp), %rax
	cmovbq	%rcx, %rax
	shrq	$3, %r15
	imulq	%rdx, %r15
	cmpq	%r15, %rax
	cmovbq	%rax, %r15
	testq	%r15, %r15
	je	.LBB242_303
	movq	16(%rsp), %rax
	xorl	%r12d, %r12d
	leaq	8(%rax), %rbp
	.p2align	4
.LBB242_294:
.Ltmp7318:
	movq	purrdf_sparql_eval::scratch::value_bytes@GOTPCREL(%rip), %rax
	movq	%rbp, %rdi
	vzeroupper
	callq	*%rax
.Ltmp7319:
	addq	%rax, %r12
	cmovbq	%r13, %r12
	addq	$88, %rbp
	decq	%r15
	jne	.LBB242_294
	jmp	.LBB242_303
.LBB242_296:
	movq	%rbx, %rdx
	subq	%r14, %rdx
	movabsq	$-6148914691236517205, %rax
	mulxq	%rax, %rax, %rax
	movq	1176(%rsp), %rcx
	movq	$0, 104(%rsp)
	movq	$0, 72(%rsp)
	movq	$0, 704(%rsp)
	shrq	$4, %rax
	addq	%r14, %rcx
	jmp	.LBB242_299
	.p2align	4
.LBB242_297:
	movq	72(%rsp), %rsi
	addq	%rdx, %rsi
	cmovbq	%r15, %rsi
	movq	%rsi, 72(%rsp)
.LBB242_298:
	addq	$24, %rcx
	decq	%rax
	je	.LBB242_291
.LBB242_299:
	movzbl	-8(%rcx), %esi
	leaq	.LJTI242_0(%rip), %rdi
	movq	(%rcx), %rdx
	movslq	(%rdi,%rsi,4), %rsi
	addq	%rdi, %rsi
	jmpq	*%rsi
.LBB242_300:
	movq	704(%rsp), %rsi
	addq	%rdx, %rsi
	cmovbq	%r15, %rsi
	movq	%rsi, 704(%rsp)
	jmp	.LBB242_298
	.p2align	4
.LBB242_301:
	movq	104(%rsp), %rsi
	cmpq	%rdx, %rsi
	cmovbeq	%rdx, %rsi
	movq	%rsi, 104(%rsp)
	jmp	.LBB242_298
.LBB242_302:
	xorl	%r12d, %r12d
.LBB242_303:
	movq	24(%rsp), %rax
	movq	$-1, %r15
	movl	296(%rax), %eax
	testl	%eax, %eax
	je	.LBB242_319
.LBB242_304:
	cmpq	$-1, 288(%rsp)
	je	.LBB242_306
	movq	24(%rsp), %rcx
	movq	80(%rcx), %rax
	addq	704(%rsp), %rax
	cmovbq	%r15, %rax
	cmpq	16(%rcx), %rax
	ja	.LBB242_320
.LBB242_306:
	cmpq	$-1, 136(%rsp)
	je	.LBB242_308
	movq	32(%rsp), %rcx
	movq	1048(%rcx), %rax
	movq	1056(%rcx), %rcx
	movq	24(%rsp), %rdx
	subq	%rcx, %rax
	movl	$0, %ecx
	cmovaeq	%rax, %rcx
	addq	%r12, %rcx
	cmovbq	%r15, %rcx
	addq	72(%rsp), %rcx
	cmovbq	%r15, %rcx
	addq	104(%rsp), %rcx
	movq	104(%rdx), %rax
	cmovbq	%r15, %rcx
	addq	%rcx, %rax
	cmovbq	%r15, %rax
	cmpq	40(%rdx), %rax
	ja	.LBB242_320
.LBB242_308:
	cmpq	$-1, 288(%rsp)
	movq	32(%rsp), %r9
	movq	152(%rsp), %r13
	movq	744(%rsp), %r10
	je	.LBB242_310
	movq	%r14, %rax
	cmpq	%r10, 296(%rsp)
	jne	.LBB242_314
.LBB242_310:
	movq	64(%rsp), %r12
	movb	$1, %bpl
	cmpq	%r10, 296(%rsp)
	je	.LBB242_393
.LBB242_311:
	movq	752(%rsp), %rax
	cmpb	$2, -24(%rax,%rbx)
	je	.LBB242_378
	addq	$-24, %rbx
	cmpq	%rbx, %r14
	jne	.LBB242_311
	jmp	.LBB242_393
	.p2align	4
.LBB242_313:
	addq	$24, %rax
	cmpq	%rax, %rbx
	je	.LBB242_310
.LBB242_314:
	movq	752(%rsp), %rcx
	cmpb	$0, (%rcx,%rax)
	jne	.LBB242_313
	movq	752(%rsp), %rcx
	movzbl	1(%rcx,%rax), %ecx
	cmpq	$255, %rcx
	je	.LBB242_313
	movq	632(%r9), %rdx
	testq	%rdx, %rdx
	je	.LBB242_313
	movl	1228(%r9), %esi
	cmpq	%rsi, 56(%rdx)
	jbe	.LBB242_313
	movq	752(%rsp), %rdi
	movq	%rsi, %r8
	shlq	$7, %r8
	leaq	(%r8,%rsi,8), %rsi
	addq	48(%rdx), %rsi
	movq	8(%rdi,%rax), %rdi
	lock		addq	%rdi, (%rsi,%rcx,8)
	jmp	.LBB242_313
.LBB242_319:
	movq	800(%rsp), %rax
	cmpb	$-1, (%rax)
	je	.LBB242_304
.LBB242_320:
	movq	152(%rsp), %r13
	movq	296(%rsp), %rax
	cmpq	744(%rsp), %rax
	jne	.LBB242_330
	movq	64(%rsp), %r12
.LBB242_322:
	cmpq	$-1, 136(%rsp)
	je	.LBB242_377
	movq	24(%rsp), %r14
	cmpq	144(%rsp), %r13
	jae	.LBB242_422
	movq	16(%rsp), %rdx
	cmpq	48(%rsp), %rdx
	je	.LBB242_386
	movq	144(%rsp), %rax
	addq	$88, %rdx
	leaq	-1(%rax), %rbx
	movq	%rdx, %rax
.LBB242_326:
	movq	%rax, %rdx
	movq	-8(%rdx), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rdx, 16(%rsp)
	movq	%rcx, 368(%rsp)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, 304(%rsp)
	cmpq	$-1, %rax
	je	.LBB242_385
	vmovdqu64	304(%rsp), %zmm0
	movq	%rax, 176(%rsp)
	movq	368(%rsp), %rax
	leaq	184(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7362:
	movq	80(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp7363:
	cmpq	%r13, %rbx
	je	.LBB242_387
	movq	16(%rsp), %rdx
	incq	%r13
	leaq	-88(%rdx), %rcx
	leaq	88(%rdx), %rax
	addq	$88, %rcx
	cmpq	48(%rsp), %rcx
	jne	.LBB242_326
	jmp	.LBB242_386
.LBB242_330:
	movq	752(%rsp), %rax
	movq	64(%rsp), %r12
	addq	%rax, %r14
	addq	%rax, %rbx
	jmp	.LBB242_334
.LBB242_331:
	movq	64(%rsp), %r12
.LBB242_332:
	movq	$-1, %r15
.LBB242_333:
	addq	$24, %r14
	cmpq	%rbx, %r14
	je	.LBB242_322
.LBB242_334:
	movzbl	(%r14), %eax
	leaq	.LJTI242_1(%rip), %rcx
	movslq	(%rcx,%rax,4), %rax
	addq	%rcx, %rax
	jmpq	*%rax
.LBB242_335:
	cmpq	$-1, 288(%rsp)
	je	.LBB242_333
	movq	24(%rsp), %rdx
	movzbl	1(%r14), %r15d
	movq	8(%r14), %r12
	movq	16(%r14), %rbp
	movq	$-1, %rsi
	movq	80(%rdx), %rax
	.p2align	4
.LBB242_337:
	movq	%rax, %rcx
	addq	%r12, %rcx
	cmovbq	%rsi, %rcx
	lock		cmpxchgq	%rcx, 80(%rdx)
	jne	.LBB242_337
	movq	128(%rsp), %rcx
	addq	%r12, %rax
	cmovbq	%rsi, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB242_341
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$0, 176(%rsp)
.Ltmp7349:
	movq	128(%rsp), %rsi
	leaq	304(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.2910935600939035342)
.Ltmp7350:
	cmpb	$-1, 304(%rsp)
	jne	.LBB242_450
.LBB242_341:
	cmpl	$255, %r15d
	je	.LBB242_331
	movq	32(%rsp), %rcx
	movq	632(%rcx), %rax
	testq	%rax, %rax
	je	.LBB242_331
	movl	1228(%rcx), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB242_331
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	lock		addq	%r12, (%rcx,%r15,8)
	jmp	.LBB242_331
.LBB242_345:
	movq	8(%r14), %r15
	cmpq	%r15, %r13
	jae	.LBB242_374
	movq	16(%rsp), %rax
	cmpq	48(%rsp), %rax
	je	.LBB242_367
	addq	$88, %rax
	leaq	-1(%r15), %r12
	.p2align	4
.LBB242_348:
	movq	%rax, %rdx
	movq	-8(%rdx), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rdx, 16(%rsp)
	movq	%rcx, 368(%rsp)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, 304(%rsp)
	cmpq	$-1, %rax
	je	.LBB242_371
	vmovdqu64	304(%rsp), %zmm0
	movq	%rax, 176(%rsp)
	movq	368(%rsp), %rax
	leaq	184(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7338:
	movq	80(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp7339:
	cmpq	%r13, %r12
	je	.LBB242_373
	movq	16(%rsp), %rdx
	incq	%r13
	leaq	-88(%rdx), %rcx
	leaq	88(%rdx), %rax
	addq	$88, %rcx
	cmpq	48(%rsp), %rcx
	jne	.LBB242_348
	jmp	.LBB242_372
.LBB242_352:
	cmpq	$-1, 136(%rsp)
	je	.LBB242_333
	movq	24(%rsp), %rdx
	movq	8(%r14), %rcx
	movl	296(%rdx), %eax
	testl	%eax, %eax
	je	.LBB242_364
	movq	104(%rdx), %rax
	addq	%rcx, %rax
	movq	40(%rdx), %rcx
	cmovbq	%r15, %rax
	cmpq	%rcx, %rax
	jbe	.LBB242_333
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$768, 176(%rsp)
.Ltmp7336:
	movq	128(%rsp), %rsi
	leaq	304(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.2910935600939035342)
.Ltmp7337:
	movq	$-1, %r15
	jmp	.LBB242_365
.LBB242_357:
	cmpq	$-1, 136(%rsp)
	je	.LBB242_333
	movq	24(%rsp), %rsi
	cmpq	$-1, 40(%rsi)
	je	.LBB242_333
	movq	8(%r14), %rcx
	movq	16(%r14), %r15
	movl	296(%rsi), %eax
	testl	%eax, %eax
	je	.LBB242_368
	movq	104(%rsi), %rax
	movq	$-1, %rdi
	.p2align	4
.LBB242_361:
	movq	%rax, %rdx
	addq	%rcx, %rdx
	cmovbq	%rdi, %rdx
	lock		cmpxchgq	%rdx, 104(%rsi)
	jne	.LBB242_361
	addq	%rcx, %rax
	movq	40(%rsi), %rcx
	cmovbq	%rdi, %rax
	cmpq	%rcx, %rax
	jbe	.LBB242_332
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$768, 176(%rsp)
.Ltmp7343:
	movq	128(%rsp), %rsi
	leaq	304(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.2910935600939035342)
.Ltmp7344:
	jmp	.LBB242_369
.LBB242_364:
	movq	800(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 320(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 304(%rsp)
.LBB242_365:
	cmpb	$-1, 304(%rsp)
	je	.LBB242_333
	jmp	.LBB242_366
.LBB242_367:
	movq	%rax, %rdx
	jmp	.LBB242_372
.LBB242_368:
	movq	800(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 320(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 304(%rsp)
.LBB242_369:
	movzbl	304(%rsp), %eax
	cmpb	$-1, %al
	sete	%cl
	testq	%r15, %r15
	sete	%dl
	orb	%cl, %dl
	je	.LBB242_459
	cmpb	$-1, %al
	movq	$-1, %r15
	je	.LBB242_333
	jmp	.LBB242_366
.LBB242_371:
	movq	16(%rsp), %rdx
.LBB242_372:
	movq	64(%rsp), %r12
	movq	%rdx, 16(%rsp)
	movq	%rdx, 392(%rsp)
	cmpq	$-1, 136(%rsp)
	jne	.LBB242_375
	jmp	.LBB242_332
.LBB242_373:
	movq	64(%rsp), %r12
	movq	%r15, %r13
.LBB242_374:
	movq	16(%rsp), %rax
	movq	%rax, 392(%rsp)
	cmpq	$-1, 136(%rsp)
	je	.LBB242_332
.LBB242_375:
.Ltmp7341:
	movq	32(%rsp), %rsi
	leaq	176(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp7342:
	cmpb	$-1, 176(%rsp)
	movq	$-1, %r15
	je	.LBB242_333
	jmp	.LBB242_366
.LBB242_377:
	movq	24(%rsp), %r14
	jmp	.LBB242_422
.LBB242_378:
	movq	752(%rsp), %rax
	movq	-16(%rax,%rbx), %rbx
	cmpq	%rbx, %r13
	jae	.LBB242_391
	movq	16(%rsp), %rdx
	cmpq	48(%rsp), %rdx
	je	.LBB242_389
	addq	$88, %rdx
	leaq	-1(%rbx), %r14
	movq	%rdx, %rax
.LBB242_381:
	movq	%rax, %rdx
	movq	-8(%rdx), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rdx, 16(%rsp)
	movq	%rcx, 368(%rsp)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, 304(%rsp)
	cmpq	$-1, %rax
	je	.LBB242_388
	vmovdqu64	304(%rsp), %zmm0
	movq	%rax, 176(%rsp)
	movq	368(%rsp), %rax
	leaq	184(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7321:
	movq	80(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp7322:
	cmpq	%r13, %r14
	je	.LBB242_390
	movq	16(%rsp), %rdx
	incq	%r13
	leaq	-88(%rdx), %rcx
	leaq	88(%rdx), %rax
	addq	$88, %rcx
	cmpq	48(%rsp), %rcx
	jne	.LBB242_381
	jmp	.LBB242_389
.LBB242_385:
	movq	16(%rsp), %rdx
.LBB242_386:
	movq	%rdx, 16(%rsp)
	movq	%rdx, 392(%rsp)
	jmp	.LBB242_422
.LBB242_387:
	movq	16(%rsp), %rax
	movq	144(%rsp), %r13
	movq	%rax, 392(%rsp)
	jmp	.LBB242_422
.LBB242_388:
	movq	16(%rsp), %rdx
.LBB242_389:
	movq	%rdx, 16(%rsp)
	movq	%rdx, 392(%rsp)
	jmp	.LBB242_392
.LBB242_390:
	movq	%rbx, %r13
.LBB242_391:
	movq	16(%rsp), %rax
	movq	%rax, 392(%rsp)
.LBB242_392:
	xorl	%ebp, %ebp
.LBB242_393:
	movq	24(%rsp), %r14
	movq	56(%rsp), %rbx
	cmpq	$-1, 288(%rsp)
	je	.LBB242_398
	cmpq	$0, 704(%rsp)
	je	.LBB242_398
	movq	704(%rsp), %rax
	movq	%rax, 304(%rsp)
	movq	$0, 312(%rsp)
.Ltmp7324:
	movq	128(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	leaq	176(%rsp), %rdi
	leaq	304(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp7325:
	cmpb	$-1, 192(%rsp)
	je	.LBB242_398
	cmpq	$-1, 136(%rsp)
	movb	$1, %bpl
	jne	.LBB242_410
	jmp	.LBB242_472
.LBB242_398:
	cmpq	$-1, 136(%rsp)
	je	.LBB242_422
	cmpq	$-1, 40(%r14)
	je	.LBB242_403
.Ltmp7326:
	movq	128(%rsp), %rsi
	movq	72(%rsp), %rcx
	leaq	176(%rsp), %rdi
	movl	$3, %edx
	xorl	%r8d, %r8d
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.2910935600939035342)
.Ltmp7327:
	cmpb	$-1, 176(%rsp)
	je	.LBB242_403
	movb	$1, %bpl
	jmp	.LBB242_410
.LBB242_403:
	testb	%bpl, %bpl
	jne	.LBB242_406
.Ltmp7328:
	movq	32(%rsp), %rsi
	leaq	176(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp7329:
	cmpb	$-1, 176(%rsp)
	movb	$1, %bpl
	jne	.LBB242_410
.LBB242_406:
	movq	104(%rsp), %rcx
	testq	%rcx, %rcx
	je	.LBB242_409
.Ltmp7330:
	movq	128(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::admit_transient@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	movl	$3, %edx
	vzeroupper
	callq	*%rax
.Ltmp7331:
	cmpb	$-1, 176(%rsp)
	setne	%bpl
	jmp	.LBB242_410
.LBB242_409:
	xorl	%ebp, %ebp
.LBB242_410:
	cmpq	144(%rsp), %r13
	jae	.LBB242_420
	movq	16(%rsp), %rdx
	cmpq	48(%rsp), %rdx
	je	.LBB242_418
	movq	144(%rsp), %rax
	addq	$88, %rdx
	leaq	-1(%rax), %rbx
	movq	%rdx, %rax
.LBB242_413:
	movq	%rax, %rdx
	movq	-8(%rdx), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rdx, 16(%rsp)
	movq	%rcx, 368(%rsp)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, 304(%rsp)
	cmpq	$-1, %rax
	je	.LBB242_417
	vmovdqu64	304(%rsp), %zmm0
	movq	%rax, 176(%rsp)
	movq	368(%rsp), %rax
	leaq	184(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7333:
	movq	80(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp7334:
	cmpq	%r13, %rbx
	je	.LBB242_419
	movq	16(%rsp), %rdx
	incq	%r13
	leaq	-88(%rdx), %rcx
	leaq	88(%rdx), %rax
	addq	$88, %rcx
	cmpq	48(%rsp), %rcx
	jne	.LBB242_413
	jmp	.LBB242_418
.LBB242_417:
	movq	16(%rsp), %rdx
.LBB242_418:
	movq	56(%rsp), %rbx
	movq	%rdx, 16(%rsp)
	movq	%rdx, 392(%rsp)
	jmp	.LBB242_421
.LBB242_419:
	movq	144(%rsp), %r13
	movq	56(%rsp), %rbx
.LBB242_420:
	movq	16(%rsp), %rax
	movq	%rax, 392(%rsp)
.LBB242_421:
	testb	%bpl, %bpl
	jne	.LBB242_471
.LBB242_422:
	cmpq	$0, 40(%rsp)
	je	.LBB242_275
	movq	840(%rsp), %rax
	movq	96(%rsp), %rsi
	movq	%r13, 152(%rsp)
	movq	%rax, 296(%rsp)
	jmp	.LBB242_425
	.p2align	4
.LBB242_424:
	movq	272(%rsp), %rax
	movq	704(%rsp), %rdx
	leaq	(%rbp,%rbp,4), %rcx
	shll	$16, %r15d
	movq	40(%rsp), %rdi
	movq	72(%rsp), %r8
	leaq	40(%rbx), %rsi
	incq	%rbp
	orl	%r15d, %r14d
	movq	$-1, %r15
	shlq	$32, %r14
	orq	%r14, %r13
	movq	24(%rsp), %r14
	movq	%r12, (%rax,%rcx,8)
	movq	%rdx, 8(%rax,%rcx,8)
	movq	104(%rsp), %rdx
	decq	%rdi
	movq	%rdi, 40(%rsp)
	movq	%rdx, 16(%rax,%rcx,8)
	movzbl	96(%rsp), %edx
	movb	%dl, 24(%rax,%rcx,8)
	movq	%r13, %rdx
	movl	%r13d, 25(%rax,%rcx,8)
	shrq	$32, %r13
	shrq	$48, %rdx
	movw	%r13w, 29(%rax,%rcx,8)
	movq	152(%rsp), %r13
	movb	%dl, 31(%rax,%rcx,8)
	movq	%r8, 32(%rax,%rcx,8)
	movq	%rbp, 280(%rsp)
	testq	%rdi, %rdi
	je	.LBB242_429
.LBB242_425:
	movq	%rsi, %rbx
	cmpq	296(%rsp), %rsi
	je	.LBB242_430
	movq	(%rbx), %r12
	testq	%r12, %r12
	je	.LBB242_429
	movq	8(%rbx), %rax
	movq	16(%rbx), %rsi
	movzbl	24(%rbx), %edx
	movq	32(%rbx), %rcx
	movzwl	29(%rbx), %r14d
	movzbl	31(%rbx), %r15d
	movl	25(%rbx), %r13d
	movq	280(%rsp), %rbp
	movq	%rax, 704(%rsp)
	movq	%rsi, 104(%rsp)
	movb	%dl, 96(%rsp)
	movq	%rcx, 72(%rsp)
	cmpq	264(%rsp), %rbp
	jne	.LBB242_424
.Ltmp7365:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	264(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp7366:
	jmp	.LBB242_424
.LBB242_429:
	addq	$40, %rbx
.LBB242_430:
	movq	64(%rsp), %r12
	jmp	.LBB242_276
.LBB242_431:
	movq	56(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB242_433
	movq	792(%rsp), %rdi
	shlq	$5, %rsi
	movl	$8, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB242_433:
.Ltmp7376:
	leaq	384(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp7377:
	movq	736(%rsp), %rax
	testq	%rax, %rax
	je	.LBB242_436
	movq	752(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB242_436:
	movq	536(%rsp), %rax
	testq	%rax, %rax
	je	.LBB242_439
	lock		decq	(%rax)
	jne	.LBB242_439
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	536(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB242_439:
	movq	568(%rsp), %rax
	testq	%rax, %rax
	je	.LBB242_442
	lock		decq	(%rax)
	jne	.LBB242_442
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	568(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB242_442:
	cmpq	1168(%rsp), %r12
	jne	.LBB242_272
.LBB242_443:
	movq	32(%rsp), %r15
	movb	$1, %bl
	xorl	%ebp, %ebp
.Ltmp7381:
	leaq	1224(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp7382:
	cmpq	$-1, 288(%rsp)
	movzbl	91(%rsp), %ecx
	movl	92(%rsp), %ebp
	sete	%al
	xorb	$1, %cl
	orb	%bpl, %cl
	orb	%al, %cl
	cmpb	$1, %cl
	je	.LBB242_447
	movq	$1, 880(%rsp)
	xorl	%ebp, %ebp
	movq	$0, 888(%rsp)
.Ltmp7383:
	movq	<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items@GOTPCREL(%rip), %rax
	movq	128(%rsp), %rsi
	leaq	448(%rsp), %rdi
	leaq	880(%rsp), %rdx
	movl	$1, %ecx
	callq	*%rax
.Ltmp7384:
	movl	92(%rsp), %ebp
.LBB242_447:
	movq	264(%rsp), %rbx
	movq	272(%rsp), %r12
	movq	280(%rsp), %r13
	lock		decq	(%r14)
	movb	$2, %al
	movl	%eax, 72(%rsp)
	jne	.LBB242_449
	#MEMBARRIER
.Ltmp7388:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	808(%rsp), %rdi
	callq	*%rax
.Ltmp7389:
	movq	%rbx, %r14
	jmp	.LBB242_487
.LBB242_366:
	movb	$1, %bpl
.LBB242_468:
	movq	24(%rsp), %r14
	movq	56(%rsp), %rbx
	jmp	.LBB242_472
.LBB242_449:
	movq	%rbx, %r14
	jmp	.LBB242_487
.LBB242_450:
	testq	%rbp, %rbp
	je	.LBB242_470
	movq	56(%rsp), %rbx
	leaq	-1(%rbp), %rax
	cmpq	%rax, %r13
	jae	.LBB242_689
	movq	16(%rsp), %r15
	cmpq	48(%rsp), %r15
	je	.LBB242_458
	notq	%r13
	addq	$88, %r15
	leaq	176(%rsp), %r14
	addq	%rbp, %r13
	movq	%r15, %rax
.LBB242_454:
	movq	%rax, %r15
	movq	-8(%r15), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rcx, 368(%rsp)
	vmovdqu64	-72(%r15), %zmm0
	vmovdqu64	%zmm0, 304(%rsp)
	cmpq	$-1, %rax
	je	.LBB242_458
	vmovdqu64	304(%rsp), %zmm0
	movq	%rax, 176(%rsp)
	movq	368(%rsp), %rax
	leaq	184(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7352:
	movq	80(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%r14, %rdx
	vzeroupper
	callq	*%rax
.Ltmp7353:
	decq	%r13
	je	.LBB242_692
	leaq	-88(%r15), %rcx
	leaq	88(%r15), %rax
	addq	$88, %rcx
	cmpq	48(%rsp), %rcx
	jne	.LBB242_454
.LBB242_458:
	movq	24(%rsp), %r14
	movq	%r15, 392(%rsp)
	jmp	.LBB242_471
.LBB242_459:
	leaq	-1(%r15), %rax
	cmpq	%rax, %r13
	jae	.LBB242_469
	movq	16(%rsp), %r12
	movq	56(%rsp), %rbx
	cmpq	48(%rsp), %r12
	je	.LBB242_466
	notq	%r13
	addq	$88, %r12
	leaq	176(%rsp), %r14
	addq	%r15, %r13
	movq	%r12, %rax
.LBB242_462:
	movq	%rax, %r12
	movq	-8(%r12), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rcx, 368(%rsp)
	vmovdqu64	-72(%r12), %zmm0
	vmovdqu64	%zmm0, 304(%rsp)
	cmpq	$-1, %rax
	je	.LBB242_466
	vmovdqu64	304(%rsp), %zmm0
	movq	%rax, 176(%rsp)
	movq	368(%rsp), %rax
	leaq	184(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7346:
	movq	80(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%r14, %rdx
	vzeroupper
	callq	*%rax
.Ltmp7347:
	decq	%r13
	je	.LBB242_466
	leaq	-88(%r12), %rcx
	leaq	88(%r12), %rax
	addq	$88, %rcx
	cmpq	48(%rsp), %rcx
	jne	.LBB242_462
.LBB242_466:
	movq	24(%rsp), %r14
	movq	%r12, 392(%rsp)
	jmp	.LBB242_471
.LBB242_467:
	xorl	%ebp, %ebp
	jmp	.LBB242_468
.LBB242_469:
	movq	16(%rsp), %r12
	movq	24(%rsp), %r14
	movq	56(%rsp), %rbx
	movq	%r12, 392(%rsp)
	jmp	.LBB242_471
.LBB242_470:
	movq	24(%rsp), %r14
	movq	56(%rsp), %rbx
.LBB242_471:
	movb	$1, %bpl
.LBB242_472:
	movq	264(%rsp), %rax
	movq	32(%rsp), %r15
	movq	272(%rsp), %r12
	movq	280(%rsp), %r13
	movq	%rax, 704(%rsp)
	testq	%rbx, %rbx
	je	.LBB242_474
	movq	792(%rsp), %rdi
	shlq	$5, %rbx
	movl	$8, %edx
	movq	%rbx, %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB242_474:
.Ltmp7355:
	leaq	384(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp7356:
	movq	736(%rsp), %rax
	testq	%rax, %rax
	je	.LBB242_477
	movq	752(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB242_477:
	movq	536(%rsp), %rax
	testq	%rax, %rax
	je	.LBB242_480
	lock		decq	(%rax)
	jne	.LBB242_480
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	536(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB242_480:
	movq	568(%rsp), %rax
	movl	%ebp, 72(%rsp)
	testq	%rax, %rax
	je	.LBB242_483
	lock		decq	(%rax)
	jne	.LBB242_483
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	568(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB242_483:
	xorl	%ebx, %ebx
.Ltmp7358:
	leaq	1224(%rsp), %rdi
	xorl	%ebp, %ebp
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp7359:
	lock		decq	(%r14)
	jne	.LBB242_486
	#MEMBARRIER
.Ltmp7360:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	808(%rsp), %rdi
	callq	*%rax
.Ltmp7361:
	movl	92(%rsp), %ebp
	movq	704(%rsp), %r14
	jmp	.LBB242_487
.LBB242_486:
	movl	92(%rsp), %ebp
	movq	704(%rsp), %r14
.LBB242_487:
	leaq	816(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB242_488:
	movq	616(%r15), %rax
	movq	%r14, 880(%rsp)
	movq	%r12, 888(%rsp)
	movq	%r13, 896(%rsp)
	testq	%rax, %rax
	je	.LBB242_497
	movl	296(%rax), %ecx
	movb	$-1, %bl
	testl	%ecx, %ecx
	jne	.LBB242_491
	movq	288(%rax), %rcx
	movzbl	272(%rax), %ebx
	movq	%rcx, 191(%rsp)
	vmovdqu	273(%rax), %xmm0
	vmovdqa	%xmm0, 176(%rsp)
.LBB242_491:
	movl	72(%rsp), %eax
	testb	%al, %al
	je	.LBB242_496
	movzbl	%al, %eax
	cmpl	$2, %eax
	je	.LBB242_498
	cmpb	$-1, %bl
	je	.LBB242_685
	cmpb	$2, 472(%r15)
	jne	.LBB242_496
	vmovdqa	176(%rsp), %xmm0
	movq	191(%rsp), %rax
	movq	696(%r15), %rdi
	movb	%bl, 448(%rsp)
	vmovdqu	%xmm0, 449(%rsp)
	movq	%rax, 464(%rsp)
	movl	40(%rdi), %eax
	testl	%eax, %eax
	jne	.LBB242_697
.LBB242_496:
	xorl	%ebp, %ebp
	jmp	.LBB242_498
.LBB242_497:
	cmpb	$2, 72(%rsp)
	movb	$-1, %bl
	sete	%al
	andb	%bpl, %al
	movl	%eax, %ebp
.LBB242_498:
	cmpb	$-1, %bl
	sete	%al
	andb	%bpl, %al
	movl	%eax, 24(%rsp)
.LBB242_499:
	movq	1264(%rsp), %rcx
	movq	1280(%rsp), %rdx
	movq	1272(%rsp), %rax
	movq	1288(%rsp), %rsi
	movq	%r14, 304(%rsp)
	movq	%r12, 312(%rsp)
	movq	%r12, 40(%rsp)
	movl	$1, %r12d
	movl	$1, %edi
	movq	%r13, 16(%rsp)
	movq	%r13, 320(%rsp)
	cmpq	$3, %rcx
	movq	%rcx, %r15
	cmovaeq	%rdx, %r15
	cmovaeq	%rcx, %rdi
	cmovaeq	%r12, %rdx
	movq	%rdi, 448(%rsp)
	movq	%rax, 456(%rsp)
	movq	%rdx, 464(%rsp)
	movq	%rsi, 472(%rsp)
	movq	%r15, %rsi
	decq	%rsi
	movq	$0, 480(%rsp)
	movq	%rsi, 488(%rsp)
	je	.LBB242_503
	cmpq	$3, %rcx
	movq	32(%rsp), %rcx
	movq	<purrdf_sparql_eval::witness::RelationWitness>::merge@GOTPCREL(%rip), %rbp
	leaq	456(%rsp), %r13
	leaq	880(%rsp), %r14
	cmovaeq	%rax, %r13
	leaq	640(%rcx), %rbx
	.p2align	4
.LBB242_501:
	movq	%r12, 480(%rsp)
	movq	16(%r13), %rax
	movq	%rax, 896(%rsp)
	vmovdqu	(%r13), %xmm0
	vmovdqa	%xmm0, 880(%rsp)
.Ltmp7414:
	movq	%rbx, %rdi
	movq	%r14, %rsi
	vzeroupper
	callq	*%rbp
.Ltmp7415:
	addq	$24, %r13
	incq	%r12
	cmpq	%r12, %r15
	jne	.LBB242_501
.LBB242_503:
.Ltmp7420:
	leaq	448(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7421:
	movq	112(%rsp), %r12
	movq	32(%rsp), %r13
	cmpb	$0, 24(%rsp)
	movq	1216(%rsp), %rdi
	je	.LBB242_516
	movq	432(%rsp), %rdx
	cmpq	%rdx, %rdi
	ja	.LBB242_695
	movq	616(%r13), %r8
	testq	%r8, %r8
	je	.LBB242_511
	cmpq	$0, 336(%r8)
	leaq	608(%rsp), %rsi
	leaq	520(%rsp), %rcx
	leaq	536(%rsp), %rax
	jne	.LBB242_512
	cmpq	$-1, 16(%r8)
	jne	.LBB242_512
	cmpq	$-1, 40(%r8)
	jne	.LBB242_512
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%xmm0, (%rsi)
	movq	$0, 16(%rsi)
	movq	$-1, 632(%rsp)
	movl	$67108864, 640(%rsp)
	jmp	.LBB242_513
.LBB242_511:
	leaq	520(%rsp), %rcx
	leaq	536(%rsp), %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%xmm0, 608(%rsp)
	movq	$0, 624(%rsp)
	movq	$-1, 632(%rsp)
	movl	$67108864, 640(%rsp)
	movq	$0, 448(%rsp)
	movq	$8, 456(%rsp)
	vmovdqu	%xmm0, 464(%rsp)
	movq	$8, 480(%rsp)
	vmovdqu	%xmm0, 488(%rsp)
	jmp	.LBB242_514
.LBB242_512:
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%xmm0, (%rsi)
	movq	$0, 16(%rsi)
	movq	$-1, 632(%rsp)
	movl	$67174400, 640(%rsp)
.LBB242_513:
	movq	$0, 448(%rsp)
	movq	$8, 456(%rsp)
	vmovdqu	%xmm0, -144(%rsi)
	movq	$8, 480(%rsp)
	vmovdqu	%xmm0, -120(%rsi)
.LBB242_514:
	movq	$8, 504(%rsp)
	movq	$0, 512(%rsp)
	movq	424(%rsp), %rsi
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	vmovdqu	%xmm0, (%rcx)
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu64	%zmm0, (%rax)
	movb	$0, 64(%rax)
	cmpq	%rdx, %rdi
	jne	.LBB242_577
.LBB242_515:
.Ltmp7439:
	leaq	448(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7440:
.LBB242_516:
	movq	320(%rsp), %rax
	vmovdqu	304(%rsp), %xmm0
	movq	%rax, 864(%rsp)
	movq	160(%rsp), %rax
	vmovdqa	%xmm0, 848(%rsp)
	testq	%rax, %rax
	je	.LBB242_519
	lock		decq	(%rax)
	jne	.LBB242_519
	#MEMBARRIER
.Ltmp7441:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	callq	*%rax
.Ltmp7442:
.LBB242_519:
	movq	872(%rsp), %rbx
	movq	32(%rsp), %r14
	movb	$1, %al
	movl	%eax, 16(%rsp)
	movq	696(%r14), %rax
	movl	40(%rax), %ecx
	testl	%ecx, %ecx
	jne	.LBB242_113
.LBB242_520:
	movzbl	16(%rax), %ecx
	cmpb	$-1, %cl
	je	.LBB242_113
	movb	%cl, 880(%rsp)
	movq	120(%rsp), %rcx
	vmovdqu	17(%rax), %xmm0
	vmovdqu	%xmm0, 881(%rsp)
	movq	32(%rax), %rax
	movq	%rax, 896(%rsp)
	lock		incq	(%rcx)
	jle	.LBB242_696
	movq	120(%rsp), %rcx
.Ltmp7443:
	leaq	448(%rsp), %rdi
	leaq	880(%rsp), %rdx
	movq	%rbx, %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
.Ltmp7444:
	vmovdqu64	480(%rsp), %zmm1
	vmovdqu64	448(%rsp), %zmm0
	movq	856(%rsp), %rbx
	movq	864(%rsp), %r14
	vmovdqu64	%zmm1, 40(%r12)
	vmovdqu64	%zmm0, 8(%r12)
	movq	$0, (%r12)
	testq	%r14, %r14
	je	.LBB242_566
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	movabsq	$9223372036854775807, %r12
	xorl	%r15d, %r15d
	jmp	.LBB242_528
	.p2align	4
.LBB242_525:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB242_526:
	vzeroupper
	callq	*%rbp
.LBB242_527:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB242_566
.LBB242_528:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB242_527
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%r12, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r12, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r12, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB242_531
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB242_531:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB242_526
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB242_531
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r12, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB242_534:
	cmpq	%rax, %rdx
	jge	.LBB242_525
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB242_534
	jmp	.LBB242_525
.LBB242_113:
	vmovups	1656(%rsp), %zmm1
	vmovdqu64	1616(%rsp), %zmm0
	movq	864(%rsp), %rcx
	movq	120(%rsp), %rax
	movq	%rcx, 1424(%rsp)
	vmovups	%zmm1, 488(%rsp)
	vmovdqa	848(%rsp), %xmm1
	vmovdqu64	%zmm0, 448(%rsp)
	cmpq	$-1, 448(%rsp)
	vmovdqa	%xmm1, 1408(%rsp)
	movq	%rax, 1432(%rsp)
	je	.LBB242_536
	leaq	880(%rsp), %rdi
	leaq	1408(%rsp), %rsi
	leaq	1616(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	520(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB242_115
.LBB242_537:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	528(%rsp), %rdi
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
	jge	.LBB242_539
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB242_539:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB242_545
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB242_539
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
.LBB242_542:
	cmpq	%rax, %rsi
	jge	.LBB242_544
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB242_542
.LBB242_544:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB242_545:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	544(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB242_546
	jmp	.LBB242_548
.LBB242_536:
	vmovdqu	1408(%rsp), %xmm0
	movq	1424(%rsp), %rax
	movq	1432(%rsp), %rcx
	movq	%rax, 904(%rsp)
	movq	%rcx, 912(%rsp)
	vmovdqu	%xmm0, 888(%rsp)
	movq	$-1, 880(%rsp)
	movq	520(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB242_537
.LBB242_115:
	movq	544(%rsp), %rax
	testq	%rax, %rax
	je	.LBB242_548
.LBB242_546:
	lock		decq	(%rax)
	jne	.LBB242_548
	leaq	544(%rsp), %rdi
	#MEMBARRIER
.Ltmp7446:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp7447:
.LBB242_548:
	vmovdqu64	912(%rsp), %zmm1
	vmovdqu64	880(%rsp), %zmm0
	movl	$0, 172(%rsp)
	vmovdqu64	%zmm1, 40(%r12)
	vmovdqu64	%zmm0, 8(%r12)
	movq	$0, (%r12)
.Ltmp7449:
	leaq	2144(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp7450:
.Ltmp7451:
	leaq	1944(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7452:
	movq	440(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB242_552
	#MEMBARRIER
.Ltmp7454:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	440(%rsp), %rdi
	callq	*%rax
.Ltmp7455:
.LBB242_552:
	cmpb	$0, 16(%rsp)
	je	.LBB242_633
	movq	424(%rsp), %rbx
	movq	432(%rsp), %r14
	testq	%r14, %r14
	je	.LBB242_622
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	movabsq	$9223372036854775807, %r12
	xorl	%r15d, %r15d
	jmp	.LBB242_558
	.p2align	4
.LBB242_555:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB242_556:
	callq	*%r13
.LBB242_557:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB242_622
.LBB242_558:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB242_557
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%r12, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r12, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r12, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB242_561
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB242_561:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB242_556
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB242_561
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r12, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rbp), %rax
	.p2align	4
.LBB242_564:
	cmpq	%rax, %rdx
	jge	.LBB242_555
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB242_564
	jmp	.LBB242_555
.LBB242_566:
	movq	848(%rsp), %rax
	testq	%rax, %rax
	je	.LBB242_578
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rdx
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
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
	jge	.LBB242_569
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB242_569:
	movq	112(%rsp), %r12
	.p2align	4
.LBB242_570:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB242_576
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB242_570
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
.LBB242_573:
	cmpq	%rax, %rsi
	jge	.LBB242_575
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB242_573
.LBB242_575:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB242_576:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
	jmp	.LBB242_579
.LBB242_577:
	leaq	888(%rsp), %rcx
	leaq	(%rdx,%rdx,4), %rax
	leaq	880(%rsp), %r14
	vpbroadcastq	%rcx, %ymm0
	vpaddq	.LCPI242_4(%rip), %ymm0, %ymm1
	vpaddq	.LCPI242_5(%rip), %ymm0, %ymm0
	leaq	(%rsi,%rax,8), %rbp
	leaq	(%rdi,%rdi,4), %rax
	leaq	(%rsi,%rax,8), %rbx
	movq	%rbp, 24(%rsp)
	vmovdqu	%ymm1, 752(%rsp)
	vmovdqu	%ymm0, 704(%rsp)
	jmp	.LBB242_637
.LBB242_578:
	movq	112(%rsp), %r12
.LBB242_579:
.Ltmp7467:
	leaq	2144(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp7468:
.Ltmp7472:
	leaq	1944(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7473:
	movq	120(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB242_583
	#MEMBARRIER
.Ltmp7477:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	120(%rsp), %rdi
	callq	*%rax
.Ltmp7478:
.LBB242_583:
	movq	1688(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB242_593
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	1696(%rsp), %rdi
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
	jge	.LBB242_586
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB242_586:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB242_592
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB242_586
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
.LBB242_589:
	cmpq	%rax, %rsi
	jge	.LBB242_591
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB242_589
.LBB242_591:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB242_592:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB242_593:
	movq	1616(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB242_603
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	1624(%rsp), %rdi
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
	jge	.LBB242_596
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB242_596:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB242_602
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB242_596
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
.LBB242_599:
	cmpq	%rax, %rsi
	jge	.LBB242_601
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB242_599
.LBB242_601:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB242_602:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB242_603:
	movq	1712(%rsp), %rax
	testq	%rax, %rax
	je	.LBB242_606
	lock		decq	(%rax)
	jne	.LBB242_606
	leaq	1712(%rsp), %rdi
	#MEMBARRIER
.Ltmp7482:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp7483:
.LBB242_606:
	movq	440(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB242_608
	#MEMBARRIER
.Ltmp7488:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	440(%rsp), %rdi
	callq	*%rax
.Ltmp7489:
.LBB242_608:
	cmpb	$0, 16(%rsp)
	je	.LBB242_633
	movq	424(%rsp), %rbx
	movq	432(%rsp), %r14
	testq	%r14, %r14
	je	.LBB242_622
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	movabsq	$9223372036854775807, %r12
	xorl	%r15d, %r15d
	jmp	.LBB242_614
	.p2align	4
.LBB242_611:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB242_612:
	callq	*%r13
.LBB242_613:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB242_622
.LBB242_614:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB242_613
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%r12, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r12, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r12, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB242_617
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB242_617:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB242_612
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB242_617
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r12, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rbp), %rax
	.p2align	4
.LBB242_620:
	cmpq	%rax, %rdx
	jge	.LBB242_611
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB242_620
	jmp	.LBB242_611
.LBB242_622:
	movq	416(%rsp), %rax
	movq	112(%rsp), %r12
	testq	%rax, %rax
	je	.LBB242_633
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rdx
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
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
	jge	.LBB242_625
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB242_625:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB242_631
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB242_625
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
.LBB242_628:
	cmpq	%rax, %rsi
	jge	.LBB242_630
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB242_628
.LBB242_630:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB242_631:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
.LBB242_632:
	vzeroupper
	callq	*%rax
.LBB242_633:
	movq	%r12, %rax
	addq	$2360, %rsp
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
.LBB242_634:
	.cfi_def_cfa_offset 2416
	movq	312(%rsp), %rax
	movq	%rax, 40(%rsp)
.LBB242_635:
	movq	16(%rsp), %rcx
	movq	40(%rsp), %rdx
	leaq	(%rcx,%rcx,4), %rax
	incq	%rcx
	movq	%rcx, 16(%rsp)
	movq	%r12, (%rdx,%rax,8)
	movq	%rbp, 8(%rdx,%rax,8)
	movq	%r14, 16(%rdx,%rax,8)
	movq	112(%rsp), %r12
	movq	24(%rsp), %rbp
	leaq	880(%rsp), %r14
	vmovdqa	176(%rsp), %xmm0
	vmovdqu	%xmm0, 24(%rdx,%rax,8)
	movq	%rcx, 320(%rsp)
.LBB242_636:
	addq	$40, %rbx
	cmpq	%rbp, %rbx
	je	.LBB242_515
.LBB242_637:
.Ltmp7422:
	leaq	448(%rsp), %rsi
	movq	%r14, %rdi
	movq	%r13, %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7423:
	cmpb	$-1, 880(%rsp)
	jne	.LBB242_515
	movq	(%rbx), %rcx
	leaq	8(%rbx), %r15
	movq	%r15, %rdx
	decq	%rcx
	cmpq	$5, %rcx
	jb	.LBB242_641
	movq	16(%rbx), %rcx
	movq	8(%rbx), %rdx
	decq	%rcx
.LBB242_641:
	movq	120(%rsp), %r8
	addq	$16, %r8
.Ltmp7424:
	leaq	2144(%rsp), %rsi
	movq	%r14, %rdi
	movq	%r13, %r9
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7425:
	movq	880(%rsp), %rax
	movl	888(%rsp), %edx
	movl	892(%rsp), %ecx
	cmpq	$-1, %rax
	jne	.LBB242_684
	cmpl	$2, %edx
	je	.LBB242_636
.Ltmp7426:
	movq	%r14, %rdi
	movq	%r13, %rsi
	callq	purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7427:
	movq	880(%rsp), %rax
	movzbl	888(%rsp), %edx
	cmpq	$-1, %rax
	jne	.LBB242_686
	testb	$1, %dl
	je	.LBB242_636
	movq	(%rbx), %rax
	movq	16(%rbx), %r12
	leaq	-1(%rax), %r14
	decq	%r12
	cmpq	$5, %r14
	cmovbq	%r14, %r12
	cmpq	$4, %r12
	jbe	.LBB242_663
	movabsq	$2305843009213693920, %rax
	leaq	(,%r12,8), %r13
	movabsq	$9223372036854775804, %rcx
	addq	$31, %rax
	cmpq	%rax, %r12
	seta	%al
	cmpq	%rcx, %r13
	seta	%cl
	orb	%al, %cl
	jne	.LBB242_682
	movl	$4, %esi
	movq	%r13, %rdi
	callq	__rustc::__rust_alloc
	movl	$4, %edi
	testq	%rax, %rax
	je	.LBB242_683
	movq	%rax, %rbp
	cmpq	$5, %r14
	jb	.LBB242_652
	movq	(%r15), %r15
.LBB242_652:
	cmpq	$8, %r12
	jb	.LBB242_655
	leaq	(%r15,%r13), %rax
	cmpq	%rax, %rbp
	jae	.LBB242_669
	movq	%rbp, %rax
	addq	%r13, %rax
	cmpq	%rax, %r15
	jae	.LBB242_669
.LBB242_655:
	movq	32(%rsp), %r13
	xorl	%eax, %eax
.LBB242_656:
	movq	%r12, %rdx
	andq	$7, %rdx
	movq	%rax, %rcx
	je	.LBB242_660
	movq	%rax, %rcx
.LBB242_658:
	movl	(%r15,%rcx,8), %esi
	movl	4(%r15,%rcx,8), %edi
	movl	%esi, (%rbp,%rcx,8)
	movl	%edi, 4(%rbp,%rcx,8)
	incq	%rcx
	decq	%rdx
	jne	.LBB242_658
	leaq	-1(%rcx), %r14
.LBB242_660:
	subq	%r12, %rax
	cmpq	$-8, %rax
	ja	.LBB242_679
	movq	%r12, %rax
	decq	%rcx
	negq	%rax
	movq	%rcx, %r14
.LBB242_662:
	movl	8(%r15,%r14,8), %ecx
	movl	12(%r15,%r14,8), %edx
	movl	%ecx, 8(%rbp,%r14,8)
	movl	%edx, 12(%rbp,%r14,8)
	movl	16(%r15,%r14,8), %ecx
	movl	20(%r15,%r14,8), %edx
	movl	%ecx, 16(%rbp,%r14,8)
	movl	%edx, 20(%rbp,%r14,8)
	movl	24(%r15,%r14,8), %ecx
	movl	28(%r15,%r14,8), %edx
	movl	%ecx, 24(%rbp,%r14,8)
	movl	%edx, 28(%rbp,%r14,8)
	movl	32(%r15,%r14,8), %ecx
	movl	36(%r15,%r14,8), %edx
	movl	%ecx, 32(%rbp,%r14,8)
	movl	%edx, 36(%rbp,%r14,8)
	movl	40(%r15,%r14,8), %ecx
	movl	44(%r15,%r14,8), %edx
	movl	%ecx, 40(%rbp,%r14,8)
	movl	%edx, 44(%rbp,%r14,8)
	movl	48(%r15,%r14,8), %ecx
	movl	52(%r15,%r14,8), %edx
	movl	%ecx, 48(%rbp,%r14,8)
	movl	%edx, 52(%rbp,%r14,8)
	movl	56(%r15,%r14,8), %ecx
	movl	60(%r15,%r14,8), %edx
	movl	%ecx, 56(%rbp,%r14,8)
	movl	%edx, 60(%rbp,%r14,8)
	movl	64(%r15,%r14,8), %ecx
	movl	68(%r15,%r14,8), %edx
	movl	%ecx, 64(%rbp,%r14,8)
	movl	%edx, 68(%rbp,%r14,8)
	leaq	8(%rax,%r14), %rdx
	addq	$8, %r14
	cmpq	$-1, %rdx
	jne	.LBB242_662
	jmp	.LBB242_679
.LBB242_663:
	cmpq	$6, %rax
	jb	.LBB242_665
	movq	(%r15), %r15
.LBB242_665:
	testq	%r12, %r12
	je	.LBB242_667
	leaq	-1(%r12), %rax
	vpmovsxbd	.LCPI242_8(%rip), %xmm2
	vmovdqu	752(%rsp), %ymm3
	vpxor	%xmm1, %xmm1, %xmm1
	incq	%r12
	vpbroadcastq	%rax, %ymm0
	vpcmpnltuq	.LCPI242_6(%rip), %ymm0, %k1
	vpxor	%xmm0, %xmm0, %xmm0
	kmovq	%k1, %k2
	vpgatherdd	(%r15,%xmm2), %xmm0 {%k2}
	kmovq	%k1, %k2
	vpgatherdd	4(%r15,%xmm2), %xmm1 {%k2}
	kmovq	%k1, %k2
	vpscatterqd	%xmm0, (,%ymm3) {%k2}
	vmovdqu	704(%rsp), %ymm0
	vpscatterqd	%xmm1, (,%ymm0) {%k1}
	jmp	.LBB242_668
.LBB242_667:
	movl	$1, %r12d
.LBB242_668:
	movq	%r12, 880(%rsp)
	leaq	888(%rsp), %rax
	vmovdqu	16(%rax), %xmm0
	movq	888(%rsp), %rbp
	movq	896(%rsp), %r14
	vmovdqa	%xmm0, 176(%rsp)
	jmp	.LBB242_680
.LBB242_669:
	cmpq	$32, %r12
	jae	.LBB242_671
	movq	32(%rsp), %r13
	xorl	%eax, %eax
	jmp	.LBB242_675
.LBB242_671:
	movq	32(%rsp), %r13
	movabsq	$2305843009213693920, %rcx
	movq	%r12, %rax
	andq	%rcx, %rax
	xorl	%ecx, %ecx
.LBB242_672:
	vmovdqu64	(%r15,%rcx,8), %zmm0
	vmovdqu64	64(%r15,%rcx,8), %zmm1
	vmovdqu64	128(%r15,%rcx,8), %zmm2
	vmovdqu64	192(%r15,%rcx,8), %zmm3
	vmovdqu64	%zmm0, (%rbp,%rcx,8)
	vmovdqu64	%zmm1, 64(%rbp,%rcx,8)
	vmovdqu64	%zmm2, 128(%rbp,%rcx,8)
	vmovdqu64	%zmm3, 192(%rbp,%rcx,8)
	addq	$32, %rcx
	cmpq	%rcx, %rax
	jne	.LBB242_672
	cmpq	%rax, %r12
	je	.LBB242_678
	testb	$24, %r12b
	je	.LBB242_656
.LBB242_675:
	movq	%rax, %rcx
	movabsq	$2305843009213693920, %rax
	addq	$24, %rax
	andq	%r12, %rax
.LBB242_676:
	vmovdqu64	(%r15,%rcx,8), %zmm0
	vmovdqu64	%zmm0, (%rbp,%rcx,8)
	addq	$8, %rcx
	cmpq	%rcx, %rax
	jne	.LBB242_676
	cmpq	%rax, %r12
	jne	.LBB242_656
.LBB242_678:
	decq	%rax
	movq	%rax, %r14
.LBB242_679:
	addq	$2, %r14
	incq	%r12
.LBB242_680:
	movq	16(%rsp), %rax
	cmpq	304(%rsp), %rax
	jne	.LBB242_635
.Ltmp7431:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	304(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp7432:
	jmp	.LBB242_634
.LBB242_682:
	xorl	%edi, %edi
.LBB242_683:
.Ltmp7434:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%r13, %rsi
	callq	*%rax
.Ltmp7435:
	jmp	.LBB242_696
.LBB242_684:
	vmovups	912(%rsp), %zmm1
	vmovups	896(%rsp), %zmm0
	movl	%edx, %esi
	shrl	$8, %esi
	vmovups	%zmm1, 192(%rsp)
	vmovups	%zmm0, 176(%rsp)
	jmp	.LBB242_687
.LBB242_685:
	movb	$-1, %bl
	xorl	%ebp, %ebp
	jmp	.LBB242_498
.LBB242_686:
	movzbl	891(%rsp), %ecx
	movzwl	889(%rsp), %esi
	vmovups	896(%rsp), %zmm0
	vmovups	912(%rsp), %zmm1
	shll	$16, %ecx
	orl	%ecx, %esi
	movl	892(%rsp), %ecx
	vmovups	%zmm0, 176(%rsp)
	vmovups	%zmm1, 192(%rsp)
.LBB242_687:
	vmovups	176(%rsp), %zmm0
	vmovups	192(%rsp), %zmm1
	movw	%si, 25(%r12)
	shrl	$16, %esi
	movb	%sil, 27(%r12)
	movl	%ecx, 28(%r12)
	vmovups	%zmm0, 32(%r12)
	vmovups	%zmm1, 48(%r12)
	movq	%rax, 16(%r12)
	movb	%dl, 24(%r12)
	movq	$1, (%r12)
.Ltmp7429:
	leaq	448(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7430:
	leaq	304(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	160(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB242_237
	jmp	.LBB242_239
.LBB242_689:
	movq	24(%rsp), %r14
	movq	16(%rsp), %r15
.LBB242_690:
	movb	$1, %bpl
	movq	%r15, 392(%rsp)
	jmp	.LBB242_472
.LBB242_692:
	movq	24(%rsp), %r14
	jmp	.LBB242_690
.LBB242_693:
.Ltmp7368:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.290(%rip), %rcx
	vzeroupper
	callq	*%rax
.Ltmp7369:
	jmp	.LBB242_696
.LBB242_694:
	leaq	1256(%rsp), %rax
	leaq	304(%rsp), %rcx
	movq	%rdi, 1256(%rsp)
	movq	%rdx, 304(%rsp)
	movq	%rax, 176(%rsp)
	leaq	<usize as core::fmt::Debug>::fmt(%rip), %rax
	movq	%rax, 184(%rsp)
	movq	%rcx, 192(%rsp)
	movq	%rax, 200(%rsp)
.Ltmp7371:
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.2159(%rip), %rdi
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.289(%rip), %rdx
	leaq	176(%rsp), %rsi
	vzeroupper
	callq	*%rax
.Ltmp7372:
	jmp	.LBB242_696
.LBB242_695:
.Ltmp7457:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.405(%rip), %rcx
	movq	%rdx, %rsi
	callq	*%rax
.Ltmp7458:
.LBB242_696:
	ud2
.LBB242_697:
	addq	$16, %rdi
.Ltmp7406:
	leaq	448(%rsp), %rsi
	vzeroupper
	callq	<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::initialize::<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::get_or_init<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::try_insert::{closure#0}>::{closure#0}, !> (.llvm.2910935600939035342)
.Ltmp7407:
	jmp	.LBB242_496
.LBB242_698:
.Ltmp7408:
	leaq	880(%rsp), %rdi
	movq	%rax, %r13
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB242_788
.LBB242_699:
.Ltmp7354:
	movq	%rax, %r13
	movq	%r15, 392(%rsp)
	jmp	.LBB242_769
.LBB242_700:
.Ltmp7348:
	movq	%rax, %r13
	movq	%r12, 392(%rsp)
	jmp	.LBB242_769
.LBB242_701:
.Ltmp7390:
	leaq	816(%rsp), %rdi
	movq	%rax, %r13
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB242_788
.LBB242_702:
.Ltmp7378:
	movq	%rax, %r13
	movb	$1, %bl
	jmp	.LBB242_772
.LBB242_703:
.Ltmp7357:
	movq	%rax, %r13
	xorl	%ebx, %ebx
	jmp	.LBB242_772
.LBB242_704:
.Ltmp7433:
	movq	%rax, %r13
	cmpq	$6, %r12
	jb	.LBB242_792
	leaq	-8(,%r12,8), %rsi
	movl	$4, %edx
	movq	%rbp, %rdi
	callq	__rustc::__rust_dealloc
	jmp	.LBB242_792
.LBB242_706:
.Ltmp7317:
	jmp	.LBB242_768
.LBB242_707:
.Ltmp7323:
	jmp	.LBB242_746
.LBB242_708:
.Ltmp7332:
	jmp	.LBB242_768
.LBB242_709:
.Ltmp7385:
	movq	%rax, %r13
	jmp	.LBB242_782
.LBB242_710:
.Ltmp7405:
	jmp	.LBB242_736
.LBB242_711:
.Ltmp7351:
	jmp	.LBB242_768
.LBB242_712:
.Ltmp7393:
	movq	%rax, %r13
	movq	%r12, 456(%rsp)
	cmpq	$6, %rbp
	jb	.LBB242_714
	movq	16(%rsp), %rdi
	leaq	-8(,%rbp,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB242_714:
	leaq	448(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB242_718
.LBB242_798:
.Ltmp7445:
	leaq	848(%rsp), %rdi
	movq	%rax, %r13
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movl	16(%rsp), %ebp
	jmp	.LBB242_799
.LBB242_715:
.Ltmp7448:
	movl	16(%rsp), %ebp
	movq	%rax, %r13
	xorl	%ebx, %ebx
	jmp	.LBB242_800
.LBB242_716:
.Ltmp7335:
	jmp	.LBB242_746
.LBB242_717:
.Ltmp7312:
	leaq	1304(%rsp), %rcx
	movq	%rax, %r13
	movq	%rcx, 80(%rsp)
.LBB242_718:
	movq	80(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB242_787
.LBB242_719:
.Ltmp7306:
	movq	%rax, %r13
	jmp	.LBB242_739
.LBB242_720:
.Ltmp7295:
	movq	%rax, %r13
	jmp	.LBB242_740
.LBB242_721:
.Ltmp7411:
	movq	%rax, %r13
	jmp	.LBB242_788
.LBB242_722:
.Ltmp7484:
	jmp	.LBB242_732
.LBB242_723:
.Ltmp7456:
	jmp	.LBB242_726
.LBB242_724:
.Ltmp7282:
	movq	%rax, %r15
	movq	%rbx, 312(%rsp)
	cmpq	$5, %r13
	ja	.LBB242_763
	jmp	.LBB242_764
.LBB242_725:
.Ltmp7490:
.LBB242_726:
	cmpb	$0, 16(%rsp)
	movq	%rax, %r13
	jne	.LBB242_809
	jmp	.LBB242_810
.LBB242_727:
.Ltmp7479:
	movl	16(%rsp), %ebp
	movq	%rax, %r13
	jmp	.LBB242_805
.LBB242_728:
.Ltmp7345:
	jmp	.LBB242_768
.LBB242_729:
.Ltmp7364:
	jmp	.LBB242_746
.LBB242_730:
.Ltmp7292:
	leaq	1328(%rsp), %rdi
	movq	%rax, %r13
	jmp	.LBB242_794
.LBB242_731:
.Ltmp7453:
.LBB242_732:
	movl	16(%rsp), %ebp
	movq	%rax, %r13
	jmp	.LBB242_806
.LBB242_733:
.Ltmp7367:
	addq	$40, %rbx
	movq	%rax, %r13
	movq	%rbx, 824(%rsp)
	cmpq	$6, %r12
	jb	.LBB242_769
	movq	704(%rsp), %rdi
	leaq	-8(,%r12,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB242_769
.LBB242_735:
.Ltmp7289:
.LBB242_736:
	movq	%rax, %r13
	jmp	.LBB242_795
.LBB242_737:
.Ltmp7298:
	movq	%rax, %r13
	movq	%rbp, 888(%rsp)
.Ltmp7299:
	leaq	448(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7300:
.Ltmp7302:
	leaq	880(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp7303:
.LBB242_739:
.Ltmp7307:
	leaq	176(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp7308:
.LBB242_740:
	leaq	1304(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB242_788
.LBB242_741:
.Ltmp7301:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB242_742:
.Ltmp7309:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB242_743:
.Ltmp7464:
	movq	%rax, %r13
	movb	$1, %bpl
	jmp	.LBB242_799
.LBB242_744:
.Ltmp7428:
	jmp	.LBB242_791
.LBB242_745:
.Ltmp7340:
.LBB242_746:
	movq	16(%rsp), %rcx
	movq	%rax, %r13
	movq	%rcx, 392(%rsp)
	jmp	.LBB242_769
.LBB242_747:
.Ltmp7320:
	jmp	.LBB242_768
.LBB242_748:
.Ltmp7399:
	movq	%rax, %r13
	testq	%r15, %r15
	je	.LBB242_752
	negq	%r15
	addq	$160, %r14
.LBB242_750:
.Ltmp7400:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7401:
	addq	$160, %r14
	decq	%r15
	jne	.LBB242_750
.LBB242_752:
	cmpq	$0, 48(%rsp)
	je	.LBB242_788
	movq	48(%rsp), %rax
	movq	64(%rsp), %rdi
	movl	$8, %edx
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
	jmp	.LBB242_788
.LBB242_754:
.Ltmp7402:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB242_755:
.Ltmp7474:
	movl	16(%rsp), %ebp
	movq	%rax, %r13
	jmp	.LBB242_803
.LBB242_756:
.Ltmp7469:
	movl	172(%rsp), %ebx
	movl	16(%rsp), %ebp
	movq	%rax, %r13
	jmp	.LBB242_801
.LBB242_757:
.Ltmp7416:
	movq	%rax, %r13
.Ltmp7417:
	leaq	448(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7418:
	jmp	.LBB242_793
.LBB242_758:
.Ltmp7419:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB242_759:
.Ltmp7261:
	movq	%rax, %r13
.Ltmp7262:
	leaq	1720(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp7263:
	jmp	.LBB242_810
.LBB242_760:
.Ltmp7264:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB242_761:
.Ltmp7279:
	movq	%rax, %r15
	movq	%rbx, 312(%rsp)
	cmpq	$6, %r13
	jb	.LBB242_764
	movq	1416(%rsp), %r14
.LBB242_763:
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
	movq	%r14, %rdi
	callq	__rustc::__rust_dealloc
.LBB242_764:
	leaq	304(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	leaq	1120(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movb	$1, %bl
	xorl	%ebp, %ebp
	movq	%r15, %r13
	jmp	.LBB242_800
.LBB242_765:
.Ltmp7459:
	movq	%rax, %r13
	jmp	.LBB242_793
.LBB242_766:
.Ltmp7373:
	jmp	.LBB242_768
.LBB242_767:
.Ltmp7370:
.LBB242_768:
	movq	%rax, %r13
.LBB242_769:
	cmpq	$0, 56(%rsp)
	je	.LBB242_771
	movq	56(%rsp), %rsi
	movq	792(%rsp), %rdi
	movl	$8, %edx
	shlq	$5, %rsi
	callq	__rustc::__rust_dealloc
.LBB242_771:
	movb	$1, %bl
.Ltmp7374:
	leaq	384(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp7375:
.LBB242_772:
	cmpq	$0, 736(%rsp)
	je	.LBB242_774
	movq	736(%rsp), %rax
	movq	752(%rsp), %rdi
	movl	$8, %edx
	shlq	$3, %rax
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB242_774:
	movq	536(%rsp), %rax
	testq	%rax, %rax
	je	.LBB242_777
	lock		decq	(%rax)
	jne	.LBB242_777
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	536(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB242_777:
	movq	568(%rsp), %rax
	testq	%rax, %rax
	je	.LBB242_780
	lock		decq	(%rax)
	jne	.LBB242_780
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	568(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB242_780:
.Ltmp7379:
	leaq	1224(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp7380:
	xorl	%ebp, %ebp
.LBB242_782:
	movq	24(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB242_784
	#MEMBARRIER
.Ltmp7386:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	808(%rsp), %rdi
	callq	*%rax
.Ltmp7387:
.LBB242_784:
	leaq	816(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	testb	%bl, %bl
	je	.LBB242_786
	leaq	264(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB242_786:
	testb	%bpl, %bpl
	je	.LBB242_788
.LBB242_787:
.Ltmp7394:
	leaq	1376(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp7395:
.LBB242_788:
.Ltmp7412:
	leaq	1264(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7413:
	jmp	.LBB242_795
.LBB242_789:
.Ltmp7396:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB242_790:
.Ltmp7436:
.LBB242_791:
	movq	%rax, %r13
.LBB242_792:
.Ltmp7437:
	leaq	448(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7438:
.LBB242_793:
	leaq	304(%rsp), %rdi
.LBB242_794:
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB242_795:
	movq	160(%rsp), %rax
	movb	$1, %bpl
	testq	%rax, %rax
	je	.LBB242_799
	lock		decq	(%rax)
	movb	$1, %bpl
	jne	.LBB242_799
	movb	$1, %bpl
	#MEMBARRIER
.Ltmp7460:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	callq	*%rax
.Ltmp7461:
	movb	$1, %bl
	jmp	.LBB242_800
.LBB242_799:
	movb	$1, %bl
.LBB242_800:
.Ltmp7465:
	leaq	2144(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp7466:
.LBB242_801:
.Ltmp7470:
	leaq	1944(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7471:
	testb	%bl, %bl
	je	.LBB242_806
.LBB242_803:
	movq	120(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB242_805
	#MEMBARRIER
.Ltmp7475:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	120(%rsp), %rdi
	callq	*%rax
.Ltmp7476:
.LBB242_805:
.Ltmp7480:
	leaq	1616(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp7481:
.LBB242_806:
	movq	440(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB242_808
	#MEMBARRIER
.Ltmp7485:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	440(%rsp), %rdi
	callq	*%rax
.Ltmp7486:
.LBB242_808:
	testb	%bpl, %bpl
	je	.LBB242_810
.LBB242_809:
	leaq	416(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB242_810:
	movq	%r13, %rdi
	callq	_Unwind_Resume@PLT
.LBB242_811:
.Ltmp7487:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end242:
