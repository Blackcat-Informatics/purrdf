define internal fastcc void @<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>> as alloc::vec::spec_extend::SpecExtend<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, core::iter::adapters::map::Map<core::slice::iter::Iter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, purrdf_sparql_eval::modifier::eval_project_sequence<purrdf_core::ir::term::TermId>::{closure#1}>>>::spec_extend(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(24) %1) unnamed_addr #0 personality ptr @rust_eh_personality !guid !126172 {
  %3 = alloca [40 x i8], align 8
  %4 = alloca [40 x i8], align 8
  tail call void @llvm.experimental.noalias.scope.decl(metadata !126173)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !126176)
  %5 = load ptr, ptr %1, align 8, !alias.scope !126176, !noalias !126173, !nonnull !1740, !noundef !1740
  %6 = getelementptr inbounds nuw i8, ptr %1, i64 8
  %7 = load ptr, ptr %6, align 8, !alias.scope !126176, !noalias !126173, !nonnull !1740, !noundef !1740
  %8 = ptrtoint ptr %7 to i64
  %9 = ptrtoint ptr %5 to i64
  %10 = sub nuw i64 %8, %9
  %11 = udiv exact i64 %10, 40
  %12 = getelementptr inbounds nuw i8, ptr %0, i64 16
  %13 = load i64, ptr %12, align 8, !alias.scope !126178, !noalias !126176, !noundef !1740
  %14 = load i64, ptr %0, align 8, !range !1835, !alias.scope !126178, !noalias !126176, !noundef !1740
  %15 = sub i64 %14, %13
  %16 = icmp ugt i64 %11, %15
  br i1 %16, label %17, label %19, !prof !1742

17:                                               ; preds = %2
; call <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  tail call fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.8174518965507137190)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %0, i64 noundef %13, i64 noundef %11, i64 noundef 8, i64 noundef 40), !noalias !126176
  %18 = load i64, ptr %12, align 8, !alias.scope !126173, !noalias !126176
  br label %19

19:                                               ; preds = %17, %2
  %20 = phi i64 [ %13, %2 ], [ %18, %17 ]
  %21 = getelementptr inbounds nuw i8, ptr %0, i64 8
  %22 = load ptr, ptr %21, align 8, !alias.scope !126173, !noalias !126176, !nonnull !1740, !noundef !1740
  %23 = icmp eq ptr %5, %7
  br i1 %23, label %.loopexit18, label %24

24:                                               ; preds = %19
  %25 = getelementptr inbounds nuw i8, ptr %1, i64 16
  %26 = load ptr, ptr %25, align 8, !alias.scope !126176, !noalias !126173, !nonnull !1740, !noundef !1740
  %27 = getelementptr inbounds nuw i8, ptr %26, i64 8
  %28 = getelementptr inbounds nuw i8, ptr %26, i64 16
  %29 = getelementptr inbounds nuw i8, ptr %3, i64 16
  %30 = getelementptr inbounds nuw i8, ptr %3, i64 8
  br label %31

31:                                               ; preds = %.loopexit, %24
  %32 = phi i64 [ %20, %24 ], [ %149, %.loopexit ]
  %33 = phi i64 [ 0, %24 ], [ %150, %.loopexit ]
  %34 = getelementptr inbounds nuw [40 x i8], ptr %5, i64 %33
  call void @llvm.experimental.noalias.scope.decl(metadata !126181)
  call void @llvm.lifetime.start.p0(ptr nonnull %4)
  call void @llvm.experimental.noalias.scope.decl(metadata !126184)
  %35 = load i64, ptr %34, align 8, !range !1778, !alias.scope !126187, !noalias !126188, !noundef !1740
  %36 = add i64 %35, -1
  %37 = icmp ugt i64 %36, 4
  %38 = getelementptr inbounds nuw i8, ptr %34, i64 8
  %39 = load ptr, ptr %38, align 8, !alias.scope !126187, !noalias !126188, !nonnull !1740
  %40 = getelementptr inbounds nuw i8, ptr %34, i64 16
  %41 = load i64, ptr %40, align 8, !alias.scope !126187, !noalias !126188
  %42 = add i64 %41, -1
  %43 = select i1 %37, i64 %42, i64 %36
  %44 = select i1 %37, ptr %39, ptr %38
  call void @llvm.lifetime.start.p0(ptr nonnull %3), !noalias !126199
  store i64 1, ptr %3, align 8, !noalias !126199
  %45 = load ptr, ptr %27, align 8, !noalias !126199, !nonnull !1740, !noundef !1740
  %46 = load i64, ptr %28, align 8, !noalias !126199, !noundef !1740
  %47 = getelementptr inbounds nuw [16 x i8], ptr %45, i64 %46
  %48 = icmp samesign ugt i64 %46, 4
  br i1 %48, label %49, label %54, !prof !1742

49:                                               ; preds = %31
; invoke <purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow
  invoke void @<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %3, i64 noundef 0, i64 noundef %46, i1 noundef zeroext true) #92
          to label %50 unwind label %136, !noalias !126199

50:                                               ; preds = %49
  %51 = load i64, ptr %3, align 8, !range !1778, !alias.scope !126200, !noalias !126203
  %52 = load ptr, ptr %30, align 8, !alias.scope !126200, !noalias !126203
  %53 = add i64 %51, -1
  br label %54

54:                                               ; preds = %50, %31
  %55 = phi ptr [ %52, %50 ], [ undef, %31 ]
  %56 = phi i64 [ %53, %50 ], [ 0, %31 ]
  %57 = icmp ugt i64 %56, 4
  %58 = call i64 @llvm.umax.i64(i64 %56, i64 4)
  %59 = select i1 %57, ptr %55, ptr %30
  %60 = select i1 %57, ptr %29, ptr %3
  %61 = load i64, ptr %60, align 8, !alias.scope !126200, !noalias !126203, !noundef !1740
  %62 = add i64 %61, -1
  %63 = icmp ult i64 %62, %58
  br i1 %63, label %.preheader17, label %66

64:                                               ; preds = %124
  %65 = add nuw i64 %58, 1
  br label %66

66:                                               ; preds = %64, %54
  %67 = phi ptr [ %45, %54 ], [ %74, %64 ]
  %68 = phi i64 [ %61, %54 ], [ %65, %64 ]
  store i64 %68, ptr %60, align 8, !noalias !126199
  %69 = icmp eq ptr %67, %47
  br i1 %69, label %.loopexit, label %.preheader

.preheader17:                                     ; preds = %54, %124
  %70 = phi i64 [ %127, %124 ], [ %62, %54 ]
  %71 = phi ptr [ %74, %124 ], [ %45, %54 ]
  %72 = icmp eq ptr %71, %47
  br i1 %72, label %129, label %73

73:                                               ; preds = %.preheader17
  %74 = getelementptr inbounds nuw i8, ptr %71, i64 16
  %75 = load i64, ptr %71, align 8, !range !1739, !noalias !126205, !noundef !1740
  %76 = getelementptr i8, ptr %71, i64 8
  %77 = load i64, ptr %76, align 8, !noalias !126205
  %78 = trunc nuw i64 %75 to i1
  br i1 %78, label %79, label %124

79:                                               ; preds = %73
  %80 = icmp ult i64 %77, %43
  br i1 %80, label %81, label %84

81:                                               ; preds = %79
  %82 = getelementptr inbounds nuw [8 x i8], ptr %44, i64 %77
  %83 = load <2 x i32>, ptr %82, align 4, !noalias !126208
  br label %124

84:                                               ; preds = %79
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %77, i64 noundef %43, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.791) #89
          to label %85 unwind label %131, !noalias !126199

85:                                               ; preds = %84
  unreachable

.preheader:                                       ; preds = %66, %118
  %86 = phi ptr [ %87, %118 ], [ %67, %66 ]
  %87 = getelementptr inbounds nuw i8, ptr %86, i64 16
  %88 = load i64, ptr %86, align 8, !range !1739, !noalias !126209, !noundef !1740
  %89 = getelementptr i8, ptr %86, i64 8
  %90 = load i64, ptr %89, align 8, !noalias !126209
  %91 = trunc nuw i64 %88 to i1
  br i1 %91, label %92, label %99

92:                                               ; preds = %.preheader
  %93 = icmp ult i64 %90, %43
  br i1 %93, label %94, label %97

94:                                               ; preds = %92
  %95 = getelementptr inbounds nuw [8 x i8], ptr %44, i64 %90
  %96 = load <2 x i32>, ptr %95, align 4, !noalias !126212
  br label %99

97:                                               ; preds = %92
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %90, i64 noundef %43, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.791) #89
          to label %98 unwind label %138, !noalias !126199

98:                                               ; preds = %97
  unreachable

99:                                               ; preds = %94, %.preheader
  %100 = phi <2 x i32> [ <i32 2, i32 undef>, %.preheader ], [ %96, %94 ]
  %101 = load i64, ptr %3, align 8, !range !1778, !alias.scope !126213, !noalias !126199, !noundef !1740
  %102 = add i64 %101, -1
  %103 = icmp ugt i64 %102, 4
  %104 = load ptr, ptr %30, align 8, !alias.scope !126213, !noalias !126199, !nonnull !1740
  %105 = select i1 %103, ptr %104, ptr %30
  %106 = select i1 %103, ptr %29, ptr %3
  %107 = call i64 @llvm.umax.i64(i64 %102, i64 4)
  %108 = load i64, ptr %106, align 8, !alias.scope !126213, !noalias !126199, !noundef !1740
  %109 = add i64 %108, -1
  %110 = icmp eq i64 %109, %107
  br i1 %110, label %111, label %118, !prof !1742

111:                                              ; preds = %99
; invoke <purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow
  invoke void @<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %3, i64 noundef %107, i64 noundef 1, i1 noundef zeroext true) #92
          to label %112 unwind label %134, !noalias !126199

112:                                              ; preds = %111
  %113 = load i64, ptr %3, align 8, !range !1778, !alias.scope !126213, !noalias !126199, !noundef !1740
  %114 = icmp ugt i64 %113, 5
  %115 = load ptr, ptr %30, align 8, !alias.scope !126213, !noalias !126199, !nonnull !1740
  %116 = select i1 %114, ptr %115, ptr %30
  %117 = select i1 %114, ptr %29, ptr %3
  br label %118

118:                                              ; preds = %112, %99
  %119 = phi ptr [ %116, %112 ], [ %105, %99 ]
  %120 = phi ptr [ %117, %112 ], [ %106, %99 ]
  %121 = getelementptr inbounds nuw [8 x i8], ptr %119, i64 %109
  store <2 x i32> %100, ptr %121, align 4, !noalias !126199
  %122 = add i64 %108, 1
  store i64 %122, ptr %120, align 8, !alias.scope !126213, !noalias !126199
  %123 = icmp eq ptr %87, %47
  br i1 %123, label %.loopexit, label %.preheader

124:                                              ; preds = %81, %73
  %125 = phi <2 x i32> [ <i32 2, i32 undef>, %73 ], [ %83, %81 ]
  %126 = getelementptr inbounds nuw [8 x i8], ptr %59, i64 %70
  store <2 x i32> %125, ptr %126, align 4, !noalias !126203
  %127 = add i64 %70, 1
  %128 = icmp eq i64 %127, %58
  br i1 %128, label %64, label %.preheader17

129:                                              ; preds = %.preheader17
  %130 = add nuw i64 %70, 1
  store i64 %130, ptr %60, align 8, !noalias !126199
  br label %.loopexit

131:                                              ; preds = %84
  %132 = landingpad { ptr, i32 }
          cleanup
  %133 = add nuw i64 %70, 1
  store i64 %133, ptr %60, align 8, !noalias !126199
  br label %140

134:                                              ; preds = %111
  %135 = landingpad { ptr, i32 }
          cleanup
  br label %140

136:                                              ; preds = %49
  %137 = landingpad { ptr, i32 }
          cleanup
  br label %140

138:                                              ; preds = %97
  %139 = landingpad { ptr, i32 }
          cleanup
  br label %140

140:                                              ; preds = %138, %136, %134, %131
  %141 = phi { ptr, i32 } [ %132, %131 ], [ %135, %134 ], [ %137, %136 ], [ %139, %138 ]
  %142 = load i64, ptr %3, align 8, !range !1778, !alias.scope !126216, !noalias !126199, !noundef !1740
  %143 = icmp ugt i64 %142, 5
  br i1 %143, label %144, label %152

144:                                              ; preds = %140
  %145 = load ptr, ptr %30, align 8, !noalias !126199, !nonnull !1740, !noundef !1740
  %146 = shl i64 %142, 3
  %147 = add i64 %146, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %145, i64 noundef %147, i64 noundef range(i64 1, -9223372036854775807) 4) #93, !noalias !126219
  br label %152

.loopexit:                                        ; preds = %118, %129, %66
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %4, ptr noundef nonnull align 8 dereferenceable(40) %3, i64 40, i1 false), !noalias !126222
  call void @llvm.lifetime.end.p0(ptr nonnull %3), !noalias !126199
  %148 = getelementptr inbounds nuw [40 x i8], ptr %22, i64 %32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %148, ptr noundef nonnull readonly align 8 dereferenceable(40) %4, i64 40, i1 false), !noalias !126223
  %149 = add i64 %32, 1
  call void @llvm.lifetime.end.p0(ptr nonnull %4)
  %150 = add nuw i64 %33, 1
  %151 = icmp eq i64 %150, %11
  br i1 %151, label %.loopexit18, label %31

152:                                              ; preds = %144, %140
  store i64 %32, ptr %12, align 8, !alias.scope !126173, !noalias !126228
  resume { ptr, i32 } %141

.loopexit18:                                      ; preds = %.loopexit, %19
  %153 = phi i64 [ %20, %19 ], [ %149, %.loopexit ]
  store i64 %153, ptr %12, align 8, !alias.scope !126173, !noalias !126228
  ret void
}
