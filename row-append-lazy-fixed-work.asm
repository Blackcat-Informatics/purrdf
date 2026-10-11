
.stage/sparql-eval-datalog-per-row-evidence/distinct-fixed-work-current:     file format elf64-x86-64


Disassembly of section .text:

0000000000706280 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push>:
  706280:	55                   	push   %rbp
  706281:	41 57                	push   %r15
  706283:	41 56                	push   %r14
  706285:	41 55                	push   %r13
  706287:	41 54                	push   %r12
  706289:	53                   	push   %rbx
  70628a:	48 81 ec f8 00 00 00 	sub    $0xf8,%rsp
  706291:	48 89 d3             	mov    %rdx,%rbx
  706294:	49 89 f7             	mov    %rsi,%r15
  706297:	49 89 fe             	mov    %rdi,%r14
  70629a:	48 8d 7e 18          	lea    0x18(%rsi),%rdi
  70629e:	4c 8b 66 18          	mov    0x18(%rsi),%r12
  7062a2:	48 8b 6e 28          	mov    0x28(%rsi),%rbp
  7062a6:	4c 39 e5             	cmp    %r12,%rbp
  7062a9:	72 51                	jb     7062fc <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x7c>
  7062ab:	48 8d 45 01          	lea    0x1(%rbp),%rax
  7062af:	4b 8d 0c 24          	lea    (%r12,%r12,1),%rcx
  7062b3:	48 39 c1             	cmp    %rax,%rcx
  7062b6:	48 0f 47 c1          	cmova  %rcx,%rax
  7062ba:	48 83 f8 05          	cmp    $0x5,%rax
  7062be:	41 bd 04 00 00 00    	mov    $0x4,%r13d
  7062c4:	4c 0f 43 e8          	cmovae %rax,%r13
  7062c8:	4d 85 e4             	test   %r12,%r12
  7062cb:	4c 0f 45 e8          	cmovne %rax,%r13
  7062cf:	4d 39 e5             	cmp    %r12,%r13
  7062d2:	76 24                	jbe    7062f8 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x78>
  7062d4:	48 b8 34 33 33 33 33 	movabs $0x333333333333334,%rax
  7062db:	33 33 03 
  7062de:	48 b9 0b 00 00 00 00 	movabs $0x800000000000000b,%rcx
  7062e5:	00 00 80 
  7062e8:	48 ff c8             	dec    %rax
  7062eb:	49 39 c5             	cmp    %rax,%r13
  7062ee:	76 5f                	jbe    70634f <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0xcf>
  7062f0:	48 ff c9             	dec    %rcx
  7062f3:	e9 0a 01 00 00       	jmp    706402 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x182>
  7062f8:	49 8d 7f 18          	lea    0x18(%r15),%rdi
  7062fc:	4c 39 e5             	cmp    %r12,%rbp
  7062ff:	75 06                	jne    706307 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x87>
  706301:	ff 15 d1 49 29 00    	call   *0x2949d1(%rip)        # 99acd8 <_DYNAMIC+0x1550>
  706307:	49 8b 47 20          	mov    0x20(%r15),%rax
  70630b:	48 8d 0c ad 00 00 00 	lea    0x0(,%rbp,4),%rcx
  706312:	00 
  706313:	48 01 e9             	add    %rbp,%rcx
  706316:	48 8b 53 20          	mov    0x20(%rbx),%rdx
  70631a:	48 89 54 c8 20       	mov    %rdx,0x20(%rax,%rcx,8)
  70631f:	0f 10 03             	movups (%rbx),%xmm0
  706322:	0f 10 4b 10          	movups 0x10(%rbx),%xmm1
  706326:	0f 11 4c c8 10       	movups %xmm1,0x10(%rax,%rcx,8)
  70632b:	0f 11 04 c8          	movups %xmm0,(%rax,%rcx,8)
  70632f:	48 ff c5             	inc    %rbp
  706332:	49 89 6f 28          	mov    %rbp,0x28(%r15)
  706336:	49 c7 06 ff ff ff ff 	movq   $0xffffffffffffffff,(%r14)
  70633d:	48 81 c4 f8 00 00 00 	add    $0xf8,%rsp
  706344:	5b                   	pop    %rbx
  706345:	41 5c                	pop    %r12
  706347:	41 5d                	pop    %r13
  706349:	41 5e                	pop    %r14
  70634b:	41 5f                	pop    %r15
  70634d:	5d                   	pop    %rbp
  70634e:	c3                   	ret
  70634f:	48 89 5c 24 08       	mov    %rbx,0x8(%rsp)
  706354:	49 8b 5f 30          	mov    0x30(%r15),%rbx
  706358:	48 85 db             	test   %rbx,%rbx
  70635b:	0f 84 c7 01 00 00    	je     706528 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x2a8>
  706361:	48 8b 3b             	mov    (%rbx),%rdi
  706364:	48 8b 43 08          	mov    0x8(%rbx),%rax
  706368:	ff 50 20             	call   *0x20(%rax)
  70636b:	84 c0                	test   %al,%al
  70636d:	74 15                	je     706384 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x104>
  70636f:	48 b9 0b 00 00 00 00 	movabs $0x800000000000000b,%rcx
  706376:	00 00 80 
  706379:	48 83 c1 fb          	add    $0xfffffffffffffffb,%rcx
  70637d:	48 8b 5c 24 08       	mov    0x8(%rsp),%rbx
  706382:	eb 44                	jmp    7063c8 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x148>
  706384:	4a 8d 04 ed 00 00 00 	lea    0x0(,%r13,8),%rax
  70638b:	00 
  70638c:	48 8d 14 80          	lea    (%rax,%rax,4),%rdx
  706390:	48 8b 33             	mov    (%rbx),%rsi
  706393:	48 8b 43 08          	mov    0x8(%rbx),%rax
  706397:	48 8d 7c 24 40       	lea    0x40(%rsp),%rdi
  70639c:	ff 50 28             	call   *0x28(%rax)
  70639f:	48                   	rex.W
