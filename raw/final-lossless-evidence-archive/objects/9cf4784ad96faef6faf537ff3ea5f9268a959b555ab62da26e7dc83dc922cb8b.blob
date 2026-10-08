<purrdf_native::py_store::quad_store::PyQuadStore>::query:
.Lfunc_begin1692:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception1692
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
	subq	$2664, %rsp
	.cfi_def_cfa_offset 2720
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	2784(%rsp), %rax
	movq	%rsi, %r12
	movq	%r9, %r14
	vmovups	(%r14), %xmm1
	movq	%r8, %r15
	movq	2728(%rsp), %r8
	movq	2752(%rsp), %r11
	movq	2744(%rsp), %rbp
	movq	2736(%rsp), %rbx
	movq	%rdi, 8(%rsp)
	movq	2760(%rsp), %rdi
	movq	2776(%rsp), %r9
	movq	2768(%rsp), %r10
	movq	%rdx, 128(%rsp)
	movq	%rcx, (%rsp)
	movq	16(%rax), %rsi
	vmovups	(%rax), %xmm0
	movq	16(%r14), %rax
	vmovups	(%r8), %ymm2
	movq	%rsi, 304(%rsp)
	movq	2720(%rsp), %rsi
	vmovaps	%xmm0, 288(%rsp)
	movq	%r15, 456(%rsp)
	movq	%rbx, 464(%rsp)
	movq	%rbp, 472(%rsp)
	movq	%r11, 480(%rsp)
	vmovups	%xmm1, 312(%rsp)
	vmovups	16(%r8), %ymm1
	movq	%rax, 328(%rsp)
	vmovups	(%rsi), %xmm3
	movq	16(%rsi), %rax
	xorl	%esi, %esi
	movq	%rax, 352(%rsp)
	movq	16(%rdi), %rax
	vmovaps	%xmm3, 336(%rsp)
	vmovups	%ymm2, 360(%rsp)
	vmovups	%ymm1, 376(%rsp)
	vmovups	(%rdi), %xmm1
	movq	%r10, 488(%rsp)
	movq	%r9, 496(%rsp)
	movq	%r12, 432(%rsp)
	movq	%rax, 424(%rsp)
	vmovups	%xmm1, 408(%rsp)
	movq	%rdx, 440(%rsp)
	movq	%rcx, 448(%rsp)
	movq	288(%rsp), %rax
	movq	304(%rsp), %rdx
	movq	%rax, 32(%rsp)
	cmpq	$-1, %rax
	movq	296(%rsp), %rax
	movq	%rax, 80(%rsp)
	cmovneq	%rax, %rsi
	movb	$1, %al
	movl	%eax, 16(%rsp)
.Ltmp35508:
	.cfi_escape 0x2e, 0x00
	leaq	1568(%rsp), %rdi
	movb	$1, %r13b
	vzeroupper
	callq	purrdf_native::py_store::env::division_policy
.Ltmp35509:
	cmpl	$1, 1568(%rsp)
	jne	.LBB2816_3
	vmovups	1592(%rsp), %ymm0
	vmovups	1580(%rsp), %ymm1
	movq	8(%rsp), %rcx
	movl	1576(%rsp), %eax
	vmovups	%ymm0, 24(%rcx)
	vmovups	%ymm1, 12(%rcx)
	movl	%eax, 8(%rcx)
	movq	$1, (%rcx)
	movq	32(%rsp), %rsi
	movb	$1, %bpl
	testq	%rsi, %rsi
	jg	.LBB2816_6
	jmp	.LBB2816_7
.LBB2816_3:
	movq	1572(%rsp), %rax
	movq	%rax, 120(%rsp)
.Ltmp35510:
	.cfi_escape 0x2e, 0x00
	leaq	1568(%rsp), %rdi
	movq	%r15, %rsi
	movb	$1, %r13b
	callq	purrdf_native::py_store::quad_store::collect_substitutions
.Ltmp35511:
	cmpl	$1, 1568(%rsp)
	leaq	1576(%rsp), %r15
	jne	.LBB2816_28
	vmovups	16(%r15), %ymm1
	vmovups	(%r15), %ymm0
	movq	8(%rsp), %rax
	vmovups	%ymm1, 880(%rsp)
	vmovups	%ymm0, 864(%rsp)
	vmovups	880(%rsp), %ymm1
	vmovups	864(%rsp), %ymm0
	vmovups	%ymm1, 24(%rax)
	vmovups	%ymm0, 8(%rax)
	movq	$1, (%rax)
	movq	32(%rsp), %rsi
	movb	$1, %bpl
	testq	%rsi, %rsi
	jle	.LBB2816_7
.LBB2816_6:
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	80(%rsp), %rdi
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LBB2816_7:
	movq	312(%rsp), %r14
	cmpq	$-1, %r14
	je	.LBB2816_15
	movq	320(%rsp), %rbx
	movq	328(%rsp), %r15
	testq	%r15, %r15
	je	.LBB2816_13
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r13
	leaq	8(%rbx), %r12
	jmp	.LBB2816_11
	.p2align	4
.LBB2816_10:
	addq	$24, %r12
	decq	%r15
	je	.LBB2816_13
.LBB2816_11:
	movq	-8(%r12), %rsi
	testq	%rsi, %rsi
	je	.LBB2816_10
	movq	(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r13
	jmp	.LBB2816_10
.LBB2816_13:
	testq	%r14, %r14
	je	.LBB2816_15
	shlq	$3, %r14
	leaq	(%r14,%r14,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB2816_15:
	movq	336(%rsp), %r14
	cmpq	$-1, %r14
	je	.LBB2816_23
	movq	344(%rsp), %rbx
	movq	352(%rsp), %r15
	testq	%r15, %r15
	je	.LBB2816_21
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r13
	leaq	8(%rbx), %r12
	jmp	.LBB2816_19
	.p2align	4
.LBB2816_18:
	addq	$24, %r12
	decq	%r15
	je	.LBB2816_21
.LBB2816_19:
	movq	-8(%r12), %rsi
	testq	%rsi, %rsi
	je	.LBB2816_18
	movq	(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r13
	jmp	.LBB2816_18
.LBB2816_21:
	testq	%r14, %r14
	je	.LBB2816_23
	shlq	$3, %r14
	leaq	(%r14,%r14,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB2816_23:
	movq	360(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB2816_77
	testq	%rsi, %rsi
	je	.LBB2816_26
	movq	368(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LBB2816_26:
	movq	384(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB2816_77
	movq	392(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
	jmp	.LBB2816_77
.LBB2816_28:
	vmovups	(%r15), %xmm0
	movq	16(%r15), %rax
	movq	%rax, 256(%rsp)
	vmovaps	%xmm0, 240(%rsp)
.Ltmp35512:
	.cfi_escape 0x2e, 0x00
	movq	2752(%rsp), %rcx
	leaq	1568(%rsp), %rdi
	movq	%rbx, %rsi
	movq	%rbp, %rdx
	callq	purrdf_native::py_store::query::collect_relations
.Ltmp35513:
	cmpl	$1, 1568(%rsp)
	jne	.LBB2816_31
	vmovups	16(%r15), %ymm1
	vmovups	(%r15), %ymm0
	movq	8(%rsp), %rax
	movb	$1, %r13b
	vmovups	%ymm1, 880(%rsp)
	vmovups	%ymm0, 864(%rsp)
	vmovups	880(%rsp), %ymm1
	vmovups	864(%rsp), %ymm0
	vmovups	%ymm1, 24(%rax)
	vmovups	%ymm0, 8(%rax)
	movq	$1, (%rax)
	jmp	.LBB2816_73
.LBB2816_31:
	movq	312(%rsp), %r8
	movq	320(%rsp), %rcx
	vmovups	(%r15), %xmm0
	movq	16(%r15), %rax
	movq	328(%rsp), %r15
	movq	352(%rsp), %r13
	movq	360(%rsp), %rbp
	movq	384(%rsp), %rbx
	movq	%r8, 64(%rsp)
	movq	%rcx, 56(%rsp)
	movq	336(%rsp), %r8
	movq	344(%rsp), %rcx
	movq	%rax, 112(%rsp)
	vmovaps	%xmm0, 96(%rsp)
	movq	%r8, 72(%rsp)
	movq	%rcx, 48(%rsp)
	movq	368(%rsp), %r8
	movq	392(%rsp), %rcx
	movq	%r8, 264(%rsp)
	movq	%rcx, 88(%rsp)
.Ltmp35515:
	.cfi_escape 0x2e, 0x00
	movq	2768(%rsp), %rsi
	movq	2776(%rsp), %rdx
	leaq	2456(%rsp), %rdi
	callq	purrdf_native::xpath_regex::selection
.Ltmp35516:
	movzbl	2456(%rsp), %eax
	cmpb	$-1, %al
	je	.LBB2816_45
	vmovups	2512(%rsp), %xmm0
	vmovups	2480(%rsp), %ymm1
	movq	16(%r14), %rcx
	movq	2720(%rsp), %rdx
	vmovups	2457(%rsp), %ymm5
	vmovaps	96(%rsp), %xmm2
	movq	%rcx, 2544(%rsp)
	movq	16(%rdx), %rcx
	vmovups	(%rdx), %xmm4
	movq	2760(%rsp), %rdx
	vmovaps	%xmm0, 816(%rsp)
	vmovups	%ymm1, 784(%rsp)
	vmovups	(%r14), %xmm1
	vmovups	%ymm5, 761(%rsp)
	movq	%r12, 840(%rsp)
	vmovaps	%xmm2, 592(%rsp)
	vmovaps	%xmm1, 2528(%rsp)
	movq	%rcx, 2568(%rsp)
	movq	2728(%rsp), %rcx
	vmovups	%xmm4, 2552(%rsp)
	vmovups	(%rcx), %ymm3
	vmovups	16(%rcx), %ymm1
	movq	112(%rsp), %rcx
	movq	%rcx, 608(%rsp)
	movq	16(%rdx), %rcx
	vmovups	%ymm3, 2576(%rsp)
	vmovups	%ymm1, 2592(%rsp)
	vmovups	(%rdx), %xmm1
	movq	(%rsp), %rdx
	vmovups	2528(%rsp), %zmm0
	movq	%rcx, 656(%rsp)
	movq	128(%rsp), %rcx
	vmovaps	%xmm1, 640(%rsp)
	vmovups	2560(%rsp), %zmm1
	vmovups	%zmm1, 696(%rsp)
	vmovups	%zmm0, 664(%rsp)
	vmovaps	240(%rsp), %xmm0
	movb	%al, 760(%rsp)
	movq	%rcx, 848(%rsp)
	movq	120(%rsp), %rax
	movq	256(%rsp), %rcx
	movq	%rdx, 856(%rsp)
	movq	%rcx, 632(%rsp)
	vmovups	%xmm0, 616(%rsp)
	movq	%rax, 832(%rsp)
.Ltmp35520:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::SuspendAttach>::new@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp35521:
	movq	%rax, 272(%rsp)
	movq	%rdx, 280(%rsp)
.Ltmp35525:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::ir::mutable::MutableDataset>::freeze@GOTPCREL(%rip), %rax
	leaq	864(%rsp), %rdi
	movq	%r12, %rsi
	callq	*%rax
.Ltmp35526:
	leaq	1568(%rsp), %rax
	cmpq	$-1, 864(%rsp)
	je	.LBB2816_113
	vmovups	896(%rsp), %zmm1
	vmovups	864(%rsp), %zmm0
	movq	%rax, 1312(%rsp)
	movq	<purrdf_core::diagnostic::RdfDiagnostic as core::fmt::Display>::fmt@GOTPCREL(%rip), %rax
	movq	%rax, 1320(%rsp)
	vmovups	%zmm1, 1600(%rsp)
	vmovups	%zmm0, 1568(%rsp)
.Ltmp35527:
	.cfi_escape 0x2e, 0x00
	movq	alloc::fmt::format::format_inner@GOTPCREL(%rip), %rax
	leaq	.Lalloc_0e5f90e3dc675d538220deef7d17e145(%rip), %rsi
	leaq	2096(%rsp), %rdi
	leaq	1312(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp35528:
	movq	2096(%rsp), %rbx
	movq	2104(%rsp), %r14
	movq	2112(%rsp), %r15
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_no_alloc_shim_is_unstable_v2@GOTPCREL(%rip), %rax
	callq	*%rax
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_alloc@GOTPCREL(%rip), %rax
	movl	$24, %edi
	movl	$8, %esi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB2816_184
	movq	%rax, %r12
	movq	%rbx, (%rax)
	movq	%r14, 8(%rax)
	movq	%r15, 16(%rax)
.Ltmp35530:
	.cfi_escape 0x2e, 0x00
	leaq	1568(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.Ltmp35531:
	movq	600(%rsp), %rbx
	movq	608(%rsp), %rax
	vxorps	%xmm0, %xmm0, %xmm0
	vmovaps	%xmm0, 144(%rsp)
	testq	%rax, %rax
	je	.LBB2816_43
	movl	$1, %r13d
	movq	%rbx, %r14
	subq	%rax, %r13
	.p2align	4
.LBB2816_41:
.Ltmp35533:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.Ltmp35534:
	incq	%r13
	addq	$160, %r14
	cmpq	$1, %r13
	jne	.LBB2816_41
.LBB2816_43:
	movq	592(%rsp), %rax
	movb	$1, %r14b
	movl	$1, %r15d
	leaq	.Lvtable.1R(%rip), %r13
	movl	$3, %ebp
	testq	%rax, %rax
	je	.LBB2816_118
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	callq	*%rax
.LBB2816_118:
.LBB2816_119:
	movq	640(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB2816_122
	testq	%rsi, %rsi
	je	.LBB2816_122
	movq	648(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBB2816_122:
	movq	$-1, %rbx
	testb	%r14b, %r14b
	je	.LBB2816_124
	.cfi_escape 0x2e, 0x00
	leaq	664(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
.LBB2816_124:
	movl	(%rsp), %r14d
	leaq	616(%rsp), %rdi
.Ltmp35641:
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp35642:
.Ltmp35647:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::SuspendAttach as core::ops::drop::Drop>::drop@GOTPCREL(%rip), %rax
	leaq	272(%rsp), %rdi
	callq	*%rax
.Ltmp35648:
	vmovaps	144(%rsp), %xmm0
	vmovaps	%xmm0, 1216(%rsp)
	cmpq	$-1, %rbx
	je	.LBB2816_130
	vmovaps	1216(%rsp), %xmm0
	movq	%rbx, 1568(%rsp)
	vmovups	%xmm0, 1576(%rsp)
	movq	%r15, 1592(%rsp)
	movq	%r12, 1600(%rsp)
	movq	%r13, 1608(%rsp)
	movl	%ebp, 1616(%rsp)
	movl	%r14d, 1620(%rsp)
.Ltmp35649:
	.cfi_escape 0x2e, 0x00
	movq	8(%rsp), %rdi
	leaq	1568(%rsp), %rsi
	callq	purrdf_native::py_store::query::materialize_results
.Ltmp35650:
	movq	32(%rsp), %rsi
	testq	%rsi, %rsi
	jle	.LBB2816_83
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	80(%rsp), %rdi
	jmp	.LBB2816_81
.LBB2816_45:
	vmovups	2464(%rsp), %ymm0
	vmovups	2480(%rsp), %ymm1
	movq	8(%rsp), %rax
	movq	64(%rsp), %r12
	vmovups	%ymm1, 24(%rax)
	vmovups	%ymm0, 8(%rax)
	movq	$1, (%rax)
	cmpq	$-1, %rbp
	je	.LBB2816_50
	testq	%rbp, %rbp
	je	.LBB2816_48
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	264(%rsp), %rdi
	movl	$1, %edx
	movq	%rbp, %rsi
	vzeroupper
	callq	*%rax
.LBB2816_48:
	testq	%rbx, %rbx
	je	.LBB2816_50
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	88(%rsp), %rdi
	movl	$1, %edx
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
.LBB2816_50:
	movq	72(%rsp), %rbp
	cmpq	$-1, %rbp
	je	.LBB2816_58
	testq	%r13, %r13
	je	.LBB2816_56
	movq	48(%rsp), %rax
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r14
	leaq	8(%rax), %rbx
	jmp	.LBB2816_54
	.p2align	4
.LBB2816_53:
	addq	$24, %rbx
	decq	%r13
	je	.LBB2816_56
.LBB2816_54:
	movq	-8(%rbx), %rsi
	testq	%rsi, %rsi
	je	.LBB2816_53
	movq	(%rbx), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r14
	jmp	.LBB2816_53
.LBB2816_56:
	testq	%rbp, %rbp
	je	.LBB2816_58
	shlq	$3, %rbp
	leaq	(%rbp,%rbp,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	48(%rsp), %rdi
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBB2816_58:
	cmpq	$-1, %r12
	je	.LBB2816_66
	testq	%r15, %r15
	je	.LBB2816_64
	movq	56(%rsp), %rax
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r14
	leaq	8(%rax), %rbx
	jmp	.LBB2816_62
	.p2align	4
.LBB2816_61:
	addq	$24, %rbx
	decq	%r15
	je	.LBB2816_64
.LBB2816_62:
	movq	-8(%rbx), %rsi
	testq	%rsi, %rsi
	je	.LBB2816_61
	movq	(%rbx), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r14
	jmp	.LBB2816_61
.LBB2816_64:
	testq	%r12, %r12
	je	.LBB2816_66
	shlq	$3, %r12
	leaq	(%r12,%r12,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	56(%rsp), %rdi
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBB2816_66:
	movq	104(%rsp), %rbx
	movq	112(%rsp), %rax
	testq	%rax, %rax
	je	.LBB2816_70
	movl	$1, %r12d
	movq	%rbx, %r14
	subq	%rax, %r12
	.p2align	4
.LBB2816_68:
.Ltmp35652:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.Ltmp35653:
	incq	%r12
	addq	$160, %r14
	cmpq	$1, %r12
	jne	.LBB2816_68
.LBB2816_70:
	movq	96(%rsp), %rax
	testq	%rax, %rax
	je	.LBB2816_72
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB2816_72:
	xorl	%r13d, %r13d
.LBB2816_73:
	movb	$1, %bpl
.Ltmp35661:
	.cfi_escape 0x2e, 0x00
	leaq	240(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp35662:
	movq	32(%rsp), %rsi
	testq	%rsi, %rsi
	jle	.LBB2816_76
.LBB2816_75:
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	80(%rsp), %rdi
	movl	$1, %edx
	callq	*%rax
.LBB2816_76:
	testb	%r13b, %r13b
	jne	.LBB2816_7
.LBB2816_77:
	testb	%bpl, %bpl
	je	.LBB2816_83
	movq	408(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB2816_83
	testq	%rsi, %rsi
	je	.LBB2816_83
	movq	416(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
.LBB2816_81:
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LBB2816_83:
	movq	8(%rsp), %rax
	cmpl	$1, (%rax)
	jne	.LBB2816_112
	leaq	8(%rax), %rbx
.Ltmp35664:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::AttachGuard>::attach@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp35665:
	movl	%eax, 592(%rsp)
	movq	PyExc_ValueError@GOTPCREL(%rip), %rax
	vmovups	16(%rbx), %ymm1
	vmovups	(%rbx), %ymm0
	movq	(%rax), %r14
	vmovups	%ymm1, 2112(%rsp)
	vmovups	%ymm0, 2096(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp35669:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr>::get_type@GOTPCREL(%rip), %rax
	leaq	2096(%rsp), %rdi
	callq	*%rax
.Ltmp35670:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	movq	PyErr_GivenExceptionMatches@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	movq	%r14, %rsi
	callq	*%rax
	movl	%eax, %ebp
	.cfi_escape 0x2e, 0x00
	movq	_Py_DecRef@GOTPCREL(%rip), %r13
	movq	%r15, %rdi
	callq	*%r13
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	*%r13
	testl	%ebp, %ebp
	je	.LBB2816_104
.Ltmp35672:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr>::value@GOTPCREL(%rip), %rax
	leaq	2096(%rsp), %rdi
	callq	*%rax
.Ltmp35673:
.Ltmp35675:
	movq	%rax, %r14
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::types::string::PyString>::new@GOTPCREL(%rip), %rbp
	leaq	.Lalloc_016e71ba2f68cc7172262ae988bb360c(%rip), %rdi
	movl	$10, %esi
	callq	*%rbp
.Ltmp35676:
.Ltmp35677:
	movq	%rax, %r12
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::inner@GOTPCREL(%rip), %rax
	leaq	1568(%rsp), %rdi
	movq	%r14, %rsi
	movq	%r12, %rdx
	callq	*%rax
.Ltmp35678:
	leaq	872(%rsp), %r15
	.cfi_escape 0x2e, 0x00
	movq	%r12, %rdi
	callq	*%r13
	cmpb	$1, 1568(%rsp)
	je	.LBB2816_97
	cmpb	$1, 1569(%rsp)
	je	.LBB2816_94
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
	movq	%r15, %r12
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	movq	%rax, %rdi
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp35679:
	.cfi_escape 0x2e, 0x00
	leaq	.Lalloc_016e71ba2f68cc7172262ae988bb360c(%rip), %rdx
	leaq	864(%rsp), %rdi
	movl	$10, %ecx
	movq	%r15, %r8
	movq	%r14, %rsi
	movq	%r12, %r15
	callq	<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::setattr::<&str, pyo3::instance::Py<pyo3::types::any::PyAny>>
.Ltmp35680:
	cmpb	$0, 864(%rsp)
	jne	.LBB2816_98
.LBB2816_94:
.Ltmp35681:
	.cfi_escape 0x2e, 0x00
	leaq	.Lalloc_6f6b49b405cc516c15819f32fccb7974(%rip), %rdi
	movl	$12, %esi
	callq	*%rbp
.Ltmp35682:
.Ltmp35683:
	movq	%rax, %r12
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::inner@GOTPCREL(%rip), %rbp
	leaq	1568(%rsp), %rdi
	movq	%r14, %rsi
	movq	%rax, %rdx
	callq	*%rbp
.Ltmp35684:
	.cfi_escape 0x2e, 0x00
	movq	%r12, %rdi
	callq	*%r13
	cmpb	$0, 1568(%rsp)
	je	.LBB2816_107
.LBB2816_97:
	leaq	1576(%rsp), %rax
	vmovups	16(%rax), %ymm1
	vmovups	(%rax), %ymm0
	vmovups	%ymm1, 16(%r15)
	vmovups	%ymm0, (%r15)
.LBB2816_98:
	vmovups	(%r15), %xmm0
	vmovups	896(%rsp), %xmm1
	movq	888(%rsp), %r15
	movq	912(%rsp), %rbp
	cmpq	$0, 2112(%rsp)
	vmovaps	%xmm0, 1568(%rsp)
	vmovaps	%xmm1, 32(%rsp)
	je	.LBB2816_111
	movq	2120(%rsp), %r12
	movq	2128(%rsp), %r14
	testq	%r12, %r12
	je	.LBB2816_105
	movq	(%r14), %rax
	testq	%rax, %rax
	je	.LBB2816_102
.Ltmp35692:
	.cfi_escape 0x2e, 0x00
	movq	%r12, %rdi
	vzeroupper
	callq	*%rax
.Ltmp35693:
.LBB2816_102:
	movq	8(%r14), %rsi
	testq	%rsi, %rsi
	je	.LBB2816_111
	movq	16(%r14), %rdx
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	vzeroupper
	callq	*%rax
	jmp	.LBB2816_111
.LBB2816_104:
	vmovaps	2096(%rsp), %xmm0
	vmovups	2120(%rsp), %xmm1
	movq	2112(%rsp), %r15
	movq	2136(%rsp), %rbp
	vmovaps	%xmm0, 1568(%rsp)
	vmovaps	%xmm1, 32(%rsp)
	jmp	.LBB2816_111
.LBB2816_105:
	.cfi_escape 0x2e, 0x00
	data16
	leaq	pyo3::internal::state::ATTACH_COUNT::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@TLSGD(%rip), %rdi
	data16
	data16
	rex64
	callq	__tls_get_addr@PLT
	cmpq	$0, (%rax)
	jle	.LBB2816_183
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	vzeroupper
	callq	*%r13
	jmp	.LBB2816_111
.LBB2816_107:
	cmpb	$0, 1569(%rsp)
	jne	.LBB2816_110
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
	movq	%r15, %r12
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	movq	_Py_IncRef@GOTPCREL(%rip), %rbp
	movq	%rax, %rdi
	callq	*%rbp
.Ltmp35686:
	.cfi_escape 0x2e, 0x00
	leaq	.Lalloc_6f6b49b405cc516c15819f32fccb7974(%rip), %rdx
	leaq	864(%rsp), %rdi
	movl	$12, %ecx
	movq	%r15, %r8
	movq	%r14, %rsi
	movq	%r12, %r15
	callq	<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::setattr::<&str, pyo3::instance::Py<pyo3::types::any::PyAny>>
.Ltmp35687:
	cmpb	$0, 864(%rsp)
	jne	.LBB2816_98
.LBB2816_110:
	vmovups	2120(%rsp), %xmm0
	vmovaps	2096(%rsp), %xmm1
	movq	2112(%rsp), %r15
	movq	2136(%rsp), %rbp
	vmovaps	%xmm0, 32(%rsp)
	vmovaps	%xmm1, 1568(%rsp)
.LBB2816_111:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::AttachGuard as core::ops::drop::Drop>::drop@GOTPCREL(%rip), %rax
	leaq	592(%rsp), %rdi
	vzeroupper
	callq	*%rax
	vmovaps	1568(%rsp), %xmm0
	vmovaps	32(%rsp), %xmm1
	movq	8(%rsp), %rax
	vmovups	%xmm0, (%rbx)
	movq	%r15, 24(%rax)
	vmovups	%xmm1, 32(%rax)
	movq	%rbp, 48(%rax)
	movq	$1, (%rax)
.LBB2816_112:
	addq	$2664, %rsp
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
.LBB2816_113:
	.cfi_def_cfa_offset 2720
	movq	872(%rsp), %rdx
	movb	$1, %r14b
	movq	%rdx, 24(%rsp)
	addq	$16, %rdx
.Ltmp35549:
	.cfi_escape 0x2e, 0x00
	leaq	1568(%rsp), %rdi
	leaq	592(%rsp), %rsi
	callq	purrdf_native::py_store::query::build_relations
.Ltmp35550:
	vmovups	1576(%rsp), %xmm0
	movl	1568(%rsp), %eax
	movq	1592(%rsp), %r15
	movq	1600(%rsp), %r12
	movq	1608(%rsp), %r13
	movl	1616(%rsp), %ebp
	movl	1620(%rsp), %ecx
	vmovaps	%xmm0, 864(%rsp)
	cmpl	$1, %eax
	jne	.LBB2816_131
	vmovaps	864(%rsp), %xmm0
	movl	%ecx, (%rsp)
	movb	$1, %r14b
	vmovaps	%xmm0, 144(%rsp)
.LBB2816_116:
	movq	24(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB2816_119
	#MEMBARRIER
.Ltmp35635:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	24(%rsp), %rdi
	callq	*%rax
.Ltmp35636:
	jmp	.LBB2816_119
.LBB2816_130:
	vmovaps	1216(%rsp), %xmm0
	movq	8(%rsp), %rax
	vmovups	%xmm0, 8(%rax)
	movq	%r15, 24(%rax)
	movq	%r12, 32(%rax)
	movq	%r13, 40(%rax)
	movl	%ebp, 48(%rax)
	movl	%r14d, 52(%rax)
	movq	$1, (%rax)
	xorl	%ebp, %ebp
	xorl	%r13d, %r13d
	movq	32(%rsp), %rsi
	testq	%rsi, %rsi
	jg	.LBB2816_75
	jmp	.LBB2816_76
.LBB2816_131:
	movq	1640(%rsp), %rax
	vmovups	1624(%rsp), %xmm0
	vmovaps	864(%rsp), %xmm1
	movq	648(%rsp), %r8
	movq	656(%rsp), %rdx
	xorl	%esi, %esi
	movb	$1, %r14b
	movq	%rax, 576(%rsp)
	movq	640(%rsp), %rax
	movq	%r8, 48(%rsp)
	vmovaps	%xmm0, 560(%rsp)
	vmovaps	%xmm1, 512(%rsp)
	movq	%r15, 528(%rsp)
	movq	%r12, 536(%rsp)
	movq	%r13, 544(%rsp)
	movl	%ebp, 552(%rsp)
	movl	%ecx, 556(%rsp)
	cmpq	$-1, %rax
	movq	%rax, 56(%rsp)
	cmovneq	%r8, %rsi
.Ltmp35551:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_validate::query::statistical_aggregates@GOTPCREL(%rip), %rax
	leaq	1272(%rsp), %rbx
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp35552:
	cmpq	$-1, 664(%rsp)
	je	.LBB2816_134
	movq	680(%rsp), %rdx
	movq	672(%rsp), %rsi
.Ltmp35553:
	.cfi_escape 0x2e, 0x00
	leaq	864(%rsp), %rdi
	callq	<alloc::vec::Vec<alloc::string::String> as core::clone::Clone>::clone
.Ltmp35554:
	jmp	.LBB2816_135
.LBB2816_134:
	movq	$0, 864(%rsp)
	movq	$8, 872(%rsp)
	movq	$0, 880(%rsp)
.LBB2816_135:
	cmpq	$-1, 688(%rsp)
	je	.LBB2816_138
	movq	704(%rsp), %rdx
	movq	696(%rsp), %rsi
.Ltmp35556:
	.cfi_escape 0x2e, 0x00
	leaq	1568(%rsp), %rdi
	callq	<alloc::vec::Vec<alloc::string::String> as core::clone::Clone>::clone
.Ltmp35557:
	movq	1568(%rsp), %rdx
	movq	1576(%rsp), %rax
	movq	1584(%rsp), %rcx
	jmp	.LBB2816_139
.LBB2816_138:
	movl	$8, %eax
	xorl	%ecx, %ecx
	xorl	%edx, %edx
.LBB2816_139:
	vmovups	864(%rsp), %xmm0
	movq	880(%rsp), %rsi
	movq	%rsi, 1504(%rsp)
	vmovaps	%xmm0, 1488(%rsp)
	movq	%rdx, 1512(%rsp)
	movq	%rax, 1520(%rsp)
	movq	%rcx, 1528(%rsp)
	movq	$0, 1536(%rsp)
	movq	$8, 1544(%rsp)
	movq	$0, 1552(%rsp)
.Ltmp35559:
	.cfi_escape 0x2e, 0x00
	leaq	1568(%rsp), %rdi
	leaq	664(%rsp), %rsi
	callq	purrdf_native::py_store::query::build_engine
.Ltmp35560:
	movq	512(%rsp), %r9
	movq	624(%rsp), %rdx
	movq	1272(%rsp), %r8
	movq	24(%rsp), %rax
	movq	632(%rsp), %r14
	movq	%rdx, 64(%rsp)
	testq	%r9, %r9
	leaq	512(%rsp), %rdx
	movq	%rax, 88(%rsp)
	movq	%r9, 72(%rsp)
	movq	%r8, 16(%rsp)
	cmoveq	%r9, %rdx
	testq	%r8, %r8
	cmoveq	%r8, %rbx
.Ltmp35562:
	.cfi_escape 0x2e, 0x00
	leaq	864(%rsp), %rdi
	leaq	1488(%rsp), %rsi
	movq	%rbx, %rcx
	callq	purrdf_native::py_store::env::extension_env
.Ltmp35563:
	vmovups	872(%rsp), %xmm0
	movq	864(%rsp), %rax
	movq	888(%rsp), %r15
	movq	896(%rsp), %r12
	movq	904(%rsp), %r13
	movl	912(%rsp), %ebp
	movl	916(%rsp), %ecx
	vmovaps	%xmm0, 1248(%rsp)
	cmpq	$-1, %rax
	je	.LBB2816_154
	vmovups	984(%rsp), %zmm1
	vmovups	1152(%rsp), %zmm0
	vmovups	920(%rsp), %zmm4
	vmovups	1048(%rsp), %zmm2
	vmovups	1112(%rsp), %zmm3
	vmovups	%zmm1, 2216(%rsp)
	vmovaps	1248(%rsp), %xmm1
	vmovups	%zmm0, 2384(%rsp)
	vmovups	%zmm3, 2344(%rsp)
	vmovups	%zmm2, 2280(%rsp)
	vmovups	%zmm4, 2152(%rsp)
	vmovups	%xmm1, 2104(%rsp)
	movq	%r15, 2120(%rsp)
	movq	%r12, 2128(%rsp)
	movq	%r13, 2136(%rsp)
	movl	%ebp, 2144(%rsp)
	movl	%ecx, 2148(%rsp)
	movq	%rax, 2096(%rsp)
.Ltmp35564:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::engine::AdmittedSubstitutions>::requested@GOTPCREL(%rip), %rax
	movq	64(%rsp), %rsi
	leaq	864(%rsp), %r13
	movl	$8, %ecx
	movq	%r14, %rdx
	xorl	%r8d, %r8d
	movq	%r13, %rdi
	vzeroupper
	callq	*%rax
.Ltmp35565:
	leaq	880(%rsp), %rax
.Ltmp35566:
	.cfi_escape 0x2e, 0x10
	movq	128(%rsp), %rdx
	movq	<purrdf_sparql_eval::engine::NativeSparqlEngine>::prepare_request@GOTPCREL(%rip), %r10
	movq	(%rsp), %rcx
	leaq	1312(%rsp), %rbx
	leaq	2096(%rsp), %r15
	leaq	1568(%rsp), %rsi
	xorl	%r8d, %r8d
	movq	%rbx, %rdi
	pushq	%rax
	.cfi_adjust_cfa_offset 8
	pushq	%r15
	.cfi_adjust_cfa_offset 8
	callq	*%r10
	addq	$16, %rsp
	.cfi_adjust_cfa_offset -16
.Ltmp35567:
	movq	1312(%rsp), %rax
	movq	1320(%rsp), %r12
	cmpq	$-1, %rax
	je	.LBB2816_160
	vmovups	1344(%rsp), %zmm1
	vmovups	1328(%rsp), %zmm0
	movq	880(%rsp), %rsi
	vmovups	%zmm1, 176(%rsp)
	vmovups	%zmm0, 160(%rsp)
	movq	%rax, 144(%rsp)
	movq	%r12, 152(%rsp)
	cmpq	$10, %rsi
	jb	.LBB2816_147
	movq	888(%rsp), %rdi
	shlq	$4, %rsi
	addq	$-16, %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBB2816_147:
	movq	1016(%rsp), %rsi
	movq	%r13, %r14
	cmpq	$9, %rsi
	jbe	.LBB2816_149
.LBB2816_148:
	movq	1024(%rsp), %rdi
	shlq	$4, %rsi
	addq	$-16, %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBB2816_149:
	vmovups	160(%rsp), %xmm0
	vmovups	208(%rsp), %ymm1
	movq	144(%rsp), %rax
	movq	152(%rsp), %rbx
	movq	176(%rsp), %r15
	movq	184(%rsp), %r12
	movq	192(%rsp), %r13
	movl	200(%rsp), %ebp
	movl	204(%rsp), %ecx
	vmovaps	%xmm0, 1232(%rsp)
	vmovups	%ymm1, 2624(%rsp)
	cmpq	$-1, %rax
	je	.LBB2816_166
	vmovaps	1232(%rsp), %xmm0
	vmovups	2624(%rsp), %ymm1
	movq	%rbx, 872(%rsp)
	movq	%r14, 1312(%rsp)
	vmovups	%xmm0, 880(%rsp)
	movq	%r15, 896(%rsp)
	movq	%r12, 904(%rsp)
	movq	%r13, 912(%rsp)
	movl	%ebp, 920(%rsp)
	movl	%ecx, 924(%rsp)
	vmovups	%ymm1, 928(%rsp)
	movq	%rax, 864(%rsp)
	movq	<purrdf_core::diagnostic::RdfDiagnostic as core::fmt::Display>::fmt@GOTPCREL(%rip), %rax
	movq	%rax, 1320(%rsp)
.Ltmp35577:
	.cfi_escape 0x2e, 0x00
	movq	alloc::fmt::format::format_inner@GOTPCREL(%rip), %rax
	leaq	.Lalloc_592fabd2ffa0e5a6b6713c88c8b00c99(%rip), %rsi
	leaq	144(%rsp), %rdi
	leaq	1312(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp35578:
	movq	944(%rsp), %rbx
	movq	872(%rsp), %rdi
	movq	880(%rsp), %rsi
.Ltmp35579:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_validate::xpath_regex::diagnostic_refusal_code@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp35580:
	testq	%rbx, %rbx
	movq	%rdx, %rcx
	setne	%dl
	testq	%rax, %rax
	sete	%sil
	orb	%dl, %sil
	cmpb	$1, %sil
	jne	.LBB2816_167
.Ltmp35584:
	.cfi_escape 0x2e, 0x00
	leaq	1312(%rsp), %rdi
	leaq	144(%rsp), %rsi
	movq	%rbx, %rdx
	callq	purrdf_native::py_store::presentation::presented_value_error
.Ltmp35585:
	jmp	.LBB2816_168
.LBB2816_154:
	vmovaps	1248(%rsp), %xmm0
	movl	%ecx, (%rsp)
	vmovaps	%xmm0, 144(%rsp)
.Ltmp35614:
	.cfi_escape 0x2e, 0x00
	leaq	1568(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
.Ltmp35615:
	cmpq	$0, 16(%rsp)
	je	.LBB2816_157
	xorl	%r14d, %r14d
.Ltmp35619:
	.cfi_escape 0x2e, 0x00
	leaq	1272(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
.Ltmp35620:
.LBB2816_157:
	cmpq	$0, 72(%rsp)
	je	.LBB2816_182
.Ltmp35624:
	.cfi_escape 0x2e, 0x00
	leaq	512(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::property_fn::PropertyFunction>)>>
.Ltmp35625:
	xorl	%r14d, %r14d
.Ltmp35630:
	.cfi_escape 0x2e, 0x00
	leaq	544(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
.Ltmp35631:
	jmp	.LBB2816_116
.LBB2816_160:
	vmovups	.Lanon.5ac75c6ab29af51a474fa1a77778ee04.4+8(%rip), %zmm0
	vmovups	.Lanon.5ac75c6ab29af51a474fa1a77778ee04.4+24(%rip), %zmm2
	vmovups	.Lanon.5ac75c6ab29af51a474fa1a77778ee04.4+112(%rip), %zmm1
	movq	88(%rsp), %rdx
	movq	120(%rsp), %rax
	movq	%r12, 136(%rsp)
	leaq	16(%r12), %rcx
	addq	$16, %rdx
	movq	%rax, 1312(%rsp)
	vmovups	%zmm0, 1320(%rsp)
	vmovups	%zmm2, 1336(%rsp)
	movq	%r15, 1400(%rsp)
	movq	$8, 1408(%rsp)
	movq	$0, 1416(%rsp)
	vmovups	%zmm1, 1424(%rsp)
.Ltmp35568:
	.cfi_escape 0x2e, 0x10
	subq	$8, %rsp
	.cfi_adjust_cfa_offset 8
	movq	72(%rsp), %r8
	leaq	152(%rsp), %rdi
	leaq	1576(%rsp), %rsi
	movq	%r14, %r9
	pushq	%rbx
	.cfi_adjust_cfa_offset 8
	vzeroupper
	callq	<purrdf_sparql_eval::engine::NativeSparqlEngine>::query_prepared_admitted::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::dataset_view::NoopReservation<!>>
	addq	$16, %rsp
	.cfi_adjust_cfa_offset -16
.Ltmp35569:
	lock		decq	(%r12)
	jne	.LBB2816_163
	#MEMBARRIER
.Ltmp35574:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::engine::PreparedQuery>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	136(%rsp), %rdi
	callq	*%rax
.Ltmp35575:
.LBB2816_163:
	movq	880(%rsp), %rsi
	cmpq	$10, %rsi
	jb	.LBB2816_165
	movq	888(%rsp), %rdi
	shlq	$4, %rsi
	addq	$-16, %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	callq	*%rax
.LBB2816_165:
	movq	1016(%rsp), %rsi
	movq	%r13, %r14
	cmpq	$10, %rsi
	jae	.LBB2816_148
	jmp	.LBB2816_149
.LBB2816_166:
	vmovaps	1232(%rsp), %xmm0
	jmp	.LBB2816_170
.LBB2816_167:
.Ltmp35582:
	.cfi_escape 0x2e, 0x00
	leaq	1312(%rsp), %rdi
	leaq	144(%rsp), %rsi
	movq	%rax, %rdx
	callq	purrdf_native::py_store::presentation::refusal_value_error
.Ltmp35583:
.LBB2816_168:
.Ltmp35590:
	.cfi_escape 0x2e, 0x00
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.Ltmp35591:
	vmovups	1312(%rsp), %xmm0
	movq	1328(%rsp), %r15
	movq	1336(%rsp), %r12
	movq	1344(%rsp), %r13
	movl	1352(%rsp), %ebp
	movl	1356(%rsp), %ecx
	movq	$-1, %rbx
.LBB2816_170:
	vmovaps	%xmm0, 144(%rsp)
.Ltmp35595:
	movl	%ecx, (%rsp)
	.cfi_escape 0x2e, 0x00
	leaq	2096(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::extension_env::ExtensionEnv>
.Ltmp35596:
.Ltmp35600:
	.cfi_escape 0x2e, 0x00
	leaq	1568(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
.Ltmp35601:
	cmpq	$0, 16(%rsp)
	je	.LBB2816_175
	xorl	%r14d, %r14d
.Ltmp35602:
	.cfi_escape 0x2e, 0x00
	leaq	1272(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
.Ltmp35603:
.LBB2816_175:
	cmpq	$0, 72(%rsp)
	je	.LBB2816_178
.Ltmp35604:
	.cfi_escape 0x2e, 0x00
	leaq	512(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::property_fn::PropertyFunction>)>>
.Ltmp35605:
	xorl	%r14d, %r14d
.Ltmp35610:
	.cfi_escape 0x2e, 0x00
	leaq	544(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
.Ltmp35611:
.LBB2816_178:
	movq	24(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB2816_180
	xorl	%r14d, %r14d
	#MEMBARRIER
.Ltmp35612:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	24(%rsp), %rdi
	callq	*%rax
.Ltmp35613:
.LBB2816_180:
	movq	56(%rsp), %rsi
	testq	%rsi, %rsi
	jle	.LBB2816_124
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	48(%rsp), %rdi
	movl	$1, %edx
	callq	*%rax
	jmp	.LBB2816_124
.LBB2816_182:
	xorl	%r14d, %r14d
	jmp	.LBB2816_116
.LBB2816_183:
.Ltmp35695:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::instance::Py<_> as core::ops::drop::Drop>::drop::drop_slow@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp35696:
	jmp	.LBB2816_111
.LBB2816_184:
.Ltmp35541:
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$24, %esi
	callq	*%rax
.Ltmp35542:
	ud2
.LBB2816_186:
.Ltmp35697:
	movq	%rax, %r15
	jmp	.LBB2816_239
.LBB2816_187:
.Ltmp35570:
	lock		decq	(%r12)
	movq	%rax, %r15
	jne	.LBB2816_202
	#MEMBARRIER
.Ltmp35571:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::engine::PreparedQuery>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	136(%rsp), %rdi
	callq	*%rax
.Ltmp35572:
	jmp	.LBB2816_202
.LBB2816_189:
.Ltmp35573:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2816_190:
.Ltmp35626:
	movq	%rax, %r15
.Ltmp35627:
	.cfi_escape 0x2e, 0x00
	leaq	544(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
.Ltmp35628:
	jmp	.LBB2816_194
.LBB2816_192:
.Ltmp35629:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2816_193:
.Ltmp35606:
	movq	%rax, %r15
.Ltmp35607:
	.cfi_escape 0x2e, 0x00
	leaq	544(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
.Ltmp35608:
.LBB2816_194:
	xorl	%r14d, %r14d
	jmp	.LBB2816_219
.LBB2816_195:
.Ltmp35609:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2816_196:
.Ltmp35581:
	movq	144(%rsp), %rsi
	movq	%rax, %r15
	testq	%rsi, %rsi
	je	.LBB2816_204
	movq	152(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
	jmp	.LBB2816_204
.LBB2816_198:
.Ltmp35558:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::string::String>>
	jmp	.LBB2816_200
.LBB2816_199:
.Ltmp35555:
	movq	%rax, %r15
.LBB2816_200:
	movb	$1, %r14b
	jmp	.LBB2816_214
.LBB2816_201:
.Ltmp35576:
	movq	%rax, %r15
.LBB2816_202:
	.cfi_escape 0x2e, 0x00
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::AdmittedSubstitutions>
	jmp	.LBB2816_210
.LBB2816_203:
.Ltmp35586:
	movq	%rax, %r15
.LBB2816_204:
.Ltmp35587:
	.cfi_escape 0x2e, 0x00
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.Ltmp35588:
	jmp	.LBB2816_210
.LBB2816_205:
.Ltmp35589:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2816_206:
.Ltmp35616:
	movq	%rax, %r15
	jmp	.LBB2816_213
.LBB2816_207:
.Ltmp35637:
	movq	%rax, %r15
	jmp	.LBB2816_269
.LBB2816_208:
.Ltmp35561:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	leaq	1488(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_algebra::parser::ParserOptions>
	jmp	.LBB2816_213
.LBB2816_209:
.Ltmp35592:
	movq	%rax, %r15
.LBB2816_210:
.Ltmp35593:
	.cfi_escape 0x2e, 0x00
	leaq	2096(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::extension_env::ExtensionEnv>
.Ltmp35594:
	jmp	.LBB2816_212
.LBB2816_211:
.Ltmp35597:
	movq	%rax, %r15
.LBB2816_212:
.Ltmp35598:
	.cfi_escape 0x2e, 0x00
	leaq	1568(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
.Ltmp35599:
.LBB2816_213:
	xorl	%r14d, %r14d
.LBB2816_214:
	cmpq	$0, 1272(%rsp)
	je	.LBB2816_217
.Ltmp35617:
	.cfi_escape 0x2e, 0x00
	leaq	1272(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
.Ltmp35618:
	jmp	.LBB2816_217
.LBB2816_216:
.Ltmp35621:
	movq	%rax, %r15
.LBB2816_217:
.Ltmp35622:
	.cfi_escape 0x2e, 0x00
	leaq	512(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry>>
.Ltmp35623:
	jmp	.LBB2816_219
.LBB2816_218:
.Ltmp35632:
	movq	%rax, %r15
.LBB2816_219:
	movq	24(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB2816_269
	#MEMBARRIER
.Ltmp35633:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	24(%rsp), %rdi
	callq	*%rax
.Ltmp35634:
	jmp	.LBB2816_269
.LBB2816_221:
.Ltmp35529:
	movq	%rax, %r15
	jmp	.LBB2816_267
.LBB2816_222:
.Ltmp35694:
	movq	8(%r14), %rsi
	movq	%rax, %r15
	testq	%rsi, %rsi
	je	.LBB2816_239
	movq	16(%r14), %rdx
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	callq	*%rax
	jmp	.LBB2816_239
.LBB2816_224:
.Ltmp35643:
	movq	%rax, %r15
	jmp	.LBB2816_275
.LBB2816_225:
.Ltmp35522:
	movq	%rax, %r15
.Ltmp35523:
	.cfi_escape 0x2e, 0x00
	leaq	592(%rsp), %rdi
	callq	core::ptr::drop_glue::<<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}::{closure#0}>
.Ltmp35524:
	jmp	.LBB2816_229
.LBB2816_226:
.Ltmp35517:
	movq	%rax, %r15
	leaq	360(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
	leaq	336(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	leaq	312(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
.Ltmp35518:
	.cfi_escape 0x2e, 0x00
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>
.Ltmp35519:
	jmp	.LBB2816_249
.LBB2816_228:
.Ltmp35651:
	movq	%rax, %r15
.LBB2816_229:
	movl	$0, 16(%rsp)
	xorl	%r13d, %r13d
	jmp	.LBB2816_276
.LBB2816_230:
.Ltmp35532:
	movq	%rax, %r15
	jmp	.LBB2816_268
.LBB2816_231:
.Ltmp35514:
	movb	$1, %r13b
	movq	%rax, %r15
	jmp	.LBB2816_250
.LBB2816_232:
.Ltmp35674:
	jmp	.LBB2816_237
.LBB2816_233:
.Ltmp35685:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	movq	%r12, %rdi
	callq	*%r13
	jmp	.LBB2816_238
.LBB2816_234:
.Ltmp35671:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	movq	_Py_DecRef@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
	jmp	.LBB2816_238
.LBB2816_235:
.Ltmp35666:
	movq	%rax, %r15
.Ltmp35667:
	.cfi_escape 0x2e, 0x00
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<pyo3::err::PyErr>
.Ltmp35668:
	jmp	.LBB2816_284
.LBB2816_236:
.Ltmp35688:
.LBB2816_237:
	movq	%rax, %r15
.LBB2816_238:
.Ltmp35689:
	.cfi_escape 0x2e, 0x00
	leaq	2096(%rsp), %rdi
	callq	core::ptr::drop_glue::<pyo3::err::PyErr>
.Ltmp35690:
.LBB2816_239:
.Ltmp35698:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::AttachGuard as core::ops::drop::Drop>::drop@GOTPCREL(%rip), %rax
	leaq	592(%rsp), %rdi
	callq	*%rax
.Ltmp35699:
	jmp	.LBB2816_284
.LBB2816_240:
.Ltmp35700:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2816_241:
.Ltmp35691:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2816_242:
.Ltmp35654:
	movq	%rax, %r15
	testq	%r12, %r12
	je	.LBB2816_246
	negq	%r12
	addq	$160, %r14
	.p2align	4
.LBB2816_244:
.Ltmp35655:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.Ltmp35656:
	addq	$160, %r14
	decq	%r12
	jne	.LBB2816_244
.LBB2816_246:
	movq	96(%rsp), %rax
	testq	%rax, %rax
	je	.LBB2816_249
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	callq	*%rax
.LBB2816_249:
	xorl	%r13d, %r13d
.LBB2816_250:
.Ltmp35658:
	.cfi_escape 0x2e, 0x00
	leaq	240(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp35659:
	jmp	.LBB2816_276
.LBB2816_251:
.Ltmp35657:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2816_252:
.Ltmp35660:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2816_253:
.Ltmp35535:
	movq	%rax, %r15
	testq	%r13, %r13
	je	.LBB2816_257
	negq	%r13
	addq	$160, %r14
.LBB2816_255:
.Ltmp35536:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.Ltmp35537:
	addq	$160, %r14
	decq	%r13
	jne	.LBB2816_255
.LBB2816_257:
	movq	592(%rsp), %rax
	testq	%rax, %rax
	je	.LBB2816_259
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	callq	*%rax
.LBB2816_259:
	movq	640(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB2816_262
	testq	%rsi, %rsi
	je	.LBB2816_262
	movq	648(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBB2816_262:
	leaq	664(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
.Ltmp35539:
	.cfi_escape 0x2e, 0x00
	leaq	616(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp35540:
	jmp	.LBB2816_275
.LBB2816_263:
.Ltmp35538:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2816_264:
.Ltmp35663:
	movq	%rax, %r15
	jmp	.LBB2816_276
.LBB2816_265:
.Ltmp35543:
	movq	%rax, %r15
	testq	%rbx, %rbx
	je	.LBB2816_267
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	movq	%r14, %rdi
	movq	%rbx, %rsi
	callq	*%rax
.LBB2816_267:
.Ltmp35544:
	.cfi_escape 0x2e, 0x00
	leaq	1568(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.Ltmp35545:
.LBB2816_268:
	movb	$1, %r14b
.Ltmp35547:
	.cfi_escape 0x2e, 0x00
	leaq	592(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>
.Ltmp35548:
.LBB2816_269:
	movq	640(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB2816_272
	testq	%rsi, %rsi
	je	.LBB2816_272
	movq	648(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBB2816_272:
	testb	%r14b, %r14b
	je	.LBB2816_274
	leaq	664(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
.LBB2816_274:
.Ltmp35638:
	.cfi_escape 0x2e, 0x00
	leaq	616(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp35639:
.LBB2816_275:
	movl	$0, 16(%rsp)
.Ltmp35644:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::SuspendAttach as core::ops::drop::Drop>::drop@GOTPCREL(%rip), %rax
	leaq	272(%rsp), %rdi
	callq	*%rax
.Ltmp35645:
	xorl	%r13d, %r13d
.LBB2816_276:
	cmpq	$0, 32(%rsp)
	jle	.LBB2816_278
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	80(%rsp), %rdi
	movq	32(%rsp), %rsi
	movl	$1, %edx
	callq	*%rax
.LBB2816_278:
	testb	%r13b, %r13b
	je	.LBB2816_280
	leaq	312(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	leaq	336(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	leaq	360(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
.LBB2816_280:
	cmpb	$0, 16(%rsp)
	je	.LBB2816_284
	movq	408(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB2816_284
	testq	%rsi, %rsi
	je	.LBB2816_284
	movq	416(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBB2816_284:
	.cfi_escape 0x2e, 0x00
	movq	%r15, %rdi
	callq	_Unwind_Resume@PLT
.LBB2816_285:
.Ltmp35646:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2816_286:
.Ltmp35640:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2816_287:
.Ltmp35546:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end2816:
