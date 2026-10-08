; ModuleID = 'object-55e2aec033d22019.object.172cdd72c3c713b2-cgu.0.rcgu.o'
source_filename = "object.172cdd72c3c713b2-cgu.0"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

@_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw2FN = external local_unnamed_addr global { { { ptr } } }

; <&[u8] as object::read::read_ref::ReadRef>::read_bytes_at_until
; Function Attrs: nonlazybind uwtable
define { ptr, i64 } @_RNvXNtNtCs1ZmoXRRPDtS_6object4read8read_refRShNtB2_7ReadRef19read_bytes_at_until(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %0, i64 noundef range(i64 0, -9223372036854775808) %1, i64 noundef %2, i64 noundef %3, i8 noundef %4) unnamed_addr #0 personality ptr @rust_eh_personality !guid !5 {
  %6 = icmp ult i64 %3, %2
  br i1 %6, label %24, label %7

7:                                                ; preds = %5
  %8 = sub nuw i64 %3, %2
  %9 = icmp ugt i64 %3, %1
  br i1 %9, label %24, label %10

10:                                               ; preds = %7
  %11 = getelementptr inbounds nuw i8, ptr %0, i64 %2
  %12 = getelementptr inbounds nuw i8, ptr %0, i64 %3
  %13 = load atomic ptr, ptr @_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw2FN monotonic, align 8, !noalias !6, !nonnull !9, !noundef !9
  %14 = tail call { i64, ptr } %13(i8 noundef %4, ptr noundef nonnull readonly %11, ptr noundef nonnull readonly %12), !noalias !6, !inline_history !10
  %15 = extractvalue { i64, ptr } %14, 0
  %16 = trunc nuw i64 %15 to i1
  br i1 %16, label %17, label %24

17:                                               ; preds = %10
  %18 = extractvalue { i64, ptr } %14, 1
  %19 = ptrtoint ptr %18 to i64
  %20 = ptrtoint ptr %11 to i64
  %21 = sub i64 %19, %20
  %22 = icmp sgt i64 %21, -1
  tail call void @llvm.assume(i1 %22)
  %23 = icmp ult i64 %21, %8
  tail call void @llvm.assume(i1 %23)
  br label %24

24:                                               ; preds = %17, %10, %7, %5
  %25 = phi i64 [ undef, %5 ], [ undef, %7 ], [ %21, %17 ], [ undef, %10 ]
  %26 = phi ptr [ null, %5 ], [ null, %7 ], [ %11, %17 ], [ null, %10 ]
  %27 = insertvalue { ptr, i64 } poison, ptr %26, 0
  %28 = insertvalue { ptr, i64 } %27, i64 %25, 1
  ret { ptr, i64 } %28
}

; Function Attrs: nonlazybind
declare i32 @rust_eh_personality(...) unnamed_addr #1

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: write)
declare void @llvm.assume(i1 noundef) #2

attributes #0 = { nonlazybind uwtable "frame-pointer"="non-leaf" "probe-stack"="inline-asm" "target-cpu"="x86-64" }
attributes #1 = { nonlazybind "target-cpu"="x86-64" }
attributes #2 = { nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: write) }

!llvm.module.flags = !{!0, !1, !2, !3}
!llvm.ident = !{!4}

!0 = !{i32 8, !"PIC Level", i32 2}
!1 = !{i32 2, !"RtLibUseGOT", i32 1}
!2 = !{i32 7, !"uwtable", i32 2}
!3 = !{i32 7, !"frame-pointer", i32 1}
!4 = !{!"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"}
!5 = !{i64 7779624040379499461}
!6 = !{!7}
!7 = distinct !{!7, !8, !"_RNCNvNtCskkIW8vVChzC_6memchr6memchr6memchr0Cs1ZmoXRRPDtS_6object: argument 0"}
!8 = distinct !{!8, !"_RNCNvNtCskkIW8vVChzC_6memchr6memchr6memchr0Cs1ZmoXRRPDtS_6object"}
!9 = !{}
!10 = distinct !{null}
