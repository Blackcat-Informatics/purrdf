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
	subq	$2648, %rsp
	.cfi_def_cfa_offset 2704
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	2768(%rsp), %rax
	movq	%rsi, %r12
	movq	%r9, %r14
	vmovups	(%r14), %xmm1
	movq	%r8, %r15
	movq	2712(%rsp), %r8
	movq	2736(%rsp), %r11
	movq	2728(%rsp), %rbp
	movq	2720(%rsp), %rbx
	movq	%rdi, 16(%rsp)
	movq	2744(%rsp), %rdi
	movq	2760(%rsp), %r9
	movq	2752(%rsp), %r10
	movq	%rdx, 120(%rsp)
	movq	%rcx, (%rsp)
	movq	16(%rax), %rsi
	vmovups	(%rax), %xmm0
	movq	16(%r14), %rax
	vmovups	(%r8), %ymm2
	movq	%rsi, 304(%rsp)
	movq	2704(%rsp), %rsi
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
	movq	296(%rsp), %rcx
	movq	304(%rsp), %rdx
	cmpq	$-1, %rax
	movq	%rax, 32(%rsp)
	movq	%rcx, 64(%rsp)
	cmovneq	%rcx, %rsi
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1584(%rsp), %rdi
	vzeroupper
	callq	purrdf_native::py_store::env::division_policy
.LtmpFN:
	cmpl	$1, 1584(%rsp)
	jne	.LBBFN_4
	vmovups	1608(%rsp), %ymm0
	vmovups	1596(%rsp), %ymm1
	movq	16(%rsp), %rcx
	movl	1592(%rsp), %eax
	movq	32(%rsp), %rsi
	movb	$1, %bpl
	vmovups	%ymm0, 24(%rcx)
	vmovups	%ymm1, 12(%rcx)
	movl	%eax, 8(%rcx)
	movq	$1, (%rcx)
	testq	%rsi, %rsi
	jle	.LBBFN_56
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	64(%rsp), %rdi
	movl	$1, %edx
	vzeroupper
	callq	*%rax
	jmp	.LBBFN_56
.LBBFN_4:
	movq	1588(%rsp), %rax
	movq	%rax, 112(%rsp)
	movb	$1, %al
	movl	%eax, 8(%rsp)
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1584(%rsp), %rdi
	movq	%r15, %rsi
	movb	$1, %r13b
	callq	purrdf_native::py_store::quad_store::collect_substitutions
.LtmpFN:
	cmpl	$1, 1584(%rsp)
	leaq	1592(%rsp), %r15
	jne	.LBBFN_7
	vmovups	16(%r15), %ymm1
	vmovups	(%r15), %ymm0
	movq	16(%rsp), %rax
	movb	$1, %bpl
	xorl	%ebx, %ebx
	vmovups	%ymm1, 608(%rsp)
	vmovups	%ymm0, 592(%rsp)
	vmovups	608(%rsp), %ymm1
	vmovups	592(%rsp), %ymm0
	vmovups	%ymm1, 24(%rax)
	vmovups	%ymm0, 8(%rax)
	movq	$1, (%rax)
	movq	32(%rsp), %rsi
	testq	%rsi, %rsi
	jg	.LBBFN_54
	jmp	.LBBFN_55
.LBBFN_7:
	vmovups	(%r15), %xmm0
	movq	16(%r15), %rax
	movq	%rax, 144(%rsp)
	vmovaps	%xmm0, 128(%rsp)
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	2736(%rsp), %rcx
	leaq	1584(%rsp), %rdi
	movq	%rbx, %rsi
	movq	%rbp, %rdx
	callq	purrdf_native::py_store::query::collect_relations
.LtmpFN:
	cmpl	$1, 1584(%rsp)
	jne	.LBBFN_11
	vmovups	16(%r15), %ymm1
	vmovups	(%r15), %ymm0
	movq	16(%rsp), %rax
	movb	$1, %r13b
	vmovups	%ymm1, 608(%rsp)
	vmovups	%ymm0, 592(%rsp)
	vmovups	608(%rsp), %ymm1
	vmovups	592(%rsp), %ymm0
	vmovups	%ymm1, 24(%rax)
	vmovups	%ymm0, 8(%rax)
	movq	$1, (%rax)
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	128(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.LtmpFN:
	xorl	%ebx, %ebx
	movb	$1, %bpl
	movq	32(%rsp), %rsi
	testq	%rsi, %rsi
	jg	.LBBFN_54
	jmp	.LBBFN_55
.LBBFN_11:
	movq	312(%rsp), %r8
	movq	320(%rsp), %rcx
	vmovups	(%r15), %xmm0
	movq	16(%r15), %rax
	movq	328(%rsp), %r15
	movq	352(%rsp), %rbx
	movq	360(%rsp), %rbp
	movq	384(%rsp), %r13
	movq	%r8, 56(%rsp)
	movq	%rcx, 48(%rsp)
	movq	336(%rsp), %r8
	movq	344(%rsp), %rcx
	movq	%rax, 96(%rsp)
	vmovaps	%xmm0, 80(%rsp)
	movq	%r8, 72(%rsp)
	movq	%rcx, 104(%rsp)
	movq	368(%rsp), %r8
	movq	392(%rsp), %rcx
	movq	%r8, 256(%rsp)
	movq	%rcx, 264(%rsp)
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	2752(%rsp), %rsi
	movq	2760(%rsp), %rdx
	leaq	2472(%rsp), %rdi
	callq	purrdf_native::xpath_regex::selection
.LtmpFN:
	movzbl	2472(%rsp), %eax
	cmpb	$-1, %al
	je	.LBBFN_25
	vmovups	2528(%rsp), %xmm0
	vmovups	2496(%rsp), %ymm1
	movq	16(%r14), %rcx
	movq	2704(%rsp), %rdx
	vmovups	2473(%rsp), %ymm5
	vmovaps	80(%rsp), %xmm2
	movq	%rcx, 2560(%rsp)
	movq	16(%rdx), %rcx
	vmovups	(%rdx), %xmm4
	movq	2744(%rsp), %rdx
	vmovaps	%xmm0, 1168(%rsp)
	vmovups	%ymm1, 1136(%rsp)
	vmovups	(%r14), %xmm1
	vmovups	%ymm5, 1113(%rsp)
	movq	%r12, 1192(%rsp)
	vmovaps	%xmm2, 944(%rsp)
	vmovaps	%xmm1, 2544(%rsp)
	movq	%rcx, 2584(%rsp)
	movq	2712(%rsp), %rcx
	vmovups	%xmm4, 2568(%rsp)
	vmovups	(%rcx), %ymm3
	vmovups	16(%rcx), %ymm1
	movq	96(%rsp), %rcx
	movq	%rcx, 960(%rsp)
	movq	16(%rdx), %rcx
	vmovups	%ymm3, 2592(%rsp)
	vmovups	%ymm1, 2608(%rsp)
	vmovups	(%rdx), %xmm1
	movq	120(%rsp), %rdx
	vmovups	2544(%rsp), %zmm0
	movq	%rcx, 1008(%rsp)
	movq	112(%rsp), %rcx
	vmovaps	%xmm1, 992(%rsp)
	vmovups	2576(%rsp), %zmm1
	vmovups	%zmm1, 1048(%rsp)
	vmovups	%zmm0, 1016(%rsp)
	vmovaps	128(%rsp), %xmm0
	movb	%al, 1112(%rsp)
	movq	%rcx, 1184(%rsp)
	movq	144(%rsp), %rax
	movq	(%rsp), %rcx
	movq	%rdx, 1200(%rsp)
	movq	%rcx, 1208(%rsp)
	movq	%rax, 984(%rsp)
	vmovups	%xmm0, 968(%rsp)
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
	leaq	592(%rsp), %rdi
	movq	%r12, %rsi
	callq	*%rax
.LtmpFN:
	cmpq	$-1, 592(%rsp)
	je	.LBBFN_113
	vmovups	624(%rsp), %zmm1
	vmovups	592(%rsp), %zmm0
	leaq	1584(%rsp), %rax
	movq	%rax, 1248(%rsp)
	movq	<purrdf_core::diagnostic::RdfDiagnostic as core::fmt::Display>::fmt@GOTPCREL(%rip), %rax
	movq	%rax, 1256(%rsp)
	vmovups	%zmm1, 1616(%rsp)
	vmovups	%zmm0, 1584(%rsp)
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	alloc::fmt::format::format_inner@GOTPCREL(%rip), %rax
	leaq	.Lalloc_0e5f90e3dc675d538220deef7d17e145(%rip), %rsi
	leaq	2112(%rsp), %rdi
	leaq	1248(%rsp), %rdx
	vzeroupper
	callq	*%rax
.LtmpFN:
	movq	2112(%rsp), %rbx
	movq	2120(%rsp), %r14
	movq	2128(%rsp), %r15
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
	leaq	1584(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.LtmpFN:
	movq	952(%rsp), %rbx
	movq	960(%rsp), %rax
	vxorps	%xmm0, %xmm0, %xmm0
	vmovaps	%xmm0, 160(%rsp)
	testq	%rax, %rax
	je	.LBBFN_23
	movl	$1, %r13d
	movq	%rbx, %r14
	subq	%rax, %r13
	.p2align	4
.LBBFN_21:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.LtmpFN:
	incq	%r13
	addq	$160, %r14
	cmpq	$1, %r13
	jne	.LBBFN_21
.LBBFN_23:
	movq	944(%rsp), %rax
	movb	$1, %r13b
	movl	$1, %r15d
	leaq	.Lvtable.22(%rip), %r14
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
	movq	992(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBBFN_122
	testq	%rsi, %rsi
	je	.LBBFN_122
	movq	1000(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBBFN_122:
	movq	$-1, %rbx
	testb	%r13b, %r13b
	je	.LBBFN_124
	.cfi_escape 0x2e, 0x00
	leaq	1016(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
.LBBFN_124:
	movl	(%rsp), %r13d
	leaq	968(%rsp), %rdi
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
	vmovaps	160(%rsp), %xmm0
	vmovaps	%xmm0, 1216(%rsp)
	cmpq	$-1, %rbx
	je	.LBBFN_130
	vmovaps	1216(%rsp), %xmm0
	movq	%rbx, 1584(%rsp)
	vmovups	%xmm0, 1592(%rsp)
	movq	%r15, 1608(%rsp)
	movq	%r12, 1616(%rsp)
	movq	%r14, 1624(%rsp)
	movl	%ebp, 1632(%rsp)
	movl	%r13d, 1636(%rsp)
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	16(%rsp), %rdi
	leaq	1584(%rsp), %rsi
	callq	purrdf_native::py_store::query::materialize_results
.LtmpFN:
	movq	32(%rsp), %rsi
	testq	%rsi, %rsi
	jle	.LBBFN_83
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	64(%rsp), %rdi
	jmp	.LBBFN_81
.LBBFN_25:
	vmovups	2480(%rsp), %ymm0
	vmovups	2496(%rsp), %ymm1
	movq	16(%rsp), %rax
	movq	56(%rsp), %r12
	vmovups	%ymm1, 24(%rax)
	vmovups	%ymm0, 8(%rax)
	movq	$1, (%rax)
	cmpq	$-1, %rbp
	je	.LBBFN_30
	testq	%rbp, %rbp
	je	.LBBFN_28
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	256(%rsp), %rdi
	movl	$1, %edx
	movq	%rbp, %rsi
	vzeroupper
	callq	*%rax
.LBBFN_28:
	testq	%r13, %r13
	je	.LBBFN_30
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	264(%rsp), %rdi
	movl	$1, %edx
	movq	%r13, %rsi
	vzeroupper
	callq	*%rax
.LBBFN_30:
	movq	72(%rsp), %rbp
	cmpq	$-1, %rbp
	je	.LBBFN_38
	movq	%r12, %r13
	testq	%rbx, %rbx
	je	.LBBFN_36
	movq	104(%rsp), %rax
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r12
	leaq	8(%rax), %r14
	jmp	.LBBFN_34
	.p2align	4
.LBBFN_33:
	addq	$24, %r14
	decq	%rbx
	je	.LBBFN_36
.LBBFN_34:
	movq	-8(%r14), %rsi
	testq	%rsi, %rsi
	je	.LBBFN_33
	movq	(%r14), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r12
	jmp	.LBBFN_33
.LBBFN_36:
	movq	%r13, %r12
	testq	%rbp, %rbp
	je	.LBBFN_38
	shlq	$3, %rbp
	leaq	(%rbp,%rbp,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	104(%rsp), %rdi
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBBFN_38:
	cmpq	$-1, %r12
	je	.LBBFN_46
	testq	%r15, %r15
	je	.LBBFN_44
	movq	48(%rsp), %rax
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r14
	leaq	8(%rax), %rbx
	jmp	.LBBFN_42
	.p2align	4
.LBBFN_41:
	addq	$24, %rbx
	decq	%r15
	je	.LBBFN_44
.LBBFN_42:
	movq	-8(%rbx), %rsi
	testq	%rsi, %rsi
	je	.LBBFN_41
	movq	(%rbx), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r14
	jmp	.LBBFN_41
.LBBFN_44:
	testq	%r12, %r12
	je	.LBBFN_46
	shlq	$3, %r12
	leaq	(%r12,%r12,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	48(%rsp), %rdi
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBBFN_46:
	movq	88(%rsp), %rbx
	movq	96(%rsp), %rax
	testq	%rax, %rax
	je	.LBBFN_50
	movl	$1, %r12d
	movq	%rbx, %r14
	subq	%rax, %r12
	.p2align	4
.LBBFN_48:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.LtmpFN:
	incq	%r12
	addq	$160, %r14
	cmpq	$1, %r12
	jne	.LBBFN_48
.LBBFN_50:
	movq	80(%rsp), %rax
	testq	%rax, %rax
	je	.LBBFN_52
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBBFN_52:
	movb	$1, %bpl
	xorl	%r13d, %r13d
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	128(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.LtmpFN:
	movb	$1, %bl
	movq	32(%rsp), %rsi
	testq	%rsi, %rsi
	jle	.LBBFN_55
.LBBFN_54:
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	64(%rsp), %rdi
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LBBFN_55:
	testb	%bl, %bl
	jne	.LBBFN_77
.LBBFN_56:
	movq	312(%rsp), %r14
	cmpq	$-1, %r14
	je	.LBBFN_64
	movq	320(%rsp), %rbx
	movq	328(%rsp), %r15
	testq	%r15, %r15
	je	.LBBFN_62
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r13
	leaq	8(%rbx), %r12
	jmp	.LBBFN_60
	.p2align	4
.LBBFN_59:
	addq	$24, %r12
	decq	%r15
	je	.LBBFN_62
.LBBFN_60:
	movq	-8(%r12), %rsi
	testq	%rsi, %rsi
	je	.LBBFN_59
	movq	(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r13
	jmp	.LBBFN_59
.LBBFN_62:
	testq	%r14, %r14
	je	.LBBFN_64
	shlq	$3, %r14
	leaq	(%r14,%r14,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBBFN_64:
	movq	336(%rsp), %r14
	cmpq	$-1, %r14
	je	.LBBFN_72
	movq	344(%rsp), %rbx
	movq	352(%rsp), %r15
	testq	%r15, %r15
	je	.LBBFN_70
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r13
	leaq	8(%rbx), %r12
	jmp	.LBBFN_68
	.p2align	4
.LBBFN_67:
	addq	$24, %r12
	decq	%r15
	je	.LBBFN_70
.LBBFN_68:
	movq	-8(%r12), %rsi
	testq	%rsi, %rsi
	je	.LBBFN_67
	movq	(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r13
	jmp	.LBBFN_67
.LBBFN_70:
	testq	%r14, %r14
	je	.LBBFN_72
	shlq	$3, %r14
	leaq	(%r14,%r14,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBBFN_72:
	movq	360(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBBFN_77
	testq	%rsi, %rsi
	je	.LBBFN_75
	movq	368(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LBBFN_75:
	movq	384(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBBFN_77
	movq	392(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
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
	movq	16(%rsp), %rax
	cmpl	$1, (%rax)
	jne	.LBBFN_112
	leaq	8(%rax), %rbx
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::AttachGuard>::attach@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LtmpFN:
	movl	%eax, 944(%rsp)
	movq	PyExc_ValueError@GOTPCREL(%rip), %rax
	vmovups	16(%rbx), %ymm1
	vmovups	(%rbx), %ymm0
	movq	(%rax), %r14
	vmovups	%ymm1, 2128(%rsp)
	vmovups	%ymm0, 2112(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr>::get_type@GOTPCREL(%rip), %rax
	leaq	2112(%rsp), %rdi
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
	leaq	2112(%rsp), %rdi
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
	leaq	1584(%rsp), %rdi
	movq	%r14, %rsi
	movq	%r12, %rdx
	callq	*%rax
.LtmpFN:
	leaq	600(%rsp), %r15
	.cfi_escape 0x2e, 0x00
	movq	%r12, %rdi
	callq	*%r13
	cmpb	$1, 1584(%rsp)
	je	.LBBFN_97
	cmpb	$1, 1585(%rsp)
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
	leaq	592(%rsp), %rdi
	movl	$10, %ecx
	movq	%r15, %r8
	movq	%r14, %rsi
	movq	%r12, %r15
	callq	<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::setattr::<&str, pyo3::instance::Py<pyo3::types::any::PyAny>>
.LtmpFN:
	cmpb	$0, 592(%rsp)
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
	leaq	1584(%rsp), %rdi
	movq	%r14, %rsi
	movq	%rax, %rdx
	callq	*%rbp
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	%r12, %rdi
	callq	*%r13
	cmpb	$0, 1584(%rsp)
	je	.LBBFN_107
.LBBFN_97:
	leaq	1592(%rsp), %rax
	vmovups	16(%rax), %ymm1
	vmovups	(%rax), %ymm0
	vmovups	%ymm1, 16(%r15)
	vmovups	%ymm0, (%r15)
.LBBFN_98:
	vmovups	(%r15), %xmm0
	vmovups	624(%rsp), %xmm1
	movq	616(%rsp), %r15
	movq	640(%rsp), %rbp
	cmpq	$0, 2128(%rsp)
	vmovaps	%xmm0, 1584(%rsp)
	vmovaps	%xmm1, 32(%rsp)
	je	.LBBFN_111
	movq	2136(%rsp), %r12
	movq	2144(%rsp), %r14
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
	vmovaps	2112(%rsp), %xmm0
	vmovups	2136(%rsp), %xmm1
	movq	2128(%rsp), %r15
	movq	2152(%rsp), %rbp
	vmovaps	%xmm0, 1584(%rsp)
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
	cmpb	$0, 1585(%rsp)
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
	leaq	592(%rsp), %rdi
	movl	$12, %ecx
	movq	%r15, %r8
	movq	%r14, %rsi
	movq	%r12, %r15
	callq	<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::setattr::<&str, pyo3::instance::Py<pyo3::types::any::PyAny>>
.LtmpFN:
	cmpb	$0, 592(%rsp)
	jne	.LBBFN_98
.LBBFN_110:
	vmovups	2136(%rsp), %xmm0
	vmovaps	2112(%rsp), %xmm1
	movq	2128(%rsp), %r15
	movq	2152(%rsp), %rbp
	vmovaps	%xmm0, 32(%rsp)
	vmovaps	%xmm1, 1584(%rsp)
.LBBFN_111:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::AttachGuard as core::ops::drop::Drop>::drop@GOTPCREL(%rip), %rax
	leaq	944(%rsp), %rdi
	vzeroupper
	callq	*%rax
	vmovaps	1584(%rsp), %xmm0
	vmovaps	32(%rsp), %xmm1
	movq	16(%rsp), %rax
	vmovups	%xmm0, (%rbx)
	movq	%r15, 24(%rax)
	vmovups	%xmm1, 32(%rax)
	movq	%rbp, 48(%rax)
	movq	$1, (%rax)
.LBBFN_112:
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
.LBBFN_113:
	.cfi_def_cfa_offset 2704
	movq	600(%rsp), %rdx
	movb	$1, %r13b
	movq	%rdx, 24(%rsp)
	addq	$16, %rdx
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1584(%rsp), %rdi
	leaq	944(%rsp), %rsi
	callq	purrdf_native::py_store::query::build_relations
.LtmpFN:
	vmovups	1592(%rsp), %xmm0
	movl	1584(%rsp), %eax
	movq	1608(%rsp), %r15
	movq	1616(%rsp), %r12
	movq	1624(%rsp), %r14
	movl	1632(%rsp), %ebp
	movl	1636(%rsp), %ecx
	vmovaps	%xmm0, 592(%rsp)
	cmpl	$1, %eax
	jne	.LBBFN_131
	vmovaps	592(%rsp), %xmm0
	movl	%ecx, (%rsp)
	movb	$1, %r13b
	vmovaps	%xmm0, 160(%rsp)
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
	movq	16(%rsp), %rax
	movb	$1, %bl
	vmovups	%xmm0, 8(%rax)
	movq	%r15, 24(%rax)
	movq	%r12, 32(%rax)
	movq	%r14, 40(%rax)
	movl	%ebp, 48(%rax)
	movl	%r13d, 52(%rax)
	movq	$1, (%rax)
	xorl	%ebp, %ebp
	movq	32(%rsp), %rsi
	testq	%rsi, %rsi
	jg	.LBBFN_54
	jmp	.LBBFN_55
.LBBFN_131:
	movq	1656(%rsp), %rax
	vmovups	1640(%rsp), %xmm0
	vmovaps	592(%rsp), %xmm1
	movq	1000(%rsp), %r8
	movq	1008(%rsp), %rdx
	xorl	%esi, %esi
	movb	$1, %r13b
	movq	%rax, 576(%rsp)
	movq	992(%rsp), %rax
	movq	%r8, 48(%rsp)
	vmovaps	%xmm0, 560(%rsp)
	vmovaps	%xmm1, 512(%rsp)
	movq	%r15, 528(%rsp)
	movq	%r12, 536(%rsp)
	movq	%r14, 544(%rsp)
	movl	%ebp, 552(%rsp)
	movl	%ecx, 556(%rsp)
	cmpq	$-1, %rax
	movq	%rax, 72(%rsp)
	cmovneq	%r8, %rsi
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_validate::query::statistical_aggregates@GOTPCREL(%rip), %rax
	leaq	1464(%rsp), %rbx
	movq	%rbx, %rdi
	callq	*%rax
.LtmpFN:
	cmpq	$-1, 1016(%rsp)
	je	.LBBFN_134
	movq	1032(%rsp), %rdx
	movq	1024(%rsp), %rsi
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	592(%rsp), %rdi
	callq	<alloc::vec::Vec<alloc::string::String> as core::clone::Clone>::clone
.LtmpFN:
	jmp	.LBBFN_135
.LBBFN_134:
	movq	$0, 592(%rsp)
	movq	$8, 600(%rsp)
	movq	$0, 608(%rsp)
.LBBFN_135:
	cmpq	$-1, 1040(%rsp)
	je	.LBBFN_138
	movq	1056(%rsp), %rdx
	movq	1048(%rsp), %rsi
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1584(%rsp), %rdi
	callq	<alloc::vec::Vec<alloc::string::String> as core::clone::Clone>::clone
.LtmpFN:
	movq	1584(%rsp), %rdx
	movq	1592(%rsp), %rax
	movq	1600(%rsp), %rcx
	jmp	.LBBFN_139
.LBBFN_138:
	movl	$8, %eax
	xorl	%ecx, %ecx
	xorl	%edx, %edx
.LBBFN_139:
	vmovups	592(%rsp), %xmm0
	movq	608(%rsp), %rsi
	movq	%rsi, 1520(%rsp)
	vmovaps	%xmm0, 1504(%rsp)
	movq	%rdx, 1528(%rsp)
	movq	%rax, 1536(%rsp)
	movq	%rcx, 1544(%rsp)
	movq	$0, 1552(%rsp)
	movq	$8, 1560(%rsp)
	movq	$0, 1568(%rsp)
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1584(%rsp), %rdi
	leaq	1016(%rsp), %rsi
	callq	purrdf_native::py_store::query::build_engine
.LtmpFN:
	movq	512(%rsp), %rax
	movq	1464(%rsp), %r8
	leaq	512(%rsp), %rdx
	testq	%rax, %rax
	movq	%rax, 56(%rsp)
	movq	%r8, 8(%rsp)
	cmoveq	%rax, %rdx
	testq	%r8, %r8
	cmoveq	%r8, %rbx
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	592(%rsp), %rdi
	leaq	1504(%rsp), %rsi
	movq	%rbx, %rcx
	callq	purrdf_native::py_store::env::extension_env
.LtmpFN:
	vmovups	600(%rsp), %xmm0
	movq	592(%rsp), %rax
	movq	616(%rsp), %r15
	movq	624(%rsp), %r12
	movq	632(%rsp), %r14
	movl	640(%rsp), %ebp
	movl	644(%rsp), %ecx
	vmovaps	%xmm0, 1248(%rsp)
	cmpq	$-1, %rax
	je	.LBBFN_154
	vmovups	712(%rsp), %zmm1
	vmovups	880(%rsp), %zmm0
	vmovups	648(%rsp), %zmm4
	vmovups	776(%rsp), %zmm2
	vmovups	840(%rsp), %zmm3
	movq	976(%rsp), %rbx
	vmovups	%zmm1, 2232(%rsp)
	vmovaps	1248(%rsp), %xmm1
	vmovups	%zmm0, 2400(%rsp)
	vmovups	%zmm3, 2360(%rsp)
	vmovups	%zmm2, 2296(%rsp)
	vmovups	%zmm4, 2168(%rsp)
	vmovups	%xmm1, 2120(%rsp)
	movq	%r15, 2136(%rsp)
	movq	%r12, 2144(%rsp)
	movq	%r14, 2152(%rsp)
	movq	984(%rsp), %r14
	movq	24(%rsp), %r15
	movl	%ebp, 2160(%rsp)
	movl	%ecx, 2164(%rsp)
	movq	%rax, 2112(%rsp)
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::engine::AdmittedSubstitutions>::requested@GOTPCREL(%rip), %rax
	leaq	592(%rsp), %rdi
	movl	$8, %ecx
	movq	%rbx, %rsi
	movq	%r14, %rdx
	xorl	%r8d, %r8d
	vzeroupper
	callq	*%rax
.LtmpFN:
	leaq	608(%rsp), %rax
.LtmpFN:
	.cfi_escape 0x2e, 0x10
	movq	120(%rsp), %rdx
	movq	<purrdf_sparql_eval::engine::NativeSparqlEngine>::prepare_request@GOTPCREL(%rip), %r10
	movq	(%rsp), %rcx
	leaq	1248(%rsp), %r12
	leaq	2112(%rsp), %r13
	leaq	1584(%rsp), %rsi
	xorl	%r8d, %r8d
	movq	%r12, %rdi
	pushq	%rax
	.cfi_adjust_cfa_offset 8
	pushq	%r13
	.cfi_adjust_cfa_offset 8
	callq	*%r10
	addq	$16, %rsp
	.cfi_adjust_cfa_offset -16
.LtmpFN:
	movq	1248(%rsp), %rax
	movq	1256(%rsp), %rbp
	cmpq	$-1, %rax
	je	.LBBFN_160
	vmovups	1280(%rsp), %zmm1
	vmovups	1264(%rsp), %zmm0
	movq	608(%rsp), %rsi
	vmovups	%zmm1, 192(%rsp)
	vmovups	%zmm0, 176(%rsp)
	movq	%rax, 160(%rsp)
	movq	%rbp, 168(%rsp)
	cmpq	$10, %rsi
	jb	.LBBFN_147
	movq	616(%rsp), %rdi
	shlq	$4, %rsi
	addq	$-16, %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBBFN_147:
	movq	744(%rsp), %rsi
	cmpq	$9, %rsi
	jbe	.LBBFN_149
.LBBFN_148:
	movq	752(%rsp), %rdi
	shlq	$4, %rsi
	addq	$-16, %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBBFN_149:
	vmovups	176(%rsp), %xmm0
	vmovups	224(%rsp), %ymm1
	movq	160(%rsp), %rax
	movq	168(%rsp), %rbx
	movq	192(%rsp), %r15
	movq	200(%rsp), %r12
	movq	208(%rsp), %r14
	movl	216(%rsp), %ebp
	movl	220(%rsp), %r13d
	vmovaps	%xmm0, 1232(%rsp)
	vmovups	%ymm1, 1424(%rsp)
	cmpq	$-1, %rax
	je	.LBBFN_166
	vmovaps	1232(%rsp), %xmm0
	vmovups	1424(%rsp), %ymm1
	movq	%rbx, 600(%rsp)
	leaq	592(%rsp), %rcx
	movq	%rcx, 1248(%rsp)
	vmovups	%xmm0, 608(%rsp)
	movq	%r15, 624(%rsp)
	movq	%r12, 632(%rsp)
	movq	%r14, 640(%rsp)
	movl	%ebp, 648(%rsp)
	movl	%r13d, 652(%rsp)
	vmovups	%ymm1, 656(%rsp)
	movq	%rax, 592(%rsp)
	movq	<purrdf_core::diagnostic::RdfDiagnostic as core::fmt::Display>::fmt@GOTPCREL(%rip), %rax
	movq	%rax, 1256(%rsp)
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	alloc::fmt::format::format_inner@GOTPCREL(%rip), %rax
	leaq	.Lalloc_592fabd2ffa0e5a6b6713c88c8b00c99(%rip), %rsi
	leaq	160(%rsp), %rdi
	leaq	1248(%rsp), %rdx
	vzeroupper
	callq	*%rax
.LtmpFN:
	movq	672(%rsp), %rbx
	movq	600(%rsp), %rdi
	movq	608(%rsp), %rsi
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
	leaq	1248(%rsp), %rdi
	leaq	160(%rsp), %rsi
	movq	%rbx, %rdx
	callq	purrdf_native::py_store::presentation::presented_value_error
.LtmpFN:
	jmp	.LBBFN_168
.LBBFN_154:
	vmovaps	1248(%rsp), %xmm0
	movl	%ecx, (%rsp)
	vmovaps	%xmm0, 160(%rsp)
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1584(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
.LtmpFN:
	cmpq	$0, 8(%rsp)
	je	.LBBFN_157
	xorl	%r13d, %r13d
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1464(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
.LtmpFN:
.LBBFN_157:
	cmpq	$0, 56(%rsp)
	je	.LBBFN_182
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	512(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::property_fn::PropertyFunction>)>>
.LtmpFN:
	xorl	%r13d, %r13d
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
	movq	112(%rsp), %rax
	addq	$16, %r15
	movq	%rbp, 1424(%rsp)
	leaq	16(%rbp), %rcx
	movq	%rax, 1248(%rsp)
	vmovups	%zmm0, 1256(%rsp)
	vmovups	%zmm2, 1272(%rsp)
	movq	%r13, 1336(%rsp)
	movq	$8, 1344(%rsp)
	movq	$0, 1352(%rsp)
	vmovups	%zmm1, 1360(%rsp)
.LtmpFN:
	.cfi_escape 0x2e, 0x10
	subq	$8, %rsp
	.cfi_adjust_cfa_offset 8
	movq	%r15, %rdx
	leaq	168(%rsp), %rdi
	leaq	1592(%rsp), %rsi
	movq	%rbx, %r8
	movq	%r14, %r9
	pushq	%r12
	.cfi_adjust_cfa_offset 8
	vzeroupper
	callq	<purrdf_sparql_eval::engine::NativeSparqlEngine>::query_prepared_admitted::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::dataset_view::NoopReservation<!>>
	addq	$16, %rsp
	.cfi_adjust_cfa_offset -16
.LtmpFN:
	lock		decq	(%rbp)
	jne	.LBBFN_163
	#MEMBARRIER
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::engine::PreparedQuery>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1424(%rsp), %rdi
	callq	*%rax
.LtmpFN:
.LBBFN_163:
	movq	608(%rsp), %rsi
	cmpq	$10, %rsi
	jb	.LBBFN_165
	movq	616(%rsp), %rdi
	shlq	$4, %rsi
	addq	$-16, %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	callq	*%rax
.LBBFN_165:
	movq	744(%rsp), %rsi
	cmpq	$10, %rsi
	jae	.LBBFN_148
	jmp	.LBBFN_149
.LBBFN_166:
	vmovaps	1232(%rsp), %xmm0
	jmp	.LBBFN_170
.LBBFN_167:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1248(%rsp), %rdi
	leaq	160(%rsp), %rsi
	movq	%rax, %rdx
	callq	purrdf_native::py_store::presentation::refusal_value_error
.LtmpFN:
.LBBFN_168:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	592(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.LtmpFN:
	vmovups	1248(%rsp), %xmm0
	movq	1264(%rsp), %r15
	movq	1272(%rsp), %r12
	movq	1280(%rsp), %r14
	movl	1288(%rsp), %ebp
	movl	1292(%rsp), %r13d
	movq	$-1, %rbx
.LBBFN_170:
	vmovaps	%xmm0, 160(%rsp)
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	2112(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::extension_env::ExtensionEnv>
.LtmpFN:
.LtmpFN:
	movl	%r13d, (%rsp)
	.cfi_escape 0x2e, 0x00
	leaq	1584(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
.LtmpFN:
	cmpq	$0, 8(%rsp)
	je	.LBBFN_175
	xorl	%r13d, %r13d
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1464(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
.LtmpFN:
.LBBFN_175:
	cmpq	$0, 56(%rsp)
	je	.LBBFN_178
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	512(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::property_fn::PropertyFunction>)>>
.LtmpFN:
	xorl	%r13d, %r13d
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	544(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
.LtmpFN:
.LBBFN_178:
	movq	24(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBBFN_180
	xorl	%r13d, %r13d
	#MEMBARRIER
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	24(%rsp), %rdi
	callq	*%rax
.LtmpFN:
.LBBFN_180:
	movq	72(%rsp), %rsi
	testq	%rsi, %rsi
	jle	.LBBFN_124
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	48(%rsp), %rdi
	movl	$1, %edx
	callq	*%rax
	jmp	.LBBFN_124
.LBBFN_182:
	xorl	%r13d, %r13d
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
	lock		decq	(%rbp)
	movq	%rax, %r15
	jne	.LBBFN_202
	#MEMBARRIER
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::engine::PreparedQuery>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1424(%rsp), %rdi
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
	xorl	%r13d, %r13d
	jmp	.LBBFN_219
.LBBFN_195:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBBFN_196:
.LtmpFN:
	movq	160(%rsp), %rsi
	movq	%rax, %r15
	testq	%rsi, %rsi
	je	.LBBFN_204
	movq	168(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
	jmp	.LBBFN_204
.LBBFN_198:
.LtmpFN:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	leaq	592(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::string::String>>
	jmp	.LBBFN_200
.LBBFN_199:
.LtmpFN:
	movq	%rax, %r15
.LBBFN_200:
	movb	$1, %r13b
	jmp	.LBBFN_214
.LBBFN_201:
.LtmpFN:
	movq	%rax, %r15
.LBBFN_202:
	.cfi_escape 0x2e, 0x00
	leaq	592(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::AdmittedSubstitutions>
	jmp	.LBBFN_210
.LBBFN_203:
.LtmpFN:
	movq	%rax, %r15
.LBBFN_204:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	592(%rsp), %rdi
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
	jmp	.LBBFN_270
.LBBFN_208:
.LtmpFN:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	leaq	1504(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_algebra::parser::ParserOptions>
	jmp	.LBBFN_213
.LBBFN_209:
.LtmpFN:
	movq	%rax, %r15
.LBBFN_210:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	2112(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::extension_env::ExtensionEnv>
.LtmpFN:
	jmp	.LBBFN_212
.LBBFN_211:
.LtmpFN:
	movq	%rax, %r15
.LBBFN_212:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1584(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
.LtmpFN:
.LBBFN_213:
	xorl	%r13d, %r13d
.LBBFN_214:
	cmpq	$0, 1464(%rsp)
	je	.LBBFN_217
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1464(%rsp), %rdi
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
	jne	.LBBFN_270
	#MEMBARRIER
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	24(%rsp), %rdi
	callq	*%rax
.LtmpFN:
	jmp	.LBBFN_270
.LBBFN_221:
.LtmpFN:
	movq	%rax, %r15
	jmp	.LBBFN_268
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
	jmp	.LBBFN_276
.LBBFN_225:
.LtmpFN:
	movq	%rax, %r15
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	944(%rsp), %rdi
	callq	core::ptr::drop_glue::<<purrdf_native::py_store::quad_store::PyQuadStore>::query_impl<()>::{closure#0}::{closure#0}>
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
	leaq	80(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>
.LtmpFN:
	jmp	.LBBFN_250
.LBBFN_228:
.LtmpFN:
	movq	%rax, %r15
.LBBFN_229:
	movl	$0, 8(%rsp)
	xorl	%r13d, %r13d
	jmp	.LBBFN_277
.LBBFN_230:
.LtmpFN:
	movq	%rax, %r15
	jmp	.LBBFN_269
.LBBFN_231:
.LtmpFN:
	movb	$1, %r13b
	movq	%rax, %r15
	jmp	.LBBFN_251
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
	jmp	.LBBFN_285
.LBBFN_236:
.LtmpFN:
.LBBFN_237:
	movq	%rax, %r15
.LBBFN_238:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	2112(%rsp), %rdi
	callq	core::ptr::drop_glue::<pyo3::err::PyErr>
.LtmpFN:
.LBBFN_239:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::AttachGuard as core::ops::drop::Drop>::drop@GOTPCREL(%rip), %rax
	leaq	944(%rsp), %rdi
	callq	*%rax
.LtmpFN:
	jmp	.LBBFN_285
.LBBFN_240:
.LtmpFN:
	movq	%rax, %r15
	jmp	.LBBFN_277
.LBBFN_241:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBBFN_242:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBBFN_243:
.LtmpFN:
	movq	%rax, %r15
	testq	%r12, %r12
	je	.LBBFN_247
	negq	%r12
	addq	$160, %r14
	.p2align	4
.LBBFN_245:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.LtmpFN:
	addq	$160, %r14
	decq	%r12
	jne	.LBBFN_245
.LBBFN_247:
	movq	80(%rsp), %rax
	testq	%rax, %rax
	je	.LBBFN_250
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	callq	*%rax
.LBBFN_250:
	xorl	%r13d, %r13d
.LBBFN_251:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	128(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.LtmpFN:
	jmp	.LBBFN_277
.LBBFN_252:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBBFN_253:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBBFN_254:
.LtmpFN:
	movq	%rax, %r15
	testq	%r13, %r13
	je	.LBBFN_258
	negq	%r13
	addq	$160, %r14
.LBBFN_256:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.LtmpFN:
	addq	$160, %r14
	decq	%r13
	jne	.LBBFN_256
.LBBFN_258:
	movq	944(%rsp), %rax
	testq	%rax, %rax
	je	.LBBFN_260
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	callq	*%rax
.LBBFN_260:
	movq	992(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBBFN_263
	testq	%rsi, %rsi
	je	.LBBFN_263
	movq	1000(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBBFN_263:
	leaq	1016(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	968(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.LtmpFN:
	jmp	.LBBFN_276
.LBBFN_264:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBBFN_265:
.LtmpFN:
	movq	%rax, %r15
	movb	$1, %al
	movb	$1, %r13b
	movl	%eax, 8(%rsp)
	jmp	.LBBFN_277
.LBBFN_266:
.LtmpFN:
	movq	%rax, %r15
	testq	%rbx, %rbx
	je	.LBBFN_268
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	movq	%r14, %rdi
	movq	%rbx, %rsi
	callq	*%rax
.LBBFN_268:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	1584(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.LtmpFN:
.LBBFN_269:
	movb	$1, %r13b
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	944(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>
.LtmpFN:
.LBBFN_270:
	movq	992(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBBFN_273
	testq	%rsi, %rsi
	je	.LBBFN_273
	movq	1000(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBBFN_273:
	testb	%r13b, %r13b
	je	.LBBFN_275
	leaq	1016(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
.LBBFN_275:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	leaq	968(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.LtmpFN:
.LBBFN_276:
	movl	$0, 8(%rsp)
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::SuspendAttach as core::ops::drop::Drop>::drop@GOTPCREL(%rip), %rax
	leaq	272(%rsp), %rdi
	callq	*%rax
.LtmpFN:
	xorl	%r13d, %r13d
.LBBFN_277:
	cmpq	$0, 32(%rsp)
	jle	.LBBFN_279
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	64(%rsp), %rdi
	movq	32(%rsp), %rsi
	movl	$1, %edx
	callq	*%rax
.LBBFN_279:
	testb	%r13b, %r13b
	je	.LBBFN_281
	leaq	312(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	leaq	336(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	leaq	360(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
.LBBFN_281:
	cmpb	$0, 8(%rsp)
	je	.LBBFN_285
	movq	408(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBBFN_285
	testq	%rsi, %rsi
	je	.LBBFN_285
	movq	416(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBBFN_285:
	.cfi_escape 0x2e, 0x00
	movq	%r15, %rdi
	callq	_Unwind_Resume@PLT
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
.LBBFN_288:
.LtmpFN:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_endFN:
