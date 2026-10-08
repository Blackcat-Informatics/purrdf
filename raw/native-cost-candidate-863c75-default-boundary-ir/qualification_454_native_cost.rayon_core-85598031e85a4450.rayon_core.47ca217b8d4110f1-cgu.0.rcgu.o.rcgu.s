	.att_syntax
	.file	"rayon_core.47ca217b8d4110f1-cgu.0"
	.section	.text.unlikely._RINvMs0_NtNtNtNtCs7jcFBdfocI9_3std3sys12thread_local6native4lazyINtB6_7StorageNtNtCs18aJq3QiqAb_15crossbeam_epoch9collector11LocalHandleuE16get_or_init_slowNvNvNtB1i_7default6HANDLE27___rust_std_internal_init_fnECs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.prefalign	4, .Lfunc_end0, nop
	.type	_RINvMs0_NtNtNtNtCs7jcFBdfocI9_3std3sys12thread_local6native4lazyINtB6_7StorageNtNtCs18aJq3QiqAb_15crossbeam_epoch9collector11LocalHandleuE16get_or_init_slowNvNvNtB1i_7default6HANDLE27___rust_std_internal_init_fnECs6a8jV7kq6PJ_10rayon_core,@function
_RINvMs0_NtNtNtNtCs7jcFBdfocI9_3std3sys12thread_local6native4lazyINtB6_7StorageNtNtCs18aJq3QiqAb_15crossbeam_epoch9collector11LocalHandleuE16get_or_init_slowNvNvNtB1i_7default6HANDLE27___rust_std_internal_init_fnECs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin0:
	.cfi_startproc
	pushq	%rbx
	.cfi_def_cfa_offset 16
	subq	$16, %rsp
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -16
	movzbl	8(%rdi), %eax
	movq	%rdi, %rbx
	testl	%eax, %eax
	je	.LBB0_3
	cmpl	$1, %eax
	je	.LBB0_10
	xorl	%ebx, %ebx
.LBB0_10:
	movq	%rbx, %rax
	addq	$16, %rsp
	.cfi_def_cfa_offset 16
	popq	%rbx
	.cfi_def_cfa_offset 8
	retq
.LBB0_3:
	.cfi_def_cfa_offset 32
	movl	_RNvNvNtCs18aJq3QiqAb_15crossbeam_epoch7default9collector9COLLECTOR.llvm.707543514826133848+8(%rip), %eax
	testl	%eax, %eax
	jne	.LBB0_4
.LBB0_5:
	movq	_RNvMs1_NtCs18aJq3QiqAb_15crossbeam_epoch9collectorNtB5_9Collector8register@GOTPCREL(%rip), %rax
	leaq	_RNvNvNtCs18aJq3QiqAb_15crossbeam_epoch7default9collector9COLLECTOR.llvm.707543514826133848(%rip), %rdi
	callq	*%rax
	movzbl	8(%rbx), %ecx
	movq	(%rbx), %rdi
	movq	%rax, (%rbx)
	movb	$1, 8(%rbx)
	cmpl	$1, %ecx
	jne	.LBB0_6
	movq	2080(%rdi), %rax
	leaq	-1(%rax), %rcx
	xorq	$1, %rax
	movq	%rcx, 2080(%rdi)
	orq	2072(%rdi), %rax
	jne	.LBB0_10
	movq	_RNvMs6_NtCs18aJq3QiqAb_15crossbeam_epoch8internalNtB5_5Local8finalize@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	%rbx, %rax
	addq	$16, %rsp
	.cfi_def_cfa_offset 16
	popq	%rbx
	.cfi_def_cfa_offset 8
	retq
.LBB0_6:
	.cfi_def_cfa_offset 32
	testl	%ecx, %ecx
	jne	.LBB0_11
	movq	_RNvNtNtNtNtCs7jcFBdfocI9_3std3sys12thread_local11destructors10linux_like8register@GOTPCREL(%rip), %rax
	leaq	_RINvNtNtNtNtCs7jcFBdfocI9_3std3sys12thread_local6native4lazy7destroyNtNtCs18aJq3QiqAb_15crossbeam_epoch9collector11LocalHandleECs6a8jV7kq6PJ_10rayon_core(%rip), %rsi
	movq	%rbx, %rdi
	callq	*%rax
	movq	%rbx, %rax
	addq	$16, %rsp
	.cfi_def_cfa_offset 16
	popq	%rbx
	.cfi_def_cfa_offset 8
	retq
.LBB0_4:
	.cfi_def_cfa_offset 32
	callq	_RINvMs0_NtNtCs18aJq3QiqAb_15crossbeam_epoch4sync9once_lockINtB6_8OnceLockNtNtBa_9collector9CollectorE10initializeNvMs1_B1b_B19_3newEBa_.llvm.707543514826133848
	jmp	.LBB0_5
.LBB0_11:
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.6(%rip), %rsi
	leaq	15(%rsp), %rdi
	movl	$87, %edx
	callq	_RNvYNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrNtNtNtCs2k2z8Zem4rB_4core2io5write5Write9write_fmtCs6a8jV7kq6PJ_10rayon_core
	movq	%rax, %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultuNtNtNtB4_2io5error5ErrorEECs6a8jV7kq6PJ_10rayon_core
	movq	_RNvNtCs7jcFBdfocI9_3std7process5abort@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end0:
	.size	_RINvMs0_NtNtNtNtCs7jcFBdfocI9_3std3sys12thread_local6native4lazyINtB6_7StorageNtNtCs18aJq3QiqAb_15crossbeam_epoch9collector11LocalHandleuE16get_or_init_slowNvNvNtB1i_7default6HANDLE27___rust_std_internal_init_fnECs6a8jV7kq6PJ_10rayon_core, .Lfunc_end0-_RINvMs0_NtNtNtNtCs7jcFBdfocI9_3std3sys12thread_local6native4lazyINtB6_7StorageNtNtCs18aJq3QiqAb_15crossbeam_epoch9collector11LocalHandleuE16get_or_init_slowNvNvNtB1i_7default6HANDLE27___rust_std_internal_init_fnECs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc

	.section	.text._RINvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB6_8Registry3newNtB6_12DefaultSpawnEB8_,"ax",@progbits
	.prefalign	4, .Lfunc_end1, nop
	.type	_RINvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB6_8Registry3newNtB6_12DefaultSpawnEB8_,@function
_RINvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB6_8Registry3newNtB6_12DefaultSpawnEB8_:
.Lfunc_begin1:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception0
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r15
	pushq	%r14
	pushq	%r13
	pushq	%r12
	pushq	%rbx
	andq	$-128, %rsp
	subq	$1280, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	16(%rsi), %r12
	movq	%rdi, %r15
	movq	%rsi, 208(%rsp)
	testq	%r12, %r12
	je	.LBB1_40
.LBB1_1:
	movq	208(%rsp), %rax
	cmpq	$65535, %r12
	movl	$65535, %r13d
	movq	$0, 112(%rsp)
	movq	$8, 120(%rsp)
	vxorps	%xmm0, %xmm0, %xmm0
	vmovups	%xmm0, 128(%rsp)
	movq	$8, 144(%rsp)
	movq	$0, 152(%rsp)
	movq	%r12, 240(%rsp)
	movq	%r15, 216(%rsp)
	cmovbq	%r12, %r13
	movq	%r13, 80(%rsp)
	movzbl	89(%rax), %eax
	movb	%al, 88(%rsp)
	movl	$8, %eax
	movq	%rax, 200(%rsp)
	testq	%r12, %r12
	je	.LBB1_52
.Ltmp11:
	leaq	112(%rsp), %rdi
	movl	$32, %ecx
	xorl	%esi, %esi
	movq	%r13, %rdx
	callq	_RINvNvMs2_NtCsc70TAahYccp_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs6a8jV7kq6PJ_10rayon_core
.Ltmp12:
	movq	136(%rsp), %rax
	movq	152(%rsp), %rbx
	subq	%rbx, %rax
	cmpq	%rax, %r13
	ja	.LBB1_270
.LBB1_4:
	movq	120(%rsp), %rax
	movq	144(%rsp), %rcx
	movq	128(%rsp), %r15
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES@GOTPCREL(%rip), %r14
	xorl	%r12d, %r12d
	movq	%rax, 96(%rsp)
	movq	%rcx, 104(%rsp)
	.p2align	4
.LBB1_5:
	cmpb	$0, 88(%rsp)
	je	.LBB1_8
.Ltmp23:
	leaq	384(%rsp), %rdi
	vzeroupper
	callq	_RNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE8new_fifoBZ_
.Ltmp24:
	movq	384(%rsp), %rcx
	jmp	.LBB1_37
	.p2align	4
.LBB1_8:
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$1024, %edi
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB1_275
	movq	%rax, %r13
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	$-1, %rdx
	movl	$1024, %ecx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	addq	%rcx, %rax
	cmovbq	%rdx, %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	addq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jle	.LBB1_11
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB1_11:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1_17
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB1_11
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdx
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	$1024, (%rcx)
	movl	$1024, %ecx
	lock		xaddq	%rcx, (%rdx)
	addq	$1024, %rcx
	cmovoq	%rax, %rcx
	movq	(%r14), %rax
	.p2align	4
.LBB1_14:
	cmpq	%rax, %rcx
	jle	.LBB1_16
	lock		cmpxchgq	%rcx, (%r14)
	jne	.LBB1_14
.LBB1_16:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB1_17:
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$16, %edi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB1_274
	movq	%rax, %rcx
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	$-1, %rdx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	addq	$16, %rax
	cmovbq	%rdx, %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	addq	$16, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jle	.LBB1_20
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB1_20:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1_26
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB1_20
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	$16, (%rdx)
	movl	$16, %edx
	lock		xaddq	%rdx, (%rsi)
	addq	$16, %rdx
	cmovoq	%rax, %rdx
	movq	(%r14), %rax
	.p2align	4
.LBB1_23:
	cmpq	%rax, %rdx
	jle	.LBB1_25
	lock		cmpxchgq	%rdx, (%r14)
	jne	.LBB1_23
.LBB1_25:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB1_26:
	leaq	640(%rsp), %rax
	movq	$1, 384(%rsp)
	movq	$1, 392(%rsp)
	movq	%rcx, 512(%rsp)
	vxorps	%xmm0, %xmm0, %xmm0
	movl	$128, %esi
	movl	$384, %edx
	leaq	256(%rsp), %rdi
	movq	%r13, (%rcx)
	movq	$64, 8(%rcx)
	movq	$0, 256(%rsp)
	vmovaps	%xmm0, (%rax)
	movq	posix_memalign@GOTPCREL(%rip), %rax
	callq	*%rax
	testl	%eax, %eax
	jne	.LBB1_276
	movq	256(%rsp), %rcx
	testq	%rcx, %rcx
	je	.LBB1_276
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	$-1, %rdx
	movl	$384, %esi
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	addq	%rsi, %rax
	cmovbq	%rdx, %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	addq	%rsi, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jle	.LBB1_30
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB1_30:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1_36
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB1_30
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	$384, (%rdx)
	movl	$384, %edx
	lock		xaddq	%rdx, (%rsi)
	addq	$384, %rdx
	cmovoq	%rax, %rdx
	movq	(%r14), %rax
	.p2align	4
.LBB1_33:
	cmpq	%rax, %rdx
	jle	.LBB1_35
	lock		cmpxchgq	%rdx, (%r14)
	jne	.LBB1_33
.LBB1_35:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB1_36:
	vmovaps	704(%rsp), %zmm0
	vmovaps	%zmm0, 320(%rcx)
	vmovaps	640(%rsp), %zmm0
	vmovaps	%zmm0, 256(%rcx)
	vmovaps	384(%rsp), %zmm0
	vmovaps	448(%rsp), %zmm1
	vmovaps	512(%rsp), %zmm2
	vmovaps	576(%rsp), %zmm3
	vmovaps	%zmm3, 192(%rcx)
	vmovaps	%zmm2, 128(%rcx)
	vmovaps	%zmm1, 64(%rcx)
	vmovaps	%zmm0, (%rcx)
	movq	%rcx, 384(%rsp)
	movq	%r13, 392(%rsp)
	movq	80(%rsp), %r13
	movq	$64, 400(%rsp)
	movb	$1, 408(%rsp)
.LBB1_37:
	lock		incq	(%rcx)
	jle	.LBB1_283
	vmovups	384(%rsp), %ymm0
	movq	96(%rsp), %rsi
	movzbl	408(%rsp), %eax
	movq	104(%rsp), %rdi
	movq	%r15, %rdx
	shlq	$5, %rdx
	incq	%r12
	incq	%r15
	vmovaps	%ymm0, 256(%rsp)
	vmovups	%ymm0, (%rsi,%rdx)
	movq	%rbx, %rdx
	shlq	$4, %rdx
	incq	%rbx
	movq	%rcx, (%rdi,%rdx)
	movb	%al, 8(%rdi,%rdx)
	cmpq	%r13, %r12
	jne	.LBB1_5
	movq	%r15, 128(%rsp)
	movq	%rbx, 152(%rsp)
	movq	240(%rsp), %r12
	movq	112(%rsp), %r13
	movq	120(%rsp), %r14
	jmp	.LBB1_53
.LBB1_40:
.Ltmp0:
	movq	_RNvNvNtCs7jcFBdfocI9_3std3env3var5inner@GOTPCREL(%rip), %rbx
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.32(%rip), %rsi
	leaq	384(%rsp), %rdi
	movl	$17, %edx
	callq	*%rbx
.Ltmp1:
	cmpl	$1, 384(%rsp)
	jne	.LBB1_78
	movq	392(%rsp), %rcx
	testq	%rcx, %rcx
	jle	.LBB1_222
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	400(%rsp), %rdi
	xorl	%edx, %edx
	movabsq	$9223372036854775807, %rsi
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB1_45
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB1_45:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1_51
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB1_45
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB1_48:
	cmpq	%rax, %rdx
	jge	.LBB1_50
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1_48
.LBB1_50:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB1_51:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	jmp	.LBB1_222
.LBB1_52:
	xorl	%r13d, %r13d
	movl	$8, %r14d
	xorl	%r15d, %r15d
.LBB1_53:
	vmovups	136(%rsp), %xmm0
	movq	152(%rsp), %rax
	movq	$0, 384(%rsp)
	movq	$8, 392(%rsp)
	movq	%r14, 232(%rsp)
	movq	%r13, 104(%rsp)
	movq	%rax, 352(%rsp)
	vmovaps	%xmm0, 336(%rsp)
	vxorps	%xmm0, %xmm0, %xmm0
	vmovups	%xmm0, 400(%rsp)
	movq	$8, 416(%rsp)
	movq	$0, 424(%rsp)
	testq	%r12, %r12
	je	.LBB1_61
.Ltmp26:
	movq	80(%rsp), %r12
	leaq	384(%rsp), %rdi
	movl	$32, %ecx
	xorl	%esi, %esi
	movq	%r12, %rdx
	vzeroupper
	callq	_RINvNvMs2_NtCsc70TAahYccp_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs6a8jV7kq6PJ_10rayon_core
.Ltmp27:
	movq	408(%rsp), %rax
	movq	424(%rsp), %rbx
	subq	%rbx, %rax
	cmpq	%rax, %r12
	ja	.LBB1_272
.LBB1_56:
	movq	400(%rsp), %r14
	movq	416(%rsp), %rax
	movq	392(%rsp), %rdx
	movq	80(%rsp), %r13
	movq	%rbx, %rcx
	shlq	$4, %rcx
	xorl	%r12d, %r12d
	leaq	8(%rcx,%rax), %rax
	movq	%r14, %rcx
	shlq	$5, %rcx
	movq	%rdx, 200(%rsp)
	addq	%rdx, %rcx
	movq	%rax, 88(%rsp)
	movq	%rcx, 96(%rsp)
	.p2align	4
.LBB1_57:
.Ltmp31:
	leaq	112(%rsp), %rdi
	vzeroupper
	callq	_RNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE8new_fifoBZ_
.Ltmp32:
	movq	112(%rsp), %rax
	lock		incq	(%rax)
	jle	.LBB1_283
	vmovups	112(%rsp), %ymm0
	movzbl	136(%rsp), %ecx
	movq	96(%rsp), %rdx
	movq	88(%rsp), %rsi
	incq	%r14
	incq	%rbx
	vmovaps	%ymm0, 256(%rsp)
	vmovups	%ymm0, (%rdx,%r12,2)
	movq	%rax, -8(%rsi,%r12)
	movb	%cl, (%rsi,%r12)
	addq	$16, %r12
	decq	%r13
	jne	.LBB1_57
	movq	%r14, 400(%rsp)
	movq	%rbx, 424(%rsp)
	movq	384(%rsp), %rax
	movq	%rax, 96(%rsp)
	jmp	.LBB1_62
.LBB1_61:
	movq	$0, 96(%rsp)
	xorl	%r14d, %r14d
.LBB1_62:
	movq	424(%rsp), %rax
	vmovups	408(%rsp), %xmm0
	movq	352(%rsp), %r12
	movq	336(%rsp), %rcx
	movq	%rax, 304(%rsp)
	movq	344(%rsp), %rax
	movq	%rcx, 224(%rsp)
	vmovaps	%xmm0, 288(%rsp)
	movq	%rax, 88(%rsp)
	movq	%r12, %rax
	shlq	$4, %rax
	leaq	(%rax,%rax,2), %r13
	movabsq	$192153584101141162, %rax
	cmpq	%rax, %r12
	jbe	.LBB1_65
	xorl	%edi, %edi
.LBB1_64:
.Ltmp80:
	movq	_RNvNtCsc70TAahYccp_5alloc7raw_vec12handle_error@GOTPCREL(%rip), %rax
	movq	%r13, %rsi
	vzeroupper
	callq	*%rax
.Ltmp81:
	jmp	.LBB1_283
.LBB1_65:
	testq	%r13, %r13
	je	.LBB1_83
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB1_281
	movq	%rax, %rbx
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	$-1, %rcx
	movabsq	$9223372036854775807, %rdx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	addq	%r13, %rax
	cmovbq	%rcx, %rax
	cmpq	%rdx, %r13
	movq	%rdx, %rcx
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	cmovbq	%r13, %rcx
	addq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jle	.LBB1_69
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB1_69:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1_75
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB1_69
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	%r13, (%rdx)
	movq	%rcx, %rdx
	lock		xaddq	%rdx, (%rsi)
	movabsq	$-9223372036854775808, %rsi
	leaq	(%rdx,%rcx), %rax
	sarq	$63, %rax
	xorq	%rax, %rsi
	addq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rsi, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB1_72:
	cmpq	%rax, %rdx
	jle	.LBB1_74
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1_72
.LBB1_74:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB1_75:
	movq	%r12, %r9
	testq	%r12, %r12
	je	.LBB1_84
.LBB1_76:
	decq	%r12
	movb	$60, %al
	bzhiq	%rax, %r12, %rcx
	leaq	1(%rcx), %rdx
	movl	%edx, %eax
	andl	$7, %eax
	cmpq	$7, %rcx
	jae	.LBB1_85
	movq	88(%rsp), %rcx
	xorl	%r13d, %r13d
	jmp	.LBB1_88
.LBB1_78:
	movq	392(%rsp), %rcx
	cmpq	$-1, %rcx
	je	.LBB1_222
	movq	400(%rsp), %rdi
	movq	408(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB1_192
	cmpq	$1, %rsi
	jne	.LBB1_176
	movzbl	(%rdi), %eax
	xorl	%r14d, %r14d
	cmpl	$43, %eax
	je	.LBB1_207
	cmpl	$45, %eax
	je	.LBB1_207
	jmp	.LBB1_177
.LBB1_83:
	movl	$8, %ebx
	xorl	%r9d, %r9d
	testq	%r12, %r12
	jne	.LBB1_76
.LBB1_84:
	xorl	%r13d, %r13d
	jmp	.LBB1_91
.LBB1_85:
	movq	88(%rsp), %rcx
	andq	$-8, %rdx
	xorl	%r13d, %r13d
	movq	%rbx, %rsi
	.p2align	4
.LBB1_86:
	movq	(%rcx), %rdi
	movzbl	8(%rcx), %r8d
	addq	$8, %r13
	movq	%rdi, (%rsi)
	movb	%r8b, 8(%rsi)
	movq	$0, 16(%rsi)
	movq	$0, 22(%rsi)
	movq	$0, 32(%rsi)
	movw	$0, 40(%rsi)
	movl	$0, 44(%rsi)
	movq	16(%rcx), %rdi
	movzbl	24(%rcx), %r8d
	movq	%rdi, 48(%rsi)
	movb	%r8b, 56(%rsi)
	movq	$0, 64(%rsi)
	movq	$0, 70(%rsi)
	movq	$0, 80(%rsi)
	movw	$0, 88(%rsi)
	movl	$0, 92(%rsi)
	movq	32(%rcx), %rdi
	movzbl	40(%rcx), %r8d
	movq	%rdi, 96(%rsi)
	movb	%r8b, 104(%rsi)
	movq	$0, 112(%rsi)
	movq	$0, 118(%rsi)
	movq	$0, 128(%rsi)
	movw	$0, 136(%rsi)
	movl	$0, 140(%rsi)
	movq	48(%rcx), %rdi
	movzbl	56(%rcx), %r8d
	movq	%rdi, 144(%rsi)
	movb	%r8b, 152(%rsi)
	movq	$0, 160(%rsi)
	movq	$0, 166(%rsi)
	movq	$0, 176(%rsi)
	movw	$0, 184(%rsi)
	movl	$0, 188(%rsi)
	movq	64(%rcx), %rdi
	movzbl	72(%rcx), %r8d
	movq	%rdi, 192(%rsi)
	movb	%r8b, 200(%rsi)
	movq	$0, 208(%rsi)
	movq	$0, 214(%rsi)
	movq	$0, 224(%rsi)
	movw	$0, 232(%rsi)
	movl	$0, 236(%rsi)
	movq	80(%rcx), %rdi
	movzbl	88(%rcx), %r8d
	movq	%rdi, 240(%rsi)
	movb	%r8b, 248(%rsi)
	movq	$0, 256(%rsi)
	movq	$0, 262(%rsi)
	movq	$0, 272(%rsi)
	movw	$0, 280(%rsi)
	movl	$0, 284(%rsi)
	movq	96(%rcx), %rdi
	movzbl	104(%rcx), %r8d
	movq	%rdi, 288(%rsi)
	movb	%r8b, 296(%rsi)
	movq	$0, 304(%rsi)
	movq	$0, 310(%rsi)
	movq	$0, 320(%rsi)
	movw	$0, 328(%rsi)
	movl	$0, 332(%rsi)
	movq	112(%rcx), %rdi
	movzbl	120(%rcx), %r8d
	subq	$-128, %rcx
	movq	%rdi, 336(%rsi)
	movb	%r8b, 344(%rsi)
	movq	$0, 352(%rsi)
	movq	$0, 358(%rsi)
	movq	$0, 368(%rsi)
	movw	$0, 376(%rsi)
	movl	$0, 380(%rsi)
	addq	$384, %rsi
	cmpq	%r13, %rdx
	jne	.LBB1_86
	testq	%rax, %rax
	je	.LBB1_91
.LBB1_88:
	leaq	(%r13,%r13,2), %rdx
	negq	%rax
	xorl	%esi, %esi
	shlq	$4, %rdx
	addq	%rbx, %rdx
	.p2align	4
.LBB1_89:
	movq	(%rcx), %rdi
	movzbl	8(%rcx), %r8d
	decq	%rsi
	addq	$16, %rcx
	movq	%rdi, (%rdx)
	movb	%r8b, 8(%rdx)
	movq	$0, 16(%rdx)
	movq	$0, 22(%rdx)
	movq	$0, 32(%rdx)
	movw	$0, 40(%rdx)
	movl	$0, 44(%rdx)
	addq	$48, %rdx
	cmpq	%rsi, %rax
	jne	.LBB1_89
	subq	%rsi, %r13
.LBB1_91:
	movq	224(%rsp), %rdi
	testq	%rdi, %rdi
	je	.LBB1_101
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	shlq	$4, %rdi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rdx, %rdi
	cmovaeq	%rdx, %rdi
	xorl	%ecx, %ecx
	movq	%rdi, %rsi
	cmpq	%rdi, %rax
	setns	%cl
	addq	%rdx, %rcx
	subq	%rdi, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB1_94
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB1_94:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1_100
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB1_94
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rsi, %rcx
	negq	%rcx
	movabsq	$9223372036854775807, %rdx
	lock		xaddq	%rcx, (%rax)
	xorl	%eax, %eax
	cmpq	%rsi, %rcx
	setns	%al
	addq	%rdx, %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	subq	%rsi, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB1_97:
	cmpq	%rax, %rcx
	jge	.LBB1_99
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB1_97
.LBB1_99:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB1_100:
	movq	free@GOTPCREL(%rip), %rax
	movq	88(%rsp), %rdi
	movq	%r9, %r12
	vzeroupper
	callq	*%rax
	movq	%r12, %r9
.LBB1_101:
	movq	80(%rsp), %r12
	movq	%r9, 112(%rsp)
	movq	%rbx, 120(%rsp)
	movq	%r13, 128(%rsp)
	shlq	$7, %r12
	cmpq	$0, 240(%rsp)
	je	.LBB1_114
	movq	posix_memalign@GOTPCREL(%rip), %rax
	leaq	384(%rsp), %rdi
	movl	$128, %esi
	movq	%r12, %rdx
	movq	$0, 384(%rsp)
	vzeroupper
	callq	*%rax
	testl	%eax, %eax
	jne	.LBB1_282
	movq	384(%rsp), %rdi
	testq	%rdi, %rdi
	je	.LBB1_282
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	$-1, %rcx
	movq	80(%rsp), %rsi
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	addq	%r12, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	addq	%r12, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jle	.LBB1_106
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB1_106:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1_112
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB1_106
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdx
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	%r12, (%rcx)
	movq	%r12, %rcx
	lock		xaddq	%rcx, (%rdx)
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	addq	%r12, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB1_109:
	cmpq	%rax, %rcx
	jle	.LBB1_111
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB1_109
.LBB1_111:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB1_112:
	movl	%esi, %eax
	andl	$7, %eax
	cmpq	$8, 240(%rsp)
	movq	%rdi, 88(%rsp)
	jae	.LBB1_115
	xorl	%ecx, %ecx
	jmp	.LBB1_118
.LBB1_114:
	movl	$128, %eax
	movq	%rax, 88(%rsp)
	jmp	.LBB1_120
.LBB1_115:
	movl	%esi, %edx
	andl	$65528, %edx
	leaq	904(%rdi), %rsi
	xorl	%ecx, %ecx
	.p2align	4
.LBB1_116:
	movl	$0, -904(%rsi)
	movw	$0, -900(%rsi)
	movl	$0, -896(%rsi)
	movl	$0, -776(%rsi)
	movw	$0, -772(%rsi)
	movl	$0, -768(%rsi)
	movl	$0, -648(%rsi)
	movw	$0, -644(%rsi)
	movl	$0, -640(%rsi)
	movl	$0, -520(%rsi)
	movw	$0, -516(%rsi)
	movl	$0, -512(%rsi)
	movl	$0, -392(%rsi)
	movw	$0, -388(%rsi)
	movl	$0, -384(%rsi)
	movl	$0, -264(%rsi)
	movw	$0, -260(%rsi)
	movl	$0, -256(%rsi)
	movl	$0, -136(%rsi)
	movw	$0, -132(%rsi)
	movl	$0, -128(%rsi)
	movl	$0, -8(%rsi)
	movw	$0, -4(%rsi)
	movl	$0, (%rsi)
	addq	$8, %rcx
	addq	$1024, %rsi
	cmpq	%rcx, %rdx
	jne	.LBB1_116
	testq	%rax, %rax
	je	.LBB1_120
.LBB1_118:
	shlq	$7, %rcx
	shll	$7, %eax
	xorl	%edx, %edx
	leaq	8(%rcx,%rdi), %rcx
	.p2align	4
.LBB1_119:
	movl	$0, -8(%rcx,%rdx)
	movw	$0, -4(%rcx,%rdx)
	movl	$0, (%rcx,%rdx)
	subq	$-128, %rdx
	cmpq	%rdx, %rax
	jne	.LBB1_119
.LBB1_120:
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$1520, %edi
	movl	$1520, %ebx
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB1_279
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rsi
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rdx
	movq	$-1, %rcx
	movq	%rax, %r13
	movabsq	$9223372036854775807, %rax
	incq	%rsi
	cmoveq	%rcx, %rsi
	addq	%rbx, %rdx
	cmovbq	%rcx, %rdx
	addq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rbx
	movq	%rsi, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%rdx, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmovoq	%rax, %rbx
	movq	%rbx, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rbx
	jle	.LBB1_123
	movq	%rbx, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB1_123:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1_129
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB1_123
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdx
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	$1520, (%rcx)
	movl	$1520, %ecx
	lock		xaddq	%rcx, (%rdx)
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	addq	$1520, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB1_126:
	cmpq	%rax, %rcx
	jle	.LBB1_128
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB1_126
.LBB1_128:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB1_129:
	movq	memset@GOTPCREL(%rip), %rax
	movl	$1520, %edx
	movq	%r13, %rdi
	xorl	%esi, %esi
	callq	*%rax
	vmovups	112(%rsp), %xmm0
	movq	128(%rsp), %rax
	movq	96(%rsp), %rcx
	movq	200(%rsp), %rdx
	leaq	112(%rsp), %rdi
	movl	$128, %esi
	movq	$0, 112(%rsp)
	movq	%rax, 904(%rsp)
	movq	80(%rsp), %rax
	vmovups	%xmm0, 888(%rsp)
	movq	%rcx, 776(%rsp)
	movq	208(%rsp), %rcx
	movq	%rdx, 784(%rsp)
	movq	%r14, 792(%rsp)
	movq	$1, 384(%rsp)
	movq	$1, 392(%rsp)
	movq	$0, 512(%rsp)
	movq	%r13, 520(%rsp)
	movq	$0, 640(%rsp)
	movq	%r13, 648(%rsp)
	movl	$0, 768(%rsp)
	movb	$0, 772(%rsp)
	movl	$640, %edx
	vmovups	24(%rcx), %ymm0
	movq	$0, 24(%rcx)
	vinsertf128	$1, 56(%rcx), %ymm0, %ymm0
	movq	$0, 56(%rcx)
	vmovups	72(%rcx), %xmm1
	movq	$0, 72(%rcx)
	movq	88(%rsp), %rcx
	vmovaps	%ymm0, 800(%rsp)
	vmovaps	%xmm1, 832(%rsp)
	movq	$1, 848(%rsp)
	movq	%rax, 856(%rsp)
	movq	%rcx, 864(%rsp)
	movq	%rax, 872(%rsp)
	movq	posix_memalign@GOTPCREL(%rip), %rax
	movq	$0, 880(%rsp)
	vzeroupper
	callq	*%rax
	testl	%eax, %eax
	jne	.LBB1_280
	movq	112(%rsp), %rax
	movq	%rax, 96(%rsp)
	testq	%rax, %rax
	je	.LBB1_280
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rdx
	movq	$-1, %rcx
	movq	232(%rsp), %rbx
	movq	104(%rsp), %r14
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movl	$640, %eax
	addq	%rax, %rdx
	cmovbq	%rcx, %rdx
	addq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rdx, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmovoq	%rcx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jle	.LBB1_133
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB1_133:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1_139
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB1_133
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdx
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	$640, (%rcx)
	movl	$640, %ecx
	lock		xaddq	%rcx, (%rdx)
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	addq	$640, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB1_136:
	cmpq	%rax, %rcx
	jle	.LBB1_138
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB1_136
.LBB1_138:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB1_139:
	movq	96(%rsp), %r12
	movq	memcpy@GOTPCREL(%rip), %rax
	leaq	384(%rsp), %rsi
	movl	$640, %edx
	movq	%r12, %rdi
	callq	*%rax
	movq	304(%rsp), %rsi
	movq	296(%rsp), %rax
	movq	288(%rsp), %rcx
	movq	%r15, %rdx
	shlq	$5, %rdx
	vxorps	%xmm0, %xmm0, %xmm0
	vmovups	%xmm0, 176(%rsp)
	movq	%rbx, 112(%rsp)
	movq	%r12, 320(%rsp)
	movq	%r14, 128(%rsp)
	addq	%rbx, %rdx
	movq	%rdx, 136(%rsp)
	movq	%rdx, 80(%rsp)
	shlq	$4, %rsi
	movq	%rax, 144(%rsp)
	movq	%rcx, 160(%rsp)
	addq	%rax, %rsi
	movq	%rsi, 104(%rsp)
	movq	%rsi, 168(%rsp)
	testq	%r15, %r15
	je	.LBB1_168
	movq	208(%rsp), %rcx
	xorl	%r13d, %r13d
	xorl	%r14d, %r14d
	vmovups	(%rcx), %xmm0
	movzbl	88(%rcx), %edx
	movq	40(%rcx), %rsi
	movq	48(%rcx), %rcx
	movb	%dl, 224(%rsp)
	movq	%rbx, %rdx
	movq	%rsi, 200(%rsp)
	movq	%rcx, 360(%rsp)
	vmovaps	%xmm0, 240(%rsp)
	jmp	.LBB1_142
	.p2align	4
.LBB1_141:
	movq	88(%rsp), %rcx
	movq	%r15, %rdx
	movq	%rcx, %rax
	cmpq	80(%rsp), %r15
	je	.LBB1_172
.LBB1_142:
	movzbl	24(%rdx), %ecx
	leaq	32(%rdx), %r15
	cmpb	$2, %cl
	je	.LBB1_169
	movq	%rdx, %rdi
	movq	16(%rdx), %rdx
	leaq	409(%rsp), %r8
	movq	%rdx, 400(%rsp)
	vmovups	(%rdi), %xmm0
	vmovaps	%xmm0, 384(%rsp)
	movl	25(%rdi), %edx
	movl	28(%rdi), %esi
	movl	%esi, 3(%r8)
	movl	%edx, (%r8)
	movb	%cl, 408(%rsp)
	cmpq	104(%rsp), %rax
	je	.LBB1_170
	leaq	16(%rax), %rsi
	movq	(%rax), %r12
	movzbl	8(%rax), %ebx
	movq	16(%rdi), %rax
	leaq	25(%rdi), %rdx
	leaq	1(%r13), %r14
	movq	%rsi, 88(%rsp)
	leaq	281(%rsp), %rsi
	movq	%rax, 272(%rsp)
	vmovups	(%rdi), %xmm0
	vmovaps	%xmm0, 256(%rsp)
	movl	(%rdx), %eax
	movl	3(%rdx), %edx
	movq	%r12, 368(%rsp)
	movb	%bl, 376(%rsp)
	movl	%edx, 3(%rsi)
	movl	%eax, (%rsi)
	movq	200(%rsp), %rsi
	movb	%cl, 280(%rsp)
	testq	%rsi, %rsi
	je	.LBB1_147
	movq	360(%rsp), %rax
	movq	32(%rax), %rax
.Ltmp37:
	leaq	384(%rsp), %rdi
	movq	%r13, %rdx
	callq	*%rax
.Ltmp38:
	movq	96(%rsp), %rcx
	lock		incq	(%rcx)
	jg	.LBB1_148
	jmp	.LBB1_283
	.p2align	4
.LBB1_147:
	movq	$-1, 384(%rsp)
	movq	96(%rsp), %rcx
	lock		incq	(%rcx)
	jle	.LBB1_283
.LBB1_148:
	vmovups	256(%rsp), %ymm0
	vmovups	384(%rsp), %xmm2
	vmovaps	240(%rsp), %xmm1
	leaq	1176(%rsp), %rax
	leaq	1136(%rsp), %rdx
	vmovups	%ymm0, (%rax)
	movq	400(%rsp), %rax
	vmovaps	%xmm2, (%rdx)
	movq	%rax, 16(%rdx)
	vmovaps	%xmm1, 1120(%rsp)
	movq	%r12, 1160(%rsp)
	movb	%bl, 1168(%rsp)
	movq	%rcx, 1208(%rsp)
	movq	%r13, 1216(%rsp)
	movq	%rcx, %rbx
	testq	%r13, %r13
	jne	.LBB1_165
	cmpb	$0, 224(%rsp)
	je	.LBB1_165
	movq	_RNvNCNKNvNtCs6a8jV7kq6PJ_10rayon_core8registry19WORKER_THREAD_STATE0s_023___RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rax
	cmpq	$0, %fs:(%rax)
	jne	.LBB1_182
.Ltmp51:
	movq	_RNvXs6_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThreadINtNtCs2k2z8Zem4rB_4core7convert4FromNtB5_13ThreadBuilderE4from@GOTPCREL(%rip), %rax
	leaq	384(%rsp), %rdi
	leaq	1120(%rsp), %rsi
	vzeroupper
	callq	*%rax
.Ltmp52:
	movq	posix_memalign@GOTPCREL(%rip), %rax
	movl	$128, %esi
	movl	$384, %edx
	leaq	328(%rsp), %rdi
	movq	$0, 328(%rsp)
	callq	*%rax
	testl	%eax, %eax
	jne	.LBB1_277
	movq	328(%rsp), %rcx
	testq	%rcx, %rcx
	je	.LBB1_277
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	$-1, %rsi
	movl	$384, %edx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdi
	incq	%rax
	cmoveq	%rsi, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	addq	%rdx, %rax
	cmovbq	%rsi, %rax
	movabsq	$9223372036854775807, %rsi
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	addq	%rdx, %rax
	cmovoq	%rsi, %rax
	movq	_RNvNCNKNvNtCs6a8jV7kq6PJ_10rayon_core8registry19WORKER_THREAD_STATE0s_023___RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rsi
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jle	.LBB1_156
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB1_156:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1_162
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB1_156
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r8
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	$384, (%rdx)
	movl	$384, %edx
	lock		xaddq	%rdx, (%r8)
	addq	$384, %rdx
	cmovoq	%rax, %rdx
	movq	(%rdi), %rax
	.p2align	4
.LBB1_159:
	cmpq	%rax, %rdx
	jle	.LBB1_161
	lock		cmpxchgq	%rdx, (%rdi)
	jne	.LBB1_159
.LBB1_161:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB1_162:
	vmovaps	704(%rsp), %zmm0
	vmovaps	%zmm0, 320(%rcx)
	vmovaps	640(%rsp), %zmm0
	vmovaps	%zmm0, 256(%rcx)
	vmovaps	384(%rsp), %zmm0
	vmovaps	448(%rsp), %zmm1
	vmovaps	512(%rsp), %zmm2
	vmovaps	576(%rsp), %zmm3
	vmovaps	%zmm3, 192(%rcx)
	vmovaps	%zmm2, 128(%rcx)
	vmovaps	%zmm1, 64(%rcx)
	vmovaps	%zmm0, (%rcx)
	cmpq	$0, %fs:(%rsi)
	jne	.LBB1_269
	movq	%rcx, %fs:(%rsi)
	cmpq	$0, 520(%rbx)
	je	.LBB1_278
	movq	512(%rbx), %rdi
	movl	$1, %r13d
	addq	$24, %rdi
.Ltmp55:
	vzeroupper
	callq	_RNvXs4_NtCs6a8jV7kq6PJ_10rayon_core5latchNtB5_9LockLatchNtB5_5Latch3set
.Ltmp56:
	jmp	.LBB1_141
	.p2align	4
.LBB1_165:
.Ltmp40:
	movq	_RNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12DefaultSpawnNtB5_11ThreadSpawn5spawn@GOTPCREL(%rip), %rax
	leaq	1120(%rsp), %rsi
	vzeroupper
	callq	*%rax
.Ltmp41:
	movq	%r14, %r13
	testq	%rax, %rax
	je	.LBB1_141
	movq	216(%rsp), %rdx
	movq	88(%rsp), %rcx
	movq	%r15, 120(%rsp)
	movq	%rcx, 152(%rsp)
	movq	$2, (%rdx)
	movq	%r14, 192(%rsp)
	movq	%rax, 8(%rdx)
	jmp	.LBB1_196
.LBB1_168:
	xorl	%r14d, %r14d
	jmp	.LBB1_173
.LBB1_169:
	movq	%r15, %rbx
	jmp	.LBB1_173
.LBB1_170:
	movq	104(%rsp), %rax
	movq	384(%rsp), %rcx
	movq	%r15, 120(%rsp)
	movq	%rax, 152(%rsp)
	movq	%r14, 192(%rsp)
	lock		decq	(%rcx)
	movq	216(%rsp), %rbx
	jne	.LBB1_174
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %rax
	leaq	384(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
	jmp	.LBB1_174
.LBB1_172:
	movq	80(%rsp), %rbx
	movq	%rcx, %rax
.LBB1_173:
	movq	%rbx, 120(%rsp)
	movq	216(%rsp), %rbx
	movq	%rax, 152(%rsp)
	movq	%r14, 192(%rsp)
.LBB1_174:
	leaq	112(%rsp), %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtNtB4_4iter8adapters9enumerate9EnumerateINtNtBG_3zip3ZipINtNtNtCsc70TAahYccp_5alloc3vec9into_iter8IntoIterINtNtCs64OF0TycdZY_15crossbeam_deque5deque6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEIB1C_INtB2s_7StealerB3c_EEEEEB3g_
	movq	96(%rsp), %rax
	movq	%rax, 8(%rbx)
	movq	$-1, (%rbx)
.LBB1_175:
	movq	208(%rsp), %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtCs6a8jV7kq6PJ_10rayon_core17ThreadPoolBuilderEBD_
	leaq	-40(%rbp), %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB1_176:
	.cfi_def_cfa %rbp, 16
	movzbl	(%rdi), %eax
.LBB1_177:
	xorl	%r8d, %r8d
	cmpb	$43, %al
	movq	%rsi, %rdx
	sete	%r8b
	movq	%r8, %rax
	subq	%r8, %rdx
	negq	%rax
	addq	%rdi, %r8
	cmpq	$17, %rdx
	jae	.LBB1_187
	testq	%rdx, %rdx
	je	.LBB1_205
	addq	%rax, %rsi
	xorl	%r12d, %r12d
	xorl	%eax, %eax
	negq	%rsi
	.p2align	4
.LBB1_180:
	movzbl	(%r8,%rax), %edx
	addl	$-48, %edx
	cmpl	$10, %edx
	setb	%r14b
	jae	.LBB1_206
	leaq	(%r12,%r12,4), %r9
	movl	%edx, %edx
	incq	%rax
	leaq	(%rdx,%r9,2), %r12
	movq	%rsi, %rdx
	addq	%rax, %rdx
	jne	.LBB1_180
	jmp	.LBB1_207
.LBB1_182:
	movq	216(%rsp), %rcx
	movq	88(%rsp), %rax
	movq	1136(%rsp), %rsi
	movq	%r15, 120(%rsp)
	movq	%rax, 152(%rsp)
	movq	%r14, 192(%rsp)
	movq	$1, (%rcx)
	testq	%rsi, %rsi
	jle	.LBB1_184
	movq	1144(%rsp), %rdi
	movl	$1, %edx
	vzeroupper
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
.LBB1_184:
	movq	1176(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB1_185
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %rax
	leaq	1176(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
	lock		decq	(%r12)
	je	.LBB1_194
.LBB1_186:
	lock		decq	(%rbx)
	je	.LBB1_195
	jmp	.LBB1_196
.LBB1_185:
	lock		decq	(%r12)
	jne	.LBB1_186
.LBB1_194:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %rax
	leaq	1160(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
	lock		decq	(%rbx)
	jne	.LBB1_196
.LBB1_195:
	#MEMBARRIER
.Ltmp42:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryE9drop_slowBK_@GOTPCREL(%rip), %rax
	leaq	1208(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp43:
.LBB1_196:
	leaq	112(%rsp), %rdi
	vzeroupper
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtNtB4_4iter8adapters9enumerate9EnumerateINtNtBG_3zip3ZipINtNtNtCsc70TAahYccp_5alloc3vec9into_iter8IntoIterINtNtCs64OF0TycdZY_15crossbeam_deque5deque6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEIB1C_INtB2s_7StealerB3c_EEEEEB3g_
	lock		decq	464(%rbx)
	jne	.LBB1_202
	movq	96(%rsp), %rax
	movq	520(%rax), %rax
	testq	%rax, %rax
	je	.LBB1_202
	movq	96(%rsp), %rcx
	shlq	$4, %rax
	xorl	%r14d, %r14d
	xorl	%r13d, %r13d
	leaq	(%rax,%rax,2), %r12
	movq	512(%rcx), %r15
	leaq	472(%rcx), %rbx
	jmp	.LBB1_200
	.p2align	4
.LBB1_199:
	addq	$48, %r13
	incq	%r14
	cmpq	%r13, %r12
	je	.LBB1_202
.LBB1_200:
	movl	$3, %eax
	xchgq	%rax, 16(%r15,%r13)
	cmpq	$2, %rax
	jne	.LBB1_199
.Ltmp45:
	movq	%rbx, %rdi
	movq	%r14, %rsi
	callq	_RNvMNtCs6a8jV7kq6PJ_10rayon_core5sleepNtB2_5Sleep20wake_specific_thread.llvm.7294274987384275483
.Ltmp46:
	jmp	.LBB1_199
.LBB1_202:
	movq	96(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB1_175
	#MEMBARRIER
.Ltmp48:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryE9drop_slowBK_@GOTPCREL(%rip), %rax
	leaq	320(%rsp), %rdi
	callq	*%rax
.Ltmp49:
	jmp	.LBB1_175
.LBB1_187:
	addq	%rax, %rsi
	xorl	%r12d, %r12d
	movl	$10, %r9d
	xorl	%r10d, %r10d
	negq	%rsi
.LBB1_188:
	movq	%r12, %rax
	mulq	%r9
	jo	.LBB1_192
	movzbl	(%r8,%r10), %edx
	movq	%rax, %r12
	addl	$-48, %edx
	addq	%rdx, %r12
	setb	%al
	xorl	%r14d, %r14d
	cmpl	$9, %edx
	ja	.LBB1_206
	testb	%al, %al
	jne	.LBB1_207
	incq	%r10
	movq	%rsi, %rax
	movb	$1, %r14b
	addq	%r10, %rax
	jne	.LBB1_188
	jmp	.LBB1_207
.LBB1_192:
	xorl	%r14d, %r14d
.LBB1_206:
.LBB1_207:
	testq	%rcx, %rcx
	je	.LBB1_217
.LBB1_208:
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB1_210
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB1_210:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1_216
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB1_210
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB1_213:
	cmpq	%rax, %rdx
	jge	.LBB1_215
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1_213
.LBB1_215:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB1_216:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1_217:
	testb	%r14b, %r14b
	je	.LBB1_222
	testq	%r12, %r12
	jne	.LBB1_1
.Ltmp2:
	movq	_RNvNtNtCs7jcFBdfocI9_3std6thread9functions21available_parallelism@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp3:
	cmpq	$1, %rax
	je	.LBB1_253
.LBB1_221:
	movq	%rdx, %r12
	jmp	.LBB1_1
.LBB1_222:
.Ltmp4:
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.33(%rip), %rsi
	leaq	384(%rsp), %rdi
	movl	$17, %edx
	callq	*%rbx
.Ltmp5:
	cmpb	$0, 384(%rsp)
	je	.LBB1_234
	movq	392(%rsp), %rcx
	testq	%rcx, %rcx
	jle	.LBB1_251
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	400(%rsp), %rdi
	xorl	%edx, %edx
	movabsq	$9223372036854775807, %rsi
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB1_227
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB1_227:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1_233
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB1_227
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB1_230:
	cmpq	%rax, %rdx
	jge	.LBB1_232
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1_230
.LBB1_232:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB1_233:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	jmp	.LBB1_251
.LBB1_234:
	movq	392(%rsp), %rcx
	cmpq	$-1, %rcx
	je	.LBB1_251
	movq	400(%rsp), %rdi
	movq	408(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB1_239
	cmpq	$1, %rsi
	jne	.LBB1_256
	movzbl	(%rdi), %eax
	cmpl	$43, %eax
	je	.LBB1_239
	cmpl	$45, %eax
	jne	.LBB1_257
.LBB1_239:
	movb	$1, %bl
.LBB1_240:
	testq	%rcx, %rcx
	je	.LBB1_250
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB1_243
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB1_243:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1_249
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB1_243
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB1_246:
	cmpq	%rax, %rdx
	jge	.LBB1_248
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1_246
.LBB1_248:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB1_249:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1_250:
	testq	%r12, %r12
	sete	%al
	orb	%al, %bl
	cmpb	$1, %bl
	jne	.LBB1_1
.LBB1_251:
.Ltmp6:
	movq	_RNvNtNtCs7jcFBdfocI9_3std6thread9functions21available_parallelism@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp7:
	cmpq	$1, %rax
	jne	.LBB1_221
.LBB1_253:
	movl	%edx, %eax
	andl	$3, %eax
	movl	$1, %r12d
	cmpl	$1, %eax
	jne	.LBB1_1
	movq	23(%rdx), %rax
	decq	%rdx
.Ltmp8:
	movq	%rdx, %rdi
	callq	*%rax
.Ltmp9:
	jmp	.LBB1_1
.LBB1_256:
	movzbl	(%rdi), %eax
.LBB1_257:
	xorl	%r8d, %r8d
	cmpb	$43, %al
	movq	%rsi, %rdx
	sete	%r8b
	movq	%r8, %rax
	subq	%r8, %rdx
	negq	%rax
	addq	%rdi, %r8
	cmpq	$17, %rdx
	jae	.LBB1_262
	testq	%rdx, %rdx
	je	.LBB1_267
	addq	%rax, %rsi
	xorl	%r12d, %r12d
	xorl	%eax, %eax
	negq	%rsi
	.p2align	4
.LBB1_260:
	movzbl	(%r8,%rax), %edx
	addl	$-48, %edx
	cmpl	$10, %edx
	setae	%bl
	jae	.LBB1_268
	leaq	(%r12,%r12,4), %r9
	movl	%edx, %edx
	incq	%rax
	leaq	(%rdx,%r9,2), %r12
	movq	%rsi, %rdx
	addq	%rax, %rdx
	jne	.LBB1_260
	jmp	.LBB1_240
.LBB1_262:
	addq	%rax, %rsi
	xorl	%r12d, %r12d
	movl	$10, %r9d
	xorl	%r10d, %r10d
	negq	%rsi
.LBB1_263:
	movq	%r12, %rax
	mulq	%r9
	jo	.LBB1_239
	movzbl	(%r8,%r10), %edx
	movq	%rax, %r12
	addl	$-48, %edx
	addq	%rdx, %r12
	setb	%al
	cmpl	$9, %edx
	ja	.LBB1_239
	testb	%al, %al
	jne	.LBB1_239
	incq	%r10
	xorl	%ebx, %ebx
	movq	%rsi, %rax
	addq	%r10, %rax
	jne	.LBB1_263
	jmp	.LBB1_240
.LBB1_205:
	movb	$1, %r14b
	xorl	%r12d, %r12d
	testq	%rcx, %rcx
	jne	.LBB1_208
	jmp	.LBB1_217
.LBB1_267:
	xorl	%r12d, %r12d
	xorl	%ebx, %ebx
	jmp	.LBB1_240
.LBB1_268:
	jmp	.LBB1_240
.LBB1_269:
	movq	88(%rsp), %rax
	movq	%r15, 120(%rsp)
	movq	%rax, 152(%rsp)
	movq	%r14, 192(%rsp)
.Ltmp53:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking5panic@GOTPCREL(%rip), %rax
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.18(%rip), %rdi
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.19(%rip), %rdx
	movl	$35, %esi
	vzeroupper
	callq	*%rax
.Ltmp54:
	jmp	.LBB1_283
.LBB1_270:
.Ltmp13:
	leaq	136(%rsp), %rdi
	movl	$16, %ecx
	movq	%rbx, %rsi
	movq	%r13, %rdx
	callq	_RINvNvMs2_NtCsc70TAahYccp_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs6a8jV7kq6PJ_10rayon_core
.Ltmp14:
	movq	152(%rsp), %rbx
	jmp	.LBB1_4
.LBB1_272:
.Ltmp28:
	movq	80(%rsp), %rdx
	leaq	408(%rsp), %rdi
	movl	$16, %ecx
	movq	%rbx, %rsi
	callq	_RINvNvMs2_NtCsc70TAahYccp_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs6a8jV7kq6PJ_10rayon_core
.Ltmp29:
	movq	424(%rsp), %rbx
	jmp	.LBB1_56
.LBB1_274:
	movq	%r15, 128(%rsp)
	movq	%rbx, 152(%rsp)
.Ltmp18:
	movq	_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$16, %esi
	callq	*%rax
.Ltmp19:
	jmp	.LBB1_283
.LBB1_275:
	movq	%r15, 128(%rsp)
	movq	%rbx, 152(%rsp)
.Ltmp20:
	movq	_RNvNtCsc70TAahYccp_5alloc7raw_vec12handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$1024, %esi
	callq	*%rax
.Ltmp21:
	jmp	.LBB1_283
.LBB1_276:
	movq	%r15, 128(%rsp)
	movq	%rbx, 152(%rsp)
.Ltmp15:
	movq	_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$128, %edi
	movl	$384, %esi
	callq	*%rax
.Ltmp16:
	jmp	.LBB1_283
.LBB1_277:
	movq	88(%rsp), %rax
	movq	%r15, 120(%rsp)
	movq	%rax, 152(%rsp)
	movq	%r14, 192(%rsp)
.Ltmp61:
	movq	_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$128, %edi
	movl	$384, %esi
	callq	*%rax
.Ltmp62:
	jmp	.LBB1_283
.LBB1_278:
	movq	88(%rsp), %rax
	movq	%r15, 120(%rsp)
	movq	%rax, 152(%rsp)
	movq	%r14, 192(%rsp)
.Ltmp58:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking18panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.8(%rip), %rdx
	xorl	%edi, %edi
	xorl	%esi, %esi
	vzeroupper
	callq	*%rax
.Ltmp59:
	jmp	.LBB1_283
.LBB1_279:
.Ltmp77:
	movq	_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$1520, %esi
	callq	*%rax
.Ltmp78:
	jmp	.LBB1_283
.LBB1_280:
.Ltmp71:
	movq	_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$128, %edi
	movl	$640, %esi
	leaq	512(%rsp), %rbx
	callq	*%rax
.Ltmp72:
	jmp	.LBB1_283
.LBB1_281:
	movl	$8, %edi
	jmp	.LBB1_64
.LBB1_282:
.Ltmp34:
	movq	_RNvNtCsc70TAahYccp_5alloc7raw_vec12handle_error@GOTPCREL(%rip), %rax
	movl	$128, %edi
	movq	%r12, %rsi
	callq	*%rax
.Ltmp35:
.LBB1_283:
	ud2
.LBB1_284:
.Ltmp44:
	jmp	.LBB1_313
.LBB1_285:
.Ltmp50:
	jmp	.LBB1_296
.LBB1_286:
.Ltmp36:
	movq	%rax, 80(%rsp)
	jmp	.LBB1_293
.LBB1_287:
.Ltmp47:
	movq	96(%rsp), %rbx
	movq	%rax, 80(%rsp)
	jmp	.LBB1_315
.LBB1_288:
.Ltmp73:
	movq	%rax, 80(%rsp)
.Ltmp74:
	movq	%rbx, %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryEBF_
.Ltmp75:
	leaq	288(%rsp), %rdi
	jmp	.LBB1_309
.LBB1_290:
.Ltmp76:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1_291:
.Ltmp79:
	cmpq	$0, 240(%rsp)
	movq	%rax, 80(%rsp)
	je	.LBB1_293
	movq	88(%rsp), %rdi
	movl	$128, %edx
	movq	%r12, %rsi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
.LBB1_293:
	leaq	112(%rsp), %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc3vec3VecNtNtCs6a8jV7kq6PJ_10rayon_core8registry10ThreadInfoEEB1c_
	jmp	.LBB1_328
.LBB1_294:
.Ltmp30:
	movq	%rax, 80(%rsp)
	jmp	.LBB1_308
.LBB1_295:
.Ltmp10:
.LBB1_296:
	movq	%rax, 80(%rsp)
	jmp	.LBB1_362
.LBB1_297:
.Ltmp63:
	movq	%rax, 80(%rsp)
.Ltmp64:
	leaq	384(%rsp), %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry12WorkerThreadEBF_
.Ltmp65:
	jmp	.LBB1_303
.LBB1_298:
.Ltmp66:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1_299:
.Ltmp39:
	movq	88(%rsp), %rcx
	movq	%r15, 120(%rsp)
	movq	%rax, 80(%rsp)
	movq	%rcx, 152(%rsp)
	movq	%r14, 192(%rsp)
	lock		decq	(%r12)
	jne	.LBB1_301
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %rax
	leaq	368(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB1_301:
	movq	256(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB1_303
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %rax
	leaq	256(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB1_303:
	movq	96(%rsp), %rbx
	jmp	.LBB1_314
.LBB1_304:
.Ltmp57:
	movq	88(%rsp), %rcx
	movq	%r15, 120(%rsp)
	movq	%rax, 80(%rsp)
	movq	%rcx, 152(%rsp)
	movq	%r14, 192(%rsp)
	jmp	.LBB1_314
.LBB1_305:
.Ltmp17:
	leaq	384(%rsp), %rdi
	movq	%rax, 80(%rsp)
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc4sync8ArcInnerINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEEEB35_
	jmp	.LBB1_311
.LBB1_306:
.Ltmp25:
	movq	%r15, 128(%rsp)
	movq	%rax, 80(%rsp)
	movq	%rbx, 152(%rsp)
	jmp	.LBB1_311
.LBB1_307:
.Ltmp33:
	movq	%r14, 400(%rsp)
	movq	%rax, 80(%rsp)
	movq	%rbx, 424(%rsp)
.LBB1_308:
	leaq	384(%rsp), %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueTINtNtCsc70TAahYccp_5alloc3vec3VecINtNtCs64OF0TycdZY_15crossbeam_deque5deque6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEIBD_INtB1c_7StealerB1W_EEEEB20_
	leaq	336(%rsp), %rdi
.LBB1_309:
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc3vec3VecINtNtCs64OF0TycdZY_15crossbeam_deque5deque7StealerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEEB20_
	movq	104(%rsp), %r13
	jmp	.LBB1_350
.LBB1_310:
.Ltmp22:
	movq	%rax, 80(%rsp)
.LBB1_311:
	leaq	112(%rsp), %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueTINtNtCsc70TAahYccp_5alloc3vec3VecINtNtCs64OF0TycdZY_15crossbeam_deque5deque6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEIBD_INtB1c_7StealerB1W_EEEEB20_
	jmp	.LBB1_362
.LBB1_312:
.Ltmp60:
.LBB1_313:
	movq	%rax, 80(%rsp)
.LBB1_314:
	leaq	112(%rsp), %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtNtB4_4iter8adapters9enumerate9EnumerateINtNtBG_3zip3ZipINtNtNtCsc70TAahYccp_5alloc3vec9into_iter8IntoIterINtNtCs64OF0TycdZY_15crossbeam_deque5deque6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEIB1C_INtB2s_7StealerB3c_EEEEEB3g_
.Ltmp67:
	movq	%rbx, %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry10TerminatorEBF_
.Ltmp68:
.LBB1_315:
	lock		decq	(%rbx)
	jne	.LBB1_362
	#MEMBARRIER
.Ltmp69:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryE9drop_slowBK_@GOTPCREL(%rip), %rax
	leaq	320(%rsp), %rdi
	callq	*%rax
.Ltmp70:
	jmp	.LBB1_362
.LBB1_317:
.Ltmp82:
	movq	%rax, 80(%rsp)
	testq	%r12, %r12
	jne	.LBB1_376
.LBB1_318:
	movq	224(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB1_328
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	shlq	$4, %rsi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rdx, %rsi
	cmovaeq	%rdx, %rsi
	xorl	%ecx, %ecx
	cmpq	%rsi, %rax
	setns	%cl
	addq	%rdx, %rcx
	subq	%rsi, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB1_321
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB1_321:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1_327
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB1_321
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rsi, %rcx
	negq	%rcx
	movabsq	$9223372036854775807, %rdx
	lock		xaddq	%rcx, (%rax)
	xorl	%eax, %eax
	cmpq	%rsi, %rcx
	setns	%al
	addq	%rdx, %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	subq	%rsi, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB1_324:
	cmpq	%rax, %rcx
	jge	.LBB1_326
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB1_324
.LBB1_326:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB1_327:
	movq	free@GOTPCREL(%rip), %rax
	movq	88(%rsp), %rdi
	callq	*%rax
.LBB1_328:
	movq	296(%rsp), %rax
	movq	304(%rsp), %r13
	movq	%rax, 88(%rsp)
	testq	%r13, %r13
	jne	.LBB1_368
.LBB1_329:
	movq	288(%rsp), %rcx
	movq	104(%rsp), %r13
	testq	%rcx, %rcx
	je	.LBB1_339
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	shlq	$4, %rcx
	movabsq	$9223372036854775807, %rsi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB1_332
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB1_332:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1_338
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB1_332
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB1_335:
	cmpq	%rax, %rdx
	jge	.LBB1_337
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1_335
.LBB1_337:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB1_338:
	movq	free@GOTPCREL(%rip), %rax
	movq	88(%rsp), %rdi
	callq	*%rax
.LBB1_339:
	testq	%r14, %r14
	jne	.LBB1_372
.LBB1_340:
	movq	96(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB1_350
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	shlq	$5, %rsi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rdx, %rsi
	cmovaeq	%rdx, %rsi
	xorl	%ecx, %ecx
	cmpq	%rsi, %rax
	setns	%cl
	addq	%rdx, %rcx
	subq	%rsi, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB1_343
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB1_343:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1_349
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB1_343
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rsi, %rcx
	negq	%rcx
	movabsq	$9223372036854775807, %rdx
	lock		xaddq	%rcx, (%rax)
	xorl	%eax, %eax
	cmpq	%rsi, %rcx
	setns	%al
	addq	%rdx, %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	subq	%rsi, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB1_346:
	cmpq	%rax, %rcx
	jge	.LBB1_348
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB1_346
.LBB1_348:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB1_349:
	movq	free@GOTPCREL(%rip), %rax
	movq	200(%rsp), %rdi
	callq	*%rax
.LBB1_350:
	testq	%r15, %r15
	jne	.LBB1_364
.LBB1_351:
	testq	%r13, %r13
	je	.LBB1_362
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	shlq	$5, %r13
	movabsq	$9223372036854775807, %rdx
	cmpq	%rdx, %r13
	cmovaeq	%rdx, %r13
	xorl	%ecx, %ecx
	cmpq	%r13, %rax
	setns	%cl
	addq	%rdx, %rcx
	subq	%r13, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB1_354
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
.LBB1_354:
	movq	232(%rsp), %rdi
	.p2align	4
.LBB1_355:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1_361
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB1_355
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%r13, %rcx
	negq	%rcx
	movabsq	$9223372036854775807, %rdx
	lock		xaddq	%rcx, (%rax)
	xorl	%eax, %eax
	cmpq	%r13, %rcx
	setns	%al
	addq	%rdx, %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	subq	%r13, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB1_358:
	cmpq	%rax, %rcx
	jge	.LBB1_360
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB1_358
.LBB1_360:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB1_361:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1_362:
.Ltmp83:
	movq	208(%rsp), %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtCs6a8jV7kq6PJ_10rayon_core17ThreadPoolBuilderEBD_
.Ltmp84:
	movq	80(%rsp), %rdi
	callq	_Unwind_Resume@PLT
.LBB1_364:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %r14
	movq	232(%rsp), %rbx
	jmp	.LBB1_366
	.p2align	4
.LBB1_365:
	addq	$32, %rbx
	decq	%r15
	je	.LBB1_351
.LBB1_366:
	movq	(%rbx), %rax
	lock		decq	(%rax)
	jne	.LBB1_365
	movq	%rbx, %rdi
	#MEMBARRIER
	callq	*%r14
	jmp	.LBB1_365
.LBB1_368:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %rbx
	movq	88(%rsp), %r12
	jmp	.LBB1_370
	.p2align	4
.LBB1_369:
	addq	$16, %r12
	decq	%r13
	je	.LBB1_329
.LBB1_370:
	movq	(%r12), %rax
	lock		decq	(%rax)
	jne	.LBB1_369
	movq	%r12, %rdi
	#MEMBARRIER
	callq	*%rbx
	jmp	.LBB1_369
.LBB1_372:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %r12
	movq	200(%rsp), %rbx
	jmp	.LBB1_374
	.p2align	4
.LBB1_373:
	addq	$32, %rbx
	decq	%r14
	je	.LBB1_340
.LBB1_374:
	movq	(%rbx), %rax
	lock		decq	(%rax)
	jne	.LBB1_373
	movq	%rbx, %rdi
	#MEMBARRIER
	callq	*%r12
	jmp	.LBB1_373
.LBB1_376:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %r13
	movq	88(%rsp), %rbx
	jmp	.LBB1_378
	.p2align	4
.LBB1_377:
	addq	$16, %rbx
	decq	%r12
	je	.LBB1_318
.LBB1_378:
	movq	(%rbx), %rax
	lock		decq	(%rax)
	jne	.LBB1_377
	movq	%rbx, %rdi
	#MEMBARRIER
	callq	*%r13
	jmp	.LBB1_377
.LBB1_380:
.Ltmp85:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end1:
	.size	_RINvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB6_8Registry3newNtB6_12DefaultSpawnEB8_, .Lfunc_end1-_RINvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB6_8Registry3newNtB6_12DefaultSpawnEB8_
	.cfi_endproc
	.section	.gcc_except_table._RINvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB6_8Registry3newNtB6_12DefaultSpawnEB8_,"a",@progbits
	.p2align	2, 0x0
GCC_except_table1:
.Lexception0:
	.byte	255
	.byte	155
	.uleb128 .Lttbase0-.Lttbaseref0
.Lttbaseref0:
	.byte	1
	.uleb128 .Lcst_end0-.Lcst_begin0
.Lcst_begin0:
	.uleb128 .Ltmp11-.Lfunc_begin1
	.uleb128 .Ltmp12-.Ltmp11
	.uleb128 .Ltmp22-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp23-.Lfunc_begin1
	.uleb128 .Ltmp24-.Ltmp23
	.uleb128 .Ltmp25-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp24-.Lfunc_begin1
	.uleb128 .Ltmp0-.Ltmp24
	.byte	0
	.byte	0
	.uleb128 .Ltmp0-.Lfunc_begin1
	.uleb128 .Ltmp1-.Ltmp0
	.uleb128 .Ltmp10-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp1-.Lfunc_begin1
	.uleb128 .Ltmp26-.Ltmp1
	.byte	0
	.byte	0
	.uleb128 .Ltmp26-.Lfunc_begin1
	.uleb128 .Ltmp27-.Ltmp26
	.uleb128 .Ltmp30-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp31-.Lfunc_begin1
	.uleb128 .Ltmp32-.Ltmp31
	.uleb128 .Ltmp33-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp80-.Lfunc_begin1
	.uleb128 .Ltmp81-.Ltmp80
	.uleb128 .Ltmp82-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp81-.Lfunc_begin1
	.uleb128 .Ltmp37-.Ltmp81
	.byte	0
	.byte	0
	.uleb128 .Ltmp37-.Lfunc_begin1
	.uleb128 .Ltmp38-.Ltmp37
	.uleb128 .Ltmp39-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp51-.Lfunc_begin1
	.uleb128 .Ltmp52-.Ltmp51
	.uleb128 .Ltmp57-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp52-.Lfunc_begin1
	.uleb128 .Ltmp55-.Ltmp52
	.byte	0
	.byte	0
	.uleb128 .Ltmp55-.Lfunc_begin1
	.uleb128 .Ltmp41-.Ltmp55
	.uleb128 .Ltmp57-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp41-.Lfunc_begin1
	.uleb128 .Ltmp42-.Ltmp41
	.byte	0
	.byte	0
	.uleb128 .Ltmp42-.Lfunc_begin1
	.uleb128 .Ltmp43-.Ltmp42
	.uleb128 .Ltmp44-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp45-.Lfunc_begin1
	.uleb128 .Ltmp46-.Ltmp45
	.uleb128 .Ltmp47-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp48-.Lfunc_begin1
	.uleb128 .Ltmp49-.Ltmp48
	.uleb128 .Ltmp50-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp49-.Lfunc_begin1
	.uleb128 .Ltmp2-.Ltmp49
	.byte	0
	.byte	0
	.uleb128 .Ltmp2-.Lfunc_begin1
	.uleb128 .Ltmp5-.Ltmp2
	.uleb128 .Ltmp10-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp5-.Lfunc_begin1
	.uleb128 .Ltmp6-.Ltmp5
	.byte	0
	.byte	0
	.uleb128 .Ltmp6-.Lfunc_begin1
	.uleb128 .Ltmp9-.Ltmp6
	.uleb128 .Ltmp10-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp53-.Lfunc_begin1
	.uleb128 .Ltmp54-.Ltmp53
	.uleb128 .Ltmp60-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp13-.Lfunc_begin1
	.uleb128 .Ltmp14-.Ltmp13
	.uleb128 .Ltmp22-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp28-.Lfunc_begin1
	.uleb128 .Ltmp29-.Ltmp28
	.uleb128 .Ltmp30-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp18-.Lfunc_begin1
	.uleb128 .Ltmp21-.Ltmp18
	.uleb128 .Ltmp22-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp15-.Lfunc_begin1
	.uleb128 .Ltmp16-.Ltmp15
	.uleb128 .Ltmp17-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp61-.Lfunc_begin1
	.uleb128 .Ltmp62-.Ltmp61
	.uleb128 .Ltmp63-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp58-.Lfunc_begin1
	.uleb128 .Ltmp59-.Ltmp58
	.uleb128 .Ltmp60-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp77-.Lfunc_begin1
	.uleb128 .Ltmp78-.Ltmp77
	.uleb128 .Ltmp79-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp71-.Lfunc_begin1
	.uleb128 .Ltmp72-.Ltmp71
	.uleb128 .Ltmp73-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp34-.Lfunc_begin1
	.uleb128 .Ltmp35-.Ltmp34
	.uleb128 .Ltmp36-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp74-.Lfunc_begin1
	.uleb128 .Ltmp75-.Ltmp74
	.uleb128 .Ltmp76-.Lfunc_begin1
	.byte	1
	.uleb128 .Ltmp75-.Lfunc_begin1
	.uleb128 .Ltmp64-.Ltmp75
	.byte	0
	.byte	0
	.uleb128 .Ltmp64-.Lfunc_begin1
	.uleb128 .Ltmp65-.Ltmp64
	.uleb128 .Ltmp66-.Lfunc_begin1
	.byte	1
	.uleb128 .Ltmp65-.Lfunc_begin1
	.uleb128 .Ltmp67-.Ltmp65
	.byte	0
	.byte	0
	.uleb128 .Ltmp67-.Lfunc_begin1
	.uleb128 .Ltmp70-.Ltmp67
	.uleb128 .Ltmp85-.Lfunc_begin1
	.byte	1
	.uleb128 .Ltmp70-.Lfunc_begin1
	.uleb128 .Ltmp83-.Ltmp70
	.byte	0
	.byte	0
	.uleb128 .Ltmp83-.Lfunc_begin1
	.uleb128 .Ltmp84-.Ltmp83
	.uleb128 .Ltmp85-.Lfunc_begin1
	.byte	1
	.uleb128 .Ltmp84-.Lfunc_begin1
	.uleb128 .Lfunc_end1-.Ltmp84
	.byte	0
	.byte	0
.Lcst_end0:
	.byte	127
	.byte	0
	.p2align	2, 0x0
.Lttbase0:
	.byte	0
	.p2align	2, 0x0

	.section	.text._RINvNtCs18aJq3QiqAb_15crossbeam_epoch7default11with_handleNCNvB2_3pin0NtNtB4_5guard5GuardECs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.prefalign	4, .Lfunc_end2, nop
	.type	_RINvNtCs18aJq3QiqAb_15crossbeam_epoch7default11with_handleNCNvB2_3pin0NtNtB4_5guard5GuardECs6a8jV7kq6PJ_10rayon_core,@function
_RINvNtCs18aJq3QiqAb_15crossbeam_epoch7default11with_handleNCNvB2_3pin0NtNtB4_5guard5GuardECs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin2:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception1
	pushq	%r14
	.cfi_def_cfa_offset 16
	pushq	%rbx
	.cfi_def_cfa_offset 24
	pushq	%rax
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -24
	.cfi_offset %r14, -16
	movq	_RNvNCNKNvNtCs18aJq3QiqAb_15crossbeam_epoch7default6HANDLE0023___RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rax
	cmpb	$1, %fs:8(%rax)
	jne	.LBB2_2
	addq	%fs:0, %rax
.LBB2_3:
	movq	(%rax), %rbx
	movq	%rbx, (%rsp)
	movq	2072(%rbx), %rax
	cmpq	$-1, %rax
	je	.LBB2_8
	leaq	1(%rax), %rcx
	movq	%rcx, 2072(%rbx)
	testq	%rax, %rax
	jne	.LBB2_19
	movq	8(%rbx), %rax
	movq	384(%rax), %rcx
	xorl	%eax, %eax
	orq	$1, %rcx
	lock		cmpxchgq	%rcx, 2176(%rbx)
	#MEMBARRIER
	movq	2088(%rbx), %rax
	leaq	1(%rax), %rcx
	movq	%rcx, 2088(%rbx)
	testb	$127, %al
	je	.LBB2_6
.LBB2_19:
	movq	%rbx, %rax
	addq	$8, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	retq
.LBB2_2:
	.cfi_def_cfa_offset 32
	movq	%fs:0, %rdi
	addq	_RNvNCNKNvNtCs18aJq3QiqAb_15crossbeam_epoch7default6HANDLE0023___RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rdi
	callq	_RINvMs0_NtNtNtNtCs7jcFBdfocI9_3std3sys12thread_local6native4lazyINtB6_7StorageNtNtCs18aJq3QiqAb_15crossbeam_epoch9collector11LocalHandleuE16get_or_init_slowNvNvNtB1i_7default6HANDLE27___rust_std_internal_init_fnECs6a8jV7kq6PJ_10rayon_core
	testq	%rax, %rax
	jne	.LBB2_3
	movl	_RNvNvNtCs18aJq3QiqAb_15crossbeam_epoch7default9collector9COLLECTOR.llvm.707543514826133848+8(%rip), %eax
	testl	%eax, %eax
	jne	.LBB2_12
.LBB2_13:
	movq	_RNvMs1_NtCs18aJq3QiqAb_15crossbeam_epoch9collectorNtB5_9Collector8register@GOTPCREL(%rip), %rax
	leaq	_RNvNvNtCs18aJq3QiqAb_15crossbeam_epoch7default9collector9COLLECTOR.llvm.707543514826133848(%rip), %rdi
	callq	*%rax
	movq	%rax, (%rsp)
	movq	%rax, %rbx
	movq	2072(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB2_20
	leaq	1(%rax), %rcx
	movq	%rcx, 2072(%rbx)
	testq	%rax, %rax
	jne	.LBB2_17
	movq	8(%rbx), %rax
	movq	384(%rax), %rcx
	xorl	%eax, %eax
	orq	$1, %rcx
	lock		cmpxchgq	%rcx, 2176(%rbx)
	#MEMBARRIER
	movq	2088(%rbx), %rax
	leaq	1(%rax), %rcx
	movq	%rcx, 2088(%rbx)
	testb	$127, %al
	je	.LBB2_16
.LBB2_17:
	movq	2080(%rbx), %rax
	leaq	-1(%rax), %rcx
	xorq	$1, %rax
	movq	%rcx, 2080(%rbx)
	orq	2072(%rbx), %rax
	jne	.LBB2_19
	movq	_RNvMs6_NtCs18aJq3QiqAb_15crossbeam_epoch8internalNtB5_5Local8finalize@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
	jmp	.LBB2_19
.LBB2_8:
.Ltmp99:
	movq	_RNvNtCs2k2z8Zem4rB_4core6option13unwrap_failed@GOTPCREL(%rip), %rax
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.42(%rip), %rdi
	callq	*%rax
.Ltmp100:
	jmp	.LBB2_9
.LBB2_6:
	movq	8(%rbx), %rdi
	subq	$-128, %rdi
.Ltmp97:
	movq	_RNvMs5_NtCs18aJq3QiqAb_15crossbeam_epoch8internalNtB5_6Global7collect@GOTPCREL(%rip), %rax
	movq	%rsp, %rsi
	callq	*%rax
.Ltmp98:
	jmp	.LBB2_19
.LBB2_12:
	callq	_RINvMs0_NtNtCs18aJq3QiqAb_15crossbeam_epoch4sync9once_lockINtB6_8OnceLockNtNtBa_9collector9CollectorE10initializeNvMs1_B1b_B19_3newEBa_.llvm.707543514826133848
	jmp	.LBB2_13
.LBB2_20:
.Ltmp88:
	movq	_RNvNtCs2k2z8Zem4rB_4core6option13unwrap_failed@GOTPCREL(%rip), %rax
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.42(%rip), %rdi
	callq	*%rax
.Ltmp89:
.LBB2_9:
	ud2
.LBB2_16:
	movq	8(%rbx), %rdi
	subq	$-128, %rdi
.Ltmp86:
	movq	_RNvMs5_NtCs18aJq3QiqAb_15crossbeam_epoch8internalNtB5_6Global7collect@GOTPCREL(%rip), %rax
	movq	%rsp, %rsi
	callq	*%rax
.Ltmp87:
	jmp	.LBB2_17
.LBB2_22:
.Ltmp90:
	movq	%rax, %r14
.Ltmp91:
	movq	%rbx, %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs18aJq3QiqAb_15crossbeam_epoch5guard5GuardECs6a8jV7kq6PJ_10rayon_core
.Ltmp92:
	movq	2080(%rbx), %rax
	leaq	-1(%rax), %rcx
	xorq	$1, %rax
	movq	%rcx, 2080(%rbx)
	orq	2072(%rbx), %rax
	jne	.LBB2_25
.Ltmp94:
	movq	_RNvMs6_NtCs18aJq3QiqAb_15crossbeam_epoch8internalNtB5_5Local8finalize@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp95:
	jmp	.LBB2_25
.LBB2_26:
.Ltmp96:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2_21:
.Ltmp93:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB2_7:
.Ltmp101:
	movq	%rax, %r14
.Ltmp102:
	movq	%rbx, %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs18aJq3QiqAb_15crossbeam_epoch5guard5GuardECs6a8jV7kq6PJ_10rayon_core
.Ltmp103:
.LBB2_25:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB2_10:
.Ltmp104:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end2:
	.size	_RINvNtCs18aJq3QiqAb_15crossbeam_epoch7default11with_handleNCNvB2_3pin0NtNtB4_5guard5GuardECs6a8jV7kq6PJ_10rayon_core, .Lfunc_end2-_RINvNtCs18aJq3QiqAb_15crossbeam_epoch7default11with_handleNCNvB2_3pin0NtNtB4_5guard5GuardECs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc
	.section	.gcc_except_table._RINvNtCs18aJq3QiqAb_15crossbeam_epoch7default11with_handleNCNvB2_3pin0NtNtB4_5guard5GuardECs6a8jV7kq6PJ_10rayon_core,"a",@progbits
	.p2align	2, 0x0
GCC_except_table2:
.Lexception1:
	.byte	255
	.byte	155
	.uleb128 .Lttbase1-.Lttbaseref1
.Lttbaseref1:
	.byte	1
	.uleb128 .Lcst_end1-.Lcst_begin1
.Lcst_begin1:
	.uleb128 .Lfunc_begin2-.Lfunc_begin2
	.uleb128 .Ltmp99-.Lfunc_begin2
	.byte	0
	.byte	0
	.uleb128 .Ltmp99-.Lfunc_begin2
	.uleb128 .Ltmp98-.Ltmp99
	.uleb128 .Ltmp101-.Lfunc_begin2
	.byte	0
	.uleb128 .Ltmp98-.Lfunc_begin2
	.uleb128 .Ltmp88-.Ltmp98
	.byte	0
	.byte	0
	.uleb128 .Ltmp88-.Lfunc_begin2
	.uleb128 .Ltmp87-.Ltmp88
	.uleb128 .Ltmp90-.Lfunc_begin2
	.byte	0
	.uleb128 .Ltmp91-.Lfunc_begin2
	.uleb128 .Ltmp92-.Ltmp91
	.uleb128 .Ltmp93-.Lfunc_begin2
	.byte	1
	.uleb128 .Ltmp94-.Lfunc_begin2
	.uleb128 .Ltmp95-.Ltmp94
	.uleb128 .Ltmp96-.Lfunc_begin2
	.byte	1
	.uleb128 .Ltmp95-.Lfunc_begin2
	.uleb128 .Ltmp102-.Ltmp95
	.byte	0
	.byte	0
	.uleb128 .Ltmp102-.Lfunc_begin2
	.uleb128 .Ltmp103-.Ltmp102
	.uleb128 .Ltmp104-.Lfunc_begin2
	.byte	1
	.uleb128 .Ltmp103-.Lfunc_begin2
	.uleb128 .Lfunc_end2-.Ltmp103
	.byte	0
	.byte	0
.Lcst_end1:
	.byte	127
	.byte	0
	.p2align	2, 0x0
.Lttbase1:
	.byte	0
	.p2align	2, 0x0

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_4cell10UnsafeCellINtNtB4_6option6OptionINtNtB4_6result6ResultuINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EEEEECs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.prefalign	4, .Lfunc_end3, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_4cell10UnsafeCellINtNtB4_6option6OptionINtNtB4_6result6ResultuINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EEEEECs6a8jV7kq6PJ_10rayon_core,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_4cell10UnsafeCellINtNtB4_6option6OptionINtNtB4_6result6ResultuINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EEEEECs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin3:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception2
	pushq	%r15
	.cfi_def_cfa_offset 16
	pushq	%r14
	.cfi_def_cfa_offset 24
	pushq	%rbx
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -32
	.cfi_offset %r14, -24
	.cfi_offset %r15, -16
	cmpq	$0, (%rdi)
	je	.LBB3_17
	movq	8(%rdi), %rbx
	testq	%rbx, %rbx
	je	.LBB3_17
	movq	16(%rdi), %r15
	movq	(%r15), %rax
	testq	%rax, %rax
	je	.LBB3_4
.Ltmp105:
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp106:
.LBB3_4:
	movq	8(%r15), %rcx
	testq	%rcx, %rcx
	je	.LBB3_17
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	xorl	%esi, %esi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB3_7
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB3_7:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB3_13
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB3_7
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB3_10:
	cmpq	%rax, %rsi
	jge	.LBB3_12
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB3_10
.LBB3_12:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB3_13:
	movq	%rbx, %rdi
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	jmpq	*free@GOTPCREL(%rip)
.LBB3_17:
	.cfi_def_cfa_offset 32
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	retq
.LBB3_14:
	.cfi_def_cfa_offset 32
.Ltmp107:
	movq	8(%r15), %rsi
	movq	%rax, %r14
	testq	%rsi, %rsi
	je	.LBB3_16
	movq	16(%r15), %rdx
	movq	%rbx, %rdi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
.LBB3_16:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end3:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_4cell10UnsafeCellINtNtB4_6option6OptionINtNtB4_6result6ResultuINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EEEEECs6a8jV7kq6PJ_10rayon_core, .Lfunc_end3-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_4cell10UnsafeCellINtNtB4_6option6OptionINtNtB4_6result6ResultuINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EEEEECs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc
	.section	.gcc_except_table._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_4cell10UnsafeCellINtNtB4_6option6OptionINtNtB4_6result6ResultuINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EEEEECs6a8jV7kq6PJ_10rayon_core,"a",@progbits
	.p2align	2, 0x0
GCC_except_table3:
.Lexception2:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end2-.Lcst_begin2
.Lcst_begin2:
	.uleb128 .Ltmp105-.Lfunc_begin3
	.uleb128 .Ltmp106-.Ltmp105
	.uleb128 .Ltmp107-.Lfunc_begin3
	.byte	0
	.uleb128 .Ltmp106-.Lfunc_begin3
	.uleb128 .Lfunc_end3-.Ltmp106
	.byte	0
	.byte	0
.Lcst_end2:
	.p2align	2, 0x0

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6option6OptionINtNtCsc70TAahYccp_5alloc5boxed3BoxDINtNtNtB4_3ops8function2FnTjEEp6OutputuNtNtB4_6marker4SyncNtB2c_4SendEL_EEECs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.prefalign	4, .Lfunc_end4, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6option6OptionINtNtCsc70TAahYccp_5alloc5boxed3BoxDINtNtNtB4_3ops8function2FnTjEEp6OutputuNtNtB4_6marker4SyncNtB2c_4SendEL_EEECs6a8jV7kq6PJ_10rayon_core,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6option6OptionINtNtCsc70TAahYccp_5alloc5boxed3BoxDINtNtNtB4_3ops8function2FnTjEEp6OutputuNtNtB4_6marker4SyncNtB2c_4SendEL_EEECs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin4:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception3
	pushq	%r15
	.cfi_def_cfa_offset 16
	pushq	%r14
	.cfi_def_cfa_offset 24
	pushq	%rbx
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -32
	.cfi_offset %r14, -24
	.cfi_offset %r15, -16
	testq	%rdi, %rdi
	je	.LBB4_16
	movq	(%rsi), %rax
	movq	%rsi, %r14
	movq	%rdi, %rbx
	testq	%rax, %rax
	je	.LBB4_3
.Ltmp108:
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp109:
.LBB4_3:
	movq	8(%r14), %rcx
	testq	%rcx, %rcx
	je	.LBB4_16
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	xorl	%esi, %esi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB4_6
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB4_6:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB4_12
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB4_6
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB4_9:
	cmpq	%rax, %rsi
	jge	.LBB4_11
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB4_9
.LBB4_11:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB4_12:
	movq	%rbx, %rdi
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	jmpq	*free@GOTPCREL(%rip)
.LBB4_16:
	.cfi_def_cfa_offset 32
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	retq
.LBB4_13:
	.cfi_def_cfa_offset 32
.Ltmp110:
	movq	8(%r14), %rsi
	movq	%rax, %r15
	testq	%rsi, %rsi
	je	.LBB4_15
	movq	16(%r14), %rdx
	movq	%rbx, %rdi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
.LBB4_15:
	movq	%r15, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end4:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6option6OptionINtNtCsc70TAahYccp_5alloc5boxed3BoxDINtNtNtB4_3ops8function2FnTjEEp6OutputuNtNtB4_6marker4SyncNtB2c_4SendEL_EEECs6a8jV7kq6PJ_10rayon_core, .Lfunc_end4-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6option6OptionINtNtCsc70TAahYccp_5alloc5boxed3BoxDINtNtNtB4_3ops8function2FnTjEEp6OutputuNtNtB4_6marker4SyncNtB2c_4SendEL_EEECs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc
	.section	.gcc_except_table._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6option6OptionINtNtCsc70TAahYccp_5alloc5boxed3BoxDINtNtNtB4_3ops8function2FnTjEEp6OutputuNtNtB4_6marker4SyncNtB2c_4SendEL_EEECs6a8jV7kq6PJ_10rayon_core,"a",@progbits
	.p2align	2, 0x0
GCC_except_table4:
.Lexception3:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end3-.Lcst_begin3
.Lcst_begin3:
	.uleb128 .Ltmp108-.Lfunc_begin4
	.uleb128 .Ltmp109-.Ltmp108
	.uleb128 .Ltmp110-.Lfunc_begin4
	.byte	0
	.uleb128 .Ltmp109-.Lfunc_begin4
	.uleb128 .Lfunc_end4-.Ltmp109
	.byte	0
	.byte	0
.Lcst_end3:
	.p2align	2, 0x0

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultINtNtCsc70TAahYccp_5alloc4sync3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryENtB1z_20ThreadPoolBuildErrorEEB1z_,"ax",@progbits
	.prefalign	4, .Lfunc_end5, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultINtNtCsc70TAahYccp_5alloc4sync3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryENtB1z_20ThreadPoolBuildErrorEEB1z_,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultINtNtCsc70TAahYccp_5alloc4sync3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryENtB1z_20ThreadPoolBuildErrorEEB1z_:
.Lfunc_begin5:
	.cfi_startproc
	movq	(%rdi), %rax
	cmpq	$-1, %rax
	je	.LBB5_1
	cmpl	$2, %eax
	jb	.LBB5_6
	movq	8(%rdi), %rax
	movl	%eax, %ecx
	andl	$3, %ecx
	leal	-2(%rcx), %edx
	cmpl	$2, %edx
	jb	.LBB5_6
	testq	%rcx, %rcx
	je	.LBB5_6
	leaq	-1(%rax), %rdi
	jmpq	*23(%rax)
.LBB5_1:
	movq	8(%rdi), %rax
	lock		decq	(%rax)
	jne	.LBB5_6
	addq	$8, %rdi
	#MEMBARRIER
	jmpq	*_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryE9drop_slowBK_@GOTPCREL(%rip)
.LBB5_6:
	retq
.Lfunc_end5:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultINtNtCsc70TAahYccp_5alloc4sync3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryENtB1z_20ThreadPoolBuildErrorEEB1z_, .Lfunc_end5-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultINtNtCsc70TAahYccp_5alloc4sync3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryENtB1z_20ThreadPoolBuildErrorEEB1z_
	.cfi_endproc

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultRINtNtCsc70TAahYccp_5alloc4sync3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryENtB1A_20ThreadPoolBuildErrorEEB1A_.llvm.7294274987384275483,"ax",@progbits
	.hidden	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultRINtNtCsc70TAahYccp_5alloc4sync3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryENtB1A_20ThreadPoolBuildErrorEEB1A_.llvm.7294274987384275483
	.globl	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultRINtNtCsc70TAahYccp_5alloc4sync3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryENtB1A_20ThreadPoolBuildErrorEEB1A_.llvm.7294274987384275483
	.prefalign	4, .Lfunc_end6, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultRINtNtCsc70TAahYccp_5alloc4sync3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryENtB1A_20ThreadPoolBuildErrorEEB1A_.llvm.7294274987384275483,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultRINtNtCsc70TAahYccp_5alloc4sync3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryENtB1A_20ThreadPoolBuildErrorEEB1A_.llvm.7294274987384275483:
.Lfunc_begin6:
	.cfi_startproc
	addq	$-2, %rdi
	cmpq	$-3, %rdi
	jae	.LBB6_3
	movl	%esi, %eax
	andl	$3, %eax
	leal	-2(%rax), %ecx
	cmpl	$2, %ecx
	jb	.LBB6_3
	testq	%rax, %rax
	jne	.LBB6_4
.LBB6_3:
	retq
.LBB6_4:
	leaq	-1(%rsi), %rdi
	jmpq	*23(%rsi)
.Lfunc_end6:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultRINtNtCsc70TAahYccp_5alloc4sync3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryENtB1A_20ThreadPoolBuildErrorEEB1A_.llvm.7294274987384275483, .Lfunc_end6-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultRINtNtCsc70TAahYccp_5alloc4sync3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryENtB1A_20ThreadPoolBuildErrorEEB1A_.llvm.7294274987384275483
	.cfi_endproc

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultuINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EEECs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.prefalign	4, .Lfunc_end7, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultuINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EEECs6a8jV7kq6PJ_10rayon_core,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultuINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EEECs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin7:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception4
	pushq	%r15
	.cfi_def_cfa_offset 16
	pushq	%r14
	.cfi_def_cfa_offset 24
	pushq	%rbx
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -32
	.cfi_offset %r14, -24
	.cfi_offset %r15, -16
	movq	(%rsi), %rax
	movq	%rsi, %r14
	movq	%rdi, %rbx
	testq	%rax, %rax
	je	.LBB7_2
.Ltmp111:
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp112:
.LBB7_2:
	movq	8(%r14), %rcx
	testq	%rcx, %rcx
	je	.LBB7_15
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	xorl	%esi, %esi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB7_5
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB7_5:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB7_11
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB7_5
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB7_8:
	cmpq	%rax, %rsi
	jge	.LBB7_10
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB7_8
.LBB7_10:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB7_11:
	movq	%rbx, %rdi
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	jmpq	*free@GOTPCREL(%rip)
.LBB7_15:
	.cfi_def_cfa_offset 32
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	retq
.LBB7_12:
	.cfi_def_cfa_offset 32
.Ltmp113:
	movq	8(%r14), %rsi
	movq	%rax, %r15
	testq	%rsi, %rsi
	je	.LBB7_14
	movq	16(%r14), %rdx
	movq	%rbx, %rdi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
.LBB7_14:
	movq	%r15, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end7:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultuINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EEECs6a8jV7kq6PJ_10rayon_core, .Lfunc_end7-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultuINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EEECs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc
	.section	.gcc_except_table._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultuINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EEECs6a8jV7kq6PJ_10rayon_core,"a",@progbits
	.p2align	2, 0x0
GCC_except_table7:
.Lexception4:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end4-.Lcst_begin4
.Lcst_begin4:
	.uleb128 .Ltmp111-.Lfunc_begin7
	.uleb128 .Ltmp112-.Ltmp111
	.uleb128 .Ltmp113-.Lfunc_begin7
	.byte	0
	.uleb128 .Ltmp112-.Lfunc_begin7
	.uleb128 .Lfunc_end7-.Ltmp112
	.byte	0
	.byte	0
.Lcst_end4:
	.p2align	2, 0x0

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultuNtNtNtB4_2io5error5ErrorEECs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.prefalign	4, .Lfunc_end8, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultuNtNtNtB4_2io5error5ErrorEECs6a8jV7kq6PJ_10rayon_core,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultuNtNtNtB4_2io5error5ErrorEECs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin8:
	.cfi_startproc
	testq	%rdi, %rdi
	je	.LBB8_3
	movl	%edi, %eax
	andl	$3, %eax
	leal	-2(%rax), %ecx
	cmpl	$2, %ecx
	jb	.LBB8_3
	testq	%rax, %rax
	jne	.LBB8_4
.LBB8_3:
	retq
.LBB8_4:
	leaq	-1(%rdi), %rax
	movq	%rdi, %rcx
	movq	%rax, %rdi
	jmpq	*23(%rcx)
.Lfunc_end8:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultuNtNtNtB4_2io5error5ErrorEECs6a8jV7kq6PJ_10rayon_core, .Lfunc_end8-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultuNtNtNtB4_2io5error5ErrorEECs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc3vec3VecINtNtBG_5boxed3BoxDINtNtNtB4_3ops8function6FnOnceuEp6OutputuNtNtB4_6marker4SendEL_EEECs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.prefalign	4, .Lfunc_end9, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc3vec3VecINtNtBG_5boxed3BoxDINtNtNtB4_3ops8function6FnOnceuEp6OutputuNtNtB4_6marker4SendEL_EEECs6a8jV7kq6PJ_10rayon_core,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc3vec3VecINtNtBG_5boxed3BoxDINtNtNtB4_3ops8function6FnOnceuEp6OutputuNtNtB4_6marker4SendEL_EEECs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin9:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception5
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
	subq	$24, %rsp
	.cfi_def_cfa_offset 80
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	8(%rdi), %rax
	movq	16(%rdi), %rcx
	movq	%rax, (%rsp)
	movq	%rdi, 16(%rsp)
	movq	%rcx, 8(%rsp)
	testq	%rcx, %rcx
	je	.LBB9_15
	movq	(%rsp), %rax
	movq	8(%rsp), %rcx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r14
	xorl	%r12d, %r12d
	leaq	24(%rax), %r13
	leaq	-1(%rcx), %rbp
	jmp	.LBB9_2
	.p2align	4
.LBB9_12:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB9_13:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.LBB9_14:
	addq	$16, %r13
	decq	%rbp
	cmpq	8(%rsp), %r12
	je	.LBB9_15
.LBB9_2:
	movq	(%rsp), %rcx
	movq	%r12, %rax
	shlq	$4, %rax
	incq	%r12
	movq	8(%rcx,%rax), %rbx
	movq	(%rcx,%rax), %r15
	movq	(%rbx), %rax
	testq	%rax, %rax
	je	.LBB9_4
.Ltmp114:
	movq	%r15, %rdi
	callq	*%rax
.Ltmp115:
.LBB9_4:
	movq	8(%rbx), %rcx
	testq	%rcx, %rcx
	je	.LBB9_14
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	xorl	%edx, %edx
	movabsq	$9223372036854775807, %rsi
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB9_7
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB9_7:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB9_13
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB9_7
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r14), %rax
	.p2align	4
.LBB9_10:
	cmpq	%rax, %rdx
	jge	.LBB9_12
	lock		cmpxchgq	%rdx, (%r14)
	jne	.LBB9_10
	jmp	.LBB9_12
.LBB9_15:
	movq	16(%rsp), %rax
	movq	(%rax), %rcx
	testq	%rcx, %rcx
	je	.LBB9_34
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	shlq	$4, %rcx
	movabsq	$9223372036854775807, %rsi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB9_18
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB9_18:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB9_24
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB9_18
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB9_21:
	cmpq	%rax, %rdx
	jge	.LBB9_23
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB9_21
.LBB9_23:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB9_24:
	movq	(%rsp), %rdi
	addq	$24, %rsp
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
	jmpq	*free@GOTPCREL(%rip)
.LBB9_34:
	.cfi_def_cfa_offset 80
	addq	$24, %rsp
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
.LBB9_25:
	.cfi_def_cfa_offset 80
.Ltmp116:
	movq	8(%rbx), %rsi
	movq	%rax, %r14
	testq	%rsi, %rsi
	je	.LBB9_27
	movq	16(%rbx), %rdx
	movq	%r15, %rdi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
.LBB9_27:
	cmpq	8(%rsp), %r12
	je	.LBB9_30
	.p2align	4
.LBB9_28:
	movq	-8(%r13), %rdi
	movq	(%r13), %rsi
.Ltmp117:
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc5boxed3BoxDINtNtNtB4_3ops8function6FnOnceuEp6OutputuNtNtB4_6marker4SendEL_EECs6a8jV7kq6PJ_10rayon_core
.Ltmp118:
	addq	$16, %r13
	decq	%rbp
	jne	.LBB9_28
.LBB9_30:
	movq	16(%rsp), %rax
	movq	(%rax), %rsi
	testq	%rsi, %rsi
	je	.LBB9_32
	movq	(%rsp), %rdi
	shlq	$4, %rsi
	movl	$8, %edx
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
.LBB9_32:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB9_33:
.Ltmp119:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end9:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc3vec3VecINtNtBG_5boxed3BoxDINtNtNtB4_3ops8function6FnOnceuEp6OutputuNtNtB4_6marker4SendEL_EEECs6a8jV7kq6PJ_10rayon_core, .Lfunc_end9-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc3vec3VecINtNtBG_5boxed3BoxDINtNtNtB4_3ops8function6FnOnceuEp6OutputuNtNtB4_6marker4SendEL_EEECs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc
	.section	.gcc_except_table._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc3vec3VecINtNtBG_5boxed3BoxDINtNtNtB4_3ops8function6FnOnceuEp6OutputuNtNtB4_6marker4SendEL_EEECs6a8jV7kq6PJ_10rayon_core,"a",@progbits
	.p2align	2, 0x0
GCC_except_table9:
.Lexception5:
	.byte	255
	.byte	155
	.uleb128 .Lttbase2-.Lttbaseref2
.Lttbaseref2:
	.byte	1
	.uleb128 .Lcst_end5-.Lcst_begin5
.Lcst_begin5:
	.uleb128 .Lfunc_begin9-.Lfunc_begin9
	.uleb128 .Ltmp114-.Lfunc_begin9
	.byte	0
	.byte	0
	.uleb128 .Ltmp114-.Lfunc_begin9
	.uleb128 .Ltmp115-.Ltmp114
	.uleb128 .Ltmp116-.Lfunc_begin9
	.byte	0
	.uleb128 .Ltmp115-.Lfunc_begin9
	.uleb128 .Ltmp117-.Ltmp115
	.byte	0
	.byte	0
	.uleb128 .Ltmp117-.Lfunc_begin9
	.uleb128 .Ltmp118-.Ltmp117
	.uleb128 .Ltmp119-.Lfunc_begin9
	.byte	1
	.uleb128 .Ltmp118-.Lfunc_begin9
	.uleb128 .Lfunc_end9-.Ltmp118
	.byte	0
	.byte	0
.Lcst_end5:
	.byte	127
	.byte	0
	.p2align	2, 0x0
.Lttbase2:
	.byte	0
	.p2align	2, 0x0

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc3vec3VecINtNtCs64OF0TycdZY_15crossbeam_deque5deque7StealerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEEB20_,"ax",@progbits
	.prefalign	4, .Lfunc_end10, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc3vec3VecINtNtCs64OF0TycdZY_15crossbeam_deque5deque7StealerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEEB20_,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc3vec3VecINtNtCs64OF0TycdZY_15crossbeam_deque5deque7StealerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEEB20_:
.Lfunc_begin10:
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
	movq	8(%rdi), %rbx
	movq	16(%rdi), %r12
	movq	%rdi, %r14
	testq	%r12, %r12
	je	.LBB10_5
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %r13
	movq	%rbx, %r15
	jmp	.LBB10_2
	.p2align	4
.LBB10_4:
	addq	$16, %r15
	decq	%r12
	je	.LBB10_5
.LBB10_2:
	movq	(%r15), %rax
	lock		decq	(%rax)
	jne	.LBB10_4
	movq	%r15, %rdi
	#MEMBARRIER
	callq	*%r13
	jmp	.LBB10_4
.LBB10_5:
	movq	(%r14), %rcx
	testq	%rcx, %rcx
	je	.LBB10_15
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	shlq	$4, %rcx
	movabsq	$9223372036854775807, %rdx
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB10_8
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB10_8:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB10_14
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB10_8
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB10_11:
	cmpq	%rax, %rsi
	jge	.LBB10_13
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB10_11
.LBB10_13:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB10_14:
	movq	%rbx, %rdi
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
	jmpq	*free@GOTPCREL(%rip)
.LBB10_15:
	.cfi_def_cfa_offset 48
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
.Lfunc_end10:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc3vec3VecINtNtCs64OF0TycdZY_15crossbeam_deque5deque7StealerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEEB20_, .Lfunc_end10-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc3vec3VecINtNtCs64OF0TycdZY_15crossbeam_deque5deque7StealerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEEB20_
	.cfi_endproc

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc3vec3VecNtNtCs6a8jV7kq6PJ_10rayon_core8registry10ThreadInfoEEB1c_,"ax",@progbits
	.prefalign	4, .Lfunc_end11, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc3vec3VecNtNtCs6a8jV7kq6PJ_10rayon_core8registry10ThreadInfoEEB1c_,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc3vec3VecNtNtCs6a8jV7kq6PJ_10rayon_core8registry10ThreadInfoEEB1c_:
.Lfunc_begin11:
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
	movq	8(%rdi), %rbx
	movq	16(%rdi), %r12
	movq	%rdi, %r14
	testq	%r12, %r12
	je	.LBB11_5
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %r13
	movq	%rbx, %r15
	jmp	.LBB11_2
	.p2align	4
.LBB11_4:
	addq	$48, %r15
	decq	%r12
	je	.LBB11_5
.LBB11_2:
	movq	(%r15), %rax
	lock		decq	(%rax)
	jne	.LBB11_4
	movq	%r15, %rdi
	#MEMBARRIER
	callq	*%r13
	jmp	.LBB11_4
.LBB11_5:
	movq	(%r14), %rax
	testq	%rax, %rax
	je	.LBB11_15
	shlq	$4, %rax
	movabsq	$9223372036854775807, %rdx
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB11_8
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB11_8:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB11_14
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB11_8
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB11_11:
	cmpq	%rax, %rsi
	jge	.LBB11_13
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB11_11
.LBB11_13:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB11_14:
	movq	%rbx, %rdi
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
	jmpq	*free@GOTPCREL(%rip)
.LBB11_15:
	.cfi_def_cfa_offset 48
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
.Lfunc_end11:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc3vec3VecNtNtCs6a8jV7kq6PJ_10rayon_core8registry10ThreadInfoEEB1c_, .Lfunc_end11-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc3vec3VecNtNtCs6a8jV7kq6PJ_10rayon_core8registry10ThreadInfoEEB1c_
	.cfi_endproc

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc4sync8ArcInnerINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEEEB35_,"ax",@progbits
	.prefalign	4, .Lfunc_end12, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc4sync8ArcInnerINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEEEB35_,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc4sync8ArcInnerINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEEEB35_:
.Lfunc_begin12:
	.cfi_startproc
	pushq	%rbx
	.cfi_def_cfa_offset 16
	.cfi_offset %rbx, -16
	movq	128(%rdi), %rbx
	andq	$-8, %rbx
	movq	8(%rbx), %rcx
	testq	%rcx, %rcx
	je	.LBB12_10
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	shlq	$4, %rcx
	movq	(%rbx), %rdi
	xorl	%esi, %esi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB12_3
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB12_3:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB12_9
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB12_3
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB12_6:
	cmpq	%rax, %rsi
	jge	.LBB12_8
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB12_6
.LBB12_8:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB12_9:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB12_10:
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movabsq	$-9223372036854775808, %rcx
	addq	$-16, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB12_12
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB12_12:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB12_18
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB12_12
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	$-16, %rdx
	lock		xaddq	%rdx, (%rax)
	addq	$-16, %rdx
	cmovoq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
	.p2align	4
.LBB12_15:
	cmpq	%rax, %rdx
	jge	.LBB12_17
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB12_15
.LBB12_17:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB12_18:
	movq	%rbx, %rdi
	popq	%rbx
	.cfi_def_cfa_offset 8
	jmpq	*free@GOTPCREL(%rip)
.Lfunc_end12:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc4sync8ArcInnerINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEEEB35_, .Lfunc_end12-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc4sync8ArcInnerINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEEEB35_
	.cfi_endproc

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc5boxed3BoxDINtNtNtB4_3ops8function6FnOnceuEp6OutputuNtNtB4_6marker4SendEL_EECs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.prefalign	4, .Lfunc_end13, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc5boxed3BoxDINtNtNtB4_3ops8function6FnOnceuEp6OutputuNtNtB4_6marker4SendEL_EECs6a8jV7kq6PJ_10rayon_core,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc5boxed3BoxDINtNtNtB4_3ops8function6FnOnceuEp6OutputuNtNtB4_6marker4SendEL_EECs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin13:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception6
	pushq	%r15
	.cfi_def_cfa_offset 16
	pushq	%r14
	.cfi_def_cfa_offset 24
	pushq	%rbx
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -32
	.cfi_offset %r14, -24
	.cfi_offset %r15, -16
	movq	(%rsi), %rax
	movq	%rsi, %r14
	movq	%rdi, %rbx
	testq	%rax, %rax
	je	.LBB13_2
.Ltmp120:
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp121:
.LBB13_2:
	movq	8(%r14), %rcx
	testq	%rcx, %rcx
	je	.LBB13_12
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	xorl	%esi, %esi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB13_5
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB13_5:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB13_11
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB13_5
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB13_8:
	cmpq	%rax, %rsi
	jge	.LBB13_10
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB13_8
.LBB13_10:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB13_11:
	movq	%rbx, %rdi
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	jmpq	*free@GOTPCREL(%rip)
.LBB13_12:
	.cfi_def_cfa_offset 32
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	retq
.LBB13_13:
	.cfi_def_cfa_offset 32
.Ltmp122:
	movq	8(%r14), %rsi
	movq	%rax, %r15
	testq	%rsi, %rsi
	je	.LBB13_15
	movq	16(%r14), %rdx
	movq	%rbx, %rdi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
.LBB13_15:
	movq	%r15, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end13:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc5boxed3BoxDINtNtNtB4_3ops8function6FnOnceuEp6OutputuNtNtB4_6marker4SendEL_EECs6a8jV7kq6PJ_10rayon_core, .Lfunc_end13-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc5boxed3BoxDINtNtNtB4_3ops8function6FnOnceuEp6OutputuNtNtB4_6marker4SendEL_EECs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc
	.section	.gcc_except_table._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc5boxed3BoxDINtNtNtB4_3ops8function6FnOnceuEp6OutputuNtNtB4_6marker4SendEL_EECs6a8jV7kq6PJ_10rayon_core,"a",@progbits
	.p2align	2, 0x0
GCC_except_table13:
.Lexception6:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end6-.Lcst_begin6
.Lcst_begin6:
	.uleb128 .Ltmp120-.Lfunc_begin13
	.uleb128 .Ltmp121-.Ltmp120
	.uleb128 .Ltmp122-.Lfunc_begin13
	.byte	0
	.uleb128 .Ltmp121-.Lfunc_begin13
	.uleb128 .Lfunc_end13-.Ltmp121
	.byte	0
	.byte	0
.Lcst_end6:
	.p2align	2, 0x0

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EECs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.prefalign	4, .Lfunc_end14, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EECs6a8jV7kq6PJ_10rayon_core,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EECs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin14:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception7
	pushq	%r15
	.cfi_def_cfa_offset 16
	pushq	%r14
	.cfi_def_cfa_offset 24
	pushq	%rbx
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -32
	.cfi_offset %r14, -24
	.cfi_offset %r15, -16
	movq	(%rsi), %rax
	movq	%rsi, %r14
	movq	%rdi, %rbx
	testq	%rax, %rax
	je	.LBB14_2
.Ltmp123:
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp124:
.LBB14_2:
	movq	8(%r14), %rcx
	testq	%rcx, %rcx
	je	.LBB14_12
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	xorl	%esi, %esi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB14_5
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB14_5:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB14_11
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB14_5
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB14_8:
	cmpq	%rax, %rsi
	jge	.LBB14_10
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB14_8
.LBB14_10:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB14_11:
	movq	%rbx, %rdi
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	jmpq	*free@GOTPCREL(%rip)
.LBB14_12:
	.cfi_def_cfa_offset 32
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	retq
.LBB14_13:
	.cfi_def_cfa_offset 32
.Ltmp125:
	movq	8(%r14), %rsi
	movq	%rax, %r15
	testq	%rsi, %rsi
	je	.LBB14_15
	movq	16(%r14), %rdx
	movq	%rbx, %rdi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
.LBB14_15:
	movq	%r15, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end14:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EECs6a8jV7kq6PJ_10rayon_core, .Lfunc_end14-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EECs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc
	.section	.gcc_except_table._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EECs6a8jV7kq6PJ_10rayon_core,"a",@progbits
	.p2align	2, 0x0
GCC_except_table14:
.Lexception7:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end7-.Lcst_begin7
.Lcst_begin7:
	.uleb128 .Ltmp123-.Lfunc_begin14
	.uleb128 .Ltmp124-.Ltmp123
	.uleb128 .Ltmp125-.Lfunc_begin14
	.byte	0
	.uleb128 .Ltmp124-.Lfunc_begin14
	.uleb128 .Lfunc_end14-.Ltmp124
	.byte	0
	.byte	0
.Lcst_end7:
	.p2align	2, 0x0

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtCs7jcFBdfocI9_3std4sync6poison11PoisonErrorINtNtBE_5mutex10MutexGuardbEEECs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483,"ax",@progbits
	.hidden	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtCs7jcFBdfocI9_3std4sync6poison11PoisonErrorINtNtBE_5mutex10MutexGuardbEEECs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483
	.globl	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtCs7jcFBdfocI9_3std4sync6poison11PoisonErrorINtNtBE_5mutex10MutexGuardbEEECs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483
	.prefalign	4, .Lfunc_end15, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtCs7jcFBdfocI9_3std4sync6poison11PoisonErrorINtNtBE_5mutex10MutexGuardbEEECs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtCs7jcFBdfocI9_3std4sync6poison11PoisonErrorINtNtBE_5mutex10MutexGuardbEEECs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483:
.Lfunc_begin15:
	.cfi_startproc
	pushq	%rbx
	.cfi_def_cfa_offset 16
	.cfi_offset %rbx, -16
	movq	(%rdi), %rbx
	cmpb	$0, 8(%rdi)
	jne	.LBB15_4
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count18GLOBAL_PANIC_COUNT@GOTPCREL(%rip), %rax
	movq	(%rax), %rax
	shlq	%rax
	testq	%rax, %rax
	jne	.LBB15_2
.LBB15_4:
	xorl	%eax, %eax
	xchgl	%eax, (%rbx)
	cmpl	$2, %eax
	je	.LBB15_6
	popq	%rbx
	.cfi_def_cfa_offset 8
	retq
.LBB15_6:
	.cfi_def_cfa_offset 16
	movl	$202, %edi
	movq	%rbx, %rsi
	movl	$129, %edx
	movl	$1, %ecx
	xorl	%eax, %eax
	popq	%rbx
	.cfi_def_cfa_offset 8
	jmpq	*syscall@GOTPCREL(%rip)
.LBB15_2:
	.cfi_def_cfa_offset 16
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count17is_zero_slow_path@GOTPCREL(%rip), %rax
	callq	*%rax
	testb	%al, %al
	jne	.LBB15_4
	movb	$1, 4(%rbx)
	jmp	.LBB15_4
.Lfunc_end15:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtCs7jcFBdfocI9_3std4sync6poison11PoisonErrorINtNtBE_5mutex10MutexGuardbEEECs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483, .Lfunc_end15-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtCs7jcFBdfocI9_3std4sync6poison11PoisonErrorINtNtBE_5mutex10MutexGuardbEEECs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483
	.cfi_endproc

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEECs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.prefalign	4, .Lfunc_end16, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEECs6a8jV7kq6PJ_10rayon_core,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEECs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin16:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception8
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
	pushq	%rax
	.cfi_def_cfa_offset 64
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	16(%rdi), %r15
	movq	8(%rdi), %rbp
	leaq	8(%rdi), %rbx
	movq	%rdi, %r14
	testq	%r15, %r15
	setne	%r13b
	testq	%rbp, %rbp
	je	.LBB16_14
	testq	%r15, %r15
	je	.LBB16_14
	movq	24(%r14), %r12
	movq	(%r12), %rax
	testq	%rax, %rax
	je	.LBB16_4
.Ltmp126:
	movq	%r15, %rdi
	callq	*%rax
.Ltmp127:
.LBB16_4:
	movq	8(%r12), %rcx
	testq	%rcx, %rcx
	je	.LBB16_14
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	xorl	%esi, %esi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB16_7
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB16_7:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB16_13
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB16_7
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB16_10:
	cmpq	%rax, %rsi
	jge	.LBB16_12
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB16_10
.LBB16_12:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB16_13:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.LBB16_14:
	movq	$0, (%rbx)
.LBB16_15:
	movq	(%r14), %r15
	testq	%r15, %r15
	je	.LBB16_22
	andb	%r13b, %bpl
	je	.LBB16_17
	movb	$1, 32(%r15)
.LBB16_17:
	lock		decq	24(%r15)
	jne	.LBB16_20
	movq	16(%r15), %rsi
	movl	$1, %eax
	xchgl	%eax, 56(%rsi)
	cmpl	$-1, %eax
	je	.LBB16_19
.LBB16_20:
	lock		decq	(%r15)
	jne	.LBB16_22
.LBB16_21:
	#MEMBARRIER
.Ltmp142:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtNtCs7jcFBdfocI9_3std6thread6scoped9ScopeDataE9drop_slowCs6a8jV7kq6PJ_10rayon_core@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
.Ltmp143:
.LBB16_22:
	addq	$8, %rsp
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
.LBB16_19:
	.cfi_def_cfa_offset 64
	movq	syscall@GOTPCREL(%rip), %r8
	addq	$56, %rsi
	movl	$202, %edi
	movl	$129, %edx
	movl	$1, %ecx
	xorl	%eax, %eax
	callq	*%r8
	lock		decq	(%r15)
	je	.LBB16_21
	jmp	.LBB16_22
.LBB16_23:
.Ltmp128:
	movq	8(%r12), %rsi
	movq	%rax, (%rsp)
	testq	%rsi, %rsi
	je	.LBB16_25
	movq	16(%r12), %rdx
	movq	%r15, %rdi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
.LBB16_25:
	movq	$0, (%rbx)
.Ltmp129:
	movq	_RNvCs2NWS7XDLE6y_7___rustc20___rust_panic_cleanup@GOTPCREL(%rip), %rax
	movq	(%rsp), %rdi
	callq	*%rax
	movq	%rdx, (%rsp)
.Ltmp130:
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count18GLOBAL_PANIC_COUNT@GOTPCREL(%rip), %rcx
	movq	%rax, %r12
	lock		decq	(%rcx)
	decq	%fs:_RNvNCNKNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count17LOCAL_PANIC_COUNT0s_023___RUST_STD_INTERNAL_VAL.llvm.11640361436736466388@TPOFF
	movb	$0, %fs:_RNvNCNKNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count17LOCAL_PANIC_COUNT0s_023___RUST_STD_INTERNAL_VAL.llvm.11640361436736466388@TPOFF+8
	testq	%r12, %r12
	je	.LBB16_15
.Ltmp132:
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.60(%rip), %rdi
	movl	$62, %esi
	callq	_RNvYNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrNtNtNtCs2k2z8Zem4rB_4core2io5write5Write9write_allCs6a8jV7kq6PJ_10rayon_core
.Ltmp133:
.Ltmp134:
	movq	%rax, %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultuNtNtNtB4_2io5error5ErrorEECs6a8jV7kq6PJ_10rayon_core
.Ltmp135:
	movq	_RNvNtCs7jcFBdfocI9_3std7process5abort@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB16_33:
.Ltmp136:
	movq	%rax, %r15
.Ltmp137:
	movq	(%rsp), %rsi
	movq	%r12, %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultuINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EEECs6a8jV7kq6PJ_10rayon_core
.Ltmp138:
	movq	(%r14), %rax
	testq	%rax, %rax
	je	.LBB16_37
	lock		decq	(%rax)
	jne	.LBB16_37
	#MEMBARRIER
.Ltmp140:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtNtCs7jcFBdfocI9_3std6thread6scoped9ScopeDataE9drop_slowCs6a8jV7kq6PJ_10rayon_core@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
.Ltmp141:
	jmp	.LBB16_37
.LBB16_31:
.Ltmp139:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB16_30:
.Ltmp131:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking19panic_cannot_unwind@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB16_40:
.Ltmp144:
	movq	%rax, %r15
.LBB16_37:
.Ltmp145:
	movq	%rbx, %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_4cell10UnsafeCellINtNtB4_6option6OptionINtNtB4_6result6ResultuINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EEEEECs6a8jV7kq6PJ_10rayon_core
.Ltmp146:
	movq	%r15, %rdi
	callq	_Unwind_Resume@PLT
.LBB16_39:
.Ltmp147:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end16:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEECs6a8jV7kq6PJ_10rayon_core, .Lfunc_end16-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEECs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc
	.section	.gcc_except_table._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEECs6a8jV7kq6PJ_10rayon_core,"a",@progbits
	.p2align	2, 0x0
GCC_except_table16:
.Lexception8:
	.byte	255
	.byte	155
	.uleb128 .Lttbase3-.Lttbaseref3
.Lttbaseref3:
	.byte	1
	.uleb128 .Lcst_end8-.Lcst_begin8
.Lcst_begin8:
	.uleb128 .Ltmp126-.Lfunc_begin16
	.uleb128 .Ltmp127-.Ltmp126
	.uleb128 .Ltmp128-.Lfunc_begin16
	.byte	5
	.uleb128 .Ltmp127-.Lfunc_begin16
	.uleb128 .Ltmp142-.Ltmp127
	.byte	0
	.byte	0
	.uleb128 .Ltmp142-.Lfunc_begin16
	.uleb128 .Ltmp143-.Ltmp142
	.uleb128 .Ltmp144-.Lfunc_begin16
	.byte	0
	.uleb128 .Ltmp143-.Lfunc_begin16
	.uleb128 .Ltmp129-.Ltmp143
	.byte	0
	.byte	0
	.uleb128 .Ltmp129-.Lfunc_begin16
	.uleb128 .Ltmp130-.Ltmp129
	.uleb128 .Ltmp131-.Lfunc_begin16
	.byte	1
	.uleb128 .Ltmp132-.Lfunc_begin16
	.uleb128 .Ltmp135-.Ltmp132
	.uleb128 .Ltmp136-.Lfunc_begin16
	.byte	0
	.uleb128 .Ltmp135-.Lfunc_begin16
	.uleb128 .Ltmp137-.Ltmp135
	.byte	0
	.byte	0
	.uleb128 .Ltmp137-.Lfunc_begin16
	.uleb128 .Ltmp138-.Ltmp137
	.uleb128 .Ltmp139-.Lfunc_begin16
	.byte	1
	.uleb128 .Ltmp140-.Lfunc_begin16
	.uleb128 .Ltmp141-.Ltmp140
	.uleb128 .Ltmp147-.Lfunc_begin16
	.byte	1
	.uleb128 .Ltmp141-.Lfunc_begin16
	.uleb128 .Ltmp145-.Ltmp141
	.byte	0
	.byte	0
	.uleb128 .Ltmp145-.Lfunc_begin16
	.uleb128 .Ltmp146-.Ltmp145
	.uleb128 .Ltmp147-.Lfunc_begin16
	.byte	1
	.uleb128 .Ltmp146-.Lfunc_begin16
	.uleb128 .Lfunc_end16-.Ltmp146
	.byte	0
	.byte	0
.Lcst_end8:
	.byte	127
	.byte	0
	.byte	0
	.byte	0
	.byte	1
	.byte	125
	.p2align	2, 0x0
	.long	0
.Lttbase3:
	.byte	0
	.p2align	2, 0x0

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtNtB4_4iter8adapters9enumerate9EnumerateINtNtBG_3zip3ZipINtNtNtCsc70TAahYccp_5alloc3vec9into_iter8IntoIterINtNtCs64OF0TycdZY_15crossbeam_deque5deque6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEIB1C_INtB2s_7StealerB3c_EEEEEB3g_,"ax",@progbits
	.prefalign	4, .Lfunc_end17, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtNtB4_4iter8adapters9enumerate9EnumerateINtNtBG_3zip3ZipINtNtNtCsc70TAahYccp_5alloc3vec9into_iter8IntoIterINtNtCs64OF0TycdZY_15crossbeam_deque5deque6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEIB1C_INtB2s_7StealerB3c_EEEEEB3g_,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtNtB4_4iter8adapters9enumerate9EnumerateINtNtBG_3zip3ZipINtNtNtCsc70TAahYccp_5alloc3vec9into_iter8IntoIterINtNtCs64OF0TycdZY_15crossbeam_deque5deque6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEIB1C_INtB2s_7StealerB3c_EEEEEB3g_:
.Lfunc_begin17:
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
	movq	8(%rdi), %r14
	movq	24(%rdi), %r15
	movq	%rdi, %rbx
	subq	%r14, %r15
	je	.LBB17_5
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %r12
	shrq	$5, %r15
	jmp	.LBB17_2
	.p2align	4
.LBB17_4:
	addq	$32, %r14
	decq	%r15
	je	.LBB17_5
.LBB17_2:
	movq	(%r14), %rax
	lock		decq	(%rax)
	jne	.LBB17_4
	movq	%r14, %rdi
	#MEMBARRIER
	callq	*%r12
	jmp	.LBB17_4
.LBB17_5:
	movq	16(%rbx), %rcx
	movabsq	$9223372036854775807, %r15
	testq	%rcx, %rcx
	je	.LBB17_15
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	shlq	$5, %rcx
	movq	(%rbx), %rdi
	cmpq	%r15, %rcx
	cmovaeq	%r15, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%r15, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB17_8
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB17_8:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB17_14
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB17_8
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r15, %rax
	subq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB17_11:
	cmpq	%rax, %rdx
	jge	.LBB17_13
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB17_11
.LBB17_13:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB17_14:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB17_15:
	movq	40(%rbx), %r14
	movq	56(%rbx), %r12
	subq	%r14, %r12
	je	.LBB17_20
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %r13
	shrq	$4, %r12
	jmp	.LBB17_17
	.p2align	4
.LBB17_19:
	addq	$16, %r14
	decq	%r12
	je	.LBB17_20
.LBB17_17:
	movq	(%r14), %rax
	lock		decq	(%rax)
	jne	.LBB17_19
	movq	%r14, %rdi
	#MEMBARRIER
	callq	*%r13
	jmp	.LBB17_19
.LBB17_20:
	movq	48(%rbx), %rcx
	testq	%rcx, %rcx
	je	.LBB17_30
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	shlq	$4, %rcx
	movq	32(%rbx), %rdi
	cmpq	%r15, %rcx
	cmovaeq	%r15, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%r15, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB17_23
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB17_23:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB17_29
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB17_23
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r15, %rax
	subq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB17_26:
	cmpq	%rax, %rdx
	jge	.LBB17_28
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB17_26
.LBB17_28:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB17_29:
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
	jmpq	*free@GOTPCREL(%rip)
.LBB17_30:
	.cfi_def_cfa_offset 48
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
.Lfunc_end17:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtNtB4_4iter8adapters9enumerate9EnumerateINtNtBG_3zip3ZipINtNtNtCsc70TAahYccp_5alloc3vec9into_iter8IntoIterINtNtCs64OF0TycdZY_15crossbeam_deque5deque6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEIB1C_INtB2s_7StealerB3c_EEEEEB3g_, .Lfunc_end17-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtNtB4_4iter8adapters9enumerate9EnumerateINtNtBG_3zip3ZipINtNtNtCsc70TAahYccp_5alloc3vec9into_iter8IntoIterINtNtCs64OF0TycdZY_15crossbeam_deque5deque6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEIB1C_INtB2s_7StealerB3c_EEEEEB3g_
	.cfi_endproc

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtNtCs7jcFBdfocI9_3std4sync6poison5mutex10MutexGuardbEECs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483,"ax",@progbits
	.hidden	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtNtCs7jcFBdfocI9_3std4sync6poison5mutex10MutexGuardbEECs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483
	.globl	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtNtCs7jcFBdfocI9_3std4sync6poison5mutex10MutexGuardbEECs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483
	.prefalign	4, .Lfunc_end18, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtNtCs7jcFBdfocI9_3std4sync6poison5mutex10MutexGuardbEECs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtNtCs7jcFBdfocI9_3std4sync6poison5mutex10MutexGuardbEECs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483:
.Lfunc_begin18:
	.cfi_startproc
	pushq	%rbx
	.cfi_def_cfa_offset 16
	.cfi_offset %rbx, -16
	testb	$1, %sil
	jne	.LBB18_4
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count18GLOBAL_PANIC_COUNT@GOTPCREL(%rip), %rax
	movq	(%rax), %rax
	shlq	%rax
	testq	%rax, %rax
	jne	.LBB18_2
.LBB18_4:
	xorl	%eax, %eax
	xchgl	%eax, (%rdi)
	cmpl	$2, %eax
	je	.LBB18_6
	popq	%rbx
	.cfi_def_cfa_offset 8
	retq
.LBB18_6:
	.cfi_def_cfa_offset 16
	movq	%rdi, %rsi
	movl	$202, %edi
	movl	$129, %edx
	movl	$1, %ecx
	xorl	%eax, %eax
	popq	%rbx
	.cfi_def_cfa_offset 8
	jmpq	*syscall@GOTPCREL(%rip)
.LBB18_2:
	.cfi_def_cfa_offset 16
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count17is_zero_slow_path@GOTPCREL(%rip), %rax
	movq	%rdi, %rbx
	callq	*%rax
	movq	%rbx, %rdi
	testb	%al, %al
	jne	.LBB18_4
	movb	$1, 4(%rdi)
	jmp	.LBB18_4
.Lfunc_end18:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtNtCs7jcFBdfocI9_3std4sync6poison5mutex10MutexGuardbEECs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483, .Lfunc_end18-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtNtCs7jcFBdfocI9_3std4sync6poison5mutex10MutexGuardbEECs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483
	.cfi_endproc

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNvNtNtB4_2io5write17default_write_fmt7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrEECs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.prefalign	4, .Lfunc_end19, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNvNtNtB4_2io5write17default_write_fmt7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrEECs6a8jV7kq6PJ_10rayon_core,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNvNtNtB4_2io5write17default_write_fmt7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrEECs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin19:
	.cfi_startproc
	movq	8(%rdi), %rax
	testq	%rax, %rax
	je	.LBB19_3
	movl	%eax, %ecx
	andl	$3, %ecx
	leal	-2(%rcx), %edx
	cmpl	$2, %edx
	jb	.LBB19_3
	testq	%rcx, %rcx
	jne	.LBB19_4
.LBB19_3:
	retq
.LBB19_4:
	leaq	-1(%rax), %rdi
	jmpq	*23(%rax)
.Lfunc_end19:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNvNtNtB4_2io5write17default_write_fmt7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrEECs6a8jV7kq6PJ_10rayon_core, .Lfunc_end19-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNvNtNtB4_2io5write17default_write_fmt7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrEECs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNCINvNtNtCs7jcFBdfocI9_3std6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1I_12DefaultSpawnNtB1I_11ThreadSpawn5spawn0uEs_0EB1K_,"ax",@progbits
	.prefalign	4, .Lfunc_end20, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNCINvNtNtCs7jcFBdfocI9_3std6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1I_12DefaultSpawnNtB1I_11ThreadSpawn5spawn0uEs_0EB1K_,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNCINvNtNtCs7jcFBdfocI9_3std6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1I_12DefaultSpawnNtB1I_11ThreadSpawn5spawn0uEs_0EB1K_:
.Lfunc_begin20:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception9
	pushq	%r14
	.cfi_def_cfa_offset 16
	pushq	%rbx
	.cfi_def_cfa_offset 24
	pushq	%rax
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -24
	.cfi_offset %r14, -16
	movq	%rdi, %rbx
	addq	$104, %rdi
.Ltmp148:
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtNtCs7jcFBdfocI9_3std6thread9spawnhook15ChildSpawnHooksECs6a8jV7kq6PJ_10rayon_core
.Ltmp149:
	movq	16(%rbx), %rcx
	cmpq	$-1, %rcx
	je	.LBB20_12
	testq	%rcx, %rcx
	je	.LBB20_12
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	24(%rbx), %rdi
	xorl	%esi, %esi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB20_5
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB20_5:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB20_11
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB20_5
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB20_8:
	cmpq	%rax, %rsi
	jge	.LBB20_10
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB20_8
.LBB20_10:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB20_11:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB20_12:
	movq	56(%rbx), %rax
	lock		decq	(%rax)
	je	.LBB20_13
	movq	40(%rbx), %rax
	lock		decq	(%rax)
	je	.LBB20_15
.LBB20_16:
	movq	88(%rbx), %rax
	lock		decq	(%rax)
	je	.LBB20_17
	jmp	.LBB20_18
.LBB20_13:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %rax
	leaq	56(%rbx), %rdi
	#MEMBARRIER
	callq	*%rax
	movq	40(%rbx), %rax
	lock		decq	(%rax)
	jne	.LBB20_16
.LBB20_15:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %rax
	leaq	40(%rbx), %rdi
	#MEMBARRIER
	callq	*%rax
	movq	88(%rbx), %rax
	lock		decq	(%rax)
	jne	.LBB20_18
.LBB20_17:
	leaq	88(%rbx), %rdi
	#MEMBARRIER
.Ltmp153:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryE9drop_slowBK_@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp154:
.LBB20_18:
	movq	136(%rbx), %rax
	lock		decq	(%rax)
	jne	.LBB20_25
	addq	$136, %rbx
	#MEMBARRIER
	movq	%rbx, %rdi
	addq	$8, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEE9drop_slowCs6a8jV7kq6PJ_10rayon_core@GOTPCREL(%rip)
.LBB20_25:
	.cfi_def_cfa_offset 32
	addq	$8, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	retq
.LBB20_24:
	.cfi_def_cfa_offset 32
.Ltmp155:
	movq	%rax, %r14
	jmp	.LBB20_21
.LBB20_20:
.Ltmp150:
	movq	%rax, %r14
.Ltmp151:
	movq	%rbx, %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtBJ_12DefaultSpawnNtBJ_11ThreadSpawn5spawn0EBL_
.Ltmp152:
.LBB20_21:
	movq	136(%rbx), %rax
	lock		decq	(%rax)
	jne	.LBB20_23
	addq	$136, %rbx
	#MEMBARRIER
.Ltmp156:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEE9drop_slowCs6a8jV7kq6PJ_10rayon_core@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp157:
.LBB20_23:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB20_26:
.Ltmp158:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end20:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNCINvNtNtCs7jcFBdfocI9_3std6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1I_12DefaultSpawnNtB1I_11ThreadSpawn5spawn0uEs_0EB1K_, .Lfunc_end20-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNCINvNtNtCs7jcFBdfocI9_3std6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1I_12DefaultSpawnNtB1I_11ThreadSpawn5spawn0uEs_0EB1K_
	.cfi_endproc
	.section	.gcc_except_table._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNCINvNtNtCs7jcFBdfocI9_3std6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1I_12DefaultSpawnNtB1I_11ThreadSpawn5spawn0uEs_0EB1K_,"a",@progbits
	.p2align	2, 0x0
GCC_except_table20:
.Lexception9:
	.byte	255
	.byte	155
	.uleb128 .Lttbase4-.Lttbaseref4
.Lttbaseref4:
	.byte	1
	.uleb128 .Lcst_end9-.Lcst_begin9
.Lcst_begin9:
	.uleb128 .Ltmp148-.Lfunc_begin20
	.uleb128 .Ltmp149-.Ltmp148
	.uleb128 .Ltmp150-.Lfunc_begin20
	.byte	0
	.uleb128 .Ltmp149-.Lfunc_begin20
	.uleb128 .Ltmp153-.Ltmp149
	.byte	0
	.byte	0
	.uleb128 .Ltmp153-.Lfunc_begin20
	.uleb128 .Ltmp154-.Ltmp153
	.uleb128 .Ltmp155-.Lfunc_begin20
	.byte	0
	.uleb128 .Ltmp154-.Lfunc_begin20
	.uleb128 .Ltmp151-.Ltmp154
	.byte	0
	.byte	0
	.uleb128 .Ltmp151-.Lfunc_begin20
	.uleb128 .Ltmp157-.Ltmp151
	.uleb128 .Ltmp158-.Lfunc_begin20
	.byte	1
	.uleb128 .Ltmp157-.Lfunc_begin20
	.uleb128 .Lfunc_end20-.Ltmp157
	.byte	0
	.byte	0
.Lcst_end9:
	.byte	127
	.byte	0
	.p2align	2, 0x0
.Lttbase4:
	.byte	0
	.p2align	2, 0x0

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtBJ_12DefaultSpawnNtBJ_11ThreadSpawn5spawn0EBL_,"ax",@progbits
	.prefalign	4, .Lfunc_end21, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtBJ_12DefaultSpawnNtBJ_11ThreadSpawn5spawn0EBL_,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtBJ_12DefaultSpawnNtBJ_11ThreadSpawn5spawn0EBL_:
.Lfunc_begin21:
	.cfi_startproc
	pushq	%rbx
	.cfi_def_cfa_offset 16
	.cfi_offset %rbx, -16
	movq	16(%rdi), %rcx
	movq	%rdi, %rbx
	cmpq	$-1, %rcx
	je	.LBB21_11
	testq	%rcx, %rcx
	je	.LBB21_11
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	24(%rbx), %rdi
	xorl	%esi, %esi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB21_4
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB21_4:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB21_10
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB21_4
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB21_7:
	cmpq	%rax, %rsi
	jge	.LBB21_9
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB21_7
.LBB21_9:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB21_10:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB21_11:
	movq	56(%rbx), %rax
	lock		decq	(%rax)
	je	.LBB21_12
	movq	40(%rbx), %rax
	lock		decq	(%rax)
	je	.LBB21_14
.LBB21_15:
	movq	88(%rbx), %rax
	lock		decq	(%rax)
	jne	.LBB21_16
.LBB21_17:
	addq	$88, %rbx
	#MEMBARRIER
	movq	%rbx, %rdi
	popq	%rbx
	.cfi_def_cfa_offset 8
	jmpq	*_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryE9drop_slowBK_@GOTPCREL(%rip)
.LBB21_12:
	.cfi_def_cfa_offset 16
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %rax
	leaq	56(%rbx), %rdi
	#MEMBARRIER
	callq	*%rax
	movq	40(%rbx), %rax
	lock		decq	(%rax)
	jne	.LBB21_15
.LBB21_14:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %rax
	leaq	40(%rbx), %rdi
	#MEMBARRIER
	callq	*%rax
	movq	88(%rbx), %rax
	lock		decq	(%rax)
	je	.LBB21_17
.LBB21_16:
	popq	%rbx
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end21:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtBJ_12DefaultSpawnNtBJ_11ThreadSpawn5spawn0EBL_, .Lfunc_end21-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtBJ_12DefaultSpawnNtBJ_11ThreadSpawn5spawn0EBL_
	.cfi_endproc

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtCs6a8jV7kq6PJ_10rayon_core17ThreadPoolBuilderEBD_,"ax",@progbits
	.prefalign	4, .Lfunc_end22, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtCs6a8jV7kq6PJ_10rayon_core17ThreadPoolBuilderEBD_,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtCs6a8jV7kq6PJ_10rayon_core17ThreadPoolBuilderEBD_:
.Lfunc_begin22:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception10
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
	movq	24(%rdi), %r15
	movq	%rdi, %rbx
	testq	%r15, %r15
	je	.LBB22_13
	movq	32(%rbx), %r12
	movq	(%r12), %rax
	testq	%rax, %rax
	je	.LBB22_3
.Ltmp159:
	movq	%r15, %rdi
	callq	*%rax
.Ltmp160:
.LBB22_3:
	movq	8(%r12), %rcx
	testq	%rcx, %rcx
	je	.LBB22_13
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	xorl	%esi, %esi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB22_6
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB22_6:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB22_12
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB22_6
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB22_9:
	cmpq	%rax, %rsi
	jge	.LBB22_11
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB22_9
.LBB22_11:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB22_12:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.LBB22_13:
	movq	40(%rbx), %r15
	testq	%r15, %r15
	je	.LBB22_26
	movq	48(%rbx), %r12
	movq	(%r12), %rax
	testq	%rax, %rax
	je	.LBB22_16
.Ltmp164:
	movq	%r15, %rdi
	callq	*%rax
.Ltmp165:
.LBB22_16:
	movq	8(%r12), %rcx
	testq	%rcx, %rcx
	je	.LBB22_26
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	xorl	%esi, %esi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB22_19
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB22_19:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB22_25
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB22_19
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB22_22:
	cmpq	%rax, %rsi
	jge	.LBB22_24
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB22_22
.LBB22_24:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB22_25:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.LBB22_26:
	movq	56(%rbx), %r15
	testq	%r15, %r15
	je	.LBB22_39
	movq	64(%rbx), %r12
	movq	(%r12), %rax
	testq	%rax, %rax
	je	.LBB22_29
.Ltmp169:
	movq	%r15, %rdi
	callq	*%rax
.Ltmp170:
.LBB22_29:
	movq	8(%r12), %rcx
	testq	%rcx, %rcx
	je	.LBB22_39
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	xorl	%esi, %esi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB22_32
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB22_32:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB22_38
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB22_32
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB22_35:
	cmpq	%rax, %rsi
	jge	.LBB22_37
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB22_35
.LBB22_37:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB22_38:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.LBB22_39:
	movq	72(%rbx), %r15
	testq	%r15, %r15
	je	.LBB22_64
	movq	80(%rbx), %rbx
	movq	(%rbx), %rax
	testq	%rax, %rax
	je	.LBB22_42
.Ltmp175:
	movq	%r15, %rdi
	callq	*%rax
.Ltmp176:
.LBB22_42:
	movq	8(%rbx), %rcx
	testq	%rcx, %rcx
	je	.LBB22_64
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	xorl	%esi, %esi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB22_45
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB22_45:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB22_51
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB22_45
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB22_48:
	cmpq	%rax, %rsi
	jge	.LBB22_50
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB22_48
.LBB22_50:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB22_51:
	movq	%r15, %rdi
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
	jmpq	*free@GOTPCREL(%rip)
.LBB22_64:
	.cfi_def_cfa_offset 48
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
.LBB22_61:
	.cfi_def_cfa_offset 48
.Ltmp177:
	movq	8(%rbx), %rsi
	movq	%rax, %r14
	testq	%rsi, %rsi
	je	.LBB22_63
	movq	16(%rbx), %rdx
	movq	%r15, %rdi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB22_58:
.Ltmp171:
	movq	8(%r12), %rsi
	movq	%rax, %r14
	testq	%rsi, %rsi
	je	.LBB22_60
	movq	16(%r12), %rdx
	movq	%r15, %rdi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
	jmp	.LBB22_60
.LBB22_55:
.Ltmp166:
	movq	8(%r12), %rsi
	movq	%rax, %r14
	testq	%rsi, %rsi
	je	.LBB22_57
	movq	16(%r12), %rdx
	movq	%r15, %rdi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
	jmp	.LBB22_57
.LBB22_52:
.Ltmp161:
	movq	8(%r12), %rsi
	movq	%rax, %r14
	testq	%rsi, %rsi
	je	.LBB22_54
	movq	16(%r12), %rdx
	movq	%r15, %rdi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
.LBB22_54:
	movq	40(%rbx), %rdi
	movq	48(%rbx), %rsi
.Ltmp162:
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6option6OptionINtNtCsc70TAahYccp_5alloc5boxed3BoxDINtNtNtB4_3ops8function2FnTjEEp6OutputuNtNtB4_6marker4SyncNtB2c_4SendEL_EEECs6a8jV7kq6PJ_10rayon_core
.Ltmp163:
.LBB22_57:
	movq	56(%rbx), %rdi
	movq	64(%rbx), %rsi
.Ltmp167:
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6option6OptionINtNtCsc70TAahYccp_5alloc5boxed3BoxDINtNtNtB4_3ops8function2FnTjEEp6OutputuNtNtB4_6marker4SyncNtB2c_4SendEL_EEECs6a8jV7kq6PJ_10rayon_core
.Ltmp168:
.LBB22_60:
	movq	72(%rbx), %rdi
	movq	80(%rbx), %rsi
.Ltmp172:
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6option6OptionINtNtCsc70TAahYccp_5alloc5boxed3BoxDINtNtNtB4_3ops8function2FnTjEEp6OutputuNtNtB4_6marker4SyncNtB2c_4SendEL_EEECs6a8jV7kq6PJ_10rayon_core
.Ltmp173:
.LBB22_63:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB22_65:
.Ltmp174:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end22:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtCs6a8jV7kq6PJ_10rayon_core17ThreadPoolBuilderEBD_, .Lfunc_end22-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtCs6a8jV7kq6PJ_10rayon_core17ThreadPoolBuilderEBD_
	.cfi_endproc
	.section	.gcc_except_table._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtCs6a8jV7kq6PJ_10rayon_core17ThreadPoolBuilderEBD_,"a",@progbits
	.p2align	2, 0x0
GCC_except_table22:
.Lexception10:
	.byte	255
	.byte	155
	.uleb128 .Lttbase5-.Lttbaseref5
.Lttbaseref5:
	.byte	1
	.uleb128 .Lcst_end10-.Lcst_begin10
.Lcst_begin10:
	.uleb128 .Ltmp159-.Lfunc_begin22
	.uleb128 .Ltmp160-.Ltmp159
	.uleb128 .Ltmp161-.Lfunc_begin22
	.byte	0
	.uleb128 .Ltmp160-.Lfunc_begin22
	.uleb128 .Ltmp164-.Ltmp160
	.byte	0
	.byte	0
	.uleb128 .Ltmp164-.Lfunc_begin22
	.uleb128 .Ltmp165-.Ltmp164
	.uleb128 .Ltmp166-.Lfunc_begin22
	.byte	0
	.uleb128 .Ltmp165-.Lfunc_begin22
	.uleb128 .Ltmp169-.Ltmp165
	.byte	0
	.byte	0
	.uleb128 .Ltmp169-.Lfunc_begin22
	.uleb128 .Ltmp170-.Ltmp169
	.uleb128 .Ltmp171-.Lfunc_begin22
	.byte	0
	.uleb128 .Ltmp170-.Lfunc_begin22
	.uleb128 .Ltmp175-.Ltmp170
	.byte	0
	.byte	0
	.uleb128 .Ltmp175-.Lfunc_begin22
	.uleb128 .Ltmp176-.Ltmp175
	.uleb128 .Ltmp177-.Lfunc_begin22
	.byte	0
	.uleb128 .Ltmp176-.Lfunc_begin22
	.uleb128 .Ltmp162-.Ltmp176
	.byte	0
	.byte	0
	.uleb128 .Ltmp162-.Lfunc_begin22
	.uleb128 .Ltmp173-.Ltmp162
	.uleb128 .Ltmp174-.Lfunc_begin22
	.byte	1
	.uleb128 .Ltmp173-.Lfunc_begin22
	.uleb128 .Lfunc_end22-.Ltmp173
	.byte	0
	.byte	0
.Lcst_end10:
	.byte	127
	.byte	0
	.p2align	2, 0x0
.Lttbase5:
	.byte	0
	.p2align	2, 0x0

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtCs6a8jV7kq6PJ_10rayon_core20ThreadPoolBuildErrorEBD_.llvm.7294274987384275483,"ax",@progbits
	.hidden	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtCs6a8jV7kq6PJ_10rayon_core20ThreadPoolBuildErrorEBD_.llvm.7294274987384275483
	.globl	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtCs6a8jV7kq6PJ_10rayon_core20ThreadPoolBuildErrorEBD_.llvm.7294274987384275483
	.prefalign	4, .Lfunc_end23, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtCs6a8jV7kq6PJ_10rayon_core20ThreadPoolBuildErrorEBD_.llvm.7294274987384275483,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtCs6a8jV7kq6PJ_10rayon_core20ThreadPoolBuildErrorEBD_.llvm.7294274987384275483:
.Lfunc_begin23:
	.cfi_startproc
	cmpl	$2, (%rdi)
	jb	.LBB23_3
	movq	8(%rdi), %rax
	movl	%eax, %ecx
	andl	$3, %ecx
	leal	-2(%rcx), %edx
	cmpl	$2, %edx
	jb	.LBB23_3
	testq	%rcx, %rcx
	jne	.LBB23_4
.LBB23_3:
	retq
.LBB23_4:
	leaq	-1(%rax), %rdi
	jmpq	*23(%rax)
.Lfunc_end23:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtCs6a8jV7kq6PJ_10rayon_core20ThreadPoolBuildErrorEBD_.llvm.7294274987384275483, .Lfunc_end23-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtCs6a8jV7kq6PJ_10rayon_core20ThreadPoolBuildErrorEBD_.llvm.7294274987384275483
	.cfi_endproc

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs18aJq3QiqAb_15crossbeam_epoch5guard5GuardECs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.prefalign	4, .Lfunc_end24, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs18aJq3QiqAb_15crossbeam_epoch5guard5GuardECs6a8jV7kq6PJ_10rayon_core,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs18aJq3QiqAb_15crossbeam_epoch5guard5GuardECs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin24:
	.cfi_startproc
	testq	%rdi, %rdi
	je	.LBB24_3
	decq	2072(%rdi)
	jne	.LBB24_3
	movq	$0, 2176(%rdi)
	cmpq	$0, 2080(%rdi)
	je	.LBB24_4
.LBB24_3:
	retq
.LBB24_4:
	jmpq	*_RNvMs6_NtCs18aJq3QiqAb_15crossbeam_epoch8internalNtB5_5Local8finalize@GOTPCREL(%rip)
.Lfunc_end24:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs18aJq3QiqAb_15crossbeam_epoch5guard5GuardECs6a8jV7kq6PJ_10rayon_core, .Lfunc_end24-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs18aJq3QiqAb_15crossbeam_epoch5guard5GuardECs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core3job7JobFifoEBF_,"ax",@progbits
	.prefalign	4, .Lfunc_end25, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core3job7JobFifoEBF_,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core3job7JobFifoEBF_:
.Lfunc_begin25:
	.cfi_startproc
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
	pushq	%rax
	.cfi_def_cfa_offset 64
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	128(%rdi), %r15
	movq	(%rdi), %r12
	movq	8(%rdi), %rdi
	movabsq	$-9223372036854775808, %r14
	andq	$-2, %r12
	andq	$-2, %r15
	cmpq	%r15, %r12
	jne	.LBB25_2
	movq	%rdi, %rbx
	jmp	.LBB25_15
.LBB25_2:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	jmp	.LBB25_3
	.p2align	4
.LBB25_4:
	movq	%rdi, %rbx
	addq	$2, %r12
	cmpq	%r15, %r12
	je	.LBB25_15
.LBB25_3:
	movl	%r12d, %eax
	notl	%eax
	testb	$126, %al
	jne	.LBB25_4
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	(%rdi), %rbx
	movq	$-1520, %rcx
	addq	%rcx, %rax
	cmovoq	%r14, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB25_7
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB25_7:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB25_13
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB25_7
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	$-1520, %rcx
	lock		xaddq	%rcx, (%rax)
	movq	(%rbp), %rax
	addq	$-1520, %rcx
	cmovoq	%r14, %rcx
	.p2align	4
.LBB25_10:
	cmpq	%rax, %rcx
	jge	.LBB25_12
	lock		cmpxchgq	%rcx, (%rbp)
	jne	.LBB25_10
.LBB25_12:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB25_13:
	callq	*%r13
	movq	%rbx, %rdi
	addq	$2, %r12
	cmpq	%r15, %r12
	jne	.LBB25_3
.LBB25_15:
	movq	$-1520, %rax
	addq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	cmovoq	%r14, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB25_17
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB25_17:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB25_23
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB25_17
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	movq	$-1520, %rcx
	lock		xaddq	%rcx, (%rax)
	movq	(%rdx), %rax
	addq	$-1520, %rcx
	cmovoq	%r14, %rcx
	.p2align	4
.LBB25_20:
	cmpq	%rax, %rcx
	jge	.LBB25_22
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB25_20
.LBB25_22:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB25_23:
	movq	%rbx, %rdi
	addq	$8, %rsp
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
	jmpq	*free@GOTPCREL(%rip)
.Lfunc_end25:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core3job7JobFifoEBF_, .Lfunc_end25-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core3job7JobFifoEBF_
	.cfi_endproc

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry10TerminatorEBF_,"ax",@progbits
	.prefalign	4, .Lfunc_end26, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry10TerminatorEBF_,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry10TerminatorEBF_:
.Lfunc_begin26:
	.cfi_startproc
	lock		decq	464(%rdi)
	jne	.LBB26_6
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
	movq	520(%rdi), %rax
	movq	%rdi, %rbx
	testq	%rax, %rax
	je	.LBB26_5
	movq	512(%rbx), %r15
	shlq	$4, %rax
	addq	$472, %rbx
	xorl	%r14d, %r14d
	xorl	%r13d, %r13d
	leaq	(%rax,%rax,2), %r12
	jmp	.LBB26_3
	.p2align	4
.LBB26_4:
	addq	$48, %r13
	incq	%r14
	cmpq	%r13, %r12
	je	.LBB26_5
.LBB26_3:
	movl	$3, %eax
	xchgq	%rax, 16(%r15,%r13)
	cmpq	$2, %rax
	jne	.LBB26_4
	movq	%rbx, %rdi
	movq	%r14, %rsi
	callq	_RNvMNtCs6a8jV7kq6PJ_10rayon_core5sleepNtB2_5Sleep20wake_specific_thread.llvm.7294274987384275483
	jmp	.LBB26_4
.LBB26_5:
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
	.cfi_restore %rbx
	.cfi_restore %r12
	.cfi_restore %r13
	.cfi_restore %r14
	.cfi_restore %r15
.LBB26_6:
	retq
.Lfunc_end26:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry10TerminatorEBF_, .Lfunc_end26-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry10TerminatorEBF_
	.cfi_endproc

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry12WorkerThreadEBF_,"ax",@progbits
	.prefalign	4, .Lfunc_end27, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry12WorkerThreadEBF_,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry12WorkerThreadEBF_:
.Lfunc_begin27:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception11
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
	pushq	%rax
	.cfi_def_cfa_offset 64
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	_RNvNCNKNvNtCs6a8jV7kq6PJ_10rayon_core8registry19WORKER_THREAD_STATE0s_023___RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rax
	movq	%rdi, %rbx
	cmpq	%rdi, %fs:(%rax)
	jne	.LBB27_1
	movq	280(%rbx), %rcx
	movq	$0, %fs:(%rax)
	lock		decq	(%rcx)
	jne	.LBB27_12
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %rax
	leaq	280(%rbx), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB27_12:
	movq	312(%rbx), %rax
	lock		decq	(%rax)
	jne	.LBB27_14
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %rax
	leaq	312(%rbx), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB27_14:
	movq	128(%rbx), %r12
	movq	(%rbx), %r13
	movq	8(%rbx), %rdi
	movabsq	$-9223372036854775808, %r15
	movq	%rbx, (%rsp)
	andq	$-2, %r13
	andq	$-2, %r12
	cmpq	%r12, %r13
	jne	.LBB27_16
	movq	%rdi, %r14
	jmp	.LBB27_29
.LBB27_16:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbx
	movq	free@GOTPCREL(%rip), %rbp
	jmp	.LBB27_17
	.p2align	4
.LBB27_18:
	movq	%rdi, %r14
	addq	$2, %r13
	cmpq	%r12, %r13
	je	.LBB27_29
.LBB27_17:
	movl	%r13d, %eax
	notl	%eax
	testb	$126, %al
	jne	.LBB27_18
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	(%rdi), %r14
	movq	$-1520, %rcx
	addq	%rcx, %rax
	cmovoq	%r15, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB27_21
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB27_21:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB27_27
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB27_21
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	$-1520, %rcx
	lock		xaddq	%rcx, (%rax)
	movq	(%rbx), %rax
	addq	$-1520, %rcx
	cmovoq	%r15, %rcx
	.p2align	4
.LBB27_24:
	cmpq	%rax, %rcx
	jge	.LBB27_26
	lock		cmpxchgq	%rcx, (%rbx)
	jne	.LBB27_24
.LBB27_26:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB27_27:
	callq	*%rbp
	movq	%r14, %rdi
	addq	$2, %r13
	cmpq	%r12, %r13
	jne	.LBB27_17
.LBB27_29:
	movq	$-1520, %rax
	addq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	cmovoq	%r15, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB27_31
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
.LBB27_31:
	movq	(%rsp), %rbx
	.p2align	4
.LBB27_32:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB27_38
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB27_32
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	movq	$-1520, %rcx
	lock		xaddq	%rcx, (%rax)
	movq	(%rdx), %rax
	addq	$-1520, %rcx
	cmovoq	%r15, %rcx
	.p2align	4
.LBB27_35:
	cmpq	%rax, %rcx
	jge	.LBB27_37
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB27_35
.LBB27_37:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB27_38:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
	movq	272(%rbx), %rax
	lock		decq	(%rax)
	jne	.LBB27_39
	addq	$272, %rbx
	#MEMBARRIER
	movq	%rbx, %rdi
	addq	$8, %rsp
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
	jmpq	*_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryE9drop_slowBK_@GOTPCREL(%rip)
.LBB27_39:
	.cfi_def_cfa_offset 64
	addq	$8, %rsp
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
.LBB27_1:
	.cfi_def_cfa_offset 64
.Ltmp178:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking5panic@GOTPCREL(%rip), %rax
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.21(%rip), %rdi
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.22(%rip), %rdx
	movl	$49, %esi
	callq	*%rax
.Ltmp179:
	ud2
.LBB27_3:
.Ltmp180:
	movq	280(%rbx), %rcx
	movq	%rax, %r14
	lock		decq	(%rcx)
	jne	.LBB27_5
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %rax
	leaq	280(%rbx), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB27_5:
	movq	312(%rbx), %rax
	lock		decq	(%rax)
	jne	.LBB27_7
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %rax
	leaq	312(%rbx), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB27_7:
	movq	%rbx, %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core3job7JobFifoEBF_
	movq	272(%rbx), %rax
	lock		decq	(%rax)
	jne	.LBB27_9
	addq	$272, %rbx
	#MEMBARRIER
.Ltmp181:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryE9drop_slowBK_@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp182:
.LBB27_9:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB27_40:
.Ltmp183:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end27:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry12WorkerThreadEBF_, .Lfunc_end27-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry12WorkerThreadEBF_
	.cfi_endproc
	.section	.gcc_except_table._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry12WorkerThreadEBF_,"a",@progbits
	.p2align	2, 0x0
GCC_except_table27:
.Lexception11:
	.byte	255
	.byte	155
	.uleb128 .Lttbase6-.Lttbaseref6
.Lttbaseref6:
	.byte	1
	.uleb128 .Lcst_end11-.Lcst_begin11
.Lcst_begin11:
	.uleb128 .Lfunc_begin27-.Lfunc_begin27
	.uleb128 .Ltmp178-.Lfunc_begin27
	.byte	0
	.byte	0
	.uleb128 .Ltmp178-.Lfunc_begin27
	.uleb128 .Ltmp179-.Ltmp178
	.uleb128 .Ltmp180-.Lfunc_begin27
	.byte	0
	.uleb128 .Ltmp179-.Lfunc_begin27
	.uleb128 .Ltmp181-.Ltmp179
	.byte	0
	.byte	0
	.uleb128 .Ltmp181-.Lfunc_begin27
	.uleb128 .Ltmp182-.Ltmp181
	.uleb128 .Ltmp183-.Lfunc_begin27
	.byte	1
	.uleb128 .Ltmp182-.Lfunc_begin27
	.uleb128 .Lfunc_end27-.Ltmp182
	.byte	0
	.byte	0
.Lcst_end11:
	.byte	127
	.byte	0
	.p2align	2, 0x0
.Lttbase6:
	.byte	0
	.p2align	2, 0x0

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryEBF_,"ax",@progbits
	.prefalign	4, .Lfunc_end28, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryEBF_,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryEBF_:
.Lfunc_begin28:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception12
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
	pushq	%rax
	.cfi_def_cfa_offset 64
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	384(%rdi), %r14
	movq	392(%rdi), %rbx
	movq	%rdi, %rbp
	testq	%rbx, %rbx
	je	.LBB28_5
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %r12
	movq	%r14, %r15
	jmp	.LBB28_2
	.p2align	4
.LBB28_4:
	addq	$48, %r15
	decq	%rbx
	je	.LBB28_5
.LBB28_2:
	movq	(%r15), %rax
	lock		decq	(%rax)
	jne	.LBB28_4
	movq	%r15, %rdi
	#MEMBARRIER
	callq	*%r12
	jmp	.LBB28_4
.LBB28_5:
	movq	376(%rbp), %rax
	movabsq	$9223372036854775807, %rbx
	testq	%rax, %rax
	je	.LBB28_15
	shlq	$4, %rax
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	cmpq	%rbx, %rcx
	cmovaeq	%rbx, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rbx, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB28_8
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB28_8:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB28_14
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB28_8
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rbx, %rax
	subq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB28_11:
	cmpq	%rax, %rdx
	jge	.LBB28_13
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB28_11
.LBB28_13:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB28_14:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
.LBB28_15:
	movq	344(%rbp), %rcx
	testq	%rcx, %rcx
	je	.LBB28_25
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	shlq	$7, %rcx
	movq	352(%rbp), %rdi
	cmpq	%rbx, %rcx
	cmovaeq	%rbx, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rbx, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB28_18
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB28_18:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB28_24
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB28_18
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rbx, %rax
	subq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB28_21:
	cmpq	%rax, %rdx
	jge	.LBB28_23
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB28_21
.LBB28_23:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB28_24:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB28_25:
	movq	128(%rbp), %r13
	movq	%rbp, %rax
	movq	(%rbp), %rbp
	movq	8(%rax), %rdi
	movabsq	$-9223372036854775808, %r15
	movq	%rax, (%rsp)
	andq	$-2, %rbp
	andq	$-2, %r13
	cmpq	%r13, %rbp
	jne	.LBB28_27
	movq	%rdi, %r14
	jmp	.LBB28_40
.LBB28_27:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbx
	movq	free@GOTPCREL(%rip), %r12
	jmp	.LBB28_28
	.p2align	4
.LBB28_29:
	movq	%rdi, %r14
	addq	$2, %rbp
	cmpq	%r13, %rbp
	je	.LBB28_40
.LBB28_28:
	movl	%ebp, %eax
	notl	%eax
	testb	$126, %al
	jne	.LBB28_29
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	(%rdi), %r14
	movq	$-1520, %rcx
	addq	%rcx, %rax
	cmovoq	%r15, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB28_32
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB28_32:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB28_38
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB28_32
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	$-1520, %rcx
	lock		xaddq	%rcx, (%rax)
	movq	(%rbx), %rax
	addq	$-1520, %rcx
	cmovoq	%r15, %rcx
	.p2align	4
.LBB28_35:
	cmpq	%rax, %rcx
	jge	.LBB28_37
	lock		cmpxchgq	%rcx, (%rbx)
	jne	.LBB28_35
.LBB28_37:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB28_38:
	callq	*%r12
	movq	%r14, %rdi
	addq	$2, %rbp
	cmpq	%r13, %rbp
	jne	.LBB28_28
.LBB28_40:
	movq	$-1520, %rax
	addq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	cmovoq	%r15, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB28_42
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
.LBB28_42:
	movq	(%rsp), %rbp
	.p2align	4
.LBB28_43:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB28_49
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB28_43
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	movq	$-1520, %rcx
	lock		xaddq	%rcx, (%rax)
	movq	(%rdx), %rax
	addq	$-1520, %rcx
	cmovoq	%r15, %rcx
	.p2align	4
.LBB28_46:
	cmpq	%rax, %rcx
	jge	.LBB28_48
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB28_46
.LBB28_48:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB28_49:
	movq	free@GOTPCREL(%rip), %r13
	movq	%r14, %rdi
	callq	*%r13
	movq	272(%rbp), %r14
	movq	280(%rbp), %rbx
	testq	%rbx, %rbx
	je	.LBB28_54
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %r12
	movq	%r14, %r15
	jmp	.LBB28_51
	.p2align	4
.LBB28_53:
	addq	$32, %r15
	decq	%rbx
	je	.LBB28_54
.LBB28_51:
	movq	(%r15), %rax
	lock		decq	(%rax)
	jne	.LBB28_53
	movq	%r15, %rdi
	#MEMBARRIER
	callq	*%r12
	jmp	.LBB28_53
.LBB28_54:
	movq	264(%rbp), %rcx
	movabsq	$9223372036854775807, %r15
	testq	%rcx, %rcx
	je	.LBB28_64
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	shlq	$5, %rcx
	cmpq	%r15, %rcx
	cmovaeq	%r15, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%r15, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB28_57
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB28_57:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB28_63
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB28_57
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r15, %rax
	subq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB28_60:
	cmpq	%rax, %rdx
	jge	.LBB28_62
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB28_60
.LBB28_62:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB28_63:
	movq	%r14, %rdi
	callq	*%r13
.LBB28_64:
	movq	288(%rbp), %r14
	testq	%r14, %r14
	je	.LBB28_77
	movq	296(%rbp), %rbx
	movq	(%rbx), %rax
	testq	%rax, %rax
	je	.LBB28_67
.Ltmp184:
	movq	%r14, %rdi
	callq	*%rax
.Ltmp185:
.LBB28_67:
	movq	8(%rbx), %rcx
	testq	%rcx, %rcx
	je	.LBB28_77
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%r15, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB28_70
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB28_70:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB28_76
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB28_70
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r15, %rax
	subq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB28_73:
	cmpq	%rax, %rdx
	jge	.LBB28_75
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB28_73
.LBB28_75:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB28_76:
	movq	%r14, %rdi
	callq	*%r13
.LBB28_77:
	movq	304(%rbp), %r14
	testq	%r14, %r14
	je	.LBB28_90
	movq	312(%rbp), %rbx
	movq	(%rbx), %rax
	testq	%rax, %rax
	je	.LBB28_80
.Ltmp189:
	movq	%r14, %rdi
	callq	*%rax
.Ltmp190:
.LBB28_80:
	movq	8(%rbx), %rcx
	testq	%rcx, %rcx
	je	.LBB28_90
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%r15, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB28_83
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB28_83:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB28_89
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB28_83
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r15, %rax
	subq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB28_86:
	cmpq	%rax, %rdx
	jge	.LBB28_88
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB28_86
.LBB28_88:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB28_89:
	movq	%r14, %rdi
	callq	*%r13
.LBB28_90:
	movq	320(%rbp), %r14
	testq	%r14, %r14
	je	.LBB28_112
	movq	328(%rbp), %rbx
	movq	(%rbx), %rax
	testq	%rax, %rax
	je	.LBB28_93
.Ltmp195:
	movq	%r14, %rdi
	callq	*%rax
.Ltmp196:
.LBB28_93:
	movq	8(%rbx), %rcx
	testq	%rcx, %rcx
	je	.LBB28_112
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%r15, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB28_96
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB28_96:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB28_102
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB28_96
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r15, %rax
	subq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB28_99:
	cmpq	%rax, %rdx
	jge	.LBB28_101
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB28_99
.LBB28_101:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB28_102:
	movq	%r14, %rdi
	addq	$8, %rsp
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
	jmpq	*free@GOTPCREL(%rip)
.LBB28_112:
	.cfi_def_cfa_offset 64
	addq	$8, %rsp
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
.LBB28_109:
	.cfi_def_cfa_offset 64
.Ltmp197:
	movq	8(%rbx), %rsi
	movq	%rax, %r15
	testq	%rsi, %rsi
	je	.LBB28_111
	movq	16(%rbx), %rdx
	movq	%r14, %rdi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
	movq	%r15, %rdi
	callq	_Unwind_Resume@PLT
.LBB28_106:
.Ltmp191:
	movq	8(%rbx), %rsi
	movq	%rax, %r15
	testq	%rsi, %rsi
	je	.LBB28_108
	movq	16(%rbx), %rdx
	movq	%r14, %rdi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
	jmp	.LBB28_108
.LBB28_103:
.Ltmp186:
	movq	8(%rbx), %rsi
	movq	%rax, %r15
	testq	%rsi, %rsi
	je	.LBB28_105
	movq	16(%rbx), %rdx
	movq	%r14, %rdi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
.LBB28_105:
	movq	(%rsp), %rax
	movq	304(%rax), %rdi
	movq	312(%rax), %rsi
.Ltmp187:
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6option6OptionINtNtCsc70TAahYccp_5alloc5boxed3BoxDINtNtNtB4_3ops8function2FnTjEEp6OutputuNtNtB4_6marker4SyncNtB2c_4SendEL_EEECs6a8jV7kq6PJ_10rayon_core
.Ltmp188:
.LBB28_108:
	movq	(%rsp), %rax
	movq	320(%rax), %rdi
	movq	328(%rax), %rsi
.Ltmp192:
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6option6OptionINtNtCsc70TAahYccp_5alloc5boxed3BoxDINtNtNtB4_3ops8function2FnTjEEp6OutputuNtNtB4_6marker4SyncNtB2c_4SendEL_EEECs6a8jV7kq6PJ_10rayon_core
.Ltmp193:
.LBB28_111:
	movq	%r15, %rdi
	callq	_Unwind_Resume@PLT
.LBB28_113:
.Ltmp194:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end28:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryEBF_, .Lfunc_end28-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryEBF_
	.cfi_endproc
	.section	.gcc_except_table._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryEBF_,"a",@progbits
	.p2align	2, 0x0
GCC_except_table28:
.Lexception12:
	.byte	255
	.byte	155
	.uleb128 .Lttbase7-.Lttbaseref7
.Lttbaseref7:
	.byte	1
	.uleb128 .Lcst_end12-.Lcst_begin12
.Lcst_begin12:
	.uleb128 .Lfunc_begin28-.Lfunc_begin28
	.uleb128 .Ltmp184-.Lfunc_begin28
	.byte	0
	.byte	0
	.uleb128 .Ltmp184-.Lfunc_begin28
	.uleb128 .Ltmp185-.Ltmp184
	.uleb128 .Ltmp186-.Lfunc_begin28
	.byte	0
	.uleb128 .Ltmp185-.Lfunc_begin28
	.uleb128 .Ltmp189-.Ltmp185
	.byte	0
	.byte	0
	.uleb128 .Ltmp189-.Lfunc_begin28
	.uleb128 .Ltmp190-.Ltmp189
	.uleb128 .Ltmp191-.Lfunc_begin28
	.byte	0
	.uleb128 .Ltmp190-.Lfunc_begin28
	.uleb128 .Ltmp195-.Ltmp190
	.byte	0
	.byte	0
	.uleb128 .Ltmp195-.Lfunc_begin28
	.uleb128 .Ltmp196-.Ltmp195
	.uleb128 .Ltmp197-.Lfunc_begin28
	.byte	0
	.uleb128 .Ltmp196-.Lfunc_begin28
	.uleb128 .Ltmp187-.Ltmp196
	.byte	0
	.byte	0
	.uleb128 .Ltmp187-.Lfunc_begin28
	.uleb128 .Ltmp193-.Ltmp187
	.uleb128 .Ltmp194-.Lfunc_begin28
	.byte	1
	.uleb128 .Ltmp193-.Lfunc_begin28
	.uleb128 .Lfunc_end28-.Ltmp193
	.byte	0
	.byte	0
.Lcst_end12:
	.byte	127
	.byte	0
	.p2align	2, 0x0
.Lttbase7:
	.byte	0
	.p2align	2, 0x0

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtNtCs7jcFBdfocI9_3std6thread9lifecycle10ThreadInitECs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.prefalign	4, .Lfunc_end29, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtNtCs7jcFBdfocI9_3std6thread9lifecycle10ThreadInitECs6a8jV7kq6PJ_10rayon_core,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtNtCs7jcFBdfocI9_3std6thread9lifecycle10ThreadInitECs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin29:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception13
	pushq	%r15
	.cfi_def_cfa_offset 16
	pushq	%r14
	.cfi_def_cfa_offset 24
	pushq	%rbx
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -32
	.cfi_offset %r14, -24
	.cfi_offset %r15, -16
	movq	(%rdi), %rax
	movq	%rdi, %r15
	lock		decq	(%rax)
	jne	.LBB29_2
	#MEMBARRIER
.Ltmp198:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtNtCs7jcFBdfocI9_3std6thread6thread5InnerNtNtBM_5alloc6SystemE9drop_slowBM_@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.Ltmp199:
.LBB29_2:
	movq	8(%r15), %rbx
	movq	16(%r15), %r15
	movq	(%r15), %rax
	testq	%rax, %rax
	je	.LBB29_4
.Ltmp204:
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp205:
.LBB29_4:
	movq	8(%r15), %rcx
	testq	%rcx, %rcx
	je	.LBB29_18
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	xorl	%esi, %esi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB29_7
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB29_7:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB29_13
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB29_7
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB29_10:
	cmpq	%rax, %rsi
	jge	.LBB29_12
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB29_10
.LBB29_12:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB29_13:
	movq	%rbx, %rdi
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	jmpq	*free@GOTPCREL(%rip)
.LBB29_18:
	.cfi_def_cfa_offset 32
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	retq
.LBB29_14:
	.cfi_def_cfa_offset 32
.Ltmp200:
	movq	8(%r15), %rdi
	movq	16(%r15), %rsi
	movq	%rax, %r14
.Ltmp201:
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc5boxed3BoxDINtNtNtB4_3ops8function6FnOnceuEp6OutputuNtNtB4_6marker4SendEL_EECs6a8jV7kq6PJ_10rayon_core
.Ltmp202:
	jmp	.LBB29_17
.LBB29_19:
.Ltmp203:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB29_15:
.Ltmp206:
	movq	8(%r15), %rsi
	movq	%rax, %r14
	testq	%rsi, %rsi
	je	.LBB29_17
	movq	16(%r15), %rdx
	movq	%rbx, %rdi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
.LBB29_17:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end29:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtNtCs7jcFBdfocI9_3std6thread9lifecycle10ThreadInitECs6a8jV7kq6PJ_10rayon_core, .Lfunc_end29-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtNtCs7jcFBdfocI9_3std6thread9lifecycle10ThreadInitECs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc
	.section	.gcc_except_table._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtNtCs7jcFBdfocI9_3std6thread9lifecycle10ThreadInitECs6a8jV7kq6PJ_10rayon_core,"a",@progbits
	.p2align	2, 0x0
GCC_except_table29:
.Lexception13:
	.byte	255
	.byte	155
	.uleb128 .Lttbase8-.Lttbaseref8
.Lttbaseref8:
	.byte	1
	.uleb128 .Lcst_end13-.Lcst_begin13
.Lcst_begin13:
	.uleb128 .Ltmp198-.Lfunc_begin29
	.uleb128 .Ltmp199-.Ltmp198
	.uleb128 .Ltmp200-.Lfunc_begin29
	.byte	0
	.uleb128 .Ltmp204-.Lfunc_begin29
	.uleb128 .Ltmp205-.Ltmp204
	.uleb128 .Ltmp206-.Lfunc_begin29
	.byte	0
	.uleb128 .Ltmp201-.Lfunc_begin29
	.uleb128 .Ltmp202-.Ltmp201
	.uleb128 .Ltmp203-.Lfunc_begin29
	.byte	1
	.uleb128 .Ltmp202-.Lfunc_begin29
	.uleb128 .Lfunc_end29-.Ltmp202
	.byte	0
	.byte	0
.Lcst_end13:
	.byte	127
	.byte	0
	.p2align	2, 0x0
.Lttbase8:
	.byte	0
	.p2align	2, 0x0

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtNtCs7jcFBdfocI9_3std6thread9spawnhook15ChildSpawnHooksECs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.prefalign	4, .Lfunc_end30, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtNtCs7jcFBdfocI9_3std6thread9spawnhook15ChildSpawnHooksECs6a8jV7kq6PJ_10rayon_core,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtNtCs7jcFBdfocI9_3std6thread9spawnhook15ChildSpawnHooksECs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin30:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception14
	pushq	%r15
	.cfi_def_cfa_offset 16
	pushq	%r14
	.cfi_def_cfa_offset 24
	pushq	%rbx
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -32
	.cfi_offset %r14, -24
	.cfi_offset %r15, -16
	leaq	24(%rdi), %r15
	movq	%rdi, %rbx
.Ltmp207:
	movq	_RNvXNtNtCs7jcFBdfocI9_3std6thread9spawnhookNtB2_10SpawnHooksNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.Ltmp208:
	movq	(%r15), %rax
	testq	%rax, %rax
	je	.LBB30_4
	lock		decq	(%rax)
	jne	.LBB30_4
	#MEMBARRIER
.Ltmp213:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtNtCs7jcFBdfocI9_3std6thread9spawnhook9SpawnHookE9drop_slowBM_@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.Ltmp214:
.LBB30_4:
	movq	%rbx, %rdi
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	jmp	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc3vec3VecINtNtBG_5boxed3BoxDINtNtNtB4_3ops8function6FnOnceuEp6OutputuNtNtB4_6marker4SendEL_EEECs6a8jV7kq6PJ_10rayon_core
.LBB30_9:
	.cfi_def_cfa_offset 32
.Ltmp215:
	movq	%rax, %r14
	jmp	.LBB30_10
.LBB30_5:
.Ltmp209:
	movq	%rax, %r14
	movq	(%r15), %rax
	testq	%rax, %rax
	je	.LBB30_10
	lock		decq	(%rax)
	jne	.LBB30_10
	#MEMBARRIER
.Ltmp210:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtNtCs7jcFBdfocI9_3std6thread9spawnhook9SpawnHookE9drop_slowBM_@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.Ltmp211:
.LBB30_10:
.Ltmp216:
	movq	%rbx, %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc3vec3VecINtNtBG_5boxed3BoxDINtNtNtB4_3ops8function6FnOnceuEp6OutputuNtNtB4_6marker4SendEL_EEECs6a8jV7kq6PJ_10rayon_core
.Ltmp217:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB30_8:
.Ltmp212:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB30_12:
.Ltmp218:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end30:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtNtCs7jcFBdfocI9_3std6thread9spawnhook15ChildSpawnHooksECs6a8jV7kq6PJ_10rayon_core, .Lfunc_end30-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtNtCs7jcFBdfocI9_3std6thread9spawnhook15ChildSpawnHooksECs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc
	.section	.gcc_except_table._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtNtCs7jcFBdfocI9_3std6thread9spawnhook15ChildSpawnHooksECs6a8jV7kq6PJ_10rayon_core,"a",@progbits
	.p2align	2, 0x0
GCC_except_table30:
.Lexception14:
	.byte	255
	.byte	155
	.uleb128 .Lttbase9-.Lttbaseref9
.Lttbaseref9:
	.byte	1
	.uleb128 .Lcst_end14-.Lcst_begin14
.Lcst_begin14:
	.uleb128 .Ltmp207-.Lfunc_begin30
	.uleb128 .Ltmp208-.Ltmp207
	.uleb128 .Ltmp209-.Lfunc_begin30
	.byte	0
	.uleb128 .Ltmp213-.Lfunc_begin30
	.uleb128 .Ltmp214-.Ltmp213
	.uleb128 .Ltmp215-.Lfunc_begin30
	.byte	0
	.uleb128 .Ltmp214-.Lfunc_begin30
	.uleb128 .Ltmp210-.Ltmp214
	.byte	0
	.byte	0
	.uleb128 .Ltmp210-.Lfunc_begin30
	.uleb128 .Ltmp211-.Ltmp210
	.uleb128 .Ltmp212-.Lfunc_begin30
	.byte	1
	.uleb128 .Ltmp216-.Lfunc_begin30
	.uleb128 .Ltmp217-.Ltmp216
	.uleb128 .Ltmp218-.Lfunc_begin30
	.byte	1
	.uleb128 .Ltmp217-.Lfunc_begin30
	.uleb128 .Lfunc_end30-.Ltmp217
	.byte	0
	.byte	0
.Lcst_end14:
	.byte	127
	.byte	0
	.p2align	2, 0x0
.Lttbase9:
	.byte	0
	.p2align	2, 0x0

	.section	.text._RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueTINtNtCsc70TAahYccp_5alloc3vec3VecINtNtCs64OF0TycdZY_15crossbeam_deque5deque6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEIBD_INtB1c_7StealerB1W_EEEEB20_,"ax",@progbits
	.prefalign	4, .Lfunc_end31, nop
	.type	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueTINtNtCsc70TAahYccp_5alloc3vec3VecINtNtCs64OF0TycdZY_15crossbeam_deque5deque6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEIBD_INtB1c_7StealerB1W_EEEEB20_,@function
_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueTINtNtCsc70TAahYccp_5alloc3vec3VecINtNtCs64OF0TycdZY_15crossbeam_deque5deque6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEIBD_INtB1c_7StealerB1W_EEEEB20_:
.Lfunc_begin31:
	.cfi_startproc
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
	pushq	%rax
	.cfi_def_cfa_offset 64
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	8(%rdi), %r14
	movq	16(%rdi), %r12
	movq	%rdi, %rbx
	testq	%r12, %r12
	je	.LBB31_5
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %r13
	movq	%r14, %r15
	jmp	.LBB31_2
	.p2align	4
.LBB31_4:
	addq	$32, %r15
	decq	%r12
	je	.LBB31_5
.LBB31_2:
	movq	(%r15), %rax
	lock		decq	(%rax)
	jne	.LBB31_4
	movq	%r15, %rdi
	#MEMBARRIER
	callq	*%r13
	jmp	.LBB31_4
.LBB31_5:
	movq	(%rbx), %rcx
	movabsq	$9223372036854775807, %r12
	testq	%rcx, %rcx
	je	.LBB31_15
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	shlq	$5, %rcx
	cmpq	%r12, %rcx
	cmovaeq	%r12, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%r12, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB31_8
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB31_8:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB31_14
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB31_8
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r12, %rax
	subq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB31_11:
	cmpq	%rax, %rdx
	jge	.LBB31_13
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB31_11
.LBB31_13:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB31_14:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
.LBB31_15:
	movq	32(%rbx), %r14
	movq	40(%rbx), %r13
	testq	%r13, %r13
	je	.LBB31_20
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %rbp
	movq	%r14, %r15
	jmp	.LBB31_17
	.p2align	4
.LBB31_19:
	addq	$16, %r15
	decq	%r13
	je	.LBB31_20
.LBB31_17:
	movq	(%r15), %rax
	lock		decq	(%rax)
	jne	.LBB31_19
	movq	%r15, %rdi
	#MEMBARRIER
	callq	*%rbp
	jmp	.LBB31_19
.LBB31_20:
	movq	24(%rbx), %rcx
	testq	%rcx, %rcx
	je	.LBB31_30
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	shlq	$4, %rcx
	cmpq	%r12, %rcx
	cmovaeq	%r12, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%r12, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB31_23
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB31_23:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB31_29
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB31_23
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r12, %rax
	subq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB31_26:
	cmpq	%rax, %rdx
	jge	.LBB31_28
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB31_26
.LBB31_28:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB31_29:
	movq	%r14, %rdi
	addq	$8, %rsp
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
	jmpq	*free@GOTPCREL(%rip)
.LBB31_30:
	.cfi_def_cfa_offset 64
	addq	$8, %rsp
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
.Lfunc_end31:
	.size	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueTINtNtCsc70TAahYccp_5alloc3vec3VecINtNtCs64OF0TycdZY_15crossbeam_deque5deque6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEIBD_INtB1c_7StealerB1W_EEEEB20_, .Lfunc_end31-_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueTINtNtCsc70TAahYccp_5alloc3vec3VecINtNtCs64OF0TycdZY_15crossbeam_deque5deque6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEIBD_INtB1c_7StealerB1W_EEEEB20_
	.cfi_endproc

	.section	.text._RINvNtNtCs7jcFBdfocI9_3std3sys9backtrace28___rust_begin_short_backtraceNCNCNCINvNtNtB6_6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB23_12DefaultSpawnNtB23_11ThreadSpawn5spawn0uEs_000uEB25_,"ax",@progbits
	.globl	_RINvNtNtCs7jcFBdfocI9_3std3sys9backtrace28___rust_begin_short_backtraceNCNCNCINvNtNtB6_6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB23_12DefaultSpawnNtB23_11ThreadSpawn5spawn0uEs_000uEB25_
	.prefalign	4, .Lfunc_end32, nop
	.type	_RINvNtNtCs7jcFBdfocI9_3std3sys9backtrace28___rust_begin_short_backtraceNCNCNCINvNtNtB6_6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB23_12DefaultSpawnNtB23_11ThreadSpawn5spawn0uEs_000uEB25_,@function
_RINvNtNtCs7jcFBdfocI9_3std3sys9backtrace28___rust_begin_short_backtraceNCNCNCINvNtNtB6_6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB23_12DefaultSpawnNtB23_11ThreadSpawn5spawn0uEs_000uEB25_:
.Lfunc_begin32:
	.cfi_startproc
	subq	$40, %rsp
	.cfi_def_cfa_offset 48
	vmovups	(%rdi), %ymm0
	movq	_RNvMs_NtNtCs7jcFBdfocI9_3std6thread9spawnhookNtB4_15ChildSpawnHooks15inherit_and_run@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	vmovups	%ymm0, (%rsp)
	vzeroupper
	callq	*%rax
	#APP
	#NO_APP
	addq	$40, %rsp
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end32:
	.size	_RINvNtNtCs7jcFBdfocI9_3std3sys9backtrace28___rust_begin_short_backtraceNCNCNCINvNtNtB6_6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB23_12DefaultSpawnNtB23_11ThreadSpawn5spawn0uEs_000uEB25_, .Lfunc_end32-_RINvNtNtCs7jcFBdfocI9_3std3sys9backtrace28___rust_begin_short_backtraceNCNCNCINvNtNtB6_6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB23_12DefaultSpawnNtB23_11ThreadSpawn5spawn0uEs_000uEB25_
	.cfi_endproc

	.section	.text._RINvNtNtCs7jcFBdfocI9_3std3sys9backtrace28___rust_begin_short_backtraceNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1f_12DefaultSpawnNtB1f_11ThreadSpawn5spawn0uEB1h_,"ax",@progbits
	.globl	_RINvNtNtCs7jcFBdfocI9_3std3sys9backtrace28___rust_begin_short_backtraceNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1f_12DefaultSpawnNtB1f_11ThreadSpawn5spawn0uEB1h_
	.prefalign	4, .Lfunc_end33, nop
	.type	_RINvNtNtCs7jcFBdfocI9_3std3sys9backtrace28___rust_begin_short_backtraceNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1f_12DefaultSpawnNtB1f_11ThreadSpawn5spawn0uEB1h_,@function
_RINvNtNtCs7jcFBdfocI9_3std3sys9backtrace28___rust_begin_short_backtraceNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1f_12DefaultSpawnNtB1f_11ThreadSpawn5spawn0uEB1h_:
.Lfunc_begin33:
	.cfi_startproc
	pushq	%rax
	.cfi_def_cfa_offset 16
	movq	_RNvMNtCs6a8jV7kq6PJ_10rayon_core8registryNtB2_13ThreadBuilder3run@GOTPCREL(%rip), %rax
	callq	*%rax
	#APP
	#NO_APP
	popq	%rax
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end33:
	.size	_RINvNtNtCs7jcFBdfocI9_3std3sys9backtrace28___rust_begin_short_backtraceNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1f_12DefaultSpawnNtB1f_11ThreadSpawn5spawn0uEB1h_, .Lfunc_end33-_RINvNtNtCs7jcFBdfocI9_3std3sys9backtrace28___rust_begin_short_backtraceNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1f_12DefaultSpawnNtB1f_11ThreadSpawn5spawn0uEB1h_
	.cfi_endproc

	.section	.text._RINvNtNtNtNtCs7jcFBdfocI9_3std3sys12thread_local6native4lazy7destroyNtNtCs18aJq3QiqAb_15crossbeam_epoch9collector11LocalHandleECs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.prefalign	4, .Lfunc_end34, nop
	.type	_RINvNtNtNtNtCs7jcFBdfocI9_3std3sys12thread_local6native4lazy7destroyNtNtCs18aJq3QiqAb_15crossbeam_epoch9collector11LocalHandleECs6a8jV7kq6PJ_10rayon_core,@function
_RINvNtNtNtNtCs7jcFBdfocI9_3std3sys12thread_local6native4lazy7destroyNtNtCs18aJq3QiqAb_15crossbeam_epoch9collector11LocalHandleECs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin34:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception15
	pushq	%rax
	.cfi_def_cfa_offset 16
	movzbl	8(%rdi), %eax
	movb	$2, 8(%rdi)
	cmpb	$1, %al
	jne	.LBB34_3
	movq	(%rdi), %rdi
	movq	2080(%rdi), %rax
	leaq	-1(%rax), %rcx
	xorq	$1, %rax
	movq	%rcx, 2080(%rdi)
	orq	2072(%rdi), %rax
	je	.LBB34_2
.LBB34_3:
	popq	%rax
	.cfi_def_cfa_offset 8
	retq
.LBB34_2:
	.cfi_def_cfa_offset 16
.Ltmp219:
	movq	_RNvMs6_NtCs18aJq3QiqAb_15crossbeam_epoch8internalNtB5_5Local8finalize@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp220:
	jmp	.LBB34_3
.LBB34_4:
.Ltmp221:
.Ltmp222:
	callq	_RNvXNvNtNtCs7jcFBdfocI9_3std3sys12thread_local20abort_on_dtor_unwindNtB2_15DtorUnwindGuardNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop
.Ltmp223:
	ud2
.LBB34_6:
.Ltmp224:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end34:
	.size	_RINvNtNtNtNtCs7jcFBdfocI9_3std3sys12thread_local6native4lazy7destroyNtNtCs18aJq3QiqAb_15crossbeam_epoch9collector11LocalHandleECs6a8jV7kq6PJ_10rayon_core, .Lfunc_end34-_RINvNtNtNtNtCs7jcFBdfocI9_3std3sys12thread_local6native4lazy7destroyNtNtCs18aJq3QiqAb_15crossbeam_epoch9collector11LocalHandleECs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc
	.section	.gcc_except_table._RINvNtNtNtNtCs7jcFBdfocI9_3std3sys12thread_local6native4lazy7destroyNtNtCs18aJq3QiqAb_15crossbeam_epoch9collector11LocalHandleECs6a8jV7kq6PJ_10rayon_core,"a",@progbits
	.p2align	2, 0x0
GCC_except_table34:
.Lexception15:
	.byte	255
	.byte	155
	.uleb128 .Lttbase10-.Lttbaseref10
.Lttbaseref10:
	.byte	1
	.uleb128 .Lcst_end15-.Lcst_begin15
.Lcst_begin15:
	.uleb128 .Ltmp219-.Lfunc_begin34
	.uleb128 .Ltmp220-.Ltmp219
	.uleb128 .Ltmp221-.Lfunc_begin34
	.byte	1
	.uleb128 .Ltmp222-.Lfunc_begin34
	.uleb128 .Ltmp223-.Ltmp222
	.uleb128 .Ltmp224-.Lfunc_begin34
	.byte	1
	.uleb128 .Ltmp223-.Lfunc_begin34
	.uleb128 .Lfunc_end34-.Ltmp223
	.byte	0
	.byte	0
.Lcst_end15:
	.byte	127
	.byte	0
	.p2align	2, 0x0
.Lttbase10:
	.byte	0
	.p2align	2, 0x0

	.section	.text.unlikely._RINvNvMs2_NtCsc70TAahYccp_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.prefalign	4, .Lfunc_end35, nop
	.type	_RINvNvMs2_NtCsc70TAahYccp_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs6a8jV7kq6PJ_10rayon_core,@function
_RINvNvMs2_NtCsc70TAahYccp_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin35:
	.cfi_startproc
	pushq	%r14
	.cfi_def_cfa_offset 16
	pushq	%rbx
	.cfi_def_cfa_offset 24
	subq	$24, %rsp
	.cfi_def_cfa_offset 48
	.cfi_offset %rbx, -24
	.cfi_offset %r14, -16
	addq	%rdx, %rsi
	jb	.LBB35_1
	movq	(%rdi), %rax
	movq	%rcx, %r8
	movq	8(%rdi), %rdx
	movl	$4, %r14d
	movq	%rdi, %rbx
	movq	%rsp, %rdi
	leaq	(%rax,%rax), %rcx
	cmpq	%rsi, %rcx
	cmovaq	%rcx, %rsi
	cmpq	$5, %rsi
	cmovaeq	%rsi, %r14
	movq	%rax, %rsi
	movq	%r14, %rcx
	callq	_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs6a8jV7kq6PJ_10rayon_core
	cmpl	$1, (%rsp)
	je	.LBB35_3
	movq	8(%rsp), %rax
	movq	%rax, 8(%rbx)
	movq	%r14, (%rbx)
	addq	$24, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	retq
.LBB35_1:
	.cfi_def_cfa_offset 48
	movq	_RNvNtCsc70TAahYccp_5alloc7raw_vec12handle_error@GOTPCREL(%rip), %rax
	xorl	%edi, %edi
	callq	*%rax
.LBB35_3:
	movq	8(%rsp), %rdi
	movq	16(%rsp), %rsi
	movq	_RNvNtCsc70TAahYccp_5alloc7raw_vec12handle_error@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end35:
	.size	_RINvNvMs2_NtCsc70TAahYccp_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs6a8jV7kq6PJ_10rayon_core, .Lfunc_end35-_RINvNvMs2_NtCsc70TAahYccp_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc

	.section	.text._RINvNvMs_NtCs18aJq3QiqAb_15crossbeam_epoch8deferredNtB7_8Deferred3new4callNCINvMNtB9_5guardNtB1g_5Guard15defer_uncheckedNCNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB22_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE6resize0uE0EB2X_,"ax",@progbits
	.prefalign	4, .Lfunc_end36, nop
	.type	_RINvNvMs_NtCs18aJq3QiqAb_15crossbeam_epoch8deferredNtB7_8Deferred3new4callNCINvMNtB9_5guardNtB1g_5Guard15defer_uncheckedNCNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB22_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE6resize0uE0EB2X_,@function
_RINvNvMs_NtCs18aJq3QiqAb_15crossbeam_epoch8deferredNtB7_8Deferred3new4callNCINvMNtB9_5guardNtB1g_5Guard15defer_uncheckedNCNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB22_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE6resize0uE0EB2X_:
.Lfunc_begin36:
	.cfi_startproc
	pushq	%rbx
	.cfi_def_cfa_offset 16
	.cfi_offset %rbx, -16
	movq	(%rdi), %rbx
	andq	$-8, %rbx
	movq	8(%rbx), %rcx
	testq	%rcx, %rcx
	je	.LBB36_10
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	shlq	$4, %rcx
	movq	(%rbx), %rdi
	xorl	%esi, %esi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB36_3
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB36_3:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB36_9
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB36_3
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB36_6:
	cmpq	%rax, %rsi
	jge	.LBB36_8
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB36_6
.LBB36_8:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB36_9:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB36_10:
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movabsq	$-9223372036854775808, %rcx
	addq	$-16, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB36_12
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB36_12:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB36_18
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB36_12
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	$-16, %rdx
	lock		xaddq	%rdx, (%rax)
	addq	$-16, %rdx
	cmovoq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
	.p2align	4
.LBB36_15:
	cmpq	%rax, %rdx
	jge	.LBB36_17
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB36_15
.LBB36_17:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB36_18:
	movq	%rbx, %rdi
	popq	%rbx
	.cfi_def_cfa_offset 8
	jmpq	*free@GOTPCREL(%rip)
.Lfunc_end36:
	.size	_RINvNvMs_NtCs18aJq3QiqAb_15crossbeam_epoch8deferredNtB7_8Deferred3new4callNCINvMNtB9_5guardNtB1g_5Guard15defer_uncheckedNCNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB22_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE6resize0uE0EB2X_, .Lfunc_end36-_RINvNvMs_NtCs18aJq3QiqAb_15crossbeam_epoch8deferredNtB7_8Deferred3new4callNCINvMNtB9_5guardNtB1g_5Guard15defer_uncheckedNCNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB22_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE6resize0uE0EB2X_
	.cfi_endproc

	.section	.text._RNCINvMs0_NtNtCs7jcFBdfocI9_3std4sync4onceNtB8_4Once9call_onceNCINvNtCs6a8jV7kq6PJ_10rayon_core8registry19set_global_registryNvB13_23default_global_registryE0E0B15_.llvm.7294274987384275483,"ax",@progbits
	.hidden	_RNCINvMs0_NtNtCs7jcFBdfocI9_3std4sync4onceNtB8_4Once9call_onceNCINvNtCs6a8jV7kq6PJ_10rayon_core8registry19set_global_registryNvB13_23default_global_registryE0E0B15_.llvm.7294274987384275483
	.globl	_RNCINvMs0_NtNtCs7jcFBdfocI9_3std4sync4onceNtB8_4Once9call_onceNCINvNtCs6a8jV7kq6PJ_10rayon_core8registry19set_global_registryNvB13_23default_global_registryE0E0B15_.llvm.7294274987384275483
	.prefalign	4, .Lfunc_end37, nop
	.type	_RNCINvMs0_NtNtCs7jcFBdfocI9_3std4sync4onceNtB8_4Once9call_onceNCINvNtCs6a8jV7kq6PJ_10rayon_core8registry19set_global_registryNvB13_23default_global_registryE0E0B15_.llvm.7294274987384275483,@function
_RNCINvMs0_NtNtCs7jcFBdfocI9_3std4sync4onceNtB8_4Once9call_onceNCINvNtCs6a8jV7kq6PJ_10rayon_core8registry19set_global_registryNvB13_23default_global_registryE0E0B15_.llvm.7294274987384275483:
.Lfunc_begin37:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception16
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
	subq	$208, %rsp
	.cfi_def_cfa_offset 256
	.cfi_offset %rbx, -48
	.cfi_offset %r12, -40
	.cfi_offset %r13, -32
	.cfi_offset %r14, -24
	.cfi_offset %r15, -16
	movq	(%rdi), %rax
	movq	(%rax), %r15
	movq	$0, (%rax)
	testq	%r15, %r15
	je	.LBB37_30
	movw	$0, 104(%rsp)
	movq	$0, 56(%rsp)
	movq	$0, 16(%rsp)
	movq	$0, 72(%rsp)
	movq	$0, 88(%rsp)
	vxorps	%xmm0, %xmm0, %xmm0
	vmovups	%xmm0, 32(%rsp)
	leaq	112(%rsp), %rsi
	movq	%rsp, %rdi
	vmovups	48(%rsp), %zmm0
	vmovups	16(%rsp), %zmm1
	vmovups	%zmm0, 144(%rsp)
	vmovups	%zmm1, 112(%rsp)
	vzeroupper
	callq	_RINvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB6_8Registry3newNtB6_12DefaultSpawnEB8_
	movq	(%rsp), %r12
	movq	8(%rsp), %rbx
	cmpq	$-1, %r12
	je	.LBB37_2
	cmpl	$2, %r12d
	jne	.LBB37_22
	movl	%ebx, %r13d
	andl	$3, %r13d
	leaq	.LJTI37_0(%rip), %rax
	movslq	(%rax,%r13,4), %rcx
	addq	%rax, %rcx
	jmpq	*%rcx
.LBB37_6:
	movzbl	16(%rbx), %eax
	cmpb	$36, %al
	je	.LBB37_11
	jmp	.LBB37_22
.LBB37_2:
	movq	%rbx, %r14
.LBB37_21:
	movq	_RNvNtCs6a8jV7kq6PJ_10rayon_core8registry12THE_REGISTRY@GOTPCREL(%rip), %rbx
	movq	$-1, %r12
	movq	%r14, (%rbx)
.LBB37_22:
	cmpl	$2, (%r15)
	jne	.LBB37_26
	movq	8(%r15), %rdi
	movl	%edi, %eax
	andl	$3, %eax
	leal	-2(%rax), %ecx
	cmpl	$2, %ecx
	jb	.LBB37_26
	testq	%rax, %rax
	je	.LBB37_26
	movq	23(%rdi), %rax
	decq	%rdi
.Ltmp235:
	callq	*%rax
.Ltmp236:
.LBB37_26:
	movq	%r12, (%r15)
	movq	%rbx, 8(%r15)
	addq	$208, %rsp
	.cfi_def_cfa_offset 48
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
.LBB37_5:
	.cfi_def_cfa_offset 256
	movq	_RNvNtNtNtCs2k2z8Zem4rB_4core2io5error12os_functions12OS_FUNCTIONS@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	shrq	$32, %rdi
	movq	(%rax), %rax
	movq	8(%rax), %rax
.Ltmp225:
	callq	*%rax
.Ltmp226:
	jmp	.LBB37_10
.LBB37_9:
	movq	%rbx, %rax
	shrq	$32, %rax
.LBB37_10:
	cmpb	$36, %al
	je	.LBB37_11
	jmp	.LBB37_22
.LBB37_7:
	movzbl	31(%rbx), %eax
	cmpb	$36, %al
	jne	.LBB37_22
.LBB37_11:
	movq	_RNvNCNKNvNtCs6a8jV7kq6PJ_10rayon_core8registry19WORKER_THREAD_STATE0s_023___RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rax
	cmpq	$0, %fs:(%rax)
	jne	.LBB37_22
	movq	$1, 32(%rsp)
	movb	$1, 104(%rsp)
.Ltmp227:
	leaq	112(%rsp), %rdi
	leaq	16(%rsp), %rsi
	callq	_RINvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB6_8Registry3newNtB6_12DefaultSpawnEB8_
.Ltmp228:
	movq	112(%rsp), %rax
	movq	120(%rsp), %r14
	cmpq	$-1, %rax
	je	.LBB37_18
	cmpl	$2, %eax
	jb	.LBB37_22
	movl	%r14d, %eax
	andl	$3, %eax
	leal	-2(%rax), %ecx
	cmpl	$2, %ecx
	jb	.LBB37_22
	testq	%rax, %rax
	je	.LBB37_22
	movq	23(%r14), %rax
	decq	%r14
.Ltmp229:
	movq	%r14, %rdi
	callq	*%rax
.Ltmp230:
	jmp	.LBB37_22
.LBB37_18:
	leal	-2(%r13), %eax
	cmpl	$2, %eax
	jb	.LBB37_21
	testq	%r13, %r13
	je	.LBB37_21
	movq	23(%rbx), %rax
	decq	%rbx
	movq	%rbx, %rdi
	callq	*%rax
	jmp	.LBB37_21
.LBB37_30:
	movq	_RNvNtCs2k2z8Zem4rB_4core6option13unwrap_failed@GOTPCREL(%rip), %rax
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.16(%rip), %rdi
	callq	*%rax
.LBB37_28:
.Ltmp237:
	movq	%rax, %r14
	movq	%r12, (%r15)
	movq	%rbx, 8(%r15)
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB37_8:
.Ltmp231:
	movq	%rax, %r14
.Ltmp232:
	movq	%rsp, %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultINtNtCsc70TAahYccp_5alloc4sync3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryENtB1z_20ThreadPoolBuildErrorEEB1z_
.Ltmp233:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB37_27:
.Ltmp234:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end37:
	.size	_RNCINvMs0_NtNtCs7jcFBdfocI9_3std4sync4onceNtB8_4Once9call_onceNCINvNtCs6a8jV7kq6PJ_10rayon_core8registry19set_global_registryNvB13_23default_global_registryE0E0B15_.llvm.7294274987384275483, .Lfunc_end37-_RNCINvMs0_NtNtCs7jcFBdfocI9_3std4sync4onceNtB8_4Once9call_onceNCINvNtCs6a8jV7kq6PJ_10rayon_core8registry19set_global_registryNvB13_23default_global_registryE0E0B15_.llvm.7294274987384275483
	.cfi_endproc
	.section	.rodata._RNCINvMs0_NtNtCs7jcFBdfocI9_3std4sync4onceNtB8_4Once9call_onceNCINvNtCs6a8jV7kq6PJ_10rayon_core8registry19set_global_registryNvB13_23default_global_registryE0E0B15_.llvm.7294274987384275483,"a",@progbits
	.p2align	2, 0x0
.LJTI37_0:
	.long	.LBB37_6-.LJTI37_0
	.long	.LBB37_7-.LJTI37_0
	.long	.LBB37_5-.LJTI37_0
	.long	.LBB37_9-.LJTI37_0
	.section	.gcc_except_table._RNCINvMs0_NtNtCs7jcFBdfocI9_3std4sync4onceNtB8_4Once9call_onceNCINvNtCs6a8jV7kq6PJ_10rayon_core8registry19set_global_registryNvB13_23default_global_registryE0E0B15_.llvm.7294274987384275483,"a",@progbits
	.p2align	2, 0x0
GCC_except_table37:
.Lexception16:
	.byte	255
	.byte	155
	.uleb128 .Lttbase11-.Lttbaseref11
.Lttbaseref11:
	.byte	1
	.uleb128 .Lcst_end16-.Lcst_begin16
.Lcst_begin16:
	.uleb128 .Lfunc_begin37-.Lfunc_begin37
	.uleb128 .Ltmp235-.Lfunc_begin37
	.byte	0
	.byte	0
	.uleb128 .Ltmp235-.Lfunc_begin37
	.uleb128 .Ltmp236-.Ltmp235
	.uleb128 .Ltmp237-.Lfunc_begin37
	.byte	0
	.uleb128 .Ltmp225-.Lfunc_begin37
	.uleb128 .Ltmp230-.Ltmp225
	.uleb128 .Ltmp231-.Lfunc_begin37
	.byte	0
	.uleb128 .Ltmp230-.Lfunc_begin37
	.uleb128 .Ltmp232-.Ltmp230
	.byte	0
	.byte	0
	.uleb128 .Ltmp232-.Lfunc_begin37
	.uleb128 .Ltmp233-.Ltmp232
	.uleb128 .Ltmp234-.Lfunc_begin37
	.byte	1
	.uleb128 .Ltmp233-.Lfunc_begin37
	.uleb128 .Lfunc_end37-.Ltmp233
	.byte	0
	.byte	0
.Lcst_end16:
	.byte	127
	.byte	0
	.p2align	2, 0x0
.Lttbase11:
	.byte	0
	.p2align	2, 0x0

	.section	.text._RNSNvYNCINvMs0_NtNtCs7jcFBdfocI9_3std4sync4onceNtBd_4Once9call_onceNCINvNtCs6a8jV7kq6PJ_10rayon_core8registry19set_global_registryNvB18_23default_global_registryE0E0INtNtNtCs2k2z8Zem4rB_4core3ops8function6FnOnceTRNtBd_9OnceStateEE9call_once6vtableB1a_.llvm.7294274987384275483,"ax",@progbits
	.hidden	_RNSNvYNCINvMs0_NtNtCs7jcFBdfocI9_3std4sync4onceNtBd_4Once9call_onceNCINvNtCs6a8jV7kq6PJ_10rayon_core8registry19set_global_registryNvB18_23default_global_registryE0E0INtNtNtCs2k2z8Zem4rB_4core3ops8function6FnOnceTRNtBd_9OnceStateEE9call_once6vtableB1a_.llvm.7294274987384275483
	.globl	_RNSNvYNCINvMs0_NtNtCs7jcFBdfocI9_3std4sync4onceNtBd_4Once9call_onceNCINvNtCs6a8jV7kq6PJ_10rayon_core8registry19set_global_registryNvB18_23default_global_registryE0E0INtNtNtCs2k2z8Zem4rB_4core3ops8function6FnOnceTRNtBd_9OnceStateEE9call_once6vtableB1a_.llvm.7294274987384275483
	.prefalign	4, .Lfunc_end38, nop
	.type	_RNSNvYNCINvMs0_NtNtCs7jcFBdfocI9_3std4sync4onceNtBd_4Once9call_onceNCINvNtCs6a8jV7kq6PJ_10rayon_core8registry19set_global_registryNvB18_23default_global_registryE0E0INtNtNtCs2k2z8Zem4rB_4core3ops8function6FnOnceTRNtBd_9OnceStateEE9call_once6vtableB1a_.llvm.7294274987384275483,@function
_RNSNvYNCINvMs0_NtNtCs7jcFBdfocI9_3std4sync4onceNtBd_4Once9call_onceNCINvNtCs6a8jV7kq6PJ_10rayon_core8registry19set_global_registryNvB18_23default_global_registryE0E0INtNtNtCs2k2z8Zem4rB_4core3ops8function6FnOnceTRNtBd_9OnceStateEE9call_once6vtableB1a_.llvm.7294274987384275483:
.Lfunc_begin38:
	.cfi_startproc
	pushq	%rax
	.cfi_def_cfa_offset 16
	movq	(%rdi), %rax
	movq	%rsp, %rdi
	movq	%rax, (%rsp)
	callq	_RNCINvMs0_NtNtCs7jcFBdfocI9_3std4sync4onceNtB8_4Once9call_onceNCINvNtCs6a8jV7kq6PJ_10rayon_core8registry19set_global_registryNvB13_23default_global_registryE0E0B15_.llvm.7294274987384275483
	popq	%rax
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end38:
	.size	_RNSNvYNCINvMs0_NtNtCs7jcFBdfocI9_3std4sync4onceNtBd_4Once9call_onceNCINvNtCs6a8jV7kq6PJ_10rayon_core8registry19set_global_registryNvB18_23default_global_registryE0E0INtNtNtCs2k2z8Zem4rB_4core3ops8function6FnOnceTRNtBd_9OnceStateEE9call_once6vtableB1a_.llvm.7294274987384275483, .Lfunc_end38-_RNSNvYNCINvMs0_NtNtCs7jcFBdfocI9_3std4sync4onceNtBd_4Once9call_onceNCINvNtCs6a8jV7kq6PJ_10rayon_core8registry19set_global_registryNvB18_23default_global_registryE0E0INtNtNtCs2k2z8Zem4rB_4core3ops8function6FnOnceTRNtBd_9OnceStateEE9call_once6vtableB1a_.llvm.7294274987384275483
	.cfi_endproc

	.section	.text._RNSNvYNCINvNtNtCs7jcFBdfocI9_3std6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1b_12DefaultSpawnNtB1b_11ThreadSpawn5spawn0uEs_0INtNtNtCs2k2z8Zem4rB_4core3ops8function6FnOnceuE9call_once6vtableB1d_,"ax",@progbits
	.prefalign	4, .Lfunc_end39, nop
	.type	_RNSNvYNCINvNtNtCs7jcFBdfocI9_3std6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1b_12DefaultSpawnNtB1b_11ThreadSpawn5spawn0uEs_0INtNtNtCs2k2z8Zem4rB_4core3ops8function6FnOnceuE9call_once6vtableB1d_,@function
_RNSNvYNCINvNtNtCs7jcFBdfocI9_3std6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1b_12DefaultSpawnNtB1b_11ThreadSpawn5spawn0uEs_0INtNtNtCs2k2z8Zem4rB_4core3ops8function6FnOnceuE9call_once6vtableB1d_:
.Lfunc_begin39:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception17
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
	subq	$280, %rsp
	.cfi_def_cfa_offset 336
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	vmovups	104(%rdi), %ymm0
	movq	%rdi, %r15
	vmovups	%ymm0, 248(%rsp)
	vmovups	16(%rdi), %zmm0
	vmovups	40(%rdi), %zmm1
	vmovups	%zmm1, 184(%rsp)
	vmovups	%zmm0, 160(%rsp)
	vmovups	(%rdi), %xmm0
	vmovups	160(%rsp), %zmm2
	vmovups	216(%rsp), %zmm1
	leaq	120(%rsp), %rdi
	vmovaps	%xmm0, 16(%rsp)
	vmovups	%zmm2, 32(%rsp)
	vmovups	%zmm1, 88(%rsp)
.Ltmp238:
	movq	_RINvNtNtCs7jcFBdfocI9_3std3sys9backtrace28___rust_begin_short_backtraceNCNCNCINvNtNtB6_6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB23_12DefaultSpawnNtB23_11ThreadSpawn5spawn0uEs_000uEB25_@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp239:
.Ltmp244:
	movq	_RINvNtNtCs7jcFBdfocI9_3std3sys9backtrace28___rust_begin_short_backtraceNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1f_12DefaultSpawnNtB1f_11ThreadSpawn5spawn0uEB1h_@GOTPCREL(%rip), %rax
	leaq	16(%rsp), %rdi
	callq	*%rax
.Ltmp245:
	xorl	%ebx, %ebx
	movq	136(%r15), %rbp
	cmpq	$0, 24(%rbp)
	je	.LBB39_22
.LBB39_9:
	movq	32(%rbp), %r12
	testq	%r12, %r12
	je	.LBB39_22
	movq	40(%rbp), %r13
	movq	(%r13), %rax
	testq	%rax, %rax
	je	.LBB39_12
.Ltmp250:
	movq	%r12, %rdi
	addq	$136, %r15
	callq	*%rax
.Ltmp251:
.LBB39_12:
	movq	8(%r13), %rcx
	testq	%rcx, %rcx
	je	.LBB39_22
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	xorl	%esi, %esi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB39_15
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB39_15:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB39_21
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB39_15
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB39_18:
	cmpq	%rax, %rsi
	jge	.LBB39_20
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB39_18
.LBB39_20:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB39_21:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	callq	*%rax
.LBB39_22:
	movq	$1, 24(%rbp)
	movq	%rbx, 32(%rbp)
	movq	%r14, 40(%rbp)
	movq	%rbp, 8(%rsp)
	lock		decq	(%rbp)
	jne	.LBB39_24
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEE9drop_slowCs6a8jV7kq6PJ_10rayon_core@GOTPCREL(%rip), %rax
	leaq	8(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB39_24:
	addq	$280, %rsp
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
.LBB39_26:
	.cfi_def_cfa_offset 336
.Ltmp252:
	movq	8(%r13), %rsi
	movq	%rax, (%rsp)
	testq	%rsi, %rsi
	je	.LBB39_28
	movq	16(%r13), %rdx
	movq	%r12, %rdi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
.LBB39_28:
	movq	$1, 24(%rbp)
	movq	%rbx, 32(%rbp)
	movq	%r14, 40(%rbp)
	lock		decq	(%rbp)
	jne	.LBB39_30
	#MEMBARRIER
.Ltmp253:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEE9drop_slowCs6a8jV7kq6PJ_10rayon_core@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
.Ltmp254:
.LBB39_30:
	movq	(%rsp), %rdi
	callq	_Unwind_Resume@PLT
.LBB39_31:
.Ltmp255:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB39_5:
.Ltmp246:
	movq	%rax, %rbx
	jmp	.LBB39_6
.LBB39_3:
.Ltmp240:
	movq	%rax, %rbx
.Ltmp241:
	leaq	16(%rsp), %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtBJ_12DefaultSpawnNtBJ_11ThreadSpawn5spawn0EBL_
.Ltmp242:
.LBB39_6:
.Ltmp247:
	movq	_RNvCs2NWS7XDLE6y_7___rustc20___rust_panic_cleanup@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp248:
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count18GLOBAL_PANIC_COUNT@GOTPCREL(%rip), %rcx
	movq	%rax, %rbx
	movq	%rdx, %r14
	lock		decq	(%rcx)
	decq	%fs:_RNvNCNKNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count17LOCAL_PANIC_COUNT0s_023___RUST_STD_INTERNAL_VAL.llvm.11640361436736466388@TPOFF
	movb	$0, %fs:_RNvNCNKNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count17LOCAL_PANIC_COUNT0s_023___RUST_STD_INTERNAL_VAL.llvm.11640361436736466388@TPOFF+8
	movq	136(%r15), %rbp
	cmpq	$0, 24(%rbp)
	jne	.LBB39_9
	jmp	.LBB39_22
.LBB39_4:
.Ltmp243:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB39_25:
.Ltmp249:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking19panic_cannot_unwind@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end39:
	.size	_RNSNvYNCINvNtNtCs7jcFBdfocI9_3std6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1b_12DefaultSpawnNtB1b_11ThreadSpawn5spawn0uEs_0INtNtNtCs2k2z8Zem4rB_4core3ops8function6FnOnceuE9call_once6vtableB1d_, .Lfunc_end39-_RNSNvYNCINvNtNtCs7jcFBdfocI9_3std6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1b_12DefaultSpawnNtB1b_11ThreadSpawn5spawn0uEs_0INtNtNtCs2k2z8Zem4rB_4core3ops8function6FnOnceuE9call_once6vtableB1d_
	.cfi_endproc
	.section	.gcc_except_table._RNSNvYNCINvNtNtCs7jcFBdfocI9_3std6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1b_12DefaultSpawnNtB1b_11ThreadSpawn5spawn0uEs_0INtNtNtCs2k2z8Zem4rB_4core3ops8function6FnOnceuE9call_once6vtableB1d_,"a",@progbits
	.p2align	2, 0x0
GCC_except_table39:
.Lexception17:
	.byte	255
	.byte	155
	.uleb128 .Lttbase12-.Lttbaseref12
.Lttbaseref12:
	.byte	1
	.uleb128 .Lcst_end17-.Lcst_begin17
.Lcst_begin17:
	.uleb128 .Ltmp238-.Lfunc_begin39
	.uleb128 .Ltmp239-.Ltmp238
	.uleb128 .Ltmp240-.Lfunc_begin39
	.byte	5
	.uleb128 .Ltmp244-.Lfunc_begin39
	.uleb128 .Ltmp245-.Ltmp244
	.uleb128 .Ltmp246-.Lfunc_begin39
	.byte	7
	.uleb128 .Ltmp250-.Lfunc_begin39
	.uleb128 .Ltmp251-.Ltmp250
	.uleb128 .Ltmp252-.Lfunc_begin39
	.byte	0
	.uleb128 .Ltmp251-.Lfunc_begin39
	.uleb128 .Ltmp253-.Ltmp251
	.byte	0
	.byte	0
	.uleb128 .Ltmp253-.Lfunc_begin39
	.uleb128 .Ltmp254-.Ltmp253
	.uleb128 .Ltmp255-.Lfunc_begin39
	.byte	1
	.uleb128 .Ltmp254-.Lfunc_begin39
	.uleb128 .Ltmp241-.Ltmp254
	.byte	0
	.byte	0
	.uleb128 .Ltmp241-.Lfunc_begin39
	.uleb128 .Ltmp242-.Ltmp241
	.uleb128 .Ltmp243-.Lfunc_begin39
	.byte	1
	.uleb128 .Ltmp247-.Lfunc_begin39
	.uleb128 .Ltmp248-.Ltmp247
	.uleb128 .Ltmp249-.Lfunc_begin39
	.byte	1
	.uleb128 .Ltmp248-.Lfunc_begin39
	.uleb128 .Lfunc_end39-.Ltmp248
	.byte	0
	.byte	0
.Lcst_end17:
	.byte	127
	.byte	0
	.byte	0
	.byte	0
	.byte	1
	.byte	125
	.byte	1
	.byte	0
	.p2align	2, 0x0
	.long	0
.Lttbase12:
	.byte	0
	.p2align	2, 0x0

	.section	.text._RNvCs6a8jV7kq6PJ_10rayon_core19current_num_threads,"ax",@progbits
	.globl	_RNvCs6a8jV7kq6PJ_10rayon_core19current_num_threads
	.prefalign	4, .Lfunc_end40, nop
	.type	_RNvCs6a8jV7kq6PJ_10rayon_core19current_num_threads,@function
_RNvCs6a8jV7kq6PJ_10rayon_core19current_num_threads:
.Lfunc_begin40:
	.cfi_startproc
	pushq	%rax
	.cfi_def_cfa_offset 16
	movq	_RNvNCNKNvNtCs6a8jV7kq6PJ_10rayon_core8registry19WORKER_THREAD_STATE0s_023___RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rax
	movq	%fs:(%rax), %rax
	testq	%rax, %rax
	je	.LBB40_2
	addq	$272, %rax
	movq	(%rax), %rax
	movq	520(%rax), %rax
	popq	%rcx
	.cfi_def_cfa_offset 8
	retq
.LBB40_2:
	.cfi_def_cfa_offset 16
	movq	_RNvNtCs6a8jV7kq6PJ_10rayon_core8registry15global_registry@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	(%rax), %rax
	movq	520(%rax), %rax
	popq	%rcx
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end40:
	.size	_RNvCs6a8jV7kq6PJ_10rayon_core19current_num_threads, .Lfunc_end40-_RNvCs6a8jV7kq6PJ_10rayon_core19current_num_threads
	.cfi_endproc

	.section	.text.unlikely._RNvMNtCs6a8jV7kq6PJ_10rayon_core5sleepNtB2_5Sleep16wake_any_threads,"ax",@progbits
	.globl	_RNvMNtCs6a8jV7kq6PJ_10rayon_core5sleepNtB2_5Sleep16wake_any_threads
	.prefalign	4, .Lfunc_end41, nop
	.type	_RNvMNtCs6a8jV7kq6PJ_10rayon_core5sleepNtB2_5Sleep16wake_any_threads,@function
_RNvMNtCs6a8jV7kq6PJ_10rayon_core5sleepNtB2_5Sleep16wake_any_threads:
.Lfunc_begin41:
	.cfi_startproc
	testl	%esi, %esi
	je	.LBB41_7
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
	movq	16(%rdi), %r12
	movq	%rdi, %r14
	testq	%r12, %r12
	je	.LBB41_6
	movl	%esi, %ebx
	xorl	%r15d, %r15d
	jmp	.LBB41_3
	.p2align	4
.LBB41_4:
	incq	%r15
	cmpq	%r15, %r12
	je	.LBB41_6
.LBB41_3:
	movq	%r14, %rdi
	movq	%r15, %rsi
	callq	_RNvMNtCs6a8jV7kq6PJ_10rayon_core5sleepNtB2_5Sleep20wake_specific_thread.llvm.7294274987384275483
	testb	%al, %al
	je	.LBB41_4
	decl	%ebx
	jne	.LBB41_4
.LBB41_6:
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
	.cfi_restore %rbx
	.cfi_restore %r12
	.cfi_restore %r14
	.cfi_restore %r15
.LBB41_7:
	retq
.Lfunc_end41:
	.size	_RNvMNtCs6a8jV7kq6PJ_10rayon_core5sleepNtB2_5Sleep16wake_any_threads, .Lfunc_end41-_RNvMNtCs6a8jV7kq6PJ_10rayon_core5sleepNtB2_5Sleep16wake_any_threads
	.cfi_endproc

	.section	.text._RNvMNtCs6a8jV7kq6PJ_10rayon_core5sleepNtB2_5Sleep20wake_specific_thread.llvm.7294274987384275483,"ax",@progbits
	.hidden	_RNvMNtCs6a8jV7kq6PJ_10rayon_core5sleepNtB2_5Sleep20wake_specific_thread.llvm.7294274987384275483
	.globl	_RNvMNtCs6a8jV7kq6PJ_10rayon_core5sleepNtB2_5Sleep20wake_specific_thread.llvm.7294274987384275483
	.prefalign	4, .Lfunc_end42, nop
	.type	_RNvMNtCs6a8jV7kq6PJ_10rayon_core5sleepNtB2_5Sleep20wake_specific_thread.llvm.7294274987384275483,@function
_RNvMNtCs6a8jV7kq6PJ_10rayon_core5sleepNtB2_5Sleep20wake_specific_thread.llvm.7294274987384275483:
.Lfunc_begin42:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception18
	pushq	%rbp
	.cfi_def_cfa_offset 16
	pushq	%r15
	.cfi_def_cfa_offset 24
	pushq	%r14
	.cfi_def_cfa_offset 32
	pushq	%r12
	.cfi_def_cfa_offset 40
	pushq	%rbx
	.cfi_def_cfa_offset 48
	subq	$16, %rsp
	.cfi_def_cfa_offset 64
	.cfi_offset %rbx, -48
	.cfi_offset %r12, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	16(%rdi), %rax
	cmpq	%rax, %rsi
	jae	.LBB42_12
	movq	8(%rdi), %rcx
	shlq	$7, %rsi
	movl	$1, %edx
	xorl	%eax, %eax
	movq	%rdi, %r14
	lock		cmpxchgl	%edx, (%rcx,%rsi)
	leaq	(%rcx,%rsi), %rbx
	jne	.LBB42_2
.LBB42_3:
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count18GLOBAL_PANIC_COUNT@GOTPCREL(%rip), %r12
	movq	(%r12), %rax
	shlq	%rax
	testq	%rax, %rax
	jne	.LBB42_5
	xorl	%r15d, %r15d
	movzbl	4(%rbx), %eax
	testb	%al, %al
	jne	.LBB42_7
.LBB42_13:
	movzbl	5(%rbx), %ebp
	testb	%bpl, %bpl
	je	.LBB42_15
	movb	$0, 5(%rbx)
	movq	syscall@GOTPCREL(%rip), %r8
	leaq	8(%rbx), %rsi
	movl	$202, %edi
	movl	$129, %edx
	movl	$1, %ecx
	xorl	%eax, %eax
	lock		incl	8(%rbx)
	callq	*%r8
	lock		decq	24(%r14)
.LBB42_15:
	testb	%r15b, %r15b
	jne	.LBB42_19
	movq	(%r12), %rax
	shlq	%rax
	testq	%rax, %rax
	jne	.LBB42_17
.LBB42_19:
	xorl	%eax, %eax
	xchgl	%eax, (%rbx)
	cmpl	$2, %eax
	je	.LBB42_20
.LBB42_21:
	movl	%ebp, %eax
	addq	$16, %rsp
	.cfi_def_cfa_offset 48
	popq	%rbx
	.cfi_def_cfa_offset 40
	popq	%r12
	.cfi_def_cfa_offset 32
	popq	%r14
	.cfi_def_cfa_offset 24
	popq	%r15
	.cfi_def_cfa_offset 16
	popq	%rbp
	.cfi_def_cfa_offset 8
	retq
.LBB42_2:
	.cfi_def_cfa_offset 64
	movq	_RNvMNtNtNtNtCs7jcFBdfocI9_3std3sys4sync5mutex5futexNtB2_5Mutex14lock_contended@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
	jmp	.LBB42_3
.LBB42_5:
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count17is_zero_slow_path@GOTPCREL(%rip), %rax
	callq	*%rax
	movl	%eax, %r15d
	xorb	$1, %r15b
	movzbl	4(%rbx), %eax
	testb	%al, %al
	je	.LBB42_13
.LBB42_7:
	movq	%rbx, (%rsp)
	movb	%r15b, 8(%rsp)
.Ltmp256:
	movq	_RNvNtCs2k2z8Zem4rB_4core6result13unwrap_failed@GOTPCREL(%rip), %rax
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.26.llvm.7294274987384275483(%rip), %rdi
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.25.llvm.7294274987384275483(%rip), %rcx
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.29(%rip), %r8
	movl	$43, %esi
	movq	%rsp, %rdx
	callq	*%rax
.Ltmp257:
	ud2
.LBB42_20:
	movq	syscall@GOTPCREL(%rip), %r8
	movl	$202, %edi
	movl	$129, %edx
	movl	$1, %ecx
	movq	%rbx, %rsi
	xorl	%eax, %eax
	callq	*%r8
	jmp	.LBB42_21
.LBB42_17:
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count17is_zero_slow_path@GOTPCREL(%rip), %rax
	callq	*%rax
	testb	%al, %al
	jne	.LBB42_19
	movb	$1, 4(%rbx)
	jmp	.LBB42_19
.LBB42_12:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking18panic_bounds_check@GOTPCREL(%rip), %rcx
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.28(%rip), %rdx
	movq	%rsi, %rdi
	movq	%rax, %rsi
	callq	*%rcx
.LBB42_10:
.Ltmp258:
	movq	%rax, %rbx
.Ltmp259:
	movq	%rsp, %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtCs7jcFBdfocI9_3std4sync6poison11PoisonErrorINtNtBE_5mutex10MutexGuardbEEECs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483
.Ltmp260:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB42_9:
.Ltmp261:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end42:
	.size	_RNvMNtCs6a8jV7kq6PJ_10rayon_core5sleepNtB2_5Sleep20wake_specific_thread.llvm.7294274987384275483, .Lfunc_end42-_RNvMNtCs6a8jV7kq6PJ_10rayon_core5sleepNtB2_5Sleep20wake_specific_thread.llvm.7294274987384275483
	.cfi_endproc
	.section	.gcc_except_table._RNvMNtCs6a8jV7kq6PJ_10rayon_core5sleepNtB2_5Sleep20wake_specific_thread.llvm.7294274987384275483,"a",@progbits
	.p2align	2, 0x0
GCC_except_table42:
.Lexception18:
	.byte	255
	.byte	155
	.uleb128 .Lttbase13-.Lttbaseref13
.Lttbaseref13:
	.byte	1
	.uleb128 .Lcst_end18-.Lcst_begin18
.Lcst_begin18:
	.uleb128 .Lfunc_begin42-.Lfunc_begin42
	.uleb128 .Ltmp256-.Lfunc_begin42
	.byte	0
	.byte	0
	.uleb128 .Ltmp256-.Lfunc_begin42
	.uleb128 .Ltmp257-.Ltmp256
	.uleb128 .Ltmp258-.Lfunc_begin42
	.byte	0
	.uleb128 .Ltmp257-.Lfunc_begin42
	.uleb128 .Ltmp259-.Ltmp257
	.byte	0
	.byte	0
	.uleb128 .Ltmp259-.Lfunc_begin42
	.uleb128 .Ltmp260-.Ltmp259
	.uleb128 .Ltmp261-.Lfunc_begin42
	.byte	1
	.uleb128 .Ltmp260-.Lfunc_begin42
	.uleb128 .Lfunc_end42-.Ltmp260
	.byte	0
	.byte	0
.Lcst_end18:
	.byte	127
	.byte	0
	.p2align	2, 0x0
.Lttbase13:
	.byte	0
	.p2align	2, 0x0

	.section	.text._RNvMNtCs6a8jV7kq6PJ_10rayon_core8registryNtB2_13ThreadBuilder3run,"ax",@progbits
	.globl	_RNvMNtCs6a8jV7kq6PJ_10rayon_core8registryNtB2_13ThreadBuilder3run
	.prefalign	4, .Lfunc_end43, nop
	.type	_RNvMNtCs6a8jV7kq6PJ_10rayon_core8registryNtB2_13ThreadBuilder3run,@function
_RNvMNtCs6a8jV7kq6PJ_10rayon_core8registryNtB2_13ThreadBuilder3run:
.Lfunc_begin43:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception19
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r15
	pushq	%r14
	pushq	%r13
	pushq	%r12
	pushq	%rbx
	andq	$-128, %rsp
	subq	$512, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	_RNvXs6_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThreadINtNtCs2k2z8Zem4rB_4core7convert4FromNtB5_13ThreadBuilderE4from@GOTPCREL(%rip), %rax
	movq	%rdi, %rsi
	movq	%rsp, %rbx
	movq	%rbx, %rdi
	callq	*%rax
	movq	_RNvNCNKNvNtCs6a8jV7kq6PJ_10rayon_core8registry19WORKER_THREAD_STATE0s_023___RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rax
	cmpq	$0, %fs:(%rax)
	jne	.LBB43_1
	movq	272(%rsp), %r12
	movq	%rbx, %fs:(%rax)
	movq	256(%rsp), %rbx
	movq	520(%r12), %rsi
	cmpq	%rsi, %rbx
	jae	.LBB43_12
	movq	512(%r12), %rax
	leaq	(%rbx,%rbx,2), %rcx
	shlq	$4, %rcx
	leaq	24(%rax,%rcx), %rdi
.Ltmp266:
	callq	_RNvXs4_NtCs6a8jV7kq6PJ_10rayon_core5latchNtB5_9LockLatchNtB5_5Latch3set
.Ltmp267:
	movq	432(%r12), %rdi
	testq	%rdi, %rdi
	je	.LBB43_10
	movq	440(%r12), %rax
	movq	40(%rax), %rax
.Ltmp269:
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp270:
.LBB43_10:
	movq	272(%rsp), %r15
	movq	256(%rsp), %r14
	movq	520(%r15), %rsi
	cmpq	%rsi, %r14
	jae	.LBB43_11
	movq	512(%r15), %rax
	leaq	(%r14,%r14,2), %r13
	shlq	$4, %r13
	movq	16(%rax,%r13), %rcx
	cmpq	$3, %rcx
	jne	.LBB43_22
.LBB43_23:
	movq	520(%r15), %rsi
	cmpq	%rsi, %r14
	jae	.LBB43_24
	movq	512(%r15), %rax
	leaq	36(%rax,%r13), %rdi
.Ltmp290:
	callq	_RNvXs4_NtCs6a8jV7kq6PJ_10rayon_core5latchNtB5_9LockLatchNtB5_5Latch3set
.Ltmp291:
	movq	448(%r12), %rdi
	testq	%rdi, %rdi
	je	.LBB43_30
	movq	456(%r12), %rax
	movq	40(%rax), %rax
.Ltmp295:
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp296:
.LBB43_30:
	movq	%rsp, %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry12WorkerThreadEBF_
	leaq	-40(%rbp), %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB43_1:
	.cfi_def_cfa %rbp, 16
.Ltmp262:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking5panic@GOTPCREL(%rip), %rax
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.18(%rip), %rdi
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.19(%rip), %rdx
	movl	$35, %esi
	callq	*%rax
.Ltmp263:
	jmp	.LBB43_2
.LBB43_22:
.Ltmp286:
	leaq	16(%rax,%r13), %rsi
	movq	_RNvMs8_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThread15wait_until_cold@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	callq	*%rax
.Ltmp287:
	jmp	.LBB43_23
.LBB43_12:
.Ltmp264:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking18panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.51(%rip), %rdx
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp265:
	jmp	.LBB43_2
.LBB43_11:
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.45(%rip), %rdx
	jmp	.LBB43_25
.LBB43_24:
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.46(%rip), %rdx
.LBB43_25:
.Ltmp288:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking18panic_bounds_check@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
.Ltmp289:
.LBB43_2:
	ud2
.LBB43_32:
.Ltmp297:
.Ltmp298:
	movq	_RNvCs2NWS7XDLE6y_7___rustc20___rust_panic_cleanup@GOTPCREL(%rip), %rcx
	movq	%rax, %rdi
	callq	*%rcx
.Ltmp299:
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count18GLOBAL_PANIC_COUNT@GOTPCREL(%rip), %rcx
	movq	%rax, %r14
	movq	%rdx, %rbx
	lock		decq	(%rcx)
	decq	%fs:_RNvNCNKNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count17LOCAL_PANIC_COUNT0s_023___RUST_STD_INTERNAL_VAL.llvm.11640361436736466388@TPOFF
	movb	$0, %fs:_RNvNCNKNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count17LOCAL_PANIC_COUNT0s_023___RUST_STD_INTERNAL_VAL.llvm.11640361436736466388@TPOFF+8
	movq	416(%r12), %rdi
	testq	%rdi, %rdi
	je	.LBB43_35
	movq	424(%r12), %rax
	movq	40(%rax), %rax
.Ltmp301:
	movq	%r14, %rsi
	movq	%rbx, %rdx
	callq	*%rax
.Ltmp302:
	jmp	.LBB43_30
.LBB43_35:
.Ltmp306:
	movq	_RNvNtNtCs7jcFBdfocI9_3std2io5stdio7__eprint@GOTPCREL(%rip), %rax
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.52.llvm.7294274987384275483(%rip), %rdi
	movl	$87, %esi
	callq	*%rax
.Ltmp307:
	jmp	.LBB43_18
.LBB43_38:
.Ltmp308:
	movq	%rax, %r15
.Ltmp309:
	movq	%r14, %rdi
	movq	%rbx, %rsi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EECs6a8jV7kq6PJ_10rayon_core
.Ltmp310:
	jmp	.LBB43_4
.LBB43_36:
.Ltmp303:
.Ltmp304:
	movq	_RNvNtNtCs7jcFBdfocI9_3std2io5stdio7__eprint@GOTPCREL(%rip), %rax
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.52.llvm.7294274987384275483(%rip), %rdi
	movl	$87, %esi
	callq	*%rax
.Ltmp305:
	jmp	.LBB43_18
.LBB43_37:
.Ltmp311:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB43_31:
.Ltmp300:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking19panic_cannot_unwind@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB43_14:
.Ltmp271:
.Ltmp272:
	movq	_RNvCs2NWS7XDLE6y_7___rustc20___rust_panic_cleanup@GOTPCREL(%rip), %rcx
	movq	%rax, %rdi
	callq	*%rcx
.Ltmp273:
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count18GLOBAL_PANIC_COUNT@GOTPCREL(%rip), %rcx
	movq	%rax, %r15
	movq	%rdx, %r14
	lock		decq	(%rcx)
	decq	%fs:_RNvNCNKNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count17LOCAL_PANIC_COUNT0s_023___RUST_STD_INTERNAL_VAL.llvm.11640361436736466388@TPOFF
	movb	$0, %fs:_RNvNCNKNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count17LOCAL_PANIC_COUNT0s_023___RUST_STD_INTERNAL_VAL.llvm.11640361436736466388@TPOFF+8
	movq	416(%r12), %rdi
	testq	%rdi, %rdi
	je	.LBB43_17
	movq	424(%r12), %rax
	movq	40(%rax), %rax
.Ltmp275:
	movq	%r15, %rsi
	movq	%r14, %rdx
	callq	*%rax
.Ltmp276:
	jmp	.LBB43_10
.LBB43_17:
.Ltmp280:
	movq	_RNvNtNtCs7jcFBdfocI9_3std2io5stdio7__eprint@GOTPCREL(%rip), %rax
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.52.llvm.7294274987384275483(%rip), %rdi
	movl	$87, %esi
	callq	*%rax
.Ltmp281:
	jmp	.LBB43_18
.LBB43_39:
.Ltmp282:
.Ltmp283:
	movq	%r15, %rdi
	movq	%r14, %rsi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EECs6a8jV7kq6PJ_10rayon_core
.Ltmp284:
	jmp	.LBB43_40
.LBB43_19:
.Ltmp277:
.Ltmp278:
	movq	_RNvNtNtCs7jcFBdfocI9_3std2io5stdio7__eprint@GOTPCREL(%rip), %rax
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.52.llvm.7294274987384275483(%rip), %rdi
	movl	$87, %esi
	callq	*%rax
.Ltmp279:
	jmp	.LBB43_18
.LBB43_20:
.Ltmp285:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB43_13:
.Ltmp274:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking19panic_cannot_unwind@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB43_26:
.Ltmp292:
.LBB43_40:
.Ltmp293:
	movq	_RNvNtNtCs7jcFBdfocI9_3std2io5stdio7__eprint@GOTPCREL(%rip), %rax
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.52.llvm.7294274987384275483(%rip), %rdi
	movl	$87, %esi
	callq	*%rax
.Ltmp294:
.LBB43_18:
	movq	_RNvNtCs7jcFBdfocI9_3std7process5abort@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB43_3:
.Ltmp268:
	movq	%rax, %r15
.LBB43_4:
.Ltmp312:
	movq	%rsp, %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry12WorkerThreadEBF_
.Ltmp313:
	movq	%r15, %rdi
	callq	_Unwind_Resume@PLT
.LBB43_41:
.Ltmp314:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end43:
	.size	_RNvMNtCs6a8jV7kq6PJ_10rayon_core8registryNtB2_13ThreadBuilder3run, .Lfunc_end43-_RNvMNtCs6a8jV7kq6PJ_10rayon_core8registryNtB2_13ThreadBuilder3run
	.cfi_endproc
	.section	.gcc_except_table._RNvMNtCs6a8jV7kq6PJ_10rayon_core8registryNtB2_13ThreadBuilder3run,"a",@progbits
	.p2align	2, 0x0
GCC_except_table43:
.Lexception19:
	.byte	255
	.byte	155
	.uleb128 .Lttbase14-.Lttbaseref14
.Lttbaseref14:
	.byte	1
	.uleb128 .Lcst_end19-.Lcst_begin19
.Lcst_begin19:
	.uleb128 .Lfunc_begin43-.Lfunc_begin43
	.uleb128 .Ltmp266-.Lfunc_begin43
	.byte	0
	.byte	0
	.uleb128 .Ltmp266-.Lfunc_begin43
	.uleb128 .Ltmp267-.Ltmp266
	.uleb128 .Ltmp268-.Lfunc_begin43
	.byte	0
	.uleb128 .Ltmp269-.Lfunc_begin43
	.uleb128 .Ltmp270-.Ltmp269
	.uleb128 .Ltmp271-.Lfunc_begin43
	.byte	5
	.uleb128 .Ltmp290-.Lfunc_begin43
	.uleb128 .Ltmp291-.Ltmp290
	.uleb128 .Ltmp292-.Lfunc_begin43
	.byte	0
	.uleb128 .Ltmp295-.Lfunc_begin43
	.uleb128 .Ltmp296-.Ltmp295
	.uleb128 .Ltmp297-.Lfunc_begin43
	.byte	5
	.uleb128 .Ltmp296-.Lfunc_begin43
	.uleb128 .Ltmp262-.Ltmp296
	.byte	0
	.byte	0
	.uleb128 .Ltmp262-.Lfunc_begin43
	.uleb128 .Ltmp263-.Ltmp262
	.uleb128 .Ltmp268-.Lfunc_begin43
	.byte	0
	.uleb128 .Ltmp286-.Lfunc_begin43
	.uleb128 .Ltmp287-.Ltmp286
	.uleb128 .Ltmp292-.Lfunc_begin43
	.byte	0
	.uleb128 .Ltmp264-.Lfunc_begin43
	.uleb128 .Ltmp265-.Ltmp264
	.uleb128 .Ltmp268-.Lfunc_begin43
	.byte	0
	.uleb128 .Ltmp288-.Lfunc_begin43
	.uleb128 .Ltmp289-.Ltmp288
	.uleb128 .Ltmp292-.Lfunc_begin43
	.byte	0
	.uleb128 .Ltmp298-.Lfunc_begin43
	.uleb128 .Ltmp299-.Ltmp298
	.uleb128 .Ltmp300-.Lfunc_begin43
	.byte	1
	.uleb128 .Ltmp301-.Lfunc_begin43
	.uleb128 .Ltmp302-.Ltmp301
	.uleb128 .Ltmp303-.Lfunc_begin43
	.byte	0
	.uleb128 .Ltmp306-.Lfunc_begin43
	.uleb128 .Ltmp307-.Ltmp306
	.uleb128 .Ltmp308-.Lfunc_begin43
	.byte	0
	.uleb128 .Ltmp309-.Lfunc_begin43
	.uleb128 .Ltmp305-.Ltmp309
	.uleb128 .Ltmp311-.Lfunc_begin43
	.byte	1
	.uleb128 .Ltmp305-.Lfunc_begin43
	.uleb128 .Ltmp272-.Ltmp305
	.byte	0
	.byte	0
	.uleb128 .Ltmp272-.Lfunc_begin43
	.uleb128 .Ltmp273-.Ltmp272
	.uleb128 .Ltmp274-.Lfunc_begin43
	.byte	1
	.uleb128 .Ltmp275-.Lfunc_begin43
	.uleb128 .Ltmp276-.Ltmp275
	.uleb128 .Ltmp277-.Lfunc_begin43
	.byte	0
	.uleb128 .Ltmp280-.Lfunc_begin43
	.uleb128 .Ltmp281-.Ltmp280
	.uleb128 .Ltmp282-.Lfunc_begin43
	.byte	0
	.uleb128 .Ltmp283-.Lfunc_begin43
	.uleb128 .Ltmp279-.Ltmp283
	.uleb128 .Ltmp285-.Lfunc_begin43
	.byte	1
	.uleb128 .Ltmp279-.Lfunc_begin43
	.uleb128 .Ltmp293-.Ltmp279
	.byte	0
	.byte	0
	.uleb128 .Ltmp293-.Lfunc_begin43
	.uleb128 .Ltmp294-.Ltmp293
	.uleb128 .Ltmp314-.Lfunc_begin43
	.byte	1
	.uleb128 .Ltmp294-.Lfunc_begin43
	.uleb128 .Ltmp312-.Ltmp294
	.byte	0
	.byte	0
	.uleb128 .Ltmp312-.Lfunc_begin43
	.uleb128 .Ltmp313-.Ltmp312
	.uleb128 .Ltmp314-.Lfunc_begin43
	.byte	1
	.uleb128 .Ltmp313-.Lfunc_begin43
	.uleb128 .Lfunc_end43-.Ltmp313
	.byte	0
	.byte	0
.Lcst_end19:
	.byte	127
	.byte	0
	.byte	0
	.byte	0
	.byte	1
	.byte	125
	.p2align	2, 0x0
	.long	0
.Lttbase14:
	.byte	0
	.p2align	2, 0x0

	.section	.text._RNvMs3_NtCs6a8jV7kq6PJ_10rayon_core5latchNtB5_9LockLatch14wait_and_reset,"ax",@progbits
	.globl	_RNvMs3_NtCs6a8jV7kq6PJ_10rayon_core5latchNtB5_9LockLatch14wait_and_reset
	.prefalign	4, .Lfunc_end44, nop
	.type	_RNvMs3_NtCs6a8jV7kq6PJ_10rayon_core5latchNtB5_9LockLatch14wait_and_reset,@function
_RNvMs3_NtCs6a8jV7kq6PJ_10rayon_core5latchNtB5_9LockLatch14wait_and_reset:
.Lfunc_begin44:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception20
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
	subq	$40, %rsp
	.cfi_def_cfa_offset 96
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movl	$1, %ecx
	xorl	%eax, %eax
	movq	%rdi, %rbx
	lock		cmpxchgl	%ecx, (%rdi)
	jne	.LBB44_1
.LBB44_2:
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count18GLOBAL_PANIC_COUNT@GOTPCREL(%rip), %rax
	movq	(%rax), %rax
	shlq	%rax
	testq	%rax, %rax
	jne	.LBB44_4
	xorl	%eax, %eax
	movzbl	4(%rbx), %ecx
	testb	%cl, %cl
	jne	.LBB44_19
.LBB44_6:
	movq	syscall@GOTPCREL(%rip), %r12
	movq	__errno_location@GOTPCREL(%rip), %rbp
	leaq	8(%rbx), %r14
	leaq	24(%rsp), %r13
	movl	%eax, 12(%rsp)
	.p2align	4
.LBB44_7:
	cmpb	$0, 5(%rbx)
	jne	.LBB44_25
	movl	8(%rbx), %r15d
	xorl	%eax, %eax
	xchgl	%eax, (%rbx)
	cmpl	$2, %eax
	je	.LBB44_9
.LBB44_10:
	movq	$0, 16(%rsp)
	.p2align	4
.LBB44_11:
	movl	(%r14), %eax
	cmpl	%r15d, %eax
	jne	.LBB44_14
	cmpb	$0, 16(%rsp)
	movl	$0, %r8d
	movl	$202, %edi
	movl	$137, %edx
	movl	$-1, (%rsp)
	movq	%r14, %rsi
	movl	%r15d, %ecx
	cmovneq	%r13, %r8
	xorl	%r9d, %r9d
	xorl	%eax, %eax
	callq	*%r12
	testq	%rax, %rax
	jns	.LBB44_14
	callq	*%rbp
	cmpl	$4, (%rax)
	je	.LBB44_11
.LBB44_14:
	movl	$1, %ecx
	xorl	%eax, %eax
	lock		cmpxchgl	%ecx, (%rbx)
	jne	.LBB44_15
	movzbl	4(%rbx), %eax
	testb	%al, %al
	je	.LBB44_7
	jmp	.LBB44_17
.LBB44_9:
	movl	$202, %edi
	movl	$129, %edx
	movl	$1, %ecx
	movq	%rbx, %rsi
	xorl	%eax, %eax
	callq	*%r12
	jmp	.LBB44_10
.LBB44_15:
	movq	_RNvMNtNtNtNtCs7jcFBdfocI9_3std3sys4sync5mutex5futexNtB2_5Mutex14lock_contended@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
	movzbl	4(%rbx), %eax
	testb	%al, %al
	je	.LBB44_7
.LBB44_17:
	movl	12(%rsp), %eax
	movq	%rbx, 16(%rsp)
	movb	%al, 24(%rsp)
.Ltmp321:
	movq	_RNvNtCs2k2z8Zem4rB_4core6result13unwrap_failed@GOTPCREL(%rip), %rax
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.26.llvm.7294274987384275483(%rip), %rdi
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.25.llvm.7294274987384275483(%rip), %rcx
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.36.llvm.7294274987384275483(%rip), %r8
	leaq	16(%rsp), %rdx
	movl	$43, %esi
	callq	*%rax
.Ltmp322:
	jmp	.LBB44_20
.LBB44_25:
	cmpb	$0, 12(%rsp)
	movb	$0, 5(%rbx)
	jne	.LBB44_29
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count18GLOBAL_PANIC_COUNT@GOTPCREL(%rip), %rax
	movq	(%rax), %rax
	shlq	%rax
	testq	%rax, %rax
	jne	.LBB44_27
.LBB44_29:
	xorl	%eax, %eax
	xchgl	%eax, (%rbx)
	cmpl	$2, %eax
	je	.LBB44_31
	addq	$40, %rsp
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
.LBB44_1:
	.cfi_def_cfa_offset 96
	movq	_RNvMNtNtNtNtCs7jcFBdfocI9_3std3sys4sync5mutex5futexNtB2_5Mutex14lock_contended@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
	jmp	.LBB44_2
.LBB44_4:
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count17is_zero_slow_path@GOTPCREL(%rip), %rax
	callq	*%rax
	xorb	$1, %al
	movzbl	4(%rbx), %ecx
	testb	%cl, %cl
	je	.LBB44_6
.LBB44_19:
	movq	%rbx, 16(%rsp)
	movb	%al, 24(%rsp)
.Ltmp315:
	movq	_RNvNtCs2k2z8Zem4rB_4core6result13unwrap_failed@GOTPCREL(%rip), %rax
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.26.llvm.7294274987384275483(%rip), %rdi
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.25.llvm.7294274987384275483(%rip), %rcx
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.35.llvm.7294274987384275483(%rip), %r8
	leaq	16(%rsp), %rdx
	movl	$43, %esi
	callq	*%rax
.Ltmp316:
.LBB44_20:
	ud2
.LBB44_31:
	movl	$202, %edi
	movq	%rbx, %rsi
	movl	$129, %edx
	movl	$1, %ecx
	xorl	%eax, %eax
	addq	$40, %rsp
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
	jmpq	*syscall@GOTPCREL(%rip)
.LBB44_27:
	.cfi_def_cfa_offset 96
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count17is_zero_slow_path@GOTPCREL(%rip), %rax
	callq	*%rax
	testb	%al, %al
	jne	.LBB44_29
	movb	$1, 4(%rbx)
	jmp	.LBB44_29
.LBB44_18:
.Ltmp317:
	movq	%rax, %rbx
.Ltmp318:
	leaq	16(%rsp), %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtCs7jcFBdfocI9_3std4sync6poison11PoisonErrorINtNtBE_5mutex10MutexGuardbEEECs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483
.Ltmp319:
	jmp	.LBB44_23
.LBB44_21:
.Ltmp320:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB44_22:
.Ltmp323:
	movq	%rax, %rbx
.Ltmp324:
	leaq	16(%rsp), %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtCs7jcFBdfocI9_3std4sync6poison11PoisonErrorINtNtBE_5mutex10MutexGuardbEEECs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483
.Ltmp325:
.LBB44_23:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB44_24:
.Ltmp326:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end44:
	.size	_RNvMs3_NtCs6a8jV7kq6PJ_10rayon_core5latchNtB5_9LockLatch14wait_and_reset, .Lfunc_end44-_RNvMs3_NtCs6a8jV7kq6PJ_10rayon_core5latchNtB5_9LockLatch14wait_and_reset
	.cfi_endproc
	.section	.gcc_except_table._RNvMs3_NtCs6a8jV7kq6PJ_10rayon_core5latchNtB5_9LockLatch14wait_and_reset,"a",@progbits
	.p2align	2, 0x0
GCC_except_table44:
.Lexception20:
	.byte	255
	.byte	155
	.uleb128 .Lttbase15-.Lttbaseref15
.Lttbaseref15:
	.byte	1
	.uleb128 .Lcst_end20-.Lcst_begin20
.Lcst_begin20:
	.uleb128 .Lfunc_begin44-.Lfunc_begin44
	.uleb128 .Ltmp321-.Lfunc_begin44
	.byte	0
	.byte	0
	.uleb128 .Ltmp321-.Lfunc_begin44
	.uleb128 .Ltmp322-.Ltmp321
	.uleb128 .Ltmp323-.Lfunc_begin44
	.byte	0
	.uleb128 .Ltmp322-.Lfunc_begin44
	.uleb128 .Ltmp315-.Ltmp322
	.byte	0
	.byte	0
	.uleb128 .Ltmp315-.Lfunc_begin44
	.uleb128 .Ltmp316-.Ltmp315
	.uleb128 .Ltmp317-.Lfunc_begin44
	.byte	0
	.uleb128 .Ltmp316-.Lfunc_begin44
	.uleb128 .Ltmp318-.Ltmp316
	.byte	0
	.byte	0
	.uleb128 .Ltmp318-.Lfunc_begin44
	.uleb128 .Ltmp319-.Ltmp318
	.uleb128 .Ltmp320-.Lfunc_begin44
	.byte	1
	.uleb128 .Ltmp319-.Lfunc_begin44
	.uleb128 .Ltmp324-.Ltmp319
	.byte	0
	.byte	0
	.uleb128 .Ltmp324-.Lfunc_begin44
	.uleb128 .Ltmp325-.Ltmp324
	.uleb128 .Ltmp326-.Lfunc_begin44
	.byte	1
	.uleb128 .Ltmp325-.Lfunc_begin44
	.uleb128 .Lfunc_end44-.Ltmp325
	.byte	0
	.byte	0
.Lcst_end20:
	.byte	127
	.byte	0
	.p2align	2, 0x0
.Lttbase15:
	.byte	0
	.p2align	2, 0x0

	.section	.text.unlikely._RNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE6resizeBZ_,"ax",@progbits
	.prefalign	4, .Lfunc_end45, nop
	.type	_RNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE6resizeBZ_,@function
_RNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE6resizeBZ_:
.Lfunc_begin45:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception21
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
	subq	$56, %rsp
	.cfi_def_cfa_offset 112
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	(%rdi), %rax
	movq	%rsi, %r15
	shlq	$4, %r15
	movabsq	$9223372036854775800, %rcx
	movq	264(%rax), %rbp
	movq	256(%rax), %r13
	movq	%rsi, %rax
	shrq	$60, %rax
	setne	%al
	cmpq	%rcx, %r15
	seta	%cl
	orb	%al, %cl
	je	.LBB45_3
	xorl	%edi, %edi
	movq	_RNvNtCsc70TAahYccp_5alloc7raw_vec12handle_error@GOTPCREL(%rip), %rax
	movq	%r15, %rsi
	callq	*%rax
.LBB45_3:
	movq	malloc@GOTPCREL(%rip), %rax
	movq	8(%rdi), %rbx
	movq	16(%rdi), %r12
	movq	%rsi, (%rsp)
	movq	%rdi, 16(%rsp)
	movq	%r15, %rdi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB45_4
	movq	%rax, %r14
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rdi
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rsi
	movabsq	$-9223372036854775808, %rdx
	leaq	(%r15,%rax), %rcx
	sarq	$63, %rcx
	xorq	%rdx, %rcx
	addq	%r15, %rax
	cmovoq	%rcx, %rax
	incq	%rdi
	movq	$-1, %rcx
	cmoveq	%rcx, %rdi
	addq	%r15, %rsi
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmovbq	%rcx, %rsi
	movq	%rdi, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%rsi, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jle	.LBB45_7
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB45_7:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB45_13
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB45_7
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdx
	lock		incq	(%rax)
	lock		addq	%r15, (%rcx)
	movq	%r15, %rcx
	lock		xaddq	%rcx, (%rdx)
	movabsq	$-9223372036854775808, %rdx
	leaq	(%rcx,%r15), %rax
	sarq	$63, %rax
	xorq	%rdx, %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	addq	%r15, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB45_10:
	cmpq	%rax, %rcx
	jle	.LBB45_12
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB45_10
.LBB45_12:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB45_13:
	movq	%r13, %rcx
	subq	%rbp, %rcx
	je	.LBB45_17
	movq	(%rsp), %rax
	movl	%ebp, %edx
	subl	%r13d, %edx
	decq	%r12
	andl	$7, %edx
	leaq	-1(%rax), %rax
	je	.LBB45_16
	.p2align	4
.LBB45_15:
	movq	%r13, %rsi
	andq	%r12, %rsi
	movq	%r13, %rdi
	andq	%rax, %rdi
	incq	%r13
	shlq	$4, %rsi
	shlq	$4, %rdi
	decq	%rdx
	vmovups	(%rbx,%rsi), %xmm0
	vmovups	%xmm0, (%r14,%rdi)
	jne	.LBB45_15
.LBB45_16:
	cmpq	$-8, %rcx
	ja	.LBB45_17
	.p2align	4
.LBB45_30:
	movq	%r13, %rcx
	andq	%r12, %rcx
	movq	%r13, %rdx
	andq	%rax, %rdx
	shlq	$4, %rcx
	shlq	$4, %rdx
	vmovups	(%rbx,%rcx), %xmm0
	leaq	1(%r13), %rcx
	movq	%rcx, %rsi
	andq	%r12, %rsi
	andq	%rax, %rcx
	shlq	$4, %rsi
	shlq	$4, %rcx
	vmovups	%xmm0, (%r14,%rdx)
	leaq	2(%r13), %rdx
	vmovups	(%rbx,%rsi), %xmm0
	movq	%rdx, %rsi
	andq	%r12, %rsi
	andq	%rax, %rdx
	shlq	$4, %rsi
	shlq	$4, %rdx
	vmovups	%xmm0, (%r14,%rcx)
	leaq	3(%r13), %rcx
	vmovups	(%rbx,%rsi), %xmm0
	vmovups	%xmm0, (%r14,%rdx)
	movq	%rcx, %rdx
	andq	%r12, %rdx
	andq	%rax, %rcx
	shlq	$4, %rdx
	shlq	$4, %rcx
	vmovups	(%rbx,%rdx), %xmm0
	leaq	4(%r13), %rdx
	movq	%rdx, %rsi
	andq	%r12, %rsi
	andq	%rax, %rdx
	shlq	$4, %rsi
	shlq	$4, %rdx
	vmovups	%xmm0, (%r14,%rcx)
	leaq	5(%r13), %rcx
	vmovups	(%rbx,%rsi), %xmm0
	movq	%rcx, %rsi
	andq	%r12, %rsi
	andq	%rax, %rcx
	shlq	$4, %rsi
	shlq	$4, %rcx
	vmovups	%xmm0, (%r14,%rdx)
	vmovups	(%rbx,%rsi), %xmm0
	vmovups	%xmm0, (%r14,%rcx)
	leaq	6(%r13), %rcx
	movq	%rcx, %rdx
	andq	%r12, %rdx
	andq	%rax, %rcx
	shlq	$4, %rdx
	shlq	$4, %rcx
	vmovups	(%rbx,%rdx), %xmm0
	leaq	7(%r13), %rdx
	addq	$8, %r13
	movq	%rdx, %rsi
	andq	%r12, %rsi
	andq	%rax, %rdx
	shlq	$4, %rsi
	shlq	$4, %rdx
	vmovups	%xmm0, (%r14,%rcx)
	vmovups	(%rbx,%rsi), %xmm0
	vmovups	%xmm0, (%r14,%rdx)
	cmpq	%rbp, %r13
	jne	.LBB45_30
.LBB45_17:
	callq	_RINvNtCs18aJq3QiqAb_15crossbeam_epoch7default11with_handleNCNvB2_3pin0NtNtB4_5guard5GuardECs6a8jV7kq6PJ_10rayon_core
	movq	%rax, %r15
	movq	%rax, 8(%rsp)
	movq	16(%rsp), %rax
	movq	(%rsp), %r13
	movl	$16, %edi
	movq	%r14, 8(%rax)
	movq	%r13, 16(%rax)
	movq	(%rax), %rbx
	movq	malloc@GOTPCREL(%rip), %rax
	callq	*%rax
	testq	%rax, %rax
	je	.LBB45_28
	movq	%rax, %r12
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	addq	$16, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	addq	$16, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jle	.LBB45_20
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB45_20:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB45_26
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB45_20
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	$16, (%rdx)
	movl	$16, %edx
	lock		xaddq	%rdx, (%rsi)
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rsi
	movq	(%rsi), %rax
	addq	$16, %rdx
	cmovoq	%rcx, %rdx
	.p2align	4
.LBB45_23:
	cmpq	%rax, %rdx
	jle	.LBB45_25
	lock		cmpxchgq	%rdx, (%rsi)
	jne	.LBB45_23
.LBB45_25:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB45_26:
	movq	%r14, (%r12)
	movq	%r13, 8(%r12)
	xchgq	%r12, 128(%rbx)
	testq	%r15, %r15
	je	.LBB45_31
	leaq	_RINvNvMs_NtCs18aJq3QiqAb_15crossbeam_epoch8deferredNtB7_8Deferred3new4callNCINvMNtB9_5guardNtB1g_5Guard15defer_uncheckedNCNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB22_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE6resize0uE0EB2X_(%rip), %rax
	movq	%rax, 24(%rsp)
	movq	%r12, 32(%rsp)
.Ltmp327:
	movq	_RNvMs6_NtCs18aJq3QiqAb_15crossbeam_epoch8internalNtB5_5Local5defer@GOTPCREL(%rip), %rax
	leaq	24(%rsp), %rsi
	leaq	8(%rsp), %rdx
	movq	%r15, %rdi
	callq	*%rax
.Ltmp328:
	cmpq	$64, %r13
	jae	.LBB45_51
	jmp	.LBB45_52
.LBB45_31:
	andq	$-8, %r12
	movq	8(%r12), %rdx
	testq	%rdx, %rdx
	je	.LBB45_41
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	shlq	$4, %rdx
	movq	(%r12), %rdi
	xorl	%esi, %esi
	cmpq	%rdx, %rax
	setns	%sil
	addq	%rcx, %rsi
	subq	%rdx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB45_34
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB45_34:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB45_40
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB45_34
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rdx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rdx, %rsi
	setns	%al
	addq	%rcx, %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	subq	%rdx, %rsi
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB45_37:
	cmpq	%rax, %rsi
	jge	.LBB45_39
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB45_37
.LBB45_39:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB45_40:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB45_41:
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movabsq	$-9223372036854775808, %rcx
	addq	$-16, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB45_43
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB45_43:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB45_49
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB45_43
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	$-16, %rcx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	lock		xaddq	%rcx, (%rax)
	movabsq	$-9223372036854775808, %rax
	addq	$-16, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB45_46:
	cmpq	%rax, %rcx
	jge	.LBB45_48
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB45_46
.LBB45_48:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB45_49:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	callq	*%rax
	cmpq	$64, %r13
	jb	.LBB45_52
.LBB45_51:
.Ltmp329:
	movq	_RNvMNtCs18aJq3QiqAb_15crossbeam_epoch5guardNtB2_5Guard5flush@GOTPCREL(%rip), %rax
	leaq	8(%rsp), %rdi
	callq	*%rax
.Ltmp330:
.LBB45_52:
	testq	%r15, %r15
	je	.LBB45_55
	decq	2072(%r15)
	jne	.LBB45_55
	movq	$0, 2176(%r15)
	cmpq	$0, 2080(%r15)
	je	.LBB45_59
.LBB45_55:
	addq	$56, %rsp
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
.LBB45_59:
	.cfi_def_cfa_offset 112
	movq	%r15, %rdi
	addq	$56, %rsp
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
	jmpq	*_RNvMs6_NtCs18aJq3QiqAb_15crossbeam_epoch8internalNtB5_5Local8finalize@GOTPCREL(%rip)
.LBB45_4:
	.cfi_def_cfa_offset 112
	movl	$8, %edi
	movq	_RNvNtCsc70TAahYccp_5alloc7raw_vec12handle_error@GOTPCREL(%rip), %rax
	movq	%r15, %rsi
	callq	*%rax
.LBB45_28:
.Ltmp331:
	movq	_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$16, %esi
	callq	*%rax
.Ltmp332:
	ud2
.LBB45_57:
.Ltmp333:
	movq	%rax, %rbx
.Ltmp334:
	movq	%r15, %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs18aJq3QiqAb_15crossbeam_epoch5guard5GuardECs6a8jV7kq6PJ_10rayon_core
.Ltmp335:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB45_56:
.Ltmp336:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end45:
	.size	_RNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE6resizeBZ_, .Lfunc_end45-_RNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE6resizeBZ_
	.cfi_endproc
	.section	.gcc_except_table._RNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE6resizeBZ_,"a",@progbits
	.p2align	2, 0x0
GCC_except_table45:
.Lexception21:
	.byte	255
	.byte	155
	.uleb128 .Lttbase16-.Lttbaseref16
.Lttbaseref16:
	.byte	1
	.uleb128 .Lcst_end21-.Lcst_begin21
.Lcst_begin21:
	.uleb128 .Lfunc_begin45-.Lfunc_begin45
	.uleb128 .Ltmp327-.Lfunc_begin45
	.byte	0
	.byte	0
	.uleb128 .Ltmp327-.Lfunc_begin45
	.uleb128 .Ltmp328-.Ltmp327
	.uleb128 .Ltmp333-.Lfunc_begin45
	.byte	0
	.uleb128 .Ltmp328-.Lfunc_begin45
	.uleb128 .Ltmp329-.Ltmp328
	.byte	0
	.byte	0
	.uleb128 .Ltmp329-.Lfunc_begin45
	.uleb128 .Ltmp330-.Ltmp329
	.uleb128 .Ltmp333-.Lfunc_begin45
	.byte	0
	.uleb128 .Ltmp330-.Lfunc_begin45
	.uleb128 .Ltmp331-.Ltmp330
	.byte	0
	.byte	0
	.uleb128 .Ltmp331-.Lfunc_begin45
	.uleb128 .Ltmp332-.Ltmp331
	.uleb128 .Ltmp333-.Lfunc_begin45
	.byte	0
	.uleb128 .Ltmp334-.Lfunc_begin45
	.uleb128 .Ltmp335-.Ltmp334
	.uleb128 .Ltmp336-.Lfunc_begin45
	.byte	1
	.uleb128 .Ltmp335-.Lfunc_begin45
	.uleb128 .Lfunc_end45-.Ltmp335
	.byte	0
	.byte	0
.Lcst_end21:
	.byte	127
	.byte	0
	.p2align	2, 0x0
.Lttbase16:
	.byte	0
	.p2align	2, 0x0

	.section	.text._RNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE8new_fifoBZ_,"ax",@progbits
	.prefalign	4, .Lfunc_end46, nop
	.type	_RNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE8new_fifoBZ_,@function
_RNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE8new_fifoBZ_:
.Lfunc_begin46:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception22
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r15
	pushq	%r14
	pushq	%r13
	pushq	%r12
	pushq	%rbx
	andq	$-128, %rsp
	subq	$640, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	malloc@GOTPCREL(%rip), %r12
	movq	%rdi, %rbx
	movl	$1024, %edi
	movl	$1024, %r13d
	callq	*%r12
	testq	%rax, %rax
	je	.LBB46_32
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rsi
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rdx
	movq	$-1, %rcx
	movabsq	$9223372036854775807, %r15
	movq	%rax, %r14
	incq	%rsi
	cmoveq	%rcx, %rsi
	addq	%r13, %rdx
	cmovbq	%rcx, %rdx
	addq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %r13
	movq	%rsi, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%rdx, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmovoq	%r15, %r13
	movq	%r13, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %r13
	jle	.LBB46_3
	movq	%r13, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB46_3:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB46_9
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB46_3
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdx
	lock		incq	(%rax)
	lock		addq	$1024, (%rcx)
	movl	$1024, %ecx
	lock		xaddq	%rcx, (%rdx)
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	movq	(%rdx), %rax
	addq	$1024, %rcx
	cmovoq	%r15, %rcx
	.p2align	4
.LBB46_6:
	cmpq	%rax, %rcx
	jle	.LBB46_8
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB46_6
.LBB46_8:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB46_9:
	movl	$16, %edi
	callq	*%r12
	testq	%rax, %rax
	je	.LBB46_33
	movq	%rax, %rcx
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	$-1, %rdx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	addq	$16, %rax
	cmovbq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	addq	$16, %rax
	cmovoq	%r15, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jle	.LBB46_12
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB46_12:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB46_18
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB46_12
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	$16, (%rdx)
	movl	$16, %edx
	lock		xaddq	%rdx, (%rsi)
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rsi
	movq	(%rsi), %rax
	addq	$16, %rdx
	cmovoq	%r15, %rdx
	.p2align	4
.LBB46_15:
	cmpq	%rax, %rdx
	jle	.LBB46_17
	lock		cmpxchgq	%rdx, (%rsi)
	jne	.LBB46_15
.LBB46_17:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB46_18:
	movq	posix_memalign@GOTPCREL(%rip), %rax
	leaq	120(%rsp), %rdi
	movl	$128, %esi
	movl	$384, %edx
	movq	%r14, (%rcx)
	movq	$1, 128(%rsp)
	movq	$1, 136(%rsp)
	movq	$64, 8(%rcx)
	movq	%rcx, 256(%rsp)
	vxorps	%xmm0, %xmm0, %xmm0
	vmovaps	%xmm0, 384(%rsp)
	movq	$0, 120(%rsp)
	callq	*%rax
	testl	%eax, %eax
	jne	.LBB46_29
	movq	120(%rsp), %rcx
	testq	%rcx, %rcx
	je	.LBB46_29
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rsi
	movq	$-1, %rdx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movl	$384, %eax
	addq	%rax, %rsi
	cmovbq	%rdx, %rsi
	addq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	%rsi, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmovoq	%r15, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jle	.LBB46_22
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB46_22:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB46_28
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB46_22
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	$384, (%rdx)
	movl	$384, %edx
	lock		xaddq	%rdx, (%rsi)
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rsi
	movq	(%rsi), %rax
	addq	$384, %rdx
	cmovoq	%r15, %rdx
	.p2align	4
.LBB46_25:
	cmpq	%rax, %rdx
	jle	.LBB46_27
	lock		cmpxchgq	%rdx, (%rsi)
	jne	.LBB46_25
.LBB46_27:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB46_28:
	vmovaps	448(%rsp), %zmm0
	vmovaps	%zmm0, 320(%rcx)
	vmovaps	384(%rsp), %zmm0
	vmovaps	%zmm0, 256(%rcx)
	vmovaps	128(%rsp), %zmm0
	vmovaps	192(%rsp), %zmm1
	vmovaps	256(%rsp), %zmm2
	vmovaps	320(%rsp), %zmm3
	vmovaps	%zmm3, 192(%rcx)
	vmovaps	%zmm2, 128(%rcx)
	vmovaps	%zmm1, 64(%rcx)
	vmovaps	%zmm0, (%rcx)
	movq	%rcx, (%rbx)
	movq	%r14, 8(%rbx)
	movq	$64, 16(%rbx)
	movb	$0, 24(%rbx)
	leaq	-40(%rbp), %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	vzeroupper
	retq
.LBB46_32:
	.cfi_def_cfa %rbp, 16
	movq	_RNvNtCsc70TAahYccp_5alloc7raw_vec12handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$1024, %esi
	callq	*%rax
.LBB46_33:
	movq	_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$16, %esi
	callq	*%rax
.LBB46_29:
.Ltmp337:
	movq	_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$128, %edi
	movl	$384, %esi
	callq	*%rax
.Ltmp338:
	ud2
.LBB46_31:
.Ltmp339:
	leaq	128(%rsp), %rdi
	movq	%rax, %rbx
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc4sync8ArcInnerINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEEEB35_
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end46:
	.size	_RNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE8new_fifoBZ_, .Lfunc_end46-_RNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE8new_fifoBZ_
	.cfi_endproc
	.section	.gcc_except_table._RNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE8new_fifoBZ_,"a",@progbits
	.p2align	2, 0x0
GCC_except_table46:
.Lexception22:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end22-.Lcst_begin22
.Lcst_begin22:
	.uleb128 .Lfunc_begin46-.Lfunc_begin46
	.uleb128 .Ltmp337-.Lfunc_begin46
	.byte	0
	.byte	0
	.uleb128 .Ltmp337-.Lfunc_begin46
	.uleb128 .Ltmp338-.Ltmp337
	.uleb128 .Ltmp339-.Lfunc_begin46
	.byte	0
	.uleb128 .Ltmp338-.Lfunc_begin46
	.uleb128 .Lfunc_end46-.Ltmp338
	.byte	0
	.byte	0
.Lcst_end22:
	.p2align	2, 0x0

	.section	.text._RNvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_8Registry26notify_worker_latch_is_set,"ax",@progbits
	.globl	_RNvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_8Registry26notify_worker_latch_is_set
	.prefalign	4, .Lfunc_end47, nop
	.type	_RNvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_8Registry26notify_worker_latch_is_set,@function
_RNvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_8Registry26notify_worker_latch_is_set:
.Lfunc_begin47:
	.cfi_startproc
	addq	$344, %rdi
	jmp	_RNvMNtCs6a8jV7kq6PJ_10rayon_core5sleepNtB2_5Sleep20wake_specific_thread.llvm.7294274987384275483
.Lfunc_end47:
	.size	_RNvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_8Registry26notify_worker_latch_is_set, .Lfunc_end47-_RNvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_8Registry26notify_worker_latch_is_set
	.cfi_endproc

	.section	.text._RNvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_8Registry6inject,"ax",@progbits
	.globl	_RNvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_8Registry6inject
	.prefalign	4, .Lfunc_end48, nop
	.type	_RNvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_8Registry6inject,@function
_RNvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_8Registry6inject:
.Lfunc_begin48:
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
	movq	(%rdi), %rax
	movq	128(%rdi), %r14
	movq	%rdi, %rbx
	movabsq	$4294967296, %r15
	xorq	%rax, %r14
	callq	_RNvMsg_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_8InjectorNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE4pushB11_.llvm.7294274987384275483
	lock		orl	$0, -64(%rsp)
	.p2align	4
.LBB48_1:
	movq	368(%rbx), %rax
	testq	%r15, %rax
	jne	.LBB48_2
	leaq	(%rax,%r15), %rcx
	lock		cmpxchgq	%rcx, 368(%rbx)
	jne	.LBB48_1
	movq	%rcx, %rax
	andq	$65535, %rax
	jne	.LBB48_5
	jmp	.LBB48_7
.LBB48_2:
	movq	%rax, %rcx
	movq	%rcx, %rax
	andq	$65535, %rax
	je	.LBB48_7
.LBB48_5:
	cmpq	$2, %r14
	jae	.LBB48_8
	shrl	$16, %ecx
	cmpl	%eax, %ecx
	je	.LBB48_8
.LBB48_7:
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	retq
.LBB48_8:
	.cfi_def_cfa_offset 32
	addq	$344, %rbx
	movl	$1, %esi
	movq	%rbx, %rdi
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	jmpq	*_RNvMNtCs6a8jV7kq6PJ_10rayon_core5sleepNtB2_5Sleep16wake_any_threads@GOTPCREL(%rip)
.Lfunc_end48:
	.size	_RNvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_8Registry6inject, .Lfunc_end48-_RNvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_8Registry6inject
	.cfi_endproc

	.section	.text.unlikely._RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.prefalign	4, .Lfunc_end49, nop
	.type	_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs6a8jV7kq6PJ_10rayon_core,@function
_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin49:
	.cfi_startproc
	pushq	%r15
	.cfi_def_cfa_offset 16
	pushq	%r14
	.cfi_def_cfa_offset 24
	pushq	%rbx
	.cfi_def_cfa_offset 32
	subq	$16, %rsp
	.cfi_def_cfa_offset 48
	.cfi_offset %rbx, -32
	.cfi_offset %r14, -24
	.cfi_offset %r15, -16
	movq	%rdx, %r9
	movq	%r8, %rax
	mulq	%rcx
	movabsq	$9223372036854775800, %rcx
	movl	$1, %r15d
	movq	%rdi, %rbx
	seto	%dl
	movq	%rax, %r14
	cmpq	%rcx, %r14
	seta	%cl
	orb	%dl, %cl
	je	.LBB49_2
	movl	$8, %eax
	xorl	%r14d, %r14d
	jmp	.LBB49_22
.LBB49_2:
	testq	%rsi, %rsi
	je	.LBB49_4
	movq	_RNvXCs2911K1BwAFx_18purrdf_alloc_probeNtB2_17CountingAllocatorNtNtNtCs2k2z8Zem4rB_4core5alloc6global11GlobalAlloc7realloc@GOTPCREL(%rip), %rax
	imulq	%rsi, %r8
	leaq	_RNvCs3dQcT9XZUpl_29qualification_454_native_cost6GLOBAL.llvm.4264773384553001850(%rip), %rdi
	movl	$8, %edx
	movq	%r9, %rsi
	movq	%r8, %rcx
	movq	%r14, %r8
	callq	*%rax
	movq	%rax, %rcx
.LBB49_20:
	testq	%rcx, %rcx
	jne	.LBB49_6
	jmp	.LBB49_21
.LBB49_4:
	testq	%r14, %r14
	je	.LBB49_5
	cmpq	$7, %r14
	jbe	.LBB49_8
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
	movq	%rax, %rcx
	testq	%rcx, %rcx
	jne	.LBB49_12
	jmp	.LBB49_21
.LBB49_5:
	movl	$8, %ecx
.LBB49_6:
	movq	%rcx, 8(%rbx)
	movl	$16, %eax
	xorl	%r15d, %r15d
	jmp	.LBB49_22
.LBB49_8:
	movq	posix_memalign@GOTPCREL(%rip), %rax
	leaq	8(%rsp), %rdi
	movl	$8, %esi
	movq	%r14, %rdx
	movq	$0, 8(%rsp)
	callq	*%rax
	testl	%eax, %eax
	je	.LBB49_9
.LBB49_21:
	movl	$16, %eax
	movq	$8, 8(%rbx)
.LBB49_22:
	movq	%r14, (%rbx,%rax)
	movq	%r15, (%rbx)
	addq	$16, %rsp
	.cfi_def_cfa_offset 32
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	retq
.LBB49_9:
	.cfi_def_cfa_offset 48
	movq	8(%rsp), %rcx
	testq	%rcx, %rcx
	je	.LBB49_21
.LBB49_12:
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rdi
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %r8
	movabsq	$-9223372036854775808, %rdx
	movq	$-1, %r9
	leaq	(%r14,%rax), %rsi
	sarq	$63, %rsi
	xorq	%rdx, %rsi
	addq	%r14, %rax
	cmovoq	%rsi, %rax
	incq	%rdi
	cmoveq	%r9, %rdi
	addq	%r14, %r8
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmovbq	%r9, %r8
	movq	%rdi, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%r8, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jle	.LBB49_14
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB49_14:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB49_20
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB49_14
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rsi
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdi
	lock		incq	(%rax)
	lock		addq	%r14, (%rsi)
	movq	%r14, %rsi
	lock		xaddq	%rsi, (%rdi)
	leaq	(%rsi,%r14), %rax
	sarq	$63, %rax
	xorq	%rdx, %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	addq	%r14, %rsi
	cmovoq	%rax, %rsi
	movq	(%rdx), %rax
	.p2align	4
.LBB49_17:
	cmpq	%rax, %rsi
	jle	.LBB49_19
	lock		cmpxchgq	%rsi, (%rdx)
	jne	.LBB49_17
.LBB49_19:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jmp	.LBB49_20
.Lfunc_end49:
	.size	_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs6a8jV7kq6PJ_10rayon_core, .Lfunc_end49-_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc

	.section	.text._RNvMs8_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_7StealerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE5stealB10_,"ax",@progbits
	.prefalign	4, .Lfunc_end50, nop
	.type	_RNvMs8_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_7StealerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE5stealB10_,@function
_RNvMs8_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_7StealerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE5stealB10_:
.Lfunc_begin50:
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
	movq	(%rsi), %r15
	movq	_RNvNCNKNvNtCs18aJq3QiqAb_15crossbeam_epoch7default6HANDLE0023___RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rax
	movq	%rdi, %rbx
	movq	256(%r15), %r14
	cmpb	$1, %fs:8(%rax)
	jne	.LBB50_2
	addq	%fs:0, %rax
.LBB50_3:
	movq	(%rax), %rax
	movq	2072(%rax), %rcx
.LBB50_4:
	testq	%rcx, %rcx
	je	.LBB50_10
	lock		orl	$0, -64(%rsp)
.LBB50_10:
	callq	_RINvNtCs18aJq3QiqAb_15crossbeam_epoch7default11with_handleNCNvB2_3pin0NtNtB4_5guard5GuardECs6a8jV7kq6PJ_10rayon_core
	movq	264(%r15), %rcx
	movq	%rax, %rdi
	subq	%r14, %rcx
	testq	%rcx, %rcx
	jle	.LBB50_11
	movq	128(%r15), %rax
	movq	%rax, %rcx
	andq	$-8, %rcx
	movq	(%rcx), %rsi
	movq	8(%rcx), %rcx
	decq	%rcx
	andq	%r14, %rcx
	shlq	$4, %rcx
	movq	(%rsi,%rcx), %rdx
	movq	8(%rsi,%rcx), %rsi
	movq	128(%r15), %r8
	movl	$2, %ecx
	cmpq	%rax, %r8
	jne	.LBB50_12
	leaq	1(%r14), %r8
	movq	%r14, %rax
	lock		cmpxchgq	%r8, 256(%r15)
	jne	.LBB50_12
	movq	%rdx, 8(%rbx)
	movq	%rsi, 16(%rbx)
	movq	$1, (%rbx)
	testq	%rdi, %rdi
	jne	.LBB50_17
	jmp	.LBB50_19
.LBB50_11:
	xorl	%ecx, %ecx
.LBB50_12:
	movq	%rcx, (%rbx)
	testq	%rdi, %rdi
	je	.LBB50_19
.LBB50_17:
	decq	2072(%rdi)
	jne	.LBB50_19
	movq	$0, 2176(%rdi)
	cmpq	$0, 2080(%rdi)
	je	.LBB50_20
.LBB50_19:
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	retq
.LBB50_2:
	.cfi_def_cfa_offset 32
	movq	%fs:0, %rdi
	addq	_RNvNCNKNvNtCs18aJq3QiqAb_15crossbeam_epoch7default6HANDLE0023___RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rdi
	callq	_RINvMs0_NtNtNtNtCs7jcFBdfocI9_3std3sys12thread_local6native4lazyINtB6_7StorageNtNtCs18aJq3QiqAb_15crossbeam_epoch9collector11LocalHandleuE16get_or_init_slowNvNvNtB1i_7default6HANDLE27___rust_std_internal_init_fnECs6a8jV7kq6PJ_10rayon_core
	testq	%rax, %rax
	jne	.LBB50_3
	movl	_RNvNvNtCs18aJq3QiqAb_15crossbeam_epoch7default9collector9COLLECTOR.llvm.707543514826133848+8(%rip), %eax
	testl	%eax, %eax
	jne	.LBB50_7
.LBB50_8:
	movq	_RNvMs1_NtCs18aJq3QiqAb_15crossbeam_epoch9collectorNtB5_9Collector8register@GOTPCREL(%rip), %rax
	leaq	_RNvNvNtCs18aJq3QiqAb_15crossbeam_epoch7default9collector9COLLECTOR.llvm.707543514826133848(%rip), %rdi
	callq	*%rax
	movq	2080(%rax), %rdx
	movq	2072(%rax), %rcx
	leaq	-1(%rdx), %rsi
	xorq	$1, %rdx
	orq	%rcx, %rdx
	movq	%rsi, 2080(%rax)
	jne	.LBB50_4
	movq	_RNvMs6_NtCs18aJq3QiqAb_15crossbeam_epoch8internalNtB5_5Local8finalize@GOTPCREL(%rip), %rcx
	movq	%rax, %rdi
	callq	*%rcx
	jmp	.LBB50_10
.LBB50_20:
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	jmpq	*_RNvMs6_NtCs18aJq3QiqAb_15crossbeam_epoch8internalNtB5_5Local8finalize@GOTPCREL(%rip)
.LBB50_7:
	.cfi_def_cfa_offset 32
	callq	_RINvMs0_NtNtCs18aJq3QiqAb_15crossbeam_epoch4sync9once_lockINtB6_8OnceLockNtNtBa_9collector9CollectorE10initializeNvMs1_B1b_B19_3newEBa_.llvm.707543514826133848
	jmp	.LBB50_8
.Lfunc_end50:
	.size	_RNvMs8_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_7StealerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE5stealB10_, .Lfunc_end50-_RNvMs8_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_7StealerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE5stealB10_
	.cfi_endproc

	.section	.text._RNvMs8_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThread14take_local_job,"ax",@progbits
	.prefalign	4, .Lfunc_end51, nop
	.type	_RNvMs8_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThread14take_local_job,@function
_RNvMs8_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThread14take_local_job:
.Lfunc_begin51:
	.cfi_startproc
	pushq	%r14
	.cfi_def_cfa_offset 16
	pushq	%rbx
	.cfi_def_cfa_offset 24
	subq	$24, %rsp
	.cfi_def_cfa_offset 48
	.cfi_offset %rbx, -24
	.cfi_offset %r14, -16
	movq	280(%rdi), %rax
	movq	%rdi, %rbx
	movq	264(%rax), %rdi
	movq	256(%rax), %rdx
	movq	%rdi, %rcx
	subq	%rdx, %rcx
	testq	%rcx, %rcx
	jle	.LBB51_13
	cmpb	$1, 304(%rbx)
	jne	.LBB51_5
	leaq	-1(%rdi), %rcx
	movq	%rcx, 264(%rax)
	lock		orl	$0, -64(%rsp)
	movq	280(%rbx), %r9
	movq	%rcx, %r8
	movq	256(%r9), %r10
	subq	%r10, %r8
	js	.LBB51_8
	movq	296(%rbx), %rsi
	movq	288(%rbx), %rdx
	leaq	-1(%rsi), %r11
	andq	%rcx, %r11
	shlq	$4, %r11
	movq	(%rdx,%r11), %rax
	movq	8(%rdx,%r11), %rdx
	cmpq	%r10, %rcx
	jne	.LBB51_9
	movq	%rax, %rsi
	movq	%rcx, %rax
	lock		cmpxchgq	%rdi, 256(%r9)
	movq	%rsi, %rax
	movq	280(%rbx), %rcx
	movq	%rdi, 264(%rcx)
	jne	.LBB51_13
.LBB51_18:
	addq	$24, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	retq
.LBB51_5:
	.cfi_def_cfa_offset 48
	movl	$1, %edx
	lock		xaddq	%rdx, 256(%rax)
	cmpq	%rdi, %rdx
	js	.LBB51_6
	movq	280(%rbx), %rax
	movq	%rdx, 256(%rax)
	jmp	.LBB51_13
.LBB51_8:
	movq	%rdi, 264(%r9)
.LBB51_13:
	addq	$312, %rbx
	movq	%rsp, %r14
	.p2align	4
.LBB51_14:
	movq	%r14, %rdi
	movq	%rbx, %rsi
	callq	_RNvMs8_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_7StealerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE5stealB10_
	movq	(%rsp), %rax
	cmpq	$2, %rax
	je	.LBB51_14
	testq	%rax, %rax
	jne	.LBB51_16
	xorl	%eax, %eax
	addq	$24, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	retq
.LBB51_16:
	.cfi_def_cfa_offset 48
	movq	8(%rsp), %rax
	movq	16(%rsp), %rdx
	addq	$24, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	retq
.LBB51_6:
	.cfi_def_cfa_offset 48
	movq	296(%rbx), %rsi
	movq	288(%rbx), %rdi
	leaq	-1(%rsi), %r8
	andq	%rdx, %r8
	shlq	$4, %r8
	testq	%rsi, %rsi
	movq	(%rdi,%r8), %rax
	movq	8(%rdi,%r8), %rdx
	leaq	3(%rsi), %rdi
	cmovnsq	%rsi, %rdi
	cmpq	$65, %rsi
	jb	.LBB51_18
	sarq	$2, %rdi
	cmpq	%rdi, %rcx
	jg	.LBB51_18
	jmp	.LBB51_11
.LBB51_9:
	leaq	3(%rsi), %rcx
	testq	%rsi, %rsi
	cmovnsq	%rsi, %rcx
	cmpq	$65, %rsi
	jb	.LBB51_18
	sarq	$2, %rcx
	cmpq	%rcx, %r8
	jge	.LBB51_18
.LBB51_11:
	addq	$280, %rbx
	shrq	%rsi
	movq	%rdx, %r14
	movq	%rbx, %rdi
	movq	%rax, %rbx
	callq	_RNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE6resizeBZ_
	movq	%r14, %rdx
	movq	%rbx, %rax
	addq	$24, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end51:
	.size	_RNvMs8_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThread14take_local_job, .Lfunc_end51-_RNvMs8_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThread14take_local_job
	.cfi_endproc

	.section	.text.unlikely._RNvMs8_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThread15wait_until_cold,"ax",@progbits
	.globl	_RNvMs8_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThread15wait_until_cold
	.prefalign	4, .Lfunc_end52, nop
	.type	_RNvMs8_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThread15wait_until_cold,@function
_RNvMs8_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThread15wait_until_cold:
.Lfunc_begin52:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception23
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
	subq	$168, %rsp
	.cfi_def_cfa_offset 224
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	%rdi, 16(%rsp)
	movq	(%rsi), %rax
	cmpq	$3, %rax
	je	.LBB52_238
	movq	16(%rsp), %rax
	movq	%fs:0, %r12
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.20(%rip), %rcx
	movq	%rsi, %rbx
	movq	%rsi, 24(%rsp)
	addq	_RNvNCNKNvNtCs18aJq3QiqAb_15crossbeam_epoch7default6HANDLE0023___RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %r12
	movq	%rcx, 136(%rsp)
	addq	$280, %rax
	movq	%rax, 56(%rsp)
	movq	%r12, 40(%rsp)
	.p2align	4
.LBB52_2:
.Ltmp340:
	movq	16(%rsp), %rdi
	callq	_RNvMs8_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThread14take_local_job
.Ltmp341:
	movq	%rax, %r15
	testq	%rax, %rax
	je	.LBB52_5
	movq	%rdx, %r14
	jmp	.LBB52_236
	.p2align	4
.LBB52_5:
	movq	16(%rsp), %rax
	movq	256(%rax), %rcx
	movq	272(%rax), %rax
	movq	%rcx, 160(%rsp)
	lock		addq	$65536, 496(%rax)
	movq	(%rbx), %rax
	cmpq	$3, %rax
	je	.LBB52_239
	movq	$-1, 64(%rsp)
	movl	$0, 36(%rsp)
.LBB52_7:
	movq	56(%rsp), %rax
	movq	(%rax), %rdx
	movq	264(%rdx), %rcx
	movq	256(%rdx), %rsi
	movq	%rcx, %rax
	subq	%rsi, %rax
	testq	%rax, %rax
	jle	.LBB52_16
	movq	16(%rsp), %rsi
	cmpb	$1, 304(%rsi)
	jne	.LBB52_12
	leaq	-1(%rcx), %rax
	movq	%rax, 264(%rdx)
	lock		orl	$0, -64(%rsp)
	movq	56(%rsp), %rdx
	movq	(%rdx), %rdi
	movq	%rax, %rdx
	movq	256(%rdi), %r8
	subq	%r8, %rdx
	js	.LBB52_14
	movq	16(%rsp), %rsi
	movq	288(%rsi), %r9
	movq	296(%rsi), %rsi
	leaq	-1(%rsi), %r10
	andq	%rax, %r10
	shlq	$4, %r10
	movq	(%r9,%r10), %r15
	movq	8(%r9,%r10), %r14
	cmpq	%r8, %rax
	jne	.LBB52_226
	lock		cmpxchgq	%rcx, 256(%rdi)
	movq	56(%rsp), %rax
	movq	(%rax), %rax
	movq	%rcx, 264(%rax)
	jne	.LBB52_16
	jmp	.LBB52_229
	.p2align	4
.LBB52_12:
	movl	$1, %edi
	lock		xaddq	%rdi, 256(%rdx)
	cmpq	%rcx, %rdi
	js	.LBB52_222
	movq	56(%rsp), %rax
	movq	(%rax), %rax
	movq	%rdi, 256(%rax)
	jmp	.LBB52_16
.LBB52_14:
	movq	%rcx, 264(%rdi)
	jmp	.LBB52_16
	.p2align	4
.LBB52_15:
	testq	%rbx, %rbx
	jle	.LBB52_35
.LBB52_16:
	movq	16(%rsp), %rax
	movq	312(%rax), %rbp
	movq	_RNvNCNKNvNtCs18aJq3QiqAb_15crossbeam_epoch7default6HANDLE0023___RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rax
	movq	256(%rbp), %r13
	cmpb	$1, %fs:8(%rax)
	movq	%r12, %rax
	jne	.LBB52_28
.LBB52_17:
	movq	(%rax), %rax
	movq	2072(%rax), %rcx
.LBB52_18:
	testq	%rcx, %rcx
	je	.LBB52_20
	lock		orl	$0, -64(%rsp)
.LBB52_20:
.Ltmp354:
	callq	_RINvNtCs18aJq3QiqAb_15crossbeam_epoch7default11with_handleNCNvB2_3pin0NtNtB4_5guard5GuardECs6a8jV7kq6PJ_10rayon_core
.Ltmp355:
	movq	264(%rbp), %rbx
	movq	%rax, %rdi
	subq	%r13, %rbx
	testq	%rbx, %rbx
	jle	.LBB52_24
	movq	128(%rbp), %rax
	movq	%rax, %rcx
	andq	$-8, %rcx
	movq	(%rcx), %rdx
	movq	8(%rcx), %rcx
	decq	%rcx
	andq	%r13, %rcx
	shlq	$4, %rcx
	movq	(%rdx,%rcx), %r15
	movq	8(%rdx,%rcx), %r14
	movq	128(%rbp), %rdx
	cmpq	%rax, %rdx
	jne	.LBB52_24
	leaq	1(%r13), %rcx
	movq	%r13, %rax
	lock		cmpxchgq	%rcx, 256(%rbp)
	je	.LBB52_180
.LBB52_24:
	testq	%rdi, %rdi
	je	.LBB52_15
	decq	2072(%rdi)
	jne	.LBB52_15
	movq	$0, 2176(%rdi)
	cmpq	$0, 2080(%rdi)
	jne	.LBB52_15
.Ltmp358:
	movq	_RNvMs6_NtCs18aJq3QiqAb_15crossbeam_epoch8internalNtB5_5Local8finalize@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp359:
	jmp	.LBB52_15
.LBB52_28:
.Ltmp346:
	movq	%r12, %rdi
	callq	_RINvMs0_NtNtNtNtCs7jcFBdfocI9_3std3sys12thread_local6native4lazyINtB6_7StorageNtNtCs18aJq3QiqAb_15crossbeam_epoch9collector11LocalHandleuE16get_or_init_slowNvNvNtB1i_7default6HANDLE27___rust_std_internal_init_fnECs6a8jV7kq6PJ_10rayon_core
.Ltmp347:
	testq	%rax, %rax
	jne	.LBB52_17
	movl	_RNvNvNtCs18aJq3QiqAb_15crossbeam_epoch7default9collector9COLLECTOR.llvm.707543514826133848+8(%rip), %eax
	testl	%eax, %eax
	jne	.LBB52_34
.LBB52_31:
.Ltmp350:
	movq	_RNvMs1_NtCs18aJq3QiqAb_15crossbeam_epoch9collectorNtB5_9Collector8register@GOTPCREL(%rip), %rax
	leaq	_RNvNvNtCs18aJq3QiqAb_15crossbeam_epoch7default9collector9COLLECTOR.llvm.707543514826133848(%rip), %rdi
	callq	*%rax
.Ltmp351:
	movq	2080(%rax), %rdx
	movq	2072(%rax), %rcx
	leaq	-1(%rdx), %rsi
	xorq	$1, %rdx
	orq	%rcx, %rdx
	movq	%rsi, 2080(%rax)
	jne	.LBB52_18
.Ltmp352:
	movq	_RNvMs6_NtCs18aJq3QiqAb_15crossbeam_epoch8internalNtB5_5Local8finalize@GOTPCREL(%rip), %rbx
	movq	%rax, %rdi
	callq	*%rbx
.Ltmp353:
	jmp	.LBB52_20
.LBB52_34:
.Ltmp348:
	callq	_RINvMs0_NtNtCs18aJq3QiqAb_15crossbeam_epoch4sync9once_lockINtB6_8OnceLockNtNtBa_9collector9CollectorE10initializeNvMs1_B1b_B19_3newEBa_.llvm.707543514826133848
.Ltmp349:
	jmp	.LBB52_31
	.p2align	4
.LBB52_35:
	movq	16(%rsp), %rax
	movq	272(%rax), %r14
	movq	520(%r14), %r15
	cmpq	$2, %r15
	jb	.LBB52_107
	movq	512(%r14), %rax
	movq	%r15, 48(%rsp)
	movq	%rax, 152(%rsp)
.LBB52_37:
	movq	16(%rsp), %rsi
	movq	$0, 128(%rsp)
	movq	264(%rsi), %rax
	movq	%rax, %rcx
	shrq	$12, %rcx
	xorq	%rax, %rcx
	movq	%rcx, %rdx
	shlq	$25, %rdx
	xorq	%rcx, %rdx
	movabsq	$2685821657736338717, %rcx
	movq	%rdx, %rax
	shrq	$27, %rax
	xorq	%rdx, %rax
	xorl	%edx, %edx
	movq	%rax, 264(%rsi)
	imulq	%rcx, %rax
	divq	%r15
	movq	%rdx, %r13
	movq	%rdx, 144(%rsp)
	jmp	.LBB52_40
	.p2align	4
.LBB52_38:
	movq	40(%rsp), %r12
.LBB52_39:
	incq	%r13
	cmpq	%r15, %r13
	je	.LBB52_70
.LBB52_40:
	movq	16(%rsp), %rax
	cmpq	256(%rax), %r13
	je	.LBB52_39
	cmpq	%r15, %r13
	jae	.LBB52_243
	movq	152(%rsp), %rcx
	leaq	(%r13,%r13,2), %rax
	shlq	$4, %rax
	movq	(%rcx,%rax), %r12
	movq	_RNvNCNKNvNtCs18aJq3QiqAb_15crossbeam_epoch7default6HANDLE0023___RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rax
	movq	256(%r12), %rbp
	cmpb	$1, %fs:8(%rax)
	movq	40(%rsp), %rax
	jne	.LBB52_63
.LBB52_43:
	movq	(%rax), %rax
	movq	2072(%rax), %rcx
.LBB52_44:
	testq	%rcx, %rcx
	je	.LBB52_46
	lock		orl	$0, -64(%rsp)
.LBB52_46:
.Ltmp369:
	callq	_RINvNtCs18aJq3QiqAb_15crossbeam_epoch7default11with_handleNCNvB2_3pin0NtNtB4_5guard5GuardECs6a8jV7kq6PJ_10rayon_core
.Ltmp370:
	movq	264(%r12), %rcx
	movq	%rax, %rdi
	subq	%rbp, %rcx
	testq	%rcx, %rcx
	jle	.LBB52_54
	movq	128(%r12), %rax
	movl	$2, %ebx
	movq	%rax, %rcx
	andq	$-8, %rcx
	movq	(%rcx), %rdx
	movq	8(%rcx), %rcx
	decq	%rcx
	andq	%rbp, %rcx
	shlq	$4, %rcx
	movq	(%rdx,%rcx), %r15
	movq	8(%rdx,%rcx), %r14
	movq	128(%r12), %rdx
	cmpq	%rax, %rdx
	jne	.LBB52_55
	leaq	1(%rbp), %rcx
	movq	%rbp, %rax
	lock		cmpxchgq	%rcx, 256(%r12)
	jne	.LBB52_55
	movq	40(%rsp), %r12
	testq	%rdi, %rdi
	je	.LBB52_225
	decq	2072(%rdi)
	movq	24(%rsp), %rbx
	jne	.LBB52_169
	movq	$0, 2176(%rdi)
	cmpq	$0, 2080(%rdi)
	jne	.LBB52_169
	movq	%r15, 96(%rsp)
	movq	48(%rsp), %r15
	movl	$1, %ebx
	movq	%r14, 88(%rsp)
	jmp	.LBB52_58
.LBB52_54:
	xorl	%ebx, %ebx
.LBB52_55:
	testq	%rdi, %rdi
	je	.LBB52_59
	decq	2072(%rdi)
	movq	48(%rsp), %r15
	jne	.LBB52_60
	movq	$0, 2176(%rdi)
	cmpq	$0, 2080(%rdi)
	jne	.LBB52_60
.LBB52_58:
.Ltmp371:
	movq	_RNvMs6_NtCs18aJq3QiqAb_15crossbeam_epoch8internalNtB5_5Local8finalize@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp372:
	jmp	.LBB52_60
.LBB52_59:
	movq	48(%rsp), %r15
.LBB52_60:
	testq	%rbx, %rbx
	je	.LBB52_38
	movq	40(%rsp), %r12
	cmpl	$2, %ebx
	jne	.LBB52_221
	movb	$1, %al
	movq	%rax, 128(%rsp)
	jmp	.LBB52_39
.LBB52_63:
.Ltmp361:
	movq	40(%rsp), %rdi
	callq	_RINvMs0_NtNtNtNtCs7jcFBdfocI9_3std3sys12thread_local6native4lazyINtB6_7StorageNtNtCs18aJq3QiqAb_15crossbeam_epoch9collector11LocalHandleuE16get_or_init_slowNvNvNtB1i_7default6HANDLE27___rust_std_internal_init_fnECs6a8jV7kq6PJ_10rayon_core
.Ltmp362:
	testq	%rax, %rax
	jne	.LBB52_43
	movl	_RNvNvNtCs18aJq3QiqAb_15crossbeam_epoch7default9collector9COLLECTOR.llvm.707543514826133848+8(%rip), %eax
	testl	%eax, %eax
	jne	.LBB52_69
.LBB52_66:
.Ltmp365:
	movq	_RNvMs1_NtCs18aJq3QiqAb_15crossbeam_epoch9collectorNtB5_9Collector8register@GOTPCREL(%rip), %rax
	leaq	_RNvNvNtCs18aJq3QiqAb_15crossbeam_epoch7default9collector9COLLECTOR.llvm.707543514826133848(%rip), %rdi
	callq	*%rax
.Ltmp366:
	movq	2080(%rax), %rdx
	movq	2072(%rax), %rcx
	leaq	-1(%rdx), %rsi
	xorq	$1, %rdx
	orq	%rcx, %rdx
	movq	%rsi, 2080(%rax)
	jne	.LBB52_44
.Ltmp367:
	movq	_RNvMs6_NtCs18aJq3QiqAb_15crossbeam_epoch8internalNtB5_5Local8finalize@GOTPCREL(%rip), %rbx
	movq	%rax, %rdi
	callq	*%rbx
.Ltmp368:
	jmp	.LBB52_46
.LBB52_69:
.Ltmp363:
	callq	_RINvMs0_NtNtCs18aJq3QiqAb_15crossbeam_epoch4sync9once_lockINtB6_8OnceLockNtNtBa_9collector9CollectorE10initializeNvMs1_B1b_B19_3newEBa_.llvm.707543514826133848
.Ltmp364:
	jmp	.LBB52_66
	.p2align	4
.LBB52_70:
	cmpq	$0, 144(%rsp)
	je	.LBB52_104
	movq	152(%rsp), %rbp
	xorl	%r13d, %r13d
	jmp	.LBB52_74
	.p2align	4
.LBB52_72:
	movq	40(%rsp), %r12
.LBB52_73:
	incq	%r13
	addq	$48, %rbp
	cmpq	%r13, 144(%rsp)
	je	.LBB52_104
.LBB52_74:
	movq	16(%rsp), %rax
	cmpq	256(%rax), %r13
	je	.LBB52_73
	cmpq	%r15, %r13
	jae	.LBB52_243
	movq	(%rbp), %r12
	movq	_RNvNCNKNvNtCs18aJq3QiqAb_15crossbeam_epoch7default6HANDLE0023___RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rax
	movq	256(%r12), %rbx
	cmpb	$1, %fs:8(%rax)
	movq	40(%rsp), %rax
	jne	.LBB52_97
.LBB52_77:
	movq	(%rax), %rax
	movq	2072(%rax), %rcx
.LBB52_78:
	testq	%rcx, %rcx
	je	.LBB52_80
	lock		orl	$0, -64(%rsp)
.LBB52_80:
.Ltmp382:
	callq	_RINvNtCs18aJq3QiqAb_15crossbeam_epoch7default11with_handleNCNvB2_3pin0NtNtB4_5guard5GuardECs6a8jV7kq6PJ_10rayon_core
.Ltmp383:
	movq	264(%r12), %rcx
	movq	%rax, %rdi
	subq	%rbx, %rcx
	testq	%rcx, %rcx
	jle	.LBB52_88
	movq	128(%r12), %rsi
	movq	%rbx, %rax
	movq	%rsi, %rcx
	andq	$-8, %rcx
	movq	(%rcx), %rdx
	movq	8(%rcx), %rcx
	decq	%rcx
	andq	%rbx, %rcx
	movl	$2, %ebx
	shlq	$4, %rcx
	movq	(%rdx,%rcx), %r15
	movq	8(%rdx,%rcx), %r14
	movq	128(%r12), %rdx
	cmpq	%rsi, %rdx
	jne	.LBB52_89
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, 256(%r12)
	jne	.LBB52_89
	movq	40(%rsp), %r12
	testq	%rdi, %rdi
	je	.LBB52_233
	decq	2072(%rdi)
	movq	24(%rsp), %rbx
	jne	.LBB52_234
	movq	$0, 2176(%rdi)
	cmpq	$0, 2080(%rdi)
	jne	.LBB52_234
	movq	%r15, 80(%rsp)
	movq	48(%rsp), %r15
	movl	$1, %ebx
	movq	%r14, 72(%rsp)
	jmp	.LBB52_92
.LBB52_88:
	xorl	%ebx, %ebx
.LBB52_89:
	testq	%rdi, %rdi
	je	.LBB52_93
	decq	2072(%rdi)
	movq	48(%rsp), %r15
	jne	.LBB52_94
	movq	$0, 2176(%rdi)
	cmpq	$0, 2080(%rdi)
	jne	.LBB52_94
.LBB52_92:
.Ltmp384:
	movq	_RNvMs6_NtCs18aJq3QiqAb_15crossbeam_epoch8internalNtB5_5Local8finalize@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp385:
	jmp	.LBB52_94
.LBB52_93:
	movq	48(%rsp), %r15
.LBB52_94:
	testq	%rbx, %rbx
	je	.LBB52_72
	movq	40(%rsp), %r12
	cmpl	$2, %ebx
	jne	.LBB52_231
	movb	$1, %al
	movq	%rax, 128(%rsp)
	jmp	.LBB52_73
.LBB52_97:
.Ltmp374:
	movq	40(%rsp), %rdi
	callq	_RINvMs0_NtNtNtNtCs7jcFBdfocI9_3std3sys12thread_local6native4lazyINtB6_7StorageNtNtCs18aJq3QiqAb_15crossbeam_epoch9collector11LocalHandleuE16get_or_init_slowNvNvNtB1i_7default6HANDLE27___rust_std_internal_init_fnECs6a8jV7kq6PJ_10rayon_core
.Ltmp375:
	testq	%rax, %rax
	jne	.LBB52_77
	movl	_RNvNvNtCs18aJq3QiqAb_15crossbeam_epoch7default9collector9COLLECTOR.llvm.707543514826133848+8(%rip), %eax
	testl	%eax, %eax
	jne	.LBB52_103
.LBB52_100:
.Ltmp378:
	movq	_RNvMs1_NtCs18aJq3QiqAb_15crossbeam_epoch9collectorNtB5_9Collector8register@GOTPCREL(%rip), %rax
	leaq	_RNvNvNtCs18aJq3QiqAb_15crossbeam_epoch7default9collector9COLLECTOR.llvm.707543514826133848(%rip), %rdi
	callq	*%rax
.Ltmp379:
	movq	2080(%rax), %rdx
	movq	2072(%rax), %rcx
	leaq	-1(%rdx), %rsi
	xorq	$1, %rdx
	orq	%rcx, %rdx
	movq	%rsi, 2080(%rax)
	jne	.LBB52_78
.Ltmp380:
	movq	_RNvMs6_NtCs18aJq3QiqAb_15crossbeam_epoch8internalNtB5_5Local8finalize@GOTPCREL(%rip), %r14
	movq	%rax, %rdi
	callq	*%r14
.Ltmp381:
	jmp	.LBB52_80
.LBB52_103:
.Ltmp376:
	callq	_RINvMs0_NtNtCs18aJq3QiqAb_15crossbeam_epoch4sync9once_lockINtB6_8OnceLockNtNtBa_9collector9CollectorE10initializeNvMs1_B1b_B19_3newEBa_.llvm.707543514826133848
.Ltmp377:
	jmp	.LBB52_100
.LBB52_104:
	testb	$1, 128(%rsp)
	jne	.LBB52_37
	movq	16(%rsp), %rax
	movq	272(%rax), %r14
	jmp	.LBB52_107
	.p2align	4
.LBB52_106:
	lock		cmpxchgq	%r15, 128(%r14)
	je	.LBB52_170
.LBB52_107:
	movq	128(%r14), %rax
	movq	136(%r14), %r13
	movq	%rax, %rcx
	shrq	%rcx
	movl	%ecx, %ebp
	andl	$63, %ebp
	cmpq	$63, %rbp
	jne	.LBB52_115
	xorl	%ebx, %ebx
	jmp	.LBB52_111
	.p2align	4
.LBB52_109:
	incl	%ebx
.LBB52_110:
	movq	128(%r14), %rax
	movq	136(%r14), %r13
	movq	%rax, %rcx
	shrq	%rcx
	movl	%ecx, %ebp
	andl	$63, %ebp
	cmpq	$63, %rbp
	jne	.LBB52_115
.LBB52_111:
	cmpl	$6, %ebx
	ja	.LBB52_114
	movl	$1, %eax
	.p2align	4
.LBB52_113:
	shrxl	%ebx, %eax, %ecx
	incl	%eax
	pause
	testl	%ecx, %ecx
	je	.LBB52_113
	jmp	.LBB52_109
	.p2align	4
.LBB52_114:
	movq	sched_yield@GOTPCREL(%rip), %rax
	callq	*%rax
	cmpl	$11, %ebx
	jae	.LBB52_110
	jmp	.LBB52_109
	.p2align	4
.LBB52_115:
	leaq	2(%rax), %r15
	testb	$1, %al
	jne	.LBB52_106
	lock		orl	$0, -64(%rsp)
	movq	256(%r14), %rdx
	movq	24(%rsp), %rbx
	movq	%rdx, %rsi
	shrq	%rsi
	cmpq	%rsi, %rcx
	je	.LBB52_118
	xorq	%rax, %rdx
	xorl	%ecx, %ecx
	cmpq	$127, %rdx
	seta	%cl
	orq	%rcx, %r15
	jmp	.LBB52_106
.LBB52_118:
	movl	36(%rsp), %ebp
	cmpl	$32, %ebp
	jae	.LBB52_120
	movq	sched_yield@GOTPCREL(%rip), %rax
	callq	*%rax
	incl	%ebp
	movl	%ebp, 36(%rsp)
	jmp	.LBB52_142
.LBB52_120:
	movq	16(%rsp), %rax
	movq	272(%rax), %r15
	je	.LBB52_154
	movl	$1, %ecx
	xorl	%eax, %eax
	lock		cmpxchgq	%rcx, (%rbx)
	jne	.LBB52_142
	movq	488(%r15), %rax
	movq	160(%rsp), %rdx
	cmpq	%rax, %rdx
	jae	.LBB52_242
	movq	480(%r15), %rcx
	shlq	$7, %rdx
	movl	$1, %esi
	xorl	%eax, %eax
	lock		cmpxchgl	%esi, (%rcx,%rdx)
	leaq	(%rcx,%rdx), %r14
	jne	.LBB52_159
.LBB52_124:
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count18GLOBAL_PANIC_COUNT@GOTPCREL(%rip), %rax
	movq	(%rax), %rax
	shlq	%rax
	testq	%rax, %rax
	jne	.LBB52_160
	xorl	%ebp, %ebp
	movzbl	4(%r14), %eax
	testb	%al, %al
	jne	.LBB52_241
.LBB52_126:
	movl	$0, 36(%rsp)
	movl	$1, %eax
	movl	$2, %ecx
	lock		cmpxchgq	%rcx, (%rbx)
	jne	.LBB52_139
	.p2align	4
.LBB52_127:
	movq	496(%r15), %rax
	movq	%rax, %rcx
	shrq	$32, %rcx
	cmpq	64(%rsp), %rcx
	jne	.LBB52_137
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, 496(%r15)
	jne	.LBB52_127
	lock		orl	$0, -64(%rsp)
	movq	16(%rsp), %rax
	movq	312(%rax), %rax
	movq	256(%rax), %rcx
	lock		orl	$0, -64(%rsp)
	movq	264(%rax), %rax
	subq	%rcx, %rax
	testq	%rax, %rax
	jg	.LBB52_131
	movq	16(%rsp), %rax
	movq	272(%rax), %rax
	movq	128(%rax), %rcx
	movq	256(%rax), %rax
	xorq	%rcx, %rax
	cmpq	$1, %rax
	jbe	.LBB52_143
.LBB52_131:
	lock		decq	496(%r15)
.LBB52_132:
	movq	(%rbx), %rax
	cmpq	$3, %rax
	je	.LBB52_134
	movl	$2, %eax
	xorl	%ecx, %ecx
	lock		cmpxchgq	%rcx, (%rbx)
.LBB52_134:
	testb	%bpl, %bpl
	jne	.LBB52_136
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count18GLOBAL_PANIC_COUNT@GOTPCREL(%rip), %rax
	movq	(%rax), %rax
	shlq	%rax
	testq	%rax, %rax
	jne	.LBB52_166
.LBB52_136:
	movl	$0, 36(%rsp)
	jmp	.LBB52_141
.LBB52_137:
	movq	(%rbx), %rax
	movl	$32, 36(%rsp)
	cmpq	$3, %rax
	je	.LBB52_139
	movl	$2, %eax
	xorl	%ecx, %ecx
	lock		cmpxchgq	%rcx, (%rbx)
.LBB52_139:
	testb	%bpl, %bpl
	jne	.LBB52_141
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count18GLOBAL_PANIC_COUNT@GOTPCREL(%rip), %rax
	movq	(%rax), %rax
	shlq	%rax
	testq	%rax, %rax
	jne	.LBB52_163
.LBB52_141:
	xorl	%eax, %eax
	xchgl	%eax, (%r14)
	movq	$-1, 64(%rsp)
	cmpl	$2, %eax
	je	.LBB52_162
.LBB52_142:
	movq	(%rbx), %rax
	cmpq	$3, %rax
	jne	.LBB52_7
	jmp	.LBB52_239
.LBB52_143:
	leaq	8(%r14), %r15
	movb	$1, 5(%r14)
	.p2align	4
.LBB52_144:
	cmpb	$1, 5(%r14)
	jne	.LBB52_132
	movl	8(%r14), %r13d
	xorl	%eax, %eax
	xchgl	%eax, (%r14)
	cmpl	$2, %eax
	je	.LBB52_152
.LBB52_146:
	movq	$0, 104(%rsp)
	.p2align	4
.LBB52_147:
	movl	(%r15), %eax
	cmpl	%r13d, %eax
	jne	.LBB52_150
	cmpb	$0, 104(%rsp)
	movq	syscall@GOTPCREL(%rip), %r10
	movl	$0, %r8d
	leaq	112(%rsp), %rax
	movl	$202, %edi
	movl	$137, %edx
	movl	$-1, (%rsp)
	movq	%r15, %rsi
	movl	%r13d, %ecx
	cmovneq	%rax, %r8
	xorl	%r9d, %r9d
	xorl	%eax, %eax
	callq	*%r10
	testq	%rax, %rax
	jns	.LBB52_150
	movq	__errno_location@GOTPCREL(%rip), %rax
	callq	*%rax
	cmpl	$4, (%rax)
	je	.LBB52_147
.LBB52_150:
	movl	$1, %ecx
	xorl	%eax, %eax
	lock		cmpxchgl	%ecx, (%r14)
	jne	.LBB52_153
	movzbl	4(%r14), %eax
	testb	%al, %al
	je	.LBB52_144
	jmp	.LBB52_240
.LBB52_152:
	movq	syscall@GOTPCREL(%rip), %r8
	movl	$202, %edi
	movl	$129, %edx
	movl	$1, %ecx
	movq	%r14, %rsi
	xorl	%eax, %eax
	callq	*%r8
	jmp	.LBB52_146
.LBB52_153:
	movq	_RNvMNtNtNtNtCs7jcFBdfocI9_3std3sys4sync5mutex5futexNtB2_5Mutex14lock_contended@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
	movzbl	4(%r14), %eax
	testb	%al, %al
	je	.LBB52_144
	jmp	.LBB52_240
.LBB52_154:
	movabsq	$4294967296, %rcx
.LBB52_155:
	movq	496(%r15), %rax
	testq	%rcx, %rax
	je	.LBB52_157
	leaq	(%rax,%rcx), %rdx
	lock		cmpxchgq	%rdx, 496(%r15)
	jne	.LBB52_155
	jmp	.LBB52_158
.LBB52_157:
	movq	%rax, %rdx
.LBB52_158:
	movq	sched_yield@GOTPCREL(%rip), %rax
	shrq	$32, %rdx
	movq	%rdx, 64(%rsp)
	callq	*%rax
	movl	$33, 36(%rsp)
	jmp	.LBB52_142
.LBB52_159:
	movq	_RNvMNtNtNtNtCs7jcFBdfocI9_3std3sys4sync5mutex5futexNtB2_5Mutex14lock_contended@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
	jmp	.LBB52_124
.LBB52_160:
.Ltmp394:
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count17is_zero_slow_path@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp395:
	movl	%eax, %ebp
	xorb	$1, %bpl
	movzbl	4(%r14), %eax
	testb	%al, %al
	je	.LBB52_126
	jmp	.LBB52_241
.LBB52_162:
	movq	syscall@GOTPCREL(%rip), %r8
	movl	$202, %edi
	movl	$129, %edx
	movl	$1, %ecx
	movq	%r14, %rsi
	xorl	%eax, %eax
	callq	*%r8
	movq	$-1, 64(%rsp)
	jmp	.LBB52_142
.LBB52_163:
.Ltmp402:
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count17is_zero_slow_path@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp403:
	testb	%al, %al
	jne	.LBB52_141
	movb	$1, 4(%r14)
	jmp	.LBB52_141
.LBB52_166:
.Ltmp410:
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count17is_zero_slow_path@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp411:
	testb	%al, %al
	jne	.LBB52_136
	movb	$1, 4(%r14)
	jmp	.LBB52_136
.LBB52_169:
	movq	%r15, 96(%rsp)
	movq	%r14, 88(%rsp)
	jmp	.LBB52_235
.LBB52_170:
	cmpl	$62, %ebp
	jne	.LBB52_184
	movq	(%r13), %rax
	testq	%rax, %rax
	je	.LBB52_213
.LBB52_172:
	movq	(%rax), %rcx
	andq	$-2, %r15
	xorl	%edx, %edx
	movq	%rax, 136(%r14)
	testq	%rcx, %rcx
	setne	%dl
	leaq	2(%r15,%rdx), %rcx
	movq	%rcx, 128(%r14)
	movq	1512(%r13), %rax
	testb	$1, %al
	jne	.LBB52_192
	xorl	%ebx, %ebx
	jmp	.LBB52_176
	.p2align	4
.LBB52_174:
	incl	%ebx
.LBB52_175:
	movq	1512(%r13), %rax
	testb	$1, %al
	jne	.LBB52_192
.LBB52_176:
	cmpl	$6, %ebx
	ja	.LBB52_179
	movl	$1, %eax
	.p2align	4
.LBB52_178:
	shrxl	%ebx, %eax, %ecx
	incl	%eax
	pause
	testl	%ecx, %ecx
	je	.LBB52_178
	jmp	.LBB52_174
	.p2align	4
.LBB52_179:
	movq	sched_yield@GOTPCREL(%rip), %rax
	callq	*%rax
	cmpl	$11, %ebx
	jae	.LBB52_175
	jmp	.LBB52_174
.LBB52_180:
	testq	%rdi, %rdi
	je	.LBB52_229
	decq	2072(%rdi)
	movq	24(%rsp), %rbx
	jne	.LBB52_235
	movq	$0, 2176(%rdi)
	cmpq	$0, 2080(%rdi)
	jne	.LBB52_235
.Ltmp356:
	movq	_RNvMs6_NtCs18aJq3QiqAb_15crossbeam_epoch8internalNtB5_5Local8finalize@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp357:
	jmp	.LBB52_235
.LBB52_184:
	leaq	(%rbp,%rbp,2), %rax
	leaq	8(%r13,%rax,8), %rbx
	movq	24(%r13,%rax,8), %rax
	testb	$1, %al
	jne	.LBB52_193
	xorl	%r14d, %r14d
	jmp	.LBB52_188
	.p2align	4
.LBB52_186:
	incl	%r14d
.LBB52_187:
	movq	16(%rbx), %rax
	testb	$1, %al
	jne	.LBB52_193
.LBB52_188:
	cmpl	$6, %r14d
	ja	.LBB52_191
	movl	$1, %eax
	.p2align	4
.LBB52_190:
	shrxl	%r14d, %eax, %ecx
	incl	%eax
	pause
	testl	%ecx, %ecx
	je	.LBB52_190
	jmp	.LBB52_186
	.p2align	4
.LBB52_191:
	movq	sched_yield@GOTPCREL(%rip), %rax
	callq	*%rax
	cmpl	$11, %r14d
	jae	.LBB52_187
	jmp	.LBB52_186
.LBB52_192:
	movq	1496(%r13), %r15
	movq	1504(%r13), %r14
	jmp	.LBB52_197
.LBB52_193:
	movq	(%rbx), %r15
	movq	8(%rbx), %r14
	movq	16(%rbx), %rax
	.p2align	4
.LBB52_194:
	movq	%rax, %rcx
	orq	$2, %rcx
	lock		cmpxchgq	%rcx, 16(%rbx)
	jne	.LBB52_194
	testb	$4, %al
	jne	.LBB52_197
.LBB52_229:
	movq	24(%rsp), %rbx
	jmp	.LBB52_235
.LBB52_197:
	movq	24(%rsp), %rbx
	testq	%rbp, %rbp
	jne	.LBB52_208
.LBB52_198:
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	$-1520, %rcx
	movabsq	$-9223372036854775808, %rdx
	addq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB52_200
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB52_200:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB52_206
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB52_200
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	$-1520, %rcx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	lock		xaddq	%rcx, (%rax)
	movabsq	$-9223372036854775808, %rax
	addq	$-1520, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB52_203:
	cmpq	%rax, %rcx
	jge	.LBB52_205
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB52_203
.LBB52_205:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB52_206:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	callq	*%rax
	jmp	.LBB52_235
	.p2align	4
.LBB52_207:
	testq	%rbp, %rbp
	je	.LBB52_198
.LBB52_208:
	decq	%rbp
	leaq	(%rbp,%rbp,2), %rax
	movq	24(%r13,%rax,8), %rcx
	testb	$2, %cl
	jne	.LBB52_207
	leaq	(%r13,%rax,8), %rcx
	movq	24(%rcx), %rax
	.p2align	4
.LBB52_210:
	movq	%rax, %rdx
	orq	$4, %rdx
	lock		cmpxchgq	%rdx, 24(%rcx)
	jne	.LBB52_210
	testb	$2, %al
	jne	.LBB52_207
	jmp	.LBB52_235
.LBB52_213:
	xorl	%ebx, %ebx
	jmp	.LBB52_216
	.p2align	4
.LBB52_214:
	incl	%ebx
.LBB52_215:
	movq	(%r13), %rax
	testq	%rax, %rax
	jne	.LBB52_172
.LBB52_216:
	cmpl	$6, %ebx
	ja	.LBB52_219
	movl	$1, %eax
	.p2align	4
.LBB52_218:
	shrxl	%ebx, %eax, %ecx
	incl	%eax
	pause
	testl	%ecx, %ecx
	je	.LBB52_218
	jmp	.LBB52_214
	.p2align	4
.LBB52_219:
	movq	sched_yield@GOTPCREL(%rip), %rax
	callq	*%rax
	cmpl	$11, %ebx
	jae	.LBB52_215
	jmp	.LBB52_214
.LBB52_221:
	movq	88(%rsp), %r14
	movq	96(%rsp), %r15
	movq	24(%rsp), %rbx
	jmp	.LBB52_235
.LBB52_222:
	movq	16(%rsp), %rdx
	movq	296(%rdx), %rsi
	movq	288(%rdx), %rcx
	leaq	-1(%rsi), %rdx
	andq	%rdi, %rdx
	shlq	$4, %rdx
	testq	%rsi, %rsi
	movq	(%rcx,%rdx), %r15
	movq	8(%rcx,%rdx), %r14
	leaq	3(%rsi), %rcx
	cmovnsq	%rsi, %rcx
	cmpq	$65, %rsi
	jb	.LBB52_229
	movq	24(%rsp), %rbx
	sarq	$2, %rcx
	cmpq	%rcx, %rax
	jg	.LBB52_235
	shrq	%rsi
.Ltmp342:
	movq	56(%rsp), %rdi
	callq	_RNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE6resizeBZ_
.Ltmp343:
	jmp	.LBB52_235
.LBB52_225:
	movq	24(%rsp), %rbx
	movq	%r15, 96(%rsp)
	movq	%r14, 88(%rsp)
	jmp	.LBB52_235
.LBB52_226:
	leaq	3(%rsi), %rax
	testq	%rsi, %rsi
	cmovnsq	%rsi, %rax
	cmpq	$65, %rsi
	jb	.LBB52_229
	movq	24(%rsp), %rbx
	sarq	$2, %rax
	cmpq	%rax, %rdx
	jge	.LBB52_235
	shrq	%rsi
.Ltmp344:
	movq	56(%rsp), %rdi
	callq	_RNvMs4_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_6WorkerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE6resizeBZ_
.Ltmp345:
	jmp	.LBB52_235
.LBB52_231:
	movq	72(%rsp), %r14
	movq	80(%rsp), %r15
	movq	24(%rsp), %rbx
	jmp	.LBB52_235
.LBB52_233:
	movq	24(%rsp), %rbx
.LBB52_234:
	movq	%r15, 80(%rsp)
	movq	%r14, 72(%rsp)
	.p2align	4
.LBB52_235:
	movq	16(%rsp), %rax
	movq	272(%rax), %rdi
	movq	$-65536, %rax
	lock		xaddq	%rax, 496(%rdi)
	addq	$472, %rdi
	movzwl	%ax, %esi
	movl	$2, %eax
	cmpq	$2, %rsi
	cmovaeq	%rax, %rsi
.Ltmp387:
	movq	_RNvMNtCs6a8jV7kq6PJ_10rayon_core5sleepNtB2_5Sleep16wake_any_threads@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp388:
.LBB52_236:
.Ltmp389:
	movq	%r14, %rdi
	callq	*%r15
.Ltmp390:
	movq	(%rbx), %rax
	cmpq	$3, %rax
	jne	.LBB52_2
.LBB52_238:
	addq	$168, %rsp
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
.LBB52_239:
	.cfi_def_cfa_offset 224
	movq	16(%rsp), %rax
	movl	$2, %esi
	movq	272(%rax), %rdi
	movq	$-65536, %rax
	lock		xaddq	%rax, 496(%rdi)
	addq	$472, %rdi
	movzwl	%ax, %eax
	cmpq	$2, %rax
	cmovbq	%rax, %rsi
.Ltmp413:
	movq	_RNvMNtCs6a8jV7kq6PJ_10rayon_core5sleepNtB2_5Sleep16wake_any_threads@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp414:
	jmp	.LBB52_238
.LBB52_240:
	movq	%r14, 104(%rsp)
	movb	%bpl, 112(%rsp)
.Ltmp404:
	movq	_RNvNtCs2k2z8Zem4rB_4core6result13unwrap_failed@GOTPCREL(%rip), %rax
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.26.llvm.7294274987384275483(%rip), %rdi
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.25.llvm.7294274987384275483(%rip), %rcx
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.3(%rip), %r8
	leaq	104(%rsp), %rdx
	movl	$43, %esi
	callq	*%rax
.Ltmp405:
	jmp	.LBB52_244
.LBB52_241:
	movq	%r14, 104(%rsp)
	movb	%bpl, 112(%rsp)
.Ltmp396:
	movq	_RNvNtCs2k2z8Zem4rB_4core6result13unwrap_failed@GOTPCREL(%rip), %rax
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.26.llvm.7294274987384275483(%rip), %rdi
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.25.llvm.7294274987384275483(%rip), %rcx
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.2(%rip), %r8
	leaq	104(%rsp), %rdx
	movl	$43, %esi
	callq	*%rax
.Ltmp397:
	jmp	.LBB52_244
.LBB52_242:
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.1(%rip), %rcx
	movq	%rax, 48(%rsp)
	movq	%rdx, %r13
	movq	%rcx, 136(%rsp)
.LBB52_243:
.Ltmp392:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking18panic_bounds_check@GOTPCREL(%rip), %rax
	movq	48(%rsp), %rsi
	movq	136(%rsp), %rdx
	movq	%r13, %rdi
	callq	*%rax
.Ltmp393:
.LBB52_244:
	ud2
.LBB52_245:
.Ltmp412:
	jmp	.LBB52_263
.LBB52_246:
.Ltmp386:
	jmp	.LBB52_263
.LBB52_247:
.Ltmp373:
	jmp	.LBB52_263
.LBB52_248:
.Ltmp415:
	jmp	.LBB52_263
.LBB52_249:
.Ltmp391:
	jmp	.LBB52_263
.LBB52_250:
.Ltmp398:
	movq	104(%rsp), %rbx
	cmpb	$0, 112(%rsp)
	jne	.LBB52_256
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count18GLOBAL_PANIC_COUNT@GOTPCREL(%rip), %rax
	movq	(%rax), %rax
	shlq	%rax
	testq	%rax, %rax
	je	.LBB52_256
.Ltmp399:
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count17is_zero_slow_path@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp400:
	jmp	.LBB52_259
.LBB52_253:
.Ltmp401:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB52_254:
.Ltmp406:
	movq	104(%rsp), %rbx
	cmpb	$0, 112(%rsp)
	jne	.LBB52_256
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count18GLOBAL_PANIC_COUNT@GOTPCREL(%rip), %rax
	movq	(%rax), %rax
	shlq	%rax
	testq	%rax, %rax
	jne	.LBB52_258
.LBB52_256:
	xorl	%eax, %eax
	xchgl	%eax, (%rbx)
	cmpl	$2, %eax
	jne	.LBB52_263
	movq	syscall@GOTPCREL(%rip), %r8
	movl	$202, %edi
	movl	$129, %edx
	movl	$1, %ecx
	movq	%rbx, %rsi
	xorl	%eax, %eax
	callq	*%r8
	jmp	.LBB52_263
.LBB52_258:
.Ltmp407:
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count17is_zero_slow_path@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp408:
.LBB52_259:
	testb	%al, %al
	jne	.LBB52_256
	movb	$1, 4(%rbx)
	jmp	.LBB52_256
.LBB52_261:
.Ltmp409:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB52_262:
.Ltmp360:
.LBB52_263:
.Ltmp416:
	movq	_RNvNtNtCs7jcFBdfocI9_3std2io5stdio7__eprint@GOTPCREL(%rip), %rax
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.52.llvm.7294274987384275483(%rip), %rdi
	movl	$87, %esi
	callq	*%rax
.Ltmp417:
	movq	_RNvNtCs7jcFBdfocI9_3std7process5abort@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB52_265:
.Ltmp418:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end52:
	.size	_RNvMs8_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThread15wait_until_cold, .Lfunc_end52-_RNvMs8_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThread15wait_until_cold
	.cfi_endproc
	.section	.gcc_except_table._RNvMs8_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThread15wait_until_cold,"a",@progbits
	.p2align	2, 0x0
GCC_except_table52:
.Lexception23:
	.byte	255
	.byte	155
	.uleb128 .Lttbase17-.Lttbaseref17
.Lttbaseref17:
	.byte	1
	.uleb128 .Lcst_end23-.Lcst_begin23
.Lcst_begin23:
	.uleb128 .Ltmp340-.Lfunc_begin52
	.uleb128 .Ltmp341-.Ltmp340
	.uleb128 .Ltmp391-.Lfunc_begin52
	.byte	0
	.uleb128 .Ltmp354-.Lfunc_begin52
	.uleb128 .Ltmp349-.Ltmp354
	.uleb128 .Ltmp360-.Lfunc_begin52
	.byte	0
	.uleb128 .Ltmp369-.Lfunc_begin52
	.uleb128 .Ltmp364-.Ltmp369
	.uleb128 .Ltmp373-.Lfunc_begin52
	.byte	0
	.uleb128 .Ltmp382-.Lfunc_begin52
	.uleb128 .Ltmp377-.Ltmp382
	.uleb128 .Ltmp386-.Lfunc_begin52
	.byte	0
	.uleb128 .Ltmp377-.Lfunc_begin52
	.uleb128 .Ltmp394-.Ltmp377
	.byte	0
	.byte	0
	.uleb128 .Ltmp394-.Lfunc_begin52
	.uleb128 .Ltmp395-.Ltmp394
	.uleb128 .Ltmp412-.Lfunc_begin52
	.byte	0
	.uleb128 .Ltmp395-.Lfunc_begin52
	.uleb128 .Ltmp402-.Ltmp395
	.byte	0
	.byte	0
	.uleb128 .Ltmp402-.Lfunc_begin52
	.uleb128 .Ltmp411-.Ltmp402
	.uleb128 .Ltmp412-.Lfunc_begin52
	.byte	0
	.uleb128 .Ltmp411-.Lfunc_begin52
	.uleb128 .Ltmp356-.Ltmp411
	.byte	0
	.byte	0
	.uleb128 .Ltmp356-.Lfunc_begin52
	.uleb128 .Ltmp357-.Ltmp356
	.uleb128 .Ltmp391-.Lfunc_begin52
	.byte	0
	.uleb128 .Ltmp357-.Lfunc_begin52
	.uleb128 .Ltmp342-.Ltmp357
	.byte	0
	.byte	0
	.uleb128 .Ltmp342-.Lfunc_begin52
	.uleb128 .Ltmp390-.Ltmp342
	.uleb128 .Ltmp391-.Lfunc_begin52
	.byte	0
	.uleb128 .Ltmp413-.Lfunc_begin52
	.uleb128 .Ltmp414-.Ltmp413
	.uleb128 .Ltmp415-.Lfunc_begin52
	.byte	0
	.uleb128 .Ltmp404-.Lfunc_begin52
	.uleb128 .Ltmp405-.Ltmp404
	.uleb128 .Ltmp406-.Lfunc_begin52
	.byte	0
	.uleb128 .Ltmp396-.Lfunc_begin52
	.uleb128 .Ltmp397-.Ltmp396
	.uleb128 .Ltmp398-.Lfunc_begin52
	.byte	0
	.uleb128 .Ltmp392-.Lfunc_begin52
	.uleb128 .Ltmp393-.Ltmp392
	.uleb128 .Ltmp415-.Lfunc_begin52
	.byte	0
	.uleb128 .Ltmp399-.Lfunc_begin52
	.uleb128 .Ltmp400-.Ltmp399
	.uleb128 .Ltmp401-.Lfunc_begin52
	.byte	1
	.uleb128 .Ltmp400-.Lfunc_begin52
	.uleb128 .Ltmp407-.Ltmp400
	.byte	0
	.byte	0
	.uleb128 .Ltmp407-.Lfunc_begin52
	.uleb128 .Ltmp408-.Ltmp407
	.uleb128 .Ltmp409-.Lfunc_begin52
	.byte	1
	.uleb128 .Ltmp408-.Lfunc_begin52
	.uleb128 .Ltmp416-.Ltmp408
	.byte	0
	.byte	0
	.uleb128 .Ltmp416-.Lfunc_begin52
	.uleb128 .Ltmp417-.Ltmp416
	.uleb128 .Ltmp418-.Lfunc_begin52
	.byte	1
	.uleb128 .Ltmp417-.Lfunc_begin52
	.uleb128 .Lfunc_end52-.Ltmp417
	.byte	0
	.byte	0
.Lcst_end23:
	.byte	127
	.byte	0
	.p2align	2, 0x0
.Lttbase17:
	.byte	0
	.p2align	2, 0x0

	.section	.text._RNvMsg_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_8InjectorNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE4pushB11_.llvm.7294274987384275483,"ax",@progbits
	.hidden	_RNvMsg_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_8InjectorNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE4pushB11_.llvm.7294274987384275483
	.globl	_RNvMsg_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_8InjectorNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE4pushB11_.llvm.7294274987384275483
	.prefalign	4, .Lfunc_end53, nop
	.type	_RNvMsg_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_8InjectorNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE4pushB11_.llvm.7294274987384275483,@function
_RNvMsg_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_8InjectorNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE4pushB11_.llvm.7294274987384275483:
.Lfunc_begin53:
	.cfi_startproc
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
	subq	$24, %rsp
	.cfi_def_cfa_offset 80
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	%rdx, 16(%rsp)
	movq	%rsi, 8(%rsp)
	movl	$1537, %r14d
	movq	%rdi, %r12
	xorl	%ebp, %ebp
	movq	128(%rdi), %r13
	movq	136(%rdi), %rbx
	movq	$0, (%rsp)
	bextrl	%r14d, %r13d, %r15d
	cmpl	$63, %r15d
	je	.LBB53_2
	jmp	.LBB53_8
	.p2align	4
.LBB53_6:
	incl	%ebp
.LBB53_7:
	movq	128(%r12), %r13
	movq	136(%r12), %rbx
	bextrl	%r14d, %r13d, %r15d
	cmpl	$63, %r15d
	jne	.LBB53_8
.LBB53_2:
	cmpl	$6, %ebp
	ja	.LBB53_5
	movl	$1, %eax
	.p2align	4
.LBB53_4:
	shrxl	%ebp, %eax, %ecx
	incl	%eax
	pause
	testl	%ecx, %ecx
	je	.LBB53_4
	jmp	.LBB53_6
	.p2align	4
.LBB53_5:
	movq	sched_yield@GOTPCREL(%rip), %rax
	callq	*%rax
	cmpl	$11, %ebp
	jb	.LBB53_6
	jmp	.LBB53_7
	.p2align	4
.LBB53_8:
	movq	%r15, %rax
	xorq	$62, %rax
	orq	(%rsp), %rax
	jne	.LBB53_19
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$1520, %edi
	callq	*%rax
	movq	%rax, (%rsp)
	testq	%rax, %rax
	je	.LBB53_38
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	$-1, %rdx
	movl	$1520, %ecx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	addq	%rcx, %rax
	cmovbq	%rdx, %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	addq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jle	.LBB53_12
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB53_12:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB53_18
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB53_12
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	$1520, (%rcx)
	movl	$1520, %ecx
	lock		xaddq	%rcx, (%rsi)
	addq	$1520, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB53_15:
	cmpq	%rax, %rcx
	jle	.LBB53_17
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB53_15
.LBB53_17:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB53_18:
	movq	(%rsp), %rdi
	movq	memset@GOTPCREL(%rip), %rax
	movl	$1520, %edx
	xorl	%esi, %esi
	callq	*%rax
.LBB53_19:
	leaq	2(%r13), %rcx
	movq	%r13, %rax
	lock		cmpxchgq	%rcx, 128(%r12)
	je	.LBB53_20
	movq	136(%r12), %rbx
	cmpl	$6, %ebp
	movl	$6, %ecx
	movl	$1, %edx
	cmovbl	%ebp, %ecx
	.p2align	4
.LBB53_36:
	shrxl	%ecx, %edx, %esi
	incl	%edx
	pause
	testl	%esi, %esi
	je	.LBB53_36
	cmpl	$7, %ebp
	movq	%rax, %r13
	adcl	$0, %ebp
	bextrl	%r14d, %r13d, %r15d
	cmpl	$63, %r15d
	je	.LBB53_2
	jmp	.LBB53_8
.LBB53_20:
	cmpl	$62, %r15d
	jne	.LBB53_21
	movq	(%rsp), %rax
	testq	%rax, %rax
	je	.LBB53_31
	addq	$4, %r13
	movq	%rax, 136(%r12)
	movq	%r13, 128(%r12)
	movq	%rax, (%rbx)
	movq	8(%rsp), %rax
	movq	16(%rsp), %rcx
	movq	%rax, 1496(%rbx)
	movq	%rcx, 1504(%rbx)
	lock		orq	$1, 1512(%rbx)
	jmp	.LBB53_34
.LBB53_21:
	movq	8(%rsp), %rcx
	movq	16(%rsp), %rdx
	leaq	(%r15,%r15,2), %rax
	movq	%rcx, 8(%rbx,%rax,8)
	movq	%rdx, 16(%rbx,%rax,8)
	lock		orq	$1, 24(%rbx,%rax,8)
	movq	(%rsp), %rdi
	testq	%rdi, %rdi
	je	.LBB53_34
	movq	$-1520, %rax
	addq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movabsq	$-9223372036854775808, %rcx
	cmovoq	%rcx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB53_24
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB53_24:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB53_30
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB53_24
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	$-1520, %rdx
	lock		xaddq	%rdx, (%rax)
	addq	$-1520, %rdx
	cmovoq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
	.p2align	4
.LBB53_27:
	cmpq	%rax, %rdx
	jge	.LBB53_29
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB53_27
.LBB53_29:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB53_30:
	addq	$24, %rsp
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
	jmpq	*free@GOTPCREL(%rip)
.LBB53_34:
	.cfi_def_cfa_offset 80
	addq	$24, %rsp
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
.LBB53_31:
	.cfi_def_cfa_offset 80
	movq	_RNvNtCs2k2z8Zem4rB_4core6option13unwrap_failed@GOTPCREL(%rip), %rax
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.48(%rip), %rdi
	callq	*%rax
.LBB53_38:
	movq	_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$1520, %esi
	callq	*%rax
.Lfunc_end53:
	.size	_RNvMsg_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_8InjectorNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE4pushB11_.llvm.7294274987384275483, .Lfunc_end53-_RNvMsg_NtCs64OF0TycdZY_15crossbeam_deque5dequeINtB5_8InjectorNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefE4pushB11_.llvm.7294274987384275483
	.cfi_endproc

	.section	.text._RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_,"ax",@progbits
	.globl	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_
	.prefalign	4, .Lfunc_end54, nop
	.type	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_,@function
_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_:
.Lfunc_begin54:
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
	movq	(%rdi), %rbx
	movq	128(%rbx), %r14
	andq	$-8, %r14
	movq	8(%r14), %rcx
	testq	%rcx, %rcx
	je	.LBB54_10
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	shlq	$4, %rcx
	movq	(%r14), %rdi
	xorl	%esi, %esi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB54_3
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB54_3:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB54_9
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB54_3
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB54_6:
	cmpq	%rax, %rsi
	jge	.LBB54_8
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB54_6
.LBB54_8:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB54_9:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB54_10:
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movabsq	$-9223372036854775808, %r15
	addq	$-16, %rax
	cmovoq	%r15, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB54_12
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB54_12:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB54_18
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB54_12
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	movq	$-16, %rcx
	lock		xaddq	%rcx, (%rax)
	movq	(%rdx), %rax
	addq	$-16, %rcx
	cmovoq	%r15, %rcx
	.p2align	4
.LBB54_15:
	cmpq	%rax, %rcx
	jge	.LBB54_17
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB54_15
.LBB54_17:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB54_18:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
	cmpq	$-1, %rbx
	je	.LBB54_29
	lock		decq	8(%rbx)
	jne	.LBB54_29
	#MEMBARRIER
	movq	$-384, %rax
	addq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	cmovoq	%r15, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB54_22
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB54_22:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB54_28
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB54_22
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	movq	$-384, %rcx
	lock		xaddq	%rcx, (%rax)
	movq	(%rdx), %rax
	addq	$-384, %rcx
	cmovoq	%r15, %rcx
	.p2align	4
.LBB54_25:
	cmpq	%rax, %rcx
	jge	.LBB54_27
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB54_25
.LBB54_27:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB54_28:
	movq	%rbx, %rdi
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	jmpq	*free@GOTPCREL(%rip)
.LBB54_29:
	.cfi_def_cfa_offset 32
	popq	%rbx
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end54:
	.size	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_, .Lfunc_end54-_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_
	.cfi_endproc

	.section	.text._RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEE9drop_slowCs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.globl	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEE9drop_slowCs6a8jV7kq6PJ_10rayon_core
	.prefalign	4, .Lfunc_end55, nop
	.type	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEE9drop_slowCs6a8jV7kq6PJ_10rayon_core,@function
_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEE9drop_slowCs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin55:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception24
	pushq	%r14
	.cfi_def_cfa_offset 16
	pushq	%rbx
	.cfi_def_cfa_offset 24
	pushq	%rax
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -24
	.cfi_offset %r14, -16
	movq	(%rdi), %rbx
	leaq	16(%rbx), %rdi
.Ltmp419:
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEECs6a8jV7kq6PJ_10rayon_core
.Ltmp420:
	cmpq	$-1, %rbx
	je	.LBB55_16
	lock		decq	8(%rbx)
	jne	.LBB55_16
	#MEMBARRIER
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movabsq	$-9223372036854775808, %rcx
	addq	$-48, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB55_5
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB55_5:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB55_11
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB55_5
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	$-48, %rdx
	lock		xaddq	%rdx, (%rax)
	addq	$-48, %rdx
	cmovoq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
	.p2align	4
.LBB55_8:
	cmpq	%rax, %rdx
	jge	.LBB55_10
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB55_8
.LBB55_10:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB55_11:
	movq	%rbx, %rdi
	addq	$8, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*free@GOTPCREL(%rip)
.LBB55_16:
	.cfi_def_cfa_offset 32
	addq	$8, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	retq
.LBB55_12:
	.cfi_def_cfa_offset 32
.Ltmp421:
	movq	%rax, %r14
	cmpq	$-1, %rbx
	je	.LBB55_15
	lock		decq	8(%rbx)
	jne	.LBB55_15
	movl	$48, %esi
	movl	$8, %edx
	movq	%rbx, %rdi
	#MEMBARRIER
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
.LBB55_15:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end55:
	.size	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEE9drop_slowCs6a8jV7kq6PJ_10rayon_core, .Lfunc_end55-_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEE9drop_slowCs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc
	.section	.gcc_except_table._RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEE9drop_slowCs6a8jV7kq6PJ_10rayon_core,"a",@progbits
	.p2align	2, 0x0
GCC_except_table55:
.Lexception24:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end24-.Lcst_begin24
.Lcst_begin24:
	.uleb128 .Ltmp419-.Lfunc_begin55
	.uleb128 .Ltmp420-.Ltmp419
	.uleb128 .Ltmp421-.Lfunc_begin55
	.byte	0
	.uleb128 .Ltmp420-.Lfunc_begin55
	.uleb128 .Lfunc_end55-.Ltmp420
	.byte	0
	.byte	0
.Lcst_end24:
	.p2align	2, 0x0

	.section	.text._RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryE9drop_slowBK_,"ax",@progbits
	.globl	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryE9drop_slowBK_
	.prefalign	4, .Lfunc_end56, nop
	.type	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryE9drop_slowBK_,@function
_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryE9drop_slowBK_:
.Lfunc_begin56:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception25
	pushq	%r14
	.cfi_def_cfa_offset 16
	pushq	%rbx
	.cfi_def_cfa_offset 24
	pushq	%rax
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -24
	.cfi_offset %r14, -16
	movq	(%rdi), %rbx
	leaq	128(%rbx), %rdi
.Ltmp422:
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryEBF_
.Ltmp423:
	cmpq	$-1, %rbx
	je	.LBB56_16
	lock		decq	8(%rbx)
	jne	.LBB56_16
	#MEMBARRIER
	movq	$-640, %rax
	addq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movabsq	$-9223372036854775808, %rcx
	cmovoq	%rcx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB56_5
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB56_5:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB56_11
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB56_5
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	$-640, %rdx
	lock		xaddq	%rdx, (%rax)
	addq	$-640, %rdx
	cmovoq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
	.p2align	4
.LBB56_8:
	cmpq	%rax, %rdx
	jge	.LBB56_10
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB56_8
.LBB56_10:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB56_11:
	movq	%rbx, %rdi
	addq	$8, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*free@GOTPCREL(%rip)
.LBB56_16:
	.cfi_def_cfa_offset 32
	addq	$8, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	retq
.LBB56_12:
	.cfi_def_cfa_offset 32
.Ltmp424:
	movq	%rax, %r14
	cmpq	$-1, %rbx
	je	.LBB56_15
	lock		decq	8(%rbx)
	jne	.LBB56_15
	movl	$640, %esi
	movl	$128, %edx
	movq	%rbx, %rdi
	#MEMBARRIER
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
.LBB56_15:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end56:
	.size	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryE9drop_slowBK_, .Lfunc_end56-_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryE9drop_slowBK_
	.cfi_endproc
	.section	.gcc_except_table._RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryE9drop_slowBK_,"a",@progbits
	.p2align	2, 0x0
GCC_except_table56:
.Lexception25:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end25-.Lcst_begin25
.Lcst_begin25:
	.uleb128 .Ltmp422-.Lfunc_begin56
	.uleb128 .Ltmp423-.Ltmp422
	.uleb128 .Ltmp424-.Lfunc_begin56
	.byte	0
	.uleb128 .Ltmp423-.Lfunc_begin56
	.uleb128 .Lfunc_end56-.Ltmp423
	.byte	0
	.byte	0
.Lcst_end25:
	.p2align	2, 0x0

	.section	.text._RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtNtCs7jcFBdfocI9_3std6thread6scoped9ScopeDataE9drop_slowCs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.globl	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtNtCs7jcFBdfocI9_3std6thread6scoped9ScopeDataE9drop_slowCs6a8jV7kq6PJ_10rayon_core
	.prefalign	4, .Lfunc_end57, nop
	.type	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtNtCs7jcFBdfocI9_3std6thread6scoped9ScopeDataE9drop_slowCs6a8jV7kq6PJ_10rayon_core,@function
_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtNtCs7jcFBdfocI9_3std6thread6scoped9ScopeDataE9drop_slowCs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin57:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception26
	pushq	%r14
	.cfi_def_cfa_offset 16
	pushq	%rbx
	.cfi_def_cfa_offset 24
	pushq	%rax
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -24
	.cfi_offset %r14, -16
	movq	(%rdi), %rbx
	movq	16(%rbx), %rax
	lock		decq	(%rax)
	jne	.LBB57_2
	leaq	16(%rbx), %rdi
	#MEMBARRIER
.Ltmp425:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtNtCs7jcFBdfocI9_3std6thread6thread5InnerNtNtBM_5alloc6SystemE9drop_slowBM_@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp426:
.LBB57_2:
	cmpq	$-1, %rbx
	je	.LBB57_17
	lock		decq	8(%rbx)
	jne	.LBB57_17
	#MEMBARRIER
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movabsq	$-9223372036854775808, %rcx
	addq	$-40, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB57_6
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB57_6:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB57_12
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB57_6
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	$-40, %rdx
	lock		xaddq	%rdx, (%rax)
	addq	$-40, %rdx
	cmovoq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
	.p2align	4
.LBB57_9:
	cmpq	%rax, %rdx
	jge	.LBB57_11
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB57_9
.LBB57_11:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB57_12:
	movq	%rbx, %rdi
	addq	$8, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*free@GOTPCREL(%rip)
.LBB57_17:
	.cfi_def_cfa_offset 32
	addq	$8, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	retq
.LBB57_13:
	.cfi_def_cfa_offset 32
.Ltmp427:
	movq	%rax, %r14
	cmpq	$-1, %rbx
	je	.LBB57_16
	lock		decq	8(%rbx)
	jne	.LBB57_16
	movl	$40, %esi
	movl	$8, %edx
	movq	%rbx, %rdi
	#MEMBARRIER
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
.LBB57_16:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end57:
	.size	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtNtCs7jcFBdfocI9_3std6thread6scoped9ScopeDataE9drop_slowCs6a8jV7kq6PJ_10rayon_core, .Lfunc_end57-_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtNtCs7jcFBdfocI9_3std6thread6scoped9ScopeDataE9drop_slowCs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc
	.section	.gcc_except_table._RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtNtCs7jcFBdfocI9_3std6thread6scoped9ScopeDataE9drop_slowCs6a8jV7kq6PJ_10rayon_core,"a",@progbits
	.p2align	2, 0x0
GCC_except_table57:
.Lexception26:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end26-.Lcst_begin26
.Lcst_begin26:
	.uleb128 .Ltmp425-.Lfunc_begin57
	.uleb128 .Ltmp426-.Ltmp425
	.uleb128 .Ltmp427-.Lfunc_begin57
	.byte	0
	.uleb128 .Ltmp426-.Lfunc_begin57
	.uleb128 .Lfunc_end57-.Ltmp426
	.byte	0
	.byte	0
.Lcst_end26:
	.p2align	2, 0x0

	.section	.text.unlikely._RNvNtCs6a8jV7kq6PJ_10rayon_core4join23join_recover_from_panic,"ax",@progbits
	.globl	_RNvNtCs6a8jV7kq6PJ_10rayon_core4join23join_recover_from_panic
	.prefalign	4, .Lfunc_end58, nop
	.type	_RNvNtCs6a8jV7kq6PJ_10rayon_core4join23join_recover_from_panic,@function
_RNvNtCs6a8jV7kq6PJ_10rayon_core4join23join_recover_from_panic:
.Lfunc_begin58:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception27
	pushq	%r15
	.cfi_def_cfa_offset 16
	pushq	%r14
	.cfi_def_cfa_offset 24
	pushq	%rbx
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -32
	.cfi_offset %r14, -24
	.cfi_offset %r15, -16
	movq	8(%rsi), %rax
	movq	%rcx, %r14
	movq	%rdx, %rbx
	cmpq	$3, %rax
	jne	.LBB58_1
.LBB58_2:
	movq	_RNvNtCs6a8jV7kq6PJ_10rayon_core6unwind16resume_unwinding@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	movq	%r14, %rsi
	callq	*%rax
.LBB58_1:
.Ltmp428:
	movq	_RNvMs8_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThread15wait_until_cold@GOTPCREL(%rip), %rax
	addq	$8, %rsi
	callq	*%rax
.Ltmp429:
	jmp	.LBB58_2
.LBB58_3:
.Ltmp430:
	movq	%rax, %r15
	movq	(%r14), %rax
	testq	%rax, %rax
	je	.LBB58_5
.Ltmp431:
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp432:
.LBB58_5:
	movq	8(%r14), %rsi
	testq	%rsi, %rsi
	je	.LBB58_7
	movq	16(%r14), %rdx
	movq	%rbx, %rdi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
.LBB58_7:
	movq	%r15, %rdi
	callq	_Unwind_Resume@PLT
.LBB58_8:
.Ltmp433:
	movq	8(%r14), %rsi
	testq	%rsi, %rsi
	je	.LBB58_10
	movq	16(%r14), %rdx
	movq	%rbx, %rdi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
.LBB58_10:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end58:
	.size	_RNvNtCs6a8jV7kq6PJ_10rayon_core4join23join_recover_from_panic, .Lfunc_end58-_RNvNtCs6a8jV7kq6PJ_10rayon_core4join23join_recover_from_panic
	.cfi_endproc
	.section	.gcc_except_table._RNvNtCs6a8jV7kq6PJ_10rayon_core4join23join_recover_from_panic,"a",@progbits
	.p2align	2, 0x0
GCC_except_table58:
.Lexception27:
	.byte	255
	.byte	155
	.uleb128 .Lttbase18-.Lttbaseref18
.Lttbaseref18:
	.byte	1
	.uleb128 .Lcst_end27-.Lcst_begin27
.Lcst_begin27:
	.uleb128 .Lfunc_begin58-.Lfunc_begin58
	.uleb128 .Ltmp428-.Lfunc_begin58
	.byte	0
	.byte	0
	.uleb128 .Ltmp428-.Lfunc_begin58
	.uleb128 .Ltmp429-.Ltmp428
	.uleb128 .Ltmp430-.Lfunc_begin58
	.byte	0
	.uleb128 .Ltmp431-.Lfunc_begin58
	.uleb128 .Ltmp432-.Ltmp431
	.uleb128 .Ltmp433-.Lfunc_begin58
	.byte	1
	.uleb128 .Ltmp432-.Lfunc_begin58
	.uleb128 .Lfunc_end58-.Ltmp432
	.byte	0
	.byte	0
.Lcst_end27:
	.byte	127
	.byte	0
	.p2align	2, 0x0
.Lttbase18:
	.byte	0
	.p2align	2, 0x0

	.section	.text._RNvNtCs6a8jV7kq6PJ_10rayon_core6unwind16resume_unwinding,"ax",@progbits
	.globl	_RNvNtCs6a8jV7kq6PJ_10rayon_core6unwind16resume_unwinding
	.prefalign	4, .Lfunc_end59, nop
	.type	_RNvNtCs6a8jV7kq6PJ_10rayon_core6unwind16resume_unwinding,@function
_RNvNtCs6a8jV7kq6PJ_10rayon_core6unwind16resume_unwinding:
.Lfunc_begin59:
	.cfi_startproc
	pushq	%rax
	.cfi_def_cfa_offset 16
	movq	_RNvNtCs7jcFBdfocI9_3std5panic13resume_unwind@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end59:
	.size	_RNvNtCs6a8jV7kq6PJ_10rayon_core6unwind16resume_unwinding, .Lfunc_end59-_RNvNtCs6a8jV7kq6PJ_10rayon_core6unwind16resume_unwinding
	.cfi_endproc

	.section	.text._RNvNtCs6a8jV7kq6PJ_10rayon_core8registry15global_registry,"ax",@progbits
	.globl	_RNvNtCs6a8jV7kq6PJ_10rayon_core8registry15global_registry
	.prefalign	4, .Lfunc_end60, nop
	.type	_RNvNtCs6a8jV7kq6PJ_10rayon_core8registry15global_registry,@function
_RNvNtCs6a8jV7kq6PJ_10rayon_core8registry15global_registry:
.Lfunc_begin60:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception28
	pushq	%rbx
	.cfi_def_cfa_offset 16
	subq	$32, %rsp
	.cfi_def_cfa_offset 48
	.cfi_offset %rbx, -16
	movq	_RNvNtCs6a8jV7kq6PJ_10rayon_core8registry16THE_REGISTRY_SET@GOTPCREL(%rip), %rax
	movq	$0, (%rsp)
	movl	(%rax), %eax
	testl	%eax, %eax
	jne	.LBB60_1
.LBB60_2:
	movq	(%rsp), %rcx
	movq	8(%rsp), %rdi
	cmpq	$-1, %rcx
	je	.LBB60_3
	movq	_RNvNtCs6a8jV7kq6PJ_10rayon_core8registry12THE_REGISTRY@GOTPCREL(%rip), %rax
	cmpq	$0, (%rax)
	je	.LBB60_9
	cmpq	$2, %rcx
	jb	.LBB60_19
	movl	%edi, %ecx
	andl	$3, %ecx
	leal	-2(%rcx), %edx
	cmpl	$2, %edx
	jb	.LBB60_19
	testq	%rcx, %rcx
	je	.LBB60_19
	movq	%rax, %rbx
	movq	23(%rdi), %rax
	decq	%rdi
	callq	*%rax
	movq	%rbx, %rax
.LBB60_19:
	addq	$32, %rsp
	.cfi_def_cfa_offset 16
	popq	%rbx
	.cfi_def_cfa_offset 8
	retq
.LBB60_3:
	.cfi_def_cfa_offset 48
	movq	%rdi, %rax
	addq	$32, %rsp
	.cfi_def_cfa_offset 16
	popq	%rbx
	.cfi_def_cfa_offset 8
	retq
.LBB60_1:
	.cfi_def_cfa_offset 48
	movq	%rsp, %rax
	movq	%rax, 16(%rsp)
	leaq	16(%rsp), %rax
	movq	%rax, 24(%rsp)
.Ltmp434:
	movq	_RNvNtCs6a8jV7kq6PJ_10rayon_core8registry16THE_REGISTRY_SET@GOTPCREL(%rip), %rdi
	movq	_RNvMs0_NtNtNtNtCs7jcFBdfocI9_3std3sys4sync4once5futexNtB5_4Once4call@GOTPCREL(%rip), %rax
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.5.llvm.7294274987384275483(%rip), %rcx
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.9.llvm.7294274987384275483(%rip), %r8
	leaq	24(%rsp), %rdx
	xorl	%esi, %esi
	callq	*%rax
.Ltmp435:
	jmp	.LBB60_2
.LBB60_9:
	movq	%rcx, (%rsp)
	movq	%rdi, 8(%rsp)
.Ltmp440:
	movq	_RNvNtCs2k2z8Zem4rB_4core6result13unwrap_failed@GOTPCREL(%rip), %rax
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.49.llvm.7294274987384275483(%rip), %rdi
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.27.llvm.7294274987384275483(%rip), %rcx
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.50.llvm.7294274987384275483(%rip), %r8
	movl	$48, %esi
	movq	%rsp, %rdx
	callq	*%rax
.Ltmp441:
	ud2
.LBB60_4:
.Ltmp436:
	cmpl	$2, (%rsp)
	movq	%rax, %rbx
	jne	.LBB60_7
	movq	8(%rsp), %rdi
	movl	%edi, %eax
	andl	$3, %eax
	cmpl	$1, %eax
	jne	.LBB60_7
	movq	23(%rdi), %rax
	decq	%rdi
.Ltmp437:
	callq	*%rax
.Ltmp438:
	jmp	.LBB60_7
.LBB60_20:
.Ltmp439:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB60_11:
.Ltmp442:
	cmpl	$2, (%rsp)
	movq	%rax, %rbx
	jb	.LBB60_7
	movq	8(%rsp), %rdi
	movl	%edi, %eax
	andl	$3, %eax
	cmpl	$1, %eax
	je	.LBB60_13
.LBB60_7:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB60_13:
	movq	23(%rdi), %rax
	decq	%rdi
.Ltmp443:
	callq	*%rax
.Ltmp444:
	jmp	.LBB60_7
.LBB60_14:
.Ltmp445:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end60:
	.size	_RNvNtCs6a8jV7kq6PJ_10rayon_core8registry15global_registry, .Lfunc_end60-_RNvNtCs6a8jV7kq6PJ_10rayon_core8registry15global_registry
	.cfi_endproc
	.section	.gcc_except_table._RNvNtCs6a8jV7kq6PJ_10rayon_core8registry15global_registry,"a",@progbits
	.p2align	2, 0x0
GCC_except_table60:
.Lexception28:
	.byte	255
	.byte	155
	.uleb128 .Lttbase19-.Lttbaseref19
.Lttbaseref19:
	.byte	1
	.uleb128 .Lcst_end28-.Lcst_begin28
.Lcst_begin28:
	.uleb128 .Lfunc_begin60-.Lfunc_begin60
	.uleb128 .Ltmp434-.Lfunc_begin60
	.byte	0
	.byte	0
	.uleb128 .Ltmp434-.Lfunc_begin60
	.uleb128 .Ltmp435-.Ltmp434
	.uleb128 .Ltmp436-.Lfunc_begin60
	.byte	0
	.uleb128 .Ltmp440-.Lfunc_begin60
	.uleb128 .Ltmp441-.Ltmp440
	.uleb128 .Ltmp442-.Lfunc_begin60
	.byte	0
	.uleb128 .Ltmp437-.Lfunc_begin60
	.uleb128 .Ltmp438-.Ltmp437
	.uleb128 .Ltmp439-.Lfunc_begin60
	.byte	1
	.uleb128 .Ltmp438-.Lfunc_begin60
	.uleb128 .Ltmp443-.Ltmp438
	.byte	0
	.byte	0
	.uleb128 .Ltmp443-.Lfunc_begin60
	.uleb128 .Ltmp444-.Ltmp443
	.uleb128 .Ltmp445-.Lfunc_begin60
	.byte	1
	.uleb128 .Ltmp444-.Lfunc_begin60
	.uleb128 .Lfunc_end60-.Ltmp444
	.byte	0
	.byte	0
.Lcst_end28:
	.byte	127
	.byte	0
	.p2align	2, 0x0
.Lttbase19:
	.byte	0
	.p2align	2, 0x0

	.section	.text.unlikely._RNvXNtCs6a8jV7kq6PJ_10rayon_core6unwindNtB2_12AbortIfPanicNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop,"ax",@progbits
	.globl	_RNvXNtCs6a8jV7kq6PJ_10rayon_core6unwindNtB2_12AbortIfPanicNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop
	.prefalign	4, .Lfunc_end61, nop
	.type	_RNvXNtCs6a8jV7kq6PJ_10rayon_core6unwindNtB2_12AbortIfPanicNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop,@function
_RNvXNtCs6a8jV7kq6PJ_10rayon_core6unwindNtB2_12AbortIfPanicNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop:
.Lfunc_begin61:
	.cfi_startproc
	pushq	%rax
	.cfi_def_cfa_offset 16
	movq	_RNvNtNtCs7jcFBdfocI9_3std2io5stdio7__eprint@GOTPCREL(%rip), %rax
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.52.llvm.7294274987384275483(%rip), %rdi
	movl	$87, %esi
	callq	*%rax
	movq	_RNvNtCs7jcFBdfocI9_3std7process5abort@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end61:
	.size	_RNvXNtCs6a8jV7kq6PJ_10rayon_core6unwindNtB2_12AbortIfPanicNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop, .Lfunc_end61-_RNvXNtCs6a8jV7kq6PJ_10rayon_core6unwindNtB2_12AbortIfPanicNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop
	.cfi_endproc

	.section	.text._RNvXNvNtNtCs2k2z8Zem4rB_4core2io5write17default_write_fmtINtB2_7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrENtNtB8_3fmt5Write9write_strCs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.prefalign	4, .Lfunc_end62, nop
	.type	_RNvXNvNtNtCs2k2z8Zem4rB_4core2io5write17default_write_fmtINtB2_7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrENtNtB8_3fmt5Write9write_strCs6a8jV7kq6PJ_10rayon_core,@function
_RNvXNvNtNtCs2k2z8Zem4rB_4core2io5write17default_write_fmtINtB2_7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrENtNtB8_3fmt5Write9write_strCs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin62:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception29
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
	pushq	%rax
	.cfi_def_cfa_offset 64
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	testq	%rdx, %rdx
	je	.LBB62_11
	movq	write@GOTPCREL(%rip), %r13
	movq	_RNvNtNtNtCs2k2z8Zem4rB_4core2io5error12os_functions12OS_FUNCTIONS@GOTPCREL(%rip), %rbx
	leaq	anon.f13bb3f258876cb12f4a84b0a5cdace5.73.llvm.11640361436736466388(%rip), %rbp
	movq	%rdx, %r14
	movq	%rsi, %r15
	movq	%rdi, (%rsp)
	jmp	.LBB62_3
	.p2align	4
.LBB62_2:
	testq	%r14, %r14
	je	.LBB62_11
.LBB62_3:
	movl	$2, %edi
	movq	%r15, %rsi
	movq	%r14, %rdx
	callq	*%r13
	cmpq	$-1, %rax
	je	.LBB62_7
	testq	%rax, %rax
	je	.LBB62_12
	movq	%r14, %rcx
	subq	%rax, %rcx
	jb	.LBB62_19
	addq	%rax, %r15
	movq	%rcx, %r14
	jmp	.LBB62_2
	.p2align	4
.LBB62_7:
	movq	__errno_location@GOTPCREL(%rip), %rax
	callq	*%rax
	movslq	(%rax), %r12
	movq	(%rbx), %rax
	cmpq	%rbp, %rax
	jne	.LBB62_9
.LBB62_8:
	movq	(%rbx), %rax
	movl	%r12d, %edi
	movq	16(%rax), %rax
	callq	*%rax
	testb	%al, %al
	jne	.LBB62_2
	jmp	.LBB62_13
.LBB62_9:
	movq	%rbp, (%rbx)
	jmp	.LBB62_8
.LBB62_11:
	xorl	%eax, %eax
	jmp	.LBB62_18
.LBB62_12:
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.144(%rip), %r12
	movq	(%rsp), %rbx
	movq	8(%rbx), %rdi
	testq	%rdi, %rdi
	jne	.LBB62_14
	jmp	.LBB62_17
.LBB62_13:
	shlq	$32, %r12
	orq	$2, %r12
	movq	(%rsp), %rbx
	movq	8(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB62_17
.LBB62_14:
	movl	%edi, %eax
	andl	$3, %eax
	leal	-2(%rax), %ecx
	cmpl	$2, %ecx
	jb	.LBB62_17
	testq	%rax, %rax
	je	.LBB62_17
	movq	23(%rdi), %rax
	decq	%rdi
.Ltmp446:
	callq	*%rax
.Ltmp447:
.LBB62_17:
	movb	$1, %al
	movq	%r12, 8(%rbx)
.LBB62_18:
	addq	$8, %rsp
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
.LBB62_19:
	.cfi_def_cfa_offset 64
	movq	_RNvNtNtCs2k2z8Zem4rB_4core5slice5index16slice_index_fail@GOTPCREL(%rip), %r8
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.145(%rip), %rcx
	movq	%rax, %rdi
	movq	%r14, %rsi
	movq	%r14, %rdx
	callq	*%r8
.LBB62_20:
.Ltmp448:
	movq	%rax, %rdi
	movq	%r12, 8(%rbx)
	callq	_Unwind_Resume@PLT
.Lfunc_end62:
	.size	_RNvXNvNtNtCs2k2z8Zem4rB_4core2io5write17default_write_fmtINtB2_7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrENtNtB8_3fmt5Write9write_strCs6a8jV7kq6PJ_10rayon_core, .Lfunc_end62-_RNvXNvNtNtCs2k2z8Zem4rB_4core2io5write17default_write_fmtINtB2_7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrENtNtB8_3fmt5Write9write_strCs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc
	.section	.gcc_except_table._RNvXNvNtNtCs2k2z8Zem4rB_4core2io5write17default_write_fmtINtB2_7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrENtNtB8_3fmt5Write9write_strCs6a8jV7kq6PJ_10rayon_core,"a",@progbits
	.p2align	2, 0x0
GCC_except_table62:
.Lexception29:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end29-.Lcst_begin29
.Lcst_begin29:
	.uleb128 .Lfunc_begin62-.Lfunc_begin62
	.uleb128 .Ltmp446-.Lfunc_begin62
	.byte	0
	.byte	0
	.uleb128 .Ltmp446-.Lfunc_begin62
	.uleb128 .Ltmp447-.Ltmp446
	.uleb128 .Ltmp448-.Lfunc_begin62
	.byte	0
	.uleb128 .Ltmp447-.Lfunc_begin62
	.uleb128 .Lfunc_end62-.Ltmp447
	.byte	0
	.byte	0
.Lcst_end29:
	.p2align	2, 0x0

	.section	.text.unlikely._RNvXNvNtNtCs7jcFBdfocI9_3std3sys12thread_local20abort_on_dtor_unwindNtB2_15DtorUnwindGuardNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop,"ax",@progbits
	.prefalign	4, .Lfunc_end63, nop
	.type	_RNvXNvNtNtCs7jcFBdfocI9_3std3sys12thread_local20abort_on_dtor_unwindNtB2_15DtorUnwindGuardNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop,@function
_RNvXNvNtNtCs7jcFBdfocI9_3std3sys12thread_local20abort_on_dtor_unwindNtB2_15DtorUnwindGuardNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop:
.Lfunc_begin63:
	.cfi_startproc
	pushq	%rax
	.cfi_def_cfa_offset 16
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.53(%rip), %rsi
	leaq	7(%rsp), %rdi
	movl	$123, %edx
	callq	_RNvYNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrNtNtNtCs2k2z8Zem4rB_4core2io5write5Write9write_fmtCs6a8jV7kq6PJ_10rayon_core
	movq	%rax, %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtB4_6result6ResultuNtNtNtB4_2io5error5ErrorEECs6a8jV7kq6PJ_10rayon_core
	movq	_RNvNtCs7jcFBdfocI9_3std7process5abort@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end63:
	.size	_RNvXNvNtNtCs7jcFBdfocI9_3std3sys12thread_local20abort_on_dtor_unwindNtB2_15DtorUnwindGuardNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop, .Lfunc_end63-_RNvXNvNtNtCs7jcFBdfocI9_3std3sys12thread_local20abort_on_dtor_unwindNtB2_15DtorUnwindGuardNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop
	.cfi_endproc

	.section	.text._RNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12DefaultSpawnNtB5_11ThreadSpawn5spawn,"ax",@progbits
	.globl	_RNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12DefaultSpawnNtB5_11ThreadSpawn5spawn
	.prefalign	4, .Lfunc_end64, nop
	.type	_RNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12DefaultSpawnNtB5_11ThreadSpawn5spawn,@function
_RNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12DefaultSpawnNtB5_11ThreadSpawn5spawn:
.Lfunc_begin64:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception30
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
	subq	$648, %rsp
	.cfi_def_cfa_offset 704
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	16(%rsi), %r15
	movq	%rsi, %rbx
	cmpq	$-1, %r15
	je	.LBB64_4
	movq	24(%rbx), %r13
	movq	32(%rbx), %rbp
	testq	%rbp, %rbp
	jns	.LBB64_5
	xorl	%edi, %edi
.LBB64_3:
.Ltmp449:
	movq	_RNvNtCsc70TAahYccp_5alloc7raw_vec12handle_error@GOTPCREL(%rip), %rax
	movq	%rbp, %rsi
	callq	*%rax
.Ltmp450:
	jmp	.LBB64_126
.LBB64_4:
	movq	$-1, %rbp
	jmp	.LBB64_17
.LBB64_5:
	je	.LBB64_16
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%rbp, %rdi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB64_124
	movq	%rax, %r14
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rsi
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rdi
	movabsq	$-9223372036854775808, %rcx
	movq	$-1, %r8
	leaq	(%rbp,%rax), %rdx
	sarq	$63, %rdx
	xorq	%rcx, %rdx
	addq	%rbp, %rax
	cmovoq	%rdx, %rax
	incq	%rsi
	cmoveq	%r8, %rsi
	addq	%rbp, %rdi
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmovbq	%r8, %rdi
	movq	%rsi, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%rdi, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jle	.LBB64_9
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB64_9:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB64_15
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB64_9
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	%rbp, (%rdx)
	movq	%rbp, %rdx
	lock		xaddq	%rdx, (%rsi)
	leaq	(%rdx,%rbp), %rax
	sarq	$63, %rax
	xorq	%rcx, %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	addq	%rbp, %rdx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB64_12:
	cmpq	%rax, %rdx
	jle	.LBB64_14
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB64_12
.LBB64_14:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB64_15:
	movq	memcpy@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	movq	%r13, %rsi
	movq	%rbp, %rdx
	callq	*%rax
	jmp	.LBB64_17
.LBB64_16:
	movl	$1, %r14d
	xorl	%ebp, %ebp
.LBB64_17:
	movq	(%rbx), %rax
	movq	8(%rbx), %r12
	movq	%rbp, 8(%rsp)
	movq	%r14, 24(%rsp)
	movq	%rax, 64(%rsp)
	movq	%r12, 72(%rsp)
	movq	%rbp, 80(%rsp)
	movq	%r14, 88(%rsp)
	movq	%rbp, 96(%rsp)
	movb	$0, 104(%rsp)
	testq	%rax, %rax
	jne	.LBB64_31
	movq	_RNvNCNvNtNtCs7jcFBdfocI9_3std6thread9lifecycle15spawn_unchecked03MIN@GOTPCREL(%rip), %rbp
	movq	(%rbp), %r12
	testq	%r12, %r12
	je	.LBB64_20
	decq	%r12
	jmp	.LBB64_30
.LBB64_20:
	movabsq	$5423250206716022345, %rax
	movb	$1, %r15b
	movq	%rax, 262(%rsp)
	movabsq	$5641125080090236242, %rax
	movq	%rax, 256(%rsp)
	movb	$0, 270(%rsp)
.Ltmp455:
	movq	_RNvMs3_NtNtCs2k2z8Zem4rB_4core3ffi5c_strNtB5_4CStr19from_bytes_with_nul@GOTPCREL(%rip), %rax
	leaq	112(%rsp), %rdi
	leaq	256(%rsp), %rsi
	movl	$15, %edx
	callq	*%rax
.Ltmp456:
	cmpl	$1, 112(%rsp)
	jne	.LBB64_23
	leaq	anon.f13bb3f258876cb12f4a84b0a5cdace5.76.llvm.11640361436736466388(%rip), %r13
	jmp	.LBB64_25
.LBB64_23:
	movq	120(%rsp), %rdx
.Ltmp457:
	leaq	32(%rsp), %rdi
	callq	_RNCNvNtNtNtCs7jcFBdfocI9_3std3sys3env4unix6getenv0B9_.llvm.11640361436736466388
.Ltmp458:
	movq	32(%rsp), %r12
	movq	40(%rsp), %r13
	cmpq	$-2, %r12
	jne	.LBB64_83
.LBB64_25:
	movl	%r13d, %eax
	andl	$3, %eax
	leal	-2(%rax), %ecx
	cmpl	$2, %ecx
	jb	.LBB64_28
	testq	%rax, %rax
	je	.LBB64_28
	movq	23(%r13), %rax
	decq	%r13
.Ltmp462:
	movq	%r13, %rdi
	callq	*%rax
.Ltmp463:
	jmp	.LBB64_28
.LBB64_83:
	cmpq	$-1, %r12
	je	.LBB64_28
.Ltmp459:
	movq	48(%rsp), %rdx
	movq	_RNvNtNtCs2k2z8Zem4rB_4core3str8converts9from_utf8@GOTPCREL(%rip), %rax
	leaq	256(%rsp), %rdi
	movq	%r13, %rsi
	callq	*%rax
.Ltmp460:
	cmpl	$1, 256(%rsp)
	je	.LBB64_90
	movq	272(%rsp), %rcx
	testq	%rcx, %rcx
	je	.LBB64_90
	movq	264(%rsp), %rsi
	cmpq	$1, %rcx
	jne	.LBB64_104
	movzbl	(%rsi), %eax
	cmpl	$43, %eax
	je	.LBB64_90
	cmpl	$45, %eax
	jne	.LBB64_105
.LBB64_90:
	movq	%r12, %r9
.LBB64_91:
	movl	$2097152, %r12d
.LBB64_92:
	testq	%r9, %r9
	je	.LBB64_29
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movabsq	$9223372036854775807, %rcx
	cmpq	%rcx, %r9
	cmovaeq	%rcx, %r9
	xorl	%edx, %edx
	movq	%r9, %rsi
	cmpq	%r9, %rax
	setns	%dl
	addq	%rcx, %rdx
	subq	%r9, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB64_95
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB64_95:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB64_101
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB64_95
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rsi, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rsi, %rdx
	setns	%al
	addq	%rcx, %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	subq	%rsi, %rdx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
.LBB64_98:
	cmpq	%rax, %rdx
	jge	.LBB64_100
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB64_98
.LBB64_100:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB64_101:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	callq	*%rax
	jmp	.LBB64_29
.LBB64_28:
	movl	$2097152, %r12d
.LBB64_29:
	leaq	1(%r12), %rax
	movq	%rax, (%rbp)
.LBB64_30:
	movq	8(%rsp), %rbp
.LBB64_31:
	movq	_RNvNvMNtNtCs7jcFBdfocI9_3std6thread2idNtB4_8ThreadId3new7COUNTER.llvm.11640361436736466388(%rip), %rax
	leaq	80(%rsp), %rdi
	.p2align	4
.LBB64_32:
	cmpq	$-1, %rax
	je	.LBB64_103
	leaq	1(%rax), %r14
	lock		cmpxchgq	%r14, _RNvNvMNtNtCs7jcFBdfocI9_3std6thread2idNtB4_8ThreadId3new7COUNTER.llvm.11640361436736466388(%rip)
	jne	.LBB64_32
	cmpq	$-1, %rbp
	je	.LBB64_37
	xorl	%r15d, %r15d
.Ltmp464:
	movq	_RNvXNtNtNtCs7jcFBdfocI9_3std6thread6thread18thread_name_stringNtB2_16ThreadNameStringINtNtCs2k2z8Zem4rB_4core7convert4FromNtNtCsc70TAahYccp_5alloc6string6StringE4from@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp465:
	movq	%rax, %r13
	movq	%rdx, %rbp
	jmp	.LBB64_38
.LBB64_37:
	xorl	%r13d, %r13d
.LBB64_38:
	movq	malloc@GOTPCREL(%rip), %r15
	movl	$64, %edi
	callq	*%r15
	testq	%rax, %rax
	je	.LBB64_125
	movq	$1, (%rax)
	movq	$1, 8(%rax)
	movq	%r13, 24(%rax)
	movq	%rbp, 32(%rax)
	movq	%r14, 16(%rax)
	movl	$3, 48(%rax)
	movl	$0, 56(%rax)
	movq	%rax, (%rsp)
.Ltmp466:
	movq	_RNvNtNtCs7jcFBdfocI9_3std6thread9spawnhook15run_spawn_hooks@GOTPCREL(%rip), %rax
	leaq	32(%rsp), %rdi
	movq	%rsp, %rsi
	callq	*%rax
.Ltmp467:
	movl	$48, %edi
	movq	$1, 256(%rsp)
	movq	$1, 264(%rsp)
	vxorps	%xmm0, %xmm0, %xmm0
	vmovups	%xmm0, 272(%rsp)
	callq	*%r15
	testq	%rax, %rax
	je	.LBB64_121
	movq	%rax, %r13
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	$-1, %rcx
	movabsq	$9223372036854775807, %rbp
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	addq	$48, %rax
	cmovbq	%rcx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	addq	$48, %rax
	cmovoq	%rbp, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jle	.LBB64_43
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB64_43:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB64_49
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB64_43
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdx
	lock		incq	(%rax)
	lock		addq	$48, (%rcx)
	movl	$48, %ecx
	lock		xaddq	%rcx, (%rdx)
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	movq	(%rdx), %rax
	addq	$48, %rcx
	cmovoq	%rbp, %rcx
	.p2align	4
.LBB64_46:
	cmpq	%rax, %rcx
	jle	.LBB64_48
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB64_46
.LBB64_48:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB64_49:
	vmovups	272(%rsp), %ymm1
	vmovups	256(%rsp), %ymm0
	movq	%r13, 16(%rsp)
	vmovups	%ymm1, 16(%r13)
	vmovups	%ymm0, (%r13)
	lock		incq	(%r13)
	jle	.LBB64_126
	vmovups	32(%rsp), %ymm0
	vmovups	(%rbx), %zmm2
	vmovups	40(%rbx), %zmm1
	movq	16(%r13), %rdi
	vmovups	%ymm0, 216(%rsp)
	vmovups	%zmm2, 112(%rsp)
	vmovups	%zmm1, 152(%rsp)
	movq	%r13, 248(%rsp)
	testq	%rdi, %rdi
	je	.LBB64_52
	lock		incq	24(%rdi)
	jle	.LBB64_119
.LBB64_52:
	vmovups	112(%rsp), %zmm0
	vmovups	192(%rsp), %zmm2
	vmovups	176(%rsp), %zmm1
	movl	$144, %edi
	movl	$144, %r14d
	vmovups	%zmm2, 336(%rsp)
	vmovups	%zmm1, 320(%rsp)
	vmovups	%zmm0, 256(%rsp)
	vzeroupper
	callq	*%r15
	testq	%rax, %rax
	je	.LBB64_122
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rdi
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rsi
	movq	$-1, %rdx
	movq	%rax, %rcx
	incq	%rdi
	cmoveq	%rdx, %rdi
	addq	%r14, %rsi
	cmovbq	%rdx, %rsi
	addq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %r14
	movq	%rdi, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%rsi, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmovoq	%rbp, %r14
	movq	%r14, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %r14
	jle	.LBB64_55
	movq	%r14, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB64_55:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB64_61
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB64_55
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	$144, (%rdx)
	movl	$144, %edx
	lock		xaddq	%rdx, (%rsi)
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rsi
	movq	(%rsi), %rax
	addq	$144, %rdx
	cmovoq	%rbp, %rdx
	.p2align	4
.LBB64_58:
	cmpq	%rax, %rdx
	jle	.LBB64_60
	lock		cmpxchgq	%rdx, (%rsi)
	jne	.LBB64_58
.LBB64_60:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB64_61:
	vmovups	112(%rsp), %zmm0
	vmovups	192(%rsp), %zmm2
	vmovups	176(%rsp), %zmm1
	movq	(%rsp), %rax
	vmovups	%zmm2, 80(%rcx)
	vmovups	%zmm1, 64(%rcx)
	vmovups	%zmm0, (%rcx)
	lock		incq	(%rax)
	jle	.LBB64_126
	movq	(%rsp), %rax
	movl	$24, %edi
	movq	%rax, 256(%rsp)
	movq	%rcx, 264(%rsp)
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.14(%rip), %rcx
	movq	%rcx, 272(%rsp)
	vzeroupper
	callq	*%r15
	testq	%rax, %rax
	je	.LBB64_123
	movq	%rax, %rsi
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	addq	$24, %rax
	cmovbq	%rcx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	addq	$24, %rax
	cmovoq	%rbp, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jle	.LBB64_65
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB64_65:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB64_71
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB64_65
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdx
	lock		incq	(%rax)
	lock		addq	$24, (%rcx)
	movl	$24, %ecx
	lock		xaddq	%rcx, (%rdx)
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	movq	(%rdx), %rax
	addq	$24, %rcx
	cmovoq	%rbp, %rcx
	.p2align	4
.LBB64_68:
	cmpq	%rax, %rcx
	jle	.LBB64_70
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB64_68
.LBB64_70:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB64_71:
	vmovups	256(%rsp), %xmm0
	movq	272(%rsp), %rax
	movq	%rax, 16(%rsi)
	vmovups	%xmm0, (%rsi)
.Ltmp474:
	movq	_RNvMs0_NtNtNtCs7jcFBdfocI9_3std3sys6thread4unixNtB5_6Thread3new@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	callq	*%rax
.Ltmp475:
	movq	%rdx, %r12
	cmpq	$1, %rax
	jne	.LBB64_77
	lock		decq	(%r13)
	jne	.LBB64_75
	#MEMBARRIER
.Ltmp483:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEE9drop_slowCs6a8jV7kq6PJ_10rayon_core@GOTPCREL(%rip), %rax
	leaq	16(%rsp), %rdi
	callq	*%rax
.Ltmp484:
.LBB64_75:
	movq	(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB64_82
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtNtCs7jcFBdfocI9_3std6thread6thread5InnerNtNtBM_5alloc6SystemE9drop_slowBM_@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	#MEMBARRIER
	callq	*%rax
	jmp	.LBB64_82
.LBB64_77:
	movq	(%rsp), %r14
	movq	pthread_detach@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	leaq	264(%rsp), %rbx
	movq	%r14, 256(%rsp)
	movq	%r13, 264(%rsp)
	movq	%r12, 272(%rsp)
	callq	*%rax
	lock		decq	(%r14)
	jne	.LBB64_79
	#MEMBARRIER
.Ltmp477:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtNtCs7jcFBdfocI9_3std6thread6thread5InnerNtNtBM_5alloc6SystemE9drop_slowBM_@GOTPCREL(%rip), %rax
	leaq	256(%rsp), %rdi
	callq	*%rax
.Ltmp478:
.LBB64_79:
	movq	264(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB64_81
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEE9drop_slowCs6a8jV7kq6PJ_10rayon_core@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	#MEMBARRIER
	callq	*%rax
.LBB64_81:
	xorl	%r12d, %r12d
.LBB64_82:
	movq	%r12, %rax
	addq	$648, %rsp
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
.LBB64_103:
	.cfi_def_cfa_offset 704
	movb	$1, %r15b
.Ltmp513:
	callq	_RNvNvMNtNtCs7jcFBdfocI9_3std6thread2idNtB4_8ThreadId3new9exhausted.llvm.11640361436736466388
.Ltmp514:
	jmp	.LBB64_126
.LBB64_104:
	movzbl	(%rsi), %eax
.LBB64_105:
	xorl	%edi, %edi
	cmpb	$43, %al
	movq	%rcx, %rdx
	movq	%r12, %r9
	sete	%dil
	movq	%rdi, %rax
	subq	%rdi, %rdx
	addq	%rdi, %rsi
	negq	%rax
	cmpq	$17, %rdx
	jae	.LBB64_110
	testq	%rdx, %rdx
	je	.LBB64_117
	addq	%rax, %rcx
	xorl	%r12d, %r12d
	xorl	%eax, %eax
	negq	%rcx
.LBB64_108:
	movzbl	(%rsi,%rax), %edx
	addl	$-48, %edx
	cmpl	$9, %edx
	ja	.LBB64_91
	leaq	(%r12,%r12,4), %rdi
	movl	%edx, %edx
	incq	%rax
	leaq	(%rdx,%rdi,2), %r12
	movq	%rcx, %rdx
	addq	%rax, %rdx
	jne	.LBB64_108
	jmp	.LBB64_92
.LBB64_110:
	addq	%rax, %rcx
	xorl	%r12d, %r12d
	movl	$10, %edi
	xorl	%r8d, %r8d
	negq	%rcx
.LBB64_111:
	movq	%r12, %rax
	mulq	%rdi
	jo	.LBB64_91
	movzbl	(%rsi,%r8), %edx
	movq	%rax, %r12
	addl	$-48, %edx
	addq	%rdx, %r12
	setb	%al
	cmpl	$9, %edx
	ja	.LBB64_91
	testb	%al, %al
	jne	.LBB64_91
	incq	%r8
	movq	%rcx, %rax
	addq	%r8, %rax
	jne	.LBB64_111
	jmp	.LBB64_92
.LBB64_117:
	xorl	%r12d, %r12d
	jmp	.LBB64_92
.LBB64_119:
	addq	$16, %rdi
.Ltmp469:
	vzeroupper
	callq	_RNvMNtNtCs7jcFBdfocI9_3std6thread6scopedNtB2_9ScopeData8overflow.llvm.11640361436736466388
.Ltmp470:
	jmp	.LBB64_126
.LBB64_121:
.Ltmp500:
	movq	_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$48, %esi
	leaq	272(%rsp), %r13
	callq	*%rax
.Ltmp501:
	jmp	.LBB64_126
.LBB64_122:
.Ltmp492:
	movq	_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$144, %esi
	callq	*%rax
.Ltmp493:
	jmp	.LBB64_126
.LBB64_123:
.Ltmp486:
	movq	_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$24, %esi
	callq	*%rax
.Ltmp487:
	jmp	.LBB64_126
.LBB64_124:
	movl	$1, %edi
	jmp	.LBB64_3
.LBB64_125:
.Ltmp510:
	movq	_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$64, %esi
	callq	*%rax
.Ltmp511:
.LBB64_126:
	ud2
.LBB64_127:
.Ltmp512:
	movq	%rax, %r12
	testq	%r13, %r13
	je	.LBB64_156
	movb	$0, (%r13)
	testq	%rbp, %rbp
	je	.LBB64_156
	movl	$1, %edx
	movq	%r13, %rdi
	movq	%rbp, %rsi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
	xorl	%r15d, %r15d
	jmp	.LBB64_158
.LBB64_132:
.Ltmp461:
	movb	$1, %r15b
	movq	%r12, %rcx
	movq	%rax, %r12
	testq	%rcx, %rcx
	je	.LBB64_158
	movl	$1, %edx
	movq	%rcx, %rsi
	movq	%r13, %rdi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
	jmp	.LBB64_158
.LBB64_134:
.Ltmp479:
	movq	264(%rsp), %rcx
	movq	%rax, %r12
	lock		decq	(%rcx)
	jne	.LBB64_178
	#MEMBARRIER
.Ltmp480:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEE9drop_slowCs6a8jV7kq6PJ_10rayon_core@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp481:
	jmp	.LBB64_178
.LBB64_136:
.Ltmp482:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB64_137:
.Ltmp485:
	movq	%rax, %r12
	jmp	.LBB64_152
.LBB64_138:
.Ltmp488:
	movq	%rax, %r12
.Ltmp489:
	leaq	256(%rsp), %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtNtCs7jcFBdfocI9_3std6thread9lifecycle10ThreadInitECs6a8jV7kq6PJ_10rayon_core
.Ltmp490:
	jmp	.LBB64_149
.LBB64_139:
.Ltmp491:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB64_140:
.Ltmp494:
	movq	%rax, %r12
.Ltmp495:
	leaq	256(%rsp), %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNCINvNtNtCs7jcFBdfocI9_3std6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1I_12DefaultSpawnNtB1I_11ThreadSpawn5spawn0uEs_0EB1K_
.Ltmp496:
	jmp	.LBB64_149
.LBB64_141:
.Ltmp497:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB64_142:
.Ltmp476:
	movq	%rax, %r12
	jmp	.LBB64_149
.LBB64_143:
.Ltmp502:
	movq	%rax, %r12
.Ltmp503:
	movq	%r13, %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEECs6a8jV7kq6PJ_10rayon_core
.Ltmp504:
.Ltmp506:
	leaq	32(%rsp), %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtNtCs7jcFBdfocI9_3std6thread9spawnhook15ChildSpawnHooksECs6a8jV7kq6PJ_10rayon_core
.Ltmp507:
	jmp	.LBB64_147
.LBB64_145:
.Ltmp505:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB64_146:
.Ltmp468:
	movq	%rax, %r12
.LBB64_147:
	movb	$1, %bpl
	jmp	.LBB64_153
.LBB64_148:
.Ltmp471:
	movq	%rax, %r12
.Ltmp472:
	leaq	112(%rsp), %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNCINvNtNtCs7jcFBdfocI9_3std6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1I_12DefaultSpawnNtB1I_11ThreadSpawn5spawn0uEs_0EB1K_
.Ltmp473:
.LBB64_149:
	lock		decq	(%r13)
	jne	.LBB64_152
	#MEMBARRIER
.Ltmp498:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtNtCs7jcFBdfocI9_3std6thread9lifecycle6PacketuEE9drop_slowCs6a8jV7kq6PJ_10rayon_core@GOTPCREL(%rip), %rax
	leaq	16(%rsp), %rdi
	callq	*%rax
.Ltmp499:
.LBB64_152:
	xorl	%ebp, %ebp
.LBB64_153:
	movq	(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB64_155
	#MEMBARRIER
.Ltmp508:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtNtCs7jcFBdfocI9_3std6thread6thread5InnerNtNtBM_5alloc6SystemE9drop_slowBM_@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	callq	*%rax
.Ltmp509:
.LBB64_155:
	testb	%bpl, %bpl
	je	.LBB64_178
.LBB64_156:
	xorl	%r15d, %r15d
	jmp	.LBB64_158
.LBB64_157:
.Ltmp515:
	movq	%rax, %r12
.LBB64_158:
.Ltmp516:
	movq	%rbx, %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtBJ_12DefaultSpawnNtBJ_11ThreadSpawn5spawn0EBL_
.Ltmp517:
	movq	24(%rsp), %rdi
	movq	8(%rsp), %rsi
	testq	%rsi, %rsi
	setle	%al
	xorb	$1, %r15b
	orb	%al, %r15b
	jne	.LBB64_178
	movl	$1, %edx
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
	movq	%r12, %rdi
	callq	_Unwind_Resume@PLT
.LBB64_161:
.Ltmp518:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB64_162:
.Ltmp451:
	movq	%rax, %r12
	testq	%r15, %r15
	jne	.LBB64_166
	movq	56(%rbx), %rax
	lock		decq	(%rax)
	jne	.LBB64_164
.LBB64_175:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %rax
	leaq	56(%rbx), %rdi
	#MEMBARRIER
	callq	*%rax
	movq	40(%rbx), %rax
	lock		decq	(%rax)
	je	.LBB64_176
.LBB64_165:
	movq	88(%rbx), %rax
	lock		decq	(%rax)
	je	.LBB64_177
	jmp	.LBB64_178
.LBB64_166:
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	xorl	%edx, %edx
	movabsq	$9223372036854775807, %rcx
	cmpq	%r15, %rax
	setns	%dl
	addq	%rcx, %rdx
	subq	%r15, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB64_168
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB64_168:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB64_174
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB64_168
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%r15, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%r15, %rdx
	setns	%al
	addq	%rcx, %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	subq	%r15, %rdx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB64_171:
	cmpq	%rax, %rdx
	jge	.LBB64_173
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB64_171
.LBB64_173:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB64_174:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	callq	*%rax
	movq	56(%rbx), %rax
	lock		decq	(%rax)
	je	.LBB64_175
.LBB64_164:
	movq	40(%rbx), %rax
	lock		decq	(%rax)
	jne	.LBB64_165
.LBB64_176:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %rax
	leaq	40(%rbx), %rdi
	#MEMBARRIER
	callq	*%rax
	movq	88(%rbx), %rax
	lock		decq	(%rax)
	jne	.LBB64_178
.LBB64_177:
	addq	$88, %rbx
	#MEMBARRIER
.Ltmp452:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryE9drop_slowBK_@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp453:
.LBB64_178:
	movq	%r12, %rdi
	callq	_Unwind_Resume@PLT
.LBB64_179:
.Ltmp454:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end64:
	.size	_RNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12DefaultSpawnNtB5_11ThreadSpawn5spawn, .Lfunc_end64-_RNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12DefaultSpawnNtB5_11ThreadSpawn5spawn
	.cfi_endproc
	.section	.gcc_except_table._RNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12DefaultSpawnNtB5_11ThreadSpawn5spawn,"a",@progbits
	.p2align	2, 0x0
GCC_except_table64:
.Lexception30:
	.byte	255
	.byte	155
	.uleb128 .Lttbase20-.Lttbaseref20
.Lttbaseref20:
	.byte	1
	.uleb128 .Lcst_end30-.Lcst_begin30
.Lcst_begin30:
	.uleb128 .Ltmp449-.Lfunc_begin64
	.uleb128 .Ltmp450-.Ltmp449
	.uleb128 .Ltmp451-.Lfunc_begin64
	.byte	0
	.uleb128 .Ltmp450-.Lfunc_begin64
	.uleb128 .Ltmp455-.Ltmp450
	.byte	0
	.byte	0
	.uleb128 .Ltmp455-.Lfunc_begin64
	.uleb128 .Ltmp463-.Ltmp455
	.uleb128 .Ltmp515-.Lfunc_begin64
	.byte	0
	.uleb128 .Ltmp459-.Lfunc_begin64
	.uleb128 .Ltmp460-.Ltmp459
	.uleb128 .Ltmp461-.Lfunc_begin64
	.byte	0
	.uleb128 .Ltmp460-.Lfunc_begin64
	.uleb128 .Ltmp464-.Ltmp460
	.byte	0
	.byte	0
	.uleb128 .Ltmp464-.Lfunc_begin64
	.uleb128 .Ltmp465-.Ltmp464
	.uleb128 .Ltmp515-.Lfunc_begin64
	.byte	0
	.uleb128 .Ltmp465-.Lfunc_begin64
	.uleb128 .Ltmp466-.Ltmp465
	.byte	0
	.byte	0
	.uleb128 .Ltmp466-.Lfunc_begin64
	.uleb128 .Ltmp467-.Ltmp466
	.uleb128 .Ltmp468-.Lfunc_begin64
	.byte	0
	.uleb128 .Ltmp467-.Lfunc_begin64
	.uleb128 .Ltmp474-.Ltmp467
	.byte	0
	.byte	0
	.uleb128 .Ltmp474-.Lfunc_begin64
	.uleb128 .Ltmp475-.Ltmp474
	.uleb128 .Ltmp476-.Lfunc_begin64
	.byte	0
	.uleb128 .Ltmp483-.Lfunc_begin64
	.uleb128 .Ltmp484-.Ltmp483
	.uleb128 .Ltmp485-.Lfunc_begin64
	.byte	0
	.uleb128 .Ltmp484-.Lfunc_begin64
	.uleb128 .Ltmp477-.Ltmp484
	.byte	0
	.byte	0
	.uleb128 .Ltmp477-.Lfunc_begin64
	.uleb128 .Ltmp478-.Ltmp477
	.uleb128 .Ltmp479-.Lfunc_begin64
	.byte	0
	.uleb128 .Ltmp478-.Lfunc_begin64
	.uleb128 .Ltmp513-.Ltmp478
	.byte	0
	.byte	0
	.uleb128 .Ltmp513-.Lfunc_begin64
	.uleb128 .Ltmp514-.Ltmp513
	.uleb128 .Ltmp515-.Lfunc_begin64
	.byte	0
	.uleb128 .Ltmp469-.Lfunc_begin64
	.uleb128 .Ltmp470-.Ltmp469
	.uleb128 .Ltmp471-.Lfunc_begin64
	.byte	0
	.uleb128 .Ltmp500-.Lfunc_begin64
	.uleb128 .Ltmp501-.Ltmp500
	.uleb128 .Ltmp502-.Lfunc_begin64
	.byte	0
	.uleb128 .Ltmp492-.Lfunc_begin64
	.uleb128 .Ltmp493-.Ltmp492
	.uleb128 .Ltmp494-.Lfunc_begin64
	.byte	0
	.uleb128 .Ltmp486-.Lfunc_begin64
	.uleb128 .Ltmp487-.Ltmp486
	.uleb128 .Ltmp488-.Lfunc_begin64
	.byte	0
	.uleb128 .Ltmp510-.Lfunc_begin64
	.uleb128 .Ltmp511-.Ltmp510
	.uleb128 .Ltmp512-.Lfunc_begin64
	.byte	0
	.uleb128 .Ltmp511-.Lfunc_begin64
	.uleb128 .Ltmp480-.Ltmp511
	.byte	0
	.byte	0
	.uleb128 .Ltmp480-.Lfunc_begin64
	.uleb128 .Ltmp481-.Ltmp480
	.uleb128 .Ltmp482-.Lfunc_begin64
	.byte	1
	.uleb128 .Ltmp481-.Lfunc_begin64
	.uleb128 .Ltmp489-.Ltmp481
	.byte	0
	.byte	0
	.uleb128 .Ltmp489-.Lfunc_begin64
	.uleb128 .Ltmp490-.Ltmp489
	.uleb128 .Ltmp491-.Lfunc_begin64
	.byte	1
	.uleb128 .Ltmp490-.Lfunc_begin64
	.uleb128 .Ltmp495-.Ltmp490
	.byte	0
	.byte	0
	.uleb128 .Ltmp495-.Lfunc_begin64
	.uleb128 .Ltmp496-.Ltmp495
	.uleb128 .Ltmp497-.Lfunc_begin64
	.byte	1
	.uleb128 .Ltmp496-.Lfunc_begin64
	.uleb128 .Ltmp503-.Ltmp496
	.byte	0
	.byte	0
	.uleb128 .Ltmp503-.Lfunc_begin64
	.uleb128 .Ltmp504-.Ltmp503
	.uleb128 .Ltmp505-.Lfunc_begin64
	.byte	1
	.uleb128 .Ltmp506-.Lfunc_begin64
	.uleb128 .Ltmp507-.Ltmp506
	.uleb128 .Ltmp518-.Lfunc_begin64
	.byte	1
	.uleb128 .Ltmp507-.Lfunc_begin64
	.uleb128 .Ltmp472-.Ltmp507
	.byte	0
	.byte	0
	.uleb128 .Ltmp472-.Lfunc_begin64
	.uleb128 .Ltmp517-.Ltmp472
	.uleb128 .Ltmp518-.Lfunc_begin64
	.byte	1
	.uleb128 .Ltmp517-.Lfunc_begin64
	.uleb128 .Ltmp452-.Ltmp517
	.byte	0
	.byte	0
	.uleb128 .Ltmp452-.Lfunc_begin64
	.uleb128 .Ltmp453-.Ltmp452
	.uleb128 .Ltmp454-.Lfunc_begin64
	.byte	1
	.uleb128 .Ltmp453-.Lfunc_begin64
	.uleb128 .Lfunc_end64-.Ltmp453
	.byte	0
	.byte	0
.Lcst_end30:
	.byte	127
	.byte	0
	.p2align	2, 0x0
.Lttbase20:
	.byte	0
	.p2align	2, 0x0

	.section	.text._RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtCs6a8jV7kq6PJ_10rayon_core9ErrorKindNtB6_5Debug3fmtBy_,"ax",@progbits
	.prefalign	4, .Lfunc_end65, nop
	.type	_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtCs6a8jV7kq6PJ_10rayon_core9ErrorKindNtB6_5Debug3fmtBy_,@function
_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtCs6a8jV7kq6PJ_10rayon_core9ErrorKindNtB6_5Debug3fmtBy_:
.Lfunc_begin65:
	.cfi_startproc
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
	subq	$56, %rsp
	.cfi_def_cfa_offset 112
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	(%rdi), %r12
	movq	%rsi, %r15
	movq	(%r12), %rax
	testq	%rax, %rax
	je	.LBB65_3
	cmpl	$1, %eax
	jne	.LBB65_5
	movq	(%r15), %rdi
	movq	8(%r15), %rax
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.124(%rip), %rsi
	movl	$26, %edx
	jmp	.LBB65_4
.LBB65_3:
	movq	(%r15), %rdi
	movq	8(%r15), %rax
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.123(%rip), %rsi
	movl	$28, %edx
.LBB65_4:
	addq	$56, %rsp
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
	jmpq	*24(%rax)
.LBB65_5:
	.cfi_def_cfa_offset 112
	movq	8(%r15), %rbp
	movq	(%r15), %r14
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.126(%rip), %rsi
	movl	$7, %edx
	movq	24(%rbp), %r13
	movq	%r14, %rdi
	callq	*%r13
	movb	$1, %bl
	testb	%al, %al
	je	.LBB65_6
.LBB65_14:
	movl	%ebx, %eax
	addq	$56, %rsp
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
.LBB65_6:
	.cfi_def_cfa_offset 112
	addq	$8, %r12
	testb	$-128, 18(%r15)
	jne	.LBB65_10
	leaq	anon.a647af78948fdeb0321157bdb82ef7e0.46.llvm.9794848731438112354(%rip), %rsi
	movl	$1, %edx
	movq	%r14, %rdi
	callq	*%r13
	testb	%al, %al
	jne	.LBB65_14
	movq	_RNvXs2_NtNtCs2k2z8Zem4rB_4core2io5errorNtNtB5_4repr4ReprNtNtB9_3fmt5Debug3fmt@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	movq	%r15, %rsi
	callq	*%rax
	testb	%al, %al
	jne	.LBB65_14
	movq	8(%r15), %rax
	movq	(%r15), %r14
	movq	24(%rax), %r13
	jmp	.LBB65_13
.LBB65_10:
	leaq	anon.a647af78948fdeb0321157bdb82ef7e0.47.llvm.9794848731438112354(%rip), %rsi
	movl	$2, %edx
	movq	%r14, %rdi
	callq	*%r13
	testb	%al, %al
	jne	.LBB65_14
	movq	16(%r15), %rcx
	leaq	7(%rsp), %rax
	movq	%r14, 32(%rsp)
	movq	%rbp, 40(%rsp)
	leaq	32(%rsp), %rdx
	leaq	8(%rsp), %rsi
	movb	$1, 7(%rsp)
	movq	%r12, %rdi
	movq	%rax, 48(%rsp)
	movq	_RNvXs2_NtNtCs2k2z8Zem4rB_4core2io5errorNtNtB5_4repr4ReprNtNtB9_3fmt5Debug3fmt@GOTPCREL(%rip), %rax
	movq	%rcx, 24(%rsp)
	leaq	anon.a647af78948fdeb0321157bdb82ef7e0.96.llvm.9794848731438112354(%rip), %rcx
	movq	%rdx, 8(%rsp)
	movq	%rcx, 16(%rsp)
	callq	*%rax
	testb	%al, %al
	jne	.LBB65_14
	movq	16(%rsp), %rax
	movq	8(%rsp), %rdi
	leaq	anon.a647af78948fdeb0321157bdb82ef7e0.42.llvm.9794848731438112354(%rip), %rsi
	movl	$2, %edx
	movq	24(%rax), %rax
	callq	*%rax
	testb	%al, %al
	jne	.LBB65_14
.LBB65_13:
	leaq	anon.a647af78948fdeb0321157bdb82ef7e0.45.llvm.9794848731438112354(%rip), %rsi
	movl	$1, %edx
	movq	%r14, %rdi
	callq	*%r13
	movl	%eax, %ebx
	jmp	.LBB65_14
.Lfunc_end65:
	.size	_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtCs6a8jV7kq6PJ_10rayon_core9ErrorKindNtB6_5Debug3fmtBy_, .Lfunc_end65-_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtCs6a8jV7kq6PJ_10rayon_core9ErrorKindNtB6_5Debug3fmtBy_
	.cfi_endproc

	.section	.text._RNvXs4_NtCs6a8jV7kq6PJ_10rayon_core5latchNtB5_9LockLatchNtB5_5Latch3set,"ax",@progbits
	.prefalign	4, .Lfunc_end66, nop
	.type	_RNvXs4_NtCs6a8jV7kq6PJ_10rayon_core5latchNtB5_9LockLatchNtB5_5Latch3set,@function
_RNvXs4_NtCs6a8jV7kq6PJ_10rayon_core5latchNtB5_9LockLatchNtB5_5Latch3set:
.Lfunc_begin66:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception31
	pushq	%rbp
	.cfi_def_cfa_offset 16
	pushq	%r15
	.cfi_def_cfa_offset 24
	pushq	%r14
	.cfi_def_cfa_offset 32
	pushq	%rbx
	.cfi_def_cfa_offset 40
	subq	$24, %rsp
	.cfi_def_cfa_offset 64
	.cfi_offset %rbx, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movl	$1, %ecx
	xorl	%eax, %eax
	movq	%rdi, %rbx
	lock		cmpxchgl	%ecx, (%rdi)
	jne	.LBB66_1
.LBB66_2:
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count18GLOBAL_PANIC_COUNT@GOTPCREL(%rip), %r14
	movq	(%r14), %rax
	shlq	%rax
	testq	%rax, %rax
	jne	.LBB66_4
	xorl	%ebp, %ebp
	movzbl	4(%rbx), %eax
	testb	%al, %al
	jne	.LBB66_6
.LBB66_11:
	movb	$1, 5(%rbx)
	movq	syscall@GOTPCREL(%rip), %r8
	leaq	8(%rbx), %rsi
	movl	$202, %edi
	movl	$129, %edx
	movl	$2147483647, %ecx
	xorl	%r15d, %r15d
	xorl	%eax, %eax
	lock		incl	8(%rbx)
	callq	*%r8
	testb	%bpl, %bpl
	jne	.LBB66_15
	movq	(%r14), %rax
	shlq	%rax
	testq	%rax, %rax
	jne	.LBB66_13
.LBB66_15:
	xchgl	%r15d, (%rbx)
	cmpl	$2, %r15d
	je	.LBB66_17
	addq	$24, %rsp
	.cfi_def_cfa_offset 40
	popq	%rbx
	.cfi_def_cfa_offset 32
	popq	%r14
	.cfi_def_cfa_offset 24
	popq	%r15
	.cfi_def_cfa_offset 16
	popq	%rbp
	.cfi_def_cfa_offset 8
	retq
.LBB66_1:
	.cfi_def_cfa_offset 64
	movq	_RNvMNtNtNtNtCs7jcFBdfocI9_3std3sys4sync5mutex5futexNtB2_5Mutex14lock_contended@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
	jmp	.LBB66_2
.LBB66_4:
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count17is_zero_slow_path@GOTPCREL(%rip), %rax
	callq	*%rax
	movl	%eax, %ebp
	xorb	$1, %bpl
	movzbl	4(%rbx), %eax
	testb	%al, %al
	je	.LBB66_11
.LBB66_6:
	movq	%rbx, 8(%rsp)
	movb	%bpl, 16(%rsp)
.Ltmp519:
	movq	_RNvNtCs2k2z8Zem4rB_4core6result13unwrap_failed@GOTPCREL(%rip), %rax
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.26.llvm.7294274987384275483(%rip), %rdi
	leaq	anon.f9c45627dbfd27b230899f0dc87ea5ab.25.llvm.7294274987384275483(%rip), %rcx
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.76(%rip), %r8
	leaq	8(%rsp), %rdx
	movl	$43, %esi
	callq	*%rax
.Ltmp520:
	ud2
.LBB66_17:
	movl	$202, %edi
	movq	%rbx, %rsi
	movl	$129, %edx
	movl	$1, %ecx
	xorl	%eax, %eax
	addq	$24, %rsp
	.cfi_def_cfa_offset 40
	popq	%rbx
	.cfi_def_cfa_offset 32
	popq	%r14
	.cfi_def_cfa_offset 24
	popq	%r15
	.cfi_def_cfa_offset 16
	popq	%rbp
	.cfi_def_cfa_offset 8
	jmpq	*syscall@GOTPCREL(%rip)
.LBB66_13:
	.cfi_def_cfa_offset 64
	movq	_RNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count17is_zero_slow_path@GOTPCREL(%rip), %rax
	callq	*%rax
	testb	%al, %al
	jne	.LBB66_15
	movb	$1, 4(%rbx)
	jmp	.LBB66_15
.LBB66_9:
.Ltmp521:
	movq	%rax, %rbx
.Ltmp522:
	leaq	8(%rsp), %rdi
	callq	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtCs7jcFBdfocI9_3std4sync6poison11PoisonErrorINtNtBE_5mutex10MutexGuardbEEECs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483
.Ltmp523:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB66_8:
.Ltmp524:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end66:
	.size	_RNvXs4_NtCs6a8jV7kq6PJ_10rayon_core5latchNtB5_9LockLatchNtB5_5Latch3set, .Lfunc_end66-_RNvXs4_NtCs6a8jV7kq6PJ_10rayon_core5latchNtB5_9LockLatchNtB5_5Latch3set
	.cfi_endproc
	.section	.gcc_except_table._RNvXs4_NtCs6a8jV7kq6PJ_10rayon_core5latchNtB5_9LockLatchNtB5_5Latch3set,"a",@progbits
	.p2align	2, 0x0
GCC_except_table66:
.Lexception31:
	.byte	255
	.byte	155
	.uleb128 .Lttbase21-.Lttbaseref21
.Lttbaseref21:
	.byte	1
	.uleb128 .Lcst_end31-.Lcst_begin31
.Lcst_begin31:
	.uleb128 .Lfunc_begin66-.Lfunc_begin66
	.uleb128 .Ltmp519-.Lfunc_begin66
	.byte	0
	.byte	0
	.uleb128 .Ltmp519-.Lfunc_begin66
	.uleb128 .Ltmp520-.Ltmp519
	.uleb128 .Ltmp521-.Lfunc_begin66
	.byte	0
	.uleb128 .Ltmp520-.Lfunc_begin66
	.uleb128 .Ltmp522-.Ltmp520
	.byte	0
	.byte	0
	.uleb128 .Ltmp522-.Lfunc_begin66
	.uleb128 .Ltmp523-.Ltmp522
	.uleb128 .Ltmp524-.Lfunc_begin66
	.byte	1
	.uleb128 .Ltmp523-.Lfunc_begin66
	.uleb128 .Lfunc_end66-.Ltmp523
	.byte	0
	.byte	0
.Lcst_end31:
	.byte	127
	.byte	0
	.p2align	2, 0x0
.Lttbase21:
	.byte	0
	.p2align	2, 0x0

	.section	.text._RNvXs6_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThreadINtNtCs2k2z8Zem4rB_4core7convert4FromNtB5_13ThreadBuilderE4from,"ax",@progbits
	.globl	_RNvXs6_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThreadINtNtCs2k2z8Zem4rB_4core7convert4FromNtB5_13ThreadBuilderE4from
	.prefalign	4, .Lfunc_end67, nop
	.type	_RNvXs6_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThreadINtNtCs2k2z8Zem4rB_4core7convert4FromNtB5_13ThreadBuilderE4from,@function
_RNvXs6_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThreadINtNtCs2k2z8Zem4rB_4core7convert4FromNtB5_13ThreadBuilderE4from:
.Lfunc_begin67:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception32
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
	subq	$72, %rsp
	.cfi_def_cfa_offset 128
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	vmovups	56(%rsi), %ymm0
	movq	40(%rsi), %rax
	movzbl	48(%rsi), %ebp
	movq	malloc@GOTPCREL(%rip), %r15
	movq	%rdi, %r14
	movl	$1520, %edi
	movq	%rsi, %rbx
	movl	$1520, %r12d
	movq	%rax, 16(%rsp)
	movq	%rax, 8(%rsp)
	movb	%bpl, 24(%rsp)
	vmovups	%ymm0, 32(%rsp)
	vzeroupper
	callq	*%r15
	testq	%rax, %rax
	je	.LBB67_24
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rsi
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rdx
	movq	$-1, %rcx
	movq	%rax, %r15
	incq	%rsi
	cmoveq	%rcx, %rsi
	addq	%r12, %rdx
	cmovbq	%rcx, %rdx
	addq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %r12
	movq	%rsi, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movq	%rdx, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	movabsq	$9223372036854775807, %rdx
	cmovoq	%rdx, %r12
	movq	%r12, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %r12
	jle	.LBB67_3
	movq	%r12, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
.LBB67_3:
	movb	%bpl, 7(%rsp)
	.p2align	4
.LBB67_4:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB67_10
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB67_4
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe19PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe23PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	$1520, (%rcx)
	movl	$1520, %ecx
	lock		xaddq	%rcx, (%rsi)
	addq	$1520, %rcx
	cmovoq	%rdx, %rcx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	movq	(%rdx), %rax
	.p2align	4
.LBB67_7:
	cmpq	%rax, %rcx
	jle	.LBB67_9
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB67_7
.LBB67_9:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB67_10:
	movq	memset@GOTPCREL(%rip), %rax
	movl	$1520, %edx
	movabsq	$576460752303423488, %r12
	movq	%r15, %rdi
	xorl	%esi, %esi
	callq	*%rax
	movq	96(%rbx), %rax
	movabsq	$8387220255154660723, %rcx
	movabsq	$7816392313619706465, %rdx
	movabsq	$-2389207006547353658, %rsi
	movabsq	$-6481707427168261424, %rdi
	movabsq	$-2011800112340241627, %r8
	.p2align	4
.LBB67_11:
	movl	$1, %r9d
	lock		xaddq	%r9, _RNvNvMs9_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB7_14XorShift64Star3new7COUNTER(%rip)
	movq	%r9, %r10
	xorq	%rcx, %r10
	leaq	(%r10,%rdx), %r11
	rorxq	$48, %r10, %r13
	addq	%rdi, %r10
	xorq	%r11, %r13
	rorxq	$32, %r10, %rbp
	xorq	%r8, %r10
	leaq	(%r13,%rsi), %r11
	rorxq	$43, %r13, %r13
	xorq	%r11, %r9
	xorq	%r11, %r13
	xorq	%r12, %r13
	addq	%r10, %r9
	rorxq	$51, %r10, %r10
	xorq	%r9, %r10
	addq	%r13, %rbp
	rorxq	$48, %r13, %r11
	rorxq	$32, %r9, %r9
	xorq	%rbp, %r11
	addq	%r10, %rbp
	rorxq	$47, %r10, %r10
	rorxq	$32, %rbp, %r13
	addq	%r11, %r9
	rorxq	$43, %r11, %r11
	xorq	%rbp, %r10
	xorq	%r9, %r11
	xorq	%r12, %r9
	xorq	$255, %r13
	addq	%r10, %r9
	rorxq	$51, %r10, %r10
	addq	%r11, %r13
	rorxq	$48, %r11, %r11
	xorq	%r9, %r10
	xorq	%r13, %r11
	rorxq	$32, %r9, %r9
	addq	%r10, %r13
	rorxq	$47, %r10, %r10
	addq	%r11, %r9
	rorxq	$43, %r11, %r11
	xorq	%r13, %r10
	xorq	%r9, %r11
	rorxq	$32, %r13, %r13
	addq	%r10, %r9
	rorxq	$51, %r10, %r10
	addq	%r11, %r13
	rorxq	$48, %r11, %r11
	xorq	%r9, %r10
	xorq	%r13, %r11
	rorxq	$32, %r9, %rbp
	addq	%r10, %r13
	rorxq	$47, %r10, %r10
	addq	%r11, %rbp
	rorxq	$43, %r11, %r11
	xorq	%r13, %r10
	xorq	%rbp, %r11
	rorxq	$32, %r13, %r9
	addq	%r10, %rbp
	rorxq	$51, %r10, %r10
	addq	%r11, %r9
	rorxq	$48, %r11, %r11
	xorq	%r9, %r11
	xorq	%rbp, %r10
	addq	%r10, %r9
	rorxq	$47, %r10, %r10
	rorxq	$43, %r11, %r11
	xorq	%r10, %r11
	rorxq	$32, %r9, %r10
	xorq	%r11, %r10
	cmpq	%r9, %r10
	je	.LBB67_11
	leaq	56(%rbx), %rdx
	movq	88(%rbx), %rcx
	movzbl	7(%rsp), %esi
	xorq	%r9, %r10
	vmovups	(%rdx), %ymm0
	movq	8(%rsp), %rdx
	vmovups	%ymm0, 280(%r14)
	movq	%rdx, 312(%r14)
	movb	%sil, 320(%r14)
	movq	$0, (%r14)
	movq	%r15, 8(%r14)
	movq	$0, 128(%r14)
	movq	%r15, 136(%r14)
	movq	%rax, 256(%r14)
	movq	%r10, 264(%r14)
	movq	%rcx, 272(%r14)
	movq	16(%rbx), %rcx
	cmpq	$-1, %rcx
	je	.LBB67_23
	testq	%rcx, %rcx
	je	.LBB67_23
	movq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	movq	24(%rbx), %rdi
	xorl	%edx, %edx
	movabsq	$9223372036854775807, %rsi
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	cmpq	%fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF, %rax
	jge	.LBB67_16
	movq	%rax, %fs:_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601@TPOFF
	.p2align	4
.LBB67_16:
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip), %rax
	testq	%rax, %rax
	jns	.LBB67_22
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, _RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
	jne	.LBB67_16
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe18PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe20PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB67_19:
	cmpq	%rax, %rdx
	jge	.LBB67_21
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB67_19
.LBB67_21:
	lock		decq	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601(%rip)
.LBB67_22:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB67_23:
	movq	%r14, %rax
	addq	$72, %rsp
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
.LBB67_24:
	.cfi_def_cfa_offset 128
.Ltmp525:
	movq	_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$1520, %esi
	callq	*%rax
.Ltmp526:
	ud2
.LBB67_26:
.Ltmp527:
	movq	8(%rsp), %rcx
	movq	%rax, %r14
	lock		decq	(%rcx)
	je	.LBB67_27
	movq	32(%rsp), %rax
	lock		decq	(%rax)
	je	.LBB67_29
.LBB67_30:
	movq	16(%rbx), %rsi
	cmpq	$-1, %rsi
	jne	.LBB67_31
	jmp	.LBB67_33
.LBB67_27:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %rax
	leaq	16(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
	movq	32(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB67_30
.LBB67_29:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcINtNtCs8qdfgzTqI2d_15crossbeam_utils12cache_padded11CachePaddedINtNtCs64OF0TycdZY_15crossbeam_deque5deque5InnerNtNtCs6a8jV7kq6PJ_10rayon_core3job6JobRefEEE9drop_slowB2x_@GOTPCREL(%rip), %rax
	leaq	32(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
	movq	16(%rbx), %rsi
	cmpq	$-1, %rsi
	je	.LBB67_33
.LBB67_31:
	testq	%rsi, %rsi
	je	.LBB67_33
	movq	24(%rbx), %rdi
	movl	$1, %edx
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
.LBB67_33:
	movq	88(%rbx), %rax
	lock		decq	(%rax)
	jne	.LBB67_35
	addq	$88, %rbx
	#MEMBARRIER
.Ltmp528:
	movq	_RNvMsn_NtCsc70TAahYccp_5alloc4syncINtB5_3ArcNtNtCs6a8jV7kq6PJ_10rayon_core8registry8RegistryE9drop_slowBK_@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp529:
.LBB67_35:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB67_36:
.Ltmp530:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end67:
	.size	_RNvXs6_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThreadINtNtCs2k2z8Zem4rB_4core7convert4FromNtB5_13ThreadBuilderE4from, .Lfunc_end67-_RNvXs6_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThreadINtNtCs2k2z8Zem4rB_4core7convert4FromNtB5_13ThreadBuilderE4from
	.cfi_endproc
	.section	.gcc_except_table._RNvXs6_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB5_12WorkerThreadINtNtCs2k2z8Zem4rB_4core7convert4FromNtB5_13ThreadBuilderE4from,"a",@progbits
	.p2align	2, 0x0
GCC_except_table67:
.Lexception32:
	.byte	255
	.byte	155
	.uleb128 .Lttbase22-.Lttbaseref22
.Lttbaseref22:
	.byte	1
	.uleb128 .Lcst_end32-.Lcst_begin32
.Lcst_begin32:
	.uleb128 .Lfunc_begin67-.Lfunc_begin67
	.uleb128 .Ltmp525-.Lfunc_begin67
	.byte	0
	.byte	0
	.uleb128 .Ltmp525-.Lfunc_begin67
	.uleb128 .Ltmp526-.Ltmp525
	.uleb128 .Ltmp527-.Lfunc_begin67
	.byte	0
	.uleb128 .Ltmp526-.Lfunc_begin67
	.uleb128 .Ltmp528-.Ltmp526
	.byte	0
	.byte	0
	.uleb128 .Ltmp528-.Lfunc_begin67
	.uleb128 .Ltmp529-.Ltmp528
	.uleb128 .Ltmp530-.Lfunc_begin67
	.byte	1
	.uleb128 .Ltmp529-.Lfunc_begin67
	.uleb128 .Lfunc_end67-.Ltmp529
	.byte	0
	.byte	0
.Lcst_end32:
	.byte	127
	.byte	0
	.p2align	2, 0x0
.Lttbase22:
	.byte	0
	.p2align	2, 0x0

	.section	.text._RNvXs_NtNtCs7jcFBdfocI9_3std4sync6poisonINtB4_11PoisonErrorINtNtB4_5mutex10MutexGuardbEENtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmtCs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483,"ax",@progbits
	.hidden	_RNvXs_NtNtCs7jcFBdfocI9_3std4sync6poisonINtB4_11PoisonErrorINtNtB4_5mutex10MutexGuardbEENtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmtCs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483
	.globl	_RNvXs_NtNtCs7jcFBdfocI9_3std4sync6poisonINtB4_11PoisonErrorINtNtB4_5mutex10MutexGuardbEENtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmtCs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483
	.prefalign	4, .Lfunc_end68, nop
	.type	_RNvXs_NtNtCs7jcFBdfocI9_3std4sync6poisonINtB4_11PoisonErrorINtNtB4_5mutex10MutexGuardbEENtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmtCs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483,@function
_RNvXs_NtNtCs7jcFBdfocI9_3std4sync6poisonINtB4_11PoisonErrorINtNtB4_5mutex10MutexGuardbEENtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmtCs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483:
.Lfunc_begin68:
	.cfi_startproc
	pushq	%r14
	.cfi_def_cfa_offset 16
	pushq	%rbx
	.cfi_def_cfa_offset 24
	pushq	%rax
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -24
	.cfi_offset %r14, -16
	movq	8(%rsi), %rax
	movq	(%rsi), %rbx
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.119(%rip), %rsi
	movl	$11, %edx
	movq	24(%rax), %rax
	movq	%rbx, %rdi
	movq	%rax, %r14
	callq	*%rax
	testb	%al, %al
	je	.LBB68_2
	movb	$1, %al
	addq	$8, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	retq
.LBB68_2:
	.cfi_def_cfa_offset 32
	leaq	anon.a647af78948fdeb0321157bdb82ef7e0.34.llvm.9794848731438112354(%rip), %rsi
	movl	$7, %edx
	movq	%rbx, %rdi
	movq	%r14, %rax
	addq	$8, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*%rax
.Lfunc_end68:
	.size	_RNvXs_NtNtCs7jcFBdfocI9_3std4sync6poisonINtB4_11PoisonErrorINtNtB4_5mutex10MutexGuardbEENtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmtCs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483, .Lfunc_end68-_RNvXs_NtNtCs7jcFBdfocI9_3std4sync6poisonINtB4_11PoisonErrorINtNtB4_5mutex10MutexGuardbEENtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmtCs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483
	.cfi_endproc

	.section	.text._RNvXsb_Cs6a8jV7kq6PJ_10rayon_coreNtB5_20ThreadPoolBuildErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.llvm.7294274987384275483,"ax",@progbits
	.hidden	_RNvXsb_Cs6a8jV7kq6PJ_10rayon_coreNtB5_20ThreadPoolBuildErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.llvm.7294274987384275483
	.globl	_RNvXsb_Cs6a8jV7kq6PJ_10rayon_coreNtB5_20ThreadPoolBuildErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.llvm.7294274987384275483
	.prefalign	4, .Lfunc_end69, nop
	.type	_RNvXsb_Cs6a8jV7kq6PJ_10rayon_coreNtB5_20ThreadPoolBuildErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.llvm.7294274987384275483,@function
_RNvXsb_Cs6a8jV7kq6PJ_10rayon_coreNtB5_20ThreadPoolBuildErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.llvm.7294274987384275483:
.Lfunc_begin69:
	.cfi_startproc
	pushq	%rbx
	.cfi_def_cfa_offset 16
	subq	$32, %rsp
	.cfi_def_cfa_offset 48
	.cfi_offset %rbx, -16
	movq	8(%rsi), %rax
	movq	%rdi, 24(%rsp)
	movq	(%rsi), %rdi
	movq	%rsi, %rbx
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.121(%rip), %rsi
	movl	$20, %edx
	movq	24(%rax), %rax
	callq	*%rax
	movq	%rbx, 8(%rsp)
	movq	_RNvMs2_NtNtCs2k2z8Zem4rB_4core3fmt8buildersNtB5_11DebugStruct5field@GOTPCREL(%rip), %rbx
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.122(%rip), %rsi
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.120(%rip), %r8
	leaq	8(%rsp), %rdi
	leaq	24(%rsp), %rcx
	movl	$4, %edx
	movb	%al, 16(%rsp)
	movb	$0, 17(%rsp)
	callq	*%rbx
	movzbl	17(%rsp), %eax
	movzbl	16(%rsp), %ecx
	movl	%eax, %edx
	notb	%dl
	orb	%cl, %dl
	testb	$1, %dl
	je	.LBB69_2
	orb	%cl, %al
	andb	$1, %al
	addq	$32, %rsp
	.cfi_def_cfa_offset 16
	popq	%rbx
	.cfi_def_cfa_offset 8
	retq
.LBB69_2:
	.cfi_def_cfa_offset 48
	movq	8(%rsp), %rax
	testb	$-128, 18(%rax)
	jne	.LBB69_4
	movq	(%rax), %rdi
	movq	8(%rax), %rax
	leaq	anon.a647af78948fdeb0321157bdb82ef7e0.91.llvm.9794848731438112354(%rip), %rsi
	movl	$2, %edx
	jmp	.LBB69_5
.LBB69_4:
	movq	(%rax), %rdi
	movq	8(%rax), %rax
	leaq	anon.a647af78948fdeb0321157bdb82ef7e0.37.llvm.9794848731438112354(%rip), %rsi
	movl	$1, %edx
.LBB69_5:
	movq	24(%rax), %rax
	callq	*%rax
	andb	$1, %al
	addq	$32, %rsp
	.cfi_def_cfa_offset 16
	popq	%rbx
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end69:
	.size	_RNvXsb_Cs6a8jV7kq6PJ_10rayon_coreNtB5_20ThreadPoolBuildErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.llvm.7294274987384275483, .Lfunc_end69-_RNvXsb_Cs6a8jV7kq6PJ_10rayon_coreNtB5_20ThreadPoolBuildErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.llvm.7294274987384275483
	.cfi_endproc

	.section	.rodata.cst16,"aM",@progbits,16
	.p2align	4, 0x0
.LCPI70_0:
	.long	18
	.long	12
	.long	6
	.long	0
.LCPI70_1:
	.byte	240
	.byte	128
	.byte	128
	.byte	128
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
.LCPI70_2:
	.byte	255
	.byte	63
	.byte	63
	.byte	63
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
.LCPI70_3:
	.byte	240
	.byte	128
	.byte	128
	.byte	128
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.section	.rodata.cst4,"aM",@progbits,4
	.p2align	2, 0x0
.LCPI70_4:
	.byte	255
	.byte	63
	.byte	63
	.byte	63
	.section	.text._RNvYINtNvNtNtCs2k2z8Zem4rB_4core2io5write17default_write_fmt7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrENtNtBb_3fmt5Write10write_charCs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.prefalign	4, .Lfunc_end70, nop
	.type	_RNvYINtNvNtNtCs2k2z8Zem4rB_4core2io5write17default_write_fmt7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrENtNtBb_3fmt5Write10write_charCs6a8jV7kq6PJ_10rayon_core,@function
_RNvYINtNvNtNtCs2k2z8Zem4rB_4core2io5write17default_write_fmt7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrENtNtBb_3fmt5Write10write_charCs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin70:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception33
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
	subq	$24, %rsp
	.cfi_def_cfa_offset 80
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movl	$0, 12(%rsp)
	movq	%rdi, 16(%rsp)
	cmpl	$128, %esi
	jae	.LBB70_1
	movb	%sil, 12(%rsp)
	movl	$1, %r14d
	jmp	.LBB70_6
.LBB70_1:
	vpbroadcastd	%esi, %xmm0
	vpsrlvd	.LCPI70_0(%rip), %xmm0, %xmm0
	vpmovdb	%xmm0, %xmm1
	vmovd	.LCPI70_3(%rip), %xmm0
	vpternlogd	$248, .LCPI70_4(%rip){1to4}, %xmm1, %xmm0
	cmpl	$2048, %esi
	jae	.LBB70_3
	vpextrb	$2, %xmm1, %eax
	movl	$2, %r14d
	orb	$-64, %al
	movb	%al, 12(%rsp)
	vpextrb	$3, %xmm0, 13(%rsp)
	jmp	.LBB70_6
.LBB70_3:
	cmpl	$65535, %esi
	ja	.LBB70_5
	vpextrb	$1, %xmm1, %eax
	movl	$3, %r14d
	orb	$-32, %al
	movb	%al, 12(%rsp)
	vpextrb	$2, %xmm0, 13(%rsp)
	vpextrb	$3, %xmm0, 14(%rsp)
	jmp	.LBB70_6
.LBB70_5:
	movl	$4, %r14d
	vmovd	%xmm0, 12(%rsp)
.LBB70_6:
	movq	write@GOTPCREL(%rip), %r13
	movq	_RNvNtNtNtCs2k2z8Zem4rB_4core2io5error12os_functions12OS_FUNCTIONS@GOTPCREL(%rip), %rbx
	leaq	12(%rsp), %r15
	leaq	anon.f13bb3f258876cb12f4a84b0a5cdace5.73.llvm.11640361436736466388(%rip), %rbp
	jmp	.LBB70_7
	.p2align	4
.LBB70_16:
	testq	%r14, %r14
	je	.LBB70_17
.LBB70_7:
	movl	$2, %edi
	movq	%r15, %rsi
	movq	%r14, %rdx
	callq	*%r13
	cmpq	$-1, %rax
	je	.LBB70_11
	testq	%rax, %rax
	je	.LBB70_18
	movq	%r14, %rcx
	subq	%rax, %rcx
	jb	.LBB70_10
	addq	%rax, %r15
	movq	%rcx, %r14
	jmp	.LBB70_16
	.p2align	4
.LBB70_11:
	movq	__errno_location@GOTPCREL(%rip), %rax
	callq	*%rax
	movslq	(%rax), %r12
	movq	(%rbx), %rax
	cmpq	%rbp, %rax
	jne	.LBB70_12
.LBB70_13:
	movq	(%rbx), %rax
	movl	%r12d, %edi
	movq	16(%rax), %rax
	callq	*%rax
	testb	%al, %al
	jne	.LBB70_16
	jmp	.LBB70_14
.LBB70_12:
	movq	%rbp, (%rbx)
	jmp	.LBB70_13
.LBB70_17:
	xorl	%eax, %eax
	jmp	.LBB70_24
.LBB70_18:
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.144(%rip), %r12
	movq	16(%rsp), %rbx
	movq	8(%rbx), %rdi
	testq	%rdi, %rdi
	jne	.LBB70_20
	jmp	.LBB70_23
.LBB70_14:
	shlq	$32, %r12
	orq	$2, %r12
	movq	16(%rsp), %rbx
	movq	8(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB70_23
.LBB70_20:
	movl	%edi, %eax
	andl	$3, %eax
	leal	-2(%rax), %ecx
	cmpl	$2, %ecx
	jb	.LBB70_23
	testq	%rax, %rax
	je	.LBB70_23
	movq	23(%rdi), %rax
	decq	%rdi
.Ltmp531:
	callq	*%rax
.Ltmp532:
.LBB70_23:
	movb	$1, %al
	movq	%r12, 8(%rbx)
.LBB70_24:
	addq	$24, %rsp
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
.LBB70_10:
	.cfi_def_cfa_offset 80
	movq	_RNvNtNtCs2k2z8Zem4rB_4core5slice5index16slice_index_fail@GOTPCREL(%rip), %r8
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.145(%rip), %rcx
	movq	%rax, %rdi
	movq	%r14, %rsi
	movq	%r14, %rdx
	callq	*%r8
.LBB70_26:
.Ltmp533:
	movq	%rax, %rdi
	movq	%r12, 8(%rbx)
	callq	_Unwind_Resume@PLT
.Lfunc_end70:
	.size	_RNvYINtNvNtNtCs2k2z8Zem4rB_4core2io5write17default_write_fmt7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrENtNtBb_3fmt5Write10write_charCs6a8jV7kq6PJ_10rayon_core, .Lfunc_end70-_RNvYINtNvNtNtCs2k2z8Zem4rB_4core2io5write17default_write_fmt7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrENtNtBb_3fmt5Write10write_charCs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc
	.section	.gcc_except_table._RNvYINtNvNtNtCs2k2z8Zem4rB_4core2io5write17default_write_fmt7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrENtNtBb_3fmt5Write10write_charCs6a8jV7kq6PJ_10rayon_core,"a",@progbits
	.p2align	2, 0x0
GCC_except_table70:
.Lexception33:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end33-.Lcst_begin33
.Lcst_begin33:
	.uleb128 .Lfunc_begin70-.Lfunc_begin70
	.uleb128 .Ltmp531-.Lfunc_begin70
	.byte	0
	.byte	0
	.uleb128 .Ltmp531-.Lfunc_begin70
	.uleb128 .Ltmp532-.Ltmp531
	.uleb128 .Ltmp533-.Lfunc_begin70
	.byte	0
	.uleb128 .Ltmp532-.Lfunc_begin70
	.uleb128 .Lfunc_end70-.Ltmp532
	.byte	0
	.byte	0
.Lcst_end33:
	.p2align	2, 0x0

	.section	.text._RNvYINtNvNtNtCs2k2z8Zem4rB_4core2io5write17default_write_fmt7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrENtNtBb_3fmt5Write9write_fmtCs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.prefalign	4, .Lfunc_end71, nop
	.type	_RNvYINtNvNtNtCs2k2z8Zem4rB_4core2io5write17default_write_fmt7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrENtNtBb_3fmt5Write9write_fmtCs6a8jV7kq6PJ_10rayon_core,@function
_RNvYINtNvNtNtCs2k2z8Zem4rB_4core2io5write17default_write_fmt7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrENtNtBb_3fmt5Write9write_fmtCs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin71:
	.cfi_startproc
	movq	%rdx, %rcx
	movq	%rsi, %rdx
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.10(%rip), %rsi
	jmpq	*_RNvNtCs2k2z8Zem4rB_4core3fmt5write@GOTPCREL(%rip)
.Lfunc_end71:
	.size	_RNvYINtNvNtNtCs2k2z8Zem4rB_4core2io5write17default_write_fmt7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrENtNtBb_3fmt5Write9write_fmtCs6a8jV7kq6PJ_10rayon_core, .Lfunc_end71-_RNvYINtNvNtNtCs2k2z8Zem4rB_4core2io5write17default_write_fmt7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrENtNtBb_3fmt5Write9write_fmtCs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc

	.section	.text._RNvYNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrNtNtNtCs2k2z8Zem4rB_4core2io5write5Write9write_allCs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.prefalign	4, .Lfunc_end72, nop
	.type	_RNvYNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrNtNtNtCs2k2z8Zem4rB_4core2io5write5Write9write_allCs6a8jV7kq6PJ_10rayon_core,@function
_RNvYNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrNtNtNtCs2k2z8Zem4rB_4core2io5write5Write9write_allCs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin72:
	.cfi_startproc
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
	pushq	%rax
	.cfi_def_cfa_offset 64
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	testq	%rsi, %rsi
	je	.LBB72_11
	movq	write@GOTPCREL(%rip), %r13
	movq	_RNvNtNtNtCs2k2z8Zem4rB_4core2io5error12os_functions12OS_FUNCTIONS@GOTPCREL(%rip), %rbx
	leaq	anon.f13bb3f258876cb12f4a84b0a5cdace5.73.llvm.11640361436736466388(%rip), %rbp
	movq	%rsi, %r14
	movq	%rdi, %r15
	jmp	.LBB72_3
	.p2align	4
.LBB72_2:
	testq	%r14, %r14
	je	.LBB72_11
.LBB72_3:
	movl	$2, %edi
	movq	%r15, %rsi
	movq	%r14, %rdx
	callq	*%r13
	cmpq	$-1, %rax
	je	.LBB72_7
	testq	%rax, %rax
	je	.LBB72_12
	movq	%r14, %rcx
	subq	%rax, %rcx
	jb	.LBB72_15
	addq	%rax, %r15
	movq	%rcx, %r14
	jmp	.LBB72_2
	.p2align	4
.LBB72_7:
	movq	__errno_location@GOTPCREL(%rip), %rax
	callq	*%rax
	movslq	(%rax), %r12
	movq	(%rbx), %rax
	cmpq	%rbp, %rax
	jne	.LBB72_9
.LBB72_8:
	movq	(%rbx), %rax
	movl	%r12d, %edi
	movq	16(%rax), %rax
	callq	*%rax
	testb	%al, %al
	jne	.LBB72_2
	jmp	.LBB72_13
.LBB72_9:
	movq	%rbp, (%rbx)
	jmp	.LBB72_8
.LBB72_11:
	xorl	%eax, %eax
.LBB72_14:
	addq	$8, %rsp
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
.LBB72_12:
	.cfi_def_cfa_offset 64
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.144(%rip), %rax
	jmp	.LBB72_14
.LBB72_13:
	shlq	$32, %r12
	orq	$2, %r12
	movq	%r12, %rax
	jmp	.LBB72_14
.LBB72_15:
	movq	_RNvNtNtCs2k2z8Zem4rB_4core5slice5index16slice_index_fail@GOTPCREL(%rip), %r8
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.145(%rip), %rcx
	movq	%rax, %rdi
	movq	%r14, %rsi
	movq	%r14, %rdx
	callq	*%r8
.Lfunc_end72:
	.size	_RNvYNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrNtNtNtCs2k2z8Zem4rB_4core2io5write5Write9write_allCs6a8jV7kq6PJ_10rayon_core, .Lfunc_end72-_RNvYNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrNtNtNtCs2k2z8Zem4rB_4core2io5write5Write9write_allCs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc

	.section	.text._RNvYNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrNtNtNtCs2k2z8Zem4rB_4core2io5write5Write9write_fmtCs6a8jV7kq6PJ_10rayon_core,"ax",@progbits
	.prefalign	4, .Lfunc_end73, nop
	.type	_RNvYNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrNtNtNtCs2k2z8Zem4rB_4core2io5write5Write9write_fmtCs6a8jV7kq6PJ_10rayon_core,@function
_RNvYNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrNtNtNtCs2k2z8Zem4rB_4core2io5write5Write9write_fmtCs6a8jV7kq6PJ_10rayon_core:
.Lfunc_begin73:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception34
	pushq	%rbx
	.cfi_def_cfa_offset 16
	subq	$16, %rsp
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -16
	movq	%rdx, %rcx
	movq	%rsi, %rdx
	movq	%rdi, (%rsp)
	movq	$0, 8(%rsp)
.Ltmp534:
	movq	_RNvNtCs2k2z8Zem4rB_4core3fmt5write@GOTPCREL(%rip), %rax
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.10(%rip), %rsi
	movq	%rsp, %rdi
	callq	*%rax
.Ltmp535:
	movq	8(%rsp), %rdi
	testb	%al, %al
	je	.LBB73_10
	movq	%rdi, %rax
	testq	%rdi, %rdi
	je	.LBB73_3
.LBB73_15:
	addq	$16, %rsp
	.cfi_def_cfa_offset 16
	popq	%rbx
	.cfi_def_cfa_offset 8
	retq
.LBB73_10:
	.cfi_def_cfa_offset 32
	testq	%rdi, %rdi
	je	.LBB73_14
	movl	%edi, %ecx
	andl	$3, %ecx
	xorl	%eax, %eax
	leal	-2(%rcx), %edx
	cmpl	$2, %edx
	jb	.LBB73_15
	testq	%rcx, %rcx
	je	.LBB73_15
	movq	23(%rdi), %rax
	decq	%rdi
	callq	*%rax
.LBB73_14:
	xorl	%eax, %eax
	addq	$16, %rsp
	.cfi_def_cfa_offset 16
	popq	%rbx
	.cfi_def_cfa_offset 8
	retq
.LBB73_3:
	.cfi_def_cfa_offset 32
.Ltmp536:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking9panic_fmt@GOTPCREL(%rip), %rax
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.11(%rip), %rdi
	leaq	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.13(%rip), %rdx
	movl	$173, %esi
	callq	*%rax
.Ltmp537:
	ud2
.LBB73_5:
.Ltmp538:
	movq	8(%rsp), %rdi
	testq	%rdi, %rdi
	je	.LBB73_9
	movl	%edi, %ecx
	andl	$3, %ecx
	leal	-2(%rcx), %edx
	cmpl	$2, %edx
	jb	.LBB73_9
	testq	%rcx, %rcx
	je	.LBB73_9
	movq	23(%rdi), %rcx
	decq	%rdi
.Ltmp539:
	movq	%rax, %rbx
	callq	*%rcx
	movq	%rbx, %rax
.Ltmp540:
.LBB73_9:
	movq	%rax, %rdi
	callq	_Unwind_Resume@PLT
.LBB73_16:
.Ltmp541:
	movq	_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end73:
	.size	_RNvYNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrNtNtNtCs2k2z8Zem4rB_4core2io5write5Write9write_fmtCs6a8jV7kq6PJ_10rayon_core, .Lfunc_end73-_RNvYNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrNtNtNtCs2k2z8Zem4rB_4core2io5write5Write9write_fmtCs6a8jV7kq6PJ_10rayon_core
	.cfi_endproc
	.section	.gcc_except_table._RNvYNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrNtNtNtCs2k2z8Zem4rB_4core2io5write5Write9write_fmtCs6a8jV7kq6PJ_10rayon_core,"a",@progbits
	.p2align	2, 0x0
GCC_except_table73:
.Lexception34:
	.byte	255
	.byte	155
	.uleb128 .Lttbase23-.Lttbaseref23
.Lttbaseref23:
	.byte	1
	.uleb128 .Lcst_end34-.Lcst_begin34
.Lcst_begin34:
	.uleb128 .Ltmp534-.Lfunc_begin73
	.uleb128 .Ltmp535-.Ltmp534
	.uleb128 .Ltmp538-.Lfunc_begin73
	.byte	0
	.uleb128 .Ltmp535-.Lfunc_begin73
	.uleb128 .Ltmp536-.Ltmp535
	.byte	0
	.byte	0
	.uleb128 .Ltmp536-.Lfunc_begin73
	.uleb128 .Ltmp537-.Ltmp536
	.uleb128 .Ltmp538-.Lfunc_begin73
	.byte	0
	.uleb128 .Ltmp539-.Lfunc_begin73
	.uleb128 .Ltmp540-.Ltmp539
	.uleb128 .Ltmp541-.Lfunc_begin73
	.byte	1
	.uleb128 .Ltmp540-.Lfunc_begin73
	.uleb128 .Lfunc_end73-.Ltmp540
	.byte	0
	.byte	0
.Lcst_end34:
	.byte	127
	.byte	0
	.p2align	2, 0x0
.Lttbase23:
	.byte	0
	.p2align	2, 0x0

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.0,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.0:
	.asciz	"/kache/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rayon-core-1.13.0/src/sleep/mod.rs"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.0, 95

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.1,@object
	.section	.data.rel.ro..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.1,"aw",@progbits
	.p2align	3, 0x0
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.1:
	.quad	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.0
	.asciz	"^\000\000\000\000\000\000\000\202\000\000\0004\000\000"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.1, 24

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.2,@object
	.section	.data.rel.ro..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.2,"aw",@progbits
	.p2align	3, 0x0
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.2:
	.quad	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.0
	.asciz	"^\000\000\000\000\000\000\000\203\000\000\000<\000\000"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.2, 24

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.3,@object
	.section	.data.rel.ro..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.3,"aw",@progbits
	.p2align	3, 0x0
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.3:
	.quad	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.0
	.asciz	"^\000\000\000\000\000\000\000\273\000\000\000C\000\000"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.3, 24

	.hidden	anon.f9c45627dbfd27b230899f0dc87ea5ab.5.llvm.7294274987384275483
	.type	anon.f9c45627dbfd27b230899f0dc87ea5ab.5.llvm.7294274987384275483,@object
	.section	.data.rel.ro.anon.f9c45627dbfd27b230899f0dc87ea5ab.5.llvm.7294274987384275483,"aw",@progbits
	.globl	anon.f9c45627dbfd27b230899f0dc87ea5ab.5.llvm.7294274987384275483
	.p2align	3, 0x0
anon.f9c45627dbfd27b230899f0dc87ea5ab.5.llvm.7294274987384275483:
	.asciz	"\000\000\000\000\000\000\000\000\b\000\000\000\000\000\000\000\b\000\000\000\000\000\000"
	.quad	_RNSNvYNCINvMs0_NtNtCs7jcFBdfocI9_3std4sync4onceNtBd_4Once9call_onceNCINvNtCs6a8jV7kq6PJ_10rayon_core8registry19set_global_registryNvB18_23default_global_registryE0E0INtNtNtCs2k2z8Zem4rB_4core3ops8function6FnOnceTRNtBd_9OnceStateEE9call_once6vtableB1a_.llvm.7294274987384275483
	.quad	_RNCINvMs0_NtNtCs7jcFBdfocI9_3std4sync4onceNtB8_4Once9call_onceNCINvNtCs6a8jV7kq6PJ_10rayon_core8registry19set_global_registryNvB13_23default_global_registryE0E0B15_.llvm.7294274987384275483
	.size	anon.f9c45627dbfd27b230899f0dc87ea5ab.5.llvm.7294274987384275483, 40

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.6,@object
	.section	.rodata..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.6,"a",@progbits
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.6:
	.ascii	"fatal runtime error: unreachable, aborting\n"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.6, 43

	.hidden	anon.f9c45627dbfd27b230899f0dc87ea5ab.7.llvm.7294274987384275483
	.type	anon.f9c45627dbfd27b230899f0dc87ea5ab.7.llvm.7294274987384275483,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
	.globl	anon.f9c45627dbfd27b230899f0dc87ea5ab.7.llvm.7294274987384275483
anon.f9c45627dbfd27b230899f0dc87ea5ab.7.llvm.7294274987384275483:
	.asciz	"/kache/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rayon-core-1.13.0/src/registry.rs"
	.size	anon.f9c45627dbfd27b230899f0dc87ea5ab.7.llvm.7294274987384275483, 94

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.8,@object
	.section	.data.rel.ro..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.8,"aw",@progbits
	.p2align	3, 0x0
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.8:
	.quad	anon.f9c45627dbfd27b230899f0dc87ea5ab.7.llvm.7294274987384275483
	.asciz	"]\000\000\000\000\000\000\0004\001\000\0006\000\000"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.8, 24

	.hidden	anon.f9c45627dbfd27b230899f0dc87ea5ab.9.llvm.7294274987384275483
	.type	anon.f9c45627dbfd27b230899f0dc87ea5ab.9.llvm.7294274987384275483,@object
	.section	.data.rel.ro.anon.f9c45627dbfd27b230899f0dc87ea5ab.9.llvm.7294274987384275483,"aw",@progbits
	.globl	anon.f9c45627dbfd27b230899f0dc87ea5ab.9.llvm.7294274987384275483
	.p2align	3, 0x0
anon.f9c45627dbfd27b230899f0dc87ea5ab.9.llvm.7294274987384275483:
	.quad	anon.f9c45627dbfd27b230899f0dc87ea5ab.7.llvm.7294274987384275483
	.asciz	"]\000\000\000\000\000\000\000\303\000\000\000\026\000\000"
	.size	anon.f9c45627dbfd27b230899f0dc87ea5ab.9.llvm.7294274987384275483, 24

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.10,@object
	.section	.data.rel.ro..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.10,"aw",@progbits
	.p2align	3, 0x0
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.10:
	.quad	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNvNtNtB4_2io5write17default_write_fmt7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrEECs6a8jV7kq6PJ_10rayon_core
	.asciz	"\020\000\000\000\000\000\000\000\b\000\000\000\000\000\000"
	.quad	_RNvXNvNtNtCs2k2z8Zem4rB_4core2io5write17default_write_fmtINtB2_7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrENtNtB8_3fmt5Write9write_strCs6a8jV7kq6PJ_10rayon_core
	.quad	_RNvYINtNvNtNtCs2k2z8Zem4rB_4core2io5write17default_write_fmt7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrENtNtBb_3fmt5Write10write_charCs6a8jV7kq6PJ_10rayon_core
	.quad	_RNvYINtNvNtNtCs2k2z8Zem4rB_4core2io5write17default_write_fmt7AdapterNtNtNtNtCs7jcFBdfocI9_3std3sys5stdio4unix6StderrENtNtBb_3fmt5Write9write_fmtCs6a8jV7kq6PJ_10rayon_core
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.10, 48

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.11,@object
	.section	.rodata..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.11,"a",@progbits
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.11:
	.ascii	"a formatting trait implementation returned an error when the underlying stream did not"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.11, 86

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.12,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.12:
	.asciz	"/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad/library/core/src/io/write.rs"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.12, 77

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.13,@object
	.section	.data.rel.ro..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.13,"aw",@progbits
	.p2align	3, 0x0
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.13:
	.quad	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.12
	.asciz	"L\000\000\000\000\000\000\000\233\001\000\000\021\000\000"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.13, 24

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.14,@object
	.section	.data.rel.ro..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.14,"aw",@progbits
	.p2align	3, 0x0
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.14:
	.quad	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNCINvNtNtCs7jcFBdfocI9_3std6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1I_12DefaultSpawnNtB1I_11ThreadSpawn5spawn0uEs_0EB1K_
	.asciz	"\220\000\000\000\000\000\000\000\b\000\000\000\000\000\000"
	.quad	_RNSNvYNCINvNtNtCs7jcFBdfocI9_3std6thread9lifecycle15spawn_uncheckedNCNvXs0_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB1b_12DefaultSpawnNtB1b_11ThreadSpawn5spawn0uEs_0INtNtNtCs2k2z8Zem4rB_4core3ops8function6FnOnceuE9call_once6vtableB1d_
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.14, 32

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.15,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.15:
	.asciz	"/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad/library/std/src/sync/once.rs"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.15, 77

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.16,@object
	.section	.data.rel.ro..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.16,"aw",@progbits
	.p2align	3, 0x0
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.16:
	.quad	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.15
	.asciz	"L\000\000\000\000\000\000\000\247\000\000\0002\000\000"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.16, 24

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.17,@object
	.section	.rodata..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.17,"a",@progbits
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.17:
	.ascii	"RUST_MIN_STACK"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.17, 14

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.18,@object
	.section	.rodata..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.18,"a",@progbits
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.18:
	.ascii	"assertion failed: t.get().is_null()"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.18, 35

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.19,@object
	.section	.data.rel.ro..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.19,"aw",@progbits
	.p2align	3, 0x0
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.19:
	.quad	anon.f9c45627dbfd27b230899f0dc87ea5ab.7.llvm.7294274987384275483
	.asciz	"]\000\000\000\000\000\000\000\306\002\000\000\r\000\000"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.19, 24

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.20,@object
	.section	.data.rel.ro..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.20,"aw",@progbits
	.p2align	3, 0x0
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.20:
	.quad	anon.f9c45627dbfd27b230899f0dc87ea5ab.7.llvm.7294274987384275483
	.asciz	"]\000\000\000\000\000\000\000{\003\000\000#\000\000"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.20, 24

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.21,@object
	.section	.rodata..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.21,"a",@progbits
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.21:
	.ascii	"assertion failed: t.get().eq(&(self as *const _))"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.21, 49

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.22,@object
	.section	.data.rel.ro..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.22,"aw",@progbits
	.p2align	3, 0x0
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.22:
	.quad	anon.f9c45627dbfd27b230899f0dc87ea5ab.7.llvm.7294274987384275483
	.asciz	"]\000\000\000\000\000\000\000\263\002\000\000\r\000\000"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.22, 24

	.hidden	anon.f9c45627dbfd27b230899f0dc87ea5ab.25.llvm.7294274987384275483
	.type	anon.f9c45627dbfd27b230899f0dc87ea5ab.25.llvm.7294274987384275483,@object
	.section	.data.rel.ro.anon.f9c45627dbfd27b230899f0dc87ea5ab.25.llvm.7294274987384275483,"aw",@progbits
	.globl	anon.f9c45627dbfd27b230899f0dc87ea5ab.25.llvm.7294274987384275483
	.p2align	3, 0x0
anon.f9c45627dbfd27b230899f0dc87ea5ab.25.llvm.7294274987384275483:
	.quad	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtNtCs7jcFBdfocI9_3std4sync6poison11PoisonErrorINtNtBE_5mutex10MutexGuardbEEECs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483
	.asciz	"\020\000\000\000\000\000\000\000\b\000\000\000\000\000\000"
	.quad	_RNvXs_NtNtCs7jcFBdfocI9_3std4sync6poisonINtB4_11PoisonErrorINtNtB4_5mutex10MutexGuardbEENtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmtCs6a8jV7kq6PJ_10rayon_core.llvm.7294274987384275483
	.size	anon.f9c45627dbfd27b230899f0dc87ea5ab.25.llvm.7294274987384275483, 32

	.hidden	anon.f9c45627dbfd27b230899f0dc87ea5ab.26.llvm.7294274987384275483
	.type	anon.f9c45627dbfd27b230899f0dc87ea5ab.26.llvm.7294274987384275483,@object
	.section	.rodata.anon.f9c45627dbfd27b230899f0dc87ea5ab.26.llvm.7294274987384275483,"a",@progbits
	.globl	anon.f9c45627dbfd27b230899f0dc87ea5ab.26.llvm.7294274987384275483
anon.f9c45627dbfd27b230899f0dc87ea5ab.26.llvm.7294274987384275483:
	.ascii	"called `Result::unwrap()` on an `Err` value"
	.size	anon.f9c45627dbfd27b230899f0dc87ea5ab.26.llvm.7294274987384275483, 43

	.hidden	anon.f9c45627dbfd27b230899f0dc87ea5ab.27.llvm.7294274987384275483
	.type	anon.f9c45627dbfd27b230899f0dc87ea5ab.27.llvm.7294274987384275483,@object
	.section	.data.rel.ro.anon.f9c45627dbfd27b230899f0dc87ea5ab.27.llvm.7294274987384275483,"aw",@progbits
	.globl	anon.f9c45627dbfd27b230899f0dc87ea5ab.27.llvm.7294274987384275483
	.p2align	3, 0x0
anon.f9c45627dbfd27b230899f0dc87ea5ab.27.llvm.7294274987384275483:
	.quad	_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtCs6a8jV7kq6PJ_10rayon_core20ThreadPoolBuildErrorEBD_.llvm.7294274987384275483
	.asciz	"\020\000\000\000\000\000\000\000\b\000\000\000\000\000\000"
	.quad	_RNvXsb_Cs6a8jV7kq6PJ_10rayon_coreNtB5_20ThreadPoolBuildErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.llvm.7294274987384275483
	.size	anon.f9c45627dbfd27b230899f0dc87ea5ab.27.llvm.7294274987384275483, 32

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.28,@object
	.section	.data.rel.ro..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.28,"aw",@progbits
	.p2align	3, 0x0
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.28:
	.quad	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.0
	.asciz	"^\000\000\000\000\000\000\000!\001\000\0004\000\000"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.28, 24

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.29,@object
	.section	.data.rel.ro..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.29,"aw",@progbits
	.p2align	3, 0x0
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.29:
	.quad	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.0
	.asciz	"^\000\000\000\000\000\000\000#\001\000\000<\000\000"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.29, 24

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.32,@object
	.section	.rodata..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.32,"a",@progbits
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.32:
	.ascii	"RAYON_NUM_THREADS"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.32, 17

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.33,@object
	.section	.rodata..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.33,"a",@progbits
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.33:
	.ascii	"RAYON_RS_NUM_CPUS"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.33, 17

	.hidden	anon.f9c45627dbfd27b230899f0dc87ea5ab.34.llvm.7294274987384275483
	.type	anon.f9c45627dbfd27b230899f0dc87ea5ab.34.llvm.7294274987384275483,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
	.globl	anon.f9c45627dbfd27b230899f0dc87ea5ab.34.llvm.7294274987384275483
anon.f9c45627dbfd27b230899f0dc87ea5ab.34.llvm.7294274987384275483:
	.asciz	"/kache/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rayon-core-1.13.0/src/latch.rs"
	.size	anon.f9c45627dbfd27b230899f0dc87ea5ab.34.llvm.7294274987384275483, 91

	.hidden	anon.f9c45627dbfd27b230899f0dc87ea5ab.35.llvm.7294274987384275483
	.type	anon.f9c45627dbfd27b230899f0dc87ea5ab.35.llvm.7294274987384275483,@object
	.section	.data.rel.ro.anon.f9c45627dbfd27b230899f0dc87ea5ab.35.llvm.7294274987384275483,"aw",@progbits
	.globl	anon.f9c45627dbfd27b230899f0dc87ea5ab.35.llvm.7294274987384275483
	.p2align	3, 0x0
anon.f9c45627dbfd27b230899f0dc87ea5ab.35.llvm.7294274987384275483:
	.quad	anon.f9c45627dbfd27b230899f0dc87ea5ab.34.llvm.7294274987384275483
	.asciz	"Z\000\000\000\000\000\000\000\364\000\000\000'\000\000"
	.size	anon.f9c45627dbfd27b230899f0dc87ea5ab.35.llvm.7294274987384275483, 24

	.hidden	anon.f9c45627dbfd27b230899f0dc87ea5ab.36.llvm.7294274987384275483
	.type	anon.f9c45627dbfd27b230899f0dc87ea5ab.36.llvm.7294274987384275483,@object
	.section	.data.rel.ro.anon.f9c45627dbfd27b230899f0dc87ea5ab.36.llvm.7294274987384275483,"aw",@progbits
	.globl	anon.f9c45627dbfd27b230899f0dc87ea5ab.36.llvm.7294274987384275483
	.p2align	3, 0x0
anon.f9c45627dbfd27b230899f0dc87ea5ab.36.llvm.7294274987384275483:
	.quad	anon.f9c45627dbfd27b230899f0dc87ea5ab.34.llvm.7294274987384275483
	.asciz	"Z\000\000\000\000\000\000\000\366\000\000\000(\000\000"
	.size	anon.f9c45627dbfd27b230899f0dc87ea5ab.36.llvm.7294274987384275483, 24

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.41,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.41:
	.asciz	"/kache/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/crossbeam-epoch-0.9.18/src/internal.rs"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.41, 99

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.42,@object
	.section	.data.rel.ro..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.42,"aw",@progbits
	.p2align	3, 0x0
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.42:
	.quad	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.41
	.asciz	"b\000\000\000\000\000\000\000\201\001\000\0009\000\000"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.42, 24

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.45,@object
	.section	.data.rel.ro..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.45,"aw",@progbits
	.p2align	3, 0x0
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.45:
	.quad	anon.f9c45627dbfd27b230899f0dc87ea5ab.7.llvm.7294274987384275483
	.asciz	"]\000\000\000\000\000\000\0008\003\000\000/\000\000"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.45, 24

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.46,@object
	.section	.data.rel.ro..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.46,"aw",@progbits
	.p2align	3, 0x0
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.46:
	.quad	anon.f9c45627dbfd27b230899f0dc87ea5ab.7.llvm.7294274987384275483
	.asciz	"]\000\000\000\000\000\000\000>\003\000\000*\000\000"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.46, 24

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.47,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.47:
	.asciz	"/kache/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/crossbeam-deque-0.8.6/src/deque.rs"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.47, 95

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.48,@object
	.section	.data.rel.ro..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.48,"aw",@progbits
	.p2align	3, 0x0
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.48:
	.quad	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.47
	.asciz	"^\000\000\000\000\000\000\000\177\005\000\000C\000\000"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.48, 24

	.type	_RNvNCNKNvNtCs6a8jV7kq6PJ_10rayon_core8registry19WORKER_THREAD_STATE0s_023___RUST_STD_INTERNAL_VAL,@object
	.section	.tbss._RNvNCNKNvNtCs6a8jV7kq6PJ_10rayon_core8registry19WORKER_THREAD_STATE0s_023___RUST_STD_INTERNAL_VAL,"awT",@nobits
	.globl	_RNvNCNKNvNtCs6a8jV7kq6PJ_10rayon_core8registry19WORKER_THREAD_STATE0s_023___RUST_STD_INTERNAL_VAL
	.p2align	3, 0x0
_RNvNCNKNvNtCs6a8jV7kq6PJ_10rayon_core8registry19WORKER_THREAD_STATE0s_023___RUST_STD_INTERNAL_VAL:
	.zero	8
	.size	_RNvNCNKNvNtCs6a8jV7kq6PJ_10rayon_core8registry19WORKER_THREAD_STATE0s_023___RUST_STD_INTERNAL_VAL, 8

	.type	_RNvNCNKNvNvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtBd_8Registry14in_worker_cold10LOCK_LATCH0s_023___RUST_STD_INTERNAL_VAL,@object
	.section	.tbss._RNvNCNKNvNvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtBd_8Registry14in_worker_cold10LOCK_LATCH0s_023___RUST_STD_INTERNAL_VAL,"awT",@nobits
	.globl	_RNvNCNKNvNvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtBd_8Registry14in_worker_cold10LOCK_LATCH0s_023___RUST_STD_INTERNAL_VAL
	.p2align	2, 0x0
_RNvNCNKNvNvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtBd_8Registry14in_worker_cold10LOCK_LATCH0s_023___RUST_STD_INTERNAL_VAL:
	.zero	6
	.zero	2
	.zero	4
	.size	_RNvNCNKNvNvMs4_NtCs6a8jV7kq6PJ_10rayon_core8registryNtBd_8Registry14in_worker_cold10LOCK_LATCH0s_023___RUST_STD_INTERNAL_VAL, 12

	.type	_RNvNtCs6a8jV7kq6PJ_10rayon_core8registry12THE_REGISTRY,@object
	.section	.bss._RNvNtCs6a8jV7kq6PJ_10rayon_core8registry12THE_REGISTRY,"aw",@nobits
	.globl	_RNvNtCs6a8jV7kq6PJ_10rayon_core8registry12THE_REGISTRY
	.p2align	3, 0x0
_RNvNtCs6a8jV7kq6PJ_10rayon_core8registry12THE_REGISTRY:
	.zero	8
	.size	_RNvNtCs6a8jV7kq6PJ_10rayon_core8registry12THE_REGISTRY, 8

	.hidden	anon.f9c45627dbfd27b230899f0dc87ea5ab.49.llvm.7294274987384275483
	.type	anon.f9c45627dbfd27b230899f0dc87ea5ab.49.llvm.7294274987384275483,@object
	.section	.rodata.anon.f9c45627dbfd27b230899f0dc87ea5ab.49.llvm.7294274987384275483,"a",@progbits
	.globl	anon.f9c45627dbfd27b230899f0dc87ea5ab.49.llvm.7294274987384275483
anon.f9c45627dbfd27b230899f0dc87ea5ab.49.llvm.7294274987384275483:
	.ascii	"The global thread pool has not been initialized."
	.size	anon.f9c45627dbfd27b230899f0dc87ea5ab.49.llvm.7294274987384275483, 48

	.hidden	anon.f9c45627dbfd27b230899f0dc87ea5ab.50.llvm.7294274987384275483
	.type	anon.f9c45627dbfd27b230899f0dc87ea5ab.50.llvm.7294274987384275483,@object
	.section	.data.rel.ro.anon.f9c45627dbfd27b230899f0dc87ea5ab.50.llvm.7294274987384275483,"aw",@progbits
	.globl	anon.f9c45627dbfd27b230899f0dc87ea5ab.50.llvm.7294274987384275483
	.p2align	3, 0x0
anon.f9c45627dbfd27b230899f0dc87ea5ab.50.llvm.7294274987384275483:
	.quad	anon.f9c45627dbfd27b230899f0dc87ea5ab.7.llvm.7294274987384275483
	.asciz	"]\000\000\000\000\000\000\000\253\000\000\000\n\000\000"
	.size	anon.f9c45627dbfd27b230899f0dc87ea5ab.50.llvm.7294274987384275483, 24

	.type	_RNvNtCs6a8jV7kq6PJ_10rayon_core8registry16THE_REGISTRY_SET,@object
	.section	.data._RNvNtCs6a8jV7kq6PJ_10rayon_core8registry16THE_REGISTRY_SET,"aw",@progbits
	.globl	_RNvNtCs6a8jV7kq6PJ_10rayon_core8registry16THE_REGISTRY_SET
	.p2align	2, 0x0
_RNvNtCs6a8jV7kq6PJ_10rayon_core8registry16THE_REGISTRY_SET:
	.asciz	"\003\000\000"
	.size	_RNvNtCs6a8jV7kq6PJ_10rayon_core8registry16THE_REGISTRY_SET, 4

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.51,@object
	.section	.data.rel.ro..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.51,"aw",@progbits
	.p2align	3, 0x0
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.51:
	.quad	anon.f9c45627dbfd27b230899f0dc87ea5ab.7.llvm.7294274987384275483
	.asciz	"]\000\000\000\000\000\000\000\225\003\000\000&\000\000"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.51, 24

	.type	_RNvNvMs9_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB7_14XorShift64Star3new7COUNTER,@object
	.section	.bss._RNvNvMs9_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB7_14XorShift64Star3new7COUNTER,"aw",@nobits
	.p2align	3, 0x0
_RNvNvMs9_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB7_14XorShift64Star3new7COUNTER:
	.zero	8
	.size	_RNvNvMs9_NtCs6a8jV7kq6PJ_10rayon_core8registryNtB7_14XorShift64Star3new7COUNTER, 8

	.hidden	anon.f9c45627dbfd27b230899f0dc87ea5ab.52.llvm.7294274987384275483
	.type	anon.f9c45627dbfd27b230899f0dc87ea5ab.52.llvm.7294274987384275483,@object
	.section	.rodata.anon.f9c45627dbfd27b230899f0dc87ea5ab.52.llvm.7294274987384275483,"a",@progbits
	.globl	anon.f9c45627dbfd27b230899f0dc87ea5ab.52.llvm.7294274987384275483
anon.f9c45627dbfd27b230899f0dc87ea5ab.52.llvm.7294274987384275483:
	.ascii	"Rayon: detected unexpected panic; aborting\n"
	.size	anon.f9c45627dbfd27b230899f0dc87ea5ab.52.llvm.7294274987384275483, 43

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.53,@object
	.section	.rodata..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.53,"a",@progbits
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.53:
	.ascii	"fatal runtime error: thread local panicked on drop, aborting\n"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.53, 61

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.60,@object
	.section	.rodata..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.60,"a",@progbits
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.60:
	.ascii	"fatal runtime error: thread result panicked on drop, aborting\n"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.60, 62

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.76,@object
	.section	.data.rel.ro..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.76,"aw",@progbits
	.p2align	3, 0x0
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.76:
	.quad	anon.f9c45627dbfd27b230899f0dc87ea5ab.34.llvm.7294274987384275483
	.asciz	"Z\000\000\000\000\000\000\000\007\001\000\000*\000\000"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.76, 24

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.119,@object
	.section	.rodata..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.119,"a",@progbits
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.119:
	.ascii	"PoisonError"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.119, 11

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.120,@object
	.section	.data.rel.ro..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.120,"aw",@progbits
	.p2align	3, 0x0
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.120:
	.asciz	"\000\000\000\000\000\000\000\000\b\000\000\000\000\000\000\000\b\000\000\000\000\000\000"
	.quad	_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtCs6a8jV7kq6PJ_10rayon_core9ErrorKindNtB6_5Debug3fmtBy_
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.120, 32

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.121,@object
	.section	.rodata..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.121,"a",@progbits
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.121:
	.ascii	"ThreadPoolBuildError"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.121, 20

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.122,@object
	.section	.rodata.cst4,"aM",@progbits,4
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.122:
	.ascii	"kind"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.122, 4

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.123,@object
	.section	.rodata..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.123,"a",@progbits
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.123:
	.ascii	"GlobalPoolAlreadyInitialized"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.123, 28

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.124,@object
	.section	.rodata..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.124,"a",@progbits
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.124:
	.ascii	"CurrentThreadAlreadyInPool"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.124, 26

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.126,@object
	.section	.rodata..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.126,"a",@progbits
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.126:
	.ascii	"IOError"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.126, 7

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.143,@object
	.section	.rodata..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.143,"a",@progbits
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.143:
	.ascii	"failed to write whole buffer"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.143, 28

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.144,@object
	.section	.data.rel.ro..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.144,"aw",@progbits
	.p2align	3, 0x0
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.144:
	.quad	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.143
	.ascii	"\034\000\000\000\000\000\000\000\027"
	.zero	7
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.144, 24

	.type	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.145,@object
	.section	.data.rel.ro..Lanon.f9c45627dbfd27b230899f0dc87ea5ab.145,"aw",@progbits
	.p2align	3, 0x0
.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.145:
	.quad	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.12
	.asciz	"L\000\000\000\000\000\000\000\334\000\000\000$\000\000"
	.size	.Lanon.f9c45627dbfd27b230899f0dc87ea5ab.145, 24

	.hidden	anon.a647af78948fdeb0321157bdb82ef7e0.34.llvm.9794848731438112354
	.hidden	anon.a647af78948fdeb0321157bdb82ef7e0.37.llvm.9794848731438112354
	.hidden	anon.a647af78948fdeb0321157bdb82ef7e0.42.llvm.9794848731438112354
	.hidden	anon.a647af78948fdeb0321157bdb82ef7e0.45.llvm.9794848731438112354
	.hidden	anon.a647af78948fdeb0321157bdb82ef7e0.46.llvm.9794848731438112354
	.hidden	anon.a647af78948fdeb0321157bdb82ef7e0.47.llvm.9794848731438112354
	.hidden	anon.a647af78948fdeb0321157bdb82ef7e0.91.llvm.9794848731438112354
	.hidden	anon.a647af78948fdeb0321157bdb82ef7e0.96.llvm.9794848731438112354
	.hidden	_RNvNvNtCs18aJq3QiqAb_15crossbeam_epoch7default9collector9COLLECTOR.llvm.707543514826133848
	.hidden	_RNvCs2911K1BwAFx_18purrdf_alloc_probe13PROCESS_STATE.llvm.6551227014246703601
	.hidden	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_LIVE_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601
	.hidden	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe17THREAD_PEAK_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601
	.hidden	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe18THREAD_ALLOCATIONS0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601
	.hidden	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe19THREAD_TROUGH_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601
	.hidden	_RNvNCNKNvCs2911K1BwAFx_18purrdf_alloc_probe22THREAD_REQUESTED_BYTES0s_023___RUST_STD_INTERNAL_VAL.llvm.6551227014246703601
	.hidden	_RNvCs3dQcT9XZUpl_29qualification_454_native_cost6GLOBAL.llvm.4264773384553001850
	.hidden	anon.f13bb3f258876cb12f4a84b0a5cdace5.73.llvm.11640361436736466388
	.hidden	anon.f13bb3f258876cb12f4a84b0a5cdace5.76.llvm.11640361436736466388
	.hidden	_RNvNCNKNvNtNtCs7jcFBdfocI9_3std9panicking11panic_count17LOCAL_PANIC_COUNT0s_023___RUST_STD_INTERNAL_VAL.llvm.11640361436736466388
	.hidden	_RNvNvMNtNtCs7jcFBdfocI9_3std6thread2idNtB4_8ThreadId3new7COUNTER.llvm.11640361436736466388
	.hidden	_RINvMs0_NtNtCs18aJq3QiqAb_15crossbeam_epoch4sync9once_lockINtB6_8OnceLockNtNtBa_9collector9CollectorE10initializeNvMs1_B1b_B19_3newEBa_.llvm.707543514826133848
	.hidden	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
	.hidden	_RNCNvNtNtNtCs7jcFBdfocI9_3std3sys3env4unix6getenv0B9_.llvm.11640361436736466388
	.hidden	_RNvNvMNtNtCs7jcFBdfocI9_3std6thread2idNtB4_8ThreadId3new9exhausted.llvm.11640361436736466388
	.hidden	_RNvMNtNtCs7jcFBdfocI9_3std6thread6scopedNtB2_9ScopeData8overflow.llvm.11640361436736466388
	.hidden	DW.ref.rust_eh_personality
	.weak	DW.ref.rust_eh_personality
	.section	.data.DW.ref.rust_eh_personality,"awG",@progbits,DW.ref.rust_eh_personality,comdat
	.p2align	3, 0x0
	.type	DW.ref.rust_eh_personality,@object
	.size	DW.ref.rust_eh_personality, 8
DW.ref.rust_eh_personality:
	.quad	rust_eh_personality
	.ident	"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"
	.section	".note.GNU-stack","",@progbits
