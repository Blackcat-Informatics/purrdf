purrdf_sparql_eval::modifier::aggregate_numeric_cost:
.Lfunc_begin1924:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception1280
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
	subq	$264, %rsp
	.cfi_def_cfa_offset 320
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	%rdx, %rax
	shlq	$4, %rax
	movq	%rcx, %r12
	shrq	$32, %r12
	movq	%rcx, %r14
	movq	%rdx, %r13
	movq	%rsi, %r15
	leaq	(%rax,%rax,4), %rbx
	leaq	(%rsi,%rbx), %rbp
	testq	%rdx, %rdx
	je	.LBB1924_9
	movq	%r15, %rax
	jmp	.LBB1924_3
	.p2align	4
.LBB1924_2:
	addq	$80, %rax
	cmpq	%rbp, %rax
	je	.LBB1924_9
.LBB1924_3:
	cmpq	$0, (%rax)
	js	.LBB1924_2
	cmpq	$20, 16(%rax)
	jb	.LBB1924_2
	movq	(%rdi), %rax
	leal	-3(%rax), %ecx
	cmpl	$2, %ecx
	jb	.LBB1924_53
	cmpl	$1, %eax
	je	.LBB1924_99
	xorl	%ebx, %ebx
	cmpl	$2, %eax
	je	.LBB1924_10
	jmp	.LBB1924_101
.LBB1924_9:
	cmpl	$2, (%rdi)
	movb	$1, %bl
	jne	.LBB1924_100
.LBB1924_10:
	cmpl	$18, %r12d
	setne	%al
	orb	%r14b, %al
	testb	$1, %al
	jne	.LBB1924_12
	testl	$65280, %r14d
	sete	%al
	xorl	%ecx, %ecx
	testb	%al, %bl
	jne	.LBB1924_100
	testq	%r13, %r13
	je	.LBB1924_100
.LBB1924_14:
	movq	%rcx, 256(%rsp)
	movq	$0, 128(%rsp)
	xorl	%ebx, %ebx
	movq	$0, 24(%rsp)
	xorl	%esi, %esi
	xorl	%edi, %edi
	xorl	%eax, %eax
.LBB1924_15:
	addq	$80, %r15
	movq	%rax, 40(%rsp)
	movq	%rdi, 80(%rsp)
	movq	%rsi, 248(%rsp)
	.p2align	4
.LBB1924_16:
	cmpq	$0, -80(%r15)
	js	.LBB1924_20
	cmpq	$-1, -32(%r15)
	jne	.LBB1924_20
	movq	-40(%r15), %rsi
	cmpq	$33, %rsi
	jb	.LBB1924_20
	movq	-48(%r15), %rdi
	vmovdqu	anon.8c4c8f20c49bb342a4ce83ac1e804353.131.llvm.9305710216504555276(%rip), %ymm0
	movzbl	anon.8c4c8f20c49bb342a4ce83ac1e804353.131.llvm.9305710216504555276+32(%rip), %eax
	vpxor	(%rdi), %ymm0, %ymm0
	movzbl	32(%rdi), %ecx
	vmovd	%eax, %xmm1
	vmovd	%ecx, %xmm2
	vpternlogq	$246, %ymm2, %ymm1, %ymm0
	vptest	%ymm0, %ymm0
	je	.LBB1924_22
	.p2align	4
.LBB1924_20:
	movq	$0, 160(%rsp)
.LBB1924_21:
	leaq	-80(%r15), %rax
	addq	$80, %r15
	addq	$80, %rax
	cmpq	%rbp, %rax
	jne	.LBB1924_16
	jmp	.LBB1924_45
.LBB1924_22:
	movq	<purrdf_xsd::datatype::XsdDatatype>::from_local@GOTPCREL(%rip), %rax
	addq	$-33, %rsi
	addq	$33, %rdi
	vzeroupper
	callq	*%rax
	cmpb	$-1, %al
	je	.LBB1924_20
	movzbl	%al, %ecx
	movq	-72(%r15), %rsi
	movq	-64(%r15), %rdx
	movq	<purrdf_xsd::exact::cost::Shape>::of_lexical@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	callq	*%rax
	cmpb	$0, 160(%rsp)
	je	.LBB1924_21
	movq	40(%rsp), %rdx
	movl	196(%rsp), %eax
	movq	176(%rsp), %r11
	movq	184(%rsp), %rcx
	movq	248(%rsp), %rdi
	movq	$-1, %rsi
	movq	168(%rsp), %r8
	incq	%rdx
	movl	%eax, 52(%rsp)
	movq	%r11, %rax
	cmoveq	%rsi, %rdx
	subq	%rcx, %rax
	movl	$0, %esi
	cmovaeq	%rax, %rsi
	cmpq	%rdi, %rsi
	cmovbeq	%rdi, %rsi
	movq	80(%rsp), %rdi
	cmpq	%rdi, %rcx
	cmovaq	%rcx, %rdi
	testb	$1, 128(%rsp)
	je	.LBB1924_34
	movq	%rdx, %rax
	decq	%rax
	movq	%rdx, 40(%rsp)
	je	.LBB1924_128
	movabsq	$-2601111570856684097, %r10
	movq	%rax, %rdx
	movq	%rax, %r9
	shrq	$10, %r9
	mulxq	%r10, %rdx, %rdx
	movl	$10, %r10d
	movq	%r11, 240(%rsp)
	shrq	$33, %rdx
	cmpq	$9765625, %r9
	movl	$0, %r9d
	cmovael	%r10d, %r9d
	cmovbq	%rax, %rdx
	cmpq	$100000, %rdx
	jb	.LBB1924_28
	shrq	$5, %rdx
	movabsq	$755578637259143235, %rax
	orl	$5, %r9d
	mulxq	%rax, %rdx, %rdx
	shrq	$7, %rdx
.LBB1924_28:
	leal	393206(%rdx), %eax
	leal	524188(%rdx), %r11d
	movabsq	$2049638230412172401, %r10
	andl	%eax, %r11d
	leal	916504(%rdx), %eax
	addl	$514288, %edx
	andl	%eax, %edx
	xorl	%r11d, %edx
	shrl	$17, %edx
	leal	1(%rdx,%r9), %eax
	movq	$-1, %rdx
	movabsq	$-2049638230412172401, %r9
	addq	%rsi, %rax
	cmovbq	%rdx, %rax
	addq	%rdi, %rax
	cmovbq	%rdx, %rax
	movq	%rax, %rdx
	mulxq	%r9, %rdx, %rdx
	movabsq	$-8198552921648689607, %r9
	movq	%rax, %r11
	imulq	%r9, %r11
	xorl	%r9d, %r9d
	shrq	$3, %rdx
	cmpq	%r10, %r11
	seta	%r9b
	addq	%rdx, %r9
	cmpq	$39, 32(%rsp)
	setb	%dl
	cmpq	$19, 16(%rsp)
	setb	%r11b
	testb	%r11b, %dl
	je	.LBB1924_32
	cmpq	$39, 240(%rsp)
	setb	%dl
	cmpq	$19, %rcx
	setb	%r10b
	testb	%r10b, %dl
	je	.LBB1924_32
	cmpq	$18, 80(%rsp)
	ja	.LBB1924_32
	cmpq	$38, %rax
	jbe	.LBB1924_41
.LBB1924_32:
	movq	16(%rsp), %r10
	movq	%r10, %rdx
	subq	%rcx, %rdx
	jae	.LBB1924_35
	subq	%r10, %rcx
	movabsq	$-2049638230412172401, %r11
	movq	$-1, %r10
	movq	%rcx, %rdx
	mulxq	%r11, %rcx, %rcx
	movq	8(%rsp), %rdx
	shrq	$3, %rcx
	incq	%rcx
	addq	%rcx, %rdx
	cmovbq	%r10, %rdx
	movq	%rdx, %rcx
	movq	%rdx, 8(%rsp)
	jmp	.LBB1924_36
.LBB1924_34:
	movl	192(%rsp), %eax
	movq	%r8, 8(%rsp)
	movq	%r11, 32(%rsp)
	movq	%rcx, 16(%rsp)
	movl	%eax, 4(%rsp)
	movq	%rdx, %rax
	jmp	.LBB1924_42
.LBB1924_35:
	movabsq	$-2049638230412172401, %rcx
	mulxq	%rcx, %rcx, %rcx
	movq	$-1, %rdx
	shrq	$3, %rcx
	incq	%rcx
	addq	%rcx, %r8
	cmovbq	%rdx, %r8
	movq	%r8, %rcx
.LBB1924_36:
	movq	%rcx, %rdx
	shrq	$62, %rdx
	jne	.LBB1924_43
	leaq	(,%rcx,4), %rdx
.LBB1924_38:
	movq	8(%rsp), %r10
	movabsq	$4611686018427387903, %r11
	cmpq	%r8, %r10
	cmovaq	%r10, %r8
	movq	$-1, %r10
	incq	%r8
	cmoveq	%r10, %r8
	cmpq	%r11, %r8
	ja	.LBB1924_44
	leaq	(,%r8,4), %r10
.LBB1924_40:
	addq	%r8, %rcx
	movq	24(%rsp), %r8
	movq	$-1, %r11
	cmovbq	%r11, %rcx
	addq	%r10, %rdx
	cmovbq	%r11, %rdx
	addq	%rcx, %rbx
	cmovbq	%r11, %rbx
	cmpq	%rdx, %r8
	cmovbeq	%rdx, %r8
	movq	%r8, 24(%rsp)
.LBB1924_41:
	movq	%rax, 32(%rsp)
	movq	40(%rsp), %rax
	movl	$-1, 4(%rsp)
	movq	%r9, 8(%rsp)
	movq	%rdi, 16(%rsp)
.LBB1924_42:
	movb	$1, %cl
	movq	%rcx, 128(%rsp)
	cmpq	%rbp, %r15
	jne	.LBB1924_15
	jmp	.LBB1924_123
.LBB1924_43:
	movq	$-1, %rdx
	jmp	.LBB1924_38
.LBB1924_44:
	movq	$-1, %r10
	jmp	.LBB1924_40
.LBB1924_45:
	testb	$1, 128(%rsp)
	movq	24(%rsp), %rdx
	je	.LBB1924_102
	movq	16(%rsp), %rsi
	movq	32(%rsp), %rbp
	cmpq	$18, %rsi
	ja	.LBB1924_49
.LBB1924_47:
	cmpq	$38, %rbp
	ja	.LBB1924_49
	movl	%r14d, %eax
	movq	%r12, %rcx
	andl	$1, %eax
	xorq	$18, %rcx
	orq	%rax, %rcx
	movl	%r14d, %eax
	andl	$65280, %eax
	orq	%rcx, %rax
	je	.LBB1924_102
.LBB1924_49:
	cmpb	$0, 256(%rsp)
	je	.LBB1924_103
	cmpq	%rbp, %rsi
	jae	.LBB1924_112
	movq	8(%rsp), %rax
	movq	%rdx, %r15
	testq	%rsi, %rsi
	je	.LBB1924_113
	incq	%rbp
	movq	$-1, %rcx
	cmoveq	%rcx, %rbp
	jmp	.LBB1924_113
.LBB1924_12:
	xorl	%ecx, %ecx
	testq	%r13, %r13
	jne	.LBB1924_14
.LBB1924_100:
	xorl	%ebx, %ebx
.LBB1924_101:
	xorl	%edx, %edx
.LBB1924_102:
	movq	%rbx, %rax
	addq	$264, %rsp
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
.LBB1924_53:
	.cfi_def_cfa_offset 320
	movq	<purrdf_xsd::datatype::XsdDatatype>::from_local@GOTPCREL(%rip), %r14
	addq	$80, %r15
	addq	$-80, %rbx
	leaq	88(%rsp), %r12
	xorl	%r13d, %r13d
	.p2align	4
.LBB1924_54:
	cmpq	$0, -80(%r15)
	js	.LBB1924_58
	cmpq	$-1, -32(%r15)
	jne	.LBB1924_58
	movq	-40(%r15), %rsi
	cmpq	$33, %rsi
	jb	.LBB1924_58
	movq	-48(%r15), %rdi
	vmovdqu	anon.8c4c8f20c49bb342a4ce83ac1e804353.131.llvm.9305710216504555276(%rip), %ymm1
	movzbl	anon.8c4c8f20c49bb342a4ce83ac1e804353.131.llvm.9305710216504555276+32(%rip), %eax
	vpxor	(%rdi), %ymm1, %ymm1
	movzbl	32(%rdi), %ecx
	vmovd	%eax, %xmm2
	vmovdqu	%ymm2, 128(%rsp)
	vmovd	%ecx, %xmm0
	vpternlogq	$246, %ymm0, %ymm2, %ymm1
	vptest	%ymm1, %ymm1
	je	.LBB1924_60
	.p2align	4
.LBB1924_58:
	movq	$0, 88(%rsp)
.LBB1924_59:
	leaq	-80(%r15), %rax
	addq	$80, %r15
	addq	$-80, %rbx
	addq	$80, %rax
	cmpq	%rbp, %rax
	jne	.LBB1924_54
	jmp	.LBB1924_87
.LBB1924_60:
	addq	$-33, %rsi
	addq	$33, %rdi
	vzeroupper
	callq	*%r14
	cmpb	$-1, %al
	je	.LBB1924_58
	movzbl	%al, %ecx
	movq	-72(%r15), %rsi
	movq	-64(%r15), %rdx
	movq	<purrdf_xsd::exact::cost::Shape>::of_lexical@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	callq	*%rax
	cmpb	$0, 88(%rsp)
	je	.LBB1924_59
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$128, %edi
	movl	$128, %r13d
	callq	*%rax
	testq	%rax, %rax
	je	.LBB1924_129
	movq	%rax, %r12
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	addq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %r13
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmovbq	%rcx, %r13
	movabsq	$9223372036854775807, %rcx
	subq	$-128, %rax
	movq	%r13, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB1924_65
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB1924_65:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1924_71
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB1924_65
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	$128, (%rdx)
	movl	$128, %edx
	lock		xaddq	%rdx, (%rsi)
	subq	$-128, %rdx
	cmovoq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
.LBB1924_68:
	cmpq	%rax, %rdx
	jle	.LBB1924_70
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1924_68
.LBB1924_70:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB1924_71:
	vmovdqu	96(%rsp), %ymm0
	movq	$4, 56(%rsp)
	movq	%r12, 64(%rsp)
	movq	$1, 72(%rsp)
	vmovdqu	%ymm0, (%r12)
	testq	%rbx, %rbx
	je	.LBB1924_121
	movl	$1, %ebx
.LBB1924_73:
	addq	$80, %r15
.LBB1924_74:
	cmpq	$0, -80(%r15)
	js	.LBB1924_78
	cmpq	$-1, -32(%r15)
	jne	.LBB1924_78
	movq	-40(%r15), %rsi
	cmpq	$33, %rsi
	jb	.LBB1924_78
	movq	-48(%r15), %rdi
	vmovdqu	anon.8c4c8f20c49bb342a4ce83ac1e804353.131.llvm.9305710216504555276(%rip), %ymm1
	movzbl	32(%rdi), %eax
	vpxor	(%rdi), %ymm1, %ymm1
	vmovd	%eax, %xmm0
	vpternlogq	$246, 128(%rsp), %ymm0, %ymm1
	vptest	%ymm1, %ymm1
	je	.LBB1924_80
.LBB1924_78:
	movq	$0, 160(%rsp)
.LBB1924_79:
	leaq	-80(%r15), %rax
	addq	$80, %r15
	addq	$80, %rax
	cmpq	%rbp, %rax
	jne	.LBB1924_74
	jmp	.LBB1924_122
.LBB1924_80:
	movq	-72(%r15), %rax
	movq	-64(%r15), %r13
	addq	$-33, %rsi
	addq	$33, %rdi
	movq	%rax, 24(%rsp)
	vzeroupper
	callq	*%r14
	cmpb	$-1, %al
	je	.LBB1924_78
.Ltmp39392:
	movzbl	%al, %ecx
	movq	24(%rsp), %rsi
	movq	<purrdf_xsd::exact::cost::Shape>::of_lexical@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	movq	%r13, %rdx
	callq	*%rax
.Ltmp39393:
	cmpb	$0, 160(%rsp)
	je	.LBB1924_79
	cmpq	56(%rsp), %rbx
	jne	.LBB1924_86
.Ltmp39395:
	movl	$1, %edx
	movl	$8, %ecx
	movl	$32, %r8d
	leaq	56(%rsp), %rdi
	movq	%rbx, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.2910935600939035342)
.Ltmp39396:
	movq	64(%rsp), %r12
.LBB1924_86:
	leaq	168(%rsp), %rcx
	movq	%rbx, %rax
	shlq	$5, %rax
	incq	%rbx
	vmovdqu	(%rcx), %ymm0
	vmovdqu	%ymm0, (%r12,%rax)
	movq	%rbx, 72(%rsp)
	cmpq	%rbp, %r15
	jne	.LBB1924_73
	jmp	.LBB1924_122
.LBB1924_87:
	movl	$8, %r12d
	xorl	%ebx, %ebx
.LBB1924_88:
.Ltmp39398:
	movq	purrdf_xsd::exact::cost::compare_chain@GOTPCREL(%rip), %rax
	movl	$1, %edx
	movq	%r12, %rdi
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
.Ltmp39399:
	movq	%rax, %rbx
	testq	%r13, %r13
	je	.LBB1924_102
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$5, %r13
	movabsq	$9223372036854775807, %rcx
	movq	%rdx, %r14
	cmpq	%rcx, %r13
	cmovaeq	%rcx, %r13
	xorl	%edx, %edx
	cmpq	%r13, %rax
	setns	%dl
	addq	%rcx, %rdx
	subq	%r13, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB1924_92
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB1924_92:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1924_98
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB1924_92
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%r13, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%r13, %rdx
	setns	%al
	addq	%rcx, %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	subq	%r13, %rdx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB1924_95:
	cmpq	%rax, %rdx
	jge	.LBB1924_97
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1924_95
.LBB1924_97:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB1924_98:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	callq	*%rax
	movq	%r14, %rdx
	jmp	.LBB1924_102
.LBB1924_99:
	movb	$1, %cl
	testq	%r13, %r13
	jne	.LBB1924_14
	jmp	.LBB1924_100
.LBB1924_103:
	movq	%rdx, %r15
	movq	%r13, 176(%rsp)
	movq	$0, 184(%rsp)
	movw	$0, 160(%rsp)
.Ltmp39401:
	leaq	88(%rsp), %rdi
	leaq	160(%rsp), %rsi
	vzeroupper
	callq	purrdf_xsd::numeric::exact_path::shape_of (.llvm.9305710216504555276)
.Ltmp39402:
	cmpl	$1, 88(%rsp)
	jne	.LBB1924_125
	vmovups	96(%rsp), %ymm0
	leaq	160(%rsp), %rdi
	vmovups	%ymm0, 128(%rsp)
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_xsd::value::XsdValue>
	movq	8(%rsp), %rax
	vmovups	128(%rsp), %ymm0
	movq	16(%rsp), %r13
	movl	4(%rsp), %edx
	movl	52(%rsp), %ecx
	leaq	88(%rsp), %rdi
	leaq	160(%rsp), %rsi
	movq	%rax, 88(%rsp)
	movq	purrdf_xsd::exact::cost::decimal_div@GOTPCREL(%rip), %rax
	movq	%rbp, 96(%rsp)
	movq	%r13, 104(%rsp)
	movl	%edx, 112(%rsp)
	movq	%r14, %rdx
	movl	%ecx, 116(%rsp)
	vmovups	%ymm0, 160(%rsp)
	vzeroupper
	callq	*%rax
	vmovdqu	128(%rsp), %ymm0
	movq	%r13, %rdi
	movq	%rax, %rcx
	movq	%rdx, %rsi
	testb	$1, %r14b
	je	.LBB1924_109
	vpextrq	$1, %xmm0, %rax
	movq	%rax, %rdx
	shrq	$62, %rdx
	jne	.LBB1924_127
	shlq	$2, %rax
.LBB1924_108:
	addq	%rdi, %rax
	movq	$-1, %r12
	cmovaeq	%rax, %r12
.LBB1924_109:
	xorl	%eax, %eax
	subq	%rdi, %rbp
	vextracti128	$1, %ymm0, %xmm0
	movq	$-1, %r8
	movabsq	$-8198552921648689607, %r9
	movabsq	$2049638230412172401, %r10
	cmovaeq	%rbp, %rax
	vmovq	%xmm0, %rdi
	addq	%rax, %rdi
	movabsq	$-2049638230412172401, %rax
	cmovbq	%r8, %rdi
	incq	%rdi
	cmoveq	%r8, %rdi
	addq	%r12, %rdi
	cmovbq	%r8, %rdi
	movq	%rdi, %rdx
	mulxq	%rax, %rdx, %rdx
	imulq	%rdi, %r9
	xorl	%eax, %eax
	shrq	$3, %rdx
	cmpq	%r10, %r9
	seta	%al
	addq	%rdx, %rax
	cmpq	%rdi, %r12
	jae	.LBB1924_117
	testq	%r12, %r12
	je	.LBB1924_118
	incq	%rdi
	movq	$-1, %rdx
	cmoveq	%rdx, %rdi
	jmp	.LBB1924_118
