purrdf_sparql_eval::expr::eval_filter::<purrdf_core::ir::dataset::RdfDataset>:
.Lfunc_begin240:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception160
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
.Ltmp7307:
	leaq	1824(%rsp), %rdi
	movq	%r14, %rsi
	movq	%r15, 32(%rsp)
	movq	%r15, %rdx
	movq	%r14, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp7308:
	cmpl	$1, 1824(%rsp)
	jne	.LBB240_25
	vmovdqu64	1872(%rsp), %zmm1
	vmovdqu64	1840(%rsp), %zmm0
	movq	1792(%rsp), %rax
	vmovdqu64	%zmm1, 48(%r12)
	vmovdqu64	%zmm0, 16(%r12)
	movq	$1, (%r12)
	cmpq	$6, %rax
	jb	.LBB240_12
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
	jge	.LBB240_5
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB240_5:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB240_11
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB240_5
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
.LBB240_8:
	cmpq	%rax, %rsi
	jge	.LBB240_10
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB240_8
.LBB240_10:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB240_11:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB240_12:
	movq	1720(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB240_22
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
	jge	.LBB240_15
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB240_15:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB240_21
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB240_15
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
.LBB240_18:
	cmpq	%rax, %rsi
	jge	.LBB240_20
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB240_18
.LBB240_20:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB240_21:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB240_22:
	movq	1816(%rsp), %rax
	testq	%rax, %rax
	je	.LBB240_633
	lock		decq	(%rax)
	jne	.LBB240_633
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1816(%rsp), %rdi
	#MEMBARRIER
	jmp	.LBB240_632
.LBB240_25:
	vmovdqu64	1864(%rsp), %zmm1
	vmovdqu64	1832(%rsp), %zmm0
	vmovdqu64	%zmm1, 480(%rsp)
	vmovdqu64	%zmm0, 448(%rsp)
.Ltmp7309:
	leaq	880(%rsp), %rdi
	leaq	1720(%rsp), %rsi
	leaq	448(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp7310:
	cmpq	$-1, 880(%rsp)
	je	.LBB240_812
	vmovdqu	880(%rsp), %ymm0
	vmovdqu64	1760(%rsp), %zmm1
	vmovdqu64	1720(%rsp), %zmm2
	vmovdqu	%ymm0, 416(%rsp)
	vmovdqu64	%zmm1, 1656(%rsp)
	vmovdqu64	%zmm2, 1616(%rsp)
	movq	440(%rsp), %rax
	lock		incq	(%rax)
	jle	.LBB240_696
	movq	%rax, 120(%rsp)
	movb	$1, %al
	movl	%eax, 16(%rsp)
.Ltmp7315:
	movq	32(%rsp), %r14
	movq	%rbx, %rsi
	movq	%r14, %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop
.Ltmp7316:
	cmpb	$2, 472(%r14)
	sete	%cl
	andb	%cl, %al
	cmpb	$1, %al
	jne	.LBB240_32
	movq	616(%r14), %rax
	testq	%rax, %rax
	je	.LBB240_33
	cmpq	$-2, 24(%rax)
	jb	.LBB240_32
	cmpq	$-2, 32(%rax)
	jae	.LBB240_35
.LBB240_32:
	xorl	%ebp, %ebp
	jmp	.LBB240_36
.LBB240_812:
	leaq	8(%r12), %rdi
	leaq	1720(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
	movq	$0, (%r12)
	jmp	.LBB240_633
.LBB240_33:
	movb	$1, %bpl
	jmp	.LBB240_36
.LBB240_35:
	cmpq	$-2, 48(%rax)
	setae	%bpl
.LBB240_36:
	movq	432(%rsp), %r14
.Ltmp7317:
	movq	32(%rsp), %r15
	movzbl	%bpl, %edx
	leaq	1944(%rsp), %rdi
	movq	%r14, %rcx
	movq	%r15, %rsi
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7318:
	movb	$1, %al
	movl	%eax, 16(%rsp)
.Ltmp7319:
	movb	$1, %al
	movq	%r15, %rdi
	movq	%r13, 872(%rsp)
	movq	%r13, %rsi
	movq	%rbx, %rdx
	movl	%eax, 172(%rsp)
	callq	purrdf_sparql_eval::vm::program_at::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7320:
	movq	120(%rsp), %rcx
	movb	$1, %dl
	movl	%edx, 172(%rsp)
	addq	$16, %rcx
.Ltmp7321:
	leaq	2144(%rsp), %r13
	movb	$1, %dl
	movq	%rax, %rsi
	movq	%r15, %r8
	movl	%edx, 16(%rsp)
	movq	%r13, %rdi
	movq	%rbx, %rdx
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::link::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7322:
	movq	%r12, 112(%rsp)
	testb	%bpl, %bpl
	je	.LBB240_44
	movq	2128(%rsp), %rax
	movq	424(%rsp), %rbx
	cmpq	%r14, %rax
	cmovbq	%rax, %r14
.Ltmp7333:
	movq	32(%rsp), %rdi
	movq	%r14, %rsi
	callq	<purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot
.Ltmp7334:
	movq	32(%rsp), %rcx
	leaq	1944(%rsp), %rsi
	movq	%rax, 160(%rsp)
	movq	616(%rcx), %rcx
	testq	%rcx, %rcx
	je	.LBB240_70
	cmpq	$-2, 16(%rcx)
	movb	$1, %al
	jb	.LBB240_71
	cmpq	$-2, 40(%rcx)
	setb	%al
	jmp	.LBB240_71
.LBB240_44:
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
	je	.LBB240_88
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	24(%rsp), %rbx
	movl	$8, %eax
	movq	$0, 16(%rsp)
	movq	%rax, 40(%rsp)
	jmp	.LBB240_49
.LBB240_46:
	movq	1128(%rsp), %rax
	movq	%rax, 40(%rsp)
.LBB240_47:
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
.LBB240_48:
	cmpq	%r15, %rbx
	je	.LBB240_87
.LBB240_49:
	vmovups	8(%rbx), %ymm0
	movq	(%rbx), %r13
	addq	$40, %rbx
	vmovups	%ymm0, 176(%rsp)
	testq	%r13, %r13
	je	.LBB240_88
	vmovdqu	176(%rsp), %ymm0
	leaq	1416(%rsp), %rax
	movq	%r13, 1408(%rsp)
	vmovdqu	%ymm0, (%rax)
.Ltmp7323:
	movq	32(%rsp), %rdx
	leaq	448(%rsp), %rdi
	leaq	1944(%rsp), %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7324:
	cmpb	$-1, 448(%rsp)
	jne	.LBB240_84
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
.Ltmp7325:
	movq	32(%rsp), %r9
	leaq	448(%rsp), %rdi
	leaq	2144(%rsp), %rsi
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7326:
	movq	448(%rsp), %rax
	movl	456(%rsp), %edx
	movl	460(%rsp), %ecx
	cmpq	$-1, %rax
	jne	.LBB240_86
	cmpl	$2, %edx
	je	.LBB240_58
.Ltmp7327:
	movq	32(%rsp), %rsi
	leaq	448(%rsp), %rdi
	callq	purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7328:
	movq	448(%rsp), %rax
	movzbl	456(%rsp), %edx
	cmpq	$-1, %rax
	jne	.LBB240_152
	testb	$1, %dl
	jne	.LBB240_68
.LBB240_58:
	cmpq	$6, %r13
	jb	.LBB240_48
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
	jge	.LBB240_61
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB240_61:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB240_67
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB240_61
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
.LBB240_64:
	cmpq	%rax, %rdx
	jge	.LBB240_66
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB240_64
.LBB240_66:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB240_67:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
	jmp	.LBB240_48
.LBB240_68:
	leaq	184(%rsp), %rcx
	vmovdqu	(%rcx), %xmm0
	movq	16(%rcx), %rax
	movq	16(%rsp), %rcx
	movq	%rax, 464(%rsp)
	vmovdqa	%xmm0, 448(%rsp)
	cmpq	1120(%rsp), %rcx
	jne	.LBB240_47
.Ltmp7330:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	1120(%rsp), %rdi
	callq	*%rax
.Ltmp7331:
	jmp	.LBB240_46
.LBB240_70:
	xorl	%eax, %eax
.LBB240_71:
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
	je	.LBB240_73
.Ltmp7337:
	movq	%rsi, (%rsp)
	movzbl	%cl, %esi
	leaq	448(%rsp), %rdi
	leaq	1408(%rsp), %r8
	leaq	176(%rsp), %r9
	movq	%rbx, %rdx
	movq	%r14, %rcx
	callq	purrdf_sparql_eval::parallel::par_blocks_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>
.Ltmp7338:
	jmp	.LBB240_74
.LBB240_73:
.Ltmp7335:
	movq	%rsi, (%rsp)
	movzbl	%cl, %esi
	leaq	448(%rsp), %rdi
	leaq	1408(%rsp), %r8
	leaq	176(%rsp), %r9
	movq	%rbx, %rdx
	movq	%r14, %rcx
	callq	purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>
.Ltmp7336:
.LBB240_74:
	movq	448(%rsp), %r14
	cmpq	$-1, %r14
	je	.LBB240_79
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
.Ltmp7340:
	leaq	880(%rsp), %rdi
	leaq	448(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp7341:
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
	jne	.LBB240_80
.Ltmp7459:
	leaq	1408(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp7460:
	movq	96(%rsp), %r12
	movl	$0, 24(%rsp)
	jmp	.LBB240_499
.LBB240_79:
	vmovdqu64	496(%rsp), %zmm0
	vmovdqu	464(%rsp), %ymm1
	vmovdqu64	%zmm0, 48(%r12)
	vmovdqu	%ymm1, 16(%r12)
	vmovdqu64	%zmm0, 880(%rsp)
	movq	$1, (%r12)
	movq	160(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB240_237
	jmp	.LBB240_239
.LBB240_80:
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
.Ltmp7343:
	leaq	1352(%rsp), %rdi
	leaq	448(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp7344:
	movq	1368(%rsp), %rcx
	movq	1360(%rsp), %rbp
	imulq	$200, %rcx, %rax
	addq	%rbp, %rax
	movq	%rax, 40(%rsp)
	testq	%rcx, %rcx
	je	.LBB240_141
	movl	%ecx, %esi
	andl	$3, %esi
	cmpq	$4, %rcx
	jae	.LBB240_116
	xorl	%edi, %edi
	xorl	%r10d, %r10d
	jmp	.LBB240_135
.LBB240_84:
	movq	%rbx, 312(%rsp)
	cmpq	$6, %r13
	jb	.LBB240_89
	movq	1416(%rsp), %rdi
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB240_89
.LBB240_86:
	vmovups	480(%rsp), %zmm1
	vmovups	464(%rsp), %zmm0
	movl	%edx, %esi
	shrl	$8, %esi
	vmovups	%zmm1, 896(%rsp)
	vmovups	%zmm0, 880(%rsp)
	jmp	.LBB240_153
.LBB240_87:
	movq	%r15, %rbx
.LBB240_88:
	movq	%rbx, 312(%rsp)
.LBB240_89:
	subq	%rbx, %r15
	je	.LBB240_102
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	shrq	$3, %r15
	movabsq	$-3689348814741910323, %r14
	imulq	%r15, %r14
	xorl	%r15d, %r15d
	jmp	.LBB240_94
	.p2align	4
.LBB240_91:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB240_92:
	vzeroupper
	callq	*%r13
.LBB240_93:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB240_102
.LBB240_94:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB240_93
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
	jge	.LBB240_97
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB240_97:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB240_92
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB240_97
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
.LBB240_100:
	cmpq	%rax, %rdx
	jge	.LBB240_91
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB240_100
	jmp	.LBB240_91
.LBB240_102:
	movq	752(%rsp), %rax
	movq	872(%rsp), %rbx
	movq	32(%rsp), %r14
	testq	%rax, %rax
	je	.LBB240_112
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
	jge	.LBB240_105
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB240_105:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB240_111
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB240_105
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
.LBB240_108:
	cmpq	%rax, %rdx
	jge	.LBB240_110
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB240_108
.LBB240_110:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB240_111:
	movq	free@GOTPCREL(%rip), %rax
	movq	24(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB240_112:
	vmovdqu	1120(%rsp), %xmm0
	movq	1136(%rsp), %rax
	movq	112(%rsp), %r12
	movl	$0, 16(%rsp)
	movq	%rax, 864(%rsp)
	vmovdqa	%xmm0, 848(%rsp)
	movq	696(%r14), %rax
	movl	40(%rax), %ecx
	testl	%ecx, %ecx
	jne	.LBB240_113
	jmp	.LBB240_520
.LBB240_116:
	movq	%rcx, %r8
	andq	$-4, %r8
	leaq	776(%rbp), %r9
	xorl	%edi, %edi
	xorl	%r10d, %r10d
	jmp	.LBB240_118
	.p2align	4
.LBB240_117:
	addq	$4, %rdi
	addq	$800, %r9
	cmpq	%rdi, %r8
	je	.LBB240_134
.LBB240_118:
	movq	-600(%r9), %rax
	mulq	-608(%r9)
	jo	.LBB240_127
	addq	%rax, %r10
	movq	$-1, %r11
	jb	.LBB240_120
.LBB240_128:
	movq	%r10, %r11
	movq	-400(%r9), %rax
	mulq	-408(%r9)
	jno	.LBB240_121
.LBB240_129:
	movq	$-1, %rax
	addq	%rax, %r11
	movq	$-1, %r10
	jae	.LBB240_130
	.p2align	4
.LBB240_122:
	movq	-200(%r9), %rax
	mulq	-208(%r9)
	jo	.LBB240_131
.LBB240_123:
	addq	%rax, %r10
	movq	$-1, %r11
	jb	.LBB240_124
.LBB240_132:
	movq	%r10, %r11
	movq	(%r9), %rax
	mulq	-8(%r9)
	jno	.LBB240_125
.LBB240_133:
	movq	$-1, %rax
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB240_117
	jmp	.LBB240_126
.LBB240_127:
	movq	$-1, %rax
	addq	%rax, %r10
	movq	$-1, %r11
	jae	.LBB240_128
	.p2align	4
.LBB240_120:
	movq	-400(%r9), %rax
	mulq	-408(%r9)
	jo	.LBB240_129
.LBB240_121:
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB240_122
.LBB240_130:
	movq	%r11, %r10
	movq	-200(%r9), %rax
	mulq	-208(%r9)
	jno	.LBB240_123
.LBB240_131:
	movq	$-1, %rax
	addq	%rax, %r10
	movq	$-1, %r11
	jae	.LBB240_132
	.p2align	4
.LBB240_124:
	movq	(%r9), %rax
	mulq	-8(%r9)
	jo	.LBB240_133
.LBB240_125:
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB240_117
.LBB240_126:
	movq	%r11, %r10
	jmp	.LBB240_117
.LBB240_134:
	testq	%rsi, %rsi
	je	.LBB240_138
.LBB240_135:
	imulq	$200, %rdi, %rax
	imulq	$200, %rsi, %rsi
	movq	$-1, %r9
	xorl	%r8d, %r8d
	leaq	176(%rax,%rbp), %rdi
	.p2align	4
.LBB240_136:
	movq	(%rdi,%r8), %rax
	mulq	-8(%rdi,%r8)
	jo	.LBB240_813
.LBB240_137:
	addq	%rax, %r10
	cmovbq	%r9, %r10
	addq	$200, %r8
	cmpq	%r8, %rsi
	jne	.LBB240_136
	jmp	.LBB240_138
.LBB240_813:
	movq	$-1, %rax
	jmp	.LBB240_137
.LBB240_138:
	testq	%r10, %r10
	je	.LBB240_141
	cmpq	$0, 336(%rbx)
	je	.LBB240_141
	lock		addq	%r10, 352(%rbx)
.LBB240_141:
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
	je	.LBB240_814
	leaq	456(%rsp), %r14
	addq	$200, %rbp
	movl	$8, %ecx
	xorl	%r15d, %r15d
	xorl	%r12d, %r12d
	.p2align	4
.LBB240_143:
	movq	-200(%rbp), %rax
	cmpq	$-1, %rax
	je	.LBB240_150
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
	jne	.LBB240_147
.Ltmp7346:
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp7347:
	movq	184(%rsp), %rcx
.LBB240_147:
	vmovdqu64	448(%rsp), %zmm0
	vmovdqu64	512(%rsp), %zmm1
	vmovdqu64	544(%rsp), %zmm2
	leaq	1(%r13), %rax
	vmovdqu64	%zmm2, 96(%rcx,%r12)
	vmovdqu64	%zmm1, 64(%rcx,%r12)
	vmovdqu64	%zmm0, (%rcx,%r12)
	movq	%rax, 192(%rsp)
	testb	%r15b, %r15b
	jne	.LBB240_186
	addq	$160, %r12
	addq	$200, %rbp
	addq	$200, %rbx
	movq	%rax, %r15
	cmpq	40(%rsp), %rbx
	jne	.LBB240_143
	movq	40(%rsp), %rbp
	movq	%rax, %r15
.LBB240_150:
	movq	%rcx, %r12
	jmp	.LBB240_151
.LBB240_814:
	movl	$8, %r12d
	xorl	%r15d, %r15d
.LBB240_151:
	movq	%rbp, 888(%rsp)
	xorl	%ebp, %ebp
	jmp	.LBB240_187
.LBB240_152:
	movzbl	459(%rsp), %ecx
	movzwl	457(%rsp), %esi
	vmovups	464(%rsp), %zmm0
	vmovups	480(%rsp), %zmm1
	shll	$16, %ecx
	orl	%ecx, %esi
	movl	460(%rsp), %ecx
	vmovups	%zmm0, 880(%rsp)
	vmovups	%zmm1, 896(%rsp)
.LBB240_153:
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
	jb	.LBB240_155
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
	movq	%r14, %rdi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB240_155:
	subq	%rbx, %r15
	je	.LBB240_168
	shrq	$3, %r15
	movabsq	$-3689348814741910323, %r14
	imulq	%r15, %r14
	xorl	%r15d, %r15d
	jmp	.LBB240_160
.LBB240_157:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB240_158:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB240_159:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB240_168
.LBB240_160:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB240_159
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
	jge	.LBB240_163
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB240_163:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB240_158
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB240_163
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
.LBB240_166:
	cmpq	%rax, %rdx
	jge	.LBB240_157
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB240_166
	jmp	.LBB240_157
.LBB240_168:
	movq	752(%rsp), %rax
	testq	%rax, %rax
	je	.LBB240_170
	movq	24(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB240_170:
	movq	16(%rsp), %r15
	movq	40(%rsp), %r14
	testq	%r15, %r15
	je	.LBB240_183
	xorl	%ebx, %ebx
	jmp	.LBB240_175
.LBB240_172:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB240_173:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB240_174:
	incq	%rbx
	cmpq	%r15, %rbx
	je	.LBB240_183
.LBB240_175:
	leaq	(%rbx,%rbx,4), %rcx
	movq	(%r14,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB240_174
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
	jge	.LBB240_178
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB240_178:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB240_173
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB240_178
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
.LBB240_181:
	cmpq	%rax, %rdx
	jge	.LBB240_172
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB240_181
	jmp	.LBB240_172
.LBB240_183:
	movq	1120(%rsp), %rax
	testq	%rax, %rax
	je	.LBB240_185
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r14, %rdi
	leaq	(%rax,%rax,4), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB240_185:
	movq	112(%rsp), %r12
	movl	$0, 16(%rsp)
	jmp	.LBB240_579
.LBB240_186:
	movq	%rbp, 888(%rsp)
	incq	%r13
	movb	$1, %bpl
	movq	%rcx, %r12
	movq	%r13, %r15
.LBB240_187:
.Ltmp7354:
	leaq	880(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp7355:
	movq	176(%rsp), %r13
	testq	%r15, %r15
	je	.LBB240_191
	cmpq	$8, %r15
	jae	.LBB240_192
	xorl	%eax, %eax
	xorl	%ebx, %ebx
	jmp	.LBB240_201
.LBB240_191:
	xorl	%ebx, %ebx
	jmp	.LBB240_203
.LBB240_192:
	cmpq	$32, %r15
	jae	.LBB240_194
	xorl	%eax, %eax
	xorl	%ebx, %ebx
	jmp	.LBB240_198
.LBB240_194:
	vmovdqa64	.LCPI240_0(%rip), %zmm1
	vpbroadcastq	.LCPI240_1(%rip), %zmm2
	vpbroadcastq	.LCPI240_2(%rip), %zmm3
	movq	%r15, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
	.p2align	4
.LBB240_195:
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
	jne	.LBB240_195
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
	je	.LBB240_203
	testb	$24, %r15b
	je	.LBB240_201
.LBB240_198:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI240_0(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI240_1(%rip), %zmm2
	vpbroadcastq	.LCPI240_3(%rip), %zmm3
	movq	%r15, %rax
	andq	$-8, %rax
	vmovq	%rbx, %xmm0
	subq	%rax, %rcx
	.p2align	4
.LBB240_199:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	64(%r12,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB240_199
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rbx
	cmpq	%rax, %r15
	je	.LBB240_203
.LBB240_201:
	movq	%r15, %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	64(%rax,%r12), %rax
	.p2align	4
.LBB240_202:
	addq	(%rax), %rbx
	addq	$160, %rax
	decq	%rcx
	jne	.LBB240_202
.LBB240_203:
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
.Ltmp7360:
	movq	16(%rsp), %rdx
	leaq	448(%rsp), %rdi
	leaq	880(%rsp), %rsi
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp7361:
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
	je	.LBB240_240
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
	je	.LBB240_218
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r12
	movabsq	$9223372036854775807, %r15
	xorl	%r14d, %r14d
	jmp	.LBB240_210
.LBB240_207:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB240_208:
	vzeroupper
	callq	*%r12
.LBB240_209:
	incq	%r14
	cmpq	16(%rsp), %r14
	je	.LBB240_218
.LBB240_210:
	leaq	(%r14,%r14,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB240_209
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
	jge	.LBB240_213
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB240_213:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB240_208
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB240_213
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
.LBB240_216:
	cmpq	%rax, %rdx
	jge	.LBB240_207
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB240_216
	jmp	.LBB240_207
.LBB240_218:
	addq	152(%rsp), %r13
	movq	704(%rsp), %rax
	testq	%rax, %rax
	je	.LBB240_220
	shlq	$3, %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	leaq	(%rax,%rax,4), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB240_220:
	shlq	$8, %r13
	movq	112(%rsp), %r12
	movl	92(%rsp), %ebp
	movq	752(%rsp), %r15
	movq	104(%rsp), %rbx
	movq	%r13, 16(%rsp)
	movq	32(%rsp), %r13
	testq	%r15, %r15
	je	.LBB240_224
.LBB240_221:
	movq	64(%rsp), %r14
	movq	%r15, %rax
	movl	$1, %r15d
	subq	%rax, %r15
	.p2align	4
.LBB240_222:
.Ltmp7447:
	movq	%r14, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7448:
	incq	%r15
	addq	$160, %r14
	cmpq	$1, %r15
	jne	.LBB240_222
.LBB240_224:
	movq	48(%rsp), %rax
	testq	%rax, %rax
	je	.LBB240_234
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
	jge	.LBB240_227
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB240_227:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB240_233
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB240_227
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
.LBB240_230:
	cmpq	%rax, %rsi
	jge	.LBB240_232
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB240_230
.LBB240_232:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB240_233:
	movq	free@GOTPCREL(%rip), %rax
	movq	64(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB240_234:
	cmpq	$-1, %rbx
	je	.LBB240_246
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
.Ltmp7453:
	leaq	1264(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7454:
	movq	160(%rsp), %rax
	testq	%rax, %rax
	je	.LBB240_239
.LBB240_237:
	lock		decq	(%rax)
	jne	.LBB240_239
	#MEMBARRIER
.Ltmp7512:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp7513:
.LBB240_239:
	movb	$1, %al
	movl	%eax, 16(%rsp)
	jmp	.LBB240_579
.LBB240_240:
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
	je	.LBB240_247
	movb	%r14b, 91(%rsp)
	lock		incq	(%rax)
	jle	.LBB240_696
	movq	616(%r13), %r14
	movq	48(%rsp), %r13
	movq	%r14, 808(%rsp)
	movq	%r14, 24(%rsp)
	movq	16(%r14), %rax
	movq	40(%r14), %rcx
	movq	%rax, 288(%rsp)
	movq	%rcx, 136(%rsp)
	cmpq	$-1, %rcx
	je	.LBB240_270
	testq	%r15, %r15
	je	.LBB240_254
	movq	32(%rsp), %rdx
	cmpq	$8, %r15
	jae	.LBB240_255
	xorl	%eax, %eax
	xorl	%esi, %esi
	jmp	.LBB240_267
.LBB240_246:
	movq	%r13, %r15
	movq	24(%rsp), %r13
	movq	80(%rsp), %r14
	movq	40(%rsp), %r12
	jmp	.LBB240_488
.LBB240_247:
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
	je	.LBB240_259
	movq	24(%rsp), %rax
	addq	$40, %r12
	movq	%r12, %rcx
	leaq	(,%rax,8), %rax
	leaq	(%rax,%rax,4), %r14
	leaq	264(%rsp), %rax
	movq	%rax, 80(%rsp)
	jmp	.LBB240_250
.LBB240_249:
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
	je	.LBB240_257
.LBB240_250:
	movq	-40(%rcx), %rbp
	movq	264(%rsp), %rax
	movq	%rcx, %r12
	testq	%rbp, %rbp
	je	.LBB240_258
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
	jne	.LBB240_249
.Ltmp7441:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	264(%rsp), %rdi
	callq	*%rax
.Ltmp7442:
	movq	272(%rsp), %rax
	movq	%rax, 40(%rsp)
	jmp	.LBB240_249
.LBB240_254:
	movq	32(%rsp), %rdx
	xorl	%esi, %esi
	jmp	.LBB240_269
.LBB240_255:
	cmpq	$32, %r15
	jae	.LBB240_260
	xorl	%eax, %eax
	xorl	%esi, %esi
	jmp	.LBB240_264
.LBB240_257:
	movq	264(%rsp), %rax
.LBB240_258:
	movq	272(%rsp), %rcx
	movl	92(%rsp), %ebp
	movq	%rax, 80(%rsp)
	movq	%rcx, 40(%rsp)
.LBB240_259:
	leaq	448(%rsp), %rdi
	movq	%r12, 456(%rsp)
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	112(%rsp), %r12
	movb	$2, %al
	movq	$-1, %rbx
	movq	$0, 16(%rsp)
	movl	%eax, 72(%rsp)
	testq	%r15, %r15
	jne	.LBB240_221
	jmp	.LBB240_224
.LBB240_260:
	vmovdqa64	.LCPI240_0(%rip), %zmm1
	vpbroadcastq	.LCPI240_1(%rip), %zmm2
	vpbroadcastq	.LCPI240_2(%rip), %zmm3
	movq	%r15, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
.LBB240_261:
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
	jne	.LBB240_261
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
	je	.LBB240_269
	testb	$24, %r15b
	je	.LBB240_267
.LBB240_264:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI240_0(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI240_1(%rip), %zmm2
	vpbroadcastq	.LCPI240_3(%rip), %zmm3
	movq	%r15, %rax
	andq	$-8, %rax
	vmovq	%rsi, %xmm0
	subq	%rax, %rcx
.LBB240_265:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	16(%r12,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB240_265
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rsi
	cmpq	%rax, %r15
	je	.LBB240_269
.LBB240_267:
	movq	%r15, %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	16(%rax,%r12), %rax
.LBB240_268:
	addq	(%rax), %rsi
	addq	$160, %rax
	decq	%rcx
	jne	.LBB240_268
.LBB240_269:
	movb	$1, %bl
	leaq	888(%rdx), %rdi
.Ltmp7363:
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts@GOTPCREL(%rip), %rax
	movb	$1, %bpl
	vzeroupper
	callq	*%rax
.Ltmp7364:
.LBB240_270:
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
	je	.LBB240_443
	movq	32(%rsp), %rcx
	leaq	272(%r14), %rax
	movq	$-1, %r15
	movq	%rax, 800(%rsp)
	leaq	888(%rcx), %rax
	movq	%rax, 80(%rsp)
.LBB240_272:
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
	je	.LBB240_443
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
	je	.LBB240_431
	movq	544(%rsp), %r13
	shlq	$5, %rax
	movq	$0, 744(%rsp)
	movq	%r12, 64(%rsp)
	addq	%rdx, %rax
	movq	%rax, 1184(%rsp)
	leaq	8(%rcx), %rax
	movq	%rax, 1176(%rsp)
	jmp	.LBB240_277
.LBB240_275:
	movq	96(%rsp), %rbx
.LBB240_276:
	movq	1208(%rsp), %rdx
	movq	%rbx, 824(%rsp)
	addq	$32, %rdx
	cmpq	1184(%rsp), %rdx
	je	.LBB240_431
.LBB240_277:
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
	je	.LBB240_287
	cmpq	$-1, 288(%rsp)
	je	.LBB240_287
	movq	24(%rsp), %rdx
	movq	80(%rdx), %rax
	.p2align	4
.LBB240_280:
	movq	%rax, %rcx
	addq	%r14, %rcx
	cmovbq	%r15, %rcx
	lock		cmpxchgq	%rcx, 80(%rdx)
	jne	.LBB240_280
	movq	128(%rsp), %rcx
	addq	%r14, %rax
	cmovbq	%r15, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB240_284
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$0, 176(%rsp)
.Ltmp7365:
	movq	128(%rsp), %rsi
	leaq	304(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.4261325137610144415)
.Ltmp7366:
	cmpb	$-1, 304(%rsp)
	jne	.LBB240_467
.LBB240_284:
	movq	32(%rsp), %rax
	movq	632(%rax), %rax
	testq	%rax, %rax
	je	.LBB240_287
	movq	32(%rsp), %rcx
	movl	1228(%rcx), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB240_287
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	movq	1192(%rsp), %rax
	lock		addq	%r14, (%rcx,%rax,8)
.LBB240_287:
	movq	1200(%rsp), %rdx
	movq	296(%rsp), %rdi
	cmpq	%rdx, %rdi
	ja	.LBB240_694
	cmpq	%rdx, %rbx
	movq	%rdx, %rsi
	cmovbq	%rbx, %rsi
	cmpq	%rdi, %rbx
	cmovbq	%rdi, %rsi
	cmpq	%rdi, %rsi
	jb	.LBB240_693
	leaq	(,%rdi,8), %rax
	movq	%rsi, 744(%rsp)
	leaq	(%rax,%rax,2), %r14
	leaq	(,%rsi,8), %rax
	leaq	(%rax,%rax,2), %rbx
	cmpq	%rsi, %rdi
	jne	.LBB240_296
	movq	$0, 72(%rsp)
	movq	$0, 704(%rsp)
	movq	$0, 104(%rsp)
.LBB240_291:
	cmpq	$-1, 136(%rsp)
	movq	%r13, 152(%rsp)
	movq	$-1, %r13
	je	.LBB240_302
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
	je	.LBB240_303
	movq	16(%rsp), %rax
	xorl	%r12d, %r12d
	leaq	8(%rax), %rbp
	.p2align	4
.LBB240_294:
.Ltmp7368:
	movq	purrdf_sparql_eval::scratch::value_bytes@GOTPCREL(%rip), %rax
	movq	%rbp, %rdi
	vzeroupper
	callq	*%rax
.Ltmp7369:
	addq	%rax, %r12
	cmovbq	%r13, %r12
	addq	$88, %rbp
	decq	%r15
	jne	.LBB240_294
	jmp	.LBB240_303
.LBB240_296:
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
	jmp	.LBB240_299
	.p2align	4
.LBB240_297:
	movq	72(%rsp), %rsi
	addq	%rdx, %rsi
	cmovbq	%r15, %rsi
	movq	%rsi, 72(%rsp)
.LBB240_298:
	addq	$24, %rcx
	decq	%rax
	je	.LBB240_291
.LBB240_299:
	movzbl	-8(%rcx), %esi
	leaq	.LJTI240_0(%rip), %rdi
	movq	(%rcx), %rdx
	movslq	(%rdi,%rsi,4), %rsi
	addq	%rdi, %rsi
	jmpq	*%rsi
.LBB240_300:
	movq	704(%rsp), %rsi
	addq	%rdx, %rsi
	cmovbq	%r15, %rsi
	movq	%rsi, 704(%rsp)
	jmp	.LBB240_298
	.p2align	4
.LBB240_301:
	movq	104(%rsp), %rsi
	cmpq	%rdx, %rsi
	cmovbeq	%rdx, %rsi
	movq	%rsi, 104(%rsp)
	jmp	.LBB240_298
.LBB240_302:
	xorl	%r12d, %r12d
.LBB240_303:
	movq	24(%rsp), %rax
	movq	$-1, %r15
	movl	296(%rax), %eax
	testl	%eax, %eax
	je	.LBB240_319
.LBB240_304:
	cmpq	$-1, 288(%rsp)
	je	.LBB240_306
	movq	24(%rsp), %rcx
	movq	80(%rcx), %rax
	addq	704(%rsp), %rax
	cmovbq	%r15, %rax
	cmpq	16(%rcx), %rax
	ja	.LBB240_320
.LBB240_306:
	cmpq	$-1, 136(%rsp)
	je	.LBB240_308
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
	ja	.LBB240_320
.LBB240_308:
	cmpq	$-1, 288(%rsp)
	movq	32(%rsp), %r9
	movq	152(%rsp), %r13
	movq	744(%rsp), %r10
	je	.LBB240_310
	movq	%r14, %rax
	cmpq	%r10, 296(%rsp)
	jne	.LBB240_314
.LBB240_310:
	movq	64(%rsp), %r12
	movb	$1, %bpl
	cmpq	%r10, 296(%rsp)
	je	.LBB240_393
.LBB240_311:
	movq	752(%rsp), %rax
	cmpb	$2, -24(%rax,%rbx)
	je	.LBB240_378
	addq	$-24, %rbx
	cmpq	%rbx, %r14
	jne	.LBB240_311
	jmp	.LBB240_393
	.p2align	4
.LBB240_313:
	addq	$24, %rax
	cmpq	%rax, %rbx
	je	.LBB240_310
.LBB240_314:
	movq	752(%rsp), %rcx
	cmpb	$0, (%rcx,%rax)
	jne	.LBB240_313
	movq	752(%rsp), %rcx
	movzbl	1(%rcx,%rax), %ecx
	cmpq	$255, %rcx
	je	.LBB240_313
	movq	632(%r9), %rdx
	testq	%rdx, %rdx
	je	.LBB240_313
	movl	1228(%r9), %esi
	cmpq	%rsi, 56(%rdx)
	jbe	.LBB240_313
	movq	752(%rsp), %rdi
	movq	%rsi, %r8
	shlq	$7, %r8
	leaq	(%r8,%rsi,8), %rsi
	addq	48(%rdx), %rsi
	movq	8(%rdi,%rax), %rdi
	lock		addq	%rdi, (%rsi,%rcx,8)
	jmp	.LBB240_313
.LBB240_319:
	movq	800(%rsp), %rax
	cmpb	$-1, (%rax)
	je	.LBB240_304
.LBB240_320:
	movq	152(%rsp), %r13
	movq	296(%rsp), %rax
	cmpq	744(%rsp), %rax
	jne	.LBB240_330
	movq	64(%rsp), %r12
.LBB240_322:
	cmpq	$-1, 136(%rsp)
	je	.LBB240_377
	movq	24(%rsp), %r14
	cmpq	144(%rsp), %r13
	jae	.LBB240_422
	movq	16(%rsp), %rdx
	cmpq	48(%rsp), %rdx
	je	.LBB240_386
	movq	144(%rsp), %rax
	addq	$88, %rdx
	leaq	-1(%rax), %rbx
	movq	%rdx, %rax
.LBB240_326:
	movq	%rax, %rdx
	movq	-8(%rdx), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rdx, 16(%rsp)
	movq	%rcx, 368(%rsp)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, 304(%rsp)
	cmpq	$-1, %rax
	je	.LBB240_385
	vmovdqu64	304(%rsp), %zmm0
	movq	%rax, 176(%rsp)
	movq	368(%rsp), %rax
	leaq	184(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7412:
	movq	80(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp7413:
	cmpq	%r13, %rbx
	je	.LBB240_387
	movq	16(%rsp), %rdx
	incq	%r13
	leaq	-88(%rdx), %rcx
	leaq	88(%rdx), %rax
	addq	$88, %rcx
	cmpq	48(%rsp), %rcx
	jne	.LBB240_326
	jmp	.LBB240_386
.LBB240_330:
	movq	752(%rsp), %rax
	movq	64(%rsp), %r12
	addq	%rax, %r14
	addq	%rax, %rbx
	jmp	.LBB240_334
.LBB240_331:
	movq	64(%rsp), %r12
.LBB240_332:
	movq	$-1, %r15
.LBB240_333:
	addq	$24, %r14
	cmpq	%rbx, %r14
	je	.LBB240_322
.LBB240_334:
	movzbl	(%r14), %eax
	leaq	.LJTI240_1(%rip), %rcx
	movslq	(%rcx,%rax,4), %rax
	addq	%rcx, %rax
	jmpq	*%rax
.LBB240_335:
	cmpq	$-1, 288(%rsp)
	je	.LBB240_333
	movq	24(%rsp), %rdx
	movzbl	1(%r14), %r15d
	movq	8(%r14), %r12
	movq	16(%r14), %rbp
	movq	$-1, %rsi
	movq	80(%rdx), %rax
	.p2align	4
.LBB240_337:
	movq	%rax, %rcx
	addq	%r12, %rcx
	cmovbq	%rsi, %rcx
	lock		cmpxchgq	%rcx, 80(%rdx)
	jne	.LBB240_337
	movq	128(%rsp), %rcx
	addq	%r12, %rax
	cmovbq	%rsi, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB240_341
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$0, 176(%rsp)
.Ltmp7399:
	movq	128(%rsp), %rsi
	leaq	304(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.4261325137610144415)
.Ltmp7400:
	cmpb	$-1, 304(%rsp)
	jne	.LBB240_450
.LBB240_341:
	cmpl	$255, %r15d
	je	.LBB240_331
	movq	32(%rsp), %rcx
	movq	632(%rcx), %rax
	testq	%rax, %rax
	je	.LBB240_331
	movl	1228(%rcx), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB240_331
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	lock		addq	%r12, (%rcx,%r15,8)
	jmp	.LBB240_331
.LBB240_345:
	movq	8(%r14), %r15
	cmpq	%r15, %r13
	jae	.LBB240_374
	movq	16(%rsp), %rax
	cmpq	48(%rsp), %rax
	je	.LBB240_367
	addq	$88, %rax
	leaq	-1(%r15), %r12
	.p2align	4
.LBB240_348:
	movq	%rax, %rdx
	movq	-8(%rdx), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rdx, 16(%rsp)
	movq	%rcx, 368(%rsp)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, 304(%rsp)
	cmpq	$-1, %rax
	je	.LBB240_371
	vmovdqu64	304(%rsp), %zmm0
	movq	%rax, 176(%rsp)
	movq	368(%rsp), %rax
	leaq	184(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7388:
	movq	80(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp7389:
	cmpq	%r13, %r12
	je	.LBB240_373
	movq	16(%rsp), %rdx
	incq	%r13
	leaq	-88(%rdx), %rcx
	leaq	88(%rdx), %rax
	addq	$88, %rcx
	cmpq	48(%rsp), %rcx
	jne	.LBB240_348
	jmp	.LBB240_372
.LBB240_352:
	cmpq	$-1, 136(%rsp)
	je	.LBB240_333
	movq	24(%rsp), %rdx
	movq	8(%r14), %rcx
	movl	296(%rdx), %eax
	testl	%eax, %eax
	je	.LBB240_364
	movq	104(%rdx), %rax
	addq	%rcx, %rax
	movq	40(%rdx), %rcx
	cmovbq	%r15, %rax
	cmpq	%rcx, %rax
	jbe	.LBB240_333
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$768, 176(%rsp)
.Ltmp7386:
	movq	128(%rsp), %rsi
	leaq	304(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.4261325137610144415)
.Ltmp7387:
	movq	$-1, %r15
	jmp	.LBB240_365
.LBB240_357:
	cmpq	$-1, 136(%rsp)
	je	.LBB240_333
	movq	24(%rsp), %rsi
	cmpq	$-1, 40(%rsi)
	je	.LBB240_333
	movq	8(%r14), %rcx
	movq	16(%r14), %r15
	movl	296(%rsi), %eax
	testl	%eax, %eax
	je	.LBB240_368
	movq	104(%rsi), %rax
	movq	$-1, %rdi
	.p2align	4
.LBB240_361:
	movq	%rax, %rdx
	addq	%rcx, %rdx
	cmovbq	%rdi, %rdx
	lock		cmpxchgq	%rdx, 104(%rsi)
	jne	.LBB240_361
	addq	%rcx, %rax
	movq	40(%rsi), %rcx
	cmovbq	%rdi, %rax
	cmpq	%rcx, %rax
	jbe	.LBB240_332
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$768, 176(%rsp)
.Ltmp7393:
	movq	128(%rsp), %rsi
	leaq	304(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.4261325137610144415)
.Ltmp7394:
	jmp	.LBB240_369
.LBB240_364:
	movq	800(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 320(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 304(%rsp)
.LBB240_365:
	cmpb	$-1, 304(%rsp)
	je	.LBB240_333
	jmp	.LBB240_366
.LBB240_367:
	movq	%rax, %rdx
	jmp	.LBB240_372
.LBB240_368:
	movq	800(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 320(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 304(%rsp)
.LBB240_369:
	movzbl	304(%rsp), %eax
	cmpb	$-1, %al
	sete	%cl
	testq	%r15, %r15
	sete	%dl
	orb	%cl, %dl
	je	.LBB240_459
	cmpb	$-1, %al
	movq	$-1, %r15
	je	.LBB240_333
	jmp	.LBB240_366
.LBB240_371:
	movq	16(%rsp), %rdx
.LBB240_372:
	movq	64(%rsp), %r12
	movq	%rdx, 16(%rsp)
	movq	%rdx, 392(%rsp)
	cmpq	$-1, 136(%rsp)
	jne	.LBB240_375
	jmp	.LBB240_332
.LBB240_373:
	movq	64(%rsp), %r12
	movq	%r15, %r13
.LBB240_374:
	movq	16(%rsp), %rax
	movq	%rax, 392(%rsp)
	cmpq	$-1, 136(%rsp)
	je	.LBB240_332
.LBB240_375:
.Ltmp7391:
	movq	32(%rsp), %rsi
	leaq	176(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp7392:
	cmpb	$-1, 176(%rsp)
	movq	$-1, %r15
	je	.LBB240_333
	jmp	.LBB240_366
.LBB240_377:
	movq	24(%rsp), %r14
	jmp	.LBB240_422
.LBB240_378:
	movq	752(%rsp), %rax
	movq	-16(%rax,%rbx), %rbx
	cmpq	%rbx, %r13
	jae	.LBB240_391
	movq	16(%rsp), %rdx
	cmpq	48(%rsp), %rdx
	je	.LBB240_389
	addq	$88, %rdx
	leaq	-1(%rbx), %r14
	movq	%rdx, %rax
.LBB240_381:
	movq	%rax, %rdx
	movq	-8(%rdx), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rdx, 16(%rsp)
	movq	%rcx, 368(%rsp)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, 304(%rsp)
	cmpq	$-1, %rax
	je	.LBB240_388
	vmovdqu64	304(%rsp), %zmm0
	movq	%rax, 176(%rsp)
	movq	368(%rsp), %rax
	leaq	184(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7371:
	movq	80(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp7372:
	cmpq	%r13, %r14
	je	.LBB240_390
	movq	16(%rsp), %rdx
	incq	%r13
	leaq	-88(%rdx), %rcx
	leaq	88(%rdx), %rax
	addq	$88, %rcx
	cmpq	48(%rsp), %rcx
	jne	.LBB240_381
	jmp	.LBB240_389
.LBB240_385:
	movq	16(%rsp), %rdx
.LBB240_386:
	movq	%rdx, 16(%rsp)
	movq	%rdx, 392(%rsp)
	jmp	.LBB240_422
.LBB240_387:
	movq	16(%rsp), %rax
	movq	144(%rsp), %r13
	movq	%rax, 392(%rsp)
	jmp	.LBB240_422
.LBB240_388:
	movq	16(%rsp), %rdx
.LBB240_389:
	movq	%rdx, 16(%rsp)
	movq	%rdx, 392(%rsp)
	jmp	.LBB240_392
.LBB240_390:
	movq	%rbx, %r13
.LBB240_391:
	movq	16(%rsp), %rax
	movq	%rax, 392(%rsp)
.LBB240_392:
	xorl	%ebp, %ebp
.LBB240_393:
	movq	24(%rsp), %r14
	movq	56(%rsp), %rbx
	cmpq	$-1, 288(%rsp)
	je	.LBB240_398
	cmpq	$0, 704(%rsp)
	je	.LBB240_398
	movq	704(%rsp), %rax
	movq	%rax, 304(%rsp)
	movq	$0, 312(%rsp)
.Ltmp7374:
	movq	128(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	leaq	176(%rsp), %rdi
	leaq	304(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp7375:
	cmpb	$-1, 192(%rsp)
	je	.LBB240_398
	cmpq	$-1, 136(%rsp)
	movb	$1, %bpl
	jne	.LBB240_410
	jmp	.LBB240_472
.LBB240_398:
	cmpq	$-1, 136(%rsp)
	je	.LBB240_422
	cmpq	$-1, 40(%r14)
	je	.LBB240_403
.Ltmp7376:
	movq	128(%rsp), %rsi
	movq	72(%rsp), %rcx
	leaq	176(%rsp), %rdi
	movl	$3, %edx
	xorl	%r8d, %r8d
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.4261325137610144415)
.Ltmp7377:
	cmpb	$-1, 176(%rsp)
	je	.LBB240_403
	movb	$1, %bpl
	jmp	.LBB240_410
.LBB240_403:
	testb	%bpl, %bpl
	jne	.LBB240_406
.Ltmp7378:
	movq	32(%rsp), %rsi
	leaq	176(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp7379:
	cmpb	$-1, 176(%rsp)
	movb	$1, %bpl
	jne	.LBB240_410
.LBB240_406:
	movq	104(%rsp), %rcx
	testq	%rcx, %rcx
	je	.LBB240_409
.Ltmp7380:
	movq	128(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::admit_transient@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	movl	$3, %edx
	vzeroupper
	callq	*%rax
.Ltmp7381:
	cmpb	$-1, 176(%rsp)
	setne	%bpl
	jmp	.LBB240_410
.LBB240_409:
	xorl	%ebp, %ebp
.LBB240_410:
	cmpq	144(%rsp), %r13
	jae	.LBB240_420
	movq	16(%rsp), %rdx
	cmpq	48(%rsp), %rdx
	je	.LBB240_418
	movq	144(%rsp), %rax
	addq	$88, %rdx
	leaq	-1(%rax), %rbx
	movq	%rdx, %rax
.LBB240_413:
	movq	%rax, %rdx
	movq	-8(%rdx), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rdx, 16(%rsp)
	movq	%rcx, 368(%rsp)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, 304(%rsp)
	cmpq	$-1, %rax
	je	.LBB240_417
	vmovdqu64	304(%rsp), %zmm0
	movq	%rax, 176(%rsp)
	movq	368(%rsp), %rax
	leaq	184(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7383:
	movq	80(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp7384:
	cmpq	%r13, %rbx
	je	.LBB240_419
	movq	16(%rsp), %rdx
	incq	%r13
	leaq	-88(%rdx), %rcx
	leaq	88(%rdx), %rax
	addq	$88, %rcx
	cmpq	48(%rsp), %rcx
	jne	.LBB240_413
	jmp	.LBB240_418
.LBB240_417:
	movq	16(%rsp), %rdx
.LBB240_418:
	movq	56(%rsp), %rbx
	movq	%rdx, 16(%rsp)
	movq	%rdx, 392(%rsp)
	jmp	.LBB240_421
.LBB240_419:
	movq	144(%rsp), %r13
	movq	56(%rsp), %rbx
.LBB240_420:
	movq	16(%rsp), %rax
	movq	%rax, 392(%rsp)
.LBB240_421:
	testb	%bpl, %bpl
	jne	.LBB240_471
.LBB240_422:
	cmpq	$0, 40(%rsp)
	je	.LBB240_275
	movq	840(%rsp), %rax
	movq	96(%rsp), %rsi
	movq	%r13, 152(%rsp)
	movq	%rax, 296(%rsp)
	jmp	.LBB240_425
	.p2align	4
.LBB240_424:
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
	je	.LBB240_429
.LBB240_425:
	movq	%rsi, %rbx
	cmpq	296(%rsp), %rsi
	je	.LBB240_430
	movq	(%rbx), %r12
	testq	%r12, %r12
	je	.LBB240_429
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
	jne	.LBB240_424
.Ltmp7415:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	264(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp7416:
	jmp	.LBB240_424
.LBB240_429:
	addq	$40, %rbx
.LBB240_430:
	movq	64(%rsp), %r12
	jmp	.LBB240_276
.LBB240_431:
	movq	56(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB240_433
	movq	792(%rsp), %rdi
	shlq	$5, %rsi
	movl	$8, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB240_433:
.Ltmp7426:
	leaq	384(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp7427:
	movq	736(%rsp), %rax
	testq	%rax, %rax
	je	.LBB240_436
	movq	752(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB240_436:
	movq	536(%rsp), %rax
	testq	%rax, %rax
	je	.LBB240_439
	lock		decq	(%rax)
	jne	.LBB240_439
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	536(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB240_439:
	movq	568(%rsp), %rax
	testq	%rax, %rax
	je	.LBB240_442
	lock		decq	(%rax)
	jne	.LBB240_442
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	568(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB240_442:
	cmpq	1168(%rsp), %r12
	jne	.LBB240_272
.LBB240_443:
	movq	32(%rsp), %r15
	movb	$1, %bl
	xorl	%ebp, %ebp
.Ltmp7431:
	leaq	1224(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp7432:
	cmpq	$-1, 288(%rsp)
	movzbl	91(%rsp), %ecx
	movl	92(%rsp), %ebp
	sete	%al
	xorb	$1, %cl
	orb	%bpl, %cl
	orb	%al, %cl
	cmpb	$1, %cl
	je	.LBB240_447
	movq	$1, 880(%rsp)
	xorl	%ebp, %ebp
	movq	$0, 888(%rsp)
.Ltmp7433:
	movq	<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items@GOTPCREL(%rip), %rax
	movq	128(%rsp), %rsi
	leaq	448(%rsp), %rdi
	leaq	880(%rsp), %rdx
	movl	$1, %ecx
	callq	*%rax
.Ltmp7434:
	movl	92(%rsp), %ebp
.LBB240_447:
	movq	264(%rsp), %rbx
	movq	272(%rsp), %r12
	movq	280(%rsp), %r13
	lock		decq	(%r14)
	movb	$2, %al
	movl	%eax, 72(%rsp)
	jne	.LBB240_449
	#MEMBARRIER
.Ltmp7438:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	808(%rsp), %rdi
	callq	*%rax
.Ltmp7439:
	movq	%rbx, %r14
	jmp	.LBB240_487
.LBB240_366:
	movb	$1, %bpl
.LBB240_468:
	movq	24(%rsp), %r14
	movq	56(%rsp), %rbx
	jmp	.LBB240_472
.LBB240_449:
	movq	%rbx, %r14
	jmp	.LBB240_487
.LBB240_450:
	testq	%rbp, %rbp
	je	.LBB240_470
	movq	56(%rsp), %rbx
	leaq	-1(%rbp), %rax
	cmpq	%rax, %r13
	jae	.LBB240_689
	movq	16(%rsp), %r15
	cmpq	48(%rsp), %r15
	je	.LBB240_458
	notq	%r13
	addq	$88, %r15
	leaq	176(%rsp), %r14
	addq	%rbp, %r13
	movq	%r15, %rax
.LBB240_454:
	movq	%rax, %r15
	movq	-8(%r15), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rcx, 368(%rsp)
	vmovdqu64	-72(%r15), %zmm0
	vmovdqu64	%zmm0, 304(%rsp)
	cmpq	$-1, %rax
	je	.LBB240_458
	vmovdqu64	304(%rsp), %zmm0
	movq	%rax, 176(%rsp)
	movq	368(%rsp), %rax
	leaq	184(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7402:
	movq	80(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%r14, %rdx
	vzeroupper
	callq	*%rax
.Ltmp7403:
	decq	%r13
	je	.LBB240_692
	leaq	-88(%r15), %rcx
	leaq	88(%r15), %rax
	addq	$88, %rcx
	cmpq	48(%rsp), %rcx
	jne	.LBB240_454
.LBB240_458:
	movq	24(%rsp), %r14
	movq	%r15, 392(%rsp)
	jmp	.LBB240_471
.LBB240_459:
	leaq	-1(%r15), %rax
	cmpq	%rax, %r13
	jae	.LBB240_469
	movq	16(%rsp), %r12
	movq	56(%rsp), %rbx
	cmpq	48(%rsp), %r12
	je	.LBB240_466
	notq	%r13
	addq	$88, %r12
	leaq	176(%rsp), %r14
	addq	%r15, %r13
	movq	%r12, %rax
.LBB240_462:
	movq	%rax, %r12
	movq	-8(%r12), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rcx, 368(%rsp)
	vmovdqu64	-72(%r12), %zmm0
	vmovdqu64	%zmm0, 304(%rsp)
	cmpq	$-1, %rax
	je	.LBB240_466
	vmovdqu64	304(%rsp), %zmm0
	movq	%rax, 176(%rsp)
	movq	368(%rsp), %rax
	leaq	184(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7396:
	movq	80(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%r14, %rdx
	vzeroupper
	callq	*%rax
.Ltmp7397:
	decq	%r13
	je	.LBB240_466
	leaq	-88(%r12), %rcx
	leaq	88(%r12), %rax
	addq	$88, %rcx
	cmpq	48(%rsp), %rcx
	jne	.LBB240_462
.LBB240_466:
	movq	24(%rsp), %r14
	movq	%r12, 392(%rsp)
	jmp	.LBB240_471
.LBB240_467:
	xorl	%ebp, %ebp
	jmp	.LBB240_468
.LBB240_469:
	movq	16(%rsp), %r12
	movq	24(%rsp), %r14
	movq	56(%rsp), %rbx
	movq	%r12, 392(%rsp)
	jmp	.LBB240_471
.LBB240_470:
	movq	24(%rsp), %r14
	movq	56(%rsp), %rbx
.LBB240_471:
	movb	$1, %bpl
.LBB240_472:
	movq	264(%rsp), %rax
	movq	32(%rsp), %r15
	movq	272(%rsp), %r12
	movq	280(%rsp), %r13
	movq	%rax, 704(%rsp)
	testq	%rbx, %rbx
	je	.LBB240_474
	movq	792(%rsp), %rdi
	shlq	$5, %rbx
	movl	$8, %edx
	movq	%rbx, %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB240_474:
.Ltmp7405:
	leaq	384(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp7406:
	movq	736(%rsp), %rax
	testq	%rax, %rax
	je	.LBB240_477
	movq	752(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB240_477:
	movq	536(%rsp), %rax
	testq	%rax, %rax
	je	.LBB240_480
	lock		decq	(%rax)
	jne	.LBB240_480
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	536(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB240_480:
	movq	568(%rsp), %rax
	movl	%ebp, 72(%rsp)
	testq	%rax, %rax
	je	.LBB240_483
	lock		decq	(%rax)
	jne	.LBB240_483
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	568(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB240_483:
	xorl	%ebx, %ebx
.Ltmp7408:
	leaq	1224(%rsp), %rdi
	xorl	%ebp, %ebp
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp7409:
	lock		decq	(%r14)
	jne	.LBB240_486
	#MEMBARRIER
.Ltmp7410:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	808(%rsp), %rdi
	callq	*%rax
.Ltmp7411:
	movl	92(%rsp), %ebp
	movq	704(%rsp), %r14
	jmp	.LBB240_487
.LBB240_486:
	movl	92(%rsp), %ebp
	movq	704(%rsp), %r14
.LBB240_487:
	leaq	816(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB240_488:
	movq	616(%r15), %rax
	movq	%r14, 880(%rsp)
	movq	%r12, 888(%rsp)
	movq	%r13, 896(%rsp)
	testq	%rax, %rax
	je	.LBB240_497
	movl	296(%rax), %ecx
	movb	$-1, %bl
	testl	%ecx, %ecx
	jne	.LBB240_491
	movq	288(%rax), %rcx
	movzbl	272(%rax), %ebx
	movq	%rcx, 191(%rsp)
	vmovdqu	273(%rax), %xmm0
	vmovdqa	%xmm0, 176(%rsp)
.LBB240_491:
	movl	72(%rsp), %eax
	testb	%al, %al
	je	.LBB240_496
	movzbl	%al, %eax
	cmpl	$2, %eax
	je	.LBB240_498
	cmpb	$-1, %bl
	je	.LBB240_685
	cmpb	$2, 472(%r15)
	jne	.LBB240_496
	vmovdqa	176(%rsp), %xmm0
	movq	191(%rsp), %rax
	movq	696(%r15), %rdi
	movb	%bl, 448(%rsp)
	vmovdqu	%xmm0, 449(%rsp)
	movq	%rax, 464(%rsp)
	movl	40(%rdi), %eax
	testl	%eax, %eax
	jne	.LBB240_697
.LBB240_496:
	xorl	%ebp, %ebp
	jmp	.LBB240_498
.LBB240_497:
	cmpb	$2, 72(%rsp)
	movb	$-1, %bl
	sete	%al
	andb	%bpl, %al
	movl	%eax, %ebp
.LBB240_498:
	cmpb	$-1, %bl
	sete	%al
	andb	%bpl, %al
	movl	%eax, 24(%rsp)
.LBB240_499:
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
	je	.LBB240_503
	cmpq	$3, %rcx
	movq	32(%rsp), %rcx
	movq	<purrdf_sparql_eval::witness::RelationWitness>::merge@GOTPCREL(%rip), %rbp
	leaq	456(%rsp), %r13
	leaq	880(%rsp), %r14
	cmovaeq	%rax, %r13
	leaq	640(%rcx), %rbx
	.p2align	4
.LBB240_501:
	movq	%r12, 480(%rsp)
	movq	16(%r13), %rax
	movq	%rax, 896(%rsp)
	vmovdqu	(%r13), %xmm0
	vmovdqa	%xmm0, 880(%rsp)
.Ltmp7464:
	movq	%rbx, %rdi
	movq	%r14, %rsi
	vzeroupper
	callq	*%rbp
.Ltmp7465:
	addq	$24, %r13
	incq	%r12
	cmpq	%r12, %r15
	jne	.LBB240_501
.LBB240_503:
.Ltmp7470:
	leaq	448(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7471:
	movq	112(%rsp), %r12
	movq	32(%rsp), %r13
	cmpb	$0, 24(%rsp)
	movq	1216(%rsp), %rdi
	je	.LBB240_516
	movq	432(%rsp), %rdx
	cmpq	%rdx, %rdi
	ja	.LBB240_695
	movq	616(%r13), %r8
	testq	%r8, %r8
	je	.LBB240_511
	cmpq	$0, 336(%r8)
	leaq	608(%rsp), %rsi
	leaq	520(%rsp), %rcx
	leaq	536(%rsp), %rax
	jne	.LBB240_512
	cmpq	$-1, 16(%r8)
	jne	.LBB240_512
	cmpq	$-1, 40(%r8)
	jne	.LBB240_512
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%xmm0, (%rsi)
	movq	$0, 16(%rsi)
	movq	$-1, 632(%rsp)
	movl	$67108864, 640(%rsp)
	jmp	.LBB240_513
.LBB240_511:
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
	jmp	.LBB240_514
.LBB240_512:
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%xmm0, (%rsi)
	movq	$0, 16(%rsi)
	movq	$-1, 632(%rsp)
	movl	$67174400, 640(%rsp)
.LBB240_513:
	movq	$0, 448(%rsp)
	movq	$8, 456(%rsp)
	vmovdqu	%xmm0, -144(%rsi)
	movq	$8, 480(%rsp)
	vmovdqu	%xmm0, -120(%rsi)
.LBB240_514:
	movq	$8, 504(%rsp)
	movq	$0, 512(%rsp)
	movq	424(%rsp), %rsi
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	vmovdqu	%xmm0, (%rcx)
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu64	%zmm0, (%rax)
	movb	$0, 64(%rax)
	cmpq	%rdx, %rdi
	jne	.LBB240_577
.LBB240_515:
.Ltmp7489:
	leaq	448(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7490:
.LBB240_516:
	movq	320(%rsp), %rax
	vmovdqu	304(%rsp), %xmm0
	movq	%rax, 864(%rsp)
	movq	160(%rsp), %rax
	vmovdqa	%xmm0, 848(%rsp)
	testq	%rax, %rax
	je	.LBB240_519
	lock		decq	(%rax)
	jne	.LBB240_519
	#MEMBARRIER
.Ltmp7491:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	callq	*%rax
.Ltmp7492:
.LBB240_519:
	movq	872(%rsp), %rbx
	movq	32(%rsp), %r14
	movb	$1, %al
	movl	%eax, 16(%rsp)
	movq	696(%r14), %rax
	movl	40(%rax), %ecx
	testl	%ecx, %ecx
	jne	.LBB240_113
.LBB240_520:
	movzbl	16(%rax), %ecx
	cmpb	$-1, %cl
	je	.LBB240_113
	movb	%cl, 880(%rsp)
	movq	120(%rsp), %rcx
	vmovdqu	17(%rax), %xmm0
	vmovdqu	%xmm0, 881(%rsp)
	movq	32(%rax), %rax
	movq	%rax, 896(%rsp)
	lock		incq	(%rcx)
	jle	.LBB240_696
	movq	120(%rsp), %rcx
.Ltmp7493:
	leaq	448(%rsp), %rdi
	leaq	880(%rsp), %rdx
	movq	%rbx, %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
.Ltmp7494:
	vmovdqu64	480(%rsp), %zmm1
	vmovdqu64	448(%rsp), %zmm0
	movq	856(%rsp), %rbx
	movq	864(%rsp), %r14
	vmovdqu64	%zmm1, 40(%r12)
	vmovdqu64	%zmm0, 8(%r12)
	movq	$0, (%r12)
	testq	%r14, %r14
	je	.LBB240_566
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	movabsq	$9223372036854775807, %r12
	xorl	%r15d, %r15d
	jmp	.LBB240_528
	.p2align	4
.LBB240_525:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB240_526:
	vzeroupper
	callq	*%rbp
.LBB240_527:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB240_566
.LBB240_528:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB240_527
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
	jge	.LBB240_531
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB240_531:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB240_526
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB240_531
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
.LBB240_534:
	cmpq	%rax, %rdx
	jge	.LBB240_525
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB240_534
	jmp	.LBB240_525
.LBB240_113:
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
	je	.LBB240_536
	leaq	880(%rsp), %rdi
	leaq	1408(%rsp), %rsi
	leaq	1616(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	520(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB240_115
.LBB240_537:
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
	jge	.LBB240_539
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB240_539:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB240_545
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB240_539
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
.LBB240_542:
	cmpq	%rax, %rsi
	jge	.LBB240_544
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB240_542
.LBB240_544:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB240_545:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	544(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB240_546
	jmp	.LBB240_548
.LBB240_536:
	vmovdqu	1408(%rsp), %xmm0
	movq	1424(%rsp), %rax
	movq	1432(%rsp), %rcx
	movq	%rax, 904(%rsp)
	movq	%rcx, 912(%rsp)
	vmovdqu	%xmm0, 888(%rsp)
	movq	$-1, 880(%rsp)
	movq	520(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB240_537
.LBB240_115:
	movq	544(%rsp), %rax
	testq	%rax, %rax
	je	.LBB240_548
.LBB240_546:
	lock		decq	(%rax)
	jne	.LBB240_548
	leaq	544(%rsp), %rdi
	#MEMBARRIER
.Ltmp7496:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp7497:
.LBB240_548:
	vmovdqu64	912(%rsp), %zmm1
	vmovdqu64	880(%rsp), %zmm0
	movl	$0, 172(%rsp)
	vmovdqu64	%zmm1, 40(%r12)
	vmovdqu64	%zmm0, 8(%r12)
	movq	$0, (%r12)
.Ltmp7499:
	leaq	2144(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp7500:
.Ltmp7501:
	leaq	1944(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7502:
	movq	440(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB240_552
	#MEMBARRIER
.Ltmp7504:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	440(%rsp), %rdi
	callq	*%rax
.Ltmp7505:
.LBB240_552:
	cmpb	$0, 16(%rsp)
	je	.LBB240_633
	movq	424(%rsp), %rbx
	movq	432(%rsp), %r14
	testq	%r14, %r14
	je	.LBB240_622
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	movabsq	$9223372036854775807, %r12
	xorl	%r15d, %r15d
	jmp	.LBB240_558
	.p2align	4
.LBB240_555:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB240_556:
	callq	*%r13
.LBB240_557:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB240_622
.LBB240_558:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB240_557
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
	jge	.LBB240_561
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB240_561:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB240_556
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB240_561
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
.LBB240_564:
	cmpq	%rax, %rdx
	jge	.LBB240_555
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB240_564
	jmp	.LBB240_555
.LBB240_566:
	movq	848(%rsp), %rax
	testq	%rax, %rax
	je	.LBB240_578
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
	jge	.LBB240_569
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB240_569:
	movq	112(%rsp), %r12
	.p2align	4
.LBB240_570:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB240_576
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB240_570
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
.LBB240_573:
	cmpq	%rax, %rsi
	jge	.LBB240_575
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB240_573
.LBB240_575:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB240_576:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
	jmp	.LBB240_579
.LBB240_577:
	leaq	888(%rsp), %rcx
	leaq	(%rdx,%rdx,4), %rax
	leaq	880(%rsp), %r14
	vpbroadcastq	%rcx, %ymm0
	vpaddq	.LCPI240_4(%rip), %ymm0, %ymm1
	vpaddq	.LCPI240_5(%rip), %ymm0, %ymm0
	leaq	(%rsi,%rax,8), %rbp
	leaq	(%rdi,%rdi,4), %rax
	leaq	(%rsi,%rax,8), %rbx
	movq	%rbp, 24(%rsp)
	vmovdqu	%ymm1, 752(%rsp)
	vmovdqu	%ymm0, 704(%rsp)
	jmp	.LBB240_637
.LBB240_578:
	movq	112(%rsp), %r12
.LBB240_579:
.Ltmp7517:
	leaq	2144(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp7518:
.Ltmp7522:
	leaq	1944(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7523:
	movq	120(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB240_583
	#MEMBARRIER
.Ltmp7527:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	120(%rsp), %rdi
	callq	*%rax
.Ltmp7528:
.LBB240_583:
	movq	1688(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB240_593
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
	jge	.LBB240_586
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB240_586:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB240_592
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB240_586
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
.LBB240_589:
	cmpq	%rax, %rsi
	jge	.LBB240_591
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB240_589
.LBB240_591:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB240_592:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB240_593:
	movq	1616(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB240_603
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
	jge	.LBB240_596
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB240_596:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB240_602
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB240_596
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
.LBB240_599:
	cmpq	%rax, %rsi
	jge	.LBB240_601
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB240_599
.LBB240_601:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB240_602:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB240_603:
	movq	1712(%rsp), %rax
	testq	%rax, %rax
	je	.LBB240_606
	lock		decq	(%rax)
	jne	.LBB240_606
	leaq	1712(%rsp), %rdi
	#MEMBARRIER
.Ltmp7532:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp7533:
.LBB240_606:
	movq	440(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB240_608
	#MEMBARRIER
.Ltmp7538:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	440(%rsp), %rdi
	callq	*%rax
.Ltmp7539:
.LBB240_608:
	cmpb	$0, 16(%rsp)
	je	.LBB240_633
	movq	424(%rsp), %rbx
	movq	432(%rsp), %r14
	testq	%r14, %r14
	je	.LBB240_622
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	movabsq	$9223372036854775807, %r12
	xorl	%r15d, %r15d
	jmp	.LBB240_614
	.p2align	4
.LBB240_611:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB240_612:
	callq	*%r13
.LBB240_613:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB240_622
.LBB240_614:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB240_613
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
	jge	.LBB240_617
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB240_617:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB240_612
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB240_617
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
.LBB240_620:
	cmpq	%rax, %rdx
	jge	.LBB240_611
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB240_620
	jmp	.LBB240_611
.LBB240_622:
	movq	416(%rsp), %rax
	movq	112(%rsp), %r12
	testq	%rax, %rax
	je	.LBB240_633
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
	jge	.LBB240_625
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB240_625:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB240_631
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB240_625
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
.LBB240_628:
	cmpq	%rax, %rsi
	jge	.LBB240_630
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB240_628
.LBB240_630:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB240_631:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
.LBB240_632:
	vzeroupper
	callq	*%rax
.LBB240_633:
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
.LBB240_634:
	.cfi_def_cfa_offset 2416
	movq	312(%rsp), %rax
	movq	%rax, 40(%rsp)
.LBB240_635:
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
.LBB240_636:
	addq	$40, %rbx
	cmpq	%rbp, %rbx
	je	.LBB240_515
.LBB240_637:
.Ltmp7472:
	leaq	448(%rsp), %rsi
	movq	%r14, %rdi
	movq	%r13, %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7473:
	cmpb	$-1, 880(%rsp)
	jne	.LBB240_515
	movq	(%rbx), %rcx
	leaq	8(%rbx), %r15
	movq	%r15, %rdx
	decq	%rcx
	cmpq	$5, %rcx
	jb	.LBB240_641
	movq	16(%rbx), %rcx
	movq	8(%rbx), %rdx
	decq	%rcx
.LBB240_641:
	movq	120(%rsp), %r8
	addq	$16, %r8
.Ltmp7474:
	leaq	2144(%rsp), %rsi
	movq	%r14, %rdi
	movq	%r13, %r9
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7475:
	movq	880(%rsp), %rax
	movl	888(%rsp), %edx
	movl	892(%rsp), %ecx
	cmpq	$-1, %rax
	jne	.LBB240_684
	cmpl	$2, %edx
	je	.LBB240_636
.Ltmp7476:
	movq	%r14, %rdi
	movq	%r13, %rsi
	callq	purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7477:
	movq	880(%rsp), %rax
	movzbl	888(%rsp), %edx
	cmpq	$-1, %rax
	jne	.LBB240_686
	testb	$1, %dl
	je	.LBB240_636
	movq	(%rbx), %rax
	movq	16(%rbx), %r12
	leaq	-1(%rax), %r14
	decq	%r12
	cmpq	$5, %r14
	cmovbq	%r14, %r12
	cmpq	$4, %r12
	jbe	.LBB240_663
	movabsq	$2305843009213693920, %rax
	leaq	(,%r12,8), %r13
	movabsq	$9223372036854775804, %rcx
	addq	$31, %rax
	cmpq	%rax, %r12
	seta	%al
	cmpq	%rcx, %r13
	seta	%cl
	orb	%al, %cl
	jne	.LBB240_682
	movl	$4, %esi
	movq	%r13, %rdi
	callq	__rustc::__rust_alloc
	movl	$4, %edi
	testq	%rax, %rax
	je	.LBB240_683
	movq	%rax, %rbp
	cmpq	$5, %r14
	jb	.LBB240_652
	movq	(%r15), %r15
.LBB240_652:
	cmpq	$8, %r12
	jb	.LBB240_655
	leaq	(%r15,%r13), %rax
	cmpq	%rax, %rbp
	jae	.LBB240_669
	movq	%rbp, %rax
	addq	%r13, %rax
	cmpq	%rax, %r15
	jae	.LBB240_669
.LBB240_655:
	movq	32(%rsp), %r13
	xorl	%eax, %eax
.LBB240_656:
	movq	%r12, %rdx
	andq	$7, %rdx
	movq	%rax, %rcx
	je	.LBB240_660
	movq	%rax, %rcx
.LBB240_658:
	movl	(%r15,%rcx,8), %esi
	movl	4(%r15,%rcx,8), %edi
	movl	%esi, (%rbp,%rcx,8)
	movl	%edi, 4(%rbp,%rcx,8)
	incq	%rcx
	decq	%rdx
	jne	.LBB240_658
	leaq	-1(%rcx), %r14
.LBB240_660:
	subq	%r12, %rax
	cmpq	$-8, %rax
	ja	.LBB240_679
	movq	%r12, %rax
	decq	%rcx
	negq	%rax
	movq	%rcx, %r14
.LBB240_662:
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
	jne	.LBB240_662
	jmp	.LBB240_679
.LBB240_663:
	cmpq	$6, %rax
	jb	.LBB240_665
	movq	(%r15), %r15
.LBB240_665:
	testq	%r12, %r12
	je	.LBB240_667
	leaq	-1(%r12), %rax
	vpmovsxbd	.LCPI240_8(%rip), %xmm2
	vmovdqu	752(%rsp), %ymm3
	vpxor	%xmm1, %xmm1, %xmm1
	incq	%r12
	vpbroadcastq	%rax, %ymm0
	vpcmpnltuq	.LCPI240_6(%rip), %ymm0, %k1
	vpxor	%xmm0, %xmm0, %xmm0
	kmovq	%k1, %k2
	vpgatherdd	(%r15,%xmm2), %xmm0 {%k2}
	kmovq	%k1, %k2
	vpgatherdd	4(%r15,%xmm2), %xmm1 {%k2}
	kmovq	%k1, %k2
	vpscatterqd	%xmm0, (,%ymm3) {%k2}
	vmovdqu	704(%rsp), %ymm0
	vpscatterqd	%xmm1, (,%ymm0) {%k1}
	jmp	.LBB240_668
.LBB240_667:
	movl	$1, %r12d
.LBB240_668:
	movq	%r12, 880(%rsp)
	leaq	888(%rsp), %rax
	vmovdqu	16(%rax), %xmm0
	movq	888(%rsp), %rbp
	movq	896(%rsp), %r14
	vmovdqa	%xmm0, 176(%rsp)
	jmp	.LBB240_680
.LBB240_669:
	cmpq	$32, %r12
	jae	.LBB240_671
	movq	32(%rsp), %r13
	xorl	%eax, %eax
	jmp	.LBB240_675
.LBB240_671:
	movq	32(%rsp), %r13
	movabsq	$2305843009213693920, %rcx
	movq	%r12, %rax
	andq	%rcx, %rax
	xorl	%ecx, %ecx
.LBB240_672:
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
	jne	.LBB240_672
	cmpq	%rax, %r12
	je	.LBB240_678
	testb	$24, %r12b
	je	.LBB240_656
.LBB240_675:
	movq	%rax, %rcx
	movabsq	$2305843009213693920, %rax
	addq	$24, %rax
	andq	%r12, %rax
.LBB240_676:
	vmovdqu64	(%r15,%rcx,8), %zmm0
	vmovdqu64	%zmm0, (%rbp,%rcx,8)
	addq	$8, %rcx
	cmpq	%rcx, %rax
	jne	.LBB240_676
	cmpq	%rax, %r12
	jne	.LBB240_656
.LBB240_678:
	decq	%rax
	movq	%rax, %r14
.LBB240_679:
	addq	$2, %r14
	incq	%r12
.LBB240_680:
	movq	16(%rsp), %rax
	cmpq	304(%rsp), %rax
	jne	.LBB240_635
.Ltmp7481:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	304(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp7482:
	jmp	.LBB240_634
.LBB240_682:
	xorl	%edi, %edi
.LBB240_683:
.Ltmp7484:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%r13, %rsi
	callq	*%rax
.Ltmp7485:
	jmp	.LBB240_696
.LBB240_684:
	vmovups	912(%rsp), %zmm1
	vmovups	896(%rsp), %zmm0
	movl	%edx, %esi
	shrl	$8, %esi
	vmovups	%zmm1, 192(%rsp)
	vmovups	%zmm0, 176(%rsp)
	jmp	.LBB240_687
.LBB240_685:
	movb	$-1, %bl
	xorl	%ebp, %ebp
	jmp	.LBB240_498
.LBB240_686:
	movzbl	891(%rsp), %ecx
	movzwl	889(%rsp), %esi
	vmovups	896(%rsp), %zmm0
	vmovups	912(%rsp), %zmm1
	shll	$16, %ecx
	orl	%ecx, %esi
	movl	892(%rsp), %ecx
	vmovups	%zmm0, 176(%rsp)
	vmovups	%zmm1, 192(%rsp)
.LBB240_687:
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
.Ltmp7479:
	leaq	448(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7480:
	leaq	304(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	160(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB240_237
	jmp	.LBB240_239
.LBB240_689:
	movq	24(%rsp), %r14
	movq	16(%rsp), %r15
.LBB240_690:
	movb	$1, %bpl
	movq	%r15, 392(%rsp)
	jmp	.LBB240_472
.LBB240_692:
	movq	24(%rsp), %r14
	jmp	.LBB240_690
.LBB240_693:
.Ltmp7418:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.290(%rip), %rcx
	vzeroupper
	callq	*%rax
.Ltmp7419:
	jmp	.LBB240_696
.LBB240_694:
	leaq	1256(%rsp), %rax
	leaq	304(%rsp), %rcx
	movq	%rdi, 1256(%rsp)
	movq	%rdx, 304(%rsp)
	movq	%rax, 176(%rsp)
	leaq	<usize as core::fmt::Debug>::fmt(%rip), %rax
	movq	%rax, 184(%rsp)
	movq	%rcx, 192(%rsp)
	movq	%rax, 200(%rsp)
.Ltmp7421:
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.2158(%rip), %rdi
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.289(%rip), %rdx
	leaq	176(%rsp), %rsi
	vzeroupper
	callq	*%rax
.Ltmp7422:
	jmp	.LBB240_696
.LBB240_695:
.Ltmp7507:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.405(%rip), %rcx
	movq	%rdx, %rsi
	callq	*%rax
.Ltmp7508:
.LBB240_696:
	ud2
.LBB240_697:
	addq	$16, %rdi
.Ltmp7456:
	leaq	448(%rsp), %rsi
	vzeroupper
	callq	<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::initialize::<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::get_or_init<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::try_insert::{closure#0}>::{closure#0}, !> (.llvm.4261325137610144415)
.Ltmp7457:
	jmp	.LBB240_496
.LBB240_698:
.Ltmp7458:
	leaq	880(%rsp), %rdi
	movq	%rax, %r13
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB240_788
.LBB240_699:
.Ltmp7404:
	movq	%rax, %r13
	movq	%r15, 392(%rsp)
	jmp	.LBB240_769
.LBB240_700:
.Ltmp7398:
	movq	%rax, %r13
	movq	%r12, 392(%rsp)
	jmp	.LBB240_769
.LBB240_701:
.Ltmp7440:
	leaq	816(%rsp), %rdi
	movq	%rax, %r13
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB240_788
.LBB240_702:
.Ltmp7428:
	movq	%rax, %r13
	movb	$1, %bl
	jmp	.LBB240_772
.LBB240_703:
.Ltmp7407:
	movq	%rax, %r13
	xorl	%ebx, %ebx
	jmp	.LBB240_772
.LBB240_704:
.Ltmp7483:
	movq	%rax, %r13
	cmpq	$6, %r12
	jb	.LBB240_792
	leaq	-8(,%r12,8), %rsi
	movl	$4, %edx
	movq	%rbp, %rdi
	callq	__rustc::__rust_dealloc
	jmp	.LBB240_792
.LBB240_706:
.Ltmp7367:
	jmp	.LBB240_768
.LBB240_707:
.Ltmp7373:
	jmp	.LBB240_746
.LBB240_708:
.Ltmp7382:
	jmp	.LBB240_768
.LBB240_709:
.Ltmp7435:
	movq	%rax, %r13
	jmp	.LBB240_782
.LBB240_710:
.Ltmp7455:
	jmp	.LBB240_736
.LBB240_711:
.Ltmp7401:
	jmp	.LBB240_768
.LBB240_712:
.Ltmp7443:
	movq	%rax, %r13
	movq	%r12, 456(%rsp)
	cmpq	$6, %rbp
	jb	.LBB240_714
	movq	16(%rsp), %rdi
	leaq	-8(,%rbp,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB240_714:
	leaq	448(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB240_718
.LBB240_798:
.Ltmp7495:
	leaq	848(%rsp), %rdi
	movq	%rax, %r13
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movl	16(%rsp), %ebp
	jmp	.LBB240_799
.LBB240_715:
.Ltmp7498:
	movl	16(%rsp), %ebp
	movq	%rax, %r13
	xorl	%ebx, %ebx
	jmp	.LBB240_800
.LBB240_716:
.Ltmp7385:
	jmp	.LBB240_746
.LBB240_717:
.Ltmp7362:
	leaq	1304(%rsp), %rcx
	movq	%rax, %r13
	movq	%rcx, 80(%rsp)
.LBB240_718:
	movq	80(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB240_787
.LBB240_719:
.Ltmp7356:
	movq	%rax, %r13
	jmp	.LBB240_739
.LBB240_720:
.Ltmp7345:
	movq	%rax, %r13
	jmp	.LBB240_740
.LBB240_721:
.Ltmp7461:
	movq	%rax, %r13
	jmp	.LBB240_788
.LBB240_722:
.Ltmp7534:
	jmp	.LBB240_732
.LBB240_723:
.Ltmp7506:
	jmp	.LBB240_726
.LBB240_724:
.Ltmp7332:
	movq	%rax, %r15
	movq	%rbx, 312(%rsp)
	cmpq	$5, %r13
	ja	.LBB240_763
	jmp	.LBB240_764
.LBB240_725:
.Ltmp7540:
.LBB240_726:
	cmpb	$0, 16(%rsp)
	movq	%rax, %r13
	jne	.LBB240_809
	jmp	.LBB240_810
.LBB240_727:
.Ltmp7529:
	movl	16(%rsp), %ebp
	movq	%rax, %r13
	jmp	.LBB240_805
.LBB240_728:
.Ltmp7395:
	jmp	.LBB240_768
.LBB240_729:
.Ltmp7414:
	jmp	.LBB240_746
.LBB240_730:
.Ltmp7342:
	leaq	1328(%rsp), %rdi
	movq	%rax, %r13
	jmp	.LBB240_794
.LBB240_731:
.Ltmp7503:
.LBB240_732:
	movl	16(%rsp), %ebp
	movq	%rax, %r13
	jmp	.LBB240_806
.LBB240_733:
.Ltmp7417:
	addq	$40, %rbx
	movq	%rax, %r13
	movq	%rbx, 824(%rsp)
	cmpq	$6, %r12
	jb	.LBB240_769
	movq	704(%rsp), %rdi
	leaq	-8(,%r12,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB240_769
.LBB240_735:
.Ltmp7339:
.LBB240_736:
	movq	%rax, %r13
	jmp	.LBB240_795
.LBB240_737:
.Ltmp7348:
	movq	%rax, %r13
	movq	%rbp, 888(%rsp)
.Ltmp7349:
	leaq	448(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7350:
.Ltmp7352:
	leaq	880(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp7353:
.LBB240_739:
.Ltmp7357:
	leaq	176(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp7358:
.LBB240_740:
	leaq	1304(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB240_788
.LBB240_741:
.Ltmp7351:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB240_742:
.Ltmp7359:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB240_743:
.Ltmp7514:
	movq	%rax, %r13
	movb	$1, %bpl
	jmp	.LBB240_799
.LBB240_744:
.Ltmp7478:
	jmp	.LBB240_791
.LBB240_745:
.Ltmp7390:
.LBB240_746:
	movq	16(%rsp), %rcx
	movq	%rax, %r13
	movq	%rcx, 392(%rsp)
	jmp	.LBB240_769
.LBB240_747:
.Ltmp7370:
	jmp	.LBB240_768
.LBB240_748:
.Ltmp7449:
	movq	%rax, %r13
	testq	%r15, %r15
	je	.LBB240_752
	negq	%r15
	addq	$160, %r14
.LBB240_750:
.Ltmp7450:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7451:
	addq	$160, %r14
	decq	%r15
	jne	.LBB240_750
.LBB240_752:
	cmpq	$0, 48(%rsp)
	je	.LBB240_788
	movq	48(%rsp), %rax
	movq	64(%rsp), %rdi
	movl	$8, %edx
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
	jmp	.LBB240_788
.LBB240_754:
.Ltmp7452:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB240_755:
.Ltmp7524:
	movl	16(%rsp), %ebp
	movq	%rax, %r13
	jmp	.LBB240_803
.LBB240_756:
.Ltmp7519:
	movl	172(%rsp), %ebx
	movl	16(%rsp), %ebp
	movq	%rax, %r13
	jmp	.LBB240_801
.LBB240_757:
.Ltmp7466:
	movq	%rax, %r13
.Ltmp7467:
	leaq	448(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7468:
	jmp	.LBB240_793
.LBB240_758:
.Ltmp7469:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB240_759:
.Ltmp7311:
	movq	%rax, %r13
.Ltmp7312:
	leaq	1720(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp7313:
	jmp	.LBB240_810
.LBB240_760:
.Ltmp7314:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB240_761:
.Ltmp7329:
	movq	%rax, %r15
	movq	%rbx, 312(%rsp)
	cmpq	$6, %r13
	jb	.LBB240_764
	movq	1416(%rsp), %r14
.LBB240_763:
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
	movq	%r14, %rdi
	callq	__rustc::__rust_dealloc
.LBB240_764:
	leaq	304(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	leaq	1120(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movb	$1, %bl
	xorl	%ebp, %ebp
	movq	%r15, %r13
	jmp	.LBB240_800
.LBB240_765:
.Ltmp7509:
	movq	%rax, %r13
	jmp	.LBB240_793
.LBB240_766:
.Ltmp7423:
	jmp	.LBB240_768
.LBB240_767:
.Ltmp7420:
.LBB240_768:
	movq	%rax, %r13
.LBB240_769:
	cmpq	$0, 56(%rsp)
	je	.LBB240_771
	movq	56(%rsp), %rsi
	movq	792(%rsp), %rdi
	movl	$8, %edx
	shlq	$5, %rsi
	callq	__rustc::__rust_dealloc
.LBB240_771:
	movb	$1, %bl
.Ltmp7424:
	leaq	384(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp7425:
.LBB240_772:
	cmpq	$0, 736(%rsp)
	je	.LBB240_774
	movq	736(%rsp), %rax
	movq	752(%rsp), %rdi
	movl	$8, %edx
	shlq	$3, %rax
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB240_774:
	movq	536(%rsp), %rax
	testq	%rax, %rax
	je	.LBB240_777
	lock		decq	(%rax)
	jne	.LBB240_777
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	536(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB240_777:
	movq	568(%rsp), %rax
	testq	%rax, %rax
	je	.LBB240_780
	lock		decq	(%rax)
	jne	.LBB240_780
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	568(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB240_780:
.Ltmp7429:
	leaq	1224(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp7430:
	xorl	%ebp, %ebp
.LBB240_782:
	movq	24(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB240_784
	#MEMBARRIER
.Ltmp7436:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	808(%rsp), %rdi
	callq	*%rax
.Ltmp7437:
.LBB240_784:
	leaq	816(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	testb	%bl, %bl
	je	.LBB240_786
	leaq	264(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB240_786:
	testb	%bpl, %bpl
	je	.LBB240_788
.LBB240_787:
.Ltmp7444:
	leaq	1376(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp7445:
.LBB240_788:
.Ltmp7462:
	leaq	1264(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7463:
	jmp	.LBB240_795
.LBB240_789:
.Ltmp7446:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB240_790:
.Ltmp7486:
.LBB240_791:
	movq	%rax, %r13
.LBB240_792:
.Ltmp7487:
	leaq	448(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7488:
.LBB240_793:
	leaq	304(%rsp), %rdi
.LBB240_794:
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB240_795:
	movq	160(%rsp), %rax
	movb	$1, %bpl
	testq	%rax, %rax
	je	.LBB240_799
	lock		decq	(%rax)
	movb	$1, %bpl
	jne	.LBB240_799
	movb	$1, %bpl
	#MEMBARRIER
.Ltmp7510:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	callq	*%rax
.Ltmp7511:
	movb	$1, %bl
	jmp	.LBB240_800
.LBB240_799:
	movb	$1, %bl
.LBB240_800:
.Ltmp7515:
	leaq	2144(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp7516:
.LBB240_801:
.Ltmp7520:
	leaq	1944(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7521:
	testb	%bl, %bl
	je	.LBB240_806
.LBB240_803:
	movq	120(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB240_805
	#MEMBARRIER
.Ltmp7525:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	120(%rsp), %rdi
	callq	*%rax
.Ltmp7526:
.LBB240_805:
.Ltmp7530:
	leaq	1616(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp7531:
.LBB240_806:
	movq	440(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB240_808
	#MEMBARRIER
.Ltmp7535:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	440(%rsp), %rdi
	callq	*%rax
.Ltmp7536:
.LBB240_808:
	testb	%bpl, %bpl
	je	.LBB240_810
.LBB240_809:
	leaq	416(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB240_810:
	movq	%r13, %rdi
	callq	_Unwind_Resume@PLT
.LBB240_811:
.Ltmp7537:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end240:
