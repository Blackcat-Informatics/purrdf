	.att_syntax
	.file	"adler2.e3b6bd9f192b25d9-cgu.0"
	.section	.rodata.cst4,"aM",@progbits,4
	.p2align	2, 0x0
.LCPI0_0:
	.long	2147975281
.LCPI0_1:
	.long	65521
	.section	.text._RNvMCsjy79vW79x0H_6adler2NtB2_7Adler3211write_slice,"ax",@progbits
	.globl	_RNvMCsjy79vW79x0H_6adler2NtB2_7Adler3211write_slice
	.prefalign	4, .Lfunc_end0, nop
	.type	_RNvMCsjy79vW79x0H_6adler2NtB2_7Adler3211write_slice,@function
_RNvMCsjy79vW79x0H_6adler2NtB2_7Adler3211write_slice:
	.cfi_startproc
	pushq	%r15
	.cfi_def_cfa_offset 16
	pushq	%r14
	.cfi_def_cfa_offset 24
	pushq	%r13
	.cfi_def_cfa_offset 32
	pushq	%r12
	.cfi_def_cfa_offset 40
	pushq	%rbx
	.cfi_def_cfa_offset 48
	.cfi_offset %rbx, -48
	.cfi_offset %r12, -40
	.cfi_offset %r13, -32
	.cfi_offset %r14, -24
	.cfi_offset %r15, -16
	movq	%rdx, %rax
	movzwl	(%rdi), %ecx
	movq	%rdi, -8(%rsp)
	movzwl	2(%rdi), %r8d
	movabsq	$9223372036854775804, %rdx
	andq	%rax, %rdx
	andl	$3, %eax
	movq	%rax, -16(%rsp)
	movabsq	$3402281327715882719, %rax
	mulxq	%rax, %rax, %rax
	shrq	$12, %rax
	imulq	$22208, %rax, %rax
	movq	%rdx, %rdi
	subq	%rax, %rdi
	xorl	%r14d, %r14d
	cmpq	$22208, %rax
	movl	%ecx, -20(%rsp)
	jae	.LBB0_11
	xorl	%r13d, %r13d
	xorl	%r11d, %r11d
	xorl	%r10d, %r10d
	xorl	%ebx, %ebx
	xorl	%r15d, %r15d
	xorl	%ecx, %ecx
	xorl	%r12d, %r12d
	testq	%rdi, %rdi
	jne	.LBB0_4
	jmp	.LBB0_6
.LBB0_11:
	imull	$22208, %ecx, %ecx
	vpxor	%xmm0, %xmm0, %xmm0
	vpbroadcastd	.LCPI0_0(%rip), %xmm1
	vpbroadcastd	.LCPI0_1(%rip), %xmm2
	movl	$2147975281, %r10d
	movq	%rax, %r11
	movq	%rsi, %rbx
	vpxor	%xmm3, %xmm3, %xmm3
	.p2align	4
.LBB0_12:
	xorl	%r14d, %r14d
	.p2align	4
.LBB0_13:
	vpmovzxbd	(%rbx,%r14), %xmm4
	vpaddd	%xmm4, %xmm3, %xmm3
	vpaddd	%xmm0, %xmm3, %xmm0
	vpmovzxbd	4(%rbx,%r14), %xmm4
	vpaddd	%xmm4, %xmm3, %xmm3
	vpaddd	%xmm0, %xmm3, %xmm0
	vpmovzxbd	8(%rbx,%r14), %xmm4
	vpaddd	%xmm4, %xmm3, %xmm3
	vpaddd	%xmm0, %xmm3, %xmm0
	vpmovzxbd	12(%rbx,%r14), %xmm4
	vpaddd	%xmm4, %xmm3, %xmm3
	vpaddd	%xmm0, %xmm3, %xmm0
	addq	$16, %r14
	cmpq	$22208, %r14
	jne	.LBB0_13
	addq	$22208, %rbx
	addq	$-22208, %r11
	addl	%ecx, %r8d
	vpshufd	$245, %xmm3, %xmm4
	vpmuludq	%xmm1, %xmm4, %xmm4
	vpmuludq	%xmm1, %xmm3, %xmm5
	vpshufd	$245, %xmm5, %xmm5
	vpblendd	$10, %xmm4, %xmm5, %xmm4
	vpsrld	$15, %xmm4, %xmm4
	vpmulld	%xmm2, %xmm4, %xmm4
	vpsubd	%xmm4, %xmm3, %xmm3
	vpshufd	$245, %xmm0, %xmm4
	vpmuludq	%xmm1, %xmm4, %xmm4
	vpmuludq	%xmm1, %xmm0, %xmm5
	vpshufd	$245, %xmm5, %xmm5
	vpblendd	$10, %xmm4, %xmm5, %xmm4
	vpsrld	$15, %xmm4, %xmm4
	vpmulld	%xmm2, %xmm4, %xmm4
	vpsubd	%xmm4, %xmm0, %xmm0
	movq	%r8, %r9
	imulq	%r10, %r9
	shrq	$47, %r9
	imull	$65521, %r9d, %r9d
	subl	%r9d, %r8d
	cmpq	$22208, %r11
	jae	.LBB0_12
	vpextrd	$3, %xmm0, %r11d
	vpextrd	$2, %xmm0, %r13d
	vpextrd	$1, %xmm0, %r14d
	vpextrd	$3, %xmm3, %r10d
	vpextrd	$2, %xmm3, %ebx
	vmovd	%xmm0, %ecx
	vpextrd	$1, %xmm3, %r15d
	vmovd	%xmm3, %r12d
	testq	%rdi, %rdi
	je	.LBB0_6
	.p2align	4
.LBB0_4:
	movzbl	(%rsi,%rax), %r9d
	addl	%r9d, %r12d
	movzbl	1(%rsi,%rax), %r9d
	addl	%r9d, %r15d
	movzbl	2(%rsi,%rax), %r9d
	addl	%r9d, %ebx
	movzbl	3(%rsi,%rax), %r9d
	addl	%r9d, %r10d
	addl	%r12d, %ecx
	addl	%r15d, %r14d
	addl	%ebx, %r13d
	addl	%r10d, %r11d
	addq	$4, %rax
	cmpq	%rax, %rdx
	jne	.LBB0_4
	movl	%r12d, %r9d
	movl	$2147975281, %eax
	imulq	%rax, %r9
	shrq	$47, %r9
	imull	$65521, %r9d, %r9d
	subl	%r9d, %r12d
	movl	%r15d, %r9d
	imulq	%rax, %r9
	shrq	$47, %r9
	imull	$65521, %r9d, %r9d
	subl	%r9d, %r15d
	movl	%ebx, %r9d
	imulq	%rax, %r9
	shrq	$47, %r9
	imull	$65521, %r9d, %r9d
	subl	%r9d, %ebx
	movl	%r10d, %r9d
	imulq	%rax, %r9
	shrq	$47, %r9
	imull	$65521, %r9d, %r9d
	subl	%r9d, %r10d
	movl	%ecx, %r9d
	imulq	%rax, %r9
	shrq	$47, %r9
	imull	$65521, %r9d, %r9d
	subl	%r9d, %ecx
	movl	%r14d, %r9d
	imulq	%rax, %r9
	shrq	$47, %r9
	imull	$65521, %r9d, %r9d
	subl	%r9d, %r14d
	movl	%r13d, %r9d
	imulq	%rax, %r9
	shrq	$47, %r9
	imull	$65521, %r9d, %r9d
	subl	%r9d, %r13d
	movl	%r11d, %r9d
	imulq	%rax, %r9
	shrq	$47, %r9
	imull	$65521, %r9d, %eax
	subl	%eax, %r11d
.LBB0_6:
	addl	%r13d, %r14d
	addl	%ecx, %r11d
	movl	$65521, %eax
	subl	%r10d, %eax
	leal	(%rax,%rax,2), %ecx
	movl	-20(%rsp), %r9d
	imull	%r9d, %edi
	addl	%edi, %r8d
	movl	$2147975281, %eax
	movq	%r8, %rdi
	imulq	%rax, %rdi
	shrq	$47, %rdi
	imull	$65521, %edi, %edi
	subl	%edi, %r8d
	leal	(%r8,%r14,4), %edi
	addl	%ecx, %edi
	leal	(%r15,%rbx,2), %ecx
	subl	%ecx, %edi
	leal	(%rdi,%r11,4), %ecx
	addl	$196563, %ecx
	addl	%ebx, %r10d
	addl	%r15d, %r10d
	addl	%r12d, %r10d
	addl	%r9d, %r10d
	movq	-16(%rsp), %r8
	testq	%r8, %r8
	je	.LBB0_10
	movzbl	(%rsi,%rdx), %edi
	addl	%edi, %r10d
	addl	%r10d, %ecx
	cmpl	$1, %r8d
	je	.LBB0_10
	movzbl	1(%rsi,%rdx), %edi
	addl	%edi, %r10d
	addl	%r10d, %ecx
	cmpl	$2, %r8d
	je	.LBB0_10
	movzbl	2(%rsi,%rdx), %edx
	addl	%edx, %r10d
	addl	%r10d, %ecx
.LBB0_10:
	movl	%r10d, %edx
	imulq	%rax, %rdx
	shrq	$47, %rdx
	imull	$65521, %edx, %edx
	subl	%edx, %r10d
	movq	-8(%rsp), %rsi
	movw	%r10w, (%rsi)
	movl	%ecx, %edx
	imulq	%rax, %rdx
	shrq	$47, %rdx
	imull	$65521, %edx, %eax
	subl	%eax, %ecx
	movw	%cx, 2(%rsi)
	popq	%rbx
	.cfi_def_cfa_offset 40
	popq	%r12
	.cfi_def_cfa_offset 32
	popq	%r13
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end0:
	.size	_RNvMCsjy79vW79x0H_6adler2NtB2_7Adler3211write_slice, .Lfunc_end0-_RNvMCsjy79vW79x0H_6adler2NtB2_7Adler3211write_slice
	.cfi_endproc

	.ident	"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"
	.section	".note.GNU-stack","",@progbits