.LBB1924_112:
	movq	8(%rsp), %rax
	addq	$2, %rsi
	movq	$-1, %rbp
	movq	%rdx, %r15
	cmovaeq	%rsi, %rbp
.LBB1924_113:
	movl	$9, %ecx
	mulq	%rcx
	jo	.LBB1924_124
	cmpl	$0, 4(%rsp)
	jns	.LBB1924_116
	incq	%rbp
	movq	$-1, %rcx
	cmoveq	%rcx, %rbp
.LBB1924_116:
	addq	%rbp, %rax
	movq	$-1, %rdx
	cmovbq	%rdx, %rax
	movq	%rax, %rcx
	incq	%rcx
	cmoveq	%rdx, %rcx
	jmp	.LBB1924_120
.LBB1924_117:
	addq	$2, %r12
	cmovbq	%r8, %r12
	movq	%r12, %rdi
.LBB1924_118:
	movl	$9, %edx
	mulq	%rdx
	jo	.LBB1924_124
	incq	%rdi
	movq	$-1, %rdx
	cmoveq	%rdx, %rdi
	addq	%rdi, %rax
	cmovbq	%rdx, %rax
	movq	%rax, %rdi
	incq	%rdi
	cmoveq	%rdx, %rdi
	addq	%rdi, %rcx
	cmovbq	%rdx, %rcx
	cmpq	%rax, %rsi
	cmovaq	%rsi, %rax
.LBB1924_120:
	addq	%rcx, %rbx
	movq	$-1, %rdx
	cmovbq	%rdx, %rbx
	cmpq	%rax, %r15
	movq	%r15, %rdx
	cmovbeq	%rax, %rdx
	jmp	.LBB1924_102
.LBB1924_121:
	movl	$1, %ebx
.LBB1924_122:
	movq	56(%rsp), %r13
	jmp	.LBB1924_88
.LBB1924_123:
	movq	24(%rsp), %rdx
	movq	16(%rsp), %rsi
	movq	32(%rsp), %rbp
	cmpq	$18, %rsi
	jbe	.LBB1924_47
	jmp	.LBB1924_49
.LBB1924_124:
	movq	$-1, %rcx
	movq	$-1, %rax
	jmp	.LBB1924_120
.LBB1924_125:
.Ltmp39403:
	movq	core::option::expect_failed@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.551(%rip), %rdi
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.552(%rip), %rdx
	movl	$22, %esi
	callq	*%rax
.Ltmp39404:
	ud2
.LBB1924_127:
	movq	$-1, %rax
	jmp	.LBB1924_108
.LBB1924_128:
	movq	core::num::imp::int_log10::panic_for_nonpositive_argument@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.1111(%rip), %rdi
	callq	*%rax
.LBB1924_129:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$128, %esi
	callq	*%rax
.LBB1924_130:
.Ltmp39397:
	jmp	.LBB1924_132
.LBB1924_131:
.Ltmp39394:
.LBB1924_132:
	movq	56(%rsp), %rsi
	movq	%rax, %rbx
	testq	%rsi, %rsi
	je	.LBB1924_137
	movq	64(%rsp), %rdi
	shlq	$5, %rsi
	movl	$8, %edx
	jmp	.LBB1924_136
.LBB1924_134:
.Ltmp39400:
	movq	%rax, %rbx
	testq	%r13, %r13
	je	.LBB1924_137
	shlq	$5, %r13
	movl	$8, %edx
	movq	%r12, %rdi
	movq	%r13, %rsi
.LBB1924_136:
	callq	__rustc::__rust_dealloc
.LBB1924_137:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB1924_138:
.Ltmp39405:
	leaq	160(%rsp), %rdi
	movq	%rax, %rbx
	callq	core::ptr::drop_glue::<purrdf_xsd::value::XsdValue>
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end1924:
