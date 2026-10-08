<purrdf_native::py_store::quad_store::PyQuadStore>::__pymethod_query__:
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
	jne	.LBBFN_2
	vmovups	24(%rsp), %ymm1
	vmovups	8(%rsp), %ymm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%ymm0, 8(%r14)
	movq	$1, (%r14)
	jmp	.LBBFN_117
.LBBFN_2:
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
	je	.LBBFN_4
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
	jmp	.LBBFN_117
.LBBFN_4:
	movq	480(%rsp), %rsi
	addq	$16, %r15
	movq	%r15, 632(%rsp)
	movq	$0, 624(%rsp)
.LtmpEH:
	.cfi_escape 0x2e, 0x00
	movq	<&str as pyo3::conversion::FromPyObject>::extract@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	callq	*%rax
.LtmpEH:
	cmpl	$1, (%rsp)
	je	.LBBFN_118
	movq	8(%rsp), %r13
	movq	16(%rsp), %rbp
	movq	488(%rsp), %r12
	testq	%r12, %r12
	je	.LBBFN_14
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBBFN_14
	movq	8(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movq	PyType_GetFlags@GOTPCREL(%rip), %rax
	callq	*%rax
	testl	$536870912, %eax
	jne	.LBBFN_20
	.cfi_escape 0x2e, 0x00
	movq	PyDict_Type@GOTPCREL(%rip), %r15
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.LtmpEH:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r15, %rdx
	callq	*%rax
.LtmpEH:
	leaq	8(%rsp), %rdi
.LtmpEH:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_f98cafa4d6296fc0caeb65b9f5347e73(%rip), %rsi
	leaq	56(%rsp), %rcx
	movl	$13, %edx
	callq	*%rax
.LtmpEH:
	vmovups	16(%rsp), %ymm0
	movq	48(%rsp), %rcx
	movq	8(%rsp), %rax
	movq	%rcx, 48(%r14)
	vmovups	%ymm0, 16(%r14)
	movq	%rax, 8(%r14)
.LBBFN_12:
	movq	$1, (%r14)
	lock		decq	(%rbx)
	jmp	.LBBFN_117
.LBBFN_14:
	xorl	%eax, %eax
	movq	496(%rsp), %r12
	movq	%rax, 392(%rsp)
	testq	%r12, %r12
	je	.LBBFN_21
.LBBFN_15:
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBBFN_22
.LtmpEH:
	.cfi_escape 0x2e, 0x00
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	<alloc::vec::Vec<alloc::string::String> as pyo3::conversion::FromPyObject>::extract
.LtmpEH:
	movq	8(%rsp), %rax
	cmpb	$0, (%rsp)
	leaq	16(%rsp), %rcx
	je	.LBBFN_23
	vmovups	(%rcx), %ymm0
	movq	32(%rcx), %rdx
	leaq	64(%rsp), %rdi
	movq	%rdx, 144(%rsp)
	movq	%rdx, 40(%rsp)
	vmovups	%ymm0, 8(%rsp)
	vmovups	%ymm0, 112(%rsp)
	movq	%rax, (%rsp)
.LtmpEH:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_908475b3e6cbafcd4d8e4fa6ac07f48c(%rip), %rsi
	movl	$20, %edx
	movq	%rsp, %rcx
	vzeroupper
	callq	*%rax
.LtmpEH:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %ymm0
	vmovups	%ymm1, 192(%rsp)
	vmovups	%ymm0, 176(%rsp)
	vmovups	192(%rsp), %ymm1
	vmovups	176(%rsp), %ymm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%ymm0, 8(%r14)
	movq	$1, (%r14)
	jmp	.LBBFN_31
.LBBFN_20:
	leaq	224(%rsp), %rax
	movq	%r12, 224(%rsp)
	movq	496(%rsp), %r12
	movq	%rax, 392(%rsp)
	testq	%r12, %r12
	jne	.LBBFN_15
.LBBFN_21:
	movq	$-1, 64(%rsp)
	jmp	.LBBFN_25
.LBBFN_22:
	movq	$-1, %rax
	jmp	.LBBFN_24
.LBBFN_23:
	vmovups	(%rcx), %xmm0
	vmovaps	%xmm0, 112(%rsp)
.LBBFN_24:
	vmovaps	112(%rsp), %xmm0
	vmovups	%xmm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBBFN_25:
	movq	80(%rsp), %rax
	movq	64(%rsp), %rcx
	movq	72(%rsp), %rdx
	movq	504(%rsp), %r12
	movq	%rbx, 168(%rsp)
	movq	%rcx, 256(%rsp)
	movq	%rdx, 264(%rsp)
	movq	%rax, 272(%rsp)
	testq	%r12, %r12
	je	.LBBFN_32
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBBFN_33
.LtmpEH:
	.cfi_escape 0x2e, 0x00
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	<alloc::vec::Vec<alloc::string::String> as pyo3::conversion::FromPyObject>::extract
.LtmpEH:
	movq	8(%rsp), %rax
	cmpb	$0, (%rsp)
	leaq	16(%rsp), %rcx
	je	.LBBFN_34
	vmovups	(%rcx), %ymm0
	movq	32(%rcx), %rdx
	leaq	64(%rsp), %rdi
	movq	%rdx, 144(%rsp)
	movq	%rdx, 40(%rsp)
	vmovups	%ymm0, 8(%rsp)
	vmovups	%ymm0, 112(%rsp)
	movq	%rax, (%rsp)
.LtmpEH:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_ed816c88becfff4b944384e4ed1786c6(%rip), %rsi
	movl	$22, %edx
	movq	%rsp, %rcx
	vzeroupper
	callq	*%rax
.LtmpEH:
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
	jne	.LBBFN_69
.LBBFN_31:
	lock		decq	(%rbx)
	jmp	.LBBFN_117
.LBBFN_32:
	movq	$-1, 64(%rsp)
	jmp	.LBBFN_36
.LBBFN_33:
	movq	$-1, %rax
	jmp	.LBBFN_35
.LBBFN_34:
	vmovups	(%rcx), %xmm0
	vmovaps	%xmm0, 112(%rsp)
.LBBFN_35:
	vmovaps	112(%rsp), %xmm0
	vmovups	%xmm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBBFN_36:
	movq	80(%rsp), %rax
	movq	64(%rsp), %rcx
	movq	72(%rsp), %rdx
	movq	512(%rsp), %r12
	movq	%rcx, 320(%rsp)
	movq	%rdx, 328(%rsp)
	movq	%rax, 336(%rsp)
	testq	%r12, %r12
	je	.LBBFN_42
	leaq	16(%rsp), %rbx
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBBFN_43
.LtmpEH:
	.cfi_escape 0x2e, 0x00
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	<(alloc::string::String, alloc::string::String) as pyo3::conversion::FromPyObject>::extract
.LtmpEH:
	movq	8(%rsp), %rax
	cmpl	$1, (%rsp)
	jne	.LBBFN_44
	vmovups	(%rbx), %ymm0
	movq	32(%rbx), %rcx
	leaq	64(%rsp), %rdi
	movq	%rcx, 216(%rsp)
	vmovups	%ymm0, 184(%rsp)
	movq	%rax, 176(%rsp)
.LtmpEH:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_93df691527a5f68f324bb7548b51c806(%rip), %rsi
	leaq	176(%rsp), %rcx
	movl	$21, %edx
	vzeroupper
	callq	*%rax
.LtmpEH:
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
	jmp	.LBBFN_68
.LBBFN_42:
	movq	$-1, 64(%rsp)
	jmp	.LBBFN_45
.LBBFN_43:
	movq	$-1, %rax
.LBBFN_44:
	vmovups	(%rbx), %ymm0
	movq	32(%rbx), %rcx
	movq	168(%rsp), %rbx
	movq	%rcx, 104(%rsp)
	vmovups	%ymm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBBFN_45:
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
	je	.LBBFN_51
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBBFN_51
	movq	8(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movq	PyType_GetFlags@GOTPCREL(%rip), %rax
	callq	*%rax
	testl	$536870912, %eax
	jne	.LBBFN_56
	.cfi_escape 0x2e, 0x00
	movq	PyDict_Type@GOTPCREL(%rip), %r15
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.LtmpEH:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r15, %rdx
	callq	*%rax
.LtmpEH:
	leaq	8(%rsp), %rdi
.LtmpEH:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_9865433ad06614f46384a300efcb7195(%rip), %rsi
	leaq	56(%rsp), %rcx
	movl	$9, %edx
	callq	*%rax
.LtmpEH:
	jmp	.LBBFN_65
.LBBFN_51:
	xorl	%eax, %eax
	movq	528(%rsp), %r12
	movq	%rax, 384(%rsp)
	testq	%r12, %r12
	jne	.LBBFN_52
	jmp	.LBBFN_58
.LBBFN_56:
	leaq	232(%rsp), %rax
	movq	%r12, 232(%rsp)
	movq	528(%rsp), %r12
	movq	%rax, 384(%rsp)
	testq	%r12, %r12
	je	.LBBFN_58
.LBBFN_52:
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBBFN_58
	movq	8(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movq	PyType_GetFlags@GOTPCREL(%rip), %rax
	callq	*%rax
	testl	$536870912, %eax
	jne	.LBBFN_59
	.cfi_escape 0x2e, 0x00
	movq	PyDict_Type@GOTPCREL(%rip), %r15
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.LtmpEH:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r15, %rdx
	callq	*%rax
.LtmpEH:
	leaq	8(%rsp), %rdi
.LtmpEH:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_fcd42a11584ddca09ee4bceee5bc0781(%rip), %rsi
	leaq	56(%rsp), %rcx
	movl	$20, %edx
	callq	*%rax
.LtmpEH:
	jmp	.LBBFN_65
.LBBFN_58:
	xorl	%eax, %eax
	movq	536(%rsp), %r12
	movq	%rax, 376(%rsp)
	testq	%r12, %r12
	jne	.LBBFN_61
	jmp	.LBBFN_78
.LBBFN_59:
	leaq	240(%rsp), %rax
	movq	%r12, 240(%rsp)
	movq	536(%rsp), %r12
	movq	%rax, 376(%rsp)
	testq	%r12, %r12
	je	.LBBFN_78
.LBBFN_61:
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBBFN_78
	movq	8(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movq	PyType_GetFlags@GOTPCREL(%rip), %rax
	callq	*%rax
	testl	$536870912, %eax
	jne	.LBBFN_79
	.cfi_escape 0x2e, 0x00
	movq	PyDict_Type@GOTPCREL(%rip), %r15
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.LtmpEH:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r15, %rdx
	callq	*%rax
.LtmpEH:
	leaq	8(%rsp), %rdi
.LtmpEH:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_2f742b7bf960932f7bae3af5028d09e0(%rip), %rsi
	leaq	56(%rsp), %rcx
	movl	$14, %edx
	callq	*%rax
.LtmpEH:
.LBBFN_65:
	vmovups	16(%rsp), %ymm0
	movq	48(%rsp), %rcx
	movq	8(%rsp), %rax
	movq	%rcx, 48(%r14)
	vmovups	%ymm0, 16(%r14)
	movq	%rax, 8(%r14)
.LBBFN_66:
	movq	$1, (%r14)
.LBBFN_67:
	.cfi_escape 0x2e, 0x00
	leaq	400(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
.LBBFN_68:
	.cfi_escape 0x2e, 0x00
	leaq	320(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	movq	256(%rsp), %r12
	cmpq	$-1, %r12
	je	.LBBFN_31
.LBBFN_69:
	movq	264(%rsp), %r15
	movq	272(%rsp), %r13
	testq	%r13, %r13
	je	.LBBFN_74
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rbx
	leaq	8(%r15), %rbp
	jmp	.LBBFN_72
	.p2align	4
.LBBFN_71:
	addq	$24, %rbp
	decq	%r13
	je	.LBBFN_74
.LBBFN_72:
	movq	-8(%rbp), %rsi
	testq	%rsi, %rsi
	je	.LBBFN_71
	movq	(%rbp), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%rbx
	jmp	.LBBFN_71
.LBBFN_74:
	testq	%r12, %r12
	je	.LBBFN_76
	shlq	$3, %r12
	leaq	(%r12,%r12,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%r15, %rdi
	vzeroupper
	callq	*%rax
.LBBFN_76:
	movq	168(%rsp), %rax
	lock		decq	(%rax)
	jmp	.LBBFN_117
.LBBFN_78:
	xorl	%eax, %eax
	movq	544(%rsp), %r12
	movq	%rax, 368(%rsp)
	testq	%r12, %r12
	je	.LBBFN_86
.LBBFN_81:
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBBFN_87
.LtmpEH:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::string::String as pyo3::conversion::FromPyObject>::extract@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	*%rax
.LtmpEH:
	movq	8(%rsp), %rax
	cmpb	$0, (%rsp)
	leaq	16(%rsp), %rcx
	je	.LBBFN_88
	vmovups	(%rcx), %ymm0
	movq	32(%rcx), %rdx
	leaq	64(%rsp), %rdi
	movq	%rdx, 144(%rsp)
	movq	%rdx, 40(%rsp)
	vmovups	%ymm0, 8(%rsp)
	vmovups	%ymm0, 112(%rsp)
	movq	%rax, (%rsp)
.LtmpEH:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_589d20955c603729090b6aaccd02222f(%rip), %rsi
	movl	$19, %edx
	movq	%rsp, %rcx
	vzeroupper
	callq	*%rax
.LtmpEH:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %ymm0
	vmovups	%ymm1, 192(%rsp)
	vmovups	%ymm0, 176(%rsp)
	vmovups	192(%rsp), %ymm1
	vmovups	176(%rsp), %ymm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%ymm0, 8(%r14)
	jmp	.LBBFN_66
.LBBFN_79:
	leaq	248(%rsp), %rax
	movq	%r12, 248(%rsp)
	movq	544(%rsp), %r12
	movq	%rax, 368(%rsp)
	testq	%r12, %r12
	jne	.LBBFN_81
.LBBFN_86:
	movq	$-1, 64(%rsp)
	jmp	.LBBFN_90
.LBBFN_87:
	movq	$-1, %rax
	jmp	.LBBFN_89
.LBBFN_88:
	vmovups	(%rcx), %xmm0
	vmovaps	%xmm0, 112(%rsp)
.LBBFN_89:
	vmovaps	112(%rsp), %xmm0
	vmovups	%xmm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBBFN_90:
	movq	80(%rsp), %rax
	movq	64(%rsp), %rcx
	movq	72(%rsp), %rdx
	movq	552(%rsp), %r12
	movq	%r13, 360(%rsp)
	movq	%rcx, 288(%rsp)
	movq	%rdx, 296(%rsp)
	movq	%rax, 304(%rsp)
	testq	%r12, %r12
	je	.LBBFN_96
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	xorl	%r13d, %r13d
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBBFN_97
.LtmpEH:
	.cfi_escape 0x2e, 0x00
	movq	<&str as pyo3::conversion::FromPyObject>::extract@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	*%rax
.LtmpEH:
	movq	8(%rsp), %r13
	movq	16(%rsp), %rax
	cmpb	$0, (%rsp)
	je	.LBBFN_98
	vmovups	24(%rsp), %ymm0
	leaq	64(%rsp), %rdi
	vmovups	%ymm0, 192(%rsp)
	movq	%r13, 176(%rsp)
	movq	%rax, 184(%rsp)
.LtmpEH:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_6c43a98927907a5f8539235d09232c7e(%rip), %rsi
	leaq	176(%rsp), %rcx
	movl	$11, %edx
	vzeroupper
	callq	*%rax
.LtmpEH:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %xmm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%xmm0, 8(%r14)
	jmp	.LBBFN_105
.LBBFN_96:
	xorl	%r13d, %r13d
	jmp	.LBBFN_99
.LBBFN_97:
	jmp	.LBBFN_99
.LBBFN_98:
	movq	%rax, 352(%rsp)
.LBBFN_99:
	movq	560(%rsp), %r12
	testq	%r12, %r12
	je	.LBBFN_108
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBBFN_109
.LtmpEH:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::string::String as pyo3::conversion::FromPyObject>::extract@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	*%rax
.LtmpEH:
	movq	8(%rsp), %rax
	cmpb	$0, (%rsp)
	leaq	16(%rsp), %rcx
	je	.LBBFN_110
	vmovups	(%rcx), %ymm0
	movq	32(%rcx), %rdx
	leaq	64(%rsp), %rdi
	movq	%rdx, 144(%rsp)
	movq	%rdx, 40(%rsp)
	vmovups	%ymm0, 8(%rsp)
	vmovups	%ymm0, 112(%rsp)
	movq	%rax, (%rsp)
.LtmpEH:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_8497b6da903267d7e7d13af19cddca66(%rip), %rsi
	movl	$8, %edx
	movq	%rsp, %rcx
	vzeroupper
	callq	*%rax
.LtmpEH:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %ymm0
	vmovups	%ymm1, 192(%rsp)
	vmovups	%ymm0, 176(%rsp)
	vmovups	192(%rsp), %ymm1
	vmovups	176(%rsp), %ymm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%ymm0, 8(%r14)
.LBBFN_105:
	movq	288(%rsp), %rsi
	movq	$1, (%r14)
	cmpq	$-1, %rsi
	je	.LBBFN_67
	testq	%rsi, %rsi
	je	.LBBFN_67
	movq	296(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
	jmp	.LBBFN_67
.LBBFN_108:
	movq	%rbx, %r12
	movq	$-1, 64(%rsp)
	jmp	.LBBFN_112
.LBBFN_109:
	movq	%rbx, %r12
	movq	$-1, %rax
	jmp	.LBBFN_111
.LBBFN_110:
	vmovups	(%rcx), %xmm0
	movq	%rbx, %r12
	vmovaps	%xmm0, 112(%rsp)
.LBBFN_111:
	vmovaps	112(%rsp), %xmm0
	vmovups	%xmm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBBFN_112:
	movq	64(%rsp), %rcx
	movq	80(%rsp), %rax
	movq	%rcx, 448(%rsp)
	movq	72(%rsp), %rcx
	movq	%rax, 464(%rsp)
	movq	%rcx, 456(%rsp)
.LtmpEH:
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
.LtmpEH:
	movq	576(%rsp), %rax
	cmpl	$1, 568(%rsp)
	jne	.LBBFN_115
	vmovups	584(%rsp), %ymm0
	movq	616(%rsp), %rcx
	movq	%rcx, 48(%r14)
	movl	$1, %ecx
	vmovups	%ymm0, 16(%r14)
	jmp	.LBBFN_116
.LBBFN_115:
	xorl	%ecx, %ecx
.LBBFN_116:
	movq	%rax, 8(%r14)
	movq	%rcx, (%r14)
	lock		decq	(%r12)
.LBBFN_117:
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
.LBBFN_118:
	.cfi_def_cfa_offset 736
	leaq	8(%rsp), %rax
	leaq	64(%rsp), %rdi
	vmovups	16(%rax), %ymm1
	vmovups	(%rax), %ymm0
	vmovups	%ymm1, 192(%rsp)
	vmovups	%ymm0, 176(%rsp)
.LtmpEH:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_da5c5f922604d9376dbdf48c863f8565(%rip), %rsi
	leaq	176(%rsp), %rcx
	movl	$5, %edx
	vzeroupper
	callq	*%rax
.LtmpEH:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %xmm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%xmm0, 8(%r14)
	jmp	.LBBFN_12
.LBBFN_120:
.LtmpEH:
	movq	%rax, %r14
	lock		decq	(%r12)
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBBFN_121:
.LtmpEH:
	movq	168(%rsp), %r12
	movq	%rax, %r14
	jmp	.LBBFN_128
.LBBFN_122:
.LtmpEH:
	movq	288(%rsp), %rsi
	movq	%rbx, %r12
	movq	%rax, %r14
	cmpq	$-1, %rsi
	je	.LBBFN_127
	testq	%rsi, %rsi
	je	.LBBFN_127
	movq	296(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
	jmp	.LBBFN_127
.LBBFN_125:
.LtmpEH:
	movq	%rbx, %r12
	movq	%rax, %r14
	jmp	.LBBFN_129
.LBBFN_126:
.LtmpEH:
	movq	%rbx, %r12
	movq	%rax, %r14
.LBBFN_127:
	.cfi_escape 0x2e, 0x00
	leaq	400(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
.LBBFN_128:
	.cfi_escape 0x2e, 0x00
	leaq	320(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
.LBBFN_129:
	.cfi_escape 0x2e, 0x00
	leaq	256(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	lock		decq	(%r12)
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBBFN_130:
.LtmpEH:
	movq	%rbx, %r12
	movq	%rax, %r14
	lock		decq	(%r12)
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_endFN:
