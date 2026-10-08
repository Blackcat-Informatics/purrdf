<purrdf_native::py_store::quad_store::PyQuadStore>::query:
.Lfunc_beginFN:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .LexceptionFN
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
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1568(%rsp), %rdi
	movb	$1, %r13b
	vzeroupper
	callq	purrdf_native::py_store::env::division_policy
.LtmpFN:
	cmpl	$1, 1568(%rsp)
	jne	.LBBFN_3
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
	jg	.LBBFN_6
	jmp	.LBBFN_7
.LBBFN_3:
	movq	1572(%rsp), %rax
	movq	%rax, 120(%rsp)
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1568(%rsp), %rdi
	movq	%r15, %rsi
	movb	$1, %r13b
	callq	purrdf_native::py_store::quad_store::collect_substitutions
.LtmpFN:
	cmpl	$1, 1568(%rsp)
	leaq	1576(%rsp), %r15
	jne	.LBBFN_28
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
	jle	.LBBFN_7
.LBBFN_6:
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	80(%rsp), %rdi
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LBBFN_7:
	movq	312(%rsp), %r14
	cmpq	$-1, %r14
	je	.LBBFN_15
	movq	320(%rsp), %rbx
	movq	328(%rsp), %r15
	testq	%r15, %r15
	je	.LBBFN_13
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r13
	leaq	8(%rbx), %r12
	jmp	.LBBFN_11
	.p2align	4
.LBBFN_10:
	addq	$24, %r12
	decq	%r15
	je	.LBBFN_13
.LBBFN_11:
	movq	-8(%r12), %rsi
	testq	%rsi, %rsi
	je	.LBBFN_10
	movq	(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r13
	jmp	.LBBFN_10
.LBBFN_13:
	testq	%r14, %r14
	je	.LBBFN_15
	shlq	$3, %r14
	leaq	(%r14,%r14,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBBFN_15:
	movq	336(%rsp), %r14
	cmpq	$-1, %r14
	je	.LBBFN_23
	movq	344(%rsp), %rbx
	movq	352(%rsp), %r15
	testq	%r15, %r15
	je	.LBBFN_21
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r13
	leaq	8(%rbx), %r12
	jmp	.LBBFN_19
	.p2align	4
.LBBFN_18:
	addq	$24, %r12
	decq	%r15
	je	.LBBFN_21
.LBBFN_19:
	movq	-8(%r12), %rsi
	testq	%rsi, %rsi
	je	.LBBFN_18
	movq	(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r13
	jmp	.LBBFN_18
.LBBFN_21:
	testq	%r14, %r14
	je	.LBBFN_23
	shlq	$3, %r14
	leaq	(%r14,%r14,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBBFN_23:
	movq	360(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBBFN_77
	testq	%rsi, %rsi
	je	.LBBFN_26
	movq	368(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LBBFN_26:
	movq	384(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBBFN_77
	movq	392(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
	jmp	.LBBFN_77
.LBBFN_28:
	vmovups	(%r15), %xmm0
	movq	16(%r15), %rax
	movq	%rax, 256(%rsp)
	vmovaps	%xmm0, 240(%rsp)
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	2752(%rsp), %rcx
	leaq	1568(%rsp), %rdi
	movq	%rbx, %rsi
	movq	%rbp, %rdx
	callq	purrdf_native::py_store::query::collect_relations
.LtmpFN:
	cmpl	$1, 1568(%rsp)
	jne	.LBBFN_31
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
	jmp	.LBBFN_73
.LBBFN_31:
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
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	2768(%rsp), %rsi
	movq	2776(%rsp), %rdx
	leaq	2456(%rsp), %rdi
	callq	purrdf_native::xpath_regex::selection
.LtmpFN:
	movzbl	2456(%rsp), %eax
	cmpb	$-1, %al
	je	.LBBFN_45
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
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::SuspendAttach>::new@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LtmpFN:
	movq	%rax, 272(%rsp)
	movq	%rdx, 280(%rsp)
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::ir::mutable::MutableDataset>::freeze@GOTPCREL(%rip), %rax
	leaq	864(%rsp), %rdi
	movq	%r12, %rsi
	callq	*%rax
.LtmpFN:
	leaq	1568(%rsp), %rax
	cmpq	$-1, 864(%rsp)
	je	.LBBFN_113
	vmovups	896(%rsp), %zmm1
	vmovups	864(%rsp), %zmm0
	movq	%rax, 1312(%rsp)
	movq	<purrdf_core::diagnostic::RdfDiagnostic as core::fmt::Display>::fmt@GOTPCREL(%rip), %rax
	movq	%rax, 1320(%rsp)
	vmovups	%zmm1, 1600(%rsp)
	vmovups	%zmm0, 1568(%rsp)
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	alloc::fmt::format::format_inner@GOTPCREL(%rip), %rax
	leaq	.Lalloc_0e5f90e3dc675d538220deef7d17e145(%rip), %rsi
	leaq	2096(%rsp), %rdi
	leaq	1312(%rsp), %rdx
	vzeroupper
	callq	*%rax
.LtmpFN:
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
	je	.LBBFN_184
	movq	%rax, %r12
	movq	%rbx, (%rax)
	movq	%r14, 8(%rax)
	movq	%r15, 16(%rax)
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1568(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.LtmpFN:
	movq	600(%rsp), %rbx
	movq	608(%rsp), %rax
	vxorps	%xmm0, %xmm0, %xmm0
	vmovaps	%xmm0, 144(%rsp)
	testq	%rax, %rax
	je	.LBBFN_43
	movl	$1, %r13d
	movq	%rbx, %r14
	subq	%rax, %r13
	.p2align	4
.LBBFN_41:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.LtmpFN:
	incq	%r13
	addq	$160, %r14
	cmpq	$1, %r13
	jne	.LBBFN_41
.LBBFN_43:
	movq	592(%rsp), %rax
	movb	$1, %r14b
	movl	$1, %r15d
	leaq	.Lvtable.1R(%rip), %r13
	movl	$3, %ebp
	testq	%rax, %rax
	je	.LBBFN_118
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	callq	*%rax
.LBBFN_118:
.LBBFN_119:
	movq	640(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBBFN_122
	testq	%rsi, %rsi
	je	.LBBFN_122
	movq	648(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBBFN_122:
	movq	$-1, %rbx
	testb	%r14b, %r14b
	je	.LBBFN_124
	.cfi_escape 0x2e, 0x00
	leaq	664(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
.LBBFN_124:
	movl	(%rsp), %r14d
	leaq	616(%rsp), %rdi
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.LtmpFN:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::SuspendAttach as core::ops::drop::Drop>::drop@GOTPCREL(%rip), %rax
	leaq	272(%rsp), %rdi
	callq	*%rax
.LtmpFN:
	vmovaps	144(%rsp), %xmm0
	vmovaps	%xmm0, 1216(%rsp)
	cmpq	$-1, %rbx
	je	.LBBFN_130
	vmovaps	1216(%rsp), %xmm0
	movq	%rbx, 1568(%rsp)
	vmovups	%xmm0, 1576(%rsp)
	movq	%r15, 1592(%rsp)
	movq	%r12, 1600(%rsp)
	movq	%r13, 1608(%rsp)
	movl	%ebp, 1616(%rsp)
	movl	%r14d, 1620(%rsp)
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	8(%rsp), %rdi
	leaq	1568(%rsp), %rsi
	callq	purrdf_native::py_store::query::materialize_results
.LtmpFN:
	movq	32(%rsp), %rsi
	testq	%rsi, %rsi
	jle	.LBBFN_83
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	80(%rsp), %rdi
	jmp	.LBBFN_81
.LBBFN_45:
	vmovups	2464(%rsp), %ymm0
	vmovups	2480(%rsp), %ymm1
	movq	8(%rsp), %rax
	movq	64(%rsp), %r12
	vmovups	%ymm1, 24(%rax)
	vmovups	%ymm0, 8(%rax)
	movq	$1, (%rax)
	cmpq	$-1, %rbp
	je	.LBBFN_50
	testq	%rbp, %rbp
	je	.LBBFN_48
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	264(%rsp), %rdi
	movl	$1, %edx
	movq	%rbp, %rsi
	vzeroupper
	callq	*%rax
.LBBFN_48:
	testq	%rbx, %rbx
	je	.LBBFN_50
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	88(%rsp), %rdi
	movl	$1, %edx
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
.LBBFN_50:
	movq	72(%rsp), %rbp
	cmpq	$-1, %rbp
	je	.LBBFN_58
	testq	%r13, %r13
	je	.LBBFN_56
	movq	48(%rsp), %rax
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r14
	leaq	8(%rax), %rbx
	jmp	.LBBFN_54
	.p2align	4
.LBBFN_53:
	addq	$24, %rbx
	decq	%r13
	je	.LBBFN_56
.LBBFN_54:
	movq	-8(%rbx), %rsi
	testq	%rsi, %rsi
	je	.LBBFN_53
	movq	(%rbx), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r14
	jmp	.LBBFN_53
.LBBFN_56:
	testq	%rbp, %rbp
	je	.LBBFN_58
	shlq	$3, %rbp
	leaq	(%rbp,%rbp,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	48(%rsp), %rdi
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBBFN_58:
	cmpq	$-1, %r12
	je	.LBBFN_66
	testq	%r15, %r15
	je	.LBBFN_64
	movq	56(%rsp), %rax
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r14
	leaq	8(%rax), %rbx
	jmp	.LBBFN_62
	.p2align	4
.LBBFN_61:
	addq	$24, %rbx
	decq	%r15
	je	.LBBFN_64
.LBBFN_62:
	movq	-8(%rbx), %rsi
	testq	%rsi, %rsi
	je	.LBBFN_61
	movq	(%rbx), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r14
	jmp	.LBBFN_61
.LBBFN_64:
	testq	%r12, %r12
	je	.LBBFN_66
	shlq	$3, %r12
	leaq	(%r12,%r12,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	56(%rsp), %rdi
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBBFN_66:
	movq	104(%rsp), %rbx
	movq	112(%rsp), %rax
	testq	%rax, %rax
	je	.LBBFN_70
	movl	$1, %r12d
	movq	%rbx, %r14
	subq	%rax, %r12
	.p2align	4
.LBBFN_68:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.LtmpFN:
	incq	%r12
	addq	$160, %r14
	cmpq	$1, %r12
	jne	.LBBFN_68
.LBBFN_70:
	movq	96(%rsp), %rax
	testq	%rax, %rax
	je	.LBBFN_72
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBBFN_72:
	xorl	%r13d, %r13d
.LBBFN_73:
	movb	$1, %bpl
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	240(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.LtmpFN:
	movq	32(%rsp), %rsi
	testq	%rsi, %rsi
	jle	.LBBFN_76
.LBBFN_75:
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	80(%rsp), %rdi
	movl	$1, %edx
	callq	*%rax
.LBBFN_76:
	testb	%r13b, %r13b
	jne	.LBBFN_7
.LBBFN_77:
	testb	%bpl, %bpl
	je	.LBBFN_83
	movq	408(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBBFN_83
	testq	%rsi, %rsi
	je	.LBBFN_83
	movq	416(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
.LBBFN_81:
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LBBFN_83:
	movq	8(%rsp), %rax
	cmpl	$1, (%rax)
	jne	.LBBFN_112
	leaq	8(%rax), %rbx
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::AttachGuard>::attach@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LtmpFN:
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
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr>::get_type@GOTPCREL(%rip), %rax
	leaq	2096(%rsp), %rdi
	callq	*%rax
.LtmpFN:
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
	je	.LBBFN_104
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr>::value@GOTPCREL(%rip), %rax
	leaq	2096(%rsp), %rdi
	callq	*%rax
.LtmpFN:
.LtmpFN:
	movq	%rax, %r14
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::types::string::PyString>::new@GOTPCREL(%rip), %rbp
	leaq	.Lalloc_016e71ba2f68cc7172262ae988bb360c(%rip), %rdi
	movl	$10, %esi
	callq	*%rbp
.LtmpFN:
.LtmpFN:
	movq	%rax, %r12
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::inner@GOTPCREL(%rip), %rax
	leaq	1568(%rsp), %rdi
	movq	%r14, %rsi
	movq	%r12, %rdx
	callq	*%rax
.LtmpFN:
	leaq	872(%rsp), %r15
	.cfi_escape 0x2e, 0x00
	movq	%r12, %rdi
	callq	*%r13
	cmpb	$1, 1568(%rsp)
	je	.LBBFN_97
	cmpb	$1, 1569(%rsp)
	je	.LBBFN_94
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
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	.Lalloc_016e71ba2f68cc7172262ae988bb360c(%rip), %rdx
	leaq	864(%rsp), %rdi
	movl	$10, %ecx
	movq	%r15, %r8
	movq	%r14, %rsi
	movq	%r12, %r15
	callq	<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::setattr::<&str, pyo3::instance::Py<pyo3::types::any::PyAny>>
.LtmpFN:
	cmpb	$0, 864(%rsp)
	jne	.LBBFN_98
.LBBFN_94:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	.Lalloc_6f6b49b405cc516c15819f32fccb7974(%rip), %rdi
	movl	$12, %esi
	callq	*%rbp
.LtmpFN:
.LtmpFN:
	movq	%rax, %r12
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::inner@GOTPCREL(%rip), %rbp
	leaq	1568(%rsp), %rdi
	movq	%r14, %rsi
	movq	%rax, %rdx
	callq	*%rbp
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	%r12, %rdi
	callq	*%r13
	cmpb	$0, 1568(%rsp)
	je	.LBBFN_107
.LBBFN_97:
	leaq	1576(%rsp), %rax
	vmovups	16(%rax), %ymm1
	vmovups	(%rax), %ymm0
	vmovups	%ymm1, 16(%r15)
	vmovups	%ymm0, (%r15)
.LBBFN_98:
	vmovups	(%r15), %xmm0
	vmovups	896(%rsp), %xmm1
	movq	888(%rsp), %r15
	movq	912(%rsp), %rbp
	cmpq	$0, 2112(%rsp)
	vmovaps	%xmm0, 1568(%rsp)
	vmovaps	%xmm1, 32(%rsp)
	je	.LBBFN_111
	movq	2120(%rsp), %r12
	movq	2128(%rsp), %r14
	testq	%r12, %r12
	je	.LBBFN_105
	movq	(%r14), %rax
	testq	%rax, %rax
	je	.LBBFN_102
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	%r12, %rdi
	vzeroupper
	callq	*%rax
.LtmpFN:
.LBBFN_102:
	movq	8(%r14), %rsi
	testq	%rsi, %rsi
	je	.LBBFN_111
	movq	16(%r14), %rdx
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	vzeroupper
	callq	*%rax
	jmp	.LBBFN_111
.LBBFN_104:
	vmovaps	2096(%rsp), %xmm0
	vmovups	2120(%rsp), %xmm1
	movq	2112(%rsp), %r15
	movq	2136(%rsp), %rbp
	vmovaps	%xmm0, 1568(%rsp)
	vmovaps	%xmm1, 32(%rsp)
	jmp	.LBBFN_111
.LBBFN_105:
	.cfi_escape 0x2e, 0x00
	data16
	leaq	pyo3::internal::state::ATTACH_COUNT::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@TLSGD(%rip), %rdi
	data16
	data16
	rex64
	callq	__tls_get_addr@PLT
	cmpq	$0, (%rax)
	jle	.LBBFN_183
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	vzeroupper
	callq	*%r13
	jmp	.LBBFN_111
.LBBFN_107:
	cmpb	$0, 1569(%rsp)
	jne	.LBBFN_110
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
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	.Lalloc_6f6b49b405cc516c15819f32fccb7974(%rip), %rdx
	leaq	864(%rsp), %rdi
	movl	$12, %ecx
	movq	%r15, %r8
	movq	%r14, %rsi
	movq	%r12, %r15
	callq	<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::setattr::<&str, pyo3::instance::Py<pyo3::types::any::PyAny>>
.LtmpFN:
	cmpb	$0, 864(%rsp)
	jne	.LBBFN_98
.LBBFN_110:
	vmovups	2120(%rsp), %xmm0
	vmovaps	2096(%rsp), %xmm1
	movq	2112(%rsp), %r15
	movq	2136(%rsp), %rbp
	vmovaps	%xmm0, 32(%rsp)
	vmovaps	%xmm1, 1568(%rsp)
.LBBFN_111:
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
.LBBFN_112:
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
.LBBFN_113:
	.cfi_def_cfa_offset 2720
	movq	872(%rsp), %rdx
	movb	$1, %r14b
	movq	%rdx, 24(%rsp)
	addq	$16, %rdx
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1568(%rsp), %rdi
	leaq	592(%rsp), %rsi
	callq	purrdf_native::py_store::query::build_relations
.LtmpFN:
	vmovups	1576(%rsp), %xmm0
	movl	1568(%rsp), %eax
	movq	1592(%rsp), %r15
	movq	1600(%rsp), %r12
	movq	1608(%rsp), %r13
	movl	1616(%rsp), %ebp
	movl	1620(%rsp), %ecx
	vmovaps	%xmm0, 864(%rsp)
	cmpl	$1, %eax
	jne	.LBBFN_131
	vmovaps	864(%rsp), %xmm0
	movl	%ecx, (%rsp)
	movb	$1, %r14b
	vmovaps	%xmm0, 144(%rsp)
.LBBFN_116:
	movq	24(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBBFN_119
	#MEMBARRIER
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	24(%rsp), %rdi
	callq	*%rax
.LtmpFN:
	jmp	.LBBFN_119
.LBBFN_130:
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
	jg	.LBBFN_75
	jmp	.LBBFN_76
.LBBFN_131:
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
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_validate::query::statistical_aggregates@GOTPCREL(%rip), %rax
	leaq	1272(%rsp), %rbx
	movq	%rbx, %rdi
	callq	*%rax
.LtmpFN:
	cmpq	$-1, 664(%rsp)
	je	.LBBFN_134
	movq	680(%rsp), %rdx
	movq	672(%rsp), %rsi
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	864(%rsp), %rdi
	callq	<alloc::vec::Vec<alloc::string::String> as core::clone::Clone>::clone
.LtmpFN:
	jmp	.LBBFN_135
.LBBFN_134:
	movq	$0, 864(%rsp)
	movq	$8, 872(%rsp)
	movq	$0, 880(%rsp)
.LBBFN_135:
	cmpq	$-1, 688(%rsp)
	je	.LBBFN_138
	movq	704(%rsp), %rdx
	movq	696(%rsp), %rsi
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1568(%rsp), %rdi
	callq	<alloc::vec::Vec<alloc::string::String> as core::clone::Clone>::clone
.LtmpFN:
	movq	1568(%rsp), %rdx
	movq	1576(%rsp), %rax
	movq	1584(%rsp), %rcx
	jmp	.LBBFN_139
.LBBFN_138:
	movl	$8, %eax
	xorl	%ecx, %ecx
	xorl	%edx, %edx
.LBBFN_139:
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
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1568(%rsp), %rdi
	leaq	664(%rsp), %rsi
	callq	purrdf_native::py_store::query::build_engine
.LtmpFN:
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
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	864(%rsp), %rdi
	leaq	1488(%rsp), %rsi
	movq	%rbx, %rcx
	callq	purrdf_native::py_store::env::extension_env
.LtmpFN:
	vmovups	872(%rsp), %xmm0
	movq	864(%rsp), %rax
	movq	888(%rsp), %r15
	movq	896(%rsp), %r12
	movq	904(%rsp), %r13
	movl	912(%rsp), %ebp
	movl	916(%rsp), %ecx
	vmovaps	%xmm0, 1248(%rsp)
	cmpq	$-1, %rax
	je	.LBBFN_154
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
.LtmpFN:
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
.LtmpFN:
	leaq	880(%rsp), %rax
.LtmpFN:
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
.LtmpFN:
	movq	1312(%rsp), %rax
	movq	1320(%rsp), %r12
	cmpq	$-1, %rax
	je	.LBBFN_160
	vmovups	1344(%rsp), %zmm1
	vmovups	1328(%rsp), %zmm0
	movq	880(%rsp), %rsi
	vmovups	%zmm1, 176(%rsp)
	vmovups	%zmm0, 160(%rsp)
	movq	%rax, 144(%rsp)
	movq	%r12, 152(%rsp)
	cmpq	$10, %rsi
	jb	.LBBFN_147
	movq	888(%rsp), %rdi
	shlq	$4, %rsi
	addq	$-16, %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBBFN_147:
	movq	1016(%rsp), %rsi
	movq	%r13, %r14
	cmpq	$9, %rsi
	jbe	.LBBFN_149
.LBBFN_148:
	movq	1024(%rsp), %rdi
	shlq	$4, %rsi
	addq	$-16, %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBBFN_149:
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
	je	.LBBFN_166
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
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	alloc::fmt::format::format_inner@GOTPCREL(%rip), %rax
	leaq	.Lalloc_592fabd2ffa0e5a6b6713c88c8b00c99(%rip), %rsi
	leaq	144(%rsp), %rdi
	leaq	1312(%rsp), %rdx
	vzeroupper
	callq	*%rax
.LtmpFN:
	movq	944(%rsp), %rbx
	movq	872(%rsp), %rdi
	movq	880(%rsp), %rsi
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_validate::xpath_regex::diagnostic_refusal_code@GOTPCREL(%rip), %rax
	callq	*%rax
.LtmpFN:
	testq	%rbx, %rbx
	movq	%rdx, %rcx
	setne	%dl
	testq	%rax, %rax
	sete	%sil
	orb	%dl, %sil
	cmpb	$1, %sil
	jne	.LBBFN_167
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1312(%rsp), %rdi
	leaq	144(%rsp), %rsi
	movq	%rbx, %rdx
	callq	purrdf_native::py_store::presentation::presented_value_error
.LtmpFN:
	jmp	.LBBFN_168
.LBBFN_154:
	vmovaps	1248(%rsp), %xmm0
	movl	%ecx, (%rsp)
	vmovaps	%xmm0, 144(%rsp)
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1568(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
.LtmpFN:
	cmpq	$0, 16(%rsp)
	je	.LBBFN_157
	xorl	%r14d, %r14d
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1272(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
.LtmpFN:
.LBBFN_157:
	cmpq	$0, 72(%rsp)
	je	.LBBFN_182
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	512(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::property_fn::PropertyFunction>)>>
.LtmpFN:
	xorl	%r14d, %r14d
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	544(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
.LtmpFN:
	jmp	.LBBFN_116
.LBBFN_160:
	vmovups	.Lanon.HASH.4+8(%rip), %zmm0
	vmovups	.Lanon.HASH.4+24(%rip), %zmm2
	vmovups	.Lanon.HASH.4+112(%rip), %zmm1
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
.LtmpFN:
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
.LtmpFN:
	lock		decq	(%r12)
	jne	.LBBFN_163
	#MEMBARRIER
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::engine::PreparedQuery>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	136(%rsp), %rdi
	callq	*%rax
.LtmpFN:
.LBBFN_163:
	movq	880(%rsp), %rsi
	cmpq	$10, %rsi
	jb	.LBBFN_165
	movq	888(%rsp), %rdi
	shlq	$4, %rsi
	addq	$-16, %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	callq	*%rax
.LBBFN_165:
	movq	1016(%rsp), %rsi
	movq	%r13, %r14
	cmpq	$10, %rsi
	jae	.LBBFN_148
	jmp	.LBBFN_149
.LBBFN_166:
	vmovaps	1232(%rsp), %xmm0
	jmp	.LBBFN_170
.LBBFN_167:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1312(%rsp), %rdi
	leaq	144(%rsp), %rsi
	movq	%rax, %rdx
	callq	purrdf_native::py_store::presentation::refusal_value_error
.LtmpFN:
.LBBFN_168:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.LtmpFN:
	vmovups	1312(%rsp), %xmm0
	movq	1328(%rsp), %r15
	movq	1336(%rsp), %r12
	movq	1344(%rsp), %r13
	movl	1352(%rsp), %ebp
	movl	1356(%rsp), %ecx
	movq	$-1, %rbx
.LBBFN_170:
	vmovaps	%xmm0, 144(%rsp)
.LtmpFN:
	movl	%ecx, (%rsp)
	.cfi_escape 0x2e, 0x00
	leaq	2096(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::extension_env::ExtensionEnv>
.LtmpFN:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1568(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
.LtmpFN:
	cmpq	$0, 16(%rsp)
	je	.LBBFN_175
	xorl	%r14d, %r14d
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1272(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
.LtmpFN:
.LBBFN_175:
	cmpq	$0, 72(%rsp)
	je	.LBBFN_178
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	512(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::property_fn::PropertyFunction>)>>
.LtmpFN:
	xorl	%r14d, %r14d
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	544(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
.LtmpFN:
.LBBFN_178:
	movq	24(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBBFN_180
	xorl	%r14d, %r14d
	#MEMBARRIER
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	24(%rsp), %rdi
	callq	*%rax
.LtmpFN:
.LBBFN_180:
	movq	56(%rsp), %rsi
	testq	%rsi, %rsi
	jle	.LBBFN_124
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	48(%rsp), %rdi
	movl	$1, %edx
	callq	*%rax
	jmp	.LBBFN_124
.LBBFN_182:
	xorl	%r14d, %r14d
	jmp	.LBBFN_116
.LBBFN_183:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::instance::Py<_> as core::ops::drop::Drop>::drop::drop_slow@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.LtmpFN:
	jmp	.LBBFN_111
.LBBFN_184:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$24, %esi
	callq	*%rax
.LtmpFN:
	ud2
.LBBFN_186:
.LtmpFN:
	movq	%rax, %r15
	jmp	.LBBFN_239
.LBBFN_187:
.LtmpFN:
	lock		decq	(%r12)
	movq	%rax, %r15
	jne	.LBBFN_202
	#MEMBARRIER
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::engine::PreparedQuery>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	136(%rsp), %rdi
	callq	*%rax
.LtmpFN:
	jmp	.LBBFN_202
.LBBFN_189:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBBFN_190:
.LtmpFN:
	movq	%rax, %r15
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	544(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
.LtmpFN:
	jmp	.LBBFN_194
.LBBFN_192:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBBFN_193:
.LtmpFN:
	movq	%rax, %r15
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	544(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
.LtmpFN:
.LBBFN_194:
	xorl	%r14d, %r14d
	jmp	.LBBFN_219
.LBBFN_195:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBBFN_196:
.LtmpFN:
	movq	144(%rsp), %rsi
	movq	%rax, %r15
	testq	%rsi, %rsi
	je	.LBBFN_204
	movq	152(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
	jmp	.LBBFN_204
.LBBFN_198:
.LtmpFN:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::string::String>>
	jmp	.LBBFN_200
.LBBFN_199:
.LtmpFN:
	movq	%rax, %r15
.LBBFN_200:
	movb	$1, %r14b
	jmp	.LBBFN_214
.LBBFN_201:
.LtmpFN:
	movq	%rax, %r15
.LBBFN_202:
	.cfi_escape 0x2e, 0x00
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::AdmittedSubstitutions>
	jmp	.LBBFN_210
.LBBFN_203:
.LtmpFN:
	movq	%rax, %r15
.LBBFN_204:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.LtmpFN:
	jmp	.LBBFN_210
.LBBFN_205:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBBFN_206:
.LtmpFN:
	movq	%rax, %r15
	jmp	.LBBFN_213
.LBBFN_207:
.LtmpFN:
	movq	%rax, %r15
	jmp	.LBBFN_269
.LBBFN_208:
.LtmpFN:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	leaq	1488(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_algebra::parser::ParserOptions>
	jmp	.LBBFN_213
.LBBFN_209:
.LtmpFN:
	movq	%rax, %r15
.LBBFN_210:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	2096(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::extension_env::ExtensionEnv>
.LtmpFN:
	jmp	.LBBFN_212
.LBBFN_211:
.LtmpFN:
	movq	%rax, %r15
.LBBFN_212:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1568(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
.LtmpFN:
.LBBFN_213:
	xorl	%r14d, %r14d
.LBBFN_214:
	cmpq	$0, 1272(%rsp)
	je	.LBBFN_217
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1272(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
.LtmpFN:
	jmp	.LBBFN_217
.LBBFN_216:
.LtmpFN:
	movq	%rax, %r15
.LBBFN_217:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	512(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry>>
.LtmpFN:
	jmp	.LBBFN_219
.LBBFN_218:
.LtmpFN:
	movq	%rax, %r15
.LBBFN_219:
	movq	24(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBBFN_269
	#MEMBARRIER
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	24(%rsp), %rdi
	callq	*%rax
.LtmpFN:
	jmp	.LBBFN_269
.LBBFN_221:
.LtmpFN:
	movq	%rax, %r15
	jmp	.LBBFN_267
.LBBFN_222:
.LtmpFN:
	movq	8(%r14), %rsi
	movq	%rax, %r15
	testq	%rsi, %rsi
	je	.LBBFN_239
	movq	16(%r14), %rdx
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	callq	*%rax
	jmp	.LBBFN_239
.LBBFN_224:
.LtmpFN:
	movq	%rax, %r15
	jmp	.LBBFN_275
.LBBFN_225:
.LtmpFN:
	movq	%rax, %r15
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	592(%rsp), %rdi
	callq	core::ptr::drop_glue::<<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}::{closure#0}>
.LtmpFN:
	jmp	.LBBFN_229
.LBBFN_226:
.LtmpFN:
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
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>
.LtmpFN:
	jmp	.LBBFN_249
.LBBFN_228:
.LtmpFN:
	movq	%rax, %r15
.LBBFN_229:
	movl	$0, 16(%rsp)
	xorl	%r13d, %r13d
	jmp	.LBBFN_276
.LBBFN_230:
.LtmpFN:
	movq	%rax, %r15
	jmp	.LBBFN_268
.LBBFN_231:
.LtmpFN:
	movb	$1, %r13b
	movq	%rax, %r15
	jmp	.LBBFN_250
.LBBFN_232:
.LtmpFN:
	jmp	.LBBFN_237
.LBBFN_233:
.LtmpFN:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	movq	%r12, %rdi
	callq	*%r13
	jmp	.LBBFN_238
.LBBFN_234:
.LtmpFN:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	movq	_Py_DecRef@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
	jmp	.LBBFN_238
.LBBFN_235:
.LtmpFN:
	movq	%rax, %r15
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<pyo3::err::PyErr>
.LtmpFN:
	jmp	.LBBFN_284
.LBBFN_236:
.LtmpFN:
.LBBFN_237:
	movq	%rax, %r15
.LBBFN_238:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	2096(%rsp), %rdi
	callq	core::ptr::drop_glue::<pyo3::err::PyErr>
.LtmpFN:
.LBBFN_239:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::AttachGuard as core::ops::drop::Drop>::drop@GOTPCREL(%rip), %rax
	leaq	592(%rsp), %rdi
	callq	*%rax
.LtmpFN:
	jmp	.LBBFN_284
.LBBFN_240:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBBFN_241:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBBFN_242:
.LtmpFN:
	movq	%rax, %r15
	testq	%r12, %r12
	je	.LBBFN_246
	negq	%r12
	addq	$160, %r14
	.p2align	4
.LBBFN_244:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.LtmpFN:
	addq	$160, %r14
	decq	%r12
	jne	.LBBFN_244
.LBBFN_246:
	movq	96(%rsp), %rax
	testq	%rax, %rax
	je	.LBBFN_249
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	callq	*%rax
.LBBFN_249:
	xorl	%r13d, %r13d
.LBBFN_250:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	240(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.LtmpFN:
	jmp	.LBBFN_276
.LBBFN_251:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBBFN_252:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBBFN_253:
.LtmpFN:
	movq	%rax, %r15
	testq	%r13, %r13
	je	.LBBFN_257
	negq	%r13
	addq	$160, %r14
.LBBFN_255:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.LtmpFN:
	addq	$160, %r14
	decq	%r13
	jne	.LBBFN_255
.LBBFN_257:
	movq	592(%rsp), %rax
	testq	%rax, %rax
	je	.LBBFN_259
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	callq	*%rax
.LBBFN_259:
	movq	640(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBBFN_262
	testq	%rsi, %rsi
	je	.LBBFN_262
	movq	648(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBBFN_262:
	leaq	664(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	616(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.LtmpFN:
	jmp	.LBBFN_275
.LBBFN_263:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBBFN_264:
.LtmpFN:
	movq	%rax, %r15
	jmp	.LBBFN_276
.LBBFN_265:
.LtmpFN:
	movq	%rax, %r15
	testq	%rbx, %rbx
	je	.LBBFN_267
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	movq	%r14, %rdi
	movq	%rbx, %rsi
	callq	*%rax
.LBBFN_267:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1568(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.LtmpFN:
.LBBFN_268:
	movb	$1, %r14b
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	592(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>
.LtmpFN:
.LBBFN_269:
	movq	640(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBBFN_272
	testq	%rsi, %rsi
	je	.LBBFN_272
	movq	648(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBBFN_272:
	testb	%r14b, %r14b
	je	.LBBFN_274
	leaq	664(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
.LBBFN_274:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	616(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.LtmpFN:
.LBBFN_275:
	movl	$0, 16(%rsp)
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::SuspendAttach as core::ops::drop::Drop>::drop@GOTPCREL(%rip), %rax
	leaq	272(%rsp), %rdi
	callq	*%rax
.LtmpFN:
	xorl	%r13d, %r13d
.LBBFN_276:
	cmpq	$0, 32(%rsp)
	jle	.LBBFN_278
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	80(%rsp), %rdi
	movq	32(%rsp), %rsi
	movl	$1, %edx
	callq	*%rax
.LBBFN_278:
	testb	%r13b, %r13b
	je	.LBBFN_280
	leaq	312(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	leaq	336(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	leaq	360(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
.LBBFN_280:
	cmpb	$0, 16(%rsp)
	je	.LBBFN_284
	movq	408(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBBFN_284
	testq	%rsi, %rsi
	je	.LBBFN_284
	movq	416(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBBFN_284:
	.cfi_escape 0x2e, 0x00
	movq	%r15, %rdi
	callq	_Unwind_Resume@PLT
.LBBFN_285:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBBFN_286:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBBFN_287:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_endFN:
