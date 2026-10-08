	.att_syntax
	.file	"hashbrown.32bd1dc6e5d86b0c-cgu.0"
	.section	.text._RNvMNtCs4m51Kb4I71M_9hashbrown3rawNtB2_11Fallibility17capacity_overflow,"ax",@progbits
	.globl	_RNvMNtCs4m51Kb4I71M_9hashbrown3rawNtB2_11Fallibility17capacity_overflow
	.prefalign	4, .Lfunc_end0, nop
	.type	_RNvMNtCs4m51Kb4I71M_9hashbrown3rawNtB2_11Fallibility17capacity_overflow,@function
_RNvMNtCs4m51Kb4I71M_9hashbrown3rawNtB2_11Fallibility17capacity_overflow:
.Lfunc_begin0:
	.cfi_startproc
	testl	%edi, %edi
	jne	.LBB0_2
	xorl	%eax, %eax
	retq
.LBB0_2:
	pushq	%rax
	.cfi_def_cfa_offset 16
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking9panic_fmt@GOTPCREL(%rip), %rax
	leaq	anon.f083946614dbed2886867a2b7043ff2f.0.llvm.11433797203420380433(%rip), %rdi
	leaq	anon.f083946614dbed2886867a2b7043ff2f.2.llvm.11433797203420380433(%rip), %rdx
	movl	$57, %esi
	callq	*%rax
.Lfunc_end0:
	.size	_RNvMNtCs4m51Kb4I71M_9hashbrown3rawNtB2_11Fallibility17capacity_overflow, .Lfunc_end0-_RNvMNtCs4m51Kb4I71M_9hashbrown3rawNtB2_11Fallibility17capacity_overflow
	.cfi_endproc

	.section	.text._RNvMNtCs4m51Kb4I71M_9hashbrown3rawNtB2_11Fallibility9alloc_err,"ax",@progbits
	.globl	_RNvMNtCs4m51Kb4I71M_9hashbrown3rawNtB2_11Fallibility9alloc_err
	.prefalign	4, .Lfunc_end1, nop
	.type	_RNvMNtCs4m51Kb4I71M_9hashbrown3rawNtB2_11Fallibility9alloc_err,@function
_RNvMNtCs4m51Kb4I71M_9hashbrown3rawNtB2_11Fallibility9alloc_err:
.Lfunc_begin1:
	.cfi_startproc
	testl	%edi, %edi
	jne	.LBB1_2
	movq	%rsi, %rax
	retq
.LBB1_2:
	pushq	%rax
	.cfi_def_cfa_offset 16
	movq	_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error@GOTPCREL(%rip), %rax
	movq	%rsi, %rdi
	movq	%rdx, %rsi
	callq	*%rax
.Lfunc_end1:
	.size	_RNvMNtCs4m51Kb4I71M_9hashbrown3rawNtB2_11Fallibility9alloc_err, .Lfunc_end1-_RNvMNtCs4m51Kb4I71M_9hashbrown3rawNtB2_11Fallibility9alloc_err
	.cfi_endproc

	.hidden	anon.f083946614dbed2886867a2b7043ff2f.0.llvm.11433797203420380433
	.type	anon.f083946614dbed2886867a2b7043ff2f.0.llvm.11433797203420380433,@object
	.section	.rodata.anon.f083946614dbed2886867a2b7043ff2f.0.llvm.11433797203420380433,"a",@progbits
	.globl	anon.f083946614dbed2886867a2b7043ff2f.0.llvm.11433797203420380433
anon.f083946614dbed2886867a2b7043ff2f.0.llvm.11433797203420380433:
	.ascii	"Hash table capacity overflow"
	.size	anon.f083946614dbed2886867a2b7043ff2f.0.llvm.11433797203420380433, 28

	.hidden	anon.f083946614dbed2886867a2b7043ff2f.1.llvm.11433797203420380433
	.type	anon.f083946614dbed2886867a2b7043ff2f.1.llvm.11433797203420380433,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
	.globl	anon.f083946614dbed2886867a2b7043ff2f.1.llvm.11433797203420380433
anon.f083946614dbed2886867a2b7043ff2f.1.llvm.11433797203420380433:
	.asciz	"/kache/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.17.1/src/raw.rs"
	.size	anon.f083946614dbed2886867a2b7043ff2f.1.llvm.11433797203420380433, 88

	.hidden	anon.f083946614dbed2886867a2b7043ff2f.2.llvm.11433797203420380433
	.type	anon.f083946614dbed2886867a2b7043ff2f.2.llvm.11433797203420380433,@object
	.section	.data.rel.ro.anon.f083946614dbed2886867a2b7043ff2f.2.llvm.11433797203420380433,"aw",@progbits
	.globl	anon.f083946614dbed2886867a2b7043ff2f.2.llvm.11433797203420380433
	.p2align	3, 0x0
anon.f083946614dbed2886867a2b7043ff2f.2.llvm.11433797203420380433:
	.quad	anon.f083946614dbed2886867a2b7043ff2f.1.llvm.11433797203420380433
	.asciz	"W\000\000\000\000\000\000\000$\000\000\000(\000\000"
	.size	anon.f083946614dbed2886867a2b7043ff2f.2.llvm.11433797203420380433, 24

	.ident	"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"
	.section	".note.GNU-stack","",@progbits
