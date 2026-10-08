define void @purrdf_sparql_eval::modifier::eval_project::<purrdf_core::ir::dataset::RdfDataset>(ptr dead_on_unwind noalias nofree noundef writable writeonly sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef nonnull readonly align 8 captures(address) %3, i64 noundef range(i64 0, 576460752303423488) %4, ptr noalias nofree noundef align 16 dereferenceable(1232) %5) unnamed_addr #10 personality ptr @rust_eh_personality !guid !46185 {
  %7 = alloca [16 x i8], align 8
  %8 = alloca [16 x i8], align 8
  %9 = alloca [24 x i8], align 8
  %10 = alloca [40 x i8], align 8
  %11 = alloca [40 x i8], align 8
  %12 = alloca [24 x i8], align 8
  %13 = alloca [112 x i8], align 16
  %14 = alloca [48 x i8], align 8
  %15 = alloca [32 x i8], align 8
  %16 = alloca [112 x i8], align 16
  %17 = alloca [32 x i8], align 8
  %18 = alloca [104 x i8], align 8
  %19 = alloca [96 x i8], align 8
  %20 = alloca [8 x i8], align 8
  %21 = alloca [32 x i8], align 8
  %22 = alloca [32 x i8], align 8
  %23 = alloca [32 x i8], align 8
  %24 = alloca [104 x i8], align 8
  %25 = alloca [96 x i8], align 8
  %26 = alloca [104 x i8], align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %26)
; call <purrdf_sparql_eval::governor::lift::Lift>::at
  call void @<purrdf_sparql_eval::governor::lift::Lift>::at(ptr noalias nofree noundef nonnull sret([104 x i8]) align 8 captures(none) dereferenceable(104) %26, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !46186)
  %27 = getelementptr inbounds nuw i8, ptr %5, i64 1112
  %28 = getelementptr inbounds nuw i8, ptr %5, i64 1128
  %29 = load i64, ptr %28, align 8, !alias.scope !46186, !noalias !46189, !noundef !1701
  %30 = icmp ult i64 %29, 192153584101141163
  tail call void @llvm.assume(i1 %30)
  %31 = icmp eq i64 %29, 0
  br i1 %31, label %32, label %33

32:                                               ; preds = %6
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %16, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef nonnull align 16 dereferenceable(1232) %5, ptr noundef nonnull readonly align 8 dereferenceable(144) %2)
          to label %271 unwind label %269, !inline_history !46193

33:                                               ; preds = %6
  %34 = getelementptr inbounds nuw i8, ptr %5, i64 1120
  %35 = load ptr, ptr %34, align 16, !alias.scope !46186, !noalias !46189, !nonnull !1701, !noundef !1701
  %36 = mul nuw nsw i64 %29, 48
  %37 = getelementptr inbounds nuw i8, ptr %35, i64 %36
  call void @llvm.lifetime.start.p0(ptr nonnull %9), !noalias !46194
  %38 = shl nuw nsw i64 %4, 4
  %39 = getelementptr inbounds nuw i8, ptr %3, i64 %38
  %40 = icmp eq i64 %4, 0
  br i1 %40, label %41, label %.preheader61

41:                                               ; preds = %33
  %42 = getelementptr inbounds nuw i8, ptr %35, i64 48
  tail call void @llvm.experimental.noalias.scope.decl(metadata !46201)
  %43 = getelementptr i8, ptr %35, i64 24
  %44 = load ptr, ptr %43, align 8, !noalias !46204
  %45 = getelementptr i8, ptr %35, i64 32
  %46 = load i64, ptr %45, align 8, !noalias !46204
  br label %.loopexit60

.preheader61:                                     ; preds = %33, %72
  %47 = phi ptr [ %48, %72 ], [ %35, %33 ]
  %48 = getelementptr inbounds nuw i8, ptr %47, i64 48
  %49 = getelementptr inbounds nuw i8, ptr %47, i64 24
  %50 = load ptr, ptr %49, align 8, !noalias !46207, !nonnull !1701, !noundef !1701
  %51 = getelementptr i8, ptr %47, i64 32
  %52 = load i64, ptr %51, align 8, !noalias !46207
  tail call void @llvm.experimental.noalias.scope.decl(metadata !46201)
  %53 = getelementptr inbounds nuw i8, ptr %50, i64 16
  br label %54

54:                                               ; preds = %70, %.preheader61
  %55 = phi ptr [ %3, %.preheader61 ], [ %56, %70 ]
  %56 = getelementptr inbounds nuw i8, ptr %55, i64 16
  %57 = load ptr, ptr %55, align 8, !alias.scope !46201, !noalias !46213, !nonnull !1701, !noundef !1701
  %58 = getelementptr i8, ptr %55, i64 8
  %59 = load i64, ptr %58, align 8, !alias.scope !46201, !noalias !46213, !noundef !1701
  %60 = icmp eq ptr %57, %50
  %61 = icmp eq i64 %59, %52
  %62 = xor i1 %61, true
  %63 = or i1 %60, %62
  br i1 %63, label %68, label %64

64:                                               ; preds = %54
  %65 = getelementptr inbounds nuw i8, ptr %57, i64 16
  %66 = tail call i32 @bcmp(ptr nonnull readonly %65, ptr nonnull readonly %53, i64 %52), !alias.scope !46216, !noalias !46220
  %67 = icmp eq i32 %66, 0
  br i1 %67, label %72, label %70

68:                                               ; preds = %54
  %69 = and i1 %60, %61
  br i1 %69, label %72, label %70

70:                                               ; preds = %68, %64
  %71 = icmp eq ptr %56, %39
  br i1 %71, label %.loopexit60, label %54

72:                                               ; preds = %68, %64
  %73 = icmp eq ptr %48, %37
  br i1 %73, label %.thread, label %.preheader61

.thread:                                          ; preds = %72
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !46194
  call void @llvm.lifetime.start.p0(ptr nonnull %15), !noalias !46221
  store ptr inttoptr (i64 8 to ptr), ptr %15, align 8, !noalias !46221
  %74 = getelementptr inbounds nuw i8, ptr %15, i64 8
  %75 = getelementptr inbounds nuw i8, ptr %15, i64 16
  store i64 0, ptr %75, align 8, !noalias !46221
  %76 = getelementptr inbounds nuw i8, ptr %15, i64 24
  store ptr inttoptr (i64 8 to ptr), ptr %76, align 8, !noalias !46221
  br label %.loopexit53

.loopexit60:                                      ; preds = %70, %41
  %77 = phi i64 [ %46, %41 ], [ %52, %70 ]
  %78 = phi ptr [ %44, %41 ], [ %50, %70 ]
  %79 = phi ptr [ %42, %41 ], [ %48, %70 ]
  %80 = atomicrmw add ptr %78, i64 1 monotonic, align 8, !noalias !46204
  %81 = icmp slt i64 %80, 0
  br i1 %81, label %82, label %88

82:                                               ; preds = %.loopexit60
  tail call void @llvm.trap()
  unreachable

83:                                               ; preds = %__rustc::__rust_alloc (.exit.thread)
  %84 = landingpad { ptr, i32 }
          cleanup
  %85 = atomicrmw sub ptr %78, i64 1 release, align 8, !noalias !46222
  %86 = icmp eq i64 %85, 1
  br i1 %86, label %87, label %786

87:                                               ; preds = %83
  fence acquire
; invoke <alloc::sync::Arc<str>>::drop_slow
  invoke void @<alloc::sync::Arc<str>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %8) #87
          to label %786 unwind label %218, !noalias !46194

88:                                               ; preds = %.loopexit60
  call void @llvm.lifetime.start.p0(ptr nonnull %8), !noalias !46194
  store ptr %78, ptr %8, align 8, !noalias !46194
  %89 = getelementptr inbounds nuw i8, ptr %8, i64 8
  store i64 %77, ptr %89, align 8, !noalias !46194
  %90 = tail call noundef dereferenceable_or_null(64) ptr @malloc(i64 noundef range(i64 1, 0) 64) #88, !noalias !46229
  %91 = icmp eq ptr %90, null
  br i1 %91, label %__rustc::__rust_alloc (.exit.thread), label %92

92:                                               ; preds = %88
  %93 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.7972336046345193745))
  %94 = load i64, ptr %93, align 8, !noalias !46229, !noundef !1701
  %95 = tail call i64 @llvm.uadd.sat.i64(i64 %94, i64 1)
  store i64 %95, ptr %93, align 8, !noalias !46229
  %96 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.7972336046345193745))
  %97 = load i64, ptr %96, align 8, !noalias !46229, !noundef !1701
  %98 = tail call i64 @llvm.uadd.sat.i64(i64 %97, i64 64)
  store i64 %98, ptr %96, align 8, !noalias !46229
  %99 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.7972336046345193745))
  %100 = load i64, ptr %99, align 8, !noalias !46229, !noundef !1701
  %101 = tail call i64 @llvm.sadd.sat.i64(i64 %100, i64 64)
  store i64 %101, ptr %99, align 8, !noalias !46229
  %102 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.7972336046345193745))
  %103 = load i64, ptr %102, align 8, !noalias !46229, !noundef !1701
  %104 = icmp sgt i64 %101, %103
  br i1 %104, label %105, label %.preheader381

105:                                              ; preds = %92
  store i64 %101, ptr %102, align 8, !noalias !46229
  br label %.preheader381

.preheader381:                                    ; preds = %105, %92
  br label %106

106:                                              ; preds = %.preheader381, %109
  %107 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745) acquire, align 8, !noalias !46229
  %108 = icmp slt i64 %107, 0
  br i1 %108, label %109, label %__rustc::__rust_alloc (.exit)

109:                                              ; preds = %106
  %110 = add nsw i64 %107, 1
  %111 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745), i64 %107, i64 %110 acq_rel acquire, align 8, !noalias !46229
  %112 = extractvalue { i64, i1 } %111, 1
  br i1 %112, label %113, label %106

113:                                              ; preds = %109
  %114 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !46229
  %115 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 64 monotonic, align 8, !noalias !46229
  %116 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 64 monotonic, align 8, !noalias !46229
  %117 = tail call i64 @llvm.sadd.sat.i64(i64 %116, i64 64)
  %118 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !46229
  br label %119

119:                                              ; preds = %122, %113
  %120 = phi i64 [ %118, %113 ], [ %125, %122 ]
  %121 = icmp sgt i64 %117, %120
  br i1 %121, label %122, label %126

122:                                              ; preds = %119
  %123 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %120, i64 %117 monotonic monotonic, align 8, !noalias !46229
  %124 = extractvalue { i64, i1 } %123, 1
  %125 = extractvalue { i64, i1 } %123, 0
  br i1 %124, label %126, label %119

126:                                              ; preds = %122, %119
  %127 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745), i64 1 release, align 8, !noalias !46229
  br label %__rustc::__rust_alloc (.exit)

__rustc::__rust_alloc (.exit.thread): ; preds = %88
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef 8, i64 64) #90
          to label %128 unwind label %83, !noalias !46194

128:                                              ; preds = %__rustc::__rust_alloc (.exit.thread)
  unreachable

__rustc::__rust_alloc (.exit):  ; preds = %106, %126
  store ptr %78, ptr %90, align 8, !noalias !46194
  %129 = getelementptr inbounds nuw i8, ptr %90, i64 8
  store i64 %77, ptr %129, align 8, !noalias !46194
  store i64 4, ptr %9, align 8, !noalias !46194
  %130 = getelementptr inbounds nuw i8, ptr %9, i64 8
  store ptr %90, ptr %130, align 8, !noalias !46194
  %131 = getelementptr inbounds nuw i8, ptr %9, i64 16
  store i64 1, ptr %131, align 8, !noalias !46194
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !46194
  tail call void @llvm.experimental.noalias.scope.decl(metadata !46232)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !46235)
  %132 = icmp eq ptr %79, %37
  br i1 %132, label %.loopexit55, label %133

133:                                              ; preds = %__rustc::__rust_alloc (.exit)
  %134 = getelementptr inbounds nuw i8, ptr %7, i64 8
  br i1 %40, label %.preheader, label %.preheader57

.preheader:                                       ; preds = %133, %152
  %135 = phi ptr [ %153, %152 ], [ %90, %133 ]
  %136 = phi i64 [ %156, %152 ], [ 1, %133 ]
  %137 = phi ptr [ %138, %152 ], [ %79, %133 ]
  %138 = getelementptr inbounds nuw i8, ptr %137, i64 48
  tail call void @llvm.experimental.noalias.scope.decl(metadata !46238)
  %139 = getelementptr i8, ptr %137, i64 24
  %140 = load ptr, ptr %139, align 8, !noalias !46241, !nonnull !1701, !noundef !1701
  %141 = getelementptr i8, ptr %137, i64 32
  %142 = load i64, ptr %141, align 8, !noalias !46241
  %143 = atomicrmw add ptr %140, i64 1 monotonic, align 8, !noalias !46241
  %144 = icmp slt i64 %143, 0
  br i1 %144, label %.loopexit54, label %145

145:                                              ; preds = %.preheader
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !46246
  store ptr %140, ptr %7, align 8, !noalias !46246
  store i64 %142, ptr %134, align 8, !noalias !46246
  %146 = icmp samesign ult i64 %136, 576460752303423488
  tail call void @llvm.assume(i1 %146)
  %147 = load i64, ptr %9, align 8, !range !1810, !alias.scope !46247, !noalias !46248, !noundef !1701
  %148 = icmp eq i64 %136, %147
  br i1 %148, label %149, label %152

149:                                              ; preds = %145
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.2908892455657212669)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %9, i64 noundef %136, i64 noundef 1, i64 noundef 8, i64 noundef 16)
          to label %150 unwind label %158, !noalias !46248

150:                                              ; preds = %149
  %151 = load ptr, ptr %130, align 8, !alias.scope !46247, !noalias !46248
  br label %152

152:                                              ; preds = %150, %145
  %153 = phi ptr [ %151, %150 ], [ %135, %145 ]
  %154 = getelementptr inbounds nuw [16 x i8], ptr %153, i64 %136
  store ptr %140, ptr %154, align 8, !noalias !46246
  %155 = getelementptr inbounds nuw i8, ptr %154, i64 8
  store i64 %142, ptr %155, align 8, !noalias !46246
  %156 = add nuw nsw i64 %136, 1
  store i64 %156, ptr %131, align 8, !alias.scope !46247, !noalias !46248
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !46246
  %157 = icmp eq ptr %138, %37
  br i1 %157, label %.loopexit55, label %.preheader

158:                                              ; preds = %149
  %159 = landingpad { ptr, i32 }
          cleanup
  br label %206

.preheader57:                                     ; preds = %133, %198
  %160 = phi ptr [ %199, %198 ], [ %90, %133 ]
  %161 = phi i64 [ %202, %198 ], [ 1, %133 ]
  %162 = phi ptr [ %165, %198 ], [ %79, %133 ]
  br label %163

163:                                              ; preds = %189, %.preheader57
  %164 = phi ptr [ %165, %189 ], [ %162, %.preheader57 ]
  %165 = getelementptr inbounds nuw i8, ptr %164, i64 48
  %166 = getelementptr i8, ptr %164, i64 24
  %167 = load ptr, ptr %166, align 8, !noalias !46249, !nonnull !1701, !noundef !1701
  %168 = getelementptr i8, ptr %164, i64 32
  %169 = load i64, ptr %168, align 8, !noalias !46249
  tail call void @llvm.experimental.noalias.scope.decl(metadata !46238)
  %170 = getelementptr inbounds nuw i8, ptr %167, i64 16
  br label %171

171:                                              ; preds = %187, %163
  %172 = phi ptr [ %3, %163 ], [ %173, %187 ]
  %173 = getelementptr inbounds nuw i8, ptr %172, i64 16
  %174 = load ptr, ptr %172, align 8, !alias.scope !46238, !noalias !46255, !nonnull !1701, !noundef !1701
  %175 = getelementptr i8, ptr %172, i64 8
  %176 = load i64, ptr %175, align 8, !alias.scope !46238, !noalias !46255, !noundef !1701
  %177 = icmp eq ptr %174, %167
  %178 = icmp eq i64 %176, %169
  %179 = xor i1 %178, true
  %180 = or i1 %177, %179
  br i1 %180, label %185, label %181

181:                                              ; preds = %171
  %182 = getelementptr inbounds nuw i8, ptr %174, i64 16
  %183 = tail call i32 @bcmp(ptr nonnull readonly %182, ptr nonnull readonly %170, i64 %169), !alias.scope !46258, !noalias !46262
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
  br i1 %190, label %.loopexit55, label %163

191:                                              ; preds = %187
  %192 = atomicrmw add ptr %167, i64 1 monotonic, align 8, !noalias !46241
  %193 = icmp slt i64 %192, 0
  br i1 %193, label %.loopexit54, label %194

.loopexit54:                                      ; preds = %191, %.preheader
  tail call void @llvm.trap()
  unreachable

194:                                              ; preds = %191
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !46246
  store ptr %167, ptr %7, align 8, !noalias !46246
  store i64 %169, ptr %134, align 8, !noalias !46246
  %195 = icmp samesign ult i64 %161, 576460752303423488
  tail call void @llvm.assume(i1 %195)
  %196 = load i64, ptr %9, align 8, !range !1810, !alias.scope !46247, !noalias !46248, !noundef !1701
  %197 = icmp eq i64 %161, %196
  br i1 %197, label %212, label %198

198:                                              ; preds = %213, %194
  %199 = phi ptr [ %214, %213 ], [ %160, %194 ]
  %200 = getelementptr inbounds nuw [16 x i8], ptr %199, i64 %161
  store ptr %167, ptr %200, align 8, !noalias !46246
  %201 = getelementptr inbounds nuw i8, ptr %200, i64 8
  store i64 %169, ptr %201, align 8, !noalias !46246
  %202 = add nuw nsw i64 %161, 1
  store i64 %202, ptr %131, align 8, !alias.scope !46247, !noalias !46248
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !46246
  %203 = icmp eq ptr %165, %37
  br i1 %203, label %.loopexit55, label %.preheader57

204:                                              ; preds = %212
  %205 = landingpad { ptr, i32 }
          cleanup
  br label %206

206:                                              ; preds = %204, %158
  %207 = phi ptr [ %167, %204 ], [ %140, %158 ]
  %208 = phi { ptr, i32 } [ %205, %204 ], [ %159, %158 ]
  %209 = atomicrmw sub ptr %207, i64 1 release, align 8, !noalias !46263
  %210 = icmp eq i64 %209, 1
  br i1 %210, label %211, label %217

211:                                              ; preds = %206
  fence acquire
; invoke <alloc::sync::Arc<str>>::drop_slow
  invoke void @<alloc::sync::Arc<str>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %7) #87
          to label %217 unwind label %215, !noalias !46246

212:                                              ; preds = %194
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.2908892455657212669)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %9, i64 noundef %161, i64 noundef 1, i64 noundef 8, i64 noundef 16)
          to label %213 unwind label %204, !noalias !46248

213:                                              ; preds = %212
  %214 = load ptr, ptr %130, align 8, !alias.scope !46247, !noalias !46248
  br label %198

215:                                              ; preds = %211
  %216 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !46246
  unreachable

217:                                              ; preds = %211, %206
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_algebra::ast::Variable>>
  invoke void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_algebra::ast::Variable>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %9) #89
          to label %786 unwind label %218, !noalias !46194

218:                                              ; preds = %217, %87
  %219 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !46194
  unreachable

.loopexit55:                                      ; preds = %198, %189, %152, %__rustc::__rust_alloc (.exit)
  %220 = phi i64 [ %161, %189 ], [ %156, %152 ], [ 1, %__rustc::__rust_alloc (.exit) ], [ %202, %198 ]
  %221 = load i64, ptr %9, align 8, !noalias !46270
  %222 = load ptr, ptr %130, align 8, !noalias !46270
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !46194
  %223 = icmp ult i64 %220, 576460752303423488
  tail call void @llvm.assume(i1 %223)
  %224 = shl nuw nsw i64 %220, 4
  %225 = getelementptr inbounds nuw i8, ptr %222, i64 %224
  call void @llvm.lifetime.start.p0(ptr nonnull %15), !noalias !46221
  store ptr %222, ptr %15, align 8, !noalias !46221
  %226 = getelementptr inbounds nuw i8, ptr %15, i64 8
  %227 = getelementptr inbounds nuw i8, ptr %15, i64 16
  store i64 %221, ptr %227, align 8, !noalias !46221
  %228 = getelementptr inbounds nuw i8, ptr %15, i64 24
  store ptr %225, ptr %228, align 8, !noalias !46221
  %229 = getelementptr inbounds nuw i8, ptr %14, i64 24
  %230 = getelementptr inbounds nuw i8, ptr %14, i64 32
  %231 = getelementptr inbounds nuw i8, ptr %14, i64 40
  %232 = load i64, ptr %28, align 8, !alias.scope !46271, !noalias !46274
  br label %234

233:                                              ; preds = %244
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>(ptr noalias nofree noundef align 8 dereferenceable(32) %15) #89
          to label %786 unwind label %261, !noalias !46276, !inline_history !46277

234:                                              ; preds = %263, %.loopexit55
  %235 = phi i64 [ %232, %.loopexit55 ], [ %266, %263 ]
  %236 = phi ptr [ %222, %.loopexit55 ], [ %237, %263 ]
  %237 = getelementptr inbounds nuw i8, ptr %236, i64 16
  %238 = load ptr, ptr %236, align 8, !noalias !46278, !nonnull !1701, !noundef !1701
  %239 = getelementptr inbounds nuw i8, ptr %236, i64 8
  %240 = load i64, ptr %239, align 8, !noalias !46278, !noundef !1701
  call void @llvm.lifetime.start.p0(ptr nonnull %14), !noalias !46221
  store ptr %238, ptr %229, align 8, !noalias !46221
  store i64 %240, ptr %230, align 8, !noalias !46221
  store i64 2, ptr %14, align 8, !noalias !46221
  store i8 0, ptr %231, align 8, !noalias !46221
  tail call void @llvm.experimental.noalias.scope.decl(metadata !46271)
  %241 = load i64, ptr %27, align 8, !range !1810, !alias.scope !46271, !noalias !46274, !noundef !1701
  %242 = icmp eq i64 %235, %241
  br i1 %242, label %243, label %263

243:                                              ; preds = %234
; invoke <alloc::raw_vec::RawVec<core::option::Option<(alloc::string::String, alloc::string::String)>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<core::option::Option<(alloc::string::String, alloc::string::String)>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %27)
          to label %263 unwind label %244, !noalias !46274

244:                                              ; preds = %243
  %245 = landingpad { ptr, i32 }
          cleanup
  store ptr %237, ptr %226, align 8
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(48) %14) #89
          to label %233 unwind label %246, !noalias !46281

246:                                              ; preds = %244
  %247 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !46281
  unreachable

.loopexit53:                                      ; preds = %263, %.thread
  %248 = phi ptr [ %74, %.thread ], [ %226, %263 ]
  %249 = phi ptr [ inttoptr (i64 8 to ptr), %.thread ], [ %225, %263 ]
  store ptr %249, ptr %248, align 1
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>(ptr noalias nofree noundef align 8 dereferenceable(32) %15)
          to label %250 unwind label %269, !inline_history !46277

250:                                              ; preds = %.loopexit53
  call void @llvm.lifetime.end.p0(ptr nonnull %15), !noalias !46221
  call void @llvm.lifetime.start.p0(ptr nonnull %13), !noalias !46221
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %13, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef nonnull align 16 dereferenceable(1232) %5, ptr noundef nonnull readonly align 8 dereferenceable(144) %2)
          to label %251 unwind label %269, !inline_history !46193

251:                                              ; preds = %250
  %252 = load i64, ptr %28, align 8, !alias.scope !46282, !noalias !46285, !noundef !1701
  %253 = icmp ugt i64 %29, %252
  br i1 %253, label %260, label %254

254:                                              ; preds = %251
  %255 = sub nuw i64 %252, %29
  %256 = load ptr, ptr %34, align 16, !alias.scope !46282, !noalias !46285, !nonnull !1701, !noundef !1701
  %257 = getelementptr inbounds nuw [48 x i8], ptr %256, i64 %29
  store i64 %29, ptr %28, align 8, !alias.scope !46282, !noalias !46285
; invoke core::ptr::drop_glue::<[purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>]>
  invoke fastcc void @core::ptr::drop_glue::<[purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>]>(ptr noalias nofree noundef nonnull align 8 %257, i64 noundef %255)
          to label %260 unwind label %258

258:                                              ; preds = %254
  %259 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<core::result::Result<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>, purrdf_sparql_eval::error::EvalError>>
  invoke fastcc void @core::ptr::drop_glue::<core::result::Result<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>, purrdf_sparql_eval::error::EvalError>>(ptr noalias nofree noundef align 16 dereferenceable(112) %13) #89
          to label %786 unwind label %261, !noalias !46285, !inline_history !46277

260:                                              ; preds = %254, %251
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(112) %16, ptr noundef nonnull align 16 dereferenceable(112) %13, i64 112, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %13), !noalias !46221
  br label %271

261:                                              ; preds = %258, %233
  %262 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !46285, !inline_history !46277
  unreachable

263:                                              ; preds = %243, %234
  %264 = load ptr, ptr %34, align 16, !alias.scope !46271, !noalias !46274, !nonnull !1701, !noundef !1701
  %265 = getelementptr inbounds nuw [48 x i8], ptr %264, i64 %235
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %265, ptr noundef nonnull align 8 dereferenceable(48) %14, i64 48, i1 false), !noalias !46281
  %266 = add i64 %235, 1
  store i64 %266, ptr %28, align 8, !alias.scope !46271, !noalias !46274
  call void @llvm.lifetime.end.p0(ptr nonnull %14), !noalias !46221
  %267 = icmp eq ptr %237, %225
  br i1 %267, label %.loopexit53, label %234

268:                                              ; preds = %366
  br i1 %368, label %786, label %784

269:                                              ; preds = %364, %357, %250, %.loopexit53, %32
  %270 = landingpad { ptr, i32 }
          cleanup
  br label %786

271:                                              ; preds = %260, %32
  %272 = load i64, ptr %16, align 16, !range !1848, !noundef !1701
  %273 = trunc nuw i64 %272 to i1
  br i1 %273, label %274, label %357

274:                                              ; preds = %271
  %275 = getelementptr inbounds nuw i8, ptr %16, i64 16
  %276 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %276, ptr noundef nonnull align 16 dereferenceable(96) %275, i64 96, i1 false)
  store i64 1, ptr %0, align 16
  tail call void @llvm.experimental.noalias.scope.decl(metadata !46286)
  %277 = getelementptr inbounds nuw i8, ptr %26, i64 72
  %278 = load i64, ptr %277, align 8, !range !1933, !alias.scope !46289, !noundef !1701
  %279 = icmp ugt i64 %278, 5
  br i1 %279, label %280, label %314

280:                                              ; preds = %274
  %281 = getelementptr inbounds nuw i8, ptr %26, i64 80
  %282 = load ptr, ptr %281, align 8, !alias.scope !46286, !nonnull !1701, !noundef !1701
  %283 = mul i64 %278, 3
  %284 = add i64 %283, -3
  %285 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.7972336046345193745))
  %286 = load i64, ptr %285, align 8, !noalias !46292, !noundef !1701
  %287 = tail call i64 @llvm.umin.i64(i64 %284, i64 9223372036854775807)
  %288 = tail call i64 @llvm.ssub.sat.i64(i64 %286, i64 %287)
  store i64 %288, ptr %285, align 8, !noalias !46292
  %289 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.7972336046345193745))
  %290 = load i64, ptr %289, align 8, !noalias !46292, !noundef !1701
  %291 = icmp slt i64 %288, %290
  br i1 %291, label %292, label %.preheader332

292:                                              ; preds = %280
  store i64 %288, ptr %289, align 8, !noalias !46292
  br label %.preheader332

.preheader332:                                    ; preds = %292, %280
  br label %293

293:                                              ; preds = %.preheader332, %296
  %294 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745) acquire, align 8, !noalias !46292
  %295 = icmp slt i64 %294, 0
  br i1 %295, label %296, label %__rustc::__rust_dealloc (.exit)

296:                                              ; preds = %293
  %297 = add nsw i64 %294, 1
  %298 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745), i64 %294, i64 %297 acq_rel acquire, align 8, !noalias !46292
  %299 = extractvalue { i64, i1 } %298, 1
  br i1 %299, label %300, label %293

300:                                              ; preds = %296
  %301 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %287 monotonic, align 8, !noalias !46292
  %302 = tail call i64 @llvm.ssub.sat.i64(i64 %301, i64 %287)
  %303 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !46292
  br label %304

304:                                              ; preds = %307, %300
  %305 = phi i64 [ %303, %300 ], [ %310, %307 ]
  %306 = icmp slt i64 %302, %305
  br i1 %306, label %307, label %311

307:                                              ; preds = %304
  %308 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %305, i64 %302 monotonic monotonic, align 8, !noalias !46292
  %309 = extractvalue { i64, i1 } %308, 1
  %310 = extractvalue { i64, i1 } %308, 0
  br i1 %309, label %311, label %304

311:                                              ; preds = %307, %304
  %312 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745), i64 1 release, align 8, !noalias !46292
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %293, %311
  %313 = icmp ne i64 %284, 0
  tail call void @llvm.assume(i1 %313), !noalias !46292
  tail call void @free(ptr noundef nonnull %282) #88, !noalias !46292
  br label %314

314:                                              ; preds = %__rustc::__rust_dealloc (.exit), %274
  %315 = load i64, ptr %26, align 8, !range !2051, !alias.scope !46286, !noundef !1701
  %316 = icmp sgt i64 %315, 0
  br i1 %316, label %317, label %349

317:                                              ; preds = %314
  %318 = getelementptr inbounds nuw i8, ptr %26, i64 8
  %319 = load ptr, ptr %318, align 8, !alias.scope !46286, !nonnull !1701, !noundef !1701
  %320 = mul nuw i64 %315, 3
  %321 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.7972336046345193745))
  %322 = load i64, ptr %321, align 8, !noalias !46286, !noundef !1701
  %323 = tail call i64 @llvm.umin.i64(i64 %320, i64 9223372036854775807)
  %324 = tail call i64 @llvm.ssub.sat.i64(i64 %322, i64 %323)
  store i64 %324, ptr %321, align 8, !noalias !46286
  %325 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.7972336046345193745))
  %326 = load i64, ptr %325, align 8, !noalias !46286, !noundef !1701
  %327 = icmp slt i64 %324, %326
  br i1 %327, label %328, label %.preheader331

328:                                              ; preds = %317
  store i64 %324, ptr %325, align 8, !noalias !46286
  br label %.preheader331

.preheader331:                                    ; preds = %328, %317
  br label %329

329:                                              ; preds = %.preheader331, %332
  %330 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745) acquire, align 8, !noalias !46286
  %331 = icmp slt i64 %330, 0
  br i1 %331, label %332, label %__rustc::__rust_dealloc (.exit45)

332:                                              ; preds = %329
  %333 = add nsw i64 %330, 1
  %334 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745), i64 %330, i64 %333 acq_rel acquire, align 8, !noalias !46286
  %335 = extractvalue { i64, i1 } %334, 1
  br i1 %335, label %336, label %329

336:                                              ; preds = %332
  %337 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %323 monotonic, align 8, !noalias !46286
  %338 = tail call i64 @llvm.ssub.sat.i64(i64 %337, i64 %323)
  %339 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !46286
  br label %340

340:                                              ; preds = %343, %336
  %341 = phi i64 [ %339, %336 ], [ %346, %343 ]
  %342 = icmp slt i64 %338, %341
  br i1 %342, label %343, label %347

343:                                              ; preds = %340
  %344 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %341, i64 %338 monotonic monotonic, align 8, !noalias !46286
  %345 = extractvalue { i64, i1 } %344, 1
  %346 = extractvalue { i64, i1 } %344, 0
  br i1 %345, label %347, label %340

347:                                              ; preds = %343, %340
  %348 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745), i64 1 release, align 8, !noalias !46286
  br label %__rustc::__rust_dealloc (.exit45)

__rustc::__rust_dealloc (.exit45): ; preds = %329, %347
  tail call void @free(ptr noundef nonnull %319) #88, !noalias !46286
  br label %349

349:                                              ; preds = %__rustc::__rust_dealloc (.exit45), %314
  %350 = getelementptr inbounds nuw i8, ptr %26, i64 96
  tail call void @llvm.experimental.noalias.scope.decl(metadata !46295)
  %351 = load ptr, ptr %350, align 8, !alias.scope !46298, !noundef !1701
  %352 = icmp eq ptr %351, null
  br i1 %352, label %719, label %353

353:                                              ; preds = %349
  %354 = atomicrmw sub ptr %351, i64 1 release, align 8, !noalias !46299
  %355 = icmp eq i64 %354, 1
  br i1 %355, label %356, label %719

356:                                              ; preds = %353
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %350) #87
  br label %719

357:                                              ; preds = %271
  %358 = getelementptr inbounds nuw i8, ptr %16, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %21)
; invoke <purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %21, ptr noalias nofree noundef align 8 dereferenceable(104) %26, i64 noundef 0, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(96) %358)
          to label %359 unwind label %269

359:                                              ; preds = %357
  %360 = load i64, ptr %21, align 8, !range !2051, !noundef !1701
  %361 = icmp eq i64 %360, -1
  br i1 %361, label %364, label %362

362:                                              ; preds = %359
  call void @llvm.lifetime.start.p0(ptr nonnull %22)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %22, ptr noundef nonnull align 8 dereferenceable(32) %21, i64 32, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %21)
  call void @llvm.lifetime.start.p0(ptr nonnull %20)
; invoke <purrdf_sparql_eval::solution::VarSchema>::interned
  %363 = invoke noundef nonnull ptr @<purrdf_sparql_eval::solution::VarSchema>::interned(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %3, i64 noundef %4)
          to label %374 unwind label %369

364:                                              ; preds = %359
  call void @llvm.lifetime.end.p0(ptr nonnull %21)
; invoke <purrdf_sparql_eval::solution::VarSchema>::interned
  %365 = invoke noundef nonnull ptr @<purrdf_sparql_eval::solution::VarSchema>::interned(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %3, i64 noundef %4)
          to label %727 unwind label %269

366:                                              ; preds = %726, %722, %371, %369
  %367 = phi { ptr, i32 } [ %370, %369 ], [ %632, %371 ], [ %723, %726 ], [ %723, %722 ]
  %368 = phi i1 [ true, %369 ], [ false, %371 ], [ true, %726 ], [ true, %722 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>(ptr noalias nofree noundef align 8 dereferenceable(32) %22) #89
          to label %268 unwind label %720

369:                                              ; preds = %362
  %370 = landingpad { ptr, i32 }
          cleanup
  br label %366

371:                                              ; preds = %633, %630
  br i1 %631, label %722, label %366

372:                                              ; preds = %__rustc::__rust_alloc (.exit46.thread)
  %373 = landingpad { ptr, i32 }
          cleanup
  br label %722

374:                                              ; preds = %362
  store ptr %363, ptr %20, align 8
  %375 = getelementptr i8, ptr %363, i64 24
  %376 = load ptr, ptr %375, align 8, !nonnull !1701, !noundef !1701
  %377 = getelementptr i8, ptr %363, i64 32
  %378 = load i64, ptr %377, align 8, !noundef !1701
  %379 = getelementptr inbounds nuw i8, ptr %22, i64 24
  %380 = shl nuw i64 %378, 4
  %381 = icmp eq i64 %378, 0
  br i1 %381, label %.loopexit52, label %382

382:                                              ; preds = %374
  %383 = tail call noundef ptr @malloc(i64 noundef range(i64 1, 0) %380) #88, !noalias !46304
  %384 = icmp eq ptr %383, null
  br i1 %384, label %__rustc::__rust_alloc (.exit46.thread), label %385

385:                                              ; preds = %382
  %386 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.7972336046345193745))
  %387 = load i64, ptr %386, align 8, !noalias !46304, !noundef !1701
  %388 = tail call i64 @llvm.uadd.sat.i64(i64 %387, i64 1)
  store i64 %388, ptr %386, align 8, !noalias !46304
  %389 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.7972336046345193745))
  %390 = load i64, ptr %389, align 8, !noalias !46304, !noundef !1701
  %391 = tail call i64 @llvm.uadd.sat.i64(i64 %390, i64 %380)
  store i64 %391, ptr %389, align 8, !noalias !46304
  %392 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.7972336046345193745))
  %393 = load i64, ptr %392, align 8, !noalias !46304, !noundef !1701
  %394 = tail call i64 @llvm.umin.i64(i64 %380, i64 9223372036854775807)
  %395 = tail call i64 @llvm.sadd.sat.i64(i64 %393, i64 %394)
  store i64 %395, ptr %392, align 8, !noalias !46304
  %396 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.7972336046345193745))
  %397 = load i64, ptr %396, align 8, !noalias !46304, !noundef !1701
  %398 = icmp sgt i64 %395, %397
  br i1 %398, label %399, label %.preheader358

399:                                              ; preds = %385
  store i64 %395, ptr %396, align 8, !noalias !46304
  br label %.preheader358

.preheader358:                                    ; preds = %399, %385
  br label %400

400:                                              ; preds = %.preheader358, %403
  %401 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745) acquire, align 8, !noalias !46304
  %402 = icmp slt i64 %401, 0
  br i1 %402, label %403, label %__rustc::__rust_alloc (.exit46)

403:                                              ; preds = %400
  %404 = add nsw i64 %401, 1
  %405 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745), i64 %401, i64 %404 acq_rel acquire, align 8, !noalias !46304
  %406 = extractvalue { i64, i1 } %405, 1
  br i1 %406, label %407, label %400

407:                                              ; preds = %403
  %408 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !46304
  %409 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 %380 monotonic, align 8, !noalias !46304
  %410 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %394 monotonic, align 8, !noalias !46304
  %411 = tail call i64 @llvm.sadd.sat.i64(i64 %410, i64 %394)
  %412 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !46304
  br label %413

413:                                              ; preds = %416, %407
  %414 = phi i64 [ %412, %407 ], [ %419, %416 ]
  %415 = icmp sgt i64 %411, %414
  br i1 %415, label %416, label %420

416:                                              ; preds = %413
  %417 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %414, i64 %411 monotonic monotonic, align 8, !noalias !46304
  %418 = extractvalue { i64, i1 } %417, 1
  %419 = extractvalue { i64, i1 } %417, 0
  br i1 %418, label %420, label %413

420:                                              ; preds = %416, %413
  %421 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745), i64 1 release, align 8, !noalias !46304
  br label %__rustc::__rust_alloc (.exit46)

__rustc::__rust_alloc (.exit46): ; preds = %400, %420
  %422 = load ptr, ptr %379, align 8, !noalias !46313, !nonnull !1701, !noundef !1701
  %423 = getelementptr inbounds nuw i8, ptr %422, i64 16
  br label %425

__rustc::__rust_alloc (.exit46.thread): ; preds = %382
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef 8, i64 %380) #90
          to label %424 unwind label %372

424:                                              ; preds = %__rustc::__rust_alloc (.exit46.thread)
  unreachable

425:                                              ; preds = %429, %__rustc::__rust_alloc (.exit46)
  %426 = phi i64 [ %434, %429 ], [ 0, %__rustc::__rust_alloc (.exit46) ]
  %427 = getelementptr inbounds nuw [16 x i8], ptr %376, i64 %426
; invoke <purrdf_sparql_eval::solution::VarSchema>::index_of
  %428 = invoke fastcc { i64, i64 } @<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.2908892455657212669)(ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %423, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(16) %427) #91
          to label %429 unwind label %436, !noalias !46333

429:                                              ; preds = %425
  %430 = extractvalue { i64, i64 } %428, 0
  %431 = extractvalue { i64, i64 } %428, 1
  %432 = getelementptr inbounds nuw [16 x i8], ptr %383, i64 %426
  store i64 %430, ptr %432, align 8, !noalias !46334
  %433 = getelementptr inbounds nuw i8, ptr %432, i64 8
  store i64 %431, ptr %433, align 8, !noalias !46334
  %434 = add nuw i64 %426, 1
  %435 = icmp eq i64 %434, %378
  br i1 %435, label %.loopexit52, label %425

436:                                              ; preds = %425
  %437 = landingpad { ptr, i32 }
          cleanup
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %383, i64 noundef %380, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !46339
  br label %722

.loopexit52:                                      ; preds = %429, %374
  %438 = phi ptr [ inttoptr (i64 8 to ptr), %374 ], [ %383, %429 ]
  %439 = getelementptr inbounds nuw i8, ptr %22, i64 8
  %440 = load ptr, ptr %439, align 8, !nonnull !1701, !noundef !1701
  %441 = getelementptr inbounds nuw i8, ptr %22, i64 16
  %442 = load i64, ptr %441, align 8, !noundef !1701
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !46340
  %443 = mul nuw nsw i64 %442, 40
  %444 = icmp eq i64 %442, 0
  br i1 %444, label %445, label %448

445:                                              ; preds = %.loopexit52
  store i64 0, ptr %12, align 8, !noalias !46340
  %446 = getelementptr inbounds nuw i8, ptr %12, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %446, align 8, !noalias !46340
  %447 = getelementptr inbounds nuw i8, ptr %12, i64 16
  br label %.loopexit51

448:                                              ; preds = %.loopexit52
  %449 = tail call noundef ptr @malloc(i64 noundef range(i64 1, 0) %443) #88, !noalias !46347
  %450 = icmp eq ptr %449, null
  br i1 %450, label %__rustc::__rust_alloc (.exit47.thread), label %451

451:                                              ; preds = %448
  %452 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.7972336046345193745))
  %453 = load i64, ptr %452, align 8, !noalias !46347, !noundef !1701
  %454 = tail call i64 @llvm.uadd.sat.i64(i64 %453, i64 1)
  store i64 %454, ptr %452, align 8, !noalias !46347
  %455 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.7972336046345193745))
  %456 = load i64, ptr %455, align 8, !noalias !46347, !noundef !1701
  %457 = tail call i64 @llvm.uadd.sat.i64(i64 %456, i64 %443)
  store i64 %457, ptr %455, align 8, !noalias !46347
  %458 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.7972336046345193745))
  %459 = load i64, ptr %458, align 8, !noalias !46347, !noundef !1701
  %460 = tail call i64 @llvm.sadd.sat.i64(i64 %459, i64 %443)
  store i64 %460, ptr %458, align 8, !noalias !46347
  %461 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.7972336046345193745))
  %462 = load i64, ptr %461, align 8, !noalias !46347, !noundef !1701
  %463 = icmp sgt i64 %460, %462
  br i1 %463, label %464, label %.preheader357

464:                                              ; preds = %451
  store i64 %460, ptr %461, align 8, !noalias !46347
  br label %.preheader357

.preheader357:                                    ; preds = %464, %451
  br label %465

465:                                              ; preds = %.preheader357, %468
  %466 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745) acquire, align 8, !noalias !46347
  %467 = icmp slt i64 %466, 0
  br i1 %467, label %468, label %__rustc::__rust_alloc (.exit47)

468:                                              ; preds = %465
  %469 = add nsw i64 %466, 1
  %470 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745), i64 %466, i64 %469 acq_rel acquire, align 8, !noalias !46347
  %471 = extractvalue { i64, i1 } %470, 1
  br i1 %471, label %472, label %465

472:                                              ; preds = %468
  %473 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !46347
  %474 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 %443 monotonic, align 8, !noalias !46347
  %475 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %443 monotonic, align 8, !noalias !46347
  %476 = tail call i64 @llvm.sadd.sat.i64(i64 %475, i64 %443)
  %477 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !46347
  br label %478

478:                                              ; preds = %481, %472
  %479 = phi i64 [ %477, %472 ], [ %484, %481 ]
  %480 = icmp sgt i64 %476, %479
  br i1 %480, label %481, label %485

481:                                              ; preds = %478
  %482 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %479, i64 %476 monotonic monotonic, align 8, !noalias !46347
  %483 = extractvalue { i64, i1 } %482, 1
  %484 = extractvalue { i64, i1 } %482, 0
  br i1 %483, label %485, label %478

485:                                              ; preds = %481, %478
  %486 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745), i64 1 release, align 8, !noalias !46347
  br label %__rustc::__rust_alloc (.exit47)

__rustc::__rust_alloc (.exit47.thread): ; preds = %448
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef 8, i64 %443) #90
          to label %487 unwind label %627

487:                                              ; preds = %__rustc::__rust_alloc (.exit47.thread)
  unreachable

__rustc::__rust_alloc (.exit47): ; preds = %465, %485
  store i64 %442, ptr %12, align 8, !noalias !46340
  %488 = getelementptr inbounds nuw i8, ptr %12, i64 8
  store ptr %449, ptr %488, align 8, !noalias !46340
  %489 = getelementptr inbounds nuw i8, ptr %12, i64 16
  tail call void @llvm.experimental.noalias.scope.decl(metadata !46350)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !46353)
  %490 = getelementptr inbounds nuw i8, ptr %10, i64 16
  %491 = getelementptr inbounds nuw i8, ptr %10, i64 8
  %492 = getelementptr inbounds nuw [16 x i8], ptr %438, i64 %378
  %493 = icmp ugt i64 %378, 4
  br label %494

494:                                              ; preds = %.loopexit, %__rustc::__rust_alloc (.exit47)
  %495 = phi i64 [ 0, %__rustc::__rust_alloc (.exit47) ], [ %624, %.loopexit ]
  %496 = getelementptr inbounds nuw [40 x i8], ptr %440, i64 %495
  call void @llvm.experimental.noalias.scope.decl(metadata !46356)
  call void @llvm.lifetime.start.p0(ptr nonnull %11)
  call void @llvm.experimental.noalias.scope.decl(metadata !46359)
  call void @llvm.lifetime.start.p0(ptr nonnull %10), !noalias !46362
  store i64 1, ptr %10, align 8, !noalias !46362
  br i1 %493, label %497, label %508, !prof !1796

497:                                              ; preds = %494
; invoke <purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::grow
  invoke void @<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %10, i64 noundef 0, i64 noundef %378, i1 noundef zeroext true) #87
          to label %498 unwind label %611, !noalias !46362

498:                                              ; preds = %497
  %499 = load i64, ptr %10, align 8, !range !1933, !alias.scope !46378, !noalias !46381
  %500 = freeze i64 %499
  %501 = add i64 %500, -1
  %502 = call i64 @llvm.umax.i64(i64 %501, i64 4)
  %503 = icmp ugt i64 %501, 4
  %504 = load ptr, ptr %491, align 8, !alias.scope !46378, !noalias !46381
  %505 = select i1 %503, ptr %504, ptr %491
  %506 = select i1 %503, ptr %490, ptr %10
  %507 = load i64, ptr %506, align 8, !alias.scope !46378, !noalias !46381
  br label %508

508:                                              ; preds = %498, %494
  %509 = phi i64 [ %507, %498 ], [ 1, %494 ]
  %510 = phi ptr [ %505, %498 ], [ %491, %494 ]
  %511 = phi i64 [ %502, %498 ], [ 4, %494 ]
  %512 = phi ptr [ %506, %498 ], [ %10, %494 ]
  %513 = add i64 %509, -1
  %514 = icmp ult i64 %513, %511
  br i1 %514, label %515, label %528

515:                                              ; preds = %508
  %516 = getelementptr inbounds nuw i8, ptr %496, i64 8
  %517 = getelementptr inbounds nuw i8, ptr %496, i64 16
  %518 = load i64, ptr %496, align 8, !range !1933, !alias.scope !46383, !noalias !46384
  %519 = add i64 %518, -1
  %520 = icmp ugt i64 %519, 4
  %521 = load i64, ptr %517, align 8, !alias.scope !46383, !noalias !46384
  %522 = add i64 %521, -1
  %523 = select i1 %520, i64 %522, i64 %519
  %524 = load ptr, ptr %516, align 8, !alias.scope !46383, !noalias !46384, !nonnull !1701
  %525 = select i1 %520, ptr %524, ptr %516
  br label %543

526:                                              ; preds = %599
  %527 = add nuw i64 %511, 1
  br label %528

528:                                              ; preds = %526, %508
  %529 = phi ptr [ %438, %508 ], [ %548, %526 ]
  %530 = phi i64 [ %509, %508 ], [ %527, %526 ]
  store i64 %530, ptr %512, align 8, !alias.scope !46378, !noalias !46381
  %531 = icmp eq ptr %529, %492
  br i1 %531, label %.loopexit, label %532

532:                                              ; preds = %528
  %533 = getelementptr inbounds nuw i8, ptr %496, i64 8
  %534 = getelementptr inbounds nuw i8, ptr %496, i64 16
  %535 = load i64, ptr %496, align 8, !range !1933, !alias.scope !46383, !noalias !46384
  %536 = add i64 %535, -1
  %537 = icmp ugt i64 %536, 4
  %538 = load i64, ptr %534, align 8, !alias.scope !46383, !noalias !46384
  %539 = add i64 %538, -1
  %540 = select i1 %537, i64 %539, i64 %536
  %541 = load ptr, ptr %533, align 8, !alias.scope !46383, !noalias !46384, !nonnull !1701
  %542 = select i1 %537, ptr %541, ptr %533
  br label %560

543:                                              ; preds = %599, %515
  %544 = phi i64 [ %513, %515 ], [ %602, %599 ]
  %545 = phi ptr [ %438, %515 ], [ %548, %599 ]
  %546 = icmp eq ptr %545, %492
  br i1 %546, label %604, label %547

547:                                              ; preds = %543
  %548 = getelementptr inbounds nuw i8, ptr %545, i64 16
  %549 = load i64, ptr %545, align 8, !range !1848, !noalias !46385, !noundef !1701
  %550 = getelementptr i8, ptr %545, i64 8
  %551 = load i64, ptr %550, align 8, !noalias !46385
  %552 = trunc nuw i64 %549 to i1
  br i1 %552, label %553, label %599

553:                                              ; preds = %547
  %554 = icmp ult i64 %551, %523
  br i1 %554, label %557, label %555

555:                                              ; preds = %553
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %551, i64 noundef %523, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.98b87673fb26e02e6f5c0d2e6355fa66.716) #92
          to label %556 unwind label %606, !noalias !46381

556:                                              ; preds = %555
  unreachable

557:                                              ; preds = %553
  %558 = getelementptr inbounds nuw [8 x i8], ptr %525, i64 %551
  %559 = load <2 x i32>, ptr %558, align 4, !noalias !46388
  br label %599

560:                                              ; preds = %593, %532
  %561 = phi ptr [ %529, %532 ], [ %562, %593 ]
  %562 = getelementptr inbounds nuw i8, ptr %561, i64 16
  %563 = load i64, ptr %561, align 8, !range !1848, !noalias !46389, !noundef !1701
  %564 = getelementptr i8, ptr %561, i64 8
  %565 = load i64, ptr %564, align 8, !noalias !46389
  %566 = trunc nuw i64 %563 to i1
  br i1 %566, label %567, label %574

567:                                              ; preds = %560
  %568 = icmp ult i64 %565, %540
  br i1 %568, label %571, label %569

569:                                              ; preds = %567
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %565, i64 noundef %540, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.98b87673fb26e02e6f5c0d2e6355fa66.716) #92
          to label %570 unwind label %613, !noalias !46362

570:                                              ; preds = %569
  unreachable

571:                                              ; preds = %567
  %572 = getelementptr inbounds nuw [8 x i8], ptr %542, i64 %565
  %573 = load <2 x i32>, ptr %572, align 4, !noalias !46392
  br label %574

574:                                              ; preds = %571, %560
  %575 = phi <2 x i32> [ <i32 2, i32 undef>, %560 ], [ %573, %571 ]
  %576 = load i64, ptr %10, align 8, !range !1933, !alias.scope !46393, !noalias !46381, !noundef !1701
  %577 = add i64 %576, -1
  %578 = icmp ugt i64 %577, 4
  %579 = load ptr, ptr %491, align 8, !alias.scope !46393, !noalias !46381, !nonnull !1701
  %580 = select i1 %578, ptr %579, ptr %491
  %581 = select i1 %578, ptr %490, ptr %10
  %582 = call i64 @llvm.umax.i64(i64 %577, i64 4)
  %583 = load i64, ptr %581, align 8, !alias.scope !46393, !noalias !46381, !noundef !1701
  %584 = add i64 %583, -1
  %585 = icmp eq i64 %584, %582
  br i1 %585, label %586, label %593, !prof !1796

586:                                              ; preds = %574
; invoke <purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::grow
  invoke void @<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %10, i64 noundef %582, i64 noundef 1, i1 noundef zeroext true) #87
          to label %587 unwind label %609, !noalias !46362

587:                                              ; preds = %586
  %588 = load i64, ptr %10, align 8, !range !1933, !alias.scope !46393, !noalias !46381, !noundef !1701
  %589 = icmp ugt i64 %588, 5
  %590 = load ptr, ptr %491, align 8, !alias.scope !46393, !noalias !46381, !nonnull !1701
  %591 = select i1 %589, ptr %590, ptr %491
  %592 = select i1 %589, ptr %490, ptr %10
  br label %593

593:                                              ; preds = %587, %574
  %594 = phi ptr [ %591, %587 ], [ %580, %574 ]
  %595 = phi ptr [ %592, %587 ], [ %581, %574 ]
  %596 = getelementptr inbounds nuw [8 x i8], ptr %594, i64 %584
  store <2 x i32> %575, ptr %596, align 4, !noalias !46381
  %597 = add i64 %583, 1
  store i64 %597, ptr %595, align 8, !alias.scope !46393, !noalias !46381
  %598 = icmp eq ptr %562, %492
  br i1 %598, label %.loopexit, label %560

599:                                              ; preds = %557, %547
  %600 = phi <2 x i32> [ <i32 2, i32 undef>, %547 ], [ %559, %557 ]
  %601 = getelementptr inbounds nuw [8 x i8], ptr %510, i64 %544
  store <2 x i32> %600, ptr %601, align 4, !noalias !46381
  %602 = add i64 %544, 1
  %603 = icmp eq i64 %602, %511
  br i1 %603, label %526, label %543

604:                                              ; preds = %543
  %605 = add nuw i64 %544, 1
  store i64 %605, ptr %512, align 8, !alias.scope !46378, !noalias !46381
  br label %.loopexit

606:                                              ; preds = %555
  %607 = landingpad { ptr, i32 }
          cleanup
  %608 = add nuw i64 %544, 1
  store i64 %608, ptr %512, align 8, !alias.scope !46378, !noalias !46381
  br label %615

609:                                              ; preds = %586
  %610 = landingpad { ptr, i32 }
          cleanup
  br label %615

611:                                              ; preds = %497
  %612 = landingpad { ptr, i32 }
          cleanup
  br label %615

613:                                              ; preds = %569
  %614 = landingpad { ptr, i32 }
          cleanup
  br label %615

615:                                              ; preds = %613, %611, %609, %606
  %616 = phi { ptr, i32 } [ %607, %606 ], [ %610, %609 ], [ %612, %611 ], [ %614, %613 ]
  %617 = load i64, ptr %10, align 8, !range !1933, !alias.scope !46396, !noalias !46362, !noundef !1701
  %618 = icmp ugt i64 %617, 5
  br i1 %618, label %619, label %626

619:                                              ; preds = %615
  %620 = load ptr, ptr %491, align 8, !noalias !46362, !nonnull !1701, !noundef !1701
  %621 = shl i64 %617, 3
  %622 = add i64 %621, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %620, i64 noundef %622, i64 noundef range(i64 1, -9223372036854775807) 4) #88, !noalias !46399
  br label %626

.loopexit:                                        ; preds = %593, %604, %528
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %11, ptr noundef nonnull align 8 dereferenceable(40) %10, i64 40, i1 false), !noalias !46402
  call void @llvm.lifetime.end.p0(ptr nonnull %10), !noalias !46362
  %623 = getelementptr inbounds nuw [40 x i8], ptr %449, i64 %495
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %623, ptr noundef nonnull readonly align 8 dereferenceable(40) %11, i64 40, i1 false), !noalias !46403
  %624 = add nuw i64 %495, 1
  call void @llvm.lifetime.end.p0(ptr nonnull %11)
  %625 = icmp eq i64 %624, %442
  br i1 %625, label %.loopexit51, label %494

626:                                              ; preds = %619, %615
  store i64 %495, ptr %489, align 8, !alias.scope !46408, !noalias !46409
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %12) #89, !noalias !46340
  br label %630

627:                                              ; preds = %686, %__rustc::__rust_alloc (.exit47.thread)
  %628 = phi i1 [ false, %686 ], [ true, %__rustc::__rust_alloc (.exit47.thread) ]
  %629 = landingpad { ptr, i32 }
          cleanup
  br label %630

630:                                              ; preds = %627, %626
  %631 = phi i1 [ %628, %627 ], [ true, %626 ]
  %632 = phi { ptr, i32 } [ %629, %627 ], [ %616, %626 ]
  br i1 %381, label %371, label %633

633:                                              ; preds = %630
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %438, i64 noundef %380, i64 noundef range(i64 1, -9223372036854775807) 8) #88
  br label %371

.loopexit51:                                      ; preds = %.loopexit, %445
  %634 = phi ptr [ %447, %445 ], [ %489, %.loopexit ]
  store i64 %442, ptr %634, align 8, !alias.scope !46408, !noalias !46409
  call void @llvm.lifetime.start.p0(ptr nonnull %17)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %17, ptr noundef nonnull align 8 dereferenceable(24) %12, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !46340
  call void @llvm.lifetime.start.p0(ptr nonnull %19)
  call void @llvm.lifetime.start.p0(ptr nonnull %18)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %18, ptr noundef nonnull align 8 dereferenceable(104) %26, i64 104, i1 false)
  %635 = getelementptr inbounds nuw i8, ptr %17, i64 24
  store ptr %363, ptr %635, align 8
  call void @llvm.experimental.noalias.scope.decl(metadata !46410)
  call void @llvm.experimental.noalias.scope.decl(metadata !46413)
  call void @llvm.experimental.noalias.scope.decl(metadata !46415)
  %636 = load i64, ptr %18, align 8, !range !2051, !alias.scope !46413, !noalias !46417, !noundef !1701
  %637 = icmp eq i64 %636, -1
  br i1 %637, label %639, label %638

638:                                              ; preds = %.loopexit51
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %19, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %17, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %26)
  br label %641

639:                                              ; preds = %.loopexit51
  %640 = getelementptr inbounds nuw i8, ptr %19, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %640, ptr noundef nonnull readonly align 8 dereferenceable(32) %17, i64 32, i1 false), !alias.scope !46417, !noalias !46413
  store i64 -1, ptr %19, align 8, !alias.scope !46410, !noalias !46418
  br label %641

641:                                              ; preds = %639, %638
  %642 = getelementptr inbounds nuw i8, ptr %18, i64 72
  %643 = load i64, ptr %642, align 8, !range !1933, !alias.scope !46419, !noalias !46417, !noundef !1701
  %644 = icmp ugt i64 %643, 5
  br i1 %644, label %645, label %679

645:                                              ; preds = %641
  %646 = getelementptr inbounds nuw i8, ptr %18, i64 80
  %647 = load ptr, ptr %646, align 8, !alias.scope !46413, !noalias !46417, !nonnull !1701, !noundef !1701
  %648 = mul i64 %643, 3
  %649 = add i64 %648, -3
  %650 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.7972336046345193745))
  %651 = load i64, ptr %650, align 8, !noalias !46422, !noundef !1701
  %652 = call i64 @llvm.umin.i64(i64 %649, i64 9223372036854775807)
  %653 = call i64 @llvm.ssub.sat.i64(i64 %651, i64 %652)
  store i64 %653, ptr %650, align 8, !noalias !46422
  %654 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.7972336046345193745))
  %655 = load i64, ptr %654, align 8, !noalias !46422, !noundef !1701
  %656 = icmp slt i64 %653, %655
  br i1 %656, label %657, label %.preheader335

657:                                              ; preds = %645
  store i64 %653, ptr %654, align 8, !noalias !46422
  br label %.preheader335

.preheader335:                                    ; preds = %657, %645
  br label %658

658:                                              ; preds = %.preheader335, %661
  %659 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745) acquire, align 8, !noalias !46422
  %660 = icmp slt i64 %659, 0
  br i1 %660, label %661, label %__rustc::__rust_dealloc (.exit48)

661:                                              ; preds = %658
  %662 = add nsw i64 %659, 1
  %663 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745), i64 %659, i64 %662 acq_rel acquire, align 8, !noalias !46422
  %664 = extractvalue { i64, i1 } %663, 1
  br i1 %664, label %665, label %658

665:                                              ; preds = %661
  %666 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %652 monotonic, align 8, !noalias !46422
  %667 = call i64 @llvm.ssub.sat.i64(i64 %666, i64 %652)
  %668 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !46422
  br label %669

669:                                              ; preds = %672, %665
  %670 = phi i64 [ %668, %665 ], [ %675, %672 ]
  %671 = icmp slt i64 %667, %670
  br i1 %671, label %672, label %676

672:                                              ; preds = %669
  %673 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %670, i64 %667 monotonic monotonic, align 8, !noalias !46422
  %674 = extractvalue { i64, i1 } %673, 1
  %675 = extractvalue { i64, i1 } %673, 0
  br i1 %674, label %676, label %669

676:                                              ; preds = %672, %669
  %677 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745), i64 1 release, align 8, !noalias !46422
  br label %__rustc::__rust_dealloc (.exit48)

__rustc::__rust_dealloc (.exit48): ; preds = %658, %676
  %678 = icmp ne i64 %649, 0
  call void @llvm.assume(i1 %678), !noalias !46422
  call void @free(ptr noundef nonnull %647) #88, !noalias !46422
  br label %679

679:                                              ; preds = %__rustc::__rust_dealloc (.exit48), %641
  %680 = getelementptr inbounds nuw i8, ptr %18, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !46425)
  %681 = load ptr, ptr %680, align 8, !alias.scope !46428, !noalias !46417, !noundef !1701
  %682 = icmp eq ptr %681, null
  br i1 %682, label %687, label %683

683:                                              ; preds = %679
  %684 = atomicrmw sub ptr %681, i64 1 release, align 8, !noalias !46429
  %685 = icmp eq i64 %684, 1
  br i1 %685, label %686, label %687

686:                                              ; preds = %683
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %680) #87
          to label %687 unwind label %627

687:                                              ; preds = %686, %683, %679
  call void @llvm.lifetime.end.p0(ptr nonnull %17)
  call void @llvm.lifetime.end.p0(ptr nonnull %18)
  %688 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %688, ptr noundef nonnull align 8 dereferenceable(96) %19, i64 96, i1 false)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %19)
  br i1 %381, label %718, label %689

689:                                              ; preds = %687
  %690 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.7972336046345193745))
  %691 = load i64, ptr %690, align 8, !noundef !1701
  %692 = call i64 @llvm.umin.i64(i64 %380, i64 9223372036854775807)
  %693 = call i64 @llvm.ssub.sat.i64(i64 %691, i64 %692)
  store i64 %693, ptr %690, align 8
  %694 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.7972336046345193745))
  %695 = load i64, ptr %694, align 8, !noundef !1701
  %696 = icmp slt i64 %693, %695
  br i1 %696, label %697, label %.preheader334

697:                                              ; preds = %689
  store i64 %693, ptr %694, align 8
  br label %.preheader334

.preheader334:                                    ; preds = %697, %689
  br label %698

698:                                              ; preds = %.preheader334, %701
  %699 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745) acquire, align 8
  %700 = icmp slt i64 %699, 0
  br i1 %700, label %701, label %__rustc::__rust_dealloc (.exit49)

701:                                              ; preds = %698
  %702 = add nsw i64 %699, 1
  %703 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745), i64 %699, i64 %702 acq_rel acquire, align 8
  %704 = extractvalue { i64, i1 } %703, 1
  br i1 %704, label %705, label %698

705:                                              ; preds = %701
  %706 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %692 monotonic, align 8
  %707 = call i64 @llvm.ssub.sat.i64(i64 %706, i64 %692)
  %708 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8
  br label %709

709:                                              ; preds = %712, %705
  %710 = phi i64 [ %708, %705 ], [ %715, %712 ]
  %711 = icmp slt i64 %707, %710
  br i1 %711, label %712, label %716

712:                                              ; preds = %709
  %713 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %710, i64 %707 monotonic monotonic, align 8
  %714 = extractvalue { i64, i1 } %713, 1
  %715 = extractvalue { i64, i1 } %713, 0
  br i1 %714, label %716, label %709

716:                                              ; preds = %712, %709
  %717 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745), i64 1 release, align 8
  br label %__rustc::__rust_dealloc (.exit49)

__rustc::__rust_dealloc (.exit49): ; preds = %698, %716
  call void @free(ptr noundef nonnull %438) #88
  br label %718

718:                                              ; preds = %__rustc::__rust_dealloc (.exit49), %687
  call void @llvm.lifetime.end.p0(ptr nonnull %20)
; call core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
  call fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>(ptr noalias nofree noundef align 8 dereferenceable(32) %22)
  call void @llvm.lifetime.end.p0(ptr nonnull %22)
  br label %719

719:                                              ; preds = %782, %718, %356, %353, %349
  call void @llvm.lifetime.end.p0(ptr nonnull %26)
  ret void

720:                                              ; preds = %786, %726, %366
  %721 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86
  unreachable

722:                                              ; preds = %436, %372, %371
  %723 = phi { ptr, i32 } [ %632, %371 ], [ %373, %372 ], [ %437, %436 ]
  %724 = atomicrmw sub ptr %363, i64 1 release, align 8, !noalias !46434
  %725 = icmp eq i64 %724, 1
  br i1 %725, label %726, label %366

726:                                              ; preds = %722
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %20) #87
          to label %366 unwind label %720

727:                                              ; preds = %364
  call void @llvm.lifetime.start.p0(ptr nonnull %25)
  call void @llvm.lifetime.start.p0(ptr nonnull %24)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %24, ptr noundef nonnull align 8 dereferenceable(104) %26, i64 104, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %23)
  %728 = getelementptr inbounds nuw i8, ptr %23, i64 24
  store ptr %365, ptr %728, align 8, !alias.scope !46439
  store i64 0, ptr %23, align 8, !alias.scope !46439
  %729 = getelementptr inbounds nuw i8, ptr %23, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %729, align 8, !alias.scope !46439
  %730 = getelementptr inbounds nuw i8, ptr %23, i64 16
  store i64 0, ptr %730, align 8, !alias.scope !46439
  tail call void @llvm.experimental.noalias.scope.decl(metadata !46442)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !46445)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !46447)
  %731 = load i64, ptr %24, align 8, !range !2051, !alias.scope !46445, !noalias !46449, !noundef !1701
  %732 = icmp eq i64 %731, -1
  br i1 %732, label %734, label %733

733:                                              ; preds = %727
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %25, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %23, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %26)
  br label %736

734:                                              ; preds = %727
  %735 = getelementptr inbounds nuw i8, ptr %25, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %735, ptr noundef nonnull readonly align 8 dereferenceable(32) %23, i64 32, i1 false), !alias.scope !46449, !noalias !46445
  store i64 -1, ptr %25, align 8, !alias.scope !46442, !noalias !46450
  br label %736

736:                                              ; preds = %734, %733
  %737 = getelementptr inbounds nuw i8, ptr %24, i64 72
  %738 = load i64, ptr %737, align 8, !range !1933, !alias.scope !46451, !noalias !46449, !noundef !1701
  %739 = icmp ugt i64 %738, 5
  br i1 %739, label %740, label %774

740:                                              ; preds = %736
  %741 = getelementptr inbounds nuw i8, ptr %24, i64 80
  %742 = load ptr, ptr %741, align 8, !alias.scope !46445, !noalias !46449, !nonnull !1701, !noundef !1701
  %743 = mul i64 %738, 3
  %744 = add i64 %743, -3
  %745 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.7972336046345193745))
  %746 = load i64, ptr %745, align 8, !noalias !46454, !noundef !1701
  %747 = tail call i64 @llvm.umin.i64(i64 %744, i64 9223372036854775807)
  %748 = tail call i64 @llvm.ssub.sat.i64(i64 %746, i64 %747)
  store i64 %748, ptr %745, align 8, !noalias !46454
  %749 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.7972336046345193745))
  %750 = load i64, ptr %749, align 8, !noalias !46454, !noundef !1701
  %751 = icmp slt i64 %748, %750
  br i1 %751, label %752, label %.preheader333

752:                                              ; preds = %740
  store i64 %748, ptr %749, align 8, !noalias !46454
  br label %.preheader333

.preheader333:                                    ; preds = %752, %740
  br label %753

753:                                              ; preds = %.preheader333, %756
  %754 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745) acquire, align 8, !noalias !46454
  %755 = icmp slt i64 %754, 0
  br i1 %755, label %756, label %__rustc::__rust_dealloc (.exit50)

756:                                              ; preds = %753
  %757 = add nsw i64 %754, 1
  %758 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745), i64 %754, i64 %757 acq_rel acquire, align 8, !noalias !46454
  %759 = extractvalue { i64, i1 } %758, 1
  br i1 %759, label %760, label %753

760:                                              ; preds = %756
  %761 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %747 monotonic, align 8, !noalias !46454
  %762 = tail call i64 @llvm.ssub.sat.i64(i64 %761, i64 %747)
  %763 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !46454
  br label %764

764:                                              ; preds = %767, %760
  %765 = phi i64 [ %763, %760 ], [ %770, %767 ]
  %766 = icmp slt i64 %762, %765
  br i1 %766, label %767, label %771

767:                                              ; preds = %764
  %768 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %765, i64 %762 monotonic monotonic, align 8, !noalias !46454
  %769 = extractvalue { i64, i1 } %768, 1
  %770 = extractvalue { i64, i1 } %768, 0
  br i1 %769, label %771, label %764

771:                                              ; preds = %767, %764
  %772 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.7972336046345193745), i64 1 release, align 8, !noalias !46454
  br label %__rustc::__rust_dealloc (.exit50)

__rustc::__rust_dealloc (.exit50): ; preds = %753, %771
  %773 = icmp ne i64 %744, 0
  tail call void @llvm.assume(i1 %773), !noalias !46454
  tail call void @free(ptr noundef nonnull %742) #88, !noalias !46454
  br label %774

774:                                              ; preds = %__rustc::__rust_dealloc (.exit50), %736
  %775 = getelementptr inbounds nuw i8, ptr %24, i64 96
  tail call void @llvm.experimental.noalias.scope.decl(metadata !46457)
  %776 = load ptr, ptr %775, align 8, !alias.scope !46460, !noalias !46449, !noundef !1701
  %777 = icmp eq ptr %776, null
  br i1 %777, label %782, label %778

778:                                              ; preds = %774
  %779 = atomicrmw sub ptr %776, i64 1 release, align 8, !noalias !46461
  %780 = icmp eq i64 %779, 1
  br i1 %780, label %781, label %782

781:                                              ; preds = %778
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %775) #87
  br label %782

782:                                              ; preds = %781, %778, %774
  call void @llvm.lifetime.end.p0(ptr nonnull %23)
  call void @llvm.lifetime.end.p0(ptr nonnull %24)
  %783 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %783, ptr noundef nonnull align 8 dereferenceable(96) %25, i64 96, i1 false)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %25)
  br label %719

784:                                              ; preds = %786, %268
  %785 = phi { ptr, i32 } [ %367, %268 ], [ %787, %786 ]
  resume { ptr, i32 } %785

786:                                              ; preds = %269, %268, %258, %233, %217, %87, %83
  %787 = phi { ptr, i32 } [ %367, %268 ], [ %84, %83 ], [ %84, %87 ], [ %208, %217 ], [ %245, %233 ], [ %259, %258 ], [ %270, %269 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef align 8 dereferenceable(104) %26) #89
          to label %784 unwind label %720
}
