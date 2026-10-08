<purrdf_native::py_store::quad_store::PyQuadStore>::query:
.Lfunc_begin1846:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception1846
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
.Ltmp41900:
	.cfi_escape 0x2e, 0x00
	leaq	1584(%rsp), %rdi
	vzeroupper
	callq	purrdf_native::py_store::env::division_policy
.Ltmp41901:
	cmpl	$1, 1584(%rsp)
	jne	.LBB3010_4
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
	jle	.LBB3010_56
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	64(%rsp), %rdi
	movl	$1, %edx
	vzeroupper
	callq	*%rax
	jmp	.LBB3010_56
.LBB3010_4:
	movq	1588(%rsp), %rax
	movq	%rax, 112(%rsp)
	movb	$1, %al
	movl	%eax, 8(%rsp)
.Ltmp41903:
	.cfi_escape 0x2e, 0x00
	leaq	1584(%rsp), %rdi
	movq	%r15, %rsi
	movb	$1, %r13b
	callq	purrdf_native::py_store::quad_store::collect_substitutions
.Ltmp41904:
	cmpl	$1, 1584(%rsp)
	leaq	1592(%rsp), %r15
	jne	.LBB3010_7
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
	jg	.LBB3010_54
	jmp	.LBB3010_55
.LBB3010_7:
	vmovups	(%r15), %xmm0
	movq	16(%r15), %rax
	movq	%rax, 144(%rsp)
	vmovaps	%xmm0, 128(%rsp)
.Ltmp41905:
	.cfi_escape 0x2e, 0x00
	movq	2736(%rsp), %rcx
	leaq	1584(%rsp), %rdi
	movq	%rbx, %rsi
	movq	%rbp, %rdx
	callq	purrdf_native::py_store::query::collect_relations
.Ltmp41906:
	cmpl	$1, 1584(%rsp)
	jne	.LBB3010_11
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
.Ltmp42056:
	.cfi_escape 0x2e, 0x00
	leaq	128(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp42057:
	xorl	%ebx, %ebx
	movb	$1, %bpl
	movq	32(%rsp), %rsi
	testq	%rsi, %rsi
	jg	.LBB3010_54
	jmp	.LBB3010_55
.LBB3010_11:
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
.Ltmp41908:
	.cfi_escape 0x2e, 0x00
	movq	2752(%rsp), %rsi
	movq	2760(%rsp), %rdx
	leaq	2472(%rsp), %rdi
	callq	purrdf_native::xpath_regex::selection
.Ltmp41909:
	movzbl	2472(%rsp), %eax
	cmpb	$-1, %al
	je	.LBB3010_25
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
.Ltmp41913:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::SuspendAttach>::new@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp41914:
	movq	%rax, 272(%rsp)
	movq	%rdx, 280(%rsp)
.Ltmp41918:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::ir::mutable::MutableDataset>::freeze@GOTPCREL(%rip), %rax
	leaq	592(%rsp), %rdi
	movq	%r12, %rsi
	callq	*%rax
.Ltmp41919:
	cmpq	$-1, 592(%rsp)
	je	.LBB3010_113
	vmovups	624(%rsp), %zmm1
	vmovups	592(%rsp), %zmm0
	leaq	1584(%rsp), %rax
	movq	%rax, 1248(%rsp)
	movq	<purrdf_core::diagnostic::RdfDiagnostic as core::fmt::Display>::fmt@GOTPCREL(%rip), %rax
	movq	%rax, 1256(%rsp)
	vmovups	%zmm1, 1616(%rsp)
	vmovups	%zmm0, 1584(%rsp)
.Ltmp41920:
	.cfi_escape 0x2e, 0x00
	movq	alloc::fmt::format::format_inner@GOTPCREL(%rip), %rax
	leaq	.Lalloc_0e5f90e3dc675d538220deef7d17e145(%rip), %rsi
	leaq	2112(%rsp), %rdi
	leaq	1248(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp41921:
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
	je	.LBB3010_184
	movq	%rax, %r12
	movq	%rbx, (%rax)
	movq	%r14, 8(%rax)
	movq	%r15, 16(%rax)
.Ltmp41923:
	.cfi_escape 0x2e, 0x00
	leaq	1584(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.Ltmp41924:
	movq	952(%rsp), %rbx
	movq	960(%rsp), %rax
	vxorps	%xmm0, %xmm0, %xmm0
	vmovaps	%xmm0, 160(%rsp)
	testq	%rax, %rax
	je	.LBB3010_23
	movl	$1, %r13d
	movq	%rbx, %r14
	subq	%rax, %r13
	.p2align	4
.LBB3010_21:
.Ltmp41926:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.Ltmp41927:
	incq	%r13
	addq	$160, %r14
	cmpq	$1, %r13
	jne	.LBB3010_21
.LBB3010_23:
	movq	944(%rsp), %rax
	movb	$1, %r13b
	movl	$1, %r15d
	leaq	.Lvtable.22(%rip), %r14
	movl	$3, %ebp
	testq	%rax, %rax
	je	.LBB3010_118
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	callq	*%rax
.LBB3010_118:
.LBB3010_119:
	movq	992(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB3010_122
	testq	%rsi, %rsi
	je	.LBB3010_122
	movq	1000(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBB3010_122:
	movq	$-1, %rbx
	testb	%r13b, %r13b
	je	.LBB3010_124
	.cfi_escape 0x2e, 0x00
	leaq	1016(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
.LBB3010_124:
	movl	(%rsp), %r13d
	leaq	968(%rsp), %rdi
.Ltmp42034:
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp42035:
.Ltmp42040:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::SuspendAttach as core::ops::drop::Drop>::drop@GOTPCREL(%rip), %rax
	leaq	272(%rsp), %rdi
	callq	*%rax
.Ltmp42041:
	vmovaps	160(%rsp), %xmm0
	vmovaps	%xmm0, 1216(%rsp)
	cmpq	$-1, %rbx
	je	.LBB3010_130
	vmovaps	1216(%rsp), %xmm0
	movq	%rbx, 1584(%rsp)
	vmovups	%xmm0, 1592(%rsp)
	movq	%r15, 1608(%rsp)
	movq	%r12, 1616(%rsp)
	movq	%r14, 1624(%rsp)
	movl	%ebp, 1632(%rsp)
	movl	%r13d, 1636(%rsp)
.Ltmp42042:
	.cfi_escape 0x2e, 0x00
	movq	16(%rsp), %rdi
	leaq	1584(%rsp), %rsi
	callq	purrdf_native::py_store::query::materialize_results
.Ltmp42043:
	movq	32(%rsp), %rsi
	testq	%rsi, %rsi
	jle	.LBB3010_83
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	64(%rsp), %rdi
	jmp	.LBB3010_81
.LBB3010_25:
	vmovups	2480(%rsp), %ymm0
	vmovups	2496(%rsp), %ymm1
	movq	16(%rsp), %rax
	movq	56(%rsp), %r12
	vmovups	%ymm1, 24(%rax)
	vmovups	%ymm0, 8(%rax)
	movq	$1, (%rax)
	cmpq	$-1, %rbp
	je	.LBB3010_30
	testq	%rbp, %rbp
	je	.LBB3010_28
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	256(%rsp), %rdi
	movl	$1, %edx
	movq	%rbp, %rsi
	vzeroupper
	callq	*%rax
.LBB3010_28:
	testq	%r13, %r13
	je	.LBB3010_30
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	264(%rsp), %rdi
	movl	$1, %edx
	movq	%r13, %rsi
	vzeroupper
	callq	*%rax
.LBB3010_30:
	movq	72(%rsp), %rbp
	cmpq	$-1, %rbp
	je	.LBB3010_38
	movq	%r12, %r13
	testq	%rbx, %rbx
	je	.LBB3010_36
	movq	104(%rsp), %rax
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r12
	leaq	8(%rax), %r14
	jmp	.LBB3010_34
	.p2align	4
.LBB3010_33:
	addq	$24, %r14
	decq	%rbx
	je	.LBB3010_36
.LBB3010_34:
	movq	-8(%r14), %rsi
	testq	%rsi, %rsi
	je	.LBB3010_33
	movq	(%r14), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r12
	jmp	.LBB3010_33
.LBB3010_36:
	movq	%r13, %r12
	testq	%rbp, %rbp
	je	.LBB3010_38
	shlq	$3, %rbp
	leaq	(%rbp,%rbp,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	104(%rsp), %rdi
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBB3010_38:
	cmpq	$-1, %r12
	je	.LBB3010_46
	testq	%r15, %r15
	je	.LBB3010_44
	movq	48(%rsp), %rax
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r14
	leaq	8(%rax), %rbx
	jmp	.LBB3010_42
	.p2align	4
.LBB3010_41:
	addq	$24, %rbx
	decq	%r15
	je	.LBB3010_44
.LBB3010_42:
	movq	-8(%rbx), %rsi
	testq	%rsi, %rsi
	je	.LBB3010_41
	movq	(%rbx), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r14
	jmp	.LBB3010_41
.LBB3010_44:
	testq	%r12, %r12
	je	.LBB3010_46
	shlq	$3, %r12
	leaq	(%r12,%r12,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	48(%rsp), %rdi
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBB3010_46:
	movq	88(%rsp), %rbx
	movq	96(%rsp), %rax
	testq	%rax, %rax
	je	.LBB3010_50
	movl	$1, %r12d
	movq	%rbx, %r14
	subq	%rax, %r12
	.p2align	4
.LBB3010_48:
.Ltmp42045:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.Ltmp42046:
	incq	%r12
	addq	$160, %r14
	cmpq	$1, %r12
	jne	.LBB3010_48
.LBB3010_50:
	movq	80(%rsp), %rax
	testq	%rax, %rax
	je	.LBB3010_52
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB3010_52:
	movb	$1, %bpl
	xorl	%r13d, %r13d
.Ltmp42054:
	.cfi_escape 0x2e, 0x00
	leaq	128(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp42055:
	movb	$1, %bl
	movq	32(%rsp), %rsi
	testq	%rsi, %rsi
	jle	.LBB3010_55
.LBB3010_54:
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	64(%rsp), %rdi
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LBB3010_55:
	testb	%bl, %bl
	jne	.LBB3010_77
.LBB3010_56:
	movq	312(%rsp), %r14
	cmpq	$-1, %r14
	je	.LBB3010_64
	movq	320(%rsp), %rbx
	movq	328(%rsp), %r15
	testq	%r15, %r15
	je	.LBB3010_62
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r13
	leaq	8(%rbx), %r12
	jmp	.LBB3010_60
	.p2align	4
.LBB3010_59:
	addq	$24, %r12
	decq	%r15
	je	.LBB3010_62
.LBB3010_60:
	movq	-8(%r12), %rsi
	testq	%rsi, %rsi
	je	.LBB3010_59
	movq	(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r13
	jmp	.LBB3010_59
.LBB3010_62:
	testq	%r14, %r14
	je	.LBB3010_64
	shlq	$3, %r14
	leaq	(%r14,%r14,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB3010_64:
	movq	336(%rsp), %r14
	cmpq	$-1, %r14
	je	.LBB3010_72
	movq	344(%rsp), %rbx
	movq	352(%rsp), %r15
	testq	%r15, %r15
	je	.LBB3010_70
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r13
	leaq	8(%rbx), %r12
	jmp	.LBB3010_68
	.p2align	4
.LBB3010_67:
	addq	$24, %r12
	decq	%r15
	je	.LBB3010_70
.LBB3010_68:
	movq	-8(%r12), %rsi
	testq	%rsi, %rsi
	je	.LBB3010_67
	movq	(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r13
	jmp	.LBB3010_67
.LBB3010_70:
	testq	%r14, %r14
	je	.LBB3010_72
	shlq	$3, %r14
	leaq	(%r14,%r14,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB3010_72:
	movq	360(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB3010_77
	testq	%rsi, %rsi
	je	.LBB3010_75
	movq	368(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LBB3010_75:
	movq	384(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB3010_77
	movq	392(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LBB3010_77:
	testb	%bpl, %bpl
	je	.LBB3010_83
	movq	408(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB3010_83
	testq	%rsi, %rsi
	je	.LBB3010_83
	movq	416(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
.LBB3010_81:
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LBB3010_83:
	movq	16(%rsp), %rax
	cmpl	$1, (%rax)
	jne	.LBB3010_112
	leaq	8(%rax), %rbx
.Ltmp42059:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::AttachGuard>::attach@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp42060:
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
.Ltmp42064:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr>::get_type@GOTPCREL(%rip), %rax
	leaq	2112(%rsp), %rdi
	callq	*%rax
.Ltmp42065:
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
	je	.LBB3010_104
.Ltmp42067:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr>::value@GOTPCREL(%rip), %rax
	leaq	2112(%rsp), %rdi
	callq	*%rax
.Ltmp42068:
.Ltmp42070:
	movq	%rax, %r14
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::types::string::PyString>::new@GOTPCREL(%rip), %rbp
	leaq	.Lalloc_016e71ba2f68cc7172262ae988bb360c(%rip), %rdi
	movl	$10, %esi
	callq	*%rbp
.Ltmp42071:
.Ltmp42072:
	movq	%rax, %r12
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::inner@GOTPCREL(%rip), %rax
	leaq	1584(%rsp), %rdi
	movq	%r14, %rsi
	movq	%r12, %rdx
	callq	*%rax
.Ltmp42073:
	leaq	600(%rsp), %r15
	.cfi_escape 0x2e, 0x00
	movq	%r12, %rdi
	callq	*%r13
	cmpb	$1, 1584(%rsp)
	je	.LBB3010_97
	cmpb	$1, 1585(%rsp)
	je	.LBB3010_94
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
.Ltmp42074:
	.cfi_escape 0x2e, 0x00
	leaq	.Lalloc_016e71ba2f68cc7172262ae988bb360c(%rip), %rdx
	leaq	592(%rsp), %rdi
	movl	$10, %ecx
	movq	%r15, %r8
	movq	%r14, %rsi
	movq	%r12, %r15
	callq	<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::setattr::<&str, pyo3::instance::Py<pyo3::types::any::PyAny>>
.Ltmp42075:
	cmpb	$0, 592(%rsp)
	jne	.LBB3010_98
.LBB3010_94:
.Ltmp42076:
	.cfi_escape 0x2e, 0x00
	leaq	.Lalloc_6f6b49b405cc516c15819f32fccb7974(%rip), %rdi
	movl	$12, %esi
	callq	*%rbp
.Ltmp42077:
.Ltmp42078:
	movq	%rax, %r12
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::inner@GOTPCREL(%rip), %rbp
	leaq	1584(%rsp), %rdi
	movq	%r14, %rsi
	movq	%rax, %rdx
	callq	*%rbp
.Ltmp42079:
	.cfi_escape 0x2e, 0x00
	movq	%r12, %rdi
	callq	*%r13
	cmpb	$0, 1584(%rsp)
	je	.LBB3010_107
.LBB3010_97:
	leaq	1592(%rsp), %rax
	vmovups	16(%rax), %ymm1
	vmovups	(%rax), %ymm0
	vmovups	%ymm1, 16(%r15)
	vmovups	%ymm0, (%r15)
.LBB3010_98:
	vmovups	(%r15), %xmm0
	vmovups	624(%rsp), %xmm1
	movq	616(%rsp), %r15
	movq	640(%rsp), %rbp
	cmpq	$0, 2128(%rsp)
	vmovaps	%xmm0, 1584(%rsp)
	vmovaps	%xmm1, 32(%rsp)
	je	.LBB3010_111
	movq	2136(%rsp), %r12
	movq	2144(%rsp), %r14
	testq	%r12, %r12
	je	.LBB3010_105
	movq	(%r14), %rax
	testq	%rax, %rax
	je	.LBB3010_102
.Ltmp42087:
	.cfi_escape 0x2e, 0x00
	movq	%r12, %rdi
	vzeroupper
	callq	*%rax
.Ltmp42088:
.LBB3010_102:
	movq	8(%r14), %rsi
	testq	%rsi, %rsi
	je	.LBB3010_111
	movq	16(%r14), %rdx
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	vzeroupper
	callq	*%rax
	jmp	.LBB3010_111
.LBB3010_104:
	vmovaps	2112(%rsp), %xmm0
	vmovups	2136(%rsp), %xmm1
	movq	2128(%rsp), %r15
	movq	2152(%rsp), %rbp
	vmovaps	%xmm0, 1584(%rsp)
	vmovaps	%xmm1, 32(%rsp)
	jmp	.LBB3010_111
.LBB3010_105:
	.cfi_escape 0x2e, 0x00
	data16
	leaq	pyo3::internal::state::ATTACH_COUNT::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@TLSGD(%rip), %rdi
	data16
	data16
	rex64
	callq	__tls_get_addr@PLT
	cmpq	$0, (%rax)
	jle	.LBB3010_183
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	vzeroupper
	callq	*%r13
	jmp	.LBB3010_111
.LBB3010_107:
	cmpb	$0, 1585(%rsp)
	jne	.LBB3010_110
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
.Ltmp42081:
	.cfi_escape 0x2e, 0x00
	leaq	.Lalloc_6f6b49b405cc516c15819f32fccb7974(%rip), %rdx
	leaq	592(%rsp), %rdi
	movl	$12, %ecx
	movq	%r15, %r8
	movq	%r14, %rsi
	movq	%r12, %r15
	callq	<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::setattr::<&str, pyo3::instance::Py<pyo3::types::any::PyAny>>
.Ltmp42082:
	cmpb	$0, 592(%rsp)
	jne	.LBB3010_98
.LBB3010_110:
	vmovups	2136(%rsp), %xmm0
	vmovaps	2112(%rsp), %xmm1
	movq	2128(%rsp), %r15
	movq	2152(%rsp), %rbp
	vmovaps	%xmm0, 32(%rsp)
	vmovaps	%xmm1, 1584(%rsp)
.LBB3010_111:
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
.LBB3010_112:
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
.LBB3010_113:
	.cfi_def_cfa_offset 2704
	movq	600(%rsp), %rdx
	movb	$1, %r13b
	movq	%rdx, 24(%rsp)
	addq	$16, %rdx
.Ltmp41942:
	.cfi_escape 0x2e, 0x00
	leaq	1584(%rsp), %rdi
	leaq	944(%rsp), %rsi
	callq	purrdf_native::py_store::query::build_relations
.Ltmp41943:
	vmovups	1592(%rsp), %xmm0
	movl	1584(%rsp), %eax
	movq	1608(%rsp), %r15
	movq	1616(%rsp), %r12
	movq	1624(%rsp), %r14
	movl	1632(%rsp), %ebp
	movl	1636(%rsp), %ecx
	vmovaps	%xmm0, 592(%rsp)
	cmpl	$1, %eax
	jne	.LBB3010_131
	vmovaps	592(%rsp), %xmm0
	movl	%ecx, (%rsp)
	movb	$1, %r13b
	vmovaps	%xmm0, 160(%rsp)
.LBB3010_116:
	movq	24(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB3010_119
	#MEMBARRIER
.Ltmp42028:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	24(%rsp), %rdi
	callq	*%rax
.Ltmp42029:
	jmp	.LBB3010_119
.LBB3010_130:
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
	jg	.LBB3010_54
	jmp	.LBB3010_55
.LBB3010_131:
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
.Ltmp41944:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_validate::query::statistical_aggregates@GOTPCREL(%rip), %rax
	leaq	1464(%rsp), %rbx
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp41945:
	cmpq	$-1, 1016(%rsp)
	je	.LBB3010_134
	movq	1032(%rsp), %rdx
	movq	1024(%rsp), %rsi
.Ltmp41946:
	.cfi_escape 0x2e, 0x00
	leaq	592(%rsp), %rdi
	callq	<alloc::vec::Vec<alloc::string::String> as core::clone::Clone>::clone
.Ltmp41947:
	jmp	.LBB3010_135
.LBB3010_134:
	movq	$0, 592(%rsp)
	movq	$8, 600(%rsp)
	movq	$0, 608(%rsp)
.LBB3010_135:
	cmpq	$-1, 1040(%rsp)
	je	.LBB3010_138
	movq	1056(%rsp), %rdx
	movq	1048(%rsp), %rsi
.Ltmp41949:
	.cfi_escape 0x2e, 0x00
	leaq	1584(%rsp), %rdi
	callq	<alloc::vec::Vec<alloc::string::String> as core::clone::Clone>::clone
.Ltmp41950:
	movq	1584(%rsp), %rdx
	movq	1592(%rsp), %rax
	movq	1600(%rsp), %rcx
	jmp	.LBB3010_139
.LBB3010_138:
	movl	$8, %eax
	xorl	%ecx, %ecx
	xorl	%edx, %edx
.LBB3010_139:
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
.Ltmp41952:
	.cfi_escape 0x2e, 0x00
	leaq	1584(%rsp), %rdi
	leaq	1016(%rsp), %rsi
	callq	purrdf_native::py_store::query::build_engine
.Ltmp41953:
	movq	512(%rsp), %rax
	movq	1464(%rsp), %r8
	leaq	512(%rsp), %rdx
	testq	%rax, %rax
	movq	%rax, 56(%rsp)
	movq	%r8, 8(%rsp)
	cmoveq	%rax, %rdx
	testq	%r8, %r8
	cmoveq	%r8, %rbx
.Ltmp41955:
	.cfi_escape 0x2e, 0x00
	leaq	592(%rsp), %rdi
	leaq	1504(%rsp), %rsi
	movq	%rbx, %rcx
	callq	purrdf_native::py_store::env::extension_env
.Ltmp41956:
	vmovups	600(%rsp), %xmm0
	movq	592(%rsp), %rax
	movq	616(%rsp), %r15
	movq	624(%rsp), %r12
	movq	632(%rsp), %r14
	movl	640(%rsp), %ebp
	movl	644(%rsp), %ecx
	vmovaps	%xmm0, 1248(%rsp)
	cmpq	$-1, %rax
	je	.LBB3010_154
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
.Ltmp41957:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::engine::AdmittedSubstitutions>::requested@GOTPCREL(%rip), %rax
	leaq	592(%rsp), %rdi
	movl	$8, %ecx
	movq	%rbx, %rsi
	movq	%r14, %rdx
	xorl	%r8d, %r8d
	vzeroupper
	callq	*%rax
.Ltmp41958:
	leaq	608(%rsp), %rax
.Ltmp41959:
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
.Ltmp41960:
	movq	1248(%rsp), %rax
	movq	1256(%rsp), %rbp
	cmpq	$-1, %rax
	je	.LBB3010_160
	vmovups	1280(%rsp), %zmm1
	vmovups	1264(%rsp), %zmm0
	movq	608(%rsp), %rsi
	vmovups	%zmm1, 192(%rsp)
	vmovups	%zmm0, 176(%rsp)
	movq	%rax, 160(%rsp)
	movq	%rbp, 168(%rsp)
	cmpq	$10, %rsi
	jb	.LBB3010_147
	movq	616(%rsp), %rdi
	shlq	$4, %rsi
	addq	$-16, %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBB3010_147:
	movq	744(%rsp), %rsi
	cmpq	$9, %rsi
	jbe	.LBB3010_149
.LBB3010_148:
	movq	752(%rsp), %rdi
	shlq	$4, %rsi
	addq	$-16, %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBB3010_149:
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
	je	.LBB3010_166
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
.Ltmp41970:
	.cfi_escape 0x2e, 0x00
	movq	alloc::fmt::format::format_inner@GOTPCREL(%rip), %rax
	leaq	.Lalloc_592fabd2ffa0e5a6b6713c88c8b00c99(%rip), %rsi
	leaq	160(%rsp), %rdi
	leaq	1248(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp41971:
	movq	672(%rsp), %rbx
	movq	600(%rsp), %rdi
	movq	608(%rsp), %rsi
.Ltmp41972:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_validate::xpath_regex::diagnostic_refusal_code@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp41973:
	testq	%rbx, %rbx
	movq	%rdx, %rcx
	setne	%dl
	testq	%rax, %rax
	sete	%sil
	orb	%dl, %sil
	cmpb	$1, %sil
	jne	.LBB3010_167
.Ltmp41977:
	.cfi_escape 0x2e, 0x00
	leaq	1248(%rsp), %rdi
	leaq	160(%rsp), %rsi
	movq	%rbx, %rdx
	callq	purrdf_native::py_store::presentation::presented_value_error
.Ltmp41978:
	jmp	.LBB3010_168
.LBB3010_154:
	vmovaps	1248(%rsp), %xmm0
	movl	%ecx, (%rsp)
	vmovaps	%xmm0, 160(%rsp)
.Ltmp42007:
	.cfi_escape 0x2e, 0x00
	leaq	1584(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
.Ltmp42008:
	cmpq	$0, 8(%rsp)
	je	.LBB3010_157
	xorl	%r13d, %r13d
.Ltmp42012:
	.cfi_escape 0x2e, 0x00
	leaq	1464(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
.Ltmp42013:
.LBB3010_157:
	cmpq	$0, 56(%rsp)
	je	.LBB3010_182
.Ltmp42017:
	.cfi_escape 0x2e, 0x00
	leaq	512(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::property_fn::PropertyFunction>)>>
.Ltmp42018:
	xorl	%r13d, %r13d
.Ltmp42023:
	.cfi_escape 0x2e, 0x00
	leaq	544(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
.Ltmp42024:
	jmp	.LBB3010_116
.LBB3010_160:
	vmovups	.Lanon.4895d20928f7255621bf668e37519d0a.4+8(%rip), %zmm0
	vmovups	.Lanon.4895d20928f7255621bf668e37519d0a.4+24(%rip), %zmm2
	vmovups	.Lanon.4895d20928f7255621bf668e37519d0a.4+112(%rip), %zmm1
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
.Ltmp41961:
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
.Ltmp41962:
	lock		decq	(%rbp)
	jne	.LBB3010_163
	#MEMBARRIER
.Ltmp41967:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::engine::PreparedQuery>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1424(%rsp), %rdi
	callq	*%rax
.Ltmp41968:
.LBB3010_163:
	movq	608(%rsp), %rsi
	cmpq	$10, %rsi
	jb	.LBB3010_165
	movq	616(%rsp), %rdi
	shlq	$4, %rsi
	addq	$-16, %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	callq	*%rax
.LBB3010_165:
	movq	744(%rsp), %rsi
	cmpq	$10, %rsi
	jae	.LBB3010_148
	jmp	.LBB3010_149
.LBB3010_166:
	vmovaps	1232(%rsp), %xmm0
	jmp	.LBB3010_170
.LBB3010_167:
.Ltmp41975:
	.cfi_escape 0x2e, 0x00
	leaq	1248(%rsp), %rdi
	leaq	160(%rsp), %rsi
	movq	%rax, %rdx
	callq	purrdf_native::py_store::presentation::refusal_value_error
.Ltmp41976:
.LBB3010_168:
.Ltmp41983:
	.cfi_escape 0x2e, 0x00
	leaq	592(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.Ltmp41984:
	vmovups	1248(%rsp), %xmm0
	movq	1264(%rsp), %r15
	movq	1272(%rsp), %r12
	movq	1280(%rsp), %r14
	movl	1288(%rsp), %ebp
	movl	1292(%rsp), %r13d
	movq	$-1, %rbx
.LBB3010_170:
	vmovaps	%xmm0, 160(%rsp)
.Ltmp41988:
	.cfi_escape 0x2e, 0x00
	leaq	2112(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::extension_env::ExtensionEnv>
.Ltmp41989:
.Ltmp41993:
	movl	%r13d, (%rsp)
	.cfi_escape 0x2e, 0x00
	leaq	1584(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
.Ltmp41994:
	cmpq	$0, 8(%rsp)
	je	.LBB3010_175
	xorl	%r13d, %r13d
.Ltmp41995:
	.cfi_escape 0x2e, 0x00
	leaq	1464(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
.Ltmp41996:
.LBB3010_175:
	cmpq	$0, 56(%rsp)
	je	.LBB3010_178
.Ltmp41997:
	.cfi_escape 0x2e, 0x00
	leaq	512(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::property_fn::PropertyFunction>)>>
.Ltmp41998:
	xorl	%r13d, %r13d
.Ltmp42003:
	.cfi_escape 0x2e, 0x00
	leaq	544(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
.Ltmp42004:
.LBB3010_178:
	movq	24(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB3010_180
	xorl	%r13d, %r13d
	#MEMBARRIER
.Ltmp42005:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	24(%rsp), %rdi
	callq	*%rax
.Ltmp42006:
.LBB3010_180:
	movq	72(%rsp), %rsi
	testq	%rsi, %rsi
	jle	.LBB3010_124
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	48(%rsp), %rdi
	movl	$1, %edx
	callq	*%rax
	jmp	.LBB3010_124
.LBB3010_182:
	xorl	%r13d, %r13d
	jmp	.LBB3010_116
.LBB3010_183:
.Ltmp42090:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::instance::Py<_> as core::ops::drop::Drop>::drop::drop_slow@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp42091:
	jmp	.LBB3010_111
.LBB3010_184:
.Ltmp41934:
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$24, %esi
	callq	*%rax
.Ltmp41935:
	ud2
.LBB3010_186:
.Ltmp42092:
	movq	%rax, %r15
	jmp	.LBB3010_239
.LBB3010_187:
.Ltmp41963:
	lock		decq	(%rbp)
	movq	%rax, %r15
	jne	.LBB3010_202
	#MEMBARRIER
.Ltmp41964:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::engine::PreparedQuery>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1424(%rsp), %rdi
	callq	*%rax
.Ltmp41965:
	jmp	.LBB3010_202
.LBB3010_189:
.Ltmp41966:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3010_190:
.Ltmp42019:
	movq	%rax, %r15
.Ltmp42020:
	.cfi_escape 0x2e, 0x00
	leaq	544(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
.Ltmp42021:
	jmp	.LBB3010_194
.LBB3010_192:
.Ltmp42022:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3010_193:
.Ltmp41999:
	movq	%rax, %r15
.Ltmp42000:
	.cfi_escape 0x2e, 0x00
	leaq	544(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
.Ltmp42001:
.LBB3010_194:
	xorl	%r13d, %r13d
	jmp	.LBB3010_219
.LBB3010_195:
.Ltmp42002:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3010_196:
.Ltmp41974:
	movq	160(%rsp), %rsi
	movq	%rax, %r15
	testq	%rsi, %rsi
	je	.LBB3010_204
	movq	168(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
	jmp	.LBB3010_204
.LBB3010_198:
.Ltmp41951:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	leaq	592(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::string::String>>
	jmp	.LBB3010_200
.LBB3010_199:
.Ltmp41948:
	movq	%rax, %r15
.LBB3010_200:
	movb	$1, %r13b
	jmp	.LBB3010_214
.LBB3010_201:
.Ltmp41969:
	movq	%rax, %r15
.LBB3010_202:
	.cfi_escape 0x2e, 0x00
	leaq	592(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::AdmittedSubstitutions>
	jmp	.LBB3010_210
.LBB3010_203:
.Ltmp41979:
	movq	%rax, %r15
.LBB3010_204:
.Ltmp41980:
	.cfi_escape 0x2e, 0x00
	leaq	592(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.Ltmp41981:
	jmp	.LBB3010_210
.LBB3010_205:
.Ltmp41982:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3010_206:
.Ltmp42009:
	movq	%rax, %r15
	jmp	.LBB3010_213
.LBB3010_207:
.Ltmp42030:
	movq	%rax, %r15
	jmp	.LBB3010_270
.LBB3010_208:
.Ltmp41954:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	leaq	1504(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_algebra::parser::ParserOptions>
	jmp	.LBB3010_213
.LBB3010_209:
.Ltmp41985:
	movq	%rax, %r15
.LBB3010_210:
.Ltmp41986:
	.cfi_escape 0x2e, 0x00
	leaq	2112(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::extension_env::ExtensionEnv>
.Ltmp41987:
	jmp	.LBB3010_212
.LBB3010_211:
.Ltmp41990:
	movq	%rax, %r15
.LBB3010_212:
.Ltmp41991:
	.cfi_escape 0x2e, 0x00
	leaq	1584(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
.Ltmp41992:
.LBB3010_213:
	xorl	%r13d, %r13d
.LBB3010_214:
	cmpq	$0, 1464(%rsp)
	je	.LBB3010_217
.Ltmp42010:
	.cfi_escape 0x2e, 0x00
	leaq	1464(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
.Ltmp42011:
	jmp	.LBB3010_217
.LBB3010_216:
.Ltmp42014:
	movq	%rax, %r15
.LBB3010_217:
.Ltmp42015:
	.cfi_escape 0x2e, 0x00
	leaq	512(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry>>
.Ltmp42016:
	jmp	.LBB3010_219
.LBB3010_218:
.Ltmp42025:
	movq	%rax, %r15
.LBB3010_219:
	movq	24(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB3010_270
	#MEMBARRIER
.Ltmp42026:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	24(%rsp), %rdi
	callq	*%rax
.Ltmp42027:
	jmp	.LBB3010_270
.LBB3010_221:
.Ltmp41922:
	movq	%rax, %r15
	jmp	.LBB3010_268
.LBB3010_222:
.Ltmp42089:
	movq	8(%r14), %rsi
	movq	%rax, %r15
	testq	%rsi, %rsi
	je	.LBB3010_239
	movq	16(%r14), %rdx
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	callq	*%rax
	jmp	.LBB3010_239
.LBB3010_224:
.Ltmp42036:
	movq	%rax, %r15
	jmp	.LBB3010_276
.LBB3010_225:
.Ltmp41915:
	movq	%rax, %r15
.Ltmp41916:
	.cfi_escape 0x2e, 0x00
	leaq	944(%rsp), %rdi
	callq	core::ptr::drop_glue::<<purrdf_native::py_store::quad_store::PyQuadStore>::query_impl<()>::{closure#0}::{closure#0}>
.Ltmp41917:
	jmp	.LBB3010_229
.LBB3010_226:
.Ltmp41910:
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
.Ltmp41911:
	.cfi_escape 0x2e, 0x00
	leaq	80(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>
.Ltmp41912:
	jmp	.LBB3010_250
.LBB3010_228:
.Ltmp42044:
	movq	%rax, %r15
.LBB3010_229:
	movl	$0, 8(%rsp)
	xorl	%r13d, %r13d
	jmp	.LBB3010_277
.LBB3010_230:
.Ltmp41925:
	movq	%rax, %r15
	jmp	.LBB3010_269
.LBB3010_231:
.Ltmp41907:
	movb	$1, %r13b
	movq	%rax, %r15
	jmp	.LBB3010_251
.LBB3010_232:
.Ltmp42069:
	jmp	.LBB3010_237
.LBB3010_233:
.Ltmp42080:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	movq	%r12, %rdi
	callq	*%r13
	jmp	.LBB3010_238
.LBB3010_234:
.Ltmp42066:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	movq	_Py_DecRef@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
	jmp	.LBB3010_238
.LBB3010_235:
.Ltmp42061:
	movq	%rax, %r15
.Ltmp42062:
	.cfi_escape 0x2e, 0x00
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<pyo3::err::PyErr>
.Ltmp42063:
	jmp	.LBB3010_285
.LBB3010_236:
.Ltmp42083:
.LBB3010_237:
	movq	%rax, %r15
.LBB3010_238:
.Ltmp42084:
	.cfi_escape 0x2e, 0x00
	leaq	2112(%rsp), %rdi
	callq	core::ptr::drop_glue::<pyo3::err::PyErr>
.Ltmp42085:
.LBB3010_239:
.Ltmp42093:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::AttachGuard as core::ops::drop::Drop>::drop@GOTPCREL(%rip), %rax
	leaq	944(%rsp), %rdi
	callq	*%rax
.Ltmp42094:
	jmp	.LBB3010_285
.LBB3010_240:
.Ltmp42058:
	movq	%rax, %r15
	jmp	.LBB3010_277
.LBB3010_241:
.Ltmp42095:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3010_242:
.Ltmp42086:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3010_243:
.Ltmp42047:
	movq	%rax, %r15
	testq	%r12, %r12
	je	.LBB3010_247
	negq	%r12
	addq	$160, %r14
	.p2align	4
.LBB3010_245:
.Ltmp42048:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.Ltmp42049:
	addq	$160, %r14
	decq	%r12
	jne	.LBB3010_245
.LBB3010_247:
	movq	80(%rsp), %rax
	testq	%rax, %rax
	je	.LBB3010_250
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	callq	*%rax
.LBB3010_250:
	xorl	%r13d, %r13d
.LBB3010_251:
.Ltmp42051:
	.cfi_escape 0x2e, 0x00
	leaq	128(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp42052:
	jmp	.LBB3010_277
.LBB3010_252:
.Ltmp42050:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3010_253:
.Ltmp42053:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3010_254:
.Ltmp41928:
	movq	%rax, %r15
	testq	%r13, %r13
	je	.LBB3010_258
	negq	%r13
	addq	$160, %r14
.LBB3010_256:
.Ltmp41929:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.Ltmp41930:
	addq	$160, %r14
	decq	%r13
	jne	.LBB3010_256
.LBB3010_258:
	movq	944(%rsp), %rax
	testq	%rax, %rax
	je	.LBB3010_260
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	callq	*%rax
.LBB3010_260:
	movq	992(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB3010_263
	testq	%rsi, %rsi
	je	.LBB3010_263
	movq	1000(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBB3010_263:
	leaq	1016(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
.Ltmp41932:
	.cfi_escape 0x2e, 0x00
	leaq	968(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp41933:
	jmp	.LBB3010_276
.LBB3010_264:
.Ltmp41931:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3010_265:
.Ltmp41902:
	movq	%rax, %r15
	movb	$1, %al
	movb	$1, %r13b
	movl	%eax, 8(%rsp)
	jmp	.LBB3010_277
.LBB3010_266:
.Ltmp41936:
	movq	%rax, %r15
	testq	%rbx, %rbx
	je	.LBB3010_268
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	movq	%r14, %rdi
	movq	%rbx, %rsi
	callq	*%rax
.LBB3010_268:
.Ltmp41937:
	.cfi_escape 0x2e, 0x00
	leaq	1584(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.Ltmp41938:
.LBB3010_269:
	movb	$1, %r13b
.Ltmp41940:
	.cfi_escape 0x2e, 0x00
	leaq	944(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>
.Ltmp41941:
.LBB3010_270:
	movq	992(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB3010_273
	testq	%rsi, %rsi
	je	.LBB3010_273
	movq	1000(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBB3010_273:
	testb	%r13b, %r13b
	je	.LBB3010_275
	leaq	1016(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
.LBB3010_275:
.Ltmp42031:
	.cfi_escape 0x2e, 0x00
	leaq	968(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp42032:
.LBB3010_276:
	movl	$0, 8(%rsp)
.Ltmp42037:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::SuspendAttach as core::ops::drop::Drop>::drop@GOTPCREL(%rip), %rax
	leaq	272(%rsp), %rdi
	callq	*%rax
.Ltmp42038:
	xorl	%r13d, %r13d
.LBB3010_277:
	cmpq	$0, 32(%rsp)
	jle	.LBB3010_279
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	64(%rsp), %rdi
	movq	32(%rsp), %rsi
	movl	$1, %edx
	callq	*%rax
.LBB3010_279:
	testb	%r13b, %r13b
	je	.LBB3010_281
	leaq	312(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	leaq	336(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	leaq	360(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
.LBB3010_281:
	cmpb	$0, 8(%rsp)
	je	.LBB3010_285
	movq	408(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB3010_285
	testq	%rsi, %rsi
	je	.LBB3010_285
	movq	416(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBB3010_285:
	.cfi_escape 0x2e, 0x00
	movq	%r15, %rdi
	callq	_Unwind_Resume@PLT
.LBB3010_286:
.Ltmp42039:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3010_287:
.Ltmp42033:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3010_288:
.Ltmp41939:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end3010:
