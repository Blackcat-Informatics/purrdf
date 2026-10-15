	.att_syntax
	.file	"probe.898880eada2a4e89-cgu.0"
	.section	.text.probe,"ax",@progbits
	.globl	probe
	.prefalign	4, .Lfunc_end0, nop
	.type	probe,@function
probe:
	.cfi_startproc
	testq	%rsi, %rsi
	je	.LBB0_1
	cmpq	$8, %rsi
	jae	.LBB0_5
	xorl	%ecx, %ecx
	xorl	%eax, %eax
	jmp	.LBB0_4
.LBB0_1:
	xorl	%eax, %eax
	retq
.LBB0_5:
	movabsq	$2305843009213693944, %rcx
	andq	%rsi, %rcx
	leaq	(,%rsi,4), %rax
	andq	$-32, %rax
	pxor	%xmm0, %xmm0
	xorl	%edx, %edx
	pxor	%xmm1, %xmm1
	.p2align	4
.LBB0_6:
	movdqu	(%rdi,%rdx), %xmm2
	paddd	%xmm2, %xmm1
	movdqu	16(%rdi,%rdx), %xmm2
	paddd	%xmm2, %xmm0
	addq	$32, %rdx
	cmpq	%rdx, %rax
	jne	.LBB0_6
	paddd	%xmm1, %xmm0
	pshufd	$238, %xmm0, %xmm1
	paddd	%xmm0, %xmm1
	pshufd	$85, %xmm1, %xmm0
	paddd	%xmm1, %xmm0
	movd	%xmm0, %eax
	jmp	.LBB0_8
.LBB0_4:
	addl	(%rdi,%rcx,4), %eax
	incq	%rcx
.LBB0_8:
	cmpq	%rcx, %rsi
	jne	.LBB0_4
	retq
.Lfunc_end0:
	.size	probe, .Lfunc_end0-probe
	.cfi_endproc

	.ident	"rustc version 1.101.0-nightly (21b707e3f 2026-09-30)"
	.section	".note.GNU-stack","",@progbits
