<purrdf_native::py_store::quad_store::PyQuadStore>::query:
.Lfunc_begin1700:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception1700
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
	subq	$2824, %rsp
	.cfi_def_cfa_offset 2880
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	2944(%rsp), %rax
	movq	%rsi, %rbp
	movq	%r9, %r14
	vmovups	(%r14), %xmm1
	movq	%r8, %r15
	movq	2888(%rsp), %r8
	movq	2912(%rsp), %r11
	movq	2904(%rsp), %r13
	movq	2896(%rsp), %rbx
	movq	%rdi, 16(%rsp)
	movq	2920(%rsp), %rdi
	movq	2936(%rsp), %r9
	movq	2928(%rsp), %r10
	movq	%rdx, 24(%rsp)
	movq	%rcx, 8(%rsp)
	movq	16(%rax), %rsi
	vmovups	(%rax), %xmm0
	movq	16(%r14), %rax
	vmovups	(%r8), %ymm2
	movq	%rsi, 400(%rsp)
	movq	2880(%rsp), %rsi
	vmovaps	%xmm0, 384(%rsp)
	movq	%r15, 552(%rsp)
	movq	%rbx, 560(%rsp)
	movq	%r13, 568(%rsp)
	movq	%r11, 576(%rsp)
	vmovups	%xmm1, 408(%rsp)
	vmovups	16(%r8), %ymm1
	movq	%rax, 424(%rsp)
	vmovups	(%rsi), %xmm3
	movq	16(%rsi), %rax
	xorl	%esi, %esi
	movq	%rax, 448(%rsp)
	movq	16(%rdi), %rax
	vmovaps	%xmm3, 432(%rsp)
	vmovups	%ymm2, 456(%rsp)
	vmovups	%ymm1, 472(%rsp)
	vmovups	(%rdi), %xmm1
	movq	%r10, 584(%rsp)
	movq	%r9, 592(%rsp)
	movq	%rbp, 528(%rsp)
	movq	%rax, 520(%rsp)
	vmovups	%xmm1, 504(%rsp)
	movq	%rdx, 536(%rsp)
	movq	%rcx, 544(%rsp)
	movq	384(%rsp), %rax
	movq	400(%rsp), %rdx
	movq	%rax, 48(%rsp)
	cmpq	$-1, %rax
	movq	392(%rsp), %rax
	movq	%rax, 72(%rsp)
	cmovneq	%rax, %rsi
	movb	$1, %al
	movl	%eax, 40(%rsp)
.Ltmp35772:
	.cfi_escape 0x2e, 0x00
	leaq	1728(%rsp), %rdi
	movb	$1, %r12b
	vzeroupper
	callq	purrdf_native::py_store::env::division_policy
.Ltmp35773:
	cmpl	$1, 1728(%rsp)
	jne	.LBB2826_3
	vmovups	1752(%rsp), %ymm0
	vmovups	1740(%rsp), %ymm1
	movq	16(%rsp), %r15
	movl	1736(%rsp), %eax
	vmovups	%ymm0, 24(%r15)
	vmovups	%ymm1, 12(%r15)
	movl	%eax, 8(%r15)
	jmp	.LBB2826_6
.LBB2826_3:
	movq	1732(%rsp), %rax
	movq	%rax, 128(%rsp)
.Ltmp35774:
	.cfi_escape 0x2e, 0x00
	leaq	1728(%rsp), %rdi
	movq	%r15, %rsi
	movb	$1, %r12b
	callq	purrdf_native::py_store::quad_store::collect_substitutions
.Ltmp35775:
	cmpl	$1, 1728(%rsp)
	leaq	1736(%rsp), %r15
	jne	.LBB2826_29
	vmovups	16(%r15), %ymm1
	vmovups	(%r15), %ymm0
	movq	16(%rsp), %r15
	vmovups	%ymm1, 624(%rsp)
	vmovups	%ymm0, 608(%rsp)
	vmovups	624(%rsp), %ymm1
	vmovups	608(%rsp), %ymm0
	vmovups	%ymm1, 24(%r15)
	vmovups	%ymm0, 8(%r15)
.LBB2826_6:
	movq	48(%rsp), %rsi
	movb	$1, %bpl
	movq	$1, (%r15)
	testq	%rsi, %rsi
	jle	.LBB2826_8
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	72(%rsp), %rdi
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LBB2826_8:
	movq	408(%rsp), %r14
	cmpq	$-1, %r14
	je	.LBB2826_16
	movq	416(%rsp), %rbx
	movq	424(%rsp), %r15
	testq	%r15, %r15
	je	.LBB2826_14
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r13
	leaq	8(%rbx), %r12
	jmp	.LBB2826_12
	.p2align	4
.LBB2826_11:
	addq	$24, %r12
	decq	%r15
	je	.LBB2826_14
.LBB2826_12:
	movq	-8(%r12), %rsi
	testq	%rsi, %rsi
	je	.LBB2826_11
	movq	(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r13
	jmp	.LBB2826_11
.LBB2826_14:
	movq	16(%rsp), %r15
	testq	%r14, %r14
	je	.LBB2826_16
	shlq	$3, %r14
	leaq	(%r14,%r14,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB2826_16:
	movq	432(%rsp), %r14
	cmpq	$-1, %r14
	je	.LBB2826_24
	movq	440(%rsp), %rbx
	movq	448(%rsp), %r15
	testq	%r15, %r15
	je	.LBB2826_22
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r13
	leaq	8(%rbx), %r12
	jmp	.LBB2826_20
	.p2align	4
.LBB2826_19:
	addq	$24, %r12
	decq	%r15
	je	.LBB2826_22
.LBB2826_20:
	movq	-8(%r12), %rsi
	testq	%rsi, %rsi
	je	.LBB2826_19
	movq	(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r13
	jmp	.LBB2826_19
.LBB2826_22:
	movq	16(%rsp), %r15
	testq	%r14, %r14
	je	.LBB2826_24
	shlq	$3, %r14
	leaq	(%r14,%r14,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB2826_24:
	movq	456(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB2826_78
	testq	%rsi, %rsi
	je	.LBB2826_27
	movq	464(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LBB2826_27:
	movq	480(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB2826_78
	movq	488(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
	jmp	.LBB2826_78
.LBB2826_29:
	vmovups	(%r15), %xmm0
	movq	16(%r15), %rax
	movq	%rax, 256(%rsp)
	vmovaps	%xmm0, 240(%rsp)
.Ltmp35776:
	.cfi_escape 0x2e, 0x00
	movq	2912(%rsp), %rcx
	leaq	1728(%rsp), %rdi
	movq	%rbx, %rsi
	movq	%r13, %rdx
	callq	purrdf_native::py_store::query::collect_relations
.Ltmp35777:
	cmpl	$1, 1728(%rsp)
	jne	.LBB2826_32
	vmovups	16(%r15), %ymm1
	vmovups	(%r15), %ymm0
	movq	16(%rsp), %rax
	movb	$1, %r12b
	vmovups	%ymm1, 624(%rsp)
	vmovups	%ymm0, 608(%rsp)
	vmovups	624(%rsp), %ymm1
	vmovups	608(%rsp), %ymm0
	vmovups	%ymm1, 24(%rax)
	vmovups	%ymm0, 8(%rax)
	movq	$1, (%rax)
	jmp	.LBB2826_74
.LBB2826_32:
	movq	408(%rsp), %r8
	movq	416(%rsp), %rcx
	vmovups	(%r15), %xmm0
	movq	16(%r15), %rax
	movq	424(%rsp), %r15
	movq	448(%rsp), %r12
	movq	456(%rsp), %r13
	movq	480(%rsp), %rbx
	movq	%r8, 88(%rsp)
	movq	%rcx, 64(%rsp)
	movq	432(%rsp), %r8
	movq	440(%rsp), %rcx
	movq	%rax, 112(%rsp)
	vmovaps	%xmm0, 96(%rsp)
	movq	%r8, 80(%rsp)
	movq	%rcx, 120(%rsp)
	movq	464(%rsp), %r8
	movq	488(%rsp), %rcx
	movq	%r8, 352(%rsp)
	movq	%rcx, 360(%rsp)
.Ltmp35779:
	.cfi_escape 0x2e, 0x00
	movq	2928(%rsp), %rsi
	movq	2936(%rsp), %rdx
	leaq	2616(%rsp), %rdi
	callq	purrdf_native::xpath_regex::selection
.Ltmp35780:
	movzbl	2616(%rsp), %eax
	cmpb	$-1, %al
	je	.LBB2826_46
	vmovups	2672(%rsp), %xmm0
	vmovups	2640(%rsp), %ymm1
	movq	16(%r14), %rcx
	movq	2880(%rsp), %rdx
	vmovups	2617(%rsp), %ymm5
	vmovaps	96(%rsp), %xmm2
	movq	%rcx, 2704(%rsp)
	movq	16(%rdx), %rcx
	vmovups	(%rdx), %xmm4
	movq	2920(%rsp), %rdx
	vmovaps	%xmm0, 1200(%rsp)
	vmovups	%ymm1, 1168(%rsp)
	vmovups	(%r14), %xmm1
	vmovups	%ymm5, 1145(%rsp)
	movq	%rbp, 1224(%rsp)
	vmovaps	%xmm2, 976(%rsp)
	vmovaps	%xmm1, 2688(%rsp)
	movq	%rcx, 2728(%rsp)
	movq	2888(%rsp), %rcx
	vmovups	%xmm4, 2712(%rsp)
	vmovups	(%rcx), %ymm3
	vmovups	16(%rcx), %ymm1
	movq	112(%rsp), %rcx
	movq	%rcx, 992(%rsp)
	movq	16(%rdx), %rcx
	vmovups	%ymm3, 2736(%rsp)
	vmovups	%ymm1, 2752(%rsp)
	vmovups	(%rdx), %xmm1
	movq	8(%rsp), %rdx
	vmovups	2688(%rsp), %zmm0
	movq	%rcx, 1040(%rsp)
	movq	24(%rsp), %rcx
	vmovaps	%xmm1, 1024(%rsp)
	vmovups	2720(%rsp), %zmm1
	vmovups	%zmm1, 1080(%rsp)
	vmovups	%zmm0, 1048(%rsp)
	vmovaps	240(%rsp), %xmm0
	movb	%al, 1144(%rsp)
	movq	%rcx, 1232(%rsp)
	movq	128(%rsp), %rax
	movq	256(%rsp), %rcx
	movq	%rdx, 1240(%rsp)
	movq	%rcx, 1016(%rsp)
	vmovups	%xmm0, 1000(%rsp)
	movq	%rax, 1216(%rsp)
.Ltmp35784:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::SuspendAttach>::new@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp35785:
	movq	%rax, 368(%rsp)
	movq	%rdx, 376(%rsp)
.Ltmp35789:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::ir::mutable::MutableDataset>::freeze@GOTPCREL(%rip), %rax
	leaq	608(%rsp), %rdi
	movq	%rbp, %rsi
	callq	*%rax
.Ltmp35790:
	movq	16(%rsp), %r15
	leaq	1728(%rsp), %rax
	cmpq	$-1, 608(%rsp)
	je	.LBB2826_114
	vmovups	640(%rsp), %zmm1
	vmovups	608(%rsp), %zmm0
	movq	%rax, 1360(%rsp)
	movq	<purrdf_core::diagnostic::RdfDiagnostic as core::fmt::Display>::fmt@GOTPCREL(%rip), %rax
	movq	%rax, 1368(%rsp)
	vmovups	%zmm1, 1760(%rsp)
	vmovups	%zmm0, 1728(%rsp)
.Ltmp35791:
	.cfi_escape 0x2e, 0x00
	movq	alloc::fmt::format::format_inner@GOTPCREL(%rip), %rax
	leaq	.Lalloc_0e5f90e3dc675d538220deef7d17e145(%rip), %rsi
	leaq	2256(%rsp), %rdi
	leaq	1360(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp35792:
	movq	2256(%rsp), %rbx
	movq	2264(%rsp), %r14
	movq	2272(%rsp), %r15
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_no_alloc_shim_is_unstable_v2@GOTPCREL(%rip), %rax
	callq	*%rax
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_alloc@GOTPCREL(%rip), %rax
	movl	$24, %edi
	movl	$8, %esi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB2826_211
	movq	%rax, %rbp
	movq	%rbx, (%rax)
	movq	%r14, 8(%rax)
	movq	%r15, 16(%rax)
.Ltmp35794:
	.cfi_escape 0x2e, 0x00
	leaq	1728(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.Ltmp35795:
	movq	16(%rsp), %r15
	movq	984(%rsp), %rbx
	movq	992(%rsp), %rax
	vxorps	%xmm0, %xmm0, %xmm0
	vmovaps	%xmm0, 144(%rsp)
	testq	%rax, %rax
	je	.LBB2826_44
	movl	$1, %r12d
	movq	%rbx, %r14
	subq	%rax, %r12
	.p2align	4
.LBB2826_42:
.Ltmp35797:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.Ltmp35798:
	incq	%r12
	addq	$160, %r14
	cmpq	$1, %r12
	jne	.LBB2826_42
.LBB2826_44:
	movq	976(%rsp), %rax
	movl	$1, %ecx
	movb	$1, %r14b
	leaq	.Lvtable.1R(%rip), %r13
	movl	$3, %r12d
	movq	%rcx, 8(%rsp)
	testq	%rax, %rax
	je	.LBB2826_119
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	callq	*%rax
.LBB2826_119:
.LBB2826_120:
	movq	1024(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB2826_123
	testq	%rsi, %rsi
	je	.LBB2826_123
	movq	1032(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBB2826_123:
	movq	$-1, %rbx
	testb	%r14b, %r14b
	je	.LBB2826_125
	.cfi_escape 0x2e, 0x00
	leaq	1048(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
.LBB2826_125:
	movq	8(%rsp), %r14
.LBB2826_126:
.Ltmp35948:
	.cfi_escape 0x2e, 0x00
	leaq	1000(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp35949:
.Ltmp35954:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::SuspendAttach as core::ops::drop::Drop>::drop@GOTPCREL(%rip), %rax
	leaq	368(%rsp), %rdi
	callq	*%rax
.Ltmp35955:
	vmovaps	144(%rsp), %xmm0
	vmovaps	%xmm0, 960(%rsp)
	cmpq	$-1, %rbx
	je	.LBB2826_132
	vmovaps	960(%rsp), %xmm0
	movl	24(%rsp), %eax
	movq	%rbx, 1728(%rsp)
	vmovups	%xmm0, 1736(%rsp)
	movq	%r14, 1752(%rsp)
	movq	%rbp, 1760(%rsp)
	movq	%r13, 1768(%rsp)
	movl	%r12d, 1776(%rsp)
	movl	%eax, 1780(%rsp)
.Ltmp35956:
	.cfi_escape 0x2e, 0x00
	leaq	1728(%rsp), %rsi
	movq	%r15, %rdi
	callq	purrdf_native::py_store::query::materialize_results
.Ltmp35957:
	movq	48(%rsp), %rsi
	testq	%rsi, %rsi
	jle	.LBB2826_84
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	72(%rsp), %rdi
	jmp	.LBB2826_82
.LBB2826_46:
	vmovups	2624(%rsp), %ymm0
	vmovups	2640(%rsp), %ymm1
	movq	16(%rsp), %rax
	vmovups	%ymm1, 24(%rax)
	vmovups	%ymm0, 8(%rax)
	movq	$1, (%rax)
	cmpq	$-1, %r13
	je	.LBB2826_51
	testq	%r13, %r13
	je	.LBB2826_49
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	352(%rsp), %rdi
	movl	$1, %edx
	movq	%r13, %rsi
	vzeroupper
	callq	*%rax
.LBB2826_49:
	testq	%rbx, %rbx
	je	.LBB2826_51
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	360(%rsp), %rdi
	movl	$1, %edx
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
.LBB2826_51:
	movq	80(%rsp), %r13
	cmpq	$-1, %r13
	je	.LBB2826_59
	testq	%r12, %r12
	je	.LBB2826_57
	movq	120(%rsp), %rax
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r14
	leaq	8(%rax), %rbx
	jmp	.LBB2826_55
	.p2align	4
.LBB2826_54:
	addq	$24, %rbx
	decq	%r12
	je	.LBB2826_57
.LBB2826_55:
	movq	-8(%rbx), %rsi
	testq	%rsi, %rsi
	je	.LBB2826_54
	movq	(%rbx), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r14
	jmp	.LBB2826_54
.LBB2826_57:
	testq	%r13, %r13
	je	.LBB2826_59
	shlq	$3, %r13
	leaq	(%r13,%r13,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	120(%rsp), %rdi
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBB2826_59:
	movq	88(%rsp), %r12
	cmpq	$-1, %r12
	je	.LBB2826_67
	testq	%r15, %r15
	je	.LBB2826_65
	movq	64(%rsp), %rax
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r14
	leaq	8(%rax), %rbx
	jmp	.LBB2826_63
	.p2align	4
.LBB2826_62:
	addq	$24, %rbx
	decq	%r15
	je	.LBB2826_65
.LBB2826_63:
	movq	-8(%rbx), %rsi
	testq	%rsi, %rsi
	je	.LBB2826_62
	movq	(%rbx), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r14
	jmp	.LBB2826_62
.LBB2826_65:
	testq	%r12, %r12
	je	.LBB2826_67
	shlq	$3, %r12
	leaq	(%r12,%r12,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	64(%rsp), %rdi
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBB2826_67:
	movq	104(%rsp), %rbx
	movq	112(%rsp), %rax
	testq	%rax, %rax
	je	.LBB2826_71
	movl	$1, %r12d
	movq	%rbx, %r14
	subq	%rax, %r12
	.p2align	4
.LBB2826_69:
.Ltmp35959:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.Ltmp35960:
	incq	%r12
	addq	$160, %r14
	cmpq	$1, %r12
	jne	.LBB2826_69
.LBB2826_71:
	movq	96(%rsp), %rax
	testq	%rax, %rax
	je	.LBB2826_73
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB2826_73:
	xorl	%r12d, %r12d
.LBB2826_74:
	movb	$1, %bpl
.Ltmp35968:
	.cfi_escape 0x2e, 0x00
	leaq	240(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp35969:
	movq	48(%rsp), %rsi
	testq	%rsi, %rsi
	jle	.LBB2826_77
.LBB2826_76:
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	72(%rsp), %rdi
	movl	$1, %edx
	callq	*%rax
.LBB2826_77:
	movq	16(%rsp), %r15
	testb	%r12b, %r12b
	jne	.LBB2826_8
.LBB2826_78:
	testb	%bpl, %bpl
	je	.LBB2826_84
	movq	504(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB2826_84
	testq	%rsi, %rsi
	je	.LBB2826_84
	movq	512(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
.LBB2826_82:
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LBB2826_84:
	cmpl	$1, (%r15)
	jne	.LBB2826_113
	leaq	8(%r15), %rbx
.Ltmp35971:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::AttachGuard>::attach@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp35972:
	movl	%eax, 976(%rsp)
	movq	PyExc_ValueError@GOTPCREL(%rip), %rax
	vmovups	16(%rbx), %ymm1
	vmovups	(%rbx), %ymm0
	movq	(%rax), %r14
	vmovups	%ymm1, 2272(%rsp)
	vmovups	%ymm0, 2256(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp35976:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr>::get_type@GOTPCREL(%rip), %rax
	leaq	2256(%rsp), %rdi
	callq	*%rax
.Ltmp35977:
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
	je	.LBB2826_105
.Ltmp35979:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr>::value@GOTPCREL(%rip), %rax
	leaq	2256(%rsp), %rdi
	callq	*%rax
.Ltmp35980:
.Ltmp35982:
	movq	%rax, %r14
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::types::string::PyString>::new@GOTPCREL(%rip), %rbp
	leaq	.Lalloc_016e71ba2f68cc7172262ae988bb360c(%rip), %rdi
	movl	$10, %esi
	callq	*%rbp
.Ltmp35983:
.Ltmp35984:
	movq	%rax, %r12
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::inner@GOTPCREL(%rip), %rax
	leaq	1728(%rsp), %rdi
	movq	%r14, %rsi
	movq	%r12, %rdx
	callq	*%rax
.Ltmp35985:
	leaq	616(%rsp), %r15
	.cfi_escape 0x2e, 0x00
	movq	%r12, %rdi
	callq	*%r13
	cmpb	$1, 1728(%rsp)
	je	.LBB2826_98
	cmpb	$1, 1729(%rsp)
	je	.LBB2826_95
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
.Ltmp35986:
	.cfi_escape 0x2e, 0x00
	leaq	.Lalloc_016e71ba2f68cc7172262ae988bb360c(%rip), %rdx
	leaq	608(%rsp), %rdi
	movl	$10, %ecx
	movq	%r15, %r8
	movq	%r14, %rsi
	movq	%r12, %r15
	callq	<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::setattr::<&str, pyo3::instance::Py<pyo3::types::any::PyAny>>
.Ltmp35987:
	cmpb	$0, 608(%rsp)
	jne	.LBB2826_99
.LBB2826_95:
.Ltmp35988:
	.cfi_escape 0x2e, 0x00
	leaq	.Lalloc_6f6b49b405cc516c15819f32fccb7974(%rip), %rdi
	movl	$12, %esi
	callq	*%rbp
.Ltmp35989:
.Ltmp35990:
	movq	%rax, %r12
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::inner@GOTPCREL(%rip), %rbp
	leaq	1728(%rsp), %rdi
	movq	%r14, %rsi
	movq	%rax, %rdx
	callq	*%rbp
.Ltmp35991:
	.cfi_escape 0x2e, 0x00
	movq	%r12, %rdi
	callq	*%r13
	cmpb	$0, 1728(%rsp)
	je	.LBB2826_108
.LBB2826_98:
	leaq	1736(%rsp), %rax
	vmovups	16(%rax), %ymm1
	vmovups	(%rax), %ymm0
	vmovups	%ymm1, 16(%r15)
	vmovups	%ymm0, (%r15)
.LBB2826_99:
	vmovups	(%r15), %xmm0
	vmovups	640(%rsp), %xmm1
	movq	632(%rsp), %r15
	movq	656(%rsp), %rbp
	cmpq	$0, 2272(%rsp)
	vmovaps	%xmm0, 1728(%rsp)
	vmovaps	%xmm1, 48(%rsp)
	je	.LBB2826_112
	movq	2280(%rsp), %r12
	movq	2288(%rsp), %r14
	testq	%r12, %r12
	je	.LBB2826_106
	movq	(%r14), %rax
	testq	%rax, %rax
	je	.LBB2826_103
.Ltmp35999:
	.cfi_escape 0x2e, 0x00
	movq	%r12, %rdi
	vzeroupper
	callq	*%rax
.Ltmp36000:
.LBB2826_103:
	movq	8(%r14), %rsi
	testq	%rsi, %rsi
	je	.LBB2826_112
	movq	16(%r14), %rdx
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	vzeroupper
	callq	*%rax
	jmp	.LBB2826_112
.LBB2826_105:
	vmovaps	2256(%rsp), %xmm0
	vmovups	2280(%rsp), %xmm1
	movq	2272(%rsp), %r15
	movq	2296(%rsp), %rbp
	vmovaps	%xmm0, 1728(%rsp)
	vmovaps	%xmm1, 48(%rsp)
	jmp	.LBB2826_112
.LBB2826_106:
	.cfi_escape 0x2e, 0x00
	data16
	leaq	pyo3::internal::state::ATTACH_COUNT::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@TLSGD(%rip), %rdi
	data16
	data16
	rex64
	callq	__tls_get_addr@PLT
	cmpq	$0, (%rax)
	jle	.LBB2826_210
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	vzeroupper
	callq	*%r13
	jmp	.LBB2826_112
.LBB2826_108:
	cmpb	$0, 1729(%rsp)
	jne	.LBB2826_111
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
.Ltmp35993:
	.cfi_escape 0x2e, 0x00
	leaq	.Lalloc_6f6b49b405cc516c15819f32fccb7974(%rip), %rdx
	leaq	608(%rsp), %rdi
	movl	$12, %ecx
	movq	%r15, %r8
	movq	%r14, %rsi
	movq	%r12, %r15
	callq	<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::setattr::<&str, pyo3::instance::Py<pyo3::types::any::PyAny>>
.Ltmp35994:
	cmpb	$0, 608(%rsp)
	jne	.LBB2826_99
.LBB2826_111:
	vmovups	2280(%rsp), %xmm0
	vmovaps	2256(%rsp), %xmm1
	movq	2272(%rsp), %r15
	movq	2296(%rsp), %rbp
	vmovaps	%xmm0, 48(%rsp)
	vmovaps	%xmm1, 1728(%rsp)
.LBB2826_112:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::AttachGuard as core::ops::drop::Drop>::drop@GOTPCREL(%rip), %rax
	leaq	976(%rsp), %rdi
	vzeroupper
	callq	*%rax
	vmovaps	1728(%rsp), %xmm0
	vmovaps	48(%rsp), %xmm1
	movq	16(%rsp), %rax
	vmovups	%xmm0, (%rbx)
	movq	%r15, 24(%rax)
	vmovups	%xmm1, 32(%rax)
	movq	%rbp, 48(%rax)
	movq	$1, (%rax)
.LBB2826_113:
	addq	$2824, %rsp
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
.LBB2826_114:
	.cfi_def_cfa_offset 2880
	movq	616(%rsp), %rdx
	movb	$1, %r14b
	movq	%rdx, 32(%rsp)
	addq	$16, %rdx
.Ltmp35813:
	.cfi_escape 0x2e, 0x00
	leaq	1728(%rsp), %rdi
	leaq	976(%rsp), %rsi
	callq	purrdf_native::py_store::query::build_relations
.Ltmp35814:
	vmovups	1736(%rsp), %xmm0
	movl	1728(%rsp), %eax
	movq	1752(%rsp), %rcx
	movq	1760(%rsp), %rbp
	movq	1768(%rsp), %r13
	movl	1776(%rsp), %r12d
	movl	1780(%rsp), %edx
	vmovaps	%xmm0, 608(%rsp)
	cmpl	$1, %eax
	jne	.LBB2826_133
	vmovaps	608(%rsp), %xmm0
	movl	%edx, 24(%rsp)
	movq	%rcx, 8(%rsp)
	movb	$1, %r14b
	vmovaps	%xmm0, 144(%rsp)
.LBB2826_117:
	movq	32(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB2826_120
	#MEMBARRIER
.Ltmp35942:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	32(%rsp), %rdi
	callq	*%rax
.Ltmp35943:
	jmp	.LBB2826_120
.LBB2826_132:
	vmovaps	960(%rsp), %xmm0
	movl	24(%rsp), %eax
	vmovups	%xmm0, 8(%r15)
	movq	%r14, 24(%r15)
	movq	%rbp, 32(%r15)
	movq	%r13, 40(%r15)
	movl	%r12d, 48(%r15)
	movl	%eax, 52(%r15)
	movq	$1, (%r15)
	xorl	%ebp, %ebp
	xorl	%r12d, %r12d
	movq	48(%rsp), %rsi
	testq	%rsi, %rsi
	jg	.LBB2826_76
	jmp	.LBB2826_77
.LBB2826_133:
	vmovups	1784(%rsp), %xmm0
	vmovaps	608(%rsp), %xmm1
	movq	1800(%rsp), %rax
	movq	%rax, 336(%rsp)
	movq	1024(%rsp), %rax
	vmovaps	%xmm0, 320(%rsp)
	vmovaps	%xmm1, 272(%rsp)
	movq	%rcx, 288(%rsp)
	movq	%rbp, 296(%rsp)
	movq	%r13, 304(%rsp)
	movl	%r12d, 312(%rsp)
	movl	%edx, 316(%rsp)
	movq	1040(%rsp), %rdx
	movq	%rax, 40(%rsp)
	cmpq	$-1, %rax
	je	.LBB2826_135
	movq	1032(%rsp), %rsi
	jmp	.LBB2826_136
.LBB2826_135:
	xorl	%esi, %esi
.LBB2826_136:
	movb	$1, %r14b
.Ltmp35815:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_validate::query::statistical_aggregates@GOTPCREL(%rip), %rax
	leaq	1320(%rsp), %rbx
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp35816:
	cmpq	$-1, 1048(%rsp)
	je	.LBB2826_139
	movq	1064(%rsp), %rdx
	movq	1056(%rsp), %rsi
.Ltmp35817:
	.cfi_escape 0x2e, 0x00
	leaq	608(%rsp), %rdi
	callq	<alloc::vec::Vec<alloc::string::String> as core::clone::Clone>::clone
.Ltmp35818:
	jmp	.LBB2826_140
.LBB2826_139:
	movq	$0, 608(%rsp)
	movq	$8, 616(%rsp)
	movq	$0, 624(%rsp)
.LBB2826_140:
	cmpq	$-1, 1072(%rsp)
	je	.LBB2826_143
	movq	1088(%rsp), %rdx
	movq	1080(%rsp), %rsi
.Ltmp35820:
	.cfi_escape 0x2e, 0x00
	leaq	1728(%rsp), %rdi
	callq	<alloc::vec::Vec<alloc::string::String> as core::clone::Clone>::clone
.Ltmp35821:
	movq	1728(%rsp), %rdx
	movq	1736(%rsp), %rax
	movq	1744(%rsp), %rcx
	jmp	.LBB2826_144
.LBB2826_143:
	movl	$8, %eax
	xorl	%ecx, %ecx
	xorl	%edx, %edx
.LBB2826_144:
	vmovups	608(%rsp), %xmm0
	movq	624(%rsp), %rsi
	movq	%rsi, 1568(%rsp)
	vmovaps	%xmm0, 1552(%rsp)
	movq	%rdx, 1576(%rsp)
	movq	%rax, 1584(%rsp)
	movq	%rcx, 1592(%rsp)
	movq	$0, 1600(%rsp)
	movq	$8, 1608(%rsp)
	movq	$0, 1616(%rsp)
.Ltmp35823:
	.cfi_escape 0x2e, 0x00
	leaq	1728(%rsp), %rdi
	leaq	1048(%rsp), %rsi
	callq	purrdf_native::py_store::query::build_engine
.Ltmp35824:
	movq	1016(%rsp), %rdx
	movq	272(%rsp), %r15
	movq	1320(%rsp), %r14
	movq	32(%rsp), %rax
	movq	1008(%rsp), %r8
	movq	%rdx, 80(%rsp)
	testq	%r15, %r15
	leaq	272(%rsp), %rdx
	movq	%rax, 64(%rsp)
	movq	%r8, 88(%rsp)
	cmoveq	%r15, %rdx
	testq	%r14, %r14
	cmoveq	%r14, %rbx
.Ltmp35826:
	.cfi_escape 0x2e, 0x00
	leaq	608(%rsp), %rdi
	leaq	1552(%rsp), %rsi
	movq	%rbx, %rcx
	callq	purrdf_native::py_store::env::extension_env
.Ltmp35827:
	vmovups	616(%rsp), %xmm0
	movq	608(%rsp), %rax
	movq	632(%rsp), %rcx
	movq	640(%rsp), %rbp
	movq	648(%rsp), %r13
	movl	656(%rsp), %r12d
	movl	660(%rsp), %edx
	movq	128(%rsp), %rsi
	vmovaps	%xmm0, 1264(%rsp)
	cmpq	$-1, %rax
	je	.LBB2826_149
	vmovups	728(%rsp), %zmm1
	vmovups	896(%rsp), %zmm0
	vmovups	664(%rsp), %zmm4
	vmovups	792(%rsp), %zmm2
	vmovups	856(%rsp), %zmm3
	movl	$3, 1312(%rsp)
	vmovups	%zmm1, 2376(%rsp)
	vmovaps	1264(%rsp), %xmm1
	vmovups	%zmm0, 2544(%rsp)
	vmovups	%zmm3, 2504(%rsp)
	vmovups	%zmm2, 2440(%rsp)
	vmovups	%zmm4, 2312(%rsp)
	vmovups	%xmm1, 2264(%rsp)
	movq	%rcx, 2280(%rsp)
	movq	%rbp, 2288(%rsp)
	movq	%r13, 2296(%rsp)
	movl	%r12d, 2304(%rsp)
	movl	%edx, 2308(%rsp)
	movq	%rax, 2256(%rsp)
	cmpb	$2, %sil
	jne	.LBB2826_155
	vmovups	.Lanon.5ac75c6ab29af51a474fa1a77778ee04.4+24(%rip), %zmm0
	vmovups	.Lanon.5ac75c6ab29af51a474fa1a77778ee04.4+8(%rip), %zmm1
	leaq	2256(%rsp), %rax
	vmovups	%zmm0, 1648(%rsp)
	vmovups	%zmm1, 1632(%rsp)
	movq	%rax, 1712(%rsp)
	movq	$8, 1720(%rsp)
	jmp	.LBB2826_167
.LBB2826_149:
	vmovaps	1264(%rsp), %xmm0
	movl	%edx, 24(%rsp)
	movq	%rcx, 8(%rsp)
	vmovaps	%xmm0, 144(%rsp)
.Ltmp35921:
	.cfi_escape 0x2e, 0x00
	leaq	1728(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
.Ltmp35922:
	testq	%r14, %r14
	je	.LBB2826_152
	xorl	%r14d, %r14d
.Ltmp35926:
	.cfi_escape 0x2e, 0x00
	leaq	1320(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
.Ltmp35927:
.LBB2826_152:
	testq	%r15, %r15
	je	.LBB2826_159
.Ltmp35931:
	.cfi_escape 0x2e, 0x00
	leaq	272(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::property_fn::PropertyFunction>)>>
.Ltmp35932:
	movq	16(%rsp), %r15
	xorl	%r14d, %r14d
.Ltmp35937:
	.cfi_escape 0x2e, 0x00
	leaq	304(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
.Ltmp35938:
	jmp	.LBB2826_117
.LBB2826_155:
	vmovups	.Lanon.5ac75c6ab29af51a474fa1a77778ee04.4+120(%rip), %zmm0
	vmovups	.Lanon.5ac75c6ab29af51a474fa1a77778ee04.4+8(%rip), %zmm1
	leaq	2256(%rsp), %rbx
	vmovups	%zmm0, 1480(%rsp)
	vmovups	.Lanon.5ac75c6ab29af51a474fa1a77778ee04.4+24(%rip), %zmm0
	movq	%rsi, 1360(%rsp)
	vmovups	%zmm1, 1368(%rsp)
	vmovups	%zmm0, 1384(%rsp)
	movq	%rbx, 1448(%rsp)
	movq	$8, 1456(%rsp)
	vxorps	%xmm0, %xmm0, %xmm0
	vmovups	%xmm0, 1464(%rsp)
.Ltmp35828:
	.cfi_escape 0x2e, 0x00
	movq	88(%rsp), %r14
	movq	80(%rsp), %r15
	movq	<purrdf_sparql_eval::engine::AdmittedSubstitutions>::requested@GOTPCREL(%rip), %rax
	leaq	608(%rsp), %rdi
	movl	$8, %ecx
	xorl	%r8d, %r8d
	movq	%r14, %rsi
	movq	%r15, %rdx
	vzeroupper
	callq	*%rax
.Ltmp35829:
	leaq	624(%rsp), %rax
.Ltmp35831:
	.cfi_escape 0x2e, 0x10
	movq	24(%rsp), %rdx
	movq	<purrdf_sparql_eval::engine::NativeSparqlEngine>::prepare_request@GOTPCREL(%rip), %r10
	movq	8(%rsp), %rcx
	leaq	144(%rsp), %rdi
	leaq	1728(%rsp), %rsi
	xorl	%r8d, %r8d
	pushq	%rax
	.cfi_adjust_cfa_offset 8
	pushq	%rbx
	.cfi_adjust_cfa_offset 8
	callq	*%r10
	addq	$16, %rsp
	.cfi_adjust_cfa_offset -16
.Ltmp35832:
	movq	144(%rsp), %rax
	movq	152(%rsp), %rbx
	cmpq	$-1, %rax
	je	.LBB2826_160
	vmovups	176(%rsp), %zmm1
	vmovups	160(%rsp), %zmm0
	vmovups	%zmm1, 1664(%rsp)
	vmovups	%zmm0, 1648(%rsp)
	movq	%rax, 1632(%rsp)
	movq	%rbx, 1640(%rsp)
	jmp	.LBB2826_163
.LBB2826_159:
	movq	16(%rsp), %r15
	xorl	%r14d, %r14d
	jmp	.LBB2826_117
.LBB2826_160:
	movq	64(%rsp), %rdx
	movq	%rbx, 136(%rsp)
	leaq	16(%rbx), %rcx
	addq	$16, %rdx
.Ltmp35833:
	.cfi_escape 0x2e, 0x10
	subq	$8, %rsp
	.cfi_adjust_cfa_offset 8
	leaq	1368(%rsp), %rax
	leaq	1640(%rsp), %rdi
	leaq	1736(%rsp), %rsi
	movq	%r14, %r8
	movq	%r15, %r9
	pushq	%rax
	.cfi_adjust_cfa_offset 8
	callq	<purrdf_sparql_eval::engine::NativeSparqlEngine>::query_prepared_admitted::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::dataset_view::NoopReservation<!>>
	addq	$16, %rsp
	.cfi_adjust_cfa_offset -16
.Ltmp35834:
	lock		decq	(%rbx)
	jne	.LBB2826_163
	#MEMBARRIER
.Ltmp35839:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::engine::PreparedQuery>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	136(%rsp), %rdi
	callq	*%rax
.Ltmp35840:
.LBB2826_163:
	movq	624(%rsp), %rsi
	cmpq	$10, %rsi
	jb	.LBB2826_165
	movq	632(%rsp), %rdi
	shlq	$4, %rsi
	addq	$-16, %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBB2826_165:
	movq	760(%rsp), %rsi
	cmpq	$10, %rsi
	jb	.LBB2826_167
	movq	768(%rsp), %rdi
	shlq	$4, %rsi
	addq	$-16, %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBB2826_167:
	vmovups	1632(%rsp), %zmm0
	vmovups	1664(%rsp), %zmm1
	vmovups	%zmm0, 608(%rsp)
	vmovups	%zmm1, 640(%rsp)
	movl	1312(%rsp), %eax
	testl	%eax, %eax
	je	.LBB2826_174
.LBB2826_168:
	movq	640(%rsp), %rax
	vmovaps	624(%rsp), %xmm0
	vmovups	672(%rsp), %ymm1
	movq	608(%rsp), %r14
	movq	616(%rsp), %rbx
	movq	648(%rsp), %rbp
	movq	656(%rsp), %r13
	movl	664(%rsp), %r12d
	movq	%rax, 8(%rsp)
	movl	668(%rsp), %eax
	vmovaps	%xmm0, 1248(%rsp)
	vmovups	%ymm1, 2784(%rsp)
	movl	%eax, 24(%rsp)
.Ltmp35882:
	.cfi_escape 0x2e, 0x00
	leaq	1288(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::user_fn::RefusalPublication>
.Ltmp35883:
	cmpq	$-1, %r14
	je	.LBB2826_186
	vmovaps	1248(%rsp), %xmm0
	movq	8(%rsp), %rax
	movq	%rbx, 616(%rsp)
	movl	24(%rsp), %r8d
	leaq	608(%rsp), %rcx
	movq	%rcx, 1360(%rsp)
	vmovups	%xmm0, 624(%rsp)
	vmovups	2784(%rsp), %ymm0
	movq	%rax, 640(%rsp)
	movq	<purrdf_core::diagnostic::RdfDiagnostic as core::fmt::Display>::fmt@GOTPCREL(%rip), %rax
	movq	%rbp, 648(%rsp)
	movq	%r13, 656(%rsp)
	movl	%r12d, 664(%rsp)
	movl	%r8d, 668(%rsp)
	movq	%rax, 1368(%rsp)
	vmovups	%ymm0, 672(%rsp)
	movq	%r14, 608(%rsp)
.Ltmp35884:
	.cfi_escape 0x2e, 0x00
	movq	alloc::fmt::format::format_inner@GOTPCREL(%rip), %rax
	leaq	.Lalloc_592fabd2ffa0e5a6b6713c88c8b00c99(%rip), %rsi
	leaq	144(%rsp), %rdi
	leaq	1360(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp35885:
	movq	688(%rsp), %rbx
	movq	616(%rsp), %rdi
	movq	624(%rsp), %rsi
.Ltmp35886:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_validate::xpath_regex::diagnostic_refusal_code@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp35887:
	testq	%rbx, %rbx
	movq	%rdx, %rcx
	setne	%dl
	testq	%rax, %rax
	sete	%sil
	orb	%dl, %sil
	cmpb	$1, %sil
	jne	.LBB2826_187
.Ltmp35891:
	.cfi_escape 0x2e, 0x00
	leaq	1360(%rsp), %rdi
	leaq	144(%rsp), %rsi
	movq	%rbx, %rdx
	callq	purrdf_native::py_store::presentation::presented_value_error
.Ltmp35892:
	jmp	.LBB2826_188
.LBB2826_174:
	movq	1304(%rsp), %r14
	movl	$1, %ecx
	xorl	%eax, %eax
	lock		cmpxchgl	%ecx, 32(%r14)
	leaq	32(%r14), %rbx
	jne	.LBB2826_213
.LBB2826_175:
	movq	std::panicking::panic_count::GLOBAL_PANIC_COUNT@GOTPCREL(%rip), %r15
	movq	(%r15), %rax
	shlq	%rax
	testq	%rax, %rax
	jne	.LBB2826_214
	movzbl	36(%r14), %eax
	vmovups	40(%r14), %ymm0
	leaq	36(%r14), %r12
	vmovups	%ymm0, 144(%rsp)
	movq	$0, 40(%r14)
	movq	(%r15), %rax
	shlq	%rax
	testq	%rax, %rax
	jne	.LBB2826_217
.LBB2826_177:
	xorl	%eax, %eax
	xchgl	%eax, (%rbx)
	cmpl	$2, %eax
	je	.LBB2826_220
.LBB2826_178:
	cmpq	$-1, 608(%rsp)
	movq	144(%rsp), %rax
	je	.LBB2826_204
	testq	%rax, %rax
	je	.LBB2826_204
	vmovups	144(%rsp), %ymm0
	movq	1304(%rsp), %rax
	vmovups	%ymm0, 1360(%rsp)
	movq	16(%rax), %rcx
	movq	24(%rax), %rax
	movq	16(%rax), %rdx
	movq	32(%rax), %rax
	decq	%rdx
	andq	$-16, %rdx
	leaq	16(%rcx,%rdx), %rdi
.Ltmp35853:
	.cfi_escape 0x2e, 0x00
	leaq	1360(%rsp), %rsi
	vzeroupper
	callq	*%rax
.Ltmp35854:
	movq	1360(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB2826_183
	#MEMBARRIER
.Ltmp35858:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn core::error::Error + core::marker::Sync + core::marker::Send>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1360(%rsp), %rdi
	callq	*%rax
.Ltmp35859:
.LBB2826_183:
	movq	1376(%rsp), %rax
	testq	%rax, %rax
	je	.LBB2826_168
	lock		decq	(%rax)
	jne	.LBB2826_168
	leaq	1376(%rsp), %rdi
	#MEMBARRIER
.Ltmp35864:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn purrdf_sparql_eval::user_fn::UserFunctionAdmission>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp35865:
	jmp	.LBB2826_168
.LBB2826_186:
	vmovaps	1248(%rsp), %xmm0
	movq	8(%rsp), %rax
	movl	24(%rsp), %ecx
	jmp	.LBB2826_190
.LBB2826_187:
.Ltmp35889:
	.cfi_escape 0x2e, 0x00
	leaq	1360(%rsp), %rdi
	leaq	144(%rsp), %rsi
	movq	%rax, %rdx
	callq	purrdf_native::py_store::presentation::refusal_value_error
.Ltmp35890:
.LBB2826_188:
.Ltmp35897:
	.cfi_escape 0x2e, 0x00
	leaq	608(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.Ltmp35898:
	vmovups	1360(%rsp), %xmm0
	movq	1376(%rsp), %rax
	movq	1384(%rsp), %rbp
	movq	1392(%rsp), %r13
	movl	1400(%rsp), %r12d
	movl	1404(%rsp), %ecx
	movq	$-1, %rbx
.LBB2826_190:
	vmovaps	%xmm0, 144(%rsp)
.Ltmp35902:
	movl	%ecx, 24(%rsp)
	movq	%rax, 8(%rsp)
	.cfi_escape 0x2e, 0x00
	leaq	2256(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::extension_env::ExtensionEnv>
.Ltmp35903:
.Ltmp35907:
	.cfi_escape 0x2e, 0x00
	leaq	1728(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
.Ltmp35908:
	cmpq	$0, 1320(%rsp)
	je	.LBB2826_195
	xorl	%r14d, %r14d
.Ltmp35909:
	.cfi_escape 0x2e, 0x00
	leaq	1320(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
.Ltmp35910:
.LBB2826_195:
	cmpq	$0, 272(%rsp)
	movq	16(%rsp), %r15
	je	.LBB2826_198
.Ltmp35911:
	.cfi_escape 0x2e, 0x00
	leaq	272(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::property_fn::PropertyFunction>)>>
.Ltmp35912:
	xorl	%r14d, %r14d
.Ltmp35917:
	.cfi_escape 0x2e, 0x00
	leaq	304(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
.Ltmp35918:
.LBB2826_198:
	movq	32(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB2826_200
	xorl	%r14d, %r14d
	#MEMBARRIER
.Ltmp35919:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	32(%rsp), %rdi
	callq	*%rax
.Ltmp35920:
.LBB2826_200:
	movq	40(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB2826_125
	movq	8(%rsp), %r14
	testq	%rsi, %rsi
	je	.LBB2826_126
	movq	1032(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
	jmp	.LBB2826_126
.LBB2826_204:
	testq	%rax, %rax
	je	.LBB2826_168
	lock		decq	(%rax)
	jne	.LBB2826_207
	#MEMBARRIER
.Ltmp35867:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn core::error::Error + core::marker::Sync + core::marker::Send>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	144(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp35868:
.LBB2826_207:
	movq	160(%rsp), %rax
	testq	%rax, %rax
	je	.LBB2826_168
	lock		decq	(%rax)
	jne	.LBB2826_168
	leaq	160(%rsp), %rdi
	#MEMBARRIER
.Ltmp35873:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn purrdf_sparql_eval::user_fn::UserFunctionAdmission>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp35874:
	jmp	.LBB2826_168
.LBB2826_210:
.Ltmp36002:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::instance::Py<_> as core::ops::drop::Drop>::drop::drop_slow@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp36003:
	jmp	.LBB2826_112
.LBB2826_211:
.Ltmp35805:
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$24, %esi
	callq	*%rax
.Ltmp35806:
	ud2
.LBB2826_213:
.Ltmp35842:
	.cfi_escape 0x2e, 0x00
	movq	<std::sys::sync::mutex::futex::Mutex>::lock_contended@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.Ltmp35843:
	jmp	.LBB2826_175
.LBB2826_214:
.Ltmp35844:
	.cfi_escape 0x2e, 0x00
	movq	std::panicking::panic_count::is_zero_slow_path@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp35845:
	movzbl	36(%r14), %ecx
	vmovups	40(%r14), %ymm0
	vmovups	%ymm0, 144(%rsp)
	movq	$0, 40(%r14)
	testb	%al, %al
	je	.LBB2826_177
	addq	$36, %r14
	movq	%r14, %r12
	movq	(%r15), %rax
	shlq	%rax
	testq	%rax, %rax
	je	.LBB2826_177
.LBB2826_217:
.Ltmp35846:
	.cfi_escape 0x2e, 0x00
	movq	std::panicking::panic_count::is_zero_slow_path@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp35847:
	testb	%al, %al
	jne	.LBB2826_177
	movb	$1, (%r12)
	jmp	.LBB2826_177
.LBB2826_220:
.Ltmp35848:
	.cfi_escape 0x2e, 0x00
	movq	<std::sys::sync::mutex::futex::Mutex>::wake@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.Ltmp35849:
	jmp	.LBB2826_178
.LBB2826_221:
.Ltmp35850:
	cmpq	$0, 144(%rsp)
	movq	%rax, %r15
	je	.LBB2826_238
.Ltmp35851:
	.cfi_escape 0x2e, 0x00
	leaq	144(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::user_fn::UserFunctionRefusal>
.Ltmp35852:
	jmp	.LBB2826_238
.LBB2826_223:
.Ltmp36004:
	movq	%rax, %r15
	jmp	.LBB2826_292
.LBB2826_224:
.Ltmp35875:
	movq	%rax, %r15
	jmp	.LBB2826_238
.LBB2826_225:
.Ltmp35866:
	movq	%rax, %r15
	jmp	.LBB2826_238
.LBB2826_226:
.Ltmp35869:
	movq	%rax, %r15
	movq	160(%rsp), %rax
	testq	%rax, %rax
	je	.LBB2826_238
	lock		decq	(%rax)
	jne	.LBB2826_238
	leaq	160(%rsp), %rdi
	#MEMBARRIER
.Ltmp35870:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn purrdf_sparql_eval::user_fn::UserFunctionAdmission>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp35871:
	jmp	.LBB2826_238
.LBB2826_229:
.Ltmp35872:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2826_230:
.Ltmp35860:
	movq	%rax, %r15
	movq	1376(%rsp), %rax
	testq	%rax, %rax
	je	.LBB2826_238
	lock		decq	(%rax)
	jne	.LBB2826_238
	leaq	1376(%rsp), %rdi
	#MEMBARRIER
.Ltmp35861:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn purrdf_sparql_eval::user_fn::UserFunctionAdmission>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp35862:
	jmp	.LBB2826_238
.LBB2826_233:
.Ltmp35863:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2826_234:
.Ltmp35835:
	lock		decq	(%rbx)
	movq	%rax, %r15
	jne	.LBB2826_245
	#MEMBARRIER
.Ltmp35836:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::engine::PreparedQuery>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	136(%rsp), %rdi
	callq	*%rax
.Ltmp35837:
	jmp	.LBB2826_245
.LBB2826_236:
.Ltmp35838:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2826_237:
.Ltmp35855:
	movq	%rax, %r15
.Ltmp35856:
	.cfi_escape 0x2e, 0x00
	leaq	1360(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::user_fn::UserFunctionRefusal>
.Ltmp35857:
.LBB2826_238:
.Ltmp35876:
	.cfi_escape 0x2e, 0x00
	leaq	608(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::result::Result<purrdf_core::backend::SparqlResult, purrdf_core::diagnostic::RdfDiagnostic>>
.Ltmp35877:
	jmp	.LBB2826_246
.LBB2826_239:
.Ltmp35878:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2826_240:
.Ltmp35933:
	movq	%rax, %r15
.Ltmp35934:
	.cfi_escape 0x2e, 0x00
	leaq	304(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
.Ltmp35935:
	jmp	.LBB2826_248
.LBB2826_242:
.Ltmp35936:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2826_243:
.Ltmp35830:
	movq	%rax, %r15
	jmp	.LBB2826_246
.LBB2826_244:
.Ltmp35841:
	movq	%rax, %r15
.LBB2826_245:
	.cfi_escape 0x2e, 0x00
	leaq	608(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::AdmittedSubstitutions>
.LBB2826_246:
.Ltmp35879:
	.cfi_escape 0x2e, 0x00
	leaq	1288(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::user_fn::RefusalPublication>
.Ltmp35880:
	jmp	.LBB2826_263
.LBB2826_247:
.Ltmp35913:
	movq	%rax, %r15
.Ltmp35914:
	.cfi_escape 0x2e, 0x00
	leaq	304(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
.Ltmp35915:
.LBB2826_248:
	xorl	%r14d, %r14d
	jmp	.LBB2826_272
.LBB2826_249:
.Ltmp35916:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2826_250:
.Ltmp35888:
	movq	144(%rsp), %rsi
	movq	%rax, %r15
	testq	%rsi, %rsi
	je	.LBB2826_257
	movq	152(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
	jmp	.LBB2826_257
.LBB2826_252:
.Ltmp35881:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2826_253:
.Ltmp35822:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	leaq	608(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::string::String>>
	jmp	.LBB2826_255
.LBB2826_254:
.Ltmp35819:
	movq	%rax, %r15
.LBB2826_255:
	movb	$1, %r14b
	jmp	.LBB2826_267
.LBB2826_256:
.Ltmp35893:
	movq	%rax, %r15
.LBB2826_257:
.Ltmp35894:
	.cfi_escape 0x2e, 0x00
	leaq	608(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.Ltmp35895:
	jmp	.LBB2826_263
.LBB2826_258:
.Ltmp35896:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2826_259:
.Ltmp35923:
	movq	%rax, %r15
	jmp	.LBB2826_266
.LBB2826_260:
.Ltmp35944:
	movq	%rax, %r15
	jmp	.LBB2826_322
.LBB2826_261:
.Ltmp35825:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	leaq	1552(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_algebra::parser::ParserOptions>
	jmp	.LBB2826_266
.LBB2826_262:
.Ltmp35899:
	movq	%rax, %r15
.LBB2826_263:
.Ltmp35900:
	.cfi_escape 0x2e, 0x00
	leaq	2256(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::extension_env::ExtensionEnv>
.Ltmp35901:
	jmp	.LBB2826_265
.LBB2826_264:
.Ltmp35904:
	movq	%rax, %r15
.LBB2826_265:
.Ltmp35905:
	.cfi_escape 0x2e, 0x00
	leaq	1728(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
.Ltmp35906:
.LBB2826_266:
	xorl	%r14d, %r14d
.LBB2826_267:
	cmpq	$0, 1320(%rsp)
	je	.LBB2826_270
.Ltmp35924:
	.cfi_escape 0x2e, 0x00
	leaq	1320(%rsp), %rdi
	callq	core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
.Ltmp35925:
	jmp	.LBB2826_270
.LBB2826_269:
.Ltmp35928:
	movq	%rax, %r15
.LBB2826_270:
.Ltmp35929:
	.cfi_escape 0x2e, 0x00
	leaq	272(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry>>
.Ltmp35930:
	jmp	.LBB2826_272
.LBB2826_271:
.Ltmp35939:
	movq	%rax, %r15
.LBB2826_272:
	movq	32(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB2826_322
	#MEMBARRIER
.Ltmp35940:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	32(%rsp), %rdi
	callq	*%rax
.Ltmp35941:
	jmp	.LBB2826_322
.LBB2826_274:
.Ltmp35793:
	movq	%rax, %r15
	jmp	.LBB2826_320
.LBB2826_275:
.Ltmp36001:
	movq	8(%r14), %rsi
	movq	%rax, %r15
	testq	%rsi, %rsi
	je	.LBB2826_292
	movq	16(%r14), %rdx
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	callq	*%rax
	jmp	.LBB2826_292
.LBB2826_277:
.Ltmp35950:
	movq	%rax, %r15
	jmp	.LBB2826_328
.LBB2826_278:
.Ltmp35786:
	movq	%rax, %r15
.Ltmp35787:
	.cfi_escape 0x2e, 0x00
	leaq	976(%rsp), %rdi
	callq	core::ptr::drop_glue::<<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}::{closure#0}>
.Ltmp35788:
	jmp	.LBB2826_282
.LBB2826_279:
.Ltmp35781:
	movq	%rax, %r15
	leaq	456(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
	leaq	432(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	leaq	408(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
.Ltmp35782:
	.cfi_escape 0x2e, 0x00
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>
.Ltmp35783:
	jmp	.LBB2826_302
.LBB2826_281:
.Ltmp35958:
	movq	%rax, %r15
.LBB2826_282:
	movl	$0, 40(%rsp)
	xorl	%r12d, %r12d
	jmp	.LBB2826_329
.LBB2826_283:
.Ltmp35796:
	movq	%rax, %r15
	jmp	.LBB2826_321
.LBB2826_284:
.Ltmp35778:
	movb	$1, %r12b
	movq	%rax, %r15
	jmp	.LBB2826_303
.LBB2826_285:
.Ltmp35981:
	jmp	.LBB2826_290
.LBB2826_286:
.Ltmp35992:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	movq	%r12, %rdi
	callq	*%r13
	jmp	.LBB2826_291
.LBB2826_287:
.Ltmp35978:
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	movq	_Py_DecRef@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
	jmp	.LBB2826_291
.LBB2826_288:
.Ltmp35973:
	movq	%rax, %r15
.Ltmp35974:
	.cfi_escape 0x2e, 0x00
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<pyo3::err::PyErr>
.Ltmp35975:
	jmp	.LBB2826_337
.LBB2826_289:
.Ltmp35995:
.LBB2826_290:
	movq	%rax, %r15
.LBB2826_291:
.Ltmp35996:
	.cfi_escape 0x2e, 0x00
	leaq	2256(%rsp), %rdi
	callq	core::ptr::drop_glue::<pyo3::err::PyErr>
.Ltmp35997:
.LBB2826_292:
.Ltmp36005:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::AttachGuard as core::ops::drop::Drop>::drop@GOTPCREL(%rip), %rax
	leaq	976(%rsp), %rdi
	callq	*%rax
.Ltmp36006:
	jmp	.LBB2826_337
.LBB2826_293:
.Ltmp36007:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2826_294:
.Ltmp35998:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2826_295:
.Ltmp35961:
	movq	%rax, %r15
	testq	%r12, %r12
	je	.LBB2826_299
	negq	%r12
	addq	$160, %r14
	.p2align	4
.LBB2826_297:
.Ltmp35962:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.Ltmp35963:
	addq	$160, %r14
	decq	%r12
	jne	.LBB2826_297
.LBB2826_299:
	movq	96(%rsp), %rax
	testq	%rax, %rax
	je	.LBB2826_302
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	callq	*%rax
.LBB2826_302:
	xorl	%r12d, %r12d
.LBB2826_303:
.Ltmp35965:
	.cfi_escape 0x2e, 0x00
	leaq	240(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp35966:
	jmp	.LBB2826_329
.LBB2826_304:
.Ltmp35964:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2826_305:
.Ltmp35967:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2826_306:
.Ltmp35799:
	movq	%rax, %r15
	testq	%r12, %r12
	je	.LBB2826_310
	negq	%r12
	addq	$160, %r14
.LBB2826_308:
.Ltmp35800:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
.Ltmp35801:
	addq	$160, %r14
	decq	%r12
	jne	.LBB2826_308
.LBB2826_310:
	movq	976(%rsp), %rax
	testq	%rax, %rax
	je	.LBB2826_312
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	callq	*%rax
.LBB2826_312:
	movq	1024(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB2826_315
	testq	%rsi, %rsi
	je	.LBB2826_315
	movq	1032(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBB2826_315:
	leaq	1048(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
.Ltmp35803:
	.cfi_escape 0x2e, 0x00
	leaq	1000(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp35804:
	jmp	.LBB2826_328
.LBB2826_316:
.Ltmp35802:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2826_317:
.Ltmp35970:
	movq	%rax, %r15
	jmp	.LBB2826_329
.LBB2826_318:
.Ltmp35807:
	movq	%rax, %r15
	testq	%rbx, %rbx
	je	.LBB2826_320
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	movq	%r14, %rdi
	movq	%rbx, %rsi
	callq	*%rax
.LBB2826_320:
.Ltmp35808:
	.cfi_escape 0x2e, 0x00
	leaq	1728(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
.Ltmp35809:
.LBB2826_321:
	movb	$1, %r14b
.Ltmp35811:
	.cfi_escape 0x2e, 0x00
	leaq	976(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>
.Ltmp35812:
.LBB2826_322:
	movq	1024(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB2826_325
	testq	%rsi, %rsi
	je	.LBB2826_325
	movq	1032(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBB2826_325:
	testb	%r14b, %r14b
	je	.LBB2826_327
	leaq	1048(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
.LBB2826_327:
.Ltmp35945:
	.cfi_escape 0x2e, 0x00
	leaq	1000(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
.Ltmp35946:
.LBB2826_328:
	movl	$0, 40(%rsp)
.Ltmp35951:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::internal::state::SuspendAttach as core::ops::drop::Drop>::drop@GOTPCREL(%rip), %rax
	leaq	368(%rsp), %rdi
	callq	*%rax
.Ltmp35952:
	xorl	%r12d, %r12d
.LBB2826_329:
	cmpq	$0, 48(%rsp)
	jle	.LBB2826_331
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	72(%rsp), %rdi
	movq	48(%rsp), %rsi
	movl	$1, %edx
	callq	*%rax
.LBB2826_331:
	testb	%r12b, %r12b
	je	.LBB2826_333
	leaq	408(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	leaq	432(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	leaq	456(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
.LBB2826_333:
	cmpb	$0, 40(%rsp)
	je	.LBB2826_337
	movq	504(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB2826_337
	testq	%rsi, %rsi
	je	.LBB2826_337
	movq	512(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBB2826_337:
	.cfi_escape 0x2e, 0x00
	movq	%r15, %rdi
	callq	_Unwind_Resume@PLT
.LBB2826_338:
.Ltmp35953:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2826_339:
.Ltmp35947:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2826_340:
.Ltmp35810:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end2826:
<purrdf_native::py_store::quad_store::PyQuadStore>::__pymethod_query__:
.Lfunc_begin1760:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception1760
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
	jne	.LBB2894_2
	vmovups	24(%rsp), %ymm1
	vmovups	8(%rsp), %ymm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%ymm0, 8(%r14)
	movq	$1, (%r14)
	jmp	.LBB2894_117
.LBB2894_2:
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
	je	.LBB2894_4
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
	jmp	.LBB2894_117
.LBB2894_4:
	movq	480(%rsp), %rsi
	addq	$16, %r15
	movq	%r15, 632(%rsp)
	movq	$0, 624(%rsp)
.Ltmp37591:
	.cfi_escape 0x2e, 0x00
	movq	<&str as pyo3::conversion::FromPyObject>::extract@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	callq	*%rax
.Ltmp37592:
	cmpl	$1, (%rsp)
	je	.LBB2894_118
	movq	8(%rsp), %r13
	movq	16(%rsp), %rbp
	movq	488(%rsp), %r12
	testq	%r12, %r12
	je	.LBB2894_14
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB2894_14
	movq	8(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movq	PyType_GetFlags@GOTPCREL(%rip), %rax
	callq	*%rax
	testl	$536870912, %eax
	jne	.LBB2894_20
	.cfi_escape 0x2e, 0x00
	movq	PyDict_Type@GOTPCREL(%rip), %r15
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.Ltmp37593:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r15, %rdx
	callq	*%rax
.Ltmp37594:
	leaq	8(%rsp), %rdi
.Ltmp37595:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_f98cafa4d6296fc0caeb65b9f5347e73(%rip), %rsi
	leaq	56(%rsp), %rcx
	movl	$13, %edx
	callq	*%rax
.Ltmp37596:
	vmovups	16(%rsp), %ymm0
	movq	48(%rsp), %rcx
	movq	8(%rsp), %rax
	movq	%rcx, 48(%r14)
	vmovups	%ymm0, 16(%r14)
	movq	%rax, 8(%r14)
.LBB2894_12:
	movq	$1, (%r14)
	lock		decq	(%rbx)
	jmp	.LBB2894_117
.LBB2894_14:
	xorl	%eax, %eax
	movq	496(%rsp), %r12
	movq	%rax, 392(%rsp)
	testq	%r12, %r12
	je	.LBB2894_21
.LBB2894_15:
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB2894_22
.Ltmp37597:
	.cfi_escape 0x2e, 0x00
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	<alloc::vec::Vec<alloc::string::String> as pyo3::conversion::FromPyObject>::extract
.Ltmp37598:
	movq	8(%rsp), %rax
	cmpb	$0, (%rsp)
	leaq	16(%rsp), %rcx
	je	.LBB2894_23
	vmovups	(%rcx), %ymm0
	movq	32(%rcx), %rdx
	leaq	64(%rsp), %rdi
	movq	%rdx, 144(%rsp)
	movq	%rdx, 40(%rsp)
	vmovups	%ymm0, 8(%rsp)
	vmovups	%ymm0, 112(%rsp)
	movq	%rax, (%rsp)
.Ltmp37599:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_908475b3e6cbafcd4d8e4fa6ac07f48c(%rip), %rsi
	movl	$20, %edx
	movq	%rsp, %rcx
	vzeroupper
	callq	*%rax
.Ltmp37600:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %ymm0
	vmovups	%ymm1, 192(%rsp)
	vmovups	%ymm0, 176(%rsp)
	vmovups	192(%rsp), %ymm1
	vmovups	176(%rsp), %ymm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%ymm0, 8(%r14)
	movq	$1, (%r14)
	jmp	.LBB2894_31
.LBB2894_20:
	leaq	224(%rsp), %rax
	movq	%r12, 224(%rsp)
	movq	496(%rsp), %r12
	movq	%rax, 392(%rsp)
	testq	%r12, %r12
	jne	.LBB2894_15
.LBB2894_21:
	movq	$-1, 64(%rsp)
	jmp	.LBB2894_25
.LBB2894_22:
	movq	$-1, %rax
	jmp	.LBB2894_24
.LBB2894_23:
	vmovups	(%rcx), %xmm0
	vmovaps	%xmm0, 112(%rsp)
.LBB2894_24:
	vmovaps	112(%rsp), %xmm0
	vmovups	%xmm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBB2894_25:
	movq	80(%rsp), %rax
	movq	64(%rsp), %rcx
	movq	72(%rsp), %rdx
	movq	504(%rsp), %r12
	movq	%rbx, 168(%rsp)
	movq	%rcx, 256(%rsp)
	movq	%rdx, 264(%rsp)
	movq	%rax, 272(%rsp)
	testq	%r12, %r12
	je	.LBB2894_32
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB2894_33
.Ltmp37601:
	.cfi_escape 0x2e, 0x00
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	<alloc::vec::Vec<alloc::string::String> as pyo3::conversion::FromPyObject>::extract
.Ltmp37602:
	movq	8(%rsp), %rax
	cmpb	$0, (%rsp)
	leaq	16(%rsp), %rcx
	je	.LBB2894_34
	vmovups	(%rcx), %ymm0
	movq	32(%rcx), %rdx
	leaq	64(%rsp), %rdi
	movq	%rdx, 144(%rsp)
	movq	%rdx, 40(%rsp)
	vmovups	%ymm0, 8(%rsp)
	vmovups	%ymm0, 112(%rsp)
	movq	%rax, (%rsp)
.Ltmp37603:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_ed816c88becfff4b944384e4ed1786c6(%rip), %rsi
	movl	$22, %edx
	movq	%rsp, %rcx
	vzeroupper
	callq	*%rax
.Ltmp37604:
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
	jne	.LBB2894_69
.LBB2894_31:
	lock		decq	(%rbx)
	jmp	.LBB2894_117
.LBB2894_32:
	movq	$-1, 64(%rsp)
	jmp	.LBB2894_36
.LBB2894_33:
	movq	$-1, %rax
	jmp	.LBB2894_35
.LBB2894_34:
	vmovups	(%rcx), %xmm0
	vmovaps	%xmm0, 112(%rsp)
.LBB2894_35:
	vmovaps	112(%rsp), %xmm0
	vmovups	%xmm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBB2894_36:
	movq	80(%rsp), %rax
	movq	64(%rsp), %rcx
	movq	72(%rsp), %rdx
	movq	512(%rsp), %r12
	movq	%rcx, 320(%rsp)
	movq	%rdx, 328(%rsp)
	movq	%rax, 336(%rsp)
	testq	%r12, %r12
	je	.LBB2894_42
	leaq	16(%rsp), %rbx
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB2894_43
.Ltmp37606:
	.cfi_escape 0x2e, 0x00
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	<(alloc::string::String, alloc::string::String) as pyo3::conversion::FromPyObject>::extract
.Ltmp37607:
	movq	8(%rsp), %rax
	cmpl	$1, (%rsp)
	jne	.LBB2894_44
	vmovups	(%rbx), %ymm0
	movq	32(%rbx), %rcx
	leaq	64(%rsp), %rdi
	movq	%rcx, 216(%rsp)
	vmovups	%ymm0, 184(%rsp)
	movq	%rax, 176(%rsp)
.Ltmp37608:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_93df691527a5f68f324bb7548b51c806(%rip), %rsi
	leaq	176(%rsp), %rcx
	movl	$21, %edx
	vzeroupper
	callq	*%rax
.Ltmp37609:
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
	jmp	.LBB2894_68
.LBB2894_42:
	movq	$-1, 64(%rsp)
	jmp	.LBB2894_45
.LBB2894_43:
	movq	$-1, %rax
.LBB2894_44:
	vmovups	(%rbx), %ymm0
	movq	32(%rbx), %rcx
	movq	168(%rsp), %rbx
	movq	%rcx, 104(%rsp)
	vmovups	%ymm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBB2894_45:
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
	je	.LBB2894_51
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB2894_51
	movq	8(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movq	PyType_GetFlags@GOTPCREL(%rip), %rax
	callq	*%rax
	testl	$536870912, %eax
	jne	.LBB2894_56
	.cfi_escape 0x2e, 0x00
	movq	PyDict_Type@GOTPCREL(%rip), %r15
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.Ltmp37611:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r15, %rdx
	callq	*%rax
.Ltmp37612:
	leaq	8(%rsp), %rdi
.Ltmp37613:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_9865433ad06614f46384a300efcb7195(%rip), %rsi
	leaq	56(%rsp), %rcx
	movl	$9, %edx
	callq	*%rax
.Ltmp37614:
	jmp	.LBB2894_65
.LBB2894_51:
	xorl	%eax, %eax
	movq	528(%rsp), %r12
	movq	%rax, 384(%rsp)
	testq	%r12, %r12
	jne	.LBB2894_52
	jmp	.LBB2894_58
.LBB2894_56:
	leaq	232(%rsp), %rax
	movq	%r12, 232(%rsp)
	movq	528(%rsp), %r12
	movq	%rax, 384(%rsp)
	testq	%r12, %r12
	je	.LBB2894_58
.LBB2894_52:
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB2894_58
	movq	8(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movq	PyType_GetFlags@GOTPCREL(%rip), %rax
	callq	*%rax
	testl	$536870912, %eax
	jne	.LBB2894_59
	.cfi_escape 0x2e, 0x00
	movq	PyDict_Type@GOTPCREL(%rip), %r15
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.Ltmp37615:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r15, %rdx
	callq	*%rax
.Ltmp37616:
	leaq	8(%rsp), %rdi
.Ltmp37617:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_fcd42a11584ddca09ee4bceee5bc0781(%rip), %rsi
	leaq	56(%rsp), %rcx
	movl	$20, %edx
	callq	*%rax
.Ltmp37618:
	jmp	.LBB2894_65
.LBB2894_58:
	xorl	%eax, %eax
	movq	536(%rsp), %r12
	movq	%rax, 376(%rsp)
	testq	%r12, %r12
	jne	.LBB2894_61
	jmp	.LBB2894_78
.LBB2894_59:
	leaq	240(%rsp), %rax
	movq	%r12, 240(%rsp)
	movq	536(%rsp), %r12
	movq	%rax, 376(%rsp)
	testq	%r12, %r12
	je	.LBB2894_78
.LBB2894_61:
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB2894_78
	movq	8(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movq	PyType_GetFlags@GOTPCREL(%rip), %rax
	callq	*%rax
	testl	$536870912, %eax
	jne	.LBB2894_79
	.cfi_escape 0x2e, 0x00
	movq	PyDict_Type@GOTPCREL(%rip), %r15
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.Ltmp37619:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r15, %rdx
	callq	*%rax
.Ltmp37620:
	leaq	8(%rsp), %rdi
.Ltmp37621:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_2f742b7bf960932f7bae3af5028d09e0(%rip), %rsi
	leaq	56(%rsp), %rcx
	movl	$14, %edx
	callq	*%rax
.Ltmp37622:
.LBB2894_65:
	vmovups	16(%rsp), %ymm0
	movq	48(%rsp), %rcx
	movq	8(%rsp), %rax
	movq	%rcx, 48(%r14)
	vmovups	%ymm0, 16(%r14)
	movq	%rax, 8(%r14)
.LBB2894_66:
	movq	$1, (%r14)
.LBB2894_67:
	.cfi_escape 0x2e, 0x00
	leaq	400(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
.LBB2894_68:
	.cfi_escape 0x2e, 0x00
	leaq	320(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	movq	256(%rsp), %r12
	cmpq	$-1, %r12
	je	.LBB2894_31
.LBB2894_69:
	movq	264(%rsp), %r15
	movq	272(%rsp), %r13
	testq	%r13, %r13
	je	.LBB2894_74
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rbx
	leaq	8(%r15), %rbp
	jmp	.LBB2894_72
	.p2align	4
.LBB2894_71:
	addq	$24, %rbp
	decq	%r13
	je	.LBB2894_74
.LBB2894_72:
	movq	-8(%rbp), %rsi
	testq	%rsi, %rsi
	je	.LBB2894_71
	movq	(%rbp), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%rbx
	jmp	.LBB2894_71
.LBB2894_74:
	testq	%r12, %r12
	je	.LBB2894_76
	shlq	$3, %r12
	leaq	(%r12,%r12,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%r15, %rdi
	vzeroupper
	callq	*%rax
.LBB2894_76:
	movq	168(%rsp), %rax
	lock		decq	(%rax)
	jmp	.LBB2894_117
.LBB2894_78:
	xorl	%eax, %eax
	movq	544(%rsp), %r12
	movq	%rax, 368(%rsp)
	testq	%r12, %r12
	je	.LBB2894_86
.LBB2894_81:
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB2894_87
.Ltmp37623:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::string::String as pyo3::conversion::FromPyObject>::extract@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	*%rax
.Ltmp37624:
	movq	8(%rsp), %rax
	cmpb	$0, (%rsp)
	leaq	16(%rsp), %rcx
	je	.LBB2894_88
	vmovups	(%rcx), %ymm0
	movq	32(%rcx), %rdx
	leaq	64(%rsp), %rdi
	movq	%rdx, 144(%rsp)
	movq	%rdx, 40(%rsp)
	vmovups	%ymm0, 8(%rsp)
	vmovups	%ymm0, 112(%rsp)
	movq	%rax, (%rsp)
.Ltmp37625:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_589d20955c603729090b6aaccd02222f(%rip), %rsi
	movl	$19, %edx
	movq	%rsp, %rcx
	vzeroupper
	callq	*%rax
.Ltmp37626:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %ymm0
	vmovups	%ymm1, 192(%rsp)
	vmovups	%ymm0, 176(%rsp)
	vmovups	192(%rsp), %ymm1
	vmovups	176(%rsp), %ymm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%ymm0, 8(%r14)
	jmp	.LBB2894_66
.LBB2894_79:
	leaq	248(%rsp), %rax
	movq	%r12, 248(%rsp)
	movq	544(%rsp), %r12
	movq	%rax, 368(%rsp)
	testq	%r12, %r12
	jne	.LBB2894_81
.LBB2894_86:
	movq	$-1, 64(%rsp)
	jmp	.LBB2894_90
.LBB2894_87:
	movq	$-1, %rax
	jmp	.LBB2894_89
.LBB2894_88:
	vmovups	(%rcx), %xmm0
	vmovaps	%xmm0, 112(%rsp)
.LBB2894_89:
	vmovaps	112(%rsp), %xmm0
	vmovups	%xmm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBB2894_90:
	movq	80(%rsp), %rax
	movq	64(%rsp), %rcx
	movq	72(%rsp), %rdx
	movq	552(%rsp), %r12
	movq	%r13, 360(%rsp)
	movq	%rcx, 288(%rsp)
	movq	%rdx, 296(%rsp)
	movq	%rax, 304(%rsp)
	testq	%r12, %r12
	je	.LBB2894_96
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	xorl	%r13d, %r13d
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB2894_97
.Ltmp37628:
	.cfi_escape 0x2e, 0x00
	movq	<&str as pyo3::conversion::FromPyObject>::extract@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	*%rax
.Ltmp37629:
	movq	8(%rsp), %r13
	movq	16(%rsp), %rax
	cmpb	$0, (%rsp)
	je	.LBB2894_98
	vmovups	24(%rsp), %ymm0
	leaq	64(%rsp), %rdi
	vmovups	%ymm0, 192(%rsp)
	movq	%r13, 176(%rsp)
	movq	%rax, 184(%rsp)
.Ltmp37630:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_6c43a98927907a5f8539235d09232c7e(%rip), %rsi
	leaq	176(%rsp), %rcx
	movl	$11, %edx
	vzeroupper
	callq	*%rax
.Ltmp37631:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %xmm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%xmm0, 8(%r14)
	jmp	.LBB2894_105
.LBB2894_96:
	xorl	%r13d, %r13d
	jmp	.LBB2894_99
.LBB2894_97:
	jmp	.LBB2894_99
.LBB2894_98:
	movq	%rax, 352(%rsp)
.LBB2894_99:
	movq	560(%rsp), %r12
	testq	%r12, %r12
	je	.LBB2894_108
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB2894_109
.Ltmp37632:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::string::String as pyo3::conversion::FromPyObject>::extract@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	*%rax
.Ltmp37633:
	movq	8(%rsp), %rax
	cmpb	$0, (%rsp)
	leaq	16(%rsp), %rcx
	je	.LBB2894_110
	vmovups	(%rcx), %ymm0
	movq	32(%rcx), %rdx
	leaq	64(%rsp), %rdi
	movq	%rdx, 144(%rsp)
	movq	%rdx, 40(%rsp)
	vmovups	%ymm0, 8(%rsp)
	vmovups	%ymm0, 112(%rsp)
	movq	%rax, (%rsp)
.Ltmp37634:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_8497b6da903267d7e7d13af19cddca66(%rip), %rsi
	movl	$8, %edx
	movq	%rsp, %rcx
	vzeroupper
	callq	*%rax
.Ltmp37635:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %ymm0
	vmovups	%ymm1, 192(%rsp)
	vmovups	%ymm0, 176(%rsp)
	vmovups	192(%rsp), %ymm1
	vmovups	176(%rsp), %ymm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%ymm0, 8(%r14)
.LBB2894_105:
	movq	288(%rsp), %rsi
	movq	$1, (%r14)
	cmpq	$-1, %rsi
	je	.LBB2894_67
	testq	%rsi, %rsi
	je	.LBB2894_67
	movq	296(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
	jmp	.LBB2894_67
.LBB2894_108:
	movq	%rbx, %r12
	movq	$-1, 64(%rsp)
	jmp	.LBB2894_112
.LBB2894_109:
	movq	%rbx, %r12
	movq	$-1, %rax
	jmp	.LBB2894_111
.LBB2894_110:
	vmovups	(%rcx), %xmm0
	movq	%rbx, %r12
	vmovaps	%xmm0, 112(%rsp)
.LBB2894_111:
	vmovaps	112(%rsp), %xmm0
	vmovups	%xmm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBB2894_112:
	movq	64(%rsp), %rcx
	movq	80(%rsp), %rax
	movq	%rcx, 448(%rsp)
	movq	72(%rsp), %rcx
	movq	%rax, 464(%rsp)
	movq	%rcx, 456(%rsp)
.Ltmp37637:
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
.Ltmp37638:
	movq	576(%rsp), %rax
	cmpl	$1, 568(%rsp)
	jne	.LBB2894_115
	vmovups	584(%rsp), %ymm0
	movq	616(%rsp), %rcx
	movq	%rcx, 48(%r14)
	movl	$1, %ecx
	vmovups	%ymm0, 16(%r14)
	jmp	.LBB2894_116
.LBB2894_115:
	xorl	%ecx, %ecx
.LBB2894_116:
	movq	%rax, 8(%r14)
	movq	%rcx, (%r14)
	lock		decq	(%r12)
.LBB2894_117:
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
.LBB2894_118:
	.cfi_def_cfa_offset 736
	leaq	8(%rsp), %rax
	leaq	64(%rsp), %rdi
	vmovups	16(%rax), %ymm1
	vmovups	(%rax), %ymm0
	vmovups	%ymm1, 192(%rsp)
	vmovups	%ymm0, 176(%rsp)
.Ltmp37640:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_da5c5f922604d9376dbdf48c863f8565(%rip), %rsi
	leaq	176(%rsp), %rcx
	movl	$5, %edx
	vzeroupper
	callq	*%rax
.Ltmp37641:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %xmm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%xmm0, 8(%r14)
	jmp	.LBB2894_12
.LBB2894_120:
.Ltmp37639:
	movq	%rax, %r14
	lock		decq	(%r12)
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB2894_121:
.Ltmp37610:
	movq	168(%rsp), %r12
	movq	%rax, %r14
	jmp	.LBB2894_128
.LBB2894_122:
.Ltmp37636:
	movq	288(%rsp), %rsi
	movq	%rbx, %r12
	movq	%rax, %r14
	cmpq	$-1, %rsi
	je	.LBB2894_127
	testq	%rsi, %rsi
	je	.LBB2894_127
	movq	296(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
	jmp	.LBB2894_127
.LBB2894_125:
.Ltmp37605:
	movq	%rbx, %r12
	movq	%rax, %r14
	jmp	.LBB2894_129
.LBB2894_126:
.Ltmp37627:
	movq	%rbx, %r12
	movq	%rax, %r14
.LBB2894_127:
	.cfi_escape 0x2e, 0x00
	leaq	400(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
.LBB2894_128:
	.cfi_escape 0x2e, 0x00
	leaq	320(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
.LBB2894_129:
	.cfi_escape 0x2e, 0x00
	leaq	256(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	lock		decq	(%r12)
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB2894_130:
.Ltmp37642:
	movq	%rbx, %r12
	movq	%rax, %r14
	lock		decq	(%r12)
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end2894:
