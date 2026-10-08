; ModuleID = 'addr2line-a74baf2303dab72e.addr2line.77efcc4c7d7093d8-cgu.0.rcgu.o'
source_filename = "addr2line.77efcc4c7d7093d8-cgu.0"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

@anon.9ab48ae7fdf266c6dfa6017d5efa634b.0 = private unnamed_addr constant [62 x i8] c"/cargo/registry/25cdd57fae9f0462/addr2line-0.27.1/src/line.rs\00", align 1, !guid !0
@anon.9ab48ae7fdf266c6dfa6017d5efa634b.1 = private unnamed_addr constant <{ ptr, [16 x i8] }> <{ ptr @anon.9ab48ae7fdf266c6dfa6017d5efa634b.0, [16 x i8] c"=\00\00\00\00\00\00\00\9E\00\00\00\19\00\00\00" }>, align 8, !guid !1
@anon.9ab48ae7fdf266c6dfa6017d5efa634b.2 = private unnamed_addr constant <{ ptr, [16 x i8] }> <{ ptr @anon.9ab48ae7fdf266c6dfa6017d5efa634b.0, [16 x i8] c"=\00\00\00\00\00\00\00\A8\00\00\00$\00\00\00" }>, align 8, !guid !2

; <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
; Function Attrs: cold nonlazybind optsize uwtable
define void @_RINvNvMs2_NtCsc70TAahYccp_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECsaiq3CZtZqOa_9addr2line(ptr noalias nofree noundef align 8 captures(none) dereferenceable(16) %0, i64 noundef %1, i64 noundef %2, i64 noundef range(i64 1, -9223372036854775807) %3, i64 noundef %4) unnamed_addr #0 !guid !15 {
; call <alloc::raw_vec::RawVecInner>::grow_amortized
  %6 = tail call { i64, i64 } @_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner14grow_amortizedCs3wRyrdzSNKt_5gimli(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %0, i64 noundef %1, i64 noundef %2, i64 noundef %3, i64 noundef %4)
  %7 = extractvalue { i64, i64 } %6, 0
  %8 = icmp eq i64 %7, -1
  br i1 %8, label %11, label %9, !prof !16

9:                                                ; preds = %5
  %10 = extractvalue { i64, i64 } %6, 1
; call alloc::raw_vec::handle_error
  tail call void @_RNvNtCsc70TAahYccp_5alloc7raw_vec12handle_error(i64 noundef %7, i64 %10) #10
  unreachable

11:                                               ; preds = %5
  ret void
}

; <addr2line::line::Lines>::find_location
; Function Attrs: nonlazybind optsize uwtable
define void @_RNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB4_5Lines13find_location(ptr dead_on_unwind noalias nofree noundef writable writeonly sret([32 x i8]) align 8 captures(none) dereferenceable(32) %0, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(32) %1, i64 noundef %2) unnamed_addr #1 personality ptr @rust_eh_personality !guid !17 {
  %4 = getelementptr inbounds nuw i8, ptr %1, i64 16
  %5 = load ptr, ptr %4, align 8, !nonnull !18, !noundef !18
  %6 = getelementptr inbounds nuw i8, ptr %1, i64 24
  %7 = load i64, ptr %6, align 8, !noundef !18
  switch i64 %7, label %.preheader8 [
    i64 0, label %68
    i64 1, label %.loopexit9
  ]

.loopexit9:                                       ; preds = %.preheader8, %3
  %8 = phi i64 [ 0, %3 ], [ %27, %.preheader8 ]
  %9 = getelementptr inbounds nuw [32 x i8], ptr %5, i64 %8
  %10 = getelementptr inbounds nuw i8, ptr %9, i64 16
  %11 = load i64, ptr %10, align 8, !alias.scope !19, !noalias !24, !noundef !18
  %12 = icmp uge i64 %2, %11
  %13 = getelementptr inbounds nuw i8, ptr %9, i64 24
  %14 = load i64, ptr %13, align 8, !alias.scope !19, !noalias !24
  %15 = icmp uge i64 %2, %14
  %16 = xor i1 %12, true
  %17 = select i1 %16, i1 true, i1 %15
  br i1 %17, label %30, label %35

.preheader8:                                      ; preds = %3, %.preheader8
  %18 = phi i64 [ %28, %.preheader8 ], [ %7, %3 ]
  %19 = phi i64 [ %27, %.preheader8 ], [ 0, %3 ]
  %20 = lshr i64 %18, 1
  %21 = add nuw i64 %20, %19
  %22 = icmp ult i64 %21, %7
  tail call void @llvm.assume(i1 %22)
  %23 = getelementptr inbounds nuw [32 x i8], ptr %5, i64 %21
  %24 = getelementptr inbounds nuw i8, ptr %23, i64 16
  %25 = load i64, ptr %24, align 8, !alias.scope !26, !noalias !29, !noundef !18
  %26 = icmp ult i64 %2, %25
  %27 = select i1 %26, i64 %19, i64 %21, !unpredictable !18
  %28 = sub i64 %18, %20
  %29 = icmp ugt i64 %28, 1
  br i1 %29, label %.preheader8, label %.loopexit9

30:                                               ; preds = %.loopexit9
  %31 = select i1 %12, i1 %15, i1 false
  %32 = zext i1 %31 to i64
  %33 = add nuw nsw i64 %8, %32
  %34 = icmp ule i64 %33, %7
  tail call void @llvm.assume(i1 %34)
  br label %68

35:                                               ; preds = %.loopexit9
  %36 = icmp ult i64 %8, %7
  br i1 %36, label %37, label %56

37:                                               ; preds = %35
  %38 = load ptr, ptr %9, align 8, !nonnull !18, !noundef !18
  %39 = getelementptr inbounds nuw i8, ptr %9, i64 8
  %40 = load i64, ptr %39, align 8, !noundef !18
  switch i64 %40, label %.preheader [
    i64 0, label %68
    i64 1, label %.loopexit
  ]

.loopexit:                                        ; preds = %.preheader, %37
  %41 = phi i64 [ 0, %37 ], [ %53, %.preheader ]
  %42 = getelementptr inbounds nuw [24 x i8], ptr %38, i64 %41
  %43 = load i64, ptr %42, align 8, !alias.scope !31, !noalias !36, !noundef !18
  %44 = icmp eq i64 %43, %2
  br i1 %44, label %63, label %57

.preheader:                                       ; preds = %37, %.preheader
  %45 = phi i64 [ %54, %.preheader ], [ %40, %37 ]
  %46 = phi i64 [ %53, %.preheader ], [ 0, %37 ]
  %47 = lshr i64 %45, 1
  %48 = add nuw i64 %47, %46
  %49 = icmp ult i64 %48, %40
  tail call void @llvm.assume(i1 %49)
  %50 = getelementptr inbounds nuw [24 x i8], ptr %38, i64 %48
  %51 = load i64, ptr %50, align 8, !alias.scope !38, !noalias !41, !noundef !18
  %52 = icmp ugt i64 %51, %2
  %53 = select i1 %52, i64 %46, i64 %48, !unpredictable !18
  %54 = sub i64 %45, %47
  %55 = icmp ugt i64 %54, 1
  br i1 %55, label %.preheader, label %.loopexit

56:                                               ; preds = %35
; call core::panicking::panic_bounds_check
  tail call void @_RNvNtCs2k2z8Zem4rB_4core9panicking18panic_bounds_check(i64 noundef %8, i64 noundef %7, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.9ab48ae7fdf266c6dfa6017d5efa634b.1) #11
  unreachable

57:                                               ; preds = %.loopexit
  %58 = icmp ult i64 %43, %2
  %59 = zext i1 %58 to i64
  %60 = add nuw nsw i64 %41, %59
  %61 = icmp ule i64 %60, %40
  tail call void @llvm.assume(i1 %61)
  %62 = icmp eq i64 %60, 0
  br i1 %62, label %68, label %66

63:                                               ; preds = %66, %.loopexit
  %64 = phi i64 [ %67, %66 ], [ %41, %.loopexit ]
  %65 = icmp ult i64 %64, %40
  br i1 %65, label %69, label %98

66:                                               ; preds = %57
  %67 = add nsw i64 %60, -1
  br label %63

68:                                               ; preds = %57, %37, %30, %3
  store i32 2, ptr %0, align 8
  br label %99

69:                                               ; preds = %63
  %70 = getelementptr inbounds nuw [24 x i8], ptr %38, i64 %64
  tail call void @llvm.experimental.noalias.scope.decl(metadata !43)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !46)
  %71 = getelementptr inbounds nuw i8, ptr %1, i64 8
  %72 = load i64, ptr %71, align 8, !alias.scope !43, !noalias !48, !noundef !18
  %73 = getelementptr inbounds nuw i8, ptr %70, i64 8
  %74 = load i64, ptr %73, align 8, !alias.scope !46, !noalias !50, !noundef !18
  %75 = icmp ult i64 %74, %72
  br i1 %75, label %76, label %83

76:                                               ; preds = %69
  %77 = load ptr, ptr %1, align 8, !alias.scope !43, !noalias !48, !nonnull !18, !noundef !18
  %78 = getelementptr inbounds nuw [24 x i8], ptr %77, i64 %74
  %79 = getelementptr inbounds nuw i8, ptr %78, i64 8
  %80 = load ptr, ptr %79, align 8, !noalias !51, !nonnull !18, !noundef !18
  %81 = getelementptr inbounds nuw i8, ptr %78, i64 16
  %82 = load i64, ptr %81, align 8, !noalias !51, !noundef !18
  br label %83

83:                                               ; preds = %76, %69
  %84 = phi i64 [ %82, %76 ], [ undef, %69 ]
  %85 = phi ptr [ %80, %76 ], [ null, %69 ]
  %86 = getelementptr inbounds nuw i8, ptr %70, i64 16
  %87 = load i32, ptr %86, align 8, !alias.scope !46, !noalias !50, !noundef !18
  %88 = icmp ne i32 %87, 0
  %89 = getelementptr inbounds nuw i8, ptr %70, i64 20
  %90 = load i32, ptr %89, align 4, !alias.scope !46, !noalias !50
  %91 = select i1 %88, i32 %90, i32 undef
  %92 = zext i1 %88 to i32
  store i32 %92, ptr %0, align 8
  %93 = getelementptr inbounds nuw i8, ptr %0, i64 4
  store i32 %87, ptr %93, align 4
  %94 = getelementptr inbounds nuw i8, ptr %0, i64 8
  store i32 %92, ptr %94, align 8
  %95 = getelementptr inbounds nuw i8, ptr %0, i64 12
  store i32 %91, ptr %95, align 4
  %96 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store ptr %85, ptr %96, align 8
  %97 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store i64 %84, ptr %97, align 8
  br label %99

98:                                               ; preds = %63
; call core::panicking::panic_bounds_check
  tail call void @_RNvNtCs2k2z8Zem4rB_4core9panicking18panic_bounds_check(i64 noundef %64, i64 noundef %40, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.9ab48ae7fdf266c6dfa6017d5efa634b.2) #11
  unreachable

99:                                               ; preds = %83, %68
  ret void
}

; addr2line::line::has_backward_slash_root
; Function Attrs: mustprogress nofree norecurse nosync nounwind nonlazybind optsize willreturn memory(argmem: read) uwtable
define internal fastcc noundef zeroext i1 @_RNvNtCsaiq3CZtZqOa_9addr2line4line23has_backward_slash_root(ptr noalias nofree noundef nonnull readonly captures(none) %0, i64 noundef %1) unnamed_addr #2 !guid !52 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %24, label %4

4:                                                ; preds = %2
  %5 = load i8, ptr %0, align 1
  %6 = icmp eq i8 %5, 92
  br i1 %6, label %24, label %7

7:                                                ; preds = %4
  %8 = icmp ult i64 %1, 3
  br i1 %8, label %24, label %11

9:                                                ; preds = %11
  %10 = icmp eq i64 %1, 3
  br i1 %10, label %19, label %15

11:                                               ; preds = %7
  %12 = getelementptr inbounds nuw i8, ptr %0, i64 1
  %13 = load i8, ptr %12, align 1, !alias.scope !53, !noundef !18
  %14 = icmp sgt i8 %13, -65
  br i1 %14, label %9, label %24

15:                                               ; preds = %9
  %16 = getelementptr inbounds nuw i8, ptr %0, i64 3
  %17 = load i8, ptr %16, align 1, !alias.scope !53, !noundef !18
  %18 = icmp sgt i8 %17, -65
  br i1 %18, label %19, label %24

19:                                               ; preds = %15, %9
  %20 = load i16, ptr %12, align 1
  %21 = icmp ne i16 %20, 23610
  %22 = zext i1 %21 to i32
  %23 = icmp eq i32 %22, 0
  br label %24

24:                                               ; preds = %19, %15, %11, %7, %4, %2
  %25 = phi i1 [ true, %4 ], [ %23, %19 ], [ false, %11 ], [ false, %15 ], [ false, %7 ], [ false, %2 ]
  ret i1 %25
}

; addr2line::line::path_push
; Function Attrs: nonlazybind optsize uwtable
define void @_RNvNtCsaiq3CZtZqOa_9addr2line4line9path_push(ptr noalias nofree noundef align 8 captures(none) dereferenceable(24) %0, ptr noalias nofree noundef nonnull readonly captures(none) %1, i64 noundef %2) unnamed_addr #1 personality ptr @rust_eh_personality !guid !58 {
  %4 = alloca [24 x i8], align 8
  %5 = icmp eq i64 %2, 0
  br i1 %5, label %26, label %6

6:                                                ; preds = %3
  %7 = load i8, ptr %1, align 1, !alias.scope !59
  %8 = icmp eq i8 %7, 47
  br i1 %8, label %28, label %9

9:                                                ; preds = %6
  %10 = icmp ult i64 %2, 3
  br i1 %10, label %26, label %13

11:                                               ; preds = %13
  %12 = icmp eq i64 %2, 3
  br i1 %12, label %21, label %17

13:                                               ; preds = %9
  %14 = getelementptr inbounds nuw i8, ptr %1, i64 1
  %15 = load i8, ptr %14, align 1, !alias.scope !62, !noundef !18
  %16 = icmp sgt i8 %15, -65
  br i1 %16, label %11, label %26

17:                                               ; preds = %11
  %18 = getelementptr inbounds nuw i8, ptr %1, i64 3
  %19 = load i8, ptr %18, align 1, !alias.scope !62, !noundef !18
  %20 = icmp sgt i8 %19, -65
  br i1 %20, label %21, label %26

21:                                               ; preds = %17, %11
  %22 = load i16, ptr %14, align 1
  %23 = icmp ne i16 %22, 12090
  %24 = zext i1 %23 to i32
  %25 = icmp eq i32 %24, 0
  br i1 %25, label %28, label %26

26:                                               ; preds = %21, %17, %13, %9, %3
; call addr2line::line::has_backward_slash_root
  %27 = tail call fastcc noundef zeroext i1 @_RNvNtCsaiq3CZtZqOa_9addr2line4line23has_backward_slash_root(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %1, i64 noundef %2)
  br i1 %27, label %28, label %40

28:                                               ; preds = %26, %21, %6
  call void @llvm.lifetime.start.p0(ptr nonnull %4), !noalias !67
; call <alloc::raw_vec::RawVecInner>::try_allocate_in
  call void @_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner15try_allocate_inCs3wRyrdzSNKt_5gimli(ptr noalias nofree noundef nonnull sret([24 x i8]) align 8 captures(none) dereferenceable(24) %4, i64 noundef range(i64 0, -9223372036854775808) %2, i1 noundef zeroext false, i64 noundef 1, i64 noundef 1), !noalias !67
  %29 = load i64, ptr %4, align 8, !range !71, !noalias !67, !noundef !18
  %30 = trunc nuw i64 %29 to i1
  %31 = getelementptr inbounds nuw i8, ptr %4, i64 8
  %32 = load i64, ptr %31, align 8, !range !72, !noalias !67, !noundef !18
  %33 = getelementptr inbounds nuw i8, ptr %4, i64 16
  br i1 %30, label %34, label %36, !prof !73

34:                                               ; preds = %28
  %35 = load i64, ptr %33, align 8, !noalias !67
; call alloc::raw_vec::handle_error
  tail call void @_RNvNtCsc70TAahYccp_5alloc7raw_vec12handle_error(i64 noundef %32, i64 %35) #10, !noalias !67
  unreachable

36:                                               ; preds = %28
  %37 = load ptr, ptr %33, align 8, !noalias !67, !nonnull !18, !noundef !18
  %38 = icmp samesign ule i64 %2, %32
  tail call void @llvm.assume(i1 %38)
  call void @llvm.lifetime.end.p0(ptr nonnull %4), !noalias !67
  br i1 %5, label %81, label %39

39:                                               ; preds = %36
  tail call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 1 %37, ptr nonnull readonly align 1 %1, i64 range(i64 0, -9223372036854775808) %2, i1 false), !noalias !74
  br label %81

40:                                               ; preds = %26
  %41 = getelementptr inbounds nuw i8, ptr %0, i64 8
  %42 = getelementptr inbounds nuw i8, ptr %0, i64 16
  %43 = load i64, ptr %42, align 8, !noundef !18
  %44 = icmp sgt i64 %43, -1
  tail call void @llvm.assume(i1 %44)
  %45 = icmp eq i64 %43, 0
  br i1 %45, label %54, label %46

46:                                               ; preds = %40
  %47 = load ptr, ptr %41, align 8, !nonnull !18, !noundef !18
; call addr2line::line::has_backward_slash_root
  %48 = tail call fastcc noundef zeroext i1 @_RNvNtCsaiq3CZtZqOa_9addr2line4line23has_backward_slash_root(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %47, i64 noundef %43)
  %49 = select i1 %48, i8 92, i8 47
  %50 = getelementptr i8, ptr %47, i64 %43
  %51 = getelementptr i8, ptr %50, i64 -1
  %52 = load i8, ptr %51, align 1
  %53 = icmp eq i8 %49, %52
  br i1 %53, label %54, label %71

54:                                               ; preds = %76, %46, %40
  %55 = phi i64 [ %43, %46 ], [ %79, %76 ], [ 0, %40 ]
  %56 = load i64, ptr %0, align 8, !range !75, !alias.scope !76, !noundef !18
  %57 = sub i64 %56, %55
  %58 = icmp ugt i64 %2, %57
  br i1 %58, label %59, label %62, !prof !73

59:                                               ; preds = %54
; call <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  tail call void @_RINvNvMs2_NtCsc70TAahYccp_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECsaiq3CZtZqOa_9addr2line(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %0, i64 noundef %55, i64 noundef %2, i64 noundef 1, i64 noundef 1)
  %60 = load i64, ptr %42, align 8, !alias.scope !81, !noundef !18
  %61 = icmp sgt i64 %60, -1
  tail call void @llvm.assume(i1 %61)
  br label %64

62:                                               ; preds = %54
  %63 = icmp sgt i64 %55, -1
  tail call void @llvm.assume(i1 %63)
  br i1 %5, label %68, label %64

64:                                               ; preds = %62, %59
  %65 = phi i64 [ %60, %59 ], [ %55, %62 ]
  %66 = load ptr, ptr %41, align 8, !alias.scope !81, !nonnull !18, !noundef !18
  %67 = getelementptr inbounds nuw i8, ptr %66, i64 %65
  tail call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 1 %67, ptr nonnull readonly align 1 %1, i64 %2, i1 false)
  br label %68

68:                                               ; preds = %64, %62
  %69 = phi i64 [ %65, %64 ], [ %55, %62 ]
  %70 = add i64 %69, %2
  store i64 %70, ptr %42, align 8, !alias.scope !81
  br label %80

71:                                               ; preds = %46
  %72 = load i64, ptr %0, align 8, !range !75, !alias.scope !82, !noundef !18
  %73 = icmp eq i64 %72, %43
  br i1 %73, label %74, label %76, !prof !73

74:                                               ; preds = %71
; call <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  tail call void @_RINvNvMs2_NtCsc70TAahYccp_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECsaiq3CZtZqOa_9addr2line(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %0, i64 noundef %43, i64 noundef 1, i64 noundef 1, i64 noundef 1)
  %75 = load ptr, ptr %41, align 8, !alias.scope !87
  br label %76

76:                                               ; preds = %74, %71
  %77 = phi ptr [ %47, %71 ], [ %75, %74 ]
  %78 = getelementptr inbounds nuw i8, ptr %77, i64 %43
  store i8 %49, ptr %78, align 1
  %79 = add nuw i64 %43, 1
  store i64 %79, ptr %42, align 8, !alias.scope !87
  br label %54

80:                                               ; preds = %81, %68
  ret void

81:                                               ; preds = %36, %39
; call <alloc::raw_vec::RawVecInner>::deallocate
  tail call void @_RNvMs3_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner10deallocateCs3wRyrdzSNKt_5gimli(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %0, i64 noundef 1, i64 noundef 1)
  store i64 %32, ptr %0, align 8
  %82 = getelementptr inbounds nuw i8, ptr %0, i64 8
  store ptr %37, ptr %82, align 8
  %83 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %2, ptr %83, align 8
  br label %80
}

; Function Attrs: nonlazybind
declare i32 @rust_eh_personality(...) unnamed_addr #3

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.start.p0(ptr captures(none)) #4

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.end.p0(ptr captures(none)) #4

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: write)
declare void @llvm.assume(i1 noundef) #5

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i64(ptr noalias writeonly captures(none), ptr noalias readonly captures(none), i64, i1 immarg) #4

; core::panicking::panic_bounds_check
; Function Attrs: cold minsize noinline noreturn nonlazybind optsize uwtable
declare void @_RNvNtCs2k2z8Zem4rB_4core9panicking18panic_bounds_check(i64 noundef, i64 noundef, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24)) unnamed_addr #6

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: readwrite)
declare void @llvm.experimental.noalias.scope.decl(metadata) #7

; alloc::raw_vec::handle_error
; Function Attrs: cold minsize noreturn nonlazybind optsize uwtable
declare void @_RNvNtCsc70TAahYccp_5alloc7raw_vec12handle_error(i64 noundef range(i64 0, -9223372036854775807), i64) unnamed_addr #8

; <alloc::raw_vec::RawVecInner>::deallocate
; Function Attrs: nounwind nonlazybind optsize uwtable
declare void @_RNvMs3_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner10deallocateCs3wRyrdzSNKt_5gimli(ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(16), i64 noundef range(i64 1, -9223372036854775807), i64 noundef) unnamed_addr #9

; <alloc::raw_vec::RawVecInner>::grow_amortized
; Function Attrs: nounwind nonlazybind optsize uwtable
declare { i64, i64 } @_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner14grow_amortizedCs3wRyrdzSNKt_5gimli(ptr noalias nofree noundef align 8 captures(none) dereferenceable(16), i64 noundef, i64 noundef, i64 noundef range(i64 1, -9223372036854775807), i64 noundef) unnamed_addr #9

; <alloc::raw_vec::RawVecInner>::try_allocate_in
; Function Attrs: nounwind nonlazybind optsize uwtable
declare void @_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner15try_allocate_inCs3wRyrdzSNKt_5gimli(ptr dead_on_unwind noalias nofree noundef writable writeonly sret([24 x i8]) align 8 captures(none) dereferenceable(24) initializes((0, 16)), i64 noundef, i1 noundef zeroext, i64 noundef range(i64 1, -9223372036854775807), i64 noundef) unnamed_addr #9

attributes #0 = { cold nonlazybind optsize uwtable "frame-pointer"="non-leaf" "probe-stack"="inline-asm" "target-cpu"="x86-64" }
attributes #1 = { nonlazybind optsize uwtable "frame-pointer"="non-leaf" "probe-stack"="inline-asm" "target-cpu"="x86-64" }
attributes #2 = { mustprogress nofree norecurse nosync nounwind nonlazybind optsize willreturn memory(argmem: read) uwtable "frame-pointer"="non-leaf" "probe-stack"="inline-asm" "target-cpu"="x86-64" }
attributes #3 = { nonlazybind "target-cpu"="x86-64" }
attributes #4 = { nocallback nofree nosync nounwind willreturn memory(argmem: readwrite) }
attributes #5 = { nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: write) }
attributes #6 = { cold minsize noinline noreturn nonlazybind optsize uwtable "frame-pointer"="non-leaf" "probe-stack"="inline-asm" "target-cpu"="x86-64" }
attributes #7 = { nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: readwrite) }
attributes #8 = { cold minsize noreturn nonlazybind optsize uwtable "frame-pointer"="non-leaf" "probe-stack"="inline-asm" "target-cpu"="x86-64" }
attributes #9 = { nounwind nonlazybind optsize uwtable "frame-pointer"="non-leaf" "probe-stack"="inline-asm" "target-cpu"="x86-64" }
attributes #10 = { noreturn }
attributes #11 = { noinline noreturn }

!llvm.module.flags = !{!3, !4, !5, !6, !7, !8, !9}
!llvm.ident = !{!10}
!llvm.dbg.cu = !{!11, !13}

!0 = !{i64 8270812689482800106}
!1 = !{i64 862950153275297797}
!2 = !{i64 -4669174285332871887}
!3 = !{i32 8, !"PIC Level", i32 2}
!4 = !{i32 2, !"RtLibUseGOT", i32 1}
!5 = !{i32 7, !"uwtable", i32 2}
!6 = !{i32 7, !"frame-pointer", i32 1}
!7 = !{i32 7, !"PIE Level", i32 2}
!8 = !{i32 7, !"Dwarf Version", i32 4}
!9 = !{i32 2, !"Debug Info Version", i32 3}
!10 = !{!"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"}
!11 = distinct !DICompileUnit(language: DW_LANG_Rust, file: !12, producer: "clang LLVM (rustc version 1.100.0-nightly (4b6d04e70 2026-09-13))", isOptimized: true, runtimeVersion: 0, emissionKind: FullDebug, splitDebugInlining: false, nameTableKind: None)
!12 = !DIFile(filename: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad/library/alloc/src/lib.rs/@/alloc.8d16d54ddffdc8a3-cgu.0", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad")
!13 = distinct !DICompileUnit(language: DW_LANG_Rust, file: !14, producer: "clang LLVM (rustc version 1.100.0-nightly (4b6d04e70 2026-09-13))", isOptimized: true, runtimeVersion: 0, emissionKind: FullDebug, splitDebugInlining: false, nameTableKind: None)
!14 = !DIFile(filename: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad/library/std/src/lib.rs/@/std.552422bcccb5c833-cgu.0", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad")
!15 = !{i64 -8110596204451610801}
!16 = !{!"branch_weights", !"expected", i32 2000, i32 1}
!17 = !{i64 3556600013026998855}
!18 = !{}
!19 = !{!20, !22}
!20 = distinct !{!20, !21, !"_RNCNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB6_5Lines13find_location0B8_: argument 1"}
!21 = distinct !{!21, !"_RNCNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB6_5Lines13find_location0B8_"}
!22 = distinct !{!22, !23, !"_RINvMNtCs2k2z8Zem4rB_4core5sliceSNtNtCsaiq3CZtZqOa_9addr2line4line12LineSequence16binary_search_byNCNvMs_Bx_NtBx_5Lines13find_location0EBz_: argument 0"}
!23 = distinct !{!23, !"_RINvMNtCs2k2z8Zem4rB_4core5sliceSNtNtCsaiq3CZtZqOa_9addr2line4line12LineSequence16binary_search_byNCNvMs_Bx_NtBx_5Lines13find_location0EBz_"}
!24 = !{!25}
!25 = distinct !{!25, !21, !"_RNCNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB6_5Lines13find_location0B8_: argument 0"}
!26 = !{!27, !22}
!27 = distinct !{!27, !28, !"_RNCNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB6_5Lines13find_location0B8_: argument 1"}
!28 = distinct !{!28, !"_RNCNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB6_5Lines13find_location0B8_"}
!29 = !{!30}
!30 = distinct !{!30, !28, !"_RNCNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB6_5Lines13find_location0B8_: argument 0"}
!31 = !{!32, !34}
!32 = distinct !{!32, !33, !"_RNCNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB6_5Lines13find_locations_0B8_: argument 1"}
!33 = distinct !{!33, !"_RNCNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB6_5Lines13find_locations_0B8_"}
!34 = distinct !{!34, !35, !"_RINvMNtCs2k2z8Zem4rB_4core5sliceSNtNtCsaiq3CZtZqOa_9addr2line4line7LineRow16binary_search_byNCNvMs_Bx_NtBx_5Lines13find_locations_0EBz_: argument 0"}
!35 = distinct !{!35, !"_RINvMNtCs2k2z8Zem4rB_4core5sliceSNtNtCsaiq3CZtZqOa_9addr2line4line7LineRow16binary_search_byNCNvMs_Bx_NtBx_5Lines13find_locations_0EBz_"}
!36 = !{!37}
!37 = distinct !{!37, !33, !"_RNCNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB6_5Lines13find_locations_0B8_: argument 0"}
!38 = !{!39, !34}
!39 = distinct !{!39, !40, !"_RNCNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB6_5Lines13find_locations_0B8_: argument 1"}
!40 = distinct !{!40, !"_RNCNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB6_5Lines13find_locations_0B8_"}
!41 = !{!42}
!42 = distinct !{!42, !40, !"_RNCNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB6_5Lines13find_locations_0B8_: argument 0"}
!43 = !{!44}
!44 = distinct !{!44, !45, !"_RNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB4_5Lines12row_location: argument 1"}
!45 = distinct !{!45, !"_RNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB4_5Lines12row_location"}
!46 = !{!47}
!47 = distinct !{!47, !45, !"_RNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB4_5Lines12row_location: argument 2"}
!48 = !{!49, !47}
!49 = distinct !{!49, !45, !"_RNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB4_5Lines12row_location: argument 0"}
!50 = !{!49, !44}
!51 = !{!49, !44, !47}
!52 = !{i64 -5146587992388764467}
!53 = !{!54, !56}
!54 = distinct !{!54, !55, !"_RNvNtNtCs2k2z8Zem4rB_4core3str6traits11check_range: argument 0"}
!55 = distinct !{!55, !"_RNvNtNtCs2k2z8Zem4rB_4core3str6traits11check_range"}
!56 = distinct !{!56, !57, !"_RNvXs6_NtNtCs2k2z8Zem4rB_4core3str6traitsINtNtB9_5range5RangejEINtNtNtB9_5slice5index10SliceIndexeE3get: argument 0"}
!57 = distinct !{!57, !"_RNvXs6_NtNtCs2k2z8Zem4rB_4core3str6traitsINtNtB9_5range5RangejEINtNtNtB9_5slice5index10SliceIndexeE3get"}
!58 = !{i64 7818363993949599616}
!59 = !{!60}
!60 = distinct !{!60, !61, !"_RNvNtCsaiq3CZtZqOa_9addr2line4line22has_forward_slash_root: argument 0"}
!61 = distinct !{!61, !"_RNvNtCsaiq3CZtZqOa_9addr2line4line22has_forward_slash_root"}
!62 = !{!63, !65, !60}
!63 = distinct !{!63, !64, !"_RNvNtNtCs2k2z8Zem4rB_4core3str6traits11check_range: argument 0"}
!64 = distinct !{!64, !"_RNvNtNtCs2k2z8Zem4rB_4core3str6traits11check_range"}
!65 = distinct !{!65, !66, !"_RNvXs6_NtNtCs2k2z8Zem4rB_4core3str6traitsINtNtB9_5range5RangejEINtNtNtB9_5slice5index10SliceIndexeE3get: argument 0"}
!66 = distinct !{!66, !"_RNvXs6_NtNtCs2k2z8Zem4rB_4core3str6traitsINtNtB9_5range5RangejEINtNtNtB9_5slice5index10SliceIndexeE3get"}
!67 = !{!68, !70}
!68 = distinct !{!68, !69, !"_RINvXs_NvMNtCsc70TAahYccp_5alloc5sliceSp9to_vec_inhNtB5_10ConvertVec6to_vecNtNtBa_5alloc6GlobalECsaiq3CZtZqOa_9addr2line: argument 0"}
!69 = distinct !{!69, !"_RINvXs_NvMNtCsc70TAahYccp_5alloc5sliceSp9to_vec_inhNtB5_10ConvertVec6to_vecNtNtBa_5alloc6GlobalECsaiq3CZtZqOa_9addr2line"}
!70 = distinct !{!70, !69, !"_RINvXs_NvMNtCsc70TAahYccp_5alloc5sliceSp9to_vec_inhNtB5_10ConvertVec6to_vecNtNtBa_5alloc6GlobalECsaiq3CZtZqOa_9addr2line: argument 1"}
!71 = !{i64 0, i64 2}
!72 = !{i64 0, i64 -9223372036854775807}
!73 = !{!"branch_weights", !"expected", i32 1, i32 2000}
!74 = !{!68}
!75 = !{i64 0, i64 -9223372036854775808}
!76 = !{!77, !79}
!77 = distinct !{!77, !78, !"_RNvMs_NtCsc70TAahYccp_5alloc3vecINtB4_3VechE7reserveCsaiq3CZtZqOa_9addr2line: argument 0"}
!78 = distinct !{!78, !"_RNvMs_NtCsc70TAahYccp_5alloc3vecINtB4_3VechE7reserveCsaiq3CZtZqOa_9addr2line"}
!79 = distinct !{!79, !80, !"_RNvMs_NtCsc70TAahYccp_5alloc3vecINtB4_3VechE15append_elementsCsaiq3CZtZqOa_9addr2line: argument 0"}
!80 = distinct !{!80, !"_RNvMs_NtCsc70TAahYccp_5alloc3vecINtB4_3VechE15append_elementsCsaiq3CZtZqOa_9addr2line"}
!81 = !{!79}
!82 = !{!83, !85}
!83 = distinct !{!83, !84, !"_RNvMs_NtCsc70TAahYccp_5alloc3vecINtB4_3VechE7reserveCsaiq3CZtZqOa_9addr2line: argument 0"}
!84 = distinct !{!84, !"_RNvMs_NtCsc70TAahYccp_5alloc3vecINtB4_3VechE7reserveCsaiq3CZtZqOa_9addr2line"}
!85 = distinct !{!85, !86, !"_RNvMNtCsc70TAahYccp_5alloc6stringNtB2_6String4push: argument 0"}
!86 = distinct !{!86, !"_RNvMNtCsc70TAahYccp_5alloc6stringNtB2_6String4push"}
!87 = !{!85}
