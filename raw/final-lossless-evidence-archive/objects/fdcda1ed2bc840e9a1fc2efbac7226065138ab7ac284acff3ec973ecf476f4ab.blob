purrdf_sparql_eval::engine::checked_query_read::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::governed::GovernedOutcome, <purrdf_sparql_eval::engine::NativeSparqlEngine>::query_governed_prepared_in_state<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>:
.Lfunc_begin888:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception888
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
	subq	$4096, %rsp
	.cfi_adjust_cfa_offset 4096
	movq	$0, (%rsp)
	subq	$4096, %rsp
	.cfi_adjust_cfa_offset 4096
	movq	$0, (%rsp)
	subq	$2200, %rsp
	.cfi_def_cfa_offset 10448
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	16(%rdx), %r8
	movq	%rsi, %rcx
	movq	24(%rdx), %rsi
	movq	%rdi, 440(%rsp)
	movq	8(%rdx), %rax
	movq	(%rdx), %rdi
	movq	48(%rdx), %rbx
	movq	56(%rdx), %r14
	movq	%rcx, 136(%rsp)
	vmovups	40(%r8), %zmm0
	movzbl	(%r8), %ebp
	movq	%rsi, 112(%rsp)
	movq	32(%rdx), %rsi
	movq	%rax, 48(%rsp)
	movq	%rsi, 264(%rsp)
	movq	40(%rdx), %rsi
	vmovups	%zmm0, 679(%rsp)
	vmovups	1(%r8), %zmm0
	movq	%rsi, 432(%rsp)
	vmovups	%zmm0, 640(%rsp)
	vmovups	120(%r8), %zmm0
	movq	104(%r8), %r15
	movq	112(%r8), %rsi
	vmovups	%zmm0, 5536(%rsp)
	testq	%rsi, %rsi
	je	.LBB1578_2
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::user_fn::RefusalPublication>::admission@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	136(%rsp), %rcx
	jmp	.LBB1578_3
.LBB1578_2:
	xorl	%eax, %eax
.LBB1578_3:
	vmovups	679(%rsp), %zmm1
	vmovups	640(%rsp), %zmm0
	vmovups	%zmm1, 2183(%rsp)
	vmovups	5536(%rsp), %zmm1
	vmovups	%zmm0, 2144(%rsp)
	vmovups	%zmm1, 7984(%rsp)
	cmpb	$2, %bpl
	jne	.LBB1578_5
	vmovups	2183(%rsp), %zmm1
	vmovups	2151(%rsp), %zmm0
	vmovups	%zmm1, 3655(%rsp)
	vmovups	%zmm0, 3623(%rsp)
	vmovdqu64	3623(%rsp), %zmm0
	vmovdqu64	3655(%rsp), %zmm1
	vmovdqu64	%zmm0, 648(%rsp)
	vmovdqu64	%zmm1, 680(%rsp)
	movq	$-2, 640(%rsp)
	jmp	.LBB1578_11
.LBB1578_5:
	vmovups	2183(%rsp), %zmm1
	vmovups	2144(%rsp), %zmm0
	vmovups	%zmm1, 3655(%rsp)
	vmovups	7984(%rsp), %zmm1
	vmovups	%zmm0, 3616(%rsp)
	vmovups	3616(%rsp), %zmm3
	vmovups	%zmm1, 3552(%rsp)
	vmovups	3655(%rsp), %zmm1
	movb	%bpl, 3432(%rsp)
	vmovups	%zmm1, 3472(%rsp)
	vmovups	%zmm3, 3433(%rsp)
	movq	%r15, 3536(%rsp)
	movq	%rax, 3544(%rsp)
	movq	264(%rsp), %rax
	vmovups	3496(%rsp), %zmm0
	vmovdqu64	3432(%rsp), %zmm2
	vmovdqu64	3552(%rsp), %zmm1
	movzbl	(%r14), %ebp
	movq	(%rbx), %r14
	leaq	16(%rax), %rsi
	vmovups	%zmm0, 704(%rsp)
	vmovdqu64	%zmm2, 640(%rsp)
	vmovdqu64	%zmm1, 760(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	112(%rsp), %rbx
	movq	<purrdf_sparql_eval::engine::RequestParameters>::check@GOTPCREL(%rip), %rax
	leaq	2144(%rsp), %rdi
	leaq	640(%rsp), %rcx
	movq	%rbx, %rdx
	vzeroupper
	callq	*%rax
	cmpq	$-1, 2144(%rsp)
	je	.LBB1578_7
	vmovdqu64	2176(%rsp), %zmm1
	vmovdqu64	2144(%rsp), %zmm0
	vmovdqu64	%zmm1, 680(%rsp)
	vmovdqu64	%zmm0, 648(%rsp)
	jmp	.LBB1578_9
.LBB1578_7:
	movl	$192, %r15d
	addq	3520(%rsp), %r15
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::engine::relation_identity@GOTPCREL(%rip), %rax
	leaq	2144(%rsp), %rdi
	movq	%rbx, %rsi
	movq	%r15, %rdx
	callq	*%rax
	vmovups	2152(%rsp), %zmm0
	movq	2216(%rsp), %rcx
	movq	2144(%rsp), %rax
	movq	%rcx, 3680(%rsp)
	vmovups	%zmm0, 3616(%rsp)
	cmpq	$-1, %rax
	je	.LBB1578_30
	vmovdqu	2224(%rsp), %xmm0
	vmovdqu64	3616(%rsp), %zmm1
	movq	3680(%rsp), %rcx
	vmovdqu	%xmm0, 728(%rsp)
	vmovdqu64	%zmm1, 656(%rsp)
	movq	%rcx, 720(%rsp)
	movq	%rax, 648(%rsp)
.LBB1578_9:
	movq	$-2, 640(%rsp)
.LBB1578_10:
	movq	136(%rsp), %rcx
.LBB1578_11:
	vmovups	704(%rsp), %zmm1
	vmovups	768(%rsp), %zmm2
	vmovdqu64	640(%rsp), %zmm0
	vmovups	832(%rsp), %zmm3
	vmovups	%zmm1, 5232(%rsp)
	vmovups	%zmm2, 5296(%rsp)
	vmovdqu64	896(%rsp), %zmm2
	vmovdqu64	944(%rsp), %zmm1
	vmovdqu64	%zmm0, 5168(%rsp)
	vmovups	%zmm3, 5360(%rsp)
	vmovdqu64	%zmm2, 5424(%rsp)
	vmovdqu64	%zmm1, 5472(%rsp)
	movl	24(%rcx), %eax
	testl	%eax, %eax
	je	.LBB1578_13
.LBB1578_12:
	vmovups	5472(%rsp), %zmm0
	vmovups	5424(%rsp), %zmm1
	movq	440(%rsp), %rax
	vmovups	5168(%rsp), %zmm4
	vmovups	5296(%rsp), %zmm2
	vmovups	5360(%rsp), %zmm3
	vmovups	%zmm0, 304(%rax)
	vmovups	%zmm1, 256(%rax)
	vmovups	5232(%rsp), %zmm1
	vmovups	%zmm3, 192(%rax)
	vmovups	%zmm2, 128(%rax)
	vmovups	%zmm4, (%rax)
	vmovups	%zmm1, 64(%rax)
	addq	$10392, %rsp
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
.LBB1578_13:
	.cfi_def_cfa_offset 10448
	movq	16(%rcx), %r14
	movl	$1, %ecx
	xorl	%eax, %eax
	lock		cmpxchgl	%ecx, 32(%r14)
	leaq	32(%r14), %rbx
	jne	.LBB1578_434
.LBB1578_14:
	movq	std::panicking::panic_count::GLOBAL_PANIC_COUNT@GOTPCREL(%rip), %r15
	movq	(%r15), %rax
	shlq	%rax
	testq	%rax, %rax
	jne	.LBB1578_435
	movzbl	36(%r14), %eax
	vmovdqu	40(%r14), %ymm0
	leaq	36(%r14), %r12
	vmovdqu	%ymm0, 2144(%rsp)
	movq	$0, 40(%r14)
	movq	(%r15), %rax
	shlq	%rax
	testq	%rax, %rax
	jne	.LBB1578_438
.LBB1578_16:
	xorl	%eax, %eax
	xchgl	%eax, (%rbx)
	cmpl	$2, %eax
	je	.LBB1578_441
.LBB1578_17:
	cmpq	$-2, 5168(%rsp)
	movq	2144(%rsp), %rax
	setne	%cl
	testq	%rax, %rax
	sete	%dl
	orb	%cl, %dl
	cmpb	$1, %dl
	jne	.LBB1578_24
	testq	%rax, %rax
	je	.LBB1578_12
	lock		decq	(%rax)
	jne	.LBB1578_21
	#MEMBARRIER
.Ltmp18244:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn core::error::Error + core::marker::Sync + core::marker::Send>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	2144(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp18245:
.LBB1578_21:
	movq	2160(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1578_12
	lock		decq	(%rax)
	jne	.LBB1578_12
	leaq	2160(%rsp), %rdi
	#MEMBARRIER
.Ltmp18250:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn purrdf_sparql_eval::user_fn::UserFunctionAdmission>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp18251:
	jmp	.LBB1578_12
.LBB1578_24:
	vmovdqu	2144(%rsp), %ymm0
	movq	136(%rsp), %rax
	vmovdqu	%ymm0, 640(%rsp)
	movq	16(%rax), %rax
	movq	16(%rax), %rcx
	movq	24(%rax), %rax
	movq	16(%rax), %rdx
	movq	32(%rax), %rax
	decq	%rdx
	andq	$-16, %rdx
	leaq	16(%rcx,%rdx), %rdi
.Ltmp18230:
	.cfi_escape 0x2e, 0x00
	leaq	640(%rsp), %rsi
	vzeroupper
	callq	*%rax
.Ltmp18231:
	movq	640(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB1578_27
	#MEMBARRIER
.Ltmp18235:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn core::error::Error + core::marker::Sync + core::marker::Send>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	640(%rsp), %rdi
	callq	*%rax
.Ltmp18236:
.LBB1578_27:
	movq	656(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1578_12
	lock		decq	(%rax)
	jne	.LBB1578_12
	leaq	656(%rsp), %rdi
	#MEMBARRIER
.Ltmp18241:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn purrdf_sparql_eval::user_fn::UserFunctionAdmission>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp18242:
	jmp	.LBB1578_12
.LBB1578_30:
	vmovdqu64	3616(%rsp), %zmm0
	movq	3680(%rsp), %rax
	movq	%r15, 104(%rsp)
	movb	%bpl, 47(%rsp)
	movq	%r14, 96(%rsp)
	movq	%rax, 512(%rsp)
	leaq	16(%r14), %rax
	movq	%rax, 424(%rsp)
	vmovdqu64	%zmm0, 448(%rsp)
	movq	32(%r14), %rax
	cmpq	$-1, %rax
	je	.LBB1578_388
	movq	%rax, 416(%rsp)
	movl	$3, 608(%rsp)
.Ltmp17856:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_algebra::algebra::Query>::dataset@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.Ltmp17857:
.Ltmp17858:
	.cfi_escape 0x2e, 0x00
	movq	48(%rsp), %rdx
	leaq	144(%rsp), %rdi
	movq	%rax, %rsi
	callq	<purrdf_sparql_eval::dataset_spec::ActiveDataset>::from_query_dataset::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp17859:
	movq	112(%rsp), %rax
	leaq	24(%rax), %rbx
.Ltmp17860:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::plan::Tree>::build@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp17861:
	movq	%rax, 120(%rsp)
	movq	%rdx, 128(%rsp)
.Ltmp17862:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::bgp::PlanSurvey>::for_shape@GOTPCREL(%rip), %rax
	leaq	200(%rsp), %rdi
	leaq	120(%rsp), %rsi
	callq	*%rax
.Ltmp17863:
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_no_alloc_shim_is_unstable_v2@GOTPCREL(%rip), %rax
	callq	*%rax
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_alloc@GOTPCREL(%rip), %rax
	movl	$40, %edi
	movl	$8, %esi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB1578_443
	vmovdqu	.Lanon.4895d20928f7255621bf668e37519d0a.0(%rip), %ymm0
	movzbl	144(%rsp), %edi
	movq	152(%rsp), %rsi
	movq	160(%rsp), %rdx
	movabsq	$9223372036854775793, %rcx
	movl	$1, %r15d
	movl	$3, (%rax)
	movl	$1, 8(%rax)
	movq	$1, 8(%rsp)
	movq	%rbx, 16(%rax)
	movq	%rax, 16(%rsp)
	addq	$15, %rcx
	movq	%rcx, 304(%rsp)
	movb	%dil, 46(%rsp)
	movq	%rsi, 256(%rsp)
	movq	%rdx, 408(%rsp)
	vmovdqu	%ymm0, 64(%rsp)
	jmp	.LBB1578_42
.LBB1578_37:
	movq	%rbx, %rax
	shlq	$4, %rax
	addq	%rax, %rbx
	addq	$33, %rbx
	je	.LBB1578_40
	movq	2144(%rsp), %rdi
.LBB1578_39:
	subq	%rax, %rdi
	addq	$-16, %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$16, %edx
	movq	%rbx, %rsi
	callq	*%rax
.LBB1578_40:
	movq	24(%rsp), %r15
.LBB1578_41:
	testq	%r15, %r15
	je	.LBB1578_271
.LBB1578_42:
	decq	%r15
	movl	$2, %esi
	movq	%r15, 24(%rsp)
	leaq	(%r15,%r15,4), %rcx
	movq	16(%rsp), %rax
	movq	8(%rax,%rcx,8), %rdx
	movl	(%rax,%rcx,8), %r14d
	movq	16(%rax,%rcx,8), %r12
	movq	%rdx, 32(%rsp)
	movl	%r14d, %edx
	subl	$3, %edx
	cmovbl	%esi, %edx
	testl	%edx, %edx
	je	.LBB1578_58
	movq	24(%rax,%rcx,8), %rbp
	cmpl	$1, %edx
	jne	.LBB1578_68
	vmovdqu	.Lanon.4895d20928f7255621bf668e37519d0a.0(%rip), %ymm0
	vmovdqu	%ymm0, 2144(%rsp)
.Ltmp17947:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::property_fn_plan::collect_certainly_bound@GOTPCREL(%rip), %rax
	leaq	2144(%rsp), %rsi
	movq	%rbp, %rdi
	vzeroupper
	callq	*%rax
.Ltmp17948:
	leaq	640(%rsp), %rbx
.Ltmp17949:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::bgp::predicted_rows@GOTPCREL(%rip), %rax
	leaq	200(%rsp), %rsi
	movq	%rbp, %rdi
	callq	*%rax
.Ltmp17950:
	testb	$1, %al
	movl	$1, %eax
	movq	%rdx, %r8
	cmoveq	%rax, %r8
.Ltmp17951:
	.cfi_escape 0x2e, 0x10
	subq	$8, %rsp
	.cfi_adjust_cfa_offset 8
	movq	40(%rsp), %rsi
	movq	112(%rsp), %r9
	leaq	208(%rsp), %rax
	movq	%r12, %rdx
	movq	%rbx, %rdi
	leaq	2152(%rsp), %rcx
	pushq	%rax
	.cfi_adjust_cfa_offset 8
	movq	purrdf_sparql_eval::bgp::record_call_estimate@GOTPCREL(%rip), %rax
	callq	*%rax
	addq	$16, %rsp
	.cfi_adjust_cfa_offset -16
.Ltmp17952:
	movq	640(%rsp), %r14
	cmpq	$-1, %r14
	jne	.LBB1578_282
	movq	2152(%rsp), %rbx
	testq	%rbx, %rbx
	je	.LBB1578_40
	movq	2168(%rsp), %r14
	testq	%r14, %r14
	je	.LBB1578_37
	movq	2144(%rsp), %r15
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	vpcmpltb	(%r15), %xmm0, %k0
	leaq	16(%r15), %r12
	kmovd	%k0, %ebp
	jmp	.LBB1578_52
	.p2align	4
.LBB1578_51:
	blsrl	%ebp, %ebp
	decq	%r14
	je	.LBB1578_37
.LBB1578_52:
	testw	%bp, %bp
	jne	.LBB1578_56
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	.p2align	4
.LBB1578_54:
	vpcmpltb	(%r12), %xmm0, %k0
	addq	$-256, %r15
	addq	$16, %r12
	kortestw	%k0, %k0
	je	.LBB1578_54
	kmovd	%k0, %ebp
.LBB1578_56:
	xorl	%eax, %eax
	tzcntl	%ebp, %eax
	movq	%r15, %rdi
	shll	$4, %eax
	subq	%rax, %rdi
	movq	-16(%rdi), %rax
	lock		decq	(%rax)
	jne	.LBB1578_51
	addq	$-16, %rdi
	#MEMBARRIER
.Ltmp17959:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp17960:
	jmp	.LBB1578_51
	.p2align	4
.LBB1578_58:
	movq	248(%rsp), %rdi
	addq	$16, %rdi
.Ltmp17962:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::plan::PlanShape>::node_of@GOTPCREL(%rip), %rax
	movq	%r12, %rsi
	vzeroupper
	callq	*%rax
.Ltmp17963:
	movq	32(%rsp), %rbx
	shrq	$32, %rbx
	testb	$1, %al
	je	.LBB1578_78
	movq	248(%rsp), %rdi
	addq	$16, %rdi
.Ltmp17964:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::plan::PlanShape>::positive_region@GOTPCREL(%rip), %rax
	movl	%edx, %esi
	callq	*%rax
.Ltmp17965:
	testb	%al, %al
	je	.LBB1578_78
.Ltmp17966:
	.cfi_escape 0x2e, 0x10
	subq	$8, %rsp
	.cfi_adjust_cfa_offset 8
	movq	56(%rsp), %rsi
	leaq	152(%rsp), %rdx
	movq	40(%rsp), %rcx
	leaq	648(%rsp), %rdi
	movl	%ebx, %r8d
	movq	%r12, %r9
	pushq	$0
	.cfi_adjust_cfa_offset 8
	callq	<purrdf_sparql_eval::bgp::positive::PositivePlan>::build_with_seed::<purrdf_core::ir::dataset::RdfDataset>
	addq	$16, %rsp
	.cfi_adjust_cfa_offset -16
.Ltmp17967:
	cmpl	$1, 640(%rsp)
	je	.LBB1578_295
	leaq	648(%rsp), %rcx
	vmovdqu	648(%rsp), %xmm0
	movq	648(%rsp), %rax
	vmovdqu64	16(%rcx), %zmm1
	vmovups	40(%rcx), %zmm2
	vmovups	104(%rcx), %zmm3
	vmovdqu64	%zmm1, 7984(%rsp)
	vmovups	%zmm2, 8008(%rsp)
	vmovdqu64	120(%rcx), %zmm2
	vmovups	%zmm3, 1888(%rsp)
	vmovdqu64	%zmm2, 1904(%rsp)
	cmpq	$-1, %rax
	je	.LBB1578_78
	vmovdqa	%xmm0, 640(%rsp)
	vmovups	7984(%rsp), %zmm0
	vmovdqu64	8008(%rsp), %zmm1
	leaq	648(%rsp), %rax
	vmovdqu64	1904(%rsp), %zmm2
	vmovdqu64	%zmm1, 32(%rax)
	vmovups	%zmm0, 8(%rax)
	vmovdqu64	1888(%rsp), %zmm0
	vmovdqu64	%zmm2, 112(%rax)
	vmovdqu64	%zmm0, 96(%rax)
.Ltmp17968:
	.cfi_escape 0x2e, 0x20
	subq	$8, %rsp
	.cfi_adjust_cfa_offset 8
	movq	56(%rsp), %rdx
	leaq	208(%rsp), %rax
	movq	40(%rsp), %r8
	leaq	2152(%rsp), %rdi
	leaq	648(%rsp), %rsi
	leaq	152(%rsp), %rcx
	movl	%ebx, %r9d
	pushq	%rax
	.cfi_adjust_cfa_offset 8
	pushq	$0
	.cfi_adjust_cfa_offset 8
	pushq	%r12
	.cfi_adjust_cfa_offset 8
	vzeroupper
	callq	<purrdf_sparql_eval::bgp::positive::PositivePlan>::survey_seeded::<purrdf_core::ir::dataset::RdfDataset>
	addq	$32, %rsp
	.cfi_adjust_cfa_offset -32
.Ltmp17969:
	movq	2144(%rsp), %r14
	cmpq	$-1, %r14
	jne	.LBB1578_297
.Ltmp17975:
	.cfi_escape 0x2e, 0x00
	leaq	640(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::bgp::positive::PositivePlan>
.Ltmp17976:
	jmp	.LBB1578_40
	.p2align	4
.LBB1578_68:
	leaq	(%rax,%rcx,8), %rdx
	movl	4(%rax,%rcx,8), %ebx
	movzbl	32(%rdx), %r13d
.Ltmp17864:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::bgp::positive::PositivePlan>::pure_eligible@GOTPCREL(%rip), %rax
	movq	%rbp, %rdi
	vzeroupper
	callq	*%rax
.Ltmp17865:
	testb	%al, %al
	je	.LBB1578_80
.Ltmp17870:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::bgp::predicted_rows@GOTPCREL(%rip), %rax
	leaq	200(%rsp), %rsi
	movq	%r12, %rdi
	callq	*%rax
.Ltmp17871:
	vpbroadcastq	.LCPI1578_5(%rip), %xmm0
	testb	$1, %al
	movl	$1, %eax
	movabsq	$2746377873070565055, %rcx
	movq	%rdx, %r15
	cmoveq	%rax, %r15
	movq	%r12, %rax
	xorq	%rcx, %rax
	movq	72(%rsp), %rcx
	vpinsrq	$0, %rax, %xmm0, %xmm0
	vaesenc	.LCPI1578_1(%rip), %xmm0, %xmm0
	movq	64(%rsp), %rax
	vaesenc	.LCPI1578_2(%rip), %xmm0, %xmm0
	vaesenc	.LCPI1578_3(%rip), %xmm0, %xmm0
	vmovq	%xmm0, %rdx
	movq	%rdx, %rsi
	shrq	$57, %rsi
	vpbroadcastb	%esi, %xmm0
	xorl	%esi, %esi
.LBB1578_72:
	andq	%rcx, %rdx
	vmovdqu	(%rax,%rdx), %xmm1
	vpcmpeqb	%xmm0, %xmm1, %k0
	kortestw	%k0, %k0
	je	.LBB1578_76
	kmovd	%k0, %r8d
.LBB1578_74:
	xorl	%r9d, %r9d
	tzcntl	%r8d, %r9d
	addq	%rdx, %r9
	andq	%rcx, %r9
	movq	%r9, %rdi
	negq	%rdi
	imulq	$104, %rdi, %rdi
	cmpq	%r12, -104(%rax,%rdi)
	je	.LBB1578_94
	leal	-1(%r8), %edi
	andw	%r8w, %di
	movl	%edi, %r8d
	jne	.LBB1578_74
.LBB1578_76:
	vpcmpeqd	%xmm2, %xmm2, %xmm2
	vpcmpeqb	%xmm2, %xmm1, %k0
	kortestw	%k0, %k0
	jne	.LBB1578_98
	leaq	16(%rdx,%rsi), %rdx
	addq	$16, %rsi
	jmp	.LBB1578_72
	.p2align	4
.LBB1578_78:
	movl	(%r12), %eax
	movl	$1, %ecx
	leaq	.LJTI1578_0(%rip), %rdx
	subl	$10, %eax
	cmovbl	%ecx, %eax
	movslq	(%rdx,%rax,4), %rax
	addq	%rdx, %rax
	jmpq	*%rax
.LBB1578_79:
	movl	$88, %eax
	jmp	.LBB1578_214
.LBB1578_80:
	vpbroadcastq	.LCPI1578_5(%rip), %xmm0
	movabsq	$2746377873070565055, %rcx
	movq	%r12, %rax
	xorq	%rcx, %rax
	movq	72(%rsp), %rcx
	vpinsrq	$0, %rax, %xmm0, %xmm0
	vaesenc	.LCPI1578_1(%rip), %xmm0, %xmm0
	movq	64(%rsp), %rax
	vaesenc	.LCPI1578_2(%rip), %xmm0, %xmm0
	vaesenc	.LCPI1578_3(%rip), %xmm0, %xmm0
	vmovq	%xmm0, %rdx
	movq	%rdx, %rsi
	shrq	$57, %rsi
	vpbroadcastb	%esi, %xmm0
	xorl	%esi, %esi
.LBB1578_81:
	andq	%rcx, %rdx
	vmovdqu	(%rax,%rdx), %xmm1
	vpcmpeqb	%xmm0, %xmm1, %k0
	kortestw	%k0, %k0
	je	.LBB1578_85
	kmovd	%k0, %r8d
.LBB1578_83:
	xorl	%r9d, %r9d
	tzcntl	%r8d, %r9d
	addq	%rdx, %r9
	andq	%rcx, %r9
	movq	%r9, %rdi
	negq	%rdi
	imulq	$104, %rdi, %rdi
	cmpq	%r12, -104(%rax,%rdi)
	je	.LBB1578_87
	leal	-1(%r8), %edi
	andw	%r8w, %di
	movl	%edi, %r8d
	jne	.LBB1578_83
.LBB1578_85:
	vpcmpeqd	%xmm2, %xmm2, %xmm2
	vpcmpeqb	%xmm2, %xmm1, %k0
	kortestw	%k0, %k0
	jne	.LBB1578_91
	leaq	16(%rdx,%rsi), %rdx
	addq	$16, %rsi
	jmp	.LBB1578_81
.LBB1578_87:
	imulq	$104, %r9, %rdx
	movabsq	$5675921253449092805, %rsi
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	sarq	$3, %rdx
	imulq	%rsi, %rdx
	leaq	-16(%rdx), %rsi
	andq	%rcx, %rsi
	vpcmpeqb	(%rax,%rsi), %xmm0, %k0
	kmovd	%k0, %ecx
	vpcmpeqb	(%rax,%rdx), %xmm0, %k0
	lzcntw	%cx, %cx
	kmovd	%k0, %r8d
	orl	$65536, %r8d
	tzcntl	%r8d, %r8d
	addl	%ecx, %r8d
	movb	$-128, %cl
	cmpw	$15, %r8w
	ja	.LBB1578_89
	incq	80(%rsp)
	movb	$-1, %cl
.LBB1578_89:
	movb	%cl, (%rax,%rdx)
	movb	%cl, 16(%rax,%rsi)
	addq	%rax, %rdi
	decq	88(%rsp)
	movq	-96(%rdi), %rax
	cmpq	$-1, %rax
	je	.LBB1578_91
	vmovdqu64	-88(%rdi), %zmm0
	vmovdqu64	-64(%rdi), %zmm1
	leaq	648(%rsp), %rcx
	vmovdqu64	%zmm1, 24(%rcx)
	vmovdqu64	%zmm0, (%rcx)
	movq	%rax, 640(%rsp)
.Ltmp17866:
	.cfi_escape 0x2e, 0x00
	leaq	640(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::bgp::SeedEstimate>
.Ltmp17867:
.LBB1578_91:
	movq	24(%rsp), %r15
	cmpq	8(%rsp), %r15
	jne	.LBB1578_93
.Ltmp17868:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::bgp::survey_pattern_plans::Step<purrdf_core::ir::term::TermId>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	8(%rsp), %rdi
	callq	*%rax
.Ltmp17869:
.LBB1578_93:
	movq	16(%rsp), %rax
	leaq	(%r15,%r15,4), %rcx
	movl	$3, (%rax,%rcx,8)
	movl	%r14d, 8(%rax,%rcx,8)
	movl	%ebx, 12(%rax,%rcx,8)
	movq	%rbp, 16(%rax,%rcx,8)
	jmp	.LBB1578_219
.LBB1578_94:
	imulq	$104, %r9, %rdx
	movabsq	$5675921253449092805, %rsi
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	sarq	$3, %rdx
	imulq	%rsi, %rdx
	leaq	-16(%rdx), %rsi
	andq	%rcx, %rsi
	vpcmpeqb	(%rax,%rsi), %xmm0, %k0
	kmovd	%k0, %ecx
	vpcmpeqb	(%rax,%rdx), %xmm0, %k0
	lzcntw	%cx, %cx
	kmovd	%k0, %r8d
	orl	$65536, %r8d
	tzcntl	%r8d, %r8d
	addl	%ecx, %r8d
	movb	$-128, %cl
	cmpw	$15, %r8w
	ja	.LBB1578_96
	incq	80(%rsp)
	movb	$-1, %cl
.LBB1578_96:
	movb	%cl, (%rax,%rdx)
	movb	%cl, 16(%rax,%rsi)
	addq	%rax, %rdi
	decq	88(%rsp)
	movq	-96(%rdi), %rax
	cmpq	$-1, %rax
	je	.LBB1578_98
	vmovups	-88(%rdi), %zmm0
	vmovups	-64(%rdi), %zmm1
	movq	%rax, 5536(%rsp)
	leaq	5544(%rsp), %rax
	vmovups	%zmm1, 664(%rsp)
	vmovups	%zmm0, 640(%rsp)
	vmovdqu64	656(%rsp), %zmm1
	vmovdqu64	640(%rsp), %zmm0
	vmovdqu64	%zmm1, 16(%rax)
	vmovdqu64	%zmm0, (%rax)
	jmp	.LBB1578_113
.LBB1578_98:
	vmovdqu	.Lanon.4895d20928f7255621bf668e37519d0a.0(%rip), %ymm0
	vmovdqu	%ymm0, 3616(%rsp)
.Ltmp17872:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::property_fn_plan::collect_certainly_bound@GOTPCREL(%rip), %rax
	leaq	3616(%rsp), %rsi
	movq	%r12, %rdi
	vzeroupper
	callq	*%rax
.Ltmp17873:
.Ltmp17874:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	callq	*%rax
.Ltmp17875:
	movq	%rax, %r12
	movl	$1, %eax
	xorl	%ecx, %ecx
	lock		cmpxchgq	%rcx, (%r12)
	jne	.LBB1578_109
	#MEMBARRIER
	movq	64(%r12), %rax
	movq	16(%r12), %rdx
	movq	24(%r12), %rcx
	movq	%rax, 2176(%rsp)
	vmovdqu	32(%r12), %ymm0
	vmovdqu	%ymm0, 2144(%rsp)
	cmpq	$-1, %r12
	je	.LBB1578_104
	lock		decq	8(%r12)
	jne	.LBB1578_104
	#MEMBARRIER
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movq	%rdx, 400(%rsp)
	movl	$72, %esi
	movl	$8, %edx
	movq	%r12, %rdi
	movq	%rcx, %r12
	vzeroupper
	callq	*%rax
	movq	400(%rsp), %rdx
	movq	%r12, %rcx
.LBB1578_104:
	cmpq	$-1, %rdx
	je	.LBB1578_108
	vmovdqu	2144(%rsp), %ymm0
	movq	2176(%rsp), %rax
	movq	%rdx, 640(%rsp)
	movq	%rcx, 648(%rsp)
	leaq	648(%rsp), %rcx
	movq	%rax, 40(%rcx)
	vmovdqu	%ymm0, 8(%rcx)
	jmp	.LBB1578_112
.LBB1578_106:
	movl	$32, %eax
	jmp	.LBB1578_214
.LBB1578_107:
	movl	$8, %eax
	jmp	.LBB1578_214
.LBB1578_108:
	movq	%rcx, %r12
.LBB1578_109:
	movq	%r12, 528(%rsp)
	leaq	16(%r12), %rsi
.Ltmp17876:
	.cfi_escape 0x2e, 0x00
	leaq	640(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone
.Ltmp17877:
	lock		decq	(%r12)
	jne	.LBB1578_112
	#MEMBARRIER
.Ltmp17882:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	528(%rsp), %rdi
	callq	*%rax
.Ltmp17883:
.LBB1578_112:
	vmovdqu	3616(%rsp), %ymm0
	vmovdqu	656(%rsp), %xmm2
	vmovdqu	664(%rsp), %ymm1
	leaq	5592(%rsp), %rax
	movq	648(%rsp), %rcx
	vmovdqu	%ymm0, (%rax)
	movq	640(%rsp), %rax
	vmovdqu	%xmm2, 5552(%rsp)
	movq	%rcx, 5544(%rsp)
	vmovdqu	%ymm1, 5560(%rsp)
	movq	%rax, 5536(%rsp)
.LBB1578_113:
	testb	$1, %r13b
	movl	$1, %ecx
	movq	%r15, %rax
	movb	$1, %r12b
	cmovneq	%rcx, %rax
	movq	%rax, 5624(%rsp)
.Ltmp17888:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::bgp::positive::PositivePlan>::seed_eligible@GOTPCREL(%rip), %rax
	leaq	5536(%rsp), %rsi
	movq	%rbp, %rdi
	vzeroupper
	callq	*%rax
.Ltmp17889:
	testb	%al, %al
	je	.LBB1578_127
.Ltmp17890:
	.cfi_escape 0x2e, 0x10
	subq	$8, %rsp
	.cfi_adjust_cfa_offset 8
	movq	56(%rsp), %rsi
	leaq	152(%rsp), %rdx
	leaq	5544(%rsp), %rax
	leaq	648(%rsp), %rdi
	movl	%r14d, %ecx
	movl	%ebx, %r8d
	movq	%rbp, %r9
	pushq	%rax
	.cfi_adjust_cfa_offset 8
	callq	<purrdf_sparql_eval::bgp::positive::PositivePlan>::build_with_seed::<purrdf_core::ir::dataset::RdfDataset>
	addq	$16, %rsp
	.cfi_adjust_cfa_offset -16
.Ltmp17891:
	cmpl	$1, 640(%rsp)
	je	.LBB1578_296
	leaq	648(%rsp), %rcx
	vmovdqu	648(%rsp), %xmm0
	movq	648(%rsp), %rax
	vmovdqu64	16(%rcx), %zmm1
	vmovups	40(%rcx), %zmm2
	vmovups	104(%rcx), %zmm3
	vmovdqu64	%zmm1, 9184(%rsp)
	vmovups	%zmm2, 9208(%rsp)
	vmovdqu64	120(%rcx), %zmm2
	vmovups	%zmm3, 1984(%rsp)
	vmovdqu64	%zmm2, 2000(%rsp)
	cmpq	$-1, %rax
	je	.LBB1578_127
	vmovdqa	%xmm0, 640(%rsp)
	vmovups	9184(%rsp), %zmm0
	vmovdqu64	9208(%rsp), %zmm1
	leaq	648(%rsp), %rax
	vmovdqu64	2000(%rsp), %zmm2
	vmovdqu64	%zmm1, 32(%rax)
	vmovups	%zmm0, 8(%rax)
	vmovdqu64	1984(%rsp), %zmm0
	vmovdqu64	%zmm2, 112(%rax)
	vmovdqu64	%zmm0, 96(%rax)
.Ltmp17892:
	.cfi_escape 0x2e, 0x20
	subq	$8, %rsp
	.cfi_adjust_cfa_offset 8
	movq	56(%rsp), %rdx
	leaq	208(%rsp), %rax
	leaq	2152(%rsp), %rdi
	leaq	648(%rsp), %rsi
	leaq	152(%rsp), %rcx
	movl	%r14d, %r8d
	movl	%ebx, %r9d
	pushq	%rax
	.cfi_adjust_cfa_offset 8
	leaq	5552(%rsp), %rax
	pushq	%rax
	.cfi_adjust_cfa_offset 8
	pushq	%rbp
	.cfi_adjust_cfa_offset 8
	vzeroupper
	callq	<purrdf_sparql_eval::bgp::positive::PositivePlan>::survey_seeded::<purrdf_core::ir::dataset::RdfDataset>
	addq	$32, %rsp
	.cfi_adjust_cfa_offset -32
.Ltmp17893:
	movq	32(%rsp), %r12
	movq	2144(%rsp), %r14
	cmpq	$-1, %r14
	jne	.LBB1578_298
.Ltmp17897:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::bgp::PlanSurvey>::estimate_of@GOTPCREL(%rip), %rax
	leaq	200(%rsp), %rdi
	movq	%rbp, %rsi
	callq	*%rax
.Ltmp17898:
	testq	%rax, %rax
	je	.LBB1578_444
	vmovdqu	(%rax), %xmm0
	vmovdqa	%xmm0, 272(%rsp)
.Ltmp17899:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::solution::VarSchema>::union@GOTPCREL(%rip), %rax
	leaq	528(%rsp), %rdi
	leaq	5536(%rsp), %rsi
	leaq	640(%rsp), %rdx
	callq	*%rax
.Ltmp17900:
	movq	544(%rsp), %rax
	movq	%rax, 288(%rsp)
	movq	272(%rsp), %rbx
	testb	$1, %r13b
	je	.LBB1578_142
	cmpq	$1, %rbx
	movq	%r15, %rax
	adcq	$0, %rbx
	mulq	%rbx
	jo	.LBB1578_267
	movq	%rax, %rbx
.LBB1578_126:
	movq	%rbx, 272(%rsp)
	movq	280(%rsp), %rax
	cmpq	%rbx, %rax
	cmovbeq	%rbx, %rax
	movq	%rax, 280(%rsp)
.Ltmp17906:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::bgp::PlanSurvey>::record@GOTPCREL(%rip), %rax
	leaq	200(%rsp), %rdi
	leaq	272(%rsp), %rdx
	movq	%r12, %rsi
	callq	*%rax
.Ltmp17907:
	jmp	.LBB1578_144
.LBB1578_127:
	movq	24(%rsp), %r15
	cmpq	8(%rsp), %r15
	jne	.LBB1578_129
.Ltmp17930:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::bgp::survey_pattern_plans::Step<purrdf_core::ir::term::TermId>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	8(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp17931:
.LBB1578_129:
	movq	16(%rsp), %rax
	leaq	(%r15,%r15,4), %rcx
	incq	%r15
	movl	$3, (%rax,%rcx,8)
	movl	%r14d, 8(%rax,%rcx,8)
	movl	%ebx, 12(%rax,%rcx,8)
	movq	%rbp, 16(%rax,%rcx,8)
	movq	%r15, 24(%rsp)
.Ltmp17937:
	.cfi_escape 0x2e, 0x00
	leaq	5536(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
.Ltmp17938:
	movq	5600(%rsp), %rbx
	testq	%rbx, %rbx
	je	.LBB1578_40
	movq	5616(%rsp), %r14
	testq	%r14, %r14
	je	.LBB1578_140
	movq	5592(%rsp), %r15
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	vpcmpltb	(%r15), %xmm0, %k0
	leaq	16(%r15), %r12
	kmovd	%k0, %ebp
	jmp	.LBB1578_134
	.p2align	4
.LBB1578_133:
	blsrl	%ebp, %ebp
	decq	%r14
	je	.LBB1578_140
.LBB1578_134:
	testw	%bp, %bp
	jne	.LBB1578_138
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	.p2align	4
.LBB1578_136:
	vpcmpltb	(%r12), %xmm0, %k0
	addq	$-256, %r15
	addq	$16, %r12
	kortestw	%k0, %k0
	je	.LBB1578_136
	kmovd	%k0, %ebp
.LBB1578_138:
	xorl	%eax, %eax
	tzcntl	%ebp, %eax
	movq	%r15, %rdi
	shll	$4, %eax
	subq	%rax, %rdi
	movq	-16(%rdi), %rax
	lock		decq	(%rax)
	jne	.LBB1578_133
	addq	$-16, %rdi
	#MEMBARRIER
.Ltmp17942:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp17943:
	jmp	.LBB1578_133
.LBB1578_140:
	movq	%rbx, %rax
	shlq	$4, %rax
	addq	%rax, %rbx
	addq	$33, %rbx
	je	.LBB1578_40
	movq	5592(%rsp), %rdi
	jmp	.LBB1578_39
.LBB1578_142:
.Ltmp17902:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::bgp::PlanSurvey>::record@GOTPCREL(%rip), %rax
	leaq	200(%rsp), %rdi
	leaq	272(%rsp), %rdx
	movq	%r12, %rsi
	callq	*%rax
.Ltmp17903:
.Ltmp17904:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::property_fn_plan::collect_certainly_bound@GOTPCREL(%rip), %rax
	leaq	5592(%rsp), %rsi
	movq	%rbp, %rdi
	callq	*%rax
.Ltmp17905:
.LBB1578_144:
	vmovups	552(%rsp), %ymm1
	leaq	5592(%rsp), %rax
	vmovdqu	528(%rsp), %ymm0
	cmpq	$0, 80(%rsp)
	vmovups	%ymm1, 2168(%rsp)
	vmovdqu	(%rax), %ymm1
	leaq	2200(%rsp), %rax
	vmovdqu	%ymm0, 2144(%rsp)
	vmovdqu	%ymm1, (%rax)
	movq	%rbx, 2232(%rsp)
	je	.LBB1578_266
.LBB1578_145:
	vpbroadcastq	.LCPI1578_5(%rip), %xmm0
	movabsq	$2746377873070565055, %rcx
	movq	%r12, %rax
	movq	72(%rsp), %rdx
	leaq	640(%rsp), %rbx
	xorl	%r9d, %r9d
	xorl	%r8d, %r8d
	xorq	%rcx, %rax
	vpinsrq	$0, %rax, %xmm0, %xmm0
	vaesenc	.LCPI1578_1(%rip), %xmm0, %xmm0
	movq	64(%rsp), %rax
	vaesenc	.LCPI1578_2(%rip), %xmm0, %xmm0
	vaesenc	.LCPI1578_3(%rip), %xmm0, %xmm0
	vmovq	%xmm0, %rdi
	movq	%rdi, %rcx
	shrq	$57, %rcx
	vpbroadcastb	%ecx, %xmm0
.LBB1578_146:
	andq	%rdx, %rdi
	vmovdqu	(%rax,%rdi), %xmm1
	vpcmpeqb	%xmm0, %xmm1, %k0
	kortestw	%k0, %k0
	je	.LBB1578_150
	kmovd	%k0, %r10d
.LBB1578_148:
	xorl	%r11d, %r11d
	tzcntl	%r10d, %r11d
	addq	%rdi, %r11
	andq	%rdx, %r11
	negq	%r11
	imulq	$104, %r11, %r11
	cmpq	%r12, -104(%rax,%r11)
	je	.LBB1578_157
	leal	-1(%r10), %r11d
	andw	%r10w, %r11w
	movl	%r11d, %r10d
	jne	.LBB1578_148
.LBB1578_150:
	cmpq	$1, %r9
	je	.LBB1578_153
	vpmovmskb	%xmm1, %esi
	testl	%esi, %esi
	je	.LBB1578_155
	tzcntl	%esi, %esi
	addq	%rdi, %rsi
	andq	%rdx, %rsi
.LBB1578_153:
	vpcmpeqd	%xmm2, %xmm2, %xmm2
	vpcmpeqb	%xmm2, %xmm1, %k0
	kortestw	%k0, %k0
	jne	.LBB1578_222
	movl	$1, %r9d
	jmp	.LBB1578_156
.LBB1578_155:
	xorl	%r9d, %r9d
.LBB1578_156:
	leaq	16(%r8,%rdi), %rdi
	addq	$16, %r8
	jmp	.LBB1578_146
.LBB1578_157:
	addq	%r11, %rax
	vmovdqu64	2176(%rsp), %zmm2
	vmovups	-96(%rax), %zmm0
	vmovdqu64	-64(%rax), %zmm1
	vmovdqu64	%zmm1, 3648(%rsp)
	vmovups	%zmm0, 3616(%rsp)
	vmovdqu64	2144(%rsp), %zmm0
	vmovdqu64	%zmm2, -64(%rax)
	vmovdqu64	%zmm0, -96(%rax)
	cmpq	$-1, 3616(%rsp)
	je	.LBB1578_224
.Ltmp17917:
	.cfi_escape 0x2e, 0x00
	leaq	3616(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::bgp::SeedEstimate>
.Ltmp17918:
	jmp	.LBB1578_224
.LBB1578_159:
	movq	16(%r12), %rbx
	testq	%rbx, %rbx
	je	.LBB1578_442
	movq	24(%rsp), %r15
	cmpq	8(%rsp), %r15
	jne	.LBB1578_162
.Ltmp17990:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::bgp::survey_pattern_plans::Step<purrdf_core::ir::term::TermId>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	8(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp17991:
.LBB1578_162:
	movq	16(%rsp), %rax
	movq	32(%rsp), %rdx
	leaq	(%r15,%r15,4), %rcx
	leaq	1(%r15), %r14
	movl	$3, (%rax,%rcx,8)
	movq	%rdx, 8(%rax,%rcx,8)
	movq	%rbx, 16(%rax,%rcx,8)
	movq	%r14, 24(%rsp)
	movq	8(%r12), %rbx
	testq	%rbx, %rbx
	je	.LBB1578_442
	cmpq	8(%rsp), %r14
	jne	.LBB1578_243
.Ltmp17992:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::bgp::survey_pattern_plans::Step<purrdf_core::ir::term::TermId>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	8(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp17993:
	jmp	.LBB1578_243
.LBB1578_165:
	movq	72(%r12), %rbx
	testq	%rbx, %rbx
	je	.LBB1578_442
	movq	80(%r12), %r14
	testq	%r14, %r14
	je	.LBB1578_442
	movq	24(%rsp), %r15
	cmpq	8(%rsp), %r15
	jne	.LBB1578_169
.Ltmp18025:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::bgp::survey_pattern_plans::Step<purrdf_core::ir::term::TermId>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	8(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp18026:
.LBB1578_169:
	movq	16(%rsp), %rax
	movq	32(%rsp), %rdx
	leaq	(%r15,%r15,4), %rcx
	movq	%rdx, (%rax,%rcx,8)
	movq	%r12, 8(%rax,%rcx,8)
	movq	%rbx, 16(%rax,%rcx,8)
	movq	%r14, 24(%rax,%rcx,8)
	leaq	1(%r15), %r14
	movb	$1, 32(%rax,%rcx,8)
	movq	%r14, 24(%rsp)
	movq	72(%r12), %rbx
	testq	%rbx, %rbx
	je	.LBB1578_442
	cmpq	8(%rsp), %r14
	jne	.LBB1578_243
.Ltmp18027:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::bgp::survey_pattern_plans::Step<purrdf_core::ir::term::TermId>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	8(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp18028:
	jmp	.LBB1578_243
.LBB1578_172:
	movq	24(%r12), %rax
	movq	48(%r12), %rcx
	movq	%rcx, 640(%rsp)
	movq	%rcx, 648(%rsp)
	movq	%rax, 656(%rsp)
.Ltmp17988:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::bgp::PlanSurvey>::record@GOTPCREL(%rip), %rax
	leaq	200(%rsp), %rdi
	leaq	640(%rsp), %rdx
	movq	%r12, %rsi
	vzeroupper
	callq	*%rax
.Ltmp17989:
	jmp	.LBB1578_40
.LBB1578_173:
	movq	24(%r12), %rax
	testq	%rax, %rax
	je	.LBB1578_40
	vmovdqu	.Lanon.4895d20928f7255621bf668e37519d0a.0(%rip), %ymm0
	movq	16(%r12), %r10
	leaq	648(%rsp), %rcx
	movq	$0, 640(%rsp)
	movq	$8, 648(%rsp)
	movq	$0, 656(%rsp)
	vmovdqu	%ymm0, 16(%rcx)
	vmovdqu	%ymm0, 48(%rcx)
	movq	$1, 728(%rsp)
.Ltmp18038:
	.cfi_escape 0x2e, 0x20
	movq	48(%rsp), %rsi
	leaq	144(%rsp), %rdx
	leaq	200(%rsp), %r11
	movq	32(%rsp), %rcx
	leaq	2144(%rsp), %rdi
	movl	%ebx, %r8d
	movq	%r12, %r9
	pushq	%r11
	.cfi_adjust_cfa_offset 8
	leaq	648(%rsp), %rbx
	pushq	%rbx
	.cfi_adjust_cfa_offset 8
	pushq	%rax
	.cfi_adjust_cfa_offset 8
	pushq	%r10
	.cfi_adjust_cfa_offset 8
	vzeroupper
	callq	purrdf_sparql_eval::bgp::survey_bgp_seeded::<purrdf_core::ir::dataset::RdfDataset>
	addq	$32, %rsp
	.cfi_adjust_cfa_offset -32
.Ltmp18039:
.Ltmp18044:
	.cfi_escape 0x2e, 0x00
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::bgp::SeedEstimate>
.Ltmp18045:
	movq	2144(%rsp), %r14
	cmpq	$-1, %r14
	je	.LBB1578_40
	jmp	.LBB1578_433
.LBB1578_177:
	movq	16(%r12), %rbx
	testq	%rbx, %rbx
	je	.LBB1578_442
	cmpl	$28, (%rbx)
	jne	.LBB1578_230
	movq	8(%r12), %r14
	testq	%r14, %r14
	je	.LBB1578_442
	movq	24(%rsp), %r15
	cmpq	8(%rsp), %r15
	jne	.LBB1578_182
.Ltmp18031:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::bgp::survey_pattern_plans::Step<purrdf_core::ir::term::TermId>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	8(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp18032:
.LBB1578_182:
	movq	16(%rsp), %rcx
	leaq	(%r15,%r15,4), %rax
	movl	$4, (%rcx,%rax,8)
	movq	%rbx, 8(%rcx,%rax,8)
	addq	$8, %rbx
	movq	%rbx, 16(%rcx,%rax,8)
	movq	%r14, 24(%rcx,%rax,8)
	jmp	.LBB1578_234
.LBB1578_183:
	movq	8(%rsp), %rax
	movq	24(%rsp), %r15
	movq	16(%r12), %r14
	movq	24(%r12), %rbx
	subq	%r15, %rax
	cmpq	%rax, %rbx
	ja	.LBB1578_268
	movq	32(%rsp), %r9
	testq	%rbx, %rbx
	je	.LBB1578_193
	movq	16(%rsp), %rcx
.LBB1578_186:
	shlq	$4, %rbx
	movabsq	$-2049638230412172401, %rsi
	leaq	(%rbx,%rbx,8), %rdx
	leaq	(%r14,%rdx), %rax
	addq	$-144, %rdx
	mulxq	%rsi, %rsi, %rsi
	shrl	$7, %esi
	incl	%esi
	andl	$7, %esi
	je	.LBB1578_190
	leaq	(%r15,%r15,4), %rdi
	negq	%rsi
	xorl	%r8d, %r8d
	leaq	16(%rcx,%rdi,8), %rdi
.LBB1578_188:
	addq	$-144, %rax
	movl	$3, -16(%rdi)
	movq	%r9, -8(%rdi)
	decq	%r8
	movq	%rax, (%rdi)
	addq	$40, %rdi
	cmpq	%r8, %rsi
	jne	.LBB1578_188
	subq	%r8, %r15
.LBB1578_190:
	cmpq	$1008, %rdx
	jb	.LBB1578_193
	leaq	(%r15,%r15,4), %rdx
	leaq	296(%rcx,%rdx,8), %rcx
.LBB1578_192:
	leaq	-144(%rax), %rdx
	leaq	-288(%rax), %rdi
	leaq	-432(%rax), %rsi
	movl	$3, -296(%rcx)
	movq	%r9, -288(%rcx)
	addq	$8, %r15
	movq	%rdx, -280(%rcx)
	movl	$3, -256(%rcx)
	movq	%r9, -248(%rcx)
	movq	%rdi, -240(%rcx)
	movl	$3, -216(%rcx)
	movq	%r9, -208(%rcx)
	movq	%rsi, -200(%rcx)
	leaq	-576(%rax), %rdi
	leaq	-720(%rax), %rsi
	movl	$3, -176(%rcx)
	movq	%r9, -168(%rcx)
	movq	%rdi, -160(%rcx)
	movl	$3, -136(%rcx)
	movq	%r9, -128(%rcx)
	movq	%rsi, -120(%rcx)
	leaq	-864(%rax), %rdi
	leaq	-1008(%rax), %rsi
	addq	$-1152, %rax
	movl	$3, -96(%rcx)
	movq	%r9, -88(%rcx)
	movq	%rdi, -80(%rcx)
	movl	$3, -56(%rcx)
	movq	%r9, -48(%rcx)
	movq	%rsi, -40(%rcx)
	movl	$3, -16(%rcx)
	movq	%r9, -8(%rcx)
	movq	%rax, (%rcx)
	addq	$320, %rcx
	cmpq	%r14, %rax
	jne	.LBB1578_192
.LBB1578_193:
	movq	%r15, 24(%rsp)
	jmp	.LBB1578_41
.LBB1578_194:
	movq	16(%r12), %rbx
	testq	%rbx, %rbx
	je	.LBB1578_442
	cmpl	$28, (%rbx)
	jne	.LBB1578_237
	movq	8(%r12), %r14
	testq	%r14, %r14
	je	.LBB1578_442
	movq	24(%rsp), %r15
	cmpq	8(%rsp), %r15
	jne	.LBB1578_199
.Ltmp18021:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::bgp::survey_pattern_plans::Step<purrdf_core::ir::term::TermId>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	8(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp18022:
.LBB1578_199:
	movq	16(%rsp), %rcx
	leaq	(%r15,%r15,4), %rax
	movl	$4, (%rcx,%rax,8)
	movq	%rbx, 8(%rcx,%rax,8)
	addq	$8, %rbx
	movq	%rbx, 16(%rcx,%rax,8)
	movq	%r14, 24(%rcx,%rax,8)
	jmp	.LBB1578_240
.LBB1578_200:
	movl	$72, %eax
	jmp	.LBB1578_214
.LBB1578_201:
	cmpb	$0, 8(%r12)
	je	.LBB1578_226
	movq	32(%r12), %rbx
	testq	%rbx, %rbx
	je	.LBB1578_442
	movq	24(%rsp), %r15
	cmpq	8(%rsp), %r15
	jne	.LBB1578_205
.Ltmp18013:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::bgp::survey_pattern_plans::Step<purrdf_core::ir::term::TermId>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	8(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp18014:
.LBB1578_205:
	movq	16(%rsp), %rcx
	leaq	(%r15,%r15,4), %rax
	movl	$3, (%rcx,%rax,8)
	movl	$0, 8(%rcx,%rax,8)
	jmp	.LBB1578_218
.LBB1578_206:
	movl	$24, %eax
	jmp	.LBB1578_214
.LBB1578_207:
	movq	24(%r12), %rbx
	testq	%rbx, %rbx
	je	.LBB1578_442
	movq	24(%rsp), %r15
	cmpq	8(%rsp), %r15
	jne	.LBB1578_210
.Ltmp17977:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::bgp::survey_pattern_plans::Step<purrdf_core::ir::term::TermId>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	8(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp17978:
.LBB1578_210:
	movq	16(%rsp), %rax
	movq	32(%rsp), %rdx
	leaq	(%r15,%r15,4), %rcx
	leaq	1(%r15), %r14
	movl	$3, (%rax,%rcx,8)
	movq	%rdx, 8(%rax,%rcx,8)
	movq	%rbx, 16(%rax,%rcx,8)
	movq	%r14, 24(%rsp)
	movq	16(%r12), %rbx
	testq	%rbx, %rbx
	je	.LBB1578_442
	cmpq	8(%rsp), %r14
	jne	.LBB1578_243
.Ltmp17979:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::bgp::survey_pattern_plans::Step<purrdf_core::ir::term::TermId>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	8(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp17980:
	jmp	.LBB1578_243
.LBB1578_213:
	movl	$56, %eax
.LBB1578_214:
	movq	(%r12,%rax), %rbx
	testq	%rbx, %rbx
	je	.LBB1578_442
	movq	24(%rsp), %r15
	cmpq	8(%rsp), %r15
	jne	.LBB1578_217
.Ltmp18017:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::bgp::survey_pattern_plans::Step<purrdf_core::ir::term::TermId>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	8(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp18018:
.LBB1578_217:
	movq	16(%rsp), %rcx
	movq	32(%rsp), %rdx
	leaq	(%r15,%r15,4), %rax
	movl	$3, (%rcx,%rax,8)
	movq	%rdx, 8(%rcx,%rax,8)
.LBB1578_218:
	movq	%rbx, 16(%rcx,%rax,8)
.LBB1578_219:
	incq	%r15
	movq	%r15, 24(%rsp)
	jmp	.LBB1578_41
.LBB1578_220:
	vmovdqu	.Lanon.4895d20928f7255621bf668e37519d0a.0(%rip), %ymm0
	leaq	8(%r12), %rdx
	vmovdqu	%ymm0, 2144(%rsp)
.Ltmp17981:
	.cfi_escape 0x2e, 0x10
	subq	$8, %rsp
	.cfi_adjust_cfa_offset 8
	movq	112(%rsp), %r9
	leaq	208(%rsp), %rax
	movl	$1, %r8d
	leaq	648(%rsp), %rdi
	movq	%r12, %rsi
	leaq	2152(%rsp), %rcx
	pushq	%rax
	.cfi_adjust_cfa_offset 8
	movq	purrdf_sparql_eval::bgp::record_call_estimate@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	addq	$16, %rsp
	.cfi_adjust_cfa_offset -16
.Ltmp17982:
	movq	640(%rsp), %r14
	cmpq	$-1, %r14
	je	.LBB1578_40
	jmp	.LBB1578_432
.LBB1578_222:
	movzbl	(%rax,%rsi), %edi
	testb	%dil, %dil
	jns	.LBB1578_270
.LBB1578_223:
	leaq	-16(%rsi), %r8
	movb	%cl, (%rax,%rsi)
	vpbroadcastb	.LCPI1578_6(%rip), %xmm1
	andb	$1, %dil
	negq	%rsi
	andq	%rdx, %r8
	movzbl	%dil, %edi
	movb	%cl, 16(%rax,%r8)
	imulq	$104, %rsi, %rcx
	vmovdqa	80(%rsp), %xmm0
	vpinsrq	$0, %rdi, %xmm1, %xmm1
	vpsubq	%xmm1, %xmm0, %xmm0
	vmovdqa	%xmm0, 80(%rsp)
	movq	%r12, -104(%rax,%rcx)
	vmovdqu64	2144(%rsp), %zmm0
	vmovdqu64	2176(%rsp), %zmm1
	vmovdqu64	%zmm0, -96(%rax,%rcx)
	vmovdqu64	%zmm1, -64(%rax,%rcx)
.LBB1578_224:
	xorl	%r12d, %r12d
.Ltmp17920:
	.cfi_escape 0x2e, 0x00
	movq	%rbx, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::bgp::positive::PositivePlan>
.Ltmp17921:
.Ltmp17922:
	.cfi_escape 0x2e, 0x00
	leaq	5536(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
.Ltmp17923:
	jmp	.LBB1578_40
.LBB1578_226:
	movq	16(%r12), %r14
	movq	24(%r12), %rbx
	addq	$16, %r14
	testq	%rbx, %rbx
	js	.LBB1578_293
	je	.LBB1578_244
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_no_alloc_shim_is_unstable_v2@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_alloc@GOTPCREL(%rip), %rax
	movl	$1, %esi
	movq	%rbx, %rdi
	movl	$1, %r13d
	callq	*%rax
	testq	%rax, %rax
	je	.LBB1578_294
	movq	%rax, %r15
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rsi
	movq	memcpy@GOTPCREL(%rip), %r14
	movq	%rax, %rdi
	movq	%rbx, %rdx
	callq	*%r14
	jmp	.LBB1578_245
.LBB1578_230:
	movq	8(%r12), %r14
	testq	%r14, %r14
	je	.LBB1578_442
	movq	24(%rsp), %r15
	cmpq	8(%rsp), %r15
	jne	.LBB1578_233
.Ltmp18029:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::bgp::survey_pattern_plans::Step<purrdf_core::ir::term::TermId>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	8(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp18030:
.LBB1578_233:
	movq	16(%rsp), %rcx
	movq	32(%rsp), %rdx
	leaq	(%r15,%r15,4), %rax
	movq	%rdx, (%rcx,%rax,8)
	movq	%r12, 8(%rcx,%rax,8)
	movq	%r14, 16(%rcx,%rax,8)
	movq	%rbx, 24(%rcx,%rax,8)
	movb	$0, 32(%rcx,%rax,8)
.LBB1578_234:
	leaq	1(%r15), %r14
	movq	%r14, 24(%rsp)
	movq	8(%r12), %rbx
	testq	%rbx, %rbx
	je	.LBB1578_442
	cmpq	8(%rsp), %r14
	jne	.LBB1578_243
.Ltmp18033:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::bgp::survey_pattern_plans::Step<purrdf_core::ir::term::TermId>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	8(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp18034:
	jmp	.LBB1578_243
.LBB1578_237:
	movq	24(%rsp), %r15
	cmpq	8(%rsp), %r15
	jne	.LBB1578_239
.Ltmp18019:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::bgp::survey_pattern_plans::Step<purrdf_core::ir::term::TermId>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	8(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp18020:
.LBB1578_239:
	movq	16(%rsp), %rax
	movq	32(%rsp), %rdx
	leaq	(%r15,%r15,4), %rcx
	movl	$3, (%rax,%rcx,8)
	movq	%rdx, 8(%rax,%rcx,8)
	movq	%rbx, 16(%rax,%rcx,8)
.LBB1578_240:
	leaq	1(%r15), %r14
	movq	%r14, 24(%rsp)
	movq	8(%r12), %rbx
	testq	%rbx, %rbx
	je	.LBB1578_442
	cmpq	8(%rsp), %r14
	jne	.LBB1578_243
.Ltmp18023:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::bgp::survey_pattern_plans::Step<purrdf_core::ir::term::TermId>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	8(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp18024:
.LBB1578_243:
	movq	16(%rsp), %rcx
	movq	32(%rsp), %rdx
	leaq	(%r14,%r14,4), %rax
	addq	$2, %r15
	movl	$3, (%rcx,%rax,8)
	movq	%rdx, 8(%rcx,%rax,8)
	movq	%rbx, 16(%rcx,%rax,8)
	movq	%r15, 24(%rsp)
	jmp	.LBB1578_41
.LBB1578_244:
	movl	$1, %r15d
.LBB1578_245:
	movq	304(%rsp), %rax
	movq	%rbx, 648(%rsp)
	movq	%r15, 656(%rsp)
	movq	%rbx, 664(%rsp)
	movq	%rax, 640(%rsp)
.Ltmp17994:
	.cfi_escape 0x2e, 0x00
	movq	48(%rsp), %rdi
	movq	<purrdf_core::ir::dataset::RdfDataset>::term_id_by_value@GOTPCREL(%rip), %rax
	leaq	640(%rsp), %rsi
	vzeroupper
	callq	*%rax
.Ltmp17995:
	movl	%eax, %ebx
	testl	%eax, %eax
	je	.LBB1578_260
.Ltmp17996:
	.cfi_escape 0x2e, 0x00
	movq	48(%rsp), %rdi
	movq	<purrdf_core::ir::dataset::RdfDataset>::has_named_graph@GOTPCREL(%rip), %rax
	movl	%ebx, %esi
	callq	*%rax
.Ltmp17997:
	testb	%al, %al
	je	.LBB1578_260
	cmpb	$0, 46(%rsp)
	je	.LBB1578_261
	cmpq	$0, 256(%rsp)
	je	.LBB1578_260
	movq	408(%rsp), %rax
	movq	256(%rsp), %rcx
	movzwl	54(%rcx), %esi
	testl	%esi, %esi
	je	.LBB1578_257
.LBB1578_252:
	movl	%esi, %edi
	shll	$2, %edi
	xorl	%edx, %edx
.LBB1578_253:
	cmpl	8(%rcx,%rdx,4), %ebx
	seta	%r8b
	sbbb	$0, %r8b
	cmpb	$1, %r8b
	jne	.LBB1578_256
	incq	%rdx
	addq	$-4, %rdi
	jne	.LBB1578_253
	jmp	.LBB1578_257
.LBB1578_256:
	movzbl	%r8b, %esi
	testl	%esi, %esi
	jne	.LBB1578_258
	jmp	.LBB1578_261
.LBB1578_257:
	movq	%rsi, %rdx
.LBB1578_258:
	subq	$1, %rax
	jb	.LBB1578_260
	movq	56(%rcx,%rdx,8), %rcx
	movzwl	54(%rcx), %esi
	testl	%esi, %esi
	jne	.LBB1578_252
	jmp	.LBB1578_257
.LBB1578_260:
.Ltmp18003:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::bgp::PlanSurvey>::record@GOTPCREL(%rip), %rax
	leaq	200(%rsp), %rdi
	leaq	.Lanon.4895d20928f7255621bf668e37519d0a.1(%rip), %rdx
	movq	%r12, %rsi
	callq	*%rax
.Ltmp18004:
	jmp	.LBB1578_265
.LBB1578_261:
	movq	32(%r12), %r14
	testq	%r14, %r14
	je	.LBB1578_455
	movq	24(%rsp), %r15
	cmpq	8(%rsp), %r15
	jne	.LBB1578_264
.Ltmp17998:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::bgp::survey_pattern_plans::Step<purrdf_core::ir::term::TermId>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	8(%rsp), %rdi
	callq	*%rax
.Ltmp17999:
.LBB1578_264:
	movq	16(%rsp), %rax
	leaq	(%r15,%r15,4), %rcx
	incq	%r15
	movl	$3, (%rax,%rcx,8)
	movl	$2, 8(%rax,%rcx,8)
	movl	%ebx, 12(%rax,%rcx,8)
	movq	%r14, 16(%rax,%rcx,8)
	movq	%r15, 24(%rsp)
.LBB1578_265:
.Ltmp18009:
	.cfi_escape 0x2e, 0x00
	leaq	640(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::ir::term::TermValue>
.Ltmp18010:
	jmp	.LBB1578_40
.LBB1578_266:
.Ltmp17911:
	.cfi_escape 0x2e, 0x00
	movq	<hashbrown::raw::RawTable<(usize, purrdf_sparql_eval::bgp::SeedEstimate)>>::reserve_rehash::<hashbrown::map::make_hasher<usize, purrdf_sparql_eval::bgp::SeedEstimate, purrdf_hash::fixed::FixedState>::{closure#0}>@GOTPCREL(%rip), %rax
	movl	$1, %esi
	leaq	64(%rsp), %rdi
	leaq	96(%rsp), %rdx
	movl	$1, %ecx
	vzeroupper
	callq	*%rax
.Ltmp17912:
	jmp	.LBB1578_145
.LBB1578_267:
	movq	$-1, %rbx
	jmp	.LBB1578_126
.LBB1578_268:
.Ltmp18015:
	.cfi_escape 0x2e, 0x00
	movl	$8, %ecx
	movl	$40, %r8d
	leaq	8(%rsp), %rdi
	movq	%r15, %rsi
	movq	%rbx, %rdx
	vzeroupper
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
.Ltmp18016:
	movq	16(%rsp), %rcx
	movq	24(%rsp), %r15
	movq	32(%rsp), %r9
	jmp	.LBB1578_186
.LBB1578_270:
	vmovdqa	(%rax), %xmm0
	vpmovmskb	%xmm0, %esi
	tzcntl	%esi, %esi
	movzbl	(%rax,%rsi), %edi
	jmp	.LBB1578_223
.LBB1578_271:
	movq	72(%rsp), %rbx
	testq	%rbx, %rbx
	je	.LBB1578_281
	movq	88(%rsp), %r14
	testq	%r14, %r14
	je	.LBB1578_279
	movq	64(%rsp), %r15
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	vpcmpltb	(%r15), %xmm0, %k0
	leaq	16(%r15), %r12
	kmovd	%k0, %ebp
	.p2align	4
.LBB1578_274:
	testw	%bp, %bp
	jne	.LBB1578_277
	.p2align	4
.LBB1578_275:
	vpcmpltb	(%r12), %xmm0, %k0
	addq	$-1664, %r15
	addq	$16, %r12
	kortestw	%k0, %k0
	je	.LBB1578_275
	kmovd	%k0, %ebp
.LBB1578_277:
	xorl	%eax, %eax
	tzcntl	%ebp, %eax
	negq	%rax
	imulq	$104, %rax, %rax
	leaq	-96(%r15,%rax), %rdi
.Ltmp18050:
	.cfi_escape 0x2e, 0x00
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::bgp::SeedEstimate>
.Ltmp18051:
	blsrl	%ebp, %ebp
	decq	%r14
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	jne	.LBB1578_274
.LBB1578_279:
	imulq	$104, %rbx, %rax
	andq	$-16, %rax
	addq	%rax, %rbx
	addq	$129, %rbx
	je	.LBB1578_281
	movq	64(%rsp), %rdi
	subq	%rax, %rdi
	addq	$-112, %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$16, %edx
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
.LBB1578_281:
	movq	8(%rsp), %rax
	movq	$-1, %r14
	testq	%rax, %rax
	jne	.LBB1578_311
	jmp	.LBB1578_319
.LBB1578_282:
	vmovdqu64	672(%rsp), %zmm1
	vmovdqu64	648(%rsp), %zmm0
	movq	2152(%rsp), %rbx
	vmovdqu64	%zmm1, 6808(%rsp)
	vmovdqu64	%zmm0, 6784(%rsp)
	testq	%rbx, %rbx
	je	.LBB1578_300
	movq	2168(%rsp), %r15
	testq	%r15, %r15
	je	.LBB1578_291
	movq	2144(%rsp), %r12
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	vpcmpltb	(%r12), %xmm0, %k0
	leaq	16(%r12), %r13
	kmovd	%k0, %ebp
	jmp	.LBB1578_286
.LBB1578_285:
	blsrl	%ebp, %ebp
	decq	%r15
	je	.LBB1578_291
.LBB1578_286:
	testw	%bp, %bp
	jne	.LBB1578_289
	.p2align	4
.LBB1578_287:
	vpcmpltb	(%r13), %xmm0, %k0
	addq	$-256, %r12
	addq	$16, %r13
	kortestw	%k0, %k0
	je	.LBB1578_287
	kmovd	%k0, %ebp
.LBB1578_289:
	xorl	%eax, %eax
	tzcntl	%ebp, %eax
	movq	%r12, %rdi
	shll	$4, %eax
	subq	%rax, %rdi
	movq	-16(%rdi), %rax
	lock		decq	(%rax)
	jne	.LBB1578_285
	addq	$-16, %rdi
	#MEMBARRIER
.Ltmp17956:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	vpcmpeqd	%xmm0, %xmm0, %xmm0
.Ltmp17957:
	jmp	.LBB1578_285
.LBB1578_291:
	movq	%rbx, %rax
	shlq	$4, %rax
	addq	%rax, %rbx
	addq	$33, %rbx
	je	.LBB1578_300
	movq	2144(%rsp), %rdi
	subq	%rax, %rdi
	addq	$-16, %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$16, %edx
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
	movq	72(%rsp), %rbx
	testq	%rbx, %rbx
	jne	.LBB1578_301
	jmp	.LBB1578_310
.LBB1578_293:
	xorl	%r13d, %r13d
.LBB1578_294:
.Ltmp18011:
	.cfi_escape 0x2e, 0x00
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
.Ltmp18012:
	jmp	.LBB1578_457
.LBB1578_295:
	leaq	648(%rsp), %rax
	movq	656(%rsp), %r14
	vmovups	40(%rax), %zmm1
	vmovups	16(%rax), %zmm0
	vmovups	%zmm1, 8008(%rsp)
	vmovups	%zmm0, 7984(%rsp)
	vmovdqu64	7984(%rsp), %zmm0
	vmovdqu64	8008(%rsp), %zmm1
	vmovdqu64	%zmm0, 6784(%rsp)
	vmovdqu64	%zmm1, 6808(%rsp)
	movq	72(%rsp), %rbx
	testq	%rbx, %rbx
	jne	.LBB1578_301
	jmp	.LBB1578_310
.LBB1578_296:
	leaq	648(%rsp), %rax
	movq	656(%rsp), %r14
	vmovups	40(%rax), %zmm1
	vmovups	16(%rax), %zmm0
	vmovups	%zmm1, 9208(%rsp)
	vmovups	%zmm0, 9184(%rsp)
	vmovdqu64	9184(%rsp), %zmm0
	vmovdqu64	9208(%rsp), %zmm1
	vmovdqu64	%zmm0, 6784(%rsp)
	vmovdqu64	%zmm1, 6808(%rsp)
	jmp	.LBB1578_299
.LBB1578_297:
	vmovdqu64	2176(%rsp), %zmm1
	vmovdqu64	2152(%rsp), %zmm0
	vmovdqu64	%zmm1, 6808(%rsp)
	vmovdqu64	%zmm0, 6784(%rsp)
.Ltmp17973:
	.cfi_escape 0x2e, 0x00
	leaq	640(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::bgp::positive::PositivePlan>
.Ltmp17974:
	jmp	.LBB1578_300
.LBB1578_298:
	vmovdqu64	2176(%rsp), %zmm1
	vmovdqu64	2152(%rsp), %zmm0
	vmovdqu64	%zmm1, 6808(%rsp)
	vmovdqu64	%zmm0, 6784(%rsp)
.Ltmp17894:
	.cfi_escape 0x2e, 0x00
	leaq	640(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::bgp::positive::PositivePlan>
.Ltmp17895:
.LBB1578_299:
.Ltmp17945:
	.cfi_escape 0x2e, 0x00
	leaq	5536(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::bgp::SeedEstimate>
.Ltmp17946:
.LBB1578_300:
	movq	72(%rsp), %rbx
	testq	%rbx, %rbx
	je	.LBB1578_310
.LBB1578_301:
	movq	88(%rsp), %r15
	testq	%r15, %r15
	je	.LBB1578_308
	movq	64(%rsp), %r12
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	vpcmpltb	(%r12), %xmm0, %k0
	leaq	16(%r12), %r13
	kmovd	%k0, %ebp
.LBB1578_303:
	testw	%bp, %bp
	jne	.LBB1578_306
	.p2align	4
.LBB1578_304:
	vpcmpltb	(%r13), %xmm0, %k0
	addq	$-1664, %r12
	addq	$16, %r13
	kortestw	%k0, %k0
	je	.LBB1578_304
	kmovd	%k0, %ebp
.LBB1578_306:
	xorl	%eax, %eax
	tzcntl	%ebp, %eax
	negq	%rax
	imulq	$104, %rax, %rax
	leaq	-96(%r12,%rax), %rdi
.Ltmp18053:
	.cfi_escape 0x2e, 0x00
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::bgp::SeedEstimate>
.Ltmp18054:
	blsrl	%ebp, %ebp
	decq	%r15
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	jne	.LBB1578_303
.LBB1578_308:
	imulq	$104, %rbx, %rax
	andq	$-16, %rax
	addq	%rax, %rbx
	addq	$129, %rbx
	je	.LBB1578_310
	movq	64(%rsp), %rdi
	subq	%rax, %rdi
	addq	$-112, %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$16, %edx
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
.LBB1578_310:
	movq	8(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1578_312
.LBB1578_311:
	movq	16(%rsp), %rdi
	shlq	$3, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	vzeroupper
	callq	*%rax
.LBB1578_312:
	cmpq	$-1, %r14
	je	.LBB1578_319
	vmovdqu64	6808(%rsp), %zmm1
	vmovdqu64	6784(%rsp), %zmm0
	vmovdqu64	%zmm1, 672(%rsp)
	vmovdqu64	%zmm0, 648(%rsp)
	movq	%r14, 640(%rsp)
.Ltmp18056:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::engine::eval_diagnostic_code@GOTPCREL(%rip), %rax
	leaq	.Lalloc_817b9bde4895730a1eec55b6889dc033(%rip), %rsi
	leaq	640(%rsp), %rdi
	movl	$27, %edx
	vzeroupper
	callq	*%rax
.Ltmp18057:
	movq	%rax, %r13
	leaq	3616(%rsp), %rcx
	leaq	.Lvtable.1T(%rip), %rax
	movq	%rdx, %r15
	movq	$0, 3616(%rsp)
	movq	$1, 3624(%rsp)
	movq	$1610612768, 2160(%rsp)
	movq	$0, 3632(%rsp)
	movq	%rcx, 2144(%rsp)
	movq	%rax, 2152(%rsp)
.Ltmp18059:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::error::EvalError as core::fmt::Display>::fmt@GOTPCREL(%rip), %rax
	leaq	640(%rsp), %rdi
	leaq	2144(%rsp), %rsi
	callq	*%rax
.Ltmp18060:
	testb	%al, %al
	jne	.LBB1578_445
	movq	3624(%rsp), %rax
	movq	3616(%rsp), %r14
	movq	3632(%rsp), %rbx
	movq	%rax, 32(%rsp)
	testq	%r15, %r15
	je	.LBB1578_326
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_no_alloc_shim_is_unstable_v2@GOTPCREL(%rip), %rax
	callq	*%rax
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_alloc@GOTPCREL(%rip), %rax
	movl	$1, %esi
	movq	%r15, %rdi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB1578_456
	movq	%rax, %r12
	.cfi_escape 0x2e, 0x00
	movq	%r13, %rsi
	movq	memcpy@GOTPCREL(%rip), %r13
	movq	%rax, %rdi
	movq	%r15, %rdx
	callq	*%r13
	jmp	.LBB1578_327
.LBB1578_319:
	vmovdqu	232(%rsp), %xmm0
	movq	216(%rsp), %rax
	movq	224(%rsp), %rdx
	movq	120(%rsp), %rcx
	movq	200(%rsp), %r12
	movq	208(%rsp), %r15
	movq	248(%rsp), %r14
	movq	%rax, 104(%rsp)
	movq	%rdx, 32(%rsp)
	vmovdqa	%xmm0, 304(%rsp)
	lock		decq	(%rcx)
	jne	.LBB1578_321
	#MEMBARRIER
.Ltmp18082:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::plan::PlanShape>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	120(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp18083:
.LBB1578_321:
	movq	176(%rsp), %rsi
	testq	%rsi, %rsi
	jle	.LBB1578_323
	movq	184(%rsp), %rdi
	shlq	$2, %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$4, %edx
	vzeroupper
	callq	*%rax
.LBB1578_323:
	cmpq	$0, 144(%rsp)
	movq	$-1, %rbx
	je	.LBB1578_349
	movq	152(%rsp), %rcx
	testq	%rcx, %rcx
	je	.LBB1578_336
	movq	160(%rsp), %rdx
	movq	168(%rsp), %rax
	movq	$0, 648(%rsp)
	movq	%rcx, 656(%rsp)
	movq	%rdx, 664(%rsp)
	movq	$0, 680(%rsp)
	movq	%rcx, 688(%rsp)
	movl	$1, %ecx
	movq	%rdx, 696(%rsp)
	jmp	.LBB1578_337
.LBB1578_326:
	movl	$1, %r12d
.LBB1578_327:
.Ltmp18064:
	.cfi_escape 0x2e, 0x00
	leaq	640(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::error::EvalError>
.Ltmp18065:
	vpxor	%xmm0, %xmm0, %xmm0
	movq	%r14, 104(%rsp)
	vmovdqu	%xmm0, 3624(%rsp)
.Ltmp18066:
	.cfi_escape 0x2e, 0x00
	leaq	200(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::bgp::PlanSurvey>
.Ltmp18067:
	movq	120(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB1578_331
	#MEMBARRIER
.Ltmp18069:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::plan::PlanShape>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	120(%rsp), %rdi
	callq	*%rax
.Ltmp18070:
.LBB1578_331:
	movq	176(%rsp), %rsi
	testq	%rsi, %rsi
	jle	.LBB1578_333
	movq	184(%rsp), %rdi
	shlq	$2, %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$4, %edx
	callq	*%rax
.LBB1578_333:
	vpbroadcastb	.LCPI1578_6(%rip), %xmm0
	cmpq	$0, 144(%rsp)
	vpinsrq	$0, %rbx, %xmm0, %xmm0
	vmovdqa	%xmm0, 304(%rsp)
	je	.LBB1578_348
	movq	152(%rsp), %rcx
	testq	%rcx, %rcx
	je	.LBB1578_342
	movq	160(%rsp), %rdx
	movq	168(%rsp), %rax
	movq	$0, 648(%rsp)
	movq	%rcx, 656(%rsp)
	movq	%rdx, 664(%rsp)
	movq	$0, 680(%rsp)
	movq	%rcx, 688(%rsp)
	movl	$1, %ecx
	movq	%rdx, 696(%rsp)
	jmp	.LBB1578_343
.LBB1578_336:
	xorl	%ecx, %ecx
	xorl	%eax, %eax
.LBB1578_337:
	movq	%rcx, 640(%rsp)
	movq	%rcx, 672(%rsp)
	movq	%rax, 704(%rsp)
.Ltmp18085:
	.cfi_escape 0x2e, 0x00
	leaq	2144(%rsp), %rdi
	leaq	640(%rsp), %rsi
	vzeroupper
	callq	<alloc::collections::btree::map::IntoIter<purrdf_core::ir::term::BlankScope, alloc::collections::btree::set_val::SetValZST>>::dying_next
.Ltmp18086:
	cmpq	$0, 2144(%rsp)
	je	.LBB1578_349
	leaq	2144(%rsp), %r13
	leaq	640(%rsp), %rbp
	.p2align	4
.LBB1578_340:
.Ltmp18088:
	.cfi_escape 0x2e, 0x00
	movq	%r13, %rdi
	movq	%rbp, %rsi
	callq	<alloc::collections::btree::map::IntoIter<purrdf_core::ir::term::BlankScope, alloc::collections::btree::set_val::SetValZST>>::dying_next
.Ltmp18089:
	cmpq	$0, 2144(%rsp)
	jne	.LBB1578_340
	jmp	.LBB1578_349
.LBB1578_342:
	xorl	%ecx, %ecx
	xorl	%eax, %eax
.LBB1578_343:
	movq	%rcx, 640(%rsp)
	movq	%rcx, 672(%rsp)
	movq	%rax, 704(%rsp)
.Ltmp18071:
	.cfi_escape 0x2e, 0x00
	leaq	2144(%rsp), %rdi
	leaq	640(%rsp), %rsi
	callq	<alloc::collections::btree::map::IntoIter<purrdf_core::ir::term::BlankScope, alloc::collections::btree::set_val::SetValZST>>::dying_next
.Ltmp18072:
	cmpq	$0, 2144(%rsp)
	je	.LBB1578_348
	leaq	2144(%rsp), %r13
	leaq	640(%rsp), %rbp
	.p2align	4
.LBB1578_346:
.Ltmp18073:
	.cfi_escape 0x2e, 0x00
	movq	%r13, %rdi
	movq	%rbp, %rsi
	callq	<alloc::collections::btree::map::IntoIter<purrdf_core::ir::term::BlankScope, alloc::collections::btree::set_val::SetValZST>>::dying_next
.Ltmp18074:
	cmpq	$0, 2144(%rsp)
	jne	.LBB1578_346
.LBB1578_348:
	movq	%r15, %rbx
.LBB1578_349:
	movq	3632(%rsp), %rax
	vmovdqu	3616(%rsp), %xmm0
	vmovdqa	304(%rsp), %xmm1
	movq	%rax, 720(%rsp)
	movq	104(%rsp), %rax
	vmovdqu	%xmm0, 704(%rsp)
	movq	%rbx, 640(%rsp)
	movq	%r12, 648(%rsp)
	movq	%r15, 656(%rsp)
	movq	%rax, 664(%rsp)
	movq	32(%rsp), %rax
	movq	%rax, 672(%rsp)
	vmovdqu	%xmm1, 680(%rsp)
	movq	%r14, 696(%rsp)
	movb	$0, 728(%rsp)
	movl	608(%rsp), %eax
	testl	%eax, %eax
	je	.LBB1578_355
.LBB1578_350:
	leaq	648(%rsp), %rcx
	leaq	704(%rsp), %rax
	movq	640(%rsp), %rbx
	vmovdqu	(%rcx), %ymm0
	vmovups	24(%rcx), %ymm1
	vmovdqu	%ymm0, 2080(%rsp)
	vmovups	%ymm1, 2104(%rsp)
	vmovdqu	(%rax), %ymm1
	vmovdqu	%ymm1, 3392(%rsp)
.Ltmp18128:
	.cfi_escape 0x2e, 0x00
	leaq	584(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::user_fn::RefusalPublication>
.Ltmp18129:
	movq	112(%rsp), %r14
	cmpq	$-1, %rbx
	je	.LBB1578_367
	vmovups	2104(%rsp), %ymm1
	vmovdqu	2080(%rsp), %ymm0
	vmovups	%ymm1, 344(%rsp)
	vmovdqu	3392(%rsp), %ymm1
	vmovdqu	%ymm0, 320(%rsp)
	vmovdqu	%ymm1, 4864(%rsp)
.LBB1578_353:
	vmovups	344(%rsp), %ymm1
	vmovdqu	320(%rsp), %ymm0
	movq	$-2, %rax
	vmovups	%ymm1, 680(%rsp)
	vmovdqu	4864(%rsp), %ymm1
	vmovdqu	%ymm0, 656(%rsp)
	vmovdqu	%ymm1, 712(%rsp)
.LBB1578_354:
	movq	%rbx, 648(%rsp)
	movq	%rax, 640(%rsp)
	jmp	.LBB1578_400
.LBB1578_355:
	movq	600(%rsp), %r14
	movl	$1, %ecx
	xorl	%eax, %eax
	lock		cmpxchgl	%ecx, 32(%r14)
	leaq	32(%r14), %rbx
	jne	.LBB1578_446
.LBB1578_356:
	movq	std::panicking::panic_count::GLOBAL_PANIC_COUNT@GOTPCREL(%rip), %r15
	movq	(%r15), %rax
	shlq	%rax
	testq	%rax, %rax
	jne	.LBB1578_447
	movzbl	36(%r14), %eax
	vmovdqu	40(%r14), %ymm0
	leaq	36(%r14), %r12
	vmovdqu	%ymm0, 3616(%rsp)
	movq	$0, 40(%r14)
	movq	(%r15), %rax
	shlq	%rax
	testq	%rax, %rax
	jne	.LBB1578_450
.LBB1578_358:
	xorl	%eax, %eax
	xchgl	%eax, (%rbx)
	cmpl	$2, %eax
	je	.LBB1578_453
.LBB1578_359:
	cmpq	$-1, 640(%rsp)
	movq	3616(%rsp), %rax
	je	.LBB1578_377
	testq	%rax, %rax
	je	.LBB1578_377
	vmovdqu	3616(%rsp), %ymm0
	movq	600(%rsp), %rax
	vmovdqu	%ymm0, 2144(%rsp)
	movq	16(%rax), %rcx
	movq	24(%rax), %rax
	movq	16(%rax), %rdx
	movq	32(%rax), %rax
	decq	%rdx
	andq	$-16, %rdx
	leaq	16(%rcx,%rdx), %rdi
.Ltmp18102:
	.cfi_escape 0x2e, 0x00
	leaq	2144(%rsp), %rsi
	vzeroupper
	callq	*%rax
.Ltmp18103:
	movq	2144(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB1578_364
	#MEMBARRIER
.Ltmp18107:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn core::error::Error + core::marker::Sync + core::marker::Send>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	2144(%rsp), %rdi
	callq	*%rax
.Ltmp18108:
.LBB1578_364:
	movq	2160(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1578_350
	lock		decq	(%rax)
	jne	.LBB1578_350
	leaq	2160(%rsp), %rdi
	#MEMBARRIER
.Ltmp18113:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn purrdf_sparql_eval::user_fn::UserFunctionAdmission>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp18114:
	jmp	.LBB1578_350
.LBB1578_367:
	vmovdqu	2104(%rsp), %ymm1
	vmovdqu	2080(%rsp), %ymm0
	vmovdqu	%ymm1, 664(%rsp)
	vmovdqu	%ymm0, 640(%rsp)
.Ltmp18130:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::bgp::PlanSurvey>::peak_cells@GOTPCREL(%rip), %rax
	leaq	640(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp18131:
.Ltmp18135:
	movq	%rax, %rbx
	.cfi_escape 0x2e, 0x00
	leaq	640(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::bgp::PlanSurvey>
.Ltmp18136:
	movq	416(%rsp), %rax
	cmpq	%rax, %rbx
	jbe	.LBB1578_388
	movq	%rax, 648(%rsp)
	movq	%rbx, 656(%rsp)
	movw	$514, 640(%rsp)
.Ltmp18137:
	.cfi_escape 0x2e, 0x00
	movq	96(%rsp), %rcx
	movq	<purrdf_sparql_eval::governor::GovernorState>::record_trip@GOTPCREL(%rip), %rax
	leaq	9184(%rsp), %rdi
	leaq	640(%rsp), %rdx
	leaq	16(%rcx), %rsi
	callq	*%rax
.Ltmp18138:
.Ltmp18139:
	.cfi_escape 0x2e, 0x00
	movq	96(%rsp), %rcx
	movq	<purrdf_sparql_eval::governor::GovernorState>::evidence@GOTPCREL(%rip), %rax
	leaq	640(%rsp), %rdi
	leaq	16(%rcx), %rsi
	callq	*%rax
.Ltmp18140:
.Ltmp18141:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::string::String as core::clone::Clone>::clone@GOTPCREL(%rip), %rax
	leaq	7984(%rsp), %rdi
	leaq	448(%rsp), %rsi
	callq	*%rax
.Ltmp18142:
	movq	488(%rsp), %rdx
	movq	480(%rsp), %rsi
.Ltmp18144:
	.cfi_escape 0x2e, 0x00
	leaq	5536(%rsp), %rdi
	callq	<alloc::vec::Vec<alloc::string::String> as core::clone::Clone>::clone
.Ltmp18145:
	cmpq	$0, 512(%rsp)
	je	.LBB1578_383
	movq	496(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB1578_454
	movq	504(%rsp), %rdx
.Ltmp18147:
	.cfi_escape 0x2e, 0x00
	leaq	2144(%rsp), %rdi
	callq	<alloc::collections::btree::map::BTreeMap<_, _, _> as core::clone::Clone>::clone::clone_subtree::<alloc::string::String, purrdf_sparql_eval::witness::RelationAttestations, alloc::alloc::Global>
.Ltmp18148:
	jmp	.LBB1578_384
.LBB1578_377:
	testq	%rax, %rax
	je	.LBB1578_350
	lock		decq	(%rax)
	jne	.LBB1578_380
	#MEMBARRIER
.Ltmp18116:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn core::error::Error + core::marker::Sync + core::marker::Send>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	3616(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp18117:
.LBB1578_380:
	movq	3632(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1578_350
	lock		decq	(%rax)
	jne	.LBB1578_350
	leaq	3632(%rsp), %rdi
	#MEMBARRIER
.Ltmp18122:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn purrdf_sparql_eval::user_fn::UserFunctionAdmission>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp18123:
	jmp	.LBB1578_350
.LBB1578_383:
	movq	$0, 2144(%rsp)
	movq	$0, 2160(%rsp)
.LBB1578_384:
	vmovups	7984(%rsp), %xmm0
	movq	2160(%rsp), %rax
	movq	2144(%rsp), %rcx
	movq	2152(%rsp), %rdx
	movq	%rax, 3680(%rsp)
	movq	%rcx, 3664(%rsp)
	movq	%rdx, 3672(%rsp)
	movq	8000(%rsp), %rcx
	movq	5552(%rsp), %rax
	vmovaps	%xmm0, 3616(%rsp)
	vmovdqu	5536(%rsp), %xmm0
	movq	%rcx, 3632(%rsp)
	movq	%rax, 3656(%rsp)
	vmovdqu	%xmm0, 3640(%rsp)
.Ltmp18152:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::engine::empty_result_for@GOTPCREL(%rip), %rax
	leaq	5536(%rsp), %rdi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp18153:
.Ltmp18154:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::engine::certain_partial@GOTPCREL(%rip), %rax
	leaq	2144(%rsp), %rdi
	leaq	5536(%rsp), %rsi
	movl	$1, %edx
	callq	*%rax
.Ltmp18155:
	movq	9200(%rsp), %rax
	vmovdqu	9184(%rsp), %xmm0
	vmovups	704(%rsp), %zmm1
	vmovups	768(%rsp), %zmm2
	vmovups	640(%rsp), %zmm4
	vmovups	3616(%rsp), %zmm3
	movq	2152(%rsp), %rbx
	movq	%rax, 5152(%rsp)
	movq	832(%rsp), %rax
	vmovdqa	%xmm0, 5136(%rsp)
	vmovups	%zmm1, 4928(%rsp)
	vmovups	%zmm2, 4992(%rsp)
	vmovdqu	2184(%rsp), %ymm1
	vmovdqu	2160(%rsp), %ymm2
	vmovups	%zmm4, 4864(%rsp)
	movq	%rax, 5056(%rsp)
	movq	3680(%rsp), %rax
	vmovups	%zmm3, 5064(%rsp)
	movq	%rax, 5128(%rsp)
	movq	2144(%rsp), %rax
	vmovdqu	%ymm1, 344(%rsp)
	vmovdqu	%ymm2, 320(%rsp)
	cmpq	$-1, %rax
	je	.LBB1578_353
	cmpq	$-2, %rax
	jne	.LBB1578_421
.LBB1578_388:
.Ltmp18176:
	.cfi_escape 0x2e, 0x00
	movq	432(%rsp), %rsi
	movq	48(%rsp), %rdx
	leaq	640(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::engine::NativeSparqlEngine>::eval_ctx::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::dataset_view::NoopReservation<!>>
.Ltmp18177:
.Ltmp18178:
	.cfi_escape 0x2e, 0x00
	leaq	3616(%rsp), %rdi
	leaq	640(%rsp), %rsi
	leaq	3432(%rsp), %rdx
	callq	purrdf_sparql_eval::engine::apply_query_options::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp18179:
	movq	4816(%rsp), %r15
	cmpq	$2, %r15
	jne	.LBB1578_392
	vmovups	3648(%rsp), %zmm1
	vmovups	3616(%rsp), %zmm0
	vmovups	%zmm1, 8016(%rsp)
	vmovups	%zmm0, 7984(%rsp)
	vmovups	7984(%rsp), %zmm0
	vmovups	8016(%rsp), %zmm1
	vmovups	%zmm0, 9184(%rsp)
	vmovups	%zmm1, 9216(%rsp)
	jmp	.LBB1578_399
.LBB1578_392:
	.cfi_escape 0x2e, 0x00
	movq	memcpy@GOTPCREL(%rip), %r14
	leaq	7984(%rsp), %rbx
	leaq	3616(%rsp), %rsi
	movl	$1200, %edx
	movq	%rbx, %rdi
	callq	*%r14
	vmovdqu	4824(%rsp), %ymm0
	movq	4856(%rsp), %rax
	movq	%rax, 3384(%rsp)
	vmovdqu	%ymm0, 3352(%rsp)
	.cfi_escape 0x2e, 0x00
	leaq	2144(%rsp), %rdi
	movl	$1200, %edx
	movq	%rbx, %rsi
	vzeroupper
	callq	*%r14
	movq	96(%rsp), %rcx
	movq	%r15, 3344(%rsp)
	lock		incq	(%rcx)
	jle	.LBB1578_457
	movq	2760(%rsp), %rax
	cmpq	%rcx, %rax
	je	.LBB1578_395
	movq	$0, 3200(%rsp)
.LBB1578_395:
	testq	%rax, %rax
	je	.LBB1578_398
	lock		decq	(%rax)
	jne	.LBB1578_398
	leaq	2760(%rsp), %rdi
	#MEMBARRIER
.Ltmp18180:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp18181:
.LBB1578_398:
	movq	96(%rsp), %rax
	leaq	3352(%rsp), %rbx
	movq	%rax, 2760(%rsp)
	.cfi_escape 0x2e, 0x00
	leaq	9184(%rsp), %rdi
	leaq	2144(%rsp), %rsi
	movl	$1200, %edx
	callq	*%r14
	vmovups	(%rbx), %xmm0
	vmovups	15(%rbx), %xmm1
	movq	3344(%rsp), %r12
	movq	3384(%rsp), %r15
	vmovaps	%xmm0, 1888(%rsp)
	vmovups	%xmm1, 1903(%rsp)
	cmpq	$2, %r12
	jne	.LBB1578_410
.LBB1578_399:
	vmovups	9216(%rsp), %zmm1
	vmovups	9184(%rsp), %zmm0
	vmovups	%zmm1, 6816(%rsp)
	vmovups	%zmm0, 6784(%rsp)
	vmovdqu64	6784(%rsp), %zmm0
	vmovdqu64	6816(%rsp), %zmm1
	vmovdqu64	%zmm0, 648(%rsp)
	vmovdqu64	%zmm1, 680(%rsp)
	movq	$-2, 640(%rsp)
.LBB1578_400:
	movq	448(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB1578_402
	movq	456(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	vzeroupper
	callq	*%rax
.LBB1578_402:
	movq	480(%rsp), %rbx
	movq	488(%rsp), %r14
	testq	%r14, %r14
	je	.LBB1578_407
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %r12
	leaq	8(%rbx), %r15
	jmp	.LBB1578_405
	.p2align	4
.LBB1578_404:
	addq	$24, %r15
	decq	%r14
	je	.LBB1578_407
.LBB1578_405:
	movq	-8(%r15), %rsi
	testq	%rsi, %rsi
	je	.LBB1578_404
	movq	(%r15), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	*%r12
	jmp	.LBB1578_404
.LBB1578_407:
	movq	472(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1578_409
	shlq	$3, %rax
	leaq	(%rax,%rax,2), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB1578_409:
	leaq	496(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::collections::btree::map::BTreeMap<alloc::string::String, purrdf_sparql_eval::witness::RelationAttestations>>
	jmp	.LBB1578_10
.LBB1578_410:
	.cfi_escape 0x2e, 0x00
	leaq	6784(%rsp), %rbx
	leaq	9184(%rsp), %rsi
	movl	$1200, %edx
	movq	%rbx, %rdi
	callq	*%r14
	vmovdqa	1888(%rsp), %xmm0
	vmovdqu	1903(%rsp), %xmm1
	vmovdqu	%xmm0, 6744(%rsp)
	vmovdqu	%xmm1, 6759(%rsp)
	.cfi_escape 0x2e, 0x00
	leaq	5536(%rsp), %rdi
	movl	$1200, %edx
	movq	%rbx, %rsi
	callq	*%r14
	movzbl	47(%rsp), %eax
	movq	264(%rsp), %rcx
	movq	%r12, 6736(%rsp)
	movb	$1, 6775(%rsp)
	movq	%r15, 6776(%rsp)
	orb	%al, 6770(%rsp)
	movq	(%rcx), %rax
	movq	8(%rcx), %rcx
	movq	%rax, 536(%rsp)
	movq	%rcx, 544(%rsp)
	movq	$0, 528(%rsp)
	testq	%rcx, %rcx
	je	.LBB1578_418
	movb	$1, %bpl
.Ltmp18186:
	.cfi_escape 0x2e, 0x00
	movq	112(%rsp), %rbx
	leaq	640(%rsp), %rdi
	movq	%rbx, %rsi
	callq	<purrdf_sparql_algebra::algebra::Query as core::clone::Clone>::clone
.Ltmp18187:
.Ltmp18188:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::substitute::apply_shacl_prebinding@GOTPCREL(%rip), %rax
	leaq	2144(%rsp), %rdi
	leaq	640(%rsp), %rsi
	leaq	528(%rsp), %rdx
	callq	*%rax
.Ltmp18189:
	vmovups	2152(%rsp), %zmm0
	vmovups	2184(%rsp), %zmm1
	movq	2144(%rsp), %rax
	vmovups	%zmm0, 1888(%rsp)
	vmovups	%zmm1, 1920(%rsp)
	cmpq	$-1, %rax
	je	.LBB1578_422
	vmovups	2344(%rsp), %zmm2
	vmovups	2312(%rsp), %zmm1
	vmovdqu64	2248(%rsp), %zmm0
	vmovups	%zmm2, 3816(%rsp)
	vmovups	%zmm1, 3784(%rsp)
	vmovdqu64	1888(%rsp), %zmm2
	vmovdqu64	1920(%rsp), %zmm1
	vmovdqu64	%zmm0, 3720(%rsp)
	vmovdqu64	%zmm2, 3624(%rsp)
	vmovdqu64	%zmm1, 3656(%rsp)
	movq	%rax, 3616(%rsp)
.Ltmp18190:
	.cfi_escape 0x2e, 0x00
	leaq	7984(%rsp), %rdi
	leaq	3616(%rsp), %rsi
	leaq	5536(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	purrdf_sparql_eval::eval::evaluate_query_evaluated_over::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp18191:
	cmpq	$-2, 7984(%rsp)
	jne	.LBB1578_423
	leaq	8000(%rsp), %rsi
.Ltmp18194:
	.cfi_escape 0x2e, 0x00
	leaq	640(%rsp), %rdi
	callq	<purrdf_sparql_eval::engine::evaluate_with_substitutions<purrdf_core::ir::dataset::RdfDataset>::{closure#0} as core::ops::function::FnOnce<(purrdf_sparql_eval::error::EvalError,)>>::call_once
.Ltmp18195:
	vmovdqu64	672(%rsp), %zmm1
	vmovdqu64	640(%rsp), %zmm0
	vmovdqu64	%zmm1, 6824(%rsp)
	vmovdqu64	%zmm0, 6792(%rsp)
	movq	$-2, 6784(%rsp)
	jmp	.LBB1578_425
.LBB1578_418:
	movq	112(%rsp), %rbx
	movb	$1, %bpl
	leaq	352(%rbx), %rdx
.Ltmp18202:
	.cfi_escape 0x2e, 0x00
	leaq	9184(%rsp), %rdi
	leaq	5536(%rsp), %rcx
	movq	%rbx, %rsi
	callq	purrdf_sparql_eval::eval::evaluate_query_evaluated_over::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp18203:
	cmpq	$-2, 9184(%rsp)
	jne	.LBB1578_426
	leaq	9200(%rsp), %rsi
	leaq	6792(%rsp), %rdi
.Ltmp18208:
	.cfi_escape 0x2e, 0x00
	callq	<purrdf_sparql_eval::engine::evaluate_with_substitutions<purrdf_core::ir::dataset::RdfDataset>::{closure#0} as core::ops::function::FnOnce<(purrdf_sparql_eval::error::EvalError,)>>::call_once
.Ltmp18209:
	jmp	.LBB1578_429
.LBB1578_421:
	vmovups	344(%rsp), %ymm1
	leaq	4896(%rsp), %rcx
	vmovdqu	320(%rsp), %ymm0
	vmovups	4864(%rsp), %ymm5
	vmovups	(%rcx), %zmm4
	vmovdqu64	128(%rcx), %zmm2
	vmovups	192(%rcx), %zmm3
	vmovups	%ymm1, 680(%rsp)
	vmovdqu64	64(%rcx), %zmm1
	movq	256(%rcx), %rcx
	vmovdqu	%ymm0, 656(%rsp)
	vmovups	%ymm5, 712(%rsp)
	vmovups	%zmm4, 744(%rsp)
	vmovdqu64	%zmm2, 872(%rsp)
	vmovups	%zmm3, 936(%rsp)
	movq	%rcx, 1000(%rsp)
	vmovdqu64	%zmm1, 808(%rsp)
	jmp	.LBB1578_354
.LBB1578_422:
	vmovdqu64	1888(%rsp), %zmm0
	vmovdqu64	1920(%rsp), %zmm1
	vmovdqu64	%zmm0, 6792(%rsp)
	vmovdqu64	%zmm1, 6824(%rsp)
	jmp	.LBB1578_429
.LBB1578_423:
.Ltmp18192:
	.cfi_escape 0x2e, 0x00
	leaq	640(%rsp), %rdi
	leaq	7984(%rsp), %rdx
	movq	%rbx, %rsi
	callq	<purrdf_sparql_eval::engine::PreparedQuery>::restore_evaluated_layout::<purrdf_core::ir::term::TermId>
.Ltmp18193:
	vmovdqu64	640(%rsp), %zmm0
	vmovdqu64	704(%rsp), %zmm1
	vmovdqu64	%zmm1, 8048(%rsp)
	vmovdqu64	%zmm0, 7984(%rsp)
	vmovdqu64	%zmm0, 6784(%rsp)
	vmovdqu64	%zmm1, 6848(%rsp)
.LBB1578_425:
.Ltmp18200:
	.cfi_escape 0x2e, 0x00
	leaq	3616(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_algebra::algebra::Query>
.Ltmp18201:
	jmp	.LBB1578_428
.LBB1578_426:
.Ltmp18204:
	.cfi_escape 0x2e, 0x00
	leaq	640(%rsp), %rdi
	leaq	9184(%rsp), %rdx
	movq	%rbx, %rsi
	callq	<purrdf_sparql_eval::engine::PreparedQuery>::restore_evaluated_layout::<purrdf_core::ir::term::TermId>
.Ltmp18205:
	vmovdqu64	640(%rsp), %zmm0
	vmovdqu64	704(%rsp), %zmm1
	vmovdqu64	%zmm1, 9248(%rsp)
	vmovdqu64	%zmm0, 9184(%rsp)
	vmovdqu64	%zmm0, 6784(%rsp)
	vmovdqu64	%zmm1, 6848(%rsp)
.LBB1578_428:
	movq	6784(%rsp), %rax
	cmpq	$-2, %rax
	jne	.LBB1578_430
.LBB1578_429:
	vmovups	6824(%rsp), %zmm1
	vmovups	6792(%rsp), %zmm0
	vmovups	%zmm1, 2016(%rsp)
	vmovups	%zmm0, 1984(%rsp)
	vmovdqu64	1984(%rsp), %zmm0
	vmovdqu64	2016(%rsp), %zmm1
	vmovdqu64	%zmm0, 648(%rsp)
	vmovdqu64	%zmm1, 680(%rsp)
	movq	$-2, 640(%rsp)
.Ltmp18213:
	.cfi_escape 0x2e, 0x00
	leaq	5536(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::eval::EvalCtx>
.Ltmp18214:
	jmp	.LBB1578_400
.LBB1578_430:
	vmovups	6824(%rsp), %zmm1
	vmovdqu64	6792(%rsp), %zmm0
	vmovups	6888(%rsp), %xmm3
	movq	6904(%rsp), %rcx
	xorl	%ebp, %ebp
	movq	%rcx, 2264(%rsp)
	vmovups	%zmm1, 2016(%rsp)
	vmovdqu64	%zmm0, 1984(%rsp)
	vmovups	%xmm3, 2248(%rsp)
	movq	%rax, 2144(%rsp)
	vmovdqu64	1984(%rsp), %zmm2
	vmovdqu64	2016(%rsp), %zmm1
	vmovdqu64	%zmm2, 2152(%rsp)
	vmovdqu64	%zmm1, 2184(%rsp)
.Ltmp18206:
	.cfi_escape 0x2e, 0x00
	movq	424(%rsp), %rcx
	leaq	640(%rsp), %rdi
	leaq	2144(%rsp), %rsi
	leaq	5536(%rsp), %rdx
	leaq	448(%rsp), %r8
	vzeroupper
	callq	purrdf_sparql_eval::engine::materialize_governed::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp18207:
	.cfi_escape 0x2e, 0x00
	leaq	5536(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::eval::EvalCtx>
	jmp	.LBB1578_10
.LBB1578_432:
	vmovdqu64	672(%rsp), %zmm1
	vmovdqu64	648(%rsp), %zmm0
	vmovdqu64	%zmm1, 6808(%rsp)
	vmovdqu64	%zmm0, 6784(%rsp)
.Ltmp17986:
	.cfi_escape 0x2e, 0x00
	leaq	.Lanon.4895d20928f7255621bf668e37519d0a.0(%rip), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<std::collections::hash::set::HashSet<purrdf_sparql_algebra::ast::Variable, purrdf_hash::fixed::FixedState>>
.Ltmp17987:
	jmp	.LBB1578_300
.LBB1578_433:
	vmovups	2176(%rsp), %zmm1
	vmovups	2152(%rsp), %zmm0
	vmovups	%zmm1, 6808(%rsp)
	vmovups	%zmm0, 6784(%rsp)
	movq	72(%rsp), %rbx
	testq	%rbx, %rbx
	jne	.LBB1578_301
	jmp	.LBB1578_310
.LBB1578_434:
.Ltmp18219:
	.cfi_escape 0x2e, 0x00
	movq	<std::sys::sync::mutex::futex::Mutex>::lock_contended@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.Ltmp18220:
	jmp	.LBB1578_14
.LBB1578_435:
.Ltmp18221:
	.cfi_escape 0x2e, 0x00
	movq	std::panicking::panic_count::is_zero_slow_path@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp18222:
	movzbl	36(%r14), %ecx
	vmovdqu	40(%r14), %ymm0
	vmovdqu	%ymm0, 2144(%rsp)
	movq	$0, 40(%r14)
	testb	%al, %al
	je	.LBB1578_16
	addq	$36, %r14
	movq	%r14, %r12
	movq	(%r15), %rax
	shlq	%rax
	testq	%rax, %rax
	je	.LBB1578_16
.LBB1578_438:
.Ltmp18223:
	.cfi_escape 0x2e, 0x00
	movq	std::panicking::panic_count::is_zero_slow_path@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp18224:
	testb	%al, %al
	jne	.LBB1578_16
	movb	$1, (%r12)
	jmp	.LBB1578_16
.LBB1578_441:
.Ltmp18225:
	.cfi_escape 0x2e, 0x00
	movq	<std::sys::sync::mutex::futex::Mutex>::wake@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.Ltmp18226:
	jmp	.LBB1578_17
.LBB1578_442:
.Ltmp18035:
	.cfi_escape 0x2e, 0x00
	movq	core::option::expect_failed@GOTPCREL(%rip), %rax
	leaq	.Lalloc_3520ecca3c1410886a1093aa5b6a9ef0(%rip), %rdi
	leaq	.Lalloc_c80cb1eed396638af3e1ff32f4192b23(%rip), %rdx
	movl	$48, %esi
	vzeroupper
	callq	*%rax
.Ltmp18036:
	jmp	.LBB1578_457
.LBB1578_443:
.Ltmp18160:
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$40, %esi
	callq	*%rax
.Ltmp18161:
	jmp	.LBB1578_457
.LBB1578_444:
.Ltmp17925:
	.cfi_escape 0x2e, 0x00
	movq	core::option::expect_failed@GOTPCREL(%rip), %rax
	leaq	.Lalloc_a0f84bfa4ef44e88ef35ce6fc697d04f(%rip), %rdi
	leaq	.Lalloc_9f1cbd7d1bea04a26c70a005baa42817(%rip), %rdx
	movl	$28, %esi
	callq	*%rax
.Ltmp17926:
	jmp	.LBB1578_457
.LBB1578_445:
.Ltmp18076:
	.cfi_escape 0x2e, 0x00
	movq	core::result::unwrap_failed@GOTPCREL(%rip), %rax
	leaq	.Lalloc_cc656815297f75969399c3f4b1ad3de4(%rip), %rdi
	leaq	.Lvtable.2q(%rip), %rcx
	leaq	.Lalloc_d4c8062c4f28c49e31e589e7f415a063(%rip), %r8
	leaq	63(%rsp), %rdx
	movl	$55, %esi
	callq	*%rax
.Ltmp18077:
	jmp	.LBB1578_457
.LBB1578_446:
.Ltmp18091:
	.cfi_escape 0x2e, 0x00
	movq	<std::sys::sync::mutex::futex::Mutex>::lock_contended@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.Ltmp18092:
	jmp	.LBB1578_356
.LBB1578_447:
.Ltmp18093:
	.cfi_escape 0x2e, 0x00
	movq	std::panicking::panic_count::is_zero_slow_path@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp18094:
	movzbl	36(%r14), %ecx
	vmovdqu	40(%r14), %ymm0
	vmovdqu	%ymm0, 3616(%rsp)
	movq	$0, 40(%r14)
	testb	%al, %al
	je	.LBB1578_358
	addq	$36, %r14
	movq	%r14, %r12
	movq	(%r15), %rax
	shlq	%rax
	testq	%rax, %rax
	je	.LBB1578_358
.LBB1578_450:
.Ltmp18095:
	.cfi_escape 0x2e, 0x00
	movq	std::panicking::panic_count::is_zero_slow_path@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp18096:
	testb	%al, %al
	jne	.LBB1578_358
	movb	$1, (%r12)
	jmp	.LBB1578_358
.LBB1578_453:
.Ltmp18097:
	.cfi_escape 0x2e, 0x00
	movq	<std::sys::sync::mutex::futex::Mutex>::wake@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.Ltmp18098:
	jmp	.LBB1578_359
.LBB1578_454:
.Ltmp18149:
	.cfi_escape 0x2e, 0x00
	movq	core::option::unwrap_failed@GOTPCREL(%rip), %rax
	leaq	.Lalloc_58a7e7a63bfb0c1b555825150a2e554d(%rip), %rdi
	callq	*%rax
.Ltmp18150:
	jmp	.LBB1578_457
.LBB1578_455:
.Ltmp18000:
	.cfi_escape 0x2e, 0x00
	movq	core::option::expect_failed@GOTPCREL(%rip), %rax
	leaq	.Lalloc_3520ecca3c1410886a1093aa5b6a9ef0(%rip), %rdi
	leaq	.Lalloc_c80cb1eed396638af3e1ff32f4192b23(%rip), %rdx
	movl	$48, %esi
	callq	*%rax
.Ltmp18001:
	jmp	.LBB1578_457
.LBB1578_456:
.Ltmp18061:
	.cfi_escape 0x2e, 0x00
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$1, %edi
	movq	%r15, %rsi
	callq	*%rax
.Ltmp18062:
.LBB1578_457:
	ud2
.LBB1578_458:
.Ltmp18099:
	cmpq	$0, 3616(%rsp)
	movq	%rax, %rbx
	je	.LBB1578_486
.Ltmp18100:
	.cfi_escape 0x2e, 0x00
	leaq	3616(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::user_fn::UserFunctionRefusal>
.Ltmp18101:
	jmp	.LBB1578_486
.LBB1578_460:
.Ltmp17913:
	movq	%rax, %rbx
.Ltmp17914:
	.cfi_escape 0x2e, 0x00
	leaq	2144(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::bgp::SeedEstimate>
.Ltmp17915:
	jmp	.LBB1578_495
.LBB1578_461:
.Ltmp17916:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1578_462:
.Ltmp18227:
	cmpq	$0, 2144(%rsp)
	movq	%rax, %rbx
	je	.LBB1578_525
.Ltmp18228:
	.cfi_escape 0x2e, 0x00
	leaq	2144(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::user_fn::UserFunctionRefusal>
.Ltmp18229:
	jmp	.LBB1578_525
.LBB1578_464:
.Ltmp17896:
	movb	$1, %r12b
	movq	%rax, %rbx
	jmp	.LBB1578_556
.LBB1578_465:
.Ltmp18124:
	movq	%rax, %rbx
	jmp	.LBB1578_486
.LBB1578_466:
.Ltmp18118:
	movq	%rax, %rbx
	movq	3632(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1578_486
	lock		decq	(%rax)
	jne	.LBB1578_486
	leaq	3632(%rsp), %rdi
	#MEMBARRIER
.Ltmp18119:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn purrdf_sparql_eval::user_fn::UserFunctionAdmission>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp18120:
	jmp	.LBB1578_486
.LBB1578_469:
.Ltmp18121:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1578_470:
.Ltmp18115:
	movq	%rax, %rbx
	jmp	.LBB1578_486
.LBB1578_471:
.Ltmp18109:
	movq	%rax, %rbx
	movq	2160(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1578_486
	lock		decq	(%rax)
	jne	.LBB1578_486
	leaq	2160(%rsp), %rdi
	#MEMBARRIER
.Ltmp18110:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn purrdf_sparql_eval::user_fn::UserFunctionAdmission>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp18111:
	jmp	.LBB1578_486
.LBB1578_474:
.Ltmp18112:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1578_475:
.Ltmp18182:
	movq	96(%rsp), %rcx
	movq	%rax, %rbx
	movq	%rcx, 2760(%rsp)
.Ltmp18183:
	.cfi_escape 0x2e, 0x00
	leaq	2144(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::eval::EvalCtx>
.Ltmp18184:
	jmp	.LBB1578_582
.LBB1578_476:
.Ltmp18185:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1578_477:
.Ltmp18040:
	movq	%rax, %rbx
.Ltmp18041:
	.cfi_escape 0x2e, 0x00
	leaq	640(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::bgp::SeedEstimate>
.Ltmp18042:
	jmp	.LBB1578_563
.LBB1578_478:
.Ltmp18043:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1578_479:
.Ltmp18005:
	jmp	.LBB1578_538
.LBB1578_480:
.Ltmp18196:
	movq	%rax, %rbx
.Ltmp18197:
	.cfi_escape 0x2e, 0x00
	leaq	3616(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_algebra::algebra::Query>
.Ltmp18198:
	movb	$1, %bpl
	jmp	.LBB1578_498
.LBB1578_482:
.Ltmp18199:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1578_483:
.Ltmp18146:
	movq	%rax, %rbx
	jmp	.LBB1578_544
.LBB1578_484:
.Ltmp18143:
	movq	%rax, %rbx
	jmp	.LBB1578_546
.LBB1578_485:
.Ltmp18104:
	movq	%rax, %rbx
.Ltmp18105:
	.cfi_escape 0x2e, 0x00
	leaq	2144(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::user_fn::UserFunctionRefusal>
.Ltmp18106:
.LBB1578_486:
.Ltmp18125:
	.cfi_escape 0x2e, 0x00
	leaq	640(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::result::Result<purrdf_sparql_eval::bgp::PlanSurvey, purrdf_core::diagnostic::RdfDiagnostic>>
.Ltmp18126:
	jmp	.LBB1578_581
.LBB1578_487:
.Ltmp18127:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1578_488:
.Ltmp17983:
	movq	%rax, %rbx
.Ltmp17984:
	.cfi_escape 0x2e, 0x00
	leaq	.Lanon.4895d20928f7255621bf668e37519d0a.0(%rip), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::set::HashSet<purrdf_sparql_algebra::ast::Variable, purrdf_hash::fixed::FixedState>>
.Ltmp17985:
	jmp	.LBB1578_563
.LBB1578_489:
.Ltmp18063:
	movq	%rax, %rbx
	testq	%r14, %r14
	je	.LBB1578_550
	.cfi_escape 0x2e, 0x00
	movq	32(%rsp), %rdi
	movl	$1, %edx
	movq	%r14, %rsi
	jmp	.LBB1578_549
.LBB1578_491:
.Ltmp18156:
	movq	%rax, %rbx
.Ltmp18157:
	.cfi_escape 0x2e, 0x00
	leaq	3616(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governed::RelationIdentity>
.Ltmp18158:
	jmp	.LBB1578_546
.LBB1578_492:
.Ltmp18132:
	movq	%rax, %rbx
.Ltmp18133:
	.cfi_escape 0x2e, 0x00
	leaq	640(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::bgp::PlanSurvey>
.Ltmp18134:
	jmp	.LBB1578_582
.LBB1578_493:
.Ltmp18159:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1578_494:
.Ltmp17919:
	movq	%rax, %rbx
.LBB1578_495:
	xorl	%r12d, %r12d
	jmp	.LBB1578_555
.LBB1578_496:
.Ltmp18058:
	movq	%rax, %rbx
	jmp	.LBB1578_550
.LBB1578_497:
.Ltmp18210:
	movq	%rax, %rbx
.LBB1578_498:
.Ltmp18211:
	.cfi_escape 0x2e, 0x00
	leaq	5536(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::eval::EvalCtx>
.Ltmp18212:
	testb	%bpl, %bpl
	jne	.LBB1578_582
	jmp	.LBB1578_583
.LBB1578_500:
.Ltmp18252:
	movq	%rax, %rbx
	jmp	.LBB1578_525
.LBB1578_501:
.Ltmp17924:
	jmp	.LBB1578_562
.LBB1578_502:
.Ltmp17970:
	movq	%rax, %rbx
.Ltmp17971:
	.cfi_escape 0x2e, 0x00
	leaq	640(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::bgp::positive::PositivePlan>
.Ltmp17972:
	jmp	.LBB1578_563
.LBB1578_503:
.Ltmp17958:
	jmp	.LBB1578_562
.LBB1578_504:
.Ltmp17878:
	lock		decq	(%r12)
	movq	%rax, %rbx
	jne	.LBB1578_527
	#MEMBARRIER
.Ltmp17879:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	528(%rsp), %rdi
	callq	*%rax
.Ltmp17880:
	jmp	.LBB1578_527
.LBB1578_506:
.Ltmp17881:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1578_507:
.Ltmp18243:
	movq	%rax, %rbx
	jmp	.LBB1578_525
.LBB1578_508:
.Ltmp18246:
	movq	%rax, %rbx
	movq	2160(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1578_525
	lock		decq	(%rax)
	jne	.LBB1578_525
	leaq	2160(%rsp), %rdi
	#MEMBARRIER
.Ltmp18247:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn purrdf_sparql_eval::user_fn::UserFunctionAdmission>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp18248:
	jmp	.LBB1578_525
.LBB1578_511:
.Ltmp18249:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1578_512:
.Ltmp17908:
	movq	%rax, %rbx
.Ltmp17909:
	.cfi_escape 0x2e, 0x00
	leaq	528(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
.Ltmp17910:
	jmp	.LBB1578_554
.LBB1578_514:
.Ltmp18068:
	movq	%rax, %rbx
	jmp	.LBB1578_576
.LBB1578_515:
.Ltmp18084:
	movq	%rax, %rbx
	jmp	.LBB1578_578
.LBB1578_516:
.Ltmp17939:
	movq	%rax, %rbx
.Ltmp17940:
	.cfi_escape 0x2e, 0x00
	leaq	5592(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::set::HashSet<purrdf_sparql_algebra::ast::Variable, purrdf_hash::fixed::FixedState>>
.Ltmp17941:
	jmp	.LBB1578_563
.LBB1578_517:
.Ltmp18237:
	movq	%rax, %rbx
	movq	656(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1578_525
	lock		decq	(%rax)
	jne	.LBB1578_525
	leaq	656(%rsp), %rdi
	#MEMBARRIER
.Ltmp18238:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<dyn purrdf_sparql_eval::user_fn::UserFunctionAdmission>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp18239:
	jmp	.LBB1578_525
.LBB1578_520:
.Ltmp18240:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1578_521:
.Ltmp18087:
	jmp	.LBB1578_533
.LBB1578_522:
.Ltmp17901:
	jmp	.LBB1578_553
.LBB1578_523:
.Ltmp18215:
	movq	%rax, %rbx
	jmp	.LBB1578_582
.LBB1578_524:
.Ltmp18232:
	movq	%rax, %rbx
.Ltmp18233:
	.cfi_escape 0x2e, 0x00
	leaq	640(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::user_fn::UserFunctionRefusal>
.Ltmp18234:
.LBB1578_525:
.Ltmp18253:
	.cfi_escape 0x2e, 0x00
	leaq	5168(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::result::Result<purrdf_sparql_eval::governed::GovernedOutcome, purrdf_core::diagnostic::RdfDiagnostic>>
.Ltmp18254:
	jmp	.LBB1578_583
.LBB1578_526:
.Ltmp17884:
	movq	%rax, %rbx
.LBB1578_527:
.Ltmp17885:
	.cfi_escape 0x2e, 0x00
	leaq	3616(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::set::HashSet<purrdf_sparql_algebra::ast::Variable, purrdf_hash::fixed::FixedState>>
.Ltmp17886:
	jmp	.LBB1578_563
.LBB1578_528:
.Ltmp17887:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1578_529:
.Ltmp18055:
	jmp	.LBB1578_536
.LBB1578_530:
.Ltmp18075:
	jmp	.LBB1578_533
.LBB1578_531:
.Ltmp18255:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1578_532:
.Ltmp18090:
.LBB1578_533:
	movq	%rax, %rbx
	jmp	.LBB1578_581
.LBB1578_534:
.Ltmp17932:
	movq	%rax, %rbx
	jmp	.LBB1578_556
.LBB1578_535:
.Ltmp18052:
.LBB1578_536:
	movq	%rax, %rbx
	jmp	.LBB1578_573
.LBB1578_537:
.Ltmp18002:
.LBB1578_538:
	movq	%rax, %rbx
.Ltmp18006:
	.cfi_escape 0x2e, 0x00
	leaq	640(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::ir::term::TermValue>
.Ltmp18007:
	jmp	.LBB1578_563
.LBB1578_539:
.Ltmp17944:
	jmp	.LBB1578_562
.LBB1578_540:
.Ltmp17953:
	movq	%rax, %rbx
.Ltmp17954:
	.cfi_escape 0x2e, 0x00
	leaq	2144(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::set::HashSet<purrdf_sparql_algebra::ast::Variable, purrdf_hash::fixed::FixedState>>
.Ltmp17955:
	jmp	.LBB1578_563
.LBB1578_541:
.Ltmp18046:
	jmp	.LBB1578_562
.LBB1578_542:
.Ltmp17961:
	jmp	.LBB1578_562
.LBB1578_543:
.Ltmp18151:
	movq	%rax, %rbx
	.cfi_escape 0x2e, 0x00
	leaq	5536(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::string::String>>
.LBB1578_544:
	movq	7984(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB1578_546
	movq	7992(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$1, %edx
	callq	*%rax
.LBB1578_546:
	.cfi_escape 0x2e, 0x00
	leaq	640(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::governor::GovernorEvidence>
	jmp	.LBB1578_582
.LBB1578_547:
.Ltmp18078:
	movq	3616(%rsp), %rsi
	movq	%rax, %rbx
	testq	%rsi, %rsi
	je	.LBB1578_550
	movq	3624(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
.LBB1578_549:
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1578_550:
.Ltmp18079:
	.cfi_escape 0x2e, 0x00
	leaq	640(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::error::EvalError>
.Ltmp18080:
	jmp	.LBB1578_575
.LBB1578_551:
.Ltmp18081:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1578_552:
.Ltmp17927:
.LBB1578_553:
	movq	%rax, %rbx
.LBB1578_554:
	movb	$1, %r12b
.LBB1578_555:
.Ltmp17928:
	.cfi_escape 0x2e, 0x00
	leaq	640(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::bgp::positive::PositivePlan>
.Ltmp17929:
.LBB1578_556:
.Ltmp17933:
	.cfi_escape 0x2e, 0x00
	leaq	5536(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
.Ltmp17934:
	testb	%r12b, %r12b
	je	.LBB1578_563
.Ltmp17935:
	.cfi_escape 0x2e, 0x00
	leaq	5592(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::set::HashSet<purrdf_sparql_algebra::ast::Variable, purrdf_hash::fixed::FixedState>>
.Ltmp17936:
	jmp	.LBB1578_563
.LBB1578_559:
.Ltmp18008:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1578_560:
.Ltmp18162:
	movq	%rax, %rbx
	jmp	.LBB1578_575
.LBB1578_561:
.Ltmp18037:
.LBB1578_562:
	movq	%rax, %rbx
.LBB1578_563:
	movq	72(%rsp), %r14
	testq	%r14, %r14
	je	.LBB1578_573
	movq	88(%rsp), %r15
	testq	%r15, %r15
	je	.LBB1578_571
	movq	64(%rsp), %r12
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	vpcmpltb	(%r12), %xmm0, %k0
	leaq	16(%r12), %r13
	kmovd	%k0, %ebp
	.p2align	4
.LBB1578_566:
	testw	%bp, %bp
	jne	.LBB1578_569
	.p2align	4
.LBB1578_567:
	vpcmpltb	(%r13), %xmm0, %k0
	addq	$-1664, %r12
	addq	$16, %r13
	kortestw	%k0, %k0
	je	.LBB1578_567
	kmovd	%k0, %ebp
.LBB1578_569:
	xorl	%eax, %eax
	tzcntl	%ebp, %eax
	negq	%rax
	imulq	$104, %rax, %rax
	leaq	-96(%r12,%rax), %rdi
.Ltmp18047:
	.cfi_escape 0x2e, 0x00
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::bgp::SeedEstimate>
.Ltmp18048:
	blsrl	%ebp, %ebp
	decq	%r15
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	jne	.LBB1578_566
.LBB1578_571:
	imulq	$104, %r14, %rax
	andq	$-16, %rax
	addq	%rax, %r14
	addq	$129, %r14
	je	.LBB1578_573
	movq	64(%rsp), %rdi
	subq	%rax, %rdi
	addq	$-112, %rdi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$16, %edx
	movq	%r14, %rsi
	callq	*%rax
.LBB1578_573:
	movq	8(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1578_575
	movq	16(%rsp), %rdi
	shlq	$3, %rax
	leaq	(%rax,%rax,4), %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$8, %edx
	callq	*%rax
.LBB1578_575:
.Ltmp18163:
	.cfi_escape 0x2e, 0x00
	leaq	200(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::bgp::PlanSurvey>
.Ltmp18164:
.LBB1578_576:
	movq	120(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB1578_578
	#MEMBARRIER
.Ltmp18165:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::plan::PlanShape>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	120(%rsp), %rdi
	callq	*%rax
.Ltmp18166:
.LBB1578_578:
	movq	176(%rsp), %rsi
	testq	%rsi, %rsi
	jle	.LBB1578_580
	movq	184(%rsp), %rdi
	shlq	$2, %rsi
	.cfi_escape 0x2e, 0x00
	movq	__rustc::__rust_dealloc@GOTPCREL(%rip), %rax
	movl	$4, %edx
	callq	*%rax
.LBB1578_580:
	cmpq	$0, 144(%rsp)
	jne	.LBB1578_584
.LBB1578_581:
.Ltmp18173:
	.cfi_escape 0x2e, 0x00
	leaq	584(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::user_fn::RefusalPublication>
.Ltmp18174:
.LBB1578_582:
.Ltmp18216:
	.cfi_escape 0x2e, 0x00
	leaq	448(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governed::RelationIdentity>
.Ltmp18217:
.LBB1578_583:
	.cfi_escape 0x2e, 0x00
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB1578_584:
	movq	152(%rsp), %rcx
	testq	%rcx, %rcx
	jne	.LBB1578_586
	xorl	%ecx, %ecx
	xorl	%eax, %eax
	jmp	.LBB1578_587
.LBB1578_586:
	movq	160(%rsp), %rdx
	movq	168(%rsp), %rax
	movq	$0, 328(%rsp)
	movq	%rcx, 336(%rsp)
	movq	%rdx, 344(%rsp)
	movq	$0, 360(%rsp)
	movq	%rcx, 368(%rsp)
	movl	$1, %ecx
	movq	%rdx, 376(%rsp)
.LBB1578_587:
	movq	%rcx, 320(%rsp)
	movq	%rcx, 352(%rsp)
	movq	%rax, 384(%rsp)
.Ltmp18167:
	.cfi_escape 0x2e, 0x00
	leaq	616(%rsp), %rdi
	leaq	320(%rsp), %rsi
	callq	<alloc::collections::btree::map::IntoIter<purrdf_core::ir::term::BlankScope, alloc::collections::btree::set_val::SetValZST>>::dying_next
.Ltmp18168:
	cmpq	$0, 616(%rsp)
	je	.LBB1578_581
	leaq	616(%rsp), %r14
	leaq	320(%rsp), %r15
	.p2align	4
.LBB1578_590:
.Ltmp18170:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	movq	%r15, %rsi
	callq	<alloc::collections::btree::map::IntoIter<purrdf_core::ir::term::BlankScope, alloc::collections::btree::set_val::SetValZST>>::dying_next
.Ltmp18171:
	cmpq	$0, 616(%rsp)
	jne	.LBB1578_590
	jmp	.LBB1578_581
.LBB1578_592:
.Ltmp18172:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1578_593:
.Ltmp18218:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1578_594:
.Ltmp18175:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1578_595:
.Ltmp18049:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1578_596:
.Ltmp18169:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end1578:
