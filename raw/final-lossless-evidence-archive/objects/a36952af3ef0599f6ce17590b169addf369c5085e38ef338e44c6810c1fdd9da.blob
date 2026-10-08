define void @purrdf_sparql_eval::modifier::eval_project::<purrdf_core::ir::dataset::RdfDataset>(ptr dead_on_unwind noalias nofree noundef writable writeonly sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef nonnull readonly align 8 captures(address) %3, i64 noundef range(i64 0, 576460752303423488) %4, ptr noalias nofree noundef align 16 dereferenceable(1232) %5) unnamed_addr #8 personality ptr @rust_eh_personality !guid !36499 {
  %7 = alloca [16 x i8], align 8
  %8 = alloca [16 x i8], align 8
  %9 = alloca [24 x i8], align 8
  %10 = alloca [112 x i8], align 16
  %11 = alloca [48 x i8], align 8
  %12 = alloca [32 x i8], align 8
  %13 = alloca [32 x i8], align 8
  %14 = alloca [104 x i8], align 8
  %15 = alloca [96 x i8], align 8
  %16 = alloca [24 x i8], align 8
  %17 = alloca [24 x i8], align 8
  %18 = alloca [24 x i8], align 8
  %19 = alloca [8 x i8], align 8
  %20 = alloca [112 x i8], align 16
  %21 = alloca [104 x i8], align 8
  %22 = alloca [32 x i8], align 8
  %23 = alloca [32 x i8], align 8
  %24 = alloca [32 x i8], align 8
  %25 = alloca [104 x i8], align 8
  %26 = alloca [96 x i8], align 8
; call <purrdf_sparql_eval::governor::lift::Lift>::at
  call void @<purrdf_sparql_eval::governor::lift::Lift>::at(ptr noalias nofree noundef nonnull sret([104 x i8]) align 8 captures(none) dereferenceable(104) %21, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36500)
  %27 = getelementptr inbounds nuw i8, ptr %5, i64 1112
  %28 = getelementptr inbounds nuw i8, ptr %5, i64 1128
  %29 = load i64, ptr %28, align 8, !alias.scope !36500, !noalias !36503, !noundef !1733
  %30 = icmp ult i64 %29, 192153584101141163
  tail call void @llvm.assume(i1 %30)
  %31 = icmp eq i64 %29, 0
  br i1 %31, label %32, label %33

32:                                               ; preds = %6
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %20, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef nonnull align 16 dereferenceable(1232) %5, ptr noundef nonnull readonly align 8 dereferenceable(144) %2)
          to label %271 unwind label %269, !inline_history !36507

33:                                               ; preds = %6
  %34 = getelementptr inbounds nuw i8, ptr %5, i64 1120
  %35 = load ptr, ptr %34, align 16, !alias.scope !36500, !noalias !36503, !nonnull !1733, !noundef !1733
  %36 = mul nuw nsw i64 %29, 48
  %37 = getelementptr inbounds nuw i8, ptr %35, i64 %36
  call void @llvm.lifetime.start.p0(ptr nonnull %9), !noalias !36508
  %38 = shl nuw nsw i64 %4, 4
  %39 = getelementptr inbounds nuw i8, ptr %3, i64 %38
  %40 = icmp eq i64 %4, 0
  br i1 %40, label %41, label %.preheader41

41:                                               ; preds = %33
  %42 = getelementptr inbounds nuw i8, ptr %35, i64 48
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36515)
  %43 = getelementptr i8, ptr %35, i64 24
  %44 = load ptr, ptr %43, align 8, !noalias !36518
  %45 = getelementptr i8, ptr %35, i64 32
  %46 = load i64, ptr %45, align 8, !noalias !36518
  br label %.loopexit40

.preheader41:                                     ; preds = %33, %72
  %47 = phi ptr [ %48, %72 ], [ %35, %33 ]
  %48 = getelementptr inbounds nuw i8, ptr %47, i64 48
  %49 = getelementptr inbounds nuw i8, ptr %47, i64 24
  %50 = load ptr, ptr %49, align 8, !noalias !36521, !nonnull !1733, !noundef !1733
  %51 = getelementptr i8, ptr %47, i64 32
  %52 = load i64, ptr %51, align 8, !noalias !36521
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36515)
  %53 = getelementptr inbounds nuw i8, ptr %50, i64 16
  br label %54

54:                                               ; preds = %70, %.preheader41
  %55 = phi ptr [ %3, %.preheader41 ], [ %56, %70 ]
  %56 = getelementptr inbounds nuw i8, ptr %55, i64 16
  %57 = load ptr, ptr %55, align 8, !alias.scope !36515, !noalias !36527, !nonnull !1733, !noundef !1733
  %58 = getelementptr i8, ptr %55, i64 8
  %59 = load i64, ptr %58, align 8, !alias.scope !36515, !noalias !36527, !noundef !1733
  %60 = icmp eq ptr %57, %50
  %61 = icmp eq i64 %59, %52
  %62 = xor i1 %61, true
  %63 = or i1 %60, %62
  br i1 %63, label %68, label %64

64:                                               ; preds = %54
  %65 = getelementptr inbounds nuw i8, ptr %57, i64 16
  %66 = tail call i32 @bcmp(ptr nonnull readonly %65, ptr nonnull readonly %53, i64 %52), !alias.scope !36530, !noalias !36534
  %67 = icmp eq i32 %66, 0
  br i1 %67, label %72, label %70

68:                                               ; preds = %54
  %69 = and i1 %60, %61
  br i1 %69, label %72, label %70

70:                                               ; preds = %68, %64
  %71 = icmp eq ptr %56, %39
  br i1 %71, label %.loopexit40, label %54

72:                                               ; preds = %68, %64
  %73 = icmp eq ptr %48, %37
  br i1 %73, label %.thread, label %.preheader41

.thread:                                          ; preds = %72
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !36508
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !36535
  store ptr inttoptr (i64 8 to ptr), ptr %12, align 8, !noalias !36535
  %74 = getelementptr inbounds nuw i8, ptr %12, i64 8
  %75 = getelementptr inbounds nuw i8, ptr %12, i64 16
  store i64 0, ptr %75, align 8, !noalias !36535
  %76 = getelementptr inbounds nuw i8, ptr %12, i64 24
  store ptr inttoptr (i64 8 to ptr), ptr %76, align 8, !noalias !36535
  br label %.loopexit33

.loopexit40:                                      ; preds = %70, %41
  %77 = phi i64 [ %46, %41 ], [ %52, %70 ]
  %78 = phi ptr [ %44, %41 ], [ %50, %70 ]
  %79 = phi ptr [ %42, %41 ], [ %48, %70 ]
  %80 = atomicrmw add ptr %78, i64 1 monotonic, align 8, !noalias !36518
  %81 = icmp slt i64 %80, 0
  br i1 %81, label %82, label %88

82:                                               ; preds = %.loopexit40
  tail call void @llvm.trap()
  unreachable

83:                                               ; preds = %__rustc::__rust_alloc (.exit.thread)
  %84 = landingpad { ptr, i32 }
          cleanup
  %85 = atomicrmw sub ptr %78, i64 1 release, align 8, !noalias !36536
  %86 = icmp eq i64 %85, 1
  br i1 %86, label %87, label %614

87:                                               ; preds = %83
  fence acquire
; invoke <alloc::sync::Arc<str>>::drop_slow
  invoke void @<alloc::sync::Arc<str>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %8) #91
          to label %614 unwind label %218, !noalias !36508

88:                                               ; preds = %.loopexit40
  call void @llvm.lifetime.start.p0(ptr nonnull %8), !noalias !36508
  store ptr %78, ptr %8, align 8, !noalias !36508
  %89 = getelementptr inbounds nuw i8, ptr %8, i64 8
  store i64 %77, ptr %89, align 8, !noalias !36508
  %90 = tail call noundef dereferenceable_or_null(64) ptr @malloc(i64 noundef range(i64 1, 0) 64) #92, !noalias !36543
  %91 = icmp eq ptr %90, null
  br i1 %91, label %__rustc::__rust_alloc (.exit.thread), label %92

92:                                               ; preds = %88
  %93 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %94 = load i64, ptr %93, align 8, !noalias !36543, !noundef !1733
  %95 = tail call i64 @llvm.uadd.sat.i64(i64 %94, i64 1)
  store i64 %95, ptr %93, align 8, !noalias !36543
  %96 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %97 = load i64, ptr %96, align 8, !noalias !36543, !noundef !1733
  %98 = tail call i64 @llvm.uadd.sat.i64(i64 %97, i64 64)
  store i64 %98, ptr %96, align 8, !noalias !36543
  %99 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %100 = load i64, ptr %99, align 8, !noalias !36543, !noundef !1733
  %101 = tail call i64 @llvm.sadd.sat.i64(i64 %100, i64 64)
  store i64 %101, ptr %99, align 8, !noalias !36543
  %102 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %103 = load i64, ptr %102, align 8, !noalias !36543, !noundef !1733
  %104 = icmp sgt i64 %101, %103
  br i1 %104, label %105, label %.preheader229

105:                                              ; preds = %92
  store i64 %101, ptr %102, align 8, !noalias !36543
  br label %.preheader229

.preheader229:                                    ; preds = %105, %92
  br label %106

106:                                              ; preds = %.preheader229, %109
  %107 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !36543
  %108 = icmp slt i64 %107, 0
  br i1 %108, label %109, label %__rustc::__rust_alloc (.exit)

109:                                              ; preds = %106
  %110 = add nsw i64 %107, 1
  %111 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %107, i64 %110 acq_rel acquire, align 8, !noalias !36543
  %112 = extractvalue { i64, i1 } %111, 1
  br i1 %112, label %113, label %106

113:                                              ; preds = %109
  %114 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !36543
  %115 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 64 monotonic, align 8, !noalias !36543
  %116 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 64 monotonic, align 8, !noalias !36543
  %117 = tail call i64 @llvm.sadd.sat.i64(i64 %116, i64 64)
  %118 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !36543
  br label %119

119:                                              ; preds = %122, %113
  %120 = phi i64 [ %118, %113 ], [ %125, %122 ]
  %121 = icmp sgt i64 %117, %120
  br i1 %121, label %122, label %126

122:                                              ; preds = %119
  %123 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %120, i64 %117 monotonic monotonic, align 8, !noalias !36543
  %124 = extractvalue { i64, i1 } %123, 1
  %125 = extractvalue { i64, i1 } %123, 0
  br i1 %124, label %126, label %119

126:                                              ; preds = %122, %119
  %127 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !36543
  br label %__rustc::__rust_alloc (.exit)

__rustc::__rust_alloc (.exit.thread): ; preds = %88
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef 8, i64 64) #93
          to label %128 unwind label %83, !noalias !36508

128:                                              ; preds = %__rustc::__rust_alloc (.exit.thread)
  unreachable

__rustc::__rust_alloc (.exit):  ; preds = %106, %126
  store ptr %78, ptr %90, align 8, !noalias !36508
  %129 = getelementptr inbounds nuw i8, ptr %90, i64 8
  store i64 %77, ptr %129, align 8, !noalias !36508
  store i64 4, ptr %9, align 8, !noalias !36508
  %130 = getelementptr inbounds nuw i8, ptr %9, i64 8
  store ptr %90, ptr %130, align 8, !noalias !36508
  %131 = getelementptr inbounds nuw i8, ptr %9, i64 16
  store i64 1, ptr %131, align 8, !noalias !36508
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !36508
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36546)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36549)
  %132 = icmp eq ptr %79, %37
  br i1 %132, label %.loopexit35, label %133

133:                                              ; preds = %__rustc::__rust_alloc (.exit)
  %134 = getelementptr inbounds nuw i8, ptr %7, i64 8
  br i1 %40, label %.preheader, label %.preheader37

.preheader:                                       ; preds = %133, %152
  %135 = phi ptr [ %153, %152 ], [ %90, %133 ]
  %136 = phi i64 [ %156, %152 ], [ 1, %133 ]
  %137 = phi ptr [ %138, %152 ], [ %79, %133 ]
  %138 = getelementptr inbounds nuw i8, ptr %137, i64 48
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36552)
  %139 = getelementptr i8, ptr %137, i64 24
  %140 = load ptr, ptr %139, align 8, !noalias !36555, !nonnull !1733, !noundef !1733
  %141 = getelementptr i8, ptr %137, i64 32
  %142 = load i64, ptr %141, align 8, !noalias !36555
  %143 = atomicrmw add ptr %140, i64 1 monotonic, align 8, !noalias !36555
  %144 = icmp slt i64 %143, 0
  br i1 %144, label %.loopexit34, label %145

145:                                              ; preds = %.preheader
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !36560
  store ptr %140, ptr %7, align 8, !noalias !36560
  store i64 %142, ptr %134, align 8, !noalias !36560
  %146 = icmp samesign ult i64 %136, 576460752303423488
  tail call void @llvm.assume(i1 %146)
  %147 = load i64, ptr %9, align 8, !range !1828, !alias.scope !36561, !noalias !36562, !noundef !1733
  %148 = icmp eq i64 %136, %147
  br i1 %148, label %149, label %152

149:                                              ; preds = %145
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.1794586459888082020)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %9, i64 noundef %136, i64 noundef 1, i64 noundef 8, i64 noundef 16)
          to label %150 unwind label %158, !noalias !36562

150:                                              ; preds = %149
  %151 = load ptr, ptr %130, align 8, !alias.scope !36561, !noalias !36562
  br label %152

152:                                              ; preds = %150, %145
  %153 = phi ptr [ %151, %150 ], [ %135, %145 ]
  %154 = getelementptr inbounds nuw [16 x i8], ptr %153, i64 %136
  store ptr %140, ptr %154, align 8, !noalias !36560
  %155 = getelementptr inbounds nuw i8, ptr %154, i64 8
  store i64 %142, ptr %155, align 8, !noalias !36560
  %156 = add nuw nsw i64 %136, 1
  store i64 %156, ptr %131, align 8, !alias.scope !36561, !noalias !36562
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !36560
  %157 = icmp eq ptr %138, %37
  br i1 %157, label %.loopexit35, label %.preheader

158:                                              ; preds = %149
  %159 = landingpad { ptr, i32 }
          cleanup
  br label %206

.preheader37:                                     ; preds = %133, %198
  %160 = phi ptr [ %199, %198 ], [ %90, %133 ]
  %161 = phi i64 [ %202, %198 ], [ 1, %133 ]
  %162 = phi ptr [ %165, %198 ], [ %79, %133 ]
  br label %163

163:                                              ; preds = %189, %.preheader37
  %164 = phi ptr [ %165, %189 ], [ %162, %.preheader37 ]
  %165 = getelementptr inbounds nuw i8, ptr %164, i64 48
  %166 = getelementptr i8, ptr %164, i64 24
  %167 = load ptr, ptr %166, align 8, !noalias !36563, !nonnull !1733, !noundef !1733
  %168 = getelementptr i8, ptr %164, i64 32
  %169 = load i64, ptr %168, align 8, !noalias !36563
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36552)
  %170 = getelementptr inbounds nuw i8, ptr %167, i64 16
  br label %171

171:                                              ; preds = %187, %163
  %172 = phi ptr [ %3, %163 ], [ %173, %187 ]
  %173 = getelementptr inbounds nuw i8, ptr %172, i64 16
  %174 = load ptr, ptr %172, align 8, !alias.scope !36552, !noalias !36569, !nonnull !1733, !noundef !1733
  %175 = getelementptr i8, ptr %172, i64 8
  %176 = load i64, ptr %175, align 8, !alias.scope !36552, !noalias !36569, !noundef !1733
  %177 = icmp eq ptr %174, %167
  %178 = icmp eq i64 %176, %169
  %179 = xor i1 %178, true
  %180 = or i1 %177, %179
  br i1 %180, label %185, label %181

181:                                              ; preds = %171
  %182 = getelementptr inbounds nuw i8, ptr %174, i64 16
  %183 = tail call i32 @bcmp(ptr nonnull readonly %182, ptr nonnull readonly %170, i64 %169), !alias.scope !36572, !noalias !36576
  %184 = icmp eq i32 %183, 0
  br i1 %184, label %189, label %187

185:                                              ; preds = %171
  %186 = and i1 %177, %178
  br i1 %186, label %189, label %187

187:                                              ; preds = %185, %181
  %188 = icmp eq ptr %173, %39
  br i1 %188, label %191, label %171

189:                                              ; preds = %185, %181
  %190 = icmp eq ptr %165, %37
  br i1 %190, label %.loopexit35, label %163

191:                                              ; preds = %187
  %192 = atomicrmw add ptr %167, i64 1 monotonic, align 8, !noalias !36555
  %193 = icmp slt i64 %192, 0
  br i1 %193, label %.loopexit34, label %194

.loopexit34:                                      ; preds = %191, %.preheader
  tail call void @llvm.trap()
  unreachable

194:                                              ; preds = %191
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !36560
  store ptr %167, ptr %7, align 8, !noalias !36560
  store i64 %169, ptr %134, align 8, !noalias !36560
  %195 = icmp samesign ult i64 %161, 576460752303423488
  tail call void @llvm.assume(i1 %195)
  %196 = load i64, ptr %9, align 8, !range !1828, !alias.scope !36561, !noalias !36562, !noundef !1733
  %197 = icmp eq i64 %161, %196
  br i1 %197, label %212, label %198

198:                                              ; preds = %213, %194
  %199 = phi ptr [ %214, %213 ], [ %160, %194 ]
  %200 = getelementptr inbounds nuw [16 x i8], ptr %199, i64 %161
  store ptr %167, ptr %200, align 8, !noalias !36560
  %201 = getelementptr inbounds nuw i8, ptr %200, i64 8
  store i64 %169, ptr %201, align 8, !noalias !36560
  %202 = add nuw nsw i64 %161, 1
  store i64 %202, ptr %131, align 8, !alias.scope !36561, !noalias !36562
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !36560
  %203 = icmp eq ptr %165, %37
  br i1 %203, label %.loopexit35, label %.preheader37

204:                                              ; preds = %212
  %205 = landingpad { ptr, i32 }
          cleanup
  br label %206

206:                                              ; preds = %204, %158
  %207 = phi ptr [ %167, %204 ], [ %140, %158 ]
  %208 = phi { ptr, i32 } [ %205, %204 ], [ %159, %158 ]
  %209 = atomicrmw sub ptr %207, i64 1 release, align 8, !noalias !36577
  %210 = icmp eq i64 %209, 1
  br i1 %210, label %211, label %217

211:                                              ; preds = %206
  fence acquire
; invoke <alloc::sync::Arc<str>>::drop_slow
  invoke void @<alloc::sync::Arc<str>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %7) #91
          to label %217 unwind label %215, !noalias !36560

212:                                              ; preds = %194
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.1794586459888082020)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %9, i64 noundef %161, i64 noundef 1, i64 noundef 8, i64 noundef 16)
          to label %213 unwind label %204, !noalias !36562

213:                                              ; preds = %212
  %214 = load ptr, ptr %130, align 8, !alias.scope !36561, !noalias !36562
  br label %198

215:                                              ; preds = %211
  %216 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !36560
  unreachable

217:                                              ; preds = %211, %206
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_algebra::ast::Variable>>
  invoke void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_algebra::ast::Variable>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %9) #89
          to label %614 unwind label %218, !noalias !36508

218:                                              ; preds = %217, %87
  %219 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !36508
  unreachable

.loopexit35:                                      ; preds = %198, %189, %152, %__rustc::__rust_alloc (.exit)
  %220 = phi i64 [ %161, %189 ], [ %156, %152 ], [ 1, %__rustc::__rust_alloc (.exit) ], [ %202, %198 ]
  %221 = load i64, ptr %9, align 8, !noalias !36584
  %222 = load ptr, ptr %130, align 8, !noalias !36584
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !36508
  %223 = icmp ult i64 %220, 576460752303423488
  tail call void @llvm.assume(i1 %223)
  %224 = shl nuw nsw i64 %220, 4
  %225 = getelementptr inbounds nuw i8, ptr %222, i64 %224
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !36535
  store ptr %222, ptr %12, align 8, !noalias !36535
  %226 = getelementptr inbounds nuw i8, ptr %12, i64 8
  %227 = getelementptr inbounds nuw i8, ptr %12, i64 16
  store i64 %221, ptr %227, align 8, !noalias !36535
  %228 = getelementptr inbounds nuw i8, ptr %12, i64 24
  store ptr %225, ptr %228, align 8, !noalias !36535
  %229 = getelementptr inbounds nuw i8, ptr %11, i64 24
  %230 = getelementptr inbounds nuw i8, ptr %11, i64 32
  %231 = getelementptr inbounds nuw i8, ptr %11, i64 40
  %232 = load i64, ptr %28, align 8, !alias.scope !36585, !noalias !36588
  br label %234

233:                                              ; preds = %244
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>(ptr noalias nofree noundef align 8 dereferenceable(32) %12) #89
          to label %614 unwind label %261, !noalias !36590, !inline_history !36591

234:                                              ; preds = %263, %.loopexit35
  %235 = phi i64 [ %232, %.loopexit35 ], [ %266, %263 ]
  %236 = phi ptr [ %222, %.loopexit35 ], [ %237, %263 ]
  %237 = getelementptr inbounds nuw i8, ptr %236, i64 16
  %238 = load ptr, ptr %236, align 8, !noalias !36592, !nonnull !1733, !noundef !1733
  %239 = getelementptr inbounds nuw i8, ptr %236, i64 8
  %240 = load i64, ptr %239, align 8, !noalias !36592, !noundef !1733
  call void @llvm.lifetime.start.p0(ptr nonnull %11), !noalias !36535
  store ptr %238, ptr %229, align 8, !noalias !36535
  store i64 %240, ptr %230, align 8, !noalias !36535
  store i64 2, ptr %11, align 8, !noalias !36535
  store i8 0, ptr %231, align 8, !noalias !36535
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36585)
  %241 = load i64, ptr %27, align 8, !range !1828, !alias.scope !36585, !noalias !36588, !noundef !1733
  %242 = icmp eq i64 %235, %241
  br i1 %242, label %243, label %263

243:                                              ; preds = %234
; invoke <alloc::raw_vec::RawVec<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %27)
          to label %263 unwind label %244, !noalias !36588

244:                                              ; preds = %243
  %245 = landingpad { ptr, i32 }
          cleanup
  store ptr %237, ptr %226, align 8
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(48) %11) #89
          to label %233 unwind label %246, !noalias !36595

246:                                              ; preds = %244
  %247 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !36595
  unreachable

.loopexit33:                                      ; preds = %263, %.thread
  %248 = phi ptr [ %74, %.thread ], [ %226, %263 ]
  %249 = phi ptr [ inttoptr (i64 8 to ptr), %.thread ], [ %225, %263 ]
  store ptr %249, ptr %248, align 1
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>(ptr noalias nofree noundef align 8 dereferenceable(32) %12)
          to label %250 unwind label %269, !inline_history !36591

250:                                              ; preds = %.loopexit33
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !36535
  call void @llvm.lifetime.start.p0(ptr nonnull %10), !noalias !36535
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %10, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef nonnull align 16 dereferenceable(1232) %5, ptr noundef nonnull readonly align 8 dereferenceable(144) %2)
          to label %251 unwind label %269, !inline_history !36507

251:                                              ; preds = %250
  %252 = load i64, ptr %28, align 8, !alias.scope !36596, !noalias !36599, !noundef !1733
  %253 = icmp ugt i64 %29, %252
  br i1 %253, label %260, label %254

254:                                              ; preds = %251
  %255 = sub nuw i64 %252, %29
  %256 = load ptr, ptr %34, align 16, !alias.scope !36596, !noalias !36599, !nonnull !1733, !noundef !1733
  %257 = getelementptr inbounds nuw [48 x i8], ptr %256, i64 %29
  store i64 %29, ptr %28, align 8, !alias.scope !36596, !noalias !36599
; invoke core::ptr::drop_glue::<[purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>]>
  invoke fastcc void @core::ptr::drop_glue::<[purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>]>(ptr noalias nofree noundef nonnull align 8 %257, i64 noundef %255)
          to label %260 unwind label %258

258:                                              ; preds = %254
  %259 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<core::result::Result<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>, purrdf_sparql_eval::error::EvalError>>
  invoke fastcc void @core::ptr::drop_glue::<core::result::Result<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>, purrdf_sparql_eval::error::EvalError>>(ptr noalias nofree noundef align 16 dereferenceable(112) %10) #89
          to label %614 unwind label %261, !noalias !36599, !inline_history !36591

260:                                              ; preds = %254, %251
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(112) %20, ptr noundef nonnull align 16 dereferenceable(112) %10, i64 112, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %10), !noalias !36535
  br label %271

261:                                              ; preds = %258, %233
  %262 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !36599, !inline_history !36591
  unreachable

263:                                              ; preds = %243, %234
  %264 = load ptr, ptr %34, align 16, !alias.scope !36585, !noalias !36588, !nonnull !1733, !noundef !1733
  %265 = getelementptr inbounds nuw [48 x i8], ptr %264, i64 %235
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %265, ptr noundef nonnull align 8 dereferenceable(48) %11, i64 48, i1 false), !noalias !36595
  %266 = add i64 %235, 1
  store i64 %266, ptr %28, align 8, !alias.scope !36585, !noalias !36588
  call void @llvm.lifetime.end.p0(ptr nonnull %11), !noalias !36535
  %267 = icmp eq ptr %237, %225
  br i1 %267, label %.loopexit33, label %234

268:                                              ; preds = %368
  br i1 %369, label %614, label %612

269:                                              ; preds = %364, %357, %250, %.loopexit33, %32
  %270 = landingpad { ptr, i32 }
          cleanup
  br label %614

271:                                              ; preds = %260, %32
  %272 = load i64, ptr %20, align 16, !range !1732, !noundef !1733
  %273 = trunc nuw i64 %272 to i1
  br i1 %273, label %274, label %357

274:                                              ; preds = %271
  %275 = getelementptr inbounds nuw i8, ptr %20, i64 16
  %276 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %276, ptr noundef nonnull align 16 dereferenceable(96) %275, i64 96, i1 false)
  store i64 1, ptr %0, align 16
  %277 = getelementptr inbounds nuw i8, ptr %21, i64 72
  %278 = load i64, ptr %277, align 8, !range !1771, !noundef !1733
  %279 = icmp ugt i64 %278, 5
  br i1 %279, label %280, label %314

280:                                              ; preds = %274
  %281 = getelementptr inbounds nuw i8, ptr %21, i64 80
  %282 = load ptr, ptr %281, align 8, !nonnull !1733, !noundef !1733
  %283 = mul i64 %278, 3
  %284 = add i64 %283, -3
  %285 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %286 = load i64, ptr %285, align 8, !noalias !36600, !noundef !1733
  %287 = tail call i64 @llvm.umin.i64(i64 %284, i64 9223372036854775807)
  %288 = tail call i64 @llvm.ssub.sat.i64(i64 %286, i64 %287)
  store i64 %288, ptr %285, align 8, !noalias !36600
  %289 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %290 = load i64, ptr %289, align 8, !noalias !36600, !noundef !1733
  %291 = icmp slt i64 %288, %290
  br i1 %291, label %292, label %.preheader203

292:                                              ; preds = %280
  store i64 %288, ptr %289, align 8, !noalias !36600
  br label %.preheader203

.preheader203:                                    ; preds = %292, %280
  br label %293

293:                                              ; preds = %.preheader203, %296
  %294 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !36600
  %295 = icmp slt i64 %294, 0
  br i1 %295, label %296, label %__rustc::__rust_dealloc (.exit)

296:                                              ; preds = %293
  %297 = add nsw i64 %294, 1
  %298 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %294, i64 %297 acq_rel acquire, align 8, !noalias !36600
  %299 = extractvalue { i64, i1 } %298, 1
  br i1 %299, label %300, label %293

300:                                              ; preds = %296
  %301 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %287 monotonic, align 8, !noalias !36600
  %302 = tail call i64 @llvm.ssub.sat.i64(i64 %301, i64 %287)
  %303 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !36600
  br label %304

304:                                              ; preds = %307, %300
  %305 = phi i64 [ %303, %300 ], [ %310, %307 ]
  %306 = icmp slt i64 %302, %305
  br i1 %306, label %307, label %311

307:                                              ; preds = %304
  %308 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %305, i64 %302 monotonic monotonic, align 8, !noalias !36600
  %309 = extractvalue { i64, i1 } %308, 1
  %310 = extractvalue { i64, i1 } %308, 0
  br i1 %309, label %311, label %304

311:                                              ; preds = %307, %304
  %312 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !36600
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %293, %311
  %313 = icmp ne i64 %284, 0
  tail call void @llvm.assume(i1 %313), !noalias !36600
  tail call void @free(ptr noundef nonnull %282) #92, !noalias !36600
  br label %314

314:                                              ; preds = %__rustc::__rust_dealloc (.exit), %274
  %315 = load i64, ptr %21, align 8, !range !2052, !noundef !1733
  %316 = icmp sgt i64 %315, 0
  br i1 %316, label %317, label %349

317:                                              ; preds = %314
  %318 = getelementptr inbounds nuw i8, ptr %21, i64 8
  %319 = load ptr, ptr %318, align 8, !nonnull !1733, !noundef !1733
  %320 = mul nuw i64 %315, 3
  %321 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %322 = load i64, ptr %321, align 8, !noalias !36605, !noundef !1733
  %323 = tail call i64 @llvm.umin.i64(i64 %320, i64 9223372036854775807)
  %324 = tail call i64 @llvm.ssub.sat.i64(i64 %322, i64 %323)
  store i64 %324, ptr %321, align 8, !noalias !36605
  %325 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %326 = load i64, ptr %325, align 8, !noalias !36605, !noundef !1733
  %327 = icmp slt i64 %324, %326
  br i1 %327, label %328, label %.preheader202

328:                                              ; preds = %317
  store i64 %324, ptr %325, align 8, !noalias !36605
  br label %.preheader202

.preheader202:                                    ; preds = %328, %317
  br label %329

329:                                              ; preds = %.preheader202, %332
  %330 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !36605
  %331 = icmp slt i64 %330, 0
  br i1 %331, label %332, label %__rustc::__rust_dealloc (.exit28)

332:                                              ; preds = %329
  %333 = add nsw i64 %330, 1
  %334 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %330, i64 %333 acq_rel acquire, align 8, !noalias !36605
  %335 = extractvalue { i64, i1 } %334, 1
  br i1 %335, label %336, label %329

336:                                              ; preds = %332
  %337 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %323 monotonic, align 8, !noalias !36605
  %338 = tail call i64 @llvm.ssub.sat.i64(i64 %337, i64 %323)
  %339 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !36605
  br label %340

340:                                              ; preds = %343, %336
  %341 = phi i64 [ %339, %336 ], [ %346, %343 ]
  %342 = icmp slt i64 %338, %341
  br i1 %342, label %343, label %347

343:                                              ; preds = %340
  %344 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %341, i64 %338 monotonic monotonic, align 8, !noalias !36605
  %345 = extractvalue { i64, i1 } %344, 1
  %346 = extractvalue { i64, i1 } %344, 0
  br i1 %345, label %347, label %340

347:                                              ; preds = %343, %340
  %348 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !36605
  br label %__rustc::__rust_dealloc (.exit28)

__rustc::__rust_dealloc (.exit28): ; preds = %329, %347
  tail call void @free(ptr noundef nonnull %319) #92, !noalias !36605
  br label %349

349:                                              ; preds = %__rustc::__rust_dealloc (.exit28), %314
  %350 = getelementptr inbounds nuw i8, ptr %21, i64 96
  %351 = load ptr, ptr %350, align 8, !noundef !1733
  %352 = icmp eq ptr %351, null
  br i1 %352, label %552, label %353

353:                                              ; preds = %349
  %354 = atomicrmw sub ptr %351, i64 1 release, align 8, !noalias !36606
  %355 = icmp eq i64 %354, 1
  br i1 %355, label %356, label %552

356:                                              ; preds = %353
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %350) #91
  br label %552

357:                                              ; preds = %271
  %358 = getelementptr inbounds nuw i8, ptr %20, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %22)
; invoke <purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %22, ptr noalias nofree noundef align 8 dereferenceable(104) %21, i64 noundef 0, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(96) %358)
          to label %359 unwind label %269

359:                                              ; preds = %357
  %360 = load i64, ptr %22, align 8, !range !2052, !noundef !1733
  %361 = icmp eq i64 %360, -1
  br i1 %361, label %364, label %362

362:                                              ; preds = %359
  call void @llvm.lifetime.start.p0(ptr nonnull %23)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %23, ptr noundef nonnull align 8 dereferenceable(32) %22, i64 32, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %22)
; invoke <purrdf_sparql_eval::solution::VarSchema>::interned
  %363 = invoke noundef nonnull ptr @<purrdf_sparql_eval::solution::VarSchema>::interned(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %3, i64 noundef %4)
          to label %371 unwind label %366

364:                                              ; preds = %359
  call void @llvm.lifetime.end.p0(ptr nonnull %22)
; invoke <purrdf_sparql_eval::solution::VarSchema>::interned
  %365 = invoke noundef nonnull ptr @<purrdf_sparql_eval::solution::VarSchema>::interned(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %3, i64 noundef %4)
          to label %555 unwind label %269

366:                                              ; preds = %362
  %367 = landingpad { ptr, i32 }
          cleanup
  br label %368

368:                                              ; preds = %550, %547, %434, %366
  %369 = phi i1 [ true, %366 ], [ false, %434 ], [ false, %550 ], [ false, %547 ]
  %370 = phi { ptr, i32 } [ %367, %366 ], [ %449, %434 ], [ %546, %550 ], [ %546, %547 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>(ptr noalias nofree noundef align 8 dereferenceable(32) %23) #89
          to label %268 unwind label %553

371:                                              ; preds = %362
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36613)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36616)
  call void @llvm.lifetime.start.p0(ptr nonnull %19)
  store ptr %363, ptr %19, align 8, !noalias !36618
  call void @llvm.lifetime.start.p0(ptr nonnull %18), !noalias !36618
  %372 = getelementptr inbounds nuw i8, ptr %363, i64 24
  %373 = load ptr, ptr %372, align 8, !noalias !36618, !nonnull !1733, !noundef !1733
  %374 = getelementptr inbounds nuw i8, ptr %363, i64 32
  %375 = load i64, ptr %374, align 8, !noalias !36618, !noundef !1733
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36620)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36623), !noalias !36626
  %376 = shl nuw nsw i64 %375, 4
  %377 = icmp eq i64 %375, 0
  br i1 %377, label %.loopexit, label %378

378:                                              ; preds = %371
  %379 = tail call noundef ptr @malloc(i64 noundef range(i64 1, 0) %376) #92, !noalias !36627
  %380 = icmp eq ptr %379, null
  br i1 %380, label %__rustc::__rust_alloc (.exit29.thread), label %381

381:                                              ; preds = %378
  %382 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %383 = load i64, ptr %382, align 8, !noalias !36627, !noundef !1733
  %384 = tail call i64 @llvm.uadd.sat.i64(i64 %383, i64 1)
  store i64 %384, ptr %382, align 8, !noalias !36627
  %385 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %386 = load i64, ptr %385, align 8, !noalias !36627, !noundef !1733
  %387 = tail call i64 @llvm.uadd.sat.i64(i64 %386, i64 %376)
  store i64 %387, ptr %385, align 8, !noalias !36627
  %388 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %389 = load i64, ptr %388, align 8, !noalias !36627, !noundef !1733
  %390 = tail call i64 @llvm.sadd.sat.i64(i64 %389, i64 %376)
  store i64 %390, ptr %388, align 8, !noalias !36627
  %391 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %392 = load i64, ptr %391, align 8, !noalias !36627, !noundef !1733
  %393 = icmp sgt i64 %390, %392
  br i1 %393, label %394, label %.preheader207

394:                                              ; preds = %381
  store i64 %390, ptr %391, align 8, !noalias !36627
  br label %.preheader207

.preheader207:                                    ; preds = %394, %381
  br label %395

395:                                              ; preds = %.preheader207, %398
  %396 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !36627
  %397 = icmp slt i64 %396, 0
  br i1 %397, label %398, label %__rustc::__rust_alloc (.exit29)

398:                                              ; preds = %395
  %399 = add nsw i64 %396, 1
  %400 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %396, i64 %399 acq_rel acquire, align 8, !noalias !36627
  %401 = extractvalue { i64, i1 } %400, 1
  br i1 %401, label %402, label %395

402:                                              ; preds = %398
  %403 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !36627
  %404 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 %376 monotonic, align 8, !noalias !36627
  %405 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %376 monotonic, align 8, !noalias !36627
  %406 = tail call i64 @llvm.sadd.sat.i64(i64 %405, i64 %376)
  %407 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !36627
  br label %408

408:                                              ; preds = %411, %402
  %409 = phi i64 [ %407, %402 ], [ %414, %411 ]
  %410 = icmp sgt i64 %406, %409
  br i1 %410, label %411, label %415

411:                                              ; preds = %408
  %412 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %409, i64 %406 monotonic monotonic, align 8, !noalias !36627
  %413 = extractvalue { i64, i1 } %412, 1
  %414 = extractvalue { i64, i1 } %412, 0
  br i1 %413, label %415, label %408

415:                                              ; preds = %411, %408
  %416 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !36627
  br label %__rustc::__rust_alloc (.exit29)

__rustc::__rust_alloc (.exit29.thread): ; preds = %378
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef 8, i64 %376) #93
          to label %417 unwind label %435

417:                                              ; preds = %__rustc::__rust_alloc (.exit29.thread)
  unreachable

__rustc::__rust_alloc (.exit29): ; preds = %395, %415
  %418 = getelementptr inbounds nuw i8, ptr %23, i64 24
  %419 = load ptr, ptr %418, align 8, !noalias !36632, !nonnull !1733, !noundef !1733
  %420 = getelementptr inbounds nuw i8, ptr %419, i64 16
  br label %421

421:                                              ; preds = %425, %__rustc::__rust_alloc (.exit29)
  %422 = phi i64 [ 0, %__rustc::__rust_alloc (.exit29) ], [ %430, %425 ]
  %423 = getelementptr inbounds nuw [16 x i8], ptr %373, i64 %422
; invoke <purrdf_sparql_eval::solution::VarSchema>::index_of
  %424 = invoke fastcc { i64, i64 } @<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.1794586459888082020)(ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %420, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(16) %423) #87
          to label %425 unwind label %432, !noalias !36652

425:                                              ; preds = %421
  %426 = extractvalue { i64, i64 } %424, 0
  %427 = extractvalue { i64, i64 } %424, 1
  %428 = getelementptr inbounds nuw [16 x i8], ptr %379, i64 %422
  store i64 %426, ptr %428, align 8, !noalias !36653
  %429 = getelementptr inbounds nuw i8, ptr %428, i64 8
  store i64 %427, ptr %429, align 8, !noalias !36653
  %430 = add nuw i64 %422, 1
  %431 = icmp eq i64 %430, %375
  br i1 %431, label %.loopexit, label %421

432:                                              ; preds = %421
  %433 = landingpad { ptr, i32 }
          cleanup
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %379, i64 noundef %376, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !36658
  br label %545

434:                                              ; preds = %452, %447
  br i1 %448, label %368, label %545

435:                                              ; preds = %__rustc::__rust_alloc (.exit29.thread)
  %436 = landingpad { ptr, i32 }
          cleanup
  br label %545

.loopexit:                                        ; preds = %425, %371
  %437 = phi ptr [ inttoptr (i64 8 to ptr), %371 ], [ %379, %425 ]
  store i64 %375, ptr %18, align 8, !alias.scope !36659, !noalias !36660
  %438 = getelementptr inbounds nuw i8, ptr %18, i64 8
  store ptr %437, ptr %438, align 8, !alias.scope !36659, !noalias !36660
  %439 = getelementptr inbounds nuw i8, ptr %18, i64 16
  store i64 %375, ptr %439, align 8, !alias.scope !36659, !noalias !36660
  call void @llvm.lifetime.start.p0(ptr nonnull %17), !noalias !36618
  call void @llvm.lifetime.start.p0(ptr nonnull %16), !noalias !36618
  %440 = getelementptr inbounds nuw i8, ptr %23, i64 8
  %441 = load ptr, ptr %440, align 8, !alias.scope !36616, !noalias !36626, !nonnull !1733, !noundef !1733
  %442 = getelementptr inbounds nuw i8, ptr %23, i64 16
  %443 = load i64, ptr %442, align 8, !alias.scope !36616, !noalias !36626, !noundef !1733
  %444 = getelementptr inbounds nuw [40 x i8], ptr %441, i64 %443
  store ptr %441, ptr %16, align 8, !noalias !36618
  %445 = getelementptr inbounds nuw i8, ptr %16, i64 8
  store ptr %444, ptr %445, align 8, !noalias !36618
  %446 = getelementptr inbounds nuw i8, ptr %16, i64 16
  store ptr %18, ptr %446, align 8, !noalias !36618
; invoke <alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>> as alloc::vec::spec_from_iter::SpecFromIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, core::iter::adapters::map::Map<core::slice::iter::Iter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, purrdf_sparql_eval::modifier::eval_project_sequence<purrdf_core::ir::term::TermId>::{closure#1}>>>::from_iter
  invoke fastcc void @<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>> as alloc::vec::spec_from_iter::SpecFromIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, core::iter::adapters::map::Map<core::slice::iter::Iter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, purrdf_sparql_eval::modifier::eval_project_sequence<purrdf_core::ir::term::TermId>::{closure#1}>>>::from_iter(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %17, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %16)
          to label %455 unwind label %447, !noalias !36626

447:                                              ; preds = %507, %.loopexit
  %448 = phi i1 [ true, %507 ], [ false, %.loopexit ]
  %449 = landingpad { ptr, i32 }
          cleanup
  %450 = load i64, ptr %18, align 8
  %451 = icmp eq i64 %450, 0
  br i1 %451, label %434, label %452

452:                                              ; preds = %447
  %453 = load ptr, ptr %438, align 8, !nonnull !1733, !noundef !1733
  %454 = shl nuw i64 %450, 4
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %453, i64 noundef %454, i64 noundef range(i64 1, -9223372036854775807) 8) #92
  br label %434

455:                                              ; preds = %.loopexit
  call void @llvm.lifetime.end.p0(ptr nonnull %16), !noalias !36618
  call void @llvm.lifetime.start.p0(ptr nonnull %15), !noalias !36618
  call void @llvm.lifetime.start.p0(ptr nonnull %14), !noalias !36618
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %14, ptr noundef nonnull align 8 dereferenceable(104) %21, i64 104, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %13), !noalias !36618
  %456 = getelementptr inbounds nuw i8, ptr %13, i64 24
  store ptr %363, ptr %456, align 8, !noalias !36618
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %13, ptr noundef nonnull align 8 dereferenceable(24) %17, i64 24, i1 false), !noalias !36618
  call void @llvm.experimental.noalias.scope.decl(metadata !36661)
  call void @llvm.experimental.noalias.scope.decl(metadata !36664)
  call void @llvm.experimental.noalias.scope.decl(metadata !36666)
  %457 = load i64, ptr %14, align 8, !range !2052, !alias.scope !36664, !noalias !36668, !noundef !1733
  %458 = icmp eq i64 %457, -1
  br i1 %458, label %460, label %459

459:                                              ; preds = %455
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %15, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %13, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %21)
  br label %462

460:                                              ; preds = %455
  %461 = getelementptr inbounds nuw i8, ptr %15, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %461, ptr noundef nonnull readonly align 8 dereferenceable(32) %13, i64 32, i1 false), !alias.scope !36669, !noalias !36670
  store i64 -1, ptr %15, align 8, !alias.scope !36661, !noalias !36671
  br label %462

462:                                              ; preds = %460, %459
  %463 = getelementptr inbounds nuw i8, ptr %14, i64 72
  %464 = load i64, ptr %463, align 8, !range !1771, !alias.scope !36672, !noalias !36668, !noundef !1733
  %465 = icmp ugt i64 %464, 5
  br i1 %465, label %466, label %500

466:                                              ; preds = %462
  %467 = getelementptr inbounds nuw i8, ptr %14, i64 80
  %468 = load ptr, ptr %467, align 8, !alias.scope !36664, !noalias !36668, !nonnull !1733, !noundef !1733
  %469 = mul i64 %464, 3
  %470 = add i64 %469, -3
  %471 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %472 = load i64, ptr %471, align 8, !noalias !36675, !noundef !1733
  %473 = call i64 @llvm.umin.i64(i64 %470, i64 9223372036854775807)
  %474 = call i64 @llvm.ssub.sat.i64(i64 %472, i64 %473)
  store i64 %474, ptr %471, align 8, !noalias !36675
  %475 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %476 = load i64, ptr %475, align 8, !noalias !36675, !noundef !1733
  %477 = icmp slt i64 %474, %476
  br i1 %477, label %478, label %.preheader206

478:                                              ; preds = %466
  store i64 %474, ptr %475, align 8, !noalias !36675
  br label %.preheader206

.preheader206:                                    ; preds = %478, %466
  br label %479

479:                                              ; preds = %.preheader206, %482
  %480 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !36675
  %481 = icmp slt i64 %480, 0
  br i1 %481, label %482, label %__rustc::__rust_dealloc (.exit30)

482:                                              ; preds = %479
  %483 = add nsw i64 %480, 1
  %484 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %480, i64 %483 acq_rel acquire, align 8, !noalias !36675
  %485 = extractvalue { i64, i1 } %484, 1
  br i1 %485, label %486, label %479

486:                                              ; preds = %482
  %487 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %473 monotonic, align 8, !noalias !36675
  %488 = call i64 @llvm.ssub.sat.i64(i64 %487, i64 %473)
  %489 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !36675
  br label %490

490:                                              ; preds = %493, %486
  %491 = phi i64 [ %489, %486 ], [ %496, %493 ]
  %492 = icmp slt i64 %488, %491
  br i1 %492, label %493, label %497

493:                                              ; preds = %490
  %494 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %491, i64 %488 monotonic monotonic, align 8, !noalias !36675
  %495 = extractvalue { i64, i1 } %494, 1
  %496 = extractvalue { i64, i1 } %494, 0
  br i1 %495, label %497, label %490

497:                                              ; preds = %493, %490
  %498 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !36675
  br label %__rustc::__rust_dealloc (.exit30)

__rustc::__rust_dealloc (.exit30): ; preds = %479, %497
  %499 = icmp ne i64 %470, 0
  call void @llvm.assume(i1 %499), !noalias !36675
  call void @free(ptr noundef nonnull %468) #92, !noalias !36675
  br label %500

500:                                              ; preds = %__rustc::__rust_dealloc (.exit30), %462
  %501 = getelementptr inbounds nuw i8, ptr %14, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !36678), !noalias !36626
  %502 = load ptr, ptr %501, align 8, !alias.scope !36681, !noalias !36668, !noundef !1733
  %503 = icmp eq ptr %502, null
  br i1 %503, label %508, label %504

504:                                              ; preds = %500
  %505 = atomicrmw sub ptr %502, i64 1 release, align 8, !noalias !36682
  %506 = icmp eq i64 %505, 1
  br i1 %506, label %507, label %508

507:                                              ; preds = %504
  fence acquire, !noalias !36626
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %501) #91
          to label %508 unwind label %447

508:                                              ; preds = %507, %504, %500
  call void @llvm.lifetime.end.p0(ptr nonnull %13), !noalias !36618
  call void @llvm.lifetime.end.p0(ptr nonnull %14), !noalias !36618
  %509 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %509, ptr noundef nonnull align 8 dereferenceable(96) %15, i64 96, i1 false), !noalias !36687
  store i64 0, ptr %0, align 16, !alias.scope !36613, !noalias !36687
  call void @llvm.lifetime.end.p0(ptr nonnull %15), !noalias !36618
  call void @llvm.lifetime.end.p0(ptr nonnull %17), !noalias !36618
  %510 = load i64, ptr %18, align 8
  %511 = icmp eq i64 %510, 0
  br i1 %511, label %551, label %512

512:                                              ; preds = %508
  %513 = load ptr, ptr %438, align 8, !nonnull !1733, !noundef !1733
  %514 = shl nuw i64 %510, 4
  %515 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %516 = load i64, ptr %515, align 8, !noundef !1733
  %517 = call i64 @llvm.umin.i64(i64 %514, i64 9223372036854775807)
  %518 = call i64 @llvm.ssub.sat.i64(i64 %516, i64 %517)
  store i64 %518, ptr %515, align 8
  %519 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %520 = load i64, ptr %519, align 8, !noundef !1733
  %521 = icmp slt i64 %518, %520
  br i1 %521, label %522, label %.preheader205

522:                                              ; preds = %512
  store i64 %518, ptr %519, align 8
  br label %.preheader205

.preheader205:                                    ; preds = %522, %512
  br label %523

523:                                              ; preds = %.preheader205, %526
  %524 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8
  %525 = icmp slt i64 %524, 0
  br i1 %525, label %526, label %__rustc::__rust_dealloc (.exit31)

526:                                              ; preds = %523
  %527 = add nsw i64 %524, 1
  %528 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %524, i64 %527 acq_rel acquire, align 8
  %529 = extractvalue { i64, i1 } %528, 1
  br i1 %529, label %530, label %523

530:                                              ; preds = %526
  %531 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %517 monotonic, align 8
  %532 = call i64 @llvm.ssub.sat.i64(i64 %531, i64 %517)
  %533 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8
  br label %534

534:                                              ; preds = %537, %530
  %535 = phi i64 [ %533, %530 ], [ %540, %537 ]
  %536 = icmp slt i64 %532, %535
  br i1 %536, label %537, label %541

537:                                              ; preds = %534
  %538 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %535, i64 %532 monotonic monotonic, align 8
  %539 = extractvalue { i64, i1 } %538, 1
  %540 = extractvalue { i64, i1 } %538, 0
  br i1 %539, label %541, label %534

541:                                              ; preds = %537, %534
  %542 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8
  br label %__rustc::__rust_dealloc (.exit31)

__rustc::__rust_dealloc (.exit31): ; preds = %523, %541
  call void @free(ptr noundef nonnull %513) #92
  br label %551

543:                                              ; preds = %550, %545
  %544 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !36613
  unreachable

545:                                              ; preds = %435, %434, %432
  %546 = phi { ptr, i32 } [ %449, %434 ], [ %436, %435 ], [ %433, %432 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef nonnull align 8 dereferenceable(104) %21) #89
          to label %547 unwind label %543

547:                                              ; preds = %545
  %548 = atomicrmw sub ptr %363, i64 1 release, align 8, !noalias !36688
  %549 = icmp eq i64 %548, 1
  br i1 %549, label %550, label %368

550:                                              ; preds = %547
  fence acquire, !noalias !36613
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %19) #91
          to label %368 unwind label %543

551:                                              ; preds = %__rustc::__rust_dealloc (.exit31), %508
  call void @llvm.lifetime.end.p0(ptr nonnull %18), !noalias !36618
  call void @llvm.lifetime.end.p0(ptr nonnull %19)
; call core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
  call fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>(ptr noalias nofree noundef align 8 dereferenceable(32) %23)
  call void @llvm.lifetime.end.p0(ptr nonnull %23)
  br label %552

552:                                              ; preds = %610, %551, %356, %353, %349
  ret void

553:                                              ; preds = %614, %368
  %554 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90
  unreachable

555:                                              ; preds = %364
  call void @llvm.lifetime.start.p0(ptr nonnull %26)
  call void @llvm.lifetime.start.p0(ptr nonnull %25)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %25, ptr noundef nonnull align 8 dereferenceable(104) %21, i64 104, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %24)
  %556 = getelementptr inbounds nuw i8, ptr %24, i64 24
  store ptr %365, ptr %556, align 8, !alias.scope !36693
  store i64 0, ptr %24, align 8, !alias.scope !36693
  %557 = getelementptr inbounds nuw i8, ptr %24, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %557, align 8, !alias.scope !36693
  %558 = getelementptr inbounds nuw i8, ptr %24, i64 16
  store i64 0, ptr %558, align 8, !alias.scope !36693
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36696)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36699)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36701)
  %559 = load i64, ptr %25, align 8, !range !2052, !alias.scope !36699, !noalias !36703, !noundef !1733
  %560 = icmp eq i64 %559, -1
  br i1 %560, label %562, label %561

561:                                              ; preds = %555
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %26, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %24, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %21)
  br label %564

562:                                              ; preds = %555
  %563 = getelementptr inbounds nuw i8, ptr %26, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %563, ptr noundef nonnull readonly align 8 dereferenceable(32) %24, i64 32, i1 false), !alias.scope !36703, !noalias !36699
  store i64 -1, ptr %26, align 8, !alias.scope !36696, !noalias !36704
  br label %564

564:                                              ; preds = %562, %561
  %565 = getelementptr inbounds nuw i8, ptr %25, i64 72
  %566 = load i64, ptr %565, align 8, !range !1771, !alias.scope !36705, !noalias !36703, !noundef !1733
  %567 = icmp ugt i64 %566, 5
  br i1 %567, label %568, label %602

568:                                              ; preds = %564
  %569 = getelementptr inbounds nuw i8, ptr %25, i64 80
  %570 = load ptr, ptr %569, align 8, !alias.scope !36699, !noalias !36703, !nonnull !1733, !noundef !1733
  %571 = mul i64 %566, 3
  %572 = add i64 %571, -3
  %573 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %574 = load i64, ptr %573, align 8, !noalias !36708, !noundef !1733
  %575 = tail call i64 @llvm.umin.i64(i64 %572, i64 9223372036854775807)
  %576 = tail call i64 @llvm.ssub.sat.i64(i64 %574, i64 %575)
  store i64 %576, ptr %573, align 8, !noalias !36708
  %577 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %578 = load i64, ptr %577, align 8, !noalias !36708, !noundef !1733
  %579 = icmp slt i64 %576, %578
  br i1 %579, label %580, label %.preheader204

580:                                              ; preds = %568
  store i64 %576, ptr %577, align 8, !noalias !36708
  br label %.preheader204

.preheader204:                                    ; preds = %580, %568
  br label %581

581:                                              ; preds = %.preheader204, %584
  %582 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !36708
  %583 = icmp slt i64 %582, 0
  br i1 %583, label %584, label %__rustc::__rust_dealloc (.exit32)

584:                                              ; preds = %581
  %585 = add nsw i64 %582, 1
  %586 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %582, i64 %585 acq_rel acquire, align 8, !noalias !36708
  %587 = extractvalue { i64, i1 } %586, 1
  br i1 %587, label %588, label %581

588:                                              ; preds = %584
  %589 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %575 monotonic, align 8, !noalias !36708
  %590 = tail call i64 @llvm.ssub.sat.i64(i64 %589, i64 %575)
  %591 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !36708
  br label %592

592:                                              ; preds = %595, %588
  %593 = phi i64 [ %591, %588 ], [ %598, %595 ]
  %594 = icmp slt i64 %590, %593
  br i1 %594, label %595, label %599

595:                                              ; preds = %592
  %596 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %593, i64 %590 monotonic monotonic, align 8, !noalias !36708
  %597 = extractvalue { i64, i1 } %596, 1
  %598 = extractvalue { i64, i1 } %596, 0
  br i1 %597, label %599, label %592

599:                                              ; preds = %595, %592
  %600 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !36708
  br label %__rustc::__rust_dealloc (.exit32)

__rustc::__rust_dealloc (.exit32): ; preds = %581, %599
  %601 = icmp ne i64 %572, 0
  tail call void @llvm.assume(i1 %601), !noalias !36708
  tail call void @free(ptr noundef nonnull %570) #92, !noalias !36708
  br label %602

602:                                              ; preds = %__rustc::__rust_dealloc (.exit32), %564
  %603 = getelementptr inbounds nuw i8, ptr %25, i64 96
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36711)
  %604 = load ptr, ptr %603, align 8, !alias.scope !36714, !noalias !36703, !noundef !1733
  %605 = icmp eq ptr %604, null
  br i1 %605, label %610, label %606

606:                                              ; preds = %602
  %607 = atomicrmw sub ptr %604, i64 1 release, align 8, !noalias !36715
  %608 = icmp eq i64 %607, 1
  br i1 %608, label %609, label %610

609:                                              ; preds = %606
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %603) #91
  br label %610

610:                                              ; preds = %609, %606, %602
  call void @llvm.lifetime.end.p0(ptr nonnull %24)
  call void @llvm.lifetime.end.p0(ptr nonnull %25)
  %611 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %611, ptr noundef nonnull align 8 dereferenceable(96) %26, i64 96, i1 false)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %26)
  br label %552

612:                                              ; preds = %614, %268
  %613 = phi { ptr, i32 } [ %370, %268 ], [ %615, %614 ]
  resume { ptr, i32 } %613

614:                                              ; preds = %269, %268, %258, %233, %217, %87, %83
  %615 = phi { ptr, i32 } [ %270, %269 ], [ %370, %268 ], [ %84, %83 ], [ %84, %87 ], [ %208, %217 ], [ %245, %233 ], [ %259, %258 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef align 8 dereferenceable(104) %21) #89
          to label %612 unwind label %553
}
