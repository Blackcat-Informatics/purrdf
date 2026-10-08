define internal fastcc void @purrdf_sparql_eval::row_checkpoint::admit_rows::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>(ptr dead_on_unwind noalias nofree noundef nonnull writable writeonly align 16 captures(none) dereferenceable(96) %0, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(24) %2, ptr noalias nofree noundef nonnull align 8 captures(none) dead_on_return dereferenceable(24) %3) unnamed_addr #0 personality ptr @rust_eh_personality !guid !15573 {
  %5 = alloca [24 x i8], align 8
  %6 = alloca [48 x i8], align 8
  %7 = alloca [96 x i8], align 16
  %8 = alloca [32 x i8], align 8
  %9 = alloca [96 x i8], align 16
  %10 = alloca [24 x i8], align 8
  %11 = alloca [24 x i8], align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %11)
  call void @llvm.lifetime.start.p0(ptr nonnull %10)
  call void @llvm.lifetime.start.p0(ptr nonnull %9)
  %12 = getelementptr inbounds nuw i8, ptr %2, i64 16
  %13 = load i64, ptr %12, align 8, !noundef !1740
  %14 = icmp ult i64 %13, 230584300921369396
  tail call void @llvm.assume(i1 %14)
; invoke purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
  invoke fastcc void @purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %9, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %3, i64 noundef %13)
          to label %15 unwind label %177

15:                                               ; preds = %4
  %16 = load i64, ptr %9, align 16, !range !2527, !noundef !1740
  %17 = icmp eq i64 %16, -1
  %18 = getelementptr inbounds nuw i8, ptr %9, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %10, ptr noundef nonnull align 8 dereferenceable(24) %18, i64 24, i1 false)
  br i1 %17, label %23, label %19

19:                                               ; preds = %15
  %20 = getelementptr inbounds nuw i8, ptr %9, i64 32
  %21 = getelementptr inbounds nuw i8, ptr %0, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(64) %21, ptr noundef nonnull align 16 dereferenceable(64) %20, i64 64, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %9)
  %22 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %22, ptr noundef nonnull align 8 dereferenceable(24) %10, i64 24, i1 false)
  store i64 %16, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %10)
  call void @llvm.lifetime.end.p0(ptr nonnull %11)
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  tail call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(24) %2)
  br label %62

23:                                               ; preds = %15
  call void @llvm.lifetime.end.p0(ptr nonnull %9)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %11, ptr noundef nonnull align 8 dereferenceable(24) %10, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %10)
  %24 = getelementptr inbounds nuw i8, ptr %2, i64 8
  %25 = load ptr, ptr %24, align 8, !nonnull !1740, !noundef !1740
  %26 = load i64, ptr %2, align 8, !range !1835, !noundef !1740
  %27 = mul nuw nsw i64 %13, 40
  %28 = getelementptr inbounds nuw i8, ptr %25, i64 %27
  call void @llvm.lifetime.start.p0(ptr nonnull %8)
  store ptr %25, ptr %8, align 8
  %29 = getelementptr inbounds nuw i8, ptr %8, i64 8
  %30 = getelementptr inbounds nuw i8, ptr %8, i64 16
  store i64 %26, ptr %30, align 8
  %31 = getelementptr inbounds nuw i8, ptr %8, i64 24
  store ptr %28, ptr %31, align 8
  %32 = icmp eq i64 %13, 0
  br i1 %32, label %.loopexit9, label %33

33:                                               ; preds = %23
  %34 = getelementptr inbounds nuw i8, ptr %6, i64 8
  %35 = getelementptr inbounds nuw i8, ptr %7, i64 8
  %36 = getelementptr inbounds nuw i8, ptr %1, i64 664
  %37 = getelementptr inbounds nuw i8, ptr %1, i64 888
  %38 = getelementptr inbounds nuw i8, ptr %6, i64 16
  %39 = getelementptr inbounds nuw i8, ptr %7, i64 16
  %40 = getelementptr inbounds nuw i8, ptr %7, i64 24
  %41 = getelementptr inbounds nuw i8, ptr %11, i64 16
  %42 = getelementptr inbounds nuw i8, ptr %11, i64 8
  br label %47

43:                                               ; preds = %52
  %44 = landingpad { ptr, i32 }
          cleanup
  store ptr %49, ptr %29, align 8
  br label %45

45:                                               ; preds = %84, %81, %43
  %46 = phi { ptr, i32 } [ %44, %43 ], [ %82, %84 ], [ %82, %81 ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %8) #89
          to label %56 unwind label %173

47:                                               ; preds = %87, %33
  %48 = phi ptr [ %25, %33 ], [ %49, %87 ]
  %49 = getelementptr inbounds nuw i8, ptr %48, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %6)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %34, ptr noundef nonnull align 8 dereferenceable(40) %48, i64 40, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %7)
  store ptr %1, ptr %6, align 8
  tail call void @llvm.experimental.noalias.scope.decl(metadata !15574)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !15577)
  %50 = load i64, ptr %34, align 8, !alias.scope !15577, !noalias !15579, !noundef !1740
  %51 = icmp eq i64 %50, 0
  br i1 %51, label %52, label %54

52:                                               ; preds = %47
  %53 = load ptr, ptr %36, align 8, !alias.scope !15574, !noalias !15581, !nonnull !1740, !align !1836, !noundef !1740
; invoke purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(96) %7, ptr noalias nofree noundef align 8 dereferenceable(184) %37, ptr noundef nonnull align 8 %53, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %38)
          to label %63 unwind label %43

54:                                               ; preds = %47
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %35, ptr noundef nonnull align 8 dereferenceable(40) %48, i64 40, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %6)
  br label %74

.loopexit9:                                       ; preds = %87, %23
  %55 = phi ptr [ %25, %23 ], [ %28, %87 ]
  store ptr %55, ptr %29, align 8
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %8)
          to label %60 unwind label %58

56:                                               ; preds = %58, %45
  %57 = phi { ptr, i32 } [ %59, %58 ], [ %46, %45 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %11) #89
  br label %175

58:                                               ; preds = %66, %.loopexit9
  %59 = landingpad { ptr, i32 }
          cleanup
  br label %56

60:                                               ; preds = %.loopexit9
  call void @llvm.lifetime.end.p0(ptr nonnull %8)
  %61 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %61, ptr noundef nonnull align 8 dereferenceable(24) %11, i64 24, i1 false)
  store i64 -1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %11)
  br label %62

62:                                               ; preds = %172, %60, %19
  ret void

63:                                               ; preds = %52
  %64 = load i64, ptr %7, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %6)
  %65 = icmp eq i64 %64, -1
  br i1 %65, label %74, label %66

66:                                               ; preds = %63
  store ptr %49, ptr %29, align 8
  %67 = load i64, ptr %35, align 8
  %68 = load ptr, ptr %39, align 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %5, ptr noundef nonnull align 8 dereferenceable(24) %40, i64 24, i1 false)
  %69 = getelementptr inbounds nuw i8, ptr %7, i64 48
  %70 = getelementptr inbounds nuw i8, ptr %0, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %70, ptr noundef nonnull align 16 dereferenceable(48) %69, i64 48, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %7)
  %71 = getelementptr inbounds nuw i8, ptr %0, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %71, ptr noundef nonnull align 8 dereferenceable(24) %5, i64 24, i1 false)
  store i64 %64, ptr %0, align 16
  %72 = getelementptr inbounds nuw i8, ptr %0, i64 8
  store i64 %67, ptr %72, align 8
  %73 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store ptr %68, ptr %73, align 16
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %8)
          to label %94 unwind label %58

74:                                               ; preds = %63, %54
  %75 = load i64, ptr %35, align 8
  %76 = load ptr, ptr %39, align 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %5, ptr noundef nonnull align 8 dereferenceable(24) %40, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %7)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !15582)
  %77 = load i64, ptr %41, align 8, !alias.scope !15582, !noalias !15585, !noundef !1740
  %78 = load i64, ptr %11, align 8, !range !1835, !alias.scope !15582, !noalias !15585, !noundef !1740
  %79 = icmp eq i64 %77, %78
  br i1 %79, label %80, label %87

80:                                               ; preds = %74
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %11)
          to label %87 unwind label %81, !noalias !15585

81:                                               ; preds = %80
  %82 = landingpad { ptr, i32 }
          cleanup
  store ptr %49, ptr %29, align 8
  %83 = icmp ugt i64 %75, 5
  br i1 %83, label %84, label %45

84:                                               ; preds = %81
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %76) ]
  %85 = shl i64 %75, 3
  %86 = add i64 %85, -8
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %76, i64 noundef %86, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !15587
  br label %45

87:                                               ; preds = %80, %74
  %88 = load ptr, ptr %42, align 8, !alias.scope !15582, !noalias !15585, !nonnull !1740, !noundef !1740
  %89 = getelementptr inbounds nuw [40 x i8], ptr %88, i64 %77
  store i64 %75, ptr %89, align 8, !noalias !15582
  %90 = getelementptr inbounds nuw i8, ptr %89, i64 8
  store ptr %76, ptr %90, align 8, !noalias !15582
  %91 = getelementptr inbounds nuw i8, ptr %89, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %91, ptr noundef nonnull align 8 dereferenceable(24) %5, i64 24, i1 false)
  %92 = add i64 %77, 1
  store i64 %92, ptr %41, align 8, !alias.scope !15582, !noalias !15585
  %93 = icmp eq ptr %49, %28
  br i1 %93, label %.loopexit9, label %47

94:                                               ; preds = %66
  call void @llvm.lifetime.end.p0(ptr nonnull %8)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !15590)
  %95 = load ptr, ptr %42, align 8, !alias.scope !15590, !nonnull !1740, !noundef !1740
  %96 = load i64, ptr %41, align 8, !alias.scope !15590, !noundef !1740
  tail call void @llvm.experimental.noalias.scope.decl(metadata !15593)
  %97 = icmp eq i64 %96, 0
  br i1 %97, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %94
  %98 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %99 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %100

100:                                              ; preds = %.preheader, %138
  %101 = phi i64 [ %103, %138 ], [ 0, %.preheader ]
  %102 = getelementptr inbounds nuw [40 x i8], ptr %95, i64 %101
  %103 = add nuw nsw i64 %101, 1
  %104 = load i64, ptr %102, align 8, !range !1778, !alias.scope !15596, !noalias !15590, !noundef !1740
  %105 = icmp ugt i64 %104, 5
  br i1 %105, label %106, label %138

106:                                              ; preds = %100
  %107 = getelementptr i8, ptr %102, i64 8
  %108 = load ptr, ptr %107, align 8, !alias.scope !15593, !noalias !15590, !nonnull !1740, !noundef !1740
  %109 = shl i64 %104, 3
  %110 = add i64 %109, -8
  %111 = load i64, ptr %98, align 8, !noalias !15599, !noundef !1740
  %112 = tail call i64 @llvm.umin.i64(i64 %110, i64 9223372036854775807)
  %113 = tail call i64 @llvm.ssub.sat.i64(i64 %111, i64 %112)
  store i64 %113, ptr %98, align 8, !noalias !15599
  %114 = load i64, ptr %99, align 8, !noalias !15599, !noundef !1740
  %115 = icmp slt i64 %113, %114
  br i1 %115, label %116, label %.preheader59

116:                                              ; preds = %106
  store i64 %113, ptr %99, align 8, !noalias !15599
  br label %.preheader59

.preheader59:                                     ; preds = %116, %106
  br label %117

117:                                              ; preds = %.preheader59, %120
  %118 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !15599
  %119 = icmp slt i64 %118, 0
  br i1 %119, label %120, label %__rustc::__rust_dealloc (.exit)

120:                                              ; preds = %117
  %121 = add nsw i64 %118, 1
  %122 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %118, i64 %121 acq_rel acquire, align 8, !noalias !15599
  %123 = extractvalue { i64, i1 } %122, 1
  br i1 %123, label %124, label %117

124:                                              ; preds = %120
  %125 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %112 monotonic, align 8, !noalias !15599
  %126 = tail call i64 @llvm.ssub.sat.i64(i64 %125, i64 %112)
  %127 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !15599
  br label %128

128:                                              ; preds = %131, %124
  %129 = phi i64 [ %127, %124 ], [ %134, %131 ]
  %130 = icmp slt i64 %126, %129
  br i1 %130, label %131, label %135

131:                                              ; preds = %128
  %132 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %129, i64 %126 monotonic monotonic, align 8, !noalias !15599
  %133 = extractvalue { i64, i1 } %132, 1
  %134 = extractvalue { i64, i1 } %132, 0
  br i1 %133, label %135, label %128

135:                                              ; preds = %131, %128
  %136 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !15599
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %117, %135
  %137 = icmp ne i64 %110, 0
  tail call void @llvm.assume(i1 %137), !noalias !15599
  tail call void @free(ptr noundef nonnull %108) #92, !noalias !15599
  br label %138

138:                                              ; preds = %__rustc::__rust_dealloc (.exit), %100
  %139 = icmp eq i64 %103, %96
  br i1 %139, label %.loopexit, label %100

.loopexit:                                        ; preds = %138, %94
  %140 = load i64, ptr %11, align 8, !alias.scope !15590
  %141 = icmp eq i64 %140, 0
  br i1 %141, label %172, label %142

142:                                              ; preds = %.loopexit
  %143 = mul nuw i64 %140, 40
  %144 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %145 = load i64, ptr %144, align 8, !noalias !15590, !noundef !1740
  %146 = tail call i64 @llvm.umin.i64(i64 %143, i64 9223372036854775807)
  %147 = tail call i64 @llvm.ssub.sat.i64(i64 %145, i64 %146)
  store i64 %147, ptr %144, align 8, !noalias !15590
  %148 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %149 = load i64, ptr %148, align 8, !noalias !15590, !noundef !1740
  %150 = icmp slt i64 %147, %149
  br i1 %150, label %151, label %.preheader58

151:                                              ; preds = %142
  store i64 %147, ptr %148, align 8, !noalias !15590
  br label %.preheader58

.preheader58:                                     ; preds = %151, %142
  br label %152

152:                                              ; preds = %.preheader58, %155
  %153 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !15590
  %154 = icmp slt i64 %153, 0
  br i1 %154, label %155, label %__rustc::__rust_dealloc (.exit8)

155:                                              ; preds = %152
  %156 = add nsw i64 %153, 1
  %157 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %153, i64 %156 acq_rel acquire, align 8, !noalias !15590
  %158 = extractvalue { i64, i1 } %157, 1
  br i1 %158, label %159, label %152

159:                                              ; preds = %155
  %160 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %146 monotonic, align 8, !noalias !15590
  %161 = tail call i64 @llvm.ssub.sat.i64(i64 %160, i64 %146)
  %162 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !15590
  br label %163

163:                                              ; preds = %166, %159
  %164 = phi i64 [ %162, %159 ], [ %169, %166 ]
  %165 = icmp slt i64 %161, %164
  br i1 %165, label %166, label %170

166:                                              ; preds = %163
  %167 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %164, i64 %161 monotonic monotonic, align 8, !noalias !15590
  %168 = extractvalue { i64, i1 } %167, 1
  %169 = extractvalue { i64, i1 } %167, 0
  br i1 %168, label %170, label %163

170:                                              ; preds = %166, %163
  %171 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !15590
  br label %__rustc::__rust_dealloc (.exit8)

__rustc::__rust_dealloc (.exit8): ; preds = %152, %170
  tail call void @free(ptr noundef nonnull %95) #92, !noalias !15590
  br label %172

172:                                              ; preds = %__rustc::__rust_dealloc (.exit8), %.loopexit
  call void @llvm.lifetime.end.p0(ptr nonnull %11)
  br label %62

173:                                              ; preds = %177, %45
  %174 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #90
  unreachable

175:                                              ; preds = %177, %56
  %176 = phi { ptr, i32 } [ %178, %177 ], [ %57, %56 ]
  resume { ptr, i32 } %176

177:                                              ; preds = %4
  %178 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(24) %2) #89
          to label %175 unwind label %173
}
define internal fastcc void @purrdf_sparql_eval::row_checkpoint::admit_rows::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, &mut purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>(ptr dead_on_unwind noalias nofree noundef nonnull writable writeonly align 16 captures(none) dereferenceable(96) %0, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(24) %2, ptr noalias nofree noundef nonnull align 8 captures(none) dead_on_return dereferenceable(24) %3) unnamed_addr #0 personality ptr @rust_eh_personality !guid !15602 {
  %5 = alloca [48 x i8], align 8
  %6 = alloca [24 x i8], align 8
  %7 = alloca [96 x i8], align 16
  %8 = alloca [32 x i8], align 8
  %9 = alloca [96 x i8], align 16
  %10 = alloca [24 x i8], align 8
  %11 = alloca [24 x i8], align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %11)
  call void @llvm.lifetime.start.p0(ptr nonnull %10)
  call void @llvm.lifetime.start.p0(ptr nonnull %9)
  %12 = getelementptr inbounds nuw i8, ptr %2, i64 16
  %13 = load i64, ptr %12, align 8, !noundef !1740
  %14 = icmp ult i64 %13, 230584300921369396
  tail call void @llvm.assume(i1 %14)
; invoke purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
  invoke fastcc void @purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %9, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %3, i64 noundef %13)
          to label %15 unwind label %177

15:                                               ; preds = %4
  %16 = load i64, ptr %9, align 16, !range !2527, !noundef !1740
  %17 = icmp eq i64 %16, -1
  %18 = getelementptr inbounds nuw i8, ptr %9, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %10, ptr noundef nonnull align 8 dereferenceable(24) %18, i64 24, i1 false)
  br i1 %17, label %23, label %19

19:                                               ; preds = %15
  %20 = getelementptr inbounds nuw i8, ptr %9, i64 32
  %21 = getelementptr inbounds nuw i8, ptr %0, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(64) %21, ptr noundef nonnull align 16 dereferenceable(64) %20, i64 64, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %9)
  %22 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %22, ptr noundef nonnull align 8 dereferenceable(24) %10, i64 24, i1 false)
  store i64 %16, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %10)
  call void @llvm.lifetime.end.p0(ptr nonnull %11)
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  tail call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(24) %2)
  br label %62

23:                                               ; preds = %15
  call void @llvm.lifetime.end.p0(ptr nonnull %9)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %11, ptr noundef nonnull align 8 dereferenceable(24) %10, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %10)
  %24 = getelementptr inbounds nuw i8, ptr %2, i64 8
  %25 = load ptr, ptr %24, align 8, !nonnull !1740, !noundef !1740
  %26 = load i64, ptr %2, align 8, !range !1835, !noundef !1740
  %27 = mul nuw nsw i64 %13, 40
  %28 = getelementptr inbounds nuw i8, ptr %25, i64 %27
  call void @llvm.lifetime.start.p0(ptr nonnull %8)
  store ptr %25, ptr %8, align 8
  %29 = getelementptr inbounds nuw i8, ptr %8, i64 8
  %30 = getelementptr inbounds nuw i8, ptr %8, i64 16
  store i64 %26, ptr %30, align 8
  %31 = getelementptr inbounds nuw i8, ptr %8, i64 24
  store ptr %28, ptr %31, align 8
  %32 = icmp eq i64 %13, 0
  br i1 %32, label %.loopexit9, label %33

33:                                               ; preds = %23
  %34 = getelementptr inbounds nuw i8, ptr %5, i64 8
  %35 = getelementptr inbounds nuw i8, ptr %7, i64 8
  %36 = getelementptr inbounds nuw i8, ptr %1, i64 664
  %37 = getelementptr inbounds nuw i8, ptr %1, i64 888
  %38 = getelementptr inbounds nuw i8, ptr %5, i64 16
  %39 = getelementptr inbounds nuw i8, ptr %7, i64 16
  %40 = getelementptr inbounds nuw i8, ptr %7, i64 24
  %41 = getelementptr inbounds nuw i8, ptr %11, i64 16
  %42 = getelementptr inbounds nuw i8, ptr %11, i64 8
  br label %47

43:                                               ; preds = %52
  %44 = landingpad { ptr, i32 }
          cleanup
  store ptr %49, ptr %29, align 8
  br label %45

45:                                               ; preds = %84, %81, %43
  %46 = phi { ptr, i32 } [ %44, %43 ], [ %82, %84 ], [ %82, %81 ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %8) #89
          to label %56 unwind label %173

47:                                               ; preds = %87, %33
  %48 = phi ptr [ %25, %33 ], [ %49, %87 ]
  %49 = getelementptr inbounds nuw i8, ptr %48, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %7)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !15603)
  call void @llvm.lifetime.start.p0(ptr nonnull %5)
  store ptr %1, ptr %5, align 8, !noalias !15606
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %34, ptr noundef nonnull align 8 dereferenceable(40) %48, i64 40, i1 false)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !15609)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !15612)
  %50 = load i64, ptr %34, align 8, !alias.scope !15612, !noalias !15614, !noundef !1740
  %51 = icmp eq i64 %50, 0
  br i1 %51, label %52, label %54

52:                                               ; preds = %47
  %53 = load ptr, ptr %36, align 8, !alias.scope !15616, !noalias !15617, !nonnull !1740, !align !1836, !noundef !1740
; invoke purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(96) %7, ptr noalias nofree noundef align 8 dereferenceable(184) %37, ptr noundef nonnull align 8 %53, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %38)
          to label %63 unwind label %43

54:                                               ; preds = %47
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %35, ptr noundef nonnull align 8 dereferenceable(40) %48, i64 40, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %5)
  br label %74

.loopexit9:                                       ; preds = %87, %23
  %55 = phi ptr [ %25, %23 ], [ %28, %87 ]
  store ptr %55, ptr %29, align 8
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %8)
          to label %60 unwind label %58

56:                                               ; preds = %58, %45
  %57 = phi { ptr, i32 } [ %59, %58 ], [ %46, %45 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %11) #89
  br label %175

58:                                               ; preds = %66, %.loopexit9
  %59 = landingpad { ptr, i32 }
          cleanup
  br label %56

60:                                               ; preds = %.loopexit9
  call void @llvm.lifetime.end.p0(ptr nonnull %8)
  %61 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %61, ptr noundef nonnull align 8 dereferenceable(24) %11, i64 24, i1 false)
  store i64 -1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %11)
  br label %62

62:                                               ; preds = %172, %60, %19
  ret void

63:                                               ; preds = %52
  %64 = load i64, ptr %7, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %5)
  %65 = icmp eq i64 %64, -1
  br i1 %65, label %74, label %66

66:                                               ; preds = %63
  store ptr %49, ptr %29, align 8
  %67 = load i64, ptr %35, align 8
  %68 = load ptr, ptr %39, align 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %6, ptr noundef nonnull align 8 dereferenceable(24) %40, i64 24, i1 false)
  %69 = getelementptr inbounds nuw i8, ptr %7, i64 48
  %70 = getelementptr inbounds nuw i8, ptr %0, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %70, ptr noundef nonnull align 16 dereferenceable(48) %69, i64 48, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %7)
  %71 = getelementptr inbounds nuw i8, ptr %0, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %71, ptr noundef nonnull align 8 dereferenceable(24) %6, i64 24, i1 false)
  store i64 %64, ptr %0, align 16
  %72 = getelementptr inbounds nuw i8, ptr %0, i64 8
  store i64 %67, ptr %72, align 8
  %73 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store ptr %68, ptr %73, align 16
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %8)
          to label %94 unwind label %58

74:                                               ; preds = %63, %54
  %75 = load i64, ptr %35, align 8
  %76 = load ptr, ptr %39, align 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %6, ptr noundef nonnull align 8 dereferenceable(24) %40, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %7)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !15618)
  %77 = load i64, ptr %41, align 8, !alias.scope !15618, !noalias !15621, !noundef !1740
  %78 = load i64, ptr %11, align 8, !range !1835, !alias.scope !15618, !noalias !15621, !noundef !1740
  %79 = icmp eq i64 %77, %78
  br i1 %79, label %80, label %87

80:                                               ; preds = %74
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %11)
          to label %87 unwind label %81, !noalias !15621

81:                                               ; preds = %80
  %82 = landingpad { ptr, i32 }
          cleanup
  store ptr %49, ptr %29, align 8
  %83 = icmp ugt i64 %75, 5
  br i1 %83, label %84, label %45

84:                                               ; preds = %81
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %76) ]
  %85 = shl i64 %75, 3
  %86 = add i64 %85, -8
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %76, i64 noundef %86, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !15623
  br label %45

87:                                               ; preds = %80, %74
  %88 = load ptr, ptr %42, align 8, !alias.scope !15618, !noalias !15621, !nonnull !1740, !noundef !1740
  %89 = getelementptr inbounds nuw [40 x i8], ptr %88, i64 %77
  store i64 %75, ptr %89, align 8, !noalias !15618
  %90 = getelementptr inbounds nuw i8, ptr %89, i64 8
  store ptr %76, ptr %90, align 8, !noalias !15618
  %91 = getelementptr inbounds nuw i8, ptr %89, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %91, ptr noundef nonnull align 8 dereferenceable(24) %6, i64 24, i1 false)
  %92 = add i64 %77, 1
  store i64 %92, ptr %41, align 8, !alias.scope !15618, !noalias !15621
  %93 = icmp eq ptr %49, %28
  br i1 %93, label %.loopexit9, label %47

94:                                               ; preds = %66
  call void @llvm.lifetime.end.p0(ptr nonnull %8)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !15626)
  %95 = load ptr, ptr %42, align 8, !alias.scope !15626, !nonnull !1740, !noundef !1740
  %96 = load i64, ptr %41, align 8, !alias.scope !15626, !noundef !1740
  tail call void @llvm.experimental.noalias.scope.decl(metadata !15629)
  %97 = icmp eq i64 %96, 0
  br i1 %97, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %94
  %98 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %99 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %100

100:                                              ; preds = %.preheader, %138
  %101 = phi i64 [ %103, %138 ], [ 0, %.preheader ]
  %102 = getelementptr inbounds nuw [40 x i8], ptr %95, i64 %101
  %103 = add nuw nsw i64 %101, 1
  %104 = load i64, ptr %102, align 8, !range !1778, !alias.scope !15632, !noalias !15626, !noundef !1740
  %105 = icmp ugt i64 %104, 5
  br i1 %105, label %106, label %138

106:                                              ; preds = %100
  %107 = getelementptr i8, ptr %102, i64 8
  %108 = load ptr, ptr %107, align 8, !alias.scope !15629, !noalias !15626, !nonnull !1740, !noundef !1740
  %109 = shl i64 %104, 3
  %110 = add i64 %109, -8
  %111 = load i64, ptr %98, align 8, !noalias !15635, !noundef !1740
  %112 = tail call i64 @llvm.umin.i64(i64 %110, i64 9223372036854775807)
  %113 = tail call i64 @llvm.ssub.sat.i64(i64 %111, i64 %112)
  store i64 %113, ptr %98, align 8, !noalias !15635
  %114 = load i64, ptr %99, align 8, !noalias !15635, !noundef !1740
  %115 = icmp slt i64 %113, %114
  br i1 %115, label %116, label %.preheader59

116:                                              ; preds = %106
  store i64 %113, ptr %99, align 8, !noalias !15635
  br label %.preheader59

.preheader59:                                     ; preds = %116, %106
  br label %117

117:                                              ; preds = %.preheader59, %120
  %118 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !15635
  %119 = icmp slt i64 %118, 0
  br i1 %119, label %120, label %__rustc::__rust_dealloc (.exit)

120:                                              ; preds = %117
  %121 = add nsw i64 %118, 1
  %122 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %118, i64 %121 acq_rel acquire, align 8, !noalias !15635
  %123 = extractvalue { i64, i1 } %122, 1
  br i1 %123, label %124, label %117

124:                                              ; preds = %120
  %125 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %112 monotonic, align 8, !noalias !15635
  %126 = tail call i64 @llvm.ssub.sat.i64(i64 %125, i64 %112)
  %127 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !15635
  br label %128

128:                                              ; preds = %131, %124
  %129 = phi i64 [ %127, %124 ], [ %134, %131 ]
  %130 = icmp slt i64 %126, %129
  br i1 %130, label %131, label %135

131:                                              ; preds = %128
  %132 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %129, i64 %126 monotonic monotonic, align 8, !noalias !15635
  %133 = extractvalue { i64, i1 } %132, 1
  %134 = extractvalue { i64, i1 } %132, 0
  br i1 %133, label %135, label %128

135:                                              ; preds = %131, %128
  %136 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !15635
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %117, %135
  %137 = icmp ne i64 %110, 0
  tail call void @llvm.assume(i1 %137), !noalias !15635
  tail call void @free(ptr noundef nonnull %108) #92, !noalias !15635
  br label %138

138:                                              ; preds = %__rustc::__rust_dealloc (.exit), %100
  %139 = icmp eq i64 %103, %96
  br i1 %139, label %.loopexit, label %100

.loopexit:                                        ; preds = %138, %94
  %140 = load i64, ptr %11, align 8, !alias.scope !15626
  %141 = icmp eq i64 %140, 0
  br i1 %141, label %172, label %142

142:                                              ; preds = %.loopexit
  %143 = mul nuw i64 %140, 40
  %144 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %145 = load i64, ptr %144, align 8, !noalias !15626, !noundef !1740
  %146 = tail call i64 @llvm.umin.i64(i64 %143, i64 9223372036854775807)
  %147 = tail call i64 @llvm.ssub.sat.i64(i64 %145, i64 %146)
  store i64 %147, ptr %144, align 8, !noalias !15626
  %148 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %149 = load i64, ptr %148, align 8, !noalias !15626, !noundef !1740
  %150 = icmp slt i64 %147, %149
  br i1 %150, label %151, label %.preheader58

151:                                              ; preds = %142
  store i64 %147, ptr %148, align 8, !noalias !15626
  br label %.preheader58

.preheader58:                                     ; preds = %151, %142
  br label %152

152:                                              ; preds = %.preheader58, %155
  %153 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !15626
  %154 = icmp slt i64 %153, 0
  br i1 %154, label %155, label %__rustc::__rust_dealloc (.exit8)

155:                                              ; preds = %152
  %156 = add nsw i64 %153, 1
  %157 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %153, i64 %156 acq_rel acquire, align 8, !noalias !15626
  %158 = extractvalue { i64, i1 } %157, 1
  br i1 %158, label %159, label %152

159:                                              ; preds = %155
  %160 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %146 monotonic, align 8, !noalias !15626
  %161 = tail call i64 @llvm.ssub.sat.i64(i64 %160, i64 %146)
  %162 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !15626
  br label %163

163:                                              ; preds = %166, %159
  %164 = phi i64 [ %162, %159 ], [ %169, %166 ]
  %165 = icmp slt i64 %161, %164
  br i1 %165, label %166, label %170

166:                                              ; preds = %163
  %167 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %164, i64 %161 monotonic monotonic, align 8, !noalias !15626
  %168 = extractvalue { i64, i1 } %167, 1
  %169 = extractvalue { i64, i1 } %167, 0
  br i1 %168, label %170, label %163

170:                                              ; preds = %166, %163
  %171 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !15626
  br label %__rustc::__rust_dealloc (.exit8)

__rustc::__rust_dealloc (.exit8): ; preds = %152, %170
  tail call void @free(ptr noundef nonnull %95) #92, !noalias !15626
  br label %172

172:                                              ; preds = %__rustc::__rust_dealloc (.exit8), %.loopexit
  call void @llvm.lifetime.end.p0(ptr nonnull %11)
  br label %62

173:                                              ; preds = %177, %45
  %174 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #90
  unreachable

175:                                              ; preds = %177, %56
  %176 = phi { ptr, i32 } [ %178, %177 ], [ %57, %56 ]
  resume { ptr, i32 } %176

177:                                              ; preds = %4
  %178 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(24) %2) #89
          to label %175 unwind label %173
}
define internal fastcc void @purrdf_sparql_eval::row_checkpoint::commit_items::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>(ptr dead_on_unwind noalias nofree noundef nonnull writable writeonly align 16 captures(none) dereferenceable(96) %0, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %1, i8 noundef range(i8 0, 17) %2, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(32) %3, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(24) %4, i64 noundef range(i64 0, 2) %5) unnamed_addr #0 personality ptr @rust_eh_personality !guid !15905 {
  %7 = alloca [24 x i8], align 8
  %8 = alloca [24 x i8], align 8
  %9 = alloca [24 x i8], align 8
  %10 = alloca [24 x i8], align 8
  %11 = alloca [24 x i8], align 8
  %12 = alloca [80 x i8], align 8
  %13 = alloca [24 x i8], align 8
  %14 = alloca [24 x i8], align 8
  %15 = alloca [80 x i8], align 8
  %16 = alloca [80 x i8], align 8
  %17 = alloca [24 x i8], align 8
  %18 = alloca [80 x i8], align 8
  %19 = alloca [80 x i8], align 8
  %20 = alloca [24 x i8], align 8
  %21 = alloca [24 x i8], align 8
  %22 = alloca [80 x i8], align 8
  %23 = alloca [24 x i8], align 8
  %24 = alloca [24 x i8], align 8
  %25 = alloca [32 x i8], align 8
  %26 = alloca [8 x i8], align 8
  %27 = alloca [8 x i8], align 8
  %28 = alloca [24 x i8], align 8
  %29 = alloca [48 x i8], align 8
  %30 = alloca [96 x i8], align 16
  %31 = alloca [24 x i8], align 8
  %32 = alloca [24 x i8], align 8
  %33 = alloca [24 x i8], align 8
  %34 = alloca [24 x i8], align 8
  %35 = alloca [24 x i8], align 8
  %36 = alloca [24 x i8], align 8
  %37 = alloca [32 x i8], align 8
  %38 = alloca [160 x i8], align 8
  %39 = alloca [32 x i8], align 8
  %40 = alloca [8 x i8], align 8
  %41 = alloca [24 x i8], align 8
  %42 = alloca [48 x i8], align 8
  %43 = alloca [96 x i8], align 16
  %44 = alloca [32 x i8], align 8
  %45 = alloca [32 x i8], align 8
  %46 = alloca [24 x i8], align 8
  %47 = alloca [96 x i8], align 16
  %48 = alloca [24 x i8], align 8
  %49 = alloca [24 x i8], align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %49)
  call void @llvm.lifetime.start.p0(ptr nonnull %48)
  call void @llvm.lifetime.start.p0(ptr nonnull %47)
  call void @llvm.lifetime.start.p0(ptr nonnull %46)
  store i64 0, ptr %46, align 8
  %50 = getelementptr inbounds nuw i8, ptr %46, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %50, align 8
  %51 = getelementptr inbounds nuw i8, ptr %46, i64 16
  store i64 0, ptr %51, align 8
  %52 = getelementptr inbounds nuw i8, ptr %4, i64 16
  %53 = load i64, ptr %52, align 8, !noundef !1740
  %54 = icmp ult i64 %53, 230584300921369396
  tail call void @llvm.assume(i1 %54)
; invoke purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
  invoke fastcc void @purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %47, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %46, i64 noundef %53)
          to label %55 unwind label %1225

55:                                               ; preds = %6
  call void @llvm.lifetime.end.p0(ptr nonnull %46)
  %56 = load i64, ptr %47, align 16, !range !2527, !noundef !1740
  %57 = icmp eq i64 %56, -1
  %58 = getelementptr inbounds nuw i8, ptr %47, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %48, ptr noundef nonnull align 8 dereferenceable(24) %58, i64 24, i1 false)
  br i1 %57, label %63, label %59

59:                                               ; preds = %55
  %60 = getelementptr inbounds nuw i8, ptr %47, i64 32
  %61 = getelementptr inbounds nuw i8, ptr %0, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(64) %61, ptr noundef nonnull align 16 dereferenceable(64) %60, i64 64, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %47)
  %62 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %62, ptr noundef nonnull align 8 dereferenceable(24) %48, i64 24, i1 false)
  store i64 %56, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %48)
  call void @llvm.lifetime.end.p0(ptr nonnull %49)
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(24) %4)
          to label %1224 unwind label %1222

63:                                               ; preds = %55
  call void @llvm.lifetime.end.p0(ptr nonnull %47)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %49, ptr noundef nonnull align 8 dereferenceable(24) %48, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %48)
  call void @llvm.lifetime.start.p0(ptr nonnull %45)
  %64 = getelementptr inbounds nuw i8, ptr %4, i64 8
  %65 = load ptr, ptr %64, align 8, !nonnull !1740, !noundef !1740
  %66 = load i64, ptr %4, align 8, !range !1835, !noundef !1740
  %67 = getelementptr inbounds nuw [40 x i8], ptr %65, i64 %53
  store ptr %65, ptr %45, align 8
  %68 = getelementptr inbounds nuw i8, ptr %45, i64 16
  store i64 %66, ptr %68, align 8
  %69 = getelementptr inbounds nuw i8, ptr %45, i64 8
  store ptr %65, ptr %69, align 8
  %70 = getelementptr inbounds nuw i8, ptr %45, i64 24
  store ptr %67, ptr %70, align 8
  %71 = getelementptr inbounds nuw i8, ptr %1, i64 616
  %72 = load ptr, ptr %71, align 8, !noundef !1740
  %73 = icmp eq ptr %72, null
  br i1 %73, label %77, label %74

74:                                               ; preds = %63
  %75 = atomicrmw add ptr %72, i64 1 monotonic, align 8
  %76 = icmp slt i64 %75, 0
  br i1 %76, label %173, label %161

77:                                               ; preds = %63
  call void @llvm.lifetime.start.p0(ptr nonnull %44)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %44, ptr noundef nonnull align 8 dereferenceable(32) %45, i64 32, i1 false)
  %78 = getelementptr inbounds nuw i8, ptr %44, i64 24
  %79 = load ptr, ptr %78, align 8, !alias.scope !15906, !noalias !15909, !nonnull !1740, !noundef !1740
  %80 = getelementptr inbounds nuw i8, ptr %44, i64 8
  %81 = load ptr, ptr %80, align 8, !alias.scope !15906, !noalias !15909
  %82 = icmp eq ptr %81, %79
  br i1 %82, label %.loopexit129, label %83

83:                                               ; preds = %77
  %84 = getelementptr inbounds nuw i8, ptr %42, i64 8
  %85 = getelementptr inbounds nuw i8, ptr %43, i64 8
  %86 = getelementptr inbounds nuw i8, ptr %1, i64 664
  %87 = getelementptr inbounds nuw i8, ptr %1, i64 888
  %88 = getelementptr inbounds nuw i8, ptr %42, i64 16
  %89 = getelementptr inbounds nuw i8, ptr %43, i64 16
  %90 = getelementptr inbounds nuw i8, ptr %43, i64 24
  %91 = getelementptr inbounds nuw i8, ptr %49, i64 16
  %92 = getelementptr inbounds nuw i8, ptr %49, i64 8
  br label %97

93:                                               ; preds = %102
  %94 = landingpad { ptr, i32 }
          cleanup
  store ptr %99, ptr %80, align 8
  br label %95

95:                                               ; preds = %145, %142, %93
  %96 = phi { ptr, i32 } [ %94, %93 ], [ %143, %145 ], [ %143, %142 ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %44) #89
          to label %1215 unwind label %159

97:                                               ; preds = %148, %83
  %98 = phi ptr [ %81, %83 ], [ %99, %148 ]
  %99 = getelementptr inbounds nuw i8, ptr %98, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %42)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %84, ptr noundef nonnull align 8 dereferenceable(40) %98, i64 40, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %43)
  store ptr %1, ptr %42, align 8
  tail call void @llvm.experimental.noalias.scope.decl(metadata !15911)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !15914)
  %100 = load i64, ptr %84, align 8, !alias.scope !15914, !noalias !15916, !noundef !1740
  %101 = icmp eq i64 %100, 0
  br i1 %101, label %102, label %104

102:                                              ; preds = %97
  %103 = load ptr, ptr %86, align 8, !alias.scope !15911, !noalias !15918, !nonnull !1740, !align !1836, !noundef !1740
; invoke purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(96) %43, ptr noalias nofree noundef align 8 dereferenceable(184) %87, ptr noundef nonnull align 8 %103, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %88)
          to label %124 unwind label %93

104:                                              ; preds = %97
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %85, ptr noundef nonnull align 8 dereferenceable(40) %98, i64 40, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %42)
  br label %135

.loopexit129:                                     ; preds = %148, %77
  %105 = phi ptr [ %81, %77 ], [ %99, %148 ]
  store ptr %105, ptr %80, align 8
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %44)
          to label %110 unwind label %106

106:                                              ; preds = %1119, %327, %127, %.loopexit129
  %107 = phi i8 [ %1039, %1119 ], [ 0, %327 ], [ 1, %127 ], [ 1, %.loopexit129 ]
  %108 = phi i8 [ 0, %1119 ], [ 0, %327 ], [ 1, %127 ], [ 1, %.loopexit129 ]
  %109 = landingpad { ptr, i32 }
          cleanup
  br i1 %73, label %113, label %1211

110:                                              ; preds = %.loopexit129
  call void @llvm.lifetime.end.p0(ptr nonnull %44)
  %111 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %111, ptr noundef nonnull align 8 dereferenceable(24) %49, i64 24, i1 false)
  %112 = getelementptr inbounds nuw i8, ptr %0, i64 32
  store i8 2, ptr %112, align 16
  store i64 -1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %45)
  call void @llvm.lifetime.end.p0(ptr nonnull %49)
  br label %1224

113:                                              ; preds = %1211, %118, %106
  %114 = phi i8 [ %119, %118 ], [ %1214, %1211 ], [ %107, %106 ]
  %115 = phi i8 [ %120, %118 ], [ %1213, %1211 ], [ %108, %106 ]
  %116 = phi { ptr, i32 } [ %121, %118 ], [ %1212, %1211 ], [ %109, %106 ]
  %117 = trunc nuw i8 %114 to i1
  br i1 %117, label %1215, label %1218

118:                                              ; preds = %1123, %345
  %119 = phi i8 [ %157, %1123 ], [ 0, %345 ]
  %120 = phi i8 [ %158, %1123 ], [ 0, %345 ]
  %121 = landingpad { ptr, i32 }
          cleanup
  br label %113

122:                                              ; preds = %__rustc::__rust_dealloc (.exit120), %.loopexit, %1121
  call void @llvm.lifetime.end.p0(ptr nonnull %49)
  %123 = trunc nuw i8 %158 to i1
  br i1 %123, label %1224, label %347

124:                                              ; preds = %102
  %125 = load i64, ptr %43, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %42)
  %126 = icmp eq i64 %125, -1
  br i1 %126, label %135, label %127

127:                                              ; preds = %124
  store ptr %99, ptr %80, align 8
  %128 = load i64, ptr %85, align 8
  %129 = load ptr, ptr %89, align 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %41, ptr noundef nonnull align 8 dereferenceable(24) %90, i64 24, i1 false)
  %130 = getelementptr inbounds nuw i8, ptr %43, i64 48
  %131 = getelementptr inbounds nuw i8, ptr %0, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %131, ptr noundef nonnull align 16 dereferenceable(48) %130, i64 48, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %43)
  %132 = getelementptr inbounds nuw i8, ptr %0, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %132, ptr noundef nonnull align 8 dereferenceable(24) %41, i64 24, i1 false)
  store i64 %125, ptr %0, align 16
  %133 = getelementptr inbounds nuw i8, ptr %0, i64 8
  store i64 %128, ptr %133, align 8
  %134 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store ptr %129, ptr %134, align 16
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %44)
          to label %155 unwind label %106

135:                                              ; preds = %124, %104
  %136 = load i64, ptr %85, align 8
  %137 = load ptr, ptr %89, align 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %41, ptr noundef nonnull align 8 dereferenceable(24) %90, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %43)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !15919)
  %138 = load i64, ptr %91, align 8, !alias.scope !15919, !noalias !15922, !noundef !1740
  %139 = load i64, ptr %49, align 8, !range !1835, !alias.scope !15919, !noalias !15922, !noundef !1740
  %140 = icmp eq i64 %138, %139
  br i1 %140, label %141, label %148

141:                                              ; preds = %135
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %49)
          to label %148 unwind label %142, !noalias !15922

142:                                              ; preds = %141
  %143 = landingpad { ptr, i32 }
          cleanup
  store ptr %99, ptr %80, align 8
  %144 = icmp ugt i64 %136, 5
  br i1 %144, label %145, label %95

145:                                              ; preds = %142
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %137) ]
  %146 = shl i64 %136, 3
  %147 = add i64 %146, -8
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %137, i64 noundef %147, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !15924
  br label %95

148:                                              ; preds = %141, %135
  %149 = load ptr, ptr %92, align 8, !alias.scope !15919, !noalias !15922, !nonnull !1740, !noundef !1740
  %150 = getelementptr inbounds nuw [40 x i8], ptr %149, i64 %138
  store i64 %136, ptr %150, align 8, !noalias !15919
  %151 = getelementptr inbounds nuw i8, ptr %150, i64 8
  store ptr %137, ptr %151, align 8, !noalias !15919
  %152 = getelementptr inbounds nuw i8, ptr %150, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %152, ptr noundef nonnull align 8 dereferenceable(24) %41, i64 24, i1 false)
  %153 = add i64 %138, 1
  store i64 %153, ptr %91, align 8, !alias.scope !15919, !noalias !15922
  %154 = icmp eq ptr %99, %79
  br i1 %154, label %.loopexit129, label %97

155:                                              ; preds = %127
  call void @llvm.lifetime.end.p0(ptr nonnull %44)
  br label %156

156:                                              ; preds = %1120, %155
  %157 = phi i8 [ %1039, %1120 ], [ 1, %155 ]
  %158 = phi i8 [ 0, %1120 ], [ 1, %155 ]
  br i1 %73, label %1121, label %1123

159:                                              ; preds = %1229, %1225, %1211, %427, %288, %273, %95
  %160 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90
  unreachable

161:                                              ; preds = %74
  %162 = load ptr, ptr %71, align 8, !nonnull !1740, !noundef !1740
  call void @llvm.lifetime.start.p0(ptr nonnull %40)
  store ptr %162, ptr %40, align 8
  %163 = getelementptr inbounds nuw i8, ptr %162, i64 16
  %164 = load i64, ptr %163, align 8
  %165 = icmp eq i64 %164, -1
  %166 = getelementptr inbounds nuw i8, ptr %162, i64 40
  %167 = load i64, ptr %166, align 8
  %168 = icmp ne i64 %167, -1
  %169 = getelementptr inbounds nuw i8, ptr %3, i64 8
  %170 = load ptr, ptr %169, align 8
  %171 = getelementptr inbounds nuw i8, ptr %3, i64 16
  %172 = load i64, ptr %171, align 8
  br i1 %168, label %246, label %174

173:                                              ; preds = %74
  tail call void @llvm.trap()
  unreachable

174:                                              ; preds = %285, %281, %246, %161
  %175 = phi i64 [ %172, %281 ], [ 0, %246 ], [ %172, %161 ], [ %172, %285 ]
  %176 = load i64, ptr %3, align 8, !range !1835, !noundef !1740
  %177 = icmp ult i64 %175, 57646075230342349
  tail call void @llvm.assume(i1 %177)
  %178 = mul nuw nsw i64 %175, 160
  %179 = getelementptr inbounds nuw i8, ptr %170, i64 %178
  call void @llvm.lifetime.start.p0(ptr nonnull %39)
  store ptr %170, ptr %39, align 8
  %180 = getelementptr inbounds nuw i8, ptr %39, i64 8
  store ptr %170, ptr %180, align 8
  %181 = getelementptr inbounds nuw i8, ptr %39, i64 16
  store i64 %176, ptr %181, align 8
  %182 = getelementptr inbounds nuw i8, ptr %39, i64 24
  store ptr %179, ptr %182, align 8
  %183 = icmp eq i64 %175, 0
  br i1 %183, label %.loopexit155, label %184

184:                                              ; preds = %174
  %185 = getelementptr inbounds nuw i8, ptr %38, i64 8
  %186 = getelementptr inbounds nuw i8, ptr %38, i64 24
  %187 = getelementptr inbounds nuw i8, ptr %38, i64 32
  %188 = getelementptr inbounds nuw i8, ptr %38, i64 40
  %189 = getelementptr inbounds nuw i8, ptr %38, i64 16
  %190 = getelementptr inbounds nuw i8, ptr %37, i64 16
  %191 = getelementptr inbounds nuw i8, ptr %37, i64 8
  %192 = getelementptr inbounds nuw i8, ptr %37, i64 24
  %193 = getelementptr inbounds nuw i8, ptr %38, i64 96
  %194 = getelementptr inbounds nuw i8, ptr %38, i64 48
  %195 = getelementptr inbounds nuw i8, ptr %38, i64 56
  %196 = getelementptr inbounds nuw i8, ptr %38, i64 64
  %197 = getelementptr inbounds nuw i8, ptr %162, i64 80
  %198 = getelementptr inbounds nuw i8, ptr %23, i64 1
  %199 = getelementptr inbounds nuw i8, ptr %23, i64 8
  %200 = getelementptr inbounds nuw i8, ptr %23, i64 16
  %201 = getelementptr inbounds nuw i8, ptr %1, i64 632
  %202 = getelementptr inbounds nuw i8, ptr %1, i64 1228
  %203 = zext nneg i8 %2 to i64
  %204 = getelementptr inbounds nuw i8, ptr %162, i64 296
  %205 = getelementptr inbounds nuw i8, ptr %162, i64 272
  %206 = getelementptr inbounds nuw i8, ptr %1, i64 1048
  %207 = getelementptr inbounds nuw i8, ptr %1, i64 1056
  %208 = getelementptr inbounds nuw i8, ptr %162, i64 104
  %209 = getelementptr inbounds nuw i8, ptr %17, i64 1
  %210 = getelementptr inbounds nuw i8, ptr %17, i64 8
  %211 = getelementptr inbounds nuw i8, ptr %17, i64 16
  %212 = getelementptr inbounds nuw i8, ptr %22, i64 8
  %213 = getelementptr inbounds nuw i8, ptr %1, i64 888
  %214 = getelementptr inbounds nuw i8, ptr %11, i64 1
  %215 = getelementptr inbounds nuw i8, ptr %11, i64 8
  %216 = getelementptr inbounds nuw i8, ptr %11, i64 16
  %217 = getelementptr inbounds nuw i8, ptr %18, i64 8
  %218 = getelementptr inbounds nuw i8, ptr %20, i64 1
  %219 = getelementptr inbounds nuw i8, ptr %20, i64 8
  %220 = getelementptr inbounds nuw i8, ptr %20, i64 16
  %221 = getelementptr inbounds nuw i8, ptr %19, i64 8
  %222 = getelementptr inbounds nuw i8, ptr %16, i64 8
  %223 = getelementptr inbounds nuw i8, ptr %15, i64 8
  %224 = getelementptr inbounds nuw i8, ptr %13, i64 1
  %225 = getelementptr inbounds nuw i8, ptr %13, i64 8
  %226 = getelementptr inbounds nuw i8, ptr %13, i64 16
  %227 = getelementptr inbounds nuw i8, ptr %12, i64 8
  %228 = getelementptr inbounds nuw i8, ptr %29, i64 8
  %229 = getelementptr inbounds nuw i8, ptr %30, i64 8
  %230 = getelementptr inbounds nuw i8, ptr %1, i64 664
  %231 = getelementptr inbounds nuw i8, ptr %29, i64 16
  %232 = getelementptr inbounds nuw i8, ptr %30, i64 16
  %233 = getelementptr inbounds nuw i8, ptr %30, i64 24
  %234 = getelementptr inbounds nuw i8, ptr %49, i64 16
  %235 = getelementptr inbounds nuw i8, ptr %49, i64 8
  %236 = getelementptr inbounds nuw i8, ptr %38, i64 88
  %237 = getelementptr inbounds nuw i8, ptr %38, i64 120
  %238 = getelementptr inbounds nuw i8, ptr %8, i64 1
  %239 = getelementptr inbounds nuw i8, ptr %8, i64 8
  %240 = getelementptr inbounds nuw i8, ptr %8, i64 16
  %241 = getelementptr inbounds nuw i8, ptr %7, i64 1
  %242 = getelementptr inbounds nuw i8, ptr %7, i64 8
  %243 = getelementptr inbounds nuw i8, ptr %7, i64 16
  %244 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %245 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %289

246:                                              ; preds = %161
  %247 = icmp eq i64 %172, 0
  br i1 %247, label %174, label %iter.check

iter.check:                                       ; preds = %246
  %min.iters.check = icmp ult i64 %172, 8
  br i1 %min.iters.check, label %.preheader156.preheader, label %vector.main.loop.iter.check

vector.main.loop.iter.check:                      ; preds = %iter.check
  %min.iters.check259 = icmp ult i64 %172, 32
  br i1 %min.iters.check259, label %vec.epilog.ph, label %vector.ph

vector.ph:                                        ; preds = %vector.main.loop.iter.check
  %n.mod.vf = and i64 %172, 24
  %n.vec = and i64 %172, -32
  br label %vector.body

vector.body:                                      ; preds = %vector.body, %vector.ph
  %index = phi i64 [ 0, %vector.ph ], [ %index.next, %vector.body ]
  %vec.ind = phi <8 x i64> [ <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>, %vector.ph ], [ %vec.ind.next, %vector.body ]
  %vec.phi = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %248, %vector.body ]
  %vec.phi260 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %249, %vector.body ]
  %vec.phi261 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %250, %vector.body ]
  %vec.phi262 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %251, %vector.body ]
  %step.add = add nuw <8 x i64> %vec.ind, splat (i64 8)
  %step.add.2 = add nuw <8 x i64> %vec.ind, splat (i64 16)
  %step.add.3 = add nuw <8 x i64> %vec.ind, splat (i64 24)
  %wide.gep = getelementptr inbounds nuw [160 x i8], ptr %170, <8 x i64> %vec.ind
  %wide.gep263 = getelementptr inbounds nuw [160 x i8], ptr %170, <8 x i64> %step.add
  %wide.gep264 = getelementptr inbounds nuw [160 x i8], ptr %170, <8 x i64> %step.add.2
  %wide.gep265 = getelementptr inbounds nuw [160 x i8], ptr %170, <8 x i64> %step.add.3
  %wide.gep266 = getelementptr i8, <8 x ptr> %wide.gep, i64 16
  %wide.gep267 = getelementptr i8, <8 x ptr> %wide.gep263, i64 16
  %wide.gep268 = getelementptr i8, <8 x ptr> %wide.gep264, i64 16
  %wide.gep269 = getelementptr i8, <8 x ptr> %wide.gep265, i64 16
  %wide.masked.gather = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep266, <8 x i1> splat (i1 true), <8 x i64> poison)
  %wide.masked.gather270 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep267, <8 x i1> splat (i1 true), <8 x i64> poison)
  %wide.masked.gather271 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep268, <8 x i1> splat (i1 true), <8 x i64> poison)
  %wide.masked.gather272 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep269, <8 x i1> splat (i1 true), <8 x i64> poison)
  %248 = add <8 x i64> %wide.masked.gather, %vec.phi
  %249 = add <8 x i64> %wide.masked.gather270, %vec.phi260
  %250 = add <8 x i64> %wide.masked.gather271, %vec.phi261
  %251 = add <8 x i64> %wide.masked.gather272, %vec.phi262
  %index.next = add nuw i64 %index, 32
  %vec.ind.next = add nuw <8 x i64> %vec.ind, splat (i64 32)
  %252 = icmp eq i64 %index.next, %n.vec
  br i1 %252, label %middle.block, label %vector.body, !llvm.loop !15927

middle.block:                                     ; preds = %vector.body
  %bin.rdx = add <8 x i64> %249, %248
  %bin.rdx273 = add <8 x i64> %250, %bin.rdx
  %bin.rdx274 = add <8 x i64> %251, %bin.rdx273
  %253 = tail call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %bin.rdx274)
  %cmp.n = icmp eq i64 %172, %n.vec
  br i1 %cmp.n, label %.loopexit287, label %vec.epilog.iter.check

vec.epilog.iter.check:                            ; preds = %middle.block
  %min.epilog.iters.check = icmp eq i64 %n.mod.vf, 0
  br i1 %min.epilog.iters.check, label %.preheader156.preheader, label %vec.epilog.ph, !prof !11074

vec.epilog.ph:                                    ; preds = %vector.main.loop.iter.check, %vec.epilog.iter.check
  %vec.epilog.resume.val = phi i64 [ %n.vec, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %bc.merge.rdx = phi i64 [ %253, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %n.vec276 = and i64 %172, -8
  %254 = insertelement <8 x i64> <i64 poison, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0>, i64 %bc.merge.rdx, i64 0
  %broadcast.splatinsert = insertelement <8 x i64> poison, i64 %vec.epilog.resume.val, i64 0
  %broadcast.splat = shufflevector <8 x i64> %broadcast.splatinsert, <8 x i64> poison, <8 x i32> zeroinitializer
  %induction = or disjoint <8 x i64> %broadcast.splat, <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>
  br label %vec.epilog.vector.body

vec.epilog.vector.body:                           ; preds = %vec.epilog.vector.body, %vec.epilog.ph
  %index277 = phi i64 [ %vec.epilog.resume.val, %vec.epilog.ph ], [ %index.next283, %vec.epilog.vector.body ]
  %vec.ind278 = phi <8 x i64> [ %induction, %vec.epilog.ph ], [ %vec.ind.next284, %vec.epilog.vector.body ]
  %vec.phi279 = phi <8 x i64> [ %254, %vec.epilog.ph ], [ %255, %vec.epilog.vector.body ]
  %wide.gep280 = getelementptr inbounds nuw [160 x i8], ptr %170, <8 x i64> %vec.ind278
  %wide.gep281 = getelementptr i8, <8 x ptr> %wide.gep280, i64 16
  %wide.masked.gather282 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep281, <8 x i1> splat (i1 true), <8 x i64> poison)
  %255 = add <8 x i64> %wide.masked.gather282, %vec.phi279
  %index.next283 = add nuw i64 %index277, 8
  %vec.ind.next284 = add nuw <8 x i64> %vec.ind278, splat (i64 8)
  %256 = icmp eq i64 %index.next283, %n.vec276
  br i1 %256, label %vec.epilog.middle.block, label %vec.epilog.vector.body, !llvm.loop !15928

vec.epilog.middle.block:                          ; preds = %vec.epilog.vector.body
  %257 = tail call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %255)
  %cmp.n285 = icmp eq i64 %172, %n.vec276
  br i1 %cmp.n285, label %.loopexit287, label %.preheader156.preheader

.preheader156.preheader:                          ; preds = %iter.check, %vec.epilog.iter.check, %vec.epilog.middle.block
  %.ph549 = phi i64 [ 0, %iter.check ], [ %n.vec, %vec.epilog.iter.check ], [ %n.vec276, %vec.epilog.middle.block ]
  %.ph550 = phi i64 [ 0, %iter.check ], [ %253, %vec.epilog.iter.check ], [ %257, %vec.epilog.middle.block ]
  br label %.preheader156

.preheader156:                                    ; preds = %.preheader156.preheader, %.preheader156
  %258 = phi i64 [ %265, %.preheader156 ], [ %.ph549, %.preheader156.preheader ]
  %259 = phi i64 [ %264, %.preheader156 ], [ %.ph550, %.preheader156.preheader ]
  %260 = getelementptr inbounds nuw [160 x i8], ptr %170, i64 %258
  %261 = getelementptr i8, ptr %260, i64 16
  %262 = load i64, ptr %261, align 8, !noundef !1740
  %263 = icmp ult i64 %262, 104811045873349726
  tail call void @llvm.assume(i1 %263)
  %264 = add i64 %262, %259
  %265 = add nuw i64 %258, 1
  %266 = icmp eq i64 %265, %172
  br i1 %266, label %.loopexit287, label %.preheader156, !llvm.loop !15929

267:                                              ; preds = %.loopexit130, %.loopexit.split-lp, %288
  %268 = phi i8 [ %430, %288 ], [ 1, %.loopexit130 ], [ %.ph, %.loopexit.split-lp ]
  %269 = phi i8 [ 0, %288 ], [ 0, %.loopexit130 ], [ %.ph131, %.loopexit.split-lp ]
  %270 = phi { ptr, i32 } [ %431, %288 ], [ %lpad.loopexit, %.loopexit130 ], [ %lpad.loopexit.split-lp, %.loopexit.split-lp ]
  %271 = atomicrmw sub ptr %162, i64 1 release, align 8, !noalias !15930
  %272 = icmp eq i64 %271, 1
  br i1 %272, label %273, label %1211

273:                                              ; preds = %267
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %40) #91
          to label %1211 unwind label %159

.loopexit130:                                     ; preds = %344
  %lpad.loopexit = landingpad { ptr, i32 }
          cleanup
  br label %267

.loopexit.split-lp:                               ; preds = %280, %285, %.loopexit155, %1115
  %.ph = phi i8 [ 1, %285 ], [ 1, %.loopexit155 ], [ 1, %280 ], [ %1039, %1115 ]
  %.ph131 = phi i8 [ 1, %285 ], [ 0, %.loopexit155 ], [ 1, %280 ], [ 0, %1115 ]
  %lpad.loopexit.split-lp = landingpad { ptr, i32 }
          cleanup
  br label %267

.loopexit287:                                     ; preds = %.preheader156, %vec.epilog.middle.block, %middle.block
  %.lcssa258 = phi i64 [ %257, %vec.epilog.middle.block ], [ %253, %middle.block ], [ %264, %.preheader156 ]
  %274 = getelementptr inbounds nuw i8, ptr %1, i64 912
  %275 = getelementptr inbounds nuw i8, ptr %1, i64 928
  %276 = load i64, ptr %275, align 16, !alias.scope !15935, !noundef !1740
  %277 = load i64, ptr %274, align 16, !range !1835, !alias.scope !15935, !noundef !1740
  %278 = sub i64 %277, %276
  %279 = icmp ugt i64 %.lcssa258, %278
  br i1 %279, label %280, label %281, !prof !3851

280:                                              ; preds = %.loopexit287
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.12908414067662811932)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %274, i64 noundef %276, i64 noundef %.lcssa258, i64 noundef 8, i64 noundef 80)
          to label %281 unwind label %.loopexit.split-lp

281:                                              ; preds = %280, %.loopexit287
  %282 = getelementptr inbounds nuw i8, ptr %1, i64 1016
  %283 = load i64, ptr %282, align 8, !alias.scope !15940, !noundef !1740
  %284 = icmp ugt i64 %.lcssa258, %283
  br i1 %284, label %285, label %174, !prof !3851

285:                                              ; preds = %281
  %286 = getelementptr inbounds nuw i8, ptr %1, i64 1000
; invoke <hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>
  %287 = invoke { i64, i64 } @<hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>(ptr noalias nofree noundef nonnull align 8 dereferenceable(32) %286, i64 noundef %.lcssa258, ptr noundef nonnull align 8 %274, i1 noundef zeroext true) #91
          to label %174 unwind label %.loopexit.split-lp

288:                                              ; preds = %1210, %1207, %1204
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %39) #89
          to label %267 unwind label %159

289:                                              ; preds = %490, %184
  %290 = phi ptr [ %170, %184 ], [ %291, %490 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !15943)
  %291 = getelementptr inbounds nuw i8, ptr %290, i64 160
  store ptr %291, ptr %180, align 8, !alias.scope !15943, !noalias !15946
  %292 = load i64, ptr %290, align 8, !noalias !15943
  %293 = icmp eq i64 %292, -1
  br i1 %293, label %.loopexit155, label %294

294:                                              ; preds = %289
  %295 = getelementptr inbounds nuw i8, ptr %290, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %38)
  store i64 %292, ptr %38, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(152) %185, ptr noundef nonnull align 8 dereferenceable(152) %295, i64 152, i1 false)
  %296 = load i64, ptr %186, align 8
  %297 = load ptr, ptr %187, align 8
  %298 = load i64, ptr %188, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %37)
  %299 = load ptr, ptr %185, align 8, !nonnull !1740, !noundef !1740
  %300 = load i64, ptr %189, align 8, !noundef !1740
  %301 = icmp ult i64 %300, 104811045873349726
  call void @llvm.assume(i1 %301)
  %302 = getelementptr inbounds nuw [88 x i8], ptr %299, i64 %300
  store ptr %299, ptr %37, align 8
  store i64 %292, ptr %190, align 8
  store ptr %299, ptr %191, align 8
  store ptr %302, ptr %192, align 8
  %303 = load ptr, ptr %195, align 8, !nonnull !1740, !noundef !1740
  %304 = load i64, ptr %194, align 8, !range !1835, !noundef !1740
  %305 = load i64, ptr %196, align 8, !noundef !1740
  %306 = icmp ult i64 %305, 288230376151711744
  call void @llvm.assume(i1 %306)
  %307 = shl nuw nsw i64 %305, 5
  %308 = getelementptr inbounds nuw i8, ptr %303, i64 %307
  %309 = icmp eq i64 %305, 0
  br i1 %309, label %.loopexit154, label %310

310:                                              ; preds = %294
  %311 = load i64, ptr %193, align 8, !noundef !1740
  %312 = icmp ult i64 %298, 384307168202282326
  %313 = ptrtoint ptr %302 to i64
  %314 = load ptr, ptr %69, align 8
  br label %381

.loopexit155:                                     ; preds = %490, %289, %174
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %39)
          to label %315 unwind label %.loopexit.split-lp

315:                                              ; preds = %.loopexit155
  call void @llvm.lifetime.end.p0(ptr nonnull %39)
  %316 = getelementptr inbounds nuw i8, ptr %3, i64 24
  %317 = load i8, ptr %316, align 8, !range !1747, !noundef !1740
  %318 = trunc nuw i8 %317 to i1
  %319 = trunc nuw i64 %5 to i1
  %320 = xor i1 %319, true
  %321 = or i1 %320, %318
  %322 = select i1 %321, i1 true, i1 %165
  br i1 %322, label %<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit), label %328

<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit.sink.split): ; preds = %.noexc, %340
  call void @llvm.lifetime.end.p0(ptr nonnull %10), !noalias !15948
  br label %<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit)

<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit): ; preds = %<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit.sink.split), %315
  %323 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %323, ptr noundef nonnull align 8 dereferenceable(24) %49, i64 24, i1 false)
  %324 = getelementptr inbounds nuw i8, ptr %0, i64 32
  store i8 2, ptr %324, align 16
  store i64 -1, ptr %0, align 16
  %325 = atomicrmw sub ptr %162, i64 1 release, align 8, !noalias !15955
  %326 = icmp eq i64 %325, 1
  br i1 %326, label %327, label %345

327:                                              ; preds = %<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit)
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %40) #91
          to label %345 unwind label %106

328:                                              ; preds = %315
  %329 = getelementptr inbounds nuw i8, ptr %162, i64 80
  %330 = getelementptr inbounds nuw i8, ptr %9, i64 1
  %331 = getelementptr inbounds nuw i8, ptr %9, i64 8
  %332 = getelementptr inbounds nuw i8, ptr %9, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %10), !noalias !15948
  %333 = load atomic i64, ptr %329 monotonic, align 8, !noalias !15960
  br label %334

334:                                              ; preds = %334, %328
  %335 = phi i64 [ %333, %328 ], [ %339, %334 ]
  %336 = call i64 @llvm.uadd.sat.i64(i64 %335, i64 1)
  %337 = cmpxchg weak ptr %329, i64 %335, i64 %336 monotonic monotonic, align 8, !noalias !15960
  %338 = extractvalue { i64, i1 } %337, 1
  %339 = extractvalue { i64, i1 } %337, 0
  br i1 %338, label %340, label %334

340:                                              ; preds = %334
  %341 = call i64 @llvm.uadd.sat.i64(i64 %339, i64 1)
  %342 = load i64, ptr %163, align 8, !noalias !15960
  %343 = icmp ugt i64 %341, %342
  br i1 %343, label %344, label %<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit.sink.split)

344:                                              ; preds = %340
  call void @llvm.lifetime.start.p0(ptr nonnull %9), !noalias !15960
  store i8 0, ptr %330, align 1, !noalias !15960
  store i64 %342, ptr %331, align 8, !noalias !15960
  store i64 %341, ptr %332, align 8, !noalias !15960
  store i8 0, ptr %9, align 8, !noalias !15960
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %10, ptr noundef nonnull align 8 %163, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %9)
          to label %.noexc unwind label %.loopexit130

.noexc:                                           ; preds = %344
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !15960
  br label %<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit.sink.split)

345:                                              ; preds = %327, %<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit)
  call void @llvm.lifetime.end.p0(ptr nonnull %40)
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %45)
          to label %346 unwind label %118

346:                                              ; preds = %345
  call void @llvm.lifetime.end.p0(ptr nonnull %45)
  call void @llvm.lifetime.end.p0(ptr nonnull %49)
  br label %347

347:                                              ; preds = %1224, %346, %122
  ret void

348:                                              ; preds = %697
  %349 = landingpad { ptr, i32 }
          cleanup
  store ptr %693, ptr %191, align 8
  br label %376

350:                                              ; preds = %675
  %351 = landingpad { ptr, i32 }
          cleanup
  br label %376

352:                                              ; preds = %761
  %353 = landingpad { ptr, i32 }
          cleanup
  store ptr %757, ptr %191, align 8
  br label %376

354:                                              ; preds = %657
  %355 = landingpad { ptr, i32 }
          cleanup
  store ptr %653, ptr %191, align 8
  br label %376

356:                                              ; preds = %1004
  %357 = landingpad { ptr, i32 }
          cleanup
  store ptr %1001, ptr %69, align 8
  br label %376

358:                                              ; preds = %985
  %359 = landingpad { ptr, i32 }
          cleanup
  store ptr %981, ptr %191, align 8
  br label %376

360:                                              ; preds = %916
  %361 = landingpad { ptr, i32 }
          cleanup
  br label %376

362:                                              ; preds = %889
  %363 = landingpad { ptr, i32 }
          cleanup
  store ptr %885, ptr %191, align 8
  br label %376

364:                                              ; preds = %831
  %365 = landingpad { ptr, i32 }
          cleanup
  store ptr %827, ptr %191, align 8
  br label %376

366:                                              ; preds = %793, %777, %745
  %367 = landingpad { ptr, i32 }
          cleanup
  br label %376

368:                                              ; preds = %.preheader150
  %369 = landingpad { ptr, i32 }
          cleanup
  br label %376

370:                                              ; preds = %511
  %371 = landingpad { ptr, i32 }
          cleanup
  br label %376

372:                                              ; preds = %965, %947, %952
  %373 = landingpad { ptr, i32 }
          cleanup
  br label %376

374:                                              ; preds = %538, %494
  %375 = landingpad { ptr, i32 }
          cleanup
  br label %376

376:                                              ; preds = %1028, %1025, %374, %372, %370, %368, %366, %364, %362, %360, %358, %356, %354, %352, %350, %348
  %377 = phi { ptr, i32 } [ %1026, %1025 ], [ %1026, %1028 ], [ %349, %348 ], [ %351, %350 ], [ %353, %352 ], [ %355, %354 ], [ %357, %356 ], [ %359, %358 ], [ %361, %360 ], [ %363, %362 ], [ %365, %364 ], [ %367, %366 ], [ %369, %368 ], [ %371, %370 ], [ %373, %372 ], [ %375, %374 ]
  %378 = icmp eq i64 %304, 0
  br i1 %378, label %427, label %379

379:                                              ; preds = %376
  %380 = shl nuw i64 %304, 5
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %303, i64 noundef %380, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !15963
  br label %427

381:                                              ; preds = %.loopexit136, %310
  %382 = phi ptr [ %314, %310 ], [ %998, %.loopexit136 ]
  %383 = phi ptr [ %299, %310 ], [ %811, %.loopexit136 ]
  %384 = phi i64 [ 0, %310 ], [ %536, %.loopexit136 ]
  %385 = phi ptr [ %303, %310 ], [ %388, %.loopexit136 ]
  %386 = phi ptr [ %299, %310 ], [ %813, %.loopexit136 ]
  %387 = phi i64 [ %311, %310 ], [ %812, %.loopexit136 ]
  %388 = getelementptr inbounds nuw i8, ptr %385, i64 32
  %389 = load i64, ptr %385, align 8, !noalias !15966
  %390 = getelementptr inbounds nuw i8, ptr %385, i64 8
  %391 = load i64, ptr %390, align 8, !noalias !15966
  %392 = getelementptr inbounds nuw i8, ptr %385, i64 16
  %393 = load i64, ptr %392, align 8, !noalias !15966
  %394 = getelementptr inbounds nuw i8, ptr %385, i64 24
  %395 = load i64, ptr %394, align 8, !noalias !15966
  %396 = icmp eq i64 %389, 0
  %397 = select i1 %396, i1 true, i1 %165
  br i1 %397, label %492, label %499

.loopexit154:                                     ; preds = %.loopexit136, %294
  %398 = icmp eq i64 %304, 0
  br i1 %398, label %428, label %399

399:                                              ; preds = %.loopexit154
  %400 = shl nuw i64 %304, 5
  %401 = load i64, ptr %244, align 8, !noalias !15969, !noundef !1740
  %402 = call i64 @llvm.umin.i64(i64 %400, i64 9223372036854775807)
  %403 = call i64 @llvm.ssub.sat.i64(i64 %401, i64 %402)
  store i64 %403, ptr %244, align 8, !noalias !15969
  %404 = load i64, ptr %245, align 8, !noalias !15969, !noundef !1740
  %405 = icmp slt i64 %403, %404
  br i1 %405, label %406, label %.preheader312

406:                                              ; preds = %399
  store i64 %403, ptr %245, align 8, !noalias !15969
  br label %.preheader312

.preheader312:                                    ; preds = %406, %399
  br label %407

407:                                              ; preds = %.preheader312, %410
  %408 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !15969
  %409 = icmp slt i64 %408, 0
  br i1 %409, label %410, label %__rustc::__rust_dealloc (.exit)

410:                                              ; preds = %407
  %411 = add nsw i64 %408, 1
  %412 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %408, i64 %411 acq_rel acquire, align 8, !noalias !15969
  %413 = extractvalue { i64, i1 } %412, 1
  br i1 %413, label %414, label %407

414:                                              ; preds = %410
  %415 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %402 monotonic, align 8, !noalias !15969
  %416 = call i64 @llvm.ssub.sat.i64(i64 %415, i64 %402)
  %417 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !15969
  br label %418

418:                                              ; preds = %421, %414
  %419 = phi i64 [ %417, %414 ], [ %424, %421 ]
  %420 = icmp slt i64 %416, %419
  br i1 %420, label %421, label %425

421:                                              ; preds = %418
  %422 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %419, i64 %416 monotonic monotonic, align 8, !noalias !15969
  %423 = extractvalue { i64, i1 } %422, 1
  %424 = extractvalue { i64, i1 } %422, 0
  br i1 %423, label %425, label %418

425:                                              ; preds = %421, %418
  %426 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !15969
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %407, %425
  call void @free(ptr noundef nonnull %303) #92, !noalias !15969
  br label %428

427:                                              ; preds = %379, %376
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %37) #89
          to label %429 unwind label %159

428:                                              ; preds = %__rustc::__rust_dealloc (.exit), %.loopexit154
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %37)
          to label %439 unwind label %435

429:                                              ; preds = %437, %435, %427
  %430 = phi i8 [ 1, %427 ], [ 1, %435 ], [ %1039, %437 ]
  %431 = phi { ptr, i32 } [ %377, %427 ], [ %436, %435 ], [ %438, %437 ]
  %432 = icmp eq i64 %296, 0
  br i1 %432, label %469, label %433

433:                                              ; preds = %429
  %434 = mul nuw i64 %296, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %297) ]
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %297, i64 noundef %434, i64 noundef range(i64 1, -9223372036854775807) 8) #92
  br label %469

435:                                              ; preds = %428
  %436 = landingpad { ptr, i32 }
          cleanup
  br label %429

437:                                              ; preds = %1070
  %438 = landingpad { ptr, i32 }
          cleanup
  br label %429

439:                                              ; preds = %428
  call void @llvm.lifetime.end.p0(ptr nonnull %37)
  %440 = icmp eq i64 %296, 0
  br i1 %440, label %476, label %441

441:                                              ; preds = %439
  %442 = mul nuw i64 %296, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %297) ]
  %443 = load i64, ptr %244, align 8, !noundef !1740
  %444 = call i64 @llvm.umin.i64(i64 %442, i64 9223372036854775807)
  %445 = call i64 @llvm.ssub.sat.i64(i64 %443, i64 %444)
  store i64 %445, ptr %244, align 8
  %446 = load i64, ptr %245, align 8, !noundef !1740
  %447 = icmp slt i64 %445, %446
  br i1 %447, label %448, label %.preheader311

448:                                              ; preds = %441
  store i64 %445, ptr %245, align 8
  br label %.preheader311

.preheader311:                                    ; preds = %448, %441
  br label %449

449:                                              ; preds = %.preheader311, %452
  %450 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8
  %451 = icmp slt i64 %450, 0
  br i1 %451, label %452, label %__rustc::__rust_dealloc (.exit114)

452:                                              ; preds = %449
  %453 = add nsw i64 %450, 1
  %454 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %450, i64 %453 acq_rel acquire, align 8
  %455 = extractvalue { i64, i1 } %454, 1
  br i1 %455, label %456, label %449

456:                                              ; preds = %452
  %457 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %444 monotonic, align 8
  %458 = call i64 @llvm.ssub.sat.i64(i64 %457, i64 %444)
  %459 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8
  br label %460

460:                                              ; preds = %463, %456
  %461 = phi i64 [ %459, %456 ], [ %466, %463 ]
  %462 = icmp slt i64 %458, %461
  br i1 %462, label %463, label %467

463:                                              ; preds = %460
  %464 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %461, i64 %458 monotonic monotonic, align 8
  %465 = extractvalue { i64, i1 } %464, 1
  %466 = extractvalue { i64, i1 } %464, 0
  br i1 %465, label %467, label %460

467:                                              ; preds = %463, %460
  %468 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8
  br label %__rustc::__rust_dealloc (.exit114)

__rustc::__rust_dealloc (.exit114): ; preds = %449, %467
  call void @free(ptr noundef nonnull %297) #92
  br label %476

469:                                              ; preds = %433, %429
  call void @llvm.experimental.noalias.scope.decl(metadata !15972)
  %470 = load ptr, ptr %236, align 8, !alias.scope !15972, !noundef !1740
  %471 = icmp eq ptr %470, null
  br i1 %471, label %1204, label %472

472:                                              ; preds = %469
  %473 = atomicrmw sub ptr %470, i64 1 release, align 8, !noalias !15975
  %474 = icmp eq i64 %473, 1
  br i1 %474, label %475, label %1204

475:                                              ; preds = %472
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %236) #91
  br label %1204

476:                                              ; preds = %__rustc::__rust_dealloc (.exit114), %439
  call void @llvm.experimental.noalias.scope.decl(metadata !15980)
  %477 = load ptr, ptr %236, align 8, !alias.scope !15980, !noundef !1740
  %478 = icmp eq ptr %477, null
  br i1 %478, label %483, label %479

479:                                              ; preds = %476
  %480 = atomicrmw sub ptr %477, i64 1 release, align 8, !noalias !15983
  %481 = icmp eq i64 %480, 1
  br i1 %481, label %482, label %483

482:                                              ; preds = %479
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %236) #91
  br label %483

483:                                              ; preds = %482, %479, %476
  call void @llvm.experimental.noalias.scope.decl(metadata !15988)
  %484 = load ptr, ptr %237, align 8, !alias.scope !15988, !noundef !1740
  %485 = icmp eq ptr %484, null
  br i1 %485, label %490, label %486

486:                                              ; preds = %483
  %487 = atomicrmw sub ptr %484, i64 1 release, align 8, !noalias !15991
  %488 = icmp eq i64 %487, 1
  br i1 %488, label %489, label %490

489:                                              ; preds = %486
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %237) #91
  br label %490

490:                                              ; preds = %489, %486, %483
  call void @llvm.lifetime.end.p0(ptr nonnull %38)
  %491 = icmp eq ptr %291, %179
  br i1 %491, label %.loopexit155, label %289

492:                                              ; preds = %527, %521, %515, %381
  call void @llvm.assume(i1 %312)
  %493 = icmp ugt i64 %384, %298
  br i1 %493, label %494, label %533, !prof !1742

494:                                              ; preds = %492
  call void @llvm.lifetime.start.p0(ptr nonnull %27)
  store i64 %384, ptr %27, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %26)
  store i64 %298, ptr %26, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %25)
  store ptr %27, ptr %25, align 8
  %495 = getelementptr inbounds nuw i8, ptr %25, i64 8
  store ptr @<usize as core::fmt::Debug>::fmt, ptr %495, align 8
  %496 = getelementptr inbounds nuw i8, ptr %25, i64 16
  store ptr %26, ptr %496, align 8
  %497 = getelementptr inbounds nuw i8, ptr %25, i64 24
  store ptr @<usize as core::fmt::Debug>::fmt, ptr %497, align 8
; invoke core::panicking::panic_fmt
  invoke void @core::panicking::panic_fmt(ptr noundef nonnull @anon.e5162873a9a3251d11c4df37a70e4654.2158, ptr noundef nonnull %25, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.289) #88
          to label %498 unwind label %374

498:                                              ; preds = %494
  unreachable

499:                                              ; preds = %381
  call void @llvm.lifetime.start.p0(ptr nonnull %24), !noalias !15996
  %500 = load atomic i64, ptr %197 monotonic, align 8, !noalias !16003
  br label %501

501:                                              ; preds = %501, %499
  %502 = phi i64 [ %500, %499 ], [ %506, %501 ]
  %503 = call i64 @llvm.uadd.sat.i64(i64 %502, i64 %389)
  %504 = cmpxchg weak ptr %197, i64 %502, i64 %503 monotonic monotonic, align 8, !noalias !16003
  %505 = extractvalue { i64, i1 } %504, 1
  %506 = extractvalue { i64, i1 } %504, 0
  br i1 %505, label %507, label %501

507:                                              ; preds = %501
  %508 = call i64 @llvm.uadd.sat.i64(i64 %506, i64 %389)
  %509 = load i64, ptr %163, align 8, !noalias !16003
  %510 = icmp ugt i64 %508, %509
  br i1 %510, label %511, label %515

511:                                              ; preds = %507
  call void @llvm.lifetime.start.p0(ptr nonnull %23), !noalias !16003
  store i8 0, ptr %198, align 1, !noalias !16003
  store i64 %509, ptr %199, align 8, !noalias !16003
  store i64 %508, ptr %200, align 8, !noalias !16003
  store i8 0, ptr %23, align 8, !noalias !16003
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %24, ptr noundef nonnull align 8 %163, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %23)
          to label %512 unwind label %370

512:                                              ; preds = %511
  call void @llvm.lifetime.end.p0(ptr nonnull %23), !noalias !16003
  %513 = load i8, ptr %24, align 8, !noalias !15996
  %514 = icmp eq i8 %513, -1
  br i1 %514, label %515, label %518

515:                                              ; preds = %512, %507
  call void @llvm.lifetime.end.p0(ptr nonnull %24), !noalias !15996
  %516 = load ptr, ptr %201, align 8, !noundef !1740
  %517 = icmp eq ptr %516, null
  br i1 %517, label %492, label %521

518:                                              ; preds = %512
  call void @llvm.lifetime.end.p0(ptr nonnull %24), !noalias !15996
  %519 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %519, ptr noundef nonnull align 8 dereferenceable(24) %49, i64 24, i1 false)
  %520 = getelementptr inbounds nuw i8, ptr %0, i64 32
  store i8 0, ptr %520, align 16
  store i64 -1, ptr %0, align 16
  br label %1038

521:                                              ; preds = %515
  %522 = load i32, ptr %202, align 4, !noundef !1740
  %523 = getelementptr i8, ptr %516, i64 56
  %524 = load i64, ptr %523, align 8, !noundef !1740
  %525 = zext i32 %522 to i64
  %526 = icmp ugt i64 %524, %525
  br i1 %526, label %527, label %492

527:                                              ; preds = %521
  %528 = getelementptr i8, ptr %516, i64 48
  %529 = load ptr, ptr %528, align 8, !nonnull !1740, !noundef !1740
  %530 = getelementptr inbounds nuw [136 x i8], ptr %529, i64 %525
  %531 = getelementptr inbounds nuw [8 x i8], ptr %530, i64 %203
  %532 = atomicrmw add ptr %531, i64 %389 monotonic, align 8
  br label %492

533:                                              ; preds = %492
  %534 = icmp ult i64 %391, %384
  %535 = call i64 @llvm.umin.i64(i64 %391, i64 range(i64 0, 384307168202282326) %298)
  %536 = select i1 %534, i64 %384, i64 %535
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %297) ]
  %537 = icmp samesign ult i64 %536, %384
  br i1 %537, label %538, label %539, !prof !10952

538:                                              ; preds = %533
; invoke core::slice::index::slice_index_fail
  invoke void @core::slice::index::slice_index_fail(i64 noundef %384, i64 noundef %536, i64 noundef %298, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.290) #93
          to label %1069 unwind label %374

539:                                              ; preds = %533
  %540 = mul nuw nsw i64 %384, 24
  %541 = getelementptr inbounds nuw i8, ptr %297, i64 %540
  %542 = mul nuw nsw i64 %536, 24
  %543 = getelementptr inbounds nuw i8, ptr %297, i64 %542
  %544 = icmp eq i64 %384, %536
  br i1 %544, label %.loopexit153, label %545

545:                                              ; preds = %539
  %546 = sub nuw nsw i64 %542, %540
  %547 = udiv exact i64 %546, 24
  br label %548

548:                                              ; preds = %563, %545
  %549 = phi i64 [ 0, %545 ], [ %564, %563 ]
  %550 = phi i64 [ 0, %545 ], [ %565, %563 ]
  %551 = phi i64 [ 0, %545 ], [ %566, %563 ]
  %552 = phi i64 [ 0, %545 ], [ %567, %563 ]
  %553 = getelementptr inbounds nuw [24 x i8], ptr %541, i64 %552
  %554 = load i8, ptr %553, align 8, !range !11184, !noalias !16006, !noundef !1740
  %555 = getelementptr i8, ptr %553, i64 8
  %556 = load i64, ptr %555, align 8, !noalias !16006
  switch i8 %554, label %.unreachabledefault [
    i8 0, label %557
    i8 1, label %559
    i8 2, label %563
    i8 3, label %561
  ]

.unreachabledefault:                              ; preds = %548
  unreachable

default.unreachable806:                           ; preds = %.preheader140
  unreachable

557:                                              ; preds = %548
  %558 = call i64 @llvm.uadd.sat.i64(i64 %551, i64 %556)
  br label %563

559:                                              ; preds = %548
  %560 = call i64 @llvm.uadd.sat.i64(i64 %550, i64 %556)
  br label %563

561:                                              ; preds = %548
  %562 = call i64 @llvm.umax.i64(i64 %549, i64 %556)
  br label %563

563:                                              ; preds = %561, %559, %557, %548
  %564 = phi i64 [ %549, %557 ], [ %549, %559 ], [ %562, %561 ], [ %549, %548 ]
  %565 = phi i64 [ %550, %557 ], [ %560, %559 ], [ %550, %561 ], [ %550, %548 ]
  %566 = phi i64 [ %558, %557 ], [ %551, %559 ], [ %551, %561 ], [ %551, %548 ]
  %567 = add nuw i64 %552, 1
  %568 = icmp eq i64 %567, %547
  br i1 %568, label %.loopexit153, label %548

.loopexit153:                                     ; preds = %563, %539
  %569 = phi i64 [ 0, %539 ], [ %566, %563 ]
  %570 = phi i64 [ 0, %539 ], [ %565, %563 ]
  %571 = phi i64 [ 0, %539 ], [ %564, %563 ]
  br i1 %168, label %575, label %.loopexit151

.loopexit151:                                     ; preds = %587, %575, %.loopexit153
  %572 = phi i64 [ 0, %.loopexit153 ], [ 0, %575 ], [ %589, %587 ]
  %573 = load atomic i32, ptr %204 acquire, align 8, !noalias !16010
  %574 = icmp eq i32 %573, 0
  br i1 %574, label %591, label %594

575:                                              ; preds = %.loopexit153
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %386) ]
  %576 = ptrtoint ptr %386 to i64
  %577 = call i64 @llvm.usub.sat.i64(i64 %393, i64 %387)
  %578 = sub nuw i64 %313, %576
  %579 = udiv exact i64 %578, 88
  %580 = call i64 @llvm.umin.i64(i64 %577, i64 %579)
  %581 = icmp eq i64 %580, 0
  br i1 %581, label %.loopexit151, label %.preheader150

.preheader150:                                    ; preds = %575, %587
  %582 = phi i64 [ %589, %587 ], [ 0, %575 ]
  %583 = phi i64 [ %588, %587 ], [ 0, %575 ]
  %584 = getelementptr inbounds nuw [88 x i8], ptr %386, i64 %583
  %585 = getelementptr inbounds nuw i8, ptr %584, i64 8
; invoke purrdf_sparql_eval::scratch::value_bytes
  %586 = invoke noundef i64 @purrdf_sparql_eval::scratch::value_bytes(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(80) %585)
          to label %587 unwind label %368

587:                                              ; preds = %.preheader150
  %588 = add nuw nsw i64 %583, 1
  %589 = call noundef range(i64 32, 0) i64 @llvm.uadd.sat.i64(i64 %582, i64 %586)
  %590 = icmp eq i64 %588, %580
  br i1 %590, label %.loopexit151, label %.preheader150

591:                                              ; preds = %.loopexit151
  %592 = load i8, ptr %205, align 8
  %593 = icmp eq i8 %592, -1
  br i1 %593, label %594, label %601

594:                                              ; preds = %591, %.loopexit151
  br i1 %165, label %595, label %596

595:                                              ; preds = %596, %594
  br i1 %168, label %604, label %602

596:                                              ; preds = %594
  %597 = load atomic i64, ptr %197 monotonic, align 8, !noalias !16013
  %598 = load i64, ptr %163, align 8, !noalias !16013
  %599 = call i64 @llvm.uadd.sat.i64(i64 %597, i64 %569)
  %600 = icmp ugt i64 %599, %598
  br i1 %600, label %601, label %595

601:                                              ; preds = %604, %596, %591
  br i1 %544, label %.loopexit142, label %.preheader140

602:                                              ; preds = %604, %595
  %603 = or i1 %165, %544
  br i1 %603, label %.loopexit149, label %.preheader148

604:                                              ; preds = %595
  %605 = load i64, ptr %206, align 8, !noundef !1740
  %606 = load atomic i64, ptr %207 monotonic, align 16
  %607 = call noundef i64 @llvm.usub.sat.i64(i64 %605, i64 %606)
  %608 = call i64 @llvm.uadd.sat.i64(i64 %607, i64 %572)
  %609 = call i64 @llvm.uadd.sat.i64(i64 %608, i64 %570)
  %610 = call i64 @llvm.uadd.sat.i64(i64 %609, i64 %571)
  %611 = load atomic i64, ptr %208 monotonic, align 8, !noalias !16016
  %612 = load i64, ptr %166, align 8, !noalias !16016
  %613 = call i64 @llvm.uadd.sat.i64(i64 %611, i64 %610)
  %614 = icmp ugt i64 %613, %612
  br i1 %614, label %601, label %602

.preheader140:                                    ; preds = %601, %795
  %615 = phi ptr [ %796, %795 ], [ %383, %601 ]
  %616 = phi ptr [ %620, %795 ], [ %541, %601 ]
  %617 = phi ptr [ %799, %795 ], [ %386, %601 ]
  %618 = phi i64 [ %798, %795 ], [ %387, %601 ]
  %619 = phi ptr [ %797, %795 ], [ %383, %601 ]
  %620 = getelementptr inbounds nuw i8, ptr %616, i64 24
  %621 = load i8, ptr %616, align 8, !range !11184, !noundef !1740
  switch i8 %621, label %default.unreachable806 [
    i8 0, label %627
    i8 1, label %634
    i8 2, label %639
    i8 3, label %662
  ]

.loopexit142:                                     ; preds = %795, %601
  %622 = phi ptr [ %383, %601 ], [ %796, %795 ]
  %623 = phi i64 [ %387, %601 ], [ %798, %795 ]
  %624 = phi ptr [ %386, %601 ], [ %799, %795 ]
  %625 = icmp ult i64 %623, %393
  %626 = select i1 %168, i1 %625, i1 false
  br i1 %626, label %817, label %810

627:                                              ; preds = %.preheader140
  %628 = getelementptr inbounds nuw i8, ptr %616, i64 1
  %629 = load i8, ptr %628, align 1, !range !1741, !noundef !1740
  %630 = getelementptr inbounds nuw i8, ptr %616, i64 8
  %631 = load i64, ptr %630, align 8, !noundef !1740
  %632 = getelementptr inbounds nuw i8, ptr %616, i64 16
  %633 = load i64, ptr %632, align 8, !noundef !1740
  br i1 %165, label %795, label %663

634:                                              ; preds = %.preheader140
  %635 = getelementptr inbounds nuw i8, ptr %616, i64 8
  %636 = load i64, ptr %635, align 8, !noundef !1740
  %637 = getelementptr inbounds nuw i8, ptr %616, i64 16
  %638 = load i64, ptr %637, align 8, !noundef !1740
  br i1 %168, label %722, label %795

639:                                              ; preds = %.preheader140
  %640 = getelementptr inbounds nuw i8, ptr %616, i64 8
  %641 = load i64, ptr %640, align 8, !noundef !1740
  %642 = icmp ult i64 %618, %641
  br i1 %642, label %643, label %772

643:                                              ; preds = %639
  %644 = icmp eq ptr %617, %302
  br i1 %644, label %.loopexit134, label %645

645:                                              ; preds = %643
  %646 = add i64 %641, -1
  br label %650

647:                                              ; preds = %660
  %648 = add i64 %652, 1
  %649 = icmp eq ptr %653, %302
  br i1 %649, label %.loopexit134, label %650

650:                                              ; preds = %647, %645
  %651 = phi ptr [ %653, %647 ], [ %617, %645 ]
  %652 = phi i64 [ %648, %647 ], [ %618, %645 ]
  %653 = getelementptr inbounds nuw i8, ptr %651, i64 88
  %654 = getelementptr inbounds nuw i8, ptr %651, i64 8
  %655 = load i64, ptr %654, align 8, !noalias !16019
  %656 = icmp eq i64 %655, -1
  br i1 %656, label %.loopexit134, label %657

657:                                              ; preds = %650
  %658 = getelementptr inbounds nuw i8, ptr %651, i64 16
  %659 = load i64, ptr %651, align 8, !noalias !16019
  call void @llvm.lifetime.start.p0(ptr nonnull %22), !noalias !16022
  store i64 %655, ptr %22, align 8, !noalias !16022
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %212, ptr noundef nonnull align 8 dereferenceable(72) %658, i64 72, i1 false)
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %213, i64 noundef %659, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %22)
          to label %660 unwind label %354

660:                                              ; preds = %657
  call void @llvm.lifetime.end.p0(ptr nonnull %22), !noalias !16022
  %661 = icmp eq i64 %652, %646
  br i1 %661, label %.loopexit134, label %647

662:                                              ; preds = %.preheader140
  br i1 %168, label %781, label %795

663:                                              ; preds = %627
  call void @llvm.lifetime.start.p0(ptr nonnull %21), !noalias !16025
  %664 = load atomic i64, ptr %197 monotonic, align 8, !noalias !16032
  br label %665

665:                                              ; preds = %665, %663
  %666 = phi i64 [ %664, %663 ], [ %670, %665 ]
  %667 = call i64 @llvm.uadd.sat.i64(i64 %666, i64 %631)
  %668 = cmpxchg weak ptr %197, i64 %666, i64 %667 monotonic monotonic, align 8, !noalias !16032
  %669 = extractvalue { i64, i1 } %668, 1
  %670 = extractvalue { i64, i1 } %668, 0
  br i1 %669, label %671, label %665

671:                                              ; preds = %665
  %672 = call i64 @llvm.uadd.sat.i64(i64 %670, i64 %631)
  %673 = load i64, ptr %163, align 8, !noalias !16032
  %674 = icmp ugt i64 %672, %673
  br i1 %674, label %675, label %684

675:                                              ; preds = %671
  call void @llvm.lifetime.start.p0(ptr nonnull %20), !noalias !16032
  store i8 0, ptr %218, align 1, !noalias !16032
  store i64 %673, ptr %219, align 8, !noalias !16032
  store i64 %672, ptr %220, align 8, !noalias !16032
  store i8 0, ptr %20, align 8, !noalias !16032
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %21, ptr noundef nonnull align 8 %163, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %20)
          to label %676 unwind label %350

676:                                              ; preds = %675
  call void @llvm.lifetime.end.p0(ptr nonnull %20), !noalias !16032
  %677 = load i8, ptr %21, align 8, !noalias !16025
  %678 = icmp eq i8 %677, -1
  br i1 %678, label %684, label %679

679:                                              ; preds = %676
  call void @llvm.lifetime.end.p0(ptr nonnull %21), !noalias !16025
  %680 = icmp ne i64 %633, 0
  %681 = add i64 %633, -1
  %682 = icmp ult i64 %618, %681
  %683 = select i1 %680, i1 %682, i1 false
  br i1 %683, label %686, label %.loopexit141

684:                                              ; preds = %676, %671
  call void @llvm.lifetime.end.p0(ptr nonnull %21), !noalias !16025
  %685 = icmp eq i8 %629, -1
  br i1 %685, label %795, label %705

686:                                              ; preds = %679
  %687 = icmp eq ptr %617, %302
  br i1 %687, label %.loopexit132, label %688

688:                                              ; preds = %686
  %689 = add i64 %633, -2
  br label %690

690:                                              ; preds = %700, %688
  %691 = phi ptr [ %693, %700 ], [ %617, %688 ]
  %692 = phi i64 [ %702, %700 ], [ %618, %688 ]
  %693 = getelementptr inbounds nuw i8, ptr %691, i64 88
  %694 = getelementptr inbounds nuw i8, ptr %691, i64 8
  %695 = load i64, ptr %694, align 8, !noalias !16035
  %696 = icmp eq i64 %695, -1
  br i1 %696, label %.loopexit132, label %697

697:                                              ; preds = %690
  %698 = getelementptr inbounds nuw i8, ptr %691, i64 16
  %699 = load i64, ptr %691, align 8, !noalias !16035
  call void @llvm.lifetime.start.p0(ptr nonnull %19), !noalias !16038
  store i64 %695, ptr %19, align 8, !noalias !16038
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %221, ptr noundef nonnull align 8 dereferenceable(72) %698, i64 72, i1 false)
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %213, i64 noundef %699, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %19)
          to label %700 unwind label %348

700:                                              ; preds = %697
  call void @llvm.lifetime.end.p0(ptr nonnull %19), !noalias !16038
  %701 = icmp eq i64 %692, %689
  %702 = add nuw i64 %692, 1
  %703 = icmp eq ptr %693, %302
  %704 = select i1 %701, i1 true, i1 %703
  br i1 %704, label %.loopexit132, label %690

705:                                              ; preds = %684
  %706 = load ptr, ptr %201, align 8, !noundef !1740
  %707 = icmp eq ptr %706, null
  br i1 %707, label %795, label %708

708:                                              ; preds = %705
  %709 = load i32, ptr %202, align 4, !noundef !1740
  %710 = getelementptr i8, ptr %706, i64 56
  %711 = load i64, ptr %710, align 8, !noundef !1740
  %712 = zext i32 %709 to i64
  %713 = icmp ugt i64 %711, %712
  br i1 %713, label %714, label %795

714:                                              ; preds = %708
  %715 = getelementptr i8, ptr %706, i64 48
  %716 = load ptr, ptr %715, align 8, !nonnull !1740, !noundef !1740
  %717 = zext nneg i8 %629 to i64
  %718 = getelementptr inbounds nuw [136 x i8], ptr %716, i64 %712
  %719 = getelementptr inbounds nuw [8 x i8], ptr %718, i64 %717
  %720 = atomicrmw add ptr %719, i64 %631 monotonic, align 8
  br label %795

721:                                              ; preds = %725
  br i1 %726, label %.loopexit141, label %795

722:                                              ; preds = %634
  call void @llvm.lifetime.start.p0(ptr nonnull %33)
  %723 = load i64, ptr %166, align 8
  %724 = icmp eq i64 %723, -1
  br i1 %724, label %.thread128, label %729

.thread128:                                       ; preds = %740, %722
  call void @llvm.lifetime.end.p0(ptr nonnull %33)
  br label %795

725:                                              ; preds = %746, %744
  %.pr = load i8, ptr %33, align 8
  %726 = icmp ne i8 %.pr, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %33)
  %727 = icmp ne i64 %638, 0
  %728 = and i1 %727, %726
  br i1 %728, label %747, label %721

729:                                              ; preds = %722
  %730 = load atomic i32, ptr %204 acquire, align 8, !noalias !16041
  %731 = icmp eq i32 %730, 0
  br i1 %731, label %744, label %732

732:                                              ; preds = %729
  %733 = load atomic i64, ptr %208 monotonic, align 8, !noalias !16041
  br label %734

734:                                              ; preds = %734, %732
  %735 = phi i64 [ %733, %732 ], [ %739, %734 ]
  %736 = call i64 @llvm.uadd.sat.i64(i64 %735, i64 %636)
  %737 = cmpxchg weak ptr %208, i64 %735, i64 %736 monotonic monotonic, align 8, !noalias !16041
  %738 = extractvalue { i64, i1 } %737, 1
  %739 = extractvalue { i64, i1 } %737, 0
  br i1 %738, label %740, label %734

740:                                              ; preds = %734
  %741 = call i64 @llvm.uadd.sat.i64(i64 %739, i64 %636)
  %742 = load i64, ptr %166, align 8, !noalias !16041
  %743 = icmp ugt i64 %741, %742
  br i1 %743, label %745, label %.thread128

744:                                              ; preds = %729
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %33, ptr noundef nonnull align 8 dereferenceable(24) %205, i64 24, i1 false)
  br label %725

745:                                              ; preds = %740
  call void @llvm.lifetime.start.p0(ptr nonnull %11), !noalias !16041
  store i8 3, ptr %214, align 1, !noalias !16041
  store i64 %742, ptr %215, align 8, !noalias !16041
  store i64 %741, ptr %216, align 8, !noalias !16041
  store i8 0, ptr %11, align 8, !noalias !16041
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %33, ptr noundef nonnull align 8 %163, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %11)
          to label %746 unwind label %366

746:                                              ; preds = %745
  call void @llvm.lifetime.end.p0(ptr nonnull %11), !noalias !16041
  br label %725

747:                                              ; preds = %725
  %748 = add i64 %638, -1
  %749 = icmp ult i64 %618, %748
  br i1 %749, label %750, label %.loopexit141

750:                                              ; preds = %747
  %751 = icmp eq ptr %617, %302
  br i1 %751, label %.loopexit132, label %752

752:                                              ; preds = %750
  %753 = add i64 %638, -2
  br label %754

754:                                              ; preds = %764, %752
  %755 = phi ptr [ %757, %764 ], [ %617, %752 ]
  %756 = phi i64 [ %766, %764 ], [ %618, %752 ]
  %757 = getelementptr inbounds nuw i8, ptr %755, i64 88
  %758 = getelementptr inbounds nuw i8, ptr %755, i64 8
  %759 = load i64, ptr %758, align 8, !noalias !16044
  %760 = icmp eq i64 %759, -1
  br i1 %760, label %.loopexit132, label %761

761:                                              ; preds = %754
  %762 = getelementptr inbounds nuw i8, ptr %755, i64 16
  %763 = load i64, ptr %755, align 8, !noalias !16044
  call void @llvm.lifetime.start.p0(ptr nonnull %18), !noalias !16047
  store i64 %759, ptr %18, align 8, !noalias !16047
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %217, ptr noundef nonnull align 8 dereferenceable(72) %762, i64 72, i1 false)
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %213, i64 noundef %763, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %18)
          to label %764 unwind label %352

764:                                              ; preds = %761
  call void @llvm.lifetime.end.p0(ptr nonnull %18), !noalias !16047
  %765 = icmp eq i64 %756, %753
  %766 = add nuw i64 %756, 1
  %767 = icmp eq ptr %757, %302
  %768 = select i1 %765, i1 true, i1 %767
  br i1 %768, label %.loopexit132, label %754

.loopexit134:                                     ; preds = %660, %650, %647, %643
  %769 = phi ptr [ %619, %643 ], [ %653, %647 ], [ %653, %650 ], [ %653, %660 ]
  %770 = phi i64 [ %618, %643 ], [ %641, %660 ], [ %652, %650 ], [ %648, %647 ]
  %771 = phi ptr [ %617, %643 ], [ %653, %647 ], [ %653, %650 ], [ %653, %660 ]
  store ptr %769, ptr %191, align 8
  br label %772

772:                                              ; preds = %.loopexit134, %639
  %773 = phi ptr [ %615, %639 ], [ %769, %.loopexit134 ]
  %774 = phi ptr [ %619, %639 ], [ %769, %.loopexit134 ]
  %775 = phi i64 [ %618, %639 ], [ %770, %.loopexit134 ]
  %776 = phi ptr [ %617, %639 ], [ %771, %.loopexit134 ]
  br i1 %168, label %777, label %795

777:                                              ; preds = %772
  call void @llvm.lifetime.start.p0(ptr nonnull %32)
; invoke <purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
  invoke fastcc void @<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %32, ptr noundef nonnull align 16 %1)
          to label %778 unwind label %366

778:                                              ; preds = %777
  %779 = load i8, ptr %32, align 8, !range !1743, !noundef !1740
  %780 = icmp eq i8 %779, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %32)
  br i1 %780, label %795, label %.loopexit141

781:                                              ; preds = %662
  %782 = getelementptr inbounds nuw i8, ptr %616, i64 8
  %783 = load i64, ptr %782, align 8, !noundef !1740
  call void @llvm.lifetime.start.p0(ptr nonnull %31)
  %784 = load atomic i32, ptr %204 acquire, align 8, !noalias !16050
  %785 = icmp eq i32 %784, 0
  br i1 %785, label %786, label %787

786:                                              ; preds = %781
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %31, ptr noundef nonnull align 8 dereferenceable(24) %205, i64 24, i1 false)
  br label %801

787:                                              ; preds = %781
  %788 = load atomic i64, ptr %208 monotonic, align 8, !noalias !16050
  %789 = call i64 @llvm.uadd.sat.i64(i64 %788, i64 %783)
  %790 = load i64, ptr %166, align 8, !noalias !16050
  %791 = icmp ugt i64 %789, %790
  br i1 %791, label %793, label %792

792:                                              ; preds = %787
  call void @llvm.lifetime.end.p0(ptr nonnull %31)
  br label %795

793:                                              ; preds = %787
  call void @llvm.lifetime.start.p0(ptr nonnull %17), !noalias !16050
  store i8 3, ptr %209, align 1, !noalias !16050
  store i64 %790, ptr %210, align 8, !noalias !16050
  store i64 %789, ptr %211, align 8, !noalias !16050
  store i8 0, ptr %17, align 8, !noalias !16050
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)(ptr noalias nofree noundef align 8 captures(none) dereferenceable(24) %31, ptr noundef nonnull align 8 %163, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %17)
          to label %794 unwind label %366

794:                                              ; preds = %793
  call void @llvm.lifetime.end.p0(ptr nonnull %17), !noalias !16050
  br label %801

795:                                              ; preds = %.thread128, %801, %792, %778, %772, %721, %714, %708, %705, %684, %662, %634, %627
  %796 = phi ptr [ %615, %721 ], [ %773, %772 ], [ %615, %662 ], [ %615, %627 ], [ %615, %634 ], [ %615, %792 ], [ %773, %778 ], [ %615, %801 ], [ %615, %684 ], [ %615, %705 ], [ %615, %714 ], [ %615, %708 ], [ %615, %.thread128 ]
  %797 = phi ptr [ %619, %721 ], [ %774, %772 ], [ %619, %662 ], [ %619, %627 ], [ %619, %634 ], [ %619, %792 ], [ %774, %778 ], [ %619, %801 ], [ %619, %684 ], [ %619, %705 ], [ %619, %714 ], [ %619, %708 ], [ %619, %.thread128 ]
  %798 = phi i64 [ %618, %721 ], [ %775, %772 ], [ %618, %662 ], [ %618, %627 ], [ %618, %634 ], [ %618, %792 ], [ %775, %778 ], [ %618, %801 ], [ %618, %684 ], [ %618, %705 ], [ %618, %714 ], [ %618, %708 ], [ %618, %.thread128 ]
  %799 = phi ptr [ %617, %721 ], [ %776, %772 ], [ %617, %662 ], [ %617, %627 ], [ %617, %634 ], [ %617, %792 ], [ %776, %778 ], [ %617, %801 ], [ %617, %684 ], [ %617, %705 ], [ %617, %714 ], [ %617, %708 ], [ %617, %.thread128 ]
  %800 = icmp eq ptr %620, %543
  br i1 %800, label %.loopexit142, label %.preheader140

801:                                              ; preds = %794, %786
  %802 = load i8, ptr %31, align 8
  %803 = icmp eq i8 %802, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %31)
  br i1 %803, label %795, label %.loopexit141

.loopexit132:                                     ; preds = %764, %754, %700, %690, %750, %686
  %804 = phi ptr [ %619, %750 ], [ %619, %686 ], [ %693, %700 ], [ %693, %690 ], [ %757, %754 ], [ %757, %764 ]
  store ptr %804, ptr %191, align 8
  br label %.loopexit141

.loopexit141:                                     ; preds = %801, %778, %721, %.loopexit132, %747, %679
  %805 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %805, ptr noundef nonnull align 8 dereferenceable(24) %49, i64 24, i1 false)
  %806 = getelementptr inbounds nuw i8, ptr %0, i64 32
  store i8 1, ptr %806, align 16
  store i64 -1, ptr %0, align 16
  br label %1038

.loopexit138:                                     ; preds = %834, %824, %821, %817
  %807 = phi ptr [ %622, %817 ], [ %827, %821 ], [ %827, %824 ], [ %827, %834 ]
  %808 = phi i64 [ %623, %817 ], [ %393, %834 ], [ %826, %824 ], [ %822, %821 ]
  %809 = phi ptr [ %624, %817 ], [ %827, %821 ], [ %827, %824 ], [ %827, %834 ]
  store ptr %807, ptr %191, align 8
  br label %810

810:                                              ; preds = %928, %903, %.loopexit138, %.loopexit142
  %811 = phi ptr [ %898, %903 ], [ %929, %928 ], [ %622, %.loopexit142 ], [ %807, %.loopexit138 ]
  %812 = phi i64 [ %899, %903 ], [ %930, %928 ], [ %623, %.loopexit142 ], [ %808, %.loopexit138 ]
  %813 = phi ptr [ %900, %903 ], [ %931, %928 ], [ %624, %.loopexit142 ], [ %809, %.loopexit138 ]
  %814 = icmp eq i64 %395, 0
  br i1 %814, label %.loopexit136, label %815

815:                                              ; preds = %810
  %816 = load ptr, ptr %70, align 8, !alias.scope !16053, !noalias !16056, !nonnull !1740, !noundef !1740
  br label %993

817:                                              ; preds = %.loopexit142
  %818 = icmp eq ptr %624, %302
  br i1 %818, label %.loopexit138, label %819

819:                                              ; preds = %817
  %820 = add i64 %393, -1
  br label %824

821:                                              ; preds = %834
  %822 = add i64 %826, 1
  %823 = icmp eq ptr %827, %302
  br i1 %823, label %.loopexit138, label %824

824:                                              ; preds = %821, %819
  %825 = phi ptr [ %827, %821 ], [ %624, %819 ]
  %826 = phi i64 [ %822, %821 ], [ %623, %819 ]
  %827 = getelementptr inbounds nuw i8, ptr %825, i64 88
  %828 = getelementptr inbounds nuw i8, ptr %825, i64 8
  %829 = load i64, ptr %828, align 8, !noalias !16058
  %830 = icmp eq i64 %829, -1
  br i1 %830, label %.loopexit138, label %831

831:                                              ; preds = %824
  %832 = getelementptr inbounds nuw i8, ptr %825, i64 16
  %833 = load i64, ptr %825, align 8, !noalias !16058
  call void @llvm.lifetime.start.p0(ptr nonnull %16), !noalias !16061
  store i64 %829, ptr %16, align 8, !noalias !16061
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %222, ptr noundef nonnull align 8 dereferenceable(72) %832, i64 72, i1 false)
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %213, i64 noundef %833, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %16)
          to label %834 unwind label %364

834:                                              ; preds = %831
  call void @llvm.lifetime.end.p0(ptr nonnull %16), !noalias !16061
  %835 = icmp eq i64 %826, %820
  br i1 %835, label %.loopexit138, label %821

.loopexit149:                                     ; preds = %851, %602
  %836 = icmp eq i64 %384, %536
  br i1 %836, label %.loopexit147, label %.lr.ph

837:                                              ; preds = %.lr.ph
  %838 = icmp eq ptr %541, %840
  br i1 %838, label %.loopexit147, label %.lr.ph

.lr.ph:                                           ; preds = %.loopexit149, %837
  %839 = phi ptr [ %840, %837 ], [ %543, %.loopexit149 ]
  %840 = getelementptr inbounds i8, ptr %839, i64 -24
  %841 = load i8, ptr %840, align 8, !range !11184, !noalias !16064, !noundef !1740
  %842 = icmp eq i8 %841, 2
  br i1 %842, label %871, label %837

.preheader148:                                    ; preds = %602, %851
  %843 = phi ptr [ %844, %851 ], [ %541, %602 ]
  %844 = getelementptr inbounds nuw i8, ptr %843, i64 24
  %845 = load i8, ptr %843, align 8, !range !11184, !noundef !1740
  %846 = icmp eq i8 %845, 0
  br i1 %846, label %847, label %851

847:                                              ; preds = %.preheader148
  %848 = getelementptr inbounds nuw i8, ptr %843, i64 1
  %849 = load i8, ptr %848, align 1, !range !1741, !noundef !1740
  %850 = icmp eq i8 %849, -1
  br i1 %850, label %851, label %853

851:                                              ; preds = %862, %856, %853, %847, %.preheader148
  %852 = icmp eq ptr %844, %543
  br i1 %852, label %.loopexit149, label %.preheader148

853:                                              ; preds = %847
  %854 = load ptr, ptr %201, align 8, !noundef !1740
  %855 = icmp eq ptr %854, null
  br i1 %855, label %851, label %856

856:                                              ; preds = %853
  %857 = load i32, ptr %202, align 4, !noundef !1740
  %858 = getelementptr i8, ptr %854, i64 56
  %859 = load i64, ptr %858, align 8, !noundef !1740
  %860 = zext i32 %857 to i64
  %861 = icmp ugt i64 %859, %860
  br i1 %861, label %862, label %851

862:                                              ; preds = %856
  %863 = getelementptr i8, ptr %854, i64 48
  %864 = load ptr, ptr %863, align 8, !nonnull !1740, !noundef !1740
  %865 = getelementptr inbounds nuw i8, ptr %843, i64 8
  %866 = load i64, ptr %865, align 8, !noundef !1740
  %867 = zext nneg i8 %849 to i64
  %868 = getelementptr inbounds nuw [136 x i8], ptr %864, i64 %860
  %869 = getelementptr inbounds nuw [8 x i8], ptr %868, i64 %867
  %870 = atomicrmw add ptr %869, i64 %866 monotonic, align 8
  br label %851

871:                                              ; preds = %.lr.ph
  %872 = getelementptr i8, ptr %839, i64 -16
  %873 = load i64, ptr %872, align 8, !noalias !16064
  %874 = icmp ult i64 %387, %873
  br i1 %874, label %875, label %.loopexit147

875:                                              ; preds = %871
  %876 = icmp eq ptr %386, %302
  br i1 %876, label %.loopexit145, label %877

877:                                              ; preds = %875
  %878 = add i64 %873, -1
  br label %882

879:                                              ; preds = %892
  %880 = add i64 %884, 1
  %881 = icmp eq ptr %885, %302
  br i1 %881, label %.loopexit145, label %882

882:                                              ; preds = %879, %877
  %883 = phi ptr [ %885, %879 ], [ %386, %877 ]
  %884 = phi i64 [ %880, %879 ], [ %387, %877 ]
  %885 = getelementptr inbounds nuw i8, ptr %883, i64 88
  %886 = getelementptr inbounds nuw i8, ptr %883, i64 8
  %887 = load i64, ptr %886, align 8, !noalias !16067
  %888 = icmp eq i64 %887, -1
  br i1 %888, label %.loopexit145, label %889

889:                                              ; preds = %882
  %890 = getelementptr inbounds nuw i8, ptr %883, i64 16
  %891 = load i64, ptr %883, align 8, !noalias !16067
  call void @llvm.lifetime.start.p0(ptr nonnull %15), !noalias !16070
  store i64 %887, ptr %15, align 8, !noalias !16070
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %223, ptr noundef nonnull align 8 dereferenceable(72) %890, i64 72, i1 false)
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %213, i64 noundef %891, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %15)
          to label %892 unwind label %362

892:                                              ; preds = %889
  call void @llvm.lifetime.end.p0(ptr nonnull %15), !noalias !16070
  %893 = icmp eq i64 %884, %878
  br i1 %893, label %.loopexit145, label %879

.loopexit145:                                     ; preds = %892, %882, %879, %875
  %894 = phi ptr [ %383, %875 ], [ %885, %879 ], [ %885, %882 ], [ %885, %892 ]
  %895 = phi i64 [ %387, %875 ], [ %873, %892 ], [ %884, %882 ], [ %880, %879 ]
  %896 = phi ptr [ %386, %875 ], [ %885, %879 ], [ %885, %882 ], [ %885, %892 ]
  store ptr %894, ptr %191, align 8
  br label %.loopexit147

.loopexit147:                                     ; preds = %837, %.loopexit149, %.loopexit145, %871
  %897 = phi i1 [ false, %.loopexit145 ], [ false, %871 ], [ true, %.loopexit149 ], [ true, %837 ]
  %898 = phi ptr [ %894, %.loopexit145 ], [ %383, %871 ], [ %383, %.loopexit149 ], [ %383, %837 ]
  %899 = phi i64 [ %895, %.loopexit145 ], [ %387, %871 ], [ %387, %.loopexit149 ], [ %387, %837 ]
  %900 = phi ptr [ %896, %.loopexit145 ], [ %386, %871 ], [ %386, %.loopexit149 ], [ %386, %837 ]
  %901 = icmp eq i64 %569, 0
  %902 = or i1 %165, %901
  br i1 %902, label %903, label %904

903:                                              ; preds = %920, %.loopexit147
  br i1 %168, label %922, label %810

904:                                              ; preds = %.loopexit147
  call void @llvm.lifetime.start.p0(ptr nonnull %14), !noalias !16073
  %905 = load atomic i64, ptr %197 monotonic, align 8, !noalias !16080
  br label %906

906:                                              ; preds = %906, %904
  %907 = phi i64 [ %905, %904 ], [ %911, %906 ]
  %908 = call i64 @llvm.uadd.sat.i64(i64 %907, i64 %569)
  %909 = cmpxchg weak ptr %197, i64 %907, i64 %908 monotonic monotonic, align 8, !noalias !16080
  %910 = extractvalue { i64, i1 } %909, 1
  %911 = extractvalue { i64, i1 } %909, 0
  br i1 %910, label %912, label %906

912:                                              ; preds = %906
  %913 = call i64 @llvm.uadd.sat.i64(i64 %911, i64 %569)
  %914 = load i64, ptr %163, align 8, !noalias !16080
  %915 = icmp ugt i64 %913, %914
  br i1 %915, label %916, label %920

916:                                              ; preds = %912
  call void @llvm.lifetime.start.p0(ptr nonnull %13), !noalias !16080
  store i8 0, ptr %224, align 1, !noalias !16080
  store i64 %914, ptr %225, align 8, !noalias !16080
  store i64 %913, ptr %226, align 8, !noalias !16080
  store i8 0, ptr %13, align 8, !noalias !16080
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %14, ptr noundef nonnull align 8 %163, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %13)
          to label %917 unwind label %360

917:                                              ; preds = %916
  call void @llvm.lifetime.end.p0(ptr nonnull %13), !noalias !16080
  %918 = load i8, ptr %14, align 8, !noalias !16073
  %919 = icmp eq i8 %918, -1
  br i1 %919, label %920, label %921

920:                                              ; preds = %917, %912
  call void @llvm.lifetime.end.p0(ptr nonnull %14), !noalias !16073
  br label %903

921:                                              ; preds = %917
  call void @llvm.lifetime.end.p0(ptr nonnull %14), !noalias !16073
  br i1 %168, label %968, label %990

922:                                              ; preds = %903
  call void @llvm.lifetime.start.p0(ptr nonnull %36)
  %923 = load i64, ptr %166, align 8
  %924 = icmp eq i64 %923, -1
  br i1 %924, label %<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.12908414067662811932.exit.thread), label %932

.loopexit143:                                     ; preds = %988, %978, %975, %971
  %925 = phi ptr [ %898, %971 ], [ %981, %975 ], [ %981, %978 ], [ %981, %988 ]
  %926 = phi i64 [ %899, %971 ], [ %393, %988 ], [ %980, %978 ], [ %976, %975 ]
  %927 = phi ptr [ %900, %971 ], [ %981, %975 ], [ %981, %978 ], [ %981, %988 ]
  store ptr %925, ptr %191, align 8
  br label %928

928:                                              ; preds = %968, %.loopexit143
  %929 = phi ptr [ %898, %968 ], [ %925, %.loopexit143 ]
  %930 = phi i64 [ %899, %968 ], [ %926, %.loopexit143 ]
  %931 = phi ptr [ %900, %968 ], [ %927, %.loopexit143 ]
  br i1 %969, label %990, label %810

932:                                              ; preds = %922
  %933 = load atomic i32, ptr %204 acquire, align 8, !noalias !16083
  %934 = icmp eq i32 %933, 0
  br i1 %934, label %946, label %935

935:                                              ; preds = %932
  %936 = load atomic i64, ptr %208 monotonic, align 8, !noalias !16083
  br label %937

937:                                              ; preds = %937, %935
  %938 = phi i64 [ %936, %935 ], [ %942, %937 ]
  %939 = call i64 @llvm.uadd.sat.i64(i64 %938, i64 %570)
  %940 = cmpxchg weak ptr %208, i64 %938, i64 %939 monotonic monotonic, align 8, !noalias !16083
  %941 = extractvalue { i64, i1 } %940, 1
  %942 = extractvalue { i64, i1 } %940, 0
  br i1 %941, label %943, label %937

943:                                              ; preds = %937
  %944 = call i64 @llvm.uadd.sat.i64(i64 %942, i64 %570)
  %.sroa.3122.0.copyload = load i64, ptr %166, align 8, !noalias !16083
  %945 = icmp ugt i64 %944, %.sroa.3122.0.copyload
  br i1 %945, label %947, label %<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.12908414067662811932.exit.thread)

946:                                              ; preds = %932
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %36, ptr noundef nonnull align 8 dereferenceable(24) %205, i64 24, i1 false)
  br label %<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.12908414067662811932.exit)

947:                                              ; preds = %943
  call void @llvm.lifetime.start.p0(ptr nonnull %8), !noalias !16083
  store i8 3, ptr %238, align 1, !noalias !16083
  store i64 %.sroa.3122.0.copyload, ptr %239, align 8, !noalias !16083
  store i64 %944, ptr %240, align 8, !noalias !16083
  store i8 0, ptr %8, align 8, !noalias !16083
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %36, ptr noundef nonnull align 8 %163, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %8)
          to label %.noexc115 unwind label %372

.noexc115:                                        ; preds = %947
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !16083
  br label %<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.12908414067662811932.exit)

<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.12908414067662811932.exit.thread): ; preds = %943, %<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.12908414067662811932.exit), %922
  call void @llvm.lifetime.end.p0(ptr nonnull %36)
  br i1 %897, label %950, label %952

<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.12908414067662811932.exit): ; preds = %.noexc115, %946
  %.pr127 = load i8, ptr %36, align 8
  %948 = icmp eq i8 %.pr127, -1
  br i1 %948, label %<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.12908414067662811932.exit.thread), label %949

949:                                              ; preds = %<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.12908414067662811932.exit)
  call void @llvm.lifetime.end.p0(ptr nonnull %36)
  br label %968

950:                                              ; preds = %953, %<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.12908414067662811932.exit.thread)
  %951 = icmp eq i64 %571, 0
  br i1 %951, label %968, label %956

952:                                              ; preds = %<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.12908414067662811932.exit.thread)
  call void @llvm.lifetime.start.p0(ptr nonnull %35)
; invoke <purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
  invoke fastcc void @<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %35, ptr noundef nonnull align 16 %1)
          to label %953 unwind label %372

953:                                              ; preds = %952
  %954 = load i8, ptr %35, align 8, !range !1743, !noundef !1740
  %955 = icmp eq i8 %954, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %35)
  br i1 %955, label %950, label %968

956:                                              ; preds = %950
  call void @llvm.lifetime.start.p0(ptr nonnull %34)
  call void @llvm.experimental.noalias.scope.decl(metadata !16086)
  %957 = load atomic i32, ptr %204 acquire, align 8, !noalias !16086
  %958 = icmp eq i32 %957, 0
  br i1 %958, label %959, label %960

959:                                              ; preds = %956
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %34, ptr noundef nonnull align 8 dereferenceable(24) %205, i64 24, i1 false)
  br label %<purrdf_sparql_eval::governor::GovernorState>::admit_transient (.exit)

960:                                              ; preds = %956
  %961 = load atomic i64, ptr %208 monotonic, align 8, !noalias !16086
  %962 = call i64 @llvm.uadd.sat.i64(i64 %961, i64 %571)
  %.sroa.3125.0.copyload = load i64, ptr %166, align 8, !noalias !16086
  %963 = icmp ugt i64 %962, %.sroa.3125.0.copyload
  br i1 %963, label %965, label %964

964:                                              ; preds = %960
  store i8 -1, ptr %34, align 8, !alias.scope !16086
  br label %<purrdf_sparql_eval::governor::GovernorState>::admit_transient (.exit)

965:                                              ; preds = %960
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !16086
  store i8 3, ptr %241, align 1, !noalias !16086
  store i64 %.sroa.3125.0.copyload, ptr %242, align 8, !noalias !16086
  store i64 %962, ptr %243, align 8, !noalias !16086
  store i8 0, ptr %7, align 8, !noalias !16086
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %34, ptr noundef nonnull align 8 %163, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %7)
          to label %.noexc116 unwind label %372

.noexc116:                                        ; preds = %965
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !16086
  br label %<purrdf_sparql_eval::governor::GovernorState>::admit_transient (.exit)

<purrdf_sparql_eval::governor::GovernorState>::admit_transient (.exit): ; preds = %.noexc116, %964, %959
  %966 = load i8, ptr %34, align 8, !range !1743, !noundef !1740
  %967 = icmp ne i8 %966, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %34)
  br label %968

968:                                              ; preds = %<purrdf_sparql_eval::governor::GovernorState>::admit_transient (.exit), %953, %950, %949, %921
  %969 = phi i1 [ false, %950 ], [ %967, %<purrdf_sparql_eval::governor::GovernorState>::admit_transient (.exit) ], [ true, %921 ], [ true, %949 ], [ true, %953 ]
  %970 = icmp ult i64 %899, %393
  br i1 %970, label %971, label %928

971:                                              ; preds = %968
  %972 = icmp eq ptr %900, %302
  br i1 %972, label %.loopexit143, label %973

973:                                              ; preds = %971
  %974 = add i64 %393, -1
  br label %978

975:                                              ; preds = %988
  %976 = add i64 %980, 1
  %977 = icmp eq ptr %981, %302
  br i1 %977, label %.loopexit143, label %978

978:                                              ; preds = %975, %973
  %979 = phi ptr [ %981, %975 ], [ %900, %973 ]
  %980 = phi i64 [ %976, %975 ], [ %899, %973 ]
  %981 = getelementptr inbounds nuw i8, ptr %979, i64 88
  %982 = getelementptr inbounds nuw i8, ptr %979, i64 8
  %983 = load i64, ptr %982, align 8, !noalias !16089
  %984 = icmp eq i64 %983, -1
  br i1 %984, label %.loopexit143, label %985

985:                                              ; preds = %978
  %986 = getelementptr inbounds nuw i8, ptr %979, i64 16
  %987 = load i64, ptr %979, align 8, !noalias !16089
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !16092
  store i64 %983, ptr %12, align 8, !noalias !16092
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %227, ptr noundef nonnull align 8 dereferenceable(72) %986, i64 72, i1 false)
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %213, i64 noundef %987, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %12)
          to label %988 unwind label %358

988:                                              ; preds = %985
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !16092
  %989 = icmp eq i64 %980, %974
  br i1 %989, label %.loopexit143, label %975

990:                                              ; preds = %928, %921
  %991 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %991, ptr noundef nonnull align 8 dereferenceable(24) %49, i64 24, i1 false)
  %992 = getelementptr inbounds nuw i8, ptr %0, i64 32
  store i8 1, ptr %992, align 16
  store i64 -1, ptr %0, align 16
  br label %1038

993:                                              ; preds = %1031, %815
  %994 = phi i64 [ %395, %815 ], [ %996, %1031 ]
  %995 = phi ptr [ %382, %815 ], [ %1001, %1031 ]
  %996 = add i64 %994, -1
  %997 = icmp eq ptr %995, %816
  br i1 %997, label %.loopexit136, label %1000

.loopexit136:                                     ; preds = %1031, %993, %810
  %998 = phi ptr [ %382, %810 ], [ %1001, %1031 ], [ %995, %993 ]
  store ptr %998, ptr %69, align 8
  %999 = icmp eq ptr %388, %308
  br i1 %999, label %.loopexit154, label %381

1000:                                             ; preds = %993
  %1001 = getelementptr inbounds nuw i8, ptr %995, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %29)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %228, ptr noundef nonnull align 8 dereferenceable(40) %995, i64 40, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %30)
  store ptr %1, ptr %29, align 8
  call void @llvm.experimental.noalias.scope.decl(metadata !16095)
  call void @llvm.experimental.noalias.scope.decl(metadata !16098)
  %1002 = load i64, ptr %228, align 8, !alias.scope !16098, !noalias !16100, !noundef !1740
  %1003 = icmp eq i64 %1002, 0
  br i1 %1003, label %1004, label %1006

1004:                                             ; preds = %1000
  %1005 = load ptr, ptr %230, align 8, !alias.scope !16095, !noalias !16102, !nonnull !1740, !align !1836, !noundef !1740
; invoke purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(96) %30, ptr noalias nofree noundef align 8 dereferenceable(184) %213, ptr noundef nonnull align 8 %1005, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %231)
          to label %1007 unwind label %356

1006:                                             ; preds = %1000
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %229, ptr noundef nonnull align 8 dereferenceable(40) %995, i64 40, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %29)
  br label %1018

1007:                                             ; preds = %1004
  %1008 = load i64, ptr %30, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %29)
  %1009 = icmp eq i64 %1008, -1
  br i1 %1009, label %1018, label %1010

1010:                                             ; preds = %1007
  store ptr %1001, ptr %69, align 8
  %1011 = load i64, ptr %229, align 8
  %1012 = load ptr, ptr %232, align 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %28, ptr noundef nonnull align 8 dereferenceable(24) %233, i64 24, i1 false)
  %1013 = getelementptr inbounds nuw i8, ptr %30, i64 48
  %1014 = getelementptr inbounds nuw i8, ptr %0, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %1014, ptr noundef nonnull align 16 dereferenceable(48) %1013, i64 48, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %30)
  %1015 = getelementptr inbounds nuw i8, ptr %0, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %1015, ptr noundef nonnull align 8 dereferenceable(24) %28, i64 24, i1 false)
  store i64 %1008, ptr %0, align 16
  %1016 = getelementptr inbounds nuw i8, ptr %0, i64 8
  store i64 %1011, ptr %1016, align 8
  %1017 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store ptr %1012, ptr %1017, align 16
  br label %1038

1018:                                             ; preds = %1007, %1006
  %1019 = load i64, ptr %229, align 8
  %1020 = load ptr, ptr %232, align 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %28, ptr noundef nonnull align 8 dereferenceable(24) %233, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %30)
  call void @llvm.experimental.noalias.scope.decl(metadata !16103)
  %1021 = load i64, ptr %234, align 8, !alias.scope !16103, !noalias !16106, !noundef !1740
  %1022 = load i64, ptr %49, align 8, !range !1835, !alias.scope !16103, !noalias !16106, !noundef !1740
  %1023 = icmp eq i64 %1021, %1022
  br i1 %1023, label %1024, label %1031

1024:                                             ; preds = %1018
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %49)
          to label %1031 unwind label %1025, !noalias !16106

1025:                                             ; preds = %1024
  %1026 = landingpad { ptr, i32 }
          cleanup
  store ptr %1001, ptr %69, align 8
  %1027 = icmp ugt i64 %1019, 5
  br i1 %1027, label %1028, label %376

1028:                                             ; preds = %1025
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1020) ]
  %1029 = shl i64 %1019, 3
  %1030 = add i64 %1029, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1020, i64 noundef %1030, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !16108
  br label %376

1031:                                             ; preds = %1024, %1018
  %1032 = load ptr, ptr %235, align 8, !alias.scope !16103, !noalias !16106, !nonnull !1740, !noundef !1740
  %1033 = getelementptr inbounds nuw [40 x i8], ptr %1032, i64 %1021
  store i64 %1019, ptr %1033, align 8, !noalias !16103
  %1034 = getelementptr inbounds nuw i8, ptr %1033, i64 8
  store ptr %1020, ptr %1034, align 8, !noalias !16103
  %1035 = getelementptr inbounds nuw i8, ptr %1033, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %1035, ptr noundef nonnull align 8 dereferenceable(24) %28, i64 24, i1 false)
  %1036 = add i64 %1021, 1
  store i64 %1036, ptr %234, align 8, !alias.scope !16103, !noalias !16106
  %1037 = icmp eq i64 %996, 0
  br i1 %1037, label %.loopexit136, label %993

1038:                                             ; preds = %1010, %990, %.loopexit141, %518
  %1039 = phi i8 [ 0, %518 ], [ 0, %990 ], [ 1, %1010 ], [ 0, %.loopexit141 ]
  %1040 = icmp eq i64 %304, 0
  br i1 %1040, label %1070, label %1041

1041:                                             ; preds = %1038
  %1042 = shl nuw i64 %304, 5
  %1043 = load i64, ptr %244, align 8, !noalias !16111, !noundef !1740
  %1044 = call i64 @llvm.umin.i64(i64 %1042, i64 9223372036854775807)
  %1045 = call i64 @llvm.ssub.sat.i64(i64 %1043, i64 %1044)
  store i64 %1045, ptr %244, align 8, !noalias !16111
  %1046 = load i64, ptr %245, align 8, !noalias !16111, !noundef !1740
  %1047 = icmp slt i64 %1045, %1046
  br i1 %1047, label %1048, label %.preheader300

1048:                                             ; preds = %1041
  store i64 %1045, ptr %245, align 8, !noalias !16111
  br label %.preheader300

.preheader300:                                    ; preds = %1048, %1041
  br label %1049

1049:                                             ; preds = %.preheader300, %1052
  %1050 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !16111
  %1051 = icmp slt i64 %1050, 0
  br i1 %1051, label %1052, label %__rustc::__rust_dealloc (.exit117)

1052:                                             ; preds = %1049
  %1053 = add nsw i64 %1050, 1
  %1054 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1050, i64 %1053 acq_rel acquire, align 8, !noalias !16111
  %1055 = extractvalue { i64, i1 } %1054, 1
  br i1 %1055, label %1056, label %1049

1056:                                             ; preds = %1052
  %1057 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1044 monotonic, align 8, !noalias !16111
  %1058 = call i64 @llvm.ssub.sat.i64(i64 %1057, i64 %1044)
  %1059 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !16111
  br label %1060

1060:                                             ; preds = %1063, %1056
  %1061 = phi i64 [ %1059, %1056 ], [ %1066, %1063 ]
  %1062 = icmp slt i64 %1058, %1061
  br i1 %1062, label %1063, label %1067

1063:                                             ; preds = %1060
  %1064 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1061, i64 %1058 monotonic monotonic, align 8, !noalias !16111
  %1065 = extractvalue { i64, i1 } %1064, 1
  %1066 = extractvalue { i64, i1 } %1064, 0
  br i1 %1065, label %1067, label %1060

1067:                                             ; preds = %1063, %1060
  %1068 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !16111
  br label %__rustc::__rust_dealloc (.exit117)

__rustc::__rust_dealloc (.exit117): ; preds = %1049, %1067
  call void @free(ptr noundef nonnull %303) #92, !noalias !16111
  br label %1070

1069:                                             ; preds = %538
  unreachable

1070:                                             ; preds = %__rustc::__rust_dealloc (.exit117), %1038
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %37)
          to label %1071 unwind label %437

1071:                                             ; preds = %1070
  call void @llvm.lifetime.end.p0(ptr nonnull %37)
  %1072 = icmp eq i64 %296, 0
  br i1 %1072, label %1101, label %1073

1073:                                             ; preds = %1071
  %1074 = mul nuw i64 %296, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %297) ]
  %1075 = load i64, ptr %244, align 8, !noundef !1740
  %1076 = call i64 @llvm.umin.i64(i64 %1074, i64 9223372036854775807)
  %1077 = call i64 @llvm.ssub.sat.i64(i64 %1075, i64 %1076)
  store i64 %1077, ptr %244, align 8
  %1078 = load i64, ptr %245, align 8, !noundef !1740
  %1079 = icmp slt i64 %1077, %1078
  br i1 %1079, label %1080, label %.preheader299

1080:                                             ; preds = %1073
  store i64 %1077, ptr %245, align 8
  br label %.preheader299

.preheader299:                                    ; preds = %1080, %1073
  br label %1081

1081:                                             ; preds = %.preheader299, %1084
  %1082 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8
  %1083 = icmp slt i64 %1082, 0
  br i1 %1083, label %1084, label %__rustc::__rust_dealloc (.exit118)

1084:                                             ; preds = %1081
  %1085 = add nsw i64 %1082, 1
  %1086 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1082, i64 %1085 acq_rel acquire, align 8
  %1087 = extractvalue { i64, i1 } %1086, 1
  br i1 %1087, label %1088, label %1081

1088:                                             ; preds = %1084
  %1089 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1076 monotonic, align 8
  %1090 = call i64 @llvm.ssub.sat.i64(i64 %1089, i64 %1076)
  %1091 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8
  br label %1092

1092:                                             ; preds = %1095, %1088
  %1093 = phi i64 [ %1091, %1088 ], [ %1098, %1095 ]
  %1094 = icmp slt i64 %1090, %1093
  br i1 %1094, label %1095, label %1099

1095:                                             ; preds = %1092
  %1096 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1093, i64 %1090 monotonic monotonic, align 8
  %1097 = extractvalue { i64, i1 } %1096, 1
  %1098 = extractvalue { i64, i1 } %1096, 0
  br i1 %1097, label %1099, label %1092

1099:                                             ; preds = %1095, %1092
  %1100 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8
  br label %__rustc::__rust_dealloc (.exit118)

__rustc::__rust_dealloc (.exit118): ; preds = %1081, %1099
  call void @free(ptr noundef nonnull %297) #92
  br label %1101

1101:                                             ; preds = %__rustc::__rust_dealloc (.exit118), %1071
  call void @llvm.experimental.noalias.scope.decl(metadata !16114)
  %1102 = load ptr, ptr %236, align 8, !alias.scope !16114, !noundef !1740
  %1103 = icmp eq ptr %1102, null
  br i1 %1103, label %1108, label %1104

1104:                                             ; preds = %1101
  %1105 = atomicrmw sub ptr %1102, i64 1 release, align 8, !noalias !16117
  %1106 = icmp eq i64 %1105, 1
  br i1 %1106, label %1107, label %1108

1107:                                             ; preds = %1104
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %236) #91
  br label %1108

1108:                                             ; preds = %1107, %1104, %1101
  call void @llvm.experimental.noalias.scope.decl(metadata !16122)
  %1109 = load ptr, ptr %237, align 8, !alias.scope !16122, !noundef !1740
  %1110 = icmp eq ptr %1109, null
  br i1 %1110, label %1115, label %1111

1111:                                             ; preds = %1108
  %1112 = atomicrmw sub ptr %1109, i64 1 release, align 8, !noalias !16125
  %1113 = icmp eq i64 %1112, 1
  br i1 %1113, label %1114, label %1115

1114:                                             ; preds = %1111
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %237) #91
  br label %1115

1115:                                             ; preds = %1114, %1111, %1108
  call void @llvm.lifetime.end.p0(ptr nonnull %38)
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %39)
          to label %1116 unwind label %.loopexit.split-lp

1116:                                             ; preds = %1115
  call void @llvm.lifetime.end.p0(ptr nonnull %39)
  %1117 = atomicrmw sub ptr %162, i64 1 release, align 8, !noalias !16130
  %1118 = icmp eq i64 %1117, 1
  br i1 %1118, label %1119, label %1120

1119:                                             ; preds = %1116
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %40) #91
          to label %1120 unwind label %106

1120:                                             ; preds = %1119, %1116
  call void @llvm.lifetime.end.p0(ptr nonnull %40)
  br label %156

1121:                                             ; preds = %1123, %156
  call void @llvm.lifetime.end.p0(ptr nonnull %45)
  %1122 = trunc nuw i8 %157 to i1
  br i1 %1122, label %1124, label %122

1123:                                             ; preds = %156
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %45)
          to label %1121 unwind label %118

1124:                                             ; preds = %1121
  call void @llvm.experimental.noalias.scope.decl(metadata !16135)
  %1125 = getelementptr inbounds nuw i8, ptr %49, i64 8
  %1126 = load ptr, ptr %1125, align 8, !alias.scope !16135, !nonnull !1740, !noundef !1740
  %1127 = getelementptr inbounds nuw i8, ptr %49, i64 16
  %1128 = load i64, ptr %1127, align 8, !alias.scope !16135, !noundef !1740
  call void @llvm.experimental.noalias.scope.decl(metadata !16138)
  %1129 = icmp eq i64 %1128, 0
  br i1 %1129, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %1124
  %1130 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1131 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %1132

1132:                                             ; preds = %.preheader, %1170
  %1133 = phi i64 [ %1135, %1170 ], [ 0, %.preheader ]
  %1134 = getelementptr inbounds nuw [40 x i8], ptr %1126, i64 %1133
  %1135 = add nuw nsw i64 %1133, 1
  %1136 = load i64, ptr %1134, align 8, !range !1778, !alias.scope !16141, !noalias !16135, !noundef !1740
  %1137 = icmp ugt i64 %1136, 5
  br i1 %1137, label %1138, label %1170

1138:                                             ; preds = %1132
  %1139 = getelementptr i8, ptr %1134, i64 8
  %1140 = load ptr, ptr %1139, align 8, !alias.scope !16138, !noalias !16135, !nonnull !1740, !noundef !1740
  %1141 = shl i64 %1136, 3
  %1142 = add i64 %1141, -8
  %1143 = load i64, ptr %1130, align 8, !noalias !16144, !noundef !1740
  %1144 = call i64 @llvm.umin.i64(i64 %1142, i64 9223372036854775807)
  %1145 = call i64 @llvm.ssub.sat.i64(i64 %1143, i64 %1144)
  store i64 %1145, ptr %1130, align 8, !noalias !16144
  %1146 = load i64, ptr %1131, align 8, !noalias !16144, !noundef !1740
  %1147 = icmp slt i64 %1145, %1146
  br i1 %1147, label %1148, label %.preheader289

1148:                                             ; preds = %1138
  store i64 %1145, ptr %1131, align 8, !noalias !16144
  br label %.preheader289

.preheader289:                                    ; preds = %1148, %1138
  br label %1149

1149:                                             ; preds = %.preheader289, %1152
  %1150 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !16144
  %1151 = icmp slt i64 %1150, 0
  br i1 %1151, label %1152, label %__rustc::__rust_dealloc (.exit119)

1152:                                             ; preds = %1149
  %1153 = add nsw i64 %1150, 1
  %1154 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1150, i64 %1153 acq_rel acquire, align 8, !noalias !16144
  %1155 = extractvalue { i64, i1 } %1154, 1
  br i1 %1155, label %1156, label %1149

1156:                                             ; preds = %1152
  %1157 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1144 monotonic, align 8, !noalias !16144
  %1158 = call i64 @llvm.ssub.sat.i64(i64 %1157, i64 %1144)
  %1159 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !16144
  br label %1160

1160:                                             ; preds = %1163, %1156
  %1161 = phi i64 [ %1159, %1156 ], [ %1166, %1163 ]
  %1162 = icmp slt i64 %1158, %1161
  br i1 %1162, label %1163, label %1167

1163:                                             ; preds = %1160
  %1164 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1161, i64 %1158 monotonic monotonic, align 8, !noalias !16144
  %1165 = extractvalue { i64, i1 } %1164, 1
  %1166 = extractvalue { i64, i1 } %1164, 0
  br i1 %1165, label %1167, label %1160

1167:                                             ; preds = %1163, %1160
  %1168 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !16144
  br label %__rustc::__rust_dealloc (.exit119)

__rustc::__rust_dealloc (.exit119): ; preds = %1149, %1167
  %1169 = icmp ne i64 %1142, 0
  call void @llvm.assume(i1 %1169), !noalias !16144
  call void @free(ptr noundef nonnull %1140) #92, !noalias !16144
  br label %1170

1170:                                             ; preds = %__rustc::__rust_dealloc (.exit119), %1132
  %1171 = icmp eq i64 %1135, %1128
  br i1 %1171, label %.loopexit, label %1132

.loopexit:                                        ; preds = %1170, %1124
  %1172 = load i64, ptr %49, align 8, !alias.scope !16135
  %1173 = icmp eq i64 %1172, 0
  br i1 %1173, label %122, label %1174

1174:                                             ; preds = %.loopexit
  %1175 = mul nuw i64 %1172, 40
  %1176 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1177 = load i64, ptr %1176, align 8, !noalias !16135, !noundef !1740
  %1178 = call i64 @llvm.umin.i64(i64 %1175, i64 9223372036854775807)
  %1179 = call i64 @llvm.ssub.sat.i64(i64 %1177, i64 %1178)
  store i64 %1179, ptr %1176, align 8, !noalias !16135
  %1180 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1181 = load i64, ptr %1180, align 8, !noalias !16135, !noundef !1740
  %1182 = icmp slt i64 %1179, %1181
  br i1 %1182, label %1183, label %.preheader288

1183:                                             ; preds = %1174
  store i64 %1179, ptr %1180, align 8, !noalias !16135
  br label %.preheader288

.preheader288:                                    ; preds = %1183, %1174
  br label %1184

1184:                                             ; preds = %.preheader288, %1187
  %1185 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !16135
  %1186 = icmp slt i64 %1185, 0
  br i1 %1186, label %1187, label %__rustc::__rust_dealloc (.exit120)

1187:                                             ; preds = %1184
  %1188 = add nsw i64 %1185, 1
  %1189 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1185, i64 %1188 acq_rel acquire, align 8, !noalias !16135
  %1190 = extractvalue { i64, i1 } %1189, 1
  br i1 %1190, label %1191, label %1184

1191:                                             ; preds = %1187
  %1192 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1178 monotonic, align 8, !noalias !16135
  %1193 = call i64 @llvm.ssub.sat.i64(i64 %1192, i64 %1178)
  %1194 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !16135
  br label %1195

1195:                                             ; preds = %1198, %1191
  %1196 = phi i64 [ %1194, %1191 ], [ %1201, %1198 ]
  %1197 = icmp slt i64 %1193, %1196
  br i1 %1197, label %1198, label %1202

1198:                                             ; preds = %1195
  %1199 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1196, i64 %1193 monotonic monotonic, align 8, !noalias !16135
  %1200 = extractvalue { i64, i1 } %1199, 1
  %1201 = extractvalue { i64, i1 } %1199, 0
  br i1 %1200, label %1202, label %1195

1202:                                             ; preds = %1198, %1195
  %1203 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !16135
  br label %__rustc::__rust_dealloc (.exit120)

__rustc::__rust_dealloc (.exit120): ; preds = %1184, %1202
  call void @free(ptr noundef nonnull %1126) #92, !noalias !16135
  br label %122

1204:                                             ; preds = %475, %472, %469
  call void @llvm.experimental.noalias.scope.decl(metadata !16147)
  %1205 = load ptr, ptr %237, align 8, !alias.scope !16147, !noundef !1740
  %1206 = icmp eq ptr %1205, null
  br i1 %1206, label %288, label %1207

1207:                                             ; preds = %1204
  %1208 = atomicrmw sub ptr %1205, i64 1 release, align 8, !noalias !16150
  %1209 = icmp eq i64 %1208, 1
  br i1 %1209, label %1210, label %288

1210:                                             ; preds = %1207
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %237) #91
  br label %288

1211:                                             ; preds = %273, %267, %106
  %1212 = phi { ptr, i32 } [ %109, %106 ], [ %270, %267 ], [ %270, %273 ]
  %1213 = phi i8 [ %108, %106 ], [ %269, %267 ], [ %269, %273 ]
  %1214 = phi i8 [ %107, %106 ], [ %268, %267 ], [ %268, %273 ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %45) #89
          to label %113 unwind label %159

1215:                                             ; preds = %113, %95
  %1216 = phi { ptr, i32 } [ %116, %113 ], [ %96, %95 ]
  %1217 = phi i8 [ %115, %113 ], [ 1, %95 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %49) #89
  br label %1218

1218:                                             ; preds = %1215, %113
  %1219 = phi i8 [ %115, %113 ], [ %1217, %1215 ]
  %1220 = phi { ptr, i32 } [ %116, %113 ], [ %1216, %1215 ]
  %1221 = trunc nuw i8 %1219 to i1
  br i1 %1221, label %1229, label %1227

1222:                                             ; preds = %59
  %1223 = landingpad { ptr, i32 }
          cleanup
  br label %1229

1224:                                             ; preds = %122, %110, %59
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(24) %3)
  br label %347

1225:                                             ; preds = %6
  %1226 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(24) %4) #89
          to label %1229 unwind label %159

1227:                                             ; preds = %1229, %1218
  %1228 = phi { ptr, i32 } [ %1230, %1229 ], [ %1220, %1218 ]
  resume { ptr, i32 } %1228

1229:                                             ; preds = %1225, %1222, %1218
  %1230 = phi { ptr, i32 } [ %1223, %1222 ], [ %1220, %1218 ], [ %1226, %1225 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(24) %3) #89
          to label %1227 unwind label %159
}
