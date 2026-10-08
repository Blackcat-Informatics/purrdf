purrdf_native::py_store::query::materialize_results:
.LOCAL:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .EXCEPTION
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
	je	.LOCAL
	cmpq	$1, %rax
	jne	.LOCAL
	movq	24(%rsp), %rax
	movq	8(%rax), %rsi
	movq	%rsi, 384(%rsp)
	addq	$16, %rsi
.LOCAL:
	movq	purrdf_rdf::native_quads::flat_rdf_quads_from_dataset@GOTPCREL(%rip), %rax
	leaq	144(%rsp), %rdi
	callq	*%rax
.LOCAL:
	movq	160(%rsp), %rax
	movq	152(%rsp), %rbx
	imulq	$440, %rax, %r14
	testq	%rax, %rax
	je	.LOCAL
	xorl	%ecx, %ecx
	.p2align	4
.LOCAL:
	cmpq	$-1, 360(%rbx,%rcx)
	jne	.LOCAL
	addq	$440, %rcx
	cmpq	%rcx, %r14
	jne	.LOCAL
.LOCAL:
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
	je	.LOCAL
	movq	%rbp, 64(%rsp)
	leaq	632(%rsp), %rbp
	leaq	1240(%rsp), %r12
	leaq	440(%rbx), %r13
	movq	%rbx, %r15
	jmp	.LOCAL
	.p2align	4
.LOCAL:
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
	je	.LOCAL
.LOCAL:
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
	je	.LOCAL
.LOCAL:
	leaq	992(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::model::RdfTerm>
.LOCAL:
.LOCAL:
	cmpl	$2, 632(%rsp)
	je	.LOCAL
	movq	728(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LOCAL
	testq	%rsi, %rsi
	je	.LOCAL
	movq	736(%rsp), %rdi
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LOCAL:
	movq	752(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LOCAL
	testq	%rsi, %rsi
	je	.LOCAL
	movq	760(%rsp), %rdi
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LOCAL:
	movq	776(%rsp), %rsi
	cmpq	$-1, %rsi
	je	.LOCAL
	testq	%rsi, %rsi
	je	.LOCAL
	movq	784(%rsp), %rdi
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
	jmp	.LOCAL
.LOCAL:
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
	je	.LOCAL
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
	leaq	.VTERR(%rip), %rax
	vmovq	%rax, %xmm0
	vmovdqa	%xmm0, 576(%rsp)
	.p2align	4
.LOCAL:
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
	jne	.LOCAL
	jmp	.LOCAL
	.p2align	4
.LOCAL:
	vmovaps	48(%rsp), %xmm1
	vmovaps	320(%rsp), %xmm2
	movq	$-1, %rax
.LOCAL:
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
	je	.LOCAL
.LOCAL:
	movq	(%r15), %rax
	cmpq	$-1, %rax
	je	.LOCAL
	movq	%rax, 1072(%rsp)
	leaq	1080(%rsp), %rcx
	movq	72(%r15), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	8(%r15), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.LOCAL:
	movq	<purrdf_core::ir::term::TermValue>::into_rdf_term@GOTPCREL(%rip), %rax
	leaq	616(%rsp), %rdi
	leaq	1072(%rsp), %rsi
	vzeroupper
	callq	*%rax
.LOCAL:
	movq	616(%rsp), %rax
	cmpq	$-1, %rax
	je	.LOCAL
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
	jmp	.LOCAL
	.p2align	4
.LOCAL:
	movq	280(%rsp), %r15
	movq	%r15, %rbx
	jmp	.LOCAL
	.p2align	4
.LOCAL:
	leaq	296(%rsp), %rax
	movq	$1610612768, 568(%rsp)
	movq	$0, 296(%rsp)
	movq	$1, 304(%rsp)
	movq	$0, 312(%rsp)
	movq	%rax, 552(%rsp)
	leaq	.VTSTRING(%rip), %rax
	movq	%rax, 560(%rsp)
.LOCAL:
	movq	<purrdf_core::ir::term::NonIriPredicate as core::fmt::Display>::fmt@GOTPCREL(%rip), %rax
	leaq	15(%rsp), %rdi
	leaq	552(%rsp), %rsi
	callq	*%rax
.LOCAL:
	testb	%al, %al
	jne	.LOCAL
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
	je	.LOCAL
	cmpq	$0, 88(%rsp)
	movq	%rbx, %rcx
	leaq	80(%r15), %rbx
	movq	%rcx, (%rax)
	movq	%r12, 8(%rax)
	movq	%rax, %r12
	movq	%r14, 16(%rax)
	movq	%rbx, 344(%rsp)
	je	.LOCAL
	cmpq	$0, 112(%rsp)
	je	.LOCAL
	movq	120(%rsp), %rax
	movq	128(%rsp), %r14
	movq	%rax, 48(%rsp)
	testq	%rax, %rax
	je	.LOCAL
	movq	(%r14), %rax
	testq	%rax, %rax
	je	.LOCAL
.LOCAL:
	movq	48(%rsp), %rdi
	callq	*%rax
.LOCAL:
.LOCAL:
	movq	8(%r14), %rsi
	testq	%rsi, %rsi
	je	.LOCAL
	movq	16(%r14), %rdx
	movq	48(%rsp), %rdi
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	callq	*%rax
	jmp	.LOCAL
.LOCAL:
	movq	272(%rsp), %rax
	cmpq	$0, (%rax)
	jle	.LOCAL
	movq	_Py_DecRef@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
	.p2align	4
.LOCAL:
	leaq	96(%rsp), %rax
	movq	$1, 88(%rsp)
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%xmm0, (%rax)
	vpbroadcastd	.LOCAL(%rip), %xmm0
	leaq	.VTERR(%rip), %rax
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
.LOCAL:
	movq	72(%rsp), %rcx
	movq	280(%rsp), %rdx
	vpbroadcastq	.LOCAL(%rip), %xmm0
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
	je	.LOCAL
	shrq	$6, %rax
	movl	$1, %r14d
	subq	%rax, %r14
	jmp	.LOCAL
	.p2align	4
.LOCAL:
	addq	$80, %rbx
	incq	%r14
	cmpq	$1, %r14
	je	.LOCAL
.LOCAL:
	cmpq	$-1, (%rbx)
	je	.LOCAL
.LOCAL:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_core::ir::term::TermValue>
.LOCAL:
	jmp	.LOCAL
	.p2align	4
.LOCAL:
	movq	288(%rsp), %rax
	movq	72(%rsp), %rcx
	movq	%rax, 472(%rsp)
	movq	%rcx, 480(%rsp)
	movq	%r15, 488(%rsp)
.LOCAL:
	leaq	336(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<core::option::Option<purrdf_core::ir::term::TermValue>>>
.LOCAL:
	cmpl	$1, 88(%rsp)
	movq	40(%rsp), %rcx
	movq	440(%rsp), %r14
	je	.LOCAL
	vmovdqu	472(%rsp), %xmm0
	movq	488(%rsp), %rax
	movq	%rbp, %rbx
	movq	%rax, 400(%rsp)
	vmovdqa	%xmm0, 384(%rsp)
	movq	%rax, 16(%rcx)
	vmovdqu	%xmm0, (%rcx)
	cmpq	%r14, %rbp
	jne	.LOCAL
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	movq	<pyo3::instance::Py<_> as core::ops::drop::Drop>::drop::drop_slow@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
.LOCAL:
	jmp	.LOCAL
.LOCAL:
	movq	24(%rsp), %rax
	movzbl	8(%rax), %ebx
	movq	<purrdf_native::py_store::query::PyQueryBoolean as pyo3::impl_::pyclass::PyClassImpl>::lazy_type_object::TYPE_OBJECT@GOTPCREL(%rip), %rax
	movl	88(%rax), %ecx
	testl	%ecx, %ecx
	jne	.LOCAL
	addq	$80, %rax
.LOCAL:
	movq	(%rax), %rdx
.LOCAL:
	movq	PyBaseObject_Type@GOTPCREL(%rip), %rsi
	movq	<pyo3::internal::pyclass_init::PyNativeTypeInitializer<_> as pyo3::internal::pyclass_init::PyObjectInit<_>>::into_new_object::inner@GOTPCREL(%rip), %rax
	leaq	616(%rsp), %rdi
	callq	*%rax
.LOCAL:
	movq	624(%rsp), %rax
	cmpb	$0, 616(%rsp)
	je	.LOCAL
	vmovdqu	632(%rsp), %ymm0
	movq	664(%rsp), %rcx
	movq	%rcx, 48(%rbp)
	vmovdqu	%ymm0, 16(%rbp)
	movq	%rax, 8(%rbp)
	movq	$1, (%rbp)
	movq	24(%rsp), %rdi
	cmpq	$0, (%rdi)
	jns	.LOCAL
	jmp	.LOCAL
.LOCAL:
	movb	%bl, 16(%rax)
	movq	$0, 24(%rax)
	movq	%rax, 8(%rbp)
	movq	$0, (%rbp)
	movq	24(%rsp), %rdi
	cmpq	$0, (%rdi)
	jns	.LOCAL
	jmp	.LOCAL
.LOCAL:
	movq	64(%rsp), %rbp
	movq	%r14, %r12
.LOCAL:
	vmovdqa	.LOCAL(%rip), %ymm0
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
	je	.LOCAL
	shrq	$3, %rax
	movl	$1, %r14d
	subq	%rax, %r14
	.p2align	4
.LOCAL:
.LOCAL:
	movq	%r12, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::model::RdfQuad>
.LOCAL:
	incq	%r14
	addq	$440, %r12
	cmpq	$1, %r14
	jne	.LOCAL
.LOCAL:
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
	je	.LOCAL
	cmpq	$359, %rsi
	ja	.LOCAL
	testq	%rsi, %rsi
	je	.LOCAL
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LOCAL:
	movl	$8, %ebx
	jmp	.LOCAL
.LOCAL:
	vmovups	144(%rsp), %xmm0
	movq	160(%rsp), %rax
	movq	%rax, 1088(%rsp)
	vmovaps	%xmm0, 1072(%rsp)
	movq	$0, 1096(%rsp)
.LOCAL:
	leaq	616(%rsp), %rdi
	leaq	1072(%rsp), %rsi
	callq	<pyo3::pyclass_init::PyClassInitializer<purrdf_native::py_store::query::PyQueryQuads>>::create_class_object
.LOCAL:
	cmpb	$0, 616(%rsp)
	je	.LOCAL
	movq	664(%rsp), %rax
	vmovups	632(%rsp), %ymm0
	movl	$1, %ecx
	movq	%rax, 48(%rbp)
	movq	624(%rsp), %rax
	vmovups	%ymm0, 16(%rbp)
	jmp	.LOCAL
.LOCAL:
	leaq	96(%rsp), %rax
	movq	%rbp, 208(%rsp)
	vmovdqu	(%rax), %ymm0
	vmovups	16(%rax), %ymm1
	vmovdqu	%ymm0, 384(%rsp)
	vmovups	%ymm1, 400(%rsp)
.LOCAL:
	leaq	472(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>
.LOCAL:
	cmpq	$0, 144(%rsp)
	movq	16(%rsp), %r13
	movq	32(%rsp), %r12
	je	.LOCAL
	cmpq	$0, 168(%rsp)
	je	.LOCAL
	movq	176(%rsp), %r15
	movq	184(%rsp), %rbx
	testq	%r15, %r15
	je	.LOCAL
	movq	(%rbx), %rax
	testq	%rax, %rax
	je	.LOCAL
.LOCAL:
	movq	%r15, %rdi
	callq	*%rax
.LOCAL:
.LOCAL:
	movq	8(%rbx), %rsi
	testq	%rsi, %rsi
	je	.LOCAL
	movq	16(%rbx), %rdx
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
	jmp	.LOCAL
.LOCAL:
	movq	16(%rsp), %r13
	movq	32(%rsp), %r12
	movq	%r14, %rbp
	movq	%r14, %r15
	jmp	.LOCAL
.LOCAL:
	movq	__rustc::__rust_realloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	movq	%r15, %rcx
	vzeroupper
	callq	*%rax
	movq	%rax, %rbx
	testq	%rax, %rax
	je	.LOCAL
.LOCAL:
.LOCAL:
	leaq	88(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::model::RdfQuad>>
.LOCAL:
	movq	%r14, 1072(%rsp)
	movq	%rbx, 1080(%rsp)
	movq	%r13, 1088(%rsp)
	movq	$0, 1096(%rsp)
.LOCAL:
	leaq	616(%rsp), %rdi
	leaq	1072(%rsp), %rsi
	callq	<pyo3::pyclass_init::PyClassInitializer<purrdf_native::py_store::query::PyQueryTriples>>::create_class_object
.LOCAL:
	movq	624(%rsp), %rax
	cmpb	$0, 616(%rsp)
	je	.LOCAL
	vmovdqu	632(%rsp), %ymm0
	movq	664(%rsp), %rcx
	movq	%rcx, 48(%rbp)
	movl	$1, %ecx
	vmovdqu	%ymm0, 16(%rbp)
	jmp	.LOCAL
.LOCAL:
	movq	624(%rsp), %rax
.LOCAL:
	xorl	%ecx, %ecx
.LOCAL:
	movq	%rax, 8(%rbp)
	movq	%rcx, (%rbp)
	movq	384(%rsp), %rcx
	lock		decq	(%rcx)
	jne	.LOCAL
	#MEMBARRIER
.LOCAL:
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	384(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LOCAL:
	jmp	.LOCAL
.LOCAL:
	movq	272(%rsp), %rax
	cmpq	$0, (%rax)
	jle	.LOCAL
	movq	_Py_DecRef@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.LOCAL:
	vmovdqu	384(%rsp), %ymm0
	vmovups	400(%rsp), %ymm1
	movq	40(%rsp), %r15
	movq	$1, 144(%rsp)
	vmovdqu	%ymm0, 152(%rsp)
	vmovups	%ymm1, 168(%rsp)
.LOCAL:
	movq	%r15, %rdx
	subq	%r13, %rdx
	movabsq	$-6148914691236517205, %rax
	vmovdqa	.LOCAL(%rip), %ymm0
	mulxq	%rax, %rbx, %rbx
	shrq	$4, %rbx
	subq	%rbp, %r14
	movq	%r14, %rdx
	mulxq	%rax, %rax, %rax
	movq	%r13, 616(%rsp)
	movq	%rbx, 624(%rsp)
	movq	%r12, 632(%rsp)
	vmovdqu	%ymm0, 200(%rsp)
	je	.LOCAL
	shrq	$4, %rax
	movl	$1, %r14d
	subq	%rax, %r14
	.p2align	4
.LOCAL:
.LOCAL:
	movq	%rbp, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>>
.LOCAL:
	incq	%r14
	addq	$24, %rbp
	cmpq	$1, %r14
	jne	.LOCAL
.LOCAL:
.LOCAL:
	leaq	200(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>>>
.LOCAL:
	cmpl	$1, 144(%rsp)
	jne	.LOCAL
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
	je	.LOCAL
	movl	$1, %r13d
	subq	%rbx, %r13
	movq	16(%rsp), %rbx
	.p2align	4
.LOCAL:
.LOCAL:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>
.LOCAL:
	incq	%r13
	addq	$24, %rbx
	cmpq	$1, %r13
	jne	.LOCAL
.LOCAL:
	movq	32(%rsp), %rax
	testq	%rax, %rax
	je	.LOCAL
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rbx
	movq	16(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,2), %rsi
	callq	*%rbx
.LOCAL:
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
	je	.LOCAL
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r12
	leaq	8(%rbx), %r15
	jmp	.LOCAL
	.p2align	4
.LOCAL:
	addq	$24, %r15
	decq	%r14
	je	.LOCAL
.LOCAL:
	movq	-8(%r15), %rsi
	testq	%rsi, %rsi
	je	.LOCAL
	movq	(%r15), %rdi
	movl	$1, %edx
	callq	*%r12
	jmp	.LOCAL
.LOCAL:
	movq	248(%rsp), %rax
	movq	256(%rsp), %rbp
	movq	%r12, 448(%rsp)
	movq	%r13, 456(%rsp)
	movq	%rbx, 464(%rsp)
	movq	240(%rsp), %rbx
	movq	%rax, 48(%rsp)
	leaq	(,%rbp,8), %rax
	leaq	(%rax,%rax,2), %r14
.LOCAL:
	movq	alloc::sync::arcinner_layout_for_value_layout@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r14, %rsi
	callq	*%rax
.LOCAL:
	movq	%rax, %r13
	movq	%rdx, %r12
	testq	%rdx, %rdx
	je	.LOCAL
	movq	__rustc::__rust_no_alloc_shim_is_unstable_v2@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	__rustc::__rust_alloc@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	movq	%r13, %rsi
	callq	*%rax
	movq	%rax, %r15
	testq	%r15, %r15
	je	.LOCAL
.LOCAL:
	movq	48(%rsp), %r12
	movq	memcpy@GOTPCREL(%rip), %rax
	leaq	16(%r15), %rdi
	movq	$1, (%r15)
	movq	$1, 8(%r15)
	movq	%r14, %rdx
	movq	%r12, %rsi
	callq	*%rax
	testq	%rbx, %rbx
	je	.LOCAL
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	shlq	$3, %rbx
	movl	$8, %edx
	movq	%r12, %rdi
	leaq	(%rbx,%rbx,2), %rsi
	callq	*%rax
.LOCAL:
	vmovdqu	448(%rsp), %xmm0
	movq	464(%rsp), %rax
	movq	%rax, 1088(%rsp)
	vmovdqa	%xmm0, 1072(%rsp)
	movq	%r15, 1096(%rsp)
	movq	%rbp, 1104(%rsp)
	movq	$0, 1112(%rsp)
.LOCAL:
	leaq	616(%rsp), %rdi
	leaq	1072(%rsp), %rsi
	callq	<pyo3::pyclass_init::PyClassInitializer<purrdf_native::py_store::query::PyQuerySolutions>>::create_class_object
.LOCAL:
	movq	624(%rsp), %rax
	cmpl	$1, 616(%rsp)
	movq	64(%rsp), %rdx
	jne	.LOCAL
	vmovdqu	632(%rsp), %ymm0
	movq	664(%rsp), %rcx
	movq	%rcx, 48(%rdx)
	vmovdqu	%ymm0, 16(%rdx)
	movq	%rax, 8(%rdx)
	movq	$1, (%rdx)
	movq	24(%rsp), %rdi
	cmpq	$0, (%rdi)
	jns	.LOCAL
	jmp	.LOCAL
.LOCAL:
	movq	240(%rsp), %rax
	testq	%rax, %rax
	je	.LOCAL
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r14
	shlq	$3, %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	leaq	(%rax,%rax,2), %rsi
	callq	*%r14
.LOCAL:
	movq	24(%rsp), %rdi
	cmpq	$0, (%rdi)
	js	.LOCAL
.LOCAL:
	movq	48(%rdi), %rax
	lock		decq	(%rax)
	jne	.LOCAL
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	addq	$48, %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LOCAL:
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
.LOCAL:
	.cfi_def_cfa_offset 1328
	movq	%rax, 8(%rdx)
	movq	$0, (%rdx)
	movq	24(%rsp), %rdi
	cmpq	$0, (%rdi)
	jns	.LOCAL
	jmp	.LOCAL
.LOCAL:
	movq	%r13, %r15
	testq	%r15, %r15
	jne	.LOCAL
.LOCAL:
.LOCAL:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r12, %rsi
	callq	*%rax
.LOCAL:
	jmp	.LOCAL
.LOCAL:
	movq	%r12, %r14
	movq	%rbp, 208(%rsp)
.LOCAL:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$24, %esi
	callq	*%rax
.LOCAL:
	jmp	.LOCAL
.LOCAL:
	movq	%rbp, 208(%rsp)
.LOCAL:
	movq	core::result::unwrap_failed@GOTPCREL(%rip), %rax
	movq	16(%rsp), %rbx
	leaq	.Lalloc_cc656815297f75969399c3f4b1ad3de4(%rip), %rdi
	leaq	.VTFMT(%rip), %rcx
	leaq	.Lalloc_d4c8062c4f28c49e31e589e7f415a063(%rip), %r8
	leaq	15(%rsp), %rdx
	movl	$55, %esi
	callq	*%rax
.LOCAL:
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	leaq	616(%rsp), %rdi
	callq	<pyo3::impl_::pyclass::lazy_type_object::LazyTypeObject<purrdf_native::py_store::query::PyQueryBoolean>>::try_init
.LOCAL:
	cmpb	$0, 616(%rsp)
	je	.LOCAL
.LOCAL:
	movq	pyo3::impl_::pyclass::lazy_type_object::type_object_init_failed@GOTPCREL(%rip), %rax
	leaq	624(%rsp), %rdi
	leaq	.Lalloc_c061a48059fbd8e0b91c7f3c14785c78(%rip), %rsi
	movl	$12, %edx
	callq	*%rax
.LOCAL:
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r15, %rsi
	callq	*%rax
.LOCAL:
.LOCAL:
	ud2
.LOCAL:
.LOCAL:
	movq	<pyo3::instance::Py<_> as core::ops::drop::Drop>::drop::drop_slow@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.LOCAL:
	jmp	.LOCAL
.LOCAL:
	movq	624(%rsp), %rax
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	movq	%rax, %r12
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	movq	%rax, %r14
	movq	%rbp, 208(%rsp)
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	movq	8(%rbx), %rsi
	movq	%rax, %r12
	testq	%rsi, %rsi
	je	.LOCAL
	movq	16(%rbx), %rdx
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.LOCAL:
	vmovdqu	384(%rsp), %ymm0
	vmovups	400(%rsp), %ymm1
	leaq	152(%rsp), %rax
	movq	$1, 144(%rsp)
	vmovups	%ymm1, 16(%rax)
	vmovdqu	%ymm0, (%rax)
.LOCAL:
	movq	16(%rsp), %rdi
	movq	40(%rsp), %rsi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDrop<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>>
.LOCAL:
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LOCAL:
.LOCAL:
	movq	16(%rsp), %rbx
	movq	40(%rsp), %r14
	movq	%rax, %r12
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	movq	%rbp, 208(%rsp)
	movq	%r14, %rcx
	movq	%rax, %r14
	movq	8(%rcx), %rsi
	testq	%rsi, %rsi
	je	.LOCAL
	movq	16(%rcx), %rdx
	movq	48(%rsp), %rdi
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	callq	*%rax
.LOCAL:
	movl	80(%rsp), %ecx
	leaq	96(%rsp), %rax
	leaq	.VTERR(%rip), %rdx
	movq	$1, 88(%rsp)
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%xmm0, (%rax)
	movq	$1, 112(%rsp)
	movq	%r12, 120(%rsp)
	movq	%rdx, 128(%rsp)
	movl	$3, 136(%rsp)
	movl	%ecx, 140(%rsp)
.LOCAL:
	movq	72(%rsp), %rdi
	movq	%r15, %rsi
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDrop<core::option::Option<purrdf_core::model::RdfTerm>>>
.LOCAL:
	movq	16(%rsp), %rbx
	movq	%r14, %r12
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LOCAL:
.LOCAL:
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	movq	%rax, %r12
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	movq	16(%rsp), %rbx
	movq	%rax, %r12
	movq	%rbp, 208(%rsp)
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	movq	%rax, %rbp
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	leaq	632(%rsp), %rdi
	movq	%rax, %rbp
	movq	%r13, 96(%rsp)
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_core::diagnostic::RdfLocation>>
.LOCAL:
	movq	%rbx, %rdi
	movq	%r15, %rsi
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>>
.LOCAL:
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LOCAL:
.LOCAL:
	movq	%rax, %r12
	testq	%r13, %r13
	je	.LOCAL
	negq	%r13
	addq	$24, %rbx
.LOCAL:
.LOCAL:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>
.LOCAL:
	addq	$24, %rbx
	decq	%r13
	jne	.LOCAL
.LOCAL:
	cmpq	$0, 32(%rsp)
	je	.LOCAL
	movq	32(%rsp), %rax
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rbx
	movq	16(%rsp), %rdi
	movl	$8, %edx
	shlq	$3, %rax
	leaq	(%rax,%rax,2), %rsi
	callq	*%rbx
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LOCAL:
.LOCAL:
	movq	16(%rsp), %rbx
	movq	%rax, %r12
	movq	%rbp, 208(%rsp)
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	movq	%rax, %rbp
	testq	%r14, %r14
	je	.LOCAL
	negq	%r14
	addq	$440, %r12
	.p2align	4
.LOCAL:
.LOCAL:
	movq	%r12, %rdi
	callq	core::ptr::drop_glue::<purrdf_core::model::RdfQuad>
.LOCAL:
	addq	$440, %r12
	decq	%r14
	jne	.LOCAL
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LOCAL:
.LOCAL:
	movq	%rax, %r12
	testq	%r14, %r14
	je	.LOCAL
	negq	%r14
	addq	$24, %rbp
.LOCAL:
.LOCAL:
	movq	%rbp, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>>
.LOCAL:
	addq	$24, %rbp
	decq	%r14
	jne	.LOCAL
.LOCAL:
.LOCAL:
	leaq	616(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>, alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>>
.LOCAL:
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LOCAL:
.LOCAL:
	movq	16(%rsp), %rbx
	movq	%rax, %r12
	movq	%rbp, 208(%rsp)
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	movq	%rax, %r12
	movq	%rbp, 208(%rsp)
	testq	%r14, %r14
	jne	.LOCAL
.LOCAL:
.LOCAL:
	leaq	616(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<core::option::Option<purrdf_core::ir::term::TermValue>, core::option::Option<purrdf_core::model::RdfTerm>>>
.LOCAL:
	movq	16(%rsp), %rbx
	jmp	.LOCAL
.LOCAL:
	addq	$80, %rbx
	negq	%r14
	jmp	.LOCAL
.LOCAL:
	addq	$80, %rbx
	decq	%r14
	je	.LOCAL
.LOCAL:
	cmpq	$-1, (%rbx)
	je	.LOCAL
.LOCAL:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_core::ir::term::TermValue>
.LOCAL:
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LOCAL:
.LOCAL:
	movq	%rax, %rbp
.LOCAL:
.LOCAL:
	leaq	616(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<purrdf_core::model::RdfQuad, purrdf_core::model::RdfTriple>>
.LOCAL:
.LOCAL:
.LOCAL:
	leaq	88(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::model::RdfQuad>>
.LOCAL:
.LOCAL:
	movq	384(%rsp), %rax
	lock		decq	(%rax)
	jne	.LOCAL
	#MEMBARRIER
.LOCAL:
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	384(%rsp), %rdi
	callq	*%rax
.LOCAL:
	movq	24(%rsp), %rdi
	movq	%rbp, %r12
	jmp	.LOCAL
.LOCAL:
	movq	24(%rsp), %rdi
	movq	%rbp, %r12
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LOCAL:
.LOCAL:
	movq	%rax, %r12
.LOCAL:
	leaq	448(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>>
.LOCAL:
	movq	24(%rsp), %rdi
	jmp	.LOCAL
.LOCAL:
.LOCAL:
.LOCAL:
	movq	24(%rsp), %rdi
	movq	%rax, %r12
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	movq	%rax, %r12
.LOCAL:
	movq	296(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LOCAL
	movq	304(%rsp), %rdi
	jmp	.LOCAL
.LOCAL:
.LOCAL:
	movq	%rax, %r12
	movq	%rbx, %rsi
	testq	%rbx, %rbx
	movq	16(%rsp), %rbx
	je	.LOCAL
	movq	%r14, %rdi
.LOCAL:
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LOCAL:
	leaq	80(%r15), %rax
	movq	%rax, 344(%rsp)
.LOCAL:
	movq	72(%rsp), %rdi
	movq	%r15, %rsi
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDrop<core::option::Option<purrdf_core::model::RdfTerm>>>
.LOCAL:
.LOCAL:
.LOCAL:
	leaq	336(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<core::option::Option<purrdf_core::ir::term::TermValue>>>
.LOCAL:
.LOCAL:
	cmpq	$0, 88(%rsp)
	movq	40(%rsp), %r14
	je	.LOCAL
.LOCAL:
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<pyo3::err::PyErr>
.LOCAL:
.LOCAL:
.LOCAL:
	movq	%rbx, %rdi
	movq	%r14, %rsi
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDrop<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>>
.LOCAL:
.LOCAL:
.LOCAL:
	leaq	200(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>>>
.LOCAL:
.LOCAL:
	cmpq	$0, 144(%rsp)
	je	.LOCAL
	leaq	152(%rsp), %rdi
.LOCAL:
	callq	core::ptr::drop_glue::<pyo3::err::PyErr>
.LOCAL:
.LOCAL:
	leaq	240(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::string::String>>
	movq	24(%rsp), %rdi
.LOCAL:
	cmpq	$0, (%rdi)
	js	.LOCAL
	movq	48(%rdi), %rax
	lock		decq	(%rax)
	jne	.LOCAL
	addq	$48, %rdi
	#MEMBARRIER
.LOCAL:
	movq	<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.LOCAL:
.LOCAL:
	movq	%r12, %rdi
	callq	_Unwind_Resume@PLT
.LOCAL:
.LOCAL:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LOCAL:
.LOCAL:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LOCAL:
.LOCAL:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LOCAL:
.LOCAL:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LOCAL:
.LOCAL:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LOCAL:
.LOCAL:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LOCAL:
.LOCAL:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LOCAL:
