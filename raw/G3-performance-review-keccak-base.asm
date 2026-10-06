
/opt/.cargo/slots/1332a15ff3f90e54/0/target/debug/g3-throughput-base:     file format elf64-x86-64


Disassembly of section .text:

0000000000067080 <purrdf_hash::sha3::keccak_f1600>:
   67080:	55                   	push   %rbp
   67081:	41 57                	push   %r15
   67083:	41 56                	push   %r14
   67085:	41 55                	push   %r13
   67087:	41 54                	push   %r12
   67089:	53                   	push   %rbx
   6708a:	48 83 ec 58          	sub    $0x58,%rsp
   6708e:	48 8b 57 50          	mov    0x50(%rdi),%rdx
   67092:	48 8b 4f 08          	mov    0x8(%rdi),%rcx
   67096:	48 8b 07             	mov    (%rdi),%rax
   67099:	4c 8b 87 a0 00 00 00 	mov    0xa0(%rdi),%r8
   670a0:	48 8b 9f 80 00 00 00 	mov    0x80(%rdi),%rbx
   670a7:	4c 8b a7 a8 00 00 00 	mov    0xa8(%rdi),%r12
   670ae:	48 8b 77 60          	mov    0x60(%rdi),%rsi
   670b2:	4c 8b 97 b0 00 00 00 	mov    0xb0(%rdi),%r10
   670b9:	4c 8b 8f 90 00 00 00 	mov    0x90(%rdi),%r9
   670c0:	4c 8b bf b8 00 00 00 	mov    0xb8(%rdi),%r15
   670c7:	48 8b af c0 00 00 00 	mov    0xc0(%rdi),%rbp
   670ce:	48 89 7c 24 38       	mov    %rdi,0x38(%rsp)
   670d3:	48 89 54 24 b0       	mov    %rdx,-0x50(%rsp)
   670d8:	48 8b 57 30          	mov    0x30(%rdi),%rdx
   670dc:	48 89 4c 24 a0       	mov    %rcx,-0x60(%rsp)
   670e1:	48 8b 4f 78          	mov    0x78(%rdi),%rcx
   670e5:	48 89 44 24 80       	mov    %rax,-0x80(%rsp)
   670ea:	48 8b 87 98 00 00 00 	mov    0x98(%rdi),%rax
   670f1:	48 89 54 24 f8       	mov    %rdx,-0x8(%rsp)
   670f6:	48 8b 57 38          	mov    0x38(%rdi),%rdx
   670fa:	48 89 4c 24 a8       	mov    %rcx,-0x58(%rsp)
   670ff:	48 8b 4f 58          	mov    0x58(%rdi),%rcx
   67103:	48 89 54 24 c8       	mov    %rdx,-0x38(%rsp)
   67108:	48 8b 57 40          	mov    0x40(%rdi),%rdx
   6710c:	48 89 4c 24 98       	mov    %rcx,-0x68(%rsp)
   67111:	48 8b 8f 88 00 00 00 	mov    0x88(%rdi),%rcx
   67118:	48 89 54 24 e8       	mov    %rdx,-0x18(%rsp)
   6711d:	48 8b 57 48          	mov    0x48(%rdi),%rdx
   67121:	48 89 4c 24 b8       	mov    %rcx,-0x48(%rsp)
   67126:	48 8b 4f 68          	mov    0x68(%rdi),%rcx
   6712a:	48 89 54 24 88       	mov    %rdx,-0x78(%rsp)
   6712f:	48 8b 57 28          	mov    0x28(%rdi),%rdx
   67133:	48 89 4c 24 90       	mov    %rcx,-0x70(%rsp)
   67138:	48 8b 4f 70          	mov    0x70(%rdi),%rcx
   6713c:	48 89 54 24 d0       	mov    %rdx,-0x30(%rsp)
   67141:	48 8b 57 10          	mov    0x10(%rdi),%rdx
   67145:	48 89 4c 24 e0       	mov    %rcx,-0x20(%rsp)
   6714a:	48 8b 4f 20          	mov    0x20(%rdi),%rcx
   6714e:	48 89 54 24 f0       	mov    %rdx,-0x10(%rsp)
   67153:	48 8b 57 18          	mov    0x18(%rdi),%rdx
   67157:	48 89 4c 24 d8       	mov    %rcx,-0x28(%rsp)
   6715c:	b9 08 00 00 00       	mov    $0x8,%ecx
   67161:	48 89 4c 24 20       	mov    %rcx,0x20(%rsp)
   67166:	48 89 54 24 c0       	mov    %rdx,-0x40(%rsp)
   6716b:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
   67170:	48 8b 54 24 d0       	mov    -0x30(%rsp),%rdx
   67175:	4c 89 c1             	mov    %r8,%rcx
   67178:	48 33 4c 24 a8       	xor    -0x58(%rsp),%rcx
   6717d:	4c 8b 74 24 98       	mov    -0x68(%rsp),%r14
   67182:	4c 89 64 24 10       	mov    %r12,0x10(%rsp)
   67187:	48 8b 7c 24 d8       	mov    -0x28(%rsp),%rdi
   6718c:	4d 89 fd             	mov    %r15,%r13
   6718f:	4c 33 6c 24 e8       	xor    -0x18(%rsp),%r13
   67194:	49 89 eb             	mov    %rbp,%r11
   67197:	49 31 c3             	xor    %rax,%r11
   6719a:	48 33 54 24 b0       	xor    -0x50(%rsp),%rdx
   6719f:	48 33 7c 24 e0       	xor    -0x20(%rsp),%rdi
   671a4:	4d 31 e6             	xor    %r12,%r14
   671a7:	4c 8b 64 24 f0       	mov    -0x10(%rsp),%r12
   671ac:	48 31 ca             	xor    %rcx,%rdx
   671af:	48 8b 4c 24 a0       	mov    -0x60(%rsp),%rcx
   671b4:	4c 33 64 24 b8       	xor    -0x48(%rsp),%r12
   671b9:	49 31 fb             	xor    %rdi,%r11
   671bc:	4c 33 5c 24 88       	xor    -0x78(%rsp),%r11
   671c1:	48 33 54 24 80       	xor    -0x80(%rsp),%rdx
   671c6:	48 31 d9             	xor    %rbx,%rcx
   671c9:	4c 31 f1             	xor    %r14,%rcx
   671cc:	4d 89 d6             	mov    %r10,%r14
   671cf:	49 31 f6             	xor    %rsi,%r14
   671d2:	48 33 4c 24 f8       	xor    -0x8(%rsp),%rcx
   671d7:	4d 31 e6             	xor    %r12,%r14
   671da:	4c 8b 64 24 c0       	mov    -0x40(%rsp),%r12
   671df:	4c 33 74 24 c8       	xor    -0x38(%rsp),%r14
   671e4:	4d 31 cc             	xor    %r9,%r12
   671e7:	4d 31 ec             	xor    %r13,%r12
   671ea:	4c 33 64 24 90       	xor    -0x70(%rsp),%r12
   671ef:	c4 43 fb f0 eb 3f    	rorx   $0x3f,%r11,%r13
   671f5:	4d 31 f5             	xor    %r14,%r13
   671f8:	c4 43 fb f0 f6 3f    	rorx   $0x3f,%r14,%r14
   671fe:	49 31 d6             	xor    %rdx,%r14
   67201:	c4 e3 fb f0 d2 3f    	rorx   $0x3f,%rdx,%rdx
   67207:	4d 31 e9             	xor    %r13,%r9
   6720a:	4c 31 6c 24 c0       	xor    %r13,-0x40(%rsp)
   6720f:	4c 31 6c 24 90       	xor    %r13,-0x70(%rsp)
   67214:	4d 31 ef             	xor    %r13,%r15
   67217:	4c 33 6c 24 e8       	xor    -0x18(%rsp),%r13
   6721c:	c4 43 fb f0 c9 2b    	rorx   $0x2b,%r9,%r9
   67222:	4c 31 f3             	xor    %r14,%rbx
   67225:	4c 31 74 24 a0       	xor    %r14,-0x60(%rsp)
   6722a:	4c 31 74 24 98       	xor    %r14,-0x68(%rsp)
   6722f:	c4 e3 fb f0 db 13    	rorx   $0x13,%rbx,%rbx
   67235:	4c 31 e2             	xor    %r12,%rdx
   67238:	c4 c3 fb f0 fc 3f    	rorx   $0x3f,%r12,%rdi
   6723e:	48 31 cf             	xor    %rcx,%rdi
   67241:	48 31 d0             	xor    %rdx,%rax
   67244:	c4 e3 fb f0 c9 3f    	rorx   $0x3f,%rcx,%rcx
   6724a:	48 31 d5             	xor    %rdx,%rbp
   6724d:	48 31 54 24 88       	xor    %rdx,-0x78(%rsp)
   67252:	48 31 54 24 d8       	xor    %rdx,-0x28(%rsp)
   67257:	48 33 54 24 e0       	xor    -0x20(%rsp),%rdx
   6725c:	48 31 fe             	xor    %rdi,%rsi
   6725f:	49 31 fa             	xor    %rdi,%r10
   67262:	48 89 04 24          	mov    %rax,(%rsp)
   67266:	48 31 7c 24 c8       	xor    %rdi,-0x38(%rsp)
   6726b:	48 31 7c 24 b8       	xor    %rdi,-0x48(%rsp)
   67270:	48 33 7c 24 f0       	xor    -0x10(%rsp),%rdi
   67275:	48 8b 44 24 80       	mov    -0x80(%rsp),%rax
   6727a:	4c 31 d9             	xor    %r11,%rcx
   6727d:	4c 8b 5c 24 f8       	mov    -0x8(%rsp),%r11
   67282:	c4 63 fb f0 e5 32    	rorx   $0x32,%rbp,%r12
   67288:	c4 e3 fb f0 f6 15    	rorx   $0x15,%rsi,%rsi
   6728e:	c4 43 fb f0 d2 03    	rorx   $0x3,%r10,%r10
   67294:	c4 e3 fb f0 d2 19    	rorx   $0x19,%rdx,%rdx
   6729a:	48 31 c8             	xor    %rcx,%rax
   6729d:	48 89 7c 24 08       	mov    %rdi,0x8(%rsp)
   672a2:	48 89 cf             	mov    %rcx,%rdi
   672a5:	48 8b 4c 24 b0       	mov    -0x50(%rsp),%rcx
   672aa:	49 31 f8             	xor    %rdi,%r8
   672ad:	48 31 7c 24 d0       	xor    %rdi,-0x30(%rsp)
   672b2:	4d 31 f3             	xor    %r14,%r11
   672b5:	4c 33 74 24 10       	xor    0x10(%rsp),%r14
   672ba:	c4 e2 98 f2 e8       	andn   %rax,%r12,%rbp
   672bf:	c4 43 fb f0 db 14    	rorx   $0x14,%r11,%r11
   672c5:	48 89 44 24 80       	mov    %rax,-0x80(%rsp)
   672ca:	c4 43 fb f0 c0 2e    	rorx   $0x2e,%r8,%r8
   672d0:	4c 31 cd             	xor    %r9,%rbp
   672d3:	48 89 6c 24 e0       	mov    %rbp,-0x20(%rsp)
   672d8:	c4 e2 a0 f2 ee       	andn   %rsi,%r11,%rbp
   672dd:	48 31 f9             	xor    %rdi,%rcx
   672e0:	48 33 7c 24 a8       	xor    -0x58(%rsp),%rdi
   672e5:	48 89 7c 24 18       	mov    %rdi,0x18(%rsp)
   672ea:	c4 c2 c8 f2 f9       	andn   %r9,%rsi,%rdi
   672ef:	c4 42 b0 f2 cc       	andn   %r12,%r9,%r9
   672f4:	49 31 f1             	xor    %rsi,%r9
   672f7:	c4 c2 f8 f2 f3       	andn   %r11,%rax,%rsi
   672fc:	4c 31 df             	xor    %r11,%rdi
   672ff:	c4 63 fb f0 d9 3d    	rorx   $0x3d,%rcx,%r11
   67305:	c4 c2 e0 f2 ca       	andn   %r10,%rbx,%rcx
   6730a:	4c 31 e6             	xor    %r12,%rsi
   6730d:	4c 89 4c 24 e8       	mov    %r9,-0x18(%rsp)
   67312:	c4 63 fb f0 4c 24 88 	rorx   $0x2c,-0x78(%rsp),%r9
   67319:	2c 
   6731a:	48 89 7c 24 f8       	mov    %rdi,-0x8(%rsp)
   6731f:	4c 31 d9             	xor    %r11,%rcx
   67322:	48 89 74 24 a8       	mov    %rsi,-0x58(%rsp)
   67327:	c4 e3 fb f0 74 24 c0 	rorx   $0x24,-0x40(%rsp),%rsi
   6732e:	24 
   6732f:	48 89 4c 24 b0       	mov    %rcx,-0x50(%rsp)
   67334:	c4 c2 b0 f2 fb       	andn   %r11,%r9,%rdi
   67339:	c4 e2 a8 f2 c6       	andn   %rsi,%r10,%rax
   6733e:	48 31 f7             	xor    %rsi,%rdi
   67341:	48 31 d8             	xor    %rbx,%rax
   67344:	48 89 7c 24 c0       	mov    %rdi,-0x40(%rsp)
   67349:	c4 c3 fb f0 ff 08    	rorx   $0x8,%r15,%rdi
   6734f:	48 89 44 24 f0       	mov    %rax,-0x10(%rsp)
   67354:	c4 e2 a0 f2 c3       	andn   %rbx,%r11,%rax
   67359:	4c 31 c8             	xor    %r9,%rax
   6735c:	48 89 44 24 30       	mov    %rax,0x30(%rsp)
   67361:	c4 c2 c8 f2 c1       	andn   %r9,%rsi,%rax
   67366:	c4 e3 fb f0 74 24 a0 	rorx   $0x3f,-0x60(%rsp),%rsi
   6736d:	3f 
   6736e:	c4 63 fb f0 4c 24 c8 	rorx   $0x3a,-0x38(%rsp),%r9
   67375:	3a 
   67376:	4c 31 d0             	xor    %r10,%rax
   67379:	c4 63 fb f0 54 24 90 	rorx   $0x27,-0x70(%rsp),%r10
   67380:	27 
   67381:	48 89 44 24 88       	mov    %rax,-0x78(%rsp)
   67386:	c4 e3 fb f0 04 24 38 	rorx   $0x38,(%rsp),%rax
   6738d:	c4 e2 b8 f2 ce       	andn   %rsi,%r8,%rcx
   67392:	c4 62 a8 f2 d8       	andn   %rax,%r10,%r11
   67397:	48 31 c1             	xor    %rax,%rcx
   6739a:	c4 c2 f8 f2 c0       	andn   %r8,%rax,%rax
   6739f:	4c 31 d0             	xor    %r10,%rax
   673a2:	48 89 4c 24 90       	mov    %rcx,-0x70(%rsp)
   673a7:	4d 31 cb             	xor    %r9,%r11
   673aa:	48 89 44 24 a0       	mov    %rax,-0x60(%rsp)
   673af:	c4 c2 b0 f2 c2       	andn   %r10,%r9,%rax
   673b4:	c4 63 fb f0 54 24 b8 	rorx   $0x31,-0x48(%rsp),%r10
   673bb:	31 
   673bc:	4c 89 5c 24 10       	mov    %r11,0x10(%rsp)
   673c1:	48 31 f0             	xor    %rsi,%rax
   673c4:	48 89 44 24 c8       	mov    %rax,-0x38(%rsp)
   673c9:	c4 c2 c8 f2 c1       	andn   %r9,%rsi,%rax
   673ce:	c4 e3 fb f0 74 24 d0 	rorx   $0x1c,-0x30(%rsp),%rsi
   673d5:	1c 
   673d6:	4c 31 c0             	xor    %r8,%rax
   673d9:	c4 63 fb f0 44 24 98 	rorx   $0x36,-0x68(%rsp),%r8
   673e0:	36 
   673e1:	48 89 04 24          	mov    %rax,(%rsp)
   673e5:	c4 e3 fb f0 44 24 d8 	rorx   $0x25,-0x28(%rsp),%rax
   673ec:	25 
   673ed:	c4 42 c8 f2 e0       	andn   %r8,%rsi,%r12
   673f2:	c4 c2 b8 f2 da       	andn   %r10,%r8,%rbx
   673f7:	c4 e2 c0 f2 c8       	andn   %rax,%rdi,%rcx
   673fc:	49 31 c4             	xor    %rax,%r12
   673ff:	c4 e2 f8 f2 c6       	andn   %rsi,%rax,%rax
   67404:	48 31 f3             	xor    %rsi,%rbx
   67407:	c4 e3 fb f0 74 24 18 	rorx   $0x17,0x18(%rsp),%rsi
   6740e:	17 
   6740f:	48 31 f8             	xor    %rdi,%rax
   67412:	4c 31 d1             	xor    %r10,%rcx
   67415:	c4 62 a8 f2 d7       	andn   %rdi,%r10,%r10
   6741a:	c4 c3 fb f0 fe 3e    	rorx   $0x3e,%r14,%rdi
   67420:	49 89 df             	mov    %rbx,%r15
   67423:	48 89 5c 24 50       	mov    %rbx,0x50(%rsp)
   67428:	4c 89 64 24 40       	mov    %r12,0x40(%rsp)
   6742d:	48 89 44 24 98       	mov    %rax,-0x68(%rsp)
   67432:	c4 e3 fb f0 44 24 08 	rorx   $0x2,0x8(%rsp),%rax
   67439:	02 
   6743a:	4d 31 c2             	xor    %r8,%r10
   6743d:	48 89 4c 24 d8       	mov    %rcx,-0x28(%rsp)
   67442:	c4 c3 fb f0 cd 09    	rorx   $0x9,%r13,%rcx
   67448:	4c 8b 2c 24          	mov    (%rsp),%r13
   6744c:	4c 89 54 24 28       	mov    %r10,0x28(%rsp)
   67451:	c4 62 c0 f2 c0       	andn   %rax,%rdi,%r8
   67456:	c4 62 f8 f2 f1       	andn   %rcx,%rax,%r14
   6745b:	49 31 f0             	xor    %rsi,%r8
   6745e:	49 31 fe             	xor    %rdi,%r14
   67461:	4c 89 c3             	mov    %r8,%rbx
   67464:	4c 89 44 24 b8       	mov    %r8,-0x48(%rsp)
   67469:	c4 62 e8 f2 c6       	andn   %rsi,%rdx,%r8
   6746e:	c4 e2 c8 f2 f7       	andn   %rdi,%rsi,%rsi
   67473:	48 31 d6             	xor    %rdx,%rsi
   67476:	c4 e2 f0 f2 d2       	andn   %rdx,%rcx,%rdx
   6747b:	49 31 c8             	xor    %rcx,%r8
   6747e:	4c 89 f9             	mov    %r15,%rcx
   67481:	4c 31 d9             	xor    %r11,%rcx
   67484:	4c 8b 5c 24 88       	mov    -0x78(%rsp),%r11
   67489:	48 31 c2             	xor    %rax,%rdx
   6748c:	49 89 f1             	mov    %rsi,%r9
   6748f:	48 89 74 24 d0       	mov    %rsi,-0x30(%rsp)
   67494:	48 8d 05 9d 64 fa ff 	lea    -0x59b63(%rip),%rax        # d938 <anon.9574f76b19d4346c35d273383b004f84.0.llvm.12991719356936023942+0x4d>
   6749b:	4d 89 cf             	mov    %r9,%r15
   6749e:	4c 33 7c 24 e8       	xor    -0x18(%rsp),%r15
   674a3:	4c 8b 4c 24 a8       	mov    -0x58(%rsp),%r9
   674a8:	4c 89 44 24 48       	mov    %r8,0x48(%rsp)
   674ad:	48 89 d6             	mov    %rdx,%rsi
   674b0:	48 89 54 24 08       	mov    %rdx,0x8(%rsp)
   674b5:	48 8b 54 24 20       	mov    0x20(%rsp),%rdx
   674ba:	4d 31 dd             	xor    %r11,%r13
   674bd:	48 33 6c 02 f8       	xor    -0x8(%rdx,%rax,1),%rbp
   674c2:	48 8b 44 24 30       	mov    0x30(%rsp),%rax
   674c7:	48 33 44 24 f8       	xor    -0x8(%rsp),%rax
   674cc:	48 33 6c 24 80       	xor    -0x80(%rsp),%rbp
   674d1:	48 31 c1             	xor    %rax,%rcx
   674d4:	48 8b 44 24 b0       	mov    -0x50(%rsp),%rax
   674d9:	48 89 ef             	mov    %rbp,%rdi
   674dc:	48 89 6c 24 80       	mov    %rbp,-0x80(%rsp)
   674e1:	4c 89 e5             	mov    %r12,%rbp
   674e4:	48 33 6c 24 c8       	xor    -0x38(%rsp),%rbp
   674e9:	48 33 6c 24 c0       	xor    -0x40(%rsp),%rbp
   674ee:	4c 31 c1             	xor    %r8,%rcx
   674f1:	4c 8b 44 24 e0       	mov    -0x20(%rsp),%r8
   674f6:	4c 31 d0             	xor    %r10,%rax
   674f9:	4c 8b 54 24 90       	mov    -0x70(%rsp),%r10
   674fe:	49 31 c7             	xor    %rax,%r15
   67501:	48 89 d8             	mov    %rbx,%rax
   67504:	48 33 44 24 e0       	xor    -0x20(%rsp),%rax
   67509:	48 31 f5             	xor    %rsi,%rbp
   6750c:	c4 e3 fb f0 d9 3f    	rorx   $0x3f,%rcx,%rbx
   67512:	48 31 fd             	xor    %rdi,%rbp
   67515:	48 8b 7c 24 98       	mov    -0x68(%rsp),%rdi
   6751a:	4c 89 d2             	mov    %r10,%rdx
   6751d:	48 33 54 24 f0       	xor    -0x10(%rsp),%rdx
   67522:	48 31 d0             	xor    %rdx,%rax
   67525:	48 33 44 24 d8       	xor    -0x28(%rsp),%rax
   6752a:	4c 89 ca             	mov    %r9,%rdx
   6752d:	4c 31 f2             	xor    %r14,%rdx
   67530:	49 31 d5             	xor    %rdx,%r13
   67533:	48 8b 54 24 a0       	mov    -0x60(%rsp),%rdx
   67538:	49 31 fd             	xor    %rdi,%r13
   6753b:	4c 31 eb             	xor    %r13,%rbx
   6753e:	c4 43 fb f0 e5 3f    	rorx   $0x3f,%r13,%r12
   67544:	48 31 5c 24 08       	xor    %rbx,0x8(%rsp)
   67549:	c4 e3 fb f0 f0 3f    	rorx   $0x3f,%rax,%rsi
   6754f:	48 31 ce             	xor    %rcx,%rsi
   67552:	49 31 d7             	xor    %rdx,%r15
   67555:	48 8b 4c 24 80       	mov    -0x80(%rsp),%rcx
   6755a:	48 31 f2             	xor    %rsi,%rdx
   6755d:	48 31 74 24 d0       	xor    %rsi,-0x30(%rsp)
   67562:	48 31 74 24 b0       	xor    %rsi,-0x50(%rsp)
   67567:	48 31 74 24 28       	xor    %rsi,0x28(%rsp)
   6756c:	48 33 74 24 e8       	xor    -0x18(%rsp),%rsi
   67571:	c4 43 fb f0 ef 3f    	rorx   $0x3f,%r15,%r13
   67577:	4d 31 fc             	xor    %r15,%r12
   6757a:	4c 8b 7c 24 c0       	mov    -0x40(%rsp),%r15
   6757f:	49 31 ed             	xor    %rbp,%r13
   67582:	c4 e3 fb f0 ed 3f    	rorx   $0x3f,%rbp,%rbp
   67588:	48 89 54 24 a0       	mov    %rdx,-0x60(%rsp)
   6758d:	48 8b 54 24 c8       	mov    -0x38(%rsp),%rdx
   67592:	4d 31 e0             	xor    %r12,%r8
   67595:	4d 31 e2             	xor    %r12,%r10
   67598:	4c 31 64 24 b8       	xor    %r12,-0x48(%rsp)
   6759d:	48 31 c5             	xor    %rax,%rbp
   675a0:	48 8b 44 24 30       	mov    0x30(%rsp),%rax
   675a5:	4c 89 54 24 90       	mov    %r10,-0x70(%rsp)
   675aa:	4c 8b 54 24 f8       	mov    -0x8(%rsp),%r10
   675af:	4c 31 6c 24 10       	xor    %r13,0x10(%rsp)
   675b4:	49 31 eb             	xor    %rbp,%r11
   675b7:	49 31 e9             	xor    %rbp,%r9
   675ba:	49 31 ee             	xor    %rbp,%r14
   675bd:	48 31 ef             	xor    %rbp,%rdi
   675c0:	48 33 2c 24          	xor    (%rsp),%rbp
   675c4:	4c 89 5c 24 88       	mov    %r11,-0x78(%rsp)
   675c9:	c4 63 fb f0 5c 24 a0 	rorx   $0x15,-0x60(%rsp),%r11
   675d0:	15 
   675d1:	48 31 d9             	xor    %rbx,%rcx
   675d4:	4c 89 4c 24 a8       	mov    %r9,-0x58(%rsp)
   675d9:	48 89 7c 24 98       	mov    %rdi,-0x68(%rsp)
   675de:	48 89 74 24 18       	mov    %rsi,0x18(%rsp)
   675e3:	48 8b 74 24 d8       	mov    -0x28(%rsp),%rsi
   675e8:	49 31 df             	xor    %rbx,%r15
   675eb:	48 89 4c 24 80       	mov    %rcx,-0x80(%rsp)
   675f0:	48 31 da             	xor    %rbx,%rdx
   675f3:	48 33 5c 24 40       	xor    0x40(%rsp),%rbx
   675f8:	4c 31 e8             	xor    %r13,%rax
   675fb:	4d 31 ea             	xor    %r13,%r10
   675fe:	c4 e3 fb f0 c0 14    	rorx   $0x14,%rax,%rax
   67604:	4c 31 e6             	xor    %r12,%rsi
   67607:	4c 33 64 24 f0       	xor    -0x10(%rsp),%r12
   6760c:	c4 63 fb f0 ce 2b    	rorx   $0x2b,%rsi,%r9
   67612:	c4 c3 fb f0 f6 32    	rorx   $0x32,%r14,%rsi
   67618:	48 89 1c 24          	mov    %rbx,(%rsp)
   6761c:	48 8b 5c 24 50       	mov    0x50(%rsp),%rbx
   67621:	c4 e2 c8 f2 f9       	andn   %rcx,%rsi,%rdi
   67626:	c4 42 a0 f2 f1       	andn   %r9,%r11,%r14
   6762b:	4c 31 cf             	xor    %r9,%rdi
   6762e:	49 31 c6             	xor    %rax,%r14
   67631:	c4 62 b0 f2 ce       	andn   %rsi,%r9,%r9
   67636:	48 89 7c 24 c0       	mov    %rdi,-0x40(%rsp)
   6763b:	c4 c2 f8 f2 fb       	andn   %r11,%rax,%rdi
   67640:	c4 e2 f0 f2 c0       	andn   %rax,%rcx,%rax
   67645:	4d 31 d9             	xor    %r11,%r9
   67648:	4c 89 74 24 a0       	mov    %r14,-0x60(%rsp)
   6764d:	c4 63 fb f0 74 24 b8 	rorx   $0x8,-0x48(%rsp),%r14
   67654:	08 
   67655:	48 31 f0             	xor    %rsi,%rax
   67658:	c4 e3 fb f0 74 24 88 	rorx   $0x2c,-0x78(%rsp),%rsi
   6765f:	2c 
   67660:	4c 89 4c 24 f0       	mov    %r9,-0x10(%rsp)
   67665:	c4 63 fb f0 ca 3d    	rorx   $0x3d,%rdx,%r9
   6766b:	c4 e3 fb f0 54 24 18 	rorx   $0x2,0x18(%rsp),%rdx
   67672:	02 
   67673:	48 89 44 24 d8       	mov    %rax,-0x28(%rsp)
   67678:	c4 c3 fb f0 c0 24    	rorx   $0x24,%r8,%rax
   6767e:	c4 63 fb f0 44 24 d0 	rorx   $0x3,-0x30(%rsp),%r8
   67685:	03 
   67686:	4c 31 eb             	xor    %r13,%rbx
   67689:	4c 33 6c 24 48       	xor    0x48(%rsp),%r13
   6768e:	c4 63 fb f0 db 13    	rorx   $0x13,%rbx,%r11
   67694:	c4 e2 b8 f2 d8       	andn   %rax,%r8,%rbx
   67699:	4c 31 db             	xor    %r11,%rbx
   6769c:	48 89 5c 24 e8       	mov    %rbx,-0x18(%rsp)
   676a1:	c4 c2 b0 f2 db       	andn   %r11,%r9,%rbx
   676a6:	c4 42 a0 f2 d8       	andn   %r8,%r11,%r11
   676ab:	4d 31 cb             	xor    %r9,%r11
   676ae:	c4 42 c8 f2 c9       	andn   %r9,%rsi,%r9
   676b3:	48 31 f3             	xor    %rsi,%rbx
   676b6:	49 31 c1             	xor    %rax,%r9
   676b9:	c4 e2 f8 f2 c6       	andn   %rsi,%rax,%rax
   676be:	c4 e3 fb f0 74 24 98 	rorx   $0x38,-0x68(%rsp),%rsi
   676c5:	38 
   676c6:	4c 89 5c 24 c8       	mov    %r11,-0x38(%rsp)
   676cb:	48 89 5c 24 f8       	mov    %rbx,-0x8(%rsp)
   676d0:	4c 31 c0             	xor    %r8,%rax
   676d3:	4c 89 4c 24 d0       	mov    %r9,-0x30(%rsp)
   676d8:	c4 63 fb f0 4c 24 90 	rorx   $0x27,-0x70(%rsp),%r9
   676df:	27 
   676e0:	c4 63 fb f0 44 24 b0 	rorx   $0x3a,-0x50(%rsp),%r8
   676e7:	3a 
   676e8:	48 89 44 24 88       	mov    %rax,-0x78(%rsp)
   676ed:	c4 c3 fb f0 c2 3f    	rorx   $0x3f,%r10,%rax
   676f3:	c4 63 fb f0 54 24 08 	rorx   $0x2e,0x8(%rsp),%r10
   676fa:	2e 
   676fb:	c4 e2 b0 f2 ce       	andn   %rsi,%r9,%rcx
   67700:	c4 62 a8 f2 d8       	andn   %rax,%r10,%r11
   67705:	4c 31 c1             	xor    %r8,%rcx
   67708:	49 31 f3             	xor    %rsi,%r11
   6770b:	c4 c2 c8 f2 f2       	andn   %r10,%rsi,%rsi
   67710:	48 89 4c 24 98       	mov    %rcx,-0x68(%rsp)
   67715:	4c 31 ce             	xor    %r9,%rsi
   67718:	c4 42 b8 f2 c9       	andn   %r9,%r8,%r9
   6771d:	4c 89 5c 24 90       	mov    %r11,-0x70(%rsp)
   67722:	c4 63 fb f0 5c 24 28 	rorx   $0x31,0x28(%rsp),%r11
   67729:	31 
   6772a:	49 31 c1             	xor    %rax,%r9
   6772d:	c4 c2 f8 f2 c0       	andn   %r8,%rax,%rax
   67732:	c4 43 fb f0 c7 1c    	rorx   $0x1c,%r15,%r8
   67738:	4c 31 d0             	xor    %r10,%rax
   6773b:	c4 63 fb f0 54 24 10 	rorx   $0x36,0x10(%rsp),%r10
   67742:	36 
   67743:	4c 89 4c 24 b0       	mov    %r9,-0x50(%rsp)
   67748:	48 89 44 24 e0       	mov    %rax,-0x20(%rsp)
   6774d:	c4 e3 fb f0 44 24 a8 	rorx   $0x25,-0x58(%rsp),%rax
   67754:	25 
   67755:	c4 c2 a8 f2 db       	andn   %r11,%r10,%rbx
   6775a:	c4 c2 b8 f2 ca       	andn   %r10,%r8,%rcx
   6775f:	c4 62 88 f2 c8       	andn   %rax,%r14,%r9
   67764:	48 31 c1             	xor    %rax,%rcx
   67767:	c4 c2 f8 f2 c0       	andn   %r8,%rax,%rax
   6776c:	4c 31 c3             	xor    %r8,%rbx
   6776f:	c4 63 fb f0 c5 19    	rorx   $0x19,%rbp,%r8
   67775:	4d 31 d9             	xor    %r11,%r9
   67778:	c4 42 a0 f2 de       	andn   %r14,%r11,%r11
   6777d:	48 89 4c 24 a8       	mov    %rcx,-0x58(%rsp)
   67782:	c4 c3 fb f0 cc 09    	rorx   $0x9,%r12,%rcx
   67788:	4c 31 f0             	xor    %r14,%rax
   6778b:	4d 31 d3             	xor    %r10,%r11
   6778e:	c4 63 fb f0 14 24 17 	rorx   $0x17,(%rsp),%r10
   67795:	c4 e2 e8 f2 e9       	andn   %rcx,%rdx,%rbp
   6779a:	4c 89 5c 24 b8       	mov    %r11,-0x48(%rsp)
   6779f:	c4 43 fb f0 dd 3e    	rorx   $0x3e,%r13,%r11
   677a5:	c4 62 a0 f2 fa       	andn   %rdx,%r11,%r15
   677aa:	4c 31 dd             	xor    %r11,%rbp
   677ad:	c4 42 b8 f2 e2       	andn   %r10,%r8,%r12
   677b2:	4d 31 d7             	xor    %r10,%r15
   677b5:	c4 42 a8 f2 d3       	andn   %r11,%r10,%r10
   677ba:	4c 8b 5c 24 20       	mov    0x20(%rsp),%r11
   677bf:	4d 31 c2             	xor    %r8,%r10
   677c2:	c4 42 f0 f2 c0       	andn   %r8,%rcx,%r8
   677c7:	49 31 cc             	xor    %rcx,%r12
   677ca:	49 31 d0             	xor    %rdx,%r8
   677cd:	48 8d 15 64 61 fa ff 	lea    -0x59e9c(%rip),%rdx        # d938 <anon.9574f76b19d4346c35d273383b004f84.0.llvm.12991719356936023942+0x4d>
   677d4:	49 33 3c 13          	xor    (%r11,%rdx,1),%rdi
   677d8:	49 83 c3 10          	add    $0x10,%r11
   677dc:	48 33 7c 24 80       	xor    -0x80(%rsp),%rdi
   677e1:	4c 89 5c 24 20       	mov    %r11,0x20(%rsp)
   677e6:	48 89 7c 24 80       	mov    %rdi,-0x80(%rsp)
   677eb:	49 81 fb c8 00 00 00 	cmp    $0xc8,%r11
   677f2:	0f 85 78 f9 ff ff    	jne    67170 <purrdf_hash::sha3::keccak_f1600+0xf0>
   677f8:	48 8b 4c 24 38       	mov    0x38(%rsp),%rcx
   677fd:	48 8b 54 24 98       	mov    -0x68(%rsp),%rdx
   67802:	48 8b 7c 24 e8       	mov    -0x18(%rsp),%rdi
   67807:	4c 89 81 a0 00 00 00 	mov    %r8,0xa0(%rcx)
   6780e:	4c 8b 44 24 b8       	mov    -0x48(%rsp),%r8
   67813:	48 89 51 58          	mov    %rdx,0x58(%rcx)
   67817:	48 8b 54 24 80       	mov    -0x80(%rsp),%rdx
   6781c:	4c 89 81 88 00 00 00 	mov    %r8,0x88(%rcx)
   67823:	4c 89 b9 b8 00 00 00 	mov    %r15,0xb8(%rcx)
   6782a:	48 89 79 40          	mov    %rdi,0x40(%rcx)
   6782e:	4c 8b 44 24 e0       	mov    -0x20(%rsp),%r8
   67833:	48 8b 7c 24 a8       	mov    -0x58(%rsp),%rdi
   67838:	4c 89 41 70          	mov    %r8,0x70(%rcx)
   6783c:	48 89 79 78          	mov    %rdi,0x78(%rcx)
   67840:	4c 8b 44 24 a0       	mov    -0x60(%rsp),%r8
   67845:	48 8b 7c 24 f0       	mov    -0x10(%rsp),%rdi
   6784a:	4c 89 a1 a8 00 00 00 	mov    %r12,0xa8(%rcx)
   67851:	4c 89 41 08          	mov    %r8,0x8(%rcx)
   67855:	48 89 79 10          	mov    %rdi,0x10(%rcx)
   67859:	4c 8b 44 24 c0       	mov    -0x40(%rsp),%r8
   6785e:	48 8b 7c 24 d8       	mov    -0x28(%rsp),%rdi
   67863:	4c 89 41 18          	mov    %r8,0x18(%rcx)
   67867:	48 89 79 20          	mov    %rdi,0x20(%rcx)
   6786b:	4c 8b 44 24 d0       	mov    -0x30(%rsp),%r8
   67870:	48 8b 7c 24 f8       	mov    -0x8(%rsp),%rdi
   67875:	4c 89 41 28          	mov    %r8,0x28(%rcx)
   67879:	48 89 79 30          	mov    %rdi,0x30(%rcx)
   6787d:	4c 8b 44 24 c8       	mov    -0x38(%rsp),%r8
   67882:	48 8b 7c 24 88       	mov    -0x78(%rsp),%rdi
   67887:	4c 89 41 38          	mov    %r8,0x38(%rcx)
   6788b:	48 89 79 48          	mov    %rdi,0x48(%rcx)
   6788f:	4c 8b 44 24 b0       	mov    -0x50(%rsp),%r8
   67894:	48 8b 7c 24 90       	mov    -0x70(%rsp),%rdi
   67899:	4c 89 41 50          	mov    %r8,0x50(%rcx)
   6789d:	48 89 79 68          	mov    %rdi,0x68(%rcx)
   678a1:	48 89 71 60          	mov    %rsi,0x60(%rcx)
   678a5:	48 89 99 80 00 00 00 	mov    %rbx,0x80(%rcx)
   678ac:	4c 89 89 90 00 00 00 	mov    %r9,0x90(%rcx)
   678b3:	48 89 81 98 00 00 00 	mov    %rax,0x98(%rcx)
   678ba:	4c 89 91 b0 00 00 00 	mov    %r10,0xb0(%rcx)
   678c1:	48 89 a9 c0 00 00 00 	mov    %rbp,0xc0(%rcx)
   678c8:	48 89 11             	mov    %rdx,(%rcx)
   678cb:	48 83 c4 58          	add    $0x58,%rsp
   678cf:	5b                   	pop    %rbx
   678d0:	41 5c                	pop    %r12
   678d2:	41 5d                	pop    %r13
   678d4:	41 5e                	pop    %r14
   678d6:	41 5f                	pop    %r15
   678d8:	5d                   	pop    %rbp
   678d9:	c3                   	ret

Disassembly of section .init:

Disassembly of section .fini:

Disassembly of section .plt:
