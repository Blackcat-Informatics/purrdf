purrdf_native::py_store::query::materialize_results:
.Lfunc_begin2160:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception2160
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
	subq	$1272, %rsp
	.cfi_def_cfa_offset 1328
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	(%rsi), %rax
	movabsq	$-9223372036854775807, %rcx
	movq	%rdi, %rbp
	movq	%rsi, 24(%rsp)
	addq	%rax, %rcx
	sarq	$63, %rax
	andq	%rcx, %rax
	je	.LBB3380_22
	cmpq	$1, %rax
	jne	.LBB3380_54
	movq	24(%rsp), %rax
	movq	8(%rax), %rsi
	movq	%rsi, 384(%rsp)
	addq	$16, %rsi
.Ltmp53590:
	movq	purrdf_rdf::native_quads::flat_rdf_quads_from_dataset@GOTPCREL(%rip), %rax
	leaq	144(%rsp), %rdi
	callq	*%rax
.Ltmp53591:
	movq	160(%rsp), %rax
	movq	152(%rsp), %rbx
	imulq	$440, %rax, %r14
	testq	%rax, %rax
	je	.LBB3380_7
	xorl	%ecx, %ecx
	.p2align	4
.LBB3380_5:
	cmpq	$-1, 360(%rbx,%rcx)
	jne	.LBB3380_69
	addq	$440, %rcx
	cmpq	%rcx, %r14
	jne	.LBB3380_5
.LBB3380_7:
	movq	144(%rsp), %rcx
	movq	%rbx, 88(%rsp)
	addq	%rbx, %r14
	movq	%rbx, %r12
	movq	%rbx, %r15
	imulq	$440, %rcx, %rdx
	movq	%rcx, 104(%rsp)
	movq	%rcx, 48(%rsp)
	movabsq	$3279421168659475843, %rcx
	movq	%r14, 112(%rsp)
	movq	%rdx, 80(%rsp)
	mulxq	%rcx, %rcx, %rcx
	movq	%rcx, 320(%rsp)
	testq	%rax, %rax
	je	.LBB3380_61
	movq	%rbp, 64(%rsp)
	leaq	632(%rsp), %rbp
	leaq	1240(%rsp), %r12
	leaq	440(%rbx), %r13
	movq	%rbx, %r15
	jmp	.LBB3380_10
	.p2align	4
.LBB3380_9:
	movq	$2, (%r15)
	leaq	-440(%r13), %rax
	addq	$440, %r13
	vmovups	1072(%rsp), %zmm0
	vmovups	1136(%rsp), %zmm1
	vmovups	1200(%rsp), %zmm2
	addq	$440, %rax
	vmovups	%zmm2, 136(%r15)
	vmovups	%zmm1, 72(%r15)
	vmovups	%zmm0, 8(%r15)
	vmovups	200(%rbp), %zmm0
	vmovups	216(%rbp), %zmm1
	vmovups	%zmm0, 200(%r15)
	vmovups	%zmm1, 216(%r15)
	vmovdqu64	280(%rbp), %zmm0
	vmovups	296(%rbp), %zmm1
	vmovdqu64	%zmm0, 280(%r15)
	vmovups	%zmm1, 296(%r15)
	addq	$360, %r15
	cmpq	%r14, %rax
	je	.LBB3380_60
.LBB3380_10:
	vmovups	-64(%r13), %zmm0
	vmovups	%zmm0, 376(%rbp)
	vmovups	-120(%r13), %zmm0
	vmovups	%zmm0, 320(%rbp)
	vmovups	-184(%r13), %zmm0
	vmovups	%zmm0, 256(%rbp)
	vmovups	-440(%r13), %zmm0
	vmovups	-376(%r13), %zmm1
	vmovups	-312(%r13), %zmm2
	vmovups	-248(%r13), %zmm3
	vmovups	%zmm3, 192(%rbp)
	vmovups	%zmm2, 128(%rbp)
	vmovups	%zmm1, 64(%rbp)
	vmovups	%zmm0, (%rbp)
	movq	%rbx, 616(%rsp)
	movq	%r15, 624(%rsp)
	movq	-248(%r13), %rax
	cmpq	$-1, 992(%rsp)
	movq	%rax, 16(%r12)
	vmovups	-264(%r13), %xmm0
	vmovups	%xmm0, (%r12)
	je	.LBB3380_12
.Ltmp53594:
	leaq	992(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::model::RdfTerm>
.Ltmp53595:
.LBB3380_12:
	cmpl	$2, 632(%rsp)
	je	.LBB3380_9
	movq	728(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB3380_16
	testq	%rsi, %rsi
	je	.LBB3380_16
	movq	736(%rsp), %rdi
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LBB3380_16:
	movq	752(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB3380_19
	testq	%rsi, %rsi
	je	.LBB3380_19
	movq	760(%rsp), %rdi
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LBB3380_19:
	movq	776(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LBB3380_9
	testq	%rsi, %rsi
	je	.LBB3380_9
	movq	784(%rsp), %rdi
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
	jmp	.LBB3380_9
.LBB3380_22:
	movq	24(%rsp), %rdx
	movq	%rbp, 64(%rsp)
	movq	$0, 144(%rsp)
	movq	16(%rdx), %rax
	vmovdqu	(%rdx), %xmm0
	movq	32(%rdx), %r13
	movq	24(%rdx), %r12
	movq	%rax, 256(%rsp)
	movq	40(%rdx), %rax
	movq	%r13, 200(%rsp)
	movq	%r12, 216(%rsp)
	movq	%r13, %rbp
	movq	%r13, %r15
	movq	%r13, 16(%rsp)
	movq	%r12, 32(%rsp)
	vmovdqa	%xmm0, 240(%rsp)
	leaq	(%rax,%rax,2), %rcx
	leaq	(%r13,%rcx,8), %r14
	leaq	144(%rsp), %rcx
	movq	%r14, 224(%rsp)
	movq	%rcx, 232(%rsp)
	testq	%rax, %rax
	je	.LBB3380_95
	data16
	leaq	pyo3::internal::state::ATTACH_COUNT::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@TLSGD(%rip), %rdi
	data16
	data16
	rex64
	callq	__tls_get_addr@PLT
	movq	%r13, %rbx
	leaq	624(%rsp), %r13
	movq	%r14, 440(%rsp)
	movq	%rax, 272(%rsp)
	leaq	.Lvtable.22(%rip), %rax
	vmovq	%rax, %xmm0
	vmovdqa	%xmm0, 576(%rsp)
	.p2align	4
.LBB3380_24:
	movq	16(%rbx), %rax
	movq	8(%rbx), %rdx
	movq	(%rbx), %rsi
	leaq	24(%rbx), %rbp
	movq	%rbx, 40(%rsp)
	movq	$0, 88(%rsp)
	movq	%rax, %rcx
	shlq	$4, %rcx
	movq	%rdx, 336(%rsp)
	movq	%rsi, 288(%rsp)
	movq	%rsi, 352(%rsp)
	leaq	88(%rsp), %rsi
	movq	%rdx, %r15
	movq	%rdx, 72(%rsp)
	movq	%rdx, %rbx
	leaq	(%rcx,%rcx,4), %r14
	leaq	(%rdx,%r14), %rcx
	movq	%rcx, 360(%rsp)
	movq	%rcx, 280(%rsp)
	movq	%rsi, 368(%rsp)
	testq	%rax, %rax
	jne	.LBB3380_27
	jmp	.LBB3380_45
	.p2align	4
.LBB3380_25:
	vmovaps	48(%rsp), %xmm1
	vmovaps	320(%rsp), %xmm2
	movq	$-1, %rax
.LBB3380_26:
	movq	%rax, (%r15)
	addq	$-80, %r14
	vmovaps	%xmm2, 320(%rsp)
	vmovaps	%xmm1, 48(%rsp)
	vmovaps	592(%rsp), %xmm0
	vmovups	%xmm0, 8(%r15)
	movq	%r12, 24(%r15)
	vmovups	%xmm2, 32(%r15)
	vmovlps	%xmm1, 48(%r15)
	vmovdqa	528(%rsp), %xmm0
	vmovdqu	%xmm0, 56(%r15)
	movq	544(%rsp), %rax
	movq	%rax, 72(%r15)
	leaq	80(%r15), %r15
	je	.LBB3380_31
.LBB3380_27:
	movq	(%r15), %rax
	cmpq	$-1, %rax
	je	.LBB3380_25
	movq	%rax, 1072(%rsp)
	leaq	1080(%rsp), %rcx
	movq	72(%r15), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	8(%r15), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp53624:
	movq	<purrdf_core::ir::term::TermValue>::into_rdf_term@GOTPCREL(%rip), %rax
	leaq	616(%rsp), %rdi
	leaq	1072(%rsp), %rsi
	vzeroupper
	callq	*%rax
.Ltmp53625:
	movq	616(%rsp), %rax
	cmpq	$-1, %rax
	je	.LBB3380_32
	movl	668(%rsp), %ecx
	vmovups	(%r13), %xmm0
	vmovups	48(%r13), %xmm3
	vmovups	648(%rsp), %xmm2
	vmovsd	664(%rsp), %xmm1
	movq	640(%rsp), %r12
	movl	%ecx, 80(%rsp)
	movq	64(%r13), %rcx
	vmovaps	%xmm0, 592(%rsp)
	vmovaps	%xmm3, 528(%rsp)
	movq	%rcx, 544(%rsp)
	jmp	.LBB3380_26
	.p2align	4
.LBB3380_31:
	movq	280(%rsp), %r15
	movq	%r15, %rbx
	jmp	.LBB3380_45
	.p2align	4
.LBB3380_32:
	leaq	296(%rsp), %rax
	movq	$1610612768, 568(%rsp)
	movq	$0, 296(%rsp)
	movq	$1, 304(%rsp)
	movq	$0, 312(%rsp)
	movq	%rax, 552(%rsp)
	leaq	.Lvtable.1T(%rip), %rax
	movq	%rax, 560(%rsp)
.Ltmp53627:
	movq	<purrdf_core::ir::term::NonIriPredicate as core::fmt::Display>::fmt@GOTPCREL(%rip), %rax
	leaq	15(%rsp), %rdi
	leaq	552(%rsp), %rsi
	callq	*%rax
.Ltmp53628:
	testb	%al, %al
	jne	.LBB3380_130
	movq	__rustc::__rust_no_alloc_shim_is_unstable_v2@GOTPCREL(%rip), %rax
	movq	296(%rsp), %rbx
	movq	304(%rsp), %r12
	movq	312(%rsp), %r14
	callq	*%rax
	movq	__rustc::__rust_alloc@GOTPCREL(%rip), %rax
	movl	$24, %edi
	movl	$8, %esi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB3380_129
	cmpq	$0, 88(%rsp)
	movq	%rbx, %rcx
	leaq	80(%r15), %rbx
	movq	%rcx, (%rax)
	movq	%r12, 8(%rax)
	movq	%rax, %r12
	movq	%r14, 16(%rax)
	movq	%rbx, 344(%rsp)
	je	.LBB3380_44
	cmpq	$0, 112(%rsp)
	je	.LBB3380_44
	movq	120(%rsp), %rax
	movq	128(%rsp), %r14
	movq	%rax, 48(%rsp)
	testq	%rax, %rax
	je	.LBB3380_42
	movq	(%r14), %rax
	testq	%rax, %rax
	je	.LBB3380_40
.Ltmp53630:
	movq	48(%rsp), %rdi
	callq	*%rax
.Ltmp53631:
.LBB3380_40:
	movq	8(%r14), %rsi
	testq	%rsi, %rsi
	je	.LBB3380_44
	movq	16(%r14), %rdx
	movq	48(%rsp), %rdi
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	callq	*%rax
	jmp	.LBB3380_44
.LBB3380_42:
	movq	272(%rsp), %rax
	cmpq	$0, (%rax)
	jle	.LBB3380_53
	movq	_Py_DecRef@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
	.p2align	4
.LBB3380_44:
	leaq	96(%rsp), %rax
	movq	$1, 88(%rsp)
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%xmm0, (%rax)
	vpbroadcastd	.LCPI3380_3(%rip), %xmm0
	leaq	.Lvtable.22(%rip), %rax
	movq	$1, 112(%rsp)
	movq	%r12, 120(%rsp)
	movq	%rax, 128(%rsp)
	movl	80(%rsp), %eax
	movl	$3, 136(%rsp)
	movl	%eax, 140(%rsp)
	vpinsrd	$1, %eax, %xmm0, %xmm0
	vmovdqa	%xmm0, 48(%rsp)
	vmovq	%r12, %xmm0
	vpunpcklqdq	576(%rsp), %xmm0, %xmm0
	movl	$1, %r12d
	vmovdqa	%xmm0, 320(%rsp)
.LBB3380_45:
	movq	72(%rsp), %rcx
	movq	280(%rsp), %rdx
	vpbroadcastq	.LCPI3380_2(%rip), %xmm0
	movabsq	$-3689348814741910323, %rax
	subq	%rcx, %r15
	shrq	$4, %r15
	imulq	%rax, %r15
	subq	%rbx, %rdx
	mulxq	%rax, %rax, %rax
	movq	288(%rsp), %rdx
	movq	%rcx, 616(%rsp)
	movq	$0, 352(%rsp)
	vmovdqa	%xmm0, 336(%rsp)
	movq	$8, 360(%rsp)
	movq	%r15, 624(%rsp)
	movq	%rdx, 632(%rsp)
	je	.LBB3380_50
	shrq	$6, %rax
	movl	$1, %r14d
	subq	%rax, %r14
	jmp	.LBB3380_48
	.p2align	4
.LBB3380_47:
	addq	$80, %rbx
	incq	%r14
	cmpq	$1, %r14
	je	.LBB3380_50
.LBB3380_48:
	cmpq	$-1, (%rbx)
	je	.LBB3380_47
.Ltmp53648:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_core::ir::term::TermValue>
.Ltmp53649:
	jmp	.LBB3380_47
	.p2align	4
.LBB3380_50:
	movq	288(%rsp), %rax
	movq	72(%rsp), %rcx
	movq	%rax, 472(%rsp)
	movq	%rcx, 480(%rsp)
	movq	%r15, 488(%rsp)
.Ltmp53659:
	leaq	336(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<core::option::Option<purrdf_core::ir::term::TermValue>>>
.Ltmp53660:
	cmpl	$1, 88(%rsp)
	movq	40(%rsp), %rcx
	movq	440(%rsp), %r14
	je	.LBB3380_72
	vmovdqu	472(%rsp), %xmm0
	movq	488(%rsp), %rax
	movq	%rbp, %rbx
	movq	%rax, 400(%rsp)
	vmovdqa	%xmm0, 384(%rsp)
	movq	%rax, 16(%rcx)
	vmovdqu	%xmm0, (%rcx)
	cmpq	%r14, %rbp
	jne	.LBB3380_24
	jmp	.LBB3380_80
.LBB3380_53:
.Ltmp53633:
	movq	<pyo3::instance::Py<_> as core::ops::drop::Drop>::drop::drop_slow@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
.Ltmp53634:
	jmp	.LBB3380_44
.LBB3380_54:
	movq	24(%rsp), %rax
	movzbl	8(%rax), %ebx
	movq	<purrdf_native::py_store::query::PyQueryBoolean as pyo3::impl_::pyclass::PyClassImpl>::lazy_type_object::TYPE_OBJECT@GOTPCREL(%rip), %rax
	movl	88(%rax), %ecx
	testl	%ecx, %ecx
	jne	.LBB3380_131
	addq	$80, %rax
.LBB3380_56:
	movq	(%rax), %rdx
.Ltmp53588:
	movq	PyBaseObject_Type@GOTPCREL(%rip), %rsi
	movq	<pyo3::internal::pyclass_init::PyNativeTypeInitializer<_> as pyo3::internal::pyclass_init::PyObjectInit<_>>::into_new_object::inner@GOTPCREL(%rip), %rax
	leaq	616(%rsp), %rdi
	callq	*%rax
.Ltmp53589:
	movq	624(%rsp), %rax
	cmpb	$0, 616(%rsp)
	je	.LBB3380_59
	vmovdqu	632(%rsp), %ymm0
	movq	664(%rsp), %rcx
	movq	%rcx, 48(%rbp)
	vmovdqu	%ymm0, 16(%rbp)
	movq	%rax, 8(%rbp)
	movq	$1, (%rbp)
	movq	24(%rsp), %rdi
	cmpq	$0, (%rdi)
	jns	.LBB3380_123
	jmp	.LBB3380_125
.LBB3380_59:
	movb	%bl, 16(%rax)
	movq	$0, 24(%rax)
	movq	%rax, 8(%rbp)
	movq	$0, (%rbp)
	movq	24(%rsp), %rdi
	cmpq	$0, (%rdi)
	jns	.LBB3380_123
	jmp	.LBB3380_125
.LBB3380_60:
	movq	64(%rsp), %rbp
	movq	%r14, %r12
.LBB3380_61:
	vmovdqa	.LCPI3380_0(%rip), %ymm0
	subq	%rbx, %r15
	movq	%r14, %rdx
	subq	%r12, %rdx
	movabsq	$5738987045154082725, %r13
	movabsq	$2683162774357752963, %rax
	shrq	$3, %r15
	shrq	$3, %rdx
	mulxq	%rax, %rax, %rax
	imulq	%r15, %r13
	movq	48(%rsp), %r15
	movq	%rbx, 616(%rsp)
	movq	%r13, 624(%rsp)
	movq	%r15, 632(%rsp)
	vmovdqu	%ymm0, 88(%rsp)
	cmpq	%r12, %r14
	je	.LBB3380_65
	shrq	$3, %rax
	movl	$1, %r14d
	subq	%rax, %r14
	.p2align	4
.LBB3380_63:
.Ltmp53600:
	movq	%r12, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::model::RdfQuad>
.Ltmp53601:
	incq	%r14
	addq	$440, %r12
	cmpq	$1, %r14
	jne	.LBB3380_63
.LBB3380_65:
	movq	320(%rsp), %rcx
	movq	80(%rsp), %rsi
	shrq	$6, %rcx
	testq	%r15, %r15
	setne	%al
	imulq	$360, %rcx, %r15
	movq	%rcx, %r14
	cmpq	%r15, %rsi
	setne	%cl
	testb	%cl, %al
	je	.LBB3380_85
	cmpq	$359, %rsi
	ja	.LBB3380_81
	testq	%rsi, %rsi
	je	.LBB3380_84
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB3380_84:
	movl	$8, %ebx
	jmp	.LBB3380_85
.LBB3380_69:
	vmovups	144(%rsp), %xmm0
	movq	160(%rsp), %rax
	movq	%rax, 1088(%rsp)
	vmovaps	%xmm0, 1072(%rsp)
	movq	$0, 1096(%rsp)
.Ltmp53592:
	leaq	616(%rsp), %rdi
	leaq	1072(%rsp), %rsi
	callq	<pyo3::pyclass_init::PyClassInitializer<purrdf_native::py_store::query::PyQueryQuads>>::create_class_object
.Ltmp53593:
	cmpb	$0, 616(%rsp)
	je	.LBB3380_83
	movq	664(%rsp), %rax
	vmovups	632(%rsp), %ymm0
	movl	$1, %ecx
	movq	%rax, 48(%rbp)
	movq	624(%rsp), %rax
	vmovups	%ymm0, 16(%rbp)
	jmp	.LBB3380_90
.LBB3380_72:
	leaq	96(%rsp), %rax
	movq	%rbp, 208(%rsp)
	vmovdqu	(%rax), %ymm0
	vmovups	16(%rax), %ymm1
	vmovdqu	%ymm0, 384(%rsp)
	vmovups	%ymm1, 400(%rsp)
.Ltmp53665:
	leaq	472(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>
.Ltmp53666:
	cmpq	$0, 144(%rsp)
	movq	16(%rsp), %r13
	movq	32(%rsp), %r12
	je	.LBB3380_94
	cmpq	$0, 168(%rsp)
	je	.LBB3380_94
	movq	176(%rsp), %r15
	movq	184(%rsp), %rbx
	testq	%r15, %r15
	je	.LBB3380_92
	movq	(%rbx), %rax
	testq	%rax, %rax
	je	.LBB3380_78
.Ltmp53671:
	movq	%r15, %rdi
	callq	*%rax
.Ltmp53672:
.LBB3380_78:
	movq	8(%rbx), %rsi
	testq	%rsi, %rsi
	je	.LBB3380_94
	movq	16(%rbx), %rdx
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
	jmp	.LBB3380_94
.LBB3380_80:
	movq	16(%rsp), %r13
	movq	32(%rsp), %r12
	movq	%r14, %rbp
	movq	%r14, %r15
	jmp	.LBB3380_95
.LBB3380_81:
	movq	__rustc::__rust_realloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	movq	%r15, %rcx
	vzeroupper
	callq	*%rax
	movq	%rax, %rbx
	testq	%rax, %rax
	je	.LBB3380_82
.LBB3380_85:
.Ltmp53614:
	leaq	88(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::model::RdfQuad>>
.Ltmp53615:
	movq	%r14, 1072(%rsp)
	movq	%rbx, 1080(%rsp)
	movq	%r13, 1088(%rsp)
	movq	$0, 1096(%rsp)
.Ltmp53616:
	leaq	616(%rsp), %rdi
	leaq	1072(%rsp), %rsi
	callq	<pyo3::pyclass_init::PyClassInitializer<purrdf_native::py_store::query::PyQueryTriples>>::create_class_object
.Ltmp53617:
	movq	624(%rsp), %rax
	cmpb	$0, 616(%rsp)
	je	.LBB3380_89
	vmovdqu	632(%rsp), %ymm0
	movq	664(%rsp), %rcx
	movq	%rcx, 48(%rbp)
	movl	$1, %ecx
	vmovdqu	%ymm0, 16(%rbp)
	jmp	.LBB3380_90
.LBB3380_83:
	movq	624(%rsp), %rax
.LBB3380_89:
	xorl	%ecx, %ecx
.LBB3380_90:
	movq	%rax, 8(%rbp)
	movq	%rcx, (%rbp)
	movq	384(%rsp), %rcx
	lock		decq	(%rcx)
	jne	.LBB3380_122
	#MEMBARRIER
.Ltmp53621:
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	384(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp53622:
	jmp	.LBB3380_122
.LBB3380_92:
	movq	272(%rsp), %rax
	cmpq	$0, (%rax)
	jle	.LBB3380_135
	movq	_Py_DecRef@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.LBB3380_94:
	vmovdqu	384(%rsp), %ymm0
	vmovups	400(%rsp), %ymm1
	movq	40(%rsp), %r15
	movq	$1, 144(%rsp)
	vmovdqu	%ymm0, 152(%rsp)
	vmovups	%ymm1, 168(%rsp)
.LBB3380_95:
	movq	%r15, %rdx
	subq	%r13, %rdx
	movabsq	$-6148914691236517205, %rax
	vmovdqa	.LCPI3380_0(%rip), %ymm0
	mulxq	%rax, %rbx, %rbx
	shrq	$4, %rbx
	subq	%rbp, %r14
	movq	%r14, %rdx
	mulxq	%rax, %rax, %rax
	movq	%r13, 616(%rsp)
	movq	%rbx, 624(%rsp)
	movq	%r12, 632(%rsp)
	vmovdqu	%ymm0, 200(%rsp)
	je	.LBB3380_99
	shrq	$4, %rax
	movl	$1, %r14d
	subq	%rax, %r14
	.p2align	4
.LBB3380_97:
.Ltmp53680:
	movq	%rbp, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>>
.Ltmp53681:
	incq	%r14
	addq	$24, %rbp
	cmpq	$1, %r14
	jne	.LBB3380_97
.LBB3380_99:
.Ltmp53691:
	leaq	200(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>>>
.Ltmp53692:
	cmpl	$1, 144(%rsp)
	jne	.LBB3380_112
	vmovdqu	176(%rsp), %xmm0
	movq	%r15, %rcx
	movq	192(%rsp), %rax
	movq	152(%rsp), %r14
	movq	160(%rsp), %r15
	movq	168(%rsp), %r12
	movq	64(%rsp), %rbp
	movq	%rax, 512(%rsp)
	vmovdqa	%xmm0, 496(%rsp)
	cmpq	%r13, %rcx
	je	.LBB3380_105
	movl	$1, %r13d
	subq	%rbx, %r13
	movq	16(%rsp), %rbx
	.p2align	4
.LBB3380_103:
.Ltmp53707:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>
.Ltmp53708:
	incq	%r13
	addq	$24, %rbx
	cmpq	$1, %r13
	jne	.LBB3380_103
.LBB3380_105:
	movq	32(%rsp), %rax
	testq	%rax, %rax
	je	.LBB3380_107
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rbx
	movq	16(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,2), %rsi
	callq	*%rbx
.LBB3380_107:
	vmovdqa	496(%rsp), %xmm0
	movq	512(%rsp), %rax
	movq	248(%rsp), %rbx
	movq	%rax, 48(%rbp)
	vmovdqu	%xmm0, 32(%rbp)
	movq	%r14, 8(%rbp)
	movq	256(%rsp), %r14
	movq	%r15, 16(%rbp)
	movq	%r12, 24(%rbp)
	movq	$1, (%rbp)
	testq	%r14, %r14
	je	.LBB3380_120
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r12
	leaq	8(%rbx), %r15
	jmp	.LBB3380_110
	.p2align	4
.LBB3380_109:
	addq	$24, %r15
	decq	%r14
	je	.LBB3380_120
.LBB3380_110:
	movq	-8(%r15), %rsi
	testq	%rsi, %rsi
	je	.LBB3380_109
	movq	(%r15), %rdi
	movl	$1, %edx
	callq	*%r12
	jmp	.LBB3380_109
.LBB3380_112:
	movq	248(%rsp), %rax
	movq	256(%rsp), %rbp
	movq	%r12, 448(%rsp)
	movq	%r13, 456(%rsp)
	movq	%rbx, 464(%rsp)
	movq	240(%rsp), %rbx
	movq	%rax, 48(%rsp)
	leaq	(,%rbp,8), %rax
	leaq	(%rax,%rax,2), %r14
.Ltmp53697:
	movq	alloc::sync::arcinner_layout_for_value_layout@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp53698:
	movq	%rax, %r13
	movq	%rdx, %r12
	testq	%rdx, %rdx
	je	.LBB3380_127
	movq	__rustc::__rust_no_alloc_shim_is_unstable_v2@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	__rustc::__rust_alloc@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	movq	%r13, %rsi
	callq	*%rax
	movq	%rax, %r15
	testq	%r15, %r15
	je	.LBB3380_128
.LBB3380_115:
	movq	48(%rsp), %r12
	movq	memcpy@GOTPCREL(%rip), %rax
	leaq	16(%r15), %rdi
	movq	$1, (%r15)
	movq	$1, 8(%r15)
	movq	%r14, %rdx
	movq	%r12, %rsi
	callq	*%rax
	testq	%rbx, %rbx
	je	.LBB3380_117
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	shlq	$3, %rbx
	movl	$8, %edx
	movq	%r12, %rdi
	leaq	(%rbx,%rbx,2), %rsi
	callq	*%rax
.LBB3380_117:
	vmovdqu	448(%rsp), %xmm0
	movq	464(%rsp), %rax
	movq	%rax, 1088(%rsp)
	vmovdqa	%xmm0, 1072(%rsp)
	movq	%r15, 1096(%rsp)
	movq	%rbp, 1104(%rsp)
	movq	$0, 1112(%rsp)
.Ltmp53699:
	leaq	616(%rsp), %rdi
	leaq	1072(%rsp), %rsi
	callq	<pyo3::pyclass_init::PyClassInitializer<purrdf_native::py_store::query::PyQuerySolutions>>::create_class_object
.Ltmp53700:
	movq	624(%rsp), %rax
	cmpl	$1, 616(%rsp)
	movq	64(%rsp), %rdx
	jne	.LBB3380_126
	vmovdqu	632(%rsp), %ymm0
	movq	664(%rsp), %rcx
	movq	%rcx, 48(%rdx)
	vmovdqu	%ymm0, 16(%rdx)
	movq	%rax, 8(%rdx)
	movq	$1, (%rdx)
	movq	24(%rsp), %rdi
	cmpq	$0, (%rdi)
	jns	.LBB3380_123
	jmp	.LBB3380_125
.LBB3380_120:
	movq	240(%rsp), %rax
	testq	%rax, %rax
	je	.LBB3380_122
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r14
	shlq	$3, %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	leaq	(%rax,%rax,2), %rsi
	callq	*%r14
.LBB3380_122:
	movq	24(%rsp), %rdi
	cmpq	$0, (%rdi)
	js	.LBB3380_125
.LBB3380_123:
	movq	48(%rdi), %rax
	lock		decq	(%rax)
	jne	.LBB3380_125
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	addq	$48, %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB3380_125:
	addq	$1272, %rsp
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
.LBB3380_126:
	.cfi_def_cfa_offset 1328
	movq	%rax, 8(%rdx)
	movq	$0, (%rdx)
	movq	24(%rsp), %rdi
	cmpq	$0, (%rdi)
	jns	.LBB3380_123
	jmp	.LBB3380_125
.LBB3380_127:
	movq	%r13, %r15
	testq	%r15, %r15
	jne	.LBB3380_115
.LBB3380_128:
.Ltmp53702:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r12, %rsi
	callq	*%rax
.Ltmp53703:
	jmp	.LBB3380_134
.LBB3380_129:
	movq	%r12, %r14
	movq	%rbp, 208(%rsp)
.Ltmp53639:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$24, %esi
	callq	*%rax
.Ltmp53640:
	jmp	.LBB3380_134
.LBB3380_130:
	movq	%rbp, 208(%rsp)
.Ltmp53642:
	movq	core::result::unwrap_failed@GOTPCREL(%rip), %rax
	movq	16(%rsp), %rbx
	leaq	.Lalloc_cc656815297f75969399c3f4b1ad3de4(%rip), %rdi
	leaq	.Lvtable.2q(%rip), %rcx
	leaq	.Lalloc_d4c8062c4f28c49e31e589e7f415a063(%rip), %r8
	leaq	15(%rsp), %rdx
	movl	$55, %esi
	callq	*%rax
.Ltmp53643:
	jmp	.LBB3380_134
.LBB3380_131:
.Ltmp53584:
	leaq	616(%rsp), %rdi
	callq	<pyo3::impl_::pyclass::lazy_type_object::LazyTypeObject<purrdf_native::py_store::query::PyQueryBoolean>>::try_init
.Ltmp53585:
	cmpb	$0, 616(%rsp)
	je	.LBB3380_136
.Ltmp53586:
	movq	pyo3::impl_::pyclass::lazy_type_object::type_object_init_failed@GOTPCREL(%rip), %rax
	leaq	624(%rsp), %rdi
	leaq	.Lalloc_c061a48059fbd8e0b91c7f3c14785c78(%rip), %rsi
	movl	$12, %edx
	callq	*%rax
.Ltmp53587:
	jmp	.LBB3380_134
.LBB3380_82:
.Ltmp53606:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r15, %rsi
	callq	*%rax
.Ltmp53607:
.LBB3380_134:
	ud2
.LBB3380_135:
.Ltmp53674:
	movq	<pyo3::instance::Py<_> as core::ops::drop::Drop>::drop::drop_slow@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp53675:
	jmp	.LBB3380_94
.LBB3380_136:
	movq	624(%rsp), %rax
	jmp	.LBB3380_56
.LBB3380_137:
.Ltmp53676:
	movq	%rax, %r12
	jmp	.LBB3380_141
.LBB3380_138:
.Ltmp53635:
	movq	%rax, %r14
	movq	%rbp, 208(%rsp)
	jmp	.LBB3380_146
.LBB3380_139:
.Ltmp53673:
	movq	8(%rbx), %rsi
	movq	%rax, %r12
	testq	%rsi, %rsi
	je	.LBB3380_141
	movq	16(%rbx), %rdx
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.LBB3380_141:
	vmovdqu	384(%rsp), %ymm0
	vmovups	400(%rsp), %ymm1
	leaq	152(%rsp), %rax
	movq	$1, 144(%rsp)
	vmovups	%ymm1, 16(%rax)
	vmovdqu	%ymm0, (%rax)
.Ltmp53677:
	movq	16(%rsp), %rdi
	movq	40(%rsp), %rsi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDrop<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>>
.Ltmp53678:
	jmp	.LBB3380_202
.LBB3380_142:
.Ltmp53679:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3380_143:
.Ltmp53667:
	movq	16(%rsp), %rbx
	movq	40(%rsp), %r14
	movq	%rax, %r12
	jmp	.LBB3380_201
.LBB3380_144:
.Ltmp53632:
	movq	%rbp, 208(%rsp)
	movq	%r14, %rcx
	movq	%rax, %r14
	movq	8(%rcx), %rsi
	testq	%rsi, %rsi
	je	.LBB3380_146
	movq	16(%rcx), %rdx
	movq	48(%rsp), %rdi
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3380_146:
	movl	80(%rsp), %ecx
	leaq	96(%rsp), %rax
	leaq	.Lvtable.22(%rip), %rdx
	movq	$1, 88(%rsp)
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%xmm0, (%rax)
	movq	$1, 112(%rsp)
	movq	%r12, 120(%rsp)
	movq	%rdx, 128(%rsp)
	movl	$3, 136(%rsp)
	movl	%ecx, 140(%rsp)
.Ltmp53636:
	movq	72(%rsp), %rdi
	movq	%r15, %rsi
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDrop<core::option::Option<purrdf_core::model::RdfTerm>>>
.Ltmp53637:
	movq	16(%rsp), %rbx
	movq	%r14, %r12
	jmp	.LBB3380_198
.LBB3380_147:
.Ltmp53638:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3380_148:
.Ltmp53701:
	jmp	.LBB3380_190
.LBB3380_149:
.Ltmp53693:
	movq	%rax, %r12
	jmp	.LBB3380_203
.LBB3380_150:
.Ltmp53629:
	movq	16(%rsp), %rbx
	movq	%rax, %r12
	movq	%rbp, 208(%rsp)
	jmp	.LBB3380_192
.LBB3380_151:
.Ltmp53618:
	movq	%rax, %rbp
	jmp	.LBB3380_184
.LBB3380_152:
.Ltmp53596:
	leaq	632(%rsp), %rdi
	movq	%rax, %rbp
	movq	%r13, 96(%rsp)
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_core::diagnostic::RdfLocation>>
.Ltmp53597:
	movq	%rbx, %rdi
	movq	%r15, %rsi
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>>
.Ltmp53598:
	jmp	.LBB3380_183
.LBB3380_153:
.Ltmp53599:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3380_154:
.Ltmp53709:
	movq	%rax, %r12
	testq	%r13, %r13
	je	.LBB3380_158
	negq	%r13
	addq	$24, %rbx
.LBB3380_156:
.Ltmp53710:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>
.Ltmp53711:
	addq	$24, %rbx
	decq	%r13
	jne	.LBB3380_156
.LBB3380_158:
	cmpq	$0, 32(%rsp)
	je	.LBB3380_205
	movq	32(%rsp), %rax
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rbx
	movq	16(%rsp), %rdi
	movl	$8, %edx
	shlq	$3, %rax
	leaq	(%rax,%rax,2), %rsi
	callq	*%rbx
	jmp	.LBB3380_205
.LBB3380_160:
.Ltmp53712:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3380_161:
.Ltmp53661:
	movq	16(%rsp), %rbx
	movq	%rax, %r12
	movq	%rbp, 208(%rsp)
	jmp	.LBB3380_199
.LBB3380_162:
.Ltmp53602:
	movq	%rax, %rbp
	testq	%r14, %r14
	je	.LBB3380_182
	negq	%r14
	addq	$440, %r12
	.p2align	4
.LBB3380_164:
.Ltmp53603:
	movq	%r12, %rdi
	callq	core::ptr::drop_glue::<purrdf_core::model::RdfQuad>
.Ltmp53604:
	addq	$440, %r12
	decq	%r14
	jne	.LBB3380_164
	jmp	.LBB3380_182
.LBB3380_166:
.Ltmp53605:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3380_167:
.Ltmp53682:
	movq	%rax, %r12
	testq	%r14, %r14
	je	.LBB3380_171
	negq	%r14
	addq	$24, %rbp
.LBB3380_169:
.Ltmp53683:
	movq	%rbp, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>>
.Ltmp53684:
	addq	$24, %rbp
	decq	%r14
	jne	.LBB3380_169
.LBB3380_171:
.Ltmp53686:
	leaq	616(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>, alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>>
.Ltmp53687:
	jmp	.LBB3380_202
.LBB3380_172:
.Ltmp53685:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3380_173:
.Ltmp53626:
	movq	16(%rsp), %rbx
	movq	%rax, %r12
	movq	%rbp, 208(%rsp)
	jmp	.LBB3380_197
.LBB3380_174:
.Ltmp53650:
	movq	%rax, %r12
	movq	%rbp, 208(%rsp)
	testq	%r14, %r14
	jne	.LBB3380_176
.LBB3380_175:
.Ltmp53654:
	leaq	616(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<core::option::Option<purrdf_core::ir::term::TermValue>, core::option::Option<purrdf_core::model::RdfTerm>>>
.Ltmp53655:
	movq	16(%rsp), %rbx
	jmp	.LBB3380_198
.LBB3380_176:
	addq	$80, %rbx
	negq	%r14
	jmp	.LBB3380_178
.LBB3380_177:
	addq	$80, %rbx
	decq	%r14
	je	.LBB3380_175
.LBB3380_178:
	cmpq	$-1, (%rbx)
	je	.LBB3380_177
.Ltmp53651:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_core::ir::term::TermValue>
.Ltmp53652:
	jmp	.LBB3380_177
.LBB3380_180:
.Ltmp53653:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3380_181:
.Ltmp53608:
	movq	%rax, %rbp
.LBB3380_182:
.Ltmp53609:
	leaq	616(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<purrdf_core::model::RdfQuad, purrdf_core::model::RdfTriple>>
.Ltmp53610:
.LBB3380_183:
.Ltmp53611:
	leaq	88(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::model::RdfQuad>>
.Ltmp53612:
.LBB3380_184:
	movq	384(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB3380_186
	#MEMBARRIER
.Ltmp53619:
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	384(%rsp), %rdi
	callq	*%rax
.Ltmp53620:
	movq	24(%rsp), %rdi
	movq	%rbp, %r12
	jmp	.LBB3380_206
.LBB3380_186:
	movq	24(%rsp), %rdi
	movq	%rbp, %r12
	jmp	.LBB3380_206
.LBB3380_187:
.Ltmp53613:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3380_188:
.Ltmp53704:
	movq	%rax, %r12
.Ltmp53705:
	leaq	448(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>>
.Ltmp53706:
	movq	24(%rsp), %rdi
	jmp	.LBB3380_206
.LBB3380_189:
.Ltmp53623:
.LBB3380_190:
	movq	24(%rsp), %rdi
	movq	%rax, %r12
	jmp	.LBB3380_206
.LBB3380_191:
.Ltmp53644:
	movq	%rax, %r12
.LBB3380_192:
	movq	296(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB3380_197
	movq	304(%rsp), %rdi
	jmp	.LBB3380_196
.LBB3380_194:
.Ltmp53641:
	movq	%rax, %r12
	movq	%rbx, %rsi
	testq	%rbx, %rbx
	movq	16(%rsp), %rbx
	je	.LBB3380_197
	movq	%r14, %rdi
.LBB3380_196:
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBB3380_197:
	leaq	80(%r15), %rax
	movq	%rax, 344(%rsp)
.Ltmp53645:
	movq	72(%rsp), %rdi
	movq	%r15, %rsi
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDrop<core::option::Option<purrdf_core::model::RdfTerm>>>
.Ltmp53646:
.LBB3380_198:
.Ltmp53656:
	leaq	336(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<core::option::Option<purrdf_core::ir::term::TermValue>>>
.Ltmp53657:
.LBB3380_199:
	cmpq	$0, 88(%rsp)
	movq	40(%rsp), %r14
	je	.LBB3380_201
.Ltmp53662:
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<pyo3::err::PyErr>
.Ltmp53663:
.LBB3380_201:
.Ltmp53668:
	movq	%rbx, %rdi
	movq	%r14, %rsi
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDrop<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>>
.Ltmp53669:
.LBB3380_202:
.Ltmp53688:
	leaq	200(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>>>
.Ltmp53689:
.LBB3380_203:
	cmpq	$0, 144(%rsp)
	je	.LBB3380_205
	leaq	152(%rsp), %rdi
.Ltmp53694:
	callq	core::ptr::drop_glue::<pyo3::err::PyErr>
.Ltmp53695:
.LBB3380_205:
	leaq	240(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::string::String>>
	movq	24(%rsp), %rdi
.LBB3380_206:
	cmpq	$0, (%rdi)
	js	.LBB3380_209
	movq	48(%rdi), %rax
	lock		decq	(%rax)
	jne	.LBB3380_209
	addq	$48, %rdi
	#MEMBARRIER
.Ltmp53713:
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp53714:
.LBB3380_209:
	movq	%r12, %rdi
	callq	_Unwind_Resume@PLT
.LBB3380_210:
.Ltmp53664:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3380_211:
.Ltmp53696:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3380_212:
.Ltmp53647:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3380_213:
.Ltmp53670:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3380_214:
.Ltmp53690:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3380_215:
.Ltmp53658:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB3380_216:
.Ltmp53715:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end3380:
