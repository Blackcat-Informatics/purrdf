; ModuleID = 'purrdf_stack-b92b83ba5be029a7.purrdf_stack.4d20217323d74ed4-cgu.0.rcgu.o'
source_filename = "purrdf_stack.4d20217323d74ed4-cgu.0"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

@anon.b0b42612b86aa4e26bca5f33320cc1f1.0.llvm.12345758532933907005 = hidden unnamed_addr constant [80 x i8] c"/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad/library/std/src/thread/local.rs\00", align 1, !guid !0
@anon.b0b42612b86aa4e26bca5f33320cc1f1.1.llvm.12345758532933907005 = hidden unnamed_addr constant <{ ptr, [16 x i8] }> <{ ptr @anon.b0b42612b86aa4e26bca5f33320cc1f1.0.llvm.12345758532933907005, [16 x i8] c"O\00\00\00\00\00\00\00\AD\01\00\00\19\00\00\00" }>, align 8, !guid !1
@_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack4WALK0s_023___RUST_STD_INTERNAL_VAL = thread_local local_unnamed_addr global <{ [8 x i8], [24 x i8] }> <{ [8 x i8] zeroinitializer, [24 x i8] undef }>, align 8, !guid !2
@_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack5FLOOR0s_023___RUST_STD_INTERNAL_VAL = thread_local local_unnamed_addr global [8 x i8] c"\FF\FF\FF\FF\FF\FF\FF\FF", align 8, !guid !3
@_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack8RESERVED0s_023___RUST_STD_INTERNAL_VAL.llvm.12345758532933907005 = hidden thread_local unnamed_addr global [8 x i8] zeroinitializer, align 8, !guid !4

; purrdf_stack::is_low_cold
; Function Attrs: cold noinline nounwind nonlazybind uwtable
define noundef zeroext i1 @_RNvCs6CxjjnbUHAu_12purrdf_stack11is_low_cold(i64 noundef %0) unnamed_addr #0 personality ptr @rust_eh_personality !guid !9 {
  %2 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack5FLOOR0s_023___RUST_STD_INTERNAL_VAL)
  %3 = load i64, ptr %2, align 8, !noundef !10
  %4 = icmp eq i64 %3, -2
  br i1 %4, label %7, label %5

5:                                                ; preds = %1
  %6 = icmp ult i64 %0, %3
  br i1 %6, label %11, label %9

7:                                                ; preds = %13, %1
  %8 = phi i1 [ %15, %13 ], [ true, %1 ]
  ret i1 %8

9:                                                ; preds = %5
  %10 = sub nuw i64 %0, %3
  br label %13

11:                                               ; preds = %5
; call purrdf_stack::refresh
  %12 = tail call fastcc noundef i64 @_RNvCs6CxjjnbUHAu_12purrdf_stack7refresh.llvm.12345758532933907005(i64 noundef %0)
  br label %13

13:                                               ; preds = %11, %9
  %14 = phi i64 [ %12, %11 ], [ %10, %9 ]
  %15 = icmp ult i64 %14, 131072
  br label %7
}

; purrdf_stack::walk_refuse
; Function Attrs: cold mustprogress nofree noinline norecurse nosync nounwind nonlazybind willreturn memory(readwrite, argmem: none, inaccessiblemem: none, target_mem: none) uwtable
define noundef zeroext i1 @_RNvCs6CxjjnbUHAu_12purrdf_stack11walk_refuse(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %0, i64 noundef %1) unnamed_addr #1 personality ptr @rust_eh_personality !guid !11 {
  %3 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack4WALK0s_023___RUST_STD_INTERNAL_VAL)
  %4 = load i64, ptr %3, align 8
  switch i64 %4, label %5 [
    i64 0, label %13
    i64 1, label %6
    i64 2, label %12
  ]

5:                                                ; preds = %2
  unreachable

6:                                                ; preds = %2
  %7 = getelementptr inbounds nuw i8, ptr %3, i64 8
  %8 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack5FLOOR0s_023___RUST_STD_INTERNAL_VAL)
  %9 = load i64, ptr %8, align 8, !noundef !10
  store i64 -2, ptr %8, align 8
  store i64 2, ptr %3, align 8
  store i64 %9, ptr %7, align 8
  %10 = getelementptr inbounds nuw i8, ptr %3, i64 16
  store ptr %0, ptr %10, align 8
  %11 = getelementptr inbounds nuw i8, ptr %3, i64 24
  store i64 %1, ptr %11, align 8
  br label %13

12:                                               ; preds = %2
  br label %13

13:                                               ; preds = %12, %6, %2
  %14 = phi i1 [ true, %12 ], [ true, %6 ], [ false, %2 ]
  ret i1 %14
}

; purrdf_stack::refresh
; Function Attrs: nounwind nonlazybind uwtable
define hidden fastcc noundef range(i64 0, -1) i64 @_RNvCs6CxjjnbUHAu_12purrdf_stack7refresh.llvm.12345758532933907005(i64 noundef range(i64 0, -1) %0) unnamed_addr #2 personality ptr @rust_eh_personality !guid !12 {
  %2 = alloca [8 x i8], align 8
  %3 = alloca [8 x i8], align 8
  %4 = alloca [56 x i8], align 8
  %5 = alloca [56 x i8], align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %5)
  %6 = call noundef i32 @pthread_attr_init(ptr noundef nonnull %5) #8
  %7 = icmp eq i32 %6, 0
  br i1 %7, label %9, label %8

8:                                                ; preds = %1
  call void @llvm.lifetime.end.p0(ptr nonnull %5)
  br label %25

9:                                                ; preds = %1
  call void @llvm.lifetime.start.p0(ptr nonnull %4)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %4, ptr noundef nonnull align 8 dereferenceable(56) %5, i64 56, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %3)
  store ptr null, ptr %3, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %2)
  store i64 0, ptr %2, align 8
  %10 = call noundef i64 @pthread_self() #8
  %11 = call noundef i32 @pthread_getattr_np(i64 noundef %10, ptr noundef nonnull %4) #8
  %12 = icmp eq i32 %11, 0
  br i1 %12, label %13, label %16

13:                                               ; preds = %9
  %14 = call noundef i32 @pthread_attr_getstack(ptr noundef nonnull %4, ptr noundef nonnull %3, ptr noundef nonnull %2) #8
  %15 = icmp eq i32 %14, 0
  br i1 %15, label %18, label %16

16:                                               ; preds = %13, %9
  %17 = call noundef i32 @pthread_attr_destroy(ptr noundef nonnull %4) #8
  call void @llvm.lifetime.end.p0(ptr nonnull %2)
  call void @llvm.lifetime.end.p0(ptr nonnull %3)
  call void @llvm.lifetime.end.p0(ptr nonnull %4)
  call void @llvm.lifetime.end.p0(ptr nonnull %5)
  br label %25

18:                                               ; preds = %13
  %19 = load ptr, ptr %3, align 8, !noundef !10
  %20 = ptrtoint ptr %19 to i64
  %21 = call noundef i32 @pthread_attr_destroy(ptr noundef nonnull %4) #8
  call void @llvm.lifetime.end.p0(ptr nonnull %2)
  call void @llvm.lifetime.end.p0(ptr nonnull %3)
  call void @llvm.lifetime.end.p0(ptr nonnull %4)
  call void @llvm.lifetime.end.p0(ptr nonnull %5)
  %22 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack8RESERVED0s_023___RUST_STD_INTERNAL_VAL.llvm.12345758532933907005)
  %23 = load i64, ptr %22, align 8, !noundef !10
  %24 = call i64 @llvm.uadd.sat.i64(i64 %20, i64 %23)
  br label %25

25:                                               ; preds = %18, %16, %8
  %26 = phi i64 [ %24, %18 ], [ 0, %16 ], [ 0, %8 ]
  %27 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack5FLOOR0s_023___RUST_STD_INTERNAL_VAL)
  store i64 %26, ptr %27, align 8
  %28 = call i64 @llvm.usub.sat.i64(i64 %0, i64 %26)
  ret i64 %28
}

; purrdf_stack::remaining
; Function Attrs: nounwind nonlazybind uwtable
define noundef i64 @_RNvCs6CxjjnbUHAu_12purrdf_stack9remaining() unnamed_addr #2 personality ptr @rust_eh_personality !guid !13 {
  %1 = alloca [8 x i8], align 8
  %2 = alloca [1 x i8], align 1
  call void @llvm.lifetime.start.p0(ptr nonnull %2)
  store i8 0, ptr %2, align 1
  call void @llvm.lifetime.start.p0(ptr nonnull %1)
  store ptr %2, ptr %1, align 8
  call void asm sideeffect "", "r,~{memory}"(ptr nonnull %1) #8
  %3 = load ptr, ptr %1, align 8, !noundef !10
  %4 = ptrtoint ptr %3 to i64
  call void @llvm.lifetime.end.p0(ptr nonnull %1)
  call void @llvm.lifetime.end.p0(ptr nonnull %2)
  %5 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack5FLOOR0s_023___RUST_STD_INTERNAL_VAL)
  %6 = load i64, ptr %5, align 8, !noundef !10
  %7 = icmp eq i64 %6, -2
  br i1 %7, label %10, label %8

8:                                                ; preds = %0
  %9 = icmp ugt i64 %6, %4
  br i1 %9, label %14, label %12

10:                                               ; preds = %14, %12, %0
  %11 = phi i64 [ 0, %0 ], [ %15, %14 ], [ %13, %12 ]
  ret i64 %11

12:                                               ; preds = %8
  %13 = sub nuw i64 %4, %6
  br label %10

14:                                               ; preds = %8
; call purrdf_stack::refresh
  %15 = call fastcc noundef i64 @_RNvCs6CxjjnbUHAu_12purrdf_stack7refresh.llvm.12345758532933907005(i64 noundef %4)
  br label %10
}

; <purrdf_stack::walk::Close as core::ops::drop::Drop>::drop
; Function Attrs: nonlazybind uwtable
define void @_RNvXNvCs6CxjjnbUHAu_12purrdf_stack4walkNtB2_5CloseNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop(ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(32) %0) unnamed_addr #3 personality ptr @rust_eh_personality !guid !14 {
  %2 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack4WALK0s_023___RUST_STD_INTERNAL_VAL)
  %3 = load i64, ptr %2, align 8, !noalias !15
  %4 = getelementptr inbounds nuw i8, ptr %2, i64 8
  %5 = load i64, ptr %4, align 8, !noalias !15
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %2, ptr noundef nonnull readonly align 8 dereferenceable(32) %0, i64 32, i1 false), !noalias !18
  switch i64 %3, label %9 [
    i64 -1, label %6
    i64 2, label %7
  ], !prof !23

6:                                                ; preds = %1
; call std::thread::local::panic_access_error
  tail call void @_RNvNtNtCs7jcFBdfocI9_3std6thread5local18panic_access_error(ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.b0b42612b86aa4e26bca5f33320cc1f1.1.llvm.12345758532933907005) #9, !noalias !15
  unreachable

7:                                                ; preds = %1
  %8 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @_RNvNCNKNvCs6CxjjnbUHAu_12purrdf_stack5FLOOR0s_023___RUST_STD_INTERNAL_VAL)
  store i64 %5, ptr %8, align 8
  br label %9

9:                                                ; preds = %7, %1
  ret void
}

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.start.p0(ptr captures(none)) #4

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.end.p0(ptr captures(none)) #4

; std::thread::local::panic_access_error
; Function Attrs: cold noinline noreturn nonlazybind uwtable
declare void @_RNvNtNtCs7jcFBdfocI9_3std6thread5local18panic_access_error(ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24)) unnamed_addr #5

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i64(ptr noalias writeonly captures(none), ptr noalias readonly captures(none), i64, i1 immarg) #4

; Function Attrs: nounwind nonlazybind uwtable
declare noundef range(i32 0, 10) i32 @rust_eh_personality(i32 noundef, i32 noundef, i64 noundef, ptr noundef, ptr noundef) unnamed_addr #2

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare nonnull ptr @llvm.threadlocal.address.p0(ptr nonnull) #6

; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.uadd.sat.i64(i64, i64) #7

; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.usub.sat.i64(i64, i64) #7

; Function Attrs: nounwind nonlazybind uwtable
declare noundef i32 @pthread_attr_init(ptr noundef) unnamed_addr #2

; Function Attrs: nounwind nonlazybind uwtable
declare noundef i64 @pthread_self() unnamed_addr #2

; Function Attrs: nounwind nonlazybind uwtable
declare noundef i32 @pthread_getattr_np(i64 noundef, ptr noundef) unnamed_addr #2

; Function Attrs: nounwind nonlazybind uwtable
declare noundef i32 @pthread_attr_getstack(ptr noundef, ptr noundef, ptr noundef) unnamed_addr #2

; Function Attrs: nounwind nonlazybind uwtable
declare noundef i32 @pthread_attr_destroy(ptr noundef) unnamed_addr #2

attributes #0 = { cold noinline nounwind nonlazybind uwtable "probe-stack"="inline-asm" "target-cpu"="znver5" "target-features"="+prfchw,-cldemote,+avx,+aes,+sahf,+pclmul,-xop,+crc32,-amx-fp8,+xsaves,-avx512fp16,-usermsr,-sm4,-egpr,+sse4.1,-avx10.1,+avx512ifma,+xsave,+sse4.2,-tsxldtrk,-sm3,-ptwrite,-widekl,-movrs,+invpcid,+64bit,+xsavec,+avx512vpopcntdq,+cmov,+avx512vp2intersect,-avxvnniint8,+avx512cd,+movbe,-ccmp,-amx-int8,-kl,-avx512bmm,-sha512,+avxvnni,-rtm,+adx,+avx2,-hreset,+movdiri,-serialize,+vpclmulqdq,+avx512vl,-uintr,-cf,-jmpabs,+clflushopt,-raoint,-cmpccxadd,+bmi,-amx-tile,+sse,+gfni,-avxvnniint16,-amx-fp16,-zu,-ndd,+xsaveopt,+rdrnd,+avx512f,-amx-bf16,+avx512bf16,+avx512vnni,-push2pop2,+cx8,+avx512bw,+sse3,+pku,-nf,-amx-avx512,+fsgsbase,+clzero,+mwaitx,-lwp,+lzcnt,+sha,+movdir64b,-ppx,+wbnoinvd,-enqcmd,-avxneconvert,-tbm,-pconfig,-amx-complex,+ssse3,+cx16,-avx10.2,+bmi2,+fma,+popcnt,-avxifma,+f16c,+avx512bitalg,+rdpru,+clwb,+mmx,+sse2,+rdseed,+avx512vbmi2,+prefetchi,+rdpid,-amx-movrs,-fma4,+avx512vbmi,+shstk,+vaes,-waitpkg,-sgx,+fxsr,+avx512dq,+sse4a" }
attributes #1 = { cold mustprogress nofree noinline norecurse nosync nounwind nonlazybind willreturn memory(readwrite, argmem: none, inaccessiblemem: none, target_mem: none) uwtable "probe-stack"="inline-asm" "target-cpu"="znver5" "target-features"="+prfchw,-cldemote,+avx,+aes,+sahf,+pclmul,-xop,+crc32,-amx-fp8,+xsaves,-avx512fp16,-usermsr,-sm4,-egpr,+sse4.1,-avx10.1,+avx512ifma,+xsave,+sse4.2,-tsxldtrk,-sm3,-ptwrite,-widekl,-movrs,+invpcid,+64bit,+xsavec,+avx512vpopcntdq,+cmov,+avx512vp2intersect,-avxvnniint8,+avx512cd,+movbe,-ccmp,-amx-int8,-kl,-avx512bmm,-sha512,+avxvnni,-rtm,+adx,+avx2,-hreset,+movdiri,-serialize,+vpclmulqdq,+avx512vl,-uintr,-cf,-jmpabs,+clflushopt,-raoint,-cmpccxadd,+bmi,-amx-tile,+sse,+gfni,-avxvnniint16,-amx-fp16,-zu,-ndd,+xsaveopt,+rdrnd,+avx512f,-amx-bf16,+avx512bf16,+avx512vnni,-push2pop2,+cx8,+avx512bw,+sse3,+pku,-nf,-amx-avx512,+fsgsbase,+clzero,+mwaitx,-lwp,+lzcnt,+sha,+movdir64b,-ppx,+wbnoinvd,-enqcmd,-avxneconvert,-tbm,-pconfig,-amx-complex,+ssse3,+cx16,-avx10.2,+bmi2,+fma,+popcnt,-avxifma,+f16c,+avx512bitalg,+rdpru,+clwb,+mmx,+sse2,+rdseed,+avx512vbmi2,+prefetchi,+rdpid,-amx-movrs,-fma4,+avx512vbmi,+shstk,+vaes,-waitpkg,-sgx,+fxsr,+avx512dq,+sse4a" }
attributes #2 = { nounwind nonlazybind uwtable "probe-stack"="inline-asm" "target-cpu"="znver5" "target-features"="+prfchw,-cldemote,+avx,+aes,+sahf,+pclmul,-xop,+crc32,-amx-fp8,+xsaves,-avx512fp16,-usermsr,-sm4,-egpr,+sse4.1,-avx10.1,+avx512ifma,+xsave,+sse4.2,-tsxldtrk,-sm3,-ptwrite,-widekl,-movrs,+invpcid,+64bit,+xsavec,+avx512vpopcntdq,+cmov,+avx512vp2intersect,-avxvnniint8,+avx512cd,+movbe,-ccmp,-amx-int8,-kl,-avx512bmm,-sha512,+avxvnni,-rtm,+adx,+avx2,-hreset,+movdiri,-serialize,+vpclmulqdq,+avx512vl,-uintr,-cf,-jmpabs,+clflushopt,-raoint,-cmpccxadd,+bmi,-amx-tile,+sse,+gfni,-avxvnniint16,-amx-fp16,-zu,-ndd,+xsaveopt,+rdrnd,+avx512f,-amx-bf16,+avx512bf16,+avx512vnni,-push2pop2,+cx8,+avx512bw,+sse3,+pku,-nf,-amx-avx512,+fsgsbase,+clzero,+mwaitx,-lwp,+lzcnt,+sha,+movdir64b,-ppx,+wbnoinvd,-enqcmd,-avxneconvert,-tbm,-pconfig,-amx-complex,+ssse3,+cx16,-avx10.2,+bmi2,+fma,+popcnt,-avxifma,+f16c,+avx512bitalg,+rdpru,+clwb,+mmx,+sse2,+rdseed,+avx512vbmi2,+prefetchi,+rdpid,-amx-movrs,-fma4,+avx512vbmi,+shstk,+vaes,-waitpkg,-sgx,+fxsr,+avx512dq,+sse4a" }
attributes #3 = { nonlazybind uwtable "probe-stack"="inline-asm" "target-cpu"="znver5" "target-features"="+prfchw,-cldemote,+avx,+aes,+sahf,+pclmul,-xop,+crc32,-amx-fp8,+xsaves,-avx512fp16,-usermsr,-sm4,-egpr,+sse4.1,-avx10.1,+avx512ifma,+xsave,+sse4.2,-tsxldtrk,-sm3,-ptwrite,-widekl,-movrs,+invpcid,+64bit,+xsavec,+avx512vpopcntdq,+cmov,+avx512vp2intersect,-avxvnniint8,+avx512cd,+movbe,-ccmp,-amx-int8,-kl,-avx512bmm,-sha512,+avxvnni,-rtm,+adx,+avx2,-hreset,+movdiri,-serialize,+vpclmulqdq,+avx512vl,-uintr,-cf,-jmpabs,+clflushopt,-raoint,-cmpccxadd,+bmi,-amx-tile,+sse,+gfni,-avxvnniint16,-amx-fp16,-zu,-ndd,+xsaveopt,+rdrnd,+avx512f,-amx-bf16,+avx512bf16,+avx512vnni,-push2pop2,+cx8,+avx512bw,+sse3,+pku,-nf,-amx-avx512,+fsgsbase,+clzero,+mwaitx,-lwp,+lzcnt,+sha,+movdir64b,-ppx,+wbnoinvd,-enqcmd,-avxneconvert,-tbm,-pconfig,-amx-complex,+ssse3,+cx16,-avx10.2,+bmi2,+fma,+popcnt,-avxifma,+f16c,+avx512bitalg,+rdpru,+clwb,+mmx,+sse2,+rdseed,+avx512vbmi2,+prefetchi,+rdpid,-amx-movrs,-fma4,+avx512vbmi,+shstk,+vaes,-waitpkg,-sgx,+fxsr,+avx512dq,+sse4a" }
attributes #4 = { nocallback nofree nosync nounwind willreturn memory(argmem: readwrite) }
attributes #5 = { cold noinline noreturn nonlazybind uwtable "probe-stack"="inline-asm" "target-cpu"="znver5" "target-features"="+prfchw,-cldemote,+avx,+aes,+sahf,+pclmul,-xop,+crc32,-amx-fp8,+xsaves,-avx512fp16,-usermsr,-sm4,-egpr,+sse4.1,-avx10.1,+avx512ifma,+xsave,+sse4.2,-tsxldtrk,-sm3,-ptwrite,-widekl,-movrs,+invpcid,+64bit,+xsavec,+avx512vpopcntdq,+cmov,+avx512vp2intersect,-avxvnniint8,+avx512cd,+movbe,-ccmp,-amx-int8,-kl,-avx512bmm,-sha512,+avxvnni,-rtm,+adx,+avx2,-hreset,+movdiri,-serialize,+vpclmulqdq,+avx512vl,-uintr,-cf,-jmpabs,+clflushopt,-raoint,-cmpccxadd,+bmi,-amx-tile,+sse,+gfni,-avxvnniint16,-amx-fp16,-zu,-ndd,+xsaveopt,+rdrnd,+avx512f,-amx-bf16,+avx512bf16,+avx512vnni,-push2pop2,+cx8,+avx512bw,+sse3,+pku,-nf,-amx-avx512,+fsgsbase,+clzero,+mwaitx,-lwp,+lzcnt,+sha,+movdir64b,-ppx,+wbnoinvd,-enqcmd,-avxneconvert,-tbm,-pconfig,-amx-complex,+ssse3,+cx16,-avx10.2,+bmi2,+fma,+popcnt,-avxifma,+f16c,+avx512bitalg,+rdpru,+clwb,+mmx,+sse2,+rdseed,+avx512vbmi2,+prefetchi,+rdpid,-amx-movrs,-fma4,+avx512vbmi,+shstk,+vaes,-waitpkg,-sgx,+fxsr,+avx512dq,+sse4a" }
attributes #6 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #7 = { nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none) }
attributes #8 = { nounwind }
attributes #9 = { noinline noreturn }

!llvm.module.flags = !{!5, !6, !7}
!llvm.ident = !{!8}

!0 = !{i64 -9197586924146788638}
!1 = !{i64 -9216730992011529634}
!2 = !{i64 -5817022429948055995}
!3 = !{i64 -6118632917974911884}
!4 = !{i64 5778966726096123573}
!5 = !{i32 8, !"PIC Level", i32 2}
!6 = !{i32 2, !"RtLibUseGOT", i32 1}
!7 = !{i32 7, !"uwtable", i32 2}
!8 = !{!"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"}
!9 = !{i64 -82910376062180045}
!10 = !{}
!11 = !{i64 -660562422991372989}
!12 = !{i64 -4982135267746282809}
!13 = !{i64 6988135199322536488}
!14 = !{i64 1273439320875404183}
!15 = !{!16}
!16 = distinct !{!16, !17, !"_RINvMs2_NtNtCs7jcFBdfocI9_3std6thread5localINtB6_8LocalKeyINtNtCs2k2z8Zem4rB_4core4cell4CellNtCs6CxjjnbUHAu_12purrdf_stack9WalkStateEE4withNCNvXNvB1u_4walkNtB2i_5CloseNtNtNtBZ_3ops4drop4Drop4drop0B1s_EB1u_: argument 0"}
!17 = distinct !{!17, !"_RINvMs2_NtNtCs7jcFBdfocI9_3std6thread5localINtB6_8LocalKeyINtNtCs2k2z8Zem4rB_4core4cell4CellNtCs6CxjjnbUHAu_12purrdf_stack9WalkStateEE4withNCNvXNvB1u_4walkNtB2i_5CloseNtNtNtBZ_3ops4drop4Drop4drop0B1s_EB1u_"}
!18 = !{!19, !21, !16}
!19 = distinct !{!19, !20, !"_RNCNvXNvCs6CxjjnbUHAu_12purrdf_stack4walkNtB4_5CloseNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop0B6_: argument 0"}
!20 = distinct !{!20, !"_RNCNvXNvCs6CxjjnbUHAu_12purrdf_stack4walkNtB4_5CloseNtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4drop0B6_"}
!21 = distinct !{!21, !22, !"_RINvMs2_NtNtCs7jcFBdfocI9_3std6thread5localINtB6_8LocalKeyINtNtCs2k2z8Zem4rB_4core4cell4CellNtCs6CxjjnbUHAu_12purrdf_stack9WalkStateEE8try_withNCNvXNvB1u_4walkNtB2m_5CloseNtNtNtBZ_3ops4drop4Drop4drop0B1s_EB1u_: argument 0"}
!22 = distinct !{!22, !"_RINvMs2_NtNtCs7jcFBdfocI9_3std6thread5localINtB6_8LocalKeyINtNtCs2k2z8Zem4rB_4core4cell4CellNtCs6CxjjnbUHAu_12purrdf_stack9WalkStateEE8try_withNCNvXNvB1u_4walkNtB2m_5CloseNtNtNtBZ_3ops4drop4Drop4drop0B1s_EB1u_"}
!23 = !{!"branch_weights", i32 2000, i32 2, i32 2000}
