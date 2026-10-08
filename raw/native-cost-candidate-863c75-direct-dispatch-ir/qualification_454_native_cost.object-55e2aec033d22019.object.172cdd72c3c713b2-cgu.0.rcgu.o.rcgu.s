	.att_syntax
	.file	"object.172cdd72c3c713b2-cgu.0"
	.section	.text._RNvXNtNtCs1ZmoXRRPDtS_6object4read8read_refRShNtB2_7ReadRef19read_bytes_at_until,"ax",@progbits
	.globl	_RNvXNtNtCs1ZmoXRRPDtS_6object4read8read_refRShNtB2_7ReadRef19read_bytes_at_until
	.prefalign	4, .Lfunc_end0, nop
	.type	_RNvXNtNtCs1ZmoXRRPDtS_6object4read8read_refRShNtB2_7ReadRef19read_bytes_at_until,@function
_RNvXNtNtCs1ZmoXRRPDtS_6object4read8read_refRShNtB2_7ReadRef19read_bytes_at_until:
	.cfi_startproc
	cmpq	%rdx, %rcx
	jb	.LBB0_1
	cmpq	%rsi, %rcx
	jbe	.LBB0_4
.LBB0_1:
	xorl	%eax, %eax
	retq
.LBB0_4:
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%rbx
	pushq	%rax
	.cfi_offset %rbx, -24
	addq	%rdi, %rdx
	addq	%rdi, %rcx
	movq	_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw2FN@GOTPCREL(%rip), %rax
	movq	(%rax), %rax
	movzbl	%r8b, %edi
	movq	%rdx, %rbx
	movq	%rdx, %rsi
	movq	%rcx, %rdx
	callq	*%rax
	cmpq	$1, %rax
	jne	.LBB0_5
	movq	%rbx, %rax
	subq	%rbx, %rdx
	addq	$8, %rsp
	popq	%rbx
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	.cfi_restore %rbx
	.cfi_restore %rbp
	retq
.LBB0_5:
	.cfi_def_cfa %rbp, 16
	.cfi_offset %rbx, -24
	.cfi_offset %rbp, -16
	xorl	%eax, %eax
	addq	$8, %rsp
	popq	%rbx
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	.cfi_restore %rbx
	.cfi_restore %rbp
	retq
.Lfunc_end0:
	.size	_RNvXNtNtCs1ZmoXRRPDtS_6object4read8read_refRShNtB2_7ReadRef19read_bytes_at_until, .Lfunc_end0-_RNvXNtNtCs1ZmoXRRPDtS_6object4read8read_refRShNtB2_7ReadRef19read_bytes_at_until
	.cfi_endproc

	.ident	"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"
	.section	".note.GNU-stack","",@progbits
