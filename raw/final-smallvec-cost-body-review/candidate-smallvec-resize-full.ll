define internal fastcc void @<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::resize(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %0, i64 noundef %1) unnamed_addr #0 personality ptr @rust_eh_personality !guid !84817 {
  %3 = load i64, ptr %0, align 8, !range !1778, !noundef !1740
  %4 = add i64 %3, -1
  %5 = icmp ugt i64 %4, 4
  %6 = getelementptr inbounds nuw i8, ptr %0, i64 16
  %7 = load i64, ptr %6, align 8
  %8 = add i64 %7, -1
  %9 = select i1 %5, i64 %8, i64 %4
  %10 = icmp ugt i64 %1, %9
  br i1 %10, label %20, label %11

11:                                               ; preds = %2
  %12 = icmp ugt i64 %3, 5
  %13 = select i1 %12, i64 16, i64 0
  %14 = getelementptr inbounds nuw i8, ptr %0, i64 %13
  %15 = load i64, ptr %14, align 8, !alias.scope !84818, !noundef !1740
  %16 = add i64 %15, -1
  %17 = icmp ult i64 %1, %16
  br i1 %17, label %18, label %.loopexit

18:                                               ; preds = %11
  %19 = add nuw i64 %1, 1
  store i64 %19, ptr %14, align 8, !alias.scope !84818
  br label %.loopexit

20:                                               ; preds = %2
  %21 = sub nuw i64 %1, %9
  %22 = tail call i64 @llvm.umax.i64(i64 %4, i64 4)
  %23 = sub i64 %22, %9
  %24 = icmp ult i64 %23, %21
  br i1 %24, label %25, label %29, !prof !3851

25:                                               ; preds = %20
; call <purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow
  tail call void @<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %0, i64 noundef %9, i64 noundef %21, i1 noundef zeroext true) #92, !noalias !84821
  %26 = load i64, ptr %0, align 8, !range !1778, !alias.scope !84824, !noalias !84821
  %27 = add i64 %26, -1
  %28 = tail call i64 @llvm.umax.i64(i64 %27, i64 4)
  br label %29

29:                                               ; preds = %25, %20
  %30 = phi i64 [ %22, %20 ], [ %28, %25 ]
  %31 = phi i64 [ %4, %20 ], [ %27, %25 ]
  %32 = icmp ugt i64 %31, 4
  %33 = getelementptr inbounds nuw i8, ptr %0, i64 8
  %34 = load ptr, ptr %33, align 8, !alias.scope !84824, !noalias !84821, !nonnull !1740
  %35 = select i1 %32, ptr %34, ptr %33
  %36 = select i1 %32, ptr %6, ptr %0
  %37 = load i64, ptr %36, align 8, !alias.scope !84824, !noalias !84821, !noundef !1740
  %38 = add i64 %37, -1
  %39 = icmp ult i64 %38, %30
  br i1 %39, label %.preheader, label %42

40:                                               ; preds = %49
  %41 = add nuw i64 %30, 1
  store i64 %41, ptr %36, align 8
  br i1 %51, label %.loopexit, label %42

42:                                               ; preds = %40, %29
  %43 = phi i64 [ %53, %40 ], [ %21, %29 ]
  %44 = phi i32 [ %47, %40 ], [ 2, %29 ]
  br label %57

.preheader:                                       ; preds = %29, %49
  %45 = phi i64 [ %55, %49 ], [ %38, %29 ]
  %46 = phi i64 [ %53, %49 ], [ %21, %29 ]
  %47 = phi i32 [ %52, %49 ], [ 2, %29 ]
  %48 = icmp eq i32 %47, -1
  br i1 %48, label %82, label %49

49:                                               ; preds = %.preheader
  %50 = add i64 %46, -1
  %51 = icmp eq i64 %50, 0
  %52 = select i1 %51, i32 -1, i32 %47
  %53 = tail call i64 @llvm.umax.i64(i64 %50, i64 1)
  %54 = getelementptr inbounds nuw [8 x i8], ptr %35, i64 %45
  store i32 %47, ptr %54, align 4, !noalias !84821
  %55 = add i64 %45, 1
  %56 = icmp eq i64 %55, %30
  br i1 %56, label %40, label %.preheader

57:                                               ; preds = %77, %42
  %58 = phi i64 [ %59, %77 ], [ %43, %42 ]
  %59 = add i64 %58, -1
  %60 = icmp eq i64 %59, 0
  %61 = load i64, ptr %0, align 8, !range !1778, !alias.scope !84826, !noalias !84821, !noundef !1740
  %62 = add i64 %61, -1
  %63 = icmp ugt i64 %62, 4
  %64 = load ptr, ptr %33, align 8, !alias.scope !84826, !noalias !84821, !nonnull !1740
  %65 = select i1 %63, ptr %64, ptr %33
  %66 = select i1 %63, ptr %6, ptr %0
  %67 = tail call i64 @llvm.umax.i64(i64 %62, i64 4)
  %68 = load i64, ptr %66, align 8, !alias.scope !84826, !noalias !84821, !noundef !1740
  %69 = add i64 %68, -1
  %70 = icmp eq i64 %69, %67
  br i1 %70, label %71, label %77, !prof !1742

71:                                               ; preds = %57
; call <purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow
  tail call void @<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %0, i64 noundef %67, i64 noundef 1, i1 noundef zeroext true) #92, !noalias !84821
  %72 = load i64, ptr %0, align 8, !range !1778, !alias.scope !84826, !noalias !84821, !noundef !1740
  %73 = icmp ugt i64 %72, 5
  %74 = load ptr, ptr %33, align 8, !alias.scope !84826, !noalias !84821, !nonnull !1740
  %75 = select i1 %73, ptr %74, ptr %33
  %76 = select i1 %73, ptr %6, ptr %0
  br label %77

77:                                               ; preds = %71, %57
  %78 = phi ptr [ %75, %71 ], [ %65, %57 ]
  %79 = phi ptr [ %76, %71 ], [ %66, %57 ]
  %80 = getelementptr inbounds nuw [8 x i8], ptr %78, i64 %69
  store i32 %44, ptr %80, align 4, !noalias !84821
  %81 = add i64 %68, 1
  store i64 %81, ptr %79, align 8, !alias.scope !84826, !noalias !84821
  br i1 %60, label %.loopexit, label %57

82:                                               ; preds = %.preheader
  %83 = add nuw i64 %45, 1
  store i64 %83, ptr %36, align 8
  br label %.loopexit

.loopexit:                                        ; preds = %77, %82, %40, %18, %11
  ret void
}
