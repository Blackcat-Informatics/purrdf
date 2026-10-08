define internal fastcc void @<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::FromIterator<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::from_iter::<core::iter::adapters::map::Map<core::slice::iter::Iter<core::option::Option<usize>>, purrdf_sparql_eval::modifier::eval_project_sequence<purrdf_core::ir::term::TermId>::{closure#1}>>(ptr dead_on_unwind noalias nofree noundef nonnull writable writeonly align 8 captures(none) dereferenceable(40) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(24) %1) unnamed_addr #2 personality ptr @rust_eh_personality !guid !64766 {
  %3 = alloca [40 x i8], align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %3)
  store i64 1, ptr %3, align 8
  tail call void @llvm.experimental.noalias.scope.decl(metadata !64767)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !64770)
  %4 = load ptr, ptr %1, align 8, !alias.scope !64772, !noalias !64767, !nonnull !1740, !noundef !1740
  %5 = getelementptr inbounds nuw i8, ptr %1, i64 8
  %6 = load ptr, ptr %5, align 8, !alias.scope !64772, !noalias !64767, !nonnull !1740, !noundef !1740
  %7 = getelementptr inbounds nuw i8, ptr %1, i64 16
  %8 = load ptr, ptr %7, align 8, !alias.scope !64772, !noalias !64767
  %9 = ptrtoint ptr %6 to i64
  %10 = ptrtoint ptr %4 to i64
  %11 = sub nuw i64 %9, %10
  %12 = getelementptr inbounds nuw i8, ptr %3, i64 16
  %13 = icmp ugt i64 %11, 64
  br i1 %13, label %16, label %14, !prof !1742

14:                                               ; preds = %2
  %15 = getelementptr inbounds nuw i8, ptr %3, i64 8
  br label %26

16:                                               ; preds = %2
  %17 = lshr exact i64 %11, 4
; invoke <purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow
  invoke void @<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %3, i64 noundef 0, i64 noundef %17, i1 noundef zeroext true) #91
          to label %18 unwind label %134

18:                                               ; preds = %16
  %19 = load i64, ptr %3, align 8, !range !1778, !alias.scope !64767, !noalias !64770
  %20 = freeze i64 %19
  %21 = add i64 %20, -1
  %22 = call i64 @llvm.umax.i64(i64 %21, i64 4)
  %23 = icmp ugt i64 %21, 4
  %24 = getelementptr inbounds nuw i8, ptr %3, i64 8
  %25 = load ptr, ptr %24, align 8, !alias.scope !64767, !noalias !64770
  %spec.select = select i1 %23, ptr %25, ptr %24
  %spec.select17 = select i1 %23, ptr %12, ptr %3
  %.pre = load i64, ptr %spec.select17, align 8, !alias.scope !64767, !noalias !64770
  br label %26

26:                                               ; preds = %18, %14
  %27 = phi i64 [ 1, %14 ], [ %.pre, %18 ]
  %28 = phi ptr [ %15, %14 ], [ %spec.select, %18 ]
  %29 = phi i64 [ 4, %14 ], [ %22, %18 ]
  %30 = phi ptr [ %15, %14 ], [ %24, %18 ]
  %31 = phi ptr [ %3, %14 ], [ %spec.select17, %18 ]
  %32 = add i64 %27, -1
  %33 = icmp ult i64 %32, %29
  br i1 %33, label %34, label %39

34:                                               ; preds = %26
  %35 = getelementptr inbounds nuw i8, ptr %8, i64 8
  %36 = getelementptr inbounds nuw i8, ptr %8, i64 16
  br label %46

37:                                               ; preds = %122
  %38 = add nuw i64 %29, 1
  br label %39

39:                                               ; preds = %37, %26
  %40 = phi ptr [ %4, %26 ], [ %51, %37 ]
  %41 = phi i64 [ %27, %26 ], [ %38, %37 ]
  store i64 %41, ptr %31, align 8, !alias.scope !64767, !noalias !64770
  %42 = icmp eq ptr %40, %6
  br i1 %42, label %.loopexit, label %43

43:                                               ; preds = %39
  %44 = getelementptr inbounds nuw i8, ptr %8, i64 8
  %45 = getelementptr inbounds nuw i8, ptr %8, i64 16
  br label %73

46:                                               ; preds = %122, %34
  %47 = phi i64 [ %32, %34 ], [ %125, %122 ]
  %48 = phi ptr [ %4, %34 ], [ %51, %122 ]
  %49 = icmp eq ptr %48, %6
  br i1 %49, label %127, label %50

50:                                               ; preds = %46
  %51 = getelementptr inbounds nuw i8, ptr %48, i64 16
  %52 = load i64, ptr %48, align 8, !range !1739, !noalias !64776, !noundef !1740
  %53 = getelementptr i8, ptr %48, i64 8
  %54 = load i64, ptr %53, align 8, !noalias !64776
  %55 = trunc nuw i64 %52 to i1
  br i1 %55, label %56, label %122

56:                                               ; preds = %50
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %8) ]
  %57 = load i64, ptr %8, align 8, !range !1778, !noalias !64776, !noundef !1740
  %58 = add i64 %57, -1
  %59 = icmp ugt i64 %58, 4
  br i1 %59, label %60, label %64

60:                                               ; preds = %56
  %61 = load ptr, ptr %35, align 8, !noalias !64776, !nonnull !1740, !noundef !1740
  %62 = load i64, ptr %36, align 8, !noalias !64776, !noundef !1740
  %63 = add i64 %62, -1
  br label %64

64:                                               ; preds = %60, %56
  %65 = phi i64 [ %63, %60 ], [ %58, %56 ]
  %66 = phi ptr [ %61, %60 ], [ %35, %56 ]
  %67 = icmp ult i64 %54, %65
  br i1 %67, label %70, label %68

68:                                               ; preds = %64
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %54, i64 noundef %65, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.791) #88
          to label %69 unwind label %129, !noalias !64770

69:                                               ; preds = %68
  unreachable

70:                                               ; preds = %64
  %71 = getelementptr inbounds nuw [8 x i8], ptr %66, i64 %54
  %72 = load <2 x i32>, ptr %71, align 4, !noalias !64776
  br label %122

73:                                               ; preds = %116, %43
  %74 = phi ptr [ %40, %43 ], [ %75, %116 ]
  %75 = getelementptr inbounds nuw i8, ptr %74, i64 16
  %76 = load i64, ptr %74, align 8, !range !1739, !noalias !64779, !noundef !1740
  %77 = getelementptr i8, ptr %74, i64 8
  %78 = load i64, ptr %77, align 8, !noalias !64779
  %79 = trunc nuw i64 %76 to i1
  br i1 %79, label %80, label %97

80:                                               ; preds = %73
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %8) ]
  %81 = load i64, ptr %8, align 8, !range !1778, !noalias !64779, !noundef !1740
  %82 = add i64 %81, -1
  %83 = icmp ugt i64 %82, 4
  br i1 %83, label %84, label %88

84:                                               ; preds = %80
  %85 = load ptr, ptr %44, align 8, !noalias !64779, !nonnull !1740, !noundef !1740
  %86 = load i64, ptr %45, align 8, !noalias !64779, !noundef !1740
  %87 = add i64 %86, -1
  br label %88

88:                                               ; preds = %84, %80
  %89 = phi i64 [ %87, %84 ], [ %82, %80 ]
  %90 = phi ptr [ %85, %84 ], [ %44, %80 ]
  %91 = icmp ult i64 %78, %89
  br i1 %91, label %94, label %92

92:                                               ; preds = %88
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %78, i64 noundef %89, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.791) #88
          to label %93 unwind label %134

93:                                               ; preds = %92
  unreachable

94:                                               ; preds = %88
  %95 = getelementptr inbounds nuw [8 x i8], ptr %90, i64 %78
  %96 = load <2 x i32>, ptr %95, align 4, !noalias !64779
  br label %97

97:                                               ; preds = %94, %73
  %98 = phi <2 x i32> [ <i32 2, i32 undef>, %73 ], [ %96, %94 ]
  %99 = load i64, ptr %3, align 8, !range !1778, !alias.scope !64782, !noalias !64770, !noundef !1740
  %100 = add i64 %99, -1
  %101 = icmp ugt i64 %100, 4
  %102 = load ptr, ptr %30, align 8, !alias.scope !64782, !noalias !64770, !nonnull !1740
  %103 = select i1 %101, ptr %102, ptr %30
  %104 = select i1 %101, ptr %12, ptr %3
  %105 = call i64 @llvm.umax.i64(i64 %100, i64 4)
  %106 = load i64, ptr %104, align 8, !alias.scope !64782, !noalias !64770, !noundef !1740
  %107 = add i64 %106, -1
  %108 = icmp eq i64 %107, %105
  br i1 %108, label %109, label %116, !prof !1742

109:                                              ; preds = %97
; invoke <purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow
  invoke void @<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %3, i64 noundef %105, i64 noundef 1, i1 noundef zeroext true) #91
          to label %110 unwind label %132

110:                                              ; preds = %109
  %111 = load i64, ptr %3, align 8, !range !1778, !alias.scope !64782, !noalias !64770, !noundef !1740
  %112 = icmp ugt i64 %111, 5
  %113 = load ptr, ptr %30, align 8, !alias.scope !64782, !noalias !64770, !nonnull !1740
  %114 = select i1 %112, ptr %113, ptr %30
  %115 = select i1 %112, ptr %12, ptr %3
  br label %116

116:                                              ; preds = %110, %97
  %117 = phi ptr [ %114, %110 ], [ %103, %97 ]
  %118 = phi ptr [ %115, %110 ], [ %104, %97 ]
  %119 = getelementptr inbounds nuw [8 x i8], ptr %117, i64 %107
  store <2 x i32> %98, ptr %119, align 4, !noalias !64770
  %120 = add i64 %106, 1
  store i64 %120, ptr %118, align 8, !alias.scope !64782, !noalias !64770
  %121 = icmp eq ptr %75, %6
  br i1 %121, label %.loopexit, label %73

122:                                              ; preds = %70, %50
  %123 = phi <2 x i32> [ <i32 2, i32 undef>, %50 ], [ %72, %70 ]
  %124 = getelementptr inbounds nuw [8 x i8], ptr %28, i64 %47
  store <2 x i32> %123, ptr %124, align 4, !noalias !64770
  %125 = add i64 %47, 1
  %126 = icmp eq i64 %125, %29
  br i1 %126, label %37, label %46

127:                                              ; preds = %46
  %128 = add nuw i64 %47, 1
  store i64 %128, ptr %31, align 8, !alias.scope !64767, !noalias !64770
  br label %.loopexit

129:                                              ; preds = %68
  %130 = landingpad { ptr, i32 }
          cleanup
  %131 = add nuw i64 %47, 1
  store i64 %131, ptr %31, align 8, !alias.scope !64767, !noalias !64770
  br label %136

132:                                              ; preds = %109
  %133 = landingpad { ptr, i32 }
          cleanup
  br label %136

134:                                              ; preds = %92, %16
  %135 = landingpad { ptr, i32 }
          cleanup
  br label %136

136:                                              ; preds = %134, %132, %129
  %137 = phi { ptr, i32 } [ %130, %129 ], [ %133, %132 ], [ %135, %134 ]
  %138 = load i64, ptr %3, align 8, !range !1778, !alias.scope !19667, !noundef !1740
  %139 = icmp ugt i64 %138, 5
  br i1 %139, label %140, label %145

140:                                              ; preds = %136
  %141 = getelementptr inbounds nuw i8, ptr %3, i64 8
  %142 = load ptr, ptr %141, align 8, !nonnull !1740, !noundef !1740
  %143 = shl i64 %138, 3
  %144 = add i64 %143, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %142, i64 noundef %144, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !64785
  br label %145

.loopexit:                                        ; preds = %116, %127, %39
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %0, ptr noundef nonnull align 8 dereferenceable(40) %3, i64 40, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %3)
  ret void

145:                                              ; preds = %140, %136
  resume { ptr, i32 } %137
}
