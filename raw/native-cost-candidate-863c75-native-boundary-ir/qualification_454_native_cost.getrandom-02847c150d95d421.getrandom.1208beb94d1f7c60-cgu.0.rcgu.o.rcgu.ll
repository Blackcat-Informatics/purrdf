; ModuleID = 'getrandom-02847c150d95d421.getrandom.1208beb94d1f7c60-cgu.0.rcgu.o'
source_filename = "getrandom.1208beb94d1f7c60-cgu.0"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

@_RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback12GETRANDOM_FN = local_unnamed_addr global [8 x i8] zeroinitializer, align 8, !guid !0
@_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file2FD = internal global [4 x i8] c"\FF\FF\FF\FF", align 4, !guid !1
@anon.c1f65b2e263de8b258326b6bc7df4cfe.0 = private unnamed_addr constant [13 x i8] c"/dev/urandom\00", align 1, !guid !2
@anon.c1f65b2e263de8b258326b6bc7df4cfe.1 = private unnamed_addr constant [12 x i8] c"/dev/random\00", align 1, !guid !3
@anon.c1f65b2e263de8b258326b6bc7df4cfe.2 = private unnamed_addr constant [10 x i8] c"getrandom\00", align 1, !guid !4
@anon.c1f65b2e263de8b258326b6bc7df4cfe.4 = private unnamed_addr constant [39 x i8] c"getrandom: this target is not supported", align 1, !guid !5
@anon.c1f65b2e263de8b258326b6bc7df4cfe.5 = private unnamed_addr constant [38 x i8] c"errno: did not return a positive value", align 1, !guid !6
@anon.c1f65b2e263de8b258326b6bc7df4cfe.6 = private unnamed_addr constant [20 x i8] c"unexpected situation", align 1, !guid !7
@anon.c1f65b2e263de8b258326b6bc7df4cfe.8 = private unnamed_addr constant [5 x i8] c"Error", align 1, !guid !8
@anon.c1f65b2e263de8b258326b6bc7df4cfe.9 = private unnamed_addr constant <{ [24 x i8], ptr }> <{ [24 x i8] c"\00\00\00\00\00\00\00\00\04\00\00\00\00\00\00\00\04\00\00\00\00\00\00\00", ptr @_RNvXsQ_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_5Debug3fmt }>, align 8, !guid !9
@anon.c1f65b2e263de8b258326b6bc7df4cfe.10 = private unnamed_addr constant [8 x i8] c"os_error", align 1, !guid !10
@anon.c1f65b2e263de8b258326b6bc7df4cfe.11 = private unnamed_addr constant [13 x i8] c"internal_code", align 1, !guid !11
@anon.c1f65b2e263de8b258326b6bc7df4cfe.12 = private unnamed_addr constant <{ [24 x i8], ptr }> <{ [24 x i8] c"\00\00\00\00\00\00\00\00\10\00\00\00\00\00\00\00\08\00\00\00\00\00\00\00", ptr @_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtReNtB6_5Debug3fmtCs1xZImt19Kx0_9getrandom }>, align 8, !guid !12
@anon.c1f65b2e263de8b258326b6bc7df4cfe.13 = private unnamed_addr constant [11 x i8] c"description", align 1, !guid !13
@anon.c1f65b2e263de8b258326b6bc7df4cfe.14 = private unnamed_addr constant [12 x i8] c"unknown_code", align 1, !guid !14
@anon.a647af78948fdeb0321157bdb82ef7e0.2.llvm.9794848731438112354 = external hidden unnamed_addr constant [16 x i8], align 1, !guid !15
@anon.a647af78948fdeb0321157bdb82ef7e0.37.llvm.9794848731438112354 = external hidden unnamed_addr constant [1 x i8], align 1, !guid !16
@anon.a647af78948fdeb0321157bdb82ef7e0.91.llvm.9794848731438112354 = external hidden unnamed_addr constant [2 x i8], align 1, !guid !17
@anon.a647af78948fdeb0321157bdb82ef7e0.394.llvm.9794848731438112354 = external hidden unnamed_addr constant [2 x i8], align 1, !guid !18
@anon.a647af78948fdeb0321157bdb82ef7e0.408.llvm.9794848731438112354 = external hidden unnamed_addr constant [16 x i8], align 1, !guid !19
@switch.table._RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.rel = private unnamed_addr constant [3 x i32] [i32 trunc (i64 sub (i64 ptrtoint (ptr @anon.c1f65b2e263de8b258326b6bc7df4cfe.4 to i64), i64 ptrtoint (ptr @switch.table._RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.rel to i64)) to i32), i32 trunc (i64 sub (i64 ptrtoint (ptr @anon.c1f65b2e263de8b258326b6bc7df4cfe.5 to i64), i64 ptrtoint (ptr @switch.table._RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.rel to i64)) to i32), i32 trunc (i64 sub (i64 ptrtoint (ptr @anon.c1f65b2e263de8b258326b6bc7df4cfe.6 to i64), i64 ptrtoint (ptr @switch.table._RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.rel to i64)) to i32)], align 4, !guid !20
@switch.table._RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.11 = private unnamed_addr constant [3 x i8] c"'&\14", align 8, !guid !21

; getrandom::backends::linux_android_with_fallback::use_file_fallback
; Function Attrs: noinline nounwind nonlazybind uwtable
define noundef i32 @_RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback17use_file_fallback(ptr noalias nofree noundef nonnull captures(none) %0, i64 noundef range(i64 0, -9223372036854775808) %1) unnamed_addr #0 personality ptr @rust_eh_personality !guid !31 {
  %3 = load atomic i32, ptr @_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file2FD acquire, align 4, !noalias !32
  %4 = icmp ugt i32 %3, -3
  br i1 %4, label %5, label %10, !prof !35

5:                                                ; preds = %2
; call getrandom::backends::use_file::open_or_wait
  %6 = tail call fastcc { i32, i32 } @_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file12open_or_wait() #12, !noalias !32
  %7 = extractvalue { i32, i32 } %6, 0
  %8 = extractvalue { i32, i32 } %6, 1
  %9 = trunc nuw i32 %7 to i1
  br i1 %9, label %.loopexit, label %10

10:                                               ; preds = %5, %2
  %11 = phi i32 [ %3, %2 ], [ %8, %5 ]
  %12 = icmp eq i64 %1, 0
  br i1 %12, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %10, %29
  %13 = phi ptr [ %31, %29 ], [ %0, %10 ]
  %14 = phi i64 [ %30, %29 ], [ %1, %10 ]
  %15 = tail call noundef i64 @read(i32 noundef %11, ptr noundef nonnull %13, i64 noundef range(i64 1, -9223372036854775808) %14) #13
  %16 = icmp sgt i64 %15, 0
  br i1 %16, label %19, label %17

17:                                               ; preds = %.preheader
  %18 = icmp eq i64 %15, -1
  br i1 %18, label %21, label %.loopexit

19:                                               ; preds = %.preheader
  %20 = icmp ult i64 %14, %15
  br i1 %20, label %.loopexit, label %33

21:                                               ; preds = %17
  %22 = tail call noundef ptr @__errno_location() #13, !noalias !36
  %23 = load i32, ptr %22, align 4, !noalias !36, !noundef !39
  %24 = icmp sgt i32 %23, 0
  %25 = sub nsw i32 0, %23
  %26 = select i1 %24, i32 %25, i32 65537
  %27 = icmp ne i32 %26, 0
  tail call void @llvm.assume(i1 %27)
  %28 = icmp eq i32 %26, -4
  br i1 %28, label %29, label %.loopexit, !prof !40

29:                                               ; preds = %33, %21
  %30 = phi i64 [ %34, %33 ], [ %14, %21 ]
  %31 = phi ptr [ %35, %33 ], [ %13, %21 ]
  %32 = icmp eq i64 %30, 0
  br i1 %32, label %.loopexit, label %.preheader

33:                                               ; preds = %19
  %34 = sub nuw nsw i64 %14, %15
  %35 = getelementptr inbounds nuw i8, ptr %13, i64 %15
  br label %29

.loopexit:                                        ; preds = %29, %21, %19, %17, %10, %5
  %36 = phi i32 [ %8, %5 ], [ 0, %10 ], [ 0, %29 ], [ %26, %21 ], [ 65538, %19 ], [ 65538, %17 ]
  ret i32 %36
}

; getrandom::backends::linux_android_with_fallback::init
; Function Attrs: cold noinline nounwind nonlazybind uwtable
define noundef nonnull ptr @_RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback4init() unnamed_addr #1 !guid !41 {
  %1 = tail call noundef ptr @dlsym(ptr noundef null, ptr noundef nonnull @anon.c1f65b2e263de8b258326b6bc7df4cfe.2) #13
  %2 = icmp eq ptr %1, null
  br i1 %2, label %16, label %3

3:                                                ; preds = %0
  %4 = tail call noundef i64 %1(ptr noundef nonnull inttoptr (i64 1 to ptr), i64 noundef 0, i32 noundef 0) #13
  %5 = icmp slt i64 %4, 0
  br i1 %5, label %6, label %16

6:                                                ; preds = %3
  %7 = tail call noundef ptr @__errno_location() #13
  %8 = load i32, ptr %7, align 4, !noundef !39
  %9 = icmp sgt i32 %8, 0
  %10 = sub nsw i32 0, %8
  %11 = select i1 %9, i32 %10, i32 65537
  %12 = icmp ne i32 %11, 0
  tail call void @llvm.assume(i1 %12)
  %13 = icmp sgt i32 %11, -1
  br i1 %13, label %16, label %14, !prof !42

14:                                               ; preds = %6
  switch i32 %11, label %16 [
    i32 -38, label %15
    i32 -1, label %15
  ]

15:                                               ; preds = %14, %14
  br label %16

16:                                               ; preds = %15, %14, %6, %3, %0
  %17 = phi ptr [ %1, %3 ], [ inttoptr (i64 -1 to ptr), %0 ], [ %1, %6 ], [ %1, %14 ], [ inttoptr (i64 -1 to ptr), %15 ]
  store atomic ptr %17, ptr @_RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback12GETRANDOM_FN release, align 8
  ret ptr %17
}

; getrandom::backends::use_file::open_or_wait
; Function Attrs: cold noinline nounwind nonlazybind uwtable
define internal fastcc { i32, i32 } @_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file12open_or_wait() unnamed_addr #1 !guid !43 {
  %1 = alloca [8 x i8], align 4
  br label %2

2:                                                ; preds = %.backedge, %0
  %3 = load atomic i32, ptr @_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file2FD acquire, align 4
  switch i32 %3, label %.loopexit5 [
    i32 -1, label %4
    i32 -2, label %7
  ]

4:                                                ; preds = %2
  %5 = cmpxchg weak ptr @_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file2FD, i32 -1, i32 -2 acq_rel monotonic, align 4
  %6 = extractvalue { i32, i1 } %5, 1
  br i1 %6, label %.preheader, label %.backedge

7:                                                ; preds = %2
  %8 = tail call noundef i64 (i64, ...) @syscall(i64 noundef 202, ptr noundef nonnull align 4 @_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file2FD, i32 noundef 128, i32 noundef -2, ptr noundef null) #13
  br label %.backedge

.backedge:                                        ; preds = %7, %4
  br label %2

.preheader:                                       ; preds = %4, %11
  %9 = tail call noundef i32 (ptr, i32, ...) @open(ptr noundef nonnull @anon.c1f65b2e263de8b258326b6bc7df4cfe.1, i32 noundef 524288) #13
  %10 = icmp sgt i32 %9, -1
  br i1 %10, label %19, label %11

11:                                               ; preds = %.preheader
  %12 = tail call noundef ptr @__errno_location() #13, !noalias !44
  %13 = load i32, ptr %12, align 4, !noalias !44, !noundef !39
  %14 = icmp sgt i32 %13, 0
  %15 = sub nsw i32 0, %13
  %16 = select i1 %14, i32 %15, i32 65537
  %17 = icmp ne i32 %16, 0
  tail call void @llvm.assume(i1 %17)
  %18 = icmp eq i32 %16, -4
  br i1 %18, label %.preheader, label %.loopexit, !prof !40

19:                                               ; preds = %.preheader
  call void @llvm.lifetime.start.p0(ptr nonnull %1)
  store i32 %9, ptr %1, align 4
  %20 = getelementptr inbounds nuw i8, ptr %1, i64 4
  store i16 1, ptr %20, align 4
  %21 = getelementptr inbounds nuw i8, ptr %1, i64 6
  store i16 0, ptr %21, align 2
  br label %22

22:                                               ; preds = %25, %19
  %23 = call noundef i32 @poll(ptr noundef nonnull %1, i64 noundef 1, i32 noundef -1) #13
  %24 = icmp sgt i32 %23, -1
  br i1 %24, label %35, label %25

25:                                               ; preds = %22
  %26 = call noundef ptr @__errno_location() #13
  %27 = load i32, ptr %26, align 4, !noundef !39
  %28 = icmp sgt i32 %27, 0
  %29 = sub nsw i32 0, %27
  %30 = select i1 %28, i32 %29, i32 65537
  %31 = icmp ne i32 %30, 0
  call void @llvm.assume(i1 %31)
  %32 = icmp eq i32 %30, -4
  br i1 %32, label %22, label %33, !prof !40

33:                                               ; preds = %25
  %34 = call noundef i32 @close(i32 noundef %9) #13
  call void @llvm.lifetime.end.p0(ptr nonnull %1)
  br label %.loopexit

35:                                               ; preds = %22
  %36 = call noundef i32 @close(i32 noundef %9) #13
  call void @llvm.lifetime.end.p0(ptr nonnull %1)
  br label %37

37:                                               ; preds = %40, %35
  %38 = call noundef i32 (ptr, i32, ...) @open(ptr noundef nonnull @anon.c1f65b2e263de8b258326b6bc7df4cfe.0, i32 noundef 524288) #13
  %39 = icmp slt i32 %38, 0
  br i1 %39, label %40, label %48

40:                                               ; preds = %37
  %41 = call noundef ptr @__errno_location() #13, !noalias !47
  %42 = load i32, ptr %41, align 4, !noalias !47, !noundef !39
  %43 = icmp sgt i32 %42, 0
  %44 = sub nsw i32 0, %42
  %45 = select i1 %43, i32 %44, i32 65537
  %46 = icmp ne i32 %45, 0
  call void @llvm.assume(i1 %46)
  %47 = icmp eq i32 %45, -4
  br i1 %47, label %37, label %48, !prof !40

48:                                               ; preds = %40, %37
  %49 = phi i32 [ %38, %37 ], [ %45, %40 ]
  %50 = lshr i32 %38, 31
  br label %.loopexit

.loopexit:                                        ; preds = %11, %48, %33
  %51 = phi i32 [ %30, %33 ], [ %49, %48 ], [ %16, %11 ]
  %52 = phi i32 [ 1, %33 ], [ %50, %48 ], [ 1, %11 ]
  %53 = insertvalue { i32, i32 } poison, i32 %52, 0
  %54 = trunc nuw i32 %52 to i1
  %55 = select i1 %54, i32 -1, i32 %51
  store atomic i32 %55, ptr @_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file2FD release, align 4
  %56 = call noundef i64 (i64, ...) @syscall(i64 noundef 202, ptr noundef nonnull align 4 @_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file2FD, i32 noundef 129, i32 noundef 2147483647) #13
  br label %.loopexit5

.loopexit5:                                       ; preds = %2, %.loopexit
  %57 = phi { i32, i32 } [ %53, %.loopexit ], [ { i32 0, i32 poison }, %2 ]
  %58 = phi i32 [ %51, %.loopexit ], [ %3, %2 ]
  %59 = insertvalue { i32, i32 } %57, i32 %58, 1
  ret { i32, i32 } %59
}

; getrandom::backends::use_file::util_libc::last_os_error
; Function Attrs: nounwind nonlazybind uwtable
define noundef range(i32 -2147483647, 65538) i32 @_RNvNtNtNtCs1xZImt19Kx0_9getrandom8backends8use_file9util_libc13last_os_error() unnamed_addr #2 !guid !50 {
  %1 = tail call noundef ptr @__errno_location() #13
  %2 = load i32, ptr %1, align 4, !noundef !39
  %3 = icmp sgt i32 %2, 0
  %4 = sub nsw i32 0, %2
  %5 = select i1 %3, i32 %4, i32 65537
  ret i32 %5
}

; <&str as core::fmt::Debug>::fmt
; Function Attrs: nonlazybind uwtable
define internal noundef zeroext i1 @_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtReNtB6_5Debug3fmtCs1xZImt19Kx0_9getrandom(ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(16) %0, ptr noalias nofree noundef align 8 dereferenceable(24) %1) unnamed_addr #3 !guid !51 {
  %3 = load ptr, ptr %0, align 8, !nonnull !39, !noundef !39
  %4 = getelementptr inbounds nuw i8, ptr %0, i64 8
  %5 = load i64, ptr %4, align 8, !noundef !39
; call <str as core::fmt::Debug>::fmt
  %6 = tail call noundef zeroext i1 @_RNvXsh_NtCs2k2z8Zem4rB_4core3fmteNtB5_5Debug3fmt(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %3, i64 noundef %5, ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %1)
  ret i1 %6
}

; <i32 as core::fmt::Debug>::fmt
; Function Attrs: inlinehint nonlazybind uwtable
define internal noundef zeroext i1 @_RNvXsQ_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_5Debug3fmt(ptr noalias nofree noundef readonly align 4 captures(none) dereferenceable(4) %0, ptr noalias nofree noundef align 8 captures(none) dereferenceable(24) %1) unnamed_addr #4 !guid !52 {
  %3 = alloca [8 x i8], align 1
  %4 = alloca [8 x i8], align 1
  %5 = getelementptr inbounds nuw i8, ptr %1, i64 16
  %6 = load i32, ptr %5, align 8, !noundef !39
  %7 = and i32 %6, 33554432
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %9, label %12

9:                                                ; preds = %2
  %10 = and i32 %6, 67108864
  %11 = icmp eq i32 %10, 0
  br i1 %11, label %28, label %30

12:                                               ; preds = %2
  tail call void @llvm.experimental.noalias.scope.decl(metadata !53)
  %13 = load i32, ptr %0, align 4, !dbg !56, !alias.scope !53, !noalias !64, !noundef !39
  call void @llvm.lifetime.start.p0(ptr nonnull %4), !dbg !66, !noalias !70
  br label %14, !dbg !74

14:                                               ; preds = %14, %12
  %15 = phi i32 [ %13, %12 ], [ %17, %14 ], !dbg !78
  %16 = phi i64 [ 8, %12 ], [ %18, %14 ], !dbg !79
  %17 = lshr i32 %15, 4, !dbg !80
  %18 = add nsw i64 %16, -1, !dbg !82
  %19 = and i32 %15, 15, !dbg !83
  %20 = zext nneg i32 %19 to i64, !dbg !84
  %21 = getelementptr inbounds nuw i8, ptr %4, i64 %18, !dbg !85
  %22 = getelementptr inbounds nuw i8, ptr @anon.a647af78948fdeb0321157bdb82ef7e0.2.llvm.9794848731438112354, i64 %20, !dbg !86
  %23 = load i8, ptr %22, align 1, !dbg !86, !noalias !70, !noundef !39
  store i8 %23, ptr %21, align 1, !dbg !87, !noalias !70
  %24 = icmp eq i32 %17, 0, !dbg !94
  br i1 %24, label %_RNvXsv_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_8LowerHex3fmt.exit, label %14, !dbg !94

_RNvXsv_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_8LowerHex3fmt.exit: ; preds = %14
  %25 = getelementptr inbounds nuw i8, ptr %4, i64 %18
  %26 = sub nsw i64 9, %16, !dbg !95
; call <core::fmt::Formatter>::pad_integral
  %27 = call noundef zeroext i1 @_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter12pad_integral(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %1, i1 noundef zeroext true, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a647af78948fdeb0321157bdb82ef7e0.394.llvm.9794848731438112354, i64 noundef 2, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %25, i64 noundef %26), !dbg !111, !noalias !113
  call void @llvm.lifetime.end.p0(ptr nonnull %4), !dbg !114, !noalias !70
  br label %46

28:                                               ; preds = %9
; call <i32 as core::fmt::Display>::fmt
  %29 = tail call noundef zeroext i1 @_RNvXs9_NtNtNtCs2k2z8Zem4rB_4core3fmt3num3implNtB9_7Display3fmt(ptr noalias nofree noundef nonnull readonly align 4 captures(address, read_provenance) dereferenceable(4) %0, ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %1)
  br label %46

30:                                               ; preds = %9
  tail call void @llvm.experimental.noalias.scope.decl(metadata !115)
  %31 = load i32, ptr %0, align 4, !dbg !118, !alias.scope !115, !noalias !121, !noundef !39
  call void @llvm.lifetime.start.p0(ptr nonnull %3), !dbg !123, !noalias !127
  br label %32, !dbg !131

32:                                               ; preds = %32, %30
  %33 = phi i32 [ %31, %30 ], [ %35, %32 ], !dbg !135
  %34 = phi i64 [ 8, %30 ], [ %36, %32 ], !dbg !136
  %35 = lshr i32 %33, 4, !dbg !137
  %36 = add nsw i64 %34, -1, !dbg !139
  %37 = and i32 %33, 15, !dbg !140
  %38 = zext nneg i32 %37 to i64, !dbg !141
  %39 = getelementptr inbounds nuw i8, ptr %3, i64 %36, !dbg !142
  %40 = getelementptr inbounds nuw i8, ptr @anon.a647af78948fdeb0321157bdb82ef7e0.408.llvm.9794848731438112354, i64 %38, !dbg !143
  %41 = load i8, ptr %40, align 1, !dbg !143, !noalias !127, !noundef !39
  store i8 %41, ptr %39, align 1, !dbg !144, !noalias !127
  %42 = icmp eq i32 %35, 0, !dbg !147
  br i1 %42, label %_RNvXsx_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_8UpperHex3fmt.exit, label %32, !dbg !147

_RNvXsx_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_8UpperHex3fmt.exit: ; preds = %32
  %43 = getelementptr inbounds nuw i8, ptr %3, i64 %36
  %44 = sub nsw i64 9, %34, !dbg !148
; call <core::fmt::Formatter>::pad_integral
  %45 = call noundef zeroext i1 @_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter12pad_integral(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %1, i1 noundef zeroext true, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a647af78948fdeb0321157bdb82ef7e0.394.llvm.9794848731438112354, i64 noundef 2, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %43, i64 noundef %44), !dbg !153, !noalias !155
  call void @llvm.lifetime.end.p0(ptr nonnull %3), !dbg !156, !noalias !127
  br label %46

46:                                               ; preds = %_RNvXsx_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_8UpperHex3fmt.exit, %28, %_RNvXsv_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_8LowerHex3fmt.exit
  %47 = phi i1 [ %27, %_RNvXsv_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_8LowerHex3fmt.exit ], [ %45, %_RNvXsx_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_8UpperHex3fmt.exit ], [ %29, %28 ]
  ret i1 %47
}

; <getrandom::error::Error as core::fmt::Debug>::fmt
; Function Attrs: nonlazybind uwtable
define noundef zeroext i1 @_RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt(ptr noalias nofree noundef readonly align 4 captures(none) dereferenceable(4) %0, ptr noalias nofree noundef align 8 dereferenceable(24) %1) unnamed_addr #3 !guid !157 {
  %3 = alloca [4 x i8], align 4
  %4 = alloca [4 x i8], align 4
  %5 = alloca [16 x i8], align 8
  %6 = alloca [4 x i8], align 4
  %7 = alloca [16 x i8], align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %7)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !158)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !161)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !163), !dbg !166
  tail call void @llvm.experimental.noalias.scope.decl(metadata !170), !dbg !166
  tail call void @llvm.experimental.noalias.scope.decl(metadata !172), !dbg !175
  %8 = load ptr, ptr %1, align 8, !dbg !180, !alias.scope !183, !noalias !184, !nonnull !39, !noundef !39
  %9 = getelementptr inbounds nuw i8, ptr %1, i64 8, !dbg !180
  %10 = load ptr, ptr %9, align 8, !dbg !180, !alias.scope !183, !noalias !184, !nonnull !39, !align !188, !noundef !39
  %11 = getelementptr inbounds nuw i8, ptr %10, i64 24, !dbg !180
  %12 = load ptr, ptr %11, align 8, !dbg !180, !invariant.load !39, !noalias !189, !nonnull !39
  %13 = tail call noundef zeroext i1 %12(ptr noundef nonnull %8, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.c1f65b2e263de8b258326b6bc7df4cfe.8, i64 noundef 5) #14, !dbg !190, !noalias !191, !inline_history !192
  store ptr %1, ptr %7, align 8, !dbg !193, !alias.scope !195, !noalias !196
  %14 = getelementptr inbounds nuw i8, ptr %7, i64 8, !dbg !193
  %15 = zext i1 %13 to i8, !dbg !193
  store i8 %15, ptr %14, align 8, !dbg !193, !alias.scope !195, !noalias !196
  %16 = getelementptr inbounds nuw i8, ptr %7, i64 9, !dbg !193
  store i8 0, ptr %16, align 1, !dbg !193, !alias.scope !195, !noalias !196
  %17 = load i32, ptr %0, align 4, !range !197, !noundef !39
  %18 = icmp ult i32 %17, -2147483647
  br i1 %18, label %22, label %19, !prof !42

19:                                               ; preds = %2
  %20 = sub nsw i32 0, %17
  call void @llvm.lifetime.start.p0(ptr nonnull %6)
  store i32 %20, ptr %6, align 4
; call <core::fmt::builders::DebugStruct>::field
  %21 = call noundef nonnull align 8 ptr @_RNvMs2_NtNtCs2k2z8Zem4rB_4core3fmt8buildersNtB5_11DebugStruct5field(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %7, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.c1f65b2e263de8b258326b6bc7df4cfe.10, i64 noundef 8, ptr noundef nonnull %6, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(32) @anon.c1f65b2e263de8b258326b6bc7df4cfe.9)
  call void @llvm.lifetime.end.p0(ptr nonnull %6)
  br label %24

22:                                               ; preds = %2
  %switch.tableidx = add i32 %17, -65536
  %23 = icmp ult i32 %switch.tableidx, 3
  br i1 %23, label %switch.lookup, label %54

24:                                               ; preds = %54, %switch.lookup, %19
  call void @llvm.experimental.noalias.scope.decl(metadata !198)
  %25 = load i8, ptr %16, align 1, !dbg !201, !range !204, !alias.scope !198, !noundef !39
  %26 = trunc nuw i8 %25 to i1, !dbg !201
  %27 = load i8, ptr %14, align 8, !dbg !205, !range !204, !alias.scope !198
  %28 = trunc nuw i8 %27 to i1, !dbg !205
  %.not = xor i1 %26, true, !dbg !201
  %brmerge = select i1 %.not, i1 true, i1 %28, !dbg !201
  %.mux = select i1 %26, i1 true, i1 %28, !dbg !201
  br i1 %brmerge, label %_RNvMs2_NtNtCs2k2z8Zem4rB_4core3fmt8buildersNtB5_11DebugStruct6finish.exit, label %29, !dbg !201

29:                                               ; preds = %24
  %30 = load ptr, ptr %7, align 8, !dbg !206, !alias.scope !198, !nonnull !39, !align !188, !noundef !39
  %31 = getelementptr inbounds nuw i8, ptr %30, i64 16, !dbg !219
  %32 = load i32, ptr %31, align 8, !dbg !219, !noalias !198, !noundef !39
  %33 = and i32 %32, 8388608, !dbg !219
  %34 = icmp eq i32 %33, 0, !dbg !219
  %35 = getelementptr inbounds nuw i8, ptr %30, i64 8, !dbg !222
  br i1 %34, label %36, label %42, !dbg !224

36:                                               ; preds = %29
  call void @llvm.experimental.noalias.scope.decl(metadata !225), !dbg !228
  %37 = load ptr, ptr %30, align 8, !dbg !229, !alias.scope !225, !noalias !231, !nonnull !39, !noundef !39
  %38 = load ptr, ptr %35, align 8, !dbg !229, !alias.scope !225, !noalias !231, !nonnull !39, !align !188, !noundef !39
  %39 = getelementptr inbounds nuw i8, ptr %38, i64 24, !dbg !229
  %40 = load ptr, ptr %39, align 8, !dbg !229, !invariant.load !39, !noalias !233, !nonnull !39
  %41 = call noundef zeroext i1 %40(ptr noundef nonnull %37, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a647af78948fdeb0321157bdb82ef7e0.91.llvm.9794848731438112354, i64 noundef 2) #14, !dbg !234, !noalias !235, !inline_history !236
  br label %_RNvMs2_NtNtCs2k2z8Zem4rB_4core3fmt8buildersNtB5_11DebugStruct6finish.exit, !dbg !228

42:                                               ; preds = %29
  call void @llvm.experimental.noalias.scope.decl(metadata !237), !dbg !240
  %43 = load ptr, ptr %30, align 8, !dbg !241, !alias.scope !237, !noalias !243, !nonnull !39, !noundef !39
  %44 = load ptr, ptr %35, align 8, !dbg !241, !alias.scope !237, !noalias !243, !nonnull !39, !align !188, !noundef !39
  %45 = getelementptr inbounds nuw i8, ptr %44, i64 24, !dbg !241
  %46 = load ptr, ptr %45, align 8, !dbg !241, !invariant.load !39, !noalias !245, !nonnull !39
  %47 = call noundef zeroext i1 %46(ptr noundef nonnull %43, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a647af78948fdeb0321157bdb82ef7e0.37.llvm.9794848731438112354, i64 noundef 1) #14, !dbg !246, !noalias !247, !inline_history !236
  br label %_RNvMs2_NtNtCs2k2z8Zem4rB_4core3fmt8buildersNtB5_11DebugStruct6finish.exit, !dbg !240

_RNvMs2_NtNtCs2k2z8Zem4rB_4core3fmt8buildersNtB5_11DebugStruct6finish.exit: ; preds = %24, %36, %42
  %48 = phi i1 [ %.mux, %24 ], [ %41, %36 ], [ %47, %42 ], !dbg !248
  call void @llvm.lifetime.end.p0(ptr nonnull %7)
  ret i1 %48

switch.lookup:                                    ; preds = %22
  %49 = zext nneg i32 %switch.tableidx to i64
  %reltable.shift = shl i64 %49, 2
  %reltable.intrinsic = call ptr @llvm.load.relative.i64(ptr @switch.table._RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.rel, i64 %reltable.shift)
  %50 = zext nneg i32 %switch.tableidx to i64
  %switch.gep1 = getelementptr inbounds nuw i8, ptr @switch.table._RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.11, i64 %50
  %switch.load2 = load i8, ptr %switch.gep1, align 1
  %switch.ext = zext i8 %switch.load2 to i64
  call void @llvm.lifetime.start.p0(ptr nonnull %5)
  store ptr %reltable.intrinsic, ptr %5, align 8, !captures !249
  %51 = getelementptr inbounds nuw i8, ptr %5, i64 8
  store i64 %switch.ext, ptr %51, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %4)
  store i32 %17, ptr %4, align 4
; call <core::fmt::builders::DebugStruct>::field
  %52 = call noundef nonnull align 8 ptr @_RNvMs2_NtNtCs2k2z8Zem4rB_4core3fmt8buildersNtB5_11DebugStruct5field(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %7, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.c1f65b2e263de8b258326b6bc7df4cfe.11, i64 noundef 13, ptr noundef nonnull %4, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(32) @anon.c1f65b2e263de8b258326b6bc7df4cfe.9)
  call void @llvm.lifetime.end.p0(ptr nonnull %4)
; call <core::fmt::builders::DebugStruct>::field
  %53 = call noundef nonnull align 8 ptr @_RNvMs2_NtNtCs2k2z8Zem4rB_4core3fmt8buildersNtB5_11DebugStruct5field(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %7, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.c1f65b2e263de8b258326b6bc7df4cfe.13, i64 noundef 11, ptr noundef nonnull %5, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(32) @anon.c1f65b2e263de8b258326b6bc7df4cfe.12)
  call void @llvm.lifetime.end.p0(ptr nonnull %5)
  br label %24

54:                                               ; preds = %22
  call void @llvm.lifetime.start.p0(ptr nonnull %3)
  store i32 %17, ptr %3, align 4
; call <core::fmt::builders::DebugStruct>::field
  %55 = call noundef nonnull align 8 ptr @_RNvMs2_NtNtCs2k2z8Zem4rB_4core3fmt8buildersNtB5_11DebugStruct5field(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %7, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.c1f65b2e263de8b258326b6bc7df4cfe.14, i64 noundef 12, ptr noundef nonnull %3, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(32) @anon.c1f65b2e263de8b258326b6bc7df4cfe.9)
  call void @llvm.lifetime.end.p0(ptr nonnull %3)
  br label %24
}

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: write)
declare void @llvm.assume(i1 noundef) #5

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.start.p0(ptr captures(none)) #6

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.end.p0(ptr captures(none)) #6

; Function Attrs: nonlazybind
declare i32 @rust_eh_personality(...) unnamed_addr #7

; Function Attrs: nofree nounwind nonlazybind uwtable
declare noundef i64 @read(i32 noundef, ptr noundef captures(none), i64 noundef) unnamed_addr #8

; Function Attrs: nounwind nonlazybind uwtable
declare noundef ptr @dlsym(ptr noundef, ptr noundef) unnamed_addr #2

; Function Attrs: nofree nounwind nonlazybind uwtable
declare noundef i32 @open(ptr noundef readonly captures(none), i32 noundef, ...) unnamed_addr #8

; Function Attrs: nounwind nonlazybind uwtable
declare noundef i32 @poll(ptr noundef, i64 noundef, i32 noundef) unnamed_addr #2

; Function Attrs: nounwind nonlazybind uwtable
declare noundef i32 @close(i32 noundef) unnamed_addr #2

; Function Attrs: nounwind nonlazybind uwtable
declare noundef i64 @syscall(i64 noundef, ...) unnamed_addr #2

; Function Attrs: nounwind nonlazybind uwtable
declare noundef ptr @__errno_location() unnamed_addr #2

; <str as core::fmt::Debug>::fmt
; Function Attrs: nonlazybind uwtable
declare noundef zeroext i1 @_RNvXsh_NtCs2k2z8Zem4rB_4core3fmteNtB5_5Debug3fmt(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance), i64 noundef, ptr noalias nofree noundef align 8 dereferenceable(24)) unnamed_addr #3

; <core::fmt::builders::DebugStruct>::field
; Function Attrs: nonlazybind uwtable
declare noundef nonnull align 8 ptr @_RNvMs2_NtNtCs2k2z8Zem4rB_4core3fmt8buildersNtB5_11DebugStruct5field(ptr noalias nofree noundef align 8 dereferenceable(16), ptr noalias nofree noundef nonnull readonly captures(address, read_provenance), i64 noundef, ptr noundef nonnull, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(32)) unnamed_addr #3

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: readwrite)
declare void @llvm.experimental.noalias.scope.decl(metadata) #9

; <i32 as core::fmt::Display>::fmt
; Function Attrs: nonlazybind uwtable
declare noundef zeroext i1 @_RNvXs9_NtNtNtCs2k2z8Zem4rB_4core3fmt3num3implNtB9_7Display3fmt(ptr noalias nofree noundef readonly align 4 captures(none) dereferenceable(4), ptr noalias nofree noundef align 8 captures(none) dereferenceable(24)) unnamed_addr #10

; <core::fmt::Formatter>::pad_integral
; Function Attrs: nonlazybind uwtable
declare noundef zeroext i1 @_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter12pad_integral(ptr noalias nofree noundef align 8 captures(none) dereferenceable(24), i1 noundef zeroext, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance), i64 noundef, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance), i64 noundef) unnamed_addr #10

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(argmem: read)
declare ptr @llvm.load.relative.i64(ptr, i64) #11

attributes #0 = { noinline nounwind nonlazybind uwtable "probe-stack"="inline-asm" "target-cpu"="znver5" "target-features"="+prfchw,-cldemote,+avx,+aes,+sahf,+pclmul,-xop,+crc32,-amx-fp8,+xsaves,-avx512fp16,-usermsr,-sm4,-egpr,+sse4.1,-avx10.1,+avx512ifma,+xsave,+sse4.2,-tsxldtrk,-sm3,-ptwrite,-widekl,-movrs,+invpcid,+64bit,+xsavec,+avx512vpopcntdq,+cmov,+avx512vp2intersect,-avxvnniint8,+avx512cd,+movbe,-ccmp,-amx-int8,-kl,-avx512bmm,-sha512,+avxvnni,-rtm,+adx,+avx2,-hreset,+movdiri,-serialize,+vpclmulqdq,+avx512vl,-uintr,-cf,-jmpabs,+clflushopt,-raoint,-cmpccxadd,+bmi,-amx-tile,+sse,+gfni,-avxvnniint16,-amx-fp16,-zu,-ndd,+xsaveopt,+rdrnd,+avx512f,-amx-bf16,+avx512bf16,+avx512vnni,-push2pop2,+cx8,+avx512bw,+sse3,+pku,-nf,-amx-avx512,+fsgsbase,+clzero,+mwaitx,-lwp,+lzcnt,+sha,+movdir64b,-ppx,+wbnoinvd,-enqcmd,-avxneconvert,-tbm,-pconfig,-amx-complex,+ssse3,+cx16,-avx10.2,+bmi2,+fma,+popcnt,-avxifma,+f16c,+avx512bitalg,+rdpru,+clwb,+mmx,+sse2,+rdseed,+avx512vbmi2,+prefetchi,+rdpid,-amx-movrs,-fma4,+avx512vbmi,+shstk,+vaes,-waitpkg,-sgx,+fxsr,+avx512dq,+sse4a" }
attributes #1 = { cold noinline nounwind nonlazybind uwtable "probe-stack"="inline-asm" "target-cpu"="znver5" "target-features"="+prfchw,-cldemote,+avx,+aes,+sahf,+pclmul,-xop,+crc32,-amx-fp8,+xsaves,-avx512fp16,-usermsr,-sm4,-egpr,+sse4.1,-avx10.1,+avx512ifma,+xsave,+sse4.2,-tsxldtrk,-sm3,-ptwrite,-widekl,-movrs,+invpcid,+64bit,+xsavec,+avx512vpopcntdq,+cmov,+avx512vp2intersect,-avxvnniint8,+avx512cd,+movbe,-ccmp,-amx-int8,-kl,-avx512bmm,-sha512,+avxvnni,-rtm,+adx,+avx2,-hreset,+movdiri,-serialize,+vpclmulqdq,+avx512vl,-uintr,-cf,-jmpabs,+clflushopt,-raoint,-cmpccxadd,+bmi,-amx-tile,+sse,+gfni,-avxvnniint16,-amx-fp16,-zu,-ndd,+xsaveopt,+rdrnd,+avx512f,-amx-bf16,+avx512bf16,+avx512vnni,-push2pop2,+cx8,+avx512bw,+sse3,+pku,-nf,-amx-avx512,+fsgsbase,+clzero,+mwaitx,-lwp,+lzcnt,+sha,+movdir64b,-ppx,+wbnoinvd,-enqcmd,-avxneconvert,-tbm,-pconfig,-amx-complex,+ssse3,+cx16,-avx10.2,+bmi2,+fma,+popcnt,-avxifma,+f16c,+avx512bitalg,+rdpru,+clwb,+mmx,+sse2,+rdseed,+avx512vbmi2,+prefetchi,+rdpid,-amx-movrs,-fma4,+avx512vbmi,+shstk,+vaes,-waitpkg,-sgx,+fxsr,+avx512dq,+sse4a" }
attributes #2 = { nounwind nonlazybind uwtable "probe-stack"="inline-asm" "target-cpu"="znver5" "target-features"="+prfchw,-cldemote,+avx,+aes,+sahf,+pclmul,-xop,+crc32,-amx-fp8,+xsaves,-avx512fp16,-usermsr,-sm4,-egpr,+sse4.1,-avx10.1,+avx512ifma,+xsave,+sse4.2,-tsxldtrk,-sm3,-ptwrite,-widekl,-movrs,+invpcid,+64bit,+xsavec,+avx512vpopcntdq,+cmov,+avx512vp2intersect,-avxvnniint8,+avx512cd,+movbe,-ccmp,-amx-int8,-kl,-avx512bmm,-sha512,+avxvnni,-rtm,+adx,+avx2,-hreset,+movdiri,-serialize,+vpclmulqdq,+avx512vl,-uintr,-cf,-jmpabs,+clflushopt,-raoint,-cmpccxadd,+bmi,-amx-tile,+sse,+gfni,-avxvnniint16,-amx-fp16,-zu,-ndd,+xsaveopt,+rdrnd,+avx512f,-amx-bf16,+avx512bf16,+avx512vnni,-push2pop2,+cx8,+avx512bw,+sse3,+pku,-nf,-amx-avx512,+fsgsbase,+clzero,+mwaitx,-lwp,+lzcnt,+sha,+movdir64b,-ppx,+wbnoinvd,-enqcmd,-avxneconvert,-tbm,-pconfig,-amx-complex,+ssse3,+cx16,-avx10.2,+bmi2,+fma,+popcnt,-avxifma,+f16c,+avx512bitalg,+rdpru,+clwb,+mmx,+sse2,+rdseed,+avx512vbmi2,+prefetchi,+rdpid,-amx-movrs,-fma4,+avx512vbmi,+shstk,+vaes,-waitpkg,-sgx,+fxsr,+avx512dq,+sse4a" }
attributes #3 = { nonlazybind uwtable "probe-stack"="inline-asm" "target-cpu"="znver5" "target-features"="+prfchw,-cldemote,+avx,+aes,+sahf,+pclmul,-xop,+crc32,-amx-fp8,+xsaves,-avx512fp16,-usermsr,-sm4,-egpr,+sse4.1,-avx10.1,+avx512ifma,+xsave,+sse4.2,-tsxldtrk,-sm3,-ptwrite,-widekl,-movrs,+invpcid,+64bit,+xsavec,+avx512vpopcntdq,+cmov,+avx512vp2intersect,-avxvnniint8,+avx512cd,+movbe,-ccmp,-amx-int8,-kl,-avx512bmm,-sha512,+avxvnni,-rtm,+adx,+avx2,-hreset,+movdiri,-serialize,+vpclmulqdq,+avx512vl,-uintr,-cf,-jmpabs,+clflushopt,-raoint,-cmpccxadd,+bmi,-amx-tile,+sse,+gfni,-avxvnniint16,-amx-fp16,-zu,-ndd,+xsaveopt,+rdrnd,+avx512f,-amx-bf16,+avx512bf16,+avx512vnni,-push2pop2,+cx8,+avx512bw,+sse3,+pku,-nf,-amx-avx512,+fsgsbase,+clzero,+mwaitx,-lwp,+lzcnt,+sha,+movdir64b,-ppx,+wbnoinvd,-enqcmd,-avxneconvert,-tbm,-pconfig,-amx-complex,+ssse3,+cx16,-avx10.2,+bmi2,+fma,+popcnt,-avxifma,+f16c,+avx512bitalg,+rdpru,+clwb,+mmx,+sse2,+rdseed,+avx512vbmi2,+prefetchi,+rdpid,-amx-movrs,-fma4,+avx512vbmi,+shstk,+vaes,-waitpkg,-sgx,+fxsr,+avx512dq,+sse4a" }
attributes #4 = { inlinehint nonlazybind uwtable "probe-stack"="inline-asm" "target-cpu"="znver5" "target-features"="+prfchw,-cldemote,+avx,+aes,+sahf,+pclmul,-xop,+crc32,-amx-fp8,+xsaves,-avx512fp16,-usermsr,-sm4,-egpr,+sse4.1,-avx10.1,+avx512ifma,+xsave,+sse4.2,-tsxldtrk,-sm3,-ptwrite,-widekl,-movrs,+invpcid,+64bit,+xsavec,+avx512vpopcntdq,+cmov,+avx512vp2intersect,-avxvnniint8,+avx512cd,+movbe,-ccmp,-amx-int8,-kl,-avx512bmm,-sha512,+avxvnni,-rtm,+adx,+avx2,-hreset,+movdiri,-serialize,+vpclmulqdq,+avx512vl,-uintr,-cf,-jmpabs,+clflushopt,-raoint,-cmpccxadd,+bmi,-amx-tile,+sse,+gfni,-avxvnniint16,-amx-fp16,-zu,-ndd,+xsaveopt,+rdrnd,+avx512f,-amx-bf16,+avx512bf16,+avx512vnni,-push2pop2,+cx8,+avx512bw,+sse3,+pku,-nf,-amx-avx512,+fsgsbase,+clzero,+mwaitx,-lwp,+lzcnt,+sha,+movdir64b,-ppx,+wbnoinvd,-enqcmd,-avxneconvert,-tbm,-pconfig,-amx-complex,+ssse3,+cx16,-avx10.2,+bmi2,+fma,+popcnt,-avxifma,+f16c,+avx512bitalg,+rdpru,+clwb,+mmx,+sse2,+rdseed,+avx512vbmi2,+prefetchi,+rdpid,-amx-movrs,-fma4,+avx512vbmi,+shstk,+vaes,-waitpkg,-sgx,+fxsr,+avx512dq,+sse4a" }
attributes #5 = { nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: write) }
attributes #6 = { nocallback nofree nosync nounwind willreturn memory(argmem: readwrite) }
attributes #7 = { nonlazybind "target-cpu"="znver5" }
attributes #8 = { nofree nounwind nonlazybind uwtable "probe-stack"="inline-asm" "target-cpu"="znver5" "target-features"="+prfchw,-cldemote,+avx,+aes,+sahf,+pclmul,-xop,+crc32,-amx-fp8,+xsaves,-avx512fp16,-usermsr,-sm4,-egpr,+sse4.1,-avx10.1,+avx512ifma,+xsave,+sse4.2,-tsxldtrk,-sm3,-ptwrite,-widekl,-movrs,+invpcid,+64bit,+xsavec,+avx512vpopcntdq,+cmov,+avx512vp2intersect,-avxvnniint8,+avx512cd,+movbe,-ccmp,-amx-int8,-kl,-avx512bmm,-sha512,+avxvnni,-rtm,+adx,+avx2,-hreset,+movdiri,-serialize,+vpclmulqdq,+avx512vl,-uintr,-cf,-jmpabs,+clflushopt,-raoint,-cmpccxadd,+bmi,-amx-tile,+sse,+gfni,-avxvnniint16,-amx-fp16,-zu,-ndd,+xsaveopt,+rdrnd,+avx512f,-amx-bf16,+avx512bf16,+avx512vnni,-push2pop2,+cx8,+avx512bw,+sse3,+pku,-nf,-amx-avx512,+fsgsbase,+clzero,+mwaitx,-lwp,+lzcnt,+sha,+movdir64b,-ppx,+wbnoinvd,-enqcmd,-avxneconvert,-tbm,-pconfig,-amx-complex,+ssse3,+cx16,-avx10.2,+bmi2,+fma,+popcnt,-avxifma,+f16c,+avx512bitalg,+rdpru,+clwb,+mmx,+sse2,+rdseed,+avx512vbmi2,+prefetchi,+rdpid,-amx-movrs,-fma4,+avx512vbmi,+shstk,+vaes,-waitpkg,-sgx,+fxsr,+avx512dq,+sse4a" }
attributes #9 = { nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: readwrite) }
attributes #10 = { nonlazybind uwtable "frame-pointer"="non-leaf" "probe-stack"="inline-asm" "target-cpu"="x86-64" }
attributes #11 = { nocallback nofree nosync nounwind willreturn memory(argmem: read) }
attributes #12 = { noinline }
attributes #13 = { nounwind }
attributes #14 = { inlinehint }

!llvm.module.flags = !{!22, !23, !24, !25, !26, !27}
!llvm.ident = !{!28}
!llvm.dbg.cu = !{!29}

!0 = !{i64 6759579385189244834}
!1 = !{i64 -9182080926099000343}
!2 = !{i64 -2650404148832356881}
!3 = !{i64 -8597380891756280088}
!4 = !{i64 -6756264916177263484}
!5 = !{i64 -4090048523166623682}
!6 = !{i64 -1201435909640870887}
!7 = !{i64 6762107205269800777}
!8 = !{i64 -8520554173024134025}
!9 = !{i64 4826402721926290532}
!10 = !{i64 614821033928074426}
!11 = !{i64 8669754322761801947}
!12 = !{i64 2415868163376344318}
!13 = !{i64 -5766411506231277720}
!14 = !{i64 -1686185477648915232}
!15 = !{i64 7183740237764684748}
!16 = !{i64 -9022508866943940068}
!17 = !{i64 -4798871649414273494}
!18 = !{i64 5443013378088383623}
!19 = !{i64 5190441010787001809}
!20 = !{i64 1761330546986324449}
!21 = !{i64 -3557405042054802291}
!22 = !{i32 8, !"PIC Level", i32 2}
!23 = !{i32 2, !"RtLibUseGOT", i32 1}
!24 = !{i32 7, !"uwtable", i32 2}
!25 = !{i32 7, !"frame-pointer", i32 1}
!26 = !{i32 7, !"Dwarf Version", i32 4}
!27 = !{i32 2, !"Debug Info Version", i32 3}
!28 = !{!"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"}
!29 = distinct !DICompileUnit(language: DW_LANG_Rust, file: !30, producer: "clang LLVM (rustc version 1.100.0-nightly (4b6d04e70 2026-09-13))", isOptimized: true, runtimeVersion: 0, emissionKind: FullDebug, splitDebugInlining: false, nameTableKind: None)
!30 = !DIFile(filename: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad/library/core/src/lib.rs/@/core.1b0f708be8c133d1-cgu.0", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad")
!31 = !{i64 5016708724740021877}
!32 = !{!33}
!33 = distinct !{!33, !34, !"_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file10fill_inner: argument 0"}
!34 = distinct !{!34, !"_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file10fill_inner"}
!35 = !{!"branch_weights", i32 4001, i32 4000000}
!36 = !{!37, !33}
!37 = distinct !{!37, !38, !"_RINvNtNtNtCs1xZImt19Kx0_9getrandom8backends8use_file9util_libc14sys_fill_exactNCNvB4_10fill_inner0EB8_: argument 0"}
!38 = distinct !{!38, !"_RINvNtNtNtCs1xZImt19Kx0_9getrandom8backends8use_file9util_libc14sys_fill_exactNCNvB4_10fill_inner0EB8_"}
!39 = !{}
!40 = !{!"branch_weights", i32 2000, i32 6004}
!41 = !{i64 -1692717362700821296}
!42 = !{!"branch_weights", i32 2002, i32 2000}
!43 = !{i64 3600121909165076736}
!44 = !{!45}
!45 = distinct !{!45, !46, !"_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file13open_readonly: argument 0"}
!46 = distinct !{!46, !"_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file13open_readonly"}
!47 = !{!48}
!48 = distinct !{!48, !49, !"_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file13open_readonly: argument 0"}
!49 = distinct !{!49, !"_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file13open_readonly"}
!50 = !{i64 8697847299763557962}
!51 = !{i64 6052131635460327948}
!52 = !{i64 -523055114370099933}
!53 = !{!54}
!54 = distinct !{!54, !55, !"_RNvXsv_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_8LowerHex3fmt: argument 0"}
!55 = distinct !{!55, !"_RNvXsv_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_8LowerHex3fmt"}
!56 = !DILocation(line: 57, column: 35, scope: !57)
!57 = distinct !DISubprogram(name: "fmt", linkageName: "_RNvXsv_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_8LowerHex3fmt", scope: !59, file: !58, line: 56, type: !63, scopeLine: 56, flags: DIFlagPrototyped, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !29, templateParams: !39)
!58 = !DIFile(filename: "library/core/src/fmt/num.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "f4173485a6c9246e1475bee25026a645")
!59 = !DINamespace(name: "{impl#33}", scope: !60)
!60 = !DINamespace(name: "num", scope: !61)
!61 = !DINamespace(name: "fmt", scope: !62)
!62 = !DINamespace(name: "core", scope: null)
!63 = !DISubroutineType(types: !39)
!64 = !{!65}
!65 = distinct !{!65, !55, !"_RNvXsv_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_8LowerHex3fmt: argument 1"}
!66 = !DILocation(line: 27, column: 21, scope: !67, inlinedAt: !69)
!67 = distinct !DISubprogram(name: "fmt", linkageName: "_RNvXsu_NtNtCs2k2z8Zem4rB_4core3fmt3nummNtB7_8LowerHex3fmt", scope: !68, file: !58, line: 14, type: !63, scopeLine: 14, flags: DIFlagPrototyped, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !29, templateParams: !39)
!68 = !DINamespace(name: "{impl#32}", scope: !60)
!69 = distinct !DILocation(line: 57, column: 17, scope: !57)
!70 = !{!71, !73, !54, !65}
!71 = distinct !{!71, !72, !"_RNvXsu_NtNtCs2k2z8Zem4rB_4core3fmt3nummNtB7_8LowerHex3fmt: argument 0"}
!72 = distinct !{!72, !"_RNvXsu_NtNtCs2k2z8Zem4rB_4core3fmt3nummNtB7_8LowerHex3fmt"}
!73 = distinct !{!73, !72, !"_RNvXsu_NtNtCs2k2z8Zem4rB_4core3fmt3nummNtB7_8LowerHex3fmt: argument 1"}
!74 = !DILocation(line: 34, column: 17, scope: !75, inlinedAt: !69)
!75 = distinct !DILexicalBlock(scope: !76, file: !58, line: 33, column: 17)
!76 = distinct !DILexicalBlock(scope: !77, file: !58, line: 29, column: 17)
!77 = distinct !DILexicalBlock(scope: !67, file: !58, line: 27, column: 17)
!78 = !DILocation(line: 0, scope: !76, inlinedAt: !69)
!79 = !DILocation(line: 0, scope: !77, inlinedAt: !69)
!80 = !DILocation(line: 36, column: 21, scope: !81, inlinedAt: !69)
!81 = distinct !DILexicalBlock(scope: !75, file: !58, line: 35, column: 21)
!82 = !DILocation(line: 38, column: 21, scope: !81, inlinedAt: !69)
!83 = !DILocation(line: 35, column: 33, scope: !75, inlinedAt: !69)
!84 = !DILocation(line: 41, column: 47, scope: !81, inlinedAt: !69)
!85 = !DILocation(line: 41, column: 21, scope: !81, inlinedAt: !69)
!86 = !DILocation(line: 41, column: 39, scope: !81, inlinedAt: !69)
!87 = !DILocation(line: 575, column: 9, scope: !88, inlinedAt: !93)
!88 = distinct !DISubprogram(name: "write<u8>", linkageName: "_RNvMs1_NtNtCs2k2z8Zem4rB_4core3mem12maybe_uninitINtB5_11MaybeUninithE5writeB9_", scope: !90, file: !89, line: 574, type: !63, scopeLine: 574, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !29, templateParams: !39)
!89 = !DIFile(filename: "library/core/src/mem/maybe_uninit.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "40dbcdd878e6b463fb6e4e245b9345ae")
!90 = !DINamespace(name: "MaybeUninit", scope: !91)
!91 = !DINamespace(name: "maybe_uninit", scope: !92)
!92 = !DINamespace(name: "mem", scope: !62)
!93 = distinct !DILocation(line: 41, column: 33, scope: !81, inlinedAt: !69)
!94 = !DILocation(line: 42, column: 24, scope: !81, inlinedAt: !69)
!95 = !DILocation(line: 380, column: 27, scope: !96, inlinedAt: !101)
!96 = distinct !DISubprogram(name: "get_unchecked<core::mem::maybe_uninit::MaybeUninit<u8>>", linkageName: "_RNvXs2_NtNtCs2k2z8Zem4rB_4core5slice5indexINtNtNtB9_3ops5range5RangejEINtB5_10SliceIndexSINtNtNtB9_3mem12maybe_uninit11MaybeUninithEE13get_uncheckedB9_", scope: !98, file: !97, line: 362, type: !63, scopeLine: 362, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !29, templateParams: !39)
!97 = !DIFile(filename: "library/core/src/slice/index.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "ab05556326093fb751c425f6c6070f9a")
!98 = !DINamespace(name: "{impl#4}", scope: !99)
!99 = !DINamespace(name: "index", scope: !100)
!100 = !DINamespace(name: "slice", scope: !62)
!101 = distinct !DILocation(line: 533, column: 44, scope: !102, inlinedAt: !104)
!102 = distinct !DISubprogram(name: "get_unchecked<core::mem::maybe_uninit::MaybeUninit<u8>>", linkageName: "_RNvXs5_NtNtCs2k2z8Zem4rB_4core5slice5indexINtNtNtB9_3ops5range9RangeFromjEINtB5_10SliceIndexSINtNtNtB9_3mem12maybe_uninit11MaybeUninithEE13get_uncheckedB9_", scope: !103, file: !97, line: 531, type: !63, scopeLine: 531, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !29, templateParams: !39)
!103 = !DINamespace(name: "{impl#7}", scope: !99)
!104 = distinct !DILocation(line: 649, column: 26, scope: !105, inlinedAt: !108)
!105 = distinct !DISubprogram(name: "get_unchecked<core::mem::maybe_uninit::MaybeUninit<u8>, core::ops::range::RangeFrom<usize>>", linkageName: "_RINvMNtCs2k2z8Zem4rB_4core5sliceSINtNtNtB5_3mem12maybe_uninit11MaybeUninithE13get_uncheckedINtNtNtB5_3ops5range9RangeFromjEEB5_", scope: !107, file: !106, line: 642, type: !63, scopeLine: 642, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !29, templateParams: !39)
!106 = !DIFile(filename: "library/core/src/slice/mod.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "65ce4dca37c4df823e7cad3ec4cb5362")
!107 = !DINamespace(name: "{impl#0}", scope: !100)
!108 = distinct !DILocation(line: 115, column: 32, scope: !109, inlinedAt: !110)
!109 = distinct !DISubprogram(name: "slice_buffer_to_str", linkageName: "_RNvNtNtCs2k2z8Zem4rB_4core3fmt3num19slice_buffer_to_str", scope: !60, file: !58, line: 113, type: !63, scopeLine: 113, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !29, templateParams: !39)
!110 = distinct !DILocation(line: 48, column: 39, scope: !75, inlinedAt: !69)
!111 = !DILocation(line: 49, column: 19, scope: !112, inlinedAt: !69)
!112 = distinct !DILexicalBlock(scope: !75, file: !58, line: 48, column: 17)
!113 = !{!71, !54}
!114 = !DILocation(line: 50, column: 13, scope: !67, inlinedAt: !69)
!115 = !{!116}
!116 = distinct !{!116, !117, !"_RNvXsx_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_8UpperHex3fmt: argument 0"}
!117 = distinct !{!117, !"_RNvXsx_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_8UpperHex3fmt"}
!118 = !DILocation(line: 57, column: 35, scope: !119)
!119 = distinct !DISubprogram(name: "fmt", linkageName: "_RNvXsx_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_8UpperHex3fmt", scope: !120, file: !58, line: 56, type: !63, scopeLine: 56, flags: DIFlagPrototyped, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !29, templateParams: !39)
!120 = !DINamespace(name: "{impl#35}", scope: !60)
!121 = !{!122}
!122 = distinct !{!122, !117, !"_RNvXsx_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_8UpperHex3fmt: argument 1"}
!123 = !DILocation(line: 27, column: 21, scope: !124, inlinedAt: !126)
!124 = distinct !DISubprogram(name: "fmt", linkageName: "_RNvXsw_NtNtCs2k2z8Zem4rB_4core3fmt3nummNtB7_8UpperHex3fmt", scope: !125, file: !58, line: 14, type: !63, scopeLine: 14, flags: DIFlagPrototyped, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !29, templateParams: !39)
!125 = !DINamespace(name: "{impl#34}", scope: !60)
!126 = distinct !DILocation(line: 57, column: 17, scope: !119)
!127 = !{!128, !130, !116, !122}
!128 = distinct !{!128, !129, !"_RNvXsw_NtNtCs2k2z8Zem4rB_4core3fmt3nummNtB7_8UpperHex3fmt: argument 0"}
!129 = distinct !{!129, !"_RNvXsw_NtNtCs2k2z8Zem4rB_4core3fmt3nummNtB7_8UpperHex3fmt"}
!130 = distinct !{!130, !129, !"_RNvXsw_NtNtCs2k2z8Zem4rB_4core3fmt3nummNtB7_8UpperHex3fmt: argument 1"}
!131 = !DILocation(line: 34, column: 17, scope: !132, inlinedAt: !126)
!132 = distinct !DILexicalBlock(scope: !133, file: !58, line: 33, column: 17)
!133 = distinct !DILexicalBlock(scope: !134, file: !58, line: 29, column: 17)
!134 = distinct !DILexicalBlock(scope: !124, file: !58, line: 27, column: 17)
!135 = !DILocation(line: 0, scope: !133, inlinedAt: !126)
!136 = !DILocation(line: 0, scope: !134, inlinedAt: !126)
!137 = !DILocation(line: 36, column: 21, scope: !138, inlinedAt: !126)
!138 = distinct !DILexicalBlock(scope: !132, file: !58, line: 35, column: 21)
!139 = !DILocation(line: 38, column: 21, scope: !138, inlinedAt: !126)
!140 = !DILocation(line: 35, column: 33, scope: !132, inlinedAt: !126)
!141 = !DILocation(line: 41, column: 47, scope: !138, inlinedAt: !126)
!142 = !DILocation(line: 41, column: 21, scope: !138, inlinedAt: !126)
!143 = !DILocation(line: 41, column: 39, scope: !138, inlinedAt: !126)
!144 = !DILocation(line: 575, column: 9, scope: !145, inlinedAt: !146)
!145 = distinct !DISubprogram(name: "write<u8>", linkageName: "_RNvMs1_NtNtCs2k2z8Zem4rB_4core3mem12maybe_uninitINtB5_11MaybeUninithE5writeB9_", scope: !90, file: !89, line: 574, type: !63, scopeLine: 574, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !29, templateParams: !39)
!146 = distinct !DILocation(line: 41, column: 33, scope: !138, inlinedAt: !126)
!147 = !DILocation(line: 42, column: 24, scope: !138, inlinedAt: !126)
!148 = !DILocation(line: 380, column: 27, scope: !96, inlinedAt: !149)
!149 = distinct !DILocation(line: 533, column: 44, scope: !102, inlinedAt: !150)
!150 = distinct !DILocation(line: 649, column: 26, scope: !105, inlinedAt: !151)
!151 = distinct !DILocation(line: 115, column: 32, scope: !109, inlinedAt: !152)
!152 = distinct !DILocation(line: 48, column: 39, scope: !132, inlinedAt: !126)
!153 = !DILocation(line: 49, column: 19, scope: !154, inlinedAt: !126)
!154 = distinct !DILexicalBlock(scope: !132, file: !58, line: 48, column: 17)
!155 = !{!128, !116}
!156 = !DILocation(line: 50, column: 13, scope: !124, inlinedAt: !126)
!157 = !{i64 6905812972072055133}
!158 = !{!159}
!159 = distinct !{!159, !160, !"_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter12debug_struct: argument 0"}
!160 = distinct !{!160, !"_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter12debug_struct"}
!161 = !{!162}
!162 = distinct !{!162, !160, !"_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter12debug_struct: argument 1"}
!163 = !{!164}
!164 = distinct !{!164, !165, !"_RNvNtNtCs2k2z8Zem4rB_4core3fmt8builders16debug_struct_new: argument 0"}
!165 = distinct !{!165, !"_RNvNtNtCs2k2z8Zem4rB_4core3fmt8builders16debug_struct_new"}
!166 = !DILocation(line: 2449, column: 9, scope: !167)
!167 = distinct !DISubprogram(name: "debug_struct", linkageName: "_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter12debug_struct", scope: !169, file: !168, line: 2448, type: !63, scopeLine: 2448, flags: DIFlagPrototyped, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !29, templateParams: !39)
!168 = !DIFile(filename: "library/core/src/fmt/mod.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "9ac494f4a38037ac7986832caaa85718")
!169 = !DINamespace(name: "Formatter", scope: !61)
!170 = !{!171}
!171 = distinct !{!171, !165, !"_RNvNtNtCs2k2z8Zem4rB_4core3fmt8builders16debug_struct_new: argument 1"}
!172 = !{!173}
!173 = distinct !{!173, !174, !"_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter9write_str: argument 0"}
!174 = distinct !{!174, !"_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter9write_str"}
!175 = !DILocation(line: 122, column: 22, scope: !176, inlinedAt: !179)
!176 = distinct !DISubprogram(name: "debug_struct_new", linkageName: "_RNvNtNtCs2k2z8Zem4rB_4core3fmt8builders16debug_struct_new", scope: !178, file: !177, line: 118, type: !63, scopeLine: 118, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !29, templateParams: !39)
!177 = !DIFile(filename: "library/core/src/fmt/builders.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "ca80f8fd9e04b7ff3028b0d8bb353fd6")
!178 = !DINamespace(name: "builders", scope: !61)
!179 = distinct !DILocation(line: 2449, column: 9, scope: !167)
!180 = !DILocation(line: 2100, column: 9, scope: !181, inlinedAt: !182)
!181 = distinct !DISubprogram(name: "write_str", linkageName: "_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter9write_str", scope: !169, file: !168, line: 2099, type: !63, scopeLine: 2099, flags: DIFlagPrototyped, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !29, templateParams: !39)
!182 = distinct !DILocation(line: 122, column: 22, scope: !176, inlinedAt: !179)
!183 = !{!173, !171, !162}
!184 = !{!185, !164, !186, !159, !187}
!185 = distinct !{!185, !174, !"_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter9write_str: argument 1"}
!186 = distinct !{!186, !165, !"_RNvNtNtCs2k2z8Zem4rB_4core3fmt8builders16debug_struct_new: argument 2"}
!187 = distinct !{!187, !160, !"_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter12debug_struct: argument 2"}
!188 = !{i64 8}
!189 = !{!173, !185, !164, !171, !186, !159, !162, !187}
!190 = !DILocation(line: 2100, column: 18, scope: !181, inlinedAt: !182)
!191 = !{!173, !164, !171, !159, !162}
!192 = distinct !{null}
!193 = !DILocation(line: 123, column: 5, scope: !194, inlinedAt: !179)
!194 = distinct !DILexicalBlock(scope: !176, file: !177, line: 122, column: 5)
!195 = !{!164, !159}
!196 = !{!171, !186, !162, !187}
!197 = !{i32 1, i32 0}
!198 = !{!199}
!199 = distinct !{!199, !200, !"_RNvMs2_NtNtCs2k2z8Zem4rB_4core3fmt8buildersNtB5_11DebugStruct6finish: argument 0"}
!200 = distinct !{!200, !"_RNvMs2_NtNtCs2k2z8Zem4rB_4core3fmt8buildersNtB5_11DebugStruct6finish"}
!201 = !DILocation(line: 269, column: 12, scope: !202)
!202 = distinct !DISubprogram(name: "finish", linkageName: "_RNvMs2_NtNtCs2k2z8Zem4rB_4core3fmt8buildersNtB5_11DebugStruct6finish", scope: !203, file: !177, line: 268, type: !63, scopeLine: 268, flags: DIFlagPrototyped, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !29, templateParams: !39)
!203 = !DINamespace(name: "DebugStruct", scope: !178)
!204 = !{i8 0, i8 2}
!205 = !DILocation(line: 0, scope: !202)
!206 = !DILocation(line: 278, column: 9, scope: !207, inlinedAt: !208)
!207 = distinct !DISubprogram(name: "is_pretty", linkageName: "_RNvMs2_NtNtCs2k2z8Zem4rB_4core3fmt8buildersNtB5_11DebugStruct9is_pretty", scope: !203, file: !177, line: 277, type: !63, scopeLine: 277, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !29, templateParams: !39)
!208 = !DILocation(line: 271, column: 25, scope: !209, inlinedAt: !212)
!209 = distinct !DISubprogram(name: "{closure#0}", linkageName: "_RNCNvMs2_NtNtCs2k2z8Zem4rB_4core3fmt8buildersNtB7_11DebugStruct6finish0Bb_", scope: !210, file: !177, line: 270, type: !63, scopeLine: 270, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !29, templateParams: !39)
!210 = !DINamespace(name: "finish", scope: !211)
!211 = !DINamespace(name: "{impl#4}", scope: !178)
!212 = !DILocation(line: 1490, column: 22, scope: !213, inlinedAt: !218)
!213 = distinct !DILexicalBlock(scope: !215, file: !214, line: 1490, column: 13)
!214 = !DIFile(filename: "library/core/src/result.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "92ab1ec144e579029b4ec17e105b17eb")
!215 = distinct !DISubprogram(name: "and_then<(), core::fmt::Error, (), core::fmt::builders::{impl#4}::finish::{closure_env#0}>", linkageName: "_RINvMNtCs2k2z8Zem4rB_4core6resultINtB3_6ResultuNtNtB5_3fmt5ErrorE8and_thenuNCNvMs2_NtBL_8buildersNtB1j_11DebugStruct6finish0EB5_", scope: !216, file: !214, line: 1485, type: !63, scopeLine: 1485, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !29, templateParams: !39)
!216 = !DINamespace(name: "Result", scope: !217)
!217 = !DINamespace(name: "result", scope: !62)
!218 = !DILocation(line: 270, column: 39, scope: !202)
!219 = !DILocation(line: 2373, column: 9, scope: !220, inlinedAt: !221)
!220 = distinct !DISubprogram(name: "alternate", linkageName: "_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter9alternate", scope: !169, file: !168, line: 2372, type: !63, scopeLine: 2372, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !29, templateParams: !39)
!221 = !DILocation(line: 278, column: 18, scope: !207, inlinedAt: !208)
!222 = !DILocation(line: 2100, column: 9, scope: !181, inlinedAt: !223)
!223 = !DILocation(line: 271, scope: !209, inlinedAt: !212)
!224 = !DILocation(line: 271, column: 20, scope: !209, inlinedAt: !212)
!225 = !{!226}
!226 = distinct !{!226, !227, !"_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter9write_str: argument 0"}
!227 = distinct !{!227, !"_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter9write_str"}
!228 = !DILocation(line: 271, column: 81, scope: !209, inlinedAt: !212)
!229 = !DILocation(line: 2100, column: 9, scope: !181, inlinedAt: !230)
!230 = distinct !DILocation(line: 271, column: 81, scope: !209, inlinedAt: !212)
!231 = !{!232, !199}
!232 = distinct !{!232, !227, !"_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter9write_str: argument 1"}
!233 = !{!226, !232, !199}
!234 = !DILocation(line: 2100, column: 18, scope: !181, inlinedAt: !230)
!235 = !{!226, !199}
!236 = distinct !{null}
!237 = !{!238}
!238 = distinct !{!238, !239, !"_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter9write_str: argument 0"}
!239 = distinct !{!239, !"_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter9write_str"}
!240 = !DILocation(line: 271, column: 48, scope: !209, inlinedAt: !212)
!241 = !DILocation(line: 2100, column: 9, scope: !181, inlinedAt: !242)
!242 = distinct !DILocation(line: 271, column: 48, scope: !209, inlinedAt: !212)
!243 = !{!244, !199}
!244 = distinct !{!244, !239, !"_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter9write_str: argument 1"}
!245 = !{!238, !244, !199}
!246 = !DILocation(line: 2100, column: 18, scope: !181, inlinedAt: !242)
!247 = !{!238, !199}
!248 = !DILocation(line: 274, column: 9, scope: !202)
!249 = !{!"address", !"read_provenance"}
