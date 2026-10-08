purrdf_sparql_eval::modifier::eval_group_with::<purrdf_core::ir::dataset::RdfDataset, ()>:
.Lfunc_begin363:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception269
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
	subq	$2488, %rsp
	.cfi_def_cfa_offset 2544
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	vmovdqu	anon.e5162873a9a3251d11c4df37a70e4654.29.llvm.10720091597982897309(%rip), %ymm0
	movq	$0, 384(%rsp)
	movq	$8, 392(%rsp)
	movq	$0, 400(%rsp)
	movq	%r9, 328(%rsp)
	movq	%rdx, %r14
	movq	%rsi, 1192(%rsp)
	movq	%rdi, 120(%rsp)
	movq	%r8, 128(%rsp)
	vmovdqu	%ymm0, 408(%rsp)
	testq	%r8, %r8
	je	.LBB363_5
	movq	128(%rsp), %rbx
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %r13
	movq	%rcx, %r15
	leaq	384(%rsp), %r12
	shlq	$4, %rbx
	addq	%rcx, %rbx
	.p2align	4
.LBB363_2:
	movq	(%r15), %rsi
	movq	8(%r15), %rdx
	lock		incq	(%rsi)
	jle	.LBB363_944
.Ltmp15712:
	movq	%r12, %rdi
	vzeroupper
	callq	*%r13
.Ltmp15713:
	addq	$16, %r15
	cmpq	%rbx, %r15
	jne	.LBB363_2
.LBB363_5:
	vmovdqu	408(%rsp), %ymm0
	movq	384(%rsp), %rax
	movq	392(%rsp), %rcx
	movq	400(%rsp), %rdx
	movq	408(%rsp), %rsi
	movq	2544(%rsp), %rdi
	movq	%rcx, 1640(%rsp)
	movq	%rax, 1632(%rsp)
	movq	%rdx, 1648(%rsp)
	imulq	$120, %rdi, %rcx
	addq	328(%rsp), %rcx
	vmovdqu	%ymm0, 1656(%rsp)
	movq	%rsi, 1656(%rsp)
	movq	1648(%rsp), %rax
	movq	%rcx, 40(%rsp)
	movq	%rax, 968(%rsp)
	testq	%rdi, %rdi
	je	.LBB363_11
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rbx
	movq	328(%rsp), %r12
	leaq	1632(%rsp), %r15
	.p2align	4
.LBB363_7:
	movq	(%r12), %rsi
	movq	1648(%rsp), %r13
	lock		incq	(%rsi)
	jle	.LBB363_944
	movq	8(%r12), %rdx
.Ltmp15718:
	movq	%r15, %rdi
	vzeroupper
	callq	*%rbx
.Ltmp15719:
	cmpq	%r13, %rax
	jne	.LBB363_65
	addq	$120, %r12
	cmpq	40(%rsp), %r12
	jne	.LBB363_7
.LBB363_11:
	movq	1632(%rsp), %rax
	vmovdqu	1664(%rsp), %xmm0
	movq	1656(%rsp), %rdi
	movq	1640(%rsp), %rcx
	movq	1648(%rsp), %rdx
	movq	1680(%rsp), %r8
	movq	1656(%rsp), %rsi
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
	movq	%rax, 320(%rsp)
	testq	%rax, %rax
	je	.LBB363_938
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
	jle	.LBB363_14
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB363_14:
	movq	120(%rsp), %rbx
	.p2align	4
.LBB363_15:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_21
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_15
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
.LBB363_18:
	cmpq	%rax, %rcx
	jle	.LBB363_20
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB363_18
.LBB363_20:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_21:
	vmovdqu64	384(%rsp), %zmm0
	movq	320(%rsp), %rcx
	movq	448(%rsp), %rax
	movb	$1, %r15b
	movq	%rax, 64(%rcx)
	movq	%rcx, 1552(%rsp)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp15726:
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	1192(%rsp), %rsi
	leaq	1736(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15727:
	movb	$1, %r15b
.Ltmp15728:
	movq	2552(%rsp), %rdx
	leaq	2368(%rsp), %rdi
	movq	%r14, %rsi
	movq	%r14, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp15729:
	cmpl	$1, 2368(%rsp)
	jne	.LBB363_47
	vmovdqu64	2416(%rsp), %zmm1
	vmovdqu64	2384(%rsp), %zmm0
	movq	1808(%rsp), %rax
	vmovdqu64	%zmm1, 48(%rbx)
	vmovdqu64	%zmm0, 16(%rbx)
	movq	$1, (%rbx)
	cmpq	$6, %rax
	jb	.LBB363_34
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	1816(%rsp), %rdi
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
	jge	.LBB363_27
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_27:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_33
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_27
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
.LBB363_30:
	cmpq	%rax, %rdx
	jge	.LBB363_32
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB363_30
.LBB363_32:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_33:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB363_34:
	movq	1736(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB363_44
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	1744(%rsp), %rdi
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
	jge	.LBB363_37
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_37:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_43
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_37
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
.LBB363_40:
	cmpq	%rax, %rdx
	jge	.LBB363_42
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB363_40
.LBB363_42:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_43:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB363_44:
	movq	1832(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_552
	lock		decq	(%rax)
	jne	.LBB363_552
	movb	$1, %r15b
	leaq	1832(%rsp), %rdi
	#MEMBARRIER
.Ltmp16123:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp16124:
	jmp	.LBB363_552
.LBB363_47:
	vmovdqu64	2408(%rsp), %zmm1
	vmovdqu64	2376(%rsp), %zmm0
	vmovdqu64	%zmm1, 416(%rsp)
	vmovdqu64	%zmm0, 384(%rsp)
.Ltmp15730:
	leaq	720(%rsp), %rdi
	leaq	1736(%rsp), %rsi
	leaq	384(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp15731:
	cmpq	$-1, 720(%rsp)
	je	.LBB363_75
	vmovdqu	720(%rsp), %ymm0
	vmovdqu	%ymm0, 1152(%rsp)
	movq	1176(%rsp), %rax
	lock		incq	(%rax)
	jle	.LBB363_944
	movq	320(%rsp), %rcx
	movq	1176(%rsp), %rax
	movq	968(%rsp), %rsi
	leaq	384(%rsp), %r11
	movq	32(%rcx), %rdx
	movq	%rax, 160(%rsp)
	cmpq	%rdx, %rsi
	ja	.LBB363_866
	movq	%rsi, %r14
	shlq	$4, %r14
	movq	%rsi, 960(%rsp)
	movq	%r14, 952(%rsp)
	testq	%rsi, %rsi
	je	.LBB363_77
	movq	320(%rsp), %rax
	movq	%r14, %rdi
	movq	24(%rax), %rbx
	movq	malloc@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	%rax, 176(%rsp)
	testq	%rax, %rax
	je	.LBB363_942
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
	jle	.LBB363_55
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_55:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_61
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_55
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rax
	movq	952(%rsp), %rdx
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
.LBB363_58:
	cmpq	%rax, %rdx
	jle	.LBB363_60
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB363_58
.LBB363_60:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_61:
	movq	960(%rsp), %r15
	xorl	%r14d, %r14d
	.p2align	4
.LBB363_62:
	movq	160(%rsp), %rdi
	leaq	(%rbx,%r14), %rsi
	addq	$16, %rdi
.Ltmp15732:
	callq	<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.10720091597982897309)
.Ltmp15733:
	movq	176(%rsp), %rcx
	movq	%rax, (%rcx,%r14)
	movq	%rdx, 8(%rcx,%r14)
	addq	$16, %r14
	decq	%r15
	jne	.LBB363_62
	leaq	384(%rsp), %r11
	jmp	.LBB363_78
.LBB363_65:
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$51, %edi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB363_939
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rdx
	movq	120(%rsp), %rbx
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
	jle	.LBB363_68
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_68:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_74
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_68
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
.LBB363_71:
	cmpq	%rax, %rsi
	jle	.LBB363_73
	lock		cmpxchgq	%rsi, (%rdx)
	jne	.LBB363_71
.LBB363_73:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_74:
	vmovups	.Lanon.e5162873a9a3251d11c4df37a70e4654.528+19(%rip), %ymm0
	vmovups	.Lanon.e5162873a9a3251d11c4df37a70e4654.528(%rip), %ymm1
	movabsq	$9223372036854775793, %rax
	leaq	1632(%rsp), %rdi
	addq	$35, %rax
	movq	%rax, 16(%rbx)
	movq	$51, 24(%rbx)
	movq	%rcx, 32(%rbx)
	movq	$51, 40(%rbx)
	movq	$1, (%rbx)
	vmovups	%ymm0, 19(%rcx)
	vmovups	%ymm1, (%rcx)
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.10720091597982897309)
	jmp	.LBB363_554
.LBB363_75:
	vmovups	1776(%rsp), %zmm1
	vmovups	1736(%rsp), %zmm0
	movq	320(%rsp), %rax
	movq	%rax, 1288(%rsp)
	movq	$0, 1264(%rsp)
	movq	$8, 1272(%rsp)
	movq	$0, 1280(%rsp)
	vmovups	%zmm1, 424(%rsp)
	vmovups	%zmm0, 384(%rsp)
	cmpq	$-1, 384(%rsp)
	je	.LBB363_156
	leaq	720(%rsp), %rdi
	leaq	1264(%rsp), %rsi
	leaq	1736(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	456(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB363_157
	jmp	.LBB363_166
.LBB363_77:
	movl	$8, %eax
	movq	%rax, 176(%rsp)
.LBB363_78:
	vmovdqu	anon.e5162873a9a3251d11c4df37a70e4654.29.llvm.10720091597982897309(%rip), %ymm0
	movq	1168(%rsp), %rax
	vmovdqu	%ymm0, 1696(%rsp)
	testq	%rax, %rax
	je	.LBB363_128
	movq	1160(%rsp), %rbx
	movq	176(%rsp), %rdx
	movq	952(%rsp), %r15
	leaq	(%rax,%rax,4), %rax
	movl	$2, %ecx
	leaq	392(%rsp), %r13
	xorl	%r14d, %r14d
	vmovd	%ecx, %xmm0
	vmovdqa	%xmm0, 48(%rsp)
	leaq	(%rbx,%rax,8), %rax
	movq	%rax, 376(%rsp)
	leaq	(%rdx,%r15), %rax
	negq	%r15
	movq	%rax, 80(%rsp)
	jmp	.LBB363_82
	.p2align	4
.LBB363_80:
	movq	-16(%r12), %rax
	movq	72(%rsp), %rcx
	leaq	384(%rsp), %r11
	movq	%rcx, (%rax,%r14,8)
	incq	%r14
	movq	%r14, -8(%r12)
.LBB363_81:
	incq	%rcx
	addq	$40, %rbx
	movq	%rcx, %r14
	cmpq	376(%rsp), %rbx
	je	.LBB363_127
.LBB363_82:
	cmpq	$5, 960(%rsp)
	movq	$1, 384(%rsp)
	jae	.LBB363_122
	xorl	%eax, %eax
.LBB363_84:
	xorl	%r12d, %r12d
	cmpq	$5, %rax
	movl	$4, %edx
	movq	%r14, 72(%rsp)
	setae	%r12b
	cmovbq	%rdx, %rax
	cmovbq	%r13, %rcx
	shll	$4, %r12d
	movq	384(%rsp,%r12), %rbp
	leaq	-1(%rbp), %rdx
	cmpq	%rax, %rdx
	jae	.LBB363_105
	movq	176(%rsp), %r9
	leaq	8(%rbx), %rdx
	incq	%rax
	xorl	%r8d, %r8d
	jmp	.LBB363_89
	.p2align	4
.LBB363_86:
	movq	8(%r9), %rdi
	cmpq	%rsi, %rdi
	jae	.LBB363_890
	vmovq	(%r10,%rdi,8), %xmm0
.LBB363_88:
	vmovq	%xmm0, -8(%rcx,%rbp,8)
	addq	$16, %r9
	incq	%rbp
	addq	$-16, %r8
	cmpq	%rbp, %rax
	je	.LBB363_94
.LBB363_89:
	cmpq	%r8, %r15
	je	.LBB363_93
	vmovdqa	48(%rsp), %xmm0
	cmpl	$1, (%r9)
	jne	.LBB363_88
	movq	(%rbx), %rsi
	movq	%rdx, %r10
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB363_86
	movq	16(%rbx), %rsi
	movq	8(%rbx), %r10
	decq	%rsi
	jmp	.LBB363_86
	.p2align	4
.LBB363_105:
	movq	176(%rsp), %r14
	movq	%rbp, %rax
	movq	%rax, 384(%rsp,%r12)
	cmpq	80(%rsp), %r14
	jne	.LBB363_95
	jmp	.LBB363_106
	.p2align	4
.LBB363_93:
	movq	%rbp, 384(%rsp,%r12)
	jmp	.LBB363_106
	.p2align	4
.LBB363_94:
	movq	176(%rsp), %r14
	subq	%r8, %r14
	movq	%rax, 384(%rsp,%r12)
	cmpq	80(%rsp), %r14
	je	.LBB363_106
.LBB363_95:
	leaq	8(%rbx), %r12
	.p2align	4
.LBB363_96:
	vmovdqa	48(%rsp), %xmm0
	cmpl	$1, (%r14)
	vmovdqa	%xmm0, 96(%rsp)
	jne	.LBB363_101
	movq	(%rbx), %rsi
	movq	%r12, %rax
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB363_99
	movq	16(%rbx), %rsi
	movq	8(%rbx), %rax
	decq	%rsi
.LBB363_99:
	movq	8(%r14), %rdi
	cmpq	%rsi, %rdi
	jae	.LBB363_889
	vmovq	(%rax,%rdi,8), %xmm0
	vmovdqa	%xmm0, 96(%rsp)
.LBB363_101:
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
	je	.LBB363_103
.LBB363_102:
	vmovdqa	96(%rsp), %xmm0
	addq	$16, %r14
	vmovq	%xmm0, -8(%rax,%rbp,8)
	incq	%rbp
	movq	%rbp, (%rcx)
	cmpq	80(%rsp), %r14
	jne	.LBB363_96
	jmp	.LBB363_106
.LBB363_103:
.Ltmp15746:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	movl	$1, %edx
	movl	$1, %ecx
	movq	%r11, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15747:
	cmpq	$6, 384(%rsp)
	movq	392(%rsp), %rax
	leaq	384(%rsp), %r11
	leaq	400(%rsp), %rdx
	movq	%r11, %rcx
	cmovbq	%r13, %rax
	cmovaeq	%rdx, %rcx
	jmp	.LBB363_102
	.p2align	4
.LBB363_106:
	vmovdqu	384(%rsp), %ymm0
	movq	416(%rsp), %rax
	movq	1720(%rsp), %r14
	movq	%rax, 752(%rsp)
	vmovdqu	%ymm0, 720(%rsp)
.Ltmp15749:
	leaq	1696(%rsp), %rsi
	leaq	720(%rsp), %rdx
	movq	%r11, %rdi
	vzeroupper
	callq	<hashbrown::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>), purrdf_hash::fixed::FixedState>>::rustc_entry
.Ltmp15750:
	movq	384(%rsp), %rax
	movq	392(%rsp), %r12
	testq	%rax, %rax
	je	.LBB363_120
	vmovdqu	16(%r13), %xmm0
	movq	%rax, 96(%rsp)
	movq	400(%rsp), %rcx
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%r12, 152(%rsp)
	movq	424(%rsp), %r12
	movq	432(%rsp), %rbp
	movl	$8, %edi
	movq	%rcx, 336(%rsp)
	vmovdqa	%xmm0, 2000(%rsp)
	callq	*%rax
	testq	%rax, %rax
	je	.LBB363_933
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
	jle	.LBB363_111
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_111:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_117
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_111
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
.LBB363_114:
	cmpq	%rax, %rdx
	jle	.LBB363_116
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rsi
	lock		cmpxchgq	%rdx, (%rsi)
	jne	.LBB363_114
.LBB363_116:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_117:
	movq	72(%rsp), %rax
	movq	%rax, (%rcx)
	movq	8(%r12), %rdx
	movq	(%r12), %rax
	movq	%rdx, %rsi
	andq	%rbp, %rsi
	vmovdqu	(%rax,%rsi), %xmm0
	vpmovmskb	%xmm0, %edi
	testl	%edi, %edi
	je	.LBB363_124
.LBB363_118:
	tzcntl	%edi, %edi
	addq	%rsi, %rdi
	andq	%rdx, %rdi
	movzbl	(%rax,%rdi), %esi
	testb	%sil, %sil
	jns	.LBB363_126
.LBB363_119:
	shrq	$57, %rbp
	leaq	-16(%rdi), %r8
	andb	$1, %sil
	movb	%bpl, (%rax,%rdi)
	negq	%rdi
	andq	%rdx, %r8
	movzbl	%sil, %esi
	leaq	(%rdi,%rdi,8), %rdx
	movq	96(%rsp), %rdi
	movb	%bpl, 16(%rax,%r8)
	subq	%rsi, 16(%r12)
	movq	152(%rsp), %r8
	movq	%rdi, -72(%rax,%rdx,8)
	movq	336(%rsp), %rdi
	movq	%r8, -64(%rax,%rdx,8)
	movq	%rdi, -56(%rax,%rdx,8)
	vmovdqa	2000(%rsp), %xmm0
	vmovdqu	%xmm0, -48(%rax,%rdx,8)
	movq	%r14, -32(%rax,%rdx,8)
	movq	$1, -24(%rax,%rdx,8)
	movq	%rcx, -16(%rax,%rdx,8)
	movq	$1, -8(%rax,%rdx,8)
	incq	24(%r12)
	movq	72(%rsp), %rcx
	jmp	.LBB363_81
	.p2align	4
.LBB363_120:
	movq	-8(%r12), %r14
	cmpq	-24(%r12), %r14
	jne	.LBB363_80
.Ltmp15754:
	movq	<alloc::raw_vec::RawVec<usize>>::grow_one@GOTPCREL(%rip), %rax
	leaq	-24(%r12), %rdi
	callq	*%rax
.Ltmp15755:
	jmp	.LBB363_80
.LBB363_122:
.Ltmp15737:
	movq	960(%rsp), %rdx
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	movq	%r11, %rdi
	xorl	%esi, %esi
	vzeroupper
	callq	*%rax
.Ltmp15738:
	movq	384(%rsp), %rax
	movq	392(%rsp), %rcx
	leaq	384(%rsp), %r11
	decq	%rax
	jmp	.LBB363_84
.LBB363_124:
	movl	$16, %r8d
.LBB363_125:
	addq	%r8, %rsi
	addq	$16, %r8
	andq	%rdx, %rsi
	vmovdqu	(%rax,%rsi), %xmm0
	vpmovmskb	%xmm0, %edi
	testl	%edi, %edi
	jne	.LBB363_118
	jmp	.LBB363_125
.LBB363_126:
	vmovdqa	(%rax), %xmm0
	vpmovmskb	%xmm0, %esi
	xorl	%edi, %edi
	tzcntl	%esi, %edi
	movzbl	(%rax,%rdi), %esi
	jmp	.LBB363_119
.LBB363_127:
	movq	1720(%rsp), %rax
	orq	%rax, 128(%rsp)
	cmpq	$0, 2544(%rsp)
	jne	.LBB363_129
	jmp	.LBB363_142
.LBB363_128:
	xorl	%eax, %eax
	cmpq	$0, 2544(%rsp)
	je	.LBB363_142
.LBB363_129:
	cmpq	$0, 128(%rsp)
	jne	.LBB363_142
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqa	%xmm0, 720(%rsp)
	movq	$8, 736(%rsp)
	movq	$1, 384(%rsp)
	movq	$0, 744(%rsp)
.Ltmp15757:
	leaq	1264(%rsp), %rdi
	leaq	1696(%rsp), %rsi
	leaq	384(%rsp), %rdx
	leaq	720(%rsp), %rcx
	vzeroupper
	callq	<hashbrown::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>), purrdf_hash::fixed::FixedState>>::insert
.Ltmp15758:
	movq	1272(%rsp), %rcx
	testq	%rcx, %rcx
	jle	.LBB363_141
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$3, %rcx
	movabsq	$9223372036854775807, %rsi
	movq	1280(%rsp), %rdi
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
	jge	.LBB363_134
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_134:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_140
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_134
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
.LBB363_137:
	cmpq	%rax, %rdx
	jge	.LBB363_139
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB363_137
.LBB363_139:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_140:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_141:
	movq	1720(%rsp), %rax
.LBB363_142:
	movq	1696(%rsp), %rcx
	movq	1704(%rsp), %rsi
	vmovdqa	(%rcx), %xmm0
	testq	%rsi, %rsi
	je	.LBB363_144
	leaq	(,%rsi,8), %rdx
	movq	%rcx, %r8
	movl	$16, %r9d
	leaq	(%rdx,%rdx,8), %rdx
	andq	$-16, %rdx
	subq	%rdx, %r8
	leaq	97(%rdx,%rsi), %rdi
	addq	$-80, %r8
	jmp	.LBB363_145
.LBB363_144:
	xorl	%r9d, %r9d
.LBB363_145:
	vpcmpeqd	%xmm1, %xmm1, %xmm1
	leaq	1(%rcx,%rsi), %rsi
	leaq	16(%rcx), %rdx
	movq	%r9, 1264(%rsp)
	movq	%rdi, 1272(%rsp)
	movq	%r8, 1280(%rsp)
	movq	%rcx, 1288(%rsp)
	vpcmpgtb	%xmm1, %xmm0, %k0
	movq	%rdx, 1296(%rsp)
	movq	%rsi, 1304(%rsp)
	kmovw	%k0, 1312(%rsp)
	movq	%rax, 1320(%rsp)
	testq	%rax, %rax
	je	.LBB363_155
	kortestw	%k0, %k0
	je	.LBB363_148
	kmovd	%k0, %esi
	jmp	.LBB363_151
.LBB363_148:
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	.p2align	4
.LBB363_149:
	vpcmpltb	(%rdx), %xmm0, %k0
	addq	$-1152, %rcx
	addq	$16, %rdx
	kortestw	%k0, %k0
	je	.LBB363_149
	kmovd	%k0, %esi
	movq	%rdx, 1296(%rsp)
	movq	%rcx, 1288(%rsp)
.LBB363_151:
	xorl	%edx, %edx
	blsrl	%esi, %edx
	tzcntl	%esi, %esi
	leaq	-1(%rax), %rdi
	negq	%rsi
	movw	%dx, 1312(%rsp)
	movq	%rdi, 1320(%rsp)
	leaq	(%rsi,%rsi,8), %rdx
	movq	-24(%rcx,%rdx,8), %r14
	cmpq	$-1, %r14
	je	.LBB363_155
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
	movq	%r8, 2032(%rsp)
	movq	%rsi, 48(%rsp)
	vmovdqu	-56(%rcx), %xmm0
	vmovdqa	%xmm0, 2016(%rsp)
	movq	-16(%rcx), %rsi
	movq	%rsi, 96(%rsp)
	leaq	(,%rbp,8), %rsi
	leaq	(%rsi,%rsi,8), %rbx
	cmpq	%rdx, %rax
	jbe	.LBB363_170
	xorl	%r13d, %r13d
.LBB363_154:
.Ltmp15765:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
.Ltmp15766:
	jmp	.LBB363_944
.LBB363_155:
	leaq	1264(%rsp), %rdi
	movq	$0, 1072(%rsp)
	movq	$8, 1080(%rsp)
	movq	$0, 1088(%rsp)
	vzeroupper
	callq	core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>
	movl	$8, %eax
	xorl	%r15d, %r15d
	movq	%rax, 48(%rsp)
	jmp	.LBB363_186
.LBB363_156:
	movq	1272(%rsp), %rcx
	movq	1264(%rsp), %rax
	movq	1280(%rsp), %rdx
	movq	%rcx, 736(%rsp)
	movq	1288(%rsp), %rcx
	movq	%rax, 728(%rsp)
	movq	%rdx, 744(%rsp)
	movq	%rcx, 752(%rsp)
	movq	$-1, 720(%rsp)
	movq	456(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB363_166
.LBB363_157:
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
	jge	.LBB363_159
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_159:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_165
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_159
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
.LBB363_162:
	cmpq	%rax, %rdx
	jge	.LBB363_164
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB363_162
.LBB363_164:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_165:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB363_166:
	movq	480(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_169
	lock		decq	(%rax)
	jne	.LBB363_169
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	480(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB363_169:
	vmovdqu64	752(%rsp), %zmm1
	vmovdqu64	720(%rsp), %zmm0
	vmovdqu64	%zmm1, 40(%rbx)
	vmovdqu64	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
	jmp	.LBB363_554
.LBB363_170:
	movq	-8(%rcx), %r15
	testq	%rbx, %rbx
	je	.LBB363_172
	movl	$8, %esi
	movq	%rdi, 80(%rsp)
	movq	%rbx, %rdi
	movl	$8, %r13d
	vzeroupper
	callq	__rustc::__rust_alloc
	movq	80(%rsp), %rdi
	testq	%rax, %rax
	jne	.LBB363_173
	jmp	.LBB363_154
.LBB363_172:
	movl	$8, %eax
	xorl	%ebp, %ebp
.LBB363_173:
	movq	48(%rsp), %rcx
	movq	%r12, (%rax)
	movq	96(%rsp), %rdx
	movq	%rcx, 8(%rax)
	vmovaps	2016(%rsp), %xmm0
	vmovups	%xmm0, 16(%rax)
	movq	2032(%rsp), %rcx
	movq	%rcx, 32(%rax)
	movq	%rdi, 40(%rax)
	movq	%r14, 48(%rax)
	movq	%rdx, 56(%rax)
	movq	%r15, 64(%rax)
	movq	%rbp, 208(%rsp)
	movq	%rax, 216(%rsp)
	movq	$1, 224(%rsp)
	vmovdqu64	1264(%rsp), %zmm0
	vmovdqu64	%zmm0, 720(%rsp)
	movq	776(%rsp), %rdx
	testq	%rdx, %rdx
	je	.LBB363_185
	movzwl	768(%rsp), %ebp
	movq	744(%rsp), %r15
	movq	752(%rsp), %r12
	leaq	440(%rsp), %r13
	movl	$1, %ebx
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	jmp	.LBB363_176
	.p2align	4
.LBB363_175:
	movq	448(%rsp), %rdx
	leaq	(%rbx,%rbx,8), %rcx
	incq	%rbx
	movq	%rdx, 64(%rax,%rcx,8)
	movq	%r14, %rdx
	vmovdqu64	384(%rsp), %zmm0
	vmovdqu64	%zmm0, (%rax,%rcx,8)
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	movq	%rbx, 224(%rsp)
	testq	%r14, %r14
	je	.LBB363_183
.LBB363_176:
	testw	%bp, %bp
	jne	.LBB363_179
	.p2align	4
.LBB363_177:
	vpcmpltb	(%r12), %xmm0, %k0
	addq	$-1152, %r15
	addq	$16, %r12
	kortestw	%k0, %k0
	je	.LBB363_177
	kmovd	%k0, %ebp
.LBB363_179:
	xorl	%ecx, %ecx
	tzcntl	%ebp, %ecx
	leaq	-1(%rdx), %r14
	blsrl	%ebp, %ebp
	negq	%rcx
	leaq	(%rcx,%rcx,8), %rsi
	movq	-24(%r15,%rsi,8), %rcx
	cmpq	$-1, %rcx
	je	.LBB363_184
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
	cmpq	208(%rsp), %rbx
	jne	.LBB363_175
.Ltmp15760:
	movl	$8, %ecx
	movl	$72, %r8d
	leaq	208(%rsp), %rdi
	movq	%rbx, %rsi
	vzeroupper
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.10720091597982897309)
.Ltmp15761:
	movq	216(%rsp), %rax
	jmp	.LBB363_175
.LBB363_183:
	xorl	%r14d, %r14d
.LBB363_184:
	movq	%r12, 752(%rsp)
	movq	%r15, 744(%rsp)
	movw	%bp, 768(%rsp)
	movq	%r14, 776(%rsp)
.LBB363_185:
	leaq	720(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>
	vmovdqu	208(%rsp), %xmm0
	movq	224(%rsp), %r15
	movq	%r15, 1088(%rsp)
	vmovdqa	%xmm0, 1072(%rsp)
	movq	1080(%rsp), %rax
	movq	%rax, 48(%rsp)
	cmpq	$2, %r15
	jae	.LBB363_884
.LBB363_186:
	movq	320(%rsp), %rax
	movq	32(%rax), %rax
	movq	%rax, 1200(%rsp)
	movq	2552(%rsp), %rax
	cmpb	$2, 472(%rax)
	jne	.LBB363_189
	movq	616(%rax), %rax
	testq	%rax, %rax
	je	.LBB363_256
	cmpq	$-2, 24(%rax)
	jb	.LBB363_189
	cmpq	$-2, 32(%rax)
	jae	.LBB363_267
.LBB363_189:
	xorl	%ebx, %ebx
.LBB363_190:
	movq	160(%rsp), %r8
	addq	$16, %r8
.Ltmp15787:
	movq	1192(%rsp), %rsi
	movq	328(%rsp), %rdx
	movq	2544(%rsp), %rcx
	movq	2552(%rsp), %r9
	leaq	1128(%rsp), %r12
	movq	%r12, %rdi
	vzeroupper
	callq	purrdf_sparql_eval::modifier::link_aggregates::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp15788:
	testb	%bl, %bl
	je	.LBB363_197
	movq	2552(%rsp), %rbx
	movq	1040(%rbx), %rax
	movq	616(%rbx), %rsi
	addq	904(%rbx), %rax
	movq	%rax, 1928(%rsp)
.Ltmp15808:
	leaq	2016(%rsp), %r14
	movq	%r14, %rdi
	callq	<purrdf_sparql_eval::row_checkpoint::ItemLedger>::for_items::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp15809:
.Ltmp15810:
	movq	%rbx, %rdi
	movq	%r15, %rsi
	callq	<purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot
.Ltmp15811:
	movq	%rax, 624(%rsp)
	movq	616(%rbx), %rax
	testq	%rax, %rax
	je	.LBB363_260
	cmpq	$-2, 16(%rax)
	movb	$1, %cl
	jb	.LBB363_261
	cmpq	$-2, 40(%rax)
	setb	%cl
	jmp	.LBB363_261
.LBB363_197:
	testq	%r15, %r15
	je	.LBB363_258
	movq	malloc@GOTPCREL(%rip), %rbx
	leaq	(,%r15,8), %rax
	leaq	(%rax,%rax,4), %r14
	movq	%r14, %rdi
	callq	*%rbx
	testq	%rax, %rax
	je	.LBB363_943
	movq	%rax, %rbp
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rsi
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdi
	movabsq	$-9223372036854775808, %rcx
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
	jle	.LBB363_201
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_201:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_207
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_201
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
.LBB363_204:
	cmpq	%rax, %rdx
	jle	.LBB363_206
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB363_204
.LBB363_206:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_207:
	movq	48(%rsp), %r12
	addq	$16, 328(%rsp)
	leaq	(%r15,%r15,8), %rax
	movq	%r15, 1264(%rsp)
	movq	%rbp, 1272(%rsp)
	movq	$0, 1280(%rsp)
	movq	$0, 96(%rsp)
	leaq	(%r12,%rax,8), %rax
	movq	%rax, 80(%rsp)
	jmp	.LBB363_210
.LBB363_208:
	movq	1272(%rsp), %rbp
.LBB363_209:
	movq	96(%rsp), %rdx
	addq	$72, %r12
	leaq	(%rdx,%rdx,4), %rax
	incq	%rdx
	movq	%rdx, 96(%rsp)
	movq	%r13, (%rbp,%rax,8)
	movq	%r14, 8(%rbp,%rax,8)
	vmovdqa	384(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rbp,%rax,8)
	movq	400(%rsp), %rcx
	movq	%rcx, 32(%rbp,%rax,8)
	movq	%rdx, 1280(%rsp)
	cmpq	80(%rsp), %r12
	je	.LBB363_259
.LBB363_210:
	movq	1200(%rsp), %rsi
.Ltmp15790:
	leaq	720(%rsp), %rdi
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::from_elem
.Ltmp15791:
	movq	720(%rsp), %r13
	leaq	728(%rsp), %rdi
	movq	%r13, %rax
	cmpq	$6, %r13
	jb	.LBB363_213
	movq	728(%rsp), %rdi
	movq	736(%rsp), %rax
.LBB363_213:
	movq	968(%rsp), %rdx
	decq	%rax
	cmpq	%rax, %rdx
	ja	.LBB363_886
	movq	(%r12), %rsi
	decq	%rsi
	cmpq	$4, %rsi
	jbe	.LBB363_216
	movq	16(%r12), %rsi
	movq	8(%r12), %rax
	decq	%rsi
	jmp	.LBB363_217
.LBB363_216:
	leaq	8(%r12), %rax
.LBB363_217:
	movq	%rbp, 48(%rsp)
	cmpq	%rsi, %rdx
	jne	.LBB363_887
	movq	memcpy@GOTPCREL(%rip), %rbx
	shlq	$3, %rdx
	movq	%rax, %rsi
	callq	*%rbx
	movq	1144(%rsp), %r15
	movq	2544(%rsp), %rax
	cmpq	%r15, %rax
	cmovbq	%rax, %r15
	testq	%r15, %r15
	je	.LBB363_229
	movq	1136(%rsp), %rbp
	movq	328(%rsp), %r14
	xorl	%ebx, %ebx
	addq	$16, %rbp
	jmp	.LBB363_221
	.p2align	4
.LBB363_220:
	incq	%rbx
	addq	$24, %rbp
	addq	$120, %r14
	vmovq	%xmm0, (%rax,%rdi,8)
	cmpq	%rbx, %r15
	je	.LBB363_228
.LBB363_221:
	vmovups	1160(%rsp), %xmm0
	movq	160(%rsp), %rax
	movq	-8(%rbp), %rdx
	movq	(%rbp), %rcx
	movq	56(%r12), %r8
	movq	64(%r12), %r9
	addq	$16, %rax
.Ltmp15795:
	movq	2552(%rsp), %rsi
	leaq	384(%rsp), %rdi
	movq	%rax, 16(%rsp)
	movq	%rsi, 24(%rsp)
	movq	%r14, %rsi
	vmovups	%xmm0, (%rsp)
	callq	purrdf_sparql_eval::modifier::eval_aggregate::<purrdf_core::ir::dataset::RdfDataset, ()>
.Ltmp15796:
	vmovq	392(%rsp), %xmm0
	movq	384(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB363_231
	movq	720(%rsp), %r13
	movq	%r13, %rsi
	cmpq	$6, %r13
	jb	.LBB363_225
	movq	736(%rsp), %rsi
.LBB363_225:
	movq	968(%rsp), %rdi
	decq	%rsi
	addq	%rbx, %rdi
	cmpq	%rsi, %rdi
	jae	.LBB363_935
	leaq	728(%rsp), %rax
	cmpq	$6, %r13
	jb	.LBB363_220
	movq	728(%rsp), %rax
	jmp	.LBB363_220
.LBB363_228:
	movq	720(%rsp), %r13
.LBB363_229:
	leaq	728(%rsp), %rax
	movq	728(%rsp), %r14
	movq	120(%rsp), %rbx
	movq	48(%rsp), %rbp
	movq	96(%rsp), %rcx
	vmovups	8(%rax), %xmm0
	movq	24(%rax), %rax
	movq	%rax, 400(%rsp)
	vmovaps	%xmm0, 384(%rsp)
	cmpq	1264(%rsp), %rcx
	jne	.LBB363_209
.Ltmp15800:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	1264(%rsp), %rdi
	callq	*%rax
.Ltmp15801:
	jmp	.LBB363_208
.LBB363_231:
	vmovdqu64	400(%rsp), %zmm1
	vmovdqu64	416(%rsp), %zmm2
	movq	120(%rsp), %rcx
	vmovdqu64	%zmm2, 48(%rcx)
	vmovdqu64	%zmm1, 32(%rcx)
	movq	%rax, 16(%rcx)
	movq	720(%rsp), %rax
	vmovq	%xmm0, 24(%rcx)
	movq	$1, (%rcx)
	cmpq	$6, %rax
	jb	.LBB363_233
	movq	728(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB363_233:
	movq	96(%rsp), %rbp
	movq	48(%rsp), %r12
	testq	%rbp, %rbp
	je	.LBB363_246
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r14
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r15
	movq	free@GOTPCREL(%rip), %r13
	xorl	%ebx, %ebx
	jmp	.LBB363_238
	.p2align	4
.LBB363_235:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_236:
	vzeroupper
	callq	*%r13
.LBB363_237:
	incq	%rbx
	cmpq	%rbp, %rbx
	je	.LBB363_246
.LBB363_238:
	leaq	(%rbx,%rbx,4), %rcx
	movq	(%r12,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB363_237
	leaq	(%r12,%rcx,8), %rdx
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
	jge	.LBB363_241
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_241:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_236
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_241
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
.LBB363_244:
	cmpq	%rax, %rdx
	jge	.LBB363_235
	lock		cmpxchgq	%rdx, (%r15)
	jne	.LBB363_244
	jmp	.LBB363_235
.LBB363_246:
	movq	1264(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_468
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
	jge	.LBB363_249
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_249:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_255
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_249
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
.LBB363_252:
	cmpq	%rax, %rdx
	jge	.LBB363_254
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB363_252
.LBB363_254:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_255:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	vzeroupper
	callq	*%rax
	jmp	.LBB363_468
.LBB363_256:
	cmpq	$0, 2544(%rsp)
	jne	.LBB363_268
	movb	$1, %bl
	jmp	.LBB363_190
.LBB363_258:
	movq	120(%rsp), %rbx
	movq	$0, 1264(%rsp)
	movq	$8, 1272(%rsp)
	movq	$0, 1280(%rsp)
.LBB363_259:
	movq	1280(%rsp), %rax
	movq	1264(%rsp), %rdx
	movq	1272(%rsp), %rcx
	movq	%rax, 1488(%rsp)
	movq	%rdx, 1472(%rsp)
	movq	%rcx, 1480(%rsp)
	jmp	.LBB363_611
.LBB363_260:
	xorl	%ecx, %ecx
.LBB363_261:
	movq	2552(%rsp), %rdx
	leaq	624(%rsp), %rsi
	movq	%r12, 1840(%rsp)
	leaq	1200(%rsp), %rdi
	movq	%rdi, 208(%rsp)
	movq	328(%rsp), %rdi
	movq	%rdx, 1848(%rsp)
	movq	%rsi, 1856(%rsp)
	leaq	968(%rsp), %rsi
	movzbl	1234(%rdx), %eax
	movq	%r14, 1864(%rsp)
	movq	%rsi, 216(%rsp)
	movq	2544(%rsp), %rsi
	movq	%rdi, 224(%rsp)
	leaq	1152(%rsp), %rdi
	xorb	$1, %al
	movq	%rsi, 232(%rsp)
	leaq	160(%rsp), %rsi
	movq	%rdi, 240(%rsp)
	leaq	639(%rsp), %rdi
	movq	%rsi, 248(%rsp)
	leaq	1928(%rsp), %rsi
	movq	%rdi, 256(%rsp)
	movq	%rsi, 264(%rsp)
	testb	%cl, %cl
	je	.LBB363_264
	cmpq	$1025, %r15
	movq	%r14, 1560(%rsp)
	setae	%cl
	testb	%al, %cl
	jne	.LBB363_271
	movq	48(%rsp), %rax
	vmovdqu	1840(%rsp), %ymm0
	vmovdqu64	208(%rsp), %zmm1
	movq	%r14, 1008(%rsp)
	movq	%rax, 344(%rsp)
	leaq	1216(%rsp), %rax
	movq	%r15, 352(%rsp)
	movq	%rax, 640(%rsp)
	leaq	344(%rsp), %rax
	movq	%rax, 648(%rsp)
	leaq	720(%rsp), %rax
	movq	%rax, 656(%rsp)
	leaq	1008(%rsp), %rax
	vmovdqu	%ymm0, 1216(%rsp)
	vmovdqu64	%zmm1, 720(%rsp)
	movq	%rax, 664(%rsp)
.Ltmp15870:
	leaq	384(%rsp), %rdi
	leaq	640(%rsp), %rsi
	vzeroupper
	callq	purrdf_sparql_eval::parallel::par_chunk_try_map_init::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#6}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#7}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#8}>::{closure#0}
.Ltmp15871:
	jmp	.LBB363_333
.LBB363_264:
	movq	48(%rsp), %rcx
	leaq	1840(%rsp), %rbp
	cmpq	$1025, %r15
	leaq	1560(%rsp), %rsi
	leaq	208(%rsp), %rdi
	movq	%r14, 184(%rsp)
	movq	%rbp, 640(%rsp)
	movq	%rsi, 648(%rsp)
	leaq	184(%rsp), %rsi
	setae	%dl
	movq	%rdi, 656(%rsp)
	movq	%rsi, 664(%rsp)
	movq	%rcx, 1560(%rsp)
	movq	%r15, 1568(%rsp)
	testb	%al, %dl
	jne	.LBB363_273
.Ltmp15834:
	leaq	384(%rsp), %rdi
	leaq	640(%rsp), %rsi
	callq	purrdf_sparql_eval::parallel::par_chunk_try_map_init::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#6}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#7}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#8}>::{closure#0}
.Ltmp15835:
	jmp	.LBB363_333
.LBB363_271:
	movq	rayon_core::registry::WORKER_THREAD_STATE::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rbx
	movq	%fs:(%rbx), %rax
	testq	%rax, %rax
	je	.LBB363_307
	addq	$272, %rax
	jmp	.LBB363_308
.LBB363_273:
	movq	rayon_core::registry::WORKER_THREAD_STATE::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rax
	movq	%fs:(%rax), %rax
	testq	%rax, %rax
	je	.LBB363_310
	addq	$272, %rax
	movq	%r15, %r14
	jmp	.LBB363_312
.LBB363_267:
	cmpq	$0, 2544(%rsp)
	sete	%cl
	cmpq	$-2, 48(%rax)
	setb	%al
	setae	%bl
	orb	%cl, %al
	jne	.LBB363_190
.LBB363_268:
	movq	2552(%rsp), %rax
	movq	328(%rsp), %r12
	leaq	384(%rsp), %rbp
	addq	$584, %rax
	movq	%rax, 96(%rsp)
.LBB363_269:
	movq	56(%r12), %r13
	movq	96(%rsp), %rbx
	testq	%r13, %r13
	je	.LBB363_287
	movq	48(%r12), %r14
	shlq	$6, %r13
	jmp	.LBB363_276
	.p2align	4
.LBB363_275:
	addq	$64, %r14
	addq	$-64, %r13
	je	.LBB363_287
.LBB363_276:
	movq	2552(%rsp), %rax
	cmpq	$0, 584(%rax)
	movq	672(%rax), %rdx
	movq	680(%rax), %rcx
	je	.LBB363_279
	movq	%rbx, 384(%rsp)
.Ltmp15768:
	leaq	<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop::{closure#0}(%rip), %r9
	xorl	%edi, %edi
	movq	%r14, %rsi
	movq	%rbp, %r8
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.10720091597982897309)
.Ltmp15769:
	jmp	.LBB363_280
.LBB363_279:
.Ltmp15770:
	movl	$1, %r8d
	leaq	purrdf_sparql_eval::parallel::is_parallel_safe_pattern::{closure#0} (.llvm.10720091597982897309)(%rip), %r9
	xorl	%edi, %edi
	movq	%r14, %rsi
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.10720091597982897309)
.Ltmp15771:
.LBB363_280:
	testb	%al, %al
	jne	.LBB363_189
	movq	2552(%rsp), %rax
	movq	616(%rax), %rax
	testq	%rax, %rax
	je	.LBB363_275
	cmpq	$0, 336(%rax)
	jne	.LBB363_285
	vpcmpeqd	%ymm0, %ymm0, %ymm0
	vpcmpneqq	16(%rax), %ymm0, %k0
	kmovd	%k0, %ecx
	testb	$15, %cl
	jne	.LBB363_285
	cmpq	$-1, 48(%rax)
	je	.LBB363_275
.LBB363_285:
.Ltmp15772:
	movq	purrdf_sparql_eval::parallel::expression_re_enters_evaluation@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15773:
	testb	%al, %al
	je	.LBB363_275
	jmp	.LBB363_189
.LBB363_287:
	movq	104(%r12), %rax
	testq	%rax, %rax
	je	.LBB363_301
	movq	96(%r12), %r14
	shlq	$3, %rax
	leaq	(%rax,%rax,8), %r13
	addq	$8, %r14
	jmp	.LBB363_290
	.p2align	4
.LBB363_289:
	addq	$72, %r14
	addq	$-72, %r13
	je	.LBB363_301
.LBB363_290:
	movq	2552(%rsp), %rax
	cmpq	$0, 584(%rax)
	movq	672(%rax), %rdx
	movq	680(%rax), %rcx
	je	.LBB363_293
	movq	%rbx, 384(%rsp)
.Ltmp15775:
	leaq	<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop::{closure#0}(%rip), %r9
	xorl	%edi, %edi
	movq	%r14, %rsi
	movq	%rbp, %r8
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.10720091597982897309)
.Ltmp15776:
	jmp	.LBB363_294
.LBB363_293:
.Ltmp15777:
	movl	$1, %r8d
	leaq	purrdf_sparql_eval::parallel::is_parallel_safe_pattern::{closure#0} (.llvm.10720091597982897309)(%rip), %r9
	xorl	%edi, %edi
	movq	%r14, %rsi
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.10720091597982897309)
.Ltmp15778:
.LBB363_294:
	testb	%al, %al
	jne	.LBB363_189
	movq	2552(%rsp), %rax
	movq	616(%rax), %rax
	testq	%rax, %rax
	je	.LBB363_289
	cmpq	$0, 336(%rax)
	jne	.LBB363_299
	vpcmpeqd	%ymm0, %ymm0, %ymm0
	vpcmpneqq	16(%rax), %ymm0, %k0
	kmovd	%k0, %ecx
	testb	$15, %cl
	jne	.LBB363_299
	cmpq	$-1, 48(%rax)
	je	.LBB363_289
.LBB363_299:
.Ltmp15779:
	movq	purrdf_sparql_eval::parallel::expression_re_enters_evaluation@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15780:
	testb	%al, %al
	je	.LBB363_289
	jmp	.LBB363_189
.LBB363_301:
	cmpl	$8, 16(%r12)
	jne	.LBB363_306
	movq	2552(%rsp), %rax
	movq	24(%r12), %rsi
	movq	32(%r12), %rdx
	movq	688(%rax), %rdi
	addq	$16, %rsi
.Ltmp15782:
	movq	<purrdf_sparql_eval::agg_fn::AggregateRegistry>::resolve@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp15783:
	testq	%rax, %rax
	je	.LBB363_189
	movq	(%rax), %rcx
	movq	8(%rax), %rax
	movq	16(%rax), %rdx
	movq	32(%rax), %rax
	decq	%rdx
	andq	$-16, %rdx
	leaq	16(%rcx,%rdx), %rdi
.Ltmp15784:
	callq	*%rax
.Ltmp15785:
	testb	%al, %al
	jne	.LBB363_189
.LBB363_306:
	addq	$120, %r12
	movb	$1, %bl
	cmpq	40(%rsp), %r12
	jne	.LBB363_269
	jmp	.LBB363_190
.LBB363_307:
.Ltmp15836:
	movq	rayon_core::registry::global_registry@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15837:
.LBB363_308:
	movq	(%rax), %rax
	movq	520(%rax), %rbp
	movq	%r15, %rax
	cmpq	$1, %rbp
	adcq	$0, %rbp
	movq	%rbp, %rcx
	shlq	$6, %rcx
	orq	%rcx, %rax
	shrq	$32, %rax
	je	.LBB363_314
	movq	%r15, %rax
	xorl	%edx, %edx
	divq	%rcx
	jmp	.LBB363_315
.LBB363_310:
.Ltmp15812:
	movq	rayon_core::registry::global_registry@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15813:
	movq	1560(%rsp), %rcx
	movq	1568(%rsp), %r14
	movq	%rcx, 48(%rsp)
.LBB363_312:
	movq	(%rax), %rax
	movq	520(%rax), %rcx
	movq	%r15, %rax
	cmpq	$1, %rcx
	adcq	$0, %rcx
	shlq	$2, %rcx
	orq	%rcx, %rax
	shrq	$32, %rax
	je	.LBB363_321
	movq	%r15, %rax
	xorl	%edx, %edx
	divq	%rcx
	jmp	.LBB363_322
.LBB363_314:
	movl	%r15d, %eax
	xorl	%edx, %edx
	divl	%ecx
.LBB363_315:
	movq	48(%rsp), %rdx
	cmpq	$65, %rax
	movl	$64, %ecx
	cmovaeq	%rax, %rcx
	movq	%rdx, 720(%rsp)
	movq	%r15, 728(%rsp)
	movq	%rcx, 736(%rsp)
.Ltmp15838:
	leaq	1008(%rsp), %r14
	leaq	720(%rsp), %rsi
	movq	%r14, %rdi
	callq	<alloc::vec::Vec<&[(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)]> as alloc::vec::spec_from_iter::SpecFromIter<&[(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)], core::slice::iter::Chunks<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>>>::from_iter
.Ltmp15839:
	movq	1024(%rsp), %r13
	movabsq	$38430716820228232, %rax
	imulq	$240, %r13, %r15
	cmpq	%rax, %r13
	jbe	.LBB363_319
	xorl	%r12d, %r12d
.LBB363_318:
.Ltmp15867:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	movq	%r15, %rsi
	callq	*%rax
.Ltmp15868:
	jmp	.LBB363_944
.LBB363_319:
	testq	%r15, %r15
	je	.LBB363_370
	movl	$16, %esi
	movq	%r15, %rdi
	movl	$16, %r12d
	callq	__rustc::__rust_alloc
	movq	%r13, %rcx
	testq	%rax, %rax
	jne	.LBB363_371
	jmp	.LBB363_318
.LBB363_321:
	movl	%r15d, %eax
	xorl	%edx, %edx
	divl	%ecx
.LBB363_322:
	cmpq	$17, %rax
	movl	$16, %r12d
	movq	$0, 976(%rsp)
	movq	$16, 984(%rsp)
	movq	$0, 992(%rsp)
	cmovaeq	%rax, %r12
	movq	%r14, %rax
	orq	%r12, %rax
	shrq	$32, %rax
	je	.LBB363_324
	movq	%r14, %rax
	xorl	%edx, %edx
	divq	%r12
	jmp	.LBB363_325
.LBB363_324:
	movl	%r14d, %eax
	xorl	%edx, %edx
	divl	%r12d
.LBB363_325:
	xorl	%r15d, %r15d
	testq	%rdx, %rdx
	setne	%r15b
	addq	%rax, %r15
	movq	%r15, 1968(%rsp)
	jne	.LBB363_893
	xorl	%eax, %eax
	xorl	%r13d, %r13d
	subq	%r13, %rax
	cmpq	%r15, %rax
	jb	.LBB363_895
.LBB363_327:
	movq	48(%rsp), %rax
	leaq	208(%rsp), %rcx
	movq	%r12, 360(%rsp)
	movq	984(%rsp), %rbx
	movq	%rax, 720(%rsp)
	movq	%r14, 728(%rsp)
	movq	%r12, 736(%rsp)
	movq	%rbp, 744(%rsp)
	movq	%rcx, 752(%rsp)
	leaq	184(%rsp), %rcx
	movq	%rax, 344(%rsp)
	movq	%r14, 352(%rsp)
	movq	%rcx, 760(%rsp)
	movq	rayon_core::registry::WORKER_THREAD_STATE::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rcx
	movq	%fs:(%rcx), %rax
	testq	%rax, %rax
	je	.LBB363_329
	addq	$272, %rax
	jmp	.LBB363_330
.LBB363_329:
.Ltmp15816:
	movq	rayon_core::registry::global_registry@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15817:
.LBB363_330:
	movq	(%rax), %rax
	leaq	744(%rsp), %rdx
	movq	520(%rax), %rcx
	imulq	$224, %r13, %rax
	movq	%rdx, 1216(%rsp)
	addq	%rax, %rbx
	movq	%rbx, 1224(%rsp)
	movq	%r15, 1232(%rsp)
.Ltmp15818:
	leaq	1216(%rsp), %rbx
	leaq	1008(%rsp), %rdi
	leaq	344(%rsp), %r9
	movl	$1, %r8d
	movq	%r15, %rsi
	xorl	%edx, %edx
	movq	%rbx, (%rsp)
	callq	rayon::iter::plumbing::bridge_producer_consumer::helper::<rayon::slice::chunks::ChunksProducer<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>, rayon::iter::map::MapConsumer<rayon::iter::collect::consumer::CollectConsumer<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>, purrdf_sparql_eval::parallel::par_chunk_try_map_init<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#6}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#7}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#8}>::{closure#1}>>
.Ltmp15819:
	movq	1024(%rsp), %r14
	movq	%r14, 1216(%rsp)
	cmpq	%r15, %r14
	jne	.LBB363_896
	vmovdqu	976(%rsp), %xmm0
	addq	%r15, %r13
	movq	%r13, 736(%rsp)
	vmovdqa	%xmm0, 720(%rsp)
.Ltmp15826:
	leaq	384(%rsp), %rdi
	leaq	720(%rsp), %rsi
	callq	purrdf_sparql_eval::parallel::concatenate_chunks::<purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)>
.Ltmp15827:
.LBB363_333:
	movq	120(%rsp), %rbx
.LBB363_334:
	movq	384(%rsp), %rcx
	cmpq	$-1, %rcx
	je	.LBB363_347
	vmovups	544(%rsp), %zmm0
	vmovups	496(%rsp), %zmm2
	movq	408(%rsp), %rax
	vmovdqu	392(%rsp), %xmm1
	movq	424(%rsp), %rsi
	movq	416(%rsp), %rdx
	movq	400(%rsp), %rbx
	movq	%rcx, 1448(%rsp)
	movl	$1, %edi
	leaq	-3(%rax), %rcx
	cmpq	$-2, %rcx
	movl	$1, %ecx
	cmovbq	%rax, %rdi
	cmovbq	%rsi, %rax
	cmovaeq	%rsi, %rcx
	vmovups	%zmm0, 1376(%rsp)
	vmovups	%zmm2, 1328(%rsp)
	vmovdqu64	432(%rsp), %zmm0
	decq	%rax
	vmovdqu	%xmm1, 1456(%rsp)
	vmovdqu64	1376(%rsp), %zmm3
	vmovdqu64	1328(%rsp), %zmm2
	vmovdqu64	%zmm0, 1264(%rsp)
	vmovdqu64	%zmm0, 408(%rsp)
	vmovdqu64	%zmm3, 520(%rsp)
	vmovdqu64	%zmm2, 472(%rsp)
	movq	%rdi, 384(%rsp)
	movq	%rdx, 392(%rsp)
	movq	%rcx, 400(%rsp)
	movq	$0, 584(%rsp)
	movq	%rax, 592(%rsp)
.Ltmp15873:
	leaq	720(%rsp), %rdi
	leaq	384(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
.Ltmp15874:
	vmovups	752(%rsp), %zmm1
	movq	2552(%rsp), %rax
	vmovups	720(%rsp), %ymm0
	vmovdqu64	816(%rsp), %zmm2
	cmpq	$0, 616(%rax)
	vmovups	%zmm1, 2192(%rsp)
	vmovdqu64	864(%rsp), %zmm1
	vmovdqu64	%zmm2, 2256(%rsp)
	vmovups	%ymm0, 1968(%rsp)
	vmovdqu64	%zmm1, 2304(%rsp)
	je	.LBB363_348
	vmovdqu	1448(%rsp), %xmm0
	vmovdqu64	2192(%rsp), %zmm3
	vmovdqu64	2304(%rsp), %zmm2
	vmovdqu64	2256(%rsp), %zmm1
	addq	$888, %rax
	movzbl	2176(%rsp), %r14d
	movq	%rax, 96(%rsp)
	movq	1464(%rsp), %rax
	movq	%rax, 1520(%rsp)
	vmovdqu64	%zmm2, 832(%rsp)
	vmovdqa	%xmm0, 1504(%rsp)
	vmovdqu64	%zmm1, 784(%rsp)
	vmovdqu64	%zmm3, 720(%rsp)
	testb	%r14b, %r14b
	je	.LBB363_350
	movq	720(%rsp), %rax
	vmovdqu64	2216(%rsp), %zmm0
	vmovdqu64	2304(%rsp), %zmm2
	vmovdqu64	2280(%rsp), %zmm1
	movq	736(%rsp), %rcx
	movb	%r14b, 72(%rsp)
	movq	728(%rsp), %r14
	movl	$1, %edx
	movl	$1, %esi
	movq	$0, 208(%rsp)
	movq	$8, 216(%rsp)
	movq	$0, 224(%rsp)
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
	je	.LBB363_359
	cmpq	$3, %rax
	leaq	1272(%rsp), %r12
	movl	$8, %ecx
	cmovbq	%rdx, %r14
	xorl	%ebx, %ebx
	xorl	%ebp, %ebp
	addq	$8, %r14
.LBB363_340:
	leaq	1(%rbx), %r13
	movq	%r13, 560(%rsp)
	movq	-8(%r14), %rax
	cmpq	$-1, %rax
	je	.LBB363_391
	vmovups	(%r14), %zmm0
	vmovups	64(%r14), %zmm1
	vmovups	88(%r14), %zmm2
	vmovups	%zmm2, 88(%r12)
	vmovups	%zmm1, 64(%r12)
	vmovups	%zmm0, (%r12)
	movq	%rax, 1264(%rsp)
	movq	%rbx, %rax
	movzbl	1416(%rsp), %ebx
	cmpq	208(%rsp), %rax
	jne	.LBB363_344
.Ltmp15903:
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15904:
	movq	216(%rsp), %rcx
.LBB363_344:
	vmovdqu64	1264(%rsp), %zmm0
	vmovdqu64	1328(%rsp), %zmm1
	vmovdqu64	1360(%rsp), %zmm2
	vmovdqu64	%zmm2, 96(%rcx,%rbp)
	vmovdqu64	%zmm1, 64(%rcx,%rbp)
	vmovdqu64	%zmm0, (%rcx,%rbp)
	movq	%r13, 224(%rsp)
	testb	%bl, %bl
	jne	.LBB363_392
	addq	$160, %rbp
	addq	$168, %r14
	movq	%r13, %rbx
	cmpq	%r13, %r15
	jne	.LBB363_340
	xorl	%ebp, %ebp
	movq	%r15, %rbx
	jmp	.LBB363_393
.LBB363_347:
	vmovdqu64	432(%rsp), %zmm0
	vmovdqu	400(%rsp), %ymm1
	vmovdqu64	%zmm0, 48(%rbx)
	vmovdqu	%ymm1, 16(%rbx)
	vmovdqu64	%zmm0, 1264(%rsp)
	movq	$1, (%rbx)
	movq	624(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB363_465
	jmp	.LBB363_467
.LBB363_348:
	leaq	1160(%rsp), %rcx
	movq	1152(%rsp), %rax
	vmovups	(%rcx), %xmm0
	movq	$0, 1152(%rsp)
	movq	$8, 1160(%rsp)
	movq	$0, 1168(%rsp)
	vmovaps	%xmm0, 720(%rsp)
	cmpq	%rbx, %rax
	jbe	.LBB363_354
	vmovdqa	720(%rsp), %xmm0
	leaq	384(%rsp), %rdi
	movq	%rax, 384(%rsp)
	vmovdqu	%xmm0, 392(%rsp)
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	$8, 1952(%rsp)
	movq	$0, 1960(%rsp)
	xorl	%eax, %eax
	jmp	.LBB363_355
.LBB363_350:
	movq	1520(%rsp), %rbx
	movq	$0, 1216(%rsp)
	movq	$8, 1224(%rsp)
	movq	$0, 1232(%rsp)
.Ltmp15878:
	leaq	384(%rsp), %rdi
	leaq	1216(%rsp), %rsi
	movq	%rbx, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp15879:
	movq	384(%rsp), %r13
	movq	392(%rsp), %rax
	movq	400(%rsp), %rdx
	movq	408(%rsp), %rbp
	cmpq	$-1, %r13
	je	.LBB363_360
	vmovdqu	432(%rsp), %ymm0
	vmovdqu	448(%rsp), %ymm1
	movq	%rax, 96(%rsp)
	movq	424(%rsp), %rax
	movq	416(%rsp), %r12
	movq	%rdx, 48(%rsp)
	movq	%rax, 1064(%rsp)
	vmovdqu	%ymm0, 1264(%rsp)
	vmovdqu	%ymm1, 1280(%rsp)
.Ltmp15883:
	leaq	1448(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15884:
	movq	120(%rsp), %rbx
.LBB363_353:
	vmovups	1264(%rsp), %ymm0
	vmovups	1280(%rsp), %ymm1
	movq	%r12, 72(%rsp)
	shrq	$8, %r12
	vmovups	%ymm0, 1840(%rsp)
	vmovups	%ymm1, 1856(%rsp)
	jmp	.LBB363_439
.LBB363_354:
	vmovdqa	720(%rsp), %xmm0
	vmovdqu	%xmm0, 1952(%rsp)
.LBB363_355:
	movl	2176(%rsp), %esi
	movq	%rax, 1944(%rsp)
.Ltmp16041:
	movq	2552(%rsp), %rdx
	leaq	384(%rsp), %rdi
	leaq	1448(%rsp), %rcx
	leaq	2192(%rsp), %r8
	leaq	1944(%rsp), %r9
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::ItemLedger>::commit_into::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#9}>
.Ltmp16042:
	movq	384(%rsp), %rax
	movq	392(%rsp), %rsi
	movq	400(%rsp), %rdx
	movq	408(%rsp), %rbp
	movq	424(%rsp), %r14
	movq	416(%rsp), %rbx
	cmpq	$-1, %rax
	je	.LBB363_358
	vmovdqu	432(%rsp), %ymm0
	vmovdqu	448(%rsp), %ymm1
	movq	120(%rsp), %rcx
	vmovdqu	%ymm1, 80(%rcx)
	vmovdqu	%ymm0, 64(%rcx)
	movq	%rax, 16(%rcx)
	movq	%rsi, 24(%rcx)
	movq	%rdx, 32(%rcx)
	movq	%rbp, 40(%rcx)
	movq	%rbx, 48(%rcx)
	movq	%r14, 56(%rcx)
	movq	$1, (%rcx)
	jmp	.LBB363_463
.LBB363_358:
	movq	%rsi, 96(%rsp)
	movq	%rdx, 48(%rsp)
	jmp	.LBB363_602
.LBB363_359:
	xorl	%ebx, %ebx
	movl	$8, %r14d
	xorl	%ebp, %ebp
	jmp	.LBB363_394
.LBB363_360:
	movb	%r14b, 72(%rsp)
	movq	1512(%rsp), %r14
	movq	%rax, 640(%rsp)
	leaq	(,%rbx,8), %rcx
	movq	1504(%rsp), %rax
	movq	%rdx, 648(%rsp)
	movq	%rbp, 656(%rsp)
	leaq	(%rcx,%rcx,4), %r12
	leaq	(%r14,%r12), %rcx
	movq	%r14, 208(%rsp)
	movq	%rax, 224(%rsp)
	movq	%rcx, 232(%rsp)
	testq	%rbx, %rbx
	je	.LBB363_437
	leaq	(,%rbp,8), %rax
	addq	$40, %r14
	movq	%rcx, 40(%rsp)
	leaq	(%rax,%rax,4), %r15
	jmp	.LBB363_363
.LBB363_362:
	vmovdqa	80(%rsp), %xmm0
	movq	48(%rsp), %rax
	movq	%rbx, (%rdx,%r15)
	incq	%rbp
	addq	$40, %r14
	movq	%rax, 8(%rdx,%r15)
	vmovdqu	%xmm0, 16(%rdx,%r15)
	movq	%r13, 32(%rdx,%r15)
	addq	$40, %r15
	addq	$-40, %r12
	movq	%rbp, 656(%rsp)
	je	.LBB363_436
.LBB363_363:
	movq	2552(%rsp), %rcx
	leaq	1272(%rsp), %rsi
	movq	%rcx, 1264(%rsp)
	movq	-8(%r14), %rax
	movq	%rax, 32(%rsi)
	vmovdqu	-40(%r14), %ymm0
	vmovdqu	%ymm0, (%rsi)
	cmpq	$0, 1272(%rsp)
	je	.LBB363_365
	leaq	-40(%r14), %rax
	leaq	392(%rsp), %rsi
	movq	32(%rax), %rcx
	movq	%rcx, 32(%rsi)
	vmovdqu	(%rax), %ymm0
	vmovdqu	%ymm0, (%rsi)
	jmp	.LBB363_367
.LBB363_365:
	movq	%rdx, %r13
	movq	664(%rcx), %rdx
.Ltmp15886:
	movq	96(%rsp), %rsi
	leaq	384(%rsp), %rdi
	leaq	1280(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp15887:
	movq	384(%rsp), %rax
	movq	%r13, %rdx
	cmpq	$-1, %rax
	jne	.LBB363_575
.LBB363_367:
	vmovdqu	408(%rsp), %xmm0
	movq	400(%rsp), %rax
	movq	392(%rsp), %rbx
	movq	424(%rsp), %r13
	movq	%rax, 48(%rsp)
	vmovdqa	%xmm0, 80(%rsp)
	cmpq	640(%rsp), %rbp
	jne	.LBB363_362
.Ltmp15891:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	640(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15892:
	movq	648(%rsp), %rdx
	jmp	.LBB363_362
.LBB363_370:
	movl	$16, %eax
	xorl	%ecx, %ecx
.LBB363_371:
	testq	%r13, %r13
	je	.LBB363_374
	movl	%r13d, %edx
	andl	$7, %edx
	movq	%rbx, %r9
	cmpq	$8, %r13
	jae	.LBB363_375
	xorl	%esi, %esi
	leaq	720(%rsp), %rbx
	jmp	.LBB363_378
.LBB363_374:
	movq	%rbx, %r9
	xorl	%r15d, %r15d
	leaq	720(%rsp), %rbx
	jmp	.LBB363_381
.LBB363_375:
	movabsq	$72057594037927928, %rdi
	leaq	1696(%rax), %r8
	leaq	720(%rsp), %rbx
	xorl	%esi, %esi
	andq	%r13, %rdi
.LBB363_376:
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
	jne	.LBB363_376
	testq	%rdx, %rdx
	je	.LBB363_380
.LBB363_378:
	imulq	$240, %rsi, %rsi
	imulq	$240, %rdx, %rdx
	xorl	%edi, %edi
	addq	%rax, %rsi
.LBB363_379:
	movl	$0, (%rsi,%rdi)
	movb	$0, 4(%rsi,%rdi)
	movq	$2, 16(%rsi,%rdi)
	addq	$240, %rdi
	cmpq	%rdi, %rdx
	jne	.LBB363_379
.LBB363_380:
	movq	1024(%rsp), %r15
.LBB363_381:
	movq	%rcx, 344(%rsp)
	movq	%rax, 352(%rsp)
	movq	%fs:(%r9), %rax
	leaq	976(%rsp), %rcx
	cmpq	%r15, %rbp
	leaq	1840(%rsp), %rdx
	movq	%r13, 360(%rsp)
	movq	$0, 976(%rsp)
	movq	%rcx, 720(%rsp)
	leaq	208(%rsp), %rcx
	movq	%r14, 728(%rsp)
	movq	%rdx, 736(%rsp)
	cmovbq	%rbp, %r15
	leaq	1560(%rsp), %rdx
	movq	%rcx, 744(%rsp)
	leaq	344(%rsp), %rcx
	movq	%rdx, 752(%rsp)
	movq	%rcx, 760(%rsp)
	testq	%rax, %rax
	je	.LBB363_383
	addq	$272, %rax
	jmp	.LBB363_384
.LBB363_383:
.Ltmp15840:
	movq	rayon_core::registry::global_registry@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15841:
.LBB363_384:
	movq	(%rax), %rax
	movq	520(%rax), %rdx
.Ltmp15842:
	movl	$1, %ecx
	movq	%rbx, (%rsp)
	movq	%r15, %rdi
	xorl	%esi, %esi
	xorl	%r8d, %r8d
	movq	%r15, %r9
	callq	rayon::iter::plumbing::bridge_producer_consumer::helper::<rayon::range::IterProducer<usize>, rayon::iter::for_each::ForEachConsumer<purrdf_sparql_eval::parallel::par_blocks_try_map_init<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#6}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#7}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#8}>::{closure#1}>>
.Ltmp15843:
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
	je	.LBB363_421
	addq	$-240, %rcx
	movq	%rcx, %rdx
	mulxq	%rsi, %rdx, %rdx
	shrl	$7, %edx
	incl	%edx
	andl	$7, %edx
	je	.LBB363_401
	imulq	$240, %rdx, %r8
	movq	%rbx, %rdi
	movq	%rbx, %rdx
	jmp	.LBB363_389
.LBB363_388:
	addq	$240, %rdi
	addq	$-240, %r8
	je	.LBB363_402
.LBB363_389:
	vmovdqu64	24(%rdi), %zmm0
	vmovdqu64	88(%rdi), %zmm1
	vmovdqu64	152(%rdi), %zmm2
	vmovdqu64	176(%rdi), %zmm3
	movq	16(%rdi), %r9
	vmovdqu64	%zmm3, 872(%rsp)
	vmovdqu64	%zmm2, 848(%rsp)
	vmovdqu64	%zmm1, 784(%rsp)
	vmovdqu64	%zmm0, 720(%rsp)
	cmpq	$2, %r9
	je	.LBB363_388
	movq	%r9, (%rdx)
	vmovdqu64	720(%rsp), %zmm0
	vmovdqu64	784(%rsp), %zmm1
	vmovdqu64	848(%rsp), %zmm2
	vmovdqu64	872(%rsp), %zmm3
	vmovdqu64	%zmm2, 136(%rdx)
	vmovdqu64	%zmm0, 8(%rdx)
	vmovdqu64	%zmm1, 72(%rdx)
	vmovdqu64	%zmm3, 160(%rdx)
	addq	$224, %rdx
	jmp	.LBB363_388
.LBB363_391:
	xorl	%ebp, %ebp
	jmp	.LBB363_393
.LBB363_392:
	movb	$1, %bpl
	movq	%r13, %rbx
.LBB363_393:
	movq	%rcx, %r14
.LBB363_394:
.Ltmp15911:
	leaq	384(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
.Ltmp15912:
	movq	208(%rsp), %rsi
	testq	%rbx, %rbx
	je	.LBB363_398
	cmpq	$8, %rbx
	jae	.LBB363_399
	xorl	%eax, %eax
	xorl	%edx, %edx
	jmp	.LBB363_448
.LBB363_398:
	xorl	%edx, %edx
	jmp	.LBB363_450
.LBB363_399:
	cmpq	$32, %rbx
	jae	.LBB363_441
	xorl	%eax, %eax
	xorl	%edx, %edx
	jmp	.LBB363_445
.LBB363_401:
	movq	%rbx, %rdi
	movq	%rbx, %rdx
.LBB363_402:
	movq	%rax, %r14
	cmpq	$1680, %rcx
	jae	.LBB363_404
	jmp	.LBB363_421
.LBB363_403:
	addq	$1920, %rdi
	cmpq	%rax, %rdi
	je	.LBB363_420
.LBB363_404:
	vmovups	24(%rdi), %zmm0
	vmovups	88(%rdi), %zmm1
	vmovups	152(%rdi), %zmm2
	vmovups	176(%rdi), %zmm3
	movq	16(%rdi), %rcx
	vmovups	%zmm3, 872(%rsp)
	vmovups	%zmm2, 848(%rsp)
	vmovups	%zmm1, 784(%rsp)
	vmovups	%zmm0, 720(%rsp)
	cmpq	$2, %rcx
	je	.LBB363_406
	movq	%rcx, (%rdx)
	vmovups	720(%rsp), %zmm0
	vmovups	784(%rsp), %zmm1
	vmovups	848(%rsp), %zmm2
	vmovups	872(%rsp), %zmm3
	vmovups	%zmm2, 136(%rdx)
	vmovups	%zmm0, 8(%rdx)
	vmovups	%zmm1, 72(%rdx)
	vmovups	%zmm3, 160(%rdx)
	addq	$224, %rdx
.LBB363_406:
	vmovups	264(%rdi), %zmm0
	vmovups	328(%rdi), %zmm1
	vmovups	392(%rdi), %zmm2
	vmovups	416(%rdi), %zmm3
	movq	256(%rdi), %rcx
	vmovups	%zmm3, 872(%rsp)
	vmovups	%zmm2, 848(%rsp)
	vmovups	%zmm1, 784(%rsp)
	vmovups	%zmm0, 720(%rsp)
	cmpq	$2, %rcx
	je	.LBB363_408
	movq	%rcx, (%rdx)
	vmovups	720(%rsp), %zmm0
	vmovups	784(%rsp), %zmm1
	vmovups	848(%rsp), %zmm2
	vmovups	872(%rsp), %zmm3
	vmovups	%zmm2, 136(%rdx)
	vmovups	%zmm0, 8(%rdx)
	vmovups	%zmm1, 72(%rdx)
	vmovups	%zmm3, 160(%rdx)
	addq	$224, %rdx
.LBB363_408:
	vmovups	504(%rdi), %zmm0
	vmovups	568(%rdi), %zmm1
	vmovups	632(%rdi), %zmm2
	vmovups	656(%rdi), %zmm3
	movq	496(%rdi), %rcx
	vmovups	%zmm3, 872(%rsp)
	vmovups	%zmm2, 848(%rsp)
	vmovups	%zmm1, 784(%rsp)
	vmovups	%zmm0, 720(%rsp)
	cmpq	$2, %rcx
	je	.LBB363_410
	movq	%rcx, (%rdx)
	vmovups	720(%rsp), %zmm0
	vmovups	784(%rsp), %zmm1
	vmovups	848(%rsp), %zmm2
	vmovups	872(%rsp), %zmm3
	vmovups	%zmm2, 136(%rdx)
	vmovups	%zmm0, 8(%rdx)
	vmovups	%zmm1, 72(%rdx)
	vmovups	%zmm3, 160(%rdx)
	addq	$224, %rdx
.LBB363_410:
	vmovups	744(%rdi), %zmm0
	vmovups	808(%rdi), %zmm1
	vmovups	872(%rdi), %zmm2
	vmovups	896(%rdi), %zmm3
	movq	736(%rdi), %rcx
	vmovups	%zmm3, 872(%rsp)
	vmovups	%zmm2, 848(%rsp)
	vmovups	%zmm1, 784(%rsp)
	vmovups	%zmm0, 720(%rsp)
	cmpq	$2, %rcx
	je	.LBB363_412
	movq	%rcx, (%rdx)
	vmovups	720(%rsp), %zmm0
	vmovups	784(%rsp), %zmm1
	vmovups	848(%rsp), %zmm2
	vmovups	872(%rsp), %zmm3
	vmovups	%zmm2, 136(%rdx)
	vmovups	%zmm0, 8(%rdx)
	vmovups	%zmm1, 72(%rdx)
	vmovups	%zmm3, 160(%rdx)
	addq	$224, %rdx
.LBB363_412:
	vmovups	984(%rdi), %zmm0
	vmovups	1048(%rdi), %zmm1
	vmovups	1112(%rdi), %zmm2
	vmovups	1136(%rdi), %zmm3
	movq	976(%rdi), %rcx
	vmovups	%zmm3, 872(%rsp)
	vmovups	%zmm2, 848(%rsp)
	vmovups	%zmm1, 784(%rsp)
	vmovups	%zmm0, 720(%rsp)
	cmpq	$2, %rcx
	je	.LBB363_414
	movq	%rcx, (%rdx)
	vmovups	720(%rsp), %zmm0
	vmovups	784(%rsp), %zmm1
	vmovups	848(%rsp), %zmm2
	vmovups	872(%rsp), %zmm3
	vmovups	%zmm2, 136(%rdx)
	vmovups	%zmm0, 8(%rdx)
	vmovups	%zmm1, 72(%rdx)
	vmovups	%zmm3, 160(%rdx)
	addq	$224, %rdx
.LBB363_414:
	vmovups	1224(%rdi), %zmm0
	vmovups	1288(%rdi), %zmm1
	vmovups	1352(%rdi), %zmm2
	vmovups	1376(%rdi), %zmm3
	movq	1216(%rdi), %rcx
	vmovups	%zmm3, 872(%rsp)
	vmovups	%zmm2, 848(%rsp)
	vmovups	%zmm1, 784(%rsp)
	vmovups	%zmm0, 720(%rsp)
	cmpq	$2, %rcx
	je	.LBB363_416
	movq	%rcx, (%rdx)
	vmovups	720(%rsp), %zmm0
	vmovups	784(%rsp), %zmm1
	vmovups	848(%rsp), %zmm2
	vmovups	872(%rsp), %zmm3
	vmovups	%zmm2, 136(%rdx)
	vmovups	%zmm0, 8(%rdx)
	vmovups	%zmm1, 72(%rdx)
	vmovups	%zmm3, 160(%rdx)
	addq	$224, %rdx
.LBB363_416:
	vmovups	1464(%rdi), %zmm0
	vmovups	1528(%rdi), %zmm1
	vmovups	1592(%rdi), %zmm2
	vmovups	1616(%rdi), %zmm3
	movq	1456(%rdi), %rcx
	vmovups	%zmm3, 872(%rsp)
	vmovups	%zmm2, 848(%rsp)
	vmovups	%zmm1, 784(%rsp)
	vmovups	%zmm0, 720(%rsp)
	cmpq	$2, %rcx
	je	.LBB363_418
	movq	%rcx, (%rdx)
	vmovups	720(%rsp), %zmm0
	vmovups	784(%rsp), %zmm1
	vmovups	848(%rsp), %zmm2
	vmovups	872(%rsp), %zmm3
	vmovups	%zmm2, 136(%rdx)
	vmovups	%zmm0, 8(%rdx)
	vmovups	%zmm1, 72(%rdx)
	vmovups	%zmm3, 160(%rdx)
	addq	$224, %rdx
.LBB363_418:
	vmovdqu64	1704(%rdi), %zmm0
	vmovdqu64	1768(%rdi), %zmm1
	vmovdqu64	1832(%rdi), %zmm2
	vmovdqu64	1856(%rdi), %zmm3
	movq	1696(%rdi), %rcx
	vmovdqu64	%zmm3, 872(%rsp)
	vmovdqu64	%zmm2, 848(%rsp)
	vmovdqu64	%zmm1, 784(%rsp)
	vmovdqu64	%zmm0, 720(%rsp)
	cmpq	$2, %rcx
	je	.LBB363_403
	movq	%rcx, (%rdx)
	vmovdqu64	720(%rsp), %zmm0
	vmovdqu64	784(%rsp), %zmm1
	vmovdqu64	848(%rsp), %zmm2
	vmovdqu64	872(%rsp), %zmm3
	vmovdqu64	%zmm2, 136(%rdx)
	vmovdqu64	%zmm0, 8(%rdx)
	vmovdqu64	%zmm1, 72(%rdx)
	vmovdqu64	%zmm3, 160(%rdx)
	addq	$224, %rdx
	jmp	.LBB363_403
.LBB363_420:
	movq	%rax, %r14
.LBB363_421:
	vmovdqa	.LCPI363_0(%rip), %ymm0
	subq	%rbx, %rdx
	movabsq	$7905747460161236407, %rbp
	movq	%r11, 96(%rsp)
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
	vmovdqu	%ymm0, 720(%rsp)
	je	.LBB363_426
	shrq	$7, %rax
	movl	$1, %r12d
	addq	$256, %r14
	subq	%rax, %r12
	jmp	.LBB363_424
	.p2align	4
.LBB363_423:
	addq	$240, %r14
	incq	%r12
	cmpq	$1, %r12
	je	.LBB363_426
.LBB363_424:
	cmpl	$2, -240(%r14)
	je	.LBB363_423
.Ltmp15848:
	leaq	-240(%r14), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>
.Ltmp15849:
	jmp	.LBB363_423
.LBB363_426:
	movq	96(%rsp), %rcx
	testq	%rbx, %rbx
	setne	%al
	imulq	$224, %r15, %r14
	cmpq	%r14, %rcx
	setne	%dl
	andb	%al, %dl
	cmpb	$1, %dl
	jne	.LBB363_432
	cmpq	$223, %rcx
	ja	.LBB363_431
	testq	%rcx, %rcx
	je	.LBB363_430
	movl	$16, %edx
	movq	%r13, %rdi
	movq	%rcx, %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB363_430:
	movl	$16, %r13d
	jmp	.LBB363_432
.LBB363_431:
	movq	<purrdf_alloc_probe::CountingAllocator as core::alloc::global::GlobalAlloc>::realloc@GOTPCREL(%rip), %rax
	leaq	qualification_454_native_cost::GLOBAL (.llvm.10137715445955899992)(%rip), %rdi
	movl	$16, %edx
	movq	%r13, %rsi
	movq	%r14, %r8
	vzeroupper
	callq	*%rax
	movq	%rax, %r13
	testq	%rax, %rax
	je	.LBB363_934
.LBB363_432:
	movq	%r15, 1216(%rsp)
	movq	%r13, 1224(%rsp)
	movq	%rbp, 1232(%rsp)
.Ltmp15862:
	leaq	720(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>>>
.Ltmp15863:
.Ltmp15864:
	leaq	384(%rsp), %rdi
	leaq	1216(%rsp), %rsi
	callq	purrdf_sparql_eval::parallel::concatenate_chunks::<purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)>
.Ltmp15865:
	movq	1008(%rsp), %rsi
	movq	120(%rsp), %rbx
	testq	%rsi, %rsi
	je	.LBB363_334
	movq	1016(%rsp), %rdi
	shlq	$4, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB363_334
.LBB363_436:
	movq	40(%rsp), %r14
.LBB363_437:
	movq	%r14, 216(%rsp)
.Ltmp15897:
	leaq	208(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15898:
	movq	120(%rsp), %rbx
	movq	640(%rsp), %rax
	movq	648(%rsp), %rcx
	movq	$-1, %r13
	xorl	%r12d, %r12d
	movq	$0, 72(%rsp)
	movq	%rax, 96(%rsp)
	movq	%rcx, 48(%rsp)
.LBB363_439:
.Ltmp15900:
	leaq	720(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
.Ltmp15901:
	cmpq	$-1, %r13
	jne	.LBB363_462
	jmp	.LBB363_601
.LBB363_441:
	vmovdqa64	.LCPI363_1(%rip), %zmm1
	vpbroadcastq	.LCPI363_2(%rip), %zmm2
	vpbroadcastq	.LCPI363_3(%rip), %zmm3
	movq	%rbx, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
.LBB363_442:
	vpmullq	%zmm2, %zmm1, %zmm7
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm8, %xmm8, %xmm8
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$-32, %rcx
	vpgatherqq	64(%r14,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm0, %zmm8, %zmm0
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	1344(%r14,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm4, %zmm8, %zmm4
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	2624(%r14,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm5, %zmm8, %zmm5
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	3904(%r14,%zmm7), %zmm8 {%k1}
	vpaddq	%zmm6, %zmm8, %zmm6
	jne	.LBB363_442
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
	cmpq	%rax, %rbx
	je	.LBB363_450
	testb	$24, %bl
	je	.LBB363_448
.LBB363_445:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI363_1(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI363_2(%rip), %zmm2
	vpbroadcastq	.LCPI363_4(%rip), %zmm3
	movq	%rbx, %rax
	andq	$-8, %rax
	vmovq	%rdx, %xmm0
	subq	%rax, %rcx
.LBB363_446:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	64(%r14,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB363_446
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rdx
	cmpq	%rax, %rbx
	je	.LBB363_450
.LBB363_448:
	movq	%rbx, %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	64(%rax,%r14), %rax
	.p2align	4
.LBB363_449:
	addq	(%rax), %rdx
	addq	$160, %rax
	decq	%rcx
	jne	.LBB363_449
.LBB363_450:
	movq	%rsi, 1560(%rsp)
	movq	%r14, 1568(%rsp)
	movq	%rbx, 1576(%rsp)
	movq	%rbx, 608(%rsp)
	movq	1520(%rsp), %rbx
	movq	$0, 1264(%rsp)
	movq	$8, 1272(%rsp)
	movq	%rdx, 1064(%rsp)
	movq	%rsi, 1040(%rsp)
	movb	%bpl, 1584(%rsp)
	movq	$0, 1280(%rsp)
.Ltmp15922:
	leaq	384(%rsp), %rdi
	leaq	1264(%rsp), %rsi
	movq	%rbx, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp15923:
	movq	384(%rsp), %r13
	movq	392(%rsp), %rax
	movq	400(%rsp), %rdi
	movq	408(%rsp), %rsi
	movl	%ebp, 172(%rsp)
	movq	%r14, 1104(%rsp)
	cmpq	$-1, %r13
	je	.LBB363_555
	vmovdqu	432(%rsp), %ymm0
	vmovdqu	448(%rsp), %ymm1
	movq	%rax, 96(%rsp)
	movzbl	416(%rsp), %eax
	movzbl	423(%rsp), %ebp
	movzwl	421(%rsp), %r14d
	movl	417(%rsp), %r12d
	movq	424(%rsp), %rbx
	movq	%rdi, 48(%rsp)
	movq	%rsi, 80(%rsp)
	movq	%rax, 72(%rsp)
	vmovdqu	%ymm0, 1216(%rsp)
	vmovdqu	%ymm1, 1232(%rsp)
.Ltmp15927:
	leaq	1448(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15928:
	shll	$16, %ebp
	orl	%ebp, %r14d
	movl	172(%rsp), %ebp
	shlq	$32, %r14
	orq	%r14, %r12
.LBB363_454:
	movq	608(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_458
	movq	1104(%rsp), %r14
	movl	$1, %r15d
	subq	%rax, %r15
	.p2align	4
.LBB363_456:
.Ltmp16032:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp16033:
	incq	%r15
	addq	$160, %r14
	cmpq	$1, %r15
	jne	.LBB363_456
.LBB363_458:
	movq	1040(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_460
	movq	1104(%rsp), %rdi
	shlq	$5, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
.LBB363_460:
	cmpq	$-1, %r13
	je	.LBB363_561
	vmovdqu	1232(%rsp), %ymm1
	vmovdqu	1216(%rsp), %ymm0
	movq	%rbx, 1064(%rsp)
	movq	120(%rsp), %rbx
	movq	80(%rsp), %rbp
	vmovdqu	%ymm1, 1856(%rsp)
	vmovdqu	%ymm0, 1840(%rsp)
.LBB363_462:
	vmovdqu	1856(%rsp), %ymm1
	vmovdqu	1840(%rsp), %ymm0
	movq	96(%rsp), %rcx
	movzbl	72(%rsp), %eax
	movq	48(%rsp), %rdx
	shlq	$8, %r12
	orq	%r12, %rax
	vmovdqu	%ymm1, 80(%rbx)
	vmovdqu	%ymm0, 64(%rbx)
	movq	%r13, 16(%rbx)
	movq	%rcx, 24(%rbx)
	movq	1064(%rsp), %rcx
	movq	%rdx, 32(%rbx)
	movq	%rbp, 40(%rbx)
	movq	%rax, 48(%rbx)
	movq	%rcx, 56(%rbx)
	movq	$1, (%rbx)
.LBB363_463:
.Ltmp16046:
	leaq	1968(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp16047:
	movq	624(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_467
.LBB363_465:
	lock		decq	(%rax)
	jne	.LBB363_467
	#MEMBARRIER
.Ltmp16090:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	624(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16091:
.LBB363_467:
.Ltmp16095:
	leaq	2016(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp16096:
.LBB363_468:
	movb	$1, %bl
	movq	1136(%rsp), %r14
	movq	1144(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_472
.LBB363_469:
	movl	$1, %r12d
	movq	%r14, %r15
	subq	%rax, %r12
	.p2align	4
.LBB363_470:
.Ltmp16100:
	movq	%r15, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
.Ltmp16101:
	incq	%r12
	addq	$24, %r15
	cmpq	$1, %r12
	jne	.LBB363_470
.LBB363_472:
	movq	1128(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_482
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
	jge	.LBB363_475
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_475:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_481
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_475
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
.LBB363_478:
	cmpq	%rax, %rdx
	jge	.LBB363_480
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB363_478
.LBB363_480:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_481:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.LBB363_482:
	movl	%ebx, 96(%rsp)
	movq	1080(%rsp), %rbx
	movq	1088(%rsp), %r14
	testq	%r14, %r14
	je	.LBB363_505
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	xorl	%r15d, %r15d
	jmp	.LBB363_487
	.p2align	4
.LBB363_484:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_485:
	vzeroupper
	callq	*%r13
.LBB363_486:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB363_505
.LBB363_487:
	leaq	(%r15,%r15,8), %rax
	leaq	(%rbx,%rax,8), %r12
	movq	(%rbx,%rax,8), %rax
	cmpq	$6, %rax
	jb	.LBB363_497
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
	jge	.LBB363_490
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_490:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_496
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_490
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
.LBB363_493:
	cmpq	%rax, %rdx
	jge	.LBB363_495
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB363_493
.LBB363_495:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_496:
	vzeroupper
	callq	*%r13
.LBB363_497:
	movq	48(%r12), %rcx
	testq	%rcx, %rcx
	je	.LBB363_486
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
	jge	.LBB363_500
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_500:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_485
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_500
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
.LBB363_503:
	cmpq	%rax, %rdx
	jge	.LBB363_484
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB363_503
	jmp	.LBB363_484
.LBB363_505:
	movq	1072(%rsp), %rax
	movl	96(%rsp), %r15d
	testq	%rax, %rax
	je	.LBB363_515
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
	jge	.LBB363_508
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_508:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_514
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_508
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
.LBB363_511:
	cmpq	%rax, %rdx
	jge	.LBB363_513
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB363_511
.LBB363_513:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_514:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB363_515:
	cmpq	$0, 960(%rsp)
	movq	120(%rsp), %rbx
	je	.LBB363_525
	movq	952(%rsp), %rdi
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
	jge	.LBB363_518
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_518:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_524
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_518
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
.LBB363_521:
	cmpq	%rax, %rcx
	jge	.LBB363_523
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB363_521
.LBB363_523:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_524:
	movq	free@GOTPCREL(%rip), %rax
	movq	176(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB363_525:
	movq	160(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB363_527
	movb	$1, %bpl
	#MEMBARRIER
.Ltmp16106:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16107:
.LBB363_527:
.Ltmp16109:
	leaq	1152(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp16110:
	movq	1808(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB363_538
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	1816(%rsp), %rdi
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
	jge	.LBB363_531
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_531:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_537
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_531
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
.LBB363_534:
	cmpq	%rax, %rdx
	jge	.LBB363_536
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB363_534
.LBB363_536:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_537:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_538:
	movq	1736(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB363_548
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	1744(%rsp), %rdi
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
	jge	.LBB363_541
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_541:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_547
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_541
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
.LBB363_544:
	cmpq	%rax, %rdx
	jge	.LBB363_546
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB363_544
.LBB363_546:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_547:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_548:
	movq	1832(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_551
	lock		decq	(%rax)
	jne	.LBB363_551
	leaq	1832(%rsp), %rdi
	#MEMBARRIER
.Ltmp16112:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp16113:
.LBB363_551:
	testb	%r15b, %r15b
	je	.LBB363_554
.LBB363_552:
	movq	320(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB363_554
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1552(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB363_554:
	movq	%rbx, %rax
	addq	$2488, %rsp
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
.LBB363_555:
	.cfi_def_cfa_offset 2544
	movq	%rax, 184(%rsp)
	movq	1512(%rsp), %rcx
	movq	1504(%rsp), %rax
	leaq	(%rbx,%rbx,4), %rdx
	movq	%rdi, 192(%rsp)
	movq	%rsi, 200(%rsp)
	movq	%rcx, 976(%rsp)
	movq	%rax, 992(%rsp)
	leaq	(%rcx,%rdx,8), %rdx
	movq	%rcx, 984(%rsp)
	movq	2552(%rsp), %rcx
	movq	%rdx, 1000(%rsp)
	movq	616(%rcx), %rax
	movq	%rax, 1544(%rsp)
	testq	%rax, %rax
	je	.LBB363_562
	lock		incq	(%rax)
	movq	1040(%rsp), %rcx
	movq	608(%rsp), %rdx
	jle	.LBB363_944
	movq	2552(%rsp), %rax
	movq	616(%rax), %rbp
	movq	%rbp, 1208(%rsp)
	movq	%rbp, 144(%rsp)
	movq	16(%rbp), %rax
	movq	40(%rbp), %rsi
	movq	%rax, 1056(%rsp)
	movq	%rsi, 1112(%rsp)
	cmpq	$-1, %rsi
	je	.LBB363_574
	testq	%rdx, %rdx
	je	.LBB363_584
	cmpq	$8, %rdx
	jae	.LBB363_683
	xorl	%eax, %eax
	xorl	%ebx, %ebx
	jmp	.LBB363_695
.LBB363_561:
	movq	72(%rsp), %rdx
	jmp	.LBB363_590
.LBB363_562:
	movq	984(%rsp), %rcx
	movq	976(%rsp), %rax
	movq	992(%rsp), %rdx
	movq	%rsi, %rbp
	movq	%rdi, 48(%rsp)
	movq	%rcx, 216(%rsp)
	movq	1000(%rsp), %rcx
	movq	%rax, 208(%rsp)
	movq	%rdx, 224(%rsp)
	movq	%rcx, 232(%rsp)
	leaq	392(%rsp), %rcx
	movq	232(%rsp), %rax
	movq	216(%rsp), %r14
	movq	%rax, 376(%rsp)
	cmpq	%rax, %r14
	je	.LBB363_572
	leaq	(,%rbp,8), %rax
	leaq	(%rax,%rax,4), %rbx
	jmp	.LBB363_565
.LBB363_564:
	movq	48(%rsp), %rcx
	movq	72(%rsp), %rax
	shll	$16, %r15d
	movq	152(%rsp), %rdx
	addq	$40, %r14
	orl	%r15d, %ebp
	shlq	$32, %rbp
	orq	%rbp, %r13
	movq	80(%rsp), %rbp
	movq	%rax, (%rcx,%rbx)
	movq	40(%rsp), %rax
	incq	%rbp
	movq	%rax, 8(%rcx,%rbx)
	movzbl	336(%rsp), %eax
	movq	%r12, 16(%rcx,%rbx)
	movb	%al, 24(%rcx,%rbx)
	movq	%r13, %rax
	shrq	$48, %rax
	movl	%r13d, 25(%rcx,%rbx)
	shrq	$32, %r13
	movb	%al, 31(%rcx,%rbx)
	movw	%r13w, 29(%rcx,%rbx)
	movq	%rdx, 32(%rcx,%rbx)
	addq	$40, %rbx
	leaq	392(%rsp), %rcx
	movq	%rbp, 200(%rsp)
	cmpq	376(%rsp), %r14
	je	.LBB363_572
.LBB363_565:
	movq	32(%r14), %rax
	leaq	1272(%rsp), %rdx
	movq	%rax, 32(%rdx)
	movq	2552(%rsp), %rax
	vmovdqu	(%r14), %ymm0
	vmovdqu	%ymm0, (%rdx)
	movq	%rax, 1264(%rsp)
	cmpq	$0, 1272(%rsp)
	je	.LBB363_567
	movq	32(%r14), %rax
	movq	%rax, 32(%rcx)
	vmovdqu	(%r14), %ymm0
	vmovdqu	%ymm0, (%rcx)
	jmp	.LBB363_569
.LBB363_567:
	movq	664(%rax), %rdx
.Ltmp16011:
	movq	96(%rsp), %rsi
	leaq	384(%rsp), %rdi
	leaq	1280(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp16012:
	movq	384(%rsp), %r13
	cmpq	$-1, %r13
	jne	.LBB363_686
.LBB363_569:
	movq	400(%rsp), %rcx
	movq	%rbp, %rdx
	movq	392(%rsp), %rax
	movzbl	416(%rsp), %esi
	movq	408(%rsp), %r12
	movzbl	423(%rsp), %r15d
	movzwl	421(%rsp), %ebp
	movl	417(%rsp), %r13d
	movq	%rdx, 80(%rsp)
	movq	%rcx, 40(%rsp)
	movq	424(%rsp), %rcx
	movq	%rax, 72(%rsp)
	movb	%sil, 336(%rsp)
	movq	%rcx, 152(%rsp)
	cmpq	184(%rsp), %rdx
	jne	.LBB363_564
.Ltmp16019:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	184(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16020:
	movq	192(%rsp), %rax
	movq	%rax, 48(%rsp)
	jmp	.LBB363_564
.LBB363_572:
	movb	$1, %al
	movq	%rbp, 80(%rsp)
	movq	%r14, 216(%rsp)
	movl	%eax, 40(%rsp)
.Ltmp16024:
	leaq	208(%rsp), %rdi
	movb	$1, %r14b
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16025:
	movq	184(%rsp), %rax
	movq	192(%rsp), %rcx
	movl	172(%rsp), %ebp
	movq	$-1, %r13
	movq	%rax, 96(%rsp)
	movb	$2, %al
	movq	%rcx, 48(%rsp)
	movq	%rax, 72(%rsp)
	jmp	.LBB363_454
.LBB363_574:
	leaq	(%rdx,%rdx,4), %rax
	movq	%r14, 1008(%rsp)
	movq	%r14, 1016(%rsp)
	movq	%rcx, 1024(%rsp)
	shlq	$5, %rax
	addq	%r14, %rax
	movq	%rax, 1536(%rsp)
	movq	%rax, 1032(%rsp)
	testq	%rdx, %rdx
	jne	.LBB363_700
	jmp	.LBB363_585
.LBB363_575:
	vmovups	432(%rsp), %ymm0
	movq	392(%rsp), %rdx
	movq	400(%rsp), %rcx
	movq	416(%rsp), %r12
	movq	%rax, 40(%rsp)
	movq	%r14, 216(%rsp)
	movq	%rdx, 96(%rsp)
	movq	408(%rsp), %rdx
	movq	%rcx, 48(%rsp)
	movq	424(%rsp), %rcx
	vmovups	%ymm0, 1264(%rsp)
	vmovdqu	448(%rsp), %ymm0
	movq	%rdx, 80(%rsp)
	movq	%rcx, 1064(%rsp)
	vmovdqu	%ymm0, 1280(%rsp)
.Ltmp15889:
	leaq	208(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15890:
	movq	120(%rsp), %rbx
	movq	%r13, %rdi
	testq	%rbp, %rbp
	je	.LBB363_581
	leaq	8(%rdi), %r14
	xorl	%r15d, %r15d
	jmp	.LBB363_579
.LBB363_578:
	incq	%r15
	addq	$40, %r14
	cmpq	%r15, %rbp
	je	.LBB363_581
.LBB363_579:
	movq	-8(%r14), %rax
	cmpq	$6, %rax
	jb	.LBB363_578
	movq	(%r14), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	movq	%r13, %rdi
	jmp	.LBB363_578
.LBB363_581:
	movq	640(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_583
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
.LBB363_583:
	movq	80(%rsp), %rbp
	movq	40(%rsp), %r13
	jmp	.LBB363_353
.LBB363_584:
	movq	%r14, 1008(%rsp)
	movq	%r14, 1016(%rsp)
	movq	%rcx, 1024(%rsp)
	movq	%r14, 1032(%rsp)
.LBB363_585:
	movb	$1, %al
	xorl	%r14d, %r14d
	movl	%eax, 40(%rsp)
.Ltmp16002:
	leaq	1008(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp16003:
	movq	184(%rsp), %rax
	movq	192(%rsp), %rdx
	movq	200(%rsp), %rcx
	movq	%rax, 96(%rsp)
	movq	%rdx, 48(%rsp)
	movq	%rcx, 80(%rsp)
	lock		decq	(%rbp)
	jne	.LBB363_588
	#MEMBARRIER
	movl	$0, 40(%rsp)
.Ltmp16007:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1208(%rsp), %rdi
	xorl	%r14d, %r14d
	callq	*%rax
.Ltmp16008:
.LBB363_588:
	xorl	%r15d, %r15d
.Ltmp16009:
	leaq	976(%rsp), %rdi
	xorl	%r14d, %r14d
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16010:
	movl	172(%rsp), %ebp
	movb	$2, %dl
.LBB363_590:
	movq	96(%rsp), %rax
	movq	48(%rsp), %rcx
	movq	80(%rsp), %rsi
	movq	%rax, 1264(%rsp)
	movq	%rcx, 1272(%rsp)
	movq	2552(%rsp), %rcx
	movq	%rsi, 1280(%rsp)
	movq	616(%rcx), %rax
	testq	%rax, %rax
	je	.LBB363_599
	movl	296(%rax), %ecx
	movb	$-1, %bl
	testl	%ecx, %ecx
	jne	.LBB363_593
	movq	288(%rax), %rcx
	movzbl	272(%rax), %ebx
	movq	%rcx, 223(%rsp)
	vmovdqu	273(%rax), %xmm0
	vmovdqa	%xmm0, 208(%rsp)
.LBB363_593:
	testb	%dl, %dl
	je	.LBB363_598
	movzbl	%dl, %eax
	cmpl	$2, %eax
	je	.LBB363_600
	cmpb	$-1, %bl
	je	.LBB363_685
	movq	2552(%rsp), %rax
	cmpb	$2, 472(%rax)
	jne	.LBB363_598
	vmovdqa	208(%rsp), %xmm0
	movq	696(%rax), %rdi
	movq	223(%rsp), %rax
	movb	%bl, 384(%rsp)
	vmovdqu	%xmm0, 385(%rsp)
	movq	%rax, 400(%rsp)
	movl	40(%rdi), %eax
	testl	%eax, %eax
	jne	.LBB363_940
.LBB363_598:
	xorl	%ebp, %ebp
	jmp	.LBB363_600
.LBB363_599:
	cmpb	$2, %dl
	movb	$-1, %bl
	sete	%al
	andb	%bpl, %al
	movl	%eax, %ebp
.LBB363_600:
	cmpb	$-1, %bl
	sete	%al
	andb	%bpl, %al
	movq	80(%rsp), %rbp
	movq	%rax, 72(%rsp)
.LBB363_601:
	movzbl	72(%rsp), %ebx
	movq	1064(%rsp), %r14
.LBB363_602:
	vmovdqu	1968(%rsp), %ymm0
	movq	96(%rsp), %rax
	movq	48(%rsp), %rcx
	movq	%rax, 720(%rsp)
	movq	%rcx, 728(%rsp)
	movq	%rbp, 736(%rsp)
	vmovdqu	%ymm0, 384(%rsp)
.Ltmp16049:
	movq	2552(%rsp), %rdi
	leaq	384(%rsp), %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::absorb_worker_witnesses::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp16050:
	testb	$1, %bl
	je	.LBB363_606
	movq	1088(%rsp), %rdx
	cmpq	%rdx, %r14
	ja	.LBB363_932
	jne	.LBB363_659
.LBB363_606:
	movq	736(%rsp), %rax
	vmovdqu	720(%rsp), %xmm0
	movq	%rax, 1488(%rsp)
	movq	624(%rsp), %rax
	vmovdqa	%xmm0, 1472(%rsp)
	testq	%rax, %rax
	je	.LBB363_609
	lock		decq	(%rax)
	jne	.LBB363_609
	#MEMBARRIER
.Ltmp16067:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	624(%rsp), %rdi
	callq	*%rax
.Ltmp16068:
.LBB363_609:
.Ltmp16069:
	leaq	2016(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp16070:
	movq	120(%rsp), %rbx
.LBB363_611:
	movq	2552(%rsp), %rax
	movq	696(%rax), %rax
	movl	40(%rax), %ecx
	testl	%ecx, %ecx
	je	.LBB363_614
.LBB363_612:
	vmovdqa	1472(%rsp), %xmm0
	movq	1488(%rsp), %rax
	cmpq	$-1, 1736(%rsp)
	movq	320(%rsp), %rcx
	movq	%rax, 736(%rsp)
	vmovdqa	%xmm0, 720(%rsp)
	movq	%rcx, 744(%rsp)
	je	.LBB363_617
	leaq	384(%rsp), %rdi
	leaq	720(%rsp), %rsi
	leaq	1736(%rsp), %rdx
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	jmp	.LBB363_618
.LBB363_614:
	movzbl	16(%rax), %ecx
	cmpb	$-1, %cl
	je	.LBB363_612
	movb	%cl, 720(%rsp)
	vmovdqu	17(%rax), %xmm0
	vmovdqu	%xmm0, 721(%rsp)
	movq	32(%rax), %rax
	movq	%rax, 736(%rsp)
.Ltmp16071:
	movq	1192(%rsp), %rsi
	movq	320(%rsp), %rcx
	leaq	384(%rsp), %rdi
	leaq	720(%rsp), %rdx
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
.Ltmp16072:
	vmovdqu64	416(%rsp), %zmm1
	vmovdqu64	384(%rsp), %zmm0
	leaq	1472(%rsp), %rdi
	vmovdqu64	%zmm1, 40(%rbx)
	vmovdqu64	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	xorl	%ebx, %ebx
	movq	1136(%rsp), %r14
	movq	1144(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB363_469
	jmp	.LBB363_472
.LBB363_617:
	vmovdqu	720(%rsp), %xmm0
	movq	736(%rsp), %rax
	movq	744(%rsp), %rcx
	movq	%rax, 408(%rsp)
	movq	%rcx, 416(%rsp)
	vmovdqu	%xmm0, 392(%rsp)
	movq	$-1, 384(%rsp)
.LBB363_618:
	movq	1808(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB363_620
	movq	1816(%rsp), %rdi
	leaq	-3(%rax,%rax,2), %rsi
	movl	$1, %edx
	callq	__rustc::__rust_dealloc
.LBB363_620:
	movq	1832(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_623
	lock		decq	(%rax)
	jne	.LBB363_623
	leaq	1832(%rsp), %rdi
	#MEMBARRIER
.Ltmp16074:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp16075:
.LBB363_623:
	vmovdqu64	416(%rsp), %zmm1
	vmovdqu64	384(%rsp), %zmm0
	movq	1136(%rsp), %r14
	movq	1144(%rsp), %rax
	vmovdqu64	%zmm1, 40(%rbx)
	vmovdqu64	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
	testq	%rax, %rax
	je	.LBB363_627
	movl	$1, %r12d
	movq	%r14, %r15
	subq	%rax, %r12
	.p2align	4
.LBB363_625:
.Ltmp16077:
	movq	%r15, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
.Ltmp16078:
	incq	%r12
	addq	$24, %r15
	cmpq	$1, %r12
	jne	.LBB363_625
.LBB363_627:
	movq	1128(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_629
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r14, %rdi
	leaq	(%rax,%rax,2), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB363_629:
	movq	1080(%rsp), %rbx
	movq	1088(%rsp), %r14
	testq	%r14, %r14
	je	.LBB363_652
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	xorl	%r15d, %r15d
	jmp	.LBB363_634
	.p2align	4
.LBB363_631:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_632:
	vzeroupper
	callq	*%rbp
.LBB363_633:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB363_652
.LBB363_634:
	leaq	(%r15,%r15,8), %rax
	leaq	(%rbx,%rax,8), %r12
	movq	(%rbx,%rax,8), %rax
	cmpq	$6, %rax
	jb	.LBB363_644
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
	jge	.LBB363_637
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_637:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_643
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_637
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
.LBB363_640:
	cmpq	%rax, %rdx
	jge	.LBB363_642
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB363_640
.LBB363_642:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_643:
	vzeroupper
	callq	*%rbp
.LBB363_644:
	movq	48(%r12), %rcx
	testq	%rcx, %rcx
	je	.LBB363_633
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
	jge	.LBB363_647
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_647:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_632
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_647
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
.LBB363_650:
	cmpq	%rax, %rdx
	jge	.LBB363_631
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB363_650
	jmp	.LBB363_631
.LBB363_652:
	movq	1072(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_654
	shlq	$3, %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	leaq	(%rax,%rax,8), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB363_654:
	cmpq	$0, 960(%rsp)
	je	.LBB363_656
	movq	176(%rsp), %rdi
	movq	952(%rsp), %rsi
	movl	$8, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB363_656:
	movq	160(%rsp), %rax
	lock		decq	(%rax)
	movq	120(%rsp), %rbx
	jne	.LBB363_658
	xorl	%ebp, %ebp
	#MEMBARRIER
.Ltmp16083:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	xorl	%r15d, %r15d
	vzeroupper
	callq	*%rax
.Ltmp16084:
.LBB363_658:
	leaq	1152(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
	jmp	.LBB363_554
.LBB363_659:
	movq	1080(%rsp), %rax
	leaq	(%rdx,%rdx,8), %rcx
	addq	$16, 328(%rsp)
	leaq	1592(%rsp), %rbx
	leaq	(%rax,%rcx,8), %rcx
	movq	%rcx, 96(%rsp)
	leaq	(%r14,%r14,8), %rcx
	leaq	(%rax,%rcx,8), %rbp
.LBB363_660:
	movq	1200(%rsp), %rsi
.Ltmp16051:
	movq	%rbx, %rdi
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::from_elem
.Ltmp16052:
	movq	1592(%rsp), %r13
	leaq	1600(%rsp), %rdi
	movq	%r13, %rax
	cmpq	$6, %r13
	jb	.LBB363_663
	movq	1600(%rsp), %rdi
	movq	1608(%rsp), %rax
.LBB363_663:
	movq	968(%rsp), %rdx
	decq	%rax
	cmpq	%rax, %rdx
	ja	.LBB363_929
	movq	(%rbp), %rsi
	decq	%rsi
	cmpq	$4, %rsi
	jbe	.LBB363_666
	movq	16(%rbp), %rsi
	movq	8(%rbp), %rax
	decq	%rsi
	jmp	.LBB363_667
.LBB363_666:
	leaq	8(%rbp), %rax
.LBB363_667:
	cmpq	%rsi, %rdx
	jne	.LBB363_930
	movq	memcpy@GOTPCREL(%rip), %r14
	shlq	$3, %rdx
	movq	%rax, %rsi
	callq	*%r14
	movq	1144(%rsp), %rbx
	movq	2544(%rsp), %rax
	cmpq	%rbx, %rax
	cmovbq	%rax, %rbx
	testq	%rbx, %rbx
	je	.LBB363_678
	movq	1136(%rsp), %r15
	movq	328(%rsp), %r12
	xorl	%r14d, %r14d
	addq	$16, %r15
	jmp	.LBB363_671
.LBB363_670:
	incq	%r14
	addq	$24, %r15
	addq	$120, %r12
	vmovq	%xmm0, (%rax,%rdi,8)
	cmpq	%r14, %rbx
	je	.LBB363_678
.LBB363_671:
	vmovups	1160(%rsp), %xmm0
	movq	160(%rsp), %rax
	movq	-8(%r15), %rdx
	movq	(%r15), %rcx
	movq	56(%rbp), %r8
	movq	64(%rbp), %r9
	addq	$16, %rax
.Ltmp16056:
	movq	2552(%rsp), %rsi
	leaq	384(%rsp), %rdi
	movq	%rax, 16(%rsp)
	movq	%rsi, 24(%rsp)
	movq	%r12, %rsi
	vmovups	%xmm0, (%rsp)
	callq	purrdf_sparql_eval::modifier::eval_aggregate::<purrdf_core::ir::dataset::RdfDataset, ()>
.Ltmp16057:
	vmovq	392(%rsp), %xmm0
	movq	384(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB363_680
	movq	1592(%rsp), %r13
	movq	%r13, %rsi
	cmpq	$6, %r13
	jb	.LBB363_675
	movq	1608(%rsp), %rsi
.LBB363_675:
	movq	968(%rsp), %rdi
	decq	%rsi
	addq	%r14, %rdi
	cmpq	%rsi, %rdi
	jae	.LBB363_941
	leaq	1600(%rsp), %rax
	cmpq	$6, %r13
	jb	.LBB363_670
	movq	1600(%rsp), %rax
	jmp	.LBB363_670
.LBB363_678:
.Ltmp16061:
	leaq	1592(%rsp), %rbx
	leaq	720(%rsp), %rdi
	movq	%rbx, %rsi
	callq	<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::push_mut
.Ltmp16062:
	addq	$72, %rbp
	cmpq	96(%rsp), %rbp
	jne	.LBB363_660
	jmp	.LBB363_606
.LBB363_680:
	vmovups	400(%rsp), %zmm1
	vmovups	416(%rsp), %zmm2
	movq	120(%rsp), %rcx
	vmovups	%zmm2, 48(%rcx)
	vmovups	%zmm1, 32(%rcx)
	movq	%rax, 16(%rcx)
	movq	1592(%rsp), %rax
	vmovq	%xmm0, 24(%rcx)
	movq	$1, (%rcx)
	cmpq	$6, %rax
	jb	.LBB363_682
	movq	1600(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB363_682:
	leaq	720(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	624(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB363_465
	jmp	.LBB363_467
.LBB363_683:
	cmpq	$32, %rdx
	jae	.LBB363_688
	xorl	%eax, %eax
	xorl	%ebx, %ebx
	jmp	.LBB363_692
.LBB363_685:
	movb	$-1, %bl
	xorl	%ebp, %ebp
	jmp	.LBB363_600
.LBB363_686:
	vmovups	432(%rsp), %ymm0
	movq	392(%rsp), %rax
	movq	400(%rsp), %rdx
	movq	408(%rsp), %rcx
	movzbl	423(%rsp), %ebp
	movzwl	421(%rsp), %r15d
	movl	417(%rsp), %r12d
	movq	424(%rsp), %rbx
	addq	$40, %r14
	movq	%r14, 216(%rsp)
	movq	%rax, 96(%rsp)
	movzbl	416(%rsp), %eax
	movq	%rdx, 48(%rsp)
	movq	%rcx, 80(%rsp)
	vmovups	%ymm0, 1216(%rsp)
	vmovdqu	448(%rsp), %ymm0
	movq	%rax, 72(%rsp)
	movb	$1, %al
	movl	%eax, 40(%rsp)
	vmovdqu	%ymm0, 1232(%rsp)
.Ltmp16014:
	leaq	208(%rsp), %rdi
	movb	$1, %r14b
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16015:
	shll	$16, %ebp
	movb	$1, %r14b
	orl	%ebp, %r15d
	shlq	$32, %r15
	orq	%r15, %r12
	movb	$1, %r15b
	jmp	.LBB363_917
.LBB363_688:
	vmovdqa64	.LCPI363_1(%rip), %zmm1
	vpbroadcastq	.LCPI363_2(%rip), %zmm2
	vpbroadcastq	.LCPI363_3(%rip), %zmm3
	movq	%rdx, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
.LBB363_689:
	vpmullq	%zmm2, %zmm1, %zmm7
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm8, %xmm8, %xmm8
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$-32, %rcx
	vpgatherqq	16(%r14,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm0, %zmm8, %zmm0
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	1296(%r14,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm4, %zmm8, %zmm4
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	2576(%r14,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm5, %zmm8, %zmm5
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	3856(%r14,%zmm7), %zmm8 {%k1}
	vpaddq	%zmm6, %zmm8, %zmm6
	jne	.LBB363_689
	vpaddq	%zmm0, %zmm4, %zmm0
	movq	608(%rsp), %rcx
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
	je	.LBB363_697
	testb	$24, %cl
	je	.LBB363_695
.LBB363_692:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI363_1(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI363_2(%rip), %zmm2
	vpbroadcastq	.LCPI363_4(%rip), %zmm3
	movq	608(%rsp), %rax
	vmovq	%rbx, %xmm0
	andq	$-8, %rax
	subq	%rax, %rcx
.LBB363_693:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	16(%r14,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB363_693
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rbx
	cmpq	%rax, 608(%rsp)
	je	.LBB363_697
.LBB363_695:
	movq	608(%rsp), %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	16(%rax,%r14), %rax
.LBB363_696:
	addq	(%rax), %rbx
	addq	$160, %rax
	decq	%rcx
	jne	.LBB363_696
.LBB363_697:
	movq	2552(%rsp), %rcx
	movq	912(%rcx), %rax
	movq	928(%rcx), %rsi
	leaq	912(%rcx), %r15
	subq	%rsi, %rax
	cmpq	%rax, %rbx
	ja	.LBB363_936
.LBB363_698:
	movq	2552(%rsp), %rax
	cmpq	1016(%rax), %rbx
	ja	.LBB363_937
.LBB363_699:
	movq	608(%rsp), %rax
	movq	1104(%rsp), %r14
	movq	1040(%rsp), %rdx
	leaq	(%rax,%rax,4), %rcx
	movq	%r14, 1008(%rsp)
	movq	%r14, 1016(%rsp)
	movq	%rdx, 1024(%rsp)
	shlq	$5, %rcx
	addq	%r14, %rcx
	movq	%rcx, 1536(%rsp)
	movq	%rcx, 1032(%rsp)
.LBB363_700:
	leaq	16(%rbp), %rax
	leaq	272(%rbp), %rcx
	movq	%rax, 312(%rsp)
	movq	%rcx, 1184(%rsp)
.LBB363_701:
	leaq	160(%r14), %rdx
	movq	%rdx, 1016(%rsp)
	vmovups	96(%r14), %zmm0
	movq	(%r14), %rax
	vmovups	%zmm0, 1352(%rsp)
	vmovups	72(%r14), %zmm0
	vmovups	%zmm0, 1328(%rsp)
	vmovups	8(%r14), %zmm0
	vmovups	%zmm0, 1264(%rsp)
	cmpq	$-1, %rax
	je	.LBB363_585
	vmovdqu64	1264(%rsp), %zmm0
	vmovdqu64	1328(%rsp), %zmm1
	vmovdqu64	1352(%rsp), %zmm2
	leaq	392(%rsp), %rcx
	movq	%rax, 384(%rsp)
	movq	%rdx, 1896(%rsp)
	vmovdqu64	%zmm2, 88(%rcx)
	vmovdqu64	%zmm1, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
	movq	392(%rsp), %rdx
	movq	416(%rsp), %rcx
	imulq	$88, 400(%rsp), %rsi
	movq	408(%rsp), %r14
	movq	424(%rsp), %rdi
	movq	432(%rsp), %r15
	movq	%rcx, 128(%rsp)
	movq	%rdx, 344(%rsp)
	movq	%rax, 360(%rsp)
	movq	440(%rsp), %rax
	movq	448(%rsp), %rcx
	movq	%rdi, 1920(%rsp)
	movq	%rdx, 80(%rsp)
	movq	%rdx, 352(%rsp)
	movq	%r14, 944(%rsp)
	addq	%rdx, %rsi
	movq	%rsi, 136(%rsp)
	movq	%rsi, 368(%rsp)
	movq	%rax, 1096(%rsp)
	testq	%rcx, %rcx
	je	.LBB363_853
	movq	1096(%rsp), %r12
	movq	%rcx, %rax
	movq	128(%rsp), %rcx
	movq	480(%rsp), %rdx
	shlq	$5, %rax
	movq	$0, 1048(%rsp)
	movq	%r15, 936(%rsp)
	addq	%r12, %rax
	movq	%rdx, 48(%rsp)
	movq	%rax, 1912(%rsp)
	leaq	8(%rcx), %rax
	movq	%rax, 1904(%rsp)
	jmp	.LBB363_705
.LBB363_704:
	movq	%r15, 984(%rsp)
	movq	944(%rsp), %r14
	movq	936(%rsp), %r15
	addq	$32, %r12
	cmpq	1912(%rsp), %r12
	je	.LBB363_853
.LBB363_705:
	movq	16(%r12), %rax
	movq	24(%r12), %rdx
	movq	1048(%rsp), %r13
	movq	(%r12), %rcx
	movq	8(%r12), %rbx
	movq	%rax, 40(%rsp)
	movq	%rdx, 72(%rsp)
	testq	%rcx, %rcx
	je	.LBB363_712
	cmpq	$-1, 1056(%rsp)
	je	.LBB363_712
	movq	80(%rbp), %rax
	movq	$-1, %rsi
.LBB363_708:
	movq	%rax, %rdx
	addq	%rcx, %rdx
	cmovbq	%rsi, %rdx
	lock		cmpxchgq	%rdx, 80(%rbp)
	jne	.LBB363_708
	movq	312(%rsp), %rdx
	addq	%rcx, %rax
	cmovbq	%rsi, %rax
	movq	(%rdx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB363_712
	movq	%rcx, 216(%rsp)
	movq	%rax, 224(%rsp)
	movw	$0, 208(%rsp)
.Ltmp15934:
	movq	312(%rsp), %rsi
	leaq	640(%rsp), %rdi
	leaq	208(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.10720091597982897309)
.Ltmp15935:
	cmpb	$-1, 640(%rsp)
	jne	.LBB363_888
.LBB363_712:
	movq	1920(%rsp), %rdx
	cmpq	%rdx, %r13
	ja	.LBB363_931
	movq	1112(%rsp), %rdi
	movq	48(%rsp), %r10
	cmpq	%rdx, %rbx
	movq	%rdx, %rsi
	leaq	.LJTI363_0(%rip), %r9
	movq	%r12, 616(%rsp)
	cmovbq	%rbx, %rsi
	cmpq	%r13, %rbx
	cmovbq	%r13, %rsi
	cmpq	%r13, %rsi
	jb	.LBB363_928
	leaq	(,%r13,8), %rax
	movq	%r13, %rcx
	movq	$-1, %r8
	movq	%rcx, 152(%rsp)
	leaq	(%rax,%rax,2), %r13
	leaq	(,%rsi,8), %rax
	leaq	(%rax,%rax,2), %r12
	cmpq	%rsi, %rcx
	jne	.LBB363_721
	xorl	%r15d, %r15d
	xorl	%r14d, %r14d
	xorl	%ebx, %ebx
.LBB363_716:
	movq	$-1, %rbp
	movq	%r15, 336(%rsp)
	movq	%r14, 376(%rsp)
	movq	%rbx, 1120(%rsp)
	movq	%rsi, 1048(%rsp)
	cmpq	$-1, %rdi
	je	.LBB363_727
	movq	40(%rsp), %rax
	movl	$0, %ecx
	movq	136(%rsp), %r15
	movl	$0, %ebx
	subq	%r10, %rax
	cmovbq	%rcx, %rax
	subq	80(%rsp), %r15
	movabsq	$3353953467947191203, %rcx
	shrq	$3, %r15
	imulq	%rcx, %r15
	cmpq	%r15, %rax
	cmovbq	%rax, %r15
	testq	%r15, %r15
	je	.LBB363_728
	movq	80(%rsp), %rax
	xorl	%ebx, %ebx
	leaq	8(%rax), %r14
.LBB363_719:
.Ltmp15937:
	movq	purrdf_sparql_eval::scratch::value_bytes@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15938:
	addq	%rax, %rbx
	cmovbq	%rbp, %rbx
	addq	$88, %r14
	decq	%r15
	jne	.LBB363_719
	jmp	.LBB363_728
.LBB363_721:
	movq	%r12, %rdx
	subq	%r13, %rdx
	movabsq	$-6148914691236517205, %rax
	xorl	%ebx, %ebx
	xorl	%r14d, %r14d
	xorl	%r15d, %r15d
	mulxq	%rax, %rax, %rax
	movq	1904(%rsp), %rcx
	shrq	$4, %rax
	addq	%r13, %rcx
	jmp	.LBB363_724
.LBB363_722:
	addq	%rdx, %r14
	cmovbq	%r8, %r14
.LBB363_723:
	addq	$24, %rcx
	decq	%rax
	je	.LBB363_716
.LBB363_724:
	movzbl	-8(%rcx), %r11d
	movq	(%rcx), %rdx
	movslq	(%r9,%r11,4), %r11
	addq	%r9, %r11
	jmpq	*%r11
.LBB363_725:
	addq	%rdx, %r15
	cmovbq	%r8, %r15
	jmp	.LBB363_723
.LBB363_726:
	cmpq	%rdx, %rbx
	cmovbeq	%rdx, %rbx
	jmp	.LBB363_723
.LBB363_727:
	xorl	%ebx, %ebx
.LBB363_728:
	movq	144(%rsp), %rax
	movl	296(%rax), %eax
	movq	1112(%rsp), %r15
	testl	%eax, %eax
	je	.LBB363_750
.LBB363_729:
	cmpq	$-1, 1056(%rsp)
	je	.LBB363_731
	movq	144(%rsp), %rcx
	movq	80(%rcx), %rax
	addq	336(%rsp), %rax
	cmovbq	%rbp, %rax
	cmpq	16(%rcx), %rax
	ja	.LBB363_751
.LBB363_731:
	cmpq	$-1, %r15
	je	.LBB363_733
	movq	2552(%rsp), %rcx
	movq	$-1, %rdx
	movq	1048(%rcx), %rax
	movq	1056(%rcx), %rcx
	movq	144(%rsp), %rsi
	subq	%rcx, %rax
	movl	$0, %ecx
	cmovaeq	%rax, %rcx
	addq	%rbx, %rcx
	cmovbq	%rdx, %rcx
	addq	376(%rsp), %rcx
	cmovbq	%rdx, %rcx
	addq	1120(%rsp), %rcx
	movq	104(%rsi), %rax
	cmovbq	%rdx, %rcx
	addq	%rcx, %rax
	cmovbq	%rdx, %rax
	cmpq	40(%rsi), %rax
	ja	.LBB363_751
.LBB363_733:
	cmpq	$-1, 1056(%rsp)
	movq	144(%rsp), %rbp
	je	.LBB363_735
	movq	152(%rsp), %rcx
	movq	%r13, %rax
	cmpq	1048(%rsp), %rcx
	jne	.LBB363_745
.LBB363_735:
	movb	$1, %r14b
	movq	152(%rsp), %rax
	cmpq	1048(%rsp), %rax
	je	.LBB363_738
.LBB363_736:
	movq	128(%rsp), %rax
	cmpb	$2, -24(%rax,%r12)
	je	.LBB363_819
	addq	$-24, %r12
	cmpq	%r12, %r13
	jne	.LBB363_736
.LBB363_738:
	movq	616(%rsp), %r12
.LBB363_739:
	cmpq	$-1, 1056(%rsp)
	je	.LBB363_807
	cmpq	$0, 336(%rsp)
	je	.LBB363_807
	movq	336(%rsp), %rax
	movq	%rax, 640(%rsp)
	movq	$0, 648(%rsp)
.Ltmp15943:
	movq	312(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	leaq	208(%rsp), %rdi
	leaq	640(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp15944:
	cmpb	$-1, 224(%rsp)
	je	.LBB363_807
	movb	$1, %r14b
	cmpq	$-1, %r15
	jne	.LBB363_830
	jmp	.LBB363_900
.LBB363_744:
	addq	$24, %rax
	cmpq	%rax, %r12
	je	.LBB363_735
.LBB363_745:
	movq	128(%rsp), %rcx
	cmpb	$0, (%rcx,%rax)
	jne	.LBB363_744
	movq	128(%rsp), %rcx
	movzbl	1(%rcx,%rax), %ecx
	cmpq	$255, %rcx
	je	.LBB363_744
	movq	2552(%rsp), %rdx
	movq	632(%rdx), %rdx
	testq	%rdx, %rdx
	je	.LBB363_744
	movq	2552(%rsp), %rsi
	movl	1228(%rsi), %esi
	cmpq	%rsi, 56(%rdx)
	jbe	.LBB363_744
	movq	128(%rsp), %rdi
	movq	%rsi, %r8
	shlq	$7, %r8
	leaq	(%r8,%rsi,8), %rsi
	addq	48(%rdx), %rsi
	movq	8(%rdi,%rax), %rdi
	lock		addq	%rdi, (%rsi,%rcx,8)
	jmp	.LBB363_744
.LBB363_750:
	movq	1184(%rsp), %rax
	cmpb	$-1, (%rax)
	je	.LBB363_729
.LBB363_751:
	movq	152(%rsp), %rax
	cmpq	1048(%rsp), %rax
	jne	.LBB363_762
	movq	144(%rsp), %rbp
.LBB363_753:
	cmpq	$-1, %r15
	je	.LBB363_812
	movq	48(%rsp), %r15
	movq	616(%rsp), %r12
	cmpq	40(%rsp), %r15
	jae	.LBB363_839
	movq	80(%rsp), %r14
	cmpq	136(%rsp), %r14
	je	.LBB363_761
	movq	40(%rsp), %rax
	addq	$88, %r14
	leaq	-1(%rax), %rbx
	movq	%r14, %rax
.LBB363_757:
	movq	%rax, %r14
	movq	-8(%r14), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rcx, 704(%rsp)
	vmovdqu64	-72(%r14), %zmm0
	vmovdqu64	%zmm0, 640(%rsp)
	cmpq	$-1, %rax
	je	.LBB363_761
	vmovdqu64	640(%rsp), %zmm0
	movq	%rax, 208(%rsp)
	movq	704(%rsp), %rax
	leaq	216(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp15974:
	movq	96(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp15975:
	cmpq	%r15, %rbx
	je	.LBB363_826
	leaq	-88(%r14), %rcx
	incq	%r15
	leaq	88(%r14), %rax
	addq	$88, %rcx
	cmpq	136(%rsp), %rcx
	jne	.LBB363_757
.LBB363_761:
	movq	%r15, 48(%rsp)
	movq	%r14, 80(%rsp)
	movq	%r14, 352(%rsp)
	jmp	.LBB363_839
.LBB363_762:
	movq	128(%rsp), %rax
	movq	144(%rsp), %rbp
	movq	$-1, %rbx
	addq	%rax, %r13
	addq	%rax, %r12
	jmp	.LBB363_766
.LBB363_763:
	movq	1112(%rsp), %r15
.LBB363_764:
	movq	$-1, %rbx
.LBB363_765:
	addq	$24, %r13
	cmpq	%r12, %r13
	je	.LBB363_753
.LBB363_766:
	movzbl	(%r13), %eax
	leaq	.LJTI363_1(%rip), %rcx
	movslq	(%rcx,%rax,4), %rax
	addq	%rcx, %rax
	jmpq	*%rax
.LBB363_767:
	cmpq	$-1, 1056(%rsp)
	je	.LBB363_765
	movzbl	1(%r13), %ebx
	movq	8(%r13), %r14
	movq	16(%r13), %r15
	movq	80(%rbp), %rax
	movq	$-1, %rdx
	.p2align	4
.LBB363_769:
	movq	%rax, %rcx
	addq	%r14, %rcx
	cmovbq	%rdx, %rcx
	lock		cmpxchgq	%rcx, 80(%rbp)
	jne	.LBB363_769
	movq	312(%rsp), %rcx
	addq	%r14, %rax
	cmovbq	%rdx, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB363_773
	movq	%rcx, 216(%rsp)
	movq	%rax, 224(%rsp)
	movw	$0, 208(%rsp)
.Ltmp15968:
	movq	312(%rsp), %rsi
	leaq	640(%rsp), %rdi
	leaq	208(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.10720091597982897309)
.Ltmp15969:
	cmpb	$-1, 640(%rsp)
	jne	.LBB363_867
.LBB363_773:
	cmpl	$255, %ebx
	je	.LBB363_763
	movq	2552(%rsp), %rcx
	movq	1112(%rsp), %r15
	movq	632(%rcx), %rax
	testq	%rax, %rax
	je	.LBB363_764
	movl	1228(%rcx), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB363_764
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	lock		addq	%r14, (%rcx,%rbx,8)
	jmp	.LBB363_764
.LBB363_777:
	movq	8(%r13), %rbx
	cmpq	%rbx, 48(%rsp)
	jae	.LBB363_803
	movq	80(%rsp), %rdx
	cmpq	136(%rsp), %rdx
	je	.LBB363_801
	addq	$88, %rdx
	leaq	-1(%rbx), %r14
	movq	%rdx, %rax
.LBB363_780:
	movq	%rax, %rdx
	movq	-8(%rdx), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rdx, 80(%rsp)
	movq	%rcx, 704(%rsp)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, 640(%rsp)
	cmpq	$-1, %rax
	je	.LBB363_800
	vmovdqu64	640(%rsp), %zmm0
	movq	%rax, 208(%rsp)
	movq	704(%rsp), %rax
	leaq	216(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp15957:
	movq	96(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp15958:
	movq	48(%rsp), %rsi
	cmpq	%rsi, %r14
	je	.LBB363_802
	movq	80(%rsp), %rdx
	incq	%rsi
	movq	%rsi, 48(%rsp)
	leaq	-88(%rdx), %rcx
	leaq	88(%rdx), %rax
	addq	$88, %rcx
	cmpq	136(%rsp), %rcx
	jne	.LBB363_780
	jmp	.LBB363_801
.LBB363_784:
	cmpq	$-1, %r15
	je	.LBB363_765
	movq	8(%r13), %rcx
	movl	296(%rbp), %eax
	testl	%eax, %eax
	je	.LBB363_795
	movq	104(%rbp), %rax
	addq	%rcx, %rax
	movq	40(%rbp), %rcx
	cmovbq	%rbx, %rax
	cmpq	%rcx, %rax
	jbe	.LBB363_765
	movq	%rcx, 216(%rsp)
	movq	%rax, 224(%rsp)
	movw	$768, 208(%rsp)
.Ltmp15955:
	movq	312(%rsp), %rsi
	leaq	640(%rsp), %rdi
	leaq	208(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.10720091597982897309)
.Ltmp15956:
	jmp	.LBB363_796
.LBB363_788:
	cmpq	$-1, %r15
	je	.LBB363_765
	cmpq	$-1, 40(%rbp)
	je	.LBB363_765
	movq	8(%r13), %rcx
	movq	16(%r13), %r14
	movl	296(%rbp), %eax
	testl	%eax, %eax
	je	.LBB363_797
	movq	104(%rbp), %rax
.LBB363_792:
	movq	%rax, %rdx
	addq	%rcx, %rdx
	cmovbq	%rbx, %rdx
	lock		cmpxchgq	%rdx, 104(%rbp)
	jne	.LBB363_792
	addq	%rcx, %rax
	movq	40(%rbp), %rcx
	cmovbq	%rbx, %rax
	cmpq	%rcx, %rax
	jbe	.LBB363_765
	movq	%rcx, 216(%rsp)
	movq	%rax, 224(%rsp)
	movw	$768, 208(%rsp)
.Ltmp15962:
	movq	312(%rsp), %rsi
	leaq	640(%rsp), %rdi
	leaq	208(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.10720091597982897309)
.Ltmp15963:
	jmp	.LBB363_798
.LBB363_795:
	movq	1184(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 656(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 640(%rsp)
.LBB363_796:
	cmpb	$-1, 640(%rsp)
	je	.LBB363_765
	jmp	.LBB363_900
.LBB363_797:
	movq	1184(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 656(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 640(%rsp)
.LBB363_798:
	movzbl	640(%rsp), %eax
	cmpb	$-1, %al
	sete	%cl
	testq	%r14, %r14
	sete	%dl
	orb	%cl, %dl
	je	.LBB363_876
	cmpb	$-1, %al
	je	.LBB363_765
	jmp	.LBB363_900
.LBB363_800:
	movq	80(%rsp), %rdx
.LBB363_801:
	movq	%rdx, 80(%rsp)
	movq	%rdx, 352(%rsp)
	jmp	.LBB363_804
.LBB363_802:
	movq	%rbx, 48(%rsp)
.LBB363_803:
	movq	80(%rsp), %rax
	movq	%rax, 352(%rsp)
.LBB363_804:
	movq	$-1, %rbx
	cmpq	$-1, %r15
	je	.LBB363_765
.Ltmp15960:
	movq	2552(%rsp), %rsi
	leaq	208(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp15961:
	cmpb	$-1, 208(%rsp)
	je	.LBB363_765
	jmp	.LBB363_900
.LBB363_807:
	cmpq	$-1, %r15
	je	.LBB363_839
	cmpq	$-1, 40(%rbp)
	je	.LBB363_813
.Ltmp15945:
	movq	312(%rsp), %rsi
	movq	376(%rsp), %rcx
	leaq	208(%rsp), %rdi
	movl	$3, %edx
	xorl	%r8d, %r8d
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.10720091597982897309)
.Ltmp15946:
	cmpb	$-1, 208(%rsp)
	je	.LBB363_813
	movb	$1, %r14b
	jmp	.LBB363_830
.LBB363_812:
	movq	616(%rsp), %r12
	jmp	.LBB363_839
.LBB363_813:
	testb	%r14b, %r14b
	jne	.LBB363_816
.Ltmp15947:
	movq	2552(%rsp), %rsi
	leaq	208(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp15948:
	cmpb	$-1, 208(%rsp)
	movb	$1, %r14b
	jne	.LBB363_830
.LBB363_816:
	movq	1120(%rsp), %rcx
	testq	%rcx, %rcx
	je	.LBB363_829
.Ltmp15949:
	movq	312(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::admit_transient@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdi
	movl	$3, %edx
	vzeroupper
	callq	*%rax
.Ltmp15950:
	cmpb	$-1, 208(%rsp)
	setne	%r14b
	jmp	.LBB363_830
.LBB363_819:
	movq	128(%rsp), %rax
	movq	-16(%rax,%r12), %rbx
	cmpq	%rbx, 48(%rsp)
	jae	.LBB363_827
	movq	80(%rsp), %rdx
	movq	616(%rsp), %r12
	cmpq	136(%rsp), %rdx
	je	.LBB363_851
	addq	$88, %rdx
	leaq	-1(%rbx), %r14
	movq	%rdx, %rax
.LBB363_822:
	movq	%rax, %rdx
	movq	-8(%rdx), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rdx, 80(%rsp)
	movq	%rcx, 704(%rsp)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, 640(%rsp)
	cmpq	$-1, %rax
	je	.LBB363_850
	vmovdqu64	640(%rsp), %zmm0
	movq	%rax, 208(%rsp)
	movq	704(%rsp), %rax
	leaq	216(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp15940:
	movq	96(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp15941:
	movq	48(%rsp), %rsi
	cmpq	%rsi, %r14
	je	.LBB363_852
	movq	80(%rsp), %rdx
	incq	%rsi
	movq	%rsi, 48(%rsp)
	leaq	-88(%rdx), %rcx
	leaq	88(%rdx), %rax
	addq	$88, %rcx
	cmpq	136(%rsp), %rcx
	jne	.LBB363_822
	jmp	.LBB363_851
.LBB363_826:
	movq	40(%rsp), %rax
	movq	%r14, 80(%rsp)
	movq	%r14, 352(%rsp)
	movq	%rax, 48(%rsp)
	jmp	.LBB363_839
.LBB363_827:
	movq	616(%rsp), %r12
.LBB363_828:
	movq	80(%rsp), %rax
	xorl	%r14d, %r14d
	movq	%rax, 352(%rsp)
	jmp	.LBB363_739
.LBB363_829:
	xorl	%r14d, %r14d
.LBB363_830:
	movq	40(%rsp), %rax
	cmpq	%rax, 48(%rsp)
	jae	.LBB363_837
	movq	80(%rsp), %r15
	cmpq	136(%rsp), %r15
	je	.LBB363_838
	movq	40(%rsp), %rax
	addq	$88, %r15
	leaq	-1(%rax), %rbx
	movq	%r15, %rax
.LBB363_833:
	movq	%rax, %r15
	movq	-8(%r15), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rcx, 704(%rsp)
	vmovdqu64	-72(%r15), %zmm0
	vmovdqu64	%zmm0, 640(%rsp)
	cmpq	$-1, %rax
	je	.LBB363_838
	vmovdqu64	640(%rsp), %zmm0
	movq	%rax, 208(%rsp)
	movq	704(%rsp), %rax
	leaq	216(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp15952:
	movq	96(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp15953:
	movq	48(%rsp), %rdx
	cmpq	%rdx, %rbx
	je	.LBB363_849
	leaq	-88(%r15), %rcx
	incq	%rdx
	leaq	88(%r15), %rax
	addq	$88, %rcx
	movq	%rdx, 48(%rsp)
	cmpq	136(%rsp), %rcx
	jne	.LBB363_833
	jmp	.LBB363_838
.LBB363_837:
	movq	80(%rsp), %r15
.LBB363_838:
	movq	%r15, 80(%rsp)
	movq	%r15, 352(%rsp)
	testb	%r14b, %r14b
	jne	.LBB363_900
.LBB363_839:
	movq	984(%rsp), %r15
	cmpq	$0, 72(%rsp)
	je	.LBB363_704
	movq	1000(%rsp), %rax
	movq	%rax, 1120(%rsp)
	jmp	.LBB363_842
.LBB363_841:
	movq	192(%rsp), %rax
	movq	40(%rsp), %rdx
	leaq	(%rbx,%rbx,4), %rcx
	shll	$16, %r14d
	movq	72(%rsp), %rsi
	movq	336(%rsp), %rdi
	addq	$40, %r15
	incq	%rbx
	orl	%r14d, %r12d
	shlq	$32, %r12
	orq	%r12, %rbp
	movq	616(%rsp), %r12
	movq	%rdx, (%rax,%rcx,8)
	movq	152(%rsp), %rdx
	decq	%rsi
	movq	%rsi, 72(%rsp)
	movq	%rdx, 8(%rax,%rcx,8)
	movq	376(%rsp), %rdx
	movq	%rdx, 16(%rax,%rcx,8)
	movq	%rbp, %rdx
	movb	%r13b, 24(%rax,%rcx,8)
	movl	%ebp, 25(%rax,%rcx,8)
	shrq	$32, %rbp
	shrq	$48, %rdx
	movw	%bp, 29(%rax,%rcx,8)
	movq	144(%rsp), %rbp
	movb	%dl, 31(%rax,%rcx,8)
	movq	%rdi, 32(%rax,%rcx,8)
	movq	%rbx, 200(%rsp)
	testq	%rsi, %rsi
	je	.LBB363_704
.LBB363_842:
	cmpq	1120(%rsp), %r15
	je	.LBB363_704
	movq	32(%r15), %rax
	leaq	648(%rsp), %rcx
	movq	%rax, 32(%rcx)
	movq	2552(%rsp), %rax
	vmovdqu	(%r15), %ymm0
	vmovdqu	%ymm0, (%rcx)
	movq	%rax, 640(%rsp)
	cmpq	$0, 648(%rsp)
	je	.LBB363_845
	movq	32(%r15), %rax
	leaq	216(%rsp), %rcx
	movq	%rax, 32(%rcx)
	vmovdqu	(%r15), %ymm0
	vmovdqu	%ymm0, (%rcx)
	jmp	.LBB363_847
.LBB363_845:
	movq	664(%rax), %rdx
.Ltmp15977:
	movq	96(%rsp), %rsi
	leaq	208(%rsp), %rdi
	leaq	656(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp15978:
	movq	208(%rsp), %r13
	cmpq	$-1, %r13
	jne	.LBB363_865
.LBB363_847:
	movq	224(%rsp), %rcx
	movq	216(%rsp), %rax
	movq	232(%rsp), %rdx
	movzbl	240(%rsp), %r13d
	movzbl	247(%rsp), %r14d
	movzwl	245(%rsp), %r12d
	movl	241(%rsp), %ebp
	movq	200(%rsp), %rbx
	movq	%rcx, 152(%rsp)
	movq	248(%rsp), %rcx
	movq	%rax, 40(%rsp)
	movq	%rdx, 376(%rsp)
	movq	%rcx, 336(%rsp)
	cmpq	184(%rsp), %rbx
	jne	.LBB363_841
.Ltmp15987:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	184(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15988:
	jmp	.LBB363_841
.LBB363_849:
	movq	40(%rsp), %rax
	movq	%rax, 48(%rsp)
	movq	%r15, 80(%rsp)
	movq	%r15, 352(%rsp)
	testb	%r14b, %r14b
	je	.LBB363_839
	jmp	.LBB363_900
.LBB363_850:
	movq	80(%rsp), %rdx
.LBB363_851:
	movq	%rdx, 80(%rsp)
	movq	%rdx, 352(%rsp)
	xorl	%r14d, %r14d
	jmp	.LBB363_739
.LBB363_852:
	movq	%rbx, 48(%rsp)
	jmp	.LBB363_828
.LBB363_853:
	testq	%r15, %r15
	je	.LBB363_855
	movq	1096(%rsp), %rdi
	shlq	$5, %r15
	movl	$8, %edx
	movq	%r15, %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB363_855:
.Ltmp15997:
	leaq	344(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp15998:
	testq	%r14, %r14
	je	.LBB363_858
	movq	128(%rsp), %rdi
	shlq	$3, %r14
	movl	$8, %edx
	leaq	(%r14,%r14,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB363_858:
	movq	472(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_861
	lock		decq	(%rax)
	jne	.LBB363_861
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	472(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB363_861:
	movq	504(%rsp), %rax
	movq	1896(%rsp), %r14
	testq	%rax, %rax
	je	.LBB363_864
	lock		decq	(%rax)
	jne	.LBB363_864
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	504(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB363_864:
	cmpq	1536(%rsp), %r14
	jne	.LBB363_701
	jmp	.LBB363_585
.LBB363_865:
	movq	216(%rsp), %rax
	vmovups	256(%rsp), %ymm0
	movzwl	245(%rsp), %ecx
	movl	241(%rsp), %r12d
	movq	248(%rsp), %rbx
	movq	144(%rsp), %rbp
	addq	$40, %r15
	movq	%r15, 984(%rsp)
	movq	%rax, 96(%rsp)
	movq	224(%rsp), %rax
	vmovups	%ymm0, 1216(%rsp)
	vmovdqu	272(%rsp), %ymm0
	movq	%rax, 48(%rsp)
	movq	232(%rsp), %rax
	movq	%rax, 80(%rsp)
	movzbl	240(%rsp), %eax
	vmovdqu	%ymm0, 1232(%rsp)
	movq	%rax, 72(%rsp)
	movzbl	247(%rsp), %eax
	shll	$16, %eax
	orl	%eax, %ecx
	movb	$1, %al
	shlq	$32, %rcx
	movl	%eax, 40(%rsp)
	orq	%rcx, %r12
	jmp	.LBB363_901
.LBB363_866:
.Ltmp16114:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.536(%rip), %rcx
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
.Ltmp16115:
	jmp	.LBB363_944
.LBB363_867:
	testq	%r15, %r15
	je	.LBB363_900
	leaq	-1(%r15), %rax
	cmpq	%rax, 48(%rsp)
	jae	.LBB363_898
	movq	80(%rsp), %rbx
	cmpq	136(%rsp), %rbx
	je	.LBB363_875
	movq	48(%rsp), %rax
	addq	$88, %rbx
	leaq	208(%rsp), %r14
	notq	%rax
	addq	%r15, %rax
	movq	%rax, %r15
	movq	%rbx, %rax
.LBB363_871:
	movq	%rax, %rbx
	movq	-8(%rbx), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rcx, 704(%rsp)
	vmovdqu64	-72(%rbx), %zmm0
	vmovdqu64	%zmm0, 640(%rsp)
	cmpq	$-1, %rax
	je	.LBB363_875
	vmovdqu64	640(%rsp), %zmm0
	movq	%rax, 208(%rsp)
	movq	704(%rsp), %rax
	leaq	216(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp15971:
	movq	96(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%r14, %rdx
	vzeroupper
	callq	*%rax
.Ltmp15972:
	decq	%r15
	je	.LBB363_897
	leaq	-88(%rbx), %rcx
	leaq	88(%rbx), %rax
	addq	$88, %rcx
	cmpq	136(%rsp), %rcx
	jne	.LBB363_871
.LBB363_875:
	movq	144(%rsp), %rbp
	movq	%rbx, 352(%rsp)
	jmp	.LBB363_900
.LBB363_876:
	movq	48(%rsp), %r15
	leaq	-1(%r14), %rax
	cmpq	%rax, %r15
	jae	.LBB363_892
	movq	80(%rsp), %rbx
	cmpq	136(%rsp), %rbx
	je	.LBB363_883
	notq	%r15
	addq	$88, %rbx
	addq	%r14, %r15
	leaq	208(%rsp), %r14
	movq	%rbx, %rax
.LBB363_879:
	movq	%rax, %rbx
	movq	-8(%rbx), %rcx
	movq	-88(%rax), %rsi
	movq	-80(%rax), %rax
	movq	%rcx, 704(%rsp)
	vmovdqu64	-72(%rbx), %zmm0
	vmovdqu64	%zmm0, 640(%rsp)
	cmpq	$-1, %rax
	je	.LBB363_883
	vmovdqu64	640(%rsp), %zmm0
	movq	%rax, 208(%rsp)
	movq	704(%rsp), %rax
	leaq	216(%rsp), %rcx
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp15965:
	movq	96(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%r14, %rdx
	vzeroupper
	callq	*%rax
.Ltmp15966:
	decq	%r15
	je	.LBB363_891
	leaq	-88(%rbx), %rcx
	leaq	88(%rbx), %rax
	addq	$88, %rcx
	cmpq	136(%rsp), %rcx
	jne	.LBB363_879
.LBB363_883:
	movq	%rbx, 352(%rsp)
	jmp	.LBB363_900
.LBB363_884:
	cmpq	$21, %r15
	jae	.LBB363_945
	movq	48(%rsp), %rdi
	movq	%r15, %rsi
	callq	core::slice::sort::shared::smallsort::insertion_sort_shift_left::<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>)), <[(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>))]>::sort_unstable_by_key<usize, purrdf_sparql_eval::modifier::contextual_group_rows<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#3}>::{closure#0}>
	jmp	.LBB363_186
.LBB363_886:
.Ltmp15803:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %r8
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.529(%rip), %rcx
	movq	%rdx, %rsi
	xorl	%edi, %edi
	movq	%rax, %rdx
	callq	*%r8
.Ltmp15804:
	jmp	.LBB363_944
.LBB363_887:
.Ltmp15793:
	movq	core::slice::copy_from_slice_impl::len_mismatch_fail@GOTPCREL(%rip), %rcx
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.530(%rip), %rax
	movq	%rdx, %rdi
	movq	%rax, %rdx
	callq	*%rcx
.Ltmp15794:
	jmp	.LBB363_944
.LBB363_888:
	movq	184(%rsp), %rax
	movq	192(%rsp), %rdx
	movq	200(%rsp), %rcx
	movq	$-1, %r13
	movq	$0, 72(%rsp)
	movl	$0, 40(%rsp)
	movq	%rax, 96(%rsp)
	movq	%rdx, 48(%rsp)
	movq	%rcx, 80(%rsp)
	jmp	.LBB363_902
.LBB363_889:
.Ltmp15743:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.786(%rip), %rdx
	vzeroupper
	callq	*%rax
.Ltmp15744:
	jmp	.LBB363_944
.LBB363_890:
.Ltmp15740:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.786(%rip), %rdx
	vzeroupper
	callq	*%rax
.Ltmp15741:
	jmp	.LBB363_944
.LBB363_891:
	movq	%rbx, 80(%rsp)
.LBB363_892:
	movq	80(%rsp), %rax
	jmp	.LBB363_899
.LBB363_893:
.Ltmp15814:
	leaq	976(%rsp), %rdi
	movl	$16, %ecx
	movl	$224, %r8d
	xorl	%esi, %esi
	movq	%r15, %rdx
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.10720091597982897309)
.Ltmp15815:
	movq	976(%rsp), %rax
	movq	992(%rsp), %r13
	subq	%r13, %rax
	cmpq	%r15, %rax
	jae	.LBB363_327
.LBB363_895:
.Ltmp15828:
	movq	core::panicking::panic@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.1066(%rip), %rdi
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.1068(%rip), %rdx
	movl	$47, %esi
	callq	*%rax
.Ltmp15829:
	jmp	.LBB363_944
.LBB363_896:
	leaq	1968(%rsp), %rax
	movq	%rax, 720(%rsp)
	movq	<usize as core::fmt::Display>::fmt@GOTPCREL(%rip), %rax
	movq	%rax, 728(%rsp)
	movq	%rbx, 736(%rsp)
	movq	%rax, 744(%rsp)
.Ltmp15820:
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.619(%rip), %rdi
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.621(%rip), %rdx
	leaq	720(%rsp), %rsi
	callq	*%rax
.Ltmp15821:
	jmp	.LBB363_944
.LBB363_897:
	movq	%rbx, 80(%rsp)
.LBB363_898:
	movq	80(%rsp), %rax
	movq	144(%rsp), %rbp
.LBB363_899:
	movq	%rax, 352(%rsp)
.LBB363_900:
	movq	184(%rsp), %rax
	movq	192(%rsp), %rdx
	movq	200(%rsp), %rcx
	movq	$-1, %r13
	movl	$0, 40(%rsp)
	movq	%rax, 96(%rsp)
	movb	$1, %al
	movq	%rdx, 48(%rsp)
	movq	%rcx, 80(%rsp)
	movq	%rax, 72(%rsp)
.LBB363_901:
	movq	944(%rsp), %r14
	movq	936(%rsp), %r15
.LBB363_902:
	testq	%r15, %r15
	je	.LBB363_904
	movq	1096(%rsp), %rdi
	shlq	$5, %r15
	movl	$8, %edx
	movq	%r15, %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB363_904:
.Ltmp15980:
	leaq	344(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp15981:
	testq	%r14, %r14
	je	.LBB363_907
	movq	128(%rsp), %rdi
	shlq	$3, %r14
	movl	$8, %edx
	leaq	(%r14,%r14,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB363_907:
	movq	472(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_910
	lock		decq	(%rax)
	jne	.LBB363_910
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	472(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB363_910:
	movq	504(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_913
	lock		decq	(%rax)
	jne	.LBB363_913
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	504(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB363_913:
	xorl	%r14d, %r14d
.Ltmp15983:
	leaq	1008(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp15984:
	lock		decq	(%rbp)
	jne	.LBB363_916
	xorl	%r14d, %r14d
	#MEMBARRIER
.Ltmp15985:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1208(%rsp), %rdi
	callq	*%rax
.Ltmp15986:
.LBB363_916:
	movl	40(%rsp), %r15d
	xorl	%r14d, %r14d
.LBB363_917:
	cmpq	$0, 1544(%rsp)
	movl	172(%rsp), %ebp
	je	.LBB363_919
.Ltmp16016:
	leaq	976(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16017:
.LBB363_919:
	testb	%r15b, %r15b
	je	.LBB363_927
	movq	192(%rsp), %rax
	movq	200(%rsp), %r15
	movq	%rax, 40(%rsp)
	testq	%r15, %r15
	je	.LBB363_925
	movq	40(%rsp), %rax
	leaq	8(%rax), %rbp
	jmp	.LBB363_923
.LBB363_922:
	addq	$40, %rbp
	decq	%r15
	je	.LBB363_925
.LBB363_923:
	movq	-8(%rbp), %rax
	cmpq	$6, %rax
	jb	.LBB363_922
	movq	(%rbp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB363_922
.LBB363_925:
	movq	184(%rsp), %rax
	movl	172(%rsp), %ebp
	testq	%rax, %rax
	je	.LBB363_927
	movq	40(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
.LBB363_927:
	testb	%r14b, %r14b
	jne	.LBB363_454
	jmp	.LBB363_460
.LBB363_928:
.Ltmp15990:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.290(%rip), %rcx
	movq	%r13, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15991:
	jmp	.LBB363_944
.LBB363_929:
.Ltmp16064:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %r8
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.532(%rip), %rcx
	movq	%rdx, %rsi
	xorl	%edi, %edi
	movq	%rax, %rdx
	callq	*%r8
.Ltmp16065:
	jmp	.LBB363_944
.LBB363_930:
.Ltmp16054:
	movq	core::slice::copy_from_slice_impl::len_mismatch_fail@GOTPCREL(%rip), %rcx
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.533(%rip), %rax
	movq	%rdx, %rdi
	movq	%rax, %rdx
	callq	*%rcx
.Ltmp16055:
	jmp	.LBB363_944
.LBB363_931:
	leaq	1936(%rsp), %rax
	leaq	640(%rsp), %rcx
	movq	%r13, 1936(%rsp)
	movq	%rdx, 640(%rsp)
	movq	%rax, 208(%rsp)
	leaq	<usize as core::fmt::Debug>::fmt(%rip), %rax
	movq	%rax, 216(%rsp)
	movq	%rcx, 224(%rsp)
	movq	%rax, 232(%rsp)
.Ltmp15992:
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.2158(%rip), %rdi
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.289(%rip), %rdx
	leaq	208(%rsp), %rsi
	vzeroupper
	callq	*%rax
.Ltmp15993:
	jmp	.LBB363_944
.LBB363_932:
.Ltmp16085:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.535(%rip), %rcx
	movq	%r14, %rdi
	movq	%rdx, %rsi
	callq	*%rax
.Ltmp16086:
	jmp	.LBB363_944
.LBB363_933:
.Ltmp15751:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$8, %esi
	callq	*%rax
.Ltmp15752:
	jmp	.LBB363_944
.LBB363_934:
.Ltmp15854:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$16, %edi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp15855:
	jmp	.LBB363_944
.LBB363_935:
.Ltmp15798:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.531(%rip), %rdx
	callq	*%rax
.Ltmp15799:
	jmp	.LBB363_944
.LBB363_936:
	movb	$1, %al
	movl	%eax, 40(%rsp)
.Ltmp15930:
	movl	$8, %ecx
	movl	$80, %r8d
	movb	$1, %r14b
	movq	%r15, %rdi
	movq	%rbx, %rdx
	vzeroupper
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.10720091597982897309)
.Ltmp15931:
	jmp	.LBB363_698
.LBB363_937:
	leaq	1000(%rax), %rdi
	movb	$1, %al
	movl	%eax, 40(%rsp)
.Ltmp15932:
	movq	<hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	movb	$1, %r14b
	movq	%rbx, %rsi
	movq	%r15, %rdx
	vzeroupper
	callq	*%rax
.Ltmp15933:
	jmp	.LBB363_699
.LBB363_938:
.Ltmp16129:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	callq	*%rax
.Ltmp16130:
	jmp	.LBB363_944
.LBB363_939:
.Ltmp15721:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$1, %edi
	movl	$51, %esi
	callq	*%rax
.Ltmp15722:
	jmp	.LBB363_944
.LBB363_940:
	addq	$16, %rdi
.Ltmp16038:
	leaq	384(%rsp), %rsi
	callq	<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::initialize::<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::get_or_init<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::try_insert::{closure#0}>::{closure#0}, !> (.llvm.10720091597982897309)
.Ltmp16039:
	jmp	.LBB363_598
.LBB363_941:
.Ltmp16059:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.534(%rip), %rdx
	callq	*%rax
.Ltmp16060:
	jmp	.LBB363_944
.LBB363_942:
.Ltmp15735:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp15736:
	jmp	.LBB363_944
.LBB363_943:
.Ltmp15806:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp15807:
.LBB363_944:
	ud2
.LBB363_945:
.Ltmp15763:
	movq	core::slice::sort::unstable::ipnsort::<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>)), <[(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>))]>::sort_unstable_by_key<usize, purrdf_sparql_eval::modifier::contextual_group_rows<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#3}>::{closure#0}>@GOTPCREL(%rip), %rax
	movq	48(%rsp), %rdi
	movq	%r15, %rsi
	callq	*%rax
.Ltmp15764:
	jmp	.LBB363_186
.LBB363_946:
.Ltmp16040:
	leaq	1264(%rsp), %rdi
	movq	%rax, %r12
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB363_1103
.LBB363_947:
.Ltmp15973:
	jmp	.LBB363_949
.LBB363_948:
.Ltmp15967:
.LBB363_949:
	movq	%rax, %r12
	movq	%rbx, 352(%rsp)
	jmp	.LBB363_1083
.LBB363_950:
.Ltmp15999:
	movq	%rax, %r12
	movb	$1, %al
	movl	%eax, 40(%rsp)
	jmp	.LBB363_1086
.LBB363_951:
.Ltmp15982:
	movq	%rax, %r12
	jmp	.LBB363_1086
.LBB363_952:
.Ltmp15739:
	jmp	.LBB363_1109
.LBB363_953:
.Ltmp15936:
	jmp	.LBB363_1082
.LBB363_954:
.Ltmp15942:
	jmp	.LBB363_1009
.LBB363_955:
.Ltmp16018:
	movq	%rax, %r12
	movl	%r15d, 40(%rsp)
	jmp	.LBB363_1099
.LBB363_956:
.Ltmp16004:
	movq	%rax, %r12
	jmp	.LBB363_1096
.LBB363_957:
.Ltmp15951:
	jmp	.LBB363_1082
.LBB363_958:
.Ltmp16026:
	cmpq	$0, 1544(%rsp)
	movq	%rax, %r12
	jne	.LBB363_1098
	jmp	.LBB363_1099
.LBB363_959:
.Ltmp15970:
	jmp	.LBB363_1082
.LBB363_960:
.Ltmp15899:
	movq	%rax, %r12
	jmp	.LBB363_993
.LBB363_961:
.Ltmp15954:
	movq	%rax, %r12
	movq	%r15, 352(%rsp)
	jmp	.LBB363_1083
.LBB363_962:
.Ltmp15929:
	movq	%rax, %r12
	jmp	.LBB363_1102
.LBB363_1001:
.Ltmp15885:
	movq	%rax, %r12
	jmp	.LBB363_1002
.LBB363_963:
.Ltmp16063:
	jmp	.LBB363_1067
.LBB363_964:
.Ltmp16013:
	addq	$40, %r14
	movq	%rax, %r12
	movq	%r14, 216(%rsp)
	jmp	.LBB363_977
.LBB363_965:
.Ltmp16073:
	leaq	1472(%rsp), %rdi
	movq	%rax, %rbx
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movb	$1, %bpl
	jmp	.LBB363_967
.LBB363_966:
.Ltmp16076:
	movq	%rax, %rbx
	xorl	%ebp, %ebp
.LBB363_967:
	xorl	%r15d, %r15d
	jmp	.LBB363_1130
.LBB363_968:
.Ltmp16053:
	jmp	.LBB363_1067
.LBB363_969:
.Ltmp15924:
	movq	%rax, %r12
.Ltmp15925:
	leaq	1448(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15926:
	jmp	.LBB363_1102
.LBB363_970:
.Ltmp15902:
	jmp	.LBB363_982
.LBB363_971:
.Ltmp15913:
	movq	%rax, %r12
	jmp	.LBB363_998
.LBB363_972:
.Ltmp15880:
	movq	%rax, %r12
.Ltmp15881:
	leaq	1448(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15882:
	jmp	.LBB363_1002
.LBB363_973:
.Ltmp15844:
	movq	%rax, %rbx
.Ltmp15845:
	leaq	344(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>>>
.Ltmp15846:
	jmp	.LBB363_1124
.LBB363_974:
.Ltmp15847:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_975:
.Ltmp16021:
	addq	$40, %r14
	cmpq	$6, 72(%rsp)
	movq	%rax, %r12
	movq	%r14, 216(%rsp)
	jb	.LBB363_977
	movq	72(%rsp), %rax
	movq	40(%rsp), %rdi
	movl	$4, %edx
	leaq	-8(,%rax,8), %rsi
	callq	__rustc::__rust_dealloc
.LBB363_977:
	movb	$1, %r14b
.Ltmp16022:
	leaq	208(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16023:
	jmp	.LBB363_1100
.LBB363_978:
.Ltmp15964:
	jmp	.LBB363_1082
.LBB363_979:
.Ltmp15976:
	movq	%rax, %r12
	movq	%r14, 352(%rsp)
	jmp	.LBB363_1083
.LBB363_980:
.Ltmp15786:
	jmp	.LBB363_1037
.LBB363_981:
.Ltmp16043:
.LBB363_982:
	movq	%rax, %r12
	jmp	.LBB363_1103
.LBB363_983:
.Ltmp15866:
	jmp	.LBB363_1123
.LBB363_984:
.Ltmp15979:
	addq	$40, %r15
	movq	%rax, %r12
	movq	%r15, 984(%rsp)
	jmp	.LBB363_1083
.LBB363_985:
.Ltmp16048:
	jmp	.LBB363_1012
.LBB363_986:
.Ltmp15888:
	movq	%rax, %r12
	movq	%r14, 216(%rsp)
	jmp	.LBB363_992
.LBB363_987:
.Ltmp15748:
	jmp	.LBB363_1109
.LBB363_988:
.Ltmp15989:
	addq	$40, %r15
	cmpq	$6, 40(%rsp)
	movq	%rax, %r12
	movq	%r15, 984(%rsp)
	jb	.LBB363_1083
	movq	40(%rsp), %rax
	movq	152(%rsp), %rdi
	movl	$4, %edx
	leaq	-8(,%rax,8), %rsi
	callq	__rustc::__rust_dealloc
	jmp	.LBB363_1083
.LBB363_990:
.Ltmp15893:
	movq	%rax, %r12
	movq	%r14, 216(%rsp)
	cmpq	$6, %rbx
	jb	.LBB363_992
	movq	48(%rsp), %rdi
	leaq	-8(,%rbx,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB363_992:
.Ltmp15894:
	leaq	208(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15895:
.LBB363_993:
	leaq	640(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB363_999
.LBB363_994:
.Ltmp15896:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_995:
.Ltmp15875:
	movq	%rax, %rbx
.Ltmp15876:
	leaq	1448(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15877:
	jmp	.LBB363_1126
.LBB363_996:
.Ltmp15905:
	movq	%rax, %r12
.Ltmp15906:
	leaq	1264(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp15907:
.Ltmp15909:
	leaq	384(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
.Ltmp15910:
.LBB363_998:
.Ltmp15914:
	leaq	208(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp15915:
.LBB363_999:
	cmpb	$0, 72(%rsp)
	je	.LBB363_1002
.Ltmp15919:
	leaq	1504(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15920:
	jmp	.LBB363_1103
.LBB363_1002:
.Ltmp15917:
	leaq	720(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
.Ltmp15918:
	jmp	.LBB363_1103
.LBB363_1003:
.Ltmp15908:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_1004:
.Ltmp15916:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_1005:
.Ltmp15921:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_1006:
.Ltmp15802:
	movq	%rax, %rbx
	cmpq	$6, %r13
	jb	.LBB363_1118
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
	movq	%r14, %rdi
	jmp	.LBB363_1117
.LBB363_1008:
.Ltmp15959:
.LBB363_1009:
	movq	80(%rsp), %rcx
	movq	%rax, %r12
	movq	%rcx, 352(%rsp)
	jmp	.LBB363_1083
.LBB363_1010:
.Ltmp16108:
	movq	%rax, %rbx
	jmp	.LBB363_1150
.LBB363_1011:
.Ltmp15872:
.LBB363_1012:
	movq	%rax, %rbx
	jmp	.LBB363_1126
.LBB363_1013:
.Ltmp15759:
	jmp	.LBB363_1069
.LBB363_1014:
.Ltmp15850:
	movq	%rax, %rbx
	testq	%r12, %r12
	je	.LBB363_1052
	negq	%r12
	jmp	.LBB363_1017
.LBB363_1016:
	addq	$240, %r14
	decq	%r12
	je	.LBB363_1052
.LBB363_1017:
	cmpl	$2, (%r14)
	je	.LBB363_1016
.Ltmp15851:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>
.Ltmp15852:
	jmp	.LBB363_1016
.LBB363_1019:
.Ltmp15853:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_1020:
.Ltmp16092:
	movq	%rax, %rbx
	jmp	.LBB363_1129
.LBB363_1021:
.Ltmp15939:
	jmp	.LBB363_1082
.LBB363_1022:
.Ltmp16058:
	movq	1592(%rsp), %r13
	jmp	.LBB363_1078
.LBB363_1023:
.Ltmp15792:
	movq	%rax, %rbx
	jmp	.LBB363_1118
.LBB363_1024:
.Ltmp15789:
	jmp	.LBB363_1037
.LBB363_1025:
.Ltmp15762:
	leaq	384(%rsp), %rdi
	movq	%r12, 752(%rsp)
	movq	%r15, 744(%rsp)
	movw	%bp, 768(%rsp)
	movq	%rax, %rbx
	movq	%r14, 776(%rsp)
	callq	core::ptr::drop_glue::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>
	leaq	720(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>
	leaq	208(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>>
	jmp	.LBB363_1137
.LBB363_1026:
.Ltmp16097:
	movq	%rax, %rbx
	jmp	.LBB363_1119
.LBB363_1027:
.Ltmp16034:
	movq	%rax, %r12
	testq	%r15, %r15
	je	.LBB363_1031
	negq	%r15
	addq	$160, %r14
.LBB363_1029:
.Ltmp16035:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp16036:
	addq	$160, %r14
	decq	%r15
	jne	.LBB363_1029
.LBB363_1031:
	cmpq	$0, 1040(%rsp)
	je	.LBB363_1103
	movq	1040(%rsp), %rax
	movq	1104(%rsp), %rdi
	movl	$8, %edx
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
	jmp	.LBB363_1103
.LBB363_1033:
.Ltmp16037:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_1034:
.Ltmp15723:
	jmp	.LBB363_1071
.LBB363_1035:
.Ltmp15781:
	jmp	.LBB363_1037
.LBB363_1036:
.Ltmp15774:
.LBB363_1037:
	movq	%rax, %rbx
	movb	$1, %bpl
	movb	$1, %r15b
	jmp	.LBB363_1131
.LBB363_1038:
.Ltmp16131:
	movq	%rax, %rbx
.Ltmp16132:
	leaq	400(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.10720091597982897309)
.Ltmp16133:
	jmp	.LBB363_1162
.LBB363_1039:
.Ltmp16134:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_1040:
.Ltmp16079:
	movq	%rax, %rbx
	testq	%r12, %r12
	je	.LBB363_1044
	negq	%r12
	addq	$24, %r15
.LBB363_1042:
.Ltmp16080:
	movq	%r15, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
.Ltmp16081:
	addq	$24, %r15
	decq	%r12
	jne	.LBB363_1042
.LBB363_1044:
	movq	1128(%rsp), %rax
	xorl	%ebp, %ebp
	testq	%rax, %rax
	je	.LBB363_1046
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r14, %rdi
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
	xorl	%ebp, %ebp
.LBB363_1046:
	xorl	%r15d, %r15d
	jmp	.LBB363_1131
.LBB363_1047:
.Ltmp16082:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_1048:
.Ltmp16125:
	movq	%rax, %rbx
	jmp	.LBB363_1159
.LBB363_1049:
.Ltmp16111:
	movq	%rax, %rbx
	jmp	.LBB363_1152
.LBB363_1050:
.Ltmp15797:
	movq	720(%rsp), %r13
	jmp	.LBB363_1115
.LBB363_1051:
.Ltmp15856:
	movq	%rax, %rbx
.LBB363_1052:
.Ltmp15857:
	leaq	640(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>, core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>
.Ltmp15858:
.Ltmp15859:
	leaq	720(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>>>
.Ltmp15860:
	jmp	.LBB363_1124
.LBB363_1054:
.Ltmp15861:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_1055:
.Ltmp16102:
	movl	%ebx, %r13d
	movq	%rax, %rbx
	testq	%r12, %r12
	je	.LBB363_1059
	negq	%r12
	addq	$24, %r15
.LBB363_1057:
.Ltmp16103:
	movq	%r15, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
.Ltmp16104:
	addq	$24, %r15
	decq	%r12
	jne	.LBB363_1057
.LBB363_1059:
	movq	1128(%rsp), %rax
	movb	$1, %bpl
	testq	%rax, %rax
	jne	.LBB363_1061
	movl	%r13d, %r15d
	jmp	.LBB363_1131
.LBB363_1061:
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r14, %rdi
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
	movl	%r13d, %r15d
	jmp	.LBB363_1131
.LBB363_1062:
.Ltmp16105:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_1063:
.Ltmp15753:
	cmpq	$6, 96(%rsp)
	movq	%rax, %rbx
	jb	.LBB363_1113
	movq	96(%rsp), %rax
	movq	152(%rsp), %rdi
	movl	$4, %edx
	leaq	-8(,%rax,8), %rsi
	jmp	.LBB363_1112
.LBB363_1065:
.Ltmp15734:
	movq	176(%rsp), %rdi
	movq	952(%rsp), %rsi
	movl	$8, %edx
	movq	%rax, %rbx
	callq	__rustc::__rust_dealloc
	jmp	.LBB363_1121
.LBB363_1066:
.Ltmp16087:
.LBB363_1067:
	movq	%rax, %rbx
	jmp	.LBB363_1080
.LBB363_1068:
.Ltmp15756:
.LBB363_1069:
	movq	%rax, %rbx
	jmp	.LBB363_1113
.LBB363_1070:
.Ltmp15720:
.LBB363_1071:
	movq	%rax, %rbx
.Ltmp15724:
	leaq	1632(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.10720091597982897309)
.Ltmp15725:
	jmp	.LBB363_1162
.LBB363_1072:
.Ltmp15822:
	movq	1008(%rsp), %rdi
	movq	%rax, %rbx
.Ltmp15823:
	movq	%r14, %rsi
	callq	core::ptr::drop_glue::<rayon::iter::collect::consumer::CollectResult<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>
.Ltmp15824:
	jmp	.LBB363_1075
.LBB363_1073:
.Ltmp15825:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_1074:
.Ltmp15830:
	movq	%rax, %rbx
.LBB363_1075:
.Ltmp15831:
	leaq	976(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>
.Ltmp15832:
	jmp	.LBB363_1126
.LBB363_1076:
.Ltmp15833:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_1077:
.Ltmp16066:
.LBB363_1078:
	movq	%rax, %rbx
	cmpq	$6, %r13
	jb	.LBB363_1080
	movq	1600(%rsp), %rdi
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB363_1080:
	leaq	720(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB363_1126
.LBB363_1081:
.Ltmp15994:
.LBB363_1082:
	movq	%rax, %r12
.LBB363_1083:
	cmpq	$0, 936(%rsp)
	je	.LBB363_1085
	movq	936(%rsp), %rsi
	movq	1096(%rsp), %rdi
	movl	$8, %edx
	shlq	$5, %rsi
	callq	__rustc::__rust_dealloc
.LBB363_1085:
	movb	$1, %al
	movl	%eax, 40(%rsp)
.Ltmp15995:
	leaq	344(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp15996:
.LBB363_1086:
	cmpq	$0, 944(%rsp)
	je	.LBB363_1088
	movq	944(%rsp), %rax
	movq	128(%rsp), %rdi
	movl	$8, %edx
	shlq	$3, %rax
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB363_1088:
	movq	472(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_1091
	lock		decq	(%rax)
	jne	.LBB363_1091
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	472(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB363_1091:
	movq	504(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_1094
	lock		decq	(%rax)
	jne	.LBB363_1094
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	504(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB363_1094:
.Ltmp16000:
	leaq	1008(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp16001:
	xorl	%r14d, %r14d
.LBB363_1096:
	movq	144(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB363_1098
	#MEMBARRIER
.Ltmp16005:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1208(%rsp), %rdi
	callq	*%rax
.Ltmp16006:
.LBB363_1098:
.Ltmp16027:
	leaq	976(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16028:
.LBB363_1099:
	cmpb	$0, 40(%rsp)
	je	.LBB363_1101
.LBB363_1100:
	leaq	184(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB363_1101:
	testb	%r14b, %r14b
	je	.LBB363_1103
.LBB363_1102:
.Ltmp16029:
	leaq	1560(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp16030:
.LBB363_1103:
.Ltmp16044:
	leaq	1968(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp16045:
	movq	%r12, %rbx
	jmp	.LBB363_1126
.LBB363_1104:
.Ltmp16031:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_1105:
.Ltmp15742:
	movq	%rax, %rbx
	movq	%rbp, 384(%rsp,%r12)
	jmp	.LBB363_1110
.LBB363_1106:
.Ltmp15714:
	movq	%rax, %rbx
.Ltmp15715:
	leaq	384(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.10720091597982897309)
.Ltmp15716:
	jmp	.LBB363_1162
.LBB363_1107:
.Ltmp15717:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_1108:
.Ltmp15745:
.LBB363_1109:
	movq	%rax, %rbx
.LBB363_1110:
	movq	384(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB363_1113
	movq	392(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
.LBB363_1112:
	callq	__rustc::__rust_dealloc
.LBB363_1113:
	leaq	1696(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>), purrdf_hash::fixed::FixedState>>
	jmp	.LBB363_1137
.LBB363_1114:
.Ltmp15805:
.LBB363_1115:
	movq	%rax, %rbx
	cmpq	$6, %r13
	jb	.LBB363_1118
	movq	728(%rsp), %rdi
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
.LBB363_1117:
	callq	__rustc::__rust_dealloc
.LBB363_1118:
	leaq	1264(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB363_1119:
	movb	$1, %bpl
	movb	$1, %r15b
	jmp	.LBB363_1130
.LBB363_1120:
.Ltmp16116:
	movq	%rax, %rbx
.LBB363_1121:
	movb	$1, %bpl
	movb	$1, %r15b
	jmp	.LBB363_1148
.LBB363_1122:
.Ltmp15869:
.LBB363_1123:
	movq	%rax, %rbx
.LBB363_1124:
	movq	1008(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB363_1126
	movq	1016(%rsp), %rdi
	shlq	$4, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB363_1126:
	movq	624(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_1129
	lock		decq	(%rax)
	jne	.LBB363_1129
	#MEMBARRIER
.Ltmp16088:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	624(%rsp), %rdi
	callq	*%rax
.Ltmp16089:
.LBB363_1129:
	movb	$1, %bpl
.Ltmp16093:
	leaq	2016(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp16094:
	movb	$1, %r15b
.LBB363_1130:
.Ltmp16098:
	leaq	1128(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>>
.Ltmp16099:
.LBB363_1131:
	leaq	1072(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>>
	jmp	.LBB363_1138
.LBB363_1132:
.Ltmp15767:
	movq	%rax, %rbx
	cmpq	$6, %r12
	jb	.LBB363_1134
	movq	48(%rsp), %rdi
	leaq	-8(,%r12,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB363_1134:
	testq	%r14, %r14
	je	.LBB363_1136
	movq	96(%rsp), %rdi
	shlq	$3, %r14
	movl	$8, %edx
	movq	%r14, %rsi
	callq	__rustc::__rust_dealloc
.LBB363_1136:
	leaq	1264(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>
.LBB363_1137:
	movb	$1, %r15b
	movb	$1, %bpl
.LBB363_1138:
	cmpq	$0, 960(%rsp)
	je	.LBB363_1148
	movq	952(%rsp), %rdi
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
	jge	.LBB363_1141
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_1141:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_1147
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_1141
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
.LBB363_1144:
	cmpq	%rax, %rcx
	jge	.LBB363_1146
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB363_1144
.LBB363_1146:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_1147:
	movq	free@GOTPCREL(%rip), %rax
	movq	176(%rsp), %rdi
	callq	*%rax
.LBB363_1148:
	movq	160(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB363_1150
	#MEMBARRIER
.Ltmp16117:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	callq	*%rax
.Ltmp16118:
.LBB363_1150:
.Ltmp16119:
	leaq	1152(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp16120:
	testb	%bpl, %bpl
	je	.LBB363_1159
.LBB363_1152:
	movq	1808(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB363_1153
	movq	1816(%rsp), %rdi
	leaq	-3(%rax,%rax,2), %rsi
	movl	$1, %edx
	callq	__rustc::__rust_dealloc
	movq	1736(%rsp), %rax
	testq	%rax, %rax
	jg	.LBB363_1156
.LBB363_1154:
	movq	1832(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB363_1157
	jmp	.LBB363_1159
.LBB363_1153:
	movq	1736(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB363_1154
.LBB363_1156:
	movq	1744(%rsp), %rdi
	leaq	(%rax,%rax,2), %rsi
	movl	$1, %edx
	callq	__rustc::__rust_dealloc
	movq	1832(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_1159
.LBB363_1157:
	lock		decq	(%rax)
	jne	.LBB363_1159
	leaq	1832(%rsp), %rdi
	#MEMBARRIER
.Ltmp16121:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp16122:
.LBB363_1159:
	testb	%r15b, %r15b
	je	.LBB363_1162
	movq	320(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB363_1162
	#MEMBARRIER
.Ltmp16126:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1552(%rsp), %rdi
	callq	*%rax
.Ltmp16127:
.LBB363_1162:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB363_1163:
.Ltmp16128:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end363:
