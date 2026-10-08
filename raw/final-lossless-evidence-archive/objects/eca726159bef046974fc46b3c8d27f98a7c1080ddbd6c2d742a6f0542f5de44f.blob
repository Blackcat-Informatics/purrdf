purrdf_sparql_eval::binop::eval_application::<purrdf_core::ir::dataset::RdfDataset, ()>:
.Lfunc_begin313:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception219
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
	subq	$1464, %rsp
	.cfi_def_cfa_offset 1520
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	%rdi, 40(%rsp)
	leaq	888(%rsp), %rdi
	movq	%r8, %r12
	movq	%rcx, %r13
	movq	%rdx, %rbx
	callq	*%rax
	movb	$1, %bpl
.Ltmp11200:
	leaq	1344(%rsp), %rdi
	movq	%rbx, %rsi
	movq	%r12, %rdx
	movq	%rbx, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp11201:
	cmpl	$1, 1344(%rsp)
	jne	.LBB313_3
	vmovups	1360(%rsp), %zmm0
	vmovups	1392(%rsp), %zmm1
	movq	40(%rsp), %rax
	vmovups	%zmm1, 48(%rax)
	vmovups	%zmm0, 16(%rax)
	movq	$1, (%rax)
	jmp	.LBB313_95
.LBB313_3:
	vmovups	1384(%rsp), %zmm1
	vmovups	1352(%rsp), %zmm0
	vmovups	%zmm1, 80(%rsp)
	vmovups	%zmm0, 48(%rsp)
.Ltmp11202:
	leaq	208(%rsp), %rdi
	leaq	888(%rsp), %rsi
	leaq	48(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp11203:
	cmpq	$-1, 208(%rsp)
	je	.LBB313_9
	vmovups	208(%rsp), %ymm0
	cmpq	$-1, 888(%rsp)
	vmovups	%ymm0, 432(%rsp)
	je	.LBB313_11
	vmovups	928(%rsp), %zmm1
	vmovups	888(%rsp), %zmm0
	movq	456(%rsp), %rax
	movq	%rax, 728(%rsp)
	movq	$0, 704(%rsp)
	movq	$8, 712(%rsp)
	movq	$0, 720(%rsp)
	vmovups	%zmm1, 88(%rsp)
	vmovups	%zmm0, 48(%rsp)
	cmpq	$-1, 48(%rsp)
	je	.LBB313_18
	leaq	208(%rsp), %rdi
	leaq	704(%rsp), %rsi
	leaq	888(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	120(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB313_8
.LBB313_19:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	128(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB313_21
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB313_21:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB313_27
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB313_21
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB313_24:
	cmpq	%rax, %rsi
	jge	.LBB313_26
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB313_24
.LBB313_26:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB313_27:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	144(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB313_28
	jmp	.LBB313_30
.LBB313_9:
	xorl	%ebp, %ebp
.Ltmp11417:
	leaq	48(%rsp), %rdi
	leaq	888(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
.Ltmp11418:
	vmovups	48(%rsp), %zmm0
	vmovups	80(%rsp), %zmm1
	movq	40(%rsp), %rax
	vmovups	%zmm1, 40(%rax)
	vmovups	%zmm0, 8(%rax)
	movq	$0, (%rax)
	jmp	.LBB313_118
.LBB313_11:
	cmpl	$28, (%r13)
	jne	.LBB313_31
	movq	624(%r12), %rax
	xorl	%r14d, %r14d
	testq	%rax, %rax
	je	.LBB313_43
	testb	$1, 1200(%r12)
	je	.LBB313_44
	movq	1208(%r12), %rcx
	cmpq	40(%rax), %rcx
	jne	.LBB313_37
	movl	1216(%r12), %ecx
	subl	80(%rax), %ecx
	jb	.LBB313_37
	cmpq	%rcx, 32(%rax)
	jbe	.LBB313_37
	movq	24(%rax), %rax
	shlq	$4, %rcx
	movq	(%rax,%rcx), %r14
	movq	8(%rax,%rcx), %r15
	jmp	.LBB313_44
.LBB313_18:
	movq	712(%rsp), %rcx
	movq	704(%rsp), %rax
	movq	720(%rsp), %rdx
	movq	%rcx, 224(%rsp)
	movq	728(%rsp), %rcx
	movq	%rax, 216(%rsp)
	movq	%rdx, 232(%rsp)
	movq	%rcx, 240(%rsp)
	movq	$-1, 208(%rsp)
	movq	120(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB313_19
.LBB313_8:
	movq	144(%rsp), %rax
	testq	%rax, %rax
	je	.LBB313_30
.LBB313_28:
	lock		decq	(%rax)
	jne	.LBB313_30
	leaq	144(%rsp), %rdi
	#MEMBARRIER
.Ltmp11204:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp11205:
.LBB313_30:
	vmovups	208(%rsp), %zmm0
	vmovups	240(%rsp), %zmm1
	movq	40(%rsp), %rax
	movl	$0, 32(%rsp)
	vmovups	%zmm1, 40(%rax)
	vmovups	%zmm0, 8(%rax)
	movq	$0, (%rax)
	jmp	.LBB313_71
.LBB313_31:
	movb	$1, %bpl
.Ltmp11207:
	leaq	48(%rsp), %rdi
	leaq	432(%rsp), %rsi
	movq	%r13, 200(%rsp)
	movq	%r13, %rdx
	movq	%r12, %rcx
	vzeroupper
	callq	purrdf_sparql_eval::service_endpoints::admit_lateral_endpoints::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp11208:
	cmpq	$-1, 48(%rsp)
	movq	%r12, 24(%rsp)
	jne	.LBB313_42
	movq	24(%rsp), %rax
	movq	200(%rsp), %rbx
	movq	584(%rax), %rdi
	testq	%rdi, %rdi
	je	.LBB313_38
	addq	$16, %rdi
	movq	%rbx, %rsi
	callq	<hashbrown::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>::get::<usize>
	testq	%rax, %rax
	je	.LBB313_38
	cmpl	$1, (%rax)
	jne	.LBB313_38
	addq	$8, %rax
	leaq	384(%rsp), %rdi
	movq	%rax, %rsi
	callq	<purrdf_sparql_eval::deferred_exists::DeferredLateral as core::clone::Clone>::clone
	cmpq	$0, 384(%rsp)
	jne	.LBB313_59
	jmp	.LBB313_39
.LBB313_37:
	xorl	%r14d, %r14d
.LBB313_43:
.LBB313_44:
	movb	$1, %bpl
.Ltmp11395:
	leaq	528(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r13, %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::enter_node
.Ltmp11396:
.Ltmp11397:
	leaq	1080(%rsp), %rdi
	movq	%r12, %rsi
	xorl	%edx, %edx
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge
.Ltmp11398:
	cmpb	$-1, 1080(%rsp)
	leaq	1200(%r12), %rbx
	je	.LBB313_54
.Ltmp11399:
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	callq	*%rax
.Ltmp11400:
	vmovups	1080(%rsp), %xmm0
	movq	%rax, 728(%rsp)
	movq	1096(%rsp), %rax
	leaq	48(%rsp), %rdi
	leaq	704(%rsp), %rsi
	leaq	208(%rsp), %rdx
	movq	$0, 704(%rsp)
	movq	$8, 712(%rsp)
	movq	$0, 208(%rsp)
	movq	$1, 216(%rsp)
	movq	$0, 224(%rsp)
	movq	$0, 720(%rsp)
	movq	%rax, 248(%rsp)
	vmovups	%xmm0, 232(%rsp)
	movq	$0, 256(%rsp)
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	80(%rsp), %rcx
	vmovups	64(%rsp), %xmm0
	movq	48(%rsp), %rax
	movq	56(%rsp), %rsi
	movq	88(%rsp), %rdx
	movq	%rcx, 688(%rsp)
	movzbl	96(%rsp), %ecx
.LBB313_49:
	vmovaps	%xmm0, 672(%rsp)
	vmovups	97(%rsp), %xmm0
	movq	112(%rsp), %rdi
	vmovups	120(%rsp), %xmm1
	vmovups	528(%rsp), %xmm2
	movq	%rax, 1248(%rsp)
	movq	688(%rsp), %rax
	movq	%rsi, 1256(%rsp)
	movq	%rax, 1280(%rsp)
	vmovaps	%xmm0, 608(%rsp)
	movq	%rdi, 623(%rsp)
	movq	136(%rsp), %rdi
	vmovaps	%xmm1, 640(%rsp)
	vmovaps	672(%rsp), %xmm1
	vmovaps	%xmm2, (%rbx)
	vmovaps	608(%rsp), %xmm2
	movq	623(%rsp), %rax
	movq	%rdi, 656(%rsp)
	movq	544(%rsp), %rdi
	vmovups	%xmm1, 1264(%rsp)
	vmovaps	640(%rsp), %xmm1
	movq	%rdx, 1288(%rsp)
	movb	%cl, 1296(%rsp)
	vmovups	%xmm2, 1297(%rsp)
	movq	%rax, 1312(%rsp)
	movq	656(%rsp), %rax
	movq	%rdi, 16(%rbx)
	movl	552(%rsp), %edi
	movq	%rax, 1336(%rsp)
	movl	%edi, 1228(%r12)
	movzbl	556(%rsp), %edi
	vmovups	%xmm1, 1320(%rsp)
	movb	%dil, 1238(%r12)
.Ltmp11404:
	leaq	1168(%rsp), %rdi
	leaq	888(%rsp), %rsi
	leaq	1248(%rsp), %rcx
	movl	$1, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp11405:
	cmpq	$-1, 1168(%rsp)
	je	.LBB313_58
	vmovups	888(%rsp), %zmm0
	vmovups	928(%rsp), %zmm1
	vmovups	%zmm0, 48(%rsp)
	vmovups	%zmm1, 88(%rsp)
	cmpq	$-1, 48(%rsp)
	je	.LBB313_63
	leaq	208(%rsp), %rdi
	leaq	1168(%rsp), %rsi
	leaq	888(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	jmp	.LBB313_64
.LBB313_54:
	addq	$8, %r13
.Ltmp11401:
	leaq	208(%rsp), %rdi
	leaq	432(%rsp), %rdx
	movq	%r13, %rsi
	movq	%r14, %rcx
	movq	%r15, %r8
	movq	%r12, %r9
	callq	purrdf_sparql_eval::property_fn_eval::eval_call_over::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp11402:
	vmovups	216(%rsp), %ymm0
	vmovups	249(%rsp), %xmm1
	movq	208(%rsp), %rcx
	movzbl	248(%rsp), %eax
	movq	264(%rsp), %rdx
	vmovaps	%xmm1, 800(%rsp)
	vmovups	%ymm0, 704(%rsp)
	movq	%rdx, 815(%rsp)
	cmpq	$-1, %rcx
	je	.LBB313_119
	vmovaps	800(%rsp), %xmm1
	vmovaps	704(%rsp), %xmm2
	movq	815(%rsp), %rdi
	movq	720(%rsp), %rsi
	vmovaps	272(%rsp), %xmm0
	movq	40(%rsp), %r8
	movq	288(%rsp), %rdx
	movq	%rsi, 688(%rsp)
	movq	728(%rsp), %rsi
	movq	%rdx, 656(%rsp)
	movq	296(%rsp), %rdx
	vmovaps	%xmm1, 608(%rsp)
	movq	%rdi, 623(%rsp)
	movq	544(%rsp), %rdi
	vmovaps	%xmm2, 672(%rsp)
	vmovups	528(%rsp), %xmm2
	vmovaps	%xmm0, 640(%rsp)
	vmovaps	672(%rsp), %xmm1
	movq	%rdi, 16(%rbx)
	movl	552(%rsp), %edi
	vmovaps	%xmm2, (%rbx)
	vmovaps	608(%rsp), %xmm2
	vmovups	%xmm1, 24(%r8)
	vmovaps	640(%rsp), %xmm1
	movl	%edi, 1228(%r12)
	movzbl	556(%rsp), %edi
	movb	%dil, 1238(%r12)
	movq	688(%rsp), %rdi
	movq	%rdi, 40(%r8)
	movq	%rsi, 48(%r8)
	movq	623(%rsp), %rsi
	vmovups	%xmm2, 57(%r8)
	movq	%rsi, 72(%r8)
	movq	656(%rsp), %rsi
	vmovaps	%xmm1, 80(%r8)
	movq	%rsi, 96(%r8)
	movq	%rcx, 16(%r8)
	movb	%al, 56(%r8)
	movq	%rdx, 104(%r8)
	movq	$1, (%r8)
	jmp	.LBB313_57
.LBB313_38:
	movq	$0, 384(%rsp)
.LBB313_39:
	movb	$1, %bpl
.Ltmp11209:
	movq	purrdf_sparql_eval::deferred_exists::is_lateral_placeholder@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp11210:
	testb	%al, %al
	je	.LBB313_59
.Ltmp11390:
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.443(%rip), %rsi
	leaq	48(%rsp), %rdi
	movl	$84, %edx
	callq	<purrdf_sparql_eval::error::EvalError>::internal::<&str>
.Ltmp11391:
.LBB313_42:
	vmovups	48(%rsp), %zmm0
	vmovups	80(%rsp), %zmm1
	movq	40(%rsp), %rax
	vmovups	%zmm1, 48(%rax)
	vmovups	%zmm0, 16(%rax)
	movq	$1, (%rax)
.LBB313_57:
	movb	$1, %bpl
	movq	456(%rsp), %rax
	lock		decq	(%rax)
	movl	%ebp, 32(%rsp)
	je	.LBB313_70
	jmp	.LBB313_71
.LBB313_58:
.Ltmp11409:
	leaq	208(%rsp), %rdi
	leaq	888(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
.Ltmp11410:
	jmp	.LBB313_69
.LBB313_59:
	movq	456(%rsp), %rax
	lock		incq	(%rax)
	jle	.LBB313_339
	vmovups	anon.e5162873a9a3251d11c4df37a70e4654.29.llvm.18159039729619107857(%rip), %ymm0
	movq	448(%rsp), %r15
	movq	%rax, 168(%rsp)
	movq	%rax, 520(%rsp)
	movabsq	$128102389400760776, %rax
	movq	$0, 800(%rsp)
	movq	$8, 808(%rsp)
	movq	$0, 816(%rsp)
	decq	%rax
	vmovups	%ymm0, 824(%rsp)
	cmpq	%rax, %r15
	jbe	.LBB313_121
	xorl	%r14d, %r14d
.LBB313_62:
	movb	$1, %bpl
.Ltmp11383:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
.Ltmp11384:
	jmp	.LBB313_339
.LBB313_63:
	vmovups	1168(%rsp), %ymm0
	vmovups	%ymm0, 216(%rsp)
	movq	$-1, 208(%rsp)
.LBB313_64:
	movq	120(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB313_66
	movq	128(%rsp), %rdi
	leaq	-3(%rax,%rax,2), %rsi
	movl	$1, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB313_66:
	movq	144(%rsp), %rax
	testq	%rax, %rax
	je	.LBB313_69
	lock		decq	(%rax)
	jne	.LBB313_69
	leaq	144(%rsp), %rdi
	#MEMBARRIER
.Ltmp11407:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp11408:
.LBB313_69:
	vmovups	208(%rsp), %zmm0
	vmovups	240(%rsp), %zmm1
	movq	40(%rsp), %rax
	xorl	%ebp, %ebp
	vmovups	%zmm1, 40(%rax)
	vmovups	%zmm0, 8(%rax)
	movq	$0, (%rax)
	movq	456(%rsp), %rax
	lock		decq	(%rax)
	movl	%ebp, 32(%rsp)
	jne	.LBB313_71
.LBB313_70:
	leaq	456(%rsp), %rdi
	#MEMBARRIER
.Ltmp11414:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp11415:
.LBB313_71:
	movq	440(%rsp), %r14
	movq	448(%rsp), %rbx
	testq	%rbx, %rbx
	je	.LBB313_84
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	movabsq	$9223372036854775807, %r12
	xorl	%r15d, %r15d
	jmp	.LBB313_76
	.p2align	4
.LBB313_73:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB313_74:
	vzeroupper
	callq	*%r13
.LBB313_75:
	incq	%r15
	cmpq	%rbx, %r15
	je	.LBB313_84
.LBB313_76:
	leaq	(%r15,%r15,4), %rcx
	movq	(%r14,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB313_75
	leaq	(%r14,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%r12, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r12, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r12, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB313_79
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB313_79:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB313_74
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB313_79
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r12, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rbp), %rax
	.p2align	4
.LBB313_82:
	cmpq	%rax, %rdx
	jge	.LBB313_73
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB313_82
	jmp	.LBB313_73
.LBB313_84:
	movq	432(%rsp), %rax
	movl	32(%rsp), %ebx
	testq	%rax, %rax
	je	.LBB313_94
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rdx
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB313_87
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB313_87:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB313_93
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB313_87
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB313_90:
	cmpq	%rax, %rsi
	jge	.LBB313_92
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB313_90
.LBB313_92:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB313_93:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.LBB313_94:
	testb	%bl, %bl
	je	.LBB313_118
.LBB313_95:
	movq	960(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB313_105
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	968(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB313_98
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB313_98:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB313_104
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB313_98
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB313_101:
	cmpq	%rax, %rsi
	jge	.LBB313_103
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB313_101
.LBB313_103:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB313_104:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB313_105:
	movq	888(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB313_115
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	896(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB313_108
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB313_108:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB313_114
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB313_108
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB313_111:
	cmpq	%rax, %rsi
	jge	.LBB313_113
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB313_111
.LBB313_113:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB313_114:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB313_115:
	movq	984(%rsp), %rax
	testq	%rax, %rax
	je	.LBB313_118
	lock		decq	(%rax)
	jne	.LBB313_118
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	984(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB313_118:
	movq	40(%rsp), %rax
	addq	$1464, %rsp
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
.LBB313_119:
	.cfi_def_cfa_offset 1520
	cmpb	$-1, %al
	je	.LBB313_238
	vmovaps	800(%rsp), %xmm0
	movq	815(%rsp), %rcx
	leaq	48(%rsp), %rdi
	leaq	704(%rsp), %rsi
	leaq	208(%rsp), %rdx
	movq	%rcx, 248(%rsp)
	vmovups	%xmm0, 233(%rsp)
	movq	$0, 208(%rsp)
	movq	$1, 216(%rsp)
	movq	$0, 224(%rsp)
	movb	%al, 232(%rsp)
	movq	$0, 256(%rsp)
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	48(%rsp), %rax
	movq	88(%rsp), %rdx
	movzbl	96(%rsp), %ecx
	jmp	.LBB313_239
.LBB313_121:
	testq	%r15, %r15
	je	.LBB313_240
	leaq	(,%r15,8), %rax
	movl	$8, %esi
	movl	$8, %r14d
	leaq	(%rax,%rax,8), %rbx
	movq	%rbx, %rdi
	vzeroupper
	callq	__rustc::__rust_alloc
	testq	%rax, %rax
	je	.LBB313_62
	movq	440(%rsp), %rdx
	movq	%rax, %rcx
	movq	%r15, 408(%rsp)
	movq	%rax, 416(%rsp)
	leaq	(%r15,%r15,4), %rax
	movq	24(%rsp), %rsi
	movq	%rcx, %rbp
	movq	$0, 424(%rsp)
	movq	$0, 8(%rsp)
	leaq	(%rdx,%rax,8), %rax
	movq	%rdx, 32(%rsp)
	movq	168(%rsp), %rdx
	movq	%rax, 344(%rsp)
	leaq	1096(%rsi), %rax
	movq	%rax, 368(%rsp)
	leaq	16(%rdx), %rax
	movq	%rax, 328(%rsp)
	jmp	.LBB313_125
.LBB313_124:
	movq	8(%rsp), %rdx
	movq	112(%rsp), %rcx
	movq	32(%rsp), %rsi
	leaq	(%rdx,%rdx,8), %rax
	addq	$40, %rsi
	incq	%rdx
	movq	%rdx, 8(%rsp)
	movq	%rsi, 32(%rsp)
	movq	%rcx, 64(%rbp,%rax,8)
	vmovups	48(%rsp), %zmm0
	vmovups	%zmm0, (%rbp,%rax,8)
	movq	%rdx, 424(%rsp)
	cmpq	344(%rsp), %rsi
	je	.LBB313_245
.LBB313_125:
	movq	384(%rsp), %r12
	testq	%r12, %r12
	je	.LBB313_128
	movq	32(%rsp), %rax
	movq	(%rax), %r14
	decq	%r14
	cmpq	$4, %r14
	jbe	.LBB313_130
	movq	16(%rax), %r14
	movq	8(%rax), %r13
	decq	%r14
	jmp	.LBB313_131
.LBB313_128:
	movq	24(%rsp), %rax
	movzbl	1237(%rax), %eax
	incq	%rax
	movq	%rax, 704(%rsp)
	movq	32(%rsp), %rax
	movq	$0, 720(%rsp)
	movq	(%rax), %rcx
	decq	%rcx
	cmpq	$4, %rcx
	jbe	.LBB313_140
	movq	16(%rax), %rcx
	movq	8(%rax), %rdx
	decq	%rcx
	jmp	.LBB313_141
.LBB313_130:
	leaq	8(%rax), %r13
.LBB313_131:
	leaq	464(%rsp), %rax
	movb	$0, 464(%rsp)
	movq	%rax, 48(%rsp)
	leaq	48(%rsp), %rax
	#APP
	#NO_APP
	movq	purrdf_stack::FLOOR::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rax
	movq	48(%rsp), %rdi
	movq	%fs:(%rax), %rcx
	movq	%rdi, %rax
	subq	%rcx, %rax
	cmpq	$131072, %rax
	setb	%al
	cmpq	%rcx, %rdi
	jb	.LBB313_234
	testb	%al, %al
	jne	.LBB313_235
.LBB313_133:
	movq	168(%rsp), %rax
	movq	24(%rax), %rcx
	movq	32(%rax), %r8
.Ltmp11213:
	movq	24(%rsp), %r9
	leaq	48(%rsp), %rdi
	movq	%r13, %rsi
	movq	%r14, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::expr::outer_bindings_for_substitution::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp11214:
	leaq	56(%rsp), %rdx
	movq	48(%rsp), %rax
	vmovups	(%rdx), %ymm0
	vmovups	16(%rdx), %ymm1
	vmovups	%ymm0, 464(%rsp)
	vmovups	%ymm1, 480(%rsp)
	cmpq	$-1, %rax
	je	.LBB313_137
	vmovups	48(%rdx), %ymm0
	vmovups	464(%rsp), %ymm2
	vmovups	480(%rsp), %ymm1
	movq	80(%rdx), %rcx
	leaq	224(%rsp), %rsi
	movq	%rcx, 88(%rsi)
	vmovups	%ymm0, 56(%rsi)
	vmovups	%ymm1, 24(%rsi)
	vmovups	%ymm2, 8(%rsi)
	movq	%rax, 224(%rsp)
.LBB313_136:
	movq	$1, 208(%rsp)
	jmp	.LBB313_199
.LBB313_137:
	vmovups	464(%rsp), %ymm0
	vmovups	480(%rsp), %ymm1
	movq	400(%rsp), %rsi
	movq	392(%rsp), %rdx
	leaq	16(%r12), %rax
	vmovups	%ymm0, 1024(%rsp)
	vmovups	%ymm1, 1040(%rsp)
	movq	80(%r12), %rcx
	movq	$3, 864(%rsp)
	movq	%rax, 872(%rsp)
	movl	$0, %eax
	addq	$16, %rcx
	cmpq	$1, %rsi
	adcq	$1, %rax
	testq	%rdx, %rdx
	movq	%rcx, 880(%rsp)
	cmoveq	%rdx, %rax
	testq	%rax, %rax
	je	.LBB313_144
	cmpq	$1, %rax
	jne	.LBB313_151
	addq	$16, %rsi
	leaq	48(%r12), %rcx
.Ltmp11215:
	movq	purrdf_sparql_eval::deferred_exists::with_row@GOTPCREL(%rip), %rax
	leaq	1200(%rsp), %rdi
	leaq	1024(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp11216:
	jmp	.LBB313_146
.LBB313_140:
	leaq	8(%rax), %rdx
.LBB313_141:
.Ltmp11302:
	movq	24(%rsp), %rax
	movq	200(%rsp), %rsi
	movq	328(%rsp), %r8
	leaq	48(%rsp), %rdi
	leaq	704(%rsp), %r9
	movq	%rax, (%rsp)
	vzeroupper
	callq	purrdf_sparql_eval::binop::eval_correlated::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp11303:
	cmpl	$1, 48(%rsp)
	je	.LBB313_321
	leaq	56(%rsp), %rcx
	movq	56(%rsp), %rax
	movq	%rbp, %r13
	vmovups	32(%rcx), %zmm1
	vmovups	8(%rcx), %zmm0
	leaq	224(%rsp), %rcx
	vmovups	%zmm1, 232(%rsp)
	vmovups	%zmm0, 208(%rsp)
	vmovups	208(%rsp), %ymm0
	vmovups	40(%rcx), %ymm1
	vmovups	%ymm0, 992(%rsp)
	vmovups	16(%rcx), %ymm0
	jmp	.LBB313_201
.LBB313_144:
	leaq	56(%rsp), %rax
	movq	$0, 48(%rsp)
	movq	$8, 56(%rsp)
	vxorps	%xmm0, %xmm0, %xmm0
	leaq	48(%r12), %rcx
	vmovups	%xmm0, 8(%rax)
	movq	$8, 80(%rsp)
	movq	$0, 88(%rsp)
.Ltmp11217:
	movq	purrdf_sparql_eval::deferred_exists::with_row@GOTPCREL(%rip), %rax
	leaq	48(%rsp), %r14
	leaq	1200(%rsp), %rdi
	leaq	1024(%rsp), %rdx
	movq	%r14, %rsi
	vzeroupper
	callq	*%rax
.Ltmp11218:
.Ltmp11222:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp11223:
.LBB313_146:
	cmpq	$-1, 1200(%rsp)
	je	.LBB313_152
	movq	24(%rsp), %rax
	vmovups	1216(%rsp), %ymm1
	vmovups	1200(%rsp), %ymm0
	cmpq	$0, 632(%rax)
	vmovups	%ymm1, 480(%rsp)
	vmovups	%ymm0, 464(%rsp)
	je	.LBB313_177
	movq	80(%r12), %rax
	cmpq	$0, 40(%rax)
	je	.LBB313_177
	lock		incq	(%rax)
	jle	.LBB313_339
	movq	80(%r12), %rsi
	movb	$1, %bl
	movq	%rbp, %r14
.Ltmp11224:
	movq	368(%rsp), %rdi
	vzeroupper
	callq	<alloc::vec::Vec<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::expr::SubstitutionSource, purrdf_hash::fixed::FixedState>>>>::push_mut
.Ltmp11225:
	jmp	.LBB313_178
.LBB313_151:
	movq	$-1, 1200(%rsp)
.LBB313_152:
	movq	40(%r12), %rdi
.Ltmp11233:
	movq	24(%rsp), %rdx
	leaq	864(%rsp), %rsi
	vzeroupper
	callq	purrdf_sparql_eval::deferred_exists::nested_sites::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp11234:
	movq	%rax, 592(%rsp)
	movq	%rdx, 184(%rsp)
	movq	%rdx, 600(%rsp)
	leaq	48(%r12), %rdx
	movq	%rbp, 376(%rsp)
	movq	%rax, 192(%rsp)
	movq	$0, 352(%rsp)
	movq	$0, 360(%rsp)
.Ltmp11235:
	movq	<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>::then@GOTPCREL(%rip), %rax
	leaq	392(%rsp), %rdi
	leaq	1024(%rsp), %rsi
	xorl	%ebx, %ebx
	callq	*%rax
.Ltmp11236:
	movq	%rax, 464(%rsp)
	movq	%rdx, 472(%rsp)
.Ltmp11237:
	movq	<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>::layers@GOTPCREL(%rip), %rax
	leaq	48(%rsp), %rdi
	leaq	464(%rsp), %rsi
	callq	*%rax
.Ltmp11238:
	movq	48(%rsp), %rax
	movq	%r12, 336(%rsp)
	movq	56(%rsp), %r12
	movq	184(%rsp), %rcx
	movq	192(%rsp), %r15
	movq	%rax, 176(%rsp)
	movq	64(%rsp), %rax
	movq	%r12, 16(%rsp)
	testq	%rax, %rax
	je	.LBB313_185
	shlq	$4, %r15
	leaq	(%r12,%rax,8), %rax
	xorl	%ebp, %ebp
	xorl	%r14d, %r14d
	addq	%rcx, %r15
	movq	%rax, 512(%rsp)
	jmp	.LBB313_160
.LBB313_157:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB313_158:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbp, %rdi
	callq	*%rax
.LBB313_159:
	addq	$8, %r12
	movq	%r13, 352(%rsp)
	movq	%r13, %rbp
	movq	%rbx, %r14
	cmpq	512(%rsp), %r12
	je	.LBB313_186
.LBB313_160:
	movq	%rbp, %rsi
	testq	%rbp, %rbp
	jne	.LBB313_162
	movq	336(%rsp), %rax
	movq	40(%rax), %rsi
.LBB313_162:
	vmovups	anon.e5162873a9a3251d11c4df37a70e4654.29.llvm.18159039729619107857(%rip), %ymm0
	movq	(%r12), %rdx
	leaq	16(%r14), %rax
	testq	%r14, %r14
	cmoveq	%r14, %rax
	movq	%rax, 1136(%rsp)
	movq	%r15, 1144(%rsp)
	movq	$0, 1152(%rsp)
	vmovups	%ymm0, 1104(%rsp)
.Ltmp11240:
	leaq	48(%rsp), %rdi
	leaq	1104(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::expr::substitute_pattern_deferring::<false>
.Ltmp11241:
	movq	48(%rsp), %rax
	movq	56(%rsp), %r13
	cmpq	$-1, %rax
	jne	.LBB313_223
.Ltmp11259:
	movq	<purrdf_sparql_eval::expr::Deferral>::into_placeholders@GOTPCREL(%rip), %rax
	leaq	1104(%rsp), %rdi
	callq	*%rax
.Ltmp11260:
	movq	%rax, %rbx
	testq	%r14, %r14
	je	.LBB313_168
	lock		decq	(%r14)
	jne	.LBB313_168
	#MEMBARRIER
.Ltmp11262:
	movq	<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	360(%rsp), %rdi
	callq	*%rax
.Ltmp11263:
.LBB313_168:
	movq	%rbx, 360(%rsp)
	testq	%rbp, %rbp
	je	.LBB313_159
.Ltmp11267:
	movq	%rbp, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_algebra::algebra::GraphPattern>
.Ltmp11268:
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-144, %rcx
	movabsq	$-9223372036854775808, %rdx
	addq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB313_172
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB313_172:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB313_158
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB313_172
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	$-144, %rcx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	lock		xaddq	%rcx, (%rax)
	movabsq	$-9223372036854775808, %rax
	addq	$-144, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB313_175:
	cmpq	%rax, %rcx
	jge	.LBB313_157
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB313_175
	jmp	.LBB313_157
.LBB313_177:
	movq	%rbp, %r14
	xorl	%ebx, %ebx
.LBB313_178:
	movq	40(%r12), %rsi
.Ltmp11226:
	movq	24(%rsp), %r8
	leaq	48(%rsp), %rdi
	leaq	464(%rsp), %rdx
	leaq	864(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::binop::eval_substituted_delivered::<purrdf_core::ir::dataset::RdfDataset, false, false, ()>
.Ltmp11227:
	movq	%r14, %rbp
	testb	%bl, %bl
	je	.LBB313_183
	movq	24(%rsp), %rax
	movq	1112(%rax), %rax
	testq	%rax, %rax
	je	.LBB313_183
	movq	24(%rsp), %rdx
	leaq	-1(%rax), %rcx
	movq	%rcx, 1112(%rdx)
	movq	1104(%rdx), %rcx
	movq	-8(%rcx,%rax,8), %rax
	movq	%rax, 592(%rsp)
	lock		decq	(%rax)
	jne	.LBB313_183
	movq	<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::expr::SubstitutionSource, purrdf_hash::fixed::FixedState>>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	592(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB313_183:
	vmovups	96(%rsp), %zmm1
	vmovups	48(%rsp), %zmm0
	vmovups	%zmm1, 256(%rsp)
	vmovups	%zmm0, 208(%rsp)
.Ltmp11231:
	leaq	464(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp11232:
.LBB313_184:
.Ltmp11257:
	leaq	1024(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp11258:
	jmp	.LBB313_199
.LBB313_185:
	xorl	%ebx, %ebx
	xorl	%r13d, %r13d
.LBB313_186:
	movq	176(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB313_188
	movq	16(%rsp), %rdi
	shlq	$3, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB313_188:
.Ltmp11272:
	leaq	464(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>
.Ltmp11273:
	movq	336(%rsp), %rax
	testq	%r13, %r13
	jne	.LBB313_191
	movq	40(%rax), %r13
.LBB313_191:
	movq	$0, 48(%rsp)
.Ltmp11277:
	movq	24(%rsp), %rsi
	leaq	464(%rsp), %rdi
	leaq	48(%rsp), %rdx
	movq	%rbx, %rcx
	callq	<purrdf_sparql_eval::eval::EvalCtx>::enter_substituted_exists
.Ltmp11278:
	movq	376(%rsp), %rbp
	movq	488(%rsp), %rdx
.Ltmp11279:
	leaq	208(%rsp), %rdi
	movq	%r13, %rsi
	movq	%r13, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp11280:
.Ltmp11284:
	leaq	464(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::eval::EvalScopeGuard<purrdf_core::ir::dataset::RdfDataset>>
.Ltmp11285:
.Ltmp11289:
	leaq	352(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>>
.Ltmp11290:
	cmpq	$0, 192(%rsp)
	movq	184(%rsp), %rax
	je	.LBB313_198
	lock		decq	(%rax)
	jne	.LBB313_198
	#MEMBARRIER
.Ltmp11294:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::deferred_exists::NestedSites>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	600(%rsp), %rdi
	callq	*%rax
.Ltmp11295:
.LBB313_198:
.Ltmp11300:
	leaq	1024(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp11301:
.LBB313_199:
	cmpl	$1, 208(%rsp)
	je	.LBB313_320
	leaq	224(%rsp), %rcx
	movq	216(%rsp), %rax
	movq	%rbp, %r13
	vmovups	24(%rcx), %zmm1
	vmovups	(%rcx), %zmm0
	leaq	736(%rsp), %rcx
	vmovups	%zmm1, 728(%rsp)
	vmovups	%zmm0, 704(%rsp)
	vmovups	704(%rsp), %ymm0
	vmovups	24(%rcx), %ymm1
	vmovups	%ymm0, 992(%rsp)
	vmovups	(%rcx), %ymm0
.LBB313_201:
	vmovups	%ymm1, 552(%rsp)
	vmovups	%ymm0, 528(%rsp)
	cmpq	$-1, %rax
	jne	.LBB313_241
	movq	1016(%rsp), %rax
	movq	32(%rax), %rbx
	testq	%rbx, %rbx
	je	.LBB313_207
	movq	24(%rax), %r14
	shlq	$4, %rbx
	addq	%r14, %rbx
	.p2align	4
.LBB313_204:
	movq	(%r14), %rax
	lock		incq	(%rax)
	jle	.LBB313_339
	movq	8(%r14), %rdx
	movq	(%r14), %rsi
.Ltmp11310:
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	800(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp11311:
	addq	$16, %r14
	cmpq	%rbx, %r14
	jne	.LBB313_204
.LBB313_207:
	movq	32(%rsp), %rax
	movq	(%rax), %r14
	leaq	-1(%r14), %rdx
	cmpq	$4, %rdx
	jbe	.LBB313_212
	movq	16(%rax), %r14
	movq	8(%rax), %rsi
	leaq	-8(,%r14,8), %rdx
	leaq	-1(%r14), %r15
	cmpq	$5, %r15
	jb	.LBB313_213
	movq	%rsi, %r12
	movl	$4, %esi
	movq	%rdx, %rdi
	movq	%rdx, %rbx
	vzeroupper
	callq	__rustc::__rust_alloc
	testq	%rax, %rax
	je	.LBB313_337
	movb	$61, %cl
	leaq	-2(%r14), %rsi
	movq	%r12, %rdi
	bzhiq	%rcx, %r15, %rcx
	cmpq	%rsi, %rcx
	cmovbq	%rcx, %rsi
	cmpq	$16, %rsi
	jae	.LBB313_214
	movq	%r15, %rcx
	movq	%rdi, %rdx
	xorl	%esi, %esi
	movq	%r13, %rbp
	jmp	.LBB313_216
.LBB313_212:
	leaq	8(%rax), %rsi
	shlq	$3, %rdx
.LBB313_213:
	movq	memcpy@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdi
	movq	%r13, %rbp
	vzeroupper
	callq	*%rax
	jmp	.LBB313_220
.LBB313_214:
	incq	%rsi
	movl	$16, %edx
	movq	%r13, %rbp
	movl	%esi, %ecx
	andl	$15, %ecx
	cmoveq	%rdx, %rcx
	xorl	%r8d, %r8d
	subq	%rcx, %rsi
	movq	%r15, %rcx
	leaq	(%rdi,%rsi,8), %rdx
	subq	%rsi, %rcx
.LBB313_215:
	vmovups	(%rdi,%r8,8), %zmm0
	vmovups	64(%rdi,%r8,8), %zmm1
	vmovups	%zmm1, 64(%rax,%r8,8)
	vmovups	%zmm0, (%rax,%r8,8)
	addq	$16, %r8
	cmpq	%r8, %rsi
	jne	.LBB313_215
.LBB313_216:
	leaq	(%rdi,%r15,8), %rdi
	leaq	4(%rax,%rsi,8), %rsi
	xorl	%r8d, %r8d
.LBB313_217:
	cmpq	%rdi, %rdx
	je	.LBB313_219
	movl	(%rdx), %r9d
	movl	4(%rdx), %r10d
	addq	$8, %rdx
	movl	%r9d, -4(%rsi,%r8,8)
	movl	%r10d, (%rsi,%r8,8)
	incq	%r8
	cmpq	%r8, %rcx
	jne	.LBB313_217
.LBB313_219:
	movq	%rax, 208(%rsp)
	movq	%r14, 216(%rsp)
.LBB313_220:
	vmovups	992(%rsp), %ymm0
	leaq	56(%rsp), %rcx
	movq	208(%rsp), %rax
	movq	216(%rsp), %rdx
	vmovups	%ymm0, 32(%rcx)
	vmovups	224(%rsp), %xmm0
	movq	%r14, 48(%rsp)
	movq	%rax, (%rcx)
	movq	%rdx, 8(%rcx)
	vmovups	%xmm0, 16(%rcx)
	movq	8(%rsp), %rcx
	cmpq	408(%rsp), %rcx
	jne	.LBB313_124
.Ltmp11318:
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::governor::soundness::NodeAnalysis>>::grow_one@GOTPCREL(%rip), %rax
	leaq	408(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp11319:
	movq	416(%rsp), %rbp
	jmp	.LBB313_124
.LBB313_223:
	leaq	56(%rsp), %rcx
	vmovups	8(%rcx), %zmm0
	vmovups	24(%rcx), %zmm1
	leaq	224(%rsp), %rcx
	vmovups	%zmm1, 32(%rcx)
	vmovups	%zmm0, 16(%rcx)
	movq	%rax, 224(%rsp)
	movq	%r13, 232(%rsp)
	movq	$1, 208(%rsp)
.Ltmp11245:
	leaq	1104(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>
.Ltmp11246:
	movq	176(%rsp), %rsi
	movq	16(%rsp), %rdi
	testq	%rsi, %rsi
	je	.LBB313_226
	shlq	$3, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB313_226:
.Ltmp11248:
	leaq	464(%rsp), %rdi
	movq	%r14, %rbx
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>
.Ltmp11249:
	testq	%r14, %r14
	je	.LBB313_230
	lock		decq	(%r14)
	jne	.LBB313_230
	#MEMBARRIER
.Ltmp11250:
	movq	<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	360(%rsp), %rdi
	callq	*%rax
.Ltmp11251:
.LBB313_230:
.Ltmp11253:
	leaq	352(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>>
.Ltmp11254:
	movq	376(%rsp), %rbp
	cmpq	$0, 192(%rsp)
	movq	184(%rsp), %rax
	je	.LBB313_184
	lock		decq	(%rax)
	jne	.LBB313_184
	#MEMBARRIER
.Ltmp11255:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::deferred_exists::NestedSites>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	600(%rsp), %rdi
	callq	*%rax
.Ltmp11256:
	jmp	.LBB313_184
.LBB313_234:
	movb	$1, %al
	testb	%al, %al
	je	.LBB313_133
.LBB313_235:
.Ltmp11211:
	movq	purrdf_stack::is_low_cold@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp11212:
	testb	%al, %al
	je	.LBB313_133
	movabsq	$-9223372036854775784, %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.438(%rip), %rcx
	movq	%rax, 224(%rsp)
	movq	%rcx, 232(%rsp)
	movq	$41, 240(%rsp)
	jmp	.LBB313_136
.LBB313_238:
	vmovups	704(%rsp), %ymm0
	leaq	56(%rsp), %rax
	vmovups	%ymm0, (%rax)
	movq	$-1, %rax
.LBB313_239:
	vmovups	64(%rsp), %xmm0
	movq	56(%rsp), %rsi
	movq	80(%rsp), %rdi
	movq	%rdi, 688(%rsp)
	jmp	.LBB313_49
.LBB313_240:
	movq	$0, 408(%rsp)
	movq	$8, 416(%rsp)
	movq	$0, 424(%rsp)
	movl	$8, %ebp
	movq	$0, 8(%rsp)
	jmp	.LBB313_245
.LBB313_241:
	vmovups	992(%rsp), %ymm0
	vmovups	528(%rsp), %ymm2
	vmovups	552(%rsp), %ymm1
	movq	%rax, 48(%rsp)
	vmovups	%ymm0, 56(%rsp)
	vmovups	%ymm2, 88(%rsp)
	vmovups	%ymm1, 112(%rsp)
.Ltmp11305:
	leaq	208(%rsp), %rdi
	leaq	888(%rsp), %rsi
	leaq	48(%rsp), %rcx
	movl	$1, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp11306:
	cmpq	$-1, 208(%rsp)
	je	.LBB313_244
.Ltmp11307:
	leaq	208(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp11308:
.LBB313_244:
	movq	%r13, %rbp
.LBB313_245:
	movq	168(%rsp), %rax
	leaq	16(%rax), %rsi
.Ltmp11332:
	movq	<purrdf_sparql_eval::solution::VarSchema>::union@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdi
	leaq	800(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp11333:
	vmovups	208(%rsp), %ymm0
	vmovups	232(%rsp), %ymm1
	movq	$1, 48(%rsp)
	movq	$1, 56(%rsp)
	vmovups	%ymm0, 64(%rsp)
	vmovups	%ymm1, 88(%rsp)
.Ltmp11335:
	movl	$8, %edi
	movl	$72, %esi
	vzeroupper
	callq	alloc::boxed::box_new_uninit
.Ltmp11336:
	movq	112(%rsp), %rcx
	movq	%rax, 16(%rsp)
	movq	%rcx, 64(%rax)
	movq	168(%rsp), %rcx
	vmovups	48(%rsp), %zmm0
	vmovups	%zmm0, (%rax)
	movq	%rax, 464(%rsp)
	movq	32(%rcx), %rsi
	movq	32(%rax), %r13
	testq	%r13, %r13
	je	.LBB313_252
	movq	24(%rsp), %rax
	movq	616(%rax), %rax
	testq	%rax, %rax
	je	.LBB313_252
	movq	32(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB313_252
	movq	%rax, %rdx
	orq	%r13, %rdx
	movabsq	$230584300921369396, %rcx
	shrq	$32, %rdx
	je	.LBB313_328
	xorl	%edx, %edx
	divq	%r13
	jmp	.LBB313_329
.LBB313_252:
	movq	8(%rsp), %r14
	movl	$0, 328(%rsp)
.LBB313_253:
	movq	%rsi, 344(%rsp)
	movq	%r13, 336(%rsp)
	testq	%r14, %r14
	je	.LBB313_256
	leaq	(,%r14,8), %rax
	movl	$8, %esi
	movq	%rbp, %r12
	leaq	(%rax,%rax,4), %rbx
	movq	%rbx, %rdi
	vzeroupper
	callq	__rustc::__rust_alloc
	testq	%rax, %rax
	je	.LBB313_338
	movq	%rax, %rdx
	movq	%r12, %rbp
	jmp	.LBB313_257
.LBB313_256:
	movl	$8, %edx
.LBB313_257:
	movq	8(%rsp), %rax
	movq	%r14, 528(%rsp)
	movq	%rdx, 536(%rsp)
	movq	$0, 544(%rsp)
	testq	%rax, %rax
	je	.LBB313_312
	leaq	(%rax,%rax,8), %rax
	movq	16(%rsp), %rcx
	movq	$0, 8(%rsp)
	leaq	(%rbp,%rax,8), %rax
	movq	%rax, 376(%rsp)
	movq	344(%rsp), %rax
	addq	$16, %rcx
	movq	%rcx, 176(%rsp)
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.449(%rip), %rcx
	movq	%rcx, 368(%rsp)
	leaq	(,%rax,8), %rax
	movq	%rax, 192(%rsp)
	jmp	.LBB313_260
.LBB313_259:
	movq	%r13, %rbp
	addq	$72, %rbp
	cmpq	376(%rsp), %rbp
	je	.LBB313_312
.LBB313_260:
	movq	64(%rbp), %rsi
	movq	%rdx, %r15
	addq	$16, %rsi
.Ltmp11344:
	movq	176(%rsp), %rdx
	movq	purrdf_sparql_eval::binop::right_to_out_map@GOTPCREL(%rip), %rax
	leaq	704(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp11345:
	movq	56(%rbp), %rax
	movq	%rbp, %r13
	testq	%rax, %rax
	je	.LBB313_306
	movq	48(%rbp), %r12
	leaq	8(%rbp), %rcx
	movq	712(%rsp), %rbp
	movq	720(%rsp), %r14
	leaq	(%rax,%rax,4), %rax
	movq	%r15, %rdx
	movq	%rcx, 32(%rsp)
	leaq	(%r12,%rax,8), %rax
	movq	%rax, 512(%rsp)
	jmp	.LBB313_265
.LBB313_263:
	movq	8(%rsp), %rsi
	leaq	(%rsi,%rsi,4), %rax
	incq	%rsi
	movq	%rsi, 8(%rsp)
	movq	%r15, (%rdx,%rax,8)
	movq	%rbx, 8(%rdx,%rax,8)
	vmovaps	208(%rsp), %xmm0
	vmovups	%xmm0, 16(%rdx,%rax,8)
	movq	224(%rsp), %rcx
	movq	%rcx, 32(%rdx,%rax,8)
	movq	%rsi, 544(%rsp)
	addq	$40, %r12
	cmpq	512(%rsp), %r12
	je	.LBB313_307
.LBB313_265:
	movq	(%r12), %rcx
	leaq	8(%r12), %rbx
	movq	%rdx, 200(%rsp)
	movq	%rbx, %rax
	decq	%rcx
	cmpq	$5, %rcx
	jb	.LBB313_267
	movq	16(%r12), %rcx
	movq	8(%r12), %rax
	decq	%rcx
.LBB313_267:
	testq	%rcx, %rcx
	je	.LBB313_279
	leaq	(%rax,%rcx,8), %rcx
	xorl	%edx, %edx
	jmp	.LBB313_270
	.p2align	4
.LBB313_269:
	leaq	(%rax,%rdx,8), %rsi
	incq	%rdx
	addq	$8, %rsi
	cmpq	%rcx, %rsi
	je	.LBB313_279
.LBB313_270:
	cmpq	%rdx, %r14
	je	.LBB313_334
	movq	(%r13), %r8
	movq	32(%rsp), %rsi
	decq	%r8
	cmpq	$4, %r8
	jbe	.LBB313_273
	movq	16(%r13), %r8
	movq	8(%r13), %rsi
	decq	%r8
.LBB313_273:
	movq	(%rbp,%rdx,8), %rdi
	cmpq	%r8, %rdi
	jae	.LBB313_269
	movl	(%rax,%rdx,8), %r8d
	cmpl	$2, %r8d
	je	.LBB313_269
	movl	(%rsi,%rdi,8), %r9d
	cmpl	$2, %r9d
	je	.LBB313_269
	cmpl	%r9d, %r8d
	jne	.LBB313_305
	movl	4(%rsi,%rdi,8), %esi
	cmpl	%esi, 4(%rax,%rdx,8)
	je	.LBB313_269
.LBB313_305:
	movq	200(%rsp), %rdx
	addq	$40, %r12
	cmpq	512(%rsp), %r12
	jne	.LBB313_265
	jmp	.LBB313_307
.LBB313_279:
	cmpb	$0, 328(%rsp)
	je	.LBB313_281
	movq	8(%rsp), %rax
	cmpq	184(%rsp), %rax
	jae	.LBB313_309
.LBB313_281:
.Ltmp11352:
	movq	336(%rsp), %rsi
	leaq	48(%rsp), %rdi
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::from_elem
.Ltmp11353:
	movq	48(%rsp), %r15
	leaq	56(%rsp), %rdi
	movq	%r15, %rdx
	cmpq	$6, %r15
	jb	.LBB313_284
	movq	56(%rsp), %rdi
	movq	64(%rsp), %rdx
.LBB313_284:
	movq	344(%rsp), %r8
	decq	%rdx
	cmpq	%rdx, %r8
	ja	.LBB313_332
	movq	(%r13), %rax
	movq	16(%r13), %rsi
	decq	%rax
	decq	%rsi
	cmpq	$5, %rax
	cmovbq	%rax, %rsi
	cmpq	%rsi, %r8
	jne	.LBB313_333
	movq	32(%rsp), %rsi
	cmpq	$5, %rax
	jb	.LBB313_288
	movq	32(%rsp), %rax
	movq	(%rax), %rsi
.LBB313_288:
	movq	192(%rsp), %rdx
	movq	memcpy@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	(%r12), %rax
	decq	%rax
	cmpq	$5, %rax
	jb	.LBB313_290
	movq	16(%r12), %rax
	movq	8(%r12), %rbx
	decq	%rax
.LBB313_290:
	testq	%rax, %rax
	je	.LBB313_302
	shlq	$3, %rax
	leaq	56(%rsp), %r9
	xorl	%edi, %edi
	jmp	.LBB313_294
	.p2align	4
.LBB313_292:
	movl	4(%rbx,%rdi,8), %esi
	movl	%ecx, (%r8,%rdx,8)
	movl	%esi, 4(%r8,%rdx,8)
.LBB313_293:
	incq	%rdi
	addq	$-8, %rax
	je	.LBB313_301
.LBB313_294:
	movl	(%rbx,%rdi,8), %ecx
	cmpl	$2, %ecx
	je	.LBB313_293
	cmpq	%r14, %rdi
	jae	.LBB313_336
	movq	48(%rsp), %rsi
	movq	%rsi, %r8
	cmpq	$6, %rsi
	jb	.LBB313_298
	movq	64(%rsp), %r8
.LBB313_298:
	movq	(%rbp,%rdi,8), %rdx
	decq	%r8
	cmpq	%r8, %rdx
	jae	.LBB313_335
	movq	%r9, %r8
	cmpq	$6, %rsi
	jb	.LBB313_292
	movq	56(%rsp), %r8
	jmp	.LBB313_292
.LBB313_301:
	movq	48(%rsp), %r15
.LBB313_302:
	leaq	64(%rsp), %rax
	movq	56(%rsp), %rbx
	movq	200(%rsp), %rdx
	movq	8(%rsp), %rcx
	vmovups	(%rax), %xmm0
	movq	16(%rax), %rax
	movq	%rax, 224(%rsp)
	vmovaps	%xmm0, 208(%rsp)
	cmpq	528(%rsp), %rcx
	jne	.LBB313_263
.Ltmp11359:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	528(%rsp), %rdi
	callq	*%rax
.Ltmp11360:
	movq	536(%rsp), %rdx
	jmp	.LBB313_263
.LBB313_306:
	movq	%r15, %rdx
.LBB313_307:
	movq	704(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB313_259
	movq	712(%rsp), %rdi
	movq	%rdx, %rbx
	shlq	$3, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
	movq	%rbx, %rdx
	jmp	.LBB313_259
.LBB313_309:
	movq	8(%rsp), %rdx
	incq	%rdx
.Ltmp11349:
	movq	24(%rsp), %rsi
	movq	336(%rsp), %rcx
	leaq	48(%rsp), %rdi
	callq	<purrdf_sparql_eval::eval::EvalCtx>::observe_cells
.Ltmp11350:
	movq	704(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB313_312
	shlq	$3, %rsi
	movl	$8, %edx
	movq	%rbp, %rdi
	callq	__rustc::__rust_dealloc
.LBB313_312:
	movq	528(%rsp), %rcx
	movq	544(%rsp), %rax
	movq	536(%rsp), %r8
	movq	%rcx, 208(%rsp)
	movq	16(%rsp), %rcx
	movq	%rax, 224(%rsp)
	movq	%r8, 216(%rsp)
	movq	%rcx, 232(%rsp)
.Ltmp11367:
	leaq	48(%rsp), %rdi
	leaq	888(%rsp), %rsi
	leaq	208(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::finish::<purrdf_core::ir::term::TermId>
.Ltmp11368:
	vmovups	48(%rsp), %zmm0
	vmovups	80(%rsp), %zmm1
	movq	40(%rsp), %rax
	xorl	%ebp, %ebp
	vmovups	%zmm1, 40(%rax)
	vmovups	%zmm0, 8(%rax)
	movq	$0, (%rax)
.Ltmp11372:
	leaq	408(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>>
.Ltmp11373:
	xorl	%ebp, %ebp
.Ltmp11374:
	leaq	800(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.18159039729619107857)
.Ltmp11375:
	movq	168(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB313_317
	xorl	%ebp, %ebp
	#MEMBARRIER
.Ltmp11377:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	520(%rsp), %rdi
	callq	*%rax
.Ltmp11378:
.LBB313_317:
	cmpq	$0, 384(%rsp)
	je	.LBB313_319
	xorl	%ebp, %ebp
.Ltmp11379:
	leaq	384(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::DeferredLateral>
.Ltmp11380:
.LBB313_319:
	xorl	%ebp, %ebp
.Ltmp11381:
	leaq	432(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp11382:
	jmp	.LBB313_118
.LBB313_320:
	leaq	224(%rsp), %rax
	vmovups	32(%rax), %zmm1
	vmovups	(%rax), %zmm0
	vmovups	%zmm1, 736(%rsp)
	vmovups	%zmm0, 704(%rsp)
	vmovups	704(%rsp), %zmm0
	vmovups	736(%rsp), %zmm1
	jmp	.LBB313_322
.LBB313_321:
	leaq	56(%rsp), %rax
	vmovups	40(%rax), %zmm1
	vmovups	8(%rax), %zmm0
	vmovups	%zmm1, 240(%rsp)
	vmovups	%zmm0, 208(%rsp)
	vmovups	208(%rsp), %zmm0
	vmovups	240(%rsp), %zmm1
.LBB313_322:
	movq	40(%rsp), %rax
	movb	$1, %bpl
	vmovups	%zmm1, 48(%rax)
	vmovups	%zmm0, 16(%rax)
	movq	$1, (%rax)
.Ltmp11324:
	leaq	408(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>>
.Ltmp11325:
	movb	$1, %bpl
.Ltmp11326:
	leaq	800(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.18159039729619107857)
.Ltmp11327:
	movq	168(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB313_326
	movb	$1, %bpl
	#MEMBARRIER
.Ltmp11328:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	520(%rsp), %rdi
	callq	*%rax
.Ltmp11329:
.LBB313_326:
	cmpq	$0, 384(%rsp)
	je	.LBB313_57
	movb	$1, %bpl
.Ltmp11330:
	leaq	384(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::DeferredLateral>
.Ltmp11331:
	jmp	.LBB313_57
.LBB313_328:
	xorl	%edx, %edx
	divl	%r13d
.LBB313_329:
	movq	%rax, 184(%rsp)
	cmpq	%rcx, %rax
	jae	.LBB313_331
	movq	8(%rsp), %r14
	cmpq	%r14, %rax
	cmovbq	%rax, %r14
	movb	$1, %al
	movl	%eax, 328(%rsp)
	jmp	.LBB313_253
.LBB313_331:
	movq	8(%rsp), %r14
	movl	$0, 328(%rsp)
	jmp	.LBB313_253
.LBB313_332:
.Ltmp11362:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.451(%rip), %rcx
	xorl	%edi, %edi
	movq	%r8, %rsi
	callq	*%rax
.Ltmp11363:
	jmp	.LBB313_339
.LBB313_333:
.Ltmp11355:
	movq	core::slice::copy_from_slice_impl::len_mismatch_fail@GOTPCREL(%rip), %rax
	movq	344(%rsp), %rdi
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.448(%rip), %rdx
	callq	*%rax
.Ltmp11356:
	jmp	.LBB313_339
.LBB313_334:
.Ltmp11347:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.710(%rip), %rdx
	movq	%r14, %rdi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp11348:
	jmp	.LBB313_339
.LBB313_335:
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.450(%rip), %rax
	movq	%rdx, %rdi
	movq	%r8, %r14
	movq	%rax, 368(%rsp)
.LBB313_336:
.Ltmp11357:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	movq	368(%rsp), %rdx
	movq	%r14, %rsi
	callq	*%rax
.Ltmp11358:
	jmp	.LBB313_339
.LBB313_337:
.Ltmp11313:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$4, %edi
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp11314:
	jmp	.LBB313_339
.LBB313_338:
.Ltmp11341:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp11342:
.LBB313_339:
	ud2
.LBB313_340:
.Ltmp11252:
	jmp	.LBB313_356
.LBB313_341:
.Ltmp11247:
	movq	%rax, %r12
	jmp	.LBB313_375
.LBB313_342:
.Ltmp11309:
	jmp	.LBB313_370
.LBB313_343:
.Ltmp11343:
	movq	16(%rsp), %r14
	movq	%rax, %r12
	jmp	.LBB313_403
.LBB313_344:
.Ltmp11219:
	movq	%rax, %r12
.Ltmp11220:
	leaq	48(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp11221:
	jmp	.LBB313_385
.LBB313_345:
.Ltmp11281:
	movq	%rax, %r12
.Ltmp11282:
	leaq	464(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::eval::EvalScopeGuard<purrdf_core::ir::dataset::RdfDataset>>
.Ltmp11283:
	jmp	.LBB313_381
.LBB313_346:
.Ltmp11369:
	movq	%rax, %r12
	xorl	%ebp, %ebp
	jmp	.LBB313_405
.LBB313_347:
.Ltmp11337:
	movq	%rax, %r12
.Ltmp11338:
	leaq	64(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.18159039729619107857)
.Ltmp11339:
	jmp	.LBB313_363
.LBB313_349:
.Ltmp11340:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB313_350:
.Ltmp11334:
	jmp	.LBB313_370
.LBB313_351:
.Ltmp11228:
	movq	%rax, %r12
.Ltmp11229:
	leaq	464(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp11230:
	jmp	.LBB313_385
.LBB313_352:
.Ltmp11291:
	movq	%rax, %r12
	jmp	.LBB313_382
.LBB313_353:
.Ltmp11239:
	movq	%rax, %r12
	xorl	%r14d, %r14d
	jmp	.LBB313_377
.LBB313_354:
.Ltmp11376:
	movq	%rax, %r12
	jmp	.LBB313_408
.LBB313_355:
.Ltmp11286:
.LBB313_356:
	movq	%rax, %r12
	jmp	.LBB313_381
.LBB313_357:
.Ltmp11315:
	jmp	.LBB313_393
.LBB313_358:
.Ltmp11274:
	movq	%rax, %r12
	movq	%rbx, %r14
	jmp	.LBB313_378
.LBB313_359:
.Ltmp11346:
	movq	16(%rsp), %r14
	movq	%rax, %r12
	jmp	.LBB313_402
.LBB313_360:
.Ltmp11264:
	movq	%rax, %r12
	movq	%rbx, 360(%rsp)
	movq	%rbx, %r14
	jmp	.LBB313_373
.LBB313_361:
.Ltmp11296:
	movq	%rax, %r12
	jmp	.LBB313_385
.LBB313_362:
.Ltmp11320:
	movq	%rax, %r12
.Ltmp11321:
	leaq	48(%rsp), %rdi
	callq	core::ptr::drop_glue::<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>
.Ltmp11322:
.LBB313_363:
	movb	$1, %bpl
	jmp	.LBB313_405
.LBB313_364:
.Ltmp11323:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB313_365:
.Ltmp11411:
	movq	%rax, %r12
	xorl	%ebp, %ebp
	jmp	.LBB313_412
.LBB313_366:
.Ltmp11392:
	movq	%rax, %r12
	jmp	.LBB313_410
.LBB313_367:
.Ltmp11269:
	movl	$144, %esi
	movl	$8, %edx
	movq	%rbp, %rdi
	movq	%rax, %r12
	callq	__rustc::__rust_dealloc
	movq	%r13, 352(%rsp)
	movq	%rbx, %r14
	jmp	.LBB313_375
.LBB313_368:
.Ltmp11406:
	movq	%rax, %r12
	movb	$1, %bpl
	jmp	.LBB313_412
.LBB313_369:
.Ltmp11304:
.LBB313_370:
	movq	%rax, %r12
	movb	$1, %bpl
	jmp	.LBB313_405
.LBB313_371:
.Ltmp11416:
	movq	%rax, %r12
	jmp	.LBB313_414
.LBB313_372:
.Ltmp11261:
	movq	%rax, %r12
.LBB313_373:
.Ltmp11265:
	movq	%r13, %rdi
	callq	core::ptr::drop_glue::<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>
.Ltmp11266:
	jmp	.LBB313_375
.LBB313_374:
.Ltmp11242:
	movq	%rax, %r12
.Ltmp11243:
	leaq	1104(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>
.Ltmp11244:
.LBB313_375:
	cmpq	$0, 176(%rsp)
	je	.LBB313_377
	movq	176(%rsp), %rsi
	movq	16(%rsp), %rdi
	movl	$8, %edx
	shlq	$3, %rsi
	callq	__rustc::__rust_dealloc
.LBB313_377:
.Ltmp11270:
	leaq	464(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>
.Ltmp11271:
.LBB313_378:
	testq	%r14, %r14
	je	.LBB313_381
	lock		decq	(%r14)
	jne	.LBB313_381
	#MEMBARRIER
.Ltmp11275:
	movq	<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	360(%rsp), %rdi
	callq	*%rax
.Ltmp11276:
.LBB313_381:
.Ltmp11287:
	leaq	352(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>>
.Ltmp11288:
.LBB313_382:
	cmpq	$0, 192(%rsp)
	je	.LBB313_385
	movq	184(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB313_385
	#MEMBARRIER
.Ltmp11292:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::deferred_exists::NestedSites>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	600(%rsp), %rdi
	callq	*%rax
.Ltmp11293:
.LBB313_385:
	movb	$1, %bpl
.Ltmp11297:
	leaq	1024(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp11298:
	jmp	.LBB313_405
.LBB313_386:
.Ltmp11299:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB313_387:
.Ltmp11206:
	movq	%rax, %r12
	xorl	%ebp, %ebp
	jmp	.LBB313_414
.LBB313_388:
.Ltmp11361:
	movq	%rax, %r12
	cmpq	$6, %r15
	jb	.LBB313_397
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	movq	%rbx, %rdi
	callq	__rustc::__rust_dealloc
	jmp	.LBB313_397
.LBB313_390:
.Ltmp11354:
	jmp	.LBB313_396
.LBB313_391:
.Ltmp11403:
	movq	%rax, %r12
	jmp	.LBB313_412
.LBB313_392:
.Ltmp11312:
.LBB313_393:
	movb	$1, %bpl
	movq	%rax, %r12
.Ltmp11316:
	leaq	992(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp11317:
	jmp	.LBB313_405
.LBB313_394:
.Ltmp11419:
	movq	%rax, %r12
	jmp	.LBB313_415
.LBB313_395:
.Ltmp11351:
.LBB313_396:
	movq	%rax, %r12
.LBB313_397:
	movq	16(%rsp), %r14
	jmp	.LBB313_400
.LBB313_398:
.Ltmp11364:
	movq	%rax, %r12
	movq	48(%rsp), %rax
	movq	16(%rsp), %r14
	cmpq	$6, %rax
	jb	.LBB313_400
	movq	56(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB313_400:
	movq	704(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB313_402
	movq	712(%rsp), %rdi
	shlq	$3, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB313_402:
	leaq	528(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB313_403:
	lock		decq	(%r14)
	movb	$1, %bpl
	jne	.LBB313_405
	#MEMBARRIER
.Ltmp11365:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	464(%rsp), %rdi
	callq	*%rax
.Ltmp11366:
.LBB313_405:
.Ltmp11370:
	leaq	408(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>>
.Ltmp11371:
	jmp	.LBB313_407
.LBB313_406:
.Ltmp11385:
	movq	%rax, %r12
.LBB313_407:
.Ltmp11386:
	leaq	800(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.18159039729619107857)
.Ltmp11387:
.LBB313_408:
	movq	168(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB313_410
	#MEMBARRIER
.Ltmp11388:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	520(%rsp), %rdi
	callq	*%rax
.Ltmp11389:
.LBB313_410:
	cmpq	$0, 384(%rsp)
	je	.LBB313_412
.Ltmp11393:
	leaq	384(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::DeferredLateral>
.Ltmp11394:
.LBB313_412:
	movq	456(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB313_414
	leaq	456(%rsp), %rdi
	#MEMBARRIER
.Ltmp11412:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp11413:
.LBB313_414:
	leaq	432(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB313_415:
	testb	%bpl, %bpl
	je	.LBB313_417
.Ltmp11420:
	leaq	888(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp11421:
.LBB313_417:
	movq	%r12, %rdi
	callq	_Unwind_Resume@PLT
.LBB313_418:
.Ltmp11422:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end313:
