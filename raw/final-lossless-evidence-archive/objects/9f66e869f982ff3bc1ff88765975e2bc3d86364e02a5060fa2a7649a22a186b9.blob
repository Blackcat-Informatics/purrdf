purrdf_sparql_eval::binop::eval_application::<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#0}:
.Lfunc_begin1295:
	.cfi_startproc
	cmpq	$0, 40(%rsi)
	je	.LBB1295_1
	vpbroadcastq	.LCPI1295_4(%rip), %xmm0
	movabsq	$2746377873070565055, %rax
	movq	16(%rsi), %rdx
	movq	24(%rsi), %rsi
	vpcmpeqd	%xmm1, %xmm1, %xmm1
	xorl	%r8d, %r8d
	xorq	%rdi, %rax
	vpinsrq	$0, %rax, %xmm0, %xmm0
	vaesenc	.LCPI1295_1(%rip), %xmm0, %xmm0
	vaesenc	.LCPI1295_2(%rip), %xmm0, %xmm0
	vaesenc	.LCPI1295_3(%rip), %xmm0, %xmm0
	vmovq	%xmm0, %rax
	movq	%rax, %rcx
	shrq	$57, %rcx
	vpbroadcastb	%ecx, %xmm0
	xorl	%ecx, %ecx
.LBB1295_4:
	andq	%rsi, %rax
	vmovdqu	(%rdx,%rax), %xmm2
	vpcmpeqb	%xmm0, %xmm2, %k0
	kortestw	%k0, %k0
	je	.LBB1295_10
	kmovd	%k0, %r9d
.LBB1295_6:
	xorl	%r10d, %r10d
	tzcntl	%r9d, %r10d
	addq	%rax, %r10
	andq	%rsi, %r10
	negq	%r10
	leaq	(%r10,%r10,4), %r10
	cmpq	%rdi, -40(%rdx,%r10,8)
	je	.LBB1295_7
	leal	-1(%r9), %r10d
	andw	%r9w, %r10w
	movl	%r10d, %r9d
	jne	.LBB1295_6
	.p2align	4
.LBB1295_10:
	vpcmpeqb	%xmm1, %xmm2, %k0
	kortestw	%k0, %k0
	jne	.LBB1295_8
	leaq	16(%rax,%r8), %rax
	addq	$16, %r8
	jmp	.LBB1295_4
.LBB1295_7:
	leaq	(%rdx,%r10,8), %rcx
.LBB1295_8:
	leaq	-32(%rcx), %rax
	testq	%rcx, %rcx
	cmoveq	%rcx, %rax
	retq
.LBB1295_1:
	xorl	%eax, %eax
	retq
.Lfunc_end1295:
