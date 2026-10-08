	.att_syntax
	.file	"purrdf_alloc_probe.18fd37fd33e105d1-cgu.0"
	.section	.text._RNvMs_Cs2911K1BwAFx_18purrdf_alloc_probeNtB4_19CurrentThreadWindow4open,"ax",@progbits
	.globl	_RNvMs_Cs2911K1BwAFx_18purrdf_alloc_probeNtB4_19CurrentThreadWindow4open
	.prefalign	4, .Lfunc_end0, nop
	.type	_RNvMs_Cs2911K1BwAFx_18purrdf_alloc_probeNtB4_19CurrentThreadWindow4open,@function
_RNvMs_Cs2911K1BwAFx_18purrdf_alloc_probeNtB4_19CurrentThreadWindow4open:
.Lfunc_begin0:
	.cfi_startproc
	pushq	%rbx
	.cfi_def_cfa_offset 16
	.cfi_offset %rbx, -16
	movq	%rdi, %rbx
	leaq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_WINDOW_OPEN0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TLSLD(%rip), %rdi
	callq	__tls_get_addr@PLT
	cmpb	$0, _RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_WINDOW_OPEN0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax)
	movb	$1, _RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_WINDOW_OPEN0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax)
	jne	.LBB0_2
	movq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax), %rdx
	movq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax), %rcx
	movq	%rdx, _RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax)
	movq	%rdx, _RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax)
	movq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax), %rax
	movq	%rcx, (%rbx)
	movq	%rax, 8(%rbx)
	movq	%rdx, 16(%rbx)
	movq	%rbx, %rax
	popq	%rbx
	.cfi_def_cfa_offset 8
	retq
.LBB0_2:
	.cfi_def_cfa_offset 16
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking9panic_fmt@GOTPCREL(%rip), %rax
	leaq	anon.030a113babdea385b7b535f4f2fa1758.5.llvm.6551227014246703601(%rip), %rdi
	leaq	anon.030a113babdea385b7b535f4f2fa1758.6.llvm.6551227014246703601(%rip), %rdx
	movl	$105, %esi
	callq	*%rax
.Lfunc_end0:
	.size	_RNvMs_Cs2911K1BwAFx_18purrdf_alloc_probeNtB4_19CurrentThreadWindow4open, .Lfunc_end0-_RNvMs_Cs2911K1BwAFx_18purrdf_alloc_probeNtB4_19CurrentThreadWindow4open
	.cfi_endproc

	.section	.rodata.cst8,"aM",@progbits,8
	.p2align	3, 0x0
.LCPI1_0:
	.quad	-9223372036854775808
.LCPI1_1:
	.quad	9223372036854775807
	.section	.text._RNvMs_Cs2911K1BwAFx_18purrdf_alloc_probeNtB4_19CurrentThreadWindow5close,"ax",@progbits
	.globl	_RNvMs_Cs2911K1BwAFx_18purrdf_alloc_probeNtB4_19CurrentThreadWindow5close
	.prefalign	4, .Lfunc_end1, nop
	.type	_RNvMs_Cs2911K1BwAFx_18purrdf_alloc_probeNtB4_19CurrentThreadWindow5close,@function
_RNvMs_Cs2911K1BwAFx_18purrdf_alloc_probeNtB4_19CurrentThreadWindow5close:
.Lfunc_begin1:
	.cfi_startproc
	pushq	%r14
	.cfi_def_cfa_offset 16
	pushq	%rbx
	.cfi_def_cfa_offset 24
	pushq	%rax
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -24
	.cfi_offset %r14, -16
	movq	%rsi, %r14
	movq	%rdi, %rbx
	leaq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TLSLD(%rip), %rdi
	callq	__tls_get_addr@PLT
	vmovdqu	(%r14), %xmm0
	vmovq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax), %xmm1
	vmovq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax), %xmm2
	vmovq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax), %xmm3
	movb	$0, _RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_WINDOW_OPEN0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax)
	vpunpcklqdq	%xmm1, %xmm2, %xmm1
	vmovq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax), %xmm2
	vpmaxuq	%xmm0, %xmm1, %xmm1
	vpsubq	%xmm0, %xmm1, %xmm0
	vmovq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax), %xmm1
	movq	%rbx, %rax
	vmovdqu	%xmm0, (%rbx)
	vpunpcklqdq	%xmm1, %xmm2, %xmm1
	vmovq	16(%r14), %xmm2
	vpunpcklqdq	%xmm3, %xmm2, %xmm2
	vpsubq	%xmm2, %xmm1, %xmm0
	vpcmpgtq	%xmm1, %xmm2, %k0
	vpbroadcastq	.LCPI1_0(%rip), %xmm2
	vpmovq2m	%xmm0, %k1
	kxorw	%k1, %k0, %k2
	vpbroadcastq	.LCPI1_1(%rip), %xmm2 {%k1}
	vmovdqa64	%xmm2, %xmm0 {%k2}
	vmovdqu	%xmm0, 16(%rbx)
	addq	$8, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end1:
	.size	_RNvMs_Cs2911K1BwAFx_18purrdf_alloc_probeNtB4_19CurrentThreadWindow5close, .Lfunc_end1-_RNvMs_Cs2911K1BwAFx_18purrdf_alloc_probeNtB4_19CurrentThreadWindow5close
	.cfi_endproc

	.section	.text._RNvXCs2911K1BwAFx_18purrdf_alloc_probeNtB2_17CountingAllocatorNtNtNtCs2k2z8Zem4rB_4core5alloc6global11GlobalAlloc5alloc,"ax",@progbits
	.globl	_RNvXCs2911K1BwAFx_18purrdf_alloc_probeNtB2_17CountingAllocatorNtNtNtCs2k2z8Zem4rB_4core5alloc6global11GlobalAlloc5alloc
	.prefalign	4, .Lfunc_end2, nop
	.type	_RNvXCs2911K1BwAFx_18purrdf_alloc_probeNtB2_17CountingAllocatorNtNtNtCs2k2z8Zem4rB_4core5alloc6global11GlobalAlloc5alloc,@function
_RNvXCs2911K1BwAFx_18purrdf_alloc_probeNtB2_17CountingAllocatorNtNtNtCs2k2z8Zem4rB_4core5alloc6global11GlobalAlloc5alloc:
.Lfunc_begin2:
	.cfi_startproc
	pushq	%r14
	.cfi_def_cfa_offset 16
	pushq	%rbx
	.cfi_def_cfa_offset 24
	pushq	%rax
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -24
	.cfi_offset %r14, -16
	cmpq	$17, %rsi
	movq	%rdx, %rbx
	setb	%al
	cmpq	%rdx, %rsi
	setbe	%cl
	testb	%cl, %al
	je	.LBB2_11
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
	testq	%rax, %rax
	jne	.LBB2_2
	jmp	.LBB2_12
.LBB2_11:
	movq	posix_memalign@GOTPCREL(%rip), %rcx
	cmpq	$9, %rsi
	movl	$8, %eax
	movq	$0, (%rsp)
	movq	%rsp, %rdi
	movq	%rbx, %rdx
	cmovaeq	%rsi, %rax
	movq	%rax, %rsi
	callq	*%rcx
	testl	%eax, %eax
	je	.LBB2_13
.LBB2_12:
	xorl	%eax, %eax
	addq	$8, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	retq
.LBB2_13:
	.cfi_def_cfa_offset 32
	movq	(%rsp), %rax
	testq	%rax, %rax
	je	.LBB2_12
.LBB2_2:
	movq	%rax, %r14
	leaq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TLSLD(%rip), %rdi
	callq	__tls_get_addr@PLT
	movq	$-1, %rsi
	movabsq	$9223372036854775807, %rdi
	movq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax), %rdx
	incq	%rdx
	cmoveq	%rsi, %rdx
	movq	%rdx, _RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax)
	movq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax), %rdx
	addq	%rbx, %rdx
	cmovbq	%rsi, %rdx
	movq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax), %rsi
	cmpq	%rdi, %rbx
	movq	%rdx, _RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax)
	movq	%rdi, %rdx
	cmovbq	%rbx, %rdx
	addq	%rdx, %rsi
	cmovoq	%rdi, %rsi
	movq	%rsi, _RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax)
	cmpq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax), %rsi
	jle	.LBB2_4
	leaq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax), %rax
	movq	%rsi, (%rax)
	.p2align	4
.LBB2_4:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB2_10
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB2_4
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	%rbx, (%rcx)
	movq	%rdx, %rcx
	lock		xaddq	%rcx, (%rsi)
	movabsq	$-9223372036854775808, %rsi
	leaq	(%rcx,%rdx), %rax
	sarq	$63, %rax
	xorq	%rax, %rsi
	addq	%rdx, %rcx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	cmovoq	%rsi, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB2_7:
	cmpq	%rax, %rcx
	jle	.LBB2_9
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB2_7
.LBB2_9:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB2_10:
	movq	%r14, %rax
	addq	$8, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end2:
	.size	_RNvXCs2911K1BwAFx_18purrdf_alloc_probeNtB2_17CountingAllocatorNtNtNtCs2k2z8Zem4rB_4core5alloc6global11GlobalAlloc5alloc, .Lfunc_end2-_RNvXCs2911K1BwAFx_18purrdf_alloc_probeNtB2_17CountingAllocatorNtNtNtCs2k2z8Zem4rB_4core5alloc6global11GlobalAlloc5alloc
	.cfi_endproc

	.section	.text._RNvXCs2911K1BwAFx_18purrdf_alloc_probeNtB2_17CountingAllocatorNtNtNtCs2k2z8Zem4rB_4core5alloc6global11GlobalAlloc7dealloc,"ax",@progbits
	.globl	_RNvXCs2911K1BwAFx_18purrdf_alloc_probeNtB2_17CountingAllocatorNtNtNtCs2k2z8Zem4rB_4core5alloc6global11GlobalAlloc7dealloc
	.prefalign	4, .Lfunc_end3, nop
	.type	_RNvXCs2911K1BwAFx_18purrdf_alloc_probeNtB2_17CountingAllocatorNtNtNtCs2k2z8Zem4rB_4core5alloc6global11GlobalAlloc7dealloc,@function
_RNvXCs2911K1BwAFx_18purrdf_alloc_probeNtB2_17CountingAllocatorNtNtNtCs2k2z8Zem4rB_4core5alloc6global11GlobalAlloc7dealloc:
.Lfunc_begin3:
	.cfi_startproc
	pushq	%r14
	.cfi_def_cfa_offset 16
	pushq	%rbx
	.cfi_def_cfa_offset 24
	pushq	%rax
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -24
	.cfi_offset %r14, -16
	movq	%rcx, %r14
	movq	%rsi, %rbx
	leaq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TLSLD(%rip), %rdi
	callq	__tls_get_addr@PLT
	movabsq	$9223372036854775807, %rdx
	cmpq	%rdx, %r14
	cmovaeq	%rdx, %r14
	xorl	%edi, %edi
	movq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax), %rsi
	cmpq	%r14, %rsi
	setns	%dil
	addq	%rdx, %rdi
	subq	%r14, %rsi
	cmovoq	%rdi, %rsi
	movq	%rsi, _RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax)
	cmpq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax), %rsi
	jge	.LBB3_2
	leaq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax), %rax
	movq	%rsi, (%rax)
	.p2align	4
.LBB3_2:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB3_8
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB3_2
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%r14, %rcx
	negq	%rcx
	lock		xaddq	%rcx, (%rax)
	xorl	%eax, %eax
	cmpq	%r14, %rcx
	setns	%al
	addq	%rdx, %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	subq	%r14, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB3_5:
	cmpq	%rax, %rcx
	jge	.LBB3_7
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB3_5
.LBB3_7:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB3_8:
	movq	%rbx, %rdi
	addq	$8, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*free@GOTPCREL(%rip)
.Lfunc_end3:
	.size	_RNvXCs2911K1BwAFx_18purrdf_alloc_probeNtB2_17CountingAllocatorNtNtNtCs2k2z8Zem4rB_4core5alloc6global11GlobalAlloc7dealloc, .Lfunc_end3-_RNvXCs2911K1BwAFx_18purrdf_alloc_probeNtB2_17CountingAllocatorNtNtNtCs2k2z8Zem4rB_4core5alloc6global11GlobalAlloc7dealloc
	.cfi_endproc

	.section	.text._RNvXCs2911K1BwAFx_18purrdf_alloc_probeNtB2_17CountingAllocatorNtNtNtCs2k2z8Zem4rB_4core5alloc6global11GlobalAlloc7realloc,"ax",@progbits
	.globl	_RNvXCs2911K1BwAFx_18purrdf_alloc_probeNtB2_17CountingAllocatorNtNtNtCs2k2z8Zem4rB_4core5alloc6global11GlobalAlloc7realloc
	.prefalign	4, .Lfunc_end4, nop
	.type	_RNvXCs2911K1BwAFx_18purrdf_alloc_probeNtB2_17CountingAllocatorNtNtNtCs2k2z8Zem4rB_4core5alloc6global11GlobalAlloc7realloc,@function
_RNvXCs2911K1BwAFx_18purrdf_alloc_probeNtB2_17CountingAllocatorNtNtNtCs2k2z8Zem4rB_4core5alloc6global11GlobalAlloc7realloc:
.Lfunc_begin4:
	.cfi_startproc
	pushq	%r15
	.cfi_def_cfa_offset 16
	pushq	%r14
	.cfi_def_cfa_offset 24
	pushq	%r12
	.cfi_def_cfa_offset 32
	pushq	%rbx
	.cfi_def_cfa_offset 40
	pushq	%rax
	.cfi_def_cfa_offset 48
	.cfi_offset %rbx, -40
	.cfi_offset %r12, -32
	.cfi_offset %r14, -24
	.cfi_offset %r15, -16
	cmpq	$17, %rdx
	movq	%rcx, %r15
	movq	%r8, %rbx
	movq	%rsi, %r12
	setb	%cl
	cmpq	%r8, %rdx
	setbe	%al
	andb	%cl, %al
	cmpq	%r15, %r8
	jae	.LBB4_4
	testb	%al, %al
	jne	.LBB4_5
	movq	posix_memalign@GOTPCREL(%rip), %rax
	cmpq	$9, %rdx
	movl	$8, %esi
	movq	$0, (%rsp)
	movq	%rsp, %rdi
	cmovaeq	%rdx, %rsi
	movq	%rbx, %rdx
	callq	*%rax
	movq	(%rsp), %r14
	testl	%eax, %eax
	setne	%al
	testq	%r14, %r14
	sete	%cl
	orb	%al, %cl
	jne	.LBB4_3
	movq	memcpy@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	movq	%r12, %rsi
	movq	%rbx, %rdx
	jmp	.LBB4_11
.LBB4_4:
	testb	%al, %al
	je	.LBB4_7
.LBB4_5:
	movq	realloc@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	movq	%rbx, %rsi
	callq	*%rax
	movq	%rax, %r14
	testq	%rax, %rax
	jne	.LBB4_12
	jmp	.LBB4_3
.LBB4_7:
	movq	posix_memalign@GOTPCREL(%rip), %rax
	cmpq	$9, %rdx
	movl	$8, %esi
	movq	$0, (%rsp)
	movq	%rsp, %rdi
	cmovaeq	%rdx, %rsi
	movq	%rbx, %rdx
	callq	*%rax
	movq	(%rsp), %r14
	testl	%eax, %eax
	setne	%al
	testq	%r14, %r14
	sete	%cl
	orb	%al, %cl
	je	.LBB4_10
.LBB4_3:
	xorl	%r14d, %r14d
	jmp	.LBB4_29
.LBB4_10:
	movq	memcpy@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	movq	%r12, %rsi
	movq	%r15, %rdx
.LBB4_11:
	callq	*%rax
	movq	free@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	callq	*%rax
.LBB4_12:
	leaq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TLSLD(%rip), %rdi
	callq	__tls_get_addr@PLT
	movabsq	$9223372036854775807, %rsi
	cmpq	%rsi, %r15
	cmovaeq	%rsi, %r15
	movq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax), %rdi
	movq	%rax, %rcx
	movq	%rax, %rdx
	xorl	%eax, %eax
	cmpq	%r15, %rdi
	setns	%al
	addq	%rsi, %rax
	subq	%r15, %rdi
	cmovoq	%rax, %rdi
	movq	%rcx, %rax
	movq	%rdi, _RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rdx)
	cmpq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rcx), %rdi
	jge	.LBB4_14
	leaq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax), %rax
	movq	%rdi, (%rax)
.LBB4_14:
	leaq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rdx), %rdx
	.p2align	4
.LBB4_15:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB4_21
	leaq	1(%rax), %rdi
	lock		cmpxchgq	%rdi, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB4_15
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	negq	%rdi
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r8
	lock		xaddq	%rdi, (%rax)
	xorl	%eax, %eax
	cmpq	%r15, %rdi
	setns	%al
	addq	%rsi, %rax
	subq	%r15, %rdi
	cmovoq	%rax, %rdi
	movq	(%r8), %rax
	.p2align	4
.LBB4_18:
	cmpq	%rax, %rdi
	jge	.LBB4_20
	lock		cmpxchgq	%rdi, (%r8)
	jne	.LBB4_18
.LBB4_20:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB4_21:
	movq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rcx), %rdi
	movq	$-1, %r8
	movq	%rcx, %rax
	incq	%rdi
	cmoveq	%r8, %rdi
	movq	%rdi, _RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rcx)
	movq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rcx), %rdi
	addq	%rbx, %rdi
	cmovbq	%r8, %rdi
	movq	(%rdx), %r8
	cmpq	%rsi, %rbx
	movq	%rdi, _RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rcx)
	movq	%rsi, %rdi
	cmovbq	%rbx, %rdi
	addq	%rdi, %r8
	cmovoq	%rsi, %r8
	movq	%r8, (%rdx)
	cmpq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rcx), %r8
	jle	.LBB4_23
	leaq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax), %rax
	movq	%r8, (%rax)
	.p2align	4
.LBB4_23:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB4_29
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB4_23
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdx
	lock		incq	(%rax)
	lock		addq	%rbx, (%rcx)
	movq	%rdi, %rcx
	lock		xaddq	%rcx, (%rdx)
	movabsq	$-9223372036854775808, %rdx
	leaq	(%rcx,%rdi), %rax
	sarq	$63, %rax
	xorq	%rax, %rdx
	addq	%rdi, %rcx
	cmovoq	%rdx, %rcx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	movq	(%rdx), %rax
	.p2align	4
.LBB4_26:
	cmpq	%rax, %rcx
	jle	.LBB4_28
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB4_26
.LBB4_28:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB4_29:
	movq	%r14, %rax
	addq	$8, %rsp
	.cfi_def_cfa_offset 40
	popq	%rbx
	.cfi_def_cfa_offset 32
	popq	%r12
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end4:
	.size	_RNvXCs2911K1BwAFx_18purrdf_alloc_probeNtB2_17CountingAllocatorNtNtNtCs2k2z8Zem4rB_4core5alloc6global11GlobalAlloc7realloc, .Lfunc_end4-_RNvXCs2911K1BwAFx_18purrdf_alloc_probeNtB2_17CountingAllocatorNtNtNtCs2k2z8Zem4rB_4core5alloc6global11GlobalAlloc7realloc
	.cfi_endproc

	.section	.text._RNvXs0_Cs2911K1BwAFx_18purrdf_alloc_probeNtB5_19CurrentThreadWindowNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop,"ax",@progbits
	.globl	_RNvXs0_Cs2911K1BwAFx_18purrdf_alloc_probeNtB5_19CurrentThreadWindowNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop
	.prefalign	4, .Lfunc_end5, nop
	.type	_RNvXs0_Cs2911K1BwAFx_18purrdf_alloc_probeNtB5_19CurrentThreadWindowNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop,@function
_RNvXs0_Cs2911K1BwAFx_18purrdf_alloc_probeNtB5_19CurrentThreadWindowNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop:
.Lfunc_begin5:
	.cfi_startproc
	pushq	%rax
	.cfi_def_cfa_offset 16
	leaq	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_WINDOW_OPEN0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TLSLD(%rip), %rdi
	callq	__tls_get_addr@PLT
	movb	$0, _RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_WINDOW_OPEN0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@DTPOFF(%rax)
	popq	%rax
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end5:
	.size	_RNvXs0_Cs2911K1BwAFx_18purrdf_alloc_probeNtB5_19CurrentThreadWindowNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop, .Lfunc_end5-_RNvXs0_Cs2911K1BwAFx_18purrdf_alloc_probeNtB5_19CurrentThreadWindowNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop
	.cfi_endproc

	.hidden	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601
	.type	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601,@object
	.section	.bss._RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601,"aw",@nobits
	.globl	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601
	.p2align	3, 0x0
_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601:
	.zero	8
	.size	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601, 8

	.type	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES,@object
	.section	.bss._RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES,"aw",@nobits
	.globl	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES
	.p2align	3, 0x0
_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES:
	.zero	8
	.size	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES, 8

	.type	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES,@object
	.section	.bss._RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES,"aw",@nobits
	.globl	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES
	.p2align	3, 0x0
_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES:
	.zero	8
	.size	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES, 8

	.type	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS,@object
	.section	.bss._RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS,"aw",@nobits
	.globl	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS
	.p2align	3, 0x0
_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS:
	.zero	8
	.size	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS, 8

	.type	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES,@object
	.section	.bss._RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES,"aw",@nobits
	.globl	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES
	.p2align	3, 0x0
_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES:
	.zero	8
	.size	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES, 8

	.type	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES,@object
	.section	.bss._RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES,"aw",@nobits
	.globl	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES
	.p2align	3, 0x0
_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES:
	.zero	8
	.size	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES, 8

	.hidden	anon.030a113babdea385b7b535f4f2fa1758.3.llvm.6551227014246703601
	.type	anon.030a113babdea385b7b535f4f2fa1758.3.llvm.6551227014246703601,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
	.globl	anon.030a113babdea385b7b535f4f2fa1758.3.llvm.6551227014246703601
anon.030a113babdea385b7b535f4f2fa1758.3.llvm.6551227014246703601:
	.asciz	"/proc/self/cwd/src/lib.rs"
	.size	anon.030a113babdea385b7b535f4f2fa1758.3.llvm.6551227014246703601, 26

	.hidden	anon.030a113babdea385b7b535f4f2fa1758.5.llvm.6551227014246703601
	.type	anon.030a113babdea385b7b535f4f2fa1758.5.llvm.6551227014246703601,@object
	.section	.rodata.anon.030a113babdea385b7b535f4f2fa1758.5.llvm.6551227014246703601,"a",@progbits
	.globl	anon.030a113babdea385b7b535f4f2fa1758.5.llvm.6551227014246703601
anon.030a113babdea385b7b535f4f2fa1758.5.llvm.6551227014246703601:
	.ascii	"a CurrentThreadWindow is already open on this thread"
	.size	anon.030a113babdea385b7b535f4f2fa1758.5.llvm.6551227014246703601, 52

	.hidden	anon.030a113babdea385b7b535f4f2fa1758.6.llvm.6551227014246703601
	.type	anon.030a113babdea385b7b535f4f2fa1758.6.llvm.6551227014246703601,@object
	.section	.data.rel.ro.anon.030a113babdea385b7b535f4f2fa1758.6.llvm.6551227014246703601,"aw",@progbits
	.globl	anon.030a113babdea385b7b535f4f2fa1758.6.llvm.6551227014246703601
	.p2align	3, 0x0
anon.030a113babdea385b7b535f4f2fa1758.6.llvm.6551227014246703601:
	.quad	anon.030a113babdea385b7b535f4f2fa1758.3.llvm.6551227014246703601
	.asciz	"\031\000\000\000\000\000\000\000\363\001\000\000\t\000\000"
	.size	anon.030a113babdea385b7b535f4f2fa1758.6.llvm.6551227014246703601, 24

	.hidden	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601
	.type	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601,@object
	.section	.tbss._RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601,"awT",@nobits
	.globl	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601
	.p2align	3, 0x0
_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601:
	.zero	8
	.size	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601, 8

	.hidden	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601
	.type	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601,@object
	.section	.tbss._RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601,"awT",@nobits
	.globl	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601
	.p2align	3, 0x0
_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601:
	.zero	8
	.size	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601, 8

	.hidden	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601
	.type	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601,@object
	.section	.tbss._RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601,"awT",@nobits
	.globl	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601
	.p2align	3, 0x0
_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601:
	.zero	8
	.size	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601, 8

	.hidden	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_WINDOW_OPEN0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601
	.type	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_WINDOW_OPEN0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601,@object
	.section	.tbss._RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_WINDOW_OPEN0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601,"awT",@nobits
	.globl	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_WINDOW_OPEN0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601
_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_WINDOW_OPEN0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601:
	.zero	1
	.size	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_WINDOW_OPEN0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601, 1

	.hidden	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601
	.type	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601,@object
	.section	.tbss._RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601,"awT",@nobits
	.globl	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601
	.p2align	3, 0x0
_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601:
	.zero	8
	.size	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601, 8

	.hidden	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601
	.type	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601,@object
	.section	.tbss._RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601,"awT",@nobits
	.globl	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601
	.p2align	3, 0x0
_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601:
	.zero	8
	.size	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601, 8

	.ident	"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"
	.section	".note.GNU-stack","",@progbits
