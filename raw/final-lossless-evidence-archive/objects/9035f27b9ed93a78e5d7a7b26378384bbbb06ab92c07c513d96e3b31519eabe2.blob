<purrdf_native::py_store::quad_store::PyQuadStore>::__pymethod_query__:
.Lfunc_beginID:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .LexceptionID
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
	leaq	.LALLOCIDe7258a9778c8e4642ffd2ad0599(%rip), %rsi
	leaq	8(%rsp), %rdi
	leaq	488(%rsp), %r9
	pushq	$11
	.cfi_adjust_cfa_offset 8
	vzeroupper
	callq	<pyo3::impl_::extract_argument::FunctionDescription>::extract_arguments_fastcall::<pyo3::impl_::extract_argument::NoVarargs, pyo3::impl_::extract_argument::NoVarkeywords>
	addq	$16, %rsp
	.cfi_adjust_cfa_offset -16
	cmpl	$1, (%rsp)
	jne	.LBBID_2
	vmovups	24(%rsp), %ymm1
	vmovups	8(%rsp), %ymm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%ymm0, 8(%r14)
	movq	$1, (%r14)
	jmp	.LBBID_117
.LBBID_2:
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
	je	.LBBID_4
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
	jmp	.LBBID_117
.LBBID_4:
	movq	480(%rsp), %rsi
	addq	$16, %r15
	movq	%r15, 632(%rsp)
	movq	$0, 624(%rsp)
.LtmpID:
	.cfi_escape 0x2e, 0x00
	movq	<&str as pyo3::conversion::FromPyObject>::extract@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	callq	*%rax
.LtmpID:
	cmpl	$1, (%rsp)
	je	.LBBID_118
	movq	8(%rsp), %r13
	movq	16(%rsp), %rbp
	movq	488(%rsp), %r12
	testq	%r12, %r12
	je	.LBBID_14
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBBID_14
	movq	8(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movq	PyType_GetFlags@GOTPCREL(%rip), %rax
	callq	*%rax
	testl	$536870912, %eax
	jne	.LBBID_20
	.cfi_escape 0x2e, 0x00
	movq	PyDict_Type@GOTPCREL(%rip), %r15
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.LtmpID:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r15, %rdx
	callq	*%rax
.LtmpID:
	leaq	8(%rsp), %rdi
.LtmpID:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.LALLOCIDcafa4d6296fc0caeb65b9f5347e73(%rip), %rsi
	leaq	56(%rsp), %rcx
	movl	$13, %edx
	callq	*%rax
.LtmpID:
	vmovups	16(%rsp), %ymm0
	movq	48(%rsp), %rcx
	movq	8(%rsp), %rax
	movq	%rcx, 48(%r14)
	vmovups	%ymm0, 16(%r14)
	movq	%rax, 8(%r14)
.LBBID_12:
	movq	$1, (%r14)
	lock		decq	(%rbx)
	jmp	.LBBID_117
.LBBID_14:
	xorl	%eax, %eax
	movq	496(%rsp), %r12
	movq	%rax, 392(%rsp)
	testq	%r12, %r12
	je	.LBBID_21
.LBBID_15:
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBBID_22
.LtmpID:
	.cfi_escape 0x2e, 0x00
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	<alloc::vec::Vec<alloc::string::String> as pyo3::conversion::FromPyObject>::extract
.LtmpID:
	movq	8(%rsp), %rax
	cmpb	$0, (%rsp)
	leaq	16(%rsp), %rcx
	je	.LBBID_23
	vmovups	(%rcx), %ymm0
	movq	32(%rcx), %rdx
	leaq	64(%rsp), %rdi
	movq	%rdx, 144(%rsp)
	movq	%rdx, 40(%rsp)
	vmovups	%ymm0, 8(%rsp)
	vmovups	%ymm0, 112(%rsp)
	movq	%rax, (%rsp)
.LtmpID:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_IDb3e6cbafcd4d8e4fa6ac07f48c(%rip), %rsi
	movl	$20, %edx
	movq	%rsp, %rcx
	vzeroupper
	callq	*%rax
.LtmpID:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %ymm0
	vmovups	%ymm1, 192(%rsp)
	vmovups	%ymm0, 176(%rsp)
	vmovups	192(%rsp), %ymm1
	vmovups	176(%rsp), %ymm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%ymm0, 8(%r14)
	movq	$1, (%r14)
	jmp	.LBBID_31
.LBBID_20:
	leaq	224(%rsp), %rax
	movq	%r12, 224(%rsp)
	movq	496(%rsp), %r12
	movq	%rax, 392(%rsp)
	testq	%r12, %r12
	jne	.LBBID_15
.LBBID_21:
	movq	$-1, 64(%rsp)
	jmp	.LBBID_25
.LBBID_22:
	movq	$-1, %rax
	jmp	.LBBID_24
.LBBID_23:
	vmovups	(%rcx), %xmm0
	vmovaps	%xmm0, 112(%rsp)
.LBBID_24:
	vmovaps	112(%rsp), %xmm0
	vmovups	%xmm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBBID_25:
	movq	80(%rsp), %rax
	movq	64(%rsp), %rcx
	movq	72(%rsp), %rdx
	movq	504(%rsp), %r12
	movq	%rbx, 168(%rsp)
	movq	%rcx, 256(%rsp)
	movq	%rdx, 264(%rsp)
	movq	%rax, 272(%rsp)
	testq	%r12, %r12
	je	.LBBID_32
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBBID_33
.LtmpID:
	.cfi_escape 0x2e, 0x00
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	<alloc::vec::Vec<alloc::string::String> as pyo3::conversion::FromPyObject>::extract
.LtmpID:
	movq	8(%rsp), %rax
	cmpb	$0, (%rsp)
	leaq	16(%rsp), %rcx
	je	.LBBID_34
	vmovups	(%rcx), %ymm0
	movq	32(%rcx), %rdx
	leaq	64(%rsp), %rdi
	movq	%rdx, 144(%rsp)
	movq	%rdx, 40(%rsp)
	vmovups	%ymm0, 8(%rsp)
	vmovups	%ymm0, 112(%rsp)
	movq	%rax, (%rsp)
.LtmpID:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.LALLOCIDc88becfff4b944384e4ed1786c6(%rip), %rsi
	movl	$22, %edx
	movq	%rsp, %rcx
	vzeroupper
	callq	*%rax
.LtmpID:
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
	jne	.LBBID_69
.LBBID_31:
	lock		decq	(%rbx)
	jmp	.LBBID_117
.LBBID_32:
	movq	$-1, 64(%rsp)
	jmp	.LBBID_36
.LBBID_33:
	movq	$-1, %rax
	jmp	.LBBID_35
.LBBID_34:
	vmovups	(%rcx), %xmm0
	vmovaps	%xmm0, 112(%rsp)
.LBBID_35:
	vmovaps	112(%rsp), %xmm0
	vmovups	%xmm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBBID_36:
	movq	80(%rsp), %rax
	movq	64(%rsp), %rcx
	movq	72(%rsp), %rdx
	movq	512(%rsp), %r12
	movq	%rcx, 320(%rsp)
	movq	%rdx, 328(%rsp)
	movq	%rax, 336(%rsp)
	testq	%r12, %r12
	je	.LBBID_42
	leaq	16(%rsp), %rbx
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBBID_43
.LtmpID:
	.cfi_escape 0x2e, 0x00
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	<(alloc::string::String, alloc::string::String) as pyo3::conversion::FromPyObject>::extract
.LtmpID:
	movq	8(%rsp), %rax
	cmpl	$1, (%rsp)
	jne	.LBBID_44
	vmovups	(%rbx), %ymm0
	movq	32(%rbx), %rcx
	leaq	64(%rsp), %rdi
	movq	%rcx, 216(%rsp)
	vmovups	%ymm0, 184(%rsp)
	movq	%rax, 176(%rsp)
.LtmpID:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_IDdf691527a5f68f324bb7548b51c806(%rip), %rsi
	leaq	176(%rsp), %rcx
	movl	$21, %edx
	vzeroupper
	callq	*%rax
.LtmpID:
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
	jmp	.LBBID_68
.LBBID_42:
	movq	$-1, 64(%rsp)
	jmp	.LBBID_45
.LBBID_43:
	movq	$-1, %rax
.LBBID_44:
	vmovups	(%rbx), %ymm0
	movq	32(%rbx), %rcx
	movq	168(%rsp), %rbx
	movq	%rcx, 104(%rsp)
	vmovups	%ymm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBBID_45:
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
	je	.LBBID_51
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBBID_51
	movq	8(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movq	PyType_GetFlags@GOTPCREL(%rip), %rax
	callq	*%rax
	testl	$536870912, %eax
	jne	.LBBID_56
	.cfi_escape 0x2e, 0x00
	movq	PyDict_Type@GOTPCREL(%rip), %r15
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.LtmpID:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r15, %rdx
	callq	*%rax
.LtmpID:
	leaq	8(%rsp), %rdi
.LtmpID:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_IDad06614f46384a300efcb7195(%rip), %rsi
	leaq	56(%rsp), %rcx
	movl	$9, %edx
	callq	*%rax
.LtmpID:
	jmp	.LBBID_65
.LBBID_51:
	xorl	%eax, %eax
	movq	528(%rsp), %r12
	movq	%rax, 384(%rsp)
	testq	%r12, %r12
	jne	.LBBID_52
	jmp	.LBBID_58
.LBBID_56:
	leaq	232(%rsp), %rax
	movq	%r12, 232(%rsp)
	movq	528(%rsp), %r12
	movq	%rax, 384(%rsp)
	testq	%r12, %r12
	je	.LBBID_58
.LBBID_52:
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBBID_58
	movq	8(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movq	PyType_GetFlags@GOTPCREL(%rip), %rax
	callq	*%rax
	testl	$536870912, %eax
	jne	.LBBID_59
	.cfi_escape 0x2e, 0x00
	movq	PyDict_Type@GOTPCREL(%rip), %r15
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.LtmpID:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r15, %rdx
	callq	*%rax
.LtmpID:
	leaq	8(%rsp), %rdi
.LtmpID:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.LALLOCIDa11584ddca09ee4bceee5bc0781(%rip), %rsi
	leaq	56(%rsp), %rcx
	movl	$20, %edx
	callq	*%rax
.LtmpID:
	jmp	.LBBID_65
.LBBID_58:
	xorl	%eax, %eax
	movq	536(%rsp), %r12
	movq	%rax, 376(%rsp)
	testq	%r12, %r12
	jne	.LBBID_61
	jmp	.LBBID_78
.LBBID_59:
	leaq	240(%rsp), %rax
	movq	%r12, 240(%rsp)
	movq	536(%rsp), %r12
	movq	%rax, 376(%rsp)
	testq	%r12, %r12
	je	.LBBID_78
.LBBID_61:
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBBID_78
	movq	8(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movq	PyType_GetFlags@GOTPCREL(%rip), %rax
	callq	*%rax
	testl	$536870912, %eax
	jne	.LBBID_79
	.cfi_escape 0x2e, 0x00
	movq	PyDict_Type@GOTPCREL(%rip), %r15
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.LtmpID:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r15, %rdx
	callq	*%rax
.LtmpID:
	leaq	8(%rsp), %rdi
.LtmpID:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_IDf742b7bf960932f7bae3af5028d09e0(%rip), %rsi
	leaq	56(%rsp), %rcx
	movl	$14, %edx
	callq	*%rax
.LtmpID:
.LBBID_65:
	vmovups	16(%rsp), %ymm0
	movq	48(%rsp), %rcx
	movq	8(%rsp), %rax
	movq	%rcx, 48(%r14)
	vmovups	%ymm0, 16(%r14)
	movq	%rax, 8(%r14)
.LBBID_66:
	movq	$1, (%r14)
.LBBID_67:
	.cfi_escape 0x2e, 0x00
	leaq	400(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
.LBBID_68:
	.cfi_escape 0x2e, 0x00
	leaq	320(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	movq	256(%rsp), %r12
	cmpq	$-1, %r12
	je	.LBBID_31
.LBBID_69:
	movq	264(%rsp), %r15
	movq	272(%rsp), %r13
	testq	%r13, %r13
	je	.LBBID_74
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rbx
	leaq	8(%r15), %rbp
	jmp	.LBBID_72
	.p2align	4
.LBBID_71:
	addq	$24, %rbp
	decq	%r13
	je	.LBBID_74
.LBBID_72:
	movq	-8(%rbp), %rsi
	testq	%rsi, %rsi
	je	.LBBID_71
	movq	(%rbp), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%rbx
	jmp	.LBBID_71
.LBBID_74:
	testq	%r12, %r12
	je	.LBBID_76
	shlq	$3, %r12
	leaq	(%r12,%r12,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%r15, %rdi
	vzeroupper
	callq	*%rax
.LBBID_76:
	movq	168(%rsp), %rax
	lock		decq	(%rax)
	jmp	.LBBID_117
.LBBID_78:
	xorl	%eax, %eax
	movq	544(%rsp), %r12
	movq	%rax, 368(%rsp)
	testq	%r12, %r12
	je	.LBBID_86
.LBBID_81:
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBBID_87
.LtmpID:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::string::String as pyo3::conversion::FromPyObject>::extract@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	*%rax
.LtmpID:
	movq	8(%rsp), %rax
	cmpb	$0, (%rsp)
	leaq	16(%rsp), %rcx
	je	.LBBID_88
	vmovups	(%rcx), %ymm0
	movq	32(%rcx), %rdx
	leaq	64(%rsp), %rdi
	movq	%rdx, 144(%rsp)
	movq	%rdx, 40(%rsp)
	vmovups	%ymm0, 8(%rsp)
	vmovups	%ymm0, 112(%rsp)
	movq	%rax, (%rsp)
.LtmpID:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_IDd20955c603729090b6aaccd02222f(%rip), %rsi
	movl	$19, %edx
	movq	%rsp, %rcx
	vzeroupper
	callq	*%rax
.LtmpID:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %ymm0
	vmovups	%ymm1, 192(%rsp)
	vmovups	%ymm0, 176(%rsp)
	vmovups	192(%rsp), %ymm1
	vmovups	176(%rsp), %ymm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%ymm0, 8(%r14)
	jmp	.LBBID_66
.LBBID_79:
	leaq	248(%rsp), %rax
	movq	%r12, 248(%rsp)
	movq	544(%rsp), %r12
	movq	%rax, 368(%rsp)
	testq	%r12, %r12
	jne	.LBBID_81
.LBBID_86:
	movq	$-1, 64(%rsp)
	jmp	.LBBID_90
.LBBID_87:
	movq	$-1, %rax
	jmp	.LBBID_89
.LBBID_88:
	vmovups	(%rcx), %xmm0
	vmovaps	%xmm0, 112(%rsp)
.LBBID_89:
	vmovaps	112(%rsp), %xmm0
	vmovups	%xmm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBBID_90:
	movq	80(%rsp), %rax
	movq	64(%rsp), %rcx
	movq	72(%rsp), %rdx
	movq	552(%rsp), %r12
	movq	%r13, 360(%rsp)
	movq	%rcx, 288(%rsp)
	movq	%rdx, 296(%rsp)
	movq	%rax, 304(%rsp)
	testq	%r12, %r12
	je	.LBBID_96
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	xorl	%r13d, %r13d
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBBID_97
.LtmpID:
	.cfi_escape 0x2e, 0x00
	movq	<&str as pyo3::conversion::FromPyObject>::extract@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	*%rax
.LtmpID:
	movq	8(%rsp), %r13
	movq	16(%rsp), %rax
	cmpb	$0, (%rsp)
	je	.LBBID_98
	vmovups	24(%rsp), %ymm0
	leaq	64(%rsp), %rdi
	vmovups	%ymm0, 192(%rsp)
	movq	%r13, 176(%rsp)
	movq	%rax, 184(%rsp)
.LtmpID:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_IDc43a98927907a5f8539235d09232c7e(%rip), %rsi
	leaq	176(%rsp), %rcx
	movl	$11, %edx
	vzeroupper
	callq	*%rax
.LtmpID:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %xmm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%xmm0, 8(%r14)
	jmp	.LBBID_105
.LBBID_96:
	xorl	%r13d, %r13d
	jmp	.LBBID_99
.LBBID_97:
	jmp	.LBBID_99
.LBBID_98:
	movq	%rax, 352(%rsp)
.LBBID_99:
	movq	560(%rsp), %r12
	testq	%r12, %r12
	je	.LBBID_108
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBBID_109
.LtmpID:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::string::String as pyo3::conversion::FromPyObject>::extract@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	*%rax
.LtmpID:
	movq	8(%rsp), %rax
	cmpb	$0, (%rsp)
	leaq	16(%rsp), %rcx
	je	.LBBID_110
	vmovups	(%rcx), %ymm0
	movq	32(%rcx), %rdx
	leaq	64(%rsp), %rdi
	movq	%rdx, 144(%rsp)
	movq	%rdx, 40(%rsp)
	vmovups	%ymm0, 8(%rsp)
	vmovups	%ymm0, 112(%rsp)
	movq	%rax, (%rsp)
.LtmpID:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_IDb6da903267d7e7d13af19cddca66(%rip), %rsi
	movl	$8, %edx
	movq	%rsp, %rcx
	vzeroupper
	callq	*%rax
.LtmpID:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %ymm0
	vmovups	%ymm1, 192(%rsp)
	vmovups	%ymm0, 176(%rsp)
	vmovups	192(%rsp), %ymm1
	vmovups	176(%rsp), %ymm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%ymm0, 8(%r14)
.LBBID_105:
	movq	288(%rsp), %rsi
	movq	$1, (%r14)
	cmpq	$-1, %rsi
	je	.LBBID_67
	testq	%rsi, %rsi
	je	.LBBID_67
	movq	296(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
	jmp	.LBBID_67
.LBBID_108:
	movq	%rbx, %r12
	movq	$-1, 64(%rsp)
	jmp	.LBBID_112
.LBBID_109:
	movq	%rbx, %r12
	movq	$-1, %rax
	jmp	.LBBID_111
.LBBID_110:
	vmovups	(%rcx), %xmm0
	movq	%rbx, %r12
	vmovaps	%xmm0, 112(%rsp)
.LBBID_111:
	vmovaps	112(%rsp), %xmm0
	vmovups	%xmm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBBID_112:
	movq	64(%rsp), %rcx
	movq	80(%rsp), %rax
	movq	%rcx, 448(%rsp)
	movq	72(%rsp), %rcx
	movq	%rax, 464(%rsp)
	movq	%rcx, 456(%rsp)
.LtmpID:
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
.LtmpID:
	movq	576(%rsp), %rax
	cmpl	$1, 568(%rsp)
	jne	.LBBID_115
	vmovups	584(%rsp), %ymm0
	movq	616(%rsp), %rcx
	movq	%rcx, 48(%r14)
	movl	$1, %ecx
	vmovups	%ymm0, 16(%r14)
	jmp	.LBBID_116
.LBBID_115:
	xorl	%ecx, %ecx
.LBBID_116:
	movq	%rax, 8(%r14)
	movq	%rcx, (%r14)
	lock		decq	(%r12)
.LBBID_117:
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
.LBBID_118:
	.cfi_def_cfa_offset 736
	leaq	8(%rsp), %rax
	leaq	64(%rsp), %rdi
	vmovups	16(%rax), %ymm1
	vmovups	(%rax), %ymm0
	vmovups	%ymm1, 192(%rsp)
	vmovups	%ymm0, 176(%rsp)
.LtmpID:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.LALLOCIDc5f922604d9376dbdf48c863f8565(%rip), %rsi
	leaq	176(%rsp), %rcx
	movl	$5, %edx
	vzeroupper
	callq	*%rax
.LtmpID:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %xmm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%xmm0, 8(%r14)
	jmp	.LBBID_12
.LBBID_120:
.LtmpID:
	movq	%rax, %r14
	lock		decq	(%r12)
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBBID_121:
.LtmpID:
	movq	168(%rsp), %r12
	movq	%rax, %r14
	jmp	.LBBID_128
.LBBID_122:
.LtmpID:
	movq	288(%rsp), %rsi
	movq	%rbx, %r12
	movq	%rax, %r14
	cmpq	$-1, %rsi
	je	.LBBID_127
	testq	%rsi, %rsi
	je	.LBBID_127
	movq	296(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
	jmp	.LBBID_127
.LBBID_125:
.LtmpID:
	movq	%rbx, %r12
	movq	%rax, %r14
	jmp	.LBBID_129
.LBBID_126:
.LtmpID:
	movq	%rbx, %r12
	movq	%rax, %r14
.LBBID_127:
	.cfi_escape 0x2e, 0x00
	leaq	400(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
.LBBID_128:
	.cfi_escape 0x2e, 0x00
	leaq	320(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
.LBBID_129:
	.cfi_escape 0x2e, 0x00
	leaq	256(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	lock		decq	(%r12)
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBBID_130:
.LtmpID:
	movq	%rbx, %r12
	movq	%rax, %r14
	lock		decq	(%r12)
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_endID:
