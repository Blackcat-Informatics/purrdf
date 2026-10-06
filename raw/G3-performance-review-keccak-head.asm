
/opt/.cargo/slots/882ed9338c736857/0/target/debug/g3-throughput-head:     file format elf64-x86-64


Disassembly of section .text:

0000000000068220 <purrdf_hash::sha3::keccak_f1600>:
   68220:	55                   	push   %rbp
   68221:	41 57                	push   %r15
   68223:	41 56                	push   %r14
   68225:	41 55                	push   %r13
   68227:	41 54                	push   %r12
   68229:	53                   	push   %rbx
   6822a:	48 81 ec 98 01 00 00 	sub    $0x198,%rsp
   68231:	48 8b 8f a0 00 00 00 	mov    0xa0(%rdi),%rcx
   68238:	48 8b 97 a8 00 00 00 	mov    0xa8(%rdi),%rdx
   6823f:	48 8b 47 08          	mov    0x8(%rdi),%rax
   68243:	48 8b 2f             	mov    (%rdi),%rbp
   68246:	48 8b 77 28          	mov    0x28(%rdi),%rsi
   6824a:	4c 8b 67 18          	mov    0x18(%rdi),%r12
   6824e:	48 89 7c 24 58       	mov    %rdi,0x58(%rsp)
   68253:	48 89 4c 24 10       	mov    %rcx,0x10(%rsp)
   68258:	48 8b 8f 88 00 00 00 	mov    0x88(%rdi),%rcx
   6825f:	48 89 54 24 30       	mov    %rdx,0x30(%rsp)
   68264:	48 8b 97 b0 00 00 00 	mov    0xb0(%rdi),%rdx
   6826b:	48 89 44 24 d0       	mov    %rax,-0x30(%rsp)
   68270:	b8 08 00 00 00       	mov    $0x8,%eax
   68275:	48 89 44 24 40       	mov    %rax,0x40(%rsp)
   6827a:	48 89 4c 24 f0       	mov    %rcx,-0x10(%rsp)
   6827f:	48 8b 8f 90 00 00 00 	mov    0x90(%rdi),%rcx
   68286:	48 89 54 24 e0       	mov    %rdx,-0x20(%rsp)
   6828b:	48 8b 97 b8 00 00 00 	mov    0xb8(%rdi),%rdx
   68292:	48 89 4c 24 08       	mov    %rcx,0x8(%rsp)
   68297:	48 8b 8f 98 00 00 00 	mov    0x98(%rdi),%rcx
   6829e:	48 89 54 24 20       	mov    %rdx,0x20(%rsp)
   682a3:	48 8b 97 c0 00 00 00 	mov    0xc0(%rdi),%rdx
   682aa:	48 89 4c 24 e8       	mov    %rcx,-0x18(%rsp)
   682af:	48 8b 4f 50          	mov    0x50(%rdi),%rcx
   682b3:	48 89 54 24 b0       	mov    %rdx,-0x50(%rsp)
   682b8:	48 8b 57 78          	mov    0x78(%rdi),%rdx
   682bc:	48 89 4c 24 c0       	mov    %rcx,-0x40(%rsp)
   682c1:	48 8b 4f 30          	mov    0x30(%rdi),%rcx
   682c5:	48 89 14 24          	mov    %rdx,(%rsp)
   682c9:	48 8b 57 58          	mov    0x58(%rdi),%rdx
   682cd:	48 89 4c 24 c8       	mov    %rcx,-0x38(%rsp)
   682d2:	48 8b 8f 80 00 00 00 	mov    0x80(%rdi),%rcx
   682d9:	48 89 54 24 d8       	mov    %rdx,-0x28(%rsp)
   682de:	48 8b 57 38          	mov    0x38(%rdi),%rdx
   682e2:	48 89 4c 24 18       	mov    %rcx,0x18(%rsp)
   682e7:	48 8b 4f 60          	mov    0x60(%rdi),%rcx
   682eb:	48 89 54 24 88       	mov    %rdx,-0x78(%rsp)
   682f0:	48 8b 57 40          	mov    0x40(%rdi),%rdx
   682f4:	48 89 4c 24 a8       	mov    %rcx,-0x58(%rsp)
   682f9:	48 8b 4f 68          	mov    0x68(%rdi),%rcx
   682fd:	48 89 54 24 b8       	mov    %rdx,-0x48(%rsp)
   68302:	48 8b 57 48          	mov    0x48(%rdi),%rdx
   68306:	48 89 4c 24 80       	mov    %rcx,-0x80(%rsp)
   6830b:	48 8b 4f 70          	mov    0x70(%rdi),%rcx
   6830f:	48 89 54 24 28       	mov    %rdx,0x28(%rsp)
   68314:	48 8b 57 10          	mov    0x10(%rdi),%rdx
   68318:	48 89 4c 24 a0       	mov    %rcx,-0x60(%rsp)
   6831d:	48 8b 4f 20          	mov    0x20(%rdi),%rcx
   68321:	48 89 54 24 98       	mov    %rdx,-0x68(%rsp)
   68326:	48 89 4c 24 90       	mov    %rcx,-0x70(%rsp)
   6832b:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
   68330:	48 8b 54 24 e0       	mov    -0x20(%rsp),%rdx
   68335:	4c 8b 44 24 08       	mov    0x8(%rsp),%r8
   6833a:	48 89 f1             	mov    %rsi,%rcx
   6833d:	4c 8b 5c 24 d0       	mov    -0x30(%rsp),%r11
   68342:	48 89 74 24 f8       	mov    %rsi,-0x8(%rsp)
   68347:	48 8b 74 24 d8       	mov    -0x28(%rsp),%rsi
   6834c:	4c 89 64 24 38       	mov    %r12,0x38(%rsp)
   68351:	4c 8b 54 24 e8       	mov    -0x18(%rsp),%r10
   68356:	4c 8b 4c 24 b0       	mov    -0x50(%rsp),%r9
   6835b:	48 8b 44 24 10       	mov    0x10(%rsp),%rax
   68360:	4c 8b 6c 24 98       	mov    -0x68(%rsp),%r13
   68365:	4c 8b 74 24 a8       	mov    -0x58(%rsp),%r14
   6836a:	48 33 0c 24          	xor    (%rsp),%rcx
   6836e:	48 33 74 24 30       	xor    0x30(%rsp),%rsi
   68373:	4c 33 5c 24 18       	xor    0x18(%rsp),%r11
   68378:	48 33 44 24 c0       	xor    -0x40(%rsp),%rax
   6837d:	48 89 d7             	mov    %rdx,%rdi
   68380:	49 89 d7             	mov    %rdx,%r15
   68383:	48 8b 54 24 20       	mov    0x20(%rsp),%rdx
   68388:	4d 31 e0             	xor    %r12,%r8
   6838b:	4c 8b 64 24 a0       	mov    -0x60(%rsp),%r12
   68390:	48 33 7c 24 f0       	xor    -0x10(%rsp),%rdi
   68395:	48 33 54 24 b8       	xor    -0x48(%rsp),%rdx
   6839a:	49 31 f3             	xor    %rsi,%r11
   6839d:	4c 33 5c 24 c8       	xor    -0x38(%rsp),%r11
   683a2:	4c 89 ee             	mov    %r13,%rsi
   683a5:	4c 31 f6             	xor    %r14,%rsi
   683a8:	48 31 c8             	xor    %rcx,%rax
   683ab:	48 31 e8             	xor    %rbp,%rax
   683ae:	4d 31 e2             	xor    %r12,%r10
   683b1:	48 31 fe             	xor    %rdi,%rsi
   683b4:	48 8b 7c 24 28       	mov    0x28(%rsp),%rdi
   683b9:	49 31 d0             	xor    %rdx,%r8
   683bc:	48 8b 54 24 90       	mov    -0x70(%rsp),%rdx
   683c1:	4c 33 44 24 80       	xor    -0x80(%rsp),%r8
   683c6:	c4 c3 fb f0 cb 3f    	rorx   $0x3f,%r11,%rcx
   683cc:	49 31 d1             	xor    %rdx,%r9
   683cf:	c4 c3 fb f0 d8 3f    	rorx   $0x3f,%r8,%rbx
   683d5:	4d 31 d1             	xor    %r10,%r9
   683d8:	4c 8b 54 24 88       	mov    -0x78(%rsp),%r10
   683dd:	4c 31 db             	xor    %r11,%rbx
   683e0:	c4 63 fb f0 d8 3f    	rorx   $0x3f,%rax,%r11
   683e6:	4d 31 c3             	xor    %r8,%r11
   683e9:	4d 89 f0             	mov    %r14,%r8
   683ec:	49 31 df             	xor    %rbx,%r15
   683ef:	49 31 d8             	xor    %rbx,%r8
   683f2:	48 31 5c 24 f0       	xor    %rbx,-0x10(%rsp)
   683f7:	49 31 f9             	xor    %rdi,%r9
   683fa:	4c 8b 74 24 d8       	mov    -0x28(%rsp),%r14
   683ff:	4c 31 c9             	xor    %r9,%rcx
   68402:	c4 43 fb f0 c9 3f    	rorx   $0x3f,%r9,%r9
   68408:	4c 31 da             	xor    %r11,%rdx
   6840b:	4c 89 7c 24 e0       	mov    %r15,-0x20(%rsp)
   68410:	4c 31 df             	xor    %r11,%rdi
   68413:	4c 31 5c 24 e8       	xor    %r11,-0x18(%rsp)
   68418:	4c 8b 7c 24 18       	mov    0x18(%rsp),%r15
   6841d:	48 89 54 24 90       	mov    %rdx,-0x70(%rsp)
   68422:	48 8b 54 24 c0       	mov    -0x40(%rsp),%rdx
   68427:	48 31 cd             	xor    %rcx,%rbp
   6842a:	48 31 4c 24 f8       	xor    %rcx,-0x8(%rsp)
   6842f:	4c 31 d6             	xor    %r10,%rsi
   68432:	49 31 da             	xor    %rbx,%r10
   68435:	4c 31 eb             	xor    %r13,%rbx
   68438:	4c 8b 6c 24 08       	mov    0x8(%rsp),%r13
   6843d:	4c 89 54 24 88       	mov    %r10,-0x78(%rsp)
   68442:	4c 8b 54 24 38       	mov    0x38(%rsp),%r10
   68447:	49 31 f1             	xor    %rsi,%r9
   6844a:	c4 e3 fb f0 f6 3f    	rorx   $0x3f,%rsi,%rsi
   68450:	48 89 5c 24 a8       	mov    %rbx,-0x58(%rsp)
   68455:	48 8b 5c 24 10       	mov    0x10(%rsp),%rbx
   6845a:	4c 31 4c 24 80       	xor    %r9,-0x80(%rsp)
   6845f:	4c 31 4c 24 20       	xor    %r9,0x20(%rsp)
   68464:	48 31 c6             	xor    %rax,%rsi
   68467:	48 8b 44 24 c8       	mov    -0x38(%rsp),%rax
   6846c:	48 31 ca             	xor    %rcx,%rdx
   6846f:	49 31 f7             	xor    %rsi,%r15
   68472:	49 31 f6             	xor    %rsi,%r14
   68475:	4d 31 cd             	xor    %r9,%r13
   68478:	4d 31 ca             	xor    %r9,%r10
   6847b:	4c 33 4c 24 b8       	xor    -0x48(%rsp),%r9
   68480:	48 31 cb             	xor    %rcx,%rbx
   68483:	48 33 0c 24          	xor    (%rsp),%rcx
   68487:	48 31 f0             	xor    %rsi,%rax
   6848a:	c4 e3 fb f0 c0 14    	rorx   $0x14,%rax,%rax
   68490:	4c 89 4c 24 98       	mov    %r9,-0x68(%rsp)
   68495:	4c 8b 4c 24 b0       	mov    -0x50(%rsp),%r9
   6849a:	48 89 4c 24 b0       	mov    %rcx,-0x50(%rsp)
   6849f:	c4 c3 fb f0 c8 15    	rorx   $0x15,%r8,%rcx
   684a5:	4d 31 d9             	xor    %r11,%r9
   684a8:	4d 31 e3             	xor    %r12,%r11
   684ab:	4c 8b 64 24 d0       	mov    -0x30(%rsp),%r12
   684b0:	c4 43 fb f0 c1 32    	rorx   $0x32,%r9,%r8
   684b6:	c4 62 b8 f2 cd       	andn   %rbp,%r8,%r9
   684bb:	49 31 f4             	xor    %rsi,%r12
   684be:	48 33 74 24 30       	xor    0x30(%rsp),%rsi
   684c3:	48 89 74 24 a0       	mov    %rsi,-0x60(%rsp)
   684c8:	c4 c3 fb f0 f5 2b    	rorx   $0x2b,%r13,%rsi
   684ce:	c4 62 f0 f2 ee       	andn   %rsi,%rcx,%r13
   684d3:	49 31 f1             	xor    %rsi,%r9
   684d6:	c4 c2 c8 f2 f0       	andn   %r8,%rsi,%rsi
   684db:	48 31 ce             	xor    %rcx,%rsi
   684de:	c4 e2 f8 f2 c9       	andn   %rcx,%rax,%rcx
   684e3:	49 31 c5             	xor    %rax,%r13
   684e6:	c4 e2 d0 f2 c0       	andn   %rax,%rbp,%rax
   684eb:	4c 89 4c 24 c8       	mov    %r9,-0x38(%rsp)
   684f0:	c4 63 fb f0 cb 2e    	rorx   $0x2e,%rbx,%r9
   684f6:	c4 e3 fb f0 5c 24 90 	rorx   $0x25,-0x70(%rsp),%rbx
   684fd:	25 
   684fe:	48 89 4c 24 38       	mov    %rcx,0x38(%rsp)
   68503:	c4 e3 fb f0 cf 2c    	rorx   $0x2c,%rdi,%rcx
   68509:	c4 e3 fb f0 7c 24 e0 	rorx   $0x3,-0x20(%rsp),%rdi
   68510:	03 
   68511:	4c 31 c0             	xor    %r8,%rax
   68514:	c4 43 fb f0 c7 13    	rorx   $0x13,%r15,%r8
   6851a:	48 89 74 24 10       	mov    %rsi,0x10(%rsp)
   6851f:	c4 e3 fb f0 f2 3d    	rorx   $0x3d,%rdx,%rsi
   68525:	4c 89 2c 24          	mov    %r13,(%rsp)
   68529:	c4 42 c8 f2 f8       	andn   %r8,%rsi,%r15
   6852e:	48 89 44 24 18       	mov    %rax,0x18(%rsp)
   68533:	c4 c3 fb f0 c2 24    	rorx   $0x24,%r10,%rax
   68539:	49 31 cf             	xor    %rcx,%r15
   6853c:	4c 89 7c 24 28       	mov    %r15,0x28(%rsp)
   68541:	c4 e2 b8 f2 d7       	andn   %rdi,%r8,%rdx
   68546:	c4 62 c0 f2 d0       	andn   %rax,%rdi,%r10
   6854b:	48 31 f2             	xor    %rsi,%rdx
   6854e:	c4 e2 f0 f2 f6       	andn   %rsi,%rcx,%rsi
   68553:	4d 31 c2             	xor    %r8,%r10
   68556:	c4 63 fb f0 44 24 e8 	rorx   $0x38,-0x18(%rsp),%r8
   6855d:	38 
   6855e:	48 31 c6             	xor    %rax,%rsi
   68561:	c4 e2 f8 f2 c1       	andn   %rcx,%rax,%rax
   68566:	c4 e3 fb f0 4c 24 88 	rorx   $0x3a,-0x78(%rsp),%rcx
   6856d:	3a 
   6856e:	4c 89 54 24 08       	mov    %r10,0x8(%rsp)
   68573:	c4 43 fb f0 d4 3f    	rorx   $0x3f,%r12,%r10
   68579:	48 89 54 24 d8       	mov    %rdx,-0x28(%rsp)
   6857e:	48 89 74 24 e0       	mov    %rsi,-0x20(%rsp)
   68583:	c4 e3 fb f0 74 24 80 	rorx   $0x27,-0x80(%rsp),%rsi
   6858a:	27 
   6858b:	48 31 f8             	xor    %rdi,%rax
   6858e:	c4 c2 b0 f2 d2       	andn   %r10,%r9,%rdx
   68593:	48 89 44 24 d0       	mov    %rax,-0x30(%rsp)
   68598:	c4 c2 b8 f2 f9       	andn   %r9,%r8,%rdi
   6859d:	4c 31 c2             	xor    %r8,%rdx
   685a0:	c4 e2 a8 f2 c1       	andn   %rcx,%r10,%rax
   685a5:	48 89 54 24 88       	mov    %rdx,-0x78(%rsp)
   685aa:	c4 42 c8 f2 e8       	andn   %r8,%rsi,%r13
   685af:	4c 31 c8             	xor    %r9,%rax
   685b2:	c4 63 fb f0 4c 24 20 	rorx   $0x8,0x20(%rsp),%r9
   685b9:	08 
   685ba:	c4 63 fb f0 44 24 f0 	rorx   $0x31,-0x10(%rsp),%r8
   685c1:	31 
   685c2:	48 31 f7             	xor    %rsi,%rdi
   685c5:	c4 e2 f0 f2 f6       	andn   %rsi,%rcx,%rsi
   685ca:	49 31 cd             	xor    %rcx,%r13
   685cd:	c4 e3 fb f0 4c 24 f8 	rorx   $0x1c,-0x8(%rsp),%rcx
   685d4:	1c 
   685d5:	4c 31 d6             	xor    %r10,%rsi
   685d8:	48 89 7c 24 80       	mov    %rdi,-0x80(%rsp)
   685dd:	48 89 44 24 e8       	mov    %rax,-0x18(%rsp)
   685e2:	48 89 74 24 b8       	mov    %rsi,-0x48(%rsp)
   685e7:	c4 c3 fb f0 f6 36    	rorx   $0x36,%r14,%rsi
   685ed:	4c 89 6c 24 30       	mov    %r13,0x30(%rsp)
   685f2:	c4 c2 b8 f2 f9       	andn   %r9,%r8,%rdi
   685f7:	c4 42 c8 f2 d0       	andn   %r8,%rsi,%r10
   685fc:	c4 e2 b0 f2 d3       	andn   %rbx,%r9,%rdx
   68601:	48 31 f7             	xor    %rsi,%rdi
   68604:	c4 e2 f0 f2 f6       	andn   %rsi,%rcx,%rsi
   68609:	c4 e2 e0 f2 c1       	andn   %rcx,%rbx,%rax
   6860e:	49 31 ca             	xor    %rcx,%r10
   68611:	c4 e3 fb f0 4c 24 98 	rorx   $0x9,-0x68(%rsp),%rcx
   68618:	09 
   68619:	4c 31 c2             	xor    %r8,%rdx
   6861c:	c4 63 fb f0 44 24 a0 	rorx   $0x3e,-0x60(%rsp),%r8
   68623:	3e 
   68624:	49 89 fe             	mov    %rdi,%r14
   68627:	48 89 7c 24 20       	mov    %rdi,0x20(%rsp)
   6862c:	c4 e3 fb f0 7c 24 b0 	rorx   $0x17,-0x50(%rsp),%rdi
   68633:	17 
   68634:	4c 31 c8             	xor    %r9,%rax
   68637:	48 31 de             	xor    %rbx,%rsi
   6863a:	48 89 54 24 c0       	mov    %rdx,-0x40(%rsp)
   6863f:	4c 89 54 24 f8       	mov    %r10,-0x8(%rsp)
   68644:	48 89 44 24 90       	mov    %rax,-0x70(%rsp)
   68649:	c4 e3 fb f0 44 24 a8 	rorx   $0x2,-0x58(%rsp),%rax
   68650:	02 
   68651:	48 89 74 24 f0       	mov    %rsi,-0x10(%rsp)
   68656:	c4 c3 fb f0 f3 19    	rorx   $0x19,%r11,%rsi
   6865c:	c4 62 f0 f2 ce       	andn   %rsi,%rcx,%r9
   68661:	c4 e2 c8 f2 d7       	andn   %rdi,%rsi,%rdx
   68666:	48 31 ca             	xor    %rcx,%rdx
   68669:	c4 62 b8 f2 e0       	andn   %rax,%r8,%r12
   6866e:	49 31 c1             	xor    %rax,%r9
   68671:	48 89 54 24 b0       	mov    %rdx,-0x50(%rsp)
   68676:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
   6867b:	49 31 fc             	xor    %rdi,%r12
   6867e:	c4 c2 c0 f2 f8       	andn   %r8,%rdi,%rdi
   68683:	4c 89 4c 24 a0       	mov    %r9,-0x60(%rsp)
   68688:	48 31 f7             	xor    %rsi,%rdi
   6868b:	48 8d 35 f6 0e fc ff 	lea    -0x3f10a(%rip),%rsi        # 29588 <anon.360b797b0f742dc484d3b12692d3ab1f.14.llvm.13132551382588709452+0x138>
   68692:	4d 89 e3             	mov    %r12,%r11
   68695:	4c 33 5c 24 08       	xor    0x8(%rsp),%r11
   6869a:	48 89 fb             	mov    %rdi,%rbx
   6869d:	48 89 7c 24 a8       	mov    %rdi,-0x58(%rsp)
   686a2:	c4 e2 f8 f2 f9       	andn   %rcx,%rax,%rdi
   686a7:	48 8b 44 24 38       	mov    0x38(%rsp),%rax
   686ac:	4c 89 d1             	mov    %r10,%rcx
   686af:	48 33 0c 24          	xor    (%rsp),%rcx
   686b3:	4c 8b 54 24 18       	mov    0x18(%rsp),%r10
   686b8:	4c 31 c7             	xor    %r8,%rdi
   686bb:	48 89 7c 24 98       	mov    %rdi,-0x68(%rsp)
   686c0:	48 33 44 32 f8       	xor    -0x8(%rdx,%rsi,1),%rax
   686c5:	48 8b 74 24 88       	mov    -0x78(%rsp),%rsi
   686ca:	48 8b 54 24 d8       	mov    -0x28(%rsp),%rdx
   686cf:	49 31 fa             	xor    %rdi,%r10
   686d2:	48 31 e8             	xor    %rbp,%rax
   686d5:	4c 31 f2             	xor    %r14,%rdx
   686d8:	4c 8b 74 24 b8       	mov    -0x48(%rsp),%r14
   686dd:	48 8b 6c 24 90       	mov    -0x70(%rsp),%rbp
   686e2:	49 89 c0             	mov    %rax,%r8
   686e5:	48 89 44 24 38       	mov    %rax,0x38(%rsp)
   686ea:	4c 89 f8             	mov    %r15,%rax
   686ed:	4c 31 e8             	xor    %r13,%rax
   686f0:	4c 8b 7c 24 f0       	mov    -0x10(%rsp),%r15
   686f5:	4c 8b 6c 24 b0       	mov    -0x50(%rsp),%r13
   686fa:	48 31 c8             	xor    %rcx,%rax
   686fd:	48 89 d9             	mov    %rbx,%rcx
   68700:	48 8b 5c 24 c8       	mov    -0x38(%rsp),%rbx
   68705:	48 33 4c 24 10       	xor    0x10(%rsp),%rcx
   6870a:	4d 31 f7             	xor    %r14,%r15
   6870d:	4c 33 7c 24 e0       	xor    -0x20(%rsp),%r15
   68712:	4c 31 e8             	xor    %r13,%rax
   68715:	48 31 de             	xor    %rbx,%rsi
   68718:	48 31 d1             	xor    %rdx,%rcx
   6871b:	48 8b 54 24 80       	mov    -0x80(%rsp),%rdx
   68720:	4c 31 de             	xor    %r11,%rsi
   68723:	4c 8b 5c 24 e8       	mov    -0x18(%rsp),%r11
   68728:	4c 33 5c 24 d0       	xor    -0x30(%rsp),%r11
   6872d:	4d 31 cf             	xor    %r9,%r15
   68730:	c4 63 fb f0 c8 3f    	rorx   $0x3f,%rax,%r9
   68736:	4d 31 c7             	xor    %r8,%r15
   68739:	48 31 d1             	xor    %rdx,%rcx
   6873c:	4d 31 d3             	xor    %r10,%r11
   6873f:	4c 8b 54 24 c0       	mov    -0x40(%rsp),%r10
   68744:	49 31 eb             	xor    %rbp,%r11
   68747:	c4 c3 fb f0 fb 3f    	rorx   $0x3f,%r11,%rdi
   6874d:	4d 31 d9             	xor    %r11,%r9
   68750:	4c 8b 5c 24 20       	mov    0x20(%rsp),%r11
   68755:	48 31 cf             	xor    %rcx,%rdi
   68758:	c4 e3 fb f0 c9 3f    	rorx   $0x3f,%rcx,%rcx
   6875e:	4d 31 ce             	xor    %r9,%r14
   68761:	4c 31 4c 24 38       	xor    %r9,0x38(%rsp)
   68766:	4c 31 4c 24 a0       	xor    %r9,-0x60(%rsp)
   6876b:	48 31 fb             	xor    %rdi,%rbx
   6876e:	4c 31 f9             	xor    %r15,%rcx
   68771:	49 31 fc             	xor    %rdi,%r12
   68774:	48 31 7c 24 88       	xor    %rdi,-0x78(%rsp)
   68779:	4c 89 74 24 b8       	mov    %r14,-0x48(%rsp)
   6877e:	4c 31 d6             	xor    %r10,%rsi
   68781:	49 31 fa             	xor    %rdi,%r10
   68784:	48 89 5c 24 c8       	mov    %rbx,-0x38(%rsp)
   68789:	48 8b 5c 24 18       	mov    0x18(%rsp),%rbx
   6878e:	48 31 4c 24 28       	xor    %rcx,0x28(%rsp)
   68793:	48 31 4c 24 f8       	xor    %rcx,-0x8(%rsp)
   68798:	48 33 7c 24 08       	xor    0x8(%rsp),%rdi
   6879d:	c4 63 fb f0 c6 3f    	rorx   $0x3f,%rsi,%r8
   687a3:	4c 89 54 24 c0       	mov    %r10,-0x40(%rsp)
   687a8:	4c 8b 54 24 30       	mov    0x30(%rsp),%r10
   687ad:	49 31 c0             	xor    %rax,%r8
   687b0:	c4 c3 fb f0 c7 3f    	rorx   $0x3f,%r15,%rax
   687b6:	48 31 f0             	xor    %rsi,%rax
   687b9:	48 8b 74 24 e0       	mov    -0x20(%rsp),%rsi
   687be:	4c 31 c2             	xor    %r8,%rdx
   687c1:	4d 31 c3             	xor    %r8,%r11
   687c4:	4c 31 44 24 a8       	xor    %r8,-0x58(%rsp)
   687c9:	4c 31 44 24 d8       	xor    %r8,-0x28(%rsp)
   687ce:	4c 33 44 24 10       	xor    0x10(%rsp),%r8
   687d3:	48 31 c5             	xor    %rax,%rbp
   687d6:	c4 43 fb f0 f3 31    	rorx   $0x31,%r11,%r14
   687dc:	48 31 44 24 98       	xor    %rax,-0x68(%rsp)
   687e1:	48 31 44 24 d0       	xor    %rax,-0x30(%rsp)
   687e6:	48 89 54 24 80       	mov    %rdx,-0x80(%rsp)
   687eb:	48 89 6c 24 90       	mov    %rbp,-0x70(%rsp)
   687f0:	48 8b 2c 24          	mov    (%rsp),%rbp
   687f4:	48 31 c3             	xor    %rax,%rbx
   687f7:	48 33 44 24 e8       	xor    -0x18(%rsp),%rax
   687fc:	c4 e3 fb f0 d7 09    	rorx   $0x9,%rdi,%rdx
   68802:	49 31 ca             	xor    %rcx,%r10
   68805:	c4 e3 fb f0 db 25    	rorx   $0x25,%rbx,%rbx
   6880b:	c4 43 fb f0 d2 36    	rorx   $0x36,%r10,%r10
   68811:	4c 31 ce             	xor    %r9,%rsi
   68814:	4c 33 4c 24 f0       	xor    -0x10(%rsp),%r9
   68819:	c4 42 a8 f2 fe       	andn   %r14,%r10,%r15
   6881e:	c4 e3 fb f0 f6 1c    	rorx   $0x1c,%rsi,%rsi
   68824:	48 31 cd             	xor    %rcx,%rbp
   68827:	4c 31 e9             	xor    %r13,%rcx
   6882a:	c4 43 fb f0 ec 08    	rorx   $0x8,%r12,%r13
   68830:	49 31 f7             	xor    %rsi,%r15
   68833:	c4 e3 fb f0 c0 19    	rorx   $0x19,%rax,%rax
   68839:	c4 62 90 f2 db       	andn   %rbx,%r13,%r11
   6883e:	c4 e3 fb f0 c9 3e    	rorx   $0x3e,%rcx,%rcx
   68844:	4c 89 7c 24 18       	mov    %r15,0x18(%rsp)
   68849:	c4 63 fb f0 7c 24 a8 	rorx   $0x3,-0x58(%rsp),%r15
   68850:	03 
   68851:	4d 31 f3             	xor    %r14,%r11
   68854:	c4 42 88 f2 f5       	andn   %r13,%r14,%r14
   68859:	4d 31 d6             	xor    %r10,%r14
   6885c:	c4 42 c8 f2 d2       	andn   %r10,%rsi,%r10
   68861:	c4 e2 e0 f2 f6       	andn   %rsi,%rbx,%rsi
   68866:	c4 c3 fb f0 f9 17    	rorx   $0x17,%r9,%rdi
   6886c:	4c 89 5c 24 08       	mov    %r11,0x8(%rsp)
   68871:	4c 8b 5c 24 38       	mov    0x38(%rsp),%r11
   68876:	c4 63 fb f0 4c 24 c8 	rorx   $0x24,-0x38(%rsp),%r9
   6887d:	24 
   6887e:	4c 31 ee             	xor    %r13,%rsi
   68881:	4c 89 74 24 f0       	mov    %r14,-0x10(%rsp)
   68886:	c4 63 fb f0 74 24 28 	rorx   $0x14,0x28(%rsp),%r14
   6888d:	14 
   6888e:	49 31 da             	xor    %rbx,%r10
   68891:	c4 63 fb f0 ed 3f    	rorx   $0x3f,%rbp,%r13
   68897:	c4 e3 fb f0 6c 24 90 	rorx   $0x38,-0x70(%rsp),%rbp
   6889e:	38 
   6889f:	c4 e3 fb f0 5c 24 88 	rorx   $0x27,-0x78(%rsp),%rbx
   688a6:	27 
   688a7:	48 89 74 24 e8       	mov    %rsi,-0x18(%rsp)
   688ac:	c4 c3 fb f0 f0 02    	rorx   $0x2,%r8,%rsi
   688b2:	4c 89 14 24          	mov    %r10,(%rsp)
   688b6:	c4 63 fb f0 54 24 d0 	rorx   $0x2c,-0x30(%rsp),%r10
   688bd:	2c 
   688be:	c4 62 f0 f2 c6       	andn   %rsi,%rcx,%r8
   688c3:	4c 89 7c 24 60       	mov    %r15,0x60(%rsp)
   688c8:	49 31 f8             	xor    %rdi,%r8
   688cb:	4c 89 44 24 20       	mov    %r8,0x20(%rsp)
   688d0:	c4 62 f8 f2 c7       	andn   %rdi,%rax,%r8
   688d5:	c4 e2 c0 f2 f9       	andn   %rcx,%rdi,%rdi
   688da:	48 31 c7             	xor    %rax,%rdi
   688dd:	c4 e2 e8 f2 c0       	andn   %rax,%rdx,%rax
   688e2:	49 31 d0             	xor    %rdx,%r8
   688e5:	c4 42 a0 f2 e6       	andn   %r14,%r11,%r12
   688ea:	48 89 5c 24 78       	mov    %rbx,0x78(%rsp)
   688ef:	c4 e3 fb f0 5c 24 a0 	rorx   $0x2e,-0x60(%rsp),%rbx
   688f6:	2e 
   688f7:	48 89 6c 24 70       	mov    %rbp,0x70(%rsp)
   688fc:	48 89 7c 24 e0       	mov    %rdi,-0x20(%rsp)
   68901:	48 31 f0             	xor    %rsi,%rax
   68904:	c4 e3 fb f0 7c 24 98 	rorx   $0x32,-0x68(%rsp),%rdi
   6890b:	32 
   6890c:	4c 89 44 24 30       	mov    %r8,0x30(%rsp)
   68911:	c4 63 fb f0 44 24 f8 	rorx   $0x13,-0x8(%rsp),%r8
   68918:	13 
   68919:	48 89 44 24 10       	mov    %rax,0x10(%rsp)
   6891e:	c4 e2 c8 f2 c2       	andn   %rdx,%rsi,%rax
   68923:	c4 e3 fb f0 74 24 c0 	rorx   $0x2b,-0x40(%rsp),%rsi
   6892a:	2b 
   6892b:	c4 e3 fb f0 54 24 80 	rorx   $0x15,-0x80(%rsp),%rdx
   68932:	15 
   68933:	48 31 c8             	xor    %rcx,%rax
   68936:	c4 e3 fb f0 4c 24 b8 	rorx   $0x3d,-0x48(%rsp),%rcx
   6893d:	3d 
   6893e:	48 89 44 24 b0       	mov    %rax,-0x50(%rsp)
   68943:	c4 e3 fb f0 44 24 d8 	rorx   $0x3a,-0x28(%rsp),%rax
   6894a:	3a 
   6894b:	49 31 fc             	xor    %rdi,%r12
   6894e:	4c 89 64 24 90       	mov    %r12,-0x70(%rsp)
   68953:	c4 62 c8 f2 e7       	andn   %rdi,%rsi,%r12
   68958:	c4 c2 c0 f2 fb       	andn   %r11,%rdi,%rdi
   6895d:	48 31 f7             	xor    %rsi,%rdi
   68960:	c4 e2 e8 f2 f6       	andn   %rsi,%rdx,%rsi
   68965:	49 31 d4             	xor    %rdx,%r12
   68968:	c4 e2 88 f2 d2       	andn   %rdx,%r14,%rdx
   6896d:	48 89 54 24 f8       	mov    %rdx,-0x8(%rsp)
   68972:	c4 c2 f0 f2 d0       	andn   %r8,%rcx,%rdx
   68977:	4c 89 64 24 98       	mov    %r12,-0x68(%rsp)
   6897c:	c4 42 b8 f2 e7       	andn   %r15,%r8,%r12
   68981:	48 89 44 24 68       	mov    %rax,0x68(%rsp)
   68986:	4c 31 f6             	xor    %r14,%rsi
   68989:	4c 31 d2             	xor    %r10,%rdx
   6898c:	49 31 cc             	xor    %rcx,%r12
   6898f:	48 89 74 24 d0       	mov    %rsi,-0x30(%rsp)
   68994:	c4 e2 a8 f2 f1       	andn   %rcx,%r10,%rsi
   68999:	48 89 54 24 c8       	mov    %rdx,-0x38(%rsp)
   6899e:	c4 c2 80 f2 d1       	andn   %r9,%r15,%rdx
   689a3:	4c 89 64 24 88       	mov    %r12,-0x78(%rsp)
   689a8:	4c 31 ce             	xor    %r9,%rsi
   689ab:	4c 31 c2             	xor    %r8,%rdx
   689ae:	48 89 54 24 b8       	mov    %rdx,-0x48(%rsp)
   689b3:	c4 c2 b0 f2 d2       	andn   %r10,%r9,%rdx
   689b8:	4c 31 fa             	xor    %r15,%rdx
   689bb:	4c 8b 7c 24 78       	mov    0x78(%rsp),%r15
   689c0:	48 89 54 24 28       	mov    %rdx,0x28(%rsp)
   689c5:	c4 62 80 f2 e5       	andn   %rbp,%r15,%r12
   689ca:	c4 c2 f8 f2 d7       	andn   %r15,%rax,%rdx
   689cf:	49 31 c4             	xor    %rax,%r12
   689d2:	4c 31 ea             	xor    %r13,%rdx
   689d5:	4c 89 64 24 d8       	mov    %r12,-0x28(%rsp)
   689da:	c4 62 90 f2 e0       	andn   %rax,%r13,%r12
   689df:	c4 c2 e0 f2 c5       	andn   %r13,%rbx,%rax
   689e4:	48 89 54 24 c0       	mov    %rdx,-0x40(%rsp)
   689e9:	c4 e2 d0 f2 d3       	andn   %rbx,%rbp,%rdx
   689ee:	48 31 e8             	xor    %rbp,%rax
   689f1:	49 31 dc             	xor    %rbx,%r12
   689f4:	48 8b 6c 24 f8       	mov    -0x8(%rsp),%rbp
   689f9:	4c 31 fa             	xor    %r15,%rdx
   689fc:	4c 89 db             	mov    %r11,%rbx
   689ff:	48 89 44 24 80       	mov    %rax,-0x80(%rsp)
   68a04:	48 8b 44 24 40       	mov    0x40(%rsp),%rax
   68a09:	4c 89 64 24 a0       	mov    %r12,-0x60(%rsp)
   68a0e:	4c 8d 25 73 0b fc ff 	lea    -0x3f48d(%rip),%r12        # 29588 <anon.360b797b0f742dc484d3b12692d3ab1f.14.llvm.13132551382588709452+0x138>
   68a15:	48 89 54 24 a8       	mov    %rdx,-0x58(%rsp)
   68a1a:	4a 33 2c 20          	xor    (%rax,%r12,1),%rbp
   68a1e:	48 83 c0 10          	add    $0x10,%rax
   68a22:	49 89 fc             	mov    %rdi,%r12
   68a25:	48 89 44 24 40       	mov    %rax,0x40(%rsp)
   68a2a:	48 31 dd             	xor    %rbx,%rbp
   68a2d:	48 3d c8 00 00 00    	cmp    $0xc8,%rax
   68a33:	0f 85 f7 f8 ff ff    	jne    68330 <purrdf_hash::sha3::keccak_f1600+0x110>
   68a39:	48 8b 54 24 58       	mov    0x58(%rsp),%rdx
   68a3e:	4c 8b 5c 24 d0       	mov    -0x30(%rsp),%r11
   68a43:	48 8b 44 24 98       	mov    -0x68(%rsp),%rax
   68a48:	48 89 9c 24 d0 00 00 	mov    %rbx,0xd0(%rsp)
   68a4f:	00 
   68a50:	4c 89 b4 24 d8 00 00 	mov    %r14,0xd8(%rsp)
   68a57:	00 
   68a58:	4c 89 8c 24 f8 00 00 	mov    %r9,0xf8(%rsp)
   68a5f:	00 
   68a60:	4c 89 94 24 00 01 00 	mov    %r10,0x100(%rsp)
   68a67:	00 
   68a68:	48 89 8c 24 08 01 00 	mov    %rcx,0x108(%rsp)
   68a6f:	00 
   68a70:	48 8b 4c 24 e0       	mov    -0x20(%rsp),%rcx
   68a75:	4c 89 84 24 10 01 00 	mov    %r8,0x110(%rsp)
   68a7c:	00 
   68a7d:	48 89 f7             	mov    %rsi,%rdi
   68a80:	48 8b 74 24 10       	mov    0x10(%rsp),%rsi
   68a85:	c5 f8 57 c0          	vxorps %xmm0,%xmm0,%xmm0
   68a89:	4c 89 5a 08          	mov    %r11,0x8(%rdx)
   68a8d:	4c 8b 5c 24 90       	mov    -0x70(%rsp),%r11
   68a92:	48 89 42 10          	mov    %rax,0x10(%rdx)
   68a96:	4c 89 62 18          	mov    %r12,0x18(%rdx)
   68a9a:	48 8b 44 24 c0       	mov    -0x40(%rsp),%rax
   68a9f:	4c 89 5a 20          	mov    %r11,0x20(%rdx)
   68aa3:	4c 8b 5c 24 08       	mov    0x8(%rsp),%r11
   68aa8:	4c 89 9a 90 00 00 00 	mov    %r11,0x90(%rdx)
   68aaf:	4c 8b 5c 24 b0       	mov    -0x50(%rsp),%r11
   68ab4:	4c 89 9a c0 00 00 00 	mov    %r11,0xc0(%rdx)
   68abb:	48 89 42 50          	mov    %rax,0x50(%rdx)
   68abf:	48 8b 44 24 60       	mov    0x60(%rsp),%rax
   68ac4:	48 89 8a b0 00 00 00 	mov    %rcx,0xb0(%rdx)
   68acb:	48 8b 4c 24 70       	mov    0x70(%rsp),%rcx
   68ad0:	48 89 84 24 18 01 00 	mov    %rax,0x118(%rsp)
   68ad7:	00 
   68ad8:	48 8b 44 24 68       	mov    0x68(%rsp),%rax
   68add:	4c 89 ac 24 20 01 00 	mov    %r13,0x120(%rsp)
   68ae4:	00 
   68ae5:	48 89 84 24 28 01 00 	mov    %rax,0x128(%rsp)
   68aec:	00 
   68aed:	4c 89 bc 24 30 01 00 	mov    %r15,0x130(%rsp)
   68af4:	00 
   68af5:	48 89 8c 24 38 01 00 	mov    %rcx,0x138(%rsp)
   68afc:	00 
   68afd:	48 8b 44 24 e8       	mov    -0x18(%rsp),%rax
   68b02:	48 8b 4c 24 d8       	mov    -0x28(%rsp),%rcx
   68b07:	62 f1 7c 48 11 84 24 	vmovups %zmm0,0x158(%rsp)
   68b0e:	58 01 00 00 
   68b12:	62 f1 7c 48 11 44 24 	vmovups %zmm0,0x140(%rsp)
   68b19:	05 
   68b1a:	62 f1 7c 48 11 44 24 	vmovups %zmm0,0x100(%rsp)
   68b21:	04 
   68b22:	62 f1 7c 48 11 44 24 	vmovups %zmm0,0xc0(%rsp)
   68b29:	03 
   68b2a:	62 f1 7c 48 11 44 24 	vmovups %zmm0,0x80(%rsp)
   68b31:	02 
   68b32:	48 89 82 98 00 00 00 	mov    %rax,0x98(%rdx)
   68b39:	48 89 b2 a0 00 00 00 	mov    %rsi,0xa0(%rdx)
   68b40:	48 89 7a 28          	mov    %rdi,0x28(%rdx)
   68b44:	48 89 4a 58          	mov    %rcx,0x58(%rdx)
   68b48:	48 8b 74 24 f0       	mov    -0x10(%rsp),%rsi
   68b4d:	48 8b 4c 24 20       	mov    0x20(%rsp),%rcx
   68b52:	48 8d 44 24 48       	lea    0x48(%rsp),%rax
   68b57:	48 89 b2 88 00 00 00 	mov    %rsi,0x88(%rdx)
   68b5e:	48 89 8a b8 00 00 00 	mov    %rcx,0xb8(%rdx)
   68b65:	48 8b 74 24 b8       	mov    -0x48(%rsp),%rsi
   68b6a:	48 8b 4c 24 a0       	mov    -0x60(%rsp),%rcx
   68b6f:	48 89 72 40          	mov    %rsi,0x40(%rdx)
   68b73:	48 89 4a 70          	mov    %rcx,0x70(%rdx)
   68b77:	48 8b 34 24          	mov    (%rsp),%rsi
   68b7b:	48 8b 4c 24 30       	mov    0x30(%rsp),%rcx
   68b80:	48 89 72 78          	mov    %rsi,0x78(%rdx)
   68b84:	48 89 8a a8 00 00 00 	mov    %rcx,0xa8(%rdx)
   68b8b:	48 8b 74 24 c8       	mov    -0x38(%rsp),%rsi
   68b90:	48 8b 4c 24 88       	mov    -0x78(%rsp),%rcx
   68b95:	48 89 2a             	mov    %rbp,(%rdx)
   68b98:	48 89 72 30          	mov    %rsi,0x30(%rdx)
   68b9c:	48 89 4a 38          	mov    %rcx,0x38(%rdx)
   68ba0:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   68ba5:	48 8b 4c 24 a8       	mov    -0x58(%rsp),%rcx
   68baa:	48 89 72 48          	mov    %rsi,0x48(%rdx)
   68bae:	48 89 4a 60          	mov    %rcx,0x60(%rdx)
   68bb2:	48 8b 74 24 80       	mov    -0x80(%rsp),%rsi
   68bb7:	48 8b 4c 24 18       	mov    0x18(%rsp),%rcx
   68bbc:	48 89 72 68          	mov    %rsi,0x68(%rdx)
   68bc0:	48 89 8a 80 00 00 00 	mov    %rcx,0x80(%rdx)
   68bc7:	48 8d 94 24 80 00 00 	lea    0x80(%rsp),%rdx
   68bce:	00 
   68bcf:	48 8d 4c 24 48       	lea    0x48(%rsp),%rcx
   68bd4:	48 89 54 24 48       	mov    %rdx,0x48(%rsp)
   68bd9:	48 c7 44 24 50 23 00 	movq   $0x23,0x50(%rsp)
   68be0:	00 00 
   68be2:	62 f1 7c 48 11 84 24 	vmovups %zmm0,0x158(%rsp)
   68be9:	58 01 00 00 
   68bed:	48 89 54 24 48       	mov    %rdx,0x48(%rsp)
   68bf2:	62 f1 7c 48 11 44 24 	vmovups %zmm0,0x140(%rsp)
   68bf9:	05 
   68bfa:	62 f1 7c 48 11 44 24 	vmovups %zmm0,0x100(%rsp)
   68c01:	04 
   68c02:	62 f1 7c 48 11 44 24 	vmovups %zmm0,0xc0(%rsp)
   68c09:	03 
   68c0a:	62 f1 7c 48 11 44 24 	vmovups %zmm0,0x80(%rsp)
   68c11:	02 
   68c12:	48 c7 44 24 50 23 00 	movq   $0x23,0x50(%rsp)
   68c19:	00 00 
   68c1b:	48 81 c4 98 01 00 00 	add    $0x198,%rsp
   68c22:	5b                   	pop    %rbx
   68c23:	41 5c                	pop    %r12
   68c25:	41 5d                	pop    %r13
   68c27:	41 5e                	pop    %r14
   68c29:	41 5f                	pop    %r15
   68c2b:	5d                   	pop    %rbp
   68c2c:	c5 f8 77             	vzeroupper
   68c2f:	c3                   	ret

Disassembly of section .init:

Disassembly of section .fini:

Disassembly of section .plt:
