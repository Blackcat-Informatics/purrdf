<purrdf_native::py_store::quad_store::PyQuadStore>::__pymethod_query__:
.Lfunc_begin1848:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception1848
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
	jne	.LBB3012_2
	vmovups	24(%rsp), %ymm1
	vmovups	8(%rsp), %ymm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%ymm0, 8(%r14)
	movq	$1, (%r14)
	jmp	.LBB3012_117
.LBB3012_2:
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
	je	.LBB3012_4
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
	jmp	.LBB3012_117
.LBB3012_4:
	movq	480(%rsp), %rsi
	addq	$16, %r15
	movq	%r15, 632(%rsp)
	movq	$0, 624(%rsp)
.Ltmp42268:
	.cfi_escape 0x2e, 0x00
	movq	<&str as pyo3::conversion::FromPyObject>::extract@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	callq	*%rax
.Ltmp42269:
	cmpl	$1, (%rsp)
	je	.LBB3012_118
	movq	8(%rsp), %r13
	movq	16(%rsp), %rbp
	movq	488(%rsp), %r12
	testq	%r12, %r12
	je	.LBB3012_14
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB3012_14
	movq	8(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movq	PyType_GetFlags@GOTPCREL(%rip), %rax
	callq	*%rax
	testl	$536870912, %eax
	jne	.LBB3012_20
	.cfi_escape 0x2e, 0x00
	movq	PyDict_Type@GOTPCREL(%rip), %r15
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.Ltmp42270:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r15, %rdx
	callq	*%rax
.Ltmp42271:
	leaq	8(%rsp), %rdi
.Ltmp42272:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_f98cafa4d6296fc0caeb65b9f5347e73(%rip), %rsi
	leaq	56(%rsp), %rcx
	movl	$13, %edx
	callq	*%rax
.Ltmp42273:
	vmovups	16(%rsp), %ymm0
	movq	48(%rsp), %rcx
	movq	8(%rsp), %rax
	movq	%rcx, 48(%r14)
	vmovups	%ymm0, 16(%r14)
	movq	%rax, 8(%r14)
.LBB3012_12:
	movq	$1, (%r14)
	lock		decq	(%rbx)
	jmp	.LBB3012_117
.LBB3012_14:
	xorl	%eax, %eax
	movq	496(%rsp), %r12
	movq	%rax, 392(%rsp)
	testq	%r12, %r12
	je	.LBB3012_21
.LBB3012_15:
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB3012_22
.Ltmp42274:
	.cfi_escape 0x2e, 0x00
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	<alloc::vec::Vec<alloc::string::String> as pyo3::conversion::FromPyObject>::extract
.Ltmp42275:
	movq	8(%rsp), %rax
	cmpb	$0, (%rsp)
	leaq	16(%rsp), %rcx
	je	.LBB3012_23
	vmovups	(%rcx), %ymm0
	movq	32(%rcx), %rdx
	leaq	64(%rsp), %rdi
	movq	%rdx, 144(%rsp)
	movq	%rdx, 40(%rsp)
	vmovups	%ymm0, 8(%rsp)
	vmovups	%ymm0, 112(%rsp)
	movq	%rax, (%rsp)
.Ltmp42276:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_908475b3e6cbafcd4d8e4fa6ac07f48c(%rip), %rsi
	movl	$20, %edx
	movq	%rsp, %rcx
	vzeroupper
	callq	*%rax
.Ltmp42277:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %ymm0
	vmovups	%ymm1, 192(%rsp)
	vmovups	%ymm0, 176(%rsp)
	vmovups	192(%rsp), %ymm1
	vmovups	176(%rsp), %ymm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%ymm0, 8(%r14)
	movq	$1, (%r14)
	jmp	.LBB3012_31
.LBB3012_20:
	leaq	224(%rsp), %rax
	movq	%r12, 224(%rsp)
	movq	496(%rsp), %r12
	movq	%rax, 392(%rsp)
	testq	%r12, %r12
	jne	.LBB3012_15
.LBB3012_21:
	movq	$-1, 64(%rsp)
	jmp	.LBB3012_25
.LBB3012_22:
	movq	$-1, %rax
	jmp	.LBB3012_24
.LBB3012_23:
	vmovups	(%rcx), %xmm0
	vmovaps	%xmm0, 112(%rsp)
.LBB3012_24:
	vmovaps	112(%rsp), %xmm0
	vmovups	%xmm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBB3012_25:
	movq	80(%rsp), %rax
	movq	64(%rsp), %rcx
	movq	72(%rsp), %rdx
	movq	504(%rsp), %r12
	movq	%rbx, 168(%rsp)
	movq	%rcx, 256(%rsp)
	movq	%rdx, 264(%rsp)
	movq	%rax, 272(%rsp)
	testq	%r12, %r12
	je	.LBB3012_32
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB3012_33
.Ltmp42278:
	.cfi_escape 0x2e, 0x00
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	<alloc::vec::Vec<alloc::string::String> as pyo3::conversion::FromPyObject>::extract
.Ltmp42279:
	movq	8(%rsp), %rax
	cmpb	$0, (%rsp)
	leaq	16(%rsp), %rcx
	je	.LBB3012_34
	vmovups	(%rcx), %ymm0
	movq	32(%rcx), %rdx
	leaq	64(%rsp), %rdi
	movq	%rdx, 144(%rsp)
	movq	%rdx, 40(%rsp)
	vmovups	%ymm0, 8(%rsp)
	vmovups	%ymm0, 112(%rsp)
	movq	%rax, (%rsp)
.Ltmp42280:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_ed816c88becfff4b944384e4ed1786c6(%rip), %rsi
	movl	$22, %edx
	movq	%rsp, %rcx
	vzeroupper
	callq	*%rax
.Ltmp42281:
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
	jne	.LBB3012_69
.LBB3012_31:
	lock		decq	(%rbx)
	jmp	.LBB3012_117
.LBB3012_32:
	movq	$-1, 64(%rsp)
	jmp	.LBB3012_36
.LBB3012_33:
	movq	$-1, %rax
	jmp	.LBB3012_35
.LBB3012_34:
	vmovups	(%rcx), %xmm0
	vmovaps	%xmm0, 112(%rsp)
.LBB3012_35:
	vmovaps	112(%rsp), %xmm0
	vmovups	%xmm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBB3012_36:
	movq	80(%rsp), %rax
	movq	64(%rsp), %rcx
	movq	72(%rsp), %rdx
	movq	512(%rsp), %r12
	movq	%rcx, 320(%rsp)
	movq	%rdx, 328(%rsp)
	movq	%rax, 336(%rsp)
	testq	%r12, %r12
	je	.LBB3012_42
	leaq	16(%rsp), %rbx
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB3012_43
.Ltmp42283:
	.cfi_escape 0x2e, 0x00
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	<(alloc::string::String, alloc::string::String) as pyo3::conversion::FromPyObject>::extract
.Ltmp42284:
	movq	8(%rsp), %rax
	cmpl	$1, (%rsp)
	jne	.LBB3012_44
	vmovups	(%rbx), %ymm0
	movq	32(%rbx), %rcx
	leaq	64(%rsp), %rdi
	movq	%rcx, 216(%rsp)
	vmovups	%ymm0, 184(%rsp)
	movq	%rax, 176(%rsp)
.Ltmp42285:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_93df691527a5f68f324bb7548b51c806(%rip), %rsi
	leaq	176(%rsp), %rcx
	movl	$21, %edx
	vzeroupper
	callq	*%rax
.Ltmp42286:
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
	jmp	.LBB3012_68
.LBB3012_42:
	movq	$-1, 64(%rsp)
	jmp	.LBB3012_45
.LBB3012_43:
	movq	$-1, %rax
.LBB3012_44:
	vmovups	(%rbx), %ymm0
	movq	32(%rbx), %rcx
	movq	168(%rsp), %rbx
	movq	%rcx, 104(%rsp)
	vmovups	%ymm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBB3012_45:
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
	je	.LBB3012_51
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB3012_51
	movq	8(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movq	PyType_GetFlags@GOTPCREL(%rip), %rax
	callq	*%rax
	testl	$536870912, %eax
	jne	.LBB3012_56
	.cfi_escape 0x2e, 0x00
	movq	PyDict_Type@GOTPCREL(%rip), %r15
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.Ltmp42288:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r15, %rdx
	callq	*%rax
.Ltmp42289:
	leaq	8(%rsp), %rdi
.Ltmp42290:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_9865433ad06614f46384a300efcb7195(%rip), %rsi
	leaq	56(%rsp), %rcx
	movl	$9, %edx
	callq	*%rax
.Ltmp42291:
	jmp	.LBB3012_65
.LBB3012_51:
	xorl	%eax, %eax
	movq	528(%rsp), %r12
	movq	%rax, 384(%rsp)
	testq	%r12, %r12
	jne	.LBB3012_52
	jmp	.LBB3012_58
.LBB3012_56:
	leaq	232(%rsp), %rax
	movq	%r12, 232(%rsp)
	movq	528(%rsp), %r12
	movq	%rax, 384(%rsp)
	testq	%r12, %r12
	je	.LBB3012_58
.LBB3012_52:
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB3012_58
	movq	8(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movq	PyType_GetFlags@GOTPCREL(%rip), %rax
	callq	*%rax
	testl	$536870912, %eax
	jne	.LBB3012_59
	.cfi_escape 0x2e, 0x00
	movq	PyDict_Type@GOTPCREL(%rip), %r15
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.Ltmp42292:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r15, %rdx
	callq	*%rax
.Ltmp42293:
	leaq	8(%rsp), %rdi
.Ltmp42294:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_fcd42a11584ddca09ee4bceee5bc0781(%rip), %rsi
	leaq	56(%rsp), %rcx
	movl	$20, %edx
	callq	*%rax
.Ltmp42295:
	jmp	.LBB3012_65
.LBB3012_58:
	xorl	%eax, %eax
	movq	536(%rsp), %r12
	movq	%rax, 376(%rsp)
	testq	%r12, %r12
	jne	.LBB3012_61
	jmp	.LBB3012_78
.LBB3012_59:
	leaq	240(%rsp), %rax
	movq	%r12, 240(%rsp)
	movq	536(%rsp), %r12
	movq	%rax, 376(%rsp)
	testq	%r12, %r12
	je	.LBB3012_78
.LBB3012_61:
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB3012_78
	movq	8(%r12), %rdi
	.cfi_escape 0x2e, 0x00
	movq	PyType_GetFlags@GOTPCREL(%rip), %rax
	callq	*%rax
	testl	$536870912, %eax
	jne	.LBB3012_79
	.cfi_escape 0x2e, 0x00
	movq	PyDict_Type@GOTPCREL(%rip), %r15
	movq	_Py_IncRef@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.Ltmp42296:
	.cfi_escape 0x2e, 0x00
	movq	<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r15, %rdx
	callq	*%rax
.Ltmp42297:
	leaq	8(%rsp), %rdi
.Ltmp42298:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_2f742b7bf960932f7bae3af5028d09e0(%rip), %rsi
	leaq	56(%rsp), %rcx
	movl	$14, %edx
	callq	*%rax
.Ltmp42299:
.LBB3012_65:
	vmovups	16(%rsp), %ymm0
	movq	48(%rsp), %rcx
	movq	8(%rsp), %rax
	movq	%rcx, 48(%r14)
	vmovups	%ymm0, 16(%r14)
	movq	%rax, 8(%r14)
.LBB3012_66:
	movq	$1, (%r14)
.LBB3012_67:
	.cfi_escape 0x2e, 0x00
	leaq	400(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
.LBB3012_68:
	.cfi_escape 0x2e, 0x00
	leaq	320(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	movq	256(%rsp), %r12
	cmpq	$-1, %r12
	je	.LBB3012_31
.LBB3012_69:
	movq	264(%rsp), %r15
	movq	272(%rsp), %r13
	testq	%r13, %r13
	je	.LBB3012_74
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rbx
	leaq	8(%r15), %rbp
	jmp	.LBB3012_72
	.p2align	4
.LBB3012_71:
	addq	$24, %rbp
	decq	%r13
	je	.LBB3012_74
.LBB3012_72:
	movq	-8(%rbp), %rsi
	testq	%rsi, %rsi
	je	.LBB3012_71
	movq	(%rbp), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%rbx
	jmp	.LBB3012_71
.LBB3012_74:
	testq	%r12, %r12
	je	.LBB3012_76
	shlq	$3, %r12
	leaq	(%r12,%r12,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%r15, %rdi
	vzeroupper
	callq	*%rax
.LBB3012_76:
	movq	168(%rsp), %rax
	lock		decq	(%rax)
	jmp	.LBB3012_117
.LBB3012_78:
	xorl	%eax, %eax
	movq	544(%rsp), %r12
	movq	%rax, 368(%rsp)
	testq	%r12, %r12
	je	.LBB3012_86
.LBB3012_81:
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB3012_87
.Ltmp42300:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::string::String as pyo3::conversion::FromPyObject>::extract@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	*%rax
.Ltmp42301:
	movq	8(%rsp), %rax
	cmpb	$0, (%rsp)
	leaq	16(%rsp), %rcx
	je	.LBB3012_88
	vmovups	(%rcx), %ymm0
	movq	32(%rcx), %rdx
	leaq	64(%rsp), %rdi
	movq	%rdx, 144(%rsp)
	movq	%rdx, 40(%rsp)
	vmovups	%ymm0, 8(%rsp)
	vmovups	%ymm0, 112(%rsp)
	movq	%rax, (%rsp)
.Ltmp42302:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_589d20955c603729090b6aaccd02222f(%rip), %rsi
	movl	$19, %edx
	movq	%rsp, %rcx
	vzeroupper
	callq	*%rax
.Ltmp42303:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %ymm0
	vmovups	%ymm1, 192(%rsp)
	vmovups	%ymm0, 176(%rsp)
	vmovups	192(%rsp), %ymm1
	vmovups	176(%rsp), %ymm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%ymm0, 8(%r14)
	jmp	.LBB3012_66
.LBB3012_79:
	leaq	248(%rsp), %rax
	movq	%r12, 248(%rsp)
	movq	544(%rsp), %r12
	movq	%rax, 368(%rsp)
	testq	%r12, %r12
	jne	.LBB3012_81
.LBB3012_86:
	movq	$-1, 64(%rsp)
	jmp	.LBB3012_90
.LBB3012_87:
	movq	$-1, %rax
	jmp	.LBB3012_89
.LBB3012_88:
	vmovups	(%rcx), %xmm0
	vmovaps	%xmm0, 112(%rsp)
.LBB3012_89:
	vmovaps	112(%rsp), %xmm0
	vmovups	%xmm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBB3012_90:
	movq	80(%rsp), %rax
	movq	64(%rsp), %rcx
	movq	72(%rsp), %rdx
	movq	552(%rsp), %r12
	movq	%r13, 360(%rsp)
	movq	%rcx, 288(%rsp)
	movq	%rdx, 296(%rsp)
	movq	%rax, 304(%rsp)
	testq	%r12, %r12
	je	.LBB3012_96
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	xorl	%r13d, %r13d
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB3012_97
.Ltmp42305:
	.cfi_escape 0x2e, 0x00
	movq	<&str as pyo3::conversion::FromPyObject>::extract@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	*%rax
.Ltmp42306:
	movq	8(%rsp), %r13
	movq	16(%rsp), %rax
	cmpb	$0, (%rsp)
	je	.LBB3012_98
	vmovups	24(%rsp), %ymm0
	leaq	64(%rsp), %rdi
	vmovups	%ymm0, 192(%rsp)
	movq	%r13, 176(%rsp)
	movq	%rax, 184(%rsp)
.Ltmp42307:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_6c43a98927907a5f8539235d09232c7e(%rip), %rsi
	leaq	176(%rsp), %rcx
	movl	$11, %edx
	vzeroupper
	callq	*%rax
.Ltmp42308:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %xmm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%xmm0, 8(%r14)
	jmp	.LBB3012_105
.LBB3012_96:
	xorl	%r13d, %r13d
	jmp	.LBB3012_99
.LBB3012_97:
	jmp	.LBB3012_99
.LBB3012_98:
	movq	%rax, 352(%rsp)
.LBB3012_99:
	movq	560(%rsp), %r12
	testq	%r12, %r12
	je	.LBB3012_108
	.cfi_escape 0x2e, 0x00
	movq	Py_GetConstantBorrowed@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
	cmpq	%r12, %rax
	je	.LBB3012_109
.Ltmp42309:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::string::String as pyo3::conversion::FromPyObject>::extract@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	movq	%r12, %rsi
	callq	*%rax
.Ltmp42310:
	movq	8(%rsp), %rax
	cmpb	$0, (%rsp)
	leaq	16(%rsp), %rcx
	je	.LBB3012_110
	vmovups	(%rcx), %ymm0
	movq	32(%rcx), %rdx
	leaq	64(%rsp), %rdi
	movq	%rdx, 144(%rsp)
	movq	%rdx, 40(%rsp)
	vmovups	%ymm0, 8(%rsp)
	vmovups	%ymm0, 112(%rsp)
	movq	%rax, (%rsp)
.Ltmp42311:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_8497b6da903267d7e7d13af19cddca66(%rip), %rsi
	movl	$8, %edx
	movq	%rsp, %rcx
	vzeroupper
	callq	*%rax
.Ltmp42312:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %ymm0
	vmovups	%ymm1, 192(%rsp)
	vmovups	%ymm0, 176(%rsp)
	vmovups	192(%rsp), %ymm1
	vmovups	176(%rsp), %ymm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%ymm0, 8(%r14)
.LBB3012_105:
	movq	288(%rsp), %rsi
	movq	$1, (%r14)
	cmpq	$-1, %rsi
	je	.LBB3012_67
	testq	%rsi, %rsi
	je	.LBB3012_67
	movq	296(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
	jmp	.LBB3012_67
.LBB3012_108:
	movq	%rbx, %r12
	movq	$-1, 64(%rsp)
	jmp	.LBB3012_112
.LBB3012_109:
	movq	%rbx, %r12
	movq	$-1, %rax
	jmp	.LBB3012_111
.LBB3012_110:
	vmovups	(%rcx), %xmm0
	movq	%rbx, %r12
	vmovaps	%xmm0, 112(%rsp)
.LBB3012_111:
	vmovaps	112(%rsp), %xmm0
	vmovups	%xmm0, 72(%rsp)
	movq	%rax, 64(%rsp)
.LBB3012_112:
	movq	64(%rsp), %rcx
	movq	80(%rsp), %rax
	movq	%rcx, 448(%rsp)
	movq	72(%rsp), %rcx
	movq	%rax, 464(%rsp)
	movq	%rcx, 456(%rsp)
.Ltmp42314:
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
.Ltmp42315:
	movq	576(%rsp), %rax
	cmpl	$1, 568(%rsp)
	jne	.LBB3012_115
	vmovups	584(%rsp), %ymm0
	movq	616(%rsp), %rcx
	movq	%rcx, 48(%r14)
	movl	$1, %ecx
	vmovups	%ymm0, 16(%r14)
	jmp	.LBB3012_116
.LBB3012_115:
	xorl	%ecx, %ecx
.LBB3012_116:
	movq	%rax, 8(%r14)
	movq	%rcx, (%r14)
	lock		decq	(%r12)
.LBB3012_117:
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
.LBB3012_118:
	.cfi_def_cfa_offset 736
	leaq	8(%rsp), %rax
	leaq	64(%rsp), %rdi
	vmovups	16(%rax), %ymm1
	vmovups	(%rax), %ymm0
	vmovups	%ymm1, 192(%rsp)
	vmovups	%ymm0, 176(%rsp)
.Ltmp42317:
	.cfi_escape 0x2e, 0x00
	movq	pyo3::impl_::extract_argument::argument_extraction_error@GOTPCREL(%rip), %rax
	leaq	.Lalloc_da5c5f922604d9376dbdf48c863f8565(%rip), %rsi
	leaq	176(%rsp), %rcx
	movl	$5, %edx
	vzeroupper
	callq	*%rax
.Ltmp42318:
	vmovups	80(%rsp), %ymm1
	vmovups	64(%rsp), %xmm0
	vmovups	%ymm1, 24(%r14)
	vmovups	%xmm0, 8(%r14)
	jmp	.LBB3012_12
.LBB3012_120:
.Ltmp42316:
	movq	%rax, %r14
	lock		decq	(%r12)
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB3012_121:
.Ltmp42287:
	movq	168(%rsp), %r12
	movq	%rax, %r14
	jmp	.LBB3012_128
.LBB3012_122:
.Ltmp42313:
	movq	288(%rsp), %rsi
	movq	%rbx, %r12
	movq	%rax, %r14
	cmpq	$-1, %rsi
	je	.LBB3012_127
	testq	%rsi, %rsi
	je	.LBB3012_127
	movq	296(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
	jmp	.LBB3012_127
.LBB3012_125:
.Ltmp42282:
	movq	%rbx, %r12
	movq	%rax, %r14
	jmp	.LBB3012_129
.LBB3012_126:
.Ltmp42304:
	movq	%rbx, %r12
	movq	%rax, %r14
.LBB3012_127:
	.cfi_escape 0x2e, 0x00
	leaq	400(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
.LBB3012_128:
	.cfi_escape 0x2e, 0x00
	leaq	320(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
.LBB3012_129:
	.cfi_escape 0x2e, 0x00
	leaq	256(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
	lock		decq	(%r12)
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB3012_130:
.Ltmp42319:
	movq	%rbx, %r12
	movq	%rax, %r14
	lock		decq	(%r12)
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end3012:
