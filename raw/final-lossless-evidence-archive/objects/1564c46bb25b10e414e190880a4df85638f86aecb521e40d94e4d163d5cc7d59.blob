define internal fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_into::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>(ptr dead_on_unwind noalias nofree noundef nonnull writable writeonly align 16 captures(none) dereferenceable(96) %0, i8 range(i8 0, 3) %1, ptr noalias nofree noundef nonnull align 16 dereferenceable(1232) %2, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(24) %3, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(208) %4, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(24) %5) unnamed_addr #0 personality ptr @rust_eh_personality !guid !6110 {
  %7 = alloca [16 x i8], align 8
  %8 = alloca [48 x i8], align 8
  %9 = alloca [96 x i8], align 16
  %10 = alloca [32 x i8], align 8
  %11 = alloca [96 x i8], align 16
  %12 = alloca [24 x i8], align 8
  %13 = alloca [24 x i8], align 8
  %14 = alloca [64 x i8], align 8
  %15 = icmp ne i8 %1, 0
  %16 = getelementptr inbounds nuw i8, ptr %2, i64 608
  %17 = load ptr, ptr %16, align 16
  %18 = icmp ne ptr %17, null
  %19 = select i1 %15, i1 true, i1 %18, !prof !6111
  br i1 %19, label %20, label %21, !prof !6111

20:                                               ; preds = %6
; invoke core::panicking::panic_fmt
  invoke void @core::panicking::panic_fmt(ptr noundef nonnull @anon.68dd637f94a7f528fe69f6876e3d956b.151, ptr noundef nonnull inttoptr (i64 83 to ptr), ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.68dd637f94a7f528fe69f6876e3d956b.153) #93
          to label %210 unwind label %213

21:                                               ; preds = %6
  call void @llvm.lifetime.start.p0(ptr nonnull %14)
  call void @llvm.lifetime.start.p0(ptr nonnull %13)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %13, ptr noundef nonnull align 8 dereferenceable(24) %5, i64 24, i1 false)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6112)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6115)
  call void @llvm.lifetime.start.p0(ptr nonnull %7)
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !6117
  call void @llvm.lifetime.start.p0(ptr nonnull %11), !noalias !6117
  %22 = getelementptr inbounds nuw i8, ptr %3, i64 16
  %23 = load i64, ptr %22, align 8, !alias.scope !6115, !noalias !6120, !noundef !1733
  %24 = icmp ult i64 %23, 230584300921369396
  tail call void @llvm.assume(i1 %24)
; invoke purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
  invoke fastcc void @purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %11, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(24) %13, i64 noundef %23)
          to label %25 unwind label %194, !noalias !6121

25:                                               ; preds = %21
  %26 = load i64, ptr %11, align 16, !range !2520, !noalias !6117, !noundef !1733
  %27 = icmp eq i64 %26, -1
  %28 = getelementptr inbounds nuw i8, ptr %11, i64 8
  %29 = load i64, ptr %28, align 8, !noalias !6117
  %30 = getelementptr inbounds nuw i8, ptr %11, i64 16
  %31 = load ptr, ptr %30, align 16, !noalias !6117
  %32 = getelementptr inbounds nuw i8, ptr %11, i64 24
  %33 = load i64, ptr %32, align 8, !noalias !6117
  br i1 %27, label %36, label %34

34:                                               ; preds = %25
  %35 = getelementptr inbounds nuw i8, ptr %11, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(64) %14, ptr noundef nonnull align 16 dereferenceable(64) %35, i64 64, i1 false), !noalias !6122
  call void @llvm.lifetime.end.p0(ptr nonnull %11), !noalias !6117
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !6117
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %3)
          to label %199 unwind label %196

36:                                               ; preds = %25
  call void @llvm.lifetime.end.p0(ptr nonnull %11), !noalias !6117
  store i64 %29, ptr %12, align 8, !noalias !6117
  %37 = getelementptr inbounds nuw i8, ptr %12, i64 8
  store ptr %31, ptr %37, align 8, !noalias !6117
  %38 = getelementptr inbounds nuw i8, ptr %12, i64 16
  store i64 %33, ptr %38, align 8, !noalias !6117
  %39 = getelementptr inbounds nuw i8, ptr %3, i64 8
  %40 = load ptr, ptr %39, align 8, !alias.scope !6115, !noalias !6120, !nonnull !1733, !noundef !1733
  %41 = load i64, ptr %3, align 8, !range !1828, !alias.scope !6115, !noalias !6120, !noundef !1733
  %42 = mul nuw nsw i64 %23, 40
  %43 = getelementptr inbounds nuw i8, ptr %40, i64 %42
  call void @llvm.lifetime.start.p0(ptr nonnull %10), !noalias !6117
  store ptr %40, ptr %10, align 8, !noalias !6117
  %44 = getelementptr inbounds nuw i8, ptr %10, i64 8
  %45 = getelementptr inbounds nuw i8, ptr %10, i64 16
  store i64 %41, ptr %45, align 8, !noalias !6117
  %46 = getelementptr inbounds nuw i8, ptr %10, i64 24
  store ptr %43, ptr %46, align 8, !noalias !6117
  %47 = icmp eq i64 %23, 0
  br i1 %47, label %.loopexit9, label %48

48:                                               ; preds = %36
  %49 = getelementptr inbounds nuw i8, ptr %8, i64 8
  %50 = getelementptr inbounds nuw i8, ptr %9, i64 8
  %51 = getelementptr inbounds nuw i8, ptr %2, i64 656
  %52 = getelementptr inbounds nuw i8, ptr %2, i64 880
  %53 = getelementptr inbounds nuw i8, ptr %8, i64 16
  %54 = getelementptr inbounds nuw i8, ptr %9, i64 16
  %55 = getelementptr inbounds nuw i8, ptr %9, i64 24
  %56 = getelementptr inbounds nuw i8, ptr %9, i64 32
  br label %61

57:                                               ; preds = %68
  %58 = landingpad { ptr, i32 }
          cleanup
  store ptr %65, ptr %44, align 8, !noalias !6117
  br label %59

59:                                               ; preds = %105, %102, %57
  %60 = phi { ptr, i32 } [ %58, %57 ], [ %103, %105 ], [ %103, %102 ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %10) #89
          to label %73 unwind label %192, !noalias !6123

61:                                               ; preds = %108, %48
  %62 = phi ptr [ %31, %48 ], [ %109, %108 ]
  %63 = phi i64 [ %33, %48 ], [ %114, %108 ]
  %64 = phi ptr [ %40, %48 ], [ %65, %108 ]
  %65 = getelementptr inbounds nuw i8, ptr %64, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %8), !noalias !6117
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %49, ptr noundef nonnull align 8 dereferenceable(40) %64, i64 40, i1 false), !noalias !6123
  call void @llvm.lifetime.start.p0(ptr nonnull %9), !noalias !6117
  store ptr %2, ptr %8, align 8, !noalias !6117
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6124)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6127)
  %66 = load i64, ptr %49, align 8, !alias.scope !6127, !noalias !6129, !noundef !1733
  %67 = icmp eq i64 %66, 0
  br i1 %67, label %68, label %70

68:                                               ; preds = %61
  %69 = load ptr, ptr %51, align 16, !alias.scope !6131, !noalias !6132, !nonnull !1733, !align !1829, !noundef !1733
; invoke purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(96) %9, ptr noalias nofree noundef align 8 dereferenceable(184) %52, ptr noundef nonnull align 8 %69, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %53)
          to label %84 unwind label %57, !noalias !6123

70:                                               ; preds = %61
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %50, ptr noundef nonnull align 8 dereferenceable(40) %64, i64 40, i1 false), !noalias !6123
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !6117
  br label %93

.loopexit9:                                       ; preds = %108, %36
  %71 = phi i64 [ %33, %36 ], [ %114, %108 ]
  %72 = phi ptr [ %40, %36 ], [ %43, %108 ]
  store ptr %72, ptr %44, align 8, !noalias !6117
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %10)
          to label %77 unwind label %75, !noalias !6123

73:                                               ; preds = %75, %59
  %74 = phi { ptr, i32 } [ %76, %75 ], [ %60, %59 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %12) #89, !noalias !6123
  br label %211

75:                                               ; preds = %87, %.loopexit9
  %76 = landingpad { ptr, i32 }
          cleanup
  br label %73

77:                                               ; preds = %.loopexit9
  call void @llvm.lifetime.end.p0(ptr nonnull %10), !noalias !6117
  %78 = load i64, ptr %12, align 8, !noalias !6122
  %79 = load ptr, ptr %37, align 8, !noalias !6122
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !6117
  call void @llvm.lifetime.end.p0(ptr nonnull %7)
  call void @llvm.lifetime.end.p0(ptr nonnull %13)
  call void @llvm.lifetime.end.p0(ptr nonnull %14)
  %80 = getelementptr inbounds nuw i8, ptr %0, i64 8
  store i64 %78, ptr %80, align 8
  %81 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store ptr %79, ptr %81, align 16
  %82 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store i64 %71, ptr %82, align 8
  %83 = getelementptr inbounds nuw i8, ptr %0, i64 32
  store i64 0, ptr %83, align 16
  store i64 -1, ptr %0, align 16
  br label %209

84:                                               ; preds = %68
  %85 = load i64, ptr %9, align 16, !noalias !6117
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !6117
  %86 = icmp eq i64 %85, -1
  br i1 %86, label %93, label %87

87:                                               ; preds = %84
  store ptr %65, ptr %44, align 8, !noalias !6117
  %88 = load i64, ptr %50, align 8, !noalias !6117
  %89 = load ptr, ptr %54, align 16, !noalias !6117
  %90 = load i64, ptr %55, align 8, !noalias !6117
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %7, ptr noundef nonnull align 16 dereferenceable(16) %56, i64 16, i1 false), !noalias !6117
  %91 = getelementptr inbounds nuw i8, ptr %9, i64 48
  %92 = getelementptr inbounds nuw i8, ptr %14, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %92, ptr noundef nonnull align 16 dereferenceable(48) %91, i64 48, i1 false), !noalias !6122
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !6117
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %14, ptr noundef nonnull align 8 dereferenceable(16) %7, i64 16, i1 false), !noalias !6122
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %10)
          to label %116 unwind label %75, !noalias !6123

93:                                               ; preds = %84, %70
  %94 = load i64, ptr %50, align 8, !noalias !6117
  %95 = load ptr, ptr %54, align 16, !noalias !6117
  %96 = load i64, ptr %55, align 8, !noalias !6117
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %7, ptr noundef nonnull align 16 dereferenceable(16) %56, i64 16, i1 false), !noalias !6117
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !6117
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6133)
  %97 = load i64, ptr %12, align 8, !range !1828, !alias.scope !6133, !noalias !6136, !noundef !1733
  %98 = icmp eq i64 %63, %97
  br i1 %98, label %99, label %108

99:                                               ; preds = %93
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %12)
          to label %100 unwind label %102, !noalias !6138

100:                                              ; preds = %99
  %101 = load ptr, ptr %37, align 8, !alias.scope !6133, !noalias !6136
  br label %108

102:                                              ; preds = %99
  %103 = landingpad { ptr, i32 }
          cleanup
  store ptr %65, ptr %44, align 8, !noalias !6117
  %104 = icmp ugt i64 %94, 5
  br i1 %104, label %105, label %59

105:                                              ; preds = %102
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %95) ]
  %106 = shl i64 %94, 3
  %107 = add i64 %106, -8
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %95, i64 noundef %107, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !6139
  br label %59

108:                                              ; preds = %100, %93
  %109 = phi ptr [ %101, %100 ], [ %62, %93 ]
  %110 = getelementptr inbounds nuw [40 x i8], ptr %109, i64 %63
  store i64 %94, ptr %110, align 8, !noalias !6142
  %111 = getelementptr inbounds nuw i8, ptr %110, i64 8
  store ptr %95, ptr %111, align 8, !noalias !6142
  %112 = getelementptr inbounds nuw i8, ptr %110, i64 16
  store i64 %96, ptr %112, align 8, !noalias !6123
  %113 = getelementptr inbounds nuw i8, ptr %110, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %113, ptr noundef nonnull align 8 dereferenceable(16) %7, i64 16, i1 false), !noalias !6123
  %114 = add i64 %63, 1
  store i64 %114, ptr %38, align 8, !alias.scope !6133, !noalias !6136
  %115 = icmp eq ptr %65, %43
  br i1 %115, label %.loopexit9, label %61

116:                                              ; preds = %87
  call void @llvm.lifetime.end.p0(ptr nonnull %10), !noalias !6117
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6143)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6146)
  %117 = icmp eq i64 %63, 0
  br i1 %117, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %116
  %118 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %119 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %120

120:                                              ; preds = %.preheader, %158
  %121 = phi i64 [ %123, %158 ], [ 0, %.preheader ]
  %122 = getelementptr inbounds nuw [40 x i8], ptr %62, i64 %121
  %123 = add nuw nsw i64 %121, 1
  %124 = load i64, ptr %122, align 8, !range !1771, !alias.scope !6149, !noalias !6152, !noundef !1733
  %125 = icmp ugt i64 %124, 5
  br i1 %125, label %126, label %158

126:                                              ; preds = %120
  %127 = getelementptr i8, ptr %122, i64 8
  %128 = load ptr, ptr %127, align 8, !alias.scope !6146, !noalias !6152, !nonnull !1733, !noundef !1733
  %129 = shl i64 %124, 3
  %130 = add i64 %129, -8
  %131 = load i64, ptr %118, align 8, !noalias !6153, !noundef !1733
  %132 = tail call i64 @llvm.umin.i64(i64 %130, i64 9223372036854775807)
  %133 = tail call i64 @llvm.ssub.sat.i64(i64 %131, i64 %132)
  store i64 %133, ptr %118, align 8, !noalias !6153
  %134 = load i64, ptr %119, align 8, !noalias !6153, !noundef !1733
  %135 = icmp slt i64 %133, %134
  br i1 %135, label %136, label %.preheader95

136:                                              ; preds = %126
  store i64 %133, ptr %119, align 8, !noalias !6153
  br label %.preheader95

.preheader95:                                     ; preds = %136, %126
  br label %137

137:                                              ; preds = %.preheader95, %140
  %138 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !6153
  %139 = icmp slt i64 %138, 0
  br i1 %139, label %140, label %__rustc::__rust_dealloc (.exit)

140:                                              ; preds = %137
  %141 = add nsw i64 %138, 1
  %142 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %138, i64 %141 acq_rel acquire, align 8, !noalias !6153
  %143 = extractvalue { i64, i1 } %142, 1
  br i1 %143, label %144, label %137

144:                                              ; preds = %140
  %145 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %132 monotonic, align 8, !noalias !6153
  %146 = tail call i64 @llvm.ssub.sat.i64(i64 %145, i64 %132)
  %147 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !6153
  br label %148

148:                                              ; preds = %151, %144
  %149 = phi i64 [ %147, %144 ], [ %154, %151 ]
  %150 = icmp slt i64 %146, %149
  br i1 %150, label %151, label %155

151:                                              ; preds = %148
  %152 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %149, i64 %146 monotonic monotonic, align 8, !noalias !6153
  %153 = extractvalue { i64, i1 } %152, 1
  %154 = extractvalue { i64, i1 } %152, 0
  br i1 %153, label %155, label %148

155:                                              ; preds = %151, %148
  %156 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !6153
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %137, %155
  %157 = icmp ne i64 %130, 0
  tail call void @llvm.assume(i1 %157), !noalias !6153
  tail call void @free(ptr noundef nonnull %128) #92, !noalias !6153
  br label %158

158:                                              ; preds = %__rustc::__rust_dealloc (.exit), %120
  %159 = icmp eq i64 %123, %63
  br i1 %159, label %.loopexit, label %120

.loopexit:                                        ; preds = %158, %116
  %160 = load i64, ptr %12, align 8, !alias.scope !6143, !noalias !6117
  %161 = icmp eq i64 %160, 0
  br i1 %161, label %198, label %162

162:                                              ; preds = %.loopexit
  %163 = mul nuw i64 %160, 40
  %164 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %165 = load i64, ptr %164, align 8, !noalias !6152, !noundef !1733
  %166 = tail call i64 @llvm.umin.i64(i64 %163, i64 9223372036854775807)
  %167 = tail call i64 @llvm.ssub.sat.i64(i64 %165, i64 %166)
  store i64 %167, ptr %164, align 8, !noalias !6152
  %168 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %169 = load i64, ptr %168, align 8, !noalias !6152, !noundef !1733
  %170 = icmp slt i64 %167, %169
  br i1 %170, label %171, label %.preheader94

171:                                              ; preds = %162
  store i64 %167, ptr %168, align 8, !noalias !6152
  br label %.preheader94

.preheader94:                                     ; preds = %171, %162
  br label %172

172:                                              ; preds = %.preheader94, %175
  %173 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !6152
  %174 = icmp slt i64 %173, 0
  br i1 %174, label %175, label %__rustc::__rust_dealloc (.exit8)

175:                                              ; preds = %172
  %176 = add nsw i64 %173, 1
  %177 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %173, i64 %176 acq_rel acquire, align 8, !noalias !6152
  %178 = extractvalue { i64, i1 } %177, 1
  br i1 %178, label %179, label %172

179:                                              ; preds = %175
  %180 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %166 monotonic, align 8, !noalias !6152
  %181 = tail call i64 @llvm.ssub.sat.i64(i64 %180, i64 %166)
  %182 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !6152
  br label %183

183:                                              ; preds = %186, %179
  %184 = phi i64 [ %182, %179 ], [ %189, %186 ]
  %185 = icmp slt i64 %181, %184
  br i1 %185, label %186, label %190

186:                                              ; preds = %183
  %187 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %184, i64 %181 monotonic monotonic, align 8, !noalias !6152
  %188 = extractvalue { i64, i1 } %187, 1
  %189 = extractvalue { i64, i1 } %187, 0
  br i1 %188, label %190, label %183

190:                                              ; preds = %186, %183
  %191 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !6152
  br label %__rustc::__rust_dealloc (.exit8)

__rustc::__rust_dealloc (.exit8): ; preds = %172, %190
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %62) ], !noalias !6152
  tail call void @free(ptr noundef nonnull %62) #92, !noalias !6152
  br label %198

192:                                              ; preds = %194, %59
  %193 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #90, !noalias !6123
  unreachable

194:                                              ; preds = %21
  %195 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %3) #89
          to label %211 unwind label %192, !noalias !6120

196:                                              ; preds = %34
  %197 = landingpad { ptr, i32 }
          cleanup
  br label %211

198:                                              ; preds = %__rustc::__rust_dealloc (.exit8), %.loopexit
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !6117
  br label %199

199:                                              ; preds = %198, %34
  %200 = phi i64 [ %85, %198 ], [ %26, %34 ]
  %201 = phi i64 [ %88, %198 ], [ %29, %34 ]
  %202 = phi ptr [ %89, %198 ], [ %31, %34 ]
  %203 = phi i64 [ %90, %198 ], [ %33, %34 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %7)
  call void @llvm.lifetime.end.p0(ptr nonnull %13)
  %204 = getelementptr inbounds nuw i8, ptr %0, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(64) %204, ptr noundef nonnull align 8 dereferenceable(64) %14, i64 64, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %14)
  store i64 %200, ptr %0, align 16
  %205 = getelementptr inbounds nuw i8, ptr %0, i64 8
  store i64 %201, ptr %205, align 8
  %206 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store ptr %202, ptr %206, align 16
  %207 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store i64 %203, ptr %207, align 8
  br label %209

208:                                              ; preds = %211
  br i1 %19, label %218, label %217

209:                                              ; preds = %199, %77
; call core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  tail call fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(208) %4)
  ret void

210:                                              ; preds = %20
  unreachable

211:                                              ; preds = %213, %196, %194, %73
  %212 = phi { ptr, i32 } [ %197, %196 ], [ %214, %213 ], [ %74, %73 ], [ %195, %194 ]
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(208) %4) #89
          to label %208 unwind label %215

213:                                              ; preds = %20
  %214 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  tail call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %5) #89
  br label %211

215:                                              ; preds = %218, %211
  %216 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #90
  unreachable

217:                                              ; preds = %218, %208
  resume { ptr, i32 } %212

218:                                              ; preds = %208
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(24) %3) #89
          to label %217 unwind label %215
}
define internal fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr dead_on_unwind noalias nofree noundef nonnull writable writeonly align 16 captures(none) dereferenceable(96) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(200) %1, ptr noalias nofree noundef nonnull align 16 dereferenceable(1232) %2, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(24) %3, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(208) %4) unnamed_addr #0 personality ptr @rust_eh_personality !guid !6156 {
  %6 = alloca [24 x i8], align 8
  %7 = alloca [24 x i8], align 8
  %8 = alloca [24 x i8], align 8
  %9 = alloca [24 x i8], align 8
  %10 = alloca [24 x i8], align 8
  %11 = alloca [23 x i8], align 1
  %12 = alloca [24 x i8], align 8
  %13 = alloca [80 x i8], align 8
  %14 = alloca [24 x i8], align 8
  %15 = alloca [24 x i8], align 8
  %16 = alloca [80 x i8], align 8
  %17 = alloca [80 x i8], align 8
  %18 = alloca [24 x i8], align 8
  %19 = alloca [80 x i8], align 8
  %20 = alloca [80 x i8], align 8
  %21 = alloca [24 x i8], align 8
  %22 = alloca [24 x i8], align 8
  %23 = alloca [80 x i8], align 8
  %24 = alloca [24 x i8], align 8
  %25 = alloca [24 x i8], align 8
  %26 = alloca [32 x i8], align 8
  %27 = alloca [8 x i8], align 8
  %28 = alloca [8 x i8], align 8
  %29 = alloca [24 x i8], align 8
  %30 = alloca [24 x i8], align 8
  %31 = alloca [24 x i8], align 8
  %32 = alloca [24 x i8], align 8
  %33 = alloca [24 x i8], align 8
  %34 = alloca [24 x i8], align 8
  %35 = alloca [24 x i8], align 8
  %36 = alloca [32 x i8], align 8
  %37 = alloca [160 x i8], align 8
  %38 = alloca [32 x i8], align 8
  %39 = alloca [8 x i8], align 8
  %40 = alloca [24 x i8], align 8
  %41 = alloca [32 x i8], align 8
  %42 = alloca [32 x i8], align 8
  %43 = alloca [24 x i8], align 8
  %44 = alloca [96 x i8], align 16
  %45 = alloca [24 x i8], align 8
  %46 = alloca [24 x i8], align 8
  %47 = alloca [160 x i8], align 8
  %48 = alloca [32 x i8], align 8
  %49 = alloca [24 x i8], align 8
  %50 = alloca [24 x i8], align 8
  %51 = alloca [32 x i8], align 8
  %52 = alloca [24 x i8], align 8
  %53 = alloca [63 x i8], align 1
  %54 = alloca [24 x i8], align 8
  %55 = alloca [224 x i8], align 8
  %56 = alloca [24 x i8], align 8
  %57 = getelementptr inbounds nuw i8, ptr %1, i64 194
  %58 = load i8, ptr %57, align 2, !range !3719, !noundef !1733
  %59 = icmp ne i8 %58, 2
  %60 = getelementptr inbounds nuw i8, ptr %2, i64 608
  %61 = load ptr, ptr %60, align 16
  %62 = icmp eq ptr %61, null
  %63 = select i1 %59, i1 true, i1 %62
  br i1 %63, label %64, label %67

64:                                               ; preds = %5
  %65 = getelementptr inbounds nuw i8, ptr %0, i64 8
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %65, ptr noundef nonnull align 8 dereferenceable(24) %3, i64 24, i1 false)
  %66 = getelementptr inbounds nuw i8, ptr %0, i64 32
  store i64 0, ptr %66, align 16
  store i64 -1, ptr %0, align 16
; call core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  tail call fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(208) %4)
  br label %1657

67:                                               ; preds = %5
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6157)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6160)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6162)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6164)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6166)
  call void @llvm.lifetime.start.p0(ptr nonnull %56), !noalias !6168
  call void @llvm.lifetime.start.p0(ptr nonnull %55), !noalias !6168
  %68 = load i64, ptr %4, align 8, !alias.scope !6166, !noalias !6169
  %69 = getelementptr inbounds nuw i8, ptr %4, i64 8
  %70 = load i64, ptr %69, align 8, !alias.scope !6166, !noalias !6169
  %71 = getelementptr inbounds nuw i8, ptr %4, i64 16
  %72 = load i64, ptr %71, align 8, !alias.scope !6166, !noalias !6169
  %73 = getelementptr inbounds nuw i8, ptr %4, i64 24
  %74 = getelementptr inbounds nuw i8, ptr %55, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(184) %74, ptr noundef nonnull readonly align 8 dereferenceable(184) %73, i64 184, i1 false), !noalias !6169
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6170)
  %75 = icmp ugt i64 %68, 2
  %76 = select i1 %75, i64 %72, i64 %68
  %77 = add i64 %76, -1
  %78 = select i1 %75, i64 %68, i64 1
  %79 = select i1 %75, i64 1, i64 %72
  store i64 %78, ptr %55, align 8, !alias.scope !6173, !noalias !6168
  %80 = getelementptr inbounds nuw i8, ptr %55, i64 8
  store i64 %70, ptr %80, align 8, !alias.scope !6173, !noalias !6168
  %81 = getelementptr inbounds nuw i8, ptr %55, i64 16
  store i64 %79, ptr %81, align 8, !alias.scope !6173, !noalias !6168
  %82 = getelementptr inbounds nuw i8, ptr %55, i64 208
  store i64 0, ptr %82, align 8, !alias.scope !6175, !noalias !6176
  %83 = getelementptr inbounds nuw i8, ptr %55, i64 216
  store i64 %77, ptr %83, align 8, !alias.scope !6175, !noalias !6176
; invoke <purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
  invoke fastcc void @<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %56, ptr noalias nofree noundef align 8 captures(address) dereferenceable(224) %55)
          to label %86 unwind label %84, !noalias !6177

84:                                               ; preds = %67
  %85 = landingpad { ptr, i32 }
          cleanup
  br label %1655

86:                                               ; preds = %67
  call void @llvm.lifetime.end.p0(ptr nonnull %55), !noalias !6168
  %87 = getelementptr inbounds nuw i8, ptr %56, i64 8
  %88 = load ptr, ptr %87, align 8, !noalias !6168, !nonnull !1733, !noundef !1733
  %89 = getelementptr inbounds nuw i8, ptr %56, i64 16
  %90 = load i64, ptr %89, align 8, !noalias !6168, !noundef !1733
  %91 = mul nuw nsw i64 %90, 200
  %92 = getelementptr inbounds nuw i8, ptr %88, i64 %91
  %93 = icmp eq i64 %90, 0
  br i1 %93, label %162, label %.preheader193.preheader

.preheader193.preheader:                          ; preds = %86
  %xtraiter = and i64 %90, 3
  %94 = icmp ult i64 %90, 4
  br i1 %94, label %.preheader193.epil.preheader, label %.preheader193.preheader.new

.preheader193.preheader.new:                      ; preds = %.preheader193.preheader
  %unroll_iter = and i64 %90, -4
  br label %.preheader193

.preheader193:                                    ; preds = %139, %.preheader193.preheader.new
  %95 = phi i64 [ 0, %.preheader193.preheader.new ], [ %142, %139 ]
  %96 = phi i64 [ 0, %.preheader193.preheader.new ], [ %141, %139 ]
  %niter = phi i64 [ 0, %.preheader193.preheader.new ], [ %niter.next.3, %139 ]
  %97 = getelementptr inbounds nuw [200 x i8], ptr %88, i64 %95
  %98 = getelementptr i8, ptr %97, i64 168
  %99 = load i64, ptr %98, align 8, !noalias !6177, !noundef !1733
  %100 = getelementptr i8, ptr %97, i64 176
  %101 = load i64, ptr %100, align 8, !noalias !6177, !noundef !1733
  %102 = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %101, i64 %99)
  %103 = extractvalue { i64, i1 } %102, 0
  %104 = extractvalue { i64, i1 } %102, 1
  br i1 %104, label %105, label %.preheader193.1, !prof !1735

105:                                              ; preds = %.preheader193
  br label %.preheader193.1

.preheader193.1:                                  ; preds = %105, %.preheader193
  %106 = phi i64 [ -1, %105 ], [ %103, %.preheader193 ]
  %107 = tail call noundef i64 @llvm.uadd.sat.i64(i64 %96, i64 %106)
  %108 = getelementptr inbounds nuw [200 x i8], ptr %88, i64 %95
  %109 = getelementptr i8, ptr %108, i64 368
  %110 = load i64, ptr %109, align 8, !noalias !6177, !noundef !1733
  %111 = getelementptr i8, ptr %108, i64 376
  %112 = load i64, ptr %111, align 8, !noalias !6177, !noundef !1733
  %113 = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %112, i64 %110)
  %114 = extractvalue { i64, i1 } %113, 0
  %115 = extractvalue { i64, i1 } %113, 1
  br i1 %115, label %116, label %.preheader193.2, !prof !1735

116:                                              ; preds = %.preheader193.1
  br label %.preheader193.2

.preheader193.2:                                  ; preds = %116, %.preheader193.1
  %117 = phi i64 [ -1, %116 ], [ %114, %.preheader193.1 ]
  %118 = tail call noundef i64 @llvm.uadd.sat.i64(i64 %107, i64 %117)
  %119 = getelementptr inbounds nuw [200 x i8], ptr %88, i64 %95
  %120 = getelementptr i8, ptr %119, i64 568
  %121 = load i64, ptr %120, align 8, !noalias !6177, !noundef !1733
  %122 = getelementptr i8, ptr %119, i64 576
  %123 = load i64, ptr %122, align 8, !noalias !6177, !noundef !1733
  %124 = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %123, i64 %121)
  %125 = extractvalue { i64, i1 } %124, 0
  %126 = extractvalue { i64, i1 } %124, 1
  br i1 %126, label %127, label %.preheader193.3, !prof !1735

127:                                              ; preds = %.preheader193.2
  br label %.preheader193.3

.preheader193.3:                                  ; preds = %127, %.preheader193.2
  %128 = phi i64 [ -1, %127 ], [ %125, %.preheader193.2 ]
  %129 = tail call noundef i64 @llvm.uadd.sat.i64(i64 %118, i64 %128)
  %130 = getelementptr inbounds nuw [200 x i8], ptr %88, i64 %95
  %131 = getelementptr i8, ptr %130, i64 768
  %132 = load i64, ptr %131, align 8, !noalias !6177, !noundef !1733
  %133 = getelementptr i8, ptr %130, i64 776
  %134 = load i64, ptr %133, align 8, !noalias !6177, !noundef !1733
  %135 = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %134, i64 %132)
  %136 = extractvalue { i64, i1 } %135, 0
  %137 = extractvalue { i64, i1 } %135, 1
  br i1 %137, label %138, label %139, !prof !1735

138:                                              ; preds = %.preheader193.3
  br label %139

139:                                              ; preds = %138, %.preheader193.3
  %140 = phi i64 [ -1, %138 ], [ %136, %.preheader193.3 ]
  %141 = tail call noundef i64 @llvm.uadd.sat.i64(i64 %129, i64 %140)
  %142 = add nuw i64 %95, 4
  %niter.next.3 = add i64 %niter, 4
  %niter.ncmp.3 = icmp eq i64 %niter.next.3, %unroll_iter
  br i1 %niter.ncmp.3, label %.unr-lcssa, label %.preheader193

.unr-lcssa:                                       ; preds = %139
  %lcmp.mod.not = icmp eq i64 %xtraiter, 0
  br i1 %lcmp.mod.not, label %.epilog-lcssa, label %.preheader193.epil.preheader

.preheader193.epil.preheader:                     ; preds = %.unr-lcssa, %.preheader193.preheader
  %.epil.init = phi i64 [ 0, %.preheader193.preheader ], [ %142, %.unr-lcssa ]
  %.epil.init1745 = phi i64 [ 0, %.preheader193.preheader ], [ %141, %.unr-lcssa ]
  %lcmp.mod1747 = icmp ne i64 %xtraiter, 0
  tail call void @llvm.assume(i1 %lcmp.mod1747)
  br label %.preheader193.epil

.preheader193.epil:                               ; preds = %154, %.preheader193.epil.preheader
  %143 = phi i64 [ %157, %154 ], [ %.epil.init, %.preheader193.epil.preheader ]
  %144 = phi i64 [ %156, %154 ], [ %.epil.init1745, %.preheader193.epil.preheader ]
  %epil.iter = phi i64 [ %epil.iter.next, %154 ], [ 0, %.preheader193.epil.preheader ]
  %145 = getelementptr inbounds nuw [200 x i8], ptr %88, i64 %143
  %146 = getelementptr i8, ptr %145, i64 168
  %147 = load i64, ptr %146, align 8, !noalias !6177, !noundef !1733
  %148 = getelementptr i8, ptr %145, i64 176
  %149 = load i64, ptr %148, align 8, !noalias !6177, !noundef !1733
  %150 = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %149, i64 %147)
  %151 = extractvalue { i64, i1 } %150, 0
  %152 = extractvalue { i64, i1 } %150, 1
  br i1 %152, label %153, label %154, !prof !1735

153:                                              ; preds = %.preheader193.epil
  br label %154

154:                                              ; preds = %153, %.preheader193.epil
  %155 = phi i64 [ -1, %153 ], [ %151, %.preheader193.epil ]
  %156 = tail call noundef i64 @llvm.uadd.sat.i64(i64 %144, i64 %155)
  %157 = add nuw i64 %143, 1
  %epil.iter.next = add i64 %epil.iter, 1
  %epil.iter.cmp.not = icmp eq i64 %epil.iter.next, %xtraiter
  br i1 %epil.iter.cmp.not, label %.epilog-lcssa, label %.preheader193.epil, !llvm.loop !6178

.epilog-lcssa:                                    ; preds = %154, %.unr-lcssa
  %.lcssa1743 = phi i64 [ %141, %.unr-lcssa ], [ %156, %154 ]
  %158 = load ptr, ptr %60, align 16, !alias.scope !6162, !noalias !6177, !noundef !1733
  %159 = icmp eq ptr %158, null
  %160 = icmp eq i64 %.lcssa1743, 0
  %161 = or i1 %160, %159
  br i1 %161, label %162, label %214

162:                                              ; preds = %218, %214, %.epilog-lcssa, %86
  %163 = load i64, ptr %56, align 8, !range !1828, !noalias !6168, !noundef !1733
  %164 = icmp ult i64 %90, 46116860184273880
  tail call void @llvm.assume(i1 %164)
  call void @llvm.lifetime.start.p0(ptr nonnull %49), !noalias !6179
  store i64 0, ptr %49, align 8, !noalias !6179
  %165 = getelementptr inbounds nuw i8, ptr %49, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %165, align 8, !noalias !6179
  %166 = getelementptr inbounds nuw i8, ptr %49, i64 16
  store i64 0, ptr %166, align 8, !noalias !6179
  call void @llvm.lifetime.start.p0(ptr nonnull %48), !noalias !6179
  store ptr %88, ptr %48, align 8, !noalias !6183
  %167 = getelementptr inbounds nuw i8, ptr %48, i64 8
  %168 = getelementptr inbounds nuw i8, ptr %48, i64 16
  store i64 %163, ptr %168, align 8, !noalias !6183
  %169 = getelementptr inbounds nuw i8, ptr %48, i64 24
  store ptr %92, ptr %169, align 8, !noalias !6183
  br i1 %93, label %.loopexit189, label %170

170:                                              ; preds = %162
  %171 = getelementptr inbounds nuw i8, ptr %47, i64 8
  %172 = getelementptr inbounds nuw i8, ptr %47, i64 152
  br label %177

173:                                              ; preds = %184, %175
  %174 = phi { ptr, i32 } [ %176, %175 ], [ %194, %184 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(24) %49) #89
          to label %1655 unwind label %212, !noalias !6184

175:                                              ; preds = %200
  %176 = landingpad { ptr, i32 }
          cleanup
  br label %173

177:                                              ; preds = %209, %170
  %178 = phi ptr [ inttoptr (i64 8 to ptr), %170 ], [ %205, %209 ]
  %179 = phi i64 [ 0, %170 ], [ %207, %209 ]
  %180 = phi ptr [ %88, %170 ], [ %181, %209 ]
  %181 = getelementptr inbounds nuw i8, ptr %180, i64 200
  %182 = load i64, ptr %180, align 8, !noalias !6185
  %183 = icmp eq i64 %182, -1
  br i1 %183, label %.loopexit189, label %185

184:                                              ; preds = %193
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %48)
          to label %173 unwind label %212, !noalias !6184

185:                                              ; preds = %177
  %186 = getelementptr inbounds nuw i8, ptr %180, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %47), !noalias !6179
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(152) %171, ptr noundef nonnull align 8 dereferenceable(152) %186, i64 152, i1 false), !noalias !6184
  store i64 %182, ptr %47, align 8, !noalias !6179
  %187 = load i8, ptr %172, align 8, !range !1740, !noalias !6179, !noundef !1733
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6191)
  %188 = load i64, ptr %49, align 8, !range !1828, !alias.scope !6191, !noalias !6194, !noundef !1733
  %189 = icmp eq i64 %179, %188
  br i1 %189, label %190, label %204

190:                                              ; preds = %185
; invoke <alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %49)
          to label %191 unwind label %193, !noalias !6196

191:                                              ; preds = %190
  %192 = load ptr, ptr %165, align 8, !alias.scope !6191, !noalias !6194
  br label %204

193:                                              ; preds = %190
  %194 = landingpad { ptr, i32 }
          cleanup
  store ptr %181, ptr %167, align 8, !noalias !6179
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(160) %47) #89
          to label %184 unwind label %195, !noalias !6197

195:                                              ; preds = %193
  %196 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #90, !noalias !6198
  unreachable

.loopexit189:                                     ; preds = %209, %177, %162
  %197 = phi i64 [ 0, %162 ], [ %207, %209 ], [ %179, %177 ]
  %198 = phi ptr [ inttoptr (i64 8 to ptr), %162 ], [ %205, %209 ], [ %178, %177 ]
  %199 = phi ptr [ %88, %162 ], [ %92, %209 ], [ %181, %177 ]
  store ptr %199, ptr %167, align 8, !noalias !6179
  br label %200

200:                                              ; preds = %211, %.loopexit189
  %201 = phi i64 [ %207, %211 ], [ %197, %.loopexit189 ]
  %202 = phi ptr [ %205, %211 ], [ %198, %.loopexit189 ]
  %203 = phi i8 [ 1, %211 ], [ 0, %.loopexit189 ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %48)
          to label %221 unwind label %175, !noalias !6184

204:                                              ; preds = %191, %185
  %205 = phi ptr [ %192, %191 ], [ %178, %185 ]
  %206 = getelementptr inbounds nuw [160 x i8], ptr %205, i64 %179
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(160) %206, ptr noundef nonnull readonly align 8 dereferenceable(160) %47, i64 160, i1 false), !noalias !6197
  %207 = add nuw nsw i64 %179, 1
  store i64 %207, ptr %166, align 8, !alias.scope !6191, !noalias !6194
  %208 = trunc nuw i8 %187 to i1
  br i1 %208, label %211, label %209

209:                                              ; preds = %204
  call void @llvm.lifetime.end.p0(ptr nonnull %47), !noalias !6179
  %210 = icmp eq ptr %181, %92
  br i1 %210, label %.loopexit189, label %177

211:                                              ; preds = %204
  store ptr %181, ptr %167, align 8, !noalias !6179
  call void @llvm.lifetime.end.p0(ptr nonnull %47), !noalias !6179
  br label %200

212:                                              ; preds = %184, %173
  %213 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #90, !noalias !6184
  unreachable

214:                                              ; preds = %.epilog-lcssa
  %215 = getelementptr inbounds nuw i8, ptr %158, i64 336
  %216 = load ptr, ptr %215, align 8, !noalias !6177, !noundef !1733
  %217 = icmp eq ptr %216, null
  br i1 %217, label %162, label %218

218:                                              ; preds = %214
  %219 = getelementptr inbounds nuw i8, ptr %158, i64 352
  %220 = atomicrmw add ptr %219, i64 %.lcssa1743 monotonic, align 8, !noalias !6177
  br label %162

221:                                              ; preds = %200
  call void @llvm.lifetime.end.p0(ptr nonnull %48), !noalias !6179
  %222 = load i64, ptr %49, align 8, !noalias !6199
  call void @llvm.lifetime.end.p0(ptr nonnull %49), !noalias !6179
  %223 = getelementptr inbounds nuw i8, ptr %1, i64 193
  %224 = load i8, ptr %223, align 1, !range !1740, !alias.scope !6160, !noalias !6200, !noundef !1733
  %225 = trunc nuw i8 %224 to i1
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %202) ]
  %226 = icmp ne i64 %201, 0
  br i1 %226, label %iter.check, label %.loopexit188

iter.check:                                       ; preds = %221
  %min.iters.check = icmp ult i64 %201, 8
  br i1 %min.iters.check, label %.preheader187.preheader, label %vector.main.loop.iter.check

vector.main.loop.iter.check:                      ; preds = %iter.check
  %min.iters.check1380 = icmp ult i64 %201, 32
  br i1 %min.iters.check1380, label %vec.epilog.ph, label %vector.ph

vector.ph:                                        ; preds = %vector.main.loop.iter.check
  %n.mod.vf = and i64 %201, 24
  %n.vec = and i64 %201, -32
  br label %vector.body

vector.body:                                      ; preds = %vector.body, %vector.ph
  %index = phi i64 [ 0, %vector.ph ], [ %index.next, %vector.body ]
  %vec.ind = phi <8 x i64> [ <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>, %vector.ph ], [ %vec.ind.next, %vector.body ]
  %vec.phi = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %227, %vector.body ]
  %vec.phi1381 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %228, %vector.body ]
  %vec.phi1382 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %229, %vector.body ]
  %vec.phi1383 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %230, %vector.body ]
  %step.add = add nuw <8 x i64> %vec.ind, splat (i64 8)
  %step.add.2 = add nuw <8 x i64> %vec.ind, splat (i64 16)
  %step.add.3 = add nuw <8 x i64> %vec.ind, splat (i64 24)
  %wide.gep = getelementptr inbounds nuw [160 x i8], ptr %202, <8 x i64> %vec.ind
  %wide.gep1384 = getelementptr inbounds nuw [160 x i8], ptr %202, <8 x i64> %step.add
  %wide.gep1385 = getelementptr inbounds nuw [160 x i8], ptr %202, <8 x i64> %step.add.2
  %wide.gep1386 = getelementptr inbounds nuw [160 x i8], ptr %202, <8 x i64> %step.add.3
  %wide.gep1387 = getelementptr i8, <8 x ptr> %wide.gep, i64 64
  %wide.gep1388 = getelementptr i8, <8 x ptr> %wide.gep1384, i64 64
  %wide.gep1389 = getelementptr i8, <8 x ptr> %wide.gep1385, i64 64
  %wide.gep1390 = getelementptr i8, <8 x ptr> %wide.gep1386, i64 64
  %wide.masked.gather = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1387, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !6201
  %wide.masked.gather1391 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1388, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !6201
  %wide.masked.gather1392 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1389, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !6201
  %wide.masked.gather1393 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1390, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !6201
  %227 = add <8 x i64> %wide.masked.gather, %vec.phi
  %228 = add <8 x i64> %wide.masked.gather1391, %vec.phi1381
  %229 = add <8 x i64> %wide.masked.gather1392, %vec.phi1382
  %230 = add <8 x i64> %wide.masked.gather1393, %vec.phi1383
  %index.next = add nuw i64 %index, 32
  %vec.ind.next = add nuw <8 x i64> %vec.ind, splat (i64 32)
  %231 = icmp eq i64 %index.next, %n.vec
  br i1 %231, label %middle.block, label %vector.body, !llvm.loop !6204

middle.block:                                     ; preds = %vector.body
  %bin.rdx = add <8 x i64> %228, %227
  %bin.rdx1394 = add <8 x i64> %229, %bin.rdx
  %bin.rdx1395 = add <8 x i64> %230, %bin.rdx1394
  %232 = tail call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %bin.rdx1395)
  %cmp.n = icmp eq i64 %201, %n.vec
  br i1 %cmp.n, label %.loopexit188, label %vec.epilog.iter.check

vec.epilog.iter.check:                            ; preds = %middle.block
  %min.epilog.iters.check = icmp eq i64 %n.mod.vf, 0
  br i1 %min.epilog.iters.check, label %.preheader187.preheader, label %vec.epilog.ph, !prof !6205

vec.epilog.ph:                                    ; preds = %vector.main.loop.iter.check, %vec.epilog.iter.check
  %vec.epilog.resume.val = phi i64 [ %n.vec, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %bc.merge.rdx = phi i64 [ %232, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %n.vec1397 = and i64 %201, -8
  %233 = insertelement <8 x i64> <i64 poison, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0>, i64 %bc.merge.rdx, i64 0
  %broadcast.splatinsert = insertelement <8 x i64> poison, i64 %vec.epilog.resume.val, i64 0
  %broadcast.splat = shufflevector <8 x i64> %broadcast.splatinsert, <8 x i64> poison, <8 x i32> zeroinitializer
  %induction = or disjoint <8 x i64> %broadcast.splat, <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>
  br label %vec.epilog.vector.body

vec.epilog.vector.body:                           ; preds = %vec.epilog.vector.body, %vec.epilog.ph
  %index1398 = phi i64 [ %vec.epilog.resume.val, %vec.epilog.ph ], [ %index.next1404, %vec.epilog.vector.body ]
  %vec.ind1399 = phi <8 x i64> [ %induction, %vec.epilog.ph ], [ %vec.ind.next1405, %vec.epilog.vector.body ]
  %vec.phi1400 = phi <8 x i64> [ %233, %vec.epilog.ph ], [ %234, %vec.epilog.vector.body ]
  %wide.gep1401 = getelementptr inbounds nuw [160 x i8], ptr %202, <8 x i64> %vec.ind1399
  %wide.gep1402 = getelementptr i8, <8 x ptr> %wide.gep1401, i64 64
  %wide.masked.gather1403 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1402, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !6201
  %234 = add <8 x i64> %wide.masked.gather1403, %vec.phi1400
  %index.next1404 = add nuw i64 %index1398, 8
  %vec.ind.next1405 = add nuw <8 x i64> %vec.ind1399, splat (i64 8)
  %235 = icmp eq i64 %index.next1404, %n.vec1397
  br i1 %235, label %vec.epilog.middle.block, label %vec.epilog.vector.body, !llvm.loop !6206

vec.epilog.middle.block:                          ; preds = %vec.epilog.vector.body
  %236 = tail call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %234)
  %cmp.n1406 = icmp eq i64 %201, %n.vec1397
  br i1 %cmp.n1406, label %.loopexit188, label %.preheader187.preheader

.preheader187.preheader:                          ; preds = %iter.check, %vec.epilog.iter.check, %vec.epilog.middle.block
  %.ph1727 = phi i64 [ 0, %iter.check ], [ %n.vec, %vec.epilog.iter.check ], [ %n.vec1397, %vec.epilog.middle.block ]
  %.ph1728 = phi i64 [ 0, %iter.check ], [ %232, %vec.epilog.iter.check ], [ %236, %vec.epilog.middle.block ]
  br label %.preheader187

.preheader187:                                    ; preds = %.preheader187.preheader, %.preheader187
  %237 = phi i64 [ %244, %.preheader187 ], [ %.ph1727, %.preheader187.preheader ]
  %238 = phi i64 [ %243, %.preheader187 ], [ %.ph1728, %.preheader187.preheader ]
  %239 = getelementptr inbounds nuw [160 x i8], ptr %202, i64 %237
  %240 = getelementptr i8, ptr %239, i64 64
  %241 = load i64, ptr %240, align 8, !noalias !6201, !noundef !1733
  %242 = icmp ult i64 %241, 288230376151711744
  tail call void @llvm.assume(i1 %242)
  %243 = add i64 %241, %238
  %244 = add nuw i64 %237, 1
  %245 = icmp eq i64 %244, %201
  br i1 %245, label %.loopexit188, label %.preheader187, !llvm.loop !6207

.loopexit188:                                     ; preds = %.preheader187, %middle.block, %vec.epilog.middle.block, %221
  %246 = phi i64 [ 0, %221 ], [ %236, %vec.epilog.middle.block ], [ %232, %middle.block ], [ %243, %.preheader187 ]
  %247 = trunc nuw i8 %203 to i1
  call void @llvm.lifetime.start.p0(ptr nonnull %54)
  call void @llvm.lifetime.start.p0(ptr nonnull %52)
  call void @llvm.lifetime.start.p0(ptr nonnull %53)
  %248 = getelementptr inbounds nuw i8, ptr %1, i64 195
  %249 = load i8, ptr %248, align 1, !range !6208, !alias.scope !6160, !noalias !6200, !noundef !1733
  call void @llvm.lifetime.start.p0(ptr nonnull %51), !noalias !6168
  store i64 %222, ptr %51, align 8, !noalias !6168
  %250 = getelementptr inbounds nuw i8, ptr %51, i64 8
  store ptr %202, ptr %250, align 8, !noalias !6168
  %251 = getelementptr inbounds nuw i8, ptr %51, i64 16
  store i64 %201, ptr %251, align 8, !noalias !6168
  %252 = getelementptr inbounds nuw i8, ptr %51, i64 24
  store i8 %203, ptr %252, align 8, !noalias !6168
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6209)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6212)
  call void @llvm.lifetime.start.p0(ptr nonnull %29)
  call void @llvm.lifetime.start.p0(ptr nonnull %40)
  call void @llvm.lifetime.start.p0(ptr nonnull %46), !noalias !6214
  call void @llvm.lifetime.start.p0(ptr nonnull %45)
  call void @llvm.lifetime.start.p0(ptr nonnull %44), !noalias !6214
  call void @llvm.lifetime.start.p0(ptr nonnull %43), !noalias !6214
  store i64 0, ptr %43, align 8, !noalias !6214
  %253 = getelementptr inbounds nuw i8, ptr %43, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %253, align 8, !noalias !6214
  %254 = getelementptr inbounds nuw i8, ptr %43, i64 16
  store i64 0, ptr %254, align 8, !noalias !6214
  %255 = getelementptr inbounds nuw i8, ptr %3, i64 16
  %256 = load i64, ptr %255, align 8, !alias.scope !6217, !noalias !6218, !noundef !1733
  %257 = icmp ult i64 %256, 230584300921369396
  tail call void @llvm.assume(i1 %257)
; invoke purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
  invoke fastcc void @purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %44, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %43, i64 noundef %256)
          to label %258 unwind label %1565, !noalias !6219

258:                                              ; preds = %.loopexit188
  call void @llvm.lifetime.end.p0(ptr nonnull %43), !noalias !6214
  %259 = load i64, ptr %44, align 16, !range !2520, !noalias !6214, !noundef !1733
  %260 = icmp eq i64 %259, -1
  %261 = getelementptr inbounds nuw i8, ptr %44, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %45, ptr noundef nonnull align 8 dereferenceable(24) %261, i64 24, i1 false), !noalias !6214
  br i1 %260, label %343, label %262

262:                                              ; preds = %258
  %263 = getelementptr inbounds nuw i8, ptr %44, i64 32
  %264 = load i8, ptr %263, align 16, !noalias !6220
  %265 = getelementptr inbounds nuw i8, ptr %44, i64 33
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(63) %53, ptr noundef nonnull align 1 dereferenceable(63) %265, i64 63, i1 false), !noalias !6220
  call void @llvm.lifetime.end.p0(ptr nonnull %44), !noalias !6214
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %52, ptr noundef nonnull align 8 dereferenceable(24) %45, i64 24, i1 false), !noalias !6220
  call void @llvm.lifetime.end.p0(ptr nonnull %45)
  call void @llvm.lifetime.end.p0(ptr nonnull %46), !noalias !6214
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6221)
  %266 = getelementptr inbounds nuw i8, ptr %3, i64 8
  %267 = load ptr, ptr %266, align 8, !alias.scope !6224, !noalias !6218, !nonnull !1733, !noundef !1733
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6225)
  %268 = icmp eq i64 %256, 0
  br i1 %268, label %.loopexit186, label %.preheader185

.preheader185:                                    ; preds = %262
  %269 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %270 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %271

271:                                              ; preds = %.preheader185, %309
  %272 = phi i64 [ %274, %309 ], [ 0, %.preheader185 ]
  %273 = getelementptr inbounds nuw [40 x i8], ptr %267, i64 %272
  %274 = add nuw nsw i64 %272, 1
  %275 = load i64, ptr %273, align 8, !range !1771, !alias.scope !6228, !noalias !6231, !noundef !1733
  %276 = icmp ugt i64 %275, 5
  br i1 %276, label %277, label %309

277:                                              ; preds = %271
  %278 = getelementptr i8, ptr %273, i64 8
  %279 = load ptr, ptr %278, align 8, !alias.scope !6225, !noalias !6231, !nonnull !1733, !noundef !1733
  %280 = shl i64 %275, 3
  %281 = add i64 %280, -8
  %282 = load i64, ptr %269, align 8, !noalias !6232, !noundef !1733
  %283 = tail call i64 @llvm.umin.i64(i64 %281, i64 9223372036854775807)
  %284 = tail call i64 @llvm.ssub.sat.i64(i64 %282, i64 %283)
  store i64 %284, ptr %269, align 8, !noalias !6232
  %285 = load i64, ptr %270, align 8, !noalias !6232, !noundef !1733
  %286 = icmp slt i64 %284, %285
  br i1 %286, label %287, label %.preheader1726

287:                                              ; preds = %277
  store i64 %284, ptr %270, align 8, !noalias !6232
  br label %.preheader1726

.preheader1726:                                   ; preds = %287, %277
  br label %288

288:                                              ; preds = %.preheader1726, %291
  %289 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !6232
  %290 = icmp slt i64 %289, 0
  br i1 %290, label %291, label %__rustc::__rust_dealloc (.exit)

291:                                              ; preds = %288
  %292 = add nsw i64 %289, 1
  %293 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %289, i64 %292 acq_rel acquire, align 8, !noalias !6232
  %294 = extractvalue { i64, i1 } %293, 1
  br i1 %294, label %295, label %288

295:                                              ; preds = %291
  %296 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %283 monotonic, align 8, !noalias !6232
  %297 = tail call i64 @llvm.ssub.sat.i64(i64 %296, i64 %283)
  %298 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !6232
  br label %299

299:                                              ; preds = %302, %295
  %300 = phi i64 [ %298, %295 ], [ %305, %302 ]
  %301 = icmp slt i64 %297, %300
  br i1 %301, label %302, label %306

302:                                              ; preds = %299
  %303 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %300, i64 %297 monotonic monotonic, align 8, !noalias !6232
  %304 = extractvalue { i64, i1 } %303, 1
  %305 = extractvalue { i64, i1 } %303, 0
  br i1 %304, label %306, label %299

306:                                              ; preds = %302, %299
  %307 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !6232
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %288, %306
  %308 = icmp ne i64 %281, 0
  tail call void @llvm.assume(i1 %308), !noalias !6232
  tail call void @free(ptr noundef nonnull %279) #92, !noalias !6232
  br label %309

309:                                              ; preds = %__rustc::__rust_dealloc (.exit), %271
  %310 = icmp eq i64 %274, %256
  br i1 %310, label %.loopexit186, label %271

.loopexit186:                                     ; preds = %309, %262
  %311 = load i64, ptr %3, align 8, !alias.scope !6224, !noalias !6218
  %312 = icmp eq i64 %311, 0
  br i1 %312, label %1512, label %313

313:                                              ; preds = %.loopexit186
  %314 = mul nuw i64 %311, 40
  %315 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %316 = load i64, ptr %315, align 8, !noalias !6231, !noundef !1733
  %317 = tail call i64 @llvm.umin.i64(i64 %314, i64 9223372036854775807)
  %318 = tail call i64 @llvm.ssub.sat.i64(i64 %316, i64 %317)
  store i64 %318, ptr %315, align 8, !noalias !6231
  %319 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %320 = load i64, ptr %319, align 8, !noalias !6231, !noundef !1733
  %321 = icmp slt i64 %318, %320
  br i1 %321, label %322, label %.preheader1725

322:                                              ; preds = %313
  store i64 %318, ptr %319, align 8, !noalias !6231
  br label %.preheader1725

.preheader1725:                                   ; preds = %322, %313
  br label %323

323:                                              ; preds = %.preheader1725, %326
  %324 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !6231
  %325 = icmp slt i64 %324, 0
  br i1 %325, label %326, label %__rustc::__rust_dealloc (.exit136)

326:                                              ; preds = %323
  %327 = add nsw i64 %324, 1
  %328 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %324, i64 %327 acq_rel acquire, align 8, !noalias !6231
  %329 = extractvalue { i64, i1 } %328, 1
  br i1 %329, label %330, label %323

330:                                              ; preds = %326
  %331 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %317 monotonic, align 8, !noalias !6231
  %332 = tail call i64 @llvm.ssub.sat.i64(i64 %331, i64 %317)
  %333 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !6231
  br label %334

334:                                              ; preds = %337, %330
  %335 = phi i64 [ %333, %330 ], [ %340, %337 ]
  %336 = icmp slt i64 %332, %335
  br i1 %336, label %337, label %341

337:                                              ; preds = %334
  %338 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %335, i64 %332 monotonic monotonic, align 8, !noalias !6231
  %339 = extractvalue { i64, i1 } %338, 1
  %340 = extractvalue { i64, i1 } %338, 0
  br i1 %339, label %341, label %334

341:                                              ; preds = %337, %334
  %342 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !6231
  br label %__rustc::__rust_dealloc (.exit136)

__rustc::__rust_dealloc (.exit136): ; preds = %323, %341
  tail call void @free(ptr noundef nonnull %267) #92, !noalias !6231
  br label %1512

343:                                              ; preds = %258
  call void @llvm.lifetime.end.p0(ptr nonnull %44), !noalias !6214
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %46, ptr noundef nonnull align 8 dereferenceable(24) %45, i64 24, i1 false), !noalias !6214
  call void @llvm.lifetime.end.p0(ptr nonnull %45)
  call void @llvm.lifetime.start.p0(ptr nonnull %42), !noalias !6214
  %344 = getelementptr inbounds nuw i8, ptr %3, i64 8
  %345 = load ptr, ptr %344, align 8, !alias.scope !6217, !noalias !6218, !nonnull !1733, !noundef !1733
  %346 = load i64, ptr %3, align 8, !range !1828, !alias.scope !6217, !noalias !6218, !noundef !1733
  %347 = getelementptr inbounds nuw [40 x i8], ptr %345, i64 %256
  store ptr %345, ptr %42, align 8, !noalias !6214
  %348 = getelementptr inbounds nuw i8, ptr %42, i64 16
  store i64 %346, ptr %348, align 8, !noalias !6214
  %349 = getelementptr inbounds nuw i8, ptr %42, i64 8
  store ptr %345, ptr %349, align 8, !noalias !6214
  %350 = getelementptr inbounds nuw i8, ptr %42, i64 24
  store ptr %347, ptr %350, align 8, !noalias !6214
  %351 = load ptr, ptr %60, align 16, !alias.scope !6235, !noalias !6236, !noundef !1733
  %352 = icmp eq ptr %351, null
  br i1 %352, label %356, label %353

353:                                              ; preds = %343
  %354 = atomicrmw add ptr %351, i64 1 monotonic, align 8, !noalias !6236
  %355 = icmp slt i64 %354, 0
  br i1 %355, label %498, label %479

356:                                              ; preds = %343
  call void @llvm.lifetime.start.p0(ptr nonnull %41), !noalias !6214
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %41, ptr noundef nonnull align 8 dereferenceable(32) %42, i64 32, i1 false), !noalias !6214
  %357 = getelementptr inbounds nuw i8, ptr %41, i64 24
  %358 = load ptr, ptr %357, align 8, !alias.scope !6237, !noalias !6240, !nonnull !1733, !noundef !1733
  %359 = getelementptr inbounds nuw i8, ptr %41, i64 8
  %360 = load ptr, ptr %359, align 8, !alias.scope !6237, !noalias !6240
  %361 = icmp eq ptr %360, %358
  br i1 %361, label %.loopexit151, label %362

362:                                              ; preds = %356
  %363 = getelementptr inbounds nuw i8, ptr %46, i64 16
  %364 = getelementptr inbounds nuw i8, ptr %46, i64 8
  br label %366

365:                                              ; preds = %467, %464
; call core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(32) %41) #89, !noalias !6236
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %46) #89, !noalias !6236
  br label %1567

366:                                              ; preds = %470, %362
  %367 = phi ptr [ %360, %362 ], [ %368, %470 ]
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6237)
  %368 = getelementptr inbounds nuw i8, ptr %367, i64 40
  %369 = load i64, ptr %367, align 8, !noalias !6242
  %370 = getelementptr inbounds nuw i8, ptr %367, i64 8
  %371 = load ptr, ptr %370, align 8, !noalias !6242
  %372 = icmp eq i64 %369, 0
  br i1 %372, label %.loopexit151, label %458

.loopexit151:                                     ; preds = %470, %366, %356
  %373 = phi ptr [ %360, %356 ], [ %368, %366 ], [ %368, %470 ]
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6243)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6246)
  %374 = ptrtoint ptr %358 to i64
  %375 = ptrtoint ptr %373 to i64
  %376 = sub nuw i64 %374, %375
  %377 = udiv exact i64 %376, 40
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6249)
  %378 = icmp eq ptr %358, %373
  br i1 %378, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %.loopexit151
  %379 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %380 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %381

381:                                              ; preds = %.preheader, %419
  %382 = phi i64 [ %384, %419 ], [ 0, %.preheader ]
  %383 = getelementptr inbounds nuw [40 x i8], ptr %373, i64 %382
  %384 = add nuw nsw i64 %382, 1
  %385 = load i64, ptr %383, align 8, !range !1771, !alias.scope !6252, !noalias !6255, !noundef !1733
  %386 = icmp ugt i64 %385, 5
  br i1 %386, label %387, label %419

387:                                              ; preds = %381
  %388 = getelementptr i8, ptr %383, i64 8
  %389 = load ptr, ptr %388, align 8, !alias.scope !6249, !noalias !6255, !nonnull !1733, !noundef !1733
  %390 = shl i64 %385, 3
  %391 = add i64 %390, -8
  %392 = load i64, ptr %379, align 8, !noalias !6256, !noundef !1733
  %393 = tail call i64 @llvm.umin.i64(i64 %391, i64 9223372036854775807)
  %394 = tail call i64 @llvm.ssub.sat.i64(i64 %392, i64 %393)
  store i64 %394, ptr %379, align 8, !noalias !6256
  %395 = load i64, ptr %380, align 8, !noalias !6256, !noundef !1733
  %396 = icmp slt i64 %394, %395
  br i1 %396, label %397, label %.preheader1472

397:                                              ; preds = %387
  store i64 %394, ptr %380, align 8, !noalias !6256
  br label %.preheader1472

.preheader1472:                                   ; preds = %397, %387
  br label %398

398:                                              ; preds = %.preheader1472, %401
  %399 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !6256
  %400 = icmp slt i64 %399, 0
  br i1 %400, label %401, label %__rustc::__rust_dealloc (.exit137)

401:                                              ; preds = %398
  %402 = add nsw i64 %399, 1
  %403 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %399, i64 %402 acq_rel acquire, align 8, !noalias !6256
  %404 = extractvalue { i64, i1 } %403, 1
  br i1 %404, label %405, label %398

405:                                              ; preds = %401
  %406 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %393 monotonic, align 8, !noalias !6256
  %407 = tail call i64 @llvm.ssub.sat.i64(i64 %406, i64 %393)
  %408 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !6256
  br label %409

409:                                              ; preds = %412, %405
  %410 = phi i64 [ %408, %405 ], [ %415, %412 ]
  %411 = icmp slt i64 %407, %410
  br i1 %411, label %412, label %416

412:                                              ; preds = %409
  %413 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %410, i64 %407 monotonic monotonic, align 8, !noalias !6256
  %414 = extractvalue { i64, i1 } %413, 1
  %415 = extractvalue { i64, i1 } %413, 0
  br i1 %414, label %416, label %409

416:                                              ; preds = %412, %409
  %417 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !6256
  br label %__rustc::__rust_dealloc (.exit137)

__rustc::__rust_dealloc (.exit137): ; preds = %398, %416
  %418 = icmp ne i64 %391, 0
  tail call void @llvm.assume(i1 %418), !noalias !6256
  tail call void @free(ptr noundef nonnull %389) #92, !noalias !6256
  br label %419

419:                                              ; preds = %__rustc::__rust_dealloc (.exit137), %381
  %420 = icmp eq i64 %384, %377
  br i1 %420, label %.loopexit, label %381

.loopexit:                                        ; preds = %419, %.loopexit151
  %421 = getelementptr inbounds nuw i8, ptr %41, i64 16
  %422 = load i64, ptr %421, align 8, !alias.scope !6259, !noalias !6214, !noundef !1733
  %423 = icmp eq i64 %422, 0
  br i1 %423, label %457, label %424

424:                                              ; preds = %.loopexit
  %425 = load ptr, ptr %41, align 8, !alias.scope !6259, !noalias !6214, !nonnull !1733, !noundef !1733
  %426 = mul nuw i64 %422, 40
  %427 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %428 = load i64, ptr %427, align 8, !noalias !6255, !noundef !1733
  %429 = tail call i64 @llvm.umin.i64(i64 %426, i64 9223372036854775807)
  %430 = tail call i64 @llvm.ssub.sat.i64(i64 %428, i64 %429)
  store i64 %430, ptr %427, align 8, !noalias !6255
  %431 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %432 = load i64, ptr %431, align 8, !noalias !6255, !noundef !1733
  %433 = icmp slt i64 %430, %432
  br i1 %433, label %434, label %.preheader1471

434:                                              ; preds = %424
  store i64 %430, ptr %431, align 8, !noalias !6255
  br label %.preheader1471

.preheader1471:                                   ; preds = %434, %424
  br label %435

435:                                              ; preds = %.preheader1471, %438
  %436 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !6255
  %437 = icmp slt i64 %436, 0
  br i1 %437, label %438, label %__rustc::__rust_dealloc (.exit138)

438:                                              ; preds = %435
  %439 = add nsw i64 %436, 1
  %440 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %436, i64 %439 acq_rel acquire, align 8, !noalias !6255
  %441 = extractvalue { i64, i1 } %440, 1
  br i1 %441, label %442, label %435

442:                                              ; preds = %438
  %443 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %429 monotonic, align 8, !noalias !6255
  %444 = tail call i64 @llvm.ssub.sat.i64(i64 %443, i64 %429)
  %445 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !6255
  br label %446

446:                                              ; preds = %449, %442
  %447 = phi i64 [ %445, %442 ], [ %452, %449 ]
  %448 = icmp slt i64 %444, %447
  br i1 %448, label %449, label %453

449:                                              ; preds = %446
  %450 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %447, i64 %444 monotonic monotonic, align 8, !noalias !6255
  %451 = extractvalue { i64, i1 } %450, 1
  %452 = extractvalue { i64, i1 } %450, 0
  br i1 %451, label %453, label %446

453:                                              ; preds = %449, %446
  %454 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !6255
  br label %__rustc::__rust_dealloc (.exit138)

__rustc::__rust_dealloc (.exit138): ; preds = %435, %453
  tail call void @free(ptr noundef nonnull %425) #92, !noalias !6255
  br label %457

455:                                              ; preds = %1452, %627
  %456 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(32) %42) #89, !noalias !6236
  br label %1653

457:                                              ; preds = %__rustc::__rust_dealloc (.exit138), %.loopexit
  call void @llvm.lifetime.end.p0(ptr nonnull %41), !noalias !6214
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %52, ptr noundef nonnull align 8 dereferenceable(24) %46, i64 24, i1 false), !noalias !6220
  call void @llvm.lifetime.end.p0(ptr nonnull %42), !noalias !6214
  call void @llvm.lifetime.end.p0(ptr nonnull %46), !noalias !6214
  br label %1512

458:                                              ; preds = %366
  %459 = getelementptr inbounds nuw i8, ptr %367, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %40, ptr noundef nonnull align 8 dereferenceable(24) %459, i64 24, i1 false), !noalias !6236
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6260)
  %460 = load i64, ptr %363, align 8, !alias.scope !6260, !noalias !6263, !noundef !1733
  %461 = load i64, ptr %46, align 8, !range !1828, !alias.scope !6260, !noalias !6263, !noundef !1733
  %462 = icmp eq i64 %460, %461
  br i1 %462, label %463, label %470

463:                                              ; preds = %458
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %46)
          to label %470 unwind label %464, !noalias !6265

464:                                              ; preds = %463
  %465 = landingpad { ptr, i32 }
          cleanup
  store ptr %368, ptr %359, align 8, !noalias !6214
  %466 = icmp ugt i64 %369, 5
  br i1 %466, label %467, label %365

467:                                              ; preds = %464
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %371) ]
  %468 = shl i64 %369, 3
  %469 = add i64 %468, -8
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %371, i64 noundef %469, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !6266
  br label %365

470:                                              ; preds = %463, %458
  %471 = load ptr, ptr %364, align 8, !alias.scope !6260, !noalias !6263, !nonnull !1733, !noundef !1733
  %472 = getelementptr inbounds nuw [40 x i8], ptr %471, i64 %460
  store i64 %369, ptr %472, align 8, !noalias !6269
  %473 = getelementptr inbounds nuw i8, ptr %472, i64 8
  store ptr %371, ptr %473, align 8, !noalias !6269
  %474 = getelementptr inbounds nuw i8, ptr %472, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %474, ptr noundef nonnull align 8 dereferenceable(24) %40, i64 24, i1 false), !noalias !6269
  %475 = add i64 %460, 1
  store i64 %475, ptr %363, align 8, !alias.scope !6260, !noalias !6263
  %476 = icmp eq ptr %368, %358
  br i1 %476, label %.loopexit151, label %366

477:                                              ; preds = %1567, %774, %593, %578
  %478 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !6236
  unreachable

479:                                              ; preds = %353
  %480 = load ptr, ptr %60, align 16, !alias.scope !6235, !noalias !6236, !nonnull !1733, !noundef !1733
  call void @llvm.lifetime.start.p0(ptr nonnull %39), !noalias !6214
  store ptr %480, ptr %39, align 8, !noalias !6214
  %481 = getelementptr inbounds nuw i8, ptr %480, i64 16
  %482 = load i64, ptr %481, align 8, !noalias !6236
  %483 = icmp eq i64 %482, -1
  %484 = getelementptr inbounds nuw i8, ptr %480, i64 40
  %485 = load i64, ptr %484, align 8, !noalias !6236
  %486 = icmp ne i64 %485, -1
  %487 = and i1 %226, %486
  br i1 %487, label %iter.check1445, label %499

iter.check1445:                                   ; preds = %479
  %min.iters.check1408 = icmp ult i64 %201, 8
  br i1 %min.iters.check1408, label %.preheader184.preheader, label %vector.main.loop.iter.check1409

vector.main.loop.iter.check1409:                  ; preds = %iter.check1445
  %min.iters.check1410 = icmp ult i64 %201, 32
  br i1 %min.iters.check1410, label %vec.epilog.ph1449, label %vector.ph1411

vector.ph1411:                                    ; preds = %vector.main.loop.iter.check1409
  %n.mod.vf1412 = and i64 %201, 24
  %n.vec1413 = and i64 %201, -32
  br label %vector.body1414

vector.body1414:                                  ; preds = %vector.body1414, %vector.ph1411
  %index1415 = phi i64 [ 0, %vector.ph1411 ], [ %index.next1436, %vector.body1414 ]
  %vec.ind1416 = phi <8 x i64> [ <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>, %vector.ph1411 ], [ %vec.ind.next1437, %vector.body1414 ]
  %vec.phi1417 = phi <8 x i64> [ zeroinitializer, %vector.ph1411 ], [ %488, %vector.body1414 ]
  %vec.phi1418 = phi <8 x i64> [ zeroinitializer, %vector.ph1411 ], [ %489, %vector.body1414 ]
  %vec.phi1419 = phi <8 x i64> [ zeroinitializer, %vector.ph1411 ], [ %490, %vector.body1414 ]
  %vec.phi1420 = phi <8 x i64> [ zeroinitializer, %vector.ph1411 ], [ %491, %vector.body1414 ]
  %step.add1421 = add nuw <8 x i64> %vec.ind1416, splat (i64 8)
  %step.add.21422 = add nuw <8 x i64> %vec.ind1416, splat (i64 16)
  %step.add.31423 = add nuw <8 x i64> %vec.ind1416, splat (i64 24)
  %wide.gep1424 = getelementptr inbounds nuw [160 x i8], ptr %202, <8 x i64> %vec.ind1416
  %wide.gep1425 = getelementptr inbounds nuw [160 x i8], ptr %202, <8 x i64> %step.add1421
  %wide.gep1426 = getelementptr inbounds nuw [160 x i8], ptr %202, <8 x i64> %step.add.21422
  %wide.gep1427 = getelementptr inbounds nuw [160 x i8], ptr %202, <8 x i64> %step.add.31423
  %wide.gep1428 = getelementptr i8, <8 x ptr> %wide.gep1424, i64 16
  %wide.gep1429 = getelementptr i8, <8 x ptr> %wide.gep1425, i64 16
  %wide.gep1430 = getelementptr i8, <8 x ptr> %wide.gep1426, i64 16
  %wide.gep1431 = getelementptr i8, <8 x ptr> %wide.gep1427, i64 16
  %wide.masked.gather1432 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1428, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !6236
  %wide.masked.gather1433 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1429, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !6236
  %wide.masked.gather1434 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1430, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !6236
  %wide.masked.gather1435 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1431, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !6236
  %488 = add <8 x i64> %wide.masked.gather1432, %vec.phi1417
  %489 = add <8 x i64> %wide.masked.gather1433, %vec.phi1418
  %490 = add <8 x i64> %wide.masked.gather1434, %vec.phi1419
  %491 = add <8 x i64> %wide.masked.gather1435, %vec.phi1420
  %index.next1436 = add nuw i64 %index1415, 32
  %vec.ind.next1437 = add nuw <8 x i64> %vec.ind1416, splat (i64 32)
  %492 = icmp eq i64 %index.next1436, %n.vec1413
  br i1 %492, label %middle.block1438, label %vector.body1414, !llvm.loop !6270

middle.block1438:                                 ; preds = %vector.body1414
  %bin.rdx1439 = add <8 x i64> %489, %488
  %bin.rdx1440 = add <8 x i64> %490, %bin.rdx1439
  %bin.rdx1441 = add <8 x i64> %491, %bin.rdx1440
  %493 = tail call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %bin.rdx1441)
  %cmp.n1442 = icmp eq i64 %201, %n.vec1413
  br i1 %cmp.n1442, label %.loopexit1468, label %vec.epilog.iter.check1447

vec.epilog.iter.check1447:                        ; preds = %middle.block1438
  %min.epilog.iters.check1448 = icmp eq i64 %n.mod.vf1412, 0
  br i1 %min.epilog.iters.check1448, label %.preheader184.preheader, label %vec.epilog.ph1449, !prof !6205

vec.epilog.ph1449:                                ; preds = %vector.main.loop.iter.check1409, %vec.epilog.iter.check1447
  %vec.epilog.resume.val1443 = phi i64 [ %n.vec1413, %vec.epilog.iter.check1447 ], [ 0, %vector.main.loop.iter.check1409 ]
  %bc.merge.rdx1444 = phi i64 [ %493, %vec.epilog.iter.check1447 ], [ 0, %vector.main.loop.iter.check1409 ]
  %n.vec1451 = and i64 %201, -8
  %494 = insertelement <8 x i64> <i64 poison, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0>, i64 %bc.merge.rdx1444, i64 0
  %broadcast.splatinsert1452 = insertelement <8 x i64> poison, i64 %vec.epilog.resume.val1443, i64 0
  %broadcast.splat1453 = shufflevector <8 x i64> %broadcast.splatinsert1452, <8 x i64> poison, <8 x i32> zeroinitializer
  %induction1454 = or disjoint <8 x i64> %broadcast.splat1453, <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>
  br label %vec.epilog.vector.body1455

vec.epilog.vector.body1455:                       ; preds = %vec.epilog.vector.body1455, %vec.epilog.ph1449
  %index1456 = phi i64 [ %vec.epilog.resume.val1443, %vec.epilog.ph1449 ], [ %index.next1462, %vec.epilog.vector.body1455 ]
  %vec.ind1457 = phi <8 x i64> [ %induction1454, %vec.epilog.ph1449 ], [ %vec.ind.next1463, %vec.epilog.vector.body1455 ]
  %vec.phi1458 = phi <8 x i64> [ %494, %vec.epilog.ph1449 ], [ %495, %vec.epilog.vector.body1455 ]
  %wide.gep1459 = getelementptr inbounds nuw [160 x i8], ptr %202, <8 x i64> %vec.ind1457
  %wide.gep1460 = getelementptr i8, <8 x ptr> %wide.gep1459, i64 16
  %wide.masked.gather1461 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1460, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !6236
  %495 = add <8 x i64> %wide.masked.gather1461, %vec.phi1458
  %index.next1462 = add nuw i64 %index1456, 8
  %vec.ind.next1463 = add nuw <8 x i64> %vec.ind1457, splat (i64 8)
  %496 = icmp eq i64 %index.next1462, %n.vec1451
  br i1 %496, label %vec.epilog.middle.block1464, label %vec.epilog.vector.body1455, !llvm.loop !6271

vec.epilog.middle.block1464:                      ; preds = %vec.epilog.vector.body1455
  %497 = tail call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %495)
  %cmp.n1465 = icmp eq i64 %201, %n.vec1451
  br i1 %cmp.n1465, label %.loopexit1468, label %.preheader184.preheader

.preheader184.preheader:                          ; preds = %iter.check1445, %vec.epilog.iter.check1447, %vec.epilog.middle.block1464
  %.ph1717 = phi i64 [ 0, %iter.check1445 ], [ %n.vec1413, %vec.epilog.iter.check1447 ], [ %n.vec1451, %vec.epilog.middle.block1464 ]
  %.ph1718 = phi i64 [ 0, %iter.check1445 ], [ %493, %vec.epilog.iter.check1447 ], [ %497, %vec.epilog.middle.block1464 ]
  br label %.preheader184

498:                                              ; preds = %353
  tail call void @llvm.trap()
  unreachable

499:                                              ; preds = %590, %586, %479
  %500 = icmp ult i64 %201, 57646075230342349
  tail call void @llvm.assume(i1 %500)
  %501 = mul nuw nsw i64 %201, 160
  %502 = getelementptr inbounds nuw i8, ptr %202, i64 %501
  call void @llvm.lifetime.start.p0(ptr nonnull %38), !noalias !6214
  store ptr %202, ptr %38, align 8, !noalias !6214
  %503 = getelementptr inbounds nuw i8, ptr %38, i64 8
  store ptr %202, ptr %503, align 8, !noalias !6214
  %504 = getelementptr inbounds nuw i8, ptr %38, i64 16
  store i64 %222, ptr %504, align 8, !noalias !6214
  %505 = getelementptr inbounds nuw i8, ptr %38, i64 24
  store ptr %502, ptr %505, align 8, !noalias !6214
  %506 = icmp eq i64 %201, 0
  br i1 %506, label %.loopexit182, label %507

507:                                              ; preds = %499
  %508 = getelementptr inbounds nuw i8, ptr %37, i64 8
  %509 = getelementptr inbounds nuw i8, ptr %37, i64 24
  %510 = getelementptr inbounds nuw i8, ptr %37, i64 32
  %511 = getelementptr inbounds nuw i8, ptr %37, i64 40
  %512 = getelementptr inbounds nuw i8, ptr %37, i64 16
  %513 = getelementptr inbounds nuw i8, ptr %36, i64 16
  %514 = getelementptr inbounds nuw i8, ptr %36, i64 8
  %515 = getelementptr inbounds nuw i8, ptr %36, i64 24
  %516 = getelementptr inbounds nuw i8, ptr %37, i64 96
  %517 = getelementptr inbounds nuw i8, ptr %37, i64 48
  %518 = getelementptr inbounds nuw i8, ptr %37, i64 56
  %519 = getelementptr inbounds nuw i8, ptr %37, i64 64
  %520 = getelementptr inbounds nuw i8, ptr %480, i64 80
  %521 = getelementptr inbounds nuw i8, ptr %24, i64 1
  %522 = getelementptr inbounds nuw i8, ptr %24, i64 8
  %523 = getelementptr inbounds nuw i8, ptr %24, i64 16
  %524 = getelementptr inbounds nuw i8, ptr %2, i64 624
  %525 = getelementptr inbounds nuw i8, ptr %2, i64 1220
  %526 = zext nneg i8 %249 to i64
  %527 = getelementptr inbounds nuw i8, ptr %480, i64 296
  %528 = getelementptr inbounds nuw i8, ptr %480, i64 272
  %529 = getelementptr inbounds nuw i8, ptr %2, i64 1040
  %530 = getelementptr inbounds nuw i8, ptr %2, i64 1048
  %531 = getelementptr inbounds nuw i8, ptr %480, i64 104
  %532 = getelementptr inbounds nuw i8, ptr %18, i64 1
  %533 = getelementptr inbounds nuw i8, ptr %18, i64 8
  %534 = getelementptr inbounds nuw i8, ptr %18, i64 16
  %535 = getelementptr inbounds nuw i8, ptr %23, i64 8
  %536 = getelementptr inbounds nuw i8, ptr %2, i64 880
  %537 = getelementptr inbounds nuw i8, ptr %12, i64 1
  %538 = getelementptr inbounds nuw i8, ptr %12, i64 8
  %539 = getelementptr inbounds nuw i8, ptr %12, i64 16
  %540 = getelementptr inbounds nuw i8, ptr %19, i64 8
  %541 = getelementptr inbounds nuw i8, ptr %21, i64 1
  %542 = getelementptr inbounds nuw i8, ptr %21, i64 8
  %543 = getelementptr inbounds nuw i8, ptr %21, i64 16
  %544 = getelementptr inbounds nuw i8, ptr %20, i64 8
  %545 = getelementptr inbounds nuw i8, ptr %17, i64 8
  %546 = getelementptr inbounds nuw i8, ptr %16, i64 8
  %547 = getelementptr inbounds nuw i8, ptr %14, i64 1
  %548 = getelementptr inbounds nuw i8, ptr %14, i64 8
  %549 = getelementptr inbounds nuw i8, ptr %14, i64 16
  %550 = getelementptr inbounds nuw i8, ptr %13, i64 8
  %551 = getelementptr inbounds nuw i8, ptr %46, i64 16
  %552 = getelementptr inbounds nuw i8, ptr %46, i64 8
  %553 = getelementptr inbounds nuw i8, ptr %37, i64 88
  %554 = getelementptr inbounds nuw i8, ptr %37, i64 120
  %555 = getelementptr inbounds nuw i8, ptr %9, i64 1
  %556 = getelementptr inbounds nuw i8, ptr %9, i64 8
  %557 = getelementptr inbounds nuw i8, ptr %9, i64 16
  %558 = getelementptr inbounds nuw i8, ptr %6, i64 1
  %559 = getelementptr inbounds nuw i8, ptr %6, i64 8
  %560 = getelementptr inbounds nuw i8, ptr %6, i64 16
  %561 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %562 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %594

.preheader184:                                    ; preds = %.preheader184.preheader, %.preheader184
  %563 = phi i64 [ %570, %.preheader184 ], [ %.ph1717, %.preheader184.preheader ]
  %564 = phi i64 [ %569, %.preheader184 ], [ %.ph1718, %.preheader184.preheader ]
  %565 = getelementptr inbounds nuw [160 x i8], ptr %202, i64 %563
  %566 = getelementptr i8, ptr %565, i64 16
  %567 = load i64, ptr %566, align 8, !noalias !6236, !noundef !1733
  %568 = icmp ult i64 %567, 104811045873349726
  tail call void @llvm.assume(i1 %568)
  %569 = add i64 %567, %564
  %570 = add nuw i64 %563, 1
  %571 = icmp eq i64 %570, %201
  br i1 %571, label %.loopexit1468, label %.preheader184, !llvm.loop !6272

572:                                              ; preds = %.loopexit154, %.loopexit.split-lp, %593
  %573 = phi i1 [ %777, %593 ], [ true, %.loopexit154 ], [ %.ph, %.loopexit.split-lp ]
  %574 = phi i1 [ false, %593 ], [ false, %.loopexit154 ], [ %.ph155, %.loopexit.split-lp ]
  %575 = phi { ptr, i32 } [ %778, %593 ], [ %lpad.loopexit, %.loopexit154 ], [ %lpad.loopexit.split-lp, %.loopexit.split-lp ]
  %576 = atomicrmw sub ptr %480, i64 1 release, align 8, !noalias !6273
  %577 = icmp eq i64 %576, 1
  br i1 %577, label %578, label %1509

578:                                              ; preds = %572
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %39) #91
          to label %1509 unwind label %477, !noalias !6236

.loopexit154:                                     ; preds = %644
  %lpad.loopexit = landingpad { ptr, i32 }
          cleanup
  br label %572

.loopexit.split-lp:                               ; preds = %585, %590, %.loopexit182, %1448
  %.ph = phi i1 [ true, %590 ], [ true, %.loopexit182 ], [ true, %585 ], [ false, %1448 ]
  %.ph155 = phi i1 [ true, %590 ], [ false, %.loopexit182 ], [ true, %585 ], [ false, %1448 ]
  %lpad.loopexit.split-lp = landingpad { ptr, i32 }
          cleanup
  br label %572

.loopexit1468:                                    ; preds = %.preheader184, %vec.epilog.middle.block1464, %middle.block1438
  %.lcssa1368 = phi i64 [ %497, %vec.epilog.middle.block1464 ], [ %493, %middle.block1438 ], [ %569, %.preheader184 ]
  %579 = getelementptr inbounds nuw i8, ptr %2, i64 904
  %580 = getelementptr inbounds nuw i8, ptr %2, i64 920
  %581 = load i64, ptr %580, align 8, !alias.scope !6278, !noalias !6236, !noundef !1733
  %582 = load i64, ptr %579, align 8, !range !1828, !alias.scope !6278, !noalias !6236, !noundef !1733
  %583 = sub i64 %582, %581
  %584 = icmp ugt i64 %.lcssa1368, %583
  br i1 %584, label %585, label %586, !prof !6283

585:                                              ; preds = %.loopexit1468
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.1794586459888082020)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %579, i64 noundef %581, i64 noundef %.lcssa1368, i64 noundef 8, i64 noundef 80)
          to label %586 unwind label %.loopexit.split-lp, !noalias !6236

586:                                              ; preds = %585, %.loopexit1468
  %587 = getelementptr inbounds nuw i8, ptr %2, i64 1008
  %588 = load i64, ptr %587, align 16, !alias.scope !6284, !noalias !6236, !noundef !1733
  %589 = icmp ugt i64 %.lcssa1368, %588
  br i1 %589, label %590, label %499, !prof !6283

590:                                              ; preds = %586
  %591 = getelementptr inbounds nuw i8, ptr %2, i64 992
; invoke <hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>
  %592 = invoke { i64, i64 } @<hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>(ptr noalias nofree noundef nonnull align 8 dereferenceable(32) %591, i64 noundef %.lcssa1368, ptr noundef nonnull align 8 %579, i1 noundef zeroext true) #91
          to label %499 unwind label %.loopexit.split-lp, !noalias !6236

593:                                              ; preds = %1508, %1505, %1502
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %38) #89
          to label %572 unwind label %477, !noalias !6236

594:                                              ; preds = %837, %507
  %595 = phi ptr [ %345, %507 ], [ %744, %837 ]
  %596 = phi ptr [ %202, %507 ], [ %597, %837 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !6287)
  %597 = getelementptr inbounds nuw i8, ptr %596, i64 160
  store ptr %597, ptr %503, align 8, !alias.scope !6287, !noalias !6290
  %598 = load i64, ptr %596, align 8, !noalias !6292
  %599 = icmp eq i64 %598, -1
  br i1 %599, label %.loopexit182, label %600

600:                                              ; preds = %594
  %601 = getelementptr inbounds nuw i8, ptr %596, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %37), !noalias !6214
  store i64 %598, ptr %37, align 8, !noalias !6214
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(152) %508, ptr noundef nonnull align 8 dereferenceable(152) %601, i64 152, i1 false), !noalias !6236
  %602 = load i64, ptr %509, align 8, !noalias !6214
  %603 = load ptr, ptr %510, align 8, !noalias !6214
  %604 = load i64, ptr %511, align 8, !noalias !6214
  call void @llvm.lifetime.start.p0(ptr nonnull %36), !noalias !6214
  %605 = load ptr, ptr %508, align 8, !noalias !6214, !nonnull !1733, !noundef !1733
  %606 = load i64, ptr %512, align 8, !noalias !6214, !noundef !1733
  %607 = icmp ult i64 %606, 104811045873349726
  call void @llvm.assume(i1 %607)
  %608 = getelementptr inbounds nuw [88 x i8], ptr %605, i64 %606
  store ptr %605, ptr %36, align 8, !noalias !6214
  store i64 %598, ptr %513, align 8, !noalias !6214
  store ptr %605, ptr %514, align 8, !noalias !6214
  store ptr %608, ptr %515, align 8, !noalias !6214
  %609 = load ptr, ptr %518, align 8, !noalias !6214, !nonnull !1733, !noundef !1733
  %610 = load i64, ptr %517, align 8, !range !1828, !noalias !6214, !noundef !1733
  %611 = load i64, ptr %519, align 8, !noalias !6214, !noundef !1733
  %612 = icmp ult i64 %611, 288230376151711744
  call void @llvm.assume(i1 %612)
  %613 = shl nuw nsw i64 %611, 5
  %614 = getelementptr inbounds nuw i8, ptr %609, i64 %613
  %615 = icmp eq i64 %611, 0
  br i1 %615, label %.loopexit181, label %616

616:                                              ; preds = %600
  %617 = load i64, ptr %516, align 8, !noalias !6214, !noundef !1733
  %618 = icmp ult i64 %604, 384307168202282326
  %619 = ptrtoint ptr %608 to i64
  br label %727

.loopexit182:                                     ; preds = %837, %594, %499
  %620 = phi ptr [ %345, %499 ], [ %744, %837 ], [ %595, %594 ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %38)
          to label %621 unwind label %.loopexit.split-lp, !noalias !6236

621:                                              ; preds = %.loopexit182
  call void @llvm.lifetime.end.p0(ptr nonnull %38), !noalias !6214
  %622 = xor i1 %225, true
  %623 = or i1 %247, %622
  %624 = select i1 %623, i1 true, i1 %483
  br i1 %624, label %<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit), label %628

<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit.sink.split): ; preds = %.noexc, %640
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !6293
  br label %<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit)

<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit): ; preds = %<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit.sink.split), %621
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %52, ptr noundef nonnull align 8 dereferenceable(24) %46, i64 24, i1 false), !noalias !6220
  %625 = atomicrmw sub ptr %480, i64 1 release, align 8, !noalias !6300
  %626 = icmp eq i64 %625, 1
  br i1 %626, label %627, label %645

627:                                              ; preds = %<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit)
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %39) #91
          to label %645 unwind label %455, !noalias !6236

628:                                              ; preds = %621
  %629 = getelementptr inbounds nuw i8, ptr %480, i64 80
  %630 = getelementptr inbounds nuw i8, ptr %7, i64 1
  %631 = getelementptr inbounds nuw i8, ptr %7, i64 8
  %632 = getelementptr inbounds nuw i8, ptr %7, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %8), !noalias !6293
  %633 = load atomic i64, ptr %629 monotonic, align 8, !noalias !6305
  br label %634

634:                                              ; preds = %634, %628
  %635 = phi i64 [ %633, %628 ], [ %639, %634 ]
  %636 = call i64 @llvm.uadd.sat.i64(i64 %635, i64 1)
  %637 = cmpxchg weak ptr %629, i64 %635, i64 %636 monotonic monotonic, align 8, !noalias !6305
  %638 = extractvalue { i64, i1 } %637, 1
  %639 = extractvalue { i64, i1 } %637, 0
  br i1 %638, label %640, label %634

640:                                              ; preds = %634
  %641 = call i64 @llvm.uadd.sat.i64(i64 %639, i64 1)
  %642 = load i64, ptr %481, align 8, !noalias !6305
  %643 = icmp ugt i64 %641, %642
  br i1 %643, label %644, label %<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit.sink.split)

644:                                              ; preds = %640
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !6305
  store i8 0, ptr %630, align 1, !noalias !6305
  store i64 %642, ptr %631, align 8, !noalias !6305
  store i64 %641, ptr %632, align 8, !noalias !6305
  store i8 0, ptr %7, align 8, !noalias !6305
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %8, ptr noundef nonnull align 8 %481, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %7)
          to label %.noexc unwind label %.loopexit154

.noexc:                                           ; preds = %644
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !6305
  br label %<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit.sink.split)

645:                                              ; preds = %627, %<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit)
  call void @llvm.lifetime.end.p0(ptr nonnull %39), !noalias !6214
  call void @llvm.experimental.noalias.scope.decl(metadata !6308)
  call void @llvm.experimental.noalias.scope.decl(metadata !6311), !noalias !6314
  %646 = load ptr, ptr %350, align 8, !alias.scope !6315, !noalias !6316, !nonnull !1733, !noundef !1733
  %647 = ptrtoint ptr %646 to i64
  %648 = ptrtoint ptr %620 to i64
  %649 = sub nuw i64 %647, %648
  %650 = udiv exact i64 %649, 40
  call void @llvm.experimental.noalias.scope.decl(metadata !6317), !noalias !6314
  %651 = icmp eq ptr %646, %620
  br i1 %651, label %.loopexit153, label %.preheader152

.preheader152:                                    ; preds = %645
  %652 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %653 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %654

654:                                              ; preds = %.preheader152, %692
  %655 = phi i64 [ %657, %692 ], [ 0, %.preheader152 ]
  %656 = getelementptr inbounds nuw [40 x i8], ptr %620, i64 %655
  %657 = add nuw nsw i64 %655, 1
  %658 = load i64, ptr %656, align 8, !range !1771, !alias.scope !6320, !noalias !6323, !noundef !1733
  %659 = icmp ugt i64 %658, 5
  br i1 %659, label %660, label %692

660:                                              ; preds = %654
  %661 = getelementptr i8, ptr %656, i64 8
  %662 = load ptr, ptr %661, align 8, !alias.scope !6317, !noalias !6323, !nonnull !1733, !noundef !1733
  %663 = shl i64 %658, 3
  %664 = add i64 %663, -8
  %665 = load i64, ptr %652, align 8, !noalias !6324, !noundef !1733
  %666 = call i64 @llvm.umin.i64(i64 %664, i64 9223372036854775807)
  %667 = call i64 @llvm.ssub.sat.i64(i64 %665, i64 %666)
  store i64 %667, ptr %652, align 8, !noalias !6324
  %668 = load i64, ptr %653, align 8, !noalias !6324, !noundef !1733
  %669 = icmp slt i64 %667, %668
  br i1 %669, label %670, label %.preheader1480

670:                                              ; preds = %660
  store i64 %667, ptr %653, align 8, !noalias !6324
  br label %.preheader1480

.preheader1480:                                   ; preds = %670, %660
  br label %671

671:                                              ; preds = %.preheader1480, %674
  %672 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !6324
  %673 = icmp slt i64 %672, 0
  br i1 %673, label %674, label %__rustc::__rust_dealloc (.exit139)

674:                                              ; preds = %671
  %675 = add nsw i64 %672, 1
  %676 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %672, i64 %675 acq_rel acquire, align 8, !noalias !6324
  %677 = extractvalue { i64, i1 } %676, 1
  br i1 %677, label %678, label %671

678:                                              ; preds = %674
  %679 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %666 monotonic, align 8, !noalias !6324
  %680 = call i64 @llvm.ssub.sat.i64(i64 %679, i64 %666)
  %681 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !6324
  br label %682

682:                                              ; preds = %685, %678
  %683 = phi i64 [ %681, %678 ], [ %688, %685 ]
  %684 = icmp slt i64 %680, %683
  br i1 %684, label %685, label %689

685:                                              ; preds = %682
  %686 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %683, i64 %680 monotonic monotonic, align 8, !noalias !6324
  %687 = extractvalue { i64, i1 } %686, 1
  %688 = extractvalue { i64, i1 } %686, 0
  br i1 %687, label %689, label %682

689:                                              ; preds = %685, %682
  %690 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !6324
  br label %__rustc::__rust_dealloc (.exit139)

__rustc::__rust_dealloc (.exit139): ; preds = %671, %689
  %691 = icmp ne i64 %664, 0
  call void @llvm.assume(i1 %691), !noalias !6324
  call void @free(ptr noundef nonnull %662) #92, !noalias !6324
  br label %692

692:                                              ; preds = %__rustc::__rust_dealloc (.exit139), %654
  %693 = icmp eq i64 %657, %650
  br i1 %693, label %.loopexit153, label %654

.loopexit153:                                     ; preds = %692, %645
  %694 = load i64, ptr %348, align 8, !alias.scope !6315, !noalias !6316, !noundef !1733
  %695 = icmp eq i64 %694, 0
  br i1 %695, label %1600, label %.loopexit153._crit_edge

.loopexit153._crit_edge:                          ; preds = %.loopexit153
  %.pre = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %.pre683 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %1569

696:                                              ; preds = %1042
  %697 = landingpad { ptr, i32 }
          cleanup
  store ptr %1038, ptr %514, align 8, !noalias !6214
  br label %722

698:                                              ; preds = %1020
  %699 = landingpad { ptr, i32 }
          cleanup
  br label %722

700:                                              ; preds = %1108
  %701 = landingpad { ptr, i32 }
          cleanup
  store ptr %1104, ptr %514, align 8, !noalias !6214
  br label %722

702:                                              ; preds = %1002
  %703 = landingpad { ptr, i32 }
          cleanup
  store ptr %998, ptr %514, align 8, !noalias !6214
  br label %722

704:                                              ; preds = %1335
  %705 = landingpad { ptr, i32 }
          cleanup
  store ptr %1331, ptr %514, align 8, !noalias !6214
  br label %722

706:                                              ; preds = %1261
  %707 = landingpad { ptr, i32 }
          cleanup
  br label %722

708:                                              ; preds = %1234
  %709 = landingpad { ptr, i32 }
          cleanup
  store ptr %1230, ptr %514, align 8, !noalias !6214
  br label %722

710:                                              ; preds = %1176
  %711 = landingpad { ptr, i32 }
          cleanup
  store ptr %1172, ptr %514, align 8, !noalias !6214
  br label %722

712:                                              ; preds = %1140, %1124, %1092
  %713 = landingpad { ptr, i32 }
          cleanup
  br label %722

714:                                              ; preds = %.preheader176
  %715 = landingpad { ptr, i32 }
          cleanup
  br label %722

716:                                              ; preds = %858
  %717 = landingpad { ptr, i32 }
          cleanup
  br label %722

718:                                              ; preds = %1315, %1302, %1293
  %719 = landingpad { ptr, i32 }
          cleanup
  br label %722

720:                                              ; preds = %883, %841
  %721 = landingpad { ptr, i32 }
          cleanup
  br label %722

722:                                              ; preds = %1362, %1359, %720, %718, %716, %714, %712, %710, %708, %706, %704, %702, %700, %698, %696
  %723 = phi { ptr, i32 } [ %1360, %1359 ], [ %1360, %1362 ], [ %697, %696 ], [ %699, %698 ], [ %701, %700 ], [ %703, %702 ], [ %705, %704 ], [ %707, %706 ], [ %709, %708 ], [ %711, %710 ], [ %713, %712 ], [ %715, %714 ], [ %717, %716 ], [ %719, %718 ], [ %721, %720 ]
  %724 = icmp eq i64 %610, 0
  br i1 %724, label %774, label %725

725:                                              ; preds = %722
  %726 = shl nuw i64 %610, 5
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %609, i64 noundef %726, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !6327
  br label %774

727:                                              ; preds = %.loopexit162, %616
  %728 = phi ptr [ %595, %616 ], [ %1345, %.loopexit162 ]
  %729 = phi ptr [ %605, %616 ], [ %1156, %.loopexit162 ]
  %730 = phi i64 [ 0, %616 ], [ %881, %.loopexit162 ]
  %731 = phi ptr [ %609, %616 ], [ %734, %.loopexit162 ]
  %732 = phi ptr [ %605, %616 ], [ %1158, %.loopexit162 ]
  %733 = phi i64 [ %617, %616 ], [ %1157, %.loopexit162 ]
  %734 = getelementptr inbounds nuw i8, ptr %731, i64 32
  %735 = load i64, ptr %731, align 8, !noalias !6330
  %736 = getelementptr inbounds nuw i8, ptr %731, i64 8
  %737 = load i64, ptr %736, align 8, !noalias !6330
  %738 = getelementptr inbounds nuw i8, ptr %731, i64 16
  %739 = load i64, ptr %738, align 8, !noalias !6330
  %740 = getelementptr inbounds nuw i8, ptr %731, i64 24
  %741 = load i64, ptr %740, align 8, !noalias !6330
  %742 = icmp eq i64 %735, 0
  %743 = select i1 %742, i1 true, i1 %483
  br i1 %743, label %839, label %846

.loopexit181:                                     ; preds = %.loopexit162, %600
  %744 = phi ptr [ %595, %600 ], [ %1345, %.loopexit162 ]
  %745 = icmp eq i64 %610, 0
  br i1 %745, label %775, label %746

746:                                              ; preds = %.loopexit181
  %747 = shl nuw i64 %610, 5
  %748 = load i64, ptr %561, align 8, !noalias !6333, !noundef !1733
  %749 = call i64 @llvm.umin.i64(i64 %747, i64 9223372036854775807)
  %750 = call i64 @llvm.ssub.sat.i64(i64 %748, i64 %749)
  store i64 %750, ptr %561, align 8, !noalias !6333
  %751 = load i64, ptr %562, align 8, !noalias !6333, !noundef !1733
  %752 = icmp slt i64 %750, %751
  br i1 %752, label %753, label %.preheader1496

753:                                              ; preds = %746
  store i64 %750, ptr %562, align 8, !noalias !6333
  br label %.preheader1496

.preheader1496:                                   ; preds = %753, %746
  br label %754

754:                                              ; preds = %.preheader1496, %757
  %755 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !6333
  %756 = icmp slt i64 %755, 0
  br i1 %756, label %757, label %__rustc::__rust_dealloc (.exit140)

757:                                              ; preds = %754
  %758 = add nsw i64 %755, 1
  %759 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %755, i64 %758 acq_rel acquire, align 8, !noalias !6333
  %760 = extractvalue { i64, i1 } %759, 1
  br i1 %760, label %761, label %754

761:                                              ; preds = %757
  %762 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %749 monotonic, align 8, !noalias !6333
  %763 = call i64 @llvm.ssub.sat.i64(i64 %762, i64 %749)
  %764 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !6333
  br label %765

765:                                              ; preds = %768, %761
  %766 = phi i64 [ %764, %761 ], [ %771, %768 ]
  %767 = icmp slt i64 %763, %766
  br i1 %767, label %768, label %772

768:                                              ; preds = %765
  %769 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %766, i64 %763 monotonic monotonic, align 8, !noalias !6333
  %770 = extractvalue { i64, i1 } %769, 1
  %771 = extractvalue { i64, i1 } %769, 0
  br i1 %770, label %772, label %765

772:                                              ; preds = %768, %765
  %773 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !6333
  br label %__rustc::__rust_dealloc (.exit140)

__rustc::__rust_dealloc (.exit140): ; preds = %754, %772
  call void @free(ptr noundef nonnull %609) #92, !noalias !6333
  br label %775

774:                                              ; preds = %725, %722
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %36) #89
          to label %776 unwind label %477, !noalias !6236

775:                                              ; preds = %__rustc::__rust_dealloc (.exit140), %.loopexit181
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %36)
          to label %786 unwind label %782, !noalias !6236

776:                                              ; preds = %784, %782, %774
  %777 = phi i1 [ true, %774 ], [ true, %782 ], [ false, %784 ]
  %778 = phi { ptr, i32 } [ %723, %774 ], [ %783, %782 ], [ %785, %784 ]
  %779 = icmp eq i64 %602, 0
  br i1 %779, label %816, label %780

780:                                              ; preds = %776
  %781 = mul nuw i64 %602, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %603) ]
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %603, i64 noundef %781, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !6236
  br label %816

782:                                              ; preds = %775
  %783 = landingpad { ptr, i32 }
          cleanup
  br label %776

784:                                              ; preds = %1403
  %785 = landingpad { ptr, i32 }
          cleanup
  br label %776

786:                                              ; preds = %775
  call void @llvm.lifetime.end.p0(ptr nonnull %36), !noalias !6214
  %787 = icmp eq i64 %602, 0
  br i1 %787, label %823, label %788

788:                                              ; preds = %786
  %789 = mul nuw i64 %602, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %603) ]
  %790 = load i64, ptr %561, align 8, !noalias !6236, !noundef !1733
  %791 = call i64 @llvm.umin.i64(i64 %789, i64 9223372036854775807)
  %792 = call i64 @llvm.ssub.sat.i64(i64 %790, i64 %791)
  store i64 %792, ptr %561, align 8, !noalias !6236
  %793 = load i64, ptr %562, align 8, !noalias !6236, !noundef !1733
  %794 = icmp slt i64 %792, %793
  br i1 %794, label %795, label %.preheader1495

795:                                              ; preds = %788
  store i64 %792, ptr %562, align 8, !noalias !6236
  br label %.preheader1495

.preheader1495:                                   ; preds = %795, %788
  br label %796

796:                                              ; preds = %.preheader1495, %799
  %797 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !6236
  %798 = icmp slt i64 %797, 0
  br i1 %798, label %799, label %__rustc::__rust_dealloc (.exit141)

799:                                              ; preds = %796
  %800 = add nsw i64 %797, 1
  %801 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %797, i64 %800 acq_rel acquire, align 8, !noalias !6236
  %802 = extractvalue { i64, i1 } %801, 1
  br i1 %802, label %803, label %796

803:                                              ; preds = %799
  %804 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %791 monotonic, align 8, !noalias !6236
  %805 = call i64 @llvm.ssub.sat.i64(i64 %804, i64 %791)
  %806 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !6236
  br label %807

807:                                              ; preds = %810, %803
  %808 = phi i64 [ %806, %803 ], [ %813, %810 ]
  %809 = icmp slt i64 %805, %808
  br i1 %809, label %810, label %814

810:                                              ; preds = %807
  %811 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %808, i64 %805 monotonic monotonic, align 8, !noalias !6236
  %812 = extractvalue { i64, i1 } %811, 1
  %813 = extractvalue { i64, i1 } %811, 0
  br i1 %812, label %814, label %807

814:                                              ; preds = %810, %807
  %815 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !6236
  br label %__rustc::__rust_dealloc (.exit141)

__rustc::__rust_dealloc (.exit141): ; preds = %796, %814
  call void @free(ptr noundef nonnull %603) #92, !noalias !6236
  br label %823

816:                                              ; preds = %780, %776
  call void @llvm.experimental.noalias.scope.decl(metadata !6336)
  %817 = load ptr, ptr %553, align 8, !alias.scope !6336, !noalias !6214, !noundef !1733
  %818 = icmp eq ptr %817, null
  br i1 %818, label %1502, label %819

819:                                              ; preds = %816
  %820 = atomicrmw sub ptr %817, i64 1 release, align 8, !noalias !6339
  %821 = icmp eq i64 %820, 1
  br i1 %821, label %822, label %1502

822:                                              ; preds = %819
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %553) #91, !noalias !6236
  br label %1502

823:                                              ; preds = %__rustc::__rust_dealloc (.exit141), %786
  call void @llvm.experimental.noalias.scope.decl(metadata !6344)
  %824 = load ptr, ptr %553, align 8, !alias.scope !6344, !noalias !6214, !noundef !1733
  %825 = icmp eq ptr %824, null
  br i1 %825, label %830, label %826

826:                                              ; preds = %823
  %827 = atomicrmw sub ptr %824, i64 1 release, align 8, !noalias !6347
  %828 = icmp eq i64 %827, 1
  br i1 %828, label %829, label %830

829:                                              ; preds = %826
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %553) #91, !noalias !6236
  br label %830

830:                                              ; preds = %829, %826, %823
  call void @llvm.experimental.noalias.scope.decl(metadata !6352)
  %831 = load ptr, ptr %554, align 8, !alias.scope !6352, !noalias !6214, !noundef !1733
  %832 = icmp eq ptr %831, null
  br i1 %832, label %837, label %833

833:                                              ; preds = %830
  %834 = atomicrmw sub ptr %831, i64 1 release, align 8, !noalias !6355
  %835 = icmp eq i64 %834, 1
  br i1 %835, label %836, label %837

836:                                              ; preds = %833
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %554) #91, !noalias !6236
  br label %837

837:                                              ; preds = %836, %833, %830
  call void @llvm.lifetime.end.p0(ptr nonnull %37), !noalias !6214
  %838 = icmp eq ptr %597, %502
  br i1 %838, label %.loopexit182, label %594

839:                                              ; preds = %872, %866, %862, %727
  call void @llvm.assume(i1 %618)
  %840 = icmp ugt i64 %730, %604
  br i1 %840, label %841, label %878, !prof !1735

841:                                              ; preds = %839
  call void @llvm.lifetime.start.p0(ptr nonnull %28), !noalias !6214
  store i64 %730, ptr %28, align 8, !noalias !6214
  call void @llvm.lifetime.start.p0(ptr nonnull %27), !noalias !6214
  store i64 %604, ptr %27, align 8, !noalias !6214
  call void @llvm.lifetime.start.p0(ptr nonnull %26), !noalias !6214
  store ptr %28, ptr %26, align 8, !noalias !6214
  %842 = getelementptr inbounds nuw i8, ptr %26, i64 8
  store ptr @<usize as core::fmt::Debug>::fmt, ptr %842, align 8, !noalias !6214
  %843 = getelementptr inbounds nuw i8, ptr %26, i64 16
  store ptr %27, ptr %843, align 8, !noalias !6214
  %844 = getelementptr inbounds nuw i8, ptr %26, i64 24
  store ptr @<usize as core::fmt::Debug>::fmt, ptr %844, align 8, !noalias !6214
; invoke core::panicking::panic_fmt
  invoke void @core::panicking::panic_fmt(ptr noundef nonnull @anon.68dd637f94a7f528fe69f6876e3d956b.2156, ptr noundef nonnull %26, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.68dd637f94a7f528fe69f6876e3d956b.288) #88
          to label %845 unwind label %720, !noalias !6236

845:                                              ; preds = %841
  unreachable

846:                                              ; preds = %727
  call void @llvm.lifetime.start.p0(ptr nonnull %25), !noalias !6360
  %847 = load atomic i64, ptr %520 monotonic, align 8, !noalias !6367
  br label %848

848:                                              ; preds = %848, %846
  %849 = phi i64 [ %847, %846 ], [ %853, %848 ]
  %850 = call i64 @llvm.uadd.sat.i64(i64 %849, i64 %735)
  %851 = cmpxchg weak ptr %520, i64 %849, i64 %850 monotonic monotonic, align 8, !noalias !6367
  %852 = extractvalue { i64, i1 } %851, 1
  %853 = extractvalue { i64, i1 } %851, 0
  br i1 %852, label %854, label %848

854:                                              ; preds = %848
  %855 = call i64 @llvm.uadd.sat.i64(i64 %853, i64 %735)
  %856 = load i64, ptr %481, align 8, !noalias !6367
  %857 = icmp ugt i64 %855, %856
  br i1 %857, label %858, label %862

858:                                              ; preds = %854
  call void @llvm.lifetime.start.p0(ptr nonnull %24), !noalias !6370
  store i8 0, ptr %521, align 1, !noalias !6370
  store i64 %856, ptr %522, align 8, !noalias !6370
  store i64 %855, ptr %523, align 8, !noalias !6370
  store i8 0, ptr %24, align 8, !noalias !6370
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %25, ptr noundef nonnull align 8 %481, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %24)
          to label %859 unwind label %716, !noalias !6236

859:                                              ; preds = %858
  call void @llvm.lifetime.end.p0(ptr nonnull %24), !noalias !6370
  %860 = load i8, ptr %25, align 8, !noalias !6360
  %861 = icmp eq i8 %860, -1
  br i1 %861, label %862, label %865

862:                                              ; preds = %859, %854
  call void @llvm.lifetime.end.p0(ptr nonnull %25), !noalias !6360
  %863 = load ptr, ptr %524, align 16, !alias.scope !6235, !noalias !6236, !noundef !1733
  %864 = icmp eq ptr %863, null
  br i1 %864, label %839, label %866

865:                                              ; preds = %859
  call void @llvm.lifetime.end.p0(ptr nonnull %25), !noalias !6360
  br label %.loopexit167

866:                                              ; preds = %862
  %867 = load i32, ptr %525, align 4, !alias.scope !6235, !noalias !6236, !noundef !1733
  %868 = getelementptr i8, ptr %863, i64 56
  %869 = load i64, ptr %868, align 8, !noalias !6236, !noundef !1733
  %870 = zext i32 %867 to i64
  %871 = icmp ugt i64 %869, %870
  br i1 %871, label %872, label %839

872:                                              ; preds = %866
  %873 = getelementptr i8, ptr %863, i64 48
  %874 = load ptr, ptr %873, align 8, !noalias !6236, !nonnull !1733, !noundef !1733
  %875 = getelementptr inbounds nuw [136 x i8], ptr %874, i64 %870
  %876 = getelementptr inbounds nuw [8 x i8], ptr %875, i64 %526
  %877 = atomicrmw add ptr %876, i64 %735 monotonic, align 8, !noalias !6236
  br label %839

878:                                              ; preds = %839
  %879 = icmp ult i64 %737, %730
  %880 = call i64 @llvm.umin.i64(i64 %737, i64 range(i64 0, 384307168202282326) %604)
  %881 = select i1 %879, i64 %730, i64 %880
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %603) ]
  %882 = icmp samesign ult i64 %881, %730
  br i1 %882, label %883, label %884, !prof !6111

883:                                              ; preds = %878
; invoke core::slice::index::slice_index_fail
  invoke void @core::slice::index::slice_index_fail(i64 noundef %730, i64 noundef %881, i64 noundef %604, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.68dd637f94a7f528fe69f6876e3d956b.289) #93
          to label %1402 unwind label %720, !noalias !6236

884:                                              ; preds = %878
  %885 = mul nuw nsw i64 %730, 24
  %886 = getelementptr inbounds nuw i8, ptr %603, i64 %885
  %887 = mul nuw nsw i64 %881, 24
  %888 = getelementptr inbounds nuw i8, ptr %603, i64 %887
  %889 = icmp eq i64 %730, %881
  br i1 %889, label %.loopexit179, label %890

890:                                              ; preds = %884
  %891 = sub nuw nsw i64 %887, %885
  %892 = udiv exact i64 %891, 24
  br label %893

893:                                              ; preds = %908, %890
  %894 = phi i64 [ 0, %890 ], [ %909, %908 ]
  %895 = phi i64 [ 0, %890 ], [ %910, %908 ]
  %896 = phi i64 [ 0, %890 ], [ %911, %908 ]
  %897 = phi i64 [ 0, %890 ], [ %912, %908 ]
  %898 = getelementptr inbounds nuw [24 x i8], ptr %886, i64 %897
  %899 = load i8, ptr %898, align 8, !range !6371, !noalias !6372, !noundef !1733
  %900 = getelementptr i8, ptr %898, i64 8
  %901 = load i64, ptr %900, align 8, !noalias !6372
  switch i8 %899, label %.unreachabledefault [
    i8 0, label %902
    i8 1, label %904
    i8 2, label %908
    i8 3, label %906
  ]

.unreachabledefault:                              ; preds = %893
  unreachable

default.unreachable874:                           ; preds = %.preheader166
  unreachable

902:                                              ; preds = %893
  %903 = call i64 @llvm.uadd.sat.i64(i64 %896, i64 %901)
  br label %908

904:                                              ; preds = %893
  %905 = call i64 @llvm.uadd.sat.i64(i64 %895, i64 %901)
  br label %908

906:                                              ; preds = %893
  %907 = call i64 @llvm.umax.i64(i64 %894, i64 %901)
  br label %908

908:                                              ; preds = %906, %904, %902, %893
  %909 = phi i64 [ %894, %902 ], [ %894, %904 ], [ %907, %906 ], [ %894, %893 ]
  %910 = phi i64 [ %895, %902 ], [ %905, %904 ], [ %895, %906 ], [ %895, %893 ]
  %911 = phi i64 [ %903, %902 ], [ %896, %904 ], [ %896, %906 ], [ %896, %893 ]
  %912 = add nuw i64 %897, 1
  %913 = icmp eq i64 %912, %892
  br i1 %913, label %.loopexit179, label %893

.loopexit179:                                     ; preds = %908, %884
  %914 = phi i64 [ 0, %884 ], [ %911, %908 ]
  %915 = phi i64 [ 0, %884 ], [ %910, %908 ]
  %916 = phi i64 [ 0, %884 ], [ %909, %908 ]
  br i1 %486, label %920, label %.loopexit177

.loopexit177:                                     ; preds = %932, %920, %.loopexit179
  %917 = phi i64 [ 0, %.loopexit179 ], [ 0, %920 ], [ %934, %932 ]
  %918 = load atomic i32, ptr %527 acquire, align 8, !noalias !6376
  %919 = icmp eq i32 %918, 0
  br i1 %919, label %936, label %939

920:                                              ; preds = %.loopexit179
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %732) ]
  %921 = ptrtoint ptr %732 to i64
  %922 = call i64 @llvm.usub.sat.i64(i64 %739, i64 %733)
  %923 = sub nuw i64 %619, %921
  %924 = udiv exact i64 %923, 88
  %925 = call i64 @llvm.umin.i64(i64 %922, i64 %924)
  %926 = icmp eq i64 %925, 0
  br i1 %926, label %.loopexit177, label %.preheader176

.preheader176:                                    ; preds = %920, %932
  %927 = phi i64 [ %934, %932 ], [ 0, %920 ]
  %928 = phi i64 [ %933, %932 ], [ 0, %920 ]
  %929 = getelementptr inbounds nuw [88 x i8], ptr %732, i64 %928
  %930 = getelementptr inbounds nuw i8, ptr %929, i64 8
; invoke purrdf_sparql_eval::scratch::value_bytes
  %931 = invoke noundef i64 @purrdf_sparql_eval::scratch::value_bytes(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(80) %930)
          to label %932 unwind label %714, !noalias !6236

932:                                              ; preds = %.preheader176
  %933 = add nuw nsw i64 %928, 1
  %934 = call noundef range(i64 32, 0) i64 @llvm.uadd.sat.i64(i64 %927, i64 %931)
  %935 = icmp eq i64 %933, %925
  br i1 %935, label %.loopexit177, label %.preheader176

936:                                              ; preds = %.loopexit177
  %937 = load i8, ptr %528, align 8, !noalias !6236
  %938 = icmp eq i8 %937, -1
  br i1 %938, label %939, label %946

939:                                              ; preds = %936, %.loopexit177
  br i1 %483, label %940, label %941

940:                                              ; preds = %941, %939
  br i1 %486, label %949, label %947

941:                                              ; preds = %939
  %942 = load atomic i64, ptr %520 monotonic, align 8, !noalias !6379
  %943 = load i64, ptr %481, align 8, !noalias !6379
  %944 = call i64 @llvm.uadd.sat.i64(i64 %942, i64 %914)
  %945 = icmp ugt i64 %944, %943
  br i1 %945, label %946, label %940

946:                                              ; preds = %949, %941, %936
  br i1 %889, label %.loopexit168, label %.preheader166

947:                                              ; preds = %949, %940
  %948 = or i1 %483, %889
  br i1 %948, label %.loopexit175, label %.preheader174

949:                                              ; preds = %940
  %950 = load i64, ptr %529, align 16, !alias.scope !6235, !noalias !6236, !noundef !1733
  %951 = load atomic i64, ptr %530 monotonic, align 8, !alias.scope !6235, !noalias !6236
  %952 = call noundef i64 @llvm.usub.sat.i64(i64 %950, i64 %951)
  %953 = call i64 @llvm.uadd.sat.i64(i64 %952, i64 %917)
  %954 = call i64 @llvm.uadd.sat.i64(i64 %953, i64 %915)
  %955 = call i64 @llvm.uadd.sat.i64(i64 %954, i64 %916)
  %956 = load atomic i64, ptr %531 monotonic, align 8, !noalias !6382
  %957 = load i64, ptr %484, align 8, !noalias !6382
  %958 = call i64 @llvm.uadd.sat.i64(i64 %956, i64 %955)
  %959 = icmp ugt i64 %958, %957
  br i1 %959, label %946, label %947

.preheader166:                                    ; preds = %946, %1142
  %960 = phi ptr [ %1143, %1142 ], [ %729, %946 ]
  %961 = phi ptr [ %965, %1142 ], [ %886, %946 ]
  %962 = phi ptr [ %1146, %1142 ], [ %732, %946 ]
  %963 = phi i64 [ %1145, %1142 ], [ %733, %946 ]
  %964 = phi ptr [ %1144, %1142 ], [ %729, %946 ]
  %965 = getelementptr inbounds nuw i8, ptr %961, i64 24
  %966 = load i8, ptr %961, align 8, !range !6371, !noalias !6236, !noundef !1733
  switch i8 %966, label %default.unreachable874 [
    i8 0, label %972
    i8 1, label %979
    i8 2, label %984
    i8 3, label %1007
  ]

.loopexit168:                                     ; preds = %1142, %946
  %967 = phi ptr [ %729, %946 ], [ %1143, %1142 ]
  %968 = phi i64 [ %733, %946 ], [ %1145, %1142 ]
  %969 = phi ptr [ %732, %946 ], [ %1146, %1142 ]
  %970 = icmp ult i64 %968, %739
  %971 = select i1 %486, i1 %970, i1 false
  br i1 %971, label %1162, label %1155

972:                                              ; preds = %.preheader166
  %973 = getelementptr inbounds nuw i8, ptr %961, i64 1
  %974 = load i8, ptr %973, align 1, !range !1734, !noalias !6236, !noundef !1733
  %975 = getelementptr inbounds nuw i8, ptr %961, i64 8
  %976 = load i64, ptr %975, align 8, !noalias !6236, !noundef !1733
  %977 = getelementptr inbounds nuw i8, ptr %961, i64 16
  %978 = load i64, ptr %977, align 8, !noalias !6236, !noundef !1733
  br i1 %483, label %1142, label %1008

979:                                              ; preds = %.preheader166
  %980 = getelementptr inbounds nuw i8, ptr %961, i64 8
  %981 = load i64, ptr %980, align 8, !noalias !6236, !noundef !1733
  %982 = getelementptr inbounds nuw i8, ptr %961, i64 16
  %983 = load i64, ptr %982, align 8, !noalias !6236, !noundef !1733
  br i1 %486, label %1067, label %1142

984:                                              ; preds = %.preheader166
  %985 = getelementptr inbounds nuw i8, ptr %961, i64 8
  %986 = load i64, ptr %985, align 8, !noalias !6236, !noundef !1733
  %987 = icmp ult i64 %963, %986
  br i1 %987, label %988, label %1119

988:                                              ; preds = %984
  %989 = icmp eq ptr %962, %608
  br i1 %989, label %.loopexit160, label %990

990:                                              ; preds = %988
  %991 = add i64 %986, -1
  br label %995

992:                                              ; preds = %1005
  %993 = add i64 %997, 1
  %994 = icmp eq ptr %998, %608
  br i1 %994, label %.loopexit160, label %995

995:                                              ; preds = %992, %990
  %996 = phi ptr [ %998, %992 ], [ %962, %990 ]
  %997 = phi i64 [ %993, %992 ], [ %963, %990 ]
  %998 = getelementptr inbounds nuw i8, ptr %996, i64 88
  %999 = getelementptr inbounds nuw i8, ptr %996, i64 8
  %1000 = load i64, ptr %999, align 8, !noalias !6385
  %1001 = icmp eq i64 %1000, -1
  br i1 %1001, label %.loopexit160, label %1002

1002:                                             ; preds = %995
  %1003 = getelementptr inbounds nuw i8, ptr %996, i64 16
  %1004 = load i64, ptr %996, align 8, !noalias !6385
  call void @llvm.lifetime.start.p0(ptr nonnull %23), !noalias !6388
  store i64 %1000, ptr %23, align 8, !noalias !6388
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %535, ptr noundef nonnull align 8 dereferenceable(72) %1003, i64 72, i1 false), !noalias !6236
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %536, i64 noundef %1004, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %23)
          to label %1005 unwind label %702, !noalias !6236

1005:                                             ; preds = %1002
  call void @llvm.lifetime.end.p0(ptr nonnull %23), !noalias !6388
  %1006 = icmp eq i64 %997, %991
  br i1 %1006, label %.loopexit160, label %992

1007:                                             ; preds = %.preheader166
  br i1 %486, label %1128, label %1142

1008:                                             ; preds = %972
  call void @llvm.lifetime.start.p0(ptr nonnull %22), !noalias !6391
  %1009 = load atomic i64, ptr %520 monotonic, align 8, !noalias !6398
  br label %1010

1010:                                             ; preds = %1010, %1008
  %1011 = phi i64 [ %1009, %1008 ], [ %1015, %1010 ]
  %1012 = call i64 @llvm.uadd.sat.i64(i64 %1011, i64 %976)
  %1013 = cmpxchg weak ptr %520, i64 %1011, i64 %1012 monotonic monotonic, align 8, !noalias !6398
  %1014 = extractvalue { i64, i1 } %1013, 1
  %1015 = extractvalue { i64, i1 } %1013, 0
  br i1 %1014, label %1016, label %1010

1016:                                             ; preds = %1010
  %1017 = call i64 @llvm.uadd.sat.i64(i64 %1015, i64 %976)
  %1018 = load i64, ptr %481, align 8, !noalias !6398
  %1019 = icmp ugt i64 %1017, %1018
  br i1 %1019, label %1020, label %1029

1020:                                             ; preds = %1016
  call void @llvm.lifetime.start.p0(ptr nonnull %21), !noalias !6401
  store i8 0, ptr %541, align 1, !noalias !6401
  store i64 %1018, ptr %542, align 8, !noalias !6401
  store i64 %1017, ptr %543, align 8, !noalias !6401
  store i8 0, ptr %21, align 8, !noalias !6401
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %22, ptr noundef nonnull align 8 %481, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %21)
          to label %1021 unwind label %698, !noalias !6236

1021:                                             ; preds = %1020
  call void @llvm.lifetime.end.p0(ptr nonnull %21), !noalias !6401
  %1022 = load i8, ptr %22, align 8, !noalias !6391
  %1023 = icmp eq i8 %1022, -1
  br i1 %1023, label %1029, label %1024

1024:                                             ; preds = %1021
  call void @llvm.lifetime.end.p0(ptr nonnull %22), !noalias !6391
  %1025 = icmp ne i64 %978, 0
  %1026 = add i64 %978, -1
  %1027 = icmp ult i64 %963, %1026
  %1028 = select i1 %1025, i1 %1027, i1 false
  br i1 %1028, label %1031, label %.loopexit167

1029:                                             ; preds = %1021, %1016
  call void @llvm.lifetime.end.p0(ptr nonnull %22), !noalias !6391
  %1030 = icmp eq i8 %974, -1
  br i1 %1030, label %1142, label %1050

1031:                                             ; preds = %1024
  %1032 = icmp eq ptr %962, %608
  br i1 %1032, label %.loopexit158, label %1033

1033:                                             ; preds = %1031
  %1034 = add i64 %978, -2
  br label %1035

1035:                                             ; preds = %1045, %1033
  %1036 = phi ptr [ %1038, %1045 ], [ %962, %1033 ]
  %1037 = phi i64 [ %1047, %1045 ], [ %963, %1033 ]
  %1038 = getelementptr inbounds nuw i8, ptr %1036, i64 88
  %1039 = getelementptr inbounds nuw i8, ptr %1036, i64 8
  %1040 = load i64, ptr %1039, align 8, !noalias !6402
  %1041 = icmp eq i64 %1040, -1
  br i1 %1041, label %.loopexit158, label %1042

1042:                                             ; preds = %1035
  %1043 = getelementptr inbounds nuw i8, ptr %1036, i64 16
  %1044 = load i64, ptr %1036, align 8, !noalias !6402
  call void @llvm.lifetime.start.p0(ptr nonnull %20), !noalias !6405
  store i64 %1040, ptr %20, align 8, !noalias !6405
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %544, ptr noundef nonnull align 8 dereferenceable(72) %1043, i64 72, i1 false), !noalias !6236
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %536, i64 noundef %1044, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %20)
          to label %1045 unwind label %696, !noalias !6236

1045:                                             ; preds = %1042
  call void @llvm.lifetime.end.p0(ptr nonnull %20), !noalias !6405
  %1046 = icmp eq i64 %1037, %1034
  %1047 = add nuw i64 %1037, 1
  %1048 = icmp eq ptr %1038, %608
  %1049 = select i1 %1046, i1 true, i1 %1048
  br i1 %1049, label %.loopexit158, label %1035

1050:                                             ; preds = %1029
  %1051 = load ptr, ptr %524, align 16, !alias.scope !6235, !noalias !6236, !noundef !1733
  %1052 = icmp eq ptr %1051, null
  br i1 %1052, label %1142, label %1053

1053:                                             ; preds = %1050
  %1054 = load i32, ptr %525, align 4, !alias.scope !6235, !noalias !6236, !noundef !1733
  %1055 = getelementptr i8, ptr %1051, i64 56
  %1056 = load i64, ptr %1055, align 8, !noalias !6236, !noundef !1733
  %1057 = zext i32 %1054 to i64
  %1058 = icmp ugt i64 %1056, %1057
  br i1 %1058, label %1059, label %1142

1059:                                             ; preds = %1053
  %1060 = getelementptr i8, ptr %1051, i64 48
  %1061 = load ptr, ptr %1060, align 8, !noalias !6236, !nonnull !1733, !noundef !1733
  %1062 = zext nneg i8 %974 to i64
  %1063 = getelementptr inbounds nuw [136 x i8], ptr %1061, i64 %1057
  %1064 = getelementptr inbounds nuw [8 x i8], ptr %1063, i64 %1062
  %1065 = atomicrmw add ptr %1064, i64 %976 monotonic, align 8, !noalias !6236
  br label %1142

1066:                                             ; preds = %1071
  br i1 %1073, label %.loopexit167, label %1142

1067:                                             ; preds = %979
  call void @llvm.lifetime.start.p0(ptr nonnull %32), !noalias !6214
  %1068 = load i64, ptr %484, align 8, !noalias !6236
  %1069 = icmp eq i64 %1068, -1
  br i1 %1069, label %1070, label %1076

1070:                                             ; preds = %1087, %1067
  call void @llvm.lifetime.end.p0(ptr nonnull %32), !noalias !6214
  br label %1142

1071:                                             ; preds = %1093, %1091
  %1072 = load i8, ptr %32, align 8, !noalias !6214
  %1073 = icmp ne i8 %1072, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %32), !noalias !6214
  %1074 = icmp ne i64 %983, 0
  %1075 = and i1 %1074, %1073
  br i1 %1075, label %1094, label %1066

1076:                                             ; preds = %1067
  %1077 = load atomic i32, ptr %527 acquire, align 8, !noalias !6408
  %1078 = icmp eq i32 %1077, 0
  br i1 %1078, label %1091, label %1079

1079:                                             ; preds = %1076
  %1080 = load atomic i64, ptr %531 monotonic, align 8, !noalias !6408
  br label %1081

1081:                                             ; preds = %1081, %1079
  %1082 = phi i64 [ %1080, %1079 ], [ %1086, %1081 ]
  %1083 = call i64 @llvm.uadd.sat.i64(i64 %1082, i64 %981)
  %1084 = cmpxchg weak ptr %531, i64 %1082, i64 %1083 monotonic monotonic, align 8, !noalias !6408
  %1085 = extractvalue { i64, i1 } %1084, 1
  %1086 = extractvalue { i64, i1 } %1084, 0
  br i1 %1085, label %1087, label %1081

1087:                                             ; preds = %1081
  %1088 = call i64 @llvm.uadd.sat.i64(i64 %1086, i64 %981)
  %1089 = load i64, ptr %484, align 8, !noalias !6408
  %1090 = icmp ugt i64 %1088, %1089
  br i1 %1090, label %1092, label %1070

1091:                                             ; preds = %1076
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %32, ptr noundef nonnull align 8 dereferenceable(24) %528, i64 24, i1 false), !noalias !6236
  br label %1071

1092:                                             ; preds = %1087
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !6411
  store i8 3, ptr %537, align 1, !noalias !6411
  store i64 %1089, ptr %538, align 8, !noalias !6411
  store i64 %1088, ptr %539, align 8, !noalias !6411
  store i8 0, ptr %12, align 8, !noalias !6411
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %32, ptr noundef nonnull align 8 %481, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %12)
          to label %1093 unwind label %712, !noalias !6236

1093:                                             ; preds = %1092
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !6411
  br label %1071

1094:                                             ; preds = %1071
  %1095 = add i64 %983, -1
  %1096 = icmp ult i64 %963, %1095
  br i1 %1096, label %1097, label %.loopexit167

1097:                                             ; preds = %1094
  %1098 = icmp eq ptr %962, %608
  br i1 %1098, label %.loopexit158, label %1099

1099:                                             ; preds = %1097
  %1100 = add i64 %983, -2
  br label %1101

1101:                                             ; preds = %1111, %1099
  %1102 = phi ptr [ %1104, %1111 ], [ %962, %1099 ]
  %1103 = phi i64 [ %1113, %1111 ], [ %963, %1099 ]
  %1104 = getelementptr inbounds nuw i8, ptr %1102, i64 88
  %1105 = getelementptr inbounds nuw i8, ptr %1102, i64 8
  %1106 = load i64, ptr %1105, align 8, !noalias !6412
  %1107 = icmp eq i64 %1106, -1
  br i1 %1107, label %.loopexit158, label %1108

1108:                                             ; preds = %1101
  %1109 = getelementptr inbounds nuw i8, ptr %1102, i64 16
  %1110 = load i64, ptr %1102, align 8, !noalias !6412
  call void @llvm.lifetime.start.p0(ptr nonnull %19), !noalias !6415
  store i64 %1106, ptr %19, align 8, !noalias !6415
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %540, ptr noundef nonnull align 8 dereferenceable(72) %1109, i64 72, i1 false), !noalias !6236
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %536, i64 noundef %1110, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %19)
          to label %1111 unwind label %700, !noalias !6236

1111:                                             ; preds = %1108
  call void @llvm.lifetime.end.p0(ptr nonnull %19), !noalias !6415
  %1112 = icmp eq i64 %1103, %1100
  %1113 = add nuw i64 %1103, 1
  %1114 = icmp eq ptr %1104, %608
  %1115 = select i1 %1112, i1 true, i1 %1114
  br i1 %1115, label %.loopexit158, label %1101

.loopexit160:                                     ; preds = %1005, %995, %992, %988
  %1116 = phi ptr [ %964, %988 ], [ %998, %992 ], [ %998, %995 ], [ %998, %1005 ]
  %1117 = phi i64 [ %963, %988 ], [ %986, %1005 ], [ %997, %995 ], [ %993, %992 ]
  %1118 = phi ptr [ %962, %988 ], [ %998, %992 ], [ %998, %995 ], [ %998, %1005 ]
  store ptr %1116, ptr %514, align 8, !noalias !6214
  br label %1119

1119:                                             ; preds = %.loopexit160, %984
  %1120 = phi ptr [ %960, %984 ], [ %1116, %.loopexit160 ]
  %1121 = phi ptr [ %964, %984 ], [ %1116, %.loopexit160 ]
  %1122 = phi i64 [ %963, %984 ], [ %1117, %.loopexit160 ]
  %1123 = phi ptr [ %962, %984 ], [ %1118, %.loopexit160 ]
  br i1 %486, label %1124, label %1142

1124:                                             ; preds = %1119
  call void @llvm.lifetime.start.p0(ptr nonnull %31), !noalias !6214
; invoke <purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
  invoke fastcc void @<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %31, ptr noundef nonnull align 16 dereferenceable(1232) %2)
          to label %1125 unwind label %712, !noalias !6236

1125:                                             ; preds = %1124
  %1126 = load i8, ptr %31, align 8, !range !1736, !noalias !6214, !noundef !1733
  %1127 = icmp eq i8 %1126, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %31), !noalias !6214
  br i1 %1127, label %1142, label %.loopexit167

1128:                                             ; preds = %1007
  %1129 = getelementptr inbounds nuw i8, ptr %961, i64 8
  %1130 = load i64, ptr %1129, align 8, !noalias !6236, !noundef !1733
  call void @llvm.lifetime.start.p0(ptr nonnull %30), !noalias !6214
  %1131 = load atomic i32, ptr %527 acquire, align 8, !noalias !6418
  %1132 = icmp eq i32 %1131, 0
  br i1 %1132, label %1133, label %1134

1133:                                             ; preds = %1128
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %30, ptr noundef nonnull align 8 dereferenceable(24) %528, i64 24, i1 false), !noalias !6236
  br label %1148

1134:                                             ; preds = %1128
  %1135 = load atomic i64, ptr %531 monotonic, align 8, !noalias !6418
  %1136 = call i64 @llvm.uadd.sat.i64(i64 %1135, i64 %1130)
  %1137 = load i64, ptr %484, align 8, !noalias !6418
  %1138 = icmp ugt i64 %1136, %1137
  br i1 %1138, label %1140, label %1139

1139:                                             ; preds = %1134
  call void @llvm.lifetime.end.p0(ptr nonnull %30), !noalias !6214
  br label %1142

1140:                                             ; preds = %1134
  call void @llvm.lifetime.start.p0(ptr nonnull %18), !noalias !6421
  store i8 3, ptr %532, align 1, !noalias !6421
  store i64 %1137, ptr %533, align 8, !noalias !6421
  store i64 %1136, ptr %534, align 8, !noalias !6421
  store i8 0, ptr %18, align 8, !noalias !6421
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)(ptr noalias nofree noundef align 8 captures(none) dereferenceable(24) %30, ptr noundef nonnull align 8 %481, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %18)
          to label %1141 unwind label %712, !noalias !6236

1141:                                             ; preds = %1140
  call void @llvm.lifetime.end.p0(ptr nonnull %18), !noalias !6421
  br label %1148

1142:                                             ; preds = %1148, %1139, %1125, %1119, %1070, %1066, %1059, %1053, %1050, %1029, %1007, %979, %972
  %1143 = phi ptr [ %960, %1066 ], [ %1120, %1119 ], [ %960, %1007 ], [ %960, %972 ], [ %960, %979 ], [ %960, %1139 ], [ %1120, %1125 ], [ %960, %1148 ], [ %960, %1029 ], [ %960, %1050 ], [ %960, %1059 ], [ %960, %1053 ], [ %960, %1070 ]
  %1144 = phi ptr [ %964, %1066 ], [ %1121, %1119 ], [ %964, %1007 ], [ %964, %972 ], [ %964, %979 ], [ %964, %1139 ], [ %1121, %1125 ], [ %964, %1148 ], [ %964, %1029 ], [ %964, %1050 ], [ %964, %1059 ], [ %964, %1053 ], [ %964, %1070 ]
  %1145 = phi i64 [ %963, %1066 ], [ %1122, %1119 ], [ %963, %1007 ], [ %963, %972 ], [ %963, %979 ], [ %963, %1139 ], [ %1122, %1125 ], [ %963, %1148 ], [ %963, %1029 ], [ %963, %1050 ], [ %963, %1059 ], [ %963, %1053 ], [ %963, %1070 ]
  %1146 = phi ptr [ %962, %1066 ], [ %1123, %1119 ], [ %962, %1007 ], [ %962, %972 ], [ %962, %979 ], [ %962, %1139 ], [ %1123, %1125 ], [ %962, %1148 ], [ %962, %1029 ], [ %962, %1050 ], [ %962, %1059 ], [ %962, %1053 ], [ %962, %1070 ]
  %1147 = icmp eq ptr %965, %888
  br i1 %1147, label %.loopexit168, label %.preheader166

1148:                                             ; preds = %1141, %1133
  %1149 = load i8, ptr %30, align 8, !noalias !6214
  %1150 = icmp eq i8 %1149, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %30), !noalias !6214
  br i1 %1150, label %1142, label %.loopexit167

.loopexit158:                                     ; preds = %1111, %1101, %1045, %1035, %1097, %1031
  %1151 = phi ptr [ %964, %1097 ], [ %964, %1031 ], [ %1038, %1045 ], [ %1038, %1035 ], [ %1104, %1101 ], [ %1104, %1111 ]
  store ptr %1151, ptr %514, align 8, !noalias !6214
  br label %.loopexit167

.loopexit164:                                     ; preds = %1179, %1169, %1166, %1162
  %1152 = phi ptr [ %967, %1162 ], [ %1172, %1166 ], [ %1172, %1169 ], [ %1172, %1179 ]
  %1153 = phi i64 [ %968, %1162 ], [ %739, %1179 ], [ %1171, %1169 ], [ %1167, %1166 ]
  %1154 = phi ptr [ %969, %1162 ], [ %1172, %1166 ], [ %1172, %1169 ], [ %1172, %1179 ]
  store ptr %1152, ptr %514, align 8, !noalias !6214
  br label %1155

1155:                                             ; preds = %1273, %1248, %.loopexit164, %.loopexit168
  %1156 = phi ptr [ %1243, %1248 ], [ %1274, %1273 ], [ %967, %.loopexit168 ], [ %1152, %.loopexit164 ]
  %1157 = phi i64 [ %1244, %1248 ], [ %1275, %1273 ], [ %968, %.loopexit168 ], [ %1153, %.loopexit164 ]
  %1158 = phi ptr [ %1245, %1248 ], [ %1276, %1273 ], [ %969, %.loopexit168 ], [ %1154, %.loopexit164 ]
  %1159 = icmp eq i64 %741, 0
  br i1 %1159, label %.loopexit162, label %1160

1160:                                             ; preds = %1155
  %1161 = load ptr, ptr %350, align 8, !alias.scope !6422, !noalias !6425, !nonnull !1733, !noundef !1733
  br label %1340

1162:                                             ; preds = %.loopexit168
  %1163 = icmp eq ptr %969, %608
  br i1 %1163, label %.loopexit164, label %1164

1164:                                             ; preds = %1162
  %1165 = add i64 %739, -1
  br label %1169

1166:                                             ; preds = %1179
  %1167 = add i64 %1171, 1
  %1168 = icmp eq ptr %1172, %608
  br i1 %1168, label %.loopexit164, label %1169

1169:                                             ; preds = %1166, %1164
  %1170 = phi ptr [ %1172, %1166 ], [ %969, %1164 ]
  %1171 = phi i64 [ %1167, %1166 ], [ %968, %1164 ]
  %1172 = getelementptr inbounds nuw i8, ptr %1170, i64 88
  %1173 = getelementptr inbounds nuw i8, ptr %1170, i64 8
  %1174 = load i64, ptr %1173, align 8, !noalias !6427
  %1175 = icmp eq i64 %1174, -1
  br i1 %1175, label %.loopexit164, label %1176

1176:                                             ; preds = %1169
  %1177 = getelementptr inbounds nuw i8, ptr %1170, i64 16
  %1178 = load i64, ptr %1170, align 8, !noalias !6427
  call void @llvm.lifetime.start.p0(ptr nonnull %17), !noalias !6430
  store i64 %1174, ptr %17, align 8, !noalias !6430
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %545, ptr noundef nonnull align 8 dereferenceable(72) %1177, i64 72, i1 false), !noalias !6236
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %536, i64 noundef %1178, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %17)
          to label %1179 unwind label %710, !noalias !6236

1179:                                             ; preds = %1176
  call void @llvm.lifetime.end.p0(ptr nonnull %17), !noalias !6430
  %1180 = icmp eq i64 %1171, %1165
  br i1 %1180, label %.loopexit164, label %1166

.loopexit175:                                     ; preds = %1196, %947
  %1181 = icmp eq i64 %730, %881
  br i1 %1181, label %.loopexit173, label %.lr.ph

1182:                                             ; preds = %.lr.ph
  %1183 = icmp eq ptr %886, %1185
  br i1 %1183, label %.loopexit173, label %.lr.ph

.lr.ph:                                           ; preds = %.loopexit175, %1182
  %1184 = phi ptr [ %1185, %1182 ], [ %888, %.loopexit175 ]
  %1185 = getelementptr inbounds i8, ptr %1184, i64 -24
  %1186 = load i8, ptr %1185, align 8, !range !6371, !noalias !6433, !noundef !1733
  %1187 = icmp eq i8 %1186, 2
  br i1 %1187, label %1216, label %1182

.preheader174:                                    ; preds = %947, %1196
  %1188 = phi ptr [ %1189, %1196 ], [ %886, %947 ]
  %1189 = getelementptr inbounds nuw i8, ptr %1188, i64 24
  %1190 = load i8, ptr %1188, align 8, !range !6371, !noalias !6236, !noundef !1733
  %1191 = icmp eq i8 %1190, 0
  br i1 %1191, label %1192, label %1196

1192:                                             ; preds = %.preheader174
  %1193 = getelementptr inbounds nuw i8, ptr %1188, i64 1
  %1194 = load i8, ptr %1193, align 1, !range !1734, !noalias !6236, !noundef !1733
  %1195 = icmp eq i8 %1194, -1
  br i1 %1195, label %1196, label %1198

1196:                                             ; preds = %1207, %1201, %1198, %1192, %.preheader174
  %1197 = icmp eq ptr %1189, %888
  br i1 %1197, label %.loopexit175, label %.preheader174

1198:                                             ; preds = %1192
  %1199 = load ptr, ptr %524, align 16, !alias.scope !6235, !noalias !6236, !noundef !1733
  %1200 = icmp eq ptr %1199, null
  br i1 %1200, label %1196, label %1201

1201:                                             ; preds = %1198
  %1202 = load i32, ptr %525, align 4, !alias.scope !6235, !noalias !6236, !noundef !1733
  %1203 = getelementptr i8, ptr %1199, i64 56
  %1204 = load i64, ptr %1203, align 8, !noalias !6236, !noundef !1733
  %1205 = zext i32 %1202 to i64
  %1206 = icmp ugt i64 %1204, %1205
  br i1 %1206, label %1207, label %1196

1207:                                             ; preds = %1201
  %1208 = getelementptr i8, ptr %1199, i64 48
  %1209 = load ptr, ptr %1208, align 8, !noalias !6236, !nonnull !1733, !noundef !1733
  %1210 = getelementptr inbounds nuw i8, ptr %1188, i64 8
  %1211 = load i64, ptr %1210, align 8, !noalias !6236, !noundef !1733
  %1212 = zext nneg i8 %1194 to i64
  %1213 = getelementptr inbounds nuw [136 x i8], ptr %1209, i64 %1205
  %1214 = getelementptr inbounds nuw [8 x i8], ptr %1213, i64 %1212
  %1215 = atomicrmw add ptr %1214, i64 %1211 monotonic, align 8, !noalias !6236
  br label %1196

1216:                                             ; preds = %.lr.ph
  %1217 = getelementptr i8, ptr %1184, i64 -16
  %1218 = load i64, ptr %1217, align 8, !noalias !6433
  %1219 = icmp ult i64 %733, %1218
  br i1 %1219, label %1220, label %.loopexit173

1220:                                             ; preds = %1216
  %1221 = icmp eq ptr %732, %608
  br i1 %1221, label %.loopexit171, label %1222

1222:                                             ; preds = %1220
  %1223 = add i64 %1218, -1
  br label %1227

1224:                                             ; preds = %1237
  %1225 = add i64 %1229, 1
  %1226 = icmp eq ptr %1230, %608
  br i1 %1226, label %.loopexit171, label %1227

1227:                                             ; preds = %1224, %1222
  %1228 = phi ptr [ %1230, %1224 ], [ %732, %1222 ]
  %1229 = phi i64 [ %1225, %1224 ], [ %733, %1222 ]
  %1230 = getelementptr inbounds nuw i8, ptr %1228, i64 88
  %1231 = getelementptr inbounds nuw i8, ptr %1228, i64 8
  %1232 = load i64, ptr %1231, align 8, !noalias !6436
  %1233 = icmp eq i64 %1232, -1
  br i1 %1233, label %.loopexit171, label %1234

1234:                                             ; preds = %1227
  %1235 = getelementptr inbounds nuw i8, ptr %1228, i64 16
  %1236 = load i64, ptr %1228, align 8, !noalias !6436
  call void @llvm.lifetime.start.p0(ptr nonnull %16), !noalias !6439
  store i64 %1232, ptr %16, align 8, !noalias !6439
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %546, ptr noundef nonnull align 8 dereferenceable(72) %1235, i64 72, i1 false), !noalias !6236
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %536, i64 noundef %1236, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %16)
          to label %1237 unwind label %708, !noalias !6236

1237:                                             ; preds = %1234
  call void @llvm.lifetime.end.p0(ptr nonnull %16), !noalias !6439
  %1238 = icmp eq i64 %1229, %1223
  br i1 %1238, label %.loopexit171, label %1224

.loopexit171:                                     ; preds = %1237, %1227, %1224, %1220
  %1239 = phi ptr [ %729, %1220 ], [ %1230, %1224 ], [ %1230, %1227 ], [ %1230, %1237 ]
  %1240 = phi i64 [ %733, %1220 ], [ %1218, %1237 ], [ %1229, %1227 ], [ %1225, %1224 ]
  %1241 = phi ptr [ %732, %1220 ], [ %1230, %1224 ], [ %1230, %1227 ], [ %1230, %1237 ]
  store ptr %1239, ptr %514, align 8, !noalias !6214
  br label %.loopexit173

.loopexit173:                                     ; preds = %1182, %.loopexit175, %.loopexit171, %1216
  %1242 = phi i1 [ false, %.loopexit171 ], [ false, %1216 ], [ true, %.loopexit175 ], [ true, %1182 ]
  %1243 = phi ptr [ %1239, %.loopexit171 ], [ %729, %1216 ], [ %729, %.loopexit175 ], [ %729, %1182 ]
  %1244 = phi i64 [ %1240, %.loopexit171 ], [ %733, %1216 ], [ %733, %.loopexit175 ], [ %733, %1182 ]
  %1245 = phi ptr [ %1241, %.loopexit171 ], [ %732, %1216 ], [ %732, %.loopexit175 ], [ %732, %1182 ]
  %1246 = icmp eq i64 %914, 0
  %1247 = or i1 %483, %1246
  br i1 %1247, label %1248, label %1249

1248:                                             ; preds = %1265, %.loopexit173
  br i1 %486, label %1267, label %1155

1249:                                             ; preds = %.loopexit173
  call void @llvm.lifetime.start.p0(ptr nonnull %15), !noalias !6442
  %1250 = load atomic i64, ptr %520 monotonic, align 8, !noalias !6449
  br label %1251

1251:                                             ; preds = %1251, %1249
  %1252 = phi i64 [ %1250, %1249 ], [ %1256, %1251 ]
  %1253 = call i64 @llvm.uadd.sat.i64(i64 %1252, i64 %914)
  %1254 = cmpxchg weak ptr %520, i64 %1252, i64 %1253 monotonic monotonic, align 8, !noalias !6449
  %1255 = extractvalue { i64, i1 } %1254, 1
  %1256 = extractvalue { i64, i1 } %1254, 0
  br i1 %1255, label %1257, label %1251

1257:                                             ; preds = %1251
  %1258 = call i64 @llvm.uadd.sat.i64(i64 %1256, i64 %914)
  %1259 = load i64, ptr %481, align 8, !noalias !6449
  %1260 = icmp ugt i64 %1258, %1259
  br i1 %1260, label %1261, label %1265

1261:                                             ; preds = %1257
  call void @llvm.lifetime.start.p0(ptr nonnull %14), !noalias !6452
  store i8 0, ptr %547, align 1, !noalias !6452
  store i64 %1259, ptr %548, align 8, !noalias !6452
  store i64 %1258, ptr %549, align 8, !noalias !6452
  store i8 0, ptr %14, align 8, !noalias !6452
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %15, ptr noundef nonnull align 8 %481, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %14)
          to label %1262 unwind label %706, !noalias !6236

1262:                                             ; preds = %1261
  call void @llvm.lifetime.end.p0(ptr nonnull %14), !noalias !6452
  %1263 = load i8, ptr %15, align 8, !noalias !6442
  %1264 = icmp eq i8 %1263, -1
  br i1 %1264, label %1265, label %1266

1265:                                             ; preds = %1262, %1257
  call void @llvm.lifetime.end.p0(ptr nonnull %15), !noalias !6442
  br label %1248

1266:                                             ; preds = %1262
  call void @llvm.lifetime.end.p0(ptr nonnull %15), !noalias !6442
  br i1 %486, label %1318, label %.loopexit167

1267:                                             ; preds = %1248
  call void @llvm.lifetime.start.p0(ptr nonnull %35), !noalias !6214
  %1268 = load i64, ptr %484, align 8, !noalias !6236
  %1269 = icmp eq i64 %1268, -1
  br i1 %1269, label %1295, label %1277

.loopexit169:                                     ; preds = %1338, %1328, %1325, %1321
  %1270 = phi ptr [ %1243, %1321 ], [ %1331, %1325 ], [ %1331, %1328 ], [ %1331, %1338 ]
  %1271 = phi i64 [ %1244, %1321 ], [ %739, %1338 ], [ %1330, %1328 ], [ %1326, %1325 ]
  %1272 = phi ptr [ %1245, %1321 ], [ %1331, %1325 ], [ %1331, %1328 ], [ %1331, %1338 ]
  store ptr %1270, ptr %514, align 8, !noalias !6214
  br label %1273

1273:                                             ; preds = %1318, %.loopexit169
  %1274 = phi ptr [ %1243, %1318 ], [ %1270, %.loopexit169 ]
  %1275 = phi i64 [ %1244, %1318 ], [ %1271, %.loopexit169 ]
  %1276 = phi ptr [ %1245, %1318 ], [ %1272, %.loopexit169 ]
  br i1 %1319, label %.loopexit167, label %1155

1277:                                             ; preds = %1267
  %1278 = load atomic i32, ptr %527 acquire, align 8, !noalias !6453
  %1279 = icmp eq i32 %1278, 0
  br i1 %1279, label %1292, label %1280

1280:                                             ; preds = %1277
  %1281 = load atomic i64, ptr %531 monotonic, align 8, !noalias !6453
  br label %1282

1282:                                             ; preds = %1282, %1280
  %1283 = phi i64 [ %1281, %1280 ], [ %1287, %1282 ]
  %1284 = call i64 @llvm.uadd.sat.i64(i64 %1283, i64 %915)
  %1285 = cmpxchg weak ptr %531, i64 %1283, i64 %1284 monotonic monotonic, align 8, !noalias !6453
  %1286 = extractvalue { i64, i1 } %1285, 1
  %1287 = extractvalue { i64, i1 } %1285, 0
  br i1 %1286, label %1288, label %1282

1288:                                             ; preds = %1282
  %1289 = call i64 @llvm.uadd.sat.i64(i64 %1287, i64 %915)
  %1290 = load i64, ptr %484, align 8, !noalias !6453
  %1291 = icmp ugt i64 %1289, %1290
  br i1 %1291, label %1293, label %1295

1292:                                             ; preds = %1277
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %35, ptr noundef nonnull align 8 dereferenceable(24) %528, i64 24, i1 false), !noalias !6236
  br label %1296

1293:                                             ; preds = %1288
  call void @llvm.lifetime.start.p0(ptr nonnull %9), !noalias !6456
  store i8 3, ptr %555, align 1, !noalias !6456
  store i64 %1290, ptr %556, align 8, !noalias !6456
  store i64 %1289, ptr %557, align 8, !noalias !6456
  store i8 0, ptr %9, align 8, !noalias !6456
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %35, ptr noundef nonnull align 8 %481, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %9)
          to label %1294 unwind label %718, !noalias !6177

1294:                                             ; preds = %1293
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !6456
  br label %1296

1295:                                             ; preds = %1296, %1288, %1267
  call void @llvm.lifetime.end.p0(ptr nonnull %35), !noalias !6214
  br i1 %1242, label %1300, label %1302

1296:                                             ; preds = %1294, %1292
  %1297 = load i8, ptr %35, align 8, !noalias !6214
  %1298 = icmp eq i8 %1297, -1
  br i1 %1298, label %1295, label %1299

1299:                                             ; preds = %1296
  call void @llvm.lifetime.end.p0(ptr nonnull %35), !noalias !6214
  br label %1318

1300:                                             ; preds = %1303, %1295
  %1301 = icmp eq i64 %916, 0
  br i1 %1301, label %1318, label %1306

1302:                                             ; preds = %1295
  call void @llvm.lifetime.start.p0(ptr nonnull %34), !noalias !6214
; invoke <purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
  invoke fastcc void @<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %34, ptr noundef nonnull align 16 dereferenceable(1232) %2)
          to label %1303 unwind label %718, !noalias !6236

1303:                                             ; preds = %1302
  %1304 = load i8, ptr %34, align 8, !range !1736, !noalias !6214, !noundef !1733
  %1305 = icmp eq i8 %1304, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %34), !noalias !6214
  br i1 %1305, label %1300, label %1318

1306:                                             ; preds = %1300
  call void @llvm.lifetime.start.p0(ptr nonnull %33), !noalias !6214
  call void @llvm.experimental.noalias.scope.decl(metadata !6457)
  %1307 = load atomic i32, ptr %527 acquire, align 8, !noalias !6460
  %1308 = icmp eq i32 %1307, 0
  br i1 %1308, label %1309, label %1310

1309:                                             ; preds = %1306
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %33, ptr noundef nonnull align 8 dereferenceable(24) %528, i64 24, i1 false), !noalias !6236
  br label %<purrdf_sparql_eval::governor::GovernorState>::admit_transient (.exit)

1310:                                             ; preds = %1306
  %1311 = load atomic i64, ptr %531 monotonic, align 8, !noalias !6460
  %1312 = call i64 @llvm.uadd.sat.i64(i64 %1311, i64 %916)
  %.sroa.3149.0.copyload = load i64, ptr %484, align 8, !noalias !6460
  %1313 = icmp ugt i64 %1312, %.sroa.3149.0.copyload
  br i1 %1313, label %1315, label %1314

1314:                                             ; preds = %1310
  store i8 -1, ptr %33, align 8, !alias.scope !6457, !noalias !6236
  br label %<purrdf_sparql_eval::governor::GovernorState>::admit_transient (.exit)

1315:                                             ; preds = %1310
  call void @llvm.lifetime.start.p0(ptr nonnull %6), !noalias !6460
  store i8 3, ptr %558, align 1, !noalias !6460
  store i64 %.sroa.3149.0.copyload, ptr %559, align 8, !noalias !6460
  store i64 %1312, ptr %560, align 8, !noalias !6460
  store i8 0, ptr %6, align 8, !noalias !6460
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %33, ptr noundef nonnull align 8 %481, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %6)
          to label %.noexc142 unwind label %718

.noexc142:                                        ; preds = %1315
  call void @llvm.lifetime.end.p0(ptr nonnull %6), !noalias !6460
  br label %<purrdf_sparql_eval::governor::GovernorState>::admit_transient (.exit)

<purrdf_sparql_eval::governor::GovernorState>::admit_transient (.exit): ; preds = %.noexc142, %1314, %1309
  %1316 = load i8, ptr %33, align 8, !range !1736, !noalias !6214, !noundef !1733
  %1317 = icmp ne i8 %1316, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %33), !noalias !6214
  br label %1318

1318:                                             ; preds = %<purrdf_sparql_eval::governor::GovernorState>::admit_transient (.exit), %1303, %1300, %1299, %1266
  %1319 = phi i1 [ false, %1300 ], [ %1317, %<purrdf_sparql_eval::governor::GovernorState>::admit_transient (.exit) ], [ true, %1266 ], [ true, %1299 ], [ true, %1303 ]
  %1320 = icmp ult i64 %1244, %739
  br i1 %1320, label %1321, label %1273

1321:                                             ; preds = %1318
  %1322 = icmp eq ptr %1245, %608
  br i1 %1322, label %.loopexit169, label %1323

1323:                                             ; preds = %1321
  %1324 = add i64 %739, -1
  br label %1328

1325:                                             ; preds = %1338
  %1326 = add i64 %1330, 1
  %1327 = icmp eq ptr %1331, %608
  br i1 %1327, label %.loopexit169, label %1328

1328:                                             ; preds = %1325, %1323
  %1329 = phi ptr [ %1331, %1325 ], [ %1245, %1323 ]
  %1330 = phi i64 [ %1326, %1325 ], [ %1244, %1323 ]
  %1331 = getelementptr inbounds nuw i8, ptr %1329, i64 88
  %1332 = getelementptr inbounds nuw i8, ptr %1329, i64 8
  %1333 = load i64, ptr %1332, align 8, !noalias !6461
  %1334 = icmp eq i64 %1333, -1
  br i1 %1334, label %.loopexit169, label %1335

1335:                                             ; preds = %1328
  %1336 = getelementptr inbounds nuw i8, ptr %1329, i64 16
  %1337 = load i64, ptr %1329, align 8, !noalias !6461
  call void @llvm.lifetime.start.p0(ptr nonnull %13), !noalias !6464
  store i64 %1333, ptr %13, align 8, !noalias !6464
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %550, ptr noundef nonnull align 8 dereferenceable(72) %1336, i64 72, i1 false), !noalias !6236
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %536, i64 noundef %1337, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %13)
          to label %1338 unwind label %704, !noalias !6236

1338:                                             ; preds = %1335
  call void @llvm.lifetime.end.p0(ptr nonnull %13), !noalias !6464
  %1339 = icmp eq i64 %1330, %1324
  br i1 %1339, label %.loopexit169, label %1325

1340:                                             ; preds = %1365, %1160
  %1341 = phi i64 [ %741, %1160 ], [ %1343, %1365 ]
  %1342 = phi ptr [ %728, %1160 ], [ %1348, %1365 ]
  %1343 = add i64 %1341, -1
  call void @llvm.experimental.noalias.scope.decl(metadata !6422)
  %1344 = icmp eq ptr %1342, %1161
  br i1 %1344, label %.loopexit162, label %1347

.loopexit162:                                     ; preds = %1365, %1347, %1340, %1155
  %1345 = phi ptr [ %728, %1155 ], [ %1348, %1347 ], [ %1348, %1365 ], [ %1342, %1340 ]
  store ptr %1345, ptr %349, align 8, !noalias !6214
  %1346 = icmp eq ptr %734, %614
  br i1 %1346, label %.loopexit181, label %727

1347:                                             ; preds = %1340
  %1348 = getelementptr inbounds nuw i8, ptr %1342, i64 40
  %1349 = load i64, ptr %1342, align 8, !noalias !6467
  %1350 = getelementptr inbounds nuw i8, ptr %1342, i64 8
  %1351 = load ptr, ptr %1350, align 8, !noalias !6467
  %1352 = icmp eq i64 %1349, 0
  br i1 %1352, label %.loopexit162, label %1353

1353:                                             ; preds = %1347
  %1354 = getelementptr inbounds nuw i8, ptr %1342, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %29, ptr noundef nonnull align 8 dereferenceable(24) %1354, i64 24, i1 false), !noalias !6236
  call void @llvm.experimental.noalias.scope.decl(metadata !6468)
  %1355 = load i64, ptr %551, align 8, !alias.scope !6468, !noalias !6471, !noundef !1733
  %1356 = load i64, ptr %46, align 8, !range !1828, !alias.scope !6468, !noalias !6471, !noundef !1733
  %1357 = icmp eq i64 %1355, %1356
  br i1 %1357, label %1358, label %1365

1358:                                             ; preds = %1353
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %46)
          to label %1365 unwind label %1359, !noalias !6473

1359:                                             ; preds = %1358
  %1360 = landingpad { ptr, i32 }
          cleanup
  store ptr %1348, ptr %349, align 8, !noalias !6214
  %1361 = icmp ugt i64 %1349, 5
  br i1 %1361, label %1362, label %722

1362:                                             ; preds = %1359
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1351) ]
  %1363 = shl i64 %1349, 3
  %1364 = add i64 %1363, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1351, i64 noundef %1364, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !6474
  br label %722

1365:                                             ; preds = %1358, %1353
  %1366 = load ptr, ptr %552, align 8, !alias.scope !6468, !noalias !6471, !nonnull !1733, !noundef !1733
  %1367 = getelementptr inbounds nuw [40 x i8], ptr %1366, i64 %1355
  store i64 %1349, ptr %1367, align 8, !noalias !6477
  %1368 = getelementptr inbounds nuw i8, ptr %1367, i64 8
  store ptr %1351, ptr %1368, align 8, !noalias !6477
  %1369 = getelementptr inbounds nuw i8, ptr %1367, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %1369, ptr noundef nonnull align 8 dereferenceable(24) %29, i64 24, i1 false), !noalias !6477
  %1370 = add i64 %1355, 1
  store i64 %1370, ptr %551, align 8, !alias.scope !6468, !noalias !6471
  %1371 = icmp eq i64 %1343, 0
  br i1 %1371, label %.loopexit162, label %1340

.loopexit167:                                     ; preds = %1273, %1266, %1148, %1125, %1066, %.loopexit158, %1094, %1024, %865
  %1372 = phi i8 [ 0, %865 ], [ 1, %1024 ], [ 1, %1094 ], [ 1, %.loopexit158 ], [ 1, %1148 ], [ 1, %1066 ], [ 1, %1125 ], [ 1, %1266 ], [ 1, %1273 ]
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %52, ptr noundef nonnull align 8 dereferenceable(24) %46, i64 24, i1 false), !noalias !6220
  %1373 = icmp eq i64 %610, 0
  br i1 %1373, label %1403, label %1374

1374:                                             ; preds = %.loopexit167
  %1375 = shl nuw i64 %610, 5
  %1376 = load i64, ptr %561, align 8, !noalias !6478, !noundef !1733
  %1377 = call i64 @llvm.umin.i64(i64 %1375, i64 9223372036854775807)
  %1378 = call i64 @llvm.ssub.sat.i64(i64 %1376, i64 %1377)
  store i64 %1378, ptr %561, align 8, !noalias !6478
  %1379 = load i64, ptr %562, align 8, !noalias !6478, !noundef !1733
  %1380 = icmp slt i64 %1378, %1379
  br i1 %1380, label %1381, label %.preheader1484

1381:                                             ; preds = %1374
  store i64 %1378, ptr %562, align 8, !noalias !6478
  br label %.preheader1484

.preheader1484:                                   ; preds = %1381, %1374
  br label %1382

1382:                                             ; preds = %.preheader1484, %1385
  %1383 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !6478
  %1384 = icmp slt i64 %1383, 0
  br i1 %1384, label %1385, label %__rustc::__rust_dealloc (.exit143)

1385:                                             ; preds = %1382
  %1386 = add nsw i64 %1383, 1
  %1387 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1383, i64 %1386 acq_rel acquire, align 8, !noalias !6478
  %1388 = extractvalue { i64, i1 } %1387, 1
  br i1 %1388, label %1389, label %1382

1389:                                             ; preds = %1385
  %1390 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1377 monotonic, align 8, !noalias !6478
  %1391 = call i64 @llvm.ssub.sat.i64(i64 %1390, i64 %1377)
  %1392 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !6478
  br label %1393

1393:                                             ; preds = %1396, %1389
  %1394 = phi i64 [ %1392, %1389 ], [ %1399, %1396 ]
  %1395 = icmp slt i64 %1391, %1394
  br i1 %1395, label %1396, label %1400

1396:                                             ; preds = %1393
  %1397 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1394, i64 %1391 monotonic monotonic, align 8, !noalias !6478
  %1398 = extractvalue { i64, i1 } %1397, 1
  %1399 = extractvalue { i64, i1 } %1397, 0
  br i1 %1398, label %1400, label %1393

1400:                                             ; preds = %1396, %1393
  %1401 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !6478
  br label %__rustc::__rust_dealloc (.exit143)

__rustc::__rust_dealloc (.exit143): ; preds = %1382, %1400
  call void @free(ptr noundef nonnull %609) #92, !noalias !6478
  br label %1403

1402:                                             ; preds = %883
  unreachable

1403:                                             ; preds = %__rustc::__rust_dealloc (.exit143), %.loopexit167
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %36)
          to label %1404 unwind label %784, !noalias !6236

1404:                                             ; preds = %1403
  call void @llvm.lifetime.end.p0(ptr nonnull %36), !noalias !6214
  %1405 = icmp eq i64 %602, 0
  br i1 %1405, label %1434, label %1406

1406:                                             ; preds = %1404
  %1407 = mul nuw i64 %602, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %603) ]
  %1408 = load i64, ptr %561, align 8, !noalias !6236, !noundef !1733
  %1409 = call i64 @llvm.umin.i64(i64 %1407, i64 9223372036854775807)
  %1410 = call i64 @llvm.ssub.sat.i64(i64 %1408, i64 %1409)
  store i64 %1410, ptr %561, align 8, !noalias !6236
  %1411 = load i64, ptr %562, align 8, !noalias !6236, !noundef !1733
  %1412 = icmp slt i64 %1410, %1411
  br i1 %1412, label %1413, label %.preheader1483

1413:                                             ; preds = %1406
  store i64 %1410, ptr %562, align 8, !noalias !6236
  br label %.preheader1483

.preheader1483:                                   ; preds = %1413, %1406
  br label %1414

1414:                                             ; preds = %.preheader1483, %1417
  %1415 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !6236
  %1416 = icmp slt i64 %1415, 0
  br i1 %1416, label %1417, label %__rustc::__rust_dealloc (.exit144)

1417:                                             ; preds = %1414
  %1418 = add nsw i64 %1415, 1
  %1419 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1415, i64 %1418 acq_rel acquire, align 8, !noalias !6236
  %1420 = extractvalue { i64, i1 } %1419, 1
  br i1 %1420, label %1421, label %1414

1421:                                             ; preds = %1417
  %1422 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1409 monotonic, align 8, !noalias !6236
  %1423 = call i64 @llvm.ssub.sat.i64(i64 %1422, i64 %1409)
  %1424 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !6236
  br label %1425

1425:                                             ; preds = %1428, %1421
  %1426 = phi i64 [ %1424, %1421 ], [ %1431, %1428 ]
  %1427 = icmp slt i64 %1423, %1426
  br i1 %1427, label %1428, label %1432

1428:                                             ; preds = %1425
  %1429 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1426, i64 %1423 monotonic monotonic, align 8, !noalias !6236
  %1430 = extractvalue { i64, i1 } %1429, 1
  %1431 = extractvalue { i64, i1 } %1429, 0
  br i1 %1430, label %1432, label %1425

1432:                                             ; preds = %1428, %1425
  %1433 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !6236
  br label %__rustc::__rust_dealloc (.exit144)

__rustc::__rust_dealloc (.exit144): ; preds = %1414, %1432
  call void @free(ptr noundef nonnull %603) #92, !noalias !6236
  br label %1434

1434:                                             ; preds = %__rustc::__rust_dealloc (.exit144), %1404
  call void @llvm.experimental.noalias.scope.decl(metadata !6481)
  %1435 = load ptr, ptr %553, align 8, !alias.scope !6481, !noalias !6214, !noundef !1733
  %1436 = icmp eq ptr %1435, null
  br i1 %1436, label %1441, label %1437

1437:                                             ; preds = %1434
  %1438 = atomicrmw sub ptr %1435, i64 1 release, align 8, !noalias !6484
  %1439 = icmp eq i64 %1438, 1
  br i1 %1439, label %1440, label %1441

1440:                                             ; preds = %1437
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %553) #91, !noalias !6236
  br label %1441

1441:                                             ; preds = %1440, %1437, %1434
  call void @llvm.experimental.noalias.scope.decl(metadata !6489)
  %1442 = load ptr, ptr %554, align 8, !alias.scope !6489, !noalias !6214, !noundef !1733
  %1443 = icmp eq ptr %1442, null
  br i1 %1443, label %1448, label %1444

1444:                                             ; preds = %1441
  %1445 = atomicrmw sub ptr %1442, i64 1 release, align 8, !noalias !6492
  %1446 = icmp eq i64 %1445, 1
  br i1 %1446, label %1447, label %1448

1447:                                             ; preds = %1444
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %554) #91, !noalias !6236
  br label %1448

1448:                                             ; preds = %1447, %1444, %1441
  call void @llvm.lifetime.end.p0(ptr nonnull %37), !noalias !6214
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %38)
          to label %1449 unwind label %.loopexit.split-lp, !noalias !6236

1449:                                             ; preds = %1448
  call void @llvm.lifetime.end.p0(ptr nonnull %38), !noalias !6214
  %1450 = atomicrmw sub ptr %480, i64 1 release, align 8, !noalias !6497
  %1451 = icmp eq i64 %1450, 1
  br i1 %1451, label %1452, label %1453

1452:                                             ; preds = %1449
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %39) #91
          to label %1453 unwind label %455, !noalias !6236

1453:                                             ; preds = %1452, %1449
  call void @llvm.lifetime.end.p0(ptr nonnull %39), !noalias !6214
  call void @llvm.experimental.noalias.scope.decl(metadata !6502)
  call void @llvm.experimental.noalias.scope.decl(metadata !6505)
  %1454 = load ptr, ptr %349, align 8, !alias.scope !6508, !noalias !6214, !nonnull !1733, !noundef !1733
  %1455 = load ptr, ptr %350, align 8, !alias.scope !6508, !noalias !6214, !nonnull !1733, !noundef !1733
  %1456 = ptrtoint ptr %1455 to i64
  %1457 = ptrtoint ptr %1454 to i64
  %1458 = sub nuw i64 %1456, %1457
  %1459 = udiv exact i64 %1458, 40
  call void @llvm.experimental.noalias.scope.decl(metadata !6509)
  %1460 = icmp eq ptr %1455, %1454
  br i1 %1460, label %.loopexit157, label %.preheader156

.preheader156:                                    ; preds = %1453, %1498
  %1461 = phi i64 [ %1463, %1498 ], [ 0, %1453 ]
  %1462 = getelementptr inbounds nuw [40 x i8], ptr %1454, i64 %1461
  %1463 = add nuw nsw i64 %1461, 1
  %1464 = load i64, ptr %1462, align 8, !range !1771, !alias.scope !6512, !noalias !6515, !noundef !1733
  %1465 = icmp ugt i64 %1464, 5
  br i1 %1465, label %1466, label %1498

1466:                                             ; preds = %.preheader156
  %1467 = getelementptr i8, ptr %1462, i64 8
  %1468 = load ptr, ptr %1467, align 8, !alias.scope !6509, !noalias !6515, !nonnull !1733, !noundef !1733
  %1469 = shl i64 %1464, 3
  %1470 = add i64 %1469, -8
  %1471 = load i64, ptr %561, align 8, !noalias !6516, !noundef !1733
  %1472 = call i64 @llvm.umin.i64(i64 %1470, i64 9223372036854775807)
  %1473 = call i64 @llvm.ssub.sat.i64(i64 %1471, i64 %1472)
  store i64 %1473, ptr %561, align 8, !noalias !6516
  %1474 = load i64, ptr %562, align 8, !noalias !6516, !noundef !1733
  %1475 = icmp slt i64 %1473, %1474
  br i1 %1475, label %1476, label %.preheader1482

1476:                                             ; preds = %1466
  store i64 %1473, ptr %562, align 8, !noalias !6516
  br label %.preheader1482

.preheader1482:                                   ; preds = %1476, %1466
  br label %1477

1477:                                             ; preds = %.preheader1482, %1480
  %1478 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !6516
  %1479 = icmp slt i64 %1478, 0
  br i1 %1479, label %1480, label %__rustc::__rust_dealloc (.exit145)

1480:                                             ; preds = %1477
  %1481 = add nsw i64 %1478, 1
  %1482 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1478, i64 %1481 acq_rel acquire, align 8, !noalias !6516
  %1483 = extractvalue { i64, i1 } %1482, 1
  br i1 %1483, label %1484, label %1477

1484:                                             ; preds = %1480
  %1485 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1472 monotonic, align 8, !noalias !6516
  %1486 = call i64 @llvm.ssub.sat.i64(i64 %1485, i64 %1472)
  %1487 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !6516
  br label %1488

1488:                                             ; preds = %1491, %1484
  %1489 = phi i64 [ %1487, %1484 ], [ %1494, %1491 ]
  %1490 = icmp slt i64 %1486, %1489
  br i1 %1490, label %1491, label %1495

1491:                                             ; preds = %1488
  %1492 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1489, i64 %1486 monotonic monotonic, align 8, !noalias !6516
  %1493 = extractvalue { i64, i1 } %1492, 1
  %1494 = extractvalue { i64, i1 } %1492, 0
  br i1 %1493, label %1495, label %1488

1495:                                             ; preds = %1491, %1488
  %1496 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !6516
  br label %__rustc::__rust_dealloc (.exit145)

__rustc::__rust_dealloc (.exit145): ; preds = %1477, %1495
  %1497 = icmp ne i64 %1470, 0
  call void @llvm.assume(i1 %1497), !noalias !6516
  call void @free(ptr noundef nonnull %1468) #92, !noalias !6516
  br label %1498

1498:                                             ; preds = %__rustc::__rust_dealloc (.exit145), %.preheader156
  %1499 = icmp eq i64 %1463, %1459
  br i1 %1499, label %.loopexit157, label %.preheader156

.loopexit157:                                     ; preds = %1498, %1453
  %1500 = load i64, ptr %348, align 8, !alias.scope !6508, !noalias !6214, !noundef !1733
  %1501 = icmp eq i64 %1500, 0
  br i1 %1501, label %1600, label %1569

1502:                                             ; preds = %822, %819, %816
  call void @llvm.experimental.noalias.scope.decl(metadata !6519)
  %1503 = load ptr, ptr %554, align 8, !alias.scope !6519, !noalias !6214, !noundef !1733
  %1504 = icmp eq ptr %1503, null
  br i1 %1504, label %593, label %1505

1505:                                             ; preds = %1502
  %1506 = atomicrmw sub ptr %1503, i64 1 release, align 8, !noalias !6522
  %1507 = icmp eq i64 %1506, 1
  br i1 %1507, label %1508, label %593

1508:                                             ; preds = %1505
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %554) #91, !noalias !6236
  br label %593

1509:                                             ; preds = %578, %572
; call core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(32) %42) #89, !noalias !6236
  br i1 %573, label %1510, label %1511

1510:                                             ; preds = %1509
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %46) #89, !noalias !6236
  br i1 %574, label %1567, label %1653

1511:                                             ; preds = %1509
  br i1 %574, label %1567, label %1653

1512:                                             ; preds = %457, %__rustc::__rust_dealloc (.exit136), %.loopexit186
  %1513 = phi i64 [ -1, %457 ], [ %259, %.loopexit186 ], [ %259, %__rustc::__rust_dealloc (.exit136) ]
  %1514 = phi i8 [ 2, %457 ], [ %264, %.loopexit186 ], [ %264, %__rustc::__rust_dealloc (.exit136) ]
  %1515 = icmp eq i64 %201, 0
  br i1 %1515, label %._crit_edge, label %.lr.ph1376

1516:                                             ; preds = %.lr.ph1376
  %1517 = icmp eq i64 %1520, %201
  br i1 %1517, label %._crit_edge, label %.lr.ph1376

.lr.ph1376:                                       ; preds = %1512, %1516
  %1518 = phi i64 [ %1520, %1516 ], [ 0, %1512 ]
  %1519 = getelementptr inbounds nuw [160 x i8], ptr %202, i64 %1518
  %1520 = add nuw nsw i64 %1518, 1
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef readonly align 8 dereferenceable(160) %1519)
          to label %1516 unwind label %1524, !noalias !6527

1521:                                             ; preds = %.lr.ph1378
  %1522 = add i64 %1527, 1
  %1523 = icmp eq i64 %1522, %201
  br i1 %1523, label %._crit_edge1379, label %.lr.ph1378

1524:                                             ; preds = %.lr.ph1376
  %1525 = landingpad { ptr, i32 }
          cleanup
  %1526 = icmp eq i64 %1520, %201
  br i1 %1526, label %._crit_edge1379, label %.lr.ph1378

.lr.ph1378:                                       ; preds = %1524, %1521
  %1527 = phi i64 [ %1522, %1521 ], [ %1520, %1524 ]
  %1528 = getelementptr inbounds nuw [160 x i8], ptr %202, i64 %1527
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef readonly align 8 dereferenceable(160) %1528) #89
          to label %1521 unwind label %1529, !noalias !6527

1529:                                             ; preds = %.lr.ph1378
  %1530 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #90, !noalias !6530
  unreachable

._crit_edge1379:                                  ; preds = %1521, %1524
  %1531 = icmp eq i64 %222, 0
  br i1 %1531, label %1653, label %1532

1532:                                             ; preds = %._crit_edge1379
  %1533 = mul nuw i64 %222, 160
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %202, i64 noundef %1533, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !6527
  br label %1653

._crit_edge:                                      ; preds = %1516, %1512
  %1534 = icmp eq i64 %222, 0
  br i1 %1534, label %1602, label %1535

1535:                                             ; preds = %._crit_edge
  %1536 = mul nuw i64 %222, 160
  %1537 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1538 = load i64, ptr %1537, align 8, !noalias !6527, !noundef !1733
  %1539 = tail call i64 @llvm.umin.i64(i64 %1536, i64 9223372036854775807)
  %1540 = tail call i64 @llvm.ssub.sat.i64(i64 %1538, i64 %1539)
  store i64 %1540, ptr %1537, align 8, !noalias !6527
  %1541 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1542 = load i64, ptr %1541, align 8, !noalias !6527, !noundef !1733
  %1543 = icmp slt i64 %1540, %1542
  br i1 %1543, label %1544, label %.preheader1469

1544:                                             ; preds = %1535
  store i64 %1540, ptr %1541, align 8, !noalias !6527
  br label %.preheader1469

.preheader1469:                                   ; preds = %1544, %1535
  br label %1545

1545:                                             ; preds = %.preheader1469, %1548
  %1546 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !6527
  %1547 = icmp slt i64 %1546, 0
  br i1 %1547, label %1548, label %__rustc::__rust_dealloc (.exit146)

1548:                                             ; preds = %1545
  %1549 = add nsw i64 %1546, 1
  %1550 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1546, i64 %1549 acq_rel acquire, align 8, !noalias !6527
  %1551 = extractvalue { i64, i1 } %1550, 1
  br i1 %1551, label %1552, label %1545

1552:                                             ; preds = %1548
  %1553 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1539 monotonic, align 8, !noalias !6527
  %1554 = tail call i64 @llvm.ssub.sat.i64(i64 %1553, i64 %1539)
  %1555 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !6527
  br label %1556

1556:                                             ; preds = %1559, %1552
  %1557 = phi i64 [ %1555, %1552 ], [ %1562, %1559 ]
  %1558 = icmp slt i64 %1554, %1557
  br i1 %1558, label %1559, label %1563

1559:                                             ; preds = %1556
  %1560 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1557, i64 %1554 monotonic monotonic, align 8, !noalias !6527
  %1561 = extractvalue { i64, i1 } %1560, 1
  %1562 = extractvalue { i64, i1 } %1560, 0
  br i1 %1561, label %1563, label %1556

1563:                                             ; preds = %1559, %1556
  %1564 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !6527
  br label %__rustc::__rust_dealloc (.exit146)

__rustc::__rust_dealloc (.exit146): ; preds = %1545, %1563
  tail call void @free(ptr noundef nonnull %202) #92, !noalias !6527
  br label %1602

1565:                                             ; preds = %.loopexit188
  %1566 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  tail call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %3) #89, !noalias !6533
  br label %1567

1567:                                             ; preds = %1565, %1511, %1510, %365
  %1568 = phi { ptr, i32 } [ %575, %1511 ], [ %1566, %1565 ], [ %575, %1510 ], [ %465, %365 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %51) #89
          to label %1653 unwind label %477, !noalias !6534

1569:                                             ; preds = %.loopexit153._crit_edge, %.loopexit157
  %.pre-phi684 = phi ptr [ %.pre683, %.loopexit153._crit_edge ], [ %562, %.loopexit157 ]
  %.pre-phi = phi ptr [ %.pre, %.loopexit153._crit_edge ], [ %561, %.loopexit157 ]
  %1570 = phi i64 [ %694, %.loopexit153._crit_edge ], [ %1500, %.loopexit157 ]
  %1571 = phi i8 [ 2, %.loopexit153._crit_edge ], [ %1372, %.loopexit157 ]
  %1572 = load ptr, ptr %42, align 8, !noalias !6316, !nonnull !1733, !noundef !1733
  %1573 = mul nuw i64 %1570, 40
  %1574 = load i64, ptr %.pre-phi, align 8, !noalias !6236, !noundef !1733
  %1575 = call i64 @llvm.umin.i64(i64 %1573, i64 9223372036854775807)
  %1576 = call i64 @llvm.ssub.sat.i64(i64 %1574, i64 %1575)
  store i64 %1576, ptr %.pre-phi, align 8, !noalias !6236
  %1577 = load i64, ptr %.pre-phi684, align 8, !noalias !6236, !noundef !1733
  %1578 = icmp slt i64 %1576, %1577
  br i1 %1578, label %1579, label %.preheader1479

1579:                                             ; preds = %1569
  store i64 %1576, ptr %.pre-phi684, align 8, !noalias !6236
  br label %.preheader1479

.preheader1479:                                   ; preds = %1579, %1569
  br label %1580

1580:                                             ; preds = %.preheader1479, %1583
  %1581 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !6236
  %1582 = icmp slt i64 %1581, 0
  br i1 %1582, label %1583, label %__rustc::__rust_dealloc (.exit147)

1583:                                             ; preds = %1580
  %1584 = add nsw i64 %1581, 1
  %1585 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1581, i64 %1584 acq_rel acquire, align 8, !noalias !6236
  %1586 = extractvalue { i64, i1 } %1585, 1
  br i1 %1586, label %1587, label %1580

1587:                                             ; preds = %1583
  %1588 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1575 monotonic, align 8, !noalias !6236
  %1589 = call i64 @llvm.ssub.sat.i64(i64 %1588, i64 %1575)
  %1590 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !6236
  br label %1591

1591:                                             ; preds = %1594, %1587
  %1592 = phi i64 [ %1590, %1587 ], [ %1597, %1594 ]
  %1593 = icmp slt i64 %1589, %1592
  br i1 %1593, label %1594, label %1598

1594:                                             ; preds = %1591
  %1595 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1592, i64 %1589 monotonic monotonic, align 8, !noalias !6236
  %1596 = extractvalue { i64, i1 } %1595, 1
  %1597 = extractvalue { i64, i1 } %1595, 0
  br i1 %1596, label %1598, label %1591

1598:                                             ; preds = %1594, %1591
  %1599 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !6236
  br label %__rustc::__rust_dealloc (.exit147)

__rustc::__rust_dealloc (.exit147): ; preds = %1580, %1598
  call void @free(ptr noundef nonnull %1572) #92, !noalias !6236
  br label %1600

1600:                                             ; preds = %__rustc::__rust_dealloc (.exit147), %.loopexit157, %.loopexit153
  %1601 = phi i8 [ %1372, %.loopexit157 ], [ 2, %.loopexit153 ], [ %1571, %__rustc::__rust_dealloc (.exit147) ]
  call void @llvm.lifetime.end.p0(ptr nonnull %42), !noalias !6214
  call void @llvm.lifetime.end.p0(ptr nonnull %46), !noalias !6214
  call void @llvm.lifetime.end.p0(ptr nonnull %29)
  call void @llvm.lifetime.end.p0(ptr nonnull %40)
  call void @llvm.lifetime.end.p0(ptr nonnull %51), !noalias !6168
  br label %1608

1602:                                             ; preds = %__rustc::__rust_dealloc (.exit146), %._crit_edge
  call void @llvm.lifetime.end.p0(ptr nonnull %29)
  call void @llvm.lifetime.end.p0(ptr nonnull %40)
  call void @llvm.lifetime.end.p0(ptr nonnull %51), !noalias !6168
  %1603 = icmp eq i64 %1513, -1
  br i1 %1603, label %1608, label %1604

1604:                                             ; preds = %1602
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %54, ptr noundef nonnull align 8 dereferenceable(24) %52, i64 24, i1 false), !noalias !6168
  %1605 = getelementptr inbounds nuw i8, ptr %0, i64 33
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(63) %1605, ptr noundef nonnull align 1 dereferenceable(63) %53, i64 63, i1 false), !noalias !6535
  call void @llvm.lifetime.end.p0(ptr nonnull %52)
  call void @llvm.lifetime.end.p0(ptr nonnull %53)
  %1606 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %1606, ptr noundef nonnull align 8 dereferenceable(24) %54, i64 24, i1 false), !noalias !6535
  store i64 %1513, ptr %0, align 16, !alias.scope !6157, !noalias !6535
  %1607 = getelementptr inbounds nuw i8, ptr %0, i64 32
  store i8 %1514, ptr %1607, align 16, !alias.scope !6157, !noalias !6535
  call void @llvm.lifetime.end.p0(ptr nonnull %54)
  call void @llvm.lifetime.end.p0(ptr nonnull %56), !noalias !6168
  br label %1657

1608:                                             ; preds = %1602, %1600
  %1609 = phi i8 [ %1601, %1600 ], [ %1514, %1602 ]
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %54, ptr noundef nonnull align 8 dereferenceable(24) %52, i64 24, i1 false), !noalias !6168
  call void @llvm.lifetime.end.p0(ptr nonnull %52)
  call void @llvm.lifetime.end.p0(ptr nonnull %53)
  call void @llvm.lifetime.start.p0(ptr nonnull %50), !noalias !6168
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %50, ptr noundef nonnull align 8 dereferenceable(24) %54, i64 24, i1 false), !noalias !6168
  call void @llvm.lifetime.end.p0(ptr nonnull %54)
  call void @llvm.lifetime.start.p0(ptr nonnull %11)
  %1610 = load ptr, ptr %60, align 16, !alias.scope !6162, !noalias !6177, !noundef !1733
  %1611 = icmp eq ptr %1610, null
  br i1 %1611, label %1622, label %1612

1612:                                             ; preds = %1608
  %1613 = getelementptr inbounds nuw i8, ptr %1610, i64 296
  %1614 = load atomic i32, ptr %1613 acquire, align 4, !noalias !6536
  %1615 = icmp eq i32 %1614, 0
  br i1 %1615, label %1616, label %1620

1616:                                             ; preds = %1612
  %1617 = getelementptr inbounds nuw i8, ptr %1610, i64 272
  %1618 = load i8, ptr %1617, align 8, !noalias !6177
  %1619 = getelementptr inbounds nuw i8, ptr %1610, i64 273
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %11, ptr noundef nonnull align 1 dereferenceable(23) %1619, i64 23, i1 false), !noalias !6177
  br label %1620

1620:                                             ; preds = %1616, %1612
  %1621 = phi i8 [ %1618, %1616 ], [ -1, %1612 ]
  switch i8 %1609, label %1626 [
    i8 2, label %1644
    i8 0, label %1625
  ]

1622:                                             ; preds = %1608
  %1623 = icmp eq i8 %1609, 2
  %1624 = and i1 %1623, %247
  br label %1644

1625:                                             ; preds = %1641, %1628, %1620
  br label %1644

1626:                                             ; preds = %1620
  %1627 = icmp eq i8 %1621, -1
  br i1 %1627, label %1644, label %1628

1628:                                             ; preds = %1626
  %1629 = getelementptr inbounds nuw i8, ptr %2, i64 472
  %1630 = load i8, ptr %1629, align 8, !range !3719, !alias.scope !6162, !noalias !6177, !noundef !1733
  %1631 = icmp eq i8 %1630, 2
  br i1 %1631, label %1632, label %1625

1632:                                             ; preds = %1628
  %1633 = getelementptr inbounds nuw i8, ptr %2, i64 688
  call void @llvm.experimental.noalias.scope.decl(metadata !6539)
  %1634 = load ptr, ptr %1633, align 16, !alias.scope !6542, !noalias !6543, !nonnull !1733, !noundef !1733
  call void @llvm.lifetime.start.p0(ptr nonnull %10), !noalias !6545
  store i8 %1621, ptr %10, align 8, !noalias !6549
  %1635 = getelementptr inbounds nuw i8, ptr %10, i64 1
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %1635, ptr noundef nonnull align 1 dereferenceable(23) %11, i64 23, i1 false), !noalias !6168
  %1636 = getelementptr inbounds nuw i8, ptr %1634, i64 40
  %1637 = load atomic i32, ptr %1636 acquire, align 4, !noalias !6550
  %1638 = icmp eq i32 %1637, 0
  br i1 %1638, label %1641, label %1639, !prof !1946

1639:                                             ; preds = %1632
  %1640 = getelementptr inbounds nuw i8, ptr %1634, i64 16
; invoke <std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::initialize::<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::get_or_init<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::try_insert::{closure#0}>::{closure#0}, !>
  invoke fastcc void @<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::initialize::<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::get_or_init<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::try_insert::{closure#0}>::{closure#0}, !> (.llvm.1794586459888082020)(ptr noundef nonnull align 8 %1640, ptr noundef nonnull align 8 %10)
          to label %1641 unwind label %1642, !noalias !6177

1641:                                             ; preds = %1639, %1632
  call void @llvm.lifetime.end.p0(ptr nonnull %10), !noalias !6545
  br label %1625

1642:                                             ; preds = %1639
  %1643 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %50) #89, !noalias !6177
  br label %1653

1644:                                             ; preds = %1626, %1625, %1622, %1620
  %1645 = phi i8 [ -1, %1622 ], [ -1, %1626 ], [ %1621, %1625 ], [ %1621, %1620 ]
  %1646 = phi i1 [ %1624, %1622 ], [ false, %1626 ], [ false, %1625 ], [ %247, %1620 ]
  %1647 = icmp eq i8 %1645, -1
  %1648 = select i1 %1646, i1 %1647, i1 false
  %1649 = zext i1 %1648 to i64
  call void @llvm.lifetime.end.p0(ptr nonnull %11)
  %1650 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %1650, ptr noundef nonnull align 8 dereferenceable(24) %50, i64 24, i1 false), !noalias !6535
  call void @llvm.lifetime.end.p0(ptr nonnull %50), !noalias !6168
  %1651 = getelementptr inbounds nuw i8, ptr %0, i64 32
  store i64 %1649, ptr %1651, align 16, !alias.scope !6157, !noalias !6535
  %1652 = getelementptr inbounds nuw i8, ptr %0, i64 40
  store i64 %246, ptr %1652, align 8, !alias.scope !6157, !noalias !6535
  store i64 -1, ptr %0, align 16, !alias.scope !6157, !noalias !6535
  call void @llvm.lifetime.end.p0(ptr nonnull %56), !noalias !6168
  br label %1657

1653:                                             ; preds = %1655, %1642, %1567, %1532, %._crit_edge1379, %1511, %1510, %455
  %1654 = phi { ptr, i32 } [ %1643, %1642 ], [ %1656, %1655 ], [ %575, %1510 ], [ %456, %455 ], [ %575, %1511 ], [ %1568, %1567 ], [ %1525, %1532 ], [ %1525, %._crit_edge1379 ]
  resume { ptr, i32 } %1654

1655:                                             ; preds = %173, %84
  %1656 = phi { ptr, i32 } [ %174, %173 ], [ %85, %84 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  tail call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %3) #89, !noalias !6551
  br label %1653

1657:                                             ; preds = %1644, %1604, %64
  ret void
}
define internal fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>(ptr dead_on_unwind noalias nofree noundef nonnull writable writeonly align 16 captures(none) dereferenceable(96) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(200) %1, ptr noalias nofree noundef nonnull align 16 dereferenceable(1232) %2, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(24) %3, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(208) %4) unnamed_addr #0 personality ptr @rust_eh_personality !guid !6565 {
  %6 = alloca [24 x i8], align 8
  %7 = alloca [24 x i8], align 8
  %8 = alloca [23 x i8], align 1
  %9 = alloca [24 x i8], align 8
  %10 = alloca [80 x i8], align 8
  %11 = alloca [24 x i8], align 8
  %12 = alloca [24 x i8], align 8
  %13 = alloca [80 x i8], align 8
  %14 = alloca [80 x i8], align 8
  %15 = alloca [24 x i8], align 8
  %16 = alloca [80 x i8], align 8
  %17 = alloca [80 x i8], align 8
  %18 = alloca [24 x i8], align 8
  %19 = alloca [24 x i8], align 8
  %20 = alloca [80 x i8], align 8
  %21 = alloca [24 x i8], align 8
  %22 = alloca [24 x i8], align 8
  %23 = alloca [32 x i8], align 8
  %24 = alloca [8 x i8], align 8
  %25 = alloca [8 x i8], align 8
  %26 = alloca [16 x i8], align 8
  %27 = alloca [40 x i8], align 8
  %28 = alloca [15 x i8], align 1
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
  %41 = alloca [15 x i8], align 1
  %42 = alloca [48 x i8], align 8
  %43 = alloca [96 x i8], align 16
  %44 = alloca [32 x i8], align 8
  %45 = alloca [32 x i8], align 8
  %46 = alloca [24 x i8], align 8
  %47 = alloca [96 x i8], align 16
  %48 = alloca [24 x i8], align 8
  %49 = alloca [160 x i8], align 8
  %50 = alloca [32 x i8], align 8
  %51 = alloca [24 x i8], align 8
  %52 = alloca [48 x i8], align 8
  %53 = alloca [16 x i8], align 8
  %54 = alloca [96 x i8], align 16
  %55 = alloca [32 x i8], align 8
  %56 = alloca [96 x i8], align 16
  %57 = alloca [24 x i8], align 8
  %58 = alloca [24 x i8], align 8
  %59 = alloca [32 x i8], align 8
  %60 = alloca [63 x i8], align 1
  %61 = alloca [224 x i8], align 8
  %62 = alloca [24 x i8], align 8
  %63 = alloca [24 x i8], align 8
  %64 = alloca [64 x i8], align 8
  %65 = getelementptr inbounds nuw i8, ptr %1, i64 194
  %66 = load i8, ptr %65, align 2, !range !3719, !noundef !1733
  %67 = icmp eq i8 %66, 2
  br i1 %67, label %68, label %72

68:                                               ; preds = %5
  %69 = getelementptr inbounds nuw i8, ptr %2, i64 608
  %70 = load ptr, ptr %69, align 16, !noundef !1733
  %71 = icmp eq ptr %70, null
  br i1 %71, label %72, label %251

72:                                               ; preds = %68, %5
  call void @llvm.lifetime.start.p0(ptr nonnull %64)
  call void @llvm.lifetime.start.p0(ptr nonnull %63)
  store i64 0, ptr %63, align 8
  %73 = getelementptr inbounds nuw i8, ptr %63, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %73, align 8
  %74 = getelementptr inbounds nuw i8, ptr %63, i64 16
  store i64 0, ptr %74, align 8
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6566)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6569)
  call void @llvm.lifetime.start.p0(ptr nonnull %53)
  call void @llvm.lifetime.start.p0(ptr nonnull %57), !noalias !6571
  call void @llvm.lifetime.start.p0(ptr nonnull %56), !noalias !6571
  %75 = getelementptr inbounds nuw i8, ptr %3, i64 16
  %76 = load i64, ptr %75, align 8, !alias.scope !6569, !noalias !6574, !noundef !1733
  %77 = icmp ult i64 %76, 230584300921369396
  tail call void @llvm.assume(i1 %77)
; invoke purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
  invoke fastcc void @purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %56, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(24) %63, i64 noundef %76)
          to label %78 unwind label %249, !noalias !6575

78:                                               ; preds = %72
  %79 = load i64, ptr %56, align 16, !range !2520, !noalias !6571, !noundef !1733
  %80 = icmp eq i64 %79, -1
  %81 = getelementptr inbounds nuw i8, ptr %56, i64 8
  %82 = load i64, ptr %81, align 8, !noalias !6571
  %83 = getelementptr inbounds nuw i8, ptr %56, i64 16
  %84 = load ptr, ptr %83, align 16, !noalias !6571
  %85 = getelementptr inbounds nuw i8, ptr %56, i64 24
  %86 = load i64, ptr %85, align 8, !noalias !6571
  br i1 %80, label %91, label %87

87:                                               ; preds = %78
  %88 = getelementptr inbounds nuw i8, ptr %56, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(64) %64, ptr noundef nonnull align 16 dereferenceable(64) %88, i64 64, i1 false), !noalias !6576
  call void @llvm.lifetime.end.p0(ptr nonnull %56), !noalias !6571
  call void @llvm.lifetime.end.p0(ptr nonnull %57), !noalias !6571
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %3)
          to label %1712 unwind label %89

89:                                               ; preds = %87
  %90 = landingpad { ptr, i32 }
          cleanup
  br label %1724

91:                                               ; preds = %78
  call void @llvm.lifetime.end.p0(ptr nonnull %56), !noalias !6571
  store i64 %82, ptr %57, align 8, !noalias !6571
  %92 = getelementptr inbounds nuw i8, ptr %57, i64 8
  store ptr %84, ptr %92, align 8, !noalias !6571
  %93 = getelementptr inbounds nuw i8, ptr %57, i64 16
  store i64 %86, ptr %93, align 8, !noalias !6571
  %94 = getelementptr inbounds nuw i8, ptr %3, i64 8
  %95 = load ptr, ptr %94, align 8, !alias.scope !6569, !noalias !6574, !nonnull !1733, !noundef !1733
  %96 = load i64, ptr %3, align 8, !range !1828, !alias.scope !6569, !noalias !6574, !noundef !1733
  %97 = mul nuw nsw i64 %76, 40
  %98 = getelementptr inbounds nuw i8, ptr %95, i64 %97
  call void @llvm.lifetime.start.p0(ptr nonnull %55), !noalias !6571
  store ptr %95, ptr %55, align 8, !noalias !6571
  %99 = getelementptr inbounds nuw i8, ptr %55, i64 8
  %100 = getelementptr inbounds nuw i8, ptr %55, i64 16
  store i64 %96, ptr %100, align 8, !noalias !6571
  %101 = getelementptr inbounds nuw i8, ptr %55, i64 24
  store ptr %98, ptr %101, align 8, !noalias !6571
  %102 = icmp eq i64 %76, 0
  br i1 %102, label %.loopexit144, label %103

103:                                              ; preds = %91
  %104 = getelementptr inbounds nuw i8, ptr %52, i64 8
  %105 = getelementptr inbounds nuw i8, ptr %54, i64 8
  %106 = getelementptr inbounds nuw i8, ptr %2, i64 656
  %107 = getelementptr inbounds nuw i8, ptr %2, i64 880
  %108 = getelementptr inbounds nuw i8, ptr %52, i64 16
  %109 = getelementptr inbounds nuw i8, ptr %54, i64 16
  %110 = getelementptr inbounds nuw i8, ptr %54, i64 24
  %111 = getelementptr inbounds nuw i8, ptr %54, i64 32
  br label %116

112:                                              ; preds = %123
  %113 = landingpad { ptr, i32 }
          cleanup
  store ptr %120, ptr %99, align 8, !noalias !6571
  br label %114

114:                                              ; preds = %160, %157, %112
  %115 = phi { ptr, i32 } [ %113, %112 ], [ %158, %160 ], [ %158, %157 ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %55) #89
          to label %128 unwind label %247, !noalias !6577

116:                                              ; preds = %163, %103
  %117 = phi ptr [ %84, %103 ], [ %164, %163 ]
  %118 = phi i64 [ %86, %103 ], [ %169, %163 ]
  %119 = phi ptr [ %95, %103 ], [ %120, %163 ]
  %120 = getelementptr inbounds nuw i8, ptr %119, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %54), !noalias !6571
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6578)
  call void @llvm.lifetime.start.p0(ptr nonnull %52), !noalias !6571
  store ptr %2, ptr %52, align 8, !noalias !6581
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %104, ptr noundef nonnull align 8 dereferenceable(40) %119, i64 40, i1 false), !noalias !6577
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6584)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6587)
  %121 = load i64, ptr %104, align 8, !alias.scope !6587, !noalias !6589, !noundef !1733
  %122 = icmp eq i64 %121, 0
  br i1 %122, label %123, label %125

123:                                              ; preds = %116
  %124 = load ptr, ptr %106, align 16, !alias.scope !6591, !noalias !6592, !nonnull !1733, !align !1829, !noundef !1733
; invoke purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(96) %54, ptr noalias nofree noundef align 8 dereferenceable(184) %107, ptr noundef nonnull align 8 %124, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %108)
          to label %139 unwind label %112, !noalias !6577

125:                                              ; preds = %116
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %105, ptr noundef nonnull align 8 dereferenceable(40) %119, i64 40, i1 false), !noalias !6577
  call void @llvm.lifetime.end.p0(ptr nonnull %52), !noalias !6571
  br label %148

.loopexit144:                                     ; preds = %163, %91
  %126 = phi i64 [ %86, %91 ], [ %169, %163 ]
  %127 = phi ptr [ %95, %91 ], [ %98, %163 ]
  store ptr %127, ptr %99, align 8, !noalias !6571
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %55)
          to label %132 unwind label %130, !noalias !6577

128:                                              ; preds = %130, %114
  %129 = phi { ptr, i32 } [ %131, %130 ], [ %115, %114 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %57) #89, !noalias !6577
  br label %1724

130:                                              ; preds = %142, %.loopexit144
  %131 = landingpad { ptr, i32 }
          cleanup
  br label %128

132:                                              ; preds = %.loopexit144
  call void @llvm.lifetime.end.p0(ptr nonnull %55), !noalias !6571
  %133 = load i64, ptr %57, align 8, !noalias !6576
  %134 = load ptr, ptr %92, align 8, !noalias !6576
  call void @llvm.lifetime.end.p0(ptr nonnull %57), !noalias !6571
  call void @llvm.lifetime.end.p0(ptr nonnull %53)
  call void @llvm.lifetime.end.p0(ptr nonnull %63)
  call void @llvm.lifetime.end.p0(ptr nonnull %64)
  %135 = getelementptr inbounds nuw i8, ptr %0, i64 8
  store i64 %133, ptr %135, align 8
  %136 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store ptr %134, ptr %136, align 16
  %137 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store i64 %126, ptr %137, align 8
  %138 = getelementptr inbounds nuw i8, ptr %0, i64 32
  store i64 0, ptr %138, align 16
  store i64 -1, ptr %0, align 16
  br label %1721

139:                                              ; preds = %123
  %140 = load i64, ptr %54, align 16, !noalias !6571
  call void @llvm.lifetime.end.p0(ptr nonnull %52), !noalias !6571
  %141 = icmp eq i64 %140, -1
  br i1 %141, label %148, label %142

142:                                              ; preds = %139
  store ptr %120, ptr %99, align 8, !noalias !6571
  %143 = load i64, ptr %105, align 8, !noalias !6571
  %144 = load ptr, ptr %109, align 16, !noalias !6571
  %145 = load i64, ptr %110, align 8, !noalias !6571
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %53, ptr noundef nonnull align 16 dereferenceable(16) %111, i64 16, i1 false), !noalias !6571
  %146 = getelementptr inbounds nuw i8, ptr %54, i64 48
  %147 = getelementptr inbounds nuw i8, ptr %64, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %147, ptr noundef nonnull align 16 dereferenceable(48) %146, i64 48, i1 false), !noalias !6576
  call void @llvm.lifetime.end.p0(ptr nonnull %54), !noalias !6571
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %64, ptr noundef nonnull align 8 dereferenceable(16) %53, i64 16, i1 false), !noalias !6576
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %55)
          to label %171 unwind label %130, !noalias !6577

148:                                              ; preds = %139, %125
  %149 = load i64, ptr %105, align 8, !noalias !6571
  %150 = load ptr, ptr %109, align 16, !noalias !6571
  %151 = load i64, ptr %110, align 8, !noalias !6571
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %53, ptr noundef nonnull align 16 dereferenceable(16) %111, i64 16, i1 false), !noalias !6571
  call void @llvm.lifetime.end.p0(ptr nonnull %54), !noalias !6571
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6593)
  %152 = load i64, ptr %57, align 8, !range !1828, !alias.scope !6593, !noalias !6596, !noundef !1733
  %153 = icmp eq i64 %118, %152
  br i1 %153, label %154, label %163

154:                                              ; preds = %148
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %57)
          to label %155 unwind label %157, !noalias !6598

155:                                              ; preds = %154
  %156 = load ptr, ptr %92, align 8, !alias.scope !6593, !noalias !6596
  br label %163

157:                                              ; preds = %154
  %158 = landingpad { ptr, i32 }
          cleanup
  store ptr %120, ptr %99, align 8, !noalias !6571
  %159 = icmp ugt i64 %149, 5
  br i1 %159, label %160, label %114

160:                                              ; preds = %157
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %150) ]
  %161 = shl i64 %149, 3
  %162 = add i64 %161, -8
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %150, i64 noundef %162, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !6599
  br label %114

163:                                              ; preds = %155, %148
  %164 = phi ptr [ %156, %155 ], [ %117, %148 ]
  %165 = getelementptr inbounds nuw [40 x i8], ptr %164, i64 %118
  store i64 %149, ptr %165, align 8, !noalias !6602
  %166 = getelementptr inbounds nuw i8, ptr %165, i64 8
  store ptr %150, ptr %166, align 8, !noalias !6602
  %167 = getelementptr inbounds nuw i8, ptr %165, i64 16
  store i64 %151, ptr %167, align 8, !noalias !6577
  %168 = getelementptr inbounds nuw i8, ptr %165, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %168, ptr noundef nonnull align 8 dereferenceable(16) %53, i64 16, i1 false), !noalias !6577
  %169 = add i64 %118, 1
  store i64 %169, ptr %93, align 8, !alias.scope !6593, !noalias !6596
  %170 = icmp eq ptr %120, %98
  br i1 %170, label %.loopexit144, label %116

171:                                              ; preds = %142
  call void @llvm.lifetime.end.p0(ptr nonnull %55), !noalias !6571
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6603)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6606)
  %172 = icmp eq i64 %118, 0
  br i1 %172, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %171
  %173 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %174 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %175

175:                                              ; preds = %.preheader, %213
  %176 = phi i64 [ %178, %213 ], [ 0, %.preheader ]
  %177 = getelementptr inbounds nuw [40 x i8], ptr %117, i64 %176
  %178 = add nuw nsw i64 %176, 1
  %179 = load i64, ptr %177, align 8, !range !1771, !alias.scope !6609, !noalias !6612, !noundef !1733
  %180 = icmp ugt i64 %179, 5
  br i1 %180, label %181, label %213

181:                                              ; preds = %175
  %182 = getelementptr i8, ptr %177, i64 8
  %183 = load ptr, ptr %182, align 8, !alias.scope !6606, !noalias !6612, !nonnull !1733, !noundef !1733
  %184 = shl i64 %179, 3
  %185 = add i64 %184, -8
  %186 = load i64, ptr %173, align 8, !noalias !6613, !noundef !1733
  %187 = tail call i64 @llvm.umin.i64(i64 %185, i64 9223372036854775807)
  %188 = tail call i64 @llvm.ssub.sat.i64(i64 %186, i64 %187)
  store i64 %188, ptr %173, align 8, !noalias !6613
  %189 = load i64, ptr %174, align 8, !noalias !6613, !noundef !1733
  %190 = icmp slt i64 %188, %189
  br i1 %190, label %191, label %.preheader1585

191:                                              ; preds = %181
  store i64 %188, ptr %174, align 8, !noalias !6613
  br label %.preheader1585

.preheader1585:                                   ; preds = %191, %181
  br label %192

192:                                              ; preds = %.preheader1585, %195
  %193 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !6613
  %194 = icmp slt i64 %193, 0
  br i1 %194, label %195, label %__rustc::__rust_dealloc (.exit)

195:                                              ; preds = %192
  %196 = add nsw i64 %193, 1
  %197 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %193, i64 %196 acq_rel acquire, align 8, !noalias !6613
  %198 = extractvalue { i64, i1 } %197, 1
  br i1 %198, label %199, label %192

199:                                              ; preds = %195
  %200 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %187 monotonic, align 8, !noalias !6613
  %201 = tail call i64 @llvm.ssub.sat.i64(i64 %200, i64 %187)
  %202 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !6613
  br label %203

203:                                              ; preds = %206, %199
  %204 = phi i64 [ %202, %199 ], [ %209, %206 ]
  %205 = icmp slt i64 %201, %204
  br i1 %205, label %206, label %210

206:                                              ; preds = %203
  %207 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %204, i64 %201 monotonic monotonic, align 8, !noalias !6613
  %208 = extractvalue { i64, i1 } %207, 1
  %209 = extractvalue { i64, i1 } %207, 0
  br i1 %208, label %210, label %203

210:                                              ; preds = %206, %203
  %211 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !6613
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %192, %210
  %212 = icmp ne i64 %185, 0
  tail call void @llvm.assume(i1 %212), !noalias !6613
  tail call void @free(ptr noundef nonnull %183) #92, !noalias !6613
  br label %213

213:                                              ; preds = %__rustc::__rust_dealloc (.exit), %175
  %214 = icmp eq i64 %178, %118
  br i1 %214, label %.loopexit, label %175

.loopexit:                                        ; preds = %213, %171
  %215 = load i64, ptr %57, align 8, !alias.scope !6603, !noalias !6571
  %216 = icmp eq i64 %215, 0
  br i1 %216, label %1711, label %217

217:                                              ; preds = %.loopexit
  %218 = mul nuw i64 %215, 40
  %219 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %220 = load i64, ptr %219, align 8, !noalias !6612, !noundef !1733
  %221 = tail call i64 @llvm.umin.i64(i64 %218, i64 9223372036854775807)
  %222 = tail call i64 @llvm.ssub.sat.i64(i64 %220, i64 %221)
  store i64 %222, ptr %219, align 8, !noalias !6612
  %223 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %224 = load i64, ptr %223, align 8, !noalias !6612, !noundef !1733
  %225 = icmp slt i64 %222, %224
  br i1 %225, label %226, label %.preheader1584

226:                                              ; preds = %217
  store i64 %222, ptr %223, align 8, !noalias !6612
  br label %.preheader1584

.preheader1584:                                   ; preds = %226, %217
  br label %227

227:                                              ; preds = %.preheader1584, %230
  %228 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !6612
  %229 = icmp slt i64 %228, 0
  br i1 %229, label %230, label %__rustc::__rust_dealloc (.exit137)

230:                                              ; preds = %227
  %231 = add nsw i64 %228, 1
  %232 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %228, i64 %231 acq_rel acquire, align 8, !noalias !6612
  %233 = extractvalue { i64, i1 } %232, 1
  br i1 %233, label %234, label %227

234:                                              ; preds = %230
  %235 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %221 monotonic, align 8, !noalias !6612
  %236 = tail call i64 @llvm.ssub.sat.i64(i64 %235, i64 %221)
  %237 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !6612
  br label %238

238:                                              ; preds = %241, %234
  %239 = phi i64 [ %237, %234 ], [ %244, %241 ]
  %240 = icmp slt i64 %236, %239
  br i1 %240, label %241, label %245

241:                                              ; preds = %238
  %242 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %239, i64 %236 monotonic monotonic, align 8, !noalias !6612
  %243 = extractvalue { i64, i1 } %242, 1
  %244 = extractvalue { i64, i1 } %242, 0
  br i1 %243, label %245, label %238

245:                                              ; preds = %241, %238
  %246 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !6612
  br label %__rustc::__rust_dealloc (.exit137)

__rustc::__rust_dealloc (.exit137): ; preds = %227, %245
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %117) ], !noalias !6612
  tail call void @free(ptr noundef nonnull %117) #92, !noalias !6612
  br label %1711

247:                                              ; preds = %249, %114
  %248 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #90, !noalias !6577
  unreachable

249:                                              ; preds = %72
  %250 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %3) #89
          to label %1724 unwind label %247, !noalias !6574

251:                                              ; preds = %68
  call void @llvm.lifetime.start.p0(ptr nonnull %62)
  call void @llvm.lifetime.start.p0(ptr nonnull %61)
  %252 = load i64, ptr %4, align 8
  %253 = getelementptr inbounds nuw i8, ptr %4, i64 8
  %254 = load i64, ptr %253, align 8
  %255 = getelementptr inbounds nuw i8, ptr %4, i64 16
  %256 = load i64, ptr %255, align 8
  %257 = getelementptr inbounds nuw i8, ptr %4, i64 24
  %258 = getelementptr inbounds nuw i8, ptr %61, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(184) %258, ptr noundef nonnull align 8 dereferenceable(184) %257, i64 184, i1 false)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6616)
  %259 = icmp ugt i64 %252, 2
  %260 = select i1 %259, i64 %256, i64 %252
  %261 = add i64 %260, -1
  %262 = select i1 %259, i64 %252, i64 1
  %263 = select i1 %259, i64 1, i64 %256
  store i64 %262, ptr %61, align 8, !alias.scope !6619
  %264 = getelementptr inbounds nuw i8, ptr %61, i64 8
  store i64 %254, ptr %264, align 8, !alias.scope !6619
  %265 = getelementptr inbounds nuw i8, ptr %61, i64 16
  store i64 %263, ptr %265, align 8, !alias.scope !6619
  %266 = getelementptr inbounds nuw i8, ptr %61, i64 208
  store i64 0, ptr %266, align 8, !alias.scope !6621, !noalias !6616
  %267 = getelementptr inbounds nuw i8, ptr %61, i64 216
  store i64 %261, ptr %267, align 8, !alias.scope !6621, !noalias !6616
; invoke <purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
  invoke fastcc void @<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %62, ptr noalias nofree noundef align 8 captures(address) dereferenceable(224) %61)
          to label %268 unwind label %1722

268:                                              ; preds = %251
  call void @llvm.lifetime.end.p0(ptr nonnull %61)
  %269 = getelementptr inbounds nuw i8, ptr %62, i64 8
  %270 = load ptr, ptr %269, align 8, !nonnull !1733, !noundef !1733
  %271 = getelementptr inbounds nuw i8, ptr %62, i64 16
  %272 = load i64, ptr %271, align 8, !noundef !1733
  %273 = mul nuw nsw i64 %272, 200
  %274 = getelementptr inbounds nuw i8, ptr %270, i64 %273
  %275 = icmp eq i64 %272, 0
  br i1 %275, label %344, label %.preheader178.preheader

.preheader178.preheader:                          ; preds = %268
  %xtraiter = and i64 %272, 3
  %276 = icmp ult i64 %272, 4
  br i1 %276, label %.preheader178.epil.preheader, label %.preheader178.preheader.new

.preheader178.preheader.new:                      ; preds = %.preheader178.preheader
  %unroll_iter = and i64 %272, -4
  br label %.preheader178

.preheader178:                                    ; preds = %321, %.preheader178.preheader.new
  %277 = phi i64 [ 0, %.preheader178.preheader.new ], [ %324, %321 ]
  %278 = phi i64 [ 0, %.preheader178.preheader.new ], [ %323, %321 ]
  %niter = phi i64 [ 0, %.preheader178.preheader.new ], [ %niter.next.3, %321 ]
  %279 = getelementptr inbounds nuw [200 x i8], ptr %270, i64 %277
  %280 = getelementptr i8, ptr %279, i64 168
  %281 = load i64, ptr %280, align 8, !noundef !1733
  %282 = getelementptr i8, ptr %279, i64 176
  %283 = load i64, ptr %282, align 8, !noundef !1733
  %284 = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %283, i64 %281)
  %285 = extractvalue { i64, i1 } %284, 0
  %286 = extractvalue { i64, i1 } %284, 1
  br i1 %286, label %287, label %.preheader178.1, !prof !1735

287:                                              ; preds = %.preheader178
  br label %.preheader178.1

.preheader178.1:                                  ; preds = %287, %.preheader178
  %288 = phi i64 [ -1, %287 ], [ %285, %.preheader178 ]
  %289 = tail call noundef i64 @llvm.uadd.sat.i64(i64 %278, i64 %288)
  %290 = getelementptr inbounds nuw [200 x i8], ptr %270, i64 %277
  %291 = getelementptr i8, ptr %290, i64 368
  %292 = load i64, ptr %291, align 8, !noundef !1733
  %293 = getelementptr i8, ptr %290, i64 376
  %294 = load i64, ptr %293, align 8, !noundef !1733
  %295 = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %294, i64 %292)
  %296 = extractvalue { i64, i1 } %295, 0
  %297 = extractvalue { i64, i1 } %295, 1
  br i1 %297, label %298, label %.preheader178.2, !prof !1735

298:                                              ; preds = %.preheader178.1
  br label %.preheader178.2

.preheader178.2:                                  ; preds = %298, %.preheader178.1
  %299 = phi i64 [ -1, %298 ], [ %296, %.preheader178.1 ]
  %300 = tail call noundef i64 @llvm.uadd.sat.i64(i64 %289, i64 %299)
  %301 = getelementptr inbounds nuw [200 x i8], ptr %270, i64 %277
  %302 = getelementptr i8, ptr %301, i64 568
  %303 = load i64, ptr %302, align 8, !noundef !1733
  %304 = getelementptr i8, ptr %301, i64 576
  %305 = load i64, ptr %304, align 8, !noundef !1733
  %306 = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %305, i64 %303)
  %307 = extractvalue { i64, i1 } %306, 0
  %308 = extractvalue { i64, i1 } %306, 1
  br i1 %308, label %309, label %.preheader178.3, !prof !1735

309:                                              ; preds = %.preheader178.2
  br label %.preheader178.3

.preheader178.3:                                  ; preds = %309, %.preheader178.2
  %310 = phi i64 [ -1, %309 ], [ %307, %.preheader178.2 ]
  %311 = tail call noundef i64 @llvm.uadd.sat.i64(i64 %300, i64 %310)
  %312 = getelementptr inbounds nuw [200 x i8], ptr %270, i64 %277
  %313 = getelementptr i8, ptr %312, i64 768
  %314 = load i64, ptr %313, align 8, !noundef !1733
  %315 = getelementptr i8, ptr %312, i64 776
  %316 = load i64, ptr %315, align 8, !noundef !1733
  %317 = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %316, i64 %314)
  %318 = extractvalue { i64, i1 } %317, 0
  %319 = extractvalue { i64, i1 } %317, 1
  br i1 %319, label %320, label %321, !prof !1735

320:                                              ; preds = %.preheader178.3
  br label %321

321:                                              ; preds = %320, %.preheader178.3
  %322 = phi i64 [ -1, %320 ], [ %318, %.preheader178.3 ]
  %323 = tail call noundef i64 @llvm.uadd.sat.i64(i64 %311, i64 %322)
  %324 = add nuw i64 %277, 4
  %niter.next.3 = add i64 %niter, 4
  %niter.ncmp.3 = icmp eq i64 %niter.next.3, %unroll_iter
  br i1 %niter.ncmp.3, label %.unr-lcssa, label %.preheader178

.unr-lcssa:                                       ; preds = %321
  %lcmp.mod.not = icmp eq i64 %xtraiter, 0
  br i1 %lcmp.mod.not, label %.epilog-lcssa, label %.preheader178.epil.preheader

.preheader178.epil.preheader:                     ; preds = %.unr-lcssa, %.preheader178.preheader
  %.epil.init = phi i64 [ 0, %.preheader178.preheader ], [ %324, %.unr-lcssa ]
  %.epil.init1893 = phi i64 [ 0, %.preheader178.preheader ], [ %323, %.unr-lcssa ]
  %lcmp.mod1895 = icmp ne i64 %xtraiter, 0
  tail call void @llvm.assume(i1 %lcmp.mod1895)
  br label %.preheader178.epil

.preheader178.epil:                               ; preds = %336, %.preheader178.epil.preheader
  %325 = phi i64 [ %339, %336 ], [ %.epil.init, %.preheader178.epil.preheader ]
  %326 = phi i64 [ %338, %336 ], [ %.epil.init1893, %.preheader178.epil.preheader ]
  %epil.iter = phi i64 [ %epil.iter.next, %336 ], [ 0, %.preheader178.epil.preheader ]
  %327 = getelementptr inbounds nuw [200 x i8], ptr %270, i64 %325
  %328 = getelementptr i8, ptr %327, i64 168
  %329 = load i64, ptr %328, align 8, !noundef !1733
  %330 = getelementptr i8, ptr %327, i64 176
  %331 = load i64, ptr %330, align 8, !noundef !1733
  %332 = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %331, i64 %329)
  %333 = extractvalue { i64, i1 } %332, 0
  %334 = extractvalue { i64, i1 } %332, 1
  br i1 %334, label %335, label %336, !prof !1735

335:                                              ; preds = %.preheader178.epil
  br label %336

336:                                              ; preds = %335, %.preheader178.epil
  %337 = phi i64 [ -1, %335 ], [ %333, %.preheader178.epil ]
  %338 = tail call noundef i64 @llvm.uadd.sat.i64(i64 %326, i64 %337)
  %339 = add nuw i64 %325, 1
  %epil.iter.next = add i64 %epil.iter, 1
  %epil.iter.cmp.not = icmp eq i64 %epil.iter.next, %xtraiter
  br i1 %epil.iter.cmp.not, label %.epilog-lcssa, label %.preheader178.epil, !llvm.loop !6622

.epilog-lcssa:                                    ; preds = %336, %.unr-lcssa
  %.lcssa1891 = phi i64 [ %323, %.unr-lcssa ], [ %338, %336 ]
  %340 = load ptr, ptr %69, align 16, !noundef !1733
  %341 = icmp eq ptr %340, null
  %342 = icmp eq i64 %.lcssa1891, 0
  %343 = or i1 %342, %341
  br i1 %343, label %344, label %396

344:                                              ; preds = %400, %396, %.epilog-lcssa, %268
  %345 = load i64, ptr %62, align 8, !range !1828, !noundef !1733
  %346 = icmp ult i64 %272, 46116860184273880
  tail call void @llvm.assume(i1 %346)
  call void @llvm.lifetime.start.p0(ptr nonnull %51), !noalias !6623
  store i64 0, ptr %51, align 8, !noalias !6623
  %347 = getelementptr inbounds nuw i8, ptr %51, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %347, align 8, !noalias !6623
  %348 = getelementptr inbounds nuw i8, ptr %51, i64 16
  store i64 0, ptr %348, align 8, !noalias !6623
  call void @llvm.lifetime.start.p0(ptr nonnull %50), !noalias !6623
  store ptr %270, ptr %50, align 8, !noalias !6627
  %349 = getelementptr inbounds nuw i8, ptr %50, i64 8
  %350 = getelementptr inbounds nuw i8, ptr %50, i64 16
  store i64 %345, ptr %350, align 8, !noalias !6627
  %351 = getelementptr inbounds nuw i8, ptr %50, i64 24
  store ptr %274, ptr %351, align 8, !noalias !6627
  br i1 %275, label %.loopexit174, label %352

352:                                              ; preds = %344
  %353 = getelementptr inbounds nuw i8, ptr %49, i64 8
  %354 = getelementptr inbounds nuw i8, ptr %49, i64 152
  br label %359

355:                                              ; preds = %366, %357
  %356 = phi { ptr, i32 } [ %358, %357 ], [ %376, %366 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(24) %51) #89
          to label %1728 unwind label %394, !noalias !6623

357:                                              ; preds = %382
  %358 = landingpad { ptr, i32 }
          cleanup
  br label %355

359:                                              ; preds = %391, %352
  %360 = phi ptr [ inttoptr (i64 8 to ptr), %352 ], [ %387, %391 ]
  %361 = phi i64 [ 0, %352 ], [ %389, %391 ]
  %362 = phi ptr [ %270, %352 ], [ %363, %391 ]
  %363 = getelementptr inbounds nuw i8, ptr %362, i64 200
  %364 = load i64, ptr %362, align 8, !noalias !6628
  %365 = icmp eq i64 %364, -1
  br i1 %365, label %.loopexit174, label %367

366:                                              ; preds = %375
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %50)
          to label %355 unwind label %394, !noalias !6623

367:                                              ; preds = %359
  %368 = getelementptr inbounds nuw i8, ptr %362, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %49), !noalias !6623
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(152) %353, ptr noundef nonnull align 8 dereferenceable(152) %368, i64 152, i1 false), !noalias !6623
  store i64 %364, ptr %49, align 8, !noalias !6623
  %369 = load i8, ptr %354, align 8, !range !1740, !noalias !6623, !noundef !1733
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6634)
  %370 = load i64, ptr %51, align 8, !range !1828, !alias.scope !6634, !noalias !6637, !noundef !1733
  %371 = icmp eq i64 %361, %370
  br i1 %371, label %372, label %386

372:                                              ; preds = %367
; invoke <alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %51)
          to label %373 unwind label %375, !noalias !6637

373:                                              ; preds = %372
  %374 = load ptr, ptr %347, align 8, !alias.scope !6634, !noalias !6637
  br label %386

375:                                              ; preds = %372
  %376 = landingpad { ptr, i32 }
          cleanup
  store ptr %363, ptr %349, align 8, !noalias !6623
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(160) %49) #89
          to label %366 unwind label %377, !noalias !6639

377:                                              ; preds = %375
  %378 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #90, !noalias !6640
  unreachable

.loopexit174:                                     ; preds = %391, %359, %344
  %379 = phi i64 [ 0, %344 ], [ %361, %359 ], [ %389, %391 ]
  %380 = phi ptr [ inttoptr (i64 8 to ptr), %344 ], [ %360, %359 ], [ %387, %391 ]
  %381 = phi ptr [ %270, %344 ], [ %363, %359 ], [ %274, %391 ]
  store ptr %381, ptr %349, align 8, !noalias !6623
  br label %382

382:                                              ; preds = %393, %.loopexit174
  %383 = phi i64 [ %389, %393 ], [ %379, %.loopexit174 ]
  %384 = phi ptr [ %387, %393 ], [ %380, %.loopexit174 ]
  %385 = phi i8 [ 1, %393 ], [ 0, %.loopexit174 ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %50)
          to label %403 unwind label %357, !noalias !6623

386:                                              ; preds = %373, %367
  %387 = phi ptr [ %374, %373 ], [ %360, %367 ]
  %388 = getelementptr inbounds nuw [160 x i8], ptr %387, i64 %361
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(160) %388, ptr noundef nonnull readonly align 8 dereferenceable(160) %49, i64 160, i1 false), !noalias !6639
  %389 = add nuw nsw i64 %361, 1
  store i64 %389, ptr %348, align 8, !alias.scope !6634, !noalias !6637
  %390 = trunc nuw i8 %369 to i1
  br i1 %390, label %393, label %391

391:                                              ; preds = %386
  call void @llvm.lifetime.end.p0(ptr nonnull %49), !noalias !6623
  %392 = icmp eq ptr %363, %274
  br i1 %392, label %.loopexit174, label %359

393:                                              ; preds = %386
  store ptr %363, ptr %349, align 8, !noalias !6623
  call void @llvm.lifetime.end.p0(ptr nonnull %49), !noalias !6623
  br label %382

394:                                              ; preds = %366, %355
  %395 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #90, !noalias !6623
  unreachable

396:                                              ; preds = %.epilog-lcssa
  %397 = getelementptr inbounds nuw i8, ptr %340, i64 336
  %398 = load ptr, ptr %397, align 8, !noundef !1733
  %399 = icmp eq ptr %398, null
  br i1 %399, label %344, label %400

400:                                              ; preds = %396
  %401 = getelementptr inbounds nuw i8, ptr %340, i64 352
  %402 = atomicrmw add ptr %401, i64 %.lcssa1891 monotonic, align 8
  br label %344

403:                                              ; preds = %382
  call void @llvm.lifetime.end.p0(ptr nonnull %50), !noalias !6623
  %404 = load i64, ptr %51, align 8, !noalias !6641
  call void @llvm.lifetime.end.p0(ptr nonnull %51), !noalias !6623
  %405 = getelementptr inbounds nuw i8, ptr %1, i64 193
  %406 = load i8, ptr %405, align 1, !range !1740, !noundef !1733
  %407 = trunc nuw i8 %406 to i1
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %384) ]
  %408 = icmp ne i64 %383, 0
  br i1 %408, label %iter.check, label %.loopexit173

iter.check:                                       ; preds = %403
  %min.iters.check = icmp ult i64 %383, 8
  br i1 %min.iters.check, label %.preheader172.preheader, label %vector.main.loop.iter.check

vector.main.loop.iter.check:                      ; preds = %iter.check
  %min.iters.check1495 = icmp ult i64 %383, 32
  br i1 %min.iters.check1495, label %vec.epilog.ph, label %vector.ph

vector.ph:                                        ; preds = %vector.main.loop.iter.check
  %n.mod.vf = and i64 %383, 24
  %n.vec = and i64 %383, -32
  br label %vector.body

vector.body:                                      ; preds = %vector.body, %vector.ph
  %index = phi i64 [ 0, %vector.ph ], [ %index.next, %vector.body ]
  %vec.ind = phi <8 x i64> [ <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>, %vector.ph ], [ %vec.ind.next, %vector.body ]
  %vec.phi = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %409, %vector.body ]
  %vec.phi1496 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %410, %vector.body ]
  %vec.phi1497 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %411, %vector.body ]
  %vec.phi1498 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %412, %vector.body ]
  %step.add = add nuw <8 x i64> %vec.ind, splat (i64 8)
  %step.add.2 = add nuw <8 x i64> %vec.ind, splat (i64 16)
  %step.add.3 = add nuw <8 x i64> %vec.ind, splat (i64 24)
  %wide.gep = getelementptr inbounds nuw [160 x i8], ptr %384, <8 x i64> %vec.ind
  %wide.gep1499 = getelementptr inbounds nuw [160 x i8], ptr %384, <8 x i64> %step.add
  %wide.gep1500 = getelementptr inbounds nuw [160 x i8], ptr %384, <8 x i64> %step.add.2
  %wide.gep1501 = getelementptr inbounds nuw [160 x i8], ptr %384, <8 x i64> %step.add.3
  %wide.gep1502 = getelementptr i8, <8 x ptr> %wide.gep, i64 64
  %wide.gep1503 = getelementptr i8, <8 x ptr> %wide.gep1499, i64 64
  %wide.gep1504 = getelementptr i8, <8 x ptr> %wide.gep1500, i64 64
  %wide.gep1505 = getelementptr i8, <8 x ptr> %wide.gep1501, i64 64
  %wide.masked.gather = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1502, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !6642
  %wide.masked.gather1506 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1503, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !6642
  %wide.masked.gather1507 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1504, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !6642
  %wide.masked.gather1508 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1505, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !6642
  %409 = add <8 x i64> %wide.masked.gather, %vec.phi
  %410 = add <8 x i64> %wide.masked.gather1506, %vec.phi1496
  %411 = add <8 x i64> %wide.masked.gather1507, %vec.phi1497
  %412 = add <8 x i64> %wide.masked.gather1508, %vec.phi1498
  %index.next = add nuw i64 %index, 32
  %vec.ind.next = add nuw <8 x i64> %vec.ind, splat (i64 32)
  %413 = icmp eq i64 %index.next, %n.vec
  br i1 %413, label %middle.block, label %vector.body, !llvm.loop !6645

middle.block:                                     ; preds = %vector.body
  %bin.rdx = add <8 x i64> %410, %409
  %bin.rdx1509 = add <8 x i64> %411, %bin.rdx
  %bin.rdx1510 = add <8 x i64> %412, %bin.rdx1509
  %414 = tail call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %bin.rdx1510)
  %cmp.n = icmp eq i64 %383, %n.vec
  br i1 %cmp.n, label %.loopexit173, label %vec.epilog.iter.check

vec.epilog.iter.check:                            ; preds = %middle.block
  %min.epilog.iters.check = icmp eq i64 %n.mod.vf, 0
  br i1 %min.epilog.iters.check, label %.preheader172.preheader, label %vec.epilog.ph, !prof !6205

vec.epilog.ph:                                    ; preds = %vector.main.loop.iter.check, %vec.epilog.iter.check
  %vec.epilog.resume.val = phi i64 [ %n.vec, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %bc.merge.rdx = phi i64 [ %414, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %n.vec1512 = and i64 %383, -8
  %415 = insertelement <8 x i64> <i64 poison, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0>, i64 %bc.merge.rdx, i64 0
  %broadcast.splatinsert = insertelement <8 x i64> poison, i64 %vec.epilog.resume.val, i64 0
  %broadcast.splat = shufflevector <8 x i64> %broadcast.splatinsert, <8 x i64> poison, <8 x i32> zeroinitializer
  %induction = or disjoint <8 x i64> %broadcast.splat, <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>
  br label %vec.epilog.vector.body

vec.epilog.vector.body:                           ; preds = %vec.epilog.vector.body, %vec.epilog.ph
  %index1513 = phi i64 [ %vec.epilog.resume.val, %vec.epilog.ph ], [ %index.next1519, %vec.epilog.vector.body ]
  %vec.ind1514 = phi <8 x i64> [ %induction, %vec.epilog.ph ], [ %vec.ind.next1520, %vec.epilog.vector.body ]
  %vec.phi1515 = phi <8 x i64> [ %415, %vec.epilog.ph ], [ %416, %vec.epilog.vector.body ]
  %wide.gep1516 = getelementptr inbounds nuw [160 x i8], ptr %384, <8 x i64> %vec.ind1514
  %wide.gep1517 = getelementptr i8, <8 x ptr> %wide.gep1516, i64 64
  %wide.masked.gather1518 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1517, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !6642
  %416 = add <8 x i64> %wide.masked.gather1518, %vec.phi1515
  %index.next1519 = add nuw i64 %index1513, 8
  %vec.ind.next1520 = add nuw <8 x i64> %vec.ind1514, splat (i64 8)
  %417 = icmp eq i64 %index.next1519, %n.vec1512
  br i1 %417, label %vec.epilog.middle.block, label %vec.epilog.vector.body, !llvm.loop !6646

vec.epilog.middle.block:                          ; preds = %vec.epilog.vector.body
  %418 = tail call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %416)
  %cmp.n1521 = icmp eq i64 %383, %n.vec1512
  br i1 %cmp.n1521, label %.loopexit173, label %.preheader172.preheader

.preheader172.preheader:                          ; preds = %iter.check, %vec.epilog.iter.check, %vec.epilog.middle.block
  %.ph1875 = phi i64 [ 0, %iter.check ], [ %n.vec, %vec.epilog.iter.check ], [ %n.vec1512, %vec.epilog.middle.block ]
  %.ph1876 = phi i64 [ 0, %iter.check ], [ %414, %vec.epilog.iter.check ], [ %418, %vec.epilog.middle.block ]
  br label %.preheader172

.preheader172:                                    ; preds = %.preheader172.preheader, %.preheader172
  %419 = phi i64 [ %426, %.preheader172 ], [ %.ph1875, %.preheader172.preheader ]
  %420 = phi i64 [ %425, %.preheader172 ], [ %.ph1876, %.preheader172.preheader ]
  %421 = getelementptr inbounds nuw [160 x i8], ptr %384, i64 %419
  %422 = getelementptr i8, ptr %421, i64 64
  %423 = load i64, ptr %422, align 8, !noalias !6642, !noundef !1733
  %424 = icmp ult i64 %423, 288230376151711744
  tail call void @llvm.assume(i1 %424)
  %425 = add i64 %423, %420
  %426 = add nuw i64 %419, 1
  %427 = icmp eq i64 %426, %383
  br i1 %427, label %.loopexit173, label %.preheader172, !llvm.loop !6647

.loopexit173:                                     ; preds = %.preheader172, %middle.block, %vec.epilog.middle.block, %403
  %428 = phi i64 [ 0, %403 ], [ %418, %vec.epilog.middle.block ], [ %414, %middle.block ], [ %425, %.preheader172 ]
  %429 = trunc nuw i8 %385 to i1
  call void @llvm.lifetime.start.p0(ptr nonnull %60)
  %430 = getelementptr inbounds nuw i8, ptr %1, i64 195
  %431 = load i8, ptr %430, align 1, !range !6208, !noundef !1733
  call void @llvm.lifetime.start.p0(ptr nonnull %59)
  store i64 %404, ptr %59, align 8
  %432 = getelementptr inbounds nuw i8, ptr %59, i64 8
  store ptr %384, ptr %432, align 8
  %433 = getelementptr inbounds nuw i8, ptr %59, i64 16
  store i64 %383, ptr %433, align 8
  %434 = getelementptr inbounds nuw i8, ptr %59, i64 24
  store i8 %385, ptr %434, align 8
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6648)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6651)
  call void @llvm.lifetime.start.p0(ptr nonnull %28)
  call void @llvm.lifetime.start.p0(ptr nonnull %41)
  call void @llvm.lifetime.start.p0(ptr nonnull %48), !noalias !6653
  call void @llvm.lifetime.start.p0(ptr nonnull %47), !noalias !6653
  call void @llvm.lifetime.start.p0(ptr nonnull %46), !noalias !6653
  store i64 0, ptr %46, align 8, !noalias !6653
  %435 = getelementptr inbounds nuw i8, ptr %46, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %435, align 8, !noalias !6653
  %436 = getelementptr inbounds nuw i8, ptr %46, i64 16
  store i64 0, ptr %436, align 8, !noalias !6653
  %437 = getelementptr inbounds nuw i8, ptr %3, i64 16
  %438 = load i64, ptr %437, align 8, !alias.scope !6651, !noalias !6656, !noundef !1733
  %439 = icmp ult i64 %438, 230584300921369396
  tail call void @llvm.assume(i1 %439)
; invoke purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
  invoke fastcc void @purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %47, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %46, i64 noundef %438)
          to label %440 unwind label %1641, !noalias !6653

440:                                              ; preds = %.loopexit173
  call void @llvm.lifetime.end.p0(ptr nonnull %46), !noalias !6653
  %441 = load i64, ptr %47, align 16, !range !2520, !noalias !6653, !noundef !1733
  %442 = icmp eq i64 %441, -1
  %443 = getelementptr inbounds nuw i8, ptr %47, i64 8
  %444 = load i64, ptr %443, align 8, !noalias !6653
  %445 = getelementptr inbounds nuw i8, ptr %47, i64 16
  %446 = load ptr, ptr %445, align 16, !noalias !6653
  %447 = getelementptr inbounds nuw i8, ptr %47, i64 24
  %448 = load i64, ptr %447, align 8, !noalias !6653
  br i1 %442, label %453, label %449

449:                                              ; preds = %440
  %450 = getelementptr inbounds nuw i8, ptr %47, i64 32
  %451 = load i8, ptr %450, align 16, !noalias !6657
  %452 = getelementptr inbounds nuw i8, ptr %47, i64 33
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(63) %60, ptr noundef nonnull align 1 dereferenceable(63) %452, i64 63, i1 false), !noalias !6657
  call void @llvm.lifetime.end.p0(ptr nonnull %47), !noalias !6653
  call void @llvm.lifetime.end.p0(ptr nonnull %48), !noalias !6653
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %3)
          to label %1585 unwind label %1583, !noalias !6658

453:                                              ; preds = %440
  call void @llvm.lifetime.end.p0(ptr nonnull %47), !noalias !6653
  store i64 %444, ptr %48, align 8, !noalias !6653
  %454 = getelementptr inbounds nuw i8, ptr %48, i64 8
  store ptr %446, ptr %454, align 8, !noalias !6653
  %455 = getelementptr inbounds nuw i8, ptr %48, i64 16
  store i64 %448, ptr %455, align 8, !noalias !6653
  call void @llvm.lifetime.start.p0(ptr nonnull %45), !noalias !6653
  %456 = getelementptr inbounds nuw i8, ptr %3, i64 8
  %457 = load ptr, ptr %456, align 8, !alias.scope !6651, !noalias !6656, !nonnull !1733, !noundef !1733
  %458 = load i64, ptr %3, align 8, !range !1828, !alias.scope !6651, !noalias !6656, !noundef !1733
  %459 = getelementptr inbounds nuw [40 x i8], ptr %457, i64 %438
  store ptr %457, ptr %45, align 8, !noalias !6653
  %460 = getelementptr inbounds nuw i8, ptr %45, i64 16
  store i64 %458, ptr %460, align 8, !noalias !6653
  %461 = getelementptr inbounds nuw i8, ptr %45, i64 8
  store ptr %457, ptr %461, align 8, !noalias !6653
  %462 = getelementptr inbounds nuw i8, ptr %45, i64 24
  store ptr %459, ptr %462, align 8, !noalias !6653
  %463 = load ptr, ptr %69, align 16, !alias.scope !6648, !noalias !6659, !noundef !1733
  %464 = icmp eq ptr %463, null
  br i1 %464, label %468, label %465

465:                                              ; preds = %453
  %466 = atomicrmw add ptr %463, i64 1 monotonic, align 8, !noalias !6659
  %467 = icmp slt i64 %466, 0
  br i1 %467, label %583, label %564

468:                                              ; preds = %453
  call void @llvm.lifetime.start.p0(ptr nonnull %44), !noalias !6653
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %44, ptr noundef nonnull align 8 dereferenceable(32) %45, i64 32, i1 false), !noalias !6653
  %469 = getelementptr inbounds nuw i8, ptr %44, i64 24
  %470 = load ptr, ptr %469, align 8, !alias.scope !6660, !noalias !6663, !nonnull !1733, !noundef !1733
  %471 = getelementptr inbounds nuw i8, ptr %44, i64 8
  %472 = load ptr, ptr %471, align 8, !alias.scope !6660, !noalias !6663
  %473 = icmp eq ptr %472, %470
  br i1 %473, label %.loopexit147, label %474

474:                                              ; preds = %468
  %475 = getelementptr inbounds nuw i8, ptr %42, i64 8
  %476 = getelementptr inbounds nuw i8, ptr %43, i64 8
  %477 = getelementptr inbounds nuw i8, ptr %2, i64 656
  %478 = getelementptr inbounds nuw i8, ptr %2, i64 880
  %479 = getelementptr inbounds nuw i8, ptr %42, i64 16
  %480 = getelementptr inbounds nuw i8, ptr %43, i64 16
  %481 = getelementptr inbounds nuw i8, ptr %43, i64 24
  %482 = getelementptr inbounds nuw i8, ptr %43, i64 32
  %483 = getelementptr inbounds nuw i8, ptr %43, i64 33
  br label %488

484:                                              ; preds = %495
  %485 = landingpad { ptr, i32 }
          cleanup
  store ptr %492, ptr %471, align 8, !noalias !6653
  br label %486

486:                                              ; preds = %541, %538, %484
  %487 = phi { ptr, i32 } [ %485, %484 ], [ %539, %541 ], [ %539, %538 ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %44) #89
          to label %1576 unwind label %562, !noalias !6659

488:                                              ; preds = %544, %474
  %489 = phi ptr [ %446, %474 ], [ %545, %544 ]
  %490 = phi i64 [ %448, %474 ], [ %551, %544 ]
  %491 = phi ptr [ %472, %474 ], [ %492, %544 ]
  %492 = getelementptr inbounds nuw i8, ptr %491, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %42), !noalias !6653
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %475, ptr noundef nonnull align 8 dereferenceable(40) %491, i64 40, i1 false), !noalias !6659
  call void @llvm.lifetime.start.p0(ptr nonnull %43), !noalias !6653
  store ptr %2, ptr %42, align 8, !noalias !6653
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6665)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6668)
  %493 = load i64, ptr %475, align 8, !alias.scope !6668, !noalias !6670, !noundef !1733
  %494 = icmp eq i64 %493, 0
  br i1 %494, label %495, label %497

495:                                              ; preds = %488
  %496 = load ptr, ptr %477, align 16, !alias.scope !6672, !noalias !6673, !nonnull !1733, !align !1829, !noundef !1733
; invoke purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(96) %43, ptr noalias nofree noundef align 8 dereferenceable(184) %478, ptr noundef nonnull align 8 %496, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %479)
          to label %518 unwind label %484, !noalias !6659

497:                                              ; preds = %488
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %476, ptr noundef nonnull align 8 dereferenceable(40) %491, i64 40, i1 false), !noalias !6659
  call void @llvm.lifetime.end.p0(ptr nonnull %42), !noalias !6653
  br label %528

.loopexit147:                                     ; preds = %544, %468
  %498 = phi i64 [ %448, %468 ], [ %551, %544 ]
  %499 = phi ptr [ %472, %468 ], [ %492, %544 ]
  store ptr %499, ptr %471, align 8, !noalias !6653
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %44)
          to label %504 unwind label %500, !noalias !6659

500:                                              ; preds = %1510, %722, %521, %.loopexit147
  %501 = phi i8 [ %1430, %1510 ], [ 0, %722 ], [ 1, %521 ], [ 1, %.loopexit147 ]
  %502 = phi i8 [ 0, %1510 ], [ 0, %722 ], [ 1, %521 ], [ 1, %.loopexit147 ]
  %503 = landingpad { ptr, i32 }
          cleanup
  br i1 %464, label %507, label %1572

504:                                              ; preds = %.loopexit147
  call void @llvm.lifetime.end.p0(ptr nonnull %44), !noalias !6653
  %505 = load i64, ptr %48, align 8, !noalias !6657
  %506 = load ptr, ptr %454, align 8, !noalias !6657
  call void @llvm.lifetime.end.p0(ptr nonnull %45), !noalias !6653
  call void @llvm.lifetime.end.p0(ptr nonnull %48), !noalias !6653
  br label %1585

507:                                              ; preds = %1572, %512, %500
  %508 = phi i8 [ %513, %512 ], [ %1575, %1572 ], [ %501, %500 ]
  %509 = phi i8 [ %514, %512 ], [ %1574, %1572 ], [ %502, %500 ]
  %510 = phi { ptr, i32 } [ %515, %512 ], [ %1573, %1572 ], [ %503, %500 ]
  %511 = trunc nuw i8 %508 to i1
  br i1 %511, label %1576, label %1579

512:                                              ; preds = %1514, %726
  %513 = phi i8 [ %560, %1514 ], [ 0, %726 ]
  %514 = phi i8 [ %561, %1514 ], [ 0, %726 ]
  %515 = landingpad { ptr, i32 }
          cleanup
  br label %507

516:                                              ; preds = %1563, %.loopexit146, %1512
  call void @llvm.lifetime.end.p0(ptr nonnull %48), !noalias !6653
  %517 = trunc nuw i8 %561 to i1
  br i1 %517, label %1585, label %1645

518:                                              ; preds = %495
  %519 = load i64, ptr %43, align 16, !noalias !6653
  call void @llvm.lifetime.end.p0(ptr nonnull %42), !noalias !6653
  %520 = icmp eq i64 %519, -1
  br i1 %520, label %528, label %521

521:                                              ; preds = %518
  store ptr %492, ptr %471, align 8, !noalias !6653
  %522 = load i64, ptr %476, align 8, !noalias !6653
  %523 = load ptr, ptr %480, align 16, !noalias !6653
  %524 = load i64, ptr %481, align 8, !noalias !6653
  %525 = load i8, ptr %482, align 16, !noalias !6653
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(15) %41, ptr noundef nonnull align 1 dereferenceable(15) %483, i64 15, i1 false), !noalias !6653
  %526 = getelementptr inbounds nuw i8, ptr %43, i64 48
  %527 = getelementptr inbounds nuw i8, ptr %60, i64 15
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(48) %527, ptr noundef nonnull align 16 dereferenceable(48) %526, i64 48, i1 false), !noalias !6657
  call void @llvm.lifetime.end.p0(ptr nonnull %43), !noalias !6653
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(15) %60, ptr noundef nonnull align 1 dereferenceable(15) %41, i64 15, i1 false), !noalias !6657
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %44)
          to label %553 unwind label %500, !noalias !6659

528:                                              ; preds = %518, %497
  %529 = load i64, ptr %476, align 8, !noalias !6653
  %530 = load ptr, ptr %480, align 16, !noalias !6653
  %531 = load i64, ptr %481, align 8, !noalias !6653
  %532 = load i8, ptr %482, align 16, !noalias !6653
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(15) %41, ptr noundef nonnull align 1 dereferenceable(15) %483, i64 15, i1 false), !noalias !6653
  call void @llvm.lifetime.end.p0(ptr nonnull %43), !noalias !6653
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6674)
  %533 = load i64, ptr %48, align 8, !range !1828, !alias.scope !6674, !noalias !6677, !noundef !1733
  %534 = icmp eq i64 %490, %533
  br i1 %534, label %535, label %544

535:                                              ; preds = %528
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %48)
          to label %536 unwind label %538, !noalias !6679

536:                                              ; preds = %535
  %537 = load ptr, ptr %454, align 8, !alias.scope !6674, !noalias !6677
  br label %544

538:                                              ; preds = %535
  %539 = landingpad { ptr, i32 }
          cleanup
  store ptr %492, ptr %471, align 8, !noalias !6653
  %540 = icmp ugt i64 %529, 5
  br i1 %540, label %541, label %486

541:                                              ; preds = %538
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %530) ]
  %542 = shl i64 %529, 3
  %543 = add i64 %542, -8
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %530, i64 noundef %543, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !6680
  br label %486

544:                                              ; preds = %536, %528
  %545 = phi ptr [ %537, %536 ], [ %489, %528 ]
  %546 = getelementptr inbounds nuw [40 x i8], ptr %545, i64 %490
  store i64 %529, ptr %546, align 8, !noalias !6683
  %547 = getelementptr inbounds nuw i8, ptr %546, i64 8
  store ptr %530, ptr %547, align 8, !noalias !6683
  %548 = getelementptr inbounds nuw i8, ptr %546, i64 16
  store i64 %531, ptr %548, align 8, !noalias !6659
  %549 = getelementptr inbounds nuw i8, ptr %546, i64 24
  store i8 %532, ptr %549, align 8, !noalias !6659
  %550 = getelementptr inbounds nuw i8, ptr %546, i64 25
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(15) %550, ptr noundef nonnull align 1 dereferenceable(15) %41, i64 15, i1 false), !noalias !6659
  %551 = add i64 %490, 1
  store i64 %551, ptr %455, align 8, !alias.scope !6674, !noalias !6677
  %552 = icmp eq ptr %492, %470
  br i1 %552, label %.loopexit147, label %488

553:                                              ; preds = %521
  call void @llvm.lifetime.end.p0(ptr nonnull %44), !noalias !6653
  br label %554

554:                                              ; preds = %1511, %553
  %555 = phi i8 [ %525, %553 ], [ %1425, %1511 ]
  %556 = phi i64 [ %524, %553 ], [ %1426, %1511 ]
  %557 = phi ptr [ %523, %553 ], [ %1427, %1511 ]
  %558 = phi i64 [ %522, %553 ], [ %1428, %1511 ]
  %559 = phi i64 [ %519, %553 ], [ %1429, %1511 ]
  %560 = phi i8 [ 1, %553 ], [ %1430, %1511 ]
  %561 = phi i8 [ 1, %553 ], [ 0, %1511 ]
  br i1 %464, label %1512, label %1514

562:                                              ; preds = %1643, %1641, %1572, %808, %685, %666, %486
  %563 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !6659
  unreachable

564:                                              ; preds = %465
  %565 = load ptr, ptr %69, align 16, !alias.scope !6648, !noalias !6659, !nonnull !1733, !noundef !1733
  call void @llvm.lifetime.start.p0(ptr nonnull %40), !noalias !6653
  store ptr %565, ptr %40, align 8, !noalias !6653
  %566 = getelementptr inbounds nuw i8, ptr %565, i64 16
  %567 = load i64, ptr %566, align 8, !noalias !6659
  %568 = icmp eq i64 %567, -1
  %569 = getelementptr inbounds nuw i8, ptr %565, i64 40
  %570 = load i64, ptr %569, align 8, !noalias !6659
  %571 = icmp ne i64 %570, -1
  %572 = and i1 %408, %571
  br i1 %572, label %iter.check1560, label %584

iter.check1560:                                   ; preds = %564
  %min.iters.check1523 = icmp ult i64 %383, 8
  br i1 %min.iters.check1523, label %.preheader171.preheader, label %vector.main.loop.iter.check1524

vector.main.loop.iter.check1524:                  ; preds = %iter.check1560
  %min.iters.check1525 = icmp ult i64 %383, 32
  br i1 %min.iters.check1525, label %vec.epilog.ph1564, label %vector.ph1526

vector.ph1526:                                    ; preds = %vector.main.loop.iter.check1524
  %n.mod.vf1527 = and i64 %383, 24
  %n.vec1528 = and i64 %383, -32
  br label %vector.body1529

vector.body1529:                                  ; preds = %vector.body1529, %vector.ph1526
  %index1530 = phi i64 [ 0, %vector.ph1526 ], [ %index.next1551, %vector.body1529 ]
  %vec.ind1531 = phi <8 x i64> [ <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>, %vector.ph1526 ], [ %vec.ind.next1552, %vector.body1529 ]
  %vec.phi1532 = phi <8 x i64> [ zeroinitializer, %vector.ph1526 ], [ %573, %vector.body1529 ]
  %vec.phi1533 = phi <8 x i64> [ zeroinitializer, %vector.ph1526 ], [ %574, %vector.body1529 ]
  %vec.phi1534 = phi <8 x i64> [ zeroinitializer, %vector.ph1526 ], [ %575, %vector.body1529 ]
  %vec.phi1535 = phi <8 x i64> [ zeroinitializer, %vector.ph1526 ], [ %576, %vector.body1529 ]
  %step.add1536 = add nuw <8 x i64> %vec.ind1531, splat (i64 8)
  %step.add.21537 = add nuw <8 x i64> %vec.ind1531, splat (i64 16)
  %step.add.31538 = add nuw <8 x i64> %vec.ind1531, splat (i64 24)
  %wide.gep1539 = getelementptr inbounds nuw [160 x i8], ptr %384, <8 x i64> %vec.ind1531
  %wide.gep1540 = getelementptr inbounds nuw [160 x i8], ptr %384, <8 x i64> %step.add1536
  %wide.gep1541 = getelementptr inbounds nuw [160 x i8], ptr %384, <8 x i64> %step.add.21537
  %wide.gep1542 = getelementptr inbounds nuw [160 x i8], ptr %384, <8 x i64> %step.add.31538
  %wide.gep1543 = getelementptr i8, <8 x ptr> %wide.gep1539, i64 16
  %wide.gep1544 = getelementptr i8, <8 x ptr> %wide.gep1540, i64 16
  %wide.gep1545 = getelementptr i8, <8 x ptr> %wide.gep1541, i64 16
  %wide.gep1546 = getelementptr i8, <8 x ptr> %wide.gep1542, i64 16
  %wide.masked.gather1547 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1543, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !6659
  %wide.masked.gather1548 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1544, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !6659
  %wide.masked.gather1549 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1545, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !6659
  %wide.masked.gather1550 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1546, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !6659
  %573 = add <8 x i64> %wide.masked.gather1547, %vec.phi1532
  %574 = add <8 x i64> %wide.masked.gather1548, %vec.phi1533
  %575 = add <8 x i64> %wide.masked.gather1549, %vec.phi1534
  %576 = add <8 x i64> %wide.masked.gather1550, %vec.phi1535
  %index.next1551 = add nuw i64 %index1530, 32
  %vec.ind.next1552 = add nuw <8 x i64> %vec.ind1531, splat (i64 32)
  %577 = icmp eq i64 %index.next1551, %n.vec1528
  br i1 %577, label %middle.block1553, label %vector.body1529, !llvm.loop !6684

middle.block1553:                                 ; preds = %vector.body1529
  %bin.rdx1554 = add <8 x i64> %574, %573
  %bin.rdx1555 = add <8 x i64> %575, %bin.rdx1554
  %bin.rdx1556 = add <8 x i64> %576, %bin.rdx1555
  %578 = tail call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %bin.rdx1556)
  %cmp.n1557 = icmp eq i64 %383, %n.vec1528
  br i1 %cmp.n1557, label %.loopexit1583, label %vec.epilog.iter.check1562

vec.epilog.iter.check1562:                        ; preds = %middle.block1553
  %min.epilog.iters.check1563 = icmp eq i64 %n.mod.vf1527, 0
  br i1 %min.epilog.iters.check1563, label %.preheader171.preheader, label %vec.epilog.ph1564, !prof !6205

vec.epilog.ph1564:                                ; preds = %vector.main.loop.iter.check1524, %vec.epilog.iter.check1562
  %vec.epilog.resume.val1558 = phi i64 [ %n.vec1528, %vec.epilog.iter.check1562 ], [ 0, %vector.main.loop.iter.check1524 ]
  %bc.merge.rdx1559 = phi i64 [ %578, %vec.epilog.iter.check1562 ], [ 0, %vector.main.loop.iter.check1524 ]
  %n.vec1566 = and i64 %383, -8
  %579 = insertelement <8 x i64> <i64 poison, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0>, i64 %bc.merge.rdx1559, i64 0
  %broadcast.splatinsert1567 = insertelement <8 x i64> poison, i64 %vec.epilog.resume.val1558, i64 0
  %broadcast.splat1568 = shufflevector <8 x i64> %broadcast.splatinsert1567, <8 x i64> poison, <8 x i32> zeroinitializer
  %induction1569 = or disjoint <8 x i64> %broadcast.splat1568, <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>
  br label %vec.epilog.vector.body1570

vec.epilog.vector.body1570:                       ; preds = %vec.epilog.vector.body1570, %vec.epilog.ph1564
  %index1571 = phi i64 [ %vec.epilog.resume.val1558, %vec.epilog.ph1564 ], [ %index.next1577, %vec.epilog.vector.body1570 ]
  %vec.ind1572 = phi <8 x i64> [ %induction1569, %vec.epilog.ph1564 ], [ %vec.ind.next1578, %vec.epilog.vector.body1570 ]
  %vec.phi1573 = phi <8 x i64> [ %579, %vec.epilog.ph1564 ], [ %580, %vec.epilog.vector.body1570 ]
  %wide.gep1574 = getelementptr inbounds nuw [160 x i8], ptr %384, <8 x i64> %vec.ind1572
  %wide.gep1575 = getelementptr i8, <8 x ptr> %wide.gep1574, i64 16
  %wide.masked.gather1576 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1575, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !6659
  %580 = add <8 x i64> %wide.masked.gather1576, %vec.phi1573
  %index.next1577 = add nuw i64 %index1571, 8
  %vec.ind.next1578 = add nuw <8 x i64> %vec.ind1572, splat (i64 8)
  %581 = icmp eq i64 %index.next1577, %n.vec1566
  br i1 %581, label %vec.epilog.middle.block1579, label %vec.epilog.vector.body1570, !llvm.loop !6685

vec.epilog.middle.block1579:                      ; preds = %vec.epilog.vector.body1570
  %582 = tail call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %580)
  %cmp.n1580 = icmp eq i64 %383, %n.vec1566
  br i1 %cmp.n1580, label %.loopexit1583, label %.preheader171.preheader

.preheader171.preheader:                          ; preds = %iter.check1560, %vec.epilog.iter.check1562, %vec.epilog.middle.block1579
  %.ph1867 = phi i64 [ 0, %iter.check1560 ], [ %n.vec1528, %vec.epilog.iter.check1562 ], [ %n.vec1566, %vec.epilog.middle.block1579 ]
  %.ph1868 = phi i64 [ 0, %iter.check1560 ], [ %578, %vec.epilog.iter.check1562 ], [ %582, %vec.epilog.middle.block1579 ]
  br label %.preheader171

583:                                              ; preds = %465
  tail call void @llvm.trap()
  unreachable

584:                                              ; preds = %682, %678, %564
  %585 = icmp ult i64 %383, 57646075230342349
  tail call void @llvm.assume(i1 %585)
  %586 = mul nuw nsw i64 %383, 160
  %587 = getelementptr inbounds nuw i8, ptr %384, i64 %586
  call void @llvm.lifetime.start.p0(ptr nonnull %39), !noalias !6653
  store ptr %384, ptr %39, align 8, !noalias !6653
  %588 = getelementptr inbounds nuw i8, ptr %39, i64 8
  store ptr %384, ptr %588, align 8, !noalias !6653
  %589 = getelementptr inbounds nuw i8, ptr %39, i64 16
  store i64 %404, ptr %589, align 8, !noalias !6653
  %590 = getelementptr inbounds nuw i8, ptr %39, i64 24
  store ptr %587, ptr %590, align 8, !noalias !6653
  %591 = icmp eq i64 %383, 0
  br i1 %591, label %.loopexit170, label %592

592:                                              ; preds = %584
  %593 = getelementptr inbounds nuw i8, ptr %38, i64 8
  %594 = getelementptr inbounds nuw i8, ptr %38, i64 24
  %595 = getelementptr inbounds nuw i8, ptr %38, i64 32
  %596 = getelementptr inbounds nuw i8, ptr %38, i64 40
  %597 = getelementptr inbounds nuw i8, ptr %38, i64 16
  %598 = getelementptr inbounds nuw i8, ptr %37, i64 16
  %599 = getelementptr inbounds nuw i8, ptr %37, i64 8
  %600 = getelementptr inbounds nuw i8, ptr %37, i64 24
  %601 = getelementptr inbounds nuw i8, ptr %38, i64 96
  %602 = getelementptr inbounds nuw i8, ptr %38, i64 48
  %603 = getelementptr inbounds nuw i8, ptr %38, i64 56
  %604 = getelementptr inbounds nuw i8, ptr %38, i64 64
  %605 = getelementptr inbounds nuw i8, ptr %565, i64 80
  %606 = getelementptr inbounds nuw i8, ptr %21, i64 1
  %607 = getelementptr inbounds nuw i8, ptr %21, i64 8
  %608 = getelementptr inbounds nuw i8, ptr %21, i64 16
  %609 = getelementptr inbounds nuw i8, ptr %2, i64 624
  %610 = getelementptr inbounds nuw i8, ptr %2, i64 1220
  %611 = zext nneg i8 %431 to i64
  %612 = getelementptr inbounds nuw i8, ptr %565, i64 296
  %613 = getelementptr inbounds nuw i8, ptr %565, i64 272
  %614 = getelementptr inbounds nuw i8, ptr %2, i64 1040
  %615 = getelementptr inbounds nuw i8, ptr %2, i64 1048
  %616 = getelementptr inbounds nuw i8, ptr %565, i64 104
  %617 = getelementptr inbounds nuw i8, ptr %15, i64 1
  %618 = getelementptr inbounds nuw i8, ptr %15, i64 8
  %619 = getelementptr inbounds nuw i8, ptr %15, i64 16
  %620 = getelementptr inbounds nuw i8, ptr %20, i64 8
  %621 = getelementptr inbounds nuw i8, ptr %2, i64 880
  %622 = getelementptr inbounds nuw i8, ptr %9, i64 1
  %623 = getelementptr inbounds nuw i8, ptr %9, i64 8
  %624 = getelementptr inbounds nuw i8, ptr %9, i64 16
  %625 = getelementptr inbounds nuw i8, ptr %16, i64 8
  %626 = getelementptr inbounds nuw i8, ptr %18, i64 1
  %627 = getelementptr inbounds nuw i8, ptr %18, i64 8
  %628 = getelementptr inbounds nuw i8, ptr %18, i64 16
  %629 = getelementptr inbounds nuw i8, ptr %17, i64 8
  %630 = getelementptr inbounds nuw i8, ptr %14, i64 8
  %631 = getelementptr inbounds nuw i8, ptr %13, i64 8
  %632 = getelementptr inbounds nuw i8, ptr %11, i64 1
  %633 = getelementptr inbounds nuw i8, ptr %11, i64 8
  %634 = getelementptr inbounds nuw i8, ptr %11, i64 16
  %635 = getelementptr inbounds nuw i8, ptr %10, i64 8
  %636 = getelementptr inbounds nuw i8, ptr %29, i64 8
  %637 = getelementptr inbounds nuw i8, ptr %30, i64 8
  %638 = getelementptr inbounds nuw i8, ptr %2, i64 656
  %639 = getelementptr inbounds nuw i8, ptr %29, i64 16
  %640 = getelementptr inbounds nuw i8, ptr %30, i64 16
  %641 = getelementptr inbounds nuw i8, ptr %30, i64 24
  %642 = getelementptr inbounds nuw i8, ptr %38, i64 88
  %643 = getelementptr inbounds nuw i8, ptr %38, i64 120
  %644 = getelementptr inbounds nuw i8, ptr %6, i64 1
  %645 = getelementptr inbounds nuw i8, ptr %6, i64 8
  %646 = getelementptr inbounds nuw i8, ptr %6, i64 16
  %647 = getelementptr inbounds nuw i8, ptr %30, i64 32
  %648 = getelementptr inbounds nuw i8, ptr %30, i64 33
  %649 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %650 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %686

.preheader171:                                    ; preds = %.preheader171.preheader, %.preheader171
  %651 = phi i64 [ %658, %.preheader171 ], [ %.ph1867, %.preheader171.preheader ]
  %652 = phi i64 [ %657, %.preheader171 ], [ %.ph1868, %.preheader171.preheader ]
  %653 = getelementptr inbounds nuw [160 x i8], ptr %384, i64 %651
  %654 = getelementptr i8, ptr %653, i64 16
  %655 = load i64, ptr %654, align 8, !noalias !6659, !noundef !1733
  %656 = icmp ult i64 %655, 104811045873349726
  tail call void @llvm.assume(i1 %656)
  %657 = add i64 %655, %652
  %658 = add nuw i64 %651, 1
  %659 = icmp eq i64 %658, %383
  br i1 %659, label %.loopexit1583, label %.preheader171, !llvm.loop !6686

660:                                              ; preds = %685, %667
  %661 = phi i8 [ %668, %667 ], [ %811, %685 ]
  %662 = phi i8 [ %669, %667 ], [ 0, %685 ]
  %663 = phi { ptr, i32 } [ %670, %667 ], [ %812, %685 ]
  %664 = atomicrmw sub ptr %565, i64 1 release, align 8, !noalias !6687
  %665 = icmp eq i64 %664, 1
  br i1 %665, label %666, label %1572

666:                                              ; preds = %660
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %40) #91
          to label %1572 unwind label %562, !noalias !6659

667:                                              ; preds = %1506, %723, %.loopexit170, %682, %677
  %668 = phi i8 [ %1430, %1506 ], [ 1, %723 ], [ 1, %.loopexit170 ], [ 1, %682 ], [ 1, %677 ]
  %669 = phi i8 [ 0, %1506 ], [ 0, %723 ], [ 0, %.loopexit170 ], [ 1, %682 ], [ 1, %677 ]
  %670 = landingpad { ptr, i32 }
          cleanup
  br label %660

.loopexit1583:                                    ; preds = %.preheader171, %vec.epilog.middle.block1579, %middle.block1553
  %.lcssa1483 = phi i64 [ %582, %vec.epilog.middle.block1579 ], [ %578, %middle.block1553 ], [ %657, %.preheader171 ]
  %671 = getelementptr inbounds nuw i8, ptr %2, i64 904
  %672 = getelementptr inbounds nuw i8, ptr %2, i64 920
  %673 = load i64, ptr %672, align 8, !alias.scope !6692, !noalias !6659, !noundef !1733
  %674 = load i64, ptr %671, align 8, !range !1828, !alias.scope !6692, !noalias !6659, !noundef !1733
  %675 = sub i64 %674, %673
  %676 = icmp ugt i64 %.lcssa1483, %675
  br i1 %676, label %677, label %678, !prof !3840

677:                                              ; preds = %.loopexit1583
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.1794586459888082020)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %671, i64 noundef %673, i64 noundef %.lcssa1483, i64 noundef 8, i64 noundef 80)
          to label %678 unwind label %667, !noalias !6659

678:                                              ; preds = %677, %.loopexit1583
  %679 = getelementptr inbounds nuw i8, ptr %2, i64 1008
  %680 = load i64, ptr %679, align 16, !alias.scope !6697, !noalias !6659, !noundef !1733
  %681 = icmp ugt i64 %.lcssa1483, %680
  br i1 %681, label %682, label %584, !prof !3840

682:                                              ; preds = %678
  %683 = getelementptr inbounds nuw i8, ptr %2, i64 992
; invoke <hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>
  %684 = invoke { i64, i64 } @<hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>(ptr noalias nofree noundef nonnull align 8 dereferenceable(32) %683, i64 noundef %.lcssa1483, ptr noundef nonnull align 8 %671, i1 noundef zeroext true) #91
          to label %584 unwind label %667, !noalias !6659

685:                                              ; preds = %1571, %1568, %1565
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %39) #89
          to label %660 unwind label %562, !noalias !6659

686:                                              ; preds = %871, %592
  %687 = phi ptr [ %457, %592 ], [ %778, %871 ]
  %688 = phi ptr [ %384, %592 ], [ %689, %871 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !6700)
  %689 = getelementptr inbounds nuw i8, ptr %688, i64 160
  store ptr %689, ptr %588, align 8, !alias.scope !6700, !noalias !6703
  %690 = load i64, ptr %688, align 8, !noalias !6705
  %691 = icmp eq i64 %690, -1
  br i1 %691, label %.loopexit170, label %692

692:                                              ; preds = %686
  %693 = getelementptr inbounds nuw i8, ptr %688, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %38), !noalias !6653
  store i64 %690, ptr %38, align 8, !noalias !6653
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(152) %593, ptr noundef nonnull align 8 dereferenceable(152) %693, i64 152, i1 false), !noalias !6659
  %694 = load i64, ptr %594, align 8, !noalias !6653
  %695 = load ptr, ptr %595, align 8, !noalias !6653
  %696 = load i64, ptr %596, align 8, !noalias !6653
  call void @llvm.lifetime.start.p0(ptr nonnull %37), !noalias !6653
  %697 = load ptr, ptr %593, align 8, !noalias !6653, !nonnull !1733, !noundef !1733
  %698 = load i64, ptr %597, align 8, !noalias !6653, !noundef !1733
  %699 = icmp ult i64 %698, 104811045873349726
  call void @llvm.assume(i1 %699)
  %700 = getelementptr inbounds nuw [88 x i8], ptr %697, i64 %698
  store ptr %697, ptr %37, align 8, !noalias !6653
  store i64 %690, ptr %598, align 8, !noalias !6653
  store ptr %697, ptr %599, align 8, !noalias !6653
  store ptr %700, ptr %600, align 8, !noalias !6653
  %701 = load ptr, ptr %603, align 8, !noalias !6653, !nonnull !1733, !noundef !1733
  %702 = load i64, ptr %602, align 8, !range !1828, !noalias !6653, !noundef !1733
  %703 = load i64, ptr %604, align 8, !noalias !6653, !noundef !1733
  %704 = icmp ult i64 %703, 288230376151711744
  call void @llvm.assume(i1 %704)
  %705 = shl nuw nsw i64 %703, 5
  %706 = getelementptr inbounds nuw i8, ptr %701, i64 %705
  %707 = icmp eq i64 %703, 0
  br i1 %707, label %.loopexit169, label %708

708:                                              ; preds = %692
  %709 = load i64, ptr %601, align 8, !noalias !6653, !noundef !1733
  %710 = icmp ult i64 %696, 384307168202282326
  %711 = ptrtoint ptr %700 to i64
  br label %761

.loopexit170:                                     ; preds = %871, %686, %584
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %39)
          to label %712 unwind label %667, !noalias !6659

712:                                              ; preds = %.loopexit170
  call void @llvm.lifetime.end.p0(ptr nonnull %39), !noalias !6653
  %713 = xor i1 %407, true
  %714 = or i1 %429, %713
  %715 = select i1 %714, i1 true, i1 %568
  br i1 %715, label %716, label %723

716:                                              ; preds = %725, %712
  %717 = load i64, ptr %48, align 8, !noalias !6657
  %718 = load ptr, ptr %454, align 8, !noalias !6657
  %719 = load i64, ptr %455, align 8, !noalias !6657
  %720 = atomicrmw sub ptr %565, i64 1 release, align 8, !noalias !6706
  %721 = icmp eq i64 %720, 1
  br i1 %721, label %722, label %726

722:                                              ; preds = %716
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %40) #91
          to label %726 unwind label %500, !noalias !6659

723:                                              ; preds = %712
  call void @llvm.lifetime.start.p0(ptr nonnull %27), !noalias !6653
  call void @llvm.lifetime.start.p0(ptr nonnull %26), !noalias !6653
  store i64 1, ptr %26, align 8, !noalias !6653
  %724 = getelementptr inbounds nuw i8, ptr %26, i64 8
  store i64 0, ptr %724, align 8, !noalias !6653
; invoke <purrdf_sparql_eval::governor::GovernorState>::commit_reported_items
  invoke void @<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items(ptr noalias nofree noundef nonnull sret([40 x i8]) align 8 captures(address) dereferenceable(40) %27, ptr noundef nonnull align 8 %566, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %26, i64 noundef 1)
          to label %725 unwind label %667, !noalias !6659

725:                                              ; preds = %723
  call void @llvm.lifetime.end.p0(ptr nonnull %26), !noalias !6653
  call void @llvm.lifetime.end.p0(ptr nonnull %27), !noalias !6653
  br label %716

726:                                              ; preds = %722, %716
  call void @llvm.lifetime.end.p0(ptr nonnull %40), !noalias !6653
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %45)
          to label %727 unwind label %512, !noalias !6659

727:                                              ; preds = %726
  call void @llvm.lifetime.end.p0(ptr nonnull %45), !noalias !6653
  call void @llvm.lifetime.end.p0(ptr nonnull %48), !noalias !6653
  call void @llvm.lifetime.end.p0(ptr nonnull %28)
  call void @llvm.lifetime.end.p0(ptr nonnull %41)
  call void @llvm.lifetime.end.p0(ptr nonnull %59)
  br label %1658

728:                                              ; preds = %1079
  %729 = landingpad { ptr, i32 }
          cleanup
  store ptr %1075, ptr %599, align 8, !noalias !6653
  br label %756

730:                                              ; preds = %1057
  %731 = landingpad { ptr, i32 }
          cleanup
  br label %756

732:                                              ; preds = %1145
  %733 = landingpad { ptr, i32 }
          cleanup
  store ptr %1141, ptr %599, align 8, !noalias !6653
  br label %756

734:                                              ; preds = %1039
  %735 = landingpad { ptr, i32 }
          cleanup
  store ptr %1035, ptr %599, align 8, !noalias !6653
  br label %756

736:                                              ; preds = %1387
  %737 = landingpad { ptr, i32 }
          cleanup
  store ptr %1384, ptr %461, align 8, !noalias !6653
  br label %756

738:                                              ; preds = %1367
  %739 = landingpad { ptr, i32 }
          cleanup
  store ptr %1363, ptr %599, align 8, !noalias !6653
  br label %756

740:                                              ; preds = %1301
  %741 = landingpad { ptr, i32 }
          cleanup
  br label %756

742:                                              ; preds = %1274
  %743 = landingpad { ptr, i32 }
          cleanup
  store ptr %1270, ptr %599, align 8, !noalias !6653
  br label %756

744:                                              ; preds = %1216
  %745 = landingpad { ptr, i32 }
          cleanup
  store ptr %1212, ptr %599, align 8, !noalias !6653
  br label %756

746:                                              ; preds = %1177, %1161, %1129
  %747 = landingpad { ptr, i32 }
          cleanup
  br label %756

748:                                              ; preds = %.preheader165
  %749 = landingpad { ptr, i32 }
          cleanup
  br label %756

750:                                              ; preds = %892
  %751 = landingpad { ptr, i32 }
          cleanup
  br label %756

752:                                              ; preds = %1346, %1342, %1333
  %753 = landingpad { ptr, i32 }
          cleanup
  br label %756

754:                                              ; preds = %920, %875
  %755 = landingpad { ptr, i32 }
          cleanup
  br label %756

756:                                              ; preds = %1412, %1409, %754, %752, %750, %748, %746, %744, %742, %740, %738, %736, %734, %732, %730, %728
  %757 = phi { ptr, i32 } [ %1410, %1409 ], [ %1410, %1412 ], [ %729, %728 ], [ %731, %730 ], [ %733, %732 ], [ %735, %734 ], [ %737, %736 ], [ %739, %738 ], [ %741, %740 ], [ %743, %742 ], [ %745, %744 ], [ %747, %746 ], [ %749, %748 ], [ %751, %750 ], [ %753, %752 ], [ %755, %754 ]
  %758 = icmp eq i64 %702, 0
  br i1 %758, label %808, label %759

759:                                              ; preds = %756
  %760 = shl nuw i64 %702, 5
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %701, i64 noundef %760, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !6711
  br label %808

761:                                              ; preds = %.loopexit151, %708
  %762 = phi ptr [ %687, %708 ], [ %1381, %.loopexit151 ]
  %763 = phi ptr [ %697, %708 ], [ %1196, %.loopexit151 ]
  %764 = phi i64 [ 0, %708 ], [ %918, %.loopexit151 ]
  %765 = phi ptr [ %701, %708 ], [ %768, %.loopexit151 ]
  %766 = phi ptr [ %697, %708 ], [ %1198, %.loopexit151 ]
  %767 = phi i64 [ %709, %708 ], [ %1197, %.loopexit151 ]
  %768 = getelementptr inbounds nuw i8, ptr %765, i64 32
  %769 = load i64, ptr %765, align 8, !noalias !6714
  %770 = getelementptr inbounds nuw i8, ptr %765, i64 8
  %771 = load i64, ptr %770, align 8, !noalias !6714
  %772 = getelementptr inbounds nuw i8, ptr %765, i64 16
  %773 = load i64, ptr %772, align 8, !noalias !6714
  %774 = getelementptr inbounds nuw i8, ptr %765, i64 24
  %775 = load i64, ptr %774, align 8, !noalias !6714
  %776 = icmp eq i64 %769, 0
  %777 = select i1 %776, i1 true, i1 %568
  br i1 %777, label %873, label %880

.loopexit169:                                     ; preds = %.loopexit151, %692
  %778 = phi ptr [ %687, %692 ], [ %1381, %.loopexit151 ]
  %779 = icmp eq i64 %702, 0
  br i1 %779, label %809, label %780

780:                                              ; preds = %.loopexit169
  %781 = shl nuw i64 %702, 5
  %782 = load i64, ptr %649, align 8, !noalias !6717, !noundef !1733
  %783 = call i64 @llvm.umin.i64(i64 %781, i64 9223372036854775807)
  %784 = call i64 @llvm.ssub.sat.i64(i64 %782, i64 %783)
  store i64 %784, ptr %649, align 8, !noalias !6717
  %785 = load i64, ptr %650, align 8, !noalias !6717, !noundef !1733
  %786 = icmp slt i64 %784, %785
  br i1 %786, label %787, label %.preheader1629

787:                                              ; preds = %780
  store i64 %784, ptr %650, align 8, !noalias !6717
  br label %.preheader1629

.preheader1629:                                   ; preds = %787, %780
  br label %788

788:                                              ; preds = %.preheader1629, %791
  %789 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !6717
  %790 = icmp slt i64 %789, 0
  br i1 %790, label %791, label %__rustc::__rust_dealloc (.exit138)

791:                                              ; preds = %788
  %792 = add nsw i64 %789, 1
  %793 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %789, i64 %792 acq_rel acquire, align 8, !noalias !6717
  %794 = extractvalue { i64, i1 } %793, 1
  br i1 %794, label %795, label %788

795:                                              ; preds = %791
  %796 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %783 monotonic, align 8, !noalias !6717
  %797 = call i64 @llvm.ssub.sat.i64(i64 %796, i64 %783)
  %798 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !6717
  br label %799

799:                                              ; preds = %802, %795
  %800 = phi i64 [ %798, %795 ], [ %805, %802 ]
  %801 = icmp slt i64 %797, %800
  br i1 %801, label %802, label %806

802:                                              ; preds = %799
  %803 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %800, i64 %797 monotonic monotonic, align 8, !noalias !6717
  %804 = extractvalue { i64, i1 } %803, 1
  %805 = extractvalue { i64, i1 } %803, 0
  br i1 %804, label %806, label %799

806:                                              ; preds = %802, %799
  %807 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !6717
  br label %__rustc::__rust_dealloc (.exit138)

__rustc::__rust_dealloc (.exit138): ; preds = %788, %806
  call void @free(ptr noundef nonnull %701) #92, !noalias !6717
  br label %809

808:                                              ; preds = %759, %756
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %37) #89
          to label %810 unwind label %562, !noalias !6659

809:                                              ; preds = %__rustc::__rust_dealloc (.exit138), %.loopexit169
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %37)
          to label %820 unwind label %816, !noalias !6659

810:                                              ; preds = %818, %816, %808
  %811 = phi i8 [ 1, %808 ], [ 1, %816 ], [ %1430, %818 ]
  %812 = phi { ptr, i32 } [ %757, %808 ], [ %817, %816 ], [ %819, %818 ]
  %813 = icmp eq i64 %694, 0
  br i1 %813, label %850, label %814

814:                                              ; preds = %810
  %815 = mul nuw i64 %694, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %695) ]
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %695, i64 noundef %815, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !6659
  br label %850

816:                                              ; preds = %809
  %817 = landingpad { ptr, i32 }
          cleanup
  br label %810

818:                                              ; preds = %1461
  %819 = landingpad { ptr, i32 }
          cleanup
  br label %810

820:                                              ; preds = %809
  call void @llvm.lifetime.end.p0(ptr nonnull %37), !noalias !6653
  %821 = icmp eq i64 %694, 0
  br i1 %821, label %857, label %822

822:                                              ; preds = %820
  %823 = mul nuw i64 %694, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %695) ]
  %824 = load i64, ptr %649, align 8, !noalias !6659, !noundef !1733
  %825 = call i64 @llvm.umin.i64(i64 %823, i64 9223372036854775807)
  %826 = call i64 @llvm.ssub.sat.i64(i64 %824, i64 %825)
  store i64 %826, ptr %649, align 8, !noalias !6659
  %827 = load i64, ptr %650, align 8, !noalias !6659, !noundef !1733
  %828 = icmp slt i64 %826, %827
  br i1 %828, label %829, label %.preheader1628

829:                                              ; preds = %822
  store i64 %826, ptr %650, align 8, !noalias !6659
  br label %.preheader1628

.preheader1628:                                   ; preds = %829, %822
  br label %830

830:                                              ; preds = %.preheader1628, %833
  %831 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !6659
  %832 = icmp slt i64 %831, 0
  br i1 %832, label %833, label %__rustc::__rust_dealloc (.exit139)

833:                                              ; preds = %830
  %834 = add nsw i64 %831, 1
  %835 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %831, i64 %834 acq_rel acquire, align 8, !noalias !6659
  %836 = extractvalue { i64, i1 } %835, 1
  br i1 %836, label %837, label %830

837:                                              ; preds = %833
  %838 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %825 monotonic, align 8, !noalias !6659
  %839 = call i64 @llvm.ssub.sat.i64(i64 %838, i64 %825)
  %840 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !6659
  br label %841

841:                                              ; preds = %844, %837
  %842 = phi i64 [ %840, %837 ], [ %847, %844 ]
  %843 = icmp slt i64 %839, %842
  br i1 %843, label %844, label %848

844:                                              ; preds = %841
  %845 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %842, i64 %839 monotonic monotonic, align 8, !noalias !6659
  %846 = extractvalue { i64, i1 } %845, 1
  %847 = extractvalue { i64, i1 } %845, 0
  br i1 %846, label %848, label %841

848:                                              ; preds = %844, %841
  %849 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !6659
  br label %__rustc::__rust_dealloc (.exit139)

__rustc::__rust_dealloc (.exit139): ; preds = %830, %848
  call void @free(ptr noundef nonnull %695) #92, !noalias !6659
  br label %857

850:                                              ; preds = %814, %810
  call void @llvm.experimental.noalias.scope.decl(metadata !6720)
  %851 = load ptr, ptr %642, align 8, !alias.scope !6720, !noalias !6653, !noundef !1733
  %852 = icmp eq ptr %851, null
  br i1 %852, label %1565, label %853

853:                                              ; preds = %850
  %854 = atomicrmw sub ptr %851, i64 1 release, align 8, !noalias !6723
  %855 = icmp eq i64 %854, 1
  br i1 %855, label %856, label %1565

856:                                              ; preds = %853
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %642) #91, !noalias !6659
  br label %1565

857:                                              ; preds = %__rustc::__rust_dealloc (.exit139), %820
  call void @llvm.experimental.noalias.scope.decl(metadata !6728)
  %858 = load ptr, ptr %642, align 8, !alias.scope !6728, !noalias !6653, !noundef !1733
  %859 = icmp eq ptr %858, null
  br i1 %859, label %864, label %860

860:                                              ; preds = %857
  %861 = atomicrmw sub ptr %858, i64 1 release, align 8, !noalias !6731
  %862 = icmp eq i64 %861, 1
  br i1 %862, label %863, label %864

863:                                              ; preds = %860
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %642) #91, !noalias !6659
  br label %864

864:                                              ; preds = %863, %860, %857
  call void @llvm.experimental.noalias.scope.decl(metadata !6736)
  %865 = load ptr, ptr %643, align 8, !alias.scope !6736, !noalias !6653, !noundef !1733
  %866 = icmp eq ptr %865, null
  br i1 %866, label %871, label %867

867:                                              ; preds = %864
  %868 = atomicrmw sub ptr %865, i64 1 release, align 8, !noalias !6739
  %869 = icmp eq i64 %868, 1
  br i1 %869, label %870, label %871

870:                                              ; preds = %867
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %643) #91, !noalias !6659
  br label %871

871:                                              ; preds = %870, %867, %864
  call void @llvm.lifetime.end.p0(ptr nonnull %38), !noalias !6653
  %872 = icmp eq ptr %689, %587
  br i1 %872, label %.loopexit170, label %686

873:                                              ; preds = %909, %903, %896, %761
  call void @llvm.assume(i1 %710)
  %874 = icmp ugt i64 %764, %696
  br i1 %874, label %875, label %915, !prof !1735

875:                                              ; preds = %873
  call void @llvm.lifetime.start.p0(ptr nonnull %25), !noalias !6653
  store i64 %764, ptr %25, align 8, !noalias !6653
  call void @llvm.lifetime.start.p0(ptr nonnull %24), !noalias !6653
  store i64 %696, ptr %24, align 8, !noalias !6653
  call void @llvm.lifetime.start.p0(ptr nonnull %23), !noalias !6653
  store ptr %25, ptr %23, align 8, !noalias !6653
  %876 = getelementptr inbounds nuw i8, ptr %23, i64 8
  store ptr @<usize as core::fmt::Debug>::fmt, ptr %876, align 8, !noalias !6653
  %877 = getelementptr inbounds nuw i8, ptr %23, i64 16
  store ptr %24, ptr %877, align 8, !noalias !6653
  %878 = getelementptr inbounds nuw i8, ptr %23, i64 24
  store ptr @<usize as core::fmt::Debug>::fmt, ptr %878, align 8, !noalias !6653
; invoke core::panicking::panic_fmt
  invoke void @core::panicking::panic_fmt(ptr noundef nonnull @anon.68dd637f94a7f528fe69f6876e3d956b.2156, ptr noundef nonnull %23, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.68dd637f94a7f528fe69f6876e3d956b.288) #88
          to label %879 unwind label %754, !noalias !6659

879:                                              ; preds = %875
  unreachable

880:                                              ; preds = %761
  call void @llvm.lifetime.start.p0(ptr nonnull %22), !noalias !6744
  %881 = load atomic i64, ptr %605 monotonic, align 8, !noalias !6751
  br label %882

882:                                              ; preds = %882, %880
  %883 = phi i64 [ %881, %880 ], [ %887, %882 ]
  %884 = call i64 @llvm.uadd.sat.i64(i64 %883, i64 %769)
  %885 = cmpxchg weak ptr %605, i64 %883, i64 %884 monotonic monotonic, align 8, !noalias !6751
  %886 = extractvalue { i64, i1 } %885, 1
  %887 = extractvalue { i64, i1 } %885, 0
  br i1 %886, label %888, label %882

888:                                              ; preds = %882
  %889 = call i64 @llvm.uadd.sat.i64(i64 %887, i64 %769)
  %890 = load i64, ptr %566, align 8, !noalias !6751
  %891 = icmp ugt i64 %889, %890
  br i1 %891, label %892, label %896

892:                                              ; preds = %888
  call void @llvm.lifetime.start.p0(ptr nonnull %21), !noalias !6754
  store i8 0, ptr %606, align 1, !noalias !6754
  store i64 %890, ptr %607, align 8, !noalias !6754
  store i64 %889, ptr %608, align 8, !noalias !6754
  store i8 0, ptr %21, align 8, !noalias !6754
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %22, ptr noundef nonnull align 8 %566, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %21)
          to label %893 unwind label %750, !noalias !6659

893:                                              ; preds = %892
  call void @llvm.lifetime.end.p0(ptr nonnull %21), !noalias !6754
  %894 = load i8, ptr %22, align 8, !noalias !6744
  %895 = icmp eq i8 %894, -1
  br i1 %895, label %896, label %899

896:                                              ; preds = %893, %888
  call void @llvm.lifetime.end.p0(ptr nonnull %22), !noalias !6744
  %897 = load ptr, ptr %609, align 16, !alias.scope !6648, !noalias !6659, !noundef !1733
  %898 = icmp eq ptr %897, null
  br i1 %898, label %873, label %903

899:                                              ; preds = %893
  call void @llvm.lifetime.end.p0(ptr nonnull %22), !noalias !6744
  %900 = load i64, ptr %48, align 8, !noalias !6657
  %901 = load ptr, ptr %454, align 8, !noalias !6657
  %902 = load i64, ptr %455, align 8, !noalias !6657
  br label %1424

903:                                              ; preds = %896
  %904 = load i32, ptr %610, align 4, !alias.scope !6648, !noalias !6659, !noundef !1733
  %905 = getelementptr i8, ptr %897, i64 56
  %906 = load i64, ptr %905, align 8, !noalias !6659, !noundef !1733
  %907 = zext i32 %904 to i64
  %908 = icmp ugt i64 %906, %907
  br i1 %908, label %909, label %873

909:                                              ; preds = %903
  %910 = getelementptr i8, ptr %897, i64 48
  %911 = load ptr, ptr %910, align 8, !noalias !6659, !nonnull !1733, !noundef !1733
  %912 = getelementptr inbounds nuw [136 x i8], ptr %911, i64 %907
  %913 = getelementptr inbounds nuw [8 x i8], ptr %912, i64 %611
  %914 = atomicrmw add ptr %913, i64 %769 monotonic, align 8, !noalias !6659
  br label %873

915:                                              ; preds = %873
  %916 = icmp ult i64 %771, %764
  %917 = call i64 @llvm.umin.i64(i64 %771, i64 range(i64 0, 384307168202282326) %696)
  %918 = select i1 %916, i64 %764, i64 %917
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %695) ]
  %919 = icmp samesign ult i64 %918, %764
  br i1 %919, label %920, label %921, !prof !6111

920:                                              ; preds = %915
; invoke core::slice::index::slice_index_fail
  invoke void @core::slice::index::slice_index_fail(i64 noundef %764, i64 noundef %918, i64 noundef %696, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.68dd637f94a7f528fe69f6876e3d956b.289) #93
          to label %1460 unwind label %754, !noalias !6659

921:                                              ; preds = %915
  %922 = mul nuw nsw i64 %764, 24
  %923 = getelementptr inbounds nuw i8, ptr %695, i64 %922
  %924 = mul nuw nsw i64 %918, 24
  %925 = getelementptr inbounds nuw i8, ptr %695, i64 %924
  %926 = icmp eq i64 %764, %918
  br i1 %926, label %.loopexit168, label %927

927:                                              ; preds = %921
  %928 = sub nuw nsw i64 %924, %922
  %929 = udiv exact i64 %928, 24
  br label %930

930:                                              ; preds = %945, %927
  %931 = phi i64 [ 0, %927 ], [ %946, %945 ]
  %932 = phi i64 [ 0, %927 ], [ %947, %945 ]
  %933 = phi i64 [ 0, %927 ], [ %948, %945 ]
  %934 = phi i64 [ 0, %927 ], [ %949, %945 ]
  %935 = getelementptr inbounds nuw [24 x i8], ptr %923, i64 %934
  %936 = load i8, ptr %935, align 8, !range !6371, !noalias !6755, !noundef !1733
  %937 = getelementptr i8, ptr %935, i64 8
  %938 = load i64, ptr %937, align 8, !noalias !6755
  switch i8 %936, label %.unreachabledefault [
    i8 0, label %939
    i8 1, label %941
    i8 2, label %945
    i8 3, label %943
  ]

.unreachabledefault:                              ; preds = %930
  unreachable

default.unreachable915:                           ; preds = %.preheader155
  unreachable

939:                                              ; preds = %930
  %940 = call i64 @llvm.uadd.sat.i64(i64 %933, i64 %938)
  br label %945

941:                                              ; preds = %930
  %942 = call i64 @llvm.uadd.sat.i64(i64 %932, i64 %938)
  br label %945

943:                                              ; preds = %930
  %944 = call i64 @llvm.umax.i64(i64 %931, i64 %938)
  br label %945

945:                                              ; preds = %943, %941, %939, %930
  %946 = phi i64 [ %931, %939 ], [ %931, %941 ], [ %944, %943 ], [ %931, %930 ]
  %947 = phi i64 [ %932, %939 ], [ %942, %941 ], [ %932, %943 ], [ %932, %930 ]
  %948 = phi i64 [ %940, %939 ], [ %933, %941 ], [ %933, %943 ], [ %933, %930 ]
  %949 = add nuw i64 %934, 1
  %950 = icmp eq i64 %949, %929
  br i1 %950, label %.loopexit168, label %930

.loopexit168:                                     ; preds = %945, %921
  %951 = phi i64 [ 0, %921 ], [ %948, %945 ]
  %952 = phi i64 [ 0, %921 ], [ %947, %945 ]
  %953 = phi i64 [ 0, %921 ], [ %946, %945 ]
  br i1 %571, label %957, label %.loopexit166

.loopexit166:                                     ; preds = %969, %957, %.loopexit168
  %954 = phi i64 [ 0, %.loopexit168 ], [ 0, %957 ], [ %971, %969 ]
  %955 = load atomic i32, ptr %612 acquire, align 8, !noalias !6759
  %956 = icmp eq i32 %955, 0
  br i1 %956, label %973, label %976

957:                                              ; preds = %.loopexit168
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %766) ]
  %958 = ptrtoint ptr %766 to i64
  %959 = call i64 @llvm.usub.sat.i64(i64 %773, i64 %767)
  %960 = sub nuw i64 %711, %958
  %961 = udiv exact i64 %960, 88
  %962 = call i64 @llvm.umin.i64(i64 %959, i64 %961)
  %963 = icmp eq i64 %962, 0
  br i1 %963, label %.loopexit166, label %.preheader165

.preheader165:                                    ; preds = %957, %969
  %964 = phi i64 [ %971, %969 ], [ 0, %957 ]
  %965 = phi i64 [ %970, %969 ], [ 0, %957 ]
  %966 = getelementptr inbounds nuw [88 x i8], ptr %766, i64 %965
  %967 = getelementptr inbounds nuw i8, ptr %966, i64 8
; invoke purrdf_sparql_eval::scratch::value_bytes
  %968 = invoke noundef i64 @purrdf_sparql_eval::scratch::value_bytes(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(80) %967)
          to label %969 unwind label %748, !noalias !6659

969:                                              ; preds = %.preheader165
  %970 = add nuw nsw i64 %965, 1
  %971 = call noundef range(i64 32, 0) i64 @llvm.uadd.sat.i64(i64 %964, i64 %968)
  %972 = icmp eq i64 %970, %962
  br i1 %972, label %.loopexit166, label %.preheader165

973:                                              ; preds = %.loopexit166
  %974 = load i8, ptr %613, align 8, !noalias !6659
  %975 = icmp eq i8 %974, -1
  br i1 %975, label %976, label %983

976:                                              ; preds = %973, %.loopexit166
  br i1 %568, label %977, label %978

977:                                              ; preds = %978, %976
  br i1 %571, label %986, label %984

978:                                              ; preds = %976
  %979 = load atomic i64, ptr %605 monotonic, align 8, !noalias !6762
  %980 = load i64, ptr %566, align 8, !noalias !6762
  %981 = call i64 @llvm.uadd.sat.i64(i64 %979, i64 %951)
  %982 = icmp ugt i64 %981, %980
  br i1 %982, label %983, label %977

983:                                              ; preds = %986, %978, %973
  br i1 %926, label %.loopexit157, label %.preheader155

984:                                              ; preds = %986, %977
  %985 = or i1 %568, %926
  br i1 %985, label %.loopexit164, label %.preheader163

986:                                              ; preds = %977
  %987 = load i64, ptr %614, align 16, !alias.scope !6648, !noalias !6659, !noundef !1733
  %988 = load atomic i64, ptr %615 monotonic, align 8, !alias.scope !6648, !noalias !6659
  %989 = call noundef i64 @llvm.usub.sat.i64(i64 %987, i64 %988)
  %990 = call i64 @llvm.uadd.sat.i64(i64 %989, i64 %954)
  %991 = call i64 @llvm.uadd.sat.i64(i64 %990, i64 %952)
  %992 = call i64 @llvm.uadd.sat.i64(i64 %991, i64 %953)
  %993 = load atomic i64, ptr %616 monotonic, align 8, !noalias !6765
  %994 = load i64, ptr %569, align 8, !noalias !6765
  %995 = call i64 @llvm.uadd.sat.i64(i64 %993, i64 %992)
  %996 = icmp ugt i64 %995, %994
  br i1 %996, label %983, label %984

.preheader155:                                    ; preds = %983, %1179
  %997 = phi ptr [ %1180, %1179 ], [ %763, %983 ]
  %998 = phi ptr [ %1002, %1179 ], [ %923, %983 ]
  %999 = phi ptr [ %1183, %1179 ], [ %766, %983 ]
  %1000 = phi i64 [ %1182, %1179 ], [ %767, %983 ]
  %1001 = phi ptr [ %1181, %1179 ], [ %763, %983 ]
  %1002 = getelementptr inbounds nuw i8, ptr %998, i64 24
  %1003 = load i8, ptr %998, align 8, !range !6371, !noalias !6659, !noundef !1733
  switch i8 %1003, label %default.unreachable915 [
    i8 0, label %1009
    i8 1, label %1016
    i8 2, label %1021
    i8 3, label %1044
  ]

.loopexit157:                                     ; preds = %1179, %983
  %1004 = phi ptr [ %763, %983 ], [ %1180, %1179 ]
  %1005 = phi i64 [ %767, %983 ], [ %1182, %1179 ]
  %1006 = phi ptr [ %766, %983 ], [ %1183, %1179 ]
  %1007 = icmp ult i64 %1005, %773
  %1008 = select i1 %571, i1 %1007, i1 false
  br i1 %1008, label %1202, label %1195

1009:                                             ; preds = %.preheader155
  %1010 = getelementptr inbounds nuw i8, ptr %998, i64 1
  %1011 = load i8, ptr %1010, align 1, !range !1734, !noalias !6659, !noundef !1733
  %1012 = getelementptr inbounds nuw i8, ptr %998, i64 8
  %1013 = load i64, ptr %1012, align 8, !noalias !6659, !noundef !1733
  %1014 = getelementptr inbounds nuw i8, ptr %998, i64 16
  %1015 = load i64, ptr %1014, align 8, !noalias !6659, !noundef !1733
  br i1 %568, label %1179, label %1045

1016:                                             ; preds = %.preheader155
  %1017 = getelementptr inbounds nuw i8, ptr %998, i64 8
  %1018 = load i64, ptr %1017, align 8, !noalias !6659, !noundef !1733
  %1019 = getelementptr inbounds nuw i8, ptr %998, i64 16
  %1020 = load i64, ptr %1019, align 8, !noalias !6659, !noundef !1733
  br i1 %571, label %1104, label %1179

1021:                                             ; preds = %.preheader155
  %1022 = getelementptr inbounds nuw i8, ptr %998, i64 8
  %1023 = load i64, ptr %1022, align 8, !noalias !6659, !noundef !1733
  %1024 = icmp ult i64 %1000, %1023
  br i1 %1024, label %1025, label %1156

1025:                                             ; preds = %1021
  %1026 = icmp eq ptr %999, %700
  br i1 %1026, label %.loopexit150, label %1027

1027:                                             ; preds = %1025
  %1028 = add i64 %1023, -1
  br label %1032

1029:                                             ; preds = %1042
  %1030 = add i64 %1034, 1
  %1031 = icmp eq ptr %1035, %700
  br i1 %1031, label %.loopexit150, label %1032

1032:                                             ; preds = %1029, %1027
  %1033 = phi ptr [ %1035, %1029 ], [ %999, %1027 ]
  %1034 = phi i64 [ %1030, %1029 ], [ %1000, %1027 ]
  %1035 = getelementptr inbounds nuw i8, ptr %1033, i64 88
  %1036 = getelementptr inbounds nuw i8, ptr %1033, i64 8
  %1037 = load i64, ptr %1036, align 8, !noalias !6768
  %1038 = icmp eq i64 %1037, -1
  br i1 %1038, label %.loopexit150, label %1039

1039:                                             ; preds = %1032
  %1040 = getelementptr inbounds nuw i8, ptr %1033, i64 16
  %1041 = load i64, ptr %1033, align 8, !noalias !6768
  call void @llvm.lifetime.start.p0(ptr nonnull %20), !noalias !6771
  store i64 %1037, ptr %20, align 8, !noalias !6771
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %620, ptr noundef nonnull align 8 dereferenceable(72) %1040, i64 72, i1 false), !noalias !6659
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %621, i64 noundef %1041, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %20)
          to label %1042 unwind label %734, !noalias !6659

1042:                                             ; preds = %1039
  call void @llvm.lifetime.end.p0(ptr nonnull %20), !noalias !6771
  %1043 = icmp eq i64 %1034, %1028
  br i1 %1043, label %.loopexit150, label %1029

1044:                                             ; preds = %.preheader155
  br i1 %571, label %1165, label %1179

1045:                                             ; preds = %1009
  call void @llvm.lifetime.start.p0(ptr nonnull %19), !noalias !6774
  %1046 = load atomic i64, ptr %605 monotonic, align 8, !noalias !6781
  br label %1047

1047:                                             ; preds = %1047, %1045
  %1048 = phi i64 [ %1046, %1045 ], [ %1052, %1047 ]
  %1049 = call i64 @llvm.uadd.sat.i64(i64 %1048, i64 %1013)
  %1050 = cmpxchg weak ptr %605, i64 %1048, i64 %1049 monotonic monotonic, align 8, !noalias !6781
  %1051 = extractvalue { i64, i1 } %1050, 1
  %1052 = extractvalue { i64, i1 } %1050, 0
  br i1 %1051, label %1053, label %1047

1053:                                             ; preds = %1047
  %1054 = call i64 @llvm.uadd.sat.i64(i64 %1052, i64 %1013)
  %1055 = load i64, ptr %566, align 8, !noalias !6781
  %1056 = icmp ugt i64 %1054, %1055
  br i1 %1056, label %1057, label %1066

1057:                                             ; preds = %1053
  call void @llvm.lifetime.start.p0(ptr nonnull %18), !noalias !6784
  store i8 0, ptr %626, align 1, !noalias !6784
  store i64 %1055, ptr %627, align 8, !noalias !6784
  store i64 %1054, ptr %628, align 8, !noalias !6784
  store i8 0, ptr %18, align 8, !noalias !6784
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %19, ptr noundef nonnull align 8 %566, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %18)
          to label %1058 unwind label %730, !noalias !6659

1058:                                             ; preds = %1057
  call void @llvm.lifetime.end.p0(ptr nonnull %18), !noalias !6784
  %1059 = load i8, ptr %19, align 8, !noalias !6774
  %1060 = icmp eq i8 %1059, -1
  br i1 %1060, label %1066, label %1061

1061:                                             ; preds = %1058
  call void @llvm.lifetime.end.p0(ptr nonnull %19), !noalias !6774
  %1062 = icmp ne i64 %1015, 0
  %1063 = add i64 %1015, -1
  %1064 = icmp ult i64 %1000, %1063
  %1065 = select i1 %1062, i1 %1064, i1 false
  br i1 %1065, label %1068, label %.loopexit156

1066:                                             ; preds = %1058, %1053
  call void @llvm.lifetime.end.p0(ptr nonnull %19), !noalias !6774
  %1067 = icmp eq i8 %1011, -1
  br i1 %1067, label %1179, label %1087

1068:                                             ; preds = %1061
  %1069 = icmp eq ptr %999, %700
  br i1 %1069, label %.loopexit148, label %1070

1070:                                             ; preds = %1068
  %1071 = add i64 %1015, -2
  br label %1072

1072:                                             ; preds = %1082, %1070
  %1073 = phi ptr [ %1075, %1082 ], [ %999, %1070 ]
  %1074 = phi i64 [ %1084, %1082 ], [ %1000, %1070 ]
  %1075 = getelementptr inbounds nuw i8, ptr %1073, i64 88
  %1076 = getelementptr inbounds nuw i8, ptr %1073, i64 8
  %1077 = load i64, ptr %1076, align 8, !noalias !6785
  %1078 = icmp eq i64 %1077, -1
  br i1 %1078, label %.loopexit148, label %1079

1079:                                             ; preds = %1072
  %1080 = getelementptr inbounds nuw i8, ptr %1073, i64 16
  %1081 = load i64, ptr %1073, align 8, !noalias !6785
  call void @llvm.lifetime.start.p0(ptr nonnull %17), !noalias !6788
  store i64 %1077, ptr %17, align 8, !noalias !6788
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %629, ptr noundef nonnull align 8 dereferenceable(72) %1080, i64 72, i1 false), !noalias !6659
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %621, i64 noundef %1081, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %17)
          to label %1082 unwind label %728, !noalias !6659

1082:                                             ; preds = %1079
  call void @llvm.lifetime.end.p0(ptr nonnull %17), !noalias !6788
  %1083 = icmp eq i64 %1074, %1071
  %1084 = add nuw i64 %1074, 1
  %1085 = icmp eq ptr %1075, %700
  %1086 = select i1 %1083, i1 true, i1 %1085
  br i1 %1086, label %.loopexit148, label %1072

1087:                                             ; preds = %1066
  %1088 = load ptr, ptr %609, align 16, !alias.scope !6648, !noalias !6659, !noundef !1733
  %1089 = icmp eq ptr %1088, null
  br i1 %1089, label %1179, label %1090

1090:                                             ; preds = %1087
  %1091 = load i32, ptr %610, align 4, !alias.scope !6648, !noalias !6659, !noundef !1733
  %1092 = getelementptr i8, ptr %1088, i64 56
  %1093 = load i64, ptr %1092, align 8, !noalias !6659, !noundef !1733
  %1094 = zext i32 %1091 to i64
  %1095 = icmp ugt i64 %1093, %1094
  br i1 %1095, label %1096, label %1179

1096:                                             ; preds = %1090
  %1097 = getelementptr i8, ptr %1088, i64 48
  %1098 = load ptr, ptr %1097, align 8, !noalias !6659, !nonnull !1733, !noundef !1733
  %1099 = zext nneg i8 %1011 to i64
  %1100 = getelementptr inbounds nuw [136 x i8], ptr %1098, i64 %1094
  %1101 = getelementptr inbounds nuw [8 x i8], ptr %1100, i64 %1099
  %1102 = atomicrmw add ptr %1101, i64 %1013 monotonic, align 8, !noalias !6659
  br label %1179

1103:                                             ; preds = %1108
  br i1 %1110, label %.loopexit156, label %1179

1104:                                             ; preds = %1016
  call void @llvm.lifetime.start.p0(ptr nonnull %33), !noalias !6653
  %1105 = load i64, ptr %569, align 8, !noalias !6659
  %1106 = icmp eq i64 %1105, -1
  br i1 %1106, label %1107, label %1113

1107:                                             ; preds = %1124, %1104
  call void @llvm.lifetime.end.p0(ptr nonnull %33), !noalias !6653
  br label %1179

1108:                                             ; preds = %1130, %1128
  %1109 = load i8, ptr %33, align 8, !noalias !6653
  %1110 = icmp ne i8 %1109, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %33), !noalias !6653
  %1111 = icmp ne i64 %1020, 0
  %1112 = and i1 %1111, %1110
  br i1 %1112, label %1131, label %1103

1113:                                             ; preds = %1104
  %1114 = load atomic i32, ptr %612 acquire, align 8, !noalias !6791
  %1115 = icmp eq i32 %1114, 0
  br i1 %1115, label %1128, label %1116

1116:                                             ; preds = %1113
  %1117 = load atomic i64, ptr %616 monotonic, align 8, !noalias !6791
  br label %1118

1118:                                             ; preds = %1118, %1116
  %1119 = phi i64 [ %1117, %1116 ], [ %1123, %1118 ]
  %1120 = call i64 @llvm.uadd.sat.i64(i64 %1119, i64 %1018)
  %1121 = cmpxchg weak ptr %616, i64 %1119, i64 %1120 monotonic monotonic, align 8, !noalias !6791
  %1122 = extractvalue { i64, i1 } %1121, 1
  %1123 = extractvalue { i64, i1 } %1121, 0
  br i1 %1122, label %1124, label %1118

1124:                                             ; preds = %1118
  %1125 = call i64 @llvm.uadd.sat.i64(i64 %1123, i64 %1018)
  %1126 = load i64, ptr %569, align 8, !noalias !6791
  %1127 = icmp ugt i64 %1125, %1126
  br i1 %1127, label %1129, label %1107

1128:                                             ; preds = %1113
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %33, ptr noundef nonnull align 8 dereferenceable(24) %613, i64 24, i1 false), !noalias !6659
  br label %1108

1129:                                             ; preds = %1124
  call void @llvm.lifetime.start.p0(ptr nonnull %9), !noalias !6794
  store i8 3, ptr %622, align 1, !noalias !6794
  store i64 %1126, ptr %623, align 8, !noalias !6794
  store i64 %1125, ptr %624, align 8, !noalias !6794
  store i8 0, ptr %9, align 8, !noalias !6794
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %33, ptr noundef nonnull align 8 %566, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %9)
          to label %1130 unwind label %746, !noalias !6659

1130:                                             ; preds = %1129
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !6794
  br label %1108

1131:                                             ; preds = %1108
  %1132 = add i64 %1020, -1
  %1133 = icmp ult i64 %1000, %1132
  br i1 %1133, label %1134, label %.loopexit156

1134:                                             ; preds = %1131
  %1135 = icmp eq ptr %999, %700
  br i1 %1135, label %.loopexit148, label %1136

1136:                                             ; preds = %1134
  %1137 = add i64 %1020, -2
  br label %1138

1138:                                             ; preds = %1148, %1136
  %1139 = phi ptr [ %1141, %1148 ], [ %999, %1136 ]
  %1140 = phi i64 [ %1150, %1148 ], [ %1000, %1136 ]
  %1141 = getelementptr inbounds nuw i8, ptr %1139, i64 88
  %1142 = getelementptr inbounds nuw i8, ptr %1139, i64 8
  %1143 = load i64, ptr %1142, align 8, !noalias !6795
  %1144 = icmp eq i64 %1143, -1
  br i1 %1144, label %.loopexit148, label %1145

1145:                                             ; preds = %1138
  %1146 = getelementptr inbounds nuw i8, ptr %1139, i64 16
  %1147 = load i64, ptr %1139, align 8, !noalias !6795
  call void @llvm.lifetime.start.p0(ptr nonnull %16), !noalias !6798
  store i64 %1143, ptr %16, align 8, !noalias !6798
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %625, ptr noundef nonnull align 8 dereferenceable(72) %1146, i64 72, i1 false), !noalias !6659
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %621, i64 noundef %1147, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %16)
          to label %1148 unwind label %732, !noalias !6659

1148:                                             ; preds = %1145
  call void @llvm.lifetime.end.p0(ptr nonnull %16), !noalias !6798
  %1149 = icmp eq i64 %1140, %1137
  %1150 = add nuw i64 %1140, 1
  %1151 = icmp eq ptr %1141, %700
  %1152 = select i1 %1149, i1 true, i1 %1151
  br i1 %1152, label %.loopexit148, label %1138

.loopexit150:                                     ; preds = %1042, %1032, %1029, %1025
  %1153 = phi ptr [ %1001, %1025 ], [ %1035, %1029 ], [ %1035, %1032 ], [ %1035, %1042 ]
  %1154 = phi i64 [ %1000, %1025 ], [ %1023, %1042 ], [ %1034, %1032 ], [ %1030, %1029 ]
  %1155 = phi ptr [ %999, %1025 ], [ %1035, %1029 ], [ %1035, %1032 ], [ %1035, %1042 ]
  store ptr %1153, ptr %599, align 8, !noalias !6653
  br label %1156

1156:                                             ; preds = %.loopexit150, %1021
  %1157 = phi ptr [ %997, %1021 ], [ %1153, %.loopexit150 ]
  %1158 = phi ptr [ %1001, %1021 ], [ %1153, %.loopexit150 ]
  %1159 = phi i64 [ %1000, %1021 ], [ %1154, %.loopexit150 ]
  %1160 = phi ptr [ %999, %1021 ], [ %1155, %.loopexit150 ]
  br i1 %571, label %1161, label %1179

1161:                                             ; preds = %1156
  call void @llvm.lifetime.start.p0(ptr nonnull %32), !noalias !6653
; invoke <purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
  invoke fastcc void @<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %32, ptr noundef nonnull align 16 dereferenceable(1232) %2)
          to label %1162 unwind label %746, !noalias !6659

1162:                                             ; preds = %1161
  %1163 = load i8, ptr %32, align 8, !range !1736, !noalias !6653, !noundef !1733
  %1164 = icmp eq i8 %1163, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %32), !noalias !6653
  br i1 %1164, label %1179, label %.loopexit156

1165:                                             ; preds = %1044
  %1166 = getelementptr inbounds nuw i8, ptr %998, i64 8
  %1167 = load i64, ptr %1166, align 8, !noalias !6659, !noundef !1733
  call void @llvm.lifetime.start.p0(ptr nonnull %31), !noalias !6653
  %1168 = load atomic i32, ptr %612 acquire, align 8, !noalias !6801
  %1169 = icmp eq i32 %1168, 0
  br i1 %1169, label %1170, label %1171

1170:                                             ; preds = %1165
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %31, ptr noundef nonnull align 8 dereferenceable(24) %613, i64 24, i1 false), !noalias !6659
  br label %1185

1171:                                             ; preds = %1165
  %1172 = load atomic i64, ptr %616 monotonic, align 8, !noalias !6801
  %1173 = call i64 @llvm.uadd.sat.i64(i64 %1172, i64 %1167)
  %1174 = load i64, ptr %569, align 8, !noalias !6801
  %1175 = icmp ugt i64 %1173, %1174
  br i1 %1175, label %1177, label %1176

1176:                                             ; preds = %1171
  call void @llvm.lifetime.end.p0(ptr nonnull %31), !noalias !6653
  br label %1179

1177:                                             ; preds = %1171
  call void @llvm.lifetime.start.p0(ptr nonnull %15), !noalias !6804
  store i8 3, ptr %617, align 1, !noalias !6804
  store i64 %1174, ptr %618, align 8, !noalias !6804
  store i64 %1173, ptr %619, align 8, !noalias !6804
  store i8 0, ptr %15, align 8, !noalias !6804
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)(ptr noalias nofree noundef align 8 captures(none) dereferenceable(24) %31, ptr noundef nonnull align 8 %566, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %15)
          to label %1178 unwind label %746, !noalias !6659

1178:                                             ; preds = %1177
  call void @llvm.lifetime.end.p0(ptr nonnull %15), !noalias !6804
  br label %1185

1179:                                             ; preds = %1185, %1176, %1162, %1156, %1107, %1103, %1096, %1090, %1087, %1066, %1044, %1016, %1009
  %1180 = phi ptr [ %997, %1103 ], [ %1157, %1156 ], [ %997, %1044 ], [ %997, %1009 ], [ %997, %1016 ], [ %997, %1176 ], [ %1157, %1162 ], [ %997, %1185 ], [ %997, %1066 ], [ %997, %1087 ], [ %997, %1096 ], [ %997, %1090 ], [ %997, %1107 ]
  %1181 = phi ptr [ %1001, %1103 ], [ %1158, %1156 ], [ %1001, %1044 ], [ %1001, %1009 ], [ %1001, %1016 ], [ %1001, %1176 ], [ %1158, %1162 ], [ %1001, %1185 ], [ %1001, %1066 ], [ %1001, %1087 ], [ %1001, %1096 ], [ %1001, %1090 ], [ %1001, %1107 ]
  %1182 = phi i64 [ %1000, %1103 ], [ %1159, %1156 ], [ %1000, %1044 ], [ %1000, %1009 ], [ %1000, %1016 ], [ %1000, %1176 ], [ %1159, %1162 ], [ %1000, %1185 ], [ %1000, %1066 ], [ %1000, %1087 ], [ %1000, %1096 ], [ %1000, %1090 ], [ %1000, %1107 ]
  %1183 = phi ptr [ %999, %1103 ], [ %1160, %1156 ], [ %999, %1044 ], [ %999, %1009 ], [ %999, %1016 ], [ %999, %1176 ], [ %1160, %1162 ], [ %999, %1185 ], [ %999, %1066 ], [ %999, %1087 ], [ %999, %1096 ], [ %999, %1090 ], [ %999, %1107 ]
  %1184 = icmp eq ptr %1002, %925
  br i1 %1184, label %.loopexit157, label %.preheader155

1185:                                             ; preds = %1178, %1170
  %1186 = load i8, ptr %31, align 8, !noalias !6653
  %1187 = icmp eq i8 %1186, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %31), !noalias !6653
  br i1 %1187, label %1179, label %.loopexit156

.loopexit148:                                     ; preds = %1148, %1138, %1082, %1072, %1134, %1068
  %1188 = phi ptr [ %1001, %1134 ], [ %1001, %1068 ], [ %1075, %1082 ], [ %1075, %1072 ], [ %1141, %1138 ], [ %1141, %1148 ]
  store ptr %1188, ptr %599, align 8, !noalias !6653
  br label %.loopexit156

.loopexit156:                                     ; preds = %1185, %1162, %1103, %.loopexit148, %1131, %1061
  %1189 = load i64, ptr %48, align 8, !noalias !6657
  %1190 = load ptr, ptr %454, align 8, !noalias !6657
  %1191 = load i64, ptr %455, align 8, !noalias !6657
  br label %1424

.loopexit153:                                     ; preds = %1219, %1209, %1206, %1202
  %1192 = phi ptr [ %1004, %1202 ], [ %1212, %1206 ], [ %1212, %1209 ], [ %1212, %1219 ]
  %1193 = phi i64 [ %1005, %1202 ], [ %773, %1219 ], [ %1211, %1209 ], [ %1207, %1206 ]
  %1194 = phi ptr [ %1006, %1202 ], [ %1212, %1206 ], [ %1212, %1209 ], [ %1212, %1219 ]
  store ptr %1192, ptr %599, align 8, !noalias !6653
  br label %1195

1195:                                             ; preds = %1313, %1288, %.loopexit153, %.loopexit157
  %1196 = phi ptr [ %1283, %1288 ], [ %1314, %1313 ], [ %1004, %.loopexit157 ], [ %1192, %.loopexit153 ]
  %1197 = phi i64 [ %1284, %1288 ], [ %1315, %1313 ], [ %1005, %.loopexit157 ], [ %1193, %.loopexit153 ]
  %1198 = phi ptr [ %1285, %1288 ], [ %1316, %1313 ], [ %1006, %.loopexit157 ], [ %1194, %.loopexit153 ]
  %1199 = icmp eq i64 %775, 0
  br i1 %1199, label %.loopexit151, label %1200

1200:                                             ; preds = %1195
  %1201 = load ptr, ptr %462, align 8, !alias.scope !6805, !noalias !6808, !nonnull !1733, !noundef !1733
  br label %1376

1202:                                             ; preds = %.loopexit157
  %1203 = icmp eq ptr %1006, %700
  br i1 %1203, label %.loopexit153, label %1204

1204:                                             ; preds = %1202
  %1205 = add i64 %773, -1
  br label %1209

1206:                                             ; preds = %1219
  %1207 = add i64 %1211, 1
  %1208 = icmp eq ptr %1212, %700
  br i1 %1208, label %.loopexit153, label %1209

1209:                                             ; preds = %1206, %1204
  %1210 = phi ptr [ %1212, %1206 ], [ %1006, %1204 ]
  %1211 = phi i64 [ %1207, %1206 ], [ %1005, %1204 ]
  %1212 = getelementptr inbounds nuw i8, ptr %1210, i64 88
  %1213 = getelementptr inbounds nuw i8, ptr %1210, i64 8
  %1214 = load i64, ptr %1213, align 8, !noalias !6810
  %1215 = icmp eq i64 %1214, -1
  br i1 %1215, label %.loopexit153, label %1216

1216:                                             ; preds = %1209
  %1217 = getelementptr inbounds nuw i8, ptr %1210, i64 16
  %1218 = load i64, ptr %1210, align 8, !noalias !6810
  call void @llvm.lifetime.start.p0(ptr nonnull %14), !noalias !6813
  store i64 %1214, ptr %14, align 8, !noalias !6813
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %630, ptr noundef nonnull align 8 dereferenceable(72) %1217, i64 72, i1 false), !noalias !6659
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %621, i64 noundef %1218, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %14)
          to label %1219 unwind label %744, !noalias !6659

1219:                                             ; preds = %1216
  call void @llvm.lifetime.end.p0(ptr nonnull %14), !noalias !6813
  %1220 = icmp eq i64 %1211, %1205
  br i1 %1220, label %.loopexit153, label %1206

.loopexit164:                                     ; preds = %1236, %984
  %1221 = icmp eq i64 %764, %918
  br i1 %1221, label %.loopexit162, label %.lr.ph

1222:                                             ; preds = %.lr.ph
  %1223 = icmp eq ptr %923, %1225
  br i1 %1223, label %.loopexit162, label %.lr.ph

.lr.ph:                                           ; preds = %.loopexit164, %1222
  %1224 = phi ptr [ %1225, %1222 ], [ %925, %.loopexit164 ]
  %1225 = getelementptr inbounds i8, ptr %1224, i64 -24
  %1226 = load i8, ptr %1225, align 8, !range !6371, !noalias !6816, !noundef !1733
  %1227 = icmp eq i8 %1226, 2
  br i1 %1227, label %1256, label %1222

.preheader163:                                    ; preds = %984, %1236
  %1228 = phi ptr [ %1229, %1236 ], [ %923, %984 ]
  %1229 = getelementptr inbounds nuw i8, ptr %1228, i64 24
  %1230 = load i8, ptr %1228, align 8, !range !6371, !noalias !6659, !noundef !1733
  %1231 = icmp eq i8 %1230, 0
  br i1 %1231, label %1232, label %1236

1232:                                             ; preds = %.preheader163
  %1233 = getelementptr inbounds nuw i8, ptr %1228, i64 1
  %1234 = load i8, ptr %1233, align 1, !range !1734, !noalias !6659, !noundef !1733
  %1235 = icmp eq i8 %1234, -1
  br i1 %1235, label %1236, label %1238

1236:                                             ; preds = %1247, %1241, %1238, %1232, %.preheader163
  %1237 = icmp eq ptr %1229, %925
  br i1 %1237, label %.loopexit164, label %.preheader163

1238:                                             ; preds = %1232
  %1239 = load ptr, ptr %609, align 16, !alias.scope !6648, !noalias !6659, !noundef !1733
  %1240 = icmp eq ptr %1239, null
  br i1 %1240, label %1236, label %1241

1241:                                             ; preds = %1238
  %1242 = load i32, ptr %610, align 4, !alias.scope !6648, !noalias !6659, !noundef !1733
  %1243 = getelementptr i8, ptr %1239, i64 56
  %1244 = load i64, ptr %1243, align 8, !noalias !6659, !noundef !1733
  %1245 = zext i32 %1242 to i64
  %1246 = icmp ugt i64 %1244, %1245
  br i1 %1246, label %1247, label %1236

1247:                                             ; preds = %1241
  %1248 = getelementptr i8, ptr %1239, i64 48
  %1249 = load ptr, ptr %1248, align 8, !noalias !6659, !nonnull !1733, !noundef !1733
  %1250 = getelementptr inbounds nuw i8, ptr %1228, i64 8
  %1251 = load i64, ptr %1250, align 8, !noalias !6659, !noundef !1733
  %1252 = zext nneg i8 %1234 to i64
  %1253 = getelementptr inbounds nuw [136 x i8], ptr %1249, i64 %1245
  %1254 = getelementptr inbounds nuw [8 x i8], ptr %1253, i64 %1252
  %1255 = atomicrmw add ptr %1254, i64 %1251 monotonic, align 8, !noalias !6659
  br label %1236

1256:                                             ; preds = %.lr.ph
  %1257 = getelementptr i8, ptr %1224, i64 -16
  %1258 = load i64, ptr %1257, align 8, !noalias !6816
  %1259 = icmp ult i64 %767, %1258
  br i1 %1259, label %1260, label %.loopexit162

1260:                                             ; preds = %1256
  %1261 = icmp eq ptr %766, %700
  br i1 %1261, label %.loopexit160, label %1262

1262:                                             ; preds = %1260
  %1263 = add i64 %1258, -1
  br label %1267

1264:                                             ; preds = %1277
  %1265 = add i64 %1269, 1
  %1266 = icmp eq ptr %1270, %700
  br i1 %1266, label %.loopexit160, label %1267

1267:                                             ; preds = %1264, %1262
  %1268 = phi ptr [ %1270, %1264 ], [ %766, %1262 ]
  %1269 = phi i64 [ %1265, %1264 ], [ %767, %1262 ]
  %1270 = getelementptr inbounds nuw i8, ptr %1268, i64 88
  %1271 = getelementptr inbounds nuw i8, ptr %1268, i64 8
  %1272 = load i64, ptr %1271, align 8, !noalias !6819
  %1273 = icmp eq i64 %1272, -1
  br i1 %1273, label %.loopexit160, label %1274

1274:                                             ; preds = %1267
  %1275 = getelementptr inbounds nuw i8, ptr %1268, i64 16
  %1276 = load i64, ptr %1268, align 8, !noalias !6819
  call void @llvm.lifetime.start.p0(ptr nonnull %13), !noalias !6822
  store i64 %1272, ptr %13, align 8, !noalias !6822
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %631, ptr noundef nonnull align 8 dereferenceable(72) %1275, i64 72, i1 false), !noalias !6659
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %621, i64 noundef %1276, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %13)
          to label %1277 unwind label %742, !noalias !6659

1277:                                             ; preds = %1274
  call void @llvm.lifetime.end.p0(ptr nonnull %13), !noalias !6822
  %1278 = icmp eq i64 %1269, %1263
  br i1 %1278, label %.loopexit160, label %1264

.loopexit160:                                     ; preds = %1277, %1267, %1264, %1260
  %1279 = phi ptr [ %763, %1260 ], [ %1270, %1264 ], [ %1270, %1267 ], [ %1270, %1277 ]
  %1280 = phi i64 [ %767, %1260 ], [ %1258, %1277 ], [ %1269, %1267 ], [ %1265, %1264 ]
  %1281 = phi ptr [ %766, %1260 ], [ %1270, %1264 ], [ %1270, %1267 ], [ %1270, %1277 ]
  store ptr %1279, ptr %599, align 8, !noalias !6653
  br label %.loopexit162

.loopexit162:                                     ; preds = %1222, %.loopexit164, %.loopexit160, %1256
  %1282 = phi i1 [ false, %.loopexit160 ], [ false, %1256 ], [ true, %.loopexit164 ], [ true, %1222 ]
  %1283 = phi ptr [ %1279, %.loopexit160 ], [ %763, %1256 ], [ %763, %.loopexit164 ], [ %763, %1222 ]
  %1284 = phi i64 [ %1280, %.loopexit160 ], [ %767, %1256 ], [ %767, %.loopexit164 ], [ %767, %1222 ]
  %1285 = phi ptr [ %1281, %.loopexit160 ], [ %766, %1256 ], [ %766, %.loopexit164 ], [ %766, %1222 ]
  %1286 = icmp eq i64 %951, 0
  %1287 = or i1 %568, %1286
  br i1 %1287, label %1288, label %1289

1288:                                             ; preds = %1305, %.loopexit162
  br i1 %571, label %1307, label %1195

1289:                                             ; preds = %.loopexit162
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !6825
  %1290 = load atomic i64, ptr %605 monotonic, align 8, !noalias !6832
  br label %1291

1291:                                             ; preds = %1291, %1289
  %1292 = phi i64 [ %1290, %1289 ], [ %1296, %1291 ]
  %1293 = call i64 @llvm.uadd.sat.i64(i64 %1292, i64 %951)
  %1294 = cmpxchg weak ptr %605, i64 %1292, i64 %1293 monotonic monotonic, align 8, !noalias !6832
  %1295 = extractvalue { i64, i1 } %1294, 1
  %1296 = extractvalue { i64, i1 } %1294, 0
  br i1 %1295, label %1297, label %1291

1297:                                             ; preds = %1291
  %1298 = call i64 @llvm.uadd.sat.i64(i64 %1296, i64 %951)
  %1299 = load i64, ptr %566, align 8, !noalias !6832
  %1300 = icmp ugt i64 %1298, %1299
  br i1 %1300, label %1301, label %1305

1301:                                             ; preds = %1297
  call void @llvm.lifetime.start.p0(ptr nonnull %11), !noalias !6835
  store i8 0, ptr %632, align 1, !noalias !6835
  store i64 %1299, ptr %633, align 8, !noalias !6835
  store i64 %1298, ptr %634, align 8, !noalias !6835
  store i8 0, ptr %11, align 8, !noalias !6835
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %12, ptr noundef nonnull align 8 %566, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %11)
          to label %1302 unwind label %740, !noalias !6659

1302:                                             ; preds = %1301
  call void @llvm.lifetime.end.p0(ptr nonnull %11), !noalias !6835
  %1303 = load i8, ptr %12, align 8, !noalias !6825
  %1304 = icmp eq i8 %1303, -1
  br i1 %1304, label %1305, label %1306

1305:                                             ; preds = %1302, %1297
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !6825
  br label %1288

1306:                                             ; preds = %1302
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !6825
  br i1 %571, label %1350, label %1372

1307:                                             ; preds = %1288
  call void @llvm.lifetime.start.p0(ptr nonnull %36), !noalias !6653
  %1308 = load i64, ptr %569, align 8, !noalias !6659
  %1309 = icmp eq i64 %1308, -1
  br i1 %1309, label %1335, label %1317

.loopexit158:                                     ; preds = %1370, %1360, %1357, %1353
  %1310 = phi ptr [ %1283, %1353 ], [ %1363, %1357 ], [ %1363, %1360 ], [ %1363, %1370 ]
  %1311 = phi i64 [ %1284, %1353 ], [ %773, %1370 ], [ %1362, %1360 ], [ %1358, %1357 ]
  %1312 = phi ptr [ %1285, %1353 ], [ %1363, %1357 ], [ %1363, %1360 ], [ %1363, %1370 ]
  store ptr %1310, ptr %599, align 8, !noalias !6653
  br label %1313

1313:                                             ; preds = %1350, %.loopexit158
  %1314 = phi ptr [ %1283, %1350 ], [ %1310, %.loopexit158 ]
  %1315 = phi i64 [ %1284, %1350 ], [ %1311, %.loopexit158 ]
  %1316 = phi ptr [ %1285, %1350 ], [ %1312, %.loopexit158 ]
  br i1 %1351, label %1372, label %1195

1317:                                             ; preds = %1307
  %1318 = load atomic i32, ptr %612 acquire, align 8, !noalias !6836
  %1319 = icmp eq i32 %1318, 0
  br i1 %1319, label %1332, label %1320

1320:                                             ; preds = %1317
  %1321 = load atomic i64, ptr %616 monotonic, align 8, !noalias !6836
  br label %1322

1322:                                             ; preds = %1322, %1320
  %1323 = phi i64 [ %1321, %1320 ], [ %1327, %1322 ]
  %1324 = call i64 @llvm.uadd.sat.i64(i64 %1323, i64 %952)
  %1325 = cmpxchg weak ptr %616, i64 %1323, i64 %1324 monotonic monotonic, align 8, !noalias !6836
  %1326 = extractvalue { i64, i1 } %1325, 1
  %1327 = extractvalue { i64, i1 } %1325, 0
  br i1 %1326, label %1328, label %1322

1328:                                             ; preds = %1322
  %1329 = call i64 @llvm.uadd.sat.i64(i64 %1327, i64 %952)
  %1330 = load i64, ptr %569, align 8, !noalias !6836
  %1331 = icmp ugt i64 %1329, %1330
  br i1 %1331, label %1333, label %1335

1332:                                             ; preds = %1317
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %36, ptr noundef nonnull align 8 dereferenceable(24) %613, i64 24, i1 false), !noalias !6659
  br label %1336

1333:                                             ; preds = %1328
  call void @llvm.lifetime.start.p0(ptr nonnull %6), !noalias !6836
  store i8 3, ptr %644, align 1, !noalias !6836
  store i64 %1330, ptr %645, align 8, !noalias !6836
  store i64 %1329, ptr %646, align 8, !noalias !6836
  store i8 0, ptr %6, align 8, !noalias !6836
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %36, ptr noundef nonnull align 8 %566, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %6)
          to label %1334 unwind label %752

1334:                                             ; preds = %1333
  call void @llvm.lifetime.end.p0(ptr nonnull %6), !noalias !6836
  br label %1336

1335:                                             ; preds = %1336, %1328, %1307
  call void @llvm.lifetime.end.p0(ptr nonnull %36), !noalias !6653
  br i1 %1282, label %1340, label %1342

1336:                                             ; preds = %1334, %1332
  %1337 = load i8, ptr %36, align 8, !noalias !6653
  %1338 = icmp eq i8 %1337, -1
  br i1 %1338, label %1335, label %1339

1339:                                             ; preds = %1336
  call void @llvm.lifetime.end.p0(ptr nonnull %36), !noalias !6653
  br label %1350

1340:                                             ; preds = %1343, %1335
  %1341 = icmp eq i64 %953, 0
  br i1 %1341, label %1350, label %1346

1342:                                             ; preds = %1335
  call void @llvm.lifetime.start.p0(ptr nonnull %35), !noalias !6653
; invoke <purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
  invoke fastcc void @<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %35, ptr noundef nonnull align 16 dereferenceable(1232) %2)
          to label %1343 unwind label %752, !noalias !6659

1343:                                             ; preds = %1342
  %1344 = load i8, ptr %35, align 8, !range !1736, !noalias !6653, !noundef !1733
  %1345 = icmp eq i8 %1344, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %35), !noalias !6653
  br i1 %1345, label %1340, label %1350

1346:                                             ; preds = %1340
  call void @llvm.lifetime.start.p0(ptr nonnull %34), !noalias !6653
; invoke <purrdf_sparql_eval::governor::GovernorState>::admit_transient
  invoke void @<purrdf_sparql_eval::governor::GovernorState>::admit_transient(ptr noalias nofree noundef nonnull sret([24 x i8]) align 8 captures(none) dereferenceable(24) %34, ptr noundef nonnull align 8 %566, i8 noundef 3, i64 noundef %953)
          to label %1347 unwind label %752, !noalias !6659

1347:                                             ; preds = %1346
  %1348 = load i8, ptr %34, align 8, !range !1736, !noalias !6653, !noundef !1733
  %1349 = icmp ne i8 %1348, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %34), !noalias !6653
  br label %1350

1350:                                             ; preds = %1347, %1343, %1340, %1339, %1306
  %1351 = phi i1 [ false, %1340 ], [ %1349, %1347 ], [ true, %1306 ], [ true, %1339 ], [ true, %1343 ]
  %1352 = icmp ult i64 %1284, %773
  br i1 %1352, label %1353, label %1313

1353:                                             ; preds = %1350
  %1354 = icmp eq ptr %1285, %700
  br i1 %1354, label %.loopexit158, label %1355

1355:                                             ; preds = %1353
  %1356 = add i64 %773, -1
  br label %1360

1357:                                             ; preds = %1370
  %1358 = add i64 %1362, 1
  %1359 = icmp eq ptr %1363, %700
  br i1 %1359, label %.loopexit158, label %1360

1360:                                             ; preds = %1357, %1355
  %1361 = phi ptr [ %1363, %1357 ], [ %1285, %1355 ]
  %1362 = phi i64 [ %1358, %1357 ], [ %1284, %1355 ]
  %1363 = getelementptr inbounds nuw i8, ptr %1361, i64 88
  %1364 = getelementptr inbounds nuw i8, ptr %1361, i64 8
  %1365 = load i64, ptr %1364, align 8, !noalias !6839
  %1366 = icmp eq i64 %1365, -1
  br i1 %1366, label %.loopexit158, label %1367

1367:                                             ; preds = %1360
  %1368 = getelementptr inbounds nuw i8, ptr %1361, i64 16
  %1369 = load i64, ptr %1361, align 8, !noalias !6839
  call void @llvm.lifetime.start.p0(ptr nonnull %10), !noalias !6842
  store i64 %1365, ptr %10, align 8, !noalias !6842
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %635, ptr noundef nonnull align 8 dereferenceable(72) %1368, i64 72, i1 false), !noalias !6659
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %621, i64 noundef %1369, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %10)
          to label %1370 unwind label %738, !noalias !6659

1370:                                             ; preds = %1367
  call void @llvm.lifetime.end.p0(ptr nonnull %10), !noalias !6842
  %1371 = icmp eq i64 %1362, %1356
  br i1 %1371, label %.loopexit158, label %1357

1372:                                             ; preds = %1313, %1306
  %1373 = load i64, ptr %48, align 8, !noalias !6657
  %1374 = load ptr, ptr %454, align 8, !noalias !6657
  %1375 = load i64, ptr %455, align 8, !noalias !6657
  br label %1424

1376:                                             ; preds = %1415, %1200
  %1377 = phi i64 [ %775, %1200 ], [ %1379, %1415 ]
  %1378 = phi ptr [ %762, %1200 ], [ %1384, %1415 ]
  %1379 = add i64 %1377, -1
  %1380 = icmp eq ptr %1378, %1201
  br i1 %1380, label %.loopexit151, label %1383

.loopexit151:                                     ; preds = %1415, %1376, %1195
  %1381 = phi ptr [ %762, %1195 ], [ %1378, %1376 ], [ %1384, %1415 ]
  store ptr %1381, ptr %461, align 8, !noalias !6653
  %1382 = icmp eq ptr %768, %706
  br i1 %1382, label %.loopexit169, label %761

1383:                                             ; preds = %1376
  %1384 = getelementptr inbounds nuw i8, ptr %1378, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %29), !noalias !6653
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %636, ptr noundef nonnull align 8 dereferenceable(40) %1378, i64 40, i1 false), !noalias !6659
  call void @llvm.lifetime.start.p0(ptr nonnull %30), !noalias !6653
  store ptr %2, ptr %29, align 8, !noalias !6653
  call void @llvm.experimental.noalias.scope.decl(metadata !6845)
  call void @llvm.experimental.noalias.scope.decl(metadata !6848)
  %1385 = load i64, ptr %636, align 8, !alias.scope !6848, !noalias !6850, !noundef !1733
  %1386 = icmp eq i64 %1385, 0
  br i1 %1386, label %1387, label %1389

1387:                                             ; preds = %1383
  %1388 = load ptr, ptr %638, align 16, !alias.scope !6852, !noalias !6853, !nonnull !1733, !align !1829, !noundef !1733
; invoke purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(96) %30, ptr noalias nofree noundef align 8 dereferenceable(184) %621, ptr noundef nonnull align 8 %1388, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %639)
          to label %1390 unwind label %736, !noalias !6659

1389:                                             ; preds = %1383
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %637, ptr noundef nonnull align 8 dereferenceable(40) %1378, i64 40, i1 false), !noalias !6659
  call void @llvm.lifetime.end.p0(ptr nonnull %29), !noalias !6653
  br label %1400

1390:                                             ; preds = %1387
  %1391 = load i64, ptr %30, align 16, !noalias !6653
  call void @llvm.lifetime.end.p0(ptr nonnull %29), !noalias !6653
  %1392 = icmp eq i64 %1391, -1
  br i1 %1392, label %1400, label %1393

1393:                                             ; preds = %1390
  store ptr %1384, ptr %461, align 8, !noalias !6653
  %1394 = load i64, ptr %637, align 8, !noalias !6653
  %1395 = load ptr, ptr %640, align 16, !noalias !6653
  %1396 = load i64, ptr %641, align 8, !noalias !6653
  %1397 = load i8, ptr %647, align 16, !noalias !6653
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(15) %28, ptr noundef nonnull align 1 dereferenceable(15) %648, i64 15, i1 false), !noalias !6653
  %1398 = getelementptr inbounds nuw i8, ptr %30, i64 48
  %1399 = getelementptr inbounds nuw i8, ptr %60, i64 15
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(48) %1399, ptr noundef nonnull align 16 dereferenceable(48) %1398, i64 48, i1 false), !noalias !6657
  call void @llvm.lifetime.end.p0(ptr nonnull %30), !noalias !6653
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(15) %60, ptr noundef nonnull align 1 dereferenceable(15) %28, i64 15, i1 false), !noalias !6657
  br label %1424

1400:                                             ; preds = %1390, %1389
  %1401 = load i64, ptr %637, align 8, !noalias !6653
  %1402 = load ptr, ptr %640, align 16, !noalias !6653
  %1403 = load i64, ptr %641, align 8, !noalias !6653
  %1404 = load i8, ptr %647, align 16, !noalias !6653
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(15) %28, ptr noundef nonnull align 1 dereferenceable(15) %648, i64 15, i1 false), !noalias !6653
  call void @llvm.lifetime.end.p0(ptr nonnull %30), !noalias !6653
  call void @llvm.experimental.noalias.scope.decl(metadata !6854)
  %1405 = load i64, ptr %455, align 8, !alias.scope !6854, !noalias !6857, !noundef !1733
  %1406 = load i64, ptr %48, align 8, !range !1828, !alias.scope !6854, !noalias !6857, !noundef !1733
  %1407 = icmp eq i64 %1405, %1406
  br i1 %1407, label %1408, label %1415

1408:                                             ; preds = %1400
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %48)
          to label %1415 unwind label %1409, !noalias !6859

1409:                                             ; preds = %1408
  %1410 = landingpad { ptr, i32 }
          cleanup
  store ptr %1384, ptr %461, align 8, !noalias !6653
  %1411 = icmp ugt i64 %1401, 5
  br i1 %1411, label %1412, label %756

1412:                                             ; preds = %1409
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1402) ]
  %1413 = shl i64 %1401, 3
  %1414 = add i64 %1413, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1402, i64 noundef %1414, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !6860
  br label %756

1415:                                             ; preds = %1408, %1400
  %1416 = load ptr, ptr %454, align 8, !alias.scope !6854, !noalias !6857, !nonnull !1733, !noundef !1733
  %1417 = getelementptr inbounds nuw [40 x i8], ptr %1416, i64 %1405
  store i64 %1401, ptr %1417, align 8, !noalias !6863
  %1418 = getelementptr inbounds nuw i8, ptr %1417, i64 8
  store ptr %1402, ptr %1418, align 8, !noalias !6863
  %1419 = getelementptr inbounds nuw i8, ptr %1417, i64 16
  store i64 %1403, ptr %1419, align 8, !noalias !6659
  %1420 = getelementptr inbounds nuw i8, ptr %1417, i64 24
  store i8 %1404, ptr %1420, align 8, !noalias !6659
  %1421 = getelementptr inbounds nuw i8, ptr %1417, i64 25
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(15) %1421, ptr noundef nonnull align 1 dereferenceable(15) %28, i64 15, i1 false), !noalias !6659
  %1422 = add i64 %1405, 1
  store i64 %1422, ptr %455, align 8, !alias.scope !6854, !noalias !6857
  %1423 = icmp eq i64 %1379, 0
  br i1 %1423, label %.loopexit151, label %1376

1424:                                             ; preds = %1393, %1372, %.loopexit156, %899
  %1425 = phi i8 [ %1397, %1393 ], [ 1, %.loopexit156 ], [ 1, %1372 ], [ 0, %899 ]
  %1426 = phi i64 [ %1396, %1393 ], [ %1191, %.loopexit156 ], [ %1375, %1372 ], [ %902, %899 ]
  %1427 = phi ptr [ %1395, %1393 ], [ %1190, %.loopexit156 ], [ %1374, %1372 ], [ %901, %899 ]
  %1428 = phi i64 [ %1394, %1393 ], [ %1189, %.loopexit156 ], [ %1373, %1372 ], [ %900, %899 ]
  %1429 = phi i64 [ %1391, %1393 ], [ -1, %.loopexit156 ], [ -1, %1372 ], [ -1, %899 ]
  %1430 = phi i8 [ 1, %1393 ], [ 0, %.loopexit156 ], [ 0, %1372 ], [ 0, %899 ]
  %1431 = icmp eq i64 %702, 0
  br i1 %1431, label %1461, label %1432

1432:                                             ; preds = %1424
  %1433 = shl nuw i64 %702, 5
  %1434 = load i64, ptr %649, align 8, !noalias !6864, !noundef !1733
  %1435 = call i64 @llvm.umin.i64(i64 %1433, i64 9223372036854775807)
  %1436 = call i64 @llvm.ssub.sat.i64(i64 %1434, i64 %1435)
  store i64 %1436, ptr %649, align 8, !noalias !6864
  %1437 = load i64, ptr %650, align 8, !noalias !6864, !noundef !1733
  %1438 = icmp slt i64 %1436, %1437
  br i1 %1438, label %1439, label %.preheader1618

1439:                                             ; preds = %1432
  store i64 %1436, ptr %650, align 8, !noalias !6864
  br label %.preheader1618

.preheader1618:                                   ; preds = %1439, %1432
  br label %1440

1440:                                             ; preds = %.preheader1618, %1443
  %1441 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !6864
  %1442 = icmp slt i64 %1441, 0
  br i1 %1442, label %1443, label %__rustc::__rust_dealloc (.exit140)

1443:                                             ; preds = %1440
  %1444 = add nsw i64 %1441, 1
  %1445 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1441, i64 %1444 acq_rel acquire, align 8, !noalias !6864
  %1446 = extractvalue { i64, i1 } %1445, 1
  br i1 %1446, label %1447, label %1440

1447:                                             ; preds = %1443
  %1448 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1435 monotonic, align 8, !noalias !6864
  %1449 = call i64 @llvm.ssub.sat.i64(i64 %1448, i64 %1435)
  %1450 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !6864
  br label %1451

1451:                                             ; preds = %1454, %1447
  %1452 = phi i64 [ %1450, %1447 ], [ %1457, %1454 ]
  %1453 = icmp slt i64 %1449, %1452
  br i1 %1453, label %1454, label %1458

1454:                                             ; preds = %1451
  %1455 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1452, i64 %1449 monotonic monotonic, align 8, !noalias !6864
  %1456 = extractvalue { i64, i1 } %1455, 1
  %1457 = extractvalue { i64, i1 } %1455, 0
  br i1 %1456, label %1458, label %1451

1458:                                             ; preds = %1454, %1451
  %1459 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !6864
  br label %__rustc::__rust_dealloc (.exit140)

__rustc::__rust_dealloc (.exit140): ; preds = %1440, %1458
  call void @free(ptr noundef nonnull %701) #92, !noalias !6864
  br label %1461

1460:                                             ; preds = %920
  unreachable

1461:                                             ; preds = %__rustc::__rust_dealloc (.exit140), %1424
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %37)
          to label %1462 unwind label %818, !noalias !6659

1462:                                             ; preds = %1461
  call void @llvm.lifetime.end.p0(ptr nonnull %37), !noalias !6653
  %1463 = icmp eq i64 %694, 0
  br i1 %1463, label %1492, label %1464

1464:                                             ; preds = %1462
  %1465 = mul nuw i64 %694, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %695) ]
  %1466 = load i64, ptr %649, align 8, !noalias !6659, !noundef !1733
  %1467 = call i64 @llvm.umin.i64(i64 %1465, i64 9223372036854775807)
  %1468 = call i64 @llvm.ssub.sat.i64(i64 %1466, i64 %1467)
  store i64 %1468, ptr %649, align 8, !noalias !6659
  %1469 = load i64, ptr %650, align 8, !noalias !6659, !noundef !1733
  %1470 = icmp slt i64 %1468, %1469
  br i1 %1470, label %1471, label %.preheader1617

1471:                                             ; preds = %1464
  store i64 %1468, ptr %650, align 8, !noalias !6659
  br label %.preheader1617

.preheader1617:                                   ; preds = %1471, %1464
  br label %1472

1472:                                             ; preds = %.preheader1617, %1475
  %1473 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !6659
  %1474 = icmp slt i64 %1473, 0
  br i1 %1474, label %1475, label %__rustc::__rust_dealloc (.exit141)

1475:                                             ; preds = %1472
  %1476 = add nsw i64 %1473, 1
  %1477 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1473, i64 %1476 acq_rel acquire, align 8, !noalias !6659
  %1478 = extractvalue { i64, i1 } %1477, 1
  br i1 %1478, label %1479, label %1472

1479:                                             ; preds = %1475
  %1480 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1467 monotonic, align 8, !noalias !6659
  %1481 = call i64 @llvm.ssub.sat.i64(i64 %1480, i64 %1467)
  %1482 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !6659
  br label %1483

1483:                                             ; preds = %1486, %1479
  %1484 = phi i64 [ %1482, %1479 ], [ %1489, %1486 ]
  %1485 = icmp slt i64 %1481, %1484
  br i1 %1485, label %1486, label %1490

1486:                                             ; preds = %1483
  %1487 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1484, i64 %1481 monotonic monotonic, align 8, !noalias !6659
  %1488 = extractvalue { i64, i1 } %1487, 1
  %1489 = extractvalue { i64, i1 } %1487, 0
  br i1 %1488, label %1490, label %1483

1490:                                             ; preds = %1486, %1483
  %1491 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !6659
  br label %__rustc::__rust_dealloc (.exit141)

__rustc::__rust_dealloc (.exit141): ; preds = %1472, %1490
  call void @free(ptr noundef nonnull %695) #92, !noalias !6659
  br label %1492

1492:                                             ; preds = %__rustc::__rust_dealloc (.exit141), %1462
  call void @llvm.experimental.noalias.scope.decl(metadata !6867)
  %1493 = load ptr, ptr %642, align 8, !alias.scope !6867, !noalias !6653, !noundef !1733
  %1494 = icmp eq ptr %1493, null
  br i1 %1494, label %1499, label %1495

1495:                                             ; preds = %1492
  %1496 = atomicrmw sub ptr %1493, i64 1 release, align 8, !noalias !6870
  %1497 = icmp eq i64 %1496, 1
  br i1 %1497, label %1498, label %1499

1498:                                             ; preds = %1495
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %642) #91, !noalias !6659
  br label %1499

1499:                                             ; preds = %1498, %1495, %1492
  call void @llvm.experimental.noalias.scope.decl(metadata !6875)
  %1500 = load ptr, ptr %643, align 8, !alias.scope !6875, !noalias !6653, !noundef !1733
  %1501 = icmp eq ptr %1500, null
  br i1 %1501, label %1506, label %1502

1502:                                             ; preds = %1499
  %1503 = atomicrmw sub ptr %1500, i64 1 release, align 8, !noalias !6878
  %1504 = icmp eq i64 %1503, 1
  br i1 %1504, label %1505, label %1506

1505:                                             ; preds = %1502
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %643) #91, !noalias !6659
  br label %1506

1506:                                             ; preds = %1505, %1502, %1499
  call void @llvm.lifetime.end.p0(ptr nonnull %38), !noalias !6653
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %39)
          to label %1507 unwind label %667, !noalias !6659

1507:                                             ; preds = %1506
  call void @llvm.lifetime.end.p0(ptr nonnull %39), !noalias !6653
  %1508 = atomicrmw sub ptr %565, i64 1 release, align 8, !noalias !6883
  %1509 = icmp eq i64 %1508, 1
  br i1 %1509, label %1510, label %1511

1510:                                             ; preds = %1507
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %40) #91
          to label %1511 unwind label %500, !noalias !6659

1511:                                             ; preds = %1510, %1507
  call void @llvm.lifetime.end.p0(ptr nonnull %40), !noalias !6653
  br label %554

1512:                                             ; preds = %1514, %554
  call void @llvm.lifetime.end.p0(ptr nonnull %45), !noalias !6653
  %1513 = trunc nuw i8 %560 to i1
  br i1 %1513, label %1515, label %516

1514:                                             ; preds = %554
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %45)
          to label %1512 unwind label %512, !noalias !6659

1515:                                             ; preds = %1512
  call void @llvm.experimental.noalias.scope.decl(metadata !6888)
  %1516 = load ptr, ptr %454, align 8, !alias.scope !6888, !noalias !6653, !nonnull !1733, !noundef !1733
  %1517 = load i64, ptr %455, align 8, !alias.scope !6888, !noalias !6653, !noundef !1733
  call void @llvm.experimental.noalias.scope.decl(metadata !6891)
  %1518 = icmp eq i64 %1517, 0
  br i1 %1518, label %.loopexit146, label %.preheader145

.preheader145:                                    ; preds = %1515
  %1519 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1520 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %1521

1521:                                             ; preds = %.preheader145, %1559
  %1522 = phi i64 [ %1524, %1559 ], [ 0, %.preheader145 ]
  %1523 = getelementptr inbounds nuw [40 x i8], ptr %1516, i64 %1522
  %1524 = add nuw nsw i64 %1522, 1
  %1525 = load i64, ptr %1523, align 8, !range !1771, !alias.scope !6894, !noalias !6897, !noundef !1733
  %1526 = icmp ugt i64 %1525, 5
  br i1 %1526, label %1527, label %1559

1527:                                             ; preds = %1521
  %1528 = getelementptr i8, ptr %1523, i64 8
  %1529 = load ptr, ptr %1528, align 8, !alias.scope !6891, !noalias !6897, !nonnull !1733, !noundef !1733
  %1530 = shl i64 %1525, 3
  %1531 = add i64 %1530, -8
  %1532 = load i64, ptr %1519, align 8, !noalias !6898, !noundef !1733
  %1533 = call i64 @llvm.umin.i64(i64 %1531, i64 9223372036854775807)
  %1534 = call i64 @llvm.ssub.sat.i64(i64 %1532, i64 %1533)
  store i64 %1534, ptr %1519, align 8, !noalias !6898
  %1535 = load i64, ptr %1520, align 8, !noalias !6898, !noundef !1733
  %1536 = icmp slt i64 %1534, %1535
  br i1 %1536, label %1537, label %.preheader1606

1537:                                             ; preds = %1527
  store i64 %1534, ptr %1520, align 8, !noalias !6898
  br label %.preheader1606

.preheader1606:                                   ; preds = %1537, %1527
  br label %1538

1538:                                             ; preds = %.preheader1606, %1541
  %1539 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !6898
  %1540 = icmp slt i64 %1539, 0
  br i1 %1540, label %1541, label %__rustc::__rust_dealloc (.exit142)

1541:                                             ; preds = %1538
  %1542 = add nsw i64 %1539, 1
  %1543 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1539, i64 %1542 acq_rel acquire, align 8, !noalias !6898
  %1544 = extractvalue { i64, i1 } %1543, 1
  br i1 %1544, label %1545, label %1538

1545:                                             ; preds = %1541
  %1546 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1533 monotonic, align 8, !noalias !6898
  %1547 = call i64 @llvm.ssub.sat.i64(i64 %1546, i64 %1533)
  %1548 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !6898
  br label %1549

1549:                                             ; preds = %1552, %1545
  %1550 = phi i64 [ %1548, %1545 ], [ %1555, %1552 ]
  %1551 = icmp slt i64 %1547, %1550
  br i1 %1551, label %1552, label %1556

1552:                                             ; preds = %1549
  %1553 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1550, i64 %1547 monotonic monotonic, align 8, !noalias !6898
  %1554 = extractvalue { i64, i1 } %1553, 1
  %1555 = extractvalue { i64, i1 } %1553, 0
  br i1 %1554, label %1556, label %1549

1556:                                             ; preds = %1552, %1549
  %1557 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !6898
  br label %__rustc::__rust_dealloc (.exit142)

__rustc::__rust_dealloc (.exit142): ; preds = %1538, %1556
  %1558 = icmp ne i64 %1531, 0
  call void @llvm.assume(i1 %1558), !noalias !6898
  call void @free(ptr noundef nonnull %1529) #92, !noalias !6898
  br label %1559

1559:                                             ; preds = %__rustc::__rust_dealloc (.exit142), %1521
  %1560 = icmp eq i64 %1524, %1517
  br i1 %1560, label %.loopexit146, label %1521

.loopexit146:                                     ; preds = %1559, %1515
  %1561 = load i64, ptr %48, align 8, !alias.scope !6888, !noalias !6653
  %1562 = icmp eq i64 %1561, 0
  br i1 %1562, label %516, label %1563

1563:                                             ; preds = %.loopexit146
  %1564 = mul nuw i64 %1561, 40
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1516, i64 noundef %1564, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !6897
  br label %516

1565:                                             ; preds = %856, %853, %850
  call void @llvm.experimental.noalias.scope.decl(metadata !6901)
  %1566 = load ptr, ptr %643, align 8, !alias.scope !6901, !noalias !6653, !noundef !1733
  %1567 = icmp eq ptr %1566, null
  br i1 %1567, label %685, label %1568

1568:                                             ; preds = %1565
  %1569 = atomicrmw sub ptr %1566, i64 1 release, align 8, !noalias !6904
  %1570 = icmp eq i64 %1569, 1
  br i1 %1570, label %1571, label %685

1571:                                             ; preds = %1568
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %643) #91, !noalias !6659
  br label %685

1572:                                             ; preds = %666, %660, %500
  %1573 = phi { ptr, i32 } [ %503, %500 ], [ %663, %660 ], [ %663, %666 ]
  %1574 = phi i8 [ %502, %500 ], [ %662, %660 ], [ %662, %666 ]
  %1575 = phi i8 [ %501, %500 ], [ %661, %660 ], [ %661, %666 ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %45) #89
          to label %507 unwind label %562, !noalias !6659

1576:                                             ; preds = %507, %486
  %1577 = phi { ptr, i32 } [ %510, %507 ], [ %487, %486 ]
  %1578 = phi i8 [ %509, %507 ], [ 1, %486 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %48) #89, !noalias !6659
  br label %1579

1579:                                             ; preds = %1576, %507
  %1580 = phi i8 [ %509, %507 ], [ %1578, %1576 ]
  %1581 = phi { ptr, i32 } [ %510, %507 ], [ %1577, %1576 ]
  %1582 = trunc nuw i8 %1580 to i1
  br i1 %1582, label %1643, label %1726

1583:                                             ; preds = %449
  %1584 = landingpad { ptr, i32 }
          cleanup
  br label %1643

1585:                                             ; preds = %516, %504, %449
  %1586 = phi i8 [ 2, %504 ], [ %555, %516 ], [ %451, %449 ]
  %1587 = phi i64 [ %498, %504 ], [ %556, %516 ], [ %448, %449 ]
  %1588 = phi ptr [ %506, %504 ], [ %557, %516 ], [ %446, %449 ]
  %1589 = phi i64 [ %505, %504 ], [ %558, %516 ], [ %444, %449 ]
  %1590 = phi i64 [ -1, %504 ], [ %559, %516 ], [ %441, %449 ]
  %1591 = icmp eq i64 %383, 0
  br i1 %1591, label %._crit_edge, label %.lr.ph1491

1592:                                             ; preds = %.lr.ph1491
  %1593 = icmp eq i64 %1596, %383
  br i1 %1593, label %._crit_edge, label %.lr.ph1491

.lr.ph1491:                                       ; preds = %1585, %1592
  %1594 = phi i64 [ %1596, %1592 ], [ 0, %1585 ]
  %1595 = getelementptr inbounds nuw [160 x i8], ptr %384, i64 %1594
  %1596 = add nuw nsw i64 %1594, 1
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef readonly align 8 dereferenceable(160) %1595)
          to label %1592 unwind label %1600, !noalias !6909

1597:                                             ; preds = %.lr.ph1493
  %1598 = add i64 %1603, 1
  %1599 = icmp eq i64 %1598, %383
  br i1 %1599, label %._crit_edge1494, label %.lr.ph1493

1600:                                             ; preds = %.lr.ph1491
  %1601 = landingpad { ptr, i32 }
          cleanup
  %1602 = icmp eq i64 %1596, %383
  br i1 %1602, label %._crit_edge1494, label %.lr.ph1493

.lr.ph1493:                                       ; preds = %1600, %1597
  %1603 = phi i64 [ %1598, %1597 ], [ %1596, %1600 ]
  %1604 = getelementptr inbounds nuw [160 x i8], ptr %384, i64 %1603
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef readonly align 8 dereferenceable(160) %1604) #89
          to label %1597 unwind label %1605, !noalias !6909

1605:                                             ; preds = %.lr.ph1493
  %1606 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !6912
  unreachable

._crit_edge1494:                                  ; preds = %1597, %1600
  %1607 = icmp eq i64 %404, 0
  br i1 %1607, label %1726, label %1608

1608:                                             ; preds = %._crit_edge1494
  %1609 = mul nuw i64 %404, 160
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %384, i64 noundef %1609, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !6909
  br label %1726

._crit_edge:                                      ; preds = %1592, %1585
  %1610 = icmp eq i64 %404, 0
  br i1 %1610, label %1645, label %1611

1611:                                             ; preds = %._crit_edge
  %1612 = mul nuw i64 %404, 160
  %1613 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1614 = load i64, ptr %1613, align 8, !noalias !6909, !noundef !1733
  %1615 = call i64 @llvm.umin.i64(i64 %1612, i64 9223372036854775807)
  %1616 = call i64 @llvm.ssub.sat.i64(i64 %1614, i64 %1615)
  store i64 %1616, ptr %1613, align 8, !noalias !6909
  %1617 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1618 = load i64, ptr %1617, align 8, !noalias !6909, !noundef !1733
  %1619 = icmp slt i64 %1616, %1618
  br i1 %1619, label %1620, label %.preheader1603

1620:                                             ; preds = %1611
  store i64 %1616, ptr %1617, align 8, !noalias !6909
  br label %.preheader1603

.preheader1603:                                   ; preds = %1620, %1611
  br label %1621

1621:                                             ; preds = %.preheader1603, %1624
  %1622 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !6909
  %1623 = icmp slt i64 %1622, 0
  br i1 %1623, label %1624, label %__rustc::__rust_dealloc (.exit143)

1624:                                             ; preds = %1621
  %1625 = add nsw i64 %1622, 1
  %1626 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1622, i64 %1625 acq_rel acquire, align 8, !noalias !6909
  %1627 = extractvalue { i64, i1 } %1626, 1
  br i1 %1627, label %1628, label %1621

1628:                                             ; preds = %1624
  %1629 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1615 monotonic, align 8, !noalias !6909
  %1630 = call i64 @llvm.ssub.sat.i64(i64 %1629, i64 %1615)
  %1631 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !6909
  br label %1632

1632:                                             ; preds = %1635, %1628
  %1633 = phi i64 [ %1631, %1628 ], [ %1638, %1635 ]
  %1634 = icmp slt i64 %1630, %1633
  br i1 %1634, label %1635, label %1639

1635:                                             ; preds = %1632
  %1636 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1633, i64 %1630 monotonic monotonic, align 8, !noalias !6909
  %1637 = extractvalue { i64, i1 } %1636, 1
  %1638 = extractvalue { i64, i1 } %1636, 0
  br i1 %1637, label %1639, label %1632

1639:                                             ; preds = %1635, %1632
  %1640 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !6909
  br label %__rustc::__rust_dealloc (.exit143)

__rustc::__rust_dealloc (.exit143): ; preds = %1621, %1639
  call void @free(ptr noundef nonnull %384) #92, !noalias !6909
  br label %1645

1641:                                             ; preds = %.loopexit173
  %1642 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %3) #89
          to label %1643 unwind label %562, !noalias !6658

1643:                                             ; preds = %1641, %1583, %1579
  %1644 = phi { ptr, i32 } [ %1584, %1583 ], [ %1581, %1579 ], [ %1642, %1641 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %59) #89
          to label %1726 unwind label %562, !noalias !6915

1645:                                             ; preds = %__rustc::__rust_dealloc (.exit143), %._crit_edge, %516
  %1646 = phi i8 [ %1586, %._crit_edge ], [ %555, %516 ], [ %1586, %__rustc::__rust_dealloc (.exit143) ]
  %1647 = phi i64 [ %1587, %._crit_edge ], [ %556, %516 ], [ %1587, %__rustc::__rust_dealloc (.exit143) ]
  %1648 = phi ptr [ %1588, %._crit_edge ], [ %557, %516 ], [ %1588, %__rustc::__rust_dealloc (.exit143) ]
  %1649 = phi i64 [ %1589, %._crit_edge ], [ %558, %516 ], [ %1589, %__rustc::__rust_dealloc (.exit143) ]
  %1650 = phi i64 [ %1590, %._crit_edge ], [ %559, %516 ], [ %1590, %__rustc::__rust_dealloc (.exit143) ]
  call void @llvm.lifetime.end.p0(ptr nonnull %28)
  call void @llvm.lifetime.end.p0(ptr nonnull %41)
  call void @llvm.lifetime.end.p0(ptr nonnull %59)
  %1651 = icmp eq i64 %1650, -1
  br i1 %1651, label %1658, label %1652

1652:                                             ; preds = %1645
  %1653 = getelementptr inbounds nuw i8, ptr %0, i64 33
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(63) %1653, ptr noundef nonnull align 1 dereferenceable(63) %60, i64 63, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %60)
  store i64 %1650, ptr %0, align 16
  %1654 = getelementptr inbounds nuw i8, ptr %0, i64 8
  store i64 %1649, ptr %1654, align 8
  %1655 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store ptr %1648, ptr %1655, align 16
  %1656 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store i64 %1647, ptr %1656, align 8
  %1657 = getelementptr inbounds nuw i8, ptr %0, i64 32
  store i8 %1646, ptr %1657, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %62)
  br label %1708

1658:                                             ; preds = %1645, %727
  %1659 = phi i64 [ %717, %727 ], [ %1649, %1645 ]
  %1660 = phi ptr [ %718, %727 ], [ %1648, %1645 ]
  %1661 = phi i64 [ %719, %727 ], [ %1647, %1645 ]
  %1662 = phi i8 [ 2, %727 ], [ %1646, %1645 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %60)
  call void @llvm.lifetime.start.p0(ptr nonnull %58)
  store i64 %1659, ptr %58, align 8
  %1663 = getelementptr inbounds nuw i8, ptr %58, i64 8
  store ptr %1660, ptr %1663, align 8
  %1664 = getelementptr inbounds nuw i8, ptr %58, i64 16
  store i64 %1661, ptr %1664, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %8)
  %1665 = load ptr, ptr %69, align 16, !noundef !1733
  %1666 = icmp eq ptr %1665, null
  br i1 %1666, label %1677, label %1667

1667:                                             ; preds = %1658
  %1668 = getelementptr inbounds nuw i8, ptr %1665, i64 296
  %1669 = load atomic i32, ptr %1668 acquire, align 4, !noalias !6916
  %1670 = icmp eq i32 %1669, 0
  br i1 %1670, label %1671, label %1675

1671:                                             ; preds = %1667
  %1672 = getelementptr inbounds nuw i8, ptr %1665, i64 272
  %1673 = load i8, ptr %1672, align 8
  %1674 = getelementptr inbounds nuw i8, ptr %1665, i64 273
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %8, ptr noundef nonnull align 1 dereferenceable(23) %1674, i64 23, i1 false)
  br label %1675

1675:                                             ; preds = %1671, %1667
  %1676 = phi i8 [ %1673, %1671 ], [ -1, %1667 ]
  switch i8 %1662, label %1681 [
    i8 2, label %1699
    i8 0, label %1680
  ]

1677:                                             ; preds = %1658
  %1678 = icmp eq i8 %1662, 2
  %1679 = and i1 %1678, %429
  br label %1699

1680:                                             ; preds = %1696, %1683, %1675
  br label %1699

1681:                                             ; preds = %1675
  %1682 = icmp eq i8 %1676, -1
  br i1 %1682, label %1699, label %1683

1683:                                             ; preds = %1681
  %1684 = getelementptr inbounds nuw i8, ptr %2, i64 472
  %1685 = load i8, ptr %1684, align 8, !range !3719, !noundef !1733
  %1686 = icmp eq i8 %1685, 2
  br i1 %1686, label %1687, label %1680

1687:                                             ; preds = %1683
  %1688 = getelementptr inbounds nuw i8, ptr %2, i64 688
  call void @llvm.experimental.noalias.scope.decl(metadata !6919)
  %1689 = load ptr, ptr %1688, align 16, !alias.scope !6919, !noalias !6922, !nonnull !1733, !noundef !1733
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !6924
  store i8 %1676, ptr %7, align 8, !noalias !6928
  %1690 = getelementptr inbounds nuw i8, ptr %7, i64 1
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %1690, ptr noundef nonnull align 1 dereferenceable(23) %8, i64 23, i1 false)
  %1691 = getelementptr inbounds nuw i8, ptr %1689, i64 40
  %1692 = load atomic i32, ptr %1691 acquire, align 4, !noalias !6924
  %1693 = icmp eq i32 %1692, 0
  br i1 %1693, label %1696, label %1694, !prof !1946

1694:                                             ; preds = %1687
  %1695 = getelementptr inbounds nuw i8, ptr %1689, i64 16
; invoke <std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::initialize::<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::get_or_init<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::try_insert::{closure#0}>::{closure#0}, !>
  invoke fastcc void @<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::initialize::<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::get_or_init<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::try_insert::{closure#0}>::{closure#0}, !> (.llvm.1794586459888082020)(ptr noundef nonnull align 8 %1695, ptr noundef nonnull align 8 %7)
          to label %1696 unwind label %1697

1696:                                             ; preds = %1694, %1687
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !6924
  br label %1680

1697:                                             ; preds = %1694
  %1698 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %58) #89
  br label %1726

1699:                                             ; preds = %1681, %1680, %1677, %1675
  %1700 = phi i8 [ -1, %1677 ], [ -1, %1681 ], [ %1676, %1680 ], [ %1676, %1675 ]
  %1701 = phi i1 [ %1679, %1677 ], [ false, %1681 ], [ false, %1680 ], [ %429, %1675 ]
  %1702 = icmp eq i8 %1700, -1
  %1703 = select i1 %1701, i1 %1702, i1 false
  %1704 = zext i1 %1703 to i64
  call void @llvm.lifetime.end.p0(ptr nonnull %8)
  %1705 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %1705, ptr noundef nonnull align 8 dereferenceable(24) %58, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %58)
  %1706 = getelementptr inbounds nuw i8, ptr %0, i64 32
  store i64 %1704, ptr %1706, align 16
  %1707 = getelementptr inbounds nuw i8, ptr %0, i64 40
  store i64 %428, ptr %1707, align 8
  store i64 -1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %62)
  br label %1708

1708:                                             ; preds = %1721, %1699, %1652
  ret void

1709:                                             ; preds = %1728, %1724
  %1710 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #90
  unreachable

1711:                                             ; preds = %__rustc::__rust_dealloc (.exit137), %.loopexit
  call void @llvm.lifetime.end.p0(ptr nonnull %57), !noalias !6571
  br label %1712

1712:                                             ; preds = %1711, %87
  %1713 = phi i64 [ %140, %1711 ], [ %79, %87 ]
  %1714 = phi i64 [ %143, %1711 ], [ %82, %87 ]
  %1715 = phi ptr [ %144, %1711 ], [ %84, %87 ]
  %1716 = phi i64 [ %145, %1711 ], [ %86, %87 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %53)
  call void @llvm.lifetime.end.p0(ptr nonnull %63)
  %1717 = getelementptr inbounds nuw i8, ptr %0, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(64) %1717, ptr noundef nonnull align 8 dereferenceable(64) %64, i64 64, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %64)
  store i64 %1713, ptr %0, align 16
  %1718 = getelementptr inbounds nuw i8, ptr %0, i64 8
  store i64 %1714, ptr %1718, align 8
  %1719 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store ptr %1715, ptr %1719, align 16
  %1720 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store i64 %1716, ptr %1720, align 8
  br label %1721

1721:                                             ; preds = %1712, %132
; call core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  tail call fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(208) %4)
  br label %1708

1722:                                             ; preds = %251
  %1723 = landingpad { ptr, i32 }
          cleanup
  br label %1728

1724:                                             ; preds = %249, %128, %89
  %1725 = phi { ptr, i32 } [ %90, %89 ], [ %250, %249 ], [ %129, %128 ]
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(208) %4) #89
          to label %1726 unwind label %1709

1726:                                             ; preds = %1728, %1724, %1697, %1643, %1608, %._crit_edge1494, %1579
  %1727 = phi { ptr, i32 } [ %1698, %1697 ], [ %1729, %1728 ], [ %1581, %1579 ], [ %1644, %1643 ], [ %1601, %1608 ], [ %1725, %1724 ], [ %1601, %._crit_edge1494 ]
  resume { ptr, i32 } %1727

1728:                                             ; preds = %1722, %355
  %1729 = phi { ptr, i32 } [ %1723, %1722 ], [ %356, %355 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(24) %3) #89
          to label %1726 unwind label %1709
}
