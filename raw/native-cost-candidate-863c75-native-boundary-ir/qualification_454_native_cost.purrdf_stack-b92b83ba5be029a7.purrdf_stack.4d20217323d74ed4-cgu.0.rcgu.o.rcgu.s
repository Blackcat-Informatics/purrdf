	.att_syntax
	.file	"purrdf_stack.4d20217323d74ed4-cgu.0"
	.section	.text.unlikely._RNvCs6CxjjnbUHAu_12purrdf_stack11is_low_cold,"ax",@progbits
	.globl	_RNvCs6CxjjnbUHAu_12purrdf_stack11is_low_cold
	.prefalign	4, .Lfunc_end0, nop
	.type	_RNvCs6CxjjnbUHAu_12purrdf_stack11is_low_cold,@function
_RNvCs6CxjjnbUHAu_12purrdf_stack11is_low_cold:
	.cfi_startproc
	pushq	%rbx
	.cfi_def_cfa_offset 16
	.cfi_offset %rbx, -16
	movq	%rdi, %rbx
	data16
	leaq	_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack5FLOOR0s_023___RUST_STD_INTERNAL_VAL@TLSGD(%rip), %rdi
	data16
	data16
	rex64
	callq	__tls_get_addr@PLT
	movq	(%rax), %rcx
	movb	$1, %al
	cmpq	$-2, %rcx
	je	.LBB0_4
	movq	%rbx, %rax
	subq	%rcx, %rax
	jae	.LBB0_3
	movq	%rbx, %rdi
	callq	_RNvCs6CxjjnbUHAu_12purrdf_stack7refresh.llvm.12345758532933907005
.LBB0_3:
	cmpq	$131072, %rax
	setb	%al
.LBB0_4:
	popq	%rbx
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end0:
	.size	_RNvCs6CxjjnbUHAu_12purrdf_stack11is_low_cold, .Lfunc_end0-_RNvCs6CxjjnbUHAu_12purrdf_stack11is_low_cold
	.cfi_endproc

	.section	.text.unlikely._RNvCs6CxjjnbUHAu_12purrdf_stack11walk_refuse,"ax",@progbits
	.globl	_RNvCs6CxjjnbUHAu_12purrdf_stack11walk_refuse
	.prefalign	4, .Lfunc_end1, nop
	.type	_RNvCs6CxjjnbUHAu_12purrdf_stack11walk_refuse,@function
_RNvCs6CxjjnbUHAu_12purrdf_stack11walk_refuse:
	.cfi_startproc
	pushq	%r15
	.cfi_def_cfa_offset 16
	pushq	%r14
	.cfi_def_cfa_offset 24
	pushq	%rbx
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -32
	.cfi_offset %r14, -24
	.cfi_offset %r15, -16
	movq	%rsi, %rbx
	movq	%rdi, %r14
	data16
	leaq	_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack4WALK0s_023___RUST_STD_INTERNAL_VAL@TLSGD(%rip), %rdi
	data16
	data16
	rex64
	callq	__tls_get_addr@PLT
	movq	(%rax), %rcx
	testq	%rcx, %rcx
	je	.LBB1_4
	cmpq	$1, %rcx
	jne	.LBB1_3
	movq	%rax, %r15
	data16
	leaq	_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack5FLOOR0s_023___RUST_STD_INTERNAL_VAL@TLSGD(%rip), %rdi
	data16
	data16
	rex64
	callq	__tls_get_addr@PLT
	movq	$2, (%r15)
	movq	(%rax), %rcx
	movq	$-2, (%rax)
	movq	%rcx, 8(%r15)
	movq	%r14, 16(%r15)
	movq	%rbx, 24(%r15)
.LBB1_3:
	movb	$1, %al
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	retq
.LBB1_4:
	.cfi_def_cfa_offset 32
	xorl	%eax, %eax
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end1:
	.size	_RNvCs6CxjjnbUHAu_12purrdf_stack11walk_refuse, .Lfunc_end1-_RNvCs6CxjjnbUHAu_12purrdf_stack11walk_refuse
	.cfi_endproc

	.section	.text._RNvCs6CxjjnbUHAu_12purrdf_stack7refresh.llvm.12345758532933907005,"ax",@progbits
	.hidden	_RNvCs6CxjjnbUHAu_12purrdf_stack7refresh.llvm.12345758532933907005
	.globl	_RNvCs6CxjjnbUHAu_12purrdf_stack7refresh.llvm.12345758532933907005
	.prefalign	4, .Lfunc_end2, nop
	.type	_RNvCs6CxjjnbUHAu_12purrdf_stack7refresh.llvm.12345758532933907005,@function
_RNvCs6CxjjnbUHAu_12purrdf_stack7refresh.llvm.12345758532933907005:
	.cfi_startproc
	pushq	%r15
	.cfi_def_cfa_offset 16
	pushq	%r14
	.cfi_def_cfa_offset 24
	pushq	%rbx
	.cfi_def_cfa_offset 32
	subq	$128, %rsp
	.cfi_def_cfa_offset 160
	.cfi_offset %rbx, -32
	.cfi_offset %r14, -24
	.cfi_offset %r15, -16
	movq	pthread_attr_init@GOTPCREL(%rip), %rax
	movq	%rdi, %rbx
	leaq	72(%rsp), %rdi
	callq	*%rax
	testl	%eax, %eax
	jne	.LBB2_5
	vmovups	96(%rsp), %ymm1
	vmovups	72(%rsp), %ymm0
	movq	pthread_self@GOTPCREL(%rip), %rax
	movq	$0, (%rsp)
	movq	$0, 8(%rsp)
	vmovups	%ymm1, 40(%rsp)
	vmovups	%ymm0, 16(%rsp)
	vzeroupper
	callq	*%rax
	movq	pthread_getattr_np@GOTPCREL(%rip), %rcx
	leaq	16(%rsp), %rsi
	movq	%rax, %rdi
	callq	*%rcx
	testl	%eax, %eax
	jne	.LBB2_4
	movq	pthread_attr_getstack@GOTPCREL(%rip), %rax
	leaq	16(%rsp), %rdi
	leaq	8(%rsp), %rdx
	movq	%rsp, %rsi
	callq	*%rax
	testl	%eax, %eax
	je	.LBB2_7
.LBB2_4:
	movq	pthread_attr_destroy@GOTPCREL(%rip), %rax
	leaq	16(%rsp), %rdi
	callq	*%rax
.LBB2_5:
	xorl	%r14d, %r14d
.LBB2_6:
	data16
	leaq	_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack5FLOOR0s_023___RUST_STD_INTERNAL_VAL@TLSGD(%rip), %rdi
	data16
	data16
	rex64
	callq	__tls_get_addr@PLT
	movq	%r14, (%rax)
	xorl	%eax, %eax
	subq	%r14, %rbx
	cmovaeq	%rbx, %rax
	addq	$128, %rsp
	.cfi_def_cfa_offset 32
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	retq
.LBB2_7:
	.cfi_def_cfa_offset 160
	movq	pthread_attr_destroy@GOTPCREL(%rip), %rax
	movq	(%rsp), %r15
	leaq	16(%rsp), %rdi
	callq	*%rax
	leaq	_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack8RESERVED0s_023___RUST_STD_INTERNAL_VAL.llvm.12345758532933907005@TLSLD(%rip), %rdi
	callq	__tls_get_addr@PLT
	movq	$-1, %r14
	addq	_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack8RESERVED0s_023___RUST_STD_INTERNAL_VAL.llvm.12345758532933907005@DTPOFF(%rax), %r15
	cmovaeq	%r15, %r14
	jmp	.LBB2_6
.Lfunc_end2:
	.size	_RNvCs6CxjjnbUHAu_12purrdf_stack7refresh.llvm.12345758532933907005, .Lfunc_end2-_RNvCs6CxjjnbUHAu_12purrdf_stack7refresh.llvm.12345758532933907005
	.cfi_endproc

	.section	.text._RNvCs6CxjjnbUHAu_12purrdf_stack9remaining,"ax",@progbits
	.globl	_RNvCs6CxjjnbUHAu_12purrdf_stack9remaining
	.prefalign	4, .Lfunc_end3, nop
	.type	_RNvCs6CxjjnbUHAu_12purrdf_stack9remaining,@function
_RNvCs6CxjjnbUHAu_12purrdf_stack9remaining:
	.cfi_startproc
	subq	$24, %rsp
	.cfi_def_cfa_offset 32
	leaq	15(%rsp), %rax
	movb	$0, 15(%rsp)
	movq	%rax, 16(%rsp)
	leaq	16(%rsp), %rax
	#APP
	#NO_APP
	data16
	leaq	_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack5FLOOR0s_023___RUST_STD_INTERNAL_VAL@TLSGD(%rip), %rdi
	data16
	data16
	rex64
	callq	__tls_get_addr@PLT
	movq	(%rax), %rcx
	cmpq	$-2, %rcx
	jne	.LBB3_3
	xorl	%eax, %eax
	addq	$24, %rsp
	.cfi_def_cfa_offset 8
	retq
.LBB3_3:
	.cfi_def_cfa_offset 32
	movq	16(%rsp), %rax
	cmpq	%rax, %rcx
	jbe	.LBB3_4
	movq	%rax, %rdi
	callq	_RNvCs6CxjjnbUHAu_12purrdf_stack7refresh.llvm.12345758532933907005
	addq	$24, %rsp
	.cfi_def_cfa_offset 8
	retq
.LBB3_4:
	.cfi_def_cfa_offset 32
	subq	%rcx, %rax
	addq	$24, %rsp
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end3:
	.size	_RNvCs6CxjjnbUHAu_12purrdf_stack9remaining, .Lfunc_end3-_RNvCs6CxjjnbUHAu_12purrdf_stack9remaining
	.cfi_endproc

	.section	.text._RNvXNvCs6CxjjnbUHAu_12purrdf_stack4walkNtB2_5CloseNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop,"ax",@progbits
	.globl	_RNvXNvCs6CxjjnbUHAu_12purrdf_stack4walkNtB2_5CloseNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop
	.prefalign	4, .Lfunc_end4, nop
	.type	_RNvXNvCs6CxjjnbUHAu_12purrdf_stack4walkNtB2_5CloseNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop,@function
_RNvXNvCs6CxjjnbUHAu_12purrdf_stack4walkNtB2_5CloseNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop:
	.cfi_startproc
	pushq	%r14
	.cfi_def_cfa_offset 16
	pushq	%rbx
	.cfi_def_cfa_offset 24
	pushq	%rax
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -24
	.cfi_offset %r14, -16
	movq	%rdi, %rbx
	data16
	leaq	_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack4WALK0s_023___RUST_STD_INTERNAL_VAL@TLSGD(%rip), %rdi
	data16
	data16
	rex64
	callq	__tls_get_addr@PLT
	vmovups	(%rbx), %ymm0
	movq	(%rax), %rcx
	movq	8(%rax), %r14
	vmovups	%ymm0, (%rax)
	cmpq	$2, %rcx
	je	.LBB4_3
	cmpq	$-1, %rcx
	je	.LBB4_2
	addq	$8, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	vzeroupper
	retq
.LBB4_3:
	.cfi_def_cfa_offset 32
	data16
	leaq	_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack5FLOOR0s_023___RUST_STD_INTERNAL_VAL@TLSGD(%rip), %rdi
	data16
	data16
	rex64
	callq	__tls_get_addr@PLT
	movq	%r14, (%rax)
	addq	$8, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	vzeroupper
	retq
.LBB4_2:
	.cfi_def_cfa_offset 32
	movq	_RNvNtNtCs7jcFBdfocI9_3std6thread5local18panic_access_error@GOTPCREL(%rip), %rax
	leaq	anon.b0b42612b86aa4e26bca5f33320cc1f1.1.llvm.12345758532933907005(%rip), %rdi
	vzeroupper
	callq	*%rax
.Lfunc_end4:
	.size	_RNvXNvCs6CxjjnbUHAu_12purrdf_stack4walkNtB2_5CloseNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop, .Lfunc_end4-_RNvXNvCs6CxjjnbUHAu_12purrdf_stack4walkNtB2_5CloseNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop
	.cfi_endproc

	.hidden	anon.b0b42612b86aa4e26bca5f33320cc1f1.0.llvm.12345758532933907005
	.type	anon.b0b42612b86aa4e26bca5f33320cc1f1.0.llvm.12345758532933907005,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
	.globl	anon.b0b42612b86aa4e26bca5f33320cc1f1.0.llvm.12345758532933907005
anon.b0b42612b86aa4e26bca5f33320cc1f1.0.llvm.12345758532933907005:
	.asciz	"/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad/library/std/src/thread/local.rs"
	.size	anon.b0b42612b86aa4e26bca5f33320cc1f1.0.llvm.12345758532933907005, 80

	.hidden	anon.b0b42612b86aa4e26bca5f33320cc1f1.1.llvm.12345758532933907005
	.type	anon.b0b42612b86aa4e26bca5f33320cc1f1.1.llvm.12345758532933907005,@object
	.section	.data.rel.ro.anon.b0b42612b86aa4e26bca5f33320cc1f1.1.llvm.12345758532933907005,"aw",@progbits
	.globl	anon.b0b42612b86aa4e26bca5f33320cc1f1.1.llvm.12345758532933907005
	.p2align	3, 0x0
anon.b0b42612b86aa4e26bca5f33320cc1f1.1.llvm.12345758532933907005:
	.quad	anon.b0b42612b86aa4e26bca5f33320cc1f1.0.llvm.12345758532933907005
	.asciz	"O\000\000\000\000\000\000\000\255\001\000\000\031\000\000"
	.size	anon.b0b42612b86aa4e26bca5f33320cc1f1.1.llvm.12345758532933907005, 24

	.type	_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack4WALK0s_023___RUST_STD_INTERNAL_VAL,@object
	.section	.tbss._RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack4WALK0s_023___RUST_STD_INTERNAL_VAL,"awT",@nobits
	.globl	_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack4WALK0s_023___RUST_STD_INTERNAL_VAL
	.p2align	3, 0x0
_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack4WALK0s_023___RUST_STD_INTERNAL_VAL:
	.zero	8
	.zero	24
	.size	_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack4WALK0s_023___RUST_STD_INTERNAL_VAL, 32

	.type	_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack5FLOOR0s_023___RUST_STD_INTERNAL_VAL,@object
	.section	.tdata._RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack5FLOOR0s_023___RUST_STD_INTERNAL_VAL,"awT",@progbits
	.globl	_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack5FLOOR0s_023___RUST_STD_INTERNAL_VAL
	.p2align	3, 0x0
_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack5FLOOR0s_023___RUST_STD_INTERNAL_VAL:
	.zero	8,255
	.size	_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack5FLOOR0s_023___RUST_STD_INTERNAL_VAL, 8

	.hidden	_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack8RESERVED0s_023___RUST_STD_INTERNAL_VAL.llvm.12345758532933907005
	.type	_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack8RESERVED0s_023___RUST_STD_INTERNAL_VAL.llvm.12345758532933907005,@object
	.section	.tbss._RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack8RESERVED0s_023___RUST_STD_INTERNAL_VAL.llvm.12345758532933907005,"awT",@nobits
	.globl	_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack8RESERVED0s_023___RUST_STD_INTERNAL_VAL.llvm.12345758532933907005
	.p2align	3, 0x0
_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack8RESERVED0s_023___RUST_STD_INTERNAL_VAL.llvm.12345758532933907005:
	.zero	8
	.size	_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack8RESERVED0s_023___RUST_STD_INTERNAL_VAL.llvm.12345758532933907005, 8

	.ident	"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"
	.section	".note.GNU-stack","",@progbits
