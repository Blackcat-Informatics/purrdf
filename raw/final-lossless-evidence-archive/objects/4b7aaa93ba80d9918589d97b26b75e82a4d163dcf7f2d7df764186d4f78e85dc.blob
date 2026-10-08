purrdf_sparql_eval::modifier::eval_group_with::<purrdf_core::ir::dataset::RdfDataset, ()>:
.Lfunc_begin361:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception267
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
	subq	$2456, %rsp
	.cfi_def_cfa_offset 2512
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	vmovdqu	anon.e5162873a9a3251d11c4df37a70e4654.29.llvm.4261325137610144415(%rip), %ymm0
	movq	$0, 384(%rsp)
	movq	$8, 392(%rsp)
	movq	$0, 400(%rsp)
	movq	%r9, 336(%rsp)
	movq	%rdx, %r14
	movq	%rsi, 1160(%rsp)
	movq	%rdi, 72(%rsp)
	movq	%r8, 136(%rsp)
	vmovdqu	%ymm0, 408(%rsp)
	testq	%r8, %r8
	je	.LBB361_5
	movq	136(%rsp), %rbx
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %r13
	movq	%rcx, %r15
	leaq	384(%rsp), %r12
	shlq	$4, %rbx
	addq	%rcx, %rbx
	.p2align	4
.LBB361_2:
	movq	(%r15), %rsi
	movq	8(%r15), %rdx
	lock		incq	(%rsi)
	jle	.LBB361_950
.Ltmp15780:
	movq	%r12, %rdi
	vzeroupper
	callq	*%r13
.Ltmp15781:
	addq	$16, %r15
	cmpq	%rbx, %r15
	jne	.LBB361_2
.LBB361_5:
	vmovdqu	408(%rsp), %ymm0
	movq	384(%rsp), %rax
	movq	392(%rsp), %rcx
	movq	400(%rsp), %rdx
	movq	408(%rsp), %rsi
	movq	2512(%rsp), %rdi
	movq	%rcx, 1608(%rsp)
	movq	%rax, 1600(%rsp)
	movq	%rdx, 1616(%rsp)
	imulq	$120, %rdi, %rcx
	addq	336(%rsp), %rcx
	vmovdqu	%ymm0, 1624(%rsp)
	movq	%rsi, 1624(%rsp)
	movq	1616(%rsp), %rax
	movq	%rcx, 64(%rsp)
	movq	%rax, 728(%rsp)
	testq	%rdi, %rdi
	je	.LBB361_11
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rbx
	movq	336(%rsp), %r12
	leaq	1600(%rsp), %r15
	.p2align	4
.LBB361_7:
	movq	(%r12), %rsi
	movq	1616(%rsp), %r13
	lock		incq	(%rsi)
	jle	.LBB361_950
	movq	8(%r12), %rdx
.Ltmp15786:
	movq	%r15, %rdi
	vzeroupper
	callq	*%rbx
.Ltmp15787:
	cmpq	%r13, %rax
	jne	.LBB361_64
	addq	$120, %r12
	cmpq	64(%rsp), %r12
	jne	.LBB361_7
.LBB361_11:
	movq	1600(%rsp), %rax
	vmovdqu	1632(%rsp), %xmm0
	movq	1624(%rsp), %rdi
	movq	1608(%rsp), %rcx
	movq	1616(%rsp), %rdx
	movq	1648(%rsp), %r8
	movq	1624(%rsp), %rsi
	movq	%rax, 400(%rsp)
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%rdi, 424(%rsp)
	movl	$72, %edi
	movq	%r8, 448(%rsp)
	movq	%rcx, 408(%rsp)
	movq	%rdx, 416(%rsp)
	movq	%rsi, 424(%rsp)
	vmovdqu	%xmm0, 432(%rsp)
	movq	$1, 384(%rsp)
	movq	$1, 392(%rsp)
	vzeroupper
	callq	*%rax
	movq	%rax, 328(%rsp)
	testq	%rax, %rax
	je	.LBB361_944
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB361_14
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB361_14:
	movq	72(%rsp), %rbx
	leaq	392(%rsp), %rax
	movq	%rax, 176(%rsp)
	.p2align	4
.LBB361_15:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_21
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_15
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdx
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	$72, (%rcx)
	movl	$72, %ecx
	lock		xaddq	%rcx, (%rdx)
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	addq	$72, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB361_18:
	cmpq	%rax, %rcx
	jle	.LBB361_20
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB361_18
.LBB361_20:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_21:
	vmovdqu64	384(%rsp), %zmm0
	movq	328(%rsp), %rcx
	movq	448(%rsp), %rax
	movb	$1, %r15b
	movq	%rax, 64(%rcx)
	movq	%rcx, 1520(%rsp)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp15794:
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	1160(%rsp), %rsi
	leaq	1744(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15795:
	movb	$1, %r15b
.Ltmp15796:
	movq	2520(%rsp), %rdx
	leaq	2336(%rsp), %rdi
	movq	%r14, %rsi
	movq	%r14, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp15797:
	cmpl	$1, 2336(%rsp)
	jne	.LBB361_47
	vmovdqu64	2384(%rsp), %zmm1
	vmovdqu64	2352(%rsp), %zmm0
	movq	1816(%rsp), %rax
	vmovdqu64	%zmm1, 48(%rbx)
	vmovdqu64	%zmm0, 16(%rbx)
	movq	$1, (%rbx)
	cmpq	$6, %rax
	jb	.LBB361_34
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	1824(%rsp), %rdi
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
	jge	.LBB361_27
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_27:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_33
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_27
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
.LBB361_30:
	cmpq	%rax, %rdx
	jge	.LBB361_32
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB361_30
.LBB361_32:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_33:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB361_34:
	movq	1744(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB361_44
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	1752(%rsp), %rdi
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
	jge	.LBB361_37
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_37:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_43
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_37
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
.LBB361_40:
	cmpq	%rax, %rdx
	jge	.LBB361_42
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB361_40
.LBB361_42:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_43:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB361_44:
	movq	1840(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_556
	lock		decq	(%rax)
	jne	.LBB361_556
	movb	$1, %r15b
	leaq	1840(%rsp), %rdi
	#MEMBARRIER
.Ltmp16194:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp16195:
	jmp	.LBB361_556
.LBB361_47:
	vmovdqu64	2376(%rsp), %zmm1
	vmovdqu64	2344(%rsp), %zmm0
	vmovdqu64	%zmm1, 416(%rsp)
	vmovdqu64	%zmm0, 384(%rsp)
.Ltmp15798:
	leaq	800(%rsp), %rdi
	leaq	1744(%rsp), %rsi
	leaq	384(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp15799:
	cmpq	$-1, 800(%rsp)
	je	.LBB361_74
	vmovdqu	800(%rsp), %ymm0
	vmovdqu	%ymm0, 1120(%rsp)
	movq	1144(%rsp), %rax
	lock		incq	(%rax)
	jle	.LBB361_950
	movq	328(%rsp), %rcx
	movq	1144(%rsp), %rax
	movq	728(%rsp), %rsi
	movq	32(%rcx), %rdx
	movq	%rax, 184(%rsp)
	cmpq	%rdx, %rsi
	ja	.LBB361_872
	movq	%rsi, %r14
	shlq	$4, %r14
	movq	%rsi, 720(%rsp)
	movq	%r14, 712(%rsp)
	testq	%rsi, %rsi
	je	.LBB361_76
	movq	328(%rsp), %rax
	movq	%r14, %rdi
	movq	24(%rax), %rbx
	movq	malloc@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	%rax, 192(%rsp)
	testq	%rax, %rax
	je	.LBB361_948
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	movabsq	$9223372036854775807, %rdx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	%r14, %rax
	cmovbq	%rcx, %rax
	cmpq	%rdx, %r14
	movq	%rdx, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmovbq	%r14, %rcx
	addq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB361_55
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_55:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_61
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_55
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rax
	movq	712(%rsp), %rdx
	lock		addq	%rdx, (%rax)
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
.LBB361_58:
	cmpq	%rax, %rdx
	jle	.LBB361_60
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB361_58
.LBB361_60:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_61:
	movq	720(%rsp), %r15
	xorl	%r14d, %r14d
	.p2align	4
.LBB361_62:
	movq	184(%rsp), %rdi
	leaq	(%rbx,%r14), %rsi
	addq	$16, %rdi
.Ltmp15800:
	callq	<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.4261325137610144415)
.Ltmp15801:
	movq	192(%rsp), %rcx
	movq	%rax, (%rcx,%r14)
	movq	%rdx, 8(%rcx,%r14)
	addq	$16, %r14
	decq	%r15
	jne	.LBB361_62
	jmp	.LBB361_77
.LBB361_64:
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$51, %edi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB361_945
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rdx
	movq	72(%rsp), %rbx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$51, %rax
	cmovbq	%rdx, %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$51, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB361_67
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_67:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_73
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_67
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rsi
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdi
	lock		incq	(%rax)
	lock		addq	$51, (%rsi)
	movl	$51, %esi
	lock		xaddq	%rsi, (%rdi)
	addq	$51, %rsi
	cmovoq	%rdx, %rsi
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	movq	(%rdx), %rax
	.p2align	4
.LBB361_70:
	cmpq	%rax, %rsi
	jle	.LBB361_72
	lock		cmpxchgq	%rsi, (%rdx)
	jne	.LBB361_70
.LBB361_72:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_73:
	vmovups	.Lanon.e5162873a9a3251d11c4df37a70e4654.528+19(%rip), %ymm0
	vmovups	.Lanon.e5162873a9a3251d11c4df37a70e4654.528(%rip), %ymm1
	movabsq	$9223372036854775793, %rax
	leaq	1600(%rsp), %rdi
	addq	$35, %rax
	movq	%rax, 16(%rbx)
	movq	$51, 24(%rbx)
	movq	%rcx, 32(%rbx)
	movq	$51, 40(%rbx)
	movq	$1, (%rbx)
	vmovups	%ymm0, 19(%rcx)
	vmovups	%ymm1, (%rcx)
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.4261325137610144415)
	jmp	.LBB361_558
.LBB361_74:
	vmovups	1784(%rsp), %zmm1
	vmovups	1744(%rsp), %zmm0
	movq	328(%rsp), %rax
	movq	%rax, 1208(%rsp)
	movq	$0, 1184(%rsp)
	movq	$8, 1192(%rsp)
	movq	$0, 1200(%rsp)
	vmovups	%zmm1, 424(%rsp)
	vmovups	%zmm0, 384(%rsp)
	cmpq	$-1, 384(%rsp)
	je	.LBB361_154
	leaq	800(%rsp), %rdi
	leaq	1184(%rsp), %rsi
	leaq	1744(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	456(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB361_155
	jmp	.LBB361_164
.LBB361_76:
	movl	$8, %eax
	movq	%rax, 192(%rsp)
.LBB361_77:
	vmovdqu	anon.e5162873a9a3251d11c4df37a70e4654.29.llvm.4261325137610144415(%rip), %ymm0
	movq	1136(%rsp), %rax
	leaq	384(%rsp), %r11
	vmovdqu	%ymm0, 1712(%rsp)
	testq	%rax, %rax
	je	.LBB361_126
	movq	1128(%rsp), %rbx
	movq	192(%rsp), %rdx
	movq	712(%rsp), %r15
	leaq	(%rax,%rax,4), %rax
	movl	$2, %ecx
	leaq	392(%rsp), %r13
	movq	$0, 48(%rsp)
	vmovd	%ecx, %xmm0
	vmovdqa	%xmm0, 96(%rsp)
	leaq	(%rbx,%rax,8), %rax
	movq	%rax, 168(%rsp)
	leaq	(%rdx,%r15), %rax
	negq	%r15
	movq	%rax, 144(%rsp)
	jmp	.LBB361_81
	.p2align	4
.LBB361_79:
	movq	-16(%r12), %rax
	movq	48(%rsp), %rcx
	leaq	384(%rsp), %r11
	movq	%rcx, (%rax,%r14,8)
	incq	%r14
	movq	%rcx, %rax
	movq	%r14, -8(%r12)
.LBB361_80:
	incq	%rax
	addq	$40, %rbx
	movq	%rax, 48(%rsp)
	cmpq	168(%rsp), %rbx
	je	.LBB361_125
.LBB361_81:
	cmpq	$5, 720(%rsp)
	movl	$1, %ebp
	movl	$4, %eax
	movq	$1, 384(%rsp)
	movq	%r13, %rcx
	movq	%r11, %r14
	jae	.LBB361_92
	leaq	-1(%rbp), %rdx
	cmpq	%rax, %rdx
	jae	.LBB361_94
.LBB361_83:
	movq	192(%rsp), %r9
	leaq	8(%rbx), %rdx
	incq	%rax
	xorl	%r8d, %r8d
	jmp	.LBB361_87
	.p2align	4
.LBB361_84:
	movq	8(%r9), %rdi
	cmpq	%rsi, %rdi
	jae	.LBB361_896
	vmovq	(%r10,%rdi,8), %xmm0
.LBB361_86:
	vmovq	%xmm0, -8(%rcx,%rbp,8)
	addq	$16, %r9
	incq	%rbp
	addq	$-16, %r8
	cmpq	%rbp, %rax
	je	.LBB361_91
.LBB361_87:
	cmpq	%r8, %r15
	je	.LBB361_105
	vmovdqa	96(%rsp), %xmm0
	cmpl	$1, (%r9)
	jne	.LBB361_86
	movq	(%rbx), %rsi
	movq	%rdx, %r10
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB361_84
	movq	16(%rbx), %rsi
	movq	8(%rbx), %r10
	decq	%rsi
	jmp	.LBB361_84
	.p2align	4
.LBB361_105:
	movq	%rbp, (%r14)
	jmp	.LBB361_106
	.p2align	4
.LBB361_91:
	movq	192(%rsp), %r12
	subq	%r8, %r12
	movq	%rax, (%r14)
	cmpq	144(%rsp), %r12
	jne	.LBB361_95
	jmp	.LBB361_106
.LBB361_92:
.Ltmp15805:
	movq	720(%rsp), %rdx
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	movq	%r11, %rdi
	xorl	%esi, %esi
	movq	%r11, %r12
	vzeroupper
	callq	*%rax
.Ltmp15806:
	movq	384(%rsp), %rax
	xorl	%edx, %edx
	movl	$4, %ecx
	leaq	400(%rsp), %rsi
	movq	%r12, %r14
	movq	%r12, %r11
	decq	%rax
	cmpq	$5, %rax
	cmovbq	%rcx, %rax
	movq	392(%rsp), %rcx
	setae	%dl
	cmovaeq	%rsi, %r14
	cmovbq	%r13, %rcx
	shll	$4, %edx
	movq	384(%rsp,%rdx), %rbp
	leaq	-1(%rbp), %rdx
	cmpq	%rax, %rdx
	jb	.LBB361_83
	.p2align	4
.LBB361_94:
	movq	192(%rsp), %r12
	movq	%rbp, %rax
	movq	%rax, (%r14)
	cmpq	144(%rsp), %r12
	je	.LBB361_106
.LBB361_95:
	leaq	8(%rbx), %r14
	.p2align	4
.LBB361_96:
	vmovdqa	96(%rsp), %xmm0
	cmpl	$1, (%r12)
	vmovdqa	%xmm0, 32(%rsp)
	jne	.LBB361_101
	movq	(%rbx), %rsi
	movq	%r14, %rax
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB361_99
	movq	16(%rbx), %rsi
	movq	8(%rbx), %rax
	decq	%rsi
.LBB361_99:
	movq	8(%r12), %rdi
	cmpq	%rsi, %rdi
	jae	.LBB361_895
	vmovq	(%rax,%rdi,8), %xmm0
	vmovdqa	%xmm0, 32(%rsp)
.LBB361_101:
	movq	384(%rsp), %rsi
	movq	392(%rsp), %rax
	xorl	%edx, %edx
	leaq	400(%rsp), %rdi
	movq	%r11, %rcx
	decq	%rsi
	cmpq	$5, %rsi
	cmovaeq	%rdi, %rcx
	movl	$4, %edi
	setae	%dl
	cmovbq	%r13, %rax
	cmovbq	%rdi, %rsi
	shll	$4, %edx
	movq	384(%rsp,%rdx), %rbp
	leaq	-1(%rbp), %rdx
	cmpq	%rsi, %rdx
	je	.LBB361_103
.LBB361_102:
	vmovdqa	32(%rsp), %xmm0
	addq	$16, %r12
	vmovq	%xmm0, -8(%rax,%rbp,8)
	incq	%rbp
	movq	%rbp, (%rcx)
	cmpq	144(%rsp), %r12
	jne	.LBB361_96
	jmp	.LBB361_106
.LBB361_103:
.Ltmp15814:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	movl	$1, %edx
	movl	$1, %ecx
	movq	%r11, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15815:
	cmpq	$6, 384(%rsp)
	movq	392(%rsp), %rax
	leaq	384(%rsp), %r11
	leaq	400(%rsp), %rdx
	movq	%r11, %rcx
	cmovbq	%r13, %rax
	cmovaeq	%rdx, %rcx
	jmp	.LBB361_102
	.p2align	4
.LBB361_106:
	vmovdqu	384(%rsp), %ymm0
	movq	416(%rsp), %rax
	movq	1736(%rsp), %r14
	movq	%rax, 832(%rsp)
	vmovdqu	%ymm0, 800(%rsp)
.Ltmp15817:
	leaq	1712(%rsp), %rsi
	leaq	800(%rsp), %rdx
	movq	%r11, %rdi
	vzeroupper
	callq	<hashbrown::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>), purrdf_hash::fixed::FixedState>>::rustc_entry
.Ltmp15818:
	movq	384(%rsp), %rax
	movq	392(%rsp), %r12
	testq	%rax, %rax
	je	.LBB361_120
	vmovdqu	16(%r13), %xmm0
	movq	%rax, 32(%rsp)
	movq	400(%rsp), %rcx
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%r12, 88(%rsp)
	movq	424(%rsp), %rbp
	movq	432(%rsp), %r12
	movl	$8, %edi
	movq	%rcx, 56(%rsp)
	vmovdqa	%xmm0, 2144(%rsp)
	callq	*%rax
	testq	%rax, %rax
	je	.LBB361_939
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rdx
	leaq	384(%rsp), %r11
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$8, %rax
	cmovbq	%rdx, %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$8, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB361_111
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_111:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_117
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_111
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdi
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	$8, (%rdx)
	movl	$8, %edx
	lock		xaddq	%rdx, (%rdi)
	addq	$8, %rdx
	cmovoq	%rax, %rdx
	movq	(%rsi), %rax
	.p2align	4
.LBB361_114:
	cmpq	%rax, %rdx
	jle	.LBB361_116
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rsi
	lock		cmpxchgq	%rdx, (%rsi)
	jne	.LBB361_114
.LBB361_116:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_117:
	movq	48(%rsp), %rax
	movq	%rax, (%rcx)
	movq	8(%rbp), %rdx
	movq	(%rbp), %rax
	movq	%rdx, %rsi
	andq	%r12, %rsi
	vmovdqu	(%rax,%rsi), %xmm0
	vpmovmskb	%xmm0, %edi
	testl	%edi, %edi
	je	.LBB361_122
.LBB361_118:
	tzcntl	%edi, %edi
	addq	%rsi, %rdi
	andq	%rdx, %rdi
	movzbl	(%rax,%rdi), %esi
	testb	%sil, %sil
	jns	.LBB361_124
.LBB361_119:
	shrq	$57, %r12
	leaq	-16(%rdi), %r8
	andb	$1, %sil
	movb	%r12b, (%rax,%rdi)
	negq	%rdi
	andq	%rdx, %r8
	movzbl	%sil, %esi
	leaq	(%rdi,%rdi,8), %rdx
	movq	32(%rsp), %rdi
	movb	%r12b, 16(%rax,%r8)
	subq	%rsi, 16(%rbp)
	movq	88(%rsp), %r8
	movq	%rdi, -72(%rax,%rdx,8)
	movq	56(%rsp), %rdi
	movq	%r8, -64(%rax,%rdx,8)
	movq	%rdi, -56(%rax,%rdx,8)
	vmovdqa	2144(%rsp), %xmm0
	vmovdqu	%xmm0, -48(%rax,%rdx,8)
	movq	%r14, -32(%rax,%rdx,8)
	movq	$1, -24(%rax,%rdx,8)
	movq	%rcx, -16(%rax,%rdx,8)
	movq	$1, -8(%rax,%rdx,8)
	incq	24(%rbp)
	movq	48(%rsp), %rax
	jmp	.LBB361_80
	.p2align	4
.LBB361_120:
	movq	-8(%r12), %r14
	cmpq	-24(%r12), %r14
	jne	.LBB361_79
.Ltmp15822:
	movq	<alloc::raw_vec::RawVec<usize>>::grow_one@GOTPCREL(%rip), %rax
	leaq	-24(%r12), %rdi
	callq	*%rax
.Ltmp15823:
	jmp	.LBB361_79
.LBB361_122:
	movl	$16, %r8d
.LBB361_123:
	addq	%r8, %rsi
	addq	$16, %r8
	andq	%rdx, %rsi
	vmovdqu	(%rax,%rsi), %xmm0
	vpmovmskb	%xmm0, %edi
	testl	%edi, %edi
	jne	.LBB361_118
	jmp	.LBB361_123
.LBB361_124:
	vmovdqa	(%rax), %xmm0
	vpmovmskb	%xmm0, %esi
	xorl	%edi, %edi
	tzcntl	%esi, %edi
	movzbl	(%rax,%rdi), %esi
	jmp	.LBB361_119
.LBB361_125:
	movq	1736(%rsp), %rax
	orq	%rax, 136(%rsp)
	cmpq	$0, 2512(%rsp)
	jne	.LBB361_127
	jmp	.LBB361_140
.LBB361_126:
	xorl	%eax, %eax
	cmpq	$0, 2512(%rsp)
	je	.LBB361_140
.LBB361_127:
	cmpq	$0, 136(%rsp)
	jne	.LBB361_140
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqa	%xmm0, 800(%rsp)
	movq	$8, 816(%rsp)
	movq	$1, 384(%rsp)
	movq	$0, 824(%rsp)
.Ltmp15825:
	leaq	1184(%rsp), %rdi
	leaq	1712(%rsp), %rsi
	leaq	384(%rsp), %rdx
	leaq	800(%rsp), %rcx
	vzeroupper
	callq	<hashbrown::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>), purrdf_hash::fixed::FixedState>>::insert
.Ltmp15826:
	movq	1192(%rsp), %rcx
	testq	%rcx, %rcx
	jle	.LBB361_139
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$3, %rcx
	movabsq	$9223372036854775807, %rsi
	movq	1200(%rsp), %rdi
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
	jge	.LBB361_132
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_132:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_138
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_132
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
.LBB361_135:
	cmpq	%rax, %rdx
	jge	.LBB361_137
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB361_135
.LBB361_137:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_138:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_139:
	movq	1736(%rsp), %rax
.LBB361_140:
	movq	1712(%rsp), %rcx
	movq	1720(%rsp), %rsi
	vmovdqa	(%rcx), %xmm0
	testq	%rsi, %rsi
	je	.LBB361_142
	leaq	(,%rsi,8), %rdx
	movq	%rcx, %r8
	movl	$16, %r9d
	leaq	(%rdx,%rdx,8), %rdx
	andq	$-16, %rdx
	subq	%rdx, %r8
	leaq	97(%rdx,%rsi), %rdi
	addq	$-80, %r8
	jmp	.LBB361_143
.LBB361_142:
	xorl	%r9d, %r9d
.LBB361_143:
	vpcmpeqd	%xmm1, %xmm1, %xmm1
	leaq	1(%rcx,%rsi), %rsi
	leaq	16(%rcx), %rdx
	movq	%r9, 1184(%rsp)
	movq	%rdi, 1192(%rsp)
	movq	%r8, 1200(%rsp)
	movq	%rcx, 1208(%rsp)
	vpcmpgtb	%xmm1, %xmm0, %k0
	movq	%rdx, 1216(%rsp)
	movq	%rsi, 1224(%rsp)
	kmovw	%k0, 1232(%rsp)
	movq	%rax, 1240(%rsp)
	testq	%rax, %rax
	je	.LBB361_153
	kortestw	%k0, %k0
	je	.LBB361_146
	kmovd	%k0, %esi
	jmp	.LBB361_149
.LBB361_146:
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	.p2align	4
.LBB361_147:
	vpcmpltb	(%rdx), %xmm0, %k0
	addq	$-1152, %rcx
	addq	$16, %rdx
	kortestw	%k0, %k0
	je	.LBB361_147
	kmovd	%k0, %esi
	movq	%rdx, 1216(%rsp)
	movq	%rcx, 1208(%rsp)
.LBB361_149:
	xorl	%edx, %edx
	blsrl	%esi, %edx
	tzcntl	%esi, %esi
	leaq	-1(%rax), %rdi
	negq	%rsi
	movw	%dx, 1232(%rsp)
	movq	%rdi, 1240(%rsp)
	leaq	(%rsi,%rsi,8), %rdx
	movq	-24(%rcx,%rdx,8), %r14
	cmpq	$-1, %r14
	je	.LBB361_153
	leaq	(%rcx,%rdx,8), %rcx
	cmpq	$5, %rax
	movl	$4, %ebp
	movabsq	$128102389400760776, %rdx
	cmovaeq	%rax, %rbp
	decq	%rdx
	movq	-40(%rcx), %r8
	movq	-64(%rcx), %rsi
	movq	-32(%rcx), %rdi
	movq	-72(%rcx), %r12
	movq	%r8, 1984(%rsp)
	movq	%rsi, 96(%rsp)
	vmovdqu	-56(%rcx), %xmm0
	vmovdqa	%xmm0, 1968(%rsp)
	movq	-16(%rcx), %rsi
	movq	%rsi, 32(%rsp)
	leaq	(,%rbp,8), %rsi
	leaq	(%rsi,%rsi,8), %rbx
	cmpq	%rdx, %rax
	jbe	.LBB361_168
	xorl	%r13d, %r13d
.LBB361_152:
.Ltmp15833:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
.Ltmp15834:
	jmp	.LBB361_950
.LBB361_153:
	leaq	1184(%rsp), %rdi
	movq	$0, 1040(%rsp)
	movq	$8, 1048(%rsp)
	movq	$0, 1056(%rsp)
	vzeroupper
	callq	core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>
	movl	$8, %eax
	xorl	%r15d, %r15d
	movq	%rax, 32(%rsp)
	jmp	.LBB361_184
.LBB361_154:
	movq	1192(%rsp), %rcx
	movq	1184(%rsp), %rax
	movq	1200(%rsp), %rdx
	movq	%rcx, 816(%rsp)
	movq	1208(%rsp), %rcx
	movq	%rax, 808(%rsp)
	movq	%rdx, 824(%rsp)
	movq	%rcx, 832(%rsp)
	movq	$-1, 800(%rsp)
	movq	456(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB361_164
.LBB361_155:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	464(%rsp), %rdi
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
	jge	.LBB361_157
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_157:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_163
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_157
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
.LBB361_160:
	cmpq	%rax, %rdx
	jge	.LBB361_162
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB361_160
.LBB361_162:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_163:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB361_164:
	movq	480(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_167
	lock		decq	(%rax)
	jne	.LBB361_167
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	480(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB361_167:
	vmovdqu64	832(%rsp), %zmm1
	vmovdqu64	800(%rsp), %zmm0
	vmovdqu64	%zmm1, 40(%rbx)
	vmovdqu64	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
	jmp	.LBB361_558
.LBB361_168:
	movq	-8(%rcx), %r15
	testq	%rbx, %rbx
	je	.LBB361_170
	movl	$8, %esi
	movq	%rdi, 144(%rsp)
	movq	%rbx, %rdi
	movl	$8, %r13d
	vzeroupper
	callq	__rustc::__rust_alloc
	movq	144(%rsp), %rdi
	testq	%rax, %rax
	jne	.LBB361_171
	jmp	.LBB361_152
.LBB361_170:
	movl	$8, %eax
	xorl	%ebp, %ebp
.LBB361_171:
	movq	96(%rsp), %rcx
	movq	%r12, (%rax)
	movq	32(%rsp), %rdx
	movq	%rcx, 8(%rax)
	vmovaps	1968(%rsp), %xmm0
	vmovups	%xmm0, 16(%rax)
	movq	1984(%rsp), %rcx
	movq	%rcx, 32(%rax)
	movq	%rdi, 40(%rax)
	movq	%r14, 48(%rax)
	movq	%rdx, 56(%rax)
	movq	%r15, 64(%rax)
	movq	%rbp, 224(%rsp)
	movq	%rax, 232(%rsp)
	movq	$1, 240(%rsp)
	vmovdqu64	1184(%rsp), %zmm0
	vmovdqu64	%zmm0, 800(%rsp)
	movq	856(%rsp), %rdx
	testq	%rdx, %rdx
	je	.LBB361_183
	movzwl	848(%rsp), %ebp
	movq	824(%rsp), %r15
	movq	832(%rsp), %r12
	leaq	440(%rsp), %r13
	movl	$1, %ebx
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	jmp	.LBB361_174
	.p2align	4
.LBB361_173:
	movq	448(%rsp), %rdx
	leaq	(%rbx,%rbx,8), %rcx
	incq	%rbx
	movq	%rdx, 64(%rax,%rcx,8)
	movq	%r14, %rdx
	vmovdqu64	384(%rsp), %zmm0
	vmovdqu64	%zmm0, (%rax,%rcx,8)
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	movq	%rbx, 240(%rsp)
	testq	%r14, %r14
	je	.LBB361_181
.LBB361_174:
	testw	%bp, %bp
	jne	.LBB361_177
	.p2align	4
.LBB361_175:
	vpcmpltb	(%r12), %xmm0, %k0
	addq	$-1152, %r15
	addq	$16, %r12
	kortestw	%k0, %k0
	je	.LBB361_175
	kmovd	%k0, %ebp
.LBB361_177:
	xorl	%ecx, %ecx
	tzcntl	%ebp, %ecx
	leaq	-1(%rdx), %r14
	blsrl	%ebp, %ebp
	negq	%rcx
	leaq	(%rcx,%rcx,8), %rsi
	movq	-24(%r15,%rsi,8), %rcx
	cmpq	$-1, %rcx
	je	.LBB361_182
	leaq	(%r15,%rsi,8), %rsi
	vmovups	-16(%rsi), %xmm0
	movq	-32(%rsi), %rdi
	vmovaps	%xmm0, 640(%rsp)
	vmovdqu	-72(%rsi), %ymm1
	movq	-40(%rsi), %rsi
	movq	%rsi, 416(%rsp)
	vmovdqu	%ymm1, 384(%rsp)
	movq	%rdi, 424(%rsp)
	movq	%rcx, 432(%rsp)
	vmovups	%xmm0, (%r13)
	cmpq	224(%rsp), %rbx
	jne	.LBB361_173
.Ltmp15828:
	movl	$8, %ecx
	movl	$72, %r8d
	leaq	224(%rsp), %rdi
	movq	%rbx, %rsi
	vzeroupper
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.4261325137610144415)
.Ltmp15829:
	movq	232(%rsp), %rax
	jmp	.LBB361_173
.LBB361_181:
	xorl	%r14d, %r14d
.LBB361_182:
	movq	%r12, 832(%rsp)
	movq	%r15, 824(%rsp)
	movw	%bp, 848(%rsp)
	movq	%r14, 856(%rsp)
.LBB361_183:
	leaq	800(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>
	vmovdqu	224(%rsp), %xmm0
	movq	240(%rsp), %r15
	movq	%r15, 1056(%rsp)
	vmovdqa	%xmm0, 1040(%rsp)
	movq	1048(%rsp), %rdi
	movq	%rdi, 32(%rsp)
	cmpq	$2, %r15
	jae	.LBB361_890
.LBB361_184:
	movq	328(%rsp), %rax
	movq	32(%rax), %rax
	movq	%rax, 1168(%rsp)
	movq	2520(%rsp), %rax
	cmpb	$2, 472(%rax)
	jne	.LBB361_187
	movq	616(%rax), %rax
	testq	%rax, %rax
	je	.LBB361_260
	cmpq	$-2, 24(%rax)
	jb	.LBB361_187
	cmpq	$-2, 32(%rax)
	jae	.LBB361_271
.LBB361_187:
	xorl	%ebx, %ebx
.LBB361_188:
	movq	184(%rsp), %r8
	addq	$16, %r8
.Ltmp15855:
	movq	1160(%rsp), %rsi
	movq	336(%rsp), %rdx
	movq	2512(%rsp), %rcx
	movq	2520(%rsp), %r9
	leaq	1096(%rsp), %r12
	movq	%r12, %rdi
	vzeroupper
	callq	purrdf_sparql_eval::modifier::link_aggregates::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp15856:
	testb	%bl, %bl
	je	.LBB361_195
	movq	2520(%rsp), %rbx
	movq	1040(%rbx), %rax
	movq	616(%rbx), %rsi
	addq	904(%rbx), %rax
	movq	%rax, 1896(%rsp)
.Ltmp15879:
	leaq	1968(%rsp), %r14
	movq	%r14, %rdi
	callq	<purrdf_sparql_eval::row_checkpoint::ItemLedger>::for_items::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp15880:
.Ltmp15881:
	movq	%rbx, %rdi
	movq	%r15, %rsi
	callq	<purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot
.Ltmp15882:
	movq	%rax, 624(%rsp)
	movq	616(%rbx), %rax
	testq	%rax, %rax
	je	.LBB361_264
	cmpq	$-2, 16(%rax)
	movb	$1, %cl
	jb	.LBB361_265
	cmpq	$-2, 40(%rax)
	setb	%cl
	jmp	.LBB361_265
.LBB361_195:
	movq	72(%rsp), %rbx
	testq	%r15, %r15
	je	.LBB361_262
	movq	malloc@GOTPCREL(%rip), %r12
	leaq	(,%r15,8), %rax
	leaq	(%rax,%rax,4), %r14
	movq	%r14, %rdi
	callq	*%r12
	testq	%rax, %rax
	je	.LBB361_949
	movq	%rax, %r13
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rsi
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdi
	movabsq	$-9223372036854775808, %rcx
	movq	32(%rsp), %r12
	movq	$-1, %r8
	leaq	(%r14,%rax), %rdx
	sarq	$63, %rdx
	xorq	%rcx, %rdx
	addq	%r14, %rax
	cmovoq	%rdx, %rax
	incq	%rsi
	cmoveq	%r8, %rsi
	addq	%r14, %rdi
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmovbq	%r8, %rdi
	movq	%rsi, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%rdi, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB361_199
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_199:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_205
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_199
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
.LBB361_202:
	cmpq	%rax, %rdx
	jle	.LBB361_204
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB361_202
.LBB361_204:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_205:
	addq	$16, 336(%rsp)
	leaq	(%r15,%r15,8), %rax
	movq	%r15, 1968(%rsp)
	movq	%r13, 1976(%rsp)
	xorl	%ebp, %ebp
	movq	$0, 1984(%rsp)
	leaq	(%r12,%rax,8), %rax
	movq	%rax, 144(%rsp)
	jmp	.LBB361_207
.LBB361_206:
	leaq	(%rbp,%rbp,4), %rax
	addq	$72, %r12
	incq	%rbp
	movq	%r15, (%r13,%rax,8)
	movq	%r14, 8(%r13,%rax,8)
	vmovdqa	384(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%r13,%rax,8)
	movq	400(%rsp), %rcx
	movq	%rcx, 32(%r13,%rax,8)
	movq	%rbp, 1984(%rsp)
	cmpq	144(%rsp), %r12
	je	.LBB361_263
.LBB361_207:
	movq	1168(%rsp), %r14
	movq	$1, 384(%rsp)
	cmpq	$5, %r14
	jae	.LBB361_233
	vmovdqu	392(%rsp), %xmm0
	movq	416(%rsp), %rax
	movq	384(%rsp), %rdx
	movq	408(%rsp), %rcx
	movq	%rax, 832(%rsp)
	movq	%rdx, 800(%rsp)
	movq	%rcx, 824(%rsp)
	vmovdqu	%xmm0, 808(%rsp)
	testq	%r14, %r14
	je	.LBB361_210
.LBB361_209:
	movl	$2, %eax
	jmp	.LBB361_211
.LBB361_210:
	movl	$-1, %eax
.LBB361_211:
	movl	%eax, 384(%rsp)
	movq	%r14, 392(%rsp)
.Ltmp15861:
	leaq	800(%rsp), %rdi
	leaq	384(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp15862:
	vmovdqu	800(%rsp), %ymm0
	movq	832(%rsp), %rax
	leaq	1192(%rsp), %rdi
	movq	%rax, 1216(%rsp)
	vmovdqu	%ymm0, 1184(%rsp)
	movq	1184(%rsp), %r15
	movq	%r15, %rax
	cmpq	$6, %r15
	jb	.LBB361_214
	movq	1192(%rsp), %rdi
	movq	1200(%rsp), %rax
.LBB361_214:
	movq	728(%rsp), %rdx
	decq	%rax
	cmpq	%rax, %rdx
	ja	.LBB361_892
	movq	(%r12), %rsi
	movq	%r13, 32(%rsp)
	decq	%rsi
	cmpq	$4, %rsi
	jbe	.LBB361_217
	movq	16(%r12), %rsi
	movq	8(%r12), %rax
	decq	%rsi
	jmp	.LBB361_218
.LBB361_217:
	leaq	8(%r12), %rax
.LBB361_218:
	cmpq	%rsi, %rdx
	jne	.LBB361_893
	movq	memcpy@GOTPCREL(%rip), %r14
	shlq	$3, %rdx
	movq	%rax, %rsi
	movq	%r12, %r13
	vzeroupper
	callq	*%r14
	movq	1112(%rsp), %r12
	movq	2512(%rsp), %rax
	cmpq	%r12, %rax
	cmovbq	%rax, %r12
	testq	%r12, %r12
	je	.LBB361_230
	movq	%rbp, 96(%rsp)
	movq	1104(%rsp), %rbp
	movq	336(%rsp), %r14
	xorl	%ebx, %ebx
	addq	$16, %rbp
	jmp	.LBB361_222
	.p2align	4
.LBB361_221:
	incq	%rbx
	addq	$24, %rbp
	addq	$120, %r14
	vmovq	%xmm0, (%rax,%rdi,8)
	cmpq	%rbx, %r12
	je	.LBB361_229
.LBB361_222:
	vmovups	1128(%rsp), %xmm0
	movq	184(%rsp), %rax
	movq	-8(%rbp), %rdx
	movq	(%rbp), %rcx
	movq	56(%r13), %r8
	movq	64(%r13), %r9
	addq	$16, %rax
.Ltmp15866:
	movq	2520(%rsp), %rsi
	leaq	384(%rsp), %rdi
	movq	%rax, 16(%rsp)
	movq	%rsi, 24(%rsp)
	movq	%r14, %rsi
	vmovups	%xmm0, (%rsp)
	callq	purrdf_sparql_eval::modifier::eval_aggregate::<purrdf_core::ir::dataset::RdfDataset, ()>
.Ltmp15867:
	vmovq	392(%rsp), %xmm0
	movq	384(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB361_235
	movq	1184(%rsp), %r15
	movq	%r15, %rsi
	cmpq	$6, %r15
	jb	.LBB361_226
	movq	1200(%rsp), %rsi
.LBB361_226:
	movq	728(%rsp), %rdi
	decq	%rsi
	addq	%rbx, %rdi
	cmpq	%rsi, %rdi
	jae	.LBB361_941
	leaq	1192(%rsp), %rax
	cmpq	$6, %r15
	jb	.LBB361_221
	movq	1192(%rsp), %rax
	jmp	.LBB361_221
.LBB361_229:
	movq	1184(%rsp), %r15
	movq	72(%rsp), %rbx
	movq	96(%rsp), %rbp
.LBB361_230:
	leaq	1192(%rsp), %rax
	movq	%r13, %r12
	movq	1192(%rsp), %r14
	movq	32(%rsp), %r13
	vmovups	8(%rax), %xmm0
	movq	24(%rax), %rax
	movq	%rax, 400(%rsp)
	vmovaps	%xmm0, 384(%rsp)
	cmpq	1968(%rsp), %rbp
	jne	.LBB361_206
.Ltmp15871:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	1968(%rsp), %rdi
	callq	*%rax
.Ltmp15872:
	movq	1976(%rsp), %r13
	jmp	.LBB361_206
.LBB361_233:
.Ltmp15858:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	384(%rsp), %rdi
	xorl	%esi, %esi
	movq	%r14, %rdx
	xorl	%ecx, %ecx
	callq	*%rax
.Ltmp15859:
	vmovdqu	384(%rsp), %ymm0
	movq	416(%rsp), %rax
	movq	%rax, 832(%rsp)
	vmovdqu	%ymm0, 800(%rsp)
	jmp	.LBB361_209
.LBB361_235:
	vmovdqu64	400(%rsp), %zmm1
	vmovdqu64	416(%rsp), %zmm2
	movq	72(%rsp), %rcx
	vmovdqu64	%zmm2, 48(%rcx)
	vmovdqu64	%zmm1, 32(%rcx)
	movq	%rax, 16(%rcx)
	movq	1184(%rsp), %rax
	vmovq	%xmm0, 24(%rcx)
	movq	$1, (%rcx)
	cmpq	$6, %rax
	jb	.LBB361_237
	movq	1192(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB361_237:
	movq	96(%rsp), %rbp
	movq	32(%rsp), %r13
	testq	%rbp, %rbp
	je	.LBB361_250
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r14
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r15
	movq	free@GOTPCREL(%rip), %r12
	xorl	%ebx, %ebx
	jmp	.LBB361_242
	.p2align	4
.LBB361_239:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_240:
	vzeroupper
	callq	*%r12
.LBB361_241:
	incq	%rbx
	cmpq	%rbp, %rbx
	je	.LBB361_250
.LBB361_242:
	leaq	(%rbx,%rbx,4), %rcx
	movq	(%r13,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB361_241
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
	jge	.LBB361_245
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_245:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_240
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_245
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
.LBB361_248:
	cmpq	%rax, %rdx
	jge	.LBB361_239
	lock		cmpxchgq	%rdx, (%r15)
	jne	.LBB361_248
	jmp	.LBB361_239
.LBB361_250:
	movq	1968(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_472
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
	jge	.LBB361_253
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_253:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_259
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_253
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
.LBB361_256:
	cmpq	%rax, %rdx
	jge	.LBB361_258
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB361_256
.LBB361_258:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_259:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	vzeroupper
	callq	*%rax
	jmp	.LBB361_472
.LBB361_260:
	cmpq	$0, 2512(%rsp)
	jne	.LBB361_272
	movb	$1, %bl
	jmp	.LBB361_188
.LBB361_262:
	movq	$0, 1968(%rsp)
	movq	$8, 1976(%rsp)
	movq	$0, 1984(%rsp)
.LBB361_263:
	movq	1984(%rsp), %rax
	movq	1968(%rsp), %rdx
	movq	1976(%rsp), %rcx
	movq	%rax, 1456(%rsp)
	movq	%rdx, 1440(%rsp)
	movq	%rcx, 1448(%rsp)
	jmp	.LBB361_614
.LBB361_264:
	xorl	%ecx, %ecx
.LBB361_265:
	movq	2520(%rsp), %rdx
	leaq	624(%rsp), %rsi
	movq	%r12, 1664(%rsp)
	leaq	1168(%rsp), %rdi
	movq	%rdi, 224(%rsp)
	movq	336(%rsp), %rdi
	movq	%rdx, 1672(%rsp)
	movq	%rsi, 1680(%rsp)
	leaq	728(%rsp), %rsi
	movzbl	1234(%rdx), %eax
	movq	%r14, 1688(%rsp)
	movq	%rsi, 232(%rsp)
	movq	2512(%rsp), %rsi
	movq	%rdi, 240(%rsp)
	leaq	1120(%rsp), %rdi
	xorb	$1, %al
	movq	%rsi, 248(%rsp)
	leaq	184(%rsp), %rsi
	movq	%rdi, 256(%rsp)
	leaq	639(%rsp), %rdi
	movq	%rsi, 264(%rsp)
	leaq	1896(%rsp), %rsi
	movq	%rdi, 272(%rsp)
	movq	%rsi, 280(%rsp)
	testb	%cl, %cl
	je	.LBB361_268
	movq	32(%rsp), %rbx
	cmpq	$1025, %r15
	movq	%r14, 1528(%rsp)
	setae	%cl
	testb	%al, %cl
	jne	.LBB361_275
	vmovdqu	1664(%rsp), %ymm0
	vmovdqu64	224(%rsp), %zmm1
	leaq	1360(%rsp), %rax
	movq	%rbx, 344(%rsp)
	movq	%r15, 352(%rsp)
	movq	%r14, 768(%rsp)
	movq	%rax, 640(%rsp)
	leaq	344(%rsp), %rax
	movq	%rax, 648(%rsp)
	leaq	800(%rsp), %rax
	movq	%rax, 656(%rsp)
	leaq	768(%rsp), %rax
	movq	%rax, 664(%rsp)
	vmovdqu	%ymm0, 1360(%rsp)
	vmovdqu64	%zmm1, 800(%rsp)
.Ltmp15941:
	leaq	384(%rsp), %rdi
	leaq	640(%rsp), %rsi
	vzeroupper
	callq	purrdf_sparql_eval::parallel::par_chunk_try_map_init::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#6}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#7}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#8}>::{closure#0}
.Ltmp15942:
	jmp	.LBB361_337
.LBB361_268:
	movq	32(%rsp), %rbp
	leaq	1664(%rsp), %rcx
	cmpq	$1025, %r15
	movq	%r14, 200(%rsp)
	movq	%rcx, 640(%rsp)
	leaq	1528(%rsp), %rcx
	movq	%rcx, 648(%rsp)
	leaq	224(%rsp), %rcx
	movq	%rcx, 656(%rsp)
	leaq	200(%rsp), %rcx
	movq	%rcx, 664(%rsp)
	setae	%cl
	movq	%rbp, 1528(%rsp)
	movq	%r15, 1536(%rsp)
	testb	%al, %cl
	jne	.LBB361_277
.Ltmp15905:
	leaq	384(%rsp), %rdi
	leaq	640(%rsp), %rsi
	callq	purrdf_sparql_eval::parallel::par_chunk_try_map_init::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#6}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#7}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#8}>::{closure#0}
.Ltmp15906:
	jmp	.LBB361_337
.LBB361_275:
	movq	rayon_core::registry::WORKER_THREAD_STATE::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %r12
	movq	%fs:(%r12), %rax
	testq	%rax, %rax
	je	.LBB361_311
	addq	$272, %rax
	jmp	.LBB361_312
.LBB361_277:
	movq	rayon_core::registry::WORKER_THREAD_STATE::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rax
	movq	%fs:(%rax), %rax
	testq	%rax, %rax
	je	.LBB361_314
	addq	$272, %rax
	movq	%r15, %r14
	jmp	.LBB361_316
.LBB361_271:
	cmpq	$0, 2512(%rsp)
	sete	%cl
	cmpq	$-2, 48(%rax)
	setb	%al
	setae	%bl
	orb	%cl, %al
	jne	.LBB361_188
.LBB361_272:
	movq	2520(%rsp), %rax
	movq	336(%rsp), %r12
	leaq	384(%rsp), %rbp
	addq	$584, %rax
	movq	%rax, 96(%rsp)
.LBB361_273:
	movq	56(%r12), %r13
	movq	96(%rsp), %rbx
	testq	%r13, %r13
	je	.LBB361_291
	movq	48(%r12), %r14
	shlq	$6, %r13
	jmp	.LBB361_280
	.p2align	4
.LBB361_279:
	addq	$64, %r14
	addq	$-64, %r13
	je	.LBB361_291
.LBB361_280:
	movq	2520(%rsp), %rax
	cmpq	$0, 584(%rax)
	movq	672(%rax), %rdx
	movq	680(%rax), %rcx
	je	.LBB361_283
	movq	%rbx, 384(%rsp)
.Ltmp15836:
	leaq	<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop::{closure#0}(%rip), %r9
	xorl	%edi, %edi
	movq	%r14, %rsi
	movq	%rbp, %r8
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.4261325137610144415)
.Ltmp15837:
	jmp	.LBB361_284
.LBB361_283:
.Ltmp15838:
	movl	$1, %r8d
	leaq	purrdf_sparql_eval::parallel::is_parallel_safe_pattern::{closure#0} (.llvm.4261325137610144415)(%rip), %r9
	xorl	%edi, %edi
	movq	%r14, %rsi
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.4261325137610144415)
.Ltmp15839:
.LBB361_284:
	testb	%al, %al
	jne	.LBB361_187
	movq	2520(%rsp), %rax
	movq	616(%rax), %rax
	testq	%rax, %rax
	je	.LBB361_279
	cmpq	$0, 336(%rax)
	jne	.LBB361_289
	vpcmpeqd	%ymm0, %ymm0, %ymm0
	vpcmpneqq	16(%rax), %ymm0, %k0
	kmovd	%k0, %ecx
	testb	$15, %cl
	jne	.LBB361_289
	cmpq	$-1, 48(%rax)
	je	.LBB361_279
.LBB361_289:
.Ltmp15840:
	movq	purrdf_sparql_eval::parallel::expression_re_enters_evaluation@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15841:
	testb	%al, %al
	je	.LBB361_279
	jmp	.LBB361_187
.LBB361_291:
	movq	104(%r12), %rax
	testq	%rax, %rax
	je	.LBB361_305
	movq	96(%r12), %r14
	shlq	$3, %rax
	leaq	(%rax,%rax,8), %r13
	addq	$8, %r14
	jmp	.LBB361_294
	.p2align	4
.LBB361_293:
	addq	$72, %r14
	addq	$-72, %r13
	je	.LBB361_305
.LBB361_294:
	movq	2520(%rsp), %rax
	cmpq	$0, 584(%rax)
	movq	672(%rax), %rdx
	movq	680(%rax), %rcx
	je	.LBB361_297
	movq	%rbx, 384(%rsp)
.Ltmp15843:
	leaq	<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop::{closure#0}(%rip), %r9
	xorl	%edi, %edi
	movq	%r14, %rsi
	movq	%rbp, %r8
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.4261325137610144415)
.Ltmp15844:
	jmp	.LBB361_298
.LBB361_297:
.Ltmp15845:
	movl	$1, %r8d
	leaq	purrdf_sparql_eval::parallel::is_parallel_safe_pattern::{closure#0} (.llvm.4261325137610144415)(%rip), %r9
	xorl	%edi, %edi
	movq	%r14, %rsi
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.4261325137610144415)
.Ltmp15846:
.LBB361_298:
	testb	%al, %al
	jne	.LBB361_187
	movq	2520(%rsp), %rax
	movq	616(%rax), %rax
	testq	%rax, %rax
	je	.LBB361_293
	cmpq	$0, 336(%rax)
	jne	.LBB361_303
	vpcmpeqd	%ymm0, %ymm0, %ymm0
	vpcmpneqq	16(%rax), %ymm0, %k0
	kmovd	%k0, %ecx
	testb	$15, %cl
	jne	.LBB361_303
	cmpq	$-1, 48(%rax)
	je	.LBB361_293
.LBB361_303:
.Ltmp15847:
	movq	purrdf_sparql_eval::parallel::expression_re_enters_evaluation@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15848:
	testb	%al, %al
	je	.LBB361_293
	jmp	.LBB361_187
.LBB361_305:
	cmpl	$8, 16(%r12)
	jne	.LBB361_310
	movq	2520(%rsp), %rax
	movq	24(%r12), %rsi
	movq	32(%r12), %rdx
	movq	688(%rax), %rdi
	addq	$16, %rsi
.Ltmp15850:
	movq	<purrdf_sparql_eval::agg_fn::AggregateRegistry>::resolve@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp15851:
	testq	%rax, %rax
	je	.LBB361_187
	movq	(%rax), %rcx
	movq	8(%rax), %rax
	movq	16(%rax), %rdx
	movq	32(%rax), %rax
	decq	%rdx
	andq	$-16, %rdx
	leaq	16(%rcx,%rdx), %rdi
.Ltmp15852:
	callq	*%rax
.Ltmp15853:
	testb	%al, %al
	jne	.LBB361_187
.LBB361_310:
	addq	$120, %r12
	movb	$1, %bl
	cmpq	64(%rsp), %r12
	jne	.LBB361_273
	jmp	.LBB361_188
.LBB361_311:
.Ltmp15907:
	movq	rayon_core::registry::global_registry@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15908:
.LBB361_312:
	movq	(%rax), %rax
	movq	520(%rax), %rbp
	movq	%r15, %rax
	cmpq	$1, %rbp
	adcq	$0, %rbp
	movq	%rbp, %rcx
	shlq	$6, %rcx
	orq	%rcx, %rax
	shrq	$32, %rax
	je	.LBB361_318
	movq	%r15, %rax
	xorl	%edx, %edx
	divq	%rcx
	jmp	.LBB361_319
.LBB361_314:
.Ltmp15883:
	movq	rayon_core::registry::global_registry@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15884:
	movq	1528(%rsp), %rbp
	movq	1536(%rsp), %r14
.LBB361_316:
	movq	(%rax), %rax
	movq	520(%rax), %rcx
	movq	%r15, %rax
	cmpq	$1, %rcx
	adcq	$0, %rcx
	shlq	$2, %rcx
	orq	%rcx, %rax
	shrq	$32, %rax
	je	.LBB361_325
	movq	%r15, %rax
	xorl	%edx, %edx
	divq	%rcx
	jmp	.LBB361_326
.LBB361_318:
	movl	%r15d, %eax
	xorl	%edx, %edx
	divl	%ecx
.LBB361_319:
	cmpq	$65, %rax
	movl	$64, %ecx
	movq	%rbx, 800(%rsp)
	movq	%r15, 808(%rsp)
	cmovaeq	%rax, %rcx
	movq	%rcx, 816(%rsp)
.Ltmp15909:
	leaq	768(%rsp), %r14
	leaq	800(%rsp), %rsi
	movq	%r14, %rdi
	callq	<alloc::vec::Vec<&[(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)]> as alloc::vec::spec_from_iter::SpecFromIter<&[(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)], core::slice::iter::Chunks<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>>>::from_iter
.Ltmp15910:
	movq	784(%rsp), %r13
	movabsq	$38430716820228232, %rax
	imulq	$240, %r13, %r15
	cmpq	%rax, %r13
	jbe	.LBB361_323
	xorl	%r12d, %r12d
.LBB361_322:
.Ltmp15938:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	movq	%r15, %rsi
	callq	*%rax
.Ltmp15939:
	jmp	.LBB361_950
.LBB361_323:
	movq	%r12, %rbx
	testq	%r15, %r15
	je	.LBB361_374
	movl	$16, %esi
	movq	%r15, %rdi
	movl	$16, %r12d
	callq	__rustc::__rust_alloc
	movq	%r13, %rcx
	testq	%rax, %rax
	jne	.LBB361_375
	jmp	.LBB361_322
.LBB361_325:
	movl	%r15d, %eax
	xorl	%edx, %edx
	divl	%ecx
.LBB361_326:
	cmpq	$17, %rax
	movl	$16, %r12d
	movq	$0, 736(%rsp)
	movq	$16, 744(%rsp)
	movq	$0, 752(%rsp)
	cmovaeq	%rax, %r12
	movq	%r14, %rax
	orq	%r12, %rax
	shrq	$32, %rax
	je	.LBB361_328
	movq	%r14, %rax
	xorl	%edx, %edx
	divq	%r12
	jmp	.LBB361_329
.LBB361_328:
	movl	%r14d, %eax
	xorl	%edx, %edx
	divl	%r12d
.LBB361_329:
	xorl	%r15d, %r15d
	testq	%rdx, %rdx
	setne	%r15b
	addq	%rax, %r15
	movq	%r15, 1936(%rsp)
	jne	.LBB361_899
	xorl	%eax, %eax
	xorl	%r13d, %r13d
	subq	%r13, %rax
	cmpq	%r15, %rax
	jb	.LBB361_901
.LBB361_331:
	leaq	1664(%rsp), %rax
	leaq	224(%rsp), %rcx
	movq	%rbp, 800(%rsp)
	movq	%r14, 808(%rsp)
	movq	%r12, 816(%rsp)
	movq	744(%rsp), %rbx
	leaq	200(%rsp), %rdx
	movq	%r12, 360(%rsp)
	movq	%rbp, 344(%rsp)
	movq	%r14, 352(%rsp)
	movq	%rax, 824(%rsp)
	movq	%rcx, 832(%rsp)
	movq	rayon_core::registry::WORKER_THREAD_STATE::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rcx
	movq	%rdx, 840(%rsp)
	movq	%fs:(%rcx), %rax
	testq	%rax, %rax
	je	.LBB361_333
	addq	$272, %rax
	jmp	.LBB361_334
.LBB361_333:
.Ltmp15887:
	movq	rayon_core::registry::global_registry@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15888:
.LBB361_334:
	movq	(%rax), %rax
	leaq	824(%rsp), %rdx
	movq	520(%rax), %rcx
	imulq	$224, %r13, %rax
	movq	%rdx, 1360(%rsp)
	addq	%rax, %rbx
	movq	%rbx, 1368(%rsp)
	movq	%r15, 1376(%rsp)
.Ltmp15889:
	leaq	1360(%rsp), %rbx
	leaq	768(%rsp), %rdi
	leaq	344(%rsp), %r9
	movl	$1, %r8d
	movq	%r15, %rsi
	xorl	%edx, %edx
	movq	%rbx, (%rsp)
	callq	rayon::iter::plumbing::bridge_producer_consumer::helper::<rayon::slice::chunks::ChunksProducer<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>, rayon::iter::map::MapConsumer<rayon::iter::collect::consumer::CollectConsumer<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>, purrdf_sparql_eval::parallel::par_chunk_try_map_init<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#6}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#7}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#8}>::{closure#1}>>
.Ltmp15890:
	movq	784(%rsp), %r14
	movq	%r14, 1360(%rsp)
	cmpq	%r15, %r14
	jne	.LBB361_902
	vmovdqu	736(%rsp), %xmm0
	addq	%r15, %r13
	movq	%r13, 816(%rsp)
	vmovdqa	%xmm0, 800(%rsp)
.Ltmp15897:
	leaq	384(%rsp), %rdi
	leaq	800(%rsp), %rsi
	callq	purrdf_sparql_eval::parallel::concatenate_chunks::<purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)>
.Ltmp15898:
.LBB361_337:
	movq	72(%rsp), %rbx
.LBB361_338:
	movq	384(%rsp), %rcx
	cmpq	$-1, %rcx
	je	.LBB361_351
	vmovups	544(%rsp), %zmm0
	vmovups	496(%rsp), %zmm2
	movq	408(%rsp), %rax
	vmovdqu	392(%rsp), %xmm1
	movq	424(%rsp), %rsi
	movq	416(%rsp), %rdx
	movq	400(%rsp), %rbx
	movq	%rcx, 1416(%rsp)
	movl	$1, %edi
	leaq	-3(%rax), %rcx
	cmpq	$-2, %rcx
	movl	$1, %ecx
	cmovbq	%rax, %rdi
	cmovbq	%rsi, %rax
	cmovaeq	%rsi, %rcx
	vmovups	%zmm0, 1296(%rsp)
	vmovups	%zmm2, 1248(%rsp)
	vmovdqu64	432(%rsp), %zmm0
	decq	%rax
	vmovdqu	%xmm1, 1424(%rsp)
	vmovdqu64	1296(%rsp), %zmm3
	vmovdqu64	1248(%rsp), %zmm2
	vmovdqu64	%zmm0, 1184(%rsp)
	vmovdqu64	%zmm0, 408(%rsp)
	vmovdqu64	%zmm3, 520(%rsp)
	vmovdqu64	%zmm2, 472(%rsp)
	movq	%rdi, 384(%rsp)
	movq	%rdx, 392(%rsp)
	movq	%rcx, 400(%rsp)
	movq	$0, 584(%rsp)
	movq	%rax, 592(%rsp)
.Ltmp15944:
	leaq	800(%rsp), %rdi
	leaq	384(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
.Ltmp15945:
	vmovups	832(%rsp), %zmm1
	movq	2520(%rsp), %rax
	vmovups	800(%rsp), %ymm0
	vmovdqu64	896(%rsp), %zmm2
	cmpq	$0, 616(%rax)
	vmovups	%zmm1, 2160(%rsp)
	vmovdqu64	944(%rsp), %zmm1
	vmovdqu64	%zmm2, 2224(%rsp)
	vmovups	%ymm0, 1936(%rsp)
	vmovdqu64	%zmm1, 2272(%rsp)
	je	.LBB361_352
	vmovdqu	1416(%rsp), %xmm0
	vmovdqu64	2160(%rsp), %zmm3
	vmovdqu64	2272(%rsp), %zmm2
	vmovdqu64	2224(%rsp), %zmm1
	addq	$888, %rax
	movzbl	2128(%rsp), %r15d
	movq	%rax, 96(%rsp)
	movq	1432(%rsp), %rax
	movq	%rax, 1488(%rsp)
	vmovdqu64	%zmm2, 912(%rsp)
	vmovdqa	%xmm0, 1472(%rsp)
	vmovdqu64	%zmm1, 864(%rsp)
	vmovdqu64	%zmm3, 800(%rsp)
	testb	%r15b, %r15b
	je	.LBB361_354
	movq	800(%rsp), %rax
	vmovdqu64	2184(%rsp), %zmm0
	vmovdqu64	2272(%rsp), %zmm2
	vmovdqu64	2248(%rsp), %zmm1
	movq	816(%rsp), %rcx
	movq	808(%rsp), %r14
	movb	%r15b, 64(%rsp)
	movl	$1, %edx
	movl	$1, %esi
	movq	$0, 224(%rsp)
	movq	$8, 232(%rsp)
	movq	$0, 240(%rsp)
	cmpq	$3, %rax
	movq	%rax, %r15
	cmovaeq	%rcx, %r15
	cmovaeq	%rdx, %rcx
	cmovaeq	%rax, %rsi
	leaq	392(%rsp), %rdx
	decq	%r15
	vmovdqu64	%zmm2, 496(%rsp)
	vmovdqu64	%zmm1, 472(%rsp)
	vmovdqu64	%zmm0, 408(%rsp)
	movq	%rsi, 384(%rsp)
	movq	%r14, 392(%rsp)
	movq	%rcx, 400(%rsp)
	movq	$0, 560(%rsp)
	movq	%r15, 568(%rsp)
	je	.LBB361_363
	cmpq	$3, %rax
	leaq	1192(%rsp), %r12
	movl	$8, %ecx
	cmovbq	%rdx, %r14
	xorl	%ebx, %ebx
	xorl	%ebp, %ebp
	addq	$8, %r14
.LBB361_344:
	leaq	1(%rbx), %r13
	movq	%r13, 560(%rsp)
	movq	-8(%r14), %rax
	cmpq	$-1, %rax
	je	.LBB361_395
	vmovups	(%r14), %zmm0
	vmovups	64(%r14), %zmm1
	vmovups	88(%r14), %zmm2
	vmovups	%zmm2, 88(%r12)
	vmovups	%zmm1, 64(%r12)
	vmovups	%zmm0, (%r12)
	movq	%rax, 1184(%rsp)
	movq	%rbx, %rax
	movzbl	1336(%rsp), %ebx
	cmpq	224(%rsp), %rax
	jne	.LBB361_348
.Ltmp15974:
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one@GOTPCREL(%rip), %rax
	leaq	224(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15975:
	movq	232(%rsp), %rcx
.LBB361_348:
	vmovdqu64	1184(%rsp), %zmm0
	vmovdqu64	1248(%rsp), %zmm1
	vmovdqu64	1280(%rsp), %zmm2
	vmovdqu64	%zmm2, 96(%rcx,%rbp)
	vmovdqu64	%zmm1, 64(%rcx,%rbp)
	vmovdqu64	%zmm0, (%rcx,%rbp)
	movq	%r13, 240(%rsp)
	testb	%bl, %bl
	jne	.LBB361_396
	addq	$160, %rbp
	addq	$168, %r14
	movq	%r13, %rbx
	cmpq	%r13, %r15
	jne	.LBB361_344
	xorl	%ebp, %ebp
	movq	%r15, %rbx
	jmp	.LBB361_397
.LBB361_351:
	vmovdqu64	432(%rsp), %zmm0
	vmovdqu	400(%rsp), %ymm1
	vmovdqu64	%zmm0, 48(%rbx)
	vmovdqu	%ymm1, 16(%rbx)
	vmovdqu64	%zmm0, 1184(%rsp)
	movq	$1, (%rbx)
	movq	624(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB361_469
	jmp	.LBB361_471
.LBB361_352:
	leaq	1128(%rsp), %rcx
	movq	1120(%rsp), %rax
	vmovups	(%rcx), %xmm0
	movq	$0, 1120(%rsp)
	movq	$8, 1128(%rsp)
	movq	$0, 1136(%rsp)
	vmovaps	%xmm0, 800(%rsp)
	cmpq	%rbx, %rax
	jbe	.LBB361_358
	vmovdqa	800(%rsp), %xmm0
	leaq	384(%rsp), %rdi
	movq	%rax, 384(%rsp)
	vmovdqu	%xmm0, 392(%rsp)
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	$8, 1920(%rsp)
	movq	$0, 1928(%rsp)
	xorl	%eax, %eax
	jmp	.LBB361_359
.LBB361_354:
	movq	1488(%rsp), %rbx
	movq	$0, 1360(%rsp)
	movq	$8, 1368(%rsp)
	movq	$0, 1376(%rsp)
.Ltmp15949:
	leaq	384(%rsp), %rdi
	leaq	1360(%rsp), %rsi
	movq	%rbx, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp15950:
	movq	384(%rsp), %r14
	movq	392(%rsp), %rax
	movq	400(%rsp), %rdx
	movq	408(%rsp), %rbp
	cmpq	$-1, %r14
	je	.LBB361_364
	vmovdqu	432(%rsp), %ymm0
	vmovdqu	448(%rsp), %ymm1
	movq	416(%rsp), %r12
	movq	424(%rsp), %r15
	movq	%rax, 144(%rsp)
	movq	%rdx, 96(%rsp)
	vmovdqu	%ymm0, 1184(%rsp)
	vmovdqu	%ymm1, 1200(%rsp)
.Ltmp15954:
	leaq	1416(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15955:
	movq	72(%rsp), %rbx
.LBB361_357:
	vmovups	1184(%rsp), %ymm0
	vmovups	1200(%rsp), %ymm1
	movq	%r12, %r13
	shrq	$8, %r13
	vmovups	%ymm0, 1664(%rsp)
	vmovups	%ymm1, 1680(%rsp)
	jmp	.LBB361_443
.LBB361_358:
	vmovdqa	800(%rsp), %xmm0
	vmovdqu	%xmm0, 1920(%rsp)
.LBB361_359:
	movq	72(%rsp), %rbx
	movl	2128(%rsp), %esi
	movq	%rax, 1912(%rsp)
.Ltmp16112:
	movq	2520(%rsp), %rdx
	leaq	384(%rsp), %rdi
	leaq	1416(%rsp), %rcx
	leaq	2160(%rsp), %r8
	leaq	1912(%rsp), %r9
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::ItemLedger>::commit_into::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#9}>
.Ltmp16113:
	movq	384(%rsp), %rax
	movq	392(%rsp), %rdx
	movq	400(%rsp), %rcx
	movq	408(%rsp), %rbp
	movq	424(%rsp), %r15
	movq	416(%rsp), %r14
	cmpq	$-1, %rax
	je	.LBB361_362
	vmovdqu	448(%rsp), %ymm1
	vmovdqu	432(%rsp), %ymm0
	vmovdqu	%ymm1, 80(%rbx)
	vmovdqu	%ymm0, 64(%rbx)
	movq	%rax, 16(%rbx)
	movq	%rdx, 24(%rbx)
	movq	%rcx, 32(%rbx)
	movq	%rbp, 40(%rbx)
	movq	%r14, 48(%rbx)
	jmp	.LBB361_467
.LBB361_362:
	movq	%rdx, 144(%rsp)
	movq	%rcx, 96(%rsp)
	jmp	.LBB361_606
.LBB361_363:
	xorl	%ebx, %ebx
	movl	$8, %r15d
	xorl	%ebp, %ebp
	jmp	.LBB361_398
.LBB361_364:
	movb	%r15b, 64(%rsp)
	movq	1480(%rsp), %r15
	movq	%rax, 640(%rsp)
	leaq	(,%rbx,8), %rcx
	movq	1472(%rsp), %rax
	movq	%rdx, 648(%rsp)
	movq	%rbp, 656(%rsp)
	leaq	(%rcx,%rcx,4), %r12
	leaq	(%r15,%r12), %rcx
	movq	%r15, 224(%rsp)
	movq	%rax, 240(%rsp)
	movq	%rcx, 248(%rsp)
	testq	%rbx, %rbx
	je	.LBB361_441
	leaq	(,%rbp,8), %rax
	addq	$40, %r15
	movq	%rcx, 48(%rsp)
	leaq	(%rax,%rax,4), %rbx
	jmp	.LBB361_368
.LBB361_366:
	movq	648(%rsp), %rdx
.LBB361_367:
	vmovdqa	144(%rsp), %xmm0
	movq	32(%rsp), %rax
	movq	%r14, (%rdx,%rbx)
	incq	%rbp
	addq	$40, %r15
	movq	%rax, 8(%rdx,%rbx)
	vmovdqu	%xmm0, 16(%rdx,%rbx)
	movq	%r13, 32(%rdx,%rbx)
	addq	$40, %rbx
	addq	$-40, %r12
	movq	%rbp, 656(%rsp)
	je	.LBB361_440
.LBB361_368:
	movq	2520(%rsp), %rcx
	leaq	1192(%rsp), %rsi
	movq	%rcx, 1184(%rsp)
	movq	-8(%r15), %rax
	movq	%rax, 32(%rsi)
	vmovdqu	-40(%r15), %ymm0
	vmovdqu	%ymm0, (%rsi)
	cmpq	$0, 1192(%rsp)
	je	.LBB361_370
	leaq	-40(%r15), %rax
	leaq	392(%rsp), %rsi
	movq	32(%rax), %rcx
	movq	%rcx, 32(%rsi)
	vmovdqu	(%rax), %ymm0
	vmovdqu	%ymm0, (%rsi)
	jmp	.LBB361_372
.LBB361_370:
	movq	%rdx, %r14
	movq	664(%rcx), %rdx
.Ltmp15957:
	movq	96(%rsp), %rsi
	leaq	384(%rsp), %rdi
	leaq	1200(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp15958:
	movq	384(%rsp), %r13
	movq	%r14, %rdx
	cmpq	$-1, %r13
	jne	.LBB361_579
.LBB361_372:
	vmovdqu	408(%rsp), %xmm0
	movq	400(%rsp), %rax
	movq	392(%rsp), %r14
	movq	424(%rsp), %r13
	movq	%rax, 32(%rsp)
	vmovdqa	%xmm0, 144(%rsp)
	cmpq	640(%rsp), %rbp
	jne	.LBB361_367
.Ltmp15962:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	640(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15963:
	jmp	.LBB361_366
.LBB361_374:
	movl	$16, %eax
	xorl	%ecx, %ecx
.LBB361_375:
	testq	%r13, %r13
	je	.LBB361_378
	movl	%r13d, %edx
	andl	$7, %edx
	movq	%rbx, %r9
	cmpq	$8, %r13
	jae	.LBB361_379
	xorl	%esi, %esi
	leaq	800(%rsp), %rbx
	jmp	.LBB361_382
.LBB361_378:
	movq	%rbx, %r9
	xorl	%r15d, %r15d
	leaq	800(%rsp), %rbx
	jmp	.LBB361_385
.LBB361_379:
	movabsq	$72057594037927928, %rdi
	leaq	1696(%rax), %r8
	leaq	800(%rsp), %rbx
	xorl	%esi, %esi
	andq	%r13, %rdi
.LBB361_380:
	movl	$0, -1696(%r8)
	movb	$0, -1692(%r8)
	movq	$2, -1680(%r8)
	movl	$0, -1456(%r8)
	movb	$0, -1452(%r8)
	movq	$2, -1440(%r8)
	movl	$0, -1216(%r8)
	movb	$0, -1212(%r8)
	movq	$2, -1200(%r8)
	movl	$0, -976(%r8)
	movb	$0, -972(%r8)
	movq	$2, -960(%r8)
	movl	$0, -736(%r8)
	movb	$0, -732(%r8)
	movq	$2, -720(%r8)
	movl	$0, -496(%r8)
	movb	$0, -492(%r8)
	movq	$2, -480(%r8)
	movl	$0, -256(%r8)
	movb	$0, -252(%r8)
	movq	$2, -240(%r8)
	movl	$0, -16(%r8)
	movb	$0, -12(%r8)
	movq	$2, (%r8)
	addq	$8, %rsi
	addq	$1920, %r8
	cmpq	%rsi, %rdi
	jne	.LBB361_380
	testq	%rdx, %rdx
	je	.LBB361_384
.LBB361_382:
	imulq	$240, %rsi, %rsi
	imulq	$240, %rdx, %rdx
	xorl	%edi, %edi
	addq	%rax, %rsi
.LBB361_383:
	movl	$0, (%rsi,%rdi)
	movb	$0, 4(%rsi,%rdi)
	movq	$2, 16(%rsi,%rdi)
	addq	$240, %rdi
	cmpq	%rdi, %rdx
	jne	.LBB361_383
.LBB361_384:
	movq	784(%rsp), %r15
.LBB361_385:
	movq	%rcx, 344(%rsp)
	movq	%rax, 352(%rsp)
	movq	%fs:(%r9), %rax
	leaq	736(%rsp), %rcx
	cmpq	%r15, %rbp
	leaq	1664(%rsp), %rdx
	movq	%r13, 360(%rsp)
	movq	$0, 736(%rsp)
	movq	%rcx, 800(%rsp)
	leaq	224(%rsp), %rcx
	movq	%r14, 808(%rsp)
	movq	%rdx, 816(%rsp)
	cmovbq	%rbp, %r15
	leaq	1528(%rsp), %rdx
	movq	%rcx, 824(%rsp)
	leaq	344(%rsp), %rcx
	movq	%rdx, 832(%rsp)
	movq	%rcx, 840(%rsp)
	testq	%rax, %rax
	je	.LBB361_387
	addq	$272, %rax
	jmp	.LBB361_388
.LBB361_387:
.Ltmp15911:
	movq	rayon_core::registry::global_registry@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15912:
.LBB361_388:
	movq	(%rax), %rax
	movq	520(%rax), %rdx
.Ltmp15913:
	movl	$1, %ecx
	movq	%rbx, (%rsp)
	movq	%r15, %rdi
	xorl	%esi, %esi
	xorl	%r8d, %r8d
	movq	%r15, %r9
	callq	rayon::iter::plumbing::bridge_producer_consumer::helper::<rayon::range::IterProducer<usize>, rayon::iter::for_each::ForEachConsumer<purrdf_sparql_eval::parallel::par_blocks_try_map_init<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#6}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#7}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#8}>::{closure#1}>>
.Ltmp15914:
	movq	344(%rsp), %r12
	movq	360(%rsp), %rdi
	movq	352(%rsp), %rbx
	movabsq	$2635249153387078803, %rsi
	imulq	$240, %r12, %r11
	imulq	$240, %rdi, %rcx
	movq	%rbx, %r14
	movq	%r11, %rdx
	shrq	$5, %rdx
	leaq	(%rbx,%rcx), %rax
	mulxq	%rsi, %r15, %r15
	movabsq	$-8608480567731124087, %rsi
	movq	%rbx, %rdx
	testq	%rdi, %rdi
	je	.LBB361_425
	addq	$-240, %rcx
	movq	%rcx, %rdx
	mulxq	%rsi, %rdx, %rdx
	shrl	$7, %edx
	incl	%edx
	andl	$7, %edx
	je	.LBB361_405
	imulq	$240, %rdx, %r8
	movq	%rbx, %rdi
	movq	%rbx, %rdx
	jmp	.LBB361_393
.LBB361_392:
	addq	$240, %rdi
	addq	$-240, %r8
	je	.LBB361_406
.LBB361_393:
	vmovdqu64	24(%rdi), %zmm0
	vmovdqu64	88(%rdi), %zmm1
	vmovdqu64	152(%rdi), %zmm2
	vmovdqu64	176(%rdi), %zmm3
	movq	16(%rdi), %r9
	vmovdqu64	%zmm3, 952(%rsp)
	vmovdqu64	%zmm2, 928(%rsp)
	vmovdqu64	%zmm1, 864(%rsp)
	vmovdqu64	%zmm0, 800(%rsp)
	cmpq	$2, %r9
	je	.LBB361_392
	movq	%r9, (%rdx)
	vmovdqu64	800(%rsp), %zmm0
	vmovdqu64	864(%rsp), %zmm1
	vmovdqu64	928(%rsp), %zmm2
	vmovdqu64	952(%rsp), %zmm3
	vmovdqu64	%zmm2, 136(%rdx)
	vmovdqu64	%zmm0, 8(%rdx)
	vmovdqu64	%zmm1, 72(%rdx)
	vmovdqu64	%zmm3, 160(%rdx)
	addq	$224, %rdx
	jmp	.LBB361_392
.LBB361_395:
	xorl	%ebp, %ebp
	jmp	.LBB361_397
.LBB361_396:
	movb	$1, %bpl
	movq	%r13, %rbx
.LBB361_397:
	movq	%rcx, %r15
.LBB361_398:
.Ltmp15982:
	leaq	384(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
.Ltmp15983:
	movq	224(%rsp), %r12
	movq	%rbx, 616(%rsp)
	testq	%rbx, %rbx
	je	.LBB361_402
	cmpq	$8, %rbx
	jae	.LBB361_403
	xorl	%eax, %eax
	xorl	%ebx, %ebx
	jmp	.LBB361_452
.LBB361_402:
	xorl	%ebx, %ebx
	jmp	.LBB361_454
.LBB361_403:
	cmpq	$32, %rbx
	jae	.LBB361_445
	xorl	%eax, %eax
	xorl	%ebx, %ebx
	jmp	.LBB361_449
.LBB361_405:
	movq	%rbx, %rdi
	movq	%rbx, %rdx
.LBB361_406:
	movq	%rax, %r14
	cmpq	$1680, %rcx
	jae	.LBB361_408
	jmp	.LBB361_425
.LBB361_407:
	addq	$1920, %rdi
	cmpq	%rax, %rdi
	je	.LBB361_424
.LBB361_408:
	vmovups	24(%rdi), %zmm0
	vmovups	88(%rdi), %zmm1
	vmovups	152(%rdi), %zmm2
	vmovups	176(%rdi), %zmm3
	movq	16(%rdi), %rcx
	vmovups	%zmm3, 952(%rsp)
	vmovups	%zmm2, 928(%rsp)
	vmovups	%zmm1, 864(%rsp)
	vmovups	%zmm0, 800(%rsp)
	cmpq	$2, %rcx
	je	.LBB361_410
	movq	%rcx, (%rdx)
	vmovups	800(%rsp), %zmm0
	vmovups	864(%rsp), %zmm1
	vmovups	928(%rsp), %zmm2
	vmovups	952(%rsp), %zmm3
	vmovups	%zmm2, 136(%rdx)
	vmovups	%zmm0, 8(%rdx)
	vmovups	%zmm1, 72(%rdx)
	vmovups	%zmm3, 160(%rdx)
	addq	$224, %rdx
.LBB361_410:
	vmovups	264(%rdi), %zmm0
	vmovups	328(%rdi), %zmm1
	vmovups	392(%rdi), %zmm2
	vmovups	416(%rdi), %zmm3
	movq	256(%rdi), %rcx
	vmovups	%zmm3, 952(%rsp)
	vmovups	%zmm2, 928(%rsp)
	vmovups	%zmm1, 864(%rsp)
	vmovups	%zmm0, 800(%rsp)
	cmpq	$2, %rcx
	je	.LBB361_412
	movq	%rcx, (%rdx)
	vmovups	800(%rsp), %zmm0
	vmovups	864(%rsp), %zmm1
	vmovups	928(%rsp), %zmm2
	vmovups	952(%rsp), %zmm3
	vmovups	%zmm2, 136(%rdx)
	vmovups	%zmm0, 8(%rdx)
	vmovups	%zmm1, 72(%rdx)
	vmovups	%zmm3, 160(%rdx)
	addq	$224, %rdx
.LBB361_412:
	vmovups	504(%rdi), %zmm0
	vmovups	568(%rdi), %zmm1
	vmovups	632(%rdi), %zmm2
	vmovups	656(%rdi), %zmm3
	movq	496(%rdi), %rcx
	vmovups	%zmm3, 952(%rsp)
	vmovups	%zmm2, 928(%rsp)
	vmovups	%zmm1, 864(%rsp)
	vmovups	%zmm0, 800(%rsp)
	cmpq	$2, %rcx
	je	.LBB361_414
	movq	%rcx, (%rdx)
	vmovups	800(%rsp), %zmm0
	vmovups	864(%rsp), %zmm1
	vmovups	928(%rsp), %zmm2
	vmovups	952(%rsp), %zmm3
	vmovups	%zmm2, 136(%rdx)
	vmovups	%zmm0, 8(%rdx)
	vmovups	%zmm1, 72(%rdx)
	vmovups	%zmm3, 160(%rdx)
	addq	$224, %rdx
.LBB361_414:
	vmovups	744(%rdi), %zmm0
	vmovups	808(%rdi), %zmm1
	vmovups	872(%rdi), %zmm2
	vmovups	896(%rdi), %zmm3
	movq	736(%rdi), %rcx
	vmovups	%zmm3, 952(%rsp)
	vmovups	%zmm2, 928(%rsp)
	vmovups	%zmm1, 864(%rsp)
	vmovups	%zmm0, 800(%rsp)
	cmpq	$2, %rcx
	je	.LBB361_416
	movq	%rcx, (%rdx)
	vmovups	800(%rsp), %zmm0
	vmovups	864(%rsp), %zmm1
	vmovups	928(%rsp), %zmm2
	vmovups	952(%rsp), %zmm3
	vmovups	%zmm2, 136(%rdx)
	vmovups	%zmm0, 8(%rdx)
	vmovups	%zmm1, 72(%rdx)
	vmovups	%zmm3, 160(%rdx)
	addq	$224, %rdx
.LBB361_416:
	vmovups	984(%rdi), %zmm0
	vmovups	1048(%rdi), %zmm1
	vmovups	1112(%rdi), %zmm2
	vmovups	1136(%rdi), %zmm3
	movq	976(%rdi), %rcx
	vmovups	%zmm3, 952(%rsp)
	vmovups	%zmm2, 928(%rsp)
	vmovups	%zmm1, 864(%rsp)
	vmovups	%zmm0, 800(%rsp)
	cmpq	$2, %rcx
	je	.LBB361_418
	movq	%rcx, (%rdx)
	vmovups	800(%rsp), %zmm0
	vmovups	864(%rsp), %zmm1
	vmovups	928(%rsp), %zmm2
	vmovups	952(%rsp), %zmm3
	vmovups	%zmm2, 136(%rdx)
	vmovups	%zmm0, 8(%rdx)
	vmovups	%zmm1, 72(%rdx)
	vmovups	%zmm3, 160(%rdx)
	addq	$224, %rdx
.LBB361_418:
	vmovups	1224(%rdi), %zmm0
	vmovups	1288(%rdi), %zmm1
	vmovups	1352(%rdi), %zmm2
	vmovups	1376(%rdi), %zmm3
	movq	1216(%rdi), %rcx
	vmovups	%zmm3, 952(%rsp)
	vmovups	%zmm2, 928(%rsp)
	vmovups	%zmm1, 864(%rsp)
	vmovups	%zmm0, 800(%rsp)
	cmpq	$2, %rcx
	je	.LBB361_420
	movq	%rcx, (%rdx)
	vmovups	800(%rsp), %zmm0
	vmovups	864(%rsp), %zmm1
	vmovups	928(%rsp), %zmm2
	vmovups	952(%rsp), %zmm3
	vmovups	%zmm2, 136(%rdx)
	vmovups	%zmm0, 8(%rdx)
	vmovups	%zmm1, 72(%rdx)
	vmovups	%zmm3, 160(%rdx)
	addq	$224, %rdx
.LBB361_420:
	vmovups	1464(%rdi), %zmm0
	vmovups	1528(%rdi), %zmm1
	vmovups	1592(%rdi), %zmm2
	vmovups	1616(%rdi), %zmm3
	movq	1456(%rdi), %rcx
	vmovups	%zmm3, 952(%rsp)
	vmovups	%zmm2, 928(%rsp)
	vmovups	%zmm1, 864(%rsp)
	vmovups	%zmm0, 800(%rsp)
	cmpq	$2, %rcx
	je	.LBB361_422
	movq	%rcx, (%rdx)
	vmovups	800(%rsp), %zmm0
	vmovups	864(%rsp), %zmm1
	vmovups	928(%rsp), %zmm2
	vmovups	952(%rsp), %zmm3
	vmovups	%zmm2, 136(%rdx)
	vmovups	%zmm0, 8(%rdx)
	vmovups	%zmm1, 72(%rdx)
	vmovups	%zmm3, 160(%rdx)
	addq	$224, %rdx
.LBB361_422:
	vmovdqu64	1704(%rdi), %zmm0
	vmovdqu64	1768(%rdi), %zmm1
	vmovdqu64	1832(%rdi), %zmm2
	vmovdqu64	1856(%rdi), %zmm3
	movq	1696(%rdi), %rcx
	vmovdqu64	%zmm3, 952(%rsp)
	vmovdqu64	%zmm2, 928(%rsp)
	vmovdqu64	%zmm1, 864(%rsp)
	vmovdqu64	%zmm0, 800(%rsp)
	cmpq	$2, %rcx
	je	.LBB361_407
	movq	%rcx, (%rdx)
	vmovdqu64	800(%rsp), %zmm0
	vmovdqu64	864(%rsp), %zmm1
	vmovdqu64	928(%rsp), %zmm2
	vmovdqu64	952(%rsp), %zmm3
	vmovdqu64	%zmm2, 136(%rdx)
	vmovdqu64	%zmm0, 8(%rdx)
	vmovdqu64	%zmm1, 72(%rdx)
	vmovdqu64	%zmm3, 160(%rdx)
	addq	$224, %rdx
	jmp	.LBB361_407
.LBB361_424:
	movq	%rax, %r14
.LBB361_425:
	vmovdqa	.LCPI361_0(%rip), %ymm0
	subq	%rbx, %rdx
	movabsq	$7905747460161236407, %rbp
	movq	%r11, 32(%rsp)
	movq	%rbx, %r13
	shrq	$5, %rdx
	imulq	%rdx, %rbp
	subq	%r14, %rax
	movq	%rax, %rdx
	mulxq	%rsi, %rax, %rax
	movq	%rbx, 640(%rsp)
	movq	%r12, %rbx
	movq	%rbp, 648(%rsp)
	movq	%r12, 656(%rsp)
	vmovdqu	%ymm0, 800(%rsp)
	je	.LBB361_430
	shrq	$7, %rax
	movl	$1, %r12d
	addq	$256, %r14
	subq	%rax, %r12
	jmp	.LBB361_428
	.p2align	4
.LBB361_427:
	addq	$240, %r14
	incq	%r12
	cmpq	$1, %r12
	je	.LBB361_430
.LBB361_428:
	cmpl	$2, -240(%r14)
	je	.LBB361_427
.Ltmp15919:
	leaq	-240(%r14), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>
.Ltmp15920:
	jmp	.LBB361_427
.LBB361_430:
	movq	32(%rsp), %rcx
	testq	%rbx, %rbx
	setne	%al
	imulq	$224, %r15, %r14
	cmpq	%r14, %rcx
	setne	%dl
	andb	%al, %dl
	cmpb	$1, %dl
	jne	.LBB361_436
	cmpq	$223, %rcx
	ja	.LBB361_435
	testq	%rcx, %rcx
	je	.LBB361_434
	movl	$16, %edx
	movq	%r13, %rdi
	movq	%rcx, %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB361_434:
	movl	$16, %r13d
	jmp	.LBB361_436
.LBB361_435:
	movq	<purrdf_alloc_probe::CountingAllocator as core::alloc::global::GlobalAlloc>::realloc@GOTPCREL(%rip), %rax
	leaq	qualification_454_native_cost::GLOBAL (.llvm.1577329767756756036)(%rip), %rdi
	movl	$16, %edx
	movq	%r13, %rsi
	movq	%r14, %r8
	vzeroupper
	callq	*%rax
	movq	%rax, %r13
	testq	%rax, %rax
	je	.LBB361_940
.LBB361_436:
	movq	%r15, 1360(%rsp)
	movq	%r13, 1368(%rsp)
	movq	%rbp, 1376(%rsp)
.Ltmp15933:
	leaq	800(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>>>
.Ltmp15934:
.Ltmp15935:
	leaq	384(%rsp), %rdi
	leaq	1360(%rsp), %rsi
	callq	purrdf_sparql_eval::parallel::concatenate_chunks::<purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)>
.Ltmp15936:
	movq	768(%rsp), %rsi
	movq	72(%rsp), %rbx
	testq	%rsi, %rsi
	je	.LBB361_338
	movq	776(%rsp), %rdi
	shlq	$4, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB361_338
.LBB361_440:
	movq	48(%rsp), %r15
.LBB361_441:
	movq	%r15, 232(%rsp)
.Ltmp15968:
	leaq	224(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15969:
	movq	72(%rsp), %rbx
	movq	640(%rsp), %rax
	movq	648(%rsp), %rcx
	movq	$-1, %r14
	xorl	%r13d, %r13d
	xorl	%r12d, %r12d
	movq	%rax, 144(%rsp)
	movq	%rcx, 96(%rsp)
.LBB361_443:
.Ltmp15971:
	leaq	800(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
.Ltmp15972:
	cmpq	$-1, %r14
	jne	.LBB361_466
	jmp	.LBB361_605
.LBB361_445:
	vmovdqa64	.LCPI361_1(%rip), %zmm1
	vpbroadcastq	.LCPI361_2(%rip), %zmm2
	vpbroadcastq	.LCPI361_3(%rip), %zmm3
	movq	%rbx, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
.LBB361_446:
	vpmullq	%zmm2, %zmm1, %zmm7
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm8, %xmm8, %xmm8
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$-32, %rcx
	vpgatherqq	64(%r15,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm0, %zmm8, %zmm0
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	1344(%r15,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm4, %zmm8, %zmm4
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	2624(%r15,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm5, %zmm8, %zmm5
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	3904(%r15,%zmm7), %zmm8 {%k1}
	vpaddq	%zmm6, %zmm8, %zmm6
	jne	.LBB361_446
	vpaddq	%zmm0, %zmm4, %zmm0
	movq	616(%rsp), %rcx
	vpaddq	%zmm0, %zmm5, %zmm0
	vpaddq	%zmm0, %zmm6, %zmm0
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rbx
	cmpq	%rax, %rcx
	je	.LBB361_454
	testb	$24, %cl
	je	.LBB361_452
.LBB361_449:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI361_1(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI361_2(%rip), %zmm2
	vpbroadcastq	.LCPI361_4(%rip), %zmm3
	movq	616(%rsp), %rax
	vmovq	%rbx, %xmm0
	andq	$-8, %rax
	subq	%rax, %rcx
.LBB361_450:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	64(%r15,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB361_450
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rbx
	cmpq	%rax, 616(%rsp)
	je	.LBB361_454
.LBB361_452:
	movq	616(%rsp), %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	64(%rax,%r15), %rax
	.p2align	4
.LBB361_453:
	addq	(%rax), %rbx
	addq	$160, %rax
	decq	%rcx
	jne	.LBB361_453
.LBB361_454:
	movq	616(%rsp), %rax
	movq	1488(%rsp), %r14
	movq	%r12, 1528(%rsp)
	movq	%r15, 1536(%rsp)
	movq	$0, 1184(%rsp)
	movq	$8, 1192(%rsp)
	movq	$0, 1200(%rsp)
	movq	%rax, 1544(%rsp)
	movb	%bpl, 1552(%rsp)
.Ltmp15993:
	leaq	384(%rsp), %rdi
	leaq	1184(%rsp), %rsi
	movq	%r14, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp15994:
	movq	384(%rsp), %rcx
	movq	392(%rsp), %rax
	movq	400(%rsp), %rdi
	movq	408(%rsp), %rsi
	movq	%rbx, 1888(%rsp)
	movq	%r15, 1080(%rsp)
	movq	%r12, 136(%rsp)
	movq	%rcx, 48(%rsp)
	cmpq	$-1, %rcx
	je	.LBB361_559
	movq	%rax, 144(%rsp)
	movzbl	416(%rsp), %eax
	vmovdqu	432(%rsp), %ymm0
	vmovdqu	448(%rsp), %ymm1
	movl	%ebp, %r14d
	movzbl	423(%rsp), %ebp
	movzwl	421(%rsp), %ebx
	movl	417(%rsp), %r13d
	movq	%rdi, 96(%rsp)
	movq	%rsi, 32(%rsp)
	movq	%rax, 64(%rsp)
	movq	424(%rsp), %rax
	vmovdqu	%ymm0, 1360(%rsp)
	vmovdqu	%ymm1, 1376(%rsp)
	movq	%rax, 88(%rsp)
.Ltmp15998:
	leaq	1416(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15999:
	shll	$16, %ebp
	orl	%ebp, %ebx
	movl	%r14d, %ebp
	shlq	$32, %rbx
	orq	%rbx, %r13
	movq	72(%rsp), %rbx
.LBB361_458:
	movq	616(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_462
	movq	1080(%rsp), %r14
	movl	$1, %r15d
	subq	%rax, %r15
	.p2align	4
.LBB361_460:
.Ltmp16103:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp16104:
	incq	%r15
	addq	$160, %r14
	cmpq	$1, %r15
	jne	.LBB361_460
.LBB361_462:
	testq	%r12, %r12
	je	.LBB361_464
	movq	1080(%rsp), %rdi
	shlq	$5, %r12
	movl	$8, %edx
	leaq	(%r12,%r12,4), %rsi
	callq	__rustc::__rust_dealloc
.LBB361_464:
	movq	48(%rsp), %r14
	cmpq	$-1, %r14
	je	.LBB361_565
	vmovdqu	1376(%rsp), %ymm1
	vmovdqu	1360(%rsp), %ymm0
	movq	88(%rsp), %r15
	movq	32(%rsp), %rbp
	movq	64(%rsp), %r12
	vmovdqu	%ymm1, 1680(%rsp)
	vmovdqu	%ymm0, 1664(%rsp)
.LBB361_466:
	vmovdqu	1680(%rsp), %ymm1
	vmovdqu	1664(%rsp), %ymm0
	movq	144(%rsp), %rcx
	movq	96(%rsp), %rdx
	shlq	$8, %r13
	movzbl	%r12b, %eax
	orq	%r13, %rax
	vmovdqu	%ymm1, 80(%rbx)
	vmovdqu	%ymm0, 64(%rbx)
	movq	%r14, 16(%rbx)
	movq	%rcx, 24(%rbx)
	movq	%rdx, 32(%rbx)
	movq	%rbp, 40(%rbx)
	movq	%rax, 48(%rbx)
.LBB361_467:
	movq	%r15, 56(%rbx)
	movq	$1, (%rbx)
.Ltmp16117:
	leaq	1936(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp16118:
	movq	624(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_471
.LBB361_469:
	lock		decq	(%rax)
	jne	.LBB361_471
	#MEMBARRIER
.Ltmp16161:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	624(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16162:
.LBB361_471:
.Ltmp16166:
	leaq	1968(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp16167:
.LBB361_472:
	movb	$1, %bl
	movq	1104(%rsp), %r14
	movq	1112(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_476
.LBB361_473:
	movl	$1, %r12d
	movq	%r14, %r15
	subq	%rax, %r12
	.p2align	4
.LBB361_474:
.Ltmp16171:
	movq	%r15, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
.Ltmp16172:
	incq	%r12
	addq	$24, %r15
	cmpq	$1, %r12
	jne	.LBB361_474
.LBB361_476:
	movq	1096(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_486
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,2), %rcx
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
	jge	.LBB361_479
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_479:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_485
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_479
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
.LBB361_482:
	cmpq	%rax, %rdx
	jge	.LBB361_484
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB361_482
.LBB361_484:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_485:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.LBB361_486:
	movl	%ebx, 32(%rsp)
	movq	1048(%rsp), %rbx
	movq	1056(%rsp), %r14
	testq	%r14, %r14
	je	.LBB361_509
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	xorl	%r15d, %r15d
	jmp	.LBB361_491
	.p2align	4
.LBB361_488:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_489:
	vzeroupper
	callq	*%r13
.LBB361_490:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB361_509
.LBB361_491:
	leaq	(%r15,%r15,8), %rax
	leaq	(%rbx,%rax,8), %r12
	movq	(%rbx,%rax,8), %rax
	cmpq	$6, %rax
	jb	.LBB361_501
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	8(%r12), %rdi
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
	jge	.LBB361_494
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_494:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_500
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_494
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
.LBB361_497:
	cmpq	%rax, %rdx
	jge	.LBB361_499
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB361_497
.LBB361_499:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_500:
	vzeroupper
	callq	*%r13
.LBB361_501:
	movq	48(%r12), %rcx
	testq	%rcx, %rcx
	je	.LBB361_490
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$3, %rcx
	movabsq	$9223372036854775807, %rsi
	movq	56(%r12), %rdi
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
	jge	.LBB361_504
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_504:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_489
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_504
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
.LBB361_507:
	cmpq	%rax, %rdx
	jge	.LBB361_488
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB361_507
	jmp	.LBB361_488
.LBB361_509:
	movq	1040(%rsp), %rax
	movl	32(%rsp), %r15d
	testq	%rax, %rax
	je	.LBB361_519
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,8), %rcx
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
	jge	.LBB361_512
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_512:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_518
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_512
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
.LBB361_515:
	cmpq	%rax, %rdx
	jge	.LBB361_517
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB361_515
.LBB361_517:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_518:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB361_519:
	cmpq	$0, 720(%rsp)
	movq	72(%rsp), %rbx
	je	.LBB361_529
	movq	712(%rsp), %rdi
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	cmpq	%rdx, %rdi
	cmovaeq	%rdx, %rdi
	xorl	%ecx, %ecx
	cmpq	%rdi, %rax
	movq	%rdi, %rsi
	setns	%cl
	addq	%rdx, %rcx
	subq	%rdi, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB361_522
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_522:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_528
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_522
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rsi, %rcx
	negq	%rcx
	movabsq	$9223372036854775807, %rdx
	lock		xaddq	%rcx, (%rax)
	xorl	%eax, %eax
	cmpq	%rsi, %rcx
	setns	%al
	addq	%rdx, %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	subq	%rsi, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB361_525:
	cmpq	%rax, %rcx
	jge	.LBB361_527
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB361_525
.LBB361_527:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_528:
	movq	free@GOTPCREL(%rip), %rax
	movq	192(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB361_529:
	movq	184(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB361_531
	movb	$1, %bpl
	#MEMBARRIER
.Ltmp16177:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	184(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16178:
.LBB361_531:
.Ltmp16180:
	leaq	1120(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp16181:
	movq	1816(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB361_542
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	1824(%rsp), %rdi
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
	jge	.LBB361_535
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_535:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_541
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_535
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
.LBB361_538:
	cmpq	%rax, %rdx
	jge	.LBB361_540
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB361_538
.LBB361_540:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_541:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_542:
	movq	1744(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB361_552
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	1752(%rsp), %rdi
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
	jge	.LBB361_545
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_545:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_551
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_545
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
.LBB361_548:
	cmpq	%rax, %rdx
	jge	.LBB361_550
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB361_548
.LBB361_550:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_551:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_552:
	movq	1840(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_555
	lock		decq	(%rax)
	jne	.LBB361_555
	leaq	1840(%rsp), %rdi
	#MEMBARRIER
.Ltmp16183:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp16184:
.LBB361_555:
	testb	%r15b, %r15b
	je	.LBB361_558
.LBB361_556:
	movq	328(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB361_558
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1520(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB361_558:
	movq	%rbx, %rax
	addq	$2456, %rsp
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
.LBB361_559:
	.cfi_def_cfa_offset 2512
	movq	%rax, 200(%rsp)
	movq	1480(%rsp), %rcx
	movq	1472(%rsp), %rax
	leaq	(%r14,%r14,4), %rdx
	movq	%rdi, 208(%rsp)
	movq	%rsi, 216(%rsp)
	movl	%ebp, 632(%rsp)
	movq	%rcx, 736(%rsp)
	movq	%rax, 752(%rsp)
	leaq	(%rcx,%rdx,8), %rdx
	movq	%rcx, 744(%rsp)
	movq	2520(%rsp), %rcx
	movq	%rdx, 760(%rsp)
	movq	616(%rcx), %rax
	movq	%rax, 1504(%rsp)
	testq	%rax, %rax
	je	.LBB361_566
	lock		incq	(%rax)
	movq	616(%rsp), %rbx
	jle	.LBB361_950
	movq	2520(%rsp), %rax
	movq	616(%rax), %r13
	movq	%r13, 1176(%rsp)
	movq	%r13, 1032(%rsp)
	movq	16(%r13), %rax
	movq	40(%r13), %rcx
	movq	%rax, 1024(%rsp)
	movq	%rcx, 376(%rsp)
	cmpq	$-1, %rcx
	je	.LBB361_578
	testq	%rbx, %rbx
	je	.LBB361_588
	cmpq	$8, %rbx
	jae	.LBB361_686
	xorl	%eax, %eax
	xorl	%r14d, %r14d
	jmp	.LBB361_698
.LBB361_565:
	movq	64(%rsp), %rsi
	jmp	.LBB361_594
.LBB361_566:
	movq	744(%rsp), %rcx
	movq	736(%rsp), %rax
	movq	752(%rsp), %rdx
	movq	%rsi, 32(%rsp)
	movq	%rcx, 232(%rsp)
	movq	760(%rsp), %rcx
	movq	%rax, 224(%rsp)
	movq	%rdx, 240(%rsp)
	movq	%rcx, 248(%rsp)
	movq	248(%rsp), %rax
	movq	232(%rsp), %r14
	movq	%rax, 56(%rsp)
	cmpq	%rax, %r14
	je	.LBB361_576
	movq	32(%rsp), %rax
	movq	%rdi, %rbx
	leaq	(,%rax,8), %rax
	leaq	(%rax,%rax,4), %rbp
	jmp	.LBB361_569
.LBB361_568:
	movq	144(%rsp), %rax
	shll	$16, %r12d
	movq	64(%rsp), %rdx
	addq	$40, %r14
	orl	%r12d, %r15d
	shlq	$32, %r15
	orq	%r15, %rbx
	movq	%rax, (%rcx,%rbp)
	movq	48(%rsp), %rax
	movq	%rax, 8(%rcx,%rbp)
	movzbl	88(%rsp), %eax
	movq	%r13, 16(%rcx,%rbp)
	movb	%al, 24(%rcx,%rbp)
	movq	%rbx, %rax
	shrq	$48, %rax
	movl	%ebx, 25(%rcx,%rbp)
	shrq	$32, %rbx
	movb	%al, 31(%rcx,%rbp)
	movq	32(%rsp), %rax
	movw	%bx, 29(%rcx,%rbp)
	movq	%rdx, 32(%rcx,%rbp)
	addq	$40, %rbp
	movq	%rcx, %rbx
	incq	%rax
	movq	%rax, 32(%rsp)
	movq	%rax, 216(%rsp)
	cmpq	56(%rsp), %r14
	je	.LBB361_576
.LBB361_569:
	movq	32(%r14), %rax
	leaq	1192(%rsp), %rcx
	movq	%rax, 32(%rcx)
	movq	2520(%rsp), %rax
	vmovdqu	(%r14), %ymm0
	vmovdqu	%ymm0, (%rcx)
	movq	%rax, 1184(%rsp)
	cmpq	$0, 1192(%rsp)
	je	.LBB361_571
	movq	32(%r14), %rax
	leaq	392(%rsp), %rcx
	movq	%rax, 32(%rcx)
	vmovdqu	(%r14), %ymm0
	vmovdqu	%ymm0, (%rcx)
	jmp	.LBB361_573
.LBB361_571:
	movq	664(%rax), %rdx
.Ltmp16082:
	movq	96(%rsp), %rsi
	leaq	384(%rsp), %rdi
	leaq	1200(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp16083:
	movq	384(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB361_689
.LBB361_573:
	movq	400(%rsp), %rdx
	movq	%rbx, %rcx
	movq	392(%rsp), %rax
	movzbl	416(%rsp), %edi
	movq	424(%rsp), %rsi
	movq	408(%rsp), %r13
	movzbl	423(%rsp), %r12d
	movzwl	421(%rsp), %r15d
	movl	417(%rsp), %ebx
	movq	%rdx, 48(%rsp)
	movq	32(%rsp), %rdx
	movq	%rax, 144(%rsp)
	movb	%dil, 88(%rsp)
	movq	%rsi, 64(%rsp)
	cmpq	200(%rsp), %rdx
	jne	.LBB361_568
.Ltmp16090:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	200(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16091:
	movq	208(%rsp), %rcx
	jmp	.LBB361_568
.LBB361_576:
	movq	%r14, 232(%rsp)
	movb	$1, %r14b
.Ltmp16095:
	leaq	224(%rsp), %rdi
	movb	$1, %r15b
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16096:
	movq	136(%rsp), %r12
	movq	200(%rsp), %rax
	movq	208(%rsp), %rcx
	movq	72(%rsp), %rbx
	movl	632(%rsp), %ebp
	movq	$-1, 48(%rsp)
	movq	%rax, 144(%rsp)
	movb	$2, %al
	movq	%rcx, 96(%rsp)
	movq	%rax, 64(%rsp)
	jmp	.LBB361_458
.LBB361_578:
	leaq	(%rbx,%rbx,4), %rax
	movq	%r15, 768(%rsp)
	movq	%r15, 776(%rsp)
	movq	%r12, 784(%rsp)
	shlq	$5, %rax
	addq	%r15, %rax
	movq	%rax, 1496(%rsp)
	movq	%rax, 792(%rsp)
	testq	%rbx, %rbx
	jne	.LBB361_703
	jmp	.LBB361_589
.LBB361_579:
	movq	392(%rsp), %rax
	vmovdqu	432(%rsp), %ymm0
	vmovdqu	448(%rsp), %ymm1
	movq	%r13, 48(%rsp)
	movq	416(%rsp), %r12
	movq	424(%rsp), %r13
	movq	%r15, 232(%rsp)
	movq	%rax, 144(%rsp)
	movq	400(%rsp), %rax
	vmovdqu	%ymm0, 1184(%rsp)
	vmovdqu	%ymm1, 1200(%rsp)
	movq	%rax, 96(%rsp)
	movq	408(%rsp), %rax
	movq	%rax, 32(%rsp)
.Ltmp15960:
	leaq	224(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15961:
	movq	%r14, %rdi
	testq	%rbp, %rbp
	je	.LBB361_585
	leaq	8(%rdi), %rbx
	xorl	%r15d, %r15d
	jmp	.LBB361_583
.LBB361_582:
	incq	%r15
	addq	$40, %rbx
	cmpq	%r15, %rbp
	je	.LBB361_585
.LBB361_583:
	movq	-8(%rbx), %rax
	cmpq	$6, %rax
	jb	.LBB361_582
	movq	(%rbx), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	movq	%r14, %rdi
	jmp	.LBB361_582
.LBB361_585:
	movq	640(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_587
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
.LBB361_587:
	movq	72(%rsp), %rbx
	movq	32(%rsp), %rbp
	movq	48(%rsp), %r14
	movq	%r13, %r15
	jmp	.LBB361_357
.LBB361_588:
	movq	%r15, 768(%rsp)
	movq	%r15, 776(%rsp)
	movq	%r12, 784(%rsp)
	movq	%r15, 792(%rsp)
.LBB361_589:
	movb	$1, %al
	xorl	%ebp, %ebp
	movl	%eax, 56(%rsp)
.Ltmp16073:
	leaq	768(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp16074:
	movq	208(%rsp), %rcx
	movq	200(%rsp), %rax
	movq	216(%rsp), %rdx
	movq	%rcx, 96(%rsp)
	movq	1032(%rsp), %rcx
	movq	%rax, 144(%rsp)
	movq	%rdx, 32(%rsp)
	lock		decq	(%rcx)
	jne	.LBB361_592
	xorl	%r14d, %r14d
	#MEMBARRIER
.Ltmp16078:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1176(%rsp), %rdi
	xorl	%r15d, %r15d
	callq	*%rax
.Ltmp16079:
.LBB361_592:
	xorl	%r14d, %r14d
.Ltmp16080:
	leaq	736(%rsp), %rdi
	xorl	%r15d, %r15d
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16081:
	movq	72(%rsp), %rbx
	movl	632(%rsp), %ebp
	movb	$2, %sil
.LBB361_594:
	movq	144(%rsp), %rax
	movq	96(%rsp), %rcx
	movq	32(%rsp), %rdx
	movq	%rax, 1184(%rsp)
	movq	%rcx, 1192(%rsp)
	movq	2520(%rsp), %rcx
	movq	%rdx, 1200(%rsp)
	movq	616(%rcx), %rax
	testq	%rax, %rax
	je	.LBB361_603
	movl	296(%rax), %ecx
	movl	%ebp, %edx
	movb	$-1, %bpl
	testl	%ecx, %ecx
	jne	.LBB361_597
	movq	288(%rax), %rcx
	movzbl	272(%rax), %ebp
	movq	%rcx, 239(%rsp)
	vmovdqu	273(%rax), %xmm0
	vmovdqa	%xmm0, 224(%rsp)
.LBB361_597:
	testb	%sil, %sil
	je	.LBB361_602
	movzbl	%sil, %eax
	cmpl	$2, %eax
	je	.LBB361_604
	cmpb	$-1, %bpl
	je	.LBB361_688
	movq	2520(%rsp), %rax
	cmpb	$2, 472(%rax)
	jne	.LBB361_602
	vmovdqa	224(%rsp), %xmm0
	movq	696(%rax), %rdi
	movq	239(%rsp), %rax
	movb	%bpl, 384(%rsp)
	vmovdqu	%xmm0, 385(%rsp)
	movq	%rax, 400(%rsp)
	movl	40(%rdi), %eax
	testl	%eax, %eax
	jne	.LBB361_946
.LBB361_602:
	xorl	%edx, %edx
	jmp	.LBB361_604
.LBB361_603:
	cmpb	$2, %sil
	sete	%al
	andb	%bpl, %al
	movb	$-1, %bpl
	movl	%eax, %edx
.LBB361_604:
	cmpb	$-1, %bpl
	movq	1888(%rsp), %r15
	movq	32(%rsp), %rbp
	sete	%r12b
	andb	%dl, %r12b
.LBB361_605:
	movzbl	%r12b, %r14d
.LBB361_606:
	vmovdqu	1936(%rsp), %ymm0
	movq	144(%rsp), %rax
	movq	96(%rsp), %rcx
	movq	%rax, 800(%rsp)
	movq	%rcx, 808(%rsp)
	movq	%rbp, 816(%rsp)
	vmovdqu	%ymm0, 384(%rsp)
.Ltmp16120:
	movq	2520(%rsp), %rdi
	leaq	384(%rsp), %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::absorb_worker_witnesses::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp16121:
	testb	$1, %r14b
	je	.LBB361_610
	movq	1056(%rsp), %rdx
	cmpq	%rdx, %r15
	ja	.LBB361_938
	jne	.LBB361_662
.LBB361_610:
	movq	816(%rsp), %rax
	vmovdqu	800(%rsp), %xmm0
	movq	%rax, 1456(%rsp)
	movq	624(%rsp), %rax
	vmovdqa	%xmm0, 1440(%rsp)
	testq	%rax, %rax
	je	.LBB361_613
	lock		decq	(%rax)
	jne	.LBB361_613
	#MEMBARRIER
.Ltmp16138:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	624(%rsp), %rdi
	callq	*%rax
.Ltmp16139:
.LBB361_613:
.Ltmp16140:
	leaq	1968(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp16141:
.LBB361_614:
	movq	2520(%rsp), %rax
	movq	696(%rax), %rax
	movl	40(%rax), %ecx
	testl	%ecx, %ecx
	je	.LBB361_617
.LBB361_615:
	vmovdqa	1440(%rsp), %xmm0
	movq	1456(%rsp), %rax
	cmpq	$-1, 1744(%rsp)
	movq	328(%rsp), %rcx
	movq	%rax, 816(%rsp)
	vmovdqa	%xmm0, 800(%rsp)
	movq	%rcx, 824(%rsp)
	je	.LBB361_620
	leaq	384(%rsp), %rdi
	leaq	800(%rsp), %rsi
	leaq	1744(%rsp), %rdx
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	jmp	.LBB361_621
.LBB361_617:
	movzbl	16(%rax), %ecx
	cmpb	$-1, %cl
	je	.LBB361_615
	movb	%cl, 800(%rsp)
	vmovdqu	17(%rax), %xmm0
	vmovdqu	%xmm0, 801(%rsp)
	movq	32(%rax), %rax
	movq	%rax, 816(%rsp)
.Ltmp16142:
	movq	1160(%rsp), %rsi
	movq	328(%rsp), %rcx
	leaq	384(%rsp), %rdi
	leaq	800(%rsp), %rdx
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
.Ltmp16143:
	vmovdqu64	416(%rsp), %zmm1
	vmovdqu64	384(%rsp), %zmm0
	leaq	1440(%rsp), %rdi
	vmovdqu64	%zmm1, 40(%rbx)
	vmovdqu64	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	xorl	%ebx, %ebx
	movq	1104(%rsp), %r14
	movq	1112(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB361_473
	jmp	.LBB361_476
.LBB361_620:
	vmovdqu	800(%rsp), %xmm0
	movq	816(%rsp), %rax
	movq	824(%rsp), %rcx
	movq	%rax, 408(%rsp)
	movq	%rcx, 416(%rsp)
	vmovdqu	%xmm0, 392(%rsp)
	movq	$-1, 384(%rsp)
.LBB361_621:
	movq	1816(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB361_623
	movq	1824(%rsp), %rdi
	leaq	-3(%rax,%rax,2), %rsi
	movl	$1, %edx
	callq	__rustc::__rust_dealloc
.LBB361_623:
	movq	1840(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_626
	lock		decq	(%rax)
	jne	.LBB361_626
	leaq	1840(%rsp), %rdi
	#MEMBARRIER
.Ltmp16145:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp16146:
.LBB361_626:
	vmovdqu64	416(%rsp), %zmm1
	vmovdqu64	384(%rsp), %zmm0
	movq	1104(%rsp), %r14
	movq	1112(%rsp), %rax
	vmovdqu64	%zmm1, 40(%rbx)
	vmovdqu64	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
	testq	%rax, %rax
	je	.LBB361_630
	movl	$1, %r12d
	movq	%r14, %r15
	subq	%rax, %r12
	.p2align	4
.LBB361_628:
.Ltmp16148:
	movq	%r15, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
.Ltmp16149:
	incq	%r12
	addq	$24, %r15
	cmpq	$1, %r12
	jne	.LBB361_628
.LBB361_630:
	movq	1096(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_632
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r14, %rdi
	leaq	(%rax,%rax,2), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB361_632:
	movq	1048(%rsp), %rbx
	movq	1056(%rsp), %r14
	testq	%r14, %r14
	je	.LBB361_655
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	xorl	%r15d, %r15d
	jmp	.LBB361_637
	.p2align	4
.LBB361_634:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_635:
	vzeroupper
	callq	*%rbp
.LBB361_636:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB361_655
.LBB361_637:
	leaq	(%r15,%r15,8), %rax
	leaq	(%rbx,%rax,8), %r12
	movq	(%rbx,%rax,8), %rax
	cmpq	$6, %rax
	jb	.LBB361_647
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	8(%r12), %rdi
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
	jge	.LBB361_640
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_640:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_646
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_640
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
	movq	(%r13), %rax
	.p2align	4
.LBB361_643:
	cmpq	%rax, %rdx
	jge	.LBB361_645
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB361_643
.LBB361_645:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_646:
	vzeroupper
	callq	*%rbp
.LBB361_647:
	movq	48(%r12), %rcx
	testq	%rcx, %rcx
	je	.LBB361_636
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$3, %rcx
	movabsq	$9223372036854775807, %rsi
	movq	56(%r12), %rdi
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
	jge	.LBB361_650
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_650:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_635
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_650
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
	movq	(%r13), %rax
	.p2align	4
.LBB361_653:
	cmpq	%rax, %rdx
	jge	.LBB361_634
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB361_653
	jmp	.LBB361_634
.LBB361_655:
	movq	1040(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_657
	shlq	$3, %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	leaq	(%rax,%rax,8), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB361_657:
	cmpq	$0, 720(%rsp)
	je	.LBB361_659
	movq	192(%rsp), %rdi
	movq	712(%rsp), %rsi
	movl	$8, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB361_659:
	movq	184(%rsp), %rax
	lock		decq	(%rax)
	movq	72(%rsp), %rbx
	jne	.LBB361_661
	xorl	%ebp, %ebp
	#MEMBARRIER
.Ltmp16154:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	184(%rsp), %rdi
	xorl	%r15d, %r15d
	vzeroupper
	callq	*%rax
.Ltmp16155:
.LBB361_661:
	leaq	1120(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
	jmp	.LBB361_558
.LBB361_662:
	movq	1048(%rsp), %rax
	leaq	(%rdx,%rdx,8), %rcx
	addq	$16, 336(%rsp)
	leaq	1560(%rsp), %r14
	leaq	(%rax,%rcx,8), %rcx
	movq	%rcx, 32(%rsp)
	leaq	(%r15,%r15,8), %rcx
	leaq	(%rax,%rcx,8), %rbp
.LBB361_663:
	movq	1168(%rsp), %rsi
.Ltmp16122:
	movq	%r14, %rdi
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::from_elem
.Ltmp16123:
	movq	1560(%rsp), %r13
	leaq	1568(%rsp), %rdi
	movq	%r13, %rax
	cmpq	$6, %r13
	jb	.LBB361_666
	movq	1568(%rsp), %rdi
	movq	1576(%rsp), %rax
.LBB361_666:
	movq	728(%rsp), %rdx
	decq	%rax
	cmpq	%rax, %rdx
	ja	.LBB361_935
	movq	(%rbp), %rsi
	decq	%rsi
	cmpq	$4, %rsi
	jbe	.LBB361_669
	movq	16(%rbp), %rsi
	movq	8(%rbp), %rax
	decq	%rsi
	jmp	.LBB361_670
.LBB361_669:
	leaq	8(%rbp), %rax
.LBB361_670:
	cmpq	%rsi, %rdx
	jne	.LBB361_936
	movq	memcpy@GOTPCREL(%rip), %r14
	shlq	$3, %rdx
	movq	%rax, %rsi
	callq	*%r14
	movq	1112(%rsp), %rbx
	movq	2512(%rsp), %rax
	cmpq	%rbx, %rax
	cmovbq	%rax, %rbx
	testq	%rbx, %rbx
	je	.LBB361_681
	movq	1104(%rsp), %r15
	movq	336(%rsp), %r12
	xorl	%r14d, %r14d
	addq	$16, %r15
	jmp	.LBB361_674
.LBB361_673:
	incq	%r14
	addq	$24, %r15
	addq	$120, %r12
	vmovq	%xmm0, (%rax,%rdi,8)
	cmpq	%r14, %rbx
	je	.LBB361_681
.LBB361_674:
	vmovups	1128(%rsp), %xmm0
	movq	184(%rsp), %rax
	movq	-8(%r15), %rdx
	movq	(%r15), %rcx
	movq	56(%rbp), %r8
	movq	64(%rbp), %r9
	addq	$16, %rax
.Ltmp16127:
	movq	2520(%rsp), %rsi
	leaq	384(%rsp), %rdi
	movq	%rax, 16(%rsp)
	movq	%rsi, 24(%rsp)
	movq	%r12, %rsi
	vmovups	%xmm0, (%rsp)
	callq	purrdf_sparql_eval::modifier::eval_aggregate::<purrdf_core::ir::dataset::RdfDataset, ()>
.Ltmp16128:
	vmovq	392(%rsp), %xmm0
	movq	384(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB361_683
	movq	1560(%rsp), %r13
	movq	%r13, %rsi
	cmpq	$6, %r13
	jb	.LBB361_678
	movq	1576(%rsp), %rsi
.LBB361_678:
	movq	728(%rsp), %rdi
	decq	%rsi
	addq	%r14, %rdi
	cmpq	%rsi, %rdi
	jae	.LBB361_947
	leaq	1568(%rsp), %rax
	cmpq	$6, %r13
	jb	.LBB361_673
	movq	1568(%rsp), %rax
	jmp	.LBB361_673
.LBB361_681:
.Ltmp16132:
	leaq	1560(%rsp), %r14
	leaq	800(%rsp), %rdi
	movq	%r14, %rsi
	callq	<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::push_mut
.Ltmp16133:
	movq	72(%rsp), %rbx
	addq	$72, %rbp
	cmpq	32(%rsp), %rbp
	jne	.LBB361_663
	jmp	.LBB361_610
.LBB361_683:
	vmovups	400(%rsp), %zmm1
	vmovups	416(%rsp), %zmm2
	movq	72(%rsp), %rcx
	vmovups	%zmm2, 48(%rcx)
	vmovups	%zmm1, 32(%rcx)
	movq	%rax, 16(%rcx)
	movq	1560(%rsp), %rax
	vmovq	%xmm0, 24(%rcx)
	movq	$1, (%rcx)
	cmpq	$6, %rax
	jb	.LBB361_685
	movq	1568(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB361_685:
	leaq	800(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	624(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB361_469
	jmp	.LBB361_471
.LBB361_686:
	cmpq	$32, %rbx
	jae	.LBB361_691
	xorl	%eax, %eax
	xorl	%r14d, %r14d
	jmp	.LBB361_695
.LBB361_688:
	movb	$-1, %bpl
	xorl	%edx, %edx
	jmp	.LBB361_604
.LBB361_689:
	vmovups	432(%rsp), %ymm0
	movq	392(%rsp), %rdx
	movq	400(%rsp), %rcx
	movq	%rax, 48(%rsp)
	movzbl	416(%rsp), %eax
	movzbl	423(%rsp), %ebp
	movzwl	421(%rsp), %ebx
	movl	417(%rsp), %r13d
	addq	$40, %r14
	movq	%r14, 232(%rsp)
	movb	$1, %r14b
	movq	%rdx, 144(%rsp)
	movq	408(%rsp), %rdx
	movq	%rcx, 96(%rsp)
	movq	424(%rsp), %rcx
	movq	%rax, 64(%rsp)
	vmovups	%ymm0, 1360(%rsp)
	vmovdqu	448(%rsp), %ymm0
	movq	%rdx, 32(%rsp)
	movq	%rcx, 88(%rsp)
	vmovdqu	%ymm0, 1376(%rsp)
.Ltmp16085:
	leaq	224(%rsp), %rdi
	movb	$1, %r15b
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16086:
	movq	136(%rsp), %r12
	shll	$16, %ebp
	movb	$1, %r14b
	movb	$1, %r15b
	orl	%ebp, %ebx
	shlq	$32, %rbx
	orq	%rbx, %r13
	jmp	.LBB361_923
.LBB361_691:
	vmovdqa64	.LCPI361_1(%rip), %zmm1
	vpbroadcastq	.LCPI361_2(%rip), %zmm2
	vpbroadcastq	.LCPI361_3(%rip), %zmm3
	movq	%rbx, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
.LBB361_692:
	vpmullq	%zmm2, %zmm1, %zmm7
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm8, %xmm8, %xmm8
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$-32, %rcx
	vpgatherqq	16(%r15,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm0, %zmm8, %zmm0
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	1296(%r15,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm4, %zmm8, %zmm4
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	2576(%r15,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm5, %zmm8, %zmm5
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	3856(%r15,%zmm7), %zmm8 {%k1}
	vpaddq	%zmm6, %zmm8, %zmm6
	jne	.LBB361_692
	vpaddq	%zmm0, %zmm4, %zmm0
	vpaddq	%zmm0, %zmm5, %zmm0
	vpaddq	%zmm0, %zmm6, %zmm0
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %r14
	cmpq	%rax, %rbx
	je	.LBB361_700
	testb	$24, %bl
	je	.LBB361_698
.LBB361_695:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI361_1(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI361_2(%rip), %zmm2
	vpbroadcastq	.LCPI361_4(%rip), %zmm3
	movq	%rbx, %rax
	andq	$-8, %rax
	vmovq	%r14, %xmm0
	subq	%rax, %rcx
.LBB361_696:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	16(%r15,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB361_696
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %r14
	cmpq	%rax, %rbx
	je	.LBB361_700
.LBB361_698:
	movq	%rbx, %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	16(%rax,%r15), %rax
.LBB361_699:
	addq	(%rax), %r14
	addq	$160, %rax
	decq	%rcx
	jne	.LBB361_699
.LBB361_700:
	movq	2520(%rsp), %rcx
	movq	912(%rcx), %rax
	movq	928(%rcx), %rsi
	leaq	912(%rcx), %r15
	subq	%rsi, %rax
	cmpq	%rax, %r14
	ja	.LBB361_942
.LBB361_701:
	movq	2520(%rsp), %rax
	cmpq	1016(%rax), %r14
	ja	.LBB361_943
.LBB361_702:
	movq	1080(%rsp), %r15
	leaq	(%rbx,%rbx,4), %rax
	movq	1032(%rsp), %r13
	shlq	$5, %rax
	addq	%r15, %rax
	movq	%r15, 768(%rsp)
	movq	%r15, 776(%rsp)
	movq	%r12, 784(%rsp)
	movq	%rax, 1496(%rsp)
	movq	%rax, 792(%rsp)
.LBB361_703:
	leaq	16(%r13), %rax
	leaq	272(%r13), %rcx
	movq	%rax, 320(%rsp)
	movq	%rcx, 1152(%rsp)
.LBB361_704:
	leaq	160(%r15), %rdx
	movq	%rdx, 776(%rsp)
	vmovups	96(%r15), %zmm0
	movq	(%r15), %rax
	vmovups	%zmm0, 1272(%rsp)
	vmovups	72(%r15), %zmm0
	vmovups	%zmm0, 1248(%rsp)
	vmovups	8(%r15), %zmm0
	vmovups	%zmm0, 1184(%rsp)
	cmpq	$-1, %rax
	je	.LBB361_589
	vmovdqu64	1184(%rsp), %zmm0
	vmovdqu64	1248(%rsp), %zmm1
	vmovdqu64	1272(%rsp), %zmm2
	leaq	392(%rsp), %rcx
	movq	%rax, 384(%rsp)
	movq	%rdx, 1848(%rsp)
	vmovdqu64	%zmm2, 88(%rcx)
	vmovdqu64	%zmm1, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
	imulq	$88, 400(%rsp), %rsi
	movq	392(%rsp), %rcx
	movq	408(%rsp), %r15
	movq	416(%rsp), %rbx
	movq	424(%rsp), %rdx
	movq	432(%rsp), %rbp
	movq	%rcx, 344(%rsp)
	movq	%rax, 360(%rsp)
	movq	440(%rsp), %rax
	movq	%rcx, 32(%rsp)
	movq	%rcx, 352(%rsp)
	movq	%rdx, 1872(%rsp)
	movq	%rbx, 128(%rsp)
	movq	%r15, 120(%rsp)
	addq	%rcx, %rsi
	movq	448(%rsp), %rcx
	movq	%rsi, 168(%rsp)
	movq	%rsi, 368(%rsp)
	movq	%rax, 1064(%rsp)
	testq	%rcx, %rcx
	je	.LBB361_859
	movq	1064(%rsp), %rax
	movq	480(%rsp), %rsi
	shlq	$5, %rcx
	leaq	8(%rbx), %rdx
	movq	$0, 1072(%rsp)
	movq	%rbp, 176(%rsp)
	movq	%rdx, 1856(%rsp)
	addq	%rax, %rcx
	movq	%rsi, 80(%rsp)
	movq	%rcx, 1864(%rsp)
	jmp	.LBB361_709
.LBB361_707:
	movq	176(%rsp), %rbp
.LBB361_708:
	movq	1880(%rsp), %rax
	movq	%r15, 744(%rsp)
	movq	128(%rsp), %rbx
	movq	120(%rsp), %r15
	addq	$32, %rax
	cmpq	1864(%rsp), %rax
	je	.LBB361_859
.LBB361_709:
	movq	1072(%rsp), %rcx
	movq	16(%rax), %rdx
	movq	8(%rax), %r14
	movq	%rax, 1880(%rsp)
	movq	%rcx, 64(%rsp)
	movq	(%rax), %rcx
	movq	24(%rax), %rax
	movq	%rdx, 88(%rsp)
	movq	%rax, 144(%rsp)
	testq	%rcx, %rcx
	je	.LBB361_716
	cmpq	$-1, 1024(%rsp)
	je	.LBB361_716
	movq	80(%r13), %rax
	movq	$-1, %rsi
.LBB361_712:
	movq	%rax, %rdx
	addq	%rcx, %rdx
	cmovbq	%rsi, %rdx
	lock		cmpxchgq	%rdx, 80(%r13)
	jne	.LBB361_712
	movq	320(%rsp), %rdx
	addq	%rcx, %rax
	cmovbq	%rsi, %rax
	movq	(%rdx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB361_716
	movq	%rcx, 232(%rsp)
	movq	%rax, 240(%rsp)
	movw	$0, 224(%rsp)
.Ltmp16005:
	movq	320(%rsp), %rsi
	leaq	640(%rsp), %rdi
	leaq	224(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.4261325137610144415)
.Ltmp16006:
	cmpb	$-1, 640(%rsp)
	jne	.LBB361_894
.LBB361_716:
	movq	1872(%rsp), %rdx
	movq	64(%rsp), %rdi
	cmpq	%rdx, %rdi
	ja	.LBB361_937
	movq	80(%rsp), %r9
	cmpq	%rdx, %r14
	movq	%rdx, %rsi
	movq	$-1, %r12
	leaq	.LJTI361_0(%rip), %r8
	cmovbq	%r14, %rsi
	cmpq	%rdi, %r14
	cmovbq	%rdi, %rsi
	cmpq	%rdi, %rsi
	jb	.LBB361_934
	leaq	(,%rdi,8), %rax
	leaq	(%rax,%rax,2), %rbp
	leaq	(,%rsi,8), %rax
	leaq	(%rax,%rax,2), %r11
	cmpq	%rsi, %rdi
	jne	.LBB361_725
	xorl	%r15d, %r15d
	xorl	%r14d, %r14d
	xorl	%ebx, %ebx
.LBB361_720:
	cmpq	$-1, 376(%rsp)
	movq	$-1, %r12
	movq	%r15, 56(%rsp)
	movq	%r14, 1088(%rsp)
	movq	%rbx, 1512(%rsp)
	movq	%r11, 48(%rsp)
	movq	%rsi, 1072(%rsp)
	je	.LBB361_731
	movq	88(%rsp), %rax
	movl	$0, %ecx
	movq	168(%rsp), %rbx
	movl	$0, %r15d
	subq	%r9, %rax
	cmovbq	%rcx, %rax
	subq	32(%rsp), %rbx
	movabsq	$3353953467947191203, %rcx
	shrq	$3, %rbx
	imulq	%rcx, %rbx
	cmpq	%rbx, %rax
	cmovbq	%rax, %rbx
	testq	%rbx, %rbx
	je	.LBB361_732
	movq	32(%rsp), %rax
	xorl	%r15d, %r15d
	leaq	8(%rax), %r14
.LBB361_723:
.Ltmp16008:
	movq	purrdf_sparql_eval::scratch::value_bytes@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp16009:
	addq	%rax, %r15
	cmovbq	%r12, %r15
	addq	$88, %r14
	decq	%rbx
	jne	.LBB361_723
	jmp	.LBB361_732
.LBB361_725:
	movq	%r11, %rdx
	subq	%rbp, %rdx
	movabsq	$-6148914691236517205, %rax
	xorl	%ebx, %ebx
	xorl	%r14d, %r14d
	xorl	%r15d, %r15d
	mulxq	%rax, %rax, %rax
	movq	1856(%rsp), %rcx
	shrq	$4, %rax
	addq	%rbp, %rcx
	jmp	.LBB361_728
.LBB361_726:
	addq	%rdx, %r14
	cmovbq	%r12, %r14
.LBB361_727:
	addq	$24, %rcx
	decq	%rax
	je	.LBB361_720
.LBB361_728:
	movzbl	-8(%rcx), %r10d
	movq	(%rcx), %rdx
	movslq	(%r8,%r10,4), %r10
	addq	%r8, %r10
	jmpq	*%r10
.LBB361_729:
	addq	%rdx, %r15
	cmovbq	%r12, %r15
	jmp	.LBB361_727
.LBB361_730:
	cmpq	%rdx, %rbx
	cmovbeq	%rdx, %rbx
	jmp	.LBB361_727
.LBB361_731:
	xorl	%r15d, %r15d
.LBB361_732:
	movl	296(%r13), %eax
	movq	$-1, %rdx
	testl	%eax, %eax
	je	.LBB361_755
.LBB361_733:
	cmpq	$-1, 1024(%rsp)
	je	.LBB361_735
	movq	80(%r13), %rax
	addq	56(%rsp), %rax
	cmovbq	%rdx, %rax
	cmpq	16(%r13), %rax
	ja	.LBB361_756
.LBB361_735:
	cmpq	$-1, 376(%rsp)
	je	.LBB361_737
	movq	2520(%rsp), %rcx
	movq	1048(%rcx), %rax
	movq	1056(%rcx), %rcx
	subq	%rcx, %rax
	movl	$0, %ecx
	cmovaeq	%rax, %rcx
	addq	%r15, %rcx
	cmovbq	%rdx, %rcx
	addq	1088(%rsp), %rcx
	cmovbq	%rdx, %rcx
	addq	1512(%rsp), %rcx
	movq	104(%r13), %rax
	cmovbq	%rdx, %rcx
	addq	%rcx, %rax
	cmovbq	%rdx, %rax
	cmpq	40(%r13), %rax
	ja	.LBB361_756
.LBB361_737:
	cmpq	$-1, 1024(%rsp)
	movq	128(%rsp), %rbx
	movq	1072(%rsp), %r9
	movq	48(%rsp), %r10
	movq	64(%rsp), %r11
	je	.LBB361_739
	movq	%rbp, %rax
	cmpq	%r9, %r11
	jne	.LBB361_750
.LBB361_739:
	movq	120(%rsp), %r15
	movb	$1, %r14b
	cmpq	%r9, %r11
	je	.LBB361_742
.LBB361_740:
	cmpb	$2, -24(%rbx,%r10)
	je	.LBB361_823
	addq	$-24, %r10
	cmpq	%r10, %rbp
	jne	.LBB361_740
.LBB361_742:
	movq	136(%rsp), %r12
	movq	32(%rsp), %rbp
.LBB361_743:
	cmpq	$-1, 1024(%rsp)
	movq	%rbp, 32(%rsp)
	je	.LBB361_811
	cmpq	$0, 56(%rsp)
	je	.LBB361_811
	movq	56(%rsp), %rax
	movq	%rax, 640(%rsp)
	movq	$0, 648(%rsp)
.Ltmp16014:
	movq	320(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	leaq	224(%rsp), %rdi
	leaq	640(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp16015:
	cmpb	$-1, 240(%rsp)
	je	.LBB361_811
	cmpq	$-1, 376(%rsp)
	movb	$1, %bpl
	jne	.LBB361_834
	jmp	.LBB361_748
.LBB361_749:
	addq	$24, %rax
	cmpq	%rax, %r10
	je	.LBB361_739
.LBB361_750:
	cmpb	$0, (%rbx,%rax)
	jne	.LBB361_749
	movzbl	1(%rbx,%rax), %ecx
	cmpq	$255, %rcx
	je	.LBB361_749
	movq	2520(%rsp), %rdx
	movq	632(%rdx), %rdx
	testq	%rdx, %rdx
	je	.LBB361_749
	movq	2520(%rsp), %rsi
	movl	1228(%rsi), %esi
	cmpq	%rsi, 56(%rdx)
	jbe	.LBB361_749
	movq	%rsi, %r8
	shlq	$7, %r8
	movq	8(%rbx,%rax), %rdi
	leaq	(%r8,%rsi,8), %rsi
	addq	48(%rdx), %rsi
	lock		addq	%rdi, (%rsi,%rcx,8)
	jmp	.LBB361_749
.LBB361_755:
	movq	1152(%rsp), %rax
	cmpb	$-1, (%rax)
	je	.LBB361_733
.LBB361_756:
	movq	64(%rsp), %rax
	cmpq	1072(%rsp), %rax
	jne	.LBB361_766
.LBB361_757:
	cmpq	$-1, 376(%rsp)
	je	.LBB361_816
	movq	80(%rsp), %rbx
	movq	136(%rsp), %r12
	cmpq	88(%rsp), %rbx
	jae	.LBB361_846
	movq	32(%rsp), %r15
	cmpq	168(%rsp), %r15
	je	.LBB361_765
	movq	88(%rsp), %rax
	addq	$88, %r15
	leaq	-1(%rax), %r14
	movq	%r15, %rax
.LBB361_761:
	movq	%rax, %r15
	movq	-8(%r15), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rcx, 704(%rsp)
	vmovdqu64	-72(%r15), %zmm0
	vmovdqu64	%zmm0, 640(%rsp)
	cmpq	$-1, %rax
	je	.LBB361_765
	vmovdqu64	640(%rsp), %zmm0
	movq	%rax, 224(%rsp)
	movq	704(%rsp), %rax
	leaq	232(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp16045:
	movq	96(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	224(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp16046:
	cmpq	%rbx, %r14
	je	.LBB361_830
	leaq	-88(%r15), %rcx
	incq	%rbx
	leaq	88(%r15), %rax
	addq	$88, %rcx
	cmpq	168(%rsp), %rcx
	jne	.LBB361_761
.LBB361_765:
	movq	%rbx, 80(%rsp)
	movq	%r15, 32(%rsp)
	movq	%r15, 352(%rsp)
	jmp	.LBB361_846
.LBB361_766:
	movq	128(%rsp), %rbx
	movq	120(%rsp), %r15
	addq	%rbx, 48(%rsp)
	addq	%rbx, %rbp
	jmp	.LBB361_769
.LBB361_767:
	movq	128(%rsp), %rbx
	movq	120(%rsp), %r15
.LBB361_768:
	addq	$24, %rbp
	cmpq	48(%rsp), %rbp
	je	.LBB361_757
.LBB361_769:
	movzbl	(%rbp), %eax
	leaq	.LJTI361_1(%rip), %rcx
	movslq	(%rcx,%rax,4), %rax
	addq	%rcx, %rax
	jmpq	*%rax
.LBB361_770:
	cmpq	$-1, 1024(%rsp)
	je	.LBB361_768
	movzbl	1(%rbp), %r14d
	movq	8(%rbp), %r15
	movq	16(%rbp), %r12
	movq	80(%r13), %rax
	movq	$-1, %rdx
	.p2align	4
.LBB361_772:
	movq	%rax, %rcx
	addq	%r15, %rcx
	cmovbq	%rdx, %rcx
	lock		cmpxchgq	%rcx, 80(%r13)
	jne	.LBB361_772
	movq	320(%rsp), %rcx
	addq	%r15, %rax
	cmovbq	%rdx, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB361_776
	movq	%rcx, 232(%rsp)
	movq	%rax, 240(%rsp)
	movw	$0, 224(%rsp)
.Ltmp16039:
	movq	320(%rsp), %rsi
	leaq	640(%rsp), %rdi
	leaq	224(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.4261325137610144415)
.Ltmp16040:
	cmpb	$-1, 640(%rsp)
	jne	.LBB361_873
.LBB361_776:
	cmpl	$255, %r14d
	je	.LBB361_767
	movq	2520(%rsp), %rcx
	movq	632(%rcx), %rax
	testq	%rax, %rax
	je	.LBB361_767
	movl	1228(%rcx), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB361_767
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	lock		addq	%r15, (%rcx,%r14,8)
	jmp	.LBB361_767
.LBB361_780:
	movq	8(%rbp), %r14
	cmpq	%r14, 80(%rsp)
	jae	.LBB361_807
	movq	32(%rsp), %rdx
	cmpq	168(%rsp), %rdx
	je	.LBB361_800
	movq	80(%rsp), %rbx
	addq	$88, %rdx
	leaq	-1(%r14), %r15
	movq	%rdx, %rax
.LBB361_783:
	movq	%rax, %rdx
	movq	-8(%rdx), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rdx, 32(%rsp)
	movq	%rcx, 704(%rsp)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, 640(%rsp)
	cmpq	$-1, %rax
	je	.LBB361_804
	vmovdqu64	640(%rsp), %zmm0
	movq	%rax, 224(%rsp)
	movq	704(%rsp), %rax
	leaq	232(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp16028:
	movq	96(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	224(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp16029:
	cmpq	%rbx, %r15
	je	.LBB361_806
	movq	32(%rsp), %rdx
	incq	%rbx
	leaq	-88(%rdx), %rcx
	leaq	88(%rdx), %rax
	addq	$88, %rcx
	cmpq	168(%rsp), %rcx
	jne	.LBB361_783
	jmp	.LBB361_805
.LBB361_787:
	cmpq	$-1, 376(%rsp)
	je	.LBB361_768
	movq	8(%rbp), %rcx
	movl	296(%r13), %eax
	testl	%eax, %eax
	je	.LBB361_798
	movq	104(%r13), %rax
	movq	$-1, %rdx
	addq	%rcx, %rax
	movq	40(%r13), %rcx
	cmovbq	%rdx, %rax
	cmpq	%rcx, %rax
	jbe	.LBB361_768
	movq	%rcx, 232(%rsp)
	movq	%rax, 240(%rsp)
	movw	$768, 224(%rsp)
.Ltmp16026:
	movq	320(%rsp), %rsi
	leaq	640(%rsp), %rdi
	leaq	224(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.4261325137610144415)
.Ltmp16027:
	jmp	.LBB361_799
.LBB361_791:
	cmpq	$-1, 376(%rsp)
	je	.LBB361_768
	cmpq	$-1, 40(%r13)
	je	.LBB361_768
	movq	8(%rbp), %rcx
	movq	16(%rbp), %r14
	movl	296(%r13), %eax
	testl	%eax, %eax
	je	.LBB361_801
	movq	104(%r13), %rax
	movq	$-1, %rsi
.LBB361_795:
	movq	%rax, %rdx
	addq	%rcx, %rdx
	cmovbq	%rsi, %rdx
	lock		cmpxchgq	%rdx, 104(%r13)
	jne	.LBB361_795
	addq	%rcx, %rax
	movq	40(%r13), %rcx
	cmovbq	%rsi, %rax
	cmpq	%rcx, %rax
	jbe	.LBB361_768
	movq	%rcx, 232(%rsp)
	movq	%rax, 240(%rsp)
	movw	$768, 224(%rsp)
.Ltmp16033:
	movq	320(%rsp), %rsi
	leaq	640(%rsp), %rdi
	leaq	224(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.4261325137610144415)
.Ltmp16034:
	jmp	.LBB361_802
.LBB361_798:
	movq	1152(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 656(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 640(%rsp)
.LBB361_799:
	cmpb	$-1, 640(%rsp)
	je	.LBB361_768
	jmp	.LBB361_906
.LBB361_800:
	movq	80(%rsp), %rbx
	jmp	.LBB361_805
.LBB361_801:
	movq	1152(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 656(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 640(%rsp)
.LBB361_802:
	movzbl	640(%rsp), %eax
	cmpb	$-1, %al
	sete	%cl
	testq	%r14, %r14
	sete	%dl
	orb	%cl, %dl
	je	.LBB361_882
	cmpb	$-1, %al
	je	.LBB361_768
	jmp	.LBB361_906
.LBB361_804:
	movq	32(%rsp), %rdx
.LBB361_805:
	movq	%rbx, 80(%rsp)
	movq	128(%rsp), %rbx
	movq	120(%rsp), %r15
	movq	%rdx, 32(%rsp)
	movq	%rdx, 352(%rsp)
	jmp	.LBB361_808
.LBB361_806:
	movq	128(%rsp), %rbx
	movq	120(%rsp), %r15
	movq	%r14, 80(%rsp)
.LBB361_807:
	movq	32(%rsp), %rax
	movq	%rax, 352(%rsp)
.LBB361_808:
	cmpq	$-1, 376(%rsp)
	je	.LBB361_768
.Ltmp16031:
	movq	2520(%rsp), %rsi
	leaq	224(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp16032:
	cmpb	$-1, 224(%rsp)
	je	.LBB361_768
	jmp	.LBB361_906
.LBB361_811:
	cmpq	$-1, 376(%rsp)
	je	.LBB361_846
	cmpq	$-1, 40(%r13)
	je	.LBB361_817
.Ltmp16016:
	movq	320(%rsp), %rsi
	movq	1088(%rsp), %rcx
	leaq	224(%rsp), %rdi
	movl	$3, %edx
	xorl	%r8d, %r8d
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.4261325137610144415)
.Ltmp16017:
	cmpb	$-1, 224(%rsp)
	je	.LBB361_817
	movb	$1, %bpl
	jmp	.LBB361_834
.LBB361_816:
	movq	136(%rsp), %r12
	jmp	.LBB361_846
.LBB361_817:
	testb	%r14b, %r14b
	jne	.LBB361_820
.Ltmp16018:
	movq	2520(%rsp), %rsi
	leaq	224(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp16019:
	cmpb	$-1, 224(%rsp)
	movb	$1, %bpl
	jne	.LBB361_834
.LBB361_820:
	movq	1512(%rsp), %rcx
	testq	%rcx, %rcx
	je	.LBB361_833
.Ltmp16020:
	movq	320(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::admit_transient@GOTPCREL(%rip), %rax
	leaq	224(%rsp), %rdi
	movl	$3, %edx
	vzeroupper
	callq	*%rax
.Ltmp16021:
	cmpb	$-1, 224(%rsp)
	setne	%bpl
	jmp	.LBB361_834
.LBB361_823:
	movq	-16(%rbx,%r10), %r14
	cmpq	%r14, 80(%rsp)
	jae	.LBB361_831
	movq	32(%rsp), %rbp
	movq	136(%rsp), %r12
	cmpq	168(%rsp), %rbp
	je	.LBB361_856
	movq	80(%rsp), %rbx
	addq	$88, %rbp
	leaq	-1(%r14), %r15
	movq	%rbp, %rax
.LBB361_826:
	movq	%rax, %rbp
	movq	-8(%rbp), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rcx, 704(%rsp)
	vmovdqu64	-72(%rbp), %zmm0
	vmovdqu64	%zmm0, 640(%rsp)
	cmpq	$-1, %rax
	je	.LBB361_857
	vmovdqu64	640(%rsp), %zmm0
	movq	%rax, 224(%rsp)
	movq	704(%rsp), %rax
	leaq	232(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp16011:
	movq	96(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	224(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp16012:
	cmpq	%rbx, %r15
	je	.LBB361_858
	leaq	-88(%rbp), %rcx
	leaq	88(%rbp), %rax
	incq	%rbx
	addq	$88, %rcx
	cmpq	168(%rsp), %rcx
	jne	.LBB361_826
	jmp	.LBB361_857
.LBB361_830:
	movq	88(%rsp), %rax
	movq	%r15, 32(%rsp)
	movq	%r15, 352(%rsp)
	movq	%rax, 80(%rsp)
	jmp	.LBB361_846
.LBB361_831:
	movq	136(%rsp), %r12
	movq	32(%rsp), %rbp
.LBB361_832:
	xorl	%r14d, %r14d
	movq	%rbp, 352(%rsp)
	jmp	.LBB361_743
.LBB361_833:
	xorl	%ebp, %ebp
.LBB361_834:
	movq	88(%rsp), %rax
	cmpq	%rax, 80(%rsp)
	jae	.LBB361_844
	movq	32(%rsp), %rdx
	cmpq	168(%rsp), %rdx
	je	.LBB361_842
	movq	88(%rsp), %rax
	addq	$88, %rdx
	leaq	-1(%rax), %r14
	movq	%rdx, %rax
.LBB361_837:
	movq	%rax, %rdx
	movq	-8(%rdx), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rdx, 32(%rsp)
	movq	%rcx, 704(%rsp)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, 640(%rsp)
	cmpq	$-1, %rax
	je	.LBB361_841
	vmovdqu64	640(%rsp), %zmm0
	movq	%rax, 224(%rsp)
	movq	704(%rsp), %rax
	leaq	232(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp16023:
	movq	96(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	224(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp16024:
	movq	80(%rsp), %rsi
	cmpq	%rsi, %r14
	je	.LBB361_843
	movq	32(%rsp), %rdx
	incq	%rsi
	movq	%rsi, 80(%rsp)
	leaq	-88(%rdx), %rcx
	leaq	88(%rdx), %rax
	addq	$88, %rcx
	cmpq	168(%rsp), %rcx
	jne	.LBB361_837
	jmp	.LBB361_842
.LBB361_841:
	movq	32(%rsp), %rdx
.LBB361_842:
	movq	%rdx, 32(%rsp)
	movq	%rdx, 352(%rsp)
	jmp	.LBB361_845
.LBB361_843:
	movq	88(%rsp), %rax
	movq	%rax, 80(%rsp)
.LBB361_844:
	movq	32(%rsp), %rax
	movq	%rax, 352(%rsp)
.LBB361_845:
	testb	%bpl, %bpl
	jne	.LBB361_748
.LBB361_846:
	movq	744(%rsp), %r15
	cmpq	$0, 144(%rsp)
	je	.LBB361_707
	movq	760(%rsp), %rax
	movq	176(%rsp), %rbp
	movq	%rax, 1088(%rsp)
	jmp	.LBB361_849
.LBB361_848:
	movq	208(%rsp), %rax
	movq	48(%rsp), %rdx
	leaq	(%rbx,%rbx,4), %rcx
	shll	$16, %ebp
	movq	144(%rsp), %rsi
	movq	88(%rsp), %rdi
	addq	$40, %r15
	incq	%rbx
	orl	%ebp, %r12d
	movq	176(%rsp), %rbp
	shlq	$32, %r12
	orq	%r12, %r13
	movq	136(%rsp), %r12
	movq	%rdx, (%rax,%rcx,8)
	movq	64(%rsp), %rdx
	decq	%rsi
	movq	%rsi, 144(%rsp)
	movq	%rdx, 8(%rax,%rcx,8)
	movzbl	56(%rsp), %edx
	movq	%r14, 16(%rax,%rcx,8)
	movb	%dl, 24(%rax,%rcx,8)
	movq	%r13, %rdx
	movl	%r13d, 25(%rax,%rcx,8)
	shrq	$32, %r13
	shrq	$48, %rdx
	movw	%r13w, 29(%rax,%rcx,8)
	movq	1032(%rsp), %r13
	movb	%dl, 31(%rax,%rcx,8)
	movq	%rdi, 32(%rax,%rcx,8)
	movq	%rbx, 216(%rsp)
	testq	%rsi, %rsi
	je	.LBB361_708
.LBB361_849:
	cmpq	1088(%rsp), %r15
	je	.LBB361_708
	movq	32(%r15), %rax
	leaq	648(%rsp), %rcx
	movq	%rax, 32(%rcx)
	movq	2520(%rsp), %rax
	vmovdqu	(%r15), %ymm0
	vmovdqu	%ymm0, (%rcx)
	movq	%rax, 640(%rsp)
	cmpq	$0, 648(%rsp)
	je	.LBB361_852
	movq	32(%r15), %rax
	leaq	232(%rsp), %rcx
	movq	%rax, 32(%rcx)
	vmovdqu	(%r15), %ymm0
	vmovdqu	%ymm0, (%rcx)
	jmp	.LBB361_854
.LBB361_852:
	movq	664(%rax), %rdx
.Ltmp16048:
	movq	96(%rsp), %rsi
	leaq	224(%rsp), %rdi
	leaq	656(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp16049:
	movq	224(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB361_871
.LBB361_854:
	movq	232(%rsp), %rax
	movq	240(%rsp), %rsi
	movzbl	256(%rsp), %edx
	movq	264(%rsp), %rcx
	movq	248(%rsp), %r14
	movzbl	263(%rsp), %ebp
	movzwl	261(%rsp), %r12d
	movl	257(%rsp), %r13d
	movq	216(%rsp), %rbx
	movq	%rax, 48(%rsp)
	movq	%rsi, 64(%rsp)
	movb	%dl, 56(%rsp)
	movq	%rcx, 88(%rsp)
	cmpq	200(%rsp), %rbx
	jne	.LBB361_848
.Ltmp16058:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	200(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16059:
	jmp	.LBB361_848
.LBB361_856:
	movq	80(%rsp), %rbx
.LBB361_857:
	movq	%rbx, 80(%rsp)
	movq	128(%rsp), %rbx
	movq	120(%rsp), %r15
	movq	%rbp, 352(%rsp)
	xorl	%r14d, %r14d
	jmp	.LBB361_743
.LBB361_858:
	movq	128(%rsp), %rbx
	movq	120(%rsp), %r15
	movq	%r14, 80(%rsp)
	jmp	.LBB361_832
.LBB361_859:
	testq	%rbp, %rbp
	je	.LBB361_861
	movq	1064(%rsp), %rdi
	shlq	$5, %rbp
	movl	$8, %edx
	movq	%rbp, %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB361_861:
.Ltmp16068:
	leaq	344(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp16069:
	testq	%r15, %r15
	je	.LBB361_864
	shlq	$3, %r15
	movl	$8, %edx
	movq	%rbx, %rdi
	leaq	(%r15,%r15,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB361_864:
	movq	472(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_867
	lock		decq	(%rax)
	jne	.LBB361_867
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	472(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB361_867:
	movq	504(%rsp), %rax
	movq	1848(%rsp), %r15
	testq	%rax, %rax
	je	.LBB361_870
	lock		decq	(%rax)
	jne	.LBB361_870
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	504(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB361_870:
	cmpq	1496(%rsp), %r15
	jne	.LBB361_704
	jmp	.LBB361_589
.LBB361_871:
	movq	%rax, 48(%rsp)
	movq	232(%rsp), %rax
	vmovups	272(%rsp), %ymm0
	movzwl	261(%rsp), %ecx
	addq	$40, %r15
	movl	257(%rsp), %r13d
	movq	264(%rsp), %r14
	movq	128(%rsp), %rbx
	movq	176(%rsp), %rbp
	movq	%r15, 744(%rsp)
	movq	120(%rsp), %r15
	movq	%rax, 144(%rsp)
	movq	240(%rsp), %rax
	vmovups	%ymm0, 1360(%rsp)
	vmovdqu	288(%rsp), %ymm0
	movq	%rax, 96(%rsp)
	movq	248(%rsp), %rax
	movq	%rax, 32(%rsp)
	movzbl	256(%rsp), %eax
	vmovdqu	%ymm0, 1376(%rsp)
	movq	%rax, 64(%rsp)
	movzbl	263(%rsp), %eax
	shll	$16, %eax
	orl	%eax, %ecx
	movb	$1, %al
	shlq	$32, %rcx
	movl	%eax, 56(%rsp)
	orq	%rcx, %r13
	jmp	.LBB361_908
.LBB361_872:
.Ltmp16185:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.536(%rip), %rcx
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
.Ltmp16186:
	jmp	.LBB361_950
.LBB361_873:
	movq	128(%rsp), %rbx
	movq	120(%rsp), %r15
	testq	%r12, %r12
	je	.LBB361_906
	movq	80(%rsp), %rbx
	leaq	-1(%r12), %rax
	cmpq	%rax, %rbx
	jae	.LBB361_904
	movq	32(%rsp), %r15
	cmpq	168(%rsp), %r15
	je	.LBB361_881
	notq	%rbx
	addq	$88, %r15
	leaq	224(%rsp), %r14
	addq	%r12, %rbx
	movq	%r15, %rax
.LBB361_877:
	movq	%rax, %r15
	movq	-8(%r15), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rcx, 704(%rsp)
	vmovdqu64	-72(%r15), %zmm0
	vmovdqu64	%zmm0, 640(%rsp)
	cmpq	$-1, %rax
	je	.LBB361_881
	vmovdqu64	640(%rsp), %zmm0
	movq	%rax, 224(%rsp)
	movq	704(%rsp), %rax
	leaq	232(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp16042:
	movq	96(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%r14, %rdx
	vzeroupper
	callq	*%rax
.Ltmp16043:
	decq	%rbx
	je	.LBB361_903
	leaq	-88(%r15), %rcx
	leaq	88(%r15), %rax
	addq	$88, %rcx
	cmpq	168(%rsp), %rcx
	jne	.LBB361_877
.LBB361_881:
	movq	%r15, 352(%rsp)
	jmp	.LBB361_905
.LBB361_882:
	leaq	-1(%r14), %rax
	cmpq	%rax, 80(%rsp)
	jae	.LBB361_898
	movq	32(%rsp), %r12
	cmpq	168(%rsp), %r12
	je	.LBB361_889
	movq	80(%rsp), %r13
	addq	$88, %r12
	movq	%r12, %rax
	notq	%r13
	addq	%r14, %r13
	leaq	224(%rsp), %r14
.LBB361_885:
	movq	%rax, %r12
	movq	-8(%r12), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rcx, 704(%rsp)
	vmovdqu64	-72(%r12), %zmm0
	vmovdqu64	%zmm0, 640(%rsp)
	cmpq	$-1, %rax
	je	.LBB361_889
	vmovdqu64	640(%rsp), %zmm0
	movq	%rax, 224(%rsp)
	movq	704(%rsp), %rax
	leaq	232(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp16036:
	movq	96(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%r14, %rdx
	vzeroupper
	callq	*%rax
.Ltmp16037:
	decq	%r13
	je	.LBB361_897
	leaq	-88(%r12), %rcx
	leaq	88(%r12), %rax
	addq	$88, %rcx
	cmpq	168(%rsp), %rcx
	jne	.LBB361_885
.LBB361_889:
	movq	%r12, 352(%rsp)
	jmp	.LBB361_906
.LBB361_748:
	movq	200(%rsp), %rax
	movq	208(%rsp), %rdx
	movq	216(%rsp), %rcx
	jmp	.LBB361_907
.LBB361_890:
	cmpq	$21, %r15
	jae	.LBB361_951
	movq	%r15, %rsi
	callq	core::slice::sort::shared::smallsort::insertion_sort_shift_left::<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>)), <[(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>))]>::sort_unstable_by_key<usize, purrdf_sparql_eval::modifier::contextual_group_rows<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#3}>::{closure#0}>
	jmp	.LBB361_184
.LBB361_892:
.Ltmp15874:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %r8
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.529(%rip), %rcx
	movq	%rdx, %rsi
	xorl	%edi, %edi
	movq	%rax, %rdx
	vzeroupper
	callq	*%r8
.Ltmp15875:
	jmp	.LBB361_950
.LBB361_893:
.Ltmp15864:
	movq	core::slice::copy_from_slice_impl::len_mismatch_fail@GOTPCREL(%rip), %rcx
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.530(%rip), %rax
	movq	%rdx, %rdi
	movq	%rax, %rdx
	vzeroupper
	callq	*%rcx
.Ltmp15865:
	jmp	.LBB361_950
.LBB361_894:
	movq	200(%rsp), %rax
	movq	208(%rsp), %rdx
	movq	216(%rsp), %rcx
	movq	$-1, 48(%rsp)
	movq	$0, 64(%rsp)
	movl	$0, 56(%rsp)
	movq	%rax, 144(%rsp)
	movq	%rdx, 96(%rsp)
	movq	%rcx, 32(%rsp)
	jmp	.LBB361_908
.LBB361_895:
.Ltmp15811:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.786(%rip), %rdx
	vzeroupper
	callq	*%rax
.Ltmp15812:
	jmp	.LBB361_950
.LBB361_896:
.Ltmp15808:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.786(%rip), %rdx
	vzeroupper
	callq	*%rax
.Ltmp15809:
	jmp	.LBB361_950
.LBB361_897:
	movq	%r12, 32(%rsp)
.LBB361_898:
	movq	32(%rsp), %rax
	movq	%rax, 352(%rsp)
	jmp	.LBB361_906
.LBB361_899:
.Ltmp15885:
	leaq	736(%rsp), %rdi
	movl	$16, %ecx
	movl	$224, %r8d
	xorl	%esi, %esi
	movq	%r15, %rdx
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.4261325137610144415)
.Ltmp15886:
	movq	736(%rsp), %rax
	movq	752(%rsp), %r13
	subq	%r13, %rax
	cmpq	%r15, %rax
	jae	.LBB361_331
.LBB361_901:
.Ltmp15899:
	movq	core::panicking::panic@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.1066(%rip), %rdi
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.1068(%rip), %rdx
	movl	$47, %esi
	callq	*%rax
.Ltmp15900:
	jmp	.LBB361_950
.LBB361_902:
	leaq	1936(%rsp), %rax
	movq	%rax, 800(%rsp)
	movq	<usize as core::fmt::Display>::fmt@GOTPCREL(%rip), %rax
	movq	%rax, 808(%rsp)
	movq	%rbx, 816(%rsp)
	movq	%rax, 824(%rsp)
.Ltmp15891:
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.619(%rip), %rdi
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.621(%rip), %rdx
	leaq	800(%rsp), %rsi
	callq	*%rax
.Ltmp15892:
	jmp	.LBB361_950
.LBB361_903:
	movq	%r15, 32(%rsp)
.LBB361_904:
	movq	32(%rsp), %rax
	movq	%rax, 352(%rsp)
.LBB361_905:
	movq	128(%rsp), %rbx
	movq	120(%rsp), %r15
.LBB361_906:
	movq	200(%rsp), %rax
	movq	208(%rsp), %rdx
	movq	216(%rsp), %rcx
	movq	136(%rsp), %r12
.LBB361_907:
	movq	176(%rsp), %rbp
	movq	$-1, 48(%rsp)
	movl	$0, 56(%rsp)
	movq	%rax, 144(%rsp)
	movb	$1, %al
	movq	%rdx, 96(%rsp)
	movq	%rcx, 32(%rsp)
	movq	%rax, 64(%rsp)
.LBB361_908:
	testq	%rbp, %rbp
	je	.LBB361_910
	movq	1064(%rsp), %rdi
	shlq	$5, %rbp
	movl	$8, %edx
	movq	%rbp, %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB361_910:
.Ltmp16051:
	leaq	344(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp16052:
	testq	%r15, %r15
	je	.LBB361_913
	shlq	$3, %r15
	movl	$8, %edx
	movq	%rbx, %rdi
	leaq	(%r15,%r15,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB361_913:
	movq	472(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_916
	lock		decq	(%rax)
	jne	.LBB361_916
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	472(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB361_916:
	movq	504(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_919
	lock		decq	(%rax)
	jne	.LBB361_919
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	504(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB361_919:
	xorl	%ebp, %ebp
	movq	%r14, 88(%rsp)
.Ltmp16054:
	leaq	768(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp16055:
	movq	1032(%rsp), %rax
	lock		decq	(%rax)
	movl	56(%rsp), %r14d
	jne	.LBB361_922
	xorl	%r15d, %r15d
	#MEMBARRIER
.Ltmp16056:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1176(%rsp), %rdi
	callq	*%rax
.Ltmp16057:
.LBB361_922:
	xorl	%r15d, %r15d
.LBB361_923:
	cmpq	$0, 1504(%rsp)
	movq	72(%rsp), %rbx
	movl	632(%rsp), %ebp
	je	.LBB361_925
.Ltmp16087:
	leaq	736(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16088:
.LBB361_925:
	testb	%r14b, %r14b
	je	.LBB361_933
	movq	208(%rsp), %rax
	movq	216(%rsp), %rbx
	movl	%r15d, %r14d
	movq	%rax, 56(%rsp)
	testq	%rbx, %rbx
	je	.LBB361_931
	movq	56(%rsp), %rax
	leaq	8(%rax), %r15
	jmp	.LBB361_929
.LBB361_928:
	addq	$40, %r15
	decq	%rbx
	je	.LBB361_931
.LBB361_929:
	movq	-8(%r15), %rax
	cmpq	$6, %rax
	jb	.LBB361_928
	movq	(%r15), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB361_928
.LBB361_931:
	movq	200(%rsp), %rax
	movq	72(%rsp), %rbx
	movl	%r14d, %r15d
	testq	%rax, %rax
	je	.LBB361_933
	movq	56(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
.LBB361_933:
	testb	%r15b, %r15b
	jne	.LBB361_458
	jmp	.LBB361_464
.LBB361_934:
.Ltmp16061:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.290(%rip), %rcx
	vzeroupper
	callq	*%rax
.Ltmp16062:
	jmp	.LBB361_950
.LBB361_935:
.Ltmp16135:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %r8
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.532(%rip), %rcx
	movq	%rdx, %rsi
	xorl	%edi, %edi
	movq	%rax, %rdx
	callq	*%r8
.Ltmp16136:
	jmp	.LBB361_950
.LBB361_936:
.Ltmp16125:
	movq	core::slice::copy_from_slice_impl::len_mismatch_fail@GOTPCREL(%rip), %rcx
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.533(%rip), %rax
	movq	%rdx, %rdi
	movq	%rax, %rdx
	callq	*%rcx
.Ltmp16126:
	jmp	.LBB361_950
.LBB361_937:
	leaq	1904(%rsp), %rax
	leaq	640(%rsp), %rcx
	movq	%rdi, 1904(%rsp)
	movq	%rdx, 640(%rsp)
	movq	%rax, 224(%rsp)
	leaq	<usize as core::fmt::Debug>::fmt(%rip), %rax
	movq	%rax, 232(%rsp)
	movq	%rcx, 240(%rsp)
	movq	%rax, 248(%rsp)
.Ltmp16063:
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.2158(%rip), %rdi
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.289(%rip), %rdx
	leaq	224(%rsp), %rsi
	vzeroupper
	callq	*%rax
.Ltmp16064:
	jmp	.LBB361_950
.LBB361_938:
.Ltmp16156:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.535(%rip), %rcx
	movq	%r15, %rdi
	movq	%rdx, %rsi
	callq	*%rax
.Ltmp16157:
	jmp	.LBB361_950
.LBB361_939:
.Ltmp15819:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$8, %esi
	callq	*%rax
.Ltmp15820:
	jmp	.LBB361_950
.LBB361_940:
.Ltmp15925:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$16, %edi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp15926:
	jmp	.LBB361_950
.LBB361_941:
.Ltmp15869:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.531(%rip), %rdx
	callq	*%rax
.Ltmp15870:
	jmp	.LBB361_950
.LBB361_942:
	movb	$1, %al
	movl	%eax, 56(%rsp)
.Ltmp16001:
	movl	$8, %ecx
	movl	$80, %r8d
	movb	$1, %bpl
	movq	%r15, %rdi
	movq	%r14, %rdx
	vzeroupper
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.4261325137610144415)
.Ltmp16002:
	jmp	.LBB361_701
.LBB361_943:
	leaq	1000(%rax), %rdi
	movb	$1, %al
	movl	%eax, 56(%rsp)
.Ltmp16003:
	movq	<hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	movb	$1, %bpl
	movq	%r14, %rsi
	movq	%r15, %rdx
	vzeroupper
	callq	*%rax
.Ltmp16004:
	jmp	.LBB361_702
.LBB361_944:
.Ltmp16200:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	callq	*%rax
.Ltmp16201:
	jmp	.LBB361_950
.LBB361_945:
.Ltmp15789:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$1, %edi
	movl	$51, %esi
	callq	*%rax
.Ltmp15790:
	jmp	.LBB361_950
.LBB361_946:
	addq	$16, %rdi
.Ltmp16109:
	leaq	384(%rsp), %rsi
	callq	<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::initialize::<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::get_or_init<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::try_insert::{closure#0}>::{closure#0}, !> (.llvm.4261325137610144415)
.Ltmp16110:
	jmp	.LBB361_602
.LBB361_947:
.Ltmp16130:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.534(%rip), %rdx
	callq	*%rax
.Ltmp16131:
	jmp	.LBB361_950
.LBB361_948:
.Ltmp15803:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp15804:
	jmp	.LBB361_950
.LBB361_949:
.Ltmp15877:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp15878:
.LBB361_950:
	ud2
.LBB361_951:
.Ltmp15831:
	movq	core::slice::sort::unstable::ipnsort::<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>)), <[(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>))]>::sort_unstable_by_key<usize, purrdf_sparql_eval::modifier::contextual_group_rows<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#3}>::{closure#0}>@GOTPCREL(%rip), %rax
	movq	%r15, %rsi
	callq	*%rax
.Ltmp15832:
	jmp	.LBB361_184
.LBB361_952:
.Ltmp16111:
	leaq	1184(%rsp), %rdi
	movq	%rax, %rbx
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB361_1110
.LBB361_953:
.Ltmp15860:
	movq	384(%rsp), %r15
	movq	%rax, %rbx
	cmpq	$6, %r15
	jae	.LBB361_1030
	jmp	.LBB361_1125
.LBB361_954:
.Ltmp16044:
	jmp	.LBB361_986
.LBB361_955:
.Ltmp16038:
	movq	%rax, %rbx
	movq	%r12, 352(%rsp)
	jmp	.LBB361_1090
.LBB361_956:
.Ltmp16070:
	movq	%rax, %rbx
	movb	$1, %al
	movl	%eax, 56(%rsp)
	jmp	.LBB361_1093
.LBB361_957:
.Ltmp16053:
	movq	%rax, %rbx
	jmp	.LBB361_1093
.LBB361_958:
.Ltmp15807:
	jmp	.LBB361_1116
.LBB361_959:
.Ltmp16007:
	jmp	.LBB361_1089
.LBB361_960:
.Ltmp16013:
	movq	%rax, %rbx
	movq	%rbp, 352(%rsp)
	jmp	.LBB361_1090
.LBB361_961:
.Ltmp16089:
	movl	%r15d, %ebp
	movq	%rax, %rbx
	movl	%r14d, 56(%rsp)
	jmp	.LBB361_1106
.LBB361_962:
.Ltmp16075:
	movq	%rax, %rbx
	jmp	.LBB361_1103
.LBB361_963:
.Ltmp16022:
	jmp	.LBB361_1089
.LBB361_964:
.Ltmp16097:
	cmpq	$0, 1504(%rsp)
	movl	%r15d, %ebp
	movq	%rax, %rbx
	movl	%r14d, 56(%rsp)
	jne	.LBB361_1105
	jmp	.LBB361_1106
.LBB361_965:
.Ltmp16041:
	jmp	.LBB361_1089
.LBB361_966:
.Ltmp15970:
	movq	%rax, %rbx
	jmp	.LBB361_1000
.LBB361_967:
.Ltmp16025:
	jmp	.LBB361_1015
.LBB361_968:
.Ltmp16000:
	movq	%rax, %rbx
	jmp	.LBB361_1109
.LBB361_1008:
.Ltmp15956:
	movq	%rax, %rbx
	jmp	.LBB361_1009
.LBB361_969:
.Ltmp16134:
	jmp	.LBB361_1074
.LBB361_970:
.Ltmp16084:
	addq	$40, %r14
	movq	%rax, %rbx
	movq	%r14, 232(%rsp)
	jmp	.LBB361_983
.LBB361_971:
.Ltmp16144:
	leaq	1440(%rsp), %rdi
	movq	%rax, %rbx
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movb	$1, %bpl
	jmp	.LBB361_973
.LBB361_972:
.Ltmp16147:
	movq	%rax, %rbx
	xorl	%ebp, %ebp
.LBB361_973:
	xorl	%r15d, %r15d
	jmp	.LBB361_1137
.LBB361_974:
.Ltmp16124:
	jmp	.LBB361_1074
.LBB361_975:
.Ltmp15995:
	movq	%rax, %rbx
.Ltmp15996:
	leaq	1416(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15997:
	jmp	.LBB361_1109
.LBB361_976:
.Ltmp15973:
	jmp	.LBB361_989
.LBB361_977:
.Ltmp15984:
	movq	%rax, %rbx
	jmp	.LBB361_1005
.LBB361_978:
.Ltmp15951:
	movq	%rax, %rbx
.Ltmp15952:
	leaq	1416(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15953:
	jmp	.LBB361_1009
.LBB361_979:
.Ltmp15915:
	movq	%rax, %rbx
.Ltmp15916:
	leaq	344(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>>>
.Ltmp15917:
	jmp	.LBB361_1131
.LBB361_980:
.Ltmp15918:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_981:
.Ltmp16092:
	addq	$40, %r14
	cmpq	$6, 144(%rsp)
	movq	%rax, %rbx
	movq	%r14, 232(%rsp)
	jb	.LBB361_983
	movq	144(%rsp), %rax
	movq	48(%rsp), %rdi
	movl	$4, %edx
	leaq	-8(,%rax,8), %rsi
	callq	__rustc::__rust_dealloc
.LBB361_983:
	movb	$1, %bpl
.Ltmp16093:
	leaq	224(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16094:
	jmp	.LBB361_1107
.LBB361_984:
.Ltmp16035:
	jmp	.LBB361_1089
.LBB361_985:
.Ltmp16047:
.LBB361_986:
	movq	%rax, %rbx
	movq	%r15, 352(%rsp)
	jmp	.LBB361_1090
.LBB361_987:
.Ltmp15854:
	jmp	.LBB361_1044
.LBB361_988:
.Ltmp16114:
.LBB361_989:
	movq	%rax, %rbx
	jmp	.LBB361_1110
.LBB361_990:
.Ltmp15937:
	jmp	.LBB361_1130
.LBB361_991:
.Ltmp16050:
	addq	$40, %r15
	movq	%rax, %rbx
	movq	%r15, 744(%rsp)
	jmp	.LBB361_1090
.LBB361_992:
.Ltmp16119:
	jmp	.LBB361_1018
.LBB361_993:
.Ltmp15959:
	movq	%rax, %rbx
	movq	%r15, 232(%rsp)
	jmp	.LBB361_999
.LBB361_994:
.Ltmp15816:
	jmp	.LBB361_1116
.LBB361_995:
.Ltmp16060:
	addq	$40, %r15
	cmpq	$6, 48(%rsp)
	movq	%rax, %rbx
	movq	%r15, 744(%rsp)
	jb	.LBB361_1090
	movq	48(%rsp), %rax
	movq	64(%rsp), %rdi
	movl	$4, %edx
	leaq	-8(,%rax,8), %rsi
	callq	__rustc::__rust_dealloc
	jmp	.LBB361_1090
.LBB361_997:
.Ltmp15964:
	movq	%rax, %rbx
	movq	%r15, 232(%rsp)
	cmpq	$6, %r14
	jb	.LBB361_999
	movq	32(%rsp), %rdi
	leaq	-8(,%r14,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB361_999:
.Ltmp15965:
	leaq	224(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15966:
.LBB361_1000:
	leaq	640(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB361_1006
.LBB361_1001:
.Ltmp15967:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_1002:
.Ltmp15946:
	movq	%rax, %rbx
.Ltmp15947:
	leaq	1416(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15948:
	jmp	.LBB361_1133
.LBB361_1003:
.Ltmp15976:
	movq	%rax, %rbx
.Ltmp15977:
	leaq	1184(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp15978:
.Ltmp15980:
	leaq	384(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
.Ltmp15981:
.LBB361_1005:
.Ltmp15985:
	leaq	224(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp15986:
.LBB361_1006:
	cmpb	$0, 64(%rsp)
	je	.LBB361_1009
.Ltmp15990:
	leaq	1472(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15991:
	jmp	.LBB361_1110
.LBB361_1009:
.Ltmp15988:
	leaq	800(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
.Ltmp15989:
	jmp	.LBB361_1110
.LBB361_1010:
.Ltmp15979:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_1011:
.Ltmp15987:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_1012:
.Ltmp15992:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_1013:
.Ltmp15873:
	movq	%rax, %rbx
	cmpq	$5, %r15
	ja	.LBB361_1124
	jmp	.LBB361_1125
.LBB361_1014:
.Ltmp16030:
.LBB361_1015:
	movq	32(%rsp), %rcx
	movq	%rax, %rbx
	movq	%rcx, 352(%rsp)
	jmp	.LBB361_1090
.LBB361_1016:
.Ltmp16179:
	movq	%rax, %rbx
	jmp	.LBB361_1157
.LBB361_1017:
.Ltmp15943:
.LBB361_1018:
	movq	%rax, %rbx
	jmp	.LBB361_1133
.LBB361_1019:
.Ltmp15827:
	jmp	.LBB361_1076
.LBB361_1020:
.Ltmp15921:
	movq	%rax, %rbx
	testq	%r12, %r12
	je	.LBB361_1059
	negq	%r12
	jmp	.LBB361_1023
.LBB361_1022:
	addq	$240, %r14
	decq	%r12
	je	.LBB361_1059
.LBB361_1023:
	cmpl	$2, (%r14)
	je	.LBB361_1022
.Ltmp15922:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>
.Ltmp15923:
	jmp	.LBB361_1022
.LBB361_1025:
.Ltmp15924:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_1026:
.Ltmp16163:
	movq	%rax, %rbx
	jmp	.LBB361_1136
.LBB361_1027:
.Ltmp16010:
	jmp	.LBB361_1089
.LBB361_1028:
.Ltmp16129:
	movq	1560(%rsp), %r13
	jmp	.LBB361_1085
.LBB361_1029:
.Ltmp15863:
	movq	800(%rsp), %r15
	movq	%rax, %rbx
	leaq	808(%rsp), %rax
	movq	%rax, 176(%rsp)
	cmpq	$5, %r15
	jbe	.LBB361_1125
.LBB361_1030:
	movq	176(%rsp), %rax
	movq	(%rax), %r14
	jmp	.LBB361_1124
.LBB361_1031:
.Ltmp15857:
	jmp	.LBB361_1044
.LBB361_1032:
.Ltmp15830:
	leaq	384(%rsp), %rdi
	movq	%r12, 832(%rsp)
	movq	%r15, 824(%rsp)
	movw	%bp, 848(%rsp)
	movq	%rax, %rbx
	movq	%r14, 856(%rsp)
	callq	core::ptr::drop_glue::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>
	leaq	800(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>
	leaq	224(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>>
	jmp	.LBB361_1144
.LBB361_1033:
.Ltmp16168:
	movq	%rax, %rbx
	jmp	.LBB361_1126
.LBB361_1034:
.Ltmp16105:
	movq	%rax, %rbx
	testq	%r15, %r15
	je	.LBB361_1038
	negq	%r15
	addq	$160, %r14
.LBB361_1036:
.Ltmp16106:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp16107:
	addq	$160, %r14
	decq	%r15
	jne	.LBB361_1036
.LBB361_1038:
	cmpq	$0, 136(%rsp)
	je	.LBB361_1110
	movq	136(%rsp), %rax
	movq	1080(%rsp), %rdi
	movl	$8, %edx
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
	jmp	.LBB361_1110
.LBB361_1040:
.Ltmp16108:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_1041:
.Ltmp15791:
	jmp	.LBB361_1078
.LBB361_1042:
.Ltmp15849:
	jmp	.LBB361_1044
.LBB361_1043:
.Ltmp15842:
.LBB361_1044:
	movq	%rax, %rbx
	movb	$1, %bpl
	movb	$1, %r15b
	jmp	.LBB361_1138
.LBB361_1045:
.Ltmp16202:
	movq	%rax, %rbx
.Ltmp16203:
	leaq	400(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.4261325137610144415)
.Ltmp16204:
	jmp	.LBB361_1169
.LBB361_1046:
.Ltmp16205:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_1047:
.Ltmp16150:
	movq	%rax, %rbx
	testq	%r12, %r12
	je	.LBB361_1051
	negq	%r12
	addq	$24, %r15
.LBB361_1049:
.Ltmp16151:
	movq	%r15, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
.Ltmp16152:
	addq	$24, %r15
	decq	%r12
	jne	.LBB361_1049
.LBB361_1051:
	movq	1096(%rsp), %rax
	xorl	%ebp, %ebp
	testq	%rax, %rax
	je	.LBB361_1053
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r14, %rdi
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
	xorl	%ebp, %ebp
.LBB361_1053:
	xorl	%r15d, %r15d
	jmp	.LBB361_1138
.LBB361_1054:
.Ltmp16153:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_1055:
.Ltmp16196:
	movq	%rax, %rbx
	jmp	.LBB361_1166
.LBB361_1056:
.Ltmp16182:
	movq	%rax, %rbx
	jmp	.LBB361_1159
.LBB361_1057:
.Ltmp15868:
	movq	1184(%rsp), %r15
	jmp	.LBB361_1122
.LBB361_1058:
.Ltmp15927:
	movq	%rax, %rbx
.LBB361_1059:
.Ltmp15928:
	leaq	640(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>, core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>
.Ltmp15929:
.Ltmp15930:
	leaq	800(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>>>
.Ltmp15931:
	jmp	.LBB361_1131
.LBB361_1061:
.Ltmp15932:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_1062:
.Ltmp16173:
	movl	%ebx, %r13d
	movq	%rax, %rbx
	testq	%r12, %r12
	je	.LBB361_1066
	negq	%r12
	addq	$24, %r15
.LBB361_1064:
.Ltmp16174:
	movq	%r15, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
.Ltmp16175:
	addq	$24, %r15
	decq	%r12
	jne	.LBB361_1064
.LBB361_1066:
	movq	1096(%rsp), %rax
	movb	$1, %bpl
	testq	%rax, %rax
	jne	.LBB361_1068
	movl	%r13d, %r15d
	jmp	.LBB361_1138
.LBB361_1068:
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r14, %rdi
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
	movl	%r13d, %r15d
	jmp	.LBB361_1138
.LBB361_1069:
.Ltmp16176:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_1070:
.Ltmp15821:
	cmpq	$6, 32(%rsp)
	movq	%rax, %rbx
	jb	.LBB361_1120
	movq	32(%rsp), %rax
	movq	88(%rsp), %rdi
	movl	$4, %edx
	leaq	-8(,%rax,8), %rsi
	jmp	.LBB361_1119
.LBB361_1072:
.Ltmp15802:
	movq	192(%rsp), %rdi
	movq	712(%rsp), %rsi
	movl	$8, %edx
	movq	%rax, %rbx
	callq	__rustc::__rust_dealloc
	jmp	.LBB361_1128
.LBB361_1073:
.Ltmp16158:
.LBB361_1074:
	movq	%rax, %rbx
	jmp	.LBB361_1087
.LBB361_1075:
.Ltmp15824:
.LBB361_1076:
	movq	%rax, %rbx
	jmp	.LBB361_1120
.LBB361_1077:
.Ltmp15788:
.LBB361_1078:
	movq	%rax, %rbx
.Ltmp15792:
	leaq	1600(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.4261325137610144415)
.Ltmp15793:
	jmp	.LBB361_1169
.LBB361_1079:
.Ltmp15893:
	movq	768(%rsp), %rdi
	movq	%rax, %rbx
.Ltmp15894:
	movq	%r14, %rsi
	callq	core::ptr::drop_glue::<rayon::iter::collect::consumer::CollectResult<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>
.Ltmp15895:
	jmp	.LBB361_1082
.LBB361_1080:
.Ltmp15896:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_1081:
.Ltmp15901:
	movq	%rax, %rbx
.LBB361_1082:
.Ltmp15902:
	leaq	736(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>
.Ltmp15903:
	jmp	.LBB361_1133
.LBB361_1083:
.Ltmp15904:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_1084:
.Ltmp16137:
.LBB361_1085:
	movq	%rax, %rbx
	cmpq	$6, %r13
	jb	.LBB361_1087
	movq	1568(%rsp), %rdi
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB361_1087:
	leaq	800(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB361_1133
.LBB361_1088:
.Ltmp16065:
.LBB361_1089:
	movq	%rax, %rbx
.LBB361_1090:
	cmpq	$0, 176(%rsp)
	je	.LBB361_1092
	movq	176(%rsp), %rsi
	movq	1064(%rsp), %rdi
	movl	$8, %edx
	shlq	$5, %rsi
	callq	__rustc::__rust_dealloc
.LBB361_1092:
	movb	$1, %al
	movl	%eax, 56(%rsp)
.Ltmp16066:
	leaq	344(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp16067:
.LBB361_1093:
	cmpq	$0, 120(%rsp)
	je	.LBB361_1095
	movq	120(%rsp), %rax
	movq	128(%rsp), %rdi
	movl	$8, %edx
	shlq	$3, %rax
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB361_1095:
	movq	472(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_1098
	lock		decq	(%rax)
	jne	.LBB361_1098
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	472(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB361_1098:
	movq	504(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_1101
	lock		decq	(%rax)
	jne	.LBB361_1101
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	504(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB361_1101:
.Ltmp16071:
	leaq	768(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp16072:
	xorl	%ebp, %ebp
.LBB361_1103:
	movq	1032(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB361_1105
	#MEMBARRIER
.Ltmp16076:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1176(%rsp), %rdi
	callq	*%rax
.Ltmp16077:
.LBB361_1105:
.Ltmp16098:
	leaq	736(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16099:
.LBB361_1106:
	cmpb	$0, 56(%rsp)
	je	.LBB361_1108
.LBB361_1107:
	leaq	200(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB361_1108:
	testb	%bpl, %bpl
	je	.LBB361_1110
.LBB361_1109:
.Ltmp16100:
	leaq	1528(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp16101:
.LBB361_1110:
.Ltmp16115:
	leaq	1936(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp16116:
	jmp	.LBB361_1133
.LBB361_1111:
.Ltmp16102:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_1112:
.Ltmp15810:
	movq	%rax, %rbx
	movq	%rbp, (%r14)
	jmp	.LBB361_1117
.LBB361_1113:
.Ltmp15782:
	movq	%rax, %rbx
.Ltmp15783:
	leaq	384(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.4261325137610144415)
.Ltmp15784:
	jmp	.LBB361_1169
.LBB361_1114:
.Ltmp15785:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_1115:
.Ltmp15813:
.LBB361_1116:
	movq	%rax, %rbx
.LBB361_1117:
	movq	384(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB361_1120
	movq	392(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
.LBB361_1119:
	callq	__rustc::__rust_dealloc
.LBB361_1120:
	leaq	1712(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>), purrdf_hash::fixed::FixedState>>
	jmp	.LBB361_1144
.LBB361_1121:
.Ltmp15876:
.LBB361_1122:
	movq	%rax, %rbx
	cmpq	$6, %r15
	jb	.LBB361_1125
	movq	1192(%rsp), %r14
.LBB361_1124:
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	movq	%r14, %rdi
	callq	__rustc::__rust_dealloc
.LBB361_1125:
	leaq	1968(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB361_1126:
	movb	$1, %bpl
	movb	$1, %r15b
	jmp	.LBB361_1137
.LBB361_1127:
.Ltmp16187:
	movq	%rax, %rbx
.LBB361_1128:
	movb	$1, %bpl
	movb	$1, %r15b
	jmp	.LBB361_1155
.LBB361_1129:
.Ltmp15940:
.LBB361_1130:
	movq	%rax, %rbx
.LBB361_1131:
	movq	768(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB361_1133
	movq	776(%rsp), %rdi
	shlq	$4, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB361_1133:
	movq	624(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_1136
	lock		decq	(%rax)
	jne	.LBB361_1136
	#MEMBARRIER
.Ltmp16159:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	624(%rsp), %rdi
	callq	*%rax
.Ltmp16160:
.LBB361_1136:
	movb	$1, %bpl
.Ltmp16164:
	leaq	1968(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp16165:
	movb	$1, %r15b
.LBB361_1137:
.Ltmp16169:
	leaq	1096(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>>
.Ltmp16170:
.LBB361_1138:
	leaq	1040(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>>
	jmp	.LBB361_1145
.LBB361_1139:
.Ltmp15835:
	movq	%rax, %rbx
	cmpq	$6, %r12
	jb	.LBB361_1141
	movq	96(%rsp), %rdi
	leaq	-8(,%r12,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB361_1141:
	testq	%r14, %r14
	je	.LBB361_1143
	movq	32(%rsp), %rdi
	shlq	$3, %r14
	movl	$8, %edx
	movq	%r14, %rsi
	callq	__rustc::__rust_dealloc
.LBB361_1143:
	leaq	1184(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>
.LBB361_1144:
	movb	$1, %r15b
	movb	$1, %bpl
.LBB361_1145:
	cmpq	$0, 720(%rsp)
	je	.LBB361_1155
	movq	712(%rsp), %rdi
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	cmpq	%rdx, %rdi
	cmovaeq	%rdx, %rdi
	xorl	%ecx, %ecx
	cmpq	%rdi, %rax
	movq	%rdi, %rsi
	setns	%cl
	addq	%rdx, %rcx
	subq	%rdi, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB361_1148
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_1148:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_1154
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_1148
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rsi, %rcx
	negq	%rcx
	movabsq	$9223372036854775807, %rdx
	lock		xaddq	%rcx, (%rax)
	xorl	%eax, %eax
	cmpq	%rsi, %rcx
	setns	%al
	addq	%rdx, %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	subq	%rsi, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB361_1151:
	cmpq	%rax, %rcx
	jge	.LBB361_1153
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB361_1151
.LBB361_1153:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_1154:
	movq	free@GOTPCREL(%rip), %rax
	movq	192(%rsp), %rdi
	callq	*%rax
.LBB361_1155:
	movq	184(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB361_1157
	#MEMBARRIER
.Ltmp16188:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	184(%rsp), %rdi
	callq	*%rax
.Ltmp16189:
.LBB361_1157:
.Ltmp16190:
	leaq	1120(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp16191:
	testb	%bpl, %bpl
	je	.LBB361_1166
.LBB361_1159:
	movq	1816(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB361_1160
	movq	1824(%rsp), %rdi
	leaq	-3(%rax,%rax,2), %rsi
	movl	$1, %edx
	callq	__rustc::__rust_dealloc
	movq	1744(%rsp), %rax
	testq	%rax, %rax
	jg	.LBB361_1163
.LBB361_1161:
	movq	1840(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB361_1164
	jmp	.LBB361_1166
.LBB361_1160:
	movq	1744(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB361_1161
.LBB361_1163:
	movq	1752(%rsp), %rdi
	leaq	(%rax,%rax,2), %rsi
	movl	$1, %edx
	callq	__rustc::__rust_dealloc
	movq	1840(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_1166
.LBB361_1164:
	lock		decq	(%rax)
	jne	.LBB361_1166
	leaq	1840(%rsp), %rdi
	#MEMBARRIER
.Ltmp16192:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp16193:
.LBB361_1166:
	testb	%r15b, %r15b
	je	.LBB361_1169
	movq	328(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB361_1169
	#MEMBARRIER
.Ltmp16197:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1520(%rsp), %rdi
	callq	*%rax
.Ltmp16198:
.LBB361_1169:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB361_1170:
.Ltmp16199:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end361:
