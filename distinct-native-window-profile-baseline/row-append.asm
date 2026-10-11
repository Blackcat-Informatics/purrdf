
.stage/sparql-eval-datalog-per-row-evidence/distinct-fixed-work-baseline:     file format elf64-x86-64


Disassembly of section .text:

0000000000617400 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push>:
  617400:	55                   	push   %rbp
  617401:	41 57                	push   %r15
  617403:	41 56                	push   %r14
  617405:	41 55                	push   %r13
  617407:	41 54                	push   %r12
  617409:	53                   	push   %rbx
  61740a:	48 81 ec 08 01 00 00 	sub    $0x108,%rsp
  617411:	48 89 54 24 08       	mov    %rdx,0x8(%rsp)
  617416:	49 89 f7             	mov    %rsi,%r15
  617419:	48 89 7c 24 28       	mov    %rdi,0x28(%rsp)
  61741e:	48 b8 0b 00 00 00 00 	movabs $0x800000000000000b,%rax
  617425:	00 00 80 
  617428:	4c 8b 76 30          	mov    0x30(%rsi),%r14
  61742c:	48 8b 5e 18          	mov    0x18(%rsi),%rbx
  617430:	48 8b 6e 28          	mov    0x28(%rsi),%rbp
  617434:	4c 8d 60 ff          	lea    -0x1(%rax),%r12
  617438:	4c 89 64 24 60       	mov    %r12,0x60(%rsp)
  61743d:	48 8d 7c 24 60       	lea    0x60(%rsp),%rdi
  617442:	e8 29 cf d9 ff       	call   3b4370 <core::ptr::drop_glue::<purrdf_sparql_eval::error::EvalError>>
  617447:	48 39 dd             	cmp    %rbx,%rbp
  61744a:	0f 82 0e 03 00 00    	jb     61775e <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x35e>
  617450:	48 8d 45 01          	lea    0x1(%rbp),%rax
  617454:	48 8d 0c 1b          	lea    (%rbx,%rbx,1),%rcx
  617458:	48 39 c1             	cmp    %rax,%rcx
  61745b:	48 0f 47 c1          	cmova  %rcx,%rax
  61745f:	48 83 f8 05          	cmp    $0x5,%rax
  617463:	41 bd 04 00 00 00    	mov    $0x4,%r13d
  617469:	4c 0f 43 e8          	cmovae %rax,%r13
  61746d:	48 85 db             	test   %rbx,%rbx
  617470:	4c 0f 45 e8          	cmovne %rax,%r13
  617474:	49 39 dd             	cmp    %rbx,%r13
  617477:	0f 86 e1 02 00 00    	jbe    61775e <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x35e>
  61747d:	48 b8 34 33 33 33 33 	movabs $0x333333333333334,%rax
  617484:	33 33 03 
  617487:	48 ff c8             	dec    %rax
  61748a:	49 39 c5             	cmp    %rax,%r13
  61748d:	76 05                	jbe    617494 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x94>
  61748f:	e9 ac 00 00 00       	jmp    617540 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x140>
  617494:	4d 85 f6             	test   %r14,%r14
  617497:	0f 84 64 01 00 00    	je     617601 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x201>
  61749d:	49 8b 3e             	mov    (%r14),%rdi
  6174a0:	49 8b 46 08          	mov    0x8(%r14),%rax
  6174a4:	ff 50 20             	call   *0x20(%rax)
  6174a7:	84 c0                	test   %al,%al
  6174a9:	74 10                	je     6174bb <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0xbb>
  6174ab:	48 bb 0b 00 00 00 00 	movabs $0x800000000000000b,%rbx
  6174b2:	00 00 80 
  6174b5:	48 83 c3 fb          	add    $0xfffffffffffffffb,%rbx
  6174b9:	eb 3f                	jmp    6174fa <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0xfa>
  6174bb:	4a 8d 04 ed 00 00 00 	lea    0x0(,%r13,8),%rax
  6174c2:	00 
  6174c3:	48 8d 14 80          	lea    (%rax,%rax,4),%rdx
  6174c7:	49 8b 36             	mov    (%r14),%rsi
  6174ca:	49 8b 46 08          	mov    0x8(%r14),%rax
  6174ce:	48 8d 7c 24 60       	lea    0x60(%rsp),%rdi
  6174d3:	ff 50 28             	call   *0x28(%rax)
  6174d6:	48 8b 4c 24 60       	mov    0x60(%rsp),%rcx
  6174db:	4c 8b 64 24 68       	mov    0x68(%rsp),%r12
  6174e0:	4c 8b 74 24 70       	mov    0x70(%rsp),%r14
  6174e5:	48 83 f9 ff          	cmp    $0xffffffffffffffff,%rcx
  6174e9:	0f 84 15 01 00 00    	je     617604 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x204>
  6174ef:	48 8b 44 24 78       	mov    0x78(%rsp),%rax
  6174f4:	4d 89 f7             	mov    %r14,%r15
  6174f7:	48 89 cb             	mov    %rcx,%rbx
  6174fa:	0f 28 84 24 80 00 00 	movaps 0x80(%rsp),%xmm0
  617501:	00 
  617502:	0f 28 8c 24 90 00 00 	movaps 0x90(%rsp),%xmm1
  617509:	00 
  61750a:	0f 28 94 24 a0 00 00 	movaps 0xa0(%rsp),%xmm2
  617511:	00 
  617512:	0f 28 9c 24 b0 00 00 	movaps 0xb0(%rsp),%xmm3
  617519:	00 
  61751a:	0f 29 9c 24 f0 00 00 	movaps %xmm3,0xf0(%rsp)
  617521:	00 
  617522:	0f 29 94 24 e0 00 00 	movaps %xmm2,0xe0(%rsp)
  617529:	00 
  61752a:	0f 29 8c 24 d0 00 00 	movaps %xmm1,0xd0(%rsp)
  617531:	00 
  617532:	0f 29 84 24 c0 00 00 	movaps %xmm0,0xc0(%rsp)
  617539:	00 
  61753a:	4d 89 e5             	mov    %r12,%r13
  61753d:	49 89 dc             	mov    %rbx,%r12
  617540:	48 8b 4c 24 28       	mov    0x28(%rsp),%rcx
  617545:	4c 89 21             	mov    %r12,(%rcx)
  617548:	4c 89 69 08          	mov    %r13,0x8(%rcx)
  61754c:	4c 89 79 10          	mov    %r15,0x10(%rcx)
  617550:	48 89 41 18          	mov    %rax,0x18(%rcx)
  617554:	0f 28 84 24 c0 00 00 	movaps 0xc0(%rsp),%xmm0
  61755b:	00 
  61755c:	0f 28 8c 24 d0 00 00 	movaps 0xd0(%rsp),%xmm1
  617563:	00 
  617564:	0f 28 94 24 e0 00 00 	movaps 0xe0(%rsp),%xmm2
  61756b:	00 
  61756c:	0f 28 9c 24 f0 00 00 	movaps 0xf0(%rsp),%xmm3
  617573:	00 
  617574:	0f 29 59 50          	movaps %xmm3,0x50(%rcx)
  617578:	0f 29 51 40          	movaps %xmm2,0x40(%rcx)
  61757c:	0f 29 49 30          	movaps %xmm1,0x30(%rcx)
  617580:	0f 29 41 20          	movaps %xmm0,0x20(%rcx)
  617584:	48 8b 4c 24 08       	mov    0x8(%rsp),%rcx
  617589:	48 8b 01             	mov    (%rcx),%rax
  61758c:	48 8b 79 08          	mov    0x8(%rcx),%rdi
  617590:	48 85 c0             	test   %rax,%rax
  617593:	74 1b                	je     6175b0 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x1b0>
  617595:	48 89 fb             	mov    %rdi,%rbx
  617598:	48 83 f8 05          	cmp    $0x5,%rax
  61759c:	0f 86 1c 02 00 00    	jbe    6177be <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x3be>
  6175a2:	48 89 df             	mov    %rbx,%rdi
  6175a5:	ff 15 a5 0e 26 00    	call   *0x260ea5(%rip)        # 878450 <free@GLIBC_2.2.5>
  6175ab:	e9 0e 02 00 00       	jmp    6177be <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x3be>
  6175b0:	f0 48 ff 4f 38       	lock decq 0x38(%rdi)
  6175b5:	0f 85 03 02 00 00    	jne    6177be <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x3be>
  6175bb:	4c 8b 27             	mov    (%rdi),%r12
  6175be:	4c 8b 77 08          	mov    0x8(%rdi),%r14
  6175c2:	48 8b 5f 28          	mov    0x28(%rdi),%rbx
  6175c6:	4c 8b 7f 30          	mov    0x30(%rdi),%r15
  6175ca:	ff 15 80 0e 26 00    	call   *0x260e80(%rip)        # 878450 <free@GLIBC_2.2.5>
  6175d0:	49 83 fc 06          	cmp    $0x6,%r12
  6175d4:	72 09                	jb     6175df <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x1df>
  6175d6:	4c 89 f7             	mov    %r14,%rdi
  6175d9:	ff 15 71 0e 26 00    	call   *0x260e71(%rip)        # 878450 <free@GLIBC_2.2.5>
  6175df:	48 85 db             	test   %rbx,%rbx
  6175e2:	0f 84 d6 01 00 00    	je     6177be <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x3be>
  6175e8:	49 8b 07             	mov    (%r15),%rax
  6175eb:	48 85 c0             	test   %rax,%rax
  6175ee:	74 05                	je     6175f5 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x1f5>
  6175f0:	48 89 df             	mov    %rbx,%rdi
  6175f3:	ff d0                	call   *%rax
  6175f5:	49 83 7f 08 00       	cmpq   $0x0,0x8(%r15)
  6175fa:	75 a6                	jne    6175a2 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x1a2>
  6175fc:	e9 bd 01 00 00       	jmp    6177be <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x3be>
  617601:	45 31 e4             	xor    %r12d,%r12d
  617604:	4c 89 64 24 38       	mov    %r12,0x38(%rsp)
  617609:	4c 89 74 24 40       	mov    %r14,0x40(%rsp)
  61760e:	48 c7 44 24 48 00 00 	movq   $0x0,0x48(%rsp)
  617615:	00 00 
  617617:	48 c7 44 24 50 08 00 	movq   $0x8,0x50(%rsp)
  61761e:	00 00 
  617620:	48 c7 44 24 58 00 00 	movq   $0x0,0x58(%rsp)
  617627:	00 00 
  617629:	48 8d 7c 24 60       	lea    0x60(%rsp),%rdi
  61762e:	ba 08 00 00 00       	mov    $0x8,%edx
  617633:	41 b8 08 00 00 00    	mov    $0x8,%r8d
  617639:	41 b9 28 00 00 00    	mov    $0x28,%r9d
  61763f:	31 f6                	xor    %esi,%esi
  617641:	4c 89 e9             	mov    %r13,%rcx
  617644:	e8 97 f0 fb ff       	call   5d66e0 <<alloc::raw_vec::RawVecInner>::finish_grow>
  617649:	83 7c 24 60 01       	cmpl   $0x1,0x60(%rsp)
  61764e:	75 4a                	jne    61769a <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x29a>
  617650:	48 8d 7c 24 48       	lea    0x48(%rsp),%rdi
  617655:	e8 96 8b d8 ff       	call   3a01f0 <core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::solution::RetainedRow>>>
  61765a:	4c 8d 2d 17 af a7 ff 	lea    -0x5850e9(%rip),%r13        # 92578 <anon.8345c53e15290efb765041df6b8127af.229.llvm.10639538939204970671+0xaaf>
  617661:	41 bf 16 00 00 00    	mov    $0x16,%r15d
  617667:	4d 85 e4             	test   %r12,%r12
  61766a:	48 bb 0b 00 00 00 00 	movabs $0x800000000000000b,%rbx
  617671:	00 00 80 
  617674:	74 1d                	je     617693 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x293>
  617676:	49 8b 06             	mov    (%r14),%rax
  617679:	48 85 c0             	test   %rax,%rax
  61767c:	74 05                	je     617683 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x283>
  61767e:	4c 89 e7             	mov    %r12,%rdi
  617681:	ff d0                	call   *%rax
  617683:	49 83 7e 08 00       	cmpq   $0x0,0x8(%r14)
  617688:	74 09                	je     617693 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x293>
  61768a:	4c 89 e7             	mov    %r12,%rdi
  61768d:	ff 15 bd 0d 26 00    	call   *0x260dbd(%rip)        # 878450 <free@GLIBC_2.2.5>
  617693:	31 c0                	xor    %eax,%eax
  617695:	e9 a3 fe ff ff       	jmp    61753d <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x13d>
  61769a:	48 8b 44 24 68       	mov    0x68(%rsp),%rax
  61769f:	4c 89 6c 24 10       	mov    %r13,0x10(%rsp)
  6176a4:	48 89 44 24 18       	mov    %rax,0x18(%rsp)
  6176a9:	48 c7 44 24 20 00 00 	movq   $0x0,0x20(%rsp)
  6176b0:	00 00 
  6176b2:	49 8b 4f 20          	mov    0x20(%r15),%rcx
  6176b6:	48 89 4c 24 30       	mov    %rcx,0x30(%rsp)
  6176bb:	4c 39 ed             	cmp    %r13,%rbp
  6176be:	0f 87 0c 01 00 00    	ja     6177d0 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x3d0>
  6176c4:	31 db                	xor    %ebx,%ebx
  6176c6:	48 85 ed             	test   %rbp,%rbp
  6176c9:	74 1f                	je     6176ea <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x2ea>
  6176cb:	48 8d 0c 9b          	lea    (%rbx,%rbx,4),%rcx
  6176cf:	48 8d 3c c8          	lea    (%rax,%rcx,8),%rdi
  6176d3:	48 8d 04 ed 00 00 00 	lea    0x0(,%rbp,8),%rax
  6176da:	00 
  6176db:	48 8d 14 80          	lea    (%rax,%rax,4),%rdx
  6176df:	48 8b 74 24 30       	mov    0x30(%rsp),%rsi
  6176e4:	ff 15 76 0d 26 00    	call   *0x260d76(%rip)        # 878460 <memcpy@GLIBC_2.14>
  6176ea:	48 01 eb             	add    %rbp,%rbx
  6176ed:	48 89 5c 24 20       	mov    %rbx,0x20(%rsp)
  6176f2:	49 c7 47 28 00 00 00 	movq   $0x0,0x28(%r15)
  6176f9:	00 
  6176fa:	49 8d 5f 18          	lea    0x18(%r15),%rbx
  6176fe:	48 89 df             	mov    %rbx,%rdi
  617701:	e8 ea 8a d8 ff       	call   3a01f0 <core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::solution::RetainedRow>>>
  617706:	48 8b 44 24 20       	mov    0x20(%rsp),%rax
  61770b:	48 89 43 10          	mov    %rax,0x10(%rbx)
  61770f:	0f 10 44 24 10       	movups 0x10(%rsp),%xmm0
  617714:	0f 11 03             	movups %xmm0,(%rbx)
  617717:	49 83 3f 00          	cmpq   $0x0,(%r15)
  61771b:	74 2a                	je     617747 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x347>
  61771d:	4d 8b 6f 08          	mov    0x8(%r15),%r13
  617721:	4d 85 ed             	test   %r13,%r13
  617724:	74 21                	je     617747 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x347>
  617726:	49 8b 5f 10          	mov    0x10(%r15),%rbx
  61772a:	48 8b 03             	mov    (%rbx),%rax
  61772d:	48 85 c0             	test   %rax,%rax
  617730:	74 05                	je     617737 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x337>
  617732:	4c 89 ef             	mov    %r13,%rdi
  617735:	ff d0                	call   *%rax
  617737:	48 83 7b 08 00       	cmpq   $0x0,0x8(%rbx)
  61773c:	74 09                	je     617747 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x347>
  61773e:	4c 89 ef             	mov    %r13,%rdi
  617741:	ff 15 09 0d 26 00    	call   *0x260d09(%rip)        # 878450 <free@GLIBC_2.2.5>
  617747:	49 c7 07 01 00 00 00 	movq   $0x1,(%r15)
  61774e:	4d 89 67 08          	mov    %r12,0x8(%r15)
  617752:	4d 89 77 10          	mov    %r14,0x10(%r15)
  617756:	49 8b 5f 18          	mov    0x18(%r15),%rbx
  61775a:	49 8b 6f 28          	mov    0x28(%r15),%rbp
  61775e:	48 8b 44 24 08       	mov    0x8(%rsp),%rax
  617763:	4c 8b 20             	mov    (%rax),%r12
  617766:	4c 8b 68 08          	mov    0x8(%rax),%r13
  61776a:	48 39 dd             	cmp    %rbx,%rbp
  61776d:	75 0a                	jne    617779 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x379>
  61776f:	49 8d 7f 18          	lea    0x18(%r15),%rdi
  617773:	ff 15 ef 22 26 00    	call   *0x2622ef(%rip)        # 879a68 <_DYNAMIC+0x1820>
  617779:	48 8b 54 24 08       	mov    0x8(%rsp),%rdx
  61777e:	48 83 c2 10          	add    $0x10,%rdx
  617782:	49 8b 47 20          	mov    0x20(%r15),%rax
  617786:	48 8d 0c ad 00 00 00 	lea    0x0(,%rbp,4),%rcx
  61778d:	00 
  61778e:	48 01 e9             	add    %rbp,%rcx
  617791:	4c 89 24 c8          	mov    %r12,(%rax,%rcx,8)
  617795:	4c 89 6c c8 08       	mov    %r13,0x8(%rax,%rcx,8)
  61779a:	0f 10 02             	movups (%rdx),%xmm0
  61779d:	0f 11 44 c8 10       	movups %xmm0,0x10(%rax,%rcx,8)
  6177a2:	48 8b 52 10          	mov    0x10(%rdx),%rdx
  6177a6:	48 89 54 c8 20       	mov    %rdx,0x20(%rax,%rcx,8)
  6177ab:	48 ff c5             	inc    %rbp
  6177ae:	49 89 6f 28          	mov    %rbp,0x28(%r15)
  6177b2:	48 8b 44 24 28       	mov    0x28(%rsp),%rax
  6177b7:	48 c7 00 ff ff ff ff 	movq   $0xffffffffffffffff,(%rax)
  6177be:	48 81 c4 08 01 00 00 	add    $0x108,%rsp
  6177c5:	5b                   	pop    %rbx
  6177c6:	41 5c                	pop    %r12
  6177c8:	41 5d                	pop    %r13
  6177ca:	41 5e                	pop    %r14
  6177cc:	41 5f                	pop    %r15
  6177ce:	5d                   	pop    %rbp
  6177cf:	c3                   	ret
  6177d0:	48 8d 7c 24 10       	lea    0x10(%rsp),%rdi
  6177d5:	b9 08 00 00 00       	mov    $0x8,%ecx
  6177da:	41 b8 28 00 00 00    	mov    $0x28,%r8d
  6177e0:	31 f6                	xor    %esi,%esi
  6177e2:	48 89 ea             	mov    %rbp,%rdx
  6177e5:	e8 f6 19 f1 ff       	call   5291e0 <<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>>
  6177ea:	48 8b 44 24 18       	mov    0x18(%rsp),%rax
  6177ef:	48 8b 5c 24 20       	mov    0x20(%rsp),%rbx
  6177f4:	e9 d2 fe ff ff       	jmp    6176cb <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x2cb>
  6177f9:	48 89 c5             	mov    %rax,%rbp
  6177fc:	48 8d 7c 24 10       	lea    0x10(%rsp),%rdi
  617801:	e8 ea 89 d8 ff       	call   3a01f0 <core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::solution::RetainedRow>>>
  617806:	eb 52                	jmp    61785a <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x45a>
  617808:	48 89 c5             	mov    %rax,%rbp
  61780b:	48 83 7b 08 00       	cmpq   $0x0,0x8(%rbx)
  617810:	74 09                	je     61781b <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x41b>
  617812:	4c 89 ef             	mov    %r13,%rdi
  617815:	ff 15 35 0c 26 00    	call   *0x260c35(%rip)        # 878450 <free@GLIBC_2.2.5>
  61781b:	49 c7 07 01 00 00 00 	movq   $0x1,(%r15)
  617822:	4d 89 67 08          	mov    %r12,0x8(%r15)
  617826:	4d 89 77 10          	mov    %r14,0x10(%r15)
  61782a:	eb 74                	jmp    6178a0 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x4a0>
  61782c:	48 89 c5             	mov    %rax,%rbp
  61782f:	49 83 7e 08 00       	cmpq   $0x0,0x8(%r14)
  617834:	74 6a                	je     6178a0 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x4a0>
  617836:	4c 89 e7             	mov    %r12,%rdi
  617839:	ff 15 11 0c 26 00    	call   *0x260c11(%rip)        # 878450 <free@GLIBC_2.2.5>
  61783f:	eb 5f                	jmp    6178a0 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x4a0>
  617841:	48 89 c5             	mov    %rax,%rbp
  617844:	48 8b 44 24 20       	mov    0x20(%rsp),%rax
  617849:	48 89 43 10          	mov    %rax,0x10(%rbx)
  61784d:	0f 10 44 24 10       	movups 0x10(%rsp),%xmm0
  617852:	0f 11 03             	movups %xmm0,(%rbx)
  617855:	eb 03                	jmp    61785a <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x45a>
  617857:	48 89 c5             	mov    %rax,%rbp
  61785a:	48 8d 7c 24 38       	lea    0x38(%rsp),%rdi
  61785f:	e8 ec f8 d9 ff       	call   3b7150 <core::ptr::drop_glue::<purrdf_sparql_eval::workspace::WorkspaceAllocation>>
  617864:	eb 3a                	jmp    6178a0 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x4a0>
  617866:	ff 15 ac 0c 26 00    	call   *0x260cac(%rip)        # 878518 <_DYNAMIC+0x2d0>
  61786c:	48 89 c5             	mov    %rax,%rbp
  61786f:	49 83 7f 08 00       	cmpq   $0x0,0x8(%r15)
  617874:	74 3b                	je     6178b1 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x4b1>
  617876:	48 89 df             	mov    %rbx,%rdi
  617879:	ff 15 d1 0b 26 00    	call   *0x260bd1(%rip)        # 878450 <free@GLIBC_2.2.5>
  61787f:	48 89 ef             	mov    %rbp,%rdi
  617882:	e8 49 33 22 00       	call   83abd0 <_Unwind_Resume@plt>
  617887:	48 89 c5             	mov    %rax,%rbp
  61788a:	4c 89 e7             	mov    %r12,%rdi
  61788d:	4c 89 ee             	mov    %r13,%rsi
  617890:	e8 ab 7b d7 ff       	call   38f440 <core::ptr::drop_glue::<purrdf_sparql_eval::solution::RowStorage<purrdf_core::ir::term::TermId>>>
  617895:	eb 1a                	jmp    6178b1 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x4b1>
  617897:	ff 15 7b 0c 26 00    	call   *0x260c7b(%rip)        # 878518 <_DYNAMIC+0x2d0>
  61789d:	48 89 c5             	mov    %rax,%rbp
  6178a0:	48 8b 44 24 08       	mov    0x8(%rsp),%rax
  6178a5:	48 8b 38             	mov    (%rax),%rdi
  6178a8:	48 8b 70 08          	mov    0x8(%rax),%rsi
  6178ac:	e8 8f 7b d7 ff       	call   38f440 <core::ptr::drop_glue::<purrdf_sparql_eval::solution::RowStorage<purrdf_core::ir::term::TermId>>>
  6178b1:	48 89 ef             	mov    %rbp,%rdi
  6178b4:	e8 17 33 22 00       	call   83abd0 <_Unwind_Resume@plt>
  6178b9:	ff 15 59 0c 26 00    	call   *0x260c59(%rip)        # 878518 <_DYNAMIC+0x2d0>
