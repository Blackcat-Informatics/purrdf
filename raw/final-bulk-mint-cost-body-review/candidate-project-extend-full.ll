define internal fastcc void @<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::adapters::map::Map<core::slice::iter::Iter<core::option::Option<usize>>, purrdf_sparql_eval::modifier::eval_project_sequence<purrdf_core::ir::term::TermId>::{closure#1}>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(24) %1) unnamed_addr #0 personality ptr @rust_eh_personality !guid !64560 {
  %3 = load ptr, ptr %1, align 8, !alias.scope !64561, !nonnull !1740, !noundef !1740
  %4 = getelementptr inbounds nuw i8, ptr %1, i64 8
  %5 = load ptr, ptr %4, align 8, !alias.scope !64561, !nonnull !1740, !noundef !1740
  %6 = getelementptr inbounds nuw i8, ptr %1, i64 16
  %7 = load ptr, ptr %6, align 8, !alias.scope !64561
  %8 = ptrtoint ptr %5 to i64
  %9 = ptrtoint ptr %3 to i64
  %10 = sub nuw i64 %8, %9
  %11 = lshr exact i64 %10, 4
  %12 = load i64, ptr %0, align 8, !range !1778, !alias.scope !64565, !noundef !1740
  %13 = add i64 %12, -1
  %14 = icmp ugt i64 %13, 4
  %15 = getelementptr inbounds nuw i8, ptr %0, i64 16
  %16 = load i64, ptr %15, align 8, !alias.scope !64565
  %17 = add i64 %16, -1
  %18 = select i1 %14, i64 %17, i64 %13
  %19 = tail call i64 @llvm.umax.i64(i64 %13, i64 4)
  %20 = sub i64 %19, %18
  %21 = icmp ult i64 %20, %11
  br i1 %21, label %22, label %26, !prof !1742

22:                                               ; preds = %2
; call <purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow
  tail call void @<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %0, i64 noundef %18, i64 noundef %11, i1 noundef zeroext true) #92
  %23 = load i64, ptr %0, align 8, !range !1778
  %24 = add i64 %23, -1
  %25 = tail call i64 @llvm.umax.i64(i64 %24, i64 4)
  br label %26

26:                                               ; preds = %22, %2
  %27 = phi i64 [ %19, %2 ], [ %25, %22 ]
  %28 = phi i64 [ %13, %2 ], [ %24, %22 ]
  %29 = icmp ugt i64 %28, 4
  %30 = getelementptr inbounds nuw i8, ptr %0, i64 8
  %31 = load ptr, ptr %30, align 8, !nonnull !1740
  %32 = select i1 %29, ptr %31, ptr %30
  %33 = select i1 %29, ptr %15, ptr %0
  %34 = load i64, ptr %33, align 8, !noundef !1740
  %35 = add i64 %34, -1
  %36 = icmp ult i64 %35, %27
  br i1 %36, label %37, label %42

37:                                               ; preds = %26
  %38 = getelementptr inbounds nuw i8, ptr %7, i64 8
  %39 = getelementptr inbounds nuw i8, ptr %7, i64 16
  br label %49

40:                                               ; preds = %123
  %41 = add nuw i64 %27, 1
  br label %42

42:                                               ; preds = %40, %26
  %43 = phi ptr [ %3, %26 ], [ %54, %40 ]
  %44 = phi i64 [ %34, %26 ], [ %41, %40 ]
  store i64 %44, ptr %33, align 8
  %45 = icmp eq ptr %43, %5
  br i1 %45, label %.loopexit, label %46

46:                                               ; preds = %42
  %47 = getelementptr inbounds nuw i8, ptr %7, i64 8
  %48 = getelementptr inbounds nuw i8, ptr %7, i64 16
  br label %76

49:                                               ; preds = %123, %37
  %50 = phi i64 [ %35, %37 ], [ %126, %123 ]
  %51 = phi ptr [ %3, %37 ], [ %54, %123 ]
  %52 = icmp eq ptr %51, %5
  br i1 %52, label %128, label %53

53:                                               ; preds = %49
  %54 = getelementptr inbounds nuw i8, ptr %51, i64 16
  %55 = load i64, ptr %51, align 8, !range !1739, !noalias !64568, !noundef !1740
  %56 = getelementptr i8, ptr %51, i64 8
  %57 = load i64, ptr %56, align 8, !noalias !64568
  %58 = trunc nuw i64 %55 to i1
  br i1 %58, label %59, label %123

59:                                               ; preds = %53
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %7) ]
  %60 = load i64, ptr %7, align 8, !range !1778, !noalias !64568, !noundef !1740
  %61 = add i64 %60, -1
  %62 = icmp ugt i64 %61, 4
  br i1 %62, label %63, label %67

63:                                               ; preds = %59
  %64 = load ptr, ptr %38, align 8, !noalias !64568, !nonnull !1740, !noundef !1740
  %65 = load i64, ptr %39, align 8, !noalias !64568, !noundef !1740
  %66 = add i64 %65, -1
  br label %67

67:                                               ; preds = %63, %59
  %68 = phi i64 [ %66, %63 ], [ %61, %59 ]
  %69 = phi ptr [ %64, %63 ], [ %38, %59 ]
  %70 = icmp ult i64 %57, %68
  br i1 %70, label %73, label %71

71:                                               ; preds = %67
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %57, i64 noundef %68, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.791) #89
          to label %72 unwind label %130

72:                                               ; preds = %71
  unreachable

73:                                               ; preds = %67
  %74 = getelementptr inbounds nuw [8 x i8], ptr %69, i64 %57
  %75 = load <2 x i32>, ptr %74, align 4, !noalias !64568
  br label %123

76:                                               ; preds = %117, %46
  %77 = phi ptr [ %43, %46 ], [ %78, %117 ]
  %78 = getelementptr inbounds nuw i8, ptr %77, i64 16
  %79 = load i64, ptr %77, align 8, !range !1739, !noalias !64571, !noundef !1740
  %80 = getelementptr i8, ptr %77, i64 8
  %81 = load i64, ptr %80, align 8, !noalias !64571
  %82 = trunc nuw i64 %79 to i1
  br i1 %82, label %83, label %99

83:                                               ; preds = %76
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %7) ]
  %84 = load i64, ptr %7, align 8, !range !1778, !noalias !64571, !noundef !1740
  %85 = add i64 %84, -1
  %86 = icmp ugt i64 %85, 4
  br i1 %86, label %87, label %91

87:                                               ; preds = %83
  %88 = load ptr, ptr %47, align 8, !noalias !64571, !nonnull !1740, !noundef !1740
  %89 = load i64, ptr %48, align 8, !noalias !64571, !noundef !1740
  %90 = add i64 %89, -1
  br label %91

91:                                               ; preds = %87, %83
  %92 = phi i64 [ %90, %87 ], [ %85, %83 ]
  %93 = phi ptr [ %88, %87 ], [ %47, %83 ]
  %94 = icmp ult i64 %81, %92
  br i1 %94, label %96, label %95

95:                                               ; preds = %91
; call core::panicking::panic_bounds_check
  tail call void @core::panicking::panic_bounds_check(i64 noundef %81, i64 noundef %92, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.791) #89, !noalias !64571
  unreachable

96:                                               ; preds = %91
  %97 = getelementptr inbounds nuw [8 x i8], ptr %93, i64 %81
  %98 = load <2 x i32>, ptr %97, align 4, !noalias !64571
  br label %99

99:                                               ; preds = %96, %76
  %100 = phi <2 x i32> [ <i32 2, i32 undef>, %76 ], [ %98, %96 ]
  %101 = load i64, ptr %0, align 8, !range !1778, !alias.scope !64574, !noundef !1740
  %102 = add i64 %101, -1
  %103 = icmp ugt i64 %102, 4
  %104 = load ptr, ptr %30, align 8, !alias.scope !64574, !nonnull !1740
  %105 = select i1 %103, ptr %104, ptr %30
  %106 = select i1 %103, ptr %15, ptr %0
  %107 = tail call i64 @llvm.umax.i64(i64 %102, i64 4)
  %108 = load i64, ptr %106, align 8, !alias.scope !64574, !noundef !1740
  %109 = add i64 %108, -1
  %110 = icmp eq i64 %109, %107
  br i1 %110, label %111, label %117, !prof !1742

111:                                              ; preds = %99
; call <purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow
  tail call void @<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %0, i64 noundef %107, i64 noundef 1, i1 noundef zeroext true) #92
  %112 = load i64, ptr %0, align 8, !range !1778, !alias.scope !64574, !noundef !1740
  %113 = icmp ugt i64 %112, 5
  %114 = load ptr, ptr %30, align 8, !alias.scope !64574, !nonnull !1740
  %115 = select i1 %113, ptr %114, ptr %30
  %116 = select i1 %113, ptr %15, ptr %0
  br label %117

117:                                              ; preds = %111, %99
  %118 = phi ptr [ %115, %111 ], [ %105, %99 ]
  %119 = phi ptr [ %116, %111 ], [ %106, %99 ]
  %120 = getelementptr inbounds nuw [8 x i8], ptr %118, i64 %109
  store <2 x i32> %100, ptr %120, align 4
  %121 = add i64 %108, 1
  store i64 %121, ptr %119, align 8, !alias.scope !64574
  %122 = icmp eq ptr %78, %5
  br i1 %122, label %.loopexit, label %76

.loopexit:                                        ; preds = %117, %128, %42
  ret void

123:                                              ; preds = %73, %53
  %124 = phi <2 x i32> [ <i32 2, i32 undef>, %53 ], [ %75, %73 ]
  %125 = getelementptr inbounds nuw [8 x i8], ptr %32, i64 %50
  store <2 x i32> %124, ptr %125, align 4
  %126 = add i64 %50, 1
  %127 = icmp eq i64 %126, %27
  br i1 %127, label %40, label %49

128:                                              ; preds = %49
  %129 = add nuw i64 %50, 1
  store i64 %129, ptr %33, align 8
  br label %.loopexit

130:                                              ; preds = %71
  %131 = landingpad { ptr, i32 }
          cleanup
  %132 = add nuw i64 %50, 1
  store i64 %132, ptr %33, align 8
  resume { ptr, i32 } %131
}
