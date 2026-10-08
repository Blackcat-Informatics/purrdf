<purrdf_native::py_store::quad_store::PyQuadStore>::query:
.Lfunc_begin1855:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception1855
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
	subq	$2760, %rsp
	.cfi_def_cfa_offset 2816
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	2880(%rsp), %rax
	movq	%rsi, %rbp
	movq	%r9, %r14
	vmovups	(%r14), %xmm1
	movq	%r8, %r15
	movq	2824(%rsp), %r8
	movq	2848(%rsp), %r11
	movq	2840(%rsp), %r13
	movq	2832(%rsp), %rbx
	movq	%rdi, 16(%rsp)
	movq	2856(%rsp), %rdi
	movq	2872(%rsp), %r9
	movq	2864(%rsp), %r10
	movq	%rdx, 40(%rsp)
	movq	%rcx, 8(%rsp)
	movq	16(%rax), %rsi
	vmovups	(%rax), %xmm0
	movq	16(%r14), %rax
	vmovups	(%r8), %ymm2
	movq	%rsi, 320(%rsp)
	movq	2816(%rsp), %rsi
	vmovaps	%xmm0, 304(%rsp)
	movq	%r15, 472(%rsp)
	movq	%rbx, 480(%rsp)
	movq	%r13, 488(%rsp)
	movq	%r11, 496(%rsp)
	vmovups	%xmm1, 328(%rsp)
	vmovups	16(%r8), %ymm1
	movq	%rax, 344(%rsp)
	vmovups	(%rsi), %xmm3
	movq	16(%rsi), %rax
	xorl	%esi, %esi
	movq	%rax, 368(%rsp)
	movq	16(%rdi), %rax
	vmovaps	%xmm3, 352(%rsp)
	vmovups	%ymm2, 376(%rsp)
	vmovups	%ymm1, 392(%rsp)
	vmovups	(%rdi), %xmm1
	movq	%r10, 504(%rsp)
	movq	%r9, 512(%rsp)
	movq	%rbp, 448(%rsp)
	movq	%rax, 440(%rsp)
	vmovups	%xmm1, 424(%rsp)
	movq	%rdx, 456(%rsp)
	movq	%rcx, 464(%rsp)
	movq	304(%rsp), %rax
	movq	312(%rsp), %rcx
	movq	320(%rsp), %rdx
	cmpq	$-1, %rax
	movq	%rax, 48(%rsp)
	movq	%rcx, 64(%rsp)
	cmovneq	%rcx, %rsi
.Ltmp42203:
	.cfi_escape 0x2e, 0x00
	leaq	1696(%rsp), %rdi
	vzeroupper
	callq	purrdf_native::py_store::env::division_policy
.Ltmp42204:
	cmpl	$1, 1696(%rsp)
	jne	.LBB3021_4
	vmovups	1720(%rsp), %ymm0
	vmovups	1708(%rsp), %ymm1
	movq	16(%rsp), %rcx
	movl	1704(%rsp), %eax
	movq	48(%rsp), %rsi
	movb	$1, %bpl
	vmovups	%ymm0, 24(%rcx)
	vmovups	%ymm1, 12(%rcx)
	movl	%eax, 8(%rcx)
	movq	$1, (%rcx)
	testq	%rsi, %rsi
	jle	.LBB3021_56
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	64(%rsp), %rdi
	movl	$1, %edx
	vzeroupper
	callq	*%rax
	jmp	.LBB3021_56
.LBB3021_4:
	movq	1700(%rsp), %rax
	movq	%rax, 152(%rsp)
	movb	$1, %al
	movl	%eax, 24(%rsp)
.Ltmp42206:
	.cfi_escape 0x2e, 0x00
	leaq	1696(%rsp), %rdi
	movq	%r15, %rsi
	movb	$1, %r12b
	callq	purrdf_native::py_store::quad_store::collect_substitutions
.Ltmp42207:
	cmpl	$1, 1696(%rsp)
	leaq	1704(%rsp), %r15
	jne	.LBB3021_7
	vmovups	16(%r15), %ymm1
	vmovups	(%r15), %ymm0
	movq	16(%rsp), %rax
	movb	$1, %bpl
	xorl	%ebx, %ebx
	vmovups	%ymm1, 544(%rsp)
	vmovups	%ymm0, 528(%rsp)
	vmovups	544(%rsp), %ymm1
	vmovups	528(%rsp), %ymm0
	vmovups	%ymm1, 24(%rax)
	vmovups	%ymm0, 8(%rax)
	movq	$1, (%rax)
	movq	48(%rsp), %rsi
	testq	%rsi, %rsi
	jg	.LBB3021_54
	jmp	.LBB3021_55
.LBB3021_7:
	vmovups	(%r15), %xmm0
	movq	16(%r15), %rax
	movq	%rax, 176(%rsp)
	vmovaps	%xmm0, 160(%rsp)
.Ltmp42208:
	.cfi_escape 0x2e, 0x00
	movq	2848(%rsp), %rcx
	leaq	1696(%rsp), %rdi
	movq	%rbx, %rsi
	movq	%r13, %rdx
	callq	purrdf_native::py_store::query::collect_relations
.Ltmp42209:
	cmpl	$1, 1696(%rsp)
	jne	.LBB3021_11
	vmovups	16(%r15), %ymm1
	vmovups	(%r15), %ymm0
	movq	16(%rsp), %rax
	movb	$1, %r12b
	vmovups	%ymm1, 544(%rsp)
	vmovups	%ymm0, 528(%rsp)
	vmovups	544(%rsp), %ymm1
	vmovups	528(%rsp), %ymm0
	vmovups	%ymm1, 24(%rax)
	vmovups	%ymm0, 8(%rax)
	movq	$1, (%rax)
.Ltmp42402:
	.cfi_escape 0x2e, 0x00
	leaq	160(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp42403:
	xorl	%ebx, %ebx
	movb	$1, %bpl
	movq	48(%rsp), %rsi
	testq	%rsi, %rsi
	jg	.LBB3021_54
	jmp	.LBB3021_55
.LBB3021_11:
	movq	328(%rsp), %r8
	movq	336(%rsp), %rcx
	vmovups	(%r15), %xmm0
	movq	16(%r15), %rax
	movq	344(%rsp), %r15
	movq	368(%rsp), %rbx
	movq	376(%rsp), %r13
	movq	400(%rsp), %r12
	movq	%r8, 72(%rsp)
	movq	%rcx, 144(%rsp)
	movq	352(%rsp), %r8
	movq	360(%rsp), %rcx
	movq	%rax, 96(%rsp)
	vmovaps	%xmm0, 80(%rsp)
	movq	%r8, 280(%rsp)
	movq	%rcx, 136(%rsp)
	movq	384(%rsp), %r8
	movq	408(%rsp), %rcx
	movq	%r8, 264(%rsp)
	movq	%rcx, 272(%rsp)
.Ltmp42211:
	.cfi_escape 0x2e, 0x00
	movq	2864(%rsp), %rsi
	movq	2872(%rsp), %rdx
	leaq	2584(%rsp), %rdi
	callq	purrdf_native::xpath_regex::selection
.Ltmp42212:
	movzbl	2584(%rsp), %eax
	cmpb	$-1, %al
	je	.LBB3021_25
	vmovups	2640(%rsp), %xmm0
	vmovups	2608(%rsp), %ymm1
	movq	16(%r14), %rcx
	movq	2816(%rsp), %rdx
	vmovups	2585(%rsp), %ymm5
	vmovaps	80(%rsp), %xmm2
	movq	%rcx, 2672(%rsp)
	movq	16(%rdx), %rcx
	vmovups	(%rdx), %xmm4
	movq	2856(%rsp), %rdx
	vmovaps	%xmm0, 1216(%rsp)
	vmovups	%ymm1, 1184(%rsp)
	vmovups	(%r14), %xmm1
	vmovups	%ymm5, 1161(%rsp)
	movq	%rbp, 1240(%rsp)
	vmovaps	%xmm2, 992(%rsp)
	vmovaps	%xmm1, 2656(%rsp)
	movq	%rcx, 2696(%rsp)
	movq	2824(%rsp), %rcx
	vmovups	%xmm4, 2680(%rsp)
	vmovups	(%rcx), %ymm3
	vmovups	16(%rcx), %ymm1
	movq	96(%rsp), %rcx
	movq	%rcx, 1008(%rsp)
	movq	16(%rdx), %rcx
	vmovups	%ymm3, 2704(%rsp)
	vmovups	%ymm1, 2720(%rsp)
	vmovups	(%rdx), %xmm1
	movq	40(%rsp), %rdx
	vmovups	2656(%rsp), %zmm0
	movq	%rcx, 1056(%rsp)
	movq	152(%rsp), %rcx
	vmovaps	%xmm1, 1040(%rsp)
	vmovups	2688(%rsp), %zmm1
	vmovups	%zmm1, 1096(%rsp)
	vmovups	%zmm0, 1064(%rsp)
	vmovaps	160(%rsp), %xmm0
	movb	%al, 1160(%rsp)
	movq	%rcx, 1232(%rsp)
	movq	176(%rsp), %rax
	movq	8(%rsp), %rcx
	movq	%rdx, 1248(%rsp)
	movq	%rcx, 1256(%rsp)
	movq	%rax, 1032(%rsp)
	vmovups	%xmm0, 1016(%rsp)
.Ltmp42216:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::SuspendAttach>::new@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp42217:
	movq	%rax, 288(%rsp)
	movq	%rdx, 296(%rsp)
.Ltmp42221:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::ir::mutable::MutableDataset>::freeze@GOTPCREL(%rip), %rax
	leaq	528(%rsp), %rdi
	movq	%rbp, %rsi
	callq	*%rax
.Ltmp42222:
	cmpq	$-1, 528(%rsp)
	je	.LBB3021_113
	vmovups	560(%rsp), %zmm1
	vmovups	528(%rsp), %zmm0
	leaq	1696(%rsp), %rax
	movq	%rax, 1392(%rsp)
	movq	<purrdf_core::diagnostic::RdfDiagnostic as core::fmt::Display>::fmt@GOTPCREL(%rip), %rax
	movq	%rax, 1400(%rsp)
	vmovups	%zmm1, 1728(%rsp)
	vmovups	%zmm0, 1696(%rsp)
.Ltmp42223:
	.cfi_escape 0x2e, 0x00
	movq	alloc::fmt::format::format_inner@GOTPCREL(%rip), %rax
	leaq	.Lalloc_0e5f90e3dc675d538220deef7d17e145(%rip), %rsi
	leaq	2224(%rsp), %rdi
	leaq	1392(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp42224:
	movq	2224(%rsp), %rbx
	movq	2232(%rsp), %r14
	movq	2240(%rsp), %r15
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_no_alloc_shim_is_unstable_v2@GOTPCREL(%rip), %rax
	callq	*%rax
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_alloc@GOTPCREL(%rip), %rax
	movl	$24, %edi
	movl	$8, %esi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB3021_210
	movq	%rax, %rbp
	movq	%rbx, (%rax)
	movq	%r14, 8(%rax)
	movq	%r15, 16(%rax)
.Ltmp42226:
	.cfi_escape 0x2e, 0x00
	leaq	1696(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.Ltmp42227:
	movq	1000(%rsp), %rbx
	movq	1008(%rsp), %rax
	vxorps	%xmm0, %xmm0, %xmm0
	vmovaps	%xmm0, 880(%rsp)
	testq	%rax, %rax
	je	.LBB3021_23
	movl	$1, %r12d
	movq	%rbx, %r14
	subq	%rax, %r12
	.p2align	4
.LBB3021_21:
.Ltmp42229:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.Ltmp42230:
	incq	%r12
	addq	$160, %r14
	cmpq	$1, %r12
	jne	.LBB3021_21
.LBB3021_23:
	movq	992(%rsp), %rax
	movb	$1, %r13b
	movl	$1, %r14d
	leaq	.Lvtable.22(%rip), %r12
	movl	$3, %r15d
	testq	%rax, %rax
	je	.LBB3021_118
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	callq	*%rax
.LBB3021_118:
.LBB3021_119:
	movq	1040(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB3021_122
	testq	%rsi, %rsi
	je	.LBB3021_122
	movq	1048(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBB3021_122:
	movq	$-1, %rbx
	testb	%r13b, %r13b
	je	.LBB3021_124
	.cfi_escape 0x2e, 0x00
	leaq	1064(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
.LBB3021_124:
	movq	%r14, %r13
.LBB3021_125:
.Ltmp42380:
	.cfi_escape 0x2e, 0x00
	leaq	1016(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp42381:
.Ltmp42386:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::SuspendAttach as core::ops::drop::Drop>::drop@GOTPCREL(%rip), %rax
	leaq	288(%rsp), %rdi
	callq	*%rax
.Ltmp42387:
	vmovaps	880(%rsp), %xmm0
	vmovaps	%xmm0, 976(%rsp)
	cmpq	$-1, %rbx
	je	.LBB3021_131
	vmovaps	976(%rsp), %xmm0
	movl	8(%rsp), %eax
	movq	%rbx, 1696(%rsp)
	vmovups	%xmm0, 1704(%rsp)
	movq	%r13, 1720(%rsp)
	movq	%rbp, 1728(%rsp)
	movq	%r12, 1736(%rsp)
	movl	%r15d, 1744(%rsp)
	movl	%eax, 1748(%rsp)
.Ltmp42388:
	.cfi_escape 0x2e, 0x00
	movq	16(%rsp), %rdi
	leaq	1696(%rsp), %rsi
	callq	purrdf_native::py_store::query::materialize_results
.Ltmp42389:
	movq	48(%rsp), %rsi
	testq	%rsi, %rsi
	jle	.LBB3021_83
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	64(%rsp), %rdi
	jmp	.LBB3021_81
.LBB3021_25:
	vmovups	2592(%rsp), %ymm0
	vmovups	2608(%rsp), %ymm1
	movq	16(%rsp), %rax
	movq	280(%rsp), %rbp
	vmovups	%ymm1, 24(%rax)
	vmovups	%ymm0, 8(%rax)
	movq	$1, (%rax)
	cmpq	$-1, %r13
	je	.LBB3021_30
	testq	%r13, %r13
	je	.LBB3021_28
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	264(%rsp), %rdi
	movl	$1, %edx
	movq	%r13, %rsi
	vzeroupper
	callq	*%rax
.LBB3021_28:
	testq	%r12, %r12
	je	.LBB3021_30
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	272(%rsp), %rdi
	movl	$1, %edx
	movq	%r12, %rsi
	vzeroupper
	callq	*%rax
.LBB3021_30:
	movq	72(%rsp), %r13
	cmpq	$-1, %rbp
	je	.LBB3021_38
	testq	%rbx, %rbx
	je	.LBB3021_36
	movq	136(%rsp), %rax
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r12
	leaq	8(%rax), %r14
	jmp	.LBB3021_34
	.p2align	4
.LBB3021_33:
	addq	$24, %r14
	decq	%rbx
	je	.LBB3021_36
.LBB3021_34:
	movq	-8(%r14), %rsi
	testq	%rsi, %rsi
	je	.LBB3021_33
	movq	(%r14), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r12
	jmp	.LBB3021_33
.LBB3021_36:
	testq	%rbp, %rbp
	je	.LBB3021_38
	shlq	$3, %rbp
	leaq	(%rbp,%rbp,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	136(%rsp), %rdi
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBB3021_38:
	cmpq	$-1, %r13
	je	.LBB3021_46
	testq	%r15, %r15
	je	.LBB3021_44
	movq	144(%rsp), %rax
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r14
	leaq	8(%rax), %rbx
	jmp	.LBB3021_42
	.p2align	4
.LBB3021_41:
	addq	$24, %rbx
	decq	%r15
	je	.LBB3021_44
.LBB3021_42:
	movq	-8(%rbx), %rsi
	testq	%rsi, %rsi
	je	.LBB3021_41
	movq	(%rbx), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r14
	jmp	.LBB3021_41
.LBB3021_44:
	testq	%r13, %r13
	je	.LBB3021_46
	shlq	$3, %r13
	leaq	(%r13,%r13,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	144(%rsp), %rdi
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBB3021_46:
	movq	88(%rsp), %rbx
	movq	96(%rsp), %rax
	testq	%rax, %rax
	je	.LBB3021_50
	movl	$1, %r12d
	movq	%rbx, %r14
	subq	%rax, %r12
	.p2align	4
.LBB3021_48:
.Ltmp42391:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.Ltmp42392:
	incq	%r12
	addq	$160, %r14
	cmpq	$1, %r12
	jne	.LBB3021_48
.LBB3021_50:
	movq	80(%rsp), %rax
	testq	%rax, %rax
	je	.LBB3021_52
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB3021_52:
	movb	$1, %bpl
	xorl	%r12d, %r12d
.Ltmp42400:
	.cfi_escape 0x2e, 0x00
	leaq	160(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp42401:
	movb	$1, %bl
	movq	48(%rsp), %rsi
	testq	%rsi, %rsi
	jle	.LBB3021_55
.LBB3021_54:
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	64(%rsp), %rdi
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LBB3021_55:
	testb	%bl, %bl
	jne	.LBB3021_77
.LBB3021_56:
	movq	328(%rsp), %r14
	cmpq	$-1, %r14
	je	.LBB3021_64
	movq	336(%rsp), %rbx
	movq	344(%rsp), %r15
	testq	%r15, %r15
	je	.LBB3021_62
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r13
	leaq	8(%rbx), %r12
	jmp	.LBB3021_60
	.p2align	4
.LBB3021_59:
	addq	$24, %r12
	decq	%r15
	je	.LBB3021_62
.LBB3021_60:
	movq	-8(%r12), %rsi
	testq	%rsi, %rsi
	je	.LBB3021_59
	movq	(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r13
	jmp	.LBB3021_59
.LBB3021_62:
	testq	%r14, %r14
	je	.LBB3021_64
	shlq	$3, %r14
	leaq	(%r14,%r14,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB3021_64:
	movq	352(%rsp), %r14
	cmpq	$-1, %r14
	je	.LBB3021_72
	movq	360(%rsp), %rbx
	movq	368(%rsp), %r15
	testq	%r15, %r15
	je	.LBB3021_70
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r13
	leaq	8(%rbx), %r12
	jmp	.LBB3021_68
	.p2align	4
.LBB3021_67:
	addq	$24, %r12
	decq	%r15
	je	.LBB3021_70
.LBB3021_68:
	movq	-8(%r12), %rsi
	testq	%rsi, %rsi
	je	.LBB3021_67
	movq	(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r13
	jmp	.LBB3021_67
.LBB3021_70:
	testq	%r14, %r14
	je	.LBB3021_72
	shlq	$3, %r14
	leaq	(%r14,%r14,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB3021_72:
	movq	376(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB3021_77
	testq	%rsi, %rsi
	je	.LBB3021_75
	movq	384(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LBB3021_75:
	movq	400(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB3021_77
	movq	408(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LBB3021_77:
	testb	%bpl, %bpl
	je	.LBB3021_83
	movq	424(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB3021_83
	testq	%rsi, %rsi
	je	.LBB3021_83
	movq	432(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
.LBB3021_81:
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LBB3021_83:
	movq	16(%rsp), %rax
	cmpl	$1, (%rax)
	jne	.LBB3021_112
	leaq	8(%rax), %rbx
.Ltmp42405:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::AttachGuard>::attach@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp42406:
	movl	%eax, 992(%rsp)
	movq	PyExc_ValueError@GOTPCREL(%rip), %rax
	vmovups	16(%rbx), %ymm1
	vmovups	(%rbx), %ymm0
	movq	(%rax), %r14
	vmovups	%ymm1, 2240(%rsp)
	vmovups	%ymm0, 2224(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp42410:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr>::get_type@GOTPCREL(%rip), %rax
	leaq	2224(%rsp), %rdi
	callq	*%rax
.Ltmp42411:
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
	je	.LBB3021_104
.Ltmp42413:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr>::value@GOTPCREL(%rip), %rax
	leaq	2224(%rsp), %rdi
	callq	*%rax
.Ltmp42414:
.Ltmp42416:
	movq	%rax, %r14
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::types::string::PyString>::new@GOTPCREL(%rip), %rbp
	leaq	.Lalloc_016e71ba2f68cc7172262ae988bb360c(%rip), %rdi
	movl	$10, %esi
	callq	*%rbp
.Ltmp42417:
.Ltmp42418:
	movq	%rax, %r12
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::inner@GOTPCREL(%rip), %rax
	leaq	1696(%rsp), %rdi
	movq	%r14, %rsi
	movq	%r12, %rdx
	callq	*%rax
.Ltmp42419:
	leaq	536(%rsp), %r15
	.cfi_escape 0x2e, 0x00
	movq	%r12, %rdi
	callq	*%r13
	cmpb	$1, 1696(%rsp)
	je	.LBB3021_97
	cmpb	$1, 1697(%rsp)
	je	.LBB3021_94
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
.Ltmp42420:
	.cfi_escape 0x2e, 0x00
	leaq	.Lalloc_016e71ba2f68cc7172262ae988bb360c(%rip), %rdx
	leaq	528(%rsp), %rdi
	movl	$10, %ecx
	movq	%r15, %r8
	movq	%r14, %rsi
	movq	%r12, %r15
	callq	<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::setattr::<&str, pyo3::instance::Py<pyo3::types::any::PyAny>>
.Ltmp42421:
	cmpb	$0, 528(%rsp)
	jne	.LBB3021_98
.LBB3021_94:
.Ltmp42422:
	.cfi_escape 0x2e, 0x00
	leaq	.Lalloc_6f6b49b405cc516c15819f32fccb7974(%rip), %rdi
	movl	$12, %esi
	callq	*%rbp
.Ltmp42423:
.Ltmp42424:
	movq	%rax, %r12
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::inner@GOTPCREL(%rip), %rbp
	leaq	1696(%rsp), %rdi
	movq	%r14, %rsi
	movq	%rax, %rdx
	callq	*%rbp
.Ltmp42425:
	.cfi_escape 0x2e, 0x00
	movq	%r12, %rdi
	callq	*%r13
	cmpb	$0, 1696(%rsp)
	je	.LBB3021_107
.LBB3021_97:
	leaq	1704(%rsp), %rax
	vmovups	16(%rax), %ymm1
	vmovups	(%rax), %ymm0
	vmovups	%ymm1, 16(%r15)
	vmovups	%ymm0, (%r15)
.LBB3021_98:
	vmovups	(%r15), %xmm0
	vmovups	560(%rsp), %xmm1
	movq	552(%rsp), %r15
	movq	576(%rsp), %rbp
	cmpq	$0, 2240(%rsp)
	vmovaps	%xmm0, 1696(%rsp)
	vmovaps	%xmm1, 48(%rsp)
	je	.LBB3021_111
	movq	2248(%rsp), %r12
	movq	2256(%rsp), %r14
	testq	%r12, %r12
	je	.LBB3021_105
	movq	(%r14), %rax
	testq	%rax, %rax
	je	.LBB3021_102
.Ltmp42433:
	.cfi_escape 0x2e, 0x00
	movq	%r12, %rdi
	vzeroupper
	callq	*%rax
.Ltmp42434:
.LBB3021_102:
	movq	8(%r14), %rsi
	testq	%rsi, %rsi
	je	.LBB3021_111
	movq	16(%r14), %rdx
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	vzeroupper
	callq	*%rax
	jmp	.LBB3021_111
.LBB3021_104:
	vmovaps	2224(%rsp), %xmm0
	vmovups	2248(%rsp), %xmm1
	movq	2240(%rsp), %r15
	movq	2264(%rsp), %rbp
	vmovaps	%xmm0, 1696(%rsp)
	vmovaps	%xmm1, 48(%rsp)
	jmp	.LBB3021_111
.LBB3021_105:
	.cfi_escape 0x2e, 0x00
	data16
	leaq	pyo3::internal::state::ATTACH_COUNT::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@TLSGD(%rip), %rdi
	data16
	data16
	rex64
	callq	__tls_get_addr@PLT
	cmpq	$0, (%rax)
	jle	.LBB3021_209
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	vzeroupper
	callq	*%r13
	jmp	.LBB3021_111
.LBB3021_107:
	cmpb	$0, 1697(%rsp)
	jne	.LBB3021_110
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
.Ltmp42427:
	.cfi_escape 0x2e, 0x00
	leaq	.Lalloc_6f6b49b405cc516c15819f32fccb7974(%rip), %rdx
	leaq	528(%rsp), %rdi
	movl	$12, %ecx
	movq	%r15, %r8
	movq	%r14, %rsi
	movq	%r12, %r15
	callq	<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::setattr::<&str, pyo3::instance::Py<pyo3::types::any::PyAny>>
.Ltmp42428:
	cmpb	$0, 528(%rsp)
	jne	.LBB3021_98
.LBB3021_110:
	vmovups	2248(%rsp), %xmm0
	vmovaps	2224(%rsp), %xmm1
	movq	2240(%rsp), %r15
	movq	2264(%rsp), %rbp
	vmovaps	%xmm0, 48(%rsp)
	vmovaps	%xmm1, 1696(%rsp)
.LBB3021_111:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::AttachGuard as core::ops::drop::Drop>::drop@GOTPCREL(%rip), %rax
	leaq	992(%rsp), %rdi
	vzeroupper
	callq	*%rax
	vmovaps	1696(%rsp), %xmm0
	vmovaps	48(%rsp), %xmm1
	movq	16(%rsp), %rax
	vmovups	%xmm0, (%rbx)
	movq	%r15, 24(%rax)
	vmovups	%xmm1, 32(%rax)
	movq	%rbp, 48(%rax)
	movq	$1, (%rax)
.LBB3021_112:
	addq	$2760, %rsp
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
.LBB3021_113:
	.cfi_def_cfa_offset 2816
	movq	536(%rsp), %rdx
	movb	$1, %r13b
	movq	%rdx, 32(%rsp)
	addq	$16, %rdx
.Ltmp42245:
	.cfi_escape 0x2e, 0x00
	leaq	1696(%rsp), %rdi
	leaq	992(%rsp), %rsi
	callq	purrdf_native::py_store::query::build_relations
.Ltmp42246:
	vmovups	1704(%rsp), %xmm0
	movl	1696(%rsp), %eax
	movq	1720(%rsp), %r14
	movq	1728(%rsp), %rbp
	movq	1736(%rsp), %r12
	movl	1744(%rsp), %r15d
	movl	1748(%rsp), %ecx
	vmovaps	%xmm0, 528(%rsp)
	cmpl	$1, %eax
	jne	.LBB3021_132
	vmovaps	528(%rsp), %xmm0
	movl	%ecx, 8(%rsp)
	movb	$1, %r13b
	vmovaps	%xmm0, 880(%rsp)
.LBB3021_116:
	movq	32(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB3021_119
	#MEMBARRIER
.Ltmp42374:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	32(%rsp), %rdi
	callq	*%rax
.Ltmp42375:
	jmp	.LBB3021_119
.LBB3021_131:
	vmovaps	976(%rsp), %xmm0
	movq	16(%rsp), %rax
	movl	8(%rsp), %ecx
	movb	$1, %bl
	vmovups	%xmm0, 8(%rax)
	movq	%r13, 24(%rax)
	movq	%rbp, 32(%rax)
	movq	%r12, 40(%rax)
	movl	%r15d, 48(%rax)
	movl	%ecx, 52(%rax)
	movq	$1, (%rax)
	xorl	%ebp, %ebp
	movq	48(%rsp), %rsi
	testq	%rsi, %rsi
	jg	.LBB3021_54
	jmp	.LBB3021_55
.LBB3021_132:
	vmovups	1752(%rsp), %xmm0
	vmovaps	528(%rsp), %xmm1
	movq	1768(%rsp), %rax
	movq	1056(%rsp), %rdx
	movq	%rax, 256(%rsp)
	vmovaps	%xmm0, 240(%rsp)
	vmovaps	%xmm1, 192(%rsp)
	movq	%r14, 208(%rsp)
	movq	1040(%rsp), %r14
	movq	%rbp, 216(%rsp)
	movq	%r12, 224(%rsp)
	movl	%r15d, 232(%rsp)
	movl	%ecx, 236(%rsp)
	cmpq	$-1, %r14
	je	.LBB3021_134
	movq	1048(%rsp), %rsi
	jmp	.LBB3021_135
.LBB3021_134:
	xorl	%esi, %esi
.LBB3021_135:
	movb	$1, %r13b
.Ltmp42247:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_validate::query::statistical_aggregates@GOTPCREL(%rip), %rax
	leaq	1576(%rsp), %rbx
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp42248:
	cmpq	$-1, 1064(%rsp)
	je	.LBB3021_138
	movq	1080(%rsp), %rdx
	movq	1072(%rsp), %rsi
.Ltmp42249:
	.cfi_escape 0x2e, 0x00
	leaq	528(%rsp), %rdi
	callq	<alloc::vec::Vec<alloc::string::String> as core::clone::Clone>::clone
.Ltmp42250:
	jmp	.LBB3021_139
.LBB3021_138:
	movq	$0, 528(%rsp)
	movq	$8, 536(%rsp)
	movq	$0, 544(%rsp)
.LBB3021_139:
	cmpq	$-1, 1088(%rsp)
	je	.LBB3021_142
	movq	1104(%rsp), %rdx
	movq	1096(%rsp), %rsi
.Ltmp42252:
	.cfi_escape 0x2e, 0x00
	leaq	1696(%rsp), %rdi
	callq	<alloc::vec::Vec<alloc::string::String> as core::clone::Clone>::clone
.Ltmp42253:
	movq	1696(%rsp), %rdx
	movq	1704(%rsp), %rax
	movq	1712(%rsp), %rcx
	jmp	.LBB3021_143
.LBB3021_142:
	movl	$8, %eax
	xorl	%ecx, %ecx
	xorl	%edx, %edx
.LBB3021_143:
	vmovups	528(%rsp), %xmm0
	movq	544(%rsp), %rsi
	movq	%r14, 24(%rsp)
	movq	%rsi, 1632(%rsp)
	vmovaps	%xmm0, 1616(%rsp)
	movq	%rdx, 1640(%rsp)
	movq	%rax, 1648(%rsp)
	movq	%rcx, 1656(%rsp)
	movq	$0, 1664(%rsp)
	movq	$8, 1672(%rsp)
	movq	$0, 1680(%rsp)
.Ltmp42255:
	.cfi_escape 0x2e, 0x00
	leaq	1696(%rsp), %rdi
	leaq	1064(%rsp), %rsi
	callq	purrdf_native::py_store::query::build_engine
.Ltmp42256:
	movq	192(%rsp), %rax
	movq	1576(%rsp), %r13
	leaq	192(%rsp), %rdx
	testq	%rax, %rax
	movq	%rax, 72(%rsp)
	cmoveq	%rax, %rdx
	testq	%r13, %r13
	cmoveq	%r13, %rbx
.Ltmp42258:
	.cfi_escape 0x2e, 0x00
	leaq	528(%rsp), %rdi
	leaq	1616(%rsp), %rsi
	movq	%rbx, %rcx
	callq	purrdf_native::py_store::env::extension_env
.Ltmp42259:
	vmovups	536(%rsp), %xmm0
	movq	528(%rsp), %rax
	movq	552(%rsp), %r14
	movq	560(%rsp), %rbp
	movq	568(%rsp), %r12
	movl	576(%rsp), %r15d
	movl	580(%rsp), %ecx
	vmovaps	%xmm0, 1392(%rsp)
	cmpq	$-1, %rax
	je	.LBB3021_148
	vmovups	648(%rsp), %zmm1
	vmovups	816(%rsp), %zmm0
	vmovups	584(%rsp), %zmm4
	vmovups	712(%rsp), %zmm2
	vmovups	776(%rsp), %zmm3
	movq	1024(%rsp), %rbx
	movl	$3, 1384(%rsp)
	vmovups	%zmm1, 2344(%rsp)
	vmovaps	1392(%rsp), %xmm1
	vmovups	%zmm0, 2512(%rsp)
	vmovups	%zmm3, 2472(%rsp)
	vmovups	%zmm2, 2408(%rsp)
	vmovups	%zmm4, 2280(%rsp)
	vmovups	%xmm1, 2232(%rsp)
	movq	%r14, 2248(%rsp)
	movq	%rbp, 2256(%rsp)
	movq	%r12, 2264(%rsp)
	movl	%r15d, 2272(%rsp)
	movl	%ecx, 2276(%rsp)
	movq	%rax, 2224(%rsp)
	movq	1032(%rsp), %r14
	movq	32(%rsp), %r15
	movq	152(%rsp), %rax
	cmpb	$2, %al
	jne	.LBB3021_154
	vmovups	.Lanon.4895d20928f7255621bf668e37519d0a.4+24(%rip), %zmm0
	vmovups	.Lanon.4895d20928f7255621bf668e37519d0a.4+8(%rip), %zmm1
	leaq	2224(%rsp), %rax
	vmovups	%zmm0, 896(%rsp)
	vmovups	%zmm1, 880(%rsp)
	movq	%rax, 960(%rsp)
	movq	$8, 968(%rsp)
	jmp	.LBB3021_166
.LBB3021_148:
	vmovaps	1392(%rsp), %xmm0
	movl	%ecx, 8(%rsp)
	vmovaps	%xmm0, 880(%rsp)
.Ltmp42353:
	.cfi_escape 0x2e, 0x00
	leaq	1696(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
.Ltmp42354:
	testq	%r13, %r13
	je	.LBB3021_151
	xorl	%r13d, %r13d
.Ltmp42358:
	.cfi_escape 0x2e, 0x00
	leaq	1576(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
.Ltmp42359:
.LBB3021_151:
	cmpq	$0, 72(%rsp)
	je	.LBB3021_158
.Ltmp42363:
	.cfi_escape 0x2e, 0x00
	leaq	192(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::property_fn::PropertyFunction>)>>
.Ltmp42364:
	xorl	%r13d, %r13d
.Ltmp42369:
	.cfi_escape 0x2e, 0x00
	leaq	224(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
.Ltmp42370:
	jmp	.LBB3021_116
.LBB3021_154:
	vmovups	.Lanon.4895d20928f7255621bf668e37519d0a.4+120(%rip), %zmm0
	vmovups	.Lanon.4895d20928f7255621bf668e37519d0a.4+8(%rip), %zmm1
	leaq	2224(%rsp), %r12
	vmovups	%zmm0, 1512(%rsp)
	vmovups	.Lanon.4895d20928f7255621bf668e37519d0a.4+24(%rip), %zmm0
	movq	%rax, 1392(%rsp)
	vmovups	%zmm1, 1400(%rsp)
	vmovups	%zmm0, 1416(%rsp)
	movq	%r12, 1480(%rsp)
	movq	$8, 1488(%rsp)
	vxorps	%xmm0, %xmm0, %xmm0
	vmovups	%xmm0, 1496(%rsp)
.Ltmp42260:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::engine::AdmittedSubstitutions>::requested@GOTPCREL(%rip), %rax
	leaq	528(%rsp), %rdi
	movl	$8, %ecx
	movq	%rbx, %rsi
	movq	%r14, %rdx
	xorl	%r8d, %r8d
	vzeroupper
	callq	*%rax
.Ltmp42261:
	leaq	544(%rsp), %rax
.Ltmp42263:
	.cfi_escape 0x2e, 0x10
	movq	40(%rsp), %rdx
	movq	<purrdf_sparql_eval::engine::NativeSparqlEngine>::prepare_request@GOTPCREL(%rip), %r10
	movq	8(%rsp), %rcx
	leaq	1264(%rsp), %rdi
	leaq	1696(%rsp), %rsi
	xorl	%r8d, %r8d
	pushq	%rax
	.cfi_adjust_cfa_offset 8
	pushq	%r12
	.cfi_adjust_cfa_offset 8
	callq	*%r10
	addq	$16, %rsp
	.cfi_adjust_cfa_offset -16
.Ltmp42264:
	movq	1264(%rsp), %rax
	movq	1272(%rsp), %r12
	cmpq	$-1, %rax
	je	.LBB3021_159
	vmovups	1296(%rsp), %zmm1
	vmovups	1280(%rsp), %zmm0
	vmovups	%zmm1, 912(%rsp)
	vmovups	%zmm0, 896(%rsp)
	movq	%rax, 880(%rsp)
	movq	%r12, 888(%rsp)
	jmp	.LBB3021_162
.LBB3021_158:
	xorl	%r13d, %r13d
	jmp	.LBB3021_116
.LBB3021_159:
	addq	$16, %r15
	movq	%r12, 112(%rsp)
	leaq	16(%r12), %rcx
.Ltmp42265:
	.cfi_escape 0x2e, 0x10
	subq	$8, %rsp
	.cfi_adjust_cfa_offset 8
	leaq	1400(%rsp), %rax
	movq	%r15, %rdx
	leaq	888(%rsp), %rdi
	leaq	1704(%rsp), %rsi
	movq	%rbx, %r8
	movq	%r14, %r9
	pushq	%rax
	.cfi_adjust_cfa_offset 8
	callq	<purrdf_sparql_eval::engine::NativeSparqlEngine>::query_prepared_admitted::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::dataset_view::NoopReservation<!>>
	addq	$16, %rsp
	.cfi_adjust_cfa_offset -16
.Ltmp42266:
	lock		decq	(%r12)
	jne	.LBB3021_162
	#MEMBARRIER
.Ltmp42271:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::engine::PreparedQuery>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	112(%rsp), %rdi
	callq	*%rax
.Ltmp42272:
.LBB3021_162:
	movq	544(%rsp), %rsi
	cmpq	$10, %rsi
	jb	.LBB3021_164
	movq	552(%rsp), %rdi
	shlq	$4, %rsi
	addq	$-16, %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBB3021_164:
	movq	680(%rsp), %rsi
	cmpq	$10, %rsi
	jb	.LBB3021_166
	movq	688(%rsp), %rdi
	shlq	$4, %rsi
	addq	$-16, %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBB3021_166:
	vmovups	880(%rsp), %zmm0
	vmovups	912(%rsp), %zmm1
	vmovups	%zmm0, 528(%rsp)
	vmovups	%zmm1, 560(%rsp)
	movl	1384(%rsp), %eax
	testl	%eax, %eax
	je	.LBB3021_173
.LBB3021_167:
	vmovaps	544(%rsp), %xmm0
	vmovups	592(%rsp), %ymm1
	movl	588(%rsp), %eax
	movq	528(%rsp), %r14
	movq	536(%rsp), %rbx
	movq	560(%rsp), %r13
	movq	568(%rsp), %rbp
	movq	576(%rsp), %r12
	movl	584(%rsp), %r15d
	movl	%eax, 8(%rsp)
	vmovaps	%xmm0, 112(%rsp)
	vmovups	%ymm1, 1264(%rsp)
.Ltmp42314:
	.cfi_escape 0x2e, 0x00
	leaq	1360(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::user_fn::RefusalPublication>
.Ltmp42315:
	cmpq	$-1, %r14
	je	.LBB3021_185
	vmovaps	112(%rsp), %xmm0
	movl	8(%rsp), %eax
	vmovups	1264(%rsp), %ymm1
	movq	%rbx, 536(%rsp)
	vmovups	%xmm0, 544(%rsp)
	movq	%r13, 560(%rsp)
	movq	%rbp, 568(%rsp)
	movq	%r12, 576(%rsp)
	movl	%r15d, 584(%rsp)
	movl	%eax, 588(%rsp)
	leaq	528(%rsp), %rax
	vmovups	%ymm1, 592(%rsp)
	movq	%r14, 528(%rsp)
	movq	%rax, 1392(%rsp)
	movq	<purrdf_core::diagnostic::RdfDiagnostic as core::fmt::Display>::fmt@GOTPCREL(%rip), %rax
	movq	%rax, 1400(%rsp)
.Ltmp42316:
	.cfi_escape 0x2e, 0x00
	movq	alloc::fmt::format::format_inner@GOTPCREL(%rip), %rax
	leaq	.Lalloc_592fabd2ffa0e5a6b6713c88c8b00c99(%rip), %rsi
	leaq	880(%rsp), %rdi
	leaq	1392(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp42317:
	movq	608(%rsp), %rbx
	movq	536(%rsp), %rdi
	movq	544(%rsp), %rsi
.Ltmp42318:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_validate::xpath_regex::diagnostic_refusal_code@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp42319:
	testq	%rbx, %rbx
	movq	%rdx, %rcx
	setne	%dl
	testq	%rax, %rax
	sete	%sil
	orb	%dl, %sil
	cmpb	$1, %sil
	jne	.LBB3021_186
.Ltmp42323:
	.cfi_escape 0x2e, 0x00
	leaq	1392(%rsp), %rdi
	leaq	880(%rsp), %rsi
	movq	%rbx, %rdx
	callq	purrdf_native::py_store::presentation::presented_value_error
.Ltmp42324:
	jmp	.LBB3021_187
.LBB3021_173:
	movq	1376(%rsp), %r14
	movl	$1, %ecx
	xorl	%eax, %eax
	lock		cmpxchgl	%ecx, 32(%r14)
	leaq	32(%r14), %rbx
	jne	.LBB3021_212
.LBB3021_174:
	movq	std::panicking::panic_count::GLOBAL_PANIC_COUNT@GOTPCREL(%rip), %r15
	movq	(%r15), %rax
	shlq	%rax
	testq	%rax, %rax
	jne	.LBB3021_213
	movzbl	36(%r14), %eax
	vmovups	40(%r14), %ymm0
	leaq	36(%r14), %r12
	vmovups	%ymm0, 1264(%rsp)
	movq	$0, 40(%r14)
	movq	(%r15), %rax
	shlq	%rax
	testq	%rax, %rax
	jne	.LBB3021_216
.LBB3021_176:
	xorl	%eax, %eax
	xchgl	%eax, (%rbx)
	cmpl	$2, %eax
	je	.LBB3021_219
.LBB3021_177:
	cmpq	$-1, 528(%rsp)
	movq	1264(%rsp), %rax
	je	.LBB3021_203
	testq	%rax, %rax
	je	.LBB3021_203
	vmovups	1264(%rsp), %ymm0
	movq	1376(%rsp), %rax
	vmovups	%ymm0, 1392(%rsp)
	movq	16(%rax), %rcx
	movq	24(%rax), %rax
	movq	16(%rax), %rdx
	movq	32(%rax), %rax
	decq	%rdx
	andq	$-16, %rdx
	leaq	16(%rcx,%rdx), %rdi
.Ltmp42285:
	.cfi_escape 0x2e, 0x00
	leaq	1392(%rsp), %rsi
	vzeroupper
	callq	*%rax
.Ltmp42286:
	movq	1392(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB3021_182
	#MEMBARRIER
.Ltmp42290:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn core::error::Error + core::marker::Sync + core::marker::Send>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1392(%rsp), %rdi
	callq	*%rax
.Ltmp42291:
.LBB3021_182:
	movq	1408(%rsp), %rax
	testq	%rax, %rax
	je	.LBB3021_167
	lock		decq	(%rax)
	jne	.LBB3021_167
	leaq	1408(%rsp), %rdi
	#MEMBARRIER
.Ltmp42296:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn purrdf_sparql_eval::user_fn::UserFunctionAdmission>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp42297:
	jmp	.LBB3021_167
.LBB3021_185:
	vmovaps	112(%rsp), %xmm0
	jmp	.LBB3021_189
.LBB3021_186:
.Ltmp42321:
	.cfi_escape 0x2e, 0x00
	leaq	1392(%rsp), %rdi
	leaq	880(%rsp), %rsi
	movq	%rax, %rdx
	callq	purrdf_native::py_store::presentation::refusal_value_error
.Ltmp42322:
.LBB3021_187:
.Ltmp42329:
	.cfi_escape 0x2e, 0x00
	leaq	528(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.Ltmp42330:
	vmovups	1392(%rsp), %xmm0
	movl	1436(%rsp), %eax
	movq	1408(%rsp), %r13
	movq	1416(%rsp), %rbp
	movq	1424(%rsp), %r12
	movl	1432(%rsp), %r15d
	movq	$-1, %rbx
	movl	%eax, 8(%rsp)
.LBB3021_189:
	vmovaps	%xmm0, 880(%rsp)
.Ltmp42334:
	movq	%r13, 40(%rsp)
	.cfi_escape 0x2e, 0x00
	leaq	2224(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::extension_env::ExtensionEnv>
.Ltmp42335:
.Ltmp42339:
	.cfi_escape 0x2e, 0x00
	leaq	1696(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
.Ltmp42340:
	cmpq	$0, 1576(%rsp)
	movq	24(%rsp), %r14
	je	.LBB3021_194
	xorl	%r13d, %r13d
.Ltmp42341:
	.cfi_escape 0x2e, 0x00
	leaq	1576(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
.Ltmp42342:
.LBB3021_194:
	cmpq	$0, 192(%rsp)
	je	.LBB3021_197
.Ltmp42343:
	.cfi_escape 0x2e, 0x00
	leaq	192(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::property_fn::PropertyFunction>)>>
.Ltmp42344:
	xorl	%r13d, %r13d
.Ltmp42349:
	.cfi_escape 0x2e, 0x00
	leaq	224(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
.Ltmp42350:
.LBB3021_197:
	movq	32(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB3021_199
	xorl	%r13d, %r13d
	#MEMBARRIER
.Ltmp42351:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	32(%rsp), %rdi
	callq	*%rax
.Ltmp42352:
.LBB3021_199:
	cmpq	$-1, %r14
	je	.LBB3021_202
	movq	40(%rsp), %r13
	testq	%r14, %r14
	je	.LBB3021_125
	movq	1048(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	movq	%r14, %rsi
	callq	*%rax
	jmp	.LBB3021_125
.LBB3021_202:
	movq	40(%rsp), %r13
	jmp	.LBB3021_125
.LBB3021_203:
	testq	%rax, %rax
	je	.LBB3021_167
	lock		decq	(%rax)
	jne	.LBB3021_206
	#MEMBARRIER
.Ltmp42299:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn core::error::Error + core::marker::Sync + core::marker::Send>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1264(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp42300:
.LBB3021_206:
	movq	1280(%rsp), %rax
	testq	%rax, %rax
	je	.LBB3021_167
	lock		decq	(%rax)
	jne	.LBB3021_167
	leaq	1280(%rsp), %rdi
	#MEMBARRIER
.Ltmp42305:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn purrdf_sparql_eval::user_fn::UserFunctionAdmission>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp42306:
	jmp	.LBB3021_167
.LBB3021_209:
.Ltmp42436:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::instance::Py<_> as core::ops::drop::Drop>::drop::drop_slow@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp42437:
	jmp	.LBB3021_111
.LBB3021_210:
.Ltmp42237:
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$24, %esi
	callq	*%rax
.Ltmp42238:
	ud2
.LBB3021_212:
.Ltmp42274:
	.cfi_escape 0x2e, 0x00
	movq	<std::sys::sync::mutex::futex::Mutex>::lock_contended@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.Ltmp42275:
	jmp	.LBB3021_174
.LBB3021_213:
.Ltmp42276:
	.cfi_escape 0x2e, 0x00
	movq	std::panicking::panic_count::is_zero_slow_path@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp42277:
	movzbl	36(%r14), %ecx
	vmovups	40(%r14), %ymm0
	vmovups	%ymm0, 1264(%rsp)
	movq	$0, 40(%r14)
	testb	%al, %al
	je	.LBB3021_176
	addq	$36, %r14
	movq	%r14, %r12
	movq	(%r15), %rax
	shlq	%rax
	testq	%rax, %rax
	je	.LBB3021_176
.LBB3021_216:
.Ltmp42278:
	.cfi_escape 0x2e, 0x00
	movq	std::panicking::panic_count::is_zero_slow_path@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp42279:
	testb	%al, %al
	jne	.LBB3021_176
	movb	$1, (%r12)
	jmp	.LBB3021_176
.LBB3021_219:
.Ltmp42280:
	.cfi_escape 0x2e, 0x00
	movq	<std::sys::sync::mutex::futex::Mutex>::wake@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.Ltmp42281:
	jmp	.LBB3021_177
.LBB3021_220:
.Ltmp42282:
	cmpq	$0, 1264(%rsp)
	movq	%rax, %r15
	je	.LBB3021_237
.Ltmp42283:
	.cfi_escape 0x2e, 0x00
	leaq	1264(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::user_fn::UserFunctionRefusal>
.Ltmp42284:
	jmp	.LBB3021_237
.LBB3021_222:
.Ltmp42438:
	movq	%rax, %r15
	jmp	.LBB3021_291
.LBB3021_223:
.Ltmp42307:
	movq	%rax, %r15
	jmp	.LBB3021_237
.LBB3021_224:
.Ltmp42298:
	movq	%rax, %r15
	jmp	.LBB3021_237
.LBB3021_225:
.Ltmp42301:
	movq	%rax, %r15
	movq	1280(%rsp), %rax
	testq	%rax, %rax
	je	.LBB3021_237
	lock		decq	(%rax)
	jne	.LBB3021_237
	leaq	1280(%rsp), %rdi
	#MEMBARRIER
.Ltmp42302:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn purrdf_sparql_eval::user_fn::UserFunctionAdmission>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp42303:
	jmp	.LBB3021_237
.LBB3021_228:
.Ltmp42304:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3021_229:
.Ltmp42292:
	movq	%rax, %r15
	movq	1408(%rsp), %rax
	testq	%rax, %rax
	je	.LBB3021_237
	lock		decq	(%rax)
	jne	.LBB3021_237
	leaq	1408(%rsp), %rdi
	#MEMBARRIER
.Ltmp42293:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn purrdf_sparql_eval::user_fn::UserFunctionAdmission>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp42294:
	jmp	.LBB3021_237
.LBB3021_232:
.Ltmp42295:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3021_233:
.Ltmp42267:
	lock		decq	(%r12)
	movq	%rax, %r15
	jne	.LBB3021_244
	#MEMBARRIER
.Ltmp42268:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::engine::PreparedQuery>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	112(%rsp), %rdi
	callq	*%rax
.Ltmp42269:
	jmp	.LBB3021_244
.LBB3021_235:
.Ltmp42270:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3021_236:
.Ltmp42287:
	movq	%rax, %r15
.Ltmp42288:
	.cfi_escape 0x2e, 0x00
	leaq	1392(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::user_fn::UserFunctionRefusal>
.Ltmp42289:
.LBB3021_237:
.Ltmp42308:
	.cfi_escape 0x2e, 0x00
	leaq	528(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::result::Result<purrdf_core::backend::SparqlResult, purrdf_core::diagnostic::RdfDiagnostic>>
.Ltmp42309:
	jmp	.LBB3021_245
.LBB3021_238:
.Ltmp42310:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3021_239:
.Ltmp42365:
	movq	%rax, %r15
.Ltmp42366:
	.cfi_escape 0x2e, 0x00
	leaq	224(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
.Ltmp42367:
	jmp	.LBB3021_247
.LBB3021_241:
.Ltmp42368:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3021_242:
.Ltmp42262:
	movq	%rax, %r15
	jmp	.LBB3021_245
.LBB3021_243:
.Ltmp42273:
	movq	%rax, %r15
.LBB3021_244:
	.cfi_escape 0x2e, 0x00
	leaq	528(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::AdmittedSubstitutions>
.LBB3021_245:
.Ltmp42311:
	.cfi_escape 0x2e, 0x00
	leaq	1360(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::user_fn::RefusalPublication>
.Ltmp42312:
	jmp	.LBB3021_262
.LBB3021_246:
.Ltmp42345:
	movq	%rax, %r15
.Ltmp42346:
	.cfi_escape 0x2e, 0x00
	leaq	224(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
.Ltmp42347:
.LBB3021_247:
	xorl	%r13d, %r13d
	jmp	.LBB3021_271
.LBB3021_248:
.Ltmp42348:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3021_249:
.Ltmp42320:
	movq	880(%rsp), %rsi
	movq	%rax, %r15
	testq	%rsi, %rsi
	je	.LBB3021_256
	movq	888(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
	jmp	.LBB3021_256
.LBB3021_251:
.Ltmp42313:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3021_252:
.Ltmp42254:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	leaq	528(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::string::String>>
	jmp	.LBB3021_254
.LBB3021_253:
.Ltmp42251:
	movq	%rax, %r15
.LBB3021_254:
	movb	$1, %r13b
	jmp	.LBB3021_266
.LBB3021_255:
.Ltmp42325:
	movq	%rax, %r15
.LBB3021_256:
.Ltmp42326:
	.cfi_escape 0x2e, 0x00
	leaq	528(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.Ltmp42327:
	jmp	.LBB3021_262
.LBB3021_257:
.Ltmp42328:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3021_258:
.Ltmp42355:
	movq	%rax, %r15
	jmp	.LBB3021_265
.LBB3021_259:
.Ltmp42376:
	movq	%rax, %r15
	jmp	.LBB3021_322
.LBB3021_260:
.Ltmp42257:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	leaq	1616(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_algebra::parser::ParserOptions>
	jmp	.LBB3021_265
.LBB3021_261:
.Ltmp42331:
	movq	%rax, %r15
.LBB3021_262:
.Ltmp42332:
	.cfi_escape 0x2e, 0x00
	leaq	2224(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::extension_env::ExtensionEnv>
.Ltmp42333:
	jmp	.LBB3021_264
.LBB3021_263:
.Ltmp42336:
	movq	%rax, %r15
.LBB3021_264:
.Ltmp42337:
	.cfi_escape 0x2e, 0x00
	leaq	1696(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
.Ltmp42338:
.LBB3021_265:
	xorl	%r13d, %r13d
.LBB3021_266:
	cmpq	$0, 1576(%rsp)
	je	.LBB3021_269
.Ltmp42356:
	.cfi_escape 0x2e, 0x00
	leaq	1576(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
.Ltmp42357:
	jmp	.LBB3021_269
.LBB3021_268:
.Ltmp42360:
	movq	%rax, %r15
.LBB3021_269:
.Ltmp42361:
	.cfi_escape 0x2e, 0x00
	leaq	192(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry>>
.Ltmp42362:
	jmp	.LBB3021_271
.LBB3021_270:
.Ltmp42371:
	movq	%rax, %r15
.LBB3021_271:
	movq	32(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB3021_322
	#MEMBARRIER
.Ltmp42372:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	32(%rsp), %rdi
	callq	*%rax
.Ltmp42373:
	jmp	.LBB3021_322
.LBB3021_273:
.Ltmp42225:
	movq	%rax, %r15
	jmp	.LBB3021_320
.LBB3021_274:
.Ltmp42435:
	movq	8(%r14), %rsi
	movq	%rax, %r15
	testq	%rsi, %rsi
	je	.LBB3021_291
	movq	16(%r14), %rdx
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	callq	*%rax
	jmp	.LBB3021_291
.LBB3021_276:
.Ltmp42382:
	movq	%rax, %r15
	jmp	.LBB3021_328
.LBB3021_277:
.Ltmp42218:
	movq	%rax, %r15
.Ltmp42219:
	.cfi_escape 0x2e, 0x00
	leaq	992(%rsp), %rdi
	callq	core::ptr::drop_glue::<<purrdf_native::py_store::quad_store::PyQuadStore>::query_impl<()>::{closure#0}::{closure#0}>
.Ltmp42220:
	jmp	.LBB3021_281
.LBB3021_278:
.Ltmp42213:
	movq	%rax, %r15
	leaq	376(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
	leaq	352(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	leaq	328(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
.Ltmp42214:
	.cfi_escape 0x2e, 0x00
	leaq	80(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>
.Ltmp42215:
	jmp	.LBB3021_302
.LBB3021_280:
.Ltmp42390:
	movq	%rax, %r15
.LBB3021_281:
	movl	$0, 24(%rsp)
	xorl	%r12d, %r12d
	jmp	.LBB3021_329
.LBB3021_282:
.Ltmp42228:
	movq	%rax, %r15
	jmp	.LBB3021_321
.LBB3021_283:
.Ltmp42210:
	movb	$1, %r12b
	movq	%rax, %r15
	jmp	.LBB3021_303
.LBB3021_284:
.Ltmp42415:
	jmp	.LBB3021_289
.LBB3021_285:
.Ltmp42426:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	movq	%r12, %rdi
	callq	*%r13
	jmp	.LBB3021_290
.LBB3021_286:
.Ltmp42412:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	movq	_Py_DecRef@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
	jmp	.LBB3021_290
.LBB3021_287:
.Ltmp42407:
	movq	%rax, %r15
.Ltmp42408:
	.cfi_escape 0x2e, 0x00
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<pyo3::err::PyErr>
.Ltmp42409:
	jmp	.LBB3021_337
.LBB3021_288:
.Ltmp42429:
.LBB3021_289:
	movq	%rax, %r15
.LBB3021_290:
.Ltmp42430:
	.cfi_escape 0x2e, 0x00
	leaq	2224(%rsp), %rdi
	callq	core::ptr::drop_glue::<pyo3::err::PyErr>
.Ltmp42431:
.LBB3021_291:
.Ltmp42439:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::AttachGuard as core::ops::drop::Drop>::drop@GOTPCREL(%rip), %rax
	leaq	992(%rsp), %rdi
	callq	*%rax
.Ltmp42440:
	jmp	.LBB3021_337
.LBB3021_292:
.Ltmp42404:
	movq	%rax, %r15
	jmp	.LBB3021_329
.LBB3021_293:
.Ltmp42441:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3021_294:
.Ltmp42432:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3021_295:
.Ltmp42393:
	movq	%rax, %r15
	testq	%r12, %r12
	je	.LBB3021_299
	negq	%r12
	addq	$160, %r14
	.p2align	4
.LBB3021_297:
.Ltmp42394:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.Ltmp42395:
	addq	$160, %r14
	decq	%r12
	jne	.LBB3021_297
.LBB3021_299:
	movq	80(%rsp), %rax
	testq	%rax, %rax
	je	.LBB3021_302
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	callq	*%rax
.LBB3021_302:
	xorl	%r12d, %r12d
.LBB3021_303:
.Ltmp42397:
	.cfi_escape 0x2e, 0x00
	leaq	160(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp42398:
	jmp	.LBB3021_329
.LBB3021_304:
.Ltmp42396:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3021_305:
.Ltmp42399:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3021_306:
.Ltmp42231:
	movq	%rax, %r15
	testq	%r12, %r12
	je	.LBB3021_310
	negq	%r12
	addq	$160, %r14
.LBB3021_308:
.Ltmp42232:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.Ltmp42233:
	addq	$160, %r14
	decq	%r12
	jne	.LBB3021_308
.LBB3021_310:
	movq	992(%rsp), %rax
	testq	%rax, %rax
	je	.LBB3021_312
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	callq	*%rax
.LBB3021_312:
	movq	1040(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB3021_315
	testq	%rsi, %rsi
	je	.LBB3021_315
	movq	1048(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBB3021_315:
	leaq	1064(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
.Ltmp42235:
	.cfi_escape 0x2e, 0x00
	leaq	1016(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp42236:
	jmp	.LBB3021_328
.LBB3021_316:
.Ltmp42234:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3021_317:
.Ltmp42205:
	movq	%rax, %r15
	movb	$1, %al
	movb	$1, %r12b
	movl	%eax, 24(%rsp)
	jmp	.LBB3021_329
.LBB3021_318:
.Ltmp42239:
	movq	%rax, %r15
	testq	%rbx, %rbx
	je	.LBB3021_320
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	movq	%r14, %rdi
	movq	%rbx, %rsi
	callq	*%rax
.LBB3021_320:
.Ltmp42240:
	.cfi_escape 0x2e, 0x00
	leaq	1696(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.Ltmp42241:
.LBB3021_321:
	movb	$1, %r13b
.Ltmp42243:
	.cfi_escape 0x2e, 0x00
	leaq	992(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>
.Ltmp42244:
.LBB3021_322:
	movq	1040(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB3021_325
	testq	%rsi, %rsi
	je	.LBB3021_325
	movq	1048(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBB3021_325:
	testb	%r13b, %r13b
	je	.LBB3021_327
	leaq	1064(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
.LBB3021_327:
.Ltmp42377:
	.cfi_escape 0x2e, 0x00
	leaq	1016(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp42378:
.LBB3021_328:
	movl	$0, 24(%rsp)
.Ltmp42383:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::SuspendAttach as core::ops::drop::Drop>::drop@GOTPCREL(%rip), %rax
	leaq	288(%rsp), %rdi
	callq	*%rax
.Ltmp42384:
	xorl	%r12d, %r12d
.LBB3021_329:
	cmpq	$0, 48(%rsp)
	jle	.LBB3021_331
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	64(%rsp), %rdi
	movq	48(%rsp), %rsi
	movl	$1, %edx
	callq	*%rax
.LBB3021_331:
	testb	%r12b, %r12b
	je	.LBB3021_333
	leaq	328(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	leaq	352(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	leaq	376(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
.LBB3021_333:
	cmpb	$0, 24(%rsp)
	je	.LBB3021_337
	movq	424(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB3021_337
	testq	%rsi, %rsi
	je	.LBB3021_337
	movq	432(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBB3021_337:
	.cfi_escape 0x2e, 0x00
	movq	%r15, %rdi
	callq	_Unwind_Resume@PLT
.LBB3021_338:
.Ltmp42385:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3021_339:
.Ltmp42379:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3021_340:
.Ltmp42242:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end3021:
<purrdf_native::py_store::quad_store::PyQuadStore>::__pymethod_query__:
.Lfunc_begin1857:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception1857
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
	subq	$680, %rsp
	.cfi_def_cfa_offset 736
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	vxorps	%xmm0, %xmm0, %xmm0
	vmovups	%zmm0, 504(%rsp)
	movq	%rsi, %r15
	movq	%rdi, %r14
	vmovups	%zmm0, 480(%rsp)
	.cfi_escape 0x2e, 0x10
	subq	$8, %rsp
	.cfi_adjust_cfa_offset 8
	leaq	.Lalloc_f1016e7258a9778c8e4642ffd2ad0599(%rip), %rsi
	leaq	8(%rsp), %rdi
	leaq	488(%rsp), %r9
	pushq	$11
	.cfi_adjust_cfa_offset 8
	vzeroupper
	callq	<pyo3::impl_::extract_argument::FunctionDescription>::extract_arguments_fastcall::<pyo3::impl_::extract_argument::NoVarargs, pyo3::impl_::extract_argument::NoVarkeywords>
	addq	$16, %rsp
	.cfi_adjust_cfa_offset -16
	cmpl	$1, (%rsp)
	jne	.LBB3023_2
	vmovups	24(%rsp), %ymm1
	vmovups	8(%rsp), %ymm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%ymm0, 8(%r14)
	movq	$1, (%r14)
	jmp	.LBB3023_117
.LBB3023_2:
	leaq	312(%r15), %rbx
	movq	$0, 224(%rsp)
	movq	$0, 232(%rsp)
	movq	$0, 240(%rsp)
	movq	$0, 248(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::pycell::impl_::BorrowChecker as pyo3::pycell::impl_::PyClassBorrowChecker>::try_borrow@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
	testb	%al, %al
	je	.LBB3023_4
	leaq	640(%rsp), %rbx
	leaq	632(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr as core::convert::From<pyo3::pycell::PyBorrowError>>::from@GOTPCREL(%rip), %rax
	callq	*%rax
	vmovups	(%rbx), %ymm0
	movq	632(%rsp), %rax
	movq	32(%rbx), %rcx
	movq	%rax, 8(%r14)
	movq	%rcx, 48(%r14)
	vmovups	%ymm0, 16(%r14)
	movq	$1, (%r14)
	jmp	.LBB3023_117
.LBB3023_4:
	movq	480(%rsp), %rsi
	addq	$16, %r15
	movq	%r15, 632(%rsp)
	movq	$0, 624(%rsp)
.Ltmp42614:
	.cfi_escape 0x2e, 0x00
	movq	<&str as pyo3::conversion::FromPyObject>::extract@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	callq	*%rax
.Ltmp42615:
	cmpl	$1, (%rsp)
	je	.LBB3023_118
	movq	8(%rsp), %r13
	movq	16(%rsp), %rbp
	movq	488(%rsp), %r12
	testq	%r12, %r12
	je	.LBB3023_14
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB3023_14
	movq	8(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movq	PyType_GetFlags@GOTPCREL(%rip), %rax
	callq	*%rax
	testl	$536870912, %eax
	jne	.LBB3023_20
	.cfi_escape 0x2e, 0x00
	movq	PyDict_Type@GOTPCREL(%rip), %r15
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.Ltmp42616:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r15, %rdx
	callq	*%rax
.Ltmp42617:
	leaq	8(%rsp), %rdi
.Ltmp42618:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_f98cafa4d6296fc0caeb65b9f5347e73(%rip), %rsi
	leaq	56(%rsp), %rcx
	movl	$13, %edx
	callq	*%rax
.Ltmp42619:
	vmovups	16(%rsp), %ymm0
	movq	48(%rsp), %rcx
	movq	8(%rsp), %rax
	movq	%rcx, 48(%r14)
	vmovups	%ymm0, 16(%r14)
	movq	%rax, 8(%r14)
.LBB3023_12:
	movq	$1, (%r14)
	lock		decq	(%rbx)
	jmp	.LBB3023_117
.LBB3023_14:
	xorl	%eax, %eax
	movq	496(%rsp), %r12
	movq	%rax, 392(%rsp)
	testq	%r12, %r12
	je	.LBB3023_21
.LBB3023_15:
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB3023_22
.Ltmp42620:
	.cfi_escape 0x2e, 0x00
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	<alloc::vec::Vec<alloc::string::String> as pyo3::conversion::FromPyObject>::extract
.Ltmp42621:
	movq	8(%rsp), %rax
	cmpb	$0, (%rsp)
	leaq	16(%rsp), %rcx
	je	.LBB3023_23
	vmovups	(%rcx), %ymm0
	movq	32(%rcx), %rdx
	leaq	64(%rsp), %rdi
	movq	%rdx, 144(%rsp)
	movq	%rdx, 40(%rsp)
	vmovups	%ymm0, 8(%rsp)
	vmovups	%ymm0, 112(%rsp)
	movq	%rax, (%rsp)
.Ltmp42622:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_908475b3e6cbafcd4d8e4fa6ac07f48c(%rip), %rsi
	movl	$20, %edx
	movq	%rsp, %rcx
	vzeroupper
	callq	*%rax
.Ltmp42623:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %ymm0
	vmovups	%ymm1, 192(%rsp)
	vmovups	%ymm0, 176(%rsp)
	vmovups	192(%rsp), %ymm1
	vmovups	176(%rsp), %ymm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%ymm0, 8(%r14)
	movq	$1, (%r14)
	jmp	.LBB3023_31
.LBB3023_20:
	leaq	224(%rsp), %rax
	movq	%r12, 224(%rsp)
	movq	496(%rsp), %r12
	movq	%rax, 392(%rsp)
	testq	%r12, %r12
	jne	.LBB3023_15
.LBB3023_21:
	movq	$-1, 64(%rsp)
	jmp	.LBB3023_25
.LBB3023_22:
	movq	$-1, %rax
	jmp	.LBB3023_24
.LBB3023_23:
	vmovups	(%rcx), %xmm0
	vmovaps	%xmm0, 112(%rsp)
.LBB3023_24:
	vmovaps	112(%rsp), %xmm0
	vmovups	%xmm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBB3023_25:
	movq	80(%rsp), %rax
	movq	64(%rsp), %rcx
	movq	72(%rsp), %rdx
	movq	504(%rsp), %r12
	movq	%rbx, 168(%rsp)
	movq	%rcx, 256(%rsp)
	movq	%rdx, 264(%rsp)
	movq	%rax, 272(%rsp)
	testq	%r12, %r12
	je	.LBB3023_32
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB3023_33
.Ltmp42624:
	.cfi_escape 0x2e, 0x00
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	<alloc::vec::Vec<alloc::string::String> as pyo3::conversion::FromPyObject>::extract
.Ltmp42625:
	movq	8(%rsp), %rax
	cmpb	$0, (%rsp)
	leaq	16(%rsp), %rcx
	je	.LBB3023_34
	vmovups	(%rcx), %ymm0
	movq	32(%rcx), %rdx
	leaq	64(%rsp), %rdi
	movq	%rdx, 144(%rsp)
	movq	%rdx, 40(%rsp)
	vmovups	%ymm0, 8(%rsp)
	vmovups	%ymm0, 112(%rsp)
	movq	%rax, (%rsp)
.Ltmp42626:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_ed816c88becfff4b944384e4ed1786c6(%rip), %rsi
	movl	$22, %edx
	movq	%rsp, %rcx
	vzeroupper
	callq	*%rax
.Ltmp42627:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %ymm0
	vmovups	%ymm1, 192(%rsp)
	vmovups	%ymm0, 176(%rsp)
	vmovups	192(%rsp), %ymm1
	vmovups	176(%rsp), %ymm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%ymm0, 8(%r14)
	movq	$1, (%r14)
	movq	256(%rsp), %r12
	cmpq	$-1, %r12
	jne	.LBB3023_69
.LBB3023_31:
	lock		decq	(%rbx)
	jmp	.LBB3023_117
.LBB3023_32:
	movq	$-1, 64(%rsp)
	jmp	.LBB3023_36
.LBB3023_33:
	movq	$-1, %rax
	jmp	.LBB3023_35
.LBB3023_34:
	vmovups	(%rcx), %xmm0
	vmovaps	%xmm0, 112(%rsp)
.LBB3023_35:
	vmovaps	112(%rsp), %xmm0
	vmovups	%xmm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBB3023_36:
	movq	80(%rsp), %rax
	movq	64(%rsp), %rcx
	movq	72(%rsp), %rdx
	movq	512(%rsp), %r12
	movq	%rcx, 320(%rsp)
	movq	%rdx, 328(%rsp)
	movq	%rax, 336(%rsp)
	testq	%r12, %r12
	je	.LBB3023_42
	leaq	16(%rsp), %rbx
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB3023_43
.Ltmp42629:
	.cfi_escape 0x2e, 0x00
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	<(alloc::string::String, alloc::string::String) as pyo3::conversion::FromPyObject>::extract
.Ltmp42630:
	movq	8(%rsp), %rax
	cmpl	$1, (%rsp)
	jne	.LBB3023_44
	vmovups	(%rbx), %ymm0
	movq	32(%rbx), %rcx
	leaq	64(%rsp), %rdi
	movq	%rcx, 216(%rsp)
	vmovups	%ymm0, 184(%rsp)
	movq	%rax, 176(%rsp)
.Ltmp42631:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_93df691527a5f68f324bb7548b51c806(%rip), %rsi
	leaq	176(%rsp), %rcx
	movl	$21, %edx
	vzeroupper
	callq	*%rax
.Ltmp42632:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %ymm0
	movq	168(%rsp), %rbx
	vmovups	%ymm1, 128(%rsp)
	vmovups	%ymm0, 112(%rsp)
	vmovups	128(%rsp), %ymm1
	vmovups	112(%rsp), %ymm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%ymm0, 8(%r14)
	movq	$1, (%r14)
	jmp	.LBB3023_68
.LBB3023_42:
	movq	$-1, 64(%rsp)
	jmp	.LBB3023_45
.LBB3023_43:
	movq	$-1, %rax
.LBB3023_44:
	vmovups	(%rbx), %ymm0
	movq	32(%rbx), %rcx
	movq	168(%rsp), %rbx
	movq	%rcx, 104(%rsp)
	vmovups	%ymm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBB3023_45:
	vmovups	80(%rsp), %xmm1
	vmovups	72(%rsp), %xmm0
	movq	96(%rsp), %rdx
	movq	64(%rsp), %rax
	movq	88(%rsp), %rcx
	movq	520(%rsp), %r12
	movq	%rdx, 144(%rsp)
	movq	104(%rsp), %rdx
	movq	%rax, 112(%rsp)
	movq	112(%rsp), %rax
	movq	144(%rsp), %rdi
	vmovups	%xmm1, 128(%rsp)
	vmovups	%xmm0, 120(%rsp)
	movq	%rcx, 136(%rsp)
	vmovups	120(%rsp), %xmm0
	movq	136(%rsp), %rcx
	movq	136(%rsp), %rsi
	movq	%rdx, 152(%rsp)
	movq	128(%rsp), %rdx
	movq	152(%rsp), %r8
	movq	%rax, 400(%rsp)
	movq	%rdi, 432(%rsp)
	movq	%rcx, 424(%rsp)
	movq	%rsi, 424(%rsp)
	movq	%r8, 440(%rsp)
	vmovups	%xmm0, 408(%rsp)
	movq	%rdx, 416(%rsp)
	testq	%r12, %r12
	je	.LBB3023_51
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB3023_51
	movq	8(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movq	PyType_GetFlags@GOTPCREL(%rip), %rax
	callq	*%rax
	testl	$536870912, %eax
	jne	.LBB3023_56
	.cfi_escape 0x2e, 0x00
	movq	PyDict_Type@GOTPCREL(%rip), %r15
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.Ltmp42634:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r15, %rdx
	callq	*%rax
.Ltmp42635:
	leaq	8(%rsp), %rdi
.Ltmp42636:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_9865433ad06614f46384a300efcb7195(%rip), %rsi
	leaq	56(%rsp), %rcx
	movl	$9, %edx
	callq	*%rax
.Ltmp42637:
	jmp	.LBB3023_65
.LBB3023_51:
	xorl	%eax, %eax
	movq	528(%rsp), %r12
	movq	%rax, 384(%rsp)
	testq	%r12, %r12
	jne	.LBB3023_52
	jmp	.LBB3023_58
.LBB3023_56:
	leaq	232(%rsp), %rax
	movq	%r12, 232(%rsp)
	movq	528(%rsp), %r12
	movq	%rax, 384(%rsp)
	testq	%r12, %r12
	je	.LBB3023_58
.LBB3023_52:
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB3023_58
	movq	8(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movq	PyType_GetFlags@GOTPCREL(%rip), %rax
	callq	*%rax
	testl	$536870912, %eax
	jne	.LBB3023_59
	.cfi_escape 0x2e, 0x00
	movq	PyDict_Type@GOTPCREL(%rip), %r15
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.Ltmp42638:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r15, %rdx
	callq	*%rax
.Ltmp42639:
	leaq	8(%rsp), %rdi
.Ltmp42640:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_fcd42a11584ddca09ee4bceee5bc0781(%rip), %rsi
	leaq	56(%rsp), %rcx
	movl	$20, %edx
	callq	*%rax
.Ltmp42641:
	jmp	.LBB3023_65
.LBB3023_58:
	xorl	%eax, %eax
	movq	536(%rsp), %r12
	movq	%rax, 376(%rsp)
	testq	%r12, %r12
	jne	.LBB3023_61
	jmp	.LBB3023_78
.LBB3023_59:
	leaq	240(%rsp), %rax
	movq	%r12, 240(%rsp)
	movq	536(%rsp), %r12
	movq	%rax, 376(%rsp)
	testq	%r12, %r12
	je	.LBB3023_78
.LBB3023_61:
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB3023_78
	movq	8(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movq	PyType_GetFlags@GOTPCREL(%rip), %rax
	callq	*%rax
	testl	$536870912, %eax
	jne	.LBB3023_79
	.cfi_escape 0x2e, 0x00
	movq	PyDict_Type@GOTPCREL(%rip), %r15
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.Ltmp42642:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r15, %rdx
	callq	*%rax
.Ltmp42643:
	leaq	8(%rsp), %rdi
.Ltmp42644:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_2f742b7bf960932f7bae3af5028d09e0(%rip), %rsi
	leaq	56(%rsp), %rcx
	movl	$14, %edx
	callq	*%rax
.Ltmp42645:
.LBB3023_65:
	vmovups	16(%rsp), %ymm0
	movq	48(%rsp), %rcx
	movq	8(%rsp), %rax
	movq	%rcx, 48(%r14)
	vmovups	%ymm0, 16(%r14)
	movq	%rax, 8(%r14)
.LBB3023_66:
	movq	$1, (%r14)
.LBB3023_67:
	.cfi_escape 0x2e, 0x00
	leaq	400(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
.LBB3023_68:
	.cfi_escape 0x2e, 0x00
	leaq	320(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	movq	256(%rsp), %r12
	cmpq	$-1, %r12
	je	.LBB3023_31
.LBB3023_69:
	movq	264(%rsp), %r15
	movq	272(%rsp), %r13
	testq	%r13, %r13
	je	.LBB3023_74
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rbx
	leaq	8(%r15), %rbp
	jmp	.LBB3023_72
	.p2align	4
.LBB3023_71:
	addq	$24, %rbp
	decq	%r13
	je	.LBB3023_74
.LBB3023_72:
	movq	-8(%rbp), %rsi
	testq	%rsi, %rsi
	je	.LBB3023_71
	movq	(%rbp), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%rbx
	jmp	.LBB3023_71
.LBB3023_74:
	testq	%r12, %r12
	je	.LBB3023_76
	shlq	$3, %r12
	leaq	(%r12,%r12,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%r15, %rdi
	vzeroupper
	callq	*%rax
.LBB3023_76:
	movq	168(%rsp), %rax
	lock		decq	(%rax)
	jmp	.LBB3023_117
.LBB3023_78:
	xorl	%eax, %eax
	movq	544(%rsp), %r12
	movq	%rax, 368(%rsp)
	testq	%r12, %r12
	je	.LBB3023_86
.LBB3023_81:
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB3023_87
.Ltmp42646:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::string::String as pyo3::conversion::FromPyObject>::extract@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	*%rax
.Ltmp42647:
	movq	8(%rsp), %rax
	cmpb	$0, (%rsp)
	leaq	16(%rsp), %rcx
	je	.LBB3023_88
	vmovups	(%rcx), %ymm0
	movq	32(%rcx), %rdx
	leaq	64(%rsp), %rdi
	movq	%rdx, 144(%rsp)
	movq	%rdx, 40(%rsp)
	vmovups	%ymm0, 8(%rsp)
	vmovups	%ymm0, 112(%rsp)
	movq	%rax, (%rsp)
.Ltmp42648:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_589d20955c603729090b6aaccd02222f(%rip), %rsi
	movl	$19, %edx
	movq	%rsp, %rcx
	vzeroupper
	callq	*%rax
.Ltmp42649:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %ymm0
	vmovups	%ymm1, 192(%rsp)
	vmovups	%ymm0, 176(%rsp)
	vmovups	192(%rsp), %ymm1
	vmovups	176(%rsp), %ymm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%ymm0, 8(%r14)
	jmp	.LBB3023_66
.LBB3023_79:
	leaq	248(%rsp), %rax
	movq	%r12, 248(%rsp)
	movq	544(%rsp), %r12
	movq	%rax, 368(%rsp)
	testq	%r12, %r12
	jne	.LBB3023_81
.LBB3023_86:
	movq	$-1, 64(%rsp)
	jmp	.LBB3023_90
.LBB3023_87:
	movq	$-1, %rax
	jmp	.LBB3023_89
.LBB3023_88:
	vmovups	(%rcx), %xmm0
	vmovaps	%xmm0, 112(%rsp)
.LBB3023_89:
	vmovaps	112(%rsp), %xmm0
	vmovups	%xmm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBB3023_90:
	movq	80(%rsp), %rax
	movq	64(%rsp), %rcx
	movq	72(%rsp), %rdx
	movq	552(%rsp), %r12
	movq	%r13, 360(%rsp)
	movq	%rcx, 288(%rsp)
	movq	%rdx, 296(%rsp)
	movq	%rax, 304(%rsp)
	testq	%r12, %r12
	je	.LBB3023_96
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	xorl	%r13d, %r13d
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB3023_97
.Ltmp42651:
	.cfi_escape 0x2e, 0x00
	movq	<&str as pyo3::conversion::FromPyObject>::extract@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	*%rax
.Ltmp42652:
	movq	8(%rsp), %r13
	movq	16(%rsp), %rax
	cmpb	$0, (%rsp)
	je	.LBB3023_98
	vmovups	24(%rsp), %ymm0
	leaq	64(%rsp), %rdi
	vmovups	%ymm0, 192(%rsp)
	movq	%r13, 176(%rsp)
	movq	%rax, 184(%rsp)
.Ltmp42653:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_6c43a98927907a5f8539235d09232c7e(%rip), %rsi
	leaq	176(%rsp), %rcx
	movl	$11, %edx
	vzeroupper
	callq	*%rax
.Ltmp42654:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %xmm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%xmm0, 8(%r14)
	jmp	.LBB3023_105
.LBB3023_96:
	xorl	%r13d, %r13d
	jmp	.LBB3023_99
.LBB3023_97:
	jmp	.LBB3023_99
.LBB3023_98:
	movq	%rax, 352(%rsp)
.LBB3023_99:
	movq	560(%rsp), %r12
	testq	%r12, %r12
	je	.LBB3023_108
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB3023_109
.Ltmp42655:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::string::String as pyo3::conversion::FromPyObject>::extract@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	*%rax
.Ltmp42656:
	movq	8(%rsp), %rax
	cmpb	$0, (%rsp)
	leaq	16(%rsp), %rcx
	je	.LBB3023_110
	vmovups	(%rcx), %ymm0
	movq	32(%rcx), %rdx
	leaq	64(%rsp), %rdi
	movq	%rdx, 144(%rsp)
	movq	%rdx, 40(%rsp)
	vmovups	%ymm0, 8(%rsp)
	vmovups	%ymm0, 112(%rsp)
	movq	%rax, (%rsp)
.Ltmp42657:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_8497b6da903267d7e7d13af19cddca66(%rip), %rsi
	movl	$8, %edx
	movq	%rsp, %rcx
	vzeroupper
	callq	*%rax
.Ltmp42658:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %ymm0
	vmovups	%ymm1, 192(%rsp)
	vmovups	%ymm0, 176(%rsp)
	vmovups	192(%rsp), %ymm1
	vmovups	176(%rsp), %ymm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%ymm0, 8(%r14)
.LBB3023_105:
	movq	288(%rsp), %rsi
	movq	$1, (%r14)
	cmpq	$-1, %rsi
	je	.LBB3023_67
	testq	%rsi, %rsi
	je	.LBB3023_67
	movq	296(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
	jmp	.LBB3023_67
.LBB3023_108:
	movq	%rbx, %r12
	movq	$-1, 64(%rsp)
	jmp	.LBB3023_112
.LBB3023_109:
	movq	%rbx, %r12
	movq	$-1, %rax
	jmp	.LBB3023_111
.LBB3023_110:
	vmovups	(%rcx), %xmm0
	movq	%rbx, %r12
	vmovaps	%xmm0, 112(%rsp)
.LBB3023_111:
	vmovaps	112(%rsp), %xmm0
	vmovups	%xmm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBB3023_112:
	movq	64(%rsp), %rcx
	movq	80(%rsp), %rax
	movq	%rcx, 448(%rsp)
	movq	72(%rsp), %rcx
	movq	%rax, 464(%rsp)
	movq	%rcx, 456(%rsp)
.Ltmp42660:
	.cfi_escape 0x2e, 0x50
	subq	$8, %rsp
	.cfi_adjust_cfa_offset 8
	movq	368(%rsp), %rdx
	movq	400(%rsp), %r8
	leaq	456(%rsp), %rax
	leaq	296(%rsp), %r10
	leaq	408(%rsp), %r11
	leaq	328(%rsp), %rbx
	leaq	576(%rsp), %rdi
	leaq	264(%rsp), %r9
	movq	%r15, %rsi
	movq	%rbp, %rcx
	pushq	%rax
	.cfi_adjust_cfa_offset 8
	pushq	368(%rsp)
	.cfi_adjust_cfa_offset 8
	pushq	%r13
	.cfi_adjust_cfa_offset 8
	pushq	%r10
	.cfi_adjust_cfa_offset 8
	pushq	408(%rsp)
	.cfi_adjust_cfa_offset 8
	pushq	424(%rsp)
	.cfi_adjust_cfa_offset 8
	pushq	440(%rsp)
	.cfi_adjust_cfa_offset 8
	pushq	%r11
	.cfi_adjust_cfa_offset 8
	pushq	%rbx
	.cfi_adjust_cfa_offset 8
	vzeroupper
	callq	<purrdf_native::py_store::quad_store::PyQuadStore>::query
	addq	$80, %rsp
	.cfi_adjust_cfa_offset -80
.Ltmp42661:
	movq	576(%rsp), %rax
	cmpl	$1, 568(%rsp)
	jne	.LBB3023_115
	vmovups	584(%rsp), %ymm0
	movq	616(%rsp), %rcx
	movq	%rcx, 48(%r14)
	movl	$1, %ecx
	vmovups	%ymm0, 16(%r14)
	jmp	.LBB3023_116
.LBB3023_115:
	xorl	%ecx, %ecx
.LBB3023_116:
	movq	%rax, 8(%r14)
	movq	%rcx, (%r14)
	lock		decq	(%r12)
.LBB3023_117:
	movq	%r14, %rax
	addq	$680, %rsp
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
.LBB3023_118:
	.cfi_def_cfa_offset 736
	leaq	8(%rsp), %rax
	leaq	64(%rsp), %rdi
	vmovups	16(%rax), %ymm1
	vmovups	(%rax), %ymm0
	vmovups	%ymm1, 192(%rsp)
	vmovups	%ymm0, 176(%rsp)
.Ltmp42663:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_da5c5f922604d9376dbdf48c863f8565(%rip), %rsi
	leaq	176(%rsp), %rcx
	movl	$5, %edx
	vzeroupper
	callq	*%rax
.Ltmp42664:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %xmm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%xmm0, 8(%r14)
	jmp	.LBB3023_12
.LBB3023_120:
.Ltmp42662:
	movq	%rax, %r14
	lock		decq	(%r12)
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB3023_121:
.Ltmp42633:
	movq	168(%rsp), %r12
	movq	%rax, %r14
	jmp	.LBB3023_128
.LBB3023_122:
.Ltmp42659:
	movq	288(%rsp), %rsi
	movq	%rbx, %r12
	movq	%rax, %r14
	cmpq	$-1, %rsi
	je	.LBB3023_127
	testq	%rsi, %rsi
	je	.LBB3023_127
	movq	296(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
	jmp	.LBB3023_127
.LBB3023_125:
.Ltmp42628:
	movq	%rbx, %r12
	movq	%rax, %r14
	jmp	.LBB3023_129
.LBB3023_126:
.Ltmp42650:
	movq	%rbx, %r12
	movq	%rax, %r14
.LBB3023_127:
	.cfi_escape 0x2e, 0x00
	leaq	400(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
.LBB3023_128:
	.cfi_escape 0x2e, 0x00
	leaq	320(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
.LBB3023_129:
	.cfi_escape 0x2e, 0x00
	leaq	256(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	lock		decq	(%r12)
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB3023_130:
.Ltmp42665:
	movq	%rbx, %r12
	movq	%rax, %r14
	lock		decq	(%r12)
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end3023:
