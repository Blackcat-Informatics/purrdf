purrdf_sparql_eval::binop::eval_application::<purrdf_core::ir::dataset::RdfDataset, ()>:
.Lfunc_begin315:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception222
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
	movq	%rdi, 56(%rsp)
	leaq	888(%rsp), %rdi
	movq	%r8, %r12
	movq	%rcx, %r13
	movq	%rdx, %r14
	callq	*%rax
	movb	$1, %bpl
.Ltmp11255:
	leaq	1344(%rsp), %rdi
	movq	%r14, %rsi
	movq	%r12, %rdx
	movq	%r14, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp11256:
	cmpl	$1, 1344(%rsp)
	jne	.LBB315_3
	vmovups	1360(%rsp), %zmm0
	vmovups	1392(%rsp), %zmm1
	movq	56(%rsp), %rax
	vmovups	%zmm1, 48(%rax)
	vmovups	%zmm0, 16(%rax)
	movq	$1, (%rax)
	jmp	.LBB315_95
.LBB315_3:
	vmovups	1384(%rsp), %zmm1
	vmovups	1352(%rsp), %zmm0
	vmovups	%zmm1, 96(%rsp)
	vmovups	%zmm0, 64(%rsp)
.Ltmp11257:
	leaq	224(%rsp), %rdi
	leaq	888(%rsp), %rsi
	leaq	64(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp11258:
	cmpq	$-1, 224(%rsp)
	je	.LBB315_9
	vmovups	224(%rsp), %ymm0
	cmpq	$-1, 888(%rsp)
	vmovups	%ymm0, 480(%rsp)
	je	.LBB315_11
	vmovups	928(%rsp), %zmm1
	vmovups	888(%rsp), %zmm0
	movq	504(%rsp), %rax
	movq	%rax, 552(%rsp)
	movq	$0, 528(%rsp)
	movq	$8, 536(%rsp)
	movq	$0, 544(%rsp)
	vmovups	%zmm1, 104(%rsp)
	vmovups	%zmm0, 64(%rsp)
	cmpq	$-1, 64(%rsp)
	je	.LBB315_18
	leaq	224(%rsp), %rdi
	leaq	528(%rsp), %rsi
	leaq	888(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	136(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB315_8
.LBB315_19:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	144(%rsp), %rdi
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
	jge	.LBB315_21
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB315_21:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB315_27
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB315_21
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
.LBB315_24:
	cmpq	%rax, %rsi
	jge	.LBB315_26
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB315_24
.LBB315_26:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB315_27:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	160(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB315_28
	jmp	.LBB315_30
.LBB315_9:
	xorl	%ebp, %ebp
.Ltmp11475:
	leaq	64(%rsp), %rdi
	leaq	888(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
.Ltmp11476:
	vmovups	64(%rsp), %zmm0
	vmovups	96(%rsp), %zmm1
	movq	56(%rsp), %rax
	vmovups	%zmm1, 40(%rax)
	vmovups	%zmm0, 8(%rax)
	movq	$0, (%rax)
	jmp	.LBB315_118
.LBB315_11:
	cmpl	$28, (%r13)
	movq	%r12, 40(%rsp)
	jne	.LBB315_31
	movq	624(%r12), %rax
	xorl	%r14d, %r14d
	testq	%rax, %rax
	je	.LBB315_43
	testb	$1, 1200(%r12)
	je	.LBB315_44
	movq	1208(%r12), %rcx
	cmpq	40(%rax), %rcx
	jne	.LBB315_37
	movl	1216(%r12), %ecx
	subl	80(%rax), %ecx
	jb	.LBB315_37
	cmpq	%rcx, 32(%rax)
	jbe	.LBB315_37
	movq	24(%rax), %rax
	shlq	$4, %rcx
	movq	(%rax,%rcx), %r14
	movq	8(%rax,%rcx), %r15
	jmp	.LBB315_44
.LBB315_18:
	movq	536(%rsp), %rcx
	movq	528(%rsp), %rax
	movq	544(%rsp), %rdx
	movq	%rcx, 240(%rsp)
	movq	552(%rsp), %rcx
	movq	%rax, 232(%rsp)
	movq	%rdx, 248(%rsp)
	movq	%rcx, 256(%rsp)
	movq	$-1, 224(%rsp)
	movq	136(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB315_19
.LBB315_8:
	movq	160(%rsp), %rax
	testq	%rax, %rax
	je	.LBB315_30
.LBB315_28:
	lock		decq	(%rax)
	jne	.LBB315_30
	leaq	160(%rsp), %rdi
	#MEMBARRIER
.Ltmp11259:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp11260:
.LBB315_30:
	vmovups	224(%rsp), %zmm0
	vmovups	256(%rsp), %zmm1
	movq	56(%rsp), %rax
	movl	$0, 48(%rsp)
	vmovups	%zmm1, 40(%rax)
	vmovups	%zmm0, 8(%rax)
	movq	$0, (%rax)
	jmp	.LBB315_71
.LBB315_31:
	movb	$1, %bpl
.Ltmp11262:
	leaq	64(%rsp), %rdi
	leaq	480(%rsp), %rsi
	movq	%r13, 216(%rsp)
	movq	%r13, %rdx
	movq	%r12, %rcx
	vzeroupper
	callq	purrdf_sparql_eval::service_endpoints::admit_lateral_endpoints::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp11263:
	cmpq	$-1, 64(%rsp)
	jne	.LBB315_42
	movq	40(%rsp), %rax
	movq	216(%rsp), %rbx
	movq	584(%rax), %rsi
	testq	%rsi, %rsi
	je	.LBB315_38
	movq	%rbx, %rdi
	callq	purrdf_sparql_eval::binop::eval_application::<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#0}
	testq	%rax, %rax
	je	.LBB315_38
	cmpl	$1, (%rax)
	jne	.LBB315_38
	addq	$8, %rax
	leaq	432(%rsp), %rdi
	movq	%rax, %rsi
	callq	<purrdf_sparql_eval::deferred_exists::DeferredLateral as core::clone::Clone>::clone
	cmpq	$0, 432(%rsp)
	jne	.LBB315_59
	jmp	.LBB315_39
.LBB315_37:
	xorl	%r14d, %r14d
.LBB315_43:
.LBB315_44:
	movb	$1, %bpl
.Ltmp11453:
	leaq	736(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r13, %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::enter_node
.Ltmp11454:
.Ltmp11455:
	leaq	1080(%rsp), %rdi
	movq	%r12, %rsi
	xorl	%edx, %edx
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge
.Ltmp11456:
	cmpb	$-1, 1080(%rsp)
	leaq	1200(%r12), %rbx
	je	.LBB315_54
.Ltmp11457:
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	callq	*%rax
.Ltmp11458:
	vmovups	1080(%rsp), %xmm0
	movq	%rax, 552(%rsp)
	movq	1096(%rsp), %rax
	leaq	64(%rsp), %rdi
	leaq	528(%rsp), %rsi
	leaq	224(%rsp), %rdx
	movq	$0, 528(%rsp)
	movq	$8, 536(%rsp)
	movq	$0, 224(%rsp)
	movq	$1, 232(%rsp)
	movq	$0, 240(%rsp)
	movq	$0, 544(%rsp)
	movq	%rax, 264(%rsp)
	vmovups	%xmm0, 248(%rsp)
	movq	$0, 272(%rsp)
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	96(%rsp), %rcx
	vmovups	80(%rsp), %xmm0
	movq	64(%rsp), %rax
	movq	72(%rsp), %rsi
	movq	104(%rsp), %rdx
	movq	%rcx, 720(%rsp)
	movzbl	112(%rsp), %ecx
.LBB315_49:
	vmovaps	%xmm0, 704(%rsp)
	vmovups	113(%rsp), %xmm0
	movq	128(%rsp), %rdi
	vmovups	136(%rsp), %xmm1
	vmovups	736(%rsp), %xmm2
	movq	%rax, 1248(%rsp)
	movq	720(%rsp), %rax
	movq	%rsi, 1256(%rsp)
	movq	40(%rsp), %r8
	movq	%rax, 1280(%rsp)
	vmovaps	%xmm0, 640(%rsp)
	movq	%rdi, 655(%rsp)
	movq	152(%rsp), %rdi
	vmovaps	%xmm1, 672(%rsp)
	vmovaps	704(%rsp), %xmm1
	vmovaps	%xmm2, (%rbx)
	vmovaps	640(%rsp), %xmm2
	movq	655(%rsp), %rax
	movq	%rdi, 688(%rsp)
	movq	752(%rsp), %rdi
	vmovups	%xmm1, 1264(%rsp)
	vmovaps	672(%rsp), %xmm1
	movq	%rdx, 1288(%rsp)
	movb	%cl, 1296(%rsp)
	vmovups	%xmm2, 1297(%rsp)
	movq	%rax, 1312(%rsp)
	movq	688(%rsp), %rax
	movq	%rdi, 16(%rbx)
	movl	760(%rsp), %edi
	movq	%rax, 1336(%rsp)
	movl	%edi, 1228(%r8)
	movzbl	764(%rsp), %edi
	vmovups	%xmm1, 1320(%rsp)
	movb	%dil, 1238(%r8)
.Ltmp11462:
	leaq	1216(%rsp), %rdi
	leaq	888(%rsp), %rsi
	leaq	1248(%rsp), %rcx
	movl	$1, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp11463:
	cmpq	$-1, 1216(%rsp)
	je	.LBB315_58
	vmovups	888(%rsp), %zmm0
	vmovups	928(%rsp), %zmm1
	vmovups	%zmm0, 64(%rsp)
	vmovups	%zmm1, 104(%rsp)
	cmpq	$-1, 64(%rsp)
	je	.LBB315_63
	leaq	224(%rsp), %rdi
	leaq	1216(%rsp), %rsi
	leaq	888(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	jmp	.LBB315_64
.LBB315_54:
	addq	$8, %r13
.Ltmp11459:
	leaq	224(%rsp), %rdi
	leaq	480(%rsp), %rdx
	movq	%r13, %rsi
	movq	%r14, %rcx
	movq	%r15, %r8
	movq	%r12, %r9
	callq	purrdf_sparql_eval::property_fn_eval::eval_call_over::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp11460:
	vmovups	232(%rsp), %ymm0
	vmovups	265(%rsp), %xmm1
	movq	224(%rsp), %rcx
	movzbl	264(%rsp), %eax
	movq	280(%rsp), %rdx
	vmovaps	%xmm1, 800(%rsp)
	vmovups	%ymm0, 528(%rsp)
	movq	%rdx, 815(%rsp)
	cmpq	$-1, %rcx
	je	.LBB315_119
	vmovaps	288(%rsp), %xmm0
	movq	544(%rsp), %rsi
	movq	752(%rsp), %r11
	movq	815(%rsp), %rdi
	movl	760(%rsp), %r10d
	movq	40(%rsp), %r8
	movzbl	764(%rsp), %r9d
	movq	304(%rsp), %rdx
	movq	%rsi, 720(%rsp)
	movq	552(%rsp), %rsi
	movq	%r11, 16(%rbx)
	movq	%rdx, 688(%rsp)
	movq	312(%rsp), %rdx
	vmovaps	%xmm0, 672(%rsp)
	vmovaps	528(%rsp), %xmm0
	vmovaps	672(%rsp), %xmm1
	vmovaps	%xmm0, 704(%rsp)
	vmovaps	800(%rsp), %xmm0
	vmovaps	%xmm0, 640(%rsp)
	vmovups	736(%rsp), %xmm0
	movq	%rdi, 655(%rsp)
	movq	720(%rsp), %rdi
	vmovaps	640(%rsp), %xmm2
	vmovaps	%xmm0, (%rbx)
	vmovaps	704(%rsp), %xmm0
	movl	%r10d, 1228(%r8)
	movb	%r9b, 1238(%r8)
	movq	56(%rsp), %r9
	movq	%rdi, 40(%r9)
	vmovups	%xmm0, 24(%r9)
	movq	%rsi, 48(%r9)
	movq	655(%rsp), %rsi
	vmovups	%xmm2, 57(%r9)
	movq	%rsi, 72(%r9)
	movq	688(%rsp), %rsi
	vmovaps	%xmm1, 80(%r9)
	movq	%rsi, 96(%r9)
	movq	%rcx, 16(%r9)
	movb	%al, 56(%r9)
	movq	%rdx, 104(%r9)
	movq	$1, (%r9)
	jmp	.LBB315_57
.LBB315_38:
	movq	$0, 432(%rsp)
.LBB315_39:
	movb	$1, %bpl
.Ltmp11264:
	movq	purrdf_sparql_eval::deferred_exists::is_lateral_placeholder@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp11265:
	testb	%al, %al
	je	.LBB315_59
.Ltmp11448:
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.443(%rip), %rsi
	leaq	64(%rsp), %rdi
	movl	$84, %edx
	callq	<purrdf_sparql_eval::error::EvalError>::internal::<&str>
.Ltmp11449:
.LBB315_42:
	vmovups	64(%rsp), %zmm0
	vmovups	96(%rsp), %zmm1
	movq	56(%rsp), %rax
	vmovups	%zmm1, 48(%rax)
	vmovups	%zmm0, 16(%rax)
	movq	$1, (%rax)
.LBB315_57:
	movb	$1, %bpl
	movq	504(%rsp), %rax
	lock		decq	(%rax)
	movl	%ebp, 48(%rsp)
	je	.LBB315_70
	jmp	.LBB315_71
.LBB315_58:
.Ltmp11467:
	leaq	224(%rsp), %rdi
	leaq	888(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
.Ltmp11468:
	jmp	.LBB315_69
.LBB315_59:
	movq	504(%rsp), %r15
	lock		incq	(%r15)
	jle	.LBB315_347
	vmovups	anon.e5162873a9a3251d11c4df37a70e4654.29.llvm.12908414067662811932(%rip), %ymm0
	movq	496(%rsp), %rbx
	movabsq	$128102389400760776, %rax
	movq	$0, 800(%rsp)
	movq	$8, 808(%rsp)
	movq	$0, 816(%rsp)
	movq	%r15, 520(%rsp)
	decq	%rax
	vmovups	%ymm0, 824(%rsp)
	cmpq	%rax, %rbx
	jbe	.LBB315_121
	xorl	%r13d, %r13d
.LBB315_62:
	movb	$1, %bpl
.Ltmp11441:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r14, %rsi
	vzeroupper
	callq	*%rax
.Ltmp11442:
	jmp	.LBB315_347
.LBB315_63:
	vmovups	1216(%rsp), %ymm0
	vmovups	%ymm0, 232(%rsp)
	movq	$-1, 224(%rsp)
.LBB315_64:
	movq	136(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB315_66
	movq	144(%rsp), %rdi
	leaq	-3(%rax,%rax,2), %rsi
	movl	$1, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB315_66:
	movq	160(%rsp), %rax
	testq	%rax, %rax
	je	.LBB315_69
	lock		decq	(%rax)
	jne	.LBB315_69
	leaq	160(%rsp), %rdi
	#MEMBARRIER
.Ltmp11465:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp11466:
.LBB315_69:
	vmovups	224(%rsp), %zmm0
	vmovups	256(%rsp), %zmm1
	movq	56(%rsp), %rax
	xorl	%ebp, %ebp
	vmovups	%zmm1, 40(%rax)
	vmovups	%zmm0, 8(%rax)
	movq	$0, (%rax)
	movq	504(%rsp), %rax
	lock		decq	(%rax)
	movl	%ebp, 48(%rsp)
	jne	.LBB315_71
.LBB315_70:
	leaq	504(%rsp), %rdi
	#MEMBARRIER
.Ltmp11472:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp11473:
.LBB315_71:
	movq	488(%rsp), %r14
	movq	496(%rsp), %rbx
	testq	%rbx, %rbx
	je	.LBB315_84
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	movabsq	$9223372036854775807, %r12
	xorl	%r15d, %r15d
	jmp	.LBB315_76
	.p2align	4
.LBB315_73:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB315_74:
	vzeroupper
	callq	*%r13
.LBB315_75:
	incq	%r15
	cmpq	%rbx, %r15
	je	.LBB315_84
.LBB315_76:
	leaq	(%r15,%r15,4), %rcx
	movq	(%r14,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB315_75
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
	jge	.LBB315_79
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB315_79:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB315_74
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB315_79
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
.LBB315_82:
	cmpq	%rax, %rdx
	jge	.LBB315_73
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB315_82
	jmp	.LBB315_73
.LBB315_84:
	movq	480(%rsp), %rax
	movl	48(%rsp), %ebx
	testq	%rax, %rax
	je	.LBB315_94
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
	jge	.LBB315_87
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB315_87:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB315_93
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB315_87
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
.LBB315_90:
	cmpq	%rax, %rsi
	jge	.LBB315_92
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB315_90
.LBB315_92:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB315_93:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.LBB315_94:
	testb	%bl, %bl
	je	.LBB315_118
.LBB315_95:
	movq	960(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB315_105
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
	jge	.LBB315_98
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB315_98:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB315_104
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB315_98
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
.LBB315_101:
	cmpq	%rax, %rsi
	jge	.LBB315_103
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB315_101
.LBB315_103:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB315_104:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB315_105:
	movq	888(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB315_115
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
	jge	.LBB315_108
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB315_108:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB315_114
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB315_108
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
.LBB315_111:
	cmpq	%rax, %rsi
	jge	.LBB315_113
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB315_111
.LBB315_113:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB315_114:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB315_115:
	movq	984(%rsp), %rax
	testq	%rax, %rax
	je	.LBB315_118
	lock		decq	(%rax)
	jne	.LBB315_118
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	984(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB315_118:
	movq	56(%rsp), %rax
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
.LBB315_119:
	.cfi_def_cfa_offset 1520
	cmpb	$-1, %al
	je	.LBB315_240
	vmovaps	800(%rsp), %xmm0
	movq	815(%rsp), %rcx
	leaq	64(%rsp), %rdi
	leaq	528(%rsp), %rsi
	leaq	224(%rsp), %rdx
	movq	%rcx, 264(%rsp)
	vmovups	%xmm0, 249(%rsp)
	movq	$0, 224(%rsp)
	movq	$1, 232(%rsp)
	movq	$0, 240(%rsp)
	movb	%al, 248(%rsp)
	movq	$0, 272(%rsp)
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	64(%rsp), %rax
	movq	104(%rsp), %rdx
	movzbl	112(%rsp), %ecx
	jmp	.LBB315_241
.LBB315_121:
	movq	%r15, 16(%rsp)
	testq	%rbx, %rbx
	je	.LBB315_242
	leaq	(,%rbx,8), %rax
	movl	$8, %esi
	movl	$8, %r13d
	leaq	(%rax,%rax,8), %r14
	movq	%r14, %rdi
	vzeroupper
	callq	__rustc::__rust_alloc
	testq	%rax, %rax
	je	.LBB315_62
	movq	488(%rsp), %rcx
	movq	%rbx, 456(%rsp)
	movq	%rax, 464(%rsp)
	movq	%rax, 208(%rsp)
	leaq	(%rbx,%rbx,4), %rax
	movq	40(%rsp), %rbx
	movq	$0, 472(%rsp)
	movq	$0, 32(%rsp)
	leaq	(%rcx,%rax,8), %rax
	movq	%rcx, 48(%rsp)
	leaq	1096(%rbx), %rdx
	leaq	16(%r15), %rcx
	movq	%rdx, 416(%rsp)
	movq	%rcx, 512(%rsp)
	movq	%rax, 344(%rsp)
	jmp	.LBB315_125
.LBB315_124:
	movq	32(%rsp), %rsi
	movq	128(%rsp), %rcx
	movq	208(%rsp), %rdx
	movq	48(%rsp), %rdi
	leaq	(%rsi,%rsi,8), %rax
	addq	$40, %rdi
	incq	%rsi
	movq	%rsi, 32(%rsp)
	movq	%rdi, 48(%rsp)
	movq	%rcx, 64(%rdx,%rax,8)
	vmovups	64(%rsp), %zmm0
	vmovups	%zmm0, (%rdx,%rax,8)
	movq	%rsi, 472(%rsp)
	cmpq	344(%rsp), %rdi
	je	.LBB315_246
.LBB315_125:
	movq	432(%rsp), %r12
	testq	%r12, %r12
	je	.LBB315_128
	movq	48(%rsp), %rax
	movq	(%rax), %r14
	decq	%r14
	cmpq	$4, %r14
	jbe	.LBB315_130
	movq	16(%rax), %r14
	movq	8(%rax), %r13
	decq	%r14
	jmp	.LBB315_131
.LBB315_128:
	movzbl	1237(%rbx), %eax
	incq	%rax
	movq	%rax, 528(%rsp)
	movq	48(%rsp), %rax
	movq	$0, 544(%rsp)
	movq	(%rax), %rcx
	decq	%rcx
	cmpq	$4, %rcx
	jbe	.LBB315_140
	movq	16(%rax), %rcx
	movq	8(%rax), %rdx
	decq	%rcx
	jmp	.LBB315_141
.LBB315_130:
	leaq	8(%rax), %r13
.LBB315_131:
	leaq	368(%rsp), %rax
	movb	$0, 368(%rsp)
	movq	%rax, 64(%rsp)
	leaq	64(%rsp), %rax
	#APP
	#NO_APP
	movq	purrdf_stack::FLOOR::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rax
	movq	64(%rsp), %rdi
	movq	%fs:(%rax), %rcx
	movq	%rdi, %rax
	subq	%rcx, %rax
	cmpq	$131072, %rax
	setb	%al
	cmpq	%rcx, %rdi
	jb	.LBB315_236
	testb	%al, %al
	jne	.LBB315_237
.LBB315_133:
	movq	16(%rsp), %rax
	movq	24(%rax), %rcx
	movq	32(%rax), %r8
.Ltmp11268:
	leaq	64(%rsp), %rdi
	movq	%r13, %rsi
	movq	%r14, %rdx
	movq	%rbx, %r9
	vzeroupper
	callq	purrdf_sparql_eval::expr::outer_bindings_for_substitution::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp11269:
	leaq	72(%rsp), %rdx
	movq	64(%rsp), %rax
	vmovups	(%rdx), %ymm0
	vmovups	16(%rdx), %ymm1
	vmovups	%ymm0, 368(%rsp)
	vmovups	%ymm1, 384(%rsp)
	cmpq	$-1, %rax
	je	.LBB315_137
	vmovups	48(%rdx), %ymm0
	vmovups	368(%rsp), %ymm2
	vmovups	384(%rsp), %ymm1
	movq	80(%rdx), %rcx
	leaq	240(%rsp), %rsi
	movq	%rcx, 88(%rsi)
	vmovups	%ymm0, 56(%rsi)
	vmovups	%ymm1, 24(%rsi)
	vmovups	%ymm2, 8(%rsi)
	movq	%rax, 240(%rsp)
.LBB315_136:
	movq	$1, 224(%rsp)
	jmp	.LBB315_199
.LBB315_137:
	vmovups	368(%rsp), %ymm0
	vmovups	384(%rsp), %ymm1
	movq	448(%rsp), %rsi
	movq	440(%rsp), %rdx
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
	je	.LBB315_144
	cmpq	$1, %rax
	jne	.LBB315_151
	addq	$16, %rsi
	leaq	48(%r12), %rcx
.Ltmp11270:
	movq	purrdf_sparql_eval::deferred_exists::with_row@GOTPCREL(%rip), %rax
	leaq	1104(%rsp), %rdi
	leaq	1024(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp11271:
	jmp	.LBB315_146
.LBB315_140:
	leaq	8(%rax), %rdx
.LBB315_141:
.Ltmp11357:
	movq	216(%rsp), %rsi
	movq	512(%rsp), %r8
	leaq	64(%rsp), %rdi
	leaq	528(%rsp), %r9
	movq	%rbx, (%rsp)
	vzeroupper
	callq	purrdf_sparql_eval::binop::eval_correlated::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp11358:
	cmpl	$1, 64(%rsp)
	je	.LBB315_328
	leaq	72(%rsp), %rcx
	movq	72(%rsp), %rax
	movq	16(%rsp), %r15
	vmovups	32(%rcx), %zmm1
	vmovups	8(%rcx), %zmm0
	leaq	240(%rsp), %rcx
	vmovups	%zmm1, 248(%rsp)
	vmovups	%zmm0, 224(%rsp)
	vmovups	224(%rsp), %ymm0
	vmovups	40(%rcx), %ymm1
	vmovups	16(%rcx), %ymm2
	jmp	.LBB315_201
.LBB315_144:
	leaq	72(%rsp), %rax
	movq	$0, 64(%rsp)
	movq	$8, 72(%rsp)
	vxorps	%xmm0, %xmm0, %xmm0
	leaq	48(%r12), %rcx
	vmovups	%xmm0, 8(%rax)
	movq	$8, 96(%rsp)
	movq	$0, 104(%rsp)
.Ltmp11272:
	movq	purrdf_sparql_eval::deferred_exists::with_row@GOTPCREL(%rip), %rax
	leaq	64(%rsp), %r14
	leaq	1104(%rsp), %rdi
	leaq	1024(%rsp), %rdx
	movq	%r14, %rsi
	vzeroupper
	callq	*%rax
.Ltmp11273:
.Ltmp11277:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp11278:
.LBB315_146:
	cmpq	$-1, 1104(%rsp)
	je	.LBB315_152
	vmovups	1120(%rsp), %ymm1
	vmovups	1104(%rsp), %ymm0
	cmpq	$0, 632(%rbx)
	vmovups	%ymm1, 384(%rsp)
	vmovups	%ymm0, 368(%rsp)
	je	.LBB315_177
	movq	80(%r12), %rax
	cmpq	$0, 40(%rax)
	je	.LBB315_177
	lock		incq	(%rax)
	jle	.LBB315_347
	movq	80(%r12), %rsi
	movb	$1, %bl
.Ltmp11279:
	movq	416(%rsp), %rdi
	vzeroupper
	callq	<alloc::vec::Vec<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::expr::SubstitutionSource, purrdf_hash::fixed::FixedState>>>>::push_mut
.Ltmp11280:
	jmp	.LBB315_178
.LBB315_151:
	movq	$-1, 1104(%rsp)
.LBB315_152:
	movq	40(%r12), %rdi
.Ltmp11288:
	leaq	864(%rsp), %rsi
	movq	%rbx, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::deferred_exists::nested_sites::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp11289:
	movq	%rax, 624(%rsp)
	movq	%rdx, 184(%rsp)
	movq	%rdx, 632(%rsp)
	leaq	48(%r12), %rdx
	movq	%rax, 192(%rsp)
	movq	$0, 352(%rsp)
	movq	$0, 360(%rsp)
.Ltmp11290:
	movq	<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>::then@GOTPCREL(%rip), %rax
	leaq	440(%rsp), %rdi
	leaq	1024(%rsp), %rsi
	xorl	%ebx, %ebx
	callq	*%rax
.Ltmp11291:
	movq	%rax, 368(%rsp)
	movq	%rdx, 376(%rsp)
.Ltmp11292:
	movq	<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>::layers@GOTPCREL(%rip), %rax
	leaq	64(%rsp), %rdi
	leaq	368(%rsp), %rsi
	callq	*%rax
.Ltmp11293:
	movq	64(%rsp), %rax
	movq	%r12, 200(%rsp)
	movq	72(%rsp), %r12
	movq	%rax, 176(%rsp)
	movq	80(%rsp), %rax
	movq	%r12, 24(%rsp)
	testq	%rax, %rax
	je	.LBB315_185
	movq	192(%rsp), %r15
	leaq	(%r12,%rax,8), %rax
	xorl	%ebp, %ebp
	xorl	%r14d, %r14d
	movq	%rax, 424(%rsp)
	shlq	$4, %r15
	addq	184(%rsp), %r15
	jmp	.LBB315_160
.LBB315_157:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB315_158:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbp, %rdi
	callq	*%rax
.LBB315_159:
	addq	$8, %r12
	movq	%r13, 352(%rsp)
	movq	%r13, %rbp
	movq	%rbx, %r14
	cmpq	424(%rsp), %r12
	je	.LBB315_186
.LBB315_160:
	movq	%rbp, %rsi
	testq	%rbp, %rbp
	jne	.LBB315_162
	movq	200(%rsp), %rax
	movq	40(%rax), %rsi
.LBB315_162:
	vmovups	anon.e5162873a9a3251d11c4df37a70e4654.29.llvm.12908414067662811932(%rip), %ymm0
	movq	(%r12), %rdx
	leaq	16(%r14), %rax
	testq	%r14, %r14
	cmoveq	%r14, %rax
	movq	%rax, 1184(%rsp)
	movq	%r15, 1192(%rsp)
	movq	$0, 1200(%rsp)
	vmovups	%ymm0, 1152(%rsp)
.Ltmp11295:
	leaq	64(%rsp), %rdi
	leaq	1152(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::expr::substitute_pattern_deferring::<false>
.Ltmp11296:
	movq	64(%rsp), %rax
	movq	72(%rsp), %r13
	cmpq	$-1, %rax
	jne	.LBB315_225
.Ltmp11314:
	movq	<purrdf_sparql_eval::expr::Deferral>::into_placeholders@GOTPCREL(%rip), %rax
	leaq	1152(%rsp), %rdi
	callq	*%rax
.Ltmp11315:
	movq	%rax, %rbx
	testq	%r14, %r14
	je	.LBB315_168
	lock		decq	(%r14)
	jne	.LBB315_168
	#MEMBARRIER
.Ltmp11317:
	movq	<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	360(%rsp), %rdi
	callq	*%rax
.Ltmp11318:
.LBB315_168:
	movq	%rbx, 360(%rsp)
	testq	%rbp, %rbp
	je	.LBB315_159
.Ltmp11322:
	movq	%rbp, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_algebra::algebra::GraphPattern>
.Ltmp11323:
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-144, %rcx
	movabsq	$-9223372036854775808, %rdx
	addq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB315_172
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB315_172:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB315_158
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB315_172
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	$-144, %rcx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	lock		xaddq	%rcx, (%rax)
	movabsq	$-9223372036854775808, %rax
	addq	$-144, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB315_175:
	cmpq	%rax, %rcx
	jge	.LBB315_157
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB315_175
	jmp	.LBB315_157
.LBB315_177:
	xorl	%ebx, %ebx
.LBB315_178:
	movq	40(%r12), %rsi
.Ltmp11281:
	movq	40(%rsp), %r8
	leaq	64(%rsp), %rdi
	leaq	368(%rsp), %rdx
	leaq	864(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::binop::eval_substituted_delivered::<purrdf_core::ir::dataset::RdfDataset, false, false, ()>
.Ltmp11282:
	testb	%bl, %bl
	movq	40(%rsp), %rbx
	je	.LBB315_183
	movq	1112(%rbx), %rax
	testq	%rax, %rax
	je	.LBB315_183
	leaq	-1(%rax), %rcx
	movq	%rcx, 1112(%rbx)
	movq	1104(%rbx), %rcx
	movq	-8(%rcx,%rax,8), %rax
	movq	%rax, 624(%rsp)
	lock		decq	(%rax)
	jne	.LBB315_183
	movq	<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::expr::SubstitutionSource, purrdf_hash::fixed::FixedState>>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	624(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB315_183:
	vmovups	112(%rsp), %zmm1
	vmovups	64(%rsp), %zmm0
	vmovups	%zmm1, 272(%rsp)
	vmovups	%zmm0, 224(%rsp)
.Ltmp11286:
	leaq	368(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp11287:
.LBB315_184:
.Ltmp11312:
	leaq	1024(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp11313:
	jmp	.LBB315_199
.LBB315_185:
	xorl	%ebx, %ebx
	xorl	%r13d, %r13d
.LBB315_186:
	movq	176(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB315_188
	movq	24(%rsp), %rdi
	shlq	$3, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB315_188:
.Ltmp11327:
	leaq	368(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>
.Ltmp11328:
	movq	200(%rsp), %rax
	testq	%r13, %r13
	jne	.LBB315_191
	movq	40(%rax), %r13
.LBB315_191:
	movq	$0, 64(%rsp)
.Ltmp11332:
	movq	40(%rsp), %rsi
	leaq	368(%rsp), %rdi
	leaq	64(%rsp), %rdx
	movq	%rbx, %rcx
	callq	<purrdf_sparql_eval::eval::EvalCtx>::enter_substituted_exists
.Ltmp11333:
	movq	392(%rsp), %rdx
.Ltmp11334:
	leaq	224(%rsp), %rdi
	movq	%r13, %rsi
	movq	%r13, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp11335:
.Ltmp11339:
	leaq	368(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::eval::EvalScopeGuard<purrdf_core::ir::dataset::RdfDataset>>
.Ltmp11340:
.Ltmp11344:
	leaq	352(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>>
.Ltmp11345:
	movq	40(%rsp), %rbx
	cmpq	$0, 192(%rsp)
	movq	184(%rsp), %rax
	je	.LBB315_198
	lock		decq	(%rax)
	jne	.LBB315_198
	#MEMBARRIER
.Ltmp11349:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::deferred_exists::NestedSites>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	632(%rsp), %rdi
	callq	*%rax
.Ltmp11350:
.LBB315_198:
.Ltmp11355:
	leaq	1024(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp11356:
.LBB315_199:
	cmpl	$1, 224(%rsp)
	movq	16(%rsp), %r15
	je	.LBB315_327
	leaq	240(%rsp), %rcx
	movq	232(%rsp), %rax
	vmovups	24(%rcx), %zmm1
	vmovups	(%rcx), %zmm0
	leaq	560(%rsp), %rcx
	vmovups	%zmm1, 552(%rsp)
	vmovups	%zmm0, 528(%rsp)
	vmovups	528(%rsp), %ymm0
	vmovups	24(%rcx), %ymm1
	vmovups	(%rcx), %ymm2
.LBB315_201:
	vmovups	%ymm1, 760(%rsp)
	vmovups	%ymm0, 992(%rsp)
	vmovups	%ymm2, 736(%rsp)
	cmpq	$-1, %rax
	jne	.LBB315_243
	movq	1016(%rsp), %rax
	movq	32(%rax), %rbx
	testq	%rbx, %rbx
	je	.LBB315_208
	movq	24(%rax), %r14
	shlq	$4, %rbx
	addq	%r14, %rbx
	.p2align	4
.LBB315_205:
	movq	(%r14), %rax
	lock		incq	(%rax)
	jle	.LBB315_347
	movq	8(%r14), %rdx
	movq	(%r14), %rsi
.Ltmp11365:
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	800(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp11366:
	addq	$16, %r14
	cmpq	%rbx, %r14
	jne	.LBB315_205
.LBB315_208:
	movq	48(%rsp), %rax
	movq	(%rax), %rbx
	leaq	-1(%rbx), %rdx
	cmpq	$4, %rdx
	jbe	.LBB315_211
	movq	16(%rax), %rbx
	movq	8(%rax), %rsi
	leaq	-8(,%rbx,8), %rdx
	leaq	-1(%rbx), %r15
	cmpq	$5, %r15
	jae	.LBB315_213
	movq	16(%rsp), %r15
	jmp	.LBB315_212
.LBB315_211:
	leaq	8(%rax), %rsi
	shlq	$3, %rdx
.LBB315_212:
	movq	memcpy@GOTPCREL(%rip), %rax
	leaq	224(%rsp), %rdi
	vzeroupper
	callq	*%rax
	jmp	.LBB315_222
.LBB315_213:
	movq	%rsi, %r12
	movl	$4, %esi
	movq	%rdx, %rdi
	movq	%rdx, %r14
	vzeroupper
	callq	__rustc::__rust_alloc
	testq	%rax, %rax
	je	.LBB315_345
	movb	$61, %cl
	leaq	-2(%rbx), %rsi
	movq	%r12, %rdi
	bzhiq	%rcx, %r15, %rcx
	cmpq	%rsi, %rcx
	cmovbq	%rcx, %rsi
	cmpq	$16, %rsi
	jae	.LBB315_216
	movq	%r15, %rcx
	movq	%rdi, %rdx
	xorl	%esi, %esi
	jmp	.LBB315_218
.LBB315_216:
	incq	%rsi
	movl	$16, %edx
	movl	%esi, %ecx
	andl	$15, %ecx
	cmoveq	%rdx, %rcx
	xorl	%r8d, %r8d
	subq	%rcx, %rsi
	movq	%r15, %rcx
	leaq	(%rdi,%rsi,8), %rdx
	subq	%rsi, %rcx
.LBB315_217:
	vmovups	(%rdi,%r8,8), %zmm0
	vmovups	64(%rdi,%r8,8), %zmm1
	vmovups	%zmm1, 64(%rax,%r8,8)
	vmovups	%zmm0, (%rax,%r8,8)
	addq	$16, %r8
	cmpq	%r8, %rsi
	jne	.LBB315_217
.LBB315_218:
	leaq	(%rdi,%r15,8), %rdi
	leaq	4(%rax,%rsi,8), %rsi
	xorl	%r8d, %r8d
.LBB315_219:
	cmpq	%rdi, %rdx
	je	.LBB315_221
	movl	(%rdx), %r9d
	movl	4(%rdx), %r10d
	addq	$8, %rdx
	movl	%r9d, -4(%rsi,%r8,8)
	movl	%r10d, (%rsi,%r8,8)
	incq	%r8
	cmpq	%r8, %rcx
	jne	.LBB315_219
.LBB315_221:
	movq	16(%rsp), %r15
	movq	%rax, 224(%rsp)
	movq	%rbx, 232(%rsp)
.LBB315_222:
	vmovups	992(%rsp), %ymm0
	leaq	72(%rsp), %rcx
	movq	224(%rsp), %rax
	movq	232(%rsp), %rdx
	vmovups	%ymm0, 32(%rcx)
	vmovups	240(%rsp), %xmm0
	movq	%rbx, 64(%rsp)
	movq	40(%rsp), %rbx
	movq	%rax, (%rcx)
	movq	%rdx, 8(%rcx)
	vmovups	%xmm0, 16(%rcx)
	movq	32(%rsp), %rcx
	cmpq	456(%rsp), %rcx
	jne	.LBB315_124
.Ltmp11373:
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::governor::soundness::NodeAnalysis>>::grow_one@GOTPCREL(%rip), %rax
	leaq	456(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp11374:
	movq	464(%rsp), %rax
	movq	%rax, 208(%rsp)
	jmp	.LBB315_124
.LBB315_225:
	leaq	72(%rsp), %rcx
	vmovups	8(%rcx), %zmm0
	vmovups	24(%rcx), %zmm1
	leaq	240(%rsp), %rcx
	vmovups	%zmm1, 32(%rcx)
	vmovups	%zmm0, 16(%rcx)
	movq	%rax, 240(%rsp)
	movq	%r13, 248(%rsp)
	movq	$1, 224(%rsp)
.Ltmp11300:
	leaq	1152(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>
.Ltmp11301:
	movq	176(%rsp), %rsi
	movq	24(%rsp), %rdi
	testq	%rsi, %rsi
	je	.LBB315_228
	shlq	$3, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB315_228:
.Ltmp11303:
	leaq	368(%rsp), %rdi
	movq	%r14, %rbx
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>
.Ltmp11304:
	testq	%r14, %r14
	je	.LBB315_232
	lock		decq	(%r14)
	jne	.LBB315_232
	#MEMBARRIER
.Ltmp11305:
	movq	<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	360(%rsp), %rdi
	callq	*%rax
.Ltmp11306:
.LBB315_232:
.Ltmp11308:
	leaq	352(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>>
.Ltmp11309:
	movq	40(%rsp), %rbx
	cmpq	$0, 192(%rsp)
	movq	184(%rsp), %rax
	je	.LBB315_184
	lock		decq	(%rax)
	jne	.LBB315_184
	#MEMBARRIER
.Ltmp11310:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::deferred_exists::NestedSites>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	632(%rsp), %rdi
	callq	*%rax
.Ltmp11311:
	jmp	.LBB315_184
.LBB315_236:
	movb	$1, %al
	testb	%al, %al
	je	.LBB315_133
.LBB315_237:
.Ltmp11266:
	movq	purrdf_stack::is_low_cold@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp11267:
	testb	%al, %al
	je	.LBB315_133
	movabsq	$-9223372036854775784, %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.438(%rip), %rcx
	movq	%rax, 240(%rsp)
	movq	%rcx, 248(%rsp)
	movq	$41, 256(%rsp)
	jmp	.LBB315_136
.LBB315_240:
	vmovups	528(%rsp), %ymm0
	leaq	72(%rsp), %rax
	vmovups	%ymm0, (%rax)
	movq	$-1, %rax
.LBB315_241:
	vmovups	80(%rsp), %xmm0
	movq	72(%rsp), %rsi
	movq	96(%rsp), %rdi
	movq	%rdi, 720(%rsp)
	jmp	.LBB315_49
.LBB315_242:
	movq	40(%rsp), %rbx
	movl	$8, %eax
	movq	$0, 456(%rsp)
	movq	$8, 464(%rsp)
	movq	$0, 472(%rsp)
	movq	$0, 32(%rsp)
	movq	%rax, 208(%rsp)
	jmp	.LBB315_246
.LBB315_243:
	vmovups	992(%rsp), %ymm0
	vmovups	736(%rsp), %ymm2
	vmovups	760(%rsp), %ymm1
	movq	%rax, 64(%rsp)
	vmovups	%ymm0, 72(%rsp)
	vmovups	%ymm2, 104(%rsp)
	vmovups	%ymm1, 128(%rsp)
.Ltmp11360:
	leaq	224(%rsp), %rdi
	leaq	888(%rsp), %rsi
	leaq	64(%rsp), %rcx
	movl	$1, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp11361:
	cmpq	$-1, 224(%rsp)
	je	.LBB315_246
.Ltmp11362:
	leaq	224(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp11363:
.LBB315_246:
	leaq	16(%r15), %rsi
.Ltmp11387:
	movq	<purrdf_sparql_eval::solution::VarSchema>::union@GOTPCREL(%rip), %rax
	leaq	224(%rsp), %rdi
	leaq	800(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp11388:
	vmovups	224(%rsp), %ymm0
	vmovups	248(%rsp), %ymm1
	leaq	80(%rsp), %r14
	movq	$1, 64(%rsp)
	movq	$1, 72(%rsp)
	vmovups	%ymm0, 80(%rsp)
	vmovups	%ymm1, 104(%rsp)
.Ltmp11390:
	movl	$8, %edi
	movl	$72, %esi
	vzeroupper
	callq	alloc::boxed::box_new_uninit
.Ltmp11391:
	movq	128(%rsp), %rcx
	movq	%rax, 24(%rsp)
	movq	%rcx, 64(%rax)
	vmovups	64(%rsp), %zmm0
	vmovups	%zmm0, (%rax)
	movq	%rax, 1104(%rsp)
	movq	32(%r15), %rsi
	movq	32(%rax), %rdi
	testq	%rdi, %rdi
	je	.LBB315_253
	movq	616(%rbx), %rax
	testq	%rax, %rax
	je	.LBB315_253
	movq	32(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB315_253
	movq	%rax, %rdx
	orq	%rdi, %rdx
	movabsq	$230584300921369396, %rcx
	shrq	$32, %rdx
	je	.LBB315_336
	xorl	%edx, %edx
	divq	%rdi
	jmp	.LBB315_337
.LBB315_253:
	movq	32(%rsp), %rbx
	movl	$0, 200(%rsp)
.LBB315_254:
	movq	%rdi, 344(%rsp)
	movq	%rsi, 424(%rsp)
	testq	%rbx, %rbx
	je	.LBB315_257
	leaq	(,%rbx,8), %rax
	movl	$8, %esi
	leaq	(%rax,%rax,4), %r14
	movq	%r14, %rdi
	vzeroupper
	callq	__rustc::__rust_alloc
	testq	%rax, %rax
	je	.LBB315_346
	movq	%rax, %rdx
	jmp	.LBB315_258
.LBB315_257:
	movl	$8, %edx
.LBB315_258:
	movq	32(%rsp), %rax
	movq	208(%rsp), %r14
	movq	%rbx, 368(%rsp)
	movq	%rdx, 376(%rsp)
	movq	$0, 384(%rsp)
	testq	%rax, %rax
	je	.LBB315_319
	leaq	(%rax,%rax,8), %rax
	leaq	72(%rsp), %rcx
	movq	$0, 32(%rsp)
	movq	%rcx, 792(%rsp)
	movq	24(%rsp), %rcx
	leaq	(%r14,%rax,8), %rax
	movq	%rax, 176(%rsp)
	movq	424(%rsp), %rax
	addq	$16, %rcx
	movq	%rcx, 184(%rsp)
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.449(%rip), %rcx
	movq	%rcx, 416(%rsp)
	leaq	(,%rax,8), %rax
	movq	%rax, 512(%rsp)
	jmp	.LBB315_261
.LBB315_260:
	movq	%rbx, %r14
	addq	$72, %r14
	cmpq	176(%rsp), %r14
	je	.LBB315_319
.LBB315_261:
	movq	64(%r14), %rsi
	movq	%rdx, %r12
	addq	$16, %rsi
.Ltmp11399:
	movq	184(%rsp), %rdx
	movq	purrdf_sparql_eval::binop::right_to_out_map@GOTPCREL(%rip), %rax
	leaq	736(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp11400:
	movq	56(%r14), %rax
	movq	%r14, %rbx
	testq	%rax, %rax
	je	.LBB315_313
	movq	48(%r14), %r13
	leaq	8(%r14), %rcx
	movq	744(%rsp), %rbp
	movq	752(%rsp), %r14
	leaq	(%rax,%rax,4), %rax
	movq	%r12, %rdx
	movq	%rcx, 48(%rsp)
	leaq	(%r13,%rax,8), %rax
	movq	%rax, 208(%rsp)
	jmp	.LBB315_266
.LBB315_264:
	movq	32(%rsp), %rsi
	leaq	(%rsi,%rsi,4), %rax
	incq	%rsi
	movq	%rsi, 32(%rsp)
	movq	%r15, (%rdx,%rax,8)
	movq	%r12, 8(%rdx,%rax,8)
	movq	16(%rsp), %r15
	vmovaps	64(%rsp), %xmm0
	vmovups	%xmm0, 16(%rdx,%rax,8)
	movq	80(%rsp), %rcx
	movq	%rcx, 32(%rdx,%rax,8)
	movq	%rsi, 384(%rsp)
	addq	$40, %r13
	cmpq	208(%rsp), %r13
	je	.LBB315_314
.LBB315_266:
	movq	(%r13), %rcx
	leaq	8(%r13), %r12
	movq	%rdx, 216(%rsp)
	movq	%r12, %rax
	decq	%rcx
	cmpq	$5, %rcx
	jb	.LBB315_268
	movq	16(%r13), %rcx
	movq	8(%r13), %rax
	decq	%rcx
.LBB315_268:
	testq	%rcx, %rcx
	je	.LBB315_280
	leaq	(%rax,%rcx,8), %rcx
	xorl	%edx, %edx
	jmp	.LBB315_271
	.p2align	4
.LBB315_270:
	leaq	(%rax,%rdx,8), %rsi
	incq	%rdx
	addq	$8, %rsi
	cmpq	%rcx, %rsi
	je	.LBB315_280
.LBB315_271:
	cmpq	%rdx, %r14
	je	.LBB315_342
	movq	(%rbx), %r8
	movq	48(%rsp), %rsi
	decq	%r8
	cmpq	$4, %r8
	jbe	.LBB315_274
	movq	16(%rbx), %r8
	movq	8(%rbx), %rsi
	decq	%r8
.LBB315_274:
	movq	(%rbp,%rdx,8), %rdi
	cmpq	%r8, %rdi
	jae	.LBB315_270
	movl	(%rax,%rdx,8), %r8d
	cmpl	$2, %r8d
	je	.LBB315_270
	movl	(%rsi,%rdi,8), %r9d
	cmpl	$2, %r9d
	je	.LBB315_270
	cmpl	%r9d, %r8d
	jne	.LBB315_310
	movl	4(%rsi,%rdi,8), %esi
	cmpl	%esi, 4(%rax,%rdx,8)
	je	.LBB315_270
.LBB315_310:
	movq	216(%rsp), %rdx
	addq	$40, %r13
	cmpq	208(%rsp), %r13
	jne	.LBB315_266
	jmp	.LBB315_314
.LBB315_280:
	cmpb	$0, 200(%rsp)
	je	.LBB315_282
	movq	32(%rsp), %rax
	cmpq	192(%rsp), %rax
	jae	.LBB315_316
.LBB315_282:
	movq	344(%rsp), %rdx
	movq	$1, 64(%rsp)
	cmpq	$5, %rdx
	jae	.LBB315_311
	vmovups	72(%rsp), %xmm0
	movq	96(%rsp), %rax
	movq	64(%rsp), %rsi
	movq	88(%rsp), %rcx
	movq	%rax, 256(%rsp)
	movq	%rsi, 224(%rsp)
	movq	%rcx, 248(%rsp)
	vmovups	%xmm0, 232(%rsp)
	testq	%rdx, %rdx
	je	.LBB315_285
.LBB315_284:
	movl	$2, %eax
	jmp	.LBB315_286
.LBB315_285:
	movl	$-1, %eax
.LBB315_286:
	movl	%eax, 64(%rsp)
	movq	%rdx, 72(%rsp)
.Ltmp11410:
	leaq	224(%rsp), %rdi
	leaq	64(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp11411:
	vmovups	224(%rsp), %ymm0
	movq	256(%rsp), %rax
	movq	424(%rsp), %r8
	leaq	536(%rsp), %rdi
	movq	%rax, 560(%rsp)
	vmovups	%ymm0, 528(%rsp)
	movq	528(%rsp), %r15
	movq	%r15, %rdx
	cmpq	$6, %r15
	jb	.LBB315_289
	movq	536(%rsp), %rdi
	movq	544(%rsp), %rdx
.LBB315_289:
	decq	%rdx
	cmpq	%rdx, %r8
	ja	.LBB315_340
	movq	(%rbx), %rax
	movq	16(%rbx), %rsi
	decq	%rax
	decq	%rsi
	cmpq	$5, %rax
	cmovbq	%rax, %rsi
	cmpq	%rsi, %r8
	jne	.LBB315_341
	movq	48(%rsp), %rsi
	cmpq	$5, %rax
	jb	.LBB315_293
	movq	48(%rsp), %rax
	movq	(%rax), %rsi
.LBB315_293:
	movq	512(%rsp), %rdx
	movq	memcpy@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	(%r13), %rax
	decq	%rax
	cmpq	$5, %rax
	jb	.LBB315_295
	movq	16(%r13), %rax
	movq	8(%r13), %r12
	decq	%rax
.LBB315_295:
	testq	%rax, %rax
	je	.LBB315_307
	shlq	$3, %rax
	leaq	536(%rsp), %r9
	xorl	%edi, %edi
	jmp	.LBB315_299
	.p2align	4
.LBB315_297:
	movl	4(%r12,%rdi,8), %esi
	movl	%ecx, (%r8,%rdx,8)
	movl	%esi, 4(%r8,%rdx,8)
.LBB315_298:
	incq	%rdi
	addq	$-8, %rax
	je	.LBB315_306
.LBB315_299:
	movl	(%r12,%rdi,8), %ecx
	cmpl	$2, %ecx
	je	.LBB315_298
	cmpq	%r14, %rdi
	jae	.LBB315_344
	movq	528(%rsp), %rsi
	movq	%rsi, %r8
	cmpq	$6, %rsi
	jb	.LBB315_303
	movq	544(%rsp), %r8
.LBB315_303:
	movq	(%rbp,%rdi,8), %rdx
	decq	%r8
	cmpq	%r8, %rdx
	jae	.LBB315_343
	movq	%r9, %r8
	cmpq	$6, %rsi
	jb	.LBB315_297
	movq	536(%rsp), %r8
	jmp	.LBB315_297
.LBB315_306:
	movq	528(%rsp), %r15
.LBB315_307:
	leaq	536(%rsp), %rax
	movq	536(%rsp), %r12
	movq	216(%rsp), %rdx
	movq	32(%rsp), %rcx
	vmovups	8(%rax), %xmm0
	movq	24(%rax), %rax
	movq	%rax, 80(%rsp)
	vmovaps	%xmm0, 64(%rsp)
	cmpq	368(%rsp), %rcx
	jne	.LBB315_264
.Ltmp11417:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	368(%rsp), %rdi
	callq	*%rax
.Ltmp11418:
	movq	376(%rsp), %rdx
	jmp	.LBB315_264
.LBB315_311:
.Ltmp11407:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	64(%rsp), %rdi
	xorl	%esi, %esi
	xorl	%ecx, %ecx
	callq	*%rax
.Ltmp11408:
	vmovups	64(%rsp), %ymm0
	movq	96(%rsp), %rax
	movq	344(%rsp), %rdx
	movq	%rax, 256(%rsp)
	vmovups	%ymm0, 224(%rsp)
	jmp	.LBB315_284
.LBB315_313:
	movq	%r12, %rdx
.LBB315_314:
	movq	736(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB315_260
	movq	744(%rsp), %rdi
	movq	%rdx, %r14
	shlq	$3, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
	movq	%r14, %rdx
	jmp	.LBB315_260
.LBB315_316:
	movq	32(%rsp), %rdx
	incq	%rdx
.Ltmp11404:
	movq	40(%rsp), %rsi
	movq	344(%rsp), %rcx
	leaq	64(%rsp), %rdi
	callq	<purrdf_sparql_eval::eval::EvalCtx>::observe_cells
.Ltmp11405:
	movq	736(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB315_319
	shlq	$3, %rsi
	movl	$8, %edx
	movq	%rbp, %rdi
	callq	__rustc::__rust_dealloc
.LBB315_319:
	movq	368(%rsp), %rcx
	movq	384(%rsp), %rax
	movq	376(%rsp), %r8
	movq	%rcx, 224(%rsp)
	movq	24(%rsp), %rcx
	movq	%rax, 240(%rsp)
	movq	%r8, 232(%rsp)
	movq	%rcx, 248(%rsp)
.Ltmp11425:
	leaq	64(%rsp), %rdi
	leaq	888(%rsp), %rsi
	leaq	224(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::finish::<purrdf_core::ir::term::TermId>
.Ltmp11426:
	vmovups	64(%rsp), %zmm0
	vmovups	96(%rsp), %zmm1
	movq	56(%rsp), %rax
	xorl	%ebp, %ebp
	vmovups	%zmm1, 40(%rax)
	vmovups	%zmm0, 8(%rax)
	movq	$0, (%rax)
.Ltmp11430:
	leaq	456(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>>
.Ltmp11431:
	xorl	%ebp, %ebp
.Ltmp11432:
	leaq	800(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp11433:
	lock		decq	(%r15)
	jne	.LBB315_324
	xorl	%ebp, %ebp
	#MEMBARRIER
.Ltmp11435:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	520(%rsp), %rdi
	callq	*%rax
.Ltmp11436:
.LBB315_324:
	cmpq	$0, 432(%rsp)
	je	.LBB315_326
	xorl	%ebp, %ebp
.Ltmp11437:
	leaq	432(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::DeferredLateral>
.Ltmp11438:
.LBB315_326:
	xorl	%ebp, %ebp
.Ltmp11439:
	leaq	480(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp11440:
	jmp	.LBB315_118
.LBB315_327:
	leaq	240(%rsp), %rax
	vmovups	32(%rax), %zmm1
	vmovups	(%rax), %zmm0
	movq	56(%rsp), %rax
	vmovups	%zmm1, 560(%rsp)
	vmovups	%zmm0, 528(%rsp)
	vmovups	560(%rsp), %zmm1
	vmovups	528(%rsp), %zmm0
	jmp	.LBB315_329
.LBB315_328:
	leaq	72(%rsp), %rax
	movq	16(%rsp), %r15
	vmovups	40(%rax), %zmm1
	vmovups	8(%rax), %zmm0
	movq	56(%rsp), %rax
	vmovups	%zmm1, 256(%rsp)
	vmovups	%zmm0, 224(%rsp)
	vmovups	256(%rsp), %zmm1
	vmovups	224(%rsp), %zmm0
.LBB315_329:
	vmovups	%zmm1, 48(%rax)
	vmovups	%zmm0, 16(%rax)
	movq	$1, (%rax)
	movb	$1, %bpl
.Ltmp11379:
	leaq	456(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>>
.Ltmp11380:
	movb	$1, %bpl
.Ltmp11381:
	leaq	800(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp11382:
	lock		decq	(%r15)
	jne	.LBB315_334
	movb	$1, %bpl
	#MEMBARRIER
.Ltmp11383:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	520(%rsp), %rdi
	callq	*%rax
.Ltmp11384:
.LBB315_334:
	cmpq	$0, 432(%rsp)
	je	.LBB315_57
	movb	$1, %bpl
.Ltmp11385:
	leaq	432(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::DeferredLateral>
.Ltmp11386:
	jmp	.LBB315_57
.LBB315_336:
	xorl	%edx, %edx
	divl	%edi
.LBB315_337:
	movq	32(%rsp), %rbx
	movq	%rax, 192(%rsp)
	cmpq	%rcx, %rax
	jae	.LBB315_339
	cmpq	%rbx, %rax
	cmovbq	%rax, %rbx
	movb	$1, %al
	movl	%eax, 200(%rsp)
	jmp	.LBB315_254
.LBB315_339:
	movl	$0, 200(%rsp)
	jmp	.LBB315_254
.LBB315_340:
.Ltmp11420:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.451(%rip), %rcx
	xorl	%edi, %edi
	movq	%r8, %rsi
	vzeroupper
	callq	*%rax
.Ltmp11421:
	jmp	.LBB315_347
.LBB315_341:
.Ltmp11413:
	movq	core::slice::copy_from_slice_impl::len_mismatch_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.448(%rip), %rdx
	movq	%r8, %rdi
	vzeroupper
	callq	*%rax
.Ltmp11414:
	jmp	.LBB315_347
.LBB315_342:
.Ltmp11402:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.710(%rip), %rdx
	movq	%r14, %rdi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp11403:
	jmp	.LBB315_347
.LBB315_343:
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.450(%rip), %rax
	movq	%rdx, %rdi
	movq	%r8, %r14
	movq	%rax, 416(%rsp)
.LBB315_344:
.Ltmp11415:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	movq	416(%rsp), %rdx
	movq	%r14, %rsi
	callq	*%rax
.Ltmp11416:
	jmp	.LBB315_347
.LBB315_345:
.Ltmp11368:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$4, %edi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp11369:
	jmp	.LBB315_347
.LBB315_346:
.Ltmp11396:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp11397:
.LBB315_347:
	ud2
.LBB315_348:
.Ltmp11409:
	movq	%rax, %r14
	movq	64(%rsp), %rax
	movq	24(%rsp), %rbx
	cmpq	$6, %rax
	jae	.LBB315_404
	jmp	.LBB315_414
.LBB315_349:
.Ltmp11307:
	jmp	.LBB315_366
.LBB315_350:
.Ltmp11302:
	movq	%rax, %r15
	jmp	.LBB315_384
.LBB315_351:
.Ltmp11364:
	jmp	.LBB315_360
.LBB315_352:
.Ltmp11398:
	movq	24(%rsp), %rbx
	movq	%rax, %r14
	jmp	.LBB315_417
.LBB315_353:
.Ltmp11274:
	movq	%rax, %r14
.Ltmp11275:
	leaq	64(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp11276:
	movq	16(%rsp), %r15
	jmp	.LBB315_397
.LBB315_354:
.Ltmp11336:
	movq	%rax, %rbx
.Ltmp11337:
	leaq	368(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::eval::EvalScopeGuard<purrdf_core::ir::dataset::RdfDataset>>
.Ltmp11338:
	jmp	.LBB315_392
.LBB315_355:
.Ltmp11427:
	movq	%rax, %r14
	xorl	%ebp, %ebp
	jmp	.LBB315_419
.LBB315_356:
.Ltmp11392:
	movq	%rax, %rbx
.Ltmp11393:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp11394:
	movq	16(%rsp), %r15
	movb	$1, %bpl
	movq	%rbx, %r14
	jmp	.LBB315_419
.LBB315_358:
.Ltmp11395:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB315_359:
.Ltmp11389:
.LBB315_360:
	movq	%rax, %r14
	movb	$1, %bpl
	jmp	.LBB315_419
.LBB315_361:
.Ltmp11283:
	movq	%rax, %r14
.Ltmp11284:
	leaq	368(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp11285:
	movq	16(%rsp), %r15
	jmp	.LBB315_397
.LBB315_362:
.Ltmp11346:
	movq	%rax, %rbx
	jmp	.LBB315_393
.LBB315_363:
.Ltmp11294:
	movq	%rax, %r15
	xorl	%r14d, %r14d
	jmp	.LBB315_386
.LBB315_364:
.Ltmp11434:
	movq	%rax, %r14
	jmp	.LBB315_422
.LBB315_365:
.Ltmp11341:
.LBB315_366:
	movq	%rax, %rbx
	jmp	.LBB315_392
.LBB315_367:
.Ltmp11370:
	jmp	.LBB315_407
.LBB315_368:
.Ltmp11329:
	movq	%rax, %r15
	movq	%rbx, %r14
	jmp	.LBB315_387
.LBB315_369:
.Ltmp11401:
	movq	24(%rsp), %rbx
	movq	%rax, %r14
	jmp	.LBB315_416
.LBB315_370:
.Ltmp11319:
	movq	%rax, %r15
	movq	%rbx, 360(%rsp)
	movq	%rbx, %r14
	jmp	.LBB315_382
.LBB315_371:
.Ltmp11351:
	movq	16(%rsp), %r15
	movq	%rax, %r14
	jmp	.LBB315_397
.LBB315_372:
.Ltmp11375:
	movq	%rax, %r14
.Ltmp11376:
	leaq	64(%rsp), %rdi
	callq	core::ptr::drop_glue::<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>
.Ltmp11377:
	jmp	.LBB315_379
.LBB315_373:
.Ltmp11378:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB315_374:
.Ltmp11469:
	movq	%rax, %r14
	xorl	%ebp, %ebp
	jmp	.LBB315_426
.LBB315_375:
.Ltmp11450:
	movq	%rax, %r14
	jmp	.LBB315_424
.LBB315_376:
.Ltmp11324:
	movl	$144, %esi
	movl	$8, %edx
	movq	%rbp, %rdi
	movq	%rax, %r15
	callq	__rustc::__rust_dealloc
	movq	%r13, 352(%rsp)
	movq	%rbx, %r14
	jmp	.LBB315_384
.LBB315_377:
.Ltmp11464:
	movq	%rax, %r14
	movb	$1, %bpl
	jmp	.LBB315_426
.LBB315_378:
.Ltmp11359:
	movq	%rax, %r14
.LBB315_379:
	movq	16(%rsp), %r15
	movb	$1, %bpl
	jmp	.LBB315_419
.LBB315_380:
.Ltmp11474:
	movq	%rax, %r14
	jmp	.LBB315_428
.LBB315_381:
.Ltmp11316:
	movq	%rax, %r15
.LBB315_382:
.Ltmp11320:
	movq	%r13, %rdi
	callq	core::ptr::drop_glue::<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>
.Ltmp11321:
	jmp	.LBB315_384
.LBB315_383:
.Ltmp11297:
	movq	%rax, %r15
.Ltmp11298:
	leaq	1152(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>
.Ltmp11299:
.LBB315_384:
	cmpq	$0, 176(%rsp)
	je	.LBB315_386
	movq	176(%rsp), %rsi
	movq	24(%rsp), %rdi
	movl	$8, %edx
	shlq	$3, %rsi
	callq	__rustc::__rust_dealloc
.LBB315_386:
.Ltmp11325:
	leaq	368(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>
.Ltmp11326:
.LBB315_387:
	testq	%r14, %r14
	je	.LBB315_391
	lock		decq	(%r14)
	jne	.LBB315_391
	#MEMBARRIER
.Ltmp11330:
	movq	<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	360(%rsp), %rdi
	callq	*%rax
.Ltmp11331:
	movq	%r15, %rbx
	jmp	.LBB315_392
.LBB315_391:
	movq	%r15, %rbx
.LBB315_392:
.Ltmp11342:
	leaq	352(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>>
.Ltmp11343:
.LBB315_393:
	cmpq	$0, 192(%rsp)
	je	.LBB315_396
	movq	184(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB315_396
	#MEMBARRIER
.Ltmp11347:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::deferred_exists::NestedSites>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	632(%rsp), %rdi
	callq	*%rax
.Ltmp11348:
	movq	16(%rsp), %r15
	movq	%rbx, %r14
	jmp	.LBB315_397
.LBB315_396:
	movq	16(%rsp), %r15
	movq	%rbx, %r14
.LBB315_397:
	movb	$1, %bpl
.Ltmp11352:
	leaq	1024(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp11353:
	jmp	.LBB315_419
.LBB315_398:
.Ltmp11354:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB315_399:
.Ltmp11261:
	movq	%rax, %r14
	xorl	%ebp, %ebp
	jmp	.LBB315_428
.LBB315_400:
.Ltmp11419:
	movq	%rax, %r14
	cmpq	$6, %r15
	jb	.LBB315_402
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	movq	%r12, %rdi
	callq	__rustc::__rust_dealloc
.LBB315_402:
	movq	16(%rsp), %r15
	jmp	.LBB315_410
.LBB315_403:
.Ltmp11412:
	movq	%rax, %r14
	movq	224(%rsp), %rax
	movq	16(%rsp), %r15
	movq	24(%rsp), %rbx
	leaq	232(%rsp), %rcx
	movq	%rcx, 792(%rsp)
	cmpq	$5, %rax
	jbe	.LBB315_414
.LBB315_404:
	movq	792(%rsp), %rcx
	movq	(%rcx), %rdi
	jmp	.LBB315_413
.LBB315_405:
.Ltmp11461:
	movq	%rax, %r14
	jmp	.LBB315_426
.LBB315_406:
.Ltmp11367:
.LBB315_407:
	movb	$1, %bpl
	movq	%rax, %r14
.Ltmp11371:
	leaq	992(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp11372:
	movq	16(%rsp), %r15
	jmp	.LBB315_419
.LBB315_408:
.Ltmp11477:
	movq	%rax, %r14
	jmp	.LBB315_429
.LBB315_409:
.Ltmp11406:
	movq	%rax, %r14
.LBB315_410:
	movq	24(%rsp), %rbx
	jmp	.LBB315_414
.LBB315_411:
.Ltmp11422:
	movq	%rax, %r14
	movq	528(%rsp), %rax
	movq	16(%rsp), %r15
	movq	24(%rsp), %rbx
	cmpq	$6, %rax
	jb	.LBB315_414
	movq	536(%rsp), %rdi
.LBB315_413:
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB315_414:
	movq	736(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB315_416
	movq	744(%rsp), %rdi
	shlq	$3, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB315_416:
	leaq	368(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB315_417:
	lock		decq	(%rbx)
	movb	$1, %bpl
	jne	.LBB315_419
	#MEMBARRIER
.Ltmp11423:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1104(%rsp), %rdi
	callq	*%rax
.Ltmp11424:
.LBB315_419:
.Ltmp11428:
	leaq	456(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>>
.Ltmp11429:
	jmp	.LBB315_421
.LBB315_420:
.Ltmp11443:
	movq	%rax, %r14
.LBB315_421:
.Ltmp11444:
	leaq	800(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp11445:
.LBB315_422:
	lock		decq	(%r15)
	jne	.LBB315_424
	#MEMBARRIER
.Ltmp11446:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	520(%rsp), %rdi
	callq	*%rax
.Ltmp11447:
.LBB315_424:
	cmpq	$0, 432(%rsp)
	je	.LBB315_426
.Ltmp11451:
	leaq	432(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::DeferredLateral>
.Ltmp11452:
.LBB315_426:
	movq	504(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB315_428
	leaq	504(%rsp), %rdi
	#MEMBARRIER
.Ltmp11470:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp11471:
.LBB315_428:
	leaq	480(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB315_429:
	testb	%bpl, %bpl
	je	.LBB315_431
.Ltmp11478:
	leaq	888(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp11479:
.LBB315_431:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB315_432:
.Ltmp11480:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end315:
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
