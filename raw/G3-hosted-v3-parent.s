_RNvMs3_NtNtCskgWIz7JHH9v_11purrdf_core2ir5pagedNtB5_12PagedDataset13from_provider:
.Lfunc_begin582:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception582
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
	subq	$1512, %rsp
	.cfi_def_cfa_offset 1568
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	%rdx, %r15
	movq	%rdi, 24(%rsp)
	movq	%rsi, 424(%rsp)
	movq	%rdx, 432(%rsp)
	movq	16(%rdx), %rax
	movq	32(%rdx), %rbp
	decq	%rax
	andq	$-16, %rax
	movq	%rsi, 176(%rsp)
	leaq	(%rsi,%rax), %r12
	addq	$16, %r12
.Ltmp10543:
	movq	%r12, %rdi
	callq	*%rbp
.Ltmp10544:
	movq	%rax, %r14
	movq	24(%r15), %rax
.Ltmp10545:
	movq	%r12, %rdi
	movq	%rax, 416(%rsp)
	callq	*%rax
.Ltmp10546:
	movq	%rax, %r13
	movq	$0, 440(%rsp)
	movq	$1, 448(%rsp)
	vxorps	%xmm0, %xmm0, %xmm0
	vmovups	%xmm0, 456(%rsp)
	movq	$8, 472(%rsp)
	movq	$0, 480(%rsp)
	vmovups	.Lanon.63e1a61e7141635ef6e103ddf8cc2986.101(%rip), %ymm0
	vmovups	%ymm0, 488(%rsp)
	movl	$3, 552(%rsp)
	imulq	$448, %rax, %rbx
	movabsq	$20587884010836553, %rax
	cmpq	%rax, %r13
	jbe	.LBB1634_5
	xorl	%r14d, %r14d
.LBB1634_4:
.Ltmp10631:
	movq	%r14, %rdi
	movq	%rbx, %rsi
	vzeroupper
	callq	*_RNvNtCsdf08ABbzq28_5alloc7raw_vec12handle_error@GOTPCREL(%rip)
.Ltmp10632:
	jmp	.LBB1634_138
.LBB1634_5:
	movq	%r12, 168(%rsp)
	movq	%r14, 64(%rsp)
	testq	%rbx, %rbx
	je	.LBB1634_8
	vzeroupper
	callq	*_RNvCsjMmYDGKcCRn_7___rustc35___rust_no_alloc_shim_is_unstable_v2@GOTPCREL(%rip)
	movl	$8, %r14d
	movl	$8, %esi
	movq	%rbx, %rdi
	callq	*_RNvCsjMmYDGKcCRn_7___rustc12___rust_alloc@GOTPCREL(%rip)
	testq	%rax, %rax
	je	.LBB1634_4
	movq	%rax, %r12
	movq	%r13, %rax
	vmovups	.Lanon.63e1a61e7141635ef6e103ddf8cc2986.101(%rip), %ymm0
	jmp	.LBB1634_9
.LBB1634_8:
	movl	$8, %r12d
	xorl	%eax, %eax
.LBB1634_9:
	movq	%rbp, 408(%rsp)
	movq	%rax, 208(%rsp)
	movq	%r12, 216(%rsp)
	vmovups	%ymm0, 288(%rsp)
	vmovups	%ymm0, 320(%rsp)
	movq	$0, 224(%rsp)
	vmovups	%ymm0, 352(%rsp)
	movq	%r13, 280(%rsp)
	testq	%r13, %r13
	movq	%r15, 392(%rsp)
	je	.LBB1634_70
	movq	48(%r15), %rax
	xorl	%ebx, %ebx
	leaq	560(%rsp), %r14
	leaq	440(%rsp), %r15
	xorl	%ecx, %ecx
	movq	$0, 144(%rsp)
	movq	%rax, 384(%rsp)
.LBB1634_11:
	movq	%rcx, 400(%rsp)
	movq	%rbx, 192(%rsp)
.Ltmp10547:
	movq	%r14, %rdi
	movq	168(%rsp), %rsi
	movq	%rbx, %rdx
	vzeroupper
	callq	*%rax
.Ltmp10548:
	leaq	568(%rsp), %rcx
	vmovups	(%rcx), %xmm0
	vmovaps	%xmm0, 1104(%rsp)
	movq	560(%rsp), %rax
	movq	16(%rcx), %rcx
	movq	%rcx, 1120(%rsp)
	cmpq	$-1, %rax
	jne	.LBB1634_115
	movq	%rbx, 16(%rsp)
	vmovaps	1104(%rsp), %xmm0
	vmovaps	%xmm0, 256(%rsp)
	movq	1120(%rsp), %rax
	movq	%rax, 272(%rsp)
	movq	264(%rsp), %rbx
	cmpq	64(%rsp), %rbx
	jne	.LBB1634_116
	movq	256(%rsp), %rax
	movq	%rax, 88(%rsp)
	movq	%rax, 8(%rsp)
	leaq	16(%rax), %rsi
.Ltmp10557:
	movq	%r14, %rdi
	movq	%rsi, 184(%rsp)
	movq	%r15, %rdx
	callq	_RNvMNtNtNtCskgWIz7JHH9v_11purrdf_core2ir5paged11translationNtB2_15PageTranslation9try_build
.Ltmp10558:
	movq	560(%rsp), %rbx
	movq	568(%rsp), %r14
	cmpq	$-1, %rbx
	je	.LBB1634_119
	leaq	568(%rsp), %rcx
	movq	24(%rcx), %rax
	movq	%rax, 1056(%rsp)
	vmovups	8(%rcx), %xmm0
	vmovaps	%xmm0, 1040(%rsp)
	movq	600(%rsp), %r15
	vmovups	40(%rcx), %xmm0
	leaq	1120(%rsp), %r13
	vmovups	%xmm0, 32(%r13)
	movl	$336, %edx
	leaq	1168(%rsp), %rdi
	leaq	624(%rsp), %rsi
	callq	*memcpy@GOTPCREL(%rip)
	movq	1056(%rsp), %rax
	movq	%rax, 1024(%rsp)
	vmovaps	1040(%rsp), %xmm0
	vmovaps	%xmm0, 1008(%rsp)
	movq	%rax, 16(%r13)
	vmovups	%xmm0, (%r13)
	movq	%rbx, 1104(%rsp)
	movq	%r14, 1112(%rsp)
	movq	%r15, 1144(%rsp)
	movq	8(%rsp), %rax
	movzwl	440(%rax), %ecx
	movq	%rcx, 80(%rsp)
	movzbl	442(%rax), %ecx
	movl	%ecx, 204(%rsp)
	movl	436(%rax), %ecx
	movq	%rcx, 160(%rsp)
	movq	80(%rax), %rcx
	movq	1472(%rsp), %r15
	movq	1480(%rsp), %rsi
	movq	%rcx, 152(%rsp)
	testq	%rcx, %rcx
	movq	%r12, 48(%rsp)
	movq	%rsi, 32(%rsp)
	je	.LBB1634_28
	movq	72(%rax), %rbx
	movq	152(%rsp), %rax
	shlq	$4, %rax
	addq	%rbx, %rax
	movq	%rax, 40(%rsp)
	.p2align	4
.LBB1634_18:
	movl	(%rbx), %edi
	decl	%edi
	cmpq	%rdi, %rsi
	jbe	.LBB1634_137
	movl	4(%rbx), %eax
	decl	%eax
	cmpq	%rax, %rsi
	jbe	.LBB1634_128
	movl	8(%rbx), %edx
	decl	%edx
	cmpq	%rdx, %rsi
	jbe	.LBB1634_129
	movl	12(%rbx), %ecx
	movq	(%r15,%rdi,8), %r13
	movq	(%r15,%rax,8), %rbp
	movq	(%r15,%rdx,8), %r12
	testl	%ecx, %ecx
	je	.LBB1634_24
	decl	%ecx
	cmpq	%rcx, %rsi
	jbe	.LBB1634_136
	movq	(%r15,%rcx,8), %r14
	jmp	.LBB1634_25
	.p2align	4
.LBB1634_24:
	xorl	%r14d, %r14d
.LBB1634_25:
	movq	%r13, 560(%rsp)
	movq	%rbp, 568(%rsp)
	movq	%r12, 576(%rsp)
	movq	%r14, 584(%rsp)
.Ltmp10560:
	leaq	288(%rsp), %rdi
	leaq	560(%rsp), %rsi
	movq	16(%rsp), %rdx
	callq	_RNvMs1_NtCskjtayK3PjuD_9hashbrown3mapINtB5_7HashMapTNtNtNtCskgWIz7JHH9v_11purrdf_core2ir6global12GlobalTermIdBO_BO_INtNtCslK5Drzrx8K4_4core6option6OptionBO_EENtNtNtBS_5paged8provider6PageIdNtNtCsbnavtP7nVTc_11purrdf_hash5fixed10FixedStateE6insertBU_
.Ltmp10561:
	cmpq	$1, %rax
	je	.LBB1634_78
	addq	$16, %rbx
	cmpq	40(%rsp), %rbx
	movq	8(%rsp), %rax
	movq	32(%rsp), %rsi
	jne	.LBB1634_18
.LBB1634_28:
	cmpq	$0, 96(%rax)
	movq	184(%rsp), %rbx
	je	.LBB1634_31
.Ltmp10565:
	movl	$50, %edx
	movq	%rbx, %rdi
	leaq	.Lanon.63e1a61e7141635ef6e103ddf8cc2986.229(%rip), %rsi
	callq	*_RNvMsb_NtNtCskgWIz7JHH9v_11purrdf_core2ir7datasetNtB5_10RdfDataset14term_id_by_iri@GOTPCREL(%rip)
.Ltmp10566:
	testl	%eax, %eax
	je	.LBB1634_125
.LBB1634_31:
.Ltmp10569:
	movl	$50, %edx
	movq	%rbx, %rdi
	leaq	.Lanon.63e1a61e7141635ef6e103ddf8cc2986.229(%rip), %rsi
	callq	*_RNvMsb_NtNtCskgWIz7JHH9v_11purrdf_core2ir7datasetNtB5_10RdfDataset14term_id_by_iri@GOTPCREL(%rip)
.Ltmp10570:
	movq	16(%rsp), %rcx
	incq	%rcx
	movq	%rcx, 72(%rsp)
	movl	204(%rsp), %edx
	shll	$16, %edx
	movq	80(%rsp), %rcx
	orl	%edx, %ecx
	shlq	$32, %rcx
	addq	%rcx, 160(%rsp)
	xorl	%ebx, %ebx
	movq	%rcx, 80(%rsp)
	movq	%r15, 40(%rsp)
	.p2align	4
.LBB1634_33:
	testq	%rbx, %rbx
	movq	32(%rsp), %rsi
	je	.LBB1634_37
	cmpq	184(%rsp), %rbx
	je	.LBB1634_37
	movl	(%rbx), %edi
	testl	%edi, %edi
	je	.LBB1634_37
	movl	%eax, %r9d
	movq	80(%rsp), %r8
	jmp	.LBB1634_41
	.p2align	4
.LBB1634_37:
	testl	%eax, %eax
	movq	8(%rsp), %rdx
	je	.LBB1634_50
	movq	96(%rdx), %rcx
	testq	%rcx, %rcx
	je	.LBB1634_50
	movq	88(%rdx), %rbx
	movl	(%rbx), %edi
	testl	%edi, %edi
	je	.LBB1634_50
	leaq	(%rcx,%rcx,2), %rcx
	leaq	(%rbx,%rcx,4), %rcx
	movq	%rcx, 184(%rsp)
	xorl	%r9d, %r9d
	movl	%eax, %r8d
.LBB1634_41:
	decl	%edi
	cmpq	%rdi, %rsi
	jbe	.LBB1634_137
	leal	-1(%r8), %ecx
	cmpq	%rcx, %rsi
	jbe	.LBB1634_136
	movq	4(%rbx), %rax
	movl	$4294967295, %edx
	addl	%eax, %edx
	cmpq	%rdx, %rsi
	jbe	.LBB1634_129
	movq	(%r15,%rdi,8), %r12
	movq	(%r15,%rcx,8), %r13
	movq	(%r15,%rdx,8), %rbp
	shrq	$32, %rax
	movq	%r8, 80(%rsp)
	je	.LBB1634_47
	movl	$4294967295, %ecx
	addl	%ecx, %eax
	cmpq	%rax, %rsi
	jbe	.LBB1634_128
	movl	%r9d, %r15d
	movq	40(%rsp), %rcx
	movq	(%rcx,%rax,8), %r14
	jmp	.LBB1634_48
	.p2align	4
.LBB1634_47:
	movl	%r9d, %r15d
	xorl	%r14d, %r14d
.LBB1634_48:
	movq	%r12, 560(%rsp)
	movq	%r13, 568(%rsp)
	movq	%rbp, 576(%rsp)
	movq	%r14, 584(%rsp)
.Ltmp10571:
	leaq	320(%rsp), %rdi
	leaq	560(%rsp), %rsi
	movq	16(%rsp), %rdx
	callq	_RNvMs1_NtCskjtayK3PjuD_9hashbrown3mapINtB5_7HashMapTNtNtNtCskgWIz7JHH9v_11purrdf_core2ir6global12GlobalTermIdBO_BO_INtNtCslK5Drzrx8K4_4core6option6OptionBO_EENtNtNtBS_5paged8provider6PageIdNtNtCsbnavtP7nVTc_11purrdf_hash5fixed10FixedStateE6insertBU_
.Ltmp10572:
	movq	%rax, %rcx
	addq	$12, %rbx
	movl	%r15d, %eax
	cmpq	$1, %rcx
	movq	40(%rsp), %r15
	jne	.LBB1634_33
	jmp	.LBB1634_80
.LBB1634_50:
	movq	112(%rdx), %rax
	testq	%rax, %rax
	je	.LBB1634_62
	movq	%rax, %rcx
	movq	8(%rsp), %rax
	movq	104(%rax), %r12
	shlq	$4, %rcx
	addq	%r12, %rcx
	movq	%rcx, 40(%rsp)
	.p2align	4
.LBB1634_52:
	movl	(%r12), %edi
	decl	%edi
	cmpq	%rdi, %rsi
	jbe	.LBB1634_137
	movl	4(%r12), %eax
	decl	%eax
	cmpq	%rax, %rsi
	jbe	.LBB1634_128
	movl	8(%r12), %edx
	decl	%edx
	cmpq	%rdx, %rsi
	jbe	.LBB1634_129
	movl	12(%r12), %ecx
	movq	(%r15,%rdi,8), %r13
	movq	(%r15,%rax,8), %rbp
	movq	(%r15,%rdx,8), %rbx
	testl	%ecx, %ecx
	je	.LBB1634_58
	decl	%ecx
	cmpq	%rcx, %rsi
	jbe	.LBB1634_136
	movq	(%r15,%rcx,8), %r14
	jmp	.LBB1634_59
	.p2align	4
.LBB1634_58:
	xorl	%r14d, %r14d
.LBB1634_59:
	movq	%r13, 560(%rsp)
	movq	%rbp, 568(%rsp)
	movq	%rbx, 576(%rsp)
	movq	%r14, 584(%rsp)
.Ltmp10578:
	leaq	352(%rsp), %rdi
	leaq	560(%rsp), %rsi
	movq	16(%rsp), %rdx
	callq	_RNvMs1_NtCskjtayK3PjuD_9hashbrown3mapINtB5_7HashMapTNtNtNtCskgWIz7JHH9v_11purrdf_core2ir6global12GlobalTermIdBO_BO_INtNtCslK5Drzrx8K4_4core6option6OptionBO_EENtNtNtBS_5paged8provider6PageIdNtNtCsbnavtP7nVTc_11purrdf_hash5fixed10FixedStateE6insertBU_
.Ltmp10579:
	cmpq	$1, %rax
	je	.LBB1634_81
	addq	$16, %r12
	cmpq	40(%rsp), %r12
	movq	32(%rsp), %rsi
	jne	.LBB1634_52
.LBB1634_62:
	movq	160(%rsp), %rdx
	movq	%rdx, %rbp
	movabsq	$-281474976710656, %rax
	andq	%rax, %rbp
	movq	400(%rsp), %rcx
	btq	$48, %rcx
	movabsq	$281474976710656, %rax
	cmovbq	%rax, %rbp
	movq	%rdx, %r14
	movabsq	$280375465082880, %rax
	andq	%rax, %r14
	btq	$40, %rcx
	movabsq	$1099511627776, %rax
	cmovbq	%rax, %r14
	movq	%rdx, %rdi
	movabsq	$1095216660480, %rax
	andq	%rax, %rdi
	btq	$32, %rcx
	movabsq	$4294967296, %rax
	cmovbq	%rax, %rdi
	movq	%rdi, 32(%rsp)
	movl	%edx, %r12d
	andl	$-16777216, %r12d
	testl	$16777216, %ecx
	movl	$16777216, %eax
	cmovneq	%rax, %r12
	movl	%edx, %ebx
	andl	$16711680, %ebx
	testl	$65536, %ecx
	movl	$65536, %eax
	cmovneq	%rax, %rbx
	movl	%edx, %r13d
	andl	$65280, %r13d
	testl	$256, %ecx
	movl	$256, %eax
	cmovneq	%rax, %r13
	testb	$1, %cl
	movzbl	%dl, %ecx
	movl	$1, %eax
	cmovneq	%rax, %rcx
	movq	%rcx, 40(%rsp)
	movq	152(%rsp), %rax
	addq	%rax, 144(%rsp)
	jb	.LBB1634_126
	movabsq	$9223372036854775793, %rax
	addq	$15, %rax
	movq	%rax, 560(%rsp)
.Ltmp10584:
	leaq	560(%rsp), %rdi
	callq	_RINvNtCslK5Drzrx8K4_4core3ptr9drop_glueNtNtNtCskgWIz7JHH9v_11purrdf_core2ir5paged16PagedFreezeErrorEBH_
.Ltmp10585:
	movl	$400, %edx
	leaq	560(%rsp), %rdi
	leaq	1104(%rsp), %rsi
	callq	*memcpy@GOTPCREL(%rip)
	movq	272(%rsp), %rax
	movq	16(%rsp), %rdx
	movq	%rdx, 960(%rsp)
	movl	$3, 976(%rsp)
	movq	160(%rsp), %rsi
	movq	%rsi, %rcx
	shrq	$48, %rcx
	movb	%cl, 1006(%rsp)
	movq	%rsi, %rcx
	shrq	$32, %rcx
	movw	%cx, 1004(%rsp)
	movl	%esi, 1000(%rsp)
	movq	152(%rsp), %rcx
	movq	%rcx, 984(%rsp)
	movq	%rax, 992(%rsp)
	cmpq	208(%rsp), %rdx
	jne	.LBB1634_67
.Ltmp10587:
	leaq	208(%rsp), %rdi
	callq	*_RNvMs7_NtCsdf08ABbzq28_5alloc7raw_vecINtB5_6RawVecNtNtNtCskgWIz7JHH9v_11purrdf_core2ir5paged8PageSlotE8grow_oneBS_@GOTPCREL(%rip)
.Ltmp10588:
	movq	216(%rsp), %rax
	movq	%rax, 48(%rsp)
.LBB1634_67:
	imulq	$448, 16(%rsp), %rdi
	addq	48(%rsp), %rdi
	movl	$448, %edx
	leaq	560(%rsp), %rsi
	callq	*memcpy@GOTPCREL(%rip)
	movq	72(%rsp), %rax
	movq	%rax, 224(%rsp)
	movq	8(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB1634_69
	#MEMBARRIER
.Ltmp10593:
	leaq	88(%rsp), %rdi
	callq	*_RNvMsn_NtNtCsdf08ABbzq28_5alloc3rcs3arcINtB5_3ArcNtNtNtCskgWIz7JHH9v_11purrdf_core2ir7dataset10RdfDatasetE9drop_slowBR_@GOTPCREL(%rip)
.Ltmp10594:
.LBB1634_69:
	orq	%r14, %rbp
	addq	32(%rsp), %rbp
	orq	%r12, %rbp
	orq	%rbx, %rbp
	orq	%r13, %rbp
	orq	40(%rsp), %rbp
	movq	72(%rsp), %rax
	movq	%rax, %rbx
	movq	%rbp, %rcx
	cmpq	280(%rsp), %rax
	movq	48(%rsp), %r12
	movq	384(%rsp), %rax
	leaq	560(%rsp), %r14
	leaq	440(%rsp), %r15
	jne	.LBB1634_11
	jmp	.LBB1634_71
.LBB1634_70:
	movq	$0, 72(%rsp)
	movq	$0, 144(%rsp)
	xorl	%ebp, %ebp
.LBB1634_71:
	movb	$1, %r13b
.Ltmp10608:
	movq	168(%rsp), %rdi
	vzeroupper
	callq	*408(%rsp)
.Ltmp10609:
	movabsq	$9223372036854775793, %rbx
	cmpq	64(%rsp), %rax
	jne	.LBB1634_79
.Ltmp10610:
	movq	168(%rsp), %rdi
	callq	*416(%rsp)
.Ltmp10611:
	movq	280(%rsp), %rcx
	cmpq	%rcx, %rax
	jne	.LBB1634_88
	xorl	%r13d, %r13d
.Ltmp10621:
	leaq	208(%rsp), %rdi
	callq	_RNvMs_NtCsdf08ABbzq28_5alloc3vecINtB4_3VecNtNtNtCskgWIz7JHH9v_11purrdf_core2ir5paged8PageSlotE16into_boxed_sliceBK_
.Ltmp10622:
.Ltmp10624:
	movq	%rax, %r15
	movq	%rdx, %r14
	leaq	560(%rsp), %rdi
	movq	%rax, %rsi
	callq	_RNvMs_NtNtNtCskgWIz7JHH9v_11purrdf_core2ir5paged11graph_indexNtB4_14GraphPageIndex6derive
.Ltmp10625:
	vmovups	440(%rsp), %ymm0
	vmovups	472(%rsp), %ymm1
	vmovups	504(%rsp), %ymm2
	vmovups	528(%rsp), %ymm3
	movq	24(%rsp), %rcx
	vmovups	%ymm3, 88(%rcx)
	vmovups	%ymm2, 64(%rcx)
	vmovups	%ymm1, 32(%rcx)
	vmovups	%ymm0, (%rcx)
	vmovups	560(%rsp), %ymm0
	vmovups	592(%rsp), %ymm1
	vmovups	624(%rsp), %ymm2
	vmovups	656(%rsp), %ymm3
	vmovups	%ymm0, 152(%rcx)
	vmovups	%ymm1, 184(%rcx)
	vmovups	%ymm2, 216(%rcx)
	vmovups	%ymm3, 248(%rcx)
	vmovups	688(%rsp), %ymm0
	vmovups	%ymm0, 280(%rcx)
	vmovups	720(%rsp), %ymm0
	vmovups	%ymm0, 312(%rcx)
	movq	%r15, 120(%rcx)
	movq	%r14, 128(%rcx)
	movq	176(%rsp), %rax
	movq	%rax, 136(%rcx)
	movq	392(%rsp), %rax
	movq	%rax, 144(%rcx)
	movq	64(%rsp), %rax
	movq	%rax, 344(%rcx)
	movq	144(%rsp), %rax
	movq	%rax, 352(%rcx)
	movq	%rbp, %rax
	shrq	$48, %rax
	movb	%al, 366(%rcx)
	movq	%rbp, %rax
	shrq	$32, %rax
	movw	%ax, 364(%rcx)
	movl	%ebp, 360(%rcx)
	movq	352(%rsp), %rdi
	movq	360(%rsp), %rsi
	vzeroupper
	callq	_RINvNtCslK5Drzrx8K4_4core3ptr9drop_glueINtNtNtNtCskEFv3nc5qE4_3std11collections4hash3map7HashMapNtNtNtCskgWIz7JHH9v_11purrdf_core2ir7mutable7QuadKeyyNtNtCsbnavtP7nVTc_11purrdf_hash5fixed10FixedStateEEB1C_
	movq	320(%rsp), %rdi
	movq	328(%rsp), %rsi
	callq	_RINvNtCslK5Drzrx8K4_4core3ptr9drop_glueINtNtNtNtCskEFv3nc5qE4_3std11collections4hash3map7HashMapNtNtNtCskgWIz7JHH9v_11purrdf_core2ir7mutable7QuadKeyyNtNtCsbnavtP7nVTc_11purrdf_hash5fixed10FixedStateEEB1C_
	movq	288(%rsp), %rdi
	movq	296(%rsp), %rsi
	callq	_RINvNtCslK5Drzrx8K4_4core3ptr9drop_glueINtNtNtNtCskEFv3nc5qE4_3std11collections4hash3map7HashMapNtNtNtCskgWIz7JHH9v_11purrdf_core2ir7mutable7QuadKeyyNtNtCsbnavtP7nVTc_11purrdf_hash5fixed10FixedStateEEB1C_
	jmp	.LBB1634_114
.LBB1634_78:
	movq	%rdx, %r8
	movq	%r13, 96(%rsp)
	movq	%rbp, 104(%rsp)
	movq	%r12, 112(%rsp)
	movq	%r14, 120(%rsp)
	movq	%rdx, 128(%rsp)
	movb	$0, 136(%rsp)
.Ltmp10563:
	leaq	560(%rsp), %rdi
	leaq	192(%rsp), %rsi
	leaq	440(%rsp), %rdx
	leaq	96(%rsp), %rcx
	xorl	%r9d, %r9d
	callq	_RNCNvMs3_NtNtCskgWIz7JHH9v_11purrdf_core2ir5pagedNtB7_12PagedDataset13from_providers0_0Bb_
.Ltmp10564:
	jmp	.LBB1634_82
.LBB1634_79:
	addq	$16, %rbx
	jmp	.LBB1634_89
.LBB1634_80:
	movq	%rdx, %r8
	movq	%r12, 96(%rsp)
	movq	%r13, 104(%rsp)
	movq	%rbp, 112(%rsp)
	movq	%r14, 120(%rsp)
	movq	%rdx, 128(%rsp)
	movb	$1, 136(%rsp)
.Ltmp10574:
	leaq	560(%rsp), %rdi
	leaq	192(%rsp), %rsi
	leaq	440(%rsp), %rdx
	leaq	96(%rsp), %rcx
	movl	$1, %r9d
	callq	_RNCNvMs3_NtNtCskgWIz7JHH9v_11purrdf_core2ir5pagedNtB7_12PagedDataset13from_providers0_0Bb_
.Ltmp10575:
	jmp	.LBB1634_82
.LBB1634_81:
	movq	%rdx, %r8
	movq	%r13, 96(%rsp)
	movq	%rbp, 104(%rsp)
	movq	%rbx, 112(%rsp)
	movq	%r14, 120(%rsp)
	movq	%rdx, 128(%rsp)
	movb	$2, 136(%rsp)
.Ltmp10581:
	leaq	560(%rsp), %rdi
	leaq	192(%rsp), %rsi
	leaq	440(%rsp), %rdx
	leaq	96(%rsp), %rcx
	movl	$2, %r9d
	callq	_RNCNvMs3_NtNtCskgWIz7JHH9v_11purrdf_core2ir5pagedNtB7_12PagedDataset13from_providers0_0Bb_
.Ltmp10582:
.LBB1634_82:
	vmovups	584(%rsp), %ymm0
	movq	24(%rsp), %rax
	vmovups	%ymm0, 32(%rax)
	vmovups	560(%rsp), %ymm0
	vmovups	%ymm0, 8(%rax)
	movq	$-1, (%rax)
	movq	48(%rsp), %r12
	movq	16(%rsp), %rbx
	movq	32(%rsp), %rsi
.LBB1634_83:
	shlq	$3, %rsi
	movl	$8, %edx
	movq	%r15, %rdi
	vzeroupper
	callq	*_RNvCsjMmYDGKcCRn_7___rustc14___rust_dealloc@GOTPCREL(%rip)
.LBB1634_84:
	movq	1496(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB1634_86
	movq	1488(%rsp), %rdi
	shlq	$4, %rsi
	movl	$8, %edx
	callq	*_RNvCsjMmYDGKcCRn_7___rustc14___rust_dealloc@GOTPCREL(%rip)
.LBB1634_86:
	leaq	1104(%rsp), %rdi
	callq	_RINvNtCslK5Drzrx8K4_4core3ptr9drop_glueNtNtNtNtCskgWIz7JHH9v_11purrdf_core2ir5paged7summary11PageSummaryEBJ_
	movq	8(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB1634_90
	#MEMBARRIER
.Ltmp10596:
	leaq	88(%rsp), %rdi
	callq	*_RNvMsn_NtNtCsdf08ABbzq28_5alloc3rcs3arcINtB5_3ArcNtNtNtCskgWIz7JHH9v_11purrdf_core2ir7dataset10RdfDatasetE9drop_slowBR_@GOTPCREL(%rip)
.Ltmp10597:
	jmp	.LBB1634_90
.LBB1634_88:
	addq	$17, %rbx
	movq	%rcx, 64(%rsp)
.LBB1634_89:
	movq	24(%rsp), %rcx
	movq	%rbx, 8(%rcx)
	movq	64(%rsp), %rdx
	movq	%rdx, 16(%rcx)
	movq	%rax, 24(%rcx)
	movq	$-1, (%rcx)
	movq	72(%rsp), %rbx
.LBB1634_90:
	movq	360(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB1634_93
	leaq	(,%rsi,8), %rax
	leaq	(%rax,%rax,4), %rax
	andq	$-16, %rax
	addq	%rax, %rsi
	addq	$65, %rsi
	je	.LBB1634_93
	movq	352(%rsp), %rdi
	subq	%rax, %rdi
	addq	$-48, %rdi
	movl	$16, %edx
	callq	*_RNvCsjMmYDGKcCRn_7___rustc14___rust_dealloc@GOTPCREL(%rip)
.LBB1634_93:
	movq	328(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB1634_96
	leaq	(,%rsi,8), %rax
	leaq	(%rax,%rax,4), %rax
	andq	$-16, %rax
	addq	%rax, %rsi
	addq	$65, %rsi
	je	.LBB1634_96
	movq	320(%rsp), %rdi
	subq	%rax, %rdi
	addq	$-48, %rdi
	movl	$16, %edx
	callq	*_RNvCsjMmYDGKcCRn_7___rustc14___rust_dealloc@GOTPCREL(%rip)
.LBB1634_96:
	movq	296(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB1634_99
	leaq	(,%rsi,8), %rax
	leaq	(%rax,%rax,4), %rax
	andq	$-16, %rax
	addq	%rax, %rsi
	addq	$65, %rsi
	je	.LBB1634_99
	movq	288(%rsp), %rdi
	subq	%rax, %rdi
	addq	$-48, %rdi
	movl	$16, %edx
	callq	*_RNvCsjMmYDGKcCRn_7___rustc14___rust_dealloc@GOTPCREL(%rip)
.LBB1634_99:
	testq	%rbx, %rbx
	je	.LBB1634_103
	movl	$1, %r15d
	subq	%rbx, %r15
	movq	%r12, %r14
	.p2align	4
.LBB1634_101:
.Ltmp10612:
	movq	%r14, %rdi
	callq	_RINvNtCslK5Drzrx8K4_4core3ptr9drop_glueNtNtNtCskgWIz7JHH9v_11purrdf_core2ir5paged8PageSlotEBH_
.Ltmp10613:
	incq	%r15
	addq	$448, %r14
	cmpq	$1, %r15
	jne	.LBB1634_101
.LBB1634_103:
	movq	208(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1634_105
	imulq	$448, %rax, %rsi
	movl	$8, %edx
	movq	%r12, %rdi
	callq	*_RNvCsjMmYDGKcCRn_7___rustc14___rust_dealloc@GOTPCREL(%rip)
.LBB1634_105:
	movq	440(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB1634_107
	movq	448(%rsp), %rdi
	movl	$1, %edx
	callq	*_RNvCsjMmYDGKcCRn_7___rustc14___rust_dealloc@GOTPCREL(%rip)
.LBB1634_107:
	movq	464(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1634_109
	movq	472(%rsp), %rdi
	shlq	$3, %rax
	leaq	(%rax,%rax,4), %rsi
	movl	$8, %edx
	callq	*_RNvCsjMmYDGKcCRn_7___rustc14___rust_dealloc@GOTPCREL(%rip)
.LBB1634_109:
	movq	496(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1634_111
	movq	488(%rsp), %rdi
	leaq	(,%rax,8), %rcx
	andq	$-16, %rcx
	leaq	(%rcx,%rax), %rsi
	addq	$33, %rsi
	subq	%rcx, %rdi
	addq	$-16, %rdi
	movl	$16, %edx
	callq	*_RNvCsjMmYDGKcCRn_7___rustc14___rust_dealloc@GOTPCREL(%rip)
.LBB1634_111:
	leaq	520(%rsp), %rdi
.Ltmp10618:
	callq	_RINvNtCslK5Drzrx8K4_4core3ptr9drop_glueINtNtNtCskEFv3nc5qE4_3std4sync9once_lock8OnceLockINtNtNtNtBI_11collections4hash3map7HashMapyINtNtCsdf08ABbzq28_5alloc3vec3VecNtNtNtCskgWIz7JHH9v_11purrdf_core2ir6global12GlobalTermIdENtNtCsbnavtP7nVTc_11purrdf_hash5fixed10FixedStateEEEB2I_
.Ltmp10619:
	movq	176(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB1634_114
	#MEMBARRIER
	leaq	424(%rsp), %rdi
	callq	*_RNvMsn_NtNtCsdf08ABbzq28_5alloc3rcs3arcINtB5_3ArcDNtNtCslK5Drzrx8K4_4core3any3AnyNtNtBQ_6marker4SyncNtB1j_4SendEL_E9drop_slowCskgWIz7JHH9v_11purrdf_core@GOTPCREL(%rip)
.LBB1634_114:
	movq	24(%rsp), %rax
	addq	$1512, %rsp
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
	retq
.LBB1634_115:
	.cfi_def_cfa_offset 1568
	movq	608(%rsp), %rcx
	vmovups	592(%rsp), %xmm0
	movq	24(%rsp), %rdx
	vmovups	%xmm0, 40(%rdx)
	movq	%rcx, 56(%rdx)
	movq	1120(%rsp), %rcx
	movq	%rcx, 32(%rdx)
	vmovaps	1104(%rsp), %xmm0
	vmovups	%xmm0, 16(%rdx)
	movq	%rax, 8(%rdx)
	movq	$-1, (%rdx)
	jmp	.LBB1634_90
.LBB1634_116:
	movq	64(%rsp), %r14
	movq	%r14, 232(%rsp)
	movq	%rbx, 96(%rsp)
	leaq	96(%rsp), %rax
	movq	%rax, 1104(%rsp)
	movq	_RNvXs_NtNtNtCskgWIz7JHH9v_11purrdf_core2ir5paged8providerNtB4_14PageGenerationNtNtCslK5Drzrx8K4_4core3fmt7Display3fmt@GOTPCREL(%rip), %rax
	movq	%rax, 1112(%rsp)
	leaq	232(%rsp), %rcx
	movq	%rcx, 1120(%rsp)
	movq	%rax, 1128(%rsp)
.Ltmp10550:
	leaq	.Lanon.63e1a61e7141635ef6e103ddf8cc2986.1089(%rip), %rsi
	leaq	560(%rsp), %rdi
	leaq	1104(%rsp), %rdx
	callq	*_RNvNvNtCsdf08ABbzq28_5alloc3fmt6format12format_inner@GOTPCREL(%rip)
.Ltmp10551:
	movq	16(%rsp), %rax
	movq	%rax, 608(%rsp)
	movb	$1, 584(%rsp)
	movq	%r14, 592(%rsp)
	movq	%rbx, 600(%rsp)
	movq	%rax, %rbx
	movzbl	584(%rsp), %eax
	movq	24(%rsp), %rcx
	movb	%al, 32(%rcx)
	movl	585(%rsp), %eax
	movl	%eax, 33(%rcx)
	movzwl	589(%rsp), %eax
	movw	%ax, 37(%rcx)
	movzbl	591(%rsp), %eax
	movb	%al, 39(%rcx)
	movq	592(%rsp), %rax
	movq	%rax, 40(%rcx)
	movq	600(%rsp), %rax
	movq	%rax, 48(%rcx)
	movq	608(%rsp), %rax
	vmovups	560(%rsp), %xmm0
	vmovups	%xmm0, 8(%rcx)
	movq	%rax, 56(%rcx)
	movq	576(%rsp), %rax
	movq	%rax, 24(%rcx)
	movzbl	584(%rsp), %eax
	movb	%al, 32(%rcx)
	movl	585(%rsp), %eax
	movl	%eax, 33(%rcx)
	movzwl	589(%rsp), %eax
	movw	%ax, 37(%rcx)
	movzbl	591(%rsp), %eax
	movb	%al, 39(%rcx)
	movq	$-1, (%rcx)
	movq	256(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB1634_90
	#MEMBARRIER
	movb	$1, %r13b
.Ltmp10555:
	leaq	256(%rsp), %rdi
	callq	*_RNvMsn_NtNtCsdf08ABbzq28_5alloc3rcs3arcINtB5_3ArcNtNtNtCskgWIz7JHH9v_11purrdf_core2ir7dataset10RdfDatasetE9drop_slowBR_@GOTPCREL(%rip)
.Ltmp10556:
	jmp	.LBB1634_90
.LBB1634_119:
	movq	576(%rsp), %r15
	movq	584(%rsp), %rdx
	movq	$0, 232(%rsp)
	movq	$1, 240(%rsp)
	movq	$0, 248(%rsp)
	movq	$1610612768, 112(%rsp)
	leaq	232(%rsp), %rax
	movq	%rax, 96(%rsp)
	leaq	.Lanon.63e1a61e7141635ef6e103ddf8cc2986.642(%rip), %rax
	movq	%rax, 104(%rsp)
.Ltmp10598:
	leaq	96(%rsp), %rdi
	movq	%r15, %rsi
	callq	*_RNvMsa_NtCslK5Drzrx8K4_4core3fmtNtB5_9Formatter9write_str@GOTPCREL(%rip)
.Ltmp10599:
	testb	%al, %al
	movq	16(%rsp), %rbx
	jne	.LBB1634_127
	movq	248(%rsp), %rax
	movq	%rax, 1088(%rsp)
	vmovups	232(%rsp), %xmm0
	vmovaps	%xmm0, 1072(%rsp)
	testq	%r14, %r14
	je	.LBB1634_123
	movl	$1, %edx
	movq	%r15, %rdi
	movq	%r14, %rsi
	callq	*_RNvCsjMmYDGKcCRn_7___rustc14___rust_dealloc@GOTPCREL(%rip)
.LBB1634_123:
	vmovaps	1072(%rsp), %xmm0
	vmovaps	%xmm0, 1008(%rsp)
	movq	1088(%rsp), %rax
	movq	%rax, 1024(%rsp)
	movq	24(%rsp), %rcx
	vmovups	%xmm0, 16(%rcx)
	movq	%rax, 32(%rcx)
	movabsq	$9223372036854775793, %rax
	addq	$19, %rax
	movq	%rax, 8(%rcx)
	movq	%rbx, 40(%rcx)
	movq	$-1, (%rcx)
	movq	8(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB1634_90
	#MEMBARRIER
.Ltmp10600:
	leaq	88(%rsp), %rdi
	callq	*_RNvMsn_NtNtCsdf08ABbzq28_5alloc3rcs3arcINtB5_3ArcNtNtNtCskgWIz7JHH9v_11purrdf_core2ir7dataset10RdfDatasetE9drop_slowBR_@GOTPCREL(%rip)
.Ltmp10601:
	jmp	.LBB1634_90
.LBB1634_125:
	movq	8(%rsp), %rax
	movq	96(%rax), %rax
	movq	%rax, 96(%rsp)
	leaq	96(%rsp), %rax
	movq	%rax, 560(%rsp)
	movq	_RNvXsi_NtNtNtCslK5Drzrx8K4_4core3fmt3num3impjNtB9_7Display3fmt@GOTPCREL(%rip), %rax
	movq	%rax, 568(%rsp)
.Ltmp10567:
	leaq	.Lanon.63e1a61e7141635ef6e103ddf8cc2986.1456(%rip), %rdi
	leaq	.Lanon.63e1a61e7141635ef6e103ddf8cc2986.1457(%rip), %rdx
	leaq	560(%rsp), %rsi
	callq	*_RNvNtCslK5Drzrx8K4_4core9panicking9panic_fmt@GOTPCREL(%rip)
.Ltmp10568:
	jmp	.LBB1634_138
.LBB1634_126:
	movabsq	$9223372036854775793, %rcx
	addq	$15, %rcx
	movq	24(%rsp), %rax
	movq	%rcx, 8(%rax)
	movq	$-1, (%rax)
	testq	%rsi, %rsi
	movq	48(%rsp), %r12
	movq	16(%rsp), %rbx
	jne	.LBB1634_83
	jmp	.LBB1634_84
.LBB1634_129:
	movq	%rdx, %rdi
	jmp	.LBB1634_137
.LBB1634_128:
	movq	%rax, %rdi
	jmp	.LBB1634_137
.LBB1634_127:
.Ltmp10603:
	leaq	.Lanon.63e1a61e7141635ef6e103ddf8cc2986.3728(%rip), %rdi
	leaq	.Lanon.63e1a61e7141635ef6e103ddf8cc2986.728(%rip), %rcx
	leaq	.Lanon.63e1a61e7141635ef6e103ddf8cc2986.3730(%rip), %r8
	leaq	63(%rsp), %rdx
	movl	$55, %esi
	callq	*_RNvNtCslK5Drzrx8K4_4core6result13unwrap_failed@GOTPCREL(%rip)
.Ltmp10604:
	jmp	.LBB1634_138
.LBB1634_136:
	movq	%rcx, %rdi
.LBB1634_137:
.Ltmp10576:
	leaq	.Lanon.63e1a61e7141635ef6e103ddf8cc2986.352(%rip), %rdx
	movq	32(%rsp), %rsi
	callq	*_RNvNtCslK5Drzrx8K4_4core9panicking18panic_bounds_check@GOTPCREL(%rip)
.Ltmp10577:
.LBB1634_138:
	ud2
.LBB1634_139:
.Ltmp10552:
	movq	%rax, %rbx
	movq	256(%rsp), %rax
	lock		decq	(%rax)
	movb	$1, %r13b
	jne	.LBB1634_171
	#MEMBARRIER
.Ltmp10553:
	leaq	256(%rsp), %rdi
	callq	*_RNvMsn_NtNtCsdf08ABbzq28_5alloc3rcs3arcINtB5_3ArcNtNtNtCskgWIz7JHH9v_11purrdf_core2ir7dataset10RdfDatasetE9drop_slowBR_@GOTPCREL(%rip)
.Ltmp10554:
	jmp	.LBB1634_171
.LBB1634_141:
.Ltmp10626:
	movq	%rax, %rbx
.Ltmp10627:
	movq	%r15, %rdi
	movq	%r14, %rsi
	callq	_RINvNtCslK5Drzrx8K4_4core3ptr9drop_glueINtNtCsdf08ABbzq28_5alloc5boxed3BoxSNtNtNtCskgWIz7JHH9v_11purrdf_core2ir5paged8PageSlotEEB1h_
.Ltmp10628:
	xorl	%r13d, %r13d
	jmp	.LBB1634_171
.LBB1634_143:
.Ltmp10595:
	jmp	.LBB1634_150
.LBB1634_144:
.Ltmp10589:
	movq	%rax, %rbx
.Ltmp10590:
	leaq	560(%rsp), %rdi
	callq	_RINvNtCslK5Drzrx8K4_4core3ptr9drop_glueNtNtNtCskgWIz7JHH9v_11purrdf_core2ir5paged8PageSlotEBH_
.Ltmp10591:
	jmp	.LBB1634_169
.LBB1634_145:
.Ltmp10592:
	callq	*_RNvNtCslK5Drzrx8K4_4core9panicking16panic_in_cleanup@GOTPCREL(%rip)
.LBB1634_146:
.Ltmp10602:
	jmp	.LBB1634_150
.LBB1634_147:
.Ltmp10623:
	movq	%rax, %rbx
	jmp	.LBB1634_171
.LBB1634_148:
.Ltmp10559:
	movq	%rax, %rbx
	jmp	.LBB1634_169
.LBB1634_149:
.Ltmp10549:
.LBB1634_150:
	movq	%rax, %rbx
	movb	$1, %r13b
	jmp	.LBB1634_171
.LBB1634_151:
.Ltmp10586:
	jmp	.LBB1634_168
.LBB1634_152:
.Ltmp10580:
	jmp	.LBB1634_168
.LBB1634_153:
.Ltmp10620:
	movq	%rax, %rbx
	jmp	.LBB1634_175
.LBB1634_154:
.Ltmp10573:
	jmp	.LBB1634_168
.LBB1634_155:
.Ltmp10562:
	jmp	.LBB1634_168
.LBB1634_156:
.Ltmp10605:
	movq	%rax, %rbx
	movq	232(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB1634_158
	movq	240(%rsp), %rdi
	movl	$1, %edx
	callq	*_RNvCsjMmYDGKcCRn_7___rustc14___rust_dealloc@GOTPCREL(%rip)
.LBB1634_158:
	testq	%r14, %r14
	je	.LBB1634_169
	movl	$1, %edx
	movq	%r15, %rdi
	movq	%r14, %rsi
	callq	*_RNvCsjMmYDGKcCRn_7___rustc14___rust_dealloc@GOTPCREL(%rip)
	jmp	.LBB1634_169
.LBB1634_160:
.Ltmp10614:
	movq	%rax, %rbx
	testq	%r15, %r15
	je	.LBB1634_164
	negq	%r15
	addq	$448, %r14
	.p2align	4
.LBB1634_162:
.Ltmp10615:
	movq	%r14, %rdi
	callq	_RINvNtCslK5Drzrx8K4_4core3ptr9drop_glueNtNtNtCskgWIz7JHH9v_11purrdf_core2ir5paged8PageSlotEBH_
.Ltmp10616:
	addq	$448, %r14
	decq	%r15
	jne	.LBB1634_162
.LBB1634_164:
	movq	208(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1634_174
	imulq	$448, %rax, %rsi
	movl	$8, %edx
	movq	%r12, %rdi
	callq	*_RNvCsjMmYDGKcCRn_7___rustc14___rust_dealloc@GOTPCREL(%rip)
	jmp	.LBB1634_174
.LBB1634_166:
.Ltmp10617:
	callq	*_RNvNtCslK5Drzrx8K4_4core9panicking16panic_in_cleanup@GOTPCREL(%rip)
.LBB1634_167:
.Ltmp10583:
.LBB1634_168:
	movq	%rax, %rbx
	leaq	1104(%rsp), %rdi
	callq	_RINvNtCslK5Drzrx8K4_4core3ptr9drop_glueNtNtNtNtCskgWIz7JHH9v_11purrdf_core2ir5paged11translation15PageTranslationEBJ_
.LBB1634_169:
	movq	8(%rsp), %rax
	lock		decq	(%rax)
	movb	$1, %r13b
	jne	.LBB1634_171
	#MEMBARRIER
.Ltmp10606:
	leaq	88(%rsp), %rdi
	callq	*_RNvMsn_NtNtCsdf08ABbzq28_5alloc3rcs3arcINtB5_3ArcNtNtNtCskgWIz7JHH9v_11purrdf_core2ir7dataset10RdfDatasetE9drop_slowBR_@GOTPCREL(%rip)
.Ltmp10607:
.LBB1634_171:
	movq	352(%rsp), %rdi
	movq	360(%rsp), %rsi
	callq	_RINvNtCslK5Drzrx8K4_4core3ptr9drop_glueINtNtNtNtCskEFv3nc5qE4_3std11collections4hash3map7HashMapNtNtNtCskgWIz7JHH9v_11purrdf_core2ir7mutable7QuadKeyyNtNtCsbnavtP7nVTc_11purrdf_hash5fixed10FixedStateEEB1C_
	movq	320(%rsp), %rdi
	movq	328(%rsp), %rsi
	callq	_RINvNtCslK5Drzrx8K4_4core3ptr9drop_glueINtNtNtNtCskEFv3nc5qE4_3std11collections4hash3map7HashMapNtNtNtCskgWIz7JHH9v_11purrdf_core2ir7mutable7QuadKeyyNtNtCsbnavtP7nVTc_11purrdf_hash5fixed10FixedStateEEB1C_
	movq	288(%rsp), %rdi
	movq	296(%rsp), %rsi
	callq	_RINvNtCslK5Drzrx8K4_4core3ptr9drop_glueINtNtNtNtCskEFv3nc5qE4_3std11collections4hash3map7HashMapNtNtNtCskgWIz7JHH9v_11purrdf_core2ir7mutable7QuadKeyyNtNtCsbnavtP7nVTc_11purrdf_hash5fixed10FixedStateEEB1C_
	testb	%r13b, %r13b
	je	.LBB1634_174
.Ltmp10629:
	leaq	208(%rsp), %rdi
	callq	_RINvNtCslK5Drzrx8K4_4core3ptr9drop_glueINtNtCsdf08ABbzq28_5alloc3vec3VecNtNtNtCskgWIz7JHH9v_11purrdf_core2ir5paged8PageSlotEEB1e_
.Ltmp10630:
	jmp	.LBB1634_174
.LBB1634_173:
.Ltmp10633:
	movq	%rax, %rbx
.LBB1634_174:
.Ltmp10634:
	leaq	440(%rsp), %rdi
	callq	_RINvNtCslK5Drzrx8K4_4core3ptr9drop_glueNtNtNtCskgWIz7JHH9v_11purrdf_core2ir6global16GlobalDictionaryEBH_
.Ltmp10635:
.LBB1634_175:
	movq	176(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB1634_177
	#MEMBARRIER
.Ltmp10636:
	leaq	424(%rsp), %rdi
	callq	*_RNvMsn_NtNtCsdf08ABbzq28_5alloc3rcs3arcINtB5_3ArcDNtNtCslK5Drzrx8K4_4core3any3AnyNtNtBQ_6marker4SyncNtB1j_4SendEL_E9drop_slowCskgWIz7JHH9v_11purrdf_core@GOTPCREL(%rip)
.Ltmp10637:
.LBB1634_177:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB1634_178:
.Ltmp10638:
	callq	*_RNvNtCslK5Drzrx8K4_4core9panicking16panic_in_cleanup@GOTPCREL(%rip)
.Lfunc_end1634:
	.size	_RNvMs3_NtNtCskgWIz7JHH9v_11purrdf_core2ir5pagedNtB5_12PagedDataset13from_provider, .Lfunc_end1634-_RNvMs3_NtNtCskgWIz7JHH9v_11purrdf_core2ir5pagedNtB5_12PagedDataset13from_provider
